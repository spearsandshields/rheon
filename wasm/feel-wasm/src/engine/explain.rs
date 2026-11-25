//! `op: "explain"`: an evaluation, and how its value came about: the values of the nodes of the
//! text that the request asks for, one level at a time.
//!
//! The answer is that of `evaluate` with a `trace`. By default the trace has the root of the
//! text and its parts one level down; `expand` names nodes, by their places, whose parts are
//! wanted too. Each request evaluates the text again, with a record of the values of those
//! nodes only: the engine keeps nothing between requests, and an answer is as long as the
//! request asks, whatever the data is.

use super::*;
use rheon_feel_evaluator::{PartFrame, parts, prepare_traced, recording};
use rheon_feel_parser::{Positioned, parse_positioned};
use std::collections::HashMap;

/// The most records of one answer when the request does not say: beyond the limit the trace is
/// cut and says so.
const DEFAULT_RECORDS: usize = 200;

/// The most records that a request can ask for.
const MAX_RECORDS: usize = 10_000;

/// The longest value, as the text of its JSON, that a record holds whole: a longer one is
/// summarised.
const WHOLE_VALUE: usize = 4_096;

/// The most parts (items, entries and simple values, a long string counted by its length) of a
/// value that is written whole.
const WHOLE_PARTS: usize = 256;

/// Whether `value` has no more parts than `left`, which is counted down: the walk ends where
/// the count does, whatever the value holds.
fn small(value: &Value, left: &mut isize) -> bool {
  *left -= 1;
  if *left < 0 {
    return false;
  }
  match value {
    Value::List(items) => items.iter().all(|item| small(item, left)),
    Value::Context(context) => context.iter().all(|(_, entry)| small(entry, left)),
    Value::String(text) => {
      *left -= (text.len() / 64) as isize;
      *left >= 0
    }
    _ => true,
  }
}

/// A node whose value the request asks for.
struct Wanted<'t> {
  node: &'t AstNode,
  start: usize,
  end: usize,
  kind: &'static str,
}

/// The node of `tree` at the place `start..end`: the outermost that has it.
fn node_at(positioned: &Positioned, start: usize, end: usize) -> Option<&AstNode> {
  let mut stack = vec![positioned.tree()];
  while let Some(node) = stack.pop() {
    if positioned.place(node) == Some((start, end)) {
      return Some(node);
    }
    let from = stack.len();
    node.for_each_child(|child| stack.push(child));
    stack[from..].reverse();
  }
  None
}

/// Adds `node` to the nodes that are wanted, when it has a place and is not among them.
fn want<'t>(positioned: &'t Positioned, node: &'t AstNode, wanted: &mut Vec<Wanted<'t>>) {
  if let (Some((start, end)), Some(kind)) = (positioned.place(node), positioned.kind(node))
    && !wanted.iter().any(|known| known.start == start && known.end == end)
  {
    wanted.push(Wanted { node, start, end, kind });
  }
}

/// The node itself, or for an entry of a context literal the node of its value: the value of
/// the entry is the value of that node.
fn valued(node: &AstNode) -> &AstNode {
  match node {
    AstNode::ContextEntry(_, value) => value,
    node => node,
  }
}

