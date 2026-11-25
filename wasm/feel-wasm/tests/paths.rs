//! `op: "paths"` against evaluation: an evaluation with a context that holds only what `paths`
//! lists gives what the evaluation with the whole context gives. Over the requests of the
//! identity request sets (`tools/identity/requests`) and of the corpus of the tests that have a
//! context as JSON. The context is cut down to the paths: a name that no path starts with is
//! taken out; of a value at a path that is not read whole only what the paths below it read is
//! kept (a context keeps the entries that paths name, a list its items cut down the same way,
//! a simple value stays as it is; an item of a list that no
//! path lists becomes an empty context or null). Native only: the request sets are read from the work tree.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Map, Value as Json, json};
use std::collections::BTreeMap;
use std::path::Path;

#[path = "shared/big_stack.rs"]
mod big_stack;

fn call(request: &Json) -> Json {
  serde_json::from_str(&feel_wasm::call(&request.to_string())).unwrap_or(Json::Null)
}

/// The requests of `value` that evaluate an expression with a context that is a JSON object.
fn collect(value: &Json, found: &mut BTreeMap<String, Json>) {
  match value {
    Json::Array(items) => items.iter().for_each(|item| collect(item, found)),
    Json::Object(fields) => {
      if let (Some(Json::String(_)), Some(Json::Object(_))) = (fields.get("expression"), fields.get("context"))
        && fields.get("contextFormat").is_none_or(|format| format == "json")
        && fields.get("op").is_none_or(|op| op == "evaluate")
      {
        let mut kept = Map::new();
        for field in ["expression", "context", "dialect", "input", "language"] {
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

/// What the paths read below one place: whether the value there is read whole, its entries by
/// name and its items.
#[derive(Default)]
struct Kept {
  whole: bool,
  entries: BTreeMap<String, Kept>,
  items: Option<Box<Kept>>,
}

/// A key without the white space of FEEL at its ends: the engine reads a key so.
fn trimmed(key: &str) -> &str {
  key.trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '\u{200b}' | '\u{feff}' | '\u{180e}' | '\u{1680}' | '\u{2028}' | '\u{2029}'))
}

/// What stands for an item of a list that no path lists: an empty context or list for one,
/// null for a simple value.
fn unlisted(item: &Json) -> Json {
  match item {
    Json::Object(_) => json!({}),
    Json::Array(items) => Json::Array(items.iter().map(unlisted).collect()),
    _ => Json::Null,
  }
}

/// What both `a` and `b` read.
fn merged(a: &Kept, b: &Kept) -> Kept {
  let mut both = Kept {
    whole: a.whole || b.whole,
    entries: BTreeMap::new(),
    items: match (&a.items, &b.items) {
      (Some(a), Some(b)) => Some(Box::new(merged(a, b))),
      (Some(one), None) | (None, Some(one)) => Some(Box::new(merged(one, &Kept::default()))),
      (None, None) => None,
    },
  };
  for (name, kept) in a.entries.iter().chain(&b.entries) {
    let merged = match both.entries.get(name) {
      Some(other) => merged(other, kept),
      None => merged(kept, &Kept::default()),
    };
    both.entries.insert(name.clone(), merged);
  }
  both
}

/// The value cut down to what `kept` reads of it. The paths of a text alone do not know which
/// values are lists, so they are read as FEEL reads the values: the items of a value that is
/// no list are the value itself, and an entry of a list is the entry of each of its items.
fn cut(value: &Json, kept: &Kept) -> Json {
  if kept.whole {
    return value.clone();
  }
  match value {
    Json::Object(fields) => {
      // the items of a context: the context
      let kept = match &kept.items {
        Some(items) => {
          let entries = Kept {
            whole: false,
            entries: kept.entries.iter().map(|(name, kept)| (name.clone(), merged(kept, &Kept::default()))).collect(),
            items: None,
          };
          merged(&entries, items)
        }
        None => merged(kept, &Kept::default()),
      };
      if kept.whole || kept.items.is_some() {
        return cut(value, &kept);
      }
      Json::Object(
        fields
          .iter()
          .filter_map(|(key, value)| kept.entries.get(trimmed(key)).map(|kept| (key.clone(), cut(value, kept))))
          .collect(),
      )
    }
    Json::Array(items) => {
      // an item that no path lists is not given: the list keeps its length
      if kept.items.is_none() && kept.entries.is_empty() {
        return Json::Array(items.iter().map(unlisted).collect());
      }
      // an entry of a list: of its items
      let entries = Kept {
        whole: false,
        entries: kept.entries.iter().map(|(name, kept)| (name.clone(), merged(kept, &Kept::default()))).collect(),
        items: None,
      };
      let of_items = match &kept.items {
        Some(items) => merged(&entries, items),
        None => entries,
      };
      Json::Array(items.iter().map(|item| cut(item, &of_items)).collect())
    }
    // a simple value at a path that is listed is given as it is
    _ => value.clone(),
  }
}

/// The requests compared so far.
#[derive(Default)]
struct Tally {
  compared: usize,
  /// Those whose context the paths cut down.
  cut_down: usize,
  /// Those whose paths are not complete: not compared.
  incomplete: usize,
  /// Those with a value that is not null.
  values: usize,
  wrong: Vec<String>,
}

impl Tally {
  /// Evaluates `request` with its context and with the context cut down to its paths. With
  /// `typed` the paths are asked for with the context, so that they know which values are
  /// lists; without, only for the text.
  fn check(&mut self, request: &Json, typed: bool) {
    let with = |op: &str, context: &Json| {
      let mut asked = request.clone();
      asked["op"] = json!(op);
      asked["context"] = context.clone();
      asked["now"] = json!("2026-01-01T10:00:00Z");
      call(&asked)
    };
    let context = &request["context"];
    let full = with("evaluate", context);
    let paths = if typed {
      with("paths", context)
    } else {
      let mut asked = request.clone();
      asked["op"] = json!("paths");
      asked.as_object_mut().unwrap().remove("context");
      call(&asked)
    };
    if full["ok"] != true || paths["ok"] != true {
      return;
    }
    if paths["complete"] != true {
      self.incomplete += 1;
      return;
    }
    let mut kept = Kept::default();
    for path in paths["paths"].as_array().unwrap() {
      let mut at = &mut kept;
      for segment in path["path"].as_array().unwrap() {
        at = match segment.as_str() {
          Some(name) => at.entries.entry(name.to_string()).or_default(),
          None => at.items.get_or_insert_with(Box::default),
        };
      }
      at.whole |= path["whole"] == true;
    }
    let cut_context = cut(context, &kept);
    let cut_result = with("evaluate", &cut_context);
    self.compared += 1;
    self.cut_down += usize::from(cut_context != *context);
    self.values += usize::from(full["result"]["isNull"] != true);
    let same = ["display", "type", "typed"].iter().all(|field| cut_result["result"][field] == full["result"][field]);
    if cut_result["ok"] != true || !same {
      self.wrong.push(format!(
        "{}\n    paths {}\n    cut to {}\n    whole: {}\n    cut:   {}",
        request["expression"],
        paths["paths"]
          .as_array()
          .unwrap()
          .iter()
          .map(|path| format!("{}{}", path["text"].as_str().unwrap(), if path["whole"] == true { "*" } else { "" }))
          .collect::<Vec<_>>()
          .join(", "),
        cut_context,
        full["result"]["display"],
        if cut_result["ok"] == true {
          cut_result["result"]["display"].to_string()
        } else {
          cut_result["error"].to_string()
        }
      ));
    }
  }
}

/// A source of numbers that gives the same numbers in every run.
struct Rng(u64);

impl Rng {
  fn below(&mut self, n: usize) -> usize {
    self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((self.0 >> 33) as usize) % n
  }

  fn pick<'t>(&mut self, of: &[&'t str]) -> &'t str {
    of[self.below(of.len())]
  }
}

