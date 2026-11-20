use std::collections::BTreeMap;

use nexo::diagnostic::{DataValue, Diagnostic, Namespace, Operation, RelatedRole, Severity, Stage};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

const FILE: &str = "solver_depth.n";

fn array(inner: &str, depth: usize) -> String {
    format!("{}{}{}", "[".repeat(depth), inner, "]".repeat(depth))
}

fn array_type(inner: &str, depth: usize) -> String {
    format!("{}{}{}", "Array<".repeat(depth), inner, ">".repeat(depth))
}

#[allow(clippy::too_many_arguments)] // Each field is an independent normative diagnostic assertion.
fn exact(
    source: &str,
    code: &str,
    subcause: Option<&str>,
    operation: Operation,
    primary: (u64, u64),
    token: &str,
    related: &[(RelatedRole, u64, u64)],
    data: BTreeMap<String, DataValue>,
) {
    for run_entry in [false, true] {
        let result = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, run_entry);
        let CompileError::Rejected(errors) = result.unwrap_err() else {
            panic!("expected source rejection: {source}");
        };
        let errors: Vec<Diagnostic> = errors
            .into_iter()
            .map(|mut error| {
                error.publish(FILE, source).unwrap();
                *error
            })
            .collect();
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, code);
        assert_eq!(error.subcause.as_deref(), subcause);
        let rule = if let Some(subcause) = subcause {
            format!("{code}/{subcause}")
        } else {
            code.to_owned()
        };
        assert_eq!(error.rule, rule);
        assert_eq!(error.explanation_id.as_deref(), Some(rule.as_str()));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, operation);
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), primary);
        assert_eq!(&source[span.start as usize..span.end as usize], token);
        assert_eq!(
            error
                .related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            related,
        );
        assert_eq!(error.data, data);
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
        assert!(error.feature_id.is_none());
        assert!(error.unexpected_token.is_none());
        assert!(error.expected_tokens.is_empty());
        assert!(error.expected_tokens_kind.is_none());
        assert!(error.parser_context.is_none());
        assert!(error.recognized_prefix.is_none());
    }
}

fn data(pairs: &[(&str, DataValue)]) -> BTreeMap<String, DataValue> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn string(value: &str) -> DataValue {
    DataValue::String(value.to_owned())
}

fn accepts(source: &str) {
    for run_entry in [false, true] {
        assert!(
            compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, run_entry).is_ok(),
            "{source}"
        );
    }
}

struct ExpectedDiagnostic<'a> {
    code: &'a str,
    subcause: Option<&'a str>,
    operation: Operation,
    primary: (u64, u64),
    token: &'a str,
    related: &'a [(RelatedRole, u64, u64)],
    data: BTreeMap<String, DataValue>,
}

