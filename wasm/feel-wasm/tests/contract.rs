//! Contract tests (docs/CONTRACT.md §2–3). Run natively (`cargo test -p rheon-feel-wasm`) and under
//! wasm32-wasip1 via wasmtime (`source scripts/env.sh && cargo test -p rheon-feel-wasm --target wasm32-wasip1`).
//! The request corpora that must never trap the module are in `corpus/`.

use serde_json::{Value as Json, json};

mod corpus;

#[path = "shared/big_stack.rs"]
mod big_stack;

use big_stack::with_big_stack;

/// Sends a raw request text and parses the response.
fn raw(request: &str) -> Json {
  let response = feel_wasm::call(request);
  serde_json::from_str(&response).unwrap_or_else(|e| panic!("response is not JSON ({e}): {response}"))
}

fn req(request: Json) -> Json {
  raw(&request.to_string())
}

/// Evaluates an expression and returns the `result` object (asserting success).
fn eval_ok(request: Json) -> Json {
  let response = req(request);
  assert_eq!(response["ok"], true, "expected success, got {response}");
  assert!(response["elapsedMicros"].is_u64(), "{response}");
  assert!(response["warnings"].is_array(), "{response}");
  response["result"].clone()
}

fn expr(expression: &str) -> Json {
  eval_ok(json!({"op": "evaluate", "expression": expression}))
}

fn display(expression: &str) -> String {
  expr(expression)["display"].as_str().unwrap().to_string()
}

fn unary(tests: &str, input: Json) -> Json {
  eval_ok(json!({"op": "evaluate", "dialect": "unaryTests", "expression": tests, "input": input}))
}

/// The kind and the message of a failed response.
fn error_of(response: &Json) -> (&str, &str) {
  assert_eq!(response["ok"], false, "expected failure, got {response}");
  let message = response["error"]["message"].as_str().unwrap_or_else(|| panic!("no error message: {response}"));
  (response["error"]["kind"].as_str().unwrap(), message)
}

fn error_kind(response: &Json) -> &str {
  error_of(response).0
}

/// Asserts that a response is an error of `kind` with exactly `message`.
#[track_caller]
fn assert_error(response: &Json, kind: &str, message: &str) {
  assert_eq!(error_of(response), (kind, message), "{response}");
}

/// The messages of a successful response's warnings.
fn warnings(response: &Json) -> Vec<&str> {
  assert_eq!(response["ok"], true, "expected success, got {response}");
  response["warnings"].as_array().unwrap().iter().map(|w| w["message"].as_str().unwrap()).collect()
}

// ---------------------------------------------------------------- numbers

#[test]
fn number_fidelity_addition() {
  let r = expr("0.1 + 0.2");
  assert_eq!(r["display"], "0.3");
  assert_eq!(r["feel"], "0.3");
  assert_eq!(r["type"], "number");
  assert_eq!(r["isNull"], false);
  assert!(r.get("nullReason").is_none());
  assert_eq!(
    r["typed"],
    json!({"simple": {"type": "xsd:decimal", "text": "0.3", "isNil": false}, "components": null, "list": null})
  );
}

#[test]
fn number_fidelity_division() {
  assert_eq!(display("1/3"), format!("0.{}", "3".repeat(34)));
  assert_eq!(display("2/3"), "0.6666666666666666666666666666666667");
}

#[test]
fn number_fidelity_exp_and_decimal() {
  assert_eq!(display("exp(1)"), "2.718281828459045235360287471352662");
  assert_eq!(display("decimal(-2.5, 0)"), "-2");
  assert_eq!(display("decimal(2.5, 0)"), "2");
  assert_eq!(display("decimal(1/3, 2)"), "0.33");
}

#[test]
fn number_fidelity_large_values() {
  assert_eq!(display("12345678901234567890 * 10"), "123456789012345678900");
  assert_eq!(display("9007199254740993 + 0"), "9007199254740993"); // not representable as f64
}

#[test]
fn json_numbers_keep_exact_text() {
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","expression":"x + y","context":{"x":0.1,"y":0.2}}"#).unwrap());
  assert_eq!(r["display"], "0.3");
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","expression":"big + 1","context":{"big":9007199254740993123456789}}"#).unwrap());
  assert_eq!(r["display"], "9007199254740993123456790");
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","expression":"x","context":{"x":1.5e3}}"#).unwrap());
  assert_eq!(r["display"], "1500");
}

#[test]
fn json_numbers_over_34_digits_warn() {
  let response = raw(r#"{"op":"evaluate","expression":"x","context":{"x":1.23456789012345678901234567890123456789}}"#);
  assert_eq!(response["ok"], true, "{response}");
  let warnings = response["warnings"].as_array().unwrap();
  assert_eq!(warnings.len(), 1, "{response}");
  assert!(warnings[0]["message"].as_str().unwrap().contains("x: number"), "{response}");
}

// ---------------------------------------------------------------- contexts

#[test]
fn json_context_with_spaced_names() {
  let r = eval_ok(json!({"op": "evaluate", "expression": "Grace's age + 1", "context": {"Grace's age": 40}}));
  assert_eq!(r["display"], "41");
  let r = eval_ok(json!({"op": "evaluate", "expression": "first name + \" \" + last name", "context": {"first name": "Ada", "last name": "Lovelace"}}));
  assert_eq!(r["display"], "\"Ada Lovelace\"");
  assert_eq!(r["type"], "string");
}

#[test]
fn json_context_nested_values() {
  let r = eval_ok(json!({
    "op": "evaluate",
    "expression": "sum(order.items[price > 10].price)",
    "context": {"order": {"items": [{"price": 5}, {"price": 20}, {"price": 30.5}]}}
  }));
  assert_eq!(r["display"], "50.5");
  let r = eval_ok(json!({"op": "evaluate", "expression": "flag and missing = null", "context": {"flag": true, "missing": null}}));
  assert_eq!(r["display"], "true");
}

#[test]
fn feel_context_literal() {
  let r = eval_ok(json!({"op": "evaluate", "expression": "b * 2", "contextFormat": "feel", "context": "{a: 1, b: a + 1}"}));
  assert_eq!(r["display"], "4");
  let r = eval_ok(json!({"op": "evaluate", "expression": "Grace's age", "contextFormat": "feel", "context": "{\"Grace's age\": 1/3}"}));
  assert_eq!(r["display"], format!("0.{}", "3".repeat(34)));
  let r = eval_ok(json!({"op": "evaluate", "expression": "d + duration(\"P1D\")", "contextFormat": "feel", "context": "{d: date(\"2026-10-16\")}"}));
  assert_eq!(r["display"], "\"2026-10-17\"");
}

#[test]
fn bad_contexts() {
  let feel = |context: Json| req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": context}));
  assert_error(&feel(json!("{a: }")), "context", "invalid FEEL context: <ParserError> syntax error: {a: }");
  assert_error(&feel(json!("[1,2]")), "context", "FEEL context literal expected, it evaluated to [1, 2] (list<number>)");
  assert_error(&feel(json!("{a: 1, a: 2}")), "context", "FEEL context evaluated to null: duplicated context entry key: a");
  let feel_text_expected = "with contextFormat \"feel\", 'context' must be a string holding a FEEL context literal";
  assert_error(&feel(json!({"a": 1})), "context", feel_text_expected);
  let json_object_expected = "with contextFormat \"json\", 'context' must be a JSON object";
  assert_error(&req(json!({"op": "evaluate", "expression": "1", "context": [1, 2]})), "context", json_object_expected);
  assert_error(&req(json!({"op": "evaluate", "expression": "1", "context": "{a: 1}"})), "context", json_object_expected);
  let explicit_json = json!({"op": "evaluate", "expression": "1", "contextFormat": "json", "context": 1});
  assert_error(&req(explicit_json), "context", json_object_expected);
  // empty / missing contexts are fine
  assert_eq!(
    eval_ok(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": "  "}))["display"],
    "1"
  );
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "1", "context": null}))["display"], "1");
}

// ---------------------------------------------------------------- structured output

#[test]
fn list_output() {
  let r = expr("[1, 2/4, \"x\", true, null]");
  assert_eq!(r["display"], "[1, 0.5, \"x\", true, null]");
  assert_eq!(r["type"], "list<Any>");
  let r = expr("for i in 1..3 return i * i");
  assert_eq!(r["display"], "[1, 4, 9]");
  assert_eq!(r["type"], "list<number>");
  assert_eq!(r["typed"]["list"]["items"][2]["simple"]["text"], "9");
  assert_eq!(expr("[]")["display"], "[]");
}

#[test]
fn context_output() {
  let r = expr("{a: 1/3, \"b c\": [1, 2], d: {e: \"f\"}}");
  assert_eq!(r["display"], "{\"a\": 0.3333333333333333333333333333333333, \"b c\": [1, 2], \"d\": {\"e\": \"f\"}}");
  assert_eq!(r["type"], "context<a: number, b c: list<number>, d: context<e: string>>");
  let components = r["typed"]["components"].as_array().unwrap();
  assert_eq!(components.len(), 3);
  assert_eq!(components[0]["name"], "a");
  assert_eq!(expr("{}")["display"], "{}");
}

#[test]
fn long_output_is_pretty_printed() {
  let d = display("{alpha: \"aaaaaaaaaaaaaaaaaaaa\", beta: \"bbbbbbbbbbbbbbbbbbbb\", gamma: [1/3, 2/3]}");
  assert_eq!(
    d,
    "{\n  \"alpha\": \"aaaaaaaaaaaaaaaaaaaa\",\n  \"beta\": \"bbbbbbbbbbbbbbbbbbbb\",\n  \"gamma\": [0.3333333333333333333333333333333333, 0.6666666666666666666666666666666667]\n}"
  );
}

#[test]
fn string_escaping() {
  let r = expr("\"say \\\"hi\\\"\"");
  assert_eq!(r["display"], "\"say \\\"hi\\\"\"");
  assert_eq!(r["type"], "string");
}

// ---------------------------------------------------------------- temporal

#[test]
fn temporal_values() {
  let r = expr("date(\"2026-10-16\") + duration(\"P1M\")");
  assert_eq!(r["display"], "\"2026-11-16\"");
  assert_eq!(r["feel"], "2026-11-16");
  assert_eq!(r["type"], "date");
  assert_eq!(r["typed"]["simple"]["type"], "xsd:date");

  let r = expr("date and time(\"2026-10-16T10:30:00@Europe/Warsaw\")");
  assert_eq!(r["type"], "date and time");
  assert_eq!(r["feel"], "2026-10-16T10:30:00@Europe/Warsaw");

  let r = expr("time(\"10:30:00Z\") - time(\"08:15:30Z\")");
  assert_eq!(r["display"], "\"PT2H14M30S\"");
  assert_eq!(r["type"], "days and time duration");

  let r = expr("years and months duration(date(\"2020-01-01\"), date(\"2026-10-16\"))");
  assert_eq!(r["display"], "\"P6Y9M\"");
  assert_eq!(r["type"], "years and months duration");
}

#[test]
fn ranges_and_functions_display() {
  let r = expr("[1..10]");
  assert_eq!(r["display"], "\"[1..10]\"");
  assert_eq!(r["type"], "range<number>");
  let r = expr("function(x) x + 1");
  assert!(r["display"].as_str().unwrap().starts_with("\"function<"), "{r}");
  assert_eq!(expr("(function(x) x + 1)(41)")["display"], "42");
}

// ---------------------------------------------------------------- unary tests

#[test]
fn unary_tests_range() {
  assert_eq!(unary("[1..10]", json!(5))["display"], "true");
  assert_eq!(unary("[1..10]", json!(11))["display"], "false");
  assert_eq!(unary("[1..10]", json!(10))["display"], "true");
  assert_eq!(unary("(1..10)", json!(10))["display"], "false");
  assert_eq!(unary("[1..10]", json!(5))["type"], "boolean");
}

#[test]
fn unary_tests_disjunction() {
  assert_eq!(unary("> 5, < 2", json!(1))["display"], "true");
  assert_eq!(unary("> 5, < 2", json!(6))["display"], "true");
  assert_eq!(unary("> 5, < 2", json!(3))["display"], "false");
  assert_eq!(unary("not(> 5, < 2)", json!(3))["display"], "true");
}

#[test]
fn unary_tests_strings_and_misc() {
  assert_eq!(unary("\"a\",\"b\"", json!("b"))["display"], "true");
  assert_eq!(unary("\"a\",\"b\"", json!("c"))["display"], "false");
  assert_eq!(unary("-", json!("anything"))["display"], "true");
  // `?` is bound to the input and can be used in expressions...
  assert_eq!(unary("[? - 2]", json!(7))["display"], "false");
  assert_eq!(unary("(? * 0) + 7", json!(7))["display"], "true");
  // ...but dsntk (at the pinned commit) does NOT treat a boolean expression such as `? > 5`
  // as "satisfied when true" (DMN 1.2+ semantics): it compares the
  // input with the boolean result (`7 = true` is false). This documents current dsntk behaviour.
  assert_eq!(unary("? > 5 and ? < 10", json!(7))["display"], "false");
  assert_eq!(unary("null", json!(null))["display"], "true");
  // exact decimal comparison
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","dialect":"unaryTests","expression":"0.3","input":0.3,"context":{}}"#).unwrap());
  assert_eq!(r["display"], "true");
  assert_eq!(unary("0.1 + 0.2", json!(0.3))["display"], "true");
}

