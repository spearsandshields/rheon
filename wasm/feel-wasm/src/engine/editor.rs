//! The editor operations of the module: `syntax` (the tokens and the syntax errors of a text,
//! for highlighting and error marks), `complete` (what can stand at a cursor) and `outline`
//! (the names of a context without its values). See docs/CONTRACT.md §2.
//!
//! They answer for every text: a text that is no expression has errors in the answer, not a
//! failure. Places are offsets in UTF-16 code units, the unit of JavaScript and .NET strings.

use super::*;
use rheon_feel::FeelType;
use rheon_feel::bif::BUILT_IN_FUNCTION_NAMES;
use rheon_feel::signatures::{BuiltIn, built_in};
use rheon_feel::values::Value;
use rheon_feel_evaluator::{infer_type_with, members_of, type_of_text, type_of_value};
use rheon_feel_parser::{DEFAULT_WIDTH, Formatted, StartRule, Syntax, SyntaxNode, SyntaxToken, TokenClass, parse_expression, parse_tolerant_start};

/// The most items a `complete` answer holds.
const MAX_ITEMS: usize = 200;

/// The keywords that start an expression, and those that can follow an operand.
const OPERAND_KEYWORDS: [&str; 8] = ["if", "for", "some", "every", "function", "true", "false", "null"];
const OPERATOR_KEYWORDS: [&str; 6] = ["and", "or", "between", "in", "instance of", "as"];
/// The names of the built-in types, the abstract types of DMN 1.7 (Figure 10-26: comparable,
/// temporal, duration, and context of any entries), and the starts of the types that take types.
const TYPES: [&str; 18] = [
  "Any",
  "Null",
  "boolean",
  "date",
  "date and time",
  "days and time duration",
  "number",
  "string",
  "time",
  "years and months duration",
  "comparable",
  "temporal",
  "duration",
  "context",
  "context<",
  "function<",
  "list<",
  "range<",
];

/// The offsets of a text in UTF-16 code units, by byte offset.
struct Places<'a> {
  text: &'a str,
  /// For a text with other characters than ASCII: the UTF-16 offset of every byte offset.
  units: Option<Vec<u32>>,
}

impl<'a> Places<'a> {
  fn new(text: &'a str) -> Self {
    let units = (!text.is_ascii()).then(|| {
      let mut units = Vec::with_capacity(text.len() + 1);
      let mut count = 0u32;
      for ch in text.chars() {
        units.extend(std::iter::repeat_n(count, ch.len_utf8()));
        count += ch.len_utf16() as u32;
      }
      units.push(count);
      units
    });
    Self { text, units }
  }

  /// The UTF-16 offset of a byte offset.
  fn of(&self, byte: usize) -> usize {
    match &self.units {
      Some(units) => units[byte.min(self.text.len())] as usize,
      None => byte,
    }
  }

  /// The byte offset of a UTF-16 offset: the start of the character it lies in, the end of the
  /// text when it lies beyond.
  fn byte(&self, unit: usize) -> usize {
    match &self.units {
      Some(units) => units.partition_point(|&count| (count as usize) < unit).min(self.text.len()),
      None => unit.min(self.text.len()),
    }
  }
}

/// The scope of an editor operation: from `context` as for `parse`, or from `outline`.
fn scope(mut req: Map<String, Json>, mut unread: Unread, dialect: Dialect) -> Result<FeelScope, Failure> {
  let Some(outline) = req.remove("outline") else {
    return prepare_scope(req, unread, dialect, &mut vec![]);
  };
  if req.contains_key("context") || unread.context.is_some() {
    return fail(Kind::Request, "give 'context' or 'outline', not both");
  }
  unread.context = None;
  req.remove("contextFormat");
  let mut typed = false;
  req.insert("context".to_string(), context_of_outline(&outline, 0, "outline", &mut typed)?);
  let scope = prepare_scope(req, unread, dialect, &mut vec![])?;
  if !typed {
    return Ok(scope);
  }
  // the names with a type stand in the scope with their type in the place of a value
  let Some(mut context) = scope.frames().last().map(|frame| (**frame).clone()) else {
    return Ok(scope);
  };
  if let Json::Object(fields) = &outline
    && let Some(Json::Object(names)) = fields.get("names")
  {
    for (name, entry) in names {
      context.insert(Name::from(name.as_str()), value_of_outline(entry));
    }
  }
  Ok(context.into())
}

