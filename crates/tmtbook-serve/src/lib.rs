pub use axum;

use anyhow::Result;
use axum::{
    Router,
    extract::{
        Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    routing::get,
};
use notify::{Event, RecursiveMode, Watcher};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
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
pub trait DevServerHandler: Send + Sync + 'static {
    /// Perform initial setup/build before server starts serving requests.
    fn on_init(&self) -> Result<()> {
        Ok(())
    }

    /// Process a batch of filesystem events, rebuild required outputs,
    /// and return zero or more reload signals to send to connected browsers.
    fn on_events(&self, events: &[Event]) -> Vec<ReloadSignal>;

    /// Extend or customize the Axum router with project-specific dynamic or static routes.
    fn extend_router(&self, router: Router) -> Router {
        router
    }
}

/// Run a generic development server with live reload.
pub async fn run_dev_server<H>(
    watch_dir: PathBuf,
    serve_dir: Option<PathBuf>,
    host: IpAddr,
    port: u16,
    url_prefix: Option<String>,
    handler: H,
) -> Result<()>
where
    H: DevServerHandler,
{
    let handler = Arc::new(handler);

    // 1. Initial setup
    info!("Performing initial setup...");
    if let Err(e) = handler.on_init() {
        error!("Initial setup failed: {e}");
    }

    // 2. Broadcast channel for reload events
    let (tx, _rx) = broadcast::channel::<ReloadSignal>(16);
    let reload_tx = Arc::new(tx);

    // 3. Setup file watcher in a blocking thread
    let watcher_tx = Arc::clone(&reload_tx);
    let watcher_handler = Arc::clone(&handler);
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

            let signals = watcher_handler.on_events(&events);
            for signal in signals {
                let _ = watcher_tx.send(signal);
            }
        }
    });

    // 4. Axum server
    let ws_tx = Arc::clone(&reload_tx);
    let source_root = watch_dir.clone();
    let mut app = Router::new()
        .route(
            "/live-reload",
            get(move |ws: WebSocketUpgrade| {
                let tx = Arc::clone(&ws_tx);
                async move { ws.on_upgrade(|socket| handle_ws(socket, tx)) }
            }),
        )
        .route(
            "/__tmtbook/source",
            get(move |Query(q): Query<SourceQuery>| {
                let root = source_root.clone();
                async move { handle_source(root, q.path).await }
            }),
        );

    app = handler.extend_router(app);

    if let Some(ref dir) = serve_dir {
        app = app.fallback_service(ServeDir::new(dir));
    }

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

/// Query of a `GET /__tmtbook/source`: read `path`'s current raw contents
/// back, so a client can slice out the exact text a `data-tmt-start`/
/// `data-tmt-end` pair refers to -- rendered HTML has already lost the
/// original `.tmt` syntax (`**bold**` became `<em>`, ...), so there is no
/// other way to show a reader the real source.
#[derive(Debug, serde::Deserialize)]
struct SourceQuery {
    path: String,
}

async fn handle_source(root: PathBuf, path: String) -> Result<String, (StatusCode, String)> {
    let target = resolve_within(&root, &path).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    tokio::fs::read_to_string(&target)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("{}: {e}", target.display())))
}

/// Resolves `rel` against `root`, rejecting anything that would land
/// outside it -- `..` segments, an absolute path, or a symlink escape.
pub fn resolve_within(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("watched root {}: {e}", root.display()))?;
    let candidate = root
        .join(rel)
        .canonicalize()
        .map_err(|e| format!("no such file {rel}: {e}"))?;
    if candidate.starts_with(&root) {
        Ok(candidate)
    } else {
        Err(format!("{rel} is outside the served directory"))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory under the system temp dir, removed when
    /// dropped -- avoids pulling in a `tempfile` dependency for what's
    /// only ever a handful of tiny fixture files.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tmtbook-serve-test-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolve_within_accepts_a_file_under_the_root() {
        let root = TempDir::new("accepts");
        std::fs::write(root.path().join("page.tmt"), "hello").unwrap();
        let resolved = resolve_within(root.path(), "page.tmt").unwrap();
        assert_eq!(
            resolved,
            root.path().join("page.tmt").canonicalize().unwrap()
        );
    }

    #[test]
    fn resolve_within_rejects_a_path_that_climbs_out_of_the_root() {
        let root = TempDir::new("rejects");
        let inner = root.path().join("vault");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(root.path().join("secret.txt"), "nope").unwrap();
        assert!(resolve_within(&inner, "../secret.txt").is_err());
    }

    #[tokio::test]
    async fn handle_source_returns_the_files_raw_contents() {
        let root = TempDir::new("source");
        std::fs::write(root.path().join("page.tmt"), "- one\n- two\n").unwrap();

        let body = handle_source(root.path().to_path_buf(), "page.tmt".to_string())
            .await
            .unwrap();

        assert_eq!(body, "- one\n- two\n");
    }

    #[tokio::test]
    async fn handle_source_rejects_a_traversal_attempt() {
        let root = TempDir::new("source-traversal");
        let vault = root.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::write(root.path().join("secret.txt"), "nope").unwrap();

        let err = handle_source(vault, "../secret.txt".to_string())
            .await
            .unwrap_err();

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
