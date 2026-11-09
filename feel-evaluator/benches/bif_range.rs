#![feature(test)]

extern crate test;

use rheon_feel::{FeelScope, scope};
use rheon_feel_evaluator::prepare;
use test::Bencher;

#[bench]
fn feel_evaluator_bif_range_0001(b: &mut Bencher) {
  let scope = scope!();
  let input = r#"range("[1..100]")"#;
  let node = rheon_feel_parser::parse_textual_expression(&scope, input, false).unwrap();
  let evaluator = prepare(&node);
  assert_eq!("[1..100]", evaluator(&scope).to_string());
  b.iter(|| evaluator(&scope));
}
