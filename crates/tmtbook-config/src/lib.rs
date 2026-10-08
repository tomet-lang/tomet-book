use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookConfig {
    #[serde(default)]
    pub book: BookMeta,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub build: BuildConfig,
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
    #[serde(default)]
    pub markers: MarkersConfig,
    #[serde(default)]
    pub links: LinksConfig,
}

impl UiConfig {
    /// Restore any string or label the user's config did not mention.
    pub fn fill_defaults(
        &mut self,
        default_strings: impl IntoIterator<Item = (String, String)>,
        default_labels: impl IntoIterator<Item = (String, String)>,
        default_chips: impl IntoIterator<Item = HeroChipConfig>,
    ) {
        for (key, value) in default_strings {
            self.strings.entry(key).or_insert(value);
        }
        for (key, value) in default_labels {
            self.infobox.labels.entry(key).or_insert(value);
        }
        if self.hero_chips.is_empty() {
            self.hero_chips.extend(default_chips);
        }
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
            markers: MarkersConfig::default(),
            links: LinksConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarkersConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default = "default_true")]
    pub strikethrough: bool,
    #[serde(default)]
    pub custom: HashMap<String, MarkerEntryConfig>,
}

impl Default for MarkersConfig {
    fn default() -> Self {
        Self {
            enable: false,
            strikethrough: default_true(),
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MarkerEntryConfig {
    Simple(String),
    Detailed {
        icon: String,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        pkg: Option<String>,
    },
}

impl MarkerEntryConfig {
    pub fn icon(&self) -> &str {
        match self {
            Self::Simple(name) => name,
            Self::Detailed { icon, .. } => icon,
        }
    }

    pub fn color(&self) -> Option<&str> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { color, .. } => color.as_deref(),
        }
    }

    pub fn pkg(&self) -> &str {
        match self {
            Self::Simple(_) => "lucide",
            Self::Detailed { pkg, .. } => pkg.as_deref().unwrap_or("lucide"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinksConfig {
    #[serde(default = "default_true")]
    pub external_icons: bool,
    #[serde(default = "default_true", alias = "internal_icons")]
    pub note_icons: bool,
    #[serde(default)]
    pub favicon_service: FaviconService,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            external_icons: default_true(),
            note_icons: default_true(),
            favicon_service: FaviconService::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FaviconService {
    #[default]
    Google,
    DuckDuckGo,
    None,
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

impl Default for InfoboxConfig {
    fn default() -> Self {
        Self {
            enable: true,
            exclude_keys: default_exclude_keys(),
            labels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RoutingStrategy {
    #[default]
    Hierarchical,
    Flat,
    Id,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CollisionStrategy {
    #[default]
    Error,
    Disambiguate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchEngine {
    #[default]
    Native,
    Pagefind,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
    /// Which `@kind`s reach the site, as an ordered list where the last
    /// matching rule wins: `"*"` takes everything, `"!config"` puts one back.
    #[serde(default = "default_kinds")]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub search: SearchEngine,
    #[serde(default, alias = "base")]
    pub base_path: Option<String>,
    #[serde(default = "default_url_prefix")]
    pub url_prefix: String,
    #[serde(default = "default_asset_prefix")]
    pub asset_prefix: String,
    #[serde(default)]
    pub routing: RoutingStrategy,
    #[serde(default)]
    pub on_collision: CollisionStrategy,
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
    pub fn clean_base_path(&self) -> &str {
        match self.base_path.as_deref() {
            Some(base) => {
                let trimmed = base.trim_matches('/');
                if trimmed.is_empty() {
                    ""
                } else {
                    let s = base.trim_end_matches('/');
                    if s.starts_with('/') { s } else { base }
                }
            }
            None => "",
        }
    }

    pub fn full_url_prefix(&self) -> String {
        let base = self.clean_base_path();
        let prefix = self.clean_url_prefix();
        if base.is_empty() {
            prefix.to_string()
        } else if prefix.is_empty() {
            base.to_string()
        } else {
            format!("{base}{prefix}")
        }
    }

    pub fn full_asset_prefix(&self) -> String {
        let base = self.clean_base_path();
        let prefix = self.clean_asset_prefix();
        if base.is_empty() {
            prefix.to_string()
        } else if prefix.is_empty() {
            base.to_string()
        } else {
            format!("{base}{prefix}")
        }
    }

    pub fn clean_url_prefix(&self) -> &str {
        self.url_prefix.trim_end_matches('/')
    }

    pub fn clean_asset_prefix(&self) -> &str {
        self.asset_prefix.trim_end_matches('/')
    }

    pub fn wiki_out_rel(&self) -> &str {
        self.url_prefix.trim_matches('/')
    }

    pub fn asset_out_rel(&self) -> &str {
        self.asset_prefix.trim_matches('/')
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            exclude: default_exclude(),
            kinds: default_kinds(),
            search: SearchEngine::default(),
            base_path: None,
            url_prefix: default_url_prefix(),
            asset_prefix: default_asset_prefix(),
            routing: RoutingStrategy::default(),
            on_collision: CollisionStrategy::default(),
            config_path: None,
        }
    }
}

impl BookConfig {
    pub fn load_from_str(content: &str) -> Result<Self> {
        toml::from_str(content).context("Failed to parse TOML configuration")
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

    #[test]
    fn base_path_and_base_alias_configuration() {
        let def = BuildConfig::default();
        assert_eq!(def.base_path, None);
        assert_eq!(def.clean_base_path(), "");
        assert_eq!(def.full_url_prefix(), "/wiki");
        assert_eq!(def.full_asset_prefix(), "/vault");

        let custom: BookConfig = toml::from_str(
            r#"
[build]
base_path = "/docs"
"#,
        )
        .unwrap();
        assert_eq!(custom.build.base_path.as_deref(), Some("/docs"));
        assert_eq!(custom.build.clean_base_path(), "/docs");
        assert_eq!(custom.build.full_url_prefix(), "/docs/wiki");
        assert_eq!(custom.build.full_asset_prefix(), "/docs/vault");
    }
}
