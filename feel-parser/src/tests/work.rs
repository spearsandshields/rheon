//! Work counters of the lexer and parser (`work!`), per thread: deterministic measures of the
//! work of a parse, for the tests of its complexity.

use std::cell::Cell;
use std::collections::BTreeMap;

/// The units of work that are counted.
#[derive(Clone, Copy)]
pub(crate) enum Work {
  /// Parts of name chains scanned by the lexer.
  Parts,
  /// Candidate names the lexer looks up in the scope.
  Candidates,
  /// Searches of composite names in the scope.
  CompositeSearches,
  /// Indexes of input contexts built.
  IndexBuilds,
}

impl Work {
  /// Every unit, in the order of the discriminants.
  const ALL: [Work; 4] = [Work::Parts, Work::Candidates, Work::CompositeSearches, Work::IndexBuilds];

  /// The name of the unit in the counts returned by [take].
  fn name(self) -> &'static str {
    match self {
      Work::Parts => "parts",
      Work::Candidates => "candidates",
      Work::CompositeSearches => "composite searches",
      Work::IndexBuilds => "index builds",
    }
  }
}

// the counter of a unit is at the index of its discriminant: `Work::ALL` lists the units in this order
const _: () = {
  let mut index = 0;
  while index < Work::ALL.len() {
    assert!(Work::ALL[index] as usize == index);
    index += 1;
  }
};

thread_local! {
  static COUNTS: [Cell<usize>; Work::ALL.len()] = const { [const { Cell::new(0) }; Work::ALL.len()] };
}

/// Adds `n` units of work.
pub(crate) fn count(work: Work, n: usize) {
  COUNTS.with(|counts| counts[work as usize].set(counts[work as usize].get() + n));
}

/// Returns the counts since the last call (without zeros), and resets them.
pub(crate) fn take() -> BTreeMap<&'static str, usize> {
  COUNTS.with(|counts| {
    Work::ALL
      .iter()
      .zip(counts)
      .map(|(work, count)| (work.name(), count.take()))
      .filter(|(_, count)| *count > 0)
      .collect()
  })
}