/// The JSON context that has the names of an outline: null for a name without entries or with
/// a type, an object for `{"names": …}`, a list of one such value for `{"items": …}`. `typed`
/// is set when a name has a type.
fn context_of_outline(outline: &Json, depth: usize, place: &str, typed: &mut bool) -> Result<Json, Failure> {
  const SHAPE: &str = "an outline is null, {\"type\": <type>}, {\"names\": {<name>: <outline>, …}} or {\"items\": <outline>}";
  if depth > 100 {
    return fail(Kind::Context, "the outline is nested too deeply (at most 100 levels)");
  }
  match outline {
    Json::Null => Ok(Json::Null),
    Json::Object(fields) if fields.len() == 1 => match (fields.get("names"), fields.get("items"), fields.get("type")) {
      (Some(Json::Object(names)), None, None) => {
        let mut context = Map::new();
        for (name, entry) in names {
          context.insert(name.clone(), context_of_outline(entry, depth + 1, &format!("{place}.{name}"), typed)?);
        }
        Ok(Json::Object(context))
      }
      (None, Some(items), None) => Ok(Json::Array(vec![context_of_outline(items, depth + 1, &format!("{place}[]"), typed)?])),
      (None, None, Some(Json::String(text))) if type_of_text(text).is_some() => {
        *typed = true;
        Ok(Json::Null)
      }
      (None, None, Some(Json::String(text))) => fail(
        Kind::Context,
        format!("{place}: expected a FEEL type (such as number, date, list<string> or range<date>), found {text:?}"),
      ),
      (None, None, Some(other)) => fail(Kind::Context, format!("{place}: expected a FEEL type as a string, found {other}")),
      _ => fail(Kind::Context, SHAPE),
    },
    _ => fail(Kind::Context, SHAPE),
  }
}

/// The value that stands for an outline (checked by [context_of_outline]) in the scope: a type
/// for a name with a type, null for a name without, a context, or a list of one item.
fn value_of_outline(outline: &Json) -> Value {
  let Json::Object(fields) = outline else {
    return Value::Null(None);
  };
  match (fields.get("names"), fields.get("items"), fields.get("type")) {
    (Some(Json::Object(names)), ..) => {
      let mut context = FeelContext::default();
      for (name, entry) in names {
        context.insert(Name::from(name.as_str()), value_of_outline(entry));
      }
      Value::Context(context.into())
    }
    (_, Some(items), _) => Value::List(vec![value_of_outline(items)].into()),
    (_, _, Some(Json::String(text))) => type_of_text(text).map_or(Value::Null(None), Value::FeelType),
    _ => Value::Null(None),
  }
}

/// The outline of a value: its names when it is a context, the outline of its first item when
/// it is a list of contexts, otherwise null, or with `types` its type when that is known.
fn outline_of(value: &Value, depth: usize, types: bool) -> Json {
  if depth > 100 {
    return Json::Null;
  }
  let leaf = |value: &Value| match type_of_value(value) {
    feel_type if types && !matches!(feel_type, FeelType::Any | FeelType::Null) => json!({ "type": feel_type.to_string() }),
    _ => Json::Null,
  };
  match value {
    Value::Context(context) => outline_of_context(context, depth, types),
    Value::List(items) => match items.first() {
      Some(first @ (Value::Context(_) | Value::List(_))) => match outline_of(first, depth + 1, types) {
        Json::Null => leaf(value),
        // a list of lists of plain values has its type as a whole
        Json::Object(fields) if fields.contains_key("type") => leaf(value),
        item => json!({ "items": item }),
      },
      _ => leaf(value),
    },
    _ => leaf(value),
  }
}

fn outline_of_context(context: &FeelContext, depth: usize, types: bool) -> Json {
  let names: Map<String, Json> = context.iter().map(|(name, value)| (name.to_string(), outline_of(value, depth + 1, types))).collect();
  json!({ "names": names })
}

