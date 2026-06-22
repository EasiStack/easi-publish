# `easi-publish`

Compile [Typst] templates with user-supplied data into PDF or HTML documents.

[Axum](https://github.com/tokio-rs/axum) integration for
[`easi-publish`](https://crates.io/crates/easi-publish): render Typst templates into PDF
or HTML HTTP responses.

## What it does

Wraps `typst` + `typst-pdf` / `typst-html` into a small API:

- `TemplateSource` — point at a Typst template (file path or inline string).
- `DataSet` — bind your data to the template: `sys.inputs` and/or one or more virtual files the template loads via `json("...")` / `csv("...")` / `image("...")`. Build it with `DataSet::from_inputs` / `from_json_file`, or chain `inputs` and `file` for multiple sources.
- `PdfConfig` — page ranges, PDF standards (multiple compatible ones at once, e.g. `PdfStandard::Ua_1` + `A_2a`, or the `archival_accessible()` shorthand), tagging, timestamp, document `creator`, `pretty` output, clock. *(`pdf` feature.)*
- `HtmlConfig` — the smaller HTML knobs: `pretty` output and clock. *(`html` feature.)*
- `SharedFonts` — font collection (embedded by default via `typst-kit`'s `embedded-fonts` feature. System fonts added on top by `SharedFonts::new`). Build once per process, reuse across renders.
- `render_pdf(...)` — returns `Vec<u8>` containing the PDF bytes. *(`pdf` feature.)*
- `render_pdf_to_file(...)` — same, but writes to a path. *(`pdf` feature.)*
- `render_html(...)` — compile the *same* template to an HTML string instead (equations become MathML). *(`html` feature.)*
- `Renderer` — a front door that caches parsed templates and renders them by key to either format (`render_pdf` / `render_html`, plus async variants under `tokio`).

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

The Typst template references the data file by name (`#let data = json("invoice.json")`).

## Performance

`SharedFonts::new()` loads the embedded font set. This can be expensive (~tens of ms, allocates significant memory). We recommend building it once per process and sharing it across render calls. The `&SharedFonts` parameter on `render_pdf` / `render_html` exists precisely so callers can avoid re-loading.

The `typst` crate compiles templates from source on every call. If you render the same template many times, pre-compiling or caching at a higher level is a real optimisation.

## Features

The two output backends are features, so you compile only what you use:

- `pdf` (**default**) — `render_pdf` / `render_pdf_prepared` /
  `render_pdf_to_file`, `PdfConfig`, and `Renderer::render_pdf[_async]`.
- `html` — `render_html` / `render_html_prepared` (Typst → HTML, equations as
  MathML), `HtmlConfig`, and `Renderer::render_html[_async]`.

Enable both with `features = ["pdf", "html"]`, or take HTML only with
`default-features = false, features = ["html"]`.

Two more opt-in helpers (off by default):

- `image` — `downscale_image` helper (pulls a lean PNG/JPEG codec set) to embed
  images at display resolution. Big win for image-heavy documents.
- `tokio` — `Renderer::render_*_async` + a concurrency limiter / deadline.

## Built on Typst 0.15

This crate tracks **Typst 0.15** (MSRV 1.92). If you maintain templates written
against an older Typst, please not there are breaking changes from the previous 
versions.

## When to use it

- Generate invoices / receipts / certificates from templates 
- Generate reports with charts / formatted tables 
- Export the same template to HTML (math as MathML) via the `html` feature.

## When not to use it

- Read PDFs, it is output only.
- It is not a form-filling library. Templates render from data, they don't fill existing PDFs.

## License

Apache-2.0.
[Typst]: https://typst.app/
