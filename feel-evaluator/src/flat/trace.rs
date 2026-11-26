//! The record of an evaluation: the values of the nodes that a request asks for, kept while a
//! program compiled for that request runs ([super::lower_traced]). The evaluation itself is the
//! one that every other program has: a recording instruction reads a value and changes nothing.
//! A request can also ask for the values on the ways from the nulls to the nodes they are from
//! ([Follow]): they are kept while the same evaluation runs, and no node is evaluated twice.
//!
//! The recorder is of the thread, as the diagnostics of an evaluation are: [recording] starts
//! one around an evaluation and gives what it kept. Outside one, a recording instruction has no
//! effect.

use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::cell::RefCell;

/// What a program is compiled to record: the nodes, the loops and the invocations of a request,
/// by their numbers.
pub struct Tracing<'t> {
  /// The number of a node whose value the request asks for, None for another node.
  pub node: &'t dyn Fn(&AstNode) -> Option<u32>,
  /// The number of a loop or a filter that the request follows (the records of nodes in its
  /// body are of one of its iterations, or it says how many it had), None for another.
  pub repeat: &'t dyn Fn(&AstNode) -> Option<u32>,
  /// The number of an invocation that the request follows (the records of nodes in the body of
  /// the function it invokes are of that invocation), None for another.
  pub call: &'t dyn Fn(&AstNode) -> Option<u32>,
}

/// A step of the way from the root expression to where a node is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
  /// Into an iteration: the number of the loop ([Tracing::repeat]) and the index of the iteration.
  Iteration(u32, usize),
  /// Into the body of the function that an invocation invokes: the number of the invocation
  /// ([Tracing::call]).
  Call(u32),
}

/// A node whose value a request asks for, and when: a node in the body of a loop or in the
/// predicate of a filter is evaluated once for each item, a node in the body of a function once
/// for each invocation, and the request says for which.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wanted {
  /// Where the node is wanted: the steps from the root expression, outermost first. The
  /// iterations before the first invocation are of loops of the root expression, those after an
  /// invocation of loops of the body it invokes; the node is in the body of the last.
  pub within: Vec<Step>,
  /// The node is the body of a loop (its number) and is wanted for each iteration from the
  /// first index up to the second: a record for each.
  pub each: Option<(u32, usize, usize)>,
  /// The node is a loop (its number): its record says how many iterations it had.
  pub repeat: Option<u32>,
}

/// What a request asks an evaluation to record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
  /// The nodes, each at the index that is its number ([crate::Tracing::node]): the ways it is
  /// wanted (a node of a loop's body can be wanted in several of its iterations).
  pub wanted: Vec<Vec<Wanted>>,
  /// The most records that are kept.
  pub limit: usize,
  /// With it the evaluation also keeps, for each null, the values on the way to the node that
  /// the null is from ([Trace::followed]).
  pub follow: Option<Follow>,
}

/// How a node is a part of the node around it, for [Follow].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FollowPart {
  /// The root expression: a part of no node.
  #[default]
  Root,
  /// Evaluated once where the node around it is.
  Same,
  /// Evaluated once for each item that the node around it visits.
  Iteration,
  /// The body of a function, evaluated where the function is invoked.
  Body,
}

/// A node of the text, for [Follow]. The number of a node is its index, and the numbers are in
/// the order of the text: the nodes inside a node (its parts and theirs, but not the body of a
/// function it defines) have the numbers after its own, up to `last`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FollowNode {
  /// The node that it is a part of, None for the root expression and for a body.
  pub parent: Option<u32>,
  pub part: FollowPart,
  /// Whether the null of the node around it can be from this part (the condition of an `if`
  /// is a part that it cannot be from).
  pub chain: bool,
  /// Whether what is kept of its parts stays when it is not null: a context literal, whose
  /// entries a null can be from though the context is none.
  pub keeps: bool,
  /// The last number of the nodes inside it, its own when it has none.
  pub last: u32,
  /// For a name that an entry of a context literal around it gives its value: that entry.
  pub link: Option<u32>,
}

