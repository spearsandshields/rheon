//! The types of an analysis, kept in a table and referred to by index.
//!
//! A type that holds other types (a list, a range, a context, a function) holds their indexes,
//! so a type is made once however often it is read, and two types that are the same have the
//! same index: comparing two types compares two numbers.

use crate::infer::MAX_VALUE_DEPTH;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// A type of the table of one typing ([super::Typing]): a type of one typing means nothing to
/// another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(u32);

impl TypeId {
  /// The type that is not known.
  pub const ANY: TypeId = ANY;
  /// The type of numbers.
  pub const NUMBER: TypeId = NUMBER;
}

/// What a type is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
  Any,
  Null,
  Boolean,
  Number,
  String,
  Date,
  DateTime,
  Time,
  DaysAndTimeDuration,
  YearsAndMonthsDuration,
  Comparable,
  Temporal,
  Duration,
  /// A list of items of a type.
  List(TypeId),
  /// A range of endpoints of a type.
  Range(TypeId),
  /// A context: its entries, sorted by name.
  Context(Box<[(Name, TypeId)]>),
  /// A function: the types of its parameters, and of its result.
  Function(Box<[TypeId]>, TypeId),
}

/// The types that hold no other type: every table has them, at these indexes.
static SIMPLE: [Kind; 13] = [
  Kind::Any,
  Kind::Null,
  Kind::Boolean,
  Kind::Number,
  Kind::String,
  Kind::Date,
  Kind::DateTime,
  Kind::Time,
  Kind::DaysAndTimeDuration,
  Kind::YearsAndMonthsDuration,
  Kind::Comparable,
  Kind::Temporal,
  Kind::Duration,
];

pub(crate) const ANY: TypeId = TypeId(0);
pub(crate) const NULL: TypeId = TypeId(1);
pub(crate) const BOOLEAN: TypeId = TypeId(2);
pub(crate) const NUMBER: TypeId = TypeId(3);
pub(crate) const STRING: TypeId = TypeId(4);
const DATE: TypeId = TypeId(5);
const DATE_TIME: TypeId = TypeId(6);
const TIME: TypeId = TypeId(7);
const DAYS_AND_TIME_DURATION: TypeId = TypeId(8);
const YEARS_AND_MONTHS_DURATION: TypeId = TypeId(9);
const COMPARABLE: TypeId = TypeId(10);
const TEMPORAL: TypeId = TypeId(11);
const DURATION: TypeId = TypeId(12);

/// The most types that hold each other in one type (`list<list<…>>`, a function that gives a
/// function, a context of contexts) that a walk of the type follows: what the type holds below
/// that is read as `Any`. The entries of a context literal can each hold the type of the entry
/// before, so the depth of a type is not bounded by the depth of the tree.
pub(crate) const MAX_TYPE_DEPTH: usize = 256;

/// The most types that [Types::feel_type] writes out: what is beyond is `Any`. A type of the
/// table can hold one type at many places (a context whose two entries are the same context,
/// and so on down), which a [FeelType] writes out at each.
const MAX_WRITTEN: usize = 100_000;

/// The table of the types of one typing. It holds the types that hold other types, after the
/// indexes of [SIMPLE]; a table that only ever meets simple types allocates nothing.
pub(crate) struct Types {
  /// The types made, in the order of their indexes.
  kinds: Vec<Rc<Kind>>,
  /// Each type made with its index.
  made: HashMap<Rc<Kind>, TypeId>,
  /// The types of the lists and contexts of a scope read so far, by their place in memory and
  /// their depth in the value read: a value that a scope holds at many places is read once.
  values: HashMap<(usize, usize), TypeId>,
}

impl Types {
  pub(crate) fn new() -> Self {
    Self {
      kinds: Vec::new(),
      made: HashMap::new(),
      values: HashMap::new(),
    }
  }

  /// What the type `id` is; `Any` for a type that is none of this table.
  pub(crate) fn kind(&self, id: TypeId) -> &Kind {
    let index = id.0 as usize;
    match SIMPLE.get(index) {
      Some(simple) => simple,
      None => self.kinds.get(index - SIMPLE.len()).map_or(&SIMPLE[0], |kind| kind),
    }
  }

  /// The type that `kind` is: the one of the table when it has it.
  fn make(&mut self, kind: Kind) -> TypeId {
    if let Some(&id) = self.made.get(&kind) {
      return id;
    }
    let id = TypeId((SIMPLE.len() + self.kinds.len()) as u32);
    let kind = Rc::new(kind);
    self.kinds.push(Rc::clone(&kind));
    self.made.insert(kind, id);
    id
  }

  pub(crate) fn list(&mut self, item: TypeId) -> TypeId {
    self.make(Kind::List(item))
  }

  pub(crate) fn range(&mut self, endpoint: TypeId) -> TypeId {
    self.make(Kind::Range(endpoint))
  }

