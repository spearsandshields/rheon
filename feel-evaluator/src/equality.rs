//! # FEEL equality of two values
//!
//! A value can hold the same list or context on several paths (`[x, x]` and `{a: x, b: x}` hold
//! `x` twice without a copy), so a value that a loop doubles through `partial` holds
//! exponentially many paths in a linear size. An equality walk that followed every path would
//! take exponential time; this one remembers the answer for each pair of shared lists or shared
//! contexts it compared, and compares a pair again only when it was not met before.

use rheon_feel::values::Value;
use rheon_feel::{FeelScope, FunctionBody, FunctionDefinition};
use rheon_feel_temporal::EqualityKey;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher, RandomState};
use std::sync::Arc;

/// The key of a pair of lists of one length or of contexts with as many entries, when either side
/// is shared ([Value::is_shared]): the addresses of their nodes ([Value::node_address]). `None` for
/// any other pair, which is compared without being remembered (a pair of lists of different
/// lengths or of contexts of different sizes is decided at once).
fn shared_pair(lhs: &Value, rhs: &Value) -> Option<(usize, usize)> {
  let same_size = match (lhs, rhs) {
    (Value::List(ls), Value::List(rs)) => ls.len() == rs.len(),
    (Value::Context(lc), Value::Context(rc)) => lc.len() == rc.len(),
    _ => false,
  };
  if same_size && (lhs.is_shared() || rhs.is_shared()) {
    lhs.node_address().zip(rhs.node_address())
  } else {
    None
  }
}

/// Evaluates the equality of two values: `None` when the values can not be compared.
///
/// Comparing with null tests if the other value is null, so it always has a result
/// (DMN 1.6, grammar rule 49.a). Other values are compared by kind (Table 54); the known
/// deviations are listed in docs/BACKLOG.md ("Equality and comparison follow-ups").
pub(crate) fn eval_ternary_equality(lhs: &Value, rhs: &Value) -> Option<bool> {
  Equality::default().compare(lhs, rhs)
}

/// One equality walk over borrowed values: the answers for the pairs of lists it compared where
/// either list shares its items, by the addresses of their items, and for the pairs of contexts
/// it compared where either context is shared, by the addresses of the contexts.
///
/// The memory lives as long as the walk, is created when the first shared pair is met, and is
/// never iterated. While the values are borrowed, no other items or contexts are stored at the
/// addresses it holds, and the items of a list and a context are never at one address, so a
/// pair of lists and a pair of contexts never have one key. Remembering only the pairs with a
/// shared side is enough: a pair is reached again only through two paths that meet in a list or
/// a context held twice, and that list or context is shared.
///
/// One walk serves every comparison of an operation that compares several pairs of values, such
/// as a built-in that compares the items of a list with one another or with a match: a pair of
/// shared lists met again below a later comparison is answered from memory as well.
#[derive(Default)]
pub(crate) struct Equality {
  shared: Option<HashMap<(usize, usize), Option<bool>>>,
  /// The answers for the pairs of function definitions compared, by their addresses: functions
  /// that close over the same functions under several names are compared once, as shared lists
  /// and contexts are, so a comparison stays linear in the size of the values.
  functions: Option<HashMap<(usize, usize), bool>>,
  /// Identity (`is`, 10.3.2.2) instead of equality: values of other kinds are not the same, times
  /// and date and times are the same only with the same fields and zone, and functions only as
  /// the same function. Never `None`.
  identity: bool,
}

/// Whether two values are the same element of the FEEL semantic domain (DMN 1.7 Beta 1, 10.3.2.2):
/// `is`, which "never results in null". Lists, contexts and ranges are the same when their parts
/// are; shared parts are compared once, as by equality.
pub(crate) fn identical(lhs: &Value, rhs: &Value) -> bool {
  Equality {
    shared: None,
    functions: None,
    identity: true,
  }
  .compare(lhs, rhs)
    == Some(true)
}

