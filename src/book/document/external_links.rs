//! External link enhancement: adds brand SVG icons (via Simple Icons) or
//! dynamic favicons (via Google Favicon API or DuckDuckGo) to external links.

use regex::Regex;
use std::sync::LazyLock;

use super::html::escape_html;
use super::icon::is_simple_icon;
use crate::config::{FaviconService, LinksConfig};

static EXTERNAL_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Matches `<a ... href="https?://..." ...>...</a>`
    Regex::new(r#"(?s)<a\b([^>]*\bhref="(https?://([^"/:]+)[^"]*)"[^>]*)>(.*?)</a>"#)
        .expect("valid regex for external link")
});

/// Maps common domains to known Simple Icons symbol IDs.
fn domain_to_simple_icon(domain: &str) -> Option<&'static str> {
    match domain {
        // Git & Code Repositories
        "github.com" | "gist.github.com" => Some("github"),
        "gitlab.com" => Some("gitlab"),
        "bitbucket.org" => Some("bitbucket"),

        // Japanese Tech & Knowledge
        "zenn.dev" => Some("zenn"),
        "qiita.com" => Some("qiita"),
        "b.hatena.ne.jp" | "hatenablog.com" => Some("hatenabookmark"),

        // Social & Communication
        "x.com" | "twitter.com" => Some("x"),
        "youtube.com" | "youtu.be" => Some("youtube"),
        "discord.com" | "discord.gg" => Some("discord"),
        "reddit.com" => Some("reddit"),
        "twitch.tv" => Some("twitch"),
        "bsky.app" => Some("bluesky"),
        "t.me" | "telegram.org" => Some("telegram"),
        "medium.com" => Some("medium"),
        "substack.com" => Some("substack"),

        // Search & Reference
        "google.com" | "google.co.jp" => Some("google"),
        "wikipedia.org" => Some("wikipedia"),
        d if d.ends_with(".wikipedia.org") => Some("wikipedia"),
        "developer.mozilla.org" => Some("mdnwebdocs"),
        "stackoverflow.com" => Some("stackoverflow"),
        "notion.so" | "notion.site" => Some("notion"),
        "obsidian.md" => Some("obsidian"),

        // Languages, Runtimes & Packages
        "rust-lang.org" | "crates.io" | "docs.rs" => Some("rust"),
        "python.org" | "pypi.org" => Some("python"),
        "nodejs.org" | "npmjs.com" | "npmjs.org" => Some("npm"),
        "deno.land" | "deno.com" => Some("deno"),
        "bun.sh" => Some("bun"),
        "neovim.io" => Some("neovim"),

        // Web Frameworks & Tools
        "react.dev" | "reactjs.org" => Some("react"),
        "vuejs.org" => Some("vuedotjs"),
        "svelte.dev" => Some("svelte"),
        "nextjs.org" => Some("nextdotjs"),
        "astro.build" => Some("astro"),
        "vite.dev" | "vitejs.dev" => Some("vite"),
        "tailwindcss.com" => Some("tailwindcss"),
        "figma.com" => Some("figma"),

        // Cloud & Infra
        "docker.com" | "hub.docker.com" => Some("docker"),
        "kubernetes.io" => Some("kubernetes"),
        "cloudflare.com" => Some("cloudflare"),
        "vercel.com" => Some("vercel"),
        "netlify.com" => Some("netlify"),
        "supabase.com" => Some("supabase"),
        "firebase.google.com" => Some("firebase"),
        "stripe.com" => Some("stripe"),

        // Linux Distributions
        "archlinux.org" => Some("archlinux"),
        "ubuntu.com" => Some("ubuntu"),
        "debian.org" => Some("debian"),

        _ => None,
    }
}

/// Normalizes a hostname by lowercasing and stripping leading `www.`.
fn normalize_host(raw_host: &str) -> String {
    let lower = raw_host.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix("www.") {
        stripped.to_string()
    } else {
        lower
    }
}

