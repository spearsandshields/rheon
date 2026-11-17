//! The editor operations of the module: `syntax` (the tokens and the syntax errors of a text,
//! for highlighting and error marks), `complete` (what can stand at a cursor) and `outline`
//! (the names of a context without its values). See docs/CONTRACT.md §2.
//!
//! They answer for every text: a text that is no expression has errors in the answer, not a
//! failure. Places are offsets in UTF-16 code units, the unit of JavaScript and .NET strings.

use super::*;
use rheon_feel::bif::BUILT_IN_FUNCTION_NAMES;
use rheon_feel::values::Value;
use rheon_feel_parser::{Syntax, TokenClass, parse_tolerant};

/// The most items a `complete` answer holds.
const MAX_ITEMS: usize = 200;

/// The keywords that start an expression, and those that can follow an operand.
const OPERAND_KEYWORDS: [&str; 8] = ["if", "for", "some", "every", "function", "true", "false", "null"];
const OPERATOR_KEYWORDS: [&str; 5] = ["and", "or", "between", "in", "instance of"];
/// The names of the built-in types, and the starts of the types that take types.
const TYPES: [&str; 14] = [
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
  if let Some(outline) = req.remove("outline") {
    if req.contains_key("context") || unread.context.is_some() {
      return fail(Kind::Request, "give 'context' or 'outline', not both");
    }
    unread.context = None;
    req.remove("contextFormat");
    req.insert("context".to_string(), context_of_outline(&outline, 0)?);
  }
  prepare_scope(req, unread, dialect, &mut vec![])
}

/// The JSON context that has the names of an outline: null for a name without entries, an
/// object for `{"names": …}`, a list of one such value for `{"items": …}`.
fn context_of_outline(outline: &Json, depth: usize) -> Result<Json, Failure> {
  const SHAPE: &str = "an outline is null, {\"names\": {<name>: <outline>, …}} or {\"items\": <outline>}";
  if depth > 100 {
    return fail(Kind::Context, "the outline is nested too deeply (at most 100 levels)");
  }
  match outline {
    Json::Null => Ok(Json::Null),
    Json::Object(fields) if fields.len() == 1 => match (fields.get("names"), fields.get("items")) {
      (Some(Json::Object(names)), None) => {
        let mut context = Map::new();
        for (name, entry) in names {
          context.insert(name.clone(), context_of_outline(entry, depth + 1)?);
        }
        Ok(Json::Object(context))
      }
      (None, Some(items)) => Ok(Json::Array(vec![context_of_outline(items, depth + 1)?])),
      _ => fail(Kind::Context, SHAPE),
    },
    _ => fail(Kind::Context, SHAPE),
  }
}

/// The outline of a value: its names when it is a context, the outline of its first item when
/// it is a list, null otherwise.
fn outline_of(value: &Value, depth: usize) -> Json {
  if depth > 100 {
    return Json::Null;
  }
  match value {
    Value::Context(context) => outline_of_context(context, depth),
    Value::List(items) => match items.first().map(|item| outline_of(item, depth + 1)) {
      Some(Json::Null) | None => Json::Null,
      Some(item) => json!({ "items": item }),
    },
    _ => Json::Null,
  }
}

fn outline_of_context(context: &FeelContext, depth: usize) -> Json {
  let names: Map<String, Json> = context.iter().map(|(name, value)| (name.to_string(), outline_of(value, depth + 1))).collect();
  json!({ "names": names })
}

/// `op: "outline"`: the names of the context, nested as its contexts and lists are, without
/// values. The editor operations take it in place of the context.
pub(super) fn outline(req: Map<String, Json>, unread: Unread, started: Instant) -> Result<Json, Failure> {
  let scope = prepare_scope(req, unread, Dialect::Expression, &mut vec![])?;
  let outline = scope.frames().last().map_or_else(|| json!({ "names": {} }), |frame| outline_of_context(frame, 0));
  Ok(json!({ "ok": true, "outline": outline, "elapsedMicros": elapsed_micros(started) }))
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
  let syntax = parse_tolerant(&scope, &expression, dialect == Dialect::UnaryTests);
  let places = Places::new(&expression);
  let tokens: Vec<Json> = syntax.tokens.iter().map(|t| json!([t.class.as_str(), places.of(t.start), places.of(t.end)])).collect();
  let diagnostics: Vec<Json> = syntax
    .diagnostics
    .iter()
    .map(|d| json!({"code": d.code, "message": d.message, "from": places.of(d.start), "to": places.of(d.end)}))
    .collect();
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
}

