//! Index of the names of an input context, used by the lexer to recognize names in scope, and
//! the rule that says which names are nested in a value.

use crate::composite::{NameTree, contains_composite, nested_prefix};
use crate::scope::{Lengths, first_word, first_word_bound};
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ptr;

/// The names in scope of an input context, indexed without copying the input context: the
/// names of the [ParsingContext](crate::context::ParsingContext) converted from it.
///
/// A name is in scope when it is a key at any depth: of the context itself, or of the names
/// nested in its values ([nested_names]). Composite names (`a . b`) are not indexed but
/// searched, see [FrameIndex::contains_composite].
///
/// Neither the index nor the composite search recurses over the nesting of the input context,
/// so a deeply nested input context does not overflow the stack of a parse. Neither follows
/// every path to a context that the input context holds on several paths (a shared context
/// value, such as `{a: x, b: x}`, or a context that a shared list holds, such as the item of `l`
/// in `{a: l, b: l}`): the index reads it again only when the shared node it is held by is
/// reached with a longer composite-name prefix, and the composite search visits each state once.
/// Two steps do recurse: deciding whether the items of a list conform to its first item
/// ([list_item_node]), over the nesting of the items; and the conversion of an input context into
/// a parsing context, which the parsing scope does only for its text and to set a name in the top
/// input context when no context was pushed.
///
/// The items of a list are compared with its first item, which reads every item, only when a
/// lookup depends on it. The first index takes the names of the first item of every list as
/// names that are not decided yet (a list of one item has nothing to decide). A lookup that
/// they do not answer is answered by this index: a name it does not hold is no name in scope,
/// a name it holds without such a list on the way is one, and its lengths bound the lengths
/// of the names in scope (a longer bound admits more candidate names, and each is looked up).
/// A lookup of a name that only undecided lists hold builds the second index, which decides
/// every list, and that index answers from then on.
pub(crate) struct FrameIndex<'a> {
  /// The input context; its own keys are looked up in it directly.
  top: &'a FeelContext,
  /// The other keys and the lengths, indexed on first use, with the lists not decided: a parse
  /// that looks up no name beyond the keys of the input context itself (or none at all) does not
  /// index the input context.
  keys: OnceCell<Keys<'a>>,
  /// The keys and the lengths with every list decided, indexed when a lookup depends on a list
  /// that `keys` did not decide.
  decided: OnceCell<Keys<'a>>,
  /// The last first word asked for and its bound ([FrameIndex::group_bound]): the names of one
  /// expression often share their first word, and the input context does not change.
  group: RefCell<Option<(String, usize)>>,
}

/// The keys of an input context below its own keys, and the lengths of all keys.
struct Keys<'a> {
  /// Keys at depth one or more, borrowed from the input context, each with whether it is a name
  /// in scope for certain: `false` when only the first items of lists that are not decided
  /// hold it, or contexts below such items.
  names: HashMap<&'a str, bool>,
  /// Whether a list of two or more items was taken as a list of items of one type without
  /// comparing them. Without one, every name is certain and every node of `item_nodes` decided.
  undecided: bool,
  /// The lengths in bytes of the keys at any depth, those that are not decided included.
  lengths: Lengths,
  /// Length in bytes of the longest key of `names` that is longer than its first word ([first_word]).
  max_longer: usize,
  /// Length in bytes of the longest key at any depth.
  max_plain: usize,
  /// Length in bytes of the longest composite name `k1 . k2 . … . kn` (keys along a path from
  /// depth zero, which is the longest of the composite names of that path); at least `max_plain`.
  max_chain: usize,
  /// The nodes of the names nested in the lists of the input context ([list_item_node]), by the
  /// address of the list value (decided once, so kept for the composite-name search); with
  /// `undecided`, also the nodes of the lists that are not decided ([first_item_node]).
  item_nodes: HashMap<usize, Node<'a>>,
}

