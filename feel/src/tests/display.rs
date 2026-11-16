//! The text of lists, contexts and types is written as it goes, so a writer that stops early
//! stops the work, and the text itself is unchanged.

use crate::context::FeelContext;
use crate::values::Value;
use crate::{FeelNumber, FeelType, Name};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Write};

/// Accepts the first `limit` bytes, then refuses every write; counts every byte offered.
struct Stopping {
  limit: usize,
  accepted: usize,
  offered: usize,
}

impl Write for Stopping {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.offered += s.len();
    if self.accepted + s.len() > self.limit {
      return Err(fmt::Error);
    }
    self.accepted += s.len();
    Ok(())
  }
}

/// Asserts that `value` offers at most `element` bytes more than a writer that stops after 200
/// bytes accepts.
fn assert_stops(value: &impl Display, element: usize) {
  let mut sink = Stopping {
    limit: 200,
    accepted: 0,
    offered: 0,
  };
  assert!(write!(sink, "{value}").is_err());
  assert!(sink.offered <= 200 + element, "{} bytes offered", sink.offered);
}

/// More than any one element of the values below writes at once.
const ELEMENT: usize = 16;

fn long_list() -> Value {
  Value::List((1..=100_000).map(|i| Value::Number(FeelNumber::new(i, 0))).collect())
}

fn context_of(name: &str, value: Value) -> FeelContext {
  let mut context = FeelContext::default();
  context.set_entry(&Name::from(name), value);
  context
}

#[test]
fn a_long_list_stops_when_the_writer_stops() {
  assert_stops(&long_list(), ELEMENT);
  assert_stops(&Value::ExpressionList(vec![long_list()]), ELEMENT);
}

#[test]
fn a_context_stops_when_the_writer_stops() {
  assert_stops(&context_of("a", long_list()), ELEMENT);
  assert_stops(&Value::from(context_of("a", long_list())), ELEMENT);
}

#[test]
fn a_context_type_stops_when_the_writer_stops() {
  let entries: BTreeMap<Name, FeelType> = (0..1_000).map(|i| (Name::from(format!("entry{i}")), FeelType::Number)).collect();
  assert_stops(&FeelType::Context(entries.clone()), ELEMENT);
  let parameters = vec![FeelType::Context(entries); 2];
  assert_stops(&FeelType::Function(parameters, Box::new(FeelType::Any)), ELEMENT);
}

#[test]
fn a_range_and_a_null_stop_when_the_writer_stops() {
  let long = Value::String("x".repeat(1_000).into());
  let range = Value::Range(Box::new(long.clone()), crate::IntervalType::Closed, Box::new(long), crate::IntervalType::Closed);
  assert_stops(&range, 1_002);
  assert_stops(&Value::List(vec![Value::Null(Some("x".repeat(1_000))); 2].into()), 1_006);
}

#[test]
fn the_texts_are_unchanged() {
  let list = Value::List(
    vec![
      Value::Number(FeelNumber::new(1, 0)),
      Value::String("a".into()),
      Value::Null(None),
      Value::List(vec![].into()),
    ]
    .into(),
  );
  assert_eq!(r#"[1, "a", null, []]"#, list.to_string());
  assert_eq!(r#"[[1, "a", null, []]]"#, Value::ExpressionList(vec![list.clone()]).to_string());
  let mut context = context_of("", Value::Number(FeelNumber::new(1, 0)));
  context.set_entry(&Name::from("b"), list);
  assert_eq!(r#"{"": 1, b: [1, "a", null, []]}"#, context.to_string());
  assert_eq!("{}", FeelContext::default().to_string());
  let entries: BTreeMap<Name, FeelType> = [("a", FeelType::Number), ("b", FeelType::List(Box::new(FeelType::String)))]
    .into_iter()
    .map(|(name, feel_type)| (Name::from(name), feel_type))
    .collect();
  assert_eq!("context<a: number, b: list<string>>", FeelType::Context(entries.clone()).to_string());
  assert_eq!("context<>", FeelType::Context(BTreeMap::new()).to_string());
  let function = FeelType::Function(vec![FeelType::Number, FeelType::Context(entries)], Box::new(FeelType::Boolean));
  assert_eq!("function<number, context<a: number, b: list<string>>>->boolean", function.to_string());
  assert_eq!("function<>->Any", FeelType::Function(vec![], Box::new(FeelType::Any)).to_string());
  let one = Box::new(Value::Number(FeelNumber::new(1, 0)));
  assert_eq!(
    "[1..1]",
    Value::Range(one.clone(), crate::IntervalType::Closed, one.clone(), crate::IntervalType::Closed).to_string()
  );
  assert_eq!(
    "(..1)",
    Value::Range(one.clone(), crate::IntervalType::OpenedUndef, one, crate::IntervalType::Opened).to_string()
  );
  assert_eq!("null(reason)", Value::Null(Some("reason".to_string())).to_string());
}