// The rows below are literal check results from ccf6df1 or validated Block I.
fn exact_check_diagnostics(source: &str, expected: &[ExpectedDiagnostic<'_>]) {
    let profile = ReducedProfile::from_id("f0-0003-b-v1").unwrap();
    let result = compile_scalar_with_profile(FILE, source, profile, false);
    let CompileError::Rejected(errors) = result.unwrap_err() else {
        panic!("expected source rejection: {source}");
    };
    let errors: Vec<Diagnostic> = errors
        .into_iter()
        .map(|mut error| {
            error.publish(FILE, source).unwrap();
            *error
        })
        .collect();
    assert_eq!(errors.len(), expected.len(), "{source}: {errors:?}");
    for (error, wanted) in errors.iter().zip(expected) {
        assert_eq!(error.schema_version, 1);
        assert_eq!(error.diagnostic_namespace, Namespace::NexoProposedV1);
        assert_eq!(error.profile_id, "f0-0003-b-v1");
        assert_eq!(error.code, wanted.code);
        assert_eq!(error.subcause.as_deref(), wanted.subcause);
        let rule = wanted.subcause.map_or_else(
            || wanted.code.to_owned(),
            |subcause| format!("{}/{subcause}", wanted.code),
        );
        assert_eq!(error.rule, rule);
        assert_eq!(error.explanation_id.as_deref(), Some(rule.as_str()));
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(
            error.message,
            "Invalid name, call or type in the proposed profile."
        );
        assert_eq!(error.operation, wanted.operation);
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!(span.file, FILE);
        assert_eq!((span.start, span.end), wanted.primary);
        assert_eq!(
            &source[span.start as usize..span.end as usize],
            wanted.token
        );
        assert_eq!(
            error
                .related_spans
                .iter()
                .map(|r| {
                    assert_eq!(r.span.file, FILE);
                    (r.role, r.span.start, r.span.end)
                })
                .collect::<Vec<_>>(),
            wanted.related,
        );
        assert_eq!(error.data, wanted.data);
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
        assert!(error.feature_id.is_none());
        assert!(error.unexpected_token.is_none());
        assert!(error.expected_tokens.is_empty());
        assert!(error.expected_tokens_kind.is_none());
        assert!(error.parser_context.is_none());
        assert!(error.recognized_prefix.is_none());
    }
}

#[test]
fn deep_array_annotation_keeps_the_whole_type_and_written_origin() {
    let negative = format!("fn Main(){{let x:Bool={};}}", array("1", 126));
    exact(
        &negative,
        "NX-E0102",
        Some("type_mismatch"),
        Operation::TypesCheck,
        (21, 274),
        &array("1", 126),
        &[
            (RelatedRole::ExpectedType, 16, 20),
            (RelatedRole::FoundType, 21, 274),
        ],
        data(&[
            ("expected_type", string("Bool")),
            ("found_type", string(&array_type("I64", 126))),
        ]),
    );
    accepts(&format!(
        "fn Main(){{let x:{}={};}}",
        array_type("I64", 126),
        array("1", 126)
    ));
}

#[test]
fn absent_option_component_reports_the_constructor() {
    exact(
        "fn Main(){let o=None;}",
        "NX-E0113",
        None,
        Operation::TypesInfer,
        (16, 20),
        "None",
        &[],
        data(&[(
            "unresolved_components",
            DataValue::Array(vec![string("$.some")]),
        )]),
    );
    accepts("fn Main(){let o:Option<I64>=None;}");
}

#[test]
fn deep_array_equality_reports_the_incompatible_leaf() {
    let negative = format!(
        "fn Main(){{var r={};r={};}}",
        array("true", 126),
        array("1", 126)
    );
    exact(
        &negative,
        "NX-E0102",
        Some("type_mismatch"),
        Operation::TypesCheck,
        (401, 402),
        "1",
        &[
            (RelatedRole::ExpectedType, 16, 272),
            (RelatedRole::FoundType, 401, 402),
        ],
        data(&[
            ("expected_type", string("Bool")),
            ("found_type", string("I64")),
        ]),
    );
    accepts(&format!(
        "fn Main(){{var r={};r={};}}",
        array("true", 126),
        array("true", 126)
    ));
}

#[test]
fn recursive_component_has_no_finite_type() {
    exact(
        "fn Main(){var r=[];r=[r];}",
        "NX-E0103",
        None,
        Operation::TypesInfer,
        (16, 18),
        "[]",
        &[],
        BTreeMap::new(),
    );
    accepts("fn Main(){var r=[];r=[1];}");
}

#[test]
fn partial_constructor_shape_conflicts_at_the_later_alias() {
    let divergent = "(if true then {return;} else {return;})";
    let negative = format!(
        "fn Main(){{let l0={divergent};match l0{{Some(x)=>{{}},_=>{{}}}}let b:Result<I64,I64>=l0;}}"
    );
    exact(
        &negative,
        "NX-E0102",
        Some("constructor_shape"),
        Operation::TypesCheck,
        (106, 108),
        "l0",
        &[
            (RelatedRole::Origin, 66, 73),
            (RelatedRole::ExpectedType, 90, 105),
            (RelatedRole::FoundType, 106, 108),
        ],
        data(&[
            ("expected_type", string("Result<I64,I64>")),
            ("found_constructor", string("Option")),
        ]),
    );
    accepts(&negative.replace("Result<I64,I64>", "Option<I64>"));
}

#[test]
fn nested_constructor_shape_conflicts_at_the_second_arm() {
    let divergent = "(if true then {return;} else {return;})";
    let negative = format!(
        "fn Main(){{let l0={divergent};match l0{{Some(Some(x))=>{{}},Some(Ok(y))=>{{}},_=>{{}}}}}}"
    );
    exact(
        &negative,
        "NX-E1325",
        Some("constructor_shape"),
        Operation::TypesCheck,
        (89, 94),
        "Ok(y)",
        &[],
        data(&[
            ("constructor", string("prelude:Ok")),
            ("scrutinee_constructor", string("Option")),
        ]),
    );
    accepts(&negative.replace("Some(Ok(y))=>{},", ""));
}

#[test]
fn generator_value_type_depth_uses_the_last_some_expression() {
    let mut positive = String::from("gen fn G()->I64{yield 1;}fn Main(){let a0=G();");
    for index in 1..=127 {
        positive.push_str(&format!("let a{index}=Some(move a{});", index - 1));
    }
    let last = "Some(move a127)";
    let negative = format!("{positive}let a128={last};}}");
    let start = negative.rfind(last).unwrap() as u64;
    exact(
        &negative,
        "NX-E0003",
        None,
        Operation::AnalysisLimit,
        (start, start + last.len() as u64),
        last,
        &[],
        data(&[
            ("budget_kind", string("value_type_depth")),
            ("budget_scope", string("compilation")),
            ("incomplete_analysis", string("types")),
            ("limit", DataValue::UInt(128)),
            ("observed", DataValue::UInt(129)),
        ]),
    );
    positive.push('}');
    accepts(&positive);
}

#[test]
fn solver_budget_rejects_first_step_over_thirty_two() {
    let source = format!(
        "fn Main(){{{}}}",
        (0..64)
            .map(|i| format!("let x{i}:I64=0;"))
            .collect::<String>()
    );
    for run_entry in [false, true] {
        let result = compile_scalar_with_profile(FILE, &source, ReducedProfile::Types32, run_entry);
        let CompileError::Rejected(errors) = result.unwrap_err() else {
            panic!("expected type solver quota");
        };
        assert_eq!(errors.len(), 1);
        let error = &errors[0];
        error.validate(FILE, &source).unwrap();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(error.subcause, None);
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert_eq!(
            error.data,
            data(&[
                ("budget_kind", string("type_solver_steps")),
                ("budget_scope", string("compilation")),
                ("incomplete_analysis", string("types")),
                ("limit", DataValue::UInt(32)),
                ("observed", DataValue::UInt(33)),
            ])
        );
        let span = error.primary_span.as_ref().unwrap();
        assert!(span.start < span.end && span.end as usize <= source.len());
        assert!(source.is_char_boundary(span.start as usize));
        assert!(source.is_char_boundary(span.end as usize));
    }
    accepts("fn Main(){}");
}

fn syntax_limit(source: &str) {
    for run_entry in [false, true] {
        let result = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, run_entry);
        let CompileError::Rejected(errors) = result.unwrap_err() else {
            panic!("expected syntax depth rejection");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(error.stage, Stage::Parse);
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&string("syntax_nesting"))
        );
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
        let span = error.primary_span.as_ref().unwrap();
        assert!(span.start < span.end && span.end as usize <= source.len());
    }
}

