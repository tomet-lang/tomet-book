pub mod assets;
pub mod catalog;
pub mod document;
pub mod kinds;
pub mod loader;
pub mod pagefind;
pub mod renderer;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::config::BookConfig;
use assets::{copy_vault_media, write_static_assets};
use document::ProcessedDoc;
use loader::{ScannedVault, scan_vault};
use pagefind::run_pagefind;
use renderer::{Backlink, BookRenderer, EntrySummary, SectionSummary};

use rayon::prelude::*;

/// Name of the record a build leaves behind in the output directory.
const MANIFEST_FILE: &str = ".tmtbook-manifest.json";

/// A document that could not be turned into a page.
#[derive(Debug, Clone)]
pub struct DocFailure {
    pub rel_path: String,
    pub error: String,
}

/// What the previous build put in the output directory.
///
/// Lets the next build find what to delete by set difference, instead of
/// walking the whole site looking for orphans.
#[derive(Debug, Default, Serialize, Deserialize)]
struct BuildManifest {
    /// Page slugs, relative to the wiki directory.
    pages: Vec<String>,
    /// Media paths, relative to the asset directory.
    media: Vec<String>,
}

/// `lookup/manifest.json`: the section names Search's tag chips list.
#[derive(Debug, Serialize)]
struct LookupManifest {
    sections: Vec<SectionSummary>,
}

impl BuildManifest {
    fn read(out_dir: &Path) -> Option<Self> {
        let raw = fs::read_to_string(out_dir.join(MANIFEST_FILE)).ok()?;
        serde_json::from_str(&raw).ok()
    }

    fn write(&self, out_dir: &Path) -> Result<()> {
        let raw = serde_json::to_string(self)?;
        write_if_changed(&out_dir.join(MANIFEST_FILE), &raw)?;
        Ok(())
    }
}

/// What a build actually did, so callers can report on it or refuse to ship it.
#[derive(Debug)]
pub struct BuildReport {
    pub rendered: usize,
    pub written: usize,
    pub failures: Vec<DocFailure>,
    pub pruned_pages: usize,
    pub pruned_media: usize,
    /// The vault as this build saw it, so a caller that needs it again -- the
    /// dev server priming its incremental cache -- does not rescan.
    pub scanned: ScannedVault,
    /// Who points at each page, keyed by slug. Handed back so the dev server
    /// can re-render one document without reading the whole vault again.
    pub backlinks: HashMap<String, Vec<Backlink>>,
    /// The documents this build left out, by vault-relative path, so links to
    /// them keep resolving to nothing between full rebuilds.
    pub unpublished: HashSet<String>,
    /// The book's table of contents, for the same reason.
    pub book_index: Vec<EntrySummary>,
}

enum DocOutcome {
    Rendered {
        entry: EntrySummary,
        section: Option<String>,
        slug: String,
        rel_path: String,
        written: bool,
    },
    Failed(DocFailure),
}

