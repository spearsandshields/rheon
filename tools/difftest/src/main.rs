//! Op-by-op differential fuzzer: Intel BID128 FeelNumber vs dashu FeelNumber.
//!
//! Both backends are linked into this binary (crates/fn-intel, crates/fn-dashu), so every
//! input is fed to both and the `to_string()` (Display) outputs are compared.
//! Debug output (raw coefficient+exponent, i.e. cohort) is compared as a
//! secondary signal.
//!
//! Usage: difftest [iters_per_op] [only_op,...]      (run.sh builds, runs and judges)
//!        difftest eval <op> <a> [<b>] [<k>]           (one case on both backends)
//! Writes mismatches.tsv (all Display mismatches) and cohort.tsv (Debug-only) to the
//! current directory. DIFFTEST_SEED changes the seed base, DIFFTEST_TRACE=1 prints every input.
#![allow(dead_code)]
// rheon-feel-number's lib.rs compiled twice: with feature `intel` (crates/fn-intel) and
// with feature `dashu` (crates/fn-dashu).
mod intel {
  pub use fn_intel::FeelNumber;
}
mod fast {
  pub use fn_dashu::FeelNumber;
}

use dsntk_common::Jsonify;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;

// ---------------------------------------------------------------------------------------------
// Uniform interface over both backends
// ---------------------------------------------------------------------------------------------

trait Num:
  Sized
  + Copy
  + Display
  + Debug
  + Jsonify
  + PartialEq
  + PartialOrd
  + PartialEq<isize>
  + PartialOrd<isize>
  + Add<Output = Self>
  + Sub<Output = Self>
  + Mul<Output = Self>
  + Div<Output = Self>
  + Rem<Output = Self>
  + Neg<Output = Self>
  + AddAssign
  + SubAssign
  + MulAssign
  + DivAssign
  + RemAssign
  + std::str::FromStr<Err = dsntk_common::DsntkError>
  + From<u8>
  + From<i8>
  + From<u16>
  + From<i16>
  + From<u32>
  + From<i32>
  + From<u64>
  + From<i64>
  + From<usize>
  + From<isize>
{
  fn new(n: i64, s: i32) -> Self;
  fn zero() -> Self;
  fn one() -> Self;
  fn two() -> Self;
  fn billion() -> Self;
  fn infinite() -> Self;
  fn abs(&self) -> Self;
  fn ceiling(&self, s: i32) -> Result<Self, String>;
  fn floor(&self, s: i32) -> Result<Self, String>;
  fn round_down(&self, s: i32) -> Result<Self, String>;
  fn round_up(&self, s: i32) -> Result<Self, String>;
  fn round_half_up(&self, s: i32) -> Result<Self, String>;
  fn round_half_down(&self, s: i32) -> Result<Self, String>;
  fn even(&self) -> bool;
  fn odd(&self) -> bool;
  fn exp(&self) -> Option<Self>;
  fn ln(&self) -> Option<Self>;
  fn sqrt(&self) -> Option<Self>;
  fn square(&self) -> Option<Self>;
  fn pow(&self, y: &Self) -> Option<Self>;
  fn round(&self, y: &Self) -> Self;
  fn trunc(&self) -> Self;
  fn frac(&self) -> Self;
  fn is_integer(&self) -> bool;
  fn is_zero(&self) -> bool;
  fn is_one(&self) -> bool;
  fn is_negative(&self) -> bool;
  fn is_positive(&self) -> bool;
  fn conv(&self) -> String;
  fn conv_owned(self) -> String;
  fn isize_rev(k: isize, n: &Self) -> String;
}

macro_rules! impl_num {
  ($m:ident) => {
    impl Num for $m::FeelNumber {
      fn new(n: i64, s: i32) -> Self { $m::FeelNumber::new(n, s) }
      fn zero() -> Self { $m::FeelNumber::zero() }
      fn one() -> Self { $m::FeelNumber::one() }
      fn two() -> Self { $m::FeelNumber::two() }
      fn billion() -> Self { $m::FeelNumber::billion() }
      fn infinite() -> Self { $m::FeelNumber::infinite() }
      fn abs(&self) -> Self { $m::FeelNumber::abs(self) }
      fn ceiling(&self, s: i32) -> Result<Self, String> { $m::FeelNumber::ceiling(self, s).map_err(|e| e.to_string()) }
      fn floor(&self, s: i32) -> Result<Self, String> { $m::FeelNumber::floor(self, s).map_err(|e| e.to_string()) }
      fn round_down(&self, s: i32) -> Result<Self, String> { $m::FeelNumber::round_down(self, s).map_err(|e| e.to_string()) }
      fn round_up(&self, s: i32) -> Result<Self, String> { $m::FeelNumber::round_up(self, s).map_err(|e| e.to_string()) }
      fn round_half_up(&self, s: i32) -> Result<Self, String> { $m::FeelNumber::round_half_up(self, s).map_err(|e| e.to_string()) }
      fn round_half_down(&self, s: i32) -> Result<Self, String> { $m::FeelNumber::round_half_down(self, s).map_err(|e| e.to_string()) }
      fn even(&self) -> bool { $m::FeelNumber::even(self) }
      fn odd(&self) -> bool { $m::FeelNumber::odd(self) }
      fn exp(&self) -> Option<Self> { $m::FeelNumber::exp(self) }
      fn ln(&self) -> Option<Self> { $m::FeelNumber::ln(self) }
      fn sqrt(&self) -> Option<Self> { $m::FeelNumber::sqrt(self) }
      fn square(&self) -> Option<Self> { $m::FeelNumber::square(self) }
      fn pow(&self, y: &Self) -> Option<Self> { $m::FeelNumber::pow(self, y) }
      fn round(&self, y: &Self) -> Self { $m::FeelNumber::round(self, y) }
      fn trunc(&self) -> Self { $m::FeelNumber::trunc(self) }
      fn frac(&self) -> Self { $m::FeelNumber::frac(self) }
      fn is_integer(&self) -> bool { $m::FeelNumber::is_integer(self) }
      fn is_zero(&self) -> bool { $m::FeelNumber::is_zero(self) }
      fn is_one(&self) -> bool { $m::FeelNumber::is_one(self) }
      fn is_negative(&self) -> bool { $m::FeelNumber::is_negative(self) }
      fn is_positive(&self) -> bool { $m::FeelNumber::is_positive(self) }
      fn conv(&self) -> String {
        format!(
          "u8={:?} i8={:?} u16={:?} i16={:?} u32={:?} i32={:?} u64={:?} i64={:?} usize={:?} isize={:?}",
          u8::try_from(self).ok(),
          i8::try_from(self).ok(),
          u16::try_from(self).ok(),
          i16::try_from(self).ok(),
          u32::try_from(self).ok(),
          i32::try_from(self).ok(),
          u64::try_from(self).ok(),
          i64::try_from(self).ok(),
          usize::try_from(self).ok(),
          isize::try_from(self).ok()
        )
      }
      fn conv_owned(self) -> String {
        format!(
          "u8={:?} i8={:?} u16={:?} i16={:?} u32={:?} i32={:?} u64={:?} i64={:?} usize={:?} isize={:?}",
          u8::try_from(self).map_err(|e| e.to_string()),
          i8::try_from(self).map_err(|e| e.to_string()),
          u16::try_from(self).map_err(|e| e.to_string()),
          i16::try_from(self).map_err(|e| e.to_string()),
          u32::try_from(self).map_err(|e| e.to_string()),
          i32::try_from(self).map_err(|e| e.to_string()),
          u64::try_from(self).map_err(|e| e.to_string()),
          i64::try_from(self).map_err(|e| e.to_string()),
          usize::try_from(self).map_err(|e| e.to_string()),
          isize::try_from(self).map_err(|e| e.to_string())
        )
      }
      fn isize_rev(k: isize, n: &Self) -> String { format!("{} {:?}", k == *n, k.partial_cmp(n)) }
    }
  };
}
impl_num!(intel);
impl_num!(fast);

