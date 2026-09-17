use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::LazyLock;

use super::loader::MediaFileInfo;
use crate::config::BookConfig;

/// The stylesheet, assembled in cascade order. The numeric prefixes are that
/// order -- moving a file changes which rule wins.
pub const BOOK_CSS: &str = concat!(
    include_str!("../assets/static/css/00-base.css"),
    include_str!("../assets/static/css/10-shell.css"),
    include_str!("../assets/static/css/20-panes.css"),
    include_str!("../assets/static/css/30-sticky-tabs.css"),
    include_str!("../assets/static/css/35-sticky-tabs-vertical.css"),
    include_str!("../assets/static/css/40-hero.css"),
    include_str!("../assets/static/css/50-nav-infobox.css"),
    include_str!("../assets/static/css/60-content.css"),
    include_str!("../assets/static/css/70-widgets.css"),
    include_str!("../assets/static/css/72-center-peek.css"),
    include_str!("../assets/static/css/90-responsive.css"),
    include_str!("../assets/static/css/98-no-js.css"),
);
/// The script, assembled in load order. Each file is a self-contained IIFE
/// that registers what others need on `window.TMT`, so the order only has to
/// put the shared helpers and the strings helper first.
pub const BOOK_JS: &str = concat!(
    include_str!("../assets/static/js/00-utils.js"),
    include_str!("../assets/static/js/00-strings.js"),
    include_str!("../assets/static/js/05-tmt-btn.js"),
    include_str!("../assets/static/js/06-tmt-badge.js"),
    include_str!("../assets/static/js/07-tmt-icon.js"),
    include_str!("../assets/static/js/10-theme.js"),
    include_str!("../assets/static/js/20-view-mode.js"),
    include_str!("../assets/static/js/30-floating-controls.js"),
    include_str!("../assets/static/js/31-page-progress.js"),
    include_str!("../assets/static/js/36-nav-pane.js"),
    include_str!("../assets/static/js/37-data-pane.js"),
    include_str!("../assets/static/js/38-recent-notes.js"),
    include_str!("../assets/static/js/39-lookup-pane.js"),
    include_str!("../assets/static/js/40-book-view.js"),
    include_str!("../assets/static/js/50-keyboard.js"),
    include_str!("../assets/static/js/60-search.js"),
    include_str!("../assets/static/js/70-preview.js"),
    include_str!("../assets/static/js/72-center-peek.js"),
    include_str!("../assets/static/js/75-lightbox.js"),
    include_str!("../assets/static/js/78-costume.js"),
    include_str!("../assets/static/js/80-router.js"),
    include_str!("../assets/static/js/85-classic-view.js"),
    include_str!("../assets/static/js/90-edit-menu.js"),
    include_str!("../assets/static/js/92-code-copy.js"),
    include_str!("../assets/static/js/95-init.js"),
    include_str!("../assets/static/js/97-live-reload.js"),
    include_str!("../assets/static/js/99-bootstrap.js"),
);

/// Every Lucide icon, as one `<symbol id="name">` per icon -- vendored
/// whole from `lucide-static` rather than one file per icon we might use,
/// so `@doc.icon("<name>", pkg:"lucide")` can reference any of them by
/// `<use href="/icons/lucide.svg#<name>">` without a code change to add one.
pub const LUCIDE_SPRITE: &str = include_str!("../assets/static/icons/lucide/sprite.svg");