impl Equality {
  /// The equality of two values held by the values compared: remembered for a pair of lists or
  /// a pair of contexts with a shared side, compared otherwise.
  fn nested(&mut self, lhs: &Value, rhs: &Value) -> Option<bool> {
    if let Some(key) = shared_pair(lhs, rhs) {
      if let Some(answer) = self.shared.as_ref().and_then(|shared| shared.get(&key)) {
        return *answer;
      }
      let answer = self.compare(lhs, rhs);
      self.shared.get_or_insert_with(HashMap::new).insert(key, answer);
      return answer;
    }
    self.compare(lhs, rhs)
  }

  /// The equality of two values ([eval_ternary_equality]), with the values they hold compared
  /// through this walk. The two values are never looked up: each comparison of an operation is
  /// made once, and the pairs met again are the ones below them.
  /// The equality of two endpoints of ranges: an undefined endpoint equals only an undefined one.
  fn endpoints(&mut self, lhs: Option<&Value>, rhs: Option<&Value>) -> Option<bool> {
    match (lhs, rhs) {
      (None, None) => Some(true),
      (Some(lhs), Some(rhs)) => self.nested(lhs, rhs),
      _ => Some(false),
    }
  }

  /// Whether two function definitions are the same function (Table 54): the same parameters with
  /// their types, the same result type, the same body, and equal values of the names they close
  /// over. Two evaluations of one function text in one scope give equal functions.
  fn functions(&mut self, lf: &Arc<FunctionDefinition>, rf: &Arc<FunctionDefinition>) -> bool {
    if Arc::ptr_eq(lf, rf) {
      return true;
    }
    let key = (Arc::as_ptr(lf).addr(), Arc::as_ptr(rf).addr());
    if let Some(answer) = self.functions.as_ref().and_then(|functions| functions.get(&key)) {
      return *answer;
    }
    let answer = self.different_functions(lf, rf);
    self.functions.get_or_insert_with(HashMap::new).insert(key, answer);
    answer
  }

  /// [Equality::functions] of two function definitions that are not one value, not remembered.
  fn different_functions(&mut self, lf: &Arc<FunctionDefinition>, rf: &Arc<FunctionDefinition>) -> bool {
    if lf.parameters() != rf.parameters() || lf.result_type() != rf.result_type() || lf.external != rf.external || lf.closure != rf.closure || !same_bodies(&lf.body, &rf.body) {
      return false;
    }
    let (ls, rs) = (&lf.closure_ctx, &rf.closure_ctx);
    Arc::ptr_eq(ls, rs)
      || (ls.len() == rs.len()
        && ls
          .iter()
          .all(|(key, value1)| rs.get_entry(key).is_some_and(|value2| self.nested(value1, value2) == Some(true))))
  }

  pub(crate) fn compare(&mut self, lhs: &Value, rhs: &Value) -> Option<bool> {
    let answer = self.compare_kinds(lhs, rhs);
    if self.identity { Some(answer == Some(true)) } else { answer }
  }

