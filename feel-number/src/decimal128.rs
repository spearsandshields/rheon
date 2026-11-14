//! # decimal128 core of the FEEL number
//!
//! Implementation of the FEEL number type with the semantics of IEEE 754-2008 **decimal128**
//! (DMN 1.6 §10.3.2.3.1): 34 significant digits, rounding half-even, exponent (of the
//! coefficient) in range `-6176..=6111`, cohorts preserved.
//!
//! The value is stored in a backend-independent, `Copy` form [V]: sign, `u128` coefficient
//! (< 10^34) and `i32` exponent, plus infinity and NaN. Everything whose exact result has a
//! bounded number of digits is computed here, directly on that triple, in `u128` when the
//! intermediate fits and in a tiny 256-bit integer ([U256]) otherwise: `+ - *`, square root,
//! rounding to a scale, comparisons, parsing, formatting, conversions, the
//! integer-exponent power. Only the operations with an unbounded exact
//! result (`/`, `exp`, `ln`, `pow` with a non-integer exponent) are delegated to a [Backend],
//! which returns a correctly rounded value together with the side the exact value lies on; that
//! side becomes a sticky digit and the value is passed **once** through [round_pack], which also
//! applies the decimal128 exponent range (subnormals, clamping and overflow to infinity).
//!
//! The backend is `backend_dashu.rs`, on dashu-float.

use crate::errors::*;
use dsntk_common::{DsntkError, Jsonify, Result};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

/// The backend of the operations with an unbounded exact result.
type B = crate::backend_dashu::Dashu;

/// Minimum scale accepted by the scale-based rounding functions (FEEL: `-6111..=6176`).
const MIN_SCALE: i32 = -6111;

/// Maximum scale accepted by the scale-based rounding functions (FEEL: `-6111..=6176`).
const MAX_SCALE: i32 = 6176;

/// Precision of decimal128 (number of significant digits).
pub(crate) const PRECISION: usize = 34;

/// Minimum exponent of the coefficient in decimal128.
const EXP_MIN: i64 = -6176;

/// Maximum exponent of the coefficient in decimal128.
const EXP_MAX: i64 = 6111;

/// 10^34
const TEN_POW_34: u128 = 10_000_000_000_000_000_000_000_000_000_000_000;

/// `t = y ln x` above which `x^y = e^t` overflows decimal128 (the exact limit is about 14149.5).
pub(crate) const POW_T_OVERFLOW: f64 = 14200.0;

/// `t = y ln x` below which `x^y = e^t` rounds to zero in decimal128 (the exact limit is about -14221.2).
pub(crate) const POW_T_UNDERFLOW: f64 = -14250.0;

/// Working precisions of [pow_exp_log]: the first one leaves 11 digits to decide the rounding,
/// the later ones are only used when the result is very close to a rounding boundary.
const POW_PRECISIONS: [usize; 4] = [45, 60, 90, 140];

/// Unpacked decimal128 value.
#[derive(Copy, Clone, Debug, PartialEq)]
enum V {
  /// Not a number: the result of an invalid operation, propagated by every operation. It has
  /// no sign; FEEL has no such number (DMN 1.6 §10.3.2.3.1), so the evaluators give null.
  NaN,
  /// Infinity with sign (`true` = negative).
  Inf(bool),
  /// Finite number: sign (`true` = negative), coefficient (< 10^34), exponent.
  Fin(bool, u128, i32),
}

/// The rounding modes of the FEEL rounding functions (DMN 1.6 Table 77).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RoundingMode {
  /// To the nearest, a tie to the even neighbour (`decimal`).
  HalfEven,
  /// To the nearest, a tie away from zero (`round half up`).
  HalfUp,
  /// To the nearest, a tie toward zero (`round half down`).
  HalfDown,
  /// Away from zero (`round up`).
  Up,
  /// Toward zero (`round down`).
  Down,
  /// Toward positive infinity (`ceiling`).
  Ceiling,
  /// Toward negative infinity (`floor`).
  Floor,
}

/// Result of a backend operation, in a form the core rounds exactly once.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Wide {
  /// Not a number (domain error).
  NaN,
  /// Infinity with sign (`true` = negative), e.g. an overflow of the backend's own range.
  Inf(bool),
  /// `±c · 10^e` with `c < 10^34`: the result correctly rounded (half-even) to [PRECISION]
  /// digits, or the exact result. `side` is where the exact magnitude lies relative to `c · 10^e`:
  /// `Equal` when the result is exact, `Less` when it was rounded up in magnitude, `Greater`
  /// when it was rounded down (a `c` of zero with `Greater` is an underflow to zero).
  Rounded(bool, u128, i64, Ordering),
  /// `±digits · 10^e` for any number of digits: the exact result, or an approximation whose
  /// error is far below the 35th significant digit. Rounded once by the core.
  Digits(bool, String, i64),
}

/// Result of one round of `x^y = exp(y ln x)` at a working precision.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PowStep {
  /// `t = y ln x` is above [POW_T_OVERFLOW]: the result is infinite.
  Overflow,
  /// `t = y ln x` is below [POW_T_UNDERFLOW]: the result is zero.
  Underflow,
  /// Magnitude of the result as `digits · 10^e`, with at most `precision` digits (trailing
  /// zeros may be stripped), each of `ln`, `*` and `exp` correctly rounded to `precision` digits.
  Value(String, i64),
  /// The result could not be computed (e.g. a domain error inside the backend).
  Undefined,
}

/// The operations a backend provides: those whose exact result has no bounded number of digits.
///
/// All arguments are finite, non-zero magnitudes `c · 10^e` with `c < 10^34`; signs, zeros,
/// infinities, NaN, the decimal128 exponent range and the cohorts are handled by the core
/// before and after the backend is called. The backend must never panic: a domain error is
/// reported as [Wide::NaN] or [PowStep::Undefined].
pub(crate) trait Backend {
  /// `a / b` for `a = ca · 10^ea`, `b = cb · 10^eb`, correctly rounded to [PRECISION] digits.
  fn div(ca: u128, ea: i32, cb: u128, eb: i32) -> Wide;

  /// `e^x` for `x = ±c · 10^e` (`neg` is the sign of `x`), correctly rounded to [PRECISION]
  /// digits.
  fn exp(neg: bool, c: u128, e: i32) -> Wide;

  /// `ln(c · 10^e)`, correctly rounded to [PRECISION] digits.
  fn ln(c: u128, e: i32) -> Wide;

  /// One round of `x^y = exp(t)`, `t = y ln x`, for `x = cx · 10^ex` and `y = ±cy · 10^ey`
  /// (`sy` is the sign of `y`), with `ln`, `*` and `exp` each correctly rounded to `precision`
  /// digits. When `t` (any reasonable approximation of it, e.g. `f64`) is above [POW_T_OVERFLOW]
  /// or below [POW_T_UNDERFLOW] the backend returns [PowStep::Overflow] / [PowStep::Underflow]
  /// without evaluating `exp`.
  fn pow_step(cx: u128, ex: i32, sy: bool, cy: u128, ey: i32, precision: usize) -> PowStep;
}

/// The powers of ten that fit in `u128`: `POW10[n] = 10^n`.
const POW10: [u128; 39] = {
  let mut powers = [1_u128; 39];
  let mut n = 1;
  while n < 39 {
    powers[n] = powers[n - 1] * 10;
    n += 1;
  }
  powers
};

fn pow10(n: u32) -> u128 {
  POW10[n as usize]
}

