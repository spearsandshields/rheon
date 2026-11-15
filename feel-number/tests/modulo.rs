mod common;

use rheon_feel_number::FeelNumber;

fn modulo(dividend: &str, divisor: &str) -> Option<FeelNumber> {
  dividend.parse::<FeelNumber>().unwrap().modulo(&divisor.parse().unwrap())
}

#[test]
fn table_77_examples() {
  for (dividend, divisor, expected) in [
    ("12", "5", "2"),
    ("-12", "5", "3"),
    ("12", "-5", "-3"),
    ("-12", "-5", "-2"),
    ("10.1", "4.5", "1.1"),
    ("-10.1", "4.5", "3.4"),
    ("10.1", "-4.5", "-3.4"),
    ("-10.1", "-4.5", "-1.1"),
  ] {
    assert_eq!(expected, modulo(dividend, divisor).unwrap().to_string());
  }
}

#[test]
fn the_remainder_is_exact() {
  // a - b·floor(a/b) over the exact values, rounded once to 34 digits, whatever the quotient
  for (dividend, divisor, expected) in [
    ("1E+40", "3", "1"),
    ("1E+40", "7", "4"),
    ("-1E+40", "7", "3"),
    ("1E+40", "-7", "-3"),
    ("5E+33", "0.7", "0.6"),
    ("1", "3E-35", "0.00000000000000000000000000000000001"),
    (
      "0.9999999999999999999999999999999998",
      "0.3333333333333333333333333333333333",
      "0.3333333333333333333333333333333332",
    ),
    ("1E+6000", "1E-6000", "0"),
    ("1E+6000", "-7", "-6"),
    ("1E+6111", "9999999999999999999999999999999999", "1E+25"),
    ("1E-6176", "-1E-6175", "-9E-6176"),
    ("3", "1E+40", "3"),
    ("-3", "1E+40", "1E+40"),
    ("0", "-5", "0"),
    ("-0", "5", "0"),
  ] {
    let actual = modulo(dividend, divisor).unwrap();
    assert!(
      actual == expected.parse::<FeelNumber>().unwrap(),
      "modulo({dividend}, {divisor}) = {actual}, expected {expected}"
    );
  }
}

#[test]
fn a_zero_divisor_has_no_remainder() {
  assert!(modulo("12", "0").is_none());
  assert!(modulo("0", "-0.00").is_none());
}