/// What an evaluation keeps to say where its nulls are from. Every node of [Follow::nodes] has
/// its number from [Tracing::node], every loop and filter its own number from
/// [Tracing::repeat], and every invocation its own number from [Tracing::call].
///
/// While the evaluation runs, the value of each node is kept until the node around it has its
/// value. Of a node that is not null nothing inside it stays; of a null node the way stays: its
/// parts, and of the part that the null is from (the body that an invocation ran, else the
/// first null part, else the first null iteration) what was kept inside it. So what is kept is
/// as long as the ways are, not as the evaluation is: an iteration that is not the first null
/// one of its loop leaves nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Follow {
  pub nodes: Vec<FollowNode>,
  /// The most values that are kept at one time: beyond it nothing more is kept, and the trace
  /// says so ([Trace::overflow]).
  pub limit: usize,
}

/// A value kept on the way from a null to the node that it is from.
#[derive(Debug, Clone, PartialEq)]
pub struct Followed {
  /// The number of the node.
  pub target: u32,
  /// Where the node was evaluated: the steps from the root expression, outermost first, with
  /// every iteration and every invocation around it.
  pub within: Vec<Step>,
  pub value: Value,
  /// For a loop or a filter: how many iterations it had.
  pub iterations: Option<usize>,
  /// Whether values inside the node were not kept: its null is from another part of the node
  /// around it, so the values of its own parts are not all here.
  pub bare: bool,
}

/// The value that an evaluation gave for a node that a request asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
  /// The number that the request gave the node.
  pub target: u32,
  /// Which of the ways the node is wanted this record is of: its index.
  pub way: usize,
  pub value: Value,
  /// For a node wanted for each iteration of its loop: the index of the iteration.
  pub iteration: Option<usize>,
  /// For a loop: how many iterations it had (the items it visited).
  pub iterations: Option<usize>,
}

/// Where a loop that has a number is, in one body.
#[derive(Clone, Copy, Default)]
struct Repeat {
  /// The iterations begun since the loop was opened.
  begun: usize,
  /// The index of the iteration that runs, None outside one.
  current: Option<usize>,
  /// Whether an iteration whose value is null is kept for the way of a null ([Follow]).
  kept: bool,
}

/// The root expression, or a body of a function that is being evaluated.
#[derive(Default)]
struct Frame {
  /// The invocation that the request follows and that invoked the body, None for the root
  /// expression and for a body invoked otherwise.
  site: Option<u32>,
  /// The loops with a number that were opened in it, each with where it is.
  repeats: Vec<(u32, Repeat)>,
  /// Where it is evaluated, for the ways of the nulls: an index of [Ways::paths].
  place: u32,
}

impl Frame {
  fn repeat(&self, number: u32) -> Option<&Repeat> {
    self.repeats.iter().find(|(known, _)| *known == number).map(|(_, repeat)| repeat)
  }

  fn repeat_mut(&mut self, number: u32) -> &mut Repeat {
    let index = match self.repeats.iter().position(|(known, _)| *known == number) {
      Some(index) => index,
      None => {
        self.repeats.push((number, Repeat::default()));
        self.repeats.len() - 1
      }
    };
    &mut self.repeats[index].1
  }
}

struct Recorder {
  plan: Plan,
  records: Vec<Recorded>,
  /// The records kept so far, by the node and the iteration: the first evaluation of a node in
  /// an iteration is the one kept.
  seen: std::collections::HashSet<(u32, usize, Option<usize>)>,
  /// The root expression and the bodies being evaluated in it, the one that runs last.
  frames: Vec<Frame>,
  /// The invocation that the request follows and that is about to invoke a function of the text.
  calling: Option<u32>,
  /// Whether a record was not kept for the limit.
  truncated: bool,
  /// What is kept to say where the nulls are from, with a plan that asks for it.
  ways: Option<Ways>,
}

/// A value kept for the way of a null ([Follow]).
struct Kept {
  target: u32,
  /// The body it was evaluated in: the index of its frame.
  depth: usize,
  /// Where it was evaluated: an index of [Ways::paths].
  path: u32,
  value: Value,
  iterations: Option<usize>,
  /// Whether values inside the node were dropped.
  bare: bool,
  /// Whether it stays when the node around it is not null, as long as its body runs and its
  /// iteration is kept: the entry that gives a null name its value, and what is inside it.
  pinned: bool,
  /// Whether it is among the values that stay ([Ways::settled]) already.
  settled: bool,
}