/// Number of decimal digits of `c` (1 for zero), without a 128-bit division.
pub(crate) fn digit_count(c: u128) -> u32 {
  if c == 0 {
    return 1;
  }
  // With `bits` significant bits, `d = ⌊bits · 1233 / 4096⌋` (1233 / 4096 is just below log10 2)
  // is the number of digits or one less; one comparison decides.
  let bits = 128 - c.leading_zeros();
  let d = (bits * 1233) >> 12;
  d + u32::from(c >= POW10[d as usize])
}

/// Unsigned 256-bit integer: just enough for the exact intermediate values of this module
/// (sums and products of two 34-digit coefficients, square root radicands).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct U256 {
  hi: u128,
  lo: u128,
}

impl U256 {
  const ZERO: U256 = U256 { hi: 0, lo: 0 };

  fn from_u128(lo: u128) -> U256 {
    U256 { hi: 0, lo }
  }

  fn is_zero(self) -> bool {
    self.hi == 0 && self.lo == 0
  }

  /// Full product of two `u128` values.
  fn mul(a: u128, b: u128) -> U256 {
    const MASK: u128 = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & MASK);
    let (b1, b0) = (b >> 64, b & MASK);
    let (p00, p01, p10, p11) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
    let (mid, mid_carry) = p01.overflowing_add(p10);
    let (lo, carry) = p00.overflowing_add(mid << 64);
    let hi = p11 + (mid >> 64) + ((mid_carry as u128) << 64) + carry as u128;
    U256 { hi, lo }
  }

  fn checked_add(self, other: U256) -> Option<U256> {
    let (lo, carry) = self.lo.overflowing_add(other.lo);
    let hi = self.hi.checked_add(other.hi)?.checked_add(carry as u128)?;
    Some(U256 { hi, lo })
  }

  fn checked_sub(self, other: U256) -> Option<U256> {
    if self < other {
      return None;
    }
    let (lo, borrow) = self.lo.overflowing_sub(other.lo);
    Some(U256 {
      hi: self.hi - other.hi - borrow as u128,
      lo,
    })
  }

  fn checked_mul_u128(self, m: u128) -> Option<U256> {
    let lo = U256::mul(self.lo, m);
    let hi = U256::mul(self.hi, m);
    if hi.hi != 0 {
      return None;
    }
    Some(U256 {
      hi: lo.hi.checked_add(hi.lo)?,
      lo: lo.lo,
    })
  }

  fn checked_mul_pow10(self, n: u32) -> Option<U256> {
    let mut x = self;
    let mut n = n;
    while n > 0 {
      let k = n.min(38);
      x = x.checked_mul_u128(pow10(k))?;
      n -= k;
    }
    Some(x)
  }

  fn shr(self, n: u32) -> U256 {
    if n == 0 {
      self
    } else if n >= 256 {
      U256::ZERO
    } else if n >= 128 {
      U256 { hi: 0, lo: self.hi >> (n - 128) }
    } else {
      U256 {
        hi: self.hi >> n,
        lo: (self.lo >> n) | (self.hi << (128 - n)),
      }
    }
  }

  /// Number of significant bits.
  fn bits(self) -> u32 {
    if self.hi != 0 { 256 - self.hi.leading_zeros() } else { 128 - self.lo.leading_zeros() }
  }

  /// Remainder of the division by `d`, for `0 < d < 2^127` (bit by bit; the partial remainder
  /// stays below `2d`, so it fits in `u128`).
  fn rem_u128(self, d: u128) -> u128 {
    debug_assert!(d != 0 && d < 1 << 127, "divisor out of range");
    let mut rem: u128 = 0;
    for i in (0..self.bits()).rev() {
      let bit = if i >= 128 { (self.hi >> (i - 128)) & 1 } else { (self.lo >> i) & 1 };
      rem = (rem << 1) | bit;
      if rem >= d {
        rem -= d;
      }
    }
    rem
  }

  /// Quotient and remainder of the division by a `u64`.
  fn div_rem_u64(self, d: u64) -> (U256, u64) {
    let d = d as u128;
    let mut rem: u128 = 0;
    let mut limbs = [self.hi >> 64, self.hi & u64::MAX as u128, self.lo >> 64, self.lo & u64::MAX as u128];
    for limb in limbs.iter_mut() {
      let cur = (rem << 64) | *limb;
      *limb = cur / d;
      rem = cur % d;
    }
    (
      U256 {
        hi: (limbs[0] << 64) | limbs[1],
        lo: (limbs[2] << 64) | limbs[3],
      },
      rem as u64,
    )
  }

  fn to_decimal(self) -> String {
    const CHUNK: u64 = 10_000_000_000_000_000_000;
    if self.is_zero() {
      return "0".to_string();
    }
    let mut chunks = vec![];
    let mut x = self;
    while !x.is_zero() {
      let (q, r) = x.div_rem_u64(CHUNK);
      chunks.push(r);
      x = q;
    }
    let mut s = chunks.pop().unwrap_or(0).to_string();
    while let Some(chunk) = chunks.pop() {
      s.push_str(&format!("{chunk:019}"));
    }
    s
  }
}

/// Integer square root with the information whether the remainder is zero.
///
/// The top bits that fit in `u128` are handled by `u128::isqrt`, the rest two bits at a time
/// (digit-by-digit method). The radicand must be below 2^250 (it has at most 70 digits here),
/// so the root, the remainder and the candidates all fit in `u128`.
fn isqrt_rem(r: U256) -> (u128, bool) {
  let bits = r.bits();
  debug_assert!(bits <= 250, "radicand too large");
  let shift = if bits > 128 { (bits - 128).div_ceil(2) * 2 } else { 0 };
  let top = r.shr(shift).lo;
  let mut root = top.isqrt();
  let mut rem = top - root * root;
  let mut i = shift / 2;
  while i > 0 {
    i -= 1;
    rem = (rem << 2) | (r.shr(2 * i).lo & 3);
    let candidate = (root << 2) | 1;
    root <<= 1;
    if rem >= candidate {
      rem -= candidate;
      root |= 1;
    }
  }
  (root, rem != 0)
}

/// Rounds a number given as a string of decimal digits and an exponent
/// to decimal128 (34 digits, half-even, exponent range, overflow to infinity).
/// Returns the value and the flag indicating if the rounding was inexact.
fn round_pack(neg: bool, digits: &str, exp: i64) -> (V, bool) {
  let digits = digits.trim_start_matches('0');
  let n = digits.len() as i64;
  if n == 0 {
    // zero, the exponent is clamped to the allowed range
    return (V::Fin(neg, 0, exp.clamp(EXP_MIN, EXP_MAX) as i32), false);
  }
  let mut exp = exp;
  let drop = (n - PRECISION as i64).max(EXP_MIN - exp);
  let (mut coefficient, inexact) = if drop > 0 {
    let (kept, round_digit, sticky) = if drop <= n {
      let k = (n - drop) as usize;
      let tail = &digits[k..];
      (&digits[..k], tail.as_bytes()[0], tail[1..].bytes().any(|b| b != b'0'))
    } else {
      // all digits are dropped and the rounding digit is an implicit zero
      ("", b'0', true)
    };
    let mut c: u128 = if kept.is_empty() { 0 } else { kept.parse().unwrap_or(0) };
    let up = round_digit > b'5' || (round_digit == b'5' && (sticky || c % 2 == 1));
    if up {
      c += 1;
    }
    exp += drop;
    if c == TEN_POW_34 {
      c /= 10;
      exp += 1;
    }
    (c, round_digit != b'0' || sticky)
  } else {
    (digits.parse::<u128>().unwrap_or(0), false)
  };
  if coefficient == 0 {
    return (V::Fin(neg, 0, exp.clamp(EXP_MIN, EXP_MAX) as i32), inexact);
  }
  if exp > EXP_MAX {
    let pad = exp - EXP_MAX;
    if digit_count(coefficient) as i64 + pad <= PRECISION as i64 {
      coefficient *= pow10(pad as u32);
      exp = EXP_MAX;
    } else {
      return (V::Inf(neg), true);
    }
  }
  (V::Fin(neg, coefficient, exp as i32), inexact)
}

