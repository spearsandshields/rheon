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
  for text in ["{a: 1}", "abs(1)", "1 + [{a: 1}]", "for i in [1] return i", "[1][1]"] {
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

/// Operands of every kind: numbers, strings, booleans, temporal values, null and a name that no
/// scope holds (an error).
const OPERANDS: [&str; 12] = [
  "1",
  "2.5",
  "0",
  "null",
  "\"a\"",
  "true",
  "@\"2024-01-31\"",
  "@\"P1D\"",
  "@\"P1Y\"",
  "@\"10:00:00\"",
  "@\"2024-01-31T10:00:00\"",
  "x",
];

#[test]
fn arithmetic_and_comparisons_feel_and_bfeel() {
  for operator in ["+", "-", "*", "/", "**", "=", "!=", "<", "<=", ">", ">="] {
    for a in OPERANDS {
      for b in OPERANDS {
        let text = format!("({a}) {operator} ({b})");
        agree_in(Dialect::Feel, "", &text);
        agree_in(Dialect::BFeel, "", &text);
      }
    }
  }
}

#[test]
fn negation() {
  for a in OPERANDS {
    agree_in(Dialect::Feel, "", &format!("-({a})"));
    agree_in(Dialect::BFeel, "", &format!("-({a})"));
  }
  agree("-(-1)");
  agree("1e6000 * 1e6000");
}

#[test]
fn between_evaluates_all_three_operands() {
  for a in ["1", "\"b\"", "null", "x", "@\"2024-01-31\""] {
    for low in ["0", "\"a\"", "null", "y"] {
      for high in ["2", "\"c\"", "null", "z"] {
        let text = format!("({a}) between ({low}) and ({high})");
        agree_in(Dialect::Feel, "", &text);
        agree_in(Dialect::BFeel, "", &text);
      }
    }
  }
}

#[test]
fn operators_nest() {
  agree_with("{a: 2, b: 3}", "(a + b) * (a - b) / b ** a");
  agree_with("{a: 2, b: 3}", "a + b = 5");
  agree("1 + 2 * 3 - 4 / 5 ** 6 < 7");
}

const LOGIC: [&str; 6] = ["true", "false", "null", "\"a\"", "1", "x"];

#[test]
fn conjunction_and_disjunction_evaluate_the_right_operand_when_it_decides() {
  for operator in ["and", "or"] {
    for a in LOGIC {
      for b in LOGIC {
        // `x` and `y` record their errors only when they are evaluated
        let b = if b == "x" { "y" } else { b };
        let text = format!("({a}) {operator} ({b})");
        agree_in(Dialect::Feel, "", &text);
        agree_in(Dialect::BFeel, "", &text);
      }
    }
  }
  agree("true and true and x or false");
}

#[test]
fn if_takes_the_then_branch_only_when_the_condition_is_true() {
  for condition in LOGIC {
    agree_in(Dialect::Feel, "", &format!("if ({condition}) then (y) else (z)"));
    agree_in(Dialect::BFeel, "", &format!("if ({condition}) then (1 + y) else \"a\""));
  }
  agree("if true then if false then 1 else 2 else 3");
}

#[test]
fn lists_and_ranges() {
  for text in [
    "[]",
    "[1, x, \"a\", [2, [3, y]], null]",
    "[1..2]",
    "(1..2]",
    "[\"a\"..\"z\")",
    "](x)..(y)[",
    "[1..\"a\"]",
    "[@\"2024-01-01\"..@\"2024-12-31\"]",
    "[1..2] = [1..2]",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
}

#[test]
fn in_and_not_in_with_every_right_hand_form() {
  for a in ["1", "5", "null", "\"b\"", "x", "@\"2024-02-01\""] {
    for right in [
      "[1, 2]",
      "[1..10]",
      "(> 3)",
      "(1, > 3, [4..5], null, x)",
      "(\"a\"..\"c\")",
      "(= 5)",
      "(!= 5)",
      "(<= 5, >= 9)",
    ] {
      let text = format!("({a}) in {right}");
      agree_in(Dialect::Feel, "", &text);
      agree_in(Dialect::BFeel, "", &text);
    }
  }
}

#[test]
fn not_in_evaluates_its_left_operand_twice() {
  for (input, left) in [("", "x"), ("{a: 1}", "a"), ("", "1")] {
    let scope = scope(input);
    let left = rheon_feel_parser::parse_expression(&scope, left, false).unwrap();
    let right = rheon_feel_parser::parse_unary_tests(&scope, "> 3, 1", false).unwrap();
    let node = AstNode::Out(Box::new(left), Box::new(right));
    agree_node(Dialect::Feel, &scope, &node);
    agree_node(Dialect::BFeel, &scope, &node);
  }
}

#[test]
fn unary_tests_read_the_input_value() {
  for input in ["{?: 5}", "{?: null}", "{?: \"b\"}", "{?: [1, 2]}", "{}"] {
    let scope = scope(input);
    for tests in ["> 3", "< 3, = 5", "not(5)", "-", "[1..10]", "? > 4", "\"a\", \"b\"", "null", "[1, 2]", "x"] {
      let tests = rheon_feel_parser::parse_unary_tests(&scope, tests, false).unwrap();
      let node = AstNode::In(Box::new(AstNode::Name(Name::from("?"))), Box::new(tests));
      agree_node(Dialect::Feel, &scope, &node);
      agree_node(Dialect::BFeel, &scope, &node);
    }
  }
}
