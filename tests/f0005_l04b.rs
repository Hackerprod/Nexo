//! F5-L04b: public typing and activation of `while`/`if` control in `f0-0005-b-v1`.
//!
//! Expected values come from F0-0005 (`docs/adr/F0-0005-operators-control-collections.md`):
//! §2.2 «Cortocircuito, propiedad y retorno» (paragraph on `while true`), §8.2 «Barrera L08
//! para capacidades nuevas» and the F5-C02/C03/C04 rows of §9.1, never from the binary.

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const F5: &str = "f0-0005-b-v1";
const J: &str = "f0-0003-b-v1";
const A2: &str = "f0-0002-b-v2";

fn invoke(verb: &str, source: &str, profile: &str) -> (i32, String) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let folder = format!(
        "C:/nexo-work/tmp/f0005-l04b-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    fs::create_dir_all(&folder).unwrap();
    fs::write(format!("{folder}/case.n"), source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .current_dir(&folder)
        .args([verb, "case.n", "--profile", profile, "--format", "json"])
        .output()
        .unwrap();
    fs::remove_dir_all(folder).unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap().trim().into(),
    )
}

/// F5-H01 shape of §8.2: the L08 barrier on the first node of a new capability.
fn l08(start: u64, end: u64) -> String {
    format!(
        "{{\"schema_version\":1,\"profile_id\":\"{F5}\",\"outcome\":{{\"kind\":\"rejected\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[{{\"schema_version\":1,\"diagnostic_namespace\":\"Nexo-proposed-v1\",\"profile_id\":\"{F5}\",\"code\":\"NX-E0006\",\"subcause\":null,\"rule\":\"NX-E0006\",\"explanation_id\":\"NX-E0006\",\"severity\":\"error\",\"stage\":\"lower\",\"message\":\"Capacidad pendiente de implementación en L08.\",\"operation\":\"ir.lower\",\"primary_span\":{{\"file\":\"case.n\",\"start\":{start},\"end\":{end}}},\"related_spans\":[],\"cause_chain\":[],\"feature_id\":\"implementation_limit\",\"unexpected_token\":null,\"expected_tokens\":[],\"expected_tokens_kind\":null,\"parser_context\":null,\"recognized_prefix\":null,\"repair\":[],\"data\":{{\"pending_scope\":\"L08\"}}}}]}}"
    )
}

fn assert_single(output: &str, code: &str, stage: &str, start: u64, end: u64, why: &str) {
    assert_eq!(output.matches("\"code\":").count(), 1, "{why}: {output}");
    assert!(
        output.contains(&format!("\"code\":\"{code}\"")),
        "{why}: {output}"
    );
    assert!(
        output.contains(&format!("\"stage\":\"{stage}\"")),
        "{why}: {output}"
    );
    assert!(
        output.contains(&format!(
            "\"primary_span\":{{\"file\":\"case.n\",\"start\":{start},\"end\":{end}}}"
        )),
        "{why}: {output}"
    );
}

/// F5-C02 (§9.1): NX-E1317, ownership/flow.return, expected_type [8,11) of `->I64`.
fn assert_missing_return(output: &str, start: u64, why: &str) {
    assert_single(output, "NX-E1317", "ownership", start, start + 1, why);
    assert!(
        output.contains("\"operation\":\"flow.return\""),
        "{why}: {output}"
    );
    assert!(
        output.contains(
            "\"related_spans\":[{\"role\":\"expected_type\",\"span\":{\"file\":\"case.n\",\"start\":8,\"end\":11}}]"
        ),
        "{why}: {output}"
    );
}

fn both(source: &str, profile: &str) -> [(i32, String); 2] {
    [
        invoke("check", source, profile),
        invoke("run", source, profile),
    ]
}

