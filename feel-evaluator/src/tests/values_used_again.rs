//! Values used again after they were passed on. A built-in, an operator, a filter, a loop, a call
//! or a closure that derives a new value from a list, a context or a string leaves every other
//! holder of that value unchanged; values nested 100 and 499 levels deep are built, walked and
//! dropped by every channel and consumer; and values that share their items are read whole. These
//! are the expressions of `tools/identity/requests/values.json`, with the answers they have.

use super::*;
use rheon_feel::diagnostics;

/// Evaluates `text` in the scope of the FEEL context `context` (none when empty) and returns the
/// text of the value and the number of errors recorded.
fn answer(text: &str, context: &str) -> (String, usize) {
  let scope = if context.is_empty() { FeelScope::default() } else { te_scope(context) };
  let evaluator = build_evaluator(&rheon_feel_parser::parse_expression(&scope, text).unwrap());
  let (value, diagnostics) = diagnostics::collect(|| evaluator(&scope));
  (value.to_string(), diagnostics.count)
}

/// Asserts every `(expression, context, value text, errors)` and lists all that differ.
fn check(cases: &[(&str, &str, &str, usize)]) {
  let differ: Vec<String> = cases
    .iter()
    .filter_map(|&(text, context, expected, errors)| {
      let actual = answer(text, context);
      (actual != (expected.to_string(), errors)).then(|| format!("(r#\"{text}\"#, r#\"{context}\"#, r#\"{}\"#, {}),", actual.0, actual.1))
    })
    .collect();
  assert!(differ.is_empty(), "{} of {} differ:\n{}", differ.len(), cases.len(), differ.join("\n"));
}

