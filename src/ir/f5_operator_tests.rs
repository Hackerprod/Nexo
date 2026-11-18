use super::{
    compile_scalar,
    model::{self, Binary, Op},
    verify,
};

fn compound_scalar_ir(source: &str, operator: Binary) -> model::Program {
    compound_scalar_ir_in(source, operator, "Main")
}

fn compound_scalar_ir_in(source: &str, operator: Binary, name: &str) -> model::Program {
    let mut program = compile_scalar("case.n", source).unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == name)
        .unwrap();
    let instructions = &mut function.block.instructions;
    let position = instructions
        .iter()
        .position(|ins| matches!(ins.op, Op::BorrowValue(model::LocalId(0))))
        .unwrap();
    let mut begin = instructions[position].clone();
    let place = crate::diagnostic::Span {
        file: "case.n".into(),
        start: begin.span.start - 2,
        end: begin.span.end - 2,
    };
    let loan = model::LoanId(place.start as usize);
    begin.op = Op::BeginLoan {
        root: model::LocalId(0),
        indexes: vec![],
        site: model::IndexSite {
            context: place.clone(),
            selectors: vec![],
        },
        loan,
        origin_span: place.clone(),
        permission: model::LoanPermission::Inout,
        parent: None,
    };
    begin.span = place.clone();
    begin.charges.truncate(1);
    instructions[position] = begin;
    let mut load = instructions[position].clone();
    load.op = Op::CompoundLoad {
        loan: model::TempId(1),
    };
    load.output = Some(model::TempId(2));
    load.charges.clear();
    instructions.insert(position + 1, load);
    instructions[position + 2].output = Some(model::TempId(3));
    let binary = position + 3;
    assert!(matches!(instructions[binary].op, Op::Binary { .. }));
    instructions[binary].op = Op::CompoundStore {
        loan: model::TempId(1),
        old: model::TempId(2),
        rhs: model::TempId(3),
        op: operator,
    };
    instructions[binary].output = None;
    instructions[binary].ty = None;
    instructions[binary].access = model::Access::Write;
    instructions[binary].charges.clear();
    instructions[binary + 1].op = Op::EndLoan(loan);
    instructions[binary + 1].access = model::Access::Own;
    instructions[binary + 1].charges.clear();
    for (at, ins) in instructions.iter_mut().enumerate() {
        ins.origin = model::OriginId(at);
    }
    for exit in &mut function.exit_info {
        if exit.block == model::BlockId(0) && exit.index > position {
            exit.index += 1;
        }
    }
    function.content_hash = model::content_hash(function);
    program
}

#[test]
fn compound_scalar_place_has_one_exclusive_loan_and_t08_work() {
    let program = compound_scalar_ir("fn Main(){var x=1;x=x+2;}", Binary::Add);
    verify::verify_program(&program).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.work_consumed, 11);
}

#[test]
fn compound_integer_forms_reuse_checked_arithmetic_and_store_once() {
    for (op, expected) in [
        (Binary::Add, "10"),
        (Binary::Sub, "4"),
        (Binary::Mul, "21"),
        (Binary::Div, "2"),
        (Binary::Rem, "1"),
    ] {
        let program = compound_scalar_ir("fn Main(){var x=7;x=x+3;Print(x);}", op);
        verify::verify_program(&program).unwrap();
        let execution = run_scalar(&program, Default::default());
        assert_eq!(execution.outcome, Outcome::Unit, "{op:?}");
        assert_eq!(execution.observations.stdout(), vec![expected], "{op:?}");
    }
    for (source, op, code) in [
        (
            "fn Main(){var x=9223372036854775807;x=x+2;}",
            Binary::Mul,
            "R0501",
        ),
        ("fn Main(){var x=7;x=x+0;}", Binary::Div, "R0502"),
        (
            "fn Main(){var x=-9223372036854775808;x=x+-1;}",
            Binary::Div,
            "R0503",
        ),
        ("fn Main(){var x=7;x=x+0;}", Binary::Rem, "R0504"),
    ] {
        let program = compound_scalar_ir(source, op);
        verify::verify_program(&program).unwrap();
        let execution = run_scalar(&program, Default::default());
        assert_eq!(execution.outcome, Outcome::SimulatorFault, "{source}");
        assert_eq!(execution.diagnostics[0].code, code, "{source}");
    }
}

#[test]
fn compound_loan_rejects_rhs_read_of_the_reserved_root() {
    let mut program = compound_scalar_ir("fn Main(){var x=1;x=x+2;}", Binary::Add);
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let rhs = function
        .block
        .instructions
        .iter_mut()
        .find(|ins| ins.output == Some(model::TempId(3)))
        .unwrap();
    rhs.op = Op::CopyValue(model::LocalId(0));
    rhs.access = model::Access::Read;
    function.content_hash = model::content_hash(function);
    assert_eq!(
        verify::verify_program(&program),
        Err("read during exclusive loan")
    );
}

