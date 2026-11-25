//! The record of an evaluation: the values of the nodes that a request asks for, kept while a
//! program compiled for that request runs ([super::lower_traced]). The evaluation itself is the
//! one that every other program has: a recording instruction reads a value and changes nothing.
//!
//! The recorder is of the thread, as the diagnostics of an evaluation are: [recording] starts
//! one around an evaluation and gives what it kept. Outside one, a recording instruction has no
//! effect.

use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::cell::RefCell;

/// What a program is compiled to record: the nodes and the loops of a request, by their numbers.
pub struct Tracing<'t> {
  /// The number of a node whose value the request asks for, None for another node.
  pub node: &'t dyn Fn(&AstNode) -> Option<u32>,
  /// The number of a loop or a filter that the request follows (the records of nodes in its
  /// body are of one of its iterations, or it says how many it had), None for another.
  pub repeat: &'t dyn Fn(&AstNode) -> Option<u32>,
}

/// A node whose value a request asks for, and when: a node in the body of a loop or in the
/// predicate of a filter is evaluated once for each item, and the request says for which.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wanted {
  /// The iterations that the node is wanted in: for each loop around it, innermost last, the
  /// number of the loop ([crate::Tracing::repeat]) and the index of the iteration.
  pub within: Vec<(u32, usize)>,
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
  /// The number of loops that have a number.
  pub repeats: usize,
  /// The most records that are kept.
  pub limit: usize,
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

/// Where a loop that has a number is.
#[derive(Clone, Copy, Default)]
struct Repeat {
  /// The iterations begun since the loop was opened.
  begun: usize,
  /// The index of the iteration that runs, None outside one.
  current: Option<usize>,
}

struct Recorder {
  plan: Plan,
  records: Vec<Recorded>,
  /// The records kept so far, by the node and the iteration: the first evaluation of a node in
  /// an iteration is the one kept.
  seen: std::collections::HashSet<(u32, usize, Option<usize>)>,
  repeats: Vec<Repeat>,
  /// Whether a record was not kept for the limit.
  truncated: bool,
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
  let repeats = vec![Repeat::default(); plan.repeats];
  let outer = RECORDER.with(|recorder| {
    recorder.borrow_mut().replace(Recorder {
      plan,
      records: vec![],
      seen: Default::default(),
      repeats,
      truncated: false,
    })
  });
  let mut restore = Restore(outer);
  let result = f();
  let inner = RECORDER.with(|recorder| std::mem::replace(&mut *recorder.borrow_mut(), restore.0.take()));
  std::mem::forget(restore);
  let trace = inner.map_or_else(Trace::default, |recorder| Trace {
    records: recorder.records,
    truncated: recorder.truncated,
  });
  (result, trace)
}

impl Recorder {
  /// The way that the node `target` is wanted now and has no record of yet, with the iteration
  /// that the record is of (None for a way that is not of each iteration); None when the node is
  /// not wanted now.
  fn now(&self, target: u32) -> Option<(usize, Option<usize>)> {
    let current = |repeat: u32| self.repeats.get(repeat as usize).and_then(|repeat| repeat.current);
    self.plan.wanted.get(target as usize)?.iter().enumerate().find_map(|(way, wanted)| {
      if !wanted.within.iter().all(|&(repeat, index)| current(repeat) == Some(index)) {
        return None;
      }
      let iteration = match wanted.each {
        Some((repeat, from, to)) => Some(current(repeat).filter(|index| (from..to).contains(index))?),
        None => None,
      };
      (!self.seen.contains(&(target, way, iteration))).then_some((way, iteration))
    })
  }
}

/// Whether a recorder is running that wants the value of the node `target` now, and has no
/// record of it: its value is read only then.
pub(crate) fn wants(target: u32) -> bool {
  RECORDER.with(|recorder| {
    let mut recorder = recorder.borrow_mut();
    let Some(recorder) = recorder.as_mut() else {
      return false;
    };
    if recorder.now(target).is_none() {
      return false;
    }
    if recorder.records.len() >= recorder.plan.limit {
      recorder.truncated = true;
      return false;
    }
    true
  })
}

/// Keeps the value of the node `target` ([wants] said so).
pub(crate) fn record(target: u32, value: Value) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some((way, iteration)) = recorder.now(target)
    {
      let iterations = recorder.plan.wanted[target as usize][way]
        .repeat
        .and_then(|repeat| recorder.repeats.get(repeat as usize))
        .map(|repeat| repeat.begun);
      recorder.seen.insert((target, way, iteration));
      recorder.records.push(Recorded {
        target,
        way,
        value,
        iteration,
        iterations,
      });
    }
  });
}

/// The loop `repeat` is opened: it has begun no iteration.
pub(crate) fn opened(repeat: u32) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(repeat) = recorder.repeats.get_mut(repeat as usize)
    {
      *repeat = Repeat::default();
    }
  });
}

/// The loop `repeat` evaluates its body, or its predicate: for the item `item` when the loop
/// says which (a filter: None while it is evaluated before the items), else for the next.
pub(crate) fn stepped(repeat: u32, counted: bool, item: Option<usize>) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(repeat) = recorder.repeats.get_mut(repeat as usize)
    {
      if counted {
        repeat.current = Some(repeat.begun);
        repeat.begun += 1;
      } else {
        repeat.current = item;
        repeat.begun = item.map_or(repeat.begun, |item| item + 1);
      }
    }
  });
}

/// The loop `repeat` is closed: no iteration of it runs.
pub(crate) fn closed(repeat: u32) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(repeat) = recorder.repeats.get_mut(repeat as usize)
    {
      repeat.current = None;
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
/// is evaluated where it is invoked. Of a loop with several variables only the first domain is
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
