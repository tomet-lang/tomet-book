use anyhow::Result;
use notify::Event;
use percent_encoding::percent_decode_str;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{info, warn};

use crate::book::VaultState;
use crate::book::loader::{exclude_prefixes, is_ignored_rel};
use crate::config::BookConfig;
use tmtbook_serve::axum::Router;
use tmtbook_serve::axum::http::{StatusCode, Uri, header};
use tmtbook_serve::axum::response::{Html, IntoResponse, Response};
use tmtbook_serve::{DevServerHandler, ReloadSignal, resolve_within};

pub struct TometDevHandler {
    src_dir: PathBuf,
    out_dir: PathBuf,
    config: RwLock<BookConfig>,
    excludes: RwLock<Vec<String>>,
    state: Arc<RwLock<Option<VaultState>>>,
}

impl TometDevHandler {
    pub fn new(src_dir: PathBuf, out_dir: PathBuf, config: BookConfig) -> Self {
        let excludes = exclude_prefixes(&config);
        Self {
            src_dir,
            out_dir,
            config: RwLock::new(config),
            excludes: RwLock::new(excludes),
            state: Arc::new(RwLock::new(None)),
        }
    }

    pub fn state(&self) -> &Arc<RwLock<Option<VaultState>>> {
        &self.state
    }

