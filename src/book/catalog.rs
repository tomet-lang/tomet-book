//! The note list a hand-written `*.index.tmt` document defines.
//!
//! The catalog page lists every document in whatever order the vault walk
//! produced, which is a filing order, not a reading order. An index document
//! lets the author state the reading order instead: what to open first, what
//! belongs under what, and what not to show at all.
//!
//! The file is an ordinary Tomet document -- a nested list of links -- so it
//! needs no syntax of its own. An entry may also be a `${filter(...)}` query
//! (tomet-core's `tomet-transform::index_query`, shared rather than
//! reimplemented here): it expands into the same `@link(ref:"...")` shape
//! before this module ever sees the tree, so [`collect_blocks`] does not
//! need to know a query was ever there.
//!
//! ```tomet
//! @kind(doc.index)
//!
//! - @link(ref:"getting-started")
//! - @link(ref:"concepts")
//!   - @link(ref:"concepts/tomet")
//!   - ${filter(contains(path, "concepts/"), by(path))}
//! ```

use std::path::Path;

use rayon::prelude::*;
use tomet_ast::{Block, Document, Paragraph};

use super::document::strip_doc_extension;
use super::loader::DocFileInfo;

/// The suffix that marks a document as an index rather than a page.
pub const INDEX_SUFFIX: &str = ".index.tmt";

/// The suffix that marks a document as workspace configuration.
pub const CONFIG_SUFFIX: &str = ".config.tmt";

/// True for the control documents that shape the book without being part of
/// it. They are read by name and must not also become pages.
pub fn is_control_document(rel_path: &str) -> bool {
    rel_path.ends_with(INDEX_SUFFIX) || rel_path.ends_with(CONFIG_SUFFIX)
}

/// True for an index document that governs the whole book: one that sits at
/// the vault root. Index files deeper in the vault are reserved for ordering
/// their own folder, which nothing reads yet.
pub fn is_root_index(rel_path: &str) -> bool {
    rel_path.ends_with(INDEX_SUFFIX) && !rel_path.contains('/')
}

/// One line of a hand-written index: the page it points at, and whatever the
/// author nested under it.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexEntry {
    pub slug: String,
    pub children: Vec<IndexEntry>,
}

/// What reading one index document yielded.
#[derive(Debug, Default)]
pub struct IndexOutline {
    pub entries: Vec<IndexEntry>,
    /// Targets that resolved to nothing, kept so the build can name them.
    pub unresolved: Vec<String>,
    /// True when the document did not declare `@kind(doc.index)`.
    pub kind_missing: bool,
    /// A `${filter(...)}` that could not be answered (an unknown field, a
    /// bad `by(...)` direction, ...), kept so the build can name it the
    /// same way it names an unresolved link.
    pub query_errors: Vec<String>,
}

/// Read one index document into the order it describes.
///
/// `from_path` is the index document's own vault-relative path, which is what
/// makes a relative `ref:` resolve against the folder the file sits in.
/// `rows` is the vault-wide metadata table a `${filter(...)}` in this
/// document answers from -- see [`index_rows`]; an index with no query in
/// it costs nothing beyond a walk of its own blocks either way.
pub fn parse_index_document(
    source: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    rows: &[tomet_transform::IndexRow],
) -> IndexOutline {
    let Ok(mut doc) = tomet_parser::parse_document(source) else {
        return IndexOutline::default();
    };

    let kind_missing = tomet_semantics::document_kind(&doc).as_deref() != Some("doc.index");

    let mut outline = IndexOutline {
        kind_missing,
        ..Default::default()
    };

    // Expanded *before* `collect_blocks` walks the tree, so a query-
    // generated entry is, by the time collection sees it, indistinguishable
    // from one typed by hand -- `collect_blocks` needs no query-awareness
    // of its own.
    if let Err(e) = tomet_transform::expand_index_queries(&mut doc, rows) {
        outline.query_errors.push(e.to_string());
    }

    outline.entries = collect_blocks(&doc.blocks, from_path, vault_index, &mut outline.unresolved);
    outline
}

/// Turn already parsed documents into the rows a `${filter(...)}` query reads against.
///
/// Reuses ASTs already parsed in the build pipeline, avoiding redundant disk reads and parsing.
pub fn index_rows<'a>(
    docs: impl IntoParallelIterator<Item = (&'a Document, &'a DocFileInfo)>,
    src_dir: &Path,
) -> Vec<tomet_transform::IndexRow> {
    docs.into_par_iter()
        .map(|(doc, file)| make_index_row(doc, &file.abs_path, &file.rel_path, src_dir))
        .collect()
}

