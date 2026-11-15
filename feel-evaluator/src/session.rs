//! # One evaluation request
//!
//! A [Session] runs one evaluation. It owns the evaluation's limit state: the depth of nested
//! function calls and the abort that stops the evaluation when an implementation limit is hit.
//! That state starts clear and is cleared when the session ends, so nothing of one evaluation is
//! left for the next. The [Outcome] is the value, or the abort when the evaluation was stopped:
//! a value computed after a limit was hit is never the result (DMN 1.6 §7.3.8).

use rheon_feel::diagnostics::{self, Abort};

/// One evaluation request.
#[derive(Debug, Default)]
pub struct Session {}

impl Session {
  /// A session with the default configuration.
  pub fn new() -> Self {
    Self::default()
  }

  /// Runs `f` in this session, or as part of the session already running on this thread.
  pub fn run<T>(self, f: impl FnOnce() -> T) -> Outcome<T> {
    let (value, aborted) = diagnostics::evaluation(f);
    Outcome { value, aborted }
  }
}

/// The outcome of an evaluation: its value, or why it was stopped.
#[must_use]
#[derive(Debug)]
pub struct Outcome<T> {
  value: T,
  aborted: Option<Abort>,
}

impl<T> Outcome<T> {
  /// The value when the evaluation finished, the abort when it was stopped (the value computed
  /// after the abort is dropped).
  pub fn into_result(self) -> Result<T, Abort> {
    match self.aborted {
      None => Ok(self.value),
      Some(abort) => Err(abort),
    }
  }
}
