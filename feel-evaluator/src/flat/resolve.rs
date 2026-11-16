//! # Name resolution
//!
//! The pass between the parser's tree and the instructions. From the tree alone (no scope and no
//! input) it gives every binder of an expression its place and decides what every name reads:
//!
//! - a binder of the same function body (or of the root expression): its slot, or the results so
//!   far of a `for` (`partial`);
//! - inside the predicate of a filter, first the item (the special name `item`, or the item's
//!   entry of the name) while the filter visits its items, then what the name reads outside the
//!   predicate;
//! - otherwise a free name, read at run time from outside the body: from the closure of the
//!   function, the scopes of its callers and the scope of the host, then the built-in function of
//!   that name.
//!
//! The scopes of the evaluator of closures are kept as static regions: a region is one scope at
//! a position of the code (the first entries of a layout, the item of a filter, the closure of a
//! function), and the chain of a position is its region and the regions that hold it. A qualified
//! name of several segments is searched along that chain at run time, since a search that fails
//! in one scope goes on in the scope below it.
//!
//! The pass visits the nodes that the evaluator of closures evaluates, where it evaluates them,
//! and binds names only in the shapes that evaluator binds; any other shape is kept as it is,
//! unbound, for the lowering to give what that evaluator gives or to refuse it.

use rheon_feel::Name;
use rheon_feel::closure::Closure;
use rheon_feel_parser::{AstNode, ClosureBuilder};
use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// No region, record or proto: the bottom of a chain of regions.
pub(crate) const NONE: u32 = u32::MAX;

/// The results so far of a `for`.
const PARTIAL: &str = "partial";

/// The number of names up to which a name is looked up by comparing it with each.
const FEW_NAMES: usize = 16;

/// An expression after name resolution: its nodes with their bindings, and the tables they refer
/// to. Indices are `u32`, so the pass sets no limit of its own on the size of an expression.
pub(crate) struct Resolved<'a> {
  /// The nodes, each after its children.
  pub(crate) nodes: Vec<RNode<'a>>,
  /// The children of the nodes, a range per node.
  kids: Vec<u32>,
  /// The names that binders bind and that names and paths read, each once.
  pub(crate) names: Vec<Cow<'a, Name>>,
  /// The binders of each binding construct.
  pub(crate) layouts: Vec<Layout>,
  /// The scopes at the positions of the code.
  pub(crate) regions: Vec<Region>,
  /// The root expression.
  pub(crate) root: Proto,
  /// The body of every function definition.
  pub(crate) functions: Vec<Proto>,
  /// The names read from outside the bodies that read them, in the order of their first read:
  /// the report of free names.
  pub(crate) frees: Vec<Free>,
}

/// A node of the parser's tree with what the pass decided for it.
pub(crate) struct RNode<'a> {
  /// The node: the construct, as the parser built it.
  pub(crate) ast: &'a AstNode,
  /// The first of its children in [Resolved::kids].
  first: u32,
  /// The number of its children.
  len: u32,
  /// The innermost region at its position.
  pub(crate) region: u32,
  /// What the pass decided for it.
  pub(crate) info: Info,
}

/// What the pass decided for a node.
pub(crate) enum Info {
  /// Nothing: a node that binds and reads no name itself.
  None,
  /// A name, or a qualified name, read as a value.
  Read(Read),
  /// A context literal: the layout of its entries, and the entry at which it stops when one
  /// does for a reason the tree shows.
  Context { layout: u32, stop: Stop },
  /// A `for`, `some` or `every` expression: the layout of its variables, the record of its walk,
  /// and the name of the first variable named twice, if any (in [NONE] otherwise).
  Iteration { layout: u32, record: u32, duplicate: u32 },
  /// A filter: the record that holds its item.
  Filter { record: u32 },
  /// A function definition: the proto of its parameters and body (in [Resolved::functions]),
  /// [NONE] when its shape is not the one of a definition.
  Function { proto: u32 },
  /// A path or a descendant: the name of the entries it reads.
  Entry { name: u32 },
}

/// Where a context literal stops for a reason the tree shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stop {
  /// It does not: every entry is bound.
  None,
  /// At this entry, whose key an earlier entry has.
  Duplicate(u32),
  /// At this entry, which is no entry with a key.
  Irregular(u32),
}

