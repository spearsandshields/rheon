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
}

/// The root expression, or a body of a function that is being evaluated.
#[derive(Default)]
struct Frame {
  /// The invocation that the request follows and that invoked the body, None for the root
  /// expression and for a body invoked otherwise.
  site: Option<u32>,
  /// The loops with a number that were opened in it, each with where it is.
  repeats: Vec<(u32, Repeat)>,
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
  let outer = RECORDER.with(|recorder| {
    recorder.borrow_mut().replace(Recorder {
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
        .and_then(|repeat| recorder.frames.last()?.repeat(repeat))
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

/// The loop `repeat` is opened in the body that runs: it has begun no iteration.
pub(crate) fn opened(repeat: u32) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && let Some(frame) = recorder.frames.last_mut()
    {
      *frame.repeat_mut(repeat) = Repeat::default();
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
      recorder.frames.push(Frame { site, repeats: vec![] });
    }
  });
}

/// The body that runs has its value: the body that invoked it runs again.
pub(crate) fn left() {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut()
      && recorder.frames.len() > 1
    {
      recorder.frames.pop();
      recorder.calling = None;
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