  fn compare_kinds(&mut self, lhs: &Value, rhs: &Value) -> Option<bool> {
    if lhs.is_null() || rhs.is_null() {
      return Some(lhs.is_null() && rhs.is_null());
    }
    match lhs {
      Value::Boolean(ls) => match rhs {
        Value::Boolean(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::Number(ls) => match rhs {
        Value::Number(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::String(ls) => match rhs {
        Value::String(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::Context(ls) => match rhs {
        Value::Context(rs) => {
          // the same keys, and under every key equal values (Table 54): values that are not
          // equal or can not be compared make the contexts unequal, as the items of lists do
          let equal = ls.len() == rs.len()
            && ls
              .iter()
              .all(|(key, value1)| rs.get_entry(key).is_some_and(|value2| self.nested(value1, value2) == Some(true)));
          Some(equal)
        }
        _ => None,
      },
      Value::Date(ls) => match rhs {
        Value::Date(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::Time(ls) => match rhs {
        Value::Time(rs) if self.identity => Some(ls.is(rs)),
        Value::Time(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::DateTime(ls) => match rhs {
        Value::DateTime(rs) if self.identity => Some(ls.is(rs)),
        Value::DateTime(rs) => Some(*ls == *rs),
        _ => None,
      },
      // functions (Table 54): internal ones are equal when they have the same parameters, body and
      // scope, external ones the same parameters and mapping; a function is no other function
      Value::BuiltInFunction(lb) => match rhs {
        Value::BuiltInFunction(rb) => Some(lb == rb),
        rhs if is_function(rhs) => Some(false),
        _ => None,
      },
      Value::FunctionDefinition(lf) => match rhs {
        Value::FunctionDefinition(rf) => Some(self.functions(lf, rf)),
        rhs if is_function(rhs) => Some(false),
        _ => None,
      },
      Value::ExternalJavaFunction(l_class, l_method) => match rhs {
        Value::ExternalJavaFunction(r_class, r_method) => Some(l_class == r_class && l_method == r_method),
        rhs if is_function(rhs) => Some(false),
        _ => None,
      },
      Value::ExternalPmmlFunction(l_iri, l_model) => match rhs {
        Value::ExternalPmmlFunction(r_iri, r_model) => Some(l_iri == r_iri && l_model == r_model),
        rhs if is_function(rhs) => Some(false),
        _ => None,
      },
      Value::DaysAndTimeDuration(ls) => match rhs {
        Value::DaysAndTimeDuration(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::YearsAndMonthsDuration(ls) => match rhs {
        Value::YearsAndMonthsDuration(rs) => Some(*ls == *rs),
        _ => None,
      },
      Value::List(ls) => match rhs {
        Value::List(rs) => {
          if ls.len() == rs.len() {
            // items that are not equal or can not be compared make the lists unequal
            Some(ls.iter().zip(rs.iter()).all(|(l, r)| self.nested(l, r) == Some(true)))
          } else {
            Some(false)
          }
        }
        _ => None,
      },
      Value::UnaryEqual(l_end) => match rhs {
        Value::UnaryEqual(r_end) => self.nested(l_end, r_end),
        _ => Some(false),
      },
      Value::UnaryNotEqual(l_end) => match rhs {
        Value::UnaryNotEqual(r_end) => self.nested(l_end, r_end),
        _ => Some(false),
      },
      // a range, written with two endpoints or as a comparison with one (clause 10.3.2.7)
      Value::Range(..) | Value::UnaryLess(_) | Value::UnaryLessOrEqual(_) | Value::UnaryGreater(_) | Value::UnaryGreaterOrEqual(_) => {
        let (l_start, l_start_included, l_end, l_end_included) = range_parts(lhs)?;
        let (r_start, r_start_included, r_end, r_end_included) = range_parts(rhs)?;
        if l_start_included != r_start_included || l_end_included != r_end_included {
          return Some(false);
        }
        match self.endpoints(l_start, r_start) {
          Some(true) => self.endpoints(l_end, r_end),
          _ => Some(false),
        }
      }
      _ => None,
    }
  }
}

/// The start, whether it is included, the end and whether it is included of a range: of an
/// interval, and of a comparison with one endpoint, whose other endpoint is undefined and not
/// included (DMN 1.6 Table 42). An undefined endpoint is None; an endpoint written as `null` is
/// a value. None for a value that is no range.
#[allow(clippy::type_complexity)]
pub(crate) fn range_parts(value: &Value) -> Option<(Option<&Value>, bool, Option<&Value>, bool)> {
  match value {
    Value::Range(start, start_type, end, end_type) => Some((
      (!start_type.undefined()).then_some(start.as_ref()),
      start_type.closed(),
      (!end_type.undefined()).then_some(end.as_ref()),
      end_type.closed(),
    )),
    Value::UnaryLess(end) => Some((None, false, Some(end), false)),
    Value::UnaryLessOrEqual(end) => Some((None, false, Some(end), true)),
    Value::UnaryGreater(start) => Some((Some(start), false, None, false)),
    Value::UnaryGreaterOrEqual(start) => Some((Some(start), true, None, false)),
    _ => None,
  }
}

/// The number of values a [Seen] holds before it looks them up by hash: below it, comparing with
/// each value held is faster than hashing.
const SEEN_BY_HASH: usize = 16;

/// The values met so far, with no two of them equal: what `distinct values`, `union` and
/// `intersection` keep. Whether a value is held is decided by FEEL equality ([Equality::compare]
/// is true), as when it is compared with every value held; the hash only chooses the values it
/// is compared with, so the time for `n` values is linear in `n`, not quadratic.
///
/// The hasher is seeded per set, so the items of a list written to collide can not be chosen.
#[derive(Default)]
pub(crate) struct Seen<'a> {
  equality: Equality,
  /// The values held, in the order they were met.
  values: Vec<&'a Value>,
  /// Above [SEEN_BY_HASH] values: the hasher, the index of the last value held with each hash,
  /// and for each value held the index of the one before it with the same hash.
  index: Option<(RandomState, HashMap<u64, usize>)>,
  before: Vec<usize>,
  /// Above [SEEN_BY_HASH] values: the times and dates and times held, by their equality keys
  /// ([EqualityKey]): equal ones meet under one key, though `=` is no equivalence over them.
  temporal: Temporal,
}

/// The times and dates and times of a [Seen], by kind (0 for a time, 1 for a date and time): by
/// wall-clock time (every one that has it), by instant (the zoned ones), and those compared with
/// every value of their kind ([EqualityKey::Loose]).
#[derive(Default)]
struct Temporal {
  by_wall: HashMap<(u8, i64), Vec<usize>>,
  by_instant: HashMap<(u8, i64), Vec<usize>>,
  loose: Vec<usize>,
}

/// The kind and the equality key of a time or a date and time.
fn temporal_key(value: &Value) -> Option<(u8, EqualityKey)> {
  match value {
    Value::Time(time) => Some((0, time.equality_key())),
    Value::DateTime(date_time) => Some((1, date_time.equality_key())),
    _ => None,
  }
}

impl Temporal {
  /// Holds the value at `at` under its key.
  fn insert(&mut self, kind: u8, key: EqualityKey, at: usize) {
    match key {
      EqualityKey::Local { wall } => self.by_wall.entry((kind, wall)).or_default().push(at),
      EqualityKey::Zoned { wall, instant } => {
        self.by_wall.entry((kind, wall)).or_default().push(at);
        self.by_instant.entry((kind, instant)).or_default().push(at);
      }
      EqualityKey::Loose => self.loose.push(at),
    }
  }

  /// The values that may be equal to a value of `kind` and `key`: those of its keys, and the loose
  /// ones; a value with no key of its own (loose) may equal any of its kind.
  fn candidates(&self, kind: u8, key: EqualityKey) -> Vec<usize> {
    let mut found = Vec::new();
    let mut add = |map: &HashMap<(u8, i64), Vec<usize>>, k: i64| found.extend(map.get(&(kind, k)).into_iter().flatten().copied());
    match key {
      EqualityKey::Local { wall } => add(&self.by_wall, wall),
      EqualityKey::Zoned { wall, instant } => {
        add(&self.by_wall, wall);
        add(&self.by_instant, instant);
      }
      EqualityKey::Loose => {
        for ((k, _), at) in &self.by_wall {
          if *k == kind {
            found.extend(at.iter().copied());
          }
        }
      }
    }
    found.extend(self.loose.iter().copied());
    found
  }
}

/// Marks the end of a chain of values with one hash.
const NONE: usize = usize::MAX;

impl<'a> Seen<'a> {
  /// The set of the values of a list.
  pub(crate) fn of(items: &'a [Value]) -> Self {
    let mut seen = Self::default();
    for item in items {
      seen.insert(item);
    }
    seen
  }

  /// True when a value equal to this one is held.
  pub(crate) fn contains(&mut self, value: &Value) -> bool {
    if self.index.is_some()
      && let Some((kind, key)) = temporal_key(value)
    {
      let candidates = self.temporal.candidates(kind, key);
      return candidates.into_iter().any(|at| self.equality.compare(self.values[at], value) == Some(true));
    }
    match &self.index {
      None => self.values.iter().any(|held| self.equality.compare(held, value) == Some(true)),
      Some((state, last)) => {
        let mut at = last.get(&hash(state, value)).copied().unwrap_or(NONE);
        while at != NONE {
          if self.equality.compare(self.values[at], value) == Some(true) {
            return true;
          }
          at = self.before[at];
        }
        false
      }
    }
  }

  /// Holds the value unless a value equal to it is held: true when it was added.
  pub(crate) fn insert(&mut self, value: &'a Value) -> bool {
    if self.contains(value) {
      return false;
    }
    self.values.push(value);
    match &mut self.index {
      Some((state, last)) => {
        let at = self.values.len() - 1;
        // a time or a date and time is held by its keys, not in the chains of the hash
        match temporal_key(value) {
          Some((kind, key)) => {
            self.temporal.insert(kind, key, at);
            self.before.push(NONE);
          }
          None => self.before.push(last.insert(hash(state, value), at).unwrap_or(NONE)),
        }
      }
      None if self.values.len() > SEEN_BY_HASH => {
        let state = RandomState::new();
        let mut last = HashMap::new();
        let mut temporal = Temporal::default();
        self.before = self
          .values
          .iter()
          .enumerate()
          .map(|(at, held)| match temporal_key(held) {
            Some((kind, key)) => {
              temporal.insert(kind, key, at);
              NONE
            }
            None => last.insert(hash(&state, held), at).unwrap_or(NONE),
          })
          .collect();
        self.index = Some((state, last));
        self.temporal = temporal;
      }
      None => {}
    }
    true
  }
}

/// A hash that agrees with [Equality::compare]: two values that compare as equal have one hash.
fn hash(state: &RandomState, value: &Value) -> u64 {
  let mut hasher = state.build_hasher();
  digest(value, &mut hasher, true);
  hasher.finish()
}

/// Whether two function bodies are the same body (Table 54): externally defined functions have the
/// same mapping (a Java class and method signature, a PMML document and model), which their bodies
/// give when evaluated, recording nothing; other bodies are the same code ([FunctionBody::is_same]).
/// An ONNX body gives no mapping, and is the same only as itself.
fn same_bodies(lhs: &FunctionBody, rhs: &FunctionBody) -> bool {
  if let (FunctionBody::External(_), FunctionBody::External(_)) = (lhs, rhs) {
    let scope = FeelScope::default();
    match (lhs.evaluate(&scope), rhs.evaluate(&scope)) {
      (Value::ExternalJavaFunction(l_class, l_method), Value::ExternalJavaFunction(r_class, r_method)) => return l_class == r_class && l_method == r_method,
      (Value::ExternalPmmlFunction(l_iri, l_model), Value::ExternalPmmlFunction(r_iri, r_model)) => return l_iri == r_iri && l_model == r_model,
      _ => {}
    }
  }
  lhs.is_same(rhs)
}

/// Whether `value` is a function of any kind.
fn is_function(value: &Value) -> bool {
  matches!(
    value,
    Value::BuiltInFunction(_) | Value::FunctionDefinition(_) | Value::ExternalJavaFunction(..) | Value::ExternalPmmlFunction(..)
  )
}

/// Writes what the hash of a value is taken of. A list or a context adds its items when `items`
/// is set and only its size otherwise: the items of the items are not visited, so the work is
/// bounded by the size of the value itself, however many paths lead through shared parts below
/// it. The entries of a context are met in the order of their names, however it was written. A value
/// whose equality is not that of its fields (a time, a date and time, a range) adds its kind
/// only, and is then compared with every value of its kind, as it was before.
fn digest(value: &Value, hasher: &mut impl Hasher, items: bool) {
  match value {
    Value::Null(_) => hasher.write_u8(0),
    Value::Boolean(boolean) => {
      hasher.write_u8(1);
      boolean.hash(hasher);
    }
    Value::Number(number) => {
      hasher.write_u8(2);
      number.hash(hasher);
    }
    Value::String(string) => {
      hasher.write_u8(3);
      string.hash(hasher);
    }
    Value::Date(date) => {
      hasher.write_u8(4);
      date.as_tuple().hash(hasher);
    }
    Value::DaysAndTimeDuration(duration) => {
      hasher.write_u8(5);
      duration.as_nanos().hash(hasher);
    }
    Value::YearsAndMonthsDuration(duration) => {
      hasher.write_u8(6);
      duration.as_months().hash(hasher);
    }
    Value::List(list) => {
      hasher.write_u8(7);
      list.len().hash(hasher);
      if items {
        for item in list.iter() {
          digest(item, hasher, false);
        }
      }
    }
    Value::Context(context) => {
      hasher.write_u8(8);
      context.len().hash(hasher);
      if items {
        for (name, entry) in context.iter() {
          name.hash(hasher);
          digest(entry, hasher, false);
        }
      }
    }
    _ => hasher.write_u8(9),
  }
}

#[cfg(test)]
mod seen_tests {
  use super::*;
  use rheon_feel::FeelScope;

  /// A small generator of numbers that is the same on every run.
  struct Generator(u64);

  impl Generator {
    fn next(&mut self, below: u64) -> u64 {
      self.0 ^= self.0 << 13;
      self.0 ^= self.0 >> 7;
      self.0 ^= self.0 << 17;
      self.0 % below
    }

    fn pick<'a>(&mut self, texts: &[&'a str]) -> &'a str {
      texts[self.next(texts.len() as u64) as usize]
    }

    /// The text of a value of few possible values, so that equal ones are met often: equal
    /// numbers of different cohorts, contexts with their entries in another order, values that
    /// can not be compared with one another, and values that are compared by more than fields.
    fn value(&mut self, depth: u32) -> String {
      const SCALARS: &[&str] = &[
        "1",
        "1.0",
        "1.00",
        "-1",
        "0",
        "0.0",
        "-0",
        "100",
        "1e2",
        "0.1",
        "0.10",
        "12345678901234567890.5",
        "12345678901234567890.50",
        r#""a""#,
        r#""A""#,
        r#""""#,
        r#""1""#,
        "true",
        "false",
        "null",
        r#"date("2024-01-02")"#,
        "date(2024, 1, 2)",
        r#"date("2024-01-03")"#,
        r#"duration("P1D")"#,
        r#"duration("PT24H")"#,
        r#"duration("P1Y")"#,
        r#"duration("P12M")"#,
        r#"time("10:00:00+02:00")"#,
        r#"time("08:00:00Z")"#,
        r#"time("10:00:00")"#,
        r#"date and time("2024-01-02T10:00:00+02:00")"#,
        r#"date and time("2024-01-02T08:00:00Z")"#,
        "[1..2]",
        "(1..2]",
        "[1.0..2]",
      ];
      if depth == 0 || self.next(3) > 0 {
        return self.pick(SCALARS).to_string();
      }
      let size = self.next(4);
      if self.next(2) == 0 {
        let items: Vec<String> = (0..size).map(|_| self.value(depth - 1)).collect();
        format!("[{}]", items.join(", "))
      } else {
        let mut names = vec!["a", "b", "c"];
        names.truncate(size.min(3) as usize);
        if self.next(2) == 0 {
          names.reverse();
        }
        let entries: Vec<String> = names.iter().map(|name| format!("{name}: {}", self.value(depth - 1))).collect();
        format!("{{{}}}", entries.join(", "))
      }
    }
  }

  fn values(text: &str) -> Vec<Value> {
    let scope = FeelScope::default();
    let node = rheon_feel_parser::parse_expression(&scope, text).unwrap();
    match crate::evaluate(&scope, &node).into_result().unwrap() {
      Value::List(items) => items.iter().cloned().collect(),
      other => panic!("expected a list for {text}, found {other}"),
    }
  }

  /// The values a list keeps when each is compared with every value kept before it.
  fn kept_by_comparing(items: &[Value]) -> Vec<usize> {
    let mut equality = Equality::default();
    let mut kept: Vec<usize> = vec![];
    for (at, item) in items.iter().enumerate() {
      if kept.iter().all(|held| equality.compare(&items[*held], item) != Some(true)) {
        kept.push(at);
      }
    }
    kept
  }

  #[test]
  fn a_set_keeps_the_values_that_comparing_every_pair_keeps() {
    let rounds = if std::env::var_os("RHEON_FULL_TESTS").is_some() { 20_000 } else { 400 };
    let mut generator = Generator(0x9E37_79B9_7F4A_7C15);
    for _ in 0..rounds {
      let size = 1 + generator.next(120);
      let texts: Vec<String> = (0..size).map(|_| generator.value(3)).collect();
      let text = format!("[{}]", texts.join(", "));
      let items = values(&text);
      let mut seen = Seen::default();
      let kept: Vec<usize> = items.iter().enumerate().filter(|(_, item)| seen.insert(item)).map(|(at, _)| at).collect();
      assert_eq!(kept_by_comparing(&items), kept, "{text}");
      for item in &items {
        assert!(seen.contains(item), "{item} of {text}");
      }
    }
  }

  #[test]
  fn contexts_with_their_entries_in_another_order_are_one_value() {
    let one: Vec<String> = (1..=40).map(|i| format!(r#"{{a: {i}, b: "x", c: [{i}, null]}}"#)).collect();
    let other: Vec<String> = (1..=40).map(|i| format!(r#"{{c: [{i}.0, null], b: "x", a: {i}.00}}"#)).collect();
    let items = values(&format!("[{}, {}]", one.join(", "), other.join(", ")));
    let mut seen = Seen::default();
    assert_eq!(40, items.iter().filter(|item| seen.insert(item)).count());
    assert_eq!(kept_by_comparing(&items).len(), 40);
  }

  #[test]
  fn equal_values_have_one_hash() {
    let state = RandomState::new();
    let mut generator = Generator(0x2545_F491_4F6C_DD1D);
    let texts: Vec<String> = (0..600).map(|_| generator.value(3)).collect();
    let items = values(&format!("[{}]", texts.join(", ")));
    let mut equality = Equality::default();
    let mut equal_pairs = 0;
    for a in &items {
      for b in &items {
        if equality.compare(a, b) == Some(true) {
          equal_pairs += 1;
          assert_eq!(hash(&state, a), hash(&state, b), "{a} and {b}");
        }
      }
    }
    assert!(equal_pairs > 2 * items.len(), "the generated values hold too few equal pairs: {equal_pairs}");
  }

  #[test]
  fn many_values_are_held_in_time_linear_in_their_number() {
    let items: Vec<Value> = values("for i in 1..200000 return i");
    let started = std::time::Instant::now();
    let mut seen = Seen::default();
    assert_eq!(items.len(), items.iter().filter(|item| seen.insert(item)).count());
    assert_eq!(0, items.iter().filter(|item| seen.insert(item)).count());
    // comparing every pair is 20,000,000,000 comparisons, which takes minutes
    assert!(started.elapsed().as_secs() < 20, "took {:?}", started.elapsed());
  }
}
