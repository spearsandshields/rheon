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