/// Same as [round_pack] for a coefficient that fits in `u128` (at most 39 digits),
/// without any string conversion.
fn round_pack_u128(neg: bool, c: u128, exp: i64) -> (V, bool) {
  if c < TEN_POW_34 && (EXP_MIN..=EXP_MAX).contains(&exp) {
    // at most 34 digits and the exponent in range: already a decimal128 value, exact
    return (V::Fin(neg, c, exp as i32), false);
  }
  if c == 0 {
    return (V::Fin(neg, 0, exp.clamp(EXP_MIN, EXP_MAX) as i32), false);
  }
  let n = digit_count(c) as i64;
  let drop = (n - PRECISION as i64).max(EXP_MIN - exp);
  let (mut coefficient, mut exp, inexact) = if drop > 0 {
    let (q, round_digit, sticky) = if drop > n {
      // all digits are dropped and the rounding digit is an implicit zero
      (0, 0, true)
    } else if drop == n {
      // the rounding digit is the leading digit
      let p = pow10((n - 1) as u32);
      (0, c / p, !c.is_multiple_of(p))
    } else {
      let p = pow10(drop as u32);
      let (q, r) = (c / p, c % p);
      let p1 = p / 10;
      (q, r / p1, r % p1 != 0)
    };
    let up = round_digit > 5 || (round_digit == 5 && (sticky || !q.is_multiple_of(2)));
    let mut c = if up { q + 1 } else { q };
    let mut e = exp + drop;
    if c == TEN_POW_34 {
      c /= 10;
      e += 1;
    }
    (c, e, round_digit != 0 || sticky)
  } else {
    (c, exp, false)
  };
  if coefficient == 0 {
    return (V::Fin(neg, 0, exp.clamp(EXP_MIN, EXP_MAX) as i32), inexact);
  }
  if exp > EXP_MAX {
    let pad = exp - EXP_MAX;
    if digit_count(coefficient) as i64 + pad <= PRECISION as i64 {
      coefficient *= pow10(pad as u32);
      exp = EXP_MAX;
    } else {
      return (V::Inf(neg), true);
    }
  }
  (V::Fin(neg, coefficient, exp as i32), inexact)
}

/// Converts the result of a backend operation to decimal128 with a single rounding.
///
/// A [Wide::Rounded] value that is inexact comes with the side of the exact result; the value
/// is re-packed as `c ∓ 0.01 ulp` (36 digits, exact in `u128`) and rounded once by
/// [round_pack_u128]: in the normal range this gives `c` back, in the subnormal range
/// (rounding position above the last digit of `c`) the sticky digits resolve the ties
/// the way the exact result would, so there is no double rounding. `0.01 ulp` (not `0.1`)
/// is needed because a result that is a power of ten (`1.000…0`) has a ten times finer
/// spacing below it.
fn pack(w: Wide) -> (V, bool) {
  match w {
    Wide::NaN => (V::NaN, false),
    Wide::Inf(neg) => (V::Inf(neg), true),
    Wide::Rounded(neg, c, e, Ordering::Equal) => round_pack_u128(neg, c, e),
    Wide::Rounded(neg, c, e, side) => {
      if c == 0 || digit_count(c) > PRECISION as u32 {
        // underflow to zero, or a value the backend did not round to 34 digits
        return (round_pack_u128(neg, c, e).0, true);
      }
      // pad to the full precision first, so that the ulp is right
      let pad = PRECISION as u32 - digit_count(c);
      let c = c * pow10(pad);
      let c = if side == Ordering::Less { c * 100 - 1 } else { c * 100 + 1 };
      (round_pack_u128(neg, c, e - pad as i64 - 2).0, true)
    }
    Wide::Digits(neg, digits, e) => round_pack(neg, &digits, e),
  }
}

/// Moves the exponent of an exact result as close as possible to the ideal exponent.
fn ideal(v: V, inexact: bool, ideal: i32) -> V {
  match v {
    V::Fin(neg, mut c, mut e) if !inexact => {
      if c == 0 {
        return V::Fin(neg, 0, (ideal as i64).clamp(EXP_MIN, EXP_MAX) as i32);
      }
      while e < ideal && e < EXP_MAX as i32 && c % 10 == 0 {
        c /= 10;
        e += 1;
      }
      while e > ideal && e > EXP_MIN as i32 && digit_count(c) < PRECISION as u32 {
        c *= 10;
        e -= 1;
      }
      V::Fin(neg, c, e)
    }
    other => other,
  }
}

fn is_zero_v(v: V) -> bool {
  matches!(v, V::Fin(_, 0, _))
}

fn negate(v: V) -> V {
  match v {
    V::NaN => V::NaN,
    V::Inf(s) => V::Inf(!s),
    V::Fin(s, c, e) => V::Fin(!s, c, e),
  }
}

/// Exact sum of two finite values, rounded once to decimal128.
///
/// Three paths, all exact: the aligned operands fit in `u128` (the common case);
/// the smaller operand lies entirely below the 36th digit of the larger one, so it only
/// acts as a sticky digit (`larger * 10^k ± 1`, 36 digits); otherwise the aligned sum
/// has fewer than 70 digits and is computed in [U256].
fn add_fin(sa: bool, ca: u128, ea: i32, sb: bool, cb: u128, eb: i32) -> (V, bool) {
  // order the operands so that eh >= el
  let (sh, ch, eh, sl, cl, el) = if ea >= eb { (sa, ca, ea, sb, cb, eb) } else { (sb, cb, eb, sa, ca, ea) };
  if cl == 0 {
    return (V::Fin(sh, ch, eh), false);
  }
  if ch == 0 {
    return (V::Fin(sl, cl, el), false);
  }
  let d = (eh - el) as u32;
  let (dh, dl) = (digit_count(ch), digit_count(cl));
  if dh + d <= 38 {
    let x = ch * pow10(d);
    return if sh == sl {
      round_pack_u128(sh, x + cl, el as i64)
    } else if x >= cl {
      round_pack_u128(sh, x - cl, el as i64)
    } else {
      round_pack_u128(sl, cl - x, el as i64)
    };
  }
  if dl as i64 + el as i64 <= eh as i64 + dh as i64 - 36 {
    let k = 36 - dh;
    let x = ch * pow10(k);
    return round_pack_u128(sh, if sh == sl { x + 1 } else { x - 1 }, (eh - k as i32) as i64);
  }
  // d < dl - dh + 36 <= 69 here, so ch * 10^d has fewer than 70 digits
  let Some(x) = U256::from_u128(ch).checked_mul_pow10(d) else {
    return (V::NaN, false);
  };
  let l = U256::from_u128(cl);
  let (s, sum) = if sh == sl {
    (sh, x.checked_add(l))
  } else if x >= l {
    (sh, x.checked_sub(l))
  } else {
    (sl, l.checked_sub(x))
  };
  match sum {
    Some(sum) => round_pack(s, &sum.to_decimal(), el as i64),
    None => (V::NaN, false),
  }
}

