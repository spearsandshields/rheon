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
use rheon_feel_evaluator::{TypeId, Typing, type_of_text, type_of_value};
use rheon_feel_parser::{DEFAULT_WIDTH, Formatted, StartRule, Syntax, SyntaxNode, SyntaxToken, TokenClass, parse_expression, parse_tolerant_start};
use std::collections::HashMap;

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

/// The key under which the scope of an outline holds the names of its `types`: no name of FEEL
/// (it starts with a control character), so no text reads it and no answer shows it. The lexer
/// takes a key at any depth as a name in scope, and so knows the names of every type.
const TYPES_KEY: &str = "\u{1}types";

/// The scope of an editor operation: from `context` as for `parse`, or from `outline`, which
/// is returned beside it for what it says of its names ([Schema]).
fn scope(mut req: Map<String, Json>, mut unread: Unread, dialect: Dialect) -> Result<(FeelScope, Option<Json>), Failure> {
  let Some(outline) = req.remove("outline") else {
    return Ok((prepare_scope(req, unread, dialect, &mut vec![])?, None));
  };
  if req.contains_key("context") || unread.context.is_some() {
    return fail(Kind::Request, "give 'context' or 'outline', not both");
  }
  unread.context = None;
  req.remove("contextFormat");
  let mut context = context_of_outline(&outline, 0, &At::Root("outline"), true)?;
  if let (Json::Object(names), Json::Object(fields)) = (&mut context, &outline)
    && let Some(Json::Object(types)) = fields.get("types")
  {
    let mut held = Map::new();
    for (name, definition) in types {
      held.insert(name.clone(), context_of_outline(definition, 1, &At::Entry(&At::Root("outline.types"), name), false)?);
    }
    names.insert(TYPES_KEY.to_string(), Json::Object(held));
  }
  req.insert("context".to_string(), context);
  let scope = prepare_scope(req, unread, dialect, &mut vec![])?;
  Ok((scope, Some(outline)))
}

/// Where a part of an outline stands, written when a failure names it (`outline.order.lines[]`).
#[derive(Clone, Copy)]
enum At<'a> {
  Root(&'a str),
  Entry(&'a At<'a>, &'a str),
  Items(&'a At<'a>),
}

impl std::fmt::Display for At<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      At::Root(text) => write!(f, "{text}"),
      At::Entry(parent, name) => write!(f, "{parent}.{name}"),
      At::Items(parent) => write!(f, "{parent}[]"),
    }
  }
}

/// The fields that say what an outline is, of which it has one.
const FORMS: [&str; 4] = ["names", "items", "type", "function"];

/// The JSON context that has the names of an outline: null for a name without entries, with a
/// type or that is a function, an object for `{"names": …}`, a list of one such value for
/// `{"items": …}`. The shape of the outline is checked; its types are checked when they are
/// read ([Schema::read]). `root`: the outline of the request, which can have `types`.
fn context_of_outline(outline: &Json, depth: usize, place: &At, root: bool) -> Result<Json, Failure> {
  const SHAPE: &str = "an outline is null, {\"type\": <type>}, {\"names\": {<name>: <outline>, …}}, {\"items\": <outline>} or {\"function\": {\"parameters\": […], \"result\": <type>}}, with \"description\" and \"deprecated\" beside it when it has them, and \"values\" beside a type";
  if depth > 100 {
    return fail(Kind::Context, "the outline is nested too deeply (at most 100 levels)");
  }
  let fields = match outline {
    Json::Null => return Ok(Json::Null),
    Json::Object(fields) => fields,
    _ => return fail(Kind::Context, SHAPE),
  };
  for (field, value) in fields {
    let known = match (field.as_str(), value) {
      ("description", Json::String(_)) | ("deprecated", Json::Bool(_) | Json::String(_)) => true,
      ("values", Json::Array(_)) => fields.contains_key("type"),
      ("types", Json::Object(_)) => root,
      (form, _) => FORMS.contains(&form),
    };
    if !known {
      return fail(Kind::Context, format!("{place}: {SHAPE}; found the field {field:?} with the value {}", preview(value)));
    }
  }
  let forms: Vec<&str> = FORMS.into_iter().filter(|form| fields.contains_key(*form)).collect();
  match (forms.as_slice(), forms.first().and_then(|form| fields.get(*form))) {
    // the outline of a request can be its types and nothing else
    ([], None) if root && fields.contains_key("types") => Ok(Json::Object(Map::new())),
    (["names"], Some(Json::Object(names))) => {
      let mut context = Map::new();
      for (name, entry) in names {
        context.insert(name.clone(), context_of_outline(entry, depth + 1, &At::Entry(place, name), false)?);
      }
      Ok(Json::Object(context))
    }
    (["items"], Some(items)) => Ok(Json::Array(vec![context_of_outline(items, depth + 1, &At::Items(place), false)?])),
    (["type"], Some(Json::String(_))) => Ok(Json::Null),
    (["type"], Some(other)) => fail(Kind::Context, format!("{place}: expected a FEEL type as a string, found {other}")),
    (["function"], Some(Json::Object(_))) => Ok(Json::Null),
    _ => fail(Kind::Context, SHAPE),
  }
}

