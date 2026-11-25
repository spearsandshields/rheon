//! `op: "explain"`: an evaluation, and how its value came about: the values of the nodes of the
//! text that the request asks for, one level at a time.
//!
//! The answer is that of `evaluate` with a `trace`. By default the trace has the root of the
//! text and its parts one level down; `expand` names nodes, by their places, whose parts are
//! wanted too. A part of a loop or a filter that is evaluated once for each item (the body, the
//! predicate) has a record for each of its first iterations, or for those that the request
//! names; a node inside such a part is named with the iteration it is in. Each request
//! evaluates the text again, with a record of the values of those nodes only: the engine keeps
//! nothing between requests, and an answer is as long as the request asks, whatever the data is.

use super::*;
use rheon_feel_evaluator::{PartFrame, Plan, Tracing, parts, prepare_traced, recording};
use rheon_feel_parser::{Positioned, parse_positioned};
use std::collections::HashMap;

/// The most records of one answer when the request does not say: beyond the limit the trace is
/// cut and says so.
const DEFAULT_RECORDS: usize = 200;

/// The most records that a request can ask for.
const MAX_RECORDS: usize = 10_000;

/// The iterations of a loop that have a record when the request does not say which: the first.
const DEFAULT_ITERATIONS: usize = 20;

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

/// A place of the text, in bytes.
type At = (usize, usize);

/// A node whose value the request asks for, in one of the ways it is asked for.
struct Wanted<'t> {
  node: &'t AstNode,
  at: At,
  kind: &'static str,
  /// The iterations it is in: the place of each loop around it, outermost first, and the index.
  within: Vec<(At, usize)>,
  /// It is the body of this loop, wanted for each iteration from the first index up to the second.
  each: Option<(&'t AstNode, usize, usize)>,
}

/// An entry of `expand`: a node by its place, the iterations it is in, and for a loop which of
/// its iterations are wanted.
struct Asked {
  at: At,
  within: Vec<(At, usize)>,
  iterations: Option<(usize, usize)>,
}

/// A node whose parts are wanted: the node, the iterations it is in, and for a loop which of
/// its iterations.
type Open<'t> = (&'t AstNode, Vec<(At, usize)>, Option<(usize, usize)>);

/// The node of `tree` at the place `at`: the outermost that has it.
fn node_at(positioned: &Positioned, at: At) -> Option<&AstNode> {
  let mut stack = vec![positioned.tree()];
  while let Some(node) = stack.pop() {
    if positioned.place(node) == Some(at) {
      return Some(node);
    }
    let from = stack.len();
    node.for_each_child(|child| stack.push(child));
    stack[from..].reverse();
  }
  None
}

/// The node itself, or for an entry of a context literal the node of its value: the value of
/// the entry is the value of that node.
fn valued(node: &AstNode) -> &AstNode {
  match node {
    AstNode::ContextEntry(_, value) => value,
    node => node,
  }
}

/// Whether `node` visits items: a `for`, `some` or `every`, or a filter.
fn repeats(node: &AstNode) -> bool {
  matches!(node, AstNode::For(..) | AstNode::Some(..) | AstNode::Every(..) | AstNode::Filter(..))
}

/// The loops around `target` in `tree`, outermost first: those in whose body or predicate it
/// stands. None for a node that is no part of the tree as [parts] walks it (a node of the body
/// of a function definition).
fn loops_around<'t>(tree: &'t AstNode, target: &AstNode) -> Option<Vec<&'t AstNode>> {
  // a walk with its own stack: the node, and the loops around it
  let mut stack: Vec<(&AstNode, Vec<&AstNode>)> = vec![(tree, vec![])];
  while let Some((node, around)) = stack.pop() {
    if std::ptr::eq(node, target) || std::ptr::eq(valued(node), target) {
      return Some(around);
    }
    for (part, frame) in parts(valued(node)) {
      let mut around = around.clone();
      if frame == PartFrame::Iteration {
        around.push(valued(node));
      }
      stack.push((part, around));
    }
  }
  None
}

