//! The editor's typing against evaluation: the type that the editor operations infer for an
//! expression is a type of the value the expression gives. The expressions are those of the
//! identity request sets (`tools/identity/requests`), each with its context; the inferred type
//! is read from `complete`, which shows the type of the entry `zzq` of `{zzq: (<expression>)}.`,
//! and the value is that of `{zzq: (<expression>)}.zzq`.
//!
//! The inferred type is the type a value has when it is not null, so a null (also inside a
//! list) is of every type; an expression of no known type is not compared. Native only: the
//! request sets are read from the work tree.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value as Json, json};
use std::collections::BTreeSet;
use std::path::Path;

#[path = "shared/big_stack.rs"]
mod big_stack;

/// The request sets that are left out: those whose values are nested or doubled to the limits of
/// the engine, which an entry of a context literal around them exceeds or pays for again; and
/// the set of the key by which `serde_json` hands a number to its tree, which this test's own
/// reading of the file would take for numbers.
const LEFT_OUT: [&str; 5] = ["doubled-values.json", "private-number-key.json", "value-depth.json", "values-deep.json", "values.json"];

/// The expressions whose inferred type evaluation contradicts today: guesses of the typing that
/// are wrong for these values. Each is to be made right or told apart as a guess; an entry that
/// agrees again fails the test, so that it is taken off this list.
const DISAGREE: [&str; 5] = [
  // the items given to these functions join the list, of whatever type they are
  "append([1], [2])",
  "insert before([1, 2], [1], [0])",
  "list replace([1, 2], [1], [9])",
  // a filter whose predicate is a number only when it is evaluated selects one item
  "{f: function(n) f(n + 1), g: 1, r: [{g: f, v: 5}][if (g instance of number) then 1 else g(1)].v}.r",
  "{f: function(n) f(n + 1), g: 1, r: [{g: f, v: 5}][if g instance of number then 1 else g(1)].v}.r",
];

fn call(request: Json) -> Json {
  serde_json::from_str(&feel_wasm::call(&request.to_string())).expect("a response is JSON")
}

/// The requests of `value` that evaluate an expression of FEEL: the expression and what the
/// request has of `context` and `contextFormat`, each once.
fn collect(value: &Json, found: &mut BTreeSet<String>) {
  match value {
    Json::Array(items) => items.iter().for_each(|item| collect(item, found)),
    Json::Object(fields) => {
      if let Some(Json::String(expression)) = fields.get("expression")
        && fields.get("dialect").is_none_or(|dialect| dialect == "expression")
        && !expression.contains("zzq")
      {
        let mut kept = json!({ "expression": expression });
        for field in ["context", "contextFormat"] {
          if let Some(given) = fields.get(field) {
            kept[field] = given.clone();
          }
        }
        found.insert(kept.to_string());
      }
      fields.values().for_each(|inner| collect(inner, found));
    }
    _ => {}
  }
}

/// A type as an item of `complete` shows it: its name and, of a list or a range, the type of
/// what it holds when that is shown.
struct Shown<'t> {
  name: &'t str,
  of: Option<Box<Shown<'t>>>,
}

fn shown(text: &str) -> Shown<'_> {
  match text.find('<') {
    // a function is shown with its signature: only that it is one is compared
    _ if text.starts_with("function") => Shown { name: "function", of: None },
    Some(open) if text.ends_with('>') => Shown {
      name: &text[..open],
      of: Some(Box::new(shown(&text[open + 1..text.len() - 1]))),
    },
    _ => Shown { name: text, of: None },
  }
}

/// Whether the value `typed` (the `typed` form of a result) is of the type `inferred`. A range
/// and a function have no typed form: at the top they are compared by the name of the type of
/// the result (`named`), inside a list they are taken as either.
fn agrees(inferred: &Shown, typed: &Json, named: Option<&str>) -> bool {
  let simple = &typed["simple"];
  if simple["isNil"] == true {
    return true;
  }
  if let Some(kind) = simple["type"].as_str() {
    let kinds: &[&str] = match inferred.name {
      "number" => &["xsd:decimal"],
      "string" => &["xsd:string"],
      "boolean" => &["xsd:boolean"],
      "date" => &["xsd:date"],
      "time" => &["xsd:time"],
      "date and time" => &["xsd:dateTime"],
      "days and time duration" | "years and months duration" => &["xsd:duration"],
      _ => &[],
    };
    return kinds.contains(&kind);
  }
  if let Some(items) = typed["list"]["items"].as_array() {
    return inferred.name == "list" && inferred.of.as_ref().is_none_or(|item| items.iter().all(|value| agrees(item, value, None)));
  }
  if typed["components"].is_array() {
    return inferred.name == "context";
  }
  match named {
    Some(named) => named.starts_with(inferred.name),
    None => matches!(inferred.name, "range" | "function"),
  }
}

#[test]
fn an_inferred_type_is_a_type_of_the_value() {
  big_stack::with_big_stack(|| {
    let sets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/identity/requests");
    let mut files: Vec<_> = std::fs::read_dir(&sets).expect("the request sets").map(|entry| entry.unwrap().path()).collect();
    files.sort();
    let mut requests = BTreeSet::new();
    for file in files {
      let name = file.file_name().unwrap().to_string_lossy().to_string();
      if name.ends_with(".json") && !LEFT_OUT.contains(&name.as_str()) {
        collect(&serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap(), &mut requests);
      }
    }
    let (mut values, mut compared) = (0, 0);
    let mut disagree = BTreeSet::new();
    for request in &requests {
      let request: Json = serde_json::from_str(request).unwrap();
      let expression = request["expression"].as_str().unwrap();
      let with = |op: &str, text: String| {
        let mut asked = request.clone();
        asked["op"] = json!(op);
        asked["expression"] = json!(text);
        call(asked)
      };
      // the line break ends a line comment that ends the expression
      let evaluated = with("evaluate", format!("{{zzq: ({expression}\n)}}.zzq"));
      if evaluated["ok"] != true || evaluated["result"]["isNull"] == true {
        continue;
      }
      values += 1;
      let completed = with("complete", format!("{{zzq: ({expression}\n)}}."));
      let inferred = completed["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["label"] == "zzq"))
        .and_then(|item| item["type"].as_str());
      let Some(inferred) = inferred else {
        continue;
      };
      compared += 1;
      if !agrees(&shown(inferred), &evaluated["result"]["typed"], evaluated["result"]["type"].as_str()) {
        disagree.insert(format!(
          "{expression}\n    inferred {inferred}, value {} of type {}",
          evaluated["result"]["display"], evaluated["result"]["type"]
        ));
      }
    }
    // the sets are large enough to say something
    assert!(values > 3000 && compared > 3000, "{values} values, {compared} compared");
    let found: BTreeSet<&str> = disagree.iter().map(|line| line.split("\n    ").next().unwrap()).collect();
    let expected: BTreeSet<&str> = DISAGREE.into_iter().collect();
    assert_eq!(
      found,
      expected,
      "the inferred types that are no types of their values:\n{}",
      disagree.iter().cloned().collect::<Vec<_>>().join("\n")
    );
  });
}