/// `op: "outline"`: the names of the context, nested as its contexts and lists are, without
/// values; with `types: true` each name that is no context has its type when it is known. The
/// editor operations take it in place of the context.
pub(super) fn outline(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<Json, Failure> {
  let types = match req.remove("types") {
    None | Some(Json::Null) => false,
    Some(Json::Bool(types)) => types,
    Some(other) => return fail(Kind::Request, format!("'types' must be true or false, found {other}")),
  };
  let scope = prepare_scope(req, unread, Dialect::Expression, &mut vec![])?;
  let outline = scope.frames().last().map_or_else(|| json!({ "names": {} }), |frame| outline_of_context(frame, 0, types));
  Ok(json!({ "ok": true, "outline": outline, "elapsedMicros": elapsed_micros(started) }))
}

/// The syntax errors of a text as the answers hold them.
fn diagnostics_json(syntax: &Syntax, places: &Places) -> Vec<Json> {
  syntax
    .diagnostics
    .iter()
    .map(|d| json!({"code": d.code, "message": d.message, "from": places.of(d.start), "to": places.of(d.end)}))
    .collect()
}

/// The syntax errors of a text that `parse` refused, for the error of the response.
pub(super) fn diagnostics(scope: &FeelScope, expression: &str, rule: StartRule) -> Json {
  let syntax = parse_tolerant_start(scope, expression, rule);
  Json::Array(diagnostics_json(&syntax, &Places::new(expression)))
}

/// `op: "syntax"`: every token of the text with its class, and every syntax error; with
/// `"tree": true` also the nodes with their places.
pub(super) fn syntax(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<Json, Failure> {
  let dialect = dialect(&req)?;
  let expression = expression(&mut req)?;
  let tree = match req.remove("tree") {
    None | Some(Json::Null) => false,
    Some(Json::Bool(tree)) => tree,
    Some(_) => return fail(Kind::Request, "'tree' must be true or false"),
  };
  let scope = scope(req, unread, dialect)?;
  let syntax = parse_tolerant_start(&scope, &expression, dialect.rule());
  let places = Places::new(&expression);
  let tokens: Vec<Json> = syntax.tokens.iter().map(|t| json!([t.class.as_str(), places.of(t.start), places.of(t.end)])).collect();
  let diagnostics = diagnostics_json(&syntax, &places);
  let mut response = json!({
    "ok": true,
    "tokens": tokens,
    "diagnostics": diagnostics,
    "truncated": syntax.truncated,
    "elapsedMicros": elapsed_micros(started),
  });
  if tree {
    let nodes: Vec<Json> = syntax.nodes.iter().map(|n| json!([n.kind, places.of(n.start), places.of(n.end)])).collect();
    response["nodes"] = Json::Array(nodes);
  }
  Ok(response)
}

/// An item of a `complete` answer.
struct Item {
  label: String,
  kind: &'static str,
  /// Where the text that the item replaces starts (a byte offset).
  from: usize,
  /// The FEEL type of the item, when it is known.
  feel_type: Option<String>,
}

/// The invocation of a built-in function that the end of a text stands in.
struct Call {
  built_in: &'static BuiltIn,
  /// The index of the argument that the text ends in.
  argument: usize,
  /// The names of the named parameters given before that argument.
  named: Vec<String>,
  /// Whether a parameter name can stand where the text ends: no argument before is given by
  /// position, and the argument the text ends in has no name yet.
  takes_name: bool,
}

/// The innermost invocation of a built-in function whose parentheses are open where the text
/// ends, from the tokens of the text.
fn call(text: &str, syntax: &Syntax) -> Option<Call> {
  let mut tokens = syntax.tokens.iter().rev().filter(|t| !matches!(t.class, TokenClass::Whitespace | TokenClass::Comment));
  // the arguments from the last to the first: the name of each that has one
  let mut arguments: Vec<Option<String>> = vec![None];
  let mut depth = 0usize;
  loop {
    let token = tokens.next()?;
    let written = &text[token.start..token.end];
    match written {
      ")" | "]" | "}" if token.class != TokenClass::String => depth += 1,
      "[" | "{" if token.class != TokenClass::String => depth = depth.checked_sub(1)?,
      "(" if token.class != TokenClass::String && depth > 0 => depth -= 1,
      "(" if token.class != TokenClass::String => break,
      "," if depth == 0 => arguments.push(None),
      _ if depth == 0 && token.class == TokenClass::Parameter => *arguments.last_mut()? = Some(name_of(written).to_string()),
      _ => {}
    }
  }
  let name = tokens.next()?;
  let built_in = built_in(&name_of(&text[name.start..name.end]).to_string())?;
  let takes_name = arguments.iter().enumerate().all(|(index, argument)| (index == 0) == argument.is_none());
  Some(Call {
    built_in,
    argument: arguments.len() - 1,
    named: arguments.into_iter().flatten().collect(),
    takes_name,
  })
}

/// The names and types of the parameters that can be named next in the invocation: those of
/// the signatures that take names, without the ones already given.
fn parameters(call: &Call) -> Vec<(&'static str, &'static str)> {
  let mut parameters: Vec<(&'static str, &'static str)> = vec![];
  for signature in call.built_in.signatures.iter().filter(|signature| signature.named) {
    // only of a signature that has every name given so far
    if !call.named.iter().all(|given| signature.parameters.iter().any(|parameter| parameter.name == given)) {
      continue;
    }
    for parameter in signature.parameters {
      if !call.named.iter().any(|given| given == parameter.name) && !parameters.iter().any(|(name, _)| *name == parameter.name) {
        parameters.push((parameter.name, parameter.feel_type));
      }
    }
  }
  parameters
}

fn call_json(call: &Call) -> Json {
  let signatures: Vec<Json> = call
    .built_in
    .signatures
    .iter()
    .map(|signature| {
      let parameters: Vec<Json> = signature
        .parameters
        .iter()
        .map(|parameter| json!({"name": parameter.name, "type": parameter.feel_type, "optional": parameter.optional, "rest": parameter.rest}))
        .collect();
      json!({"text": signature.text(call.built_in.name), "parameters": parameters, "result": signature.result, "named": signature.named})
    })
    .collect();
  json!({"function": call.built_in.name, "summary": call.built_in.summary, "argument": call.argument, "named": call.named, "signatures": signatures})
}

/// What the grammar takes at a place of the text.
struct Expected {
  /// The name of the expectation in the answer.
  name: String,
  kinds: Vec<Offer>,
}

/// A group of items that fits a place.
enum Offer {
  /// The names in scope: of the context, and those the text declares, each with its type when
  /// it is known.
  Names(Vec<(String, Option<String>)>),
  Functions,
  Keywords(Vec<&'static str>),
  Types,
  /// The entries or properties of the value before a dot, each with its type when it is known.
  Entries(Vec<(String, Option<String>)>),
  /// The parameters of the invoked built-in function that can be named, with their types.
  Parameters(Vec<(&'static str, &'static str)>),
}

/// The text of the token that ends at the end of `text` when it is (part of) a name being typed.
fn typed_name<'t>(text: &'t str, syntax: &Syntax) -> Option<(usize, &'t str)> {
  let token = syntax.tokens.last()?;
  let name = matches!(
    token.class,
    TokenClass::Name
      | TokenClass::Property
      | TokenClass::Key
      | TokenClass::Parameter
      | TokenClass::Variable
      | TokenClass::Type
      | TokenClass::Function
      | TokenClass::Keyword
      | TokenClass::Constant
  );
  (name && token.end == text.len()).then(|| (token.start, &text[token.start..]))
}

/// The names of the path before the dot that ends the tokens of `syntax`: `order.address.` gives
/// `order`, `address`. None when something else than names and dots stands there.
fn receiver<'t>(text: &'t str, syntax: &Syntax) -> Option<Vec<&'t str>> {
  let mut tokens = syntax.tokens.iter().rev().filter(|t| !matches!(t.class, TokenClass::Whitespace | TokenClass::Comment));
  let mut path = vec![];
  loop {
    let dot = tokens.next()?;
    if &text[dot.start..dot.end] != "." {
      return None;
    }
    let name = tokens.next()?;
    if !matches!(name.class, TokenClass::Name | TokenClass::Property) {
      return None;
    }
    path.push(&text[name.start..name.end]);
    if name.class == TokenClass::Name {
      path.reverse();
      return Some(path);
    }
  }
}