/// Build an [`IndexRow`] from an AST and document metadata.
pub fn make_index_row(
    doc: &Document,
    abs_path: &Path,
    rel_path: &str,
    src_dir: &Path,
) -> tomet_transform::IndexRow {
    let fields = tomet_indexer::document_fields(doc, abs_path, src_dir);
    let path = match fields.get("path") {
        Some(tomet_ast::Value::String(p)) => p.clone(),
        _ => rel_path.to_string(),
    };
    tomet_transform::IndexRow { path, fields }
}

/// Walk the lists in a run of blocks, in source order.
fn collect_blocks(
    blocks: &[Block],
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
    unresolved: &mut Vec<String>,
) -> Vec<IndexEntry> {
    let mut entries = Vec::new();

    for block in blocks {
        let Block::Element(el) = block else { continue };
        if !matches!(
            tomet_semantics::classify_std_lenient(el),
            tomet_semantics::ElementKind::UnorderedList | tomet_semantics::ElementKind::OrderedList
        ) {
            continue;
        }

        let Some(value) = &el.value else { continue };
        for item in value.as_children() {
            // A list item's own link lives in its content; the sub-list it
            // opens lives in its children. Reading them separately is what
            // keeps a parent from claiming its first child's target.
            let target = item
                .content
                .as_ref()
                .and_then(|inlines| first_page_target(inlines, item.span));

            let children = item
                .children
                .as_deref()
                .map(|blocks| collect_blocks(blocks, from_path, vault_index, unresolved))
                .unwrap_or_default();

            let Some(target) = target else {
                // A bullet with no link is a label, not an entry. Its
                // children still belong to the list.
                entries.extend(children);
                continue;
            };

            match resolve_page_slug(&target, from_path, vault_index) {
                Some(slug) => entries.push(IndexEntry { slug, children }),
                None => {
                    unresolved.push(target);
                    entries.extend(children);
                }
            }
        }
    }

    entries
}

/// The first link in a list item that could point at a page.
///
/// Wrapping the item's inlines in a throwaway document is what lets this
/// reuse `collect_links` rather than re-deriving which elements and which
/// schemes count as a link.
fn first_page_target(inlines: &[tomet_ast::Inline], span: tomet_ast::Span) -> Option<String> {
    let probe = Document::new(
        vec![Block::Paragraph(Paragraph::new(inlines.to_vec(), span))],
        span,
    );

    tomet_links::collect_links(&probe)
        .into_iter()
        .find(|link| {
            matches!(
                link.kind,
                tomet_links::LinkKind::Ref
                    | tomet_links::LinkKind::Tm
                    | tomet_links::LinkKind::File
            )
        })
        .map(|link| link.target)
}

