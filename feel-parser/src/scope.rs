//! Implementation of the scope used while parsing FEEL expressions.

use crate::context::ParsingContext;
use crate::index::FrameIndex;
use rheon_feel::Name;
use rheon_feel::context::FeelContext;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;

/// Parsing scope: the names the lexer recognizes as names in scope.
///
/// The scope is a stack of contexts: at the bottom the input contexts (the frames of the
/// [FeelScope](rheon_feel::FeelScope) the expression is parsed in), above them the contexts
/// pushed while parsing (function parameters, iteration variables). A name is in scope when it
/// is a key at any depth of a context on the stack, a composite key `a . b` of such a context
/// ([ParsingContext::contains_flattened]), or an already parsed name (context entry keys).
///
/// The input contexts are borrowed and indexed ([FrameIndex]), never copied.
pub struct ParsingScope<'a> {
  /// Indexes of the input contexts, bottom first.
  input: Vec<FrameIndex<'a>>,
  /// Number of input contexts still on the stack (popping more contexts than were pushed pops
  /// input contexts, from the top).
  input_live: Cell<usize>,
  /// Contexts pushed while parsing, above the input contexts.
  locals: RefCell<Vec<ParsingContext>>,
  /// Set of parsed names.
  names: RefCell<HashSet<Name>>,
  /// Upper bound of the length of the keys (at any depth) of the pushed contexts; grows only.
  max_local: Cell<usize>,
  /// Upper bound of the length of the composite keys of the pushed contexts; grows only.
  max_local_chain: Cell<usize>,
  /// Length of the longest parsed name; grows only.
  max_names: Cell<usize>,
  /// Parsing contexts of the input contexts, built on the first lookup of a composite name.
  trees: OnceCell<Vec<ParsingContext>>,
}

impl<'a> ParsingScope<'a> {
  /// Creates a parsing scope over input contexts (bottom first).
  pub fn new(frames: &'a [FeelContext]) -> Self {
    Self {
      input: frames.iter().map(FrameIndex::new).collect(),
      input_live: Cell::new(frames.len()),
      locals: RefCell::new(vec![]),
      names: RefCell::new(HashSet::default()),
      max_local: Cell::new(0),
      max_local_chain: Cell::new(0),
      max_names: Cell::new(0),
      trees: OnceCell::new(),
    }
  }
}

impl Default for ParsingScope<'_> {
  /// Creates a default parsing scope containing single parsing context.
  fn default() -> Self {
    let scope = Self::new(&[]);
    scope.locals.borrow_mut().push(ParsingContext::default());
    scope
  }
}

impl Display for ParsingScope<'_> {
  /// Converts parsing scope to text representation.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let input = self.input[..self.input_live.get()].iter().map(|frame| ParsingContext::from(frame.context()).to_string());
    let locals = self.locals.borrow().iter().map(|ctx| ctx.to_string()).collect::<Vec<String>>();
    write!(f, "[{}]", input.chain(locals).collect::<Vec<String>>().join(", "))
  }
}

