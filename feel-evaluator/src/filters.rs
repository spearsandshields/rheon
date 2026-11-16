//! # FEEL filter implementation

use crate::iterations::lost_context;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::arg;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, Name, value_null};
use std::sync::{Arc, LazyLock};

/// Name of the special context entry holding the item of a filtered list.
static ITEM: LazyLock<Name> = LazyLock::new(|| Name::from("item"));

/// The outcome of the predicate for one item: `Ok(true)` when the item is kept; `Err` (a null)
/// when the predicate is neither a boolean, a number nor null.
fn outcome(predicate: Value) -> Result<bool, Value> {
  match predicate {
    Value::Boolean(true) => Ok(true),
    Value::Boolean(false) | Value::Number(_) | Value::Null(_) => Ok(false),
    _ => Err(value_null!("only number or boolean indexes are allowed in filters")),
  }
}

/// The null that ends the filter when the evaluation was stopped ([diagnostics::stopped]), read
/// before each item and before the filter expression is evaluated once more for an index: the
/// abort, not a value, is the outcome of the evaluation.
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
      Value::List(values) => {
        let kept = match kept_items(scope, &values, evaluator).and_then(|kept| stopped().map(|()| kept)) {
          Ok(kept) => kept,
          Err(null) => return null,
        };
        let rhv = evaluator(scope);
        match rhv {
          Value::Number(index) => {
            if index.is_integer() {
              let list_size = values.len();
              if list_size > 0 {
                if !index.is_negative() {
                  let n = {
                    // converted through u64, so 32-bit targets (wasm32) behave like 64-bit ones
                    if let Ok(u_index) = u64::try_from(index) {
                      u_index
                    } else {
                      return value_null!("index is out of range 1..2⁶⁴: {}", arg(&index));
                    }
                  };
                  if n > 0 && n <= list_size as u64 {
                    let n = n as usize;
                    // unwrap below is safe, index `n` is checked above, `values` holds every item
                    values.get(n - 1).unwrap().to_owned()
                  } else {
                    value_null!("index in filter is out of range [1..{}], actual index is {}", list_size, n)
                  }
                } else {
                  let n = {
                    if let Ok(u_index) = u64::try_from(index.abs()) {
                      u_index
                    } else {
                      return value_null!("index is out of range 1..2⁶⁴: {}", arg(&index));
                    }
                  };
                  if n > 0 && n <= list_size as u64 {
                    let n = n as usize;
                    // unwrap below is safe, index `n` is checked above, `values` holds every item
                    values.get(list_size - n).unwrap().to_owned()
                  } else {
                    value_null!("index in filter is out of range [-{}..-1], actual index is -{}", list_size, n)
                  }
                }
              } else {
                // return null when the list is empty, no matter what value the index has
                value_null!()
              }
            } else {
              value_null!("index in filter must be an integer value, actual value is {}", arg(&index))
            }
          }
          _ => Value::List(kept.into()),
        }
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
