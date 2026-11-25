//! The analysis of an expression without evaluating it: what its tree says about the values it
//! gives. Today that is the type of every node; the editor operations read it.
//!
//! The analysis walks the tree as name resolution leaves it ([crate::flat::resolve]), the pass
//! that evaluation lowers too: what a name reads, which construct binds it and where a
//! function body looks for the names it does not bind are decided there once, for evaluation
//! and for the analysis alike. The types are kept in a table ([types::Types]).
//!
//! The answer for a node is the type its value has when it gives one: an expression can always
//! give null instead (DMN 1.6 clause 10.3.2.1). Where the tree does not say the type (the
//! result of a function of the host, a name without a value, a list of items of several types)
//! the answer is `Any`. Nothing here is used by the evaluation.

mod reads;
mod types;

use crate::flat::{Info, LayoutKind, NONE, Place, Read, Region, RegionKind, Resolved, Target, resolve_names, variables};
use crate::infer::{addition, at_literal, signature_type, subtraction, type_written, written_type};
pub use reads::{ReadPath, Reads, Segment, text as path_text};
use rheon_feel::signatures::{BuiltIn, built_in};
use rheon_feel::{FeelScope, FeelType, Name};
use rheon_feel_parser::AstNode;
use std::cell::OnceCell;
use std::collections::BTreeMap;
pub use types::TypeId;
use types::{ANY, BOOLEAN, Kind, MAX_TYPE_DEPTH, NULL, NUMBER, STRING, Types};

/// The deepest tree that is typed: below it the type is `Any`. Name resolution, which the walk
/// reads, recurses over the whole tree as it does before an evaluation; the trees of the parser
/// are not deeper than its own limit.
pub(crate) const MAX_DEPTH: usize = 300;

/// The most nodes that one analysis types: the body of a function literal is typed again at
/// each invocation of it with the types of the arguments, and a text of many invocations of
/// functions that invoke each other would type their bodies without end. Beyond it a function
/// literal has the type its body has with the types of its parameters as written.
pub(crate) const MAX_STEPS: usize = 20_000;

/// The most types that the signature of a function is shown with ([Typing::shown]).
const MAX_SHOWN: usize = 256;

/// What the analysis knows of a value.
#[derive(Clone, Copy)]
struct Fact {
  /// Its type.
  ty: TypeId,
  /// The function definition it is (an index of [Analysis::defs]), [NONE] when it is none or
  /// not known to be one: an invocation of it types the body with the types of the arguments.
  def: u32,
}

impl Fact {
  const ANY: Fact = Fact { ty: ANY, def: NONE };

  fn of(ty: TypeId) -> Fact {
    Fact { ty, def: NONE }
  }
}

/// A function definition whose body an invocation types.
#[derive(Clone, Copy)]
enum Def {
  /// A definition written in a tree: its node, and the frame it was reached in.
  Written { unit: u32, node: u32, frame: u32 },
  /// The definition that the name at this index of the names bound around the expression is.
  Bound(u32),
}

/// Where the names that a body does not bind are read.
#[derive(Clone, Copy)]
enum Outer {
  /// Among the first `visible` names bound around the expression, then in the scope. A name
  /// bound after them is of no known type: it hides a name of the scope, and its own value is
  /// not typed there.
  Bound { visible: u32 },
  /// Where a function definition is written: along the regions from `region` of the frame
  /// `frame`, then where that frame reads its own.
  Site { frame: u32, region: u32 },
}

/// One body being typed: the root expression of a tree, or a function body.
#[derive(Clone, Copy)]
struct Frame {
  /// Where its slots start in [Analysis::slots]; they end where those of the next frame start.
  base: u32,
  outer: Outer,
}

/// The types of the expressions of one request of an editor, in one table: the pieces of a text
/// that an editor types share the types of the values of the scope and of each other.
pub struct Typing<'s> {
  types: Types,
  scope: &'s FeelScope,
  /// The names of the scope whose types a host states ([Typing::state]): they are read before
  /// the values of the scope.
  stated: std::collections::HashMap<Name, TypeId>,
}

impl<'s> Typing<'s> {
  /// The typing of expressions in `scope`.
  pub fn new(scope: &'s FeelScope) -> Self {
    Self {
      types: Types::new(),
      scope,
      stated: std::collections::HashMap::new(),
    }
  }

