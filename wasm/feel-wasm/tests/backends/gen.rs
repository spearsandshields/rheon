//! Random evaluable expressions for the backends harness, with the shrinking of a request whose
//! responses differ.
//!
//! The generator is directed by the grammar and by the scope: most names refer to a binder that
//! exists where they are written (an iteration variable, `partial`, a parameter, a context entry,
//! `item` and the entries of filtered items, a name of the input context), so most expressions
//! evaluate instead of stopping at a missing name. An expression is a tree whose nodes are
//! templates over their children; a request is built from one seed, so a seed reproduces it.

use serde_json::Value as Json;
use std::collections::BTreeMap;

/// A pseudo-random sequence (SplitMix64): the same seed gives the same numbers on every host.
pub struct Random(u64);

impl Random {
  pub fn new(seed: u64) -> Self {
    Self(seed)
  }

  fn next(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = self.0;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
  }

  /// A number in `0..n`.
  fn below(&mut self, n: usize) -> usize {
    (self.next() % n as u64) as usize
  }

  /// `true` with the probability `percent` / 100.
  fn chance(&mut self, percent: usize) -> bool {
    self.below(100) < percent
  }

  fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
    &items[self.below(items.len())]
  }
}

/// A node of an expression: `template` with `{0}`, `{1}`, … standing for the texts of `children`.
/// `construct` names what the node exercises.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
  pub construct: &'static str,
  pub template: String,
  pub children: Vec<Node>,
}

impl Node {
  fn leaf(construct: &'static str, text: impl Into<String>) -> Self {
    Node {
      construct,
      template: text.into(),
      children: vec![],
    }
  }

  fn op(construct: &'static str, template: impl Into<String>, children: Vec<Node>) -> Self {
    Node {
      construct,
      template: template.into(),
      children,
    }
  }

  /// The text of the expression.
  pub fn text(&self) -> String {
    let mut text = String::new();
    let mut rest = self.template.as_str();
    while let Some(start) = rest.find('{') {
      text.push_str(&rest[..start]);
      rest = &rest[start..];
      let digits: String = rest[1..].chars().take_while(char::is_ascii_digit).collect();
      if !digits.is_empty() && rest[1 + digits.len()..].starts_with('}') {
        text.push_str(&self.children[digits.parse::<usize>().unwrap()].text());
        rest = &rest[digits.len() + 2..];
      } else {
        text.push('{');
        rest = &rest[1..];
      }
    }
    text.push_str(rest);
    text
  }

  /// Counts the constructs of the tree.
  fn count(&self, histogram: &mut BTreeMap<&'static str, usize>) {
    *histogram.entry(self.construct).or_default() += 1;
    self.children.iter().for_each(|child| child.count(histogram));
  }

  /// The number of nodes of the tree.
  fn size(&self) -> usize {
    1 + self.children.iter().map(Node::size).sum::<usize>()
  }

  /// The trees that replace one node of this tree by a literal or by one of its children, smaller
  /// ones first.
  fn shrunk(&self) -> Vec<Node> {
    let mut candidates = vec![];
    if !self.children.is_empty() {
      for literal in ["null", "1", "true", "[]"] {
        candidates.push(Node::leaf("literal", literal));
      }
      candidates.extend(self.children.iter().cloned());
    }
    for (index, child) in self.children.iter().enumerate() {
      for smaller in child.shrunk() {
        let mut node = self.clone();
        node.children[index] = smaller;
        candidates.push(node);
      }
    }
    candidates
  }
}

/// A generated request: the expression and the options of the request around it.
#[derive(Clone, Debug, PartialEq)]
pub struct Generated {
  pub seed: u64,
  pub expression: Node,
  /// The fields of the request besides `op` and `expression`.
  pub options: Vec<(&'static str, Json)>,
}

impl Generated {
  /// The request text, as a host sends it.
  pub fn request(&self) -> String {
    let mut request = serde_json::json!({"op": "evaluate", "expression": self.expression.text()});
    for (field, value) in &self.options {
      request[*field] = value.clone();
    }
    request.to_string()
  }