/// `text` with its runs of white space (that of FEEL) as one space, and none at its ends.
fn spaced(text: &str) -> String {
  text.split(rheon_feel::names::is_white_space).filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ")
}

/// A name as written in the text, with its runs of white space as one space.
fn name_of(text: &str) -> Name {
  Name::from(spaced(text))
}

/// The context whose entries a path reaches through a value: the value when it is a context,
/// its first item when it is a list of contexts.
fn context_of(value: &Value) -> Option<&FeelContext> {
  match value {
    Value::Context(context) => Some(context),
    Value::List(items) => match items.first() {
      Some(Value::Context(context)) => Some(context),
      _ => None,
    },
    _ => None,
  }
}

/// The names of the entries of the value that the path of names leads to in the context.
fn entries(scope: &FeelScope, path: &[&str]) -> Vec<String> {
  let frames = scope.frames();
  let first = name_of(path[0]);
  let Some(mut context) = frames.iter().rev().find_map(|frame| frame.get_entry(&first)).and_then(context_of) else {
    return vec![];
  };
  for name in &path[1..] {
    let Some(next) = context.get_entry(&name_of(name)).and_then(context_of) else {
      return vec![];
    };
    context = next;
  }
  context.iter().map(|(name, _)| name.to_string()).collect()
}

/// The type of a value or an expression as an item shows it: the kind of the value, without the
/// entries of a context (`context`, `list<context>`, `list<number>`). None when it is not known.
fn shown(feel_type: &FeelType) -> Option<String> {
  match feel_type {
    FeelType::Any | FeelType::Null => None,
    FeelType::Context(_) => Some("context".to_string()),
    // a function with its signature when the types of its parameters or of its result are known
    FeelType::Function(parameters, result) if parameters.iter().chain([result.as_ref()]).any(|known| *known != FeelType::Any) => Some(feel_type.to_string()),
    FeelType::Function(..) => Some("function".to_string()),
    FeelType::List(item) => Some(match shown(item) {
      Some(item) => format!("list<{item}>"),
      None => "list".to_string(),
    }),
    FeelType::Range(endpoint) => Some(match shown(endpoint) {
      Some(endpoint) => format!("range<{endpoint}>"),
      None => "range".to_string(),
    }),
    other => Some(other.to_string()),
  }
}

/// The type of a value of the context as an item shows it ([shown]), read without its entries:
/// of a list, by its first item.
fn shown_of_value(value: &Value) -> Option<String> {
  match value {
    Value::Null(_) => None,
    // a type in the place of a value: the name of an outline that has its type
    Value::FeelType(feel_type) => shown(feel_type),
    Value::Context(_) => Some("context".to_string()),
    // a function of the context (a business knowledge model) with the types it declares
    function @ Value::FunctionDefinition(_) => shown(&function.type_of()),
    Value::BuiltInFunction(_) => Some("function".to_string()),
    Value::List(items) => Some(match items.first().and_then(shown_of_value) {
      Some(item) => format!("list<{item}>"),
      None => "list".to_string(),
    }),
    other => shown(&other.type_of()),
  }
}

