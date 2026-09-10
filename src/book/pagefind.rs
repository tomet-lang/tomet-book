use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

pub fn run_pagefind(out_dir: &Path) -> Result<()> {
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
