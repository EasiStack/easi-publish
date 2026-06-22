use std::path::PathBuf;
use std::sync::LazyLock;

use easi_publish::{
    DataSet, PdfConfig, PublishError, PreparedTemplate, SharedFonts, TemplateSource, render_pdf,
    render_pdf_prepared, render_pdf_to_file,
};

// Share fonts across all tests to speed them up and also verify that caching works 
// across renders.
static FONTS: LazyLock<SharedFonts> = LazyLock::new(SharedFonts::new);

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn render_invoice_json_file() {
    let template = TemplateSource::FilePath(fixture_dir().join("invoice.typ"));
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture_dir().join("invoice.json")).unwrap(),
    )
    .unwrap();
    let data = DataSet::from_json_file("invoice.json", json).unwrap();

    let bytes = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap();

    assert!(bytes.len() > 1000, "PDF should be non-trivial in size");
    assert_eq!(&bytes[0..5], b"%PDF-", "should start with PDF header");
}

#[test]
fn render_in_memory_template() {
    let template = TemplateSource::InMemory {
        content: "Hello, World!".to_owned(),
        root: None,
    };
    let data = DataSet::new();

    let bytes = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn render_with_sys_inputs() {
    let template = TemplateSource::InMemory {
        content: r#"#let d = sys.inputs
Invoice \##d.number for #d.customer"#
            .to_owned(),
        root: None,
    };
    let data = DataSet::from_inputs(serde_json::json!({
        "number": "INV-001",
        "customer": "Acme Corp"
    }))
    .unwrap();

    let bytes = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn render_with_serialize_struct() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct Data {
        greeting: String,
        count: i32,
    }

    let template = TemplateSource::InMemory {
        content: r#"#let d = sys.inputs
#d.greeting — count: #d.count"#
            .to_owned(),
        root: None,
    };
    let data = DataSet::from_inputs(Data {
        greeting: "Hello".to_owned(),
        count: 42,
    })
    .unwrap();

    let bytes = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn compilation_error_is_reported() {
    let template = TemplateSource::InMemory {
        content: "#let x = ".to_owned(),
        root: None,
    };
    let data = DataSet::new();

    let result = render_pdf(&FONTS, template, data, &PdfConfig::default());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PublishError::Compilation(_)));
}

#[test]
fn compilation_diagnostic_carries_location() {
    // Error on line 2 of the in-memory "main.typ".
    let template = TemplateSource::InMemory {
        content: "Some text on line one.\n#let x = \n".to_owned(),
        root: None,
    };
    let err = render_pdf(&FONTS, template, DataSet::new(), &PdfConfig::default())
        .unwrap_err();
    let PublishError::Compilation(diags) = err else {
        panic!("expected Compilation, got {err:?}");
    };
    assert!(!diags.is_empty(), "should have at least one diagnostic");
    let d = &diags[0];
    assert_eq!(d.severity, easi_publish::Severity::Error);
    assert!(!d.message.is_empty());
    assert_eq!(d.file.as_deref(), Some("main.typ"));
    assert!(d.line.is_some(), "diagnostic should carry a line: {d:?}");
    // Display includes the location.
    assert!(d.to_string().contains("main.typ:"), "{}", d);
}

#[test]
fn shared_fonts_are_reusable() {
    for i in 0..3 {
        let template = TemplateSource::InMemory {
            content: format!("Document {i}"),
            root: None,
        };
        let data = DataSet::new();
        let bytes = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
    }
}