#[test]
fn compound_verifier_requires_load_provenance_and_immediate_release() {
    let source = "fn Main(){var x=1;x=x+2;}";
    let mut program = compound_scalar_ir(source, Binary::Add);
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let store = function
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::CompoundStore { .. }))
        .unwrap();
    let Op::CompoundStore { old, .. } = &mut store.op else {
        unreachable!()
    };
    *old = model::TempId(3);
    function.content_hash = model::content_hash(function);
    assert_eq!(
        verify::verify_program(&program),
        Err("compound place sequence")
    );

    let mut program = compound_scalar_ir(source, Binary::Add);
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let end = function
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::EndLoan(_)))
        .unwrap();
    end.op = Op::DropLocal(model::LocalId(0));
    function.content_hash = model::content_hash(function);
    assert_eq!(verify::verify_program(&program), Err("compound loan end"));

    let mut program = compound_scalar_ir(source, Binary::Add);
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let store = function
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::CompoundStore { .. }))
        .unwrap();
    let Op::CompoundStore { rhs, .. } = &mut store.op else {
        unreachable!()
    };
    *rhs = model::TempId(0);
    function.content_hash = model::content_hash(function);
    assert_eq!(
        verify::verify_program(&program),
        Err("compound evaluation order")
    );

    let mut program = compound_scalar_ir(source, Binary::Add);
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let store = function
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::CompoundStore { .. }))
        .unwrap();
    let Op::CompoundStore { op, .. } = &mut store.op else {
        unreachable!()
    };
    *op = Binary::And;
    function.content_hash = model::content_hash(function);
    assert_eq!(
        verify::verify_program(&program),
        Err("compound place sequence")
    );
}

#[test]
fn cfg_compound_store_requires_load_before_rhs_and_preserves_place_sequence() {
    let source = "fn Main(){var x=1;x=x+2;if true{}Print(x);}";
    let program = compound_scalar_ir(source, Binary::Add);
    assert!(
        !program
            .functions
            .entries
            .iter()
            .find(|f| f.name == "Main")
            .unwrap()
            .blocks
            .is_empty()
    );
    verify::verify_program(&program).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.stdout(), vec!["3"]);

    for invalid in 0..5 {
        let mut forged = program.clone();
        let function = forged
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        let instructions = &mut function.block.instructions;
        let begin = instructions
            .iter()
            .position(|i| matches!(i.op, Op::BeginLoan { .. }))
            .unwrap();
        let load = instructions
            .iter()
            .position(|i| matches!(i.op, Op::CompoundLoad { .. }))
            .unwrap();
        let store = instructions
            .iter()
            .position(|i| matches!(i.op, Op::CompoundStore { .. }))
            .unwrap();
        let end = instructions
            .iter()
            .position(|i| matches!(i.op, Op::EndLoan(_)))
            .unwrap();
        match invalid {
            0 => {
                let (left, right) = instructions.split_at_mut(end);
                std::mem::swap(&mut left[store].op, &mut right[0].op);
            }
            1 => {
                let Op::CompoundStore { op, .. } = &mut instructions[store].op else {
                    unreachable!()
                };
                *op = Binary::Less;
            }
            2 => {
                let Op::CompoundStore { old, rhs, .. } = &mut instructions[store].op else {
                    unreachable!()
                };
                std::mem::swap(old, rhs);
            }
            3 => {
                let Op::BeginLoan { permission, .. } = &mut instructions[begin].op else {
                    unreachable!()
                };
                *permission = model::LoanPermission::Read;
            }
            4 => {
                let (left, right) = instructions.split_at_mut(load);
                std::mem::swap(&mut left[begin].op, &mut right[0].op);
            }
            _ => unreachable!(),
        }
        function.content_hash = model::content_hash(function);
        assert!(
            verify::verify_program(&forged).is_err(),
            "invalid={invalid}"
        );
    }
}

#[test]
fn repeated_cfg_compound_store_releases_replaced_cell() {
    let source = format!("fn F(){{var x=1;x=x+2;}}fn Main(){{{}}}", "F();".repeat(30));
    let normal = compile_scalar("case.n", &source).unwrap();
    let compound = compound_scalar_ir_in(&source, Binary::Add, "F");
    verify::verify_program(&compound).unwrap();
    let profile = crate::interp::ReducedExecutionProfile::Cells64;
    let expected = crate::interp::run_with_limits(&normal, Default::default(), profile);
    let actual = crate::interp::run_with_limits(&compound, Default::default(), profile);
    assert_eq!(expected.outcome, Outcome::Unit);
    assert_eq!(actual.outcome, Outcome::Unit);
    assert_eq!(actual.disposals, expected.disposals);
}

