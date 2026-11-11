//! Request handling: `docs/CONTRACT.md` §2 (request) and §3 (response).

use crate::display::display;
use crate::input::{feel_context, json_object_to_context, json_to_value};
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics::{self, ErrorMode};
use rheon_feel::dto::ValueDto;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, Name};
use rheon_feel_parser::AstNode;
use serde_json::{Map, Value as Json, json};
use std::sync::Mutex;
use std::time::Instant;

/// The rheon repository commit this engine is built from (`build.rs`: git HEAD, `-dirty`
/// when the tree had uncommitted changes, or `RHEON_COMMIT` from the environment).
const ENGINE_COMMIT: &str = env!("RHEON_COMMIT");

/// The upstream dsntk-rs commit the engine crates are based on (`[package.metadata.rheon]` in this crate's `Cargo.toml`).
const UPSTREAM_COMMIT: &str = env!("RHEON_UPSTREAM_COMMIT");

/// Largest accepted `tzOffsetMinutes` magnitude (±18:00, the FEEL/XSD limit).
const MAX_TZ_OFFSET_MINUTES: i64 = 18 * 60;

/// Maximum depth of a parsed expression (or FEEL context literal) AST.
///
/// dsntk's tree printer, evaluator builder and evaluator are recursive; a deeper AST could
/// overflow the host stack (a trap on wasm32, at a host-dependent depth). Such expressions are
/// rejected as `syntax` (or `context`) errors instead. See CONTRACT §2.
pub(crate) const MAX_AST_DEPTH: usize = 300;

/// The local-offset override in rheon-feel-temporal is process-global; serialize requests so
/// concurrent callers (native multi-threaded tests) can not observe each other's offset.
static ENGINE_LOCK: Mutex<()> = Mutex::new(());

/// Error kinds (CONTRACT §3).
#[derive(Clone, Copy)]
enum Kind {
  Request,
  Syntax,
  Context,
  Evaluation,
  Internal,
}

impl Kind {
  fn as_str(self) -> &'static str {
    match self {
      Kind::Request => "request",
      Kind::Syntax => "syntax",
      Kind::Context => "context",
      Kind::Evaluation => "evaluation",
      Kind::Internal => "internal",
    }
  }
}

struct Failure(Kind, String);

fn fail<T>(kind: Kind, message: impl Into<String>) -> Result<T, Failure> {
  Err(Failure(kind, message.into()))
}

#[derive(Clone, Copy, PartialEq)]
enum Dialect {
  Expression,
  UnaryTests,
}

/// Handles one JSON request (as text) and returns the JSON response text.
pub fn call(request: &str) -> String {
  let _guard = ENGINE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let result = std::panic::catch_unwind(|| handle(request));
  // Never leave a request's offset behind for the next one.
  rheon_feel_temporal::set_local_offset_override(None);
  let response = match result {
    Ok(Ok(response)) => response,
    Ok(Err(Failure(kind, message))) => error_response(kind, &message),
    Err(panic) => {
      let message = panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
      error_response(Kind::Internal, &format!("engine panicked: {message}"))
    }
  };
  serde_json::to_string(&response).unwrap_or_else(|e| {
    format!(
      r#"{{"ok":false,"error":{{"kind":"internal","message":"failed to serialize response: {}"}}}}"#,
      e.to_string().replace(['"', '\\'], "'")
    )
  })
}

/// Same as [call] for raw bytes (the wasm ABI entry point).
pub fn call_bytes(request: &[u8]) -> String {
  match std::str::from_utf8(request) {
    Ok(text) => call(text),
    Err(e) => serde_json::to_string(&error_response(Kind::Request, &format!("request is not valid UTF-8: {e}"))).unwrap_or_default(),
  }
}

fn error_response(kind: Kind, message: &str) -> Json {
  json!({ "ok": false, "error": { "kind": kind.as_str(), "message": message } })
}

