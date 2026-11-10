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
  let was_active = ACTIVE.with(|active| active.replace(true));
  let saved_count = COUNT.with(|count| count.replace(0));
  let saved_messages = MESSAGES.with(|messages| messages.replace(Vec::new()));
  let result = f();
  let diagnostics = Diagnostics {
    count: COUNT.with(Cell::get),
    messages: MESSAGES.with(|messages| messages.take()),
  };
  ACTIVE.with(|active| active.set(was_active));
  COUNT.with(|count| count.set(saved_count + if was_active { diagnostics.count } else { 0 }));
  MESSAGES.with(|messages| {
    let mut restored = saved_messages;
    if was_active {
      for message in &diagnostics.messages {
        if restored.len() < MAX_MESSAGES {
          restored.push(message.clone());
        }
      }
    }
    *messages.borrow_mut() = restored;
  });
  (result, diagnostics)
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
