//! Renders `@doc.icon("name", pkg:"lucide")` -- Tomet resolves this element
//! (`crate::vocabulary::builtin_doc_vocabularies` in the `tomet` repo) but
//! deliberately never draws it, by design: an external renderer is meant to
//! implement the notation. This is that implementation, wired in as a
//! `tomet_html::RenderOptions::custom_element` hook.
//!
//! Only `pkg:"lucide"` is supported today, referencing the sprite sheet
//! vendored whole at `frontend/icons/lucide/sprite.svg` (every
//! Lucide icon, not a curated subset) via `crate::book::assets::LUCIDE_SPRITE`.
//! Both an unknown `pkg` and an unknown `name` render a visible placeholder
//! (rather than nothing), so a typo in a document shows up in the page
//! instead of silently vanishing.

use std::collections::HashSet;
use std::sync::LazyLock;

use tomet_ast::Value;
use tomet_html::CustomElementCtx;

use crate::book::assets::LUCIDE_SPRITE;

use super::html::escape_html;

/// Every icon name the vendored sprite actually defines (its `<symbol
/// id="...">`s), so an unknown `name` can render a visible placeholder
/// instead of a `<use>` that silently draws nothing.
static LUCIDE_ICON_NAMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    LUCIDE_SPRITE
        .match_indices("<symbol id=\"")
        .filter_map(|(start, matched)| {
            let rest = &LUCIDE_SPRITE[start + matched.len()..];
            rest.find('"').map(|end| &rest[..end])
        })
        .collect()
});

pub(crate) fn is_lucide_icon(name: &str) -> bool {
    LUCIDE_ICON_NAMES.contains(name)
}

fn arg_str<'a>(args: Option<&'a Value>, key: &str) -> Option<&'a str> {
    let Value::Map(entries) = args? else {
        return None;
    };
    entries
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
}

/// A visible stand-in for an icon this crate cannot draw -- an unvendored
/// name, or a `pkg` other than `lucide`. Never blank: a document author
/// should see a typo, not lose the icon silently.
fn placeholder(name: &str, pkg: &str, inline: bool) -> String {
    let tag = if inline { "span" } else { "div" };
    format!(
        "<{tag} class=\"tm-doc-icon tm-doc-icon-missing\" title=\"unknown icon: {pkg}/{name}\">?</{tag}>"
    )
}

/// The `@meta` counterpart of [`render`]: an `icon` field written as
/// `icon: @doc.icon("cake")` rather than a plain string/emoji.
///
/// `@meta`'s data reaches `document::mod`'s page-chrome extraction as
/// `serde_json::Value` (`tomet_semantics::value_to_json`), which has no
/// element concept -- a `Value::Element` there degrades to a tagged
/// `{"element": ..., "args": ...}` object, and `MetaProperties::icon`'s
/// existing `.as_str()` read silently sees `None` for it. This reads the
/// raw `tomet_ast::Value` instead, *before* that JSON conversion, so the
/// element never has to round-trip through JSON at all -- same element,
/// same [`render`], just called directly rather than through
/// `tomet_html::RenderOptions::custom_element`'s hook (there is no
/// enclosing document render pass over `@meta`'s data for that hook to
/// fire during).
///
/// `Bindings::default()` is what `render`'s own caller
/// (`render_custom_or_generic_element` in `tomet-html`) effectively uses
/// for body content too -- `doc.icon` resolves unconditionally there, the
/// same way `std` does, so no vault/vocabulary context is needed here
/// either.
pub fn render_meta_value(el: &tomet_ast::Element, inline: bool) -> Option<String> {
    let name = el.sigil.name().map(|n| n.to_string()).unwrap_or_default();
    let bindings = tomet_semantics::Bindings::default();
    let ctx = CustomElementCtx::new(&name, el.content.as_deref(), inline, || {
        tomet_semantics::normalized_element_args_in(el, &bindings)
    });
    render(ctx)
}

/// `@meta`'s `icon` field, when it is a `@doc.icon(...)` element rather
/// than the plain string `MetaProperties::icon`'s own JSON-based read
/// already covers. `None` for a plain string (that path handles it), a
/// missing `icon` key, or an `icon` written as something else entirely.
pub fn meta_icon_element(raw_meta: Option<&tomet_ast::Value>) -> Option<String> {
    let tomet_ast::Value::Map(entries) = raw_meta? else {
        return None;
    };
    let (_, value) = entries.iter().find(|(k, _)| k == "icon")?;
    let tomet_ast::Value::Element(el) = value else {
        return None;
    };
    render_meta_value(el, true)
}