fn handle(request: &str) -> Result<Json, Failure> {
  let started = Instant::now();
  let request: Json = match serde_json::from_str(request) {
    Ok(json) => json,
    // Only `context` / `input` can hold nested JSON values, so a nesting limit is a context problem.
    Err(e) if e.to_string().starts_with("recursion limit exceeded") => {
      return fail(Kind::Context, format!("context or input JSON is nested too deeply (at most about 125 levels): {e}"));
    }
    Err(e) => return fail(Kind::Request, format!("malformed request JSON: {e}")),
  };
  let Json::Object(req) = request else {
    return fail(Kind::Request, "request must be a JSON object");
  };
  let op = match req.get("op") {
    Some(Json::String(op)) => op.as_str(),
    None => return fail(Kind::Request, "missing 'op'"),
    Some(_) => return fail(Kind::Request, "'op' must be a string"),
  };
  match op {
    "info" => Ok(info()),
    "evaluate" => evaluate(&req, started),
    "parse" => parse(&req, started),
    other => fail(Kind::Request, format!("unknown op '{other}' (expected 'evaluate', 'parse' or 'info')")),
  }
}

fn info() -> Json {
  json!({
    "ok": true,
    "engine": "rheon",
    "engineCommit": ENGINE_COMMIT,
    "upstreamCommit": UPSTREAM_COMMIT,
    "numberBackend": rheon_feel_number::NUMBER_BACKEND,
    "version": env!("CARGO_PKG_VERSION"),
  })
}

fn elapsed_micros(started: Instant) -> u64 {
  started.elapsed().as_micros().min(u64::MAX as u128) as u64
}

fn dialect(req: &Map<String, Json>) -> Result<Dialect, Failure> {
  match req.get("dialect") {
    None | Some(Json::Null) => Ok(Dialect::Expression),
    Some(Json::String(d)) if d == "expression" => Ok(Dialect::Expression),
    Some(Json::String(d)) if d == "unaryTests" => Ok(Dialect::UnaryTests),
    Some(other) => fail(Kind::Request, format!("invalid 'dialect' {other} (expected \"expression\" or \"unaryTests\")")),
  }
}

/// The expression language: `"feel"` (default) or `"b-feel"` (DMN 1.6 chapter 11).
fn language(req: &Map<String, Json>) -> Result<rheon_feel_evaluator::Dialect, Failure> {
  match req.get("language") {
    None | Some(Json::Null) => Ok(rheon_feel_evaluator::Dialect::Feel),
    Some(Json::String(l)) if l == "feel" => Ok(rheon_feel_evaluator::Dialect::Feel),
    Some(Json::String(l)) if l == "b-feel" => Ok(rheon_feel_evaluator::Dialect::BFeel),
    Some(other) => fail(Kind::Request, format!("invalid 'language' {other} (expected \"feel\" or \"b-feel\")")),
  }
}

/// The error handling mode: `"lenient"` (default) or `"strict"` (DMN 1.6 §7.3.8).
fn error_mode(req: &Map<String, Json>) -> Result<ErrorMode, Failure> {
  match req.get("errorMode") {
    None | Some(Json::Null) => Ok(ErrorMode::Lenient),
    Some(Json::String(m)) if m == "lenient" => Ok(ErrorMode::Lenient),
    Some(Json::String(m)) if m == "strict" => Ok(ErrorMode::Strict),
    Some(other) => fail(Kind::Request, format!("invalid 'errorMode' {other} (expected \"lenient\" or \"strict\")")),
  }
}

fn expression(req: &Map<String, Json>) -> Result<&str, Failure> {
  match req.get("expression") {
    Some(Json::String(e)) => Ok(e.as_str()),
    None => fail(Kind::Request, "missing 'expression'"),
    Some(_) => fail(Kind::Request, "'expression' must be a string"),
  }
}

/// Builds the input context from `context` / `contextFormat`.
fn context(req: &Map<String, Json>, warnings: &mut Vec<String>) -> Result<FeelContext, Failure> {
  let format = match req.get("contextFormat") {
    None | Some(Json::Null) => "json",
    Some(Json::String(f)) if f == "json" || f == "feel" => f.as_str(),
    Some(other) => return fail(Kind::Request, format!("invalid 'contextFormat' {other} (expected \"json\" or \"feel\")")),
  };
  match (format, req.get("context")) {
    (_, None | Some(Json::Null)) => Ok(FeelContext::default()),
    ("json", Some(Json::Object(map))) => json_object_to_context(map, "", warnings).map_err(|e| Failure(Kind::Context, e)),
    ("json", Some(_)) => fail(Kind::Context, "with contextFormat \"json\", 'context' must be a JSON object"),
    (_, Some(Json::String(text))) => feel_context(text).map_err(|e| Failure(Kind::Context, e)),
    (_, Some(_)) => fail(Kind::Context, "with contextFormat \"feel\", 'context' must be a string holding a FEEL context literal"),
  }
}

