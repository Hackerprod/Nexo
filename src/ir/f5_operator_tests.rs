use super::{
    compile_scalar,
    model::{self, Binary, Op},
    verify,
};
use crate::{
    diagnostic::Operation,
    interp::{Outcome, run_scalar},
};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn logical_short_circuit_uses_verified_cfg_and_charges_only_the_taken_rhs() {
    for (left, and, expected) in [
        (false, true, 7),
        (true, true, 9),
        (true, false, 7),
        (false, false, 9),
    ] {
        let source = "fn Main(){let b=if true then true else false;}";
        let mut program = compile_scalar("case.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        function.block.instructions[0].op = Op::ConstBool(left);
        let skip = if and { 1 } else { 0 };
        let rhs = 1 - skip;
        function.blocks[skip].instructions[0].op = Op::ConstBool(!and);
        function.blocks[rhs].instructions[0].op = Op::ConstBool(!left);
        function.blocks[skip].instructions[0].charges.clear();
        function.content_hash = model::content_hash(function);
        verify::verify_program(&program).unwrap();
        let execution = run_scalar(&program, Default::default());
        assert_eq!(execution.outcome, Outcome::Unit);
        assert_eq!(execution.work_consumed, expected, "left={left} and={and}");
    }
}

#[test]
fn logical_join_returns_the_selected_boolean() {
    for (left, and, expected) in [
        (false, true, "false"),
        (true, true, "false"),
        (true, false, "true"),
        (false, false, "true"),
    ] {
        let mut program = compile_scalar(
            "case.n",
            "fn Main(){let b=if true then true else false;Print(b);}",
        )
        .unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        function.block.instructions[0].op = Op::ConstBool(left);
        let skip = if and { 1 } else { 0 };
        let rhs = 1 - skip;
        function.blocks[skip].instructions[0].op = Op::ConstBool(!and);
        function.blocks[rhs].instructions[0].op = Op::ConstBool(!left);
        function.content_hash = model::content_hash(function);
        verify::verify_program(&program).unwrap();
        let execution = run_scalar(&program, Default::default());
        assert_eq!(execution.outcome, Outcome::Unit);
        assert_eq!(execution.observations.stdout(), vec![expected]);
    }
}

#[test]
fn logical_rhs_resources_follow_only_the_selected_cfg_edge() {
    let source = "fn Main(){let b=if true then {let r=MockOpen(1);value:true;}else{value:false;};}";
    let mut program = compile_scalar("case.n", source).unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    function.block.instructions[0].op = Op::ConstBool(false);
    function.content_hash = model::content_hash(function);
    verify::verify_program(&program).unwrap();
    let skipped = run_scalar(&program, Default::default());
    assert_eq!(skipped.outcome, Outcome::Unit);
    assert!(skipped.observations.events().is_empty());
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    function.block.instructions[0].op = Op::ConstBool(true);
    function.content_hash = model::content_hash(function);
    verify::verify_program(&program).unwrap();
    let taken = run_scalar(&program, Default::default());
    assert_eq!(taken.outcome, Outcome::Unit);
    assert_eq!(taken.observations.events(), vec!["open:0:1", "close:0:1"]);
}

#[test]
fn logical_rhs_static_io_effect_is_retained_on_an_omitted_edge() {
    let source =
        "fn Main() with no_io {let b=if false then {Print(9);value:true;}else{value:false;};}";
    let result = compile_scalar("case.n", source);
    match result {
        Err(super::CompileError::Rejected(errors)) => {
            assert!(errors.iter().any(|d| d.code == "NX-E0415"))
        }
        other => panic!("expected static IO rejection, got {other:?}"),
    }
}

#[test]
fn boolean_not_is_typed_and_executed_by_the_verified_ir() {
    let mut program = compile_scalar("case.n", "fn Main(){Print(true==false);}").unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let index = function
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::Binary { .. }))
        .unwrap();
    let Op::Binary { left, right, .. } = function.block.instructions[index].op else {
        unreachable!()
    };
    let mut drop_right = function.block.instructions[index].clone();
    drop_right.op = Op::DropValue(right);
    drop_right.output = None;
    drop_right.ty = None;
    drop_right.charges.clear();
    function.block.instructions.insert(index, drop_right);
    function.block.instructions[index + 1].op = Op::Unary {
        op: model::Unary::Not,
        operand: left,
    };
    for (at, ins) in function.block.instructions.iter_mut().enumerate() {
        ins.origin = model::OriginId(at);
    }
    for exit in &mut function.exit_info {
        if exit.block == model::BlockId(0) && exit.index >= index {
            exit.index += 1;
        }
    }
    function.content_hash = model::content_hash(function);
    verify::verify_program(&program).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.stdout(), vec!["false"]);
}

