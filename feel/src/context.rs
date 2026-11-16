//! FEEL context.

use crate::diagnostics;
use crate::errors::*;
use crate::message::{MessageArg, Size};
use crate::names::Name;
use crate::qualified_names::QualifiedName;
use crate::strings::{ToFeelString, write_separated};
use crate::value_null;
use crate::values::Value;
use dsntk_common::{DsntkError, Jsonify, Result};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Bound;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Type alias for context entries.
type FeelContextEntries = BTreeMap<Name, Value>;

/// The FEEL context.
///
/// A context remembers its nesting depth once it was read, until its entries change: every
/// method that changes an entry goes through `FeelContext::entries_mut`, which forgets it, so a
/// change costs one store and a context that is only written (a scope frame) never computes it.
/// Reading it again after a change looks at each entry once, and an entry that is a list or a
/// context read since it last changed answers in constant time.
///
/// The remembered depth is no cache: it is derived from the entries alone and exact whenever it is
/// set. It goes from "not known" to the depth on a read and back on a change, a clone copies it
/// with the identical entries, and no other state is kept. Two reads of one context, through any
/// handles and on any threads, store the same number, so a context read through a shared handle
/// stays equal to one never read.
///
/// Equality is that of the entries; the debug form is `FeelContext({..})` with the entries.
#[derive(Default)]
pub struct FeelContext {
  entries: FeelContextEntries,
  /// The nesting depth of this context when it was read since the last change, 0 otherwise
  /// (a context is at least one level deep).
  depth: AtomicU32,
}

impl Clone for FeelContext {
  fn clone(&self) -> Self {
    Self {
      entries: self.entries.clone(),
      depth: AtomicU32::new(self.depth.load(Ordering::Relaxed)),
    }
  }
}

impl PartialEq for FeelContext {
  fn eq(&self, other: &Self) -> bool {
    self.entries == other.entries
  }
}

impl fmt::Debug for FeelContext {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("FeelContext").field(&self.entries).finish()
  }
}

impl TryFrom<Value> for FeelContext {
  type Error = DsntkError;
  /// Converts [Value] to [FeelContext].
  /// The context is moved out of a value that holds it alone, and copied out of a shared one
  /// (a shallow copy: each entry clone shares what the entry holds).
  fn try_from(value: Value) -> Result<Self, Self::Error> {
    let Value::Context(ctx) = value else {
      return Err(err_value_is_not_a_context(&value));
    };
    Ok(Arc::unwrap_or_clone(ctx))
  }
}

impl From<FeelContext> for Value {
  /// Converts [FeelContext] to [Value], which holds it alone.
  fn from(ctx: FeelContext) -> Self {
    Value::Context(Arc::new(ctx))
  }
}

impl MessageArg for FeelContext {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }

  fn size(&self) -> Option<Size> {
    Some(Size::Entries(self.entries.len()))
  }
}