/// The values kept for the ways of the nulls of an evaluation.
struct Ways {
  /// The values of the nodes whose node around them has no value yet, in the order their
  /// evaluations ended: what is inside a node stands before it.
  pending: Vec<Kept>,
  /// The values that stay whatever the nodes around them are: those inside a null node whose
  /// value the request asks for.
  settled: Vec<Kept>,
  /// The places of evaluation, each the place before it and one step more; the first is the
  /// root expression. A place is made when the evaluation is there and a value is kept, or a
  /// body is entered from it; the places that nothing names any more are forgotten
  /// ([Ways::collect]).
  paths: Vec<(u32, Step)>,
  /// The index in `paths` of where the evaluation is now, None since it moved.
  here: Option<u32>,
  /// The bodies that run and were not invoked by an invocation of the text: nothing is kept
  /// inside one.
  untraced: usize,
  /// Whether a value was not kept for the limit.
  overflow: bool,
  /// How many places there are when those that nothing names are next forgotten.
  collect_at: usize,
  /// Whether a value is pinned.
  pins: bool,
  /// The parts of a null node, while what stays of them is decided: where each begins among
  /// the values pending and where its own value is.
  parts: Vec<(usize, usize)>,
  /// Which of the values inside a null node stay.
  stays: Vec<bool>,
}

impl Default for Ways {
  fn default() -> Self {
    Self {
      pending: vec![],
      settled: vec![],
      paths: vec![(0, Step::Call(0))],
      here: None,
      untraced: 0,
      overflow: false,
      collect_at: 4_096,
      pins: false,
      parts: vec![],
      stays: vec![],
    }
  }
}

thread_local! {
  static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// What an evaluation recorded.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trace {
  /// The records, in the order the evaluations of their nodes ended.
  pub records: Vec<Recorded>,
  /// Whether more records were asked for than the limit.
  pub truncated: bool,
  /// With [Plan::follow]: the values on the ways from the nulls to the nodes they are from, a
  /// node inside another before it.
  pub followed: Vec<Followed>,
  /// Whether more values were on those ways than [Follow::limit]: `followed` is then not all.
  pub overflow: bool,
}

/// Runs `f`, an evaluation of a program compiled with [crate::prepare_traced], and returns what
/// it gives with the records that `plan` asks for. The recorder of an evaluation around this
/// one is put back when this one ends, also when it panics.
pub fn recording<T>(plan: Plan, f: impl FnOnce() -> T) -> (T, Trace) {
  struct Restore(Option<Recorder>);
  impl Drop for Restore {
    fn drop(&mut self) {
      RECORDER.with(|recorder| *recorder.borrow_mut() = self.0.take());
    }
  }
  let ways = plan.follow.is_some().then(Ways::default);
  let outer = RECORDER.with(|recorder| {
    recorder.borrow_mut().replace(Recorder {
      ways,
      plan,
      records: vec![],
      seen: Default::default(),
      frames: vec![Frame::default()],
      calling: None,
      truncated: false,
    })
  });
  let mut restore = Restore(outer);
  let result = f();
  let inner = RECORDER.with(|recorder| std::mem::replace(&mut *recorder.borrow_mut(), restore.0.take()));
  std::mem::forget(restore);
  let trace = inner.map_or_else(Trace::default, |recorder| {
    let (followed, overflow) = recorder.ways.map_or((vec![], false), |ways| {
      let Ways {
        pending,
        settled,
        paths,
        overflow,
        ..
      } = ways;
      // the steps of a place, from the root expression
      let within = |mut place: u32| {
        let mut steps = vec![];
        while let Some((before, step)) = paths.get(place as usize).filter(|_| place > 0) {
          steps.push(*step);
          place = *before;
        }
        steps.reverse();
        steps
      };
      let followed = settled.into_iter().chain(pending.into_iter().filter(|kept| !kept.settled)).map(|kept| Followed {
        target: kept.target,
        within: within(kept.path),
        value: kept.value,
        iterations: kept.iterations,
        bare: kept.bare,
      });
      (followed.collect(), overflow)
    });
    Trace {
      records: recorder.records,
      truncated: recorder.truncated,
      followed,
      overflow,
    }
  });
  (result, trace)
}

