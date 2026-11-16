//! The ordering of `sort` (DMN 1.6 §10.3.4.9, Table 81).
//!
//! `precedes` is a user function: the spec states no condition on it, so it may be inconsistent
//! (`function(x, y) true`, `function(x, y) x <= y`). The engine therefore owns its algorithm and
//! never hands the predicate to a library sort that may fail on such a function. For every
//! predicate the result is a permutation of the list, fixed by the list and the answers of the
//! predicate; when the predicate is a strict weak order the result is the stable sort.

/// Lists of at most this many items are sorted by insertion alone; in longer lists, runs that are
/// shorter are extended to this length by binary insertion.
const MIN_RUN: usize = 20;

/// A merge switches from comparing items one by one to searching for whole stretches once one
/// side has provided this many items in a row.
const MIN_GALLOP: usize = 7;

/// Returns the positions `0..len`, ordered by `precedes(a, b)`, which tells whether the item at
/// position `a` precedes the item at position `b`, or `None` when the ordering is to stop: then
/// `order_by` returns `None` at once and calls `precedes` no more.
///
/// - Up to [MIN_RUN] items: insertion, moving each item before its predecessor while it precedes
///   it.
/// - Longer lists are cut into runs: a run in which every item precedes the one before it is
///   reversed, a run in which no item precedes the one before it is kept, and a run shorter than
///   [MIN_RUN] is extended by binary insertion. Then neighbouring runs are merged, level by level
///   and without recursion, taking the right item first only when it precedes the left one; once
///   one side has provided [MIN_GALLOP] items in a row, the merge searches for whole stretches.
pub(super) fn order_by(len: usize, mut precedes: impl FnMut(usize, usize) -> Option<bool>) -> Option<Vec<usize>> {
  let mut order: Vec<usize> = (0..len).collect();
  if len <= MIN_RUN {
    insertion(&mut order, &mut precedes)?;
    return Some(order);
  }
  let mut runs = vec![];
  let mut start = 0;
  while start < len {
    let mut end = start + 1;
    if end < len && precedes(order[end], order[start])? {
      end += 1;
      while end < len && precedes(order[end], order[end - 1])? {
        end += 1;
      }
      order[start..end].reverse();
    } else {
      end = (end + 1).min(len);
      while end < len && !precedes(order[end], order[end - 1])? {
        end += 1;
      }
    }
    let min_end = start.saturating_add(MIN_RUN).min(len);
    if end < min_end {
      binary_insertion(&mut order[start..min_end], end - start, &mut precedes)?;
      end = min_end;
    }
    runs.push((start, end));
    start = end;
  }
  let mut merged = Vec::with_capacity(len);
  while runs.len() > 1 {
    let mut next = Vec::with_capacity(runs.len().div_ceil(2));
    for pair in runs.chunks(2) {
      if let [(start, mid), (_, end)] = *pair {
        merge(&mut order, (start, mid, end), &mut merged, &mut precedes)?;
        next.push((start, end));
      } else {
        next.extend_from_slice(pair);
      }
    }
    runs = next;
  }
  Some(order)
}

/// Sorts `order` by insertion: each item moves before its predecessor while it precedes it.
fn insertion(order: &mut [usize], precedes: &mut impl FnMut(usize, usize) -> Option<bool>) -> Option<()> {
  for i in 1..order.len() {
    let item = order[i];
    let mut j = i;
    while j > 0 && precedes(item, order[j - 1])? {
      order[j] = order[j - 1];
      j -= 1;
    }
    order[j] = item;
  }
  Some(())
}

/// Sorts `order`, whose first `sorted` positions are in order, by inserting each further item
/// before the first item it precedes, found by binary search.
fn binary_insertion(order: &mut [usize], sorted: usize, precedes: &mut impl FnMut(usize, usize) -> Option<bool>) -> Option<()> {
  for i in sorted.max(1)..order.len() {
    let item = order[i];
    let at = bisect(&order[..i], 0, i, |other| precedes(item, other).map(|precedes| !precedes))?;
    order[at..=i].rotate_right(1);
  }
  Some(())
}

