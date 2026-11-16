//! # Diagnostics of an evaluation (DMN 1.6 §7.3.8)
//!
//! FEEL reports a semantic error by returning null with a reason (`value_null!`). DMN 1.6 requires
//! the evaluation to "report or log diagnostic information" and defines two error handling modes:
//! lenient (the default: errors are collected and evaluation continues) and strict (evaluation
//! halts at the first error and returns null).
//!
//! Every null an evaluation gives is one of four outcomes, read off the semantic tables:
//! - a value: the specification defines a non-null result for the input, so there is no null;
//! - a defined null: the specification defines null for the input (`sum([])`, a missing context
//!   entry, `stddev` of one number, a decision table without a matching rule) or the input is
//!   missing data. It is no error: the null is bare or carries a note (`value_null!(note: …)`,
//!   [Severity::Note]), which is not counted;
//! - an error null: an input outside the domain of the operation, a null operand of an
//!   arithmetic operator among them (the official TCK expects an error result for `1 + null`), or
//!   an error §7.3.8 names. It carries its reason (`value_null!(…)`, [Severity::Error]), which is
//!   counted and listed where the null is created, and strict mode stops at it;
//! - an abort: an implementation limit ([Abort]), never a value.
//!
//! While a caller collects diagnostics ([collect]), every error null created through
//! `value_null!` is recorded here (per thread). Outside a collection nothing is recorded, so
//! ordinary evaluations pay one thread-local flag check per error null.
//!
//! A collection also keeps warnings ([warn]): what the evaluation did to an input without an
//! error, such as fractional seconds truncated to nanoseconds. A warning is not an error: it is not
//! counted, and strict mode does not stop at it.
//!
//! An evaluation can also be stopped by an implementation limit ([Abort]), such as the depth of
//! nested function calls. One evaluation ([evaluation]) owns the abort and the call depth, every
//! function call reads them ([enter_call]), and nothing of them outlives the evaluation. The
//! depth of nested values is bounded where an expression produces a value ([bound_depth]).

use crate::values::{FeelList, Value};
use crate::{Evaluator, FeelScope};
use dsntk_common::{DsntkError, ToErrorMessage};
use std::cell::{Cell, RefCell};
use std::thread::LocalKey;

/// At most this many error messages, and this many warnings, are kept; [Diagnostics::count] still
/// counts all errors.
pub const MAX_MESSAGES: usize = 64;

