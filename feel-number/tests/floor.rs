mod common;

use dsntk_feel_number::FeelNumber;

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
  // the scale passed to scalbn is clamped to -6176..=6144 (dfp-number-sys behaviour)
  eqs!("0.05", num!(0.5).floor(6145).unwrap());
}