// ---------------------------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------------------------

/// (Display, Debug) of a result; panics are captured as "PANIC".
#[derive(Clone, PartialEq)]
struct Out {
  disp: String,
  dbg: String,
}

fn guard<T>(f: impl FnOnce() -> T) -> Result<T, ()> {
  catch_unwind(AssertUnwindSafe(f)).map_err(|_| ())
}

fn num<N: Num>(n: N) -> Out {
  let disp = guard(|| n.to_string()).unwrap_or_else(|_| "PANIC(display)".into());
  Out { disp, dbg: format!("{n:?}") }
}
fn opt<N: Num>(n: Option<N>) -> Out {
  match n {
    Some(n) => num(n),
    None => Out { disp: "None".into(), dbg: "None".into() },
  }
}
fn res<N: Num>(n: Result<N, String>) -> Out {
  match n {
    Ok(n) => num(n),
    Err(e) => Out { disp: format!("Err({e})"), dbg: format!("Err({e})") },
  }
}
fn txt(s: String) -> Out {
  Out { disp: s.clone(), dbg: s }
}
fn panicked(what: &str) -> Out {
  txt(format!("PANIC({what})"))
}

fn p<N: Num>(s: &str) -> Option<N> {
  s.parse::<N>().ok()
}

/// Evaluates op on backend N. `a`, `b` are string inputs; `k` is an integer side argument.
/// Returns None when an input cannot be parsed by this backend.
fn eval<N: Num>(op: &str, a: &str, b: &str, k: i64) -> Option<Out> {
  let r = guard(|| -> Option<Out> {
    Some(match op {
      "parse" => match a.parse::<N>() {
        Ok(n) => num(n),
        Err(e) => txt(format!("Err({e})")),
      },
      "add" => num(p::<N>(a)? + p::<N>(b)?),
      "sub" => num(p::<N>(a)? - p::<N>(b)?),
      "mul" => num(p::<N>(a)? * p::<N>(b)?),
      "div" => num(p::<N>(a)? / p::<N>(b)?),
      "rem" => num(p::<N>(a)? % p::<N>(b)?),
      "assign_ops" => {
        let (x, y) = (p::<N>(a)?, p::<N>(b)?);
        let mut v = vec![];
        let mut t = x;
        t += y;
        v.push(num(t));
        let mut t = x;
        t -= y;
        v.push(num(t));
        let mut t = x;
        t *= y;
        v.push(num(t));
        let mut t = x;
        t /= y;
        v.push(num(t));
        let mut t = x;
        t %= y;
        v.push(num(t));
        Out { disp: v.iter().map(|o| o.disp.clone()).collect::<Vec<_>>().join(" | "), dbg: v.iter().map(|o| o.dbg.clone()).collect::<Vec<_>>().join(" | ") }
      }
      "neg" => num(-p::<N>(a)?),
      "abs" => num(p::<N>(a)?.abs()),
      "trunc" => num(p::<N>(a)?.trunc()),
      "frac" => num(p::<N>(a)?.frac()),
      "sqrt" => opt(p::<N>(a)?.sqrt()),
      "square" => opt(p::<N>(a)?.square()),
      "exp" => opt(p::<N>(a)?.exp()),
      "ln" => opt(p::<N>(a)?.ln()),
      // FEEL log(n) (feel-evaluator core::log): n > 0 && ln(n) finite, else null
      "feel_log" => {
        let x = p::<N>(a)?;
        if x > N::zero() { opt(x.ln()) } else { txt("null".into()) }
      }
      "pow" | "pow_int" | "pow_frac" | "pow_special" => opt(p::<N>(a)?.pow(&p::<N>(b)?)),
      // FEEL modulo (core::modulo)
      "feel_modulo" => {
        let (x, y) = (p::<N>(a)?, p::<N>(b)?);
        if y.abs() == N::zero() { txt("null".into()) } else { num(x - y * (x / y).floor(0).unwrap()) }
      }
      // FeelNumber::round(&FeelNumber) (quantize) with arbitrary rhs
      "round_quantize" => num(p::<N>(a)?.round(&p::<N>(b)?)),
      // FEEL decimal(n, scale) (core::decimal)
      "feel_decimal" => {
        let (x, s) = (p::<N>(a)?, p::<N>(b)?);
        let s = s.trunc();
        if s >= -6111isize && s < 6176isize { num(x.round(&s)) } else { txt("null".into()) }
      }
      "ceiling" => res(p::<N>(a)?.ceiling(k as i32)),
      "floor" => res(p::<N>(a)?.floor(k as i32)),
      "round_down" => res(p::<N>(a)?.round_down(k as i32)),
      "round_up" => res(p::<N>(a)?.round_up(k as i32)),
      "round_half_up" => res(p::<N>(a)?.round_half_up(k as i32)),
      "round_half_down" => res(p::<N>(a)?.round_half_down(k as i32)),
      "cmp" => {
        let (x, y) = (p::<N>(a)?, p::<N>(b)?);
        txt(format!("{:?} lt={} le={} gt={} ge={} eq={} ne={}", x.partial_cmp(&y), x < y, x <= y, x > y, x >= y, x == y, x != y))
      }
      "cmp_isize" => {
        let x = p::<N>(a)?;
        let kk = k as isize;
        txt(format!("{} {:?} {}", x == kk, x.partial_cmp(&kk), N::isize_rev(kk, &x)))
      }
      "predicates" => {
        let x = p::<N>(a)?;
        txt(format!(
          "int={} zero={} one={} neg={} pos={} even={} odd={}",
          x.is_integer(),
          x.is_zero(),
          x.is_one(),
          x.is_negative(),
          x.is_positive(),
          guard(|| x.even()).map(|v| v.to_string()).unwrap_or("PANIC".into()),
          guard(|| x.odd()).map(|v| v.to_string()).unwrap_or("PANIC".into())
        ))
      }
      "format" => {
        let x = p::<N>(a)?;
        let pr = k as usize;
        let f = |s: &dyn Fn() -> String| guard(s).unwrap_or_else(|_| "PANIC".into());
        txt(format!(
          "[{}] [{}] [{}] [{}] [{}]",
          f(&|| format!("{x:.pr$}")),
          f(&|| format!("{x:+}")),
          f(&|| format!("{x:>45}")),
          f(&|| format!("{x:<45.3}")),
          f(&|| x.jsonify())
        ))
      }
      "to_int_ref" => txt(p::<N>(a)?.conv()),
      "to_int_owned" => txt(p::<N>(a)?.conv_owned()),
      "new" => num(N::new(k, b.parse::<i32>().unwrap())),
      "from_ints" => {
        let s = format!(
          "{} {} {} {} {} {} {} {} {} {}",
          N::from(k as u8),
          N::from(k as i8),
          N::from(k as u16),
          N::from(k as i16),
          N::from(k as u32),
          N::from(k as i32),
          N::from(k as u64),
          N::from(k),
          N::from(k as usize),
          N::from(k as isize)
        );
        let d = format!("{:?} {:?} {:?} {:?} {:?}", N::from(k as u64), N::from(k), N::from(k as i32), N::from(k as usize), N::from(k as isize));
        Out { disp: s, dbg: d }
      }
      "constants" => {
        let v = [N::zero(), N::one(), N::two(), N::billion()];
        let i = N::infinite();
        txt(format!(
          "{:?} {:?} inf_dbg={:?} inf_disp={} inf>1={} inf+1={:?} 1/0={:?} 0/0={:?}",
          v.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
          v.iter().map(|n| format!("{n:?}")).collect::<Vec<_>>(),
          i,
          guard(|| i.to_string()).unwrap_or("PANIC".into()),
          i > N::one(),
          i + N::one(),
          N::one() / N::zero(),
          N::zero() / N::zero()
        ))
      }
      // Random sequences of 3..8 operations on an accumulator (k = seed); every intermediate
      // Display/Debug is recorded, so state carried between ops (trim flag, cohort) is compared.
      "chain" | "chain_trans" => chain::<N>(a, k as u64, op == "chain_trans")?,
      _ => panic!("unknown op {op}"),
    })
  });
  match r {
    Ok(o) => o,
    Err(()) => Some(panicked("op")),
  }
}

