use serde_json::{Map, Value};
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;

use super::MetaCtx;
use crate::book::document::html::escape_html;
use crate::book::document::icon as doc_icon;

/// Extract media reference target string (file name or path) from a metadata value.
pub(super) fn extract_media_target(val: &Value) -> Option<String> {
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
pub(super) fn is_image_target(s: &str) -> bool {
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
                escape_html(graphemes[0])
            )
        }
        2 => {
            let cells = graphemes
                .iter()
                .map(|g| format!(r#"<span class="avatar-cell">{}</span>"#, escape_html(g)))
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<span class="avatar-text avatar-grid avatar-grid-2">{cells}</span>"#)
        }
        3 => {
            let cells = graphemes
                .iter()
                .map(|g| format!(r#"<span class="avatar-cell">{}</span>"#, escape_html(g)))
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<span class="avatar-text avatar-grid avatar-grid-3">{cells}</span>"#)
        }
        4 => {
            let cells = graphemes
                .iter()
                .map(|g| format!(r#"<span class="avatar-cell">{}</span>"#, escape_html(g)))
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<span class="avatar-text avatar-grid avatar-grid-4">{cells}</span>"#)
        }
        _ => {
            let initials = graphemes[..2].join("");
            format!(
                r#"<span class="avatar-text avatar-text-2 avatar-initials" title="{}">{}</span>"#,
                escape_html(s),
                escape_html(&initials)
            )
        }
    }
}

pub(super) fn format_text_icon_link(s: &str) -> String {
    let graphemes: Vec<&str> = s.graphemes(true).collect();
    match graphemes.len() {
        0 => String::new(),
        1 => {
            let g = graphemes[0];
            let safe_g = escape_html(g);
            let cls = if g.is_ascii() {
                "tm-link-icon tm-link-icon-text"
            } else {
                "tm-link-icon tm-link-icon-emoji"
            };
            format!(r#"<span class="{cls}" aria-hidden="true">{safe_g}</span>"#)
        }
        _ => {
            let initials = graphemes[..2.min(graphemes.len())].join("");
            let safe_init = escape_html(&initials);
            format!(
                r#"<span class="tm-link-icon tm-link-icon-text" aria-hidden="true">{safe_init}</span>"#
            )
        }
    }
}

/// Supports:
/// - `@embed("avatar.png")` / `@link(...)` elements -> `<img ...>`
/// - Image file paths or URLs (`"avatar.png"`, `"https://..."`) -> `<img ...>`
/// - Plain string / emoji (`"🎂"`, `"cat"`) -> formatted avatar icon
/// - `@doc.icon(...)` falls through to None here and is handled by `icon::meta_icon_element`.
pub(super) fn extract_icon(
    cx: &MetaCtx,
    map: &Map<String, Value>,
) -> (Option<String>, Option<String>, Option<String>) {
    let Some(val) = map.get("icon") else {
        return (None, None, None);
    };

    match val {
        Value::String(s) => {
            let s_trimmed = s.trim();
            if let Some(lucide_name) = s_trimmed.strip_prefix("lucide:")
                && doc_icon::is_lucide_icon(lucide_name)
            {
                let link_icon = doc_icon::render_link_icon(lucide_name, "lucide");
                let doc_icon = format!(
                    r#"<svg class="tm-doc-icon" data-pkg="lucide" data-icon="{}" aria-hidden="true"><use href="/icons/lucide.svg#{}"></use></svg>"#,
                    escape_html(lucide_name),
                    escape_html(lucide_name),
                );
                return (Some(doc_icon), None, link_icon);
            }
            if let Some(simple_name) = s_trimmed.strip_prefix("simple:")
                && doc_icon::is_simple_icon(simple_name)
            {
                let link_icon = doc_icon::render_link_icon(simple_name, "simple");
                let doc_icon = format!(
                    r#"<svg class="tm-doc-icon" data-pkg="simple" data-icon="{}" aria-hidden="true"><use href="/icons/simple.svg#{}"></use></svg>"#,
                    escape_html(simple_name),
                    escape_html(simple_name),
                );
                return (Some(doc_icon), None, link_icon);
            }

            if is_image_target(s) || s.starts_with("http://") || s.starts_with("https://") {
                let resolved = cx.media_path(s);
                let safe = escape_html(&resolved);
                let html = format!(
                    r#"<img class="tm-doc-icon-img" src="{safe}" alt="icon" loading="lazy">"#
                );
                let link_icon = format!(
                    r#"<img class="tm-link-icon tm-link-favicon tm-link-icon-img" src="{safe}" alt="" width="14" height="14" loading="lazy" decoding="async" />"#
                );
                (Some(html), Some(resolved), Some(link_icon))
            } else {
                (
                    Some(format_text_icon(s)),
                    None,
                    Some(format_text_icon_link(s)),
                )
            }
        }
        Value::Object(obj) => {
            let el_name = obj.get("element").and_then(|v| v.as_str());
            if el_name == Some("embed") || el_name == Some("link") || el_name == Some("ref") {
                if let Some(target) = extract_media_target(val) {
                    let resolved = cx.media_path(&target);
                    let safe = escape_html(&resolved);
                    let html = format!(
                        r#"<img class="tm-doc-icon-img" src="{safe}" alt="icon" loading="lazy">"#
                    );
                    let link_icon = format!(
                        r#"<img class="tm-link-icon tm-link-favicon tm-link-icon-img" src="{safe}" alt="" width="14" height="14" loading="lazy" decoding="async" />"#
                    );
                    (Some(html), Some(resolved), Some(link_icon))
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            }
        }
        _ => (None, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
