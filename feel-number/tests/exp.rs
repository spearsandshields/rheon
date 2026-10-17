mod common;

use dsntk_feel_number::FeelNumber;

#[test]
fn test_exp_001() {
  eqs!("2.718281828459045235360287471352662", num!(1).exp().unwrap());
}

#[test]
fn test_exp_002() {
  eqs!("54.59815003314423907811026120286088", num!(4).exp().unwrap());
}

fn dbg(n: FeelNumber) -> String {
  format!("{n:?}")
}

fn n(s: &str) -> FeelNumber {
  s.parse().unwrap()
}

// The cohort (stored exponent) of exp results follows Intel's binary128 -> decimal128
// conversion: integers and short dyadic fractions that are exact in binary128 keep
// the shortest form, everything else has 34 digits. The cohort is visible in FEEL,
// e.g. string(abs(exp(log(0.5)))).

#[test]
fn test_exp_003() {
  eqs!("+2E+0", dbg(n("0.6931471805599453094172321214581766").exp().unwrap()));
  eqs!("+5E-1", dbg(n("-0.6931471805599453094172321214581766").exp().unwrap()));
  eqs!("0.5", n("-0.6931471805599453094172321214581766").exp().unwrap().abs());
}

#[test]
fn test_exp_004() {
  // exp(1E-34) is 1 after rounding to 34 digits, but not in binary128 (1 + 2^-112)
  eqs!("+1000000000000000000000000000000000E-33", dbg(n("1E-34").exp().unwrap()));
  eqs!("1.000000000000000000000000000000000", n("1E-34").exp().unwrap().abs());
  // exp(5E-35) and exp(1E-40) are exactly 1 in binary128
  eqs!("+1E+0", dbg(n("5E-35").exp().unwrap()));
  eqs!("+1E+0", dbg(n("1E-40").exp().unwrap()));
}

#[test]
fn test_exp_005() {
  // integral after rounding to 34 digits, but not an integer in binary128
  eqs!("+2192641593867934774425562824143910E-1", dbg(n("74.46783").exp().unwrap()));
  // not dyadic: always 34 digits
  eqs!("+1599999999999999999999999998813750E-32", dbg(n("2.772588722239781237668928485091300").exp().unwrap()));
}
