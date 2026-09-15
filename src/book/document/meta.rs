//! Turning a document's `@meta` block into the page's chrome: the hero chips,
//! the infobox rows, and the images the template hangs off them.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

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
        let raws: Vec<&str> = match value {
            Value::String(s) => vec![s.as_str()],
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
            _ => continue,
        };
        for raw in raws {
            let (href, _) = cx.link(raw);
            let Some(href) = href else { continue };
            if let Some(slug) = super::links::slug_from_href(&href, config.build.clean_url_prefix())
                && !linked_slugs.contains(&slug)
            {
                linked_slugs.push(slug);
            }
        }
    }

    MetaProperties {
        linked_slugs,
        primary_color: primary_color(map),
        icon: map.get("icon").and_then(|v| v.as_str()).map(str::to_string),
        banner_url: map
            .get("banner")
            .and_then(|v| v.as_str())
            .map(|b| cx.media_path(b)),
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

    match val.as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|item| item.as_str())
            .map(|s| cx.media_path(s))
            .collect(),
        None => val
            .as_str()
            .map(|s| vec![cx.media_path(s)])
            .unwrap_or_default(),
    }
}

fn hero_chips(cx: &MetaCtx, map: &Map<String, Value>) -> Vec<HeroChipItem> {
    let mut chips = Vec::new();

    for chip_cfg in &cx.config.ui.hero_chips {
        let Some(val) = map.get(&chip_cfg.key) else {
            continue;
        };

        let push_text = |raw: &str, chips: &mut Vec<HeroChipItem>| {
            let (href, display) = cx.link(raw);
            if !display.is_empty() {
                chips.push(HeroChipItem {
                    label: chip_cfg.label.clone(),
                    value: format_chip_value(
                        &display,
                        chip_cfg.format.as_deref(),
                        Some(&cx.config.book.lang),
                    ),
                    href,
                });
            }
        };

        match val {
            Value::String(s) => push_text(s, &mut chips),
            Value::Array(arr) => {
                for item in arr.iter().filter_map(|v| v.as_str()) {
                    push_text(item, &mut chips);
                }
            }
            Value::Number(n) => chips.push(HeroChipItem {
                label: chip_cfg.label.clone(),
                value: n.to_string(),
                href: None,
            }),
            _ => continue,
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
    let scalar = |s: &str| {
        if is_url_key && is_external_url(s) {
            external_link_html(s)
        } else {
            cx.value_html(s)
        }
    };

    Some(match val {
        Value::String(s) if key == "colors" => {
            format!(r#"<div class="color-chips">{}</div>"#, color_chip_html(s))
        }
        Value::String(s) => scalar(s),
        Value::Array(arr) if key == "colors" => {
            let chips: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(color_chip_html))
                .collect();
            format!(r#"<div class="color-chips">{}</div>"#, chips.join(""))
        }
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(&scalar))
            .collect::<Vec<_>>()
            .join("、"),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "Yes".to_string(),
        Value::Bool(false) => "No".to_string(),
        _ => return None,
    })
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
}
