//! Clean slug generation for URLs and page paths.
//!
//! Converts vault paths into safe, clean web slugs by:
//! - Replacing spaces and unsafe/disallowed punctuation with hyphens (`-`).
//! - Stripping emoji and graphical symbols.
//! - Retaining alphanumeric characters (ASCII and Unicode, including Japanese Kanji/Hiragana/Katakana) and underscores (`_`).
//! - Collapsing consecutive hyphens into a single hyphen.
//! - Trimming leading and trailing hyphens from each path segment.
//! - Preserving directory hierarchy (`/`).
//! - Falling back to a deterministic hash (`doc-{hash}`) if a segment becomes empty.

use super::document::strip_doc_extension;

/// FNV-1a 32-bit hash for deterministic fallback slugs when a segment is stripped empty.
fn fnv1a_hash(s: &str) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for b in s.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// Sanitizes a single path segment (e.g. a directory name or file stem).
pub fn clean_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let mut last_was_dash = false;

    for c in segment.chars() {
        if c.is_alphanumeric() || c == '_' {
            out.push(c);
            last_was_dash = false;
        } else if c == '-' {
            if !last_was_dash && !out.is_empty() {
                out.push('-');
                last_was_dash = true;
            }
        } else {
            // Spaces, emoji, and disallowed punctuation become a hyphen delimiter.
            if !last_was_dash && !out.is_empty() {
                out.push('-');
                last_was_dash = true;
            }
        }
    }

    let trimmed = out.trim_matches(|c| c == '-' || c == '_');
    if trimmed.is_empty() {
        let hash = fnv1a_hash(segment);
        format!("doc-{hash:08x}")
    } else {
        trimmed.to_string()
    }
}

/// Generates a clean URL slug from a vault-relative document path.
///
/// Strips document extensions (`.tmt`, `.tm`), normalizes path separators to `/`,
/// and sanitizes each path segment independently.
pub fn clean_doc_slug(rel_path: &str) -> String {
    let normalized = rel_path.replace('\\', "/");
    let without_ext = strip_doc_extension(&normalized);

    without_ext
        .split('/')
        .filter(|seg| !seg.is_empty())
        .map(clean_segment)
        .collect::<Vec<String>>()
        .join("/")
}

use crate::config::{BuildConfig, CollisionStrategy, RoutingStrategy};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Metadata extracted from a document for routing resolution.
#[derive(Debug, Clone)]
pub struct DocRouteInput<'a> {
    pub rel_path: &'a str,
    pub explicit_slug: Option<&'a str>,
    pub meta_id: Option<&'a str>,
}

/// Extract explicit `slug` and `id` from a document's `@meta` block, if present.
pub fn extract_route_metadata(doc: &tomet_ast::Document) -> (Option<String>, Option<String>) {
    let raw_meta = tomet_semantics::document_meta(doc);
    let meta_json = raw_meta.as_ref().map(tomet_semantics::value_to_json);
    let slug = meta_json
        .as_ref()
        .and_then(|m| m.get("slug"))
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
    let id = meta_json
        .as_ref()
        .and_then(|m| m.get("id"))
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
    (slug, id)
}

/// Global routing table mapping every published document path to its unique clean slug.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteTable {
    rel_to_slug: HashMap<String, String>,
    slug_to_rel: HashMap<String, String>,
}

impl RouteTable {
    /// Look up the clean slug assigned to a given vault-relative path.
    pub fn get(&self, rel_path: &str) -> Option<&str> {
        let normalized = crate::book::normalize_path_str(rel_path);
        self.rel_to_slug.get(&normalized).map(|s| s.as_str())
    }

    /// Look up the vault-relative path assigned to a given clean slug.
    pub fn rel_path_for_slug(&self, slug: &str) -> Option<&str> {
        let clean = slug.trim_matches('/');
        self.slug_to_rel.get(clean).map(|s| s.as_str())
    }

    /// Look up the clean slug for a path, or fall back to `clean_doc_slug(rel_path)`.
    pub fn get_or_clean(&self, rel_path: &str) -> String {
        self.get(rel_path)
            .map(str::to_string)
            .unwrap_or_else(|| clean_doc_slug(rel_path))
    }