/// What a name reads.
pub(crate) struct Read {
  /// The records of the filters whose items are probed first, innermost first.
  pub(crate) probes: Box<[u32]>,
  /// What it reads when no item has it.
  pub(crate) target: Target,
}

/// What a name reads when no item of a filter has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
  /// A binder of the same body.
  Place(Place),
  /// A free name ([Free]).
  Free(u32),
  /// The chain of the position, searched at run time (a qualified name of several segments, the
  /// first of which is `first`, or of none: [NONE]).
  Search { first: u32 },
}

/// Where a binder is kept while the code that sees it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
  /// A slot of the body's frame.
  Slot(u32),
  /// The results so far of the `for` whose walk is this record.
  Partial(u32),
}

/// The binders of one binding construct, in binding order.
pub(crate) struct Layout {
  pub(crate) kind: LayoutKind,
  /// The name and the place of each binder.
  pub(crate) entries: Box<[(u32, Place)]>,
  /// The entries sorted by name, then by position.
  by_name: Box<[u32]>,
  /// A name that the regions which do not see every entry do not see at all: `partial` in a
  /// `for`, which the walk removes before every later domain, a variable of that name included.
  hidden: u32,
}

/// The construct that a layout binds the names of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutKind {
  Context,
  For,
  Quantified,
  Params,
}

impl Layout {
  fn new(kind: LayoutKind, entries: Vec<(u32, Place)>, hidden: u32) -> Self {
    let mut by_name: Vec<u32> = (0..entries.len() as u32).collect();
    by_name.sort_by_key(|&index| (entries[index as usize].0, index));
    Self {
      kind,
      entries: entries.into(),
      by_name: by_name.into(),
      hidden,
    }
  }

  /// The entries that the first `visible` entries show, in binding order: all of them, but the
  /// hidden name where not every entry is visible.
  pub(crate) fn visible(&self, visible: u32) -> impl Iterator<Item = &(u32, Place)> {
    let hide = (visible as usize) < self.entries.len();
    self.entries[..visible as usize].iter().filter(move |(name, _)| !hide || *name != self.hidden)
  }

  /// The place of `name` among the first `visible` entries: the last one of that name, as the
  /// scope of the evaluator of closures keeps the last value set under a name.
  pub(crate) fn find(&self, name: u32, visible: u32) -> Option<Place> {
    if name == self.hidden && (visible as usize) < self.entries.len() {
      return None;
    }
    let start = self.by_name.partition_point(|&index| self.entries[index as usize].0 < name);
    let end = start + self.by_name[start..].partition_point(|&index| self.entries[index as usize].0 == name && index < visible);
    (end > start).then(|| self.entries[self.by_name[end - 1] as usize].1)
  }
}

/// One scope at a position of the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Region {
  /// The region that holds it, [NONE] at the bottom of a body.
  pub(crate) parent: u32,
  pub(crate) kind: RegionKind,
}

/// What a region holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegionKind {
  /// The first `visible` entries of a layout.
  Layout { layout: u32, visible: u32 },
  /// The item of the filter whose walk is this record, while the filter visits its items: the
  /// special name `item` and the entries of an item that is a context.
  Item { record: u32 },
  /// The closure of the function whose body runs.
  Closure,
}

/// The root expression or the body of a function definition: what runs in one frame.
pub(crate) struct Proto {
  /// The slot of each parameter, in the order of the parameters (parameters of one name share
  /// one, which holds the last of them).
  pub(crate) params: Box<[u32]>,
  /// The slots that parameters and binders use.
  pub(crate) binders: u32,
  /// The outermost region of the body.
  pub(crate) region: u32,
  /// The names the function closes over, as the evaluator of closures lists them.
  pub(crate) closure: Closure,
  /// The body, [NONE] for a function whose body is external.
  pub(crate) body: u32,
}

/// A name read from outside the body that reads it.
pub(crate) struct Free {
  pub(crate) name: u32,
  /// Read as a qualified name: no built-in function stands for it.
  pub(crate) qualified: bool,
  /// Where it is read ([uses]).
  #[cfg_attr(not(test), allow(dead_code))]
  pub(crate) uses: u8,
}

