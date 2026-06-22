//! [`PublishErrorResponse`] maps a [`PublishError`](easi_publish::PublishError)
//! to a 500 JSON response.

use axum::response::{IntoResponse, Response};
use http::{StatusCode, header};

/// Wrapper around [`PublishError`](easi_publish::PublishError) that implements
/// [`IntoResponse`].
///
/// Returns a 500 response with a JSON error body. Use this as your handler's
/// error type: `Result<PdfResponse, PublishErrorResponse>` (or `HtmlResponse`).
pub struct PublishErrorResponse(pub easi_publish::PublishError);

impl From<easi_publish::PublishError> for PublishErrorResponse {
    fn from(err: easi_publish::PublishError) -> Self {
        Self(err)
    }
}

impl std::fmt::Display for PublishErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoResponse for PublishErrorResponse {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "document rendering failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json".to_owned())],
            serde_json::json!({ "error": "rendering failed" }).to_string(),
        )
            .into_response()
    }
}
