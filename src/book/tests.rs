use super::*;

/// A unique scratch directory; avoids pulling in a temp-file dependency.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tmtbook-test-{}-{}-{:?}",
        name,
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A document with only the fields the backlink index reads.
fn doc(slug: &str, title: &str, section: Option<&str>, outgoing: &[&str]) -> ProcessedDoc {
    ProcessedDoc {
        slug: slug.to_string(),
        rel_path: format!("{slug}.tmt"),
        source_path: format!("/vault/{slug}.tmt"),
        title: title.to_string(),
        section: section.map(str::to_string),
        kind: None,
        primary_color: None,
        icon: None,
        icon_image_url: None,
        link_icon: None,
        banner_url: None,
        banner_original_url: None,
        banner_y: None,
        images: Vec::new(),
        original_images: Vec::new(),
        hero_chips: Vec::new(),
        infobox_rows: Vec::new(),
        has_data: false,
        toc: Vec::new(),
        section_tabs: Vec::new(),
        body_html: String::new(),
        outgoing: outgoing.iter().map(|s| s.to_string()).collect(),
    }
}

fn summary(slug: &str, title: &str) -> EntrySummary {
    EntrySummary {
        url: format!("/wiki/{slug}"),
        title: title.to_string(),
        section: None,
        children: Vec::new(),
    }
}

fn listed(slugs: &[(&str, &[&str])]) -> Vec<catalog::IndexEntry> {
    slugs
        .iter()
        .map(|(slug, kids)| catalog::IndexEntry {
            slug: slug.to_string(),
            children: kids
                .iter()
                .map(|k| catalog::IndexEntry {
                    slug: k.to_string(),
                    children: Vec::new(),
                })
                .collect(),
        })
        .collect()
}

fn rendered(slugs: &[&str]) -> HashMap<String, EntrySummary> {
    slugs
        .iter()
        .map(|s| (s.to_string(), summary(s, &s.to_uppercase())))
        .collect()
}

#[test]
fn the_catalog_follows_the_written_order() {
    let out = arrange_entries(
        &listed(&[("zebra", &[]), ("apple", &[])]),
        &rendered(&["apple", "zebra"]),
    );

    let titles: Vec<&str> = out.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["ZEBRA", "APPLE"]);
}

#[test]
fn a_page_the_index_leaves_out_is_not_listed() {
    let out = arrange_entries(&listed(&[("apple", &[])]), &rendered(&["apple", "hidden"]));

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].title, "APPLE");
}

#[test]
fn nesting_survives_the_lookup() {
    let out = arrange_entries(
        &listed(&[("guide", &["install"])]),
        &rendered(&["guide", "install"]),
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].children.len(), 1);
    assert_eq!(out[0].children[0].title, "INSTALL");
}

#[test]
fn a_named_page_that_does_not_exist_hands_its_children_up() {
    // The index may have been written before the note, or the note may
    // have failed to render. What hung below it is still real.
    let out = arrange_entries(&listed(&[("ghost", &["install"])]), &rendered(&["install"]));

    let titles: Vec<&str> = out.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["INSTALL"]);
}

#[test]
fn backlinks_point_from_every_linking_page() {
    let a = doc("a", "Aardvark", Some("Animals"), &["hub"]);
    let b = doc("b", "Bison", None, &["hub", "a"]);
    let hub = doc("hub", "Hub", None, &[]);
    let index = build_backlink_index(&[&a, &b, &hub], "/wiki");

    let into_hub = &index["hub"];
    assert_eq!(into_hub.len(), 2);
    assert_eq!(into_hub[0].title, "Aardvark");
    assert_eq!(into_hub[0].url, "/wiki/a");
    assert_eq!(into_hub[0].section.as_deref(), Some("Animals"));
    assert_eq!(into_hub[1].title, "Bison");
    assert_eq!(into_hub[1].section, None);

    assert_eq!(index["a"].len(), 1, "b links to a");
    assert!(!index.contains_key("b"), "nothing links to b");
}

