//! A number has one text: its value in plain digits, without trailing zeros after the decimal
//! point, and `0` for every zero (DMN 1.6 §10.3.2.3.1: "many different pairs have the same value").

mod common;

use rheon_feel_number::{FeelNumber, RoundingMode};

fn parsed(s: &str) -> FeelNumber {
  s.parse::<FeelNumber>().unwrap()
}

#[test]
fn test_to_string_001() {
  eqs!("49", num!(49));
}

#[test]
fn test_to_string_002() {
  eqs!("49", FeelNumber::new(490, 1).to_string());
}

#[test]
fn test_to_string_003() {
  eqs!("4900", FeelNumber::new(4900, 0).to_string());
}

#[test]
fn test_to_string_004() {
  eqs!("50", FeelNumber::new(50, 0).to_string());
}

#[test]
fn test_to_string_005() {
  eqs!("50", num!(50));
}

#[test]
fn test_to_string_006() {
  eqs!("50.5", FeelNumber::new(505, 1).to_string());
}

#[test]
fn test_to_string_007() {
  eqs!("50.5", FeelNumber::new(5050, 2).to_string());
}

#[test]
fn test_to_string_008() {
  // a precision is ignored: the text is the canonical text of the value
  assert_eq!("50.1", format!("{:.2}", num!(50.1)));
}

#[test]
fn test_to_string_009() {
  assert_eq!("50.123456789", format!("{:.6}", num!(50.123456789)));
}

#[test]
fn test_to_string_010() {
  assert_eq!("  50.1", format!("{:>6}", num!(50.1)));
}

#[test]
fn test_to_string_literals_with_trailing_zeros() {
  eqs!("1.5", parsed("1.50"));
  eqs!("1", parsed("1.0"));
  eqs!("0", parsed("0.000"));
  eqs!("0", parsed("0E-6176"));
  eqs!("0", parsed("0E+6111"));
  eqs!("100", parsed("1.00E+2"));
  eqs!("0.001", parsed("0.00100"));
  eqs!("-12.3", parsed("-12.3000"));
}

#[test]
fn test_to_string_new_with_a_scale() {
  eqs!("1", FeelNumber::new(100, 2));
  eqs!("0", FeelNumber::new(0, 5));
  eqs!("1200", FeelNumber::new(1200, 0));
  eqs!("0.5", FeelNumber::new(5000, 4));
}

#[test]
fn test_to_string_arithmetic() {
  eqs!("0", parsed("1.10") - parsed("1.10"));
  eqs!("0", num!(0) * parsed("1e-10"));
  eqs!("0.5", parsed("0.25") * num!(2));
  eqs!("1.5", parsed("-1.50").abs());
  eqs!("3", parsed("1.50") + parsed("1.50"));
}

#[test]
fn test_to_string_rounding() {
  eqs!("1", parsed("1").round_to(2, RoundingMode::HalfEven).unwrap());
  eqs!("0.5", parsed("0.5").round_to(3, RoundingMode::HalfEven).unwrap());
  eqs!("0", parsed("-0.001").round_to(2, RoundingMode::HalfEven).unwrap());
  eqs!("1.5", parsed("1.5").round_to(2, RoundingMode::Ceiling).unwrap());
  eqs!("2", parsed("1.5").round_to(0, RoundingMode::Ceiling).unwrap());
}

#[test]
fn test_to_string_width_and_fill() {
  assert_eq!("   1.5", format!("{:>6}", parsed("1.50")));
  assert_eq!("0001.5", format!("{:06}", parsed("1.50")));
  assert_eq!("+1.5", format!("{:+}", parsed("1.50")));
  assert_eq!("0     ", format!("{:<6}", parsed("0.000")));
}
