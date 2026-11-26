use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_string(false, &scope!(), r#"get value({a: "foo"}, "a")"#, r#"foo"#);
}

#[test]
fn _0002() {
  te_string(false, &scope!(), r#"get value(m: {a: "foo"}, key: "a")"#, r#"foo"#);
}

#[test]
fn _0003() {
  te_null(
    false,
    &scope!(),
    r#"get value(10, "a")"#,
    r#"[core::get value] invalid argument type, expected context, actual type is number"#,
  );
}

#[test]
fn _0004() {
  te_null(
    false,
    &scope!(),
    r#"get value({a: "foo"}, true)"#,
    r#"[core::get value] invalid argument type, expected string, actual type is boolean"#,
  );
}

#[test]
fn _0005() {
  te_null(false, &scope!(), r#"get value()"#, r#"expected 2 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0006() {
  te_null(false, &scope!(), r#"get value({a: "foo"})"#, r#"expected 2 parameters, actual number of parameters is 1"#);
}

#[test]
fn _0007() {
  te_null(
    false,
    &scope!(),
    r#"get value({a: "foo"},"a",2)"#,
    r#"expected 2 parameters, actual number of parameters is 3"#,
  );
}

#[test]
fn _0008() {
  te_null(
    false,
    &scope!(),
    r#"get value(map: {a: "foo"}, key: "a")"#,
    r#"expected the parameters of get value(m, key): Any, found the parameter name 'map'"#,
  );
}

#[test]
fn _0009() {
  te_null(
    false,
    &scope!(),
    r#"get value(m: {a: "foo"}, k: "a")"#,
    r#"expected the parameters of get value(m, key): Any, found the parameter name 'k'"#,
  );
}

#[test]
fn _0010() {
  te_be_value(false, &scope!(), r#"get value(m: {a: "foo"}, key: "b")"#, r#"null"#);
}

/// DMN 1.6 Table 82: `get value({key1 : "value 1"}, "unexistent-key") = null`, a null the
/// specification defines, so no error; a path over a list selects the key of each item the same
/// way.
#[test]
fn get_value_of_a_missing_key_is_a_null_without_an_error() {
  te_errors(r#"get value({a: 1}, "b")"#, "null", &[]);
  te_errors("[{a: 1}].b", "[null]", &[]);
}
