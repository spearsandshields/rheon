//! The lowering of each construct: the flat evaluator gives the value and the diagnostics that
//! the evaluator of closures gives.

use super::{agree, agree_in, agree_node, refused, scope};
use crate::Dialect;
use rheon_feel::Name;
use rheon_feel_parser::AstNode;

/// [agree_in] FEEL with the input context `input`.
#[track_caller]
fn agree_with(input: &str, text: &str) {
  agree_in(Dialect::Feel, input, text);
}

#[test]
fn literals() {
  for text in [
    "1",
    "1.50",
    "1e3",
    "\"text\"",
    "true",
    "false",
    "null",
    "@\"2024-01-31\"",
    "@\"P1Y\"",
    "@\"10:00:00.1234567891\"",
    "@\"x\"",
  ] {
    agree(text);
  }
}

#[test]
fn a_number_without_a_value_is_an_error_at_every_evaluation() {
  use crate::Dialect;
  use crate::flat::lower::lower;
  use crate::flat::resolve::resolve;
  use rheon_feel::FeelScope;
  use rheon_feel_parser::AstNode;
  let node = AstNode::Numeric("1x".into(), "".into(), '+', "".into());
  let old = crate::builders::build_evaluator_with_dialect(&node, Dialect::Feel);
  let program = lower(resolve(&node), Dialect::Feel).unwrap_or_else(|_| panic!("refused"));
  let scope = FeelScope::default();
  let flat: rheon_feel::Evaluator = Box::new(move |scope: &FeelScope| program.run(scope));
  assert_eq!(super::outcome(old, &scope), super::outcome(flat, &scope));
}

#[test]
fn constructs_not_lowered_yet_are_refused() {
  for text in ["{a: function(x) x}", "for i in [1] return [1][i]", "[1][1]", "function(x) x"] {
    refused(text);
  }
}

#[test]
fn names_read_the_input_then_built_ins() {
  agree("x");
  agree("abs");
  agree("not");
  agree_with("{x: 1}", "x");
  agree_with("{abs: 1}", "abs");
  agree_with("{?: 5}", "?");
  agree_with("{item: 5, partial: 6}", "partial");
}

#[test]
fn qualified_names_search_the_input() {
  let segments = |names: &[&str]| AstNode::QualifiedName(names.iter().map(|name| AstNode::QualifiedNameSegment(Name::from(*name))).collect());
  for input in ["", "{a: 1}", "{a: {b: 2}}", "{a: {c: 2}}", "{abs: 1}"] {
    let scope = scope(input);
    for names in [&["a"][..], &["a", "b"], &["a", "b", "c"], &["abs"], &[]] {
      agree_node(Dialect::Feel, &scope, &segments(names));
    }
  }
}

#[test]
fn paths_and_descendants() {
  agree_with("{x: {y: 1}}", "x.y");
  agree_with("{x: {y: 1}}", "x.z");
  agree_with("{xs: [{a: 1}, {b: 2}, 3]}", "xs.a");
  agree_with("{d: @\"2024-02-29\"}", "d.year");
  agree_with("{d: @\"2024-02-29\"}", "d.hours");
  agree_with("{s: \"text\"}", "s.length");
  agree_with("{x: {a: {b: {a: 2}}, c: [{a: 3}]}}", "x...a");
  agree_with("{x: 1}", "x...a");
  agree("(x).y");
  agree("@\"P1DT2H\".hours");
  // a path to a node that is no name is an error at every evaluation, and its base is not evaluated
  let scope = scope("");
  let node = AstNode::Path(Box::new(AstNode::Name(Name::from("x"))), Box::new(AstNode::Null));
  agree_node(Dialect::Feel, &scope, &node);
  let node = AstNode::Descendant(Box::new(AstNode::Name(Name::from("x"))), Box::new(AstNode::Null));
  agree_node(Dialect::Feel, &scope, &node);
}

