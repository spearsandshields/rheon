mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn _0001() {
  eqs!("1", num!(1.5).floor(0).unwrap());
}

#[test]
fn _0002() {
  eqs!("1.5", num!(1.56).floor(1).unwrap());
}

#[test]
fn _0003() {
  eqs!("-2", num!(-1.5).floor(0).unwrap());
}

#[test]
fn _0004() {
  eqs!("-1.6", num!(-1.56).floor(1).unwrap());
}

#[test]
fn _0005() {
  eqs!("0", num!(0.333).floor(0).unwrap());
}

#[test]
fn _0006() {
  eqs!("-1", num!(-0.3333).floor(0).unwrap());
}

#[test]
fn _0007() {
  // a number with no digit below the scale is itself, at every scale of -6111..=6176
  eqs!("0.5", num!(0.5).floor(6145).unwrap());
}

#[test]
fn test_floor_scale_6176_leaves_settled_values_unchanged() {
  eqs!("5.5", num!(5.5).floor(6176).unwrap());
  assert!(num!(5.5).floor(6177).is_err());
}
