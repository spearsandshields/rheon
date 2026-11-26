//! Today's scoping, pinned with the answers and the diagnostics of each evaluation: a function body
//! reads the names it does not bind through the scopes of its caller, a function captures the
//! names its body uses as they are where it is defined, a returned function closes over the names
//! as they are where the body ends, and the hidden `partial` of a `for` is out of sight in the
//! later iteration contexts. The same expressions are requests of `tools/identity/requests/`.

use rheon_feel::{FeelScope, diagnostics};

/// Asserts that `text`, evaluated as a request evaluates it (one evaluation in a session), gives
/// the value whose text is `expected` (or `abort: <message>`), with the diagnostics `messages`
/// listed and `count` errors counted.
fn pin(text: &str, expected: &str, messages: &[&str], count: usize) {
  let scope = FeelScope::default();
  let evaluator = crate::prepare(&rheon_feel_parser::parse_expression(&scope, text).unwrap());
  let (outcome, collected) = diagnostics::collect(|| crate::Session::new().run(|| evaluator(&scope)).into_result());
  let shown = match outcome {
    Ok(value) => value.to_string(),
    Err(abort) => format!("abort: {abort}"),
  };
  if std::env::var_os("RHEON_EXPECTED").is_some() {
    if expected != shown || messages != collected.messages || count != collected.count {
      println!("PIN\t{text}\t{shown}\t{:?}\t{}", collected.messages, collected.count);
    }
    return;
  }
  assert_eq!(expected, shown, "{text}");
  assert_eq!(messages, collected.messages, "{text}");
  assert_eq!(count, collected.count, "{text}");
}

