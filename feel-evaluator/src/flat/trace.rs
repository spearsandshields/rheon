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

/// The value that an evaluation gave for a node that a request asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
  /// The number that the request gave the node ([crate::prepare_traced]).
  pub target: u32,
  pub value: Value,
}

struct Recorder {
  records: Vec<Recorded>,
  /// The targets that have their record: the first evaluation of a node is the one kept.
  seen: Vec<bool>,
  /// The most records that are kept.
  limit: usize,
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
  /// Whether more nodes were evaluated than the limit of records.
  pub truncated: bool,
}

/// Runs `f`, an evaluation of a program compiled with [crate::prepare_traced], and returns what
/// it gives with the records of at most `limit` nodes. The recorder of an evaluation around
/// this one is put back when this one ends, also when it panics.
pub fn recording<T>(limit: usize, f: impl FnOnce() -> T) -> (T, Trace) {
  struct Restore(Option<Recorder>);
  impl Drop for Restore {
    fn drop(&mut self) {
      RECORDER.with(|recorder| *recorder.borrow_mut() = self.0.take());
    }
  }
  let outer = RECORDER.with(|recorder| {
    recorder.borrow_mut().replace(Recorder {
      records: vec![],
      seen: vec![],
      limit,
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

/// Whether the node `target` has no record yet and a recorder is running: its value is read
/// only then.
pub(crate) fn wants(target: u32) -> bool {
  RECORDER.with(|recorder| {
    let mut recorder = recorder.borrow_mut();
    let Some(recorder) = recorder.as_mut() else {
      return false;
    };
    if recorder.seen.get(target as usize).copied().unwrap_or(false) {
      return false;
    }
    if recorder.records.len() >= recorder.limit {
      recorder.truncated = true;
      return false;
    }
    true
  })
}

/// Keeps the value of the node `target` ([wants] said so).
pub(crate) fn record(target: u32, value: Value) {
  RECORDER.with(|recorder| {
    if let Some(recorder) = recorder.borrow_mut().as_mut() {
      let at = target as usize;
      if recorder.seen.len() <= at {
        recorder.seen.resize(at + 1, false);
      }
      recorder.seen[at] = true;
      recorder.records.push(Recorded { target, value });
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
