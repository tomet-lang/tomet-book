//! Resolving the link references that appear inside `@meta` values.
//!
//! The body's links are resolved upstream by `tomet-transform`; these are the
//! ones that only exist as metadata strings -- `parent: "[[unit]]"` and friends
//! -- which never pass through the renderer.

use std::path::Path;

use super::html::escape_html;

/// Suffixes that mark a file as a Tomet document.
pub const DOC_EXTENSIONS: &[&str] = &[".tmt", ".tm"];

/// Strip a single Tomet document extension: `foo/bar.tmt` -> `foo/bar`.
///
/// Uses `strip_suffix` rather than `trim_end_matches`, which would chew through
/// repeated suffixes and turn `notes.tmt.tmt` into `notes`.
pub fn strip_doc_extension(path: &str) -> &str {
    for ext in DOC_EXTENSIONS {
        if let Some(stem) = path.strip_suffix(ext) {
            return stem;
        }
    }
    path
}

pub fn is_link_ref(raw: &str) -> bool {
    let s = raw.trim();
    (s.starts_with("@link(") && s.ends_with(')'))
        || (s.starts_with("link(") && s.ends_with(')'))
        || (s.starts_with("[[") && s.ends_with("]]"))
        || s.starts_with("ref:")
        || (s.starts_with('@') && s.len() > 1 && !s.contains(' ') && !s[1..].contains('@'))
}

/// Clean link references like `@link(ref:"@foo")` or `ref:bar` into plain labels.
pub fn clean_ref_target(raw: &str) -> String {
    let mut s = raw.trim();
    if s.starts_with("@link(") && s.ends_with(')') {
        s = s[6..s.len() - 1].trim();
    } else if s.starts_with("link(") && s.ends_with(')') {
        s = s[5..s.len() - 1].trim();
    } else if s.starts_with("[[") && s.ends_with("]]") {
        let inner = s[2..s.len() - 2].trim();
        if let Some((_, a)) = inner.split_once('|') {
            return a.trim().to_string();
        } else {
            s = inner;
        }
    }
    if s.starts_with("ref:") {
        s = s["ref:".len()..].trim();
    }
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s = s[1..s.len() - 1].trim();
    }
    s.trim_start_matches('@').trim().to_string()
}

/// Resolve a `@meta` value that looks like a link into `(href, label)`.
///
/// Returns `None` when the value is not a link reference at all. An empty href
/// means the reference is a link to a page that does not exist yet.
pub fn resolve_meta_link(
    raw: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    url_prefix: &str,
) -> Option<(String, String)> {
    let s = raw.trim();
    if !is_link_ref(s) {
        return None;
    }

    let mut target = s;
    let mut alias = None;

    if target.starts_with("@link(") && target.ends_with(')') {
        target = target[6..target.len() - 1].trim();
    } else if target.starts_with("link(") && target.ends_with(')') {
        target = target[5..target.len() - 1].trim();
    } else if target.starts_with("[[") && target.ends_with("]]") {
        let inner = target[2..target.len() - 2].trim();
        if let Some((t, a)) = inner.split_once('|') {
            target = t.trim();
            alias = Some(a.trim().to_string());
        } else {
            target = inner;
        }
    }

    if target.starts_with("ref:") {
        target = target["ref:".len()..].trim();
    }
    if (target.starts_with('"') && target.ends_with('"'))
        || (target.starts_with('\'') && target.ends_with('\''))
    {
        target = target[1..target.len() - 1].trim();
    }

    let clean_target = target.trim();
    let display_label = alias.unwrap_or_else(|| clean_ref_target(s));
    let clean_url_prefix = url_prefix.trim_end_matches('/');

    let resolved = vault_index
        .resolve_ref(clean_target, Some(from_path))
        .or_else(|| {
            let stripped = clean_target.trim_start_matches('@');
            if stripped != clean_target {
                vault_index.resolve_ref(stripped, Some(from_path))
            } else {
                None
            }
        });

    if let Some(path) = resolved {
        let slug = strip_doc_extension(&path.to_string_lossy().replace('\\', "/")).to_string();
        Some((format!("{clean_url_prefix}/{slug}"), display_label))
    } else {
        Some((String::new(), display_label))
    }
}

