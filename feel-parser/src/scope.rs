//! Implementation of the scope used while parsing FEEL expressions.

use crate::composite::{COMPOSITE_SEPARATOR, contains_composite};
use crate::context::ParsingContext;
use crate::index::FrameIndex;
use rheon_feel::Name;
use rheon_feel::context::FeelContext;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::Display;

/// A set of lengths of names, which only grows: the lengths below 256 as bits, the others
/// listed.
#[derive(Default)]
pub(crate) struct Lengths {
  short: [u64; 4],
  long: Vec<usize>,
}

impl Lengths {
  pub(crate) fn insert(&mut self, len: usize) {
    if len < 256 {
      self.short[len / 64] |= 1 << (len % 64);
    } else if !self.long.contains(&len) {
      self.long.push(len);
    }
  }

  pub(crate) fn contains(&self, len: usize) -> bool {
    if len < 256 {
      self.short[len / 64] & (1 << (len % 64)) != 0
    } else {
      self.long.contains(&len)
    }
  }
}

/// Parsing scope: the names the lexer recognizes as names in scope.
///
/// The scope is a stack of contexts: at the bottom the input contexts (the frames of the
/// [FeelScope](rheon_feel::FeelScope) the expression is parsed in), above them the contexts
/// pushed while parsing (function parameters, iteration variables). A name is in scope when it
/// is a key at any depth of a context on the stack, a composite key `a . b` of such a context,
/// or an already parsed name (context entry keys).
///
/// The input contexts are borrowed and indexed, never copied.
pub struct ParsingScope<'a> {
  /// Indexes of the input contexts, bottom first.
  input: Vec<FrameIndex<'a>>,
  /// Number of input contexts still on the stack (popping more contexts than were pushed pops
  /// input contexts, from the top).
  input_live: Cell<usize>,
  /// Contexts pushed while parsing, above the input contexts.
  locals: RefCell<Vec<ParsingContext>>,
  /// For each key of the pushed contexts themselves (not of contexts nested in them), the
  /// number of pushed contexts that have it: a lookup among pushed contexts without nested
  /// ones is one step, however many are pushed (nested binders push one each).
  local_keys: RefCell<HashMap<Name, usize>>,
  /// The lengths of the keys that pushed contexts themselves had so far and of the parsed
  /// names; grows only.
  lengths: RefCell<Lengths>,
  /// Set of parsed names.
  names: RefCell<HashSet<Name>>,
  /// Length of the longest parsed name that is longer than its first word ([first_word]); grows only.
  max_names_longer: Cell<usize>,
  /// `true` when a pushed context has nested contexts (their keys are not in [group_bound](Self::group_bound)'s windows).
  local_nested: Cell<bool>,
  /// Upper bound of the length of the keys (at any depth) of the pushed contexts; grows only.
  max_local: Cell<usize>,
  /// Upper bound of the length of the composite keys of the pushed contexts; grows only.
  max_local_chain: Cell<usize>,
  /// Length of the longest parsed name; grows only.
  max_names: Cell<usize>,
}

impl<'a> ParsingScope<'a> {
  /// Creates a parsing scope over input contexts (bottom first).
  pub(crate) fn new(frames: impl IntoIterator<Item = &'a FeelContext>) -> Self {
    let input: Vec<FrameIndex<'a>> = frames.into_iter().map(FrameIndex::new).collect();
    Self {
      input_live: Cell::new(input.len()),
      input,
      locals: RefCell::new(vec![]),
      local_keys: RefCell::new(HashMap::new()),
      lengths: RefCell::new(Lengths::default()),
      names: RefCell::new(HashSet::default()),
      max_names_longer: Cell::new(0),
      local_nested: Cell::new(false),
      max_local: Cell::new(0),
      max_local_chain: Cell::new(0),
      max_names: Cell::new(0),
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
    match self.locals.borrow_mut().pop() {
      Some(ctx) => {
        let mut keys = self.local_keys.borrow_mut();
        for key in ctx.keys() {
          if let Some(count) = keys.get_mut(key) {
            *count -= 1;
            if *count == 0 {
              keys.remove(key);
            }
          }
        }
      }
      None => self.input_live.set(self.input_live.get().saturating_sub(1)),
    }
  }

  /// Counts the keys of a context that is pushed.
  fn count_keys(&self, ctx: &ParsingContext) {
    let mut keys = self.local_keys.borrow_mut();
    let mut lengths = self.lengths.borrow_mut();
    for key in ctx.keys() {
      *keys.entry(key.clone()).or_insert(0) += 1;
      lengths.insert(key.as_str().len());
    }
  }

  /// Puts a context on the top of the stack.
  pub fn push(&self, ctx: ParsingContext) {
    let (max_plain, max_flattened) = ctx.key_lengths();
    self.grow_local_lengths(max_plain, max_flattened);
    self.local_nested.set(self.local_nested.get() || ctx.has_nested());
    self.count_keys(&ctx);
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
      if !top.has_key(name.as_str()) {
        *self.local_keys.borrow_mut().entry(name.clone()).or_insert(0) += 1;
      }
      self.lengths.borrow_mut().insert(name.as_str().len());
      top.set_name(name);
    }
  }

