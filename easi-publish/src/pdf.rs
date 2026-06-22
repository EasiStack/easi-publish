//! PDF export (behind the `pdf` feature).
//!
//! Compiles a Typst template + data to PDF bytes, and the [`PdfConfig`] builder
//! for the PDF-only knobs (document identifier, standards, tagging, timestamp,
//! ...). The mirror of [`html`](crate::render_html) for the other output format;
//! the data plumbing ([`TemplateData`]) and the [`Clock`] source are shared.

use std::path::Path;

use chrono::{Datelike, Timelike, Utc};
use typst::foundations::Smart;
use typst::layout::PageRanges;
use typst_layout::PagedDocument;
use typst_pdf::{PdfStandard, PdfStandards, Timestamp};

use crate::clock::Clock;
use crate::data::DataSet;
use crate::diagnostics::to_diagnostics;
use crate::error::{PublishError, Result};
use crate::fonts::SharedFonts;
use crate::template::{PreparedTemplate, TemplateSource};
use crate::world::PublishWorld;

/// Render a Typst template with data into PDF bytes.
///
/// The PDF entry point. Given the same inputs, it produces the same output
/// (modulo timestamp if set to "now"). 
///
/// # Arguments
/// * `fonts` — Shared font resources (create once, reuse across calls)
/// * `template` — The Typst template source
/// * `data` — User data to fill into the template
/// * `config` — PDF output configuration
///
/// # Errors
/// Returns [`PublishError`] on IO failure, compilation error, or export error.
pub fn render_pdf(
    fonts: &SharedFonts,
    template: TemplateSource,
    data: DataSet,
    config: &PdfConfig,
) -> Result<Vec<u8>> {
    let prepared = PreparedTemplate::new(template)?;
    render_pdf_prepared(fonts, &prepared, data, config)
}

/// Render an already-parsed [`PreparedTemplate`] with data into PDF bytes.
///
/// Build the [`PreparedTemplate`] once and reuse it across many renders to skip
/// re-reading + re-parsing the template on every call. [`render_pdf`] is a
/// convenience that prepares the template inline.
///
/// # Errors
/// Returns [`PublishError`] on data conversion, compilation, or export failure.
pub fn render_pdf_prepared(
    fonts: &SharedFonts,
    template: &PreparedTemplate,
    data: DataSet,
    config: &PdfConfig,
) -> Result<Vec<u8>> {
    // Clone is cheap, `Source` is reference-counted and `root` is a PathBuf.
    let world = PublishWorld::new(
        template.root.clone(),
        template.source.clone(),
        fonts,
        data,
        config.clock,
        typst::Features::default(),
    )?;

    let warned = typst::compile::<PagedDocument>(&world);
    let doc = warned
        .output
        .map_err(|diags| PublishError::Compilation(to_diagnostics(&world, &diags)))?;

    let options = build_pdf_options(config)?;
    let pdf_bytes = typst_pdf::pdf(&doc, &options)
        .map_err(|diags| PublishError::Export(to_diagnostics(&world, &diags)))?;

    Ok(pdf_bytes)
}

/// Render a Typst template with data and write the PDF to a file.
///
/// Convenience wrapper around [`render_pdf`] that writes the output to disk.
pub fn render_pdf_to_file(
    fonts: &SharedFonts,
    template: TemplateSource,
    data: DataSet,
    config: &PdfConfig,
    output: &Path,
) -> Result<()> {
    let bytes = render_pdf(fonts, template, data, config)?;
    std::fs::write(output, &bytes)?;
    Ok(())
}

/// Configuration for PDF generation. Uses builder pattern.
///
/// All fields have sensible defaults (PDF 1.7, tagged, auto ident,
/// system-local clock).
#[derive(Clone)]
pub struct PdfConfig {
    ident: Smart<String>,
    creator: Smart<Option<String>>,
    timestamp: Option<Timestamp>,
    page_ranges: Option<PageRanges>,
    standards: Vec<PdfStandard>,
    tagged: bool,
    pretty: bool,
    pub(crate) clock: Clock,
}

impl PdfConfig {
    /// A config with the default settings (PDF 1.7, tagged, auto ident,
    /// system-local clock, no timestamp).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a fixed document identifier (otherwise derived automatically from
    /// the content).
    #[must_use]
    pub fn ident(mut self, ident: impl Into<String>) -> Self {
        self.ident = Smart::Custom(ident.into());
        self
    }

