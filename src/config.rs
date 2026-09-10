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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMeta {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_src")]
    pub src: PathBuf,
    #[serde(default = "default_dest")]
    pub dest: PathBuf,
}

fn default_title() -> String {
    "Tomet Book".to_string()
}
fn default_lang() -> String {
    "ja".to_string()
}
fn default_src() -> PathBuf {
    PathBuf::from(".")
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
            src: default_src(),
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
    #[serde(default = "default_hero_chips")]
    pub hero_chips: Vec<HeroChipConfig>,
    #[serde(default)]
    pub infobox: InfoboxConfig,
}

fn default_view() -> String {
    "book".to_string()
}

fn default_hero_chips() -> Vec<HeroChipConfig> {
    vec![
        HeroChipConfig {
            key: "parent".to_string(),
            label: Some("所属".to_string()),
            format: None,
        },
        HeroChipConfig {
            key: "type.list-a".to_string(),
            label: Some("ファン".to_string()),
            format: None,
        },
        HeroChipConfig {
            key: "type.element-a".to_string(),
            label: None,
            format: None,
        },
        HeroChipConfig {
            key: "birth".to_string(),
            label: Some("🎂".to_string()),
            format: Some("birthday".to_string()),
        },
    ]
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            rail_title: None,
            default_view: default_view(),
            custom_css: Vec::new(),
            hero_chips: default_hero_chips(),
            infobox: InfoboxConfig::default(),
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
    #[serde(default = "default_key_labels")]
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

fn default_key_labels() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("kind".to_string(), "種別".to_string());
    m.insert("type".to_string(), "種別".to_string());
    m.insert("birth".to_string(), "誕生日".to_string());
    m.insert("height".to_string(), "身長".to_string());
    m.insert("creator".to_string(), "デザイン".to_string());
    m.insert("type.list-a".to_string(), "ファンネーム".to_string());
    m.insert("type.element-a".to_string(), "属性 / MBTI".to_string());
    m.insert("colors".to_string(), "カラー".to_string());
    m.insert("parent".to_string(), "所属ユニット".to_string());
    m.insert("group".to_string(), "所属組織".to_string());
    m.insert("place".to_string(), "出身地".to_string());
    m.insert("created".to_string(), "作成日".to_string());
    m.insert("modified".to_string(), "更新日".to_string());
    m.insert("reviewed".to_string(), "確認日".to_string());
    m.insert("rating".to_string(), "評価".to_string());
    m.insert("aliases".to_string(), "別名 / 愛称".to_string());
    m.insert("topics".to_string(), "トピック".to_string());
    m.insert("flags".to_string(), "フラグ".to_string());
    m.insert("published".to_string(), "公開日".to_string());
    m.insert("url.wiki".to_string(), "Wikipedia".to_string());
    m.insert("url.else".to_string(), "関連リンク".to_string());
    m
}

impl Default for InfoboxConfig {
    fn default() -> Self {
        Self {
            enable: true,
            exclude_keys: default_exclude_keys(),
            labels: default_key_labels(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
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

fn default_url_prefix() -> String {
    "/wiki".to_string()
}

fn default_asset_prefix() -> String {
    "/vault".to_string()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            exclude: default_exclude(),
            pagefind: true,
            url_prefix: default_url_prefix(),
            asset_prefix: default_asset_prefix(),
            config_path: None,
        }
    }
}

impl Default for BookConfig {
    fn default() -> Self {
        Self {
            book: BookMeta::default(),
            ui: UiConfig::default(),
            build: BuildConfig::default(),
        }
    }
}

impl BookConfig {
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let config_path = dir.join("tmtbook.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            let cfg: BookConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", config_path.display()))?;
            Ok(cfg)
        } else {
            Ok(BookConfig::default())
        }
    }
}