/// A short digest of an asset's contents, for `?v=` cache busting.
///
/// Both files are served from fixed paths, so without this a reader who has
/// visited before keeps the stylesheet and script their browser cached, no
/// matter how many times the book is rebuilt.
fn content_version(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub static CSS_VERSION: LazyLock<String> = LazyLock::new(|| content_version(BOOK_CSS));
pub static JS_VERSION: LazyLock<String> = LazyLock::new(|| content_version(BOOK_JS));

pub fn write_static_assets(out_dir: &Path, src_dir: &Path, config: &BookConfig) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    // 1. Write embedded book.css
    super::write_if_changed(&out_dir.join("book.css"), BOOK_CSS)
        .context("Failed to write book.css")?;

    // 2. Write embedded book.js
    super::write_if_changed(&out_dir.join("book.js"), BOOK_JS)
        .context("Failed to write book.js")?;

    // 3. Write embedded icons/lucide.svg
    super::write_if_changed(&out_dir.join("icons/lucide.svg"), LUCIDE_SPRITE)
        .context("Failed to write icons/lucide.svg")?;

    // 4. Copy user's custom CSS if specified and exists
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

    // Not the same inode, so the destination was copied rather than linked --
    // which is what happens whenever the vault and the output sit on different
    // filesystems. `fs::copy` carries the permissions but not the timestamps,
    // so the two mtimes never match and demanding that they do would re-copy
    // every file on every build. The question to ask is the one make asks:
    // is the destination at least as new as the source, and the same size?
    if src_meta.len() == dest_meta.len()
        && let (Ok(src_mtime), Ok(dest_mtime)) = (src_meta.modified(), dest_meta.modified())
        && dest_mtime >= src_mtime
    {
        return true;
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    Unchanged,
    Linked,
    Copied,
}

pub fn sync_media_file(src: &Path, dest: &Path) -> Result<SyncOutcome> {
    if dest.exists() {
        if is_same_media(src, dest) {
            return Ok(SyncOutcome::Unchanged);
        }
        let _ = fs::remove_file(dest);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // A hard link costs nothing and shares the bytes; copying is the
    // fallback for when the two paths are on different filesystems.
    if fs::hard_link(src, dest).is_ok() {
        Ok(SyncOutcome::Linked)
    } else {
        fs::copy(src, dest)?;
        Ok(SyncOutcome::Copied)
    }
}

pub fn sync_single_media(
    out_dir: &Path,
    abs_path: &Path,
    rel_path: &str,
    config: &BookConfig,
) -> Result<()> {
    let target_vault = out_dir.join(config.build.asset_out_rel());
    let dest = target_vault.join(rel_path);
    sync_media_file(abs_path, &dest)?;
    Ok(())
}

/// How the media directory was brought up to date.
///
/// Worth reporting separately: `copied` is the expensive one, and a build that
/// keeps copying the same files every time means hard links are not available
/// between the vault and the output.
#[derive(Debug, Default)]
pub struct MediaSync {
    pub unchanged: usize,
    pub linked: usize,
    pub copied: usize,
}

impl MediaSync {
    pub fn total(&self) -> usize {
        self.unchanged + self.linked + self.copied
    }
}

pub fn copy_vault_media(
    out_dir: &Path,
    media_files: &[MediaFileInfo],
    config: &BookConfig,
) -> Result<MediaSync> {
    let target_vault = out_dir.join(config.build.asset_out_rel());
    fs::create_dir_all(&target_vault)?;

    let outcomes: Result<Vec<SyncOutcome>> = media_files
        .par_iter()
        .map(|media| {
            let dest = target_vault.join(&media.rel_path);
            sync_media_file(&media.abs_path, &dest)
        })
        .collect();

    let mut sync = MediaSync::default();
    for outcome in outcomes? {
        match outcome {
            SyncOutcome::Unchanged => sync.unchanged += 1,
            SyncOutcome::Linked => sync.linked += 1,
            SyncOutcome::Copied => sync.copied += 1,
        }
    }

    Ok(sync)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tmtbook-media-{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_hardlinked_file_counts_as_unchanged() {
        let dir = scratch("hardlink");
        let src = dir.join("a.png");
        let dest = dir.join("b.png");
        fs::write(&src, "data").unwrap();
        fs::hard_link(&src, &dest).unwrap();

        assert!(is_same_media(&src, &dest));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_copied_file_counts_as_unchanged() {
        // fs::copy carries the permissions but not the timestamps, so a
        // destination that was copied rather than linked -- which is what
        // happens whenever the vault and the output live on different
        // filesystems -- has an mtime of its own. Requiring the two to match
        // exactly would re-copy every file on every build.
        let dir = scratch("copy");
        let src = dir.join("a.png");
        let dest = dir.join("b.png");
        fs::write(&src, "data").unwrap();
        fs::copy(&src, &dest).unwrap();

        assert!(is_same_media(&src, &dest), "a fresh copy is up to date");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_copy_of_a_file_that_then_changed_is_stale() {
        // Same size, but the source moved on afterwards.
        let dir = scratch("changed");
        let src = dir.join("a.png");
        let dest = dir.join("b.png");
        fs::write(&src, "aaaa").unwrap();
        fs::copy(&src, &dest).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&src, "bbbb").unwrap();

        assert!(!is_same_media(&src, &dest));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_copy_is_not_unchanged() {
        let dir = scratch("stale");
        let src = dir.join("a.png");
        let dest = dir.join("b.png");
        fs::write(&dest, "old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&src, "newer data").unwrap();

        assert!(!is_same_media(&src, &dest));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tmt_components_are_bundled_in_book_assets() {
        assert!(
            super::BOOK_JS.contains("customElements.define('tmt-btn'"),
            "tmt-btn custom element is defined in BOOK_JS"
        );
        assert!(
            super::BOOK_JS.contains("customElements.define('tmt-badge'"),
            "tmt-badge custom element is defined in BOOK_JS"
        );
        assert!(
            super::BOOK_JS.contains("customElements.define('tmt-icon'"),
            "tmt-icon custom element is defined in BOOK_JS"
        );
        assert!(
            super::BOOK_CSS.contains("tmt-btn"),
            "tmt-btn light DOM base rules are in BOOK_CSS"
        );
    }
}