/// A value of an outline as a message quotes it: its JSON text, the start of a long one.
fn preview(value: &Json) -> String {
  let text = value.to_string();
  match text.char_indices().nth(60) {
    Some((at, _)) => format!("{}…", &text[..at]),
    None => text,
  }
}

/// What an outline says of a name or an entry beside its type.
#[derive(Default, Clone)]
struct Meta {
  description: Option<String>,
  /// `true`, or the reason as text.
  deprecated: Option<Json>,
}

impl Meta {
  fn of(fields: &Map<String, Json>) -> Option<Meta> {
    let meta = Meta {
      description: fields.get("description").and_then(Json::as_str).map(str::to_string),
      deprecated: fields.get("deprecated").filter(|deprecated| **deprecated != Json::Bool(false)).cloned(),
    };
    (meta.description.is_some() || meta.deprecated.is_some()).then_some(meta)
  }
}

/// What the outline of a request says of its names: the type of each name of the scope, stated
/// to the typing, and what it says beside the types.
#[derive(Default)]
struct Schema {
  /// The type of each name of the scope that the outline has.
  roots: HashMap<String, TypeId>,
  /// What the outline says of a name of the scope.
  root_meta: HashMap<String, Meta>,
  /// What the outline says of an entry: by the type that has the entry, and its name.
  entry_meta: HashMap<(TypeId, String), Meta>,
  /// The type that each type text read so far writes: an outline writes few texts many times.
  texts: HashMap<String, Option<TypeId>>,
}

impl Schema {
  /// Reads the outline (of a shape that [context_of_outline] took) into the types of `typing`:
  /// the types of `types`, which can hold each other, then the names of the scope. A text that
  /// is no type, or names a type that the outline has not, is a failure that names the entry.
  fn read(typing: &mut Typing, outline: Option<&Json>) -> Result<Schema, Failure> {
    let mut schema = Schema::default();
    let Some(Json::Object(fields)) = outline else {
      return Ok(schema);
    };
    let mut named: HashMap<String, TypeId> = HashMap::new();
    if let Some(Json::Object(types)) = fields.get("types") {
      for name in types.keys() {
        if type_of_text(name).is_some() {
          return fail(Kind::Context, format!("outline.types: expected a name that is no type of FEEL, found {name:?}"));
        }
        named.insert(name.clone(), typing.declare(Some(Name::from(name.as_str()))));
      }
      for (name, definition) in types {
        let defined = schema.node(typing, &named, definition, &At::Entry(&At::Root("outline.types"), name))?;
        typing.define(named[name], defined);
      }
      // types that are only each other are no types
      for (name, definition) in types {
        let mut at = definition;
        for _ in 0..=types.len() {
          match at.get("type").and_then(Json::as_str).and_then(|text| types.get(text.trim())) {
            Some(next) => at = next,
            None => break,
          }
        }
        if at.get("type").and_then(Json::as_str).is_some_and(|text| types.contains_key(text.trim())) {
          return fail(
            Kind::Context,
            format!("outline.types.{name}: expected a type that is more than the name of another, found types that are only each other"),
          );
        }
      }
    }
    if let Some(Json::Object(names)) = fields.get("names") {
      for (name, entry) in names {
        let stated = schema.node(typing, &named, entry, &At::Entry(&At::Root("outline"), name))?;
        typing.state(Name::from(name.trim()), stated);
        schema.roots.insert(name.trim().to_string(), stated);
        if let Some(meta) = entry.as_object().and_then(Meta::of) {
          schema.root_meta.insert(name.trim().to_string(), meta);
        }
      }
    }
    Ok(schema)
  }

