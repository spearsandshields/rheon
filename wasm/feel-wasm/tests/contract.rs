//! Contract tests (docs/CONTRACT.md §2–3). Run natively (`cargo test -p rheon-feel-wasm`) and under
//! wasm32-wasip1 via wasmtime (`source scripts/env.sh && cargo test -p rheon-feel-wasm --target wasm32-wasip1`).

use serde_json::{Value as Json, json};

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

fn error_kind(response: &Json) -> &str {
  assert_eq!(response["ok"], false, "expected failure, got {response}");
  assert!(response["error"]["message"].is_string(), "{response}");
  response["error"]["kind"].as_str().unwrap()
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
  assert_eq!(r["typed"], json!({"simple": {"type": "xsd:decimal", "text": "0.3", "isNil": false}, "components": null, "list": null}));
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
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": "{a: }"}))), "context");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": "[1,2]"}))), "context");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": {"a": 1}}))), "context");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "context": [1, 2]}))), "context");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "context": "{a: 1}"}))), "context");
  // empty / missing contexts are fine
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": "  "}))["display"], "1");
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
  assert_eq!(response["warnings"].as_array().unwrap().len(), 1, "{response}");
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
    "modulo(9e6144, 1e-6176)",
    "modulo(5, 1e-6176)",
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
  // ordinary zeros keep dsntk's formatting
  assert_eq!(display("0.000"), "0.000");
  assert_eq!(display("0 * 1e-10"), "0.000000000");
  assert_eq!(display("0"), "0");
}

