//! What an expression reads of the values of its scope, without evaluating it: for each name of
//! the scope, the parts of its value that the expression can read.
//!
//! The answer is **exact or wider, never narrower**: an evaluation with a scope that holds only
//! what the answer lists gives what the evaluation with the whole scope gives. So the walk is
//! over every node of the tree as name resolution leaves it ([crate::flat::resolve]), and it
//! reads the value of every operand **whole** unless the construct is one of the few that are
//! known to pass a value on without looking into it:
//!
//! - a path (`a.b`) reads the entry of what its base is, a filter the items of its list, a
//!   loop the items of its domains;
//! - an entry of a context literal, a branch of `if`, an item of a list literal and the body of
//!   a `for` are what the construct gives, read only as far as that is read;
//! - an argument of a function definition of the text is what its parameter is in the body;
//! - a few built-in functions give back their argument or its items without comparing them
//!   ([PASSING]), and `get value` with a key that is a string literal is a path.
//!
//! Everything else (an operator, a comparison, an argument of another function, the result of
//! the expression) reads a value whole: all of it, however deep it is.
//!
//! A value is followed as a **source**: the paths of the scope it can be (a set), or for a
//! context literal the sources of its entries. A source is *lifted* once for each list it was
//! put in, so that `for x in l return x.a` is the list of `l[].a`, and an item of it is `l[].a`
//! again. Whether a path is a list is read from the value of the scope or from the type that
//! the host states for it; where neither says, an entry of a path is written as an entry
//! (`loans.amount`), also when the value turns out to be a list of contexts.

use super::types::{ANY, Kind};
use super::{TypeId, Typing};
use crate::flat::{Info, LayoutKind, NONE, Place, Read, Region, RegionKind, Resolved, Target, resolve_names, variables};
use rheon_feel::Name;
use rheon_feel::signatures::built_in;
use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::collections::HashMap;

/// The most nodes that one analysis walks: the body of a function definition is walked again
/// at each invocation, with the sources of the arguments. Beyond it the answer is not complete.
const MAX_STEPS: usize = 200_000;

/// The deepest that the walk goes: the depth of the tree and of the invocations in each other.
const MAX_DEPTH: usize = 300;

/// The most sources that one value can be before it is read whole.
const MAX_UNION: usize = 32;

/// The most values of the scope that are looked at to say whether the items of a list have an
/// entry: beyond it, they may have it.
const MAX_ITEMS: usize = 20_000;

/// The most places that are kept for one path.
const MAX_PLACES: usize = 64;

/// The built-in functions that give back their first argument, or a list of its items and of
/// the other arguments, without comparing the items: with positional arguments.
const PASSING: [&str; 6] = ["reverse", "sublist", "remove", "append", "concatenate", "insert before"];

/// A segment of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
  /// The entry of this name: of the scope for the first segment, of a context for the others.
  Entry(Name),
  /// The items of a list.
  Items,
}

/// A part of a value of the scope that an expression reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPath {
  pub segments: Vec<Segment>,
  /// Whether the value at the path is read as a value: all of it can be read. Otherwise only
  /// what the paths below it read, and that it is there.
  pub whole: bool,
  /// Where the text reads it, as byte ranges, in the order of the text.
  pub places: Vec<(usize, usize)>,
}

/// What an expression reads ([Typing::reads]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reads {
  pub paths: Vec<ReadPath>,
  /// The names that the text reads and that nothing gives: no construct of the text binds
  /// them, the scope has no value of them and they are no built-in functions. Only asked for.
  pub unknown: Vec<(Name, Vec<(usize, usize)>)>,
  /// Whether the walk ended: when it did not ([MAX_STEPS], [MAX_DEPTH]), the expression can
  /// read more than `paths` lists.
  pub complete: bool,
}

/// What the walk knows of a value: its source, and the function definition it is.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Val {
  /// An index of [Walk::sources], [NONE] for a value that is made of no value of the scope.
  source: u32,
  /// The lists that the source was put in.
  lift: u8,
  /// An index of [Walk::defs], [NONE] when the value is no function definition of the text.
  def: u32,
}

impl Val {
  const NONE: Val = Val { source: NONE, lift: 0, def: NONE };
  /// What the binder of an entry of a context literal holds before the entry has its value:
  /// a function written in the literal does not read the entry then, but the name further out.
  const UNSET: Val = Val {
    source: NONE,
    lift: u8::MAX,
    def: NONE,
  };
}

enum Source {
  /// The value at a path ([Walk::paths]).
  Path(u32),
  /// A context made by the text: its entries.
  Record(Vec<(Name, Val)>),
  /// One of several values.
  Union(Vec<Val>),
}

struct PathNode {
  parent: u32,
  /// The name of the entry, None for the items of a list.
  segment: Option<Name>,
  ty: TypeId,
  whole: bool,
  places: Vec<(u32, u32)>,
  /// The source that is this path.
  source: u32,
  has_children: bool,
  /// Whether the path is of the items of a list, or below them: the type of the items of a
  /// list of the scope is that of its first item, and another item can have other entries.
  in_list: bool,
}

/// A function definition of the text.
struct Def {
  node: u32,
  /// The frame it was reached in.
  frame: u32,
  /// Whether it left the place where the walk can follow it (it was read whole): its body was
  /// walked with parameters of no source, and an invocation of it reads its arguments whole.
  escaped: bool,
  /// Whether its body invoked it while it was walked: what that invocation gives is not
  /// followed, so what the walk of the body gives is read whole.
  recursive: bool,
  /// The binders around it as they were when their constructs ended, by their slots: a slot
  /// is used again by a construct after its own, and the function reads what was there.
  kept: HashMap<u32, Val>,
  /// The same for the items of the filters it is written in, by their records.
  kept_items: HashMap<u32, Val>,
}

#[derive(Clone, Copy)]
enum Outer {
  /// The names of the scope.
  Root,
  /// Where a function definition is written: the frame and the region, and the definition
  /// (an index of [Walk::defs]).
  Site { frame: u32, region: u32, def: u32 },
}

