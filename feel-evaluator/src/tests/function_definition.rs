use super::*;
use rheon_feel::{FeelType, scope};

#[test]
fn _0001() {
  let node = AstNode::FunctionDefinition(
    Box::new(AstNode::FeelType(FeelType::Boolean)),
    Box::new(AstNode::FunctionBody(Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())), false)),
  );
  assert_eq!(
    r#"null(malformed expression tree: unexpected FunctionDefinition node)"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

#[test]
fn _0002() {
  // FEEL functions are lexical closures (DMN 1.6 §10.3.2.13.2): `k` sees the names in scope
  // where it is written, where no `a` is. That `f`, which returns it, has a parameter `a` gives
  // `k` none: `r` is null, as `s` is.
  let expression = "{k: function(y) y + a, f: function(a) k, r: f(100)(1), s: k(1)}";
  te_null(
    false,
    &scope!(),
    &format!("{expression}.r"),
    "[addition] incompatible types: 1 + null(context has no value for key 'a')",
  );
  te_null(
    false,
    &scope!(),
    &format!("{expression}.s"),
    "[addition] incompatible types: 1 + null(context has no value for key 'a')",
  );
}

/// A function body reads a name from the nearest scope around the function that has it (DMN
/// 1.6 clause 10.3.2.11: the contexts of a scope are looked through from the first, the
/// innermost, to the last): a parameter, a variable of a loop or the item of a filter that the
/// function is written in before an entry of a context literal further out; an entry of a
/// context literal, also one written later, before a name further out.
#[test]
fn a_function_reads_a_name_from_the_nearest_scope_around_it() {
  for (text, expected) in [
    ("{x: \"s\", f: function(x) function() x, r: f(1)()}.r", "1"),
    ("{x: \"s\", f: function(x) {g: function() x}.g(), r: f(1)}.r", "1"),
    ("{x: \"s\", f: function(x) function() function() x, r: f(1)()()}.r", "1"),
    ("{g: function() 1, f: function(g) function() g, r: f(2)()}.r", "2"),
    ("{x: {y: \"s\"}, f: function(x) function() x.y, r: f({y: 1})()}.r", "1"),
    ("{k: \"s\", r: for k in [1] return (function() k)()}.r", "[1]"),
    ("{k: \"s\", fs: for k in [1, 2] return function() k, r: for g in fs return g()}.r", "[1, 2]"),
    ("{k: \"s\", r: some k in [1] satisfies (function() k)() = 1}.r", "true"),
    ("{a: 1, r: [{a: 5}, {a: 6}][(function() a)() = 5]}.r", "[{a: 5}]"),
    ("{item: \"s\", r: [1, 2][(function() item)() = 2]}.r", "[2]"),
    ("{x: \"s\", f: function(x) [1][(function() x)() = 1], r: f(1)}.r", "[1]"),
    ("{x: \"s\", f: function(x) {x: 2, g: function() x}.g(), r: f(1)}.r", "2"),
    ("{f: function(x) {x: \"lit\", g: function() function() x}.g()(), r: f(1)}.r", "\"lit\""),
    ("{x: \"s\", f: function() function() x, r: f()()}.r", "\"s\""),
    ("{a: 1, r: [{b: 5}, {b: 6}][(function() a)() = 1]}.r", "[{b: 5}, {b: 6}]"),
    ("{f: function() k, r: f(), k: 10}.r", "10"),
    ("{mk: {f: function() g(), g: function() 7}.f, r: mk()}.r", "7"),
    ("{x: 1, f: function(a) function(b) a + b + x, r: f(10)(100)}.r", "111"),
  ] {
    te_value(false, &scope!(), text, expected);
  }
}

/// Through the predicate of a filter to the parameter around it: the item has the name or not
/// when it is evaluated, and what is further out decides the functions around the filter
/// (review).
#[test]
fn a_function_in_a_filter_reads_the_item_then_the_scope_around_the_filter() {
  te_value(false, &scope!(), r#"{x: "s", h: function(x) function() [1][(function() x)() = 1], r: h(1)()}.r"#, "[1]");
  te_value(
    false,
    &scope!(),
    r#"{x: "s", h: function(x) function() [{x: 5}][(function() x)() = 5], r: h(1)()}.r"#,
    "[{x: 5}]",
  );
  te_value(false, &scope!(), r#"{x: 1, h: function() function() [7][(function() x)() = 1], r: h()()}.r"#, "[7]");
}

/// Deciding where the bodies read their names is the work of the text: many functions that read
/// one name, and one function that reads many (review).
#[test]
fn many_functions_and_names_are_resolved_in_the_time_of_the_text() {
  let started = std::time::Instant::now();
  let functions = vec!["function() a"; 20_000].join(", ");
  te_value(false, &scope!(), &format!("count([{functions}])"), "20000");
  let names: Vec<String> = (0..20_000).map(|i| format!("a{i}")).collect();
  te_value(false, &scope!(), &format!("count((function() [{}])())", names.join(", ")), "20000");
  assert!(started.elapsed() < std::time::Duration::from_secs(20), "{:?}", started.elapsed());
}

/// A function reads a path (`p.customer`) of the nearest scope around it that has the first
/// name, as the path reads it where the function is written: when the value there has not the
/// entry, the path has no value, and a `p` further out that has one is not read. A parameter,
/// a variable or an entry of an item hides the name around it, whatever its value is; so an
/// expression gives the same in a function that is invoked at once as without it.
#[test]
fn a_function_reads_a_path_of_the_nearest_scope_that_has_its_first_name() {
  for (text, expected) in [
    // the nearest `p` has no `customer`: null, as without the innermost function
    ("{book: {customer: \"B\"}, r: (function(p) (function(p) p.customer)(7))(book)}.r", "null"),
    ("{book: {customer: \"B\"}, r: (function(p) (function(p) (function() p.customer)())(7))(book)}.r", "null"),
    (
      "{book: {customer: \"B\"}, r: (function(p) (function(p) (function() (function() p.customer)())())(7))(book)}.r",
      "null",
    ),
    ("{p: {customer: \"B\"}, r: (function(p) (function() p.customer)())(7)}.r", "null"),
    ("{p: {customer: \"B\"}, r: (function(p) (function() p.customer)())(null)}.r", "null"),
    ("{p: {customer: \"B\"}, r: (function(p) (function() p.customer)())({other: 1})}.r", "null"),
    ("{p: {customer: \"B\"}, r: for p in [7] return (function() p.customer)()}.r", "[null]"),
    ("{p: {customer: \"B\"}, r: [{p: 7}][(function() p.customer)() = \"B\"]}.r", "[]"),
    ("{p: {customer: \"B\"}, f: function(p) function() p.customer, g: f(7), r: g()}.r", "null"),
    ("{f: function(p) {p: 7, g: function() p.customer}.g(), r: f({customer: \"B\"})}.r", "null"),
    // deeper paths, and a path beside a path that has a value
    ("{p: {a: {b: 1}}, r: (function(p) (function() p.a.b)())({a: 7})}.r", "null"),
    ("{p: {a: 1, c: 3}, r: (function(p) (function() [p.a, p.c])())({a: 2})}.r", "[2, null]"),
    // the nearest `p` has it
    ("{p: {customer: \"B\"}, r: (function(p) (function() p.customer)())({customer: \"I\"})}.r", "\"I\""),
    ("{p: {customer: \"B\"}, r: (function(q) (function() p.customer)())(7)}.r", "\"B\""),
    ("{p: {customer: \"B\"}, f: function() p.customer, r: f()}.r", "\"B\""),
    ("{f: function() p.customer, r: f(), p: {customer: \"B\"}}.r", "\"B\""),
    (
      "{p: {customer: \"B\"}, r: [{p: {customer: \"I\"}}][(function() p.customer)() = \"I\"]}.r",
      "[{p: {customer: \"I\"}}]",
    ),
    ("{p: {customer: \"B\"}, r: [{q: 1}][(function() p.customer)() = \"B\"]}.r", "[{q: 1}]"),
    ("{r: [{customer: \"X\"}][(function() item.customer)() = \"X\"]}.r", "[{customer: \"X\"}]"),
  ] {
    // compared as values: a null says why it is one
    te_value(false, &scope!(), &format!("({text}) = ({expected})"), "true");
  }
}
