# `easi-publish-axum`

[Axum](https://github.com/tokio-rs/axum) integration for
[`easi-publish`](https://crates.io/crates/easi-publish): render Typst templates into PDF
or HTML HTTP responses.

It re-exports all of `easi-publish` (so you only need this one dependency) and adds:

- **`PdfResponse`** *(`pdf` feature, default)* — implements `IntoResponse`, sets
  `Content-Type: application/pdf` and a configurable `Content-Disposition`
  (`attachment` with a sanitized filename, or `inline`).
- **`HtmlResponse`** *(`html` feature)* — implements `IntoResponse`, sets
  `Content-Type: text/html; charset=utf-8` (inline by default, or a download via
  `filename`).
- **`{Pdf,Html}Response::from_renderer(renderer, key, data)`** — the recommended
  path: renders a registered template via a shared `Renderer` asynchronously (the
  blocking compile is offloaded and bounded by the renderer's concurrency
  limit).
- **`PublishErrorResponse`** — maps a `PublishError` to a `500` JSON response.

The `pdf` / `html` features mirror `easi-publish` (PDF on by default) and are
forwarded to it.

## Example

```rust,no_run
use std::sync::Arc;
use axum::extract::State;
use axum::response::IntoResponse;
use easi_publish_axum::{Renderer, PdfResponse, PreparedTemplate, SharedFonts, TemplateSource, DataSet};

#[derive(Clone)]
struct AppState { renderer: Arc<Renderer> }

async fn invoice(State(state): State<AppState>) -> axum::response::Response {    
    let data = match DataSet::from_json_file("invoice.json", serde_json::json!({ "n": 1 })) {
        Ok(data) => data,
        Err(e) => return format!("bad data: {e}").into_response(),
    };
    match PdfResponse::from_renderer(&state.renderer, "invoice", data).await {
        Ok(pdf) => pdf.filename("invoice.pdf").into_response(),
        Err(e) => format!("render failed: {e}").into_response(),
    }
}

// At startup: build the renderer once, register templates, store in state.
fn build_state() -> AppState {
    let mut renderer = Renderer::new(SharedFonts::embedded_only());
    renderer.register_template(
        "invoice",
        PreparedTemplate::new(TemplateSource::FilePath("templates/invoice.typ".into())).unwrap(),
    );
    AppState { renderer: Arc::new(renderer) }
}
```

See the `examples/invoice` app in the repository for a complete server.

## Performance note

PDF generation depends on Typst, which is compute-heavy. Build in release (or
add `[profile.dev.package."*"] opt-level = 3` to your workspace) for fast
rendering, and embed images at display resolution. See `easi-publish`'s docs and
`easi_publish::downscale_image` if you need to downscale images.

## License

Apache-2.0.
[Typst]: https://typst.app/