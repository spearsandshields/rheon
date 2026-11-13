//! Targeted tests for the exact-op fast paths, the 256-bit helper and the guards around
//! the backend. Every expected value below was checked against the Intel backend
//! (`tools/difftest eval`), except where a comment says that Intel differs.
//!
//! The randomized comparisons with a reference whose size depends on [full] run a reduced
//! number of cases by default and their full size when `RHEON_FULL_TESTS` is set.
use super::*;

fn n(s: &str) -> FeelNumber {
  s.parse().unwrap()
}

fn dbg(n: FeelNumber) -> String {
  format!("{n:?}")
}

/// `true` when the randomized comparisons run their full size (`RHEON_FULL_TESTS` set).
fn full() -> bool {
  std::env::var_os("RHEON_FULL_TESTS").is_some()
}

/// Simple deterministic generator (splitmix64).
fn rng(state: &mut u64) -> u64 {
  *state = state.wrapping_add(0x9E3779B97F4A7C15);
  let mut z = *state;
  z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
  z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
  z ^ (z >> 31)
}

fn random_u128(s: &mut u64) -> u128 {
  let bits = rng(s) % 129;
  if bits == 0 { 0 } else { (((rng(s) as u128) << 64) | rng(s) as u128) >> (128 - bits) }
}

// --- schoolbook decimal-string arithmetic, the oracle for the 256-bit helper ---

fn digits_of(s: &str) -> Vec<u8> {
  let s = s.trim_start_matches('0');
  s.bytes().rev().map(|b| b - b'0').collect()
}

fn string_of(mut d: Vec<u8>) -> String {
  while d.len() > 1 && *d.last().unwrap() == 0 {
    d.pop();
  }
  if d.is_empty() {
    return "0".to_string();
  }
  d.iter().rev().map(|x| (b'0' + x) as char).collect()
}

fn str_add(a: &str, b: &str) -> String {
  let (a, b) = (digits_of(a), digits_of(b));
  let mut r = vec![];
  let mut carry = 0;
  for i in 0..a.len().max(b.len()) {
    let s = carry + a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0);
    r.push(s % 10);
    carry = s / 10;
  }
  if carry > 0 {
    r.push(carry);
  }
  string_of(r)
}

/// `a - b` for `a >= b`.
fn str_sub(a: &str, b: &str) -> String {
  let (a, b) = (digits_of(a), digits_of(b));
  let mut r = vec![];
  let mut borrow = 0_i8;
  for (i, x) in a.iter().enumerate() {
    let mut s = *x as i8 - borrow - b.get(i).copied().unwrap_or(0) as i8;
    borrow = 0;
    if s < 0 {
      s += 10;
      borrow = 1;
    }
    r.push(s as u8);
  }
  assert_eq!(borrow, 0, "negative difference");
  string_of(r)
}

fn str_mul(a: &str, b: &str) -> String {
  let (a, b) = (digits_of(a), digits_of(b));
  let mut r = vec![0_u32; a.len() + b.len() + 1];
  for (i, x) in a.iter().enumerate() {
    for (j, y) in b.iter().enumerate() {
      r[i + j] += *x as u32 * *y as u32;
    }
  }
  let mut carry = 0;
  for d in r.iter_mut() {
    let s = *d + carry;
    *d = s % 10;
    carry = s / 10;
  }
  assert_eq!(carry, 0);
  string_of(r.into_iter().map(|d| d as u8).collect())
}

