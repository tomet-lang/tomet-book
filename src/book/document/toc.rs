//! The table of contents and the sticky-tab tree built from it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TocItem {
    pub id: String,
    pub level: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocNode {
    pub id: String,
    pub text: String,
    pub level: u32,
    pub children: Vec<TocNode>,
}

pub type SectionTab = TocNode;

/// Headings the book treats as page furniture rather than content, so a
/// document opening with one still gets its real title from the next heading.
fn is_boilerplate_heading(text: &str) -> bool {
    text.eq_ignore_ascii_case("related")
        || text.eq_ignore_ascii_case("footnotes")
        || text.eq_ignore_ascii_case("references")
        || text == "関連"
        || text == "参考文献"
        || text == "脚注"
}

/// Turn the renderer's heading outline into the page TOC, alongside the first
/// content H1 (used as a title fallback).
///
/// Only levels 1-4 make it into the TOC; deeper headings are structure the
/// sticky tabs have no room for.
pub fn toc_from_outline(outline: &[tomet_html::HeadingInfo]) -> (Option<String>, Vec<TocItem>) {
    let mut first_h1 = None;
    let mut toc = Vec::new();

    for heading in outline {
        let text = heading.text.trim();
        if text.is_empty() {
            continue;
        }

        if heading.level == 1 && first_h1.is_none() && !is_boilerplate_heading(text) {
            first_h1 = Some(text.to_string());
        }

        if (1..=4).contains(&heading.level) {
            toc.push(TocItem {
                id: heading.id.clone().unwrap_or_default(),
                level: u32::from(heading.level),
                text: text.to_string(),
            });
        }
    }

    (first_h1, toc)
}

fn insert_node_into(parent: &mut TocNode, node: TocNode) {
    if let Some(last) = parent.children.last_mut()
        && node.level > last.level
    {
        insert_node_into(last, node);
        return;
    }
    parent.children.push(node);
}

/// Fold a flat TOC into the tree the sticky-tab header renders.
///
/// Roots are the shallowest level present (H1 when the document has any),
/// and every deeper heading nests under the most recent shallower one.
pub fn build_section_tabs(toc: &[TocItem]) -> Vec<TocNode> {
    let has_h1 = toc.iter().any(|item| item.level == 1);
    let top_level = if has_h1 {
        1
    } else {
        toc.iter().map(|item| item.level).min().unwrap_or(1)
    };

    let mut section_tabs: Vec<TocNode> = Vec::new();
    for item in toc {
        let node = TocNode {
            id: item.id.clone(),
            text: item.text.clone(),
            level: item.level,
            children: Vec::new(),
        };

        if section_tabs.is_empty() || item.level <= top_level {
            section_tabs.push(node);
        } else if let Some(last_root) = section_tabs.last_mut() {
            if item.level > last_root.level {
                insert_node_into(last_root, node);
            } else {
                section_tabs.push(node);
            }
        }
    }

    section_tabs
}

#[cfg(test)]
mod tests {
    use super::*;
    use tomet_html::HeadingInfo;

    fn heading(level: u8, id: Option<&str>, text: &str) -> HeadingInfo {
        HeadingInfo {
            level,
            id: id.map(str::to_string),
            text: text.to_string(),
            number: None,
        }
    }

    fn toc(items: &[(u32, &str)]) -> Vec<TocItem> {
        items
            .iter()
            .map(|(level, text)| TocItem {
                id: text.to_lowercase(),
                level: *level,
                text: text.to_string(),
            })
            .collect()
    }

    fn shape(nodes: &[TocNode]) -> Vec<(String, Vec<String>)> {
        nodes
            .iter()
            .map(|n| {
                (
                    n.text.clone(),
                    n.children.iter().map(|c| c.text.clone()).collect(),
                )
            })
            .collect()
    }

    // ---- outline -> TOC ----

    #[test]
    fn levels_one_to_four_become_toc_items() {
        let (_, toc) = toc_from_outline(&[
            heading(1, Some("a"), "A"),
            heading(4, Some("d"), "D"),
            heading(5, Some("e"), "E"),
        ]);

        let seen: Vec<(u32, &str)> = toc.iter().map(|i| (i.level, i.text.as_str())).collect();
        assert_eq!(
            seen,
            vec![(1, "A"), (4, "D")],
            "H5 is too deep for the tabs"
        );
    }

