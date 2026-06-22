//! HTML export (behind the `html` feature).
//!
//! Compiles the same Typst templates into an HTML string instead. 
//! Mathematical equations are exported to **MathML** automatically (no template 
//! changes required), preserving their semantics for accessibility. 
//! The Typst `Library` is built with `Feature::Html` enabled, so
//! templates may also use the `html` module (`html.elem`, `html.frame`, ...).
//!
//! The data plumbing ([`TemplateData`]) and the [`Clock`] source are shared with
//! the PDF path. PDF-only knobs (document identifier, PDF standards, tagging,
//! timestamp) don't apply here, so HTML has its own small [`HtmlConfig`].

use typst::{Feature, Features};
use typst_html::{HtmlDocument, HtmlOptions};

use crate::clock::Clock;
use crate::data::DataSet;
use crate::diagnostics::to_diagnostics;
use crate::error::{PublishError, Result};
use crate::fonts::SharedFonts;
use crate::template::{PreparedTemplate, TemplateSource};
use crate::world::PublishWorld;

/// Configuration for HTML generation. Uses builder pattern.
///
/// Far smaller than [`PdfConfig`](crate::PdfConfig), only the knobs that make
/// sense for HTML output: a [`Clock`] for `datetime.today()` and whether to
/// pretty-print.
#[derive(Clone)]
pub struct HtmlConfig {
    pretty: bool,
    pub(crate) clock: Clock,
}

impl HtmlConfig {
    /// A config with the default settings (minified output, system-local clock).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Emit human-readable (un-minified) HTML. Default is `false` (minified, the
    /// 0.15 default). Enable this for readable/diffable output while developing.
    #[must_use]
    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Set the clock source for `datetime.today()`. Use [`Clock::Fixed`] for
    /// reproducible output.
    #[must_use]
    pub fn clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }
}

impl Default for HtmlConfig {
    fn default() -> Self {
        Self {
            pretty: false,
            clock: Clock::SystemLocal,
        }
    }
}

/// Render a Typst template with data into an HTML string.
///
/// The HTML counterpart of [`render_pdf`](crate::render_pdf). Equations become MathML.
///
/// # Errors
/// Returns [`PublishError`] on IO failure, compilation error, or HTML export error.
pub fn render_html(
    fonts: &SharedFonts,
    template: TemplateSource,
    data: DataSet,
    config: &HtmlConfig,
) -> Result<String> {
    let prepared = PreparedTemplate::new(template)?;
    render_html_prepared(fonts, &prepared, data, config)
}

/// Render an already-parsed [`PreparedTemplate`] with data into an HTML string.
///
/// The HTML counterpart of [`render_pdf_prepared`](crate::render_pdf_prepared). Build
/// the template once and reuse it across many renders.
///
/// # Errors
/// Returns [`PublishError`] on data conversion, compilation, or HTML export failure.
pub fn render_html_prepared(
    fonts: &SharedFonts,
    template: &PreparedTemplate,
    data: DataSet,
    config: &HtmlConfig,
) -> Result<String> {
    let world = PublishWorld::new(
        template.root.clone(),
        template.source.clone(),
        fonts,
        data,
        config.clock,
        // HTML export requires the `html` library feature so `html.*` is defined.
        Features::from_iter([Feature::Html]),
    )?;

    let warned = typst::compile::<HtmlDocument>(&world);
    let doc = warned
        .output
        .map_err(|diags| PublishError::Compilation(to_diagnostics(&world, &diags)))?;

    let options = HtmlOptions {
        pretty: config.pretty,
    };
    typst_html::html(&doc, &options)
        .map_err(|diags| PublishError::HtmlExport(to_diagnostics(&world, &diags)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataSet;

    fn fonts() -> SharedFonts {
        SharedFonts::embedded_only()
    }

    #[test]
    fn renders_basic_html() {
        let html = render_html(
            &fonts(),
            TemplateSource::InMemory {
                content: "= Title\n\nHello *world*.".to_owned(),
                root: None,
            },
            DataSet::from_inputs(serde_json::json!({})).unwrap(),
            &HtmlConfig::default(),
        )
        .unwrap();
        assert!(html.contains("<html"), "expected an html document: {html}");
        assert!(html.contains("Hello"), "expected body text: {html}");
    }

    #[test]
    fn equations_become_mathml() {
        let html = render_html(
            &fonts(),
            TemplateSource::InMemory {
                content: "$ a^2 + b^2 = c^2 $".to_owned(),
                root: None,
            },
            DataSet::from_inputs(serde_json::json!({})).unwrap(),
            &HtmlConfig::default(),
        )
        .unwrap();
        assert!(html.contains("<math"), "equation should export as MathML: {html}");
    }

    #[test]
    fn sys_inputs_reach_the_template() {
        let html = render_html(
            &fonts(),
            TemplateSource::InMemory {
                content: "#sys.inputs.name".to_owned(),
                root: None,
            },
            DataSet::from_inputs(serde_json::json!({ "name": "Acme" })).unwrap(),
            &HtmlConfig::default(),
        )
        .unwrap();
        assert!(html.contains("Acme"), "expected injected data: {html}");
    }

    #[test]
    fn compilation_error_surfaces() {
        let err = render_html(
            &fonts(),
            TemplateSource::InMemory {
                content: "#let x = ".to_owned(),
                root: None,
            },
            DataSet::from_inputs(serde_json::json!({})).unwrap(),
            &HtmlConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, PublishError::Compilation(_)), "got {err:?}");
    }
}
