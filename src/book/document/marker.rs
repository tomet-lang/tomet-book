//! List marker processing: enhances `tomet-html`'s `<span class="tm-list-marker">`
//! into Lucide SVG icons (for checkboxes and task markers) or clean badge pills (for
//! timestamps, tags, and unmapped text).

use regex::Regex;
use std::sync::LazyLock;

use super::html::escape_html;
use super::icon::is_lucide_icon;
use crate::config::MarkersConfig;

static LI_MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    // The trailing ` ?` swallows the one space `tomet-html` always emits
    // after `</span>` to separate the marker from the text that follows
    // it -- needed there as the plain-text default, but redundant once
    // the marker becomes an icon or badge here: both `.tm-list-marker-icon`
    // and `.tm-list-marker-badge` already carry their own `margin-right`
    // (`60-content.css`), so keeping the literal space too doubles the gap.
    Regex::new(r#"(?s)<li([^>]*)>(\s*)<span class="tm-list-marker"([^>]*)>(.*?)</span> ?"#)
        .expect("valid regex for list item marker")
});

static DATA_MARKER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"data-marker="([^"]*)""#).expect("valid regex for data-marker"));

enum ResolvedMarker {
    Icon {
        icon: String,
        color: Option<String>,
        pkg: String,
        is_done: bool,
    },
    Badge {
        html: String,
    },
}

fn extract_data_marker(attrs: &str) -> Option<&str> {
    DATA_MARKER_RE
        .captures(attrs)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
}

fn strip_brackets(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

pub(crate) fn normalize_lucide_name(name: &str) -> &str {
    match name {
        "check-square" => "square-check",
        "minus-circle" => "circle-minus",
        "alert-triangle" => "triangle-alert",
        "help-circle" | "circle-help" => "circle-question-mark",
        "arrow-right-circle" => "circle-arrow-right",
        other => other,
    }
}

fn resolve_marker(raw_val: &str, content: &str, config: &MarkersConfig) -> ResolvedMarker {
    // When the feature is disabled, render everything as a clean pill badge.
    if !config.enable {
        let badge_content = if !content.trim().is_empty() {
            strip_brackets(content)
        } else {
            raw_val
        };
        return ResolvedMarker::Badge {
            html: badge_content.to_string(),
        };
    }

    // 1. Empty markers: `- ()` or `- ( )` -> square (todo checkbox)
    if raw_val.trim().is_empty() && (content.trim().is_empty() || content.trim() == "[ ]") {
        return ResolvedMarker::Icon {
            icon: "square".to_string(),
            color: None,
            pkg: "lucide".to_string(),
            is_done: false,
        };
    }

    let trimmed = raw_val.trim();

    // 2. Custom definitions from tmtbook.toml ([ui.markers.custom])
    if let Some(entry) = config.custom.get(trimmed) {
        let is_done = trimmed.eq_ignore_ascii_case("x") || trimmed.eq_ignore_ascii_case("done");
        let norm_icon = normalize_lucide_name(entry.icon());
        return ResolvedMarker::Icon {
            icon: norm_icon.to_string(),
            color: entry.color().map(str::to_string),
            pkg: entry.pkg().to_string(),
            is_done,
        };
    }

    // 3. Built-in presets (Obsidian-style alternate checkboxes)
    match trimmed {
        "x" | "X" | "done" => {
            return ResolvedMarker::Icon {
                icon: "square-check".to_string(),
                color: Some("var(--marker-done-color, #22c55e)".to_string()),
                pkg: "lucide".to_string(),
                is_done: true,
            };
        }
        "/" | "wip" => {
            return ResolvedMarker::Icon {
                icon: "clock".to_string(),
                color: Some("#eab308".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        "-" | "canceled" | "cancel" => {
            return ResolvedMarker::Icon {
                icon: "circle-minus".to_string(),
                color: Some("var(--text-muted)".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        "!" | "important" => {
            return ResolvedMarker::Icon {
                icon: "triangle-alert".to_string(),
                color: Some("#ef4444".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        "?" | "question" => {
            return ResolvedMarker::Icon {
                icon: "circle-question-mark".to_string(),
                color: Some("#3b82f6".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        "i" | "info" => {
            return ResolvedMarker::Icon {
                icon: "info".to_string(),
                color: Some("#3b82f6".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        "*" | "star" => {
            return ResolvedMarker::Icon {
                icon: "star".to_string(),
                color: Some("#f59e0b".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        ">" | "forward" => {
            return ResolvedMarker::Icon {
                icon: "circle-arrow-right".to_string(),
                color: Some("var(--text-muted)".to_string()),
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        "todo" => {
            return ResolvedMarker::Icon {
                icon: "square".to_string(),
                color: None,
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
        _ => {}
    }

    // 4. Namespaced icon name: `lucide.name`
    if let Some(lucide_name) = trimmed.strip_prefix("lucide.") {
        let norm = normalize_lucide_name(lucide_name);
        if is_lucide_icon(norm) {
            return ResolvedMarker::Icon {
                icon: norm.to_string(),
                color: None,
                pkg: "lucide".to_string(),
                is_done: false,
            };
        }
    }

    // 5. Bare icon name in Lucide sprite (e.g. `flame`, `bug`, `triangle-alert`, etc.)
    let norm = normalize_lucide_name(trimmed);
    if is_lucide_icon(norm) {
        return ResolvedMarker::Icon {
            icon: norm.to_string(),
            color: None,
            pkg: "lucide".to_string(),
            is_done: false,
        };
    }

    // 6. Fallback: clean pill badge for timestamps, tags, numbers, or inline elements
    let badge_content = if !content.trim().is_empty() {
        strip_brackets(content)
    } else {
        trimmed
    };
    ResolvedMarker::Badge {
        html: badge_content.to_string(),
    }
}

fn add_class_to_attrs(attrs: &str, new_class: &str) -> String {
    if let Some(pos) = attrs.find("class=\"") {
        let insert_pos = pos + "class=\"".len();
        format!(
            "{}{new_class} {}",
            &attrs[..insert_pos],
            &attrs[insert_pos..]
        )
    } else {
        format!(" class=\"{new_class}\"{attrs}")
    }
}

/// Enhances list markers in the rendered HTML:
/// - Replaces checkbox/icon markers with Lucide SVGs (if `config.enable` is true)
/// - Replaces raw bracketed text markers `[xxx]` with pill badges
/// - Injects task item classes onto enclosing `<li>`s
pub fn enhance_list_markers(body_html: &str, config: &MarkersConfig) -> String {
    LI_MARKER_RE
        .replace_all(body_html, |caps: &regex::Captures| {
            let li_attrs = &caps[1];
            let ws = &caps[2];
            let span_attrs = &caps[3];
            let span_content = &caps[4];

            let raw_val = extract_data_marker(span_attrs).unwrap_or("");
            let resolved = resolve_marker(raw_val, span_content, config);

            match resolved {
                ResolvedMarker::Icon {
                    icon,
                    color,
                    pkg,
                    is_done,
                } => {
                    let mut li_classes = vec!["tm-task-item"];
                    if is_done {
                        li_classes.push("tm-task-done");
                        if config.strikethrough {
                            li_classes.push("tm-strikethrough");
                        }
                    }
                    let new_li_attrs = add_class_to_attrs(li_attrs, &li_classes.join(" "));

                    let done_marker_class = if is_done { " tm-marker-done" } else { "" };
                    let style_attr = match color {
                        Some(ref c) => format!(" style=\"--marker-color: {c};\""),
                        None => String::new(),
                    };
                    let safe_icon = escape_html(&icon);
                    let safe_pkg = escape_html(&pkg);
                    let safe_marker_attr = if !raw_val.is_empty() {
                        format!(" data-marker=\"{}\"", escape_html(raw_val))
                    } else {
                        String::new()
                    };

                    format!(
                        "<li{new_li_attrs}>{ws}<span class=\"tm-list-marker tm-list-marker-icon{done_marker_class}\"{safe_marker_attr}{style_attr}><svg class=\"tm-doc-icon\" aria-hidden=\"true\"><use href=\"/icons/{safe_pkg}.svg#{safe_icon}\"></use></svg></span>"
                    )
                }
                ResolvedMarker::Badge { html } => {
                    let new_li_attrs = add_class_to_attrs(li_attrs, "tm-list-item-with-badge");
                    let safe_marker_attr = if !raw_val.is_empty() {
                        format!(" data-marker=\"{}\"", escape_html(raw_val))
                    } else {
                        String::new()
                    };

                    format!(
                        "<li{new_li_attrs}>{ws}<span class=\"tm-list-marker tm-list-marker-badge\"{safe_marker_attr}>{html}</span>"
                    )
                }
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MarkerEntryConfig;
    use std::collections::HashMap;

    #[test]
    fn disabled_by_default_renders_all_markers_as_badges_without_icons_or_strikethrough() {
        let config = MarkersConfig::default();
        let html = "<ul>\n<li><span class=\"tm-list-marker\" data-marker=\"x\">[x]</span> task</li>\n<li><span class=\"tm-list-marker\" data-marker=\"12:01\">[12:01]</span> breakfast</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(
            !out.contains("<svg"),
            "must not contain svg icon when disabled"
        );
        assert!(!out.contains("tm-task-done"), "must not have done class");
        assert!(out.contains("<li class=\"tm-list-item-with-badge\">"));
        assert!(out.contains(
            "<span class=\"tm-list-marker tm-list-marker-badge\" data-marker=\"x\">x</span>"
        ));
        assert!(out.contains(
            "<span class=\"tm-list-marker tm-list-marker-badge\" data-marker=\"12:01\">12:01</span>"
        ));
    }

    #[test]
    fn enabled_renders_empty_marker_as_square_checkbox() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom: HashMap::new(),
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\"></span> todo item</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(out.contains("class=\"tm-task-item\""));
        assert!(out.contains("<use href=\"/icons/lucide.svg#square\"></use>"));
    }

    #[test]
    fn enabled_renders_x_as_done_checkbox_with_strikethrough() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom: HashMap::new(),
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\" data-marker=\"x\">[x]</span> completed task</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(out.contains("class=\"tm-task-item tm-task-done tm-strikethrough\""));
        assert!(out.contains("<use href=\"/icons/lucide.svg#square-check\"></use>"));
        assert!(out.contains("tm-marker-done"));
    }

    #[test]
    fn enabled_renders_done_without_strikethrough_when_disabled_in_config() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: false,
            custom: HashMap::new(),
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\" data-marker=\"x\">[x]</span> completed task</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(out.contains("class=\"tm-task-item tm-task-done\""));
        assert!(!out.contains("tm-strikethrough"));
        assert!(out.contains("<use href=\"/icons/lucide.svg#square-check\"></use>"));
    }

    #[test]
    fn enabled_renders_builtin_alternate_markers() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom: HashMap::new(),
        };
        let html = r#"<ul>
<li><span class="tm-list-marker" data-marker="/">[/]</span> in progress</li>
<li><span class="tm-list-marker" data-marker="-">[-]</span> canceled</li>
<li><span class="tm-list-marker" data-marker="!">!]</span> important</li>
<li><span class="tm-list-marker" data-marker="?">[?]</span> question</li>
<li><span class="tm-list-marker" data-marker="*">[*]</span> star</li>
</ul>"#;
        let out = enhance_list_markers(html, &config);

        assert!(out.contains("<use href=\"/icons/lucide.svg#clock\"></use>"));
        assert!(out.contains("<use href=\"/icons/lucide.svg#circle-minus\"></use>"));
        assert!(out.contains("<use href=\"/icons/lucide.svg#triangle-alert\"></use>"));
        assert!(out.contains("<use href=\"/icons/lucide.svg#circle-question-mark\"></use>"));
        assert!(out.contains("<use href=\"/icons/lucide.svg#star\"></use>"));
    }

    #[test]
    fn enabled_supports_direct_lucide_icon_and_namespaced_icon() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom: HashMap::new(),
        };
        let html = r#"<ul>
<li><span class="tm-list-marker" data-marker="flame">[flame]</span> hot</li>
<li><span class="tm-list-marker" data-marker="lucide.bug">[lucide.bug]</span> bug</li>
</ul>"#;
        let out = enhance_list_markers(html, &config);

        assert!(out.contains("<use href=\"/icons/lucide.svg#flame\"></use>"));
        assert!(out.contains("<use href=\"/icons/lucide.svg#bug\"></use>"));
    }

    #[test]
    fn unmapped_text_becomes_a_badge_even_when_enabled() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom: HashMap::new(),
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\" data-marker=\"12:01\">[12:01]</span> breakfast</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(!out.contains("<svg"));
        assert!(out.contains("<li class=\"tm-list-item-with-badge\">"));
        assert!(out.contains(
            "<span class=\"tm-list-marker tm-list-marker-badge\" data-marker=\"12:01\">12:01</span>"
        ));
    }

    #[test]
    fn custom_marker_overrides_builtin() {
        let mut custom = HashMap::new();
        custom.insert(
            "fire".to_string(),
            MarkerEntryConfig::Detailed {
                icon: "flame".to_string(),
                color: Some("#f97316".to_string()),
                pkg: None,
            },
        );
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom,
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\" data-marker=\"fire\">[fire]</span> fire item</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(out.contains("<use href=\"/icons/lucide.svg#flame\"></use>"));
        assert!(out.contains("style=\"--marker-color: #f97316;\""));
    }

    /// `tomet-html` always emits one literal space after `</span>` to
    /// separate the marker from the text that follows -- needed there as
    /// the plain-text default, but redundant once the marker becomes an
    /// icon or badge here, since both already carry their own
    /// `margin-right` in `60-content.css`. Keeping the space too would
    /// double the visual gap.
    #[test]
    fn icon_marker_drops_the_redundant_trailing_space() {
        let mut custom = HashMap::new();
        custom.insert(
            "idea".to_string(),
            MarkerEntryConfig::Detailed {
                icon: "lightbulb".to_string(),
                color: Some("#ffee00".to_string()),
                pkg: None,
            },
        );
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom,
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\" data-marker=\"idea\">[idea]</span> content</li>\n</ul>";
        let out = enhance_list_markers(html, &config);
        assert!(
            out.contains("</span>content"),
            "expected no literal space between marker span and content, got: {out}"
        );
    }

    #[test]
    fn renders_badge_with_inline_elements() {
        let config = MarkersConfig::default();
        let html = "<ul>\n<li><span class=\"tm-list-marker\"><em>Important</em></span> content</li>\n<li><span class=\"tm-list-marker\"><a class=\"tm-url\" href=\"https://example.com\">Wiki</a></span> docs</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(out.contains(
            "<li class=\"tm-list-item-with-badge\"><span class=\"tm-list-marker tm-list-marker-badge\"><em>Important</em></span>content</li>"
        ));
        assert!(out.contains(
            "<li class=\"tm-list-item-with-badge\"><span class=\"tm-list-marker tm-list-marker-badge\"><a class=\"tm-url\" href=\"https://example.com\">Wiki</a></span>docs</li>"
        ));
    }

    #[test]
    fn enabled_renders_badge_with_inline_elements() {
        let config = MarkersConfig {
            enable: true,
            strikethrough: true,
            custom: HashMap::new(),
        };
        let html = "<ul>\n<li><span class=\"tm-list-marker\"><em>Important</em></span> content</li>\n<li><span class=\"tm-list-marker\"><a class=\"tm-url\" href=\"https://example.com\">Wiki</a></span> docs</li>\n</ul>";
        let out = enhance_list_markers(html, &config);

        assert!(out.contains(
            "<li class=\"tm-list-item-with-badge\"><span class=\"tm-list-marker tm-list-marker-badge\"><em>Important</em></span>content</li>"
        ));
        assert!(out.contains(
            "<li class=\"tm-list-item-with-badge\"><span class=\"tm-list-marker tm-list-marker-badge\"><a class=\"tm-url\" href=\"https://example.com\">Wiki</a></span>docs</li>"
        ));
    }
}
