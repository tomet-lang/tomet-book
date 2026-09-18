//! Building blocks for the HTML this crate assembles by hand.
//!
//! Infobox values reach the template through `| safe`, so anything produced
//! here has to be safe to drop straight into the page.

/// Escape text for interpolation into HTML text nodes and double-quoted attributes.
pub fn escape_html(raw: &str) -> String {
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
pub fn sanitize_color_hex(raw: &str) -> Option<String> {
    let hex = raw.trim().trim_start_matches('#');
    let is_hex = matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    is_hex.then(|| hex.to_ascii_lowercase())
}

/// A colour swatch, or plain escaped text when the value is not a hex colour.
pub fn color_chip_html(raw: &str) -> String {
    match sanitize_color_hex(raw) {
        Some(hex) => format!(
            r##"<span class="color-chip-wrapper" title="#{hex}"><span class="color-chip" style="background-color: #{hex};"></span><span class="color-hex">#{hex}</span></span>"##
        ),
        None => format!(r#"<span class="color-hex">{}</span>"#, escape_html(raw)),
    }
}

pub fn external_link_html(url: &str) -> String {
    let safe = escape_html(url);
    format!(r#"<a href="{safe}" target="_blank" rel="noopener noreferrer">{safe}</a>"#)
}

pub fn is_external_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_covers_the_markup_and_quote_characters() {
        assert_eq!(
            escape_html(r#"<b> & "x" 'y'"#),
            "&lt;b&gt; &amp; &quot;x&quot; &#39;y&#39;"
        );
        assert_eq!(escape_html("日本語 plain"), "日本語 plain");
    }

    #[test]
    fn external_links_escape_the_url_in_both_slots() {
        let html = external_link_html(r#"https://example.com/?a=1&b="x""#);
        assert!(html.contains(r#"href="https://example.com/?a=1&amp;b=&quot;x&quot;""#));
        assert!(!html.contains(r#"b=""#));
    }

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
}
