//! Request handling: `docs/CONTRACT.md` §2 (request) and §3 (response).

use crate::input::{Path, feel_context, json_into_value, json_object_into_context};
use crate::json::{self, Field, Order};
use crate::output::write_result;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics::{self, ErrorMode};
use rheon_feel::message::{arg, compose};
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, Name};
use rheon_feel_evaluator::Session;
use rheon_feel_parser::AstNode;
use serde_json::{Map, Value as Json, json};
use std::sync::Mutex;
use std::time::Instant;

mod editor;

/// The rheon repository commit this engine is built from (`build.rs`: git HEAD, `-dirty`
/// when the tree had uncommitted changes, or `RHEON_COMMIT` from the environment).
const ENGINE_COMMIT: &str = env!("RHEON_COMMIT");

/// The upstream dsntk-rs commit the engine crates are based on (`[package.metadata.rheon]` in this crate's `Cargo.toml`).
const UPSTREAM_COMMIT: &str = env!("RHEON_UPSTREAM_COMMIT");

/// Largest accepted `tzOffsetMinutes` magnitude (±18:00): the largest host offset a request
/// accepts for local time, not a FEEL limit (a FEEL time offset is within ±14:00).
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

/// A failure of a request: its kind, its message, and for a syntax error of an expression the
/// errors of the text with their places (as `syntax` answers them).
struct Failure(Kind, String, Option<Json>);

fn fail<T>(kind: Kind, message: impl Into<String>) -> Result<T, Failure> {
  Err(Failure(kind, message.into(), None))
}

#[derive(Clone, Copy, PartialEq)]
enum Dialect {
  Expression,
  UnaryTests,
  /// S-FEEL (DMN 1.7 Beta 1, clause 9.2, rule 3).
  SimpleExpression,
  /// S-FEEL rule 4: the list of the values.
  SimpleExpressions,
  /// FEEL grammar rule 3: the list of the values.
  TextualExpressions,
}

impl Dialect {
  /// The start rule of the parser for the dialect.
  fn rule(self) -> rheon_feel_parser::StartRule {
    use rheon_feel_parser::StartRule;
    match self {
      Dialect::Expression => StartRule::Expression,
      Dialect::UnaryTests => StartRule::UnaryTests,
      Dialect::SimpleExpression => StartRule::SimpleExpression,
      Dialect::SimpleExpressions => StartRule::SimpleExpressions,
      Dialect::TextualExpressions => StartRule::TextualExpressions,
    }
  }

  /// Whether the text is S-FEEL.
  fn simple(self) -> bool {
    matches!(self, Dialect::SimpleExpression | Dialect::SimpleExpressions)
  }
}

/// Handles one JSON request (as text) and returns the JSON response text.
pub fn call(request: &str) -> String {
  let _guard = ENGINE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let result = std::panic::catch_unwind(|| handle(request));
  // Never leave a request's offset or clock behind for the next one.
  rheon_feel_temporal::set_local_offset_override(None);
  rheon_feel_temporal::set_clock_override(None);
  let response = match result {
    // the response of an evaluation is written as text already
    Ok(Ok(Response::Text(text))) => return text,
    Ok(Ok(Response::Tree(response))) => response,
    Ok(Err(Failure(kind, message, diagnostics))) => error_response(kind, &message, diagnostics),
    Err(panic) => {
      let message = panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
      error_response(Kind::Internal, &format!("engine panicked: {message}"), None)
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
    Err(e) => serde_json::to_string(&error_response(Kind::Request, &format!("request is not valid UTF-8: {e}"), None)).unwrap_or_default(),
  }
}

/// The failure response; its message holds at most 1,000 characters, as every message does.
fn error_response(kind: Kind, message: &str, diagnostics: Option<Json>) -> Json {
  let mut response = json!({ "ok": false, "error": { "kind": kind.as_str(), "message": compose(format_args!("{message}")) } });
  if let Some(diagnostics) = diagnostics {
    response["error"]["diagnostics"] = diagnostics;
  }
  response
}

/// The `context` and the `input` of a request whose JSON is an object or an array, when the
/// strict reader took the request ([json::request]): read into values straight from the text,
/// or still text when the reader refused them. The request then holds neither field.
#[derive(Default)]
struct Unread<'a> {
  context: Option<Field<'a, FeelContext>>,
  input: Option<Field<'a, Value>>,
  /// The text of the array `entries`, whose items are read one by one.
  entries: Option<&'a str>,
  /// When the request was read as a tree of JSON values: the order in which the keys of its
  /// `context` and of its `input` are written, which the tree does not hold.
  context_order: Option<Order>,
  input_order: Option<Order>,
}

