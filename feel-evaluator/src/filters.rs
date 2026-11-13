//! # FEEL filter implementation

use crate::iterations::lost_context;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, Name, value_null};
use std::sync::LazyLock;

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

/// The context that [lost_context] names when the predicate removed the item contexts from the
/// scope: it popped more contexts than it pushed, or it replaced the special context
/// `{item: value}` with a context that has no entry `item` (no FEEL predicate does either). The
/// item is lost, so no result can be given.
const ITEM_CONTEXT: &str = "item context of a filter";

/// Evaluates the predicate for every item with copies of the item in scope: the entries of a
/// context item, and on top of them the special context `{item: value}`, which is searched
/// first, unless the item has an entry `item`. Returns which items the predicate keeps.
fn matched_copied(scope: &FeelScope, values: &[Value], evaluator: &Evaluator) -> Result<Vec<bool>, Value> {
  let name_item: &Name = &ITEM;
  let mut matched = Vec::with_capacity(values.len());
  for value in values {
    let depth = scope.depth();
    let has_item_entry = if let Value::Context(local_context) = value {
      scope.push(local_context.clone());
      local_context.contains_entry(name_item)
    } else {
      false
    };
    if !has_item_entry {
      let mut special_context = FeelContext::default();
      special_context.set_entry(name_item, value.clone());
      scope.push(special_context);
    }
    let predicate = evaluator(scope);
    // remove the item contexts (and any context the predicate left behind)
    scope.truncate(depth);
    matched.push(outcome(predicate)?);
  }
  Ok(matched)
}

/// [matched_copied] with every item moved into the scope and back: a context item with an entry
/// `item` is the item context itself; otherwise the item is the value of `item` in a special
/// context reused for all items, on top of a copy of the item's entries for a context item.
fn matched_moved(scope: &FeelScope, values: &mut [Value], evaluator: &Evaluator) -> Result<Vec<bool>, Value> {
  let name_item: &Name = &ITEM;
  let mut special_context = FeelContext::default();
  let mut matched = Vec::with_capacity(values.len());
  for slot in values.iter_mut() {
    let depth = scope.depth();
    let predicate = match std::mem::replace(slot, Value::Null(None)) {
      Value::Context(local_context) if local_context.contains_entry(name_item) => {
        scope.push(local_context);
        let predicate = evaluator(scope);
        let Some(local_context) = scope.pop_frame(depth) else {
          return Err(lost_context(ITEM_CONTEXT));
        };
        *slot = Value::Context(local_context);
        predicate
      }
      value => {
        let item_depth = if let Value::Context(local_context) = &value {
          scope.push(local_context.clone());
          depth + 1
        } else {
          depth
        };
        special_context.assign_entry(name_item, value);
        scope.push(std::mem::take(&mut special_context));
        let predicate = evaluator(scope);
        let popped = scope.pop_frame(item_depth);
        scope.truncate(depth);
        let Some(ctx) = popped else {
          return Err(lost_context(ITEM_CONTEXT));
        };
        special_context = ctx;
        // the entry stays (with a placeholder) for the next item
        let Some(item) = special_context.get_entry_mut(name_item).map(|entry| std::mem::replace(entry, Value::Null(None))) else {
          return Err(lost_context(ITEM_CONTEXT));
        };
        *slot = item;
        predicate
      }
    };
    matched.push(outcome(predicate)?);
  }
  Ok(matched)
}

/// Filter expression evaluator.
pub struct FilterExpressionEvaluator {
  /// How the predicate gets the item contexts.
  mode: ItemContexts,
}

/// How the predicate of a filter gets the contexts of an item.
#[derive(Clone, Copy, PartialEq)]
enum ItemContexts {
  /// Copies of the item: what the predicate writes into the context on the top of the scope
  /// (boxed contexts do, model-evaluator) does not change the item.
  Copied,
  /// The item itself, moved into the scope and back: for FEEL predicates, which never write
  /// into the context on the top of the scope.
  Moved,
}

impl Default for FilterExpressionEvaluator {
  /// Implements [Default] trait for [FilterExpressionEvaluator].
  fn default() -> Self {
    Self::new()
  }
}

impl FilterExpressionEvaluator {
  /// Creates a filter expression evaluator; its predicate gets copies of every item.
  pub fn new() -> Self {
    Self { mode: ItemContexts::Copied }
  }

  /// Creates a filter expression evaluator for a FEEL filter expression: every item is moved into
  /// the scope and back instead of copied, because a FEEL predicate never writes into the context
  /// on the top of the scope.
  pub fn for_feel() -> Self {
    Self { mode: ItemContexts::Moved }
  }

  pub fn evaluate(&self, scope: &FeelScope, value: Value, evaluator: &Evaluator) -> Value {
    match value {
      Value::List(mut values) => {
        let matched = match self.mode {
          ItemContexts::Copied => matched_copied(scope, &values, evaluator),
          ItemContexts::Moved => matched_moved(scope, &mut values, evaluator),
        };
        let matched = match matched {
          Ok(matched) => matched,
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
                      return value_null!("index is out of range 1..2⁶⁴: {}", index.to_string());
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
                      return value_null!("index is out of range 1..2⁶⁴: {}", index.to_string());
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
              value_null!("index in filter must be an integer value, actual value is {}", index)
            }
          }
          _ => Value::List(values.into_iter().zip(matched).filter_map(|(value, matched)| matched.then_some(value)).collect()),
        }
      }
      // A filter on a non-list value applies to the singleton list containing it (implicit
      // conversion to singleton list, DMN 1.6 §10.3.2.9.4, DMN16-94), so `item` and the context
      // entries of a singleton context are in scope like for a list element.
      Value::Null(_) => value_null!(),
      v => self.evaluate(scope, Value::List(vec![v]), evaluator),
    }
  }
}
