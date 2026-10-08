use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::LazyLock;

pub mod i18n;

use tmtbook_config::BookConfig;

// Stylesheets
pub const BOOK_CSS: &str = concat!(
    include_str!("../frontend/styles/00-base.css"),
    include_str!("../frontend/styles/10-shell.css"),
    include_str!("../frontend/styles/20-panes.css"),
    include_str!("../frontend/styles/30-sticky-tabs.css"),
    include_str!("../frontend/styles/35-sticky-tabs-vertical.css"),
    include_str!("../frontend/styles/40-hero.css"),
    include_str!("../frontend/styles/50-nav-infobox.css"),
    include_str!("../frontend/styles/60-content.css"),
    include_str!("../frontend/styles/70-widgets.css"),
    include_str!("../frontend/styles/71-source-viewer.css"),
    include_str!("../frontend/styles/72-center-peek.css"),
    include_str!(concat!(env!("OUT_DIR"), "/tailwind.css")),
    include_str!("../frontend/styles/90-responsive.css"),
    include_str!("../frontend/styles/98-no-js.css"),
);

pub const BOOK_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/tmtbook.js"));

pub const LUCIDE_SPRITE: &str = include_str!("../frontend/icons/lucide/sprite.svg");
pub const SIMPLE_SPRITE: &str = include_str!("../frontend/icons/simple/sprite.svg");

pub const TMT_BTN_CSS: &str = include_str!("../frontend/styles/components/tmt-btn.css");
pub const TMT_BADGE_CSS: &str = include_str!("../frontend/styles/components/tmt-badge.css");
pub const TMT_ICON_CSS: &str = include_str!("../frontend/styles/components/tmt-icon.css");
pub const TMT_SWATCH_CSS: &str = include_str!("../frontend/styles/components/tmt-swatch.css");
pub const TMT_SWITCH_CSS: &str = include_str!("../frontend/styles/components/tmt-switch.css");

// Templates
pub const BASE_HTML: &str = include_str!("../frontend/templates/base.html");
pub const PAGE_HTML: &str = include_str!("../frontend/templates/page.html");
pub const INDEX_HTML: &str = include_str!("../frontend/templates/index.html");
pub const MACROS_HTML: &str = include_str!("../frontend/templates/macros.html");

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

pub fn write_if_changed(path: &Path, content: &str) -> Result<bool> {
    if path.exists() {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                return Ok(false);
            }
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(true)
}

pub fn write_static_assets(out_dir: &Path, src_dir: &Path, config: &BookConfig) -> Result<()> {
    fs::create_dir_all(out_dir)?;

    write_if_changed(&out_dir.join("tmtbook.css"), BOOK_CSS)
        .context("Failed to write tmtbook.css")?;
    write_if_changed(&out_dir.join("tmtbook.js"), BOOK_JS).context("Failed to write tmtbook.js")?;

    write_if_changed(&out_dir.join("icons/lucide.svg"), LUCIDE_SPRITE)
        .context("Failed to write icons/lucide.svg")?;
    write_if_changed(&out_dir.join("icons/simple.svg"), SIMPLE_SPRITE)
        .context("Failed to write icons/simple.svg")?;

    write_if_changed(&out_dir.join("components/tmt-btn.css"), TMT_BTN_CSS)
        .context("Failed to write components/tmt-btn.css")?;
    write_if_changed(&out_dir.join("components/tmt-badge.css"), TMT_BADGE_CSS)
        .context("Failed to write components/tmt-badge.css")?;
    write_if_changed(&out_dir.join("components/tmt-icon.css"), TMT_ICON_CSS)
        .context("Failed to write components/tmt-icon.css")?;
    write_if_changed(&out_dir.join("components/tmt-swatch.css"), TMT_SWATCH_CSS)
        .context("Failed to write components/tmt-swatch.css")?;
    write_if_changed(&out_dir.join("components/tmt-switch.css"), TMT_SWITCH_CSS)
        .context("Failed to write components/tmt-switch.css")?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_versions_are_valid() {
        assert_eq!(CSS_VERSION.len(), 16);
        assert_eq!(JS_VERSION.len(), 16);
    }
}
