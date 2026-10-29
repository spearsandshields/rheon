#![feature(test)]

extern crate test;

use flowstone_feel::{FeelScope, scope};
use flowstone_feel_parser::parse_expression;
use test::Bencher;

#[bench]
fn feel_parser_division_0001(b: &mut Bencher) {
  let scope = scope!();
  let input = r#"1/2"#;
  b.iter(|| parse_expression(&scope, input, false));
}
