mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn test_exp_001() {
  eqs!("2.718281828459045235360287471352662", num!(1).exp().unwrap());
}

#[test]
fn test_exp_002() {
  eqs!("54.59815003314423907811026120286088", num!(4).exp().unwrap());
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
fn test_exp_003() {
  // integers and short dyadic fractions after rounding to 34 digits
  assert_value("2", n("0.6931471805599453094172321214581766").exp().unwrap());
  assert_value("0.5", n("-0.6931471805599453094172321214581766").exp().unwrap());
  assert_value("219264159386793477442556282414391", n("74.46783").exp().unwrap());
}

#[test]
fn test_exp_004() {
  // results that round to 1
  assert_value("1", n("1E-34").exp().unwrap());
  assert_value("1", n("5E-35").exp().unwrap());
  assert_value("1", n("1E-40").exp().unwrap());
  assert_value("1", n("-1E-40").exp().unwrap());
  assert_value("1", n("1E-6176").exp().unwrap());
}

#[test]
fn test_exp_005() {
  assert_value("15.9999999999999999999999999881375", n("2.772588722239781237668928485091300").exp().unwrap());
  assert_value("1.000000000000000000000000000000001", n("1E-33").exp().unwrap());
}
