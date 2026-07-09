<p align="center">  
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/EasiStack/easi-publish/refs/heads/main/.github/easistack-logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/EasiStack/easi-publish/refs/heads/main/.github/easistack-logo-light.svg">
    <img alt="easiStack" src="https://raw.githubusercontent.com/EasiStack/easi-publish/refs/heads/main/.github/easistack-logo-light.svg" width="350" height="70" style="max-width: 100%;">
  </picture>
</p>

# easi-publish

**easi-publish**\
Compiles [Typst] templates with user-supplied data into PDF or HTML documents.\
[![Crates.io](https://img.shields.io/crates/v/easi-publish)](https://crates.io/crates/easi-publish)

**easi-publish-axum**\
Provides [Axum](https://github.com/tokio-rs/axum) integration for the easi-publish crate to render Typst templates into PDF or HTML HTTP responses.\
[![Crates.io](https://img.shields.io/crates/v/easi-publish-axum)](https://crates.io/crates/easi-publish-axum)



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

