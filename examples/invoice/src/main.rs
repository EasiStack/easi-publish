mod invoice;
mod routes;
mod state;
mod validation;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use easi_publish_axum::{PreparedTemplate, Renderer, SharedFonts, TemplateSource};
use tower_http::services::ServeDir;

use crate::routes::{create_invoice, index};
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Build the renderer once: fonts scanned a single time, each template parsed
    // once up front, and concurrent renders bounded to the CPU count.
    let cpus = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    let mut renderer = Renderer::new(SharedFonts::new()).with_concurrency_limit(cpus);

    let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
    for (key, file) in [
        ("plain", "invoice.typ"),
        ("logo", "invoice-logo.typ"),
        ("logo-small", "invoice-logo-small.typ"),
    ] {
        let template = PreparedTemplate::new(TemplateSource::FilePath(template_dir.join(file)))
            .unwrap_or_else(|e| panic!("parse template {file}: {e}"));
        renderer.register_template(key, template);
    }

    renderer.warm_up();
    tracing::info!("Renderer ready (fonts + templates loaded; PDF + HTML enabled)");

    let state = AppState {
        renderer: Arc::new(renderer),
    };

    let router = Router::new()
        .route("/", get(index))
        .route("/api/invoice", post(create_invoice))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!("Invoice app listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, router)
        .await
        .expect("Failed to start server");
}