struct Frame {
  base: u32,
  outer: Outer,
  /// The items of the filters whose predicates are being walked, by their records.
  items: Vec<Val>,
}

/// Whether a value has an entry of a name.
enum Has {
  Yes,
  No,
  Maybe,
}

struct Walk<'a, 's, 'n> {
  typing: &'a mut Typing<'s>,
  unit: &'a Resolved<'n>,
  place: &'a dyn Fn(&AstNode) -> Option<(usize, usize)>,
  /// Whether a name that nothing gives is reported, and is no path.
  known: bool,
  frames: Vec<Frame>,
  slots: Vec<Val>,
  sources: Vec<Source>,
  paths: Vec<PathNode>,
  interned: HashMap<(u32, Option<Name>), u32>,
  defs: Vec<Def>,
  active: Vec<u32>,
  unknown: Vec<(Name, Vec<(usize, usize)>)>,
  /// The names of the scope read so far, each with the key that the scope writes it as.
  roots: HashMap<Name, Name>,
  /// Whether the items of a list of the scope at a path have an entry of a name: all, none, or
  /// (None) some ([Walk::items_have]).
  items: HashMap<(u32, Name), Option<bool>>,
  /// Whether the name read last is a built-in function: no value of the scope and no binder.
  built_in: bool,
  /// The `for` expressions whose results so far were read: the frame and the record of each.
  partials: Vec<(u32, u32)>,
  /// Whether a function that the scope holds was read: its body is not in the text, and it
  /// reads the names of the context it was written in.
  everything: bool,
  depth: usize,
  steps: usize,
  complete: bool,
}

impl Typing<'_> {
  /// What the expression `node` reads of the scope. `place` gives the place of a node of the
  /// tree in the text (see `rheon_feel_parser::Positioned`). With `known`, a name that nothing
  /// gives is listed as unknown; without, it is a path, a name that the host is to give. With
  /// `unary_tests` the tested value, the name `?`, is read whole.
  pub fn reads(&mut self, node: &AstNode, place: &dyn Fn(&AstNode) -> Option<(usize, usize)>, known: bool, unary_tests: bool) -> Reads {
    let unit = resolve_names(node);
    let mut walk = Walk {
      typing: self,
      unit: &unit,
      place,
      known,
      frames: vec![],
      slots: vec![],
      sources: vec![],
      paths: vec![],
      interned: HashMap::new(),
      defs: vec![],
      active: vec![],
      unknown: vec![],
      roots: HashMap::new(),
      items: HashMap::new(),
      built_in: false,
      partials: vec![],
      everything: false,
      depth: 0,
      steps: 0,
      complete: true,
    };
    if unary_tests {
      let tested = walk.root(&Name::from("?"), None);
      walk.whole(tested);
    }
    let frame = walk.enter(unit.root.binders, Outer::Root);
    let result = walk.flow(frame, unit.root());
    walk.whole(result);
    walk.finish()
  }
}

