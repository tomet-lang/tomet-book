//! Turning a document's `@meta` block into the page's chrome: the hero chips,
//! the infobox rows, and the images the template hangs off them.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

use super::html::{color_chip_html, external_link_html, is_external_url, sanitize_color_hex};
use super::links::{
    clean_ref_target, format_meta_value_to_html, resolve_meta_link, resolve_meta_media_path,
};
use crate::config::BookConfig;

// Compiled once: this runs per document, in parallel.
static ISO_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^(\d{4})-(\d{2})-(\d{2})"#).unwrap());

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

pub fn format_chip_value(val: &str, format: Option<&str>, lang: Option<&str>) -> String {
    if format == Some("birthday")
        && let Some(caps) = ISO_DATE_RE.captures(val)
    {
        let m: u32 = caps[2].parse().unwrap_or(0);
        let d: u32 = caps[3].parse().unwrap_or(0);
        if let Some(l) = lang
            && crate::config::is_english(l)
        {
            const MONTHS_EN: [&str; 12] = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            if (1..=12).contains(&m) {
                return format!("{} {d}", MONTHS_EN[(m - 1) as usize]);
            }
        }
        return format!("{m}月{d}日");
    }
    val.to_string()
}

/// Extract media reference target string (file name or path) from a metadata value.
fn extract_media_target(val: &Value) -> Option<String> {
    match val {
        Value::String(s) => Some(s.clone()),
        Value::Object(map) => {
            let el_name = map.get("element").and_then(|v| v.as_str())?;
            if el_name == "link" || el_name == "embed" || el_name == "ref" {
                let args = map.get("args")?;
                match args {
                    Value::String(s) => Some(s.clone()),
                    Value::Object(arg_map) => arg_map
                        .get("target")
                        .or_else(|| arg_map.get("ref"))
                        .or_else(|| arg_map.get("path"))
                        .or_else(|| arg_map.get("src"))
                        .or_else(|| arg_map.get("url"))
                        .or_else(|| arg_map.get("href"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper to test if a string represents an image target (by file extension or scheme).
fn is_image_target(s: &str) -> bool {
    let s_clean = s
        .split('?')
        .next()
        .unwrap_or(s)
        .split('#')
        .next()
        .unwrap_or(s);
    let ext = Path::new(s_clean)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" | "ico")
    )
}

/// Extracts page icon and optional image URL for optimization.
///
/// Formats a plain text or emoji icon string according to avatar display rules:
/// - 1 char: single large glyph (`avatar-text-1`)
/// - 2 chars: horizontally auto-scaled without wrapping (`avatar-text-2`)
/// - 3 chars: 2x2 grid with 3rd item centered on bottom row (`avatar-grid avatar-grid-3`)
/// - 4 chars: 2x2 grid (`avatar-grid avatar-grid-4`)
/// - 5+ chars: initials (first 2 graphemes rendered with `avatar-text-2 avatar-initials`)
pub fn format_text_icon(s: &str) -> String {
    let graphemes: Vec<&str> = s.graphemes(true).collect();
    match graphemes.len() {
        0 => String::new(),
        1 => {
            format!(
                r#"<span class="avatar-text avatar-text-1">{}</span>"#,
                super::html::escape_html(graphemes[0])
            )
        }
        2 => {
            let cells = graphemes
                .iter()
                .map(|g| {
                    format!(
                        r#"<span class="avatar-cell">{}</span>"#,
                        super::html::escape_html(g)
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<span class="avatar-text avatar-grid avatar-grid-2">{cells}</span>"#)
        }
        3 => {
            let cells = graphemes
                .iter()
                .map(|g| {
                    format!(
                        r#"<span class="avatar-cell">{}</span>"#,
                        super::html::escape_html(g)
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<span class="avatar-text avatar-grid avatar-grid-3">{cells}</span>"#)
        }
        4 => {
            let cells = graphemes
                .iter()
                .map(|g| {
                    format!(
                        r#"<span class="avatar-cell">{}</span>"#,
                        super::html::escape_html(g)
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<span class="avatar-text avatar-grid avatar-grid-4">{cells}</span>"#)
        }
        _ => {
            let initials = graphemes[..2].join("");
            format!(
                r#"<span class="avatar-text avatar-text-2 avatar-initials" title="{}">{}</span>"#,
                super::html::escape_html(s),
                super::html::escape_html(&initials)
            )
        }
    }
}

/// Supports:
/// - `@embed("avatar.png")` / `@link(...)` elements -> `<img ...>`
/// - Image file paths or URLs (`"avatar.png"`, `"https://..."`) -> `<img ...>`
/// - Plain string / emoji (`"🎂"`, `"cat"`) -> formatted avatar icon
/// - `@doc.icon(...)` falls through to None here and is handled by `icon::meta_icon_element`.
fn extract_icon(cx: &MetaCtx, map: &Map<String, Value>) -> (Option<String>, Option<String>) {
    let Some(val) = map.get("icon") else {
        return (None, None);
    };

    match val {
        Value::String(s) => {
            if is_image_target(s) || s.starts_with("http://") || s.starts_with("https://") {
                let resolved = cx.media_path(s);
                let html = format!(
                    r#"<img class="tm-doc-icon-img" src="{}" alt="icon" loading="lazy">"#,
                    super::html::escape_html(&resolved)
                );
                (Some(html), Some(resolved))
            } else {
                (Some(format_text_icon(s)), None)
            }
        }
        Value::Object(obj) => {
            let el_name = obj.get("element").and_then(|v| v.as_str());
            if el_name == Some("embed") || el_name == Some("link") || el_name == Some("ref") {
                if let Some(target) = extract_media_target(val) {
                    let resolved = cx.media_path(&target);
                    let html = format!(
                        r#"<img class="tm-doc-icon-img" src="{}" alt="icon" loading="lazy">"#,
                        super::html::escape_html(&resolved)
                    );
                    (Some(html), Some(resolved))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
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
) -> MetaProperties {
    let Some(Value::Object(map)) = meta_json else {
        return MetaProperties::default();
    };

    let cx = MetaCtx {
        from_path,
        vault_index,
        config,
    };

    let mut linked_slugs = Vec::new();
    for value in map.values() {
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

    let (icon_html, icon_image_url) = extract_icon(&cx, map);

    MetaProperties {
        linked_slugs,
        primary_color: primary_color(map),
        icon: icon_html,
        icon_image_url,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_config() -> BookConfig {
        BookConfig::default()
    }

    fn index() -> tomet_links::VaultLinkIndex {
        tomet_links::VaultLinkIndex::from_paths(["30-39 Knowledge/rust.tmt"])
    }

    fn extract_from(json: &str, kind: Option<&str>) -> MetaProperties {
        let value: Value = serde_json::from_str(json).unwrap();
        extract(
            Some(&value),
            kind,
            Path::new("notes/a.tmt"),
            &index(),
            &ctx_config(),
        )
    }

    #[test]
    fn birthday_format_shortens_an_iso_date() {
        assert_eq!(
            format_chip_value("2001-04-09", Some("birthday"), None),
            "4月9日"
        );
        assert_eq!(
            format_chip_value("2001-04-09T00:00:00", Some("birthday"), None),
            "4月9日"
        );
        assert_eq!(
            format_chip_value("2001-04-09", Some("birthday"), Some("en")),
            "April 9"
        );
        assert_eq!(
            format_chip_value("2001-12-25", Some("birthday"), Some("en-US")),
            "December 25"
        );
    }

    #[test]
    fn birthday_format_leaves_unparsable_values_alone() {
        assert_eq!(
            format_chip_value("春ごろ", Some("birthday"), None),
            "春ごろ"
        );
        assert_eq!(format_chip_value("2001-04-09", None, None), "2001-04-09");
    }

    #[test]
    fn no_metadata_yields_nothing_to_show() {
        let props = extract(None, None, Path::new("a.tmt"), &index(), &ctx_config());
        assert!(!props.has_data());
        assert!(props.infobox_rows.is_empty());
    }

    #[test]
    fn only_hex_colours_become_the_primary_colour() {
        assert_eq!(
            extract_from(r##"{"colors": ["#A1B2C3", "#000"]}"##, None)
                .primary_color
                .as_deref(),
            Some("a1b2c3")
        );
        assert_eq!(
            extract_from(r#"{"colors": "red"}"#, None).primary_color,
            None
        );
    }

    #[test]
    fn media_references_resolve_under_the_asset_prefix() {
        let props = extract_from(
            r#"{"banner": "hero.png", "images": ["a.png", "b.png"]}"#,
            None,
        );
        assert_eq!(props.banner_url.as_deref(), Some("/vault/hero.png"));
        assert_eq!(props.images, vec!["/vault/a.png", "/vault/b.png"]);
        assert!(props.has_data());
    }

    #[test]
    fn banner_y_accepts_both_spellings_and_both_number_types() {
        assert_eq!(
            extract_from(r#"{"banner-y": 11}"#, None).banner_y,
            Some(11.0)
        );
        assert_eq!(
            extract_from(r#"{"banner_y": 12.5}"#, None).banner_y,
            Some(12.5)
        );
    }

    #[test]
    fn hero_chips_follow_the_configured_keys() {
        // The default config asks for parent, type.list-a, type.element-a, birth.
        let props = extract_from(
            r#"{"parent": "[[rust]]", "birth": "2001-04-09", "ignored": "x"}"#,
            None,
        );
        let values: Vec<&str> = props.hero_chips.iter().map(|c| c.value.as_str()).collect();
        assert_eq!(values, vec!["rust", "4月9日"]);
        assert_eq!(
            props.hero_chips[0].href.as_deref(),
            Some("/wiki/30-39 Knowledge/rust")
        );
    }

    #[test]
    fn an_array_meta_value_becomes_one_chip_per_entry() {
        let props = extract_from(r#"{"parent": ["[[rust]]", "other"]}"#, None);
        assert_eq!(props.hero_chips.len(), 2);
        assert_eq!(props.hero_chips[1].href, None);
    }

    #[test]
    fn the_document_kind_fills_in_for_a_missing_type() {
        let rows = extract_from(r#"{"place": "東京"}"#, Some("person")).infobox_rows;
        assert!(
            rows.iter()
                .any(|r| r.label == "種別" && r.value == "person")
        );
    }

    #[test]
    fn an_explicit_type_wins_over_the_document_kind() {
        let rows = extract_from(r#"{"type": "キャラクター"}"#, Some("person")).infobox_rows;
        let kinds: Vec<&str> = rows
            .iter()
            .filter(|r| r.label == "種別")
            .map(|r| r.value.as_str())
            .collect();
        assert_eq!(kinds, vec!["キャラクター"]);
    }

    #[test]
    fn excluded_keys_never_reach_the_infobox() {
        // `icon` and `banner` are chrome, not rows.
        let rows = extract_from(
            r#"{"icon": "🎂", "banner": "x.png", "place": "東京"}"#,
            None,
        )
        .infobox_rows;
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["出身地"]);
    }

    #[test]
    fn external_urls_become_links_and_arrays_are_joined() {
        let rows = extract_from(
            r#"{"url.wiki": "https://example.com", "aliases": ["A", "B"]}"#,
            None,
        )
        .infobox_rows;

        let wiki = rows.iter().find(|r| r.label == "Wikipedia").unwrap();
        assert!(wiki.value.contains(r#"href="https://example.com""#));

        let aliases = rows.iter().find(|r| r.label == "別名 / 愛称").unwrap();
        assert_eq!(aliases.value, "A、B");
    }

    #[test]
    fn booleans_and_numbers_render_as_text() {
        let rows = extract_from(r#"{"rating": 5, "flags": true}"#, None).infobox_rows;
        assert!(rows.iter().any(|r| r.value == "5"));
        assert!(rows.iter().any(|r| r.value == "Yes"));
    }

    #[test]
    fn element_link_in_banner_and_images_resolves() {
        let props = extract_from(
            r#"{
                "banner": {"element": "link", "args": {"ref": "+771a.svg"}},
                "images": [
                    {"element": "link", "args": {"ref": "a.png"}},
                    {"element": "link", "args": "b.png"}
                ]
            }"#,
            None,
        );
        assert_eq!(props.banner_url.as_deref(), Some("/vault/+771a.svg"));
        assert_eq!(props.images, vec!["/vault/a.png", "/vault/b.png"]);
        assert!(props.has_data());
    }

    #[test]
    fn element_link_in_hero_chips_and_infobox_rows() {
        let props = extract_from(
            r#"{
                "parent": {"element": "link", "args": {"ref": "rust"}},
                "author": {"element": "link", "args": {"target": "rust", "alias": "Rust Lang"}}
            }"#,
            None,
        );
        assert_eq!(props.hero_chips.len(), 1);
        assert_eq!(props.hero_chips[0].value, "rust");
        assert_eq!(
            props.hero_chips[0].href.as_deref(),
            Some("/wiki/30-39 Knowledge/rust")
        );
        assert!(
            props
                .linked_slugs
                .contains(&"30-39 Knowledge/rust".to_string())
        );

        let author_row = props
            .infobox_rows
            .iter()
            .find(|r| r.label == "author")
            .unwrap();
        assert!(
            author_row
                .value
                .contains(r#"href="/wiki/30-39 Knowledge/rust""#)
        );
        assert!(author_row.value.contains("Rust Lang"));
    }

    #[test]
    fn icon_as_embed_or_link_or_image_path_resolves() {
        let props_embed = extract_from(
            r#"{"icon": {"element": "embed", "args": {"target": "avatar.png"}}}"#,
            None,
        );
        assert_eq!(
            props_embed.icon.as_deref(),
            Some(
                r#"<img class="tm-doc-icon-img" src="/vault/avatar.png" alt="icon" loading="lazy">"#
            )
        );
        assert_eq!(
            props_embed.icon_image_url.as_deref(),
            Some("/vault/avatar.png")
        );

        let props_link = extract_from(
            r#"{"icon": {"element": "link", "args": "avatar.png"}}"#,
            None,
        );
        assert_eq!(
            props_link.icon.as_deref(),
            Some(
                r#"<img class="tm-doc-icon-img" src="/vault/avatar.png" alt="icon" loading="lazy">"#
            )
        );

        let props_str = extract_from(r#"{"icon": "profile.jpg"}"#, None);
        assert_eq!(
            props_str.icon.as_deref(),
            Some(
                r#"<img class="tm-doc-icon-img" src="/vault/profile.jpg" alt="icon" loading="lazy">"#
            )
        );

        let props_emoji = extract_from(r#"{"icon": "🎂"}"#, None);
        assert_eq!(
            props_emoji.icon.as_deref(),
            Some(r#"<span class="avatar-text avatar-text-1">🎂</span>"#)
        );
        assert_eq!(props_emoji.icon_image_url, None);
    }

    #[test]
    fn format_text_icon_handles_different_lengths() {
        assert_eq!(
            format_text_icon("🎂"),
            r#"<span class="avatar-text avatar-text-1">🎂</span>"#
        );
        assert_eq!(
            format_text_icon("🐾🩵"),
            r#"<span class="avatar-text avatar-grid avatar-grid-2"><span class="avatar-cell">🐾</span><span class="avatar-cell">🩵</span></span>"#
        );
        assert_eq!(
            format_text_icon("🐾🩵🔥"),
            r#"<span class="avatar-text avatar-grid avatar-grid-3"><span class="avatar-cell">🐾</span><span class="avatar-cell">🩵</span><span class="avatar-cell">🔥</span></span>"#
        );
        assert_eq!(
            format_text_icon("🐾🩵🦀🔥"),
            r#"<span class="avatar-text avatar-grid avatar-grid-4"><span class="avatar-cell">🐾</span><span class="avatar-cell">🩵</span><span class="avatar-cell">🦀</span><span class="avatar-cell">🔥</span></span>"#
        );
        assert_eq!(
            format_text_icon("Design"),
            r#"<span class="avatar-text avatar-text-2 avatar-initials" title="Design">De</span>"#
        );
    }
}