    pub fn full_rebuild(&self) -> bool {
        let start = Instant::now();
        let reloaded_config = match BookConfig::load_from_dir(&self.src_dir) {
            Ok(cfg) => {
                info!("⚙️ Reloaded configuration from {}", self.src_dir.display());
                let mut cfg_lock = self.config.write().unwrap();
                *cfg_lock = cfg.clone();
                let mut exc_lock = self.excludes.write().unwrap();
                *exc_lock = exclude_prefixes(&cfg);
                cfg
            }
            Err(e) => {
                warn!("Failed to reload tmtbook.toml: {e}");
                self.config.read().unwrap().clone()
            }
        };

        match VaultState::init(&self.src_dir, &reloaded_config, true) {
            Ok(vault_state) => {
                info!(
                    "⚡ In-memory vault index ready in {:?} ({} docs, {} routes)",
                    start.elapsed(),
                    vault_state.scanned.doc_files.len(),
                    vault_state.scanned.doc_files.len() - vault_state.unpublished.len()
                );
                *self.state.write().unwrap() = Some(vault_state);
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
    fn on_init(&self) -> Result<()> {
        let start = Instant::now();
        let config = self.config.read().unwrap().clone();
        match VaultState::init(&self.src_dir, &config, true) {
            Ok(vault_state) => {
                info!(
                    "⚡ Dev server ready in {:?} (in-memory SSR, {} docs)",
                    start.elapsed(),
                    vault_state.scanned.doc_files.len()
                );
                *self.state.write().unwrap() = Some(vault_state);
            }
            Err(e) => warn!("Initial vault indexing failed: {e}"),
        }
        Ok(())
    }

    fn on_events(&self, events: &[Event]) -> Vec<ReloadSignal> {
        let mut signals = Vec::new();
        let excludes = self.excludes.read().unwrap().clone();
        let plan = plan_events(events, &self.src_dir, &self.out_dir, &excludes);
        if plan.is_empty() {
            return signals;
        }

        let needs_full = plan.global;
        if needs_full {
            info!("🔄 Global change detected, re-indexing vault (in-memory)...");
            if self.full_rebuild() {
                signals.push(ReloadSignal::Full);
            }
            return signals;
        }

        if plan.css {
            info!("🎨 CSS changed, hot reloading stylesheets");
            signals.push(ReloadSignal::Css);
        }

        if !plan.docs.is_empty() {
            let state_lock = self.state.read().unwrap();
            if let Some(ref state) = *state_lock {
                let clean_prefix = state.config.build.clean_url_prefix();
                for (_abs_path, rel_path) in &plan.docs {
                    if let Some(slug) = state.route_table.get(rel_path) {
                        let doc_url = format!("{clean_prefix}/{slug}");
                        info!("⚡ Document updated: {rel_path} -> {doc_url}");
                        signals.push(ReloadSignal::Doc { url: doc_url });
                    } else {
                        drop(state_lock);
                        if self.full_rebuild() {
                            signals.push(ReloadSignal::Full);
                        }
                        return signals;
                    }
                }
            } else {
                drop(state_lock);
                if self.full_rebuild() {
                    signals.push(ReloadSignal::Full);
                }
                return signals;
            }
        }

        if !plan.media.is_empty() {
            info!("🖼️ Media changed, triggering reload");
            signals.push(ReloadSignal::Full);
        }

        signals
    }

    fn extend_router(&self, router: Router) -> Router {
        let state = Arc::clone(&self.state);
        let src_dir = self.src_dir.clone();
        let fallback_config = self.config.read().unwrap().clone();

        router.fallback(move |uri: Uri| {
            let state = Arc::clone(&state);
            let src_dir = src_dir.clone();
            let fallback_config = fallback_config.clone();
            async move { handle_http_request(&uri, &state, &src_dir, &fallback_config) }
        })
    }
}

pub fn handle_http_request(
    uri: &Uri,
    state: &RwLock<Option<VaultState>>,
    src_dir: &Path,
    config: &BookConfig,
) -> Response {
    let raw_path = uri.path();
    let path = percent_decode_str(raw_path)
        .decode_utf8()
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| raw_path.to_string());

    // 1. Embedded static assets
    match path.as_str() {
        "/tmtbook.css" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                crate::book::assets::BOOK_CSS,
            )
                .into_response();
        }
        "/tmtbook.js" => {
            return (
                StatusCode::OK,
                [(
                    header::CONTENT_TYPE,
                    "application/javascript; charset=utf-8",
                )],
                crate::book::assets::BOOK_JS,
            )
                .into_response();
        }
        "/icons/lucide.svg" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "image/svg+xml")],
                crate::book::assets::LUCIDE_SPRITE,
            )
                .into_response();
        }
        "/icons/simple.svg" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "image/svg+xml")],
                crate::book::assets::SIMPLE_SPRITE,
            )
                .into_response();
        }
        "/components/tmt-btn.css" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                crate::book::assets::TMT_BTN_CSS,
            )
                .into_response();
        }
        "/components/tmt-badge.css" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                crate::book::assets::TMT_BADGE_CSS,
            )
                .into_response();
        }
        "/components/tmt-icon.css" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                crate::book::assets::TMT_ICON_CSS,
            )
                .into_response();
        }
        "/components/tmt-swatch.css" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                crate::book::assets::TMT_SWATCH_CSS,
            )
                .into_response();
        }
        "/components/tmt-switch.css" => {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                crate::book::assets::TMT_SWITCH_CSS,
            )
                .into_response();
        }
        _ => {}
    }

    let (clean_url_prefix, clean_asset_prefix) = {
        let state_lock = state.read().unwrap();
        if let Some(ref st) = *state_lock {
            (
                st.config.build.clean_url_prefix().to_string(),
                st.config.build.clean_asset_prefix().to_string(),
            )
        } else {
            (
                config.build.clean_url_prefix().to_string(),
                config.build.clean_asset_prefix().to_string(),
            )
        }
    };
    let clean_url_prefix = clean_url_prefix.as_str();
    let clean_asset_prefix = clean_asset_prefix.as_str();

    // 2. Search index (in-memory)
    let is_search_index = path == "/search-index.json"
        || (!clean_url_prefix.is_empty()
            && path == format!("{clean_url_prefix}/search-index.json"));

    if is_search_index {
        let state_lock = state.read().unwrap();
        if let Some(ref st) = *state_lock
            && let Some(ref json) = st.search_index_json
        {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                json.clone(),
            )
                .into_response();
        }
    }

    // 3. Root "/"
    if path == "/" || path.is_empty() {
        if !clean_url_prefix.is_empty() {
            let redirect_target = format!("{clean_url_prefix}/");
            return (StatusCode::FOUND, [(header::LOCATION, redirect_target)], "").into_response();
        } else {
            let state_lock = state.read().unwrap();
            if let Some(ref st) = *state_lock {
                match st.render_index_page() {
                    Ok(html) => return Html(html).into_response(),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Html(format!("<h1>500 Internal Server Error</h1><pre>{e}</pre>")),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    // 3. Catalog index (e.g. "/wiki", "/wiki/", "/wiki/index.html")
    let is_catalog_index = if !clean_url_prefix.is_empty() {
        path == clean_url_prefix
            || path == format!("{clean_url_prefix}/")
            || path == format!("{clean_url_prefix}/index.html")
    } else {
        path == "/" || path == "/index.html"
    };

    if is_catalog_index {
        let state_lock = state.read().unwrap();
        if let Some(ref st) = *state_lock {
            match st.render_index_page() {
                Ok(html) => return Html(html).into_response(),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html(format!("<h1>500 Internal Server Error</h1><pre>{e}</pre>")),
                    )
                        .into_response();
                }
            }
        }
    }

    // 4. Wiki page on-demand rendering
    let wiki_subpath = if !clean_url_prefix.is_empty() {
        path.strip_prefix(clean_url_prefix)
            .map(|s| s.trim_matches('/'))
    } else {
        Some(path.trim_matches('/'))
    };

    if let Some(sub) = wiki_subpath
        && !sub.is_empty()
    {
        let slug = sub
            .trim_end_matches("/index.html")
            .trim_end_matches(".html")
            .trim_matches('/');
        let state_lock = state.read().unwrap();
        if let Some(ref st) = *state_lock {
            match st.render_page_by_slug(slug) {
                Ok(Some(html)) => return Html(html).into_response(),
                Ok(None) => {}
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html(format!("<h1>500 Internal Server Error</h1><pre>{e}</pre>")),
                    )
                        .into_response();
                }
            }
        }
    }

    // 5. Vault media / custom CSS streaming
    let media_rel = if !clean_asset_prefix.is_empty()
        && let Some(rel) = path.strip_prefix(clean_asset_prefix)
    {
        Some(rel.trim_matches('/'))
    } else {
        Some(path.trim_matches('/'))
    };

    if let Some(rel) = media_rel
        && !rel.is_empty()
        && let Ok(target) = resolve_within(src_dir, rel)
        && target.is_file()
        && let Ok(bytes) = std::fs::read(&target)
    {
        let mime = guess_mime(&target);
        return (StatusCode::OK, [(header::CONTENT_TYPE, mime)], bytes).into_response();
    }

    // 6. 404 Not Found fallback
    (
        StatusCode::NOT_FOUND,
        Html(format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>404 Not Found</title></head><body><h1>404 Not Found</h1><p>Cannot find: <code>{}</code></p></body></html>",
            path
        )),
    )
        .into_response()
}