    /// Set the document's producer/creator string written into the PDF metadata
    /// (otherwise Typst fills in its own default). 
    #[must_use]
    pub fn creator(mut self, creator: impl Into<String>) -> Self {
        self.creator = Smart::Custom(Some(creator.into()));
        self
    }

    /// Emit human-readable (un-minified) PDF output. Default is `false`
    /// (minified), which is smaller. Enable this only to make
    /// the generated PDF easier to inspect or diff while debugging templates.
    #[must_use]
    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Set the document's creation timestamp.
    #[must_use]
    pub fn timestamp(mut self, ts: Timestamp) -> Self {
        self.timestamp = Some(ts);
        self
    }

    /// Set the creation timestamp to the current UTC time.
    #[must_use]
    pub fn timestamp_now_utc(self) -> Self {
        let now = Utc::now();
        if let Some(dt) = typst::foundations::Datetime::from_ymd_hms(
            now.year(),
            now.month().try_into().unwrap_or(1),
            now.day().try_into().unwrap_or(1),
            now.hour().try_into().unwrap_or(0),
            now.minute().try_into().unwrap_or(0),
            now.second().try_into().unwrap_or(0),
        ) {
            Self {
                timestamp: Some(Timestamp::new_utc(dt)),
                ..self
            }
        } else {
            self
        }
    }

    /// Restrict output to specific page ranges (default: all pages).
    #[must_use]
    pub fn page_ranges(mut self, ranges: PageRanges) -> Self {
        self.page_ranges = Some(ranges);
        self
    }

    /// Set the PDF standards to conform to (e.g. PDF/A).
    ///
    /// Multiple compatible standards can be targeted at once, e.g. an accessible 
    /// and archival document:
    ///
    /// ```
    /// use easi_publish::{PdfConfig, PdfStandard};
    /// let config = PdfConfig::new().standards(&[PdfStandard::Ua_1, PdfStandard::A_2a]);
    /// ```
    ///
    /// Incompatible combinations (e.g. conflicting PDF versions) are rejected at
    /// render time with a [`PublishError::InvalidStandards`](crate::PublishError)
    /// whose message carries Typst's explanation.
    #[must_use]
    pub fn standards(mut self, standards: &[PdfStandard]) -> Self {
        self.standards = standards.to_vec();
        self
    }

    /// Target the common accessible and archival combination
    /// (PDF/UA-1 + PDF/A-2a) in one call. Shorthand for
    /// [`standards`](Self::standards) with that pair.
    #[must_use]
    pub fn archival_accessible(self) -> Self {
        self.standards(&[PdfStandard::Ua_1, PdfStandard::A_2a])
    }

    /// Enable or disable tagged (accessible) PDF output (default: enabled).
    #[must_use]
    pub fn tagged(mut self, tagged: bool) -> Self {
        self.tagged = tagged;
        self
    }

    /// Set the clock source for `datetime.today()`. Use
    /// [`Clock::Fixed`] for reproducible output.
    #[must_use]
    pub fn clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            ident: Smart::Auto,
            creator: Smart::Auto,
            timestamp: None,
            page_ranges: None,
            standards: Vec::new(),
            tagged: true,
            pretty: false,
            clock: Clock::SystemLocal,
        }
    }
}

fn build_pdf_options(config: &PdfConfig) -> Result<typst_pdf::PdfOptions> {    
    let ident = config.ident.clone();

    let standards = if config.standards.is_empty() {
        PdfStandards::default()
    } else {
        // `PdfStandards::new` now returns a `HintedString` error, which doesn't
        // implement `Display`. Surface its message text plus any hints Typst
        // attaches (e.g. why two standards conflict).
        PdfStandards::new(&config.standards).map_err(|e| {
            let mut msg = e.message().to_string();
            for hint in e.hints() {
                msg.push_str("\n  hint: ");
                msg.push_str(hint);
            }
            PublishError::InvalidStandards(msg)
        })?
    };

    Ok(typst_pdf::PdfOptions {
        ident,
        creator: config.creator.clone(),
        timestamp: config.timestamp,
        page_ranges: config.page_ranges.clone(),
        standards,
        tagged: config.tagged,
        pretty: config.pretty,
    })
}
