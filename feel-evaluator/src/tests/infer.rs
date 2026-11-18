//! The type of an expression, found from its syntax tree.

use super::te_scope;
use crate::{infer_type, members_of};
use rheon_feel::{FeelType, Name};

/// The text of the type of `text` in the context `context`.
fn ty(context: &str, text: &str) -> String {
  let scope = te_scope(context);
  let node = rheon_feel_parser::parse_expression(&scope, text, false).unwrap_or_else(|e| panic!("{text}: {e}"));
  infer_type(&node, &scope, &[]).to_string()
}

const INPUT: &str = r#"{
  n: 1, s: "a", b: true, d: @"2024-01-02", t: @"10:00:00", dt: @"2024-01-02T10:00:00", ym: @"P1Y", dd: @"P1D",
  order: {id: 7, total: 9.5, customer: {name: "Ann", since: @"2020-01-01"}, lines: [{price: 2, unit count: 3}, {price: 4, unit count: 1}]},
  orders: [{id: 1, lines: [{price: 2}]}, {id: 2, lines: []}],
  numbers: [1, 2, 3], nothing: null
}"#;

#[test]
fn literals_and_names_have_the_type_of_their_value() {
  for (text, expected) in [
    ("1", "number"),
    (r#""a""#, "string"),
    ("true", "boolean"),
    ("null", "Null"),
    (r#"@"2024-01-02""#, "date"),
    (r#"@"10:00:00""#, "time"),
    (r#"@"2024-01-02T10:00:00""#, "date and time"),
    (r#"@"P1Y2M""#, "years and months duration"),
    (r#"@"PT1H""#, "days and time duration"),
    (r#"@"-P1D""#, "days and time duration"),
    ("n", "number"),
    ("s", "string"),
    ("d", "date"),
    ("numbers", "list<number>"),
    ("nothing", "Any"),
    ("unknown name", "Any"),
    ("order.customer", "context<name: string, since: date>"),
    ("[1, 2]", "list<number>"),
    (r#"[1, "a"]"#, "list<Any>"),
    ("[1, null]", "list<number>"),
    ("[]", "list<Any>"),
    ("{a: 1, b: a + 1, c: [b]}", "context<a: number, b: number, c: list<number>>"),
    ("[1..5]", "range<number>"),
    ("(d..d]", "range<date>"),
    ("< 10", "range<number>"),
    ("function(x: number, y) x + 1", "function<number, Any>->number"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn paths_and_filters_follow_the_type_of_their_base() {
  for (text, expected) in [
    ("order.id", "number"),
    ("order.customer.name", "string"),
    ("order.customer.since.year", "number"),
    ("order.lines", "list<context<price: number, unit count: number>>"),
    ("order.lines.price", "list<number>"),
    ("order.lines[1]", "context<price: number, unit count: number>"),
    ("order.lines[1].price", "number"),
    ("order.lines[price > 2]", "list<context<price: number, unit count: number>>"),
    ("order.lines[price > 2].unit count", "list<number>"),
    ("orders.lines", "list<list<context<price: number>>>"),
    ("orders[1].lines[1].price", "number"),
    ("orders[id = 1][1].id", "number"),
    ("numbers[1]", "number"),
    ("numbers[item > 1]", "list<number>"),
    ("order.no such entry", "Any"),
    ("n.year", "Any"),
    ("d.year", "number"),
    ("dt.time offset", "days and time duration"),
    ("t.timezone", "string"),
    ("ym.years", "number"),
    ("dd.hours", "number"),
    ("[1..5].start", "number"),
    ("[1..5].start included", "boolean"),
    ("{a: {b: 1}}.a.b", "number"),
    ("{a: {b: 1}}.a", "context<b: number>"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn operators_have_the_type_of_their_result() {
  for (text, expected) in [
    ("n + 1", "number"),
    (r#"s + "b""#, "string"),
    ("n + s", "Any"),
    ("d + ym", "date"),
    ("dt - dd", "date and time"),
    ("d - d", "days and time duration"),
    ("t + dd", "time"),
    ("dd * 2", "days and time duration"),
    ("dd / dd", "number"),
    ("ym / 2", "years and months duration"),
    ("-n", "number"),
    ("-dd", "days and time duration"),
    ("n ** 2", "number"),
    ("n = 1", "boolean"),
    ("n < 1 and b", "boolean"),
    ("n in [1..5]", "boolean"),
    ("n between 1 and 2", "boolean"),
    ("n instance of number", "boolean"),
    ("some x in numbers satisfies x > 1", "boolean"),
    ("if b then 1 else 2", "number"),
    (r#"if b then 1 else "a""#, "Any"),
    ("if b then 1 else null", "number"),
    (
      "if b then order else null",
      "context<customer: context<name: string, since: date>, id: number, lines: list<context<price: number, unit count: number>>, total: number>",
    ),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn iterations_bind_their_variables_to_the_items() {
  for (text, expected) in [
    ("for x in numbers return x * 2", "list<number>"),
    ("for l in order.lines return l.price", "list<number>"),
    ("for l in order.lines return l", "list<context<price: number, unit count: number>>"),
    ("for o in orders, l in o.lines return l.price", "list<number>"),
    ("for i in 1..3 return i", "list<number>"),
    ("for x in numbers return {v: x}", "list<context<v: number>>"),
    ("for x in n return x", "list<number>"),
    ("for x in unknown name return x", "list<Any>"),
    ("for x in numbers return for y in order.lines return y.price + x", "list<list<number>>"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn invocations_have_the_type_of_the_result() {
  for (text, expected) in [
    ("sum(numbers)", "number"),
    ("count(numbers)", "number"),
    (r#"upper case("a")"#, "string"),
    (r#"contains("a", "b")"#, "boolean"),
    (r#"date("2024-01-02")"#, "date"),
    (r#"date and time("2024-01-02T10:00:00")"#, "date and time"),
    (r#"date("2024-01-02").year"#, "number"),
    (r#"split("a b", " ")"#, "list<string>"),
    ("now()", "date and time"),
    ("today().month", "number"),
    // the functions whose result has the type of an argument
    ("append(numbers, 4)", "list<number>"),
    ("reverse(order.lines)", "list<context<price: number, unit count: number>>"),
    ("sort(order.lines, function(a, b) a.price < b.price)[1].price", "number"),
    ("sublist(order.lines, 1, 1)[1].unit count", "number"),
    ("distinct values(numbers)", "list<number>"),
    ("max(numbers)", "number"),
    ("min(d, d)", "date"),
    ("abs(dd)", "days and time duration"),
    (r#"get value(order, "id")"#, "number"),
    (r#"get value(order, "customer").name"#, "string"),
    ("get value(order, s)", "Any"),
    ("flatten([[1]])", "list<Any>"),
    // functions of the text and of the scope
    ("(function(x: number) x + 1)(2)", "number"),
    ("{f: function() {a: 1}, r: f()}.r.a", "number"),
    ("{f: function() order, r: f().customer.name}.r", "string"),
    ("unknown function(1)", "Any"),
    // a name in scope hides the built-in function
    ("{sum: function(l) \"x\", r: sum(numbers)}.r", "string"),
    // named arguments
    (r#"substring(string: "abc", start position: 2)"#, "string"),
    ("append(list: numbers, item: 4)", "list<Any>"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn names_bound_around_the_expression_have_the_types_given() {
  let scope = te_scope(INPUT);
  let node = rheon_feel_parser::parse_expression(&scope, "x.price + n", false).unwrap();
  let line = FeelType::Context([(Name::from("price"), FeelType::Number)].into());
  assert_eq!("number", infer_type(&node, &scope, &[(Name::from("x"), line.clone())]).to_string());
  // the innermost binding of a name is the one read
  let bound = [(Name::from("x"), FeelType::String), (Name::from("x"), line), (Name::from("n"), FeelType::Number)];
  assert_eq!("number", infer_type(&node, &scope, &bound).to_string());
  assert_eq!("Any", infer_type(&node, &scope, &[]).to_string());
}

#[test]
fn the_members_of_a_type_are_its_entries_or_properties() {
  let names = |feel_type: &FeelType| {
    members_of(feel_type)
      .into_iter()
      .map(|(name, member_type)| format!("{name}: {member_type}"))
      .collect::<Vec<_>>()
  };
  let line = FeelType::Context([(Name::from("price"), FeelType::Number), (Name::from("unit count"), FeelType::Number)].into());
  assert_eq!(names(&line), ["price: number", "unit count: number"]);
  assert_eq!(names(&FeelType::List(Box::new(line))), ["price: list<number>", "unit count: list<number>"]);
  assert_eq!(names(&FeelType::Date), ["year: number", "month: number", "day: number", "weekday: number", "value: number"]);
  assert_eq!(
    names(&FeelType::Range(Box::new(FeelType::Date))),
    ["start: date", "end: date", "start included: boolean", "end included: boolean"]
  );
  assert!(names(&FeelType::Number).is_empty());
  assert!(names(&FeelType::Any).is_empty());
}

/// A tree deeper than the limit is typed `Any` below it, without using the stack for it.
#[test]
fn a_deep_tree_is_typed_within_the_limit() {
  let scope = te_scope("{}");
  let text = format!("{}1{}", "[".repeat(400), "]".repeat(400));
  let node = rheon_feel_parser::parse_expression(&scope, &text, false).unwrap();
  let text = infer_type(&node, &scope, &[]).to_string();
  assert!(text.starts_with("list<list<") && text.contains("Any"), "{}", &text[..40]);
}