/// The most constructs around a place whose names are typed (the innermost ones).
const MAX_BINDERS: usize = 16;

/// The type of the expression that `piece` is, `Any` when it is none.
fn type_of_piece(scope: &FeelScope, piece: &str, bound: &Bound) -> FeelType {
  match parse_expression(scope, piece) {
    Ok(node) if node.depth() <= MAX_AST_DEPTH => infer_type_with(&node, scope, &bound.names, &bound.literals),
    _ => FeelType::Any,
  }
}

/// The names that the constructs around a place bind there ([bound_at]).
#[derive(Default)]
struct Bound {
  /// The names with their types, the innermost last.
  names: Vec<(Name, FeelType)>,
  /// The function literals that names of `names` are bound to (entries of a context literal
  /// before the place): the index of the name, and the literal. An invocation of the name is
  /// typed with the types of its arguments.
  literals: Vec<(usize, AstNode)>,
}

/// The tokens of `syntax` that are no white space and no comments.
fn words(syntax: &Syntax) -> impl Iterator<Item = &SyntaxToken> {
  syntax.tokens.iter().filter(|token| !matches!(token.class, TokenClass::Whitespace | TokenClass::Comment))
}

/// The node that starts at `start` and ends last, at or before `end`: the whole expression
/// that starts there.
fn expression_at(syntax: &Syntax, start: usize, end: usize) -> Option<&SyntaxNode> {
  syntax.nodes.iter().filter(|node| node.start == start && node.end <= end).max_by_key(|node| node.end)
}

/// Whether the construct `node` is written to its end: a context to its `}`, a filter to its
/// `]`. The other constructs end with their last expression, which can go on.
fn closed(text: &str, node: &SyntaxNode) -> bool {
  match node.kind {
    "Context" => text[node.start..node.end].ends_with('}') && node.end - node.start > 1,
    "Filter" => text[node.start..node.end].ends_with(']'),
    _ => false,
  }
}

/// The names that the constructs around the place `at` of `text` bind there, with their types,
/// the innermost last: the variables of a `for`, `some` or `every` (the type of an item of
/// their domain), the parameters of a function (`Any`: they hide the names outside), the
/// entries of a context literal written before the place, and in the predicate of a filter
/// `item` and the entries of an item.
fn bound_at(scope: &FeelScope, text: &str, syntax: &Syntax, at: usize) -> Bound {
  // a construct that is not closed ends at the last token before the place
  let reach = words(syntax).take_while(|token| token.end <= at).last().map_or(at, |token| token.end);
  let mut around: Vec<&SyntaxNode> = syntax
    .nodes
    .iter()
    .filter(|node| matches!(node.kind, "For" | "Quantified" | "Function" | "Context" | "Filter") && node.start < at && node.end >= reach && !closed(text, node))
    .collect();
  around.sort_by_key(|node| node.start);
  if around.len() > MAX_BINDERS {
    around.drain(..around.len() - MAX_BINDERS);
  }
  // whether the construct `node` is the innermost one of its kinds that holds the place `place`
  let holds = |node: &SyntaxNode, kinds: &[&str], place: usize| {
    syntax
      .nodes
      .iter()
      .filter(|other| kinds.contains(&other.kind) && other.start <= place && place < other.end)
      .max_by_key(|other| other.start)
      .is_some_and(|innermost| innermost.start == node.start && innermost.end == node.end)
  };
  let mut bound = Bound::default();
  for node in around {
    match node.kind {
      "For" | "Quantified" => {
        let tokens: Vec<&SyntaxToken> = words(syntax).filter(|token| token.start >= node.start && token.end <= at).collect();
        for (index, token) in tokens.iter().enumerate() {
          if token.class != TokenClass::Variable || !holds(node, &["For", "Quantified", "Function"], token.start) {
            continue;
          }
          // the domain starts after the `in` that follows the variable
          let Some(domain_start) = tokens
            .get(index + 2)
            .filter(|_| tokens.get(index + 1).is_some_and(|word| &text[word.start..word.end] == "in"))
            .map(|word| word.start)
          else {
            break;
          };
          let Some(domain) = expression_at(syntax, domain_start, at) else {
            break;
          };
          // an interval of numbers has two endpoints
          let interval = words(syntax).find(|word| word.start >= domain.end).is_some_and(|word| &text[word.start..word.end] == "..");
          let variable_type = if interval {
            FeelType::Number
          } else {
            match type_of_piece(scope, &text[domain.start..domain.end], &bound) {
              FeelType::List(item) => *item,
              FeelType::Range(endpoint) => *endpoint,
              other => other,
            }
          };
          bound.names.push((name_of(&text[token.start..token.end]), variable_type));
        }
      }
      "Function" => {
        // the parameters stand before the first closing parenthesis of the function
        let Some(head_end) = words(syntax)
          .find(|token| token.start > node.start && &text[token.start..token.end] == ")")
          .map(|token| token.start)
        else {
          continue;
        };
        if head_end >= at {
          continue;
        }
        for token in words(syntax).filter(|token| token.class == TokenClass::Parameter && token.start > node.start && token.end <= head_end) {
          bound.names.push((name_of(&text[token.start..token.end]), FeelType::Any));
        }
      }
      "Context" => {
        // the entries of the literal that are written before the place, as a literal of their own
        let entries: Vec<&str> = syntax
          .nodes
          .iter()
          .filter(|entry| entry.kind == "Entry" && entry.start > node.start && entry.end <= at && entry.end < node.end && holds(node, &["Context"], entry.start))
          .map(|entry| &text[entry.start..entry.end])
          .collect();
        if entries.is_empty() {
          continue;
        }
        let Ok(literal) = parse_expression(scope, &format!("{{{}}}", entries.join(", "))) else {
          continue;
        };
        if literal.depth() > MAX_AST_DEPTH {
          continue;
        }
        if let (FeelType::Context(types), AstNode::Context(entries)) = (infer_type_with(&literal, scope, &bound.names, &bound.literals), &literal) {
          // the entries in the order they are written (a function literal sees the entries
          // before it), with the function literals for the invocations of their names
          for entry in entries {
            if let AstNode::ContextEntry(key, value) = entry
              && let AstNode::ContextEntryKey(name) = key.as_ref()
            {
              if let AstNode::FunctionDefinition(..) = value.as_ref() {
                bound.literals.push((bound.names.len(), (**value).clone()));
              }
              bound.names.push((name.clone(), types.get(name).cloned().unwrap_or(FeelType::Any)));
            }
          }
        }
      }
      // the predicate of a filter: the list is the expression before its bracket
      _ => {
        let list = syntax
          .nodes
          .iter()
          .filter(|list| list.start == node.start && list.end < at && words(syntax).find(|word| word.start >= list.end).is_some_and(|word| &text[word.start..word.end] == "["))
          .max_by_key(|list| list.end);
        let Some(list) = list else {
          continue;
        };
        let item = match type_of_piece(scope, &text[list.start..list.end], &bound) {
          FeelType::List(item) => *item,
          other => other,
        };
        if let FeelType::Context(types) = &item {
          bound.names.extend(types.iter().map(|(name, entry_type)| (name.clone(), entry_type.clone())));
        }
        bound.names.push((Name::from("item"), item));
      }
    }
  }
  bound
}