/// Render a `@meta` value as infobox HTML: a link when it is one, escaped text otherwise.
pub fn format_meta_value_to_html(
    raw_str: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    url_prefix: &str,
) -> String {
    if let Some((href, label)) = resolve_meta_link(raw_str, from_path, vault_index, url_prefix) {
        let label = escape_html(&label);
        if href.is_empty() {
            format!(r#"<span class="tm-link tm-unresolved" title="未作成のページ">{label}</span>"#)
        } else {
            let href = escape_html(&href);
            format!(r#"<a href="{href}" class="tm-link tm-file">{label}</a>"#)
        }
    } else {
        escape_html(&clean_ref_target(raw_str))
    }
}

/// Resolve a media reference in `@meta` (e.g. `@link(ref:+hash.png)`) to a URL.
pub fn resolve_meta_media_path(
    raw: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    asset_prefix: &str,
) -> String {
    let s = raw.trim();
    if s.starts_with('/') || s.starts_with("http://") || s.starts_with("https://") {
        return s.to_string();
    }

    let target = clean_ref_target(s);
    let clean_prefix = asset_prefix.trim_end_matches('/');

    if let Some(resolved) = vault_index.resolve_ref(&target, Some(from_path)) {
        format!(
            "{clean_prefix}/{}",
            resolved.to_string_lossy().replace('\\', "/")
        )
    } else {
        format!("{clean_prefix}/{}", target.replace('\\', "/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(paths: &[&str]) -> tomet_links::VaultLinkIndex {
        tomet_links::VaultLinkIndex::from_paths(paths)
    }

    #[test]
    fn recognizes_link_reference_forms() {
        assert!(is_link_ref(r#"@link(ref:"@foo")"#));
        assert!(is_link_ref("link(ref:bar)"));
        assert!(is_link_ref("[[foo]]"));
        assert!(is_link_ref("[[foo|別名]]"));
        assert!(is_link_ref("ref:baz"));
        assert!(is_link_ref("@foo"));
    }

    #[test]
    fn plain_values_are_not_link_references() {
        assert!(!is_link_ref("ただのテキスト"));
        assert!(!is_link_ref("2024-01-01"));
        assert!(!is_link_ref("@foo @bar"));
        assert!(!is_link_ref("a@b"));
    }

    #[test]
    fn clean_ref_target_unwraps_every_form() {
        assert_eq!(clean_ref_target(r#"@link(ref:"@foo")"#), "foo");
        assert_eq!(clean_ref_target("link(ref:bar)"), "bar");
        assert_eq!(clean_ref_target("[[foo]]"), "foo");
        assert_eq!(clean_ref_target("ref:baz"), "baz");
        assert_eq!(clean_ref_target("@qux"), "qux");
        assert_eq!(clean_ref_target("  plain  "), "plain");
    }

    #[test]
    fn clean_ref_target_prefers_the_alias() {
        assert_eq!(clean_ref_target("[[foo|別名]]"), "別名");
    }

    #[test]
    fn resolve_meta_link_builds_a_url_under_the_prefix() {
        let idx = index(&["30-39 Knowledge/rust.tmt"]);
        let (href, label) =
            resolve_meta_link("[[rust]]", Path::new("notes/a.tmt"), &idx, "/wiki").unwrap();
        assert_eq!(href, "/wiki/30-39 Knowledge/rust");
        assert_eq!(label, "rust");
    }

    #[test]
    fn resolve_meta_link_keeps_the_alias_as_the_label() {
        let idx = index(&["30-39 Knowledge/rust.tmt"]);
        let (href, label) =
            resolve_meta_link("[[rust|ラスト]]", Path::new("notes/a.tmt"), &idx, "/wiki").unwrap();
        assert_eq!(href, "/wiki/30-39 Knowledge/rust");
        assert_eq!(label, "ラスト");
    }

    #[test]
    fn resolve_meta_link_reports_unresolved_targets_with_an_empty_href() {
        let idx = index(&["a.tmt"]);
        let (href, label) =
            resolve_meta_link("[[missing]]", Path::new("a.tmt"), &idx, "/wiki").unwrap();
        assert!(href.is_empty());
        assert_eq!(label, "missing");
    }

    #[test]
    fn resolve_meta_link_ignores_non_references() {
        let idx = index(&["a.tmt"]);
        assert!(resolve_meta_link("ただの値", Path::new("a.tmt"), &idx, "/wiki").is_none());
    }

    #[test]
    fn trailing_slash_on_the_url_prefix_is_not_doubled() {
        let idx = index(&["rust.tmt"]);
        let (href, _) = resolve_meta_link("[[rust]]", Path::new("a.tmt"), &idx, "/wiki/").unwrap();
        assert_eq!(href, "/wiki/rust");
    }

    #[test]
    fn meta_values_cannot_break_out_of_the_infobox_cell() {
        let idx = index(&["a.tmt"]);
        let html = format_meta_value_to_html(
            r#"<img src=x onerror=alert(1)>"#,
            Path::new("a.tmt"),
            &idx,
            "/wiki",
        );
        // The payload survives as inert text, but never as markup.
        assert_eq!(html, "&lt;img src=x onerror=alert(1)&gt;");
    }

    #[test]
    fn unresolved_link_labels_are_escaped() {
        let idx = index(&["a.tmt"]);
        let html = format_meta_value_to_html(
            r#"[[missing|"><script>x</script>]]"#,
            Path::new("a.tmt"),
            &idx,
            "/wiki",
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn strips_a_single_document_extension() {
        assert_eq!(strip_doc_extension("foo/bar.tmt"), "foo/bar");
        assert_eq!(strip_doc_extension("foo/bar.tm"), "foo/bar");
    }

    #[test]
    fn does_not_chew_through_repeated_suffixes() {
        assert_eq!(strip_doc_extension("notes.tmt.tmt"), "notes.tmt");
    }

    #[test]
    fn leaves_other_names_alone() {
        assert_eq!(strip_doc_extension("image.png"), "image.png");
        assert_eq!(strip_doc_extension("plain"), "plain");
    }
}