impl<'a, 'n> Walk<'a, '_, 'n> {
  fn enter(&mut self, binders: u32, outer: Outer) -> usize {
    let base = self.slots.len();
    self.slots.resize(base + binders as usize, Val::NONE);
    self.frames.push(Frame {
      base: base as u32,
      outer,
      items: vec![],
    });
    self.frames.len() - 1
  }

  fn leave(&mut self) {
    if let Some(frame) = self.frames.pop() {
      self.slots.truncate(frame.base as usize);
    }
  }

  fn slot(&mut self, frame: usize, slot: u32) -> Option<&mut Val> {
    let base = self.frames.get(frame)?.base as usize;
    let end = self.frames.get(frame + 1).map_or(self.slots.len(), |next| next.base as usize);
    self.slots.get_mut(base..end)?.get_mut(slot as usize)
  }

  fn set(&mut self, frame: usize, place: Place, val: Val) {
    if let Place::Slot(slot) = place
      && let Some(held) = self.slot(frame, slot)
    {
      *held = val;
    }
  }

  //----------------------------------------------------------------------------------------------
  // Sources
  //----------------------------------------------------------------------------------------------

  fn source(&mut self, source: Source) -> u32 {
    self.sources.push(source);
    self.sources.len() as u32 - 1
  }

  /// The path `segment` below `parent` ([NONE] for a name of the scope), of the type `ty`.
  fn intern(&mut self, parent: u32, segment: Option<Name>, ty: TypeId) -> u32 {
    if let Some(&path) = self.interned.get(&(parent, segment.clone())) {
      return path;
    }
    let path = self.paths.len() as u32;
    let source = self.source(Source::Path(path));
    let in_list = segment.is_none() || self.paths.get(parent as usize).is_some_and(|parent| parent.in_list);
    self.paths.push(PathNode {
      parent,
      segment: segment.clone(),
      ty,
      whole: false,
      places: vec![],
      source,
      has_children: false,
      in_list,
    });
    if let Some(parent) = self.paths.get_mut(parent as usize) {
      parent.has_children = true;
    }
    self.interned.insert((parent, segment), path);
    path
  }

  fn of_path(&self, path: u32, lift: u8) -> Val {
    Val {
      source: self.paths[path as usize].source,
      lift,
      def: NONE,
    }
  }

  /// The name `name` of the scope, read at `at`.
  fn root(&mut self, name: &Name, at: Option<(usize, usize)>) -> Val {
    let ty = self.typing.value(name).unwrap_or(ANY);
    // the segment is the key as the scope writes it
    let key = match self.roots.get(name) {
      Some(key) => key.clone(),
      None => {
        let key = self.typing.written_key(name);
        self.roots.insert(name.clone(), key.clone());
        key
      }
    };
    let path = self.intern(NONE, Some(key), ty);
    self.note(path, at);
    self.of_path(path, 0)
  }

  fn note(&mut self, path: u32, at: Option<(usize, usize)>) {
    if let Some((start, end)) = at {
      let places = &mut self.paths[path as usize].places;
      let place = (start as u32, end as u32);
      if places.len() < MAX_PLACES && !places.contains(&place) {
        places.push(place);
      }
    }
  }

  /// Notes the place `at` for the paths that `val` is.
  fn note_val(&mut self, val: Val, at: Option<(usize, usize)>) {
    if at.is_none() || val.source == NONE {
      return;
    }
    match &self.sources[val.source as usize] {
      Source::Path(path) => self.note(*path, at),
      Source::Union(members) => {
        let paths: Vec<u32> = members
          .iter()
          .filter_map(|member| match self.sources.get(member.source as usize) {
            Some(Source::Path(path)) => Some(*path),
            _ => None,
          })
          .collect();
        for path in paths {
          self.note(path, at);
        }
      }
      Source::Record(_) => {}
    }
  }

  /// Reads `val` whole: every path it can be, the entries of a context it is, and a function
  /// definition it is leaves the walk ([Walk::escape]).
  fn whole(&mut self, val: Val) {
    let mut stack = vec![val];
    while let Some(val) = stack.pop() {
      if val.def != NONE {
        self.escape(val.def);
      }
      if val.source == NONE {
        continue;
      }
      match &self.sources[val.source as usize] {
        Source::Path(path) => {
          let path = *path as usize;
          self.paths[path].whole = true;
          // a function that a context of the scope holds (not one that an outline states)
          if !self.typing.outlined() && matches!(self.typing.types.kind(self.paths[path].ty), Kind::Function(..)) {
            self.everything = true;
          }
        }
        Source::Record(entries) => stack.extend(entries.iter().map(|(_, entry)| *entry)),
        Source::Union(members) => stack.extend(members.iter().copied()),
      }
    }
  }

  /// A function definition that the walk can not follow further: its body is walked with
  /// parameters of no source, for the names it reads from where it is written.
  fn escape(&mut self, def: u32) {
    let Def { node, frame, escaped, .. } = self.defs[def as usize];
    if escaped {
      return;
    }
    self.defs[def as usize].escaped = true;
    if self.active.contains(&node) {
      // its body is being walked: what that walk gives is read whole when it ends
      self.defs[def as usize].recursive = true;
      return;
    }
    if let Some(result) = self.call(def, node, frame, &[], None) {
      self.whole(result);
    }
  }

  /// The function definitions in `val` that were reached in the frame `leaving` or after it
  /// leave the walk: their frames end.
  fn settle(&mut self, val: Val, leaving: usize) {
    let mut stack = vec![val];
    let mut seen = 0usize;
    while let Some(val) = stack.pop() {
      seen += 1;
      if seen > MAX_STEPS {
        self.complete = false;
        return;
      }
      if val.def != NONE && self.defs[val.def as usize].frame as usize >= leaving {
        self.escape(val.def);
      }
      if val.source == NONE {
        continue;
      }
      match &self.sources[val.source as usize] {
        Source::Path(_) => {}
        Source::Record(entries) => stack.extend(entries.iter().map(|(_, entry)| *entry)),
        Source::Union(members) => stack.extend(members.iter().copied()),
      }
    }
  }

  /// The value without the function definition it is, which leaves the walk.
  fn plain(&mut self, val: Val) -> Val {
    if val.def != NONE {
      self.escape(val.def);
    }
    Val { def: NONE, ..val }
  }

  /// One of the values `a` and `b`.
  fn union(&mut self, a: Val, b: Val) -> Val {
    let (a, b) = (self.plain(a), self.plain(b));
    if a == b || b.source == NONE {
      return a;
    }
    if a.source == NONE {
      return b;
    }
    let mut members: Vec<Val> = vec![];
    for val in [a, b] {
      match &self.sources[val.source as usize] {
        Source::Union(inner) => {
          for member in inner {
            let member = Val {
              lift: member.lift.saturating_add(val.lift),
              ..*member
            };
            if !members.contains(&member) {
              members.push(member);
            }
          }
        }
        _ if !members.contains(&val) => members.push(val),
        _ => {}
      }
    }
    if members.len() > MAX_UNION {
      for member in members {
        self.whole(member);
      }
      return Val::NONE;
    }
    Val {
      source: self.source(Source::Union(members)),
      lift: 0,
      def: NONE,
    }
  }

  /// The list of values like `val`.
  fn listed(&mut self, val: Val) -> Val {
    let val = self.plain(val);
    if val.source == NONE {
      return val;
    }
    if val.lift == u8::MAX {
      self.whole(val);
      return Val::NONE;
    }
    Val { lift: val.lift + 1, ..val }
  }

  /// An item of `val`: of a list, or the value itself when it is known to be no list.
  fn items(&mut self, val: Val) -> Val {
    let val = self.plain(val);
    if val.source == NONE {
      return val;
    }
    if val.lift > 0 {
      return Val { lift: val.lift - 1, ..val };
    }
    match &self.sources[val.source as usize] {
      Source::Path(path) => {
        let path = *path;
        let ty = self.paths[path as usize].ty;
        match self.typing.types.kind(ty) {
          Kind::List(item) => {
            let item = *item;
            let items = self.intern(path, None, item);
            self.of_path(items, 0)
          }
          Kind::Any | Kind::Null => {
            let items = self.intern(path, None, ANY);
            self.of_path(items, 0)
          }
          // a value that is no list is the list of itself
          _ => val,
        }
      }
      Source::Record(_) => val,
      Source::Union(members) => {
        let members = members.clone();
        let mut items = Val::NONE;
        for member in members {
          let item = self.items(member);
          items = self.union(items, item);
        }
        items
      }
    }
  }

  /// The entry `name` of `val`: of a context, or of the contexts of a list.
  fn entry(&mut self, val: Val, name: &Name) -> Val {
    let val = self.plain(val);
    if val.source == NONE {
      return val;
    }
    match &self.sources[val.source as usize] {
      Source::Path(path) => {
        let mut path = *path;
        let mut lift = val.lift;
        // a list gives the list of the entries of its items
        for _ in 0..8 {
          let ty = self.paths[path as usize].ty;
          match self.typing.types.kind(ty) {
            Kind::List(item) => {
              let item = *item;
              path = self.intern(path, None, item);
              if lift == u8::MAX {
                let val = self.of_path(path, 0);
                self.whole(val);
                return Val::NONE;
              }
              lift += 1;
            }
            Kind::Context(_) | Kind::Any | Kind::Null => {
              // the segment is the key as the context writes it
              let (key, member) = self.key(ty, name).unwrap_or_else(|| (name.clone(), ANY));
              let entry = self.intern(path, Some(key), member);
              return self.of_path(entry, lift);
            }
            // a property of a value that has no entries (the year of a date): of the value;
            // of an item of a list of the scope also the entry, which another item can have
            _ => {
              let value = self.of_path(path, lift);
              if self.paths[path as usize].in_list && !self.typing.outlined() && !matches!(self.items_have(path, name), Has::No) {
                let entry = self.intern(path, Some(name.clone()), ANY);
                let entry = self.of_path(entry, lift);
                return self.union(value, entry);
              }
              return value;
            }
          }
        }
        let val = self.of_path(path, 0);
        self.whole(val);
        Val::NONE
      }
      Source::Record(entries) => match entries.iter().rev().find(|(key, _)| is_key(key, name)).map(|(_, entry)| *entry) {
        Some(entry) if val.lift == 0 => entry,
        // the entries of the contexts of a list: a function among them is followed no further
        Some(entry) => {
          let entry = self.plain(entry);
          Val {
            lift: entry.lift.saturating_add(val.lift),
            ..entry
          }
        }
        None => Val::NONE,
      },
      Source::Union(members) => {
        let members = members.clone();
        let mut entries = Val::NONE;
        for member in members {
          let member = Val {
            lift: member.lift.saturating_add(val.lift),
            ..member
          };
          let entry = self.entry(member, name);
          entries = self.union(entries, entry);
        }
        entries
      }
    }
  }

  /// The entry that `name` reads of a context of the type `ty`: its key as the context writes it
  /// (which can be in another form than FEEL reads a name), and its type.
  fn key(&self, ty: TypeId, name: &Name) -> Option<(Name, TypeId)> {
    let entries = self.typing.entries(ty)?;
    entries
      .iter()
      .find(|(key, _)| key == name)
      .or_else(|| entries.iter().find(|(key, _)| rheon_feel::names::is_name(key.as_str(), name.as_str())))
      .cloned()
  }

  /// Whether the values of the scope at `path`, a path below the items of a list, have an entry
  /// `name`: all of them, none of them, or some (also when there are more than [MAX_ITEMS]).
  fn items_have(&mut self, path: u32, name: &Name) -> Has {
    if let Some(known) = self.items.get(&(path, name.clone())) {
      return match known {
        Some(true) => Has::Yes,
        Some(false) => Has::No,
        None => Has::Maybe,
      };
    }
    // the segments from the name of the scope down
    let mut segments = vec![];
    let mut at = self.paths.get(path as usize);
    while let Some(node) = at {
      segments.push(node.segment.clone());
      at = self.paths.get(node.parent as usize);
    }
    segments.reverse();
    let answer = {
      let frames = self.typing.scope.frames();
      let root = match segments.first() {
        Some(Some(root)) => frames.iter().rev().find_map(|frame| frame.get_entry(root)),
        _ => None,
      };
      let mut values: Vec<&Value> = root.into_iter().collect();
      let mut known = root.is_some();
      for segment in segments.iter().skip(1) {
        let mut next = vec![];
        for value in values {
          match (segment, value) {
            // an entry of a context, or of each context of a list
            (Some(entry), Value::Context(context)) => next.extend(context.get_entry(entry)),
            (Some(entry), Value::List(items)) => next.extend(items.iter().filter_map(|item| match item {
              Value::Context(context) => context.get_entry(entry),
              _ => None,
            })),
            (Some(_), _) => {}
            // the items of a list; a value that is no list is the list of itself
            (None, Value::List(items)) => next.extend(items.iter()),
            (None, other) => next.push(other),
          }
          if next.len() > MAX_ITEMS {
            known = false;
            break;
          }
        }
        values = next;
        if !known {
          break;
        }
      }
      if known {
        let with = values
          .iter()
          .filter(|value| matches!(value, Value::Context(context) if context.get_entry(name).is_some()))
          .count();
        if with == 0 {
          Some(false)
        } else if with == values.len() {
          Some(true)
        } else {
          None
        }
      } else {
        None
      }
    };
    self.items.insert((path, name.clone()), answer);
    match answer {
      Some(true) => Has::Yes,
      Some(false) => Has::No,
      None => Has::Maybe,
    }
  }

  /// Whether `val` has an entry `name`, as far as its source says.
  fn has(&mut self, val: Val, name: &Name) -> Has {
    if val.source == NONE || val.lift > 0 {
      return Has::Maybe;
    }
    match &self.sources[val.source as usize] {
      Source::Path(path) => {
        let (ty, in_list) = (self.paths[*path as usize].ty, self.paths[*path as usize].in_list);
        // an item of a list of the scope has the type of the first item, and another item
        // can be unlike it: the items themselves say (an outline states the type of them all)
        if in_list && !self.typing.outlined() {
          return self.items_have(*path, name);
        }
        match self.typing.types.kind(ty) {
          Kind::Context(_) => match self.key(ty, name) {
            Some(_) => Has::Yes,
            None => Has::No,
          },
          Kind::Any | Kind::Null | Kind::List(_) => Has::Maybe,
          _ => Has::No,
        }
      }
      Source::Record(entries) => {
        if entries.iter().any(|(key, _)| is_key(key, name)) {
          Has::Yes
        } else {
          Has::No
        }
      }
      Source::Union(members) => {
        let members = members.clone();
        let (mut yes, mut no) = (0, 0);
        for &member in &members {
          match self.has(member, name) {
            Has::Yes => yes += 1,
            Has::No => no += 1,
            Has::Maybe => {}
          }
        }
        if no == members.len() {
          Has::No
        } else if yes == members.len() {
          Has::Yes
        } else {
          Has::Maybe
        }
      }
    }
  }

  //----------------------------------------------------------------------------------------------
  // The walk
  //----------------------------------------------------------------------------------------------

  /// Walks the node `id` and reads its value whole.
  fn read(&mut self, frame: usize, id: u32) {
    let val = self.flow(frame, id);
    self.whole(val);
  }

  /// Walks the node `id`: what its value is made of, read no further than the node itself reads.
  fn flow(&mut self, frame: usize, id: u32) -> Val {
    if !self.complete {
      return Val::NONE;
    }
    if self.depth >= MAX_DEPTH || self.steps >= MAX_STEPS {
      self.complete = false;
      return Val::NONE;
    }
    self.steps += 1;
    self.depth += 1;
    let val = self.walked(frame, id);
    self.depth -= 1;
    val
  }

  fn walked(&mut self, frame: usize, id: u32) -> Val {
    use AstNode as A;
    let unit = self.unit;
    let node = &unit.nodes[id as usize];
    let kids = unit.kids(id);
    match (node.ast, kids) {
      (A::Name(_), _) => match &node.info {
        Info::Read(read) => {
          let at = (self.place)(node.ast);
          self.name(frame, id, read, at)
        }
        _ => Val::NONE,
      },
      (A::QualifiedName(segments), _) => match &node.info {
        Info::Read(read) => {
          let at = (self.place)(node.ast);
          let mut val = self.name(frame, id, read, at);
          for segment in segments.iter().skip(1) {
            if let A::QualifiedNameSegment(name) = segment {
              val = self.entry(val, name);
            }
          }
          self.note_val(val, at);
          val
        }
        _ => Val::NONE,
      },
      (A::Path(_, name), &[base]) => match name.as_ref() {
        A::Name(name) => {
          let base = self.flow(frame, base);
          let val = self.entry(base, name);
          // the place of a path of the scope: an entry of a context that the text makes is
          // read where its value is written
          if !matches!(self.sources.get(base.source as usize), Some(Source::Record(_))) {
            self.note_val(val, (self.place)(node.ast));
          }
          val
        }
        _ => {
          self.read(frame, base);
          Val::NONE
        }
      },
      (A::Filter(_, predicate_ast), &[list, predicate]) => {
        let Info::Filter { record } = node.info else {
          self.read(frame, list);
          self.read(frame, predicate);
          return Val::NONE;
        };
        let list = self.flow(frame, list);
        let item = self.items(list);
        let items = &mut self.frames[frame].items;
        if items.len() <= record as usize {
          items.resize(record as usize + 1, Val::NONE);
        }
        items[record as usize] = item;
        let from = self.defs.len();
        self.read(frame, predicate);
        self.keep(frame, id, from);
        self.note_val(item, (self.place)(node.ast));
        match selects(predicate_ast) {
          Some(true) => item,
          Some(false) => self.listed(item),
          None => {
            let listed = self.listed(item);
            self.union(item, listed)
          }
        }
      }
      (A::List(_), _) => {
        let mut items = Val::NONE;
        for &kid in kids {
          let item = self.flow(frame, kid);
          let item = self.listed(item);
          items = self.union(items, item);
        }
        items
      }
      (A::Context(_), _) => self.context(frame, id),
      (A::If(..), &[condition, then, otherwise]) => {
        self.read(frame, condition);
        let then = self.flow(frame, then);
        let otherwise = self.flow(frame, otherwise);
        self.union(then, otherwise)
      }
      (A::For(contexts, _), _) => match self.iteration(frame, id, contexts, false) {
        Some(body) => {
          let from = self.defs.len();
          let body = self.flow(frame, body);
          // the results so far were read in the body: they are what the body gives
          if let Info::Iteration { record, .. } = node.info {
            let own = (frame as u32, record);
            if self.partials.contains(&own) {
              self.partials.retain(|read| *read != own);
              self.whole(body);
            }
          }
          self.keep(frame, id, from);
          self.listed(body)
        }
        None => Val::NONE,
      },
      (A::Some(contexts, _) | A::Every(contexts, _), _) => {
        let from = self.defs.len();
        if let Some(body) = self.iteration(frame, id, contexts, true) {
          self.read(frame, body);
        }
        self.keep(frame, id, from);
        Val::NONE
      }
      (A::EvaluatedExpression(_) | A::Satisfies(_), &[inner]) => self.flow(frame, inner),
      (A::FunctionDefinition(_, body), _) => match (&node.info, body.as_ref()) {
        (Info::Function { proto }, A::FunctionBody(_, false)) if *proto != NONE => {
          // the types of the parameters read nothing; the body is walked where it is invoked
          self.defs.push(Def {
            node: id,
            frame: frame as u32,
            escaped: false,
            recursive: false,
            kept: HashMap::new(),
            kept_items: HashMap::new(),
          });
          Val {
            source: NONE,
            lift: 0,
            def: self.defs.len() as u32 - 1,
          }
        }
        _ => self.operands(frame, kids),
      },
      (A::FunctionInvocation(_, arguments), _) => self.invocation(frame, id, arguments),
      _ => self.operands(frame, kids),
    }
  }

  /// The operands of a construct that reads them whole.
  fn operands(&mut self, frame: usize, kids: &[u32]) -> Val {
    for &kid in kids {
      self.read(frame, kid);
    }
    Val::NONE
  }

  /// What the name of the node `id` reads.
  fn name(&mut self, frame: usize, id: u32, read: &Read, at: Option<(usize, usize)>) -> Val {
    self.built_in = false;
    let unit = self.unit;
    let mut found = Val::NONE;
    if read.name != NONE && !read.probes.is_empty() {
      let name: &Name = &unit.names[read.name as usize];
      let special = name.as_str() == "item";
      // in the predicate of a filter: an entry of the item when it has one, innermost first.
      // What the name reads outside the filter is read too: the predicate is evaluated there
      // first (a number selects an item), and an item that is no context has no entries.
      for &record in read.probes.iter() {
        let item = self.frames[frame].items.get(record as usize).copied().unwrap_or(Val::NONE);
        match (self.has(item, name), special) {
          (Has::No, true) => found = self.union(found, item),
          (Has::No, false) => {}
          (_, true) => {
            // the item, or its entry of that name when it is a context that has one
            let entry = self.entry(item, name);
            let either = self.union(item, entry);
            found = self.union(found, either);
          }
          (_, false) => {
            let entry = self.entry(item, name);
            self.note_val(entry, at);
            found = self.union(found, entry);
          }
        }
      }
    }
    let probed = found.source != NONE;
    let target = match read.target {
      Target::Place(place) => self.held(frame, place),
      Target::Free(free) => self.outside(frame, unit.frees[free as usize].name, at, probed),
      Target::Search if read.name != NONE => 'found: {
        let mut region = unit.nodes[id as usize].region;
        while region != NONE {
          let Region { parent, kind } = unit.regions[region as usize];
          match kind {
            RegionKind::Layout { layout, visible } => {
              if let Some(place) = unit.layouts[layout as usize].find(read.name, visible) {
                break 'found self.held(frame, place);
              }
            }
            RegionKind::Item { .. } => {}
            RegionKind::Closure => break,
          }
          region = parent;
        }
        self.outside(frame, read.name, at, probed)
      }
      Target::Search => Val::NONE,
    };
    if found.source == NONE { target } else { self.union(found, target) }
  }

  fn held(&mut self, frame: usize, place: Place) -> Val {
    match place {
      Place::Slot(slot) => self.slot(frame, slot).map_or(Val::NONE, |held| if *held == Val::UNSET { Val::NONE } else { *held }),
      Place::Partial(record) => {
        let read = (frame as u32, record);
        if !self.partials.contains(&read) {
          self.partials.push(read);
        }
        Val::NONE
      }
    }
  }

  /// What the binder `place` of the frame `frame` holds for the function definition `def`
  /// that is written there: what it held when its construct ended, when it has ended.
  /// None for an entry of a context literal that has not its value yet.
  fn held_for(&mut self, def: u32, frame: usize, place: Place) -> Option<Val> {
    if let Place::Slot(slot) = place {
      let held = match self.defs.get(def as usize).and_then(|def| def.kept.get(&slot)) {
        Some(kept) => *kept,
        None => self.slot(frame, slot).map_or(Val::NONE, |held| *held),
      };
      return (held != Val::UNSET).then_some(held);
    }
    Some(self.held(frame, place))
  }

  /// The construct `id` of the frame `frame` ends: the function definitions written in it
  /// (those from `from` on) keep what its binders hold, and the item of a filter.
  fn keep(&mut self, frame: usize, id: u32, from: usize) {
    if self.defs.len() == from {
      return;
    }
    let unit = self.unit;
    let mut slots: Vec<(u32, Val)> = vec![];
    let mut item: Option<(u32, Val)> = None;
    match &unit.nodes[id as usize].info {
      Info::Context { layout, .. } | Info::Iteration { layout, .. } => {
        for &(_, place) in unit.layouts[*layout as usize].entries.iter() {
          if let Place::Slot(slot) = place {
            let held = self.slot(frame, slot).map_or(Val::NONE, |held| *held);
            slots.push((slot, held));
          }
        }
      }
      Info::Filter { record } => {
        let held = self.frames[frame].items.get(*record as usize).copied().unwrap_or(Val::NONE);
        item = Some((*record, held));
      }
      _ => return,
    }
    for def in self.defs[from..].iter_mut().filter(|def| def.frame as usize == frame) {
      for &(slot, held) in &slots {
        def.kept.entry(slot).or_insert(held);
      }
      if let Some((record, held)) = item {
        def.kept_items.entry(record).or_insert(held);
      }
    }
  }

  /// What the name `name` reads that the body of the frame `frame` does not bind: the nearest
  /// binder where the function is written, else the name of the scope; on the way out, an
  /// entry of the item of a filter that the function is written in, which the item can have.
  /// With `optional`, a name that the scope has not is no unknown name (an entry of an item is
  /// read).
  fn outside(&mut self, mut frame: usize, name: u32, at: Option<(usize, usize)>, optional: bool) -> Val {
    let unit = self.unit;
    let mut found = Val::NONE;
    loop {
      match self.frames[frame].outer {
        Outer::Root => {
          let name: &Name = &unit.names[name as usize];
          let read = if name.as_str() == "?" || self.typing.value(name).is_some() {
            self.root(name, at)
          } else if built_in(name.as_str()).is_some() {
            self.built_in = true;
            Val::NONE
          } else if !self.known {
            self.root(name, at)
          } else {
            if !optional && found == Val::NONE {
              match self.unknown.iter_mut().find(|(known, _)| known == name) {
                Some((_, places)) => places.extend(at.filter(|at| !places.contains(at))),
                None => self.unknown.push((name.clone(), at.into_iter().collect())),
              }
            }
            Val::NONE
          };
          // a function definition stays one when nothing else can be read
          return if found == Val::NONE {
            read
          } else if read == Val::NONE {
            found
          } else {
            self.union(found, read)
          };
        }
        Outer::Site { frame: site, region, def } => {
          let site = site as usize;
          let mut region = region;
          while region != NONE {
            let Region { parent, kind } = unit.regions[region as usize];
            match kind {
              RegionKind::Layout { layout, visible } => {
                let layout = &unit.layouts[layout as usize];
                let visible = if layout.kind == LayoutKind::Context { layout.entries.len() as u32 } else { visible };
                // the nearest binder of the name that has its value is what the body reads
                if let Some(place) = layout.find(name, visible)
                  && let Some(held) = self.held_for(def, site, place)
                {
                  return if found == Val::NONE { held } else { self.union(found, held) };
                }
              }
              // written in the predicate of a filter: an entry of its item can be read
              RegionKind::Item { record } => {
                let item = match self.defs.get(def as usize).and_then(|def| def.kept_items.get(&record)) {
                  Some(kept) => *kept,
                  None => self.frames[site].items.get(record as usize).copied().unwrap_or(Val::NONE),
                };
                let name: &Name = &unit.names[name as usize];
                let either = if name.as_str() == "item" {
                  let entry = self.entry(item, name);
                  self.union(item, entry)
                } else {
                  self.entry(item, name)
                };
                found = self.union(found, either);
              }
              RegionKind::Closure => break,
            }
            region = parent;
          }
          frame = site;
        }
      }
    }
  }

  /// A context literal: its entries, each read as far as the context is.
  fn context(&mut self, frame: usize, id: u32) -> Val {
    let unit = self.unit;
    let kids = unit.kids(id);
    let layout = match &unit.nodes[id as usize].info {
      Info::Context { layout, .. } => &unit.layouts[*layout as usize],
      _ => return self.operands(frame, kids),
    };
    for &(_, place) in layout.entries.iter() {
      self.set(frame, place, Val::UNSET);
    }
    let mut entries = vec![];
    let from = self.defs.len();
    for (position, &entry) in kids.iter().enumerate() {
      let (AstNode::ContextEntry(key, _), &[_, value]) = (unit.nodes[entry as usize].ast, unit.kids(entry)) else {
        self.read(frame, entry);
        continue;
      };
      let AstNode::ContextEntryKey(name) = key.as_ref() else {
        self.read(frame, entry);
        continue;
      };
      let val = self.flow(frame, value);
      entries.push((name.clone(), val));
      if let Some(&(_, place)) = layout.entries.get(position) {
        self.set(frame, place, val);
      }
    }
    self.keep(frame, id, from);
    Val {
      source: self.source(Source::Record(entries)),
      lift: 0,
      def: NONE,
    }
  }

  /// The variables of a `for`, `some` or `every` with the items of their domains; returns the
  /// node of the body.
  fn iteration(&mut self, frame: usize, id: u32, contexts: &AstNode, quantified: bool) -> Option<u32> {
    let unit = self.unit;
    let kids = unit.kids(id);
    let layout = match &unit.nodes[id as usize].info {
      Info::Iteration { layout, .. } => &unit.layouts[*layout as usize],
      _ => {
        self.operands(frame, kids);
        return None;
      }
    };
    for &(_, place) in layout.entries.iter() {
      self.set(frame, place, Val::NONE);
    }
    let mut kids = kids.iter().copied();
    for (level, (_, domain)) in variables(contexts, quantified).iter().enumerate() {
      let variable = match domain.len() {
        // the numbers of an interval
        2 => {
          for _ in 0..2 {
            if let Some(end) = kids.next() {
              self.read(frame, end);
            }
          }
          Val::NONE
        }
        1 => {
          let values = kids.next()?;
          let values = self.flow(frame, values);
          self.items(values)
        }
        _ => Val::NONE,
      };
      if let Some(&(_, place)) = layout.entries.get(level) {
        self.set(frame, place, variable);
      }
    }
    let body = kids.next();
    // what is left is read whole
    for kid in kids {
      self.read(frame, kid);
    }
    body
  }

  fn invocation(&mut self, frame: usize, id: u32, written: &AstNode) -> Val {
    let unit = self.unit;
    let Some((&callee, values)) = unit.kids(id).split_first() else {
      return Val::NONE;
    };
    let callee_node = &unit.nodes[callee as usize];
    let function = self.flow(frame, callee);
    // a built-in function: the name reads nothing else (no entry of an item of a filter either)
    let named_built_in = match callee_node.ast {
      AstNode::Name(name) if self.built_in && function == Val::NONE => Some(name.as_str()),
      _ => None,
    };
    self.built_in = false;
    let arguments: Vec<Val> = values.iter().map(|&value| self.flow(frame, value)).collect();
    // a function definition of the text: its body with the sources of the arguments
    if function.def != NONE && !self.defs[function.def as usize].escaped {
      let Def { node, frame: site, .. } = self.defs[function.def as usize];
      if self.active.contains(&node) {
        // it invokes itself: what this invocation gives is what its body gives, which the
        // walk of the body that is open reads whole when it ends
        self.defs[function.def as usize].recursive = true;
        for argument in arguments {
          self.whole(argument);
        }
        return Val::NONE;
      }
      if let Some(result) = self.call(function.def, node, site, &arguments, Some(written)) {
        return result;
      }
    }
    let positional = matches!(written, AstNode::PositionalParameters(_));
    let functions = arguments.iter().any(|argument| argument.def != NONE);
    if let Some(name) = named_built_in
      && positional
      && !functions
    {
      if PASSING.contains(&name) {
        return self.passed(name, &arguments);
      }
      if name == "get value"
        && let AstNode::PositionalParameters(items) = written
        && let (Some(AstNode::String(key)), &[context, _]) = (items.get(1), arguments.as_slice())
      {
        let entry = self.entry(context, &Name::from(key.as_str()));
        self.note_val(entry, (self.place)(unit.nodes[id as usize].ast));
        return entry;
      }
    }
    // another function: it can read all of its arguments
    self.whole(function);
    for argument in arguments {
      self.whole(argument);
    }
    Val::NONE
  }

  /// The result of a built-in function of [PASSING].
  fn passed(&mut self, name: &str, arguments: &[Val]) -> Val {
    let Some((&first, rest)) = arguments.split_first() else {
      return Val::NONE;
    };
    match name {
      // lists: the list of their items
      "concatenate" => {
        let mut all = first;
        for &list in rest {
          all = self.union(all, list);
        }
        all
      }
      // a list and items
      "append" => {
        let mut all = first;
        for &item in rest {
          let item = self.listed(item);
          all = self.union(all, item);
        }
        all
      }
      // a list, a position and an item
      "insert before" => match rest {
        &[position, item] => {
          self.whole(position);
          let item = self.listed(item);
          self.union(first, item)
        }
        _ => {
          for &argument in arguments {
            self.whole(argument);
          }
          Val::NONE
        }
      },
      // a list and positions
      _ => {
        for &argument in rest {
          self.whole(argument);
        }
        first
      }
    }
  }

  /// The body of the function definition `id`, reached in the frame `site`, with `arguments`
  /// (`written` as the text writes them; without, the parameters have no source). None when
  /// the definition is being walked already (it invokes itself).
  fn call(&mut self, def: u32, id: u32, site: u32, arguments: &[Val], written: Option<&AstNode>) -> Option<Val> {
    let unit = self.unit;
    let node = &unit.nodes[id as usize];
    let (AstNode::FunctionDefinition(parameters, _), Info::Function { proto }) = (node.ast, &node.info) else {
      return None;
    };
    let AstNode::FormalParameters(items) = parameters.as_ref() else {
      return None;
    };
    if *proto == NONE || self.active.contains(&id) || !self.complete {
      return None;
    }
    let named: Option<Vec<&Name>> = match written {
      Some(AstNode::NamedParameters(named)) => Some(
        named
          .iter()
          .filter_map(|argument| match argument {
            AstNode::NamedParameter(name, _) => match name.as_ref() {
              AstNode::ParameterName(name) => Some(name),
              _ => None,
            },
            _ => None,
          })
          .collect(),
      ),
      _ => None,
    };
    let proto = &unit.functions[*proto as usize];
    let mut bound: Vec<(u32, Val)> = Vec::with_capacity(items.len());
    let mut taken = vec![false; arguments.len()];
    for (position, item) in items.iter().enumerate() {
      if let AstNode::FormalParameter(parameter, _) = item
        && let AstNode::ParameterName(parameter) = parameter.as_ref()
      {
        let at = match &named {
          Some(names) => names.iter().position(|name| *name == parameter),
          None => Some(position),
        };
        let argument = at.and_then(|at| arguments.get(at).copied().inspect(|_| taken[at] = true));
        if let Some(&slot) = proto.params.get(position) {
          bound.push((slot, argument.unwrap_or(Val::NONE)));
        }
      }
    }
    // an argument that no parameter takes is evaluated all the same
    for (at, &argument) in arguments.iter().enumerate() {
      if !taken[at] {
        self.whole(argument);
      }
    }
    // a parameter has a type: the conversion to it reads the argument
    let typed = items
      .iter()
      .any(|item| !matches!(item, AstNode::FormalParameter(_, written) if matches!(written.as_ref(), AstNode::FeelType(rheon_feel::FeelType::Any))));
    if typed {
      for &(_, argument) in &bound {
        self.whole(argument);
      }
    }
    let body = self.enter(
      proto.binders,
      Outer::Site {
        frame: site,
        region: node.region,
        def,
      },
    );
    for (slot, argument) in bound {
      if let Some(held) = self.slot(body, slot) {
        *held = argument;
      }
    }
    self.active.push(id);
    let result = self.flow(body, proto.body);
    if std::mem::take(&mut self.defs[def as usize].recursive) {
      self.whole(result);
    }
    self.settle(result, body);
    self.active.pop();
    self.leave();
    Some(result)
  }

  fn finish(mut self) -> Reads {
    // a function of the scope was read: it reads the names of the context it was written in
    if self.everything {
      for name in self.typing.names() {
        let read = self.root(&name, None);
        self.whole(read);
      }
    }
    let mut paths = vec![];
    for node in &self.paths {
      // the items of a list that are only passed through are said by the paths below them
      if node.segment.is_none() && !node.whole && node.has_children {
        continue;
      }
      let mut segments = vec![];
      let mut at = Some(node);
      while let Some(path) = at {
        segments.push(match &path.segment {
          Some(name) => Segment::Entry(name.clone()),
          None => Segment::Items,
        });
        at = self.paths.get(path.parent as usize);
      }
      segments.reverse();
      let mut places: Vec<(usize, usize)> = node.places.iter().map(|&(start, end)| (start as usize, end as usize)).collect();
      places.sort_unstable();
      paths.push(ReadPath {
        segments,
        whole: node.whole,
        places,
      });
    }
    // in the order of the text, then of the paths
    paths.sort_by(|a, b| {
      let first = |path: &ReadPath| path.places.first().copied().unwrap_or((usize::MAX, usize::MAX));
      first(a).cmp(&first(b)).then_with(|| text(&a.segments).cmp(&text(&b.segments)))
    });
    let mut unknown = self.unknown;
    for (_, places) in unknown.iter_mut() {
      places.sort_unstable();
    }
    Reads {
      paths,
      unknown,
      complete: self.complete,
    }
  }
}