#[test]
fn a_value_derived_from_another_leaves_it_unchanged() {
  check(&[
    (r#"{a: [1, 2], b: append(a, 3), r: [a, b]}.r"#, r#""#, r#"[[1, 2], [1, 2, 3]]"#, 0),
    (r#"{a: {p: 1}, b: context put(a, "p", 2), r: [a.p, b.p]}.r"#, r#""#, r#"[1, 2]"#, 0),
    (
      r#"{a: {p: {q: 1}}, b: context put(a, ["p", "q"], 2), r: [a.p.q, b.p.q, a, b]}.r"#,
      r#""#,
      r#"[1, 2, {p: {q: 1}}, {p: {q: 2}}]"#,
      0,
    ),
    (
      r#"{a: {p: {q: 1}}, b: context put(a, ["p", "r"], 2), r: [a, b]}.r"#,
      r#""#,
      r#"[{p: {q: 1}}, {p: {q: 1, r: 2}}]"#,
      0,
    ),
    (
      r#"{x: {p: {q: 1}}, a: x, b: context put(a, ["p", "q"], 2), r: [x, a, b]}.r"#,
      r#""#,
      r#"[{p: {q: 1}}, {p: {q: 1}}, {p: {q: 2}}]"#,
      0,
    ),
    (r#"{s: "ab", t: s + s, r: [s, t]}.r"#, r#""#, r#"["ab", "abab"]"#, 0),
    (r#"{s: "ab", t: s + "c", u: s + "d", r: [s, t, u]}.r"#, r#""#, r#"["ab", "abc", "abd"]"#, 0),
    (r#"{s: "ab", t: (s + "c") + "d", r: [s, t]}.r"#, r#""#, r#"["ab", "abcd"]"#, 0),
    (r#"for i in 1..4 return partial"#, r#""#, r#"[[], [[]], [[], [[]]], [[], [[]], [[], [[]]]]]"#, 0),
    (
      r#"for i in 1..4 return if i = 1 then [1] else append(partial[-1], i)"#,
      r#""#,
      r#"[[1], [1, 2], [1, 2, 3], [1, 2, 3, 4]]"#,
      0,
    ),
    (
      r#"{l: [3, 1, 2], s: sort(l), r: [l, s]}.r"#,
      r#""#,
      r#"[[3, 1, 2], null(expected 2 parameters, actual number of parameters is 1)]"#,
      1,
    ),
    (r#"{l: [3, 1, 2], s: sort(l, function(x, y) x > y), r: [l, s]}.r"#, r#""#, r#"[[3, 1, 2], [3, 2, 1]]"#, 0),
    (r#"{l: [1, 2, 3], s: reverse(l), r: [l, s]}.r"#, r#""#, r#"[[1, 2, 3], [3, 2, 1]]"#, 0),
    (r#"{l: [1, 2, 3], s: list replace(l, 2, 9), r: [l, s]}.r"#, r#""#, r#"[[1, 2, 3], [1, 9, 3]]"#, 0),
    (
      r#"{l: [1, 2, 3], s: list replace(l, function(item, newItem) item > 1, 9), r: [l, s]}.r"#,
      r#""#,
      r#"[[1, 2, 3], [1, 9, 9]]"#,
      0,
    ),
    (r#"{l: [1, 2, 3], s: insert before(l, 1, 0), r: [l, s]}.r"#, r#""#, r#"[[1, 2, 3], [0, 1, 2, 3]]"#, 0),
    (r#"{l: [1, 2, 3], s: remove(l, 2), r: [l, s]}.r"#, r#""#, r#"[[1, 2, 3], [1, 3]]"#, 0),
    (r#"{l: [1, 1, 2], s: distinct values(l), r: [l, s]}.r"#, r#""#, r#"[[1, 1, 2], [1, 2]]"#, 0),
    (r#"{l: [1, 2], s: union(l, [2, 3]), r: [l, s]}.r"#, r#""#, r#"[[1, 2], [1, 2, 3]]"#, 0),
    (r#"{l: [1, 2], s: concatenate(l, l), r: [l, s]}.r"#, r#""#, r#"[[1, 2], [1, 2, 1, 2]]"#, 0),
    (r#"{l: [[1], [2, [3]]], s: flatten(l), r: [l, s]}.r"#, r#""#, r#"[[[1], [2, [3]]], [1, 2, 3]]"#, 0),
    (r#"{l: [1, 2, 3], s: sublist(l, 2), r: [l, s]}.r"#, r#""#, r#"[[1, 2, 3], [2, 3]]"#, 0),
    (r#"{l: [1, 2, 3], s: sublist(l, 1, 2), r: [l, s]}.r"#, r#""#, r#"[[1, 2, 3], [1, 2]]"#, 0),
    (r#"{a: {p: 1}, s: context merge([a, {q: 2}]), r: [a, s]}.r"#, r#""#, r#"[{p: 1}, {p: 1, q: 2}]"#, 0),
    (r#"{a: {p: 1}, s: context merge([a, {p: 2}]), r: [a, s]}.r"#, r#""#, r#"[{p: 1}, {p: 2}]"#, 0),
    (
      r#"{a: {p: 1, q: 2}, s: get entries(a), r: [a, s]}.r"#,
      r#""#,
      r#"[{p: 1, q: 2}, [{key: "p", value: 1}, {key: "q", value: 2}]]"#,
      0,
    ),
    (
      r#"{l: [3, 1, 2], r: [l[item > 1], sort(l), l]}.r"#,
      r#""#,
      r#"[[3, 2], null(expected 2 parameters, actual number of parameters is 1), [3, 1, 2]]"#,
      1,
    ),
    (
      r#"{l: [{item: 1, v: 2}, {v: 3}], r: [l[item = 1], l[v > 2], l]}.r"#,
      r#""#,
      r#"[[{item: 1, v: 2}], [{v: 3}], [{item: 1, v: 2}, {v: 3}]]"#,
      1,
    ),
    (
      r#"{l: [{item: 5}, {item: 6}], r: [l[item > 5], l]}.r"#,
      r#""#,
      r#"[[{item: 6}], [{item: 5}, {item: 6}]]"#,
      0,
    ),
    (r#"{l: [{v: 1}, {v: 2}], r: [l[v > 1], l.v, l]}.r"#, r#""#, r#"[[{v: 2}], [1, 2], [{v: 1}, {v: 2}]]"#, 0),
    (r#"{f: function(x) context put(x, "y", 1), a: {y: 0}, r: [f(a), a]}.r"#, r#""#, r#"[{y: 1}, {y: 0}]"#, 0),
    (r#"{f: function(x) append(x, 1), a: [0], r: [f(a), f(a), a]}.r"#, r#""#, r#"[[0, 1], [0, 1], [0]]"#, 0),
    (r#"{x: 1, f: function() x, g: f, r: [f(), g()]}.r"#, r#""#, r#"[1, 1]"#, 0),
    (
      r#"{x: {p: 1}, f: function() context put(x, "p", 2), r: [f(), x, f()]}.r"#,
      r#""#,
      r#"[{p: 2}, {p: 1}, {p: 2}]"#,
      0,
    ),
    (
      r#"{l: for i in 1..3 return i, r: [l, for j in l return j * 2, some x in l satisfies x > 2, l]}.r"#,
      r#""#,
      r#"[[1, 2, 3], [2, 4, 6], true, [1, 2, 3]]"#,
      0,
    ),
    (
      r#"{l: [{a: 1}, {a: 2}], r: [for c in l return context put(c, "a", 0), l]}.r"#,
      r#""#,
      r#"[[{a: 0}, {a: 0}], [{a: 1}, {a: 2}]]"#,
      0,
    ),
    (
      r#"{l: [{a: 1}, {a: 2}], r: [every c in l satisfies c.a > 0, l]}.r"#,
      r#""#,
      r#"[true, [{a: 1}, {a: 2}]]"#,
      0,
    ),
    (r#"{x: {p: {q: 1}}, a: x, b: x, r: [a.p.q, b.p.q]}.r"#, r#""#, r#"[1, 1]"#, 0),
    (r#"{x: {p: {q: 1}}, a: x, b: x, r: [a, b, x]}.r"#, r#""#, r#"[{p: {q: 1}}, {p: {q: 1}}, {p: {q: 1}}]"#, 0),
    (r#"{b: context put(a, "p", 2), r: [a.p, b.p, a]}.r"#, r#"{a: {p: 1}}"#, r#"[1, 2, {p: 1}]"#, 0),
    (
      r#"{s: sort(l), r: [l, s, l]}.r"#,
      r#"{l: [3, 1, 2]}"#,
      r#"[[3, 1, 2], null(expected 2 parameters, actual number of parameters is 1), [3, 1, 2]]"#,
      1,
    ),
    (
      r#"[a, b, context put(a, ["p", "q"], 2), a.p.q, b.p.q]"#,
      r#"{a: {p: {q: 1}}, b: {p: {q: 1}}}"#,
      r#"[{p: {q: 1}}, {p: {q: 1}}, {p: {q: 2}}, 1, 1]"#,
      0,
    ),
    (r#"a.p.q + b.p.q"#, r#"{x: {p: {q: 1}}, a: x, b: x}"#, r#"2"#, 0),
    (r#"{r: [a, context put(a, "p", 0), a]}.r"#, r#"{x: {p: 1}, a: x}"#, r#"[{p: 1}, {p: 0}, {p: 1}]"#, 0),
    (r#"{a: [1, 2], b: a, c: append(b, 3), r: [a, b, c]}.r"#, r#""#, r#"[[1, 2], [1, 2], [1, 2, 3]]"#, 0),
    (
      r#"{a: {p: [1]}, b: context put(a, "p", append(a.p, 2)), r: [a, b]}.r"#,
      r#""#,
      r#"[{p: [1]}, {p: [1, 2]}]"#,
      0,
    ),
    (
      r#"{l: [3, 1, 2], f: function(x) sort(x, function(p, q) p < q), r: [f(l), l, f(l)]}.r"#,
      r#""#,
      r#"[[1, 2, 3], [3, 1, 2], [1, 2, 3]]"#,
      0,
    ),
    (
      r#"{l: [1, 2, 3], r: [for x in l return remove(l, x), l]}.r"#,
      r#""#,
      r#"[[[2, 3], [1, 3], [1, 2]], [1, 2, 3]]"#,
      0,
    ),
    (
      r#"{l: [[1], [2]], r: [for x in l return append(x, 0), l]}.r"#,
      r#""#,
      r#"[[[1, 0], [2, 0]], [[1], [2]]]"#,
      0,
    ),
    (
      r#"{a: {p: 1}, l: [a, a], r: [for c in l return context put(c, "p", 2), l, a]}.r"#,
      r#""#,
      r#"[[{p: 2}, {p: 2}], [{p: 1}, {p: 1}], {p: 1}]"#,
      0,
    ),
    (r#"{l: [1, 2, 3], r: [l[item > 1], l[1], l[-1], l]}.r"#, r#""#, r#"[[2, 3], 1, 3, [1, 2, 3]]"#, 0),
    (
      r#"{s: "ab", l: [s, s], r: [for x in l return x + "c", l, s]}.r"#,
      r#""#,
      r#"[["abc", "abc"], ["ab", "ab"], "ab"]"#,
      0,
    ),
    (
      r#"[a, b, append(a.p, 0), a.p]"#,
      r#"{x: {p: [1, 2]}, a: x, b: x}"#,
      r#"[{p: [1, 2]}, {p: [1, 2]}, [1, 2, 0], [1, 2]]"#,
      0,
    ),
    (
      r#"{f: function(c) context merge([c, {q: 2}]), r: [f(a), a, f(a)]}.r"#,
      r#"{a: {p: 1}}"#,
      r#"[{p: 1, q: 2}, {p: 1}, {p: 1, q: 2}]"#,
      0,
    ),
    (r#"count((for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]]))"#, r#""#, r#"10"#, 0),
    (
      r#"(for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1] = (for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]"#,
      r#""#,
      r#"true"#,
      0,
    ),
    (
      r#"{x: (for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1], r: x = x}.r"#,
      r#""#,
      r#"true"#,
      0,
    ),
    (
      r#"(for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1] instance of list<Any>"#,
      r#""#,
      r#"true"#,
      0,
    ),
    (
      r#"count(distinct values((for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]))"#,
      r#""#,
      r#"1"#,
      0,
    ),
    (
      r#"{l: (for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]]), r: index of(l[-1], l[-2])}.r"#,
      r#""#,
      r#"[1, 2]"#,
      0,
    ),
    (
      r#"count(union((for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1], (for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]))"#,
      r#""#,
      r#"1"#,
      0,
    ),
    (
      r#"string length(string((for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]))"#,
      r#""#,
      r#"3580"#,
      0,
    ),
    (
      r#"count(flatten((for i in 1..10 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]))"#,
      r#""#,
      r#"512"#,
      0,
    ),
    (
      r#"{x: (for i in 1..10 return if i = 1 then {a: 1} else {a: partial[-1], b: partial[-1]})[-1], r: x = x}.r"#,
      r#""#,
      r#"true"#,
      0,
    ),
    (
      r#"count((for i in 1..10 return if i = 1 then {a: 1} else {a: partial[-1], b: partial[-1]})[-1]...a)"#,
      r#""#,
      r#"1023"#,
      0,
    ),
  ]);
}

/// Runs `f` on a thread with a stack big enough for values nested 499 levels in unoptimized builds.
fn with_big_stack(f: impl FnOnce() + Send + 'static) {
  std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

/// `inner` inside `levels` pairs of `open` and `close`.
fn nested(open: &str, inner: &str, close: &str, levels: usize) -> String {
  format!("{}{inner}{}", open.repeat(levels), close.repeat(levels))
}

/// A context literal of entries `a0` to `a<n>`, each entry after the first the value of `entry` on
/// the name before it, and the path to its last entry.
fn chain(n: usize, entry: impl Fn(&str) -> String) -> String {
  let entries: Vec<String> = (1..=n).map(|i| format!("a{i}: {}", entry(&format!("a{}", i - 1)))).collect();
  format!("{{a0: 0, {}}}.a{n}", entries.join(", "))
}

/// A recursion that nests ten list literals around each call, with the rest of `depth` nested
/// around the outer call: a list nested `depth` levels deep around `0`.
fn recursion_times_literal(depth: usize) -> String {
  let calls = depth / 10;
  let rest = depth - calls * 10;
  format!(
    "{{f: function(n) if n = 0 then 0 else {}, r: {}}}.r",
    nested("[", "f(n-1)", "]", 10),
    nested("[", &format!("f({calls})"), "]", rest)
  )
}

/// The `for` whose last result is a list (`[]` in the first iteration, else a list of the result
/// before) or a context (`{}`, else `{a: <the result before>}`) nested `n` levels deep.
fn through_partial(n: usize, first: &str, wrap: &str) -> String {
  format!("(for i in 1..{n} return if i = 1 then {first} else {wrap})[-1]")
}

/// Functions that each return the one before: the last of them nested `n` levels deep (rounded
/// down to an even number), because a function is one level deeper than its closure context and
/// that context one level deeper than the function it holds.
fn closures(n: usize) -> String {
  let links = n / 2 - 1;
  let functions: String = (1..=links).map(|i| format!(", f{i}: function() f{}", i - 1)).collect();
  format!("{{f0: function() 0{functions}}}.f{links}")
}

/// The text of the function `f<i>` of [closures].
fn closures_text(i: usize) -> String {
  (1..=i).fold("FunctionDefinition([],_,false,[],{},Any)".to_string(), |inner, i| {
    format!("FunctionDefinition([],_,false,[f{0}],{{f{0}: {inner}}},Any)", i - 1)
  })
}

#[test]
fn values_nested_100_and_499_levels_are_built_by_every_channel() {
  with_big_stack(|| {
    for n in [100, 499] {
      let mut cases = vec![
        (through_partial(n, "[]", "[partial[-1]]"), nested("[", "", "]", n)),
        (through_partial(n, "{}", "{a: partial[-1]}"), nested("{a: ", "{}", "}", n - 1)),
        (recursion_times_literal(n), nested("[", "0", "]", n)),
        (chain(n, |before| format!("{{x: {before}}}")), nested("{x: ", "0", "}", n)),
        (format!("context put({{}}, for i in 1..{n} return \"a\", 0)"), nested("{a: ", "0", "}", n)),
        (chain(n, |before| format!("[{before}..0]")), nested("[", "0", "..0]", n)),
        (closures(n), closures_text(n / 2 - 1)),
      ];
      if n == 499 {
        // copying `partial` in every iteration makes these cubic in the depth
        cases.drain(..2);
      }
      for (text, expected) in cases {
        assert_eq!((expected, 0), answer(&text, ""), "{text}");
      }
    }
  });
}

#[test]
fn values_nested_100_and_499_levels_are_walked_by_every_consumer() {
  with_big_stack(|| {
    let list_100 = through_partial(100, "[]", "[partial[-1]]");
    let context_100 = through_partial(100, "{}", "{a: partial[-1]}");
    let list_499 = format!("({})", recursion_times_literal(499));
    let context_499 = format!("({})", chain(499, |before| format!("{{x: {before}}}")));
    // count, equality with itself through a name and as two evaluations, instance of, the text length
    for (deep, text_length) in [(&list_100, "200"), (&context_100, "497"), (&list_499, "999"), (&context_499, "2496")] {
      for (text, expected) in [
        (format!("count({deep})"), "1"),
        (format!("{{x: {deep}, r: x = x}}.r"), "true"),
        (format!("{deep} = {deep}"), "true"),
        (format!("{deep} instance of Any"), "true"),
        (format!("string length(string({deep}))"), text_length),
      ] {
        assert_eq!((expected.to_string(), 0), answer(&text, ""), "{text}");
      }
    }
    for (text, expected) in [
      (format!("count(flatten({}))", through_partial(100, "[1]", "[partial[-1]]")), "1"),
      (format!("count({}...a)", through_partial(100, "{a: 1}", "{a: partial[-1]}")), "100"),
      (format!("count(flatten({list_499}))"), "1"),
      (format!("count({context_499}...x)"), "499"),
    ] {
      assert_eq!((expected.to_string(), 0), answer(&text, ""), "{text}");
    }
  });
}

#[test]
fn values_that_share_their_items_are_read_whole() {
  // each result of the `for` holds the result before it twice
  let list = (2..=10).fold("[1]".to_string(), |before, _| format!("[{before}, {before}]"));
  let context = (2..=10).fold("{a: 1}".to_string(), |before, _| format!("{{a: {before}, b: {before}}}"));
  assert_eq!((list, 0), answer(&through_partial(10, "[1]", "[partial[-1], partial[-1]]"), ""));
  assert_eq!((context, 0), answer(&through_partial(10, "{a: 1}", "{a: partial[-1], b: partial[-1]}"), ""));
}
