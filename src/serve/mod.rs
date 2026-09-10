use anyhow::Result;
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
};
use notify::{Event, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

use crate::book::build_book;
use crate::book::loader::{exclude_prefixes, is_ignored_rel};
use crate::config::BookConfig;

/// How long the watcher waits for the filesystem to go quiet before rebuilding.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Upper bound on batching, so a long stream of events still gets serviced.
const MAX_BATCH_WAIT: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ReloadSignal {
    #[serde(rename = "doc")]
    Doc { url: String },
    #[serde(rename = "css")]
    Css,
    #[serde(rename = "full")]
    Full,
}

/// What a batch of filesystem events asks the dev server to do.
#[derive(Debug, Default)]
struct RebuildPlan {
    /// Something changed that can invalidate every page.
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

/// True when an event path cannot affect the book: it is build output, or the
/// vault scanner would have skipped it anyway.
///
/// Paths outside the vault are *not* ignored -- treating an unexpected path as
/// uninteresting is how a watcher goes silent.
fn is_ignored_event_path(path: &Path, src_dir: &Path, out_dir: &Path, excludes: &[String]) -> bool {
    if path.starts_with(out_dir) {
        return true;
    }

    match path.strip_prefix(src_dir) {
        Ok(rel) => is_ignored_rel(&rel.to_string_lossy().replace('\\', "/"), excludes),
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
        // Reads never change the book. inotify emits Access(Close(Write)) right
        // after a save, so counting it as "not an edit" would push every save
        // onto the full-rebuild path.
        if matches!(event.kind, notify::EventKind::Access(_)) {
            continue;
        }

        // An in-place edit only touches its own page. Creating, renaming or
        // deleting a file changes the link index and the catalog too.
        let is_edit_in_place = matches!(event.kind, notify::EventKind::Modify(kind)
            if !matches!(kind, notify::event::ModifyKind::Name(_)));

        for path in &event.paths {
            if is_ignored_event_path(path, src_dir, out_dir, excludes) {
                continue;
            }

            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name == "tmtbook.toml"
                || file_name == "default.config.tmt"
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
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            let rel = path.strip_prefix(src_dir).ok();

            if ext == "tmt" || ext == "tm" {
                if !is_edit_in_place {
                    plan.global = true;
                } else if let Some(rel) = rel.filter(|_| seen_docs.insert(path.clone())) {
                    plan.docs
                        .push((path.clone(), rel.to_string_lossy().replace('\\', "/")));
                }
            } else if crate::book::loader::MEDIA_EXTENSIONS.contains(&ext.as_str()) {
                if let Some(rel) = rel.filter(|_| seen_media.insert(path.clone())) {
                    plan.media
                        .push((path.clone(), rel.to_string_lossy().replace('\\', "/")));
                }
            }
        }
    }

    plan
}

pub async fn run_dev_server(
    src_dir: PathBuf,
    out_dir: PathBuf,
    config: BookConfig,
    host: IpAddr,
    port: u16,
) -> Result<()> {
    // 1. Initial build
    info!("Performing initial build...");
    match build_book(&src_dir, &out_dir, &config) {
        Ok(report) if !report.failures.is_empty() => {
            warn!(
                "Initial build left out {} document(s)",
                report.failures.len()
            );
        }
        Ok(_) => {}
        Err(e) => error!("Initial build failed: {e}"),
    }

    // 2. Broadcast channel for reload events
    let (tx, _rx) = broadcast::channel::<ReloadSignal>(16);
    let reload_tx = Arc::new(tx);

    // 3. Setup file watcher
    let watcher_tx = Arc::clone(&reload_tx);
    let watch_src = src_dir.clone();
    let watch_out = out_dir.clone();
    let watch_cfg = config.clone();

    tokio::task::spawn_blocking(move || {
        let (n_tx, n_rx) = std::sync::mpsc::channel();

        let mut watcher = match notify::recommended_watcher(n_tx) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create file watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_src, RecursiveMode::Recursive) {
            error!(
                "Failed to watch source directory {}: {e}",
                watch_src.display()
            );
            return;
        }

        info!("Watching {} for changes...", watch_src.display());

        let excludes = exclude_prefixes(&watch_cfg);

        // Cache for fast incremental rebuilds
        let mut cached_scanned = crate::book::loader::scan_vault(&watch_src, &watch_cfg).ok();
        let mut cached_renderer = crate::book::renderer::BookRenderer::new(&watch_cfg).ok();

        // Events are collected until the filesystem goes quiet, then handled as
        // one batch. Dropping events instead would silently skip rebuilds when
        // two files are saved in quick succession.
        let mut pending: Vec<Event> = Vec::new();
        let mut batch_started = Instant::now();

        loop {
            let received = if pending.is_empty() {
                match n_rx.recv() {
                    Ok(res) => Some(res),
                    Err(_) => break,
                }
            } else {
                match n_rx.recv_timeout(DEBOUNCE) {
                    Ok(res) => Some(res),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            };

            if let Some(res) = received {
                match res {
                    Ok(event) => {
                        if pending.is_empty() {
                            batch_started = Instant::now();
                        }
                        pending.push(event);
                        if batch_started.elapsed() < MAX_BATCH_WAIT {
                            continue;
                        }
                    }
                    Err(e) => {
                        warn!("Watch error: {e}");
                        continue;
                    }
                }
            }

            let events = std::mem::take(&mut pending);
            let plan = plan_events(&events, &watch_src, &watch_out, &excludes);
            if plan.is_empty() {
                continue;
            }

            let can_render_incrementally = cached_scanned.is_some() && cached_renderer.is_some();
            let needs_full = plan.global || (!plan.docs.is_empty() && !can_render_incrementally);

            if needs_full {
                info!("🔄 Global change detected, rebuilding all pages (parallel)...");
                let start = Instant::now();
                let mut dev_cfg = watch_cfg.clone();
                dev_cfg.build.pagefind = false;

                match build_book(&watch_src, &watch_out, &dev_cfg) {
                    Ok(report) => {
                        info!(
                            "Full rebuild complete in {:?} ({} pages, {} failed), triggering reload",
                            start.elapsed(),
                            report.rendered,
                            report.failures.len()
                        );
                        cached_scanned =
                            crate::book::loader::scan_vault(&watch_src, &watch_cfg).ok();
                        cached_renderer = crate::book::renderer::BookRenderer::new(&watch_cfg).ok();
                        let _ = watcher_tx.send(ReloadSignal::Full);
                    }
                    Err(e) => warn!("Rebuild error: {e}"),
                }
                continue;
            }

            if plan.css {
                let _ =
                    crate::book::assets::write_static_assets(&watch_out, &watch_src, &watch_cfg);
                info!("🎨 CSS changed, hot reloading stylesheets");
                let _ = watcher_tx.send(ReloadSignal::Css);
            }

            for (abs_path, rel_path) in &plan.docs {
                let start = Instant::now();
                let scanned = cached_scanned.as_ref().unwrap();
                let renderer = cached_renderer.as_ref().unwrap();

                match crate::book::render_single_document(
                    abs_path,
                    rel_path,
                    &watch_out,
                    &watch_cfg,
                    &scanned.vault_index,
                    scanned.workspace_config_src.as_deref(),
                    renderer,
                ) {
                    Ok((written, doc_url)) => {
                        info!(
                            "⚡ Incremental rebuild: {} in {:?} (written: {}, url: {})",
                            rel_path,
                            start.elapsed(),
                            written,
                            doc_url
                        );
                        if written {
                            let _ = watcher_tx.send(ReloadSignal::Doc { url: doc_url });
                        }
                    }
                    Err(e) => warn!("Incremental rebuild error for {rel_path}: {e}"),
                }
            }

            if !plan.media.is_empty() {
                for (abs_path, rel_path) in &plan.media {
                    let _ = crate::book::assets::sync_single_media(
                        &watch_out, abs_path, rel_path, &watch_cfg,
                    );
                    info!("🖼️ Synced media: {rel_path}");
                }
                let _ = watcher_tx.send(ReloadSignal::Full);
            }
        }
    });

    // 4. Axum server
    let ws_tx = Arc::clone(&reload_tx);
    let app = Router::new()
        .route(
            "/live-reload",
            get(move |ws: WebSocketUpgrade| {
                let tx = Arc::clone(&ws_tx);
                async move { ws.on_upgrade(|socket| handle_ws(socket, tx)) }
            }),
        )
        .fallback_service(ServeDir::new(&out_dir));

    let addr = SocketAddr::from((host, port));
    let display_host = if host.is_unspecified() {
        "localhost".to_string()
    } else {
        host.to_string()
    };

    info!("📖 tmtbook dev server running at http://{display_host}:{port}");
    info!(
        "Open http://{display_host}:{port}{} in your browser",
        config.build.clean_url_prefix()
    );
    if !host.is_loopback() {
        warn!("Bound to {host} -- this vault is reachable from your network");
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn handle_ws(mut socket: WebSocket, tx: Arc<broadcast::Sender<ReloadSignal>>) {
    let mut rx = tx.subscribe();
    while let Ok(signal) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&signal) {
            if socket.send(Message::Text(json_str)).await.is_err() {
                break;
            }
        }
    }
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

    /// `plan` against the default `/vault` layout.
    pub fn plan_batch(events: &[Event]) -> RebuildPlan {
        plan(events, "/vault", "/vault/dist")
    }

    #[test]
    fn every_document_in_a_batch_is_rebuilt() {
        // Two saves inside one debounce window used to drop the second file.
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
        // A substring check on the absolute path used to silence the whole server.
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
        // New and removed files change the link index and the catalog.
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
        // inotify reports a save as Modify(Data) followed by Access(Close(Write));
        // the trailing read must not drag the batch into a full rebuild.
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
