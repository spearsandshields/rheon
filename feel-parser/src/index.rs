//! Index of the names of an input context, used by the lexer to recognize names in scope.

use crate::scope::{first_word, first_word_bound};
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::cell::OnceCell;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Separator of the keys in a composite name.
const SEPARATOR: &str = " . ";

/// Length of the separator `" . "` between the keys of a composite name.
const SEPARATOR_LEN: usize = SEPARATOR.len();

/// The names that the parsing context built from an input context ([ParsingContext::from])
/// contains, indexed without copying the input context.
///
/// A name is in the parsing context when it is a key at any depth: of the context itself, of a
/// nested context, of a context type ([Value::FeelType]), or of the item type of a list of
/// contexts. Composite names (`a . b`) are not indexed but searched, see [FrameIndex::contains_composite].
///
/// [ParsingContext::from]: crate::context::ParsingContext
pub(crate) struct FrameIndex<'a> {
  /// The input context; its own keys are looked up in it directly.
  top: &'a FeelContext,
  /// The other keys and the lengths, indexed on first use: a parse that looks up no name beyond
  /// the keys of the input context itself (or none at all) does not index the input context.
  keys: OnceCell<Keys<'a>>,
}

/// The keys of an input context below its own keys, and the lengths of all keys.
struct Keys<'a> {
  /// Keys at depth one or more, borrowed from the input context.
  nested: HashSet<&'a str>,
  /// Keys of list item types; computed by [Value::type_of], so owned.
  owned: HashSet<String>,
  /// Length in bytes of the longest key of `nested` or `owned` that is longer than its first word ([first_word]).
  max_longer: usize,
  /// Length in bytes of the longest key at any depth.
  max_plain: usize,
  /// Length in bytes of the longest composite name `k1 . k2 . … . kn` (keys along a path from
  /// depth zero, which is the longest of the composite names of that path); at least `max_plain`.
  max_chain: usize,
  /// The item types of the lists of contexts, by the address of the list in the input context
  /// (computed, so kept for [FrameIndex::contains_composite]).
  item_types: HashMap<*const Value, FeelType>,
}

/// A node of the input context whose keys are names in the parsing context.
#[derive(Clone, Copy)]
enum Node<'a> {
  Context(&'a FeelContext),
  Type(&'a BTreeMap<Name, FeelType>),
}

impl Node<'_> {
  /// Returns the address of the node (a node of the input context is reached only by one path).
  fn address(&self) -> *const () {
    match self {
      Node::Context(ctx) => *ctx as *const FeelContext as *const (),
      Node::Type(entries) => *entries as *const BTreeMap<Name, FeelType> as *const (),
    }
  }

  /// Returns `true` when `key` is a key of the node itself.
  fn contains_key(&self, key: &str) -> bool {
    match self {
      Node::Context(ctx) => ctx.contains_key(key),
      Node::Type(entries) => entries.contains_key(key),
    }
  }
}

impl<'a> FrameIndex<'a> {
  /// Creates the index of the names of an input context (indexed on first use).
  pub(crate) fn new(top: &'a FeelContext) -> Self {
    Self { top, keys: OnceCell::new() }
  }

  /// Returns the input context this index was built from.
  pub(crate) fn context(&self) -> &'a FeelContext {
    self.top
  }

  /// Returns `true` when `name` is a key at any depth.
  pub(crate) fn contains(&self, name: &str) -> bool {
    self.top.contains_key(name) || {
      let keys = self.keys();
      keys.nested.contains(name) || keys.owned.contains(name)
    }
  }

  /// Length in bytes of the longest key at any depth.
  pub(crate) fn max_plain(&self) -> usize {
    self.keys().max_plain
  }

  /// Length in bytes of the longest composite name.
  pub(crate) fn max_chain(&self) -> usize {
    self.keys().max_chain
  }

  /// Upper bound of the length in bytes of the keys of the input context itself whose first word
  /// is `word` (see [first_word]).
  pub(crate) fn group_bound(&self, word: &str) -> usize {
    first_word_bound(self.top, word, self.max_plain())
  }

  /// Length in bytes of the longest key at depth one or more (or of a list item type) that is
  /// longer than its first word: with `word` itself, a bound of the deeper keys whose first word is `word`.
  pub(crate) fn max_longer(&self) -> usize {
    self.keys().max_longer
  }

  /// Returns `true` when `name` is a composite name `k1 . k2 . … . kn` of the parsing context of
  /// the input context: the search of [ParsingContext::contains_flattened](crate::context::ParsingContext::contains_flattened)
  /// over the nodes of the input context instead of a converted copy, with an explicit stack.
  pub(crate) fn contains_composite(&self, name: &str) -> bool {
    let keys = self.keys();
    let mut visited = HashSet::new();
    let mut stack = vec![(Node::Context(self.top), 0)];
    while let Some((node, offset)) = stack.pop() {
      if !visited.insert((node.address(), offset)) {
        continue;
      }
      let rest = &name[offset..];
      if node.contains_key(rest) {
        return true;
      }
      keys.for_each_nested(node, |key, nested| {
        stack.push((nested, offset));
        if rest.strip_prefix(key).is_some_and(|tail| tail.starts_with(SEPARATOR)) {
          stack.push((nested, offset + key.len() + SEPARATOR_LEN));
        }
      });
    }
    false
  }

  fn keys(&self) -> &Keys<'a> {
    self.keys.get_or_init(|| Keys::new(self.top))
  }
}