  /// The constructs of the request: those of its expression and those of its options.
  pub fn count(&self, histogram: &mut BTreeMap<&'static str, usize>) {
    self.expression.count(histogram);
    for (field, value) in &self.options {
      let construct = match (*field, value.as_str()) {
        ("dialect", _) => "unary tests request",
        ("language", _) => "b-feel",
        ("errorMode", _) => "strict",
        ("context", _) => "input context",
        _ => continue,
      };
      *histogram.entry(construct).or_default() += 1;
    }
  }

  /// The smallest request found that still `differs`: nodes are replaced by literals or by their
  /// children while the difference stays.
  pub fn shrink(&self, differs: impl Fn(&str) -> bool) -> Generated {
    let mut best = self.clone();
    'smaller: loop {
      for expression in best.expression.shrunk() {
        if expression.size() >= best.expression.size() {
          continue;
        }
        let candidate = Generated { expression, ..best.clone() };
        if differs(&candidate.request()) {
          best = candidate;
          continue 'smaller;
        }
      }
      return best;
    }
  }
}

/// The kinds of values a binder holds, as far as the generator knows them.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
  Number,
  List,
  Context,
  /// A function of two parameters, which takes numbers.
  Function,
}

/// The generator of one expression: its random sequence, the binders in scope and the counter of
/// fresh names.
struct Generator {
  random: Random,
  scope: Vec<(String, Kind)>,
  fresh: usize,
}

/// The keys of the contexts that the generator builds and reads.
const KEYS: [&str; 4] = ["a", "b", "item", "amount"];

impl Generator {
  fn fresh(&mut self, prefix: &str) -> String {
    self.fresh += 1;
    format!("{prefix}{}", self.fresh)
  }

  fn bound(&mut self, kind: Kind) -> Option<String> {
    let names: Vec<&String> = self.scope.iter().filter(|(_, k)| *k == kind).map(|(name, _)| name).collect();
    (!names.is_empty()).then(|| self.random.pick(&names).to_string())
  }

  /// Runs `f` with `binders` added to the scope.
  fn with<T>(&mut self, binders: &[(String, Kind)], f: impl FnOnce(&mut Self) -> T) -> T {
    let depth = self.scope.len();
    self.scope.extend(binders.iter().cloned());
    let result = f(self);
    self.scope.truncate(depth);
    result
  }

  fn number_literal(&mut self) -> Node {
    let text = *self.random.pick(&["0", "1", "2", "3", "-1", "10", "1.5", "0.1", "100"]);
    Node::leaf("number", text)
  }