  /// The context of the entries of `entries`, which holds each name once.
  pub(crate) fn context(&mut self, entries: BTreeMap<Name, TypeId>) -> TypeId {
    self.make(Kind::Context(entries.into_iter().collect()))
  }

  pub(crate) fn function(&mut self, parameters: Vec<TypeId>, result: TypeId) -> TypeId {
    self.make(Kind::Function(parameters.into(), result))
  }

  /// The type of the table that `feel_type` is.
  pub(crate) fn of(&mut self, feel_type: &FeelType) -> TypeId {
    match feel_type {
      FeelType::Any => ANY,
      FeelType::Null => NULL,
      FeelType::Boolean => BOOLEAN,
      FeelType::Number => NUMBER,
      FeelType::String => STRING,
      FeelType::Date => DATE,
      FeelType::DateTime => DATE_TIME,
      FeelType::Time => TIME,
      FeelType::DaysAndTimeDuration => DAYS_AND_TIME_DURATION,
      FeelType::YearsAndMonthsDuration => YEARS_AND_MONTHS_DURATION,
      FeelType::Comparable => COMPARABLE,
      FeelType::Temporal => TEMPORAL,
      FeelType::Duration => DURATION,
      FeelType::List(item) => {
        let item = self.of(item);
        self.list(item)
      }
      FeelType::Range(endpoint) => {
        let endpoint = self.of(endpoint);
        self.range(endpoint)
      }
      FeelType::Context(entries) => {
        let entries = entries.iter().map(|(name, entry)| (name.clone(), self.of(entry))).collect();
        self.make(Kind::Context(entries))
      }
      FeelType::Function(parameters, result) => {
        let parameters = parameters.iter().map(|parameter| self.of(parameter)).collect();
        let result = self.of(result);
        self.make(Kind::Function(parameters, result))
      }
    }
  }

  /// The type `id` as a [FeelType], of at most [MAX_WRITTEN] types.
  pub(crate) fn feel_type(&self, id: TypeId) -> FeelType {
    self.written(id, &mut { MAX_WRITTEN }, 0)
  }

  /// The type `id` as a [FeelType] of at most `left` types, read from the depth `depth`: what
  /// is beyond either is `Any`.
  pub(crate) fn written(&self, id: TypeId, left: &mut usize, depth: usize) -> FeelType {
    if *left == 0 || depth >= MAX_TYPE_DEPTH {
      return FeelType::Any;
    }
    *left -= 1;
    let depth = depth + 1;
    match self.kind(id) {
      Kind::List(item) => FeelType::List(Box::new(self.written(*item, left, depth))),
      Kind::Range(endpoint) => FeelType::Range(Box::new(self.written(*endpoint, left, depth))),
      Kind::Context(entries) => FeelType::Context(entries.iter().map(|(name, entry)| (name.clone(), self.written(*entry, left, depth))).collect()),
      Kind::Function(parameters, result) => FeelType::Function(
        parameters.iter().map(|parameter| self.written(*parameter, left, depth)).collect(),
        Box::new(self.written(*result, left, depth)),
      ),
      _ => self.simple(id),
    }
  }

  /// The type `id` as far as the rules of the operators read it: a type that holds no other
  /// type as it is, and a list, a range, a context and a function without what they hold.
  pub(crate) fn simple(&self, id: TypeId) -> FeelType {
    match self.kind(id) {
      Kind::Any => FeelType::Any,
      Kind::Null => FeelType::Null,
      Kind::Boolean => FeelType::Boolean,
      Kind::Number => FeelType::Number,
      Kind::String => FeelType::String,
      Kind::Date => FeelType::Date,
      Kind::DateTime => FeelType::DateTime,
      Kind::Time => FeelType::Time,
      Kind::DaysAndTimeDuration => FeelType::DaysAndTimeDuration,
      Kind::YearsAndMonthsDuration => FeelType::YearsAndMonthsDuration,
      Kind::Comparable => FeelType::Comparable,
      Kind::Temporal => FeelType::Temporal,
      Kind::Duration => FeelType::Duration,
      Kind::List(_) => FeelType::List(Box::new(FeelType::Any)),
      Kind::Range(_) => FeelType::Range(Box::new(FeelType::Any)),
      Kind::Context(_) => FeelType::Context(BTreeMap::new()),
      Kind::Function(..) => FeelType::Function(vec![], Box::new(FeelType::Any)),
    }
  }

  /// The type of a value of a scope as an editor shows it: of a context the types of its
  /// entries, of a list the type of its first item (the items of a list of a host are of one
  /// kind; reading them all would cost what the list is long).
  pub(crate) fn of_value(&mut self, value: &Value) -> TypeId {
    self.value(value, 0)
  }

