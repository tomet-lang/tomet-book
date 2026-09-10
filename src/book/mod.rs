pub mod assets;
pub mod document;
pub mod loader;
pub mod pagefind;
pub mod renderer;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

use crate::config::BookConfig;
use assets::{copy_vault_media, write_static_assets};
use document::process_tomet_document;
use loader::scan_vault;
use pagefind::run_pagefind;
use renderer::{BookRenderer, EntrySummary, SectionSummary};

use rayon::prelude::*;

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
) -> Result<bool> {
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
        .join("wiki")
        .join(&processed.slug)
        .join("index.html");
    write_if_changed(&out_html_path, &html)
}

pub fn build_book(src_dir: &Path, out_dir: &Path, config: &BookConfig) -> Result<()> {
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

    // 3. Copy/Link media to dist/vault/
    let copied_media =
        copy_vault_media(out_dir, &scanned.media_files).context("Failed to copy vault media")?;
    info!("Linked/copied {} media assets", copied_media);

    // 4. Initialize MiniJinja renderer
    let renderer = BookRenderer::new(config)?;

    // 5. Process and render documents in parallel
    let results: Vec<(EntrySummary, Option<String>, bool)> = scanned
        .doc_files
        .par_iter()
        .filter_map(|doc_file| {
            let source = match fs::read_to_string(&doc_file.abs_path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", doc_file.abs_path.display(), e);
                    return None;
                }
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
                Err(e) => {
                    tracing::warn!("Error processing {}: {}", doc_file.rel_path, e);
                    return None;
                }
            };

            let html = match renderer.render_page(&processed) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!("Error rendering template for {}: {}", doc_file.rel_path, e);
                    return None;
                }
            };

            let out_html_path = out_dir
                .join("wiki")
                .join(&processed.slug)
                .join("index.html");

            let written = match write_if_changed(&out_html_path, &html) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("Failed to write {}: {}", out_html_path.display(), e);
                    return None;
                }
            };

            Some((
                EntrySummary {
                    url: format!("{}/{}", config.build.url_prefix, processed.slug),
                    title: processed.title,
                    section: processed.section.clone(),
                },
                processed.section,
                written,
            ))
        })
        .collect();

    let mut written_count = 0;
    let mut processed_entries = Vec::with_capacity(results.len());
    let mut section_counts: HashMap<String, usize> = HashMap::new();

    for (entry, section, written) in results {
        if written {
            written_count += 1;
        }
        if let Some(sec) = section {
            *section_counts.entry(sec).or_insert(0) += 1;
        }
        processed_entries.push(entry);
    }

    info!(
        "Rendered {} document pages ({} written, {} unchanged)",
        processed_entries.len(),
        written_count,
        processed_entries.len() - written_count
    );

    // 6. Render /wiki/index.html (Catalog page)
    let mut sections: Vec<SectionSummary> = section_counts
        .into_iter()
        .map(|(name, count)| SectionSummary { name, count })
        .collect();
    sections.sort_by(|a, b| a.name.cmp(&b.name));

    let wiki_index_html = renderer.render_index(&sections, &processed_entries)?;
    let wiki_index_path = out_dir.join("wiki").join("index.html");
    write_if_changed(&wiki_index_path, &wiki_index_html)?;

    // 7. Write root /index.html with redirect to /wiki
    let redirect_html = format!(
        r#"<!doctype html><html lang="{lang}" data-pagefind-ignore><head><meta charset="utf-8"><title>Redirecting to: {prefix}</title><meta http-equiv="refresh" content="0;url={prefix}"><link rel="canonical" href="{prefix}"></head><body><a href="{prefix}">Redirecting to {prefix}</a></body></html>"#,
        lang = config.book.lang,
        prefix = config.build.url_prefix
    );
    write_if_changed(&out_dir.join("index.html"), &redirect_html)?;

    // 8. Run Pagefind search indexer if enabled
    if config.build.pagefind {
        let _ = run_pagefind(out_dir);
    }

    info!("Build completed successfully: {}", out_dir.display());
    Ok(())
}