/// The members of the value before the dot that ends `text`, each with its type as an item
/// shows it: the entries of a context, the properties of a date, a time, a duration or a
/// range, and for a list the members of its items. The value is the expression before the dot,
/// typed without evaluating it.
fn members(scope: &FeelScope, text: &str, syntax: &Syntax) -> Vec<(String, Option<String>)> {
  let typed = || {
    let dot = words(syntax).last().filter(|token| &text[token.start..token.end] == ".")?;
    // the path that the dot belongs to starts where its base starts
    let path = syntax
      .nodes
      .iter()
      .filter(|node| node.kind == "Path" && node.start < dot.start && node.end >= dot.end)
      .max_by_key(|node| node.start)?;
    let bound = bound_at(scope, text, syntax, path.start);
    let base = type_of_piece(scope, &text[path.start..dot.start], &bound);
    let members: Vec<_> = members_of(&base).into_iter().map(|(name, member_type)| (name, shown(&member_type))).collect();
    // a name that the text binds hides the name of the context: its members are all there is
    let hidden = words(syntax)
      .find(|token| token.start >= path.start)
      .is_some_and(|first| bound.names.iter().any(|(name, _)| name.as_str() == &text[first.start..first.end]));
    Some((members, hidden))
  };
  match typed() {
    Some((members, hidden)) if hidden || !members.is_empty() => members,
    // the names of a path into the context, as far as its values say
    _ => receiver(text, syntax)
      .map(|path| entries(scope, &path))
      .unwrap_or_default()
      .into_iter()
      .map(|name| (name, None))
      .collect(),
  }
}

/// What the end of `text` is inside of, when it is no place of the grammar: a comment (a line
/// comment before its line ends, a block comment that is not closed) or a string that is not
/// closed. Nothing is offered there.
fn inside(text: &str, syntax: &Syntax) -> Option<&'static str> {
  if syntax.diagnostics.iter().any(|d| d.code == "comment-open") {
    return Some("comment");
  }
  if syntax.diagnostics.iter().any(|d| d.code == "string-open" && d.end == text.len()) {
    return Some("string");
  }
  let last = syntax.tokens.last()?;
  let comment = &text[last.start..last.end];
  (last.class == TokenClass::Comment && last.end == text.len() && comment.starts_with("//") && !comment.contains('\n')).then_some("comment")
}