#[test]
fn unary_tests_input_from_context() {
  let r = eval_ok(json!({"op": "evaluate", "dialect": "unaryTests", "expression": "> limit", "context": {"?": 12, "limit": 10}}));
  assert_eq!(r["display"], "true");
  // `input` wins over context["?"]
  let r = eval_ok(json!({"op": "evaluate", "dialect": "unaryTests", "expression": "> limit", "input": 3, "context": {"?": 12, "limit": 10}}));
  assert_eq!(r["display"], "false");
  // no input at all: tests null and warns
  let response = req(json!({"op": "evaluate", "dialect": "unaryTests", "expression": "null"}));
  assert_eq!(response["result"]["display"], "true", "{response}");
  assert_eq!(warnings(&response), ["no input value given ('input' or context[\"?\"]); testing null"]);
}

// ---------------------------------------------------------------- errors and nulls

#[test]
fn syntax_errors() {
  let response = req(json!({"op": "evaluate", "expression": "1 +"}));
  assert_eq!(error_kind(&response), "syntax");
  let response = req(json!({"op": "evaluate", "dialect": "unaryTests", "expression": "[1..", "input": 1}));
  assert_eq!(error_kind(&response), "syntax");
  let response = req(json!({"op": "parse", "expression": "if then else"}));
  assert_eq!(error_kind(&response), "syntax");
  // the engine stays healthy after errors
  assert_eq!(display("1 + 1"), "2");
}

#[test]
fn null_with_reason() {
  let r = expr("1 + \"a\"");
  assert_eq!(r["isNull"], true);
  assert_eq!(r["display"], "null");
  assert_eq!(r["type"], "Null");
  assert!(r["nullReason"].as_str().is_some_and(|s| !s.is_empty()), "{r}");
  assert!(r["feel"].as_str().unwrap().starts_with("null("), "{r}");
  assert_eq!(r["typed"]["simple"]["isNil"], true);

  let r = expr("decimal(\"x\", 1)");
  assert_eq!(r["isNull"], true);
  assert!(r["nullReason"].is_string(), "{r}");

  // a plain null has no reason
  let r = expr("null");
  assert_eq!(r["isNull"], true);
  assert!(r.get("nullReason").is_none(), "{r}");
}

/// usize/isize are 32-bit on wasm32: positions and indexes beyond 2^32 must be a null with a
/// reason, not a panic (which would trap the wasm instance). Fixed in the fork ("Build on wasm32" commit).
#[test]
fn positions_beyond_32_bits_are_null_not_panic() {
  for e in [
    "[1,2,3][5000000000]",
    "[1,2,3][-5000000000]",
    "substring(\"abc\", 5000000000)",
    "substring(\"abcdef\", 2, 4294967298)",
    "sublist([1,2,3], 5000000000)",
  ] {
    let r = expr(e);
    assert_eq!(r["isNull"], true, "{e}: {r}");
    assert!(r["nullReason"].is_string(), "{e}: {r}");
  }
  assert_eq!(display("string length(\"abc\") + 4294967296"), "4294967299");
}

/// Results that overflow the decimal128 range (infinity) or are undefined (NaN, e.g. a failed
/// quantize) are null with a reason; they used to panic in FeelNumber's Display (a trap on wasm).
#[test]
fn non_finite_results_are_null_not_panic() {
  for e in [
    "9.999999999999999999999999999999999e6144 * 10",
    "1e6144 * 10",
    "9e6144 + 9e6144",
    "-9e6144 - 9e6144",
    "9e6144 - -9e6144",
    "1 / 1e-6176",
    "1e6144 * 10 - 1e6144 * 10",
    "0 * (1e6144 * 10)",
    "sum(9e6144, 9e6144)",
    "sum([9e6144, 9e6144])",
    "mean(9e6144, 9e6144)",
    "product(1e6000, 1e6000)",
    "abs(-9e6144) * 100",
    "string(9e6144 * 10)",
    "{a: 9e6144 * 10}.a",
    "[1e6144 * 10][1]",
    "floor(9e6144 * 10)",
    "9e6144 * 10 > 0",
    "even(9e6144 * 10)",
    "substring(\"abc\", 1e6144 * 10)",
    "date(1e6144 * 10, 1, 1)",
  ] {
    let r = expr(e);
    assert_eq!(r["isNull"], true, "{e}: {r}");
  }
  assert_eq!(expr("9e6144 + 9e6144")["nullReason"], "result is not a finite number (decimal overflow)");
  // modulo is exact, so a quotient beyond the range is no overflow (DMN 1.6 Table 77)
  assert_eq!(display("modulo(9e6144, 1e-6176)"), "0");
  assert_eq!(display("modulo(5, 1e-6176)"), "0");
  // a value that already has no digit below the requested scale is returned unchanged, even when
  // decimal128 can not hold it at that scale (fork commit 21)
  assert_eq!(display("decimal(1e33, 1)"), format!("1{}", "0".repeat(33)));
  assert_eq!(display("decimal(1e33, 2)"), format!("1{}", "0".repeat(33)));
  assert_eq!(display("decimal(1e100, 0)"), format!("1{}", "0".repeat(100)));
  assert_eq!(display("decimal(1, 40)"), "1");
  assert_eq!(display("decimal(9e6144, 1)"), format!("9{}", "0".repeat(6144)));
  assert_eq!(display("decimal(1, 6144)"), "1");
  // the largest finite values still work
  assert_eq!(display("9e6144 + 0"), format!("9{}", "0".repeat(6144)));
  assert_eq!(display("2 ** 100"), "1267650600228229401496703205376");
  // a null filter index (here: an overflowed one) filters nothing, as dsntk does for any null
  assert_eq!(display("[1,2,3][1e6144 * 10]"), "[]");
}

#[test]
fn zero_with_huge_exponent_is_displayed_as_0() {
  assert_eq!(display("0 * 1e-6000"), "0");
  assert_eq!(display("decimal(1/3, -6000)"), "0");
  assert_eq!(display("string(0.1E-6176)"), "\"0\"");
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","expression":"x","context":{"x":1e-999999999}}"#).unwrap());
  assert_eq!(r["display"], "0");
  // every zero prints as 0
  assert_eq!(display("0.000"), "0");
  assert_eq!(display("0 * 1e-10"), "0");
  assert_eq!(display("0"), "0");
}

/// A number has one text in `display`, `feel`, `typed` and `string()`: its value in plain digits,
/// without trailing zeros after the decimal point (DMN 1.6 §10.3.2.3.1: "many different pairs have
/// the same value"; Table 73: `string(1.1) = "1.1"`).
#[test]
fn a_number_has_one_text() {
  for (expression, text) in [
    ("1.50", "1.5"),
    ("1.0", "1"),
    ("decimal(1, 2)", "1"),
    ("decimal(0.5, 3)", "0.5"),
    ("abs(-1.50)", "1.5"),
    ("0.25 * 2", "0.5"),
    ("1.10 - 1.10", "0"),
    ("number(\"1 000 000.00\", \" \", \".\")", "1000000"),
    ("abs(exp(1e-34))", "1"),
    ("1.00E+2", "100"),
  ] {
    let r = expr(expression);
    assert_eq!(r["display"], text, "{expression}");
    assert_eq!(r["feel"], text, "{expression}");
    assert_eq!(r["typed"]["simple"]["text"], text, "{expression}");
  }
  assert_eq!(display("[1.50, {a: 2.0}]"), "[1.5, {\"a\": 2}]");
  assert_eq!(display("string(1.10)"), "\"1.1\"");
  assert_eq!(display("string(1.50) = \"1.5\""), "true");
  assert_eq!(display("string length(string(1.00))"), "1");
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","expression":"x","context":{"x":1.50}}"#).unwrap());
  assert_eq!(r["display"], "1.5");
}