  fn number(&mut self, depth: usize) -> Node {
    if depth == 0 || self.random.chance(25) {
      if self.random.chance(55)
        && let Some(name) = self.bound(Kind::Number)
      {
        return Node::leaf("name", name);
      }
      return self.number_literal();
    }
    let d = depth - 1;
    match self.random.below(16) {
      0..=3 => {
        let operator = *self.random.pick(&["+", "-", "*", "/", "**"]);
        let (a, b) = (self.number(d), if operator == "**" { self.number_literal() } else { self.number(d) });
        Node::op("arithmetic", format!("({{0}} {operator} {{1}})"), vec![a, b])
      }
      4 => Node::op("arithmetic", "-{0}", vec![self.number(d)]),
      5 => Node::op("if", "if {0} then {1} else {2}", vec![self.boolean(d), self.number(d), self.number(d)]),
      6 => Node::op("call", *self.random.pick(&["count({0})", "sum({0})", "abs(count({0}))"]), vec![self.list(d)]),
      7 => {
        let key = *self.random.pick(&KEYS);
        Node::op("path", format!("{{0}}.{key}"), vec![self.context(d)])
      }
      8 => {
        let index = *self.random.pick(&["1", "-1", "0", "2", "1.5", "-5", "\"x\""]);
        Node::op("filter index", format!("{{0}}[{index}]"), vec![self.list(d)])
      }
      9 if self.random.chance(30) => self.dynamic_scope(d),
      9 => self.function_call(d),
      10 => self.recursion(d),
      11 => {
        let key = *self.random.pick(&KEYS);
        // a repeated key, or an entry that reads the one before it
        let template = if self.random.chance(30) {
          format!("{{{key}: {{0}}, {key}: {{1}}}}.{key}")
        } else {
          format!("{{{key}: {{0}}, c: {key} + {{1}}}}.c")
        };
        Node::op("context", template, vec![self.number(d), self.number(d)])
      }
      12 => Node::op("call", "string length(string({0}))", vec![self.number(d)]),
      13 => {
        let name = self.fresh("v");
        let list = self.list(d);
        let body = self.with(&[(name.clone(), Kind::Number)], |g| g.number(d));
        Node::op("for", format!("sum(for {name} in {{0}} return {{1}})"), vec![list, body])
      }
      14 => match self.bound(Kind::Context) {
        Some(name) => Node::leaf("path", format!("{name}.{}", self.random.pick(&KEYS))),
        None => self.number_literal(),
      },
      _ => Node::op("arithmetic", "({0} + {1})", vec![self.number(d), self.number(d)]),
    }
  }

  fn boolean(&mut self, depth: usize) -> Node {
    if depth == 0 || self.random.chance(15) {
      return Node::leaf("boolean", *self.random.pick(&["true", "false", "null"]));
    }
    let d = depth - 1;
    match self.random.below(11) {
      0..=2 => {
        let operator = *self.random.pick(&["<", "<=", ">", ">=", "=", "!="]);
        Node::op("comparison", format!("{{0}} {operator} {{1}}"), vec![self.number(d), self.number(d)])
      }
      3 => {
        let operator = *self.random.pick(&["and", "or"]);
        Node::op("logic", format!("({{0}} {operator} {{1}})"), vec![self.boolean(d), self.boolean(d)])
      }
      4 => Node::op("logic", "not({0})", vec![self.boolean(d)]),
      5 => Node::op("between", "{0} between {1} and {2}", vec![self.number(d), self.number(d), self.number(d)]),
      6 => {
        // `-` is a unary test of its own, not one of a list in parentheses
        let tests = loop {
          let tests = self.unary_tests(d);
          if tests.template != "-" {
            break tests;
          }
        };
        Node::op("in", "{0} in ({1})", vec![self.number(d), tests])
      }
      7 => {
        let ty = *self
          .random
          .pick(&["number", "string", "boolean", "list<number>", "context<a: number>", "Any", "Null", "function<Any> -> Any"]);
        let value = self.any(d);
        Node::op("instance of", format!("{{0}} instance of {ty}"), vec![value])
      }
      8 | 9 => {
        let quantifier = *self.random.pick(&["some", "every"]);
        let name = self.fresh("v");
        let list = self.list(d);
        let body = self.with(&[(name.clone(), Kind::Number)], |g| g.boolean(d));
        Node::op(
          if quantifier == "some" { "some" } else { "every" },
          format!("{quantifier} {name} in {{0}} satisfies {{1}}"),
          vec![list, body],
        )
      }
      _ => Node::op("in", "{0} in {1}", vec![self.number(d), self.list(d)]),
    }
  }