/// What the grammar takes where `text` ends, from the tolerant parse of `text`.
fn expected(scope: &FeelScope, text: &str, syntax: &Syntax, unary_tests: bool) -> Expected {
  let operators = || Offer::Keywords(OPERATOR_KEYWORDS.to_vec());
  let Some(end) = &syntax.end else {
    // the text is a whole expression: an operator can follow
    return Expected {
      name: "operator".to_string(),
      kinds: vec![operators()],
    };
  };
  match end.code {
    "expected-expression" => {
      // the names the text declares hide those of the context; their types are those of the
      // constructs around the end of the text
      // (the innermost first: of two names that are the same the first is kept); in the
      // predicate of a filter they are `item` and the entries of an item
      let bound = bound_at(scope, text, syntax, text.len());
      let mut names: Vec<(String, Option<String>)> = bound.names.iter().rev().map(|(name, feel_type)| (name.to_string(), shown(feel_type))).collect();
      names.extend(end.names.iter().map(|name| (name.clone(), None)));
      names.extend(
        scope
          .frames()
          .iter()
          .rev()
          .flat_map(|frame| frame.iter().map(|(name, value)| (name.to_string(), shown_of_value(value)))),
      );
      if unary_tests {
        names.push(("?".to_string(), None));
      }
      names.sort_by(|a, b| a.0.cmp(&b.0));
      names.dedup_by(|later, first| later.0 == first.0);
      Expected {
        name: "expression".to_string(),
        kinds: vec![Offer::Names(names), Offer::Keywords(OPERAND_KEYWORDS.to_vec()), Offer::Functions],
      }
    }
    "expected-name" if end.role == TokenClass::Property => Expected {
      name: "property".to_string(),
      kinds: vec![Offer::Entries(members(scope, text, syntax))],
    },
    // after a named parameter only a named one can follow
    "expected-name" if end.role == TokenClass::Parameter => Expected {
      name: "parameter".to_string(),
      kinds: vec![],
    },
    "expected-name" => Expected {
      name: "name".to_string(),
      kinds: vec![],
    },
    "expected-type" => Expected {
      name: "type".to_string(),
      kinds: vec![Offer::Types],
    },
    "expected" if end.token.chars().next().is_some_and(char::is_alphabetic) => Expected {
      name: format!("keyword:{}", end.token),
      kinds: vec![Offer::Keywords(vec![end.token]), operators()],
    },
    _ => Expected {
      name: "operator".to_string(),
      kinds: vec![operators()],
    },
  }
}

/// Adds the items of what is expected that start with `typed` (letters compared without case,
/// runs of white space as one space).
fn offer(items: &mut Vec<Item>, expected: &Expected, typed: &str, from: usize) {
  let normal = |text: &str| spaced(text).to_lowercase();
  let typed = normal(typed);
  let mut add = |label: &str, kind: &'static str, feel_type: Option<&str>| {
    if normal(label).starts_with(&typed) && !items.iter().any(|item| item.label == label && item.kind == kind) {
      items.push(Item {
        label: label.to_string(),
        kind,
        from,
        feel_type: feel_type.map(str::to_string),
      });
    }
  };
  for kind in &expected.kinds {
    match kind {
      Offer::Names(names) => names.iter().for_each(|(name, feel_type)| add(name, "name", feel_type.as_deref())),
      Offer::Entries(names) => names.iter().for_each(|(name, feel_type)| add(name, "entry", feel_type.as_deref())),
      Offer::Keywords(keywords) => keywords.iter().for_each(|keyword| add(keyword, "keyword", None)),
      Offer::Functions => BUILT_IN_FUNCTION_NAMES.iter().for_each(|name| add(name, "function", None)),
      Offer::Types => TYPES.iter().for_each(|name| add(name, "type", None)),
      Offer::Parameters(parameters) => parameters.iter().for_each(|(name, feel_type)| add(name, "parameter", Some(feel_type))),
    }
  }
}

