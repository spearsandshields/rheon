//! The lowering of each construct: the flat evaluator gives the value and the diagnostics that
//! the evaluator of closures gives.

use super::{agree, agree_in, agree_node, refused, scope};
use crate::Dialect;
use rheon_feel::Name;
use rheon_feel_parser::AstNode;

/// [agree_in] FEEL with the input context `input`.
#[track_caller]
fn agree_with(input: &str, text: &str) {
  agree_in(Dialect::Feel, input, text);
}

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
  for text in ["1 + 2", "[1]", "{a: 1}", "abs(1)"] {
    refused(text);
  }
}

#[test]
fn names_read_the_input_then_built_ins() {
  agree("x");
  agree("abs");
  agree("not");
  agree_with("{x: 1}", "x");
  agree_with("{abs: 1}", "abs");
  agree_with("{?: 5}", "?");
  agree_with("{item: 5, partial: 6}", "partial");
}

#[test]
fn qualified_names_search_the_input() {
  let segments = |names: &[&str]| AstNode::QualifiedName(names.iter().map(|name| AstNode::QualifiedNameSegment(Name::from(*name))).collect());
  for input in ["", "{a: 1}", "{a: {b: 2}}", "{a: {c: 2}}", "{abs: 1}"] {
    let scope = scope(input);
    for names in [&["a"][..], &["a", "b"], &["a", "b", "c"], &["abs"], &[]] {
      agree_node(Dialect::Feel, &scope, &segments(names));
    }
  }
}

#[test]
fn paths_and_descendants() {
  agree_with("{x: {y: 1}}", "x.y");
  agree_with("{x: {y: 1}}", "x.z");
  agree_with("{xs: [{a: 1}, {b: 2}, 3]}", "xs.a");
  agree_with("{d: @\"2024-02-29\"}", "d.year");
  agree_with("{d: @\"2024-02-29\"}", "d.hours");
  agree_with("{s: \"text\"}", "s.length");
  agree_with("{x: {a: {b: {a: 2}}, c: [{a: 3}]}}", "x...a");
  agree_with("{x: 1}", "x...a");
  agree("(x).y");
  agree("@\"P1DT2H\".hours");
  // a path to a node that is no name is an error at every evaluation, and its base is not evaluated
  let scope = scope("");
  let node = AstNode::Path(Box::new(AstNode::Name(Name::from("x"))), Box::new(AstNode::Null));
  agree_node(Dialect::Feel, &scope, &node);
  let node = AstNode::Descendant(Box::new(AstNode::Name(Name::from("x"))), Box::new(AstNode::Null));
  agree_node(Dialect::Feel, &scope, &node);
}
