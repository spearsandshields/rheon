//! `op: "evaluateEach"`: every answer is the answer of `evaluate` for that entry.

use serde_json::{Value as Json, json};

fn call(request: &Json) -> Json {
  serde_json::from_str(&feel_wasm::call(&request.to_string())).unwrap()
}

/// A response without its time.
fn timeless(mut response: Json) -> Json {
  if let Some(fields) = response.as_object_mut() {
    fields.remove("elapsedMicros");
  }
  response
}

/// The answers of `evaluateEach` with `shared` fields over `entries`, compared with the answers
/// of `evaluate` for each entry; returns them.
fn each(shared: Json, entries: Vec<Json>) -> Vec<Json> {
  let mut request = shared.clone();
  request["op"] = json!("evaluateEach");
  request["entries"] = Json::Array(entries.clone());
  let response = call(&request);
  assert_eq!(json!(true), response["ok"], "{response}");
  let results = response["results"].as_array().unwrap().clone();
  assert_eq!(entries.len(), results.len());
  for (entry, result) in entries.iter().zip(&results) {
    let mut single = shared.clone();
    single["op"] = json!("evaluate");
    if let Some(fields) = entry.as_object() {
      for (key, value) in fields {
        if key != "id" {
          single[key] = value.clone();
        }
      }
    }
    let mut expected = timeless(call(&single));
    if let Some(id) = entry.get("id") {
      expected["id"] = id.clone();
    }
    assert_eq!(expected, timeless(result.clone()), "entry {entry}");
  }
  results
}

#[test]
fn rows_of_the_same_names_and_of_other_names() {
  let results = each(
    json!({ "expression": "unit price * quantity + (if tax rate = null then 0 else tax rate)" }),
    vec![
      json!({ "id": 1, "context": { "unit price": 2.5, "quantity": 4, "tax rate": 1 } }),
      json!({ "id": 2, "context": { "unit price": 10, "quantity": 3, "tax rate": null } }),
      // other names: `unit price` is no name here, so the text reads differently
      json!({ "id": "c", "context": { "unit": 2, "price": 3, "quantity": 4, "tax rate": 0 } }),
      json!({ "id": 4, "context": { "unit price": 1, "quantity": 1, "tax rate": 0.5 } }),
      json!({ "context": {} }),
      json!({}),
    ],
  );
  assert_eq!(json!("11"), results[0]["result"]["display"]);
  assert_eq!(json!("30"), results[1]["result"]["display"]);
  assert_eq!(json!(1), results[0]["id"]);
  assert_eq!(json!("c"), results[2]["id"]);
  assert_eq!(json!("1.5"), results[3]["result"]["display"]);
}

#[test]
fn nested_contexts_lists_and_values_of_other_kinds() {
  each(
    json!({ "expression": "order.customer name + \" \" + string(count(order.lines)) + string(total)" }),
    vec![
      json!({ "context": { "order": { "customer name": "a", "lines": [1, 2] }, "total": 3 } }),
      json!({ "context": { "order": { "customer name": "b", "lines": [] }, "total": 4 } }),
      json!({ "context": { "order": { "customer": "b", "name": "x", "lines": [1] }, "total": 4 } }),
      json!({ "context": { "order": { "customer name": "c", "lines": [{ "customer name": 1 }] }, "total": 5 } }),
      json!({ "context": { "order": null, "total": 5 } }),
    ],
  );
  each(
    json!({ "expression": "a.b c + a.b" }),
    vec![
      json!({ "context": { "a": { "b c": 1, "b": 2 } } }),
      json!({ "context": { "a": { "b": 2 }, "c": 5 } }),
      json!({ "context": { "a": { "b c": 3, "b": 4 } } }),
      json!({ "context": { "a": { "b": 2 }, "c": 6 } }),
    ],
  );
}

#[test]
fn unary_tests_with_inputs() {
  let results = each(
    json!({ "expression": "[18..65), > limit", "dialect": "unaryTests" }),
    vec![
      json!({ "input": 20, "context": { "limit": 100 } }),
      json!({ "input": 70, "context": { "limit": 100 } }),
      json!({ "input": 170, "context": { "limit": 100 } }),
      json!({ "input": "x", "context": { "limit": 100 } }),
      json!({ "context": { "limit": 100, "?": 30 } }),
      json!({ "context": { "limit": 100 } }),
      json!({ "input": [20], "context": { "limit": 100 } }),
    ],
  );
  assert_eq!(json!("true"), results[0]["result"]["display"]);
  assert_eq!(json!("false"), results[1]["result"]["display"]);
  assert_eq!(json!("true"), results[2]["result"]["display"]);
}

#[test]
fn failures_stand_in_their_places() {
  let results = each(
    json!({ "expression": "a +", "errorMode": "strict" }),
    vec![json!({ "id": 1, "context": { "a": 1 } }), json!({ "id": 2, "context": { "a": 2 } })],
  );
  assert_eq!(json!("syntax"), results[0]["error"]["kind"]);
  let results = each(
    json!({ "expression": "a + 1", "errorMode": "strict", "maxSteps": 1000 }),
    vec![
      json!({ "id": 1, "context": { "a": 1 } }),
      json!({ "id": 2, "context": { "a": "x" } }),
      json!({ "id": 3, "context": { "a": 3 } }),
      json!({ "id": 4, "context": "no object" }),
    ],
  );
  assert_eq!(json!(true), results[0]["ok"]);
  assert_eq!(json!(false), results[1]["ok"]);
  assert_eq!(json!("4"), results[2]["result"]["display"]);
  assert_eq!(json!(false), results[3]["ok"]);
  assert_eq!(json!(4), results[3]["id"]);
  // an entry that is no object, and a bad id
  let response = call(&json!({ "op": "evaluateEach", "expression": "1", "entries": [5, { "id": [1] }, {}] }));
  let results = response["results"].as_array().unwrap();
  assert_eq!(json!("'entries[0]' must be an object"), results[0]["error"]["message"]);
  assert_eq!(json!("request"), results[1]["error"]["kind"]);
  assert_eq!(json!("1"), results[2]["result"]["display"]);
}

#[test]
fn failures_of_the_request() {
  for (request, message) in [
    (json!({ "op": "evaluateEach", "expression": "1" }), "missing 'entries'"),
    (json!({ "op": "evaluateEach", "expression": "1", "entries": {} }), "'entries' must be an array"),
    (json!({ "op": "evaluateEach", "entries": [] }), "missing 'expression'"),
  ] {
    let response = call(&request);
    assert_eq!(json!(false), response["ok"], "{response}");
    assert_eq!(json!(message), response["error"]["message"]);
  }
  let response = call(&json!({ "op": "evaluateEach", "expression": "1", "entries": [] }));
  assert_eq!(json!([]), response["results"]);
}

#[test]
fn feel_format_contexts_and_a_request_the_strict_reader_refuses() {
  each(
    json!({ "expression": "a + b", "contextFormat": "feel" }),
    vec![
      json!({ "context": "{a: 1, b: 2}" }),
      json!({ "context": "{a: 1, b: function(x) x}" }),
      json!({ "context": "{a: 3, b: 4}" }),
    ],
  );
  // a key written with an escape makes the request go through the tree of JSON values
  let text = r#"{"op":"evaluateEach","expression":"a * 2","entries":[{"context":{"a":1}},{"context":{"a":2}}],"x":1}"#;
  let response: Json = serde_json::from_str(&feel_wasm::call(text)).unwrap();
  assert_eq!(json!("4"), response["results"][1]["result"]["display"]);
}
