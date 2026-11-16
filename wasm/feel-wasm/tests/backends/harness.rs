//! The requests of the harness and the comparison of two evaluators on them.

use rheon_feel_evaluator::testing::{Backend, Refusal, take_refusals, with_backend};
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The root of the repository.
fn root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Whether a request calls `now()` or `today()`, whose answers depend on the clock (left out, as
/// the identity check leaves them out).
fn uses_clock(text: &str) -> bool {
  ["now", "today"].iter().any(|name| {
    text.match_indices(name).any(|(at, _)| {
      let before = text[..at].chars().next_back();
      let after = text[at + name.len()..].trim_start();
      !before.is_some_and(|c| c.is_alphanumeric() || c == '_') && after.starts_with('(')
    })
  })
}

/// The requests of a JSON file that holds one request or an array of them.
fn requests_of(path: &Path) -> Vec<Json> {
  let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  match serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display())) {
    Json::Array(requests) => requests,
    request => vec![request],
  }
}

/// The JSON files of a directory, by name.
fn json_files(dir: &Path) -> Vec<PathBuf> {
  let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
    .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
    .map(|entry| entry.unwrap().path())
    .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
    .collect();
  files.sort();
  files
}

/// The request of a ledger entry or of an example of a rule entry.
fn ledger_request(entry: &Json) -> Json {
  match entry.get("request") {
    Some(request) => request.clone(),
    None => serde_json::json!({"op": "evaluate", "expression": entry["expression"]}),
  }
}

/// The requests of the committed sources: the request sets and the ledger of the identity check,
/// the corpus that must never trap and the product of edge operands; with `RHEON_PARSER_CORPUS`
/// the expressions of the FEEL corpus, and with `RHEON_FULL_TESTS` the requests of the release
/// tier and the full product. Each is a request text, as a host sends it.
pub fn committed(product: Vec<String>) -> Vec<String> {
  let root = root();
  let identity = root.join("tools/identity");
  let mut requests: Vec<Json> = vec![];
  for file in json_files(&identity.join("requests")) {
    requests.extend(requests_of(&file));
  }
  for file in json_files(&identity.join("registered")) {
    for entry in requests_of(&file) {
      match entry.get("examples").and_then(Json::as_array) {
        Some(examples) => requests.extend(examples.iter().map(ledger_request)),
        None => requests.push(ledger_request(&entry)),
      }
    }
  }
  requests.extend(requests_of(&root.join("wasm/feel-wasm/tests/corpus/no-trap.json")));
  requests.extend(product.into_iter().map(|expression| serde_json::json!({"op": "evaluate", "expression": expression})));
  if let Some(dir) = std::env::var_os("RHEON_PARSER_CORPUS") {
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).expect("RHEON_PARSER_CORPUS").map(|entry| entry.unwrap().path()).collect();
    files.sort();
    for file in files.iter().filter(|file| file.extension().is_some_and(|extension| extension == "jsonl")) {
      for line in std::fs::read_to_string(file).unwrap().lines().filter(|line| !line.trim().is_empty()) {
        let item: Json = serde_json::from_str(line).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let mut request = serde_json::json!({"op": "evaluate", "expression": item["expr"]});
        if item["ctx"].as_str().is_some_and(|ctx| !ctx.is_empty()) {
          request["contextFormat"] = "feel".into();
          request["context"] = item["ctx"].clone();
        }
        requests.push(request);
      }
    }
  }
  if std::env::var_os("RHEON_FULL_TESTS").is_some() {
    for file in json_files(&identity.join("release/requests")) {
      requests.extend(requests_of(&file));
    }
    for file in json_files(&identity.join("release/registered")) {
      requests.extend(requests_of(&file).iter().map(ledger_request));
    }
  }
  requests.iter().map(Json::to_string).filter(|text| !uses_clock(text)).collect()
}

/// A response without the fields that name the build or the time it took.
fn comparable(response: &str) -> Json {
  let mut response: Json = serde_json::from_str(response).unwrap_or_else(|e| panic!("response is not JSON ({e}): {response}"));
  if let Some(fields) = response.as_object_mut() {
    for field in ["elapsedMicros", "engineCommit", "version"] {
      fields.remove(field);
    }
  }
  response
}

/// Two different responses to one request.
pub struct Difference {
  pub request: String,
  pub first: Json,
  pub second: Json,
}

/// What a comparison of two evaluators found.
#[derive(Default)]
pub struct Report {
  /// The requests sent, the parse twins included.
  pub requests: usize,
  /// The requests that the second evaluator answered itself and that were compared.
  pub compared: usize,
  /// The compared requests whose responses differ.
  pub differences: Vec<Difference>,
  /// The requests that the flat evaluator refused, by the kind of the refusal and of its node.
  pub refused: BTreeMap<&'static str, BTreeMap<String, usize>>,
}

impl Report {
  /// The number of refused requests.
  pub fn refusals(&self) -> usize {
    self.refused.values().flat_map(BTreeMap::values).sum()
  }
}

impl fmt::Display for Report {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let percent = if self.requests == 0 { 0.0 } else { self.compared as f64 * 100.0 / self.requests as f64 };
    write!(
      f,
      "backends: {} requests, {} compared ({percent:.1}%), {} differences, refused:",
      self.requests,
      self.compared,
      self.differences.len()
    )?;
    if self.refused.is_empty() {
      write!(f, " none")?;
    }
    for (kind, nodes) in &self.refused {
      let mut nodes: Vec<(&String, &usize)> = nodes.iter().collect();
      nodes.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
      let every: Vec<String> = nodes.iter().map(|(node, count)| format!("{node} {count}")).collect();
      write!(f, " {kind} {} ({})", nodes.iter().map(|(_, count)| **count).sum::<usize>(), every.join(", "))?;
    }
    if let Some(difference) = self.differences.first() {
      write!(f, "\nfirst difference: {}\n  {}\n  {}", difference.request, difference.first, difference.second)?;
    }
    Ok(())
  }
}

/// The response of `backend` to `request`, and the refusals of the flat evaluator while it
/// answered.
fn answer(backend: Backend, request: &str) -> (String, Vec<Refusal>) {
  take_refusals();
  let response = with_backend(backend, || feel_wasm::call(request));
  (response, take_refusals())
}

/// Sends every request, and the parse twin of every evaluate request, to `first` and to `second`,
/// and compares their responses whole: a request that the flat evaluator refused under `second`
/// is counted, not compared.
pub fn compare(requests: &[String], first: Backend, second: Backend) -> Report {
  let mut report = Report::default();
  for request in requests {
    let mut texts = vec![request.clone()];
    if let Ok(mut twin) = serde_json::from_str::<Json>(request)
      && twin["op"] == "evaluate"
    {
      twin["op"] = "parse".into();
      texts.push(twin.to_string());
    }
    for text in texts {
      report.requests += 1;
      let (expected, _) = answer(first, &text);
      let (actual, refusals) = answer(second, &text);
      if second == Backend::FlatOnly
        && let Some(refusal) = refusals.into_iter().next()
      {
        *report.refused.entry(refusal.kind).or_default().entry(refusal.node).or_default() += 1;
        continue;
      }
      report.compared += 1;
      let (expected, actual) = (comparable(&expected), comparable(&actual));
      if expected != actual {
        report.differences.push(Difference {
          request: text,
          first: expected,
          second: actual,
        });
      }
    }
  }
  report
}