#[test]
fn literal_true_loop_with_return_satisfies_the_result_type() {
    // §2.2: `while true{return 5;}` satisfies `->I64`; the false edge does not exist, even
    // under groups. The source is new F5 capability, so §8.2 puts it behind the L08
    // barrier on the first node of the capability (the `while` keyword), never NX-E1317.
    for source in [
        "fn F()->I64{while true{return 1;}}fn Main(){}",
        "fn F()->I64{while true{return 5;}}fn Main(){}",
        "fn F()->I64{while (true){return 1;}}fn Main(){}",
        "fn F()->I64{while ((true)){return 1;}}fn Main(){}",
        "fn F()->I64{while true{return 1;}}fn Main(){F();}",
    ] {
        for (status, output) in both(source, F5) {
            assert_eq!(status, 1, "{source}: {output}");
            assert_eq!(output, l08(12, 17), "{source}");
        }
    }
}

#[test]
fn literal_true_loop_with_break_keeps_the_missing_return() {
    // §2.2: `while true{break;}` without a later return keeps NX-E1317 (F5-C02, [30,31)).
    for (status, output) in both("fn F()->I64{while true{break;}}fn Main(){}", F5) {
        assert_eq!(status, 1, "{output}");
        assert_missing_return(&output, 30, "break creates the normal exit");
    }
    // F5-C02 positive edit: break replaced by `return 5;`.
    for (status, output) in both("fn F()->I64{while true{return 5;}}fn Main(){}", F5) {
        assert_eq!(status, 1, "{output}");
        assert_eq!(output, l08(12, 17));
    }
}

#[test]
fn false_and_non_literal_conditions_get_no_special_proof() {
    // §2.2: `while false` receives no special ownership optimization; the proof is not
    // propagated to aliases, constants, `not false` or calls.
    for (source, closing) in [
        ("fn F()->I64{while false{}}fn Main(){}", 25),
        ("fn F()->I64{while false{return 1;}}fn Main(){}", 34),
        ("fn F()->I64{let x=true;while x{return 1;}}fn Main(){}", 41),
        ("fn F()->I64{while not false{return 1;}}fn Main(){}", 38),
        ("fn F()->I64{while (not false){return 1;}}fn Main(){}", 40),
        ("fn F()->I64{while true and true{return 1;}}fn Main(){}", 42),
        ("fn F()->I64{while true==true{return 1;}}fn Main(){}", 39),
    ] {
        for (status, output) in both(source, F5) {
            assert_eq!(status, 1, "{source}: {output}");
            assert_missing_return(&output, closing, source);
        }
    }
}

#[test]
fn nested_loops_exit_only_through_their_own_break() {
    // §2.2: a break creates the exit of its own loop. The inner break does not give the outer
    // literal loop a false edge, so both functions meet the barrier on the outer `while`.
    for source in [
        "fn F()->I64{while true{while true{break;}}}fn Main(){}",
        "fn F()->I64{while true{while true{return 1;}}}fn Main(){}",
        "fn F()->I64{while true{while false{}}}fn Main(){}",
    ] {
        for (status, output) in both(source, F5) {
            assert_eq!(status, 1, "{source}: {output}");
            assert_eq!(output, l08(12, 17), "{source}");
        }
    }
    // A reachable outer break is a normal exit (F5-C02 again, at the closing brace); a break
    // after an inner loop that never exits is not reachable and creates no exit.
    for (status, output) in both(
        "fn F()->I64{while true{while false{}break;}}fn Main(){}",
        F5,
    ) {
        assert_eq!(status, 1, "{output}");
        assert_missing_return(&output, 43, "outer break");
    }
    for (status, output) in both(
        "fn F()->I64{while true{while true{return 1;}break;}}fn Main(){}",
        F5,
    ) {
        assert_eq!(status, 1, "{output}");
        assert_eq!(output, l08(12, 17));
    }
}

