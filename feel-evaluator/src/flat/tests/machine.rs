//! The program format and the run loop, on programs built by hand.

use crate::flat::program::{Op, Opnd, Program, ProgramBuilder, VerifyError};
use rheon_feel::diagnostics;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, value_null};

fn program(code: Vec<Op>, consts: Vec<Value>, messages: Vec<&str>, slots: u32) -> Program {
  Program {
    root_end: code.len() as u32,
    code,
    consts,
    messages: messages.into_iter().map(Into::into).collect(),
    names: vec![],
    name_lists: vec![],
    calls: vec![],
    searches: vec![],
    loops: vec![],
    filters: vec![],
    probes: vec![],
    closures: vec![],
    protos: vec![],
    layouts: vec![],
    regions: vec![],
    slots,
    dialect: crate::Dialect::Feel,
  }
}

#[test]
fn operands_keep_their_kind_and_index() {
  use crate::flat::program::Operand;
  assert_eq!(Operand::Take(7), Opnd::take(7).decode());
  assert_eq!(Operand::Const((1 << 30) - 1), Opnd::konst((1 << 30) - 1).decode());
}

#[test]
fn verify_rejects_indices_out_of_range() {
  let at = |at, what| Err(VerifyError { at, what });
  assert_eq!(
    at(0, "slot out of range"),
    program(vec![Op::Fail { dst: 1, msg: 0 }, Op::Return { src: Opnd::take(0) }], vec![], vec!["e"], 1).verify()
  );
  assert_eq!(
    at(0, "message out of range"),
    program(vec![Op::Warn { msg: 1 }, Op::Return { src: Opnd::konst(0) }], vec![Value::Null(None)], vec!["w"], 0).verify()
  );
  assert_eq!(
    at(0, "constant out of range"),
    program(vec![Op::Return { src: Opnd::konst(1) }], vec![Value::Null(None)], vec![], 0).verify()
  );
  assert_eq!(
    at(1, "the code of a body does not end with a return"),
    program(vec![Op::Warn { msg: 0 }], vec![], vec!["w"], 0).verify()
  );
}

#[test]
fn verify_rejects_unstructured_code() {
  use crate::flat::program::ListKind;
  let at = |at, what| Err(VerifyError { at, what });
  let null = || vec![Value::Null(None)];
  let ret = Op::Return { src: Opnd::konst(0) };
  assert_eq!(at(0, "jump out of range"), program(vec![Op::Jump { to: 0 }, ret], null(), vec![], 0).verify());
  assert_eq!(at(0, "jump out of range"), program(vec![Op::Jump { to: 2 }, ret], null(), vec![], 0).verify());
  assert_eq!(
    at(0, "no list is open"),
    program(vec![Op::ListPush { src: Opnd::konst(0) }, ret], null(), vec![], 0).verify()
  );
  assert_eq!(
    at(1, "a list, an invocation, a fold or a walk is open at the return"),
    program(vec![Op::ListNew { len: 0 }, ret], null(), vec![], 0).verify()
  );
  let across = vec![Op::Jump { to: 2 }, Op::ListNew { len: 0 }, Op::ListEnd { kind: ListKind::List, dst: 0 }, ret];
  assert_eq!(at(0, "jump across a list, an invocation or a fold"), program(across, null(), vec![], 1).verify());
  let leave = Op::Leave { dst: 0 };
  assert_eq!(at(0, "no invocation is open"), program(vec![leave, ret], null(), vec![], 1).verify());
  let past = vec![Op::Enter { dst: 0, skip: 2 }, leave, ret];
  assert_eq!(Ok(()), program(past, null(), vec![], 1).verify());
  let into = vec![Op::Enter { dst: 0, skip: 1 }, leave, ret];
  assert_eq!(at(0, "jump across a list, an invocation or a fold"), program(into, null(), vec![], 1).verify());
}

#[test]
fn a_run_records_errors_and_warnings_in_order() {
  let mut builder = ProgramBuilder::default();
  let warning = builder.message("w");
  let error = builder.message("e");
  builder.emit(Op::Warn { msg: warning });
  builder.emit(Op::Fail { dst: 0, msg: error });
  builder.emit(Op::Return { src: Opnd::take(0) });
  let end = builder.here();
  let program = builder.finish(1, end, crate::Dialect::Feel, vec![], vec![]);
  let (value, recorded) = diagnostics::collect(|| program.run(&FeelScope::default()));
  let (expected, expected_recorded) = diagnostics::collect(|| {
    diagnostics::warn("w");
    value_null!("e")
  });
  assert_eq!((format!("{expected:?}"), expected_recorded), (format!("{value:?}"), recorded));
}
