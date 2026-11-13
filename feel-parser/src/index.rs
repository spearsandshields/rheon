//! Index of the names of an input context, used by the lexer to recognize names in scope, and
//! the rule that says which names are nested in a value.

use crate::scope::{first_word, first_word_bound};
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ptr;

/// Separator of the keys in a composite name.
const SEPARATOR: &str = " . ";

/// Length of the separator `" . "` between the keys of a composite name.
const SEPARATOR_LEN: usize = SEPARATOR.len();

/// The names in scope of an input context, indexed without copying the input context: the
/// names of the [ParsingContext](crate::context::ParsingContext) converted from it.
///
/// A name is in scope when it is a key at any depth: of the context itself, or of the names
/// nested in its values ([nested_names]). Composite names (`a . b`) are not indexed but
/// searched, see [FrameIndex::contains_composite].
pub(crate) struct FrameIndex<'a> {
  /// The input context; its own keys are looked up in it directly.
  top: &'a FeelContext,
  /// The other keys and the lengths, indexed on first use: a parse that looks up no name beyond
  /// the keys of the input context itself (or none at all) does not index the input context.
  keys: OnceCell<Keys<'a>>,
}

/// The keys of an input context below its own keys, and the lengths of all keys.
struct Keys<'a> {
  /// Keys at depth one or more: borrowed from the input context, or owned for the keys of the
  /// item types of lists, which are computed.
  names: HashSet<Cow<'a, str>>,
  /// Length in bytes of the longest key of `names` that is longer than its first word ([first_word]).
  max_longer: usize,
  /// Length in bytes of the longest key at any depth.
  max_plain: usize,
  /// Length in bytes of the longest composite name `k1 . k2 . … . kn` (keys along a path from
  /// depth zero, which is the longest of the composite names of that path); at least `max_plain`.
  max_chain: usize,
  /// The item types of the lists of contexts, by the address of the list in the input context
  /// (computed, so kept for [FrameIndex::contains_composite]).
  item_types: HashMap<usize, FeelType>,
}

