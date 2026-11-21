//! # The type of a value related to a type, without building the type of the value
//!
//! [Value::type_of] builds the type of a value as a tree. A value can hold the same list or
//! context on several paths (`[x, x]` and `{a: x, b: x}` hold `x` twice without a copy), so a
//! value that a loop doubles through `partial` has exponentially many paths in a linear size, and
//! building its type, or checking each item against the type of the first, follows every path.
//! The relations here answer what the built type would answer, reading the value only as far as
//! the relation needs: the type of a value is the given type, the types of two values are equal,
//! the type of a value conforms to the type of another ([FeelType::is_conformant]), and
//! `instance of` ([FeelType::instance_of]). The conformance of a value to a type
//! ([Value::is_conformant]), a relation of the value rather than of its type, is decided here
//! too ([Conformance]).
//!
//! The type of a list is a list of the type of its first item when every other item's type
//! conforms to it, and a list of `Any` otherwise. Within one relation, whether a list's items
//! conform to its first is decided once per list that nests more than two levels (a shallower
//! one costs a pass over its items when it is reached again), and the answer for a pair of nodes
//! is remembered when either node is a shared list or a shared context. A pair is reached again
//! only through two paths that meet in a list or a context held twice, which is shared, so every
//! pair is decided once. The memory lives for one relation, keyed by the addresses of the
//! borrowed nodes (no other node is stored at an address while the values are borrowed), is
//! created on first use, and is never iterated.

use crate::context::FeelContext;
use crate::names::Name;
use crate::types::FeelType;
use crate::values::{FeelList, Value};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

const ANY: &FeelType = &FeelType::Any;
const NULL: &FeelType = &FeelType::Null;
const BOOLEAN: &FeelType = &FeelType::Boolean;
const NUMBER: &FeelType = &FeelType::Number;
const STRING: &FeelType = &FeelType::String;
const DATE: &FeelType = &FeelType::Date;
const DATE_TIME: &FeelType = &FeelType::DateTime;
const TIME: &FeelType = &FeelType::Time;
const DAYS_AND_TIME_DURATION: &FeelType = &FeelType::DaysAndTimeDuration;
const YEARS_AND_MONTHS_DURATION: &FeelType = &FeelType::YearsAndMonthsDuration;

impl Value {
  /// Whether the type of this value is `feel_type` (`self.type_of() == *feel_type`), decided
  /// without building the type of this value.
  pub fn has_type(&self, feel_type: &FeelType) -> bool {
    Relations::default().same_shapes(Ty::Of(self), Ty::Is(feel_type))
  }

  /// Whether this value and `other` have the same type (`self.type_of() == other.type_of()`),
  /// decided without building either type.
  pub fn has_type_of(&self, other: &Value) -> bool {
    Relations::default().same_shapes(Ty::Of(self), Ty::Of(other))
  }

  /// Whether the type of this value is an instance of `feel_type`
  /// (`self.type_of().instance_of(feel_type)`, [FeelType::instance_of]), decided without
  /// building the type of this value.
  pub fn type_instance_of(&self, feel_type: &FeelType) -> bool {
    Relations::default().instance_of(Ty::Of(self), feel_type)
  }
}

impl FeelList {
  /// The item whose type the types of all the items conform to, so that the type of this list
  /// ([Value::type_of]) is a list of the type of that item, wherever it stands; `None` when no
  /// item is one, and for the empty list. Decided without building a type, reading the items
  /// only as far as their conformance needs, a pair with a shared list or context once.
  pub fn item_of_the_widest_type(&self) -> Option<&Value> {
    Relations::default().widest(self)
  }
}

