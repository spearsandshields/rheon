mod common;

use flowstone_feel_number::FeelNumber;

#[test]
fn test_ln_001() {
  assert!(num!(-1).ln().is_none());
}

#[test]
fn test_ln_002() {
  assert!(num!(0).ln().is_none());
}

#[test]
fn test_ln_003() {
  eqs!("0", num!(1).ln().unwrap());
}

#[test]
fn test_ln_004() {
  eqs!("1.386294361119890618834464242916353", num!(4).ln().unwrap());
}

#[test]
fn test_ln_005() {
  eqs!("2.302585092994045684017991454684364", num!(10).ln().unwrap());
}

fn dbg(n: FeelNumber) -> String {
  format!("{n:?}")
}

fn n(s: &str) -> FeelNumber {
  s.parse().unwrap()
}

// The cohort of ln results follows Intel's binary128 -> decimal128 conversion.

#[test]
fn test_ln_006() {
  eqs!("+2E+0", dbg(n("7.389056098930650227230427460575008").ln().unwrap()));
  eqs!("-2E+0", dbg(n("0.1353352832366126918939994949724844").ln().unwrap()));
  eqs!("2", n("7.389056098930650227230427460575008").ln().unwrap().abs());
}

#[test]
fn test_ln_007() {
  eqs!("+0E+0", dbg(n("1").ln().unwrap()));
  eqs!("+0E+0", dbg(n("1.000").ln().unwrap()));
  eqs!("+6931471805599453094172321214581766E-34", dbg(n("2").ln().unwrap()));
  eqs!("+6907755278982137052053974364053093E-32", dbg(n("1E30").ln().unwrap()));
}
