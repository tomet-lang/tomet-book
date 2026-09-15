//! One source document, turned into everything a page needs.

mod html;
mod icon;
mod links;
mod meta;
mod toc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    pub banner_original_url: Option<String>,
    pub banner_y: Option<f64>,
    pub images: Vec<String>,
    pub original_images: Vec<String>,
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

/// Extract the `@config`/`@settings` blocks of a workspace-wide document.
pub fn extract_workspace_config_blocks(workspace_cfg_src: &str) -> Vec<tomet_ast::Block> {
    let Ok(cfg_doc) = tomet_parser::parse_document(workspace_cfg_src) else {
        return Vec::new();
    };

    cfg_doc
        .blocks
        .into_iter()
        .filter(|block| match block {
            tomet_ast::Block::Element(el) => {
                let kind = tomet_semantics::classify_std_lenient(el);
                kind == tomet_semantics::ElementKind::Config || kind.as_str() == "settings"
            }
            _ => false,
        })
        .collect()
}

/// Prepend pre-parsed workspace-wide config blocks so shared macros are in scope.
pub fn inject_workspace_config_blocks(
    doc: &mut tomet_ast::Document,
    prefix_blocks: &[tomet_ast::Block],
) {
    if !prefix_blocks.is_empty() {
        let mut new_blocks = prefix_blocks.to_vec();
        new_blocks.append(&mut doc.blocks);
        doc.blocks = new_blocks;
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

#[cfg(test)]
mod kind_tests {
    use super::*;

    #[test]
    fn a_document_reports_the_kind_it_declares() {
        let (_, kind) = parse_tomet_document("@kind(config)\n\n#[ x ]\n", "a.tmt").unwrap();
        assert_eq!(kind.as_deref(), Some("config"));

        let (_, none) = parse_tomet_document("#[ x ]\n", "b.tmt").unwrap();
        assert_eq!(none, None);
    }

    #[test]
    fn a_link_to_an_unpublished_document_resolves_to_nothing() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&[
            "page.tmt".to_string(),
            "secret.tmt".to_string(),
        ]);

        let unpublished: HashSet<String> = ["secret.tmt".to_string()].into_iter().collect();
        let out = process_tomet_document(
            "#[ Page ]\n\n@link(ref:\"secret\")\n",
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &unpublished,
        )
        .unwrap();

        assert!(
            out.body_html.contains("tm-ref-unresolved"),
            "a link to a page the book leaves out must read as broken: {}",
            out.body_html
        );
        assert!(out.outgoing.is_empty(), "and must not count as a backlink");
    }

    #[test]
    fn the_same_link_resolves_when_the_page_is_published() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&[
            "page.tmt".to_string(),
            "secret.tmt".to_string(),
        ]);

        let out = process_tomet_document(
            "#[ Page ]\n\n@link(ref:\"secret\")\n",
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &HashSet::new(),
        )
        .unwrap();

        assert!(
            !out.body_html.contains("tm-ref-unresolved"),
            "{}",
            out.body_html
        );
        assert_eq!(out.outgoing, ["secret"]);
    }

    #[test]
    fn a_page_renders_ruby_and_doc_icon() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&["page.tmt".to_string()]);

        let out = process_tomet_document(
            "#[ Page ]\n\n@ruby[漢字](rt:\"かんじ\") and @doc.icon(\"star\", pkg:\"lucide\").\n",
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &HashSet::new(),
        )
        .unwrap();

        assert!(
            out.body_html.contains("<ruby>漢字<rt>かんじ</rt></ruby>"),
            "{}",
            out.body_html
        );
        assert!(
            out.body_html
                .contains("<svg class=\"tm-doc-icon\" data-icon=\"star\""),
            "{}",
            out.body_html
        );
    }

    #[test]
    fn a_meta_icon_written_as_a_doc_icon_element_renders_as_svg() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&["page.tmt".to_string()]);

        let out = process_tomet_document(
            "@meta{icon: @doc.icon(\"cake\")}\n\n#[ Page ]\n\nbody\n",
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &HashSet::new(),
        )
        .unwrap();

        let icon = out.icon.as_deref().unwrap_or_default();
        assert!(
            icon.contains("<svg") && icon.contains("data-icon=\"cake\""),
            "{icon}"
        );
    }

    #[test]
    fn a_meta_icon_written_as_a_plain_string_still_works() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&["page.tmt".to_string()]);

        let out = process_tomet_document(
            "@meta{icon: \"\u{1F382}\"}\n\n#[ Page ]\n\nbody\n",
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &HashSet::new(),
        )
        .unwrap();

        assert_eq!(out.icon.as_deref(), Some("\u{1F382}"));
    }

    #[test]
    fn a_meta_banner_and_images_written_as_link_elements_resolve() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&[
            "page.tmt".to_string(),
            "+771a262c8eeb5a2459a01a44a2404eaa2999bc54.svg".to_string(),
            "a.png".to_string(),
            "b.png".to_string(),
        ]);

        let out = process_tomet_document(
            r#"@meta{
    banner: @link(ref:+771a262c8eeb5a2459a01a44a2404eaa2999bc54.svg),
    images: list(@link(ref: "a.png"), @link(ref: "b.png"))
}

#[ Page ]