fn add_v(a: V, b: V) -> V {
  match (a, b) {
    (V::NaN, _) | (_, V::NaN) => V::NaN,
    (V::Inf(x), V::Inf(y)) => {
      if x == y {
        V::Inf(x)
      } else {
        V::NaN
      }
    }
    (V::Inf(x), _) | (_, V::Inf(x)) => V::Inf(x),
    (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) => {
      let (v, inexact) = add_fin(sa, ca, ea, sb, cb, eb);
      let v = ideal(v, inexact, ea.min(eb));
      match v {
        // sign of exact zero sum: negative only when both operands are negative
        V::Fin(_, 0, e) if !inexact || ca == 0 && cb == 0 => V::Fin(sa && sb, 0, e),
        other => other,
      }
    }
  }
}

fn sub_v(a: V, b: V) -> V {
  add_v(a, negate(b))
}

/// Exact product of two finite values, rounded once to decimal128.
fn mul_fin(s: bool, ca: u128, ea: i32, cb: u128, eb: i32) -> (V, bool) {
  let e = ea as i64 + eb as i64;
  match ca.checked_mul(cb) {
    Some(p) => round_pack_u128(s, p, e),
    None => round_pack(s, &U256::mul(ca, cb).to_decimal(), e),
  }
}

fn mul_v(a: V, b: V) -> V {
  match (a, b) {
    (V::NaN, _) | (_, V::NaN) => V::NaN,
    (V::Inf(x), V::Inf(y)) => V::Inf(x != y),
    (V::Inf(x), V::Fin(y, c, _)) | (V::Fin(y, c, _), V::Inf(x)) => {
      if c == 0 {
        V::NaN
      } else {
        V::Inf(x != y)
      }
    }
    (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) => {
      let (v, inexact) = mul_fin(sa != sb, ca, ea, cb, eb);
      match ideal(v, inexact, ea + eb) {
        V::Fin(_, c, e) => V::Fin(sa != sb, c, e),
        other => other,
      }
    }
  }
}

fn div_v(a: V, b: V) -> V {
  match (a, b) {
    (V::NaN, _) | (_, V::NaN) => V::NaN,
    (V::Inf(_), V::Inf(_)) => V::NaN,
    (V::Inf(x), V::Fin(y, _, _)) => V::Inf(x != y),
    (V::Fin(x, _, _), V::Inf(y)) => V::Fin(x != y, 0, EXP_MIN as i32),
    (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) => {
      if cb == 0 {
        return if ca == 0 { V::NaN } else { V::Inf(sa != sb) };
      }
      if ca == 0 {
        return V::Fin(sa != sb, 0, ((ea - eb) as i64).clamp(EXP_MIN, EXP_MAX) as i32);
      }
      let (v, inexact) = pack(B::div(ca, ea, cb, eb));
      match ideal(v, inexact, ea - eb) {
        V::Fin(_, c, e) => V::Fin(sa != sb, c, e),
        V::Inf(_) => V::Inf(sa != sb),
        other => other,
      }
    }
  }
}

/// Rounds the value to a multiple of `10^exponent` by `mode`, in one step on the coefficient.
///
/// A value with no digit below `10^exponent` (its exponent is at least `exponent`), a zero with
/// such an exponent included, is returned as it is: rounding cannot change it, and it is never
/// padded. Otherwise the result is `±q · 10^exponent`, where `q` is the coefficient without its
/// last `k` digits, incremented when the dropped digits are not all zero and the mode says so.
/// `q + 1 <= 10^33`, so the result always fits; `exponent` must lie in `EXP_MIN..=EXP_MAX`.
fn round_at(v: V, exponent: i32, mode: RoundingMode) -> V {
  match v {
    V::Fin(s, c, e) if e < exponent => {
      let k = exponent.abs_diff(e);
      // the dropped digits compared with half a unit of the kept position; with more than 38
      // dropped digits the coefficient (below 10^34) is below half of it
      let (q, r, half) = if k > 38 {
        (0, c, Ordering::Less)
      } else {
        let p = pow10(k);
        (c / p, c % p, (c % p).cmp(&(p / 2)))
      };
      let up = r != 0
        && match mode {
          RoundingMode::Up => true,
          RoundingMode::Down => false,
          RoundingMode::Ceiling => !s,
          RoundingMode::Floor => s,
          RoundingMode::HalfUp => half != Ordering::Less,
          RoundingMode::HalfDown => half == Ordering::Greater,
          RoundingMode::HalfEven => half == Ordering::Greater || half == Ordering::Equal && q % 2 == 1,
        };
      V::Fin(s, q + u128::from(up), exponent)
    }
    other => other,
  }
}

/// Rounds the value to an integral value by `mode` (an exponent of 0 or more is left unchanged).
fn round_integral_v(v: V, mode: RoundingMode) -> V {
  round_at(v, 0, mode)
}

/// `10^n mod m` for `0 < m < 10^34`, by binary exponentiation with exact products.
fn pow10_mod(n: u32, m: u128) -> u128 {
  let (mut result, mut base, mut n) = (1 % m, 10 % m, n);
  while n > 0 {
    if n & 1 == 1 {
      result = U256::mul(result, base).rem_u128(m);
    }
    base = U256::mul(base, base).rem_u128(m);
    n >>= 1;
  }
  result
}

/// `a - b·floor(a/b)` for a finite `a` and a finite, non-zero `b` (DMN 1.6 Table 77), over the
/// exact values and rounded once to decimal128; NaN for any other operand.
///
/// The truncated remainder `R = |a| mod |b|` is exact with at most 34 digits: for `ea >= eb` it
/// is `(ca · 10^(ea-eb) mod cb) · 10^eb`, the power of ten taken modulo `cb`; for `ea < eb` it is
/// `(ca mod cb · 10^(eb-ea)) · 10^ea`, which is `|a|` itself when `|b|` is the larger. The result
/// is `R` with the sign of `b` when the signs agree, and `|b| - R` with the sign of `b` when they
/// differ and `R` is not zero; that difference is the only rounding.
fn modulo_v(a: V, b: V) -> V {
  let (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) = (a, b) else {
    return V::NaN;
  };
  if cb == 0 {
    return V::NaN;
  }
  let (r, e) = if ea >= eb {
    let d = ea.abs_diff(eb);
    let r = if digit_count(ca) + d <= 38 {
      (ca * pow10(d)) % cb
    } else {
      U256::mul(ca % cb, pow10_mod(d, cb)).rem_u128(cb)
    };
    (r, eb)
  } else {
    let d = eb.abs_diff(ea);
    let r = match POW10.get(d as usize).and_then(|p| cb.checked_mul(*p)) {
      Some(m) => ca % m,
      // |b| exceeds every u128 coefficient, so |a| < |b|
      None => ca,
    };
    (r, ea)
  };
  if r == 0 || sa == sb {
    return V::Fin(sb, r, e);
  }
  match add_v(V::Fin(false, cb, eb), V::Fin(true, r, e)) {
    V::Fin(_, c, x) => V::Fin(sb, c, x),
    other => other,
  }
}

/// Compares two values, returns `None` when unordered (NaN involved).
fn cmp_v(a: V, b: V) -> Option<Ordering> {
  match (a, b) {
    (V::NaN, _) | (_, V::NaN) => None,
    (V::Inf(x), V::Inf(y)) => Some(y.cmp(&x)),
    (V::Inf(x), _) => Some(if x { Ordering::Less } else { Ordering::Greater }),
    (_, V::Inf(y)) => Some(if y { Ordering::Greater } else { Ordering::Less }),
    (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) => {
      if ca == 0 && cb == 0 {
        return Some(Ordering::Equal);
      }
      let na = sa && ca != 0;
      let nb = sb && cb != 0;
      if ca == 0 {
        return Some(if nb { Ordering::Greater } else { Ordering::Less });
      }
      if cb == 0 {
        return Some(if na { Ordering::Less } else { Ordering::Greater });
      }
      if na != nb {
        return Some(if na { Ordering::Less } else { Ordering::Greater });
      }
      // same sign, both non-zero: compare the magnitudes ca·10^ea and cb·10^eb exactly
      let mag = match ea.cmp(&eb) {
        Ordering::Equal => ca.cmp(&cb),
        Ordering::Greater => cmp_scaled(ca, ea.abs_diff(eb), cb),
        Ordering::Less => cmp_scaled(cb, eb.abs_diff(ea), ca).reverse(),
      };
      Some(if na { mag.reverse() } else { mag })
    }
  }
}

