//! Manifest tracking of output build artifacts for incremental pruning.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::renderer::SectionSummary;
use super::write_if_changed;

pub(crate) const MANIFEST_FILE: &str = ".tmtbook-manifest.json";

/// What the previous build put in the output directory.
///
/// Lets the next build find what to delete by set difference, instead of
/// walking the whole site looking for orphans.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct BuildManifest {
    /// Page slugs, relative to the wiki directory.
    pub(crate) pages: Vec<String>,
    /// Media paths, relative to the asset directory.
    pub(crate) media: Vec<String>,
}

/// `lookup/manifest.json`: the section names Search's tag chips list.
#[derive(Debug, Serialize)]
pub(crate) struct LookupManifest {
    pub(crate) sections: Vec<SectionSummary>,
}

impl BuildManifest {
    pub(crate) fn read(out_dir: &Path) -> Option<Self> {
        let raw = fs::read_to_string(out_dir.join(MANIFEST_FILE)).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub(crate) fn write(&self, out_dir: &Path) -> Result<()> {
        let raw = serde_json::to_string(self)?;
        write_if_changed(&out_dir.join(MANIFEST_FILE), &raw)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tmtbook-manifest-{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_manifest_round_trips() {
        let dir = scratch("roundtrip");
        let manifest = BuildManifest {
            pages: vec!["a".into(), "notes/b".into()],
            media: vec!["img/c.png".into()],
        };
        manifest.write(&dir).unwrap();

        let read = BuildManifest::read(&dir).expect("manifest should be readable");
        assert_eq!(read.pages, manifest.pages);
        assert_eq!(read.media, manifest.media);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_corrupt_manifest_reads_as_none() {
        let dir = scratch("missing");
        assert!(BuildManifest::read(&dir).is_none());

        fs::write(dir.join(MANIFEST_FILE), "not json").unwrap();
        assert!(BuildManifest::read(&dir).is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
