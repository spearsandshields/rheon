mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn _0001() {
  eqs!("2", num!(1.5).ceiling(0).unwrap());
}

#[test]
fn _0002() {
  eqs!("-1", num!(-1.5).ceiling(0).unwrap());
}

#[test]
fn _0003() {
  eqs!("-1.5", num!(-1.56).ceiling(1).unwrap());
}

#[test]
fn _0004() {
  eqs!("1", num!(0.3333).ceiling(0).unwrap());
}

#[test]
fn _0005() {
  eqs!("0", num!(-0.3333).ceiling(0).unwrap());
}

#[test]
fn _0006() {
  println!("{}", num!(-0.3333).ceiling(0).unwrap());
}

#[test]
fn _0007() {
  // a number with no digit below the scale is itself, at every scale of -6111..=6176
  eqs!("0.25", num!(0.25).ceiling(6145).unwrap());
}

#[test]
fn test_ceiling_scale_6176_leaves_settled_values_unchanged() {
  eqs!("5.5", num!(5.5).ceiling(6176).unwrap());
  assert!(num!(5.5).ceiling(6177).is_err());
}
