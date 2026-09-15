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

/// A parsed link reference representation.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedLinkRef<'a> {
    /// The target name or path (without quotes, 'ref:', or outer markup).
    pub target: &'a str,
    /// The optional explicit alias (e.g. from `[[target|alias]]`).
    pub alias: Option<&'a str>,
}

/// Extract the core target and optional alias from link reference syntax.
pub fn parse_link_ref(raw: &str) -> ParsedLinkRef<'_> {
    let mut s = raw.trim();
    let mut alias = None;

    if s.starts_with("@link(") && s.ends_with(')') {
        s = s[6..s.len() - 1].trim();
    } else if s.starts_with("link(") && s.ends_with(')') {
        s = s[5..s.len() - 1].trim();
    } else if s.starts_with("[[") && s.ends_with("]]") {
        let inner = s[2..s.len() - 2].trim();
        if let Some((t, a)) = inner.split_once('|') {
            s = t.trim();
            alias = Some(a.trim());
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

    ParsedLinkRef {
        target: s.trim(),
        alias,
    }
}

/// Clean link references like `@link(ref:"@foo")` or `ref:bar` into plain labels.
pub fn clean_ref_target(raw: &str) -> String {
    let parsed = parse_link_ref(raw);
    if let Some(alias) = parsed.alias {
        return alias.to_string();
    }
    parsed.target.trim_start_matches('@').trim().to_string()
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

    let parsed = parse_link_ref(s);
    let clean_target = parsed.target;
    let display_label = parsed
        .alias
        .map(str::to_string)
        .unwrap_or_else(|| clean_target.trim_start_matches('@').trim().to_string());
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
        let slug = strip_doc_extension(&crate::book::normalize_path(path)).to_string();
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
        format!("{clean_prefix}/{}", crate::book::normalize_path(resolved))
    } else {
        format!(
            "{clean_prefix}/{}",
            crate::book::normalize_path_str(&target)
        )
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

/// The pages this document links to, as slugs, in the order they appear.
///
/// Collected from the parsed body before link resolution rewrites the targets,
/// then resolved the same way `@meta` references are. Only links that land on
/// another Tomet document count -- media and external URLs are not pages, and
/// a document linking to itself is not a backlink.
pub fn outgoing_page_slugs(
    doc: &tomet_ast::Document,
    from_path: &Path,
    own_slug: &str,
    vault_index: &tomet_links::VaultLinkIndex,
    unpublished: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut slugs: Vec<String> = Vec::new();

    for link in tomet_links::collect_links(doc) {
        // Dir and Embed point at folders and media, never at a page.
        if !matches!(
            link.kind,
            tomet_links::LinkKind::Ref | tomet_links::LinkKind::Tm | tomet_links::LinkKind::File
        ) {
            continue;
        }

        let Some(resolved) = vault_index.resolve_ref(&link.target, Some(from_path)) else {
            continue;
        };
        let path = crate::book::normalize_path(resolved);
        // A page the book leaves out has nothing to show a backlink on.
        if unpublished.contains(&path) {
            continue;
        }
        let slug = strip_doc_extension(&path);
        // strip_doc_extension leaves non-documents untouched, which is how a
        // link to an image is told apart from a link to a page.
        if slug == path || slug == own_slug || slugs.iter().any(|s| s == slug) {
            continue;
        }
        slugs.push(slug.to_string());
    }

    slugs
}

/// The slug a resolved `@meta` href points at, if it points at a page at all.
pub fn slug_from_href(href: &str, url_prefix: &str) -> Option<String> {
    let clean = url_prefix.trim_end_matches('/');
    let rest = href.strip_prefix(clean)?.trim_start_matches('/');
    (!rest.is_empty()).then(|| rest.to_string())
}

#[cfg(test)]
mod outgoing_tests {
    use super::*;

    #[test]
    fn a_href_under_the_prefix_yields_its_slug() {
        assert_eq!(
            slug_from_href("/wiki/30-39 Knowledge/rust", "/wiki").as_deref(),
            Some("30-39 Knowledge/rust")
        );
        assert_eq!(slug_from_href("/wiki/a", "/wiki/").as_deref(), Some("a"));
    }

    #[test]
    fn an_href_outside_the_prefix_is_not_a_page() {
        assert_eq!(slug_from_href("https://example.com", "/wiki"), None);
        assert_eq!(slug_from_href("/vault/img.png", "/wiki"), None);
        // The catalog itself is not a page anyone backlinks to.
        assert_eq!(slug_from_href("/wiki", "/wiki"), None);
    }

    fn slugs(source: &str, paths: &[&str], own: &str) -> Vec<String> {
        let doc = tomet_parser::parse_document(source).unwrap();
        let index = tomet_links::VaultLinkIndex::from_paths(paths);
        outgoing_page_slugs(
            &doc,
            Path::new(own),
            strip_doc_extension(own),
            &index,
            &std::collections::HashSet::new(),
        )
    }

    #[test]
    fn body_references_become_slugs() {
        // `ref:` targets are names, not paths, so a page in a folder is still
        // reached by its stem -- and the slug that comes back carries the folder.
        let out = slugs(
            "本文 @link(ref:\"other\") と @link(ref:\"page\") へ。\n",
            &["a.tmt", "other.tmt", "deep/page.tmt"],
            "a.tmt",
        );
        assert_eq!(out, vec!["other", "deep/page"]);
    }

    #[test]
    fn a_link_to_itself_is_not_an_outgoing_link() {
        let out = slugs("@link(ref:\"a\") を指す。\n", &["a.tmt"], "a.tmt");
        assert!(out.is_empty());
    }

    #[test]
    fn the_same_page_twice_is_listed_once() {
        let out = slugs(
            "@link(ref:\"other\") と @link(ref:\"other\") へ。\n",
            &["a.tmt", "other.tmt"],
            "a.tmt",
        );
        assert_eq!(out, vec!["other"]);
    }

    #[test]
    fn unresolved_references_are_left_out() {
        let out = slugs("@link(ref:\"missing\") へ。\n", &["a.tmt"], "a.tmt");
        assert!(out.is_empty());
    }

    #[test]
    fn media_is_not_a_page() {
        let out = slugs(
            "@link(ref:\"photo.png\") へ。\n",
            &["a.tmt", "photo.png"],
            "a.tmt",
        );
        assert!(out.is_empty(), "got {out:?}");
    }
}