/// The names nested in a value of an input context, the rule of [nested_names].
pub(crate) enum Nested<'a> {
  /// The names of a node: a context, or a context type or a list type of a context type.
  Node(Node<'a>),
  /// A list: the names of its item type, when that is a context type ([list_item_node]).
  List(&'a Value),
}

/// Returns the kind of names a value of an input context can carry, or `None` when it can carry
/// none: a context, a context type or list type of a context type ([Value::FeelType]), or a list;
/// whether a list carries names depends on its item type ([list_item_node]).
///
/// The one rule for nested names: the index of input contexts, the composite-name search and the
/// conversion of input contexts into parsing contexts follow it.
pub(crate) fn nested_names(value: &Value) -> Option<Nested<'_>> {
  match value {
    Value::Context(ctx) => Some(Nested::Node(Node::Context(ctx))),
    Value::FeelType(feel_type) => context_type_entries(feel_type).map(|entries| Nested::Node(Node::Type(entries))),
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

/// Returns the node of the names nested in a list value, with the item it is read from: the
/// names of the item type of the list ([Value::type_of]) when it is a context type, `None`
/// otherwise.
///
/// The item type of a list is the type of its first item when the types of the other items
/// conform to it ([FeelList::items_conform_to_first](rheon_feel::values::FeelList::items_conform_to_first)),
/// so it is a context type when that holds and the type of the first item is a context type
/// ([context_type_node]). The names are then read from the first item, and no type is built: a
/// list whose first item is not a context, a context type or an interval end of one is not
/// looked at further, and the conformance of the items reads them only as far as it needs, a pair
/// with a shared list or context once.
pub(crate) fn list_item_node(value: &Value) -> Option<(Node<'_>, &Value)> {
  let Value::List(items) = value else {
    return None;
  };
  let first = items.first()?;
  let node = context_type_node(first)?;
  work!(ListsDecided, 1);
  items.items_conform_to_first().then_some((node, first))
}

/// Returns the node of the names of the first item of a list value, with the item, and whether
/// the items are of one type without a comparison (a list of one item): the node that
/// [list_item_node] returns when the items conform to the first, taken without deciding it.
fn first_item_node(value: &Value) -> Option<(Node<'_>, &Value, bool)> {
  let Value::List(items) = value else {
    return None;
  };
  let first = items.first()?;
  Some((context_type_node(first)?, first, items.len() == 1))
}

/// Returns the node of the names of the type of a value when that is a context type
/// ([Value::type_of]): a context, whose names are its keys and the names nested in its values
/// ([nested_names]); the entries of a context type held as a value; and the node of the value
/// of an interval end, whose type is that of its value. `None` for every other value.
///
/// The names nested in a context are those of its type, except below an entry that holds an
/// internal value of a type (a context type, a context type entry, a formal parameter or an
/// interval end): its type may be a context type, but such an entry carries no names in an input
/// context ([nested_names]).
fn context_type_node(value: &Value) -> Option<Node<'_>> {
  match value {
    Value::Context(ctx) => Some(Node::Context(ctx)),
    Value::FeelType(feel_type) | Value::ContextType(feel_type) | Value::ContextTypeEntry(_, feel_type) | Value::FormalParameter(_, feel_type) => match feel_type {
      FeelType::Context(entries) => Some(Node::Type(entries)),
      _ => None,
    },
    Value::IntervalStart(value, _) | Value::IntervalEnd(value, _) => context_type_node(value),
    _ => None,
  }
}

/// Returns the address of a value, the key of its node in [Keys::item_nodes].
fn address(value: &Value) -> usize {
  ptr::from_ref(value).addr()
}

/// A node of the input context whose keys are names in scope.
#[derive(Clone, Copy)]
pub(crate) enum Node<'a> {
  /// A context: its keys, and the names nested in its values.
  Context(&'a FeelContext),
  /// A context type, or a list type of a context type ([context_type_entries]): the names of the
  /// entries, and the names nested in their types.
  Type(&'a BTreeMap<Name, FeelType>),
}

/// A node of an indexed input context, with the nodes of the lists the index decided: a node of
/// the tree of names that [contains_composite] searches.
#[derive(Clone, Copy)]
struct InputNode<'s> {
  node: Node<'s>,
  item_nodes: &'s HashMap<usize, Node<'s>>,
}

impl<'s> NameTree for InputNode<'s> {
  fn contains_key(self, key: &str) -> bool {
    match self.node {
      Node::Context(ctx) => ctx.contains_entry(key),
      Node::Type(entries) => entries.contains_key(key),
    }
  }

  /// The nested nodes are those of [nested_names], with the nodes of lists decided by [Keys::new].
  fn for_each_nested(self, mut f: impl FnMut(&str, Self)) {
    let nested = |node| InputNode {
      node,
      item_nodes: self.item_nodes,
    };
    match self.node {
      Node::Context(ctx) => {
        for (name, value) in ctx.iter() {
          let node = match nested_names(value) {
            Some(Nested::Node(node)) => Some(node),
            Some(Nested::List(list)) => self.item_nodes.get(&address(list)).copied(),
            None => None,
          };
          if let Some(node) = node {
            f(name.as_str(), nested(node));
          }
        }
      }
      Node::Type(entries) => {
        for (name, feel_type) in entries {
          if let Some(entries) = context_type_entries(feel_type) {
            f(name.as_str(), nested(Node::Type(entries)));
          }
        }
      }
    }
  }

  /// The address of the node: a context held on several paths has one address on all of them,
  /// and a state of the search depends only on the node and the offset, so a state is decided
  /// once whichever path reaches it.
  fn address(self) -> usize {
    match self.node {
      Node::Context(ctx) => ptr::from_ref(ctx).addr(),
      Node::Type(entries) => ptr::from_ref(entries).addr(),
    }
  }
}

impl<'a> FrameIndex<'a> {
  /// Creates the index of the names of an input context (indexed on first use).
  pub(crate) fn new(top: &'a FeelContext) -> Self {
    Self {
      top,
      keys: OnceCell::new(),
      decided: OnceCell::new(),
      group: RefCell::new(None),
    }
  }

  /// Returns the input context this index was built from.
  pub(crate) fn context(&self) -> &'a FeelContext {
    self.top
  }

  /// Returns `true` when `name` is a key of the input context itself.
  pub(crate) fn contains_own(&self, name: &str) -> bool {
    self.top.contains_entry(name)
  }

  /// Returns `true` when `name` is a key at any depth.
  pub(crate) fn contains(&self, name: &str) -> bool {
    if self.top.contains_entry(name) {
      return true;
    }
    match self.keys().names.get(name) {
      None => false,
      Some(true) => true,
      // only lists that are not decided hold the name
      Some(false) => self.decided().names.contains_key(name),
    }
  }

  /// Length in bytes of the longest key at any depth.
  pub(crate) fn max_plain(&self) -> usize {
    self.keys().max_plain
  }

  /// Returns `false` only when no key at any depth has `len` bytes.
  pub(crate) fn has_length(&self, len: usize) -> bool {
    self.keys().lengths.contains(len)
  }

  /// Length in bytes of the longest composite name.
  pub(crate) fn max_chain(&self) -> usize {
    self.keys().max_chain
  }

  /// Upper bound of the length in bytes of the keys of the input context itself whose first word
  /// is `word` (see [first_word]).
  pub(crate) fn group_bound(&self, word: &str) -> usize {
    let mut group = self.group.borrow_mut();
    if let Some((last, bound)) = &*group
      && last == word
    {
      return *bound;
    }
    let bound = first_word_bound(self.top.keys_from(word), word, self.max_plain());
    match &mut *group {
      Some((last, last_bound)) => {
        last.clear();
        last.push_str(word);
        *last_bound = bound;
      }
      None => *group = Some((word.to_string(), bound)),
    }
    bound
  }

  /// Length in bytes of the longest key at depth one or more (or of a list item type) that is
  /// longer than its first word: with `word` itself, a bound of the deeper keys whose first word is `word`.
  pub(crate) fn max_longer(&self) -> usize {
    self.keys().max_longer
  }

  /// Returns `true` when `name` is a flattened key of the input context, a composite name
  /// `k1 . k2 . … . kn` included: [contains_composite] over the nodes of the input context,
  /// without a converted copy.
  pub(crate) fn contains_composite(&self, name: &str) -> bool {
    let found_in = |keys: &Keys<'a>| {
      let root = InputNode {
        node: Node::Context(self.top),
        item_nodes: &keys.item_nodes,
      };
      contains_composite(root, name)
    };
    // the nodes of the lists that are not decided hold every composite name and more: a name
    // found there may lead through such a list
    let keys = self.keys();
    found_in(keys) && (!keys.undecided || found_in(self.decided()))
  }

  /// The index that answers a lookup: the one with every list decided once it exists,
  /// otherwise the one that decides none.
  fn keys(&self) -> &Keys<'a> {
    self.decided.get().unwrap_or_else(|| self.keys.get_or_init(|| Keys::new(self.top, false)))
  }

  /// The index with every list decided: the first index when it left no list undecided.
  fn decided(&self) -> &Keys<'a> {
    let keys = self.keys();
    if keys.undecided { self.decided.get_or_init(|| Keys::new(self.top, true)) } else { keys }
  }
}

