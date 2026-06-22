use std::path::{Path, PathBuf};

use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};

use crate::error::Result;

/// Build a project-rooted [`FileId`] for a virtual file named `name`.
///
/// In typst 0.15 a `FileId` is an interned [`RootedPath`] (a [`VirtualRoot`]
/// plus a [`VirtualPath`]), and [`VirtualPath::new`] now returns a `Result`
/// (it rejects backslash paths and other malformed inputs). Both our template
/// source and any virtual JSON data file live at the project root, so this
/// centralises that construction. A malformed `name` falls back to `main.typ`
/// rather than panicking on a caller-supplied filename.
pub(crate) fn project_file_id(name: &str) -> FileId {
    let vpath = VirtualPath::new(name)
        .unwrap_or_else(|_| VirtualPath::new("main.typ").expect("static path is valid"));
    FileId::new(RootedPath::new(VirtualRoot::Project, vpath))
}

/// How to load the Typst template.
pub enum TemplateSource {
    /// Load from a file on disk. The parent directory becomes the root
    /// for resolving relative imports and data files.
    FilePath(PathBuf),

    /// Template content provided as a string. An optional root directory
    /// can be provided for resolving relative imports.
    InMemory {
        /// The Typst source.
        content: String,
        /// Optional root for resolving relative imports / file reads. `None`
        /// denies all disk reads (virtual-only).
        root: Option<PathBuf>,
    },
}

/// A template read and parsed once, ready to render many times via
/// [`render_pdf_prepared`](crate::render_pdf_prepared).
///
/// [`render_pdf`](crate::render_pdf) reads + parses the template on every call.
/// Building a `PreparedTemplate` does that work up front, so a server rendering
/// the same template per request can reuse one instance and skip the repeated
/// disk read + parse. Cloning into each render is cheap, the parsed `Source`
/// is reference-counted internally.
#[derive(Clone)]
pub struct PreparedTemplate {
    pub(crate) source: Source,
    pub(crate) root: Option<PathBuf>,
}

impl PreparedTemplate {
    /// Read + parse the template once.
    ///
    /// # Errors
    /// Returns [`PublishError`](crate::PublishError) if a [`TemplateSource::FilePath`]
    /// can't be read.
    pub fn new(template: TemplateSource) -> Result<Self> {
        match template {
            TemplateSource::FilePath(path) => {
                let content = std::fs::read_to_string(&path)?;
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("main.typ");
                let id = project_file_id(file_name);
                let source = Source::new(id, content);
                let root = path.parent().map(Path::to_path_buf);
                Ok(Self { source, root })
            }
            TemplateSource::InMemory { content, root } => {
                let id = project_file_id("main.typ");
                let source = Source::new(id, content);
                Ok(Self { source, root })
            }
        }
    }
}
