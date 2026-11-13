use crate::iterations::{FeelIterator, ForExpressionEvaluator};
use crate::tests::te_scope;
use rheon_feel::values::{Value, Values, values_to_string};
use rheon_feel::{FeelNumber, value_number};
use rheon_feel_temporal::FeelDate;
use std::ops::ControlFlow;
use std::str::FromStr;

/// The iteration contexts of all iterations of `iterator`, as text.
fn iterated(iterator: &mut FeelIterator) -> Vec<String> {
  let mut contexts = vec![];
  iterator.iterate_while(|ctx| {
    contexts.push(ctx.to_string());
    ControlFlow::Continue(())
  });
  contexts
}

/// The iteration contexts as the text of a list.
fn listed(contexts: &[String]) -> String {
  format!("[{}]", contexts.join(", "))
}

#[test]
fn _0001() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(3), false).unwrap();
  let actual = iterated(&mut iterator);
  assert_eq!(3, actual.len());
  assert_eq!(r#"[{x: 1}, {x: 2}, {x: 3}]"#, listed(&actual));
}

#[test]
fn _0002() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(3), value_number!(1), false).unwrap();
  let actual = iterated(&mut iterator);
  assert_eq!(3, actual.len());
  assert_eq!(r#"[{x: 3}, {x: 2}, {x: 1}]"#, listed(&actual));
}

#[test]
fn _0003() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(3), false).unwrap();
  iterator.add_interval("y".into(), value_number!(1), value_number!(5), false).unwrap();
  let actual = iterated(&mut iterator);
  assert_eq!(15, actual.len());
  assert_eq!(
    r#"[{x: 1, y: 1}, {x: 1, y: 2}, {x: 1, y: 3}, {x: 1, y: 4}, {x: 1, y: 5}, {x: 2, y: 1}, {x: 2, y: 2}, {x: 2, y: 3}, {x: 2, y: 4}, {x: 2, y: 5}, {x: 3, y: 1}, {x: 3, y: 2}, {x: 3, y: 3}, {x: 3, y: 4}, {x: 3, y: 5}]"#,
    listed(&actual)
  );
}

#[test]
fn _0004() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(3), false).unwrap();
  iterator.add_interval("y".into(), value_number!(5), value_number!(1), false).unwrap();
  let actual = iterated(&mut iterator);
  assert_eq!(15, actual.len());
  assert_eq!(
    r#"[{x: 1, y: 5}, {x: 1, y: 4}, {x: 1, y: 3}, {x: 1, y: 2}, {x: 1, y: 1}, {x: 2, y: 5}, {x: 2, y: 4}, {x: 2, y: 3}, {x: 2, y: 2}, {x: 2, y: 1}, {x: 3, y: 5}, {x: 3, y: 4}, {x: 3, y: 3}, {x: 3, y: 2}, {x: 3, y: 1}]"#,
    listed(&actual)
  );
}

#[test]
fn _0005() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(3), value_number!(1), false).unwrap();
  iterator.add_interval("y".into(), value_number!(5), value_number!(1), false).unwrap();
  let actual = iterated(&mut iterator);
  assert_eq!(15, actual.len());
  assert_eq!(
    r#"[{x: 3, y: 5}, {x: 3, y: 4}, {x: 3, y: 3}, {x: 3, y: 2}, {x: 3, y: 1}, {x: 2, y: 5}, {x: 2, y: 4}, {x: 2, y: 3}, {x: 2, y: 2}, {x: 2, y: 1}, {x: 1, y: 5}, {x: 1, y: 4}, {x: 1, y: 3}, {x: 1, y: 2}, {x: 1, y: 1}]"#,
    listed(&actual)
  );
}