fn chain<N: Num>(a: &str, seed: u64, trans: bool) -> Option<Out> {
  let mut r = Rng(seed);
  let mut x = p::<N>(a)?;
  let (mut d, mut g) = (vec![num(x).disp], vec![format!("{x:?}")]);
  let steps = 3 + r.below(6);
  for _ in 0..steps {
    let nops = if trans { 21 } else { 17 };
    let o = r.below(nops);
    let y = loop {
      let s = match r.below(3) {
        0 => gen_num(&mut r),
        1 => format!("{}{}.{}", sign(&mut r), r.below(1000), rdig(&mut r, 33)),
        _ => format!("{}{}", sign(&mut r), r.range(1, 99)),
      };
      if let Ok(v) = s.parse::<N>() {
        break v;
      }
    };
    let sc = r.range(-3, 36) as i32;
    let pw = if o == 15 { r.range(-6, 9) } else if o == 16 { r.range(-1000, 1000) } else if o == 20 { r.range(1, 99) } else { 0 };
    let ps = if o == 16 { r.range(-40, 40) as i32 } else { 0 };
    let x0 = x;
    let step = guard(|| -> Result<Option<N>, String> { let x = x0; match o {
      0 => Ok(Some(x + y)),
      1 => Ok(Some(x - y)),
      2 => Ok(Some(x * y)),
      3 => Ok(Some(x / y)),
      4 => Ok(Some(x % y)),
      5 => Ok(Some(-x)),
      6 => Ok(Some(x.abs())),
      7 => Ok(Some(x.trunc())),
      8 => Ok(Some(x.frac())),
      9 => Ok(x.sqrt()),
      10 => Ok(x.square()),
      11 => Ok(Some(x.round(&N::new(sc as i64, 0)))),
      12 => x.floor(sc).map(Some),
      13 => x.ceiling(sc).map(Some),
      14 => x.round_half_up(sc).map(Some),
      15 => Ok(x.pow(&N::new(pw, 0))),
      16 => Ok(Some(x + N::new(pw, ps))),
      17 => Ok(x.exp()),
      18 => Ok(x.ln()),
      19 => Ok(x.pow(&y)),
      _ => Ok(x.pow(&N::new(pw, 1))),
    }});
    let step = match step {
      Ok(s) => s,
      Err(()) => {
        d.push(format!("#{o}:PANIC"));
        g.push(format!("#{o}[{y:?};{sc};{pw};{ps}]:PANIC x={x:?}"));
        break;
      }
    };
    match step {
      Ok(Some(v)) => {
        x = v;
        d.push(num(x).disp);
        g.push(format!("#{o}[{y:?};{sc};{pw};{ps}]:{x:?}"));
        if !x.is_zero() && guard(|| x.to_string()).is_err() {
          break; // non-finite: stop the chain
        }
      }
      Ok(None) => {
        d.push(format!("#{o}:None"));
        g.push(format!("#{o}:None"));
        break;
      }
      Err(e) => {
        d.push(format!("#{o}:Err({e})"));
        g.push(format!("#{o}:Err({e})"));
        break;
      }
    }
  }
  Some(Out { disp: d.join(" -> "), dbg: g.join(" -> ") })
}

