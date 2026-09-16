use crate::config::HeroChipConfig;
use std::collections::HashMap;

/// The interface's text in Japanese (default).
pub fn ui_strings() -> HashMap<String, String> {
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
        ("theme.shell.color_label", "カラー"),
        ("theme.shell.pattern_label", "模様"),
        ("theme.shell.contrast_label", "コントラスト"),
        ("theme.shell.contrast_high", "ハイコントラスト"),
        ("theme.shell.slate", "スレート"),
        ("theme.shell.violet", "バイオレット"),
        ("theme.shell.indigo", "インディゴ"),
        ("theme.shell.olive", "オリーブ"),
        ("theme.shell.contrast", "コントラスト"),
        ("theme.shell.color_slate", "スレート"),
        ("theme.shell.color_violet", "バイオレット"),
        ("theme.shell.color_indigo", "インディゴ"),
        ("theme.shell.color_olive", "オリーブ"),
        ("theme.shell.pattern_marble", "マーブル"),
        ("theme.shell.pattern_cloud", "クラウド"),
        ("theme.shell.pattern_seigaiha", "青海波"),
        ("theme.shell.pattern_flourish", "古書装丁"),
        ("theme.shell.pattern_mesh", "メッシュ"),
        ("theme.shell.pattern_none", "なし"),
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

/// Default hero chips in Japanese.
pub fn hero_chips() -> Vec<HeroChipConfig> {
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

/// Default infobox key labels in Japanese.
pub fn key_labels() -> HashMap<String, String> {
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
