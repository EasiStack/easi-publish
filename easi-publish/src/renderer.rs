//! [`Renderer`] an ergonomic front door that bundles the pieces you'd
//! otherwise wire up by hand: shared fonts (built once), default per-format
//! configs, a cache of pre-parsed templates, optional bounded concurrency, and
//! (with the `tokio` feature) async renders that offload the blocking compile.
//!
//! It composes the free functions ([`render_pdf_prepared`](crate::render_pdf_prepared),
//! [`render_html_prepared`](crate::render_html_prepared)). Reach for those
//! directly for one-off renders or full control.

use std::collections::HashMap;

use crate::{DataSet, PreparedTemplate, PublishError, Result, SharedFonts};
#[cfg(feature = "html")]
use crate::{HtmlConfig, render_html_prepared};
#[cfg(feature = "pdf")]
use crate::{PdfConfig, render_pdf_prepared};

/// Owns the shared render state for an application and renders registered
/// templates by key, to PDF and/or HTML.
///
/// Build once (e.g. into your app state), register your templates up front, and
/// render many times:
///
/// ```no_run
/// # #[cfg(feature = "pdf")] {
/// # use easi_publish::{Renderer, PreparedTemplate, SharedFonts, TemplateSource, DataSet};
/// let mut renderer = Renderer::new(SharedFonts::embedded_only());
/// renderer.register_template(
///     "invoice",
///     PreparedTemplate::new(TemplateSource::FilePath("invoice.typ".into())).unwrap(),
/// );
/// let data = DataSet::from_inputs(serde_json::json!({ "n": 1 })).unwrap();
/// let pdf = renderer.render_pdf("invoice", data).unwrap();
/// # }
/// ```
pub struct Renderer {
    fonts: SharedFonts,
    templates: HashMap<&'static str, PreparedTemplate>,
    #[cfg(feature = "pdf")]
    default_pdf_config: PdfConfig,
    #[cfg(feature = "html")]
    default_html_config: HtmlConfig,
    #[cfg(feature = "tokio")]
    limiter: Option<std::sync::Arc<tokio::sync::Semaphore>>,
    #[cfg(feature = "tokio")]
    deadline: Option<std::time::Duration>,
}

impl Renderer {
    /// Create a renderer over the given fonts, with default per-format configs
    /// and no concurrency limit.
    #[must_use]
    pub fn new(fonts: SharedFonts) -> Self {
        Self {
            fonts,
            templates: HashMap::new(),
            #[cfg(feature = "pdf")]
            default_pdf_config: PdfConfig::default(),
            #[cfg(feature = "html")]
            default_html_config: HtmlConfig::default(),
            #[cfg(feature = "tokio")]
            limiter: None,
            #[cfg(feature = "tokio")]
            deadline: None,
        }
    }

    /// Set the [`PdfConfig`] applied to every PDF render.
    #[cfg(feature = "pdf")]
    #[must_use]
    pub fn with_default_pdf_config(mut self, config: PdfConfig) -> Self {
        self.default_pdf_config = config;
        self
    }

    /// Set the [`HtmlConfig`] applied to every HTML render.
    #[cfg(feature = "html")]
    #[must_use]
    pub fn with_default_html_config(mut self, config: HtmlConfig) -> Self {
        self.default_html_config = config;
        self
    }

    /// Bound the number of concurrent `*_async` renders (each saturates a CPU
    /// core). A good default is the CPU count.
    #[cfg(feature = "tokio")]
    #[must_use]
    pub fn with_concurrency_limit(mut self, max_concurrent: usize) -> Self {
        self.limiter = Some(std::sync::Arc::new(tokio::sync::Semaphore::new(
            max_concurrent,
        )));
        self
    }