/// Resolves the HTML string for an external link's prefix icon.
fn resolve_link_icon(host: &str, config: &LinksConfig) -> Option<String> {
    let norm = normalize_host(host);

    // 1. Check known domains for Simple Icons
    if let Some(icon_name) = domain_to_simple_icon(&norm)
        && is_simple_icon(icon_name)
    {
        let safe_name = escape_html(icon_name);
        return Some(format!(
            "<svg class=\"tm-link-icon\" aria-hidden=\"true\"><use href=\"/icons/simple.svg#{safe_name}\"></use></svg>"
        ));
    }

    // 2. Fallback to Favicon API service
    match config.favicon_service {
        FaviconService::Google => {
            let safe_host = escape_html(&norm);
            Some(format!(
                "<img class=\"tm-link-icon tm-link-favicon\" src=\"https://www.google.com/s2/favicons?domain={safe_host}&amp;sz=32\" alt=\"\" width=\"14\" height=\"14\" loading=\"lazy\" decoding=\"async\" />"
            ))
        }
        FaviconService::DuckDuckGo => {
            let safe_host = escape_html(&norm);
            Some(format!(
                "<img class=\"tm-link-icon tm-link-favicon\" src=\"https://icons.duckduckgo.com/ip3/{safe_host}.ico\" alt=\"\" width=\"14\" height=\"14\" loading=\"lazy\" decoding=\"async\" />"
            ))
        }
        FaviconService::None => None,
    }
}

/// Enhances external links in rendered HTML by inserting a site/brand icon prefix.
pub fn enhance_external_links(body_html: &str, config: &LinksConfig) -> String {
    if !config.external_icons {
        return body_html.to_string();
    }

    EXTERNAL_LINK_RE
        .replace_all(body_html, |caps: &regex::Captures| {
            let attrs = &caps[1];
            let raw_host = &caps[3];
            let inner_content = &caps[4];

            // Do not inject an icon if the link content already contains an image,
            // SVG, or icon element, or if it is empty.
            if inner_content.trim().is_empty()
                || inner_content.contains("<img")
                || inner_content.contains("<svg")
                || inner_content.contains("<tmt-icon")
            {
                return caps[0].to_string();
            }

            if let Some(icon_html) = resolve_link_icon(raw_host, config) {
                format!("<a{attrs}>{icon_html}{inner_content}</a>")
            } else {
                caps[0].to_string()
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhances_github_link_with_simple_icon() {
        let config = LinksConfig::default();
        let html = r#"<p><a class="tm-url" href="https://github.com/tomet-lang/tomet">GitHub</a></p>"#;
        let out = enhance_external_links(html, &config);
        assert_eq!(
            out,
            r#"<p><a class="tm-url" href="https://github.com/tomet-lang/tomet"><svg class="tm-link-icon" aria-hidden="true"><use href="/icons/simple.svg#github"></use></svg>GitHub</a></p>"#
        );
    }

    #[test]
    fn enhances_zenn_link_with_simple_icon() {
        let config = LinksConfig::default();
        let html = r#"<p><a class="tm-url" href="https://zenn.dev/articles/123">Zenn Article</a></p>"#;
        let out = enhance_external_links(html, &config);
        assert!(out.contains("/icons/simple.svg#zenn"));
    }

    #[test]
    fn enhances_unknown_domain_with_google_favicon() {
        let config = LinksConfig::default();
        let html = r#"<p><a class="tm-url" href="https://example.com/blog">My Blog</a></p>"#;
        let out = enhance_external_links(html, &config);
        assert!(out.contains(
            r#"<img class="tm-link-icon tm-link-favicon" src="https://www.google.com/s2/favicons?domain=example.com&amp;sz=32""#
        ));
    }

    #[test]
    fn enhances_unknown_domain_with_duckduckgo_favicon() {
        let config = LinksConfig {
            external_icons: true,
            note_icons: true,
            favicon_service: FaviconService::DuckDuckGo,
        };
        let html = r#"<p><a class="tm-url" href="https://example.com/blog">My Blog</a></p>"#;
        let out = enhance_external_links(html, &config);
        assert!(out.contains(
            r#"<img class="tm-link-icon tm-link-favicon" src="https://icons.duckduckgo.com/ip3/example.com.ico""#
        ));
    }

    #[test]
    fn leaves_links_alone_when_disabled() {
        let config = LinksConfig {
            external_icons: false,
            note_icons: true,
            favicon_service: FaviconService::Google,
        };
        let html = r#"<p><a class="tm-url" href="https://github.com/">GitHub</a></p>"#;
        let out = enhance_external_links(html, &config);
        assert_eq!(out, html);
    }

    #[test]
    fn does_not_add_icon_to_links_already_containing_images() {
        let config = LinksConfig::default();
        let html = r#"<p><a href="https://github.com/"><img src="badge.png" alt="badge"></a></p>"#;
        let out = enhance_external_links(html, &config);
        assert_eq!(out, html);
    }

    #[test]
    fn does_not_add_icon_when_favicon_service_none_and_not_known_brand() {
        let config = LinksConfig {
            external_icons: true,
            note_icons: true,
            favicon_service: FaviconService::None,
        };
        let html = r#"<p><a href="https://unknown-domain-123.com/">Link</a></p>"#;
        let out = enhance_external_links(html, &config);
        assert_eq!(out, html);
    }
}