/// Applies `tzOffsetMinutes` (host local UTC offset) to dsntk's notion of local time.
///
/// Without it, local time is UTC on every target (wasm32 has no time zone; native builds would
/// otherwise use the host's zone and give different results for zone-less date-times).
fn apply_tz_offset(req: &Map<String, Json>) -> Result<(), Failure> {
  let offset = match req.get("tzOffsetMinutes") {
    None | Some(Json::Null) => Some(0),
    Some(Json::Number(n)) => match n.as_i64() {
      Some(m) if (-MAX_TZ_OFFSET_MINUTES..=MAX_TZ_OFFSET_MINUTES).contains(&m) => Some((m * 60) as i32),
      _ => {
        return fail(
          Kind::Request,
          format!("'tzOffsetMinutes' must be an integer in -{MAX_TZ_OFFSET_MINUTES}..={MAX_TZ_OFFSET_MINUTES}, got {n}"),
        );
      }
    },
    Some(other) => return fail(Kind::Request, format!("'tzOffsetMinutes' must be a number, got {other}")),
  };
  rheon_feel_temporal::set_local_offset_override(offset);
  Ok(())
}

/// The name under which the tested value of unary tests is bound.
fn question_mark() -> Name {
  Name::from("?")
}

/// Prepares the scope (and for unary tests binds `?`), returning warnings collected so far.
fn prepare_scope(req: &Map<String, Json>, dialect: Dialect, warnings: &mut Vec<String>) -> Result<FeelScope, Failure> {
  let mut ctx = context(req, warnings)?;
  if dialect == Dialect::UnaryTests {
    match req.get("input") {
      Some(input) => {
        let value = json_to_value(input, "input", warnings).map_err(|e| Failure(Kind::Context, e))?;
        ctx.set_entry(&question_mark(), value);
      }
      None => {
        if ctx.get_entry(&question_mark()).is_none() {
          warnings.push("no input value given ('input' or context[\"?\"]); testing null".to_string());
          ctx.set_entry(&question_mark(), Value::Null(None));
        }
      }
    }
  }
  Ok(ctx.into())
}

fn parse_node(scope: &FeelScope, dialect: Dialect, expression: &str) -> Result<AstNode, Failure> {
  let parsed = match dialect {
    Dialect::Expression => rheon_feel_parser::parse_expression(scope, expression, false),
    Dialect::UnaryTests => rheon_feel_parser::parse_unary_tests(scope, expression, false),
  };
  let node = parsed.map_err(|e| Failure(Kind::Syntax, syntax_message(&e.to_string())))?;
  check_ast_depth(&node).map_err(|e| Failure(Kind::Syntax, e))?;
  Ok(node)
}

/// dsntk prefixes parser errors with the error type (`<ParserError> syntax error: ...`);
/// the kind already says it is a syntax error, so drop that prefix.
fn syntax_message(message: &str) -> String {
  let message = message.trim();
  for prefix in ["<ParserError> ", "<LexerError> "] {
    if let Some(rest) = message.strip_prefix(prefix) {
      return rest.to_string();
    }
  }
  message.to_string()
}

/// Rejects ASTs deeper than [MAX_AST_DEPTH] (see there).
pub(crate) fn check_ast_depth(node: &AstNode) -> Result<(), String> {
  let depth = node.depth();
  if depth > MAX_AST_DEPTH {
    return Err(format!("expression is nested too deeply ({depth} levels, the limit is {MAX_AST_DEPTH})"));
  }
  Ok(())
}

/// Message of the `evaluation` error reported when the call depth limit was hit.
pub(crate) fn call_depth_message() -> String {
  format!(
    "function calls are nested too deeply (more than {} levels, e.g. runaway recursion); evaluation stopped",
    rheon_feel_evaluator::MAX_CALL_DEPTH
  )
}