/// [`tomet_html::RenderOptions::custom_element`] entry point. Answers only
/// for `doc.icon`; every other `Custom` kind falls through to the crate's
/// generic rendering (`None`).
pub fn render(ctx: CustomElementCtx) -> Option<String> {
    if ctx.kind != "doc.icon" {
        return None;
    }

    // `normalized_args()` clones the args map, so it's only worth calling
    // once we already know this hook is going to answer for the element.
    let args = ctx.normalized_args();
    let name = arg_str(args.as_ref(), "name").unwrap_or_default();
    let pkg = arg_str(args.as_ref(), "pkg").unwrap_or("lucide");
    let name_attr = escape_html(name);
    let pkg_attr = escape_html(pkg);

    if pkg != "lucide" || !LUCIDE_ICON_NAMES.contains(name) {
        return Some(placeholder(&name_attr, &pkg_attr, ctx.inline));
    }

    // `name` is only reached here once it has matched a real `<symbol
    // id="...">` in the sprite, so it is already a safe bare identifier --
    // no escaping needed in the `#name` fragment.
    let svg = format!(
        "<svg class=\"tm-doc-icon\" data-icon=\"{name_attr}\" aria-hidden=\"true\"><use href=\"/icons/lucide.svg#{name}\"></use></svg>"
    );

    Some(if ctx.inline {
        svg
    } else {
        format!("<div class=\"tm-doc-icon-block\">{svg}</div>")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tomet_ast::Value;

    fn args(name: &str, pkg: Option<&str>) -> Value {
        let mut entries = vec![("name".to_string(), Value::String(name.to_string()))];
        if let Some(pkg) = pkg {
            entries.push(("pkg".to_string(), Value::String(pkg.to_string())));
        }
        Value::Map(entries)
    }

    fn ctx<'a>(kind: &'a str, args: &'a Value, inline: bool) -> CustomElementCtx<'a> {
        CustomElementCtx::new(kind, None, inline, || Some(args.clone()))
    }

    #[test]
    fn ignores_kinds_other_than_doc_icon() {
        let a = args("star", Some("lucide"));
        assert_eq!(render(ctx("doc.index", &a, true)), None);
    }

    #[test]
    fn renders_a_vendored_lucide_icon_inline() {
        let a = args("star", Some("lucide"));
        let html = render(ctx("doc.icon", &a, true)).unwrap();
        assert_eq!(
            html,
            "<svg class=\"tm-doc-icon\" data-icon=\"star\" aria-hidden=\"true\"><use href=\"/icons/lucide.svg#star\"></use></svg>"
        );
    }

    #[test]
    fn a_lucide_icon_the_sprite_actually_defines_is_recognized() {
        // Not an exhaustive check of all ~1800 icons -- just that the
        // sprite-parsing in `LUCIDE_ICON_NAMES` actually found entries,
        // rather than e.g. silently matching zero symbols and treating
        // every icon as unknown.
        assert!(
            LUCIDE_ICON_NAMES.len() > 1000,
            "{}",
            LUCIDE_ICON_NAMES.len()
        );
        assert!(LUCIDE_ICON_NAMES.contains("triangle"));
    }

    #[test]
    fn defaults_pkg_to_lucide_when_omitted() {
        let a = args("star", None);
        let html = render(ctx("doc.icon", &a, true)).unwrap();
        assert!(html.contains("data-icon=\"star\""));
    }

    #[test]
    fn wraps_a_block_icon_in_a_div() {
        let a = args("star", Some("lucide"));
        let html = render(ctx("doc.icon", &a, false)).unwrap();
        assert!(html.starts_with("<div class=\"tm-doc-icon-block\"><svg"));
        assert!(html.ends_with("</svg></div>"));
    }

    #[test]
    fn an_unknown_icon_name_is_a_visible_placeholder_not_nothing() {
        let a = args("this-icon-does-not-exist", Some("lucide"));
        let html = render(ctx("doc.icon", &a, true)).unwrap();
        assert!(html.contains("tm-doc-icon-missing"));
        assert!(html.contains("this-icon-does-not-exist"));
    }

    #[test]
    fn an_unsupported_pkg_is_a_visible_placeholder() {
        let a = args("star", Some("heroicons"));
        let html = render(ctx("doc.icon", &a, true)).unwrap();
        assert!(html.contains("tm-doc-icon-missing"));
        assert!(html.contains("heroicons"));
    }

    fn parse_meta(src: &str) -> Option<tomet_ast::Value> {
        let doc = tomet_parser::parse_document(src).expect("valid Tomet source");
        tomet_semantics::document_meta(&doc)
    }

    #[test]
    fn meta_icon_renders_a_real_doc_icon_element() {
        let raw_meta = parse_meta("@meta{icon: @doc.icon(\"cake\")}\n");
        let html = meta_icon_element(raw_meta.as_ref()).unwrap();
        assert!(html.contains("<svg"));
        assert!(html.contains("data-icon=\"cake\""));
    }

    #[test]
    fn meta_icon_is_none_for_a_plain_string_icon() {
        // `MetaProperties::icon`'s own JSON-based read covers this case;
        // `meta_icon_element` deliberately answers only for an embedded
        // element, not a plain string/emoji.
        let raw_meta = parse_meta("@meta{icon: \"\u{1F382}\"}\n");
        assert!(meta_icon_element(raw_meta.as_ref()).is_none());
    }

    #[test]
    fn meta_icon_is_none_with_no_icon_key_at_all() {
        let raw_meta = parse_meta("@meta{title: \"a\"}\n");
        assert!(meta_icon_element(raw_meta.as_ref()).is_none());
    }

    #[test]
    fn meta_icon_falls_back_to_a_placeholder_for_an_unknown_name() {
        let raw_meta = parse_meta("@meta{icon: @doc.icon(\"not-a-real-icon\")}\n");
        let html = meta_icon_element(raw_meta.as_ref()).unwrap();
        assert!(html.contains("tm-doc-icon-missing"));
    }
}