/// Reads the request into its fields: with the strict reader when it takes the text, which
/// reads the JSON of `context` and `input` into values on the way, and otherwise as a tree of
/// JSON values, which also gives every error of a text that is no request.
fn request_fields(request: &str) -> Result<(Map<String, Json>, Unread<'_>), Failure> {
  if let Some(read) = json::request(request) {
    let fields: Option<Map<String, Json>> = read.fields.iter().map(|(key, text)| json::tree(text).ok().map(|value| (key.to_string(), value))).collect();
    if let Some(req) = fields {
      return Ok((
        req,
        Unread {
          context: read.context,
          input: read.input,
          entries: read.entries,
          context_order: None,
          input_order: None,
        },
      ));
    }
  }
  let text = request;
  let request = match json::tree(request) {
    Ok(json) => json,
    // Only `context` / `input` can hold nested JSON values, so a nesting limit is a context problem.
    Err(e) if e.to_string().starts_with("recursion limit exceeded") => {
      return fail(Kind::Context, format!("context or input JSON is nested too deeply (at most about 125 levels): {e}"));
    }
    Err(e) => return fail(Kind::Request, format!("malformed request JSON: {e}")),
  };
  match request {
    Json::Object(req) => {
      let order = |field: &str| {
        req
          .get(field)
          .filter(|value| value.is_object() || value.is_array())
          .and_then(|_| Order::read(text)?.field(field))
      };
      let unread = Unread {
        context_order: order("context"),
        input_order: order("input"),
        ..Unread::default()
      };
      Ok((req, unread))
    }
    _ => fail(Kind::Request, "request must be a JSON object"),
  }
}

/// Puts the JSON `text` of the field `field`, which the strict reader refused, into the request
/// as a tree of JSON values, for the conversion that gives its error or its value.
fn into_tree(req: &mut Map<String, Json>, field: &str, text: &str) -> Result<(), Failure> {
  match json::tree(text) {
    Ok(tree) => {
      req.insert(field.to_string(), tree);
      Ok(())
    }
    Err(e) => fail(Kind::Request, format!("malformed request JSON: {e}")),
  }
}

/// A response: as a tree of JSON values, or as its text.
enum Response {
  Tree(Json),
  Text(String),
}

fn handle(request: &str) -> Result<Response, Failure> {
  let started = Instant::now();
  let (req, unread) = request_fields(request)?;
  enum Op {
    Info,
    Evaluate,
    EvaluateAll,
    EvaluateEach,
    Parse,
    Syntax,
    Complete,
    Outline,
    Format,
  }
  let op = match req.get("op") {
    Some(Json::String(op)) => match op.as_str() {
      "info" => Op::Info,
      "evaluate" => Op::Evaluate,
      "evaluateAll" => Op::EvaluateAll,
      "evaluateEach" => Op::EvaluateEach,
      "parse" => Op::Parse,
      "syntax" => Op::Syntax,
      "complete" => Op::Complete,
      "outline" => Op::Outline,
      "format" => Op::Format,
      other => {
        return fail(
          Kind::Request,
          format!(
            "unknown op '{}' (expected 'evaluate', 'evaluateAll', 'evaluateEach', 'parse', 'syntax', 'complete', 'outline', 'format' or 'info')",
            arg(other)
          ),
        );
      }
    },
    None => return fail(Kind::Request, "missing 'op'"),
    Some(_) => return fail(Kind::Request, "'op' must be a string"),
  };
  // the request is handed over: its context and input are converted by value
  match op {
    Op::Info => Ok(Response::Tree(info())),
    Op::Evaluate => evaluate(req, unread, started).map(Response::Text),
    Op::EvaluateAll => evaluate_all(req, unread, started).map(Response::Text),
    Op::EvaluateEach => evaluate_each(req, unread, started).map(Response::Text),
    Op::Parse => parse(req, unread, started).map(Response::Tree),
    Op::Syntax => editor::syntax(req, unread, started).map(Response::Tree),
    Op::Complete => editor::complete(req, unread, started).map(Response::Tree),
    Op::Outline => editor::outline(req, unread, started).map(Response::Tree),
    Op::Format => editor::format(req, unread, started).map(Response::Tree),
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
    Some(Json::String(d)) if d == "simpleExpression" => Ok(Dialect::SimpleExpression),
    Some(Json::String(d)) if d == "simpleExpressions" => Ok(Dialect::SimpleExpressions),
    Some(Json::String(d)) if d == "textualExpressions" => Ok(Dialect::TextualExpressions),
    Some(other) => fail(
      Kind::Request,
      format!(
        "invalid 'dialect' {} (expected \"expression\", \"unaryTests\", \"simpleExpression\", \"simpleExpressions\" or \"textualExpressions\")",
        arg(other)
      ),
    ),
  }
}

