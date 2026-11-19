use std::collections::BTreeMap;

use nexo::diagnostic::{DataValue, Diagnostic, Operation, RelatedRole, Stage};
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