/// The names nested in a value of an input context, the rule of [nested_names].
pub(crate) enum Nested<'a> {
  /// A context: its keys, and the names nested in its values.
  Context(&'a FeelContext),
  /// A context type, or a list type of a context type ([context_type_entries]): the names of the
  /// entries, and the names nested in their types.
  Type(&'a BTreeMap<Name, FeelType>),
  /// A list: the names of the entries of its item type, when that is a context type
  /// ([list_item_context_type]), and the names nested in their types.
  List(&'a Value),
}

/// Returns the names nested in a value of an input context, or `None` when it has none: those
/// of a context, of a context type or list of context type ([Value::FeelType]), and of a list
/// whose item type is a context type.
///
/// The one rule for nested names: the index of input contexts, the composite-name search and the
/// conversion of input contexts into parsing contexts follow it.
pub(crate) fn nested_names(value: &Value) -> Option<Nested<'_>> {
  match value {
    Value::Context(ctx) => Some(Nested::Context(ctx)),
    Value::FeelType(feel_type) => context_type_entries(feel_type).map(Nested::Type),
    Value::List(_) => Some(Nested::List(value)),
    _ => None,
  }
}

/// Returns the entries of a type whose names are nested in it: those of a context type, or of
/// the item type of a list type of a context type.
pub(crate) fn context_type_entries(feel_type: &FeelType) -> Option<&BTreeMap<Name, FeelType>> {
  match feel_type {
    FeelType::Context(entries) => Some(entries),
    FeelType::List(item_type) => match item_type.as_ref() {
      FeelType::Context(entries) => Some(entries),
      _ => None,
    },
    _ => None,
  }
}

/// Returns the item type of a list value when it is a context type: the item type of
/// [Value::type_of] of the list.
///
/// The type of a list whose first item does not have a context type is never a list of contexts, so
/// such a list is not typed (its other items are not looked at). Typing the items recurses over
/// their nesting, as [Value::type_of] does.
pub(crate) fn list_item_context_type(value: &Value) -> Option<FeelType> {
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

/// Returns the address of a value, the key of its item type in [Keys::item_types].
fn address(value: &Value) -> usize {
  ptr::from_ref(value).addr()
}

/// A node of the input context whose keys are names in scope.
#[derive(Clone, Copy)]
enum Node<'a> {
  Context(&'a FeelContext),
  Type(&'a BTreeMap<Name, FeelType>),
}

impl Node<'_> {
  /// Returns the address of the node: input nodes are never shared, so the address identifies the node.
  fn address(&self) -> usize {
    match self {
      Node::Context(ctx) => ptr::from_ref(*ctx).addr(),
      Node::Type(entries) => ptr::from_ref(*entries).addr(),
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
    self.top.contains_key(name) || self.keys().names.contains(name)
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

  /// Returns `true` when `name` is a composite name `k1 . k2 . … . kn` of the input context: the
  /// search of [ParsingContext::contains_flattened](crate::context::ParsingContext::contains_flattened)
  /// over the nodes of the input context instead of a converted copy, with an explicit stack, so
  /// that a deeply nested input context does not overflow the stack.
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
  /// Indexes the keys of an input context: the keys of the nodes of the names nested in its
  /// values ([nested_names]), at any depth.
  ///
  /// The traversal uses an explicit stack, so a deeply nested input context does not overflow the
  /// stack. Typing the items of a list ([list_item_context_type]) is the exception: it recurses
  /// over the nesting of the items, as [Value::type_of] does.
  fn new(top: &'a FeelContext) -> Self {
    work!(IndexBuilds, 1);
    let mut index = Self {
      names: HashSet::new(),
      max_longer: 0,
      max_plain: 0,
      max_chain: 0,
      item_types: HashMap::new(),
    };
    // nodes to visit, with the length of the composite name prefix `k1 . … . ki . ` leading to them
    let mut stack: Vec<(Node<'a>, usize)> = vec![];
    // list item types to visit (computed, so their keys are owned), with the address of the list
    // and the same prefix length
    let mut item_types: Vec<(usize, FeelType, usize)> = vec![];
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
            index.names.insert(Cow::Borrowed(key));
            index.add_nested_key_length(key, prefix);
            push_value(value, prefix + key.len() + SEPARATOR_LEN, &mut stack, &mut item_types);
          }
        }
        Node::Type(entries) => index.add_type_entries(entries, prefix, Cow::Borrowed),
      }
    }
    for (list, item_type, prefix) in item_types {
      if let Some(entries) = context_type_entries(&item_type) {
        index.add_type_entries(entries, prefix, |key| Cow::Owned(key.to_owned()));
      }
      index.item_types.insert(list, item_type);
    }
    index
  }

  /// Adds the keys of the entries of a context type, and of the entries nested in their types,
  /// at any depth; `name` turns a key into an element of [Keys::names].
  fn add_type_entries<'e>(&mut self, entries: &'e BTreeMap<Name, FeelType>, prefix: usize, name: impl Fn(&'e str) -> Cow<'a, str>) {
    let mut stack = vec![(entries, prefix)];
    while let Some((entries, prefix)) = stack.pop() {
      for (key_name, feel_type) in entries {
        let key = key_name.as_str();
        if !self.names.contains(key) {
          self.names.insert(name(key));
        }
        self.add_nested_key_length(key, prefix);
        if let Some(entries) = context_type_entries(feel_type) {
          stack.push((entries, prefix + key.len() + SEPARATOR_LEN));
        }
      }
    }
  }

  /// Calls `f` with the key and the node of every key of a node whose value has nested names
  /// ([nested_names]; the item types of lists are those computed by [Keys::new]).
  fn for_each_nested<'s>(&'s self, node: Node<'s>, mut f: impl FnMut(&'s str, Node<'s>)) {
    match node {
      Node::Context(ctx) => {
        for (name, value) in ctx.iter() {
          let nested = match nested_names(value) {
            Some(Nested::Context(ctx)) => Some(Node::Context(ctx)),
            Some(Nested::Type(entries)) => Some(Node::Type(entries)),
            Some(Nested::List(list)) => self.item_types.get(&address(list)).and_then(context_type_entries).map(Node::Type),
            None => None,
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
}

/// Schedules the nodes of the names nested in a value of an input context ([nested_names]): a
/// context or the entries of a context type are visited, the item type of a list is computed.
fn push_value<'a>(value: &'a Value, prefix: usize, stack: &mut Vec<(Node<'a>, usize)>, item_types: &mut Vec<(usize, FeelType, usize)>) {
  match nested_names(value) {
    Some(Nested::Context(ctx)) => stack.push((Node::Context(ctx), prefix)),
    Some(Nested::Type(entries)) => stack.push((Node::Type(entries), prefix)),
    Some(Nested::List(list)) => {
      if let Some(item_type) = list_item_context_type(list) {
        item_types.push((address(list), item_type, prefix));
      }
    }
    None => {}
  }
}
