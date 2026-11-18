//! The lowering of each construct: the flat evaluator gives the value and the diagnostics
//! recorded for each case in `expected/constructs.txt`.

use super::{agree, agree_in, agree_node, scope};
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
  let node = AstNode::Numeric("1x".into(), "".into(), '+', "".into());
  agree_node(Dialect::Feel, &scope(""), &node);
}

#[test]
fn a_malformed_tree_is_null_with_one_error_at_every_evaluation() {
  let name = |name: &str| Box::new(AstNode::Name(Name::from(name)));
  let entry = AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(Name::from("a"))), Box::new(AstNode::Null));
  for (node, kind) in [
    // a context with an entry that is no entry
    (AstNode::Context(vec![entry, AstNode::Null]), "Context"),
    // a qualified name with a segment that is no segment
    (AstNode::QualifiedName(vec![AstNode::Name(Name::from("x"))]), "QualifiedName"),
    // a function definition whose parameters are no parameters
    (
      AstNode::FunctionDefinition(name("x"), Box::new(AstNode::FunctionBody(name("x"), false))),
      "FunctionDefinition",
    ),
    // the body of a function outside a definition
    (AstNode::FunctionBody(name("x"), false), "FunctionBody"),
    // a malformed node inside a well-formed one
    (AstNode::Add(Box::new(AstNode::Null), Box::new(AstNode::FunctionBody(name("x"), false))), "FunctionBody"),
  ] {
    let message = format!("malformed expression tree: unexpected {kind} node");
    let expected = (
      format!("Null(Some({message:?}))"),
      format!("Diagnostics {{ count: 1, messages: [{message:?}], warnings: [] }}"),
    );
    let evaluator = crate::evaluators::build(&node, Dialect::Feel);
    for _ in 0..2 {
      let (value, diagnostics) = rheon_feel::diagnostics::collect(|| evaluator(&scope("")));
      assert_eq!(expected, (format!("{value:?}"), format!("{diagnostics:?}")), "{node:?}");
    }
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
    let left = rheon_feel_parser::parse_expression(&scope, left).unwrap();
    let right = rheon_feel_parser::parse_unary_tests(&scope, "> 3, 1").unwrap();
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
      let tests = rheon_feel_parser::parse_unary_tests(&scope, tests).unwrap();
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
fn malformed_types_are_errors() {
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
  let nested = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text).unwrap();
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
fn malformed_iterations_are_errors() {
  let parse = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text).unwrap();
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
  let parse = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text).unwrap();
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