#[test]
fn boolean_not_rejects_integer_operand_in_verified_ir() {
    let mut program = compile_scalar("case.n", "fn Main(){Print(1+2);}").unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let index = function
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::Binary { .. }))
        .unwrap();
    let Op::Binary { left, right, .. } = function.block.instructions[index].op else {
        unreachable!()
    };
    let mut drop_right = function.block.instructions[index].clone();
    drop_right.op = Op::DropValue(right);
    drop_right.output = None;
    drop_right.ty = None;
    drop_right.charges.clear();
    function.block.instructions.insert(index, drop_right);
    function.block.instructions[index + 1].op = Op::Unary {
        op: model::Unary::Not,
        operand: left,
    };
    for (at, ins) in function.block.instructions.iter_mut().enumerate() {
        ins.origin = model::OriginId(at);
    }
    for exit in &mut function.exit_info {
        if exit.block == model::BlockId(0) && exit.index >= index {
            exit.index += 1;
        }
    }
    function.content_hash = model::content_hash(function);
    assert_eq!(verify::verify_program(&program), Err("unary domain"));
}

fn run_binary(source: &str, op: Binary) -> crate::interp::Execution {
    let mut program = compile_scalar("case.n", source).unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let mut replaced = false;
    for instruction in function.block.instructions.iter_mut().chain(
        function
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions),
    ) {
        if let Op::Binary { op: old, .. } = &mut instruction.op {
            *old = op;
            replaced = true;
        }
    }
    assert!(replaced, "IR fixture must exercise the operator");
    function.content_hash = model::content_hash(function);
    verify::verify_program(&program).unwrap();
    run_scalar(&program, Default::default())
}

