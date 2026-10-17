mod common;

use dsntk_feel_number::FeelNumber;

#[test]
fn test_round_001() {
  eqs!("123.46", num!(123.4567).round(&num!(2)));
}

#[test]
fn test_round_002() {
  eqs!("123.45", num!(123.4547).round(&num!(2)));
}

#[test]
fn test_round_003() {
  eqs!("100", num!(123.4567).round(&num!(-2)));
}

#[test]
fn test_round_004() {
  eqs!("200", num!(163.4567).round(&num!(-2)));
}

#[test]
fn test_round_005() {
  eqs!("0.0", num!(0.0).round(&num!(1)));
}

#[test]
fn test_round_006() {
  // quantum 1E+6200 is clamped to 1E+6144 (dfp-number-sys scalbn clamps the exponent)
  eqs!("+0E+6111", format!("{:?}", num!(1.5).round(&num!(-6200))));
}
