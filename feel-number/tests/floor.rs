mod common;

use rheon_feel_number::{FeelNumber, RoundingMode};

#[test]
fn _0001() {
  eqs!("1", num!(1.5).round_to(0, RoundingMode::Floor).unwrap());
}

#[test]
fn _0002() {
  eqs!("1.5", num!(1.56).round_to(1, RoundingMode::Floor).unwrap());
}

#[test]
fn _0003() {
  eqs!("-2", num!(-1.5).round_to(0, RoundingMode::Floor).unwrap());
}

#[test]
fn _0004() {
  eqs!("-1.6", num!(-1.56).round_to(1, RoundingMode::Floor).unwrap());
}

#[test]
fn _0005() {
  eqs!("0", num!(0.333).round_to(0, RoundingMode::Floor).unwrap());
}

#[test]
fn _0006() {
  eqs!("-1", num!(-0.3333).round_to(0, RoundingMode::Floor).unwrap());
}

#[test]
fn _0007() {
  // a number with no digit below the scale is itself, at every scale of -6111..=6176
  eqs!("0.5", num!(0.5).round_to(6145, RoundingMode::Floor).unwrap());
}

#[test]
fn test_floor_scale_6176_leaves_settled_values_unchanged() {
  eqs!("5.5", num!(5.5).round_to(6176, RoundingMode::Floor).unwrap());
  assert!(num!(5.5).round_to(6177, RoundingMode::Floor).is_err());
}
