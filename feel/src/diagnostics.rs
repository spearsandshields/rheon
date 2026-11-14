//! # Diagnostics of an evaluation (DMN 1.6 §7.3.8)
//!
//! FEEL reports a semantic error by returning null with a reason (`value_null!`). DMN 1.6 requires
//! the evaluation to "report or log diagnostic information" and defines two error handling modes:
//! lenient (the default: errors are collected and evaluation continues) and strict (evaluation
//! halts at the first error and returns null).
//!
//! While a caller collects diagnostics ([collect]), every null-with-reason created through
//! `value_null!` is recorded here (per thread). Outside a collection nothing is recorded, so
//! ordinary evaluations pay one thread-local flag check per error null.
//!
//! An evaluation can also be stopped by an implementation limit ([Abort]), such as the depth of
//! nested function calls. One evaluation ([evaluation]) owns the abort and the call depth, every
//! function call reads them ([enter_call]), and nothing of them outlives the evaluation.

use crate::values::Value;
use crate::{Evaluator, FeelScope};
use dsntk_common::{DsntkError, ToErrorMessage};
use std::cell::{Cell, RefCell};

/// At most this many messages are kept; [Diagnostics::count] still counts all errors.
pub const MAX_MESSAGES: usize = 64;

thread_local! {
  static ACTIVE: Cell<bool> = const { Cell::new(false) };
  static COUNT: Cell<usize> = const { Cell::new(0) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Errors recorded during one evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics {
  /// The number of errors (null values with a reason) created.
  pub count: usize,
  /// The first [MAX_MESSAGES] error messages, in the order they occurred; consecutive repeats of
  /// the same message (an error passed up through several operators) are kept once.
  pub messages: Vec<String>,
}

impl Diagnostics {
  /// The first error, if any.
  pub fn first(&self) -> Option<&str> {
    self.messages.first().map(String::as_str)
  }
}

/// Records an error message when a collection is active (called by `value_null!`).
#[inline]
pub fn record(message: &str) {
  if !ACTIVE.with(Cell::get) {
    return;
  }
  COUNT.with(|count| count.set(count.get() + 1));
  MESSAGES.with(|messages| {
    let mut messages = messages.borrow_mut();
    if messages.len() < MAX_MESSAGES && messages.last().map(String::as_str) != Some(message) {
      messages.push(message.to_string());
    }
  });
}

/// Runs `f` while collecting diagnostics and returns its result with the errors recorded.
/// Collections nest: an inner collection's errors are also counted in the outer one.
pub fn collect<T>(f: impl FnOnce() -> T) -> (T, Diagnostics) {
  let outer = Collection::start();
  let result = f();
  let diagnostics = outer.finish();
  (result, diagnostics)
}

/// The collection state of the enclosing scope, restored when the inner collection ends, also
/// when the evaluation panics (a thread that keeps collecting would record forever).
struct Collection {
  was_active: bool,
  saved_count: usize,
  saved_messages: Vec<String>,
}

impl Collection {
  fn start() -> Self {
    Self {
      was_active: ACTIVE.with(|active| active.replace(true)),
      saved_count: COUNT.with(|count| count.replace(0)),
      saved_messages: MESSAGES.with(|messages| messages.replace(Vec::new())),
    }
  }

  fn finish(self) -> Diagnostics {
    // the state stays in place for `drop`, which restores the enclosing collection and adds this one's errors to it
    Diagnostics {
      count: COUNT.with(Cell::get),
      messages: MESSAGES.with(|messages| messages.borrow().clone()),
    }
  }
}

impl Drop for Collection {
  fn drop(&mut self) {
    let count = COUNT.with(Cell::get);
    let messages = MESSAGES.with(|messages| messages.take());
    ACTIVE.with(|active| active.set(self.was_active));
    COUNT.with(|outer| outer.set(self.saved_count + if self.was_active { count } else { 0 }));
    MESSAGES.with(|outer| {
      let mut restored = std::mem::take(&mut self.saved_messages);
      if self.was_active {
        restored.extend(messages.into_iter().take(MAX_MESSAGES.saturating_sub(restored.len())));
      }
      *outer.borrow_mut() = restored;
    });
  }
}

/// Why an evaluation stopped before it finished: an implementation limit, never a FEEL value.
///
/// FEEL defines no limits, so a value computed after a limit was hit is not the result of the
/// expression. The evaluation reports the abort instead of any value (§7.3.8: the evaluation
/// "MUST report or log diagnostic information").
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Abort {
  /// More than `limit` nested function invocations.
  #[non_exhaustive]
  CallDepth { limit: usize },
  /// An engine below FEEL gave up; `reason` names the function, the input and the limit.
  #[non_exhaustive]
  Limit { reason: Box<str> },
}

impl Abort {
  /// An engine limit, described by `reason`.
  pub fn limit(reason: impl Into<Box<str>>) -> Self {
    Self::Limit { reason: reason.into() }
  }
}

impl std::fmt::Display for Abort {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::CallDepth { limit } => write!(
        f,
        "function calls are nested too deeply (more than {limit} levels, e.g. runaway recursion); evaluation stopped"
      ),
      Self::Limit { reason } => write!(f, "{reason}; evaluation stopped"),
    }
  }
}

