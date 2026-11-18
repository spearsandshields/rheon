//! An evaluation stopped by an implementation limit: its partial results never escape, and nothing
//! of it is left for the next evaluation.

use crate::prepare;
use rheon_feel::{Evaluator, FeelScope};

const CALL_DEPTH: &str = "function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped";

/// Runs `f` on a thread with a stack big enough for the call depth limit in unoptimized builds.
fn with_big_stack(f: impl FnOnce() + Send + 'static) {
  std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

fn prepared(text: &str) -> Evaluator {
  prepare(&rheon_feel_parser::parse_expression(&FeelScope::default(), text).unwrap())
}

#[test]
fn a_prepared_evaluator_outside_a_session_leaves_no_state() {
  with_big_stack(|| {
    let scope = FeelScope::default();
    assert_eq!(format!("null({CALL_DEPTH})"), prepared("{f: function(n) f(n + 1), r: f(1)}.r")(&scope).to_string());
    assert_eq!("1", prepared("abs(-1)")(&scope).to_string());
  });
}

#[test]
fn a_stopped_call_returns_no_value_to_the_enclosing_expression() {
  with_big_stack(|| {
    let scope = FeelScope::default();
    assert_eq!(format!("null({CALL_DEPTH})"), prepared("{f: function(n) f(n + 1), r: f(1)}.r = null")(&scope).to_string());
    assert_eq!(
      format!("null({CALL_DEPTH})"),
      prepared("if {f: function(n) f(n + 1), r: f(1)}.r = null then 1 else 2")(&scope).to_string()
    );
  });
}