thread_local! {
  static ACTIVE: Cell<bool> = const { Cell::new(false) };
  static COUNT: Cell<usize> = const { Cell::new(0) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
  static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Errors and warnings recorded during one evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics {
  /// The number of errors (null values with an error reason; a note is not counted) created.
  pub count: usize,
  /// The first [MAX_MESSAGES] error messages, in the order they occurred; consecutive repeats of
  /// the same message (an error passed up through several operators) are kept once.
  pub messages: Vec<String>,
  /// The first [MAX_MESSAGES] warnings, kept as the messages are.
  warnings: Vec<String>,
}

impl Diagnostics {
  /// The first error, if any.
  pub fn first(&self) -> Option<&str> {
    self.messages.first().map(String::as_str)
  }

  /// The first [MAX_MESSAGES] warnings, in the order they occurred; consecutive repeats of the
  /// same warning (a literal evaluated in a loop) are kept once.
  pub fn warnings(&self) -> &[String] {
    &self.warnings
  }
}

/// What the reason of a null says about the evaluation (DMN 1.6 §7.3.8).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
  /// A semantic error: counted, kept among the messages, and a strict evaluation fails on it.
  Error,
  /// The explanation of a null that the specification defines for its input, such as a missing
  /// context entry. It is no error: it is not counted, not kept, and strict mode does not stop at
  /// it. The null still carries the text.
  Note,
}

/// Records the message of an error when a collection is active (called by `value_null!`); a
/// note is not recorded.
#[inline]
pub fn record(severity: Severity, message: &str) {
  if severity == Severity::Note || !ACTIVE.with(Cell::get) {
    return;
  }
  COUNT.with(|count| count.set(count.get() + 1));
  keep(&MESSAGES, message);
}

/// Records a warning when a collection is active; outside one it has no effect, as [record].
pub fn warn(message: &str) {
  if ACTIVE.with(Cell::get) {
    keep(&WARNINGS, message);
  }
}

/// Keeps `message` in `list`: the first [MAX_MESSAGES], a consecutive repeat once.
fn keep(list: &'static LocalKey<RefCell<Vec<String>>>, message: &str) {
  list.with(|list| {
    let mut list = list.borrow_mut();
    if list.len() < MAX_MESSAGES && list.last().map(String::as_str) != Some(message) {
      list.push(message.to_string());
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

/// A mark of what the running collection holds now, taken before an evaluation whose diagnostics
/// count only when its result is used. Dropping the mark keeps what was recorded since;
/// [Tentative::discard] removes it.
pub fn tentative_mark() -> Tentative {
  Tentative(ACTIVE.with(Cell::get).then(|| Mark {
    count: COUNT.with(Cell::get),
    messages: MESSAGES.with(|messages| messages.borrow().len()),
    warnings: WARNINGS.with(|warnings| warnings.borrow().len()),
  }))
}

/// What the running collection held before a tentative evaluation ([tentative_mark]); nothing when
/// no collection was running.
#[must_use = "dropping the mark keeps what the evaluation recorded; discard it to remove that"]
pub struct Tentative(Option<Mark>);

/// The count of errors and the lengths of the lists of a collection.
struct Mark {
  count: usize,
  messages: usize,
  warnings: usize,
}

impl Tentative {
  /// Removes the errors and the warnings recorded since the mark: the count is set back and both
  /// lists are cut to their marked lengths. A message that repeated the last one before the mark
  /// was counted but not kept, so the cut is exact, also at the cap; a collection nested in the
  /// tentative evaluation merged its records into this one when it ended. An abort is no record:
  /// it stays.
  pub fn discard(self) {
    if let Some(mark) = self.0 {
      COUNT.with(|count| count.set(mark.count));
      MESSAGES.with(|messages| messages.borrow_mut().truncate(mark.messages));
      WARNINGS.with(|warnings| warnings.borrow_mut().truncate(mark.warnings));
    }
  }
}

/// The collection state of the enclosing scope, restored when the inner collection ends, also
/// when the evaluation panics (a thread that keeps collecting would record forever).
struct Collection {
  was_active: bool,
  saved_count: usize,
  saved_messages: Vec<String>,
  saved_warnings: Vec<String>,
}

impl Collection {
  fn start() -> Self {
    Self {
      was_active: ACTIVE.with(|active| active.replace(true)),
      saved_count: COUNT.with(|count| count.replace(0)),
      saved_messages: MESSAGES.with(|messages| messages.replace(Vec::new())),
      saved_warnings: WARNINGS.with(|warnings| warnings.replace(Vec::new())),
    }
  }

  fn finish(self) -> Diagnostics {
    // the state stays in place for `drop`, which restores the enclosing collection and adds this one's errors to it
    Diagnostics {
      count: COUNT.with(Cell::get),
      messages: MESSAGES.with(|messages| messages.borrow().clone()),
      warnings: WARNINGS.with(|warnings| warnings.borrow().clone()),
    }
  }
}

impl Drop for Collection {
  fn drop(&mut self) {
    let count = COUNT.with(Cell::get);
    ACTIVE.with(|active| active.set(self.was_active));
    COUNT.with(|outer| outer.set(self.saved_count + if self.was_active { count } else { 0 }));
    restore(&MESSAGES, std::mem::take(&mut self.saved_messages), self.was_active);
    restore(&WARNINGS, std::mem::take(&mut self.saved_warnings), self.was_active);
  }
}

/// Puts the enclosing collection's `saved` list back into `list`, followed by as many of the
/// inner collection's entries as the cap leaves room for when the enclosing one was collecting.
fn restore(list: &'static LocalKey<RefCell<Vec<String>>>, mut saved: Vec<String>, add_inner: bool) {
  list.with(|list| {
    let inner = list.take();
    if add_inner {
      saved.extend(inner.into_iter().take(MAX_MESSAGES.saturating_sub(saved.len())));
    }
    *list.borrow_mut() = saved;
  });
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
  /// A value nested more than `limit` levels deep ([bound_depth]).
  #[non_exhaustive]
  ValueDepth { limit: u32 },
  /// More than `limit` steps ([evaluation_within]): function calls and items of loops and filters.
  #[non_exhaustive]
  Steps { limit: u64 },
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
      Self::ValueDepth { limit } => write!(f, "values are nested too deeply (more than {limit} levels); evaluation stopped"),
      Self::Steps { limit } => write!(f, "evaluation took more than {limit} steps; evaluation stopped"),
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
  /// Whether `abort` is set, read by every function call and by every loop before each item.
  aborted: Cell<bool>,
  /// The steps the evaluation may still take, [NO_BUDGET] when it has no budget.
  steps: Cell<u64>,
  /// The budget the evaluation started with, for the abort.
  budget: Cell<u64>,
}

/// The steps left of an evaluation without a budget: never counted down.
const NO_BUDGET: u64 = u64::MAX;

impl Evaluation {
  fn active(&self) -> bool {
    self.explicit.get() || self.depth.get() > 0
  }

  fn clear(&self) {
    self.explicit.set(false);
    self.depth.set(0);
    self.aborted.set(false);
    self.abort.borrow_mut().take();
    self.steps.set(NO_BUDGET);
    self.budget.set(NO_BUDGET);
  }

  /// Counts one step of an evaluation that has a budget; the step after the last one of the
  /// budget stops the evaluation.
  fn step(&self) {
    match self.steps.get() {
      NO_BUDGET => {}
      0 => self.stop(Abort::Steps { limit: self.budget.get() }),
      left => self.steps.set(left - 1),
    }
  }

  fn stopped(&self) -> bool {
    self.aborted.get()
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
      steps: Cell::new(NO_BUDGET),
      budget: Cell::new(NO_BUDGET),
    }
  };
}

/// Runs `f` as one evaluation and returns its result with the abort that stopped it, if any.
///
/// Outside an evaluation, `f` starts with a clear state, and the state is cleared again when `f`
/// returns or unwinds. Inside an evaluation, `f` joins it: an abort in `f` stops the enclosing
/// evaluation too, and an abort before `f` is returned for `f` as well.
pub fn evaluation<T>(f: impl FnOnce() -> T) -> (T, Option<Abort>) {
  evaluation_within(None, f)
}

/// [evaluation] with a budget of `steps` steps, when given: every function call and every item
/// of a loop or a filter is a step, and the step after the last one of the budget stops the
/// evaluation with [Abort::Steps]. The budget belongs to the evaluation that `f` starts: inside
/// a running evaluation `f` joins it, with the budget that one has.
pub fn evaluation_within<T>(steps: Option<u64>, f: impl FnOnce() -> T) -> (T, Option<Abort>) {
  let opened = Opened::open();
  if let (true, Some(steps)) = (opened.0, steps) {
    EVALUATION.with(|evaluation| {
      evaluation.steps.set(steps);
      evaluation.budget.set(steps);
    });
  }
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

/// Whether the running evaluation was stopped ([abort]). A loop reads it before each further item
/// and ends; what the loop returns then is never observed, because the abort, not a value, is the
/// outcome of the evaluation. Outside an evaluation it is `false`.
pub fn stopped() -> bool {
  EVALUATION.with(Evaluation::stopped)
}

/// Counts one step of the running evaluation (an item of a loop or a filter) and returns whether
/// the evaluation is stopped, by this step or before it.
pub fn step() -> bool {
  EVALUATION.with(|evaluation| {
    evaluation.step();
    evaluation.stopped()
  })
}

/// The number of levels values may nest in each other within an evaluation: the depth of a scalar
/// is 0; a list or a context is one level deeper than the deepest value it holds, so an empty one
/// is 1 level deep; a function is one level deeper than its closure context; a range and the
/// other values that hold values are one level deeper than what they hold. Every consumer of a
/// value on every host walks twice as many levels, measured on the stack of each.
pub(crate) const MAX_VALUE_DEPTH: u32 = 500;

/// The value an expression produced, when it nests at most 500 levels.
///
/// A deeper value stops the running evaluation with [Abort::ValueDepth] and is dropped; the null
/// returned in its place is never observed, because the abort, not a value, is the outcome of the
/// evaluation. Outside an evaluation the null carries the abort's text. It is read where values
/// are produced (the result of an invocation, a list, range, context and function literal, the
/// results of a loop as they grow, a boxed expression), not where values are bound: a scope frame
/// may hold a value at the limit.
#[inline]
pub fn bound_depth(value: Value) -> Value {
  if value.nesting_depth() <= MAX_VALUE_DEPTH { value } else { too_deep() }
}

/// [bound_depth] for a list that grows in place, such as the results of a loop: `Ok` when it nests
/// at most 500 levels, otherwise the evaluation is stopped and the error is the null to return in
/// place of the list. The list keeps its depth, so this reads one field.
#[inline]
pub fn bound_list_depth(list: &FeelList) -> Result<(), Value> {
  if list.nesting_depth() <= MAX_VALUE_DEPTH { Ok(()) } else { Err(too_deep()) }
}

/// Stops the running evaluation for a value nested more than [MAX_VALUE_DEPTH] levels: the null
/// returned in place of the value ([bound_depth]).
#[cold]
#[inline(never)]
pub(crate) fn too_deep() -> Value {
  let reason = Abort::ValueDepth { limit: MAX_VALUE_DEPTH };
  abort(reason.clone());
  if stopped() { Value::Null(None) } else { crate::value_null!(reason) }
}

/// Enters a function call when at most `limit` calls are nested in it. `None` refuses the call:
/// the evaluation was stopped before, or this call exceeds the limit, which stops it with
/// [Abort::CallDepth]. A refused call's result is never observed, because the abort, not a value,
/// is the outcome of the evaluation.
pub fn enter_call(limit: usize) -> Option<Call> {
  EVALUATION.with(|evaluation| {
    evaluation.step();
    if evaluation.stopped() {
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
        record(Severity::Error, "before the panic");
        panic!("evaluation failed");
      })
    });
    assert!(result.is_err());
    // the thread no longer collects: nothing recorded outside a collection
    record(Severity::Error, "after the panic");
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
  fn warnings_are_collected_beside_the_errors() {
    let ((), diagnostics) = collect(|| warn("w"));
    assert_eq!(["w".to_string()], diagnostics.warnings());
    assert_eq!((0, 0, None), (diagnostics.count, diagnostics.messages.len(), diagnostics.first()));
    let ((), diagnostics) = collect(|| {
      record(Severity::Error, "e");
      warn("w");
    });
    assert_eq!((1, vec!["e".to_string()]), (diagnostics.count, diagnostics.messages.clone()));
    assert_eq!(["w".to_string()], diagnostics.warnings());
  }

  #[test]
  fn a_warning_outside_a_collection_has_no_effect() {
    warn("outside");
    let ((), diagnostics) = collect(|| ());
    assert_eq!(Diagnostics::default(), diagnostics);
  }

  #[test]
  fn warnings_keep_a_repeat_once_and_at_most_the_cap() {
    let ((), diagnostics) = collect(|| {
      warn("a");
      warn("a");
      warn("b");
      warn("a");
    });
    assert_eq!(["a", "b", "a"].map(String::from), diagnostics.warnings());
    let ((), diagnostics) = collect(|| (0..1_000).for_each(|i| warn(&i.to_string())));
    assert_eq!(MAX_MESSAGES, diagnostics.warnings().len());
    assert_eq!("63", diagnostics.warnings()[MAX_MESSAGES - 1]);
  }

  #[test]
  fn nested_collections_add_up_warnings() {
    let (inner, outer) = collect(|| {
      warn("outer");
      let (_, inner) = collect(|| warn("inner"));
      inner
    });
    assert_eq!(["inner".to_string()], inner.warnings());
    assert_eq!(["outer", "inner"].map(String::from), outer.warnings());
  }

  #[test]
  fn no_warning_outlives_a_collection_that_panics() {
    let result = std::panic::catch_unwind(|| {
      collect(|| {
        warn("before the panic");
        panic!("evaluation failed");
      })
    });
    assert!(result.is_err());
    warn("after the panic");
    let ((), diagnostics) = collect(|| ());
    assert_eq!(Diagnostics::default(), diagnostics);
  }

  #[test]
  fn a_note_is_no_error() {
    let (value, diagnostics) = collect(|| crate::value_null!(note: "n"));
    assert_eq!(Value::Null(Some("n".to_string())), value);
    assert_eq!((0, 0, None), (diagnostics.count, diagnostics.messages.len(), diagnostics.first()));
    // an error beside it is counted
    let (values, diagnostics) = collect(|| [crate::value_null!(note: "n"), crate::value_null!("e")]);
    assert_eq!([Value::Null(Some("n".to_string())), Value::Null(Some("e".to_string()))], values);
    assert_eq!((1, vec!["e".to_string()]), (diagnostics.count, diagnostics.messages));
  }

  #[test]
  fn a_note_with_arguments_is_capped_like_any_message() {
    let long = "x".repeat(2_000);
    let (value, diagnostics) = collect(|| crate::value_null!(note: "no entry {} in context", crate::message::arg(long.as_str())));
    let Value::Null(Some(note)) = value else { panic!("a null with a note expected") };
    assert!(note.starts_with("no entry xxx"), "{note}");
    assert!(note.chars().count() <= 1_000, "{note}");
    assert_eq!(0, diagnostics.count);
  }

  #[test]
  fn nested_collections_add_up() {
    let (inner, outer) = collect(|| {
      record(Severity::Error, "outer");
      let (_, inner) = collect(|| record(Severity::Error, "inner"));
      inner
    });
    assert_eq!((inner.count, inner.messages), (1, vec!["inner".to_string()]));
    assert_eq!((outer.count, outer.messages), (2, vec!["outer".to_string(), "inner".to_string()]));
  }

  /// What a collection holds: the count, the messages and the warnings.
  fn held(diagnostics: &Diagnostics) -> (usize, Vec<String>, Vec<String>) {
    (diagnostics.count, diagnostics.messages.clone(), diagnostics.warnings().to_vec())
  }

  #[test]
  fn a_discarded_mark_removes_what_was_recorded_since() {
    let strings = |items: &[&str]| items.iter().map(|item| item.to_string()).collect::<Vec<_>>();
    let ((), diagnostics) = collect(|| {
      record(Severity::Error, "before");
      warn("w before");
      let mark = tentative_mark();
      {
        record(Severity::Error, "probe");
        // a repeat of the message before the mark is counted, not kept
        record(Severity::Error, "probe");
        warn("w probe");
        // a collection inside the mark merges into it before the mark is discarded
        let ((), inner) = collect(|| record(Severity::Error, "inner"));
        assert_eq!(1, inner.count);
      }
      mark.discard();
      record(Severity::Error, "after");
    });
    assert_eq!((2, strings(&["before", "after"]), strings(&["w before"])), held(&diagnostics));
    // a repeat of the last message before the mark: counted inside, not kept, and its count discarded
    let ((), diagnostics) = collect(|| {
      record(Severity::Error, "same");
      let mark = tentative_mark();
      record(Severity::Error, "same");
      mark.discard();
    });
    assert_eq!((1, strings(&["same"]), vec![]), held(&diagnostics));
  }

  #[test]
  fn a_discarded_mark_at_the_cap_restores_the_count() {
    let ((), diagnostics) = collect(|| {
      (0..MAX_MESSAGES).for_each(|i| record(Severity::Error, &i.to_string()));
      let mark = tentative_mark();
      (0..10).for_each(|i| record(Severity::Error, &format!("probe {i}")));
      mark.discard();
    });
    assert_eq!(MAX_MESSAGES, diagnostics.count);
    assert_eq!(MAX_MESSAGES, diagnostics.messages.len());
    assert_eq!("63", diagnostics.messages[MAX_MESSAGES - 1]);
  }

  #[test]
  fn a_kept_mark_changes_nothing() {
    let (kept, diagnostics) = collect(|| {
      record(Severity::Error, "before");
      let mark = tentative_mark();
      record(Severity::Error, "probe");
      warn("w probe");
      drop(mark);
      7
    });
    assert_eq!(7, kept);
    assert_eq!((2, vec!["before".to_string(), "probe".to_string()], vec!["w probe".to_string()]), held(&diagnostics));
  }

  #[test]
  fn a_mark_outside_a_collection_has_no_effect() {
    let mark = tentative_mark();
    record(Severity::Error, "outside");
    mark.discard();
    let ((), diagnostics) = collect(|| ());
    assert_eq!(Diagnostics::default(), diagnostics);
  }

  #[test]
  fn an_abort_survives_a_discarded_mark() {
    let ((), aborted) = evaluation(|| {
      let mark = tentative_mark();
      abort(Abort::limit("in the probe"));
      mark.discard();
      assert!(stopped());
    });
    assert_eq!(Some(Abort::limit("in the probe")), aborted);
  }
}