impl Recorder {
  /// The way that the node `target` is wanted now and has no record of yet, with the iteration
  /// that the record is of (None for a way that is not of each iteration); None when the node is
  /// not wanted now.
  fn now(&self, target: u32) -> Option<(usize, Option<usize>)> {
    let top = self.frames.last()?;
    let current = |frame: &Frame, repeat: u32| frame.repeat(repeat).and_then(|repeat| repeat.current);
    self.plan.wanted.get(target as usize)?.iter().enumerate().find_map(|(way, wanted)| {
      // the steps along the bodies being evaluated, from the root expression to the one that runs
      let mut depth = 0;
      for step in &wanted.within {
        match *step {
          Step::Iteration(repeat, index) => {
            if current(&self.frames[depth], repeat) != Some(index) {
              return None;
            }
          }
          Step::Call(site) => {
            depth += 1;
            if self.frames.get(depth)?.site != Some(site) {
              return None;
            }
          }
        }
      }
      if depth + 1 != self.frames.len() {
        return None;
      }
      let iteration = match wanted.each {
        Some((repeat, from, to)) => Some(current(top, repeat).filter(|index| (from..to).contains(index))?),
        None => None,
      };
      (!self.seen.contains(&(target, way, iteration))).then_some((way, iteration))
    })
  }
}

/// Whether a recorder is running that wants the value of the node `target` now, and has no
/// record of it, or keeps the ways of the nulls: its value is read only then.
pub(crate) fn wants(target: u32) -> bool {
  RECORDER.with(|recorder| {
    let mut recorder = recorder.borrow_mut();
    let Some(recorder) = recorder.as_mut() else {
      return false;
    };
    let follows = recorder.ways.as_ref().is_some_and(|ways| !ways.overflow && ways.untraced == 0);
    if recorder.now(target).is_none() {
      return follows;
    }
    if recorder.records.len() >= recorder.plan.limit {
      recorder.truncated = true;
      return follows;
    }
    true
  })
}

/// Keeps the value of the node `target` ([wants] said so).
pub(crate) fn record(target: u32, value: Value) {
  RECORDER.with(|recorder| {
    let mut recorder = recorder.borrow_mut();
    let Some(recorder) = recorder.as_mut() else {
      return;
    };
    let asked = match recorder.now(target) {
      Some((way, iteration)) if recorder.records.len() < recorder.plan.limit => {
        let iterations = recorder.plan.wanted[target as usize][way]
          .repeat
          .and_then(|repeat| recorder.frames.last()?.repeat(repeat))
          .map(|repeat| repeat.begun);
        recorder.seen.insert((target, way, iteration));
        recorder.records.push(Recorded {
          target,
          way,
          value: value.clone(),
          iteration,
          iterations,
        });
        true
      }
      _ => false,
    };
    recorder.keep(target, value, asked);
  });
}

