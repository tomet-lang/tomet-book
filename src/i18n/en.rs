use crate::config::HeroChipConfig;
use std::collections::HashMap;

/// The interface's text in English.
pub fn ui_strings() -> HashMap<String, String> {
    [
        // Panes and navigation
        ("pane.toc", "Table of Contents"),
        ("pane.toc_short", "TOC"),
        ("pane.links", "Links"),
        ("pane.links_short", "LINKS"),
        ("pane.graph", "Graph"),
        ("pane.graph_short", "GRAPH"),
        ("pane.index", "Index"),
        ("pane.index_short", "INDEX"),
        ("index.empty", "No index entries"),
        ("pane.lookup", "Search"),
        ("pane.lookup_short", "SEARCH"),
        ("lookup.placeholder", "Filter by title..."),
        ("lookup.tags_all", "All"),
        ("pane.data", "Data & Profile"),
        ("pane.data_short", "Data"),
        ("pane.collapse", "Click to collapse panel"),
        ("pane.data_expand", "Click to expand data"),
        ("pane.data_collapse", "Click to collapse data"),
        ("nav.home", "← Home"),
        ("nav.home_title", "Go to home"),
        ("nav.section", "Section:"),
        ("nav.bar", "Navigation bar"),
        // Content chrome
        ("toc.heading", "Table of Contents"),
        ("recent.heading", "Recent Notes"),
        ("backlinks.heading", "Backlinks"),
        ("backlinks.empty", "No backlinks to this note"),
        ("graph.heading", "Local Graph"),
        ("graph.placeholder", "Graph loading..."),
        ("sticky.label", "Sticky Headings"),
        ("sticky.to_top", "Scroll to top"),
        ("sticky.to_bottom", "Scroll to bottom"),
        ("tabs.side", "Switch tab position"),
        ("tabs.top", "⬒ Top"),
        ("tabs.top_title", "Place tabs at top horizontally"),
        ("tabs.right", "◨ Right"),
        ("tabs.right_title", "Place tabs at right vertically"),
        ("costume.label", "Costume:"),
        ("costume.item", "Costume"),
        ("image.zoom", "Zoom"),
        ("image.zoom_banner", "View full banner"),
        ("image.banner_alt", "Banner"),
        ("lightbox.close", "Close"),
        ("peek.fullscreen", "Open fullscreen"),
        ("peek.close", "Close"),
        ("preview.open_peek", "Click to open"),
        ("peek.error", "Failed to load page"),
        ("peek.not_found", "Content not found"),
        // Catalog page
        ("index.sections", "Sections"),
        ("index.entries", "All Notes"),
        ("index.note_unit", "notes"),
        // Controls
        ("view.switch", "Switch view mode"),
        ("view.book", "📖 Book"),
        ("view.book_title", "Book mode (multi-pane)"),
        ("view.classic", "📄 Scroll"),
        ("view.classic_title", "Vertical scroll mode"),
        ("drag.handle", "Drag to move"),
        ("theme.toggle", "Toggle theme (Light / Dark)"),
        ("theme.toggle_label", "Toggle theme"),
        ("theme.to_light", "Switch to light theme"),
        ("theme.to_dark", "Switch to dark theme"),
        ("theme.shell_toggle", "Toggle shell theme"),
        ("theme.shell_label", "Shell Theme"),
        ("theme.shell.color_label", "Color"),
        ("theme.shell.pattern_label", "Pattern"),
        ("theme.shell.contrast_label", "Contrast"),
        ("theme.shell.contrast_high", "High Contrast"),
        ("theme.shell.slate", "Slate"),
        ("theme.shell.violet", "Violet"),
        ("theme.shell.indigo", "Indigo"),
        ("theme.shell.olive", "Olive"),
        ("theme.shell.contrast", "Contrast"),
        ("theme.shell.color_slate", "Slate"),
        ("theme.shell.color_violet", "Violet"),
        ("theme.shell.color_indigo", "Indigo"),
        ("theme.shell.color_olive", "Olive"),
        ("theme.shell.pattern_marble", "Marble"),
        ("theme.shell.pattern_cloud", "Cloud"),
        ("theme.shell.pattern_seigaiha", "Seigaiha"),
        ("theme.shell.pattern_flourish", "Flourish"),
        ("theme.shell.pattern_mesh", "Mesh"),
        ("theme.shell.pattern_none", "None"),
        // Search
        ("search.placeholder", "Search... (Ctrl+K)"),
        ("search.loading", "Preparing search index..."),
        ("search.empty", "No results found"),
        ("search.aliases", "Aliases:"),
        ("search.prev", "◀ Prev"),
        ("search.next", "Next ▶"),
        // Editing
        ("edit.open", "Edit"),
        ("edit.editor", "Open in Editor"),
        ("edit.vscode", "Open in VSCode"),
        ("edit.cursor", "Open in Cursor"),
        ("edit.copy_path", "Copy Path"),
        ("edit.copy_path_title", "Copy file path"),
        ("toast.copied", "📋 Copied path to clipboard"),
        ("toast.copy_failed", "❌ Failed to copy path"),
        // Links that point at a page nobody has written yet
        ("link.unresolved", "Page not yet created"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// Default hero chips in English.
pub fn hero_chips() -> Vec<HeroChipConfig> {
    vec![
        HeroChipConfig {
            key: "parent".to_string(),
            label: Some("Affiliation".to_string()),
            format: None,
        },
        HeroChipConfig {
            key: "type.list-a".to_string(),
            label: Some("Fans".to_string()),
            format: None,
        },
        HeroChipConfig {
            key: "type.element-a".to_string(),
            label: None,
            format: None,
        },
        HeroChipConfig {
            key: "birth".to_string(),
            label: Some("🎂".to_string()),
            format: Some("birthday".to_string()),
        },
    ]
}

/// Default infobox key labels in English.
pub fn key_labels() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("kind".to_string(), "Kind".to_string());
    m.insert("type".to_string(), "Type".to_string());
    m.insert("birth".to_string(), "Birthday".to_string());
    m.insert("height".to_string(), "Height".to_string());
    m.insert("creator".to_string(), "Creator".to_string());
    m.insert("type.list-a".to_string(), "Fan Name".to_string());
    m.insert("type.element-a".to_string(), "Element / MBTI".to_string());
    m.insert("colors".to_string(), "Colors".to_string());
    m.insert("parent".to_string(), "Affiliation".to_string());
    m.insert("group".to_string(), "Organization".to_string());
    m.insert("place".to_string(), "Origin".to_string());
    m.insert("created".to_string(), "Created".to_string());
    m.insert("modified".to_string(), "Modified".to_string());
    m.insert("reviewed".to_string(), "Reviewed".to_string());
    m.insert("rating".to_string(), "Rating".to_string());
    m.insert("aliases".to_string(), "Aliases / Nicknames".to_string());
    m.insert("topics".to_string(), "Topics".to_string());
    m.insert("flags".to_string(), "Flags".to_string());
    m.insert("published".to_string(), "Published".to_string());
    m.insert("url.wiki".to_string(), "Wikipedia".to_string());
    m.insert("url.else".to_string(), "Related Links".to_string());
    m
}
