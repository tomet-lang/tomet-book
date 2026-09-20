//! Turning a document's `@meta` block into the page's chrome: the hero chips,
//! the infobox rows, and the images the template hangs off them.

mod format;
mod icon;

#[cfg(test)]
mod tests;

use format::format_chip_value;

use icon::{extract_icon, extract_media_target};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

use super::html::{color_chip_html, external_link_html, is_external_url, sanitize_color_hex};
use super::links::{
    clean_ref_target, format_meta_value_to_html, resolve_meta_link, resolve_meta_media_path,
};
use crate::config::BookConfig;

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

/// Everything the page template reads out of `@meta`.
#[derive(Debug, Default)]
pub struct MetaProperties {
    /// Pages this document points at through its metadata (`parent: "[[unit]]"`
    /// and the like). Collected here because this is where those references are
    /// resolved; the body's own links are collected from the parsed document.
    pub linked_slugs: Vec<String>,
    pub primary_color: Option<String>,
    pub icon: Option<String>,
    pub icon_image_url: Option<String>,
    pub link_icon: Option<String>,
    pub banner_url: Option<String>,
    pub banner_y: Option<f64>,
    pub images: Vec<String>,
    pub hero_chips: Vec<HeroChipItem>,
    pub infobox_rows: Vec<InfoboxRowItem>,
}

impl MetaProperties {
    /// Whether the data pane has anything worth showing.
    pub fn has_data(&self) -> bool {
        !self.infobox_rows.is_empty() || self.banner_url.is_some() || !self.images.is_empty()
    }
}

/// Context shared by every extraction step below.
struct MetaCtx<'a> {
    from_path: &'a Path,
    vault_index: &'a tomet_links::VaultLinkIndex,
    config: &'a BookConfig,
    route_table: Option<&'a crate::book::slug::RouteTable>,
}

