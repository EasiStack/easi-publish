use serde::Serialize;
use typst::foundations::{Dict, Value};

use crate::error::{PublishError, Result};

/// The data handed to a template render: an optional `sys.inputs` dictionary
/// plus zero or more **virtual files** the template can load (`json("...")`,
/// `csv("...")`, `image("...")`, `read("...")`, ...).
///
/// This is the single data type accepted by [`render_pdf`](crate::render_pdf),
/// [`render_html`](crate::render_html), and [`Renderer`](crate::Renderer). Use a
/// `from_*` constructor for the common single-source case, or chain the builder
/// methods to combine `sys.inputs` with one or more files:
///
/// ```
/// # use easi_publish::DataSet;
/// # fn build() -> easi_publish::Result<DataSet> {
/// // Just `sys.inputs`:
/// let a = DataSet::from_inputs(serde_json::json!({ "title": "Q2" }))?;
///
/// // A single JSON file the template reads via `json("invoice.json")`:
/// let b = DataSet::from_json_file("invoice.json", serde_json::json!({ "n": 1 }))?;
///
/// // Inputs plus several files:
/// let c = DataSet::new()
///     .inputs(serde_json::json!({ "title": "Q2" }))?
///     .json_file("rows.json", serde_json::json!([1, 2, 3]))?
///     .file("logo.png", vec![0u8; 4])?;
/// # Ok(c) }
/// ```
///
/// Data conversion happens eagerly in these methods (so serialization errors
/// surface here, not at render time), and adding a file whose name duplicates an
/// earlier one fails with [`PublishError::InvalidTemplate`].
#[derive(Debug, Default, Clone)]
pub struct DataSet {
    pub(crate) inputs: Option<Dict>,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

impl DataSet {
    /// An empty data set, no `sys.inputs`, no files.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a data set whose `sys.inputs` is `data` (any `T: Serialize`).
    ///
    /// # Errors
    /// [`PublishError::InvalidJson`] if `data` can't be serialized.
    pub fn from_inputs<T: Serialize>(data: T) -> Result<Self> {
        Self::new().inputs(data)
    }

    /// Build a data set whose `sys.inputs` is a pre-built typst [`Dict`], zero
    /// conversion overhead.
    #[must_use]
    pub fn from_dict(dict: Dict) -> Self {
        Self {
            inputs: Some(dict),
            files: Vec::new(),
        }
    }

    /// Build a data set with a single virtual JSON file (the common case),
    /// loaded by the template via `json("<name>")`.
    ///
    /// # Errors
    /// [`PublishError::InvalidJson`] if `data` can't be serialized.
    pub fn from_json_file<T: Serialize>(name: impl Into<String>, data: T) -> Result<Self> {
        Self::new().json_file(name, data)
    }

    /// Set `sys.inputs` from any `T: Serialize` (replacing any previous inputs).
    ///
    /// # Errors
    /// [`PublishError::InvalidJson`] if `data` can't be serialized.
    pub fn inputs<T: Serialize>(mut self, data: T) -> Result<Self> {
        let value = serde_json::to_value(data)?;
        self.inputs = Some(json_to_dict(value));
        Ok(self)
    }

    /// Set `sys.inputs` from a pre-built typst [`Dict`] (replacing any previous
    /// inputs). Infallible escape hatch around [`inputs`](Self::inputs).
    #[must_use]
    pub fn inputs_dict(mut self, dict: Dict) -> Self {
        self.inputs = Some(dict);
        self
    }

    /// Add a virtual JSON file named `name` (serialized from `T: Serialize`),
    /// loadable via `json("<name>")`.
    ///
    /// # Errors
    /// [`PublishError::InvalidJson`] if `data` can't be serialized, or
    /// [`PublishError::InvalidTemplate`] if `name` duplicates an existing file.
    pub fn json_file<T: Serialize>(self, name: impl Into<String>, data: T) -> Result<Self> {
        let bytes = serde_json::to_vec(&data)?;
        self.file(name, bytes)
    }

    /// Add a virtual file named `name` with raw `bytes`, loadable via
    /// `read`/`image`/`csv`/`json`/... depending on its contents.
    ///
    /// # Errors
    /// [`PublishError::InvalidTemplate`] if `name` duplicates a file already
    /// added to this set.
    pub fn file(mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let name = name.into();
        if self.files.iter().any(|(existing, _)| existing == &name) {
            return Err(PublishError::InvalidTemplate(format!(
                "duplicate virtual file '{name}'"
            )));
        }
        self.files.push((name, bytes.into()));
        Ok(self)
    }
}

// --- Internal conversion ---

/// Convert a JSON value into a typst [`Dict`] for injection via `sys.inputs`.
///
/// Defers to typst's own `Deserialize for Value`, the same conversion used by
/// the template-side `json()` loader, so the JSON to typst mapping stays
/// consistent across the crate. Anything that isn't a JSON object yields an
/// empty dict, matching the `sys.inputs` contract, which is always a dict.
fn json_to_dict(value: serde_json::Value) -> Dict {
    match serde_json::from_value::<Value>(value) {
        Ok(Value::Dict(dict)) => dict,
        _ => Dict::new(),
    }
}
