mod constructs;
mod host;
mod machine;
mod names;

use crate::Dialect;
use crate::flat::lower::lower;
use crate::flat::resolve::resolve;
use rheon_feel::diagnostics::{self, Diagnostics};
use rheon_feel::{Evaluator, FeelScope};
use rheon_feel_parser::AstNode;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The text of the value and the diagnostics of one evaluation of `evaluator`, run as `prepare`
/// runs it.
fn outcome(evaluator: Evaluator, scope: &FeelScope) -> (String, Diagnostics) {
  let evaluator = diagnostics::one_evaluation(evaluator);
  let (value, diagnostics) = diagnostics::collect(|| evaluator(scope));
  (format!("{value:?}"), diagnostics)
}

/// The environment variable that, set to `1`, makes the tests write the records of
/// `expected/` from the evaluator of closures instead of comparing with them. Every test of a
/// module must run for the file of the module to be whole.
const RECORD: &str = "RHEON_RECORD_EXPECTED";

/// The committed records of the tests of `module`: for each case three lines (the name of its
/// test with its position in the test and its text, the text of the value, the diagnostics) and
/// an empty one.
fn expected(module: &str) -> &'static str {
  match module {
    "constructs" => include_str!("expected/constructs.txt"),
    "host" => include_str!("expected/host.txt"),
    other => panic!("the tests of {other} have no records"),
  }
}

thread_local! {
  /// The number of cases of the test of this thread so far.
  static CASES: Cell<usize> = const { Cell::new(0) };
}

/// The records written so far, by module, test and position.
static RECORDED: Mutex<BTreeMap<(String, String, usize), String>> = Mutex::new(BTreeMap::new());

/// The module, the name and the position of the next case of the running test, whose thread has
/// the path of the test as its name.
fn next_case() -> (String, String, usize) {
  let thread = std::thread::current();
  let path = thread.name().expect("the thread of a test has its name");
  let mut segments = path.rsplit("::");
  let test = segments.next().unwrap_or_default().to_string();
  let module = segments.next().expect("a test is in a module").to_string();
  let position = CASES.with(|cases| cases.replace(cases.get() + 1)) + 1;
  (module, test, position)
}

/// Evaluates `text` in `dialect` with the input context `input` (a FEEL context, or empty) by the
/// flat evaluator, which must lower it, and asserts that it gives the value and the diagnostics
/// recorded for the case.
#[track_caller]
pub(super) fn agree_in(dialect: Dialect, input: &str, text: &str) {
  agree_text(dialect, &scope(input), text);
}

/// [agree_in] with the scope `scope`.
#[track_caller]
pub(super) fn agree_text(dialect: Dialect, scope: &FeelScope, text: &str) {
  let node = rheon_feel_parser::parse_expression(scope, text, false).unwrap_or_else(|e| panic!("{text}: {e}"));
  agree_case(dialect, scope, &node, &format!("{text:?}"));
}

/// The scope of the input context `input` (a FEEL context, or empty).
pub(super) fn scope(input: &str) -> FeelScope {
  if input.is_empty() { FeelScope::default() } else { crate::tests::te_scope(input) }
}

/// [agree_in] for the expression `node`, a tree built by hand.
#[track_caller]
pub(super) fn agree_node(dialect: Dialect, scope: &FeelScope, node: &AstNode) {
  agree_case(dialect, scope, node, "a tree built by hand");
}

/// [agree_in] for the expression `node`, the next case of the running test, described by `text`.
#[track_caller]
fn agree_case(dialect: Dialect, scope: &FeelScope, node: &AstNode, text: &str) {
  let (module, test, position) = next_case();
  let header = format!("{test} #{position} {text}");
  let resolved = resolve(node);
  let program = lower(resolved, dialect).unwrap_or_else(|refusal| panic!("{node:?}: refused at {:?}", refusal.at));
  let flat: Evaluator = Box::new(move |scope: &FeelScope| program.run(scope));
  let flat = outcome(flat, scope);
  if std::env::var(RECORD).is_ok_and(|value| value == "1") {
    let old = outcome(crate::builders::build_evaluator_with_dialect(node, dialect), scope);
    record(&module, test, position, format!("{header}\n{}\n{:?}\n", old.0, old.1));
    assert_eq!(old, flat, "{node:?}");
    return;
  }
  let lines: Vec<&str> = expected(&module).lines().filter(|line| !line.is_empty()).collect();
  let Some(&[_, value, diagnostics]) = lines.chunks(3).find(|record| record[0] == header) else {
    panic!("no record of `{header}` in expected/{module}.txt");
  };
  assert_eq!((value, diagnostics), (flat.0.as_str(), format!("{:?}", flat.1).as_str()), "{header}");
}

/// Keeps `text` as the record of the case of `test` at `position` and writes the file of
/// `module` with the records kept so far, in the order of tests and positions.
fn record(module: &str, test: String, position: usize, text: String) {
  let mut recorded = RECORDED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  recorded.insert((module.to_string(), test, position), text);
  let file = format!("{}/src/flat/tests/expected/{module}.txt", env!("CARGO_MANIFEST_DIR"));
  let records = recorded.iter().filter(|(key, _)| key.0 == module).map(|(_, record)| record.as_str());
  let text = records.collect::<Vec<_>>().join("\n");
  std::fs::write(&file, text).unwrap_or_else(|e| panic!("{file}: {e}"));
}

/// [agree_in] FEEL without an input.
#[track_caller]
pub(super) fn agree(text: &str) {
  agree_in(Dialect::Feel, "", text);
}