  /// Returns the type of the value of `node`, where the names of `bound` (the variables and
  /// parameters of the constructs around the expression, the innermost last) have the types
  /// given with them, and `literals` gives the function literals that names of `bound` are
  /// bound to (the index of the name in `bound`, and the literal): an invocation of such a
  /// name types the body of the literal with the types of the arguments.
  pub fn infer<'n>(&mut self, node: &'n AstNode, bound: &[(Name, TypeId)], literals: &'n [(usize, AstNode)]) -> TypeId {
    let root = resolve_names(node);
    let units: Vec<OnceCell<Resolved<'n>>> = literals.iter().map(|_| OnceCell::new()).collect();
    let mut analysis = Analysis {
      typing: self,
      bound,
      literals,
      root: &root,
      units: &units,
      frames: vec![],
      slots: vec![],
      defs: vec![],
      active: vec![],
      depth: 0,
      steps: 0,
    };
    let frame = analysis.enter(root.root.binders, Outer::Bound { visible: bound.len() as u32 });
    analysis.fact(frame, 0, root.root()).ty
  }

  /// A new declared type, with its name when it has one: a type of a host, which stands for the
  /// type it is defined as ([Typing::define]) and is itself, whatever other type is defined
  /// alike. It can be held by types made before it is defined, so types can hold each other.
  pub fn declare(&mut self, name: Option<Name>) -> TypeId {
    self.types.declare(name)
  }

  /// Defines the declared type `declared` as the type `as_type`.
  pub fn define(&mut self, declared: TypeId, as_type: TypeId) {
    self.types.define(declared, as_type);
  }

  /// States the type of the name `name` of the scope: the name is read with this type, whatever
  /// the scope holds for it.
  pub fn state(&mut self, name: Name, stated: TypeId) {
    self.stated.insert(Name::from(rheon_feel::names::name_form(name.as_str())), stated);
  }

  /// The type that the declared type `id` is defined as, None for a type that is not declared.
  pub fn defined_as(&self, id: TypeId) -> Option<TypeId> {
    self.types.defined_as(id)
  }

  /// The name of the type `id` when it is a declared type that has one.
  pub fn name(&self, id: TypeId) -> Option<&Name> {
    self.types.name(id)
  }

  /// The list type of items of the type `item`.
  pub fn list(&mut self, item: TypeId) -> TypeId {
    self.types.list(item)
  }

  /// The context type of the entries `entries`, the last of a name given twice.
  pub fn context(&mut self, entries: impl IntoIterator<Item = (Name, TypeId)>) -> TypeId {
    self.types.context(entries.into_iter().collect())
  }

  /// The function type of the parameters `parameters` and the result `result`.
  pub fn function(&mut self, parameters: Vec<TypeId>, result: TypeId) -> TypeId {
    self.types.function(parameters, result)
  }

  /// The type that `text` writes as FEEL writes a type (`number`, `list<string>`,
  /// `context<a: number>`, `function<number>->string`), where a name that is no type of FEEL is
  /// the type that `named` gives for it (a qualified name with its segments joined by `.`).
  /// None when the text is no type, or `named` has none for a name.
  pub fn written(&mut self, text: &str, named: &dyn Fn(&str) -> Option<TypeId>) -> Option<TypeId> {
    let scope = FeelScope::default();
    let node = rheon_feel_parser::parse_expression(&scope, &format!("null instance of {text}")).ok()?;
    match &node {
      AstNode::InstanceOf(value, written) if matches!(value.as_ref(), AstNode::Null) => self.written_node(written, named, 0),
      _ => None,
    }
  }

  fn written_node(&mut self, node: &AstNode, named: &dyn Fn(&str) -> Option<TypeId>, depth: usize) -> Option<TypeId> {
    if depth >= MAX_TYPE_DEPTH {
      return None;
    }
    Some(match node {
      AstNode::FeelType(feel_type) => self.types.of(feel_type),
      AstNode::ListType(item) => {
        let item = self.written_node(item, named, depth + 1)?;
        self.types.list(item)
      }
      AstNode::RangeType(endpoint) => {
        let endpoint = self.written_node(endpoint, named, depth + 1)?;
        self.types.range(endpoint)
      }
      AstNode::ContextType(entries) => {
        let mut types = BTreeMap::new();
        for entry in entries {
          let AstNode::ContextTypeEntry(key, entry_type) = entry else {
            return None;
          };
          let AstNode::ContextTypeEntryKey(name) = key.as_ref() else {
            return None;
          };
          types.insert(name.clone(), self.written_node(entry_type, named, depth + 1)?);
        }
        self.types.context(types)
      }
      AstNode::FunctionType(parameters, result) => {
        let AstNode::ParameterTypes(items) = parameters.as_ref() else {
          return None;
        };
        let parameters = items.iter().map(|item| self.written_node(item, named, depth + 1)).collect::<Option<Vec<_>>>()?;
        let result = self.written_node(result, named, depth + 1)?;
        self.types.function(parameters, result)
      }
      AstNode::QualifiedName(segments) => {
        let names: Vec<&str> = segments
          .iter()
          .map(|segment| match segment {
            AstNode::Name(name) | AstNode::QualifiedNameSegment(name) => Some(name.as_str()),
            _ => None,
          })
          .collect::<Option<_>>()?;
        named(&names.join("."))?
      }
      _ => return None,
    })
  }

  /// The type of the table that `feel_type` is.
  pub fn of(&mut self, feel_type: &FeelType) -> TypeId {
    self.types.of(feel_type)
  }

  /// The type `id` as a [FeelType].
  pub fn feel_type(&self, id: TypeId) -> FeelType {
    self.types.feel_type(id)
  }

  /// The type of the items of the list type `id`, None for a type that is no list.
  pub fn item(&self, id: TypeId) -> Option<TypeId> {
    match self.types.kind(id) {
      Kind::List(item) => Some(*item),
      _ => None,
    }
  }

  /// The type of the endpoints of the range type `id`, None for a type that is no range.
  pub fn endpoint(&self, id: TypeId) -> Option<TypeId> {
    match self.types.kind(id) {
      Kind::Range(endpoint) => Some(*endpoint),
      _ => None,
    }
  }

  /// The entries of the context type `id`, sorted by name, None for a type that is no context.
  pub fn entries(&self, id: TypeId) -> Option<&[(Name, TypeId)]> {
    match self.types.kind(id) {
      Kind::Context(entries) => Some(entries),
      _ => None,
    }
  }

  /// Returns the members that a value of the type `id` has after a dot, with their types: the
  /// entries of a context, the properties of a date, a time, a duration and a range, and for a
  /// list the members of its items, each as the list of its values.
  pub fn members(&mut self, id: TypeId) -> Vec<(String, TypeId)> {
    self.types.members(id)
  }

  /// The type `id` as an item of an editor shows it: the kind of the value, without the entries
  /// of a context (`context`, `list<context>`, `list<number>`), and a function with its
  /// signature when the types of its parameters or of its result are known (of at most
  /// [MAX_SHOWN] types: what is beyond is written `Any`). None when the type is not known.
  pub fn shown(&self, id: TypeId) -> Option<String> {
    self.shown_at(id, 0)
  }

  /// [Typing::shown] of a type read at the depth `depth` of lists and ranges.
  fn shown_at(&self, id: TypeId, depth: usize) -> Option<String> {
    if depth >= MAX_TYPE_DEPTH {
      return None;
    }
    if let Some(name) = self.types.name(id) {
      return Some(name.to_string());
    }
    match self.types.kind(id) {
      Kind::Any | Kind::Null => None,
      Kind::Context(_) => Some("context".to_string()),
      Kind::Function(parameters, result) if parameters.iter().chain([result]).any(|known| *known != ANY) => Some(self.types.written(id, &mut { MAX_SHOWN }, depth).to_string()),
      Kind::Function(..) => Some("function".to_string()),
      Kind::List(item) => Some(match self.shown_at(*item, depth + 1) {
        Some(item) => format!("list<{item}>"),
        None => "list".to_string(),
      }),
      Kind::Range(endpoint) => Some(match self.shown_at(*endpoint, depth + 1) {
        Some(endpoint) => format!("range<{endpoint}>"),
        None => "range".to_string(),
      }),
      _ => Some(self.types.simple(id).to_string()),
    }
  }

  /// The name `name` as the scope writes it: the key of the value that the name reads, which
  /// can be written in another form than FEEL reads a name (`Date - Time` for `Date-Time`).
  /// The name itself when the scope has no value of it.
  fn written_key(&self, name: &Name) -> Name {
    let is = |key: &Name| rheon_feel::names::is_name(key.as_str(), name.as_str());
    if !self.stated.is_empty() {
      if self.stated.contains_key(name) {
        return name.clone();
      }
      if let Some(key) = self.stated.keys().find(|key| is(key)) {
        return key.clone();
      }
    }
    let frames = self.scope.frames();
    for frame in frames.iter().rev() {
      if frame.get_entry(name).is_some() {
        if !frame.has_other_key_forms() {
          return name.clone();
        }
        // of two keys that are one name, the key of the text of the name is read, else the first
        if frame.iter().any(|(key, _)| key == name) {
          return name.clone();
        }
        return frame.iter().map(|(key, _)| key).find(|key| is(key)).cloned().unwrap_or_else(|| name.clone());
      }
    }
    name.clone()
  }

  /// The type of what `name` holds in the scope, None when the scope has no value of the name.
  fn value(&mut self, name: &Name) -> Option<TypeId> {
    if !self.stated.is_empty() {
      let stated = match rheon_feel::names::is_in_name_form(name.as_str()) {
        true => self.stated.get(name),
        false => self.stated.get(&Name::from(rheon_feel::names::name_form(name.as_str()))),
      };
      if let Some(stated) = stated {
        return Some(*stated);
      }
    }
    let frames = self.scope.frames();
    let value = frames.iter().rev().find_map(|frame| frame.get_entry(name))?;
    Some(self.types.of_value(value))
  }
}