impl Recorder {
  /// Keeps `value`, of the node `target`, for the ways of the nulls ([Follow]), and drops what
  /// was kept inside the node and is on no way. `asked`: the request asks for the value of the
  /// node, so what is on the way of its null stays whatever is around it.
  fn keep(&mut self, target: u32, value: Value, asked: bool) {
    let (Some(follow), Some(ways)) = (self.plan.follow.as_ref(), self.ways.as_mut()) else {
      return;
    };
    let Some(node) = follow.nodes.get(target as usize).copied() else {
      return;
    };
    if ways.overflow || ways.untraced > 0 {
      return;
    }
    let depth = self.frames.len() - 1;
    let null = value.is_null();
    // what was kept inside the node: the values since its evaluation began, which are of the
    // bodies it invoked and of the nodes inside it
    let inside = |kept: &Kept| kept.depth > depth || (kept.depth == depth && kept.target > target && kept.target <= node.last);
    let (mut start, mut pinned) = (ways.pending.len(), false);
    while start > 0 && inside(&ways.pending[start - 1]) {
      start -= 1;
      pinned |= ways.pending[start].pinned && ways.pending[start].depth == depth;
    }
    // an iteration: only the first whose value is null stays, with its way (a predicate that
    // a filter evaluates before its items is of no iteration)
    if node.part == FollowPart::Iteration {
      let first = null
        && node.parent.is_some_and(|repeat| {
          let repeat = self.frames[depth].repeat_mut(repeat);
          repeat.current.is_some() && !std::mem::replace(&mut repeat.kept, true)
        });
      if !first {
        ways.pending.truncate(start);
        return;
      }
    }
    let part_of = |kept: &Kept| follow.nodes.get(kept.target as usize).copied().unwrap_or_default();
    if node.keeps {
      // a context literal: what is kept of its entries stays
    } else if !null {
      // nothing inside stays, but what is pinned in this body
      if pinned {
        let mut at = 0;
        ways.pending.retain(|kept| {
          at += 1;
          at <= start || (kept.pinned && kept.depth == depth)
        });
      } else {
        ways.pending.truncate(start);
      }
    } else {
      // the way of the null: the parts stay, and what is inside the part that the null is from.
      // The parts, each with what stands before it since the part before; what follows the
      // last part is of the body that the invocation ran, whose value stands last
      let Ways { pending, parts, stays, .. } = &mut *ways;
      parts.clear();
      let mut from = start;
      for (index, kept) in pending.iter().enumerate().skip(start) {
        if kept.depth == depth && part_of(kept).parent == Some(target) {
          parts.push((from, index));
          from = index + 1;
        }
      }
      let body = (from < pending.len()).then(|| (from, pending.len() - 1));
      let from_body = body.filter(|(_, index)| {
        let kept = &pending[*index];
        kept.depth == depth + 1 && part_of(kept).part == FollowPart::Body && kept.value.is_null()
      });
      let first = |part: FollowPart| {
        let found = parts
          .iter()
          .filter(|(_, index)| pending[*index].value.is_null() && part_of(&pending[*index]).part == part && part_of(&pending[*index]).chain);
        found.min_by_key(|(_, index)| pending[*index].target).copied()
      };
      let chosen = from_body.or_else(|| first(FollowPart::Same)).or_else(|| first(FollowPart::Iteration));
      stays.clear();
      stays.resize(pending.len() - start, false);
      let mut dropped = false;
      for (from, index) in parts.iter().copied().chain(body) {
        let whole = chosen == Some((from, index)) || part_of(&pending[index]).keeps;
        stays[index - start] = true;
        let mut bare = false;
        for at in from..index {
          let stay = whole || (pending[at].pinned && pending[at].depth == depth);
          stays[at - start] = stay;
          bare |= !stay;
        }
        pending[index].bare |= bare;
        dropped |= bare;
      }
      if dropped {
        let mut at = 0;
        pending.retain(|_| {
          at += 1;
          at <= start || stays[at - 1 - start]
        });
      }
    }
    // the entry of a context literal that gives a null name its value stays with what is
    // inside it, whatever is around the literal: the way of the name goes on there
    if let (true, Some(entry)) = (null, node.link)
      && let Some(at) = ways.pending.iter().rposition(|kept| kept.depth == depth && kept.target == entry)
    {
      let last = follow.nodes.get(entry as usize).map_or(entry, |entry| entry.last);
      let inside = |kept: &Kept| kept.depth > depth || (kept.depth == depth && kept.target > entry && kept.target <= last);
      let from = at - ways.pending[..at].iter().rev().take_while(|kept| inside(kept)).count();
      for kept in &mut ways.pending[from..=at] {
        kept.pinned = true;
      }
      ways.pins = true;
    }
    // what is on the way of a null that the request asks for stays, and so does what is pinned
    // for a name on that way
    if asked && null {
      let Ways { pending, settled, pins, .. } = &mut *ways;
      if *pins {
        for kept in pending[..start].iter_mut().filter(|kept| kept.pinned && !kept.settled) {
          kept.settled = true;
          settled.push(Kept {
            target: kept.target,
            depth: kept.depth,
            path: kept.path,
            value: kept.value.clone(),
            iterations: kept.iterations,
            bare: kept.bare,
            pinned: false,
            settled: true,
          });
        }
      }
      settled.extend(pending.drain(start..).filter(|kept| !kept.settled));
    }
    if ways.pending.len() + ways.settled.len() >= follow.limit {
      ways.overflow = true;
      return;
    }
    if ways.paths.len() > ways.collect_at {
      ways.collect(&mut self.frames);
    }
    let path = ways.place(&self.frames[depth]);
    let iterations = self.frames[depth].repeat(target).map(|repeat| repeat.begun);
    ways.pending.push(Kept {
      target,
      depth,
      path,
      value,
      iterations,
      bare: false,
      pinned: false,
      settled: false,
    });
  }

  /// The evaluation moves to another iteration or another body: the values kept next are
  /// evaluated elsewhere.
  fn moved(&mut self) {
    if let Some(ways) = self.ways.as_mut() {
      ways.here = None;
    }
  }
}

