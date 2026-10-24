use std::process::Command;

use nexo::diagnostic::{self, ExplainError};

type AvailabilityExpectation = (
    &'static str,
    &'static [(Option<&'static str>, &'static str)],
);

fn refresh_budget_metadata(d: &mut diagnostic::Diagnostic) {
    use diagnostic::DataValue as V;
    let function_scope = d.data.get("budget_kind")
        == Some(&V::String("ownership_flow_steps".into()))
        && d.data.contains_key("analysis_function");
    d.data.insert(
        "budget_scope".into(),
        V::String(
            if function_scope {
                "function"
            } else {
                "compilation"
            }
            .into(),
        ),
    );
    d.data.insert(
        "incomplete_analysis".into(),
        V::String(d.stage.as_str().into()),
    );
}

#[test]
fn cli_preserves_b_utf8_errors_and_exact_source_limit() {
    use std::fs;
    let exe = env!("CARGO_BIN_EXE_nexo");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-fixtures")
        .join("input");
    fs::create_dir_all(&root).unwrap();
    let path = root.join(format!("i01-{}.nx", std::process::id()));
    for (name, bytes, start, hex) in [
        ("truncated", b"a\xf0\x9f".as_slice(), 1, "f09f"),
        ("middle", b"a\xffb".as_slice(), 1, "ff"),
        ("one-byte", b"\xff".as_slice(), 0, "ff"),
    ] {
        fs::write(&path, bytes).unwrap();
        let output = Command::new(exe)
            .args(["check", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{name}");
        assert_eq!(output.stdout, b"", "{name}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("NX-E0004:0:0:"), "{name}: {stderr}");
        assert!(
            stderr.contains("subcause: invalid_utf8"),
            "{name}: {stderr}"
        );
        assert!(stderr.contains("stage: input"), "{name}: {stderr}");
        assert!(
            stderr.contains("operation: input.decode"),
            "{name}: {stderr}"
        );
        assert!(
            stderr.contains(&format!("\"start\":{start}")),
            "{name}: {stderr}"
        );
        assert!(
            stderr.contains(&format!("\"invalid_bytes_hex\":\"{hex}\"")),
            "{name}: {stderr}"
        );
    }
    let mut exact_bytes = vec![b' '; nexo::diagnostic::SOURCE_LIMIT];
    exact_bytes[..13].copy_from_slice(b"fn Main(){}//");
    exact_bytes[nexo::diagnostic::SOURCE_LIMIT - 1] = b'\n';
    fs::write(&path, exact_bytes).unwrap();
    let exact = Command::new(exe)
        .args(["check", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(exact.status.code(), Some(0));
    assert_eq!(exact.stdout, b"nexo-check:ok\n");
    assert_eq!(exact.stderr, b"");
    fs::write(&path, vec![b' '; nexo::diagnostic::SOURCE_LIMIT + 1]).unwrap();
    let over = Command::new(exe)
        .args(["check", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(over.status.code(), Some(1));
    assert_eq!(over.stdout, b"");
    let stderr = String::from_utf8(over.stderr).unwrap();
    assert!(stderr.starts_with("NX-E0003:0:0:"), "{stderr}");
    assert!(stderr.contains("stage: input"), "{stderr}");
    assert!(stderr.contains("operation: analysis.limit"), "{stderr}");
    assert!(
        stderr.contains("\"budget_kind\":\"source_bytes\""),
        "{stderr}"
    );
    assert!(stderr.contains("\"limit\":1048576"), "{stderr}");
    assert!(stderr.contains("\"observed\":1048577"), "{stderr}");
    fs::remove_file(path).unwrap();
}

#[test]
fn pending_scope_matches_the_published_feature_partition() {
    let features = [
        ("async", "later_adr"),
        ("attributes", "later_adr"),
        ("closure", "F0-0003"),
        ("compound_assignment", "later_adr"),
        ("comptime", "later_adr"),
        ("default_argument", "later_adr"),
        ("extensions", "later_adr"),
        ("float_literal", "later_adr"),
        ("for_iteration", "later_adr"),
        ("function_value", "later_adr"),
        ("generator", "F0-0003"),
        ("implementation_limit", "I08"),
        ("inferred_non_unit_result", "later_adr"),
        ("loop_control", "later_adr"),
        ("match", "F0-0003"),
        ("methods", "later_adr"),
        ("modules_visibility", "later_adr"),
        ("no_heap_alloc", "later_adr"),
        ("option", "F0-0003"),
        ("partial_transfer", "later_adr"),
        ("provenance_clauses", "later_adr"),
        ("range", "later_adr"),
        ("receiver_mode", "later_adr"),
        ("region", "later_adr"),
        ("resource_array", "later_adr"),
        ("text", "later_adr"),
        ("traits_generics", "later_adr"),
        ("tuple", "later_adr"),
        ("unsupported_type", "later_adr"),
        ("user_types", "later_adr"),
    ];
    let entries = diagnostic::explain("NX-E0006", None).unwrap();
    assert_eq!(entries.len(), features.len());
    for ((feature, scope), entry) in features.iter().zip(entries) {
        assert_eq!(entry.feature_ids, [*feature]);
        let mut diagnostic = diagnostic_for_catalog("NX-E0006", None);
        diagnostic.feature_id = Some((*feature).into());
        (diagnostic.stage, diagnostic.operation) = match *feature {
            "function_value" => (
                diagnostic::Stage::Names,
                diagnostic::Operation::NamesResolve,
            ),
            "implementation_limit"
            | "inferred_non_unit_result"
            | "resource_array"
            | "option"
            | "unsupported_type" => (diagnostic::Stage::Types, diagnostic::Operation::TypesCheck),
            "partial_transfer" => (
                diagnostic::Stage::Arguments,
                diagnostic::Operation::CallBind,
            ),
            _ => (diagnostic::Stage::Parse, diagnostic::Operation::SyntaxParse),
        };
        diagnostic.data.insert(
            "pending_scope".into(),
            diagnostic::DataValue::String((*scope).into()),
        );
        if *feature == "unsupported_type" {
            diagnostic.data.insert(
                "type_spelling".into(),
                diagnostic::DataValue::String("Future<i64>".into()),
            );
        }
        assert!(
            diagnostic.to_json("input.nx", " ").is_ok(),
            "{feature}: {:?}",
            diagnostic.to_json("input.nx", " ")
        );
    }
}

#[test]
fn invalid_utf8_text_uses_zero_coordinates_from_source_view() {
    let mut d = diagnostic_for_catalog("NX-E0004", Some("invalid_utf8"));
    d.primary_span.as_mut().unwrap().start = 1;
    d.primary_span.as_mut().unwrap().end = 2;
    let rendered = d
        .render_text_view(
            "input.nx",
            diagnostic::SourceView {
                bytes_len: 2,
                text: None,
            },
        )
        .unwrap();
    assert_eq!(
        rendered,
        concat!(
            "NX-E0004:0:0: fixture\n",
            "  namespace: Nexo-proposed-v1\n  profile: f0-0002-b-v2\n  stage: input\n  operation: input.decode\n",
            "  at: {\"end\":2,\"file\":\"input.nx\",\"start\":1}\n",
            "  subcause: invalid_utf8\n  data: {\"invalid_bytes_hex\":\"ff\"}\n"
        )
    );
    assert!(d.render_text("input.nx", "a").is_err());
}

#[test]
fn text_rendering_matches_unicode_crlf_and_l07_positions_completely() {
    let cases = [
        ("// á🙂\nfn Main(){\n let a = missing;\n}", 30, 37, 10),
        ("// á🙂\r\nfn Main(){\r\nlet x=missing;\r\n}", 29, 36, 7),
        ("// á🙂\nfn Main(){\nlet x=missing;\n}", 27, 34, 7),
    ];
    for (source, start, end, column) in cases {
        let mut d = diagnostic_for_catalog("NX-E0101", None);
        d.primary_span.as_mut().unwrap().start = start;
        d.primary_span.as_mut().unwrap().end = end;
        let text = d.render_text("input.nx", source).unwrap();
        let expected = format!(
            "NX-E0101:3:{column}: fixture\n  namespace: Nexo-proposed-v1\n  profile: f0-0002-b-v2\n  stage: names\n  operation: names.resolve\n  at: {{\"end\":{end},\"file\":\"input.nx\",\"start\":{start}}}\n  data: {{\"use_kind\":\"value\"}}\n"
        );
        assert_eq!(text, expected);
    }
}

#[test]
fn explain_returns_catalog_rows_and_exact_feature_cardinality() {
    let features = diagnostic::explain("NX-E0006", None).unwrap();
    assert_eq!(features.len(), 30);
    assert_eq!(features.first().unwrap().feature_ids, ["async"]);
    assert_eq!(features.last().unwrap().feature_ids, ["user_types"]);
    assert!(
        features
            .windows(2)
            .all(|pair| { pair[0].feature_ids[0].as_bytes() < pair[1].feature_ids[0].as_bytes() })
    );
    assert!(
        features.iter().all(|entry| {
            !entry.negative_source.is_empty() && !entry.positive_source.is_empty()
        })
    );
    assert!(
        features
            .iter()
            .all(|entry| entry.repair_consequences.is_empty())
    );
    let call_causes = diagnostic::explain("NX-E0111", None).unwrap();
    assert_eq!(call_causes.len(), 7);
    assert!(
        call_causes
            .iter()
            .any(|entry| entry.subcause.as_deref() == Some("unknown_argument_label"))
    );
    assert_eq!(
        diagnostic::explain("NX-E0006", Some("float_literal")),
        Err(ExplainError::UnknownSubcause)
    );
    assert_eq!(
        diagnostic::explain("NX-E9999", None),
        Err(ExplainError::UnknownCode)
    );
}

#[test]
fn explain_subcauses_are_returned_in_lexicographic_order() {
    let actual: Vec<_> = diagnostic::explain("NX-E0111", None)
        .unwrap()
        .into_iter()
        .map(|entry| entry.subcause.unwrap())
        .collect();
    assert_eq!(
        actual,
        [
            "argument_mode_mismatch",
            "duplicate_argument",
            "excess_positional_argument",
            "missing_argument",
            "non_callable_target",
            "positional_after_named",
            "unknown_argument_label",
        ]
    );
}

#[test]
fn explain_accepts_the_entry_subcauses_and_reports_availability() {
    let missing = diagnostic::explain("NX-E0112", Some("missing_main")).unwrap();
    assert_eq!(missing[0].availability, "in_profile");
    assert_eq!(missing[0].explanation_id, "NX-E0112/missing_main");
    let invalid = diagnostic::explain("NX-E0112", Some("invalid_main_signature")).unwrap();
    assert_eq!(invalid[0].availability, "in_profile");
    assert_eq!(
        diagnostic::explain("NX-E0112", Some("invented")),
        Err(ExplainError::UnknownSubcause)
    );
    for (code, subcause, expected) in [
        ("NX-E0101", None, "in_profile"),
        ("NX-E0001", None, "in_profile"),
        ("NX-E0417", Some("duplicate_operation"), "in_profile"),
        ("NX-E0417", Some("missing_with_clause"), "outside_profile"),
        (
            "NX-E0102",
            Some("field_requires_grouped_call"),
            "outside_profile",
        ),
        ("NX-E0203", Some("unsupported_move_position"), "historical"),
    ] {
        let item = diagnostic::explain(code, subcause).unwrap();
        assert_eq!(item[0].availability, expected, "{code}/{subcause:?}");
    }
}

#[test]
fn explain_cli_reports_unknown_code_subcause_and_argument_errors() {
    let exe = env!("CARGO_BIN_EXE_nexo");
    let unknown = Command::new(exe)
        .args(["explain", "NX-E9999"])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(unknown.stderr, b"nexo-explain:unknown_code\n");
    let bad_subcause = Command::new(exe)
        .args(["explain", "NX-E0006", "--subcause", "float_literal"])
        .output()
        .unwrap();
    assert_eq!(bad_subcause.status.code(), Some(2));
    assert_eq!(bad_subcause.stderr, b"nexo-explain:unknown_subcause\n");
    let bad_args = Command::new(exe)
        .args(["explain", "NX-E0001", "unexpected"])
        .output()
        .unwrap();
    assert_eq!(bad_args.status.code(), Some(2));
    assert_eq!(bad_args.stderr, b"nexo-explain:invalid_arguments\n");
}

#[test]
fn explain_json_is_single_line_and_has_documented_error_shape() {
    let exe = env!("CARGO_BIN_EXE_nexo");
    let good = Command::new(exe)
        .args(["explain", "NX-E0006", "--format", "json"])
        .output()
        .unwrap();
    assert!(good.status.success());
    assert!(good.stderr.is_empty());
    let output = String::from_utf8(good.stdout).unwrap();
    assert_eq!(output.lines().count(), 1);
    assert!(output.contains("\"entries\":["));
    assert_eq!(output.matches("\"feature_ids\":[").count(), 30);
    let bad = Command::new(exe)
        .args(["explain", "NX-E9999", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stderr.is_empty());
    assert_eq!(
        String::from_utf8(bad.stdout).unwrap(),
        "{\"schema_version\":1,\"error\":{\"reason\":\"unknown_code\",\"code\":\"NX-E9999\",\"subcause\":null}}\n"
    );
    let unknown_subcause = Command::new(exe)
        .args([
            "explain",
            "NX-E0006",
            "--subcause",
            "float_literal",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(unknown_subcause.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(unknown_subcause.stdout).unwrap(),
        "{\"schema_version\":1,\"error\":{\"reason\":\"unknown_subcause\",\"code\":\"NX-E0006\",\"subcause\":\"float_literal\"}}\n"
    );
    let missing_code = Command::new(exe)
        .args(["explain", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(missing_code.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(missing_code.stdout).unwrap(),
        "{\"schema_version\":1,\"error\":{\"reason\":\"invalid_arguments\",\"code\":null,\"subcause\":null}}\n"
    );
}

#[test]
fn invalid_format_usage_uses_requested_json_error_envelope() {
    let result = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args(["explain", "--bogus", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(result.stderr, b"");
    assert_eq!(
        std::str::from_utf8(&result.stdout).unwrap(),
        "{\"schema_version\":1,\"error\":{\"reason\":\"invalid_arguments\",\"code\":null,\"subcause\":null}}\n"
    );
}

#[cfg(unix)]
#[test]
fn explain_non_unicode_argument_returns_usage_error() {
    use std::os::unix::ffi::OsStrExt;
    let bad = std::ffi::OsStr::from_bytes(b"NX-\xff");
    let result = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .arg("explain")
        .arg(bad)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(result.stderr, b"nexo-explain:invalid_arguments\n");
}

#[cfg(windows)]
#[test]
fn explain_non_unicode_argument_returns_usage_error() {
    use std::os::windows::ffi::OsStringExt;
    let bad = std::ffi::OsString::from_wide(&[b'N' as u16, 0xd800]);
    let result = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .arg("explain")
        .arg(bad)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(result.stderr, b"nexo-explain:invalid_arguments\n");
}

const EXPECTED_CATALOG_ROWS: &[AvailabilityExpectation] = &[
    ("NX-E0001", &[(None, "in_profile")]),
    ("NX-E0002", &[(None, "in_profile")]),
    ("NX-E0003", &[(None, "in_profile")]),
    (
        "NX-E0004",
        &[
            (Some("source_unreadable"), "in_profile"),
            (Some("invalid_utf8"), "in_profile"),
        ],
    ),
    ("NX-E0005", &[(None, "in_profile")]),
    ("NX-E0006", &[(None, "in_profile")]),
    ("NX-E0101", &[(None, "in_profile")]),
    (
        "NX-E0102",
        &[
            (Some("type_mismatch"), "in_profile"),
            (Some("not_indexable"), "in_profile"),
            (Some("resource_required"), "in_profile"),
            (Some("field_requires_grouped_call"), "outside_profile"),
        ],
    ),
    ("NX-E0103", &[(None, "in_profile")]),
    ("NX-E0104", &[(None, "in_profile")]),
    ("NX-E0105", &[(None, "in_profile")]),
    ("NX-E0107", &[(None, "in_profile")]),
    ("NX-E0110", &[(None, "outside_profile")]),
    (
        "NX-E0111",
        &[
            (Some("missing_argument"), "in_profile"),
            (Some("excess_positional_argument"), "in_profile"),
            (Some("unknown_argument_label"), "in_profile"),
            (Some("duplicate_argument"), "in_profile"),
            (Some("positional_after_named"), "in_profile"),
            (Some("argument_mode_mismatch"), "in_profile"),
            (Some("non_callable_target"), "in_profile"),
        ],
    ),
    (
        "NX-E0112",
        &[
            (Some("missing_main"), "in_profile"),
            (Some("invalid_main_signature"), "in_profile"),
        ],
    ),
    ("NX-E0113", &[(None, "in_profile")]),
    ("NX-E0201", &[(None, "in_profile")]),
    (
        "NX-E0202",
        &[
            (Some("definitely_consumed"), "in_profile"),
            (Some("maybe_consumed"), "in_profile"),
        ],
    ),
    (
        "NX-E0203",
        &[
            (Some("unsupported_move_position"), "historical"),
            (Some("unsupported_resource_position"), "historical"),
            (Some("unsupported_resource_rebinding"), "historical"),
            (Some("unsupported_transfer_kind"), "historical"),
        ],
    ),
    ("NX-E0412", &[(None, "in_profile")]),
    ("NX-E0413", &[(None, "in_profile")]),
    ("NX-E0414", &[(None, "in_profile")]),
    ("NX-E0415", &[(None, "in_profile")]),
    ("NX-E0416", &[(None, "outside_profile")]),
    (
        "NX-E0417",
        &[
            (Some("duplicate_operation"), "in_profile"),
            (Some("invalid_operation"), "in_profile"),
            (Some("duplicate_restriction"), "in_profile"),
            (Some("missing_body_operation"), "outside_profile"),
            (Some("operation_list_required"), "outside_profile"),
            (Some("missing_with_clause"), "outside_profile"),
            (Some("empty_operation_list"), "in_profile"),
            (Some("unrestricted_mixed"), "in_profile"),
        ],
    ),
    ("NX-E1101", &[(None, "outside_profile")]),
    ("NX-E1102", &[(None, "outside_profile")]),
    ("NX-E1104", &[(None, "outside_profile")]),
    (
        "NX-E1105",
        &[
            (Some("active_borrow_conflict"), "in_profile"),
            (Some("permission_escalation"), "in_profile"),
            (Some("overlapping_mutable_borrows"), "in_profile"),
        ],
    ),
    ("NX-E1107", &[(None, "outside_profile")]),
    ("NX-E1108", &[(None, "outside_profile")]),
    ("NX-E1109", &[(None, "outside_profile")]),
    ("NX-E1110", &[(None, "outside_profile")]),
    ("NX-E1111", &[(None, "outside_profile")]),
    ("NX-E1112", &[(None, "outside_profile")]),
    ("NX-E1113", &[(None, "outside_profile")]),
    ("NX-E1114", &[(None, "outside_profile")]),
    ("NX-E1201", &[(None, "outside_profile")]),
    ("NX-E1202", &[(None, "outside_profile")]),
    ("NX-E1204", &[(None, "outside_profile")]),
    ("NX-E1205", &[(None, "outside_profile")]),
    (
        "NX-E1206",
        &[
            (Some("async_generator_declaration"), "outside_profile"),
            (Some("yield_in_async"), "outside_profile"),
        ],
    ),
    ("NX-E1208", &[(None, "outside_profile")]),
    ("NX-E1210", &[(None, "outside_profile")]),
    ("NX-E1211", &[(None, "outside_profile")]),
    ("NX-E1212", &[(None, "outside_profile")]),
    ("NX-E1213", &[(None, "outside_profile")]),
    (
        "NX-E1301",
        &[(Some("implicit_closure_body"), "outside_profile")],
    ),
    (
        "NX-E1302",
        &[
            (Some("missing_else"), "in_profile"),
            (Some("missing_branch_value"), "in_profile"),
            (Some("invalid_value_target"), "in_profile"),
            (Some("missing_value_operand"), "in_profile"),
            (Some("non_final_branch_value"), "in_profile"),
        ],
    ),
    ("NX-E1303", &[(None, "in_profile")]),
    (
        "NX-E1304",
        &[
            (Some("non_exhaustive"), "outside_profile"),
            (Some("unreachable_arm"), "outside_profile"),
        ],
    ),
    (
        "NX-E1305",
        &[
            (Some("guard_effect"), "outside_profile"),
            (Some("guard_mutation"), "outside_profile"),
            (Some("guard_transfer"), "outside_profile"),
        ],
    ),
    ("NX-E1307", &[(None, "outside_profile")]),
    (
        "NX-E1308",
        &[
            (Some("private_access"), "outside_profile"),
            (Some("private_reexport"), "outside_profile"),
        ],
    ),
    (
        "NX-E1309",
        &[
            (Some("alias_collision"), "outside_profile"),
            (Some("duplicate_import"), "outside_profile"),
        ],
    ),
    (
        "NX-E1310",
        &[
            (Some("unexported_type"), "outside_profile"),
            (Some("missing_provenance"), "outside_profile"),
            (Some("missing_signature_type"), "outside_profile"),
            (Some("missing_effect_bound"), "outside_profile"),
        ],
    ),
    (
        "NX-E1311",
        &[
            (Some("undeclared_dependency"), "outside_profile"),
            (Some("incompatible_versions"), "outside_profile"),
            (Some("package_cycle"), "outside_profile"),
        ],
    ),
    ("NX-E1312", &[(None, "outside_profile")]),
    ("NX-E1313", &[(None, "outside_profile")]),
    ("NX-E1314", &[(None, "outside_profile")]),
    ("NX-E1315", &[(None, "outside_profile")]),
    ("NX-E1316", &[(None, "outside_profile")]),
    ("NX-E1317", &[(None, "in_profile")]),
    ("NX-E1318", &[(None, "outside_profile")]),
    ("NX-E1319", &[(None, "outside_profile")]),
    ("NX-E1320", &[(None, "outside_profile")]),
    (
        "NX-E1321",
        &[
            (Some("duplicate_module_file"), "outside_profile"),
            (Some("path_collision"), "outside_profile"),
            (Some("module_file_missing"), "outside_profile"),
        ],
    ),
    ("NX-E1322", &[(None, "outside_profile")]),
    (
        "NX-E1323",
        &[
            (Some("alias_cycle"), "outside_profile"),
            (Some("constant_cycle"), "outside_profile"),
            (Some("recursive_value_layout"), "outside_profile"),
        ],
    ),
    ("NX-E1324", &[(None, "outside_profile")]),
];

#[test]
fn availability_matches_independent_full_registry_expectations() {
    // Independently transcribed from section 13.12 and the canonical code registry.
    const EXPECTED: &[AvailabilityExpectation] = &[
        ("NX-E0001", &[(None, "in_profile")]),
        ("NX-E0002", &[(None, "in_profile")]),
        ("NX-E0003", &[(None, "in_profile")]),
        (
            "NX-E0004",
            &[
                (Some("source_unreadable"), "in_profile"),
                (Some("invalid_utf8"), "in_profile"),
            ],
        ),
        ("NX-E0005", &[(None, "in_profile")]),
        ("NX-E0006", &[(None, "in_profile")]),
        ("NX-E0101", &[(None, "in_profile")]),
        (
            "NX-E0102",
            &[
                (Some("type_mismatch"), "in_profile"),
                (Some("not_indexable"), "in_profile"),
                (Some("resource_required"), "in_profile"),
                (Some("field_requires_grouped_call"), "outside_profile"),
            ],
        ),
        ("NX-E0103", &[(None, "in_profile")]),
        ("NX-E0104", &[(None, "in_profile")]),
        ("NX-E0105", &[(None, "in_profile")]),
        ("NX-E0107", &[(None, "in_profile")]),
        ("NX-E0110", &[(None, "outside_profile")]),
        (
            "NX-E0111",
            &[
                (Some("missing_argument"), "in_profile"),
                (Some("excess_positional_argument"), "in_profile"),
                (Some("unknown_argument_label"), "in_profile"),
                (Some("duplicate_argument"), "in_profile"),
                (Some("positional_after_named"), "in_profile"),
                (Some("argument_mode_mismatch"), "in_profile"),
                (Some("non_callable_target"), "in_profile"),
            ],
        ),
        (
            "NX-E0112",
            &[
                (Some("missing_main"), "in_profile"),
                (Some("invalid_main_signature"), "in_profile"),
            ],
        ),
        ("NX-E0113", &[(None, "in_profile")]),
        ("NX-E0201", &[(None, "in_profile")]),
        (
            "NX-E0202",
            &[
                (Some("definitely_consumed"), "in_profile"),
                (Some("maybe_consumed"), "in_profile"),
            ],
        ),
        (
            "NX-E0203",
            &[
                (Some("unsupported_move_position"), "historical"),
                (Some("unsupported_resource_position"), "historical"),
                (Some("unsupported_resource_rebinding"), "historical"),
                (Some("unsupported_transfer_kind"), "historical"),
            ],
        ),
        ("NX-E0412", &[(None, "in_profile")]),
        ("NX-E0413", &[(None, "in_profile")]),
        ("NX-E0414", &[(None, "in_profile")]),
        ("NX-E0415", &[(None, "in_profile")]),
        ("NX-E0416", &[(None, "outside_profile")]),
        (
            "NX-E0417",
            &[
                (Some("duplicate_operation"), "in_profile"),
                (Some("invalid_operation"), "in_profile"),
                (Some("duplicate_restriction"), "in_profile"),
                (Some("missing_body_operation"), "outside_profile"),
                (Some("operation_list_required"), "outside_profile"),
                (Some("missing_with_clause"), "outside_profile"),
                (Some("empty_operation_list"), "in_profile"),
                (Some("unrestricted_mixed"), "in_profile"),
            ],
        ),
        ("NX-E1101", &[(None, "outside_profile")]),
        ("NX-E1102", &[(None, "outside_profile")]),
        ("NX-E1104", &[(None, "outside_profile")]),
        (
            "NX-E1105",
            &[
                (Some("active_borrow_conflict"), "in_profile"),
                (Some("permission_escalation"), "in_profile"),
                (Some("overlapping_mutable_borrows"), "in_profile"),
            ],
        ),
        ("NX-E1107", &[(None, "outside_profile")]),
        ("NX-E1108", &[(None, "outside_profile")]),
        ("NX-E1109", &[(None, "outside_profile")]),
        ("NX-E1110", &[(None, "outside_profile")]),
        ("NX-E1111", &[(None, "outside_profile")]),
        ("NX-E1112", &[(None, "outside_profile")]),
        ("NX-E1113", &[(None, "outside_profile")]),
        ("NX-E1114", &[(None, "outside_profile")]),
        ("NX-E1201", &[(None, "outside_profile")]),
        ("NX-E1202", &[(None, "outside_profile")]),
        ("NX-E1204", &[(None, "outside_profile")]),
        ("NX-E1205", &[(None, "outside_profile")]),
        (
            "NX-E1206",
            &[
                (Some("async_generator_declaration"), "outside_profile"),
                (Some("yield_in_async"), "outside_profile"),
            ],
        ),
        ("NX-E1208", &[(None, "outside_profile")]),
        ("NX-E1210", &[(None, "outside_profile")]),
        ("NX-E1211", &[(None, "outside_profile")]),
        ("NX-E1212", &[(None, "outside_profile")]),
        ("NX-E1213", &[(None, "outside_profile")]),
        (
            "NX-E1301",
            &[(Some("implicit_closure_body"), "outside_profile")],
        ),
        (
            "NX-E1302",
            &[
                (Some("missing_else"), "in_profile"),
                (Some("missing_branch_value"), "in_profile"),
                (Some("invalid_value_target"), "in_profile"),
                (Some("missing_value_operand"), "in_profile"),
                (Some("non_final_branch_value"), "in_profile"),
            ],
        ),
        ("NX-E1303", &[(None, "in_profile")]),
        (
            "NX-E1304",
            &[
                (Some("non_exhaustive"), "outside_profile"),
                (Some("unreachable_arm"), "outside_profile"),
            ],
        ),
        (
            "NX-E1305",
            &[
                (Some("guard_effect"), "outside_profile"),
                (Some("guard_mutation"), "outside_profile"),
                (Some("guard_transfer"), "outside_profile"),
            ],
        ),
        ("NX-E1307", &[(None, "outside_profile")]),
        (
            "NX-E1308",
            &[
                (Some("private_access"), "outside_profile"),
                (Some("private_reexport"), "outside_profile"),
            ],
        ),
        (
            "NX-E1309",
            &[
                (Some("alias_collision"), "outside_profile"),
                (Some("duplicate_import"), "outside_profile"),
            ],
        ),
        (
            "NX-E1310",
            &[
                (Some("unexported_type"), "outside_profile"),
                (Some("missing_provenance"), "outside_profile"),
                (Some("missing_signature_type"), "outside_profile"),
                (Some("missing_effect_bound"), "outside_profile"),
            ],
        ),
        (
            "NX-E1311",
            &[
                (Some("undeclared_dependency"), "outside_profile"),
                (Some("incompatible_versions"), "outside_profile"),
                (Some("package_cycle"), "outside_profile"),
            ],
        ),
        ("NX-E1312", &[(None, "outside_profile")]),
        ("NX-E1313", &[(None, "outside_profile")]),
        ("NX-E1314", &[(None, "outside_profile")]),
        ("NX-E1315", &[(None, "outside_profile")]),
        ("NX-E1316", &[(None, "outside_profile")]),
        ("NX-E1317", &[(None, "in_profile")]),
        ("NX-E1318", &[(None, "outside_profile")]),
        ("NX-E1319", &[(None, "outside_profile")]),
        ("NX-E1320", &[(None, "outside_profile")]),
        (
            "NX-E1321",
            &[
                (Some("duplicate_module_file"), "outside_profile"),
                (Some("path_collision"), "outside_profile"),
                (Some("module_file_missing"), "outside_profile"),
            ],
        ),
        ("NX-E1322", &[(None, "outside_profile")]),
        (
            "NX-E1323",
            &[
                (Some("alias_cycle"), "outside_profile"),
                (Some("constant_cycle"), "outside_profile"),
                (Some("recursive_value_layout"), "outside_profile"),
            ],
        ),
        ("NX-E1324", &[(None, "outside_profile")]),
    ];
    let mut combinations = 0;
    for (code, cases) in EXPECTED {
        for (subcause, availability) in *cases {
            let entries = diagnostic::explain(code, *subcause).unwrap();
            if *code == "NX-E0006" {
                assert_eq!(entries.len(), 30);
                assert!(entries.iter().all(|e| e.availability == *availability));
            } else {
                assert_eq!(entries.len(), 1, "{code}/{subcause:?}");
                assert_eq!(
                    entries[0].availability, *availability,
                    "{code}/{subcause:?}"
                );
            }
            combinations += 1;
        }
    }
    assert_eq!(EXPECTED.len(), 70);
    assert_eq!(combinations, 113);
}

fn diagnostic_for_catalog(code: &str, subcause: Option<&str>) -> diagnostic::Diagnostic {
    use diagnostic::{
        DataValue as V, Diagnostic, Namespace, Operation, RelatedRole, RelatedSpan, Severity, Span,
        Stage, Token, TokenKind,
    };
    use std::collections::BTreeMap;
    let (stage, operation) = match code {
        "NX-E0001" => (Stage::Lex, Operation::SyntaxLex),
        "NX-E0002" | "NX-E0005" | "NX-E0417" => (Stage::Parse, Operation::SyntaxParse),
        "NX-E1302" if subcause == Some("missing_branch_value") => {
            (Stage::Ownership, Operation::Branch)
        }
        "NX-E1302" => (Stage::Parse, Operation::SyntaxParse),
        "NX-E0003" => (Stage::Parse, Operation::AnalysisLimit),
        "NX-E0004" => (
            Stage::Input,
            if subcause == Some("invalid_utf8") {
                Operation::InputDecode
            } else {
                Operation::InputRead
            },
        ),
        "NX-E0006" => (Stage::Parse, Operation::SyntaxParse),
        "NX-E0101" => (Stage::Names, Operation::NamesResolve),
        "NX-E0102" | "NX-E1303" => (Stage::Types, Operation::TypesCheck),
        "NX-E0103" | "NX-E0113" => (Stage::Types, Operation::TypesInfer),
        "NX-E0104" => (Stage::Names, Operation::NamesDeclare),
        "NX-E0105" => (Stage::Ownership, Operation::PlaceWrite),
        "NX-E0107" => (Stage::Types, Operation::ValueLiteral),
        "NX-E0111" => (Stage::Arguments, Operation::CallBind),
        "NX-E0112" => (Stage::Entry, Operation::EntryCheck),
        "NX-E0201" => (Stage::Ownership, Operation::ValueCopy),
        "NX-E0202" => (Stage::Ownership, Operation::OwnershipUse),
        "NX-E0412" => (Stage::Effects, Operation::BuildArray),
        "NX-E0413" => (Stage::Effects, Operation::ValueDrop),
        "NX-E0414" => (Stage::Effects, Operation::IntrinsicCall),
        "NX-E0415" => (Stage::Effects, Operation::IntrinsicCall),
        "NX-E1105" => (Stage::Ownership, Operation::OwnershipAccess),
        "NX-E1317" => (Stage::Ownership, Operation::Return),
        _ => panic!("unexpected in-profile fixture {code}/{subcause:?}"),
    };
    let mut data = BTreeMap::new();
    match code {
        "NX-E0003" => {
            data.insert("budget_kind".into(), V::String("syntax_nesting".into()));
            data.insert("limit".into(), V::UInt(128));
            data.insert("observed".into(), V::UInt(129));
        }
        "NX-E0004" if subcause == Some("source_unreadable") => {
            data.insert("input_path".into(), V::String("input.nx".into()));
        }
        "NX-E0004" => {
            data.insert("invalid_bytes_hex".into(), V::String("ff".into()));
        }
        "NX-E0006" => {
            data.insert("pending_scope".into(), V::String("later_adr".into()));
            if subcause == Some("unsupported_type") {
                data.insert("type_spelling".into(), V::String("Future<i64>".into()));
            }
        }
        "NX-E0101" => {
            data.insert("use_kind".into(), V::String("value".into()));
        }
        "NX-E0104" => {
            data.insert("declaration_kind".into(), V::String("binding".into()));
        }
        "NX-E0102" | "NX-E1303" => {
            data.insert("expected_type".into(), V::String("I64".into()));
            data.insert("found_type".into(), V::String("Bool".into()));
        }
        "NX-E0112" => {
            data.insert("entry".into(), V::String("Main".into()));
        }
        "NX-E0113" => {
            data.insert(
                "unresolved_components".into(),
                V::Array(vec![V::String("$.err".into())]),
            );
        }
        "NX-E0412" | "NX-E0413" | "NX-E0414" | "NX-E0415" => {
            data.insert("effect_model".into(), V::String("f0-0002-sim-v1".into()));
            let mut origin = BTreeMap::new();
            origin.insert("file".into(), V::String("input.nx".into()));
            origin.insert("start".into(), V::UInt(0));
            origin.insert("end".into(), V::UInt(1));
            data.insert("contract_origin".into(), V::Object(origin));
            data.insert("effect_evidence".into(), V::String("known_effect".into()));
            data.insert(
                "required_effect".into(),
                V::String(
                    match code {
                        "NX-E0412" => "alloc",
                        "NX-E0413" => "block",
                        "NX-E0414" => "suspend",
                        _ => "io",
                    }
                    .into(),
                ),
            );
        }
        "NX-E1105" => {
            let entity = |kind: &str, id: &str| {
                let mut fields = BTreeMap::new();
                fields.insert("kind".into(), V::String(kind.into()));
                fields.insert("id".into(), V::String(id.into()));
                V::Object(fields)
            };
            let (entity_kind, permission, access) = match subcause {
                Some("permission_escalation") => ("parameter", "read", "consume"),
                Some("active_borrow_conflict") => ("binding", "read", "borrow_inout"),
                _ => ("binding", "inout", "borrow_inout"),
            };
            data.insert("origin".into(), entity(entity_kind, "binding:0"));
            if subcause == Some("permission_escalation") {
                data.insert("parameter".into(), entity("parameter", "parameter:0"));
            } else {
                data.insert("binding".into(), entity("binding", "binding:0"));
                data.insert("loan".into(), entity("loan", "loan:0:0"));
            }
            data.insert("active_permission".into(), V::String(permission.into()));
            data.insert("requested_access".into(), V::String(access.into()));
        }
        _ => {}
    }
    let span = Span {
        file: "input.nx".into(),
        start: 0,
        end: 1,
    };
    let empty = Span {
        file: "input.nx".into(),
        start: 0,
        end: 0,
    };
    let mut d = Diagnostic {
        schema_version: diagnostic::SCHEMA_VERSION,
        diagnostic_namespace: Namespace::NexoProposedV1,
        profile_id: diagnostic::PROFILE_ID.into(),
        code: code.into(),
        subcause: subcause.map(str::to_owned),
        rule: subcause.map_or_else(|| code.to_owned(), |c| format!("{code}/{c}")),
        explanation_id: Some(subcause.map_or_else(|| code.to_owned(), |c| format!("{code}/{c}"))),
        severity: Severity::Error,
        stage,
        message: "fixture".into(),
        operation,
        primary_span: if code == "NX-E0004" && subcause == Some("source_unreadable") {
            None
        } else {
            Some(span.clone())
        },
        related_spans: vec![],
        cause_chain: vec![],
        feature_id: (code == "NX-E0006").then(|| "async".into()),
        unexpected_token: None,
        expected_tokens: vec![],
        expected_tokens_kind: None,
        parser_context: None,
        recognized_prefix: None,
        repair: vec![],
        data,
    };
    if code == "NX-E0006" {
        d.data
            .insert("pending_scope".into(), V::String("later_adr".into()));
    }
    if code == "NX-E0005" {
        d.unexpected_token = Some(Token {
            kind: TokenKind::Eof,
            text: Some(String::new()),
            span: empty.clone(),
        });
        d.parser_context = Some("statement_terminator".into());
        d.recognized_prefix = Some(empty.clone());
        d.related_spans.push(RelatedSpan {
            role: RelatedRole::PrecedingExpression,
            span: empty.clone(),
        });
        d.primary_span = Some(empty.clone());
        d.expected_tokens = vec![";".into()];
        d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::CanonicalInsertion);
    }
    if code == "NX-E0002" {
        d.unexpected_token = Some(Token {
            kind: TokenKind::Name,
            text: Some("x".into()),
            span: span.clone(),
        });
        d.parser_context = Some("grammar.Program".into());
        d.recognized_prefix = Some(empty.clone());
        d.expected_tokens = vec!["EOF".into()];
        d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::AdmissibleSet);
    }
    if code == "NX-E0001" {
        d.unexpected_token = Some(Token {
            kind: TokenKind::Invalid,
            text: Some("x".into()),
            span: span.clone(),
        });
        d.parser_context = Some("lexical_token".into());
        d.expected_tokens = vec!["valid_token".into()];
        d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::LexicalCategory);
        d.recognized_prefix = Some(empty);
    }
    if code == "NX-E0003" {
        refresh_budget_metadata(&mut d);
    }
    d
}

#[test]
fn explain_and_to_json_accept_every_emitible_catalog_combination() {
    const SOURCE: &str = "x";
    let mut accepted = 0;
    for (code, cases) in EXPECTED_CATALOG_ROWS {
        for (subcause, availability) in *cases {
            if *availability != "in_profile" {
                continue;
            }
            if *code == "NX-E0006" {
                for entry in diagnostic::explain(code, None).unwrap() {
                    let feature = entry.feature_ids[0].as_str();
                    let mut d = diagnostic_for_catalog(code, None);
                    d.feature_id = Some(feature.into());
                    d.data.insert(
                        "pending_scope".into(),
                        diagnostic::DataValue::String(
                            if feature == "implementation_limit" {
                                "I08"
                            } else if ["closure", "generator", "option", "match"].contains(&feature)
                            {
                                "F0-0003"
                            } else {
                                "later_adr"
                            }
                            .into(),
                        ),
                    );
                    match feature {
                        "function_value" => {
                            d.stage = diagnostic::Stage::Names;
                            d.operation = diagnostic::Operation::NamesResolve;
                        }
                        "implementation_limit"
                        | "inferred_non_unit_result"
                        | "resource_array"
                        | "option"
                        | "unsupported_type" => {
                            d.stage = diagnostic::Stage::Types;
                            d.operation = diagnostic::Operation::TypesCheck;
                        }
                        "partial_transfer" => {
                            d.stage = diagnostic::Stage::Arguments;
                            d.operation = diagnostic::Operation::CallBind;
                        }
                        _ => {}
                    }
                    if feature == "unsupported_type" {
                        d.data.insert(
                            "type_spelling".into(),
                            diagnostic::DataValue::String("Future<i64>".into()),
                        );
                    }
                    let json = d.to_json("input.nx", SOURCE);
                    assert!(json.is_ok(), "{code}/{feature}: {json:?}");
                    accepted += 1;
                }
            } else {
                let entries = diagnostic::explain(code, *subcause).unwrap();
                assert!(!entries.is_empty(), "{code}/{subcause:?}");
                let d = if code.starts_with('R') {
                    let mut d = diagnostic_for_catalog("NX-E0006", None);
                    d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
                    d.code = (*code).into();
                    d.rule = (*code).into();
                    d.explanation_id = None;
                    d.feature_id = None;
                    d.data.clear();
                    d.stage = diagnostic::Stage::Runtime;
                    d.operation = match *code {
                        "R0001" | "R0002" => diagnostic::Operation::RuntimeLimit,
                        "R0101" => diagnostic::Operation::ValueAdd,
                        "R0102" => diagnostic::Operation::ValueIndex,
                        "R0103" => diagnostic::Operation::ValueSub,
                        "R0201" => diagnostic::Operation::AssertEq,
                        "R0301" => diagnostic::Operation::Fail,
                        _ => unreachable!(),
                    };
                    if *code == "R0001" {
                        d.data
                            .insert("limit".into(), diagnostic::DataValue::UInt(1_000_000));
                        d.data
                            .insert("observed".into(), diagnostic::DataValue::UInt(1_000_001));
                        d.data.insert(
                            "work_consumed".into(),
                            diagnostic::DataValue::UInt(1_000_000),
                        );
                    } else if *code == "R0002" {
                        d.data.insert(
                            "runtime_budget_kind".into(),
                            diagnostic::DataValue::String("live_value_cells".into()),
                        );
                        d.data
                            .insert("limit".into(), diagnostic::DataValue::UInt(65_536));
                        d.data
                            .insert("observed".into(), diagnostic::DataValue::UInt(65_537));
                    }
                    d
                } else {
                    diagnostic_for_catalog(code, *subcause)
                };
                let json = d.to_json("input.nx", SOURCE);
                assert!(json.is_ok(), "{code}/{subcause:?}: {json:?}");
                accepted += 1;
            }
        }
    }
    let expected_types = [
        "I64",
        "Bool",
        "()",
        "MockResource",
        "MockBlockingResource",
        "Array<I64>",
        "Array<Bool>",
        "Array<Array<I64>>",
        "Result<I64,Bool>",
        "Result<Array<I64>,Bool>",
        "Array<Result<I64,Bool>>",
        "domain:copyable_data",
        "domain:result",
    ];
    let found_types = [
        "I64",
        "Bool",
        "()",
        "MockResource",
        "MockBlockingResource",
        "Array<I64>",
        "Result<I64,Bool>",
    ];
    let mut type_fixtures = 0;
    'pairs: for expected in expected_types {
        for found in found_types {
            if expected == found {
                continue;
            }
            let mut d = diagnostic_for_catalog("NX-E0102", Some("type_mismatch"));
            d.data.insert(
                "expected_type".into(),
                diagnostic::DataValue::String(expected.into()),
            );
            d.data.insert(
                "found_type".into(),
                diagnostic::DataValue::String(found.into()),
            );
            let json = d.to_json("input.nx", SOURCE);
            assert!(
                json.is_ok(),
                "§13.17 type fixture {expected} <- {found}: {json:?}"
            );
            accepted += 1;
            type_fixtures += 1;
            if type_fixtures == 59 {
                break 'pairs;
            }
        }
    }
    assert_eq!(type_fixtures, 59);
    assert_eq!(accepted, 135, "valid ADR constructed diagnostics");
    for code in ["NX-E0005", "NX-E0006", "R0101"] {
        let mut d = if code == "R0101" {
            let mut d = diagnostic_for_catalog("NX-E0006", None);
            d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
            d.code = "R0101".into();
            d.rule = "R0101".into();
            d.explanation_id = None;
            d.feature_id = None;
            d.data.clear();
            d.stage = diagnostic::Stage::Runtime;
            d.operation = diagnostic::Operation::ValueAdd;
            d
        } else {
            diagnostic_for_catalog(code, None)
        };
        if code == "NX-E0006" {
            d.feature_id = Some("float_literal".into());
        }
        assert!(d.validate("input.nx", SOURCE).is_ok(), "§13.7 {code}");
    }
}

#[test]
fn to_json_rejects_unknown_catalog_subcause_and_code() {
    let unknown_subcause = diagnostic_for_catalog("NX-E0112", Some("invented"))
        .to_json("input.nx", "x")
        .unwrap_err();
    assert_eq!(unknown_subcause.0, "unknown canonical subcause");

    let mut unknown_code = diagnostic_for_catalog("NX-E0112", Some("missing_main"));
    unknown_code.code = "NX-E9999".into();
    unknown_code.rule = "NX-E9999/missing_main".into();
    unknown_code.explanation_id = unknown_code.rule.clone().into();
    let error = unknown_code.to_json("input.nx", "x").unwrap_err();
    assert_eq!(error.0, "unknown canonical NX-E code");
}

#[test]
fn feature_explanations_match_literal_f01_f33_fixtures() {
    // Sources and paired edits are transcribed from section 14.6, independently of the implementation table.
    const FIXTURES: &[(&str, &str, &str)] = &[
        (
            "async",
            "async fn F(){} fn Main(){}",
            " fn F(){} fn Main(){}",
        ),
        (
            "attributes",
            "@derive(JsonEncode) fn Main(){}",
            " fn Main(){}",
        ),
        (
            "closure",
            "fn Main(){let f=fn[](){return ();};}",
            "fn Main(){let f=();}",
        ),
        (
            "compound_assignment",
            "fn Main(){var x=1;x+=1;}",
            "fn Main(){var x=1;x=x+1;}",
        ),
        (
            "comptime",
            "const x:I64=comptime 1;fn Main(){}",
            "fn Main(){}",
        ),
        (
            "default_argument",
            "fn F(x:view I64=1){} fn Main(){F(1);}",
            "fn F(x:view I64){} fn Main(){F(1);}",
        ),
        ("extensions", "extend I64{} fn Main(){}", " fn Main(){}"),
        (
            "float_literal",
            "fn Main(){let x=0.5;}",
            "fn Main(){let x=0;}",
        ),
        (
            "for_iteration",
            "fn Main(){for n in [1]{}}",
            "fn Main(){while false{}}",
        ),
        (
            "function_value",
            "fn F(){} fn Main(){let g=F;}",
            "fn F(){} fn Main(){F();}",
        ),
        (
            "generator",
            "gen fn G(){} fn Main(){}",
            " fn G(){} fn Main(){}",
        ),
        (
            "implementation_limit",
            "fn Main()->Result<(),I64>{return Ok(());}",
            "fn Main(){}",
        ),
        (
            "inferred_non_unit_result",
            "fn F(){return 1;} fn Main(){}",
            "fn F()->I64{return 1;} fn Main(){}",
        ),
        (
            "loop_control",
            "fn Main(){while false{break;}}",
            "fn Main(){while false{}}",
        ),
        (
            "match",
            "fn Main(){match true{true=>{},false=>{}}}",
            "fn Main(){if true{}else{}}",
        ),
        (
            "methods",
            "fn Main(){let a=[1];a.Foo();}",
            "fn Main(){let a=[1];Len(a);}",
        ),
        (
            "modules_visibility",
            "use std.app;fn Main(){}",
            "fn Main(){}",
        ),
        (
            "no_heap_alloc",
            "fn Main() with no_heap_alloc{}",
            "fn Main() with no_alloc{}",
        ),
        (
            "option",
            "fn F(x:view Option<I64>){} fn Main(){}",
            "fn F(x:view Result<I64,I64>){} fn Main(){}",
        ),
        (
            "partial_transfer",
            "fn Main(){let a=[1];let x=move a[0];}",
            "fn Main(){let a=[1];let x= a[0];}",
        ),
        (
            "provenance_clauses",
            "fn F(x:view I64)->I64 from(x){return x;} fn Main(){}",
            "fn F(x:view I64)->I64 {return x;} fn Main(){}",
        ),
        (
            "range",
            "fn Main(){let n=1;let x=0..<n;}",
            "fn Main(){let n=1;let x=0+n;}",
        ),
        (
            "receiver_mode",
            "fn F(){} fn Main(){move F();}",
            "fn F(){} fn Main(){ F();}",
        ),
        (
            "region",
            "fn Main(){region r(capacity:0){}}",
            "fn Main(){{}}",
        ),
        (
            "resource_array",
            "fn Main(){let a=[MockOpen(1)];}",
            "fn Main(){let a=MockOpen(1);}",
        ),
        ("text", "fn Main(){let x=\"hola\";}", "fn Main(){let x=1;}"),
        ("traits_generics", "trait T{} fn Main(){}", " fn Main(){}"),
        ("tuple", "fn Main(){let x=(1,2);}", "fn Main(){let x=(1);}"),
        (
            "unsupported_type",
            "fn F(x:view U32){} fn Main(){}",
            "fn F(x:view I64){} fn Main(){}",
        ),
        ("user_types", "struct S{} fn Main(){}", " fn Main(){}"),
    ];
    for (id, negative, positive) in FIXTURES {
        let entry = diagnostic::explain("NX-E0006", None)
            .unwrap()
            .into_iter()
            .find(|e| e.feature_ids == [*id])
            .unwrap();
        assert_eq!(entry.negative_source, *negative, "{id}");
        assert_eq!(entry.positive_source, *positive, "{id}");
        assert_eq!(
            entry.example_profile_id.as_deref(),
            Some("f0-0002-b-v2"),
            "{id}"
        );
        assert!(entry.repair_consequences.is_empty(), "{id}");
    }
    assert_eq!(FIXTURES.len(), 30);
}

#[test]
fn ordinary_explanations_match_literal_published_pairs_and_fixture_profile() {
    // Literal selected fixtures from sections 13.7, 13.13, 14.1-14.4, 14.7 and K09.
    const FIXTURES: &[(&str, Option<&str>, &str, &str)] = &[
        (
            "NX-E0002",
            None,
            "fn Main(){let match=1;}",
            "fn Main(){let match_value=1;}",
        ),
        (
            "NX-E0005",
            None,
            "fn F()->I64{return 1 } fn Main(){}",
            "fn F()->I64{return 1; } fn Main(){}",
        ),
        (
            "NX-E0111",
            Some("missing_argument"),
            "fn F(x:view I64){} fn Main(){F();}",
            "fn F(x:view I64){} fn Main(){F(1);}",
        ),
        (
            "NX-E0111",
            Some("excess_positional_argument"),
            "fn F(x:view I64){} fn Main(){F(1,2);}",
            "fn F(x:view I64){} fn Main(){F(1);}",
        ),
        (
            "NX-E0111",
            Some("unknown_argument_label"),
            "fn F(x:view I64){} fn Main(){F(y:1);}",
            "fn F(x:view I64){} fn Main(){F(x:1);}",
        ),
        (
            "NX-E0111",
            Some("duplicate_argument"),
            "fn F(x:view I64){} fn Main(){F(x:1,x:2);}",
            "fn F(x:view I64){} fn Main(){F(x:1);}",
        ),
        (
            "NX-E0111",
            Some("positional_after_named"),
            "fn F(x:view I64,y:view I64){} fn Main(){F(x:1,2);}",
            "fn F(x:view I64,y:view I64){} fn Main(){F(x:1,y:2);}",
        ),
        (
            "NX-E0111",
            Some("argument_mode_mismatch"),
            "fn F(x:edit I64){} fn Main(){var x=1;F(x);}",
            "fn F(x:edit I64){} fn Main(){var x=1;F(edit x);}",
        ),
        (
            "NX-E0111",
            Some("non_callable_target"),
            "fn Main(){let f=1;f();}",
            "fn F(){} fn Main(){F();}",
        ),
        (
            "NX-E0112",
            Some("missing_main"),
            "fn F(){}",
            "fn F(){} fn Main(){}",
        ),
        (
            "NX-E0112",
            Some("invalid_main_signature"),
            "fn Main(x:view I64){}",
            "fn Main(){}",
        ),
        (
            "NX-E0105",
            None,
            "fn F(x:edit I64){} fn Main(){let x=1;F(edit x);}",
            "fn F(x:edit I64){} fn Main(){var x=1;F(edit x);}",
        ),
        (
            "NX-E0113",
            None,
            "fn Main(){let r=Ok(1);}",
            "fn Main(){let r:Result<I64,I64>=Ok(1);}",
        ),
        (
            "NX-E0103",
            None,
            "fn Main(){let xs=[];}",
            "fn Main(){let xs:Array<I64>=[];}",
        ),
        (
            "NX-E1317",
            None,
            "fn F()->I64 { } fn Main(){}",
            "fn F()->I64 { return 1;} fn Main(){}",
        ),
        (
            "NX-E0102",
            Some("type_mismatch"),
            "fn F()->I64{return true;} fn Main(){}",
            "fn F()->I64{return 1;} fn Main(){}",
        ),
        (
            "NX-E0102",
            Some("not_indexable"),
            "fn Main(){let x=1[0];}",
            "fn Main(){let x=[1][0];}",
        ),
        (
            "NX-E0102",
            Some("resource_required"),
            "fn Main(){let x=1;Inspect(x);}",
            "fn Main(){let x=MockOpen(1);Inspect(x);}",
        ),
        (
            "NX-E0107",
            None,
            "fn Main() { let x = 9223372036854775808; }",
            "fn Main() { let x = 9223372036854775807; }",
        ),
        (
            "NX-E0104",
            None,
            "fn F(){} fn F(){} fn Main(){}",
            "fn F(){} fn G(){} fn Main(){}",
        ),
        (
            "NX-E0101",
            None,
            "fn Main(){ausente();}",
            "fn Ausente(){} fn Main(){Ausente();}",
        ),
        (
            "NX-E0002",
            None,
            "fn Main(){let match=1;}",
            "fn Main(){let match_value=1;}",
        ),
        (
            "NX-E0201",
            None,
            "fn Main(){let r=MockOpen(1);let s=r;}",
            "fn Main(){let r=MockOpen(1);let s=move r;}",
        ),
        (
            "NX-E0202",
            Some("definitely_consumed"),
            "fn Main(){let r=MockOpen(1);let s=move r;Inspect(r);}",
            "fn Main(){let r=MockOpen(1);let s=move r;Inspect(s);}",
        ),
        (
            "NX-E0202",
            Some("maybe_consumed"),
            "fn F(b:view Bool){let r=MockOpen(1);if b{Drop(r);}Inspect(r);} fn Main(){}",
            "fn F(b:view Bool){let r=MockOpen(1);if b{Drop(r);return;}Inspect(r);} fn Main(){}",
        ),
        (
            "NX-E1105",
            Some("permission_escalation"),
            "fn F(x:view MockResource)->MockResource{return move x;} fn Main(){}",
            "fn F(x:move MockResource)->MockResource{return move x;} fn Main(){}",
        ),
        (
            "NX-E1105",
            Some("overlapping_mutable_borrows"),
            "fn F(x:edit I64,y:edit I64){} fn Main(){var x=1;F(edit x,edit x);}",
            "fn F(x:edit I64,y:edit I64){} fn Main(){var x=1;var y=2;F(edit x,edit y);}",
        ),
        (
            "NX-E1105",
            Some("active_borrow_conflict"),
            "fn F(x:view I64,y:edit I64){} fn Main(){var x=1;F(x,edit x);}",
            "fn F(x:view I64,y:edit I64){} fn Main(){var x=1;F(x+0,edit x);}",
        ),
        (
            "NX-E1302",
            Some("missing_else"),
            "fn F(b:view Bool)->I64{return if b then {value:1;};} fn Main(){}",
            "fn F(b:view Bool)->I64{return if b then {value:1;}else{value:2;};} fn Main(){}",
        ),
        (
            "NX-E1302",
            Some("missing_branch_value"),
            "fn F()->I64{return if true then {1;}else{value:2;};} fn Main(){}",
            "fn F()->I64{return if true then {value:1;}else{value:2;};} fn Main(){}",
        ),
        (
            "NX-E1302",
            Some("invalid_value_target"),
            "fn Main(){value:1;}",
            "fn Main(){let value=1;}",
        ),
        (
            "NX-E1302",
            Some("missing_value_operand"),
            "fn F()->I64{return if true then {value:}else{value:2;};} fn Main(){}",
            "fn F()->I64{return if true then {value:1;}else{value:2;};} fn Main(){}",
        ),
        (
            "NX-E1302",
            Some("non_final_branch_value"),
            "fn F()->I64{return if true then {value:1;2;}else{value:2;};} fn Main(){}",
            "fn F()->I64{return if true then {value:1;}else{value:2;};} fn Main(){}",
        ),
        (
            "NX-E1303",
            None,
            "fn F(){let n=if true then {value:1;}else{value:true;};} fn Main(){}",
            "fn F(){let n=if true then {value:1;}else{value:2;};} fn Main(){}",
        ),
        (
            "NX-E0417",
            Some("duplicate_operation"),
            "fn Main() with {call(no_io),call(no_alloc)}{}",
            "fn Main() with {call(no_io)}{}",
        ),
        (
            "NX-E0417",
            Some("invalid_operation"),
            "fn Main() with {resume(no_io)}{}",
            "fn Main() with {call(no_io)}{}",
        ),
        (
            "NX-E0417",
            Some("duplicate_restriction"),
            "fn Main() with no_io,no_io{}",
            "fn Main() with no_io{}",
        ),
        (
            "NX-E0417",
            Some("empty_operation_list"),
            "fn Main() with {}{}",
            "fn Main() with {call(unrestricted)}{}",
        ),
        (
            "NX-E0417",
            Some("unrestricted_mixed"),
            "fn Main() with unrestricted,no_io{}",
            "fn Main() with no_io{}",
        ),
        (
            "NX-E0412",
            None,
            "fn Main() with no_alloc{let a=[1];}",
            "fn Main() with no_alloc{let a=1;}",
        ),
        (
            "NX-E0413",
            None,
            "fn Main() with no_block{let r=MockBlockingOpen(1);}",
            "fn Main() with no_block{let r=MockOpen(1);}",
        ),
        (
            "NX-E0414",
            None,
            "fn Main() with no_suspend{MockUnknown();}",
            "fn Main() with no_suspend{AssertEq(1,1);}",
        ),
        (
            "NX-E0415",
            None,
            "fn Hoja(){Print(1);} fn Medio(){Hoja();} fn Main() with no_io{Medio();}",
            "fn Hoja(){} fn Medio(){Hoja();} fn Main() with no_io{Medio();}",
        ),
        (
            "NX-E0001",
            None,
            "fn Main(){let x=1!2;}",
            "fn Main(){let x=1+2;}",
        ),
    ];
    for (code, subcause, negative, positive) in FIXTURES {
        let entry = diagnostic::explain(code, *subcause).unwrap().remove(0);
        assert_eq!(entry.negative_source, *negative, "{code}/{subcause:?}");
        assert_eq!(entry.positive_source, *positive, "{code}/{subcause:?}");
        if !matches!(*code, "NX-E0412" | "NX-E0413" | "NX-E0414" | "NX-E0415") {
            let errors = if *code == "NX-E0113" {
                // I08 keeps the catalog's type rule, while compilation reports
                // the temporary implementation limit at the same source site.
                nexo::check::check_proposed("catalog_example.nx", negative, true)
                    .err()
                    .unwrap_or_else(|| panic!("catalog negative did not reject: {negative}"))
            } else {
                match nexo::ir::compile_scalar("catalog_example.nx", negative) {
                    Err(nexo::ir::CompileError::Rejected(errors)) => errors,
                    other => {
                        panic!("catalog negative did not reject: {code}/{subcause:?}: {other:?}")
                    }
                }
            };
            assert_eq!(errors[0].code, *code, "catalog negative: {negative}");
            assert_eq!(
                errors[0].subcause.as_deref(),
                *subcause,
                "catalog negative: {negative}"
            );
            let repaired = nexo::check::check_proposed("catalog_example.nx", positive, true);
            assert!(
                repaired.is_ok(),
                "catalog positive: {code}/{subcause:?}: {positive}: {repaired:?}"
            );
        }
        assert_eq!(entry.example_profile_id.as_deref(), Some("f0-0002-b-v2"));
        assert_eq!(
            entry.repair_consequences,
            if *code == "NX-E0005" {
                vec!["Termina el binding antes del cierre del bloque."]
            } else {
                vec![]
            }
        );
    }
    assert_eq!(FIXTURES.len(), 44);

    let budget = diagnostic::explain("NX-E0003", None).unwrap().remove(0);
    let mut negative = String::from("fn Main(){var x=0;x=");
    negative.push_str(&"[".repeat(200));
    negative.push('1');
    negative.push_str(&"]".repeat(200));
    negative.push_str(";}");
    assert_eq!(budget.negative_source, negative);
    assert_eq!(budget.positive_source, "fn Main(){var x=0;x=0;}");
    assert_eq!(budget.example_profile_id.as_deref(), Some("f0-0002-b-v2"));
}

#[test]
fn published_fixture_gaps_stay_empty_and_unavailable() {
    for (code, subcause, availability) in [
        ("NX-E0004", Some("source_unreadable"), "in_profile"),
        ("NX-E0004", Some("invalid_utf8"), "in_profile"),
        (
            "NX-E0102",
            Some("field_requires_grouped_call"),
            "outside_profile",
        ),
        ("NX-E0203", Some("unsupported_move_position"), "historical"),
        ("NX-E0417", Some("missing_with_clause"), "outside_profile"),
        (
            "NX-E0417",
            Some("operation_list_required"),
            "outside_profile",
        ),
        (
            "NX-E0417",
            Some("missing_body_operation"),
            "outside_profile",
        ),
    ] {
        let item = diagnostic::explain(code, subcause).unwrap().remove(0);
        assert_eq!(item.availability, availability, "{code}/{subcause:?}");
        assert_eq!(item.negative_source, "");
        assert_eq!(item.positive_source, "");
        assert!(item.repair_consequences.is_empty());
        assert_eq!(item.example_profile_id, None);
    }
}

#[test]
fn code_specific_schema_rules_reject_one_broken_attribute_each() {
    use diagnostic::{
        Cause, CauseKind, CauseOperation, DataValue as V, Namespace, Operation, Stage, TokenKind,
    };

    fn rejects(d: diagnostic::Diagnostic, rule: &str) {
        let error = d.to_json("input.nx", "x").unwrap_err();
        assert!(error.0.contains(rule), "expected {rule:?}, got {error}");
    }
    fn base_r0101() -> diagnostic::Diagnostic {
        let mut d = diagnostic_for_catalog("NX-E0006", None);
        d.diagnostic_namespace = Namespace::F0002Sim;
        d.code = "R0101".into();
        d.rule = "R0101".into();
        d.explanation_id = None;
        d.feature_id = None;
        d.data.clear();
        d.stage = Stage::Runtime;
        d.operation = Operation::ValueAdd;
        d
    }

    let base_e0005 = || diagnostic_for_catalog("NX-E0005", None);
    let mut d = base_e0005();
    d.unexpected_token = None;
    rejects(d, "§13.6 NX-E0005 requires");
    let mut d = base_e0005();
    d.parser_context = None;
    rejects(d, "§13.6 NX-E0005 requires");
    let mut d = base_e0005();
    d.recognized_prefix = None;
    rejects(d, "§13.6 NX-E0005 requires");
    let mut d = base_e0005();
    d.related_spans.clear();
    rejects(d, "§13.15 NX-E0005 requires exactly one");
    let mut d = base_e0005();
    d.stage = Stage::Lex;
    rejects(d, "§13.12 NX-E0005");
    let mut d = base_e0005();
    d.primary_span = None;
    rejects(d, "§13.6 NX-E0005 requires");
    let mut d = base_e0005();
    d.unexpected_token.as_mut().unwrap().text = Some("EOF".into());
    rejects(d, "§13.6 EOF token text must be empty");
    let mut d = base_e0005();
    d.unexpected_token.as_mut().unwrap().span.start = 1;
    d.unexpected_token.as_mut().unwrap().span.end = 1;
    rejects(
        d,
        "§13.14 NX-E0005 unexpected_token span equals primary_span",
    );

    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.operation = Operation::ValueAdd;
    rejects(d, "§13.12 NX-E0006");
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.primary_span = None;
    rejects(d, "§13.12 NX-E0006");
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.data.remove("pending_scope");
    rejects(d, "§13.6 NX-E0006 requires data.pending_scope");
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.profile_id = "f0-0002-test-output".into();
    rejects(d, "unknown profile_id");

    for key in ["expected_type", "budget_kind", "use_kind", "pending_scope"] {
        let code = if key == "pending_scope" {
            "NX-E0001"
        } else {
            "NX-E0006"
        };
        let mut d = diagnostic_for_catalog(code, None);
        let value = match key {
            "budget_kind" => "source_bytes",
            "use_kind" => "value",
            "pending_scope" => "later_adr",
            _ => "i64",
        };
        d.data.insert(key.into(), V::String(value.into()));
        rejects(d, "§13.6 field is absent in other codes");
    }

    let mut d = base_r0101();
    d.stage = Stage::Parse;
    rejects(d, "§13.12 R0101");
    let mut d = base_r0101();
    d.operation = Operation::ValueSub;
    rejects(d, "§13.12 R0101");

    let mut d = diagnostic_for_catalog("NX-E0412", None);
    d.data.remove("effect_model");
    rejects(d, "§13.6 data.effect_model is required");
    let mut d = diagnostic_for_catalog("NX-E0412", None);
    d.data.remove("contract_origin");
    rejects(d, "§13.6 data.contract_origin is required");
    let mut d = diagnostic_for_catalog("NX-E0412", None);
    d.data.remove("effect_evidence");
    rejects(d, "§13.6 data.effect_evidence is required");
    let mut d = diagnostic_for_catalog("NX-E0412", None);
    d.data.remove("required_effect");
    rejects(d, "§13.6 data.required_effect is required");

    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.cause_chain.push(Cause {
        kind: CauseKind::Call,
        operation: CauseOperation::ValueDrop,
        span: None,
        entity: None,
        evidence: diagnostic::Evidence::Verified,
    });
    rejects(d, "§13.15 cause kind/operation correspondence");

    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.data
        .insert("expected_type".into(), V::String("i64".into()));
    rejects(d, "§13.6 field is absent in other codes");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.data
        .insert("budget_kind".into(), V::String("source_bytes".into()));
    rejects(d, "§13.6 field is absent in other codes");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.data.insert("use_kind".into(), V::String("value".into()));
    rejects(d, "§13.6 field is absent in other codes");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.data
        .insert("pending_scope".into(), V::String("later_adr".into()));
    rejects(d, "§13.6 field is absent in other codes");

    let eof = diagnostic_for_catalog("NX-E0005", None);
    assert_eq!(eof.unexpected_token.as_ref().unwrap().kind, TokenKind::Eof);
}

#[test]
fn adr_code_contract_rejects_each_broken_coordinate_and_data_rule() {
    use diagnostic::{DataValue as V, Operation, Stage};
    fn reject(d: diagnostic::Diagnostic, rule: &str) {
        let err = d.to_json("input.nx", "x").unwrap_err();
        assert!(err.0.contains(rule), "expected {rule:?}, got {err}");
    }

    let mut d = diagnostic_for_catalog("NX-E0101", None);
    d.stage = Stage::Runtime;
    reject(d, "§13.12");

    let mut d = diagnostic_for_catalog("NX-E0101", None);
    d.operation = Operation::InputRead;
    reject(d, "§13.12");

    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.operation = Operation::SyntaxParse;
    reject(d, "§13.12");

    let mut d = diagnostic_for_catalog("NX-E0003", None);
    d.stage = Stage::Types;
    reject(d, "§13.13");

    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.feature_id = Some("function_value".into());
    d.stage = Stage::Runtime;
    d.operation = Operation::InputRead;
    reject(d, "§13.12");

    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.parser_context = None;
    reject(d, "§13.6 NX-E0002");

    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.unexpected_token = None;
    reject(d, "§13.6 NX-E0002");

    let mut d = diagnostic_for_catalog("NX-E1105", Some("permission_escalation"));
    d.data.clear();
    reject(d, "§13.6 NX-E1105/permission_escalation");

    let mut d = diagnostic_for_catalog("NX-E1105", Some("overlapping_mutable_borrows"));
    d.data
        .insert("active_permission".into(), V::String("inout".into()));
    d.data
        .insert("requested_access".into(), V::String("read".into()));
    reject(d, "§13.6 NX-E1105/overlapping_mutable_borrows");

    let mut d = diagnostic_for_catalog("NX-E0102", Some("type_mismatch"));
    d.data.remove("expected_type");
    reject(d, "§13.6/§13.17 NX-E0102");

    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.feature_id = Some("unsupported_type".into());
    d.stage = Stage::Types;
    d.operation = Operation::TypesCheck;
    d.data
        .insert("type_spelling".into(), V::String("Future<i64>".into()));
    d.data.remove("type_spelling");
    reject(d, "§13.6 NX-E0006/unsupported_type");

    for (code, key, value) in [
        ("NX-E0412", "active_permission", "read"),
        ("NX-E0101", "effect_model", "f0-0002-sim-v1"),
    ] {
        let mut d = diagnostic_for_catalog(code, None);
        d.data.insert(key.into(), V::String(value.into()));
        reject(d, "§13.6 data key");
    }

    let mut d = diagnostic_for_catalog("NX-E0112", Some("missing_main"));
    d.data.insert("entry".into(), V::String("start".into()));
    reject(d, "§13.6 NX-E0112");

    let mut d = diagnostic_for_catalog("NX-E0113", None);
    d.data.insert(
        "unresolved_components".into(),
        V::Array(vec![V::String("value.sub".into())]),
    );
    reject(d, "§13.17");

    let mut d = diagnostic_for_catalog("NX-E0004", Some("source_unreadable"));
    d.primary_span = Some(diagnostic::Span {
        file: "input.nx".into(),
        start: 0,
        end: 1,
    });
    reject(d, "§13.6 source_unreadable");
}

#[test]
fn nx_e1105_accepts_the_four_published_permission_fixtures() {
    use diagnostic::{DataValue as V, EntityKind};
    fn entity(kind: EntityKind, id: &str) -> V {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("kind".into(), V::String(kind.as_str().into()));
        fields.insert("id".into(), V::String(id.into()));
        V::Object(fields)
    }
    let cases = [
        (
            "permission_escalation",
            "read",
            "consume",
            EntityKind::Parameter,
            "parameter:5",
            Some((EntityKind::Parameter, "parameter:5")),
            None,
        ),
        (
            "overlapping_mutable_borrows",
            "inout",
            "borrow_inout",
            EntityKind::Binding,
            "binding:46",
            None,
            Some("loan:52:0"),
        ),
        (
            "active_borrow_conflict",
            "read",
            "borrow_inout",
            EntityKind::Binding,
            "binding:40",
            None,
            Some("loan:46:0"),
        ),
        (
            "overlapping_mutable_borrows",
            "inout",
            "borrow_inout",
            EntityKind::Binding,
            "binding:46",
            None,
            Some("loan:56:0"),
        ),
    ];
    for (cause, active, requested, root_kind, root_id, parameter, loan) in cases {
        let mut d = diagnostic_for_catalog("NX-E1105", Some(cause));
        d.data.insert("origin".into(), entity(root_kind, root_id));
        if let Some((kind, id)) = parameter {
            d.data.insert("parameter".into(), entity(kind, id));
        }
        if let Some(id) = loan {
            d.data.insert("loan".into(), entity(EntityKind::Loan, id));
        }
        if cause != "permission_escalation" {
            d.data
                .insert("binding".into(), entity(EntityKind::Binding, root_id));
        }
        d.data
            .insert("active_permission".into(), V::String(active.into()));
        d.data
            .insert("requested_access".into(), V::String(requested.into()));
        assert!(
            d.validate("input.nx", "x").is_ok(),
            "{cause}/{active}->{requested}"
        );
    }
}

#[test]
fn diagnostic_schema_rejects_single_attribute_mutations() {
    use diagnostic::{
        Cause, CauseKind, CauseOperation, DataValue as V, Edit, Repair, RepairStatus, Stage,
        TokenKind,
    };
    fn reject(d: diagnostic::Diagnostic, expected: &str) {
        let err = d.to_json("input.nx", "x").unwrap_err();
        assert!(err.0.contains(expected), "expected {expected:?}, got {err}");
    }
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.primary_span.as_mut().unwrap().file = "other.nx".into();
    reject(d, "span file differs");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.primary_span.as_mut().unwrap().start = 2;
    d.primary_span.as_mut().unwrap().end = 1;
    reject(d, "span is outside");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.rule = "NX-E0002".into();
    reject(d, "rule does not match");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.explanation_id = Some("NX-E0002".into());
    reject(d, "explanation_id mismatch");
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.feature_id = Some("not_a_feature".into());
    reject(d, "invalid NX-E0006 feature_id");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.feature_id = Some("async".into());
    reject(d, "feature_id is reserved for NX-E0006");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.expected_tokens = vec![";".into()];
    d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::CanonicalInsertion);
    d.expected_tokens_kind = None;
    reject(d, "expected_tokens_kind required");

    let mut d = diagnostic_for_catalog("NX-E0005", None);
    let span = diagnostic::Span {
        file: "input.nx".into(),
        start: 0,
        end: 0,
    };
    d.repair.push(Repair {
        id: "insert_terminator".into(),
        status: RepairStatus::Applicable,
        description: "insert".into(),
        edits: vec![Edit {
            span: span.clone(),
            expected_text: String::new(),
            replacement: ";".into(),
        }],
        validation: None,
        consequences: vec![],
    });
    reject(d, "applicable repair lacks accepted validation");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.repair.push(Repair {
        id: "insert_terminator".into(),
        status: RepairStatus::Applicable,
        description: "insert".into(),
        edits: vec![Edit {
            span: diagnostic::Span {
                file: "input.nx".into(),
                start: 0,
                end: 0,
            },
            expected_text: String::new(),
            replacement: ";".into(),
        }],
        validation: Some(diagnostic::RepairValidation {
            profile_id: diagnostic::PROFILE_ID.into(),
            mode: diagnostic::RepairMode::Check,
            result: diagnostic::RepairResult::Rejected,
        }),
        consequences: vec![],
    });
    reject(d, "applicable repair lacks accepted validation");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.repair.push(Repair {
        id: "insert_terminator".into(),
        status: RepairStatus::Applicable,
        description: "insert".into(),
        edits: vec![Edit {
            span: diagnostic::Span {
                file: "input.nx".into(),
                start: 0,
                end: 0,
            },
            expected_text: String::new(),
            replacement: ";".into(),
        }],
        validation: Some(diagnostic::RepairValidation {
            profile_id: "other-profile".into(),
            mode: diagnostic::RepairMode::Check,
            result: diagnostic::RepairResult::Accepted,
        }),
        consequences: vec![],
    });
    reject(d, "applicable repair lacks accepted validation");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.repair.push(Repair {
        id: "unknown".into(),
        status: RepairStatus::Proposed,
        description: "insert".into(),
        edits: vec![Edit {
            span,
            expected_text: String::new(),
            replacement: ";".into(),
        }],
        validation: None,
        consequences: vec![],
    });
    reject(d, "unknown Repair.id");
    let mut d = diagnostic_for_catalog("NX-E0001", None);
    d.data.insert("unknown_key".into(), V::String("x".into()));
    reject(d, "unknown data key");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.related_spans.push(d.related_spans[0].clone());
    reject(d, "related_spans contain an exact duplicate");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    let mut later = d.related_spans[0].clone();
    later.span.start = 1;
    later.span.end = 1;
    d.related_spans.insert(0, later);
    reject(d, "related_spans are not canonically ordered");
    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.expected_tokens = vec!["z".into(), "a".into()];
    d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::AdmissibleSet);
    reject(
        d,
        "admissible expected_tokens must be unique and UTF-8 sorted",
    );
    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.expected_tokens = vec!["a".into(), "a".into()];
    d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::AdmissibleSet);
    reject(
        d,
        "admissible expected_tokens must be unique and UTF-8 sorted",
    );

    let mut r = diagnostic_for_catalog("NX-E0006", None);
    r.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    r.code = "R0101".into();
    r.rule = "R0101".into();
    r.explanation_id = None;
    r.feature_id = None;
    r.data.clear();
    r.stage = Stage::Runtime;
    r.operation = diagnostic::Operation::ValueAdd;
    r.cause_chain.push(Cause {
        kind: CauseKind::Call,
        operation: CauseOperation::FunctionCall,
        span: None,
        entity: None,
        evidence: diagnostic::Evidence::Verified,
    });
    reject(r, "runtime diagnostics have empty cause_chain");

    let mut r = diagnostic_for_catalog("NX-E0006", None);
    r.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    r.code = "R0101".into();
    r.rule = "R0101".into();
    r.explanation_id = None;
    r.feature_id = None;
    r.data.clear();
    r.stage = Stage::Runtime;
    r.operation = diagnostic::Operation::ValueAdd;
    r.diagnostic_namespace = diagnostic::Namespace::NexoProposedV1;
    reject(r, "invalid runtime diagnostic identity");

    let mut d = diagnostic_for_catalog("NX-E0101", None);
    d.data
        .insert("effect_model".into(), V::String("f0-0002-sim-v1".into()));
    reject(d, "§13.6 data key");
    let mut d = diagnostic_for_catalog("NX-E0412", None);
    d.data
        .insert("active_permission".into(), V::String("read".into()));
    reject(d, "§13.6 data key");
    let _ = TokenKind::Decimal; // vocabulary literals are exercised by the accepted fixtures below.
}

#[test]
fn published_closed_vocabulary_literals_are_accepted() {
    use diagnostic::{
        Cause, CauseKind, CauseOperation, Evidence, Namespace, Operation, Stage, Token, TokenKind,
    };
    let mut d = diagnostic_for_catalog("NX-E0004", Some("source_unreadable"));
    d.stage = Stage::Input;
    d.operation = Operation::InputRead;
    d.primary_span = None;
    assert!(d.to_json("input.nx", "x").is_ok());
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.unexpected_token = Some(Token {
        kind: TokenKind::Decimal,
        text: Some("0.5".into()),
        span: d.primary_span.clone().unwrap(),
    });
    assert!(d.to_json("input.nx", "x").is_ok());
    d.unexpected_token = None;
    d.cause_chain.push(Cause {
        kind: CauseKind::Borrow,
        operation: CauseOperation::BorrowRead,
        span: None,
        entity: None,
        evidence: Evidence::Unknown,
    });
    assert!(d.to_json("input.nx", "x").is_ok());
    let mut r = diagnostic_for_catalog("NX-E0006", None);
    r.diagnostic_namespace = Namespace::F0002Sim;
    r.code = "R0103".into();
    r.rule = "R0103".into();
    r.explanation_id = None;
    r.feature_id = None;
    r.data.clear();
    r.stage = Stage::Runtime;
    r.operation = Operation::ValueSub;
    assert!(r.to_json("input.nx", "x").is_ok());
}

#[test]
fn code_validation_rejects_invalid_lexical_and_terminator_fields() {
    use diagnostic::{DataValue as V, Operation, Stage};
    fn reject(d: diagnostic::Diagnostic, source: &str, rule: &str) {
        let error = d.to_json("input.nx", source).unwrap_err();
        assert!(error.0.contains(rule), "expected {rule:?}; got {error}");
    }

    let base_lex = || diagnostic_for_catalog("NX-E0001", None);
    let mut d = base_lex();
    d.parser_context = Some("anything".into());
    reject(d, "x", "§13.14");
    let mut d = base_lex();
    d.expected_tokens = vec!["Name".into()];
    reject(d, "x", "§13.14");
    let mut d = base_lex();
    d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::AdmissibleSet);
    reject(d, "x", "§13.14");
    let mut d = base_lex();
    d.recognized_prefix.as_mut().unwrap().end = 1;
    reject(d, "x", "§13.14");
    let mut d = base_lex();
    d.unexpected_token = None;
    reject(d, "x", "§13.14");
    let mut d = base_lex();
    d.unexpected_token.as_mut().unwrap().text = Some("y".into());
    reject(d, "x", "§13.6");
    let mut d = base_lex();
    d.primary_span.as_mut().unwrap().end = 0;
    reject(d, "x", "§13.14 NX-E0001 unexpected_token span");

    let base_terminator = || diagnostic_for_catalog("NX-E0005", None);
    let mut d = base_terminator();
    d.expected_tokens = vec!["}".into()];
    reject(d, "x", "§13.14");
    let mut d = base_terminator();
    d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::AdmissibleSet);
    reject(d, "x", "§13.14");
    let mut d = base_terminator();
    d.parser_context = Some("grammar.Block".into());
    reject(d, "x", "§13.14");
    let mut d = base_terminator();
    d.recognized_prefix.as_mut().unwrap().end = 1;
    reject(d, "x", "§13.14");

    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.expected_tokens_kind = Some(diagnostic::ExpectedTokensKind::LexicalCategory);
    reject(d, "x", "§13.14");
    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.expected_tokens.clear();
    reject(d, "x", "§13.14");
    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.unexpected_token = None;
    reject(d, "x", "§13.6 NX-E0002 requires unexpected_token");
    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.unexpected_token.as_mut().unwrap().span.end = 0;
    reject(d, "x", "§13.14");
    let mut d = diagnostic_for_catalog("NX-E0002", None);
    d.unexpected_token.as_mut().unwrap().kind = diagnostic::TokenKind::Eof;
    d.unexpected_token.as_mut().unwrap().text = Some(String::new());
    d.unexpected_token.as_mut().unwrap().span.end = 1;
    reject(d, "x", "§13.6 EOF token span");

    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.unexpected_token.as_mut().unwrap().kind = diagnostic::TokenKind::Symbol;
    d.unexpected_token.as_mut().unwrap().text = Some("}".into());
    d.unexpected_token.as_mut().unwrap().span.end = 1;
    reject(d, "x", "§13.14");
    let mut d = diagnostic_for_catalog("NX-E0005", None);
    d.unexpected_token.as_mut().unwrap().span.end = 1;
    reject(d, "x", "§13.6 EOF token span");

    let mut d = diagnostic_for_catalog("NX-E0004", Some("invalid_utf8"));
    d.primary_span = None;
    reject(d, "x", "§13.6 NX-E0004/invalid_utf8");
    for hex in ["FF", "f", "fg", ""] {
        let mut d = diagnostic_for_catalog("NX-E0004", Some("invalid_utf8"));
        d.data
            .insert("invalid_bytes_hex".into(), V::String(hex.into()));
        reject(d, "x", "§13.6 invalid_bytes_hex");
    }
    let mut d = diagnostic_for_catalog("NX-E0004", Some("invalid_utf8"));
    d.data
        .insert("invalid_bytes_hex".into(), V::String("ffee".into()));
    reject(d, "x", "§13.6 invalid_bytes_hex length");

    let mut d = diagnostic_for_catalog("NX-E0003", None);
    d.data
        .insert("budget_kind".into(), V::String("source_bytes".into()));
    d.data.insert("limit".into(), V::UInt(1_048_576));
    d.data.insert("observed".into(), V::UInt(1_048_577));
    d.primary_span.as_mut().unwrap().start = 1_048_576;
    d.primary_span.as_mut().unwrap().end = 1_048_577;
    d.stage = Stage::Input;
    refresh_budget_metadata(&mut d);
    let long = "x".repeat(1_048_577);
    assert!(d.validate("input.nx", &long).is_ok());
    for (field, value) in [("limit", 9), ("observed", 9)] {
        let mut broken = d.clone();
        broken.data.insert(field.into(), V::UInt(value));
        reject(broken, &long, "§13.6 NX-E0003/source_bytes");
    }
    let mut d_bad_span = d.clone();
    d_bad_span.primary_span.as_mut().unwrap().end = 1_048_576;
    reject(d_bad_span, &long, "§13.6 NX-E0003/source_bytes");
    let mut d_bad_start = d.clone();
    d_bad_start.primary_span.as_mut().unwrap().start = 1_048_575;
    reject(d_bad_start, &long, "§13.6 NX-E0003/source_bytes");
    let mut d_missing_primary = d.clone();
    d_missing_primary.primary_span = None;
    reject(d_missing_primary, &long, "§13.6 NX-E0003/source_bytes");

    for bad_file in [true, false] {
        let mut d = diagnostic_for_catalog("NX-E0412", None);
        if bad_file {
            let V::Object(span) = d.data.get_mut("contract_origin").unwrap() else {
                unreachable!()
            };
            span.insert("file".into(), V::String("other.nx".into()));
        } else {
            let V::Object(span) = d.data.get_mut("contract_origin").unwrap() else {
                unreachable!()
            };
            span.insert("end".into(), V::UInt(99_999));
        }
        reject(d, "x", "§13.6 data.contract_origin");
    }

    for value in ["domain:foo", "?17", "Foo", "Array< i64 >", "Never"] {
        let mut d = diagnostic_for_catalog("NX-E0102", Some("type_mismatch"));
        d.data
            .insert("expected_type".into(), V::String(value.into()));
        reject(d, "x", "§13.17");
    }
    for value in ["domain:array", "domain:copyable_data", "domain:result"] {
        let mut d = diagnostic_for_catalog("NX-E0102", Some("type_mismatch"));
        d.data.insert("found_type".into(), V::String(value.into()));
        reject(d, "x", "§13.17");
    }
    for (cause, domain) in [
        ("not_indexable", "domain:array"),
        ("resource_required", "domain:mock_resource"),
        ("type_mismatch", "domain:copyable_data"),
        ("type_mismatch", "domain:result"),
    ] {
        let mut d = diagnostic_for_catalog("NX-E0102", Some(cause));
        d.data
            .insert("expected_type".into(), V::String(domain.into()));
        assert!(
            d.validate("input.nx", "x").is_ok(),
            "permitted domain {cause}/{domain}"
        );
    }
    let mut d = diagnostic_for_catalog("NX-E0102", Some("not_indexable"));
    d.data
        .insert("expected_type".into(), V::String("domain:result".into()));
    reject(d, "x", "§13.17 expected_type domain");

    let mut repair = diagnostic_for_catalog("NX-E0006", None);
    repair.repair.push(diagnostic::Repair {
        id: "insert_terminator".into(),
        status: diagnostic::RepairStatus::Proposed,
        description: "insert".into(),
        edits: vec![],
        validation: None,
        consequences: vec![],
    });
    reject(repair, "x", "§13.6 repair is empty");
    let mut effect = diagnostic_for_catalog("NX-E0412", None);
    effect
        .data
        .insert("effect_model".into(), V::String("wrong".into()));
    reject(effect, "x", "§13.6 effect_model");
    for code in ["NX-E0412", "NX-E0413", "NX-E0414", "NX-E0415"] {
        let mut d = diagnostic_for_catalog(code, None);
        d.stage = Stage::Runtime;
        reject(d, "x", "§13.12/§13.15 effect diagnostic coordinates");
    }

    for (code, expected) in [
        ("R0001", Operation::RuntimeLimit),
        ("R0002", Operation::RuntimeLimit),
        ("R0101", Operation::ValueAdd),
        ("R0102", Operation::ValueIndex),
        ("R0103", Operation::ValueSub),
        ("R0201", Operation::AssertEq),
        ("R0301", Operation::Fail),
    ] {
        let mut d = diagnostic_for_catalog("NX-E0006", None);
        d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
        d.code = code.into();
        d.rule = code.into();
        d.explanation_id = None;
        d.feature_id = None;
        d.data.clear();
        d.stage = Stage::Runtime;
        d.operation = expected;
        if code == "R0001" {
            d.data.insert("limit".into(), V::UInt(1));
            d.data.insert("observed".into(), V::UInt(2));
            d.data.insert("work_consumed".into(), V::UInt(1));
        }
        if code == "R0002" {
            d.data.insert(
                "runtime_budget_kind".into(),
                V::String("call_frames".into()),
            );
            d.data.insert("limit".into(), V::UInt(1));
            d.data.insert("observed".into(), V::UInt(2));
        }
        let bad = if expected == Operation::ValueAdd {
            Operation::ValueSub
        } else {
            Operation::ValueAdd
        };
        d.operation = bad;
        reject(d.clone(), "x", "§13.12");
        d.operation = expected;
        d.stage = Stage::Parse;
        reject(d, "x", "§13.12");
    }
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    d.code = "R0001".into();
    d.rule = "R0001".into();
    d.explanation_id = None;
    d.feature_id = None;
    d.data.clear();
    d.stage = Stage::Runtime;
    d.operation = Operation::RuntimeLimit;
    d.data.insert("limit".into(), V::UInt(1));
    d.data.insert("observed".into(), V::UInt(2));
    reject(d, "x", "work_consumed");
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    d.code = "R0001".into();
    d.rule = "R0001".into();
    d.explanation_id = None;
    d.feature_id = None;
    d.data.clear();
    d.stage = Stage::Runtime;
    d.operation = Operation::RuntimeLimit;
    d.data.insert("limit".into(), V::UInt(1));
    d.data.insert("observed".into(), V::UInt(2));
    d.data.insert("work_consumed".into(), V::UInt(2));
    reject(d, "x", "§13.6 R0001 work_consumed");
    let mut d = diagnostic_for_catalog("NX-E0006", None);
    d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    d.code = "R0001".into();
    d.rule = "R0001".into();
    d.explanation_id = None;
    d.feature_id = None;
    d.data.clear();
    d.stage = Stage::Runtime;
    d.operation = Operation::RuntimeLimit;
    d.data.insert("limit".into(), V::UInt(1));
    d.data
        .insert("observed".into(), V::UInt(9_007_199_254_740_992));
    d.data.insert("work_consumed".into(), V::UInt(1));
    reject(d, "x", "§13.6 data.observed must be a JSON-safe UInt");
    let d = diagnostic_for_catalog("NX-E0102", Some("field_requires_grouped_call"));
    reject(d, "x", "§13.12 selection is not emitible");
    let d = diagnostic_for_catalog("NX-E0417", Some("missing_with_clause"));
    reject(d, "x", "§13.12 selection is not emitible");
}

#[test]
fn exact_component_path_runtime_and_coordinate_contracts() {
    use diagnostic::{DataValue as V, RelatedRole};

    // The exact published active_borrow_conflict matrix includes inout/read.
    let mut borrow = diagnostic_for_catalog("NX-E1105", Some("active_borrow_conflict"));
    borrow
        .data
        .insert("active_permission".into(), V::String("inout".into()));
    borrow
        .data
        .insert("requested_access".into(), V::String("read".into()));
    assert!(borrow.validate("input.nx", "x").is_ok());

    // ComponentPath accepts only complete grammar productions and rejects adjacent misspellings.
    let mut path = diagnostic_for_catalog("NX-E0113", None);
    for value in ["$", "$.element", "$.ok", "$.err", "$.element.ok.err"] {
        path.data.insert(
            "unresolved_components".into(),
            V::Array(vec![V::String(value.into())]),
        );
        assert!(path.validate("input.nx", "x").is_ok(), "{value}");
    }
    for value in ["", ".ok", "$..ok", "$.okk", "$.elementx", "$.err.okk"] {
        path.data.insert(
            "unresolved_components".into(),
            V::Array(vec![V::String(value.into())]),
        );
        assert!(
            path.validate("input.nx", "x")
                .unwrap_err()
                .0
                .contains("ComponentPath"),
            "{value}"
        );
    }
    path.data.insert(
        "unresolved_components".into(),
        V::Array(vec![V::String("$.ok".into()), V::String("$.err".into())]),
    );
    assert!(path.validate("input.nx", "x").is_ok());
    let mut duplicate = path.clone();
    if let Some(V::Array(paths)) = duplicate.data.get_mut("unresolved_components") {
        paths[1] = V::String("$.ok".into());
    }
    let error = duplicate.validate("input.nx", "x").unwrap_err();
    assert!(
        error
            .0
            .contains("§13.17 unresolved_components paths must be unique"),
        "{error}"
    );

    // R0002 accepts every closed runtime budget kind, including observation_bytes.
    let mut runtime = diagnostic_for_catalog("NX-E0006", None);
    runtime.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    runtime.code = "R0002".into();
    runtime.rule = "R0002".into();
    runtime.explanation_id = None;
    runtime.feature_id = None;
    runtime.data.clear();
    runtime.stage = diagnostic::Stage::Runtime;
    runtime.operation = diagnostic::Operation::RuntimeLimit;
    runtime.data.insert(
        "runtime_budget_kind".into(),
        V::String("observation_bytes".into()),
    );
    runtime.data.insert("limit".into(), V::UInt(65_536));
    runtime.data.insert("observed".into(), V::UInt(65_537));
    assert!(runtime.validate("input.nx", "x").is_ok());

    // Effect diagnostics accept their published stage only with an allowed operation.
    let mut effect = diagnostic_for_catalog("NX-E0412", None);
    effect.operation = diagnostic::Operation::RuntimeLimit;
    assert!(
        effect
            .validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("effect diagnostic coordinates")
    );

    // NX-E0003/ir_items is the lower-stage coordinate in §13.13.
    let mut budget = diagnostic_for_catalog("NX-E0003", None);
    budget
        .data
        .insert("budget_kind".into(), V::String("ir_items".into()));
    budget.data.insert("limit".into(), V::UInt(262_144));
    budget.data.insert("observed".into(), V::UInt(262_145));
    budget.stage = diagnostic::Stage::Lower;
    budget.operation = diagnostic::Operation::AnalysisLimit;
    refresh_budget_metadata(&mut budget);
    assert!(budget.validate("input.nx", "x").is_ok());
    budget.stage = diagnostic::Stage::Ownership;
    assert!(
        budget
            .validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("budget_kind/stage mismatch")
    );

    let mut work = diagnostic_for_catalog("NX-E0006", None);
    work.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    work.code = "R0001".into();
    work.rule = "R0001".into();
    work.explanation_id = None;
    work.feature_id = None;
    work.data.clear();
    work.stage = diagnostic::Stage::Runtime;
    work.operation = diagnostic::Operation::RuntimeLimit;
    work.data.insert("limit".into(), V::UInt(1));
    work.data.insert("observed".into(), V::UInt(2));
    work.data
        .insert("work_consumed".into(), V::String("1".into()));
    assert!(
        work.validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("data.work_consumed must be a JSON-safe UInt")
    );

    // Role is the final key when file and offsets tie.
    let mut related = diagnostic_for_catalog("NX-E0005", None);
    let span = related.related_spans[0].span.clone();
    related.related_spans.clear();
    related.related_spans.push(diagnostic::RelatedSpan {
        role: RelatedRole::PreviousDeclaration,
        span: span.clone(),
    });
    related.related_spans.push(diagnostic::RelatedSpan {
        role: RelatedRole::Declaration,
        span: span.clone(),
    });
    related.related_spans.push(diagnostic::RelatedSpan {
        role: RelatedRole::PrecedingExpression,
        span,
    });
    assert!(
        related
            .validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("canonically ordered")
    );
}

#[test]
fn published_reduced_profiles_and_diagnostic_edge_contracts() {
    use diagnostic::{
        Cause, CauseKind, CauseOperation, DataValue as V, Evidence, Operation, RelatedRole, Stage,
    };

    for (profile, kind, stage) in [
        (
            "f0-0002-b-test-types-32-v2",
            "type_solver_steps",
            Stage::Types,
        ),
        (
            "f0-0002-b-test-flow-32-v2",
            "ownership_flow_steps",
            Stage::Ownership,
        ),
        (
            "f0-0002-b-test-effects-32-v2",
            "effect_fixpoint_steps",
            Stage::Effects,
        ),
        ("f0-0002-b-test-ir-32-v2", "ir_items", Stage::Lower),
    ] {
        let mut d = diagnostic_for_catalog("NX-E0003", None);
        d.profile_id = profile.into();
        d.stage = stage;
        d.operation = Operation::AnalysisLimit;
        d.data.insert("budget_kind".into(), V::String(kind.into()));
        d.data.insert("limit".into(), V::UInt(32));
        d.data.insert("observed".into(), V::UInt(33));
        refresh_budget_metadata(&mut d);
        assert!(d.validate("input.nx", "x").is_ok(), "{profile}");
        d.data
            .insert("budget_kind".into(), V::String("function_count".into()));
        d.stage = Stage::Parse;
        d.operation = Operation::AnalysisLimit;
        d.data.insert("limit".into(), V::UInt(1_024));
        d.data.insert("observed".into(), V::UInt(1_025));
        refresh_budget_metadata(&mut d);
        assert!(
            d.validate("input.nx", "x").is_ok(),
            "{profile} inherits normal quota"
        );
    }
    let mut wrong_kind = diagnostic_for_catalog("NX-E0003", None);
    wrong_kind.profile_id = "f0-0002-b-test-types-32-v2".into();
    wrong_kind.stage = Stage::Types;
    wrong_kind.operation = Operation::AnalysisLimit;
    wrong_kind
        .data
        .insert("budget_kind".into(), V::String("array_type_depth".into()));
    wrong_kind.data.insert("limit".into(), V::UInt(32));
    wrong_kind.data.insert("observed".into(), V::UInt(33));
    assert!(wrong_kind.validate("input.nx", "x").is_err());
    for (kind, stage) in [
        ("source_bytes", Stage::Input),
        ("syntax_nesting", Stage::Parse),
        ("expression_tree_depth", Stage::Parse),
        ("function_count", Stage::Parse),
        ("array_type_depth", Stage::Types),
        ("value_type_depth", Stage::Types),
        ("type_solver_steps", Stage::Types),
        ("ownership_flow_steps", Stage::Ownership),
        ("effect_fixpoint_steps", Stage::Effects),
        ("ir_items", Stage::Lower),
    ] {
        let mut d = diagnostic_for_catalog("NX-E0003", None);
        d.data.insert("budget_kind".into(), V::String(kind.into()));
        d.stage = stage;
        d.operation = Operation::AnalysisLimit;
        refresh_budget_metadata(&mut d);
        if kind == "source_bytes" {
            d.data.insert("limit".into(), V::UInt(1_048_576));
            d.data.insert("observed".into(), V::UInt(1_048_577));
            d.primary_span = Some(diagnostic::Span {
                file: "input.nx".into(),
                start: 1_048_576,
                end: 1_048_577,
            });
            let source = "x".repeat(1_048_577);
            assert!(d.validate("input.nx", &source).is_ok(), "{kind}");
        } else {
            let limit = match kind {
                "syntax_nesting"
                | "expression_tree_depth"
                | "array_type_depth"
                | "value_type_depth" => 128,
                "function_count" => 1_024,
                "type_solver_steps" | "ownership_flow_steps" | "effect_fixpoint_steps" => 1_000_000,
                "ir_items" => 262_144,
                _ => unreachable!(),
            };
            d.data.insert("limit".into(), V::UInt(limit));
            d.data.insert("observed".into(), V::UInt(limit + 1));
            refresh_budget_metadata(&mut d);
            assert!(d.validate("input.nx", "x").is_ok(), "{kind}");
        }
    }
    for (subcause, pairs) in [
        (
            "permission_escalation",
            &[
                ("read", "write"),
                ("read", "consume"),
                ("read", "drop"),
                ("read", "borrow_inout"),
                ("inout", "consume"),
                ("inout", "drop"),
            ][..],
        ),
        (
            "active_borrow_conflict",
            &[
                ("read", "write"),
                ("read", "consume"),
                ("read", "drop"),
                ("read", "borrow_inout"),
                ("inout", "read"),
                ("inout", "write"),
                ("inout", "consume"),
                ("inout", "drop"),
            ][..],
        ),
        (
            "overlapping_mutable_borrows",
            &[("inout", "borrow_inout")][..],
        ),
    ] {
        for (active, requested) in pairs {
            let mut d = diagnostic_for_catalog("NX-E1105", Some(subcause));
            d.data
                .insert("active_permission".into(), V::String((*active).into()));
            d.data
                .insert("requested_access".into(), V::String((*requested).into()));
            assert!(
                d.validate("input.nx", "x").is_ok(),
                "{subcause}: {active}/{requested}"
            );
        }
    }
    for (code, value) in [
        ("NX-E0412", "alloc"),
        ("NX-E0413", "block"),
        ("NX-E0414", "suspend"),
        ("NX-E0415", "io"),
    ] {
        let mut d = diagnostic_for_catalog(code, None);
        d.data
            .insert("required_effect".into(), V::String(value.into()));
        for evidence in ["known_effect", "unknown_effect"] {
            d.data
                .insert("effect_evidence".into(), V::String(evidence.into()));
            assert!(
                d.validate("input.nx", "x").is_ok(),
                "{code}/{value}/{evidence}"
            );
        }
    }
    for value in ["function", "parameter", "binding"] {
        let mut d = diagnostic_for_catalog("NX-E0104", None);
        d.data
            .insert("declaration_kind".into(), V::String(value.into()));
        assert!(
            d.validate("input.nx", "x").is_ok(),
            "declaration_kind={value}"
        );
    }
    for value in ["value", "call_target"] {
        let mut d = diagnostic_for_catalog("NX-E0101", None);
        d.data.insert("use_kind".into(), V::String(value.into()));
        assert!(d.validate("input.nx", "x").is_ok(), "use_kind={value}");
    }
    for (profile, kind, limit) in [
        ("f0-0002-b-test-frames-8-v2", "call_frames", 8),
        ("f0-0002-b-test-cells-64-v2", "live_value_cells", 64),
        ("f0-0002-b-test-observations-64-v2", "observation_bytes", 64),
    ] {
        let mut d = diagnostic_for_catalog("NX-E0006", None);
        d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
        d.code = "R0002".into();
        d.rule = "R0002".into();
        d.explanation_id = None;
        d.feature_id = None;
        d.data.clear();
        d.stage = Stage::Runtime;
        d.operation = Operation::RuntimeLimit;
        d.profile_id = profile.into();
        d.data
            .insert("runtime_budget_kind".into(), V::String(kind.into()));
        d.data.insert("limit".into(), V::UInt(limit));
        d.data.insert("observed".into(), V::UInt(limit + 1));
        assert!(d.validate("input.nx", "x").is_ok(), "{profile}");
    }
    let mut wrong_runtime_kind = diagnostic_for_catalog("NX-E0006", None);
    wrong_runtime_kind.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    wrong_runtime_kind.code = "R0002".into();
    wrong_runtime_kind.rule = "R0002".into();
    wrong_runtime_kind.explanation_id = None;
    wrong_runtime_kind.feature_id = None;
    wrong_runtime_kind.data.clear();
    wrong_runtime_kind.stage = Stage::Runtime;
    wrong_runtime_kind.operation = Operation::RuntimeLimit;
    wrong_runtime_kind.profile_id = "f0-0002-b-test-frames-8-v2".into();
    wrong_runtime_kind.data.insert(
        "runtime_budget_kind".into(),
        V::String("live_value_cells".into()),
    );
    wrong_runtime_kind.data.insert("limit".into(), V::UInt(8));
    wrong_runtime_kind
        .data
        .insert("observed".into(), V::UInt(9));
    assert!(wrong_runtime_kind.validate("input.nx", "x").is_err());

    let mut structural = diagnostic_for_catalog("NX-E0003", None);
    structural.primary_span = None;
    assert!(structural.validate("input.nx", "x").is_err());
    for kind in [
        "expression_tree_depth",
        "function_count",
        "array_type_depth",
        "value_type_depth",
    ] {
        let mut empty = diagnostic_for_catalog("NX-E0003", None);
        empty
            .data
            .insert("budget_kind".into(), V::String(kind.into()));
        empty.primary_span.as_mut().unwrap().end = 0;
        assert!(empty.validate("input.nx", "x").is_err(), "{kind}");
    }
    let mut syntax_eof = diagnostic_for_catalog("NX-E0003", None);
    syntax_eof
        .data
        .insert("budget_kind".into(), V::String("syntax_nesting".into()));
    syntax_eof.primary_span.as_mut().unwrap().end = 0;
    assert!(syntax_eof.validate("input.nx", "").is_ok());
    assert!(syntax_eof.validate("input.nx", "x").is_err());

    let mut terminator = diagnostic_for_catalog("NX-E0005", None);
    terminator.related_spans.clear();
    terminator.related_spans.push(diagnostic::RelatedSpan {
        role: RelatedRole::Context,
        span: diagnostic::Span {
            file: "input.nx".into(),
            start: 0,
            end: 1,
        },
    });
    assert!(terminator.validate("input.nx", "x").is_ok());
    terminator.related_spans.push(diagnostic::RelatedSpan {
        role: RelatedRole::PrecedingExpression,
        span: diagnostic::Span {
            file: "input.nx".into(),
            start: 0,
            end: 1,
        },
    });
    assert!(terminator.validate("input.nx", "x").is_err());

    let mut no_expression = diagnostic_for_catalog("NX-E0005", None);
    let source = "fn Main(){return}";
    let primary = diagnostic::Span {
        file: "input.nx".into(),
        start: 16,
        end: 17,
    };
    no_expression.primary_span = Some(primary.clone());
    no_expression.unexpected_token = Some(diagnostic::Token {
        kind: diagnostic::TokenKind::Symbol,
        text: Some("}".into()),
        span: primary,
    });
    no_expression.recognized_prefix = Some(diagnostic::Span {
        file: "input.nx".into(),
        start: 10,
        end: 16,
    });
    no_expression.related_spans.clear();
    no_expression.related_spans.push(diagnostic::RelatedSpan {
        role: RelatedRole::Context,
        span: diagnostic::Span {
            file: "input.nx".into(),
            start: 10,
            end: 16,
        },
    });
    no_expression
        .validate("input.nx", source)
        .unwrap_or_else(|error| panic!("{error}"));

    let mut work = diagnostic_for_catalog("NX-E0006", None);
    work.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
    work.code = "R0001".into();
    work.rule = "R0001".into();
    work.explanation_id = None;
    work.feature_id = None;
    work.data.clear();
    work.stage = Stage::Runtime;
    work.operation = Operation::RuntimeLimit;
    for observed in [0, 5_000] {
        work.data.insert("limit".into(), V::UInt(1_000));
        work.data.insert("observed".into(), V::UInt(observed));
        work.data.insert("work_consumed".into(), V::UInt(1_000));
        assert!(work.validate("input.nx", "x").is_err());
    }
    work.data.insert("observed".into(), V::UInt(1_001));
    assert!(work.validate("input.nx", "x").is_ok());

    let mut parameter = diagnostic_for_catalog("NX-E0104", None);
    parameter
        .data
        .insert("declaration_kind".into(), V::String("parameter".into()));
    assert!(parameter.validate("input.nx", "x").is_ok());
    let mut escalation = diagnostic_for_catalog("NX-E1105", Some("permission_escalation"));
    escalation
        .data
        .insert("active_permission".into(), V::String("inout".into()));
    escalation
        .data
        .insert("requested_access".into(), V::String("drop".into()));
    assert!(escalation.validate("input.nx", "x").is_ok());

    let mut cause = diagnostic_for_catalog("NX-E0412", None);
    cause.cause_chain.push(Cause {
        kind: CauseKind::Reborrow,
        operation: CauseOperation::BorrowRead,
        span: None,
        entity: None,
        evidence: Evidence::Unknown,
    });
    assert!(cause.validate("input.nx", "x").is_err());
    let mut function_count = diagnostic_for_catalog("NX-E0003", None);
    function_count
        .data
        .insert("budget_kind".into(), V::String("function_count".into()));
    function_count.data.insert("limit".into(), V::UInt(1_024));
    function_count
        .data
        .insert("observed".into(), V::UInt(1_025));
    function_count.stage = Stage::Parse;
    assert!(function_count.validate("input.nx", "x").is_ok());
    function_count.stage = Stage::Types;
    assert!(function_count.validate("input.nx", "x").is_err());
    for key in ["limit", "observed"] {
        let mut required = diagnostic_for_catalog("NX-E0003", None);
        required.data.remove(key);
        assert!(
            required.validate("input.nx", "x").is_err(),
            "NX-E0003 requires {key}"
        );
    }
}

#[test]
fn reduced_profiles_inherit_every_unmodified_budget() {
    use diagnostic::{DataValue as V, Operation, Stage};
    let analysis = [
        ("source_bytes", 1_048_576, Stage::Input),
        ("syntax_nesting", 128, Stage::Parse),
        ("expression_tree_depth", 128, Stage::Parse),
        ("array_type_depth", 128, Stage::Types),
        ("value_type_depth", 128, Stage::Types),
        ("function_count", 1_024, Stage::Parse),
        ("type_solver_steps", 1_000_000, Stage::Types),
        ("ownership_flow_steps", 1_000_000, Stage::Ownership),
        ("effect_fixpoint_steps", 1_000_000, Stage::Effects),
        ("ir_items", 262_144, Stage::Lower),
    ];
    let runtime = [
        ("call_frames", 256),
        ("live_value_cells", 65_536),
        ("observation_bytes", 65_536),
    ];
    let profiles = [
        ("f0-0002-b-v2", None),
        ("f0-0002-b-test-types-32-v2", Some("type_solver_steps")),
        ("f0-0002-b-test-flow-32-v2", Some("ownership_flow_steps")),
        (
            "f0-0002-b-test-effects-32-v2",
            Some("effect_fixpoint_steps"),
        ),
        ("f0-0002-b-test-ir-32-v2", Some("ir_items")),
        ("f0-0002-b-test-frames-8-v2", None),
        ("f0-0002-b-test-cells-64-v2", None),
        ("f0-0002-b-test-observations-64-v2", None),
    ];
    for (profile, changed) in profiles {
        for (kind, normal, stage) in analysis {
            let limit = if changed == Some(kind) { 32 } else { normal };
            let mut d = diagnostic_for_catalog("NX-E0003", None);
            d.profile_id = profile.into();
            d.stage = stage;
            d.operation = Operation::AnalysisLimit;
            d.data.insert("budget_kind".into(), V::String(kind.into()));
            d.data.insert("limit".into(), V::UInt(limit));
            d.data.insert("observed".into(), V::UInt(limit + 1));
            refresh_budget_metadata(&mut d);
            let source = if kind == "source_bytes" {
                "x".repeat(limit as usize + 1)
            } else {
                "x".into()
            };
            if kind == "source_bytes" {
                d.primary_span = Some(diagnostic::Span {
                    file: "input.nx".into(),
                    start: limit,
                    end: limit + 1,
                });
            }
            assert!(
                d.validate("input.nx", &source).is_ok(),
                "{profile}/{kind} at {limit}"
            );
            d.data.insert("limit".into(), V::UInt(limit - 1));
            d.data.insert("observed".into(), V::UInt(limit));
            assert!(
                d.validate("input.nx", &source).is_err(),
                "{profile}/{kind} wrong limit"
            );
        }
    }
    for (profile, changed) in [
        ("f0-0002-b-v2", None),
        ("f0-0002-b-test-frames-8-v2", Some("call_frames")),
        ("f0-0002-b-test-cells-64-v2", Some("live_value_cells")),
        (
            "f0-0002-b-test-observations-64-v2",
            Some("observation_bytes"),
        ),
        ("f0-0002-b-test-types-32-v2", None),
        ("f0-0002-b-test-flow-32-v2", None),
        ("f0-0002-b-test-effects-32-v2", None),
        ("f0-0002-b-test-ir-32-v2", None),
    ] {
        for (kind, normal) in runtime {
            let limit = if changed == Some(kind) {
                if kind == "call_frames" { 8 } else { 64 }
            } else {
                normal
            };
            let mut d = diagnostic_for_catalog("NX-E0006", None);
            d.diagnostic_namespace = diagnostic::Namespace::F0002Sim;
            d.code = "R0002".into();
            d.rule = "R0002".into();
            d.explanation_id = None;
            d.feature_id = None;
            d.profile_id = profile.into();
            d.stage = Stage::Runtime;
            d.operation = Operation::RuntimeLimit;
            d.data.clear();
            d.data
                .insert("runtime_budget_kind".into(), V::String(kind.into()));
            d.data.insert("limit".into(), V::UInt(limit));
            d.data.insert("observed".into(), V::UInt(limit + 1));
            assert!(
                d.validate("input.nx", "x").is_ok(),
                "{profile}/{kind} at {limit}"
            );
            d.data.insert("limit".into(), V::UInt(limit - 1));
            assert!(
                d.validate("input.nx", "x").is_err(),
                "{profile}/{kind} wrong limit"
            );
        }
    }
}

#[test]
fn nx_e0003_costly_analysis_requires_a_nonempty_primary_span() {
    use diagnostic::{DataValue as V, Operation, Span, Stage};

    let cases = [
        ("type_solver_steps", Stage::Types, 1_000_000),
        ("ownership_flow_steps", Stage::Ownership, 1_000_000),
        ("effect_fixpoint_steps", Stage::Effects, 1_000_000),
        ("ir_items", Stage::Lower, 262_144),
    ];
    for (kind, stage, limit) in cases {
        let mut diagnostic = diagnostic_for_catalog("NX-E0003", None);
        diagnostic.profile_id = "f0-0002-b-v2".into();
        diagnostic.stage = stage;
        diagnostic.operation = Operation::AnalysisLimit;
        diagnostic
            .data
            .insert("budget_kind".into(), V::String(kind.into()));
        diagnostic.data.insert("limit".into(), V::UInt(limit));
        diagnostic
            .data
            .insert("observed".into(), V::UInt(limit + 1));
        refresh_budget_metadata(&mut diagnostic);
        diagnostic.primary_span = Some(Span {
            file: "input.nx".into(),
            start: 0,
            end: 1,
        });
        assert!(
            diagnostic.validate("input.nx", "0123456789").is_ok(),
            "{kind} positive span"
        );

        for offset in [0, 10] {
            let mut empty = diagnostic.clone();
            empty.primary_span.as_mut().unwrap().start = offset;
            empty.primary_span.as_mut().unwrap().end = offset;
            let error = empty.validate("input.nx", "0123456789").unwrap_err();
            assert!(
                error.0.contains(
                    "§13.13 NX-E0003 requires primary_span; costly analysis spans are nonempty"
                ),
                "{kind}: {error}"
            );
        }
    }
}

#[test]
fn nx_e0003_structural_budgets_require_primary_spans() {
    use diagnostic::{DataValue as V, Operation, Stage};

    for (kind, stage, limit) in [
        ("syntax_nesting", Stage::Parse, 128),
        ("expression_tree_depth", Stage::Parse, 128),
        ("function_count", Stage::Parse, 1_024),
        ("array_type_depth", Stage::Types, 128),
        ("value_type_depth", Stage::Types, 128),
    ] {
        let mut diagnostic = diagnostic_for_catalog("NX-E0003", None);
        diagnostic.stage = stage;
        diagnostic.operation = Operation::AnalysisLimit;
        diagnostic
            .data
            .insert("budget_kind".into(), V::String(kind.into()));
        diagnostic.data.insert("limit".into(), V::UInt(limit));
        diagnostic
            .data
            .insert("observed".into(), V::UInt(limit + 1));
        refresh_budget_metadata(&mut diagnostic);
        assert!(
            diagnostic.validate("input.nx", "x").is_ok(),
            "{kind} positive"
        );
        diagnostic.primary_span = None;
        let error = diagnostic.validate("input.nx", "x").unwrap_err();
        assert!(
            error.0.contains("§13.13 NX-E0003 requires primary_span"),
            "{kind}: {error}"
        );
    }
}

#[test]
fn cause_kind_operation_table_is_total() {
    use diagnostic::{Cause, CauseKind as K, CauseOperation as O, Evidence};
    let kinds = [
        K::Call,
        K::Copy,
        K::Consume,
        K::Borrow,
        K::Reborrow,
        K::Drop,
        K::Return,
        K::Backedge,
        K::Effect,
        K::Limit,
    ];
    let operations = [
        O::FunctionCall,
        O::IntrinsicCall,
        O::BuildArray,
        O::ValueCopy,
        O::ValueDrop,
        O::ResourceDrop,
        O::ValueConsume,
        O::BorrowRead,
        O::BorrowInout,
        O::BorrowReborrow,
        O::FlowBackedge,
    ];
    let allowed = [
        (K::Call, O::FunctionCall),
        (K::Call, O::IntrinsicCall),
        (K::Copy, O::ValueCopy),
        (K::Drop, O::ValueDrop),
        (K::Drop, O::ResourceDrop),
        (K::Consume, O::ValueConsume),
        (K::Borrow, O::BorrowRead),
        (K::Borrow, O::BorrowInout),
        (K::Reborrow, O::BorrowReborrow),
        (K::Backedge, O::FlowBackedge),
        (K::Effect, O::BuildArray),
    ];
    for kind in kinds {
        for operation in operations {
            let mut d = diagnostic_for_catalog("NX-E0006", None);
            d.cause_chain.push(Cause {
                kind,
                operation,
                span: None,
                entity: None,
                evidence: Evidence::Verified,
            });
            assert_eq!(
                d.validate("input.nx", "x").is_ok(),
                allowed.contains(&(kind, operation)),
                "{kind:?}/{operation:?}"
            );
        }
    }
}

#[test]
fn nx_e1105_permission_tables_cover_every_pair() {
    use diagnostic::DataValue as V;
    let accesses = ["read", "write", "consume", "drop", "borrow_inout"];
    let permitted = [
        ("permission_escalation", "read", "write"),
        ("permission_escalation", "read", "consume"),
        ("permission_escalation", "read", "drop"),
        ("permission_escalation", "read", "borrow_inout"),
        ("permission_escalation", "inout", "consume"),
        ("permission_escalation", "inout", "drop"),
        ("active_borrow_conflict", "read", "write"),
        ("active_borrow_conflict", "read", "consume"),
        ("active_borrow_conflict", "read", "drop"),
        ("active_borrow_conflict", "read", "borrow_inout"),
        ("active_borrow_conflict", "inout", "read"),
        ("active_borrow_conflict", "inout", "write"),
        ("active_borrow_conflict", "inout", "consume"),
        ("active_borrow_conflict", "inout", "drop"),
        ("overlapping_mutable_borrows", "inout", "borrow_inout"),
    ];
    for subcause in [
        "permission_escalation",
        "active_borrow_conflict",
        "overlapping_mutable_borrows",
    ] {
        for active in ["read", "inout"] {
            for requested in accesses {
                let mut d = diagnostic_for_catalog("NX-E1105", Some(subcause));
                d.data
                    .insert("active_permission".into(), V::String(active.into()));
                d.data
                    .insert("requested_access".into(), V::String(requested.into()));
                let expected = permitted.contains(&(subcause, active, requested));
                assert_eq!(
                    d.validate("input.nx", "x").is_ok(),
                    expected,
                    "{subcause}/{active}/{requested}"
                );
            }
        }
    }
}

#[test]
fn canonical_type_parser_stops_at_the_published_depth_bound() {
    use diagnostic::DataValue as V;
    let mut d = diagnostic_for_catalog("NX-E0102", Some("type_mismatch"));
    let at_limit = format!("{}I64{}", "Array<".repeat(128), ">".repeat(128));
    d.data.insert("expected_type".into(), V::String(at_limit));
    assert!(d.validate("input.nx", "x").is_ok());
    let over_limit = format!("{}I64{}", "Array<".repeat(129), ">".repeat(129));
    d.data.insert("expected_type".into(), V::String(over_limit));
    assert!(d.validate("input.nx", "x").is_err());
    let deeply_nested = format!("{}I64{}", "Array<".repeat(10_000), ">".repeat(10_000));
    d.data
        .insert("expected_type".into(), V::String(deeply_nested));
    let error = d.validate("input.nx", "x").unwrap_err();
    assert!(
        error
            .0
            .contains("§13.17 data.expected_type is not a canonical type")
    );
}

#[test]
fn source_span_rejects_inverted_offsets_that_are_both_valid_boundaries() {
    let mut d = diagnostic_for_catalog("NX-E0004", Some("source_unreadable"));
    d.related_spans.push(diagnostic::RelatedSpan {
        role: diagnostic::RelatedRole::Origin,
        span: diagnostic::Span {
            file: "input.nx".into(),
            start: 2,
            end: 1,
        },
    });
    assert!(
        d.validate("input.nx", "xy")
            .unwrap_err()
            .0
            .contains("span is outside source")
    );
}

#[test]
fn truncated_utf8_invalid_range_ends_at_eof() {
    assert_eq!(
        diagnostic::decode_utf8(vec![b'a', 0xf0, 0x9f]),
        Err(diagnostic::InputError::InvalidUtf8 {
            start: 1,
            end: 3,
            bytes_hex: "f09f".into(),
            message: "incomplete utf-8 byte sequence from index 1".into(),
        })
    );
}

#[test]
fn invalid_utf8_span_covers_truncated_prefix_through_eof() {
    let mut d = diagnostic_for_catalog("NX-E0004", Some("invalid_utf8"));
    d.data.insert(
        "invalid_bytes_hex".into(),
        diagnostic::DataValue::String("f09f".into()),
    );
    d.primary_span.as_mut().unwrap().start = 1;
    d.primary_span.as_mut().unwrap().end = 3;
    assert!(
        d.validate_view(
            "input.nx",
            diagnostic::SourceView {
                bytes_len: 3,
                text: None
            }
        )
        .is_ok()
    );
}

#[test]
fn cause_text_keeps_a_null_entity_and_multiple_explanations_are_separated_exactly() {
    let mut d = diagnostic_for_catalog("NX-E0101", None);
    d.cause_chain.push(diagnostic::Cause {
        kind: diagnostic::CauseKind::Call,
        operation: diagnostic::CauseOperation::FunctionCall,
        span: None,
        entity: None,
        evidence: diagnostic::Evidence::Unknown,
    });
    let text = d.render_text("input.nx", "x").unwrap();
    assert!(text.contains("  cause: {\"entity\":null,"), "{text}");

    let entries = diagnostic::explain("NX-E0111", None).unwrap();
    let rendered = diagnostic::explanation_text(&entries[..2]);
    assert!(
        rendered.contains("repair_consequences: []\n\nNX-E0111/"),
        "{rendered}"
    );
    assert_eq!(rendered.matches("\n\n").count(), 1, "{rendered}");
}

#[test]
fn every_published_closed_vocabulary_matches_its_literal_set() {
    use diagnostic::*;
    fn values<T: Copy>(items: &[T], name: impl Fn(T) -> &'static str) -> Vec<&'static str> {
        let mut result: Vec<_> = items.iter().copied().map(name).collect();
        result.sort_unstable();
        result
    }
    fn expected(items: &[&'static str]) -> Vec<&'static str> {
        let mut result = items.to_vec();
        result.sort_unstable();
        result
    }
    assert_eq!(
        values(
            &[Namespace::NexoProposedV1, Namespace::F0002Sim],
            Namespace::as_str
        ),
        expected(&["Nexo-proposed-v1", "F0-0002-sim"])
    );
    assert_eq!(
        values(&[Severity::Error], Severity::as_str),
        expected(&["error"])
    );
    assert_eq!(
        values(
            &[
                Stage::Input,
                Stage::Lex,
                Stage::Parse,
                Stage::Names,
                Stage::Arguments,
                Stage::Types,
                Stage::Ownership,
                Stage::Effects,
                Stage::Lower,
                Stage::Entry,
                Stage::Runtime
            ],
            Stage::as_str
        ),
        expected(&[
            "input",
            "lex",
            "parse",
            "names",
            "arguments",
            "types",
            "ownership",
            "effects",
            "lower",
            "entry",
            "runtime"
        ])
    );
    assert_eq!(
        values(
            &[
                ExpectedTokensKind::AdmissibleSet,
                ExpectedTokensKind::CanonicalInsertion,
                ExpectedTokensKind::LexicalCategory
            ],
            ExpectedTokensKind::as_str
        ),
        expected(&["admissible_set", "canonical_insertion", "lexical_category"])
    );
    assert_eq!(
        values(
            &[
                EntityKind::Function,
                EntityKind::Parameter,
                EntityKind::Binding,
                EntityKind::Temporary,
                EntityKind::Loan,
                EntityKind::Intrinsic,
                EntityKind::Type
            ],
            EntityKind::as_str
        ),
        expected(&[
            "function",
            "parameter",
            "binding",
            "temporary",
            "loan",
            "intrinsic",
            "type"
        ])
    );
    assert_eq!(
        values(
            &[
                Evidence::Verified,
                Evidence::ExternalAssumption,
                Evidence::Unknown
            ],
            Evidence::as_str
        ),
        expected(&["verified", "external_assumption", "unknown"])
    );
    assert_eq!(
        values(
            &[
                TokenKind::Name,
                TokenKind::Keyword,
                TokenKind::Symbol,
                TokenKind::Integer,
                TokenKind::Decimal,
                TokenKind::String,
                TokenKind::Eof,
                TokenKind::Invalid
            ],
            TokenKind::as_str
        ),
        expected(&[
            "name", "keyword", "symbol", "integer", "decimal", "string", "eof", "invalid"
        ])
    );
    assert_eq!(
        values(
            &[RepairStatus::Proposed, RepairStatus::Applicable],
            RepairStatus::as_str
        ),
        expected(&["proposed", "applicable"])
    );
    assert_eq!(
        values(
            &[RepairMode::Check, RepairMode::RunEntry],
            RepairMode::as_str
        ),
        expected(&["check", "run_entry"])
    );
    assert_eq!(
        values(
            &[RepairResult::Accepted, RepairResult::Rejected],
            RepairResult::as_str
        ),
        expected(&["accepted", "rejected"])
    );
    assert_eq!(
        values(
            &[
                RelatedRole::Declaration,
                RelatedRole::PreviousDeclaration,
                RelatedRole::Contract,
                RelatedRole::Origin,
                RelatedRole::Consumption,
                RelatedRole::Backedge,
                RelatedRole::Exit,
                RelatedRole::ExpectedType,
                RelatedRole::FoundType,
                RelatedRole::Context,
                RelatedRole::Delimiter,
                RelatedRole::PrecedingExpression
            ],
            RelatedRole::as_str
        ),
        expected(&[
            "declaration",
            "previous_declaration",
            "contract",
            "origin",
            "consumption",
            "backedge",
            "exit",
            "expected_type",
            "found_type",
            "context",
            "delimiter",
            "preceding_expression"
        ])
    );
    assert_eq!(
        values(
            &[
                CauseKind::Call,
                CauseKind::Copy,
                CauseKind::Consume,
                CauseKind::Borrow,
                CauseKind::Reborrow,
                CauseKind::Drop,
                CauseKind::Return,
                CauseKind::Backedge,
                CauseKind::Effect,
                CauseKind::Limit
            ],
            CauseKind::as_str
        ),
        expected(&[
            "call", "copy", "consume", "borrow", "reborrow", "drop", "return", "backedge",
            "effect", "limit"
        ])
    );
    assert_eq!(
        values(
            &[
                CauseOperation::FunctionCall,
                CauseOperation::IntrinsicCall,
                CauseOperation::BuildArray,
                CauseOperation::ValueCopy,
                CauseOperation::ValueDrop,
                CauseOperation::ResourceDrop,
                CauseOperation::ValueConsume,
                CauseOperation::BorrowRead,
                CauseOperation::BorrowInout,
                CauseOperation::BorrowReborrow,
                CauseOperation::FlowBackedge
            ],
            CauseOperation::as_str
        ),
        expected(&[
            "function.call",
            "intrinsic.call",
            "value.build_array",
            "value.copy",
            "value.drop",
            "resource.drop",
            "value.consume",
            "borrow.read",
            "borrow.inout",
            "borrow.reborrow",
            "flow.backedge"
        ])
    );
    assert_eq!(
        values(
            &[
                Operation::SyntaxLex,
                Operation::SyntaxParse,
                Operation::AnalysisLimit,
                Operation::InputRead,
                Operation::InputDecode,
                Operation::NamesResolve,
                Operation::NamesDeclare,
                Operation::TypesCheck,
                Operation::TypesInfer,
                Operation::PlaceWrite,
                Operation::ValueLiteral,
                Operation::CallBind,
                Operation::EntryCheck,
                Operation::ValueCopy,
                Operation::OwnershipUse,
                Operation::OwnershipAccess,
                Operation::Branch,
                Operation::Return,
                Operation::RuntimeLimit,
                Operation::ValueAdd,
                Operation::ValueSub,
                Operation::ValueIndex,
                Operation::AssertEq,
                Operation::Fail,
                Operation::FunctionCall,
                Operation::IntrinsicCall,
                Operation::BuildArray,
                Operation::ValueDrop
            ],
            Operation::as_str
        ),
        expected(&[
            "syntax.lex",
            "syntax.parse",
            "analysis.limit",
            "input.read",
            "input.decode",
            "names.resolve",
            "names.declare",
            "types.check",
            "types.infer",
            "place.write",
            "value.literal",
            "call.bind",
            "entry.check",
            "value.copy",
            "ownership.use",
            "ownership.access",
            "flow.branch",
            "flow.return",
            "runtime.limit",
            "value.add",
            "value.sub",
            "value.index",
            "intrinsic.assert_eq",
            "intrinsic.fail",
            "function.call",
            "intrinsic.call",
            "value.build_array",
            "value.drop"
        ])
    );
}

#[test]
fn terminal_controls_overlapping_edits_and_duplicate_repair_ids_are_rejected() {
    let mut message = diagnostic_for_catalog("NX-E0001", None);
    message.message = "bad\u{1b}[31m".into();
    assert!(
        message
            .validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("terminal controls")
    );

    let mut overlap = diagnostic_for_catalog("NX-E0005", None);
    overlap.repair.push(diagnostic::Repair {
        id: "insert_terminator".into(),
        status: diagnostic::RepairStatus::Proposed,
        description: "two edits".into(),
        edits: vec![
            diagnostic::Edit {
                span: diagnostic::Span {
                    file: "input.nx".into(),
                    start: 0,
                    end: 1,
                },
                expected_text: "x".into(),
                replacement: "a".into(),
            },
            diagnostic::Edit {
                span: diagnostic::Span {
                    file: "input.nx".into(),
                    start: 0,
                    end: 1,
                },
                expected_text: "x".into(),
                replacement: "b".into(),
            },
        ],
        validation: None,
        consequences: vec![],
    });
    assert!(
        overlap
            .validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("ordered and non-overlapping")
    );

    let mut duplicate = diagnostic_for_catalog("NX-E0005", None);
    let repair = diagnostic::Repair {
        id: "insert_terminator".into(),
        status: diagnostic::RepairStatus::Proposed,
        description: "one".into(),
        edits: vec![],
        validation: None,
        consequences: vec![],
    };
    duplicate.repair = vec![repair.clone(), repair];
    assert!(
        duplicate
            .validate("input.nx", "x")
            .unwrap_err()
            .0
            .contains("ordered by unique id")
    );
}
