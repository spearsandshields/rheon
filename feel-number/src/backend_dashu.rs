//! # dashu-float backend
//!
//! Implements the inexact operations of the decimal128 core ([Backend]) with the pure-Rust
//! [dashu-float](https://crates.io/crates/dashu-float) crate: `FBig<HalfEven, 10>` at
//! precision [PRECISION] (34 digits) for `/`, `exp` and `ln`, and at the working precision
//! the core asks for in the rounds of `x^y = exp(y ln x)`.
//!
//! dashu is only ever called through its `Context` API, which reports domain errors,
//! infinite operands, `0/0`, overflow and underflow as `Err` values instead of panicking
//! (the operator and method API panics). dashu returns the correctly rounded value together
//! with the adjustment it applied to the truncated significand; that adjustment tells on
//! which side of the rounded value the exact result lies, which is what [Wide::Rounded]
//! carries to the core for its single final rounding.
//!
//! dashu's own `sqrt`, `powf` and `powi` are not used: `FBig::sqrt` rounds up when the
//! integer root's remainder equals the root, `powf` mis-certifies near ties and never
//! terminates on exact ties, and `powi` has the same problem for 32..64-bit exponents.
//! The core computes `sqrt` exactly and drives `pow` through [Backend::pow_step].

use crate::decimal128::{Backend, POW_T_OVERFLOW, POW_T_UNDERFLOW, PRECISION, PowStep, Wide, digit_count};
use dashu_base::Approximation::{Exact, Inexact};
use dashu_base::Sign;
use dashu_float::round::Rounded;
use dashu_float::round::Rounding;
use dashu_float::round::mode::HalfEven;
use dashu_float::{ConstCache, Context, FBig, FpError, FpResult, Repr};
use dashu_int::IBig;
use std::cell::RefCell;
use std::cmp::Ordering;

/// dashu decimal float with half-even rounding.
type F = FBig<HalfEven, 10>;

/// dashu context with the given precision (0 = unlimited, exact) and half-even rounding.
fn ctx(precision: usize) -> Context<HalfEven> {
  Context::<HalfEven>::new(precision)
}

thread_local! {
  /// Cache of the constants (ln 2, ln 10) dashu's `exp` and `ln` need; without it every
  /// call recomputes them (about 15% of the cost of `exp`).
  static CACHE: RefCell<ConstCache> = RefCell::new(ConstCache::default());
}

fn with_cache<T>(f: impl FnOnce(&mut ConstCache) -> T) -> T {
  CACHE.with(|cache| f(&mut cache.borrow_mut()))
}

/// Converts a finite value into dashu's representation (exact).
fn repr_of(neg: bool, c: u128, e: i32) -> Repr<10> {
  let significand = IBig::from(c);
  Repr::new(if neg { -significand } else { significand }, e as isize)
}

/// Result of an exact (unlimited precision) dashu operation.
fn exact(r: FpResult<F>) -> Option<F> {
  r.ok().map(|rounded| rounded.value())
}

/// Converts the result of a dashu operation for the core.
///
/// `Err` values (the cases where dashu's convenience API would panic) are mapped to what the
/// Intel library returns: NaN for domain errors, `0/0` and infinite operands; infinity and
/// zero for overflow and underflow (dashu's exponent is an `isize`, so these two only happen
/// for astronomically large or small results).
fn wide(r: FpResult<F>) -> Wide {
  match r {
    Ok(rounded) => wide_rounded(rounded),
    Err(FpError::Overflow(sign)) => Wide::Inf(sign == Sign::Negative),
    Err(FpError::Underflow(sign)) => Wide::Rounded(sign == Sign::Negative, 0, i64::MIN, Ordering::Greater),
    Err(_) => Wide::NaN,
  }
}

/// Converts a rounded dashu value: an inexact value with at most [PRECISION] digits comes
/// with the side the exact magnitude lies on (`|x| < |v|` when the magnitude was incremented,
/// `|x| > |v|` otherwise), anything else is passed as digits.
fn wide_rounded(r: Rounded<F>) -> Wide {
  let (f, adjustment) = match r {
    Exact(f) => (f, None),
    Inexact(f, adjustment) => (f, Some(adjustment)),
  };
  let repr = f.into_repr();
  if repr.is_infinite() {
    return Wide::Inf(repr.sign() == Sign::Negative);
  }
  let (significand, exp) = repr.into_parts();
  let (sign, magnitude) = significand.into_parts();
  let neg = sign == Sign::Negative;
  let exp = exp as i64;
  match (adjustment, u128::try_from(&magnitude)) {
    (None, Ok(c)) if digit_count(c) <= PRECISION as u32 => Wide::Rounded(neg, c, exp, Ordering::Equal),
    (Some(adjustment), Ok(c)) if c != 0 && digit_count(c) <= PRECISION as u32 => {
      let magnitude_incremented = matches!((adjustment, neg), (Rounding::AddOne, false) | (Rounding::SubOne, true));
      Wide::Rounded(neg, c, exp, if magnitude_incremented { Ordering::Less } else { Ordering::Greater })
    }
    (_, Ok(c)) => Wide::Digits(neg, c.to_string(), exp),
    (_, Err(_)) => Wide::Digits(neg, magnitude.to_string(), exp),
  }
}

/// The dashu-float backend.
pub(crate) struct Dashu;

impl Backend for Dashu {
  fn div(ca: u128, ea: i32, cb: u128, eb: i32) -> Wide {
    wide(ctx(PRECISION).div(&repr_of(false, ca, ea), &repr_of(false, cb, eb)))
  }

  fn exp(neg: bool, c: u128, e: i32, precision: usize) -> Wide {
    let x = repr_of(neg, c, e);
    wide(with_cache(|cache| ctx(precision).exp(&x, Some(cache))))
  }

  fn ln(c: u128, e: i32, precision: usize) -> Wide {
    let x = repr_of(false, c, e);
    wide(with_cache(|cache| ctx(precision).ln(&x, Some(cache))))
  }

  fn pow_step(cx: u128, ex: i32, sy: bool, cy: u128, ey: i32, precision: usize) -> PowStep {
    let x = repr_of(false, cx, ex);
    let y = repr_of(sy, cy, ey);
    let c = ctx(precision);
    let Some(t) = with_cache(|cache| exact(c.ln(&x, Some(cache)))).and_then(|ln_x| exact(c.mul(ln_x.repr(), &y))) else {
      return PowStep::Undefined;
    };
    // an f64 estimate of log10(x) multiplied by a huge |y| (base close to one) is not
    // accurate enough for the range verdict, `t` itself is
    let t_approx = t.to_f64().value();
    if t_approx > POW_T_OVERFLOW {
      return PowStep::Overflow;
    }
    if t_approx < POW_T_UNDERFLOW {
      return PowStep::Underflow;
    }
    let Some(r) = with_cache(|cache| exact(c.exp(t.repr(), Some(cache)))) else {
      return PowStep::Undefined;
    };
    let (significand, e) = r.into_repr().into_parts();
    let (_, magnitude) = significand.into_parts();
    PowStep::Value(magnitude.to_string(), e as i64)
  }
}
