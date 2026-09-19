use anyhow::Result;
use notify::Event;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{info, warn};

use crate::book::build_book;
use crate::book::loader::{exclude_prefixes, is_ignored_rel};
use crate::config::BookConfig;
use tmtbook_serve::{DevServerHandler, ReloadSignal};

pub struct TometDevHandler {
    src_dir: PathBuf,
    out_dir: PathBuf,
    config: BookConfig,
    excludes: Vec<String>,
    scanned: Option<crate::book::loader::ScannedVault>,
    backlinks: HashMap<String, Vec<crate::book::renderer::Backlink>>,
    unpublished: HashSet<String>,
    book_index: Vec<crate::book::renderer::EntrySummary>,
    doc_icons: HashMap<String, String>,
}

impl TometDevHandler {
    pub fn new(src_dir: PathBuf, out_dir: PathBuf, config: BookConfig) -> Self {
        let excludes = exclude_prefixes(&config);
        Self {
            src_dir,
            out_dir,
            config,
            excludes,
            scanned: None,
            backlinks: HashMap::new(),
            unpublished: HashSet::new(),
            book_index: Vec::new(),
            doc_icons: HashMap::new(),
        }
    }

    fn full_rebuild(&mut self) -> bool {
        let start = Instant::now();
        let mut dev_cfg = self.config.clone();
        dev_cfg.build.pagefind = false;

        match build_book(&self.src_dir, &self.out_dir, &dev_cfg, true) {
            Ok(report) => {
                info!(
                    "Full rebuild complete in {:?} ({} pages, {} written, {} pruned, {} failed), triggering reload",
                    start.elapsed(),
                    report.rendered,
                    report.written,
                    report.pruned_pages + report.pruned_media,
                    report.failures.len()
                );
                self.backlinks = report.backlinks;
                self.unpublished = report.unpublished;
                self.book_index = report.book_index;
                self.scanned = Some(report.scanned);
                self.doc_icons = report.doc_icons;
                true
            }
            Err(e) => {
                warn!("Rebuild error: {e}");
                false
            }
        }
    }
}

impl DevServerHandler for TometDevHandler {
    fn on_init(&mut self) -> Result<()> {
        match build_book(&self.src_dir, &self.out_dir, &self.config, true) {
            Ok(report) => {
                if !report.failures.is_empty() {
                    warn!(
                        "Initial build left out {} document(s)",
                        report.failures.len()
                    );
                }
                self.backlinks = report.backlinks;
                self.unpublished = report.unpublished;
                self.book_index = report.book_index;
                self.scanned = Some(report.scanned);
                self.doc_icons = report.doc_icons;
            }
            Err(e) => warn!("Initial build failed: {e}"),
        }

        if self.scanned.is_none() {
            self.scanned = crate::book::loader::scan_vault(&self.src_dir, &self.config).ok();
        }

        Ok(())
    }

    fn on_events(&mut self, events: &[Event]) -> Vec<ReloadSignal> {
        let mut signals = Vec::new();
        let plan = plan_events(events, &self.src_dir, &self.out_dir, &self.excludes);
        if plan.is_empty() {
            return signals;
        }

        let can_render_incrementally = self.scanned.is_some();
        let needs_full = plan.global || (!plan.docs.is_empty() && !can_render_incrementally);

        if needs_full {
            info!("🔄 Global change detected, rebuilding all pages (parallel)...");
            if self.full_rebuild() {
                signals.push(ReloadSignal::Full);
            }
            return signals;
        }

        if plan.css {
            let _ = crate::book::assets::write_static_assets(
                &self.out_dir,
                &self.src_dir,
                &self.config,
            );
            info!("🎨 CSS changed, hot reloading stylesheets");
            signals.push(ReloadSignal::Css);
        }

        let renderer = match crate::book::renderer::BookRenderer::new(&self.config, true) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to create renderer: {e}");
                if self.full_rebuild() {
                    signals.push(ReloadSignal::Full);
                }
                return signals;
            }
        };

        for (abs_path, rel_path) in &plan.docs {
            let start = Instant::now();
            let scanned = self.scanned.as_ref().unwrap();

            match crate::book::render_single_document(
                abs_path,
                rel_path,
                &self.out_dir,
                &crate::book::RenderContext {
                    config: &self.config,
                    src_dir: &self.src_dir,
                    vault_index: &scanned.vault_index,
                    workspace_cfg_src: scanned.workspace_config_src.as_deref(),
                    workspace_cfg_blocks: &scanned.workspace_config_blocks,
                    renderer: &renderer,
                    unpublished: &self.unpublished,
                    book_index: &self.book_index,
                    doc_icons: &self.doc_icons,
                },
                self.backlinks
                    .get(crate::book::document::strip_doc_extension(rel_path))
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ) {
                Ok(Some((written, doc_url))) => {
                    info!(
                        "⚡ Incremental rebuild: {} in {:?} (written: {}, url: {})",
                        rel_path,
                        start.elapsed(),
                        written,
                        doc_url
                    );
                    if written {
                        signals.push(ReloadSignal::Doc { url: doc_url });
                    }
                }
                Ok(None) => {
                    info!("{rel_path} is not published; rebuilding in full");
                    if self.full_rebuild() {
                        signals.push(ReloadSignal::Full);
                    }
                    break;
                }
                Err(e) => warn!("Incremental rebuild error for {rel_path}: {e}"),
            }
        }

        if !plan.media.is_empty() {
            for (abs_path, rel_path) in &plan.media {
                let _ = crate::book::assets::sync_single_media(
                    &self.out_dir,
                    abs_path,
                    rel_path,
                    &self.config,
                );
                info!("🖼️ Synced media: {rel_path}");
            }
            signals.push(ReloadSignal::Full);
        }

        signals
    }
}

