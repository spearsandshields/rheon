//! Bounded messages: previews of arguments, numbers in scientific notation, the cap per message
//! and the type of a value written from the value.

use crate::closure::Closure;
use crate::context::FeelContext;
use crate::message::{arg, compose};
use crate::tests::type_relations::{doubled, nested_list_type};
use crate::values::{TypeOf, Value};
use crate::{FeelNumber, FeelScope, FeelType, FunctionBody, FunctionDefinition, IntervalType, Name, value_null, value_number};
use std::sync::Arc;

/// The 120-character preview of the list of the numbers from 1.
const LIST_PREVIEW: &str = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, ";

fn numbers(count: i64) -> Value {
  Value::List((1..=count).map(|i| Value::Number(FeelNumber::new(i, 0))).collect())
}

fn context(entries: impl IntoIterator<Item = (String, Value)>) -> FeelContext {
  let mut context = FeelContext::default();
  for (name, value) in entries {
    context.set_entry(&Name::from(name), value);
  }
  context
}

fn number(text: &str) -> FeelNumber {
  text.parse().unwrap()
}

#[test]
fn an_argument_of_at_most_120_characters_is_quoted_whole() {
  let text = "x".repeat(118);
  let value = Value::String(text.as_str().into());
  assert_eq!(value.to_string(), arg(&value).to_string());
  assert_eq!("y".repeat(120), arg(&"y".repeat(120)).to_string());
  assert_eq!("[1, 2, 3]", arg(&numbers(3)).to_string());
  assert_eq!("{a: 1}", arg(&context([("a".to_string(), value_number!(1))])).to_string());
  assert_eq!("context<a: number>", arg(&FeelType::Context([(Name::from("a"), FeelType::Number)].into())).to_string());
  assert_eq!("a b", arg(&Name::from("a b")).to_string());
  assert_eq!("", arg("").to_string());
}

#[test]
fn a_longer_argument_is_a_preview_with_its_size() {
  assert_eq!(LIST_PREVIEW, &arg(&numbers(1000)).to_string()[..120]);
  assert_eq!(format!("{LIST_PREVIEW}… (1000 items)"), arg(&numbers(1000)).to_string());
  assert_eq!(
    format!("[\"{}… (1 item)", "x".repeat(118)),
    arg(&Value::List(vec![Value::String("x".repeat(500).into())].into())).to_string()
  );
  let text = Value::String("x".repeat(5000).into());
  assert_eq!(format!("\"{}… (5000 characters)", "x".repeat(119)), arg(&text).to_string());
  assert_eq!(format!("{}… (121 characters)", "z".repeat(120)), arg(&"z".repeat(121)).to_string());
  let big = context((0..1000).map(|i| (format!("entry{i}"), value_number!(1))));
  let quoted = arg(&big).to_string();
  assert!(quoted.ends_with("… (1000 entries)"), "{quoted}");
  assert_eq!(120 + "… (1000 entries)".chars().count(), quoted.chars().count());
  assert!(arg(&Value::from(big)).to_string().ends_with("… (1000 entries)"));
  let one = context([("a".to_string(), Value::String("x".repeat(500).into()))]);
  assert!(arg(&one).to_string().ends_with("… (1 entry)"));
  // values without a size end with the cut alone
  let null = Value::Null(Some("r".repeat(500)));
  assert_eq!(format!("null({}…", "r".repeat(115)), arg(&null).to_string());
}

#[test]
fn a_long_number_is_quoted_in_scientific_notation() {
  assert_eq!("1E+50", arg(&number("1e50")).to_string());
  assert_eq!("1E+50", arg(&Value::Number(number("1e50"))).to_string());
  assert_eq!("9E+6144", arg(&number("9e6144")).to_string());
  assert_eq!("-1.5E-40", arg(&number("-1.5e-40")).to_string());
  // 40 characters of plain text stay plain, 41 do not
  assert_eq!(format!("1{}", "0".repeat(39)), arg(&number("1e39")).to_string());
  assert_eq!("1E+40", arg(&number("1e40")).to_string());
  assert_eq!("1.25E+40", arg(&number("1.25e40")).to_string());
  assert_eq!("0", arg(&number("0")).to_string());
}

#[test]
fn a_message_holds_at_most_1000_characters() {
  let text = "m".repeat(5000);
  let message = compose(format_args!("{text}"));
  assert_eq!(1000, message.chars().count());
  assert_eq!(format!("{}…", "m".repeat(999)), message);
  assert_eq!("m".repeat(1000), compose(format_args!("{}", "m".repeat(1000))));
  assert_eq!("fixed text", compose(format_args!("fixed text")));
  let value = value_null!("equal err '{}' =?= '{}'", numbers(10_000), 1);
  let Value::Null(Some(reason)) = value else { panic!("a null with a reason") };
  assert_eq!(1000, reason.chars().count());
  assert!(reason.starts_with("equal err '[1, 2, 3, "), "{reason}");
  assert!(reason.ends_with('…'));
}

