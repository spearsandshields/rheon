//! The type of an expression, found from its syntax tree.

use super::te_scope;
use crate::{infer_type, members_of};
use rheon_feel::{FeelType, Name};

/// The text of the type of `text` in the context `context`.
fn ty(context: &str, text: &str) -> String {
  let scope = te_scope(context);
  let node = rheon_feel_parser::parse_expression(&scope, text).unwrap_or_else(|e| panic!("{text}: {e}"));
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
  let node = rheon_feel_parser::parse_expression(&scope, "x.price + n").unwrap();
  let line = FeelType::Context([(Name::from("price"), FeelType::Number)].into());
  assert_eq!("number", infer_type(&node, &scope, &[(Name::from("x"), line.clone())]).to_string());
  // the innermost binding of a name is the one read
  let bound = [(Name::from("x"), FeelType::String), (Name::from("x"), line), (Name::from("n"), FeelType::Number)];
  assert_eq!("number", infer_type(&node, &scope, &bound).to_string());
  let price = rheon_feel_parser::parse_expression(&scope, "x.price").unwrap();
  assert_eq!("Any", infer_type(&price, &scope, &[]).to_string());
}

#[test]
fn an_operand_of_no_known_type_beside_one_of_a_known_type() {
  // FEEL adds and subtracts a number only with a number, a string only to a string, has a power
  // only of numbers, multiplies a duration only by a number and divides a number only by a
  // number (Tables 57 to 61): a result that is not null is of the one type
  for (text, expected) in [
    ("unknown + 1", "number"),
    ("1 - unknown", "number"),
    (r#"unknown + "!""#, "string"),
    ("unknown ** 2", "number"),
    ("unknown ** other", "number"),
    ("dd * unknown", "days and time duration"),
    ("unknown * ym", "years and months duration"),
    ("n / unknown", "number"),
    // more than one type stays open
    ("unknown * 2", "Any"),
    ("unknown / 2", "Any"),
    ("unknown + dd", "Any"),
    ("-unknown", "Any"),
    ("unknown + other", "Any"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn an_invocation_of_a_function_literal_types_its_body_with_the_arguments() {
  for (text, expected) in [
    // the parameter has the type of its argument
    ("{double: function(x) x * 2, r: double(3)}.r", "number"),
    ("{mk: function(a, b) {a: a, b: b}, r: mk(\"x\", d)}.r", "context<a: string, b: date>"),
    // named arguments by their names
    ("{mk: function(a, b) {a: a, b: b}, r: mk(b: 1, a: true)}.r", "context<a: boolean, b: number>"),
    // a type written for the parameter is its type
    ("{f: function(a: string) a, r: f(1)}.r", "string"),
    // the literal itself, without an invocation
    ("{inc: function(x) x + 1}.inc", "function<Any>->number"),
    ("{f: function(a: number) a * 2}.f", "function<number>->number"),
    // the body sees the names where the literal is, not those of the invocation
    ("{f: function() k, g: function(k) f(), r: g(1)}.r", "Any"),
    // a literal that invokes itself is not typed again inside itself
    ("{fact: function(n) if n = 0 then 1 else n * fact(n - 1), r: fact(5)}.r", "Any"),
    ("{loop: function(n) loop(n), r: loop(1)}.r", "Any"),
    // a call whose arguments give null keeps the type of the body
    ("{inc: function(x) x + 1, r: inc(\"a\")}.r", "number"),
    // the body reads the entries of the literal it is written in, also those written after it
    // (DMN 1.6 clause 10.3.2.11.1), and not the name outside (`s` is a string there)
    ("{f: function() s, s: 1, r: f()}.r", "number"),
    // the entries that a function reads are evaluated, and typed, before it
    ("{f: function() s, s: 1}.f", "function<>->number"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

/// A name is typed as what evaluation reads for it (the analysis walks the tree as name
/// resolution leaves it): each of these is the type of the value the expression gives.
#[test]
fn names_are_typed_as_evaluation_reads_them() {
  for (text, expected) in [
    // an entry reads the entries it needs, wherever they are written
    ("{a: b, b: 1}.a", "number"),
    ("{a: b + 1, b: c * 2, c: 3}", "context<a: number, b: number, c: number>"),
    ("{a: x.y, x: {y: 1}}.a", "number"),
    ("{total: unit price * 2, unit price: 3}.total", "number"),
    // a function reads the entries of the literals it is written in, the nearest first
    ("{i: f(), f: function() a, a: 1}.i", "number"),
    ("{f: function() a, a: 2, r: {a: 1, s: f()}.s}.r", "number"),
    ("{x: 1, c: {f: function() x, x: \"a\", r: f()}.r}.c", "string"),
    ("for i in 1..2 return {f: function() k, k: i}.f()", "list<number>"),
    // a function that another one of the literal invokes
    ("{i: g(), g: function() f() + 1, f: function() a, a: 1}.i", "number"),
    // `partial` in a `for` is the list of its results so far, whatever else has the name
    ("for partial in [1, 2] return partial", "list<Any>"),
    ("{partial: 1, r: for i in [1, 2] return partial}.r", "list<Any>"),
    // an entry named as a built-in function is what the name reads there
    ("{r: abs(-1), abs: 1}.r", "Any"),
  ] {
    assert_eq!(expected, ty(INPUT, text), "{text}");
  }
}

#[test]
fn invocations_of_literals_that_invoke_each_other_are_typed_within_the_limit() {
  // f0 invokes f1 twice, f1 invokes f2 twice, ...: typing every body at every invocation would
  // type 2^40 bodies; the inference ends after its limit of nodes
  let mut entries: Vec<String> = (0..40).map(|i| format!("f{i}: function(x) f{}(x) + f{}(x)", i + 1, i + 1)).collect();
  entries.push("f40: function(x) x".to_string());
  entries.push("r: f0(1)".to_string());
  let text = format!("{{{}}}.r", entries.join(", "));
  let started = std::time::Instant::now();
  // beyond the limit a literal has the type of its body with the parameters as written
  assert_eq!("Any", ty("{}", &text));
  assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
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
  let node = rheon_feel_parser::parse_expression(&scope, &text).unwrap();
  let text = infer_type(&node, &scope, &[]).to_string();
  assert!(text.starts_with("list<list<") && text.contains("Any"), "{}", text.chars().take(120).collect::<String>());
}

/// The work of the typing grows with the text, also where a type holds one type at many places
/// or types hold each other entry after entry (review of the analysis).
#[test]
fn types_that_share_or_nest_much_are_typed_within_bounds() {
  let scope = te_scope("{}");
  let typed = |text: &str| {
    let node = rheon_feel_parser::parse_expression(&scope, text).unwrap_or_else(|e| panic!("{e}"));
    let started = std::time::Instant::now();
    let text = infer_type(&node, &scope, &[]).to_string();
    assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
    text
  };
  // a context whose two entries are the context before, 40 times: one type for each in the
  // table, 2^40 types written out as a tree; what is written is cut
  let mut entries = vec!["a0: {x: 1}".to_string()];
  entries.extend((1..=40).map(|i| format!("a{i}: {{l: a{}, r: a{}}}", i - 1, i - 1)));
  let doubled = entries.join(", ");
  let text = typed(&format!("{{{doubled}, f: function() a40}}.f"));
  assert!(
    text.starts_with("function<>->context<l: context<") && text.contains("Any"),
    "{}",
    text.chars().take(120).collect::<String>()
  );
  // functions that each give the function before: a type as deep as the literal is long
  let chain: Vec<String> = (1..=3000).map(|i| format!("f{i}: function() f{}", i - 1)).collect();
  let text = typed(&format!("{{f0: function() 0, {}}}.f3000", chain.join(", ")));
  assert!(
    text.starts_with("function<>->function<>->") && text.contains("Any"),
    "{}",
    text.chars().take(120).collect::<String>()
  );
  // many names read through many functions written in each other
  let names: Vec<String> = (1..=4000).map(|i| format!("n{i}")).collect();
  let text = typed(&format!("{}[{}]", "function() ".repeat(140), names.join(", ")));
  assert!(text.starts_with("function<>->function<>->"), "{}", text.chars().take(120).collect::<String>());
}

/// A value that the scope holds at many places is read once for its type.
#[test]
fn a_value_shared_in_the_scope_is_read_once() {
  let scope = te_scope("{x: (for i in 1..40 return if i = 1 then {leaf: 1} else {a: partial[-1], b: partial[-1]})[-1]}");
  let node = rheon_feel_parser::parse_expression(&scope, "x.a.b.a").unwrap();
  let started = std::time::Instant::now();
  let text = infer_type(&node, &scope, &[]).to_string();
  assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
  assert!(text.starts_with("context<a: context<"), "{}", text.chars().take(120).collect::<String>());
}

/// A type of another typing is no type of this one: it reads as not known, and nothing fails.
#[test]
fn a_type_of_another_typing_is_not_known() {
  let scope = te_scope("{}");
  let mut one = crate::Typing::new(&scope);
  let list = one.of(&FeelType::List(Box::new(FeelType::List(Box::new(FeelType::Number)))));
  let other = crate::Typing::new(&scope);
  assert_eq!(None, other.shown(list));
  assert_eq!(None, other.item(list));
  assert_eq!(FeelType::Any, other.feel_type(list));
  assert_eq!(Some("list<list<number>>".to_string()), one.shown(list));
}
