//! # FEEL filter implementation

use crate::iterations::lost_context;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::arg;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelNumber, FeelScope, Name, value_null};
use std::sync::{Arc, LazyLock};

/// Name of the special context entry holding the item of a filtered list.
pub(crate) static ITEM: LazyLock<Name> = LazyLock::new(|| Name::from("item"));

/// The outcome of the predicate for one item: `Ok(true)` when the item is kept; `Err` (a null)
/// when the predicate is neither a boolean, a number nor null.
pub(crate) fn outcome(predicate: Value) -> Result<bool, Value> {
  match predicate {
    Value::Boolean(true) => Ok(true),
    Value::Boolean(false) | Value::Number(_) | Value::Null(_) => Ok(false),
    _ => Err(value_null!("only number or boolean indexes are allowed in filters")),
  }
}

/// The null that ends the filter when the evaluation was stopped ([diagnostics::stopped]), read
/// before each item and before the result is given: the abort, not a value, is the outcome of
/// the evaluation.
fn stopped() -> Result<(), Value> {
  if diagnostics::stopped() { Err(Value::Null(None)) } else { Ok(()) }
}

/// The context that [lost_context] names when the predicate removed the item contexts from the
/// scope: it popped more contexts than it pushed, or it replaced the special context
/// `{item: value}` with a context that has no entry `item` (no FEEL predicate does either). The
/// item is lost, so no result can be given.
const ITEM_CONTEXT: &str = "item context of a filter";

/// Evaluates the predicate for every item with the item in scope, and returns the items it keeps,
/// cloned (a list or a context item is shared, not copied).
///
/// The item is in scope through the entries of a context item, pushed shared, and on top of them
/// the special context `{item: value}`, which is searched first, unless the item has an entry
/// `item`. The special context is one context for all items, pushed shared as well, and changed
/// between items only, when the scope no longer holds it. So what the predicate writes into the
/// context on the top of the scope (a boxed context as the match of a boxed filter does,
/// model-evaluator) goes into a copy, which is dropped with the frame: neither the item nor the
/// predicate of the next item sees it.
fn kept_items(scope: &FeelScope, values: &[Value], evaluator: &Evaluator) -> Result<Vec<Value>, Value> {
  let name_item: &Name = &ITEM;
  let mut special_context = Arc::new(FeelContext::default());
  let mut kept = vec![];
  for value in values {
    stopped()?;
    let depth = scope.depth();
    let has_item_entry = if let Value::Context(local_context) = value {
      scope.push_shared(Arc::clone(local_context));
      local_context.contains_entry(name_item)
    } else {
      false
    };
    let predicate = if has_item_entry {
      let predicate = evaluator(scope);
      if scope.pop_frame(depth).is_none() {
        return Err(lost_context(ITEM_CONTEXT));
      }
      predicate
    } else {
      let item_depth = scope.depth();
      Arc::make_mut(&mut special_context).assign_entry(name_item, value.clone());
      scope.push_shared(Arc::clone(&special_context));
      let predicate = evaluator(scope);
      let popped = scope.pop_frame(item_depth);
      scope.truncate(depth);
      if !popped.is_some_and(|frame| frame.contains_entry(name_item)) {
        return Err(lost_context(ITEM_CONTEXT));
      }
      predicate
    };
    if outcome(predicate)? {
      kept.push(value.clone());
    }
  }
  Ok(kept)
}

/// The context that [lost_context] names when the filter expression, evaluated in the enclosing
/// scope, removed the frame it was given (no FEEL expression does).
const ENCLOSING_FRAME: &str = "frame of a filter expression in the enclosing scope";

/// Evaluates the filter expression in the enclosing scope, tentatively ([diagnostics::tentative_mark]),
/// in an empty frame of its own on top of it: what the expression writes into the context on the
/// top of the scope (a boxed context as the match of a boxed filter does, model-evaluator) is
/// dropped with that frame, so neither the enclosing scope nor the items see it.
fn evaluate_in_own_frame(scope: &FeelScope, evaluator: &Evaluator) -> Result<(Value, diagnostics::Tentative), Value> {
  let depth = scope.depth();
  scope.push(FeelContext::default());
  let mark = diagnostics::tentative_mark();
  let value = evaluator(scope);
  if scope.pop_frame(depth).is_none() {
    return Err(lost_context(ENCLOSING_FRAME));
  }
  Ok((value, mark))
}

/// The item of a list at the index of a filter (DMN 1.6 Table 69, "e1 is a list and e2 is an
/// integer"): counted from the first item when it is positive and from the last when it is
/// negative. Null when the list is empty, whatever the index is; otherwise null with its reason
/// when the index is no integer, beyond 2⁶⁴ in magnitude, 0, or past the end of the list.
pub(crate) fn item_at(items: &[Value], index: FeelNumber) -> Value {
  if !index.is_integer() {
    return value_null!("index in filter must be an integer value, actual value is {}", arg(&index));
  }
  if items.is_empty() {
    return value_null!();
  }
  let from_last = index.is_negative();
  // converted through u64, so 32-bit targets (wasm32) behave like 64-bit ones
  let Ok(n) = u64::try_from(if from_last { index.abs() } else { index }) else {
    return value_null!("index is out of range 1..2⁶⁴: {}", arg(&index));
  };
  let size = items.len();
  if n == 0 || n > size as u64 {
    return if from_last {
      value_null!("index in filter is out of range [-{}..-1], actual index is -{}", size, n)
    } else {
      value_null!("index in filter is out of range [1..{}], actual index is {}", size, n)
    };
  }
  // `n` is in 1..=size, checked above
  let n = n as usize;
  items[if from_last { size - n } else { n - 1 }].clone()
}

/// Filter expression evaluator.
#[derive(Default)]
#[non_exhaustive]
pub struct FilterExpressionEvaluator;

impl FilterExpressionEvaluator {
  /// Creates a filter expression evaluator.
  pub fn new() -> Self {
    Self
  }

  pub fn evaluate(&self, scope: &FeelScope, value: Value, evaluator: &Evaluator) -> Value {
    match value {
      // DMN 1.6 Table 69: a number in the enclosing scope selects an item (row 1); otherwise the
      // expression is evaluated for each item, with the item in scope (the boolean rows)
      Value::List(values) => {
        let (probe, mark) = match evaluate_in_own_frame(scope, evaluator) {
          Ok(probe) => probe,
          Err(null) => return null,
        };
        let result = match probe {
          Value::Number(index) => stopped().map(|()| item_at(&values, index)),
          _ => {
            // an evaluation that selects no item does not count: what it recorded is removed
            mark.discard();
            kept_items(scope, &values, evaluator).and_then(|kept| stopped().map(|()| Value::List(kept.into())))
          }
        };
        result.unwrap_or_else(|null| null)
      }
      // A filter on a non-list value applies to the singleton list containing it (implicit
      // conversion to singleton list, DMN 1.6 §10.3.2.9.4, DMN16-94), so `item` and the context
      // entries of a singleton context are in scope like for a list element.
      Value::Null(_) => value_null!(),
      // the list holding the value is one level deeper than the value
      v => diagnostics::bound_depth(self.evaluate(scope, Value::List(vec![v].into()), evaluator)),
    }
  }
}