/// An aborted evaluation as an error.
#[derive(ToErrorMessage)]
struct EvaluationAborted(String);

impl From<Abort> for DsntkError {
  fn from(abort: Abort) -> Self {
    EvaluationAborted(abort.to_string()).into()
  }
}

/// The state of the evaluation running on this thread.
///
/// One evaluation owns it: it is clear outside an evaluation, and the evaluation that opened it
/// clears it when it ends, also when it unwinds, so nothing of it outlives the evaluation.
struct Evaluation {
  /// Whether [evaluation] opened the running evaluation.
  explicit: Cell<bool>,
  /// The number of function calls entered and not yet left. Outside an explicit evaluation, the
  /// outermost call opens an implicit one.
  depth: Cell<usize>,
  /// Why the running evaluation was stopped, the first reason only.
  abort: RefCell<Option<Abort>>,
  /// Whether `abort` is set, read by every function call.
  aborted: Cell<bool>,
}

impl Evaluation {
  fn active(&self) -> bool {
    self.explicit.get() || self.depth.get() > 0
  }

  fn clear(&self) {
    self.explicit.set(false);
    self.depth.set(0);
    self.aborted.set(false);
    self.abort.borrow_mut().take();
  }

  fn stop(&self, reason: Abort) {
    if !self.aborted.replace(true) {
      *self.abort.borrow_mut() = Some(reason);
    }
  }

  fn abort(&self) -> Option<Abort> {
    self.abort.borrow().clone()
  }
}

thread_local! {
  static EVALUATION: Evaluation = const {
    Evaluation {
      explicit: Cell::new(false),
      depth: Cell::new(0),
      abort: RefCell::new(None),
      aborted: Cell::new(false),
    }
  };
}

/// Runs `f` as one evaluation and returns its result with the abort that stopped it, if any.
///
/// Outside an evaluation, `f` starts with a clear state, and the state is cleared again when `f`
/// returns or unwinds. Inside an evaluation, `f` joins it: an abort in `f` stops the enclosing
/// evaluation too, and an abort before `f` is returned for `f` as well.
pub fn evaluation<T>(f: impl FnOnce() -> T) -> (T, Option<Abort>) {
  let opened = Opened::open();
  let value = f();
  let abort = EVALUATION.with(Evaluation::abort);
  drop(opened);
  (value, abort)
}

/// The evaluator that runs each evaluation of `evaluator` as one [evaluation]: inside a running
/// evaluation it joins it, outside one it is an evaluation of its own. When the evaluation is
/// stopped, its result is null with the reason of the abort, never a value computed after it.
pub fn one_evaluation(evaluator: Evaluator) -> Evaluator {
  Box::new(move |scope: &FeelScope| match evaluation(|| evaluator(scope)) {
    (value, None) => value,
    (_, Some(abort)) => crate::value_null!(abort),
  })
}

/// Clears the evaluation state when the evaluation that [evaluation] opened ends.
struct Opened(bool);

impl Opened {
  fn open() -> Self {
    EVALUATION.with(|evaluation| {
      if evaluation.active() {
        Self(false)
      } else {
        evaluation.explicit.set(true);
        Self(true)
      }
    })
  }
}

impl Drop for Opened {
  fn drop(&mut self) {
    if self.0 {
      EVALUATION.with(Evaluation::clear);
    }
  }
}

/// Stops the running evaluation for `reason`; the first reason is kept. Outside an evaluation it
/// has no effect.
pub fn abort(reason: Abort) {
  EVALUATION.with(|evaluation| {
    if evaluation.active() {
      evaluation.stop(reason);
    }
  });
}

/// Enters a function call when at most `limit` calls are nested in it. `None` refuses the call:
/// the evaluation was stopped before, or this call exceeds the limit, which stops it with
/// [Abort::CallDepth]. A refused call's result is never observed, because the abort, not a value,
/// is the outcome of the evaluation.
pub fn enter_call(limit: usize) -> Option<Call> {
  EVALUATION.with(|evaluation| {
    if evaluation.aborted.get() {
      return None;
    }
    let depth = evaluation.depth.get();
    if depth >= limit {
      evaluation.stop(Abort::CallDepth { limit });
      return None;
    }
    let outermost = depth == 0 && !evaluation.explicit.get();
    evaluation.depth.set(depth + 1);
    Some(Call { outermost })
  })
}