/// Operands of every kind: numbers, strings, booleans, temporal values, null and a name that no
/// scope holds (an error).
const OPERANDS: [&str; 12] = [
  "1",
  "2.5",
  "0",
  "null",
  "\"a\"",
  "true",
  "@\"2024-01-31\"",
  "@\"P1D\"",
  "@\"P1Y\"",
  "@\"10:00:00\"",
  "@\"2024-01-31T10:00:00\"",
  "x",
];

#[test]
fn arithmetic_and_comparisons_feel_and_bfeel() {
  for operator in ["+", "-", "*", "/", "**", "=", "!=", "<", "<=", ">", ">="] {
    for a in OPERANDS {
      for b in OPERANDS {
        let text = format!("({a}) {operator} ({b})");
        agree_in(Dialect::Feel, "", &text);
        agree_in(Dialect::BFeel, "", &text);
      }
    }
  }
}

#[test]
fn negation() {
  for a in OPERANDS {
    agree_in(Dialect::Feel, "", &format!("-({a})"));
    agree_in(Dialect::BFeel, "", &format!("-({a})"));
  }
  agree("-(-1)");
  agree("1e6000 * 1e6000");
}

#[test]
fn between_evaluates_all_three_operands() {
  for a in ["1", "\"b\"", "null", "x", "@\"2024-01-31\""] {
    for low in ["0", "\"a\"", "null", "y"] {
      for high in ["2", "\"c\"", "null", "z"] {
        let text = format!("({a}) between ({low}) and ({high})");
        agree_in(Dialect::Feel, "", &text);
        agree_in(Dialect::BFeel, "", &text);
      }
    }
  }
}

#[test]
fn operators_nest() {
  agree_with("{a: 2, b: 3}", "(a + b) * (a - b) / b ** a");
  agree_with("{a: 2, b: 3}", "a + b = 5");
  agree("1 + 2 * 3 - 4 / 5 ** 6 < 7");
}

const LOGIC: [&str; 6] = ["true", "false", "null", "\"a\"", "1", "x"];

#[test]
fn conjunction_and_disjunction_evaluate_the_right_operand_when_it_decides() {
  for operator in ["and", "or"] {
    for a in LOGIC {
      for b in LOGIC {
        // `x` and `y` record their errors only when they are evaluated
        let b = if b == "x" { "y" } else { b };
        let text = format!("({a}) {operator} ({b})");
        agree_in(Dialect::Feel, "", &text);
        agree_in(Dialect::BFeel, "", &text);
      }
    }
  }
  agree("true and true and x or false");
}

#[test]
fn if_takes_the_then_branch_only_when_the_condition_is_true() {
  for condition in LOGIC {
    agree_in(Dialect::Feel, "", &format!("if ({condition}) then (y) else (z)"));
    agree_in(Dialect::BFeel, "", &format!("if ({condition}) then (1 + y) else \"a\""));
  }
  agree("if true then if false then 1 else 2 else 3");
}

