//! The lowering of each construct: the flat evaluator gives the value and the diagnostics that
//! the evaluator of closures gives.

use super::{agree, refused};

#[test]
fn literals() {
  for text in [
    "1",
    "1.50",
    "1e3",
    "\"text\"",
    "true",
    "false",
    "null",
    "@\"2024-01-31\"",
    "@\"P1Y\"",
    "@\"10:00:00.1234567891\"",
    "@\"x\"",
  ] {
    agree(text);
  }
}

#[test]
fn a_number_without_a_value_is_an_error_at_every_evaluation() {
  use crate::Dialect;
  use crate::flat::lower::lower;
  use crate::flat::resolve::resolve;
  use rheon_feel::FeelScope;
  use rheon_feel_parser::AstNode;
  let node = AstNode::Numeric("1x".into(), "".into(), '+', "".into());
  let old = crate::builders::build_evaluator_with_dialect(&node, Dialect::Feel);
  let program = lower(&resolve(&node), Dialect::Feel).unwrap_or_else(|_| panic!("refused"));
  let scope = FeelScope::default();
  let flat: rheon_feel::Evaluator = Box::new(move |scope: &FeelScope| program.run(scope));
  assert_eq!(super::outcome(old, &scope), super::outcome(flat, &scope));
}

#[test]
fn constructs_not_lowered_yet_are_refused() {
  for text in ["x", "1 + 2", "[1]", "{a: 1}", "abs(1)"] {
    refused(text);
  }
}