/// Warns about unary tests that are boolean expressions using `?` (e.g. `? > 5`): dsntk compares
/// the input with such a test's boolean result instead of treating it as satisfied when true.
fn boolean_unary_test_warnings(node: &AstNode, warnings: &mut Vec<String>) {
  let tests: &[AstNode] = match node {
    AstNode::ExpressionList(items) | AstNode::NegatedList(items) => items,
    _ => return,
  };
  for (i, test) in tests.iter().enumerate() {
    let boolean_root = match test {
      AstNode::Eq(..)
      | AstNode::Nq(..)
      | AstNode::Lt(..)
      | AstNode::Le(..)
      | AstNode::Gt(..)
      | AstNode::Ge(..)
      | AstNode::And(..)
      | AstNode::Or(..)
      | AstNode::Between(..)
      | AstNode::In(..)
      | AstNode::InstanceOf(..) => true,
      AstNode::FunctionInvocation(callee, _) => matches!(callee.as_ref(), AstNode::Name(name) if name.as_str() == "not"),
      _ => false,
    };
    if boolean_root && uses_question_mark(test) {
      warnings.push(format!(
        "unary test #{} is a boolean expression using '?': dsntk compares the input with its result (input = true/false) \
         instead of treating it as satisfied when true (DMN 1.2+); write e.g. '> 5' instead of '? > 5'",
        i + 1
      ));
    }
  }
}

fn uses_question_mark(node: &AstNode) -> bool {
  let mut stack = vec![node];
  while let Some(node) = stack.pop() {
    if matches!(node, AstNode::Name(name) if name.as_str() == "?") {
      return true;
    }
    node.for_each_child(|child| stack.push(child));
  }
  false
}

fn evaluate(req: &Map<String, Json>, started: Instant) -> Result<Json, Failure> {
  let dialect = dialect(req)?;
  let language = language(req)?;
  let error_mode = error_mode(req)?;
  let expression = expression(req)?;
  apply_tz_offset(req)?;
  let mut warnings = vec![];
  let scope = prepare_scope(req, dialect, &mut warnings)?;
  let node = parse_node(&scope, dialect, expression)?;
  let node = match dialect {
    Dialect::Expression => node,
    Dialect::UnaryTests => {
      boolean_unary_test_warnings(&node, &mut warnings);
      AstNode::In(Box::new(AstNode::Name(question_mark())), Box::new(node))
    }
  };
  rheon_feel_evaluator::reset_call_depth();
  let evaluator = rheon_feel_evaluator::prepare_with_dialect(&node, language);
  let (value, diagnostics) = diagnostics::collect(|| evaluator(&scope));
  if rheon_feel_evaluator::call_depth_exceeded() {
    return fail(Kind::Evaluation, call_depth_message());
  }
  // strict mode (DMN 1.6 §7.3.8): the first error stops the evaluation
  if error_mode == ErrorMode::Strict
    && let Some(first) = diagnostics.first()
  {
    return fail(Kind::Evaluation, format!("strict error mode: {first}"));
  }
  Ok(json!({
    "ok": true,
    "result": result_json(&value),
    "warnings": warnings.into_iter().map(|message| json!({ "message": message })).collect::<Vec<_>>(),
    "diagnostics": { "count": diagnostics.count, "messages": diagnostics.messages },
    "elapsedMicros": elapsed_micros(started),
  }))
}

fn parse(req: &Map<String, Json>, started: Instant) -> Result<Json, Failure> {
  let dialect = dialect(req)?;
  let expression = expression(req)?;
  // The context is optional for parsing, but it tells the parser which (multi-word) names exist.
  let scope = prepare_scope(req, dialect, &mut vec![])?;
  let node = parse_node(&scope, dialect, expression)?;
  Ok(json!({
    "ok": true,
    "tree": node.to_string(),
    "elapsedMicros": elapsed_micros(started),
  }))
}

/// Builds the `result` object of a successful evaluation (CONTRACT §3).
pub(crate) fn result_json(value: &Value) -> Json {
  let typed = ValueDto::try_from(value).ok().and_then(|dto| serde_json::to_value(dto).ok()).unwrap_or(Json::Null);
  let mut result = json!({
    "display": display(value),
    "feel": value.to_string(),
    "type": value.type_of().to_string(),
    "typed": typed,
    "isNull": value.is_null(),
  });
  if let Value::Null(Some(reason)) = value {
    result["nullReason"] = Json::String(reason.clone());
  }
  result
}
