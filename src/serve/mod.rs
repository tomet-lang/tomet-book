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
    let (tx, _rx) = broadcast::channel::<()>(16);
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
            error!("Failed to watch source directory {}: {e}", watch_src.display());
            return;
        }

        info!("Watching {} for changes...", watch_src.display());

        while let Ok(res) = n_rx.recv() {
            match res {
                Ok(event) => {
                    if should_ignore_event(&event, &watch_out) {
                        continue;
                    }

                    // Debounce rapid events within 300ms
                    if last_build.elapsed() < Duration::from_millis(300) {
                        continue;
                    }
                    last_build = Instant::now();

                    info!("Change detected, rebuilding...");
                    match build_book(&watch_src, &watch_out, &watch_cfg) {
                        Ok(()) => {
                            info!("Rebuild complete, triggering reload");
                            let _ = watcher_tx.send(());
                        }
                        Err(e) => {
                            warn!("Rebuild error: {e}");
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

async fn handle_ws(mut socket: WebSocket, tx: Arc<broadcast::Sender<()>>) {
    let mut rx = tx.subscribe();
    while let Ok(()) = rx.recv().await {
        if socket.send(Message::Text("reload".to_string())).await.is_err() {
            break;
        }
    }
}