    /// Build the global `RouteTable` for all published documents according to `BuildConfig`.
    pub fn build(docs: &[DocRouteInput<'_>], config: &BuildConfig) -> anyhow::Result<Self> {
        if docs.is_empty() {
            return Ok(Self::default());
        }

        // 1. Compute initial candidate slug for each document
        let mut candidates: Vec<String> = Vec::with_capacity(docs.len());
        for doc in docs {
            let candidate =
                if let Some(explicit) = doc.explicit_slug.filter(|s| !s.trim().is_empty()) {
                    clean_doc_slug(explicit)
                } else {
                    match config.routing {
                        RoutingStrategy::Hierarchical => clean_doc_slug(doc.rel_path),
                        RoutingStrategy::Flat => {
                            let stem = Path::new(doc.rel_path)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(doc.rel_path);
                            clean_segment(stem)
                        }
                        RoutingStrategy::Id => {
                            if let Some(id) = doc.meta_id.filter(|s| !s.trim().is_empty()) {
                                clean_segment(id)
                            } else {
                                format!("doc-{:08x}", fnv1a_hash(doc.rel_path))
                            }
                        }
                    }
                };
            candidates.push(candidate);
        }

        // 2. Group document indices by candidate slug to detect collisions
        let mut slug_to_indices: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, cand) in candidates.iter().enumerate() {
            slug_to_indices.entry(cand.clone()).or_default().push(i);
        }

        let mut has_collision = false;
        for indices in slug_to_indices.values() {
            if indices.len() > 1 {
                has_collision = true;
                break;
            }
        }

        // 3. Handle collisions if present
        if has_collision {
            match config.on_collision {
                CollisionStrategy::Error => {
                    let mut msg = String::from("Duplicate slug(s) detected across documents:\n");
                    for (slug, indices) in &slug_to_indices {
                        if indices.len() > 1 {
                            msg.push_str(&format!("  Slug '{slug}' collided in:\n"));
                            for &idx in indices {
                                msg.push_str(&format!("    - {}\n", docs[idx].rel_path));
                            }
                        }
                    }
                    msg.push_str(
                        "\nHint: provide a unique '@meta{ slug: \"...\" }' in the conflicting files, \
                         or configure 'on_collision = \"disambiguate\"' in tmtbook.toml.",
                    );
                    anyhow::bail!(msg);
                }
                CollisionStrategy::Disambiguate => {
                    for (slug, indices) in &slug_to_indices {
                        if indices.len() <= 1 {
                            continue;
                        }

                        // Collect colliding paths
                        let colliding_paths: Vec<&str> =
                            indices.iter().map(|&idx| docs[idx].rel_path).collect();

                        let resolved = if config.routing == RoutingStrategy::Id {
                            disambiguate_id_slugs(slug, &colliding_paths)
                        } else {
                            disambiguate_flat_slugs(&colliding_paths)
                        };

                        for (i, &idx) in indices.iter().enumerate() {
                            candidates[idx] = resolved[i].clone();
                        }
                    }
                }
            }
        }

        // 4. Build final lookup table
        let mut rel_to_slug = HashMap::with_capacity(docs.len());
        let mut slug_to_rel = HashMap::with_capacity(docs.len());
        for (i, doc) in docs.iter().enumerate() {
            let normalized = crate::book::normalize_path_str(doc.rel_path);
            let slug = candidates[i].clone();
            rel_to_slug.insert(normalized.clone(), slug.clone());
            slug_to_rel.insert(slug, normalized);
        }

        Ok(Self {
            rel_to_slug,
            slug_to_rel,
        })
    }
}

