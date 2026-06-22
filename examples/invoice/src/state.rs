use std::sync::Arc;

use easi_publish_axum::Renderer;

#[derive(Clone)]
pub struct AppState {
    pub renderer: Arc<Renderer>,
}