fn compound_index_ir(source: &str, operator: Binary) -> model::Program {
    let mut program = compile_scalar("case.n", source).unwrap();
    let function = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Main")
        .unwrap();
    let instructions = &mut function.block.instructions;
    let index_call = instructions
        .iter()
        .position(|ins| matches!(&ins.op, Op::CallDirect { function, .. } if function.0 == "Index"))
        .unwrap();
    let rhs_call = index_call + 1;
    let Op::WriteIndex { site, .. } = &instructions[rhs_call + 1].op else {
        unreachable!()
    };
    let site = site.clone();
    let loan = model::LoanId(site.context.start as usize);
    let mut begin = instructions[rhs_call + 1].clone();
    begin.op = Op::BeginLoan {
        root: model::LocalId(0),
        indexes: vec![model::TempId(2)],
        site: site.clone(),
        loan,
        origin_span: site.context.clone(),
        permission: model::LoanPermission::Inout,
        parent: None,
    };
    begin.output = Some(model::TempId(3));
    begin.ty = Some(model::Ty::I64);
    begin.span = site.context.clone();
    begin.access = model::Access::Read;
    let mut load = begin.clone();
    load.op = Op::CompoundLoad {
        loan: model::TempId(3),
    };
    load.output = Some(model::TempId(4));
    load.charges.clear();
    let mut rhs = instructions[rhs_call].clone();
    rhs.output = Some(model::TempId(5));
    let mut store = instructions[rhs_call + 1].clone();
    store.op = Op::CompoundStore {
        loan: model::TempId(3),
        old: model::TempId(4),
        rhs: model::TempId(5),
        op: operator,
    };
    let mut end = store.clone();
    end.op = Op::EndLoan(loan);
    end.access = model::Access::Own;
    instructions.splice(rhs_call..rhs_call + 2, [begin, load, rhs, store, end]);
    for ins in instructions.iter_mut().skip(rhs_call + 5) {
        match &mut ins.op {
            Op::BorrowValue(_) => ins.output = Some(model::TempId(6)),
            Op::CallIntrinsic { args, .. } => {
                args[0] = model::TempId(6);
                ins.output = Some(model::TempId(7));
            }
            Op::DropValue(id) => *id = model::TempId(7),
            Op::ConstUnit => ins.output = Some(model::TempId(8)),
            _ => {}
        }
    }
    if let model::Terminator::Return { value, .. } = &mut function.block.terminator {
        *value = Some(model::TempId(8));
    }
    for (at, ins) in instructions.iter_mut().enumerate() {
        ins.origin = model::OriginId(at);
    }
    for exit in &mut function.exit_info {
        if exit.block == model::BlockId(0) && exit.index > rhs_call {
            exit.index += 3;
        }
    }
    function.content_hash = model::content_hash(function);
    program
}

#[test]
fn indexed_compound_place_prepares_index_before_rhs_and_stores_once() {
    let source = "fn Index()->I64{Print(1);return 0;}fn Rhs()->I64{Print(2);return 3;}fn Main(){var a=[4];a[Index()]=Rhs();Print(a);}";
    let program = compound_index_ir(source, Binary::Mul);
    verify::verify_program(&program).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.stdout(), vec!["1", "2", "[12]"]);
    assert!(execution.observations.events().is_empty());
}

#[test]
fn indexed_compound_checks_bounds_before_rhs_and_preserves_fault_priority() {
    let source = "fn Index()->I64{Print(1);return 2;}fn Rhs()->I64{Print(2);return 3;}fn Main(){var a=[4];a[Index()]=Rhs();Print(a);}";
    let program = compound_index_ir(source, Binary::Mul);
    verify::verify_program(&program).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert_eq!(execution.diagnostics[0].code, "R0102");
    assert_eq!(execution.observations.stdout(), vec!["1"]);

    let source = "fn Index()->I64{Print(1);return 0;}fn Rhs()->I64{Print(2);return 7+0;}fn Main(){var a=[4];a[Index()]=Rhs();Print(a);}";
    let mut program = compound_index_ir(source, Binary::Mul);
    let rhs = program
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "Rhs")
        .unwrap();
    let ins = rhs
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::Binary { .. }))
        .unwrap();
    let Op::Binary { op, .. } = &mut ins.op else {
        unreachable!()
    };
    *op = Binary::Div;
    rhs.content_hash = model::content_hash(rhs);
    verify::verify_program(&program).unwrap();
    let (execution, snapshots) =
        crate::interp::Execution::run_with_fault_snapshots(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert_eq!(execution.diagnostics[0].code, "R0502");
    assert_eq!(execution.observations.stdout(), vec!["1", "2"]);
    assert!(
        snapshots
            .iter()
            .any(|frame| frame.iter().any(|local| local.as_deref() == Some("[4]")))
    );
}
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