/// Where a free name is read, a bit each.
pub(crate) mod uses {
  /// In the root expression: from the input.
  pub(crate) const ROOT: u8 = 1;
  /// In the body of a function: from the closure or the scopes of its callers first.
  pub(crate) const FUNCTION_BODY: u8 = 2;
  /// In the predicate of a filter: from the item first.
  pub(crate) const FILTER_PREDICATE: u8 = 4;
  /// As the function of an invocation.
  pub(crate) const CALLEE: u8 = 8;
}

impl<'a> Resolved<'a> {
  /// The children of `node`.
  pub(crate) fn kids(&self, node: u32) -> &[u32] {
    let node = &self.nodes[node as usize];
    &self.kids[node.first as usize..(node.first + node.len) as usize]
  }

  /// The node of the root expression.
  pub(crate) fn root(&self) -> u32 {
    self.root.body
  }
}

/// Resolves the names of the expression `ast`.
pub(crate) fn resolve(ast: &AstNode) -> Resolved<'_> {
  // every node of the tree is resolved once, and is the child of at most one
  let size = count(ast);
  let mut resolver = Resolver {
    out: Resolved {
      nodes: Vec::with_capacity(size),
      kids: Vec::with_capacity(size - 1),
      names: vec![],
      layouts: vec![],
      regions: vec![],
      root: Proto {
        params: Box::new([]),
        binders: 0,
        region: NONE,
        closure: Closure::default(),
        body: NONE,
      },
      functions: vec![],
      frees: vec![],
    },
    interned: HashMap::new(),
    partial: NONE,
    free_names: vec![],
    body: Body {
      proto: NONE,
      region: NONE,
      records: 0,
      next: 0,
      max: 0,
    },
  };
  let root = resolver.node(ast);
  resolver.out.root.body = root;
  resolver.out.root.binders = resolver.body.max;
  resolver.out
}

/// The number of nodes of the tree `ast`.
fn count(ast: &AstNode) -> usize {
  let mut nodes = 1;
  ast.for_each_child(|child| nodes += count(child));
  nodes
}

/// The state of the body being resolved.
#[derive(Clone, Copy)]
struct Body {
  /// The function whose body it is, [NONE] for the root expression.
  proto: u32,
  /// The innermost region at the position being resolved.
  region: u32,
  /// The records open at that position (walks of `for`, `some`, `every` and filters).
  records: u32,
  /// The first free slot.
  next: u32,
  /// The slots used so far.
  max: u32,
}

struct Resolver<'a> {
  out: Resolved<'a>,
  /// The names of the tree, each with its index, once there are more than a few (the standard
  /// map, whose hasher is keyed at random, so no input makes its lookups slow).
  interned: HashMap<&'a Name, u32>,
  /// The index of `partial`, which a `for` binds without a node of the tree, or [NONE].
  partial: u32,
  /// The free name of each name, read as a name and as a qualified name, or [NONE].
  free_names: Vec<[u32; 2]>,
  body: Body,
}

impl<'a> Resolver<'a> {
  fn name(&mut self, name: &'a Name) -> u32 {
    if Borrow::<str>::borrow(name) == PARTIAL {
      return self.partial();
    }
    // a few names are found by comparing them with each, more in a map ([Resolver::add_name])
    let known = if self.interned.is_empty() {
      self.out.names.iter().position(|known| **known == *name).map(|index| index as u32)
    } else {
      self.interned.get(name).copied()
    };
    known.unwrap_or_else(|| self.add_name(Cow::Borrowed(name)))
  }

  /// The index of `partial`.
  fn partial(&mut self) -> u32 {
    if self.partial == NONE {
      self.partial = self.add_name(Cow::Owned(Name::from(PARTIAL)));
    }
    self.partial
  }

  /// Adds a name; beyond a few names, every name of the tree is in the map too (`partial`, which
  /// is not, is never looked up by [Resolver::name]).
  fn add_name(&mut self, name: Cow<'a, Name>) -> u32 {
    let index = self.out.names.len() as u32;
    self.out.names.push(name);
    self.free_names.push([NONE; 2]);
    if self.out.names.len() > FEW_NAMES {
      let interned = &mut self.interned;
      let names = &self.out.names;
      let new = if interned.is_empty() { 0 } else { index as usize };
      for (index, known) in names.iter().enumerate().skip(new) {
        if let Cow::Borrowed(known) = known {
          interned.insert(known, index as u32);
        }
      }
    }
    index
  }

