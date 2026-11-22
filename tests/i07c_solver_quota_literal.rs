//! F0-0002 §§11.1, 11.8.A, 13.13: literal normal-profile budget recipe.
use super::i07c_normative_literals::invoke;

fn recipe(assignments: usize) -> String {
    let mut code = String::from("fn main(){let a0=0;");
    for j in 0..64 {
        code.push_str(&format!("let a{}=[a{j}];", j + 1));
    }
    code.push_str("var y=a64;");
    for _ in 0..assignments {
        code.push_str("y=a64;");
    }
    code.push('}');
    code
}

#[test]
fn literal_deep_assignment_exhausts_default_solver_budget() {
    // Exactly 15,400 × 65 = 1,001,000 comparisons, §11.8.A.
    let code = recipe(15_400);
    assert!(code.is_ascii() && !code.contains(['\n', '\r', '\u{feff}']));
    assert_eq!(code.len(), 93_307);
    assert!(code.starts_with("fn main(){let a0=0;let a1=[a0];"));
    assert!(code.ends_with("y=a64;}"));
    assert_eq!(code.matches("y=a64;").count(), 15_401);
    assert_eq!(15_400 * 65, 1_001_000);
    // §11.8.A uses the historical `fn main` spelling and promises only check.
    let (control_exit, control, _) = invoke("check", &recipe(1), "f0-0002-b-v2");
    assert_eq!(control_exit, 0, "{control}");
    assert!(
        control.contains("\"outcome\":{\"kind\":\"checked\"}"),
        "{control}"
    );
    let (exit, json, stderr) = invoke("check", &code, "f0-0002-b-v2");
    assert_eq!(exit, 1, "{json} {stderr}");
    assert_eq!(json.matches("\"code\":\"NX-").count(), 1, "{json}");
    for fragment in [
        "\"outcome\":{\"kind\":\"rejected\"}",
        "\"code\":\"NX-E0003\"",
        "\"subcause\":null",
        "\"stage\":\"types\"",
        "\"operation\":\"analysis.limit\"",
        "\"budget_kind\":\"type_solver_steps\"",
        "\"budget_scope\":\"compilation\"",
        "\"incomplete_analysis\":\"types\"",
        "\"limit\":1000000",
        "\"observed\":1000001",
        "\"stdout\":[]",
        "\"events\":[]",
        "\"observations\":[]",
    ] {
        assert!(json.contains(fragment), "missing {fragment}: {json}");
    }
    // §13.13 fixes a nonempty in-source primary, not its exact byte offset.
    let primary = json.split("\"primary_span\":{").nth(1).unwrap();
    let start: usize = primary
        .split("\"start\":")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let end: usize = primary
        .split("\"end\":")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(start < end && end <= code.len() && !code[start..end].trim().is_empty());
}