  /// The type that the outline `node` at `place` is.
  fn node(&mut self, typing: &mut Typing, named: &HashMap<String, TypeId>, node: &Json, place: &At) -> Result<TypeId, Failure> {
    let Json::Object(fields) = node else {
      return Ok(TypeId::ANY);
    };
    if let Some(Json::Object(names)) = fields.get("names") {
      let mut entries = Vec::with_capacity(names.len());
      let mut said = vec![];
      for (name, entry) in names {
        // an entry has its key as the context of the scope has it
        entries.push((Name::from(name.trim()), self.node(typing, named, entry, &At::Entry(place, name))?));
        if let Some(meta) = entry.as_object().and_then(Meta::of) {
          said.push((name.trim().to_string(), meta));
        }
      }
      let context = typing.context(entries);
      if said.is_empty() {
        return Ok(context);
      }
      // a context whose entries have more than a type is itself, whatever context is alike
      let declared = typing.declare(None);
      typing.define(declared, context);
      for (name, meta) in said {
        self.entry_meta.insert((declared, name), meta);
      }
      return Ok(declared);
    }
    if let Some(items) = fields.get("items") {
      let item = self.node(typing, named, items, &At::Items(place))?;
      return Ok(typing.list(item));
    }
    if let Some(Json::String(text)) = fields.get("type") {
      let written = match self.texts.get(text) {
        Some(written) => *written,
        None => {
          let written = typing.written(text, &|name| named.get(name).copied());
          self.texts.insert(text.clone(), written);
          written
        }
      };
      return match written {
        Some(written) => Ok(written),
        None => fail(
          Kind::Context,
          format!("{place}: expected a FEEL type (such as number, date, list<string> or range<date>), found {text:?}"),
        ),
      };
    }
    Ok(TypeId::ANY)
  }

  /// What the outline says of the entry `name` of a value of the type `base`: of an entry of a
  /// context, or of the contexts of a list.
  fn entry(&self, typing: &Typing, base: TypeId, name: &str) -> Option<&Meta> {
    let mut base = base;
    for _ in 0..64 {
      if let Some(meta) = self.entry_meta.get(&(base, name.to_string())) {
        return Some(meta);
      }
      base = typing.defined_as(base).or_else(|| typing.item(base))?;
    }
    None
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
  let (scope, outline) = scope(req, unread, dialect)?;
  Schema::read(&mut Typing::new(&scope), outline.as_ref())?;
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
  /// What an outline says of the name or the entry.
  said: Option<Meta>,
}

/// A name or an entry that is offered: its type when it is known, and what an outline says of
/// it.
struct Offered {
  label: String,
  feel_type: Option<String>,
  said: Option<Meta>,
}

impl Offered {
  fn of(label: String, feel_type: Option<String>) -> Self {
    Self { label, feel_type, said: None }
  }

