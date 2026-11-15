//! # FEEL list values

use crate::values::Value;
use std::fmt;
use std::ops::Deref;

/// A FEEL list.
///
/// The items are read as a slice (through [Deref]); a list is changed
/// only through its own methods ([FeelList::push]) or by taking its items out
/// ([FeelList::into_vec]) and building a new list from them, so a list always knows what it holds:
/// it keeps the nesting depth of its deepest item, raised in constant time by a push and computed
/// in one pass over the items when a list is built from them.
///
/// Equality is that of the items; the debug form is that of the slice of items.
#[derive(Clone, Default)]
pub struct FeelList {
  items: Vec<Value>,
  /// The nesting depth of the deepest item, 0 when there are no items.
  item_depth: u32,
}

impl FeelList {
  /// Creates a new, empty list.
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns the items of this list (outside this crate through [Deref]).
  pub(crate) fn as_slice(&self) -> &[Value] {
    &self.items
  }

  /// Appends an item at the end of this list.
  pub fn push(&mut self, value: Value) {
    self.item_depth = self.item_depth.max(value.nesting_depth());
    self.items.push(value);
  }

  /// Converts this list into the vector of its items.
  pub fn into_vec(self) -> Vec<Value> {
    self.items
  }

  /// The nesting depth of this list: one more than that of its deepest item.
  pub(crate) fn nesting_depth(&self) -> u32 {
    self.item_depth.saturating_add(1)
  }
}

impl PartialEq for FeelList {
  fn eq(&self, other: &Self) -> bool {
    self.items == other.items
  }
}

impl Deref for FeelList {
  type Target = [Value];
  fn deref(&self) -> &[Value] {
    &self.items
  }
}

impl fmt::Debug for FeelList {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.as_slice(), f)
  }
}

impl From<Vec<Value>> for FeelList {
  /// Converts a vector of items into a list, without a copy.
  fn from(items: Vec<Value>) -> Self {
    let item_depth = items.iter().map(Value::nesting_depth).max().unwrap_or(0);
    Self { items, item_depth }
  }
}

impl From<FeelList> for Value {
  /// Converts a [FeelList] into [Value::List].
  fn from(list: FeelList) -> Self {
    Value::List(list)
  }
}

impl FromIterator<Value> for FeelList {
  fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
    Self::from(iter.into_iter().collect::<Vec<Value>>())
  }
}

impl IntoIterator for FeelList {
  type Item = Value;
  type IntoIter = std::vec::IntoIter<Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.items.into_iter()
  }
}

impl<'a> IntoIterator for &'a FeelList {
  type Item = &'a Value;
  type IntoIter = std::slice::Iter<'a, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.items.iter()
  }
}
