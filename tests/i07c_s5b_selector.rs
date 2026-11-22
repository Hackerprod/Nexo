//! F0-0002 §§4.1.1, 4.1.2.3, 4.1.2.4/N(d): an invalid index base
//! cannot suppress an independent error inside its selector.
use super::i07c_normative_literals::{accepted, invoke, rejected};

const PROFILE: &str = "f0-0002-b-v2";

struct Mismatch {
    span: (usize, usize),
    expected: &'static str,
    found: &'static str,
    expected_origin: Option<(usize, usize)>,
}

fn mismatches(source: &str, expected: &[Mismatch]) {
    assert!(source.is_ascii());
    for verb in ["check", "run"] {
        let (exit, json, stderr) = invoke(verb, source, PROFILE);
        assert_eq!(exit, 1, "{verb}: {source}: {json}: {stderr}");
        assert!(stderr.is_empty(), "{verb}: {stderr}");
        assert!(
            json.contains("\"outcome\":{\"kind\":\"rejected\"}"),
            "{json}"
        );
        assert!(
            json.contains("\"stdout\":[]") && json.contains("\"events\":[]"),
            "{json}"
        );
        let diagnostics: Vec<_> = json
            .split("{\"schema_version\":1,\"diagnostic_namespace\"")
            .skip(1)
            .collect();
        assert_eq!(
            diagnostics.len(),
            expected.len(),
            "{verb}: {source}: {json}"
        );
        for (actual, wanted) in diagnostics.iter().zip(expected) {
            let (start, end) = wanted.span;
            assert!(start < end && end <= source.len(), "{source}");
            for fragment in [
                "\"code\":\"NX-E0102\"".to_owned(),
                "\"subcause\":\"type_mismatch\"".to_owned(),
                "\"rule\":\"NX-E0102/type_mismatch\"".to_owned(),
                "\"explanation_id\":\"NX-E0102/type_mismatch\"".to_owned(),
                "\"stage\":\"types\"".to_owned(),
                "\"operation\":\"types.check\"".to_owned(),
                "\"primary_span\":{\"file\":".to_owned(),
                "\"cause_chain\":[]".to_owned(),
                "\"repair\":[]".to_owned(),
                "\"feature_id\":null".to_owned(),
                "\"unexpected_token\":null".to_owned(),
                "\"expected_tokens\":[]".to_owned(),
                "\"parser_context\":null".to_owned(),
                format!(
                    "\"data\":{{\"expected_type\":\"{}\",\"found_type\":\"{}\"}}",
                    wanted.expected, wanted.found
                ),
            ] {
                assert!(
                    actual.contains(&fragment),
                    "{verb}: {source}: {fragment}: {json}"
                );
            }
            let primary = actual
                .split("\"primary_span\":")
                .nth(1)
                .unwrap()
                .split(",\"related_spans\":")
                .next()
                .unwrap();
            assert!(
                primary.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{source}: {json}"
            );
            let related = actual.split("\"related_spans\":").nth(1).unwrap();
            let related = related.split(",\"cause_chain\"").next().unwrap_or(related);
            let expected_count = usize::from(wanted.expected_origin.is_some());
            assert_eq!(
                related.matches("\"role\":\"expected_type\"").count(),
                expected_count,
                "{json}"
            );
            assert_eq!(
                related.matches("\"role\":\"found_type\"").count(),
                1,
                "{json}"
            );
            if let Some((origin_start, origin_end)) = wanted.expected_origin {
                assert!(
                    related.contains("\"role\":\"expected_type\",\"span\":{\"file\":")
                        && related
                            .contains(&format!("\"start\":{origin_start},\"end\":{origin_end}")),
                    "{json}"
                );
            }
            assert!(
                related.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{json}"
            );
        }
    }
}

fn mismatch(span: (usize, usize), expected_origin: Option<(usize, usize)>) -> Mismatch {
    Mismatch {
        span,
        expected: "I64",
        found: "Bool",
        expected_origin,
    }
}

#[test]
fn selector_array_error_survives_failed_base() {
    let source = "fn Main(){let ar=[1];let z=ar[true][[1,true]];}";
    mismatches(
        source,
        &[mismatch((30, 34), None), mismatch((39, 43), Some((37, 38)))],
    );
    mismatches(
        "fn Main(){let ar=[1];let z=ar[true][[1,2]];}",
        &[mismatch((30, 34), None)],
    );
    accepted("fn Main(){let ar=[1];let z=ar[0];}", PROFILE);
}

#[test]
fn selector_array_error_survives_failed_binary_base() {
    mismatches(
        "fn Main(){let ar=[1];let z=(1+true)[[1,true]];}",
        &[mismatch((30, 34), None), mismatch((39, 43), Some((37, 38)))],
    );
    mismatches(
        "fn Main(){let ar=[1];let z=(1+true)[[1,2]];}",
        &[mismatch((30, 34), None)],
    );
}

#[test]
fn selector_operator_error_survives_failed_base() {
    mismatches(
        "fn Main(){let ar=[1];let z=ar[true][2+true];}",
        &[mismatch((30, 34), None), mismatch((38, 42), None)],
    );
    mismatches(
        "fn Main(){let ar=[1];let z=ar[true][2+2];}",
        &[mismatch((30, 34), None)],
    );
}

#[test]
fn nested_failed_base_still_checks_outer_selector() {
    mismatches(
        "fn Main(){let ar=[1];let z=ar[true][0][[1,true]];}",
        &[mismatch((30, 34), None), mismatch((42, 46), Some((40, 41)))],
    );
    mismatches(
        "fn Main(){let ar=[1];let z=ar[true][0][[1,2]];}",
        &[mismatch((30, 34), None)],
    );
}

#[test]
fn tainted_and_pending_bases_keep_their_existing_selector_behavior() {
    mismatches(
        "fn Main(){let ar=[1];let t=ar[true];let z=t[[1,true]];}",
        &[mismatch((30, 34), None), mismatch((47, 51), Some((45, 46)))],
    );
    mismatches(
        "fn Main(){let ar=[1];let t=ar[true];let z=t[2];}",
        &[mismatch((30, 34), None)],
    );
    mismatches(
        "fn Main(){var s=[];let z=s[0][[1,true]];s=[[3]];}",
        &[mismatch((33, 37), Some((31, 32)))],
    );
}

#[test]
fn independent_selector_and_nonindexable_base_controls() {
    rejected(
        "fn Main(){let ar=[1];let z=[1,true];}",
        PROFILE,
        "NX-E0102",
        Some("type_mismatch"),
        (30, 34),
        &[("expected_type", "I64"), ("found_type", "Bool")],
        &[("expected_type", 28, 29), ("found_type", 30, 34)],
    );
    for verb in ["check", "run"] {
        let (_, json, stderr) = invoke(
            verb,
            "fn Main(){let ar=[1];let z=ar[0][[1,true]];}",
            PROFILE,
        );
        assert!(stderr.is_empty());
        assert_eq!(json.matches("\"code\":\"NX-").count(), 2, "{json}");
        assert!(json.contains("\"subcause\":\"not_indexable\""), "{json}");
        assert!(json.contains("\"start\":36,\"end\":40"), "{json}");
    }
}
