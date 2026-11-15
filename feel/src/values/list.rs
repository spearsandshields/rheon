//! # FEEL list values

use crate::values::Value;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// A FEEL list.
///
/// Copying the list copies a pointer: the items are shared between all copies and are never
/// changed while they are shared (FEEL lists are immutable, DMN 1.6 §10.3.2.5). The empty list
/// allocates nothing.
///
/// The items are read as a slice (through [Deref]); a list is changed
/// only through its own methods ([FeelList::push]) or by taking its items out
/// ([FeelList::into_vec], or by value through [IntoIterator]) and building a new list from them,
/// so a list always knows what it holds: it keeps the nesting depth of its deepest item, raised
/// in constant time by a push and computed in one pass over the items when a list is built from
/// them. Each of these changes the items in place when the list does not share them, and works
/// on a copy of the items otherwise (a shallow copy: each item is cloned, which shares what it
/// holds), so a change never shows through another copy of the list.
///
/// Equality is that of the items; the debug form is that of the slice of items.
#[derive(Clone, Default)]
pub struct FeelList {
  /// The items, `None` when there are none.
  items: Option<Arc<Vec<Value>>>,
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
    self.items.as_deref().map_or(&[], Vec::as_slice)
  }

  /// Appends an item at the end of this list: in place when the list does not share its items,
  /// otherwise to a copy of them, which this list then holds alone.
  pub fn push(&mut self, value: Value) {
    self.item_depth = self.item_depth.max(value.nesting_depth());
    let shared = self.items.get_or_insert_with(|| Arc::new(Vec::new()));
    match Arc::get_mut(shared) {
      Some(items) => items.push(value),
      None => {
        let mut items = Vec::with_capacity(shared.len() + 1);
        items.extend_from_slice(shared);
        items.push(value);
        *shared = Arc::new(items);
      }
    }
  }

  /// Converts this list into the vector of its items: they are moved out when the list does not
  /// share them, and cloned otherwise.
  pub fn into_vec(self) -> Vec<Value> {
    self.items.map(Arc::unwrap_or_clone).unwrap_or_default()
  }

  /// The nesting depth of this list: one more than that of its deepest item.
  pub(crate) fn nesting_depth(&self) -> u32 {
    self.item_depth.saturating_add(1)
  }
}

impl PartialEq for FeelList {
  fn eq(&self, other: &Self) -> bool {
    self.as_slice() == other.as_slice()
  }
}

impl Deref for FeelList {
  type Target = [Value];
  fn deref(&self) -> &[Value] {
    self.as_slice()
  }
}

impl fmt::Debug for FeelList {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.as_slice(), f)
  }
}

impl From<Vec<Value>> for FeelList {
  /// Converts a vector of items into a list, without a copy of the items.
  fn from(items: Vec<Value>) -> Self {
    if items.is_empty() {
      return Self::default();
    }
    let item_depth = items.iter().map(Value::nesting_depth).max().unwrap_or(0);
    Self {
      items: Some(Arc::new(items)),
      item_depth,
    }
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
  type IntoIter = ListItems;
  /// The items of this list, in order: moved out when the list does not share them, and cloned
  /// one at a time otherwise.
  fn into_iter(self) -> ListItems {
    ListItems(match self.items.map(Arc::try_unwrap) {
      None => Items::Owned(Vec::new().into_iter()),
      Some(Ok(items)) => Items::Owned(items.into_iter()),
      Some(Err(items)) => Items::Shared { items, next: 0 },
    })
  }
}

impl<'a> IntoIterator for &'a FeelList {
  type Item = &'a Value;
  type IntoIter = std::slice::Iter<'a, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.as_slice().iter()
  }
}

/// The items of a [FeelList] taken by value, in order ([FeelList::into_iter]): the items of a
/// list that held them alone are moved out; the items of a shared list are cloned one at a time,
/// as they are taken, so the items that are never taken are never copied.
pub struct ListItems(Items);

enum Items {
  Owned(std::vec::IntoIter<Value>),
  Shared { items: Arc<Vec<Value>>, next: usize },
}

impl Iterator for ListItems {
  type Item = Value;

  fn next(&mut self) -> Option<Value> {
    match &mut self.0 {
      Items::Owned(items) => items.next(),
      Items::Shared { items, next } => {
        let item = items.get(*next)?.clone();
        *next += 1;
        Some(item)
      }
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let left = match &self.0 {
      Items::Owned(items) => items.len(),
      Items::Shared { items, next } => items.len() - *next,
    };
    (left, Some(left))
  }
}

impl ExactSizeIterator for ListItems {}
