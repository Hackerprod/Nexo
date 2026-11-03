// Source-order permutations and concrete twins for delayed type diagnostics.
// Expected complete type labels are transcribed from F0-0002 §§4.1.2.4,6,
// 13.17 and RD120; they are not derived from the implementation under test.
use crate::common::{R, check, has_dup, run};

fn observed(source: &str) -> Result<Vec<(String, String, String, String)>, String> {
    let checked = check(source);
    if has_dup(&checked) || checked != run(source).0 {
        return Err(format!(
            "duplicate or check/run mismatch: {source}: {checked:?}"
        ));
    }
    match checked {
        R::Rej(ds) => Ok(ds
            .into_iter()
            .map(|d| {
                let token = source
                    .get(d.s as usize..d.e as usize)
                    .unwrap_or("?")
                    .to_owned();
                (d.code, token, d.exp, d.found)
            })
            .collect()),
        other => Err(format!("unexpected acceptance or bug: {source}: {other:?}")),
    }
}

pub fn run_cases(n: u64, _seed: u64, _threads: u64) {
    let dv = "(if true then {return;}else{return;})";
    let mut failures = Vec::new();
    for i in 1..=n {
        let number = i % 10_000 + 1;
        let (first, second) = if i % 2 == 0 {
            (
                format!("let ar=[{number}];"),
                format!("let aar=[[{number}]];"),
            )
        } else {
            (
                format!("let aar=[[{number}]];"),
                format!("let ar=[{number}];"),
            )
        };
        let noise = format!("let noise=[{number}];");
        let (before, after) = if i % 2 == 0 {
            (noise.as_str(), "")
        } else {
            ("", noise.as_str())
        };
        let b1 = format!("fn Main(){{{first}{second}var r=[];let a=[r,ar,aar];r=[{number}];}}");
        let b1_twin = format!("fn Main(){{{first}{second}let r=[{number}];let a=[r,ar,aar];}}");
        let b1_reversed = format!("fn Main(){{{first}{second}var r=[];let a=[r,aar,ar];}}");
        let b1_reversed_twin =
            format!("fn Main(){{{first}{second}let r=[{number}];let a=[r,aar,ar];}}");
        let b2 =
            format!("fn Main(){{{before}var r=[];let a=[1,r]+[{number}];r=[{number}];{after}}}");
        let b2_twin =
            format!("fn Main(){{{before}let r=[{number}];let a=[1,r]+[{number}];{after}}}");
        let b1_index =
            format!("fn Main(){{let ar=[{number}];var r=[];let a=ar==r[r[1]];r=[{number}];}}");
        let b1_index_twin =
            format!("fn Main(){{let ar=[{number}];let r=[{number}];let a=ar==r[r[1]];}}");
        let b3 = format!("fn Main(){{{before}var q={dv};let y=(true+{number})<q;q=true;{after}}}");
        let b3_selector =
            format!("fn Main(){{{before}var q={dv};let a=q[q<q];q=[{number}];{after}}}");
        let expected_b1 = vec![(
            "NX-E0102".into(),
            "aar".into(),
            "Array<I64>".into(),
            "Array<Array<I64>>".into(),
        )];
        let expected_b1_reversed = vec![(
            "NX-E0102".into(),
            "ar".into(),
            "Array<Array<I64>>".into(),
            "Array<I64>".into(),
        )];
        let expected_index = vec![(
            "NX-E0102".into(),
            "r[r[1]]".into(),
            "Array<I64>".into(),
            "I64".into(),
        )];
        let expected_b2 = vec![
            (
                "NX-E0102".into(),
                "r".into(),
                "I64".into(),
                "Array<I64>".into(),
            ),
            (
                "NX-E0102".into(),
                format!("[{number}]"),
                "I64".into(),
                "Array<I64>".into(),
            ),
        ];
        let expected_b3 = vec![
            (
                "NX-E0102".into(),
                "true".into(),
                "I64".into(),
                "Bool".into(),
            ),
            (
                "NX-E0102".into(),
                "true".into(),
                "I64".into(),
                "Bool".into(),
            ),
        ];
        let expected_selector = vec![
            (
                "NX-E0102".into(),
                "q".into(),
                "domain:array".into(),
                "I64".into(),
            ),
            (
                "NX-E0102".into(),
                format!("[{number}]"),
                "I64".into(),
                "Array<I64>".into(),
            ),
        ];
        for (source, expected) in [
            (b1, &expected_b1),
            (b1_twin, &expected_b1),
            (b1_reversed, &expected_b1_reversed),
            (b1_reversed_twin, &expected_b1),
            (b1_index, &expected_index),
            (b1_index_twin, &expected_index),
            (b2, &expected_b2),
            (b2_twin, &expected_b2),
            (b3, &expected_b3),
            (b3_selector, &expected_selector),
        ] {
            match observed(&source) {
                Ok(actual) if &actual == expected => {}
                other => {
                    if failures.len() < 20 {
                        failures.push(format!("{source}: {other:?}; expected {expected:?}"));
                    }
                }
            }
        }
    }
    println!(
        "TYPES programs={} field_diff={} duplicates=0 bugs=0",
        n * 10,
        failures.len()
    );
    for failure in &failures {
        println!("{failure}");
    }
    assert!(failures.is_empty(), "type differential failed");
}