impl<'a> Keys<'a> {
  /// Indexes the keys of an input context: the keys of the nodes of the names nested in its
  /// values ([nested_names]), at any depth.
  ///
  /// With `decide`, the names of a list are those of its item type ([list_item_node]), which
  /// compares its items. Without, they are the names of its first item ([first_item_node]),
  /// and they and the names below them are not certain unless the list has one item.
  ///
  /// The traversal uses an explicit stack, so a deeply nested input context does not overflow the
  /// stack. Deciding whether the items of a list conform to its first item ([list_item_node]) is
  /// the exception: it recurses over the nesting of the items.
  ///
  /// A shared context, and the context a shared list reads its names from, is read again only
  /// when the shared node is reached with a longer composite-name prefix than before: its keys
  /// are the same on every path, so with a prefix no longer it adds no key and no longer
  /// composite name, and neither do the nodes below it, whose prefixes are shorter then too. A
  /// node held once is reached on several paths only through a shared node above it, so every
  /// other context is read as often as a shared node above it is read.
  fn new(top: &'a FeelContext, decide: bool) -> Self {
    work!(IndexBuilds, 1);
    let mut index = Self {
      names: HashMap::new(),
      undecided: false,
      lengths: Lengths::default(),
      max_longer: 0,
      max_plain: 0,
      max_chain: 0,
      item_nodes: HashMap::new(),
    };
    // nodes to visit, with the length of the composite name prefix `k1 . … . ki . ` leading to
    // them, and whether their names are certain
    let mut stack: Vec<(Node<'a>, usize, bool)> = vec![];
    // the longest prefix each shared context was scheduled with, by its address
    let mut scheduled = Scheduled::default();
    for (name, value) in top.iter() {
      let key = name.as_str();
      index.add_key_length(key.len(), 0);
      index.push_value(value, nested_prefix(0, key), true, decide, &mut stack, &mut scheduled);
    }
    while let Some((node, prefix, certain)) = stack.pop() {
      match node {
        Node::Context(ctx) => {
          work!(IndexContexts, 1);
          for (name, value) in ctx.iter() {
            let key = name.as_str();
            index.add_name(key, certain);
            index.add_nested_key_length(key, prefix);
            index.push_value(value, nested_prefix(prefix, key), certain, decide, &mut stack, &mut scheduled);
          }
        }
        Node::Type(entries) => index.add_type_entries(entries, prefix, certain),
      }
    }
    index
  }

  /// Schedules the node of the names nested in a value of the input context ([nested_names]),
  /// with the prefix `prefix` of the composite names into it; the node of a list
  /// ([list_item_node]) is kept for the composite-name search. The node is admitted by the
  /// shared node that holds it ([Scheduled::admits]): a shared list for the item it is read from,
  /// which the list may hold alone, and otherwise the value that holds the names.
  ///
  /// `certain` says whether the names of the value are certain; with `decide`, a list is decided,
  /// and otherwise its names are not certain unless it has one item (see [Keys::new]).
  fn push_value(&mut self, value: &'a Value, prefix: usize, certain: bool, decide: bool, stack: &mut Vec<(Node<'a>, usize, bool)>, scheduled: &mut Scheduled) {
    let (node, holder, certain) = match nested_names(value) {
      Some(Nested::Node(node)) => (node, value, certain),
      Some(Nested::List(list)) => {
        let (node, item, of_one_type) = if decide {
          let Some((node, item)) = list_item_node(list) else {
            return;
          };
          (node, item, true)
        } else {
          let Some(found) = first_item_node(list) else {
            return;
          };
          found
        };
        self.undecided |= !of_one_type;
        self.item_nodes.insert(address(list), node);
        (node, if list.is_shared() { list } else { item }, certain && of_one_type)
      }
      None => return,
    };
    if scheduled.admits(holder, prefix, certain) {
      stack.push((node, prefix, certain));
    }
  }

