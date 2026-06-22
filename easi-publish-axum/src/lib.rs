#![warn(missing_docs)]
//! # easi-publish-axum
//!
//! Axum integration for the `easi-publish` crate.
//!
//! Provides response wrappers that implement [`axum::response::IntoResponse`]:
//! - [`PdfResponse`] (the `pdf` feature) — `Content-Type: application/pdf`.
//! - [`HtmlResponse`] (the `html` feature) — `Content-Type: text/html`.
//!
//! Plus [`PublishErrorResponse`], an [`IntoResponse`](axum::response::IntoResponse)
//! for [`PublishError`] that returns 500 with a JSON body.
//!
//! The `pdf` / `html` features mirror `easi-publish` (PDF on by default) and are
//! forwarded to it. All types from `easi-publish` are re-exported for
//! convenience.

// Re-export everything from easi-publish so you only need one dependency.
pub use easi_publish::*;

mod error;
#[cfg(feature = "html")]
mod html;
#[cfg(feature = "pdf")]
mod pdf;

// Shared response helpers (filename sanitisation). Only needed when a response
// wrapper is compiled in.
#[cfg(any(feature = "pdf", feature = "html"))]
mod util;

pub use error::PublishErrorResponse;
#[cfg(feature = "html")]
pub use html::HtmlResponse;
#[cfg(feature = "pdf")]
pub use pdf::PdfResponse;
