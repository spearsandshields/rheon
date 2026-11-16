//! # The choice of the evaluator, for tests
//!
//! Present only with the cargo feature `backends`, which test builds enable: without it, the
//! expressions are built by the flat evaluator where it lowers them and as closures otherwise.
//! A test chooses the evaluator of the expressions built inside [with_backend] on its thread,
//! and reads the expressions that the flat evaluator refused there with [take_refusals] and the
//! number it lowered with [take_lowered]. Outside
//! [with_backend] nothing is recorded, and nothing is kept beyond the closure and the refusals not
//! yet taken.

use crate::flat::NotYet;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, value_null};
use std::cell::{Cell, RefCell};

/// The evaluator that builds the expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
  /// The evaluator of closures, for every expression.
  Old,
  /// The flat evaluator where it lowers an expression, the evaluator of closures otherwise; the
  /// choice without this feature.
  Flat,
  /// The flat evaluator only: an expression it refuses evaluates to the null of [refused_marker].
  FlatOnly,
  /// The evaluator of closures with every boolean result negated: an evaluator known to differ,
  /// for the tests of the comparison of two evaluators.
  Mutant,
}

/// An expression that the flat evaluator refused: the kind of the refusal and the kind of the
/// node that stopped it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
  /// The kind of the refusal (`NotYet`).
  pub kind: &'static str,
  /// The kind of the node, as the parser's tree names it (`Add`, `For`, …).
  pub node: String,
}

thread_local! {
  static BACKEND: Cell<Option<Backend>> = const { Cell::new(None) };
  static REFUSALS: RefCell<Vec<Refusal>> = const { RefCell::new(Vec::new()) };
  static LOWERED: Cell<usize> = const { Cell::new(0) };
}

/// Runs `f` with `backend` building the expressions on this thread, and restores the previous
/// choice when `f` returns or panics.
pub fn with_backend<T>(backend: Backend, f: impl FnOnce() -> T) -> T {
  struct Restore(Option<Backend>);
  impl Drop for Restore {
    fn drop(&mut self) {
      BACKEND.with(|cell| cell.set(self.0));
    }
  }
  let _restore = Restore(BACKEND.with(|cell| cell.replace(Some(backend))));
  f()
}

/// The refusals of the flat evaluator inside [with_backend] on this thread since the last call,
/// in order.
pub fn take_refusals() -> Vec<Refusal> {
  REFUSALS.with(|refusals| refusals.take())
}

/// The number of expressions that the flat evaluator lowered inside [with_backend] on this thread
/// since the last call.
pub fn take_lowered() -> usize {
  LOWERED.with(|lowered| lowered.replace(0))
}

/// Notes an expression that the flat evaluator lowered inside [with_backend].
pub(crate) fn lowered() {
  if BACKEND.with(Cell::get).is_some() {
    LOWERED.with(|lowered| lowered.set(lowered.get() + 1));
  }
}

/// The evaluator chosen on this thread: [Backend::Flat] outside [with_backend].
pub(crate) fn backend() -> Backend {
  BACKEND.with(Cell::get).unwrap_or(Backend::Flat)
}

/// Notes a refusal of the flat evaluator inside [with_backend]; under [Backend::FlatOnly] it gives
/// the evaluator that stands for the refused expression.
pub(crate) fn refused(refusal: &NotYet) -> Option<Evaluator> {
  BACKEND.with(Cell::get)?;
  let node = format!("{:?}", refusal.at);
  let node = node.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or_default().to_string();
  let refusal = Refusal { kind: "NotYet", node };
  REFUSALS.with(|refusals| refusals.borrow_mut().push(refusal.clone()));
  (backend() == Backend::FlatOnly).then(|| refused_marker(refusal))
}

/// The evaluator of an expression that the flat evaluator refused: an error null that names the
/// refusal.
fn refused_marker(refusal: Refusal) -> Evaluator {
  Box::new(move |_: &FeelScope| value_null!("flat: refused ({} at {})", refusal.kind, refusal.node))
}

/// The evaluator of [Backend::Mutant]: `evaluator` with a boolean result negated.
pub(crate) fn mutant(evaluator: Evaluator) -> Evaluator {
  Box::new(move |scope: &FeelScope| match evaluator(scope) {
    Value::Boolean(value) => Value::Boolean(!value),
    other => other,
  })
}