#[test]
fn expression_and_signature_depth_boundaries_keep_parse_and_types_separate() {
    for depth in 126..=129 {
        let array_source = format!("fn Main(){{let x={};}}", array("1", depth));
        let some_source = format!(
            "fn Main(){{let x={}1{};}}",
            "Some(".repeat(depth),
            ")".repeat(depth)
        );
        for source in [&array_source, &some_source] {
            if depth == 126 {
                accepts(source);
            } else {
                syntax_limit(source);
            }
        }
        let signature = format!("fn F(x:view {}){{}}fn Main(){{}}", array_type("I64", depth));
        if depth <= 127 {
            accepts(&signature);
        } else {
            syntax_limit(&signature);
        }
    }
}

#[test]
fn nested_some_pattern_depth_boundary_is_a_parse_quota() {
    for depth in 126..=129 {
        let signature = format!("{}I64{}", "Option<".repeat(depth), ">".repeat(depth));
        let source = format!(
            "fn F(x:view {}){{match x{{{}_{}=>{{}},_=>{{}}}}}}fn Main(){{}}",
            signature,
            "Some(".repeat(depth),
            ")".repeat(depth),
        );
        if depth == 126 {
            accepts(&source);
        } else {
            syntax_limit(&source);
        }
    }
}

#[test]
fn deep_union_signatures_and_late_projection_export_remain_valid() {
    for constructor in ["Option<", "Result<"] {
        let signature = if constructor == "Option<" {
            format!("{}I64{}", constructor.repeat(126), ">".repeat(126))
        } else {
            format!("{}I64{}", constructor.repeat(126), ",I64>".repeat(126))
        };
        accepts(&format!(
            "fn F(x:view {signature}){{match x{{_=>{{}}}}}}fn Main(){{}}"
        ));
    }
    accepts("fn Main(){var a=[];a=[1];let b=a;let x=b[0];}");
}