    /// Set a deadline for the `*_async` renders. If a render exceeds it, the call
    /// returns [`PublishError::Timeout`] promptly.
    ///
    /// This is a best-effort only, the compile is not cancelled (Typst can't be
    /// interrupted in-thread) so it keeps running in the background and its
    /// result is discarded, so CPU is not freed. This bounds request latency,
    /// not resource use. For hard bounds, isolate rendering in a separate
    /// process. Has no effect on the synchronous renders.
    #[cfg(feature = "tokio")]
    #[must_use]
    pub fn with_deadline(mut self, deadline: std::time::Duration) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Register a pre-parsed template under `key`. Keys are explicit (no silent
    /// path/mtime caching), so callers control invalidation by re-registering.
    pub fn register_template(&mut self, key: &'static str, template: PreparedTemplate) {
        self.templates.insert(key, template);
    }

    /// Warm up first-use work (font decode + cache) so the first real render is
    /// fast. See [`SharedFonts::warm_up`].
    pub fn warm_up(&self) {
        self.fonts.warm_up();
    }

    #[cfg(any(feature = "pdf", feature = "html"))]
    fn lookup(&self, key: &str) -> Result<&PreparedTemplate> {
        self.templates.get(key).ok_or_else(|| {
            PublishError::InvalidTemplate(format!("no template registered for key '{key}'"))
        })
    }

    /// Render the template registered under `key` to PDF bytes, synchronously.
    ///
    /// # Errors
    /// [`PublishError::InvalidTemplate`] if `key` isn't registered, otherwise any
    /// render error from [`render_pdf_prepared`](crate::render_pdf_prepared).
    #[cfg(feature = "pdf")]
    pub fn render_pdf(&self, key: &str, data: DataSet) -> Result<Vec<u8>> {
        let template = self.lookup(key)?;
        render_pdf_prepared(&self.fonts, template, data, &self.default_pdf_config)
    }

    /// Render the template registered under `key` to PDF bytes asynchronously:
    /// the blocking Typst compile runs on a `spawn_blocking` thread, gated by the
    /// concurrency limit if one is set.
    ///
    /// # Errors
    /// [`PublishError::InvalidTemplate`] if `key` isn't registered or the render
    /// task fails to join, otherwise any render error from
    /// [`render_pdf_prepared`](crate::render_pdf_prepared).
    #[cfg(all(feature = "pdf", feature = "tokio"))]
    pub async fn render_pdf_async(&self, key: &str, data: DataSet) -> Result<Vec<u8>> {
        let template = self.lookup(key)?.clone();
        let fonts = self.fonts.clone();
        let config = self.default_pdf_config.clone();
        self.run_blocking(move || render_pdf_prepared(&fonts, &template, data, &config))
            .await
    }

    /// Render the template registered under `key` to an HTML string,
    /// synchronously.
    ///
    /// # Errors
    /// [`PublishError::InvalidTemplate`] if `key` isn't registered, otherwise any
    /// render error from [`render_html_prepared`](crate::render_html_prepared).
    #[cfg(feature = "html")]
    pub fn render_html(&self, key: &str, data: DataSet) -> Result<String> {
        let template = self.lookup(key)?;
        render_html_prepared(&self.fonts, template, data, &self.default_html_config)
    }

    /// Render the template registered under `key` to an HTML string
    /// asynchronously (see [`render_pdf_async`](Self::render_pdf_async) for the
    /// concurrency/deadline semantics).
    ///
    /// # Errors
    /// [`PublishError::InvalidTemplate`] if `key` isn't registered or the render
    /// task fails to join, otherwise any render error from
    /// [`render_html_prepared`](crate::render_html_prepared).
    #[cfg(all(feature = "html", feature = "tokio"))]
    pub async fn render_html_async(&self, key: &str, data: DataSet) -> Result<String> {
        let template = self.lookup(key)?.clone();
        let fonts = self.fonts.clone();
        let config = self.default_html_config.clone();
        self.run_blocking(move || render_html_prepared(&fonts, &template, data, &config))
            .await
    }

    /// Shared async machinery for the `*_async` renders: acquire a concurrency
    /// permit (if limited), offload the blocking compile, and apply the optional
    /// best-effort deadline.
    #[cfg(feature = "tokio")]
    async fn run_blocking<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        // Hold a permit (if limited) for the lifetime of the blocking task.
        let permit = match &self.limiter {
            Some(sem) => Some(
                sem.clone()
                    .acquire_owned()
                    .await
                    .map_err(|e| PublishError::InvalidTemplate(format!("limiter closed: {e}")))?,
            ),
            None => None,
        };

        let join = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            f()
        });

        // Optional best-effort deadline: returns promptly on expiry, the
        // spawn_blocking task is left to finish (and be discarded) in the
        // background, since the compile can't be cancelled.
        let joined = match self.deadline {
            Some(d) => tokio::time::timeout(d, join)
                .await
                .map_err(|_| PublishError::Timeout)?,
            None => join.await,
        };
        joined.map_err(|e| PublishError::InvalidTemplate(format!("render task failed: {e}")))?
    }
}

#[cfg(all(test, feature = "tokio", feature = "pdf"))]
mod tests {
    use super::*;
    use crate::{DataSet, TemplateSource};

    fn renderer() -> Renderer {
        let mut r = Renderer::new(SharedFonts::embedded_only()).with_concurrency_limit(2);
        r.register_template(
            "t",
            PreparedTemplate::new(TemplateSource::InMemory {
                content: "async facade test".to_owned(),
                root: None,
            })
            .unwrap(),
        );
        r
    }

    #[test]
    fn render_async_matches_sync() {
        let r = renderer();
        let sync = r.render_pdf("t", DataSet::new()).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let asy = rt
            .block_on(r.render_pdf_async("t", DataSet::new()))
            .unwrap();

        assert_eq!(&asy[0..5], b"%PDF-");
        assert_eq!(sync, asy, "async and sync renders should match");
    }

    #[test]
    fn render_async_with_generous_deadline_succeeds() {
        // Exercises the deadline code path without racing the timer: a 30s
        // deadline never fires for a millisecond render. (The firing path is
        // `tokio::time::timeout`, trusted for correctness. A zero-deadline test
        // would be flaky since a fast render can beat the timer.)
        let r = renderer().with_deadline(std::time::Duration::from_secs(30));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let bytes = rt
            .block_on(r.render_pdf_async("t", DataSet::new()))
            .unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
    }
}