/// Disambiguates flat slugs by prepending ancestor directory names with hyphens.
fn disambiguate_flat_slugs(rel_paths: &[&str]) -> Vec<String> {
    let parsed_segments: Vec<Vec<String>> = rel_paths
        .iter()
        .map(|p| {
            let normalized = p.replace('\\', "/");
            let without_ext = strip_doc_extension(&normalized);
            without_ext
                .split('/')
                .filter(|s| !s.is_empty())
                .map(clean_segment)
                .collect()
        })
        .collect();

    let n = rel_paths.len();
    let mut depth = 1;
    let max_depth = parsed_segments
        .iter()
        .map(|segs| segs.len())
        .max()
        .unwrap_or(1);

    let mut current_slugs = Vec::with_capacity(n);

    while depth <= max_depth {
        current_slugs.clear();
        for segs in &parsed_segments {
            let take_count = depth.min(segs.len());
            let slice = &segs[segs.len() - take_count..];
            current_slugs.push(slice.join("-"));
        }

        let mut unique_set = HashSet::new();
        let all_unique = current_slugs.iter().all(|s| unique_set.insert(s));
        if all_unique {
            return current_slugs;
        }

        depth += 1;
    }

    // Fallback if identical segments: sequential suffix -2, -3...
    let mut final_slugs = Vec::with_capacity(n);
    let mut seen_counts: HashMap<String, usize> = HashMap::new();
    for slug in current_slugs {
        let count = seen_counts.entry(slug.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            final_slugs.push(slug);
        } else {
            final_slugs.push(format!("{slug}-{}", *count));
        }
    }

    final_slugs
}