  fn value(&mut self, value: &Value, depth: usize) -> TypeId {
    if depth >= MAX_VALUE_DEPTH {
      return ANY;
    }
    // a list or a context is read once at each depth, at however many places the scope holds it
    let place = (value as *const Value as usize, depth);
    if matches!(value, Value::Context(_) | Value::List(_))
      && let Some(&known) = self.values.get(&place)
    {
      return known;
    }
    match value {
      Value::Context(context) => {
        let mut entries: Vec<(Name, TypeId)> = context.iter().map(|(name, value)| (name.clone(), self.value(value, depth + 1))).collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let made = self.make(Kind::Context(entries.into()));
        self.values.insert(place, made);
        made
      }
      Value::List(items) => {
        let item = items.first().map_or(ANY, |item| self.value(item, depth + 1));
        let made = self.list(item);
        self.values.insert(place, made);
        made
      }
      Value::Null(_) => ANY,
      // a type in the place of a value says the type of the name (an outline, a DMN declaration)
      Value::FeelType(feel_type) => self.of(feel_type),
      other => self.of(&other.type_of()),
    }
  }

  /// The type of the member `name` of a value of the type `base` (an entry of a context, a
  /// property of a date, a time, a duration or a range, and for a list the list of the members
  /// of its items), `Any` when the type has none of this name.
  pub(crate) fn member(&mut self, base: TypeId, name: &str) -> TypeId {
    self.member_of(base, name, 0).unwrap_or(ANY)
  }

  /// The members that a value of the type `base` has after a dot, with their types, in the
  /// order an editor lists them: the entries of a context by name, the properties of a date, a
  /// time, a duration and a range, and for a list the members of its items, each as the list of
  /// its values.
  pub(crate) fn members(&mut self, base: TypeId) -> Vec<(String, TypeId)> {
    self.members_of(base, 0)
  }

  /// [Types::members] of a type read at the depth `depth` of lists.
  fn members_of(&mut self, base: TypeId, depth: usize) -> Vec<(String, TypeId)> {
    let number = |names: &[&str]| names.iter().map(|name| (name.to_string(), NUMBER)).collect::<Vec<_>>();
    let offset = [("time offset".to_string(), DAYS_AND_TIME_DURATION), ("timezone".to_string(), STRING)];
    match self.kind(base) {
      Kind::Context(entries) => entries.iter().map(|(name, entry)| (name.to_string(), *entry)).collect(),
      Kind::List(_) if depth >= MAX_TYPE_DEPTH => vec![],
      Kind::List(item) => {
        let item = *item;
        self.members_of(item, depth + 1).into_iter().map(|(name, member)| (name, self.list(member))).collect()
      }
      Kind::Date => number(&["year", "month", "day", "weekday", "value"]),
      Kind::DateTime => number(&["year", "month", "day", "weekday", "hour", "minute", "second", "value"])
        .into_iter()
        .chain(offset)
        .collect(),
      Kind::Time => number(&["hour", "minute", "second", "value"]).into_iter().chain(offset).collect(),
      Kind::YearsAndMonthsDuration => number(&["years", "months", "value"]),
      Kind::DaysAndTimeDuration => number(&["days", "hours", "minutes", "seconds", "value"]),
      Kind::Range(endpoint) => vec![
        ("start".to_string(), *endpoint),
        ("end".to_string(), *endpoint),
        ("start included".to_string(), BOOLEAN),
        ("end included".to_string(), BOOLEAN),
      ],
      _ => vec![],
    }
  }

  /// [Types::member] of a type read at the depth `depth` of lists, None when the type has no
  /// member of this name.
  fn member_of(&mut self, base: TypeId, name: &str, depth: usize) -> Option<TypeId> {
    let number = |names: &[&str]| names.contains(&name).then_some(NUMBER);
    let offset = || match name {
      "time offset" => Some(DAYS_AND_TIME_DURATION),
      "timezone" => Some(STRING),
      _ => None,
    };
    match self.kind(base) {
      Kind::Context(entries) => entries.binary_search_by(|(entry, _)| entry.as_str().cmp(name)).ok().map(|index| entries[index].1),
      Kind::List(_) if depth >= MAX_TYPE_DEPTH => None,
      Kind::List(item) => {
        let item = *item;
        let member = self.member_of(item, name, depth + 1)?;
        Some(self.list(member))
      }
      Kind::Date => number(&["year", "month", "day", "weekday", "value"]),
      Kind::DateTime => number(&["year", "month", "day", "weekday", "hour", "minute", "second", "value"]).or_else(offset),
      Kind::Time => number(&["hour", "minute", "second", "value"]).or_else(offset),
      Kind::YearsAndMonthsDuration => number(&["years", "months", "value"]),
      Kind::DaysAndTimeDuration => number(&["days", "hours", "minutes", "seconds", "value"]),
      Kind::Range(endpoint) => match name {
        "start" | "end" => Some(*endpoint),
        "start included" | "end included" => Some(BOOLEAN),
        _ => None,
      },
      _ => None,
    }
  }
}
