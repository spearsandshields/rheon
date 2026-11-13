//! Compile-time checks that the public types a host can keep across threads are `Send + Sync`.
//! The `for`, `some` and `every` evaluators hold their iteration states as trait objects without
//! these bounds, so they are neither.

use crate::{Dialect, FilterExpressionEvaluator};

fn send_sync<T: Send + Sync>() {}

#[test]
fn evaluator_types_are_send_and_sync() {
  send_sync::<FilterExpressionEvaluator>();
  send_sync::<Dialect>();
}
