//! Work counters of the lexer and parser (`work!`), per thread, for the tests.

use std::cell::Cell;
use std::collections::BTreeMap;

/// The units of work that are counted.
#[derive(Clone, Copy)]
pub(crate) enum Work {
  Shifts,
  Reductions,
  NameTokens,
  Parts,
  Candidates,
  Contains,
  CompositeSearches,
  IndexBuilds,
}

const NAMES: [&str; 9] = [
  "shifts",
  "reductions",
  "name tokens",
  "parts",
  "candidates",
  "contains",
  "composite searches",
  "index builds",
  "list shifts",
];

thread_local! {
  static COUNTS: [Cell<usize>; 8] = const { [const { Cell::new(0) }; 8] };
}

/// Adds `n` units of work.
pub(crate) fn count(work: Work, n: usize) {
  COUNTS.with(|counts| counts[work as usize].set(counts[work as usize].get() + n));
}

/// Returns the counts since the last call (without zeros), and resets them.
pub(crate) fn take() -> BTreeMap<&'static str, usize> {
  COUNTS.with(|counts| {
    NAMES
      .iter()
      .zip(counts)
      .map(|(name, count)| (*name, count.take()))
      .filter(|(_, count)| *count > 0)
      .collect()
  })
}
