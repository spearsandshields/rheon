//! `op: "explain"` against `evaluate`: an explained evaluation is the evaluation. Over the
//! requests of the identity request sets (`tools/identity/requests`) and of the corpus of the
//! tests: the `result`, the `warnings` and the `diagnostics` of `explain` are those of
//! `evaluate`, whatever nodes are expanded; a failure of one is the failure of the other; two
//! runs give one trace; and the record of the root has the value of the result. Native only:
//! the request sets are read from the work tree.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Map, Value as Json, json};
use std::collections::BTreeMap;
use std::path::Path;

#[path = "shared/big_stack.rs"]
mod big_stack;

fn call(request: &Json) -> Json {
  serde_json::from_str(&feel_wasm::call(&request.to_string())).unwrap_or(Json::Null)
}

/// The requests of `value` that evaluate an expression.
fn collect(value: &Json, found: &mut BTreeMap<String, Json>) {
  match value {
    Json::Array(items) => items.iter().for_each(|item| collect(item, found)),
    Json::Object(fields) => {
      if let Some(Json::String(expression)) = fields.get("expression")
        && expression.len() <= 3_000
        && fields.get("op").is_none_or(|op| op == "evaluate")
        && fields.get("dialect").is_none_or(|dialect| dialect == "expression")
      {
        let mut kept = Map::new();
        for field in ["expression", "context", "contextFormat", "language", "errorMode", "maxSteps"] {
          if let Some(given) = fields.get(field) {
            kept.insert(field.to_string(), given.clone());
          }
        }
        found.insert(Json::Object(kept.clone()).to_string(), Json::Object(kept));
      }
      fields.values().for_each(|inner| collect(inner, found));
    }
    _ => {}
  }
}

/// Checks what the null records of `trace` say of where their nulls are from: a record says
/// that the null is from a part or that it is its own, not both, and only a null record says
/// either; the record that a null is said to be from, when the trace has it, is null. Returns
/// how many records say where their null is from.
fn nulls(trace: &[Json], text: &str) -> usize {
  let mut said = 0;
  for record in trace {
    let (from, own) = (record.get("nullFrom"), record.get("nullOrigin"));
    if record["value"]["isNull"] != true {
      assert!(from.is_none() && own.is_none(), "{text}: a record that is not null says where a null is from: {record}");
      continue;
    }
    assert!(from.is_none() || own.is_none(), "{text}: {record}");
    assert!(own.is_none_or(|own| own == true), "{text}: {record}");
    // why a null is the node's own: with every such record, and with no other
    let code = record.get("nullCode").and_then(Json::as_str);
    assert_eq!(code.is_some(), own.is_some(), "{text}: {record}");
    assert!(
      code.is_none_or(|code| ["missing-entry", "null-input", "operator", "function", "other"].contains(&code)),
      "{text}: {record}"
    );
    if let Some(from) = from {
      assert!(from["at"].is_array(), "{text}: {record}");
      for other in trace {
        if json!([other["from"], other["to"]]) == from["at"] && other.get("within") == from.get("within") {
          assert_eq!(other["value"]["isNull"], true, "{text}: {record} is from {other}");
        }
      }
    }
    said += usize::from(from.is_some() || own.is_some());
  }
  said
}

/// Says which request took long, when a run is asked to (`RHEON_SLOW`).
struct Slow(std::time::Instant, String);

impl Drop for Slow {
  fn drop(&mut self) {
    if std::env::var_os("RHEON_SLOW").is_some() && self.0.elapsed().as_millis() > 500 {
      println!("{:6} ms  {}", self.0.elapsed().as_millis(), self.1);
    }
  }
}

/// A response without what differs between two runs.
fn settled(mut response: Json) -> Json {
  if let Some(fields) = response.as_object_mut() {
    fields.remove("elapsedMicros");
  }
  response
}

