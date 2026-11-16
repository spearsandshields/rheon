//! # FEEL equality of two values
//!
//! A value can hold the same list or context on several paths (`[x, x]` and `{a: x, b: x}` hold
//! `x` twice without a copy), so a value that a loop doubles through `partial` holds
//! exponentially many paths in a linear size. An equality walk that followed every path would
//! take exponential time; this one remembers the answer for each pair of shared lists or shared
//! contexts it compared, and compares a pair again only when it was not met before.

use rheon_feel::values::Value;
use std::collections::HashMap;

/// The key of a pair of lists of one length or of contexts with as many entries, when either side
/// is shared ([Value::is_shared]): the addresses of their nodes ([Value::node_address]). `None` for
/// any other pair, which is compared without being remembered (a pair of lists of different
/// lengths or of contexts of different sizes is decided at once).
fn shared_pair(lhs: &Value, rhs: &Value) -> Option<(usize, usize)> {
  let same_size = match (lhs, rhs) {
    (Value::List(ls), Value::List(rs)) => ls.len() == rs.len(),
    (Value::Context(lc), Value::Context(rc)) => lc.len() == rc.len(),
    _ => false,
  };
  if same_size && (lhs.is_shared() || rhs.is_shared()) {
    lhs.node_address().zip(rhs.node_address())
  } else {
    None
  }
}

/// Evaluates the equality of two values: `None` when the values can not be compared.
///
/// Comparing with null tests if the other value is null, so it always has a result
/// (DMN 1.6, grammar rule 49.a). Other values are compared by kind (Table 54); the known
/// deviations are listed in docs/BACKLOG.md ("Equality and comparison follow-ups").
pub(crate) fn eval_ternary_equality(lhs: &Value, rhs: &Value) -> Option<bool> {
  Equality::default().compare(lhs, rhs)
}

/// One equality walk over borrowed values: the answers for the pairs of lists it compared where
/// either list shares its items, by the addresses of their items, and for the pairs of contexts
/// it compared where either context is shared, by the addresses of the contexts.
///
/// The memory lives as long as the walk, is created when the first shared pair is met, and is
/// never iterated. While the values are borrowed, no other items or contexts are stored at the
/// addresses it holds, and the items of a list and a context are never at one address, so a
/// pair of lists and a pair of contexts never have one key. Remembering only the pairs with a
/// shared side is enough: a pair is reached again only through two paths that meet in a list or
/// a context held twice, and that list or context is shared.
///
/// One walk serves every comparison of an operation that compares several pairs of values, such
/// as a built-in that compares the items of a list with one another or with a match: a pair of
/// shared lists met again below a later comparison is answered from memory as well.
#[derive(Default)]
pub(crate) struct Equality {
  shared: Option<HashMap<(usize, usize), Option<bool>>>,
}

impl Equality {
  /// The equality of two values held by the values compared: remembered for a pair of lists or
  /// a pair of contexts with a shared side, compared otherwise.
  fn nested(&mut self, lhs: &Value, rhs: &Value) -> Option<bool> {
    if let Some(key) = shared_pair(lhs, rhs) {
      if let Some(answer) = self.shared.as_ref().and_then(|shared| shared.get(&key)) {
        return *answer;
      }
      let answer = self.compare(lhs, rhs);
      self.shared.get_or_insert_with(HashMap::new).insert(key, answer);
      return answer;
    }
    self.compare(lhs, rhs)
  }

  /// The equality of two values ([eval_ternary_equality]), with the values they hold compared
  /// through this walk. The two values are never looked up: each comparison of an operation is
  /// made once, and the pairs met again are the ones below them.
  pub(crate) fn compare(&mut self, lhs: &Value, rhs: &Value) -> Option<bool> {
    if lhs.is_null() || rhs.is_null() {
      return Some(lhs.is_null() && rhs.is_null());
    }
    match lhs {
      Value::Boolean(ls) => match rhs {
        Value::Boolean(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::Number(ls) => match rhs {
        Value::Number(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::String(ls) => match rhs {
        Value::String(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::Context(ls) => match rhs {
        Value::Context(rs) => {
          if ls.len() == rs.len() {
            for (key1, value1) in ls.iter() {
              if let Some(value2) = rs.get_entry(key1) {
                if let Some(equal) = self.nested(value1, value2) {
                  if !equal {
                    return Some(false); // values in entries are NOT EQUAL
                  }
                } else {
                  // values in entries can not be compared
                  return None;
                }
              } else {
                // there is no such key in the right-side context
                return Some(false);
              }
            }
            // contexts are EQUAL
            return Some(true);
          }
          // contexts have different number of keys, so they are NOT EQUAL
          Some(false)
        }
        _ => None,
      },
      Value::Date(ls) => match rhs {
        Value::Date(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::Time(ls) => match rhs {
        Value::Time(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::DateTime(ls) => match rhs {
        Value::DateTime(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::DaysAndTimeDuration(ls) => match rhs {
        Value::DaysAndTimeDuration(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::YearsAndMonthsDuration(ls) => match rhs {
        Value::YearsAndMonthsDuration(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::List(ls) => match rhs {
        Value::List(rs) => {
          if ls.len() == rs.len() {
            for (l, r) in ls.iter().zip(rs.iter()) {
              if let Some(true) = self.nested(l, r) {
              } else {
                return Some(false);
              }
            }
            Some(true)
          } else {
            Some(false)
          }
        }
        _ => None,
      },
      Value::UnaryEqual(l_end) => match rhs {
        Value::UnaryEqual(r_end) => self.nested(l_end, r_end),
        _ => Some(false),
      },
      Value::UnaryNotEqual(l_end) => match rhs {
        Value::UnaryNotEqual(r_end) => self.nested(l_end, r_end),
        _ => Some(false),
      },
      Value::UnaryLess(l_end) => match rhs {
        Value::Range(rs, cs, re, ce) => {
          if cs.opened() && ce.opened() && !rs.is_null() {
            self.nested(l_end, re)
          } else {
            Some(false)
          }
        }
        Value::UnaryLess(r_end) => self.nested(l_end, r_end),
        _ => None,
      },
      Value::UnaryLessOrEqual(l_end) => match rhs {
        Value::Range(rs, cs, re, ce) => {
          if cs.closed() && ce.opened() && !rs.is_null() {
            self.nested(l_end, re)
          } else {
            Some(false)
          }
        }
        Value::UnaryLessOrEqual(r_end) => self.nested(l_end, r_end),
        _ => None,
      },
      Value::UnaryGreater(l_end) => match rhs {
        Value::Range(rs, cs, re, ce) => {
          if cs.opened() && ce.opened() && !re.is_null() {
            self.nested(l_end, rs)
          } else {
            Some(false)
          }
        }
        Value::UnaryGreater(r_end) => self.nested(l_end, r_end),
        _ => None,
      },
      Value::UnaryGreaterOrEqual(l_end) => match rhs {
        Value::Range(rs, cs, re, ce) => {
          if cs.closed() && ce.opened() && !re.is_null() {
            self.nested(l_end, rs)
          } else {
            Some(false)
          }
        }
        Value::UnaryGreaterOrEqual(r_end) => self.nested(l_end, r_end),
        _ => None,
      },
      Value::Range(r1s, c1s, r1e, c1e) => match rhs {
        Value::Range(r2s, c2s, r2e, c2e) => {
          if *c1s == *c2s
            && *c1e == *c2e
            && let Some(true) = self.nested(r1s, r2s)
          {
            return self.nested(r1e, r2e);
          }
          Some(false)
        }
        _ => None,
      },
      _ => None,
    }
  }
}
