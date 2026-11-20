use super::{
    ReducedProfile, compile_scalar_with_profile,
    model::{self, Intrinsic, Op},
    verify,
};
use crate::interp::{Execution, Outcome, run_scalar};

fn replace_call(source: &str, intrinsic: Intrinsic) -> model::Program {
    let mut program = compile_scalar_with_profile(
        "case.n",
        source,
        ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
        true,
    )
    .unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let call = function
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::CallDirect { .. }))
        .unwrap();
    let Op::CallDirect { args, .. } = &call.op else {
        unreachable!()
    };
    call.op = Op::CallIntrinsic {
        intrinsic,
        args: args.iter().map(|(_, id)| *id).collect(),
    };
    function.content_hash = model::content_hash(function);
    program
}

#[test]
fn push_installs_after_exclusive_loan_and_grows() {
    for suffix in ["", "if true{}"] {
        let source = format!(
            "fn F(a:edit Array<I64>,n:move I64){{}}fn Main(){{var a=[1];F(edit a,2);AssertEq(a,[1,2]);{suffix}}}"
        );
        let program = replace_call(&source, Intrinsic::Push);
        verify::verify_program(&program).unwrap();
        let result = run_scalar(&program, Default::default());
        assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
    }
}

#[test]
fn pop_moves_last_element_and_preserves_empty_capacity() {
    let program = replace_call(
        "fn F(a:edit Array<I64>)->Option<I64>{return Some(1);}fn Main(){var a=[1,2];AssertEq(F(edit a),Some(2));AssertEq(a,[1]);}",
        Intrinsic::Pop,
    );
    verify::verify_program(&program).unwrap();
    let result = run_scalar(&program, Default::default());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);

    let empty = replace_call(
        "fn F(a:edit Array<I64>)->Option<I64>{return Some(1);}fn Main(){var a:Array<I64>=[];AssertEq(F(edit a),None);AssertEq(a,[]);}",
        Intrinsic::Pop,
    );
    verify::verify_program(&empty).unwrap();
    let result = run_scalar(&empty, Default::default());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
}

#[test]
fn join_copies_elements_in_left_then_right_order() {
    let program = replace_call(
        "fn F(a:view Array<I64>,b:view Array<I64>)->Array<I64>{return [1,2];}fn Main(){let a=[1];let b=[2];AssertEq(F(a,b),[1,2]);AssertEq(a,[1]);AssertEq(b,[2]);}",
        Intrinsic::JoinArrays,
    );
    verify::verify_program(&program).unwrap();
    let result = run_scalar(&program, Default::default());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);

    let empty = replace_call(
        "fn F(a:view Array<I64>,b:view Array<I64>)->Array<I64>{return [];}fn Main(){let a:Array<I64>=[];AssertEq(F(a,a),[]);}",
        Intrinsic::JoinArrays,
    );
    verify::verify_program(&empty).unwrap();
    let result = run_scalar(&empty, Default::default());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
}

#[test]
fn nested_arrays_remain_independent_after_copy_and_push() {
    let program = replace_call(
        "fn F(a:edit Array<Array<I64>>,n:move Array<I64>){}fn Main(){var a=[[1]];var b=a;F(edit b,[2]);AssertEq(a,[[1]]);AssertEq(b,[[1],[2]]);}",
        Intrinsic::Push,
    );
    verify::verify_program(&program).unwrap();
    let result = run_scalar(&program, Default::default());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
}

#[test]
fn array_intrinsics_reject_wrong_element_or_permission_before_dispatch() {
    for (source, intrinsic, error) in [
        (
            "fn F(a:edit Array<I64>,n:move Bool){}fn Main(){var a=[1];F(edit a,true);}",
            Intrinsic::Push,
            "array intrinsic type",
        ),
        (
            "fn F(a:view Array<I64>,n:move I64){}fn Main(){let a=[1];F(a,2);}",
            Intrinsic::Push,
            "array edit permission",
        ),
        (
            "fn F(a:edit Array<I64>)->Option<Bool>{return Some(true);}fn Main(){var a=[1];F(edit a);}",
            Intrinsic::Pop,
            "instruction result type",
        ),
        (
            "fn F(a:view Array<I64>,b:view Array<Bool>)->Array<I64>{return [1];}fn Main(){let a=[1];let b=[true];F(a,b);}",
            Intrinsic::JoinArrays,
            "array intrinsic type",
        ),
    ] {
        let program = replace_call(source, intrinsic);
        assert_eq!(verify::verify_program(&program), Err(error), "{source}");
    }
}

#[test]
fn pop_reserves_option_tag_before_removing_last_element() {
    for (length, expected_fault) in [(62, false), (63, true)] {
        let items = vec!["0"; length].join(",");
        let source = format!(
            "fn F(a:edit Array<I64>)->Option<I64>{{return Some(0);}}fn Main(){{var a=[{items}];F(edit a);}}"
        );
        let program = replace_call(&source, Intrinsic::Pop);
        verify::verify_program(&program).unwrap();
        let (result, snapshots) =
            Execution::run_with_fault_snapshots_cells64(&program, Default::default());
        if expected_fault {
            assert_eq!(result.outcome, Outcome::SimulatorFault);
            assert_eq!(result.diagnostics[0].code, "R0002");
            let expected = format!("[{}]", vec!["0"; length].join(", "));
            assert!(
                snapshots
                    .iter()
                    .flatten()
                    .any(|value| value.as_deref() == Some(expected.as_str())),
                "{:?} {:?}",
                result.diagnostics,
                snapshots
            );
        } else {
            assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
        }
    }
}

#[test]
fn push_keeps_array_unchanged_when_result_cell_cannot_be_reserved() {
    for (length, expected_fault) in [(61, false), (62, true)] {
        let items = vec!["0"; length].join(",");
        let source = format!(
            "fn F(a:edit Array<I64>,n:move I64){{}}fn Main(){{var a=[{items}];F(edit a,1);}}"
        );
        let program = replace_call(&source, Intrinsic::Push);
        verify::verify_program(&program).unwrap();
        let (result, snapshots) =
            Execution::run_with_fault_snapshots_cells64(&program, Default::default());
        if expected_fault {
            assert_eq!(result.outcome, Outcome::SimulatorFault);
            assert_eq!(result.diagnostics[0].code, "R0002");
            let expected = format!("[{}]", vec!["0"; length].join(", "));
            assert!(
                snapshots
                    .iter()
                    .flatten()
                    .any(|value| value.as_deref() == Some(expected.as_str()))
            );
        } else {
            assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
        }
    }
}

#[test]
fn join_failure_preserves_both_input_arrays() {
    let zeros = vec!["0"; 30].join(",");
    let ones = vec!["1"; 30].join(",");
    let source = format!(
        "fn F(a:view Array<I64>,b:view Array<I64>)->Array<I64>{{return [];}}fn Main(){{let a=[{zeros}];let b=[{ones}];F(a,b);}}"
    );
    let program = replace_call(&source, Intrinsic::JoinArrays);
    verify::verify_program(&program).unwrap();
    let (result, snapshots) =
        Execution::run_with_fault_snapshots_cells64(&program, Default::default());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0002");
    let expected_a = format!("[{}]", vec!["0"; 30].join(", "));
    let expected_b = format!("[{}]", vec!["1"; 30].join(", "));
    let values: Vec<_> = snapshots
        .iter()
        .flatten()
        .filter_map(Option::as_deref)
        .collect();
    assert!(values.contains(&expected_a.as_str()));
    assert!(values.contains(&expected_b.as_str()));
}
