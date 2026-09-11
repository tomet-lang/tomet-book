//! One source document, turned into everything a page needs.

mod html;
mod links;
mod meta;
mod toc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::BookConfig;

pub use links::strip_doc_extension;
pub use meta::{HeroChipItem, InfoboxRowItem};
pub use toc::{SectionTab, TocItem};

use toc::{build_section_tabs, toc_from_outline};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedDoc {
    pub slug: String,
    pub rel_path: String,
    pub source_path: String,
    pub title: String,
    pub section: Option<String>,
    pub kind: Option<String>,
    pub primary_color: Option<String>,
    pub icon: Option<String>,
    pub banner_url: Option<String>,
    pub banner_y: Option<f64>,
    pub images: Vec<String>,
    pub hero_chips: Vec<HeroChipItem>,
    pub infobox_rows: Vec<InfoboxRowItem>,
    pub has_data: bool,
    pub toc: Vec<TocItem>,
    pub section_tabs: Vec<SectionTab>,
    pub body_html: String,
    /// Pages this one links to, as slugs. The build reverses these into the
    /// backlinks each page shows.
    pub outgoing: Vec<String>,
}

/// Prepend the `@config`/`@settings` blocks of a workspace-wide document, so a
/// vault's shared macros are in scope while this one expands.
fn inject_workspace_config(doc: &mut tomet_ast::Document, workspace_cfg_src: &str) {
    let Ok(cfg_doc) = tomet_parser::parse_document(workspace_cfg_src) else {
        return;
    };

    let mut prefix_blocks: Vec<tomet_ast::Block> = cfg_doc
        .blocks
        .into_iter()
        .filter(|block| match block {
            tomet_ast::Block::Element(el) => {
                let kind = tomet_semantics::classify_std_lenient(el);
                kind == tomet_semantics::ElementKind::Config || kind.as_str() == "settings"
            }
            _ => false,
        })
        .collect();

    if !prefix_blocks.is_empty() {
        prefix_blocks.append(&mut doc.blocks);
        doc.blocks = prefix_blocks;
    }
}

/// The page title: `@meta.title`, else the first H1 when the filename says
/// nothing (`index`, `readme`), else the filename.
fn resolve_title(
    rel_path: &str,
    meta_json: Option<&serde_json::Value>,
    first_h1: Option<String>,
) -> String {
    let stem = Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let meta_title = meta_json
        .and_then(|m| m.get("title"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    if let Some(meta_title) = meta_title {
        return meta_title;
    }

    let is_generic_stem = stem.eq_ignore_ascii_case("index")
        || stem.eq_ignore_ascii_case("readme")
        || stem == "Untitled";

    if is_generic_stem {
        first_h1.unwrap_or(stem)
    } else {
        stem
    }
}

pub fn process_tomet_document(
    source: &str,
    rel_path: &str,
    abs_path: Option<&Path>,
    config: &BookConfig,
    vault_index: &tomet_links::VaultLinkIndex,
    workspace_cfg_src: Option<&str>,
) -> Result<ProcessedDoc> {
    let mut doc = tomet_parser::parse_document(source)
        .map_err(|e| anyhow::anyhow!("Failed to parse {rel_path}: {e}"))?;

    // 1. Bring in the vault-wide config (e.g. default.config.tmt)
    if let Some(cfg_src) = workspace_cfg_src {
        inject_workspace_config(&mut doc, cfg_src);
    }

    // 2. Expand macros
    let doc_cfg = tomet_semantics::document_config(&doc);
    tomet_transform::expand_document_macros(&mut doc, &doc_cfg);

    // 3. Collect outgoing links, then resolve them
    let from_path = Path::new(rel_path);
    let slug = strip_doc_extension(&rel_path.replace('\\', "/")).to_string();
    let mut outgoing = links::outgoing_page_slugs(&doc, from_path, &slug, vault_index);

    let mode = tomet_transform::TargetMode::WebSlug {
        url_prefix: config.build.clean_url_prefix().to_string(),
        asset_prefix: config.build.clean_asset_prefix().to_string(),
    };
    tomet_transform::resolve_document_links(
        &mut doc,
        Some(from_path),
        |target, from| {
            vault_index
                .resolve_ref(target, from)
                .map(|p| p.to_path_buf())
        },
        &mode,
    );

    // 4. Metadata and kind
    let kind = tomet_semantics::document_kind(&doc).map(|s| s.to_string());
    let raw_meta = tomet_semantics::document_meta(&doc);
    let meta_json = raw_meta.as_ref().map(tomet_semantics::value_to_json);

    // 5. Render, and take the heading outline the renderer reports
    let render_opts = tomet_html::RenderOptions {
        number_headings: true,
        auto_slug_headings: true,
        lang: Some(config.book.lang.clone()),
    };
    let (body_html, outline) = tomet_html::render_body_with_outline(&doc, &render_opts);

    // 6. Table of contents, title, and the sticky-tab tree
    let (first_h1, mut toc) = toc_from_outline(&outline);
    let title = resolve_title(rel_path, meta_json.as_ref(), first_h1);

    // The title already stands at the top of the page; no need to repeat it.
    if let Some(first) = toc.first()
        && first.level == 1
        && first.text == title
    {
        toc.remove(0);
    }
    let section_tabs = build_section_tabs(&toc);

    // 7. Where the page lives: "30-39 Knowledge/01 Dev/a.tmt" -> section, slug
    let section = Path::new(rel_path)
        .parent()
        .and_then(|p| p.iter().next())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    // 8. The page's chrome: colours, banner, chips, infobox
    let mut props = meta::extract(
        meta_json.as_ref(),
        kind.as_deref(),
        from_path,
        vault_index,
        config,
    );

    // 本文のリンクに @meta のリンクを合流させる。自分自身と重複は落とす。
    for linked in std::mem::take(&mut props.linked_slugs) {
        if linked != slug && !outgoing.contains(&linked) {
            outgoing.push(linked);
        }
    }

    let source_path = abs_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| rel_path.to_string());

    Ok(ProcessedDoc {
        slug,
        rel_path: rel_path.to_string(),
        source_path,
        title,
        section,
        kind,
        has_data: props.has_data(),
        primary_color: props.primary_color,
        icon: props.icon,
        banner_url: props.banner_url,
        banner_y: props.banner_y,
        images: props.images,
        hero_chips: props.hero_chips,
        infobox_rows: props.infobox_rows,
        toc,
        section_tabs,
        body_html,
        outgoing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_title_wins_over_everything() {
        let meta = serde_json::json!({ "title": "From Meta" });
        assert_eq!(
            resolve_title("notes/file.tmt", Some(&meta), Some("From H1".into())),
            "From Meta"
        );
    }

    #[test]
    fn a_named_file_keeps_its_name() {
        assert_eq!(
            resolve_title("notes/Rust.tmt", None, Some("Some Heading".into())),
            "Rust"
        );
    }

    #[test]
    fn a_generic_filename_defers_to_the_first_heading() {
        for stem in ["index", "README"] {
            let path = format!("notes/{stem}.tmt");
            assert_eq!(
                resolve_title(&path, None, Some("Real Title".into())),
                "Real Title"
            );
        }
    }

    #[test]
    fn a_generic_filename_with_no_heading_falls_back_to_the_stem() {
        assert_eq!(resolve_title("notes/index.tmt", None, None), "index");
    }
}