/// A type: that of a value ([Value::type_of]), or a type given as such.
#[derive(Clone, Copy)]
enum Ty<'a> {
  Of(&'a Value),
  Is(&'a FeelType),
}

/// The outermost constructor of a type, with what a relation reads of its parts.
enum Shape<'a> {
  /// A type without parts: `Any`, `Null` and the simple types.
  Leaf(&'a FeelType),
  /// A list type; the type of its items is decided by [Relations::items].
  List(Items<'a>),
  /// A range type; the type of its endpoints is decided by [Relations::ends].
  Range(Ends<'a>),
  /// A context type: the types of its entries.
  Context(Entries<'a>),
  /// A function type, which holds types only.
  Function(Cow<'a, FeelType>),
}

/// The items of a list type: those of a list value, or the item type of a list type.
#[derive(Clone, Copy)]
enum Items<'a> {
  Of(&'a FeelList),
  Is(&'a FeelType),
}

/// The endpoints of a range type: those of a range value, or the endpoint type of a range type.
#[derive(Clone, Copy)]
enum Ends<'a> {
  Of(&'a Value, &'a Value),
  Is(&'a FeelType),
}

/// The entries of a context type: the entries of a context value, or of a context type.
#[derive(Clone, Copy)]
enum Entries<'a> {
  Of(&'a FeelContext),
  Is(&'a BTreeMap<Name, FeelType>),
}

impl<'a> Entries<'a> {
  fn len(self) -> usize {
    match self {
      Entries::Of(ctx) => ctx.len(),
      Entries::Is(entries) => entries.len(),
    }
  }

  /// The type of the entry `name`.
  fn get(self, name: &Name) -> Option<Ty<'a>> {
    match self {
      Entries::Of(ctx) => ctx.get_entry(name).map(Ty::Of),
      Entries::Is(entries) => entries.get(name).map(Ty::Is),
    }
  }

  /// Whether `holds` holds for every entry, in the order of the names.
  fn all(self, mut holds: impl FnMut(&Name, Ty<'a>) -> bool) -> bool {
    match self {
      Entries::Of(ctx) => ctx.iter().all(|(name, value)| holds(name, Ty::Of(value))),
      Entries::Is(entries) => entries.iter().all(|(name, feel_type)| holds(name, Ty::Is(feel_type))),
    }
  }
}

impl<'a> Ty<'a> {
  /// The outermost constructor of this type, as [Value::type_of] gives it.
  fn shape(self) -> Shape<'a> {
    match self {
      Ty::Is(feel_type) => match feel_type {
        FeelType::List(items) => Shape::List(Items::Is(items)),
        FeelType::Range(ends) => Shape::Range(Ends::Is(ends)),
        FeelType::Context(entries) => Shape::Context(Entries::Is(entries)),
        FeelType::Function(..) => Shape::Function(Cow::Borrowed(feel_type)),
        FeelType::Any
        | FeelType::Boolean
        | FeelType::Date
        | FeelType::DateTime
        | FeelType::DaysAndTimeDuration
        | FeelType::Null
        | FeelType::Number
        | FeelType::String
        | FeelType::Time
        | FeelType::YearsAndMonthsDuration => Shape::Leaf(feel_type),
      },
      Ty::Of(value) => match value {
        Value::List(items) => Shape::List(Items::Of(items)),
        // an undefined endpoint has the type of the other one
        Value::Range(_, start_type, end, _) if start_type.undefined() => Shape::Range(Ends::Of(end, end)),
        Value::Range(start, _, _, end_type) if end_type.undefined() => Shape::Range(Ends::Of(start, start)),
        Value::Range(start, _, end, _) => Shape::Range(Ends::Of(start, end)),
        Value::Context(ctx) => Shape::Context(Entries::Of(ctx)),
        Value::FunctionDefinition(function) => Shape::Function(Cow::Owned(FeelType::Function(
          function.parameters().iter().map(|(_, feel_type)| feel_type.clone()).collect(),
          Box::new(function.result_type().clone()),
        ))),
        Value::ContextType(feel_type) | Value::ContextTypeEntry(_, feel_type) | Value::FeelType(feel_type) | Value::FormalParameter(_, feel_type) => Ty::Is(feel_type).shape(),
        Value::IntervalEnd(value, _) | Value::IntervalStart(value, _) | Value::TestAnswer(value) => Ty::Of(value).shape(),
        // a comparison with one endpoint is a range of the type of that endpoint
        Value::UnaryGreater(end) | Value::UnaryGreaterOrEqual(end) | Value::UnaryLess(end) | Value::UnaryLessOrEqual(end) | Value::UnaryEqual(end) | Value::UnaryNotEqual(end) => {
          Shape::Range(Ends::Of(end, end))
        }
        Value::Boolean(_) => Shape::Leaf(BOOLEAN),
        Value::Date(_) => Shape::Leaf(DATE),
        Value::DateTime(_) => Shape::Leaf(DATE_TIME),
        Value::DaysAndTimeDuration(_) => Shape::Leaf(DAYS_AND_TIME_DURATION),
        Value::Null(_) => Shape::Leaf(NULL),
        Value::Number(_) => Shape::Leaf(NUMBER),
        Value::String(_) => Shape::Leaf(STRING),
        Value::Time(_) => Shape::Leaf(TIME),
        Value::YearsAndMonthsDuration(_) => Shape::Leaf(YEARS_AND_MONTHS_DURATION),
        Value::BuiltInFunction(_)
        | Value::ContextEntry(_, _)
        | Value::ContextEntryKey(_)
        | Value::ContextTypeEntryKey(_)
        | Value::ExpressionList(_)
        | Value::ExternalJavaFunction(_, _)
        | Value::ExternalPmmlFunction(_, _)
        | Value::FormalParameters(_)
        | Value::FunctionBody(_, _)
        | Value::Irrelevant
        | Value::NamedParameters(_)
        | Value::NegatedCommaList(_)
        | Value::ParameterName(_)
        | Value::ParameterTypes(_)
        | Value::QualifiedNameSegment(_) => Shape::Leaf(ANY),
      },
    }
  }

  /// This type as a key of the memory: a list or a context value by the address of its node
  /// ([Value::node_address]), a type by its own address; `None` for any other type, whose
  /// relations are decided without walking a value.
  fn node(self) -> Option<Node> {
    match self {
      Ty::Of(value) => value.node_address().map(Node::Value),
      Ty::Is(feel_type) => Some(Node::Type(std::ptr::from_ref(feel_type).addr())),
    }
  }

  /// Whether this is the type of a shared list or context ([Value::is_shared]).
  fn is_shared(self) -> bool {
    matches!(self, Ty::Of(value) if value.is_shared())
  }
}

/// A node of a memory key ([Ty::node]).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
  Value(usize),
  Type(usize),
}

/// A relation between two types.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Relation {
  /// The types are equal.
  Same,
  /// The first type conforms to the second.
  Conforms,
}

/// The memory of one relation (see the module documentation).
#[derive(Default)]
struct Relations {
  /// The place of the item of a list that all its items conform to ([Relations::widest]), by
  /// the address of the items.
  widest: Option<HashMap<usize, Option<usize>>>,
  /// The answers for the pairs with a shared list or a shared context on either side.
  pairs: Option<HashMap<(Relation, Node, Node), bool>>,
}

impl<'a> Relations {
  /// The type of the items of a list type: for a list value, `Null` when it is empty, the type
  /// of the item that the types of all its items conform to, `Any` when there is no such item.
  fn items(&mut self, items: Items<'a>) -> Ty<'a> {
    match items {
      Items::Is(feel_type) => Ty::Is(feel_type),
      Items::Of(list) if list.is_empty() => Ty::Is(NULL),
      Items::Of(list) => self.widest(list).map_or(Ty::Is(ANY), Ty::Of),
    }
  }

  /// The item of a list whose type the types of all its items conform to (the only item of a
  /// list of one); `None` when there is none, and for the empty list. The item does not depend
  /// on the order of the items but for items of one type, of which it is the first: a null
  /// beside numbers is not it, wherever it stands.
  fn widest(&mut self, list: &'a FeelList) -> Option<&'a Value> {
    if list.len() <= 1 {
      return list.first();
    }
    // a list of scalars, or of containers of scalars, is cheap to decide again when it is
    // reached again; a deeper one is decided once, so that deciding it never repeats the
    // decisions below it
    if list.nesting_depth() <= 2 {
      return self.widest_place(list).and_then(|place| list.get(place));
    }
    let address = list.address().expect("a list with items has an address");
    if let Some(place) = self.widest.as_ref().and_then(|widest| widest.get(&address)) {
      return place.and_then(|place| list.get(place));
    }
    let place = self.widest_place(list);
    self.widest.get_or_insert_with(HashMap::new).insert(address, place);
    place.and_then(|place| list.get(place))
  }

  /// The place of the item of [Relations::widest], in two passes. The first holds an item and
  /// gives it up for the next item that does not conform to it: the item that all conform to,
  /// if there is one, is that item or a later one. The second holds the items before the one
  /// held last against it; those after it conform to it.
  fn widest_place(&mut self, list: &'a FeelList) -> Option<usize> {
    let mut held = 0;
    for (place, item) in list.iter().enumerate().skip(1) {
      if !self.conforms(Ty::Of(item), Ty::Of(&list[held])) {
        held = place;
      }
    }
    // the items after the one held conform to it; those before it are still to be held against it
    let widest = &list[held];
    list.iter().take(held).all(|item| self.conforms(Ty::Of(item), Ty::Of(widest))).then_some(held)
  }

  /// The type of the endpoints of a range type: for a range value, that of its start when its
  /// end has the same type, `Any` otherwise.
  fn ends(&mut self, ends: Ends<'a>) -> Ty<'a> {
    match ends {
      Ends::Is(feel_type) => Ty::Is(feel_type),
      Ends::Of(start, end) => {
        if self.same(Ty::Of(start), Ty::Of(end)) {
          Ty::Of(start)
        } else {
          Ty::Is(ANY)
        }
      }
    }
  }

  /// The answer of `decide` for `relation` between `a` and `b`, remembered when either is a
  /// shared list or a shared context.
  fn remembered(&mut self, relation: Relation, a: Ty<'a>, b: Ty<'a>, decide: impl FnOnce(&mut Self) -> bool) -> bool {
    if (a.is_shared() || b.is_shared())
      && let (Some(a), Some(b)) = (a.node(), b.node())
    {
      let key = (relation, a, b);
      if let Some(answer) = self.pairs.as_ref().and_then(|pairs| pairs.get(&key)) {
        return *answer;
      }
      let answer = decide(self);
      self.pairs.get_or_insert_with(HashMap::new).insert(key, answer);
      return answer;
    }
    decide(self)
  }

  /// Whether the types `a` and `b` are equal.
  fn same(&mut self, a: Ty<'a>, b: Ty<'a>) -> bool {
    self.remembered(Relation::Same, a, b, |relations| relations.same_shapes(a, b))
  }

  /// [Relations::same], decided on the shapes of `a` and `b` without a lookup.
  fn same_shapes(&mut self, a: Ty<'a>, b: Ty<'a>) -> bool {
    match (a.shape(), b.shape()) {
      (Shape::Leaf(a), Shape::Leaf(b)) => a == b,
      (Shape::List(a), Shape::List(b)) => {
        let (a, b) = (self.items(a), self.items(b));
        self.same(a, b)
      }
      (Shape::Range(a), Shape::Range(b)) => {
        let (a, b) = (self.ends(a), self.ends(b));
        self.same(a, b)
      }
      (Shape::Context(a), Shape::Context(b)) => a.len() == b.len() && a.all(|name, a| b.get(name).is_some_and(|b| self.same(a, b))),
      (Shape::Function(a), Shape::Function(b)) => a == b,
      _ => false,
    }
  }

  /// Whether the type `a` conforms to the type `b` ([FeelType::is_conformant]).
  fn conforms(&mut self, a: Ty<'a>, b: Ty<'a>) -> bool {
    self.remembered(Relation::Conforms, a, b, |relations| relations.conforms_shapes(a, b))
  }

  /// [Relations::conforms], decided on the shapes of `a` and `b` without a lookup.
  fn conforms_shapes(&mut self, a: Ty<'a>, b: Ty<'a>) -> bool {
    let target = b.shape();
    if matches!(target, Shape::Leaf(FeelType::Any)) {
      return true;
    }
    let shape = a.shape();
    if matches!(shape, Shape::Leaf(FeelType::Null)) {
      return true;
    }
    match (shape, target) {
      (Shape::Leaf(a), Shape::Leaf(b)) => a == b,
      (Shape::List(a), Shape::List(b)) => {
        let (a, b) = (self.items(a), self.items(b));
        self.conforms(a, b)
      }
      (Shape::Range(a), Shape::Range(b)) => {
        let (a, b) = (self.ends(a), self.ends(b));
        self.conforms(a, b)
      }
      (Shape::Context(a), Shape::Context(b)) => b.all(|name, b| a.get(name).is_some_and(|a| self.conforms(a, b))),
      (Shape::Function(a), Shape::Function(b)) => a.is_conformant(&b),
      _ => false,
    }
  }

  /// Whether the type of `value` is an instance of `target` ([FeelType::instance_of]): a list
  /// type when its item type is that of `target` or that is `Any`; a context type when each
  /// entry of `target` is an entry whose type is `Null` or that of the entry of `target`.
  fn instance_of(&mut self, value: Ty<'a>, target: &'a FeelType) -> bool {
    match (value.shape(), target) {
      (Shape::List(items), FeelType::List(target_items)) => {
        **target_items == FeelType::Any || {
          let items = self.items(items);
          self.same(items, Ty::Is(target_items))
        }
      }
      (Shape::Context(entries), FeelType::Context(target_entries)) => target_entries.iter().all(|(name, target_type)| {
        entries
          .get(name)
          .is_some_and(|entry| matches!(entry.shape(), Shape::Leaf(FeelType::Null)) || self.same(entry, Ty::Is(target_type)))
      }),
      _ => false,
    }
  }
}

/// The conformance of a value to a type ([Value::is_conformant]), checked on the value: a
/// relation of the value itself, not of its type ([Value::type_of] types a list by its first
/// item). The answer for a shared list or context ([Value::is_shared]) and a type is remembered
/// by their addresses: a node held once is reached again only through a shared node above it,
/// so every pair is checked once. The memory lives for one check, is created on first use, and
/// is never iterated.
#[derive(Default)]
pub(super) struct Conformance {
  shared: Option<HashMap<(usize, usize), bool>>,
}

impl Conformance {
  /// Whether `value` conforms to `target`, remembered for a shared list or context.
  pub(super) fn check(&mut self, value: &Value, target: &FeelType) -> bool {
    if matches!(target, FeelType::Any) {
      return true;
    }
    if value.is_shared()
      && let Some(address) = value.node_address()
    {
      let key = (address, std::ptr::from_ref(target).addr());
      if let Some(answer) = self.shared.as_ref().and_then(|shared| shared.get(&key)) {
        return *answer;
      }
      let answer = self.decide(value, target);
      self.shared.get_or_insert_with(HashMap::new).insert(key, answer);
      return answer;
    }
    self.decide(value, target)
  }

  /// [Conformance::check] without a lookup, for a target type other than `Any`.
  fn decide(&mut self, value: &Value, target: &FeelType) -> bool {
    match value {
      Value::Null(_) => true,
      Value::Boolean(_) => matches!(target, FeelType::Boolean),
      Value::Number(_) => matches!(target, FeelType::Number),
      Value::String(_) => matches!(target, FeelType::String),
      Value::Date(_) => matches!(target, FeelType::Date),
      Value::Time(_) => matches!(target, FeelType::Time),
      Value::DateTime(_) => matches!(target, FeelType::DateTime),
      Value::DaysAndTimeDuration(_) => matches!(target, FeelType::DaysAndTimeDuration),
      Value::YearsAndMonthsDuration(_) => matches!(target, FeelType::YearsAndMonthsDuration),
      Value::Range(start, start_type, end, end_type) => match target {
        // an undefined endpoint has the type of the other one
        FeelType::Range(ends) if start_type.undefined() => self.check(end, ends),
        FeelType::Range(ends) if end_type.undefined() => self.check(start, ends),
        FeelType::Range(ends) => self.check(start, ends) && self.check(end, ends),
        _ => false,
      },
      // a comparison with one endpoint is a range
      Value::UnaryGreater(end) | Value::UnaryGreaterOrEqual(end) | Value::UnaryLess(end) | Value::UnaryLessOrEqual(end) | Value::UnaryEqual(end) | Value::UnaryNotEqual(end) => {
        match target {
          FeelType::Range(ends) => self.check(end, ends),
          _ => false,
        }
      }
      Value::List(items) => match target {
        FeelType::List(item_type) => items.iter().all(|item| self.check(item, item_type)),
        _ => false,
      },
      Value::Context(ctx) => match target {
        FeelType::Context(entries) => entries
          .iter()
          .all(|(name, entry_type)| ctx.get_entry(name).is_some_and(|entry| self.check(entry, entry_type))),
        _ => false,
      },
      Value::FunctionDefinition(function) => match target {
        // the parameters are contravariant: the target's parameter types conform to the
        // function's, and the result covariant (10.3.2.9.2). A result type that is Any is one
        // not declared: the result type of a function literal is not inferred, so it is not
        // held against the target
        FeelType::Function(parameter_types, result_type) => {
          function.parameters().len() == parameter_types.len()
            && (*function.result_type() == FeelType::Any || function.result_type().is_conformant(result_type))
            && function
              .parameters()
              .iter()
              .zip(parameter_types)
              .all(|((_, parameter_type), target_type)| target_type.is_conformant(parameter_type))
        }
        _ => false,
      },
      Value::BuiltInFunction(bif) => match target {
        FeelType::Function(parameter_types, result_type) => crate::signatures::conforms(bif, parameter_types, result_type),
        _ => false,
      },
      _ => false,
    }
  }
}