  fn with(mut self, said: Option<&Meta>) -> Self {
    self.said = said.cloned();
    self
  }
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
  Names(Vec<Offered>),
  Functions,
  Keywords(Vec<&'static str>),
  Types,
  /// The entries or properties of the value before a dot, each with its type when it is known.
  Entries(Vec<Offered>),
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

/// The type of the expression that `piece` is, the type that is not known when it is none.
fn type_of_piece(typing: &mut Typing, scope: &FeelScope, piece: &str, bound: &Bound) -> TypeId {
  match parse_expression(scope, piece) {
    Ok(node) if node.depth() <= MAX_AST_DEPTH => typing.infer(&node, &bound.names, &bound.literals),
    _ => TypeId::ANY,
  }
}

/// The names that the constructs around a place bind there ([bound_at]).
#[derive(Default)]
struct Bound {
  /// The names with their types (of the typing that typed them), the innermost last.
  names: Vec<(Name, TypeId)>,
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
fn bound_at(typing: &mut Typing, scope: &FeelScope, text: &str, syntax: &Syntax, at: usize) -> Bound {
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
            TypeId::NUMBER
          } else {
            let domain = type_of_piece(typing, scope, &text[domain.start..domain.end], &bound);
            typing.item(domain).or_else(|| typing.endpoint(domain)).unwrap_or(domain)
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
          bound.names.push((name_of(&text[token.start..token.end]), TypeId::ANY));
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
        let literal_type = typing.infer(&literal, &bound.names, &bound.literals);
        if let (Some(types), AstNode::Context(entries)) = (typing.entries(literal_type), &literal) {
          // the entries in the order they are written (a function literal sees the entries
          // before it), with the function literals for the invocations of their names
          for entry in entries {
            if let AstNode::ContextEntry(key, value) = entry
              && let AstNode::ContextEntryKey(name) = key.as_ref()
            {
              if let AstNode::FunctionDefinition(..) = value.as_ref() {
                bound.literals.push((bound.names.len(), (**value).clone()));
              }
              let entry_type = types.binary_search_by(|(entry, _)| entry.cmp(name)).map_or(TypeId::ANY, |index| types[index].1);
              bound.names.push((name.clone(), entry_type));
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
        let list = type_of_piece(typing, scope, &text[list.start..list.end], &bound);
        let item = typing.item(list).unwrap_or(list);
        if let Some(types) = typing.entries(item) {
          bound.names.extend(types.iter().cloned());
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
fn members(typing: &mut Typing, schema: &Schema, scope: &FeelScope, text: &str, syntax: &Syntax) -> Vec<Offered> {
  let mut typed = || {
    let dot = words(syntax).last().filter(|token| &text[token.start..token.end] == ".")?;
    // the path that the dot belongs to starts where its base starts
    let path = syntax
      .nodes
      .iter()
      .filter(|node| node.kind == "Path" && node.start < dot.start && node.end >= dot.end)
      .max_by_key(|node| node.start)?;
    let bound = bound_at(typing, scope, text, syntax, path.start);
    let base = type_of_piece(typing, scope, &text[path.start..dot.start], &bound);
    let members: Vec<_> = typing
      .members(base)
      .into_iter()
      .map(|(name, member_type)| {
        let said = schema.entry(typing, base, &name);
        Offered::of(name, typing.shown(member_type)).with(said)
      })
      .collect();
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
      .filter(|name| name != TYPES_KEY)
      .map(|name| Offered::of(name, None))
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
fn expected(typing: &mut Typing, schema: &Schema, scope: &FeelScope, text: &str, syntax: &Syntax, unary_tests: bool) -> Expected {
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
      let bound = bound_at(typing, scope, text, syntax, text.len());
      let mut names: Vec<Offered> = bound
        .names
        .iter()
        .rev()
        .map(|(name, bound_type)| Offered::of(name.to_string(), typing.shown(*bound_type)))
        .collect();
      names.extend(end.names.iter().map(|name| Offered::of(name.clone(), None)));
      // a name of the scope: with the type the outline states for it and what it says of it
      names.extend(scope.frames().iter().rev().flat_map(|frame| {
        frame.iter().filter(|(name, _)| name.as_str() != TYPES_KEY).map(|(name, value)| {
          let name = name.to_string();
          let shown = match schema.roots.get(&name) {
            Some(stated) => typing.shown(*stated),
            None => shown_of_value(value),
          };
          Offered::of(name.clone(), shown).with(schema.root_meta.get(&name))
        })
      }));
      if unary_tests {
        names.push(Offered::of("?".to_string(), None));
      }
      names.sort_by(|a, b| a.label.cmp(&b.label));
      names.dedup_by(|later, first| later.label == first.label);
      Expected {
        name: "expression".to_string(),
        kinds: vec![Offer::Names(names), Offer::Keywords(OPERAND_KEYWORDS.to_vec()), Offer::Functions],
      }
    }
    "expected-name" if end.role == TokenClass::Property => Expected {
      name: "property".to_string(),
      kinds: vec![Offer::Entries(members(typing, schema, scope, text, syntax))],
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
  let mut add_said = |label: &str, kind: &'static str, feel_type: Option<&str>, said: Option<&Meta>| {
    if normal(label).starts_with(&typed) && !items.iter().any(|item| item.label == label && item.kind == kind) {
      items.push(Item {
        label: label.to_string(),
        kind,
        from,
        feel_type: feel_type.map(str::to_string),
        said: said.cloned(),
      });
    }
  };
  for kind in &expected.kinds {
    match kind {
      Offer::Names(names) => names
        .iter()
        .for_each(|offered| add_said(&offered.label, "name", offered.feel_type.as_deref(), offered.said.as_ref())),
      Offer::Entries(names) => names
        .iter()
        .for_each(|offered| add_said(&offered.label, "entry", offered.feel_type.as_deref(), offered.said.as_ref())),
      Offer::Keywords(keywords) => keywords.iter().for_each(|keyword| add_said(keyword, "keyword", None, None)),
      Offer::Functions => BUILT_IN_FUNCTION_NAMES.iter().for_each(|name| add_said(name, "function", None, None)),
      Offer::Types => TYPES.iter().for_each(|name| add_said(name, "type", None, None)),
      Offer::Parameters(parameters) => parameters.iter().for_each(|(name, feel_type)| add_said(name, "parameter", Some(feel_type), None)),
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
  let (scope, outline) = scope(req, unread, dialect)?;
  // the pieces of the text that are typed share one table of types, with those of the outline
  let mut typing = Typing::new(&scope);
  let schema = Schema::read(&mut typing, outline.as_ref())?;
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
      let at_start = with_parameters(expected(&mut typing, &schema, &scope, head, &head_syntax, unary_tests), &invocation);
      offer(&mut items, &at_start, typed, from);
      expects = at_start.name;
      if let Some(space) = typed.rfind(char::is_whitespace) {
        let word = from + space + typed[space..].chars().next().map_or(0, char::len_utf8);
        let head = &before[..word];
        let at_word = expected(&mut typing, &schema, &scope, head, &parse_tolerant_start(&scope, head, rule), unary_tests);
        offer(&mut items, &at_word, &before[word..], word);
      }
    }
    None => {
      invocation = call(before, &syntax);
      let at_cursor = with_parameters(expected(&mut typing, &schema, &scope, before, &syntax, unary_tests), &invocation);
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
  let deprecated = |item: &Item| item.said.as_ref().is_some_and(|said| said.deprecated.is_some());
  items.sort_by(|a, b| {
    order(a.kind)
      .cmp(&order(b.kind))
      .then_with(|| deprecated(a).cmp(&deprecated(b)))
      .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
  });
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
      if let Some(said) = &item.said {
        if let Some(description) = &said.description {
          json["info"] = json!(description);
        }
        if let Some(deprecated) = &said.deprecated {
          json["deprecated"] = deprecated.clone();
        }
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
  let (scope, outline) = scope(req, unread, dialect)?;
  Schema::read(&mut Typing::new(&scope), outline.as_ref())?;
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
