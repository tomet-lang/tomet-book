//! Pruning stale output HTML pages, assets, and emptied directories from the build destination.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use super::normalize_path;

/// Delete the pages named by `stale_slugs`.
///
/// The fast path: the previous build listed what it wrote, so a rename only
/// costs the removal itself rather than a walk of the whole site.
pub(crate) fn prune_listed_pages(wiki_root: &Path, stale_slugs: &[&str]) -> usize {
    let mut removed = 0;
    let mut touched_dirs = false;

    for slug in stale_slugs {
        let page = wiki_root.join(slug).join("index.html");
        if fs::remove_file(&page).is_ok() {
            removed += 1;
            touched_dirs = true;
        }
    }

    if touched_dirs {
        remove_empty_dirs(wiki_root);
    }
    removed
}

/// Delete the media named by `stale_rel`.
pub(crate) fn prune_listed_media(vault_root: &Path, stale_rel: &[&str]) -> usize {
    let mut removed = 0;
    let mut touched_dirs = false;

    for rel in stale_rel {
        if fs::remove_file(vault_root.join(rel)).is_ok() {
            removed += 1;
            touched_dirs = true;
        }
    }

    if touched_dirs {
        remove_empty_dirs(vault_root);
    }
    removed
}

/// Delete pages under `wiki_root` whose source document no longer exists.
///
/// The fallback for when no manifest is available -- a first build against an
/// existing output directory, or one written by a version that kept no record.
/// Without pruning, a renamed note keeps its old HTML forever and Pagefind
/// cheerfully indexes the orphan, so search leads to a dead page.
pub(crate) fn prune_stale_pages(wiki_root: &Path, keep_slugs: &HashSet<&str>) -> usize {
    if !wiki_root.is_dir() {
        return 0;
    }

    let mut removed = 0;
    for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !entry.file_type().is_file() || path.file_name() != Some(OsStr::new("index.html")) {
            continue;
        }

        let Some(parent) = path.parent() else {
            continue;
        };
        if parent == wiki_root {
            // The catalog page, not a document page.
            continue;
        }

        let Ok(rel) = parent.strip_prefix(wiki_root) else {
            continue;
        };
        let slug = normalize_path(rel);
        if keep_slugs.contains(slug.as_str()) {
            continue;
        }

        if fs::remove_file(path).is_ok() {
            removed += 1;
        }
    }

    remove_empty_dirs(wiki_root);
    removed
}

/// Delete media under `vault_root` that is no longer present in the source vault.
pub(crate) fn prune_stale_media(vault_root: &Path, keep_rel: &HashSet<&str>) -> usize {
    if !vault_root.is_dir() {
        return 0;
    }

    let mut removed = 0;
    for entry in WalkDir::new(vault_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let Ok(rel) = entry.path().strip_prefix(vault_root) else {
            continue;
        };
        let rel_str = normalize_path(rel);
        if keep_rel.contains(rel_str.as_str()) {
            continue;
        }

        if fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }

    remove_empty_dirs(vault_root);
    removed
}

/// Drop directories left empty by pruning, deepest first. The root itself stays.
pub(crate) fn remove_empty_dirs(root: &Path) {
    for entry in WalkDir::new(root)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path == root || !entry.file_type().is_dir() {
            continue;
        }
        // Fails while the directory still holds something, which is the point.
        let _ = fs::remove_dir(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tmtbook-prune-{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x").unwrap();
    }

    #[test]
    fn listed_pruning_removes_only_what_it_is_given() {
        let root = scratch("listed-pages");
        touch(&root.join("keep/index.html"));
        touch(&root.join("notes/gone/index.html"));

        assert_eq!(prune_listed_pages(&root, &["notes/gone"]), 1);
        assert!(root.join("keep/index.html").exists());
        assert!(!root.join("notes/gone").exists(), "emptied dir should go");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn listed_pruning_tolerates_paths_that_are_already_gone() {
        let root = scratch("listed-missing");
        touch(&root.join("keep/index.html"));

        assert_eq!(prune_listed_pages(&root, &["never-existed"]), 0);
        assert!(root.join("keep/index.html").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn listed_media_pruning_removes_only_what_it_is_given() {
        let root = scratch("listed-media");
        touch(&root.join("img/keep.png"));
        touch(&root.join("img/old.png"));

        assert_eq!(prune_listed_media(&root, &["img/old.png"]), 1);
        assert!(root.join("img/keep.png").exists());
        assert!(!root.join("img/old.png").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_pages_are_pruned_and_live_ones_kept() {
        let root = scratch("prune-pages");

        touch(&root.join("keep/index.html"));
        touch(&root.join("notes/gone/index.html"));
        touch(&root.join("index.html")); // the catalog page

        let keep: HashSet<&str> = ["keep"].into_iter().collect();
        let removed = prune_stale_pages(&root, &keep);

        assert_eq!(removed, 1);
        assert!(root.join("keep/index.html").exists());
        assert!(root.join("index.html").exists(), "catalog must survive");
        assert!(!root.join("notes/gone/index.html").exists());
        assert!(
            !root.join("notes/gone").exists(),
            "emptied directories should go too"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pruning_an_untouched_tree_removes_nothing() {
        let root = scratch("prune-none");
        touch(&root.join("a/index.html"));
        touch(&root.join("b/index.html"));

        let keep: HashSet<&str> = ["a", "b"].into_iter().collect();
        assert_eq!(prune_stale_pages(&root, &keep), 0);
        assert!(root.join("a/index.html").exists());
        assert!(root.join("b/index.html").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_media_is_pruned() {
        let root = scratch("prune-media");
        touch(&root.join("img/keep.png"));
        touch(&root.join("img/old.png"));

        let keep: HashSet<&str> = ["img/keep.png"].into_iter().collect();
        assert_eq!(prune_stale_media(&root, &keep), 1);
        assert!(root.join("img/keep.png").exists());
        assert!(!root.join("img/old.png").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pruning_a_missing_directory_is_a_no_op() {
        let root = scratch("prune-missing").join("does-not-exist");
        assert_eq!(prune_stale_pages(&root, &HashSet::new()), 0);
        assert_eq!(prune_stale_media(&root, &HashSet::new()), 0);
    }
}