/// The expression language: `"feel"` (default) or `"b-feel"` (DMN 1.6 chapter 11).
fn language(req: &Map<String, Json>) -> Result<rheon_feel_evaluator::Dialect, Failure> {
  match req.get("language") {
    None | Some(Json::Null) => Ok(rheon_feel_evaluator::Dialect::Feel),
    Some(Json::String(l)) if l == "feel" => Ok(rheon_feel_evaluator::Dialect::Feel),
    Some(Json::String(l)) if l == "b-feel" => Ok(rheon_feel_evaluator::Dialect::BFeel),
    Some(other) => fail(Kind::Request, format!("invalid 'language' {} (expected \"feel\" or \"b-feel\")", arg(other))),
  }
}

/// The error handling mode: `"lenient"` (default) or `"strict"` (DMN 1.6 §7.3.8).
fn error_mode(req: &Map<String, Json>) -> Result<ErrorMode, Failure> {
  match req.get("errorMode") {
    None | Some(Json::Null) => Ok(ErrorMode::Lenient),
    Some(Json::String(m)) if m == "lenient" => Ok(ErrorMode::Lenient),
    Some(Json::String(m)) if m == "strict" => Ok(ErrorMode::Strict),
    Some(other) => fail(Kind::Request, format!("invalid 'errorMode' {} (expected \"lenient\" or \"strict\")", arg(other))),
  }
}

/// The step budget of the evaluation: `maxSteps`, a positive integer, or none.
fn max_steps(req: &Map<String, Json>) -> Result<Option<u64>, Failure> {
  match req.get("maxSteps") {
    None | Some(Json::Null) => Ok(None),
    Some(Json::Number(steps)) if steps.as_u64().is_some_and(|steps| steps > 0) => Ok(steps.as_u64()),
    Some(other) => fail(Kind::Request, format!("invalid 'maxSteps' {} (expected a positive integer)", arg(other))),
  }
}

/// Takes the expression out of the request.
fn expression(req: &mut Map<String, Json>) -> Result<String, Failure> {
  match req.remove("expression") {
    Some(Json::String(e)) => Ok(e),
    None => fail(Kind::Request, "missing 'expression'"),
    Some(_) => fail(Kind::Request, "'expression' must be a string"),
  }
}

/// Builds the input context from `context` / `contextFormat`, taking the context out of the request.
fn context(req: &mut Map<String, Json>, unread: Option<Field<FeelContext>>, mut order: Option<Order>, warnings: &mut Vec<String>) -> Result<FeelContext, Failure> {
  const FEEL_FORMAT: &str = "with contextFormat \"feel\", 'context' must be a string holding a FEEL context literal";
  // a &'static str, so that the context can be taken out of the request below
  let format = match req.get("contextFormat") {
    None | Some(Json::Null) => "json",
    Some(Json::String(f)) if f == "json" => "json",
    Some(Json::String(f)) if f == "feel" => "feel",
    Some(other) => return fail(Kind::Request, format!("invalid 'contextFormat' {} (expected \"json\" or \"feel\")", arg(other))),
  };
  match unread {
    Some(Field::Read(context, read_warnings)) if format == "json" => {
      warnings.extend(read_warnings);
      return Ok(context);
    }
    // an object where the text of a FEEL context is expected
    Some(Field::Read(..)) => return fail(Kind::Context, FEEL_FORMAT),
    Some(Field::Text(text)) => {
      into_tree(req, "context", text)?;
      order = Order::read(text);
    }
    None => {}
  }
  match (format, req.remove("context")) {
    (_, None | Some(Json::Null)) => Ok(FeelContext::default()),
    ("json", Some(Json::Object(map))) => {
      let context = json_object_into_context(map, &Path::Root(""), warnings).map_err(|e| Failure(Kind::Context, e, None))?;
      // the tree keeps the keys of an object in their own order: the context is told the written one
      Ok(match order {
        Some(order) => order.of_context(context),
        None => context,
      })
    }
    ("json", Some(_)) => fail(Kind::Context, "with contextFormat \"json\", 'context' must be a JSON object"),
    (_, Some(Json::String(text))) => feel_context(&text, warnings).map_err(|e| Failure(Kind::Context, e, None)),
    (_, Some(_)) => fail(Kind::Context, FEEL_FORMAT),
  }
}