pub(super) fn explain(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<String, Failure> {
  const EXPAND: &str = "'expand' must be a list of places, each [from, to] in UTF-16 units";
  const LIMIT: &str = "'maxRecords' must be a whole number from 1 to 10000";
  let dialect = dialect(&req)?;
  let language = language(&req)?;
  let error_mode = error_mode(&req)?;
  let max_steps = max_steps(&req)?;
  let expression = expression(&mut req)?;
  apply_tz_offset(&req)?;
  let places = editor::Places::new(&expression);
  let expand: Vec<(usize, usize)> = match req.remove("expand") {
    None | Some(Json::Null) => vec![],
    Some(Json::Array(items)) => items
      .iter()
      .map(|item| match item.as_array().map(Vec::as_slice) {
        Some([from, to]) => match (from.as_u64(), to.as_u64()) {
          (Some(from), Some(to)) if from <= to => Ok((places.byte(from as usize), places.byte(to as usize))),
          _ => fail("invalid-field", EXPAND),
        },
        _ => fail("invalid-field", EXPAND),
      })
      .collect::<Result<_, _>>()?,
    Some(_) => return fail("invalid-field", EXPAND),
  };
  let limit = match req.remove("maxRecords") {
    None | Some(Json::Null) => DEFAULT_RECORDS,
    Some(Json::Number(limit)) => match limit.as_u64() {
      Some(limit @ 1..=10_000) => limit as usize,
      _ => return fail("invalid-field", LIMIT),
    },
    Some(_) => return fail("invalid-field", LIMIT),
  };
  debug_assert!(limit <= MAX_RECORDS);
  let mut warnings = unknown_fields(&req);
  let scope = prepare_scope(req, unread, dialect, &mut warnings)?;
  let positioned = parse_positioned(&scope, dialect.rule(), &expression).map_err(|e| {
    let diagnostics = editor::diagnostics(&scope, &expression, dialect.rule());
    Failure::new("syntax-error", syntax_message(&e.to_string()), Some(diagnostics))
  })?;
  check_ast_depth(positioned.tree()).map_err(|e| Failure::new("expression-too-deep", e, None))?;
  // the nodes that are wanted: the root and its parts, and the parts of the nodes to expand
  let mut wanted: Vec<Wanted> = vec![];
  let root = positioned.tree();
  let mut open: Vec<&AstNode> = vec![root];
  // the root of a text is its value: of unary tests and of a list of expressions, each of them
  want(&positioned, root, &mut wanted);
  for &(start, end) in &expand {
    match node_at(&positioned, start, end) {
      Some(node) => open.push(node),
      None => {
        return fail(
          "invalid-field",
          format!(
            "'expand' has the place [{}, {}], which is the place of no node of the text (the places are those of the records, and of the nodes of `syntax`)",
            places.of(start),
            places.of(end)
          ),
        );
      }
    }
  }
  for node in open {
    for (part, frame) in parts(valued(node)) {
      // a part that is evaluated once for each item is asked for with the item (not yet)
      if frame == PartFrame::Same {
        want(&positioned, part, &mut wanted);
      }
    }
  }
  let targets: HashMap<usize, u32> = wanted
    .iter()
    .enumerate()
    .map(|(index, wanted)| (valued(wanted.node) as *const AstNode as usize, index as u32))
    .collect();
  let target = |node: &AstNode| targets.get(&(node as *const AstNode as usize)).copied();
  // of unary tests the evaluation is the test of the value `?`: the tests are the text
  let evaluator = match dialect {
    Dialect::UnaryTests => {
      return fail(
        "invalid-field",
        "'explain' takes an expression; unary tests are explained as the expression `? in (<tests>)`",
      );
    }
    _ => prepare_traced(positioned.tree(), language, &target),
  };
  let (answer, trace) = recording(limit, || answered(&scope, None, &evaluator, error_mode, max_steps, warnings, started));
  let mut response = answer?;
  // in the order of the text, a node before the nodes inside it
  let mut order: Vec<&rheon_feel_evaluator::Recorded> = trace.records.iter().filter(|record| (record.target as usize) < wanted.len()).collect();
  order.sort_by_key(|record| {
    let wanted = &wanted[record.target as usize];
    (wanted.start, std::cmp::Reverse(wanted.end))
  });
  // the records as text, the keys of each in alphabetical order as in every response
  let mut records = String::from(",\"trace\":[");
  for (index, record) in order.into_iter().enumerate() {
    let wanted = &wanted[record.target as usize];
    if index > 0 {
      records.push(',');
    }
    records.push_str(&format!("{{\"from\":{},\"kind\":\"{}\"", places.of(wanted.start), wanted.kind));
    if let AstNode::ContextEntry(key, _) = wanted.node
      && let AstNode::ContextEntryKey(name) = key.as_ref()
    {
      records.push_str(",\"name\":");
      crate::output::write_string(&name.to_string(), &mut records);
    }
    records.push_str(&format!(",\"to\":{},\"value\":", places.of(wanted.end)));
    write_value(&record.value, &mut records);
    records.push('}');
  }
  records.push_str(if trace.truncated { "],\"traceTruncated\":true" } else { "],\"traceTruncated\":false" });
  // before the warnings, the last field of the response of an evaluation
  match response.rfind(",\"warnings\":") {
    Some(at) => response.insert_str(at, &records),
    None => return fail("internal", "the response of the evaluation has no warnings"),
  }
  Ok(response)
}

/// Writes the value of a record: as a result of `evaluate` has it, or for a value whose text is
/// long its type and how many items or entries it holds.
fn write_value(value: &Value, out: &mut String) {
  // a value is measured before it is written: one that holds a value many times over is small
  // to hold and can be very long to write
  if small(value, &mut (WHOLE_PARTS as isize)) {
    let start = out.len();
    write_result(value, out);
    if out.len() - start <= WHOLE_VALUE {
      return;
    }
    out.truncate(start);
  }
  out.push('{');
  let count = match value {
    Value::List(items) => Some(items.len()),
    Value::Context(context) => Some(context.iter().len()),
    _ => None,
  };
  if let Some(count) = count {
    out.push_str(&format!("\"count\":{count},"));
  }
  out.push_str(if value.is_null() { "\"isNull\":true" } else { "\"isNull\":false" });
  // the type without what it holds: the type of a value that holds a value many times over is
  // as long to write as the value
  out.push_str(",\"summarized\":true,\"type\":");
  match value {
    Value::List(_) => out.push_str("\"list\""),
    Value::Context(_) => out.push_str("\"context\""),
    other => crate::output::write_string(&other.type_of().to_string(), out),
  }
  out.push('}');
}
