//! In-memory vault state and per-page rendering contexts for dev SSR and SSG.

use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::document::{self, ProcessedDoc};
use super::image_opt;
use super::kinds;
use super::loader::{ScannedVault, scan_vault};
use super::renderer::{Backlink, BookRenderer, EntrySummary, SectionSummary};
use super::slug;
use super::{
    DocFailure, arrange_entries, build_backlink_index, read_written_index, write_if_changed,
};
use crate::config::BookConfig;

/// The vault-wide state a single page needs in order to render.
///
/// The dev server holds all of this between edits; bundling it keeps the
/// per-page call about the page.
pub struct RenderContext<'a> {
    pub config: &'a BookConfig,
    pub src_dir: &'a Path,
    pub vault_index: &'a tomet_links::VaultLinkIndex,
    pub workspace_cfg_src: Option<&'a str>,
    pub workspace_cfg_blocks: &'a [tomet_ast::Block],
    pub renderer: &'a BookRenderer,
    /// The documents the last full build left out, so a page re-rendered on
    /// its own still shows links to them as broken.
    pub unpublished: &'a HashSet<String>,
    /// The book's table of contents, as the last full build arranged it.
    pub book_index: &'a [EntrySummary],
    /// Note icon HTML snippets keyed by slug, for internal link decoration.
    pub doc_icons: &'a HashMap<String, String>,
    /// The vault-wide route table.
    pub route_table: &'a slug::RouteTable,
}

/// In-memory representation of the scanned and indexed vault state.
/// Holds metadata, routes, backlinks, catalog, and renderer for on-demand SSR.
pub struct VaultState {
    pub config: BookConfig,
    pub src_dir: PathBuf,
    pub scanned: ScannedVault,
    pub route_table: slug::RouteTable,
    pub backlinks: HashMap<String, Vec<Backlink>>,
    pub unpublished: HashSet<String>,
    pub book_index: Vec<EntrySummary>,
    pub doc_icons: HashMap<String, String>,
    pub renderer: BookRenderer,
    pub sections: Vec<SectionSummary>,
    pub catalog_entries: Vec<EntrySummary>,
}

impl VaultState {
    pub fn init(src_dir: &Path, config: &BookConfig, is_dev: bool) -> Result<Self> {
        let scanned = scan_vault(src_dir, config).context("Failed to scan vault")?;

        let parsed: Vec<Result<(tomet_ast::Document, Option<String>), DocFailure>> = scanned
            .doc_files
            .par_iter()
            .map(|doc_file| {
                let fail = |error: String| DocFailure {
                    rel_path: doc_file.rel_path.clone(),
                    error,
                };

                let source = match fs::read_to_string(&doc_file.abs_path) {
                    Ok(s) => s,
                    Err(e) => return Err(fail(format!("could not read file: {e}"))),
                };

                document::parse_tomet_document(&source, &doc_file.rel_path)
                    .map_err(|e| fail(format!("could not parse document: {e}")))
            })
            .collect();

        let unpublished: HashSet<String> = parsed
            .iter()
            .zip(scanned.doc_files.iter())
            .filter(|(result, _)| match result {
                Ok((_, kind)) => !kinds::publishes(kind.as_deref(), &config.build.kinds),
                Err(_) => false,
            })
            .map(|(_, doc_file)| doc_file.rel_path.clone())
            .collect();

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

        let processed: Vec<Result<ProcessedDoc, DocFailure>> = parsed
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
            note_icons
        } else {
            HashMap::new()
        };

        let clean_url_prefix = config.build.clean_url_prefix();
        let docs: Vec<&ProcessedDoc> = processed.iter().filter_map(|r| r.as_ref().ok()).collect();
        let backlinks = build_backlink_index(&docs, clean_url_prefix);

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

        let mut section_counts: HashMap<String, usize> = HashMap::new();
        let mut processed_entries = Vec::with_capacity(docs.len());
        for doc in &docs {
            if let Some(sec) = &doc.section {
                *section_counts.entry(sec.clone()).or_insert(0) += 1;
            }
            processed_entries.push(EntrySummary {
                url: format!("{clean_url_prefix}/{}", doc.slug),
                title: doc.title.clone(),
                section: doc.section.clone(),
                children: Vec::new(),
            });
        }

