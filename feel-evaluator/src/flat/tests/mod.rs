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

/// The text of the value and the diagnostics of one evaluation of `evaluator`, run as `prepare`
/// runs it.
fn outcome(evaluator: Evaluator, scope: &FeelScope) -> (String, Diagnostics) {
  let evaluator = diagnostics::one_evaluation(evaluator);
  let (value, diagnostics) = diagnostics::collect(|| evaluator(scope));
  (format!("{value:?}"), diagnostics)
}

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
  let node = rheon_feel_parser::parse_expression(scope, text).unwrap_or_else(|e| panic!("{text}: {e}"));
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
  let program = lower(resolved, dialect, None).unwrap_or_else(|malformed| panic!("{node:?}: {}", malformed.message()));
  let flat: Evaluator = Box::new(move |scope: &FeelScope| program.run(scope));
  let flat = outcome(flat, scope);
  // a program that records the value of every node gives the same value and diagnostics: the
  // recording instructions read, and change nothing
  let every_node = |_: &AstNode| Some(0);
  let every_loop = |_: &AstNode| Some(0);
  let tracing = crate::Tracing {
    node: &every_node,
    repeat: &every_loop,
  };
  let traced = lower(resolve(node), dialect, Some(&tracing)).unwrap_or_else(|malformed| panic!("{node:?}: {}", malformed.message()));
  let traced: Evaluator = Box::new(move |scope: &FeelScope| traced.run(scope));
  let plan = crate::Plan {
    wanted: vec![vec![crate::Wanted::default()]],
    repeats: 1,
    limit: 1_000,
  };
  let (traced, trace) = crate::recording(plan, || outcome(traced, scope));
  assert_eq!(
    (&flat.0, format!("{:?}", flat.1)),
    (&traced.0, format!("{:?}", traced.1)),
    "{header}: with the value of every node recorded"
  );
  assert!(trace.records.len() <= 1, "{header}: one number, one record");
  let lines: Vec<&str> = expected(&module).lines().filter(|line| !line.is_empty()).collect();
  let Some(&[_, value, diagnostics]) = lines.chunks(3).find(|record| record[0] == header) else {
    panic!("no record of `{header}` in expected/{module}.txt");
  };
  // after a change of answers that is meant, `RHEON_EXPECTED=report` lists the new records
  if std::env::var_os("RHEON_EXPECTED").is_some() {
    if (value, diagnostics) != (flat.0.as_str(), format!("{:?}", flat.1).as_str()) {
      println!("RECORD\t{module}\t{header}\t{}\t{:?}", flat.0, flat.1);
    }
    return;
  }
  assert_eq!((value, diagnostics), (flat.0.as_str(), format!("{:?}", flat.1).as_str()), "{header}");
}

/// [agree_in] FEEL without an input.
#[track_caller]
pub(super) fn agree(text: &str) {
  agree_in(Dialect::Feel, "", text);
}
