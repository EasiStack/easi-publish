//! Performance benchmarks for easi-publish.
//!
//! NOTE: `cargo bench` uses the `bench` profile (optimized), so these numbers
//! are the optimized floor. The speed you get in a release build or with the
//! `[profile.dev.package."*"] opt-level = 3` workspace override. They do NOT
//! show the unoptimized-debug penalty (Typst can be 10-50x slower unoptimized);
//! that is what the dev-profile override addresses for `cargo run`/`cargo test`.

use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use easi_publish::{DataSet, PdfConfig, SharedFonts, TemplateSource, render_pdf};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn data() -> serde_json::Value {
    let raw = std::fs::read_to_string(fixture("invoice-min.json")).expect("read fixture json");
    serde_json::from_str(&raw).expect("parse fixture json")
}

fn render_once(fonts: &SharedFonts) -> Vec<u8> {
    render_pdf(
        fonts,
        TemplateSource::FilePath(fixture("invoice-min.typ")),
        DataSet::from_json_file("invoice.json", data()).expect("encode fixture json"),
        &PdfConfig::default(),
    )
    .expect("render_pdf fixture")
}

fn benches(c: &mut Criterion) {
    // Cost of scanning system fonts (startup, one-time).
    c.bench_function("fonts_new_system_scan", |b| b.iter(SharedFonts::new));

    // Steady-state render_pdf cost (fonts reused, cache warm).
    let fonts = SharedFonts::new();
    let _ = render_once(&fonts); // warm font decode + comemo
    c.bench_function("render_invoice_warm", |b| b.iter(|| render_once(&fonts)));
}

criterion_group!(g, benches);
criterion_main!(g);
