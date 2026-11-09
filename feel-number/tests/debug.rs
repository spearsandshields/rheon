mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn test_debug_001() {
  eqs!("+49E+0", format!("{:?}", FeelNumber::new(49, 0)));
}

#[test]
fn test_debug_002() {
  eqs!("+123456789E-8", format!("{:?}", FeelNumber::new(123456789, 8)));
}

#[test]
fn test_debug_003() {
  eqs!("+5050E-2", format!("{:?}", num!(50.50)));
}

#[test]
fn test_debug_004() {
  // scale clamped to -6176..=6144 like dfp-number-sys bid128_scalbn
  eqs!("+890E-6176", format!("{:?}", FeelNumber::new(890, 6177)));
  eqs!("+1000000000000000000000000000000000E+6111", format!("{:?}", FeelNumber::new(1, -6177)));
}