/// Compares `c · 10^d` with `other`, for `c > 0`: when `c · 10^d` does not fit in `u128`, it is
/// larger than any `u128`.
fn cmp_scaled(c: u128, d: u32, other: u128) -> Ordering {
  match POW10.get(d as usize).and_then(|p| c.checked_mul(*p)) {
    Some(scaled) => scaled.cmp(&other),
    None => Ordering::Greater,
  }
}

fn eq_v(a: V, b: V) -> bool {
  cmp_v(a, b) == Some(Ordering::Equal)
}

fn is_finite_v(v: V) -> bool {
  matches!(v, V::Fin(..))
}

/// IEEE remainder by two is zero (i.e. the value is an even integer).
fn rem_two_is_zero(v: V) -> bool {
  match v {
    V::Fin(_, c, e) => {
      if c == 0 || e > 0 {
        true
      } else if e == 0 {
        c % 2 == 0
      } else {
        let k = (-e) as u32;
        if k > 38 {
          false
        } else {
          let p = pow10(k);
          c % p == 0 && (c / p).is_multiple_of(2)
        }
      }
    }
    _ => false,
  }
}

/// Converts the value to integer, rounding toward zero (like bid128_to_int*_int).
fn to_i128_int(v: V) -> Option<i128> {
  match round_integral_v(v, RoundingMode::Down) {
    V::Fin(s, c, e) => {
      if c == 0 {
        return Some(0);
      }
      if digit_count(c) as i64 + e as i64 > 38 {
        return None;
      }
      let m = (c * pow10(e as u32)) as i128;
      Some(if s { -m } else { m })
    }
    _ => None,
  }
}

/// Converts the value to unsigned integer (like bid128_to_uint*_int).
fn to_unsigned(v: V, max: u128) -> Option<u128> {
  let n = to_i128_int(v)?;
  if n < 0 || n as u128 > max { None } else { Some(n as u128) }
}

fn to_signed(v: V, min: i128, max: i128) -> Option<i128> {
  let n = to_i128_int(v)?;
  if n < min || n > max { None } else { Some(n) }
}

/// Parses a string the same way as Intel's `bid128_from_string`.
/// Returns the value and the flag telling whether it had to be rounded.
fn parse_v(s: &str) -> (V, bool) {
  let b = s.as_bytes();
  let mut i = 0;
  while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
    i += 1;
  }
  let mut neg = false;
  if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
    neg = b[i] == b'-';
    i += 1;
  }
  let at = |k: usize| if k < b.len() { b[k] } else { 0 };
  if !(at(i) == b'.' || at(i).is_ascii_digit()) {
    let rest = s[i..].to_ascii_lowercase();
    return (if rest == "inf" || rest == "infinity" { V::Inf(neg) } else { V::NaN }, false);
  }
  let mut digits = String::new();
  let mut after = 0_i64;
  let mut seen_point = false;
  while i < b.len() {
    let c = b[i];
    if c.is_ascii_digit() {
      digits.push(c as char);
      if seen_point {
        after += 1;
      }
    } else if c == b'.' && !seen_point {
      seen_point = true;
    } else {
      break;
    }
    i += 1;
  }
  let mut exp: i64 = 0;
  if i < b.len() {
    let c = b[i];
    if c != b'e' && c != b'E' {
      return (V::NaN, false);
    }
    i += 1;
    let c = at(i);
    if !c.is_ascii_digit() && !((c == b'+' || c == b'-') && at(i + 1).is_ascii_digit()) {
      return (V::NaN, false);
    }
    let mut sign = 1;
    if c == b'-' || c == b'+' {
      if c == b'-' {
        sign = -1;
      }
      i += 1;
    }
    // Intel skips leading zeros and reads at most 7 exponent digits, ignoring the rest
    while at(i) == b'0' && at(i + 1).is_ascii_digit() {
      i += 1;
    }
    let mut count = 0;
    while at(i).is_ascii_digit() && count < 7 {
      exp = exp * 10 + (at(i) - b'0') as i64;
      i += 1;
      count += 1;
    }
    exp *= sign;
  }
  round_pack(neg, &digits, exp - after)
}

/// Converts the value into the string in the format of Intel's `bid128_to_string` (`+12E-1`,
/// `-Inf`), except NaN, which has no sign.
fn to_intel_string(v: V) -> String {
  match v {
    V::NaN => "NaN".to_string(),
    V::Inf(s) => if s { "-Inf" } else { "+Inf" }.to_string(),
    V::Fin(s, c, e) => format!("{}{}E{}{}", if s { '-' } else { '+' }, c, if e >= 0 { "+" } else { "" }, e),
  }
}

/// Exponential function of a finite, non-zero value.
fn exp_v(s: bool, c: u128, e: i32) -> V {
  // e^x overflows decimal128 for x > ~14149.7 and underflows to zero for x < ~-14224
  if c as f64 * 10f64.powi(e) > 14300.0 {
    return if s { V::Fin(false, 0, EXP_MIN as i32) } else { V::Inf(false) };
  }
  match pack(B::exp(s, c, e)).0 {
    // underflow to zero
    V::Fin(_, 0, _) => V::Fin(false, 0, EXP_MIN as i32),
    v => v,
  }
}

/// Natural logarithm of a finite, positive value.
fn ln_v(c: u128, e: i32) -> V {
  pack(B::ln(c, e)).0
}

/// `x^y = exp(y ln x)` for a finite `x > 0` and a finite, non-zero `y` that is either not an
/// integer or an integer outside `i32`, correctly rounded to 34 digits (Ziv's strategy).
///
/// The backend evaluates `ln`, `*` and `exp` at the working precision `p` (each correctly
/// rounded, see [Backend::pow_step]), so the relative error of the result is below
/// `|t| * 10^(1-p) + 10^(1-p)` with `t = y ln x`, `|t| < 14250`, i.e. below `2 * 10^5` units of
/// the `p`-th digit. The result is accepted when its digits after the final rounding position
/// are more than `10^7` units away from the half-way point `5000…`, otherwise the precision is
/// increased. When even 140 digits do not settle it, the value is taken to be an exact tie (an
/// algebraic result such as `16^-12.5 = 2^-50`, 35 digits ending in 5) and rounded half-even as
/// such (the computed digits themselves carry an error of a few units in the 140th digit, so
/// they cannot decide); a true value within `10^-133` of a tie without being one is not expected
/// to exist for decimal128 operands.
fn pow_exp_log(cx: u128, ex: i32, sy: bool, cy: u128, ey: i32) -> V {
  let mut result = None;
  for &precision in POW_PRECISIONS.iter() {
    let (mut digits, mut e) = match B::pow_step(cx, ex, sy, cy, ey, precision) {
      PowStep::Overflow => return V::Inf(false),
      PowStep::Underflow => return V::Fin(false, 0, EXP_MIN as i32),
      PowStep::Undefined => return V::NaN,
      PowStep::Value(digits, e) => (digits, e),
    };
    // exactly p digits (a backend may strip trailing zeros)
    if digits.len() < precision {
      e -= (precision - digits.len()) as i64;
      digits.push_str(&"0".repeat(precision - digits.len()));
    }
    let p = digits.len();
    // digits below the final rounding position (34 digits, or the subnormal position)
    let position = (e + (p - PRECISION) as i64).max(EXP_MIN);
    let tail_len = (position - e) as usize;
    if tail_len > p {
      // everything is below a tenth of the rounding unit: rounds to zero
      return round_pack(false, &digits, e).0;
    }
    let tail = &digits[p - tail_len..];
    let (first, middle) = (tail.as_bytes()[0], &tail[1..tail_len - 7]);
    let near_tie = (first == b'5' && middle.bytes().all(|b| b == b'0')) || (first == b'4' && middle.bytes().all(|b| b == b'9'));
    if !near_tie {
      return round_pack(false, &digits, e).0;
    }
    // an exact tie: the digits above the rounding position followed by a single 5
    result = Some(round_pack(false, &format!("{}5", &digits[..p - tail_len]), e + tail_len as i64 - 1).0);
  }
  result.unwrap_or(V::NaN)
}

