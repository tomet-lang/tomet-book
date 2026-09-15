use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    #[serde(default)]
    pub book: BookMeta,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub build: BuildConfig,
}

impl Default for BookConfig {
    fn default() -> Self {
        Self::default_with_lang("ja")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMeta {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Where the built site goes, relative to the vault.
    #[serde(default = "default_dest")]
    pub dest: PathBuf,
}

fn default_title() -> String {
    "Tomet Book".to_string()
}
fn default_lang() -> String {
    "ja".to_string()
}
fn default_dest() -> PathBuf {
    PathBuf::from("dist")
}

impl Default for BookMeta {
    fn default() -> Self {
        Self {
            title: default_title(),
            description: None,
            lang: default_lang(),
            dest: default_dest(),
        }
    }
}

pub use crate::i18n::{
    default_hero_chips_for, default_key_labels_for, default_ui_strings_for, is_english,
};

pub fn default_ui_strings() -> HashMap<String, String> {
    crate::i18n::ja::ui_strings()
}

pub fn default_hero_chips() -> Vec<HeroChipConfig> {
    crate::i18n::ja::hero_chips()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub rail_title: Option<String>,
    #[serde(default = "default_view")]
    pub default_view: String,
    #[serde(default)]
    pub custom_css: Vec<String>,
    #[serde(default)]
    pub hero_chips: Vec<HeroChipConfig>,
    #[serde(default)]
    pub infobox: InfoboxConfig,
    /// Every piece of text the interface shows, keyed by name.
    ///
    /// Defaults are chosen based on `[book] lang` ("ja" or "en");
    /// `[ui.strings]` in `tmtbook.toml` overrides any subset of them, and
    /// anything left out keeps its default.
    #[serde(default)]
    pub strings: HashMap<String, String>,
}

impl UiConfig {
    /// Restore any string or label the user's config did not mention, using defaults for `lang`.
    ///
    /// A partial `[ui.strings]` or `[ui.infobox.labels]` table replaces the whole map during
    /// deserialization, so without this an override of one label would blank out the rest.
    pub fn fill_defaults(&mut self, lang: &str) {
        for (key, value) in default_ui_strings_for(lang) {
            self.strings.entry(key).or_insert(value);
        }
        for (key, value) in default_key_labels_for(lang) {
            self.infobox.labels.entry(key).or_insert(value);
        }
        if self.hero_chips.is_empty() {
            self.hero_chips = default_hero_chips_for(lang);
        }
    }

    /// Restore defaults using the default Japanese dictionary.
    pub fn fill_string_defaults(&mut self) {
        self.fill_defaults("ja");
    }
}

fn default_view() -> String {
    "book".to_string()
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            rail_title: None,
            default_view: default_view(),
            custom_css: Vec::new(),
            hero_chips: Vec::new(),
            infobox: InfoboxConfig::default(),
            strings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroChipConfig {
    pub key: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoboxConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_exclude_keys")]
    pub exclude_keys: Vec<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

fn default_exclude_keys() -> Vec<String> {
    vec![
        "id".to_string(),
        "banner".to_string(),
        "banner-y".to_string(),
        "images".to_string(),
        "icon".to_string(),
    ]
}

pub fn default_key_labels() -> HashMap<String, String> {
    crate::i18n::ja::key_labels()
}

impl Default for InfoboxConfig {
    fn default() -> Self {
        Self {
            enable: true,
            exclude_keys: default_exclude_keys(),
            labels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
    /// Which `@kind`s reach the site, as an ordered list where the last
    /// matching rule wins: `"*"` takes everything, `"!config"` puts one back.
    /// See `book::kinds`.
    #[serde(default = "default_kinds")]
    pub kinds: Vec<String>,
    #[serde(default = "default_true")]
    pub pagefind: bool,
    #[serde(default = "default_url_prefix")]
    pub url_prefix: String,
    #[serde(default = "default_asset_prefix")]
    pub asset_prefix: String,
    #[serde(default)]
    pub config_path: Option<PathBuf>,
}

fn default_exclude() -> Vec<String> {
    vec![
        "00-09 System/01 Apps".to_string(),
        ".git".to_string(),
        ".tomet".to_string(),
        ".web".to_string(),
        "node_modules".to_string(),
        "dist".to_string(),
        "target".to_string(),
    ]
}

/// Everything except the documents that shape the book rather than belong to
/// it. Spelled as a value so that `*.config.tmt` not appearing on the site is
/// a setting the reader can see and change, not a rule buried in the code.
fn default_kinds() -> Vec<String> {
    vec!["*".to_string(), "!config".to_string(), "!index".to_string()]
}

fn default_url_prefix() -> String {
    "/wiki".to_string()
}

fn default_asset_prefix() -> String {
    "/vault".to_string()
}

impl BuildConfig {
    /// `url_prefix` without a trailing slash, for building page hrefs (`/wiki`).
    pub fn clean_url_prefix(&self) -> &str {
        self.url_prefix.trim_end_matches('/')
    }

    /// `asset_prefix` without a trailing slash, for building media hrefs (`/vault`).
    pub fn clean_asset_prefix(&self) -> &str {
        self.asset_prefix.trim_end_matches('/')
    }

    /// Where rendered pages go, relative to the destination root.
    ///
    /// Derived from `url_prefix` so the files always land where the links point;
    /// an empty result means the pages sit at the destination root.
    pub fn wiki_out_rel(&self) -> &str {
        self.url_prefix.trim_matches('/')
    }

    /// Where vault media goes, relative to the destination root.
    pub fn asset_out_rel(&self) -> &str {
        self.asset_prefix.trim_matches('/')
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            exclude: default_exclude(),
            kinds: default_kinds(),
            pagefind: true,
            url_prefix: default_url_prefix(),
            asset_prefix: default_asset_prefix(),
            config_path: None,
        }
    }
}

impl BookConfig {
    pub fn load_from_str(content: &str) -> Result<Self> {
        let mut cfg: BookConfig =
            toml::from_str(content).context("Failed to parse TOML configuration")?;
        cfg.apply_defaults();
        Ok(cfg)
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let config_path = dir.join("tmtbook.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            Self::load_from_str(&content)
                .with_context(|| format!("Failed to parse {}", config_path.display()))
        } else {
            Ok(BookConfig::default())
        }
    }

    pub fn default_with_lang(lang: &str) -> Self {
        Self {
            book: BookMeta {
                lang: lang.to_string(),
                ..Default::default()
            },
            ui: UiConfig {
                strings: default_ui_strings_for(lang),
                hero_chips: default_hero_chips_for(lang),
                infobox: InfoboxConfig {
                    labels: default_key_labels_for(lang),
                    ..Default::default()
                },
                ..Default::default()
            },
            build: BuildConfig::default(),
        }
    }

    pub fn apply_defaults(&mut self) {
        self.ui.fill_defaults(&self.book.lang);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefixes(url: &str, asset: &str) -> BuildConfig {
        BuildConfig {
            url_prefix: url.to_string(),
            asset_prefix: asset.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn output_dirs_are_derived_from_the_configured_prefixes() {
        let cfg = prefixes("/docs", "/media");
        assert_eq!(cfg.wiki_out_rel(), "docs");
        assert_eq!(cfg.asset_out_rel(), "media");
        assert_eq!(cfg.clean_url_prefix(), "/docs");
        assert_eq!(cfg.clean_asset_prefix(), "/media");
    }

    #[test]
    fn the_defaults_still_land_in_wiki_and_vault() {
        let cfg = BuildConfig::default();
        assert_eq!(cfg.wiki_out_rel(), "wiki");
        assert_eq!(cfg.asset_out_rel(), "vault");
    }

    #[test]
    fn surrounding_slashes_are_normalized() {
        let cfg = prefixes("/docs/", "media/");
        assert_eq!(cfg.wiki_out_rel(), "docs");
        assert_eq!(cfg.asset_out_rel(), "media");
        assert_eq!(cfg.clean_url_prefix(), "/docs");
    }

    #[test]
    fn nested_prefixes_keep_their_depth() {
        let cfg = prefixes("/en/wiki", "/en/vault");
        assert_eq!(cfg.wiki_out_rel(), "en/wiki");
        assert_eq!(cfg.asset_out_rel(), "en/vault");
    }

    #[test]
    fn a_root_prefix_means_the_destination_root() {
        let cfg = prefixes("/", "/");
        assert_eq!(cfg.wiki_out_rel(), "");
        assert_eq!(cfg.clean_url_prefix(), "");
    }
}

#[cfg(test)]
mod string_tests {
    use super::*;

    fn load(toml_src: &str) -> BookConfig {
        BookConfig::load_from_str(toml_src).unwrap()
    }

    #[test]
    fn the_defaults_are_complete_without_a_config_file() {
        let cfg = BookConfig::default();
        assert_eq!(
            cfg.ui.strings.get("pane.toc").map(String::as_str),
            Some("目次")
        );
        assert!(cfg.ui.strings.contains_key("search.placeholder"));
    }

    #[test]
    fn a_config_without_a_strings_table_keeps_every_default() {
        let cfg = load("[book]\ntitle = \"X\"\n");
        assert_eq!(cfg.ui.strings.len(), default_ui_strings().len());
    }

    #[test]
    fn overriding_one_string_leaves_the_rest_intact() {
        // A partial table replaces the whole map during deserialization, so
        // this is the case that would blank out the interface.
        let cfg = load("[ui.strings]\n\"pane.toc\" = \"Contents\"\n");

        assert_eq!(
            cfg.ui.strings.get("pane.toc").map(String::as_str),
            Some("Contents")
        );
        assert_eq!(
            cfg.ui.strings.get("pane.links").map(String::as_str),
            Some("リンク"),
            "untouched strings must survive"
        );
        assert_eq!(cfg.ui.strings.len(), default_ui_strings().len());
    }

    #[test]
    fn an_unknown_string_key_is_kept_rather_than_dropped() {
        let cfg = load("[ui.strings]\n\"custom.key\" = \"mine\"\n");
        assert_eq!(
            cfg.ui.strings.get("custom.key").map(String::as_str),
            Some("mine")
        );
    }

    #[test]
    fn english_language_loads_english_defaults() {
        let cfg = load("[book]\ntitle = \"My Book\"\nlang = \"en\"\n");
        assert_eq!(
            cfg.ui.strings.get("pane.toc").map(String::as_str),
            Some("Table of Contents")
        );
        assert_eq!(
            cfg.ui.strings.get("recent.heading").map(String::as_str),
            Some("Recent Notes")
        );
        assert_eq!(
            cfg.ui.infobox.labels.get("birth").map(String::as_str),
            Some("Birthday")
        );
        assert_eq!(cfg.ui.hero_chips[0].label.as_deref(), Some("Affiliation"));
    }

    #[test]
    fn english_language_with_user_overrides() {
        let cfg =
            load("[book]\nlang = \"en\"\n\n[ui.strings]\n\"pane.toc\" = \"Custom Outline\"\n");
        assert_eq!(
            cfg.ui.strings.get("pane.toc").map(String::as_str),
            Some("Custom Outline")
        );
        assert_eq!(
            cfg.ui.strings.get("pane.links").map(String::as_str),
            Some("Links")
        );
        assert_eq!(cfg.ui.strings.len(), crate::i18n::en::ui_strings().len());
    }
}