const ROOTS: [&str; 7] = ["loan", "loans", "rate", "n", "flag", "label", "book"];
const FIELDS: [&str; 10] = ["amount", "customer", "name", "age", "tags", "payments", "paid", "on", "y", "loans"];

/// A random expression over the names of [context] and the variables `bound`, at most `depth`
/// constructs deep: every construct that passes a value on, and those that read one whole.
fn expression(rng: &mut Rng, bound: &mut Vec<String>, depth: usize) -> String {
  if depth == 0 || rng.below(7) == 0 {
    return match rng.below(8) {
      0 if !bound.is_empty() => bound[rng.below(bound.len())].clone(),
      1 if !bound.is_empty() => format!("{}.{}", bound[rng.below(bound.len())], rng.pick(&FIELDS)),
      2 => (rng.below(3) + 1).to_string(),
      3 => rng.pick(&FIELDS).to_string(),
      _ => rng.pick(&ROOTS).to_string(),
    };
  }
  let depth = depth - 1;
  let e = |rng: &mut Rng, bound: &mut Vec<String>| expression(rng, bound, depth);
  let variable = format!("v{}", bound.len());
  let scoped = |rng: &mut Rng, bound: &mut Vec<String>, name: &str| {
    bound.push(name.to_string());
    let body = expression(rng, bound, depth);
    bound.pop();
    body
  };
  match rng.below(34) {
    0..=4 => format!("{}.{}", e(rng, bound), rng.pick(&FIELDS)),
    5 | 6 => format!("{}[{}]", e(rng, bound), rng.below(3) + 1),
    7 | 8 => format!("{}[{} > {}]", e(rng, bound), rng.pick(&FIELDS), e(rng, bound)),
    9 => format!("{}[item.{} = {}]", e(rng, bound), rng.pick(&FIELDS), e(rng, bound)),
    10 => format!("{}[{}]", e(rng, bound), scoped(rng, bound, "item")),
    11 | 12 => format!("(for {variable} in {} return {})", e(rng, bound), scoped(rng, bound, &variable)),
    13 => format!("(if {} then {} else {})", e(rng, bound), e(rng, bound), e(rng, bound)),
    14 => format!("{{a: {}, b: {}, c: a}}.{}", e(rng, bound), scoped(rng, bound, "a"), rng.pick(&["a", "b", "c"])),
    15 => format!("[{}, {}]", e(rng, bound), e(rng, bound)),
    16 => format!("reverse({})", e(rng, bound)),
    17 => format!("sublist({}, 1, 2)", e(rng, bound)),
    18 => format!("append({}, {})", e(rng, bound), e(rng, bound)),
    19 => format!("concatenate({}, {})", e(rng, bound), e(rng, bound)),
    20 => format!("get value({}, \"{}\")", e(rng, bound), rng.pick(&FIELDS)),
    21 => format!("(function({variable}) {})({})", scoped(rng, bound, &variable), e(rng, bound)),
    22 => format!("{{f: function({variable}) {}, r: f({})}}.r", scoped(rng, bound, &variable), e(rng, bound)),
    23 => format!(
      "{{f: function({variable}) function() {}, g: f({}), r: g()}}.r",
      scoped(rng, bound, &variable),
      e(rng, bound)
    ),
    24 => format!("sort({}, function(p, q) p.amount < q.amount)", e(rng, bound)),
    25 => format!("{}({})", rng.pick(&["count", "sum", "string", "flatten", "distinct values", "min", "max"]), e(rng, bound)),
    26 => format!("({} {} {})", e(rng, bound), rng.pick(&["=", "+", "<", "and", "in"]), e(rng, bound)),
    27 => format!("(some {variable} in {} satisfies {})", e(rng, bound), scoped(rng, bound, &variable)),
    28 => format!("insert before({}, 1, {})", e(rng, bound), e(rng, bound)),
    29 => format!("remove({}, 1)", e(rng, bound)),
    30 => format!("{{f: function(p, q) {}, r: f(q: {}, p: {})}}.r", scoped(rng, bound, "p"), e(rng, bound), e(rng, bound)),
    31 => format!("(for {variable} in {} return {{k: {}, t: partial}}.k)", e(rng, bound), scoped(rng, bound, &variable)),
    32 => format!("{{x: {}, r: {}}}.r", e(rng, bound), scoped(rng, bound, "x")),
    _ => format!("{}.{}.{}", rng.pick(&ROOTS), rng.pick(&FIELDS), rng.pick(&FIELDS)),
  }
}

