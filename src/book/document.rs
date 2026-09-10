use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::config::BookConfig;

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
    pub title: String,
    pub section: Option<String>,
    pub kind: Option<String>,
    pub primary_color: Option<String>,
    pub icon: Option<String>,
    pub banner_url: Option<String>,
    pub hero_chips: Vec<HeroChipItem>,
    pub infobox_rows: Vec<InfoboxRowItem>,
    pub has_data: bool,
    pub toc: Vec<TocItem>,
    pub body_html: String,
}

/// Helper to clean link references like `@link(ref:"@foo")` or `ref:bar` into plain labels
fn clean_ref_target(raw: &str) -> String {
    let mut s = raw.trim();
    if s.starts_with("@link(") && s.ends_with(')') {
        s = &s[6..s.len() - 1].trim();
    } else if s.starts_with("link(") && s.ends_with(')') {
        s = &s[5..s.len() - 1].trim();
    }
    if s.starts_with("ref:") {
        s = s["ref:".len()..].trim();
    }
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s = &s[1..s.len() - 1].trim();
    }
    s.trim_start_matches('@').trim().to_string()
}

pub fn process_tomet_document(
    source: &str,
    rel_path: &str,
    config: &BookConfig,
    vault_index: &tomet_links::VaultLinkIndex,
    workspace_cfg_src: Option<&str>,
) -> Result<ProcessedDoc> {
    let mut doc = tomet_parser::parse_document(source)
        .map_err(|e| anyhow::anyhow!("Failed to parse {rel_path}: {e}"))?;

    // 1. Inject external workspace config (e.g. default.config.tmt) if provided
    if let Some(cfg_src) = workspace_cfg_src {
        if let Ok(cfg_doc) = tomet_parser::parse_document(cfg_src) {
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
    }

    // 2. Expand macros
    let doc_cfg = tomet_semantics::document_config(&doc);
    tomet_transform::expand_document_macros(&mut doc, &doc_cfg);

    // 3. Resolve links
    let from_path = Path::new(rel_path);
    let mode = tomet_transform::TargetMode::WebSlug {
        url_prefix: config.build.url_prefix.clone(),
        asset_prefix: config.build.asset_prefix.clone(),
    };
    tomet_transform::resolve_document_links(
        &mut doc,
        Some(from_path),
        |target, from| vault_index.resolve_ref(target, from).map(|p| p.to_path_buf()),
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
    let html = tomet_html::render_body_with(&doc, &render_opts);

    // 6. Extract TOC and H1 from HTML
    let mut first_h1: Option<String> = None;
    let mut toc = Vec::new();

    let heading_re = regex::Regex::new(r#"<h([1-6])\b([^>]*)>([\s\S]*?)</h([1-6])>"#).unwrap();
    let id_re = regex::Regex::new(r#"id="([^"]*)""#).unwrap();
    let heading_number_re = regex::Regex::new(r#"<span class="(?:tmt|tm)-heading-number">[^<]*</span>"#).unwrap();
    let tag_re = regex::Regex::new(r#"<[^>]+>"#).unwrap();

    for cap in heading_re.captures_iter(&html) {
        let open_level: u32 = cap[1].parse().unwrap_or(1);
        let close_level: u32 = cap[4].parse().unwrap_or(1);
        if open_level != close_level {
            continue;
        }
        let attrs = &cap[2];
        let inner = &cap[3];

        let id = id_re
            .captures(attrs)
            .map(|c| c[1].to_string())
            .unwrap_or_default();
        let cleaned = heading_number_re.replace_all(inner, "");
        let text = tag_re.replace_all(&cleaned, "").trim().to_string();

        if open_level == 1 && first_h1.is_none() && !text.is_empty() {
            first_h1 = Some(text.clone());
        }
        if (open_level == 2 || open_level == 3) && !text.is_empty() {
            toc.push(TocItem {
                id,
                level: open_level,
                text,
            });
        }
    }

    // 7. Title resolution: @meta.title -> first H1 -> file stem
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
    let title = meta_title.or(first_h1).unwrap_or(stem);

    // 8. Section classification (e.g. "30-39 Knowledge/01 Dev" -> "30-39 Knowledge")
    let section = Path::new(rel_path)
        .parent()
        .and_then(|p| p.iter().next())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    // 9. Slug calculation: "foo/bar.tmt" -> "foo/bar"
    let slug = rel_path
        .trim_end_matches(".tmt")
        .trim_end_matches(".tm")
        .replace('\\', "/");

    // 10. Meta properties (colors, icon, banner, etc.)
    let mut primary_color: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut banner_url: Option<String> = None;
    let mut hero_chips = Vec::new();
    let mut infobox_rows = Vec::new();

    if let Some(serde_json::Value::Object(map)) = &meta_json {
        // Icon
        if let Some(val) = map.get("icon") {
            icon = val.as_str().map(|s| s.to_string());
        }

        // Primary Color
        if let Some(val) = map.get("colors") {
            if let Some(arr) = val.as_array() {
                if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                    primary_color = Some(first.trim_start_matches('#').to_string());
                }
            } else if let Some(s) = val.as_str() {
                primary_color = Some(s.trim_start_matches('#').to_string());
            }
        }

        // Banner Image
        if let Some(val) = map.get("banner") {
            if let Some(b) = val.as_str() {
                // If it's relative or starts with asset prefix
                if b.starts_with('/') {
                    banner_url = Some(b.to_string());
                } else {
                    banner_url = Some(format!("{}/{}", config.build.asset_prefix, b));
                }
            }
        }

        // Hero Chips based on config.ui.hero_chips
        for chip_cfg in &config.ui.hero_chips {
            if let Some(val) = map.get(&chip_cfg.key) {
                let text = match val {
                    serde_json::Value::String(s) => clean_ref_target(s),
                    serde_json::Value::Array(arr) => {
                        let parts: Vec<String> = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(clean_ref_target))
                            .collect();
                        parts.join("、")
                    }
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => continue,
                };

                if text.is_empty() {
                    continue;
                }

                let formatted_value = if chip_cfg.format.as_deref() == Some("birthday") {
                    let re = regex::Regex::new(r#"^(\d{4})-(\d{2})-(\d{2})"#).unwrap();
                    if let Some(caps) = re.captures(&text) {
                        let m: u32 = caps[2].parse().unwrap_or(0);
                        let d: u32 = caps[3].parse().unwrap_or(0);
                        format!("{m}月{d}日")
                    } else {
                        text
                    }
                } else {
                    text
                };

                hero_chips.push(HeroChipItem {
                    label: chip_cfg.label.clone(),
                    value: formatted_value,
                });
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
                    serde_json::Value::String(s) => clean_ref_target(s),
                    serde_json::Value::Array(arr) => {
                        let parts: Vec<String> = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(clean_ref_target))
                            .collect();
                        parts.join("、")
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

    let has_data = !infobox_rows.is_empty() || banner_url.is_some();

    Ok(ProcessedDoc {
        slug,
        rel_path: rel_path.to_string(),
        title,
        section,
        kind,
        primary_color,
        icon,
        banner_url,
        hero_chips,
        infobox_rows,
        has_data,
        toc,
        body_html: html,
    })
}