struct Analysis<'a, 's, 'n> {
  typing: &'a mut Typing<'s>,
  /// The names bound around the expression, the innermost last, with their types.
  bound: &'a [(Name, TypeId)],
  /// The function literals that names of `bound` are bound to: the index of the name, and the
  /// literal.
  literals: &'n [(usize, AstNode)],
  /// The resolution of the expression: the unit 0.
  root: &'a Resolved<'n>,
  /// The resolutions of the literals, each made when its body is first typed: the unit after
  /// the index of the literal.
  units: &'a [OnceCell<Resolved<'n>>],
  frames: Vec<Frame>,
  /// What the slots of the frames hold, frame after frame.
  slots: Vec<Fact>,
  defs: Vec<Def>,
  /// The function definitions whose bodies are being typed for an invocation (a definition
  /// that invokes itself is not typed again inside itself).
  active: Vec<(u32, u32)>,
  depth: usize,
  /// The nodes typed so far ([MAX_STEPS]).
  steps: usize,
}

/// Returns the type of the value of `node` in `scope`, where the names of `bound` (the
/// variables and parameters of the constructs around the expression, the innermost last) have
/// the types given with them, and `literals` gives the function literals that names of `bound`
/// are bound to (the index of the name in `bound`, and the literal).
pub(crate) fn infer<'n>(node: &'n AstNode, scope: &FeelScope, bound: &[(Name, FeelType)], literals: &'n [(usize, AstNode)]) -> FeelType {
  let mut typing = Typing::new(scope);
  let bound: Vec<(Name, TypeId)> = bound.iter().map(|(name, feel_type)| (name.clone(), typing.of(feel_type))).collect();
  let ty = typing.infer(node, &bound, literals);
  typing.feel_type(ty)
}

