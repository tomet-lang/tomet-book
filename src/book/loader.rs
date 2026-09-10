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

const MEDIA_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "webp", "pdf", "mp4", "mp3", "webm", "avif", "ico",
];

pub fn scan_vault(src_dir: &Path, config: &BookConfig) -> Result<ScannedVault> {
    let mut doc_files = Vec::new();
    let mut media_files = Vec::new();
    let mut all_rel_paths = Vec::new();

    let exclude_prefixes: Vec<String> = config
        .build
        .exclude
        .iter()
        .map(|s| s.replace('\\', "/"))
        .collect();

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

        // Check if path starts with excluded prefix
        let is_excluded = exclude_prefixes
            .iter()
            .any(|prefix| rel_str.starts_with(prefix) || rel_str.contains(&format!("/{prefix}")));
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
