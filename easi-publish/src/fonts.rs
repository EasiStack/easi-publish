use std::path::Path;
use std::sync::Arc;

use typst_kit::fonts::{FontStore, embedded, scan, system};

/// Shared font resources that can be reused across multiple compilations.
///
/// [`new`](Self::new) scans system fonts, which is expensive. Create it once
/// at application startup and pass it to all render calls. If your templates
/// only use the bundled fonts, [`embedded_only`](Self::embedded_only) skip the
/// system scan entirely, which should mean a faster startup time.
///
/// ## Variable fonts
///
/// Since Typst 0.15, **variable fonts** are supported: the `ital`, `slnt`,
/// `wght`, `wdth`, and `opsz` axes are set automatically from the text weight,
/// stretch, style, and size, and templates can drive custom axes via
/// `text(variations: ..)`. Any variable font picked up by [`new`](Self::new) /
/// [`with_extra_dirs`](Self::with_extra_dirs) just works. One gotcha: Typst
/// trims the `"Variable"`, `"Var"`, and `"VF"` suffixes from family names to
/// unify static and variable faces. Reference the **trimmed** family name in
/// `#set text(font: ..)`.
#[derive(Clone)]
pub struct SharedFonts {
    // `FontStore` (typst-kit 0.15) owns both the `FontBook` metadata and the
    // lazily-loaded font slots. `World::book` / `World::font` delegate straight
    // to it, so one shared store replaces the separate book + slots we held
    // before. System fonts are registered first and the embedded fonts last,
    // so the bundled families act as fallbacks, matching typst's own order.
    pub(crate) store: Arc<FontStore>,
}

impl SharedFonts {
    /// Scan system fonts and embedded fonts. Call once at startup.
    #[must_use]
    pub fn new() -> Self {
        let mut store = FontStore::new();
        store.extend(system());
        store.extend(embedded());
        Self {
            store: Arc::new(store),
        }
    }

    /// Scan system fonts, embedded fonts, plus additional font directories.
    #[must_use]
    pub fn with_extra_dirs<I, P>(dirs: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut store = FontStore::new();
        store.extend(system());
        for dir in dirs {
            store.extend(scan(dir.as_ref()));
        }
        store.extend(embedded());
        Self {
            store: Arc::new(store),
        }
    }

    /// Use only the fonts embedded in the binary. This skips the system scan.
    ///
    /// Much faster to construct than [`new`](Self::new) (no OS font-directory
    /// walk) and produces reproducible, host-independent output, at the cost of
    /// only having the bundled font families available. Prefer this for servers
    /// whose templates stick to the embedded fonts.
    #[must_use]
    pub fn embedded_only() -> Self {
        let mut store = FontStore::new();
        store.extend(embedded());
        Self {
            store: Arc::new(store),
        }
    }

    /// Force first-use work (lazy font decode + Typst standard-library / cache
    /// warm-up) now, by rendering a tiny throwaway document, so the first real
    /// render doesn't pay it. Call once at startup after constructing the fonts.
    ///
    /// Uses whichever output backend is enabled (PDF if present, else HTML). This is 
    /// a no-op when neither feature is on. Errors are intentionally ignored, this
    /// is a best-effort warm-up.
    #[allow(unused_variables)]
    pub fn warm_up(&self) {
        let template = crate::TemplateSource::InMemory {
            content: " ".to_owned(),
            root: None,
        };
        let data = crate::DataSet::new();

        #[cfg(feature = "pdf")]
        let _ = crate::render_pdf(self, template, data, &crate::PdfConfig::default());

        #[cfg(all(not(feature = "pdf"), feature = "html"))]
        let _ = crate::render_html(self, template, data, &crate::HtmlConfig::default());
    }
}

impl Default for SharedFonts {
    fn default() -> Self {
        Self::new()
    }
}