/// What a batch of filesystem events asks the dev server to do.
#[derive(Debug, Default)]
struct RebuildPlan {
    global: bool,
    css: bool,
    docs: Vec<(PathBuf, String)>,
    media: Vec<(PathBuf, String)>,
}

impl RebuildPlan {
    fn is_empty(&self) -> bool {
        !self.global && !self.css && self.docs.is_empty() && self.media.is_empty()
    }
}

fn is_ignored_event_path(path: &Path, src_dir: &Path, out_dir: &Path, excludes: &[String]) -> bool {
    if path.starts_with(out_dir) {
        return true;
    }

    match path.strip_prefix(src_dir) {
        Ok(rel) => is_ignored_rel(&crate::book::normalize_path(rel), excludes),
        Err(_) => false,
    }
}

fn plan_events(
    events: &[Event],
    src_dir: &Path,
    out_dir: &Path,
    excludes: &[String],
) -> RebuildPlan {
    let mut plan = RebuildPlan::default();
    let mut seen_docs: HashSet<PathBuf> = HashSet::new();
    let mut seen_media: HashSet<PathBuf> = HashSet::new();

    for event in events {
        if matches!(event.kind, notify::EventKind::Access(_)) {
            continue;
        }

        let is_edit_in_place = matches!(event.kind, notify::EventKind::Modify(kind)
            if !matches!(kind, notify::event::ModifyKind::Name(_)));

        for path in &event.paths {
            if is_ignored_event_path(path, src_dir, out_dir, excludes) {
                continue;
            }

            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name == "tmtbook.toml"
                || crate::book::catalog::is_control_document(file_name)
                || file_name.ends_with(".js")
                || file_name.ends_with(".html")
            {
                plan.global = true;
                continue;
            }

            if file_name.ends_with(".css") {
                plan.css = true;
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let rel = path.strip_prefix(src_dir).ok();

            if ext == "tmt" || ext == "tm" {
                if !is_edit_in_place {
                    plan.global = true;
                } else if let Some(rel) = rel.filter(|_| seen_docs.insert(path.to_path_buf())) {
                    plan.docs
                        .push((path.to_path_buf(), crate::book::normalize_path(rel)));
                }
            } else if crate::book::loader::MEDIA_EXTENSIONS.contains(&ext.as_str())
                && let Some(rel) = rel.filter(|_| seen_media.insert(path.to_path_buf()))
            {
                plan.media
                    .push((path.to_path_buf(), crate::book::normalize_path(rel)));
            }
        }
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, EventKind, ModifyKind, RemoveKind};

    pub fn event(kind: EventKind, paths: &[&str]) -> Event {
        let mut ev = Event::new(kind);
        for p in paths {
            ev = ev.add_path(PathBuf::from(p));
        }
        ev
    }

    fn modified(paths: &[&str]) -> Event {
        event(EventKind::Modify(ModifyKind::Any), paths)
    }

    pub fn plan(events: &[Event], src: &str, out: &str) -> RebuildPlan {
        plan_events(
            events,
            Path::new(src),
            Path::new(out),
            &["dist".to_string()],
        )
    }

    pub fn plan_batch(events: &[Event]) -> RebuildPlan {
        plan(events, "/vault", "/vault/dist")
    }

    #[test]
    fn every_document_in_a_batch_is_rebuilt() {
        let plan = plan(
            &[
                modified(&["/vault/a.tmt"]),
                modified(&["/vault/notes/b.tmt"]),
            ],
            "/vault",
            "/vault/dist",
        );

        assert!(!plan.global);
        let rels: Vec<&str> = plan.docs.iter().map(|(_, rel)| rel.as_str()).collect();
        assert_eq!(rels, vec!["a.tmt", "notes/b.tmt"]);
    }

    #[test]
    fn the_same_document_is_only_rebuilt_once() {
        let plan = plan(
            &[modified(&["/vault/a.tmt"]), modified(&["/vault/a.tmt"])],
            "/vault",
            "/vault/dist",
        );
        assert_eq!(plan.docs.len(), 1);
    }

    #[test]
    fn a_vault_path_containing_dist_is_still_watched() {
        let plan = plan(
            &[modified(&["/home/u/distrib/vault/a.tmt"])],
            "/home/u/distrib/vault",
            "/home/u/distrib/vault/dist",
        );
        assert_eq!(plan.docs.len(), 1, "event was wrongly ignored");
    }

    #[test]
    fn build_output_is_ignored() {
        let plan = plan(
            &[modified(&["/vault/dist/wiki/a/index.html"])],
            "/vault",
            "/vault/dist",
        );
        assert!(plan.is_empty());
    }

    #[test]
    fn hidden_directories_are_ignored() {
        let plan = plan(&[modified(&["/vault/.git/index"])], "/vault", "/vault/dist");
        assert!(plan.is_empty());
    }

    #[test]
    fn creating_or_deleting_a_document_forces_a_full_rebuild() {
        let created = plan(
            &[event(
                EventKind::Create(CreateKind::File),
                &["/vault/new.tmt"],
            )],
            "/vault",
            "/vault/dist",
        );
        assert!(created.global);

        let removed = plan(
            &[event(
                EventKind::Remove(RemoveKind::File),
                &["/vault/old.tmt"],
            )],
            "/vault",
            "/vault/dist",
        );
        assert!(removed.global);
    }

    #[test]
    fn editing_a_written_index_forces_a_full_rebuild() {
        let plan = plan(
            &[modified(&["/vault/book.index.tmt"])],
            "/vault",
            "/vault/dist",
        );
        assert!(plan.global);
        assert!(plan.docs.is_empty(), "it must not be rendered as a page");
    }

    #[test]
    fn config_changes_force_a_full_rebuild() {
        let plan = plan(
            &[modified(&["/vault/tmtbook.toml"])],
            "/vault",
            "/vault/dist",
        );
        assert!(plan.global);
    }

    #[test]
    fn css_and_media_are_classified_separately() {
        let plan = plan(
            &[modified(&["/vault/custom.css", "/vault/img/a.png"])],
            "/vault",
            "/vault/dist",
        );
        assert!(plan.css);
        assert!(!plan.global);
        assert_eq!(plan.media.len(), 1);
        assert_eq!(plan.media[0].1, "img/a.png");
    }
}

#[cfg(test)]
mod event_kind_tests {
    use super::tests::*;
    use notify::event::{AccessKind, AccessMode, EventKind, ModifyKind, RenameMode};

    #[test]
    fn a_save_stays_on_the_incremental_path() {
        let plan = plan_batch(&[
            event(
                EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Any)),
                &["/vault/a.tmt"],
            ),
            event(
                EventKind::Access(AccessKind::Close(AccessMode::Write)),
                &["/vault/a.tmt"],
            ),
        ]);

        assert!(!plan.global, "a plain save should not rebuild everything");
        assert_eq!(plan.docs.len(), 1);
    }

    #[test]
    fn reads_alone_do_nothing() {
        let plan = plan_batch(&[event(
            EventKind::Access(AccessKind::Open(AccessMode::Read)),
            &["/vault/a.tmt"],
        )]);
        assert!(plan.is_empty());
    }

    #[test]
    fn renaming_forces_a_full_rebuild() {
        let plan = plan_batch(&[event(
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            &["/vault/a.tmt"],
        )]);
        assert!(plan.global);
    }
}
