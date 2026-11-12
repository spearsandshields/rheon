use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  let scope = &te_scope(r#"{add: function (x: number, y: number) x + y }"#);
  te_number(false, scope, r#"add(17.82,32.18)"#, 50, 0);
}

#[test]
fn _0002() {
  let scope = &te_scope(
    r#"{
      a: 10,
      add: function (x: number, y: number) x + y + a
    }"#,
  );
  te_number(false, scope, r#"add(17.82,32.18)"#, 60, 0);
}

#[test]
fn _0003() {
  let scope = &te_scope(r#"{ add: function (x: number, y: number) x + y + a }"#);
  te_null(false, scope, r#"add(17.82,32.18)"#, "context has no value for key 'a'");
}

#[test]
fn _0004() {
  let scope = &te_scope(
    r#"{
      a: {
        b: {
          c: 10
        }
      },
      add: function (x: number, y: number) x + y + a.b.c
    }"#,
  );
  te_number(false, scope, r#"add(17.82,32.18)"#, 60, 0);
}

#[test]
fn _0005() {
  let scope = &te_scope(
    r#"{
      a: {
        date: {
          time: {
            duration: {
              date and time: 10
            }
          }
        }
      },
      add: function (date: number, time: number) date + time + a.date.time.duration.date and time
    }"#,
  );
  te_number(false, scope, r#"add(17.82,32.18)"#, 60, 0);
}

#[test]
fn _0006() {
  let scope = &te_scope(
    r#"{
      a: @"2023-01-07",
      add: function (x: number, y: number) x + y + a.year
    }"#,
  );
  te_number(false, scope, r#"add(17.82,32.18)"#, 2073, 0);
}

#[test]
fn _0007() {
  let scope = &te_scope(r#"{add: function(x: number, y: number) x + y }"#);
  te_number(false, scope, r#"add(x: 17.82, y: 32.18)"#, 50, 0);
}

#[test]
fn _0008() {
  let scope = &te_scope(r#"{add: function(x: number, y: number) x + y }"#);
  te_null(false, scope, r#"add(k: 17.82,l: 32.18)"#, "parameter with name x not found in arguments");
}

#[test]
fn _0009() {
  let scope = &te_scope(r#"{add: function(x: number, y: number) x + y }"#);
  te_null(false, scope, r#"add(x: 17.82,y: 32.18, z:11.2)"#, "invalid number of arguments");
}

#[test]
fn _0010() {
  let scope = &te_scope(r#"{add: function(x, y) x + y }"#);
  te_null(false, scope, r#"add(17.82, 32.18, 11.2)"#, "invalid number of arguments");
}

#[test]
fn _0011() {
  te_null(
    false,
    &scope!(),
    r#"pull(2)"#,
    "expected built-in function name or function definition, actual is null(context has no value for key 'pull')",
  );
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    r#"pull(x: 2)"#,
    "expected built-in function name or function definition, actual is null(context has no value for key 'pull')",
  );
}

#[test]
fn _0013() {
  // an argument that does not conform to the type of its parameter: the function is not invoked
  let not_conformant = "argument for parameter 'a' does not conform to type number";
  te_null(false, &scope!(), r#"(function(a: number) "invoked")("x")"#, not_conformant);
  te_null(false, &scope!(), r#"(function(a: number) "invoked")(a: "x")"#, not_conformant);
  te_null(false, &scope!(), r#"(function(a: number) a = null)("x")"#, not_conformant);
}

#[test]
fn _0014() {
  // implicit conversions apply before the argument is checked, and null conforms to every type
  te_be_value(false, &scope!(), r#"(function(a: number) a + 1)([1])"#, "2");
  te_be_value(false, &scope!(), r#"(function(a: list<number>) a)(1)"#, "[1]");
  te_bool(false, &scope!(), r#"(function(a: number) a = null)(null)"#, true);
}

#[test]
fn _0015() {
  // null is null, whatever produced it
  let scope = &te_scope(r#"{f: function(a: number) a}"#);
  te_bool(false, scope, r#"f("x") = null"#, true);
  te_bool(false, scope, r#"null = f("x")"#, true);
  te_bool(false, scope, r#"f("x") != null"#, false);
  te_bool(false, scope, r#"f("x") = f("y")"#, true);
  te_bool(false, scope, r#"f("x") = 1"#, false);
}
