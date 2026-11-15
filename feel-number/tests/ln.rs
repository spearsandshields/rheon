mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn test_ln_001() {
  assert!(num!(-1).ln().is_none());
}

#[test]
fn test_ln_002() {
  assert!(num!(0).ln().is_none());
}

#[test]
fn test_ln_003() {
  eqs!("0", num!(1).ln().unwrap());
}

#[test]
fn test_ln_004() {
  eqs!("1.386294361119890618834464242916353", num!(4).ln().unwrap());
}

#[test]
fn test_ln_005() {
  eqs!("2.302585092994045684017991454684364", num!(10).ln().unwrap());
}

fn n(s: &str) -> FeelNumber {
  s.parse().unwrap()
}

/// The value and its canonical text; the cohort of the result is not asserted.
#[track_caller]
fn assert_value(expected: &str, actual: FeelNumber) {
  assert_eq!(expected, actual.to_string());
  assert!(actual == n(expected), "{actual:?} is not {expected}");
}

#[test]
fn test_ln_006() {
  // integers after rounding to 34 digits
  assert_value("2", n("7.389056098930650227230427460575008").ln().unwrap());
  assert_value("-2", n("0.1353352832366126918939994949724844").ln().unwrap());
}

#[test]
fn test_ln_007() {
  assert_value("0", n("1").ln().unwrap());
  assert_value("0", n("1.000").ln().unwrap());
  assert_value("0.6931471805599453094172321214581766", n("2").ln().unwrap());
  assert_value("69.07755278982137052053974364053093", n("1E30").ln().unwrap());
}
