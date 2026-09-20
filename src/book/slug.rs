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
}
