//! # decimal128 core shared by the pure-Rust number backends
//!
//! Implementation of the FEEL number type with the semantics of IEEE 754-2008 **decimal128**,
//! exactly like the Intel-based implementation (`dfp_number.rs`): 34 significant digits,
//! rounding half-even, exponent (of the coefficient) in range `-6176..=6111`, cohorts preserved.
//!
//! The value is stored in a backend-independent, `Copy` form [V]: sign, `u128` coefficient
//! (< 10^34) and `i32` exponent, plus infinity and NaN. Everything whose exact result has a
//! bounded number of digits is computed here, directly on that triple, in `u128` when the
//! intermediate fits and in a tiny 256-bit integer ([U256]) otherwise: `+ - *`, square root,
//! rounding to integral, quantize, scaling, comparisons, parsing, formatting, conversions, the
//! integer-exponent power and the cohort rules. Only the operations with an unbounded exact
//! result (`/`, `exp`, `ln`, `pow` with a non-integer exponent) are delegated to a [Backend],
//! which returns a correctly rounded value together with the side the exact value lies on; that
//! side becomes a sticky digit and the value is passed **once** through [round_pack], which also
//! applies the decimal128 exponent range (subnormals, clamping and overflow to infinity).
//!
//! The backend is selected by a Cargo feature (`dashu`: `backend_dashu.rs`).

use crate::errors::*;
use dsntk_common::{DsntkError, Jsonify, Result};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign};
use std::str::FromStr;

/// The backend selected by the Cargo features.
#[cfg(feature = "dashu")]
type B = crate::backend_dashu::Dashu;

/// Minimum scale accepted by the scale-based rounding functions (FEEL: `-6111..=6176`).
const MIN_SCALE: i32 = -6111;

/// Maximum scale accepted by the scale-based rounding functions (FEEL: `-6111..=6176`).
const MAX_SCALE: i32 = 6176;

/// Largest adjusted exponent of the scaled intermediate `n * 10^scale` the rounding
/// functions work on (the maximum exponent of a decimal128 number).
const SCALED_EXP_MAX: i64 = 6144;

/// Precision of decimal128 (number of significant digits).
pub(crate) const PRECISION: usize = 34;

/// Precision used when the exact cohort of an `exp`/`ln` result has to be decided
/// (see [binary128_cohort]); the decision needs a few digits more than decimal128.
const COHORT_PRECISION: usize = 45;

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
  /// Not a number, with sign (`true` = negative). The sign only shows in `Debug`
  /// (`-NaN`, like Intel's `bid128_to_string`): it is flipped by negation, cleared by `abs`,
  /// kept when a NaN operand is propagated, and positive for an invalid operation.
  NaN(bool),
  /// Infinity with sign (`true` = negative).
  Inf(bool),
  /// Finite number: sign (`true` = negative), coefficient (< 10^34), exponent.
  Fin(bool, u128, i32),
}

/// Rounding modes used for rounding to integral value.
#[derive(Copy, Clone)]
enum Rnd {
  /// Round half to even.
  HalfEven,
  /// Round half away from zero.
  HalfAway,
  /// Round toward zero.
  Zero,
  /// Round toward positive infinity.
  Ceiling,
  /// Round toward negative infinity.
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

  /// `e^x` for `x = ±c · 10^e` (`neg` is the sign of `x`), correctly rounded to `precision`
  /// digits. The core calls it with [PRECISION] for the value, and with [COHORT_PRECISION]
  /// for the digits only (the side is then ignored).
  fn exp(neg: bool, c: u128, e: i32, precision: usize) -> Wide;

  /// `ln(c · 10^e)`, correctly rounded to `precision` digits (used like [Backend::exp]).
  fn ln(c: u128, e: i32, precision: usize) -> Wide;

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
/// (sums and products of two 34-digit coefficients, square root radicands, cohort decisions).
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

  /// 10^n, when it fits.
  fn pow10(n: u32) -> Option<U256> {
    U256::from_u128(1).checked_mul_pow10(n)
  }