impl fmt::Display for FeelContext {
  /// Convert [FeelContext] to string.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("{")?;
    write_separated(
      f,
      &self.entries,
      |f, (name, value)| {
        if name.is_empty() { write!(f, r#""": {value}"#) } else { write!(f, "{name}: {value}") }
      },
    )?;
    f.write_str("}")
  }
}

impl ToFeelString for FeelContext {
  /// Converts [FeelContext] to FEEL string.
  fn to_feel_string(&self) -> String {
    format!(
      "{{{}}}",
      self
        .entries
        .iter()
        .map(|(name, value)| {
          let name_str = format!("{name}");
          let padded_name_str = match name_str.as_str() {
            "{" | "}" | ":" | "," => format!("\"{name_str}\""),
            "\"" => "\"\\\"\"".to_string(),
            _ => name_str,
          };
          format!(r#"{padded_name_str}: {value}"#)
        })
        .collect::<Vec<String>>()
        .join(", ")
    )
  }
}

impl Jsonify for FeelContext {
  /// Converts [FeelContext] to JSON string.
  fn jsonify(&self) -> String {
    format!(
      "{{{}}}",
      self
        .entries
        .iter()
        .map(|(name, value)| format!(r#""{}": {}"#, name, value.jsonify()))
        .collect::<Vec<String>>()
        .join(", ")
    )
  }
}

impl FeelContext {
  /// Creates a new, empty context.
  pub fn new() -> Self {
    Self::default()
  }

  /// Returns `true` if context contains an entry specified by **name** (a [Name] or its text).
  pub fn contains_entry<Q: Ord + ?Sized>(&self, name: &Q) -> bool
  where
    Name: Borrow<Q>,
  {
    self.entries.contains_key(name)
  }

  /// Returns `true` if this [FeelContext] contains an entry specified by [QualifiedName].
  pub fn contains_entries(&self, qname: &QualifiedName) -> bool {
    self.contains_deep(qname.as_slice())
  }

  /// Returns a value of an entry with specified name.
  pub fn get_entry(&self, name: &Name) -> Option<&Value> {
    self.entries.get(name)
  }

  /// Sets a value for specified entry name.
  pub fn set_entry(&mut self, name: &Name, value: Value) {
    self.put(name.clone(), value);
  }

  /// Inserts an entry, taking the name (an entry with the same name is replaced, like [FeelContext::set_entry]).
  pub fn insert(&mut self, name: Name, value: Value) {
    self.put(name, value);
  }

  /// Sets a value for specified entry name like [FeelContext::set_entry], but clones the name only
  /// when the entry is new: for entries that are written again and again (iteration variables).
  /// For a new entry it costs one more lookup than [FeelContext::set_entry].
  ///
  /// Returns the value it replaces, or `None` when the entry is new: so a value moved into an entry
  /// is taken back out by assigning a placeholder.
  pub fn assign_entry(&mut self, name: &Name, value: Value) -> Option<Value> {
    let entries = self.entries_mut();
    if let Some(entry) = entries.get_mut(name) {
      Some(std::mem::replace(entry, value))
    } else {
      entries.insert(name.clone(), value);
      None
    }
  }

  /// Removes a value of an entry with specified name.
  pub fn remove_entry(&mut self, name: &Name) -> Option<Value> {
    self.entries_mut().remove(name)
  }

  /// Sets a null value for specified entry.
  pub fn set_null(&mut self, name: Name) {
    self.put(name, value_null!());
  }

  /// Returns a list of all [FeelContext] entries.
  pub fn get_entries(&self) -> Vec<(&Name, &Value)> {
    self.entries.iter().collect::<Vec<(&Name, &Value)>>()
  }

  /// Returns an iterator over all entries in [FeelContext], in the order of their names.
  pub fn iter(&self) -> impl ExactSizeIterator<Item = (&Name, &Value)> {
    self.entries.iter()
  }

  /// Returns an iterator over the names of the entries from the first name not less than `name` on, in their order.
  pub fn keys_from(&self, name: &str) -> impl Iterator<Item = &Name> {
    self.entries.range::<str, _>((Bound::Included(name), Bound::Unbounded)).map(|(key, _)| key)
  }

  /// Returns a first value contained by context.
  pub fn get_first(&self) -> Option<&Value> {
    self.entries.values().next()
  }

  /// Returns the number of entries in [FeelContext].
  pub fn len(&self) -> usize {
    self.entries.len()
  }

  /// Returns `true` if [FeelContext] is empty.
  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  /// Returns `true` if [FeelContext] contains an entry with specified name, and the value is context.
  pub fn is_context(&self, name: &Name) -> bool {
    matches!(self.entries.get(name), Some(Value::Context(_)))
  }

  pub fn zip(&mut self, other: &FeelContext) {
    for (name, value) in &other.entries {
      self.put(name.clone(), value.clone());
    }
  }

  pub fn overwrite(&mut self, other: &FeelContext) {
    for (name, value) in &other.entries {
      if self.entries.contains_key(name) {
        self.put(name.clone(), value.clone());
      }
    }
  }

  //TODO refactor the name, this operation is not moving, it is like prefixing
  pub fn move_entry(&mut self, name: Name, parent: Name) {
    if let Some(value) = self.remove_entry(&name) {
      self.create_entries(&[parent, name], value);
    }
  }

  /// Creates an entry with a value for specified [QualifiedName].
  /// All non-existing intermediary contexts will be created.
  pub fn create_entry(&mut self, qname: &QualifiedName, value: Value) {
    self.create_entries(qname.as_slice(), value);
  }

  /// Searches for a value of an entry pointed by specified qualified name.
  pub fn search_entry<'a>(&'a self, qname: &'a QualifiedName) -> Option<&'a Value> {
    self.search_deep(qname.as_slice())
  }

  /// Deep check for a value pointed by slice of names.
  pub fn contains_deep(&self, names: &[Name]) -> bool {
    if names.is_empty() {
      return false;
    }
    let tail = &names[1..];
    if let Some(value) = self.entries.get(&names[0]) {
      if tail.is_empty() {
        return true;
      }
      if let Value::Context(context) = value {
        return context.contains_deep(tail);
      }
    }
    false
  }

  /// Creates entries with intermediary contexts when needed.
  pub fn create_entries(&mut self, names: &[Name], value: Value) {
    // if there are no entry names provided, then fo nothing
    if names.is_empty() {
      return;
    }
    let key = names[0].clone();
    let tail = &names[1..];
    // if tail is empty, then insert the value under the key in current context and return
    if tail.is_empty() {
      self.put(key, value);
      return;
    }
    // if there is a context under the key, then insert value to this context (a copy of it, when
    // it is shared, so only the contexts on the path are copied) and return
    if let Some(Value::Context(ctx)) = self.entries_mut().get_mut(&key) {
      Arc::make_mut(ctx).create_entries(tail, value);
      return;
    }
    // insert a value from tail to newly created context
    let mut ctx = FeelContext::default();
    ctx.create_entries(tail, value);
    self.put(key, ctx.into());
  }

  /// Sets `value` at the path `names` like [FeelContext::create_entries], with an error when an
  /// entry on the path holds a value that is not a context; the context is then left as it was.
  ///
  /// A path that would nest the value more levels deep than values may nest within an evaluation
  /// ([bound_depth](crate::diagnostics::bound_depth)) is refused before anything is built: the
  /// running evaluation stops, and the context is left as it was. A path that is blocked builds
  /// nothing, so it is the error of the blocking entry whatever its length.
  pub fn apply_entries(&mut self, names: &[Name], value: Value) -> Result<()> {
    // the part of the path that exists: no deeper than this context, so read without recursion
    let mut ctx: &FeelContext = self;
    for name in names.iter().take(names.len().saturating_sub(1)) {
      match ctx.entries.get(name) {
        Some(Value::Context(inner)) => ctx = inner,
        Some(other) => return Err(err_value_is_not_a_context(other)),
        None => break,
      }
    }
    if u64::try_from(names.len()).unwrap_or(u64::MAX).saturating_add(u64::from(value.nesting_depth())) > u64::from(diagnostics::MAX_VALUE_DEPTH) {
      diagnostics::too_deep();
      return Err(err_value_nested_too_deeply(names.len()));
    }
    self.create_entries(names, value);
    Ok(())
  }

  /// Deep search for a value pointed by names: the entry named by the first name, then in each
  /// context found the entry named by the next one. One step per name, without recursion, so the
  /// number of names is not bounded by the host stack.
  pub fn search_deep(&self, names: &[Name]) -> Option<&Value> {
    let (first, rest) = names.split_first()?;
    let mut value = self.entries.get(first)?;
    for name in rest {
      let Value::Context(ctx) = value else {
        return None;
      };
      value = ctx.entries.get(name)?;
    }
    Some(value)
  }

  /// The nesting depth of this context: one more than that of its deepest entry.
  pub(crate) fn nesting_depth(&self) -> u32 {
    let known = self.depth.load(Ordering::Relaxed);
    if known != 0 {
      return known;
    }
    let depth = self.entries.values().map(Value::nesting_depth).max().unwrap_or(0).saturating_add(1);
    self.depth.store(depth, Ordering::Relaxed);
    depth
  }

  /// The entries, to be changed: the nesting depth is forgotten.
  fn entries_mut(&mut self) -> &mut FeelContextEntries {
    *self.depth.get_mut() = 0;
    &mut self.entries
  }

  /// Inserts an entry.
  fn put(&mut self, name: Name, value: Value) {
    self.entries_mut().insert(name, value);
  }
}
