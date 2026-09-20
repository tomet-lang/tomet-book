use serde::{Deserialize, Serialize};

/// A single searchable document in the index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: u32,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub path: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub headings: Vec<String>,
    #[serde(default)]
    pub text: String,
}

/// The collection of all searchable documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SearchIndex {
    pub docs: Vec<SearchDocument>,
}

impl SearchIndex {
    pub fn new(docs: Vec<SearchDocument>) -> Self {
        Self { docs }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Strips HTML tags and unescapes basic entities to produce clean plain prose text.
pub fn strip_html_to_plain_text(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut current_tag = String::new();

    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            in_tag = true;
            current_tag.clear();
            continue;
        }

        if in_tag {
            if ch == '>' {
                in_tag = false;
                let tag_lower = current_tag.to_ascii_lowercase();
                let tag_name = tag_lower
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('/');

                if tag_name == "script" || tag_name == "style" {
                    in_script_or_style = !current_tag.starts_with('/');
                } else if matches!(
                    tag_name,
                    "p" | "div"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "li"
                        | "tr"
                        | "br"
                        | "blockquote"
                ) {
                    result.push(' ');
                }
            } else {
                current_tag.push(ch);
            }
            continue;
        }

        if in_script_or_style {
            continue;
        }

        if ch == '&' {
            let mut entity = String::new();
            let mut matched_semi = false;
            while let Some(&next_ch) = chars.peek() {
                if next_ch == ';' {
                    chars.next();
                    matched_semi = true;
                    break;
                } else if next_ch.is_alphanumeric() || next_ch == '#' {
                    entity.push(chars.next().unwrap());
                    if entity.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }

            if matched_semi {
                match entity.as_str() {
                    "amp" => result.push('&'),
                    "lt" => result.push('<'),
                    "gt" => result.push('>'),
                    "quot" => result.push('"'),
                    "apos" | "#39" => result.push('\''),
                    "nbsp" => result.push(' '),
                    _ => {
                        result.push('&');
                        result.push_str(&entity);
                        result.push(';');
                    }
                }
            } else {
                result.push('&');
                result.push_str(&entity);
            }
            continue;
        }

        result.push(ch);
    }

    // Collapse whitespace
    let mut collapsed = String::with_capacity(result.len());
    let mut last_was_whitespace = false;
    for ch in result.chars() {
        if ch.is_whitespace() {
            if !last_was_whitespace {
                collapsed.push(' ');
                last_was_whitespace = true;
            }
        } else {
            collapsed.push(ch);
            last_was_whitespace = false;
        }
    }

    collapsed.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        let html = "<p>Hello <strong>world</strong>!</p>";
        assert_eq!(strip_html_to_plain_text(html), "Hello world!");
    }

    #[test]
    fn test_strip_html_ignores_script_and_style() {
        let html = "<div>Text<script>console.log('secret');</script> after <style>.btn{color:red;}</style>done</div>";
        assert_eq!(strip_html_to_plain_text(html), "Text after done");
    }

    #[test]
    fn test_strip_html_entities() {
        let html = "Tom &amp; Jerry &lt;friends&gt; &quot;quote&quot; &#39;single&#39; a&nbsp;b";
        assert_eq!(
            strip_html_to_plain_text(html),
            "Tom & Jerry <friends> \"quote\" 'single' a b"
        );
    }

    #[test]
    fn test_search_index_serialization() {
        let doc = SearchDocument {
            id: 0,
            slug: "rust/closures".to_string(),
            title: "Rust Closures".to_string(),
            url: "/wiki/rust/closures".to_string(),
            path: "rust/closures.tmt".to_string(),
            aliases: vec!["Anonymous Functions".to_string()],
            kind: Some("concept".to_string()),
            section: Some("tech".to_string()),
            image: None,
            headings: vec!["Syntax".to_string(), "Environment Capture".to_string()],
            text: "Closures are functions that can capture their environment.".to_string(),
        };

        let index = SearchIndex::new(vec![doc.clone()]);
        let json = index.to_json().unwrap();
        assert!(json.contains("rust/closures"));
        assert!(json.contains("Anonymous Functions"));
        assert!(!json.contains("\"image\"")); // skipped when None

        let deserialized: SearchIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.docs.len(), 1);
        assert_eq!(deserialized.docs[0], doc);
    }
}
