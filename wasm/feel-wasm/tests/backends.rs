//! The backends harness: the requests of the committed sources, each sent through the evaluator of
//! closures and through the flat evaluator in one process, natively, and their full responses
//! compared, as a host sees them. A request that the flat evaluator does not lower yet is counted
//! by the kind of the refusal and of its node instead of compared.
//!
//! Random requests follow the committed ones (`backends/gen.rs`): 2,000 by default, 50,000 with
//! `RHEON_FULL_TESTS`, or the one of `RHEON_BACKENDS_SEED`; a random request whose responses differ
//! is shrunk and reported with its seed.
//!
//! `backends_agree` is the group tier's step (`cargo test -p rheon-feel-wasm --test backends --
//! --ignored`); `RHEON_PARSER_CORPUS` adds the FEEL corpus and `RHEON_FULL_TESTS` the requests of
//! the release tier and the full product of edge operands.

#[path = "corpus/product.rs"]
mod product;

#[path = "backends/gen.rs"]
mod generator;
#[path = "backends/harness.rs"]
mod harness;

use generator::{CONSTRUCTS, generate, seeds};
use harness::{committed, compare};
use rheon_feel_evaluator::testing::Backend;
use std::collections::BTreeMap;

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
    let full = std::env::var_os("RHEON_FULL_TESTS").is_some();
    let generated: Vec<_> = seeds(if full { 50_000 } else { 2_000 }).into_iter().map(generate).collect();
    let requests: Vec<String> = generated.iter().map(|generated| generated.request()).collect();
    let report = compare(&requests, Backend::Old, Backend::FlatOnly);
    println!("random {report}");
    let differs = |request: &str| !compare(&[request.to_string()], Backend::Old, Backend::FlatOnly).differences.is_empty();
    for generated in generated.iter().filter(|_| !report.differences.is_empty()) {
      if differs(&generated.request()) {
        let smallest = generated.shrink(differs);
        panic!(
          "random request of seed {} (RHEON_BACKENDS_SEED={}) differs; shrunk: {}\n{report}",
          generated.seed,
          generated.seed,
          smallest.request()
        );
      }
    }
    // a difference that no request shows alone depends on the requests sent before it
    if let Some(first) = report.differences.first() {
      panic!("random request differs only after the requests before it, not alone: {}\n{report}", first.request);
    }
  });
}

#[test]
fn random_requests_are_deterministic() {
  for seed in seeds(100) {
    assert_eq!(generate(seed), generate(seed));
  }
  let texts: std::collections::BTreeSet<String> = seeds(100).into_iter().map(|seed| generate(seed).request()).collect();
  assert!(texts.len() > 90, "{} distinct requests of 100", texts.len());
}

#[test]
fn random_requests_mostly_parse() {
  let requests: Vec<String> = seeds(2_000).into_iter().map(|seed| generate(seed).request()).collect();
  let parsed = requests
    .iter()
    .filter(|request| {
      let mut parse: serde_json::Value = serde_json::from_str(request).unwrap();
      parse["op"] = "parse".into();
      let response: serde_json::Value = serde_json::from_str(&feel_wasm::call(&parse.to_string())).unwrap();
      response["ok"] == true
    })
    .count();
  assert!(parsed * 100 >= requests.len() * 95, "{parsed} of {} random requests parse", requests.len());
}

#[test]
fn random_requests_cover_every_construct() {
  let mut histogram = BTreeMap::new();
  for seed in seeds(2_000) {
    generate(seed).count(&mut histogram);
  }
  let missing: Vec<&str> = CONSTRUCTS.iter().copied().filter(|construct| !histogram.contains_key(construct)).collect();
  assert!(missing.is_empty(), "constructs never generated: {missing:?}; histogram: {histogram:?}");
}

#[test]
fn shrinking_keeps_a_difference() {
  let differs = |request: &str| !compare(&[request.to_string()], Backend::Old, Backend::Mutant).differences.is_empty();
  let generated = seeds(500)
    .into_iter()
    .map(generate)
    .find(|generated| differs(&generated.request()) && generated.request().len() > 60)
    .expect("a random request that the mutant answers differently");
  let smallest = generated.shrink(differs);
  assert!(differs(&smallest.request()), "{}", smallest.request());
  assert!(smallest.request().len() < generated.request().len(), "{} from {}", smallest.request(), generated.request());
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
