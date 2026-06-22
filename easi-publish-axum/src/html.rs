//! [`HtmlResponse`] an axum response wrapper for HTML output (behind the
//! `html` feature).

use axum::response::{IntoResponse, Response};
use http::{HeaderMap, HeaderValue, StatusCode, header};

use crate::util::sanitize_filename;

/// An HTML response wrapper for axum handlers.
///
/// Sets `Content-Type: text/html; charset=utf-8`. Inline (rendered in the
/// browser) by default; call [`filename`](Self::filename) to send it as a
/// download instead.
///
/// # Example
/// ```ignore
/// async fn handler(State(state): State<Arc<AppState>>) -> Result<HtmlResponse, PublishErrorResponse> {
///     Ok(HtmlResponse::from_renderer(&state.renderer, "invoice", my_data).await?)
/// }
/// ```
pub struct HtmlResponse {
    html: String,
    filename: Option<String>,
}

impl HtmlResponse {
    /// Create an `HtmlResponse` from an HTML string.
    #[must_use]
    pub fn new(html: String) -> Self {
        Self {
            html,
            filename: None,
        }
    }

    /// Render a template into an `HtmlResponse` in one step.
    ///
    /// # Errors
    /// Propagates the render error (compilation/export failure, invalid data).
    pub fn render(
        fonts: &easi_publish::SharedFonts,
        template: easi_publish::TemplateSource,
        data: easi_publish::DataSet,
        config: &easi_publish::HtmlConfig,
    ) -> easi_publish::Result<Self> {
        let html = easi_publish::render_html(fonts, template, data, config)?;
        Ok(Self::new(html))
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
        renderer.render_html_async(key, data).await.map(Self::new)
    }

    /// Send the HTML as a download with the given filename rather than rendering
    /// it inline.
    ///
    /// Produces `Content-Disposition: attachment; filename="name"`.
    #[must_use]
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }
}

impl IntoResponse for HtmlResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        // Default (no filename) is inline, the browser renders it. A filename
        // turns it into a download. The sanitized name is ASCII, so building the
        // header value shouldn't fail in practice.
        if let Some(ref name) = self.filename {
            if let Ok(value) = HeaderValue::from_str(&format!(
                "attachment; filename=\"{}\"",
                sanitize_filename(name)
            )) {
                headers.insert(header::CONTENT_DISPOSITION, value);
            }
        }

        (StatusCode::OK, headers, self.html).into_response()
    }
}
