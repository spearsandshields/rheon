//! # FEEL list values

use crate::values::Value;
use std::fmt;
use std::ops::Deref;

/// A FEEL list.
///
/// The items are read as a slice (through [Deref]); a list is changed
/// only through its own methods ([FeelList::push]) or by taking its items out
/// ([FeelList::into_vec]) and building a new list from them, so a list always knows what it holds.
///
/// Equality is that of the items; the debug form is that of the slice of items.
#[derive(Clone, Default, PartialEq)]
pub struct FeelList(Vec<Value>);

impl FeelList {
  /// Creates a new, empty list.
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns the items of this list (outside this crate through [Deref]).
  pub(crate) fn as_slice(&self) -> &[Value] {
    &self.0
  }

  /// Appends an item at the end of this list.
  pub fn push(&mut self, value: Value) {
    self.0.push(value);
  }

  /// Converts this list into the vector of its items.
  pub fn into_vec(self) -> Vec<Value> {
    self.0
  }
}

impl Deref for FeelList {
  type Target = [Value];
  fn deref(&self) -> &[Value] {
    &self.0
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
    Self(items)
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
    Self(iter.into_iter().collect())
  }
}

impl IntoIterator for FeelList {
  type Item = Value;
  type IntoIter = std::vec::IntoIter<Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl<'a> IntoIterator for &'a FeelList {
  type Item = &'a Value;
  type IntoIter = std::slice::Iter<'a, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.0.iter()
  }
}