impl MetaCtx<'_> {
    fn media_path(&self, raw: &str) -> String {
        resolve_meta_media_path(
            raw,
            self.from_path,
            self.vault_index,
            self.config.build.clean_asset_prefix(),
        )
    }

    /// A meta value as `(href, label)`, with `None` for anything unlinkable.
    fn link(&self, raw: &str) -> (Option<String>, String) {
        match resolve_meta_link(
            raw,
            self.from_path,
            self.vault_index,
            self.config.build.clean_url_prefix(),
            self.route_table,
        ) {
            Some((href, label)) => (if href.is_empty() { None } else { Some(href) }, label),
            None => (None, clean_ref_target(raw)),
        }
    }

    fn value_html(&self, raw: &str) -> String {
        let unresolved_title = self
            .config
            .ui
            .strings
            .get("link.unresolved")
            .map(String::as_str)
            .unwrap_or("未作成のページ");
        format_meta_value_to_html(
            raw,
            self.from_path,
            self.vault_index,
            self.config.build.clean_url_prefix(),
            unresolved_title,
            self.route_table,
        )
    }

    fn resolve_link_val(&self, val: &Value) -> Option<MetaLinkInfo> {
        match val {
            Value::String(s) => {
                let (href, display) = self.link(s);
                Some(MetaLinkInfo {
                    href,
                    label: display,
                    raw: s.clone(),
                })
            }
            Value::Number(n) => Some(MetaLinkInfo {
                href: None,
                label: n.to_string(),
                raw: n.to_string(),
            }),
            Value::Object(map) => {
                let el_name = map.get("element").and_then(|v| v.as_str())?;
                if el_name != "link" && el_name != "embed" && el_name != "ref" {
                    return None;
                }
                let args = map.get("args")?;
                let (target, alias) = match args {
                    Value::String(s) => (Some(s.as_str()), None),
                    Value::Object(arg_map) => {
                        let target = arg_map
                            .get("target")
                            .or_else(|| arg_map.get("ref"))
                            .or_else(|| arg_map.get("path"))
                            .or_else(|| arg_map.get("src"))
                            .or_else(|| arg_map.get("url"))
                            .or_else(|| arg_map.get("href"))
                            .and_then(|v| v.as_str());
                        let alias = arg_map
                            .get("alias")
                            .or_else(|| arg_map.get("text"))
                            .or_else(|| arg_map.get("label"))
                            .and_then(|v| v.as_str());
                        (target, alias)
                    }
                    _ => (None, None),
                };
                let target = target?;
                if target.starts_with('/')
                    || target.starts_with("http://")
                    || target.starts_with("https://")
                {
                    let label = alias.map(str::to_string).unwrap_or_else(|| {
                        Path::new(target)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(target)
                            .to_string()
                    });
                    Some(MetaLinkInfo {
                        href: Some(target.to_string()),
                        label,
                        raw: target.to_string(),
                    })
                } else if let Some(unres) = target.strip_prefix("unresolved:") {
                    let label = alias
                        .map(str::to_string)
                        .unwrap_or_else(|| unres.to_string());
                    Some(MetaLinkInfo {
                        href: None,
                        label,
                        raw: target.to_string(),
                    })
                } else {
                    let raw = if let Some(alias) = alias {
                        format!("[[{target}|{alias}]]")
                    } else {
                        format!("@link(ref:{target})")
                    };
                    let (href, display) = self.link(&raw);
                    Some(MetaLinkInfo {
                        href,
                        label: display,
                        raw,
                    })
                }
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct MetaLinkInfo {
    href: Option<String>,
    label: String,
    raw: String,
}

/// Read the page's chrome out of `@meta`.
///
/// `kind` is the document's element kind, used as an infobox row when the
/// metadata does not already say what this page is.
pub fn extract(
    meta_json: Option<&Value>,
    kind: Option<&str>,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    config: &BookConfig,
    route_table: Option<&crate::book::slug::RouteTable>,
) -> MetaProperties {
    let Some(meta_val) = meta_json else {
        return MetaProperties::default();
    };
    let Value::Object(map) = meta_val else {
        return MetaProperties::default();
    };

    let cx = MetaCtx {
        from_path,
        vault_index,
        config,
        route_table,
    };

    let mut linked_slugs = Vec::new();
    for (_key, value) in map {
        let items: Vec<MetaLinkInfo> = match value {
            Value::Array(arr) => arr.iter().filter_map(|v| cx.resolve_link_val(v)).collect(),
            _ => cx.resolve_link_val(value).into_iter().collect(),
        };
        for item in items {
            let Some(href) = item.href else { continue };
            if let Some(slug) = super::links::slug_from_href(&href, config.build.clean_url_prefix())
                && !linked_slugs.contains(&slug)
            {
                linked_slugs.push(slug);
            }
        }
    }

    let (icon_html, icon_image_url, link_icon) = extract_icon(&cx, map);

    MetaProperties {
        linked_slugs,
        primary_color: primary_color(map),
        icon: icon_html,
        icon_image_url,
        link_icon,
        banner_url: map
            .get("banner")
            .and_then(extract_media_target)
            .map(|b| cx.media_path(&b)),
        banner_y: map
            .get("banner-y")
            .or_else(|| map.get("banner_y"))
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))),
        images: images(&cx, map),
        hero_chips: hero_chips(&cx, map),
        infobox_rows: infobox_rows(&cx, map, kind),
    }
}

/// Only well-formed hex reaches the template's inline `style` attributes.
fn primary_color(map: &Map<String, Value>) -> Option<String> {
    let val = map.get("colors")?;
    match val.as_array() {
        Some(arr) => arr
            .first()
            .and_then(|v| v.as_str())
            .and_then(sanitize_color_hex),
        None => val.as_str().and_then(sanitize_color_hex),
    }
}

fn images(cx: &MetaCtx, map: &Map<String, Value>) -> Vec<String> {
    let Some(val) = map.get("images") else {
        return Vec::new();
    };

    match val {
        Value::Array(arr) => arr
            .iter()
            .filter_map(extract_media_target)
            .map(|s| cx.media_path(&s))
            .collect(),
        _ => extract_media_target(val)
            .map(|s| vec![cx.media_path(&s)])
            .unwrap_or_default(),
    }
}