#[test]
fn lists_and_ranges() {
  for text in [
    "[]",
    "[1, x, \"a\", [2, [3, y]], null]",
    "[1..2]",
    "(1..2]",
    "[\"a\"..\"z\")",
    "](x)..(y)[",
    "[1..\"a\"]",
    "[@\"2024-01-01\"..@\"2024-12-31\"]",
    "[1..2] = [1..2]",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
}

#[test]
fn in_and_not_in_with_every_right_hand_form() {
  for a in ["1", "5", "null", "\"b\"", "x", "@\"2024-02-01\""] {
    for right in [
      "[1, 2]",
      "[1..10]",
      "(> 3)",
      "(1, > 3, [4..5], null, x)",
      "(\"a\"..\"c\")",
      "(= 5)",
      "(!= 5)",
      "(<= 5, >= 9)",
    ] {
      let text = format!("({a}) in {right}");
      agree_in(Dialect::Feel, "", &text);
      agree_in(Dialect::BFeel, "", &text);
    }
  }
}

#[test]
fn not_in_evaluates_its_left_operand_twice() {
  for (input, left) in [("", "x"), ("{a: 1}", "a"), ("", "1")] {
    let scope = scope(input);
    let left = rheon_feel_parser::parse_expression(&scope, left, false).unwrap();
    let right = rheon_feel_parser::parse_unary_tests(&scope, "> 3, 1", false).unwrap();
    let node = AstNode::Out(Box::new(left), Box::new(right));
    agree_node(Dialect::Feel, &scope, &node);
    agree_node(Dialect::BFeel, &scope, &node);
  }
}

#[test]
fn unary_tests_read_the_input_value() {
  for input in ["{?: 5}", "{?: null}", "{?: \"b\"}", "{?: [1, 2]}", "{}"] {
    let scope = scope(input);
    for tests in ["> 3", "< 3, = 5", "not(5)", "-", "[1..10]", "? > 4", "\"a\", \"b\"", "null", "[1, 2]", "x"] {
      let tests = rheon_feel_parser::parse_unary_tests(&scope, tests, false).unwrap();
      let node = AstNode::In(Box::new(AstNode::Name(Name::from("?"))), Box::new(tests));
      agree_node(Dialect::Feel, &scope, &node);
      agree_node(Dialect::BFeel, &scope, &node);
    }
  }
}

#[test]
fn invocations_of_built_in_functions() {
  for text in [
    "abs(-1)",
    "abs(x)",
    "abs()",
    "abs(1, 2)",
    "abs(n: -1)",
    "abs(m: 1)",
    "string length(\"abc\")",
    "sum(1, 2, \"a\")",
    "sum([1, 2], 3)",
    "not(true)",
    "1(2)",
    "x(1)",
    "\"f\"(1)",
    "max([1, 2]) + min(3, 4)",
    "substring(\"abc\", 2)",
    "substring(string: \"abc\", start position: 2)",
    "date(\"2024-01-01\")",
    "matches(\"a\", \"[\")",
    "abs(abs(abs(-1)))",
    "before(1, [2..3])",
    "before(range: [2..3], point: 1)",
    "1 / 0 + abs(1e6000 * 10)",
  ] {
    agree_in(Dialect::Feel, "", text);
    agree_in(Dialect::BFeel, "", text);
  }
}

#[test]
fn invocations_count_against_the_call_depth_limit() {
  for depth in [199, 200, 201, 250] {
    let text = format!("{}1{}", "abs(".repeat(depth), ")".repeat(depth));
    agree(&text);
    // after the abort no further invocation is made and nothing it would evaluate is recorded
    agree(&format!("[{text}, abs(x), x]"));
  }
}

#[test]
fn invocations_of_functions_of_the_host() {
  let input = "{f: function(x) x + 1, g: function(a, b) a < b, h: function(n: number) n}";
  for text in ["f(2)", "f(x: 2)", "f()", "f(y: 1)", "sort([3, 1, 2], g)", "f(f(1))", "h(\"a\")", "h(n: 2)", "g(1, x)"] {
    agree_with(input, text);
  }
}

#[test]
fn malformed_invocations_are_errors_inside_the_invocation() {
  let scope = scope("");
  let abs = || Box::new(AstNode::Name(Name::from("abs")));
  let one = || Box::new(AstNode::Numeric("1".into(), "".into(), '+', "".into()));
  for arguments in [
    AstNode::Null,
    AstNode::NamedParameters(vec![AstNode::Null]),
    AstNode::NamedParameters(vec![AstNode::NamedParameter(Box::new(AstNode::Name(Name::from("n"))), one())]),
    AstNode::NamedParameters(vec![AstNode::NamedParameter(Box::new(AstNode::ParameterName(Name::from("n"))), one())]),
  ] {
    agree_node(Dialect::Feel, &scope, &AstNode::FunctionInvocation(abs(), Box::new(arguments)));
  }
}

#[test]
fn instance_of_and_types() {
  let values = ["1", "\"a\"", "true", "null", "@\"2024-01-31\"", "@\"P1D\"", "[1, 2]", "[[\"a\"]]", "[1..2]", "abs", "x"];
  let types = [
    "number",
    "string",
    "boolean",
    "date",
    "Any",
    "Null",
    "days and time duration",
    "list<number>",
    "list<list<string>>",
    "range<number>",
    "context<a: number, b: string>",
    "context<a: number, a: string>",
    "context<a: t, b: u>",
    "function<number> -> number",
    "function<number, string> -> boolean",
    "function<t> -> number",
    "list<t>",
    "range<t>",
    "t",
  ];
  for value in values {
    for feel_type in types {
      let text = format!("({value}) instance of {feel_type}");
      agree_in(Dialect::Feel, "", &text);
      agree_in(Dialect::BFeel, "", &text);
    }
  }
}

#[test]
fn malformed_types_are_the_builders_errors() {
  let scope = scope("");
  let key = |name: &str| Box::new(AstNode::ContextTypeEntryKey(Name::from(name)));
  let number = || Box::new(AstNode::FeelType(rheon_feel::FeelType::Number));
  for node in [
    AstNode::ContextType(vec![AstNode::Null, AstNode::Name(Name::from("x"))]),
    AstNode::ContextType(vec![AstNode::ContextTypeEntry(key("a"), number()), AstNode::Numeric("1".into(), "".into(), '+', "".into())]),
    AstNode::ContextType(vec![AstNode::ContextTypeEntry(Box::new(AstNode::Null), number())]),
    AstNode::ContextType(vec![]),
    AstNode::ListType(Box::new(AstNode::Null)),
    AstNode::FunctionType(Box::new(AstNode::Null), number()),
    AstNode::FunctionType(
      Box::new(AstNode::ParameterTypes(vec![AstNode::Null, AstNode::FeelType(rheon_feel::FeelType::String)])),
      number(),
    ),
    AstNode::FunctionType(Box::new(AstNode::ParameterTypes(vec![])), Box::new(AstNode::Null)),
  ] {
    agree_node(Dialect::Feel, &scope, &node);
    agree_node(Dialect::Feel, &scope, &AstNode::InstanceOf(Box::new(AstNode::Null), Box::new(node)));
  }
}

#[test]
fn context_literals_bind_their_entries_for_the_entries_after_them() {
  for text in [
    "{}",
    "{a: 1}",
    "{a: 1, b: a + 1}",
    "{a: 1, b: a + 1}.b",
    "{a: b, b: 1}",
    "{a: {b: 1}, c: a.b}",
    "{a: 1, b: {a: 2, c: a}, d: a}",
    "{a: 1, b: {c: a, a: 2, d: a}}",
    "{abs: 1, r: abs}",
    "{abs: -1, r: abs(-1)}",
    "{r: abs(-1), abs: 1}",
    "{a: 1 + \"x\", b: a, c: b + 1}",
    "{a: [1, 2], b: append(a, 3), c: a}",
    "{partial: 1, item: 2, r: partial + item}",
    "{\"a b\": 1, c: a b}",
    "[{a: 1}, {a: 2, b: a}]",
    "{a: 1}.a + {a: 2}.a",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
  agree_with("{x: 5}", "{y: x, x: 2, z: x}");
  agree_with("{x: 5, y: 6}", "{a: x, b: {x: a, c: y}}");
}

#[test]
fn a_repeated_key_ends_the_context_after_its_value() {
  for text in ["{a: 1, a: 2}", "{a: 1, a: x}", "{a: 1, b: 2, a: a + b, c: 3}", "{a: 1, a: 2}.a", "{b: {a: 1, a: 2}, c: b}"] {
    agree(text);
  }
}

#[test]
fn a_function_of_the_host_sees_the_entries_before_its_invocation() {
  // the body of `f` reads `a`, which only the scope of its invocation has
  let input = "{f: function(x) x + a, g: function(x) {r: x}}";
  for text in [
    "{a: 1, r: f(2)}.r",
    "{r: f(2), a: 1}.r",
    "{a: 1, b: {a: 10, r: f(2)}, c: f(3)}",
    "{a: 1, r: g(a)}",
    "{a: 1, a: f(1)}",
  ] {
    agree_with(input, text);
  }
}

#[test]
fn qualified_names_search_the_entries_of_a_context() {
  let segments = |names: &[&str]| AstNode::QualifiedName(names.iter().map(|name| AstNode::QualifiedNameSegment(Name::from(*name))).collect());
  let entry = |key: &str, value: AstNode| AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(Name::from(key))), Box::new(value));
  let nested = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text, false).unwrap();
  for input in ["", "{a: {b: 5}}", "{a: {c: 5}}"] {
    let scope = scope(input);
    for names in [&["a", "b"][..], &["a"], &["a", "c"], &["x", "b"], &[]] {
      let node = AstNode::Context(vec![entry("a", nested("{b: 1}")), entry("r", segments(names))]);
      agree_node(Dialect::Feel, &scope, &node);
      let node = AstNode::Context(vec![entry("r", segments(names)), entry("a", nested("{b: 1}"))]);
      agree_node(Dialect::Feel, &scope, &node);
      let node = AstNode::Context(vec![
        entry("a", nested("{b: 1}")),
        entry("s", AstNode::Context(vec![entry("a", nested("2")), entry("r", segments(names))])),
      ]);
      agree_node(Dialect::Feel, &scope, &node);
    }
  }
}