  fn unary_tests(&mut self, depth: usize) -> Node {
    let d = depth.saturating_sub(1);
    match self.random.below(7) {
      0 => Node::leaf("unary tests", "-"),
      1 => Node::op("unary tests", "{0}, {1}", vec![self.number(d), self.number(d)]),
      2 => Node::op("unary tests", "[{0}..{1}]", vec![self.number(d), self.number(d)]),
      3 => Node::op("unary tests", "not({0})", vec![self.number(d)]),
      4 => Node::op("unary tests", *self.random.pick(&["< {0}", "<= {0}", "> {0}", ">= {0}"]), vec![self.number(d)]),
      5 => Node::op("unary tests", "({0}..{1}], > {2}", vec![self.number(d), self.number(d), self.number(d)]),
      _ => Node::op("unary tests", "? > {0}", vec![self.number(d)]),
    }
  }

  fn list(&mut self, depth: usize) -> Node {
    if depth == 0 || self.random.chance(15) {
      if self.random.chance(40)
        && let Some(name) = self.bound(Kind::List)
      {
        return Node::leaf("name", name);
      }
      return Node::leaf("list", *self.random.pick(&["[]", "[1, 2, 3]", "[3, 1, 2]", "[1, null]"]));
    }
    let d = depth - 1;
    match self.random.below(12) {
      0 | 1 => {
        let items = (0..1 + self.random.below(3)).map(|_| self.number(d)).collect::<Vec<_>>();
        let template = format!("[{}]", (0..items.len()).map(|i| format!("{{{i}}}")).collect::<Vec<_>>().join(", "));
        Node::op("list", template, items)
      }
      2 => {
        let name = self.fresh("v");
        let range = *self.random.pick(&["1..3", "3..1", "1..0", "0..2"]);
        let body = self.with(&[(name.clone(), Kind::Number)], |g| g.number(d));
        Node::op("for", format!("for {name} in {range} return {{0}}"), vec![body])
      }
      3 => {
        let (x, y) = (self.fresh("v"), self.fresh("v"));
        let first = self.list(d);
        let second = self.with(&[(x.clone(), Kind::Number)], |g| g.list(d));
        let body = self.with(&[(x.clone(), Kind::Number), (y.clone(), Kind::Number)], |g| g.number(d));
        Node::op("for", format!("for {x} in {{0}}, {y} in {{1}} return {{2}}"), vec![first, second, body])
      }
      4 => {
        let name = self.fresh("v");
        let count = *self.random.pick(&["3", "5", "1"]);
        let step = self.with(&[(name.clone(), Kind::Number)], |g| g.number(d));
        Node::op(
          "partial",
          format!("for {name} in 1..{count} return if {name} = 1 then 0 else partial[-1] + {{0}}"),
          vec![step],
        )
      }
      5 => {
        let predicate = self.with(&[("item".to_string(), Kind::Number)], |g| g.boolean(d));
        Node::op("filter", "{0}[{1}]", vec![self.list(d), predicate])
      }
      6 => {
        let items = self.contexts(d);
        let predicate = self.with(&[("amount".to_string(), Kind::Number), ("item".to_string(), Kind::Context)], |g| g.boolean(d));
        Node::op("filter", "[{0}, {1}][{2}]", vec![items.0, items.1, predicate])
      }
      7 => {
        let (a, b) = (self.fresh("p"), self.fresh("p"));
        let order = self.with(&[(a.clone(), Kind::Number), (b.clone(), Kind::Number)], |g| g.boolean(d));
        Node::op("sort", format!("sort({{0}}, function({a}, {b}) {{1}})"), vec![self.list(d), order])
      }
      8 => {
        let (a, b) = (self.fresh("p"), self.fresh("p"));
        let matches = self.with(&[(a.clone(), Kind::Number), (b.clone(), Kind::Number)], |g| g.boolean(d));
        Node::op(
          "list replace",
          format!("list replace({{0}}, function({a}, {b}) {{1}}, {{2}})"),
          vec![self.list(d), matches, self.number(d)],
        )
      }
      9 => Node::op("call", "append({0}, {1})", vec![self.list(d), self.number(d)]),
      10 => Node::op(
        "path",
        "{0}.amount",
        vec![Node::op("list", "[{0}, {1}]", {
          let items = self.contexts(d);
          vec![items.0, items.1]
        })],
      ),
      _ => Node::op("call", "reverse({0})", vec![self.list(d)]),
    }
  }

