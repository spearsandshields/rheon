//! # FEEL filter implementation

use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, Name, value_null};

pub struct FilterExpressionEvaluator {}

impl Default for FilterExpressionEvaluator {
  /// Implements [Default] trait for [FilterExpressionEvaluator].
  fn default() -> Self {
    Self::new()
  }
}

impl FilterExpressionEvaluator {
  pub fn new() -> Self {
    Self {}
  }

  pub fn evaluate(&self, scope: &FeelScope, value: Value, evaluator: &Evaluator) -> Value {
    let name_item: Name = "item".into();
    match value {
      Value::List(values) => {
        let mut filtered_values = vec![];
        for value in &values {
          let (added_local_context, has_item_entry) = if let Value::Context(local_context) = value {
            scope.push(local_context.clone());
            if local_context.contains_entry(&name_item) { (true, true) } else { (true, false) }
          } else {
            (false, false)
          };
          if !has_item_entry {
            let mut special_context = FeelContext::default();
            special_context.set_entry(&name_item, value.clone());
            scope.push(special_context);
          }
          let rhv = evaluator(scope);
          match rhv {
            Value::Boolean(true) => filtered_values.push(value.clone()),
            Value::Boolean(false) | Value::Number(_) | Value::Null(_) => {}
            _ => return value_null!("only number or boolean indexes are allowed in filters"),
          }
          if !has_item_entry {
            scope.pop();
          }
          if added_local_context {
            scope.pop();
          }
        }
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
                    // unwrap below is safe, index `n` is checked above, `values` variable is immutable
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
                    // unwrap below is safe, index `n` is checked above, `values` variable is immutable
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
          _ => Value::List(filtered_values),
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