/// Disambiguates duplicate IDs by prepending parent folder or appending index.
fn disambiguate_id_slugs(id_slug: &str, rel_paths: &[&str]) -> Vec<String> {
    let mut results = Vec::with_capacity(rel_paths.len());
    let mut seen = HashSet::new();

    for (i, p) in rel_paths.iter().enumerate() {
        let parent = Path::new(p)
            .parent()
            .and_then(|d| d.file_name())
            .and_then(|s| s.to_str());
        let candidate = if let Some(parent) = parent {
            format!("{id_slug}-{}", clean_segment(parent))
        } else {
            format!("{id_slug}-{}", i + 1)
        };

        if seen.insert(candidate.clone()) {
            results.push(candidate);
        } else {
            results.push(format!("{id_slug}-{}", i + 1));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_simple_ascii_slugs() {
        assert_eq!(clean_doc_slug("hello world.tmt"), "hello-world");
        assert_eq!(clean_doc_slug("notes/my note.tm"), "notes/my-note");
        assert_eq!(clean_doc_slug("deep/nested/path.tmt"), "deep/nested/path");
    }

    #[test]
    fn preserves_japanese_characters() {
        assert_eq!(clean_doc_slug("読書 メモ.tmt"), "読書-メモ");
        assert_eq!(clean_doc_slug("知識/Rust 入門.tmt"), "知識/Rust-入門");
        assert_eq!(
            clean_doc_slug("カタカナ/ひらがな/漢字.tmt"),
            "カタカナ/ひらがな/漢字"
        );
        assert_eq!(clean_doc_slug("リーダーシップ.tmt"), "リーダーシップ");
    }

    #[test]
    fn sanitizes_disallowed_symbols() {
        assert_eq!(clean_doc_slug("Rust (基礎).tmt"), "Rust-基礎");
        assert_eq!(clean_doc_slug("note {test}.tmt"), "note-test");
        assert_eq!(clean_doc_slug("array[0].tmt"), "array-0");
        assert_eq!(clean_doc_slug("what?!.tmt"), "what");
        assert_eq!(clean_doc_slug("foo & bar.tmt"), "foo-bar");
        assert_eq!(clean_doc_slug("version 1.0.tmt"), "version-1-0");
    }

    #[test]
    fn strips_emoji_from_slug() {
        assert_eq!(clean_doc_slug("Rust 🦀 メモ.tmt"), "Rust-メモ");
        assert_eq!(clean_doc_slug("30-39 📚 読書/本.tmt"), "30-39-読書/本");
        assert_eq!(clean_doc_slug("happy 😀.tmt"), "happy");
    }

    #[test]
    fn cleans_emoticons_and_collapses_dashes() {
        assert_eq!(clean_doc_slug("(o_O).tmt"), "o_O");
        assert_eq!(clean_doc_slug("foo---bar.tmt"), "foo-bar");
        assert_eq!(clean_doc_slug("  spaced  out  .tmt"), "spaced-out");
        assert_eq!(clean_doc_slug("30-39 - Knowledge.tmt"), "30-39-Knowledge");
    }

    #[test]
    fn falls_back_to_hash_for_purely_symbolic_or_emoji_names() {
        let slug1 = clean_doc_slug("😀.tmt");
        assert!(slug1.starts_with("doc-"));
        assert_eq!(slug1.len(), 12);

        let slug2 = clean_doc_slug("(^_^).tmt");
        assert!(slug2.starts_with("doc-"));

        assert_eq!(clean_doc_slug("😀.tmt"), slug1);
    }

    #[test]
    fn preserves_inner_underscores() {
        assert_eq!(
            clean_doc_slug("my_snake_case_note.tmt"),
            "my_snake_case_note"
        );
        assert_eq!(clean_doc_slug("_hidden_name_.tmt"), "hidden_name");
    }

    #[test]
    fn route_table_hierarchical_preserves_full_tree() {
        let docs = [
            DocRouteInput {
                rel_path: "30-39 Knowledge/Rust (基礎).tmt",
                explicit_slug: None,
                meta_id: Some("random-123"),
            },
            DocRouteInput {
                rel_path: "notes/Rust (基礎).tmt",
                explicit_slug: None,
                meta_id: Some("random-456"),
            },
        ];
        let cfg = BuildConfig::default();
        let table = RouteTable::build(&docs, &cfg).unwrap();

        assert_eq!(
            table.get("30-39 Knowledge/Rust (基礎).tmt"),
            Some("30-39-Knowledge/Rust-基礎")
        );
        assert_eq!(table.get("notes/Rust (基礎).tmt"), Some("notes/Rust-基礎"));
    }

    #[test]
    fn route_table_flat_uses_stem() {
        let docs = [
            DocRouteInput {
                rel_path: "30-39 Knowledge/Rust.tmt",
                explicit_slug: None,
                meta_id: Some("ignored-id"),
            },
            DocRouteInput {
                rel_path: "notes/Web 開発 {Next}.tmt",
                explicit_slug: None,
                meta_id: None,
            },
        ];
        let cfg = BuildConfig {
            routing: RoutingStrategy::Flat,
            ..Default::default()
        };

        let table = RouteTable::build(&docs, &cfg).unwrap();
        assert_eq!(table.get("30-39 Knowledge/Rust.tmt"), Some("Rust"));
        assert_eq!(
            table.get("notes/Web 開発 {Next}.tmt"),
            Some("Web-開発-Next")
        );
    }

    #[test]
    fn route_table_flat_errors_on_collision_by_default() {
        let docs = [
            DocRouteInput {
                rel_path: "tech/rust/closures.tmt",
                explicit_slug: None,
                meta_id: None,
            },
            DocRouteInput {
                rel_path: "tech/js/closures.tmt",
                explicit_slug: None,
                meta_id: None,
            },
        ];
        let cfg = BuildConfig {
            routing: RoutingStrategy::Flat,
            on_collision: CollisionStrategy::Error,
            ..Default::default()
        };

        let err = RouteTable::build(&docs, &cfg).unwrap_err();
        assert!(err.to_string().contains("Slug 'closures'"));
        assert!(err.to_string().contains("tech/rust/closures.tmt"));
        assert!(err.to_string().contains("tech/js/closures.tmt"));
    }

    #[test]
    fn route_table_flat_disambiguates_with_parent_dir() {
        let docs = [
            DocRouteInput {
                rel_path: "tech/rust/closures.tmt",
                explicit_slug: None,
                meta_id: None,
            },
            DocRouteInput {
                rel_path: "tech/js/closures.tmt",
                explicit_slug: None,
                meta_id: None,
            },
        ];
        let cfg = BuildConfig {
            routing: RoutingStrategy::Flat,
            on_collision: CollisionStrategy::Disambiguate,
            ..Default::default()
        };

        let table = RouteTable::build(&docs, &cfg).unwrap();
        assert_eq!(table.get("tech/rust/closures.tmt"), Some("rust-closures"));
        assert_eq!(table.get("tech/js/closures.tmt"), Some("js-closures"));
    }

    #[test]
    fn route_table_flat_disambiguates_nested_parents() {
        let docs = [
            DocRouteInput {
                rel_path: "cat1/sub/foo.tmt",
                explicit_slug: None,
                meta_id: None,
            },
            DocRouteInput {
                rel_path: "cat2/sub/foo.tmt",
                explicit_slug: None,
                meta_id: None,
            },
        ];
        let cfg = BuildConfig {
            routing: RoutingStrategy::Flat,
            on_collision: CollisionStrategy::Disambiguate,
            ..Default::default()
        };

        let table = RouteTable::build(&docs, &cfg).unwrap();
        assert_eq!(table.get("cat1/sub/foo.tmt"), Some("cat1-sub-foo"));
        assert_eq!(table.get("cat2/sub/foo.tmt"), Some("cat2-sub-foo"));
    }

    #[test]
    fn route_table_explicit_slug_takes_precedence_over_all_modes() {
        let docs = [DocRouteInput {
            rel_path: "notes/rust/guide.tmt",
            explicit_slug: Some("my-super-guide"),
            meta_id: Some("id-123"),
        }];

        // Flat
        let cfg_flat = BuildConfig {
            routing: RoutingStrategy::Flat,
            ..Default::default()
        };
        let table_flat = RouteTable::build(&docs, &cfg_flat).unwrap();
        assert_eq!(
            table_flat.get("notes/rust/guide.tmt"),
            Some("my-super-guide")
        );

        // Id
        let cfg_id = BuildConfig {
            routing: RoutingStrategy::Id,
            ..Default::default()
        };
        let table_id = RouteTable::build(&docs, &cfg_id).unwrap();
        assert_eq!(table_id.get("notes/rust/guide.tmt"), Some("my-super-guide"));

        // Hierarchical
        let cfg_hier = BuildConfig::default();
        let table_hier = RouteTable::build(&docs, &cfg_hier).unwrap();
        assert_eq!(
            table_hier.get("notes/rust/guide.tmt"),
            Some("my-super-guide")
        );
    }

    #[test]
    fn route_table_id_mode_uses_meta_id() {
        let docs = [
            DocRouteInput {
                rel_path: "notes/rust.tmt",
                explicit_slug: None,
                meta_id: Some("k3j8f9a2"),
            },
            DocRouteInput {
                rel_path: "notes/missing-id.tmt",
                explicit_slug: None,
                meta_id: None,
            },
        ];
        let cfg = BuildConfig {
            routing: RoutingStrategy::Id,
            ..Default::default()
        };

        let table = RouteTable::build(&docs, &cfg).unwrap();
        assert_eq!(table.get("notes/rust.tmt"), Some("k3j8f9a2"));
        // Fallback for missing id starts with doc-
        assert!(
            table
                .get("notes/missing-id.tmt")
                .unwrap()
                .starts_with("doc-")
        );
    }

    #[test]
    fn route_table_id_mode_collision_errors_by_default() {
        let docs = [
            DocRouteInput {
                rel_path: "notes/a.tmt",
                explicit_slug: None,
                meta_id: Some("duplicate-id"),
            },
            DocRouteInput {
                rel_path: "notes/b.tmt",
                explicit_slug: None,
                meta_id: Some("duplicate-id"),
            },
        ];
        let cfg = BuildConfig {
            routing: RoutingStrategy::Id,
            on_collision: CollisionStrategy::Error,
            ..Default::default()
        };

        let err = RouteTable::build(&docs, &cfg).unwrap_err();
        assert!(err.to_string().contains("Slug 'duplicate-id'"));
    }

    #[test]
    fn route_table_rel_path_for_slug_lookup() {
        let docs = [
            DocRouteInput {
                rel_path: "tech/rust/closures.tmt",
                explicit_slug: None,
                meta_id: None,
            },
            DocRouteInput {
                rel_path: "intro.tmt",
                explicit_slug: Some("welcome"),
                meta_id: None,
            },
        ];
        let cfg = BuildConfig::default();
        let table = RouteTable::build(&docs, &cfg).unwrap();

        assert_eq!(
            table.rel_path_for_slug("tech/rust/closures"),
            Some("tech/rust/closures.tmt")
        );
        assert_eq!(
            table.rel_path_for_slug("/tech/rust/closures/"),
            Some("tech/rust/closures.tmt")
        );
        assert_eq!(table.rel_path_for_slug("welcome"), Some("intro.tmt"));
        assert_eq!(table.rel_path_for_slug("non-existent"), None);
    }
}