/// What the grammar takes at a place of the text.
struct Expected {
  /// The name of the expectation in the answer.
  name: String,
  kinds: Vec<Offer>,
}

/// A group of items that fits a place.
enum Offer {
  /// The names in scope: of the context, and those the text declares.
  Names(Vec<String>),
  Functions,
  Keywords(Vec<&'static str>),
  Types,
  /// The entries of the value before a dot.
  Entries(Vec<String>),
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

/// A name as written in the text, with its runs of white space as one space.
fn name_of(text: &str) -> Name {
  Name::from(text.split_whitespace().collect::<Vec<_>>().join(" "))
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
      let mut names: Vec<String> = scope.frames().iter().flat_map(|frame| frame.iter().map(|(name, _)| name.to_string())).collect();
      names.extend(end.names.iter().cloned());
      if unary_tests {
        names.push("?".to_string());
      }
      names.sort_unstable();
      names.dedup();
      Expected {
        name: "expression".to_string(),
        kinds: vec![Offer::Names(names), Offer::Keywords(OPERAND_KEYWORDS.to_vec()), Offer::Functions],
      }
    }
    "expected-name" if end.role == TokenClass::Property => Expected {
      name: "property".to_string(),
      kinds: vec![Offer::Entries(receiver(text, syntax).map(|path| entries(scope, &path)).unwrap_or_default())],
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
  let normal = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
  let typed = normal(typed);
  let mut add = |label: &str, kind: &'static str| {
    if normal(label).starts_with(&typed) && !items.iter().any(|item| item.label == label && item.kind == kind) {
      items.push(Item {
        label: label.to_string(),
        kind,
        from,
      });
    }
  };
  for kind in &expected.kinds {
    match kind {
      Offer::Names(names) => names.iter().for_each(|name| add(name, "name")),
      Offer::Entries(names) => names.iter().for_each(|name| add(name, "entry")),
      Offer::Keywords(keywords) => keywords.iter().for_each(|keyword| add(keyword, "keyword")),
      Offer::Functions => BUILT_IN_FUNCTION_NAMES.iter().for_each(|name| add(name, "function")),
      Offer::Types => TYPES.iter().for_each(|name| add(name, "type")),
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
  let syntax = parse_tolerant(&scope, before, unary_tests);
  let mut items = vec![];
  let expects;
  match typed_name(before, &syntax) {
    // a name is being typed: what fits where it starts, and, for a name of several words, what
    // fits where its last word starts (`if a th` is `if a` and the start of `then`)
    Some((from, typed)) => {
      let head = &before[..from];
      let at_start = expected(&scope, head, &parse_tolerant(&scope, head, unary_tests), unary_tests);
      offer(&mut items, &at_start, typed, from);
      expects = at_start.name;
      if let Some(space) = typed.rfind(char::is_whitespace) {
        let word = from + space + typed[space..].chars().next().map_or(0, char::len_utf8);
        let head = &before[..word];
        let at_word = expected(&scope, head, &parse_tolerant(&scope, head, unary_tests), unary_tests);
        offer(&mut items, &at_word, &before[word..], word);
      }
    }
    None => {
      let at_cursor = expected(&scope, before, &syntax, unary_tests);
      offer(&mut items, &at_cursor, "", cursor);
      expects = at_cursor.name;
    }
  }
  let order = |kind: &str| ["name", "entry", "keyword", "function", "type"].iter().position(|k| *k == kind);
  items.sort_by(|a, b| order(a.kind).cmp(&order(b.kind)).then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase())));
  let truncated = items.len() > MAX_ITEMS;
  items.truncate(MAX_ITEMS);
  let items: Vec<Json> = items
    .iter()
    .map(|item| json!({"label": item.label, "kind": item.kind, "from": places.of(item.from), "type": null, "detail": null}))
    .collect();
  Ok(json!({
    "ok": true,
    "to": places.of(cursor),
    "expects": expects,
    "items": items,
    "truncated": truncated,
    "elapsedMicros": elapsed_micros(started),
  }))
}