impl<'a> Keys<'a> {
  /// Indexes the keys of an input context.
  ///
  /// The nodes visited are exactly those [ParsingContext::from](crate::context::ParsingContext)
  /// converts into nested parsing contexts; the traversal uses an explicit stack, so a deeply
  /// nested context can not overflow the stack.
  fn new(top: &'a FeelContext) -> Self {
    work!(IndexBuilds, 1);
    let mut index = Self {
      nested: HashSet::new(),
      owned: HashSet::new(),
      max_longer: 0,
      max_plain: 0,
      max_chain: 0,
      item_types: HashMap::new(),
    };
    // nodes to visit, with the length of the composite name prefix `k1 . … . ki . ` leading to them
    let mut stack: Vec<(Node<'a>, usize)> = vec![];
    // list item types to visit (owned, see `owned`), with the address of the list and the same prefix length
    let mut item_types: Vec<(*const Value, FeelType, usize)> = vec![];
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
            index.add_nested_key_length(key, prefix);
            push_value(value, prefix + key.len() + SEPARATOR_LEN, &mut stack, &mut item_types);
          }
        }
        Node::Type(entries) => {
          for (name, feel_type) in entries {
            let key = name.as_str();
            index.nested.insert(key);
            index.add_nested_key_length(key, prefix);
            if let Some(entries) = context_type_entries(feel_type) {
              stack.push((Node::Type(entries), prefix + key.len() + SEPARATOR_LEN));
            }
          }
        }
      }
    }
    for (list, item_type, prefix) in item_types {
      index.add_owned_type(&item_type, prefix);
      index.item_types.insert(list, item_type);
    }
    index
  }

  /// Calls `f` with the key and the node of every nested parsing context of a node (the values
  /// that `ParsingContext::from` converts into nested parsing contexts).
  fn for_each_nested<'s>(&'s self, node: Node<'s>, mut f: impl FnMut(&'s str, Node<'s>)) {
    match node {
      Node::Context(ctx) => {
        for (name, value) in ctx.iter() {
          let nested = match value {
            Value::Context(ctx) => Some(Node::Context(ctx)),
            Value::FeelType(feel_type) => context_type_entries(feel_type).map(Node::Type),
            Value::List(_) => self.item_types.get(&(value as *const Value)).and_then(context_type_entries).map(Node::Type),
            _ => None,
          };
          if let Some(nested) = nested {
            f(name.as_str(), nested);
          }
        }
      }
      Node::Type(entries) => {
        for (name, feel_type) in entries {
          if let Some(entries) = context_type_entries(feel_type) {
            f(name.as_str(), Node::Type(entries));
          }
        }
      }
    }
  }

  fn add_key_length(&mut self, len: usize, prefix: usize) {
    self.max_plain = self.max_plain.max(len);
    self.max_chain = self.max_chain.max(prefix + len);
  }

  fn add_nested_key_length(&mut self, key: &str, prefix: usize) {
    self.add_key_length(key.len(), prefix);
    if key.len() > self.max_longer && first_word(key).len() < key.len() {
      self.max_longer = key.len();
    }
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
        self.add_nested_key_length(key, prefix);
        if let Some(entries) = context_type_entries(feel_type) {
          stack.push((entries, prefix + key.len() + SEPARATOR_LEN));
        }
      }
    }
  }
}

/// Schedules the names inside a value of an input context (see `ParsingContext::from`):
/// a context, a context type, or the item type of a list of contexts.
fn push_value<'a>(value: &'a Value, prefix: usize, stack: &mut Vec<(Node<'a>, usize)>, item_types: &mut Vec<(*const Value, FeelType, usize)>) {
  match value {
    Value::Context(ctx) => stack.push((Node::Context(ctx), prefix)),
    Value::FeelType(feel_type) => {
      if let Some(entries) = context_type_entries(feel_type) {
        stack.push((Node::Type(entries), prefix));
      }
    }
    Value::List(items) => {
      if let Some(item_type) = list_item_context_type(items) {
        item_types.push((value, item_type, prefix));
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

/// The item type of a list when it is a context type, as `Value::List(items).type_of()` computes it:
/// the type of the first item, when every other item conforms to it.
///
/// Without a context type as the first item's type the list type is never a list of contexts, so the
/// other items are not looked at.
fn list_item_context_type(items: &[Value]) -> Option<FeelType> {
  let first = items.first()?;
  let item_type = first.type_of();
  if !matches!(item_type, FeelType::Context(_)) {
    return None;
  }
  if items[1..].iter().all(|item| item.type_is_conformant(&item_type)) {
    Some(item_type)
  } else {
    None
  }
}
