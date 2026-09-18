use anyhow::{Context, Result};
use minijinja::{Environment, Value, context};
use serde::{Deserialize, Serialize};

use super::document::ProcessedDoc;
use crate::config::BookConfig;

pub struct BookRenderer<'a> {
    env: Environment<'a>,
    config: &'a BookConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionSummary {
    pub name: String,
    pub count: usize,
}

/// A page that points at the one being rendered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    pub url: String,
    pub title: String,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySummary {
    pub url: String,
    pub title: String,
    pub section: Option<String>,
    /// Pages the index nested under this one. Always empty when the catalog
    /// is generated rather than written.
    #[serde(default)]
    pub children: Vec<EntrySummary>,
}

impl<'a> BookRenderer<'a> {
    pub fn new(config: &'a BookConfig, is_dev: bool) -> Result<Self> {
        let mut env = Environment::new();

        env.add_template(
            "base.html",
            include_str!("../../../../ui/templates/base.html"),
        )
        .context("Failed to add base.html template")?;
        env.add_template(
            "page.html",
            include_str!("../../../../ui/templates/page.html"),
        )
        .context("Failed to add page.html template")?;
        env.add_template(
            "index.html",
            include_str!("../../../../ui/templates/index.html"),
        )
        .context("Failed to add index.html template")?;
        env.add_template(
            "macros.html",
            include_str!("../../../../ui/templates/macros.html"),
        )
        .context("Failed to add macros.html template")?;

        let rail_title = config
            .ui
            .rail_title
            .as_deref()
            .unwrap_or(&config.book.title);

        let rail_letters: Vec<String> = rail_title.chars().map(|c| c.to_string()).collect();

        // Vault-wide invariant values registered once as globals rather than
        // rebuilt per-page in context!().
        env.add_global("site_title", Value::from(&config.book.title));
        env.add_global(
            "description",
            Value::from_serialize(&config.book.description),
        );
        env.add_global("lang", Value::from(&config.book.lang));
        env.add_global("default_view", Value::from(&config.ui.default_view));
        env.add_global("custom_css", Value::from_serialize(&config.ui.custom_css));
        env.add_global("t", Value::from_serialize(&config.ui.strings));
        env.add_global(
            "css_version",
            Value::from(super::assets::CSS_VERSION.as_str()),
        );
        env.add_global(
            "js_version",
            Value::from(super::assets::JS_VERSION.as_str()),
        );
        env.add_global(
            "tmt_btn_css_version",
            Value::from(super::assets::TMT_BTN_CSS_VERSION.as_str()),
        );
        env.add_global(
            "tmt_badge_css_version",
            Value::from(super::assets::TMT_BADGE_CSS_VERSION.as_str()),
        );
        env.add_global(
            "tmt_icon_css_version",
            Value::from(super::assets::TMT_ICON_CSS_VERSION.as_str()),
        );
        env.add_global(
            "tmt_swatch_css_version",
            Value::from(super::assets::TMT_SWATCH_CSS_VERSION.as_str()),
        );
        env.add_global(
            "tmt_switch_css_version",
            Value::from(super::assets::TMT_SWITCH_CSS_VERSION.as_str()),
        );
        env.add_global("is_dev", Value::from(is_dev));
        env.add_global("rail_letters", Value::from_serialize(&rail_letters));

        Ok(Self { env, config })
    }

    pub fn render_page(
        &self,
        doc: &ProcessedDoc,
        backlinks: &[Backlink],
        book_index: &[EntrySummary],
    ) -> Result<String> {
        let tmpl = self.env.get_template("page.html")?;

        let ctx = context! {
            source_path => &doc.source_path,
            page_title => &doc.title,
            section => &doc.section,
            kind => &doc.kind,
            primary_color => &doc.primary_color,
            icon => &doc.icon,
            icon_image_url => &doc.icon_image_url,
            banner_url => &doc.banner_url,
            banner_original_url => &doc.banner_original_url,
            banner_y => doc.banner_y,
            images => &doc.images,
            original_images => &doc.original_images,
            hero_chips => &doc.hero_chips,
            infobox_rows => &doc.infobox_rows,
            has_data => doc.has_data,
            toc => &doc.toc,
            section_tabs => &doc.section_tabs,
            body_html => &doc.body_html,
            backlinks => backlinks,
            book_index => book_index,
            // So the index can mark where the reader is standing.
            page_url => format!("{}/{}", self.config.build.clean_url_prefix(), doc.slug),
        };

        let rendered = tmpl.render(ctx)?;
        Ok(rendered)
    }

    pub fn render_index(
        &self,
        sections: &[SectionSummary],
        entries: &[EntrySummary],
    ) -> Result<String> {
        let tmpl = self.env.get_template("index.html")?;

        let ctx = context! {
            sections => sections,
            entries => entries,
        };

        let rendered = tmpl.render(ctx)?;
        Ok(rendered)
    }
}