  fn add(&mut self, ast: &'a AstNode, info: Info, kids: &[u32]) -> u32 {
    let first = self.out.kids.len() as u32;
    self.out.kids.extend_from_slice(kids);
    self.out.nodes.push(RNode {
      ast,
      first,
      len: kids.len() as u32,
      region: self.body.region,
      info,
    });
    self.out.nodes.len() as u32 - 1
  }

  fn region(&mut self, parent: u32, kind: RegionKind) -> u32 {
    self.out.regions.push(Region { parent, kind });
    self.out.regions.len() as u32 - 1
  }

  fn layout(&mut self, kind: LayoutKind, entries: Vec<(u32, Place)>, hidden: u32) -> u32 {
    self.out.layouts.push(Layout::new(kind, entries, hidden));
    self.out.layouts.len() as u32 - 1
  }

  fn slot(&mut self) -> u32 {
    let slot = self.body.next;
    self.body.next += 1;
    self.body.max = self.body.max.max(self.body.next);
    slot
  }

  /// What `name` reads at the position being resolved.
  fn read(&mut self, name: &'a Name, qualified: bool, callee: bool) -> Read {
    let name = self.name(name);
    let mut probes = vec![];
    let mut region = self.body.region;
    while region != NONE {
      let Region { parent, kind } = self.out.regions[region as usize];
      match kind {
        RegionKind::Layout { layout, visible } => {
          if let Some(place) = self.out.layouts[layout as usize].find(name, visible) {
            return Read {
              probes: probes.into(),
              target: Target::Place(place),
            };
          }
        }
        RegionKind::Item { record } => probes.push(record),
        RegionKind::Closure => break,
      }
      region = parent;
    }
    let mut used = if self.body.proto == NONE { uses::ROOT } else { uses::FUNCTION_BODY };
    if !probes.is_empty() {
      used |= uses::FILTER_PREDICATE;
    }
    if callee {
      used |= uses::CALLEE;
    }
    Read {
      probes: probes.into(),
      target: Target::Free(self.free(name, qualified, used)),
    }
  }

  fn free(&mut self, name: u32, qualified: bool, used: u8) -> u32 {
    let mut index = self.free_names[name as usize][qualified as usize];
    if index == NONE {
      index = self.out.frees.len() as u32;
      self.free_names[name as usize][qualified as usize] = index;
      self.out.frees.push(Free { name, qualified, uses: 0 });
    }
    self.out.frees[index as usize].uses |= used;
    index
  }

  fn nodes(&mut self, asts: &'a [AstNode]) -> Vec<u32> {
    asts.iter().map(|ast| self.node(ast)).collect()
  }

