use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

use tmtbook::book;
use tmtbook::cli::{Cli, Commands};
use tmtbook::config::BookConfig;
use tmtbook::serve;

#[tokio::main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Build { dir, dest, strict } => {
            let config = BookConfig::load_from_dir(&dir)?;
            let src_dir = dir
                .canonicalize()
                .context("Failed to find source directory")?;
            let out_dir = dest.unwrap_or_else(|| src_dir.join(&config.book.dest));

            let report = book::build_book(&src_dir, &out_dir, &config)?;

            if strict && !report.failures.is_empty() {
                for failure in &report.failures {
                    eprintln!("  {}: {}", failure.rel_path, failure.error);
                }
                anyhow::bail!(
                    "{} document(s) failed to build (--strict)",
                    report.failures.len()
                );
            }
        }
        Commands::Serve {
            dir,
            dest,
            host,
            port,
        } => {
            let config = BookConfig::load_from_dir(&dir)?;
            let src_dir = dir
                .canonicalize()
                .context("Failed to find source directory")?;
            let out_dir = dest.unwrap_or_else(|| src_dir.join(&config.book.dest));

            serve::run_dev_server(src_dir, out_dir, config, host, port).await?;
        }
        Commands::Init { dir, title } => {
            let target_file = dir.join("tmtbook.toml");
            if target_file.exists() {
                anyhow::bail!("Configuration already exists: {}", target_file.display());
            }

            let book_title = title.unwrap_or_else(|| {
                dir.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("My Book")
                    .to_string()
            });

            let template = format!(
                r#"[book]
title = "{book_title}"
lang = "ja"
src = "."
dest = "dist"

[ui]
rail_title = "{book_title}"
default_view = "book"

[[ui.hero_chips]]
key = "parent"
label = "所属"

[[ui.hero_chips]]
key = "type.list-a"
label = "ファン"

[[ui.hero_chips]]
key = "type.element-a"

[[ui.hero_chips]]
key = "birth"
format = "birthday"

[build]
exclude = [
  "00-09 System/01 Apps"
]
"#
            );

            fs::write(&target_file, template)?;
            info!("Created configuration: {}", target_file.display());
        }
    }

    Ok(())
}