/// Write content to path only if content has actually changed or file does not exist.
/// Returns Ok(true) if written, Ok(false) if skipped because unchanged.
pub fn write_if_changed(path: &Path, content: &str) -> Result<bool> {
    if path.exists()
        && let Ok(meta) = fs::metadata(path)
        && meta.len() == content.len() as u64
        && let Ok(existing) = fs::read_to_string(path)
        && existing == content
    {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(true)
}

/// The vault-wide state a single page needs in order to render.
///
/// The dev server holds all of this between edits; bundling it keeps the
/// per-page call about the page.
pub struct RenderContext<'a, 'r> {
    pub config: &'a BookConfig,
    pub vault_index: &'a tomet_links::VaultLinkIndex,
    pub workspace_cfg_src: Option<&'a str>,
    pub renderer: &'a BookRenderer<'r>,
    /// The documents the last full build left out, so a page re-rendered on
    /// its own still shows links to them as broken.
    pub unpublished: &'a HashSet<String>,
    /// The book's table of contents, as the last full build arranged it.
    pub book_index: &'a [EntrySummary],
}

pub fn render_single_document(
    abs_path: &Path,
    rel_path: &str,
    out_dir: &Path,
    cx: &RenderContext<'_, '_>,
    backlinks: &[Backlink],
) -> Result<Option<(bool, String)>> {
    let config = cx.config;
    let source = fs::read_to_string(abs_path)?;
    let (doc, kind) = document::parse_tomet_document(&source, rel_path)?;

    // Its `@kind` may be what changed. Nothing to write either way.
    if !kinds::publishes(kind.as_deref(), &config.build.kinds) {
        return Ok(None);
    }

    let processed = document::process_parsed_document(
        doc,
        rel_path,
        Some(abs_path),
        config,
        cx.vault_index,
        cx.workspace_cfg_src,
        cx.unpublished,
    )?;
    let html = cx
        .renderer
        .render_page(&processed, backlinks, cx.book_index)?;
    let out_html_path = out_dir
        .join(config.build.wiki_out_rel())
        .join(&processed.slug)
        .join("index.html");
    let written = write_if_changed(&out_html_path, &html)?;
    let url = format!("{}/{}", config.build.clean_url_prefix(), processed.slug);
    Ok(Some((written, url)))
}

/// Read the vault's `*.index.tmt` documents into one list, in filename order.
///
/// Returns `None` when the vault has none, which is what keeps the generated
/// catalog as the default.
fn read_written_index(scanned: &ScannedVault, src_dir: &Path) -> Option<Vec<catalog::IndexEntry>> {
    let index_files: Vec<&loader::DocFileInfo> = scanned
        .control_files
        .iter()
        .filter(|f| catalog::is_root_index(&f.rel_path))
        .collect();
    if index_files.is_empty() {
        return None;
    }

    // Built once and shared across every index file this build has, the
    // same way `vault_index` already is -- each `${filter(...)}` answers
    // from the vault as this build's own scan sees it, not a second,
    // independently-filtered walk.
    let rows = catalog::index_rows(&scanned.doc_files, src_dir);

    let mut entries = Vec::new();
    for file in index_files {
        let Ok(source) = fs::read_to_string(&file.abs_path) else {
            warn!("Could not read index {}", file.rel_path);
            continue;
        };

        let from_path = Path::new(&file.rel_path);
        let outline =
            catalog::parse_index_document(&source, from_path, &scanned.vault_index, &rows);

        if outline.kind_missing {
            warn!("{} does not declare @kind(doc.index)", file.rel_path);
        }
        for target in &outline.unresolved {
            warn!(
                "{}: nothing in the vault answers to '{target}'",
                file.rel_path
            );
        }
        for err in &outline.query_errors {
            warn!("{}: {err}", file.rel_path);
        }

        entries.extend(outline.entries);
    }

    Some(entries)
}

/// Give every slug the index names the page summary that was rendered for it.
///
/// A slug with no page behind it is dropped: the index may have been written
/// before the note, or the note may have failed to render.
fn arrange_entries(
    index: &[catalog::IndexEntry],
    by_slug: &HashMap<String, EntrySummary>,
) -> Vec<EntrySummary> {
    fn walk(
        index: &[catalog::IndexEntry],
        by_slug: &HashMap<String, EntrySummary>,
        out: &mut Vec<EntrySummary>,
    ) {
        for entry in index {
            let mut children = Vec::new();
            walk(&entry.children, by_slug, &mut children);

            match by_slug.get(&entry.slug) {
                Some(summary) => out.push(EntrySummary {
                    children,
                    ..summary.clone()
                }),
                // The page is missing, but what hung below it is not.
                None => out.extend(children),
            }
        }
    }

    let mut out = Vec::new();
    walk(index, by_slug, &mut out);
    out
}

/// Turn every page's outgoing links into the incoming list of its targets.
///
/// The result is sorted by title so that an unchanged vault keeps producing
/// byte-identical pages, which is what lets the writer skip untouched files.
fn build_backlink_index(
    docs: &[&ProcessedDoc],
    clean_url_prefix: &str,
) -> HashMap<String, Vec<Backlink>> {
    let mut backlinks: HashMap<String, Vec<Backlink>> = HashMap::new();
    for doc in docs {
        for target in &doc.outgoing {
            backlinks.entry(target.clone()).or_default().push(Backlink {
                url: format!("{clean_url_prefix}/{}", doc.slug),
                title: doc.title.clone(),
                section: doc.section.clone(),
            });
        }
    }
    for list in backlinks.values_mut() {
        list.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.url.cmp(&b.url)));
    }
    backlinks
}

/// Delete the pages named by `stale_slugs`.
///
/// The fast path: the previous build listed what it wrote, so a rename only
/// costs the removal itself rather than a walk of the whole site.
fn prune_listed_pages(wiki_root: &Path, stale_slugs: &[&str]) -> usize {
    let mut removed = 0;
    let mut touched_dirs = false;

    for slug in stale_slugs {
        let page = wiki_root.join(slug).join("index.html");
        if fs::remove_file(&page).is_ok() {
            removed += 1;
            touched_dirs = true;
        }
    }

    if touched_dirs {
        remove_empty_dirs(wiki_root);
    }
    removed
}

/// Delete the media named by `stale_rel`.
fn prune_listed_media(vault_root: &Path, stale_rel: &[&str]) -> usize {
    let mut removed = 0;
    let mut touched_dirs = false;

    for rel in stale_rel {
        if fs::remove_file(vault_root.join(rel)).is_ok() {
            removed += 1;
            touched_dirs = true;
        }
    }

    if touched_dirs {
        remove_empty_dirs(vault_root);
    }
    removed
}

/// Delete pages under `wiki_root` whose source document no longer exists.
///
/// The fallback for when no manifest is available -- a first build against an
/// existing output directory, or one written by a version that kept no record.
/// Without pruning, a renamed note keeps its old HTML forever and Pagefind
/// cheerfully indexes the orphan, so search leads to a dead page.
fn prune_stale_pages(wiki_root: &Path, keep_slugs: &HashSet<&str>) -> usize {
    if !wiki_root.is_dir() {
        return 0;
    }

    let mut removed = 0;
    for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !entry.file_type().is_file() || path.file_name() != Some(OsStr::new("index.html")) {
            continue;
        }

        let Some(parent) = path.parent() else {
            continue;
        };
        if parent == wiki_root {
            // The catalog page, not a document page.
            continue;
        }

        let Ok(rel) = parent.strip_prefix(wiki_root) else {
            continue;
        };
        let slug = rel.to_string_lossy().replace('\\', "/");
        if keep_slugs.contains(slug.as_str()) {
            continue;
        }

        if fs::remove_file(path).is_ok() {
            removed += 1;
        }
    }

    remove_empty_dirs(wiki_root);
    removed
}