/// A function call entered by [enter_call]; dropping it leaves the call, also when unwinding.
#[must_use]
pub struct Call {
  /// Whether this call opened an implicit evaluation (no evaluation was running).
  outermost: bool,
}

impl Call {
  /// The result of the call. The outermost call of an implicit evaluation never returns a value
  /// of a stopped evaluation: it returns null with the abort's text instead.
  pub fn finish(self, value: Value) -> Value {
    if self.outermost
      && let Some(abort) = EVALUATION.with(Evaluation::abort)
    {
      return crate::value_null!(abort);
    }
    value
  }
}

impl Drop for Call {
  fn drop(&mut self) {
    EVALUATION.with(|evaluation| {
      if self.outermost {
        evaluation.clear();
      } else {
        evaluation.depth.set(evaluation.depth.get() - 1);
      }
    });
  }
}

/// Error handling mode of a DMN evaluation (§7.3.8). Lenient is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorMode {
  /// Errors are collected and the evaluation continues.
  #[default]
  Lenient,
  /// The evaluation stops at the first error, reports it and returns null.
  Strict,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn collection_ends_when_the_evaluation_panics() {
    let result = std::panic::catch_unwind(|| {
      collect(|| {
        record("before the panic");
        panic!("evaluation failed");
      })
    });
    assert!(result.is_err());
    // the thread no longer collects: nothing recorded outside a collection
    record("after the panic");
    let (_, diagnostics) = collect(|| ());
    assert_eq!(diagnostics, Diagnostics::default());
  }

  #[test]
  fn an_evaluation_that_panics_leaves_no_abort() {
    let result = std::panic::catch_unwind(|| {
      evaluation(|| {
        abort(Abort::limit("first"));
        panic!("evaluation failed");
      })
    });
    assert!(result.is_err());
    assert_eq!(((), None), evaluation(|| ()));
  }

  #[test]
  fn the_first_abort_reason_wins() {
    let ((), aborted) = evaluation(|| {
      abort(Abort::limit("first"));
      abort(Abort::limit("second"));
    });
    assert_eq!(Some(Abort::limit("first")), aborted);
    assert_eq!("first; evaluation stopped", aborted.unwrap().to_string());
  }

  #[test]
  fn an_abort_outside_an_evaluation_has_no_effect() {
    abort(Abort::limit("outside"));
    assert_eq!(((), None), evaluation(|| ()));
    assert!(enter_call(1).is_some());
  }

  #[test]
  fn nested_evaluations_share_one_abort() {
    let (inner, outer) = evaluation(|| evaluation(|| abort(Abort::limit("inner"))).1);
    assert_eq!(Some(Abort::limit("inner")), inner);
    assert_eq!(inner, outer);
    assert_eq!(((), None), evaluation(|| ()));
  }

  #[test]
  fn enter_call_refuses_beyond_the_limit() {
    let ((), aborted) = evaluation(|| {
      let first = enter_call(2).expect("the first call is entered");
      let second = enter_call(2).expect("the second call is entered");
      assert!(enter_call(2).is_none());
      // the evaluation is stopped: no further call is entered, at any depth
      drop(second);
      assert!(enter_call(2).is_none());
      drop(first);
      assert!(enter_call(2).is_none());
    });
    assert_eq!(Some(Abort::CallDepth { limit: 2 }), aborted);
    assert!(enter_call(2).is_some());
  }

  #[test]
  fn the_outermost_call_of_an_implicit_evaluation_returns_no_partial_value() {
    let call = enter_call(1).expect("the outermost call is entered");
    assert!(enter_call(1).is_none());
    let text = "function calls are nested too deeply (more than 1 levels, e.g. runaway recursion); evaluation stopped";
    assert_eq!(Value::Null(Some(text.to_string())), call.finish(Value::Boolean(true)));
    // the implicit evaluation ended with its outermost call
    assert_eq!(((), None), evaluation(|| ()));
    let call = enter_call(1).expect("a new outermost call is entered");
    assert_eq!(Value::Boolean(true), call.finish(Value::Boolean(true)));
  }

  #[test]
  fn nested_collections_add_up() {
    let (inner, outer) = collect(|| {
      record("outer");
      let (_, inner) = collect(|| record("inner"));
      inner
    });
    assert_eq!((inner.count, inner.messages), (1, vec!["inner".to_string()]));
    assert_eq!((outer.count, outer.messages), (2, vec!["outer".to_string(), "inner".to_string()]));
  }
}