#[test]
fn _0006() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(2), false).unwrap();
  iterator.add_interval("y".into(), value_number!(1), value_number!(3), false).unwrap();
  iterator.add_interval("z".into(), value_number!(1), value_number!(4), false).unwrap();
  let actual = iterated(&mut iterator);
  assert_eq!(24, actual.len());
  assert_eq!(
    r#"[{x: 1, y: 1, z: 1}, {x: 1, y: 1, z: 2}, {x: 1, y: 1, z: 3}, {x: 1, y: 1, z: 4}, {x: 1, y: 2, z: 1}, {x: 1, y: 2, z: 2}, {x: 1, y: 2, z: 3}, {x: 1, y: 2, z: 4}, {x: 1, y: 3, z: 1}, {x: 1, y: 3, z: 2}, {x: 1, y: 3, z: 3}, {x: 1, y: 3, z: 4}, {x: 2, y: 1, z: 1}, {x: 2, y: 1, z: 2}, {x: 2, y: 1, z: 3}, {x: 2, y: 1, z: 4}, {x: 2, y: 2, z: 1}, {x: 2, y: 2, z: 2}, {x: 2, y: 2, z: 3}, {x: 2, y: 2, z: 4}, {x: 2, y: 3, z: 1}, {x: 2, y: 3, z: 2}, {x: 2, y: 3, z: 3}, {x: 2, y: 3, z: 4}]"#,
    listed(&actual)
  );
}