/// sublist / substring / insert before / remove / list replace / filters: positions are converted
/// through 64-bit integers and checked with wide arithmetic, so wasm32 gives the same results and
/// null reasons as 64-bit native builds, and huge lengths can not wrap (they used to give wrong
/// strings or trap on wasm32).
#[test]
fn huge_positions_and_lengths_are_null_with_native_reasons() {
  let cases = [
    ("substring(\"abcdef\", 2, 4294967295)", "sublist: invalid range, len = 6, start position = 2, end position = 4294967297"),
    ("substring(\"abcdef\", 4, 4294967293)", "sublist: invalid range, len = 6, start position = 4, end position = 4294967297"),
    ("substring(\"abcdef\", -3, 4294967295)", "sublist: invalid range, len = 6, start position = 4, end position = 4294967299"),
    ("substring(\"abcdef\", 2, 18446744073709551615)", "sublist: invalid range, len = 6, start position = 2, end position = 18446744073709551617"),
    ("substring(\"abc\", 2, 4294967296)", "sublist: invalid range, len = 3, start position = 2, end position = 4294967298"),
    ("substring(\"abc\", -4294967296)", "substring: position is out of range, len = 3, position = -4294967296"),
    ("sublist([1,2,3,4,5,6], 2, 4294967295)", "sublist: invalid range, len = 6, start position = 2, end position = 4294967297"),
    ("sublist([1,2,3,4,5,6], -3, 4294967294)", "sublist: invalid range, len = 6, start position = 4, end position = 4294967298"),
    ("sublist([1,2,3,4,5,6], 2, 18446744073709551615)", "sublist: invalid range, len = 6, start position = 2, end position = 18446744073709551617"),
    ("sublist([1,2,3], 1, 4294967296)", "sublist: invalid range, len = 3, start position = 1, end position = 4294967297"),
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
/// seconds, 40 levels would never finish).
#[test]
fn nested_lists_are_typed_in_linear_time() {
  let deep = format!("{}1{}", "[".repeat(40), "]".repeat(40));
  let started = std::time::Instant::now();
  let r = expr(&deep);
  assert_eq!(r["type"].as_str().unwrap().matches("list<").count(), 40);
  let request = format!(r#"{{"op":"evaluate","expression":"count(x)","context":{{"x":{deep}}}}}"#);
  assert_eq!(eval_ok(serde_json::from_str(&request).unwrap())["display"], "1");
  let request = format!(r#"{{"op":"parse","expression":"x","context":{{"x":{deep}}}}}"#);
  assert_eq!(raw(&request)["ok"], true);
  assert!(started.elapsed().as_secs() < 10, "took {:?}", started.elapsed());
}

/// Deeply nested expressions are rejected before the recursive parser/evaluator code can overflow
/// the stack (a host-dependent trap on wasm), and runaway recursion stops at a fixed call depth.
#[test]
fn nesting_and_recursion_limits() {
  // The limits are sized for the release wasm build in a browser worker; unoptimized native
  // test builds use much bigger stack frames than that, so give this test a big stack.
  #[cfg(not(target_arch = "wasm32"))]
  std::thread::Builder::new()
    .stack_size(512 * 1024 * 1024)
    .spawn(nesting_and_recursion_limits_body)
    .unwrap()
    .join()
    .unwrap();
  #[cfg(target_arch = "wasm32")]
  nesting_and_recursion_limits_body();
}

fn nesting_and_recursion_limits_body() {
  for e in [
    format!("1{}", "+1".repeat(20000)),
    format!("1{}+", "+1".repeat(20000)),
    format!("{}1{}", "[".repeat(5000), "]".repeat(5000)),
    format!("{}1{}", "{a:".repeat(3000), "}".repeat(3000)),
  ] {
    let response = req(json!({"op": "evaluate", "expression": e}));
    assert_eq!(error_kind(&response), "syntax");
    let response = req(json!({"op": "parse", "expression": e}));
    assert_eq!(error_kind(&response), "syntax");
  }
  assert!(req(json!({"op": "evaluate", "expression": format!("1{}", "+1".repeat(20000))}))["error"]["message"].as_str().unwrap().contains("nested too deeply"));
  assert_eq!(display(&format!("1{}", "+1".repeat(250))), "251");
  let response = req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": format!("{}1{}", "{a:".repeat(3000), "}".repeat(3000))}));
  assert_eq!(error_kind(&response), "context");
  // recursion
  let rec = |n: u32| format!("{{f: function(n) if n = 0 then 0 else 1 + f(n - 1), r: f({n})}}.r");
  assert_eq!(display(&rec(150)), "150");
  for e in [rec(5000), "{f: function(n) f(n + 1), r: f(1)}.r".to_string()] {
    let response = req(json!({"op": "evaluate", "expression": e}));
    assert_eq!(error_kind(&response), "evaluation", "{response}");
  }
  let response = req(json!({"op": "evaluate", "expression": "1", "contextFormat": "feel", "context": "{f: function(n) f(n + 1), r: f(1)}"}));
  assert_eq!(error_kind(&response), "context");
  // the limit does not leak into the next request
  assert_eq!(display(&rec(150)), "150");
  // a JSON context nested deeper than the request parser allows is a context error
  let request = format!(r#"{{"op":"evaluate","expression":"1","context":{{"x":{}1{}}}}}"#, "[".repeat(200), "]".repeat(200));
  assert_eq!(error_kind(&raw(&request)), "context");
}

#[test]
fn json_numbers_that_underflow_warn() {
  for (number, fragment) in [("1e-7000", "became 0"), ("1.5e-6176", "was rounded"), ("1.23456789012345678901234567890123456789", "more than 34")] {
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
fn boolean_unary_tests_with_question_mark_warn() {
  for tests in ["? > 5", "? > 5 and ? < 10", "not(? = 1)", "> 1, ? < 3"] {
    let response = req(json!({"op": "evaluate", "dialect": "unaryTests", "expression": tests, "input": 7}));
    assert_eq!(response["ok"], true, "{response}");
    let warnings = response["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{tests}: {response}");
    assert!(warnings[0]["message"].as_str().unwrap().contains("'?'"), "{response}");
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
}

#[test]
fn unknown_names_are_null() {
  let r = expr("unknown variable + 1");
  assert_eq!(r["isNull"], true, "{r}");
}

#[test]
fn request_errors() {
  assert_eq!(error_kind(&raw("not json")), "request");
  assert_eq!(error_kind(&raw("[1]")), "request");
  assert_eq!(error_kind(&raw("{}")), "request");
  assert_eq!(error_kind(&req(json!({"op": "explode"}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate"}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": 1}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "dialect": "boxed"}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "contextFormat": "xml"}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": 5000}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": "UTC"}))), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": 1.5}))), "request");
  // invalid UTF-8 (only possible through bytes: the wasm ABI, or call_bytes)
  let response: Json = serde_json::from_str(&feel_wasm::call_bytes(&[0xff, 0xfe])).unwrap();
  assert_eq!(error_kind(&response), "request");
  #[cfg(target_arch = "wasm32")]
  {
    let response: Json = serde_json::from_str(&abi_call(&[0xff, 0xfe])).unwrap();
    assert_eq!(error_kind(&response), "request");
  }
  // i64::MIN must not slip through the range check (abs() overflow)
  assert_eq!(error_kind(&raw(r#"{"op":"evaluate","expression":"1","tzOffsetMinutes":-9223372036854775808}"#)), "request");
  assert_eq!(error_kind(&req(json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": -1081}))), "request");
  assert_eq!(eval_ok(json!({"op": "evaluate", "expression": "1", "tzOffsetMinutes": -1080}))["display"], "1");
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
  let without = eval_ok(json!({"op": "evaluate", "expression": "string(now())"}))["feel"].as_str().unwrap().trim_matches('"').to_string();
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
  let tz0 = |e: &str| eval_ok(json!({"op": "evaluate", "expression": e, "tzOffsetMinutes": 0}))["display"].as_str().unwrap().to_string();
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
  assert!(reason(r#"time("10:00:00+15:00")"#).starts_with("time: invalid time literal '10:00:00+15:00'"), "{}", reason(r#"time("10:00:00+15:00")"#));
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
fn long_path_chains_parse_in_bounded_time() {
  // the lexer was cubic in the chain length (fork commit f308a34): 1000 steps now take
  // milliseconds and then hit the 300-level nesting limit (CONTRACT §2)
  let expression = format!("{{a:1}}{}", ".a".repeat(1000));
  let started = std::time::Instant::now();
  let response = req(json!({"op": "parse", "expression": expression}));
  assert_eq!(error_kind(&response), "syntax", "{response}");
  assert!(response["error"]["message"].as_str().unwrap().contains("nested too deeply"), "{response}");
  assert!(started.elapsed() < std::time::Duration::from_secs(5), "parse took {:?}", started.elapsed());
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
  assert_eq!(r["nullReason"], "[core::round_up] <FeelNumberError> invalid scale, allowed range is -6111..6176, actual is 6177");
  assert_eq!(display(r#"matches("aA", "(a)\\1", "i")"#), "true");
  assert_eq!(display(r#"matches("ab", "(a)\\1")"#), "false");
  // a pathological pattern hits fancy-regex's backtracking limit and is reported, not hung
  let r = expr(r#"matches("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!", "^(a+)+\\1$")"#);
  assert!(r["isNull"] == true || r["display"] == "false", "{r}");
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

#[test]
fn zoned_date_time_display() {
  // DMN 1.6 Table 48 (DMN16-68): ISO 8601 with the offset, the IANA zone in brackets
  assert_eq!(display(r#"@"2007-12-03T10:15:30@Europe/Paris""#), r#""2007-12-03T10:15:30+01:00[Europe/Paris]""#);
  assert_eq!(display(r#"@"2007-07-03T10:15:30@America/New_York""#), r#""2007-07-03T10:15:30-04:00[America/New_York]""#);
  // offsets and local values are unchanged
  assert_eq!(display(r#"@"2007-12-03T10:15:30+01:00""#), r#""2007-12-03T10:15:30+01:00""#);
  assert_eq!(display(r#"@"2007-12-03T10:15:30""#), r#""2007-12-03T10:15:30""#);
}