// ---------------------------------------------------------------------------------------------
// Input generation
// ---------------------------------------------------------------------------------------------

struct Rng(u64);
impl Rng {
  fn next(&mut self) -> u64 {
    // splitmix64
    self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
  }
  fn below(&mut self, n: u64) -> u64 {
    self.next() % n
  }
  fn range(&mut self, lo: i64, hi: i64) -> i64 {
    lo + (self.next() % ((hi - lo + 1) as u64)) as i64
  }
  fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
    v[self.below(v.len() as u64) as usize]
  }
  fn coin(&mut self, n: u64) -> bool {
    self.below(n) == 0
  }
}

fn digits(r: &mut Rng, n: usize) -> String {
  let mut s = String::with_capacity(n);
  for i in 0..n {
    let d = match r.below(10) {
      0 => 0,
      1 => 9,
      _ => r.below(10),
    };
    let d = if i == 0 && d == 0 { 1 + r.below(9) } else { d };
    s.push((b'0' + d as u8) as char);
  }
  s
}

fn sign(r: &mut Rng) -> &'static str {
  if r.coin(3) { "-" } else { "" }
}

/// coefficient digits + exponent with adjusted exponent `adj`
fn sci(neg: &str, coef: &str, adj: i64) -> String {
  let e = adj - (coef.len() as i64 - 1);
  format!("{neg}{coef}E{e:+}")
}

/// Place a decimal point inside a digit string (or use exponent).
fn place(r: &mut Rng, neg: &str, coef: &str, e: i64) -> String {
  if e < 0 && (-e as usize) < coef.len() && r.coin(2) {
    let p = coef.len() - (-e) as usize;
    format!("{neg}{}.{}", &coef[..p], &coef[p..])
  } else if e < 0 && (-e as usize) >= coef.len() && (-e) < 60 && r.coin(2) {
    format!("{neg}0.{}{}", "0".repeat((-e) as usize - coef.len()), coef)
  } else if e == 0 {
    format!("{neg}{coef}")
  } else {
    format!("{neg}{coef}E{e}")
  }
}

const STRUCTURED: &[&str] = &[
  "0", "1", "-1", "2", "-2", "3", "7", "10", "100", "1000", "0.1", "0.01", "0.5", "-0.5", "1.5", "2.5", "-2.5", "3.5", "0.25", "0.125", "0.05",
  "0.45", "0.55", "1.45", "1.55", "9.5", "99.5", "0.3333333333333333333333333333333333", "0.6666666666666666666666666666666667",
  "1.000000000000000000000000000000001", "0.9999999999999999999999999999999999", "9999999999999999999999999999999999",
  "-9999999999999999999999999999999999", "1000000000000000000000000000000000", "1E+33", "1E+34", "1E+35", "1E-33", "1E-34", "1E-35",
  "9.999999999999999999999999999999999E+6144", "1E+6144", "-1E+6144", "1E-6143", "1E-6176", "-1E-6176", "1E-6175", "5E-6177", "1.5E-6176",
  "2.5E-6176", "1E+6111", "1000000000000000000000000000000000E+6111", "12.2384283", "4.25", "2.718281828459045235360287471352662",
  "3.141592653589793238462643383279503", "1.414213562373095048801688724209698", "50.50", "-0.000", "0.0", "123.4500", "1E+3", "1E3", "1e3",
  "0.000000000000000000000000000000000000001", "123456789012345678901234567890123456789", "-0.0000000000000000000000000000000000005",
];

const HALF_TAILS: &[&str] = &["5", "50", "500000", "49999", "4999999999", "50001", "5000000001", "4", "6", "9", "1", "0", "95", "05", "499", "501"];