fn guess_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "html" => "text/html; charset=utf-8",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
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

#[cfg(test)]
mod ssr_tests {
    use super::*;
    use std::fs;
    use tmtbook_serve::axum::body::to_bytes;
    use tmtbook_serve::axum::http::header;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tmtbook-ssr-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_dev_server_ssr_renders_page_on_demand() {
        let src = scratch("ssr-render-src");
        let rust_dir = src.join("tech/rust");
        fs::create_dir_all(&rust_dir).unwrap();
        fs::write(
            rust_dir.join("closures.tmt"),
            "#[ Rust Closures ]\n\nClosures are functions that can capture enclosing environment.\n",
        )
        .unwrap();

        let mut config = BookConfig::default();
        config.build.url_prefix = "/wiki".to_string();

        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        // Check that NO files were written to src/dist!
        assert!(
            !src.join("dist").exists(),
            "SSR dev server must not write to dist!"
        );

        // Request /wiki/tech/rust/closures
        let uri = Uri::from_static("/wiki/tech/rust/closures");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );

        let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(body_str.contains("Rust Closures"));
        assert!(body_str.contains("Closures are functions"));

        // Still no files on disk!
        assert!(!src.join("dist").exists());

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_serves_embedded_assets() {
        let src = scratch("ssr-assets-src");
        let config = BookConfig::default();
        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        // 1. CSS
        let uri = Uri::from_static("/tmtbook.css?v=12345");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/css; charset=utf-8"
        );

