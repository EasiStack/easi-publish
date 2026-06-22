use std::fmt;

use thiserror::Error;

/// Severity of a compilation/export [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A hard error — compilation/export failed.
    Error,
    /// A warning — surfaced alongside errors when present.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
        })
    }
}

/// A hint for resolving a [`Diagnostic`], with its own source location.
///
/// As of Typst 0.15 a hint can carry its own span (distinct from the
/// diagnostic's), so this mirrors [`Diagnostic`]'s location fields. In practice
/// most hints are *detached* (no span), in which case the location fields are
/// `None`.
#[derive(Debug, Clone)]
pub struct Hint {
    /// The hint text.
    pub message: String,
    /// Source file the hint's span belongs to, if it carries a location.
    pub file: Option<String>,
    /// 1-based line number, if known.
    pub line: Option<usize>,
    /// 1-based column number, if known.
    pub column: Option<usize>,
}

impl fmt::Display for Hint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{file}")?;
            if let Some(line) = self.line {
                write!(f, ":{line}")?;
                if let Some(col) = self.column {
                    write!(f, ":{col}")?;
                }
            }
            write!(f, ": ")?;
        }
        f.write_str(&self.message)
    }
}

/// A single Typst diagnostic with its source location preserved.
///
/// Unlike a flat error string, this keeps the `file`, `line`, and `column` of
/// the offending span (when Typst attaches one) so callers can point template
/// authors at the exact spot.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error or warning.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Source file the span belongs to (the template name, or an imported
    /// file), if the diagnostic carries a location.
    pub file: Option<String>,
    /// 1-based line number, if known.
    pub line: Option<usize>,
    /// 1-based column number, if known.
    pub column: Option<usize>,
    /// Typst's hints for resolving the issue, each with its own optional
    /// location.
    pub hints: Vec<Hint>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{file}")?;
            if let Some(line) = self.line {
                write!(f, ":{line}")?;
                if let Some(col) = self.column {
                    write!(f, ":{col}")?;
                }
            }
            write!(f, ": ")?;
        }
        write!(f, "{}: {}", self.severity, self.message)?;
        for hint in &self.hints {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}

/// Anything that can go wrong while turning a template + data into a PDF or
/// HTML document.
#[derive(Debug, Error)]
pub enum PublishError {
    /// Reading the template or writing the output file failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Typst compilation failed, carries the per-span diagnostics.
    #[error("Typst compilation failed:\n{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Compilation(Vec<Diagnostic>),

    /// PDF export failed, carries the per-span diagnostics. Only constructed by
    /// the PDF path (the `pdf` feature).
    #[cfg(feature = "pdf")]
    #[error("PDF export failed:\n{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Export(Vec<Diagnostic>),

    /// HTML export failed, carries the per-span diagnostics. Only constructed by
    /// [`render_html`](crate::render_html) (the `html` feature).
    #[cfg(feature = "html")]
    #[error("HTML export failed:\n{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    HtmlExport(Vec<Diagnostic>),

    /// The supplied data could not be serialized to JSON.
    #[error("Invalid JSON data: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// The template or a render key was invalid (e.g. an unknown
    /// [`Renderer`](crate::Renderer) key).
    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    /// The requested PDF standards combination is invalid.
    #[cfg(feature = "pdf")]
    #[error("Invalid PDF standards: {0}")]
    InvalidStandards(String),

    /// An image could not be decoded or re-encoded (from
    /// [`downscale_image`](crate::downscale_image)).
    #[cfg(feature = "image")]
    #[error("Image processing failed: {0}")]
    Image(String),

    /// An async render exceeded its configured deadline. The compile is not
    /// cancelled (Typst can't be interrupted in-thread), it keeps running in
    /// the background and its result is discarded. For hard CPU/memory bounds,
    /// isolate rendering in a separate process.
    #[cfg(feature = "tokio")]
    #[error("render exceeded the configured deadline")]
    Timeout,
}

/// Convenience alias for `Result<T, PublishError>`.
pub type Result<T> = std::result::Result<T, PublishError>;