impl ParsingScope<'_> {
  /// Removes the context from the top of the stack.
  pub fn pop(&self) {
    if self.locals.borrow_mut().pop().is_none() {
      self.input_live.set(self.input_live.get().saturating_sub(1));
    }
  }

  /// Puts a context on the top of the stack.
  pub fn push(&self, ctx: ParsingContext) {
    let (max_plain, max_flattened) = ctx.key_lengths();
    self.grow_local_lengths(max_plain, max_flattened);
    self.locals.borrow_mut().push(ctx);
  }

  /// Puts a default context on the top of the stack.
  pub fn push_default(&self) {
    self.locals.borrow_mut().push(ParsingContext::default());
  }

  /// Sets a specified entry name in context placed on the top of the stack.
  pub fn set_entry_name(&self, name: Name) {
    self.grow_local_lengths(name.as_str().len(), name.as_str().len());
    if let Some(top) = self.top_mut().last_mut() {
      top.set_name(name);
    }
  }

  /// Sets a context under a specified name in the context placed on the top of the stack.
  pub fn set_context(&self, name: Name, ctx: ParsingContext) {
    let len = name.as_str().len();
    let (max_plain, max_flattened) = ctx.key_lengths();
    self.grow_local_lengths(len.max(max_plain), if ctx.is_empty() { len } else { len + 3 + max_flattened });
    if let Some(top) = self.top_mut().last_mut() {
      top.set_context(name, ctx);
    }
  }

  /// Adds a name to a set of already parsed names.
  pub fn add_parsed_name(&self, name: Name) {
    self.max_names.set(self.max_names.get().max(name.as_str().len()));
    self.names.borrow_mut().insert(name);
  }

  /// Returns `true` when `name` (a flattened name, see the lexer) is in scope: a key at any depth
  /// of a context on the stack, a composite key of such a context, or a parsed name.
  pub fn contains(&self, name: &str) -> bool {
    if self.contains_plain(name) {
      return true;
    }
    // every composite key contains the separator
    if name.contains(" . ") {
      work!(CompositeSearches, 1);
      let live = &self.input[..self.input_live.get()];
      let trees = self.trees.get_or_init(|| self.input.iter().map(|frame| ParsingContext::from(frame.context())).collect());
      return trees[..live.len()].iter().chain(self.locals.borrow().iter()).any(|ctx| ctx.contains_flattened(name));
    }
    false
  }

  /// Returns `true` when `name` is in scope as a key at any depth of a context on the stack or as
  /// a parsed name: [contains](Self::contains) without the composite keys, for the names that can
  /// not be composite (see [bound](Self::bound)).
  pub(crate) fn contains_plain(&self, name: &str) -> bool {
    work!(Contains, 1);
    if self.names.borrow().contains(name) {
      return true;
    }
    let live = &self.input[..self.input_live.get()];
    self.locals.borrow().iter().any(|ctx| ctx.contains_key_deep(name)) || live.iter().any(|frame| frame.contains(name))
  }

  /// Returns an upper bound of the length in bytes of the names that [contains](Self::contains)
  /// can accept (and of the built-in property names, after a dot), for the lexer to skip the
  /// candidate names that are too long to match.
  ///
  /// The lexer's candidate names contain `" . "` only when one of their parts is empty after
  /// trimming, so composite keys are taken into account only then (`has_empty_part`).
  pub fn bound(&self, after_dot: bool, has_empty_part: bool) -> usize {
    let live = &self.input[..self.input_live.get()];
    let mut bound = live.iter().map(FrameIndex::max_plain).max().unwrap_or(0);
    bound = bound.max(self.max_local.get()).max(self.max_names.get());
    if has_empty_part {
      bound = bound.max(live.iter().map(FrameIndex::max_chain).max().unwrap_or(0));
      bound = bound.max(self.max_local_chain.get());
    }
    if after_dot {
      bound = bound.max(MAX_BUILT_IN_PROPERTY_NAME_LEN);
    }
    bound
  }

  /// Returns the pushed contexts, the top one last. When none is pushed, the top input context
  /// (if any) is converted into a pushed context first, so that it can be changed; it stays at
  /// the same place in the stack.
  fn top_mut(&self) -> std::cell::RefMut<'_, Vec<ParsingContext>> {
    let mut locals = self.locals.borrow_mut();
    if locals.is_empty() && self.input_live.get() > 0 {
      let index = self.input_live.get() - 1;
      self.input_live.set(index);
      let ctx = ParsingContext::from(self.input[index].context());
      let (max_plain, max_flattened) = ctx.key_lengths();
      self.grow_local_lengths(max_plain, max_flattened);
      locals.push(ctx);
    }
    locals
  }

  fn grow_local_lengths(&self, max_plain: usize, max_flattened: usize) {
    self.max_local.set(self.max_local.get().max(max_plain));
    self.max_local_chain.set(self.max_local_chain.get().max(max_flattened).max(max_plain));
  }

  /// Returns already parsed flattened keys and names (the set the lexer used to search before
  /// [contains](Self::contains) and [bound](Self::bound); kept as the reference in tests).
  #[cfg(test)]
  pub fn flattened(&self) -> HashSet<String> {
    let input = self.input[..self.input_live.get()].iter().map(|frame| ParsingContext::from(frame.context()));
    let mut keys = input.flat_map(|ctx| ctx.flattened_keys()).collect::<HashSet<String>>();
    keys.extend(self.locals.borrow().iter().flat_map(|ctx| ctx.flattened_keys()));
    let names = self.names.borrow().iter().map(|name| name.to_string()).collect::<HashSet<String>>();
    keys.union(&names).cloned().collect()
  }
}

/// Upper bound of the length of a built-in property name (`start included`), see [is_built_in_property_name](rheon_feel::bip::is_built_in_property_name).
pub(crate) const MAX_BUILT_IN_PROPERTY_NAME_LEN: usize = 16;