#[test]
fn filters_select_by_index_or_keep_the_items_of_their_predicate() {
  for text in [
    "[1, 2, 3][item > 1]",
    "[1, 2, 3][1]",
    "[1, 2, 3][-1]",
    "[1, 2, 3][0]",
    "[1, 2, 3][4]",
    "[1, 2, 3][-4]",
    "[1, 2, 3][1.5]",
    "[1, 2, 3][100000000000000000000]",
    "[][1]",
    "[][item > 1]",
    "[1, 2, 3][true]",
    "[1, 2, 3][false]",
    "[1, 2, 3][null]",
    "[1, 2, 3][\"x\"]",
    "[1, 2, 3][item]",
    "[1, \"a\", 3][item > 1]",
    "[1, \"a\", 3][if item = 1 then \"x\" else true]",
    "null[item > 1]",
    "null[1]",
    "5[item > 1]",
    "5[1]",
    "5[2]",
    "{a: 1}[a = 1]",
    "{a: 1}[item.a = 1]",
    "[{a: 1}, {a: 2}][a > 1]",
    "[{a: 1, item: 5}, {a: 2}][item > 1]",
    "[{item: 1}, {item: 2}][item = 2]",
    "[{a: {b: 1}}, {a: 2}][a.b = 1]",
    "[[1, 2], [3]][count(item[item > 1]) > 0]",
    "[{a: [1, 2]}][count(a[item > 1]) = 1]",
    "[{a: 1}, {a: 3}][[{b: 2}][b > a] = []]",
    "[[[1]]][1][1]",
    "{a: 2, r: [1, 2, 3][item > a]}.r",
    "{l: [1, 2], r: l[1]}.r",
    "{amount: 5, r: [{amount: 1}, {b: 2}][amount > 2]}.r",
    "for i in 1..3 return [1, 2, 3][item < i]",
    "for i in 1..4 return if i = 1 then 1 else partial[-1] + 1",
    "for item in [5] return [1, 2][item > 1]",
    "[1, 2, 3][abs(item) > 1]",
    "[{abs: 1}][abs = 1]",
    "[1, 2][x(item)]",
    "count([1, 2, 3][item > 1])",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
  for (input, text) in [
    ("{x: 2}", "[1, 2, 3][x]"),
    ("{x: 2}", "[{x: 1}, {x: 3}][x]"),
    ("{x: 2}", "[{x: 1}, {x: 3}][x > 2]"),
    ("{item: 2}", "[1, 2, 3][item]"),
    ("{l: [1, 2, 3]}", "l[item > 1][-1]"),
  ] {
    agree_with(input, text);
  }
}

#[test]
fn a_function_of_the_host_sees_the_item_of_a_filter() {
  // the bodies read `amount` and `item`, which only the scope of the invocation has
  let input = "{f: function(x) x + amount, g: function(x) item, h: function(x) item.amount}";
  for text in [
    "[{amount: 1}, {amount: 5}][f(0) > 2]",
    "[1, 2, 3][g(0) > 1]",
    "[1, 2, 3][g(0)]",
    "[{amount: 1}, {amount: 5, item: {amount: 9}}][h(0) > 2]",
    "for i in [1, 2] return [{amount: i}][f(i) > 2]",
  ] {
    agree_with(input, text);
  }
}

#[test]
fn qualified_names_search_the_item_of_a_filter() {
  let segments = |names: &[&str]| AstNode::QualifiedName(names.iter().map(|name| AstNode::QualifiedNameSegment(Name::from(*name))).collect());
  let parse = |text: &str| rheon_feel_parser::parse_expression(&scope(""), text).unwrap();
  for input in ["", "{a: {b: 5}}", "{item: {b: 6}}"] {
    let scope = scope(input);
    for list in ["[{a: {b: 1}}, {a: 2}, 3, {item: {b: 4}}]", "[{b: 1}]", "[]"] {
      for names in [&["item", "b"][..], &["item", "a", "b"], &["a", "b"], &["a"], &["item"]] {
        let node = AstNode::Filter(Box::new(parse(list)), Box::new(AstNode::Eq(Box::new(segments(names)), Box::new(parse("1")))));
        agree_node(Dialect::Feel, &scope, &node);
      }
    }
  }
}

#[test]
fn function_definitions_and_their_invocations() {
  for text in [
    "function(x) x",
    "function(x, y) x + y",
    "function(a, a) a",
    "{f: function(x) x + 1, r: f(2)}.r",
    "{y: 10, f: function(x) x + y, r: f(2)}",
    "{f: function(x) x + y, y: 10, r: f(2)}.r",
    "for i in [1, 2] return {f: function() i}.f()",
    "{f: function(n) if n = 0 then 0 else n + f(n - 1), r: f(10)}.r",
    "{f: function(x) function(y) x + y, g: f(1), r: g(2)}.r",
    "{f: function() function() z, z: 5, r: f()()}.r",
    "{f: function() function() z, r: f()(), z: 5}",
    "{f: function(a: number) a + 1, r: [f(1), f(\"a\")]}.r",
    "{f: function(a, b) a - b, r: f(b: 1, a: 5)}.r",
    "{f: function(a, a) a, r: [f(a: 1), f(1, 2)]}.r",
    "{f: function(a) a, r: f(b: 1)}.r",
    "{f: function(a) a, r: f(1, 2)}.r",
    "function(x: t) x",
    "{t: 1, f: function(x: t) x}",
    "{f: function(x: list<number>, y: context<a: string>) [x, y], r: f([1], {a: \"b\"})}.r",
    "sort([3, 1, 2], function(a, b) a < b)",
    "{f: function(a, b) x < 2, x: 1, r: sort([2, 1], f)}.r",
    "[1, 2, 3][{f: function() item > 1}.f()]",
    "{g: function() item, r: [1, 2][g() > 1]}.r",
    "{g: function() partial, r: for i in [1, 2] return g()}.r",
    "{g: function() k, r: for k in [1, 2] return g()}.r",
    "function(x) external {java: {class: \"a\", method signature: \"b\"}}",
    "{f: function(x) external {java: {class: \"a\"}}}",
    "function(x: t) external {pmml: {document: \"d\"}}",
    "{a: {b: 1}, f: function() a.b, r: f()}",
    "{f: function(x) x}.f instance of function<Any> -> Any",
    "{f: function(n) if n > 0 then f(n - 1) else 0, r: f(150)}.r",
    "{f: function(n) if n > 0 then f(n - 1) else 0, r: f(250)}.r",
    "{f: function(n) if n > 0 then [f(n - 1)] else 0, r: f(198)}.r",
    "{even: function(n) if n = 0 then true else odd(n - 1), odd: function(n) if n = 0 then false else even(n - 1), r: even(30)}.r",
    "{f: function(x) x + y, g: function(y) f(1), r: g(2)}.r",
    "{f: function(n) if n = 0 then [] else [count(f(n - 1))], r: f(5)}.r",
    "{f: function() g(), g: function() h, h: 3, r: f()}.r",
    "{f: function(l) l[item > k], k: 1, r: f([1, 2, 3])}.r",
    "{f: function(x) x * 2, r: for i in 1..3 return f(i) + count([i][f(item) > 2])}.r",
    "{f: function(x) x, r: not(f(1) in (f(2), f(1)))}.r",
    "{f: function(x) [x, x][item > 1], r: f(2)}.r",
    "{f: function(x) {a: x, b: a + 1}.b, r: f(2)}.r",
    "{f: function(x) for i in 1..x return i * x, r: f(3)}.r",
  ] {
    agree(text);
    agree_in(Dialect::BFeel, "", text);
  }
  agree_with("{y: 7, t: number}", "{f: function(x: t) x + y, r: f(1)}.r");
}

#[test]
fn a_body_searches_the_scopes_of_its_callers() {
  for text in [
    "{f: function() 2 instance of t, r: for t in [1] return f()}.r",
    "{f: function() 2 instance of a.b, r: for a in [{b: 1}] return f()}.r",
    "{f: function() 2 instance of a.b, r: [{a: {b: 1}}][f()]}.r",
    "{f: function() 2 instance of item, r: [1][f()]}.r",
  ] {
    agree(text);
  }
}

#[test]
fn a_caller_that_has_read_a_name_gives_its_value_below_its_own_scopes() {
  for text in [
    "{x: 1, g: function() x, f: function() [x, for x in [2] return g(), g()], r: f()}.r",
    "{item: 0, g: function() item, f: function() [item, [5][g() > 1], g()], r: f()}.r",
    "{x: 1, g: function() x, f: function(x) [x, g()], h: function() [x, f(2), g()], r: h()}.r",
    "{g: function() abs, f: function() [abs, g()], r: f()}.r",
    "{g: function() nothing, f: function() [nothing, g()], r: f()}.r",
    "{fib: function(n) if n < 2 then n else fib(n - 1) + fib(n - 2), r: fib(12)}.r",
  ] {
    agree(text);
  }
}
