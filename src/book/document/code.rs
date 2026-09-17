//! Code block enhancement: wraps `<pre><code>` blocks with a header bar containing
//! an optional language label (top-left) and a clipboard copy button (top-right).

use std::sync::LazyLock;
use regex::Regex;

use super::html::escape_html;

static PRE_CODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<pre([^>]*)>(\s*)<code([^>]*)>(.*?)</code>(\s*)</pre>"#)
        .expect("valid regex for pre code blocks")
});

static LANG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"class="[^"]*language-([a-zA-Z0-9_.-]+)[^"]*""#)
        .expect("valid regex for code language class")
});

fn extract_language(code_attrs: &str) -> Option<&str> {
    LANG_RE
        .captures(code_attrs)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
}

/// Enhances code blocks in the rendered HTML by wrapping them in a container
/// with a language label and a copy button.
pub fn enhance_code_blocks(body_html: &str, lang: Option<&str>) -> String {
    let copy_title = if lang.map(crate::i18n::is_english).unwrap_or(false) {
        "Copy code"
    } else {
        "コードをコピー"
    };

    PRE_CODE_RE
        .replace_all(body_html, |caps: &regex::Captures| {
            let pre_attrs = &caps[1];
            let pre_ws = &caps[2];
            let code_attrs = &caps[3];
            let code_content = &caps[4];
            let post_ws = &caps[5];

            let lang_html = match extract_language(code_attrs) {
                Some(code_lang) if !code_lang.is_empty() => {
                    format!("<span class=\"code-lang\">{}</span>", escape_html(code_lang))
                }
                _ => String::new(),
            };

            let copy_btn = format!(
                r#"<button type="button" class="code-copy-btn" title="{copy_title}" aria-label="{copy_title}"><svg class="tm-doc-icon" aria-hidden="true"><use href="/icons/lucide.svg#copy"></use></svg></button>"#
            );

            format!(
                r#"<div class="code-block-wrapper"><div class="code-block-header">{lang_html}{copy_btn}</div><pre{pre_attrs}>{pre_ws}<code{code_attrs}>{code_content}</code>{post_ws}</pre></div>"#
            )
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_code_block_with_language_label_and_copy_button() {
        let html = r#"<pre><code class="language-toml">[ui.markers]
enable = true</code></pre>"#;
        let out = enhance_code_blocks(html, None);

        assert!(out.contains("<div class=\"code-block-wrapper\">"));
        assert!(out.contains("<span class=\"code-lang\">toml</span>"));
        assert!(out.contains("class=\"code-copy-btn\""));
        assert!(out.contains("title=\"コードをコピー\""));
        assert!(out.contains("<use href=\"/icons/lucide.svg#copy\"></use>"));
        assert!(out.contains(r#"<pre><code class="language-toml">[ui.markers]
enable = true</code></pre>"#));
    }

    #[test]
    fn wraps_code_block_without_language_label() {
        let html = "<pre><code>plain code</code></pre>";
        let out = enhance_code_blocks(html, None);

        assert!(out.contains("<div class=\"code-block-wrapper\">"));
        assert!(!out.contains("code-lang"));
        assert!(out.contains("class=\"code-copy-btn\""));
        assert!(out.contains("<pre><code>plain code</code></pre>"));
    }

    #[test]
    fn preserves_pre_and_code_attributes() {
        let html = r#"<pre id="snippet1" class="custom-pre"><code class="language-rust custom-code">fn main() {}</code></pre>"#;
        let out = enhance_code_blocks(html, None);

        assert!(out.contains("<span class=\"code-lang\">rust</span>"));
        assert!(out.contains(r#"<pre id="snippet1" class="custom-pre"><code class="language-rust custom-code">fn main() {}</code></pre>"#));
    }

    #[test]
    fn supports_english_copy_title() {
        let html = r#"<pre><code class="language-rust">fn main() {}</code></pre>"#;
        let out = enhance_code_blocks(html, Some("en"));

        assert!(out.contains("title=\"Copy code\""));
        assert!(out.contains("aria-label=\"Copy code\""));
    }
}
