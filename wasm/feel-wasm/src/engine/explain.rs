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
//! that a null is from is expanded without being named, so one request says the whole way to
//! the node whose own the null is: when the evaluation of the request leaves a null at a node
//! that is not expanded, the text is evaluated once more, and that evaluation keeps the values
//! on the ways of its nulls ([rheon_feel_evaluator::Follow]); the records on the way are taken
//! from them. With
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

/// The most values that the evaluation which follows the nulls keeps at one time on the ways
/// from the nulls to the nodes they are from: with more, no null is followed.
const FOLLOW_KEPT: usize = 100_000;

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
  // a null is followed to the node it is from. The evaluation of the request says whether a
  // null is from a node whose parts have no records; then one more evaluation keeps the values
  // on the ways of the nulls, and the records on the way of each such null are taken from them
  let mut following = false;
  // the answer of the evaluation of the request, which stands when the other cannot be given
  let mut best: Option<String> = None;
  // the answer and the trace of the evaluation that the records are of
  let mut evaluated: Option<(String, rheon_feel_evaluator::Trace)> = None;
  // every node of the text with its number, for the evaluation that follows the nulls
  let mut numbering: Option<Numbering> = None;
  // the values that the evaluation kept on the ways of the nulls, each as a record, by its node
  // and where it is evaluated, and those one step inside a place
  let mut pool: Vec<(Wanted, rheon_feel_evaluator::Recorded)> = vec![];
  let mut pool_bare: Vec<bool> = vec![];
  let mut pool_at: HashMap<(usize, Vec<Step>), usize> = HashMap::new();
  let mut pool_inside: HashMap<(usize, Vec<Step>), Vec<usize>> = HashMap::new();
  // those that are records of the answer, and the nodes that are expanded with them
  let mut chosen: Vec<usize> = vec![];
  let mut followed: Vec<(usize, Vec<Step>)> = vec![];
  // the nodes that a null is from and whose parts were not all kept: not expanded
  let mut left: HashSet<(usize, Vec<Step>)> = HashSet::new();
  // the evaluations are one evaluation for the budget of steps: following a null takes steps
  // of the request's budget and never more, and where the budget ends the way ends
  let (answer, _) = diagnostics::evaluation_within(max_steps, || -> Result<String, Failure> {
    loop {
      let mut wants = Wants::default();
      add(&positioned, root, &[], None, &mut wants);
      // the nodes whose parts are wanted: the node, where it is evaluated, and the iterations of it asked for
      let mut open: Vec<Open> = vec![(root, vec![], None)];
      for Asked { at, within, iterations } in asked.iter().cloned() {
        let Some(node) = node_at(&positioned, at) else {
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
            let fits = expected.len() == loops.len()
              && last.len() == expected.len()
              && last.iter().zip(&expected).all(|(step, at)| matches!(step, Step::Iteration(found, _) if found == at));
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
        leads()?;
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
      let mut plan = Plan {
        wanted: vec![],
        limit,
        follow: None,
      };
      // the ways of a node, each with the indexes of what is wanted so: an entry of a context
      // literal and its value are one node, wanted in one way, and each has its record
      let mut ways: Vec<Vec<Vec<usize>>> = vec![];
      let number_of = |node: &AstNode, numbers: &mut HashMap<usize, u32>| -> u32 {
        let next = numbers.len() as u32;
        *numbers.entry(address(node)).or_insert(next)
      };
      // the evaluation that follows the nulls: every node has a number, every loop and every
      // invocation its own
      if following {
        let numbering = numbering.get_or_insert_with(|| Numbering::of(root, &bodies));
        for (number, node) in numbering.nodes.iter().enumerate() {
          let node = valued(node);
          targets.insert(address(node), number as u32);
          if repeats(node) {
            repeat_numbers.insert(address(node), number as u32);
          }
          if matches!(node, AstNode::FunctionInvocation(..)) {
            call_numbers.insert(address(node), number as u32);
          }
        }
        plan.wanted.resize(numbering.nodes.len(), vec![]);
        ways.resize(numbering.nodes.len(), vec![]);
        plan.follow = Some(numbering.follow.clone());
      }
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
      if evaluated.is_none() {
        let evaluator = prepare_traced(positioned.tree(), language, &tracing);
        let (answer, trace) = recording(plan, || answered(&scope, None, &evaluator, error_mode, max_steps, warnings.clone(), started));
        match answer {
          // a limit that the evaluation which follows the nulls meets (the steps left) leaves
          // the answer of the evaluation of the request, and so does a trace that is not whole
          Ok(_) if following && (trace.truncated || trace.overflow) => return best.take().ok_or_else(|| Failure::new("internal", "no answer is kept".to_string(), None)),
          Ok(response) => evaluated = Some((response, trace)),
          Err(failure) => return best.take().ok_or(failure),
        }
        // the values kept on the ways, as records
        if let (true, Some(numbering), Some((_, trace))) = (following, numbering.as_ref(), evaluated.as_ref()) {
          for kept in &trace.followed {
            let Some(node) = numbering.nodes.get(kept.target as usize).copied() else {
              continue;
            };
            let place_of = |number: u32| numbering.nodes.get(number as usize).and_then(|node| positioned.place(valued(node)));
            let within: Option<Vec<Step>> = kept
              .within
              .iter()
              .map(|step| match *step {
                rheon_feel_evaluator::Step::Iteration(repeat, index) => Some(Step::Iteration(place_of(repeat)?, index)),
                rheon_feel_evaluator::Step::Call(site) => Some(Step::Call(place_of(site)?)),
              })
              .collect();
            let (Some(at), Some(kind), Some(within)) = (positioned.place(node), positioned.kind(node), within) else {
              continue;
            };
            // a part that is evaluated for each item is of the iteration that its last step names
            let each = numbering.follow.nodes[kept.target as usize].part == rheon_feel_evaluator::FollowPart::Iteration;
            let iteration = match within.last() {
              Some(Step::Iteration(_, index)) if each => Some(*index),
              _ => None,
            };
            let key = address(valued(node));
            pool_at.entry((key, within.clone())).or_insert(pool.len());
            if let Some((_, before)) = within.split_last() {
              pool_inside.entry((key, before.to_vec())).or_default().push(pool.len());
            }
            pool_bare.push(kept.bare);
            pool.push((
              Wanted {
                node,
                at,
                kind,
                within,
                each: None,
              },
              rheon_feel_evaluator::Recorded {
                target: kept.target,
                way: 0,
                value: kept.value.clone(),
                iteration,
                iterations: kept.iterations,
              },
            ));
          }
        }
      }
      let Some((answer, trace)) = evaluated.as_ref() else {
        return fail("internal", "the evaluation has no answer");
      };
      let mut response = answer.clone();
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
      order.extend(chosen.iter().map(|index| (&pool[*index].0, &pool[*index].1)));
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
      let mut opened: HashMap<Evaluated, Option<(usize, usize)>> = open
        .iter()
        .map(|(node, within, iterations)| ((address(valued(node)), within.as_slice()), *iterations))
        .collect();
      // a node expanded with the values kept on the way of its null: of its iterations the
      // first null one was kept, whichever it was
      for (node, within) in &followed {
        opened.entry((*node, within.as_slice())).or_insert(Some((0, usize::MAX)));
      }
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
        if every {
          here
        } else if trace.truncated {
          Origin::Unknown
        } else {
          Origin::Later
        }
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
      // the nodes that a null is from and that are not expanded
      let mut more: Vec<(At, Vec<Step>)> = vec![];
      if !trace.truncated {
        for index in (0..order.len()).filter(|index| order[*index].1.value.is_null()) {
          let from = match origin(index) {
            Origin::From(part) => matches!(origin(part), Origin::Closed).then(|| (order[part].0.at, paths[part].clone())),
            // an entry of a context literal: expanded when it has no record yet, too
            Origin::At(at, steps) => {
              let entry = node_at(&positioned, at).and_then(|entry| found.get(&(address(valued(entry)), steps.as_slice())).copied());
              entry.is_none_or(|entry| matches!(origin(entry), Origin::Closed)).then_some((at, steps))
            }
            // a loop whose null can be from an iteration without a record: the first null
            // iteration is among the values kept on the ways
            Origin::Later => Some((order[index].0.at, paths[index].clone())),
            Origin::Here | Origin::Unknown | Origin::Closed => None,
          };
          if let Some(from) = from
            && !more.contains(&from)
          {
            more.push(from);
          }
        }
      }
      // they are expanded with the values that the evaluation kept on the ways: the node, when
      // it has no record, and its parts
      if following {
        let mut records: Vec<usize> = vec![];
        let mut nodes: Vec<(usize, Vec<Step>)> = vec![];
        let mut gone: Vec<(usize, Vec<Step>)> = vec![];
        // and so are the null parts among them whose own parts were kept: the way goes on
        // there, and is taken as far as it was kept
        let mut ahead: Vec<(&AstNode, Vec<Step>)> = more.drain(..).rev().filter_map(|(at, within)| Some((node_at(&positioned, at)?, within))).collect();
        while let Some((node, within)) = ahead.pop() {
          let key = (address(valued(node)), within);
          if left.contains(&key) || nodes.contains(&key) || followed.contains(&key) {
            continue;
          }
          let within = key.1.as_slice();
          let has = |node: &AstNode| found.contains_key(&(address(valued(node)), within));
          let kept = |node: &AstNode| pool_at.get(&(address(valued(node)), within.to_vec())).copied();
          let mut of_node: Vec<usize> = vec![];
          // the values inside the node were kept: its own value is asked for by the request,
          // or was kept with what is inside it
          let whole = match (has(node), kept(node)) {
            (true, _) => true,
            (false, Some(index)) if !pool_bare[index] => {
              of_node.push(index);
              true
            }
            (false, _) => false,
          };
          for (part, frame) in parts(valued(node)) {
            if positioned.place(part).is_none() {
              continue;
            }
            match frame {
              PartFrame::Same if has(part) => {}
              // a part that was not evaluated (the branch not taken) has no value
              PartFrame::Same => of_node.extend(kept(part)),
              // the iteration that was kept: the first whose value is null
              PartFrame::Iteration => {
                let of_loop = |index: &&usize| matches!(pool[**index].0.within.last(), Some(Step::Iteration(at, _)) if Some(*at) == positioned.place(valued(node)));
                let inside = pool_inside.get(&(address(valued(part)), within.to_vec()));
                of_node.extend(inside.into_iter().flatten().filter(of_loop).copied());
              }
            }
          }
          // the body that the invocation ran
          if let (AstNode::FunctionInvocation(..), Some(site)) = (valued(node), positioned.place(valued(node))) {
            for body in &bodies {
              let inside = pool_inside.get(&(address(body), within.to_vec()));
              let of_site = |index: &&usize| pool[**index].0.within.last() == Some(&Step::Call(site));
              of_node.extend(inside.into_iter().flatten().filter(of_site).copied());
            }
          }
          of_node.retain(|index| !chosen.contains(index) && !records.contains(index));
          // a node whose parts were not all kept, or whose records the trace has no room for,
          // is not expanded: its record says what that of a node that is not expanded says
          if !whole || order.len() + records.len() + of_node.len() > limit {
            gone.push(key);
            continue;
          }
          for index in &of_node {
            let (part, record) = &pool[*index];
            let inner = valued(part.node);
            if record.value.is_null() && !pool_bare[*index] && (matches!(inner, AstNode::FunctionInvocation(..)) || !parts(inner).is_empty()) {
              ahead.push((part.node, part.within.clone()));
            }
          }
          records.extend(of_node);
          nodes.push(key);
        }
        left.extend(gone);
        if !nodes.is_empty() {
          chosen.extend(records);
          followed.extend(nodes);
          continue;
        }
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
            Origin::Unknown | Origin::Closed | Origin::Later => None,
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
      if more.is_empty() || following {
        return Ok(response);
      }
      // one more evaluation, which keeps the values on the ways of the nulls
      best = Some(response);
      evaluated = None;
      following = true;
    }
  });
  answer
}

/// Every node of a text that is evaluated for a value, with its number: the root expression
/// with its parts and theirs, then each body of a function in the same way.
struct Numbering<'t> {
  /// The nodes, each at its number: as the part of the node around it (an entry of a context
  /// literal, not the node of its value).
  nodes: Vec<&'t AstNode>,
  /// What the evaluation is asked to keep on the ways of the nulls.
  follow: rheon_feel_evaluator::Follow,
}

impl<'t> Numbering<'t> {
  fn of(root: &'t AstNode, bodies: &[&'t AstNode]) -> Self {
    use rheon_feel_evaluator::{Follow, FollowNode, FollowPart};
    let address = |node: &AstNode| node as *const AstNode as usize;
    let mut nodes: Vec<&AstNode> = vec![];
    let mut table: Vec<FollowNode> = vec![];
    // a walk with its own stack, a node before its parts and the parts in the order of the text
    let mut stack: Vec<(&AstNode, Option<u32>, FollowPart, bool)> = bodies.iter().rev().map(|body| (*body, None, FollowPart::Body, true)).collect();
    stack.push((root, None, FollowPart::Root, true));
    while let Some((node, parent, part, chain)) = stack.pop() {
      let number = nodes.len() as u32;
      nodes.push(node);
      table.push(FollowNode {
        parent,
        part,
        chain,
        keeps: matches!(valued(node), AstNode::Context(_)),
        last: number,
        link: None,
      });
      let condition = matches!(valued(node), AstNode::If(..));
      let inner = parts(valued(node));
      for (index, (inner, frame)) in inner.into_iter().enumerate().rev() {
        let part = if frame == PartFrame::Iteration { FollowPart::Iteration } else { FollowPart::Same };
        // the null of an `if` is from the branch taken, never from its condition
        stack.push((inner, Some(number), part, !(condition && index == 0)));
      }
    }
    // the last node inside each: the nodes inside have the numbers after its own
    for number in (0..table.len()).rev() {
      if let Some(parent) = table[number].parent {
        table[parent as usize].last = table[parent as usize].last.max(table[number].last);
      }
    }
    // a name that an entry of a context literal around it gives its value: that entry
    let numbers: HashMap<usize, u32> = nodes.iter().enumerate().map(|(number, node)| (address(valued(node)), number as u32)).collect();
    let parents = parents_of(root);
    for (number, node) in nodes.iter().enumerate() {
      if let AstNode::Name(name) = valued(node)
        && let Some((entry, _)) = entry_of(&parents, valued(node), name)
      {
        table[number].link = numbers.get(&address(valued(entry))).copied();
      }
    }
    Numbering {
      nodes,
      follow: Follow { nodes: table, limit: FOLLOW_KEPT },
    }
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
  /// Not said by the records: the null can be from an iteration that has no record.
  Later,
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
