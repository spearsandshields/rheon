mod common;

use flowstone_feel_number::FeelNumber;

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

#[test]
fn test_scale_rounding_with_unit_coefficient() {
  // Regression: a rounded integral of ±1 was returned unscaled (round_up(0.1, 1) == 1).
  eqs!("0.1", num!(0.1).round_up(1).unwrap());
  eqs!("0.1", num!(0.05).round_up(1).unwrap());
  eqs!("0.01", num!(0.01).round_up(2).unwrap());
  eqs!("10", num!(10).round_up(-1).unwrap());
  eqs!("10", num!(1).round_up(-1).unwrap());
  eqs!("-0.000001", num!(-0.0000001).round_up(6).unwrap());
  eqs!("0.1", num!(0.19).round_down(1).unwrap());
  eqs!("0.1", num!(0.05).round_half_up(1).unwrap());
  eqs!("0.1", num!(0.15).round_half_down(1).unwrap());
  eqs!("0.1", num!(0.19).floor(1).unwrap());
  eqs!("-0.000001", num!(-0.0000001).floor(6).unwrap());
  eqs!("0.1", num!(0.01).ceiling(1).unwrap());
  assert_eq!(num!(1E+6111), num!(5.5).round_up(-6111).unwrap());
  assert_eq!(num!(-1E+6111), num!(-1.56).floor(-6111).unwrap());
  assert_eq!(num!(1E-6144), num!(1E-6144).round_up(6144).unwrap());
  eqs!("0.2", num!(0.11).round_up(1).unwrap());
  eqs!("1", num!(0.95).round_up(1).unwrap());
}

#[test]
fn test_scale_up_to_6176_leaves_settled_values_unchanged() {
  // FEEL: the scale of the rounding functions may be any integer in -6111..=6176. A value that
  // has no digit below 10^-scale is returned unchanged, even when the scaled intermediate
  // `n * 10^scale` would not fit in decimal128 (TCK 1141-1144, 017_a).
  eqs!("5.5", num!(5.5).round_up(6176).unwrap());
  eqs!("5.5", num!(5.5).round_down(6176).unwrap());
  eqs!("5.5", num!(5.5).round_half_up(6176).unwrap());
  eqs!("5.5", num!(5.5).round_half_down(6176).unwrap());
  eqs!("-5.5", num!(-5.5).round_up(6176).unwrap());
  eqs!("5.5", num!(5.5).round(&num!(6176)));
  eqs!("0.3333333333333333333333333333333333", (num!(1) / num!(3)).round_up(6176).unwrap());
  eqs!("0.3333333333333333333333333333333333", (num!(1) / num!(3)).round(&num!(6176)));
  // ...but a scale outside the FEEL range is still an error (017_b).
  assert!(num!(5.5).round_up(6177).is_err());
  assert!(num!(5.5).round_up(-6112).is_err());
  // and values that do have digits below the scale still round
  eqs!("5.6", num!(5.55).round_up(1).unwrap());
}
