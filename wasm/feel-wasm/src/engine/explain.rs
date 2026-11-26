//! `op: "explain"`: an evaluation, and how its value came about: the values of the nodes of the
//! text that the request asks for, one level at a time.
//!
//! The answer is that of `evaluate` with a `trace`. By default the trace has the root of the
//! text and its parts one level down; `expand` names nodes, by their places, whose parts are
//! wanted too. A part of a loop or a filter that is evaluated once for each item (the body, the
//! predicate) has a record for each of its first iterations, or for those that the request
//! names; a node inside such a part is named with the iteration it is in. The parts of an
//! invocation are its arguments and, when it invokes a function of the text, the body of that
//! function; a node of a body is named with the invocation that runs it. A record whose value
//! is null says where the null is from: the part that is null (`nullFrom`, which is an entry of
//! `expand`), or the node itself (`nullOrigin`); a node whose parts have no records says neither,
//! until it is expanded; with the null that is a node's own stands why (`nullCode`). The node
//! that a null is from is expanded without being named, by one more evaluation for each node
//! deeper, so one request says the whole way to the node whose own the null is. With
//! `reads` each record has the names that the text of its node reads. Each request
//! evaluates the text again, with a record of the values of those nodes only: the engine keeps
//! nothing between requests, and an answer is as long as the request asks, whatever the data is.

use super::*;
use rheon_feel_evaluator::{PartFrame, Plan, Tracing, Typing, body, parts, path_text, prepare_traced, recording};
use rheon_feel_parser::{Positioned, parse_positioned};
use std::collections::{HashMap, HashSet};

/// The most records of one answer when the request does not say: beyond the limit the trace is
/// cut and says so.
const DEFAULT_RECORDS: usize = 200;

/// The most records that a request can ask for.
const MAX_RECORDS: usize = 10_000;

/// The iterations of a loop that have a record when the request does not say which: the first.
const DEFAULT_ITERATIONS: usize = 20;

/// The most evaluations that follow a null to the node it is from: each expands the nodes that
/// the nulls of the one before are from, so a null is followed this many nodes deep, and beyond
/// it the records say what they say of a node that is not expanded.
const FOLLOW_ROUNDS: usize = 16;

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

/// A step of the way to where a node is evaluated: into an iteration of the loop or the filter
/// at a place, or into the body of the function that the invocation at a place invokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Step {
  Iteration(At, usize),
  Call(At),
}

/// A node whose value the request asks for, in one of the ways it is asked for.
struct Wanted<'t> {
  node: &'t AstNode,
  at: At,
  kind: &'static str,
  /// Where it is evaluated: the steps from the root of the text, outermost first.
  within: Vec<Step>,
  /// It is the body of this loop, wanted for each iteration from the first index up to the second.
  each: Option<(&'t AstNode, usize, usize)>,
}

/// A node, by its address, and where it is evaluated.
type Evaluated<'s> = (usize, &'s [Step]);

/// A way that a node is wanted: its place, where it is evaluated, and the iterations asked for.
type Way = (At, Vec<Step>, Option<(usize, usize)>);

/// The nodes that are wanted, each way once.
#[derive(Default)]
struct Wants<'t> {
  list: Vec<Wanted<'t>>,
  known: HashSet<Way>,
}

/// An entry of `expand`: a node by its place, where it is evaluated, and for a loop which of
/// its iterations are wanted.
#[derive(Clone)]
struct Asked {
  at: At,
  within: Vec<Step>,
  iterations: Option<(usize, usize)>,
  /// Not of the request: a node that a null is from, expanded to say where its null is from.
  followed: bool,
}

/// A node whose parts are wanted: the node, where it is evaluated, and for a loop which of its
/// iterations.
type Open<'t> = (&'t AstNode, Vec<Step>, Option<(usize, usize)>);

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

/// The node at the place `at` that is a loop or a filter (`invocation`: an invocation).
fn step_node(positioned: &Positioned, at: At, invocation: bool) -> Option<&AstNode> {
  let mut stack = vec![positioned.tree()];
  while let Some(node) = stack.pop() {
    let fits = if invocation { matches!(node, AstNode::FunctionInvocation(..)) } else { repeats(node) };
    if fits && positioned.place(node) == Some(at) {
      return Some(node);
    }
    node.for_each_child(|child| stack.push(child));
  }
  None
}