        // 2. JS
        let uri = Uri::from_static("/tmtbook.js?v=12345");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/javascript; charset=utf-8"
        );

        // 3. Icons SVG
        let uri = Uri::from_static("/icons/lucide.svg");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/svg+xml"
        );

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_root_redirect_and_catalog() {
        let src = scratch("ssr-catalog-src");
        fs::write(src.join("readme.tmt"), "#[ Welcome ]\n\nHello book.\n").unwrap();

        let mut config = BookConfig::default();
        config.build.url_prefix = "/wiki".to_string();

        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        // 1. Root redirect / -> /wiki/
        let uri = Uri::from_static("/");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::FOUND);
        assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/wiki/");

        // 2. Catalog index /wiki/
        let uri = Uri::from_static("/wiki/");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(body_str.contains("Welcome") || body_str.contains("readme"));

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_serves_vault_media_directly() {
        let src = scratch("ssr-media-src");
        let img_dir = src.join("media");
        fs::create_dir_all(&img_dir).unwrap();
        fs::write(
            img_dir.join("photo.png"),
            b"\x89PNG\r\n\x1a\nfakeimagebytes",
        )
        .unwrap();

        let mut config = BookConfig::default();
        config.build.asset_prefix = "/assets".to_string();

        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        let uri = Uri::from_static("/assets/media/photo.png");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );

        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&bytes[..], b"\x89PNG\r\n\x1a\nfakeimagebytes");

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_serves_media_with_a_literal_plus_in_the_filename() {
        // A browser leaves a literal '+' in a URL path segment unescaped --
        // '+' meaning space is a query-string/form-encoding convention, and
        // must not be applied when decoding a path back to a filename.
        let src = scratch("ssr-media-plus-src");
        let img_dir = src.join("media");
        fs::create_dir_all(&img_dir).unwrap();
        fs::write(
            img_dir.join("+771a262c.png"),
            b"\x89PNG\r\n\x1a\nfakeimagebytes",
        )
        .unwrap();

        let mut config = BookConfig::default();
        config.build.asset_prefix = "/assets".to_string();

        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        let uri = Uri::from_static("/assets/media/+771a262c.png");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_handles_404() {
        let src = scratch("ssr-404-src");
        let config = BookConfig::default();
        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        let uri = Uri::from_static("/wiki/non-existent-page");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_hot_reloads_config() {
        use notify::event::{DataChange, ModifyKind};

        let src = scratch("ssr-config-reload-src");
        fs::write(
            src.join("hello.tmt"),
            "#[ Hello ]\n\nWelcome to our wiki.\n",
        )
        .unwrap();

        // Initial tmtbook.toml with a custom UI string override
        fs::write(
            src.join("tmtbook.toml"),
            r#"
[book]
title = "Original Title"

[ui.strings]
"panel.toc" = "Initial Contents Label"
"#,
        )
        .unwrap();

        let initial_config = BookConfig::load_from_dir(&src).unwrap();
        let handler = TometDevHandler::new(src.clone(), src.join("dist"), initial_config.clone());
        handler.on_init().unwrap();

        // 1. Initial request: should render with Initial Contents Label
        let uri = Uri::from_static("/wiki/hello");
        let resp = handle_http_request(&uri, handler.state(), &src, &initial_config);
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(body_str.contains("Initial Contents Label"));
        assert!(!body_str.contains("Hot Reloaded Contents Label"));

        // 2. Modify tmtbook.toml
        fs::write(
            src.join("tmtbook.toml"),
            r#"
[book]
title = "Updated Title"

[ui.strings]
"panel.toc" = "Hot Reloaded Contents Label"
"#,
        )
        .unwrap();

        // 3. Send file modification event for tmtbook.toml
        let ev = notify::Event {
            kind: notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![src.join("tmtbook.toml")],
            attrs: Default::default(),
        };
        let signals = handler.on_events(&[ev]);
        assert_eq!(signals, vec![ReloadSignal::Full]);

        // 4. Subsequent request: should immediately render with updated label!
        let resp_after = handle_http_request(&uri, handler.state(), &src, &initial_config);
        assert_eq!(resp_after.status(), StatusCode::OK);
        let body_bytes_after = to_bytes(resp_after.into_body(), usize::MAX).await.unwrap();
        let body_str_after = String::from_utf8_lossy(&body_bytes_after);
        assert!(body_str_after.contains("Hot Reloaded Contents Label"));
        assert!(!body_str_after.contains("Initial Contents Label"));

        let _ = fs::remove_dir_all(&src);
    }

    #[tokio::test]
    async fn test_dev_server_serves_in_memory_search_index() {
        let src = scratch("ssr-search-src");
        fs::write(
            src.join("hello.tmt"),
            "#[ Hello World ]\n\nWelcome to tomet search.\n",
        )
        .unwrap();

        let mut config = BookConfig::default();
        config.build.url_prefix = "/wiki".to_string();

        let handler = TometDevHandler::new(src.clone(), src.join("dist"), config.clone());
        handler.on_init().unwrap();

        // 1. Request /search-index.json
        let uri = Uri::from_static("/search-index.json");
        let resp = handle_http_request(&uri, handler.state(), &src, &config);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );

        let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(body_str.contains("Hello World"));
        assert!(body_str.contains("Welcome to tomet search."));

        // 2. Also available at /wiki/search-index.json
        let uri_prefixed = Uri::from_static("/wiki/search-index.json");
        let resp_pref = handle_http_request(&uri_prefixed, handler.state(), &src, &config);
        assert_eq!(resp_pref.status(), StatusCode::OK);

        // 3. Ensure NO disk files written to dist
        assert!(!src.join("dist").exists());

        let _ = fs::remove_dir_all(&src);
    }
}
