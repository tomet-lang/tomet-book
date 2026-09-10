use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::config::BookConfig;

/// Suffixes that mark a file as a Tomet document.
pub const DOC_EXTENSIONS: &[&str] = &[".tmt", ".tm"];

// Compiled once: `process_tomet_document` runs per file, in parallel, so
// rebuilding this on every call was pure overhead.
static ISO_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^(\d{4})-(\d{2})-(\d{2})"#).unwrap());

/// Strip a single Tomet document extension: `foo/bar.tmt` -> `foo/bar`.
///
/// Uses `strip_suffix` rather than `trim_end_matches`, which would chew through
/// repeated suffixes and turn `notes.tmt.tmt` into `notes`.
pub fn strip_doc_extension(path: &str) -> &str {
    for ext in DOC_EXTENSIONS {
        if let Some(stem) = path.strip_suffix(ext) {
            return stem;
        }
    }
    path
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TocItem {
    pub id: String,
    pub level: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroChipItem {
    pub label: Option<String>,
    pub value: String,
    pub href: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoboxRowItem {
    pub label: String,
    pub value: String,
}

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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TocNode {
    pub id: String,
    pub text: String,
    pub level: u32,
    pub children: Vec<TocNode>,
}

pub type SectionTab = TocNode;

/// Escape text for interpolation into HTML text nodes and double-quoted attributes.
///
/// Infobox values reach the template through `| safe`, so anything built by hand
/// here has to arrive already escaped.
fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Normalize a `@meta.colors` entry to a bare hex body (`#AABBCC` -> `aabbcc`).
///
/// Returns `None` for anything that is not a 3/4/6/8-digit hex color, which keeps
/// unvetted text out of the inline `style` attributes that consume it.
fn sanitize_color_hex(raw: &str) -> Option<String> {
    let hex = raw.trim().trim_start_matches('#');
    let is_hex = matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    is_hex.then(|| hex.to_ascii_lowercase())
}

fn color_chip_html(raw: &str) -> String {
    match sanitize_color_hex(raw) {
        Some(hex) => format!(
            r##"<span class="color-chip-wrapper" title="#{hex}"><span class="color-chip" style="background-color: #{hex};"></span><span class="color-hex">#{hex}</span></span>"##
        ),
        None => format!(r#"<span class="color-hex">{}</span>"#, escape_html(raw)),
    }
}

fn external_link_html(url: &str) -> String {
    let safe = escape_html(url);
    format!(r#"<a href="{safe}" target="_blank" rel="noopener noreferrer">{safe}</a>"#)
}

fn is_external_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn is_link_ref(raw: &str) -> bool {
    let s = raw.trim();
    (s.starts_with("@link(") && s.ends_with(')'))
        || (s.starts_with("link(") && s.ends_with(')'))
        || (s.starts_with("[[") && s.ends_with("]]"))
        || s.starts_with("ref:")
        || (s.starts_with('@') && s.len() > 1 && !s.contains(' ') && !s[1..].contains('@'))
}

/// Helper to clean link references like `@link(ref:"@foo")` or `ref:bar` into plain labels
fn clean_ref_target(raw: &str) -> String {
    let mut s = raw.trim();
    if s.starts_with("@link(") && s.ends_with(')') {
        s = s[6..s.len() - 1].trim();
    } else if s.starts_with("link(") && s.ends_with(')') {
        s = s[5..s.len() - 1].trim();
    } else if s.starts_with("[[") && s.ends_with("]]") {
        let inner = &s[2..s.len() - 2].trim();
        if let Some((_, a)) = inner.split_once('|') {
            return a.trim().to_string();
        } else {
            s = inner;
        }
    }
    if s.starts_with("ref:") {
        s = s["ref:".len()..].trim();
    }
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s = s[1..s.len() - 1].trim();
    }
    s.trim_start_matches('@').trim().to_string()
}

fn resolve_meta_link(
    raw: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    url_prefix: &str,
) -> Option<(String, String)> {
    let s = raw.trim();
    if !is_link_ref(s) {
        return None;
    }

    let mut target = s;
    let mut alias = None;

    if target.starts_with("@link(") && target.ends_with(')') {
        target = target[6..target.len() - 1].trim();
    } else if target.starts_with("link(") && target.ends_with(')') {
        target = target[5..target.len() - 1].trim();
    } else if target.starts_with("[[") && target.ends_with("]]") {
        let inner = &target[2..target.len() - 2].trim();
        if let Some((t, a)) = inner.split_once('|') {
            target = t.trim();
            alias = Some(a.trim().to_string());
        } else {
            target = inner;
        }
    }

    if target.starts_with("ref:") {
        target = target["ref:".len()..].trim();
    }
    if (target.starts_with('"') && target.ends_with('"'))
        || (target.starts_with('\'') && target.ends_with('\''))
    {
        target = target[1..target.len() - 1].trim();
    }

    let clean_target = target.trim();
    let display_label = alias.unwrap_or_else(|| clean_ref_target(s));

    let clean_url_prefix = url_prefix.trim_end_matches('/');

    let resolved = vault_index
        .resolve_ref(clean_target, Some(from_path))
        .or_else(|| {
            let stripped = clean_target.trim_start_matches('@');
            if stripped != clean_target {
                vault_index.resolve_ref(stripped, Some(from_path))
            } else {
                None
            }
        });

    if let Some(path) = resolved {
        let slug = strip_doc_extension(&path.to_string_lossy().replace('\\', "/")).to_string();
        let href = format!("{clean_url_prefix}/{slug}");
        Some((href, display_label))
    } else {
        Some((String::new(), display_label))
    }
}

fn format_meta_value_to_html(
    raw_str: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    url_prefix: &str,
) -> String {
    if let Some((href, label)) = resolve_meta_link(raw_str, from_path, vault_index, url_prefix) {
        let label = escape_html(&label);
        if href.is_empty() {
            format!(r#"<span class="tm-link tm-unresolved" title="未作成のページ">{label}</span>"#)
        } else {
            let href = escape_html(&href);
            format!(r#"<a href="{href}" class="tm-link tm-file">{label}</a>"#)
        }
    } else {
        escape_html(&clean_ref_target(raw_str))
    }
}

fn format_chip_value(val: &str, format: Option<&str>) -> String {
    if format == Some("birthday")
        && let Some(caps) = ISO_DATE_RE.captures(val)
    {
        let m: u32 = caps[2].parse().unwrap_or(0);
        let d: u32 = caps[3].parse().unwrap_or(0);
        return format!("{m}月{d}日");
    }
    val.to_string()
}

/// Helper to resolve media file reference in @meta (e.g. `@link(ref:+hash.png)` or `+hash.png`)
fn resolve_meta_media_path(
    raw: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    asset_prefix: &str,
) -> String {
    let s = raw.trim();
    if s.starts_with('/') || s.starts_with("http://") || s.starts_with("https://") {
        return s.to_string();
    }

    let target = clean_ref_target(s);
    let clean_prefix = asset_prefix.trim_end_matches('/');

    if let Some(resolved) = vault_index.resolve_ref(&target, Some(from_path)) {
        format!(
            "{clean_prefix}/{}",
            resolved.to_string_lossy().replace('\\', "/")
        )
    } else {
        format!("{clean_prefix}/{}", target.replace('\\', "/"))
    }
}

/// Headings the book treats as page furniture rather than content, so a
/// document opening with one still gets its real title from the next heading.
fn is_boilerplate_heading(text: &str) -> bool {
    text.eq_ignore_ascii_case("related")
        || text.eq_ignore_ascii_case("footnotes")
        || text.eq_ignore_ascii_case("references")
        || text == "関連"
        || text == "参考文献"
        || text == "脚注"
}

/// Turn the renderer's heading outline into the page TOC, alongside the first
/// content H1 (used as a title fallback).
///
/// Only levels 1-4 make it into the TOC; deeper headings are structure the
/// sticky tabs have no room for.
fn toc_from_outline(outline: &[tomet_html::HeadingInfo]) -> (Option<String>, Vec<TocItem>) {
    let mut first_h1 = None;
    let mut toc = Vec::new();

    for heading in outline {
        let text = heading.text.trim();
        if text.is_empty() {
            continue;
        }

        if heading.level == 1 && first_h1.is_none() && !is_boilerplate_heading(text) {
            first_h1 = Some(text.to_string());
        }

        if (1..=4).contains(&heading.level) {
            toc.push(TocItem {
                id: heading.id.clone().unwrap_or_default(),
                level: u32::from(heading.level),
                text: text.to_string(),
            });
        }
    }

    (first_h1, toc)
}

fn insert_node_into(parent: &mut TocNode, node: TocNode) {
    if let Some(last) = parent.children.last_mut()
        && node.level > last.level
    {
        insert_node_into(last, node);
        return;
    }
    parent.children.push(node);
}

/// Fold a flat TOC into the tree the sticky-tab header renders.
///
/// Roots are the shallowest level present (H1 when the document has any),
/// and every deeper heading nests under the most recent shallower one.
fn build_section_tabs(toc: &[TocItem]) -> Vec<TocNode> {
    let has_h1 = toc.iter().any(|item| item.level == 1);
    let top_level = if has_h1 {
        1
    } else {
        toc.iter().map(|item| item.level).min().unwrap_or(1)
    };

    let mut section_tabs: Vec<TocNode> = Vec::new();
    for item in toc {
        let node = TocNode {
            id: item.id.clone(),
            text: item.text.clone(),
            level: item.level,
            children: Vec::new(),
        };

        if section_tabs.is_empty() || item.level <= top_level {
            section_tabs.push(node);
        } else if let Some(last_root) = section_tabs.last_mut() {
            if item.level > last_root.level {
                insert_node_into(last_root, node);
            } else {
                section_tabs.push(node);
            }
        }
    }

    section_tabs
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

    // 1. Inject external workspace config (e.g. default.config.tmt) if provided
    if let Some(cfg_src) = workspace_cfg_src
        && let Ok(cfg_doc) = tomet_parser::parse_document(cfg_src)
    {
        let mut prefix_blocks = Vec::new();
        for block in cfg_doc.blocks {
            if let tomet_ast::Block::Element(el) = &block {
                let kind = tomet_semantics::classify_std_lenient(el);
                if kind == tomet_semantics::ElementKind::Config || kind.as_str() == "settings" {
                    prefix_blocks.push(block);
                }
            }
        }
        if !prefix_blocks.is_empty() {
            prefix_blocks.append(&mut doc.blocks);
            doc.blocks = prefix_blocks;
        }
    }

    // 2. Expand macros
    let doc_cfg = tomet_semantics::document_config(&doc);
    tomet_transform::expand_document_macros(&mut doc, &doc_cfg);

    // 3. Resolve links
    let from_path = Path::new(rel_path);
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

    // 4. Extract metadata & Kind
    let kind = tomet_semantics::document_kind(&doc).map(|s| s.to_string());
    let raw_meta = tomet_semantics::document_meta(&doc);
    let meta_json = raw_meta.as_ref().map(tomet_semantics::value_to_json);

    // 5. Render HTML
    let render_opts = tomet_html::RenderOptions {
        number_headings: true,
        auto_slug_headings: true,
        lang: Some(config.book.lang.clone()),
    };
    let (html, outline) = tomet_html::render_body_with_outline(&doc, &render_opts);

    // 6. Build the TOC from the renderer's own heading outline
    let (first_h1, mut toc) = toc_from_outline(&outline);

    // 7. Title resolution: @meta.title -> (if generic stem, first H1) -> file stem
    let stem = Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let meta_title = meta_json.as_ref().and_then(|m| {
        m.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });

    let is_generic_stem = stem.eq_ignore_ascii_case("index")
        || stem.eq_ignore_ascii_case("readme")
        || stem == "Untitled";

    let title = if let Some(meta_title) = meta_title {
        meta_title
    } else if is_generic_stem {
        first_h1.unwrap_or(stem)
    } else {
        stem
    };

    // If the first TOC item is an H1 identical to the document title, omit it from TOC
    if let Some(first) = toc.first()
        && first.level == 1
        && first.text == title
    {
        toc.remove(0);
    }

    // 7b. Hierarchical section tree for horizontal accordion header
    let section_tabs = build_section_tabs(&toc);

    // 8. Section classification (e.g. "30-39 Knowledge/01 Dev" -> "30-39 Knowledge")
    let section = Path::new(rel_path)
        .parent()
        .and_then(|p| p.iter().next())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    // 9. Slug calculation: "foo/bar.tmt" -> "foo/bar"
    let slug = strip_doc_extension(&rel_path.replace('\\', "/")).to_string();

    // 10. Meta properties (colors, icon, banner, etc.)
    let mut primary_color: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut banner_url: Option<String> = None;
    let mut banner_y: Option<f64> = None;
    let mut images: Vec<String> = Vec::new();
    let mut hero_chips = Vec::new();
    let mut infobox_rows = Vec::new();

    if let Some(serde_json::Value::Object(map)) = &meta_json {
        // Icon
        if let Some(val) = map.get("icon") {
            icon = val.as_str().map(|s| s.to_string());
        }

        // Primary Color
        // Only well-formed hex reaches the template's inline `style` attributes.
        if let Some(val) = map.get("colors") {
            if let Some(arr) = val.as_array() {
                if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                    primary_color = sanitize_color_hex(first);
                }
            } else if let Some(s) = val.as_str() {
                primary_color = sanitize_color_hex(s);
            }
        }

        // Banner Image
        if let Some(val) = map.get("banner")
            && let Some(b) = val.as_str()
        {
            banner_url = Some(resolve_meta_media_path(
                b,
                from_path,
                vault_index,
                config.build.clean_asset_prefix(),
            ));
        }

        // Banner Y position (e.g. banner-y: 11)
        banner_y = map
            .get("banner-y")
            .or_else(|| map.get("banner_y"))
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)));

        // Images for infobox (e.g. images: ["@link(ref:+...)"])
        if let Some(val) = map.get("images") {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        images.push(resolve_meta_media_path(
                            s,
                            from_path,
                            vault_index,
                            config.build.clean_asset_prefix(),
                        ));
                    }
                }
            } else if let Some(s) = val.as_str() {
                images.push(resolve_meta_media_path(
                    s,
                    from_path,
                    vault_index,
                    config.build.clean_asset_prefix(),
                ));
            }
        }

        // Hero Chips based on config.ui.hero_chips
        for chip_cfg in &config.ui.hero_chips {
            if let Some(val) = map.get(&chip_cfg.key) {
                match val {
                    serde_json::Value::String(s) => {
                        let (href, display) = if let Some((h, l)) = resolve_meta_link(
                            s,
                            from_path,
                            vault_index,
                            config.build.clean_url_prefix(),
                        ) {
                            (if h.is_empty() { None } else { Some(h) }, l)
                        } else {
                            (None, clean_ref_target(s))
                        };

                        if !display.is_empty() {
                            let formatted_value =
                                format_chip_value(&display, chip_cfg.format.as_deref());
                            hero_chips.push(HeroChipItem {
                                label: chip_cfg.label.clone(),
                                value: formatted_value,
                                href,
                            });
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                let (href, display) = if let Some((h, l)) = resolve_meta_link(
                                    s,
                                    from_path,
                                    vault_index,
                                    config.build.clean_url_prefix(),
                                ) {
                                    (if h.is_empty() { None } else { Some(h) }, l)
                                } else {
                                    (None, clean_ref_target(s))
                                };

                                if !display.is_empty() {
                                    let formatted_value =
                                        format_chip_value(&display, chip_cfg.format.as_deref());
                                    hero_chips.push(HeroChipItem {
                                        label: chip_cfg.label.clone(),
                                        value: formatted_value,
                                        href,
                                    });
                                }
                            }
                        }
                    }
                    serde_json::Value::Number(n) => {
                        hero_chips.push(HeroChipItem {
                            label: chip_cfg.label.clone(),
                            value: n.to_string(),
                            href: None,
                        });
                    }
                    _ => continue,
                }
            }
        }

        // Infobox Rows based on config.ui.infobox
        if config.ui.infobox.enable {
            let exclude_set: HashSet<&str> = config
                .ui
                .infobox
                .exclude_keys
                .iter()
                .map(|s| s.as_str())
                .collect();

            // Match Astro: if (kind && (!meta || !meta.type)) rawEntries.push(['kind', kind]);
            if let Some(k) = &kind
                && !exclude_set.contains("kind")
                && !map.contains_key("type")
                && !map.contains_key("kind")
            {
                let label = config
                    .ui
                    .infobox
                    .labels
                    .get("kind")
                    .cloned()
                    .unwrap_or_else(|| "種別".to_string());
                infobox_rows.push(InfoboxRowItem {
                    label,
                    value: k.clone(),
                });
            }

            for (key, val) in map {
                if exclude_set.contains(key.as_str()) {
                    continue;
                }

                let label = config
                    .ui
                    .infobox
                    .labels
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| key.clone());

                let display_val = match val {
                    serde_json::Value::String(s) => {
                        if key == "colors" {
                            format!(r#"<div class="color-chips">{}</div>"#, color_chip_html(s))
                        } else if (key.starts_with("url.") || key == "url") && is_external_url(s) {
                            external_link_html(s)
                        } else {
                            format_meta_value_to_html(
                                s,
                                from_path,
                                vault_index,
                                config.build.clean_url_prefix(),
                            )
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        if key == "colors" {
                            let chips: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(color_chip_html))
                                .collect();
                            format!(r#"<div class="color-chips">{}</div>"#, chips.join(""))
                        } else {
                            let parts: Vec<String> = arr
                                .iter()
                                .filter_map(|v| {
                                    v.as_str().map(|s| {
                                        if (key.starts_with("url.") || key == "url")
                                            && is_external_url(s)
                                        {
                                            external_link_html(s)
                                        } else {
                                            format_meta_value_to_html(
                                                s,
                                                from_path,
                                                vault_index,
                                                config.build.clean_url_prefix(),
                                            )
                                        }
                                    })
                                })
                                .collect();
                            parts.join("、")
                        }
                    }
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => {
                        if *b {
                            "Yes".to_string()
                        } else {
                            "No".to_string()
                        }
                    }
                    _ => continue,
                };

                if !display_val.is_empty() {
                    infobox_rows.push(InfoboxRowItem {
                        label,
                        value: display_val,
                    });
                }
            }
        }
    }

    let has_data = !infobox_rows.is_empty() || banner_url.is_some() || !images.is_empty();
    let source_path = if let Some(ap) = abs_path {
        ap.to_string_lossy().to_string()
    } else {
        rel_path.to_string()
    };

    Ok(ProcessedDoc {
        slug,
        rel_path: rel_path.to_string(),
        source_path,
        title,
        section,
        kind,
        primary_color,
        icon,
        banner_url,
        banner_y,
        images,
        hero_chips,
        infobox_rows,
        has_data,
        toc,
        section_tabs,
        body_html: html,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(paths: &[&str]) -> tomet_links::VaultLinkIndex {
        tomet_links::VaultLinkIndex::from_paths(paths)
    }

    // ---- link reference parsing ----

    #[test]
    fn recognizes_link_reference_forms() {
        assert!(is_link_ref(r#"@link(ref:"@foo")"#));
        assert!(is_link_ref("link(ref:bar)"));
        assert!(is_link_ref("[[foo]]"));
        assert!(is_link_ref("[[foo|別名]]"));
        assert!(is_link_ref("ref:baz"));
        assert!(is_link_ref("@foo"));
    }

    #[test]
    fn plain_values_are_not_link_references() {
        assert!(!is_link_ref("ただのテキスト"));
        assert!(!is_link_ref("2024-01-01"));
        assert!(!is_link_ref("@foo @bar"));
        assert!(!is_link_ref("a@b"));
    }

    #[test]
    fn clean_ref_target_unwraps_every_form() {
        assert_eq!(clean_ref_target(r#"@link(ref:"@foo")"#), "foo");
        assert_eq!(clean_ref_target("link(ref:bar)"), "bar");
        assert_eq!(clean_ref_target("[[foo]]"), "foo");
        assert_eq!(clean_ref_target("ref:baz"), "baz");
        assert_eq!(clean_ref_target("@qux"), "qux");
        assert_eq!(clean_ref_target("  plain  "), "plain");
    }

    #[test]
    fn clean_ref_target_prefers_the_alias() {
        assert_eq!(clean_ref_target("[[foo|別名]]"), "別名");
    }

    #[test]
    fn resolve_meta_link_builds_a_url_under_the_prefix() {
        let idx = index(&["30-39 Knowledge/rust.tmt"]);
        let (href, label) =
            resolve_meta_link("[[rust]]", Path::new("notes/a.tmt"), &idx, "/wiki").unwrap();
        assert_eq!(href, "/wiki/30-39 Knowledge/rust");
        assert_eq!(label, "rust");
    }

    #[test]
    fn resolve_meta_link_keeps_the_alias_as_the_label() {
        let idx = index(&["30-39 Knowledge/rust.tmt"]);
        let (href, label) =
            resolve_meta_link("[[rust|ラスト]]", Path::new("notes/a.tmt"), &idx, "/wiki").unwrap();
        assert_eq!(href, "/wiki/30-39 Knowledge/rust");
        assert_eq!(label, "ラスト");
    }

    #[test]
    fn resolve_meta_link_reports_unresolved_targets_with_an_empty_href() {
        let idx = index(&["a.tmt"]);
        let (href, label) =
            resolve_meta_link("[[missing]]", Path::new("a.tmt"), &idx, "/wiki").unwrap();
        assert!(href.is_empty());
        assert_eq!(label, "missing");
    }

    #[test]
    fn resolve_meta_link_ignores_non_references() {
        let idx = index(&["a.tmt"]);
        assert!(resolve_meta_link("ただの値", Path::new("a.tmt"), &idx, "/wiki").is_none());
    }

    #[test]
    fn trailing_slash_on_the_url_prefix_is_not_doubled() {
        let idx = index(&["rust.tmt"]);
        let (href, _) = resolve_meta_link("[[rust]]", Path::new("a.tmt"), &idx, "/wiki/").unwrap();
        assert_eq!(href, "/wiki/rust");
    }

    // ---- HTML escaping ----

    #[test]
    fn escape_html_covers_the_markup_and_quote_characters() {
        assert_eq!(
            escape_html(r#"<b> & "x" 'y'"#),
            "&lt;b&gt; &amp; &quot;x&quot; &#39;y&#39;"
        );
        assert_eq!(escape_html("日本語 plain"), "日本語 plain");
    }

    #[test]
    fn meta_values_cannot_break_out_of_the_infobox_cell() {
        let idx = index(&["a.tmt"]);
        let html = format_meta_value_to_html(
            r#"<img src=x onerror=alert(1)>"#,
            Path::new("a.tmt"),
            &idx,
            "/wiki",
        );
        // The payload survives as inert text, but never as markup.
        assert_eq!(html, "&lt;img src=x onerror=alert(1)&gt;");
    }

    #[test]
    fn unresolved_link_labels_are_escaped() {
        let idx = index(&["a.tmt"]);
        let html = format_meta_value_to_html(
            r#"[[missing|"><script>x</script>]]"#,
            Path::new("a.tmt"),
            &idx,
            "/wiki",
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn external_links_escape_the_url_in_both_slots() {
        let html = external_link_html(r#"https://example.com/?a=1&b="x""#);
        assert!(html.contains(r#"href="https://example.com/?a=1&amp;b=&quot;x&quot;""#));
        assert!(!html.contains(r#"b=""#));
    }

    // ---- color handling ----

    #[test]
    fn sanitize_color_hex_accepts_the_valid_hex_lengths() {
        assert_eq!(sanitize_color_hex("#AABBCC").as_deref(), Some("aabbcc"));
        assert_eq!(sanitize_color_hex("abc").as_deref(), Some("abc"));
        assert_eq!(
            sanitize_color_hex(" #12345678 ").as_deref(),
            Some("12345678")
        );
    }

    #[test]
    fn sanitize_color_hex_rejects_anything_else() {
        assert_eq!(sanitize_color_hex("red"), None);
        assert_eq!(sanitize_color_hex("#12345"), None);
        assert_eq!(sanitize_color_hex(r#"fff;" onmouseover="alert(1)"#), None);
        assert_eq!(sanitize_color_hex(""), None);
    }

    #[test]
    fn color_chips_fall_back_to_escaped_text_for_invalid_input() {
        let html = color_chip_html(r#"fff;" onmouseover="alert(1)"#);
        assert!(!html.contains("onmouseover=\""));
        assert!(!html.contains("style="));
        assert!(html.contains("&quot;"));
    }

    #[test]
    fn valid_colors_still_render_a_swatch() {
        let html = color_chip_html("#A1B2C3");
        assert!(html.contains("background-color: #a1b2c3;"));
        assert!(html.contains(r##"title="#a1b2c3""##));
    }

    // ---- chip formatting ----

    #[test]
    fn birthday_format_shortens_an_iso_date() {
        assert_eq!(format_chip_value("2001-04-09", Some("birthday")), "4月9日");
        assert_eq!(
            format_chip_value("2001-04-09T00:00:00", Some("birthday")),
            "4月9日"
        );
    }

    #[test]
    fn birthday_format_leaves_unparsable_values_alone() {
        assert_eq!(format_chip_value("春ごろ", Some("birthday")), "春ごろ");
        assert_eq!(format_chip_value("2001-04-09", None), "2001-04-09");
    }

    // ---- section tab tree ----

    fn toc(items: &[(u32, &str)]) -> Vec<TocItem> {
        items
            .iter()
            .map(|(level, text)| TocItem {
                id: text.to_lowercase(),
                level: *level,
                text: text.to_string(),
            })
            .collect()
    }

    fn shape(nodes: &[TocNode]) -> Vec<(String, Vec<String>)> {
        nodes
            .iter()
            .map(|n| {
                (
                    n.text.clone(),
                    n.children.iter().map(|c| c.text.clone()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn flat_headings_all_become_roots() {
        let tabs = build_section_tabs(&toc(&[(1, "A"), (1, "B"), (1, "C")]));
        assert_eq!(
            shape(&tabs),
            vec![
                ("A".into(), vec![]),
                ("B".into(), vec![]),
                ("C".into(), vec![]),
            ]
        );
    }

    #[test]
    fn deeper_headings_nest_under_the_preceding_root() {
        let tabs = build_section_tabs(&toc(&[
            (1, "A"),
            (2, "A-1"),
            (2, "A-2"),
            (1, "B"),
            (2, "B-1"),
        ]));
        assert_eq!(
            shape(&tabs),
            vec![
                ("A".into(), vec!["A-1".into(), "A-2".into()]),
                ("B".into(), vec!["B-1".into()]),
            ]
        );
    }

    #[test]
    fn third_level_headings_nest_under_their_parent() {
        let tabs = build_section_tabs(&toc(&[(1, "A"), (2, "A-1"), (3, "A-1-a"), (2, "A-2")]));
        assert_eq!(tabs.len(), 1);
        let a1 = &tabs[0].children[0];
        assert_eq!(a1.text, "A-1");
        assert_eq!(a1.children.len(), 1);
        assert_eq!(a1.children[0].text, "A-1-a");
        assert_eq!(tabs[0].children[1].text, "A-2");
    }

    #[test]
    fn documents_without_h1_use_their_shallowest_level_as_roots() {
        let tabs = build_section_tabs(&toc(&[(2, "A"), (3, "A-1"), (2, "B")]));
        assert_eq!(
            shape(&tabs),
            vec![("A".into(), vec!["A-1".into()]), ("B".into(), vec![]),]
        );
    }

    #[test]
    fn an_empty_toc_yields_no_tabs() {
        assert!(build_section_tabs(&[]).is_empty());
    }
}

#[cfg(test)]
mod extension_tests {
    use super::strip_doc_extension;

    #[test]
    fn strips_a_single_document_extension() {
        assert_eq!(strip_doc_extension("foo/bar.tmt"), "foo/bar");
        assert_eq!(strip_doc_extension("foo/bar.tm"), "foo/bar");
    }

    #[test]
    fn does_not_chew_through_repeated_suffixes() {
        assert_eq!(strip_doc_extension("notes.tmt.tmt"), "notes.tmt");
    }

    #[test]
    fn leaves_other_names_alone() {
        assert_eq!(strip_doc_extension("image.png"), "image.png");
        assert_eq!(strip_doc_extension("plain"), "plain");
    }
}

#[cfg(test)]
mod outline_tests {
    use super::*;
    use tomet_html::HeadingInfo;

    fn heading(level: u8, id: Option<&str>, text: &str) -> HeadingInfo {
        HeadingInfo {
            level,
            id: id.map(str::to_string),
            text: text.to_string(),
            number: None,
        }
    }

    #[test]
    fn levels_one_to_four_become_toc_items() {
        let (_, toc) = toc_from_outline(&[
            heading(1, Some("a"), "A"),
            heading(4, Some("d"), "D"),
            heading(5, Some("e"), "E"),
        ]);

        let seen: Vec<(u32, &str)> = toc.iter().map(|i| (i.level, i.text.as_str())).collect();
        assert_eq!(
            seen,
            vec![(1, "A"), (4, "D")],
            "H5 is too deep for the tabs"
        );
    }

    #[test]
    fn a_heading_without_an_id_still_lists_but_cannot_be_linked() {
        let (_, toc) = toc_from_outline(&[heading(1, None, "A")]);
        assert_eq!(toc[0].id, "");
    }

    #[test]
    fn the_first_content_h1_is_reported_as_the_title_candidate() {
        let (first_h1, _) = toc_from_outline(&[
            heading(1, Some("a"), "Real Title"),
            heading(1, Some("b"), "Later"),
        ]);
        assert_eq!(first_h1.as_deref(), Some("Real Title"));
    }

    #[test]
    fn boilerplate_headings_never_become_the_title() {
        let (first_h1, _) = toc_from_outline(&[
            heading(1, Some("r"), "関連"),
            heading(1, Some("t"), "本当の見出し"),
        ]);
        assert_eq!(first_h1.as_deref(), Some("本当の見出し"));

        for label in ["Related", "references", "FOOTNOTES", "参考文献", "脚注"] {
            let (first_h1, _) = toc_from_outline(&[heading(1, Some("x"), label)]);
            assert_eq!(first_h1, None, "{label} should not be a title");
        }
    }

    #[test]
    fn a_deeper_first_heading_leaves_the_title_unset() {
        let (first_h1, toc) = toc_from_outline(&[heading(2, Some("s"), "Section")]);
        assert_eq!(first_h1, None);
        assert_eq!(toc.len(), 1);
    }

    #[test]
    fn blank_headings_are_dropped() {
        let (first_h1, toc) =
            toc_from_outline(&[heading(1, Some("a"), "   "), heading(1, Some("b"), "B")]);
        assert_eq!(toc.len(), 1);
        assert_eq!(first_h1.as_deref(), Some("B"));
    }

    #[test]
    fn the_number_label_never_leaks_into_the_toc_text() {
        // Numbering lives in HeadingInfo::number, so the text is the author's.
        let numbered = HeadingInfo {
            level: 1,
            id: Some("a".to_string()),
            text: "Chapter".to_string(),
            number: Some("1".to_string()),
        };
        let (_, toc) = toc_from_outline(&[numbered]);
        assert_eq!(toc[0].text, "Chapter");
    }
}
