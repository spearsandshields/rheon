//! The range functions on a range with an undefined endpoint (DMN 1.6 Table 42 and Table 79):
//! the formula of the function with the three truth values of FEEL.

use super::*;
use rheon_feel::scope;

fn yes(text: &str) {
  te_bool(false, &scope!(), text, true);
}

fn no(text: &str) {
  te_bool(false, &scope!(), text, false);
}

/// Null with the one error that names the function and the range.
fn undecided(text: &str, name: &str, range: &str) {
  undecided_with(text, name, range, "");
}

/// The same for a value and a range, where the message names the operator that decides.
fn undecided_value(text: &str, name: &str, range: &str) {
  undecided_with(text, name, range, "; 'value in range' tests a value against such a range");
}

fn undecided_with(text: &str, name: &str, range: &str, hint: &str) {
  let reason = format!("[{name}] expected endpoints that decide the result, found the range {range} with an undefined endpoint{hint}");
  te_errors(text, &format!("null({reason})"), &[&reason]);
}

#[test]
fn before_and_after_a_range_with_one_endpoint() {
  yes("before(5, (> 10))");
  yes("before(10, (> 10))");
  no("before(10, (>= 10))");
  no("before(11, (>= 10))");
  undecided("before(5, (< 10))", "before", "< 10");
  yes("before((< 10), 20)");
  yes("before((< 10), 10)");
  no("before((<= 10), 10)");
  undecided("before((> 10), 20)", "before", "> 10");
  yes("before((< 5), (> 5))");
  no("before((<= 5), (>= 5))");
  yes("before((< 5), [5..9])");
  undecided("before((> 5), [7..9])", "before", "> 5");
  yes("after(20, (< 10))");
  yes("after(10, (< 10))");
  no("after(10, (<= 10))");
  undecided("after(5, (> 1))", "after", "> 1");
  yes("after((> 5), 5)");
  no("after((>= 5), 5)");
  yes("after((> 5), (< 5))");
  yes(r#"before(@"2020-01-01", (> @"2021-01-01"))"#);
  yes(r#"after("b", (< "a"))"#);
}

#[test]
fn includes_and_during() {
  yes("includes((<= 10), 10)");
  no("includes((< 10), 10)");
  no("includes((< 10), 20)");
  undecided_value("includes((< 10), 5)", "includes", "< 10");
  yes("includes((< 10), (< 5))");
  yes("includes((< 10), (< 10))");
  no("includes((< 10), (<= 10))");
  no("includes((< 10), (< 20))");
  undecided("includes((< 10), [1..5])", "includes", "< 10");
  no("includes([1..5], (< 9))");
  undecided("includes([1..5], (< 3))", "includes", "< 3");
  yes("during(5, (>= 5))");
  no("during(5, (> 5))");
  yes("during((> 7), (> 5))");
  undecided_value("during(7, (> 5))", "during", "> 5");
}

#[test]
fn overlaps_meets_and_coincides() {
  no("overlaps((< 10), [20..30])");
  no("overlaps((< 10), [10..30])");
  undecided("overlaps((< 10), [5..30])", "overlaps", "< 10");
  no("overlaps([20..30], (< 10))");
  no("overlaps before((> 5), (> 5))");
  no("overlaps after((< 5), [7..9])");
  yes("meets((<= 5), (>= 5))");
  no("meets((< 5), (>= 5))");
  no("meets((> 5), (>= 5))");
  yes("met by((>= 5), (<= 5))");
  no("met by((> 5), (<= 5))");
  yes("coincides((< 10), (< 10))");
  no("coincides((< 10), (<= 10))");
  no("coincides((< 10), [1..10))");
  no("coincides((< 10), (> 10))");
}

#[test]
fn starts_and_finishes() {
  yes("starts(5, (>= 5))");
  no("starts(5, (> 5))");
  no("starts(5, (< 9))");
  yes("started by((>= 5), 5)");
  yes("starts((>= 1), (>= 1))");
  no("starts((>= 1), (> 1))");
  no("starts([1..5], (>= 2))");
  undecided("starts([1..5], (>= 1))", "starts", ">= 1");
  yes("started by((>= 1), (>= 1))");
  yes("finishes(10, (<= 10))");
  no("finishes(10, (< 10))");
  no("finishes(10, (> 1))");
  yes("finished by((<= 10), 10)");
  yes("finishes((<= 10), (<= 10))");
  undecided("finishes([1..10], (<= 10))", "finishes", "<= 10");
  no("finishes([1..9], (<= 10))");
}

#[test]
fn a_range_made_by_the_range_function_is_the_same_range() {
  yes(r#"before(5, range("(10..)"))"#);
  yes(r#"coincides(range("(..10)"), (< 10))"#);
  undecided_value(r#"includes(range("(..10)"), 5)"#, "includes", "(..10)");
}

#[test]
fn values_of_other_types_are_no_arguments() {
  for text in [
    r#"before("a", (> 10))"#,
    "before(true, (> 10))",
    "includes((< 10), null)",
    r#"overlaps((< 10), (< "a"))"#,
    "meets(5, (> 5))",
  ] {
    let (value, diagnostics) = te_collect(text);
    assert!(value.is_null(), "{text}: {value}");
    assert_eq!(1, diagnostics.count, "{text}: {:?}", diagnostics.messages);
    assert!(diagnostics.messages[0].contains("invalid argument type"), "{text}: {:?}", diagnostics.messages);
  }
}