/// Applies `tzOffsetMinutes` (host local UTC offset) to dsntk's notion of local time, and `now`
/// (a fixed instant) to the clock of `now()` and `today()`, for this request.
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
          format!(
            "'tzOffsetMinutes' must be an integer in -{MAX_TZ_OFFSET_MINUTES}..={MAX_TZ_OFFSET_MINUTES}, got {}",
            arg(n.as_str())
          ),
        );
      }
    },
    Some(other) => return fail(Kind::Request, format!("'tzOffsetMinutes' must be a number, got {}", arg(other))),
  };
  rheon_feel_temporal::set_local_offset_override(offset);
  // `now`: the instant that now() and today() read for this request (DMN 1.7 Beta 1, 10.3.4.11:
  // a fixed clock for repeatable evaluations); without it, the system clock
  let now = match req.get("now") {
    None | Some(Json::Null) => None,
    Some(Json::String(text)) => match rheon_feel_temporal::parse_instant(text) {
      Some(instant) => Some(instant),
      None => {
        return fail(
          Kind::Request,
          format!(
            "'now' must be an ISO 8601 instant with its offset or Z, like \"2026-01-01T10:00:00Z\", got {}",
            arg(text.as_str())
          ),
        );
      }
    },
    Some(other) => return fail(Kind::Request, format!("'now' must be a string, got {}", arg(other))),
  };
  rheon_feel_temporal::set_clock_override(now);
  Ok(())
}

/// The name under which the tested value of unary tests is bound.
fn question_mark() -> Name {
  Name::from("?")
}

/// Prepares the scope (and for unary tests binds `?`), returning warnings collected so far.
/// Takes the rest of the request: it is not needed any more.
fn prepare_scope(req: Map<String, Json>, unread: Unread, dialect: Dialect, warnings: &mut Vec<String>) -> Result<FeelScope, Failure> {
  prepare_context(req, unread, dialect, warnings).map(FeelScope::from)
}

/// The context that an expression of `dialect` is evaluated in: the `context` of the request,
/// for unary tests with the tested value as its entry `?`.
fn prepare_context(mut req: Map<String, Json>, unread: Unread, dialect: Dialect, warnings: &mut Vec<String>) -> Result<FeelContext, Failure> {
  let mut ctx = context(&mut req, unread.context, unread.context_order, warnings)?;
  let mut input_order = unread.input_order;
  if dialect == Dialect::UnaryTests {
    let read = match unread.input {
      Some(Field::Read(value, read_warnings)) => {
        warnings.extend(read_warnings);
        Some(value)
      }
      Some(Field::Text(text)) => {
        into_tree(&mut req, "input", text)?;
        input_order = Order::read(text);
        None
      }
      None => None,
    };
    let input = match read {
      Some(value) => Some(value),
      None => match req.remove("input") {
        Some(input) => {
          let value = json_into_value(input, &Path::Root("input"), warnings).map_err(|e| Failure(Kind::Context, e, None))?;
          Some(match &input_order {
            Some(order) => order.of(value),
            None => value,
          })
        }
        None => None,
      },
    };
    match input {
      Some(value) => {
        ctx.insert(question_mark(), value);
      }
      None => {
        if ctx.get_entry(&question_mark()).is_none() {
          warnings.push("no input value given ('input' or context[\"?\"]); testing null".to_string());
          ctx.insert(question_mark(), Value::Null(None));
        }
      }
    }
  }
  Ok(ctx)
}

fn parse_node(scope: &FeelScope, dialect: Dialect, expression: &str) -> Result<AstNode, Failure> {
  parse_node_sharing(scope, None, dialect, expression)
}

