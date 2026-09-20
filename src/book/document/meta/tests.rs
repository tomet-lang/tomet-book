use super::*;

fn ctx_config() -> BookConfig {
    BookConfig::default()
}

fn index() -> tomet_links::VaultLinkIndex {
    tomet_links::VaultLinkIndex::from_paths(["30-39 Knowledge/rust.tmt"])
}

fn extract_from(json: &str, kind: Option<&str>) -> MetaProperties {
    let value: Value = serde_json::from_str(json).unwrap();
    extract(
        Some(&value),
        kind,
        Path::new("notes/a.tmt"),
        &index(),
        &ctx_config(),
        None,
    )
}

#[test]
fn no_metadata_yields_nothing_to_show() {
    let props = extract(
        None,
        None,
        Path::new("a.tmt"),
        &index(),
        &ctx_config(),
        None,
    );
    assert!(!props.has_data());
    assert!(props.infobox_rows.is_empty());
}

#[test]
fn only_hex_colours_become_the_primary_colour() {
    assert_eq!(
        extract_from(r##"{"colors": ["#A1B2C3", "#000"]}"##, None)
            .primary_color
            .as_deref(),
        Some("a1b2c3")
    );
    assert_eq!(
        extract_from(r#"{"colors": "red"}"#, None).primary_color,
        None
    );
}

#[test]
fn media_references_resolve_under_the_asset_prefix() {
    let props = extract_from(
        r#"{"banner": "hero.png", "images": ["a.png", "b.png"]}"#,
        None,
    );
    assert_eq!(props.banner_url.as_deref(), Some("/vault/hero.png"));
    assert_eq!(props.images, vec!["/vault/a.png", "/vault/b.png"]);
    assert!(props.has_data());
}

#[test]
fn banner_y_accepts_both_spellings_and_both_number_types() {
    assert_eq!(
        extract_from(r#"{"banner-y": 11}"#, None).banner_y,
        Some(11.0)
    );
    assert_eq!(
        extract_from(r#"{"banner_y": 12.5}"#, None).banner_y,
        Some(12.5)
    );
}

#[test]
fn hero_chips_follow_the_configured_keys() {
    // The default config asks for parent, type.list-a, type.element-a, birth.
    let props = extract_from(
        r#"{"parent": "[[rust]]", "birth": "2001-04-09", "ignored": "x"}"#,
        None,
    );
    let values: Vec<&str> = props.hero_chips.iter().map(|c| c.value.as_str()).collect();
    assert_eq!(values, vec!["rust", "4月9日"]);
    assert_eq!(
        props.hero_chips[0].href.as_deref(),
        Some("/wiki/30-39-Knowledge/rust")
    );
}

#[test]
fn an_array_meta_value_becomes_one_chip_per_entry() {
    let props = extract_from(r#"{"parent": ["[[rust]]", "other"]}"#, None);
    assert_eq!(props.hero_chips.len(), 2);
    assert_eq!(props.hero_chips[1].href, None);
}

#[test]
fn the_document_kind_fills_in_for_a_missing_type() {
    let rows = extract_from(r#"{"place": "東京"}"#, Some("person")).infobox_rows;
    assert!(
        rows.iter()
            .any(|r| r.label == "種別" && r.value == "person")
    );
}

#[test]
fn an_explicit_type_wins_over_the_document_kind() {
    let rows = extract_from(r#"{"type": "キャラクター"}"#, Some("person")).infobox_rows;
    let kinds: Vec<&str> = rows
        .iter()
        .filter(|r| r.label == "種別")
        .map(|r| r.value.as_str())
        .collect();
    assert_eq!(kinds, vec!["キャラクター"]);
}

#[test]
fn excluded_keys_never_reach_the_infobox() {
    // `icon` and `banner` are chrome, not rows.
    let rows = extract_from(
        r#"{"icon": "🎂", "banner": "x.png", "place": "東京"}"#,
        None,
    )
    .infobox_rows;
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["出身地"]);
}

#[test]
fn external_urls_become_links_and_arrays_are_joined() {
    let rows = extract_from(
        r#"{"url.wiki": "https://example.com", "aliases": ["A", "B"]}"#,
        None,
    )
    .infobox_rows;

    let wiki = rows.iter().find(|r| r.label == "Wikipedia").unwrap();
    assert!(wiki.value.contains(r#"href="https://example.com""#));

    let aliases = rows.iter().find(|r| r.label == "別名 / 愛称").unwrap();
    assert_eq!(aliases.value, "A、B");
}

#[test]
fn booleans_and_numbers_render_as_text() {
    let rows = extract_from(r#"{"rating": 5, "flags": true}"#, None).infobox_rows;
    assert!(rows.iter().any(|r| r.value == "5"));
    assert!(rows.iter().any(|r| r.value == "Yes"));
}

#[test]
fn element_link_in_banner_and_images_resolves() {
    let props = extract_from(
        r#"{
            "banner": {"element": "link", "args": {"ref": "+771a.svg"}},
            "images": [
                {"element": "link", "args": {"ref": "a.png"}},
                {"element": "link", "args": "b.png"}
            ]
        }"#,
        None,
    );
    assert_eq!(props.banner_url.as_deref(), Some("/vault/+771a.svg"));
    assert_eq!(props.images, vec!["/vault/a.png", "/vault/b.png"]);
    assert!(props.has_data());
}

#[test]
fn element_link_in_hero_chips_and_infobox_rows() {
    let props = extract_from(
        r#"{
            "parent": {"element": "link", "args": {"ref": "rust"}},
            "author": {"element": "link", "args": {"target": "rust", "alias": "Rust Lang"}}
        }"#,
        None,
    );
    assert_eq!(props.hero_chips.len(), 1);
    assert_eq!(props.hero_chips[0].value, "rust");
    assert_eq!(
        props.hero_chips[0].href.as_deref(),
        Some("/wiki/30-39-Knowledge/rust")
    );
    assert!(
        props
            .linked_slugs
            .contains(&"30-39-Knowledge/rust".to_string())
    );

    let author_row = props
        .infobox_rows
        .iter()
        .find(|r| r.label == "author")
        .unwrap();
    assert!(
        author_row
            .value
            .contains(r#"href="/wiki/30-39-Knowledge/rust""#)
    );
    assert!(author_row.value.contains("Rust Lang"));
}

#[test]
fn icon_as_embed_or_link_or_image_path_resolves() {
    let props_embed = extract_from(
        r#"{"icon": {"element": "embed", "args": {"target": "avatar.png"}}}"#,
        None,
    );
    assert_eq!(
        props_embed.icon.as_deref(),
        Some(r#"<img class="tm-doc-icon-img" src="/vault/avatar.png" alt="icon" loading="lazy">"#)
    );
    assert_eq!(
        props_embed.icon_image_url.as_deref(),
        Some("/vault/avatar.png")
    );

    let props_link = extract_from(
        r#"{"icon": {"element": "link", "args": "avatar.png"}}"#,
        None,
    );
    assert_eq!(
        props_link.icon.as_deref(),
        Some(r#"<img class="tm-doc-icon-img" src="/vault/avatar.png" alt="icon" loading="lazy">"#)
    );

    let props_str = extract_from(r#"{"icon": "profile.jpg"}"#, None);
    assert_eq!(
        props_str.icon.as_deref(),
        Some(r#"<img class="tm-doc-icon-img" src="/vault/profile.jpg" alt="icon" loading="lazy">"#)
    );

    let props_emoji = extract_from(r#"{"icon": "🎂"}"#, None);
    assert_eq!(
        props_emoji.icon.as_deref(),
        Some(r#"<span class="avatar-text avatar-text-1">🎂</span>"#)
    );
    assert_eq!(props_emoji.icon_image_url, None);
}