#[test]
fn a_cut_never_splits_a_character() {
  for filler in ["é", "😀", "a"] {
    for offset in 0..4 {
      let text = format!("{}{}", "a".repeat(offset), filler.repeat(2000));
      let message = compose(format_args!("{text}"));
      assert_eq!(1000, message.chars().count());
      assert!(message.ends_with('…'));
      assert_eq!(text.chars().take(999).collect::<String>(), message.trim_end_matches('…'));
      let preview = arg(&text).to_string();
      assert_eq!(text.chars().take(120).collect::<String>(), preview.split('…').next().unwrap());
    }
  }
}

fn function(parameters: Vec<FeelType>, result: FeelType) -> Value {
  Value::from(FunctionDefinition::new(
    parameters.into_iter().enumerate().map(|(i, t)| (Name::from(format!("p{i}")), t)).collect(),
    FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(1)))),
    false,
    Closure::default(),
    FeelContext::default(),
    result,
  ))
}

#[test]
fn the_type_of_a_value_is_quoted_from_the_value() {
  let a = || Value::String("a".into());
  let nested = context([("outer".to_string(), Value::from(context([("inner".to_string(), numbers(2))])))]);
  let wide = context((0..10_000).map(|i| (format!("entry{i}"), value_number!(i))));
  let entry = |name: &str| format!("{name} {}", "entry".repeat(8));
  let three = || Value::from(context([(entry("x"), value_number!(1)), (entry("y"), a()), (entry("z"), Value::Null(None))]));
  let one = Box::new(value_number!(1));
  let ten = Box::new(value_number!(10));
  let values = vec![
    value_number!(1),
    a(),
    Value::Boolean(true),
    Value::Null(None),
    Value::List(vec![].into()),
    Value::List(vec![value_number!(1), a()].into()),
    Value::List(vec![Value::Null(None), value_number!(1)].into()),
    Value::List(vec![value_number!(1), Value::Null(None)].into()),
    Value::List(vec![numbers(1)].into()),
    Value::from(context([("a".to_string(), numbers(2))])),
    Value::from(nested),
    function(vec![FeelType::Number, FeelType::List(Box::new(FeelType::String))], FeelType::Boolean),
    Value::Range(one, IntervalType::Closed, ten, IntervalType::Closed),
    Value::FeelType(FeelType::Context([(Name::from("t"), FeelType::Date)].into())),
    Value::from(wide),
    Value::List((0..1000).map(|_| three()).collect()),
  ];
  for value in &values {
    assert_eq!(arg(&value.type_of()).to_string(), arg(&TypeOf(value)).to_string(), "the type of {value}");
  }
  assert_eq!("list<Any>", arg(&TypeOf(&values[5])).to_string());
  assert_eq!("function<number, list<string>>->boolean", arg(&TypeOf(&values[11])).to_string());
  assert!(arg(&TypeOf(&values[14])).to_string().ends_with("… (10000 entries)"));
  // a list type has no size: the preview ends with the cut
  let list_of_contexts = arg(&TypeOf(&values[15])).to_string();
  let full = values[15].type_of().to_string();
  assert!(full.starts_with(&format!("list<context<{}: number, ", entry("x"))), "{full}");
  assert_eq!(format!("{}…", full.chars().take(120).collect::<String>()), list_of_contexts);
}

/// The type of a list that holds the same list on 2^59 paths is written at once, in a message
/// and by [Value::type_of]: the type of the first item is built, and the other items are related
/// to it without building theirs.
#[test]
fn the_type_of_a_shared_list_is_written_at_once() {
  let list = doubled(60);
  let expected = nested_list_type(60, FeelType::Number);
  assert_eq!(expected, list.type_of());
  assert_eq!(arg(&expected).to_string(), arg(&TypeOf(&list)).to_string());
  // the items of a list after the first are not typed, so a large type among them is no cut
  let huge = Value::from(context((0..2_000).map(|i| (format!("entry{i}"), value_number!(i)))));
  let list = Value::List(vec![value_number!(1), huge].into());
  assert_eq!("list<Any>", arg(&TypeOf(&list)).to_string());
  assert_eq!("list<Any>", list.type_of().to_string());
}

#[test]
fn the_type_of_a_list_whose_first_item_has_a_huge_type_is_not_built() {
  let huge = Value::from(context((0..2_000).map(|i| (format!("entry{i}"), value_number!(i)))));
  let list = Value::List(vec![huge.clone(), huge.clone()].into());
  assert_eq!("list<…>", arg(&TypeOf(&list)).to_string());
  assert!(list.type_of().to_string().starts_with("list<context<entry0: number, "));
  // the items are not alike, so the first item's type is not needed
  let list = Value::List(vec![huge, value_number!(1)].into());
  assert_eq!("list<Any>", arg(&TypeOf(&list)).to_string());
  assert_eq!("list<Any>", list.type_of().to_string());
}