/// Merges the neighbouring runs `order[start..mid]` and `order[mid..end]`, using `merged` as the
/// buffer. Runs already in order take one call of `precedes`.
fn merge(order: &mut [usize], (start, mid, end): (usize, usize, usize), merged: &mut Vec<usize>, precedes: &mut impl FnMut(usize, usize) -> Option<bool>) -> Option<()> {
  if !precedes(order[mid], order[mid - 1])? {
    return Some(());
  }
  merged.clear();
  let (mut left, mut right) = (start, mid);
  'merge: loop {
    let (mut left_wins, mut right_wins) = (0, 0);
    while left_wins < MIN_GALLOP && right_wins < MIN_GALLOP {
      if precedes(order[right], order[left])? {
        merged.push(order[right]);
        right += 1;
        right_wins += 1;
        left_wins = 0;
      } else {
        merged.push(order[left]);
        left += 1;
        left_wins += 1;
        right_wins = 0;
      }
      if left == mid || right == end {
        break 'merge;
      }
    }
    loop {
      let first_right = order[right];
      let taken_left = prefix_len(&order[left..mid], |item| precedes(first_right, item).map(|precedes| !precedes))?;
      merged.extend_from_slice(&order[left..left + taken_left]);
      left += taken_left;
      if left == mid {
        break 'merge;
      }
      let first_left = order[left];
      let taken_right = prefix_len(&order[right..end], |item| precedes(item, first_left))?;
      merged.extend_from_slice(&order[right..right + taken_right]);
      right += taken_right;
      if right == end {
        break 'merge;
      }
      if taken_left < MIN_GALLOP && taken_right < MIN_GALLOP {
        break;
      }
    }
  }
  merged.extend_from_slice(&order[left..mid]);
  merged.extend_from_slice(&order[right..end]);
  order[start..end].copy_from_slice(merged);
  Some(())
}

/// Returns the length of the leading stretch of `items` on which `holds` is true, probing
/// positions 0, 2, 6, 14, … and then bisecting the last gap, so a stretch of `k` items takes
/// about `2 log2(k + 1)` calls. When `holds` is true on a prefix and false after it, that is the
/// exact length; otherwise it is some length in `0..=items.len()`. `None` when `holds` is `None`.
fn prefix_len(items: &[usize], mut holds: impl FnMut(usize) -> Option<bool>) -> Option<usize> {
  let (mut low, mut high) = (0, items.len());
  let mut step = 1_usize;
  while low < high {
    let probe = low + step - 1;
    if probe >= high {
      break;
    }
    if holds(items[probe])? {
      low = probe + 1;
      step = step.saturating_mul(2);
    } else {
      high = probe;
    }
  }
  bisect(items, low, high, holds)
}