pub(super) fn explain(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<String, Failure> {
  const EXPAND: &str = "'expand' must be a list of nodes, each its place [from, to] in UTF-16 units, or {\"at\": [from, to], \"within\": [{\"at\": [from, to], \"iteration\": n}, …], \"iterations\": [from, to]}";
  const LIMIT: &str = "'maxRecords' must be a whole number from 1 to 10000";
  let dialect = dialect(&req)?;
  let language = language(&req)?;
  let error_mode = error_mode(&req)?;
  let max_steps = max_steps(&req)?;
  let expression = expression(&mut req)?;
  apply_tz_offset(&req)?;
  let places = editor::Places::new(&expression);
  let place = |value: &Json| -> Result<At, Failure> {
    match value.as_array().map(Vec::as_slice) {
      Some([from, to]) => match (from.as_u64(), to.as_u64()) {
        (Some(from), Some(to)) if from <= to => Ok((places.byte(from as usize), places.byte(to as usize))),
        _ => fail("invalid-field", EXPAND),
      },
      _ => fail("invalid-field", EXPAND),
    }
  };
  let asked: Vec<Asked> = match req.remove("expand") {
    None | Some(Json::Null) => vec![],
    Some(Json::Array(items)) => items
      .iter()
      .map(|item| match item {
        Json::Array(_) => Ok(Asked {
          at: place(item)?,
          within: vec![],
          iterations: None,
        }),
        Json::Object(fields) if fields.keys().all(|field| matches!(field.as_str(), "at" | "within" | "iterations")) => {
          let at = place(fields.get("at").unwrap_or(&Json::Null))?;
          let within = match fields.get("within") {
            None | Some(Json::Null) => vec![],
            Some(Json::Array(steps)) => steps
              .iter()
              .map(|step| match (step.get("at"), step.get("iteration").and_then(Json::as_u64)) {
                (Some(at), Some(iteration)) if step.as_object().is_some_and(|fields| fields.len() == 2) => Ok((place(at)?, iteration as usize)),
                _ => fail("invalid-field", EXPAND),
              })
              .collect::<Result<_, _>>()?,
            Some(_) => return fail("invalid-field", EXPAND),
          };
          let iterations = match fields.get("iterations") {
            None | Some(Json::Null) => None,
            Some(range) => match range.as_array().map(Vec::as_slice) {
              Some([from, to]) => match (from.as_u64(), to.as_u64()) {
                (Some(from), Some(to)) if from <= to => Some((from as usize, to as usize)),
                _ => return fail("invalid-field", EXPAND),
              },
              _ => return fail("invalid-field", EXPAND),
            },
          };
          Ok(Asked { at, within, iterations })
        }
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
  if dialect == Dialect::UnaryTests {
    return fail(
      "invalid-field",
      "'explain' takes an expression; unary tests are explained as the expression `? in (<tests>)`",
    );
  }
  let mut warnings = unknown_fields(&req);
  let scope = prepare_scope(req, unread, dialect, &mut warnings)?;
  let positioned = parse_positioned(&scope, dialect.rule(), &expression).map_err(|e| {
    let diagnostics = editor::diagnostics(&scope, &expression, dialect.rule());
    Failure::new("syntax-error", syntax_message(&e.to_string()), Some(diagnostics))
  })?;
  check_ast_depth(positioned.tree()).map_err(|e| Failure::new("expression-too-deep", e, None))?;
  let shown = |at: At| format!("[{}, {}]", places.of(at.0), places.of(at.1));
  // the nodes that are wanted: the root, its parts, and the parts of the nodes to expand
  let root = positioned.tree();
  let mut wanted: Vec<Wanted> = vec![];
  add(&positioned, root, &[], None, &mut wanted);
  // the nodes whose parts are wanted: the node, the iterations it is in, and those of it asked for
  let mut open: Vec<Open> = vec![(root, vec![], None)];
  for Asked { at, within, iterations } in asked {
    let Some(node) = node_at(&positioned, at) else {
      return fail(
        "invalid-field",
        format!(
          "'expand' has the place {}, which is the place of no node of the text (the places are those of the records, and of the nodes of `syntax`)",
          shown(at)
        ),
      );
    };
    // the iterations it is in are those of the loops around it
    let Some(around) = loops_around(root, node) else {
      return fail(
        "invalid-field",
        format!("'expand' has the place {}, a node of the body of a function: its parts are not recorded", shown(at)),
      );
    };
    let expected: Vec<At> = around.iter().filter_map(|repeat| positioned.place(repeat)).collect();
    if expected.len() != around.len() || within.iter().map(|(at, _)| *at).collect::<Vec<_>>() != expected {
      let wants: Vec<String> = expected.iter().map(|at| format!("{{\"at\": {}, \"iteration\": n}}", shown(*at))).collect();
      return fail(
        "invalid-field",
        format!(
          "'expand' has the node at {}, which is evaluated once for each item of {}: expected \"within\": [{}], found {} of them",
          shown(at),
          if expected.len() == 1 {
            "the loop around it".to_string()
          } else {
            format!("the {} loops around it", expected.len())
          },
          wants.join(", "),
          within.len()
        ),
      );
    }
    if iterations.is_some() && !repeats(valued(node)) {
      return fail(
        "invalid-field",
        format!("'expand' has \"iterations\" for the node at {}, which is no loop and no filter", shown(at)),
      );
    }
    open.push((node, within, iterations));
  }
  for (node, within, iterations) in &open {
    let node = valued(node);
    for (part, frame) in parts(node) {
      match frame {
        PartFrame::Same => add(&positioned, part, within, None, &mut wanted),
        // once for each item: a record for each of the iterations asked for
        PartFrame::Iteration => {
          let (from, to) = iterations.unwrap_or((0, DEFAULT_ITERATIONS));
          add(&positioned, part, within, Some((node, from, to)), &mut wanted);
        }
      }
    }
  }
  // the numbers of the nodes and of the loops that the program records
  let mut targets: HashMap<usize, u32> = HashMap::new();
  let mut repeat_numbers: HashMap<usize, u32> = HashMap::new();
  let address = |node: &AstNode| node as *const AstNode as usize;
  let mut plan = Plan {
    wanted: vec![],
    repeats: 0,
    limit,
  };
  // the ways of a node, each with the index of what is wanted
  let mut ways: Vec<Vec<usize>> = vec![];
  let repeat_of = |node: &AstNode, numbers: &mut HashMap<usize, u32>| -> u32 {
    let next = numbers.len() as u32;
    *numbers.entry(address(node)).or_insert(next)
  };
  for (index, wanted) in wanted.iter().enumerate() {
    let node = valued(wanted.node);
    let next = targets.len() as u32;
    let target = *targets.entry(address(node)).or_insert(next) as usize;
    if plan.wanted.len() <= target {
      plan.wanted.resize(target + 1, vec![]);
      ways.resize(target + 1, vec![]);
    }
    let mut within = vec![];
    for (at, iteration) in &wanted.within {
      if let Some(repeat) = node_at(&positioned, *at) {
        within.push((repeat_of(valued(repeat), &mut repeat_numbers), *iteration));
      }
    }
    plan.wanted[target].push(rheon_feel_evaluator::Wanted {
      within,
      each: wanted.each.map(|(repeat, from, to)| (repeat_of(repeat, &mut repeat_numbers), from, to)),
      repeat: repeats(node).then(|| repeat_of(node, &mut repeat_numbers)),
    });
    ways[target].push(index);
  }
  plan.repeats = repeat_numbers.len();
  let node_number = |node: &AstNode| targets.get(&address(node)).copied();
  let repeat_number = |node: &AstNode| repeat_numbers.get(&address(node)).copied();
  let tracing = Tracing {
    node: &node_number,
    repeat: &repeat_number,
  };
  let evaluator = prepare_traced(positioned.tree(), language, &tracing);
  let (answer, trace) = recording(plan, || answered(&scope, None, &evaluator, error_mode, max_steps, warnings, started));
  let mut response = answer?;
  // in the order of the text, a node before the nodes inside it, an iteration before the next
  let mut order: Vec<(&Wanted, &rheon_feel_evaluator::Recorded)> = trace
    .records
    .iter()
    .filter_map(|record| Some((&wanted[*ways.get(record.target as usize)?.get(record.way)?], record)))
    .collect();
  order.sort_by_key(|(wanted, record)| (wanted.at.0, std::cmp::Reverse(wanted.at.1), wanted.within.clone(), record.iteration));
  // the records as text, the keys of each in alphabetical order as in every response
  let mut records = String::from(",\"trace\":[");
  for (index, (wanted, record)) in order.into_iter().enumerate() {
    if index > 0 {
      records.push(',');
    }
    records.push_str(&format!("{{\"from\":{}", places.of(wanted.at.0)));
    if let Some(iteration) = record.iteration {
      records.push_str(&format!(",\"iteration\":{iteration}"));
    }
    if let Some(iterations) = record.iterations {
      records.push_str(&format!(",\"iterations\":{iterations}"));
    }
    records.push_str(&format!(",\"kind\":\"{}\"", wanted.kind));
    if let AstNode::ContextEntry(key, _) = wanted.node
      && let AstNode::ContextEntryKey(name) = key.as_ref()
    {
      records.push_str(",\"name\":");
      crate::output::write_string(&name.to_string(), &mut records);
    }
    records.push_str(&format!(",\"to\":{},\"value\":", places.of(wanted.at.1)));
    write_value(&record.value, &mut records);
    // the iterations it is in: of the loops around it, and of its own loop for a body
    let mut within: Vec<(At, usize)> = wanted.within.clone();
    if let (Some((repeat, ..)), Some(iteration)) = (wanted.each, record.iteration)
      && let Some(at) = positioned.place(repeat)
    {
      within.push((at, iteration));
    }
    if !within.is_empty() {
      records.push_str(",\"within\":[");
      for (index, (at, iteration)) in within.iter().enumerate() {
        if index > 0 {
          records.push(',');
        }
        records.push_str(&format!("{{\"at\":[{},{}],\"iteration\":{iteration}}}", places.of(at.0), places.of(at.1)));
      }
      records.push(']');
    }
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

/// Adds `node` to the nodes that are wanted, in the iterations `within` and for each iteration
/// of `each`, when it has a place and is not among them so.
fn add<'t>(positioned: &'t Positioned, node: &'t AstNode, within: &[(At, usize)], each: Option<(&'t AstNode, usize, usize)>, wanted: &mut Vec<Wanted<'t>>) {
  if let (Some(at), Some(kind)) = (positioned.place(node), positioned.kind(node))
    && !wanted
      .iter()
      .any(|known| known.at == at && known.within == within && known.each.map(|(_, from, to)| (from, to)) == each.map(|(_, from, to)| (from, to)))
  {
    wanted.push(Wanted {
      node,
      at,
      kind,
      within: within.to_vec(),
      each,
    });
  }
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
