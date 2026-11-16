//! The backends harness: the requests of the committed sources, each sent through the evaluator of
//! closures and through the flat evaluator in one process, natively, and their full responses
//! compared, as a host sees them. A request that the flat evaluator does not lower yet is counted
//! by the kind of the refusal and of its node instead of compared.
//!
//! `backends_agree` is the group tier's step (`cargo test -p rheon-feel-wasm --test backends --
//! --ignored`); `RHEON_PARSER_CORPUS` adds the FEEL corpus and `RHEON_FULL_TESTS` the requests of
//! the release tier and the full product of edge operands.

#[path = "corpus/product.rs"]
mod product;

#[path = "backends/harness.rs"]
mod harness;

use harness::{committed, compare};
use rheon_feel_evaluator::testing::Backend;

/// Runs `body` on a thread with the stack that the deepest requests need natively.
fn with_big_stack(body: impl FnOnce() + Send + 'static) {
  if let Err(panic) = std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(body).unwrap().join() {
    std::panic::resume_unwind(panic);
  }
}

#[test]
#[ignore = "the group tier runs it: cargo test -p rheon-feel-wasm --test backends -- --ignored"]
fn backends_agree() {
  with_big_stack(|| {
    let requests = committed(product::expressions(std::env::var_os("RHEON_FULL_TESTS").is_some()));
    let report = compare(&requests, Backend::Old, Backend::FlatOnly);
    println!("{report}");
    assert!(report.differences.is_empty(), "{report}");
  });
}

/// Requests whose answers are booleans and others.
const SAMPLE: [&str; 5] = [
  r#"{"op": "evaluate", "expression": "1 < 2"}"#,
  r#"{"op": "evaluate", "expression": "\"a\" = \"b\""}"#,
  r#"{"op": "evaluate", "expression": "1 + 2"}"#,
  r#"{"op": "evaluate", "dialect": "unaryTests", "expression": "> 5", "input": 7}"#,
  r#"{"op": "evaluate", "expression": "x"}"#,
];

#[test]
fn harness_detects_a_difference() {
  let requests: Vec<String> = SAMPLE.iter().map(|request| request.to_string()).collect();
  // two equal evaluators: every request and its parse twin compared, no difference
  let same = compare(&requests, Backend::Old, Backend::Flat);
  assert_eq!((10, 10, 0, 0), (same.requests, same.compared, same.differences.len(), same.refusals()), "{same}");
  // an evaluator that negates boolean results: the three boolean answers differ
  let mutant = compare(&requests, Backend::Old, Backend::Mutant);
  assert_eq!(3, mutant.differences.len(), "{mutant}");
  assert_eq!(r#"{"op": "evaluate", "expression": "1 < 2"}"#, mutant.differences[0].request);
  // the flat evaluator alone refuses what it does not lower, and refusals are not compared
  let flat = compare(&requests, Backend::Old, Backend::FlatOnly);
  assert_eq!((10, 5, 0), (flat.requests, flat.compared, flat.differences.len()), "{flat}");
  assert_eq!(5, flat.refusals(), "{flat}");
}
