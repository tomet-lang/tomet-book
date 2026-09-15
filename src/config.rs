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

pub fn is_english(lang: &str) -> bool {
    lang.trim().to_ascii_lowercase().starts_with("en")
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

/// Look up default UI strings for a given book language code.
pub fn default_ui_strings_for(lang: &str) -> HashMap<String, String> {
    if is_english(lang) {
        default_ui_strings_en()
    } else {
        default_ui_strings_ja()
    }
}

/// The interface's text in Japanese (default).
pub fn default_ui_strings_ja() -> HashMap<String, String> {
    [
        // Panes and navigation
        ("pane.toc", "目次"),
        ("pane.toc_short", "目次"),
        ("pane.links", "リンク"),
        ("pane.links_short", "リンク"),
        ("pane.graph", "グラフ"),
        ("pane.graph_short", "グラフ"),
        ("pane.index", "索引"),
        ("pane.index_short", "索引"),
        ("index.empty", "索引が書かれていません"),
        ("pane.lookup", "検索"),
        ("pane.lookup_short", "検索"),
        ("lookup.placeholder", "タイトルで絞り込み"),
        ("lookup.tags_all", "すべて"),
        ("pane.data", "データ・プロファイル"),
        ("pane.data_short", "データ"),
        ("pane.collapse", "クリックしてパネルを折りたたむ"),
        ("pane.data_expand", "クリックしてデータを展開"),
        ("pane.data_collapse", "クリックしてデータを折りたたむ"),
        ("nav.home", "← ホーム"),
        ("nav.home_title", "ホームへ"),
        ("nav.section", "分類:"),
        ("nav.bar", "ナビゲーションバー"),
        // Content chrome
        ("toc.heading", "目次"),
        ("recent.heading", "最近開いたノート"),
        ("backlinks.heading", "バックリンク"),
        ("backlinks.empty", "リンクしているノートはありません"),
        ("graph.heading", "ローカルグラフ"),
        ("graph.placeholder", "グラフ準備中"),
        ("sticky.label", "付箋見出し"),
        ("sticky.to_top", "ページの先頭へ"),
        ("sticky.to_bottom", "ページの末尾へ"),
        ("tabs.side", "付箋の位置を切り替え"),
        ("tabs.top", "⬒ 上"),
        ("tabs.top_title", "付箋を上に横並びで置く"),
        ("tabs.right", "◨ 右"),
        ("tabs.right_title", "付箋を右に縦並びで置く"),
        ("costume.label", "衣装:"),
        ("costume.item", "衣装"),
        ("image.zoom", "拡大"),
        ("image.zoom_banner", "バナーを拡大表示"),
        ("image.banner_alt", "バナー"),
        ("lightbox.close", "閉じる"),
        ("peek.fullscreen", "全画面で開く"),
        ("peek.close", "閉じる"),
        ("preview.open_peek", "クリックで開く"),
        ("peek.error", "ページの読み込みに失敗しました"),
        ("peek.not_found", "本文が見つかりませんでした"),
        // Catalog page
        ("index.sections", "セクション分類"),
        ("index.entries", "ノート一覧"),
        ("index.note_unit", "ノート"),
        // Controls
        ("view.switch", "表示モード切替"),
        ("view.book", "📖 本"),
        ("view.book_title", "本モード (マルチペイン)"),
        ("view.classic", "📄 縦"),
        ("view.classic_title", "縦スクロールモード"),
        ("drag.handle", "ドラッグして移動"),
        ("theme.toggle", "テーマ切り替え (ライト / ダーク)"),
        ("theme.toggle_label", "テーマ切り替え"),
        ("theme.to_light", "ライトテーマに切り替え"),
        ("theme.to_dark", "ダークテーマに切り替え"),
        ("theme.shell_toggle", "シェルテーマ切り替え"),
        ("theme.shell_label", "シェルテーマ"),
        ("theme.shell.slate", "スレート"),
        ("theme.shell.violet", "バイオレット"),
        ("theme.shell.indigo", "インディゴ"),
        ("theme.shell.olive", "オリーブ"),
        ("theme.shell.contrast", "コントラスト"),
        // Search
        ("search.placeholder", "検索... (Ctrl+K)"),
        ("search.loading", "検索インデックスを準備中..."),
        ("search.empty", "見つかりませんでした"),
        ("search.aliases", "別名:"),
        ("search.prev", "◀ 前へ"),
        ("search.next", "次へ ▶"),
        // Editing
        ("edit.open", "編集"),
        ("edit.editor", "エディタで開く"),
        ("edit.vscode", "VSCode で開く"),
        ("edit.cursor", "Cursor で開く"),
        ("edit.copy_path", "パスをコピー"),
        ("edit.copy_path_title", "ファイルパスをコピー"),
        ("toast.copied", "📋 パスをクリップボードにコピーしました"),
        ("toast.copy_failed", "❌ コピーに失敗しました"),
        // Links that point at a page nobody has written yet
        ("link.unresolved", "未作成のページ"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// The interface's text in English.
pub fn default_ui_strings_en() -> HashMap<String, String> {
    [
        // Panes and navigation
        ("pane.toc", "Table of Contents"),
        ("pane.toc_short", "TOC"),
        ("pane.links", "Links"),
        ("pane.links_short", "LINKS"),
        ("pane.graph", "Graph"),
        ("pane.graph_short", "GRAPH"),
        ("pane.index", "Index"),
        ("pane.index_short", "INDEX"),
        ("index.empty", "No index entries"),
        ("pane.lookup", "Search"),
        ("pane.lookup_short", "SEARCH"),
        ("lookup.placeholder", "Filter by title..."),
        ("lookup.tags_all", "All"),
        ("pane.data", "Data & Profile"),
        ("pane.data_short", "Data"),
        ("pane.collapse", "Click to collapse panel"),
        ("pane.data_expand", "Click to expand data"),
        ("pane.data_collapse", "Click to collapse data"),
        ("nav.home", "← Home"),
        ("nav.home_title", "Go to home"),
        ("nav.section", "Section:"),
        ("nav.bar", "Navigation bar"),
        // Content chrome
        ("toc.heading", "Table of Contents"),
        ("recent.heading", "Recent Notes"),
        ("backlinks.heading", "Backlinks"),
        ("backlinks.empty", "No backlinks to this note"),
        ("graph.heading", "Local Graph"),
        ("graph.placeholder", "Graph loading..."),
        ("sticky.label", "Sticky Headings"),
        ("sticky.to_top", "Scroll to top"),
        ("sticky.to_bottom", "Scroll to bottom"),
        ("tabs.side", "Switch tab position"),
        ("tabs.top", "⬒ Top"),
        ("tabs.top_title", "Place tabs at top horizontally"),
        ("tabs.right", "◨ Right"),
        ("tabs.right_title", "Place tabs at right vertically"),
        ("costume.label", "Costume:"),
        ("costume.item", "Costume"),
        ("image.zoom", "Zoom"),
        ("image.zoom_banner", "View full banner"),
        ("image.banner_alt", "Banner"),
        ("lightbox.close", "Close"),
        ("peek.fullscreen", "Open fullscreen"),
        ("peek.close", "Close"),
        ("preview.open_peek", "Click to open"),
        ("peek.error", "Failed to load page"),
        ("peek.not_found", "Content not found"),
        // Catalog page
        ("index.sections", "Sections"),
        ("index.entries", "All Notes"),
        ("index.note_unit", "notes"),
        // Controls
        ("view.switch", "Switch view mode"),
        ("view.book", "📖 Book"),
        ("view.book_title", "Book mode (multi-pane)"),
        ("view.classic", "📄 Scroll"),
        ("view.classic_title", "Vertical scroll mode"),
        ("drag.handle", "Drag to move"),
        ("theme.toggle", "Toggle theme (Light / Dark)"),
        ("theme.toggle_label", "Toggle theme"),
        ("theme.to_light", "Switch to light theme"),
        ("theme.to_dark", "Switch to dark theme"),
        ("theme.shell_toggle", "Toggle shell theme"),
        ("theme.shell_label", "Shell Theme"),
        ("theme.shell.slate", "Slate"),
        ("theme.shell.violet", "Violet"),
        ("theme.shell.indigo", "Indigo"),
        ("theme.shell.olive", "Olive"),
        ("theme.shell.contrast", "Contrast"),
        // Search
        ("search.placeholder", "Search... (Ctrl+K)"),
        ("search.loading", "Preparing search index..."),
        ("search.empty", "No results found"),
        ("search.aliases", "Aliases:"),
        ("search.prev", "◀ Prev"),
        ("search.next", "Next ▶"),
        // Editing
        ("edit.open", "Edit"),
        ("edit.editor", "Open in Editor"),
        ("edit.vscode", "Open in VSCode"),
        ("edit.cursor", "Open in Cursor"),
        ("edit.copy_path", "Copy Path"),
        ("edit.copy_path_title", "Copy file path"),
        ("toast.copied", "📋 Copied path to clipboard"),
        ("toast.copy_failed", "❌ Failed to copy path"),
        // Links that point at a page nobody has written yet
        ("link.unresolved", "Page not yet created"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// Backwards-compatible alias for Japanese UI strings.
pub fn default_ui_strings() -> HashMap<String, String> {
    default_ui_strings_ja()
}

/// Look up default hero chips for a given book language code.
pub fn default_hero_chips_for(lang: &str) -> Vec<HeroChipConfig> {
    if is_english(lang) {
        default_hero_chips_en()
    } else {
        default_hero_chips_ja()
    }
}

pub fn default_hero_chips_ja() -> Vec<HeroChipConfig> {
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

pub fn default_hero_chips_en() -> Vec<HeroChipConfig> {
    vec![
        HeroChipConfig {
            key: "parent".to_string(),
            label: Some("Affiliation".to_string()),
            format: None,
        },
        HeroChipConfig {
            key: "type.list-a".to_string(),
            label: Some("Fans".to_string()),
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

pub fn default_hero_chips() -> Vec<HeroChipConfig> {
    default_hero_chips_ja()
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

/// Look up default infobox labels for a given book language code.
pub fn default_key_labels_for(lang: &str) -> HashMap<String, String> {
    if is_english(lang) {
        default_key_labels_en()
    } else {
        default_key_labels_ja()
    }
}

pub fn default_key_labels_ja() -> HashMap<String, String> {
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

pub fn default_key_labels_en() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("kind".to_string(), "Kind".to_string());
    m.insert("type".to_string(), "Type".to_string());
    m.insert("birth".to_string(), "Birthday".to_string());
    m.insert("height".to_string(), "Height".to_string());
    m.insert("creator".to_string(), "Creator".to_string());
    m.insert("type.list-a".to_string(), "Fan Name".to_string());
    m.insert("type.element-a".to_string(), "Element / MBTI".to_string());
    m.insert("colors".to_string(), "Colors".to_string());
    m.insert("parent".to_string(), "Affiliation".to_string());
    m.insert("group".to_string(), "Organization".to_string());
    m.insert("place".to_string(), "Origin".to_string());
    m.insert("created".to_string(), "Created".to_string());
    m.insert("modified".to_string(), "Modified".to_string());
    m.insert("reviewed".to_string(), "Reviewed".to_string());
    m.insert("rating".to_string(), "Rating".to_string());
    m.insert("aliases".to_string(), "Aliases / Nicknames".to_string());
    m.insert("topics".to_string(), "Topics".to_string());
    m.insert("flags".to_string(), "Flags".to_string());
    m.insert("published".to_string(), "Published".to_string());
    m.insert("url.wiki".to_string(), "Wikipedia".to_string());
    m.insert("url.else".to_string(), "Related Links".to_string());
    m
}

pub fn default_key_labels() -> HashMap<String, String> {
    default_key_labels_ja()
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
        assert_eq!(cfg.ui.strings.len(), default_ui_strings_en().len());
    }

    #[test]
    fn english_and_japanese_dictionaries_have_identical_keys() {
        let ja = default_ui_strings_ja();
        let en = default_ui_strings_en();
        assert_eq!(ja.len(), en.len());
        for key in ja.keys() {
            assert!(en.contains_key(key), "Missing English key: {key}");
        }

        let ja_labels = default_key_labels_ja();
        let en_labels = default_key_labels_en();
        assert_eq!(ja_labels.len(), en_labels.len());
        for key in ja_labels.keys() {
            assert!(en_labels.contains_key(key), "Missing English label: {key}");
        }

        let ja_chips = default_hero_chips_ja();
        let en_chips = default_hero_chips_en();
        assert_eq!(ja_chips.len(), en_chips.len());
        for i in 0..ja_chips.len() {
            assert_eq!(ja_chips[i].key, en_chips[i].key);
        }
    }
}
