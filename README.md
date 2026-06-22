# `easi-publish`

**easi-publish** compiles [Typst] templates with user-supplied data into PDF or HTML documents. 
**easi-publish-axum** provides [Axum](https://github.com/tokio-rs/axum) integration for the easi-publish crate to render Typst templates into PDF or HTML HTTP responses.

## Quick start

```rust
use easi_publish::{SharedFonts, TemplateSource, DataSet, PdfConfig, render_pdf};

let fonts = SharedFonts::new();   // created one-time and can be cached for efficiency
let template = TemplateSource::FilePath("templates/invoice.typ".into());
let data = DataSet::from_json_file(
    "invoice.json",
    serde_json::json!({
        "invoice_number": "INV-0001",
        "amount": 199.00,
    }),
)?;
let pdf_bytes = render_pdf(&fonts, template, data, &PdfConfig::default())?;
```

See individual crate READMEs for in-depth documentation.

