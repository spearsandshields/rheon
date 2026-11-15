mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn test_pow_001() {
  eqs!("1", num!(0).pow(&num!(0)).unwrap());
}

#[test]
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

fn n(s: &str) -> FeelNumber {
  s.parse().unwrap()
}

#[test]
fn test_pow_010() {
  // (-1)^y for an integer y: 1 or -1 by the parity of y, whatever its size and cohort
  for (x, y, expected) in [
    ("-1", "1E+10", "1"),
    ("-1", "1.0E+10", "1"),
    ("-1", "10000000000", "1"),
    ("-1", "1E+40", "1"),
    ("-1.000", "1E+40", "1"),
    ("-1.000", "1E+10", "1"),
    ("-1", "2147483649", "-1"),
    ("-1", "-2147483649", "-1"),
    ("-1", "99999999999999999999", "-1"),
  ] {
    assert_eq!(expected, n(x).pow(&n(y)).unwrap().to_string(), "{x} ** {y}");
  }
}

#[test]
fn integer_powers_are_correctly_rounded() {
  // DMN 1.6 Table 61: value(p,s) = value(e1)^value(e2) + ε, "p is limited to 34 digits", with
  // the one rounding of §10.3.2.3.1 (to the nearest, a tie to even); the expected values are the
  // exact powers rounded once
  let base = n("1") + n("0.05") / n("12");
  let mut wrong = vec![];
  for (x, y, expected) in [
    (n("1.05"), "360", "42476396.40868002039560415895318542"),
    (base, "360", "4.46774431400613221242807011041355"),
    (n("1.0025"), "120", "1.349353547190833348068274459415923"),
    (n("1.000001"), "1000000", "2.718280469319376883819799708454356"),
    (n("1.5"), "200", "1.652919910788208030156002593555710E+35"),
    (n("1.0001"), "1000000", "2.674710993142140172948354481790713E+43"),
    (n("1.1"), "100000", "1.855734417546823747428055637305764E+4139"),
    (n("1.1"), "-100000", "5.388702125393264024384868212038677E-4140"),
    (n("2"), "-20000", "2.51238805769874458518013504213361E-6021"),
    (n("0.5"), "20000", "2.51238805769874458518013504213361E-6021"),
    (n("0.3"), "-200", "3.764861949599026419883421890011116E+104"),
    (n("999"), "100", "9.047921471137090420322146062399503E+299"),
    (n("3"), "-7", "0.0004572473708276177411979881115683585"),
    (n("7"), "-3", "0.002915451895043731778425655976676385"),
    (n("12.5"), "-3", "0.000512"),
    (n("3"), "50", "717897987691852588770249"),
    (n("2"), "100", "1267650600228229401496703205376"),
    // exact ties: 35 digits ending in 5, rounded to the even neighbour
    (n("5"), "50", "8.881784197001252323389053344726562E+34"),
    (n("2"), "-50", "8.881784197001252323389053344726562E-16"),
    (n("-5"), "51", "-4.440892098500626161694526672363281E+35"),
  ] {
    let actual = x.pow(&n(y));
    if actual != Some(n(expected)) {
      wrong.push(format!("{x} ** {y}: {actual:?}, expected {expected}"));
    }
  }
  assert!(wrong.is_empty(), "{} wrong powers: {wrong:#?}", wrong.len());
}