#[test]
fn pdf_config_builder() {
    let config = PdfConfig::new().ident("test-doc").tagged(false);
    let template = TemplateSource::InMemory {
        content: "Test".to_owned(),
        root: None,
    };
    let data = DataSet::new();
    let bytes = render_pdf(&FONTS, template, data, &config).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn render_to_file_writes() {
    let dir = std::env::temp_dir().join("easi_publish_test");
    std::fs::create_dir_all(&dir).unwrap();
    let output = dir.join("test_output.pdf");

    let template = TemplateSource::InMemory {
        content: "File output test".to_owned(),
        root: None,
    };
    let data = DataSet::new();

    render_pdf_to_file(&FONTS, template, data, &PdfConfig::default(), &output).unwrap();

    assert!(output.exists());
    let bytes = std::fs::read(&output).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");

    // Cleanup
    let _ = std::fs::remove_file(&output);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn embedded_only_renders_without_system_scan() {
    let fonts = SharedFonts::embedded_only();
    let bytes = render_pdf(
        &fonts,
        TemplateSource::InMemory {
            content: "Hello from embedded fonts".to_owned(),
            root: None,
        },
        DataSet::new(),
        &PdfConfig::default(),
    )
    .unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn warm_up_is_best_effort_and_does_not_panic() {
    SharedFonts::embedded_only().warm_up();
}

#[test]
fn prepared_template_reuses_parse_and_is_deterministic() {
    // Parse the template once, render_pdf it twice. With the default config (Auto
    // ident derived from content, no timestamp) the two outputs are identical.
    let prepared =
        PreparedTemplate::new(TemplateSource::FilePath(fixture_dir().join("invoice.typ"))).unwrap();
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture_dir().join("invoice.json")).unwrap(),
    )
    .unwrap();

    let a = render_pdf_prepared(
        &FONTS,
        &prepared,
        DataSet::from_json_file("invoice.json", json.clone()).unwrap(),
        &PdfConfig::default(),
    )
    .unwrap();
    let b = render_pdf_prepared(
        &FONTS,
        &prepared,
        DataSet::from_json_file("invoice.json", json).unwrap(),
        &PdfConfig::default(),
    )
    .unwrap();

    assert_eq!(&a[0..5], b"%PDF-");
    assert_eq!(a, b, "same template + data + config should be byte-identical");
}

#[test]
fn package_imports_are_denied() {
    let template = TemplateSource::InMemory {
        content: "#import \"@preview/cetz:0.2.2\": *".to_owned(),
        root: None,
    };
    let err = render_pdf(&FONTS, template, DataSet::new(), &PdfConfig::default())
        .unwrap_err();
    assert!(matches!(err, PublishError::Compilation(_)), "got {err:?}");
}

#[test]
fn virtual_only_template_denies_disk_reads() {
    // No root, every file read is denied.
    let template = TemplateSource::InMemory {
        content: "#read(\"secrets.txt\")".to_owned(),
        root: None,
    };
    let err = render_pdf(&FONTS, template, DataSet::new(), &PdfConfig::default())
        .unwrap_err();
    assert!(matches!(err, PublishError::Compilation(_)), "got {err:?}");
}

#[test]
fn fixed_clock_is_deterministic_for_today() {
    let cfg = PdfConfig::new().clock(easi_publish::Clock::Fixed {
        year: 2020,
        month: 1,
        day: 2,
    });
    let mk = || TemplateSource::InMemory {
        content: "#datetime.today().display()".to_owned(),
        root: None,
    };
    let a = render_pdf(&FONTS, mk(), DataSet::new(), &cfg).unwrap();
    let b = render_pdf(&FONTS, mk(), DataSet::new(), &cfg).unwrap();
    assert_eq!(&a[0..5], b"%PDF-");
    assert_eq!(a, b, "fixed clock should produce identical output");
}

#[test]
fn renderer_facade_renders_registered_template() {
    let mut renderer = easi_publish::Renderer::new(SharedFonts::embedded_only());
    renderer.register_template(
        "hello",
        PreparedTemplate::new(TemplateSource::InMemory {
            content: "Hello from the facade".to_owned(),
            root: None,
        })
        .unwrap(),
    );

    let bytes = renderer
        .render_pdf("hello", DataSet::new())
        .unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");

    // Unknown key is an error, not a panic.
    let err = renderer
        .render_pdf("missing", DataSet::new())
        .unwrap_err();
    assert!(matches!(err, PublishError::InvalidTemplate(_)), "got {err:?}");
}

#[test]
fn evict_cache_runs_after_render() {
    let template = TemplateSource::InMemory {
        content: "cache test".to_owned(),
        root: None,
    };
    let _ = render_pdf(&FONTS, template, DataSet::new(), &PdfConfig::default()).unwrap();
    // Must not panic, clears the memoization cache.
    easi_publish::evict_cache(0);
}

#[test]
fn render_with_dict_directly() {
    use typst::foundations::{Dict, Str, Value};

    let dict: Dict = [(Str::from("name"), Value::Str("World".into()))]
        .into_iter()
        .collect();

    let template = TemplateSource::InMemory {
        content: r#"#let d = sys.inputs
Hello #d.name!"#
            .to_owned(),
        root: None,
    };

    let bytes = render_pdf(&FONTS, template, DataSet::from_dict(dict), &PdfConfig::default()).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn dataset_combines_inputs_and_multiple_files() {
    let template = TemplateSource::InMemory {
        content: r#"#let rows = json("rows.json")
#sys.inputs.title — #rows.len() rows"#
            .to_owned(),
        root: None,
    };
    let data = DataSet::new()
        .inputs(serde_json::json!({ "title": "Report" }))
        .unwrap()
        .json_file("rows.json", serde_json::json!([1, 2, 3]))
        .unwrap()
        .file("note.txt", b"ignored".to_vec())
        .unwrap();

    let bytes = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap();
    assert_eq!(&bytes[0..5], b"%PDF-");
}

#[test]
fn dataset_rejects_duplicate_file_names() {
    let err = DataSet::new()
        .file("a.json", b"{}".to_vec())
        .unwrap()
        .file("a.json", b"{}".to_vec())
        .unwrap_err();
    assert!(matches!(err, PublishError::InvalidTemplate(_)), "got {err:?}");
}

#[test]
fn virtual_file_colliding_with_template_name_errors() {
    // An in-memory template is named "main.typ". A virtual file of the same name
    // would shadow it, so it's rejected at render time.
    let template = TemplateSource::InMemory {
        content: "hello".to_owned(),
        root: None,
    };
    let data = DataSet::new().file("main.typ", b"x".to_vec()).unwrap();
    let err = render_pdf(&FONTS, template, data, &PdfConfig::default()).unwrap_err();
    assert!(matches!(err, PublishError::InvalidTemplate(_)), "got {err:?}");
}
