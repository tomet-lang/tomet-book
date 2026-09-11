//! Which `@kind`s the book publishes.
//!
//! `[build] exclude` drops documents by where they sit; this drops them by
//! what they say they are. One ordered list does both an allow list and a
//! deny list, the way `.gitignore` does:
//!
//! ```toml
//! kinds = ["*", "!config", "!index"]    # everything but these
//! kinds = ["page", "unit"]              # only these
//! kinds = ["*", "!deck.*", "deck.card"] # a namespace, minus one exception
//! ```
//!
//! The borrowed half is the notation, not the difficulty: gitignore's sharp
//! edges come from walking a directory tree, where an excluded parent hides
//! children that a later rule can no longer reach. Kinds are a flat set of
//! names, so the last rule that matches simply wins.

/// True when a document of this kind belongs in the book.
///
/// `kind` is `None` for a document that never declared one; only `*` matches
/// it. A document no pattern matches is left out, which is what makes a list
/// without any `*` read as "publish exactly these".
pub fn publishes(kind: Option<&str>, patterns: &[String]) -> bool {
    let name = kind.unwrap_or("");

    let mut verdict = false;
    for pattern in patterns {
        let (negated, glob) = match pattern.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, pattern.as_str()),
        };
        if glob_matches(glob, name) {
            verdict = !negated;
        }
    }
    verdict
}

/// `*` stands for any run of characters, including none.
fn glob_matches(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');

    // Everything before the first `*` has to be exactly where it says.
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };

    let mut last: Option<&str> = None;
    for part in parts {
        // A trailing `*` leaves an empty part, which anything satisfies.
        if let Some(previous) = last.replace(part)
            && !previous.is_empty()
        {
            match rest.find(previous) {
                Some(at) => rest = &rest[at + previous.len()..],
                None => return false,
            }
        }
    }

    match last {
        // No `*` at all: the prefix had to be the whole name.
        None => rest.is_empty(),
        // The final segment must land on the end.
        Some(tail) => rest.len() >= tail.len() && rest.ends_with(tail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_default_publishes_everything_but_the_control_kinds() {
        let p = patterns(&["*", "!config", "!index"]);
        assert!(publishes(Some("page"), &p));
        assert!(publishes(Some("unit"), &p));
        assert!(publishes(None, &p), "a document with no kind is a page");
        assert!(!publishes(Some("config"), &p));
        assert!(!publishes(Some("index"), &p));
    }

    #[test]
    fn a_list_without_a_star_publishes_exactly_what_it_names() {
        let p = patterns(&["page", "unit"]);
        assert!(publishes(Some("page"), &p));
        assert!(publishes(Some("unit"), &p));
        assert!(!publishes(Some("event"), &p));
        assert!(
            !publishes(None, &p),
            "only `*` reaches a document with no kind"
        );
    }

    #[test]
    fn the_last_matching_rule_wins() {
        let p = patterns(&["*", "!deck.*", "deck.card"]);
        assert!(publishes(Some("deck.card"), &p));
        assert!(!publishes(Some("deck.bookmark"), &p));
        assert!(publishes(Some("page"), &p));

        // Order is the whole rule: reversed, the namespace wins again.
        let reversed = patterns(&["*", "deck.card", "!deck.*"]);
        assert!(!publishes(Some("deck.card"), &reversed));
    }

    #[test]
    fn an_empty_list_publishes_nothing() {
        assert!(!publishes(Some("page"), &[]));
        assert!(!publishes(None, &[]));
    }

    #[test]
    fn stars_match_anywhere_in_the_name() {
        assert!(glob_matches("*", ""));
        assert!(glob_matches("*", "anything"));
        assert!(glob_matches("deck.*", "deck.card"));
        assert!(!glob_matches("deck.*", "decoy"));
        assert!(glob_matches("*.card", "deck.card"));
        assert!(glob_matches("deck.*.jp", "deck.card.jp"));
        assert!(!glob_matches("deck.*.jp", "deck.card.en"));
        assert!(glob_matches("page", "page"));
        assert!(!glob_matches("page", "pages"));
    }
}