#[test]
fn barrier_anchors_the_loop_that_needs_the_proof() {
    // §8.2: the barrier names the first node of the new capability. A literal loop that the
    // historical rule already accepts (here in `Main`, at offset 10) is not that capability;
    // the loop of `F` (offset 30) is.
    for source in [
        "fn Main(){while true{break;}}fn F()->I64{while true{return 1;}}",
        "fn Main(){while true{break;}}fn F()->I64{while (true){return 1;}}",
    ] {
        for (status, output) in both(source, F5) {
            assert_eq!(status, 1, "{source}: {output}");
            assert_eq!(output, l08(41, 46), "{source}");
        }
    }
    // §8.2 names the first source node of the capability and §2.2 gives `while false` no
    // special treatment: it is not a literal loop, so it never anchors the barrier even when
    // it precedes the `while true` in the same rejected function.
    for (status, output) in both(
        "fn F()->I64{while false{}while true{return 1;}}fn Main(){}",
        F5,
    ) {
        assert_eq!(status, 1, "{output}");
        assert_eq!(output, l08(25, 30));
    }
    // Two functions: the lower offset wins (§8.2).
    for (status, output) in both(
        "fn A()->I64{while true{return 1;}}fn B()->I64{while true{return 2;}}fn Main(){}",
        F5,
    ) {
        assert_eq!(status, 1, "{output}");
        assert_eq!(output, l08(12, 17));
    }
    // A function that keeps NX-E1317 hides the barrier of the other one (a real error
    // precedes the lower barrier): `A` has the missing return of F5-C02, `B` needs the proof.
    for (status, output) in both(
        "fn A()->I64{while true{break;}}fn B()->I64{while true{return 2;}}fn Main(){}",
        F5,
    ) {
        assert_eq!(status, 1, "{output}");
        assert_missing_return(&output, 30, "A keeps the normal exit");
    }
}

#[test]
fn bool_condition_is_still_required() {
    // F5 §1.1 keeps the J/D15 rule: a loop condition is Bool (O07/O08 show the same
    // NX-E0102/type_mismatch family for non-Bool operands). The type error keeps its stage
    // and precedes the lower barrier; the result equals the J profile modulo the profile id.
    for source in [
        "fn F()->I64{while 1{return 1;}}fn Main(){}",
        "fn Main(){while 1{}}",
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, F5);
            assert_eq!(status, 1, "{source}: {output}");
            assert!(output.contains("\"code\":\"NX-E0102\""), "{output}");
            assert!(output.contains("\"stage\":\"types\""), "{output}");
            assert!(output.contains("\"expected_type\":\"Bool\""), "{output}");
            assert_eq!(output.replace(F5, J), invoke(verb, source, J).1, "{source}");
        }
    }
}

#[test]
fn earlier_stages_and_earlier_capabilities_precede_the_loop_barrier() {
    // §8.2: a real type error keeps its stage and precedence before the lower barrier.
    for (status, output) in both("fn F()->I64{while true{return true;}}fn Main(){}", F5) {
        assert_eq!(status, 1);
        assert_single(
            &output,
            "NX-E0102",
            "types",
            30,
            34,
            "type error before lower",
        );
    }
    // §8.2: among capabilities of the same barrier the lower offset wins. `*` (F5-H01 form,
    // span of `1*2`) comes first here; the loop comes first in the second source.
    for (status, output) in both(
        "fn F()->I64{let x=1*2;while true{return x;}}fn Main(){}",
        F5,
    ) {
        assert_eq!(status, 1);
        assert_eq!(output, l08(18, 21));
    }
    for (status, output) in both("fn F()->I64{while true{return 1*2;}}fn Main(){}", F5) {
        assert_eq!(status, 1);
        assert_eq!(output, l08(12, 17));
    }
}

