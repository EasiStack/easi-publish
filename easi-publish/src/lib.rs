#![warn(missing_docs)]
//! # easi-publish
//!
//! Compile Typst templates with user data into PDF or HTML documents.
//!
//! ## Outputs are opt-in features
//!
//! Two output backends live behind cargo features, so you pull only what you
//! use:
//! - `pdf` (**default**) — [`render_pdf`] and friends, plus [`PdfConfig`].
//! - `html` — [`render_html`] and friends, plus [`HtmlConfig`]. Equations are
//!   exported to MathML automatically.
//!
//! Enable both with `features = ["pdf", "html"]`, or take HTML only with
//! `default-features = false, features = ["html"]`.
//!
//! ## Quick start
//!
//! ```no_run
//! # #[cfg(feature = "pdf")] {
//! use easi_publish::{SharedFonts, TemplateSource, DataSet, PdfConfig, render_pdf};
//!
//! let fonts = SharedFonts::new();
//! let template = TemplateSource::FilePath("templates/invoice.typ".into());
//! let data = DataSet::from_json_file(
//!     "invoice.json",
//!     serde_json::json!({"invoice_number": "INV-001"}),
//! ).unwrap();
//! let pdf_bytes = render_pdf(&fonts, template, data, &PdfConfig::default()).unwrap();
//! # }
//! ```
//!
//! ## Trust model
//!
//! This crate is built for app-authored (trusted) templates with untrusted data.
//! The pattern is a fixed template plus user data supplied as a [`DataSet`]
//! (`sys.inputs` and/or virtual files), which is the recommended safe shape. 
//! 
//! It is not recommended to render user-supplied templates here: there is no compile 
//! timeout or memory bound (a! template can loop or allocate without limit), and a 
//! template can read any file under its root. File reads are confined to the template's 
//! root, package imports (`@preview/...`) are denied, and there is no network access. 
//! Set the template root to `None` (an in-memory template with no root) to deny all
//! disk reads. Safely rendering untrusted templates requires process isolation
//! and is currently out of scope.

mod clock;
mod data;
mod error;
mod fonts;

#[cfg(feature = "image")]
mod image_util;

mod renderer;
mod template;

#[cfg(feature = "html")]
mod html;
#[cfg(feature = "pdf")]
mod pdf;

// Diagnostic mapping and the Typst `World` impl back both output backends, skip
// them entirely when neither is enabled.
#[cfg(any(feature = "pdf", feature = "html"))]
mod diagnostics;
#[cfg(any(feature = "pdf", feature = "html"))]
mod world;

pub use clock::Clock;
#[cfg(feature = "pdf")]
pub use pdf::{PdfConfig, render_pdf, render_pdf_prepared, render_pdf_to_file};
pub use data::DataSet;
pub use error::{Diagnostic, Hint, PublishError, Result, Severity};
pub use fonts::SharedFonts;

#[cfg(feature = "html")]
pub use html::{HtmlConfig, render_html, render_html_prepared};
#[cfg(feature = "image")]
pub use image_util::downscale_image;

pub use renderer::Renderer;
pub use template::{PreparedTemplate, TemplateSource};

// `Dict` lets callers pass a pre-built typst dictionary as data (no feature
// needed). The PDF-config types come from the PDF backend, so they're gated.
pub use typst::foundations::Dict;
#[cfg(feature = "pdf")]
pub use typst::layout::PageRanges;
#[cfg(feature = "pdf")]
pub use typst_pdf::{PdfStandard, Timestamp, Timezone};

/// Evict Typst's memoization-cache entries unused for the last `max_age` calls
/// to this function.
///
/// Typst caches compilation work (parsing, layout, image decode, ...) in a
/// process-global cache that otherwise grows unbounded. Long-running servers
/// should call this periodically, e.g. every N renders or on a timer to bound
/// memory. `evict_cache(0)` clears the cache entirely. A small value like
/// `evict_cache(10)` keeps recently-used entries (useful when the same template
/// or images recur, so they aren't re-decoded immediately).
pub fn evict_cache(max_age: usize) {
    comemo::evict(max_age);
}
