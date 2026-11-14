//! Request corpora that must never trap the module (CONTRACT §1: on wasm a Rust panic aborts the
//! instance). Natively a panic is caught and answered as an `internal` error, so the same checks
//! fail there with the request named; under Wasmtime a trap aborts the test binary, and the last
//! line on stderr names the request.
//!
//! `no-trap.json` is a JSON array of module requests, exactly as a host sends them: inputs at the
//! limits of the engine (temporal range edges, regex engine limits, the nesting and call-depth
//! limits, sort predicates). An input that trapped the module is added with its fix and stays as a
//! regression seed. The tests assert only that the
//! module answers; the answers themselves are pinned by the identity check against the previous
//! release (`tools/identity/`), which reads this file as one of its request sources.

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