#[test]
fn a_context_with_an_entry_that_is_no_entry_is_refused() {
  let entry = |key: &str| AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(Name::from(key))), Box::new(AstNode::Null));
  let node = AstNode::Context(vec![entry("a"), AstNode::Null]);
  assert!(crate::flat::lower::lower(crate::flat::resolve::resolve(&node), Dialect::Feel).is_err());
}

#[test]
fn for_walks_every_combination_of_its_variables() {
  for text in [
    "for i in 1..3 return i",
    "for i in 3..1 return i",
    "for i in 1..1 return i",
    "for i in [1, 2, 3] return i * 2",
    "for i in [1..3] return i",
    "for i in [3..1] return i",
    "for i in (1..3] return i",
    "for i in [1..3) return i",
    "for i in 1 return i",
    "for i in null return i",
    "for i in [] return i",
    "for i in [null, 1] return i",
    "for d in @\"2024-02-27\"..@\"2024-03-01\" return d",
    "for d in [@\"2024-02-28\"..@\"2024-03-01\"] return d",
    "for i in \"a\"..3 return i",
    "for i in 1..\"b\" return i",
    "for i in @\"2024-01-01\"..3 return i",
    "for i in 1..2, j in [i, i + 1] return [i, j]",
    "for i in 1..3, j in 1..i return j",
    "for i in [], j in [1] return j",
    "for i in [1, 2], j in [] return j",
    "for i in [1, 2], j in \"x\"..1 return j",
    "for i in [1, 2], j in [i], k in [j, i] return i + j + k",
    "for i in 1..3 return partial",
    "for i in 1..4 return count(partial) + i",
    "for i in 1..3 return sum(partial) + i",
    "count(for i in 1..600 return partial)",
    "for i in [1], i in [2] return i",
    "for i in 1..3 return {x: i, y: x + i}",
    "{a: 1, r: for i in 1..3 return i + a}.r",
    "for c in [{a: 1}, {a: 2}] return c.a",
    "for i in 1..2 return for j in 1..i return [i, j, count(partial)]",
    "for i in 1..1000 return [x(i), 1 + null]",
    "for i in [1, \"a\", true] return i + 1",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
}

#[test]
fn a_variable_named_partial_is_hidden_in_later_domains() {
  for text in [
    "for partial in [1, 2] return partial",
    "for partial in [1, 2], y in [partial] return y",
    "for x in [1, 2], partial in [x] return partial",
    "for x in [1, 2], partial in [x], y in [partial] return [x, y]",
    "for partial in [abs], y in [partial(-1)] return y",
    "for i in 1..2, j in [partial] return j",
  ] {
    agree(text);
    agree_with("{partial: 7}", text);
  }
}

#[test]
fn some_and_every_end_at_a_value_that_decides() {
  for text in [
    "some x in [1, 2, 3] satisfies x > 2",
    "some x in [1, 2, 3] satisfies x > 5",
    "every x in [1, 2, 3] satisfies x > 0",
    "every x in [1, 2, 3] satisfies x > 1",
    "some x in [1, 2] satisfies x",
    "every x in [true, null, false] satisfies x",
    "some x in [false, \"a\", true] satisfies x",
    "every x in [] satisfies false",
    "some x in [] satisfies true",
    "some x in 5 satisfies x = 5",
    "some x in [1, 2], y in [x, 3] satisfies x + y = 4",
    "every x in [1, 2], y in [x] satisfies x = y",
    "some x in [1], x in [2] satisfies true",
    "some partial in [1, 2], y in [partial] satisfies y = 2",
    "every x in [1, 2] satisfies partial = null",
    "some x in [1..3] satisfies x = 2",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
}

#[test]
fn malformed_iterations_are_the_builders_errors() {
  let parse = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text, false).unwrap();
  let (some, for_expression) = (parse("some x in [1], x in [2] satisfies x > 0"), parse("for i in [1] return i"));
  let (AstNode::Some(contexts, satisfies), AstNode::For(iteration_contexts, body)) = (&some, &for_expression) else {
    panic!("not a some and a for");
  };
  let scope = scope("");
  for node in [
    AstNode::Some(Box::new(AstNode::Null), satisfies.clone()),
    AstNode::Every(contexts.clone(), Box::new(AstNode::Null)),
    AstNode::Every(Box::new(AstNode::QuantifiedContexts(vec![])), Box::new(AstNode::Null)),
    AstNode::Some(Box::new(AstNode::QuantifiedContexts(vec![])), satisfies.clone()),
    AstNode::For(Box::new(AstNode::Null), body.clone()),
    AstNode::For(Box::new(AstNode::IterationContexts(vec![AstNode::Null])), body.clone()),
    AstNode::For(iteration_contexts.clone(), Box::new(AstNode::Null)),
  ] {
    agree_node(Dialect::Feel, &scope, &node);
  }
}

#[test]
fn a_function_of_the_host_sees_the_variables_of_the_walk() {
  // the bodies read `i`, `j` and `partial`, which only the scope of the invocation has
  let input = "{f: function(x) x + i, g: function(x) count(partial), h: function(x) [x, partial, i]}";
  for text in [
    "for i in 1..3 return f(i)",
    "for i in 1..3 return g(i)",
    "for i in 1..2, j in h(i) return j",
    "for i in 1..2, j in [f(0)] return j",
    "some i in [1, 2] satisfies f(i) = 4",
    "{i: 10, r: for j in 1..2 return f(j)}.r",
    "for i in 1..2 return {j: i, r: h(j)}",
  ] {
    agree_with(input, text);
  }
}

#[test]
fn qualified_names_search_the_variables_of_a_walk() {
  let segments = |names: &[&str]| AstNode::QualifiedName(names.iter().map(|name| AstNode::QualifiedNameSegment(Name::from(*name))).collect());
  let parse = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text, false).unwrap();
  let for_expression = parse("for c in [{b: 1}, {b: 2, c: 3}, 4] return c");
  let AstNode::For(contexts, _) = &for_expression else {
    panic!("not a for");
  };
  for input in ["", "{c: {b: 5}}"] {
    let scope = scope(input);
    for names in [&["c", "b"][..], &["c"], &["c", "c"], &["partial"]] {
      agree_node(Dialect::Feel, &scope, &AstNode::For(contexts.clone(), Box::new(segments(names))));
    }
  }
}