impl Ways {
  /// Where the evaluation is, in the body `frame` that runs: the place of the body and every
  /// iteration that runs in it, the loop around another before it.
  fn place(&mut self, frame: &Frame) -> u32 {
    if let Some(here) = self.here {
      return here;
    }
    let mut place = frame.place;
    let mut after = None;
    // the loops in the order of their numbers, which is that of the text
    while let Some((number, index)) = frame
      .repeats
      .iter()
      .filter_map(|(number, repeat)| Some((*number, repeat.current?)))
      .filter(|(number, _)| after.is_none_or(|after| *number > after))
      .min()
    {
      self.paths.push((place, Step::Iteration(number, index)));
      place = self.paths.len() as u32 - 1;
      after = Some(number);
    }
    self.here = Some(place);
    place
  }

  /// Forgets the places of evaluation that no value kept has and no body that runs is at, and
  /// gives the others their new indexes. It runs when the places are twice as many as were
  /// left the last time, so a place costs a constant on average.
  fn collect(&mut self, frames: &mut [Frame]) {
    let mut live = vec![false; self.paths.len()];
    live[0] = true;
    let named = self.pending.iter().chain(&self.settled).map(|kept| kept.path);
    for place in named.chain(frames.iter().map(|frame| frame.place)).chain(self.here) {
      // a place and the places before it, up to one that is known to be named
      let mut place = place as usize;
      while let Some(seen) = live.get_mut(place).filter(|seen| !**seen) {
        *seen = true;
        place = self.paths[place].0 as usize;
      }
    }
    // the place before a place has a smaller index, and so its new index first
    let mut moved = vec![0u32; self.paths.len()];
    let mut next = 0;
    for place in 0..self.paths.len() {
      if live[place] {
        let (before, step) = self.paths[place];
        moved[place] = next as u32;
        self.paths[next] = (moved[before as usize], step);
        next += 1;
      }
    }
    self.paths.truncate(next);
    let renamed = |place: &mut u32| *place = moved.get(*place as usize).copied().unwrap_or(0);
    self.pending.iter_mut().chain(&mut self.settled).for_each(|kept| renamed(&mut kept.path));
    frames.iter_mut().for_each(|frame| renamed(&mut frame.place));
    self.here.iter_mut().for_each(renamed);
    self.collect_at = (2 * next).max(4_096);
  }
}

/// The loop `repeat` is opened in the body that runs: it has begun no iteration.
pub(crate) fn opened(repeat: u32) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(frame) = recorder.frames.last_mut()
    {
      *frame.repeat_mut(repeat) = Repeat::default();
      recorder.moved();
    }
  });
}

/// The loop `repeat` evaluates its body, or its predicate: for the item `item` when the loop
/// says which (a filter: None while it is evaluated before the items), else for the next.
pub(crate) fn stepped(repeat: u32, counted: bool, item: Option<usize>) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(frame) = recorder.frames.last_mut()
    {
      let repeat = frame.repeat_mut(repeat);
      if counted {
        repeat.current = Some(repeat.begun);
        repeat.begun += 1;
      } else {
        repeat.current = item;
        repeat.begun = item.map_or(repeat.begun, |item| item + 1);
      }
      recorder.moved();
    }
  });
}

/// The loop `repeat` is closed: no iteration of it runs.
pub(crate) fn closed(repeat: u32) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(frame) = recorder.frames.last_mut()
    {
      frame.repeat_mut(repeat).current = None;
      recorder.moved();
    }
  });
}

/// The invocation `site` is about to invoke a function whose body is of a program, or with None
/// has ended: the body that is entered next is the one it invokes.
pub(crate) fn calling(site: Option<u32>) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut() {
      recorder.calling = site;
    }
  });
}

/// The body of a function is entered: by the invocation that is about to invoke one, if any.
pub(crate) fn entered() {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut() {
      let site = recorder.calling.take();
      // the place of the body: that of the invocation, and the step into the body
      let mut place = 0;
      if let (Some(ways), Some(frame)) = (recorder.ways.as_mut(), recorder.frames.last()) {
        match site {
          Some(site) if ways.untraced == 0 && !ways.overflow => {
            let from = ways.place(frame);
            ways.paths.push((from, Step::Call(site)));
            place = ways.paths.len() as u32 - 1;
          }
          Some(_) => {}
          None => ways.untraced += 1,
        }
      }
      recorder.frames.push(Frame { site, repeats: vec![], place });
      recorder.moved();
    }
  });
}