/// Square root of a finite, positive value, correctly rounded.
///
/// The exact integer square root with remainder of `c * 10^(2k)` (at least 69 digits, so
/// the root has at least 35 digits) is taken; a non-zero remainder means the true root is
/// strictly between `root` and `root + 1`, which the sticky digit `root * 10 + 1` represents
/// exactly for the single final rounding in [round_pack_u128].
fn sqrt_v(c: u128, e: i32) -> V {
  // sqrt(c * 10^e) with an even exponent: sqrt(c2) * 10^(e2 / 2)
  let (c2, e2) = if e % 2 != 0 { (c * 10, e - 1) } else { (c, e) };
  let k = ((69 - digit_count(c2) as i32).max(0) + 1) / 2;
  // c2 has at most 35 digits and 2k at most 68, so the radicand has at most 70 digits
  let Some(radicand) = U256::from_u128(c2).checked_mul_pow10(2 * k as u32) else {
    return V::NaN;
  };
  let (root, inexact) = isqrt_rem(radicand);
  let exp = (e2 / 2 - k) as i64;
  let (v, inexact) = if inexact {
    (round_pack_u128(false, root * 10 + 1, exp - 1).0, true)
  } else {
    round_pack_u128(false, root, exp)
  };
  ideal(v, inexact, e.div_euclid(2))
}

const ZERO: V = V::Fin(false, 0, 0);
const ONE: V = V::Fin(false, 1, 0);
const TWO: V = V::Fin(false, 2, 0);

/// FEEL number.
///
/// The value keeps its cohort, the pair `(p,s)` of DMN 1.6 §10.3.2.3.1 among the "many different
/// pairs \[that\] have the same value"; no text shows it ([Display] prints the value).
#[derive(Copy, Clone)]
pub struct FeelNumber(V);

impl FeelNumber {
  fn v(&self) -> V {
    self.0
  }

  /// The one constructor: every [FeelNumber] is made here. A zero loses its sign (DMN 1.6
  /// §10.3.2.3.1: "There is no distinction between -0 and 0"); the arithmetic on [V] keeps the
  /// signs of IEEE 754 internally.
  fn of(v: V) -> Self {
    match v {
      V::Fin(true, 0, e) => Self(V::Fin(false, 0, e)),
      v => Self(v),
    }
  }

  /// Returns [FeelNumber] value infinite.
  pub fn infinite() -> FeelNumber {
    Self::of(V::Inf(false))
  }

  /// Returns [FeelNumber] value 0 (zero).
  pub fn zero() -> FeelNumber {
    Self::of(ZERO)
  }

  /// Returns [FeelNumber] value 1 (one).
  pub fn one() -> FeelNumber {
    Self::of(ONE)
  }

  /// Returns [FeelNumber] value 2 (two).
  pub fn two() -> FeelNumber {
    Self::of(TWO)
  }

  /// Returns [FeelNumber] value 1_000_000_000 (billion).
  pub fn billion() -> FeelNumber {
    Self::of(V::Fin(false, 1_000_000_000, 0))
  }

  /// Creates the number `n · 10^-s`, rounded once to decimal128 (half-even, gradual underflow,
  /// infinite beyond the range); every scale `s` is taken as it is.
  pub fn new(n: i64, s: i32) -> Self {
    Self::of(round_pack_u128(n < 0, n.unsigned_abs() as u128, -i64::from(s)).0)
  }

  /// Creates a new [FeelNumber] from [isize].
  fn from_isize(n: isize) -> Self {
    Self::of(V::Fin(n < 0, n.unsigned_abs() as u128, 0))
  }

  /// Returns an absolute value of this [FeelNumber].
  pub fn abs(&self) -> Self {
    let v = match self.v() {
      V::Fin(_, c, e) => V::Fin(false, c, e),
      V::Inf(_) => V::Inf(false),
      V::NaN => V::NaN,
    };
    Self::of(v)
  }

  /// Rounds this number to `scale` fraction digits by `mode` ([RoundingMode]), in one step:
  /// to a multiple of `10^-scale` (DMN 1.6 Table 77: "Return n with given scale and rounding
  /// mode"). A number with no digit below the scale is returned as it is. The only error is a
  /// scale outside `-6111..=6176` (Table 77, footnote 1: "Scale is in the range
  /// [−6111..6176]"); every number, a zero or a tiny one included, can be rounded at every scale
  /// of that range, and the result always fits.
  pub fn round_to(&self, scale: i32, mode: RoundingMode) -> Result<Self> {
    if !(MIN_SCALE..=MAX_SCALE).contains(&scale) {
      return Err(err_invalid_scale(MIN_SCALE, MAX_SCALE, scale));
    }
    Ok(Self::of(round_at(self.v(), -scale, mode)))
  }

  pub fn even(&self) -> bool {
    rem_two_is_zero(self.v())
  }

  pub fn exp(&self) -> Option<Self> {
    let n = match self.v() {
      V::NaN => V::NaN,
      V::Inf(false) => V::Inf(false),
      V::Inf(true) => ZERO,
      V::Fin(_, 0, _) => ONE,
      V::Fin(s, c, e) => exp_v(s, c, e),
    };
    if is_finite_v(n) { Some(Self::of(n)) } else { None }
  }

  pub fn frac(&self) -> Self {
    let v = self.v();
    Self::of(sub_v(v, round_integral_v(v, RoundingMode::Down)))
  }

  pub fn is_integer(&self) -> bool {
    let v = self.v();
    eq_v(v, round_integral_v(v, RoundingMode::Down))
  }

  pub fn is_zero(&self) -> bool {
    eq_v(self.v(), ZERO)
  }

  /// Returns `true` when this number is finite (not an infinity and not NaN).
  ///
  /// Arithmetic on decimal128 values can overflow to infinity (e.g. `9e6144 * 10`) or produce NaN
  /// by an invalid operation (e.g. `0 / 0`, or an infinity minus itself); FEEL has no such
  /// numbers, so evaluators turn non-finite results into null.
  pub fn is_finite(&self) -> bool {
    is_finite_v(self.v())
  }

  pub fn is_one(&self) -> bool {
    eq_v(self.v(), ONE)
  }

  pub fn is_negative(&self) -> bool {
    cmp_v(self.v(), ZERO) == Some(Ordering::Less)
  }

  pub fn is_positive(&self) -> bool {
    cmp_v(self.v(), ZERO) == Some(Ordering::Greater)
  }