impl<'a, 'n> Analysis<'a, '_, 'n> {
  /// The resolution of the tree `index`: the expression, or the literal `index - 1`.
  fn unit(&self, index: u32) -> &'a Resolved<'n> {
    match index.checked_sub(1) {
      None => self.root,
      Some(literal) => {
        let tree = &self.literals[literal as usize].1;
        self.units[literal as usize].get_or_init(|| resolve_names(tree))
      }
    }
  }

  /// The unit of the function literal that the name `at` of `bound` is bound to: of the last
  /// literal given for it.
  fn bound_unit(&self, at: usize) -> Option<u32> {
    let literal = self
      .literals
      .iter()
      .rposition(|(index, literal)| *index == at && matches!(literal, AstNode::FunctionDefinition(..)))?;
    Some(literal as u32 + 1)
  }

  /// Starts a frame of `binders` slots that hold nothing yet, and returns its index.
  fn enter(&mut self, binders: u32, outer: Outer) -> usize {
    let base = self.slots.len();
    self.slots.resize(base + binders as usize, Fact::ANY);
    self.frames.push(Frame { base: base as u32, outer });
    self.frames.len() - 1
  }

  /// Ends the frame started last.
  fn leave(&mut self) {
    if let Some(frame) = self.frames.pop() {
      self.slots.truncate(frame.base as usize);
    }
  }

  /// The slot `slot` of the frame `frame`, None when the frame has none of that number.
  fn slot(&mut self, frame: usize, slot: u32) -> Option<&mut Fact> {
    let base = self.frames[frame].base as usize;
    let end = self.frames.get(frame + 1).map_or(self.slots.len(), |next| next.base as usize);
    self.slots[base..end].get_mut(slot as usize)
  }

  fn def(&mut self, def: Def) -> u32 {
    self.defs.push(def);
    self.defs.len() as u32 - 1
  }

  /// What the node `id` of the tree `unit_index` gives, typed in the frame `frame`.
  fn fact(&mut self, frame: usize, unit_index: u32, id: u32) -> Fact {
    if self.depth >= MAX_DEPTH {
      return Fact::ANY;
    }
    self.steps += 1;
    self.depth += 1;
    let fact = self.typed(frame, unit_index, id);
    self.depth -= 1;
    fact
  }

  fn ty(&mut self, frame: usize, unit_index: u32, id: u32) -> TypeId {
    self.fact(frame, unit_index, id).ty
  }

  fn typed(&mut self, frame: usize, unit_index: u32, id: u32) -> Fact {
    use AstNode as A;
    let unit = self.unit(unit_index);
    let node = &unit.nodes[id as usize];
    let kids = unit.kids(id);
    let ty = match node.ast {
      A::Numeric(..) => NUMBER,
      A::String(_) => STRING,
      A::Boolean(_) => BOOLEAN,
      A::Null => NULL,
      A::At(text) => self.typing.types.of(&at_literal(text)),
      A::Name(_) => match &node.info {
        Info::Read(read) => return self.read(frame, unit, id, read),
        _ => ANY,
      },
      A::QualifiedName(segments) => match &node.info {
        Info::Read(read) => {
          let mut ty = self.read(frame, unit, id, read).ty;
          for segment in segments.iter().skip(1) {
            if let A::QualifiedNameSegment(name) = segment {
              ty = self.typing.types.member(ty, name.as_str());
            }
          }
          ty
        }
        _ => ANY,
      },
      A::Path(_, name) => match (name.as_ref(), kids) {
        (A::Name(name), &[base]) => {
          let base = self.ty(frame, unit_index, base);
          self.typing.types.member(base, name.as_str())
        }
        _ => ANY,
      },
      A::Descendant(..) => self.typing.types.list(ANY),
      A::Filter(..) => match kids {
        &[list, predicate] => {
          let list = self.ty(frame, unit_index, list);
          // whether the predicate is an index is decided around the filter, where the entries
          // of an item are no names
          let predicate = self.ty(frame, unit_index, predicate);
          let (item, is_list) = match self.typing.types.kind(list) {
            Kind::List(item) => (*item, true),
            _ => (list, false),
          };
          if predicate == NUMBER {
            // an index selects one item
            item
          } else if is_list || item != ANY {
            self.typing.types.list(item)
          } else {
            ANY
          }
        }
        _ => ANY,
      },
      A::List(_) => {
        let mut item_type: Option<TypeId> = None;
        for &item in kids {
          let next = self.ty(frame, unit_index, item);
          item_type = Some(match item_type {
            Some(current) => self.join(current, next),
            None => next,
          });
        }
        let item = match item_type {
          Some(NULL) | None => ANY,
          Some(item_type) => item_type,
        };
        self.typing.types.list(item)
      }
      A::Context(_) => self.context(frame, unit_index, id),
      A::Range(..) => match kids {
        &[start, end] => {
          let start = self.endpoint(frame, unit_index, start);
          let end = self.endpoint(frame, unit_index, end);
          let endpoint = self.join(start, end);
          self.typing.types.range(endpoint)
        }
        _ => ANY,
      },
      // `e as T` is of the type T, or null (DMN 1.7 Beta 1, Table 62)
      A::As(_, written) => type_written(written).map_or(ANY, |written| self.typing.types.of(&written)),
      A::UnaryLt(_) | A::UnaryLe(_) | A::UnaryGt(_) | A::UnaryGe(_) | A::UnaryEq(_) | A::UnaryNe(_) => match kids {
        &[end] => {
          let end = self.ty(frame, unit_index, end);
          self.typing.types.range(end)
        }
        _ => ANY,
      },
      A::Eq(..)
      | A::Nq(..)
      | A::Lt(..)
      | A::Le(..)
      | A::Gt(..)
      | A::Ge(..)
      | A::And(..)
      | A::Or(..)
      | A::In(..)
      | A::Out(..)
      | A::Between(..)
      | A::InstanceOf(..)
      | A::Some(..)
      | A::Every(..)
      | A::ExpressionList(_)
      | A::NegatedList(_) => BOOLEAN,
      A::Neg(_) => match kids {
        &[operand] => {
          let operand = self.ty(frame, unit_index, operand);
          match self.typing.types.kind(operand) {
            Kind::Number | Kind::DaysAndTimeDuration | Kind::YearsAndMonthsDuration => operand,
            _ => ANY,
          }
        }
        _ => ANY,
      },
      A::Add(..) | A::Sub(..) | A::Mul(..) | A::Div(..) | A::Exp(..) => match kids {
        &[a, b] => {
          let a = self.ty(frame, unit_index, a);
          let b = self.ty(frame, unit_index, b);
          let (a, b) = (self.typing.types.simple(a), self.typing.types.simple(b));
          let result = match node.ast {
            A::Add(..) => addition(&a, &b),
            A::Sub(..) => subtraction(&a, &b),
            A::Mul(..) => multiplication(&a, &b),
            A::Div(..) => division(&a, &b),
            _ => power(&a, &b),
          };
          self.typing.types.of(&result)
        }
        _ => ANY,
      },
      A::If(..) => match kids {
        &[_, then, otherwise] => {
          let then = self.ty(frame, unit_index, then);
          let otherwise = self.ty(frame, unit_index, otherwise);
          self.join(then, otherwise)
        }
        _ => ANY,
      },
      A::For(contexts, _) => self.iteration(frame, unit_index, id, contexts),
      A::EvaluatedExpression(_) => match kids {
        &[inner] => self.ty(frame, unit_index, inner),
        _ => ANY,
      },
      A::FunctionDefinition(..) => return self.definition(frame, unit_index, id),
      A::FunctionInvocation(_, arguments) => self.invocation(frame, unit_index, id, arguments),
      _ => ANY,
    };
    Fact::of(ty)
  }

  /// The type of an endpoint of a range.
  fn endpoint(&mut self, frame: usize, unit_index: u32, id: u32) -> TypeId {
    let unit = self.unit(unit_index);
    match (unit.nodes[id as usize].ast, unit.kids(id)) {
      (AstNode::IntervalStart(..) | AstNode::IntervalEnd(..), &[value]) => self.ty(frame, unit_index, value),
      _ => self.ty(frame, unit_index, id),
    }
  }

  /// The type both types are: the type itself when they are the same, `Any` otherwise. A type
  /// and `Null` are the type (a value or null).
  fn join(&mut self, a: TypeId, b: TypeId) -> TypeId {
    self.join_at(a, b, 0)
  }

  /// [Analysis::join] of types read at the depth `depth` of lists.
  fn join_at(&mut self, a: TypeId, b: TypeId, depth: usize) -> TypeId {
    if a == NULL {
      return b;
    }
    if b == NULL || a == b {
      return a;
    }
    if depth < MAX_TYPE_DEPTH
      && let (Kind::List(a), Kind::List(b)) = (self.typing.types.kind(a), self.typing.types.kind(b))
    {
      let item = self.join_at(*a, *b, depth + 1);
      return self.typing.types.list(item);
    }
    ANY
  }

  /// What a name reads.
  fn read(&mut self, frame: usize, unit: &'a Resolved<'n>, id: u32, read: &Read) -> Fact {
    match read.target {
      Target::Place(place) => self.place(frame, place),
      Target::Free(free) => self.outside(frame, unit, unit.frees[free as usize].name),
      // a qualified name of several segments: its first segment, as a name reads it there
      Target::Search if read.name != NONE => {
        let mut region = unit.nodes[id as usize].region;
        while region != NONE {
          let Region { parent, kind } = unit.regions[region as usize];
          match kind {
            RegionKind::Layout { layout, visible } => {
              if let Some(place) = unit.layouts[layout as usize].find(read.name, visible) {
                return self.place(frame, place);
              }
            }
            RegionKind::Item { .. } => {}
            RegionKind::Closure => break,
          }
          region = parent;
        }
        self.outside(frame, unit, read.name)
      }
      Target::Search => Fact::ANY,
    }
  }

  /// What a binder of the frame `frame` holds.
  fn place(&mut self, frame: usize, place: Place) -> Fact {
    match place {
      Place::Slot(slot) => self.slot(frame, slot).map_or(Fact::ANY, |held| *held),
      // the results so far of a `for` are not typed
      Place::Partial(_) => Fact::ANY,
    }
  }

  /// What the name `name` of the tree `unit` reads that the body of the frame `frame` does not
  /// bind: for a function body, a binder that is in scope where the function is written (every
  /// entry of the context literals it is written in, DMN 1.6 clause 10.3.2.11.1), then what
  /// that body reads from outside; for the root of a tree, a name bound around it or a value of
  /// the scope. The bodies of one chain are of one tree, so the name is its index all along.
  fn outside(&mut self, mut frame: usize, unit: &'a Resolved<'n>, name: u32) -> Fact {
    loop {
      match self.frames[frame].outer {
        Outer::Bound { visible } => {
          let name: &Name = &unit.names[name as usize];
          return self.bound_name(name, visible as usize);
        }
        Outer::Site { frame: site, region } => {
          let site = site as usize;
          let mut region = region;
          while region != NONE {
            let Region { parent, kind } = unit.regions[region as usize];
            match kind {
              RegionKind::Layout { layout, visible } => {
                let layout = &unit.layouts[layout as usize];
                let visible = if layout.kind == LayoutKind::Context { layout.entries.len() as u32 } else { visible };
                if let Some(place) = layout.find(name, visible) {
                  return self.place(site, place);
                }
              }
              RegionKind::Item { .. } => {}
              RegionKind::Closure => break,
            }
            region = parent;
          }
          frame = site;
        }
      }
    }
  }

  /// What `name` reads among the first `visible` names bound around the expression and in the
  /// scope.
  fn bound_name(&mut self, name: &Name, visible: usize) -> Fact {
    if let Some(at) = self.bound.iter().rposition(|(bound, _)| bound == name) {
      if at >= visible {
        return Fact::ANY;
      }
      let def = if self.bound_unit(at).is_some() { self.def(Def::Bound(at as u32)) } else { NONE };
      return Fact { ty: self.bound[at].1, def };
    }
    self.typing.value(name).map_or(Fact::ANY, Fact::of)
  }

  /// A context literal: each entry with the type of its value. The entries are typed in the
  /// order they are evaluated in, the order they need each other, so an entry reads the
  /// entries it needs with their types.
  fn context(&mut self, frame: usize, unit_index: u32, id: u32) -> TypeId {
    let unit = self.unit(unit_index);
    let layout = match &unit.nodes[id as usize].info {
      Info::Context { layout, .. } => &unit.layouts[*layout as usize],
      _ => return ANY,
    };
    // the slots may hold what a construct before this one left in them
    for &(_, place) in layout.entries.iter() {
      self.set(frame, place, Fact::ANY);
    }
    let mut types = BTreeMap::new();
    for (position, &entry) in unit.kids(id).iter().enumerate() {
      let (AstNode::ContextEntry(key, value), &[_, value_id]) = (unit.nodes[entry as usize].ast, unit.kids(entry)) else {
        continue;
      };
      let AstNode::ContextEntryKey(name) = key.as_ref() else {
        continue;
      };
      let mut fact = self.fact(frame, unit_index, value_id);
      if fact.ty == NULL {
        fact.ty = ANY;
      }
      if !matches!(value.as_ref(), AstNode::FunctionDefinition(..)) {
        fact.def = NONE;
      }
      types.insert(name.clone(), fact.ty);
      if let Some(&(_, place)) = layout.entries.get(position) {
        self.set(frame, place, fact);
      }
    }
    self.typing.types.context(types)
  }

  fn set(&mut self, frame: usize, place: Place, fact: Fact) {
    if let Place::Slot(slot) = place
      && let Some(held) = self.slot(frame, slot)
    {
      *held = fact;
    }
  }

  /// A `for`: its variables with the types of the items of their domains, and the list of what
  /// its body gives.
  fn iteration(&mut self, frame: usize, unit_index: u32, id: u32, contexts: &AstNode) -> TypeId {
    let unit = self.unit(unit_index);
    let layout = match &unit.nodes[id as usize].info {
      Info::Iteration { layout, .. } => &unit.layouts[*layout as usize],
      _ => return ANY,
    };
    for &(_, place) in layout.entries.iter() {
      self.set(frame, place, Fact::ANY);
    }
    let mut kids = unit.kids(id).iter().copied();
    for (level, (_, domain)) in variables(contexts, false).iter().enumerate() {
      let variable = match domain.len() {
        // the numbers of an interval
        2 => {
          kids.next();
          kids.next();
          NUMBER
        }
        1 => {
          let Some(values) = kids.next() else {
            return ANY;
          };
          let values = self.ty(frame, unit_index, values);
          match self.typing.types.kind(values) {
            Kind::List(item) => *item,
            Kind::Range(endpoint) => *endpoint,
            Kind::Any | Kind::Null => ANY,
            // a value that is no list is a list of itself
            _ => values,
          }
        }
        _ => ANY,
      };
      if let Some(&(_, place)) = layout.entries.get(level) {
        self.set(frame, place, Fact::of(variable));
      }
    }
    let Some(body) = kids.next() else {
      return ANY;
    };
    let body = self.ty(frame, unit_index, body);
    self.typing.types.list(if body == NULL { ANY } else { body })
  }

  /// A function definition where it is written: the types of its parameters as written, and
  /// the type of its body with them.
  fn definition(&mut self, frame: usize, unit_index: u32, id: u32) -> Fact {
    let unit = self.unit(unit_index);
    let node = &unit.nodes[id as usize];
    let AstNode::FunctionDefinition(parameters, body) = node.ast else {
      return Fact::ANY;
    };
    let mut parameter_types = vec![];
    if let AstNode::FormalParameters(items) = parameters.as_ref() {
      for item in items {
        if let AstNode::FormalParameter(name, parameter_type) = item
          && let AstNode::ParameterName(_) = name.as_ref()
        {
          parameter_types.push(self.typing.types.of(&written_type(parameter_type)));
        }
      }
    }
    let result = match (&node.info, body.as_ref()) {
      (Info::Function { proto }, AstNode::FunctionBody(_, false)) if *proto != NONE => {
        let proto = &unit.functions[*proto as usize];
        let body = self.enter(
          proto.binders,
          Outer::Site {
            frame: frame as u32,
            region: node.region,
          },
        );
        for (&slot, &parameter_type) in proto.params.iter().zip(&parameter_types) {
          if let Some(held) = self.slot(body, slot) {
            *held = Fact::of(parameter_type);
          }
        }
        let result = self.ty(body, unit_index, proto.body);
        self.leave();
        result
      }
      _ => ANY,
    };
    let ty = self.typing.types.function(parameter_types, result);
    let def = self.def(Def::Written {
      unit: unit_index,
      node: id,
      frame: frame as u32,
    });
    Fact { ty, def }
  }

  fn invocation(&mut self, frame: usize, unit_index: u32, id: u32, arguments: &AstNode) -> TypeId {
    let unit = self.unit(unit_index);
    let Some((&callee, values)) = unit.kids(id).split_first() else {
      return ANY;
    };
    let argument_types: Vec<TypeId> = values.iter().map(|&value| self.ty(frame, unit_index, value)).collect();
    let positional = matches!(arguments, AstNode::PositionalParameters(_));
    let callee_node = &unit.nodes[callee as usize];
    if let AstNode::Name(name) = callee_node.ast {
      // a variable, a parameter or a value of the scope of this name is what is invoked
      let known = match &callee_node.info {
        Info::Read(read) => self.read(frame, unit, callee, read),
        _ => Fact::ANY,
      };
      if let Kind::Function(_, result) = self.typing.types.kind(known.ty) {
        let result = *result;
        // a function definition of the text: its body with the types of the arguments
        let called = if known.def == NONE { None } else { self.call(known.def, &argument_types, arguments) };
        return match called {
          Some(called) if called != ANY => called,
          _ => result,
        };
      }
      if known.ty == ANY
        && let Some(built_in) = built_in(name.as_str())
      {
        return self.built_in_result(built_in, &argument_types, positional, arguments);
      }
      return ANY;
    }
    let callee = self.ty(frame, unit_index, callee);
    match self.typing.types.kind(callee) {
      Kind::Function(_, result) => *result,
      _ => ANY,
    }
  }

  /// The type of the result of an invocation of the function definition `def` with arguments
  /// of the types `arguments` (`written` as the text writes them): the type of its body, where
  /// a parameter that has no type written has the type of its argument. The body reads the
  /// names it does not bind where the definition is written, not where it is invoked. None
  /// when the definition has no body of FEEL, is being typed for an invocation already (it
  /// invokes itself), or the analysis has typed [MAX_STEPS] nodes.
  fn call(&mut self, def: u32, arguments: &[TypeId], written: &AstNode) -> Option<TypeId> {
    match self.defs[def as usize] {
      Def::Written { unit, node, frame } => self.call_at(unit, node, frame, arguments, written),
      Def::Bound(at) => {
        let unit_index = self.bound_unit(at as usize)?;
        let unit = self.unit(unit_index);
        // the root of the literal reads the names bound before it and its own
        let root = self.enter(unit.root.binders, Outer::Bound { visible: at + 1 });
        let result = self.call_at(unit_index, unit.root(), root as u32, arguments, written);
        self.leave();
        result
      }
    }
  }

  fn call_at(&mut self, unit_index: u32, id: u32, site: u32, arguments: &[TypeId], written: &AstNode) -> Option<TypeId> {
    let unit = self.unit(unit_index);
    let node = &unit.nodes[id as usize];
    let (AstNode::FunctionDefinition(parameters, body), Info::Function { proto }) = (node.ast, &node.info) else {
      return None;
    };
    let (AstNode::FormalParameters(items), AstNode::FunctionBody(_, false)) = (parameters.as_ref(), body.as_ref()) else {
      return None;
    };
    if *proto == NONE || self.steps >= MAX_STEPS || self.active.contains(&(unit_index, id)) {
      return None;
    }
    // the names of named arguments, in their order
    let named: Option<Vec<&Name>> = match written {
      AstNode::NamedParameters(named) => Some(
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
    let mut parameters: Vec<(u32, TypeId)> = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
      if let AstNode::FormalParameter(parameter, parameter_type) = item
        && let AstNode::ParameterName(parameter) = parameter.as_ref()
      {
        let parameter_type = match written_type(parameter_type) {
          FeelType::Any => {
            let argument = match &named {
              Some(names) => names.iter().position(|name| *name == parameter).and_then(|at| arguments.get(at)),
              None => arguments.get(position),
            };
            match argument {
              Some(&NULL) | None => ANY,
              Some(&argument) => argument,
            }
          }
          written => self.typing.types.of(&written),
        };
        if let Some(&slot) = proto.params.get(position) {
          parameters.push((slot, parameter_type));
        }
      }
    }
    let body = self.enter(proto.binders, Outer::Site { frame: site, region: node.region });
    for (slot, parameter_type) in parameters {
      if let Some(held) = self.slot(body, slot) {
        *held = Fact::of(parameter_type);
      }
    }
    self.active.push((unit_index, id));
    let result = self.ty(body, unit_index, proto.body);
    self.active.pop();
    self.leave();
    Some(result)
  }

  /// The type of the result of a built-in function: of its signatures when they agree, and for
  /// the functions whose result has the type of an argument, that type.
  fn built_in_result(&mut self, built_in: &BuiltIn, arguments: &[TypeId], positional: bool, written: &AstNode) -> TypeId {
    /// What the rules below read of the first argument.
    enum First {
      List(TypeId),
      Any,
      Context,
      Magnitude,
      Other,
    }
    let name = built_in.name;
    if let Some(&first) = arguments.first().filter(|_| positional) {
      let shape = match self.typing.types.kind(first) {
        Kind::List(item) => First::List(*item),
        Kind::Any => First::Any,
        Kind::Context(_) => First::Context,
        Kind::Number | Kind::DaysAndTimeDuration | Kind::YearsAndMonthsDuration => First::Magnitude,
        _ => First::Other,
      };
      match (name, shape) {
        // a list of the items of the first list
        ("append" | "concatenate" | "insert before" | "remove" | "reverse" | "sublist" | "sort" | "distinct values" | "union" | "list replace", First::List(_)) => return first,
        // an item of the list, or one of the arguments
        ("min" | "max" | "mode" | "median", First::List(item)) if item != ANY => {
          return if name == "mode" { self.typing.types.list(item) } else { item };
        }
        ("min" | "max", First::Context | First::Magnitude | First::Other) => return first,
        ("abs", First::Magnitude) => return first,
        // the entry of a context that a string names
        ("get value", First::Context) => {
          if let AstNode::PositionalParameters(items) = written
            && let Some(AstNode::String(key)) = items.get(1)
          {
            return self.typing.types.member(first, key);
          }
          return ANY;
        }
        _ => {}
      }
    }
    if name == "range" {
      return self.typing.types.range(ANY);
    }
    let mut results = built_in.signatures.iter().map(|signature| signature.result);
    let Some(result) = results.next() else {
      return ANY;
    };
    if results.all(|other| other == result) {
      self.typing.types.of(&signature_type(result))
    } else {
      ANY
    }
  }
}

