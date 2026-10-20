use std::{fs, path::Path};
fn run(s: &str) -> nexo::runtime::Execution {
    nexo::runtime::execute(&nexo::compile(s).expect("must statically check"))
}
#[test]
fn positive_corpus() {
    for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/pass")).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let p = nexo::compile(&source)
            .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.render(&source)));
        let result = nexo::runtime::execute(&p);
        assert!(result.error.is_none(), "{}: {:?}", path.display(), result);
    }
}
#[test]
fn negative_corpus_exact_codes() {
    for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/fail")).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let code = source
            .lines()
            .next()
            .unwrap()
            .strip_prefix("// expect: ")
            .unwrap();
        let Err(error) = nexo::compile(&source) else {
            panic!("accepted {}", path.display())
        };
        assert_eq!(
            error.code,
            code,
            "{}: {}",
            path.display(),
            error.render(&source)
        );
    }
}
#[test]
fn nested_copy_output() {
    let s = include_str!("../corpus/pass/nested_copy.nx");
    assert_eq!(run(s).output, ["[[1, 9], [7, 8]]"]);
}
#[test]
fn transfer_has_one_owner_and_exact_cleanup() {
    let r = run(include_str!("../corpus/pass/resource_transfer.nx"));
    assert_eq!(
        r.events,
        [
            "open:0:10",
            "inspect:0:10",
            "open:1:20",
            "inspect:1:20",
            "close:1:20",
            "close:0:10"
        ]
    );
}
#[test]
fn early_return_unwinds_scopes_in_reverse_order() {
    let r = run(include_str!("../corpus/pass/return_cleanup.nx"));
    assert_eq!(r.events, ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]);
    assert!(r.output.is_empty());
}
#[test]
fn recoverable_failure_and_partial_initialization() {
    let r = run(
        "fn main(){ let a=mock_open(1); {let b=mock_open(2); let c=mock_open(9223372036854775807+1);}}",
    );
    assert_eq!(r.error.unwrap().code, "R0101");
    assert_eq!(r.events, ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]);
}
#[test]
fn explicit_close_and_failure_do_not_double_close() {
    let r = run("fn main(){let a=mock_open(1);let b=mock_open(2);drop(a);fail();}");
    assert_eq!(r.error.unwrap().code, "R0301");
    assert_eq!(r.events, ["open:0:1", "open:1:2", "close:0:1", "close:1:2"]);
}
#[test]
fn indexing_and_overflow_are_checked_in_every_build() {
    for (expr, code) in [
        ("[1][-1]", "R0102"),
        ("[1][1]", "R0102"),
        ("9223372036854775807+1", "R0101"),
        ("-9223372036854775808+-1", "R0101"),
    ] {
        let r = run(&format!("fn main(){{let r=mock_open(1);print({expr});}}"));
        assert_eq!(r.error.unwrap().code, code);
        assert_eq!(r.events, ["open:0:1", "close:0:1"]);
    }
}
#[test]
fn assignment_bounds_preserve_cleanup() {
    let r = run("fn main(){let r=mock_open(1);var a=[[1]];a[0][1]=2;}");
    assert_eq!(r.error.unwrap().code, "R0102");
    assert_eq!(r.events, ["open:0:1", "close:0:1"]);
}
#[test]
fn statements_after_return_still_checked() {
    let Err(e) = nexo::compile("fn main(){return;let x=unknown;}") else {
        panic!("unchecked tail")
    };
    assert_eq!(e.code, "E0101");
}
#[test]
fn malformed_inputs_return_diagnostics() {
    for s in [
        "",
        "fn",
        "fn main(",
        "fn main(){",
        "fn main(){let a=",
        "fn main(){let a=-",
        "fn main(){let a=[1,",
        "fn main(){let a=1;} garbage",
        "fn main(){ print(1.5); }",
    ] {
        assert!(nexo::compile(s).is_err(), "{s:?}");
    }
}
#[test]
fn diagnostic_unicode_comment_location() {
    let src = "// á🙂\nfn main(){\n let a = missing;\n}";
    let Err(e) = nexo::compile(src) else { panic!() };
    assert!(
        e.render(src).starts_with("E0101:3:10:"),
        "{}",
        e.render(src)
    );
}
#[test]
fn nesting_limit_is_diagnostic() {
    let src = format!(
        "fn main(){{let x={}1{};}}",
        "[".repeat(200),
        "]".repeat(200)
    );
    let Err(e) = nexo::compile(&src) else {
        panic!()
    };
    assert_eq!(e.code, "E0003");
}

#[test]
fn composed_ast_depth_is_bounded_before_recursive_passes() {
    let mut expr = "1".to_owned();
    for _ in 0..110 {
        expr = format!("({expr}+{})", vec!["0"; 128].join("+"));
    }
    let Err(e) = nexo::compile(&format!("fn main(){{print({expr});}}")) else {
        panic!("unbounded AST")
    };
    assert_eq!(e.code, "E0003");
}
#[test]
fn inferred_nesting_across_bindings_is_bounded() {
    let mut source = "fn main(){let a0=1;".to_owned();
    for i in 1..=130 {
        source.push_str(&format!("let a{i}=[a{}];", i - 1));
    }
    source.push('}');
    let Err(e) = nexo::compile(&source) else {
        panic!("unbounded type depth")
    };
    assert_eq!(e.code, "E0003");
}
#[test]
fn assignment_checks_negative_bounds_after_rhs() {
    for index in [-1, 1] {
        let r = run(&format!(
            "fn main(){{let r=mock_open(1);var a=[0];a[{index}]=9223372036854775807+1;}}"
        ));
        assert_eq!(r.error.unwrap().code, "R0101");
        assert_eq!(r.events, ["open:0:1", "close:0:1"]);
    }
    let r = run("fn main(){var a=[[0]];a[-1][9223372036854775807+1]=0;}");
    assert_eq!(r.error.unwrap().code, "R0101");
}
#[test]
fn diagnostic_render_handles_arbitrary_utf8_offsets() {
    for offset in 0..10 {
        let e = nexo::Diagnostic::new("E0001", offset, "test");
        assert!(e.render("🙂é").starts_with("E0001:1:"));
    }
}
#[test]
fn work_budget_exhaustion_closes_resources() {
    let mut source = "fn main(){let r=mock_open(7);let a0=[1];".to_owned();
    for i in 1..25 {
        source.push_str(&format!("let a{i}=[a{},a{}];", i - 1, i - 1));
    }
    source.push('}');
    let r = run(&source);
    assert_eq!(r.error.unwrap().code, "R0001");
    assert_eq!(r.events, ["open:0:7", "close:0:7"]);
}

#[test]
fn truncated_and_mutated_inputs_never_panic() {
    let source = "fn main(){let r=mock_open(1);var a=[[1,2]];let b=a;a[0][1]=3;assert_eq(b,[[1,2]]);{let s=move r;inspect(s);}print(a);}";
    for n in 0..=source.len() {
        let _ = nexo::compile(&source[..n]);
    }
    for n in 0..source.len() {
        for replacement in ["", "🙂", "{", "-", "]", "return", "//", "0"] {
            let candidate = format!("{}{}{}", &source[..n], replacement, &source[n + 1..]);
            if let Err(e) = nexo::compile(&candidate) {
                let _ = e.render(&candidate);
            }
        }
    }
}