body
"#,
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &HashSet::new(),
        )
        .unwrap();

        assert_eq!(
            out.banner_url.as_deref(),
            Some("/vault/+771a262c8eeb5a2459a01a44a2404eaa2999bc54.svg")
        );
        assert_eq!(
            out.images,
            vec!["/vault/a.png", "/vault/b.png"]
        );
    }

    #[test]
    fn a_meta_parent_and_row_written_as_link_element_resolve() {
        let config = BookConfig::default();
        let vault = tomet_links::VaultLinkIndex::from_paths(&[
            "page.tmt".to_string(),
            "30-39 Knowledge/rust.tmt".to_string(),
        ]);

        let out = process_tomet_document(
            r#"@meta{
    parent: @link(ref: "rust"),
    author: @link(ref: "rust")
}

#[ Page ]

body
"#,
            "page.tmt",
            None,
            &config,
            &vault,
            None,
            &HashSet::new(),
        )
        .unwrap();

        assert_eq!(out.hero_chips.len(), 1);
        assert_eq!(out.hero_chips[0].value, "rust");
        assert_eq!(
            out.hero_chips[0].href.as_deref(),
            Some("/wiki/30-39 Knowledge/rust")
        );

        assert!(out.outgoing.contains(&"30-39 Knowledge/rust".to_string()));

        let author_row = out.infobox_rows.iter().find(|r| r.label == "author").unwrap();
        assert!(author_row.value.contains(r#"href="/wiki/30-39 Knowledge/rust""#));
    }
}

/// Read a document far enough to know what it is.
///
/// The build has to decide what the site publishes before it resolves a
/// single link, since a link to a document the site leaves out has to come
/// out broken. That decision needs the `@kind`, and the `@kind` needs a
/// parse -- so the parse is done once, here, and handed on.
pub fn parse_tomet_document(
    source: &str,
    rel_path: &str,
) -> Result<(tomet_ast::Document, Option<String>)> {
    let doc = tomet_parser::parse_document(source)
        .map_err(|e| anyhow::anyhow!("Failed to parse {rel_path}: {e}"))?;
    let kind = tomet_semantics::document_kind(&doc).map(|s| s.to_string());
    Ok((doc, kind))
}

pub fn process_tomet_document(
    source: &str,
    rel_path: &str,
    abs_path: Option<&Path>,
    config: &BookConfig,
    vault_index: &tomet_links::VaultLinkIndex,
    workspace_cfg_src: Option<&str>,
    unpublished: &HashSet<String>,
) -> Result<ProcessedDoc> {
    let (doc, _) = parse_tomet_document(source, rel_path)?;
    process_parsed_document(
        doc,
        rel_path,
        abs_path,
        config,
        vault_index,
        workspace_cfg_src,
        unpublished,
    )
}

pub fn process_parsed_document(
    doc: tomet_ast::Document,
    rel_path: &str,
    abs_path: Option<&Path>,
    config: &BookConfig,
    vault_index: &tomet_links::VaultLinkIndex,
    workspace_cfg_src: Option<&str>,
    unpublished: &HashSet<String>,
) -> Result<ProcessedDoc> {
    let blocks = workspace_cfg_src
        .map(extract_workspace_config_blocks)
        .unwrap_or_default();
    process_parsed_document_with_blocks(
        doc,
        rel_path,
        abs_path,
        config,
        vault_index,
        &blocks,
        unpublished,
    )
}

/// Turn a parsed document into the page it becomes using pre-parsed workspace config blocks.
pub fn process_parsed_document_with_blocks(
    mut doc: tomet_ast::Document,
    rel_path: &str,
    abs_path: Option<&Path>,
    config: &BookConfig,
    vault_index: &tomet_links::VaultLinkIndex,
    workspace_cfg_blocks: &[tomet_ast::Block],
    unpublished: &HashSet<String>,
) -> Result<ProcessedDoc> {
    // 1. Bring in the vault-wide config (e.g. default.config.tmt)
    inject_workspace_config_blocks(&mut doc, workspace_cfg_blocks);

    // 2. Expand macros
    let doc_cfg = tomet_semantics::document_config(&doc);
    tomet_transform::expand_document_macros(&mut doc, &doc_cfg);

    // 3. Collect outgoing links, then resolve them
    let from_path = Path::new(rel_path);
    let slug = strip_doc_extension(&crate::book::normalize_path_str(rel_path)).to_string();
    let mut outgoing = links::outgoing_page_slugs(&doc, from_path, &slug, vault_index, unpublished);

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
                // A page the site does not publish is not a page. Resolving
                // it would print a link to a file that was never written.
                .filter(|p| !unpublished.contains(&crate::book::normalize_path(p)))
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
        custom_element: Some(tomet_html::CustomElementRenderer::new(icon::render)),
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

    // `icon: @doc.icon("cake")` -- an embedded element rather than the
    // plain string/emoji `meta::extract` reads out of `meta_json`. Checked
    // against the raw, pre-JSON value: `Value::Element` has no JSON form
    // (`tomet_semantics::value_to_json` degrades it to a tagged object),
    // so this is the one path that can still see it.
    if let Some(icon_html) = icon::meta_icon_element(raw_meta.as_ref()) {
        props.icon = Some(icon_html);
    }

    // Merge links from @meta into body links, dropping self-references and duplicates.
    for linked in std::mem::take(&mut props.linked_slugs) {
        if linked != slug && !outgoing.contains(&linked) {
            outgoing.push(linked);
        }
    }

    let source_path = abs_path
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| rel_path.to_string());

    let banner_original_url = props.banner_url.clone();
    let original_images = props.images.clone();

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
        banner_original_url,
        banner_y: props.banner_y,
        images: props.images,
        original_images,
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
