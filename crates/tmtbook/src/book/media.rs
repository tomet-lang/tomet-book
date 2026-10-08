use anyhow::Result;
use rayon::prelude::*;
use std::fs;
use std::path::Path;

use super::loader::MediaFileInfo;
use tmtbook_config::BookConfig;

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
            tmtbook_assets::BOOK_JS.contains("customElements.define('tmt-btn'")
                || tmtbook_assets::BOOK_JS.contains("customElements.define(\"tmt-btn\""),
            "tmt-btn custom element is defined in BOOK_JS"
        );
        assert!(
            tmtbook_assets::BOOK_JS.contains("customElements.define('tmt-badge'")
                || tmtbook_assets::BOOK_JS.contains("customElements.define(\"tmt-badge\""),
            "tmt-badge custom element is defined in BOOK_JS"
        );
        assert!(
            tmtbook_assets::BOOK_JS.contains("customElements.define('tmt-icon'")
                || tmtbook_assets::BOOK_JS.contains("customElements.define(\"tmt-icon\""),
            "tmt-icon custom element is defined in BOOK_JS"
        );
        assert!(
            tmtbook_assets::BOOK_CSS.contains("tmt-btn"),
            "tmt-btn light DOM base rules are in BOOK_CSS"
        );
    }
}