#[test]
fn an_explained_evaluation_is_the_evaluation() {
  big_stack::with_big_stack(|| {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<_> = std::fs::read_dir(root.join("../../tools/identity/requests"))
      .expect("the request sets")
      .map(|entry| entry.unwrap().path())
      .collect();
    files.push(root.join("tests/corpus/no-trap.json"));
    files.sort();
    let mut requests = BTreeMap::new();
    for file in files {
      if file.extension().is_some_and(|extension| extension == "json")
        && let Ok(read) = serde_json::from_str::<Json>(&std::fs::read_to_string(&file).unwrap())
      {
        collect(&read, &mut requests);
      }
    }
    let (mut explained, mut failed, mut records, mut expanded, mut iterations, mut calls, mut origins) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    for (number, request) in requests.values().enumerate() {
      if std::env::var_os("RHEON_EACH").is_some() {
        eprintln!("{number} {}", request.to_string().chars().take(300).collect::<String>());
      }
      let began = std::time::Instant::now();
      let _slow = Slow(began, request["expression"].as_str().unwrap_or_default().chars().take(120).collect());
      let with = |op: &str, more: &[(&str, Json)]| {
        let mut asked = request.clone();
        asked["op"] = json!(op);
        asked["now"] = json!("2026-01-01T10:00:00Z");
        for (field, value) in more {
          asked[*field] = value.clone();
        }
        settled(call(&asked))
      };
      // an evaluation that takes long is explained once, not once for each of its nodes
      let started = std::time::Instant::now();
      let evaluated = with("evaluate", &[]);
      let slow = started.elapsed().as_millis() > 20;
      let shallow = with("explain", &[]);
      let text = request["expression"].as_str().unwrap();
      if evaluated["ok"] != true {
        assert_eq!(evaluated, shallow, "{text}: the failure of evaluate is that of explain");
        failed += 1;
        continue;
      }
      let same = |explained: &Json, what: &str| {
        for field in ["ok", "result", "warnings", "diagnostics"] {
          assert_eq!(evaluated[field], explained[field], "{text}: `{field}` of explain {what}");
        }
      };
      same(&shallow, "by default");
      if number % 4 == 0 {
        assert_eq!(shallow, with("explain", &[]), "{text}: two runs, one answer");
      }
      // the record of the root is the result
      let trace = shallow["trace"].as_array().unwrap();
      if let Some(root) = trace.first() {
        let whole = root["from"] == 0 || text.trim_start().len() < text.len();
        if whole && root["value"].get("summarized").is_none() && root["to"].as_u64() == Some(text.trim_end().encode_utf16().count() as u64) {
          assert_eq!(root["value"], evaluated["result"], "{text}: the record of the root");
        }
      }
      // every node of the text expanded at once: the places of the syntax tree
      let syntax = with("syntax", &[("tree", json!(true))]);
      let places: Vec<Json> = syntax["nodes"]
        .as_array()
        .map_or(vec![], |nodes| nodes.iter().map(|node| json!([node[1], node[2]])).collect());
      if slow || places.len() > 400 {
        explained += 1;
        continue;
      }
      // a place of the syntax tree that is the place of no node of the evaluated tree is refused
      let mut known = vec![];
      for (index, place) in places.iter().enumerate() {
        // one by one for every eighth request: the others have all their nodes expanded at once
        if number % 8 == 0 && index < 12 && text.len() <= 300 {
          let one = with("explain", &[("expand", json!([place]))]);
          if one["ok"] == true {
            same(&one, "with a node expanded");
            known.push(place.clone());
          } else {
            assert_eq!(one["error"]["code"], "invalid-field", "{text}: {one}");
          }
        } else {
          known.push(place.clone());
        }
      }
      let mut all = with("explain", &[("expand", json!(known)), ("maxRecords", json!(10_000)), ("reads", json!(true))]);
      // without the places that are those of no node, of a node of a function's body or of a
      // node inside a loop (those are asked for below, with their iterations): found one by one
      while all["ok"] != true && all["error"]["code"] == "invalid-field" && !known.is_empty() {
        let message = all["error"]["message"].as_str().unwrap_or_default().to_string();
        let before = known.len();
        known.retain(|place| !message.contains(&format!("[{}, {}]", place[0], place[1])));
        assert!(known.len() < before, "{text}: {all}");
        all = with("explain", &[("expand", json!(known)), ("maxRecords", json!(10_000)), ("reads", json!(true))]);
      }
      same(&all, "with every node expanded");
      origins += nulls(all["trace"].as_array().unwrap(), text);
      for record in all["trace"].as_array().unwrap() {
        let (from, to) = (record["from"].as_u64().unwrap(), record["to"].as_u64().unwrap());
        assert!(from <= to && to as usize <= text.encode_utf16().count(), "{text}: {record}");
        assert!(record["kind"].is_string() && record["value"].is_object(), "{text}: {record}");
        // the names that the node reads, each a text
        assert!(record["reads"].as_array().is_some_and(|names| names.iter().all(Json::is_string)), "{text}: {record}");
      }
      // the records of the iterations of the loops and of the bodies of the invocations, each
      // expanded where it is evaluated: the predicate of a filter for an item, the body of a
      // loop for an item, the body of a function for an invocation; and those inside them
      let mut asked: std::collections::BTreeSet<String> = Default::default();
      let mut from = all.clone();
      for _ in 0..3 {
        let inside: Vec<Json> = from["trace"]
          .as_array()
          .unwrap()
          .iter()
          .filter(|record| record.get("within").is_some())
          .map(|record| json!({"at": [record["from"], record["to"]], "within": record["within"]}))
          .filter(|entry| asked.insert(entry.to_string()))
          .take(40)
          .collect();
        if inside.is_empty() {
          break;
        }
        let deeper = with("explain", &[("expand", json!(inside)), ("maxRecords", json!(10_000))]);
        same(&deeper, "with the nodes of iterations and of bodies expanded");
        let call = |entry: &&Json| entry["within"].as_array().unwrap().iter().any(|step| step.get("call").is_some());
        calls += inside.iter().filter(call).count();
        iterations += inside.len() - inside.iter().filter(call).count();
        records += deeper["trace"].as_array().unwrap().len();
        origins += nulls(deeper["trace"].as_array().unwrap(), text);
        // a record inside an iteration or a body says which
        for record in deeper["trace"].as_array().unwrap() {
          if let Some(within) = record.get("within") {
            assert!(
              within
                .as_array()
                .is_some_and(|steps| !steps.is_empty() && steps.iter().all(|step| (step["iteration"].is_u64() || step["call"] == true) && step["at"].is_array())),
              "{text}: {record}"
            );
          }
        }
        from = deeper;
      }
      explained += 1;
      records += all["trace"].as_array().unwrap().len();
      expanded += known.len();
    }
    println!(
      "{} requests, {explained} explained, {failed} failed as evaluate does, {expanded} nodes expanded, {iterations} in iterations, {calls} in bodies, {records} records, {origins} nulls with where they are from",
      requests.len()
    );
    assert!(
      explained > 5_000 && records > 20_000 && iterations > 500 && calls > 500 && origins > 1_000,
      "{explained} explained, {records} records, {iterations} in iterations, {calls} in bodies"
    );
  });
}