/// The body that runs has its value: the body that invoked it runs again.
pub(crate) fn left() {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && recorder.frames.len() > 1
    {
      let left = recorder.frames.pop();
      recorder.calling = None;
      if let (Some(Frame { site: None, .. }), Some(ways)) = (left, recorder.ways.as_mut()) {
        ways.untraced = ways.untraced.saturating_sub(1);
      }
      recorder.moved();
    }
  });
}

/// Where a part of a node is evaluated, seen from the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartFrame {
  /// Once, where the node is.
  Same,
  /// Once for each item that the node visits: the body of a `for`, `some` or `every`, the
  /// predicate of a filter.
  Iteration,
}

/// The parts of `node` that are evaluated for its value, one level down, in the order of the
/// text: the operands of an operator, the entries of a context literal, the items of a list,
/// the arguments of an invocation, the list and the predicate of a filter, the first domain and
/// the body of a loop. The nodes that only hold other nodes (the list of the arguments, the
/// contexts of an iteration) are passed through. A function definition has no parts: its body
/// ([body]) is evaluated where it is invoked. Of a loop with several variables only the first domain is
/// a part (the others are evaluated for each item of those before).
pub fn parts(node: &AstNode) -> Vec<(&AstNode, PartFrame)> {
  use AstNode as A;
  use PartFrame::{Iteration, Same};
  let mut found = vec![];
  let through = |node: &'_ AstNode| -> bool {
    matches!(
      node,
      A::EvaluatedExpression(_) | A::Satisfies(_) | A::IntervalStart(..) | A::IntervalEnd(..) | A::PositionalParameters(_) | A::NamedParameters(_) | A::NamedParameter(..)
    )
  };
  // the nodes below `node` that are evaluated, through the nodes that only hold others
  fn expressions<'a>(node: &'a AstNode, frame: PartFrame, through: &dyn Fn(&AstNode) -> bool, found: &mut Vec<(&'a AstNode, PartFrame)>) {
    if through(node) {
      node.for_each_child(|child| match child {
        AstNode::ParameterName(_) => {}
        child => expressions(child, frame, through, found),
      });
    } else {
      found.push((node, frame));
    }
  }
  match node {
    A::FunctionDefinition(..) | A::ContextEntryKey(_) | A::FeelType(_) => {}
    A::Filter(list, predicate) => {
      expressions(list, Same, &through, &mut found);
      expressions(predicate, Iteration, &through, &mut found);
    }
    A::For(contexts, body) | A::Some(contexts, body) | A::Every(contexts, body) => {
      let first = match contexts.as_ref() {
        A::IterationContexts(items) | A::QuantifiedContexts(items) => items.first(),
        _ => None,
      };
      match first {
        Some(A::IterationContextSingle(_, domain) | A::QuantifiedContext(_, domain)) => expressions(domain, Same, &through, &mut found),
        Some(A::IterationContextInterval(_, start, end)) => {
          expressions(start, Same, &through, &mut found);
          expressions(end, Same, &through, &mut found);
        }
        _ => {}
      }
      expressions(body, Iteration, &through, &mut found);
    }
    A::ContextEntry(_, value) => expressions(value, Same, &through, &mut found),
    A::InstanceOf(value, _) | A::As(value, _) => expressions(value, Same, &through, &mut found),
    A::Path(base, _) | A::Descendant(base, _) => expressions(base, Same, &through, &mut found),
    A::FunctionInvocation(callee, arguments) => {
      // the name of a function is no part; an expression that gives the function is
      if !matches!(callee.as_ref(), A::Name(_) | A::QualifiedName(_)) {
        expressions(callee, Same, &through, &mut found);
      }
      expressions(arguments, Same, &through, &mut found);
    }
    other => other.for_each_child(|child| expressions(child, Same, &through, &mut found)),
  }
  found
}

/// The body of the function definition `node` that is evaluated when the function is invoked:
/// None for another node and for a definition whose body is external.
pub fn body(node: &AstNode) -> Option<&AstNode> {
  match node {
    AstNode::FunctionDefinition(_, body) => match body.as_ref() {
      AstNode::FunctionBody(body, false) => Some(body),
      _ => None,
    },
    _ => None,
  }
}