/// `op: "complete"`: what can stand at the cursor, as items that each replace the text from
/// their `from` to the cursor.
pub(super) fn complete(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<Json, Failure> {
  let dialect = dialect(&req)?;
  let unary_tests = dialect == Dialect::UnaryTests;
  let expression = expression(&mut req)?;
  let places = Places::new(&expression);
  let cursor = match req.remove("cursor") {
    None | Some(Json::Null) => expression.len(),
    Some(Json::Number(cursor)) => match cursor.as_u64() {
      Some(cursor) => places.byte(cursor as usize),
      None => return fail(Kind::Request, "'cursor' must be a whole number that is not negative"),
    },
    Some(_) => return fail(Kind::Request, "'cursor' must be a whole number that is not negative"),
  };
  let scope = scope(req, unread, dialect)?;
  let before = &expression[..cursor];
  let rule = dialect.rule();
  let syntax = parse_tolerant_start(&scope, before, rule);
  let mut items = vec![];
  let expects;
  let invocation;
  // the names of the parameters of the invoked function go before what else an expression can be
  let with_parameters = |mut expected: Expected, invocation: &Option<Call>| {
    if let Some(call) = invocation
      .as_ref()
      .filter(|call| call.takes_name && matches!(expected.name.as_str(), "expression" | "parameter"))
    {
      expected.kinds.insert(0, Offer::Parameters(parameters(call)));
    }
    expected
  };
  match typed_name(before, &syntax) {
    // inside a comment or a string: nothing is offered
    _ if inside(before, &syntax).is_some() => {
      invocation = call(before, &syntax);
      expects = inside(before, &syntax).unwrap_or_default().to_string();
    }
    // a name is being typed: what fits where it starts, and, for a name of several words, what
    // fits where its last word starts (`if a th` is `if a` and the start of `then`)
    Some((from, typed)) => {
      let head = &before[..from];
      let head_syntax = parse_tolerant_start(&scope, head, rule);
      invocation = call(head, &head_syntax);
      let at_start = with_parameters(expected(&scope, head, &head_syntax, unary_tests), &invocation);
      offer(&mut items, &at_start, typed, from);
      expects = at_start.name;
      if let Some(space) = typed.rfind(char::is_whitespace) {
        let word = from + space + typed[space..].chars().next().map_or(0, char::len_utf8);
        let head = &before[..word];
        let at_word = expected(&scope, head, &parse_tolerant_start(&scope, head, rule), unary_tests);
        offer(&mut items, &at_word, &before[word..], word);
      }
    }
    None => {
      invocation = call(before, &syntax);
      let at_cursor = with_parameters(expected(&scope, before, &syntax, unary_tests), &invocation);
      offer(&mut items, &at_cursor, "", cursor);
      expects = at_cursor.name;
    }
  }
  if dialect.simple() {
    // S-FEEL has no keywords but true and false, and of the functions only its date literals
    items.retain(|item| match item.kind {
      "keyword" => matches!(item.label.as_str(), "true" | "false"),
      "function" => matches!(item.label.as_str(), "date" | "time" | "duration" | "days and time duration" | "years and months duration"),
      _ => true,
    });
  }
  let order = |kind: &str| ["parameter", "name", "entry", "keyword", "function", "type"].iter().position(|k| *k == kind);
  items.sort_by(|a, b| order(a.kind).cmp(&order(b.kind)).then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase())));
  let truncated = items.len() > MAX_ITEMS;
  items.truncate(MAX_ITEMS);
  let items: Vec<Json> = items
    .iter()
    .map(|item| {
      let mut json = json!({"label": item.label, "kind": item.kind, "from": places.of(item.from), "type": item.feel_type, "detail": null});
      match item.kind {
        // a parameter is inserted with its colon
        "parameter" => json["insert"] = json!(format!("{}: ", item.label)),
        "function" => {
          if let Some(built_in) = built_in(&item.label) {
            let first = &built_in.signatures[0];
            json["detail"] = json!(first.text(built_in.name));
            json["info"] = json!(built_in.summary);
            // the type of the result when every signature has the same
            if built_in.signatures.iter().all(|signature| signature.result == first.result) {
              json["type"] = json!(first.result);
            }
          }
        }
        _ => {}
      }
      json
    })
    .collect();
  let mut response = json!({
    "ok": true,
    "to": places.of(cursor),
    "expects": expects,
    "items": items,
    "truncated": truncated,
    "elapsedMicros": elapsed_micros(started),
  });
  if let Some(call) = &invocation {
    response["call"] = call_json(call);
  }
  Ok(response)
}

/// `op: "format"`: the text in the one layout of the formatter (lines of at most `width`
/// characters where it can be broken, nested parts indented). A text with syntax errors is the
/// failure that `parse` answers. A text that can not be formatted safely comes back as it is,
/// with the reason.
pub(super) fn format(mut req: Map<String, Json>, unread: Unread, started: Instant) -> Result<Json, Failure> {
  const WIDTH: &str = "'width' must be a whole number from 1 to 1000";
  let dialect = dialect(&req)?;
  let expression = expression(&mut req)?;
  let width = match req.remove("width") {
    None | Some(Json::Null) => DEFAULT_WIDTH,
    Some(Json::Number(width)) => match width.as_u64() {
      Some(width @ 1..=1000) => width as usize,
      _ => return fail(Kind::Request, WIDTH),
    },
    Some(_) => return fail(Kind::Request, WIDTH),
  };
  let scope = scope(req, unread, dialect)?;
  let (formatted, reason) = match rheon_feel_parser::format_start(&scope, &expression, dialect.rule(), width) {
    Formatted::Text(formatted) => (formatted, None),
    Formatted::Unchanged => (
      expression.clone(),
      Some("the text is left as it is: its spacing decides how its names are read, or it is nested too deeply to be checked"),
    ),
    Formatted::Errors(_) => {
      // the failure of `parse`, with the errors of the text
      parse_node(&scope, dialect, &expression)?;
      (expression.clone(), Some("the text is left as it is: it has syntax errors"))
    }
  };
  let mut response = json!({
    "ok": true,
    "formatted": formatted,
    "changed": formatted != expression,
    "elapsedMicros": elapsed_micros(started),
  });
  if let Some(reason) = reason {
    response["reason"] = json!(reason);
  }
  Ok(response)
}
