use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use super::document::ProcessedDoc;
use super::DocFailure;
use crate::config::BookConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImagePreset {
    /// Banners displayed across wide screens (max width: 1600px).
    Banner,
    /// Profile / infobox images (max width: 640px for 2x Retina on ~300px display).
    Profile,
    /// Small thumbnails for hover preview cards and index popovers (max width: 240px).
    Thumb,
}

impl ImagePreset {
    pub fn max_width(&self) -> u32 {
        match self {
            Self::Banner => 1600,
            Self::Profile => 640,
            Self::Thumb => 240,
        }
    }

    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Banner => "banners",
            Self::Profile => "profiles",
            Self::Thumb => "thumbs",
        }
    }
}

/// Optimizes an image from `src_abs` into `out_dir/cache/<preset>/<rel_path_with_webp_ext>`.
///
/// Returns the public URL path (e.g. `/cache/banners/images/hero.webp`),
/// or `None` if the file could not be decoded as an image.
pub fn optimize_image(
    src_abs: &Path,
    rel_path: &Path,
    preset: ImagePreset,
    out_dir: &Path,
) -> Result<Option<String>> {
    // Determine the destination path under dist/cache/<preset>/<rel_path_stem>.webp
    let cache_dir = out_dir.join("cache").join(preset.dir_name());
    let webp_rel = rel_path.with_extension("webp");
    let dest_abs = cache_dir.join(&webp_rel);

    let url = format!(
        "/cache/{}/{}",
        preset.dir_name(),
        webp_rel.to_string_lossy().replace('\\', "/")
    );

    // Cache check: if dest exists and was modified after or at src mtime, skip processing
    if dest_abs.exists()
        && let (Ok(src_meta), Ok(dest_meta)) = (fs::metadata(src_abs), fs::metadata(&dest_abs))
        && let (Ok(src_mtime), Ok(dest_mtime)) = (src_meta.modified(), dest_meta.modified())
        && dest_mtime >= src_mtime
    {
        debug!("Image cache hit: {}", dest_abs.display());
        return Ok(Some(url));
    }

    // Attempt to decode the source image
    let img = match image::open(src_abs) {
        Ok(img) => img,
        Err(e) => {
            debug!(
                "Could not decode image {} for optimization (falling back to original): {e}",
                src_abs.display()
            );
            return Ok(None);
        }
    };

    let max_w = preset.max_width();
    let optimized = if img.width() > max_w {
        img.resize(max_w, u32::MAX, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    if let Some(parent) = dest_abs.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // Save as WebP
    optimized
        .save_with_format(&dest_abs, image::ImageFormat::WebP)
        .with_context(|| format!("Failed to save optimized WebP image to {}", dest_abs.display()))?;

    debug!("Optimized image: {} -> {}", src_abs.display(), dest_abs.display());
    Ok(Some(url))
}

/// Resolves a media URL from a document to its relative and absolute paths in `src_dir`.
/// Returns `Some((rel_path, abs_path))` if the file exists locally in `src_dir`.
pub fn resolve_local_media_path(
    url: &str,
    src_dir: &Path,
    asset_prefix: &str,
) -> Option<(PathBuf, PathBuf)> {
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("//") {
        return None;
    }

    let prefix = asset_prefix.trim_end_matches('/');

    // 1. Strip asset prefix if present (e.g. "/vault/foo.png" -> "foo.png")
    if let Some(rel) = url.strip_prefix(prefix) {
        let rel_clean = rel.trim_start_matches('/');
        let rel_path = PathBuf::from(rel_clean);
        let src_abs = src_dir.join(&rel_path);
        if src_abs.is_file() {
            return Some((rel_path, src_abs));
        }
    }

    // 2. Direct relative / root-relative path (e.g. "/foo.png" or "foo.png")
    let rel_clean = url.trim_start_matches('/');
    let rel_path = PathBuf::from(rel_clean);
    let src_abs = src_dir.join(&rel_path);
    if src_abs.is_file() {
        return Some((rel_path, src_abs));
    }

    None
}

/// Optimizes referenced banners and profile images across all documents in parallel.
pub fn optimize_docs_media(
    docs: &mut [Result<ProcessedDoc, DocFailure>],
    src_dir: &Path,
    out_dir: &Path,
    config: &BookConfig,
) {
    let asset_prefix = config.build.clean_asset_prefix();

    // 1. Collect unique targets across all published documents
    let mut targets: HashSet<(ImagePreset, PathBuf, PathBuf)> = HashSet::new();

    for result in docs.iter() {
        let Ok(doc) = result else { continue };

        if let Some(banner) = &doc.banner_url
            && let Some((rel, abs)) = resolve_local_media_path(banner, src_dir, asset_prefix)
        {
            targets.insert((ImagePreset::Banner, rel, abs));
        }

        for img in &doc.images {
            if let Some((rel, abs)) = resolve_local_media_path(img, src_dir, asset_prefix) {
                targets.insert((ImagePreset::Profile, rel, abs));
            }
        }
    }

    if targets.is_empty() {
        return;
    }

    // 2. Optimize images concurrently using rayon
    let optimized_map: HashMap<(ImagePreset, PathBuf), String> = targets
        .into_par_iter()
        .filter_map(|(preset, rel_path, src_abs)| {
            match optimize_image(&src_abs, &rel_path, preset, out_dir) {
                Ok(Some(url)) => Some(((preset, rel_path), url)),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!("Failed to optimize image {}: {e}", src_abs.display());
                    None
                }
            }
        })
        .collect();

    // 3. Update doc URLs to point to cached WebP images
    for result in docs.iter_mut() {
        let Ok(doc) = result else { continue };

        if let Some(banner) = &doc.banner_url
            && let Some((rel, _)) = resolve_local_media_path(banner, src_dir, asset_prefix)
            && let Some(cached_url) = optimized_map.get(&(ImagePreset::Banner, rel))
        {
            doc.banner_url = Some(cached_url.clone());
        }

        for img in &mut doc.images {
            if let Some((rel, _)) = resolve_local_media_path(img, src_dir, asset_prefix)
                && let Some(cached_url) = optimized_map.get(&(ImagePreset::Profile, rel))
            {
                *img = cached_url.clone();
            }
        }
    }
}

/// Optimizes media for a single document during incremental builds.
pub fn optimize_single_doc_media(
    doc: &mut ProcessedDoc,
    src_dir: &Path,
    out_dir: &Path,
    config: &BookConfig,
) {
    let asset_prefix = config.build.clean_asset_prefix();

    if let Some(banner) = &doc.banner_url
        && let Some((rel, abs)) = resolve_local_media_path(banner, src_dir, asset_prefix)
    {
        match optimize_image(&abs, &rel, ImagePreset::Banner, out_dir) {
            Ok(Some(cached_url)) => {
                doc.banner_url = Some(cached_url);
            }
            Ok(None) => {}
            Err(e) => tracing::warn!("Failed to optimize banner {}: {e}", abs.display()),
        }
    }

    for img in &mut doc.images {
        if let Some((rel, abs)) = resolve_local_media_path(img, src_dir, asset_prefix) {
            match optimize_image(&abs, &rel, ImagePreset::Profile, out_dir) {
                Ok(Some(cached_url)) => {
                    *img = cached_url;
                }
                Ok(None) => {}
                Err(e) => tracing::warn!("Failed to optimize profile image {}: {e}", abs.display()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn optimizes_image_to_webp_within_max_width() {
        let tmp = std::env::temp_dir().join(format!("tmtbook-img-opt-test-{}", std::process::id()));
        let src_dir = tmp.join("src");
        let out_dir = tmp.join("dist");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&out_dir).unwrap();

        // Create a test 2000x1000 PNG image
        let img_path = src_dir.join("wide.png");
        let mut img = RgbImage::new(2000, 1000);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([100, 150, 200]);
        }
        img.save(&img_path).unwrap();

        // Optimize as Profile (max width 640)
        let rel_path = Path::new("wide.png");
        let url = optimize_image(&img_path, rel_path, ImagePreset::Profile, &out_dir)
            .unwrap()
            .expect("image should be optimized");

        assert_eq!(url, "/cache/profiles/wide.webp");

        let dest = out_dir.join("cache/profiles/wide.webp");
        assert!(dest.exists());

        // Check output dimensions
        let decoded = image::open(&dest).unwrap();
        assert_eq!(decoded.width(), 640);
        assert_eq!(decoded.height(), 320);

        // Second call should hit cache
        let url2 = optimize_image(&img_path, rel_path, ImagePreset::Profile, &out_dir)
            .unwrap()
            .unwrap();
        assert_eq!(url2, url);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolves_local_media_path_with_or_without_prefix() {
        let tmp = std::env::temp_dir().join(format!("tmtbook-img-res-test-{}", std::process::id()));
        let src_dir = tmp.join("vault");
        fs::create_dir_all(src_dir.join("images")).unwrap();
        fs::write(src_dir.join("images/hero.png"), b"fake").unwrap();

        // With asset prefix "/vault"
        let res = resolve_local_media_path("/vault/images/hero.png", &src_dir, "/vault");
        assert!(res.is_some());
        let (rel, abs) = res.unwrap();
        assert_eq!(rel, PathBuf::from("images/hero.png"));
        assert_eq!(abs, src_dir.join("images/hero.png"));

        // Without asset prefix "/images/hero.png"
        let res2 = resolve_local_media_path("/images/hero.png", &src_dir, "/vault");
        assert!(res2.is_some());

        // External URL returns None
        assert!(resolve_local_media_path("https://example.com/banner.png", &src_dir, "/vault").is_none());

        // Non-existent file returns None
        assert!(resolve_local_media_path("/vault/images/missing.png", &src_dir, "/vault").is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn optimizes_docs_media_in_batch() {
        let tmp = std::env::temp_dir().join(format!("tmtbook-img-batch-test-{}", std::process::id()));
        let src_dir = tmp.join("vault");
        let out_dir = tmp.join("dist");
        fs::create_dir_all(src_dir.join("images")).unwrap();
        fs::create_dir_all(&out_dir).unwrap();

        // Create sample images
        let mut banner_img = RgbImage::new(1800, 400);
        for p in banner_img.pixels_mut() {
            *p = Rgb([10, 20, 30]);
        }
        banner_img.save(src_dir.join("images/banner.png")).unwrap();

        let mut prof_img = RgbImage::new(800, 800);
        for p in prof_img.pixels_mut() {
            *p = Rgb([40, 50, 60]);
        }
        prof_img.save(src_dir.join("images/profile.png")).unwrap();

        let doc = ProcessedDoc {
            slug: "alice".to_string(),
            rel_path: "alice.tmt".to_string(),
            source_path: "/vault/alice.tmt".to_string(),
            title: "Alice".to_string(),
            section: None,
            kind: None,
            primary_color: None,
            icon: None,
            banner_url: Some("/vault/images/banner.png".to_string()),
            banner_original_url: Some("/vault/images/banner.png".to_string()),
            banner_y: None,
            images: vec!["/vault/images/profile.png".to_string()],
            original_images: vec!["/vault/images/profile.png".to_string()],
            hero_chips: Vec::new(),
            infobox_rows: Vec::new(),
            has_data: true,
            toc: Vec::new(),
            section_tabs: Vec::new(),
            body_html: String::new(),
            outgoing: Vec::new(),
        };

        let mut docs = vec![Ok(doc)];
        let config = BookConfig::default();

        optimize_docs_media(&mut docs, &src_dir, &out_dir, &config);

        let optimized_doc = docs.remove(0).unwrap();
        assert_eq!(
            optimized_doc.banner_url,
            Some("/cache/banners/images/banner.webp".to_string())
        );
        assert_eq!(
            optimized_doc.banner_original_url,
            Some("/vault/images/banner.png".to_string())
        );
        assert_eq!(
            optimized_doc.images,
            vec!["/cache/profiles/images/profile.webp".to_string()]
        );
        assert_eq!(
            optimized_doc.original_images,
            vec!["/vault/images/profile.png".to_string()]
        );

        assert!(out_dir.join("cache/banners/images/banner.webp").exists());
        assert!(out_dir.join("cache/profiles/images/profile.webp").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