#[test]
fn inherited_loop_forms_need_no_new_capability() {
    // §8.2 «Positivo heredado»: forms that the historical rule already accepts do not meet
    // the barrier, with or without a literal-`true` loop.
    for source in [
        "fn Main(){while true{break;}}",
        "fn Main(){while (true){break;}}",
        "fn Main(){while true{return;}}",
        "fn Main(){while false{}}",
        "fn Main(){while false{break;}}",
        "fn Main(){var i=0;while i<3{i=i+1;}}",
        "fn F()->I64{while true{break;}return 1;}fn Main(){}",
        "fn F()->I64{while true{} return 1;}fn Main(){}",
        "fn F()->I64{while true{while true{break;}break;}return 1;}fn Main(){}",
        "fn F()->I64{while false{return 1;}return 2;}fn Main(){}",
    ] {
        let (status, output) = invoke("check", source, F5);
        assert_eq!(status, 0, "{source}: {output}");
        assert!(
            output.contains("\"outcome\":{\"kind\":\"checked\"}"),
            "{output}"
        );
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
    }
    // The inherited loop is not the capability: the barrier of a later `*` (F5-H01 form,
    // span of `1*2`) stays on `*` even though the loop comes first in the source.
    for (status, output) in both("fn Main(){while true{break;}let x=1*2;}", F5) {
        assert_eq!(status, 1, "{output}");
        assert_eq!(output, l08(34, 37));
    }
    let (status, output) = invoke("run", "fn Main(){while true{break;}}", F5);
    assert_eq!(status, 0, "{output}");
    assert!(
        output.contains("\"outcome\":{\"kind\":\"unit\"}"),
        "{output}"
    );
}

#[test]
fn loop_control_outside_a_loop_is_unchanged() {
    // F5-C03/F5-C04 (§9.1): NX-E1326, subcause break_/continue_outside_loop, types/types.check
    // on `break` [10,15) and `continue` [10,18); the wrapped forms of the table are positive.
    for (source, subcause, end) in [
        ("fn Main(){break;}", "break_outside_loop", 15),
        ("fn Main(){continue;}", "continue_outside_loop", 18),
    ] {
        for (status, output) in both(source, F5) {
            assert_eq!(status, 1, "{source}: {output}");
            assert_single(&output, "NX-E1326", "types", 10, end, source);
            assert!(
                output.contains(&format!("\"subcause\":\"{subcause}\"")),
                "{output}"
            );
            assert!(output.contains("\"operation\":\"types.check\""), "{output}");
        }
    }
    for source in [
        "fn Main(){while true{break;}}",
        "fn Main(){while false{continue;}}",
    ] {
        let (status, output) = invoke("check", source, F5);
        assert_eq!(status, 0, "{source}: {output}");
    }
}

#[test]
fn loop_control_after_a_guarded_match_arm_is_not_a_guard_control() {
    // F0-0003 §8.2 / F5-C03, F5-C04: a `break`/`continue` outside a guard is judged only by
    // the enclosing loop; a guarded arm that has already ended leaves no guard context behind.
    // Outside a loop: NX-E1326 on the token (`break` [38,43), `continue` [38,46)). Inside a
    // loop the same statements are accepted.
    for (source, subcause, end) in [
        (
            "fn Main(){match 1{1 if true=>{},_=>{}}break;}",
            "break_outside_loop",
            43,
        ),
        (
            "fn Main(){match 1{1 if true=>{},_=>{}}continue;}",
            "continue_outside_loop",
            46,
        ),
    ] {
        for (status, output) in both(source, F5) {
            assert_eq!(status, 1, "{source}: {output}");
            assert_single(&output, "NX-E1326", "types", 38, end, source);
            assert!(
                output.contains(&format!("\"subcause\":\"{subcause}\"")),
                "{output}"
            );
        }
    }
    for source in [
        "fn Main(){while true{match 1{1 if true=>{},_=>{}}break;}}",
        "fn Main(){while false{match 1{1 if true=>{},_=>{}}continue;}}",
    ] {
        let (status, output) = invoke("check", source, F5);
        assert_eq!(status, 0, "{source}: {output}");
    }
}

