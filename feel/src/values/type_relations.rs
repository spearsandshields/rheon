//! # The type of a value related to a type, without building the type of the value
//!
//! [Value::type_of] builds the type of a value as a tree. A value can hold the same list or
//! context on several paths (`[x, x]` and `{a: x, b: x}` hold `x` twice without a copy), so a
//! value that a loop doubles through `partial` has exponentially many paths in a linear size, and
//! building its type, or checking each item against the type of the first, follows every path.
//! The relations here answer what the built type would answer, reading the value only as far as
//! the relation needs: the type of a value is the given type, the types of two values are equal,
//! the type of a value conforms to the type of another ([FeelType::is_conformant]), and
//! `instance of` ([FeelType::instance_of]).
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
        Value::Range(start, _, end, _) => Shape::Range(Ends::Of(start, end)),
        Value::Context(ctx) => Shape::Context(Entries::Of(ctx)),
        Value::FunctionDefinition(function) => Shape::Function(Cow::Owned(FeelType::Function(
          function.parameters().iter().map(|(_, feel_type)| feel_type.clone()).collect(),
          Box::new(function.result_type().clone()),
        ))),
        Value::ContextType(feel_type) | Value::ContextTypeEntry(_, feel_type) | Value::FeelType(feel_type) | Value::FormalParameter(_, feel_type) => Ty::Is(feel_type).shape(),
        Value::IntervalEnd(value, _) | Value::IntervalStart(value, _) => Ty::Of(value).shape(),
        Value::Boolean(_)
        | Value::UnaryGreater(_)
        | Value::UnaryGreaterOrEqual(_)
        | Value::UnaryLess(_)
        | Value::UnaryLessOrEqual(_)
        | Value::UnaryEqual(_)
        | Value::UnaryNotEqual(_) => Shape::Leaf(BOOLEAN),
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
  /// Whether the items of a list conform to its first item, by the address of the items.
  uniform: Option<HashMap<usize, bool>>,
  /// The answers for the pairs with a shared list or a shared context on either side.
  pairs: Option<HashMap<(Relation, Node, Node), bool>>,
}

impl<'a> Relations {
  /// The type of the items of a list type: for a list value, `Null` when it is empty, the type
  /// of its first item when the types of the other items conform to it, `Any` otherwise.
  fn items(&mut self, items: Items<'a>) -> Ty<'a> {
    match items {
      Items::Is(feel_type) => Ty::Is(feel_type),
      Items::Of(list) => match list.split_first() {
        None => Ty::Is(NULL),
        Some((first, [])) => Ty::Of(first),
        Some((first, rest)) => {
          let alike = |relations: &mut Self| rest.iter().all(|item| relations.conforms(Ty::Of(item), Ty::Of(first)));
          // a list of scalars, or of containers of scalars, is cheap to decide again when it is
          // reached again; a deeper one is decided once, so that deciding it never repeats the
          // decisions below it
          let uniform = if list.nesting_depth() <= 2 {
            alike(self)
          } else {
            let address = list.address().expect("a list with items has an address");
            match self.uniform.as_ref().and_then(|uniform| uniform.get(&address)) {
              Some(uniform) => *uniform,
              None => {
                let uniform = alike(self);
                self.uniform.get_or_insert_with(HashMap::new).insert(address, uniform);
                uniform
              }
            }
          };
          if uniform { Ty::Of(first) } else { Ty::Is(ANY) }
        }
      },
    }
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