/// General-purpose random number literal.
fn gen_num(r: &mut Rng) -> String {
  let s = sign(r);
  match r.below(100) {
    0..=9 => format!("{s}{}", r.below(1001)),
    10..=17 => {
      let fd = 1 + r.below(6) as usize;
      format!("{s}{}.{}", r.below(1000), (0..fd).map(|_| (b'0' + r.below(10) as u8) as char).collect::<String>())
    }
    18..=29 => {
      // long coefficients 30..34 digits
      let n = 30 + r.below(5) as usize;
      let c = digits(r, n);
      let e = r.range(-45, 10);
      place(r, s, &c, e)
    }
    30..=39 => {
      // 35..40 digit literals that need rounding, at half points
      let base = if r.coin(5) { "9".repeat(34) } else { digits(r, 34) };
      let tail = r.pick(HALF_TAILS);
      let c = format!("{base}{tail}");
      let e = r.range(-45, 5);
      place(r, s, &c, e)
    }
    40..=47 => {
      let n = 1 + r.below(34) as usize;
      let c = if r.coin(4) { "9".repeat(n) } else { digits(r, n) };
      sci(s, &c, r.range(6090, 6150))
    }
    48..=55 => {
      let n = 1 + r.below(34) as usize;
      let c = digits(r, n);
      sci(s, &c, r.range(-6215, -6080))
    }
    56..=61 => format!("{}", r.pick(&["0", "-0", "0E+10", "-0E-20", "0.000", "-0.0", "0E-6176", "-0E-6176", "0E+6111", "0E+9999", "-0E-9999", "00", "-00.00"])),
    62..=71 => r.pick(STRUCTURED).to_string(),
    72..=81 => {
      let n = 1 + r.below(34) as usize;
      let c = digits(r, n);
      sci(s, &c, r.range(-6176, 6144))
    }
    82..=89 => {
      // values with a 5 at some fractional position (ties for rounding-to-scale)
      let ip = r.below(100000);
      let fd = r.below(8) as usize;
      let mid: String = (0..fd).map(|_| (b'0' + r.below(10) as u8) as char).collect();
      let last = r.pick(&["5", "5", "50", "49", "51", "4999", "5001"]);
      format!("{s}{ip}.{mid}{last}")
    }
    90..=95 => {
      let v = r.pick(&[
        "127", "128", "-128", "-129", "255", "256", "32767", "32768", "-32768", "-32769", "65535", "65536", "2147483647", "2147483648",
        "-2147483648", "-2147483649", "4294967295", "4294967296", "9223372036854775807", "9223372036854775808", "-9223372036854775808",
        "-9223372036854775809", "18446744073709551615", "18446744073709551616", "1", "-1", "0",
      ]);
      let suf = r.pick(&["", "", ".0", ".5", ".9999", ".00001", "E+0", "0", "E-1", "E+1"]);
      format!("{v}{suf}")
    }
    _ => r
      .pick(&[
        ".5", "5.", ".", "-.5", "+3", "+0", " 12", "\t7", "1e5", "1E+0000003", "1E-00000003", "1E99999999", "1E-99999999", "abc", "", "1.2.3", "--1", "Infinity",
        "inf", "NaN", "-NaN", "SNaN", "1E", "1E+", "E5", "12abc", "1 2", "0x10", "1_000", "١٢", "1E+6145", "9.9999999999999999999999999999999995E+6144",
        "1E-6177", "4.9E-6177", "5.1E-6177", "1E+12345678", "-",
      ])
      .to_string(),
  }
}

fn toggle(s: &str) -> String {
  if let Some(t) = s.strip_prefix('-') { t.to_string() } else { format!("-{s}") }
}

/// Pairs for binary arithmetic, with structured relations (cancellation, ties, overflow).
fn gen_pair(r: &mut Rng) -> (String, String) {
  if r.below(10) < 6 {
    return (gen_num(r), gen_num(r));
  }
  match r.below(9) {
    0 => {
      let a = gen_num(r);
      (a.clone(), a)
    }
    1 => {
      let a = gen_num(r);
      let b = toggle(&a);
      (a, b)
    }
    2 => {
      // 34-digit integer +/- tie-ish fraction
      let a = format!("{}{}", sign(r), digits(r, 34));
      let b = format!("{}{}", sign(r), r.pick(&["0.5", "0.49999999", "0.50000001", "1.5", "2.5", "0.05", "5E-1", "5E-35", "0.4", "0.6", "5", "50", "0.5000000000000000000000000000000000001"]));
      if r.coin(2) { (a, b) } else { (b, a) }
    }
    3 => {
      // odd 34-digit coefficient times/divided by tie makers
      let mut c = digits(r, 34);
      c.pop();
      c.push(r.pick(&["1", "3", "5", "7", "9"]).chars().next().unwrap());
      let e = r.range(-40, 10);
      let a = format!("{}{c}E{e}", sign(r));
      let b = format!("{}{}", sign(r), r.pick(&["2", "20", "0.2", "8", "16", "4", "5", "0.5", "1.5", "3", "0.25", "1.25", "1.1", "11", "1E+10", "1E-10"]));
      (a, b)
    }
    4 => {
      // repeating quotients
      let a = format!("{}{}", sign(r), r.pick(&["1", "2", "10", "1E+30", "1E-30", "22", "355", "0.1", "9999999999999999999999999999999999"]));
      let b = format!("{}{}", r.pick(&["", "-"]), r.pick(&["3", "7", "9", "11", "13", "17", "19", "23", "29", "31", "97", "113", "6", "12", "0.3", "0.7", "81", "999"]));
      (a, b)
    }
    5 => {
      // near cancellation: a and a with tweaked last digits
      let c = digits(r, 34);
      let e = r.range(-40, 40);
      let mut d = c.clone().into_bytes();
      let i = 34 - 1 - r.below(5) as usize;
      d[i] = if d[i] == b'9' { b'8' } else { d[i] + 1 };
      let d = String::from_utf8(d).unwrap();
      (format!("{c}E{e}"), format!("{}E{}", d, e + r.range(-2, 2)))
    }
    6 => {
      // overflow / underflow pairs
      let a = sci(sign(r), &rdig(r, 34), r.range(3000, 6144));
      let b = sci(sign(r), &rdig(r, 34), r.range(-6176, 6144));
      (a, b)
    }
    7 => {
      // tiny operands (subnormal results)
      let a = sci(sign(r), &rdig(r, 34), r.range(-6176, -3000));
      let b = sci(sign(r), &rdig(r, 34), r.range(-20, 20));
      (a, b)
    }
    _ => {
      // differing exponents far apart: sticky-bit rounding cases
      let a = format!("{}{}", sign(r), digits(r, 34));
      let b = sci(sign(r), &format!("{}{}", r.pick(&["5", "4", "6", "49", "51", "5000000000001", "4999999999999"]), ""), -r.range(1, 80));
      (a, b)
    }
  }
}

fn gen_scale(r: &mut Rng) -> i64 {
  match r.below(20) {
    0..=10 => r.range(-6, 40),
    11..=14 => r.range(-120, 120),
    15..=17 => r.range(-6400, 6400),
    _ => [-6176i64, -6177, -6111, -6112, 6111, 6143, 6144, 6145, 6176, 6177, 2147483647, -2147483648, 1000000, -1000000][r.below(14) as usize],
  }
}

fn gen_rounding_input(r: &mut Rng) -> String {
  if r.coin(2) {
    gen_num(r)
  } else {
    // ties at the scale position
    let s = sign(r);
    let ip = r.pick(&["0", "1", "2", "3", "9", "99", "12345", "999999999999999999999999999999999"]);
    let fd = r.below(6) as usize;
    let mid: String = (0..fd).map(|_| (b'0' + r.below(10) as u8) as char).collect();
    let last = r.pick(&["5", "50", "49", "51", "4999999999999", "5000000000001", "1", "9", "0"]);
    format!("{s}{ip}.{mid}{last}")
  }
}