#[test]
fn backlinks_are_ordered_by_title_then_url() {
    // Two pages share a title; the url settles the tie.
    let z = doc("z", "Same", None, &["t"]);
    let m = doc("m", "Same", None, &["t"]);
    let first = doc("first", "Alpha", None, &["t"]);
    let index = build_backlink_index(&[&z, &m, &first], "/wiki");

    let urls: Vec<&str> = index["t"].iter().map(|b| b.url.as_str()).collect();
    assert_eq!(urls, ["/wiki/first", "/wiki/m", "/wiki/z"]);
}

#[test]
fn a_vault_without_links_has_an_empty_index() {
    let a = doc("a", "A", None, &[]);
    assert!(build_backlink_index(&[&a], "/wiki").is_empty());
}

#[test]
fn the_url_prefix_reaches_the_backlink_urls() {
    let a = doc("a", "A", None, &["b"]);
    let index = build_backlink_index(&[&a], "/docs/book");
    assert_eq!(index["b"][0].url, "/docs/book/a");
}

#[test]
fn build_book_enhances_internal_links_with_note_icons() {
    let src = scratch("build-internal-icons-src");
    let out = scratch("build-internal-icons-out");

    fs::write(
        src.join("target.tmt"),
        "@meta{icon: \"🛍️\"}\n\n#[ Target ]\n\nTarget page.\n",
    )
    .unwrap();
    fs::write(
        src.join("source.tmt"),
        "#[ Source ]\n\nVisit @link(ref:\"target\")[My Store].\n",
    )
    .unwrap();

    let mut config = BookConfig::default();
    config.build.pagefind = false;

    let report = build_book(&src, &out, &config, false).unwrap();
    assert_eq!(report.rendered, 2);
    assert!(report.doc_icons.contains_key("target"));

    let source_html = fs::read_to_string(out.join("wiki/source/index.html")).unwrap();
    assert!(
            source_html.contains(
                r#"<a class="tm-file" href="/wiki/target"><span class="tm-link-icon tm-link-icon-emoji" aria-hidden="true">🛍️</span>My Store</a>"#
            ),
            "actual source_html: {source_html}"
        );

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn build_book_emits_pagefind_path_metadata() {
    let src = scratch("build-pagefind-path-src");
    let out = scratch("build-pagefind-path-out");

    let nested_dir = src.join("dev");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(
        nested_dir.join("roadmap.tmt"),
        "#[ Roadmap ]\n\nProject roadmap contents.\n",
    )
    .unwrap();

    let mut config = BookConfig::default();
    config.build.pagefind = false;

    let report = build_book(&src, &out, &config, false).unwrap();
    assert_eq!(report.rendered, 1);

    let roadmap_html = fs::read_to_string(out.join("wiki/dev/roadmap/index.html")).unwrap();
    assert!(
            roadmap_html.contains(r#"<span class="sr-only" data-pagefind-meta="path" data-pagefind-weight="10.0">dev/roadmap</span>"#),
            "HTML should contain span with dev/roadmap: {roadmap_html}"
        );
    assert!(
        roadmap_html.contains(
            r#"<span class="sr-only" data-pagefind-weight="10.0">dev/roadmap.tmt</span>"#
        ),
        "HTML should contain span with dev/roadmap.tmt: {roadmap_html}"
    );

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn build_book_sanitizes_urls_with_spaces_symbols_and_emoji() {
    let src = scratch("build-clean-url-src");
    let out = scratch("build-clean-url-out");

    let rust_dir = src.join("30-39 Knowledge");
    fs::create_dir_all(&rust_dir).unwrap();
    fs::write(
        rust_dir.join("Rust (基礎) 🦀.tmt"),
        "#[ Rust基礎 ]\n\nLink to @link(ref:\"Web 開発 {Next}\").\n",
    )
    .unwrap();

    fs::write(
        src.join("Web 開発 {Next}.tmt"),
        "#[ Web開発 ]\n\nLink to @link(ref:\"Rust (基礎) 🦀\").\n",
    )
    .unwrap();

    let mut config = BookConfig::default();
    config.build.pagefind = false;

    let report = build_book(&src, &out, &config, false).unwrap();
    assert_eq!(report.rendered, 2);

    // 1. Filesystem outputs must have sanitized paths
    let rust_html_path = out.join("wiki/30-39-Knowledge/Rust-基礎/index.html");
    let web_html_path = out.join("wiki/Web-開発-Next/index.html");
    assert!(
        rust_html_path.is_file(),
        "expected {rust_html_path:?} to exist"
    );
    assert!(
        web_html_path.is_file(),
        "expected {web_html_path:?} to exist"
    );

    // 2. Rendered link hrefs must point to clean URLs
    let rust_html = fs::read_to_string(&rust_html_path).unwrap();
    assert!(
        rust_html.contains(r#"href="/wiki/Web-開発-Next""#),
        "Rust doc HTML must link to clean URL: {rust_html}"
    );

    let web_html = fs::read_to_string(&web_html_path).unwrap();
    assert!(
        web_html.contains(r#"href="/wiki/30-39-Knowledge/Rust-基礎""#),
        "Web doc HTML must link to clean URL: {web_html}"
    );

    // 3. Backlinks must be indexed under clean slugs
    let rust_incoming = report
        .backlinks
        .get("30-39-Knowledge/Rust-基礎")
        .expect("rust backlinks");
    assert_eq!(rust_incoming.len(), 1);
    assert_eq!(rust_incoming[0].url, "/wiki/Web-開発-Next");

    let web_incoming = report
        .backlinks
        .get("Web-開発-Next")
        .expect("web backlinks");
    assert_eq!(web_incoming.len(), 1);
    assert_eq!(web_incoming[0].url, "/wiki/30-39-Knowledge/Rust-基礎");

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn build_book_flat_routing_collision_error_by_default() {
    use crate::config::RoutingStrategy;

    let src = scratch("flat-collision-err-src");
    let out = scratch("flat-collision-err-out");

    let rust_dir = src.join("tech/rust");
    let js_dir = src.join("tech/js");
    fs::create_dir_all(&rust_dir).unwrap();
    fs::create_dir_all(&js_dir).unwrap();
    fs::write(
        rust_dir.join("closures.tmt"),
        "#[ Rust Closures ]\n\nbody\n",
    )
    .unwrap();
    fs::write(js_dir.join("closures.tmt"), "#[ JS Closures ]\n\nbody\n").unwrap();

    let mut config = BookConfig::default();
    config.build.pagefind = false;
    config.build.routing = RoutingStrategy::Flat;

    let res = build_book(&src, &out, &config, false);
    assert!(res.is_err(), "build_book must fail on collision by default");
    let err_msg = res.unwrap_err().to_string();
    assert!(err_msg.contains("Duplicate slug(s) detected"), "{err_msg}");
    assert!(err_msg.contains("tech/rust/closures.tmt"), "{err_msg}");
    assert!(err_msg.contains("tech/js/closures.tmt"), "{err_msg}");

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn build_book_flat_routing_disambiguation() {
    use crate::config::{CollisionStrategy, RoutingStrategy};

    let src = scratch("flat-disambiguate-src");
    let out = scratch("flat-disambiguate-out");

    let rust_dir = src.join("tech/rust");
    let js_dir = src.join("tech/js");
    fs::create_dir_all(&rust_dir).unwrap();
    fs::create_dir_all(&js_dir).unwrap();
    fs::write(
        rust_dir.join("closures.tmt"),
        "#[ Rust Closures ]\n\nLink to @link(ref:\"tech/js/closures.tmt\").\n",
    )
    .unwrap();
    fs::write(
        js_dir.join("closures.tmt"),
        "#[ JS Closures ]\n\nLink to @link(ref:\"tech/rust/closures.tmt\").\n",
    )
    .unwrap();

    let mut config = BookConfig::default();
    config.build.pagefind = false;
    config.build.routing = RoutingStrategy::Flat;
    config.build.on_collision = CollisionStrategy::Disambiguate;

    let report = build_book(&src, &out, &config, false).unwrap();
    assert_eq!(report.rendered, 2);

    // Outputs should be disambiguated with flat hyphenated names
    let rust_html_path = out.join("wiki/rust-closures/index.html");
    let js_html_path = out.join("wiki/js-closures/index.html");
    assert!(rust_html_path.is_file(), "expected {rust_html_path:?}");
    assert!(js_html_path.is_file(), "expected {js_html_path:?}");

    // Links must resolve to disambiguated URLs
    let rust_html = fs::read_to_string(&rust_html_path).unwrap();
    assert!(
        rust_html.contains(r#"href="/wiki/js-closures""#),
        "Rust doc must link to /wiki/js-closures: {rust_html}"
    );

    let js_html = fs::read_to_string(&js_html_path).unwrap();
    assert!(
        js_html.contains(r#"href="/wiki/rust-closures""#),
        "JS doc must link to /wiki/rust-closures: {js_html}"
    );

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn build_book_explicit_slug_precedence() {
    let src = scratch("explicit-slug-src");
    let out = scratch("explicit-slug-out");

    let nested = src.join("deep/nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("guide.tmt"),
        "@meta{ slug: \"super-guide\" }\n\n#[ Super Guide ]\n\nContent.\n",
    )
    .unwrap();
    fs::write(
        src.join("index.tmt"),
        "#[ Home ]\n\nSee @link(ref:\"deep/nested/guide.tmt\").\n",
    )
    .unwrap();

    let mut config = BookConfig::default();
    config.build.pagefind = false;

    let report = build_book(&src, &out, &config, false).unwrap();
    assert_eq!(report.rendered, 2);

    let guide_html_path = out.join("wiki/super-guide/index.html");
    assert!(guide_html_path.is_file(), "expected {guide_html_path:?}");

    let index_html = fs::read_to_string(out.join("wiki/index/index.html")).unwrap();
    assert!(
        index_html.contains(r#"href="/wiki/super-guide""#),
        "Index doc must link to /wiki/super-guide: {index_html}"
    );

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn build_book_id_routing_behavior() {
    use crate::config::RoutingStrategy;

    let src = scratch("id-routing-src");
    let out_hier = scratch("id-routing-hier-out");
    let out_id = scratch("id-routing-id-out");

    let notes_dir = src.join("notes");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::write(
        notes_dir.join("my-note.tmt"),
        "@meta{ id: \"random-uuid-1234\" }\n\n#[ My Note ]\n\nContent.\n",
    )
    .unwrap();
    fs::write(
        src.join("index.tmt"),
        "@meta{ slug: \"index\" }\n\n#[ Home ]\n\nSee @link(ref:\"notes/my-note.tmt\").\n",
    )
    .unwrap();

    // 1. Under hierarchical routing, @meta{ id } must be IGNORED
    let mut hier_config = BookConfig::default();
    hier_config.build.pagefind = false;
    hier_config.build.routing = RoutingStrategy::Hierarchical;

    let report_hier = build_book(&src, &out_hier, &hier_config, false).unwrap();
    assert_eq!(report_hier.rendered, 2);
    assert!(out_hier.join("wiki/notes/my-note/index.html").is_file());
    let index_hier = fs::read_to_string(out_hier.join("wiki/index/index.html")).unwrap();
    assert!(index_hier.contains(r#"href="/wiki/notes/my-note""#));

    // 2. Under id routing, @meta{ id } is USED
    let mut id_config = BookConfig::default();
    id_config.build.pagefind = false;
    id_config.build.routing = RoutingStrategy::Id;

    let report_id = build_book(&src, &out_id, &id_config, false).unwrap();
    assert_eq!(report_id.rendered, 2);
    assert!(out_id.join("wiki/random-uuid-1234/index.html").is_file());
    let index_id = fs::read_to_string(out_id.join("wiki/index/index.html")).unwrap();
    assert!(index_id.contains(r#"href="/wiki/random-uuid-1234""#));

    let _ = fs::remove_dir_all(&src);
    let _ = fs::remove_dir_all(&out_hier);
    let _ = fs::remove_dir_all(&out_id);
}
