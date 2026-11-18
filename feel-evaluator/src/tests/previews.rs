//! Values, types and numbers in null reasons are quoted as previews (`rheon_feel::message`).

use super::*;
use rheon_feel::scope;

/// The 120-character preview of the list of the numbers from 1.
const LIST: &str = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, ";

#[test]
fn a_compared_list_is_a_preview() {
  te_null(false, &scope!(), "(for i in 1..1000 return i) = 1", &format!("equal err '{LIST}… (1000 items)' =?= '1'"));
}

#[test]
fn a_context_without_the_entry_is_a_preview() {
  te_null(
    false,
    &scope!(),
    "{a: for i in 1..1000 return i}.b",
    "build_path: no entry b in context: {a: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, … (1 entry)",
  );
}

#[test]
fn a_long_number_is_in_scientific_notation() {
  te_null(false, &scope!(), r#"1e50 / "a""#, r#"[division] incompatible types: 1E+50 / "a""#);
  te_null(false, &scope!(), "sublist([1, 2, 3], 9e6144)", "sublist: invalid position value: 9E+6144");
}

#[test]
fn the_type_of_a_wide_context_is_a_preview() {
  let entries = (0..1000).map(|i| format!("entry{i}: {i}")).collect::<Vec<_>>().join(", ");
  let expression = format!("abs({{{entries}}})");
  let Value::Null(Some(reason)) = eval(&scope!(), &rheon_feel_parser::parse_expression(&scope!(), &expression).unwrap()) else {
    panic!("a null with a reason")
  };
  assert!(
    reason.starts_with("[core::abs] invalid argument type, expected number, actual type is context<entry0: number, "),
    "{reason}"
  );
  assert!(reason.ends_with("… (1000 entries)"), "{reason}");
}