  fn checked_shl(self, n: u32) -> Option<U256> {
    if n == 0 {
      Some(self)
    } else if n >= 256 {
      if self.is_zero() { Some(self) } else { None }
    } else if n >= 128 {
      if self.hi != 0 || (n > 128 && self.lo >> (256 - n) != 0) {
        None
      } else {
        Some(U256 { hi: self.lo << (n - 128), lo: 0 })
      }
    } else if self.hi >> (128 - n) != 0 {
      None
    } else {
      Some(U256 {
        hi: (self.hi << n) | (self.lo >> (128 - n)),
        lo: self.lo << n,
      })
    }
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

  /// Parses a string of decimal digits; `None` when it is not one or does not fit.
  fn from_decimal(s: &str) -> Option<U256> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
      return None;
    }
    let mut x = U256::ZERO;
    let head = s.len() % 19;
    let (first, rest) = s.split_at(head);
    if !first.is_empty() {
      x = U256::from_u128(first.parse::<u128>().ok()?);
    }
    for chunk in rest.as_bytes().chunks(19) {
      let c: u128 = std::str::from_utf8(chunk).ok()?.parse().ok()?;
      x = x.checked_mul_u128(pow10(19))?.checked_add(U256::from_u128(c))?;
    }
    Some(x)
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
    Wide::NaN => (V::NaN(false), false),
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

/// Magnitude digits and exponent of a backend result (for the cohort decision).
fn wide_digits(w: Wide) -> Option<(String, i64)> {
  match w {
    Wide::Rounded(_, c, e, _) => Some((c.to_string(), e)),
    Wide::Digits(_, digits, e) => Some((digits, e)),
    _ => None,
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

/// Pads the coefficient of an (inexact) result with zeros up to 34 digits.
fn full_precision(v: V) -> V {
  match v {
    V::Fin(_, c, _) if c != 0 => ideal(v, false, EXP_MIN as i32),
    other => other,
  }
}

/// Chooses the cohort (exponent) of a correctly rounded result of `exp` or `ln`
/// the way Intel's `bid128_exp` / `bid128_log` do.
///
/// Intel evaluates these functions in binary128 and converts the binary result `B`
/// with `binary128_to_bid128`, which returns:
/// - an integer `B < 10^34` with exponent 0 (e.g. `2E+0`),
/// - a dyadic fraction `B = c / 2^a` (`a <= 48`, `c * 5^a < 10^34`) with exponent `-a` (e.g. `5E-1`),
/// - any other value rounded to the full 34 digits (e.g. `1.000000000000000000000000000000000`).
///
/// `v` is the correctly rounded 34-digit result and `wide` recomputes it with
/// [COHORT_PRECISION] digits (only called when `v` is an integer or a short dyadic fraction).
/// When the wide value `w` rounds to exactly `v` in binary128 (i.e. `|w - v|` is below half
/// a binary128 ulp of `v`), Intel's exact cohort is used, otherwise the result is padded to
/// 34 digits.
fn binary128_cohort(v: V, wide: impl FnOnce() -> Option<(String, i64)>) -> V {
  let V::Fin(neg, c, e) = v else { return v };
  if c == 0 {
    return v;
  }
  // value with trailing zeros removed: c0 * 10^e0
  let (mut c0, mut e0) = (c, e);
  while c0 % 10 == 0 {
    c0 /= 10;
    e0 += 1;
  }
  // (coefficient, exponent) of Intel's exact conversion, binary exponent E (2^E <= |v| < 2^(E+1)),
  // and the flag telling whether |v| is a power of two
  let (rc, re, binary_exp, power_of_two) = if e0 >= 0 {
    if digit_count(c0) as i32 + e0 > PRECISION as i32 {
      return full_precision(v);
    }
    let n = c0 * pow10(e0 as u32);
    (n, 0, (127 - n.leading_zeros()) as i32, n.is_power_of_two())
  } else {
    let a = (-e0) as u32;
    if a > 48 || c0 % 5_u128.pow(a) != 0 {
      return full_precision(v);
    }
    let odd = c0 / 5_u128.pow(a);
    (c0, e0, (127 - odd.leading_zeros()) as i32 - a as i32, odd == 1)
  };
  let Some((w_digits, ew)) = wide() else { return full_precision(v) };
  // |w - v| = d * 10^emin with d an integer
  let emin = ew.min(re as i64);
  let scaled = |digits: Option<U256>, exp: i64| digits.and_then(|x| x.checked_mul_pow10((exp - emin) as u32));
  let (Some(wu), Some(vu)) = (scaled(U256::from_decimal(&w_digits), ew), scaled(Some(U256::from_u128(rc)), re as i64)) else {
    return full_precision(v);
  };
  let below = wu < vu;
  let d = if below { vu.checked_sub(wu) } else { wu.checked_sub(vu) }.unwrap_or(U256::ZERO);
  // half a binary128 ulp (113-bit significand) is 2^(E-113), or 2^(E-114) just below a power of two
  let shift = 113 - binary_exp + if power_of_two && below { 1 } else { 0 };
  // within: d * 10^emin * 2^shift < 1
  let within = if d.is_zero() {
    true
  } else if emin >= 0 {
    false
  } else {
    matches!((d.checked_shl(shift as u32), U256::pow10((-emin) as u32)), (Some(x), Some(limit)) if x < limit)
  };
  if within { V::Fin(neg, rc, re) } else { full_precision(v) }
}

fn is_zero_v(v: V) -> bool {
  matches!(v, V::Fin(_, 0, _))
}

/// Sign bit of the value (like `bid128_is_signed`).
fn is_signed_v(v: V) -> bool {
  matches!(v, V::Fin(true, _, _) | V::Inf(true) | V::NaN(true))
}

fn negate(v: V) -> V {
  match v {
    V::NaN(s) => V::NaN(!s),
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
    return (V::NaN(false), false);
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
    None => (V::NaN(false), false),
  }
}

fn add_v(a: V, b: V) -> V {
  match (a, b) {
    (V::NaN(s), _) | (_, V::NaN(s)) => V::NaN(s),
    (V::Inf(x), V::Inf(y)) => {
      if x == y {
        V::Inf(x)
      } else {
        V::NaN(false)
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
  // like bid128_sub: the sign of a NaN subtrahend is not inverted
  match b {
    V::NaN(_) => add_v(a, b),
    _ => add_v(a, negate(b)),
  }
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
    (V::NaN(s), _) | (_, V::NaN(s)) => V::NaN(s),
    (V::Inf(x), V::Inf(y)) => V::Inf(x != y),
    (V::Inf(x), V::Fin(y, c, _)) | (V::Fin(y, c, _), V::Inf(x)) => {
      if c == 0 {
        V::NaN(false)
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
    (V::NaN(s), _) | (_, V::NaN(s)) => V::NaN(s),
    (V::Inf(_), V::Inf(_)) => V::NaN(false),
    (V::Inf(x), V::Fin(y, _, _)) => V::Inf(x != y),
    (V::Fin(x, _, _), V::Inf(y)) => V::Fin(x != y, 0, EXP_MIN as i32),
    (V::Fin(sa, ca, ea), V::Fin(sb, cb, eb)) => {
      if cb == 0 {
        return if ca == 0 { V::NaN(false) } else { V::Inf(sa != sb) };
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

/// Multiplies the value by 10^n (exact unless out of range).
///
/// Like `dfp_number_sys::bid128_scalbn` (the binding used by the Intel backend),
/// `n` is first clamped to `-6176..=6144`. This clamping is observable through
/// `new`, `round` and the scale-based rounding functions (e.g. `ceiling(0.25, 6145)`
/// is `0.025` on both backends), so it is reproduced here for bit-identical results.
///
/// Above the maximum exponent the coefficient is padded with trailing zeros up to the
/// full 34 digits before the result is declared infinite, exactly like every other
/// operation here: `scalbn(999999999999999, 6130)` is
/// `9999999999999990000000000000000000E+6111`, the same value `FromStr`, `+` and `*`
/// produce for `999999999999999E+6130`.
///
/// Intel's `bid128_scalbn` deliberately is *not* reproduced in this respect: it shifts
/// the coefficient left only while its high 64-bit word is below that of 10^33 (i.e.
/// while `c < 0x314dc6448d93 * 2^64`), so it reports overflow for values that are
/// representable (`new(999999999999999, -6130)`, `floor(999999999999999E+6121, 9)` are
/// `+Inf` on Intel). The mathematically correct finite value is returned instead; this
/// is the one known place where this core differs from Intel on an exact operation.
fn scalbn_v(v: V, n: i32) -> V {
  let n = n.clamp(-6176, 6144);
  match v {
    V::Fin(s, c, e) => round_pack_u128(s, c, e as i64 + n as i64).0,
    other => other,
  }
}

/// Rounds the value to integral value (exponent >= 0 left unchanged, otherwise exponent 0).
fn round_integral_v(v: V, mode: Rnd) -> V {
  match v {
    V::Fin(s, c, e) if e < 0 => {
      let k = (-e) as u32;
      let (q, r, divisor) = if k > 38 { (0, c, None) } else { (c / pow10(k), c % pow10(k), Some(pow10(k))) };
      let up = if r == 0 {
        false
      } else {
        match mode {
          Rnd::Zero => false,
          Rnd::Ceiling => !s,
          Rnd::Floor => s,
          Rnd::HalfEven | Rnd::HalfAway => {
            let cmp = match divisor {
              // r compared to divisor/2 (divisor is a power of ten, so even)
              Some(d) => r.cmp(&(d / 2)),
              None => Ordering::Less,
            };
            match cmp {
              Ordering::Greater => true,
              Ordering::Less => false,
              Ordering::Equal => matches!(mode, Rnd::HalfAway) || q % 2 == 1,
            }
          }
        }
      };
      V::Fin(s, if up { q + 1 } else { q }, 0)
    }
    other => other,
  }
}

/// Quantize: rounds the value (half-even) to have the exponent of `q`.
fn quantize_v(v: V, q: V) -> V {
  match (v, q) {
    (V::NaN(s), _) | (_, V::NaN(s)) => V::NaN(s),
    (V::Inf(s), V::Inf(_)) => V::Inf(s),
    (V::Inf(_), _) | (_, V::Inf(_)) => V::NaN(false),
    (V::Fin(s, c, e), V::Fin(_, _, eq)) => {
      if e >= eq {
        let shift = (e - eq) as u32;
        if c == 0 {
          return V::Fin(s, 0, eq);
        }
        if digit_count(c) + shift > PRECISION as u32 {
          return V::NaN(false);
        }
        V::Fin(s, c * pow10(shift), eq)
      } else {
        // round to integral at the position of the quantum exponent
        match round_integral_v(V::Fin(s, c, e - eq), Rnd::HalfEven) {
          V::Fin(s, c, _) => {
            if digit_count(c) > PRECISION as u32 {
              V::NaN(false)
            } else {
              V::Fin(s, c, eq)
            }
          }
          other => other,
        }
      }
    }
  }
}

/// Compares two values, returns `None` when unordered (NaN involved).
fn cmp_v(a: V, b: V) -> Option<Ordering> {
  match (a, b) {
    (V::NaN(_), _) | (_, V::NaN(_)) => None,
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
  match round_integral_v(v, Rnd::Zero) {
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
    return (if rest == "inf" || rest == "infinity" { V::Inf(neg) } else { V::NaN(false) }, false);
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
      return (V::NaN(false), false);
    }
    i += 1;
    let c = at(i);
    if !c.is_ascii_digit() && !((c == b'+' || c == b'-') && at(i + 1).is_ascii_digit()) {
      return (V::NaN(false), false);
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

/// Converts the value into the string in the same format as Intel's `bid128_to_string`.
fn to_intel_string(v: V) -> String {
  match v {
    V::NaN(s) => if s { "-NaN" } else { "+NaN" }.to_string(),
    V::Inf(s) => if s { "-Inf" } else { "+Inf" }.to_string(),
    V::Fin(s, c, e) => format!("{}{}E{}{}", if s { '-' } else { '+' }, c, if e >= 0 { "+" } else { "" }, e),
  }
}

/// Intel's `bid128_ilogb`.
fn ilogb_v(v: V) -> i32 {
  match v {
    V::Fin(_, 0, _) | V::NaN(_) => i32::MIN,
    V::Inf(_) => i32::MAX,
    V::Fin(_, c, e) => digit_count(c) as i32 - 1 + e,
  }
}

/// Exponential function of a finite, non-zero value.
fn exp_v(s: bool, c: u128, e: i32) -> V {
  // e^x overflows decimal128 for x > ~14149.7 and underflows to zero for x < ~-14224
  if c as f64 * 10f64.powi(e) > 14300.0 {
    return if s { V::Fin(false, 0, EXP_MIN as i32) } else { V::Inf(false) };
  }
  match pack(B::exp(s, c, e, PRECISION)).0 {
    // underflow, like Intel
    V::Fin(_, 0, _) => V::Fin(false, 0, EXP_MIN as i32),
    // the cohort follows Intel's binary128 -> decimal128 conversion (e.g. exp(1E-40) = 1E+0)
    v => binary128_cohort(v, || wide_digits(B::exp(s, c, e, COHORT_PRECISION))),
  }
}

/// Natural logarithm of a finite, positive value.
fn ln_v(c: u128, e: i32) -> V {
  let v = pack(B::ln(c, e, PRECISION)).0;
  // the cohort follows Intel's binary128 -> decimal128 conversion (e.g. ln(e^2) = 2E+0)
  binary128_cohort(v, || wide_digits(B::ln(c, e, COHORT_PRECISION)))
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
      PowStep::Undefined => return V::NaN(false),
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
  result.unwrap_or(V::NaN(false))
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
    return V::NaN(false);
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
const ONE_TENTH: V = V::Fin(false, 1, -1);

/// FEEL number.
#[derive(Copy, Clone)]
pub struct FeelNumber(V, bool);

impl FeelNumber {
  fn v(&self) -> V {
    self.0
  }

  fn of(v: V, flag: bool) -> Self {
    Self(v, flag)
  }

  /// Returns [FeelNumber] value infinite.
  pub fn infinite() -> FeelNumber {
    Self::of(V::Inf(false), false)
  }

  /// Returns [FeelNumber] value 0 (zero).
  pub fn zero() -> FeelNumber {
    Self::of(ZERO, false)
  }

  /// Returns [FeelNumber] value 1 (one).
  pub fn one() -> FeelNumber {
    Self::of(ONE, false)
  }

  /// Returns [FeelNumber] value 2 (two).
  pub fn two() -> FeelNumber {
    Self::of(TWO, false)
  }

  /// Returns [FeelNumber] value 1_000_000_000 (billion).
  pub fn billion() -> FeelNumber {
    Self::of(V::Fin(false, 1_000_000_000, 0), false)
  }

  /// Creates a new [FeelNumber] from integer value and a scale.
  pub fn new(n: i64, s: i32) -> Self {
    Self::of(scalbn_v(V::Fin(n < 0, n.unsigned_abs() as u128, 0), s.saturating_neg()), false)
  }

  /// Creates a new [FeelNumber] from [isize].
  fn from_isize(n: isize) -> Self {
    Self::of(V::Fin(n < 0, n.unsigned_abs() as u128, 0), false)
  }

  /// Returns an absolute value of this [FeelNumber].
  pub fn abs(&self) -> Self {
    let v = match self.v() {
      V::Fin(_, c, e) => V::Fin(false, c, e),
      V::Inf(_) => V::Inf(false),
      V::NaN(_) => V::NaN(false),
    };
    Self::of(v, false)
  }

  /// Returns a nearest integer greater or equal to this [FeelNumber].
  pub fn ceiling(&self, scale: i32) -> Result<Self> {
    Ok(Self::of(
      if scale == 0 {
        let rounded = round_integral_v(self.v(), Rnd::Ceiling);
        if is_zero_v(rounded) { ZERO } else { rounded }
      } else {
        if self.check_scale(scale)? {
          self.unscale(round_integral_v(scalbn_v(self.v(), scale), Rnd::Ceiling), scale)
        } else {
          self.v()
        }
      },
      true,
    ))
  }

  pub fn even(&self) -> bool {
    rem_two_is_zero(self.v())
  }

  pub fn exp(&self) -> Option<Self> {
    let n = match self.v() {
      V::NaN(s) => V::NaN(s),
      V::Inf(false) => V::Inf(false),
      V::Inf(true) => ZERO,
      V::Fin(_, 0, _) => ONE,
      V::Fin(s, c, e) => exp_v(s, c, e),
    };
    if is_finite_v(n) { Some(Self::of(n, true)) } else { None }
  }

  /// Returns a nearest integer less or equal to this [FeelNumber].
  pub fn floor(&self, scale: i32) -> Result<Self> {
    Ok(Self::of(
      if scale == 0 {
        round_integral_v(self.v(), Rnd::Floor)
      } else {
        if self.check_scale(scale)? {
          self.unscale(round_integral_v(scalbn_v(self.v(), scale), Rnd::Floor), scale)
        } else {
          self.v()
        }
      },
      true,
    ))
  }

  pub fn frac(&self) -> Self {
    let v = self.v();
    Self::of(sub_v(v, round_integral_v(v, Rnd::Zero)), true)
  }

  pub fn is_integer(&self) -> bool {
    let v = self.v();
    eq_v(v, round_integral_v(v, Rnd::Zero))
  }

  pub fn is_zero(&self) -> bool {
    eq_v(self.v(), ZERO)
  }

  /// Returns `true` when this number is finite (not an infinity and not NaN).
  ///
  /// Arithmetic on decimal128 values can overflow to infinity (e.g. `9e6144 * 10`) or produce NaN
  /// (e.g. a failed quantize in `decimal(1e33, 1)`); FEEL has no such numbers, so evaluators
  /// turn non-finite results into null.
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
      V::NaN(s) => V::NaN(s),
      V::Inf(true) => V::NaN(false),
      V::Inf(false) => V::Inf(false),
      V::Fin(_, 0, _) => V::Inf(true),
      V::Fin(true, _, _) => V::NaN(false),
      V::Fin(false, c, e) => ln_v(c, e),
    };
    if is_finite_v(n) { Some(Self::of(n, true)) } else { None }
  }

  pub fn odd(&self) -> bool {
    if self.is_integer() { !rem_two_is_zero(self.v()) } else { false }
  }

  pub fn pow(&self, rhs: &FeelNumber) -> Option<Self> {
    let n = pow_v(self.v(), rhs.v());
    if is_finite_v(n) { Some(Self::of(n, true)) } else { None }
  }

  pub fn round(&self, rhs: &FeelNumber) -> Self {
    let scale = to_signed(rhs.v(), i32::MIN as i128, i32::MAX as i128).unwrap_or(i32::MIN as i128) as i32;
    let quantifier = scalbn_v(ONE, scale.saturating_neg());
    let quantized = quantize_v(self.v(), quantifier);
    // A coefficient can not be widened to the requested quantum when that needs more than 34
    // digits (`decimal(5.5, 6176)`); the value itself is already exact at that scale, so it is
    // returned unchanged instead of NaN.
    if matches!(quantized, V::NaN(_)) && self.settled_at(scale) {
      return Self::of(self.v(), false);
    }
    Self::of(quantized, false)
  }

  pub fn round_down(&self, scale: i32) -> Result<Self> {
    Ok(Self::of(
      if scale == 0 {
        round_integral_v(self.v(), Rnd::Zero)
      } else {
        if self.check_scale(scale)? {
          self.unscale(round_integral_v(scalbn_v(self.v(), scale), Rnd::Zero), scale)
        } else {
          self.v()
        }
      },
      true,
    ))
  }

  pub fn round_half_down(&self, scale: i32) -> Result<Self> {
    let positive = |n| round_integral_v(sub_v(n, ONE_TENTH), Rnd::HalfAway);
    let negative = |n| round_integral_v(add_v(n, ONE_TENTH), Rnd::HalfAway);
    let signed = is_signed_v(self.0);
    Ok(Self::of(
      if scale == 0 {
        let v = self.v();
        if signed { negative(v) } else { positive(v) }
      } else {
        if self.check_scale(scale)? {
          let scaled = scalbn_v(self.v(), scale);
          self.unscale(if signed { negative(scaled) } else { positive(scaled) }, scale)
        } else {
          self.v()
        }
      },
      true,
    ))
  }

  pub fn round_half_up(&self, scale: i32) -> Result<Self> {
    Ok(Self::of(
      if scale == 0 {
        round_integral_v(self.v(), Rnd::HalfAway)
      } else {
        if self.check_scale(scale)? {
          self.unscale(round_integral_v(scalbn_v(self.v(), scale), Rnd::HalfAway), scale)
        } else {
          self.v()
        }
      },
      true,
    ))
  }

  pub fn round_up(&self, scale: i32) -> Result<Self> {
    let mode = if is_signed_v(self.0) { Rnd::Floor } else { Rnd::Ceiling };
    Ok(Self::of(
      if scale == 0 {
        round_integral_v(self.v(), mode)
      } else {
        if self.check_scale(scale)? {
          self.unscale(round_integral_v(scalbn_v(self.v(), scale), mode), scale)
        } else {
          self.v()
        }
      },
      true,
    ))
  }

  pub fn sqrt(&self) -> Option<Self> {
    let n = match self.v() {
      V::NaN(s) => V::NaN(s),
      V::Inf(true) => V::NaN(false),
      V::Inf(false) => V::Inf(false),
      V::Fin(s, 0, e) => V::Fin(s, 0, e.div_euclid(2)),
      V::Fin(true, _, _) => V::NaN(false),
      V::Fin(false, c, e) => sqrt_v(c, e),
    };
    if is_finite_v(n) { Some(Self::of(n, true)) } else { None }
  }

  pub fn square(&self) -> Option<Self> {
    let n = pow_v(self.v(), TWO);
    if is_finite_v(n) { Some(Self::of(n, true)) } else { None }
  }

  pub fn trunc(&self) -> Self {
    Self::of(round_integral_v(self.v(), Rnd::Zero), false)
  }

  /// Parses a number like [FromStr::from_str] and also reports whether the value had
  /// to be rounded (more than 34 significant digits, or an exponent so small that the
  /// value underflows to a subnormal number or to zero).
  pub fn from_str_reporting_rounding(s: &str) -> Result<(Self, bool), DsntkError> {
    let (n, inexact) = parse_v(s);
    if is_finite_v(n) {
      Ok((Self::of(n, false), inexact))
    } else {
      Err(err_invalid_number_literal(s))
    }
  }

  /// Calculates the remainder of the division.
  fn remainder(&self, rhs: V) -> V {
    let mut n = div_v(self.v(), rhs);
    n = round_integral_v(n, Rnd::Floor);
    n = mul_v(rhs, n);
    sub_v(self.v(), n)
  }

  /// Checks the scale of a scale-based rounding function (`round up(n, scale)` etc.).
  ///
  /// `Err` when `scale` is outside the FEEL range `-6111..=6176`. `Ok(true)` when the rounding
  /// can be done on the scaled intermediate `n * 10^scale` (its adjusted exponent fits
  /// decimal128). `Ok(false)` when that intermediate would not fit but `n` has no digit below
  /// `10^-scale`, so rounding to `scale` leaves it unchanged: `round up(5.5, 6176)` is `5.5`
  /// (TCK 1141-1144). The remaining case, a value far below `10^-scale` with a negative
  /// scale (`1E-6176` at scale `-6111`), stays an error.
  fn check_scale(&self, scale: i32) -> Result<bool> {
    if !(MIN_SCALE..=MAX_SCALE).contains(&scale) {
      return Err(err_invalid_scale(MIN_SCALE, MAX_SCALE, scale));
    }
    let scaled_exp = ilogb_v(self.v()) as i64 + scale as i64;
    if (MIN_SCALE as i64..=SCALED_EXP_MAX).contains(&scaled_exp) {
      Ok(true)
    } else if self.settled_at(scale) {
      Ok(false)
    } else {
      Err(err_invalid_scale(MIN_SCALE, MAX_SCALE, scale))
    }
  }

  /// `true` when this finite, non-zero number has no digit below `10^-scale`, i.e. rounding
  /// it to `scale` fraction digits can not change its value.
  fn settled_at(&self, scale: i32) -> bool {
    matches!(self.v(), V::Fin(_, c, e) if c != 0 && i64::from(e) >= -i64::from(scale))
  }

  /// Undoes the scaling applied before rounding: `n * 10^-scale`.
  ///
  /// Zero is returned as a plain zero (no exponent). Every other rounded integral is scaled
  /// back, also when it is `1` or `-1`: an earlier special case returned `±1` unscaled, so
  /// `round up(0.1, 1)` was `1` instead of `0.1` and `floor(-1.56, -6111)` was `-1` instead
  /// of `-1E+6111`.
  fn unscale(&self, n: V, scale: i32) -> V {
    if is_zero_v(n) { ZERO } else { scalbn_v(n, scale.saturating_neg()) }
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
  // a NaN operand is propagated, the base first
  if let V::NaN(s) = x {
    return V::NaN(s);
  }
  if let V::NaN(s) = y {
    return V::NaN(s);
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
  let y_integer = eq_v(y, round_integral_v(y, Rnd::HalfEven));
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
    return V::NaN(false);
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
  let V::Fin(_, cx, ex) = x else { return V::NaN(false) };
  let V::Fin(sy, cy, ey) = y else { return V::NaN(false) };
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
  match full_precision(pow_exp_log(cx, ex, sy, cy, ey)) {
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
    Self::of(add_v(self.v(), rhs.v()), true)
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
    Self::of(sub_v(self.v(), rhs.v()), true)
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
    Self::of(mul_v(self.v(), rhs.v()), true)
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
    Self::of(div_v(self.v(), rhs.v()), true)
  }
}

impl DivAssign<FeelNumber> for FeelNumber {
  fn div_assign(&mut self, rhs: Self) {
    *self = *self / rhs;
  }
}

impl Rem<FeelNumber> for FeelNumber {
  type Output = Self;
  fn rem(self, rhs: Self) -> Self::Output {
    Self::of(self.remainder(rhs.v()), true)
  }
}

impl RemAssign<FeelNumber> for FeelNumber {
  fn rem_assign(&mut self, rhs: Self) {
    *self = *self % rhs;
  }
}

impl Neg for FeelNumber {
  type Output = Self;
  fn neg(self) -> Self::Output {
    Self::of(negate(self.v()), true)
  }
}

impl Debug for FeelNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", to_intel_string(self.v()))
  }
}

impl Display for FeelNumber {
  /// Converts [FeelNumber] to human readable string.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Infinity and NaN: FEEL has no such numbers, but never panic while formatting them
    // (a panic aborts the whole engine on wasm32).
    let (negative, c, exponent) = match self.v() {
      V::NaN(_) => return f.pad("NaN"),
      V::Inf(negative) => return f.pad(if negative { "-Infinity" } else { "Infinity" }),
      V::Fin(negative, c, e) => (negative, c, e as isize),
    };
    // A zero with a huge exponent (e.g. `0 * 1e-6000` or `decimal(0, 6000)`) would otherwise be
    // printed with thousands of zeros; print it as a plain `0` instead.
    if c == 0 && !(-34..=0).contains(&exponent) && f.precision().is_none() {
      return f.pad_integral(true, "", "0");
    }
    let sb = c.to_string();
    let decimal_points = exponent.unsigned_abs();
    let (mut before, mut after) = if exponent < 0 {
      let digit_count = sb.len();
      if digit_count <= decimal_points {
        let before = "0".to_string();
        let mut after = "0".repeat(decimal_points - digit_count);
        if self.1 {
          after.push_str(sb.trim_end_matches('0'));
        } else {
          after.push_str(&sb);
        }
        (before, after)
      } else {
        let before = sb[..digit_count - decimal_points].to_string();
        let after = if self.1 {
          sb[digit_count - decimal_points..].trim_end_matches('0').to_string()
        } else {
          sb[digit_count - decimal_points..].to_string()
        };
        (before, after)
      }
    } else {
      let mut before = sb;
      before.push_str(&"0".repeat(decimal_points));
      let after = "".to_string();
      (before, after)
    };
    if let Some(precision) = f.precision() {
      if after.len() < precision {
        after.push_str(&"0".repeat(precision - after.len()));
      } else {
        after = after[0..precision].to_string();
      }
    }
    if !after.is_empty() {
      before.push('.');
      before.push_str(&after);
    }
    f.pad_integral(!negative, "", &before)
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
    if is_finite_v(n) { Ok(Self::of(n, false)) } else { Err(err_invalid_number_literal(s)) }
  }
}

macro_rules! from_unsigned {
  ($($t:ty),*) => {
    $(
      impl From<$t> for FeelNumber {
        fn from(value: $t) -> Self {
          Self::of(V::Fin(false, value as u128, 0), false)
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
          Self::of(V::Fin(value < 0, (value as i128).unsigned_abs(), 0), false)
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