  /// Sets a context under a specified name in the context placed on the top of the stack.
  pub fn set_context(&self, name: Name, ctx: ParsingContext) {
    if let Some(top) = self.top_mut().last_mut() {
      // the bounds cover the other entries of the top context already: each was added by an
      // operation that grew them
      let (max_plain, max_flattened) = ParsingContext::entry_key_lengths(name.as_str(), &ctx);
      if !top.has_key(name.as_str()) {
        *self.local_keys.borrow_mut().entry(name.clone()).or_insert(0) += 1;
      }
      self.lengths.borrow_mut().insert(name.as_str().len());
      top.set_context(name, ctx);
      self.grow_local_lengths(max_plain, max_flattened);
      self.local_nested.set(true);
    }
  }

  /// Adds a name to a set of already parsed names.
  pub fn add_parsed_name(&self, name: Name) {
    let len = name.as_str().len();
    self.max_names.set(self.max_names.get().max(len));
    if len > self.max_names_longer.get() && first_word(name.as_str()).len() < len {
      self.max_names_longer.set(len);
    }
    self.lengths.borrow_mut().insert(len);
    self.names.borrow_mut().insert(name);
  }

  /// Returns the names that the contexts pushed while parsing hold themselves, and the parsed
  /// names: what the text itself brought into scope (for the editor's completion).
  pub(crate) fn local_names(&self) -> Vec<String> {
    let mut names: Vec<String> = self.locals.borrow().iter().flat_map(|ctx| ctx.keys().map(|name| name.to_string())).collect();
    names.extend(self.names.borrow().iter().map(|name| name.to_string()));
    names.sort_unstable();
    names.dedup();
    names
  }

  /// Returns `false` only when [contains_plain](Self::contains_plain) accepts no name of `len`
  /// bytes: no parsed name, no key of a pushed context without nested ones, and no key of an
  /// input context has this length. The lexer asks before it writes a candidate name.
  pub(crate) fn may_contain_length(&self, len: usize) -> bool {
    let live = &self.input[..self.input_live.get()];
    self.local_nested.get() || self.lengths.borrow().contains(len) || live.iter().any(|frame| frame.has_length(len))
  }

  /// Returns `true` when `name` (a flattened name, see the lexer) is in scope: a key at any depth
  /// of a context on the stack, a composite key of such a context, or a parsed name.
  pub(crate) fn contains(&self, name: &str) -> bool {
    if self.contains_plain(name) {
      return true;
    }
    // every composite key contains the separator
    if name.contains(COMPOSITE_SEPARATOR) {
      let live = &self.input[..self.input_live.get()];
      return live.iter().any(|frame| frame.contains_composite(name)) || self.locals.borrow().iter().any(|ctx| contains_composite(ctx, name));
    }
    false
  }

  /// Returns `true` when `name` is in scope as a key at any depth of a context on the stack or as
  /// a parsed name: [contains](Self::contains) without the composite keys, for the names that can
  /// not be composite (see [bound](Self::bound)).
  pub(crate) fn contains_plain(&self, name: &str) -> bool {
    if self.names.borrow().contains(name) {
      return true;
    }
    let live = &self.input[..self.input_live.get()];
    // without nested contexts the keys of the pushed contexts are all their keys
    let pushed = if self.local_nested.get() {
      let locals = self.locals.borrow();
      work!(LocalContexts, locals.len());
      locals.iter().any(|ctx| ctx.contains_key_deep(name))
    } else {
      self.local_keys.borrow().contains_key(name)
    };
    pushed || live.iter().any(|frame| frame.contains(name))
  }