#[test]
fn todays_scoping_is_pinned() {
  pin(
    r#"{f: function() a, r: {a: 1, s: f()}.s}.r"#,
    r#"null(context has no value for key 'a')"#,
    &["context has no value for key 'a'"],
    1,
  );
  pin(
    r#"{f: function() partial, r: for i in 1..3 return f()}.r"#,
    r#"[null(context has no value for key 'partial'), null(context has no value for key 'partial'), null(context has no value for key 'partial')]"#,
    &["context has no value for key 'partial'"],
    3,
  );
  pin(
    r#"{f: function() x, r: for x in 1..3 return f()}.r"#,
    r#"[null(context has no value for key 'x'), null(context has no value for key 'x'), null(context has no value for key 'x')]"#,
    &["context has no value for key 'x'"],
    3,
  );
  pin(r#"{f: function(n) if n = 0 then 0 else g(n - 1), g: function(n) f(n), r: f(3)}.r"#, r#"0"#, &[], 0);
  pin(r#"{f: function(n) if n = 0 then 0 else f(n - 1), r: for f in [1] return f}.r"#, r#"[1]"#, &[], 0);
  pin(
    r#"for x in [1, 2], y in [3, 4], x in [x] return x"#,
    r#"null(duplicate iteration variable 'x')"#,
    &["duplicate iteration variable 'x'"],
    1,
  );
  pin(r#"for partial in [1, 2] return partial"#, r#"[[], [[]]]"#, &[], 0);
  pin(
    r#"for partial in [1, 2], y in [partial] return y"#,
    r#"[null(context has no value for key 'partial'), null(context has no value for key 'partial')]"#,
    &["context has no value for key 'partial'"],
    2,
  );
  pin(
    r#"{g: function() partial, r: for partial in [1, 2], y in [g()] return y}.r"#,
    r#"[null(context has no value for key 'partial'), null(context has no value for key 'partial')]"#,
    &["context has no value for key 'partial'"],
    2,
  );
  pin(
    r#"{g: function() partial, r: for partial in [1, 2] return g()}.r"#,
    r#"[null(context has no value for key 'partial'), null(context has no value for key 'partial')]"#,
    &["context has no value for key 'partial'"],
    2,
  );
  pin(r#"{f: function(a, a) a, r: f(1, 2)}.r"#, r#"2"#, &[], 0);
  pin(r#"{f: function(a, a) a, r: f(a: 1)}.r"#, r#"null"#, &[], 0);
  pin(
    r#"{g: function() amount, r: [{amount: 1}, {amount: 5}][g() > 2]}.r"#,
    r#"[]"#,
    &[
      "context has no value for key 'amount'",
      "comparison '>' expected a number, string, date, time, date and time, or duration on the left, found Null",
      "context has no value for key 'amount'",
      "comparison '>' expected a number, string, date, time, date and time, or duration on the left, found Null",
    ],
    4,
  );
  pin(r#"{f: function(a, b) x < 2, x: 1, r: sort([2, 1], f)}.r"#, r#"[1, 2]"#, &[], 0);
  pin(r#"{x: 1, r: sort([2, 1], function(a, b) x < 2)}.r"#, r#"[1, 2]"#, &[], 0);
  pin(r#"{a: {b: 1}, f: function() a.b}.f"#, r#"FunctionDefinition([],_,false,[a.b],{a: {b: 1}},Any)"#, &[], 0);
  pin(r#"{g: function() {b: 2}, f: function() g().b}.f"#, r#"FunctionDefinition([],_,false,[b],{},Any)"#, &[], 0);
  pin(
    r#"{mk: function(n) function(m) n + m, r: mk(1)}.r"#,
    r#"FunctionDefinition([(Name("m"), Any)],_,false,[n],{n: 1},Any)"#,
    &[],
    0,
  );
  pin(r#"{mk: function(n) function(m) n + m, r: mk(1)(2)}.r"#, r#"3"#, &[], 0);
  pin(r#"{f: function(x) {x: 10, g: function(y) x + y}.g, h: f(1), r: h(5)}.r"#, r#"15"#, &[], 0);
  // an entry is evaluated after the entries it reads (DMN 1.6 clause 10.3.2.11.1)
  pin(r#"{a: f(), f: function() 1}"#, r#"{a: 1, f: FunctionDefinition([],_,false,[],{},Any)}"#, &[], 0);
  pin(
    r#"{a: 1, a: 1 + "x"}"#,
    r#"null(duplicated context entry key: a)"#,
    &["incompatible types in addition: 1(number) + \"x\"(string)", "duplicated context entry key: a"],
    2,
  );
  pin(
    r#"function(x: t) x"#,
    r#"null(expected type of the formal parameter)"#,
    &["no value for qualified name", "expected type of the formal parameter"],
    2,
  );
  pin(
    r#"{g: function() p, f: function(p) g(), r: f(7)}.r"#,
    r#"null(context has no value for key 'p')"#,
    &["context has no value for key 'p'"],
    1,
  );
  pin(
    r#"for a in [{b: 1}] return 2 instance of a.b"#,
    r#"[null(instance of expected a type on the right, found number)]"#,
    &["instance of expected a type on the right, found number"],
    1,
  );
  // a value is no type (Table 62): null, so the filter keeps no item
  pin(
    r#"[{t: 1}, {t: "a"}][2 instance of t]"#,
    r#"[]"#,
    &[
      "instance of expected a type on the right, found number",
      "instance of expected a type on the right, found string",
    ],
    2,
  );
  pin(r#"{f: function(n) if n = 0 then 0 else abs(abs(f(n - 1))), r: f(66)}.r"#, r#"0"#, &[], 0);
  pin(
    r#"{f: function(n) if n = 0 then 0 else abs(abs(f(n - 1))), r: f(67)}.r"#,
    r#"abort: function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped"#,
    &[
      "[core::abs] invalid argument type, expected number, actual type is Null",
      "function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped",
    ],
    134,
  );
  pin(r#"{f: function(n) if n = 0 then 0 else f(n - 1), r: f(199)}.r"#, r#"0"#, &[], 0);
  pin(
    r#"{f: function(n) if n = 0 then 0 else f(n - 1), r: f(200)}.r"#,
    r#"abort: function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped"#,
    &["function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped"],
    1,
  );
  pin(
    r#"{f: function(a, b) x < 2, r: for x in [1] return sort([2, 1], f)}.r"#,
    r#"[[2, 1]]"#,
    &[
      "context has no value for key 'x'",
      "comparison '<' expected a number, string, date, time, date and time, or duration on the left, found Null",
    ],
    2,
  );
  pin(
    r#"{f: function() x, r: for x in 1..3, y in [f()] return y}.r"#,
    r#"[null(context has no value for key 'x'), null(context has no value for key 'x'), null(context has no value for key 'x')]"#,
    &["context has no value for key 'x'"],
    3,
  );
  pin(
    r#"{a: {b: 1}, f: function() 1 instance of a.b, r: f()}.r"#,
    r#"null(instance of expected a type on the right, found number)"#,
    &["instance of expected a type on the right, found number"],
    1,
  );
}

/// A function sees the names in scope where it is written (DMN 1.6 §10.3.2.13.2), and of the
/// context literal it is written in every entry (§10.3.2.11.1), also one that comes after it,
/// from the moment the entry has its value: functions of one context call each other in any
/// order. It sees nothing of whoever invokes it.
#[test]
fn a_function_sees_the_scope_it_is_written_in() {
  pin(r#"{a: function(x) b(x) + 1, b: function(x) x * 2, k: 10, r: a(1)}.r"#, r#"3"#, &[], 0);
  pin(r#"{a: function(x) b(x) + 1, b: function(x) x * 2}.a(1)"#, r#"3"#, &[], 0);
  pin(r#"{f: function() k, k: 10, r: f()}.r"#, r#"10"#, &[], 0);
  // what the body of a function entry reads is evaluated before an entry that reads the function
  pin(r#"{f: function() k, r: f(), k: 10}.r"#, r#"10"#, &[], 0);
  pin(r#"{f: function() k, k: 10}.f()"#, r#"10"#, &[], 0);
  pin(r#"{x: 1, c: {f: function() x, x: 2, r: f()}.r}.c"#, r#"2"#, &[], 0);
  pin(r#"{x: 1, c: {f: function() x, r: f(), x: 2}.r}.c"#, r#"2"#, &[], 0);
  pin(r#"{f: function(n) (function(m) g(m))(n), g: function(x) x + 1, r: f(1)}.r"#, r#"2"#, &[], 0);
  pin(r#"{l: [function() k], k: 5, r: l[1]()}.r"#, r#"5"#, &[], 0);
  pin(r#"{mk: {f: function() g(), g: function() 7}.f, r: mk()}.r"#, r#"7"#, &[], 0);
  pin(r#"{c: {f: function(n) if n = 0 then 0 else c.f(n - 1)}, r: c.f(2)}.r"#, r#"0"#, &[], 0);
  pin(r#"{c: {f: function(n) if n = 0 then 0 else f(n - 1)}, r: c.f(2)}.r"#, r#"0"#, &[], 0);
  pin(
    r#"{f: function(n) if n = 0 then "done" else g(n - 1), g: function(n) f(n), h: f, r: h(3)}.r"#,
    r#""done""#,
    &[],
    0,
  );
  pin(r#"{f: function() a, a: 1, g: function() {a: 2, r: f()}.r, r: g()}.r"#, r#"1"#, &[], 0);
  pin(r#"for i in 1..2 return {f: function() k, k: i}.f()"#, r#"[1, 2]"#, &[], 0);
  pin(r#"{f: function(x) function(y) x + y + k, k: 100, r: f(1)(2)}.r"#, r#"103"#, &[], 0);
  pin(r#"{k: 1, f: function() k, r: {k: 2, s: f()}.s}.r"#, r#"1"#, &[], 0);
  pin(r#"sort([3, 1, 2], function(a, b) a < b)"#, r#"[1, 2, 3]"#, &[], 0);
  pin(
    r#"{lt: function(a, b) key(a) < key(b), key: function(v) v.n, r: sort([{n: 2}, {n: 1}], lt)}.r"#,
    r#"[{n: 1}, {n: 2}]"#,
    &[],
    0,
  );
}

/// The entries of a context literal are evaluated in the order they need each other (DMN 1.6
/// clause 10.3.2.11.1), an entry does not read its own key (clause 10.3.2.6), and entries that
/// need each other in a circle have no value.
#[test]
fn context_entries_are_evaluated_in_the_order_they_need_each_other() {
  // an entry reads an entry written after it
  pin(r#"{a: b + 1, b: 1}.a"#, r#"2"#, &[], 0);
  pin(r#"{a: b + 1, b: c * 2, c: 3}"#, r#"{a: 7, b: 6, c: 3}"#, &[], 0);
  pin(r#"{total: unit price * 2, unit price: 3}.total"#, r#"6"#, &[], 0);
  pin(r#"{a: x.y, x: {y: 1}}.a"#, r#"1"#, &[], 0);
  pin(r#"{a: [b, c], b: 1, c: b + 1}.a"#, r#"[1, 2]"#, &[], 0);
  pin(r#"{a: for i in [1, 2] return i + b, b: 10}.a"#, r#"[11, 12]"#, &[], 0);
  pin(r#"{a: [1, 2, 3][item > b], b: 1}.a"#, r#"[2, 3]"#, &[], 0);
  pin(r#"{a: {c: b}, b: 1}.a.c"#, r#"1"#, &[], 0);
  // a key of several tokens is a name in the entries before it too
  pin(r#"{x: a-b + 1, a-b: 2}.x"#, r#"3"#, &[], 0);
  pin(r#"{x: net price * 2, net price: unit price - 1, unit price: 4}.x"#, r#"6"#, &[], 0);
  pin(r#"{x: Profit and Loss + 1, Profit and Loss: 2}.x"#, r#"3"#, &[], 0);
  pin(r#"{x: f(a: 1, b: 2), f: function(a, b) a-b}.x"#, r#"-1"#, &[], 0);
  // but not in its own value: there the text is read as outside the literal
  pin(r#"{a: 5, b: 2, r: {a-b: a-b}}.r"#, r#"{a-b: 3}"#, &[], 0);
  pin(r#"{a: 1, b: 2, r: {c: a+b, a+b: a+b}}.r"#, r#"{c: 3, a+b: 3}"#, &[], 0);
  // the entry of the literal, not a name outside it
  pin(r#"{b: 100, r: {a: b + 1, b: 1}.a}.r"#, r#"2"#, &[], 0);
  pin(r#"{a: 1, b: {c: a, a: 2}}.b.c"#, r#"2"#, &[], 0);
  // a name that a construct inside the value binds is not the entry
  pin(r#"{a: for b in [1, 2] return b, b: 10}.a"#, r#"[1, 2]"#, &[], 0);
  pin(r#"{a: (function(b) b)(1), b: 10}.a"#, r#"1"#, &[], 0);
  pin(r#"{a: {b: 1, c: b}.c, b: 10}.a"#, r#"1"#, &[], 0);
  pin(r#"{a: some b in [1] satisfies b = 1, b: 10}.a"#, r#"true"#, &[], 0);
  pin(r#"{a: x.b, x: {b: 1}, b: 10}.a"#, r#"1"#, &[], 0);
  // an entry does not read its own key: the name is read outside the literal
  pin(r#"{x: 1, r: {x: x + 1}}.r"#, r#"{x: 2}"#, &[], 0);
  pin(r#"{x: 1, y: 2, r: {x: x, y: y}}.r"#, r#"{x: 1, y: 2}"#, &[], 0);
  pin(r#"{x: x}"#, r#"{x: null(context has no value for key 'x')}"#, &["context has no value for key 'x'"], 1);
  // what the body of a function entry reads is evaluated before an entry that invokes it
  pin(r#"{f: function() a, a: 1, i: f()}.i"#, r#"1"#, &[], 0);
  pin(r#"{i: f(), f: function() a, a: 1}.i"#, r#"1"#, &[], 0);
  pin(r#"{i: g(), g: function() f() + 1, f: function() a, a: 1}.i"#, r#"2"#, &[], 0);
  // function entries invoke each other and themselves
  pin(
    r#"{even: function(n) if n = 0 then true else odd(n - 1), odd: function(n) if n = 0 then false else even(n - 1), r: even(4)}.r"#,
    r#"true"#,
    &[],
    0,
  );
  pin(r#"{r: fact(5), fact: function(n) if n < 2 then 1 else n * fact(n - 1)}.r"#, r#"120"#, &[], 0);
  // entries that need each other in a circle have no value; the other entries have theirs
  let circular = |name: &str| format!("the context entry {name} needs its own value: the entries of a context must not depend on each other in a circle");
  pin(
    r#"{a: b, b: a, c: 1}"#,
    &format!("{{a: null({}), b: null({}), c: 1}}", circular("a"), circular("b")),
    &[&circular("b"), &circular("a")],
    2,
  );
  pin(r#"{a: b + 1, b: c + 1, c: a + 1, d: 4}.d"#, r#"4"#, &[&circular("c"), &circular("b"), &circular("a")], 3);
  // through a function that is invoked
  pin(r#"{a: f(), f: function() a}.f"#, r#"FunctionDefinition([],_,false,[a],{},Any)"#, &[&circular("a")], 1);
  // an entry that reads a circular entry reads its null
  pin(r#"{a: b, b: a, c: a = null}.c"#, r#"true"#, &[&circular("b"), &circular("a")], 2);
}
