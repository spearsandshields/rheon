//! Index of the names of an input context, used by the lexer to recognize names in scope.

use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::collections::{BTreeMap, HashSet};

/// Length of the separator `" . "` between the keys of a composite name.
const SEPARATOR_LEN: usize = 3;

/// The names that the parsing context built from an input context ([ParsingContext::from])
/// contains, indexed without copying the input context.
///
/// A name is in the parsing context when it is a key at any depth: of the context itself, of a
/// nested context, of a context type ([Value::FeelType]), or of the item type of a list of
/// contexts. Composite names (`a . b`) are not indexed, see [super::scope::ParsingScope::contains].
///
/// [ParsingContext::from]: super::context::ParsingContext
pub(crate) struct FrameIndex<'a> {
  /// The input context; its own keys are looked up in it directly.
  top: &'a FeelContext,
  /// Keys at depth one or more, borrowed from the input context.
  nested: HashSet<&'a str>,
  /// Keys of list item types; computed by [Value::type_of], so owned.
  owned: HashSet<String>,
  /// Length in bytes of the longest key at any depth.
  max_plain: usize,
  /// Length in bytes of the longest composite name `k1 . k2 . … . kn` (keys along a path from
  /// depth zero, which is the longest of the composite names of that path); at least `max_plain`.
  max_chain: usize,
}

/// A node of the input context whose keys are names in the parsing context.
enum Node<'a> {
  Context(&'a FeelContext),
  Type(&'a BTreeMap<Name, FeelType>),
}

impl<'a> FrameIndex<'a> {
  /// Indexes the names of an input context.
  ///
  /// The nodes visited are exactly those [ParsingContext::from](super::context::ParsingContext)
  /// converts into nested parsing contexts; the traversal uses an explicit stack, so a deeply
  /// nested context can not overflow the stack.
  pub(crate) fn new(top: &'a FeelContext) -> Self {
    let mut index = Self {
      top,
      nested: HashSet::new(),
      owned: HashSet::new(),
      max_plain: 0,
      max_chain: 0,
    };
    // nodes to visit, with the length of the composite name prefix `k1 . … . ki . ` leading to them
    let mut stack: Vec<(Node<'a>, usize)> = vec![];
    // list item types to visit (owned, see `owned`), with the same prefix length
    let mut item_types: Vec<(FeelType, usize)> = vec![];
    for (name, value) in top.iter() {
      let key = name.as_str();
      index.add_key_length(key.len(), 0);
      push_value(value, key.len() + SEPARATOR_LEN, &mut stack, &mut item_types);
    }
    while let Some((node, prefix)) = stack.pop() {
      match node {
        Node::Context(ctx) => {
          for (name, value) in ctx.iter() {
            let key = name.as_str();
            index.nested.insert(key);
            index.add_key_length(key.len(), prefix);
            push_value(value, prefix + key.len() + SEPARATOR_LEN, &mut stack, &mut item_types);
          }
        }
        Node::Type(entries) => {
          for (name, feel_type) in entries {
            let key = name.as_str();
            index.nested.insert(key);
            index.add_key_length(key.len(), prefix);
            if let Some(entries) = context_type_entries(feel_type) {
              stack.push((Node::Type(entries), prefix + key.len() + SEPARATOR_LEN));
            }
          }
        }
      }
    }
    for (item_type, prefix) in &item_types {
      index.add_owned_type(item_type, *prefix);
    }
    index
  }

  /// Returns the input context this index was built from.
  pub(crate) fn context(&self) -> &'a FeelContext {
    self.top
  }

  /// Returns `true` when `name` is a key at any depth.
  pub(crate) fn contains(&self, name: &str) -> bool {
    self.top.contains_key(name) || self.nested.contains(name) || self.owned.contains(name)
  }

  /// Length in bytes of the longest key at any depth.
  pub(crate) fn max_plain(&self) -> usize {
    self.max_plain
  }

  /// Length in bytes of the longest composite name.
  pub(crate) fn max_chain(&self) -> usize {
    self.max_chain
  }

  fn add_key_length(&mut self, len: usize, prefix: usize) {
    self.max_plain = self.max_plain.max(len);
    self.max_chain = self.max_chain.max(prefix + len);
  }

  /// Adds the keys of a list item type (a context type) at any depth.
  fn add_owned_type(&mut self, item_type: &FeelType, prefix: usize) {
    let Some(entries) = context_type_entries(item_type) else {
      return;
    };
    let mut stack = vec![(entries, prefix)];
    while let Some((entries, prefix)) = stack.pop() {
      for (name, feel_type) in entries {
        let key = name.as_str();
        if !self.owned.contains(key) {
          self.owned.insert(key.to_string());
        }
        self.add_key_length(key.len(), prefix);
        if let Some(entries) = context_type_entries(feel_type) {
          stack.push((entries, prefix + key.len() + SEPARATOR_LEN));
        }
      }
    }
  }
}

/// Schedules the names inside a value of an input context (see `ParsingContext::from`):
/// a context, a context type, or the item type of a list of contexts.
fn push_value<'a>(value: &'a Value, prefix: usize, stack: &mut Vec<(Node<'a>, usize)>, item_types: &mut Vec<(FeelType, usize)>) {
  match value {
    Value::Context(ctx) => stack.push((Node::Context(ctx), prefix)),
    Value::FeelType(feel_type) => {
      if let Some(entries) = context_type_entries(feel_type) {
        stack.push((Node::Type(entries), prefix));
      }
    }
    Value::List(_) => {
      if let Some(item_type) = list_item_context_type(value) {
        item_types.push((item_type, prefix));
      }
    }
    _ => {}
  }
}

/// The entries of a context type whose names are in a parsing context (`ParsingEntry::from(&FeelType)`):
/// those of a context type, or of the item type of a list of contexts.
fn context_type_entries(feel_type: &FeelType) -> Option<&BTreeMap<Name, FeelType>> {
  match feel_type {
    FeelType::Context(entries) => Some(entries),
    FeelType::List(item_type) => match item_type.as_ref() {
      FeelType::Context(entries) => Some(entries),
      _ => None,
    },
    _ => None,
  }
}

/// The item type of a list value when it is a context type: the item type of [Value::type_of] of the list.
///
/// The type of a list whose first item does not have a context type is never a list of contexts, so
/// such a list is not typed (its other items are not looked at).
fn list_item_context_type(value: &Value) -> Option<FeelType> {
  let Value::List(items) = value else {
    return None;
  };
  if !matches!(items.first()?.type_of(), FeelType::Context(_)) {
    return None;
  }
  match value.type_of() {
    FeelType::List(item_type) if matches!(*item_type, FeelType::Context(_)) => Some(*item_type),
    _ => None,
  }
}
