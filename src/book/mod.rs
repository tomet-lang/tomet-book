pub mod assets;
pub mod document;
pub mod loader;
pub mod pagefind;
pub mod renderer;

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::config::BookConfig;
use assets::{copy_vault_media, write_static_assets};
use document::process_tomet_document;
use loader::scan_vault;
use pagefind::run_pagefind;
use renderer::{BookRenderer, EntrySummary, SectionSummary};

use rayon::prelude::*;

/// A document that could not be turned into a page.
#[derive(Debug, Clone)]
pub struct DocFailure {
    pub rel_path: String,
    pub error: String,
}

/// What a build actually did, so callers can report on it or refuse to ship it.
#[derive(Debug, Default)]
pub struct BuildReport {
    pub rendered: usize,
    pub written: usize,
    pub failures: Vec<DocFailure>,
    pub pruned_pages: usize,
    pub pruned_media: usize,
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
    if path.exists() {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() == content.len() as u64 {
                if let Ok(existing) = fs::read_to_string(path) {
                    if existing == content {
                        return Ok(false);
                    }
                }
            }
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(true)
}

pub fn render_single_document(
    abs_path: &Path,
    rel_path: &str,
    out_dir: &Path,
    config: &BookConfig,
    vault_index: &tomet_links::VaultLinkIndex,
    workspace_cfg_src: Option<&str>,
    renderer: &BookRenderer,
) -> Result<(bool, String)> {
    let source = fs::read_to_string(abs_path)?;
    let processed = process_tomet_document(
        &source,
        rel_path,
        Some(abs_path),
        config,
        vault_index,
        workspace_cfg_src,
    )?;
    let html = renderer.render_page(&processed)?;
    let out_html_path = out_dir
        .join(config.build.wiki_out_rel())
        .join(&processed.slug)
        .join("index.html");
    let written = write_if_changed(&out_html_path, &html)?;
    let url = format!("{}/{}", config.build.clean_url_prefix(), processed.slug);
    Ok((written, url))
}

/// Delete pages under `wiki_root` whose source document no longer exists.
///
/// Without this a renamed note keeps its old HTML forever, and Pagefind
/// cheerfully indexes the orphan -- search then leads to a dead page.
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

pub fn build_book(src_dir: &Path, out_dir: &Path, config: &BookConfig) -> Result<BuildReport> {
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
    let copied_media = copy_vault_media(out_dir, &scanned.media_files, config)
        .context("Failed to copy vault media")?;
    info!("Linked/copied {} media assets", copied_media);

    // 4. Initialize MiniJinja renderer
    let renderer = BookRenderer::new(config)?;

    // Pages live where the links point, so both come from `url_prefix`.
    let wiki_rel = config.build.wiki_out_rel();
    let wiki_root = out_dir.join(wiki_rel);
    let clean_url_prefix = config.build.clean_url_prefix();

    // 5. Process and render documents in parallel
    let results: Vec<DocOutcome> = scanned
        .doc_files
        .par_iter()
        .map(|doc_file| {
            let fail = |error: String| {
                DocOutcome::Failed(DocFailure {
                    rel_path: doc_file.rel_path.clone(),
                    error,
                })
            };

            let source = match fs::read_to_string(&doc_file.abs_path) {
                Ok(s) => s,
                Err(e) => return fail(format!("could not read file: {e}")),
            };

            let processed = match process_tomet_document(
                &source,
                &doc_file.rel_path,
                Some(&doc_file.abs_path),
                config,
                &scanned.vault_index,
                scanned.workspace_config_src.as_deref(),
            ) {
                Ok(p) => p,
                Err(e) => return fail(format!("could not process document: {e}")),
            };

            let html = match renderer.render_page(&processed) {
                Ok(h) => h,
                Err(e) => return fail(format!("could not render template: {e}")),
            };

            let out_html_path = wiki_root.join(&processed.slug).join("index.html");
            let written = match write_if_changed(&out_html_path, &html) {
                Ok(w) => w,
                Err(e) => return fail(format!("could not write {}: {e}", out_html_path.display())),
            };

            DocOutcome::Rendered {
                entry: EntrySummary {
                    url: format!("{clean_url_prefix}/{}", processed.slug),
                    title: processed.title,
                    section: processed.section.clone(),
                },
                section: processed.section,
                slug: processed.slug,
                rel_path: doc_file.rel_path.clone(),
                written,
            }
        })
        .collect();

    let mut report = BuildReport::default();
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
                written,
            } => {
                if written {
                    report.written += 1;
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
                report.failures.push(failure);
            }
        }
    }
    report.rendered = processed_entries.len();

    info!(
        "Rendered {} document pages ({} written, {} unchanged)",
        report.rendered,
        report.written,
        report.rendered - report.written
    );

    // 6. Render the catalog page
    let mut sections: Vec<SectionSummary> = section_counts
        .into_iter()
        .map(|(name, count)| SectionSummary { name, count })
        .collect();
    sections.sort_by(|a, b| a.name.cmp(&b.name));

    let wiki_index_html = renderer.render_index(&sections, &processed_entries)?;
    write_if_changed(&wiki_root.join("index.html"), &wiki_index_html)?;

    // 7. Write root /index.html redirecting to the book, unless the book is already there
    if !wiki_rel.is_empty() {
        let redirect_html = format!(
            r#"<!doctype html><html lang="{lang}" data-pagefind-ignore><head><meta charset="utf-8"><title>Redirecting to: {prefix}</title><meta http-equiv="refresh" content="0;url={prefix}"><link rel="canonical" href="{prefix}"></head><body><a href="{prefix}">Redirecting to {prefix}</a></body></html>"#,
            lang = config.book.lang,
            prefix = clean_url_prefix
        );
        write_if_changed(&out_dir.join("index.html"), &redirect_html)?;
    }

    // 8. Drop output left behind by deleted or renamed sources
    if wiki_rel.is_empty() {
        warn!("url_prefix is '/', so stale pages cannot be pruned safely; skipping page cleanup");
    } else {
        let keep_slugs: HashSet<&str> = slug_owner.keys().map(|s| s.as_str()).collect();
        report.pruned_pages = prune_stale_pages(&wiki_root, &keep_slugs);
    }

    let asset_rel = config.build.asset_out_rel();
    if asset_rel.is_empty() {
        warn!(
            "asset_prefix is '/', so stale media cannot be pruned safely; skipping media cleanup"
        );
    } else {
        let keep_media: HashSet<&str> = scanned
            .media_files
            .iter()
            .map(|m| m.rel_path.as_str())
            .collect();
        report.pruned_media = prune_stale_media(&out_dir.join(asset_rel), &keep_media);
    }

    if report.pruned_pages > 0 || report.pruned_media > 0 {
        info!(
            "Pruned {} stale page(s) and {} stale media file(s)",
            report.pruned_pages, report.pruned_media
        );
    }

    // 9. Run Pagefind search indexer if enabled
    if config.build.pagefind {
        let _ = run_pagefind(out_dir);
    }

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