#[test]
fn checked_negation_uses_verified_ir_and_faults_on_minimum() {
    for (source, expected, fault) in [
        ("fn Main(){Print(0-7);}", Some("-7"), None),
        (
            "fn Main(){Print(0--9223372036854775808);}",
            None,
            Some("R0505"),
        ),
    ] {
        let mut program = compile_scalar("case.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        let index = function
            .block
            .instructions
            .iter()
            .position(|i| matches!(i.op, Op::Binary { .. }))
            .unwrap();
        let Op::Binary { left, right, .. } = function.block.instructions[index].op else {
            unreachable!()
        };
        let mut drop_left = function.block.instructions[index].clone();
        drop_left.op = Op::DropValue(left);
        drop_left.output = None;
        drop_left.ty = None;
        drop_left.charges.clear();
        function.block.instructions.insert(index, drop_left);
        function.block.instructions[index + 1].op = Op::Unary {
            op: model::Unary::Neg,
            operand: right,
        };
        for (position, instruction) in function.block.instructions.iter_mut().enumerate() {
            instruction.origin = model::OriginId(position);
        }
        for exit in &mut function.exit_info {
            if exit.block == model::BlockId(0) && exit.index >= index {
                exit.index += 1;
            }
        }
        function.content_hash = model::content_hash(function);
        verify::verify_program(&program).unwrap();
        let execution = run_scalar(&program, Default::default());
        if let Some(code) = fault {
            assert_eq!(execution.outcome, Outcome::SimulatorFault);
            assert_eq!(execution.diagnostics[0].code, code);
            assert_eq!(execution.diagnostics[0].operation, Operation::ValueNeg);
        } else {
            assert_eq!(execution.outcome, Outcome::Unit);
            assert_eq!(execution.observations.stdout(), vec![expected.unwrap()]);
        }
    }
}

#[test]
fn checked_integer_operations_use_verified_ir_and_runtime_codes() {
    for (source, op, expected) in [
        ("fn Main(){Print(7+3);}", Binary::Mul, "21"),
        ("fn Main(){Print(7+3);}", Binary::Div, "2"),
        ("fn Main(){Print(7+3);}", Binary::Rem, "1"),
        ("fn Main(){Print(-7+3);}", Binary::Div, "-2"),
        ("fn Main(){Print(-7+3);}", Binary::Rem, "-1"),
        ("fn Main(){Print(7+-3);}", Binary::Div, "-2"),
        ("fn Main(){Print(7+-3);}", Binary::Rem, "1"),
        ("fn Main(){Print(-7+-3);}", Binary::Div, "2"),
        ("fn Main(){Print(-7+-3);}", Binary::Rem, "-1"),
        (
            "fn Main(){Print(-9223372036854775808+-1);}",
            Binary::Rem,
            "0",
        ),
    ] {
        let execution = run_binary(source, op);
        assert_eq!(execution.outcome, Outcome::Unit, "{source}");
        assert_eq!(execution.observations.stdout(), vec![expected], "{source}");
    }
    for (source, op, code, operation) in [
        (
            "fn Main(){Print(9223372036854775807+2);}",
            Binary::Mul,
            "R0501",
            Operation::ValueMul,
        ),
        (
            "fn Main(){Print(7+0);}",
            Binary::Div,
            "R0502",
            Operation::ValueDiv,
        ),
        (
            "fn Main(){Print(-9223372036854775808+-1);}",
            Binary::Div,
            "R0503",
            Operation::ValueDiv,
        ),
        (
            "fn Main(){Print(7+0);}",
            Binary::Rem,
            "R0504",
            Operation::ValueRem,
        ),
    ] {
        let execution = run_binary(source, op);
        assert_eq!(execution.outcome, Outcome::SimulatorFault, "{source}");
        assert_eq!(execution.diagnostics[0].code, code, "{source}");
        assert_eq!(execution.diagnostics[0].operation, operation, "{source}");
        assert!(execution.diagnostics[0].data.is_empty(), "{source}");
        execution.diagnostics[0].validate("case.n", source).unwrap();
        assert!(execution.observations.stdout().is_empty(), "{source}");
    }
}

#[test]
fn integer_results_match_separate_arbitrary_precision_oracle() {
    // Python integers are unbounded. The oracle uses sign and magnitude for
    // truncating division; Python's floor division on signed inputs differs.
    const ORACLE: &str = r#"
import sys
lo, hi = -(1 << 63), (1 << 63) - 1
for row in sys.stdin:
    op, left, right = row.split()
    a, b = int(left), int(right)
    if op == '*':
        value = a * b
        result = str(value) if lo <= value <= hi else 'R0501'
    elif op == '/' and b == 0:
        result = 'R0502'
    elif op == '%' and b == 0:
        result = 'R0504'
    else:
        quotient = (abs(a) // abs(b)) * (1 if (a < 0) == (b < 0) else -1)
        value = quotient if op == '/' else a - quotient * b
        result = str(value) if lo <= value <= hi else 'R0503'
    print(result)
"#;
    let inputs = [i64::MIN, -7, -3, -1, 0, 1, 2, 3, 7, i64::MAX];
    let cases: Vec<_> = [Binary::Mul, Binary::Div, Binary::Rem]
        .into_iter()
        .flat_map(|op| {
            inputs
                .into_iter()
                .flat_map(move |a| inputs.into_iter().map(move |b| (op, a, b)))
        })
        .collect();
    let mut child = Command::new("python")
        .args(["-c", ORACLE])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let input = child.stdin.as_mut().unwrap();
        for (op, a, b) in &cases {
            let symbol = match op {
                Binary::Mul => '*',
                Binary::Div => '/',
                Binary::Rem => '%',
                _ => unreachable!(),
            };
            writeln!(input, "{symbol} {a} {b}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let expected = String::from_utf8(output.stdout).unwrap();
    for ((op, a, b), expected) in cases.into_iter().zip(expected.lines()) {
        let source = format!("fn Main(){{Print({a}+{b});}}");
        let execution = run_binary(&source, op);
        if expected.starts_with('R') {
            assert_eq!(execution.outcome, Outcome::SimulatorFault, "{source}");
            assert_eq!(execution.diagnostics[0].code, expected, "{source}");
        } else {
            assert_eq!(execution.outcome, Outcome::Unit, "{source}");
            assert_eq!(execution.observations.stdout(), vec![expected], "{source}");
        }
    }
}

#[test]
fn ordered_and_negated_comparisons_share_verified_ir() {
    for (source, op, expected) in [
        ("fn Main(){Print(7<3);}", Binary::LessEqual, "false"),
        ("fn Main(){Print(3<3);}", Binary::LessEqual, "true"),
        ("fn Main(){Print(7<3);}", Binary::Greater, "true"),
        ("fn Main(){Print(3<3);}", Binary::Greater, "false"),
        ("fn Main(){Print(3<3);}", Binary::GreaterEqual, "true"),
        ("fn Main(){Print(3<3);}", Binary::NotEqual, "false"),
        ("fn Main(){Print(7<3);}", Binary::NotEqual, "true"),
    ] {
        let execution = run_binary(source, op);
        assert_eq!(execution.outcome, Outcome::Unit, "{source}");
        assert_eq!(execution.observations.stdout(), vec![expected], "{source}");
    }
}

#[test]
fn new_binary_work_matches_the_existing_operand_and_result_charges() {
    for (source, old, new) in [
        ("fn Main(){Print(7+3);}", Binary::Add, Binary::Mul),
        ("fn Main(){Print(7+3);}", Binary::Add, Binary::Div),
        ("fn Main(){Print(7+3);}", Binary::Add, Binary::Rem),
        ("fn Main(){Print(7<3);}", Binary::Less, Binary::Greater),
        ("fn Main(){Print(7<3);}", Binary::Less, Binary::LessEqual),
    ] {
        assert_eq!(
            run_binary(source, old).work_consumed,
            run_binary(source, new).work_consumed
        );
    }
    assert_eq!(
        super::work::WorkPlanner::units(super::work::WorkEvent::OperatorExpression),
        1
    );
}

#[test]
fn verifier_rejects_integer_operator_on_boolean_operands() {
    let mut program = compile_scalar("case.n", "fn Main(){Print(true==false);}").unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let instruction = function
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, Op::Binary { .. }))
        .unwrap();
    let Op::Binary { op, .. } = &mut instruction.op else {
        unreachable!()
    };
    *op = Binary::Div;
    function.content_hash = model::content_hash(function);
    assert_eq!(verify::verify_program(&program), Err("binary domain"));
}