fn str_cmp(a: &str, b: &str) -> Ordering {
  let (a, b) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
  a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

#[test]
fn u256_mul_matches_schoolbook() {
  let mut s = 3_u64;
  for _ in 0..20_000 {
    let (a, b) = (random_u128(&mut s), random_u128(&mut s));
    let p = U256::mul(a, b);
    assert_eq!(p.to_decimal(), str_mul(&a.to_string(), &b.to_string()), "{a} * {b}");
    assert_eq!(U256::from_decimal(&p.to_decimal()), Some(p));
  }
  assert_eq!(U256::mul(u128::MAX, u128::MAX), U256 { hi: u128::MAX - 1, lo: 1 });
  assert_eq!(U256::mul(u128::MAX, 0), U256::ZERO);
}

#[test]
fn u256_add_sub_shift_and_pow10() {
  let mut s = 5_u64;
  for _ in 0..20_000 {
    let (a, b) = (U256::mul(random_u128(&mut s), random_u128(&mut s)), U256::mul(random_u128(&mut s), random_u128(&mut s)));
    let (x, y) = if a >= b { (a, b) } else { (b, a) };
    if let Some(sum) = x.checked_add(y) {
      assert_eq!(sum.to_decimal(), str_add(&x.to_decimal(), &y.to_decimal()));
      assert_eq!(sum.checked_sub(y), Some(x));
    }
    assert_eq!(x.checked_sub(y).unwrap().to_decimal(), str_sub(&x.to_decimal(), &y.to_decimal()));
    assert_eq!(y.checked_sub(x).is_some(), x == y);
    let k = (rng(&mut s) % 300) as u32;
    match x.checked_shl(k) {
      Some(shifted) => {
        assert_eq!(shifted.shr(k), x, "{x:?} << {k}");
        assert_eq!(shifted.to_decimal(), str_mul(&x.to_decimal(), &U256::pow2_decimal(k)), "{x:?} << {k}");
      }
      None => assert!(x.bits() + k > 256),
    }
    let m = random_u128(&mut s);
    match x.checked_mul_u128(m) {
      Some(p) => assert_eq!(p.to_decimal(), str_mul(&x.to_decimal(), &m.to_string())),
      None => assert!(str_mul(&x.to_decimal(), &m.to_string()).len() >= 77),
    }
  }
  for k in 0..78 {
    assert_eq!(U256::pow10(k).unwrap().to_decimal(), format!("1{}", "0".repeat(k as usize)));
  }
  assert_eq!(U256::pow10(78), None);
  assert_eq!(U256::from_decimal(&"9".repeat(77)).map(|x| x.to_decimal()), Some("9".repeat(77)));
  assert_eq!(U256::from_decimal(&"1".repeat(78)).map(|x| x.to_decimal()), Some("1".repeat(78)));
  assert_eq!(U256::from_decimal(&"9".repeat(78)), None);
  assert_eq!(U256::from_decimal("x"), None);
  assert_eq!(U256::from_decimal(""), None);
  assert_eq!(U256::from_u128(1).checked_shl(255).unwrap().checked_shl(1), None);
  assert_eq!(U256::ZERO.checked_shl(300), Some(U256::ZERO));
  assert_eq!(U256::from_u128(7).div_rem_u64(2), (U256::from_u128(3), 1));
}

impl U256 {
  /// 2^k as a decimal string (test helper).
  fn pow2_decimal(k: u32) -> String {
    let mut s = "1".to_string();
    for _ in 0..k {
      s = str_add(&s, &s);
    }
    s
  }
}

#[test]
fn isqrt_rem_is_exact() {
  let mut s = 11_u64;
  let check = |r: U256| {
    let (root, inexact) = isqrt_rem(r);
    let square = U256::mul(root, root);
    assert!(square <= r, "{r:?}: root {root} too large");
    assert!(U256::mul(root + 1, root + 1) > r, "{r:?}: root {root} too small");
    assert_eq!(inexact, square != r, "{r:?}");
  };
  for _ in 0..50_000 {
    let r = match rng(&mut s) % 4 {
      0 => U256::mul(random_u128(&mut s) >> 3, random_u128(&mut s) >> 3),
      1 => U256::from_u128(random_u128(&mut s)),
      2 => {
        // squares and their neighbours
        let x = random_u128(&mut s) >> 8;
        let sq = U256::mul(x, x);
        match rng(&mut s) % 3 {
          0 => sq,
          1 => sq.checked_add(U256::from_u128(1)).unwrap(),
          _ => sq.checked_sub(U256::from_u128(1)).unwrap_or(sq),
        }
      }
      _ => U256::from_u128(random_u128(&mut s)).checked_mul_pow10((rng(&mut s) % 40) as u32).unwrap_or(U256::ZERO),
    };
    if r.bits() > 250 {
      continue;
    }
    check(r);
  }
  check(U256::ZERO);
  check(U256::from_u128(1));
  check(U256 {
    hi: u128::MAX >> 6,
    lo: u128::MAX,
  });
}

#[test]
fn round_pack_u128_matches_round_pack() {
  let mut s = 1_u64;
  for _ in 0..200_000 {
    let c = random_u128(&mut s);
    let c = match rng(&mut s) % 4 {
      0 => c,
      1 => c / 10 * 10,
      2 => TEN_POW_34 - 1 - (c % 1000),
      _ => (c / 7) * 5 + 5,
    };
    let exp = match rng(&mut s) % 5 {
      0 => (rng(&mut s) % 100) as i64 - 50,
      1 => EXP_MIN - (rng(&mut s) % 60) as i64,
      2 => EXP_MAX - 40 + (rng(&mut s) % 60) as i64,
      3 => -((rng(&mut s) % 6300) as i64),
      _ => (rng(&mut s) % 6300) as i64,
    };
    let neg = rng(&mut s).is_multiple_of(2);
    assert_eq!(round_pack(neg, &c.to_string(), exp), round_pack_u128(neg, c, exp), "c={c} exp={exp}");
  }
  // zero, one, the largest coefficient of 34 digits and the smallest of 35, each at exponent 0,
  // at both exponent limits and one step past each limit
  for c in [0, 1, TEN_POW_34 - 1, TEN_POW_34] {
    for exp in [EXP_MIN - 1, EXP_MIN, 0, EXP_MAX, EXP_MAX + 1] {
      for neg in [false, true] {
        assert_eq!(round_pack(neg, &c.to_string(), exp), round_pack_u128(neg, c, exp), "c={c} exp={exp}");
      }
    }
  }
}

/// Random coefficient with 1..=34 digits.
fn coefficient(s: &mut u64) -> u128 {
  let digits = 1 + rng(s) % 34;
  let mut c: u128 = 0;
  for _ in 0..digits {
    c = c * 10 + (rng(s) % 10) as u128;
  }
  c
}

#[test]
fn digit_count_equals_ilog10() {
  let reference = |c: u128| if c == 0 { 1 } else { c.ilog10() + 1 };
  let mut values = vec![0_u128, 1, u128::MAX];
  for k in 0..128 {
    values.extend([1_u128 << k, (1_u128 << k) - 1, (1_u128 << k) + 1]);
  }
  for (k, &p) in POW10.iter().enumerate() {
    assert_eq!(10_u128.pow(k as u32), p);
    values.extend([p - 1, p, p + 1]);
  }
  let mut s = 7_u64;
  let count = if full() { 100_000 } else { 10_000 };
  values.extend((0..count).map(|_| random_u128(&mut s)));
  for c in values {
    assert_eq!(reference(c), digit_count(c), "{c}");
  }
}

/// Reference comparison of two finite non-zero values of the same sign: adjusted exponents first,
/// then both coefficients scaled to 34 digits. Every other pair is compared by [cmp_v].
fn cmp_v_reference(a: V, b: V) -> Option<Ordering> {
  match (a, b) {
    (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) if ca != 0 && cb != 0 && (sa && ca != 0) == (sb && cb != 0) => {
      let na = sa && ca != 0;
      let la = digit_count(ca) as i64 + ea as i64;
      let lb = digit_count(cb) as i64 + eb as i64;
      let mag = if la != lb {
        la.cmp(&lb)
      } else {
        let xa = ca * pow10(PRECISION as u32 - digit_count(ca));
        let xb = cb * pow10(PRECISION as u32 - digit_count(cb));
        xa.cmp(&xb)
      };
      Some(if na { mag.reverse() } else { mag })
    }
    _ => cmp_v(a, b),
  }
}

/// A random finite decimal128 value: coefficients of every length (and zero), exponents near
/// each other, far apart and at the limits.
fn random_v(state: &mut u64, base_exp: i32) -> V {
  let c = if rng(state).is_multiple_of(35) { 0 } else { coefficient(state) };
  let e = match rng(state) % 4 {
    0 => base_exp,
    1 => base_exp + (rng(state) % 7) as i32 - 3,
    2 => (rng(state) % 12288) as i32 - 6176,
    _ => [EXP_MIN as i32, EXP_MAX as i32, 0, -34, 34][(rng(state) % 5) as usize],
  }
  .clamp(EXP_MIN as i32, EXP_MAX as i32);
  V::Fin(rng(state).is_multiple_of(2), c, e)
}

#[test]
fn cmp_v_equals_reference() {
  let mut state = 11;
  for _ in 0..if full() { 300_000 } else { 20_000 } {
    let base = (rng(&mut state) % 12288) as i32 - 6176;
    let a = random_v(&mut state, base);
    let b = if rng(&mut state).is_multiple_of(8) { a } else { random_v(&mut state, base) };
    assert_eq!(cmp_v_reference(a, b), cmp_v(a, b), "{a:?} {b:?}");
  }
  // equal values in different cohorts, and the extremes
  let pairs = [
    (V::Fin(false, 110, -2), V::Fin(false, 11, -1)),
    (V::Fin(true, 1, 30), V::Fin(true, 1_000_000_000_000_000_000_000_000_000_000, 0)),
    (V::Fin(false, TEN_POW_34 - 1, EXP_MAX as i32), V::Fin(false, 1, EXP_MIN as i32)),
    (V::Fin(false, 1, 40), V::Fin(false, TEN_POW_34 - 1, 0)),
    (V::Fin(false, 1, 34), V::Fin(false, TEN_POW_34 - 1, 0)),
    (V::Fin(false, 1, 33), V::Fin(false, TEN_POW_34 - 1, 0)),
  ];
  for (a, b) in pairs {
    assert_eq!(cmp_v_reference(a, b), cmp_v(a, b), "{a:?} {b:?}");
    assert_eq!(cmp_v_reference(b, a), cmp_v(b, a), "{b:?} {a:?}");
  }
}

#[test]
fn add_paths_agree_with_exact_sum() {
  // every path of `add_fin` (u128, sticky, U256) is compared with the exact decimal-string sum
  let mut s = 7_u64;
  for _ in 0..50_000 {
    let (ca, cb) = (coefficient(&mut s), coefficient(&mut s));
    let ea = (rng(&mut s) % 200) as i32 - 100;
    let eb = match rng(&mut s) % 3 {
      0 => ea + (rng(&mut s) % 10) as i32 - 5,
      1 => ea + (rng(&mut s) % 100) as i32 - 50,
      _ => ea + (rng(&mut s) % 400) as i32 - 200,
    };
    let (sa, sb) = (rng(&mut s).is_multiple_of(2), rng(&mut s).is_multiple_of(2));
    let (sh, ch, eh, sl, cl, el) = if ea >= eb { (sa, ca, ea, sb, cb, eb) } else { (sb, cb, eb, sa, ca, ea) };
    let x = format!("{ch}{}", "0".repeat((eh - el) as usize));
    let l = cl.to_string();
    let (sign, sum) = if sh == sl {
      (sh, str_add(&x, &l))
    } else if str_cmp(&x, &l) != Ordering::Less {
      (sh, str_sub(&x, &l))
    } else {
      (sl, str_sub(&l, &x))
    };
    let expected = round_pack(sign, &sum, el as i64);
    let actual = add_fin(sa, ca, ea, sb, cb, eb);
    // the caller normalizes the cohort and fixes the sign of an exact zero, compare values only
    let norm = |(v, x): (V, bool)| match ideal(v, x, ea.min(eb)) {
      V::Fin(_, 0, e) => (V::Fin(false, 0, e), x),
      other => (other, x),
    };
    assert_eq!(norm(expected), norm(actual), "{sa} {ca}E{ea} + {sb} {cb}E{eb}");
  }
}

#[test]
fn mul_agrees_with_exact_product() {
  let mut s = 9_u64;
  for _ in 0..50_000 {
    let (ca, cb) = (coefficient(&mut s), coefficient(&mut s));
    let (ea, eb) = ((rng(&mut s) % 200) as i32 - 100, (rng(&mut s) % 200) as i32 - 100);
    let expected = round_pack(false, &str_mul(&ca.to_string(), &cb.to_string()), ea as i64 + eb as i64);
    assert_eq!(expected, mul_fin(false, ca, ea, cb, eb), "{ca}E{ea} * {cb}E{eb}");
  }
}

#[test]
fn sqrt_agrees_with_exact_root() {
  // the rounded root r must satisfy r' <= sqrt(x) < r'' for the neighbours of the sticky form,
  // checked exactly with string products: 10 * root + 1 is the 36-digit form, so compare squares
  let mut s = 13_u64;
  for _ in 0..20_000 {
    let c = coefficient(&mut s);
    if c == 0 {
      continue;
    }
    let e = (rng(&mut s) % 120) as i32 - 60;
    let v = n(&format!("{c}E{e}")).sqrt().unwrap();
    let V::Fin(_, r, re) = v.v() else { panic!() };
    // bracket: (r - 1/2 ulp)^2 < c * 10^e < (r + 1/2 ulp)^2, in units of 10^(2re - 2):
    // (10 r - 5)^2 < c * 10^(e - 2re + 2) <= (10 r + 5)^2 (equality only for exact roots)
    let lo = str_mul(&(10 * r - 5).to_string(), &(10 * r - 5).to_string());
    let hi = str_mul(&(10 * r + 5).to_string(), &(10 * r + 5).to_string());
    let k = e as i64 - 2 * re as i64 + 2;
    let x = if k >= 0 { format!("{c}{}", "0".repeat(k as usize)) } else { continue };
    assert_eq!(str_cmp(&lo, &x), Ordering::Less, "sqrt({c}E{e}) = {}", dbg(v));
    assert_ne!(str_cmp(&hi, &x), Ordering::Less, "sqrt({c}E{e}) = {}", dbg(v));
  }
}

#[test]
fn pack_rounds_once() {
  // exact
  assert_eq!(pack(Wide::Rounded(false, 125, -2, Ordering::Equal)), (V::Fin(false, 125, -2), false));
  // inexact, normal range: the rounded value is returned, flagged inexact
  assert_eq!(
    pack(Wide::Rounded(true, 3333333333333333333333333333333333, -34, Ordering::Greater)),
    (V::Fin(true, 3333333333333333333333333333333333, -34), true)
  );
  assert_eq!(
    pack(Wide::Rounded(false, 1, 0, Ordering::Less)),
    (V::Fin(false, 1_000_000_000_000_000_000_000_000_000_000_000, -33), true)
  );
  // subnormal range: rounded at the coarser position using the side (25E-6177 ties to even -> 2E-6176)
  assert_eq!(pack(Wide::Rounded(false, 25, -6177, Ordering::Equal)), (V::Fin(false, 2, -6176), true));
  assert_eq!(pack(Wide::Rounded(false, 25, -6177, Ordering::Greater)), (V::Fin(false, 3, -6176), true));
  assert_eq!(pack(Wide::Rounded(false, 25, -6177, Ordering::Less)), (V::Fin(false, 2, -6176), true));
  // underflow to zero and overflow
  assert_eq!(pack(Wide::Rounded(true, 0, i64::MIN, Ordering::Greater)), (V::Fin(true, 0, -6176), true));
  assert_eq!(pack(Wide::Inf(true)), (V::Inf(true), true));
  assert_eq!(pack(Wide::Digits(false, "1".repeat(40), 6110)), (V::Inf(false), true));
  assert_eq!(
    pack(Wide::Digits(false, "1".repeat(40), 6100)),
    (V::Fin(false, 1111111111111111111111111111111111, 6106), true)
  );
  assert_eq!(pack(Wide::NaN), (V::NaN(false), false));
  assert_eq!(
    pack(Wide::Digits(false, "123456789012345678901234567890123450".into(), -40)),
    (V::Fin(false, 1234567890123456789012345678901234, -38), true)
  );
}

#[test]
fn guards_domain_errors() {
  assert!(n("-1").sqrt().is_none());
  assert!(n("-1E-6176").sqrt().is_none());
  assert!(n("0").ln().is_none());
  assert!(n("-0").ln().is_none());
  assert!(n("-1").ln().is_none());
  assert!(n("-2.5").pow(&n("0.5")).is_none());
  assert!(n("-8").pow(&n("0.3333333333333333333333333333333333")).is_none());
}

#[test]
fn guards_zero_division() {
  // 0/0 is NaN, x/0 is infinite
  assert_eq!("+NaN", dbg(n("0") / n("0")));
  assert_eq!("+NaN", dbg(n("-0") / n("0.000")));
  assert_eq!("+Inf", dbg(n("1") / n("0")));
  assert_eq!("-Inf", dbg(n("-1") / n("0")));
  assert_eq!("-Inf", dbg(n("1") / n("-0")));
  assert_eq!("+0E-6176", dbg(n("1E-6176") / n("1E+10")));
  // 5E-6177 is a tie at the subnormal rounding position, half-even gives 0
  assert_eq!("+0E-6176", dbg(n("1E-6176") / n("2")));
  assert_eq!("+5E-6176", dbg(n("1E-6175") / n("2")));
  assert_eq!("-0E+0", dbg(n("-0") / n("7")));
  // overflowing quotients keep the sign
  assert_eq!("-Inf", dbg(n("-5769313964999E+5970") / n("8943970099E-4411")));
  assert_eq!("+Inf", dbg(n("-6E+3048") / n("-48031155018013756446E-4780")));
}

#[test]
fn guards_infinite_operands() {
  let inf = FeelNumber::infinite();
  let one = FeelNumber::one();
  assert_eq!("+Inf", dbg(inf + one));
  assert_eq!("+Inf", dbg(inf - one));
  assert_eq!("-Inf", dbg(one - inf));
  assert_eq!("+NaN", dbg(inf - inf));
  assert_eq!("+Inf", dbg(inf * one));
  assert_eq!("+NaN", dbg(inf * FeelNumber::zero()));
  assert_eq!("+Inf", dbg(inf / one));
  assert_eq!("+0E-6176", dbg(one / inf));
  assert_eq!("-0E-6176", dbg(-one / inf));
  assert_eq!("+NaN", dbg(inf / inf));
  assert_eq!("+NaN", dbg(inf % one));
  assert!(inf.sqrt().is_none());
  assert!(inf.exp().is_none());
  assert!(inf.ln().is_none());
  assert!((-inf).sqrt().is_none());
  assert!((-inf).ln().is_none());
  assert_eq!("+0E+0", dbg((-inf).exp().unwrap()));
  assert!(inf.pow(&one).is_none());
  assert_eq!("+0E+0", dbg(inf.pow(&-one).unwrap()));
  assert_eq!("+1E+0", dbg(one.pow(&inf).unwrap()));
  assert_eq!("+0E+0", dbg(n("0.5").pow(&inf).unwrap()));
  assert!(n("2").pow(&inf).is_none());
  assert_eq!("+0E+0", dbg(n("2").pow(&-inf).unwrap()));
  assert_eq!("+1E+0", dbg(inf.pow(&FeelNumber::zero()).unwrap()));
  assert!(inf.is_positive());
  assert!((-inf).is_negative());
  assert!(inf > one);
  assert!(inf.abs() == inf);
  assert_eq!("+Inf", dbg(inf.abs()));
  assert!(!inf.is_finite());
  assert!(!(n("0") / n("0")).is_finite());
  assert!(one.is_finite());
  // Display never panics
  assert_eq!("Infinity", inf.to_string());
  assert_eq!("-Infinity", (-inf).to_string());
  assert_eq!("NaN", (n("0") / n("0")).to_string());
  assert_eq!("0", (n("0") * n("1E-6000")).to_string());
  assert_eq!("0", n("0E+5000").to_string());
  assert_eq!("0.00", n("0.00").to_string());
}

#[test]
fn guards_overflow_and_underflow() {
  assert!(n("14300").exp().is_none());
  assert!(n("14150").exp().is_none());
  assert_eq!("+0E-6176", dbg(n("-14300").exp().unwrap()));
  assert_eq!("+0E-6176", dbg(n("-14250").exp().unwrap()));
  assert_eq!("+1414938539644841072829055748903542E-29", dbg(n("9.999999999999999999999999999999999E+6144").ln().unwrap()));
  assert_eq!("-1422076553433122614449511522413063E-29", dbg(n("1E-6176").ln().unwrap()));
  assert!(n("1E+6144").pow(&n("1E+6144")).is_none());
  assert_eq!("+0E-6176", dbg(n("1E-6176").pow(&n("1E+6144")).unwrap()));
  assert_eq!("+100000000000000000E+3055", dbg(n("1E+6144").sqrt().unwrap()));
  assert_eq!("+1E-3088", dbg(n("1E-6176").sqrt().unwrap()));
  assert_eq!("+1732050807568877293527446341505872E-3121", dbg(n("3E-6176").sqrt().unwrap()));
  assert_eq!("+3872983346207416885179265399782400E-3121", dbg(n("1.5E-6175").sqrt().unwrap()));
  assert_eq!("+Inf", dbg(n("1E+6144") * n("10")));
  assert_eq!("+Inf", dbg(n("1E+6144") + n("9E+6144")));
  assert_eq!("+0E-6176", dbg(n("1E-6176") * n("0.1")));
  assert_eq!("+1E-6176", dbg(n("1E-6176") * n("0.6")));
  assert_eq!("+2E-6176", dbg(n("1E-6176") * n("1.5")));
  assert_eq!("+2E-6176", dbg(n("3E-6176") * n("0.5")));
  assert_eq!("+2E-6176", dbg(n("5E-6176") * n("0.5")));
}

#[test]
fn pow_edge_cases() {
  assert_eq!("+1E+0", dbg(n("0").pow(&n("0")).unwrap()));
  assert_eq!("+1E+0", dbg(n("-0").pow(&n("0")).unwrap()));
  assert!(n("0").pow(&n("-1")).is_none());
  assert!(n("-0").pow(&n("-3")).is_none());
  assert_eq!("+0E+0", dbg(n("0").pow(&n("0.5")).unwrap()));
  assert_eq!("-0E+0", dbg(n("-0").pow(&n("3")).unwrap()));
  assert_eq!("+0E+0", dbg(n("-0").pow(&n("2")).unwrap()));
  assert_eq!("-8", n("-2").pow(&n("3")).unwrap().to_string());
  assert_eq!("-125E-3", dbg(n("-2").pow(&n("-3")).unwrap()));
  assert_eq!("+4E+0", dbg(n("-2").pow(&n("2")).unwrap()));
  assert_eq!("+1E+0", dbg(n("1").pow(&n("1E+6144")).unwrap()));
  assert_eq!("+1E+0", dbg(n("1.00").pow(&n("-1E+6144")).unwrap()));
  assert_eq!("+1E+0", dbg(n("1").pow(&n("1E+10")).unwrap()));
  assert_eq!("+1E+0", dbg(n("1.000").pow(&n("-2147483648")).unwrap()));
  // non-integer exponent: padded to 34 digits like Intel
  assert_eq!("+2000000000000000000000000000000000E-33", dbg(n("4").pow(&n("0.5")).unwrap()));
  assert_eq!("+1414213562373095048801688724209698E-33", dbg(n("2").pow(&n("0.5")).unwrap()));
  // integer exponent outside i32 (or i32::MIN): exp(y ln x), correctly rounded
  assert!(n("2").pow(&n("1E+10")).is_none());
  assert_eq!("+0E-6176", dbg(n("0.5").pow(&n("1E+10")).unwrap()));
  assert_eq!("+0E-6176", dbg(n("2").pow(&n("-1E+10")).unwrap()));
  // underflow is returned without sign correction, like Intel
  assert_eq!("+0E-6176", dbg(n("-0.5").pow(&n("2147483649")).unwrap()));
  assert_eq!(
    "+1000000000000000000000000214748365E-33",
    dbg(n("0.9999999999999999999999999999999999").pow(&n("-2147483648")).unwrap())
  );
  assert_eq!(
    "+1000000000000000000000002147483648E-33",
    dbg(n("1.000000000000000000000000000000001").pow(&n("2147483648")).unwrap())
  );
  // correctly rounded where Intel is 1 ulp off (see tests/pow.rs test_pow_002)
  assert_eq!("41959.85737359436186095331070746801", n("12.2384283").pow(&n("4.25")).unwrap().to_string());
  // near tie: 0.99999999999999999999999999999999994999... (dashu's own powf gives 1)
  assert_eq!(
    "+9999999999999999999999999999999999E-34",
    dbg(n("0.9999999999999999999999999999999999").pow(&n("0.5")).unwrap())
  );
  // exact tie: 16^-12.5 = 2^-50 = 8.8817841970012523233890533447265625E-16, rounded half-even
  // (dashu's own powf hangs on it, Intel returns ...561, 3 ulp below the true value)
  let t = std::time::Instant::now();
  assert_eq!("+8881784197001252323389053344726562E-49", dbg(n("16").pow(&n("-12.5")).unwrap()));
  assert!(t.elapsed().as_secs() < 5);
  // integer exponent: exact binary exponentiation keeps the cohort like Intel
  assert_eq!("+1125899906842624E+0", dbg(n("4").pow(&n("25")).unwrap()));
  // exact results of fractional powers, padded to 34 digits (Intel: 9.999999999999999999999999999999999
  // for 100^0.5 and 9.999999999999999999999999999999996E-51 for 1E-100^0.5)
  assert_eq!("+2000000000000000000000000000000000E-33", dbg(n("16").pow(&n("0.25")).unwrap()));
  assert_eq!("+5000000000000000000000000000000000E-34", dbg(n("16").pow(&n("-0.25")).unwrap()));
  assert_eq!("+1000000000000000000000000000000000E-32", dbg(n("100").pow(&n("0.5")).unwrap()));
  assert_eq!("+1000000000000000000000000000000000E-83", dbg(n("1E-100").pow(&n("0.5")).unwrap()));
  assert_eq!("+1000000000000000000000000000000000E+17", dbg(n("1E+100").pow(&n("0.5")).unwrap()));
}

#[test]
fn round_half_down_ties() {
  // dsntk's algorithm (same on every backend): subtract 0.1 (add for negative values)
  // at the scale position, then round half away from zero; ties go toward zero
  assert_eq!("+2E+0", dbg(n("2.5").round_half_down(0).unwrap()));
  assert_eq!("-2E+0", dbg(n("-2.5").round_half_down(0).unwrap()));
  assert_eq!("+2E+0", dbg(n("2.49").round_half_down(0).unwrap()));
  assert_eq!("+0E+0", dbg(n("0.5").round_half_down(0).unwrap()));
  assert_eq!(
    "+1234567890123456789012345678901234E+0",
    dbg(n("1234567890123456789012345678901234.5").round_half_down(0).unwrap())
  );
  assert_eq!("+999E+0", dbg(n("999.5").round_half_down(0).unwrap()));
  assert_eq!("+11E-1", dbg(n("1.15").round_half_down(1).unwrap()));
  assert_eq!("-11E-1", dbg(n("-1.15").round_half_down(1).unwrap()));
  assert_eq!("+12E-1", dbg(n("1.25").round_half_down(1).unwrap()));
  assert_eq!("+55E-1", dbg(n("5.55").round_half_down(1).unwrap()));
  // consequences of the 0.1 subtraction (identical on the Intel backend)
  assert_eq!("+2E+0", dbg(n("2.51").round_half_down(0).unwrap()));
  assert_eq!("-2E+0", dbg(n("-2.51").round_half_down(0).unwrap()));
  assert_eq!("+11E-1", dbg(n("1.151").round_half_down(1).unwrap()));
  assert_eq!("+55E-1", dbg(n("5.5500000000000000000000000000000001").round_half_down(1).unwrap()));
  assert!(n("1.5").round_half_down(7000).is_err());
  // extreme scales never panic (Intel's `ilogb + scale` overflows in debug builds)
  assert!(n("0").ceiling(i32::MIN).is_err());
  assert!(n("1E+6000").floor(i32::MAX).is_err());
  assert!(n("1").round_up(i32::MIN).is_err());
}

#[test]
fn subnormal_results_are_single_rounded() {
  // quotient 2.5E-6177 is a tie at the subnormal rounding position: half-even gives 2E-6176
  assert_eq!("+2E-6176", dbg(n("5E-6176") / n("2")));
  // slightly above and below the tie
  assert_eq!("+3E-6176", dbg(n("5E-6176") / n("1.999999999999999999999999999999999")));
  assert_eq!("+2E-6176", dbg(n("5E-6176") / n("2.000000000000000000000000000000001")));
  // exact results keep the ideal exponent
  assert_eq!("+2E-6176", dbg(n("1E-6175") / n("5")));
  assert_eq!("+1E-6143", dbg(n("1E-6143") / n("1")));
}

#[test]
fn sqrt_near_half_way() {
  // the integer root 10^34 - 1 has the remainder 10^34 - 1 (equal to the root): the true root
  // is just below the half-way point and must round down (dashu's FBig::sqrt rounds up here)
  assert_eq!("+9999999999999999999999999999999999E-34", dbg(n("0.9999999999999999999999999999999999").sqrt().unwrap()));
  assert_eq!("+9999999999999999999999999999999999E-17", dbg(n("9999999999999999999999999999999999").sqrt().unwrap()));
  assert_eq!("+9999999999999999999999999999999999E-25", dbg(n("999999999999999999.99999999999999994").sqrt().unwrap()));
  // exact roots get the ideal exponent floor(e / 2), inexact ones 34 digits
  assert_eq!("+2E+0", dbg(n("4").sqrt().unwrap()));
  assert_eq!("+20E+0", dbg(n("400").sqrt().unwrap()));
  assert_eq!("+10E-1", dbg(n("1.00").sqrt().unwrap()));
  assert_eq!("+15E-1", dbg(n("2.25").sqrt().unwrap()));
  assert_eq!("+1414213562373095048801688724209698E-33", dbg(n("2").sqrt().unwrap()));
  assert_eq!("+3162277660168379331998893544432719E-34", dbg(n("0.1").sqrt().unwrap()));
  assert_eq!("+0E+0", dbg(n("0").sqrt().unwrap()));
  assert_eq!("-0E-2", dbg(n("-0E-3").sqrt().unwrap()));
}

/// Scaling by 10^n pads the coefficient to the full 34 digits before overflowing,
/// so every representable value stays finite (mathematically correct per the oracle).
/// Intel's `bid128_scalbn` overflows some of these to infinity (its coefficient padding
/// stops at ~10^33); that quirk is not reproduced.
#[test]
fn scalbn_overflow_pads_to_34_digits() {
  assert_eq!("+9999999999999990000000000000000000E+6111", dbg(n("999999999999999E+6130")));
  assert_eq!("+9999999999999990000000000000000000E+6111", dbg(FeelNumber::new(999999999999999, -6130)));
  assert_eq!("+999999999999999000000000000000000E+6111", dbg(FeelNumber::new(999999999999999, -6129)));
  assert_eq!("+1000000000000000000000000000000000E+6111", dbg(FeelNumber::new(1, -6144)));
  assert_eq!("+100000000000000000000000000000000E+6111", dbg(FeelNumber::new(1, -6143)));
  assert_eq!("+9999999999999990000000000000000000E+6102", dbg(n("999999999999999E+6121").floor(9).unwrap()));
  assert_eq!("+999999999999990000000000000000000E+6102", dbg(n("99999999999999E+6121").floor(9).unwrap()));
  // still infinite when 34 digits are not enough
  assert_eq!("+Inf", dbg(FeelNumber::new(8999999999999999999, -6127)));
  assert_eq!("+8999999999999999999000000000000000E+6111", dbg(FeelNumber::new(8999999999999999999, -6126)));
  // the scale is clamped to 6144 like Intel's bid128_scalbn, so this is still 1E+6144
  assert_eq!("+1000000000000000000000000000000000E+6111", dbg(FeelNumber::new(1, -6145)));
  assert_eq!("+Inf", dbg(FeelNumber::new(10, -6144)));
  assert_eq!("-Inf", dbg(FeelNumber::new(-10, -6144)));
  assert_eq!("+1E-6176", dbg(FeelNumber::new(1, i32::MAX)));
  assert_eq!("+1000000000000000000000000000000000E+6111", dbg(FeelNumber::new(1, i32::MIN)));
}

/// Regression rows from the three-way fuzz (intel vs fastnum vs dashu, 200k iterations
/// per op, mpmath/decimal oracle): scale-based rounding ops on values near the overflow
/// edge, where Intel's scalbn quirk gives infinity. Expected values are the oracle ones.
#[test]
fn scale_rounding_near_overflow_edge_stays_finite() {
  assert_eq!("-9999999999999999999999900000000000E+6087", dbg(n("-99999999999999999999999E+6098").ceiling(24).unwrap()));
  assert_eq!("+9999999999999990000000000000000000E+6102", dbg(n("999999999999999E+6121").floor(9).unwrap()));
  assert_eq!(
    "+9999999999999999999999999999999000E+6088",
    dbg(n("9999999999999999999999999999999E+6091").round_half_down(23).unwrap())
  );
  assert_eq!("+9999999999999999999990000000000000E+6089", dbg(n("999999999999999999999E+6102").round_up(22).unwrap()));
  assert_eq!("-9999999999999999900000000000000000E+6090", dbg(n("-99999999999999999E+6107").round_down(21).unwrap()));
  assert_eq!("+9999999999999999999000000000000000E+6104", dbg(n("9999999999999999999E+6119").round_half_up(7).unwrap()));
  // chain steps (the input is an intermediate value in a longer op chain)
  assert_eq!("+9999999999999990000000000000000000E+6097", dbg(n("99999999999999900000E+6111").ceiling(14).unwrap()));
  assert_eq!(
    "+9999999999999999999999999999999900E+6102",
    dbg(n("99999999999999999999999999999999E+6104").round_half_up(9).unwrap())
  );
  // feel-evaluator corpus case: round up(999999999999999E+6116, 14) and its five siblings
  let x = n("999999999999999E+6116");
  for r in [x.round_up(14), x.round_down(14), x.round_half_up(14), x.round_half_down(14), x.floor(14), x.ceiling(14)] {
    assert_eq!("+9999999999999990000000000000000000E+6097", dbg(r.unwrap()));
  }
  // the same values are finite when they come from parsing, so the ops are consistent
  assert_eq!(n("9999999999999990000000000000000000E+6097"), x.round_up(14).unwrap());
  assert_eq!(x, x.round_up(14).unwrap());
}

#[test]
fn exp_ln_cohorts_follow_intel() {
  assert_eq!("+2E+0", dbg(n("0.6931471805599453094172321214581766").exp().unwrap()));
  assert_eq!("+5E-1", dbg(n("-0.6931471805599453094172321214581766").exp().unwrap()));
  assert_eq!("+1000000000000000000000000000000000E-33", dbg(n("1E-34").exp().unwrap()));
  assert_eq!("+1E+0", dbg(n("5E-35").exp().unwrap()));
  assert_eq!("+2E+0", dbg(n("7.389056098930650227230427460575008").ln().unwrap()));
  assert_eq!("+0E+0", dbg(n("1.000").ln().unwrap()));
  assert_eq!("+2718281828459045235360287471352662E-33", dbg(n("1").exp().unwrap()));
  // integral after rounding to 34 digits, but not an integer in binary128
  assert_eq!("+2192641593867934774425562824143910E-1", dbg(n("74.46783").exp().unwrap()));
  // not dyadic: always 34 digits
  assert_eq!("+1599999999999999999999999998813750E-32", dbg(n("2.772588722239781237668928485091300").exp().unwrap()));
}

#[test]
fn nan_sign_follows_intel() {
  let nan = n("0") / n("0");
  assert_eq!("+NaN", dbg(nan));
  assert_eq!("-NaN", dbg(-nan));
  assert_eq!("+NaN", dbg(-(-nan)));
  assert_eq!("+NaN", dbg((-nan).abs()));
  // a NaN operand is propagated with its sign, the first one wins
  assert_eq!("-NaN", dbg(-nan + n("1")));
  assert_eq!("-NaN", dbg(n("1") + -nan));
  assert_eq!("-NaN", dbg(n("1") - -nan));
  assert_eq!("-NaN", dbg(-nan - nan));
  assert_eq!("+NaN", dbg(nan - -nan));
  assert_eq!("-NaN", dbg(n("2") * -nan));
  assert_eq!("-NaN", dbg(-nan / n("2")));
  assert_eq!("-NaN", dbg(n("2") / -nan));
  assert_eq!("-NaN", dbg(-nan % n("2")));
  assert_eq!("-NaN", dbg((-nan).round(&n("2"))));
  assert_eq!("-NaN", dbg((-nan).trunc()));
  assert_eq!("-NaN", dbg((-nan).frac()));
  assert_eq!("-NaN", dbg((-nan).floor(0).unwrap()));
  assert!((-nan).floor(2).is_err());
  assert_eq!("-NaN", dbg((-nan).round_half_down(0).unwrap()));
  assert!((-nan).exp().is_none());
  assert_eq!(FeelNumber::new(0, 0).partial_cmp(&-nan), Some(Ordering::Less));
  // invalid operations give +NaN whatever the operands' signs
  assert_eq!("+NaN", dbg(-FeelNumber::infinite() + FeelNumber::infinite()));
  assert_eq!("+NaN", dbg(n("-0") / n("-0")));
  assert_eq!("+NaN", dbg(-FeelNumber::infinite() * n("0")));
  assert_eq!("NaN", (-nan).to_string());
  assert!(!(-nan).is_finite());
}

#[test]
fn parse_reports_rounding() {
  assert_eq!((n("1.5").v(), false), FeelNumber::from_str_reporting_rounding("1.5").map(|(n, x)| (n.v(), x)).unwrap());
  assert!(FeelNumber::from_str_reporting_rounding("1.00000000000000000000000000000000001").unwrap().1);
  assert!(FeelNumber::from_str_reporting_rounding("1E-7000").unwrap().1);
  assert!(FeelNumber::from_str_reporting_rounding("1.5E-6176").unwrap().1);
  assert!(!FeelNumber::from_str_reporting_rounding("1E-6176").unwrap().1);
  assert!(!FeelNumber::from_str_reporting_rounding("1E+6144").unwrap().1);
  assert!(FeelNumber::from_str_reporting_rounding("1E+6145").is_err());
  assert!(FeelNumber::from_str_reporting_rounding("x").is_err());
}
