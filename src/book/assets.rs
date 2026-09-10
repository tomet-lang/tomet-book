use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::loader::MediaFileInfo;
use crate::config::BookConfig;

pub fn write_static_assets(out_dir: &Path, src_dir: &Path, config: &BookConfig) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    // 1. Write embedded book.css
    let css_content = include_str!("../assets/static/book.css");
    fs::write(out_dir.join("book.css"), css_content).context("Failed to write book.css")?;

    // 2. Write embedded book.js
    let js_content = include_str!("../assets/static/book.js");
    fs::write(out_dir.join("book.js"), js_content).context("Failed to write book.js")?;

    // 3. Copy user's custom CSS if specified and exists
    for css_rel in &config.ui.custom_css {
        let clean_rel = css_rel.trim_start_matches('/');
        let src_file = src_dir.join(clean_rel);
        if src_file.exists() && src_file.is_file() {
            let dest_file = out_dir.join(clean_rel);
            if let Some(parent) = dest_file.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = fs::copy(&src_file, &dest_file);
        }
    }

    Ok(())
}

pub fn copy_vault_media(out_dir: &Path, media_files: &[MediaFileInfo]) -> Result<usize> {
    let target_vault = out_dir.join("vault");
    if target_vault.exists() {
        let _ = fs::remove_dir_all(&target_vault);
    }
    fs::create_dir_all(&target_vault)?;

    let mut count = 0;
    for media in media_files {
        let dest = target_vault.join(&media.rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try hardlink first; fallback to copy
        if fs::hard_link(&media.abs_path, &dest).is_ok() {
            count += 1;
        } else if fs::copy(&media.abs_path, &dest).is_ok() {
            count += 1;
        }
    }

    Ok(count)
}
