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

/// Simple deterministic generator (splitmix64).
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
  }

  fn below(&mut self, n: usize) -> usize {
    (self.next() % n as u64) as usize
  }

  fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
    items[self.below(items.len())]
  }
}

/// Names that are parts of each other, so that the names in scope decide how a text reads.
const KEYS: [&str; 14] = ["a", "b", "c", "a b", "b c", "a b c", "unit", "price", "unit price", "a-b", "a  b", "x", "in stock", "date"];

/// A value of a random kind: mostly a scalar, sometimes a context or a list.
fn random_value(rng: &mut Rng, depth: usize) -> Json {
  match rng.below(if depth < 3 { 12 } else { 7 }) {
    0..=2 => json!(rng.below(50)),
    3 => json!(rng.pick(&["open", "a", "x y", ""])),
    4 => json!(rng.below(2) == 0),
    5 => Json::Null,
    6 => json!(rng.below(1000) as f64 / 8.0),
    7..=9 => random_context(rng, depth + 1, None),
    10 => Json::Array((0..rng.below(3)).map(|_| random_value(rng, depth + 1)).collect()),
    _ => Json::Array((0..1 + rng.below(2)).map(|_| random_context(rng, depth + 1, None)).collect()),
  }
}

/// A context with the keys `keys`, or with random ones.
fn random_context(rng: &mut Rng, depth: usize, keys: Option<&[&str]>) -> Json {
  let chosen: Vec<&str> = match keys {
    Some(keys) => keys.to_vec(),
    None => (0..rng.below(5)).map(|_| rng.pick(&KEYS)).collect(),
  };
  let mut context = serde_json::Map::new();
  for key in chosen {
    context.insert(key.to_string(), random_value(rng, depth));
  }
  Json::Object(context)
}

/// A text of names and operators: an expression in some contexts, none in others.
fn random_expression(rng: &mut Rng) -> String {
  let mut text = String::new();
  for part in 0..1 + rng.below(4) {
    if part > 0 {
      text.push_str(rng.pick(&[" + ", " - ", " * ", ".", " . ", " ", " = ", "-", " and "]));
    }
    text.push_str(rng.pick(&[
      "a",
      "b",
      "c",
      "a b",
      "b c",
      "a b c",
      "unit price",
      "unit",
      "price",
      "a-b",
      "x",
      "in stock",
      "date",
      "1",
      "\"s\"",
      "count(a)",
      "a[1]",
      "{a: 1}.a",
      "b c.a",
    ]));
  }
  text
}

/// The number of generated requests: more with `RHEON_FULL_TESTS`.
fn cases() -> usize {
  if std::env::var_os("RHEON_FULL_TESTS").is_some() { 40_000 } else { 1_500 }
}

#[test]
fn every_answer_is_that_of_evaluate_for_generated_contexts() {
  let mut rng = Rng(0xEAC4);
  let (mut answered, mut reused_shapes) = (0, 0);
  for _ in 0..cases() {
    let unary = rng.below(5) == 0;
    let expression = if unary {
      format!("{}, > {}", random_expression(&mut rng), random_expression(&mut rng))
    } else {
      random_expression(&mut rng)
    };
    let mut shared = json!({ "expression": expression });
    if unary {
      shared["dialect"] = json!("unaryTests");
    }
    if rng.below(4) == 0 {
      shared["errorMode"] = json!("strict");
    }
    // runs of contexts with the same keys, whose values are of other kinds from entry to entry
    let mut keys: Vec<&str> = vec![];
    let mut entries = vec![];
    for _ in 0..2 + rng.below(10) {
      if keys.is_empty() || rng.below(3) == 0 {
        keys = (0..rng.below(6)).map(|_| rng.pick(&KEYS)).collect();
      } else {
        reused_shapes += 1;
      }
      let mut entry = json!({ "context": random_context(&mut rng, 0, Some(&keys)) });
      if unary && rng.below(4) != 0 {
        entry["input"] = random_value(&mut rng, 2);
      }
      entries.push(entry);
    }
    let results = each(shared, entries);
    answered += results.iter().filter(|result| result["ok"] == json!(true)).count();
  }
  // the generator gives both: texts that are expressions, and runs of the same keys
  assert!(answered > cases() && reused_shapes > cases(), "{answered} answered, {reused_shapes} runs");
}

/// `evaluateAll` parses its evaluations with one index of the names of its context: every answer
/// is that of `evaluate` with the same context, for expressions and unary tests, with and without
/// a tested value, in contexts of few and of many names.
#[test]
fn every_answer_of_evaluate_all_is_that_of_evaluate() {
  let mut rng = Rng(0xA11);
  let mut answered = 0;
  for round in 0..cases() / 5 {
    let mut context = random_context(&mut rng, 0, None);
    if round % 3 == 0 {
      // many names, of several words: the index of the names is read for every text
      for index in 0..40 {
        context[format!("total price {index}")] = json!(index);
        context[format!("a b {index}")] = random_value(&mut rng, 2);
      }
    }
    let mut evaluations = vec![];
    for id in 0..2 + rng.below(8) {
      let unary = rng.below(4) == 0;
      let mut evaluation = if unary {
        json!({ "id": id, "dialect": "unaryTests", "expression": format!("{}, > {}", random_expression(&mut rng), random_expression(&mut rng)) })
      } else if rng.below(3) == 0 {
        json!({ "id": id, "expression": format!("total price {} + a b {} + {}", rng.below(45), rng.below(45), random_expression(&mut rng)) })
      } else {
        json!({ "id": id, "expression": random_expression(&mut rng) })
      };
      if unary && rng.below(4) != 0 {
        evaluation["input"] = random_value(&mut rng, 2);
      }
      if rng.below(4) == 0 {
        evaluation["errorMode"] = json!("strict");
      }
      evaluations.push(evaluation);
    }
    let response = call(&json!({ "op": "evaluateAll", "context": context, "evaluations": evaluations }));
    assert_eq!(json!(true), response["ok"], "{response}");
    let results = response["results"].as_array().unwrap();
    assert_eq!(evaluations.len(), results.len());
    for (evaluation, result) in evaluations.iter().zip(results) {
      let mut single = evaluation.clone();
      single["op"] = json!("evaluate");
      single["context"] = context.clone();
      let id = single.as_object_mut().unwrap().remove("id").unwrap();
      let mut expected = timeless(call(&single));
      expected["id"] = id;
      assert_eq!(expected, timeless(result.clone()), "{evaluation} in {context}");
      answered += usize::from(result["ok"] == json!(true));
    }
  }
  assert!(answered > cases() / 5, "{answered} answered");
}