  /// Adds a key at depth one or more; a key that is certain on one path is certain.
  fn add_name(&mut self, key: &'a str, certain: bool) {
    *self.names.entry(key).or_insert(certain) |= certain;
  }

  /// Adds the keys of the entries of a context type, and of the entries nested in their types,
  /// at any depth.
  fn add_type_entries(&mut self, entries: &'a BTreeMap<Name, FeelType>, prefix: usize, certain: bool) {
    let mut stack = vec![(entries, prefix)];
    while let Some((entries, prefix)) = stack.pop() {
      for (key_name, feel_type) in entries {
        let key = key_name.as_str();
        self.add_name(key, certain);
        self.add_nested_key_length(key, prefix);
        if let Some(entries) = context_type_entries(feel_type) {
          stack.push((entries, nested_prefix(prefix, key)));
        }
      }
    }
  }

  fn add_key_length(&mut self, len: usize, prefix: usize) {
    self.lengths.insert(len);
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

/// The longest composite-name prefixes each shared list or context of an input context was
/// scheduled with ([Keys::new]), by its node address ([Value::node_address]): the longest of
/// all, and the longest with its names certain. Created when the first shared node is met.
#[derive(Default)]
struct Scheduled(Option<HashMap<usize, (usize, usize)>>);

impl Scheduled {
  /// Whether the node of the names held by `value` is to be read with `prefix`: always for a
  /// value that is not shared ([Value::is_shared]), and for a shared list or context when it was
  /// not scheduled with this prefix or a longer one; a read with the names certain only counts
  /// for a later one with the names certain when it was certain itself.
  fn admits(&mut self, value: &Value, prefix: usize, certain: bool) -> bool {
    let Some(address) = value.node_address().filter(|_| value.is_shared()) else {
      return true;
    };
    let (longest, longest_certain) = self.0.get_or_insert_with(HashMap::new).entry(address).or_insert((0, 0));
    let covering = if certain { *longest_certain } else { *longest };
    if covering != 0 && covering >= prefix {
      return false;
    }
    *longest = (*longest).max(prefix);
    if certain {
      *longest_certain = prefix;
    }
    true
  }
}