/// Where `target` stands in `tree`: whether in the body of a function definition (the innermost
/// around it), and the loops around it there, outermost first: those in whose body or predicate
/// it stands. None for a node that is not evaluated for a value (a name of a parameter, a type).
fn around<'t>(tree: &'t AstNode, target: &AstNode) -> Option<(bool, Vec<&'t AstNode>)> {
  // a walk with its own stack: the node, whether in a body, and the loops around it
  let mut stack: Vec<(&AstNode, bool, Vec<&AstNode>)> = vec![(tree, false, vec![])];
  while let Some((node, in_body, loops)) = stack.pop() {
    if std::ptr::eq(node, target) || std::ptr::eq(valued(node), target) {
      return Some((in_body, loops));
    }
    if let Some(body) = body(valued(node)) {
      stack.push((body, true, vec![]));
    }
    for (part, frame) in parts(valued(node)) {
      let mut loops = loops.clone();
      if frame == PartFrame::Iteration {
        loops.push(valued(node));
      }
      stack.push((part, in_body, loops));
    }
  }
  None
}

/// The bodies of the function definitions of `tree`, each evaluated when its function is invoked.
fn bodies(tree: &AstNode) -> Vec<&AstNode> {
  let mut found = vec![];
  let mut stack = vec![tree];
  while let Some(node) = stack.pop() {
    if let Some(body) = body(node) {
      found.push(body);
    }
    node.for_each_child(|child| stack.push(child));
  }
  found
}