fn gen_exp_input(r: &mut Rng) -> String {
  let s = sign(r);
  match r.below(10) {
    0..=3 => gen_num(r),
    4..=5 => format!("{s}{}.{}", r.below(400), rdig(r, 33)),
    6 => format!("{s}{}.{}", r.range(14100, 14250), rdig(r, 28)), // overflow/underflow edge
    7 => sci(s, &rdig(r, 34), r.range(-60, -1)),                   // tiny
    8 => format!("{s}{}", r.range(-20, 1000)),
    _ => format!("{s}{}", r.pick(&["1", "0.5", "2.302585092994045684017991454684364", "0.6931471805599453094172321214581766", "14142", "-14144", "14145", "-14220", "-14230", "1E-34", "1E-35"])),
  }
}

fn gen_ln_input(r: &mut Rng) -> String {
  match r.below(10) {
    0..=3 => gen_num(r),
    4..=5 => format!("{}.{}", r.below(1000), rdig(r, 33)),
    6 => {
      // near 1
      let k = r.range(1, 34) as usize;
      let tail = rdig(r, 10);
      if r.coin(2) { format!("1.{}{}", "0".repeat(k - 1), tail) } else { format!("0.{}{}", "9".repeat(k), tail) }
    }
    7 => sci("", &rdig(r, 34), r.range(-6176, 6144)),
    8 => format!("{}", r.pick(&["1", "10", "100", "1E+100", "1E-100", "2", "0.5", "2.718281828459045235360287471352662", "1E+6144", "1E-6176", "9.999999999999999999999999999999999E+6144"])),
    _ => format!("1E{}", r.range(-6176, 6144)),
  }
}

fn gen_sqrt_input(r: &mut Rng) -> String {
  match r.below(10) {
    0..=5 => gen_num(r),
    6..=7 => {
      // perfect squares with various cohorts
      let k = r.range(0, 99999999999999999);
      let k2 = (k as u128) * (k as u128);
      format!("{k2}E{}", 2 * r.range(-20, 20) + if r.coin(4) { 1 } else { 0 })
    }
    _ => sci("", &rdig(r, 34), r.range(-6176, 6144)),
  }
}

fn gen_pow_base(r: &mut Rng) -> String {
  let s = sign(r);
  match r.below(10) {
    0..=3 => gen_num(r),
    4..=5 => format!("{s}{}.{}", r.below(100), rdig(r, 10)),
    6 => format!("{s}{}", r.below(50)),
    7 => r.pick(&["0", "-0", "1", "-1", "2", "-2", "10", "0.1", "1.0000001", "0.9999999", "1.000000000000000000000000000000001", "0.5", "1E+6144", "1E-6176", "12.2384283"]).to_string(),
    _ => sci(s, &rdig(r, 34), r.range(-400, 400)),
  }
}

fn gen_pow_int(r: &mut Rng) -> String {
  match r.below(10) {
    0..=5 => format!("{}", r.range(-20, 40)),
    6..=7 => format!("{}", r.range(-5000, 5000)),
    8 => r.pick(&["2147483647", "2147483648", "-2147483648", "-2147483649", "4294967296", "1E+10", "1E+40", "-1E+40", "2.0", "3.000", "1E+1", "100000", "-100000"]).to_string(),
    _ => format!("{}", r.range(-200, 200)),
  }
}

fn gen_pow_frac(r: &mut Rng) -> String {
  match r.below(6) {
    0 => format!("{}{}.5", sign(r), r.below(20)),
    1 => format!("{}{}.{}", sign(r), r.below(20), r.pick(&["25", "75", "125", "375", "2", "1", "333333", "0000000000000000000000000000000001"])),
    2 => format!("{}0.{}", sign(r), rdig(r, 34)),
    3 => format!("{}{}.{}", sign(r), r.below(1000), rdig(r, 30)),
    4 => sci(sign(r), &rdig(r, 34), r.range(-40, -1)),
    _ => r.pick(&["0.5", "-0.5", "1.5", "4.25", "0.1", "1E-40", "0.3333333333333333333333333333333333", "2.5", "-2.5", "100.5", "1000.1"]).to_string(),
  }
}

// ---------------------------------------------------------------------------------------------
// Categorisation
// ---------------------------------------------------------------------------------------------

/// Parses "+123E-5" -> (neg, digits without trailing zeros, exponent, adjusted exponent)
fn dec(s: &str) -> Option<(bool, String, i64, i64)> {
  let s = s.trim();
  let (neg, rest) = match s.as_bytes().first()? {
    b'+' => (false, &s[1..]),
    b'-' => (true, &s[1..]),
    _ => return None,
  };
  let (c, e) = rest.split_once('E')?;
  if !c.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let mut e: i64 = e.parse().ok()?;
  let mut c = c.trim_start_matches('0').to_string();
  if c.is_empty() {
    return Some((neg, "0".into(), 0, i64::MIN / 4));
  }
  while c.ends_with('0') {
    c.pop();
    e += 1;
  }
  let adj = e + c.len() as i64 - 1;
  Some((neg, c, e, adj))
}

/// Extract all "+cE±e" tokens from a debug string.
fn tokens(s: &str) -> Vec<(bool, String, i64, i64)> {
  s.split(|c: char| c == ' ' || c == '|' || c == '(' || c == ')' || c == ',')
    .filter_map(|t| dec(t))
    .collect()
}

fn is_special(s: &str) -> bool {
  s.contains("PANIC") || s.contains("Inf") || s.contains("NaN") || s.contains("None") || s.contains("Err") || s.contains("null")
}

const TRANSCENDENTAL: &[&str] = &["exp", "ln", "feel_log", "pow", "pow_frac", "pow_special", "chain_trans"];

