//! Request corpora that must never trap the module (CONTRACT §1: on wasm a Rust panic aborts the
//! instance). Natively a panic is caught and answered as an `internal` error, so the same checks
//! fail there with the request named; under Wasmtime a trap aborts the test binary, and the last
//! line on stderr names the request.
//!
//! `no-trap.json` is a JSON array of module requests, exactly as a host sends them: inputs at the
//! limits of the engine (temporal range edges, regex engine limits, the nesting and call-depth
//! limits, sort predicates, loops over many items whose calls are refused after an abort). An input that trapped the module is added with its fix and stays as a
//! regression seed. The tests assert only that the
//! module answers; the answers themselves are pinned by the identity check against the previous
//! release (`tools/identity/`), which reads this file as one of its request sources.
//!
//! `edge-operands.json` is a product of expression templates over operands at the engine's edges
//! (see `edge_operand_products_never_trap`); it is the seed set of combinations, where `no-trap.json`
//! holds single requests.

use super::{raw, with_big_stack};
use serde_json::Value as Json;

/// A request after which nothing of the previous one may linger: it makes a function call, so a
/// call-depth or abort state left behind would show.
const PROBE: &str = r#"{"op":"evaluate","expression":"abs(-1) + 1"}"#;

/// Sends `request` and asserts that the module answers it with a response that is not a caught
/// panic, and that the next request is answered normally.
fn assert_answered(request: &str) {
  #[cfg(target_arch = "wasm32")]
  {
    use std::io::Write;
    // written past the test harness's output capture, so it is on stderr when a trap aborts the binary
    let _ = std::io::stderr().write_all(format!("no-trap: {request}\n").as_bytes());
  }
  let response = raw(request);
  assert!(response["ok"].is_boolean(), "no `ok` in the response to {request}: {response}");
  assert_ne!(response["error"]["kind"], "internal", "{request} panicked: {response}");
  let probe = raw(PROBE);
  assert_eq!(probe["result"]["display"], "2", "after {request}, {PROBE} answered {probe}");
}

#[test]
fn corpus_requests_never_trap() {
  with_big_stack(|| {
    let requests: Vec<Json> = serde_json::from_str(include_str!("no-trap.json")).expect("no-trap.json is a JSON array of requests");
    for request in &requests {
      assert_answered(&request.to_string());
    }
  });
}

/// `true` when the product also runs the `binary_full` templates (`RHEON_FULL_TESTS` set). Wasmtime
/// passes no environment variables to the test binary, so the full product runs natively only.
fn full() -> bool {
  std::env::var_os("RHEON_FULL_TESTS").is_some()
}

/// The texts of a JSON array of strings.
fn texts<'a>(product: &'a Json, key: &str) -> Vec<&'a str> {
  product[key]
    .as_array()
    .unwrap_or_else(|| panic!("edge-operands.json has no array `{key}`"))
    .iter()
    .map(|text| text.as_str().expect("a template or an operand is a string"))
    .collect()
}

/// `template` with every `{a}` replaced by `a` and every `{b}` by `b`, in one pass, so an operand
/// is never read as a placeholder.
fn instantiate(template: &str, a: &str, b: &str) -> String {
  let mut expression = String::new();
  let mut rest = template;
  while let Some(start) = rest.find('{') {
    expression.push_str(&rest[..start]);
    rest = &rest[start..];
    if let Some(after) = rest.strip_prefix("{a}") {
      expression.push_str(a);
      rest = after;
    } else if let Some(after) = rest.strip_prefix("{b}") {
      expression.push_str(b);
      rest = after;
    } else {
      expression.push('{');
      rest = &rest[1..];
    }
  }
  expression.push_str(rest);
  expression
}

/// `edge-operands.json` is a deterministic product over operand classes at the engine's edges:
/// numbers at the limits of the 64-bit integers, of 32-bit integers and of decimal128, both
/// duration kinds at the ends of their range and at one unit, dates and date-times at the ends of
/// the supported range and in daylight saving gaps and overlaps, times at the ends of the day,
/// and null, the empty string, list and context. Every `unary` template is applied to every
/// operand and every `binary` template to every ordered pair; the `binary_full` templates are
/// added when the full size runs.
#[test]
fn edge_operand_products_never_trap() {
  with_big_stack(|| {
    let product: Json = serde_json::from_str(include_str!("edge-operands.json")).expect("edge-operands.json is a JSON object");
    let operands = texts(&product, "operands");
    let mut binary = texts(&product, "binary");
    if full() {
      binary.extend(texts(&product, "binary_full"));
    }
    let evaluate = |expression: String| assert_answered(&serde_json::json!({"op": "evaluate", "expression": expression}).to_string());
    for template in texts(&product, "unary") {
      for a in &operands {
        evaluate(instantiate(template, a, ""));
      }
    }
    for template in &binary {
      for a in &operands {
        for b in &operands {
          evaluate(instantiate(template, a, b));
        }
      }
    }
  });
}
