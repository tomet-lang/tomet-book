//! Library surface for the `tmtbook` package.
//!
//! `tmtbook-serve` (`crates/serve`) is split out because it's genuinely
//! generic dev-server infrastructure -- file watching, static serving,
//! websocket live reload -- with zero knowledge of books or Tomet documents
//! (see `crates/serve/src/lib.rs`'s `DevServerHandler` trait). Everything
//! that actually knows how to build a book (rendering, config, i18n) lives
//! directly in this crate instead.
//!
//! `TometDevHandler` is the glue between the two: it implements
//! `DevServerHandler` using this crate's own rendering pipeline, so it has to
//! live here, where both are available.

pub mod book;
pub mod config;
pub mod i18n;
pub mod serve;

pub use book::{BuildReport, DocFailure, build_book};
pub use config::BookConfig;
pub use serve::TometDevHandler;

use anyhow::Result;
use std::net::IpAddr;
use std::path::PathBuf;

/// Run the local dev server with live reload: builds a `TometDevHandler`
/// (tomet-aware incremental rebuilds) and hands it to
/// `tmtbook_serve::run_dev_server`, which owns the actual file-watching,
/// static serving, and websocket reload signaling.
pub async fn run_dev_server(
    src_dir: PathBuf,
    out_dir: PathBuf,
    config: BookConfig,
    host: IpAddr,
    port: u16,
) -> Result<()> {
    let prefix = Some(config.build.clean_url_prefix().to_string());
    let handler = TometDevHandler::new(src_dir.clone(), out_dir.clone(), config);
    tmtbook_serve::run_dev_server(src_dir, None, host, port, prefix, handler).await
}