  fn node(&mut self, ast: &'a AstNode) -> u32 {
    use AstNode::*;
    match ast {
      Name(name) => {
        let read = self.read(name, false, false);
        self.add(ast, Info::Read(read), &[])
      }
      QualifiedName(segments) => self.qualified_name(ast, segments),
      Context(entries) => self.context(ast, entries),
      For(contexts, body) => self.for_expression(ast, contexts, body),
      Some(contexts, satisfies) | Every(contexts, satisfies) => self.quantified(ast, contexts, satisfies),
      Filter(list, predicate) => self.filter(ast, list, predicate),
      FunctionDefinition(parameters, body) => self.function_definition(ast, parameters, body),
      FunctionInvocation(callee, arguments) => self.invocation(ast, callee, arguments),
      // the name after the dot is the name of an entry, not a name read
      Path(base, name) | Descendant(base, name) => {
        if let Name(name) = name.as_ref() {
          let name = self.name(name);
          let base = self.node(base);
          self.add(ast, Info::Entry { name }, &[base])
        } else {
          self.add(ast, Info::None, &[])
        }
      }
      Add(a, b)
      | Sub(a, b)
      | Mul(a, b)
      | Div(a, b)
      | Exp(a, b)
      | Eq(a, b)
      | Nq(a, b)
      | Lt(a, b)
      | Le(a, b)
      | Gt(a, b)
      | Ge(a, b)
      | And(a, b)
      | Or(a, b)
      | In(a, b)
      | Out(a, b)
      | InstanceOf(a, b)
      | Range(a, b)
      | ContextEntry(a, b)
      | ContextTypeEntry(a, b)
      | FormalParameter(a, b)
      | FunctionType(a, b) => {
        let a = self.node(a);
        let b = self.node(b);
        self.add(ast, Info::None, &[a, b])
      }
      Between(a, b, c) | If(a, b, c) => {
        let a = self.node(a);
        let b = self.node(b);
        let c = self.node(c);
        self.add(ast, Info::None, &[a, b, c])
      }
      Neg(a)
      | UnaryGe(a)
      | UnaryGt(a)
      | UnaryLe(a)
      | UnaryLt(a)
      | UnaryEq(a)
      | UnaryNe(a)
      | IntervalStart(a, _)
      | IntervalEnd(a, _)
      | ListType(a)
      | RangeType(a)
      | EvaluatedExpression(a)
      | FunctionBody(a, _) => {
        let a = self.node(a);
        self.add(ast, Info::None, &[a])
      }
      List(items) | ExpressionList(items) | NegatedList(items) | ParameterTypes(items) | ContextType(items) | FormalParameters(items) => {
        let kids = self.nodes(items);
        self.add(ast, Info::None, &kids)
      }
      // literals, keys and types; and the nodes that the evaluator of closures does not evaluate
      // but builds as an error, so their children are never read
      At(_)
      | Boolean(_)
      | ContextEntryKey(_)
      | ContextTypeEntryKey(_)
      | FeelType(_)
      | Irrelevant
      | Null
      | Numeric(..)
      | ParameterName(_)
      | QualifiedNameSegment(_)
      | String(_)
      | CommaList(_)
      | IterationContexts(_)
      | IterationContextSingle(..)
      | IterationContextInterval(..)
      | NamedParameter(..)
      | NamedParameters(_)
      | PositionalParameters(_)
      | QuantifiedContext(..)
      | QuantifiedContexts(_)
      | Satisfies(_) => self.add(ast, Info::None, &[]),
    }
  }

  /// A qualified name: of one segment it reads as a name, of several it is searched along the
  /// chain of its position at run time. A segment that is no segment is kept as a child: the
  /// evaluator of closures evaluates it.
  fn qualified_name(&mut self, ast: &'a AstNode, segments: &'a [AstNode]) -> u32 {
    let names: Option<Vec<&Name>> = segments
      .iter()
      .map(|segment| match segment {
        AstNode::QualifiedNameSegment(name) => Some(name),
        _ => None,
      })
      .collect();
    match names.as_deref() {
      Some([name]) => {
        let read = self.read(name, true, false);
        self.add(ast, Info::Read(read), &[])
      }
      Some(names) => {
        let first = match names.first() {
          Some(first) => {
            let first = self.name(first);
            let used = if self.body.proto == NONE { uses::ROOT } else { uses::FUNCTION_BODY };
            self.free(first, true, used);
            first
          }
          None => NONE,
        };
        let read = Read {
          probes: Box::new([]),
          target: Target::Search { first },
        };
        self.add(ast, Info::Read(read), &[])
      }
      None => {
        let kids = self.nodes(segments);
        self.add(ast, Info::None, &kids)
      }
    }
  }

