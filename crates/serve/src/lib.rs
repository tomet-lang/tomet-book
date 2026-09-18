use anyhow::Result;
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
};
use notify::{Event, RecursiveMode, Watcher};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

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

/// Handler trait for processing filesystem events and triggering rebuilds.
/// Implementations encapsulate project-specific rebuild logic (e.g. Tomet book rendering).
pub trait DevServerHandler: Send + 'static {
    /// Perform initial build before server starts serving requests.
    fn on_init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Process a batch of filesystem events, rebuild required outputs,
    /// and return zero or more reload signals to send to connected browsers.
    fn on_events(&mut self, events: &[Event]) -> Vec<ReloadSignal>;
}

/// Run a generic development server with live reload.
pub async fn run_dev_server<H>(
    watch_dir: PathBuf,
    serve_dir: PathBuf,
    host: IpAddr,
    port: u16,
    url_prefix: Option<String>,
    mut handler: H,
) -> Result<()>
where
    H: DevServerHandler,
{
    // 1. Initial build
    info!("Performing initial build...");
    if let Err(e) = handler.on_init() {
        error!("Initial build failed: {e}");
    }

    // 2. Broadcast channel for reload events
    let (tx, _rx) = broadcast::channel::<ReloadSignal>(16);
    let reload_tx = Arc::new(tx);

    // 3. Setup file watcher in a blocking thread
    let watcher_tx = Arc::clone(&reload_tx);
    let watch_target = watch_dir.clone();

    tokio::task::spawn_blocking(move || {
        let (n_tx, n_rx) = std::sync::mpsc::channel();

        let mut watcher = match notify::recommended_watcher(n_tx) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to create file watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_target, RecursiveMode::Recursive) {
            error!("Failed to watch directory {}: {e}", watch_target.display());
            return;
        }

        info!("Watching {} for changes...", watch_target.display());

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
            if events.is_empty() {
                continue;
            }

            let signals = handler.on_events(&events);
            for signal in signals {
                let _ = watcher_tx.send(signal);
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
        .fallback_service(ServeDir::new(&serve_dir));

    let addr = SocketAddr::from((host, port));
    let display_host = if host.is_unspecified() {
        "localhost".to_string()
    } else {
        host.to_string()
    };

    info!("📖 Dev server running at http://{display_host}:{port}");
    if let Some(prefix) = url_prefix {
        info!("Open http://{display_host}:{port}{prefix} in your browser");
    }
    if !host.is_loopback() {
        warn!("Bound to {host} -- this server is reachable from your network");
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn handle_ws(mut socket: WebSocket, tx: Arc<broadcast::Sender<ReloadSignal>>) {
    let mut rx = tx.subscribe();
    while let Ok(signal) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&signal)
            && socket.send(Message::Text(json_str)).await.is_err()
        {
            break;
        }
    }
}
