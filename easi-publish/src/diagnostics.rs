//! Mapping from Typst's compile/export diagnostics to this crate's
//! [`Diagnostic`] type. Shared by the PDF and HTML render paths, the whole
//! module is gated on `any(feature = "pdf", feature = "html")`.

use typst::diag::SourceDiagnostic;
use typst::syntax::DiagSpan;
use typst::{World, WorldExt};

use crate::error::{Diagnostic, Hint, Severity};
use crate::world::PublishWorld;

/// Convert Typst's diagnostics into [`Diagnostic`]s, resolving each span to a
/// file + line/column via the world's sources.
pub(crate) fn to_diagnostics(world: &PublishWorld, diags: &[SourceDiagnostic]) -> Vec<Diagnostic> {
    diags
        .iter()
        .map(|d| {
            let (file, line, column) = locate(world, d.span);
            Diagnostic {
                severity: match d.severity {
                    typst::diag::Severity::Error => Severity::Error,
                    typst::diag::Severity::Warning => Severity::Warning,
                },
                message: d.message.to_string(),
                file,
                line,
                column,                
                hints: d
                    .hints
                    .iter()
                    .map(|h| {
                        let (file, line, column) = locate(world, h.span);
                        Hint {
                            message: h.v.to_string(),
                            file,
                            line,
                            column,
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Resolve a span to `(file, line, column)`. A detached span (no file) yields all `None`.
///
/// A Typst `Span` is an opaque handle into the syntax tree, not a file/line/col.
/// Turning it into something a human can act on means walking back through the
/// world: span -> file id -> the file's source text -> a byte range -> line/col.
/// Each of those steps can fail (a span with no file, a file we can't load, a
/// span that doesn't map to a range), so we degrade gracefully. We return as
/// much location detail as we managed to resolve rather than failing the whole
/// diagnostic.
fn locate(world: &PublishWorld, span: DiagSpan) -> (Option<String>, Option<usize>, Option<usize>) {
    // A "detached" span belongs to no file (e.g. synthetic nodes). Nothing to
    // locate so report the diagnostic with no position.
    let Some(id) = span.id() else {
        return (None, None, None);
    };

    // If we get here, then we know which file the span is in, even if the steps below fail,
    // so resolve the file name up front and reuse it in the fallbacks.
    let file = Some(id.vpath().get_without_slash().to_owned());

    // Load the file's source text via the same world Typst compiled against.
    // If it's unavailable (e.g. an IO error on re-read), we keep the file name
    // but can't compute a position.
    let Ok(source) = world.source(id) else {
        return (file, None, None);
    };

    // Map the span to a byte range within that source. A span can be valid yet
    // have no range (e.g. it refers to a different source revision). 
    let Some(range) = world.range(span) else {
        return (file, None, None);
    };

    // Resolve the byte offset to line/column using Typst's line index, so our
    // positions match what Typst (and editors via the LSP) report for the same
    // span, including its full notion of line breaks (`\r\n`, lone `\r`, and the
    // Unicode separators), not just `\n`. The index is precomputed, so this is a
    // binary search rather than a rescan from the top of the file.
    //
    // The helper is 0-based; we emit 1-based for humans. It returns `None` only if
    // the offset is past the end of the text, in which case we keep the file name
    // but report no position.
    match source.lines().byte_to_line_column(range.start) {
        Some((line, column)) => (file, Some(line + 1), Some(column + 1)),
        None => (file, None, None),
    }
}