/// Returns the first position in `low..high` at which `holds` is false, or `high`, by halving
/// the range, given that `holds` is true before `low` and false from `high` on. `None` when
/// `holds` is `None`.
fn bisect(items: &[usize], mut low: usize, mut high: usize, mut holds: impl FnMut(usize) -> Option<bool>) -> Option<usize> {
  while low < high {
    let middle = low + (high - low) / 2;
    if holds(items[middle])? {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  Some(low)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// The items of `values` in the order given by `precedes` on them, and the number of calls.
  fn sorted(values: &[i64], precedes: impl Fn(i64, i64) -> bool) -> (Vec<i64>, usize) {
    let mut calls = 0;
    let order = order_by(values.len(), |a, b| {
      calls += 1;
      Some(precedes(values[a], values[b]))
    })
    .expect("the predicate never stops the ordering");
    (order.into_iter().map(|position| values[position]).collect(), calls)
  }

  /// A pseudo-random sequence (xorshift), the same on every run.
  fn pseudo_random(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed | 1;
    move || {
      state ^= state << 13;
      state ^= state >> 7;
      state ^= state << 17;
      state
    }
  }

  #[test]
  fn a_strict_order_gives_the_stable_sort() {
    for len in [0, 1, 2, 19, 20, 21, 33, 64, 100, 1000, 5000] {
      let mut next = pseudo_random(len as u64);
      let values: Vec<i64> = (0..len).map(|_| (next() % 50) as i64 * 10_000).collect();
      // the position is kept in the low digits, so stability is visible
      let values: Vec<i64> = values.iter().enumerate().map(|(position, value)| value + position as i64).collect();
      let mut expected = values.clone();
      expected.sort_by_key(|value| value / 10_000);
      assert_eq!(expected, sorted(&values, |x, y| x / 10_000 < y / 10_000).0, "{len} items");
    }
  }

  #[test]
  fn any_predicate_gives_a_permutation() {
    for len in (0..300).step_by(7).chain([1000, 4096]) {
      let values: Vec<i64> = (0..len).collect();
      for seed in 1..5 {
        let mut next = pseudo_random(seed);
        let mut calls = 0;
        let order = order_by(values.len(), |_, _| {
          calls += 1;
          Some(next().is_multiple_of(2))
        })
        .expect("the predicate never stops the ordering");
        let mut positions = order.clone();
        positions.sort_unstable();
        assert_eq!(values.iter().map(|&v| v as usize).collect::<Vec<_>>(), positions, "{len} items, seed {seed}");
        assert!(calls <= len as usize * len as usize, "{len} items, seed {seed}: {calls} calls");
      }
    }
  }

  #[test]
  fn up_to_twenty_items_are_sorted_by_insertion() {
    let mut next = pseudo_random(7);
    for len in 0..=MIN_RUN {
      let values: Vec<i64> = (0..len).map(|_| (next() % 5) as i64).collect();
      // insertion: each item moves before its predecessor while it precedes it
      let mut expected = values.clone();
      let mut expected_calls = 0;
      for i in 1..expected.len() {
        let mut j = i;
        while j > 0 && {
          expected_calls += 1;
          expected[j] <= expected[j - 1]
        } {
          expected.swap(j, j - 1);
          j -= 1;
        }
      }
      assert_eq!((expected, expected_calls), sorted(&values, |x, y| x <= y), "{len} items");
    }
  }

  #[test]
  fn a_predicate_that_stops_ends_the_ordering_at_once() {
    // short lists are sorted by insertion, long ones by runs and merges; the predicate stops at
    // its first call, at a call during the runs and at a call during the merges
    for (len, stop_at) in [(2, 1), (20, 1), (20, 7), (21, 1), (1000, 1), (1000, 2000), (1000, 8000), (5000, 50_000)] {
      let mut next = pseudo_random(len as u64);
      let values: Vec<u64> = (0..len).map(|_| next() % 1000).collect();
      let mut calls = 0;
      let order = order_by(values.len(), |a, b| {
        calls += 1;
        (calls < stop_at).then(|| values[a] < values[b])
      });
      assert_eq!((None, stop_at), (order, calls), "{len} items, stopped at call {stop_at}");
    }
  }

  #[test]
  fn ordered_and_reversed_lists_take_one_call_per_neighbour() {
    let ascending: Vec<i64> = (0..10_000).collect();
    let descending: Vec<i64> = ascending.iter().rev().copied().collect();
    assert_eq!((ascending.clone(), 9_999), sorted(&ascending, |x, y| x < y));
    assert_eq!((ascending.clone(), 9_999), sorted(&descending, |x, y| x < y));
    assert_eq!((descending.clone(), 9_999), sorted(&descending, |_, _| false));
    assert_eq!((descending, 9_999), sorted(&ascending, |_, _| true));
  }
}