fn categorize(op: &str, inputs: &str, i: &Out, f: &Out) -> &'static str {
  if i.dbg == f.dbg {
    return "formatting (same repr, different Display)";
  }
  let (ti, tf) = (tokens(&i.dbg), tokens(&f.dbg));
  if is_special(&i.dbg) != is_special(&f.dbg) || (ti.is_empty() || tf.is_empty()) {
    if i.dbg.contains("PANIC") || f.dbg.contains("PANIC") {
      return "special values (panic)";
    }
    return "special values (null/Inf/NaN/Err)";
  }
  if ti.len() == tf.len() && ti.iter().zip(&tf).all(|(a, b)| a.0 == b.0 && a.1 == b.1 && a.2 == b.2) {
    return "formatting (same value, different cohort/trailing zeros)";
  }
  if ti.iter().zip(&tf).any(|(a, b)| a.1 == "0" && b.1 == "0" && a.0 != b.0) && ti.iter().zip(&tf).all(|(a, b)| a.1 == b.1 && (a.1 == "0" || a.2 == b.2)) {
    return "special values (signed zero)";
  }
  let big = |t: &[(bool, String, i64, i64)]| t.iter().any(|x| x.3 != i64::MIN / 4 && (x.3 > 6100 || x.3 < -6100));
  let inp_big = tokens(&inputs.replace(['E'], "E")).is_empty() && {
    // inputs are literals: estimate magnitude via exponent marker
    inputs.split(|c: char| c == ' ' || c == ',').any(|t| {
      t.split_once(['E', 'e']).and_then(|(_, e)| e.parse::<i64>().ok()).map(|e| e.abs() > 6000).unwrap_or(false)
    })
  };
  if big(&ti) || big(&tf) || inp_big {
    return "exponent range (|adj exp| > 6100)";
  }
  if TRANSCENDENTAL.contains(&op) {
    return "transcendental accuracy";
  }
  "rounding (double-rounding candidate)"
}

// ---------------------------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct Stat {
  cases: u64,
  skipped: u64,
  display_mismatch: u64,
  debug_only: u64,
  cats: BTreeMap<&'static str, u64>,
  examples: BTreeMap<&'static str, Vec<String>>,
}

const OPS: &[&str] = &[
  "parse", "add", "sub", "mul", "div", "rem", "assign_ops", "neg", "abs", "trunc", "frac", "sqrt", "square", "exp", "ln", "feel_log", "pow", "pow_int",
  "pow_frac", "pow_special", "feel_modulo", "round_quantize", "feel_decimal", "ceiling", "floor", "round_down", "round_up", "round_half_up", "round_half_down",
  "cmp", "cmp_isize", "predicates", "format", "to_int_ref", "to_int_owned", "new", "from_ints", "constants", "chain", "chain_trans",
];

fn gen_case(op: &str, r: &mut Rng) -> (String, String, i64) {
  match op {
    "parse" | "neg" | "abs" | "trunc" | "frac" | "predicates" | "square" | "to_int_ref" | "to_int_owned" => (gen_num(r), String::new(), 0),
    "add" | "sub" | "mul" | "div" | "rem" | "assign_ops" | "cmp" | "feel_modulo" => {
      let (a, b) = gen_pair(r);
      (a, b, 0)
    }
    "sqrt" => (gen_sqrt_input(r), String::new(), 0),
    "exp" => (gen_exp_input(r), String::new(), 0),
    "ln" | "feel_log" => (gen_ln_input(r), String::new(), 0),
    "pow" => (gen_num(r), gen_num(r), 0),
    "pow_int" => (gen_pow_base(r), gen_pow_int(r), 0),
    "pow_frac" => (gen_pow_base(r), gen_pow_frac(r), 0),
    "pow_special" => {
      let a = r.pick(&["0", "-0", "1", "-1", "0.0", "-0.00", "1E-6176", "-1E-6176", "1E+6144", "-1E+6144", "2", "-2", "0.5", "-0.5", "10", "1.0", "-1.000"]).to_string();
      let b = if r.coin(2) {
        r.pick(&["0", "-0", "1", "-1", "2", "-2", "0.5", "-0.5", "3", "-3", "1E+6144", "-1E+6144", "1E-6176", "2147483647", "-2147483648", "2147483648", "1E+40", "1.5", "7", "-7"]).to_string()
      } else {
        gen_num(r)
      };
      (a, b, 0)
    }
    "round_quantize" => {
      let b = if r.coin(2) { format!("{}", gen_scale(r)) } else { gen_num(r) };
      (gen_rounding_input(r), b, 0)
    }
    "feel_decimal" => {
      let b = match r.below(4) {
        0 => gen_num(r),
        1 => format!("{}.{}", r.range(-6200, 6200), r.below(10)),
        _ => format!("{}", gen_scale(r)),
      };
      (gen_rounding_input(r), b, 0)
    }
    "ceiling" | "floor" | "round_down" | "round_up" | "round_half_up" | "round_half_down" => (gen_rounding_input(r), String::new(), gen_scale(r)),
    "cmp_isize" => (gen_num(r), String::new(), if r.coin(2) { r.range(-1000, 1000) } else { r.next() as i64 >> r.below(64) }),
    "format" => (gen_num(r), String::new(), r.range(0, 40)),
    "new" => {
      let n = match r.below(4) {
        0 => r.range(-1000, 1000),
        1 => [0i64, 1, -1, i64::MAX, i64::MIN, i64::MAX - 1, i64::MIN + 1, 10, 100][r.below(9) as usize],
        _ => r.next() as i64 >> r.below(64),
      };
      (String::new(), format!("{}", gen_scale(r) as i32), n)
    }
    "from_ints" => {
      let n = match r.below(3) {
        0 => r.range(-70000, 70000),
        1 => [0i64, 1, -1, i64::MAX, i64::MIN, 127, 128, 255, 256, -128, -129, 32767, 32768, 65535, 65536, 2147483647, 2147483648, 4294967295, 4294967296][r.below(19) as usize],
        _ => r.next() as i64 >> r.below(64),
      };
      (String::new(), String::new(), n)
    }
    "constants" => (String::new(), String::new(), 0),
    "chain" | "chain_trans" => (gen_num(r), String::new(), (r.next() >> 1) as i64),
    _ => unreachable!(),
  }
}