  /// A context literal: an entry's key is seen by the entries after it, not by its own value.
  fn context(&mut self, ast: &'a AstNode, entries: &'a [AstNode]) -> u32 {
    let mark = self.body.next;
    let mut seen = BTreeSet::new();
    let mut bound = vec![];
    let mut stop = Stop::None;
    for (index, entry) in entries.iter().enumerate() {
      let AstNode::ContextEntry(key, _) = entry else {
        stop = Stop::Irregular(index as u32);
        break;
      };
      let AstNode::ContextEntryKey(name) = key.as_ref() else {
        stop = Stop::Irregular(index as u32);
        break;
      };
      let name = self.name(name);
      if !seen.insert(name) {
        stop = Stop::Duplicate(index as u32);
        break;
      }
      let slot = self.slot();
      bound.push((name, Place::Slot(slot)));
    }
    let count = bound.len() as u32;
    let layout = self.layout(LayoutKind::Context, bound, NONE);
    let outer = self.body.region;
    let mut kids = Vec::with_capacity(entries.len());
    let mut region = NONE;
    for (index, entry) in entries.iter().enumerate() {
      let visible = (index as u32).min(count);
      if visible == index as u32 || region == NONE {
        region = self.region(outer, RegionKind::Layout { layout, visible });
      }
      self.body.region = region;
      kids.push(self.node(entry));
    }
    self.body.region = outer;
    self.body.next = mark;
    self.add(ast, Info::Context { layout, stop }, &kids)
  }

  /// The variables of a `for`, `some` or `every` expression whose shapes bind them: the name of
  /// each and its domain.
  fn variables(contexts: &'a AstNode, quantified: bool) -> Vec<(&'a Name, Vec<&'a AstNode>)> {
    let mut variables = vec![];
    match contexts {
      AstNode::IterationContexts(items) if !quantified => {
        for item in items {
          match item {
            AstNode::IterationContextInterval(name, start, end) => {
              if let AstNode::Name(name) = name.as_ref() {
                variables.push((name, vec![start.as_ref(), end.as_ref()]));
              }
            }
            AstNode::IterationContextSingle(name, domain) => {
              if let AstNode::Name(name) = name.as_ref() {
                variables.push((name, vec![domain.as_ref()]));
              }
            }
            _ => {}
          }
        }
      }
      AstNode::QuantifiedContexts(items) if quantified => {
        for item in items {
          if let AstNode::QuantifiedContext(name, domain) = item
            && let AstNode::Name(name) = name.as_ref()
          {
            variables.push((name, vec![domain.as_ref()]));
          }
        }
      }
      _ => {}
    }
    variables
  }

  /// The walk of a `for`, `some` or `every` expression: the domain of the first variable is
  /// evaluated outside it, the domain of each later variable sees the variables before it, and
  /// the body sees them all and, in a `for`, `partial`, the results so far.
  fn iteration(&mut self, ast: &'a AstNode, variables: Vec<(&'a Name, Vec<&'a AstNode>)>, body: &'a AstNode, kind: LayoutKind) -> u32 {
    let mark = self.body.next;
    let record = self.body.records;
    let mut slots: BTreeMap<u32, u32> = BTreeMap::new();
    let mut entries = vec![];
    let mut duplicate = NONE;
    for (name, _) in &variables {
      let name = self.name(name);
      let slot = match slots.get(&name) {
        Some(&slot) => {
          if duplicate == NONE {
            duplicate = name;
          }
          slot
        }
        None => {
          let slot = self.slot();
          slots.insert(name, slot);
          slot
        }
      };
      entries.push((name, Place::Slot(slot)));
    }
    let hidden = if kind == LayoutKind::For {
      let partial = self.partial();
      entries.push((partial, Place::Partial(record)));
      partial
    } else {
      NONE
    };
    let count = entries.len() as u32;
    let layout = self.layout(kind, entries, hidden);
    let outer = self.body.region;
    let mut kids = vec![];
    for (level, (_, domain)) in variables.iter().enumerate() {
      if level > 0 {
        self.body.region = self.region(outer, RegionKind::Layout { layout, visible: level as u32 });
        self.body.records = record + 1;
      }
      for part in domain {
        kids.push(self.node(part));
      }
    }
    self.body.region = self.region(outer, RegionKind::Layout { layout, visible: count });
    self.body.records = record + 1;
    kids.push(self.node(body));
    self.body.region = outer;
    self.body.records = record;
    self.body.next = mark;
    self.add(ast, Info::Iteration { layout, record, duplicate }, &kids)
  }

  fn for_expression(&mut self, ast: &'a AstNode, contexts: &'a AstNode, body: &'a AstNode) -> u32 {
    let variables = Self::variables(contexts, false);
    self.iteration(ast, variables, body, LayoutKind::For)
  }

