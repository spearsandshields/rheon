//! DMN 1.6 §10.3.2.3.1: "There is no distinction between -0 and 0." No operation gives a zero
//! with a minus sign, so a zero never prints or debugs with one.

mod common;

use rheon_feel_number::{FeelNumber, RoundingMode};

fn parsed(s: &str) -> FeelNumber {
  s.parse::<FeelNumber>().unwrap()
}

#[track_caller]
fn assert_zero(expected_debug: &str, n: FeelNumber) {
  assert_eq!(expected_debug, format!("{n:?}"));
  assert!(!n.to_string().starts_with('-'), "{n}");
}

#[test]
fn test_minus_zero_001() {
  eqs!("0", FeelNumber::new(-0, 0).to_string());
}

#[test]
fn test_minus_zero_parsed() {
  eqs!("0", parsed("-0"));
  assert_zero("+0E+0", parsed("-0"));
  assert_zero("+0E-2", parsed("-0.00"));
  assert_zero("+0E-6176", parsed("-0E-7000"));
}

#[test]
fn test_minus_zero_negated() {
  eqs!("0", -FeelNumber::zero());
  assert_zero("+0E+0", -FeelNumber::zero());
  assert_zero("+0E-3", -parsed("0.000"));
}

#[test]
fn test_minus_zero_arithmetic() {
  eqs!("0", num!(0) * parsed("-5"));
  assert_zero("+0E+0", num!(0) * parsed("-5"));
  assert_zero("+0E+0", num!(0) / parsed("-5"));
  assert_zero("+0E+0", parsed("-1") * num!(0));
  assert_zero("+0E+0", num!(0) - num!(0) + parsed("-0"));
  assert_zero("+0E+0", parsed("-1") + num!(1) - num!(0));
  assert_zero("+0E-6176", parsed("-1E-6176") / num!(10));
}

#[test]
fn test_minus_zero_sqrt() {
  assert_zero("+0E-1", parsed("-0.0").sqrt().unwrap());
}

#[test]
fn test_minus_zero_rounding() {
  assert_zero("+0E+0", parsed("-0.4").round_to(0, RoundingMode::HalfUp).unwrap());
  assert_zero("+0E+0", parsed("-0.5").round_to(0, RoundingMode::Down).unwrap());
  assert_zero("+0E+0", parsed("-0.5").round_to(0, RoundingMode::Ceiling).unwrap());
  assert_zero("+0E+0", parsed("-0.0").round_to(0, RoundingMode::Floor).unwrap());
  assert_zero("+0E+0", parsed("-0.5").trunc());
  assert_zero("+0E+0", parsed("0.05").round_to(0, RoundingMode::HalfDown).unwrap());
}

#[test]
fn test_minus_zero_powers() {
  assert_zero("+0E+0", parsed("-0").pow(&num!(3)).unwrap());
  assert_zero("+0E+0", parsed("-0").square().unwrap());
}
