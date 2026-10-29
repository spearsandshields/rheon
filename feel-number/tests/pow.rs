mod common;

use flowstone_feel_number::FeelNumber;

#[test]
fn test_pow_001() {
  eqs!("1", num!(0).pow(&num!(0)).unwrap());
}

#[test]
#[cfg(feature = "intel")]
fn test_pow_002() {
  // Intel BID128 result, 1 ulp below the correctly rounded value
  eqs!("41959.857373594361860953310707468", num!(12.2384283).pow(&num!(4.25)).unwrap());
}

#[test]
#[cfg(not(feature = "intel"))]
fn test_pow_002() {
  // correctly rounded: 12.2384283^4.25 = 41959.857373594361860953310707468005517...
  eqs!("41959.85737359436186095331070746801", num!(12.2384283).pow(&num!(4.25)).unwrap());
}

#[test]
fn test_pow_003() {
  assert!(num!(9999).pow(&num!(9999)).is_none());
}

// Base within 1e-33 of one combined with a huge non-integer (or out of i32 range) exponent:
// the result is finite and must not be flushed to zero or reported as overflow.

#[test]
fn test_pow_004() {
  eqs!("1.000000010000000050000000166666667", num!(1.000000000000000000000000000000001).pow(&num!(1E+25)).unwrap());
}

#[test]
fn test_pow_005() {
  eqs!("1.000000000000500000000000125", num!(1.000000000000000000000000000000001).pow(&num!(5E+20)).unwrap());
}

#[test]
fn test_pow_006() {
  // (1 + 1e-33)^(1e33) is e (Intel: ...661, correctly rounded: ...661)
  let n = num!(1.000000000000000000000000000000001).pow(&num!(1E+33)).unwrap().to_string();
  assert!(n.starts_with("2.71828182845904523536028747135266"), "{n}");
}

#[test]
fn test_pow_007() {
  let n = num!(1.000000000000000000000000000000001).pow(&num!(-1E+33)).unwrap().to_string();
  assert!(n.starts_with("0.36787944117144232159552377016146"), "{n}");
}

#[test]
fn test_pow_008() {
  let n = num!(0.9999999999999999999999999999999999)
    .pow(&num!(-9999999999999999999999999999999999))
    .unwrap()
    .to_string();
  assert!(n.starts_with("2.71828182845904523536028747135266"), "{n}");
}

#[test]
fn test_pow_009() {
  // overflow -> None, underflow -> 0 (and not the other way round)
  assert!(num!(1.000000000000000000000000000000001).pow(&num!(1E+40)).is_none());
  eqs!("+0E-6176", format!("{:?}", num!(1.000000000000000000000000000000001).pow(&num!(-1E+40)).unwrap()));
  assert!(num!(1.000000000000000000000000000000001).pow(&num!(1E+6144)).is_none());
}

fn dbg(n: FeelNumber) -> String {
  format!("{n:?}")
}

fn n(s: &str) -> FeelNumber {
  s.parse().unwrap()
}

#[test]
fn test_pow_010() {
  // (-1)^y with an integer y outside i32: the cohort of 1 depends on the exponents
  // of x and y exactly like in Intel's bid128_pow (zero logarithm propagated through fma)
  eqs!("+1000000000000000000000000E-24", dbg(n("-1").pow(&n("1E+10")).unwrap()));
  eqs!("+10000000000000000000000000E-25", dbg(n("-1").pow(&n("1.0E+10")).unwrap()));
  eqs!("+1000000000000000000000000000000000E-33", dbg(n("-1").pow(&n("10000000000")).unwrap()));
  eqs!("+1E+0", dbg(n("-1").pow(&n("1E+40")).unwrap()));
  eqs!("+1000E-3", dbg(n("-1.000").pow(&n("1E+40")).unwrap()));
  eqs!("+1000000000000000000000000000000000E-33", dbg(n("-1.000").pow(&n("1E+10")).unwrap()));
  eqs!("-1000000000000000000000000000000000E-33", dbg(n("-1").pow(&n("2147483649")).unwrap()));
  eqs!("1.000000000000000000000000", n("-1").pow(&n("1E+10")).unwrap().abs());
  eqs!("1", n("-1").pow(&n("1E+40")).unwrap().abs());
}
