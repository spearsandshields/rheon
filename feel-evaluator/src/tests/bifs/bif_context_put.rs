use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_context(false, &scope!(), r#"context put({}, "a", 1)"#, "{a: 1}");
}

#[test]
fn _0002() {
  te_be_value(
    false,
    &scope!(),
    r#"get entries(context put({"a": 1}, "b", 2))"#,
    r#"[{key: "a", value: 1}, {key: "b", value: 2}]"#,
  );
}

#[test]
fn _0003() {
  te_context(false, &scope!(), r#"context put({"a": 1}, "a", 2)"#, "{a: 2}");
}

#[test]
fn _0004() {
  te_context(false, &scope!(), r#"context put({"a":1, "b":2, "c":3 }, "b", 3)"#, "{a: 1, b: 3, c: 3}");
}

#[test]
fn _0005() {
  te_context(false, &scope!(), r#"context put({}, "", 1)"#, r#"{"": 1}"#);
}

#[test]
fn _0006() {
  te_null(false, &scope!(), "context put({}, null, 1)", "");
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"context put(null, "a", 1)"#, "");
}

#[test]
fn _0008() {
  te_context(false, &scope!(), r#"context put({}, "a", null)"#, "{a: null}");
}

#[test]
fn _0009() {
  te_null(false, &scope!(), r#"context put({}, "a")"#, "expected 3 parameters, actual number of parameters is 2");
}

#[test]
fn _0010() {
  te_null(false, &scope!(), r#"context put({}, "a", 1, 1)"#, "expected 3 parameters, actual number of parameters is 4");
}

#[test]
fn _0011() {
  te_context(false, &scope!(), r#"context put(context: {}, key: "a", value: 1)"#, "{a: 1}");
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    r#"context put(context: {}, ky: "a", value: 1)"#,
    "expected the parameters of one of context put(context, key, value): context, context put(context, keys, value): context, found the parameter name 'ky'",
  );
}

#[test]
fn _0013() {
  te_null(
    false,
    &scope!(),
    r#"context put(context: {}, key: "a", v: 1)"#,
    "expected the parameters of one of context put(context, key, value): context, context put(context, keys, value): context, found the parameter name 'v'",
  );
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    r#"context put(c: {}, key: "a", value: 1)"#,
    "expected the parameters of one of context put(context, key, value): context, context put(context, keys, value): context, found the parameter name 'c'",
  );
}

#[test]
fn _0015() {
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4)
  te_be_value(false, &scope!(), r#"context put(context: {}, keys: "a", value: 1)"#, r#"{a: 1}"#);
}

#[test]
fn _0016() {
  te_null(
    false,
    &scope!(),
    r#"context put([], "a", 1)"#,
    "[core::context put] invalid argument type, expected context, actual type is list<Null>",
  );
}

#[test]
fn _0017() {
  te_null(
    false,
    &scope!(),
    r#"context put({}, 1, 1)"#,
    "[core::context put] invalid argument type, expected string or list<string>, actual type is number",
  );
}

#[test]
fn _0018() {
  te_context(false, &scope!(), r#"context put({x:1, y: {a: 0} }, ["y", "a"], 2)"#, "{x: 1, y: {a: 2}}");
}

#[test]
fn _0019() {
  te_null(
    false,
    &scope!(),
    r#"context put(context: {}, keys: [1], value: 1)"#,
    "[core::context put] invalid argument type, expected string, actual type is number",
  );
}

#[test]
fn _0020() {
  // context put(context, keys, value) puts under the first key the result of context put of the
  // entry under it (10.3.4.10): a key the context does not have gives that entry null, and the key
  // list builds no context
  te_string(false, &scope!(), r#"string(context put({x: 1}, ["y", "a"], 2))"#, r#"{x: 1, y: null}"#);
  te_string(false, &scope!(), r#"string(context put({}, ["a", "b", "c"], 1))"#, r#"{a: null}"#);
  te_string(false, &scope!(), r#"string(context put({x: {}}, ["x", "a", "b"], 1))"#, r#"{x: {a: null}}"#);
  te_value(false, &scope!(), r#"context put({x: {y: {}}}, ["x", "y", "z"], 1)"#, r#"{x: {y: {z: 1}}}"#);
  // an entry that holds null is to context.key' what a missing entry is (review of
  // v0.9.0..feel/f1)
  te_string(false, &scope!(), r#"string(context put({a: null}, ["a", "b"], 1))"#, r#"{a: null}"#);
  te_string(false, &scope!(), r#"string(context put({x: {a: null}}, ["x", "a", "b"], 1))"#, r#"{x: {a: null}}"#);
  te_value(false, &scope!(), r#"context put({a: null}, ["a", "b"], 1) = context put({}, ["a", "b"], 1)"#, "true");
  // an entry on the path that is no context: the whole result is null (TCK 1146 nested009)
  te_null(
    false,
    &scope!(),
    r#"context put({x: 1, y: 3}, ["y", "a"], 2)"#,
    "context put: the entry 'y' on the key path holds number, not a context",
  );
}