/// A context with values of every shape: contexts in contexts, lists of contexts, lists of
/// lists, simple values, and names that are entries of items too.
fn context() -> Json {
  let loan = |amount: u32, name: &str| {
    json!({
      "amount": amount, "paid": amount.is_multiple_of(2),
      "customer": {"name": name, "age": amount + 20, "tags": ["a", name], "on": {"y": amount}},
      "payments": [{"amount": amount, "paid": true, "on": {"y": 1}}, {"amount": 2, "paid": false, "on": {"y": 2}}],
      "tags": [amount, 2], "on": {"y": 3}})
  };
  json!({
    "loan": loan(1, "ann"),
    "loans": [loan(3, "bob"), loan(1, "cy"), loan(2, "di")],
    "rate": 2, "n": [3, 1, 2], "flag": true, "label": "amount", "amount": 2, "name": "bob",
    "book": {"loans": [loan(2, "ed")], "customer": {"name": "fay", "age": 1}, "amount": 1, "y": 2}})
}

/// The same over random expressions of every construct that passes a value on, in a context
/// of every shape.
#[test]
fn a_random_evaluation_reads_no_more_than_its_paths() {
  big_stack::with_big_stack(|| {
    let count = if std::env::var_os("RHEON_FULL_TESTS").is_some() { 200_000 } else { 20_000 };
    let context = context();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut tally = Tally::default();
    let mut untyped = Tally::default();
    for _ in 0..count {
      let text = expression(&mut rng, &mut vec![], 4);
      let request = json!({"expression": text, "context": context});
      tally.check(&request, true);
      untyped.check(&request, false);
    }
    println!(
      "without the context: {} compared, {} with a context cut down, {} wrong",
      untyped.compared,
      untyped.cut_down,
      untyped.wrong.len()
    );
    for case in untyped.wrong.iter().take(30) {
      println!("{}", case.chars().take(1500).collect::<String>());
    }
    assert!(
      untyped.wrong.is_empty(),
      "{} evaluations read more than their paths, asked without the context",
      untyped.wrong.len()
    );
    let Tally {
      compared,
      cut_down,
      incomplete,
      values,
      wrong,
    } = tally;
    println!(
      "{count} expressions, {compared} compared, {cut_down} with a context cut down, {values} with a value, {incomplete} not complete, {} wrong",
      wrong.len()
    );
    for case in wrong.iter().take(30) {
      println!("{}", case.chars().take(1500).collect::<String>());
    }
    assert!(wrong.is_empty(), "{} evaluations read more than their paths", wrong.len());
    assert!(compared > count * 9 / 10 && values > count / 5, "{compared} compared, {values} with a value");
  });
}

#[test]
fn an_evaluation_reads_no_more_than_its_paths() {
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
    let mut tally = Tally::default();
    for request in requests.values() {
      tally.check(request, true);
    }
    let Tally {
      compared,
      cut_down,
      incomplete,
      wrong,
      ..
    } = tally;
    println!(
      "{} requests, {compared} compared, {cut_down} with a context cut down, {incomplete} not complete, {} wrong",
      requests.len(),
      wrong.len()
    );
    for case in wrong.iter().take(25) {
      println!("{}", case.chars().take(900).collect::<String>());
    }
    assert!(wrong.is_empty(), "{} evaluations read more than their paths", wrong.len());
    assert!(compared > 400 && cut_down > 100, "{compared} compared, {cut_down} cut down");
  });
}
