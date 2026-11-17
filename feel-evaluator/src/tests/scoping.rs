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
  let evaluator = crate::prepare(&rheon_feel_parser::parse_expression(&scope, text, false).unwrap());
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
      "eval_greater_then",
      "context has no value for key 'amount'",
      "eval_greater_then",
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
  pin(
    r#"{a: f(), f: function() 1}"#,
    r#"{a: null(expected built-in function name or function definition, actual is null(context has no value for key 'f')), f: FunctionDefinition([],_,false,[],{},Any)}"#,
    &[
      "context has no value for key 'f'",
      "expected built-in function name or function definition, actual is null(context has no value for key 'f')",
    ],
    2,
  );
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
  pin(r#"for a in [{b: 1}] return 2 instance of a.b"#, r#"[true]"#, &[], 0);
  pin(r#"[{t: 1}, {t: "a"}][2 instance of t]"#, r#"[{t: 1}]"#, &[], 0);
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
    &["context has no value for key 'x'", "eval_less_then"],
    2,
  );
  pin(
    r#"{f: function() x, r: for x in 1..3, y in [f()] return y}.r"#,
    r#"[null(context has no value for key 'x'), null(context has no value for key 'x'), null(context has no value for key 'x')]"#,
    &["context has no value for key 'x'"],
    3,
  );
  pin(r#"{a: {b: 1}, f: function() 1 instance of a.b, r: f()}.r"#, r#"true"#, &[], 0);
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
  pin(
    r#"{f: function() k, r: f(), k: 10}.r"#,
    r#"null(context has no value for key 'k')"#,
    &["context has no value for key 'k'"],
    1,
  );
  pin(r#"{f: function() k, k: 10}.f()"#, r#"10"#, &[], 0);
  pin(r#"{x: 1, c: {f: function() x, x: 2, r: f()}.r}.c"#, r#"2"#, &[], 0);
  pin(r#"{x: 1, c: {f: function() x, r: f(), x: 2}.r}.c"#, r#"1"#, &[], 0);
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