  pub fn ln(&self) -> Option<Self> {
    let n = match self.v() {
      V::NaN => V::NaN,
      V::Inf(true) => V::NaN,
      V::Inf(false) => V::Inf(false),
      V::Fin(_, 0, _) => V::Inf(true),
      V::Fin(true, _, _) => V::NaN,
      V::Fin(false, c, e) => ln_v(c, e),
    };
    if is_finite_v(n) { Some(Self::of(n)) } else { None }
  }

  /// `self - divisor·floor(self/divisor)` (DMN 1.6 Table 77), over the exact values and rounded
  /// once to 34 digits: zero or a number with the sign of the divisor. `None` when the divisor is
  /// zero (or an operand is not finite).
  pub fn modulo(&self, divisor: &FeelNumber) -> Option<Self> {
    let n = modulo_v(self.v(), divisor.v());
    if is_finite_v(n) { Some(Self::of(n)) } else { None }
  }

  pub fn odd(&self) -> bool {
    if self.is_integer() { !rem_two_is_zero(self.v()) } else { false }
  }

  pub fn pow(&self, rhs: &FeelNumber) -> Option<Self> {
    let n = pow_v(self.v(), rhs.v());
    if is_finite_v(n) { Some(Self::of(n)) } else { None }
  }

  pub fn sqrt(&self) -> Option<Self> {
    let n = match self.v() {
      V::NaN => V::NaN,
      V::Inf(true) => V::NaN,
      V::Inf(false) => V::Inf(false),
      V::Fin(s, 0, e) => V::Fin(s, 0, e.div_euclid(2)),
      V::Fin(true, _, _) => V::NaN,
      V::Fin(false, c, e) => sqrt_v(c, e),
    };
    if is_finite_v(n) { Some(Self::of(n)) } else { None }
  }

  pub fn square(&self) -> Option<Self> {
    let n = pow_v(self.v(), TWO);
    if is_finite_v(n) { Some(Self::of(n)) } else { None }
  }

  pub fn trunc(&self) -> Self {
    Self::of(round_integral_v(self.v(), RoundingMode::Down))
  }

  /// Parses a number like [FromStr::from_str] and also reports whether the value had
  /// to be rounded (more than 34 significant digits, or an exponent so small that the
  /// value underflows to a subnormal number or to zero).
  pub fn from_str_reporting_rounding(s: &str) -> Result<(Self, bool), DsntkError> {
    let (n, inexact) = parse_v(s);
    if is_finite_v(n) {
      Ok((Self::of(n), inexact))
    } else {
      Err(err_invalid_number_literal(s))
    }
  }
}

/// Power function, mirrors the structure of Intel's `bid128_pow`:
/// - special cases (zero exponent, base one, NaN, infinities, zero base, negative base),
/// - integer exponent fitting in `i32`: binary exponentiation using decimal128
///   multiplications (each rounded to 34 digits, exactly like Intel), `1/x^|n|` for n < 0,
/// - otherwise `exp(y ln x)` correctly rounded by [pow_exp_log] (more accurate than Intel,
///   which goes through binary128 and may differ in the last digit).
fn pow_v(x: V, y: V) -> V {
  // 1^y = x^0 = 1, even for NaN
  if is_zero_v(y) || eq_v(x, ONE) {
    return ONE;
  }
  // a NaN operand is propagated
  if x == V::NaN || y == V::NaN {
    return V::NaN;
  }
  if let V::Inf(y_negative) = y {
    let ax = match x {
      V::Fin(_, c, e) => cmp_v(V::Fin(false, c, e), ONE),
      _ => Some(Ordering::Greater),
    };
    return match (ax, y_negative) {
      (Some(Ordering::Equal), _) => ONE,
      (Some(Ordering::Less), true) | (Some(Ordering::Greater), false) => V::Inf(false),
      _ => ZERO,
    };
  }
  let y_integer = eq_v(y, round_integral_v(y, RoundingMode::HalfEven));
  let y_odd = y_integer && !rem_two_is_zero(y);
  let y_negative = matches!(y, V::Fin(true, _, _));
  let x_negative = matches!(x, V::Fin(true, _, _) | V::Inf(true));
  let odd_negative = y_odd && x_negative;
  match x {
    V::Inf(_) => return if y_negative { V::Fin(odd_negative, 0, 0) } else { V::Inf(odd_negative) },
    V::Fin(_, 0, _) => return if y_negative { V::Inf(odd_negative) } else { V::Fin(odd_negative, 0, 0) },
    _ => {}
  }
  if x_negative && !y_integer {
    return V::NaN;
  }
  // integer exponent that fits in i32 (and is not i32::MIN, like in Intel's implementation)
  if y_integer && let Some(n) = to_signed(y, i32::MIN as i128 + 1, i32::MAX as i128) {
    let mut n_abs = (n as i32).unsigned_abs();
    let mut r = ONE;
    let mut p = x;
    while n_abs > 0 {
      if n_abs & 1 == 1 {
        r = mul_v(r, p);
      }
      if n_abs > 1 {
        p = mul_v(p, p);
      }
      n_abs >>= 1;
    }
    return if n < 0 { div_v(ONE, r) } else { r };
  }
  let V::Fin(_, cx, ex) = x else { return V::NaN };
  let V::Fin(sy, cy, ey) = y else { return V::NaN };
  // (-1)^y for an integer y outside i32: Intel's logarithm takes its Taylor branch with
  // t = |x| - 1 = 0, all intermediate values are zeros whose exponents propagate into
  // the final `fma(exp(l), l_lo, exp(l)) = 1`; with x = -1 written with k fractional
  // zeros (-1.000...), l_lo = 0E(ey - 34 - 3k), so the result is 1 with exponent
  // min(0, ey - 34 - 3k), but at most 34 digits (e.g. (-1)**1E+10 = 1E-24 as 1.000...).
  if (-33..=0).contains(&ex) && cx == pow10((-ex) as u32) {
    let k = -ex as i64;
    let l_lo_exp = (ey as i64 - 34 - 3 * k).clamp(EXP_MIN, EXP_MAX);
    let exp = l_lo_exp.min(0).max(1 - PRECISION as i64) as i32;
    return V::Fin(odd_negative, pow10((-exp) as u32), exp);
  }
  match pow_exp_log(cx, ex, sy, cy, ey) {
    // like Intel: zero (underflow) and infinity (overflow) are returned without sign correction
    V::Fin(_, 0, _) => V::Fin(false, 0, EXP_MIN as i32),
    V::Fin(_, c, e) => V::Fin(odd_negative, c, e),
    other => other,
  }
}

impl PartialEq<FeelNumber> for FeelNumber {
  fn eq(&self, rhs: &Self) -> bool {
    eq_v(self.v(), rhs.v())
  }
}

impl PartialEq<FeelNumber> for isize {
  fn eq(&self, rhs: &FeelNumber) -> bool {
    eq_v(FeelNumber::from_isize(*self).v(), rhs.v())
  }
}

impl PartialEq<isize> for FeelNumber {
  fn eq(&self, rhs: &isize) -> bool {
    eq_v(self.v(), FeelNumber::from_isize(*rhs).v())
  }
}

/// Same semantics as the Intel version: Equal, Greater, otherwise Less (also for NaN).
fn partial_cmp_v(a: V, b: V) -> Option<Ordering> {
  match cmp_v(a, b) {
    Some(Ordering::Equal) => Some(Ordering::Equal),
    Some(Ordering::Greater) => Some(Ordering::Greater),
    _ => Some(Ordering::Less),
  }
}