#[test]
fn pending_projection_clones_all_selector_statement_shapes() {
    let selectors = [
        "if true then {var i=0;i=1;value:i;}else{value:2;}",
        "if true then {if false {let z=1;}else{let z=2;}value:0;}else{value:1;}",
        "if true then {var i=0;while i<1{i=i+1;}value:i;}else{value:1;}",
        "if true then {for i in G(){let z=i;}value:0;}else{value:1;}",
        "match Some(Some(1)){Some(Some(n)) if n==1=>{value:0;},_=>{value:1;}}",
        "if true then {let f=fn []()->I64{return 1;};value:0;}else{value:1;}",
        "if true then {match true{true if true=>{},_=>{}}value:0;}else{value:1;}",
    ];
    for selector in selectors {
        let source = format!(
            "gen fn G()->I64{{yield 1;}}fn Main(){{var q=(if true then {{return;}} else {{return;}});let x=q[{selector}];q=[1];}}"
        );
        accepts(&source);
    }
}

#[test]
fn closure_inferred_returns_keep_type_mismatch_diagnostic() {
    // A2 §4.1.1: successive returns constrain the same inferred result.
    let source = "fn Main(){let f=fn [](){return 1;return true;};}";
    exact(
        source,
        "NX-E0102",
        Some("type_mismatch"),
        Operation::TypesCheck,
        (40, 44),
        "true",
        &[
            (RelatedRole::ExpectedType, 31, 32),
            (RelatedRole::FoundType, 40, 44),
        ],
        data(&[
            ("expected_type", string("I64")),
            ("found_type", string("Bool")),
        ]),
    );
}

#[test]
fn partial_constructor_shape_uses_canonical_message() {
    // F0-0003 §8.2 fixes this message for a partial constructor conflict.
    let source = "fn Main(){let x=(if true then {return;} else {return;});match x{Some(Some(y))=>{},Some(Ok(z))=>{},_=>{}}}";
    for run_entry in [false, true] {
        let result = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, run_entry);
        let CompileError::Rejected(errors) = result.unwrap_err() else {
            panic!("expected constructor shape rejection")
        };
        assert_eq!(errors.len(), 1);
        let mut error = errors.into_iter().next().unwrap();
        error.publish(FILE, source).unwrap();
        assert_eq!(error.code, "NX-E1325");
        assert_eq!(error.subcause.as_deref(), Some("constructor_shape"));
        assert_eq!(error.rule, "NX-E1325/constructor_shape");
        assert_eq!(
            error.explanation_id.as_deref(),
            Some("NX-E1325/constructor_shape")
        );
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((87, 92))
        );
        assert_eq!(&source[87..92], "Ok(z)");
        assert_eq!(
            error.message,
            "Constructor incompatible con la forma seleccionada."
        );
        assert_eq!(
            error.data,
            data(&[
                ("constructor", string("prelude:Ok")),
                ("scrutinee_constructor", string("Option")),
            ])
        );
        assert!(error.related_spans.is_empty());
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
    }
    accepts(
        "fn Main(){let x=(if true then {return;} else {return;});match x{Some(Some(y))=>{},_=>{}}}",
    );
}

#[test]
fn unresolved_array_match_does_not_loop_on_option_pattern() {
    // F0-0002 §4.1.2.2: the empty array still owns its NX-E0103 when a
    // later Option pattern cannot determine its element.
    exact(
        "fn Main(){var q=[];match q{Some(x)=>{},_=>{}}}",
        "NX-E0103",
        None,
        Operation::TypesInfer,
        (16, 18),
        "[]",
        &[],
        BTreeMap::new(),
    );
    accepts("fn Main(){let q:Option<I64>=Some(1);match q{Some(x)=>{},_=>{}}}");
}

#[test]
fn constructor_arity_errors_do_not_repeat_pattern_walk() {
    // F0-0003 §8.2: both a missing and an extra payload reject at the
    // constructor, before a nested pattern walk can loop.
    exact(
        "fn Main(){let x:Option<I64>=None;match x{Some()=>{},_=>{}}}",
        "NX-E1325",
        Some("constructor_arity"),
        Operation::TypesCheck,
        (41, 47),
        "Some()",
        &[],
        data(&[
            ("constructor", string("prelude:Some")),
            ("expected_arity", DataValue::UInt(1)),
            ("found_arity", DataValue::UInt(0)),
        ]),
    );
    exact(
        "fn Main(){let x:Option<I64>=None;match x{None(y)=>{},_=>{}}}",
        "NX-E1325",
        Some("constructor_arity"),
        Operation::TypesCheck,
        (41, 48),
        "None(y)",
        &[],
        data(&[
            ("constructor", string("prelude:None")),
            ("expected_arity", DataValue::UInt(0)),
            ("found_arity", DataValue::UInt(1)),
        ]),
    );
    accepts("fn Main(){let x:Option<I64>=Some(1);match x{Some(y)=>{},_=>{}}}");
    accepts("fn Main(){let x:Option<I64>=None;match x{None=>{},_=>{}}}");
}