/// Whether `key`, a key of a context that the text makes, is the entry that `name` reads: the
/// key of this text, or a key written in another form that is the same name.
fn is_key(key: &Name, name: &Name) -> bool {
  key == name || rheon_feel::names::is_name(key.as_str(), name.as_str())
}

/// Whether the predicate of a filter selects one item (a number) or the items it is true for
/// (a comparison, a test), as far as its text says: None when it does not.
fn selects(predicate: &AstNode) -> Option<bool> {
  use AstNode as A;
  match predicate {
    A::Numeric(..) => Some(true),
    A::Neg(inner) if matches!(inner.as_ref(), A::Numeric(..)) => Some(true),
    A::Boolean(_)
    | A::Eq(..)
    | A::Nq(..)
    | A::Lt(..)
    | A::Le(..)
    | A::Gt(..)
    | A::Ge(..)
    | A::And(..)
    | A::Or(..)
    | A::In(..)
    | A::Between(..)
    | A::InstanceOf(..)
    | A::Some(..)
    | A::Every(..) => Some(false),
    _ => None,
  }
}

/// The path as text: `loans[].payment`.
pub fn text(segments: &[Segment]) -> String {
  let mut text = String::new();
  for segment in segments {
    match segment {
      Segment::Entry(name) => {
        if !text.is_empty() {
          text.push('.');
        }
        text.push_str(&name.to_string());
      }
      Segment::Items => text.push_str("[]"),
    }
  }
  text
}
