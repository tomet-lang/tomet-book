//! Library surface for `tmtbook`.
//!
//! Provides the primary API for building and serving Tomet books:
//! - [`book::build_book`]: Builds an entire book from a vault directory.
//! - [`run_dev_server`]: Runs a local development server with live reload.
//! - [`config::BookConfig`]: Configuration parser and models.

pub mod book;
pub mod serve;

// Companion subcrates re-exports
pub use tmtbook_assets as assets;
pub use tmtbook_assets::i18n;
pub use tmtbook_config as config;
pub use tmtbook_render as render;
pub use tmtbook_search as search;

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
