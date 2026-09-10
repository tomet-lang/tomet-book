use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::loader::MediaFileInfo;
use crate::config::BookConfig;

pub fn write_static_assets(out_dir: &Path, src_dir: &Path, config: &BookConfig) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    // 1. Write embedded book.css
    let css_content = include_str!("../assets/static/book.css");
    super::write_if_changed(&out_dir.join("book.css"), css_content)
        .context("Failed to write book.css")?;

    // 2. Write embedded book.js
    let js_content = include_str!("../assets/static/book.js");
    super::write_if_changed(&out_dir.join("book.js"), js_content)
        .context("Failed to write book.js")?;

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

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

fn is_same_media(src_path: &Path, dest_path: &Path) -> bool {
    let Ok(src_meta) = fs::metadata(src_path) else {
        return false;
    };
    let Ok(dest_meta) = fs::metadata(dest_path) else {
        return false;
    };

    #[cfg(unix)]
    {
        if src_meta.dev() == dest_meta.dev() && src_meta.ino() == dest_meta.ino() {
            return true;
        }
    }

    if src_meta.len() == dest_meta.len()
        && let (Ok(src_mtime), Ok(dest_mtime)) = (src_meta.modified(), dest_meta.modified())
        && src_mtime == dest_mtime
    {
        return true;
    }

    false
}

pub fn sync_single_media(
    out_dir: &Path,
    abs_path: &Path,
    rel_path: &str,
    config: &BookConfig,
) -> Result<()> {
    let target_vault = out_dir.join(config.build.asset_out_rel());
    let dest = target_vault.join(rel_path);

    if dest.exists() {
        if is_same_media(abs_path, &dest) {
            return Ok(());
        }
        let _ = fs::remove_file(&dest);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if fs::hard_link(abs_path, &dest).is_err() {
        fs::copy(abs_path, &dest)?;
    }
    Ok(())
}

pub fn copy_vault_media(
    out_dir: &Path,
    media_files: &[MediaFileInfo],
    config: &BookConfig,
) -> Result<usize> {
    let target_vault = out_dir.join(config.build.asset_out_rel());
    fs::create_dir_all(&target_vault)?;

    let mut count = 0;
    for media in media_files {
        let dest = target_vault.join(&media.rel_path);

        if dest.exists() {
            if is_same_media(&media.abs_path, &dest) {
                count += 1;
                continue;
            }
            let _ = fs::remove_file(&dest);
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try hardlink first; fallback to copy
        let linked = fs::hard_link(&media.abs_path, &dest).is_ok()
            || fs::copy(&media.abs_path, &dest).is_ok();
        if linked {
            count += 1;
        }
    }

    Ok(count)
}