        let mut sections: Vec<SectionSummary> = section_counts
            .into_iter()
            .map(|(name, count)| SectionSummary { name, count })
            .collect();
        sections.sort_by(|a, b| a.name.cmp(&b.name));

        let catalog_entries = match written_index {
            Some(_) => book_index.clone(),
            None => processed_entries,
        };

        let renderer = BookRenderer::new(config, is_dev)?;

        Ok(Self {
            config: config.clone(),
            src_dir: src_dir.to_path_buf(),
            scanned,
            route_table,
            backlinks,
            unpublished,
            book_index,
            doc_icons,
            renderer,
            sections,
            catalog_entries,
        })
    }

    pub fn render_context<'a>(&'a self) -> RenderContext<'a> {
        RenderContext {
            config: &self.config,
            src_dir: &self.src_dir,
            vault_index: &self.scanned.vault_index,
            workspace_cfg_src: self.scanned.workspace_config_src.as_deref(),
            workspace_cfg_blocks: &self.scanned.workspace_config_blocks,
            renderer: &self.renderer,
            unpublished: &self.unpublished,
            book_index: &self.book_index,
            doc_icons: &self.doc_icons,
            route_table: &self.route_table,
        }
    }

    pub fn render_page_by_slug(&self, slug: &str) -> Result<Option<String>> {
        let clean_slug = slug.trim_matches('/');
        let rel_path = match self.route_table.rel_path_for_slug(clean_slug) {
            Some(p) => p,
            None => return Ok(None),
        };

        let abs_path = self.src_dir.join(rel_path);
        if !abs_path.is_file() {
            return Ok(None);
        }

        let cx = self.render_context();
        let incoming: &[Backlink] = self
            .backlinks
            .get(clean_slug)
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        let res = render_single_document_html(&abs_path, rel_path, &cx, incoming)?;
        Ok(res.map(|(html, _)| html))
    }

    pub fn render_index_page(&self) -> Result<String> {
        self.renderer
            .render_index(&self.sections, &self.catalog_entries)
    }
}

pub fn render_single_document_html(
    abs_path: &Path,
    rel_path: &str,
    cx: &RenderContext<'_>,
    backlinks: &[Backlink],
) -> Result<Option<(String, String)>> {
    let config = cx.config;
    let source = fs::read_to_string(abs_path)?;
    let (doc, kind) = document::parse_tomet_document(&source, rel_path)?;

    // Its `@kind` may be what changed. Nothing to write either way.
    if !kinds::publishes(kind.as_deref(), &config.build.kinds) {
        return Ok(None);
    }

    let mut processed = document::process_parsed_document_with_blocks(
        doc,
        rel_path,
        Some(abs_path),
        config,
        cx.vault_index,
        cx.workspace_cfg_blocks,
        cx.unpublished,
        Some(cx.route_table),
    )?;
    if config.ui.links.note_icons && !cx.doc_icons.is_empty() {
        processed.body_html = document::enhance_internal_links(
            &processed.body_html,
            cx.doc_icons,
            config.build.clean_url_prefix(),
        );
    }
    let html = cx
        .renderer
        .render_page(&processed, backlinks, cx.book_index)?;
    let url = format!("{}/{}", config.build.clean_url_prefix(), processed.slug);
    Ok(Some((html, url)))
}

pub fn render_single_document(
    abs_path: &Path,
    rel_path: &str,
    out_dir: &Path,
    cx: &RenderContext<'_>,
    backlinks: &[Backlink],
) -> Result<Option<(bool, String)>> {
    let config = cx.config;
    let source = fs::read_to_string(abs_path)?;
    let (doc, kind) = document::parse_tomet_document(&source, rel_path)?;

    // Its `@kind` may be what changed. Nothing to write either way.
    if !kinds::publishes(kind.as_deref(), &config.build.kinds) {
        return Ok(None);
    }

    let mut processed = document::process_parsed_document_with_blocks(
        doc,
        rel_path,
        Some(abs_path),
        config,
        cx.vault_index,
        cx.workspace_cfg_blocks,
        cx.unpublished,
        Some(cx.route_table),
    )?;
    image_opt::optimize_single_doc_media(&mut processed, cx.src_dir, out_dir, config);
    if config.ui.links.note_icons && !cx.doc_icons.is_empty() {
        processed.body_html = document::enhance_internal_links(
            &processed.body_html,
            cx.doc_icons,
            config.build.clean_url_prefix(),
        );
    }
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