  /// Two context literals, as the items of a list to filter.
  fn contexts(&mut self, depth: usize) -> (Node, Node) {
    (self.context(depth), self.context(depth))
  }

  fn context(&mut self, depth: usize) -> Node {
    if depth == 0 || self.random.chance(20) {
      if self.random.chance(40)
        && let Some(name) = self.bound(Kind::Context)
      {
        return Node::leaf("name", name);
      }
      return Node::leaf("context", *self.random.pick(&["{a: 1, amount: 5}", "{}", "{item: 2, b: 3}", "{amount: 50}"]));
    }
    let d = depth - 1;
    let key = *self.random.pick(&KEYS);
    match self.random.below(4) {
      0 => Node::op("context", format!("{{{key}: {{0}}, amount: {{1}}}}"), vec![self.number(d), self.number(d)]),
      // an entry that reads an earlier one, and one that reads a later one
      1 => Node::op("context", format!("{{{key}: {{0}}, amount: {key} + 1}}"), vec![self.number(d)]),
      2 => Node::op("context", format!("{{amount: {key} + 1, {key}: {{0}}}}"), vec![self.number(d)]),
      _ => Node::op("context", format!("{{{key}: {{0}}, abs: {{1}}, partial: 1}}"), vec![self.number(d), self.number(d)]),
    }
  }

  fn function_call(&mut self, depth: usize) -> Node {
    if let Some(name) = self.bound(Kind::Function)
      && self.random.chance(50)
    {
      return Node::op("call", format!("{name}({{0}}, {{1}})"), vec![self.number(depth), self.number(depth)]);
    }
    let (f, x, y) = (self.fresh("f"), self.fresh("x"), self.fresh("x"));
    let typed = self.random.chance(30);
    let parameters = if typed { format!("{x}: number, {y}") } else { format!("{x}, {y}") };
    let body = self.with(&[(x.clone(), Kind::Number), (y.clone(), Kind::Number)], |g| g.number(depth));
    let arguments = match self.random.below(6) {
      0 => format!("{y}: {{2}}, {x}: {{3}}"),
      1 => "{2}".to_string(),
      2 => "\"a\", {2}".to_string(),
      _ => "{2}, {3}".to_string(),
    };
    let construct = if arguments.contains(':') {
      "named call"
    } else if typed {
      "typed parameter"
    } else {
      "function"
    };
    let tail = self.with(&[(f.clone(), Kind::Function)], |g| g.number(depth));
    Node::op(
      construct,
      format!("{{{f}: function({parameters}) {{0}}, r: {f}({arguments}) + {{1}}}}.r"),
      vec![body, tail, self.number(depth), self.number(depth)],
    )
  }

  /// A function whose body reads a name that the definition does not see and that only the
  /// caller binds where it calls the function: a loop variable, `partial`, the item of a filter,
  /// an entry of the item, or an entry after the definition. The body reads it through the
  /// scopes of its caller.
  fn dynamic_scope(&mut self, depth: usize) -> Node {
    let (f, x, v) = (self.fresh("f"), self.fresh("x"), self.fresh("v"));
    let body = self.with(&[(x.clone(), Kind::Number)], |g| g.number(depth));
    let (template, other) = match self.random.below(5) {
      0 => (format!("{{{f}: function({x}) {{0}} + {v}, r: sum(for {v} in {{1}} return {f}({v}))}}.r"), self.list(depth)),
      1 => (
        format!("{{{f}: function({x}) {{0}} + count(partial), r: sum(for {v} in {{1}} return {f}({v}))}}.r"),
        self.list(depth),
      ),
      2 => (format!("{{{f}: function({x}) {{0}} + item, r: count({{1}}[{f}(0) > 1])}}.r"), self.list(depth)),
      3 => (
        format!("{{{f}: function({x}) {{0}} + amount, r: count([{{a: 1, amount: 5}}, {{amount: 50}}][{f}(1) > {{1}}])}}.r"),
        self.number(depth),
      ),
      _ => (format!("{{{f}: function({x}) {{0}} + {v}, {v}: {{1}}, r: {f}(1)}}.r"), self.number(depth)),
    };
    Node::op("dynamic scope", template, vec![body, other])
  }

