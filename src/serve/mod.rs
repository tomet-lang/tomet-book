use anyhow::Result;
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
};
use notify::{Event, RecursiveMode, Watcher};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

use crate::book::build_book;
use crate::config::BookConfig;

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

pub async fn run_dev_server(
    src_dir: PathBuf,
    out_dir: PathBuf,
    config: BookConfig,
    port: u16,
) -> Result<()> {
    // 1. Initial build
    info!("Performing initial build...");
    if let Err(e) = build_book(&src_dir, &out_dir, &config) {
        error!("Initial build failed: {e}");
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
        let mut last_build = Instant::now();
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

        // Cache for fast incremental rebuilds
        let mut cached_scanned = crate::book::loader::scan_vault(&watch_src, &watch_cfg).ok();
        let mut cached_renderer = crate::book::renderer::BookRenderer::new(&watch_cfg).ok();

        while let Ok(res) = n_rx.recv() {
            match res {
                Ok(event) => {
                    if should_ignore_event(&event, &watch_out) {
                        continue;
                    }

                    // Debounce rapid events within 150ms
                    if last_build.elapsed() < Duration::from_millis(150) {
                        continue;
                    }
                    last_build = Instant::now();

                    // Analyze event paths
                    let mut is_global = false;
                    let mut is_css_only = false;
                    let mut single_doc: Option<(PathBuf, String)> = None;
                    let mut single_media: Option<(PathBuf, String)> = None;

                    let is_modify_only = matches!(event.kind, notify::EventKind::Modify(_));

                    for path in &event.paths {
                        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        if file_name == "tmtbook.toml"
                            || file_name == "default.config.tmt"
                            || file_name.ends_with(".js")
                            || file_name.ends_with(".html")
                        {
                            is_global = true;
                            break;
                        }

                        if file_name.ends_with(".css") {
                            is_css_only = true;
                            continue;
                        }

                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                        if ext == "tmt" || ext == "tm" {
                            if is_modify_only && single_doc.is_none() && !is_global {
                                if let Ok(rel) = path.strip_prefix(&watch_src) {
                                    single_doc = Some((path.clone(), rel.to_string_lossy().replace('\\', "/")));
                                }
                            } else {
                                is_global = true;
                            }
                        } else if crate::book::loader::MEDIA_EXTENSIONS.contains(&ext.as_str()) {
                            if let Ok(rel) = path.strip_prefix(&watch_src) {
                                single_media = Some((path.clone(), rel.to_string_lossy().replace('\\', "/")));
                            }
                        }
                    }

                    if !is_global && is_css_only && single_doc.is_none() {
                        let _ = crate::book::assets::write_static_assets(&watch_out, &watch_src, &watch_cfg);
                        info!("🎨 CSS changed, hot reloading stylesheets");
                        let _ = watcher_tx.send(ReloadSignal::Css);
                    } else if !is_global && single_doc.is_some() && cached_scanned.is_some() && cached_renderer.is_some() {
                        let (abs_path, rel_path) = single_doc.unwrap();
                        let start = Instant::now();
                        let scanned = cached_scanned.as_ref().unwrap();
                        let renderer = cached_renderer.as_ref().unwrap();

                        match crate::book::render_single_document(
                            &abs_path,
                            &rel_path,
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
                            Err(e) => {
                                warn!("Incremental rebuild error: {e}");
                            }
                        }
                    } else if !is_global && single_media.is_some() {
                        let (abs_path, rel_path) = single_media.unwrap();
                        let _ = crate::book::assets::sync_single_media(&watch_out, &abs_path, &rel_path);
                        info!("🖼️ Synced media: {}", rel_path);
                        let _ = watcher_tx.send(ReloadSignal::Full);
                    } else {
                        info!("🔄 Global change detected, rebuilding all pages (parallel)...");
                        let start = Instant::now();
                        let mut dev_cfg = watch_cfg.clone();
                        dev_cfg.build.pagefind = false;

                        match build_book(&watch_src, &watch_out, &dev_cfg) {
                            Ok(()) => {
                                info!("Full rebuild complete in {:?}, triggering reload", start.elapsed());
                                cached_scanned = crate::book::loader::scan_vault(&watch_src, &watch_cfg).ok();
                                cached_renderer = crate::book::renderer::BookRenderer::new(&watch_cfg).ok();
                                let _ = watcher_tx.send(ReloadSignal::Full);
                            }
                            Err(e) => {
                                warn!("Rebuild error: {e}");
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Watch error: {e}");
                }
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
        .fallback_service(ServeDir::new(&out_dir))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("📖 tmtbook dev server running at http://localhost:{}", port);
    info!("Open http://localhost:{}/wiki in your browser", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

fn should_ignore_event(event: &Event, out_dir: &Path) -> bool {
    for path in &event.paths {
        if path.starts_with(out_dir) {
            return true;
        }
        let s = path.to_string_lossy();
        if s.contains(".git") || s.contains("dist") || s.contains("target") || s.contains(".web") {
            return true;
        }
    }
    false
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