/// [parse_node] with the indexes of the names of the contexts of `scope`, when they are kept for
/// the parses of one request.
fn parse_node_sharing(scope: &FeelScope, shared: Option<&rheon_feel_parser::Shared>, dialect: Dialect, expression: &str) -> Result<AstNode, Failure> {
  let parsed = match shared {
    None => rheon_feel_parser::parse_start(scope, dialect.rule(), expression),
    Some(shared) => shared.parse_start(scope, dialect.rule(), expression),
  };
  // the message is the parser's; where and what the errors are comes from the tolerant parse
  let node = parsed.map_err(|e| {
    let diagnostics = editor::diagnostics(scope, expression, dialect.rule());
    Failure(Kind::Syntax, syntax_message(&e.to_string()), Some(diagnostics))
  })?;
  check_ast_depth(&node).map_err(|e| Failure(Kind::Syntax, e, None))?;
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

/// Evaluates the request and returns the text of its response: the result is written straight
/// from its value ([write_result]), and the small parts around it as `serde_json` writes them,
/// with the keys in alphabetical order as in every response.
fn evaluate(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<String, Failure> {
  let dialect = dialect(&req)?;
  let language = language(&req)?;
  let error_mode = error_mode(&req)?;
  let max_steps = max_steps(&req)?;
  let expression = expression(&mut req)?;
  apply_tz_offset(&req)?;
  let mut warnings = unknown_fields(&req);
  let scope = prepare_scope(req, unread, dialect, &mut warnings)?;
  let evaluation = Evaluation {
    id: None,
    dialect,
    language,
    error_mode,
    max_steps,
    expression,
  };
  evaluation.run(&scope, None, warnings, started)
}

/// Evaluates several expressions over one context (`op: "evaluateAll"`): the context is read
/// once, and each entry of `evaluations` is answered as an `evaluate` request with that context
/// answers it, in the order of the entries. An entry that fails is an error in its place and
/// the others are answered; a failure of the request itself (a context that does not convert,
/// a field of the request with a value that is not allowed) fails the request. The warnings of
/// reading the context are those of the request (`contextWarnings`), not of an entry. Nothing
/// of the request is kept after it.
fn evaluate_all(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<String, Failure> {
  let language = language(&req)?;
  apply_tz_offset(&req)?;
  let evaluations = match req.remove("evaluations") {
    Some(Json::Array(evaluations)) => evaluations,
    None => return fail(Kind::Request, "missing 'evaluations'"),
    Some(_) => return fail(Kind::Request, "'evaluations' must be an array"),
  };
  let mut context_warnings = unknown_fields(&req);
  let context = context(&mut req, unread.context, unread.context_order, &mut context_warnings)?;
  // unary tests without an input take the entry `?` of the context, when it has one
  let has_question_mark = context.get_entry(&question_mark()).is_some();
  // the context is held here and shared with the scope, so that the index of its names is made
  // once for the parses of all evaluations
  let contexts = [std::sync::Arc::new(context)];
  let scope = FeelScope::new();
  scope.push_shared(contexts[0].clone());
  let text_of = |tree: Json| serde_json::to_string(&tree).or_else(|e| fail(Kind::Internal, format!("failed to serialize response: {e}")));
  let results = rheon_feel_parser::sharing(&contexts, |shared| -> Result<String, Failure> {
    let mut results = String::from("[");
    for (index, evaluation) in evaluations.into_iter().enumerate() {
      if index > 0 {
        results.push(',');
      }
      // the `id` of an entry stands in its answer, also when the entry fails
      let id = evaluation.get("id").filter(|id| id.is_string() || id.is_number()).cloned();
      match evaluate_one(evaluation, index, &scope, shared, language, has_question_mark) {
        Ok(response) => results.push_str(&response),
        Err(Failure(kind, message, diagnostics)) => {
          let mut error = error_response(kind, &message, diagnostics);
          if let (Some(id), Some(fields)) = (id, error.as_object_mut()) {
            fields.insert("id".to_string(), id);
          }
          results.push_str(&text_of(error)?);
        }
      }
    }
    results.push(']');
    Ok(results)
  })?;
  let context_warnings = text_of(Json::Array(
    context_warnings
      .into_iter()
      .map(|message| json!({ "message": compose(format_args!("{message}")) }))
      .collect(),
  ))?;
  let mut response = String::with_capacity(results.len() + context_warnings.len() + 80);
  response.push_str("{\"contextWarnings\":");
  response.push_str(&context_warnings);
  response.push_str(",\"elapsedMicros\":");
  response.push_str(&elapsed_micros(started).to_string());
  response.push_str(",\"ok\":true,\"results\":");
  response.push_str(&results);
  response.push('}');
  Ok(response)
}

/// The JSON text of the `id` of an entry, which its answer repeats.
fn id_text(entry: &Map<String, Json>) -> Result<Option<String>, Failure> {
  match entry.get("id") {
    None | Some(Json::Null) => Ok(None),
    Some(id @ (Json::String(_) | Json::Number(_))) => serde_json::to_string(id)
      .map(Some)
      .or_else(|e| fail(Kind::Internal, format!("failed to serialize response: {e}"))),
    Some(other) => fail(Kind::Request, format!("'id' must be a string or a number, got {}", arg(other))),
  }
}

/// An item of `entries`: its text when the strict reader took the request, or its tree.
enum Entry<'a> {
  Text(&'a str),
  Tree(Json),
}

/// An expression that was parsed in a context, with what of the context its parse depends on.
struct Prepared {
  names: Vec<Option<Name>>,
  evaluator: rheon_feel::Evaluator,
}

/// Contexts are compared for their names down to this depth; a deeper one is parsed in anew.
const NAMES_DEPTH: usize = 8;

/// Appends to `names` what the parse of a text in `context` depends on: its names in their
/// order and, after the names of a context that is the value of one, `None`. `false` when the
/// context holds a value whose names take more to compare (a list, a function, a type, a
/// range) or is nested deeper than [NAMES_DEPTH]: a text is then parsed in that context itself.
/// Two contexts that give the same names are the same to a parser, which reads the names in
/// scope and never a number, a string, a boolean, a date, a time, a duration or a null.
fn parse_names(context: &FeelContext, depth: usize, names: &mut Vec<Option<Name>>) -> bool {
  if depth > NAMES_DEPTH {
    return false;
  }
  for (name, value) in context.iter() {
    names.push(Some(name.clone()));
    match value {
      Value::Null(_)
      | Value::Number(_)
      | Value::String(_)
      | Value::Boolean(_)
      | Value::Date(_)
      | Value::Time(_)
      | Value::DateTime(_)
      | Value::YearsAndMonthsDuration(_)
      | Value::DaysAndTimeDuration(_) => {}
      Value::Context(inner) => {
        if !parse_names(inner, depth + 1, names) {
          return false;
        }
        names.push(None);
      }
      _ => return false,
    }
  }
  true
}

/// The fields of a request and of an entry of `evaluations` or `entries`, whatever operation
/// reads them.
const FIELDS: [&str; 19] = [
  "op",
  "dialect",
  "expression",
  "context",
  "contextFormat",
  "input",
  "tzOffsetMinutes",
  "now",
  "language",
  "errorMode",
  "maxSteps",
  "evaluations",
  "entries",
  "id",
  "cursor",
  "outline",
  "tree",
  "types",
  "width",
];

/// A warning for each field of `fields` that a request does not have: it is not read, and a
/// misspelt `errorMode` or `maxSteps` must not go unseen.
fn unknown_fields(fields: &Map<String, Json>) -> Vec<String> {
  fields
    .keys()
    .filter(|name| !FIELDS.contains(&name.as_str()))
    .map(|name| {
      compose(format_args!(
        "expected a field of a request ({}), found '{}', which is not read",
        FIELDS.join(", "),
        arg(name.as_str())
      ))
    })
    .collect()
}

/// Evaluates one expression over several contexts (`op: "evaluateEach"`): each entry of
/// `entries` is answered as an `evaluate` request with the expression and the entry's `context`
/// and `input` answers it, in the order of the entries. The expression is parsed in the context
/// of an entry and that parse serves the entries after it whose contexts have the same names
/// ([parse_names]); for another context it is parsed again. An entry that fails is an error in
/// its place and the others are answered; a failure of the request itself fails the request.
/// Nothing of the request is kept after it.
fn evaluate_each(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<String, Failure> {
  let request_warnings = unknown_fields(&req);
  let dialect = dialect(&req)?;
  let language = language(&req)?;
  let error_mode = error_mode(&req)?;
  let max_steps = max_steps(&req)?;
  let expression = expression(&mut req)?;
  apply_tz_offset(&req)?;
  // the format of the contexts is that of the request
  let format = match req.get("contextFormat") {
    None | Some(Json::Null) => None,
    Some(Json::String(f)) if f == "json" || f == "feel" => Some(Json::String(f.clone())),
    Some(other) => return fail(Kind::Request, format!("invalid 'contextFormat' {} (expected \"json\" or \"feel\")", arg(other))),
  };
  let entries: Vec<Entry> = match (unread.entries, req.remove("entries")) {
    (Some(text), _) => match json::elements(text) {
      Some(items) => items.into_iter().map(Entry::Text).collect(),
      None => return fail(Kind::Internal, "the items of 'entries' could not be read"),
    },
    (None, Some(Json::Array(items))) => items.into_iter().map(Entry::Tree).collect(),
    (None, None) => return fail(Kind::Request, "missing 'entries'"),
    (None, Some(_)) => return fail(Kind::Request, "'entries' must be an array"),
  };
  let text_of = |tree: Json| serde_json::to_string(&tree).or_else(|e| fail(Kind::Internal, format!("failed to serialize response: {e}")));
  let mut prepared: Option<Prepared> = None;
  let mut names = vec![];
  let mut results = String::from("[");
  for (index, entry) in entries.into_iter().enumerate() {
    if index > 0 {
      results.push(',');
    }
    let entry_started = Instant::now();
    let fields = match entry {
      Entry::Text(text) if text.starts_with('{') => request_fields(text),
      Entry::Tree(Json::Object(fields)) => Ok((fields, Unread::default())),
      _ => fail(Kind::Request, format!("'entries[{index}]' must be an object")),
    };
    // the `id` of an entry stands in its answer, also when the entry fails
    let id = fields
      .as_ref()
      .ok()
      .and_then(|(fields, _)| fields.get("id").filter(|id| id.is_string() || id.is_number()).cloned());
    let answer = fields.and_then(|(mut fields, unread)| {
      let id = id_text(&fields)?;
      match &format {
        Some(format) => fields.insert("contextFormat".to_string(), format.clone()),
        None => fields.remove("contextFormat"),
      };
      // the request has no warnings of its own: those of its fields stand in every answer
      let mut warnings = request_warnings.clone();
      warnings.extend(unknown_fields(&fields));
      let context = prepare_context(fields, unread, dialect, &mut warnings)?;
      names.clear();
      let comparable = parse_names(&context, 0, &mut names);
      let scope: FeelScope = context.into();
      if comparable && prepared.as_ref().is_some_and(|prepared| prepared.names == names) {
        let evaluator = &prepared.as_ref().expect("checked above").evaluator;
        return answered(&scope, id, evaluator, error_mode, max_steps, warnings, entry_started);
      }
      let evaluator = program(parse_node(&scope, dialect, &expression)?, dialect, language);
      if comparable {
        let kept = prepared.insert(Prepared {
          names: std::mem::take(&mut names),
          evaluator,
        });
        answered(&scope, id, &kept.evaluator, error_mode, max_steps, warnings, entry_started)
      } else {
        answered(&scope, id, &evaluator, error_mode, max_steps, warnings, entry_started)
      }
    });
    match answer {
      Ok(response) => results.push_str(&response),
      Err(Failure(kind, message, diagnostics)) => {
        let mut error = error_response(kind, &message, diagnostics);
        if let (Some(id), Some(fields)) = (id, error.as_object_mut()) {
          fields.insert("id".to_string(), id);
        }
        results.push_str(&text_of(error)?);
      }
    }
  }
  results.push(']');
  let mut response = String::with_capacity(results.len() + 64);
  response.push_str("{\"elapsedMicros\":");
  response.push_str(&elapsed_micros(started).to_string());
  response.push_str(",\"ok\":true,\"results\":");
  response.push_str(&results);
  response.push('}');
  Ok(response)
}

/// Answers one entry of `evaluations` in the scope of the request's context. For unary tests
/// the tested value is bound to `?` in a context of its own on top of the scope, which is
/// taken off again, so that the next entry sees the context as it was read.
fn evaluate_one(
  evaluation: Json,
  index: usize,
  scope: &FeelScope,
  shared: &rheon_feel_parser::Shared,
  language: rheon_feel_evaluator::Dialect,
  has_question_mark: bool,
) -> Result<String, Failure> {
  let shared = Some(shared);
  let started = Instant::now();
  let Json::Object(mut entry) = evaluation else {
    return fail(Kind::Request, format!("'evaluations[{index}]' must be an object"));
  };
  let id = match entry.get("id") {
    None | Some(Json::Null) => None,
    Some(id @ (Json::String(_) | Json::Number(_))) => Some(serde_json::to_string(id).or_else(|e| fail(Kind::Internal, format!("failed to serialize response: {e}")))?),
    Some(other) => return fail(Kind::Request, format!("'id' must be a string or a number, got {}", arg(other))),
  };
  let dialect = dialect(&entry)?;
  let error_mode = error_mode(&entry)?;
  let max_steps = max_steps(&entry)?;
  let expression = expression(&mut entry)?;
  let mut warnings = unknown_fields(&entry);
  let tested = if dialect == Dialect::UnaryTests {
    match entry.remove("input") {
      Some(input) => Some(json_into_value(input, &Path::Root("input"), &mut warnings).map_err(|e| Failure(Kind::Context, e, None))?),
      None if has_question_mark => None,
      None => {
        warnings.push("no input value given ('input' or context[\"?\"]); testing null".to_string());
        Some(Value::Null(None))
      }
    }
  } else {
    None
  };
  let evaluation = Evaluation {
    id,
    dialect,
    language,
    error_mode,
    max_steps,
    expression,
  };
  let depth = scope.depth();
  if let Some(value) = tested {
    let mut frame = FeelContext::default();
    frame.insert(question_mark(), value);
    scope.push(frame);
  }
  let answer = evaluation.run(scope, shared, warnings, started);
  // whatever the evaluation left on the scope goes with the context of the tested value
  scope.truncate(depth);
  answer
}

/// What one evaluation is asked to do, apart from its scope.
struct Evaluation {
  /// The JSON text of the `id` of an entry of `evaluateAll`, which its answer repeats.
  id: Option<String>,
  dialect: Dialect,
  language: rheon_feel_evaluator::Dialect,
  error_mode: ErrorMode,
  max_steps: Option<u64>,
  expression: String,
}

impl Evaluation {
  /// Parses and evaluates the expression in `scope` and returns the text of the response.
  /// `warnings` are those of the input, which come before the warnings of the evaluation;
  /// `started` is when the time of the response began.
  fn run(self, scope: &FeelScope, shared: Option<&rheon_feel_parser::Shared>, warnings: Vec<String>, started: Instant) -> Result<String, Failure> {
    let Evaluation {
      id,
      dialect,
      language,
      error_mode,
      max_steps,
      expression,
    } = self;
    let node = parse_node_sharing(scope, shared, dialect, &expression)?;
    evaluated(scope, id, dialect, language, error_mode, max_steps, node, warnings, started)
  }
}

/// Evaluates a parsed expression in `scope` and returns the text of the response.
#[allow(clippy::too_many_arguments)]
fn evaluated(
  scope: &FeelScope,
  id: Option<String>,
  dialect: Dialect,
  language: rheon_feel_evaluator::Dialect,
  error_mode: ErrorMode,
  max_steps: Option<u64>,
  node: AstNode,
  warnings: Vec<String>,
  started: Instant,
) -> Result<String, Failure> {
  let evaluator = program(node, dialect, language);
  answered(scope, id, &evaluator, error_mode, max_steps, warnings, started)
}

/// The evaluator of a parsed expression; of unary tests, the test of the value `?`.
fn program(node: AstNode, dialect: Dialect, language: rheon_feel_evaluator::Dialect) -> rheon_feel::Evaluator {
  let node = match dialect {
    Dialect::UnaryTests => AstNode::In(Box::new(AstNode::Name(question_mark())), Box::new(node)),
    _ => node,
  };
  rheon_feel_evaluator::prepare_with_dialect(&node, language)
}

/// Evaluates `evaluator` in `scope` and returns the text of the response.
fn answered(
  scope: &FeelScope,
  id: Option<String>,
  evaluator: &rheon_feel::Evaluator,
  error_mode: ErrorMode,
  max_steps: Option<u64>,
  mut warnings: Vec<String>,
  started: Instant,
) -> Result<String, Failure> {
  // an evaluation stopped by an implementation limit has no result, only the abort
  let session = match max_steps {
    Some(steps) => Session::new().with_step_budget(steps),
    None => Session::new(),
  };
  let (value, diagnostics) = session
    .run(|| diagnostics::collect(|| evaluator(scope)))
    .into_result()
    .or_else(|abort| fail(Kind::Evaluation, abort.to_string()))?;
  // strict mode (DMN 1.6 §7.3.8): the first error stops the evaluation
  if error_mode == ErrorMode::Strict
    && let Some(first) = diagnostics.first()
  {
    return fail(Kind::Evaluation, compose(format_args!("strict error mode: {first}")));
  }
  // the warnings of the input and the unary tests come first, then those of the evaluation
  warnings.extend_from_slice(diagnostics.warnings());
  let text_of = |tree: Json| serde_json::to_string(&tree).or_else(|e| fail(Kind::Internal, format!("failed to serialize response: {e}")));
  let mut result = String::new();
  write_result(&value, &mut result);
  let warnings = text_of(Json::Array(
    warnings.into_iter().map(|message| json!({ "message": compose(format_args!("{message}")) })).collect(),
  ))?;
  let diagnostics = text_of(json!({ "count": diagnostics.count, "messages": diagnostics.messages }))?;
  // the time includes the writing of the result, as it did when the result was a tree
  let elapsed = elapsed_micros(started);
  let mut response = String::with_capacity(result.len() + warnings.len() + diagnostics.len() + 80);
  response.push_str("{\"diagnostics\":");
  response.push_str(&diagnostics);
  response.push_str(",\"elapsedMicros\":");
  response.push_str(&elapsed.to_string());
  if let Some(id) = id {
    response.push_str(",\"id\":");
    response.push_str(&id);
  }
  response.push_str(",\"ok\":true,\"result\":");
  response.push_str(&result);
  response.push_str(",\"warnings\":");
  response.push_str(&warnings);
  response.push('}');
  Ok(response)
}

fn parse(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<Json, Failure> {
  let dialect = dialect(&req)?;
  let expression = expression(&mut req)?;
  // The context is optional for parsing, but it tells the parser which (multi-word) names exist.
  let scope = prepare_scope(req, unread, dialect, &mut vec![])?;
  let node = parse_node(&scope, dialect, &expression)?;
  Ok(json!({
    "ok": true,
    "tree": node.to_string(),
    "elapsedMicros": elapsed_micros(started),
  }))
}
