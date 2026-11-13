//! Implementation of the parsing context.

use crate::index::{Nested, context_type_entries, list_item_context_type, nested_names};
use crate::scope::first_word_bound;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::collections::{BTreeMap, HashSet};
use std::fmt;

/// Separator of the keys in a composite key (see [ParsingContext::contains_flattened]).
const COMPOSITE_SEPARATOR: &str = " . ";

/// Types of entries in parsing context.
pub enum ParsingEntry {
  /// Parsing entry representing a context.
  Context(ParsingContext),
  /// Parsing entry representing a variable.
  Variable,
}

impl fmt::Display for ParsingEntry {
  /// Converts parsing entry into text representation.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        ParsingEntry::Context(ctx) => ctx.to_string(),
        ParsingEntry::Variable => "<v>".to_string(),
      }
    )
  }
}

impl From<FeelType> for ParsingEntry {
  /// Converts a reference to feel type into parsing entry.
  fn from(value: FeelType) -> Self {
    Self::from(&value)
  }
}

impl From<&FeelType> for ParsingEntry {
  /// Converts a feel type into parsing entry: a context of the entries of a context type or of
  /// a list type of a context type ([context_type_entries]), else a variable.
  fn from(value: &FeelType) -> Self {
    match context_type_entries(value) {
      Some(entries) => ParsingEntry::Context(ParsingContext::from_type_entries(entries)),
      None => ParsingEntry::Variable,
    }
  }
}

impl ParsingEntry {
  /// Converts a value of an input context into parsing entry: a context of the names nested in
  /// the value ([nested_names]), else a variable.
  fn from_value(value: &Value) -> Self {
    match nested_names(value) {
      Some(Nested::Context(ctx)) => ParsingEntry::Context(ctx.into()),
      Some(Nested::Type(entries)) => ParsingEntry::Context(ParsingContext::from_type_entries(entries)),
      Some(Nested::List(list)) => list_item_context_type(list).map_or(ParsingEntry::Variable, |item_type| ParsingEntry::from(&item_type)),
      None => ParsingEntry::Variable,
    }
  }
}

/// Parsing context.
#[derive(Default)]
pub struct ParsingContext(BTreeMap<Name, ParsingEntry>);

impl fmt::Display for ParsingContext {
  /// Converts parsing context into text representation.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{{{}}}",
      self.0.iter().map(|(name, entry)| { format!(r#"{name}: {entry}"#) }).collect::<Vec<String>>().join(", ")
    )
  }
}

impl From<FeelContext> for ParsingContext {
  /// Converts feel context into parsing context.
  fn from(ctx: FeelContext) -> Self {
    Self::from(&ctx)
  }
}

impl From<&FeelContext> for ParsingContext {
  /// Converts a reference to feel context into parsing context.
  fn from(value: &FeelContext) -> Self {
    Self(value.iter().map(|(name, value)| (name.to_owned(), ParsingEntry::from_value(value))).collect())
  }
}

impl ParsingContext {
  /// Converts the entries of a context type into parsing context.
  fn from_type_entries(entries: &BTreeMap<Name, FeelType>) -> Self {
    Self(entries.iter().map(|(name, feel_type)| (name.to_owned(), feel_type.into())).collect())
  }

  /// Places a specified name in this parsing context.
  pub fn set_name(&mut self, name: Name) {
    self.0.insert(name, ParsingEntry::Variable);
  }

  /// Places parsing context under specified name.
  pub fn set_context(&mut self, name: Name, ctx: ParsingContext) {
    self.0.insert(name, ParsingEntry::Context(ctx));
  }

  /// Returns `true` when `name` is a key of this context or of a nested context, at any depth.
  pub fn contains_key_deep(&self, name: &str) -> bool {
    // the nested contexts to visit (no allocation for a context without nested contexts)
    let mut stack = vec![];
    let mut ctx = self;
    loop {
      if ctx.0.contains_key(name) {
        return true;
      }
      stack.extend(
        ctx
          .0
          .values()
          .filter_map(|entry| if let ParsingEntry::Context(sub_ctx) = entry { Some(sub_ctx) } else { None }),
      );
      match stack.pop() {
        Some(sub_ctx) => ctx = sub_ctx,
        None => return false,
      }
    }
  }

  /// Returns `true` when `name` is one of the flattened keys of this context: a key at any depth,
  /// or a composite key `k1 . k2 . … . kn` where every `k(i+1)` is a key at any depth below `ki`.
  ///
  /// The set of flattened keys grows exponentially with the nesting depth, so it is not built:
  /// this is a search over the states (context, offset in `name`), each visited at most once.
  /// In state (C, i), `name[i..]` is a flattened key of C when it is a key of C; or a flattened key
  /// of a nested context D of C; or `k . rest` for a key `k` of C whose value is D and `rest` is
  /// a flattened key of D.
  pub fn contains_flattened(&self, name: &str) -> bool {
    let mut visited: HashSet<(*const ParsingContext, usize)> = HashSet::new();
    let mut stack = vec![(self, 0_usize)];
    while let Some((ctx, offset)) = stack.pop() {
      if !visited.insert((ctx as *const ParsingContext, offset)) {
        continue;
      }
      let rest = &name[offset..];
      if ctx.0.contains_key(rest) {
        return true;
      }
      for (key, entry) in &ctx.0 {
        if let ParsingEntry::Context(sub_ctx) = entry {
          stack.push((sub_ctx, offset));
          if rest.strip_prefix(key.as_str()).is_some_and(|tail| tail.starts_with(COMPOSITE_SEPARATOR)) {
            stack.push((sub_ctx, offset + key.as_str().len() + COMPOSITE_SEPARATOR.len()));
          }
        }
      }
    }
    false
  }

  /// Returns the length in bytes of the longest key at any depth, and of the longest flattened key.
  pub fn key_lengths(&self) -> (usize, usize) {
    let (mut max_plain, mut max_flattened) = (0, 0);
    // contexts to visit, with the length of the composite key prefix `k1 . … . ki . ` leading to them
    let mut stack = vec![(self, 0_usize)];
    while let Some((ctx, prefix)) = stack.pop() {
      for (key, entry) in &ctx.0 {
        let len = key.as_str().len();
        max_plain = max_plain.max(len);
        max_flattened = max_flattened.max(prefix + len);
        if let ParsingEntry::Context(sub_ctx) = entry {
          stack.push((sub_ctx, prefix + len + COMPOSITE_SEPARATOR.len()));
        }
      }
    }
    (max_plain, max_flattened)
  }

  /// Returns `true` when this context has no entries.
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Returns `true` when this context has a nested context.
  pub(crate) fn has_nested(&self) -> bool {
    self.0.values().any(|entry| matches!(entry, ParsingEntry::Context(_)))
  }

  /// Returns an upper bound of the length of the keys of this context (not of the nested ones)
  /// whose first word is `word`, see [first_word_bound].
  pub(crate) fn group_bound(&self, word: &str, fallback: usize) -> usize {
    first_word_bound(&self.0, word, fallback)
  }
}
