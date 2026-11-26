use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_string(false, &scope!(), r#"substring before("foobar","bar")"#, "foo");
}

#[test]
fn _0002() {
  te_string(false, &scope!(), r#"substring before(string: "foobar", match: "bar")"#, "foo");
}

#[test]
fn _0003() {
  te_string(false, &scope!(), r#"substring before("foobar","xyz")"#, "");
}

#[test]
fn _0004() {
  te_null(false, &scope!(), r#"substring before()"#, r#"expected 2 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0005() {
  te_null(
    false,
    &scope!(),
    r#"substring before("a","b","c")"#,
    r#"expected 2 parameters, actual number of parameters is 3"#,
  );
}

#[test]
fn _0006() {
  te_null(
    false,
    &scope!(),
    r#"substring before(s: "foobar", match: "bar")"#,
    r#"expected the parameters of substring before(string, match): string, found the parameter name 's'"#,
  );
}

#[test]
fn _0007() {
  te_null(
    false,
    &scope!(),
    r#"substring before(string: "foobar", m: "bar")"#,
    r#"expected the parameters of substring before(string, match): string, found the parameter name 'm'"#,
  );
}

#[test]
fn _0008() {
  te_string(false, &scope!(), r#"substring before(["foobar"], "ob")"#, "fo");
}

#[test]
fn _0009() {
  te_string(false, &scope!(), r#"substring before("foobar", ["ob"])"#, "fo");
}
