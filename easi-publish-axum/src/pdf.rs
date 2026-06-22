//! [`PdfResponse`] an axum response wrapper for PDF output (behind the `pdf`
//! feature).

use axum::response::{IntoResponse, Response};
use http::{StatusCode, header};

use crate::util::sanitize_filename;

/// A PDF response wrapper for axum handlers.
///
/// Sets `Content-Type: application/pdf` and a configurable `Content-Disposition`
/// (attachment by default; [`inline`](Self::inline) to display in-browser).
///
/// # Example
/// ```ignore
/// async fn handler(State(state): State<Arc<AppState>>) -> Result<PdfResponse, PublishErrorResponse> {
///     let pdf = PdfResponse::from_renderer(&state.renderer, "invoice", my_data).await?;
///     Ok(pdf.filename("invoice.pdf"))
/// }
/// ```
pub struct PdfResponse {
    bytes: Vec<u8>,
    filename: Option<String>,
    inline: bool,
}

impl PdfResponse {
    /// Create a `PdfResponse` from raw PDF bytes.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            filename: None,
            inline: false,
        }
    }

    /// Render a template into a `PdfResponse` in one step.
    ///
    /// # Errors
    /// Propagates the render error (compilation/export failure, invalid data).
    pub fn render(
        fonts: &easi_publish::SharedFonts,
        template: easi_publish::TemplateSource,
        data: easi_publish::DataSet,
        config: &easi_publish::PdfConfig,
    ) -> easi_publish::Result<Self> {
        let bytes = easi_publish::render_pdf(fonts, template, data, config)?;
        Ok(Self::new(bytes))
    }

    /// Render via a shared [`Renderer`](easi_publish::Renderer) into a response.
    /// async, bounded by the renderer's concurrency limit, with the blocking
    /// compile offloaded. The recommended path for handlers.
    ///
    /// # Errors
    /// Propagates the render error (e.g. unknown template key, compilation
    /// failure) for the handler to map to a response.
    pub async fn from_renderer(
        renderer: &easi_publish::Renderer,
        key: &str,
        data: easi_publish::DataSet,
    ) -> easi_publish::Result<Self> {
        renderer.render_pdf_async(key, data).await.map(Self::new)
    }

    /// Set the download filename.
    ///
    /// Produces `Content-Disposition: attachment; filename="name"`.
    #[must_use]
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self.inline = false;
        self
    }

    /// Display inline in the browser rather than downloading.
    ///
    /// Produces `Content-Disposition: inline`.
    #[must_use]
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

impl IntoResponse for PdfResponse {
    fn into_response(self) -> Response {
        let disposition = if self.inline {
            "inline".to_owned()
        } else if let Some(ref name) = self.filename {
            format!("attachment; filename=\"{}\"", sanitize_filename(name))
        } else {
            "attachment".to_owned()
        };

        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/pdf".to_owned()),
                (header::CONTENT_DISPOSITION, disposition),
            ],
            self.bytes,
        )
            .into_response()
    }
}
