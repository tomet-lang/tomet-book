use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::BookConfig;

#[derive(Debug, Clone)]
pub struct ScannedVault {
    pub doc_files: Vec<DocFileInfo>,
    pub media_files: Vec<MediaFileInfo>,
    pub vault_index: tomet_links::VaultLinkIndex,
    pub workspace_config_src: Option<String>,
    pub workspace_config_blocks: Vec<tomet_ast::Block>,
    /// The documents that shape the book without being part of it:
    /// `*.index.tmt` and `*.config.tmt`, in filename order, so a long index
    /// can be split the way a long stylesheet is.
    pub control_files: Vec<DocFileInfo>,
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
pub fn is_excluded_path(rel: &str, prefix: &str) -> bool {
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

/// The paths a scan skips: the configured `build.exclude` entries plus the
/// destination directory, so a build never eats its own output.
pub fn exclude_prefixes(config: &BookConfig) -> Vec<String> {
    let mut prefixes: Vec<String> = config
        .build
        .exclude
        .iter()
        .map(|s| super::normalize_path_str(s))
        .collect();

    let dest_str = super::normalize_path(&config.book.dest);
    if !dest_str.is_empty() && !prefixes.contains(&dest_str) {
        prefixes.push(dest_str);
    }

    prefixes
}

/// True when a vault-relative path is excluded by config, or is hidden.
///
/// Shared with the dev server so the watcher and the scanner agree on what
/// belongs to the book.
pub fn is_ignored_rel(rel_str: &str, exclude_prefixes: &[String]) -> bool {
    if rel_str.starts_with('.') || rel_str.contains("/.") {
        return true;
    }
    exclude_prefixes
        .iter()
        .any(|prefix| is_excluded_path(rel_str, prefix))
}

pub fn scan_vault(src_dir: &Path, config: &BookConfig) -> Result<ScannedVault> {
    let mut doc_files = Vec::new();
    let mut media_files = Vec::new();
    let mut control_files: Vec<DocFileInfo> = Vec::new();
    let mut all_rel_paths = Vec::new();

    let exclude_prefixes = exclude_prefixes(config);

    for entry in WalkDir::new(src_dir)
        .into_iter()
        .filter_entry(|e| {
            if e.path() == src_dir {
                return true;
            }
            let Ok(rel) = e.path().strip_prefix(src_dir) else {
                return false;
            };
            let rel_str = super::normalize_path(rel);
            !is_ignored_rel(&rel_str, &exclude_prefixes)
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel = match path.strip_prefix(src_dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = super::normalize_path(rel);

        all_rel_paths.push(rel_str.clone());

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "tmt" || ext == "tm" {
            // `*.index.tmt` and `*.config.tmt` shape the book; they are not
            // part of it. Left in `doc_files` they would each publish a page
            // of their own -- which `default.config.tmt` has been doing.
            if crate::book::catalog::is_control_document(&rel_str) {
                control_files.push(DocFileInfo {
                    abs_path: path.to_path_buf(),
                    rel_path: rel_str,
                });
                continue;
            }
            doc_files.push(DocFileInfo {
                abs_path: path.to_path_buf(),
                rel_path: rel_str,
            });
        } else if MEDIA_EXTENSIONS.contains(&ext.as_str()) {
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

    let workspace_config_blocks = workspace_config_src
        .as_deref()
        .map(crate::book::document::extract_workspace_config_blocks)
        .unwrap_or_default();

    // WalkDir does not promise an order, and the catalog must not depend on
    // one: the filenames decide.
    control_files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    Ok(ScannedVault {
        doc_files,
        media_files,
        vault_index,
        workspace_config_src,
        workspace_config_blocks,
        control_files,
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

#[cfg(test)]
mod ignore_tests {
    use super::is_ignored_rel;

    fn excludes() -> Vec<String> {
        vec!["dist".to_string(), "00-09 System/01 Apps".to_string()]
    }

    #[test]
    fn hidden_paths_are_ignored() {
        assert!(is_ignored_rel(".git/config", &excludes()));
        assert!(is_ignored_rel("notes/.obsidian/app.json", &excludes()));
        assert!(!is_ignored_rel("notes/a.tmt", &excludes()));
    }

    #[test]
    fn configured_excludes_are_ignored() {
        assert!(is_ignored_rel("dist/wiki/index.html", &excludes()));
        assert!(is_ignored_rel("00-09 System/01 Apps/zed.tmt", &excludes()));
    }

    #[test]
    fn lookalike_names_survive() {
        assert!(!is_ignored_rel("distributed.tmt", &excludes()));
        assert!(!is_ignored_rel(
            "00-09 System/02 Tools/zed.tmt",
            &excludes()
        ));
    }
}