/// Resolve a link target to the slug of a page, or `None` when it points at
/// nothing, or at something that is not a document.
fn resolve_page_slug(
    target: &str,
    from_path: &Path,
    vault_index: &tomet_links::VaultLinkIndex,
) -> Option<String> {
    let resolved = vault_index.resolve_ref(target, Some(from_path))?;
    let path = resolved.to_string_lossy().replace('\\', "/");
    let slug = strip_doc_extension(&path);
    // strip_doc_extension leaves non-documents untouched, which is how a link
    // to an image is told apart from a link to a page.
    (slug != path).then(|| slug.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_of(paths: &[&str]) -> tomet_links::VaultLinkIndex {
        let owned: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
        tomet_links::VaultLinkIndex::from_paths(&owned)
    }

    fn parse(source: &str, paths: &[&str]) -> IndexOutline {
        parse_index_document(source, Path::new("book.index.tmt"), &index_of(paths), &[])
    }

    fn row(path: &str, tags: &[&str]) -> tomet_transform::IndexRow {
        tomet_transform::IndexRow {
            path: path.to_string(),
            fields: std::collections::BTreeMap::from([(
                "meta.tags".to_string(),
                tomet_ast::Value::Seq(
                    tags.iter()
                        .map(|t| tomet_ast::Value::String(t.to_string()))
                        .collect(),
                ),
            )]),
        }
    }

    #[test]
    fn the_written_order_is_the_order() {
        let out = parse(
            "@kind(doc.index)\n\n- @link(ref:\"zebra\")\n- @link(ref:\"apple\")\n- @link(ref:\"mango\")\n",
            &["apple.tmt", "mango.tmt", "zebra.tmt"],
        );

        let slugs: Vec<&str> = out.entries.iter().map(|e| e.slug.as_str()).collect();
        assert_eq!(slugs, ["zebra", "apple", "mango"]);
        assert!(out.unresolved.is_empty());
        assert!(!out.kind_missing);
    }

    #[test]
    fn nesting_is_kept() {
        let out = parse(
            "@kind(doc.index)\n\n- @link(ref:\"guide\")\n  - @link(ref:\"install\")\n  - @link(ref:\"usage\")\n- @link(ref:\"reference\")\n",
            &["guide.tmt", "install.tmt", "usage.tmt", "reference.tmt"],
        );

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0].slug, "guide");
        let kids: Vec<&str> = out.entries[0]
            .children
            .iter()
            .map(|e| e.slug.as_str())
            .collect();
        assert_eq!(kids, ["install", "usage"]);
        assert_eq!(out.entries[1].slug, "reference");
        assert!(out.entries[1].children.is_empty());
    }

    #[test]
    fn a_target_that_resolves_to_nothing_is_reported() {
        let out = parse(
            "@kind(doc.index)\n\n- @link(ref:\"ghost\")\n- @link(ref:\"real\")\n",
            &["real.tmt"],
        );

        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0].slug, "real");
        assert_eq!(out.unresolved, ["ghost"]);
    }

    #[test]
    fn a_bullet_without_a_link_hands_its_children_up() {
        let out = parse(
            "@kind(doc.index)\n\nはじめに\n\n- 読み物\n  - @link(ref:\"essay\")\n- @link(ref:\"notes\")\n",
            &["essay.tmt", "notes.tmt"],
        );

        let slugs: Vec<&str> = out.entries.iter().map(|e| e.slug.as_str()).collect();
        assert_eq!(slugs, ["essay", "notes"]);
    }

    #[test]
    fn a_link_to_an_image_is_not_a_page() {
        let out = parse(
            "@kind(doc.index)\n\n- @link(ref:\"cover.png\")\n- @link(ref:\"page\")\n",
            &["cover.png", "page.tmt"],
        );

        let slugs: Vec<&str> = out.entries.iter().map(|e| e.slug.as_str()).collect();
        assert_eq!(slugs, ["page"]);
    }

    #[test]
    fn a_missing_kind_is_noticed() {
        let out = parse("- @link(ref:\"page\")\n", &["page.tmt"]);
        assert!(out.kind_missing);
        assert_eq!(out.entries.len(), 1);
    }

    #[test]
    fn control_documents_are_named_by_their_suffix() {
        assert!(is_control_document("book.index.tmt"));
        assert!(is_control_document("default.config.tmt"));
        assert!(!is_control_document("index.tmt"));
        assert!(!is_control_document("notes/index.tm"));

        assert!(is_root_index("book.index.tmt"));
        assert!(is_root_index("10-guides.index.tmt"));
        assert!(!is_root_index("creation/book.index.tmt"));
    }

    /// The integration claim this module's own doc comment makes: a query
    /// expands into the same `@link(ref:"...")` shape a hand-written entry
    /// is, so `collect_blocks` needs no special-casing for it -- proven
    /// here rather than assumed, mixed with a hand-written sibling and a
    /// hand-written parent.
    #[test]
    fn a_query_produces_the_same_outline_as_writing_it_by_hand() {
        let rows = [row("go.tmt", &["go"]), row("rust.tmt", &["rust"])];
        let paths = ["guide.tmt", "go.tmt", "rust.tmt"];

        let generated = parse_index_document(
            "@kind(doc.index)\n\n- @link(ref:\"guide\")\n  - ${filter(contains(meta.tags, \"go\"))}\n- ${filter(contains(meta.tags, \"rust\"))}\n",
            Path::new("book.index.tmt"),
            &index_of(&paths),
            &rows,
        );
        let by_hand = parse_index_document(
            "@kind(doc.index)\n\n- @link(ref:\"guide\")\n  - @link(ref:\"go\")\n- @link(ref:\"rust\")\n",
            Path::new("book.index.tmt"),
            &index_of(&paths),
            &[],
        );

        assert!(
            generated.query_errors.is_empty(),
            "{:?}",
            generated.query_errors
        );
        assert_eq!(generated.entries, by_hand.entries);
    }

    #[test]
    fn a_query_that_cannot_be_answered_is_reported_not_dropped() {
        let out = parse_index_document(
            "@kind(doc.index)\n\n- ${filter(contains(meta.typo, \"go\"))}\n",
            Path::new("book.index.tmt"),
            &index_of(&["go.tmt"]),
            &[row("go.tmt", &["go"])],
        );
        assert!(!out.query_errors.is_empty());
    }
}
