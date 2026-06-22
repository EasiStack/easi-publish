//! HTTP route handlers.

use std::time::Instant;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use easi_publish_axum::{DataSet, HtmlResponse, PdfResponse};
use serde::Deserialize;

use crate::invoice::InvoiceData;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "page.html")]
struct PageTemplate;

pub async fn index() -> Response {
    match PageTemplate.render() {
        Ok(html) => (StatusCode::OK, Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Query parameters for the render endpoint:
/// - `?template=` selects a template variant — compare them live and watch the
///   `elapsed_ms` in the server log: `plain` (no image), `logo` (the 54 MP
///   source — slow first render + ~1 MB output), `logo-small` (downscaled —
///   default, fast + small).
/// - `?format=` selects the output: `pdf` (default) or `html`. The same Typst
///   template renders to either; HTML export turns equations into MathML.
#[derive(Deserialize)]
pub struct RenderQuery {
    template: Option<String>,
    format: Option<String>,
}

pub async fn create_invoice(
    State(state): State<AppState>,
    Query(query): Query<RenderQuery>,
    Json(invoice): Json<InvoiceData>,
) -> Response {
    // Allow-list the template key (templates are registered once at startup).
    let key = match query.template.as_deref() {
        Some("plain") => "plain",
        Some("logo") => "logo",
        None | Some("logo-small") => "logo-small",
        Some(other) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("unknown template '{other}' (use plain | logo | logo-small)"),
            )
                .into_response();
        }
    };

    // Same template + data, two output formats. The renderer owns the
    // blocking-compile offload + concurrency limit, so each arm is a one-liner.
    let data = match DataSet::from_json_file("invoice.json", invoice) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(error = %e, "failed to encode invoice data");
            return (StatusCode::BAD_REQUEST, "invalid invoice data").into_response();
        }
    };
    let started = Instant::now();
    match query.format.as_deref() {
        None | Some("pdf") => {
            match PdfResponse::from_renderer(&state.renderer, key, data).await {
                Ok(pdf) => {
                    tracing::info!(
                        template = key,
                        format = "pdf",
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "rendered invoice"
                    );
                    pdf.filename("invoice.pdf").into_response()
                }
                Err(e) => {
                    tracing::error!(error = %e, "PDF render failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "PDF generation failed").into_response()
                }
            }
        }
        Some("html") => match HtmlResponse::from_renderer(&state.renderer, key, data).await {
            Ok(html) => {
                tracing::info!(
                    template = key,
                    format = "html",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "rendered invoice"
                );
                // Inline by default, so the browser renders it directly.
                html.into_response()
            }
            Err(e) => {
                tracing::error!(error = %e, "HTML render failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "HTML generation failed").into_response()
            }
        },
        Some(other) => (
            StatusCode::BAD_REQUEST,
            format!("unknown format '{other}' (use pdf | html)"),
        )
            .into_response(),
    }
}