  /// Returns an upper bound of the length in bytes of the names that [contains](Self::contains)
  /// can accept, for the lexer to skip the candidate names that are too long to match; the
  /// composite keys count only when `composite` is `true`.
  pub(crate) fn bound(&self, composite: bool) -> usize {
    let live = &self.input[..self.input_live.get()];
    let mut bound = live.iter().map(FrameIndex::max_plain).max().unwrap_or(0);
    bound = bound.max(self.max_local.get()).max(self.max_names.get());
    if composite {
      bound = bound.max(live.iter().map(FrameIndex::max_chain).max().unwrap_or(0));
      bound = bound.max(self.max_local_chain.get());
    }
    bound
  }

  /// Returns an upper bound of the length in bytes of the names that [contains_plain](Self::contains_plain)
  /// can accept and whose first word is `word` (see [first_word]), `word` itself included. The
  /// lexer's candidate names without an empty part all start with the first part of the token,
  /// which is a first word, so this bounds them more tightly than [bound](Self::bound). A result
  /// of at least `limit` (the lexer's longest candidate) prunes nothing, so the search stops there.
  pub(crate) fn group_bound(&self, word: &str, limit: usize) -> usize {
    let live = &self.input[..self.input_live.get()];
    // `word` itself, and the names that are longer than their first word, without a lookup
    let mut bound = live.iter().fold(word.len().max(self.max_names_longer.get()), |bound, frame| bound.max(frame.max_longer()));
    if self.local_nested.get() {
      bound = bound.max(self.max_local.get());
    }
    if bound >= limit {
      return bound;
    }
    // the keys of the contexts themselves
    bound = live.iter().fold(bound, |bound, frame| bound.max(frame.group_bound(word)));
    if !self.local_nested.get() {
      let locals = self.locals.borrow();
      bound = if locals.len() <= GROUP_BOUND_CONTEXTS {
        work!(LocalContexts, locals.len());
        locals.iter().fold(bound, |bound, ctx| bound.max(ctx.group_bound(word, self.max_local.get())))
      } else {
        // the bound of all their keys, which is no smaller: a bound only prunes
        bound.max(self.max_local.get())
      };
    }
    bound
  }

  /// Returns the pushed contexts, the top one last. When none is pushed, the top input context
  /// (if any) is converted into a pushed context first, so that it can be changed; it stays at
  /// the same place in the stack.
  fn top_mut(&self) -> RefMut<'_, Vec<ParsingContext>> {
    let mut locals = self.locals.borrow_mut();
    if locals.is_empty() && self.input_live.get() > 0 {
      let index = self.input_live.get() - 1;
      self.input_live.set(index);
      let ctx = ParsingContext::from(self.input[index].context());
      let (max_plain, max_flattened) = ctx.key_lengths();
      self.grow_local_lengths(max_plain, max_flattened);
      self.local_nested.set(self.local_nested.get() || ctx.has_nested());
      self.count_keys(&ctx);
      locals.push(ctx);
    }
    locals
  }

  fn grow_local_lengths(&self, max_plain: usize, max_flattened: usize) {
    self.max_local.set(self.max_local.get().max(max_plain));
    self.max_local_chain.set(self.max_local_chain.get().max(max_flattened).max(max_plain));
  }
}

/// The most pushed contexts whose keys [ParsingScope::group_bound] looks at one context after
/// another; with more, it takes the bound of all their keys.
const GROUP_BOUND_CONTEXTS: usize = 16;

/// Number of keys of a context that [first_word_bound] looks at before it gives up.
const FIRST_WORD_WINDOW: usize = 16;

/// Returns the first word of a name: the text before its first space or additional name symbol.
pub(crate) fn first_word(name: &str) -> &str {
  name.find([' ', '.', '/', '-', '\'', '+', '*']).map_or(name, |end| &name[..end])
}

/// Returns an upper bound of the length in bytes of the keys whose first word is `word` (see
/// [first_word]), or `fallback` (a bound of all keys) when there are too many keys to look at.
/// `keys` are the ordered keys of a context from the first key not less than `word` on. The keys
/// whose first word is `word` are `word` followed by a space or a symbol, all bytes below `0`, so
/// they come first; the keys looked at may include others, which only enlarges the bound.
pub(crate) fn first_word_bound<'k>(keys: impl Iterator<Item = &'k Name>, word: &str, fallback: usize) -> usize {
  let mut bound = 0;
  for (count, key) in keys.map(Name::as_str).enumerate() {
    if !key.starts_with(word) || key.as_bytes().get(word.len()).is_some_and(|&byte| byte >= b'0') {
      break;
    }
    if count == FIRST_WORD_WINDOW {
      return fallback;
    }
    bound = bound.max(key.len());
  }
  bound
}
