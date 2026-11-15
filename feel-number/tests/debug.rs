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
  // n · 10^-s rounded once to decimal128, whatever the scale: 890E-6177 is 89E-6176 exactly,
  // 1E+6177 overflows
  eqs!("+89E-6176", format!("{:?}", FeelNumber::new(890, 6177)));
  eqs!("+8E-6176", format!("{:?}", FeelNumber::new(75, 6177)));
  eqs!("+Inf", format!("{:?}", FeelNumber::new(1, -6177)));
}