  fn recursion(&mut self, depth: usize) -> Node {
    let (f, n) = (self.fresh("f"), self.fresh("n"));
    let count = *self.random.pick(&["3", "10", "150", "199", "200", "250"]);
    let step = self.with(&[(n.clone(), Kind::Number)], |g| g.number(depth.min(1)));
    Node::op(
      "recursion",
      format!("{{{f}: function({n}) if {n} <= 0 then 0 else {f}({n} - 1) + {{0}}, r: {f}({count})}}.r"),
      vec![step],
    )
  }

  fn any(&mut self, depth: usize) -> Node {
    match self.random.below(5) {
      0 => self.boolean(depth),
      1 => self.list(depth),
      2 => self.context(depth),
      3 => Node::leaf("string", *self.random.pick(&["\"a\"", "\"\"", "\"x y\""])),
      _ => self.number(depth),
    }
  }
}

/// The input context of a request that has one: a JSON context with a number, a list and a
/// context, which the generator's names read.
fn input_context() -> Json {
  serde_json::json!({"n": 4, "l": [1, 2, 3], "c": {"a": 1, "amount": 7}})
}

/// The request of `seed`.
pub fn generate(seed: u64) -> Generated {
  let mut generator = Generator {
    random: Random::new(seed),
    scope: vec![],
    fresh: 0,
  };
  let mut options = vec![];
  if generator.random.chance(20) {
    options.push(("context", input_context()));
    generator
      .scope
      .extend([("n".to_string(), Kind::Number), ("l".to_string(), Kind::List), ("c".to_string(), Kind::Context)]);
  }
  let depth = 1 + generator.random.below(4);
  let expression = if generator.random.chance(10) {
    options.push(("dialect", "unaryTests".into()));
    options.push(("input", serde_json::json!(generator.random.below(12))));
    generator.unary_tests(depth)
  } else {
    match generator.random.below(3) {
      0 => generator.boolean(depth),
      1 => generator.list(depth),
      _ => generator.number(depth),
    }
  };
  if generator.random.chance(15) {
    options.push(("language", "b-feel".into()));
  }
  if generator.random.chance(15) {
    options.push(("errorMode", "strict".into()));
  }
  Generated { seed, expression, options }
}

/// The first seed of the harness's random requests.
pub const FIRST_SEED: u64 = 0xB4C3_E2D5;

/// The seeds of the harness: `RHEON_BACKENDS_SEED` alone when it is set, otherwise `count`
/// seeds from [FIRST_SEED].
pub fn seeds(count: u64) -> Vec<u64> {
  match std::env::var("RHEON_BACKENDS_SEED") {
    Ok(seed) => vec![seed.parse().expect("RHEON_BACKENDS_SEED is a number")],
    Err(_) => (FIRST_SEED..FIRST_SEED + count).collect(),
  }
}

/// The constructs that every run of the default size exercises.
pub const CONSTRUCTS: [&str; 31] = [
  "arithmetic",
  "between",
  "b-feel",
  "boolean",
  "call",
  "comparison",
  "context",
  "dynamic scope",
  "every",
  "filter",
  "filter index",
  "for",
  "function",
  "if",
  "in",
  "input context",
  "instance of",
  "list",
  "list replace",
  "logic",
  "name",
  "named call",
  "number",
  "partial",
  "path",
  "recursion",
  "some",
  "sort",
  "strict",
  "typed parameter",
  "unary tests request",
];