    #[test]
    fn a_heading_without_an_id_still_lists_but_cannot_be_linked() {
        let (_, toc) = toc_from_outline(&[heading(1, None, "A")]);
        assert_eq!(toc[0].id, "");
    }

    #[test]
    fn the_first_content_h1_is_reported_as_the_title_candidate() {
        let (first_h1, _) = toc_from_outline(&[
            heading(1, Some("a"), "Real Title"),
            heading(1, Some("b"), "Later"),
        ]);
        assert_eq!(first_h1.as_deref(), Some("Real Title"));
    }

    #[test]
    fn boilerplate_headings_never_become_the_title() {
        let (first_h1, _) = toc_from_outline(&[
            heading(1, Some("r"), "関連"),
            heading(1, Some("t"), "本当の見出し"),
        ]);
        assert_eq!(first_h1.as_deref(), Some("本当の見出し"));

        for label in ["Related", "references", "FOOTNOTES", "参考文献", "脚注"] {
            let (first_h1, _) = toc_from_outline(&[heading(1, Some("x"), label)]);
            assert_eq!(first_h1, None, "{label} should not be a title");
        }
    }

    #[test]
    fn a_deeper_first_heading_leaves_the_title_unset() {
        let (first_h1, toc) = toc_from_outline(&[heading(2, Some("s"), "Section")]);
        assert_eq!(first_h1, None);
        assert_eq!(toc.len(), 1);
    }

    #[test]
    fn blank_headings_are_dropped() {
        let (first_h1, toc) =
            toc_from_outline(&[heading(1, Some("a"), "   "), heading(1, Some("b"), "B")]);
        assert_eq!(toc.len(), 1);
        assert_eq!(first_h1.as_deref(), Some("B"));
    }

    #[test]
    fn the_number_label_never_leaks_into_the_toc_text() {
        // Numbering lives in HeadingInfo::number, so the text is the author's.
        let numbered = HeadingInfo {
            level: 1,
            id: Some("a".to_string()),
            text: "Chapter".to_string(),
            number: Some("1".to_string()),
        };
        let (_, toc) = toc_from_outline(&[numbered]);
        assert_eq!(toc[0].text, "Chapter");
    }

    // ---- TOC -> sticky tab tree ----

    #[test]
    fn flat_headings_all_become_roots() {
        let tabs = build_section_tabs(&toc(&[(1, "A"), (1, "B"), (1, "C")]));
        assert_eq!(
            shape(&tabs),
            vec![
                ("A".into(), vec![]),
                ("B".into(), vec![]),
                ("C".into(), vec![]),
            ]
        );
    }

    #[test]
    fn deeper_headings_nest_under_the_preceding_root() {
        let tabs = build_section_tabs(&toc(&[
            (1, "A"),
            (2, "A-1"),
            (2, "A-2"),
            (1, "B"),
            (2, "B-1"),
        ]));
        assert_eq!(
            shape(&tabs),
            vec![
                ("A".into(), vec!["A-1".into(), "A-2".into()]),
                ("B".into(), vec!["B-1".into()]),
            ]
        );
    }

    #[test]
    fn third_level_headings_nest_under_their_parent() {
        let tabs = build_section_tabs(&toc(&[(1, "A"), (2, "A-1"), (3, "A-1-a"), (2, "A-2")]));
        assert_eq!(tabs.len(), 1);
        let a1 = &tabs[0].children[0];
        assert_eq!(a1.text, "A-1");
        assert_eq!(a1.children.len(), 1);
        assert_eq!(a1.children[0].text, "A-1-a");
        assert_eq!(tabs[0].children[1].text, "A-2");
    }

    #[test]
    fn documents_without_h1_use_their_shallowest_level_as_roots() {
        let tabs = build_section_tabs(&toc(&[(2, "A"), (3, "A-1"), (2, "B")]));
        assert_eq!(
            shape(&tabs),
            vec![("A".into(), vec!["A-1".into()]), ("B".into(), vec![])]
        );
    }

    #[test]
    fn an_empty_toc_yields_no_tabs() {
        assert!(build_section_tabs(&[]).is_empty());
    }
}
