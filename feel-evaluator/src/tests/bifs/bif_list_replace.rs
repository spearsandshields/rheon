use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_value(false, &scope!(), r#"list replace([1, 7, 3], 2, 4)"#, "[1, 4, 3]");
}

#[test]
fn _0002() {
  te_value(false, &scope!(), r#"list replace(list: [1, 7, 3], position: 2, newItem: 4)"#, "[1, 4, 3]");
}

#[test]
fn _0003() {
  te_null(
    false,
    &scope!(),
    r#"list replace(null, 2, 4)"#,
    "[core::list replace] invalid argument type, expected list, actual type is Null",
  );
}

#[test]
fn _0004() {
  te_value(false, &scope!(), r#"list replace(2, 1, 4)"#, "[4]");
}

#[test]
fn _0005() {
  te_null(false, &scope!(), r#"list replace(2, 2, 4)"#, "position exceeds list bounds");
}

#[test]
fn _0006() {
  te_value(false, &scope!(), r#"list replace([1, 2, 3, 4, 5, 6], -2, 9)"#, "[1, 2, 3, 4, 9, 6]");
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"list replace([1, 2, 3, 4, 5, 6], -7, 9)"#, "position exceeds list bounds");
}

#[test]
fn _0008() {
  te_null(false, &scope!(), r#"list replace(2, 0, 4)"#, "position must not be zero");
}

#[test]
fn _0009() {
  te_null(false, &scope!(), r#"list replace(2, 32e+144, 4)"#, "position exceeds integer range");
}

#[test]
fn _0010() {
  te_null(false, &scope!(), r#"list replace(2, "1", 4)"#, "position must be number or function");
}

#[test]
fn _0011() {
  te_value(false, &scope!(), r#"list replace([2, 4, 7, 8], function(item, newItem) item < newItem, 5)"#, "[5, 5, 7, 8]");
}

#[test]
fn _0012() {
  te_value(
    false,
    &scope!(),
    r#"list replace(list: [2, 4, 7, 8], match: function(item, newItem) item < newItem, newItem: 5)"#,
    "[5, 5, 7, 8]",
  );
}

#[test]
fn _0013() {
  te_null(
    false,
    &scope!(),
    r#"list replace([2, 4, 7, 8], function(item, newItem, param) item < newItem, 5)"#,
    "list replace: matching function must have exactly two arguments",
  );
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    r#"list replace([2, 4, 7, 8], function(item) item  = 5, 5)"#,
    "list replace: matching function must have exactly two arguments",
  );
}

#[test]
fn _0015() {
  te_null(
    false,
    &scope!(),
    r#"list replace([2, 4, 7, 8], function(item, newItem) 17, 5)"#,
    "list replace: matching function must return boolean value",
  );
}

#[test]
fn _0016() {
  te_null(
    false,
    &scope!(),
    r#"list replace(lists: [1, 7, 3], position: 2, newItem: 4)"#,
    "expected the parameters of one of list replace(list, position, newItem): list, list replace(list, match, newItem): list, found the parameter name 'lists'",
  );
}

#[test]
fn _0017() {
  te_null(
    false,
    &scope!(),
    r#"list replace(list: [1, 7, 3], positions: 2, newItem: 4)"#,
    "expected the parameters of one of list replace(list, position, newItem): list, list replace(list, match, newItem): list, found the parameter name 'positions'",
  );
}

#[test]
fn _0018() {
  te_null(
    false,
    &scope!(),
    r#"list replace(list: [1, 7, 3], position: 2, newItems: 4)"#,
    "expected the parameters of one of list replace(list, position, newItem): list, list replace(list, match, newItem): list, found the parameter name 'newItems'",
  );
}

#[test]
fn _0019() {
  te_null(
    false,
    &scope!(),
    r#"list replace(list: [2, 4, 7, 8], matches: function(item, newItem) item < newItem, newItem: 5)"#,
    "expected the parameters of one of list replace(list, position, newItem): list, list replace(list, match, newItem): list, found the parameter name 'matches'",
  );
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#"list replace(list: [2, 4, 7, 8], matche: function(item, newItem) item < newItem, newItem: 5, oldItem: 12)"#,
    "expected the parameters of one of list replace(list, position, newItem): list, list replace(list, match, newItem): list, found the parameter name 'matche'",
  );
}

#[test]
fn _0021() {
  // the match function is invoked like any function: its arguments are converted and checked (Table 64)
  te_value(
    false,
    &scope!(),
    r#"list replace([[2], [4], [7]], function(item: number, newItem) item < 5, 0)"#,
    "[0, 0, [7]]",
  );
  te_null(
    false,
    &scope!(),
    r#"list replace(["a"], function(item: number, newItem) true, 0)"#,
    "list replace: matching function must return boolean value",
  );
}

#[test]
fn a_built_in_function_matches() {
  // a built-in function value is a matching function of two arguments (10.3.4.4)
  let scope = &te_scope("{}");
  te_be_value(false, scope, "list replace([1, 5, 3], before, 4)", "[4, 5, 4]");
}