fn run_op(op: &'static str, iters: u64, seed: u64, mm: &Mutex<std::fs::File>, co: &Mutex<std::fs::File>) -> Stat {
  let mut st = Stat::default();
  let mut r = Rng(seed);
  let iters = if op == "constants" { 1 } else { iters };
  let mut mm_buf = String::new();
  let mut co_buf = String::new();
  let mut attempts = 0u64;
  while st.cases < iters && attempts < iters * 3 {
    attempts += 1;
    let (a, b, k) = gen_case(op, &mut r);
    let inputs = match op {
      "new" => format!("new({k}, {b})"),
      "from_ints" => format!("from({k})"),
      "ceiling" | "floor" | "round_down" | "round_up" | "round_half_up" | "round_half_down" => format!("{a:?} @ {k}"),
      "cmp_isize" | "format" | "chain" | "chain_trans" => format!("{a:?} k={k}"),
      _ if b.is_empty() => format!("{a:?}"),
      _ => format!("{a:?} , {b:?}"),
    };
    if std::env::var_os("DIFFTEST_TRACE").is_some() {
      eprintln!("{op}\t{}\t{}\t{k}", esc(&a), esc(&b));
    }
    let (i, f) = (eval::<intel::FeelNumber>(op, &a, &b, k), eval::<fast::FeelNumber>(op, &a, &b, k));
    let (i, f) = match (i, f) {
      (Some(i), Some(f)) => (i, f),
      (None, None) => {
        st.skipped += 1;
        continue;
      }
      (i, f) => (i.unwrap_or_else(|| txt("unparseable".into())), f.unwrap_or_else(|| txt("unparseable".into()))),
    };
    st.cases += 1;
    if i.disp != f.disp {
      st.display_mismatch += 1;
      let c = categorize(op, &inputs, &i, &f);
      *st.cats.entry(c).or_default() += 1;
      let ex = st.examples.entry(c).or_default();
      if ex.len() < 8 {
        ex.push(format!("{op}({inputs}): intel={} | dashu={}   [dbg intel={} dashu={}]", i.disp, f.disp, i.dbg, f.dbg));
      }
      mm_buf.push_str(&format!("{op}\t{c}\t{}\t{}\t{k}\t{}\t{}\t{}\t{}\n", esc(&a), esc(&b), i.disp, f.disp, i.dbg, f.dbg));
    } else if std::env::var_os("DIFFTEST_ALL").is_some() {
      mm_buf.push_str(&format!("{op}\tmatch\t{}\t{}\t{k}\t{}\t{}\t{}\t{}\n", esc(&a), esc(&b), i.disp, f.disp, i.dbg, f.dbg));
    } else if i.dbg != f.dbg {
      st.debug_only += 1;
      co_buf.push_str(&format!("{op}\t{a}\t{b}\t{k}\t{}\t{}\t{}\n", i.disp, i.dbg, f.dbg));
    }
    if mm_buf.len() > 1 << 20 {
      mm.lock().unwrap().write_all(mm_buf.as_bytes()).unwrap();
      mm_buf.clear();
    }
    if co_buf.len() > 1 << 20 {
      co.lock().unwrap().write_all(co_buf.as_bytes()).unwrap();
      co_buf.clear();
    }
  }
  mm.lock().unwrap().write_all(mm_buf.as_bytes()).unwrap();
  co.lock().unwrap().write_all(co_buf.as_bytes()).unwrap();
  st
}

fn main() {
  std::panic::set_hook(Box::new(|_| {}));
  let args: Vec<String> = std::env::args().collect();
  if args.get(1).map(|s| s.as_str()) == Some("eval") {
    let (op, a, b) = (args[2].as_str(), args.get(3).cloned().unwrap_or_default(), args.get(4).cloned().unwrap_or_default());
    let k: i64 = args.get(5).map(|s| s.parse().unwrap()).unwrap_or(0);
    let i = eval::<intel::FeelNumber>(op, &a, &b, k).map(|o| format!("{} [{}]", o.disp, o.dbg));
    let f = eval::<fast::FeelNumber>(op, &a, &b, k).map(|o| format!("{} [{}]", o.disp, o.dbg));
    println!("{op}({a:?}, {b:?}, {k})\n  intel  : {i:?}\n  dashu  : {f:?}");
    return;
  }
  let iters: u64 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(200_000);
  let only: Option<&str> = args.get(2).map(|s| s.as_str());
  let ops: Vec<&'static str> = OPS.iter().copied().filter(|o| only.is_none_or(|w| w.split(',').any(|x| x == *o))).collect();
  let mm = Mutex::new(std::fs::File::create("mismatches.tsv").unwrap());
  let co = Mutex::new(std::fs::File::create("cohort.tsv").unwrap());
  let t0 = std::time::Instant::now();
  let results: Vec<(&str, Stat, f64)> = std::thread::scope(|sc| {
    let hs: Vec<_> = ops
      .iter()
      .enumerate()
      .map(|(n, op)| {
        let (mm, co) = (&mm, &co);
        sc.spawn(move || {
          let t = std::time::Instant::now();
          let base: u64 = std::env::var("DIFFTEST_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0xD5A7_0000);
          let s = run_op(op, iters, base + n as u64 * 7919, mm, co);
          (*op, s, t.elapsed().as_secs_f64())
        })
      })
      .collect();
    hs.into_iter().map(|h| h.join().unwrap()).collect()
  });
  println!("{:<16} {:>9} {:>8} {:>10} {:>10}  {:>7}  categories", "op", "cases", "skipped", "disp_mism", "cohort_only", "secs");
  let (mut tc, mut tm) = (0, 0);
  for (op, s, secs) in &results {
    tc += s.cases;
    tm += s.display_mismatch;
    let cats = s.cats.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("; ");
    println!("{:<16} {:>9} {:>8} {:>10} {:>10}  {:>7.1}  {}", op, s.cases, s.skipped, s.display_mismatch, s.debug_only, secs, cats);
  }
  println!("TOTAL cases={tc} display_mismatches={tm} wall={:.1}s", t0.elapsed().as_secs_f64());
  for (op, s, _) in &results {
    for (c, ex) in &s.examples {
      println!("--- {op} / {c}");
      for e in ex {
        println!("    {e}");
      }
    }
  }
}

fn rdig(r: &mut Rng, max: u64) -> String {
  let n = 1 + r.below(max) as usize;
  digits(r, n)
}

fn esc(s: &str) -> String {
  s.replace('\t', "\\t")
}