/// The type of `a * b`: a duration is multiplied only by a number, a number by a number or a
/// duration (Table 60).
fn multiplication(a: &FeelType, b: &FeelType) -> FeelType {
  match (a, b) {
    (FeelType::Number, FeelType::Number) => FeelType::Number,
    (duration @ (FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration), FeelType::Number | FeelType::Any)
    | (FeelType::Number | FeelType::Any, duration @ (FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration)) => duration.clone(),
    _ => FeelType::Any,
  }
}

/// The type of `a / b`: a number is divided only by a number (Table 60).
fn division(a: &FeelType, b: &FeelType) -> FeelType {
  match (a, b) {
    (FeelType::Number, FeelType::Number | FeelType::Any) => FeelType::Number,
    (duration @ (FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration), FeelType::Number) => duration.clone(),
    (FeelType::DaysAndTimeDuration, FeelType::DaysAndTimeDuration) | (FeelType::YearsAndMonthsDuration, FeelType::YearsAndMonthsDuration) => FeelType::Number,
    _ => FeelType::Any,
  }
}

/// The type of `a ** b`: only numbers have a power (Table 61), so a power that is not null is
/// a number.
fn power(a: &FeelType, b: &FeelType) -> FeelType {
  let number = |feel_type: &FeelType| matches!(feel_type, FeelType::Number | FeelType::Any);
  if number(a) && number(b) { FeelType::Number } else { FeelType::Any }
}