/// Delete media under `vault_root` that is no longer present in the source vault.
fn prune_stale_media(vault_root: &Path, keep_rel: &HashSet<&str>) -> usize {
    if !vault_root.is_dir() {
        return 0;
    }

    let mut removed = 0;
    for entry in WalkDir::new(vault_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let Ok(rel) = entry.path().strip_prefix(vault_root) else {
            continue;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if keep_rel.contains(rel_str.as_str()) {
            continue;
        }

        if fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }

    remove_empty_dirs(vault_root);
    removed
}

/// Drop directories left empty by pruning, deepest first. The root itself stays.
fn remove_empty_dirs(root: &Path) {
    for entry in WalkDir::new(root)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path == root || !entry.file_type().is_dir() {
            continue;
        }
        // Fails while the directory still holds something, which is the point.
        let _ = fs::remove_dir(path);
    }
}

pub fn build_book(
    src_dir: &Path,
    out_dir: &Path,
    config: &BookConfig,
    is_dev: bool,
) -> Result<BuildReport> {
    info!(
        "Building book from {} to {}",
        src_dir.display(),
        out_dir.display()
    );

    // 1. Scan Vault
    let scanned = scan_vault(src_dir, config).context("Failed to scan vault")?;
    info!(
        "Found {} documents and {} media files",
        scanned.doc_files.len(),
        scanned.media_files.len()
    );

    // 2. Write static assets (book.css, book.js, custom.css)
    write_static_assets(out_dir, src_dir, config).context("Failed to write static assets")?;

    // 3. Copy/Link media into the asset directory
    let media = copy_vault_media(out_dir, &scanned.media_files, config)
        .context("Failed to copy vault media")?;
    info!(
        "Media: {} unchanged, {} linked, {} copied (of {})",
        media.unchanged,
        media.linked,
        media.copied,
        media.total()
    );
    if media.copied > 0 {
        // Copying is only the fallback, so a build that keeps doing it is
        // moving every byte again -- the vault and the output are on
        // filesystems that cannot share a hard link.
        warn!(
            "Copied {} media file(s) because they could not be hard linked; \
             put the output directory on the same filesystem as the vault to avoid this",
            media.copied
        );
    }

    // 4. Initialize MiniJinja renderer
    let renderer = BookRenderer::new(config, is_dev)?;

    // Pages live where the links point, so both come from `url_prefix`.
    let wiki_rel = config.build.wiki_out_rel();
    let wiki_root = out_dir.join(wiki_rel);
    let clean_url_prefix = config.build.clean_url_prefix();

    // 5. Process every document.
    //
    // Backlinks are what force two passes: a page cannot say who points at it
    // until every other page has been read. Parsing happens once and the
    // results are kept, so the second pass only fills in the template.
    // Read every document once, and keep what came back: the `@kind` decides
    // what the site publishes, and that has to be settled before any link is
    // resolved, since a link to a document left out has to come out broken.
    info!("Parsing {} document(s)...", scanned.doc_files.len());
    let parsed: Vec<Result<(tomet_ast::Document, Option<String>), DocFailure>> = scanned
        .doc_files
        .par_iter()
        .map(|doc_file| {
            let fail = |error: String| DocFailure {
                rel_path: doc_file.rel_path.clone(),
                error,
            };

            let source = fs::read_to_string(&doc_file.abs_path)
                .map_err(|e| fail(format!("could not read file: {e}")))?;

            document::parse_tomet_document(&source, &doc_file.rel_path)
                .map_err(|e| fail(format!("could not parse document: {e}")))
        })
        .collect();

    let unpublished: HashSet<String> = parsed
        .iter()
        .zip(scanned.doc_files.iter())
        .filter(|(result, _)| match result {
            Ok((_, kind)) => !kinds::publishes(kind.as_deref(), &config.build.kinds),
            // A document that would not parse is not published either, but it
            // is reported as a failure rather than a choice.
            Err(_) => false,
        })
        .map(|(_, doc_file)| doc_file.rel_path.clone())
        .collect();

    if !unpublished.is_empty() {
        info!(
            "Leaving out {} document(s) whose @kind the book does not publish",
            unpublished.len()
        );
    }

    info!(
        "Processing {} document(s)...",
        scanned.doc_files.len() - unpublished.len()
    );
    let processed: Vec<Result<ProcessedDoc, DocFailure>> = parsed
        .into_par_iter()
        .zip(scanned.doc_files.par_iter())
        .filter(|(_, doc_file)| !unpublished.contains(&doc_file.rel_path))
        .map(|(result, doc_file)| {
            let (doc, _) = result?;
            document::process_parsed_document(
                doc,
                &doc_file.rel_path,
                Some(&doc_file.abs_path),
                config,
                &scanned.vault_index,
                scanned.workspace_config_src.as_deref(),
                &unpublished,
            )
            .map_err(|e| DocFailure {
                rel_path: doc_file.rel_path.clone(),
                error: format!("could not process document: {e}"),
            })
        })
        .collect();

    // 6. Reverse the links: who points at each page.
    let docs: Vec<&ProcessedDoc> = processed.iter().filter_map(|r| r.as_ref().ok()).collect();
    let backlinks = build_backlink_index(&docs, clean_url_prefix);

    // The book's own table of contents. Every page shows it, so it has to be
    // settled before the first one is rendered.
    let entries_by_slug: HashMap<String, EntrySummary> = docs
        .iter()
        .map(|doc| {
            (
                doc.slug.clone(),
                EntrySummary {
                    url: format!("{clean_url_prefix}/{}", doc.slug),
                    title: doc.title.clone(),
                    section: doc.section.clone(),
                    children: Vec::new(),
                },
            )
        })
        .collect();

    let written_index = read_written_index(&scanned, src_dir);
    let book_index: Vec<EntrySummary> = match &written_index {
        Some(index) => arrange_entries(index, &entries_by_slug),
        None => Vec::new(),
    };

    // Search's tag chips need the vault's section names regardless of
    // whether the vault has a written index at all, so this is built from
    // the map directly rather than from `book_index` (empty) or
    // `processed_entries` (not settled until after the render pass below).
    // Written once here as its own static file -- exactly how book.css and
    // book.js already work -- rather than inlined into every page's own
    // context.
    let mut lookup_section_totals: HashMap<String, usize> = HashMap::new();
    for entry in entries_by_slug.values() {
        if let Some(sec) = &entry.section {
            *lookup_section_totals.entry(sec.clone()).or_insert(0) += 1;
        }
    }
    let mut lookup_sections: Vec<SectionSummary> = lookup_section_totals
        .into_iter()
        .map(|(name, count)| SectionSummary { name, count })
        .collect();
    lookup_sections.sort_by(|a, b| a.name.cmp(&b.name));

    write_if_changed(
        &out_dir.join("lookup/manifest.json"),
        &serde_json::to_string(&LookupManifest {
            sections: lookup_sections,
        })?,
    )?;
    // A-Z browsing (and the per-character lookup/buckets/*.json shards it
    // used to page through) was tried and dropped -- not worth the added
    // complexity for this vault's actual usage. Nothing writes that
    // directory anymore, so a leftover one from before is just removed
    // rather than left to rot as dead output.
    let _ = fs::remove_dir_all(out_dir.join("lookup/buckets"));

    // 7. Render and write.
    info!("Rendering {} document(s)...", processed.len());
    // Reported roughly every 2 seconds of wall time rather than at some
    // fraction of the total count: a fixed fraction (e.g. every 5%) means a
    // handful of documents on a small vault, but on a 20,000-document one it
    // stretched to a full minute of silence between lines -- which is
    // exactly the "did it hang?" feeling this was meant to fix. A shared
    // atomic clock (rather than a per-thread timer, or a Mutex every
    // document) keeps at most one of rayon's worker threads actually
    // printing at a time, however many are rendering in parallel.
    //
    // In an interactive terminal each tick overwrites the last one instead
    // of appending a new log line -- a few thousand lines of "N/M rendered"
    // is not progress, it's scrollback. Piped to a file or CI, `\r` is just
    // noise with nothing to overwrite, so that case keeps one real line per
    // tick instead.
    let live_progress = std::io::stdout().is_terminal();
    let rendered_count = AtomicUsize::new(0);
    let last_progress_log_ms = AtomicU64::new(0);
    let render_start = Instant::now();
    let total_to_render = processed.len();
    let results: Vec<DocOutcome> = processed
        .par_iter()
        .map(|result| {
            let processed = match result {
                Ok(p) => p,
                Err(failure) => return DocOutcome::Failed(failure.clone()),
            };
            let fail = |error: String| {
                DocOutcome::Failed(DocFailure {
                    rel_path: processed.rel_path.clone(),
                    error,
                })
            };

            let incoming: &[Backlink] = backlinks
                .get(&processed.slug)
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            let html = match renderer.render_page(processed, incoming, &book_index) {
                Ok(h) => h,
                Err(e) => return fail(format!("could not render template: {e}")),
            };

            let out_html_path = wiki_root.join(&processed.slug).join("index.html");
            let written = match write_if_changed(&out_html_path, &html) {
                Ok(w) => w,
                Err(e) => return fail(format!("could not write {}: {e}", out_html_path.display())),
            };

            let done = rendered_count.fetch_add(1, Ordering::Relaxed) + 1;
            if done == total_to_render {
                // Always shown, win or lose the race below, so the log ends
                // on a definite "every page accounted for" line.
                if live_progress {
                    print!("\r  ...{done}/{total_to_render} rendered\x1b[K\n");
                    let _ = std::io::stdout().flush();
                } else {
                    info!("  ...{done}/{total_to_render} rendered");
                }
            } else {
                let elapsed_ms = render_start.elapsed().as_millis() as u64;
                let last = last_progress_log_ms.load(Ordering::Relaxed);
                if elapsed_ms.saturating_sub(last) >= 2000
                    && last_progress_log_ms
                        .compare_exchange(last, elapsed_ms, Ordering::Relaxed, Ordering::Relaxed)
                        .is_ok()
                {
                    if live_progress {
                        print!("\r  ...{done}/{total_to_render} rendered\x1b[K");
                        let _ = std::io::stdout().flush();
                    } else {
                        info!("  ...{done}/{total_to_render} rendered");
                    }
                }
            }

            DocOutcome::Rendered {
                entry: EntrySummary {
                    url: format!("{clean_url_prefix}/{}", processed.slug),
                    title: processed.title.clone(),
                    section: processed.section.clone(),
                    children: Vec::new(),
                },
                section: processed.section.clone(),
                slug: processed.slug.clone(),
                rel_path: processed.rel_path.clone(),
                written,
            }
        })
        .collect();

    let mut written = 0;
    let mut failures: Vec<DocFailure> = Vec::new();
    let mut processed_entries = Vec::with_capacity(results.len());
    let mut section_counts: HashMap<String, usize> = HashMap::new();
    let mut slug_owner: HashMap<String, String> = HashMap::new();

    for outcome in results {
        match outcome {
            DocOutcome::Rendered {
                entry,
                section,
                slug,
                rel_path,
                written: was_written,
            } => {
                if was_written {
                    written += 1;
                }
                if let Some(sec) = section {
                    *section_counts.entry(sec).or_insert(0) += 1;
                }
                if let Some(previous) = slug_owner.insert(slug.clone(), rel_path.clone()) {
                    warn!(
                        "Slug collision: '{previous}' and '{rel_path}' both render to {clean_url_prefix}/{slug}"
                    );
                }
                processed_entries.push(entry);
            }
            DocOutcome::Failed(failure) => {
                warn!("Skipping {}: {}", failure.rel_path, failure.error);
                failures.push(failure);
            }
        }
    }
    let rendered = processed_entries.len();

    info!(
        "Rendered {rendered} document pages ({written} written, {} unchanged)",
        rendered - written
    );

    // 6. Render the catalog page
    let mut sections: Vec<SectionSummary> = section_counts
        .into_iter()
        .map(|(name, count)| SectionSummary { name, count })
        .collect();
    sections.sort_by(|a, b| a.name.cmp(&b.name));

    // A written index replaces the generated listing outright: the order is
    // the author's, and a page they left out is a page they do not want
    // listed. Without one, the catalog stays what it was -- everything, in
    // whatever order the vault walk found it.
    let catalog_entries = match written_index {
        Some(_) => book_index.clone(),
        None => processed_entries.clone(),
    };

    let wiki_index_html = renderer.render_index(&sections, &catalog_entries)?;
    let catalog_written = write_if_changed(&wiki_root.join("index.html"), &wiki_index_html)?;

    // 7. Write root /index.html redirecting to the book, unless the book is already there
    let mut redirect_written = false;
    if !wiki_rel.is_empty() {
        let redirect_html = format!(
            r#"<!doctype html><html lang="{lang}" data-pagefind-ignore><head><meta charset="utf-8"><title>Redirecting to: {prefix}</title><meta http-equiv="refresh" content="0;url={prefix}"><link rel="canonical" href="{prefix}"></head><body><a href="{prefix}">Redirecting to {prefix}</a></body></html>"#,
            lang = config.book.lang,
            prefix = clean_url_prefix
        );
        redirect_written = write_if_changed(&out_dir.join("index.html"), &redirect_html)?;
    }

    // 8. Drop output left behind by deleted or renamed sources
    let previous = BuildManifest::read(out_dir);
    let manifest = BuildManifest {
        pages: slug_owner.keys().cloned().collect(),
        media: scanned
            .media_files
            .iter()
            .map(|m| m.rel_path.clone())
            .collect(),
    };

    let mut pruned_pages = 0;
    let mut pruned_media = 0;
    let asset_rel = config.build.asset_out_rel();

    if wiki_rel.is_empty() {
        warn!("url_prefix is '/', so stale pages cannot be pruned safely; skipping page cleanup");
    } else if let Some(previous) = &previous {
        let current: HashSet<&str> = manifest.pages.iter().map(String::as_str).collect();
        let stale: Vec<&str> = previous
            .pages
            .iter()
            .map(String::as_str)
            .filter(|slug| !current.contains(slug))
            .collect();
        pruned_pages = prune_listed_pages(&wiki_root, &stale);
    } else {
        // No record of the last build: fall back to inspecting the output.
        let keep: HashSet<&str> = manifest.pages.iter().map(String::as_str).collect();
        pruned_pages = prune_stale_pages(&wiki_root, &keep);
    }

    if !wiki_rel.is_empty() {
        // Control documents used to be published as pages. The manifest only
        // knows what the last build wrote, so an output directory carried over
        // from a version that published them would keep serving those pages
        // forever; name them directly.
        let control_slugs: Vec<&str> = scanned
            .control_files
            .iter()
            .map(|f| document::strip_doc_extension(&f.rel_path))
            .collect();
        pruned_pages += prune_listed_pages(&wiki_root, &control_slugs);
    }

    if asset_rel.is_empty() {
        warn!(
            "asset_prefix is '/', so stale media cannot be pruned safely; skipping media cleanup"
        );
    } else if let Some(previous) = &previous {
        let current: HashSet<&str> = manifest.media.iter().map(String::as_str).collect();
        let stale: Vec<&str> = previous
            .media
            .iter()
            .map(String::as_str)
            .filter(|rel| !current.contains(rel))
            .collect();
        pruned_media = prune_listed_media(&out_dir.join(asset_rel), &stale);
    } else {
        let keep: HashSet<&str> = manifest.media.iter().map(String::as_str).collect();
        pruned_media = prune_stale_media(&out_dir.join(asset_rel), &keep);
    }

    if pruned_pages > 0 || pruned_media > 0 {
        info!("Pruned {pruned_pages} stale page(s) and {pruned_media} stale media file(s)");
    }

    manifest.write(out_dir)?;

    // 9. Run Pagefind, but only when the pages it would index actually moved.
    //    Re-indexing an unchanged site is by far the most expensive thing a
    //    build can do, and it produces the same index every time.
    if config.build.pagefind {
        let output_changed = written > 0 || catalog_written || redirect_written || pruned_pages > 0;
        let index_missing = !out_dir.join("pagefind").is_dir();

        if output_changed || index_missing {
            let _ = run_pagefind(out_dir);
        } else {
            info!("No page changed, keeping the existing Pagefind index");
        }
    }

    let report = BuildReport {
        rendered,
        written,
        failures,
        pruned_pages,
        pruned_media,
        scanned,
        backlinks,
        unpublished,
        book_index,
    };

    if report.failures.is_empty() {
        info!("Build completed successfully: {}", out_dir.display());
    } else {
        warn!(
            "Build completed with {} document(s) left out: {}",
            report.failures.len(),
            out_dir.display()
        );
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory; avoids pulling in a temp-file dependency.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tmtbook-test-{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x").unwrap();
    }

    /// A document with only the fields the backlink index reads.
    fn doc(slug: &str, title: &str, section: Option<&str>, outgoing: &[&str]) -> ProcessedDoc {
        ProcessedDoc {
            slug: slug.to_string(),
            rel_path: format!("{slug}.tmt"),
            source_path: format!("/vault/{slug}.tmt"),
            title: title.to_string(),
            section: section.map(str::to_string),
            kind: None,
            primary_color: None,
            icon: None,
            banner_url: None,
            banner_y: None,
            images: Vec::new(),
            hero_chips: Vec::new(),
            infobox_rows: Vec::new(),
            has_data: false,
            toc: Vec::new(),
            section_tabs: Vec::new(),
            body_html: String::new(),
            outgoing: outgoing.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn summary(slug: &str, title: &str) -> EntrySummary {
        EntrySummary {
            url: format!("/wiki/{slug}"),
            title: title.to_string(),
            section: None,
            children: Vec::new(),
        }
    }

    fn listed(slugs: &[(&str, &[&str])]) -> Vec<catalog::IndexEntry> {
        slugs
            .iter()
            .map(|(slug, kids)| catalog::IndexEntry {
                slug: slug.to_string(),
                children: kids
                    .iter()
                    .map(|k| catalog::IndexEntry {
                        slug: k.to_string(),
                        children: Vec::new(),
                    })
                    .collect(),
            })
            .collect()
    }

    fn rendered(slugs: &[&str]) -> HashMap<String, EntrySummary> {
        slugs
            .iter()
            .map(|s| (s.to_string(), summary(s, &s.to_uppercase())))
            .collect()
    }

    #[test]
    fn the_catalog_follows_the_written_order() {
        let out = arrange_entries(
            &listed(&[("zebra", &[]), ("apple", &[])]),
            &rendered(&["apple", "zebra"]),
        );

        let titles: Vec<&str> = out.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["ZEBRA", "APPLE"]);
    }

    #[test]
    fn a_page_the_index_leaves_out_is_not_listed() {
        let out = arrange_entries(&listed(&[("apple", &[])]), &rendered(&["apple", "hidden"]));

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "APPLE");
    }

    #[test]
    fn nesting_survives_the_lookup() {
        let out = arrange_entries(
            &listed(&[("guide", &["install"])]),
            &rendered(&["guide", "install"]),
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].children.len(), 1);
        assert_eq!(out[0].children[0].title, "INSTALL");
    }

    #[test]
    fn a_named_page_that_does_not_exist_hands_its_children_up() {
        // The index may have been written before the note, or the note may
        // have failed to render. What hung below it is still real.
        let out = arrange_entries(&listed(&[("ghost", &["install"])]), &rendered(&["install"]));

        let titles: Vec<&str> = out.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["INSTALL"]);
    }

    #[test]
    fn backlinks_point_from_every_linking_page() {
        let a = doc("a", "Aardvark", Some("Animals"), &["hub"]);
        let b = doc("b", "Bison", None, &["hub", "a"]);
        let hub = doc("hub", "Hub", None, &[]);
        let index = build_backlink_index(&[&a, &b, &hub], "/wiki");

        let into_hub = &index["hub"];
        assert_eq!(into_hub.len(), 2);
        assert_eq!(into_hub[0].title, "Aardvark");
        assert_eq!(into_hub[0].url, "/wiki/a");
        assert_eq!(into_hub[0].section.as_deref(), Some("Animals"));
        assert_eq!(into_hub[1].title, "Bison");
        assert_eq!(into_hub[1].section, None);

        assert_eq!(index["a"].len(), 1, "b links to a");
        assert!(!index.contains_key("b"), "nothing links to b");
    }

    #[test]
    fn backlinks_are_ordered_by_title_then_url() {
        // Two pages share a title; the url settles the tie.
        let z = doc("z", "Same", None, &["t"]);
        let m = doc("m", "Same", None, &["t"]);
        let first = doc("first", "Alpha", None, &["t"]);
        let index = build_backlink_index(&[&z, &m, &first], "/wiki");

        let urls: Vec<&str> = index["t"].iter().map(|b| b.url.as_str()).collect();
        assert_eq!(urls, ["/wiki/first", "/wiki/m", "/wiki/z"]);
    }

    #[test]
    fn a_vault_without_links_has_an_empty_index() {
        let a = doc("a", "A", None, &[]);
        assert!(build_backlink_index(&[&a], "/wiki").is_empty());
    }

    #[test]
    fn the_url_prefix_reaches_the_backlink_urls() {
        let a = doc("a", "A", None, &["b"]);
        let index = build_backlink_index(&[&a], "/docs/book");
        assert_eq!(index["b"][0].url, "/docs/book/a");
    }

    #[test]
    fn stale_pages_are_pruned_and_live_ones_kept() {
        let root = scratch("prune-pages");

        touch(&root.join("keep/index.html"));
        touch(&root.join("notes/gone/index.html"));
        touch(&root.join("index.html")); // the catalog page

        let keep: HashSet<&str> = ["keep"].into_iter().collect();
        let removed = prune_stale_pages(&root, &keep);

        assert_eq!(removed, 1);
        assert!(root.join("keep/index.html").exists());
        assert!(root.join("index.html").exists(), "catalog must survive");
        assert!(!root.join("notes/gone/index.html").exists());
        assert!(
            !root.join("notes/gone").exists(),
            "emptied directories should go too"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pruning_an_untouched_tree_removes_nothing() {
        let root = scratch("prune-none");
        touch(&root.join("a/index.html"));
        touch(&root.join("b/index.html"));

        let keep: HashSet<&str> = ["a", "b"].into_iter().collect();
        assert_eq!(prune_stale_pages(&root, &keep), 0);
        assert!(root.join("a/index.html").exists());
        assert!(root.join("b/index.html").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_media_is_pruned() {
        let root = scratch("prune-media");
        touch(&root.join("img/keep.png"));
        touch(&root.join("img/old.png"));

        let keep: HashSet<&str> = ["img/keep.png"].into_iter().collect();
        assert_eq!(prune_stale_media(&root, &keep), 1);
        assert!(root.join("img/keep.png").exists());
        assert!(!root.join("img/old.png").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pruning_a_missing_directory_is_a_no_op() {
        let root = scratch("prune-missing").join("does-not-exist");
        assert_eq!(prune_stale_pages(&root, &HashSet::new()), 0);
        assert_eq!(prune_stale_media(&root, &HashSet::new()), 0);
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tmtbook-manifest-{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x").unwrap();
    }

    /// A document with only the fields the backlink index reads.
    fn doc(slug: &str, title: &str, section: Option<&str>, outgoing: &[&str]) -> ProcessedDoc {
        ProcessedDoc {
            slug: slug.to_string(),
            rel_path: format!("{slug}.tmt"),
            source_path: format!("/vault/{slug}.tmt"),
            title: title.to_string(),
            section: section.map(str::to_string),
            kind: None,
            primary_color: None,
            icon: None,
            banner_url: None,
            banner_y: None,
            images: Vec::new(),
            hero_chips: Vec::new(),
            infobox_rows: Vec::new(),
            has_data: false,
            toc: Vec::new(),
            section_tabs: Vec::new(),
            body_html: String::new(),
            outgoing: outgoing.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn summary(slug: &str, title: &str) -> EntrySummary {
        EntrySummary {
            url: format!("/wiki/{slug}"),
            title: title.to_string(),
            section: None,
            children: Vec::new(),
        }
    }

    fn listed(slugs: &[(&str, &[&str])]) -> Vec<catalog::IndexEntry> {
        slugs
            .iter()
            .map(|(slug, kids)| catalog::IndexEntry {
                slug: slug.to_string(),
                children: kids
                    .iter()
                    .map(|k| catalog::IndexEntry {
                        slug: k.to_string(),
                        children: Vec::new(),
                    })
                    .collect(),
            })
            .collect()
    }

    fn rendered(slugs: &[&str]) -> HashMap<String, EntrySummary> {
        slugs
            .iter()
            .map(|s| (s.to_string(), summary(s, &s.to_uppercase())))
            .collect()
    }

    #[test]
    fn the_catalog_follows_the_written_order() {
        let out = arrange_entries(
            &listed(&[("zebra", &[]), ("apple", &[])]),
            &rendered(&["apple", "zebra"]),
        );

        let titles: Vec<&str> = out.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["ZEBRA", "APPLE"]);
    }

    #[test]
    fn a_page_the_index_leaves_out_is_not_listed() {
        let out = arrange_entries(&listed(&[("apple", &[])]), &rendered(&["apple", "hidden"]));

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "APPLE");
    }

    #[test]
    fn nesting_survives_the_lookup() {
        let out = arrange_entries(
            &listed(&[("guide", &["install"])]),
            &rendered(&["guide", "install"]),
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].children.len(), 1);
        assert_eq!(out[0].children[0].title, "INSTALL");
    }

    #[test]
    fn a_named_page_that_does_not_exist_hands_its_children_up() {
        // The index may have been written before the note, or the note may
        // have failed to render. What hung below it is still real.
        let out = arrange_entries(&listed(&[("ghost", &["install"])]), &rendered(&["install"]));

        let titles: Vec<&str> = out.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["INSTALL"]);
    }

    #[test]
    fn backlinks_point_from_every_linking_page() {
        let a = doc("a", "Aardvark", Some("Animals"), &["hub"]);
        let b = doc("b", "Bison", None, &["hub", "a"]);
        let hub = doc("hub", "Hub", None, &[]);
        let index = build_backlink_index(&[&a, &b, &hub], "/wiki");

        let into_hub = &index["hub"];
        assert_eq!(into_hub.len(), 2);
        assert_eq!(into_hub[0].title, "Aardvark");
        assert_eq!(into_hub[0].url, "/wiki/a");
        assert_eq!(into_hub[0].section.as_deref(), Some("Animals"));
        assert_eq!(into_hub[1].title, "Bison");
        assert_eq!(into_hub[1].section, None);

        assert_eq!(index["a"].len(), 1, "b links to a");
        assert!(!index.contains_key("b"), "nothing links to b");
    }

    #[test]
    fn backlinks_are_ordered_by_title_then_url() {
        // Two pages share a title; the url settles the tie.
        let z = doc("z", "Same", None, &["t"]);
        let m = doc("m", "Same", None, &["t"]);
        let first = doc("first", "Alpha", None, &["t"]);
        let index = build_backlink_index(&[&z, &m, &first], "/wiki");

        let urls: Vec<&str> = index["t"].iter().map(|b| b.url.as_str()).collect();
        assert_eq!(urls, ["/wiki/first", "/wiki/m", "/wiki/z"]);
    }

    #[test]
    fn a_vault_without_links_has_an_empty_index() {
        let a = doc("a", "A", None, &[]);
        assert!(build_backlink_index(&[&a], "/wiki").is_empty());
    }

    #[test]
    fn the_url_prefix_reaches_the_backlink_urls() {
        let a = doc("a", "A", None, &["b"]);
        let index = build_backlink_index(&[&a], "/docs/book");
        assert_eq!(index["b"][0].url, "/docs/book/a");
    }

    #[test]
    fn a_manifest_round_trips() {
        let dir = scratch("roundtrip");
        let manifest = BuildManifest {
            pages: vec!["a".into(), "notes/b".into()],
            media: vec!["img/c.png".into()],
        };
        manifest.write(&dir).unwrap();

        let read = BuildManifest::read(&dir).expect("manifest should be readable");
        assert_eq!(read.pages, manifest.pages);
        assert_eq!(read.media, manifest.media);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_corrupt_manifest_reads_as_none() {
        let dir = scratch("missing");
        assert!(BuildManifest::read(&dir).is_none());

        fs::write(dir.join(MANIFEST_FILE), "not json").unwrap();
        assert!(BuildManifest::read(&dir).is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn listed_pruning_removes_only_what_it_is_given() {
        let root = scratch("listed-pages");
        touch(&root.join("keep/index.html"));
        touch(&root.join("notes/gone/index.html"));

        assert_eq!(prune_listed_pages(&root, &["notes/gone"]), 1);
        assert!(root.join("keep/index.html").exists());
        assert!(!root.join("notes/gone").exists(), "emptied dir should go");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn listed_pruning_tolerates_paths_that_are_already_gone() {
        let root = scratch("listed-missing");
        touch(&root.join("keep/index.html"));

        assert_eq!(prune_listed_pages(&root, &["never-existed"]), 0);
        assert!(root.join("keep/index.html").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn listed_media_pruning_removes_only_what_it_is_given() {
        let root = scratch("listed-media");
        touch(&root.join("img/keep.png"));
        touch(&root.join("img/old.png"));

        assert_eq!(prune_listed_media(&root, &["img/old.png"]), 1);
        assert!(root.join("img/keep.png").exists());
        assert!(!root.join("img/old.png").exists());

        let _ = fs::remove_dir_all(&root);
    }
}