#[test]
fn _0007() {
  let mut iterator = FeelIterator::new();
  let list = vec![Value::String("a".to_string()), Value::String("b".to_string()), Value::String("c".to_string())];
  iterator.add_list("x".into(), Value::List(list));
  let actual = iterated(&mut iterator);
  assert_eq!(3, actual.len());
  assert_eq!(r#"[{x: "a"}, {x: "b"}, {x: "c"}]"#, listed(&actual));
}

#[test]
fn _0008() {
  let mut iterator = FeelIterator::new();
  let list1 = vec![Value::String("a".to_string()), Value::String("b".to_string()), Value::String("c".to_string())];
  iterator.add_list("x".into(), Value::List(list1));
  let list2 = vec![value_number!(1), value_number!(2), value_number!(3)];
  iterator.add_list("y".into(), Value::List(list2));
  let actual = iterated(&mut iterator);
  assert_eq!(9, actual.len());
  assert_eq!(
    r#"[{x: "a", y: 1}, {x: "a", y: 2}, {x: "a", y: 3}, {x: "b", y: 1}, {x: "b", y: 2}, {x: "b", y: 3}, {x: "c", y: 1}, {x: "c", y: 2}, {x: "c", y: 3}]"#,
    listed(&actual)
  );
}

#[test]
fn _0009() {
  let mut iterator = FeelIterator::new();
  let list_x = vec![Value::String("a".to_string()), Value::String("b".to_string())];
  let list_y = vec![value_number!(1), value_number!(2), value_number!(3)];
  let list_z = vec![value_number!(1), value_number!(2), value_number!(3), value_number!(4)];
  iterator.add_list("x".into(), Value::List(list_x));
  iterator.add_list("y".into(), Value::List(list_y));
  iterator.add_list("z".into(), Value::List(list_z));
  let actual = iterated(&mut iterator);
  assert_eq!(24, actual.len());
  assert_eq!(
    r#"[{x: "a", y: 1, z: 1}, {x: "a", y: 1, z: 2}, {x: "a", y: 1, z: 3}, {x: "a", y: 1, z: 4}, {x: "a", y: 2, z: 1}, {x: "a", y: 2, z: 2}, {x: "a", y: 2, z: 3}, {x: "a", y: 2, z: 4}, {x: "a", y: 3, z: 1}, {x: "a", y: 3, z: 2}, {x: "a", y: 3, z: 3}, {x: "a", y: 3, z: 4}, {x: "b", y: 1, z: 1}, {x: "b", y: 1, z: 2}, {x: "b", y: 1, z: 3}, {x: "b", y: 1, z: 4}, {x: "b", y: 2, z: 1}, {x: "b", y: 2, z: 2}, {x: "b", y: 2, z: 3}, {x: "b", y: 2, z: 4}, {x: "b", y: 3, z: 1}, {x: "b", y: 3, z: 2}, {x: "b", y: 3, z: 3}, {x: "b", y: 3, z: 4}]"#,
    listed(&actual)
  );
}

#[test]
fn _0010() {
  let mut iterator = FeelIterator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(2), false).unwrap();
  iterator.add_list("y".into(), Value::List(vec![value_number!(1), value_number!(2), value_number!(3)]));
  let actual = iterated(&mut iterator);
  assert_eq!(6, actual.len());
  assert_eq!(r#"[{x: 1, y: 1}, {x: 1, y: 2}, {x: 1, y: 3}, {x: 2, y: 1}, {x: 2, y: 2}, {x: 2, y: 3}]"#, listed(&actual));
}

#[test]
fn _0011() {
  let mut iterator = ForExpressionEvaluator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(3)).unwrap();
  let scope = &te_scope(r#"{x:null}"#);
  let node = rheon_feel_parser::parse_expression(scope, "x+1", false).unwrap();
  let evaluator = crate::builders::build_evaluator(&node);
  let Value::List(actual) = iterator.evaluate(scope, &evaluator) else {
    panic!("expected a list")
  };
  assert_eq!(3, actual.len());
  assert_eq!(r#"[2, 3, 4]"#, values_to_string(&actual));
}

#[test]
fn _0012() {
  let mut iterator = ForExpressionEvaluator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(2)).unwrap();
  iterator.add_list("y".into(), Value::List(vec![value_number!(5), value_number!(6), value_number!(7)]));
  let scope = &te_scope(r#"{x:null,y:null}"#);
  let node = rheon_feel_parser::parse_expression(scope, "x+y", false).unwrap();
  let evaluator = crate::builders::build_evaluator(&node);
  let Value::List(actual) = iterator.evaluate(scope, &evaluator) else {
    panic!("expected a list")
  };
  assert_eq!(6, actual.len());
  assert_eq!(r#"[6, 7, 8, 7, 8, 9]"#, values_to_string(&actual));
}

#[test]
fn _0013() {
  let mut iterator = ForExpressionEvaluator::new();
  iterator.add_list("x".into(), Value::List(Values::default()));
  let scope = &te_scope(r#"{x:null}"#);
  let node = rheon_feel_parser::parse_expression(scope, "x+1", false).unwrap();
  let evaluator = crate::builders::build_evaluator(&node);
  let Value::List(actual) = iterator.evaluate(scope, &evaluator) else {
    panic!("expected a list")
  };
  assert_eq!(0, actual.len());
  assert_eq!(r#"[]"#, values_to_string(&actual));
}

#[test]
fn _0014() {
  let mut iterator = ForExpressionEvaluator::new();
  iterator.add_list("x".into(), value_number!(1));
  let scope = &te_scope(r#"{x:null}"#);
  let node = rheon_feel_parser::parse_expression(scope, "x+1", false).unwrap();
  let evaluator = crate::builders::build_evaluator(&node);
  let Value::List(actual) = iterator.evaluate(scope, &evaluator) else {
    panic!("expected a list")
  };
  assert_eq!(1, actual.len());
  assert_eq!(r#"[2]"#, values_to_string(&actual));
}

#[test]
fn _0015() {
  let mut iterator = ForExpressionEvaluator::new();
  iterator.add_interval("x".into(), value_number!(1), value_number!(2)).unwrap();
  iterator.add_list("y".into(), Value::List(Values::default()));
  let scope = &te_scope(r#"{x:null,y:null}"#);
  let node = rheon_feel_parser::parse_expression(scope, "x+1", false).unwrap();
  let evaluator = crate::builders::build_evaluator(&node);
  let Value::List(actual) = iterator.evaluate(scope, &evaluator) else {
    panic!("expected a list")
  };
  assert_eq!(0, actual.len());
  assert_eq!(r#"[]"#, values_to_string(&actual));
}

#[test]
fn _0016() {
  let mut iterator = ForExpressionEvaluator::new();
  let list = Value::List(vec![value_number!(1), value_number!(2), value_number!(3), value_number!(4)]);
  iterator.add_list("x".into(), list);
  iterator.add_variable("y".into(), "x".into());
  let scope = &te_scope(r#"{x:null,y:null}"#);
  let node = rheon_feel_parser::parse_expression(scope, "y", false).unwrap();
  let evaluator = crate::builders::build_evaluator(&node);
  let Value::List(actual) = iterator.evaluate(scope, &evaluator) else {
    panic!("expected a list")
  };
  assert_eq!(4, actual.len());
  assert_eq!(r#"[1, 2, 3, 4]"#, values_to_string(&actual));
}

/// One iteration context of a test iterator: an interval, a list, or a variable iterating over
/// the value of another variable.
enum State {
  Interval(&'static str, Value, Value),
  List(&'static str, Value),
  Variable(&'static str, &'static str),
}

/// An iterator over `states`, in this order (the last one advances fastest).
fn iterator(states: &[State]) -> FeelIterator {
  let mut iterator = FeelIterator::new();
  for state in states {
    match state {
      State::Interval(name, start, end) => iterator.add_interval((*name).into(), start.clone(), end.clone(), false).unwrap(),
      State::List(name, value) => iterator.add_list((*name).into(), value.clone()),
      State::Variable(name, bound) => iterator.add_variable((*name).into(), (*bound).into()),
    }
  }
  iterator
}

fn number(text: &str) -> Value {
  Value::Number(FeelNumber::from_str(text).unwrap())
}

fn date(text: &str) -> Value {
  Value::Date(FeelDate::from_str(text).unwrap())
}

fn numbers(items: &[i64]) -> Value {
  Value::List(items.iter().map(|item| Value::Number(FeelNumber::from(*item))).collect())
}

/// Checks the iteration contexts of every iterator in `cases`.
fn check(cases: Vec<(Vec<State>, &str)>) {
  for (index, (states, expected)) in cases.iter().enumerate() {
    assert_eq!(*expected, listed(&iterated(&mut iterator(states))), "case {}", index + 1);
  }
}

#[test]
fn _0017() {
  // Number intervals, ascending and descending, by steps of one. Numbers have 34 significant
  // digits, so `1e34 + 1 = 1e34`: the step is lost at 1e34, and the iteration ends there.
  check(vec![
    (vec![State::Interval("i", number("1"), number("5"))], "[{i: 1}, {i: 2}, {i: 3}, {i: 4}, {i: 5}]"),
    (vec![State::Interval("i", number("5"), number("1"))], "[{i: 5}, {i: 4}, {i: 3}, {i: 2}, {i: 1}]"),
    (vec![State::Interval("i", number("3"), number("3"))], "[{i: 3}]"),
    (vec![State::Interval("i", number("1.5"), number("4"))], "[{i: 1.5}, {i: 2.5}, {i: 3.5}]"),
    (vec![State::Interval("i", number("-1"), number("1"))], "[{i: -1}, {i: 0}, {i: 1}]"),
    (vec![State::Interval("i", number("1e34"), number("1e34"))], "[{i: 10000000000000000000000000000000000}]"),
    (
      vec![State::Interval("i", number("9999999999999999999999999999999990"), number("1e34"))],
      "[{i: 9999999999999999999999999999999990}, {i: 9999999999999999999999999999999991}, {i: 9999999999999999999999999999999992}, {i: 9999999999999999999999999999999993}, {i: 9999999999999999999999999999999994}, {i: 9999999999999999999999999999999995}, {i: 9999999999999999999999999999999996}, {i: 9999999999999999999999999999999997}, {i: 9999999999999999999999999999999998}, {i: 9999999999999999999999999999999999}, {i: 10000000000000000000000000000000000}]",
    ),
  ]);
}

#[test]
fn _0018() {
  // Date intervals, ascending and descending, by steps of one day.
  check(vec![
    (
      vec![State::Interval("d", date("2020-02-27"), date("2020-03-02"))],
      "[{d: 2020-02-27}, {d: 2020-02-28}, {d: 2020-02-29}, {d: 2020-03-01}, {d: 2020-03-02}]",
    ),
    (
      vec![State::Interval("d", date("2021-02-27"), date("2021-03-01"))],
      "[{d: 2021-02-27}, {d: 2021-02-28}, {d: 2021-03-01}]",
    ),
    (
      vec![State::Interval("d", date("2020-01-03"), date("2020-01-01"))],
      "[{d: 2020-01-03}, {d: 2020-01-02}, {d: 2020-01-01}]",
    ),
    (
      vec![State::Interval("d", date("2021-01-02"), date("2020-12-30"))],
      "[{d: 2021-01-02}, {d: 2021-01-01}, {d: 2020-12-31}, {d: 2020-12-30}]",
    ),
    (vec![State::Interval("d", date("2020-01-01"), date("2020-01-01"))], "[{d: 2020-01-01}]"),
  ]);
}

#[test]
fn _0019() {
  // Lists; a value that is not a list is iterated as the singleton list containing it.
  check(vec![
    (vec![State::List("x", numbers(&[]))], "[]"),
    (vec![State::List("x", numbers(&[7]))], "[{x: 7}]"),
    (vec![State::List("x", numbers(&[1, 2, 3]))], "[{x: 1}, {x: 2}, {x: 3}]"),
    (
      vec![State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[]), numbers(&[3])]))],
      "[{x: [1, 2]}, {x: []}, {x: [3]}]",
    ),
    (vec![State::List("x", Value::String("single".to_string()))], r#"[{x: "single"}]"#),
  ]);
}

#[test]
fn _0020() {
  // Variables iterating over the value of another variable (`y in x`, `z in y`); a value that
  // is not a list is iterated as a singleton list.
  //
  // When the list of `y` is empty, the iteration is not skipped and `y` keeps the value of the
  // preceding iteration (case 1, `{x: [], y: 2}`). By §10.3.2.14 an empty iteration context has no
  // elements to iterate over; the case pins the evaluated contexts, so that changing them is a
  // deliberate change.
  check(vec![
    (
      vec![
        State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[]), numbers(&[3])])),
        State::Variable("y", "x"),
      ],
      "[{x: [1, 2], y: 1}, {x: [1, 2], y: 2}, {x: [], y: 2}, {x: [3], y: 3}]",
    ),
    (
      vec![State::List("x", Value::List(vec![numbers(&[1, 2]), number("3")])), State::Variable("y", "x")],
      "[{x: [1, 2], y: 1}, {x: [1, 2], y: 2}, {x: 3, y: 3}]",
    ),
    (
      vec![
        State::List("x", Value::List(vec![Value::List(vec![numbers(&[1]), numbers(&[2, 3])]), Value::List(vec![numbers(&[4])])])),
        State::Variable("y", "x"),
        State::Variable("z", "y"),
      ],
      "[{x: [[1], [2, 3]], y: [1], z: 1}, {x: [[1], [2, 3]], y: [2, 3], z: 2}, {x: [[1], [2, 3]], y: [2, 3], z: 3}, {x: [[4]], y: [4], z: 4}]",
    ),
    (
      vec![
        State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[3])])),
        State::Variable("y", "x"),
        State::List("j", numbers(&[7, 8])),
      ],
      "[{j: 7, x: [1, 2], y: 1}, {j: 8, x: [1, 2], y: 1}, {j: 7, x: [1, 2], y: 2}, {j: 8, x: [1, 2], y: 2}, {j: 7, x: [3], y: 3}, {j: 8, x: [3], y: 3}]",
    ),
  ]);
}

#[test]
fn _0021() {
  // Two iteration contexts in either order: every combination, the last context advancing
  // fastest; an empty list in either position gives no iterations.
  check(vec![
    (
      vec![State::Interval("i", number("1"), number("2")), State::List("j", numbers(&[7, 8, 9]))],
      "[{i: 1, j: 7}, {i: 1, j: 8}, {i: 1, j: 9}, {i: 2, j: 7}, {i: 2, j: 8}, {i: 2, j: 9}]",
    ),
    (
      vec![State::List("j", numbers(&[7, 8, 9])), State::Interval("i", number("1"), number("2"))],
      "[{i: 1, j: 7}, {i: 2, j: 7}, {i: 1, j: 8}, {i: 2, j: 8}, {i: 1, j: 9}, {i: 2, j: 9}]",
    ),
    (vec![State::Interval("i", number("1"), number("2")), State::List("x", numbers(&[]))], "[]"),
    (vec![State::List("x", numbers(&[])), State::Interval("i", number("1"), number("2"))], "[]"),
    (
      vec![State::Interval("d", date("2020-03-01"), date("2020-02-28")), State::Interval("i", number("2"), number("1"))],
      "[{d: 2020-03-01, i: 2}, {d: 2020-03-01, i: 1}, {d: 2020-02-29, i: 2}, {d: 2020-02-29, i: 1}, {d: 2020-02-28, i: 2}, {d: 2020-02-28, i: 1}]",
    ),
    (
      vec![State::Interval("i", number("2"), number("1")), State::Interval("d", date("2020-03-01"), date("2020-02-28"))],
      "[{d: 2020-03-01, i: 2}, {d: 2020-02-29, i: 2}, {d: 2020-02-28, i: 2}, {d: 2020-03-01, i: 1}, {d: 2020-02-29, i: 1}, {d: 2020-02-28, i: 1}]",
    ),
    (
      vec![State::List("x", numbers(&[7])), State::Interval("i", number("1"), number("3"))],
      "[{i: 1, x: 7}, {i: 2, x: 7}, {i: 3, x: 7}]",
    ),
    (
      vec![State::Interval("i", number("1"), number("3")), State::List("x", numbers(&[7]))],
      "[{i: 1, x: 7}, {i: 2, x: 7}, {i: 3, x: 7}]",
    ),
    (
      vec![State::List("x", Value::String("single".to_string())), State::List("j", numbers(&[1, 2]))],
      r#"[{j: 1, x: "single"}, {j: 2, x: "single"}]"#,
    ),
    (
      vec![State::List("j", numbers(&[1, 2])), State::List("x", Value::String("single".to_string()))],
      r#"[{j: 1, x: "single"}, {j: 2, x: "single"}]"#,
    ),
    (
      vec![State::Interval("i", number("1e34"), number("1e34")), State::List("j", numbers(&[1, 2]))],
      "[{i: 10000000000000000000000000000000000, j: 1}, {i: 10000000000000000000000000000000000, j: 2}]",
    ),
    (
      vec![State::List("j", numbers(&[1, 2])), State::Interval("i", number("1e34"), number("1e34"))],
      "[{i: 10000000000000000000000000000000000, j: 1}, {i: 10000000000000000000000000000000000, j: 2}]",
    ),
  ]);
}

#[test]
fn _0022() {
  // A handler that returns Break ends the iteration after that iteration.
  let cases = [
    (vec![State::Interval("i", number("1"), number("5"))], "[{i: 1}, {i: 2}]"),
    (vec![State::List("x", numbers(&[7]))], "[{x: 7}]"),
    (vec![State::List("x", numbers(&[]))], "[]"),
    (
      vec![State::Interval("i", number("1"), number("2")), State::List("j", numbers(&[7, 8, 9]))],
      "[{i: 1, j: 7}, {i: 1, j: 8}]",
    ),
    (
      vec![State::List("j", numbers(&[7, 8, 9])), State::Interval("i", number("1"), number("2"))],
      "[{i: 1, j: 7}, {i: 2, j: 7}]",
    ),
    (
      vec![
        State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[3])])),
        State::Variable("y", "x"),
        State::List("j", numbers(&[7, 8])),
      ],
      "[{j: 7, x: [1, 2], y: 1}, {j: 8, x: [1, 2], y: 1}]",
    ),
  ];
  for (index, (states, expected)) in cases.iter().enumerate() {
    let mut seen = vec![];
    iterator(states).iterate_while(|ctx| {
      seen.push(ctx.to_string());
      if seen.len() == 2 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });
    assert_eq!(*expected, listed(&seen), "case {}", index + 1);
  }
}
