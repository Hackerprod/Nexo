use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-fixtures")
            .join(format!(
                "i11a-explain-print-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path.to_str().unwrap().to_owned()
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn nexo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn explain_format_accepts_only_text_or_json() {
    for value in ["xml", "JSON", "Text", "", "json,text"] {
        let output = nexo(&["explain", "NX-E0102", "--format", value]);
        assert_eq!(output.status.code(), Some(2), "{value:?}");
        assert_eq!(output.stdout, b"", "{value:?}");
        assert_eq!(
            output.stderr, b"nexo-explain:invalid_arguments\n",
            "{value:?}"
        );
    }
    let repeated = nexo(&["explain", "NX-E0102", "--format", "json", "--format", "xml"]);
    assert_eq!(repeated.status.code(), Some(2));
    assert_eq!(repeated.stderr, b"");
    assert!(String::from_utf8_lossy(&repeated.stdout).contains(r#""reason":"invalid_arguments""#));

    let missing_value = nexo(&["explain", "NX-E0102", "--format"]);
    assert_eq!(missing_value.status.code(), Some(2));
    assert_eq!(missing_value.stdout, b"");
    assert_eq!(missing_value.stderr, b"nexo-explain:invalid_arguments\n");
}

#[test]
fn explain_rejects_a_repeated_subcause_even_when_every_value_is_valid() {
    for args in [
        [
            "explain",
            "NX-E0102",
            "--subcause",
            "not_indexable",
            "--subcause",
            "type_mismatch",
        ],
        [
            "explain",
            "NX-E0102",
            "--subcause",
            "not_indexable",
            "--subcause",
            "not_indexable",
        ],
    ] {
        let output = nexo(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(output.stdout, b"", "{args:?}");
        assert_eq!(
            output.stderr, b"nexo-explain:invalid_arguments\n",
            "{args:?}"
        );
    }
    let single = nexo(&["explain", "NX-E0102", "--subcause", "not_indexable"]);
    assert_eq!(single.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&single.stdout).starts_with("NX-E0102/not_indexable\n"));
}

#[test]
fn explain_text_separates_entries_and_ends_with_one_lf() {
    for code in ["NX-E0102", "NX-E0202", "NX-E0203", "NX-E0101"] {
        let json = nexo(&["explain", code, "--format", "json"]);
        assert_eq!(json.status.code(), Some(0));
        let json = String::from_utf8(json.stdout).unwrap();
        let entries = json.matches(r#""explanation_id":"#).count();

        let text = nexo(&["explain", code]);
        assert_eq!(text.status.code(), Some(0), "{code}");
        assert_eq!(text.stderr, b"", "{code}");
        let text = String::from_utf8(text.stdout).unwrap();
        assert!(text.ends_with('\n'), "{code}");
        assert!(!text.ends_with("\n\n"), "{code}: extra final LF");
        let blocks = text.trim_end_matches('\n').split("\n\n").count();
        assert_eq!(blocks, entries, "{code}");
        assert!(!text.contains("\n\n\n"), "{code}");
    }
}

#[test]
fn published_reduced_profile_quotas_match_the_catalog_and_d15() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let codes = fs::read_to_string(root.join("docs/adr/F0-CODIGOS.md")).unwrap();
    let d15 = fs::read_to_string(root.join("docs/adr/F0-D15-sintaxis-oficial.md")).unwrap();
    for (id, quota) in [
        ("f0-0002-b-test-frames-8-v2", "call_frames = 8"),
        ("f0-0002-b-test-cells-64-v2", "live_value_cells = 64"),
        (
            "f0-0002-b-test-observations-64-v2",
            "observation_bytes = 64",
        ),
    ] {
        assert!(codes.contains(&format!("| `{id}` | {quota}. |")), "{id}");
        assert!(d15.contains(&format!("| `{id}` | `{quota}` |")), "{id}");
    }
    let cells_boundary = "A2 §11.5 ll. 967–970";
    assert!(codes.contains(cells_boundary));
    assert!(d15.contains(cells_boundary));
}

#[test]
fn explain_json_entries_are_comma_separated_in_a_single_object() {
    let output = nexo(&["explain", "NX-E0102", "--format", "json"]);
    assert_eq!(
        (output.status.code(), output.stderr.as_slice()),
        (Some(0), b"".as_slice())
    );
    let json = String::from_utf8(output.stdout).unwrap();
    let entries = json.matches(r#""explanation_id":"#).count();
    assert_eq!(entries, 4);
    assert!(json.starts_with(
        r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","code":"NX-E0102","entries":[{"subcause":"#
    ));
    assert!(json.ends_with("]}\n"));
    assert_eq!(json.matches(r#"},{"subcause":"#).count(), entries - 1);
    assert_eq!(json.matches('\n').count(), 1);
}

#[test]
fn print_formats_unit_arrays_and_result_values_exactly() {
    let directory = TestDirectory::new();
    let path = directory.file(
        "print.n",
        "fn Main(){Print(());Print([(),()]);Print(TryAdd(1,2));Print(TryAdd(9223372036854775807,1));Print([true,false]);}",
    );
    let output = nexo(&["run", &path]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        b"()\n[(), ()]\nOk(3)\nErr(30)\n[true, false]\n"
    );
    assert_eq!(output.stderr, b"nexo-result:()\n");
}