  /// A `some` or `every` expression; one whose parts are not of their shapes is evaluated as an
  /// error, and nothing in it is read.
  fn quantified(&mut self, ast: &'a AstNode, contexts: &'a AstNode, satisfies: &'a AstNode) -> u32 {
    match (contexts, satisfies) {
      (AstNode::QuantifiedContexts(_), AstNode::Satisfies(body)) => {
        let variables = Self::variables(contexts, true);
        self.iteration(ast, variables, body, LayoutKind::Quantified)
      }
      _ => self.add(ast, Info::None, &[]),
    }
  }

  /// A filter: the list is evaluated outside it; its predicate sees the item while the filter
  /// visits its items.
  fn filter(&mut self, ast: &'a AstNode, list: &'a AstNode, predicate: &'a AstNode) -> u32 {
    let list = self.node(list);
    let record = self.body.records;
    let outer = self.body.region;
    self.body.region = self.region(outer, RegionKind::Item { record });
    self.body.records = record + 1;
    let predicate = self.node(predicate);
    self.body.region = outer;
    self.body.records = record;
    self.add(ast, Info::Filter { record }, &[list, predicate])
  }

  /// A function definition: the types of its parameters and the mapping of an external body are
  /// evaluated where the definition is; a FEEL body is a proto of its own, which sees its
  /// parameters, then its closure, and reads every other name from outside.
  fn function_definition(&mut self, ast: &'a AstNode, parameters: &'a AstNode, body: &'a AstNode) -> u32 {
    let names: Option<Vec<&Name>> = match parameters {
      AstNode::FormalParameters(items) => items
        .iter()
        .map(|item| match item {
          AstNode::FormalParameter(name, _) => match name.as_ref() {
            AstNode::ParameterName(name) => Some(name),
            _ => None,
          },
          _ => None,
        })
        .collect(),
      _ => None,
    };
    let parameters_kid = self.node(parameters);
    let (proto, body_kid) = match (names, body) {
      (Some(names), AstNode::FunctionBody(expression, external)) => {
        let proto = self.out.functions.len() as u32;
        self.out.functions.push(Proto {
          params: Box::new([]),
          binders: 0,
          region: NONE,
          closure: ClosureBuilder::from_function_definition(parameters, body),
          body: NONE,
        });
        let body_kid = if *external {
          self.node(body)
        } else {
          let outer = self.body;
          self.body = Body {
            proto,
            region: NONE,
            records: 0,
            next: 0,
            max: 0,
          };
          let mut slots: BTreeMap<u32, u32> = BTreeMap::new();
          let mut entries = vec![];
          let mut params = vec![];
          for name in names {
            let name = self.name(name);
            let slot = match slots.get(&name) {
              Some(&slot) => slot,
              None => {
                let slot = self.slot();
                slots.insert(name, slot);
                slot
              }
            };
            entries.push((name, Place::Slot(slot)));
            params.push(slot);
          }
          let visible = entries.len() as u32;
          let layout = self.layout(LayoutKind::Params, entries, NONE);
          let closure = self.region(NONE, RegionKind::Closure);
          let region = self.region(closure, RegionKind::Layout { layout, visible });
          self.body.region = region;
          let expression = self.node(expression);
          let proto_entry = &mut self.out.functions[proto as usize];
          proto_entry.params = params.into();
          proto_entry.binders = self.body.max;
          proto_entry.region = region;
          proto_entry.body = expression;
          self.body = outer;
          self.add(body, Info::None, &[expression])
        };
        (proto, body_kid)
      }
      _ => (NONE, self.node(body)),
    };
    self.add(ast, Info::Function { proto }, &[parameters_kid, body_kid])
  }

  /// An invocation: the function, then the arguments in the order written (for named arguments,
  /// their values).
  fn invocation(&mut self, ast: &'a AstNode, callee: &'a AstNode, arguments: &'a AstNode) -> u32 {
    let mut kids = vec![match callee {
      AstNode::Name(name) => {
        let read = self.read(name, false, true);
        self.add(callee, Info::Read(read), &[])
      }
      _ => self.node(callee),
    }];
    match arguments {
      AstNode::PositionalParameters(items) => kids.extend(self.nodes(items)),
      AstNode::NamedParameters(items) => {
        for item in items {
          if let AstNode::NamedParameter(_, value) = item {
            kids.push(self.node(value));
          }
        }
      }
      _ => {}
    }
    self.add(ast, Info::None, &kids)
  }
}