impl PartialOrd<FeelNumber> for FeelNumber {
  fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
    partial_cmp_v(self.v(), rhs.v())
  }

  fn lt(&self, rhs: &FeelNumber) -> bool {
    cmp_v(self.v(), rhs.v()) == Some(Ordering::Less)
  }

  fn le(&self, rhs: &FeelNumber) -> bool {
    matches!(cmp_v(self.v(), rhs.v()), Some(Ordering::Less | Ordering::Equal))
  }

  fn gt(&self, rhs: &FeelNumber) -> bool {
    cmp_v(self.v(), rhs.v()) == Some(Ordering::Greater)
  }

  fn ge(&self, rhs: &FeelNumber) -> bool {
    matches!(cmp_v(self.v(), rhs.v()), Some(Ordering::Greater | Ordering::Equal))
  }
}

impl PartialOrd<FeelNumber> for isize {
  fn partial_cmp(&self, rhs: &FeelNumber) -> Option<Ordering> {
    partial_cmp_v(FeelNumber::from_isize(*self).v(), rhs.v())
  }
}

impl PartialOrd<isize> for FeelNumber {
  fn partial_cmp(&self, rhs: &isize) -> Option<Ordering> {
    partial_cmp_v(self.v(), FeelNumber::from_isize(*rhs).v())
  }
}

impl Add<FeelNumber> for FeelNumber {
  type Output = Self;
  fn add(self, rhs: Self) -> Self::Output {
    Self::of(add_v(self.v(), rhs.v()))
  }
}

impl AddAssign<FeelNumber> for FeelNumber {
  fn add_assign(&mut self, rhs: Self) {
    *self = *self + rhs;
  }
}

impl Sub<FeelNumber> for FeelNumber {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    Self::of(sub_v(self.v(), rhs.v()))
  }
}

impl SubAssign<FeelNumber> for FeelNumber {
  fn sub_assign(&mut self, rhs: Self) {
    *self = *self - rhs;
  }
}

impl Mul<FeelNumber> for FeelNumber {
  type Output = Self;
  fn mul(self, rhs: Self) -> Self::Output {
    Self::of(mul_v(self.v(), rhs.v()))
  }
}

impl MulAssign<FeelNumber> for FeelNumber {
  fn mul_assign(&mut self, rhs: Self) {
    *self = *self * rhs;
  }
}

impl Div<FeelNumber> for FeelNumber {
  type Output = Self;
  fn div(self, rhs: Self) -> Self::Output {
    Self::of(div_v(self.v(), rhs.v()))
  }
}

impl DivAssign<FeelNumber> for FeelNumber {
  fn div_assign(&mut self, rhs: Self) {
    *self = *self / rhs;
  }
}

impl Neg for FeelNumber {
  type Output = Self;
  fn neg(self) -> Self::Output {
    Self::of(negate(self.v()))
  }
}

impl Debug for FeelNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", to_intel_string(self.v()))
  }
}

impl Display for FeelNumber {
  /// Writes the canonical text of the value: plain digits without an exponent, no trailing zeros
  /// after the decimal point, and `0` for every zero, whatever the cohort (`1.50` and `1.5E+0`
  /// are `1.5`, `decimal(1, 2)` is `1`). Width, fill, alignment and the sign flag apply; a
  /// precision is ignored.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Infinity and NaN: FEEL has no such numbers, but never panic while formatting them
    // (a panic aborts the whole engine on wasm32).
    let (negative, c, exponent) = match self.v() {
      V::NaN => return f.pad("NaN"),
      V::Inf(negative) => return f.pad(if negative { "-Infinity" } else { "Infinity" }),
      V::Fin(negative, c, e) => (negative, c, e),
    };
    if c == 0 {
      return f.pad_integral(true, "", "0");
    }
    let mut digits = c.to_string();
    let text = if exponent >= 0 {
      digits.extend(std::iter::repeat_n('0', exponent as usize));
      digits
    } else {
      // the trailing zeros of the coefficient that stand after the decimal point
      let zeros = (digits.len() - digits.trim_end_matches('0').len()).min(exponent.unsigned_abs() as usize);
      digits.truncate(digits.len() - zeros);
      let fraction = exponent.unsigned_abs() as usize - zeros;
      if fraction == 0 {
        digits
      } else if digits.len() > fraction {
        digits.insert(digits.len() - fraction, '.');
        digits
      } else {
        let mut text = String::with_capacity(fraction + 2);
        text.push_str("0.");
        text.extend(std::iter::repeat_n('0', fraction - digits.len()));
        text.push_str(&digits);
        text
      }
    };
    f.pad_integral(!negative, "", &text)
  }
}

impl Jsonify for FeelNumber {
  /// Converts [FeelNumber] to JSON string.
  fn jsonify(&self) -> String {
    format!("{self}")
  }
}

impl FromStr for FeelNumber {
  type Err = DsntkError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let (n, _) = parse_v(s);
    if is_finite_v(n) { Ok(Self::of(n)) } else { Err(err_invalid_number_literal(s)) }
  }
}

macro_rules! from_unsigned {
  ($($t:ty),*) => {
    $(
      impl From<$t> for FeelNumber {
        fn from(value: $t) -> Self {
          Self::of(V::Fin(false, value as u128, 0))
        }
      }
    )*
  };
}

macro_rules! from_signed {
  ($($t:ty),*) => {
    $(
      impl From<$t> for FeelNumber {
        fn from(value: $t) -> Self {
          Self::of(V::Fin(value < 0, (value as i128).unsigned_abs(), 0))
        }
      }
    )*
  };
}

from_unsigned!(u8, u16, u32, u64, usize);
from_signed!(i8, i16, i32, i64, isize);

/// Conversions to unsigned integers; the intermediate type mirrors the Intel version
/// (`bid128_to_uint32_int` for u8/u16/u32, `bid128_to_uint64_int` for u64/usize).
macro_rules! try_into_unsigned {
  ($($t:ty => $wide:ty),*) => {
    $(
      impl TryFrom<FeelNumber> for $t {
        type Error = DsntkError;
        fn try_from(value: FeelNumber) -> Result<Self, Self::Error> {
          <$t>::try_from(&value)
        }
      }

      impl TryFrom<&FeelNumber> for $t {
        type Error = DsntkError;
        fn try_from(value: &FeelNumber) -> Result<Self, Self::Error> {
          let n = to_unsigned(value.v(), <$wide>::MAX as u128).ok_or_else(err_number_conversion_failed)?;
          <$t>::try_from(n).map_err(|_| err_number_conversion_failed())
        }
      }
    )*
  };
}

/// Conversions to signed integers; the intermediate type mirrors the Intel version
/// (`bid128_to_int32_int` for i8/i16/i32, `bid128_to_int64_int` for i64/isize).
macro_rules! try_into_signed {
  ($($t:ty => $wide:ty),*) => {
    $(
      impl TryFrom<FeelNumber> for $t {
        type Error = DsntkError;
        fn try_from(value: FeelNumber) -> Result<Self, Self::Error> {
          <$t>::try_from(&value)
        }
      }

      impl TryFrom<&FeelNumber> for $t {
        type Error = DsntkError;
        fn try_from(value: &FeelNumber) -> Result<Self, Self::Error> {
          let n = to_signed(value.v(), <$wide>::MIN as i128, <$wide>::MAX as i128).ok_or_else(err_number_conversion_failed)?;
          <$t>::try_from(n).map_err(|_| err_number_conversion_failed())
        }
      }
    )*
  };
}

try_into_unsigned!(u8 => u32, u16 => u32, u32 => u32, u64 => u64, usize => u64);
try_into_signed!(i8 => i32, i16 => i32, i32 => i32, i64 => i64, isize => i64);

#[cfg(test)]
mod tests;