/// DMN 1.6 §10.3.2.3.1: "There is no distinction between -0 and 0." A zero never prints with a
/// minus sign, in any text form.
#[test]
fn a_zero_has_no_sign() {
  for expression in ["-0", "0 * -5", "0 / -5", "-1 + 1", "round half up(-0.4)", "round down(-0.5)", "floor(-0.0)", "sqrt(-0.0)"] {
    let r = expr(expression);
    assert_eq!(r["display"], "0", "{expression}");
    assert_eq!(r["feel"], "0", "{expression}");
    assert_eq!(r["typed"]["simple"]["text"], "0", "{expression}");
  }
  assert_eq!(display("string(-0)"), "\"0\"");
  assert_eq!(display("string(-0) = \"0\""), "true");
  let r = eval_ok(serde_json::from_str(r#"{"op":"evaluate","expression":"x","context":{"x":-0}}"#).unwrap());
  assert_eq!(r["display"], "0");
}

/// sublist / substring / insert before / remove / list replace / filters: positions are converted
/// through 64-bit integers and checked with wide arithmetic, so wasm32 gives the same results and
/// null reasons as 64-bit native builds, and huge lengths can not wrap (they used to give wrong
/// strings or trap on wasm32).
#[test]
fn huge_positions_and_lengths_are_null_with_native_reasons() {
  let cases = [
    (
      "substring(\"abcdef\", 2, 4294967295)",
      "sublist: invalid range, len = 6, start position = 2, end position = 4294967297",
    ),
    (
      "substring(\"abcdef\", 4, 4294967293)",
      "sublist: invalid range, len = 6, start position = 4, end position = 4294967297",
    ),
    (
      "substring(\"abcdef\", -3, 4294967295)",
      "sublist: invalid range, len = 6, start position = 4, end position = 4294967299",
    ),
    (
      "substring(\"abcdef\", 2, 18446744073709551615)",
      "sublist: invalid range, len = 6, start position = 2, end position = 18446744073709551617",
    ),
    (
      "substring(\"abc\", 2, 4294967296)",
      "sublist: invalid range, len = 3, start position = 2, end position = 4294967298",
    ),
    ("substring(\"abc\", -4294967296)", "substring: position is out of range, len = 3, position = -4294967296"),
    (
      "sublist([1,2,3,4,5,6], 2, 4294967295)",
      "sublist: invalid range, len = 6, start position = 2, end position = 4294967297",
    ),
    (
      "sublist([1,2,3,4,5,6], -3, 4294967294)",
      "sublist: invalid range, len = 6, start position = 4, end position = 4294967298",
    ),
    (
      "sublist([1,2,3,4,5,6], 2, 18446744073709551615)",
      "sublist: invalid range, len = 6, start position = 2, end position = 18446744073709551617",
    ),
    (
      "sublist([1,2,3], 1, 4294967296)",
      "sublist: invalid range, len = 3, start position = 1, end position = 4294967297",
    ),
    ("sublist([1,2,3], -4294967296)", "sublist: position is out of range, len = 3, position = -4294967296"),
    ("sublist([1,2,3], -5, 1)", "sublist: invalid range, len = 3, start position = -1, end position = 0"),
    ("[1,2,3][4294967296]", "index in filter is out of range [1..3], actual index is 4294967296"),
    ("[1,2,3][-4294967297]", "index in filter is out of range [-3..-1], actual index is -4294967297"),
    ("list replace([1,2], 4294967297, 0)", "position exceeds list bounds"),
    ("list replace([1,2], -4294967297, 0)", "position exceeds list bounds"),
    ("insert before([1,2], 4294967297, 0)", "index is out of range"),
    ("remove([1,2], 4294967297)", "probably index is out of range"),
  ];
  for (e, reason) in cases {
    let r = expr(e);
    assert_eq!(r["isNull"], true, "{e}: {r}");
    assert_eq!(r["nullReason"], reason, "{e}");
  }
  // in-range positions are unaffected
  assert_eq!(display("substring(\"abcdef\", 2, 3)"), "\"bcd\"");
  assert_eq!(display("substring(\"abcdef\", -3, 2)"), "\"de\"");
  assert_eq!(display("sublist([1,2,3,4,5,6], 2, 3)"), "[2, 3, 4]");
  assert_eq!(display("sublist([1,2,3,4,5,6], -2)"), "[5, 6]");
  assert_eq!(display("insert before([1,2], -1, 0)"), "[1, 0, 2]");
  assert_eq!(display("remove([1,2,3], -1)"), "[1, 2]");
  assert_eq!(display("list replace([1,2], -1, 0)"), "[1, 0]");
  assert_eq!(display("[1,2,3][-1]"), "3");
}

#[test]
fn time_offset_out_of_range_is_null_not_panic() {
  for e in [
    "time(1, 2, 3, duration(\"PT18H\"))",
    "time(1, 2, 3, duration(\"-PT19H\"))",
    "time(1, 2, 3, duration(\"P1D\"))",
    "time(1, 2, 3, duration(\"PT18H1M\"))",
    "time(10, 0, 0, duration(\"P30D\"))",
    "date and time(date(\"2020-01-01\"), time(1, 2, 3, duration(\"PT20H\")))",
  ] {
    let r = expr(e);
    assert_eq!(r["isNull"], true, "{e}: {r}");
  }
  assert_eq!(display("string(time(1, 2, 3, duration(\"PT2H\")))"), "\"01:02:03+02:00\"");
}

/// The whole seconds of a duration are a 64-bit number on every target, so an offset of 2^32
/// seconds or more is out of range on wasm32 as natively, not taken modulo 2^32.
#[test]
fn time_offset_beyond_32_bits_is_null() {
  for (e, reason) in [
    ("time(10, 0, 0, duration(\"PT4294967296S\"))", "time: offset is out of range: P49710DT6H28M16S"),
    ("time(10, 0, 0, duration(\"PT4294971296S\"))", "time: offset is out of range: P49710DT7H34M56S"),
    ("time(10, 0, 0, duration(\"-PT4294967296S\"))", "time: offset is out of range: -P49710DT6H28M16S"),
  ] {
    let r = expr(e);
    assert_eq!(r["isNull"], true, "{e}: {r}");
    assert_eq!(r["nullReason"], reason, "{e}: {r}");
  }
}

/// Fractional seconds are read as an integer of nanoseconds, never through a binary double.
#[test]
fn fractional_seconds_are_exact() {
  assert_eq!(display("time(\"10:00:00.0157\")"), "\"10:00:00.0157\"");
  assert_eq!(display("@\"2020-01-01T10:00:00.0157Z\""), "\"2020-01-01T10:00:00.0157Z\"");
  assert_eq!(display("duration(\"PT0.000015839S\")"), "\"PT0.000015839S\"");
  assert_eq!(display("time(\"10:00:00.99999999999999999\")"), "\"10:00:00.999999999\"");
}

/// Fraction digits beyond nanoseconds are truncated with a warning that names the literal and the
/// dropped digits; a warning is no error, so it is not counted and strict mode does not stop.
#[test]
fn truncated_fractional_seconds_warn() {
  let message = "fractional seconds of '10:00:00.1234567891' truncated to nanoseconds; dropped digits: '1'";
  let response = req(json!({"op": "evaluate", "expression": "time(\"10:00:00.1234567891\")"}));
  assert_eq!(warnings(&response), [message]);
  assert_eq!(response["result"]["display"], "\"10:00:00.123456789\"");
  assert_eq!(response["diagnostics"]["count"], 0);
  let response = req(json!({"op": "evaluate", "expression": "@\"10:00:00.1234567891\"", "errorMode": "strict"}));
  assert_eq!(warnings(&response), [message]);
  let response = req(json!({
    "op": "evaluate",
    "expression": "truncated",
    "contextFormat": "feel",
    "context": "{truncated: time(\"10:00:00.1234567891\")}",
  }));
  assert_eq!(warnings(&response), [message]);
  let response = req(json!({"op": "evaluate", "expression": "time(\"10:00:00.1234567890\")"}));
  assert_eq!(warnings(&response), Vec::<&str>::new());
  let response = req(json!({"op": "evaluate", "expression": "time(10, 0, 0.1234567891)"}));
  assert_eq!(warnings(&response), ["fractional seconds of '0.1234567891' truncated to nanoseconds; dropped digits: '1'"]);
}

/// 24:00:00 is midnight only with a zero fraction (XML Schema Part 2 §3.2.7).
#[test]
fn a_time_of_day_is_below_24_hours() {
  for e in ["time(\"24:00:00.5\")", "@\"24:00:00.5\"", "date and time(\"2020-01-01T24:00:00.5Z\")"] {
    assert_eq!(expr(e)["isNull"], true, "{e}");
  }
  assert_eq!(display("time(\"24:00:00\")"), "\"00:00:00\"");
  assert_eq!(display("date and time(\"2020-12-31T24:00:00.000Z\")"), "\"2021-01-01T00:00:00Z\"");
}

#[test]
fn lexer_surrogate_escapes_are_syntax_errors_not_panic() {
  for e in ["\"\\uD800\"", "\"\\uD83D\"", "\"\\uD800x\"", "\"\\uDC00\"", "\"\\uD800\\u0041\""] {
    let response = req(json!({"op": "evaluate", "expression": e}));
    assert_eq!(error_kind(&response), "syntax", "{e}");
    let response = req(json!({"op": "parse", "expression": e}));
    assert_eq!(error_kind(&response), "syntax", "{e}");
  }
  assert_eq!(display("\"\\uD83D\\uDE40\""), "\"\u{1F640}\"");
  assert_eq!(display("\"\\u00e9\\u20AC\\U01F600\""), "\"\u{e9}\u{20ac}\u{1F600}\"");
}

/// Duration arithmetic used to wrap silently in release builds (`PT9223372036854775807S` gave
/// `-PT1S`); out-of-range durations are null now.
#[test]
fn duration_overflow_is_null_not_wrapped() {
  for e in [
    "duration(\"PT9223372036854775807S\")",
    "duration(\"PT99999999999H\")",
    "duration(\"P106752D\")",
    "duration(\"PT9223372036S\") + duration(\"PT9223372036S\")",
    "duration(\"-PT9223372036S\") - duration(\"PT9223372036S\")",
    "years and months duration(\"P9223372036854775807M\") + duration(\"P1M\")",
  ] {
    let r = expr(e);
    assert_eq!(r["isNull"], true, "{e}: {r}");
  }
  assert_eq!(display("duration(\"PT9223372036S\")"), "\"P106751DT23H47M16S\"");
  assert_eq!(display("-duration(\"P1DT2H\")"), "\"-P1DT2H\"");
}

/// `Value::type_of` on lists used to be exponential in the nesting depth (a 26-deep list took
/// seconds, 40 levels would never finish). A list nested 40 levels is typed exactly as a literal
/// and as an input context entry, and an expression parses against a context holding it; with an
/// exponential typing rule this test does not finish.
#[test]
fn nested_lists_are_typed_in_linear_time() {
  let deep = format!("{}1{}", "[".repeat(40), "]".repeat(40));
  let deep_type = format!("{}number{}", "list<".repeat(40), ">".repeat(40));
  assert_eq!(expr(&deep)["type"], deep_type);
  let request = format!(r#"{{"op":"evaluate","expression":"x","context":{{"x":{deep}}}}}"#);
  let r = eval_ok(serde_json::from_str(&request).unwrap());
  assert_eq!(r["type"], deep_type, "{r}");
  assert_eq!(r["feel"], deep, "{r}");
  let request = format!(r#"{{"op":"evaluate","expression":"count(x)","context":{{"x":{deep}}}}}"#);
  assert_eq!(eval_ok(serde_json::from_str(&request).unwrap())["display"], "1");
  let request = format!(r#"{{"op":"parse","expression":"x","context":{{"x":{deep}}}}}"#);
  assert_eq!(raw(&request)["tree"], " Name\n └─ `x`\n");
}

/// Deeply nested expressions are rejected before the recursive parser/evaluator code can overflow
/// the stack (a host-dependent trap on wasm), and runaway recursion stops at a fixed call depth.
#[test]
fn nesting_and_recursion_limits() {
  with_big_stack(nesting_and_recursion_limits_body);
}

fn nesting_and_recursion_limits_body() {
  let too_deep = |levels: usize| format!("expression is nested too deeply ({levels} levels, the limit is 300)");
  let trailing_operator = format!("1{}+", "+1".repeat(20000));
  for (e, message) in [
    (format!("1{}", "+1".repeat(20000)), too_deep(20001)),
    // a syntax error is reported before the depth is measured; it quotes a preview of the input
    (trailing_operator.clone(), format!("syntax error: {}… (40002 characters)", &trailing_operator[..120])),
    (format!("{}1{}", "[".repeat(5000), "]".repeat(5000)), too_deep(5001)),
    (format!("{}1{}", "{a:".repeat(3000), "}".repeat(3000)), too_deep(6001)),
    (format!("1{}", "+1".repeat(300)), too_deep(301)),
  ] {
    assert_error(&req(json!({"op": "evaluate", "expression": e})), "syntax", &message);
    assert_error(&req(json!({"op": "parse", "expression": e})), "syntax", &message);
  }
  assert_eq!(display(&format!("1{}", "+1".repeat(250))), "251");
  let response = req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": format!("{}1{}", "{a:".repeat(3000), "}".repeat(3000))}));
  assert_error(&response, "context", &format!("invalid FEEL context: {}", too_deep(6001)));
  // recursion
  let call_depth = "function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped";
  let rec = |n: u32| format!("{{f: function(n) if n = 0 then 0 else 1 + f(n - 1), r: f({n})}}.r");
  assert_eq!(display(&rec(150)), "150");
  for e in [rec(5000), "{f: function(n) f(n + 1), r: f(1)}.r".to_string()] {
    assert_error(&req(json!({"op": "evaluate", "expression": e})), "evaluation", call_depth);
  }
  let response = req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": "{f: function(n) f(n + 1), r: f(1)}"}));
  assert_error(&response, "context", call_depth);
  // a loop stops at the abort: it neither runs on through its items nor holds a result for each
  for e in [
    "{f: function(n) f(n + 1), r: for i in 1..10000000 return f(i)}.r",
    "{f: function(n) f(n + 1), r: for i in 1..100, j in 1..100000 return f(j)}.r",
    "{f: function(n) f(n + 1), l: [1, 2, 3], r: every i in l, j in l, k in l satisfies f(k) = null}.r",
    "{f: function(n) f(n + 1), r: (for i in 1..1000 return i)[f(item) = null]}.r",
    "{f: function(n) f(n + 1), r: sort([3, 1, 2], function(x, y) f(x) = null)}.r",
    "{f: function(n) f(n + 1), r: list replace([1, 2, 3], function(item, newItem) f(item), 0)}.r",
  ] {
    assert_error(&req(json!({"op": "evaluate", "expression": e})), "evaluation", call_depth);
  }
  // values nest at most 500 levels: a value 501 levels deep stops the evaluation, also when a
  // loop or a key list would nest it far deeper, and a value 500 levels deep is bound and returned
  let value_depth = "values are nested too deeply (more than 500 levels); evaluation stopped";
  // a list `5 * calls + base` levels deep, built by a recursion that adds 5 levels per call
  let list = |calls: u32, base: usize| {
    let base = format!("{}[]{}", "[".repeat(base - 1), "]".repeat(base - 1));
    format!("(function(f) f(f, {calls}))(function(g, n) if n = 0 then {base} else [[[[[g(g, n - 1)]]]]])")
  };
  assert_eq!(display(&format!("count({})", list(99, 5))), "1");
  assert_eq!(display(&format!("(function(x) count(x))({})", list(99, 5))), "1");
  for e in [
    format!("[{}]", list(99, 5)),
    list(100, 1),
    format!("{{a: {}}}", list(99, 5)),
    format!("for i in 1..2 return {}", list(99, 5)),
    "context put({}, for i in 1..100000 return \"a\", 0)".to_string(),
    "{f: function(x, n) if n = 0 then x else f([x], n - 1), r: for i in 1..50 return f(partial[-1], 190)}.r".to_string(),
  ] {
    assert_error(&req(json!({"op": "evaluate", "expression": e})), "evaluation", value_depth);
  }
  let response = req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": format!("{{a: {}}}", list(99, 5))}));
  assert_error(&response, "context", value_depth);
  // the limit does not leak into the next request
  assert_eq!(display(&rec(150)), "150");
  // a JSON context nested deeper than the request parser allows is a context error
  let request = format!(r#"{{"op":"evaluate","expression":"1","context":{{"x":{}1{}}}}}"#, "[".repeat(200), "]".repeat(200));
  let message = "context or input JSON is nested too deeply (at most about 125 levels): recursion limit exceeded at line 1 column 175";
  assert_error(&raw(&request), "context", message);
}

#[test]
fn json_numbers_that_underflow_warn() {
  for (number, fragment) in [
    ("1e-7000", "became 0"),
    ("1.5e-6176", "was rounded"),
    ("1.23456789012345678901234567890123456789", "more than 34"),
  ] {
    let response = raw(&format!(r#"{{"op":"evaluate","expression":"x","context":{{"x":{number}}}}}"#));
    assert_eq!(response["ok"], true, "{response}");
    let warnings = response["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{number}: {response}");
    assert!(warnings[0]["message"].as_str().unwrap().contains(fragment), "{number}: {response}");
  }
  let response = raw(r#"{"op":"evaluate","expression":"x","context":{"x":1e-6176}}"#);
  assert_eq!(response["warnings"].as_array().unwrap().len(), 0, "{response}");
}

#[test]
fn input_messages_name_the_place_of_the_value() {
  // golden texts of the input warnings and errors
  let message = |request: Json| -> String { req(request)["error"]["message"].as_str().unwrap().to_string() };
  assert_eq!(
    vec!["x.y[1].z: number 12345678901234567890123456789012345678 has more than 34 significant digits and was rounded to 12345678901234567890123456789012350000"],
    warnings(&req(
      json!({"op": "evaluate", "expression": "x", "context": {"x": {"y": [1, {"z": 12345678901234567890123456789012345678_u128}]}}})
    ))
  );
  assert_eq!(
    vec![
      "input[0]: number 1e-7000 is too small for a 34-digit decimal (exponent below -6176) and became 0",
      "input[1].k: number 1e-6177 is too small for a 34-digit decimal (exponent below -6176) and became 0"
    ],
    warnings(&raw(
      r#"{"op": "evaluate", "expression": "?", "dialect": "unaryTests", "input": [1e-7000, {"k": 1e-6177}]}"#
    ))
  );
  // a number longer than 40 characters is quoted in scientific notation
  assert_eq!(
    vec!["a[0]: number 1.5e-6176 is too small for a 34-digit decimal and was rounded to 2E-6176"],
    warnings(&raw(r#"{"op": "evaluate", "expression": "x", "context": {"a": [1.5e-6176]}}"#))
  );
  // the context's warnings come before the input's
  assert_eq!(
    vec![
      "a: number 1e-7000 is too small for a 34-digit decimal (exponent below -6176) and became 0",
      "input: number 1e-7000 is too small for a 34-digit decimal (exponent below -6176) and became 0"
    ],
    warnings(&raw(
      r#"{"op": "evaluate", "expression": "1", "dialect": "unaryTests", "input": 1e-7000, "context": {"a": 1e-7000}}"#
    ))
  );
  assert_eq!(
    "a. b [0]: empty context key",
    message(serde_json::from_str(r#"{"op": "evaluate", "expression": "x", "context": {"a": {" b ": [{"": 1}]}}}"#).unwrap())
  );
  assert_eq!(
    "context: empty context key",
    message(serde_json::from_str(r#"{"op": "evaluate", "expression": "x", "context": {" ": 1}}"#).unwrap())
  );
  assert_eq!(
    "input: empty context key",
    message(json!({"op": "evaluate", "expression": "1", "dialect": "unaryTests", "input": {"": 1}}))
  );
  assert_eq!(
    " x .q [0]: invalid number '1e+7000': <FeelNumberError> invalid number literal '1e+7000'",
    message(serde_json::from_str(r#"{"op": "evaluate", "expression": "x", "context": {" x ": {"q ": [1e7000]}}}"#).unwrap())
  );
  assert_eq!(
    "input[0]: invalid number '1e+7000': <FeelNumberError> invalid number literal '1e+7000'",
    message(serde_json::from_str(r#"{"op": "evaluate", "expression": "1", "dialect": "unaryTests", "input": [1e7000]}"#).unwrap())
  );
}

#[test]
fn boolean_unary_tests_with_question_mark_warn() {
  for (tests, number) in [("? > 5", 1), ("? > 5 and ? < 10", 1), ("not(? = 1)", 1), ("> 1, ? < 3", 2)] {
    let response = req(json!({"op": "evaluate", "dialect": "unaryTests", "expression": tests, "input": 7}));
    let expected = format!(
      "unary test #{number} is a boolean expression using '?': dsntk compares the input with its result (input = true/false) \
       instead of treating it as satisfied when true (DMN 1.2+); write e.g. '> 5' instead of '? > 5'"
    );
    assert_eq!(warnings(&response), [expected.as_str()], "{tests}");
  }
  for tests in ["> 5", "[1..10]", "(? * 0) + 7", "-", "\"a\""] {
    let response = req(json!({"op": "evaluate", "dialect": "unaryTests", "expression": tests, "input": 7}));
    assert_eq!(response["warnings"].as_array().unwrap().len(), 0, "{tests}: {response}");
  }
}

#[test]
fn syntax_error_messages_have_no_type_prefix() {
  let response = req(json!({"op": "evaluate", "expression": "1 + * 2"}));
  let message = response["error"]["message"].as_str().unwrap();
  assert!(!message.starts_with('<'), "{message}");
  assert!(message.contains("syntax error"), "{message}");
}

#[test]
fn context_keys_are_trimmed_but_inner_whitespace_is_kept() {
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "x", "context": {" x ": 1}}))["display"], "1");
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "a b", "context": {"a  b": 1}}))["isNull"], true);
  // a trimmed key is the same name as the untrimmed one; the later entry in key order (" a" < "a") wins
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "a", "context": {"a": 1, " a": 2}}))["display"], "1");
  // ("a" < "a "), also between other keys, in a nested context, and with several keys of one name
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "a", "context": {"a": 1, "a ": 2}}))["display"], "2");
  assert_eq!(
    eval_ok(json!({"op": "evaluate", "expression": "[a, b, c]", "context": {"a": 1, "b": 2, "c": 3, " b": 4, "b ": 5}}))["display"],
    "[1, 5, 3]"
  );
  assert_eq!(
    eval_ok(json!({"op": "evaluate", "expression": "[a, b, c]", "context": {"a": 1, "b": 2, "c": 3, " b": 4, "\tc": 5}}))["display"],
    "[1, 2, 3]"
  );
  assert_eq!(
    eval_ok(json!({"op": "evaluate", "expression": "x.a + x.z", "context": {"x": {"z": 10, "a": 1, "a ": 2}}}))["display"],
    "12"
  );
  assert_eq!(
    eval_ok(json!({"op": "evaluate", "expression": "[x[1].a, x[2].a]", "context": {"x": [{"a": 1, " a": 2}, {"a ": 3, "a": 4}]}}))["display"],
    "[1, 3]"
  );
  // the same in contexts of more entries than are inserted one by one: the keys in key order
  // are "\tk39\n", " k05", "k00" to "k07", "k07 ", "k08" to "k39", "k39  "
  let mut wide: serde_json::Map<String, Json> = (0..40).map(|i| (format!("k{i:02}"), json!(i))).collect();
  wide.extend([
    (" k05".to_string(), json!(-1)),
    ("k07 ".to_string(), json!(-2)),
    ("\tk39\n".to_string(), json!(-3)),
    ("k39  ".to_string(), json!(-4)),
  ]);
  let mut context = wide.clone();
  context.insert("w".to_string(), Json::Object(wide));
  let expression = "[k04, k05, k06, k07, k08, k38, k39, count(get entries(w)), w.k05, w.k07, w.k39]";
  let r = eval_ok(json!({"op": "evaluate", "expression": expression, "context": context}));
  assert_eq!(r["display"], "[4, 5, 6, -2, 8, 38, -4, 40, 5, -2, -4]", "{r}");
}

#[test]
fn unknown_names_are_null() {
  let r = expr("unknown variable + 1");
  assert_eq!(r["isNull"], true, "{r}");
}

/// Every message of a response: the error, the null reason, the diagnostics and the warnings.
fn messages(response: &Json) -> Vec<String> {
  let mut messages = vec![];
  for text in [&response["error"]["message"], &response["result"]["nullReason"]] {
    if let Some(text) = text.as_str() {
      messages.push(text.to_string());
    }
  }
  for list in [&response["diagnostics"]["messages"], &response["warnings"]] {
    for item in list.as_array().into_iter().flatten() {
      messages.push(item.as_str().or_else(|| item["message"].as_str()).unwrap().to_string());
    }
  }
  messages
}

#[test]
fn request_echoes_are_previews_and_every_message_is_capped() {
  let check = |request: Json, kind: &str, message: String| {
    let response = req(request);
    assert_error(&response, kind, &message);
    assert!(messages(&response).iter().all(|m| m.chars().count() <= 1000), "{response}");
  };
  let x = "x".repeat(200);
  check(
    json!({"op": "evaluate", "expression": "41 + 1", "dialect": x}),
    "request",
    format!("invalid 'dialect' \"{}… (200 characters) (expected \"expression\" or \"unaryTests\")", "x".repeat(119)),
  );
  check(
    json!({"op": "y".repeat(300)}),
    "request",
    format!("unknown op '{}… (300 characters)' (expected 'evaluate', 'evaluateAll', 'parse' or 'info')", "y".repeat(120)),
  );
  check(
    json!({"op": "evaluate", "expression": "1", "contextFormat": (1..=300).collect::<Vec<_>>()}),
    "request",
    format!(
      "invalid 'contextFormat' [{}… (300 items) (expected \"json\" or \"feel\")",
      &(1..=300).map(|i| i.to_string()).collect::<Vec<_>>().join(",")[..119]
    ),
  );
  let digits = "7".repeat(1000);
  let request = format!(r#"{{"op": "evaluate", "expression": "1", "tzOffsetMinutes": {digits}}}"#);
  let response = raw(&request);
  assert_error(
    &response,
    "request",
    &format!("'tzOffsetMinutes' must be an integer in -1080..=1080, got {}… (1000 characters)", &digits[..120]),
  );
  let key = "k".repeat(2000);
  let response = req(json!({"op": "evaluate", "expression": "1", "context": {key.clone(): {" ": 1}}}));
  assert_error(&response, "context", &format!("{}… (2000 characters): empty context key", &key[..120]));
  // every message quotes a path the same way: a preview and its size
  let response = raw(&format!(r#"{{"op": "evaluate", "expression": "1", "context": {{"{key}": {{"a": 1e-7000}}}}}}"#));
  assert_eq!(
    vec![format!(
      "{}… (2002 characters): number 1e-7000 is too small for a 34-digit decimal (exponent below -6176) and became 0",
      &key[..120]
    )],
    warnings(&response)
  );
  // a long number text is a preview, the rounded number is in scientific notation
  let long = format!("1.{}", "2".repeat(5000));
  let response = raw(&format!(r#"{{"op": "evaluate", "expression": "x", "context": {{"x": {long}}}}}"#));
  assert_eq!(
    vec![format!(
      "x: number {}… (5002 characters) has more than 34 significant digits and was rounded to 1.222222222222222222222222222222222",
      &long[..120]
    )],
    warnings(&response)
  );
  // a strict-mode error quotes the first reason, which quotes a preview
  let response = req(json!({"op": "evaluate", "expression": "(for i in 1..1000 return i) = 2", "errorMode": "strict"}));
  assert!(messages(&response).iter().all(|m| m.chars().count() <= 1000), "{response}");
  assert!(error_of(&response).1.ends_with("… (1000 items)' =?= '2'"), "{response}");
}

#[test]
fn request_error_messages() {
  // the request text (the serde_json texts of the locked serde_json version)
  assert_error(&raw("not json"), "request", "malformed request JSON: expected ident at line 1 column 2");
  assert_error(&raw(""), "request", "malformed request JSON: EOF while parsing a value at line 1 column 0");
  assert_error(
    &raw(r#"{"op": "evaluate", "expression": "1",}"#),
    "request",
    "malformed request JSON: trailing comma at line 1 column 38",
  );
  assert_error(&raw("[1]"), "request", "request must be a JSON object");
  // invalid UTF-8 (only possible through bytes: the wasm ABI, or call_bytes)
  let invalid_utf8 = "request is not valid UTF-8: invalid utf-8 sequence of 1 bytes from index 0";
  assert_error(&serde_json::from_str(&feel_wasm::call_bytes(&[0xff, 0xfe])).unwrap(), "request", invalid_utf8);
  #[cfg(target_arch = "wasm32")]
  assert_error(&serde_json::from_str(&abi_call(&[0xff, 0xfe])).unwrap(), "request", invalid_utf8);
  // the operation
  assert_error(&raw("{}"), "request", "missing 'op'");
  assert_error(&req(json!({"op": 1})), "request", "'op' must be a string");
  assert_error(
    &req(json!({"op": "explode"})),
    "request",
    "unknown op 'explode' (expected 'evaluate', 'evaluateAll', 'parse' or 'info')",
  );
  // the expression
  for op in ["evaluate", "parse"] {
    assert_error(&req(json!({"op": op})), "request", "missing 'expression'");
    assert_error(&req(json!({"op": op, "expression": 1})), "request", "'expression' must be a string");
    assert_error(&req(json!({"op": op, "expression": null})), "request", "'expression' must be a string");
  }
  // the options name the offending JSON value
  for (option, value, expected) in [
    ("dialect", json!("boxed"), "invalid 'dialect' \"boxed\" (expected \"expression\" or \"unaryTests\")"),
    ("dialect", json!(1), "invalid 'dialect' 1 (expected \"expression\" or \"unaryTests\")"),
    ("language", json!("x"), "invalid 'language' \"x\" (expected \"feel\" or \"b-feel\")"),
    ("language", json!(true), "invalid 'language' true (expected \"feel\" or \"b-feel\")"),
    ("errorMode", json!("x"), "invalid 'errorMode' \"x\" (expected \"lenient\" or \"strict\")"),
    ("errorMode", json!({"a": 1}), "invalid 'errorMode' {\"a\":1} (expected \"lenient\" or \"strict\")"),
    ("contextFormat", json!("xml"), "invalid 'contextFormat' \"xml\" (expected \"json\" or \"feel\")"),
    ("contextFormat", json!(2), "invalid 'contextFormat' 2 (expected \"json\" or \"feel\")"),
    ("tzOffsetMinutes", json!(5000), "'tzOffsetMinutes' must be an integer in -1080..=1080, got 5000"),
    ("tzOffsetMinutes", json!(-1081), "'tzOffsetMinutes' must be an integer in -1080..=1080, got -1081"),
    ("tzOffsetMinutes", json!(1.5), "'tzOffsetMinutes' must be an integer in -1080..=1080, got 1.5"),
    ("tzOffsetMinutes", json!("UTC"), "'tzOffsetMinutes' must be a number, got \"UTC\""),
  ] {
    let mut request = json!({"op": "evaluate", "expression": "1"});
    request[option] = value;
    assert_error(&req(request), "request", expected);
  }
  // i64::MIN must not slip through the range check (abs() overflow)
  assert_error(
    &raw(r#"{"op":"evaluate","expression":"1","tzOffsetMinutes":-9223372036854775808}"#),
    "request",
    "'tzOffsetMinutes' must be an integer in -1080..=1080, got -9223372036854775808",
  );
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": -1080}))["display"], "1");
  // strict mode reports the first diagnostic
  assert_error(
    &req(json!({"op": "evaluate", "expression": "if 1 + \"a\" = null then 1 else 2", "errorMode": "strict"})),
    "evaluation",
    "strict error mode: incompatible types in addition: 1(number) + \"a\"(string)",
  );
}

/// `and` and `or` evaluate the right operand only when the left one does not decide the result
/// (DMN 1.6 Table 51), so an error of a right operand that cannot change the result is not
/// counted, strict mode passes, and a recursion they guard returns its value; the left operand is
/// always evaluated.
#[test]
fn a_right_operand_that_cannot_change_the_result_is_not_evaluated() {
  let addition = "incompatible types in addition: 1(number) + \"a\"(string)";
  for (expression, language, shown) in [
    ("false and (1 + \"a\")", "feel", "false"),
    ("true or (1 + \"a\")", "feel", "true"),
    ("false and (1 + \"a\")", "b-feel", "false"),
    ("{f: function(n) n <= 0 or f(n - 1), r: f(3)}.r", "feel", "true"),
    ("{f: function(n) n > 0 and f(n - 1), r: f(3)}.r", "feel", "false"),
  ] {
    for error_mode in ["lenient", "strict"] {
      let response = req(json!({"op": "evaluate", "expression": expression, "language": language, "errorMode": error_mode}));
      assert_eq!(response["result"]["display"], shown, "{expression} {language} {error_mode}: {response}");
      assert_eq!(response["diagnostics"]["count"], 0, "{expression}: {response}");
    }
  }
  let response = req(json!({"op": "evaluate", "expression": "true or @\"2021-01-01T10:00:00.1234567891\" = null"}));
  assert_eq!(warnings(&response), Vec::<&str>::new());
  let response = req(json!({"op": "evaluate", "expression": "(1 + \"a\") and false"}));
  assert_eq!(response["result"]["display"], "false", "{response}");
  assert_eq!(response["diagnostics"]["count"], 1, "{response}");
  assert_error(
    &req(json!({"op": "evaluate", "expression": "true and (1 + \"a\")", "errorMode": "strict"})),
    "evaluation",
    &format!("strict error mode: {addition}"),
  );
}

/// A filter evaluates its expression once in the enclosing scope (DMN 1.6 Table 69, row 1): a
/// number selects the item; anything else selects nothing and records nothing, and the expression
/// is evaluated for each item. So a filter over item entries passes strict mode, and an index
/// reports its own errors once.
#[test]
fn a_filter_records_only_the_evaluations_it_uses() {
  let items = json!([{"amount": 50}, {"amount": 150}, {"amount": 200}]);
  for error_mode in ["lenient", "strict"] {
    let response = req(json!({"op": "evaluate", "expression": "sum(items[amount > 100].amount)", "context": {"items": items}, "errorMode": error_mode}));
    assert_eq!(response["result"]["display"], "350", "{error_mode}: {response}");
    assert_eq!(response["diagnostics"]["count"], 0, "{response}");
  }
  let response = req(json!({"op": "evaluate", "expression": "[10, 20, 30][count([1 + \"a\"])]"}));
  assert_eq!(response["result"]["display"], "10", "{response}");
  assert_eq!(response["diagnostics"]["count"], 1, "{response}");
  assert_eq!(display("{a: 2, r: [{a: \"x\"}, {a: \"y\"}][a]}.r"), "{\"a\": \"y\"}");
}

/// The request is parsed first; then, of several invalid fields, the one read first is reported:
/// dialect, language, errorMode, expression, tzOffsetMinutes, contextFormat, context, input, and
/// last the syntax of the expression. `parse` reads only dialect, expression, contextFormat,
/// context and input; `info` reads nothing but `op`.
#[test]
fn request_error_order() {
  let unknown_op = "unknown op 'explode' (expected 'evaluate', 'evaluateAll', 'parse' or 'info')";
  let dialect = "invalid 'dialect' \"boxed\" (expected \"expression\" or \"unaryTests\")";
  let language = "invalid 'language' \"x\" (expected \"feel\" or \"b-feel\")";
  let error_mode = "invalid 'errorMode' \"x\" (expected \"lenient\" or \"strict\")";
  let missing_expression = "missing 'expression'";
  let tz_offset = "'tzOffsetMinutes' must be an integer in -1080..=1080, got 5000";
  let context_format = "invalid 'contextFormat' \"xml\" (expected \"json\" or \"feel\")";
  let json_context = "with contextFormat \"json\", 'context' must be a JSON object";
  let input = "input: empty context key";
  for (request, kind, message) in [
    (json!({"op": "explode", "dialect": "boxed"}), "request", unknown_op),
    (json!({"op": "evaluate", "dialect": "boxed", "language": "x"}), "request", dialect),
    (json!({"op": "evaluate", "dialect": "boxed"}), "request", dialect),
    (json!({"op": "evaluate", "language": "x", "errorMode": "x"}), "request", language),
    (json!({"op": "evaluate", "errorMode": "x"}), "request", error_mode),
    (json!({"op": "evaluate", "tzOffsetMinutes": 5000}), "request", missing_expression),
    (
      json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": 5000, "contextFormat": "xml"}),
      "request",
      tz_offset,
    ),
    (json!({"op": "evaluate", "expression": "1 +", "tzOffsetMinutes": 5000}), "request", tz_offset),
    (
      json!({"op": "evaluate", "expression": "1", "contextFormat": "xml", "context": [1]}),
      "request",
      context_format,
    ),
    (json!({"op": "evaluate", "expression": "1 +", "context": [1]}), "context", json_context),
    (
      json!({"op": "evaluate", "expression": "1", "dialect": "unaryTests", "context": [1], "input": {"": 1}}),
      "context",
      json_context,
    ),
    (json!({"op": "evaluate", "expression": "1 +", "dialect": "unaryTests", "input": {"": 1}}), "context", input),
    (json!({"op": "parse", "dialect": "boxed", "context": [1]}), "request", dialect),
    (json!({"op": "parse", "context": [1]}), "request", missing_expression),
    (
      json!({"op": "parse", "expression": "1", "language": "x", "contextFormat": "xml"}),
      "request",
      context_format,
    ),
    (json!({"op": "parse", "expression": "1 +", "context": [1]}), "context", json_context),
    (json!({"op": "parse", "expression": "1 +", "dialect": "unaryTests", "input": {"": 1}}), "context", input),
  ] {
    assert_error(&req(request), kind, message);
  }
  // a JSON nesting error is found while the request is parsed, before 'op' is read
  let request = format!(r#"{{"x":{}1{}}}"#, "[".repeat(200), "]".repeat(200));
  let message = "context or input JSON is nested too deeply (at most about 125 levels): recursion limit exceeded at line 1 column 132";
  assert_error(&raw(&request), "context", message);
  // fields an operation does not read are not checked
  let parsed = req(json!({"op": "parse", "expression": "1", "language": "x", "errorMode": "x", "tzOffsetMinutes": "x"}));
  assert_eq!(parsed["ok"], true, "{parsed}");
  assert_eq!(req(json!({"op": "info", "dialect": "boxed", "expression": 1}))["ok"], true);
}

// ---------------------------------------------------------------- parse / info

#[test]
fn parse_tree() {
  let response = req(json!({"op": "parse", "expression": "1 + a"}));
  assert_eq!(response["ok"], true, "{response}");
  let tree = response["tree"].as_str().unwrap();
  assert!(tree.contains("Add"), "{tree}");
  assert!(!tree.contains('\u{1b}'), "tree must not contain ANSI escapes: {tree:?}");
  assert!(response["elapsedMicros"].is_u64());

  let response = req(json!({"op": "parse", "dialect": "unaryTests", "expression": "> 5, < 2"}));
  assert_eq!(response["ok"], true, "{response}");
  // names with spaces need the context to parse
  let response = req(json!({"op": "parse", "expression": "Grace's age + 1", "context": {"Grace's age": 1}}));
  assert_eq!(response["ok"], true, "{response}");
  assert!(response["tree"].as_str().unwrap().contains("Grace's age"), "{response}");
}

#[test]
fn info() {
  let response = req(json!({"op": "info"}));
  assert_eq!(response["ok"], true);
  assert_eq!(response["engine"], "rheon");
  assert_eq!(response["numberBackend"], "dashu");
  // a 40-digit SHA; `engineCommit` may carry `-dirty` when built from a modified work tree
  let sha = |key: &str| {
    let s = response[key].as_str().unwrap_or_else(|| panic!("{key} missing: {response}"));
    let s = s.strip_suffix("-dirty").unwrap_or(s);
    assert!(s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()), "{key} is not a SHA: {s}");
  };
  sha("engineCommit");
  sha("upstreamCommit");
  assert!(response["version"].is_string());
}

// ---------------------------------------------------------------- time zone

fn eval_tz(expression: &str, tz: i64, context: Json) -> Json {
  eval_ok(json!({"op": "evaluate", "expression": expression, "tzOffsetMinutes": tz, "context": context}))
}

#[test]
fn tz_offset_moves_now() {
  for (offset, expected) in [(600, "PT10H"), (-240, "-PT4H"), (330, "PT5H30M")] {
    let utc = eval_tz("string(now())", 0, json!({}))["feel"].as_str().unwrap().trim_matches('"').to_string();
    let r = eval_tz(
      "date and time(string(now())) - date and time(t0) - duration(d) between duration(\"-PT10S\") and duration(\"PT10S\")",
      offset,
      json!({"t0": utc, "d": expected}),
    );
    assert_eq!(r["display"], "true", "offset {offset}: {r}");
  }
}

#[test]
fn tz_offset_moves_today() {
  // +14:00 and -12:00 are 26 hours apart, so "today" always differs by one or two days.
  let east = eval_tz("today()", 14 * 60, json!({}))["feel"].as_str().unwrap().to_string();
  let west = eval_tz("today()", -12 * 60, json!({}))["feel"].as_str().unwrap().to_string();
  let r = eval_tz("date(east) - date(west) in [duration(\"P1D\"), duration(\"P2D\")]", 0, json!({"east": east, "west": west}));
  assert_eq!(r["display"], "true", "east {east} west {west}");
  // today() is the date part of now() for the same offset
  for offset in [-720, -240, 0, 330, 840] {
    let r = eval_tz("today() = date(now())", offset, json!({}));
    assert_eq!(r["display"], "true", "offset {offset}");
  }
}

#[test]
fn tz_offset_does_not_leak_between_requests() {
  let with_tz = eval_tz("string(now())", 600, json!({}))["feel"].as_str().unwrap().trim_matches('"').to_string();
  let without = eval_ok(json!({"op": "evaluate", "expression": "string(now())"}))["feel"]
    .as_str()
    .unwrap()
    .trim_matches('"')
    .to_string();
  let check = eval_tz(
    "date and time(a) - date and time(b) between duration(\"PT9H59M50S\") and duration(\"PT10H0M10S\")",
    0,
    json!({"a": with_tz, "b": without}),
  );
  // without tzOffsetMinutes local time is UTC on every target (natively too, not the host zone)
  assert_eq!(check["display"], "true", "{with_tz} vs {without}");
}

#[test]
fn local_time_defaults_to_utc_on_every_target() {
  assert_eq!(display("date and time(\"2020-01-01T10:00:00\") = date and time(\"2020-01-01T10:00:00Z\")"), "true");
  assert_eq!(display("time(\"10:00:00\") - time(\"08:00:00Z\")"), "\"PT2H\"");
  let r = eval_tz("time(\"10:00:00\") - time(\"08:00:00Z\")", -240, json!({}));
  assert_eq!(r["display"], "\"PT6H\"");
}

// ---------------------------------------------------------------- raw ABI (wasm32 only)

#[cfg(target_arch = "wasm32")]
fn abi_call(bytes: &[u8]) -> String {
  unsafe {
    let ptr = feel_wasm::feel_alloc(bytes.len());
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    let packed = feel_wasm::feel_call(ptr, bytes.len());
    let resp_ptr = (packed >> 32) as usize as *mut u8;
    let resp_len = (packed & 0xFFFF_FFFF) as usize;
    let text = String::from_utf8(std::slice::from_raw_parts(resp_ptr, resp_len).to_vec()).unwrap();
    feel_wasm::feel_free(resp_ptr, resp_len);
    text
  }
}

#[cfg(target_arch = "wasm32")]
#[test]
fn abi_roundtrip_and_no_leaks() {
  let request = br#"{"op":"evaluate","expression":"for i in 1..50 return {n: i / 3, s: string(i)}"}"#;
  let first: Json = serde_json::from_str(&abi_call(request)).unwrap();
  assert_eq!(first["ok"], true);
  // warm up, then make sure memory stops growing
  for _ in 0..50 {
    abi_call(request);
  }
  let pages_before = core::arch::wasm32::memory_size(0);
  for _ in 0..500 {
    abi_call(request);
    abi_call(b"{\"op\":\"evaluate\",\"expression\":\"1 +\"}");
    abi_call(b"garbage");
  }
  let pages_after = core::arch::wasm32::memory_size(0);
  assert_eq!(pages_before, pages_after, "linear memory grew: {pages_before} -> {pages_after} pages");
  // zero-length and freed-without-call buffers
  let resp: Json = serde_json::from_str(&abi_call(b"")).unwrap();
  assert_eq!(resp["error"]["kind"], "request");
  let p = feel_wasm::feel_alloc(16);
  unsafe { feel_wasm::feel_free(p, 16) };
}

// ---------------------------------------------------------------- fork fixes (regressions)

#[test]
fn scale_rounding_keeps_the_scale_for_unit_coefficients() {
  // upstream dsntk returned ±1 whenever the rounded coefficient was ±1 (fork commit c760d4d)
  assert_eq!(display("round up(0.1, 1)"), "0.1");
  assert_eq!(display("round up(0.05, 1)"), "0.1");
  assert_eq!(display("round up(0.01, 2)"), "0.01");
  assert_eq!(display("round up(10, -1)"), "10");
  assert_eq!(display("round up(1, -1)"), "10");
  assert_eq!(display("round half up(0.05, 1)"), "0.1");
  assert_eq!(display("round half down(0.15, 1)"), "0.1");
  assert_eq!(display("round down(0.19, 1)"), "0.1");
  assert_eq!(display("floor(0.19, 1)"), "0.1");
  assert_eq!(display("ceiling(0.01, 1)"), "0.1");
  assert_eq!(display("floor(-0.0000001, 6)"), "-0.000001");
  assert_eq!(display("round up(0.1, 1) * 10"), "1");
  assert_eq!(display("round up(0.11, 1)"), "0.2");
  assert_eq!(display("round up(0.95, 1)"), "1");
  assert_eq!(display("decimal(0.00001, 5)"), "0.00001");
}

#[test]
fn for_range_whose_step_is_lost_to_rounding_terminates() {
  // `1e34 + 1 = 1e34` at 34 digits: upstream looped forever (fork commit 55c7406)
  assert_eq!(display("for i in 1e34..1e34 return i"), "[10000000000000000000000000000000000]");
  assert_eq!(display("count(for i in 1e40..(1e40 + 5) return i)"), "1");
  assert_eq!(display("count(for i in -1e40..-1e40, j in 1..2 return j)"), "2");
  assert_eq!(display("for i in 1e33..(1e33 + 2) return i - 1e33"), "[0, 1, 2]");
}

#[test]
fn temporal_ordering_across_offsets_and_midnight_wrap() {
  // fork commits 4d61eb1 and cf21d52
  let tz0 = |e: &str| {
    eval_ok(json!({"op": "evaluate", "expression": e, "tzOffsetMinutes": 0}))["display"]
      .as_str()
      .unwrap()
      .to_string()
  };
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00+01:00" < @"2026-10-17T10:00:00Z""#), "true");
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00+01:00" >= @"2026-10-17T09:00:00Z""#), "true");
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00+01:00" > @"2026-10-17T08:59:59Z""#), "true");
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00@Europe/Warsaw" < @"2026-10-17T10:00:00Z""#), "true");
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00+01:00" in [@"2026-10-17T09:00:00Z"..@"2026-10-17T09:00:00Z"]"#), "true");
  assert_eq!(tz0(r#"@"10:00:00+01:00" < @"10:00:00Z""#), "true");
  assert_eq!(tz0(r#"@"10:00:00+02:00" = @"08:00:00Z""#), "true");
  assert_eq!(tz0(r#"is(@"20:00:50+00:00", @"21:00:50+01:00")"#), "false");
  assert_eq!(
    tz0(r#"sort([@"2026-10-17T10:00:00Z", @"2026-10-17T10:00:00+01:00"], function(a, b) a < b)"#),
    r#"["2026-10-17T10:00:00+01:00", "2026-10-17T10:00:00Z"]"#
  );
  // a local and a zoned value stay incomparable, like their subtraction (dsntk semantics)
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00" < @"2026-10-17T11:00:00Z""#), "false");
  assert_eq!(tz0(r#"@"2026-10-17T10:00:00" < @"2026-10-17T11:00:00""#), "true");
  // time + duration wraps around midnight, nanoseconds included
  assert_eq!(display(r#"time("23:59:59") + duration("PT1S")"#), r#""00:00:00""#);
  assert_eq!(display(r#"time("23:59:59.999999999") + duration("PT0.000000001S")"#), r#""00:00:00""#);
  assert_eq!(display(r#"time("23:00:00") + duration("PT2H")"#), r#""01:00:00""#);
  assert_eq!(display(r#"time("00:00:00") - duration("PT1S")"#), r#""23:59:59""#);
  assert_eq!(display(r#"(time("23:59:59") + duration("PT1S")) = time("00:00:00")"#), "true");
}

#[test]
fn null_reasons_are_readable_and_short() {
  // fork commit 5ecdede
  let reason = |e: &str| expr(e)["nullReason"].as_str().unwrap_or_else(|| panic!("no nullReason for {e}")).to_string();
  assert!(
    reason(r#"time("10:00:00+15:00")"#).starts_with("time: invalid time literal '10:00:00+15:00'"),
    "{}",
    reason(r#"time("10:00:00+15:00")"#)
  );
  assert_eq!(reason("exp(14150)"), "exp: result is not a finite number (decimal overflow)");
  assert_eq!(reason("log(0)"), "log: argument must be a positive number");
  assert_eq!(reason("log(-1)"), "log: argument must be a positive number");
  assert_eq!(reason(r#"duration("P999999999999999999D")"#), "duration: invalid duration literal: 'P999999999999999999D'");
  assert_eq!(reason("sublist([1,2,3], 9e6144)"), "sublist: invalid position value: 9E+6144");
  assert_eq!(reason("sublist([1,2,3], 1, 1.5e6000)"), "sublist: invalid length value: 1.5E+6000");
  assert_eq!(reason(r#"substring("abc", -9e6144)"#), "substring: invalid start position value: -9E+6144");
  assert_eq!(reason("sublist([1,2,3], 18446744073709551616)"), "sublist: invalid position value: 18446744073709551616");
  assert_eq!(reason(r#"time(10, 0, 0, duration("PT18H"))"#), "time: offset is out of range: PT18H");
}

#[test]
fn long_path_chains_are_parsed_to_the_nesting_limit() {
  // The lexer was cubic in the chain length (docs/FORK.md, commit 17). A chain of 1000 steps is
  // lexed and parsed completely and then rejected by the 300-level nesting limit (CONTRACT §2);
  // the parser's tests (path expression _0015) check that the lexer's work grows linearly with
  // the steps.
  let expression = format!("{{a:1}}{}", ".a".repeat(1000));
  for op in ["parse", "evaluate"] {
    let response = req(json!({"op": op, "expression": expression}));
    assert_error(&response, "syntax", "expression is nested too deeply (1003 levels, the limit is 300)");
  }
  let ok = req(json!({"op": "evaluate", "expression": format!("{{a:{{a:1}}}}{}", ".a".repeat(2))}));
  assert_eq!(ok["result"]["display"], "1", "{ok}");
}

#[test]
fn full_feel_scale_range_and_regex_back_references() {
  // fork commits 21-22: DMN TCK 1141-1144 017_a/017_b and 1111 K2-MatchesFunc-17, unpatched
  assert_eq!(display("round up(5.5, 6176)"), "5.5");
  assert_eq!(display("round down(5.5, 6176)"), "5.5");
  assert_eq!(display("round half up(5.5, 6176)"), "5.5");
  assert_eq!(display("round half down(5.5, 6176)"), "5.5");
  assert_eq!(display("decimal(5.5, 6176)"), "5.5");
  assert_eq!(display("round up(5.55, 1)"), "5.6");
  let r = expr("round up(5.5, 6177)");
  assert_eq!(r["isNull"], true, "{r}");
  assert_eq!(
    r["nullReason"],
    "[core::round_up] <FeelNumberError> invalid scale, allowed range is -6111..6176, actual is 6177"
  );
  assert_eq!(display(r#"matches("aA", "(a)\\1", "i")"#), "true");
  assert_eq!(display(r#"matches("ab", "(a)\\1")"#), "false");
  // a pattern the backtracking engine gives up on stops the evaluation: the limit is not a FEEL result
  let pattern = format!("{}!", "a".repeat(40));
  let limit = r"matches: the pattern '^(a|a)*\1$' exceeds the regex engine's backtracking limit; evaluation stopped";
  let e = format!(r#"matches("{pattern}", "^(a|a)*\\1$")"#);
  assert_error(&req(json!({"op": "evaluate", "expression": e})), "evaluation", limit);
  assert_error(
    &req(json!({"op": "evaluate", "expression": format!("{e} or true"), "language": "b-feel"})),
    "evaluation",
    limit,
  );
  let context = format!(r#"{{x: {e}}}"#);
  assert_error(
    &req(json!({"op": "evaluate", "expression": "x", "contextFormat": "feel", "context": context})),
    "context",
    limit,
  );
  // a pattern within the budget answers
  assert_eq!(
    display(r#"matches("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "^(a+)+\\1$")"#),
    "false"
  );
}

/// Sends a request and returns the whole response (asserting success).
fn ok(request: Json) -> Json {
  let response = req(request);
  assert_eq!(response["ok"], true, "expected success, got {response}");
  response
}

#[test]
fn b_feel_language() {
  // DMN 1.6 chapter 11: language "b-feel"; the FEEL error behind the default is a diagnostic
  let response = ok(json!({"op": "evaluate", "expression": "\"The result is: \" + 1", "language": "b-feel"}));
  assert_eq!(response["result"]["display"], "\"The result is: 1\"", "{response}");
  assert_eq!(response["diagnostics"]["count"], 1, "{response}");
  let response = ok(json!({"op": "evaluate", "expression": "\"a\" = 1", "language": "b-feel"}));
  assert_eq!(response["result"]["display"], "false", "{response}");
  let response = ok(json!({"op": "evaluate", "expression": "\"a\" = 1"}));
  assert_eq!(response["result"]["display"], "null", "{response}");
  let response = req(json!({"op": "evaluate", "expression": "1", "language": "x"}));
  assert_eq!(error_kind(&response), "request", "{response}");
}

#[test]
fn error_modes() {
  // DMN 1.6 §7.3.8: lenient (default) continues and lists the errors; strict stops at the first
  let expression = "if 1 + \"a\" = null then \"ok\" else \"no\"";
  let response = ok(json!({"op": "evaluate", "expression": expression}));
  assert_eq!(response["result"]["display"], "\"ok\"", "{response}");
  assert_eq!(response["diagnostics"]["count"], 1, "{response}");
  let response = req(json!({"op": "evaluate", "expression": expression, "errorMode": "strict"}));
  assert_eq!(error_kind(&response), "evaluation", "{response}");
  assert!(response["error"]["message"].as_str().unwrap().starts_with("strict error mode: "), "{response}");
  // a clean evaluation is unaffected by strict mode
  let response = ok(json!({"op": "evaluate", "expression": "1 + 1", "errorMode": "strict"}));
  assert_eq!(response["result"]["display"], "2", "{response}");
  assert_eq!(response["diagnostics"]["count"], 0, "{response}");
  let response = req(json!({"op": "evaluate", "expression": "1", "errorMode": "x"}));
  assert_eq!(error_kind(&response), "request", "{response}");
}

/// Asserts that `expression` answers `display` with no error counted, and that strict mode answers
/// the same result: a null the specification defines for its input is no error (DMN 1.6 §7.3.8).
#[track_caller]
fn assert_no_error(expression: &str, display: &str) {
  let lenient = ok(json!({"op": "evaluate", "expression": expression}));
  assert_eq!(lenient["result"]["display"], display, "{lenient}");
  assert_eq!(lenient["diagnostics"]["count"], 0, "{lenient}");
  let strict = ok(json!({"op": "evaluate", "expression": expression, "errorMode": "strict"}));
  assert_eq!(strict["result"], lenient["result"], "{strict}");
}

#[test]
fn defined_nulls_are_no_errors() {
  // Table 51: and, or with an operand that is no boolean; §10.3.4.4: sum and mean of []; Table 82:
  // get value of a missing key, which a path over a list selects per item
  for expression in ["true and 1", "1 and true", "false or \"x\"", "1 or 1", "sum([])", "mean([])", "get value({a: 1}, \"b\")"] {
    assert_no_error(expression, "null");
  }
  assert_no_error("[{a: 1}].b", "[null]");
}

#[test]
fn stddev_of_fewer_than_two_numbers_is_no_error() {
  // §10.3.4.4: stddev( [ 47 ] ) = null and stddev( 47 ) = null
  for expression in ["stddev([47])", "stddev([])", "stddev(47)"] {
    assert_no_error(expression, "null");
  }
  let response = ok(json!({"op": "evaluate", "expression": "stddev([47])", "errorMode": "strict"}));
  assert_eq!(response["result"]["nullReason"], "stddev: minimum two input arguments expected", "{response}");
}

#[test]
fn a_missing_context_entry_is_no_error() {
  // §10.3.2.6: m.key1 is get value(m, "key1"), which Table 82 defines as null for a missing key;
  // the null keeps its reason as a note, which is not counted and does not fail strict mode
  assert_no_error("{a: 1}.b", "null");
  let response = ok(json!({"op": "evaluate", "expression": "{a: 1}.b", "errorMode": "strict"}));
  assert_eq!(response["result"]["nullReason"], "build_path: no entry b in context: {a: 1}", "{response}");
  assert_eq!(response["diagnostics"]["messages"], json!([]), "{response}");
}

#[test]
fn if_takes_the_else_branch_unless_the_condition_is_true() {
  // Table 50: "A conditional if a then b else c is equal to b if a is true, and equal to c
  // otherwise", so a condition that is no boolean is no error
  for condition in ["9", "\"x\"", "[true]", "{}", "null"] {
    assert_no_error(&format!("if {condition} then 1 else 2"), "2");
  }
}

#[test]
fn not_equal_reports_like_equal() {
  // §10.3.2.15: FEEL(e1 != e2) is FEEL(not(e1 = e2)), so operands that = cannot compare are the
  // same error for !=
  let message = "equal err '\"a\"' =?= '1'";
  let lenient = ok(json!({"op": "evaluate", "expression": "\"a\" != 1"}));
  assert_eq!(lenient["result"]["nullReason"], message, "{lenient}");
  assert_eq!(lenient["diagnostics"]["count"], 1, "{lenient}");
  let strict = req(json!({"op": "evaluate", "expression": "\"a\" != 1", "errorMode": "strict"}));
  assert_eq!(strict["ok"], false, "{strict}");
  assert_eq!(strict["error"]["message"], format!("strict error mode: {message}"), "{strict}");
  let b_feel = ok(json!({"op": "evaluate", "expression": "\"a\" != 1", "language": "b-feel"}));
  assert_eq!(b_feel["result"]["display"], "true", "{b_feel}");
  assert_eq!(b_feel["diagnostics"]["messages"], json!([message]), "{b_feel}");
}

#[test]
fn a_null_operand_of_an_arithmetic_operator_is_an_error() {
  // before Table 57 and Table 59: "if input values are not of the listed types, the result is
  // null", and the official TCK expects an error result for a null operand of every arithmetic
  // operator, on either side (compliance-level-3/0100-arithmetic): the value is null, the error
  // is counted, and strict mode stops at it
  for (expression, message) in [
    ("1 + null", "[addition] incompatible types: 1 + null"),
    ("null + 1", "[addition] incompatible types: null + 1"),
    ("\"a\" + null", "[addition] incompatible types: \"a\" + null"),
    ("1 - null", "[subtraction] incompatible types: 1 - null"),
    ("null - 1", "[subtraction] incompatible types: null - 1"),
    ("1 * null", "[multiplication] incompatible types: 1 * null"),
    ("null * 1", "[multiplication] incompatible types: null * 1"),
    ("1 / null", "[division] incompatible types: 1 / null"),
    ("null / 1", "[division] incompatible types: null / 1"),
    ("2 ** null", "[exponentiation] incompatible types: 2 ** null"),
    ("null ** 2", "[exponentiation] incompatible types: null ** 2"),
    ("-null", "[negation] incompatible types: -null"),
  ] {
    let lenient = ok(json!({"op": "evaluate", "expression": expression}));
    assert_eq!(lenient["result"]["display"], "null", "{lenient}");
    assert_eq!(lenient["result"]["nullReason"], message, "{lenient}");
    assert_eq!(lenient["diagnostics"]["count"], 1, "{lenient}");
    assert_eq!(lenient["diagnostics"]["messages"], json!([message]), "{lenient}");
    let strict = req(json!({"op": "evaluate", "expression": expression, "errorMode": "strict"}));
    assert_eq!(strict["ok"], false, "{strict}");
    assert_eq!(strict["error"]["message"], format!("strict error mode: {message}"), "{strict}");
  }
  // B-FEEL (§11.9-§11.11) converts the null operand and gives a value; the FEEL error behind it
  // stays listed
  for (expression, display, message) in [
    ("1 + null", "1", "[addition] incompatible types: 1 + null"),
    ("null - 6", "-6", "[subtraction] incompatible types: null - 6"),
    ("null * 2", "0", "[multiplication] incompatible types: null * 2"),
    ("null / 22", "0", "[division] incompatible types: null / 22"),
    ("2 ** null", "1", "[exponentiation] incompatible types: 2 ** null"),
  ] {
    let b_feel = ok(json!({"op": "evaluate", "expression": expression, "language": "b-feel"}));
    assert_eq!(b_feel["result"]["display"], display, "{b_feel}");
    assert_eq!(b_feel["diagnostics"]["messages"], json!([message]), "{b_feel}");
  }
}

#[test]
fn a_unary_comparison_compares_with_the_value_of_its_path_endpoint() {
  // §10.3.2.7: "Endpoints can be expressions"; `< d.year` compares with the year of d
  let context = "{d: date(\"2020-06-15\"), r: [1..10]}";
  for (expression, input) in [("< d.year", 2000), ("= d.year", 2020), ("> r.start", 20)] {
    let response = ok(json!({"op": "evaluate", "dialect": "unaryTests", "expression": expression, "input": input, "contextFormat": "feel", "context": context}));
    assert_eq!(response["result"]["display"], "true", "{response}");
    assert_eq!(response["diagnostics"]["count"], 0, "{response}");
  }
}

#[test]
fn zoned_date_time_display() {
  // DMN 1.6 Table 48 (DMN16-68): ISO 8601 with the offset, the IANA zone in brackets
  assert_eq!(display(r#"@"2007-12-03T10:15:30@Europe/Paris""#), r#""2007-12-03T10:15:30+01:00[Europe/Paris]""#);
  assert_eq!(display(r#"@"2007-07-03T10:15:30@America/New_York""#), r#""2007-07-03T10:15:30-04:00[America/New_York]""#);
  // offsets and local values are unchanged
  assert_eq!(display(r#"@"2007-12-03T10:15:30+01:00""#), r#""2007-12-03T10:15:30+01:00""#);
  assert_eq!(display(r#"@"2007-12-03T10:15:30""#), r#""2007-12-03T10:15:30""#);
}

#[test]
fn zoned_date_time_display_keeps_the_seconds_of_the_offset() {
  // the offset of the zone at that date and time, as the zone database resolves it: local mean
  // time before the standard zones has seconds
  assert_eq!(display(r#"@"1900-01-01T12:00:00@Europe/Paris""#), r#""1900-01-01T12:00:00+00:09:21[Europe/Paris]""#);
  assert_eq!(display(r#"@"1880-01-01T12:00:00@America/New_York""#), r#""1880-01-01T12:00:00-04:56:02[America/New_York]""#);
  assert_eq!(display(r#"@"2020-01-01T12:00:00@Asia/Kolkata""#), r#""2020-01-01T12:00:00+05:30[Asia/Kolkata]""#);
}

/// `maxSteps` is a budget of steps for one evaluation: function calls and the items of loops and
/// filters. The step after the last one stops the evaluation.
#[test]
fn a_step_budget_stops_an_evaluation_that_takes_more_steps() {
  let stopped = |steps: u64| format!("evaluation took more than {steps} steps; evaluation stopped");
  let with = |expression: &str, steps: u64| req(json!({"op": "evaluate", "expression": expression, "maxSteps": steps}));
  for (expression, few, enough, display) in [
    ("count(for i in 1..1000 return i)", 100, 100_000, "1000"),
    ("count([1, 2, 3, 4, 5][item > 0])", 3, 100, "5"),
    ("some i in [1, 2, 3, 4, 5] satisfies i > 4", 3, 100, "true"),
    ("{f: function(n) if n = 0 then 0 else f(n - 1), r: f(50)}.r", 10, 1_000, "0"),
    ("sort([3, 1, 2], function(a, b) a < b)[1]", 1, 100, "1"),
    ("abs(abs(abs(-1)))", 2, 3, "1"),
  ] {
    assert_error(&with(expression, few), "evaluation", &stopped(few));
    let response = with(expression, enough);
    assert_eq!(response["result"]["display"], display, "{expression}: {response}");
    // without the field the evaluation has no budget
    assert_eq!(expr(expression)["display"], display, "{expression}");
  }
  // the budget is exact: a loop over 10 numbers visits its level 11 times and calls one function
  let counted = "count(for i in 1..10 return i)";
  assert_error(&with(counted, 11), "evaluation", &stopped(11));
  assert_eq!(with(counted, 12)["result"]["display"], "10");
  // in strict mode the abort is the error, as for every limit
  let strict = req(json!({"op": "evaluate", "expression": counted, "maxSteps": 5, "errorMode": "strict"}));
  assert_error(&strict, "evaluation", &stopped(5));
  // the budget of one request does not reach the next
  assert_eq!(expr(counted)["display"], "10");
}

#[test]
fn a_step_budget_is_a_positive_integer() {
  for (value, shown) in [
    (json!(0), "0"),
    (json!(-1), "-1"),
    (json!(1.5), "1.5"),
    (json!("10"), "\"10\""),
    (json!(true), "true"),
    (json!([1]), "[1]"),
  ] {
    let response = req(json!({"op": "evaluate", "expression": "1", "maxSteps": value}));
    assert_error(&response, "request", &format!("invalid 'maxSteps' {shown} (expected a positive integer)"));
  }
  // null is no budget; the field is read after `errorMode` and before the expression
  assert_eq!(req(json!({"op": "evaluate", "expression": "1", "maxSteps": null}))["result"]["display"], "1");
  let response = req(json!({"op": "evaluate", "errorMode": "x", "maxSteps": 0}));
  assert_error(&response, "request", "invalid 'errorMode' \"x\" (expected \"lenient\" or \"strict\")");
  assert_error(
    &req(json!({"op": "evaluate", "maxSteps": 0})),
    "request",
    "invalid 'maxSteps' 0 (expected a positive integer)",
  );
  // only an evaluation has a budget
  assert_eq!(req(json!({"op": "parse", "expression": "1", "maxSteps": 0}))["ok"], true);
}

/// The JSON of `context` and `input` is read into values straight from the request text. A
/// request answers the same when its text makes the module read it as a tree of JSON values
/// first, which a top-level key written with an escape does: values, warnings and their order,
/// errors and their texts.
#[test]
fn context_and_input_read_from_the_text_answer_as_read_through_a_tree() {
  let long = "1234567890123456789012345678901234567890";
  let requests = [
    // keys in any order, twice, with outer spaces, with escapes
    r#"{"op":"evaluate","expression":"[a, b, c]","context":{"c":3,"a":1,"b":2}}"#.to_string(),
    r#"{"op":"evaluate","expression":"[a, b]","context":{"b":1,"a":2,"b":3,"a":4}}"#.to_string(),
    r#"{"op":"evaluate","expression":"[a, b]","context":{"a ":1,"a":2," b":3,"b":4,"\ta\n":5}}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"\u0061":1}}"#.to_string(),
    r#"{"op":"evaluate","expression":"get entries(c)","context":{"c":{"k\"q":"\ud83d\ude00\u00e9\/\\\n","é":[true,false,null,{}],"":1}}}"#.to_string(),
    r#"{"op":"evaluate","expression":"x","context":{"x":{"":1}}}"#.to_string(),
    r#"{"op":"evaluate","expression":"x","context":{"  ":1}}"#.to_string(),
    // warnings, in the order of the keys, and not of a value that a later entry replaces
    format!(r#"{{"op":"evaluate","expression":"[z, a, m.q]","context":{{"z":{long},"a":{long}.5,"m":{{"q":1e-6200,"b":[{long}]}}}}}}"#),
    format!(r#"{{"op":"evaluate","expression":"a","context":{{"a":{long},"b":{long},"a":1}}}}"#),
    format!(r#"{{"op":"evaluate","expression":"a","context":{{"b":{long},"a":1e999999999,"a":2}}}}"#),
    // errors with their paths
    r#"{"op":"evaluate","expression":"a","context":{"b":{"list":[1,{"n":1e999999999}]},"a":1e999999999}}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"z":1e999999999,"a":{"":1}}}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":[1,2]}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"a":1},"contextFormat":"feel"}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"a":1},"contextFormat":"xml"}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":"{a: 1}"}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":null}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"a":1},"context":"{a: 2}","contextFormat":"feel"}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":"{a: 2}","context":{"a":1}}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"a":{"$serde_json::private::Number":"42"}}}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","context":{"a":{"b":1,"$serde_json::private::Number":"42"}}}"#.to_string(),
    // white space, nesting
    "\n {\t\"op\" : \"evaluate\" , \"expression\" : \"count(a) + b.c[2]\" ,\r\n \"context\" : { \"a\" : [ ] , \"b\" : { \"c\" : [ 1 , 2 ] } } } \n".to_string(),
    format!(r#"{{"op":"evaluate","expression":"count(a)","context":{{"a":{}1{}}}}}"#, "[".repeat(125), "]".repeat(125)),
    // the input of unary tests
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"> 5","input":7}"#.to_string(),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"? = [1, 2]","input":[1,2]}"#.to_string(),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"?.b = 2","input":{"b":2,"a":1,"b":2}}"#.to_string(),
    format!(r#"{{"op":"evaluate","dialect":"unaryTests","expression":"count(?) = 2","input":[{long},{{"z":{long},"a":{long}}}],"context":{{"k":{long}}}}}"#),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"> 5","input":{"":1}}"#.to_string(),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"> 5","input":[1e999999999]}"#.to_string(),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"> 5","input":[1],"input":9}"#.to_string(),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"> 5","context":{"?":7}}"#.to_string(),
    r#"{"op":"evaluate","expression":"1","input":[1e999999999]}"#.to_string(),
    // parse, and errors of other fields before the context is read
    r#"{"op":"parse","expression":"unit price + 1","context":{"unit price":1}}"#.to_string(),
    r#"{"op":"parse","expression":"a","context":{"":1}}"#.to_string(),
    r#"{"op":"evaluate","context":{"":1}}"#.to_string(),
    r#"{"op":"evaluate","expression":"a","errorMode":"x","context":{"":1}}"#.to_string(),
    r#"{"op":"info","context":{"":1}}"#.to_string(),
    r#"{"context":{"a":1}}"#.to_string(),
  ];
  let answer = |text: &str| {
    let mut response = raw(text);
    response.as_object_mut().unwrap().remove("elapsedMicros");
    response
  };
  let (mut read, mut failed) = (0, 0);
  for text in requests {
    let through_a_tree = text.replace("\"context\"", "\"\\u0063ontext\"").replace("\"input\"", "\"\\u0069nput\"");
    assert_ne!(text, through_a_tree, "{text}");
    let response = answer(&text);
    assert_eq!(answer(&through_a_tree), response, "{text}");
    if response["ok"] == true {
      read += 1;
    } else {
      failed += 1;
    }
  }
  assert!(read >= 15 && failed >= 10, "{read} answered, {failed} failed");
  // an object that serde_json refuses is refused as before (the text of the error has a column,
  // so the request cannot be compared with a longer spelling of itself)
  assert_error(
    &raw(r#"{"op":"evaluate","expression":"a","context":{"a":{"$serde_json::private::Number":true}}}"#),
    "request",
    "malformed request JSON: invalid type: boolean `true`, expected string containing a number at line 1 column 85",
  );
  // one level deeper than the deepest JSON that is read
  let too_deep = raw(&format!(
    r#"{{"op":"evaluate","expression":"count(a)","context":{{"a":{}1{}}}}}"#,
    "[".repeat(126),
    "]".repeat(126)
  ));
  assert_error(
    &too_deep,
    "context",
    "context or input JSON is nested too deeply (at most about 125 levels): recursion limit exceeded at line 1 column 182",
  );
  // the warnings come in the order of the keys
  let response = raw(&format!(r#"{{"op":"evaluate","expression":"1","context":{{"z":{long},"a":{long}}}}}"#));
  let warnings: Vec<&str> = response["warnings"].as_array().unwrap().iter().map(|w| w["message"].as_str().unwrap()).collect();
  assert!(warnings.len() == 2 && warnings[0].starts_with("a: ") && warnings[1].starts_with("z: "), "{response}");
}

/// The text of a response is what `serde_json` writes for the response as a tree: the keys of
/// every object in alphabetical order, no white space, its escapes. The response of an
/// evaluation is written without that tree, so its text is read back and written again here.
#[test]
fn response_text_is_the_text_of_its_tree() {
  let long = "1234567890123456789012345678901234567890";
  let requests = [
    r#"{"op":"evaluate","expression":"1 + 2"}"#.to_string(),
    r#"{"op":"evaluate","expression":"missing + 1"}"#.to_string(),
    r#"{"op":"evaluate","expression":"null"}"#.to_string(),
    r#"{"op":"evaluate","expression":"[1, \"a\\n\\\"b\\\"\", {\"k \\\\ é\": date(\"2026-01-31\")}, [1..2], abs]"}"#.to_string(),
    r#"{"op":"evaluate","expression":"for i in 1..50 return {n: i, t: string(i) + \"\\t\"}"}"#.to_string(),
    format!(r#"{{"op":"evaluate","expression":"[a, b + 1, \"x\" + 1]","context":{{"a":{long},"b":"text \"quoted\"\n"}}}}"#),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"> 5"}"#.to_string(),
    r#"{"op":"evaluate","dialect":"unaryTests","expression":"true","input":[1,{"a":null}]}"#.to_string(),
    r#"{"op":"evaluate","expression":"1 +"}"#.to_string(),
    r#"{"op":"evaluate","expression":"1 + \"a\"","errorMode":"strict"}"#.to_string(),
    r#"{"op":"parse","expression":"a + 1"}"#.to_string(),
    r#"{"op":"info"}"#.to_string(),
    "not json".to_string(),
  ];
  for request in requests {
    let text = feel_wasm::call(&request);
    let tree: Json = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{request}: {e}: {text}"));
    assert_eq!(serde_json::to_string(&tree).unwrap(), text, "{request}");
  }
}

/// `evaluateAll` reads the context once and answers each entry of `evaluations` as an `evaluate`
/// request with that context answers it: the same result, diagnostics and error. The warnings
/// of reading the context are those of the request, so the warnings of the `evaluate` response
/// are `contextWarnings` followed by the warnings of the entry.
#[test]
fn evaluate_all_answers_each_evaluation_as_evaluate_does() {
  let long = "1234567890123456789012345678901234567890";
  let contexts = [
    json!({"a": 1, "unit price": 2.5, "items": [{"n": 1}, {"n": 2}, {"n": 3}], "s": "text"}),
    serde_json::from_str::<Json>(&format!(r#"{{"big": {long}, "a": 7, "?": 9}}"#)).unwrap(),
    json!({}),
  ];
  let evaluations = [
    json!({"expression": "a + 1"}),
    json!({"expression": "unit price * count(items)"}),
    json!({"expression": "for i in items return i.n * a"}),
    json!({"expression": "sum(items[n > 1].n)"}),
    json!({"expression": "missing + 1"}),
    json!({"expression": "missing + 1", "errorMode": "strict"}),
    json!({"expression": "1 +"}),
    json!({"expression": "count(for i in 1..100 return i)", "maxSteps": 10}),
    json!({"expression": "count(for i in 1..100 return i)", "maxSteps": 1000}),
    json!({"expression": "> 5", "dialect": "unaryTests", "input": 7}),
    json!({"expression": "> 5", "dialect": "unaryTests"}),
    json!({"expression": "? = {\"k\": [1, 2]}", "dialect": "unaryTests", "input": {"k": [1, 2]}}),
    json!({"expression": "> 5", "dialect": "unaryTests", "input": {"": 1}}),
    json!({"expression": "a", "dialect": "xml"}),
    json!({"expression": "a", "errorMode": "soft"}),
    json!({"expression": "a", "maxSteps": 0}),
    json!({"expression": 5}),
    json!({}),
    json!({"expression": "{a: 100, r: a + 1}.r + a"}),
    json!({"expression": "a + 1"}),
  ];
  let without_time = |mut response: Json| {
    response.as_object_mut().unwrap().remove("elapsedMicros");
    response
  };
  for context in contexts {
    let all = req(json!({"op": "evaluateAll", "context": context, "evaluations": evaluations}));
    assert_eq!(all["ok"], true, "{all}");
    assert!(all["elapsedMicros"].is_u64(), "{all}");
    let context_warnings = all["contextWarnings"].as_array().unwrap().clone();
    let results = all["results"].as_array().unwrap();
    assert_eq!(evaluations.len(), results.len(), "{all}");
    for (evaluation, result) in evaluations.iter().zip(results) {
      let mut single = evaluation.as_object().unwrap().clone();
      single.insert("op".to_string(), json!("evaluate"));
      single.insert("context".to_string(), context.clone());
      let mut expected = without_time(req(Json::Object(single)));
      let mut result = without_time(result.clone());
      if expected["ok"] == true {
        // the warnings of the context stand first in the response of `evaluate`
        let warnings = expected["warnings"].as_array().unwrap().clone();
        assert!(warnings.starts_with(&context_warnings), "{evaluation}: {expected}");
        expected["warnings"] = Json::Array(warnings[context_warnings.len()..].to_vec());
        assert!(result["elapsedMicros"].is_null() && result["warnings"].is_array(), "{result}");
      }
      result.as_object_mut().unwrap().remove("elapsedMicros");
      assert_eq!(expected, result, "{evaluation} in {context}");
    }
  }
}

/// The request of `evaluateAll`: its own failures fail the request, an entry that is no object
/// is an error in its place, the text of the response is that of its tree, and the tested
/// value of one entry is not seen by the next.
#[test]
fn evaluate_all_request_and_response() {
  assert_error(&req(json!({"op": "evaluateAll"})), "request", "missing 'evaluations'");
  assert_error(&req(json!({"op": "evaluateAll", "evaluations": {}})), "request", "'evaluations' must be an array");
  assert_error(
    &req(json!({"op": "evaluateAll", "evaluations": [], "context": {"": 1}})),
    "context",
    "context: empty context key",
  );
  assert_eq!(error_kind(&req(json!({"op": "evaluateAll", "evaluations": [], "language": "x"}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluateAll", "evaluations": [], "tzOffsetMinutes": 5000}))), "request");
  assert_error(
    &req(json!({"op": "nothing"})),
    "request",
    "unknown op 'nothing' (expected 'evaluate', 'evaluateAll', 'parse' or 'info')",
  );
  let empty = req(json!({"op": "evaluateAll", "evaluations": []}));
  assert_eq!(
    (&empty["ok"], &empty["results"], &empty["contextWarnings"]),
    (&json!(true), &json!([]), &json!([])),
    "{empty}"
  );
  let request = json!({"op": "evaluateAll", "context": {"a": 1}, "language": "b-feel", "evaluations": [
    5,
    {"expression": "= 7", "dialect": "unaryTests", "input": 7},
    {"expression": "?"},
    {"expression": "= 7", "dialect": "unaryTests"},
    {"expression": "1 + \"a\\n\\\"b\\\"\""},
    {"expression": "a + 1"},
  ]});
  let text = feel_wasm::call(&request.to_string());
  let response: Json = serde_json::from_str(&text).unwrap();
  assert_eq!(serde_json::to_string(&response).unwrap(), text);
  let results = response["results"].as_array().unwrap();
  assert_error(&results[0], "request", "'evaluations[0]' must be an object");
  assert_eq!(results[1]["result"]["display"], "true", "{response}");
  // `?` of the entry before is gone
  assert_eq!(results[2]["result"]["isNull"], true, "{response}");
  assert_eq!(results[3]["result"]["display"], "false", "{response}");
  assert!(results[3]["warnings"][0]["message"].as_str().unwrap().starts_with("no input value given"), "{response}");
  // an `id` stands in the answer of its entry, a result or an error, and only there
  let text = feel_wasm::call(
    &json!({"op": "evaluateAll", "evaluations": [
      {"id": "first", "expression": "1"},
      {"id": 2, "expression": "1 +"},
      {"expression": "3"},
      {"id": "q\"\n", "expression": "4", "errorMode": "x"},
      {"id": [1], "expression": "5"},
      {"id": null, "expression": "6"},
    ]})
    .to_string(),
  );
  let with_ids: Json = serde_json::from_str(&text).unwrap();
  assert_eq!(serde_json::to_string(&with_ids).unwrap(), text);
  let ids: Vec<&Json> = with_ids["results"].as_array().unwrap().iter().map(|result| &result["id"]).collect();
  assert_eq!(ids, [&json!("first"), &json!(2), &Json::Null, &json!("q\"\n"), &Json::Null, &Json::Null], "{with_ids}");
  assert_eq!(with_ids["results"][0]["result"]["display"], "1", "{with_ids}");
  assert_eq!(error_kind(&with_ids["results"][1]), "syntax", "{with_ids}");
  assert_eq!(error_kind(&with_ids["results"][3]), "request", "{with_ids}");
  assert_error(&with_ids["results"][4], "request", "'id' must be a string or a number, got [1]");
  assert_eq!(with_ids["results"][5]["result"]["display"], "6", "{with_ids}");
  // B-FEEL for every entry: a number is added to a string as text
  assert_eq!(results[4]["result"]["type"], "string", "{response}");
  assert_eq!(results[5]["result"]["display"], "2", "{response}");
}
