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
/// order -- moving a file changes which rule wins. Tailwind's compiled
/// output (build.rs runs the standalone `tailwindcss` CLI against
/// tailwind-input.css, scanning the Jinja templates for classes actually
/// used) is positioned so its utilities win over the always-on hand-authored
/// rules above, but *before* 90-responsive.css -- that file's media-query
/// overrides must still win within their own breakpoint, which an
/// unconditional utility positioned after them would otherwise clobber at
/// every viewport width, not just the ones the media query targets.
pub const BOOK_CSS: &str = concat!(
    include_str!("../../frontend/styles/00-base.css"),
    include_str!("../../frontend/styles/10-shell.css"),
    include_str!("../../frontend/styles/20-panes.css"),
    include_str!("../../frontend/styles/30-sticky-tabs.css"),
    include_str!("../../frontend/styles/35-sticky-tabs-vertical.css"),
    include_str!("../../frontend/styles/40-hero.css"),
    include_str!("../../frontend/styles/50-nav-infobox.css"),
    include_str!("../../frontend/styles/60-content.css"),
    include_str!("../../frontend/styles/70-widgets.css"),
    include_str!("../../frontend/styles/71-source-viewer.css"),
    include_str!("../../frontend/styles/72-center-peek.css"),
    include_str!(concat!(env!("OUT_DIR"), "/tailwind.css")),
    include_str!("../../frontend/styles/90-responsive.css"),
    include_str!("../../frontend/styles/98-no-js.css"),
);
/// The compiled JavaScript runtime for tmtbook, bundled by esbuild from
/// `ui/src/index.ts` during the cargo build script.
pub const BOOK_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/tmtbook.js"));

/// Every Lucide icon, as one `<symbol id="name">` per icon -- vendored
/// whole from `lucide-static` rather than one file per icon we might use,
/// so `@doc.icon("<name>", pkg:"lucide")` can reference any of them by
/// `<use href="/icons/lucide.svg#<name>">` without a code change to add one.
pub const LUCIDE_SPRITE: &str = include_str!("../../frontend/icons/lucide/sprite.svg");
/// Simple Icons sprite sheet, vendored from `simple-icons`,
/// so `@doc.icon("<name>", pkg:"simple")` and external link icons can
/// reference any of them by `<use href="/icons/simple.svg#<name>">`.
pub const SIMPLE_SPRITE: &str = include_str!("../../frontend/icons/simple/sprite.svg");

/// One stylesheet per Custom Element, kept out of `BOOK_CSS` on purpose: each
/// is `<link>`-ed from inside that element's declarative shadow root (see
/// `dsd_btn_shadow` and friends in macros.html) instead of being inlined
/// per-instance, so the browser fetches and caches it once no matter how many
/// `<tmt-btn>` etc. appear on a page.
pub const TMT_BTN_CSS: &str = include_str!("../../frontend/styles/components/tmt-btn.css");
pub const TMT_BADGE_CSS: &str = include_str!("../../frontend/styles/components/tmt-badge.css");
pub const TMT_ICON_CSS: &str = include_str!("../../frontend/styles/components/tmt-icon.css");
pub const TMT_SWATCH_CSS: &str = include_str!("../../frontend/styles/components/tmt-swatch.css");
pub const TMT_SWITCH_CSS: &str = include_str!("../../frontend/styles/components/tmt-switch.css");

/// A short digest of an asset's contents, for `?v=` cache busting.
///
/// These files are served from fixed paths, so without this a reader who has
/// visited before keeps the stylesheet and script their browser cached, no
/// matter how many times the book is rebuilt.
fn content_version(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub static CSS_VERSION: LazyLock<String> = LazyLock::new(|| content_version(BOOK_CSS));
pub static JS_VERSION: LazyLock<String> = LazyLock::new(|| content_version(BOOK_JS));
pub static TMT_BTN_CSS_VERSION: LazyLock<String> = LazyLock::new(|| content_version(TMT_BTN_CSS));
pub static TMT_BADGE_CSS_VERSION: LazyLock<String> =
    LazyLock::new(|| content_version(TMT_BADGE_CSS));
pub static TMT_ICON_CSS_VERSION: LazyLock<String> = LazyLock::new(|| content_version(TMT_ICON_CSS));
pub static TMT_SWATCH_CSS_VERSION: LazyLock<String> =
    LazyLock::new(|| content_version(TMT_SWATCH_CSS));
pub static TMT_SWITCH_CSS_VERSION: LazyLock<String> =
    LazyLock::new(|| content_version(TMT_SWITCH_CSS));

pub fn write_static_assets(out_dir: &Path, src_dir: &Path, config: &BookConfig) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    // 1. Write embedded tmtbook.css
    super::write_if_changed(&out_dir.join("tmtbook.css"), BOOK_CSS)
        .context("Failed to write tmtbook.css")?;

    // 2. Write embedded tmtbook.js
    super::write_if_changed(&out_dir.join("tmtbook.js"), BOOK_JS)
        .context("Failed to write tmtbook.js")?;

    // 3. Write embedded icons/lucide.svg and icons/simple.svg
    super::write_if_changed(&out_dir.join("icons/lucide.svg"), LUCIDE_SPRITE)
        .context("Failed to write icons/lucide.svg")?;
    super::write_if_changed(&out_dir.join("icons/simple.svg"), SIMPLE_SPRITE)
        .context("Failed to write icons/simple.svg")?;

    // 4. Write embedded per-component stylesheets (linked from inside each
    // Custom Element's declarative shadow root -- see TMT_BTN_CSS above).
    super::write_if_changed(&out_dir.join("components/tmt-btn.css"), TMT_BTN_CSS)
        .context("Failed to write components/tmt-btn.css")?;
    super::write_if_changed(&out_dir.join("components/tmt-badge.css"), TMT_BADGE_CSS)
        .context("Failed to write components/tmt-badge.css")?;
    super::write_if_changed(&out_dir.join("components/tmt-icon.css"), TMT_ICON_CSS)
        .context("Failed to write components/tmt-icon.css")?;
    super::write_if_changed(&out_dir.join("components/tmt-swatch.css"), TMT_SWATCH_CSS)
        .context("Failed to write components/tmt-swatch.css")?;
    super::write_if_changed(&out_dir.join("components/tmt-switch.css"), TMT_SWITCH_CSS)
        .context("Failed to write components/tmt-switch.css")?;

    // 5. Copy user's custom CSS if specified and exists
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
            super::BOOK_JS.contains("customElements.define('tmt-btn'")
                || super::BOOK_JS.contains("customElements.define(\"tmt-btn\""),
            "tmt-btn custom element is defined in BOOK_JS"
        );
        assert!(
            super::BOOK_JS.contains("customElements.define('tmt-badge'")
                || super::BOOK_JS.contains("customElements.define(\"tmt-badge\""),
            "tmt-badge custom element is defined in BOOK_JS"
        );
        assert!(
            super::BOOK_JS.contains("customElements.define('tmt-icon'")
                || super::BOOK_JS.contains("customElements.define(\"tmt-icon\""),
            "tmt-icon custom element is defined in BOOK_JS"
        );
        assert!(
            super::BOOK_CSS.contains("tmt-btn"),
            "tmt-btn light DOM base rules are in BOOK_CSS"
        );
    }
}
