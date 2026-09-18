use anyhow::Result;
use std::path::Path;

#[cfg(not(feature = "embedded-search"))]
pub fn run_pagefind(out_dir: &Path) -> Result<()> {
    use std::process::Command;
    use tracing::{info, warn};

    info!("Running Pagefind search indexer on {}", out_dir.display());

    let status = Command::new("pagefind").arg("--site").arg(out_dir).status();

    match status {
        Ok(s) => {
            if s.success() {
                info!("Pagefind search index generated successfully");
            } else {
                warn!("Pagefind exited with non-zero status: {:?}", s.code());
            }
        }
        Err(e) => {
            warn!(
                "Pagefind executable not found or failed to execute: {}. Search indexing skipped.",
                e
            );
        }
    }

    Ok(())
}

/// Runs Pagefind's indexer in-process via its embedded service API, instead
/// of shelling out to a `pagefind` executable on PATH. Trades build time and
/// binary size (see the `embedded-search` feature) for a search index that
/// works regardless of what's installed on the machine running `tmtbook`.
#[cfg(feature = "embedded-search")]
pub fn run_pagefind(out_dir: &Path) -> Result<()> {
    use anyhow::Context;
    use pagefind::api::PagefindIndex;
    use pagefind::options::PagefindServiceConfig;
    use tracing::info;

    info!(
        "Running embedded Pagefind search indexer on {}",
        out_dir.display()
    );

    let out_dir = out_dir.to_path_buf();
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async move {
            let options = PagefindServiceConfig::builder().build();
            let mut index =
                PagefindIndex::new(Some(options)).context("Invalid Pagefind options")?;

            index
                .add_directory(out_dir.to_string_lossy().into_owned(), None)
                .await
                .context("Failed to walk build output for Pagefind indexing")?;

            index
                .write_files(Some(
                    out_dir.join("pagefind").to_string_lossy().into_owned(),
                ))
                .await
                .context("Failed to write Pagefind index")?;

            info!("Pagefind search index generated successfully");
            Ok(())
        })
    })
}
