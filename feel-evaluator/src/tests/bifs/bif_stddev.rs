use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), r#"stddev(2,4,7,5)"#, r#"2.081665999466132735282297706979931"#);
}

#[test]
fn _0002() {
  te_number(false, &scope!(), r#"decimal(stddev(2,4,7,5),13)"#, 20816659994661, 13);
}

#[test]
fn _0003() {
  te_number(false, &scope!(), r#"decimal(stddev(2,4,7,5),9)"#, 2081665999, 9);
}

#[test]
fn _0004() {
  te_be_value(false, &scope!(), r#"stddev([2,4,7,5])"#, r#"2.081665999466132735282297706979931"#);
}

#[test]
fn _0005() {
  te_be_value(false, &scope!(), r#"stddev(list:[2,4,7,5])"#, r#"2.081665999466132735282297706979931"#);
}

#[test]
fn _0006() {
  te_be_value(false, &scope!(), r#"stddev(5,6,8,9)"#, r#"1.825741858350553711523232609336007"#);
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"stddev()"#, r#"expected 1+ parameters, actual number of parameters is 0"#);
}

#[test]
fn _0008() {
  te_null(false, &scope!(), r#"stddev(true)"#, r#"stddev: expected number, actual type is boolean with value true"#);
}

#[test]
fn _0009() {
  te_null(
    false,
    &scope!(),
    r#"stddev(l: [2,4,7,5])"#,
    r#"expected the parameters of stddev(list): number, found the parameter name 'l'"#,
  );
}

#[test]
fn _0010() {
  te_null(false, &scope!(), r#"stddev([2])"#, r#"stddev: minimum two input arguments expected"#);
}

#[test]
fn _0011() {
  te_null(
    false,
    &scope!(),
    r#"stddev([2,"a","b"])"#,
    r#"stddev: expected number, actual type is string with value "a""#,
  );
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    r#"stddev([1, 4 ** 6174, 4 ** 6174, 4 ** 6174])"#,
    r#"stddev: intermediate result is not a finite number"#,
  );
}

#[test]
fn _0013() {
  te_null(false, &scope!(), r#"stddev([exp(7075),-exp(7074)])"#, r#"stddev: result is not a finite number"#);
}

/// DMN 1.6 §10.3.4.4, stddev: "If the list is empty or if the list contains only one element, the
/// function returns null", with `stddev( [ 47 ] ) = null` and `stddev( 47 ) = null`: a null the
/// specification defines, so no error; the reason stays as a note. An item that is no number is
/// outside the domain ("list is a list of number"), an error, also in a list of fewer than two.
#[test]
fn stddev_of_fewer_than_two_numbers_is_a_null_with_a_note() {
  let note = "null(stddev: minimum two input arguments expected)";
  te_errors("stddev([47])", note, &[]);
  te_errors("stddev([])", note, &[]);
  te_errors("stddev(47)", note, &[]);
  te_errors("stddev(list: [47])", note, &[]);
  let not_a_number = r#"stddev: expected number, actual type is string with value "a""#;
  te_errors(r#"stddev(["a"])"#, &format!("null({not_a_number})"), &[not_a_number]);
  let not_a_number = "stddev: expected number, actual type is boolean with value true";
  te_errors("stddev(true)", &format!("null({not_a_number})"), &[not_a_number]);
}