pub(super) fn explain(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<String, Failure> {
  const EXPAND: &str = "'expand' must be a list of nodes, each its place [from, to] in UTF-16 units, or {\"at\": [from, to], \"within\": [{\"at\": [from, to], \"iteration\": n} or {\"at\": [from, to], \"call\": true}, …], \"iterations\": [from, to]}";
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
  let mut asked: Vec<Asked> = match req.remove("expand") {
    None | Some(Json::Null) => vec![],
    Some(Json::Array(items)) => items
      .iter()
      .map(|item| match item {
        Json::Array(_) => Ok(Asked {
          at: place(item)?,
          within: vec![],
          iterations: None,
          followed: false,
        }),
        Json::Object(fields) if fields.keys().all(|field| matches!(field.as_str(), "at" | "within" | "iterations")) => {
          let at = place(fields.get("at").unwrap_or(&Json::Null))?;
          let within = match fields.get("within") {
            None | Some(Json::Null) => vec![],
            Some(Json::Array(steps)) => steps
              .iter()
              .map(|step| match (step.get("at"), step.get("iteration").and_then(Json::as_u64), step.get("call")) {
                _ if step.as_object().is_none_or(|fields| fields.len() != 2) => fail("invalid-field", EXPAND),
                (Some(at), Some(iteration), None) => Ok(Step::Iteration(place(at)?, iteration as usize)),
                (Some(at), None, Some(Json::Bool(true))) => Ok(Step::Call(place(at)?)),
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
          Ok(Asked {
            at,
            within,
            iterations,
            followed: false,
          })
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
  let reads = match req.remove("reads") {
    None | Some(Json::Null) => false,
    Some(Json::Bool(reads)) => reads,
    Some(_) => return fail("invalid-field", "'reads' must be true or false"),
  };
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
  let bodies = bodies(root);
  // a null is followed to the node it is from: an evaluation, then one more for each node deeper
  // that a null is from, with the parts of those nodes wanted too
  let (mut rounds, mut follow, mut added) = (0, true, 0);
  loop {
    let mut wants = Wants::default();
    add(&positioned, root, &[], None, &mut wants);
    // the nodes whose parts are wanted: the node, where it is evaluated, and the iterations of it asked for
    let mut open: Vec<Open> = vec![(root, vec![], None)];
    for Asked { at, within, iterations, followed } in asked.iter().cloned() {
      let Some(node) = node_at(&positioned, at) else {
        if followed {
          continue;
        }
        return fail(
          "invalid-field",
          format!(
            "'expand' has the place {}, which is the place of no node of the text (the places are those of the records, and of the nodes of `syntax`)",
            shown(at)
          ),
        );
      };
      // the steps lead to the node: from the end, the loops around it, then for a node of a body
      // the invocation that runs the body, which the steps before lead to
      let leads = || -> Result<(), Failure> {
        let (mut reached, mut steps) = (node, within.as_slice());
        loop {
          let Some((in_body, loops)) = around(root, reached) else {
            return fail(
              "invalid-field",
              format!("'expand' has the place {}, a node that is not evaluated for a value: it has no parts", shown(at)),
            );
          };
          let expected: Vec<At> = loops.iter().filter_map(|repeat| positioned.place(repeat)).collect();
          let (before, last) = steps.split_at(steps.len().saturating_sub(expected.len()));
          let fits =
            expected.len() == loops.len() && last.len() == expected.len() && last.iter().zip(&expected).all(|(step, at)| matches!(step, Step::Iteration(found, _) if found == at));
          let site = match before.split_last() {
            Some((Step::Call(site), _)) if in_body => step_node(&positioned, *site, true),
            _ => None,
          };
          if fits
            && in_body
            && site.is_none()
            && let Some((Step::Call(site), _)) = before.split_last()
          {
            return fail(
              "invalid-field",
              format!(
                "'expand' has the node at {}, whose \"within\" has {{\"at\": {}, \"call\": true}}: expected the place of an invocation, found the place of none",
                shown(at),
                shown(*site)
              ),
            );
          }
          if !fits || (in_body && site.is_none()) || (!in_body && !before.is_empty()) {
            let mut wants: Vec<String> = expected.iter().map(|at| format!("{{\"at\": {}, \"iteration\": n}}", shown(*at))).collect();
            if in_body {
              wants.insert(0, "{\"at\": [the place of the invocation that runs the body], \"call\": true}".to_string());
            }
            let stands = match (in_body, expected.len()) {
              (false, 0) => "is in no loop and in no body of a function".to_string(),
              (false, 1) => "is evaluated once for each item of the loop around it".to_string(),
              (false, count) => format!("is evaluated once for each item of the {count} loops around it"),
              (true, 0) => "is in the body of a function".to_string(),
              (true, 1) => "is in the body of a function, and evaluated once for each item of the loop around it there".to_string(),
              (true, count) => format!("is in the body of a function, and evaluated once for each item of the {count} loops around it there"),
            };
            return fail(
              "invalid-field",
              if std::ptr::eq(reached, node) {
                format!(
                  "'expand' has the node at {}, which {stands}: expected \"within\"{} [{}], found {} of them",
                  shown(at),
                  if in_body { " to end with" } else { ":" },
                  wants.join(", "),
                  steps.len()
                )
              } else {
                format!(
                  "'expand' has the node at {}, whose \"within\" has the invocation at {}, which {stands}: expected the steps before it{} [{}], found {} of them",
                  shown(at),
                  positioned.place(reached).map_or_else(String::new, &shown),
                  if in_body { " to end with" } else { ":" },
                  wants.join(", "),
                  steps.len()
                )
              },
            );
          }
          match site {
            Some(site) => (reached, steps) = (site, &before[..before.len() - 1]),
            None => break,
          }
        }
        if iterations.is_some() && !repeats(valued(node)) {
          return fail(
            "invalid-field",
            format!("'expand' has \"iterations\" for the node at {}, which is no loop and no filter", shown(at)),
          );
        }
        Ok(())
      };
      match leads() {
        Ok(()) => {}
        // a node that a null is from and that the steps of its record do not lead to is not expanded
        Err(_) if followed => continue,
        Err(failure) => return Err(failure),
      }
      // the node itself has a record too, where it is evaluated
      add(&positioned, node, &within, None, &mut wants);
      open.push((node, within, iterations));
    }
    for (node, within, iterations) in &open {
      let node = valued(node);
      for (part, frame) in parts(node) {
        match frame {
          PartFrame::Same => add(&positioned, part, within, None, &mut wants),
          // once for each item: a record for each of the iterations asked for
          PartFrame::Iteration => {
            let (from, to) = iterations.unwrap_or((0, DEFAULT_ITERATIONS));
            add(&positioned, part, within, Some((node, from, to)), &mut wants);
          }
        }
      }
      // the body that an invocation runs: whichever of the text its function has, which only the
      // evaluation says, so every body is wanted for this invocation and one has a record
      if let (AstNode::FunctionInvocation(..), Some(site)) = (node, positioned.place(node)) {
        let mut within = within.clone();
        within.push(Step::Call(site));
        for body in &bodies {
          add(&positioned, body, &within, None, &mut wants);
        }
      }
    }
    let wanted = wants.list;
    // the numbers of the nodes and of the loops that the program records
    let mut targets: HashMap<usize, u32> = HashMap::new();
    let mut repeat_numbers: HashMap<usize, u32> = HashMap::new();
    let mut call_numbers: HashMap<usize, u32> = HashMap::new();
    // the loop or the invocation of a step, by its place
    let mut step_nodes: HashMap<(At, bool), Option<&AstNode>> = HashMap::new();
    let address = |node: &AstNode| node as *const AstNode as usize;
    let mut plan = Plan { wanted: vec![], limit };
    // the ways of a node, each with the indexes of what is wanted so: an entry of a context
    // literal and its value are one node, wanted in one way, and each has its record
    let mut ways: Vec<Vec<Vec<usize>>> = vec![];
    let number_of = |node: &AstNode, numbers: &mut HashMap<usize, u32>| -> u32 {
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
      for step in &wanted.within {
        let (at, invocation) = match step {
          Step::Iteration(at, _) => (*at, false),
          Step::Call(at) => (*at, true),
        };
        let Some(found) = *step_nodes.entry((at, invocation)).or_insert_with(|| step_node(&positioned, at, invocation)) else {
          return fail("internal", "a step of a record leads to no node");
        };
        within.push(match step {
          Step::Iteration(_, iteration) => rheon_feel_evaluator::Step::Iteration(number_of(found, &mut repeat_numbers), *iteration),
          Step::Call(_) => rheon_feel_evaluator::Step::Call(number_of(found, &mut call_numbers)),
        });
      }
      let way = rheon_feel_evaluator::Wanted {
        within,
        each: wanted.each.map(|(repeat, from, to)| (number_of(repeat, &mut repeat_numbers), from, to)),
        repeat: repeats(node).then(|| number_of(node, &mut repeat_numbers)),
      };
      match plan.wanted[target].iter().position(|known| *known == way) {
        Some(known) => ways[target][known].push(index),
        None => {
          plan.wanted[target].push(way);
          ways[target].push(vec![index]);
        }
      }
    }
    let node_number = |node: &AstNode| targets.get(&address(node)).copied();
    let repeat_number = |node: &AstNode| repeat_numbers.get(&address(node)).copied();
    let call_number = |node: &AstNode| call_numbers.get(&address(node)).copied();
    let tracing = Tracing {
      node: &node_number,
      repeat: &repeat_number,
      call: &call_number,
    };
    let evaluator = prepare_traced(positioned.tree(), language, &tracing);
    let (answer, trace) = recording(plan, || answered(&scope, None, &evaluator, error_mode, max_steps, warnings.clone(), started));
    let mut response = answer?;
    // in the order of the text, a node before the nodes inside it, an iteration before the next
    let mut order: Vec<(&Wanted, &rheon_feel_evaluator::Recorded)> = trace
      .records
      .iter()
      .flat_map(|record| {
        let wanted = &wanted;
        let of_way = ways.get(record.target as usize).and_then(|ways| ways.get(record.way));
        of_way.into_iter().flatten().map(move |index| (&wanted[*index], record))
      })
      .collect();
    // where each is evaluated: the steps to it, and for the body of a loop its iteration
    let path_of = |wanted: &Wanted, record: &rheon_feel_evaluator::Recorded| -> Vec<Step> {
      let mut within = wanted.within.clone();
      if let (Some((repeat, ..)), Some(iteration)) = (wanted.each, record.iteration)
        && let Some(at) = positioned.place(repeat)
      {
        within.push(Step::Iteration(at, iteration));
      }
      within
    };
    order.sort_by_cached_key(|(wanted, record)| (wanted.at.0, std::cmp::Reverse(wanted.at.1), path_of(wanted, record)));
    // a node that is wanted in two ways for one evaluation (as a part, and expanded) has one record
    let mut once: HashSet<(usize, Vec<Step>)> = HashSet::new();
    order.retain(|(wanted, record)| once.insert((address(wanted.node), path_of(wanted, record))));
    let paths: Vec<Vec<Step>> = order.iter().map(|(wanted, record)| path_of(wanted, record)).collect();
    // the records by their node and where it is evaluated, and those one step inside a place
    let mut found: HashMap<(usize, &[Step]), usize> = HashMap::new();
    let mut inside: HashMap<(usize, &[Step]), Vec<usize>> = HashMap::new();
    for (index, ((wanted, _), path)) in order.iter().zip(&paths).enumerate() {
      let node = address(valued(wanted.node));
      found.entry((node, path.as_slice())).or_insert(index);
      if let Some((_, before)) = path.split_last() {
        inside.entry((node, before)).or_default().push(index);
      }
    }
    let opened: HashMap<Evaluated, Option<(usize, usize)>> = open
      .iter()
      .map(|(node, within, iterations)| ((address(valued(node)), within.as_slice()), *iterations))
      .collect();
    // the node that each node is a part of, when a null name is to be led back to its entry
    let parents = std::cell::OnceCell::new();
    // where the null of the record `index` came from
    let origin = |index: usize| -> Origin {
      let ((wanted, record), path) = (order[index], paths[index].as_slice());
      let node = valued(wanted.node);
      let node_parts = parts(node);
      let invocation = matches!(node, AstNode::FunctionInvocation(..));
      // the value of a name that an entry of a context literal around it gives is that entry's
      if let AstNode::Name(name) = node {
        return match entry_of(parents.get_or_init(|| parents_of(root)), node, name) {
          Some((entry, loops)) => match (positioned.place(entry), path.len().checked_sub(loops)) {
            (Some(at), Some(steps)) => Origin::At(at, path[..steps].to_vec()),
            _ => Origin::Here,
          },
          None => Origin::Here,
        };
      }
      // and so is the value of that entry read from the literal itself
      if let AstNode::Path(base, name) = node
        && let (AstNode::Context(entries), AstNode::Name(name)) = (base.as_ref(), name.as_ref())
        && distinct(entries)
        && let Some(entry) = entries.iter().find(|entry| key_of(entry) == Some(name))
        && let Some(at) = positioned.place(entry)
      {
        return Origin::At(at, path.to_vec());
      }
      if node_parts.is_empty() && !invocation {
        return Origin::Here;
      }
      // the parts have records when the node is expanded where this record is evaluated
      let Some(iterations) = opened.get(&(address(node), path)) else {
        return Origin::Closed;
      };
      let null = |index: &usize| order[*index].1.value.is_null();
      let same = |part: &AstNode| found.get(&(address(valued(part)), path)).copied().filter(null);
      let here = if trace.truncated { Origin::Unknown } else { Origin::Here };
      if let AstNode::If(_, then, otherwise) = node {
        // the value is that of the branch taken, whatever the condition is
        return [then, otherwise].into_iter().find_map(|branch| same(branch)).map_or(here, Origin::From);
      }
      // the value of an invocation is that of the body it runs
      if invocation
        && let Some(site) = positioned.place(node)
        && let Some(body) = bodies.iter().find_map(|body| {
          let records = inside.get(&(address(body), path))?;
          records.iter().copied().find(|index| paths[*index].last() == Some(&Step::Call(site)) && null(index))
        })
      {
        return Origin::From(body);
      }
      if let Some(part) = node_parts.iter().filter(|(_, frame)| *frame == PartFrame::Same).find_map(|(part, _)| same(part)) {
        return Origin::From(part);
      }
      let mut every = true;
      for (part, _) in node_parts.iter().filter(|(_, frame)| *frame == PartFrame::Iteration) {
        let of_node = |index: &usize| matches!(paths[*index].last(), Some(Step::Iteration(at, _)) if Some(*at) == positioned.place(node));
        if let Some(found) = inside
          .get(&(address(valued(part)), path))
          .and_then(|records| records.iter().copied().find(|index| of_node(index) && null(index)))
        {
          return Origin::From(found);
        }
        // an iteration without a record can be where the null is from
        let (from, to) = iterations.unwrap_or((0, DEFAULT_ITERATIONS));
        every &= from == 0 && record.iterations.is_none_or(|count| count <= to);
      }
      if every { here } else { Origin::Unknown }
    };
    // why the null of the record `index` is its node's own
    let code = |index: usize| -> &'static str {
      let ((wanted, _), path) = (order[index], paths[index].as_slice());
      let node = valued(wanted.node);
      match node {
        AstNode::Name(name) => match given(parents.get_or_init(|| parents_of(root)), node, name) {
          Some(true) => "null-input",
          Some(false) if scope.get_value(name).is_some() => "null-input",
          Some(false) => "missing-entry",
          None => "other",
        },
        AstNode::Path(base, name) => match (found.get(&(address(valued(base)), path)).map(|base| &order[*base].1.value), name.as_ref()) {
          (Some(Value::Context(context)), AstNode::Name(name)) if context.contains_entry(name) => "null-input",
          (Some(_), _) => "missing-entry",
          (None, _) => "other",
        },
        AstNode::FunctionInvocation(..) => "function",
        AstNode::If(..) | AstNode::For(..) | AstNode::Some(..) | AstNode::Every(..) | AstNode::Context(_) | AstNode::List(_) | AstNode::FunctionDefinition(..) => "other",
        node if parts(node).is_empty() => "other",
        _ => "operator",
      }
    };
    // the nodes that a null is from and that are not expanded are expanded by one more evaluation
    if follow && !trace.truncated && rounds < FOLLOW_ROUNDS {
      let mut more: Vec<Asked> = vec![];
      for index in (0..order.len()).filter(|index| order[*index].1.value.is_null()) {
        let from = match origin(index) {
          Origin::From(part) => matches!(origin(part), Origin::Closed).then(|| (order[part].0.at, paths[part].clone())),
          // an entry of a context literal: expanded when it has no record yet, too
          Origin::At(at, steps) => {
            let entry = node_at(&positioned, at).and_then(|entry| found.get(&(address(valued(entry)), steps.as_slice())).copied());
            entry.is_none_or(|entry| matches!(origin(entry), Origin::Closed)).then_some((at, steps))
          }
          Origin::Here | Origin::Unknown | Origin::Closed => None,
        };
        if let Some((at, within)) = from
          && !asked.iter().chain(&more).any(|known| known.at == at && known.within == within)
        {
          more.push(Asked {
            at,
            within,
            iterations: None,
            followed: true,
          });
        }
      }
      if !more.is_empty() {
        (rounds, added) = (rounds + 1, more.len());
        asked.extend(more);
        continue;
      }
    }
    // the nodes expanded last made the trace longer than the request takes: the answer is the one
    // without them
    if trace.truncated && added > 0 {
      asked.truncate(asked.len() - added);
      (follow, added) = (false, 0);
      continue;
    }
    // the names that the text of a node reads, by the node
    let mut typing = Typing::new(&scope);
    let mut read: HashMap<usize, Vec<String>> = HashMap::new();
    let write_steps = |steps: &[Step], records: &mut String| {
      records.push('[');
      for (index, step) in steps.iter().enumerate() {
        if index > 0 {
          records.push(',');
        }
        match step {
          Step::Iteration(at, iteration) => records.push_str(&format!("{{\"at\":[{},{}],\"iteration\":{iteration}}}", places.of(at.0), places.of(at.1))),
          Step::Call(at) => records.push_str(&format!("{{\"at\":[{},{}],\"call\":true}}", places.of(at.0), places.of(at.1))),
        }
      }
      records.push(']');
    };
    // the records as text, the keys of each in alphabetical order as in every response
    let mut records = String::from(",\"trace\":[");
    // the records whose null is their node's own
    let mut nulls: Vec<usize> = vec![];
    for (index, (wanted, record)) in order.iter().copied().enumerate() {
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
      // a null: the part it is from, or that it is of this node
      if record.value.is_null() {
        let from = match origin(index) {
          Origin::From(part) => Some((order[part].0.at, paths[part].clone())),
          Origin::At(at, steps) => Some((at, steps)),
          Origin::Here => {
            records.push_str(&format!(",\"nullCode\":\"{}\"", code(index)));
            nulls.push(index);
            None
          }
          Origin::Unknown | Origin::Closed => None,
        };
        if let Some((at, steps)) = from {
          records.push_str(&format!(",\"nullFrom\":{{\"at\":[{},{}]", places.of(at.0), places.of(at.1)));
          if !steps.is_empty() {
            records.push_str(",\"within\":");
            write_steps(&steps, &mut records);
          }
          records.push('}');
        }
        if nulls.last() == Some(&index) {
          records.push_str(",\"nullOrigin\":true");
        }
      }
      if reads {
        let node = valued(wanted.node);
        let names = read.entry(address(node)).or_insert_with(|| {
          let found = typing.reads(node, &|node| positioned.place(node), false, false);
          found.paths.iter().map(|path| path_text(&path.segments)).collect()
        });
        records.push_str(",\"reads\":[");
        for (index, name) in names.iter().enumerate() {
          if index > 0 {
            records.push(',');
          }
          crate::output::write_string(name, &mut records);
        }
        records.push(']');
      }
      records.push_str(&format!(",\"to\":{},\"value\":", places.of(wanted.at.1)));
      write_value(&record.value, &mut records);
      if !paths[index].is_empty() {
        records.push_str(",\"within\":");
        write_steps(&paths[index], &mut records);
      }
      records.push('}');
    }
    records.push_str(if trace.truncated { "],\"traceTruncated\":true" } else { "],\"traceTruncated\":false" });
    // before the warnings, the last field of the response of an evaluation
    match response.rfind(",\"warnings\":") {
      Some(at) => response.insert_str(at, &records),
      None => return fail("internal", "the response of the evaluation has no warnings"),
    }
    return Ok(response);
  }
}

/// The name of an entry of a context literal.
fn key_of(entry: &AstNode) -> Option<&Name> {
  match entry {
    AstNode::ContextEntry(key, _) => match key.as_ref() {
      AstNode::ContextEntryKey(name) => Some(name),
      _ => None,
    },
    _ => None,
  }
}

/// Whether the entries of a context literal have each its own name: such a literal is a context.
fn distinct(entries: &[AstNode]) -> bool {
  let mut names = HashSet::new();
  entries.iter().all(|entry| key_of(entry).is_some_and(|name| names.insert(name)))
}

/// How a node is a part of the node it is in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PartOf {
  /// Evaluated once where that node is.
  Same,
  /// Evaluated once for each item that that node visits.
  Iteration,
  /// The body of that function definition.
  Body,
}

/// For each node of `tree` that is evaluated for a value, by its address: the node that it is a
/// part of, and how.
fn parents_of(tree: &AstNode) -> HashMap<usize, (&AstNode, PartOf)> {
  let address = |node: &AstNode| node as *const AstNode as usize;
  let mut parents = HashMap::new();
  let mut stack = vec![tree];
  while let Some(node) = stack.pop() {
    let node = valued(node);
    if let Some(body) = body(node) {
      parents.insert(address(body), (node, PartOf::Body));
      stack.push(body);
    }
    for (part, frame) in parts(node) {
      let how = if frame == PartFrame::Iteration { PartOf::Iteration } else { PartOf::Same };
      parents.insert(address(valued(part)), (node, how));
      stack.push(part);
    }
  }
  parents
}

/// The entry of a context literal that gives the name `name`, read at `node`, its value: the
/// nearest entry of that name before the entry that `node` is in, of the literals around it,
/// with the number of loops around `node` that are not around the entry. None when something
/// nearer can give the name (a variable of a loop, an item of a filter), and inside the body of
/// a function, which reads the names around it from where the function was made.
fn entry_of<'t>(parents: &HashMap<usize, (&'t AstNode, PartOf)>, node: &AstNode, name: &Name) -> Option<(&'t AstNode, usize)> {
  let address = |node: &AstNode| node as *const AstNode as usize;
  let (mut inner, mut loops) = (address(node), 0);
  loop {
    let (outer, how) = *parents.get(&inner)?;
    match how {
      PartOf::Body => return None,
      // the items of a loop or a filter can have the name
      PartOf::Iteration => {
        if matches!(outer, AstNode::Filter(..)) || binds(outer, name) {
          return None;
        }
        loops += 1;
      }
      PartOf::Same => {}
    }
    if let AstNode::Context(entries) = outer {
      let at = entries.iter().position(|entry| address(valued(entry)) == inner)?;
      if let Some(entry) = entries[..at].iter().find(|entry| key_of(entry) == Some(name)) {
        // a literal that has a name twice has no entries
        return distinct(entries).then_some((entry, loops));
      }
    }
    inner = address(outer);
  }
}

/// Whether something of the text around `node` gives the name `name`, read there, its value: a
/// variable of a loop, a parameter of a function, an earlier entry of a context literal. None
/// inside the predicate of a filter, whose items can have the name.
fn given(parents: &HashMap<usize, (&AstNode, PartOf)>, node: &AstNode, name: &Name) -> Option<bool> {
  let address = |node: &AstNode| node as *const AstNode as usize;
  let mut inner = address(node);
  while let Some(&(outer, how)) = parents.get(&inner) {
    match how {
      PartOf::Body => {
        if let AstNode::FunctionDefinition(parameters, _) = outer
          && let AstNode::FormalParameters(parameters) = parameters.as_ref()
          && parameters.iter().any(|parameter| match parameter {
            AstNode::FormalParameter(parameter, _) => matches!(parameter.as_ref(), AstNode::ParameterName(found) | AstNode::Name(found) if found == name),
            _ => true,
          })
        {
          return Some(true);
        }
      }
      PartOf::Iteration => {
        if matches!(outer, AstNode::Filter(..)) {
          return None;
        }
        if binds(outer, name) {
          return Some(true);
        }
      }
      PartOf::Same => {}
    }
    if let AstNode::Context(entries) = outer
      && let Some(at) = entries.iter().position(|entry| address(valued(entry)) == inner)
      && entries[..at].iter().any(|entry| key_of(entry) == Some(name))
    {
      return Some(true);
    }
    inner = address(outer);
  }
  Some(false)
}

/// Whether the loop `node` has a variable of the name `name`.
fn binds(node: &AstNode, name: &Name) -> bool {
  let (AstNode::For(contexts, _) | AstNode::Some(contexts, _) | AstNode::Every(contexts, _)) = node else {
    return false;
  };
  let (AstNode::IterationContexts(items) | AstNode::QuantifiedContexts(items)) = contexts.as_ref() else {
    return true;
  };
  items.iter().any(|item| match item {
    AstNode::IterationContextSingle(variable, _) | AstNode::IterationContextInterval(variable, ..) | AstNode::QuantifiedContext(variable, _) => {
      !matches!(variable.as_ref(), AstNode::Name(variable) if variable != name)
    }
    _ => true,
  })
}

/// Where the null of a record is from.
enum Origin {
  /// A part of the node, whose value is null: the record at this index.
  From(usize),
  /// An entry of a context literal, whose value the node reads: its place and where it is
  /// evaluated.
  At(At, Vec<Step>),
  /// The node itself: its parts are not null, or it has none.
  Here,
  /// Not said by the records: not all the parts of the node have one (an iteration that has
  /// none, a trace that is cut).
  Unknown,
  /// Not said by the records: the node is not expanded, so its parts have none.
  Closed,
}

/// Adds `node` to the nodes that are wanted, evaluated at `within` and for each iteration of
/// `each`, when it has a place and is not among them so.
fn add<'t>(positioned: &'t Positioned, node: &'t AstNode, within: &[Step], each: Option<(&'t AstNode, usize, usize)>, wants: &mut Wants<'t>) {
  if let (Some(at), Some(kind)) = (positioned.place(node), positioned.kind(node))
    && wants.known.insert((at, within.to_vec(), each.map(|(_, from, to)| (from, to))))
  {
    wants.list.push(Wanted {
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
