use std::process::Command;

fn source(family: &str) -> String {
    match family {
        "block" => format!(
            "fn Main(){{{}let x=1;{}}}",
            "{".repeat(126),
            "}".repeat(126)
        ),
        "paren" => format!(
            "fn Main(){{let x={}1{};}}",
            "(".repeat(126),
            ")".repeat(126)
        ),
        "array" => format!(
            "fn Main(){{let x={}1{};}}",
            "[".repeat(126),
            "]".repeat(126)
        ),
        "mixed" => format!(
            "fn Main(){{{}let x={}1{};{}}}",
            "{".repeat(63),
            "(".repeat(63),
            ")".repeat(63),
            "}".repeat(63)
        ),
        "if" => format!(
            "fn Main(){{let x={}1{};}}",
            "if true then (".repeat(126),
            ") else 0".repeat(126)
        ),
        "binary" => format!(
            "fn Main(){{let x={}1{};}}",
            "1+(".repeat(126),
            ")".repeat(126)
        ),
        "type" => format!(
            "fn F(x:view {}I64{}){{}}fn Main(){{}}",
            "Array<".repeat(126),
            ">".repeat(126)
        ),
        "index" => format!(
            "fn Main(){{let a=[1];{}}}",
            (0..128)
                .map(|i| format!("let x{i}=a[0];"))
                .collect::<String>()
        ),
        "indexchain" => format!("fn Main(){{let x=a{};}}", "[0]".repeat(127)),
        "chain" => format!("fn Main(){{var x=0;{}}}", "x=x+1;".repeat(2000)),
        "captures" => format!(
            "fn Main(){{{}let f=fn [{}](){{{}}};Drop(f);}}",
            (0..500)
                .map(|i| format!("let a{i}={i};"))
                .collect::<String>(),
            (0..500)
                .map(|i| format!("view a{i}"))
                .collect::<Vec<_>>()
                .join(","),
            (0..500)
                .map(|i| format!("let b{i}=a{i};"))
                .collect::<String>()
        ),
        "states" => format!(
            "gen fn G()->I64{{}}fn Main(){{{}{}}}",
            (0..200)
                .map(|i| format!("let g{i}=G();"))
                .collect::<String>(),
            (0..200).map(|i| format!("Drop(g{i});")).collect::<String>()
        ),
        "frames8" => "fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){F5();}fn F5(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}".into(),
        _ => unreachable!(),
    }
}

#[test]
fn measured_stack_child() {
    let Ok(spec) = std::env::var("NEXO_STACK_MINIMUM_CASE") else {
        return;
    };
    let (phase, family) = spec.split_once(':').unwrap();
    if family == "objectives" {
        for name in [
            "01_integer_limits.n",
            "02_nested_copy.n",
            "03_resource_transfer.n",
            "04_return_cleanup.n",
            "05_shadow_scope.n",
            "06_result_ok.n",
            "07_result_error.n",
            "08_array_update.n",
            "09_mock_request.n",
            "10_mock_write.n",
            "11_ex04_generador.n",
            "12_option_match.n",
            "13_option_none.n",
            "14_closure_capture.n",
            "15_generator_sum.n",
        ] {
            eprintln!("objective={name}");
            let source = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("corpus/objetivo")
                    .join(name),
            )
            .unwrap();
            nexo::ir::compile_scalar_with_profile(
                "objective-stack.n",
                &source,
                nexo::ir::ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
                true,
            )
            .unwrap();
        }
        return;
    }
    let source = source(family);
    match phase {
        "FORMATTER" => {
            nexo::frontend::format::format("stack.n", &source).unwrap();
        }
        _ => {
            nexo::ir::compile_scalar_with_profile(
                "stack.n",
                &source,
                nexo::ir::ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
                true,
            )
            .unwrap();
        }
    }
}

#[test]
fn measured_frontend_stack_minima_hold_for_valid_families() {
    let binary = std::env::current_exe().unwrap();
    for phase in ["PARSER", "CHECKER", "LOWERING", "FORMATTER"] {
        let minimum = match phase {
            "FORMATTER" => {
                if cfg!(debug_assertions) {
                    1024
                } else {
                    256
                }
            }
            _ => {
                if cfg!(debug_assertions) {
                    8192
                } else {
                    1024
                }
            }
        };
        let families: &[&str] = if phase == "FORMATTER" {
            &[
                "block",
                "paren",
                "array",
                "mixed",
                "if",
                "binary",
                "type",
                "index",
                "indexchain",
                "chain",
            ]
        } else {
            &[
                "block",
                "paren",
                "array",
                "mixed",
                "if",
                "binary",
                "type",
                "index",
                "chain",
                "captures",
                "states",
                "frames8",
                "objectives",
            ]
        };
        for family in families {
            let output = Command::new(&binary)
                .args(["--exact", "measured_stack_child"])
                .env("NEXO_STACK_MINIMUM_CASE", format!("{phase}:{family}"))
                .env(format!("NEXO_PROBE_{phase}_KIB"), minimum.to_string())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{phase}/{family}/{minimum} KiB: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