#[test]
fn loop_control_after_a_for_with_a_rejected_iterable_is_outside_the_loop() {
    // F0-0003 §8.2 / F5-C03, F5-C04: the loop of a `for` ends with its body, also when the
    // iterable was rejected (`1+true`, NX-E0102 on `true` [22,26) by F0-0002 §4.1). The
    // `break`/`continue` that follows has no loop: NX-E1326 on the token (`break` [29,34),
    // `continue` [29,37)). Inside an outer `while` the same shape reports only NX-E0102.
    for profile in [J, F5] {
        for (source, subcause, end) in [
            (
                "fn Main(){for x in [1+true]{}break;}",
                "break_outside_loop",
                34,
            ),
            (
                "fn Main(){for x in [1+true]{}continue;}",
                "continue_outside_loop",
                37,
            ),
        ] {
            for (status, output) in both(source, profile) {
                assert_eq!(status, 1, "{profile} {source}: {output}");
                assert_eq!(output.matches("\"code\":").count(), 2, "{source}: {output}");
                assert!(
                    output.contains("\"code\":\"NX-E0102\"")
                        && output.contains(
                            "\"primary_span\":{\"file\":\"case.n\",\"start\":22,\"end\":26}"
                        ),
                    "{profile} {source}: {output}"
                );
                assert!(
                    output.contains("\"code\":\"NX-E1326\"")
                        && output.contains(&format!("\"subcause\":\"{subcause}\""))
                        && output.contains(&format!(
                            "\"primary_span\":{{\"file\":\"case.n\",\"start\":29,\"end\":{end}}}"
                        )),
                    "{profile} {source}: {output}"
                );
            }
        }
        for (status, output) in both("fn Main(){while true{for x in [1+true]{}break;}}", profile) {
            assert_eq!(status, 1, "{profile}: {output}");
            assert_eq!(output.matches("\"code\":").count(), 1, "{output}");
            assert!(output.contains("\"code\":\"NX-E0102\""), "{output}");
        }
    }
}

#[test]
fn historical_profiles_keep_the_false_edge() {
    // F0-0005 §12.1: the new rule belongs to `f0-0005-b-v1` only. A2 and J keep NX-E1317 at
    // the closing brace of F5-C02's shape (`}` [33,34) for this source) for the same bytes.
    for profile in [A2, J] {
        for source in [
            "fn F()->I64{while true{return 1;}}fn Main(){}",
            "fn F()->I64{while true{return 5;}}fn Main(){}",
            "fn F()->I64{while (true){return 1;}}fn Main(){}",
        ] {
            let end = if source.contains("(true)") { 35 } else { 33 };
            for (status, output) in both(source, profile) {
                assert_eq!(status, 1, "{profile} {source}: {output}");
                assert_missing_return(&output, end, source);
                assert!(
                    output.contains(&format!("\"profile_id\":\"{profile}\"")),
                    "{output}"
                );
            }
        }
        let (status, output) = invoke(
            "check",
            "fn F()->I64{while false{}return 1;}fn Main(){}",
            profile,
        );
        assert_eq!(status, 0, "{profile}: {output}");
    }
    // `break` itself exists in J only (A2 reports loop_control); J keeps its accepted forms.
    for source in [
        "fn Main(){while true{break;}}",
        "fn F()->I64{while true{break;}return 1;}fn Main(){}",
    ] {
        let (status, output) = invoke("check", source, J);
        assert_eq!(status, 0, "{source}: {output}");
    }
}

#[test]
fn literal_true_activation_does_not_open_for_over_array() {
    // §8.2 and F5-L06: `for` over an Array stays pending (public L06 in `types`) until F5-L06b,
    // and no new source is accepted before L08. The literal-`true` CFG of F5-L04 is a different
    // capability from the For-Array lowering of F5-L06a; activating the first must not accept
    // the second. This source reaches `lower` with an Array iterand (an `internal_error` in the
    // origin tree, outside L04b); it must never be accepted by `check` or `run`.
    let source = "fn Main(){var v2=[];let n=Len(v2);for x in v2{}v2[0]=7;}";
    for (_, output) in both(source, F5) {
        assert!(
            !output.contains("\"kind\":\"checked\"") && !output.contains("\"kind\":\"unit\""),
            "{output}"
        );
    }
    // The public L06 pending of F5-H06 is untouched.
    for (status, output) in both("fn Main(){for x in [1]{}}", F5) {
        assert_eq!(status, 1, "{output}");
        assert_single(&output, "NX-E0006", "types", 10, 13, "F5-H06");
        assert!(
            output.contains("\"data\":{\"pending_scope\":\"L06\"}"),
            "{output}"
        );
    }
}