// I07c survivor regressions, transcribed from the frozen reference check output.
fn check_m00436() {
    let source = "fn Main(){let p=true;var r=Some(1);var q=r;r=(if p then 1 else (if true then 1 else (Some(r)==q)));}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (56, 57),
                token: "1",
                related: &[
                    (RelatedRole::ExpectedType, 27, 34),
                    (RelatedRole::FoundType, 56, 57),
                ],
                data: data(&[
                    ("expected_type", string("Option<I64>")),
                    ("found_type", string("I64")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (77, 78),
                token: "1",
                related: &[
                    (RelatedRole::ExpectedType, 27, 34),
                    (RelatedRole::FoundType, 77, 78),
                ],
                data: data(&[
                    ("expected_type", string("Option<I64>")),
                    ("found_type", string("I64")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (94, 95),
                token: "q",
                related: &[(RelatedRole::FoundType, 94, 95)],
                data: data(&[
                    ("expected_type", string("Option<I64>")),
                    ("found_type", string("I64")),
                ]),
            },
        ],
    );
}
#[test]
fn survivor_m00436_check_diagnostics() {
    check_m00436();
}
fn check_m00466() {
    let source = "fn Main(){var b0=(if true then {return;} else {return;});b0[Some((if true then {return;} else {return;}))]=(1==(if [[1]] then ([[]]+[1]) else b0));match ((if Some([[1]]) then None else [b0,(if true then {return;} else {return;})])<(if true then {return;} else {return;})){Some(v)=>{let k=v;},_=>{}}match b0{Some(v)=>{let k=v;},_=>{}}}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0113",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (60, 105),
                token: "Some((if true then {return;} else {return;}))",
                related: &[],
                data: data(&[(
                    "unresolved_components",
                    DataValue::Array(vec![string("$.some")]),
                )]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (115, 120),
                token: "[[1]]",
                related: &[(RelatedRole::FoundType, 115, 120)],
                data: data(&[
                    ("expected_type", string("Bool")),
                    ("found_type", string("Array<Array<I64>>")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (128, 130),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (158, 169),
                token: "Some([[1]])",
                related: &[(RelatedRole::FoundType, 158, 169)],
                data: data(&[
                    ("expected_type", string("Bool")),
                    ("found_type", string("Option<Array<Array<I64>>>")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0113",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (175, 179),
                token: "None",
                related: &[],
                data: data(&[(
                    "unresolved_components",
                    DataValue::Array(vec![string("$.some")]),
                )]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (185, 229),
                token: "[b0,(if true then {return;} else {return;})]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E1325",
                subcause: Some("constructor_type"),
                operation: Operation::TypesCheck,
                primary: (272, 279),
                token: "Some(v)",
                related: &[],
                data: data(&[
                    ("constructor", string("prelude:Some")),
                    ("scrutinee_type", string("Bool")),
                ]),
            },
        ],
    );
}
#[test]
fn survivor_m00466_check_diagnostics() {
    check_m00466();
}
fn check_m00474_m00475() {
    let source = "fn Main(){match [[]]{Some(x)=>{let k:Array<I64>=x;},_=>{}}let l1=(([[]]<[[]])-(if true then {return;} else {return;}));l1[l1]=l1;var b3=[l1[Some([])]];var q4=(if true then {return;} else {return;});}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (17, 19),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (67, 76),
                token: "[[]]<[[]]",
                related: &[(RelatedRole::FoundType, 67, 76)],
                data: data(&[
                    ("expected_type", string("I64")),
                    ("found_type", string("Bool")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (68, 70),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (73, 75),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (145, 147),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
        ],
    );
}
#[test]
fn survivor_m00474_check_diagnostics() {
    check_m00474_m00475();
}
#[test]
fn survivor_m00475_check_diagnostics() {
    check_m00474_m00475();
}
fn check_m00494() {
    let source = "fn Main(){var b0=([1]==(([[]]==[[]])==Some([1])));b0=b0[[[true,(if true then {return;} else {return;})],(if true then {return;} else {return;})]];}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (26, 28),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (32, 34),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
        ],
    );
}
#[test]
fn survivor_m00494_check_diagnostics() {
    check_m00494();
}
fn check_m00588() {
    let source = "fn Main(){match (if (if [None,[[1]]] then (if Some([]) then [[1]] else [(if true then {return;} else {return;})]) else Len(None)) then (if true then {return;} else {return;}) else [Some([[1]]),(if true then {return;} else {return;})]){Some(x)=>{},_=>{}}match (if true then {return;} else {return;}){Some(v)=>{let k:Array<I64>=v;},_=>{}}}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0113",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (25, 29),
                token: "None",
                related: &[],
                data: data(&[(
                    "unresolved_components",
                    DataValue::Array(vec![string("$.some")]),
                )]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (51, 53),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (60, 65),
                token: "[[1]]",
                related: &[(RelatedRole::FoundType, 60, 65)],
                data: data(&[
                    ("expected_type", string("Bool")),
                    ("found_type", string("Array<Array<I64>>")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (71, 112),
                token: "[(if true then {return;} else {return;})]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (119, 128),
                token: "Len(None)",
                related: &[(RelatedRole::FoundType, 119, 128)],
                data: data(&[
                    ("expected_type", string("Bool")),
                    ("found_type", string("I64")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0113",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (123, 127),
                token: "None",
                related: &[],
                data: data(&[(
                    "unresolved_components",
                    DataValue::Array(vec![string("$.some")]),
                )]),
            },
            ExpectedDiagnostic {
                code: "NX-E1325",
                subcause: Some("constructor_type"),
                operation: Operation::TypesCheck,
                primary: (235, 242),
                token: "Some(x)",
                related: &[],
                data: data(&[
                    ("constructor", string("prelude:Some")),
                    ("scrutinee_type", string("Array<Option<Array<Array<I64>>>>")),
                ]),
            },
        ],
    );
}
#[test]
fn survivor_m00588_check_diagnostics() {
    check_m00588();
}
fn check_m00655() {
    let source = "fn Main(){match (if (if true then {return;} else {return;}) then (if true then {return;} else {return;}) else ((if true then {return;} else {return;})<Len(Some([1])))){Some(v)=>{let k=v;},_=>{}}var a1=[[1]];a1[Len([a1,Some([])])]=a1[Some((if [] then a1 else [[1]]))];var a3=Some([]);}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("not_indexable"),
                operation: Operation::TypesCheck,
                primary: (155, 164),
                token: "Some([1])",
                related: &[(RelatedRole::FoundType, 155, 164)],
                data: data(&[
                    ("expected_type", string("domain:array")),
                    ("found_type", string("Option<Array<I64>>")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (223, 225),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (242, 244),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (279, 281),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
        ],
    );
}
#[test]
fn survivor_m00655_check_diagnostics() {
    check_m00655();
}
fn check_m00701_m00702_m00703() {
    let source = "fn Main(){var b0=Some([(if true then {return;} else {return;})]);match (if true then {return;} else {return;}){Some(x)=>{},_=>{}}let l2:Array<I64>=((if [1,[]] then Len(Some([])) else [b0])+(if [] then ([true]-b0) else Some([1])));b0[l2]=(if true then {return;} else {return;});}";
    exact_check_diagnostics(
        source,
        &[
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (22, 63),
                token: "[(if true then {return;} else {return;})]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (155, 157),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (173, 175),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (183, 187),
                token: "[b0]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0103",
                subcause: None,
                operation: Operation::TypesInfer,
                primary: (193, 195),
                token: "[]",
                related: &[],
                data: data(&[]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (202, 208),
                token: "[true]",
                related: &[(RelatedRole::FoundType, 202, 208)],
                data: data(&[
                    ("expected_type", string("I64")),
                    ("found_type", string("Array<Bool>")),
                ]),
            },
            ExpectedDiagnostic {
                code: "NX-E0102",
                subcause: Some("type_mismatch"),
                operation: Operation::TypesCheck,
                primary: (218, 227),
                token: "Some([1])",
                related: &[(RelatedRole::FoundType, 218, 227)],
                data: data(&[
                    ("expected_type", string("I64")),
                    ("found_type", string("Option<Array<I64>>")),
                ]),
            },
        ],
    );
}
#[test]
fn survivor_m00701_check_diagnostics() {
    check_m00701_m00702_m00703();
}
#[test]
fn survivor_m00702_check_diagnostics() {
    check_m00701_m00702_m00703();
}
#[test]
fn survivor_m00703_check_diagnostics() {
    check_m00701_m00702_m00703();
}
