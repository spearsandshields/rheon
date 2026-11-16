//! The program format and the run loop, on programs built by hand.

use crate::flat::program::{Op, Opnd, Program, ProgramBuilder, VerifyError};
use rheon_feel::diagnostics;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, value_null};

fn program(code: Vec<Op>, consts: Vec<Value>, messages: Vec<&str>, slots: u32) -> Program {
  Program {
    code: code.into(),
    consts: consts.into(),
    messages: messages.into_iter().map(Into::into).collect(),
    slots,
  }
}

#[test]
fn an_instruction_takes_at_most_16_bytes() {
  assert!(std::mem::size_of::<Op>() <= 16);
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
    at(1, "the code does not end with a return"),
    program(vec![Op::Warn { msg: 0 }], vec![], vec!["w"], 0).verify()
  );
}

#[test]
fn a_run_records_errors_and_warnings_in_order() {
  let mut builder = ProgramBuilder::default();
  let warning = builder.message("w");
  let error = builder.message("e");
  builder.emit(Op::Warn { msg: warning });
  builder.emit(Op::Fail { dst: 0, msg: error });
  builder.emit(Op::Return { src: Opnd::take(0) });
  let program = builder.finish(1);
  let (value, recorded) = diagnostics::collect(|| program.run(&FeelScope::default()));
  let (expected, expected_recorded) = diagnostics::collect(|| {
    diagnostics::warn("w");
    value_null!("e")
  });
  assert_eq!((format!("{expected:?}"), expected_recorded), (format!("{value:?}"), recorded));
}
