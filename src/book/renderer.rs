use anyhow::{Context, Result};
use minijinja::{Environment, context};
use serde::{Deserialize, Serialize};

use super::document::ProcessedDoc;
use crate::config::BookConfig;

pub struct BookRenderer<'a> {
    env: Environment<'a>,
    config: &'a BookConfig,
    rail_letters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionSummary {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySummary {
    pub url: String,
    pub title: String,
    pub section: Option<String>,
}

impl<'a> BookRenderer<'a> {
    pub fn new(config: &'a BookConfig) -> Result<Self> {
        let mut env = Environment::new();

        env.add_template("base.html", include_str!("../assets/templates/base.html"))
            .context("Failed to add base.html template")?;
        env.add_template("page.html", include_str!("../assets/templates/page.html"))
            .context("Failed to add page.html template")?;
        env.add_template("index.html", include_str!("../assets/templates/index.html"))
            .context("Failed to add index.html template")?;

        let rail_title = config
            .ui
            .rail_title
            .as_deref()
            .unwrap_or(&config.book.title);

        let rail_letters: Vec<String> = rail_title.chars().map(|c| c.to_string()).collect();

        Ok(Self {
            env,
            config,
            rail_letters,
        })
    }

    pub fn render_page(&self, doc: &ProcessedDoc) -> Result<String> {
        let tmpl = self.env.get_template("page.html")?;

        let ctx = context! {
            site_title => &self.config.book.title,
            lang => &self.config.book.lang,
            default_view => &self.config.ui.default_view,
            custom_css => &self.config.ui.custom_css,
            t => &self.config.ui.strings,
            css_version => &*super::assets::CSS_VERSION,
            js_version => &*super::assets::JS_VERSION,
            rail_letters => &self.rail_letters,
            source_path => &doc.source_path,
            page_title => &doc.title,
            section => &doc.section,
            kind => &doc.kind,
            primary_color => &doc.primary_color,
            icon => &doc.icon,
            banner_url => &doc.banner_url,
            banner_y => doc.banner_y,
            images => &doc.images,
            hero_chips => &doc.hero_chips,
            infobox_rows => &doc.infobox_rows,
            has_data => doc.has_data,
            toc => &doc.toc,
            section_tabs => &doc.section_tabs,
            body_html => &doc.body_html,
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
            site_title => &self.config.book.title,
            description => &self.config.book.description,
            lang => &self.config.book.lang,
            default_view => &self.config.ui.default_view,
            custom_css => &self.config.ui.custom_css,
            t => &self.config.ui.strings,
            css_version => &*super::assets::CSS_VERSION,
            js_version => &*super::assets::JS_VERSION,
            sections => sections,
            entries => entries,
        };

        let rendered = tmpl.render(ctx)?;
        Ok(rendered)
    }
}
