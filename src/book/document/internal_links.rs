//! Internal link enhancement: adds note icon prefixes (from note's @meta.icon)
//! to internal links matching published documents in the vault.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static INTERNAL_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Matches `<a ... href="..." ...>...</a>`
    Regex::new(r#"(?s)<a\b([^>]*\bhref="([^"]+)"[^>]*)>(.*?)</a>"#)
        .expect("valid regex for internal link")
});

/// Basic percent-decoding for URLs (e.g. `%20` -> ` `, Japanese UTF-8 bytes).
fn percent_decode(s: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next()?;
            let h2 = chars.next()?;
            let buf = [h1, h2];
            let hex_str = std::str::from_utf8(&buf).ok()?;
            let val = u8::from_str_radix(hex_str, 16).ok()?;
            bytes.push(val);
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
}

/// Normalizes and extracts the document slug from an internal href.
///
/// Strips query params, `#fragment`, clean_url_prefix, and leading/trailing slashes.
fn extract_slug_from_href<'a>(href: &'a str, clean_url_prefix: &str) -> &'a str {
    let path_part = href.split(['#', '?']).next().unwrap_or(href);
    let path_no_leading = path_part.trim_start_matches('/');
    let clean_prefix = clean_url_prefix.trim_matches('/');

    let without_prefix = if !clean_prefix.is_empty() {
        if let Some(rest) = path_no_leading.strip_prefix(clean_prefix) {
            rest.trim_start_matches('/')
        } else {
            path_no_leading
        }
    } else {
        path_no_leading
    };

    without_prefix
        .strip_suffix(".html")
        .unwrap_or(without_prefix)
        .trim_end_matches('/')
}

/// Enhances internal links in rendered HTML by inserting the target note's icon prefix.
pub fn enhance_internal_links(
    body_html: &str,
    slug_to_icon: &HashMap<String, String>,
    clean_url_prefix: &str,
) -> String {
    if slug_to_icon.is_empty() {
        return body_html.to_string();
    }

    INTERNAL_LINK_RE
        .replace_all(body_html, |caps: &regex::Captures| {
            let attrs = &caps[1];
            let href = &caps[2];
            let inner_content = &caps[3];

            // 1. Skip external links, anchor-only links, mailto, etc.
            if href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with("//")
                || href.starts_with('#')
                || href.contains(':')
            {
                return caps[0].to_string();
            }

            // 2. Skip unresolved links or links that already contain an icon or image element.
            if attrs.contains("tm-ref-unresolved")
                || inner_content.trim().is_empty()
                || inner_content.contains("<img")
                || inner_content.contains("<svg")
                || inner_content.contains("<tmt-icon")
                || inner_content.contains("tm-link-icon")
            {
                return caps[0].to_string();
            }

            // 3. Extract target slug
            let slug = extract_slug_from_href(href, clean_url_prefix);
            if slug.is_empty() {
                return caps[0].to_string();
            }

            // 4. Look up note icon
            if let Some(icon_html) = slug_to_icon.get(slug) {
                format!("<a{attrs}>{icon_html}{inner_content}</a>")
            } else if slug.contains('%')
                && let Some(decoded) = percent_decode(slug)
                && let Some(icon_html) = slug_to_icon.get(&decoded)
            {
                format!("<a{attrs}>{icon_html}{inner_content}</a>")
            } else {
                caps[0].to_string()
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhances_internal_link_with_note_icon() {
        let mut icons = HashMap::new();
        icons.insert(
            "Shopify".to_string(),
            r#"<span class="tm-link-icon tm-link-icon-emoji" aria-hidden="true">🛍️</span>"#
                .to_string(),
        );

        let html = r#"<p><a class="tm-file" href="/wiki/Shopify">Shopify</a></p>"#;
        let out = enhance_internal_links(html, &icons, "/wiki");
        assert_eq!(
            out,
            r#"<p><a class="tm-file" href="/wiki/Shopify"><span class="tm-link-icon tm-link-icon-emoji" aria-hidden="true">🛍️</span>Shopify</a></p>"#
        );
    }

    #[test]
    fn enhances_internal_link_with_anchor_fragment() {
        let mut icons = HashMap::new();
        icons.insert(
            "30-39 Knowledge/Shopify".to_string(),
            r#"<svg class="tm-link-icon tm-link-icon-lucide" aria-hidden="true"><use href="/icons/lucide.svg#store"></use></svg>"#.to_string(),
        );

        let html = r#"<p><a class="tm-file" href="/wiki/30-39 Knowledge/Shopify#pricing">Shopify Pricing</a></p>"#;
        let out = enhance_internal_links(html, &icons, "/wiki");
        assert_eq!(
            out,
            r#"<p><a class="tm-file" href="/wiki/30-39 Knowledge/Shopify#pricing"><svg class="tm-link-icon tm-link-icon-lucide" aria-hidden="true"><use href="/icons/lucide.svg#store"></use></svg>Shopify Pricing</a></p>"#
        );
    }

    #[test]
    fn enhances_internal_link_with_url_decoding() {
        let mut icons = HashMap::new();
        icons.insert(
            "My Notes".to_string(),
            r#"<svg class="tm-link-icon tm-link-icon-lucide" aria-hidden="true"><use href="/icons/lucide.svg#file-text"></use></svg>"#.to_string(),
        );

        let html = r#"<p><a class="tm-file" href="/wiki/My%20Notes">My Notes</a></p>"#;
        let out = enhance_internal_links(html, &icons, "/wiki");
        assert!(out.contains("file-text"));
    }

    #[test]
    fn skips_unresolved_links() {
        let mut icons = HashMap::new();
        icons.insert(
            "Secret".to_string(),
            r#"<span class="tm-link-icon tm-link-icon-emoji">🔒</span>"#.to_string(),
        );

        let html = r#"<p><a class="tm-ref tm-ref-unresolved" aria-disabled="true" data-ref="Secret">Secret</a></p>"#;
        let out = enhance_internal_links(html, &icons, "/wiki");
        assert_eq!(out, html);
    }

    #[test]
    fn skips_links_already_containing_images_or_icons() {
        let mut icons = HashMap::new();
        icons.insert(
            "Shopify".to_string(),
            r#"<span class="tm-link-icon">🛍️</span>"#.to_string(),
        );

        let html =
            r#"<p><a class="tm-file" href="/wiki/Shopify"><img src="badge.png" alt="" /></a></p>"#;
        let out = enhance_internal_links(html, &icons, "/wiki");
        assert_eq!(out, html);
    }

    #[test]
    fn skips_external_links() {
        let mut icons = HashMap::new();
        icons.insert(
            "example".to_string(),
            r#"<span class="tm-link-icon">🌐</span>"#.to_string(),
        );

        let html = r#"<p><a class="tm-url" href="https://example.com">Example</a></p>"#;
        let out = enhance_internal_links(html, &icons, "/wiki");
        assert_eq!(out, html);
    }

    #[test]
    fn works_with_empty_url_prefix() {
        let mut icons = HashMap::new();
        icons.insert(
            "docs/guide".to_string(),
            r#"<span class="tm-link-icon tm-link-icon-emoji" aria-hidden="true">📖</span>"#
                .to_string(),
        );

        let html = r#"<p><a class="tm-file" href="/docs/guide">Guide</a></p>"#;
        let out = enhance_internal_links(html, &icons, "");
        assert_eq!(
            out,
            r#"<p><a class="tm-file" href="/docs/guide"><span class="tm-link-icon tm-link-icon-emoji" aria-hidden="true">📖</span>Guide</a></p>"#
        );
    }
}
