//! Search index building for tomet-book.

use crate::book::document::ProcessedDoc;
use tmtbook_search::{SearchDocument, SearchIndex, strip_html_to_plain_text};

/// Builds a `SearchIndex` from a collection of processed documents.
pub fn build_search_index(docs: &[&ProcessedDoc], clean_url_prefix: &str) -> SearchIndex {
    let mut search_docs = Vec::with_capacity(docs.len());

    for (idx, doc) in docs.iter().enumerate() {
        let mut aliases = Vec::new();
        for row in &doc.infobox_rows {
            let label_lower = row.label.to_lowercase();
            if label_lower == "aliases"
                || label_lower == "別名"
                || label_lower == "愛称"
                || label_lower == "別名 / 愛称"
                || label_lower == "aliases / nicknames"
            {
                for part in row.value.split(&[',', '、', ';'][..]) {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() && !aliases.iter().any(|a: &String| a == trimmed) {
                        aliases.push(trimmed.to_string());
                    }
                }
            }
        }

        let headings: Vec<String> = doc.toc.iter().map(|item| item.text.clone()).collect();
        let text = strip_html_to_plain_text(&doc.body_html);
        let image = doc
            .icon_image_url
            .clone()
            .or_else(|| doc.banner_url.clone());

        search_docs.push(SearchDocument {
            id: idx as u32,
            slug: doc.slug.clone(),
            title: doc.title.clone(),
            url: format!("{clean_url_prefix}/{}", doc.slug),
            path: doc.rel_path.clone(),
            aliases,
            kind: doc.kind.clone(),
            section: doc.section.clone(),
            image,
            headings,
            text,
        });
    }

    SearchIndex::new(search_docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_index_extracts_aliases_and_plain_text() {
        let doc = ProcessedDoc {
            slug: "talents/raora".to_string(),
            rel_path: "talents/raora.tmt".to_string(),
            source_path: "/vault/talents/raora.tmt".to_string(),
            title: "Raora Panthera".to_string(),
            section: Some("talents".to_string()),
            kind: Some("talent".to_string()),
            primary_color: None,
            icon: None,
            icon_image_url: Some("/vault/img/raora_icon.png".to_string()),
            link_icon: None,
            banner_url: None,
            banner_original_url: None,
            banner_y: None,
            images: vec![],
            original_images: vec![],
            hero_chips: vec![],
            infobox_rows: vec![crate::book::document::InfoboxRowItem {
                label: "別名 / 愛称".to_string(),
                value: "ラオラ, Raora".to_string(),
            }],
            has_data: true,
            toc: vec![crate::book::document::TocItem {
                level: 2,
                id: "overview".to_string(),
                text: "Overview".to_string(),
            }],
            section_tabs: vec![],
            body_html: "<h2>Overview</h2><p>Artist from Italy &amp; VTuber.</p>".to_string(),
            outgoing: vec![],
        };

        let index = build_search_index(&[&doc], "/wiki");
        assert_eq!(index.docs.len(), 1);
        let d = &index.docs[0];
        assert_eq!(d.slug, "talents/raora");
        assert_eq!(d.title, "Raora Panthera");
        assert_eq!(d.url, "/wiki/talents/raora");
        assert_eq!(d.aliases, vec!["ラオラ", "Raora"]);
        assert_eq!(d.headings, vec!["Overview"]);
        assert_eq!(d.image, Some("/vault/img/raora_icon.png".to_string()));
        assert_eq!(d.text, "Overview Artist from Italy & VTuber.");
    }
}
