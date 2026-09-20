//! Turning an entire vault into a published book.

pub mod assets;
pub mod catalog;
pub mod document;
pub mod image_opt;
pub mod kinds;
pub mod loader;
pub(crate) mod manifest;
pub mod pagefind;
pub(crate) mod prune;
pub mod renderer;
pub mod search;
pub mod slug;
pub mod state;

use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tracing::{info, warn};

pub use state::{RenderContext, VaultState, render_single_document, render_single_document_html};

use crate::config::{BookConfig, SearchEngine};
use assets::{copy_vault_media, write_static_assets};
use document::ProcessedDoc;
use loader::{ScannedVault, scan_vault};
use manifest::{BuildManifest, LookupManifest};
use pagefind::run_pagefind;
use prune::{prune_listed_media, prune_listed_pages, prune_stale_media, prune_stale_pages};
use renderer::{Backlink, BookRenderer, EntrySummary, SectionSummary};

pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Normalize a path string to use forward slashes.
pub fn normalize_path_str(s: &str) -> String {
    s.replace('\\', "/")
}

/// A document that could not be turned into a page.
#[derive(Debug, Clone)]
pub struct DocFailure {
    pub rel_path: String,
    pub error: String,
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
    /// Note icon HTML snippets keyed by slug, for internal link decoration.
    pub doc_icons: HashMap<String, String>,
    /// Global routing table mapping document paths to slugs.
    pub route_table: slug::RouteTable,
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
    if let Ok(meta) = fs::metadata(path)
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
pub(crate) fn read_written_index(
    scanned: &ScannedVault,
    parsed: &[Result<(tomet_ast::Document, Option<String>), DocFailure>],
    src_dir: &Path,
    route_table: Option<&slug::RouteTable>,
) -> Option<Vec<catalog::IndexEntry>> {
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
    // Reuses the ASTs already parsed during the build pipeline.
    let rows = catalog::index_rows(
        parsed
            .par_iter()
            .zip(scanned.doc_files.par_iter())
            .filter_map(|(res, file)| res.as_ref().ok().map(|(doc, _)| (doc, file))),
        src_dir,
    );

    let mut entries = Vec::new();
    for file in index_files {
        let Ok(source) = fs::read_to_string(&file.abs_path) else {
            warn!("Could not read index {}", file.rel_path);
            continue;
        };

        let from_path = Path::new(&file.rel_path);
        let outline = catalog::parse_index_document(
            &source,
            from_path,
            &scanned.vault_index,
            &rows,
            route_table,
        );

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
pub(crate) fn arrange_entries(
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
pub(crate) fn build_backlink_index(
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

    // 2. Write static assets (tmtbook.css, tmtbook.js, custom.css)
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

    let mut route_meta = Vec::new();
    for (res, doc_file) in parsed.iter().zip(scanned.doc_files.iter()) {
        if unpublished.contains(&doc_file.rel_path) {
            continue;
        }
        if let Ok((doc, _)) = res {
            let (slug, id) = slug::extract_route_metadata(doc);
            route_meta.push((doc_file.rel_path.as_str(), slug, id));
        }
    }
    let doc_route_inputs: Vec<slug::DocRouteInput<'_>> = route_meta
        .iter()
        .map(|(path, slug, id)| slug::DocRouteInput {
            rel_path: path,
            explicit_slug: slug.as_deref(),
            meta_id: id.as_deref(),
        })
        .collect();
    let route_table = slug::RouteTable::build(&doc_route_inputs, &config.build)?;

    let written_index = read_written_index(&scanned, &parsed, src_dir, Some(&route_table));

    info!(
        "Processing {} document(s)...",
        scanned.doc_files.len() - unpublished.len()
    );
    let mut processed: Vec<Result<ProcessedDoc, DocFailure>> = parsed
        .into_par_iter()
        .zip(scanned.doc_files.par_iter())
        .filter(|(_, doc_file)| !unpublished.contains(&doc_file.rel_path))
        .map(|(result, doc_file)| {
            let (doc, _) = result?;
            document::process_parsed_document_with_blocks(
                doc,
                &doc_file.rel_path,
                Some(&doc_file.abs_path),
                config,
                &scanned.vault_index,
                &scanned.workspace_config_blocks,
                &unpublished,
                Some(&route_table),
            )
            .map_err(|e| DocFailure {
                rel_path: doc_file.rel_path.clone(),
                error: format!("could not process document: {e}"),
            })
        })
        .collect();

    // 5.5. Optimize referenced banners and profile images
    image_opt::optimize_docs_media(&mut processed, src_dir, out_dir, config);

    // 5.6. Enhance internal links with target note icons
    let doc_icons: HashMap<String, String> = if config.ui.links.note_icons {
        let note_icons: HashMap<String, String> = processed
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .filter_map(|doc| {
                doc.link_icon
                    .as_ref()
                    .map(|icon| (doc.slug.clone(), icon.clone()))
            })
            .collect();

        if !note_icons.is_empty() {
            let clean_url_prefix = config.build.clean_url_prefix();
            processed.par_iter_mut().for_each(|res| {
                if let Ok(doc) = res {
                    doc.body_html = document::enhance_internal_links(
                        &doc.body_html,
                        &note_icons,
                        clean_url_prefix,
                    );
                }
            });
        }
        note_icons
    } else {
        HashMap::new()
    };

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

    let book_index: Vec<EntrySummary> = match &written_index {
        Some(index) => arrange_entries(index, &entries_by_slug),
        None => Vec::new(),
    };

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

    // 6. Render the catalog page and write lookup manifest
    let mut sections: Vec<SectionSummary> = section_counts
        .into_iter()
        .map(|(name, count)| SectionSummary { name, count })
        .collect();
    sections.sort_by(|a, b| a.name.cmp(&b.name));

    write_if_changed(
        &out_dir.join("lookup/manifest.json"),
        &serde_json::to_string(&LookupManifest {
            sections: sections.clone(),
        })?,
    )?;

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

    // 9. Search indexing based on configured search engine
    match config.build.search {
        SearchEngine::Native => {
            let start = Instant::now();
            let search_index = search::build_search_index(&docs, clean_url_prefix);
            let search_json = search_index.to_json()?;
            let search_index_path = out_dir.join("search-index.json");
            write_if_changed(&search_index_path, &search_json)?;
            info!(
                "⚡ Native search index generated in {:?} ({} documents, {} bytes)",
                start.elapsed(),
                search_index.docs.len(),
                search_json.len()
            );
        }
        SearchEngine::Pagefind => {
            let output_changed =
                written > 0 || catalog_written || redirect_written || pruned_pages > 0;
            let index_missing = !out_dir.join("pagefind").is_dir();

            if output_changed || index_missing {
                let _ = run_pagefind(out_dir);
            } else {
                info!("No page changed, keeping the existing Pagefind index");
            }
        }
        SearchEngine::None => {
            info!("Search indexing skipped (search = none)");
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
        doc_icons,
        route_table,
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
mod tests;