fn hero_chips(cx: &MetaCtx, map: &Map<String, Value>) -> Vec<HeroChipItem> {
    let mut chips = Vec::new();

    for chip_cfg in &cx.config.ui.hero_chips {
        let Some(val) = map.get(&chip_cfg.key) else {
            continue;
        };

        let push_item = |info: MetaLinkInfo, chips: &mut Vec<HeroChipItem>| {
            if !info.label.is_empty() {
                chips.push(HeroChipItem {
                    label: chip_cfg.label.clone(),
                    value: format_chip_value(
                        &info.label,
                        chip_cfg.format.as_deref(),
                        Some(&cx.config.book.lang),
                    ),
                    href: info.href,
                });
            }
        };

        match val {
            Value::Array(arr) => {
                for item in arr {
                    if let Some(info) = cx.resolve_link_val(item) {
                        push_item(info, &mut chips);
                    }
                }
            }
            _ => {
                if let Some(info) = cx.resolve_link_val(val) {
                    push_item(info, &mut chips);
                }
            }
        }
    }

    chips
}

fn infobox_rows(cx: &MetaCtx, map: &Map<String, Value>, kind: Option<&str>) -> Vec<InfoboxRowItem> {
    let infobox = &cx.config.ui.infobox;
    if !infobox.enable {
        return Vec::new();
    }

    let exclude: HashSet<&str> = infobox.exclude_keys.iter().map(String::as_str).collect();
    let label_for = |key: &str| {
        infobox
            .labels
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    };
    let mut rows = Vec::new();

    // The document's kind stands in for a `type` the metadata never gave.
    if let Some(k) = kind
        && !exclude.contains("kind")
        && !map.contains_key("type")
        && !map.contains_key("kind")
    {
        rows.push(InfoboxRowItem {
            label: infobox.labels.get("kind").cloned().unwrap_or_else(|| {
                if crate::config::is_english(&cx.config.book.lang) {
                    "Kind".to_string()
                } else {
                    "種別".to_string()
                }
            }),
            value: k.to_string(),
        });
    }

    for (key, val) in map {
        if exclude.contains(key.as_str()) {
            continue;
        }

        let Some(value) = row_value(cx, key, val) else {
            continue;
        };
        if !value.is_empty() {
            rows.push(InfoboxRowItem {
                label: label_for(key),
                value,
            });
        }
    }

    rows
}

/// Render one infobox cell. `None` for value shapes with nothing to show.
fn row_value(cx: &MetaCtx, key: &str, val: &Value) -> Option<String> {
    let is_url_key = key.starts_with("url.") || key == "url";

    if key == "colors" {
        return match val {
            Value::String(s) => Some(format!(
                r#"<div class="color-chips">{}</div>"#,
                color_chip_html(s)
            )),
            Value::Array(arr) => {
                let chips: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(color_chip_html))
                    .collect();
                Some(format!(
                    r#"<div class="color-chips">{}</div>"#,
                    chips.join("")
                ))
            }
            _ => None,
        };
    }

    if let Value::Bool(b) = val {
        return Some(if *b {
            "Yes".to_string()
        } else {
            "No".to_string()
        });
    }

    let render_info = |info: &MetaLinkInfo| -> String {
        if is_url_key && is_external_url(&info.raw) {
            external_link_html(&info.raw)
        } else if let Some(href) = &info.href {
            let label = super::html::escape_html(&info.label);
            let href = super::html::escape_html(href);
            format!(r#"<a href="{href}" class="tm-link tm-file">{label}</a>"#)
        } else if info.raw.starts_with("unresolved:") {
            let label = super::html::escape_html(&info.label);
            let unresolved_title = cx
                .config
                .ui
                .strings
                .get("link.unresolved")
                .map(String::as_str)
                .unwrap_or("未作成のページ");
            let title = super::html::escape_html(unresolved_title);
            format!(r#"<span class="tm-link tm-unresolved" title="{title}">{label}</span>"#)
        } else {
            cx.value_html(&info.raw)
        }
    };

    match val {
        Value::Array(arr) => {
            let rendered: Vec<String> = arr
                .iter()
                .filter_map(|v| cx.resolve_link_val(v))
                .map(|info| render_info(&info))
                .filter(|s| !s.is_empty())
                .collect();
            if rendered.is_empty() {
                None
            } else {
                Some(rendered.join("、"))
            }
        }
        _ => cx.resolve_link_val(val).map(|info| render_info(&info)),
    }
}
