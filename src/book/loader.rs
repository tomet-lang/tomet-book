use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::BookConfig;

#[derive(Debug, Clone)]
pub struct ScannedVault {
    pub doc_files: Vec<DocFileInfo>,
    pub media_files: Vec<MediaFileInfo>,
    pub vault_index: tomet_links::VaultLinkIndex,
    pub workspace_config_src: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DocFileInfo {
    pub abs_path: PathBuf,
    pub rel_path: String,
}

#[derive(Debug, Clone)]
pub struct MediaFileInfo {
    pub abs_path: PathBuf,
    pub rel_path: String,
}

pub const MEDIA_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "webp", "pdf", "mp4", "mp3", "webm", "avif", "ico",
];

/// Returns true when `rel` is the excluded path `prefix` itself, or lives under it.
///
/// Matching happens on path-segment boundaries, so an exclude entry like `dist`
/// drops `dist/index.tmt` but keeps `distributed.tmt` and `old/distro/x.tmt`.
/// Multi-segment entries (`"00-09 System/01 Apps"`) are matched as a whole.
fn is_excluded_path(rel: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        return false;
    }

    std::iter::once(0)
        .chain(rel.match_indices('/').map(|(i, _)| i + 1))
        .any(|start| {
            rel[start..]
                .strip_prefix(prefix)
                .is_some_and(|tail| tail.is_empty() || tail.starts_with('/'))
        })
}

pub fn scan_vault(src_dir: &Path, config: &BookConfig) -> Result<ScannedVault> {
    let mut doc_files = Vec::new();
    let mut media_files = Vec::new();
    let mut all_rel_paths = Vec::new();

    let mut exclude_prefixes: Vec<String> = config
        .build
        .exclude
        .iter()
        .map(|s| s.replace('\\', "/"))
        .collect();

    let dest_str = config.book.dest.to_string_lossy().replace('\\', "/");
    if !dest_str.is_empty() && !exclude_prefixes.contains(&dest_str) {
        exclude_prefixes.push(dest_str);
    }

    let media_ext_set: HashSet<&str> = MEDIA_EXTENSIONS.iter().copied().collect();

    for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel = match path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        // Check if path is, or lives under, an excluded directory
        let is_excluded = exclude_prefixes
            .iter()
            .any(|prefix| is_excluded_path(&rel_str, prefix));
        if is_excluded {
            continue;
        }

        // Hidden files/directories
        if rel_str.starts_with('.') || rel_str.contains("/.") {
            continue;
        }

        all_rel_paths.push(rel_str.clone());

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "tmt" || ext == "tm" {
            doc_files.push(DocFileInfo {
                abs_path: path.to_path_buf(),
                rel_path: rel_str,
            });
        } else if media_ext_set.contains(ext.as_str()) {
            media_files.push(MediaFileInfo {
                abs_path: path.to_path_buf(),
                rel_path: rel_str,
            });
        }
    }

    // Build link resolution index
    let vault_index = tomet_links::VaultLinkIndex::from_paths(&all_rel_paths);

    // Check for default.config.tmt
    let workspace_cfg_path = if let Some(custom) = &config.build.config_path {
        src_dir.join(custom)
    } else {
        src_dir.join("default.config.tmt")
    };

    let workspace_config_src = if workspace_cfg_path.exists() {
        std::fs::read_to_string(&workspace_cfg_path).ok()
    } else {
        None
    };

    Ok(ScannedVault {
        doc_files,
        media_files,
        vault_index,
        workspace_config_src,
    })
}

#[cfg(test)]
mod tests {
    use super::is_excluded_path;

    #[test]
    fn excludes_the_directory_itself_and_its_contents() {
        assert!(is_excluded_path("dist", "dist"));
        assert!(is_excluded_path("dist/wiki/index.tmt", "dist"));
        assert!(is_excluded_path("notes/dist/a.tmt", "dist"));
        assert!(is_excluded_path("a/b/dist", "dist"));
    }

    #[test]
    fn does_not_match_partial_segment_names() {
        assert!(!is_excluded_path("distributed-notes.tmt", "dist"));
        assert!(!is_excluded_path("old/distro/x.tmt", "dist"));
        assert!(!is_excluded_path("my-target/a.tmt", "target"));
        assert!(!is_excluded_path("notes/targets.tmt", "target"));
    }

    #[test]
    fn matches_multi_segment_entries_as_a_whole() {
        let prefix = "00-09 System/01 Apps";
        assert!(is_excluded_path("00-09 System/01 Apps/zed.tmt", prefix));
        assert!(is_excluded_path(
            "vault/00-09 System/01 Apps/zed.tmt",
            prefix
        ));
        assert!(!is_excluded_path("00-09 System/02 Tools/zed.tmt", prefix));
        assert!(!is_excluded_path("00-09 System/01 Appsx/zed.tmt", prefix));
    }

    #[test]
    fn ignores_empty_and_slash_only_entries() {
        assert!(!is_excluded_path("notes/a.tmt", ""));
        assert!(!is_excluded_path("notes/a.tmt", "/"));
        assert!(is_excluded_path("notes/a.tmt", "/notes/"));
    }
}