#[cfg(test)]
impl Resolved<'_> {
  /// A stable text of the resolved expression: its nodes, each under its parent, then its tables.
  pub(crate) fn dump(&self) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    self.dump_node(&mut text, self.root(), 0);
    for (index, proto) in self.functions.iter().enumerate() {
      let _ = writeln!(
        text,
        "proto {index}: params {:?}, binders {}, region {}, closure {}",
        proto.params,
        proto.binders,
        self.index(proto.region),
        proto.closure
      );
      if proto.body != NONE {
        self.dump_node(&mut text, proto.body, 1);
      }
    }
    let _ = writeln!(text, "root binders {}", self.root.binders);
    for (index, layout) in self.layouts.iter().enumerate() {
      let entries: Vec<String> = layout
        .entries
        .iter()
        .map(|(name, place)| format!("{}={}", self.names[*name as usize], place_text(*place)))
        .collect();
      let _ = writeln!(text, "layout {index} {:?}: {}", layout.kind, entries.join(" "));
    }
    for (index, region) in self.regions.iter().enumerate() {
      let kind = match region.kind {
        RegionKind::Layout { layout, visible } => format!("layout {layout} visible {visible}"),
        RegionKind::Item { record } => format!("item r{record}"),
        RegionKind::Closure => "closure".to_string(),
      };
      let _ = writeln!(text, "region {index}: {kind} in {}", self.index(region.parent));
    }
    for (index, free) in self.frees.iter().enumerate() {
      let _ = writeln!(
        text,
        "free {index}: {}{} uses {:04b}",
        self.names[free.name as usize],
        if free.qualified { " (qualified)" } else { "" },
        free.uses
      );
    }
    text
  }

  fn index(&self, index: u32) -> String {
    if index == NONE { "-".to_string() } else { index.to_string() }
  }

  fn dump_node(&self, text: &mut String, node: u32, depth: usize) {
    use std::fmt::Write;
    let rnode = &self.nodes[node as usize];
    let debug = format!("{:?}", rnode.ast);
    let kind = debug.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or_default();
    let _ = write!(text, "{:indent$}{kind}", "", indent = depth * 2);
    match rnode.ast {
      AstNode::Name(name) => {
        let _ = write!(text, " {name}");
      }
      AstNode::QualifiedName(segments) => {
        let names: Vec<String> = segments.iter().map(|segment| format!("{segment:?}")).collect();
        let _ = write!(text, " {}", names.join("."));
      }
      _ => {}
    }
    let _ = match &rnode.info {
      Info::None => Ok(()),
      Info::Read(read) => write!(text, ": {}", self.read_text(read)),
      Info::Context { layout, stop } => write!(text, ": layout {layout}, stop {stop:?}"),
      Info::Iteration { layout, record, duplicate } => write!(text, ": layout {layout}, record r{record}, duplicate {}", self.index(*duplicate)),
      Info::Filter { record } => write!(text, ": record r{record}"),
      Info::Function { proto } => write!(text, ": proto {}", self.index(*proto)),
      Info::Entry { name } => write!(text, ": entry {}", self.names[*name as usize]),
    };
    let _ = writeln!(text, " @{}", self.index(rnode.region));
    for &kid in self.kids(node) {
      self.dump_node(text, kid, depth + 1);
    }
  }

  /// The text of what a name reads: `item r0, free 1`.
  pub(crate) fn read_text(&self, read: &Read) -> String {
    let mut parts: Vec<String> = read.probes.iter().map(|record| format!("item r{record}")).collect();
    parts.push(match read.target {
      Target::Place(place) => place_text(place),
      Target::Free(free) => format!("free {free}"),
      Target::Search { .. } => "search".to_string(),
    });
    parts.join(", ")
  }
}

#[cfg(test)]
fn place_text(place: Place) -> String {
  match place {
    Place::Slot(slot) => format!("slot {slot}"),
    Place::Partial(record) => format!("partial r{record}"),
  }
}
