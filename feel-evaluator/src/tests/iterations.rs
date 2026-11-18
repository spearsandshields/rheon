use crate::iterations::{Domain, Iteration, Sequence};
use crate::tests::{build_evaluator, te_scope};
use rheon_feel::values::{Value, values_to_string};
use rheon_feel::{Evaluator, FeelNumber, FeelScope, Name, value_number};
use rheon_feel_temporal::FeelDate;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

/// One iteration context of a test iteration: an interval, a list, or a variable iterating over
/// the value of a variable before it.
enum State {
  Interval(&'static str, Value, Value),
  List(&'static str, Value),
  Variable(&'static str, &'static str),
}

/// The iteration of `states`, in this order (the last one advances fastest).
fn iteration(states: &[State]) -> Iteration {
  let variables = states
    .iter()
    .map(|state| -> (Name, Domain) {
      match state {
        State::Interval(name, start, end) => {
          let (start, end) = (start.clone(), end.clone());
          ((*name).into(), Box::new(move |_: &FeelScope| Sequence::interval(start.clone(), end.clone())))
        }
        State::List(name, value) => {
          let value = value.clone();
          ((*name).into(), Box::new(move |_: &FeelScope| Ok(Sequence::of(value.clone()))))
        }
        State::Variable(name, bound) => {
          let bound = Name::from(*bound);
          (
            (*name).into(),
            Box::new(move |scope: &FeelScope| Ok(Sequence::of(scope.get_value(&bound).unwrap_or(Value::Null(None))))),
          )
        }
      }
    })
    .collect();
  Iteration::new(variables)
}

/// The iteration contexts of the first `limit` iterations of `states`, as text: the body of an
/// `every` expression that records the context on the top of the scope.
fn visited(states: &[State], limit: usize) -> String {
  let seen = Arc::new(Mutex::new(vec![]));
  let recorded = Arc::clone(&seen);
  let body: Evaluator = Box::new(move |scope: &FeelScope| {
    let mut seen = recorded.lock().unwrap();
    seen.push(scope.peek().unwrap().to_string());
    Value::Boolean(seen.len() < limit)
  });
  iteration(states).every(&FeelScope::default(), &body);
  let seen = seen.lock().unwrap();
  format!("[{}]", seen.join(", "))
}

/// Checks the iteration contexts of every iteration in `cases`.
fn check(cases: Vec<(Vec<State>, &str)>) {
  for (index, (states, expected)) in cases.iter().enumerate() {
    assert_eq!(*expected, visited(states, usize::MAX), "case {}", index + 1);
  }
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

/// The values of `body` (FEEL text, the iteration variables bound) in every iteration of `states`.
fn collected(states: &[State], body: &str) -> String {
  collected_in("{}", states, body)
}

/// [collected] in the scope of the context `scope`.
fn collected_in(scope: &str, states: &[State], body: &str) -> String {
  let scope = &te_scope(scope);
  // the parser knows the names of the variables
  let names = states.iter().map(|state| match state {
    State::Interval(name, ..) | State::List(name, _) | State::Variable(name, _) => format!("{name}: null"),
  });
  let names = te_scope(&format!("{{{}}}", names.collect::<Vec<_>>().join(", ")));
  let node = rheon_feel_parser::parse_expression(&names, body).unwrap();
  let evaluator = build_evaluator(&node);
  let Value::List(values) = iteration(states).collect(scope, &evaluator) else {
    panic!("expected a list")
  };
  values_to_string(&values)
}

#[test]
fn _0001() {
  check(vec![
    (vec![State::Interval("x", value_number!(1), value_number!(3))], "[{x: 1}, {x: 2}, {x: 3}]"),
    (vec![State::Interval("x", value_number!(3), value_number!(1))], "[{x: 3}, {x: 2}, {x: 1}]"),
  ]);
}

#[test]
fn _0002() {
  // every combination, the last context advancing fastest
  check(vec![
    (
      vec![
        State::Interval("x", value_number!(1), value_number!(3)),
        State::Interval("y", value_number!(5), value_number!(1)),
      ],
      "[{x: 1, y: 5}, {x: 1, y: 4}, {x: 1, y: 3}, {x: 1, y: 2}, {x: 1, y: 1}, {x: 2, y: 5}, {x: 2, y: 4}, {x: 2, y: 3}, {x: 2, y: 2}, {x: 2, y: 1}, {x: 3, y: 5}, {x: 3, y: 4}, {x: 3, y: 3}, {x: 3, y: 2}, {x: 3, y: 1}]",
    ),
    (
      vec![
        State::Interval("x", value_number!(1), value_number!(2)),
        State::Interval("y", value_number!(1), value_number!(3)),
        State::Interval("z", value_number!(1), value_number!(2)),
      ],
      "[{x: 1, y: 1, z: 1}, {x: 1, y: 1, z: 2}, {x: 1, y: 2, z: 1}, {x: 1, y: 2, z: 2}, {x: 1, y: 3, z: 1}, {x: 1, y: 3, z: 2}, {x: 2, y: 1, z: 1}, {x: 2, y: 1, z: 2}, {x: 2, y: 2, z: 1}, {x: 2, y: 2, z: 2}, {x: 2, y: 3, z: 1}, {x: 2, y: 3, z: 2}]",
    ),
    (
      vec![
        State::List("x", Value::List(vec![Value::String("a".into()), Value::String("b".into())].into())),
        State::List("y", numbers(&[1, 2, 3])),
      ],
      r#"[{x: "a", y: 1}, {x: "a", y: 2}, {x: "a", y: 3}, {x: "b", y: 1}, {x: "b", y: 2}, {x: "b", y: 3}]"#,
    ),
  ]);
}

#[test]
fn _0003() {
  // the values of the body, with the implicit `partial` bound to the values before it
  assert_eq!("[2, 3, 4]", collected(&[State::Interval("x", value_number!(1), value_number!(3))], "x + 1"));
  assert_eq!(
    "[6, 7, 8, 7, 8, 9]",
    collected(&[State::Interval("x", value_number!(1), value_number!(2)), State::List("y", numbers(&[5, 6, 7]))], "x + y")
  );
  assert_eq!("[]", collected(&[State::List("x", numbers(&[]))], "x + 1"));
  assert_eq!("[2]", collected(&[State::List("x", value_number!(1))], "x + 1"));
  assert_eq!(
    "[]",
    collected(&[State::Interval("x", value_number!(1), value_number!(2)), State::List("y", numbers(&[]))], "x + 1")
  );
  assert_eq!("[0, 1, 2]", collected(&[State::List("x", numbers(&[7, 8, 9]))], "count(partial)"));
  assert_eq!(
    "[0, 1, 2, 3]",
    collected(&[State::List("x", numbers(&[1, 2])), State::List("y", numbers(&[3, 4]))], "count(partial)")
  );
  assert_eq!("[1, 2, 3, 4]", collected(&[State::List("x", numbers(&[1, 2, 3, 4])), State::Variable("y", "x")], "y"));
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
      vec![State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[]), numbers(&[3])].into()))],
      "[{x: [1, 2]}, {x: []}, {x: [3]}]",
    ),
    (vec![State::List("x", Value::String("single".into()))], r#"[{x: "single"}]"#),
  ]);
}

#[test]
fn _0020() {
  // Variables iterating over the value of a variable before them (`y in x`, `z in y`); a value
  // that is not a list is iterated as a singleton list. An empty list contributes no iteration
  // (DMN 1.6 §10.3.2.14: an iteration binds "the element to the corresponding variable"), and a
  // variable after a dependent one restarts in every iteration of it.
  check(vec![
    (
      vec![
        State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[]), numbers(&[3])].into())),
        State::Variable("y", "x"),
      ],
      "[{x: [1, 2], y: 1}, {x: [1, 2], y: 2}, {x: [3], y: 3}]",
    ),
    (
      vec![State::List("x", Value::List(vec![numbers(&[1, 2]), number("3")].into())), State::Variable("y", "x")],
      "[{x: [1, 2], y: 1}, {x: [1, 2], y: 2}, {x: 3, y: 3}]",
    ),
    (
      vec![
        State::List(
          "x",
          Value::List(vec![Value::List(vec![numbers(&[1]), numbers(&[2, 3])].into()), Value::List(vec![numbers(&[4])].into())].into()),
        ),
        State::Variable("y", "x"),
        State::Variable("z", "y"),
      ],
      "[{x: [[1], [2, 3]], y: [1], z: 1}, {x: [[1], [2, 3]], y: [2, 3], z: 2}, {x: [[1], [2, 3]], y: [2, 3], z: 3}, {x: [[4]], y: [4], z: 4}]",
    ),
    (
      vec![
        State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[3])].into())),
        State::Variable("y", "x"),
        State::List("j", numbers(&[7, 8])),
      ],
      "[{j: 7, x: [1, 2], y: 1}, {j: 8, x: [1, 2], y: 1}, {j: 7, x: [1, 2], y: 2}, {j: 8, x: [1, 2], y: 2}, {j: 7, x: [3], y: 3}, {j: 8, x: [3], y: 3}]",
    ),
    (
      vec![State::List("x", Value::List(vec![numbers(&[]), numbers(&[])].into())), State::Variable("y", "x")],
      "[]",
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
      vec![State::List("x", Value::String("single".into())), State::List("j", numbers(&[1, 2]))],
      r#"[{j: 1, x: "single"}, {j: 2, x: "single"}]"#,
    ),
    (
      vec![State::List("j", numbers(&[1, 2])), State::List("x", Value::String("single".into()))],
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
  // A decided body ends the iteration after that iteration.
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
        State::List("x", Value::List(vec![numbers(&[1, 2]), numbers(&[3])].into())),
        State::Variable("y", "x"),
        State::List("j", numbers(&[7, 8])),
      ],
      "[{j: 7, x: [1, 2], y: 1}, {j: 8, x: [1, 2], y: 1}]",
    ),
  ];
  for (index, (states, expected)) in cases.iter().enumerate() {
    assert_eq!(*expected, visited(states, 2), "case {}", index + 1);
  }
}

#[test]
fn _0023() {
  // A date interval that ends at the last or the first supported date, where no date follows,
  // restarts at its start when the iteration context before it advances.
  check(vec![
    (
      vec![
        State::Interval("i", number("1"), number("2")),
        State::Interval("d", date("262142-12-30"), date("262142-12-31")),
      ],
      "[{d: 262142-12-30, i: 1}, {d: 262142-12-31, i: 1}, {d: 262142-12-30, i: 2}, {d: 262142-12-31, i: 2}]",
    ),
    (
      vec![
        State::Interval("i", number("1"), number("2")),
        State::Interval("d", date("-262143-01-02"), date("-262143-01-01")),
      ],
      "[{d: -262143-01-02, i: 1}, {d: -262143-01-01, i: 1}, {d: -262143-01-02, i: 2}, {d: -262143-01-01, i: 2}]",
    ),
  ]);
}

#[test]
fn _0024() {
  // A domain sees the variables before it only: the variable of a closed context is unbound
  // (`z` when `y` is evaluated again), and so is the implicit `partial` of the `for` expression,
  // so a `partial` of the scope is seen.
  assert_eq!(
    "[[1, null, 5], [2, null, 5]]",
    collected(
      &[State::List("x", numbers(&[1, 2])), State::Variable("y", "z"), State::List("z", numbers(&[5]))],
      "[x, y, z]"
    )
  );
  assert_eq!(
    "[7, 7]",
    collected_in("{partial: 7}", &[State::List("x", numbers(&[1, 2])), State::Variable("y", "partial")], "y")
  );
}

/// The numbers of the interval `start..end` by adding or subtracting one, as a number does.
fn stepped(start: FeelNumber, end: FeelNumber) -> Vec<FeelNumber> {
  let ascending = start <= end;
  let mut items = vec![];
  let mut next = Some(start);
  while let Some(current) = next {
    let following = if ascending { current + FeelNumber::one() } else { current - FeelNumber::one() };
    let within = if ascending { following <= end } else { following >= end };
    next = (within && following != current).then_some(following);
    items.push(current);
  }
  items
}

#[test]
fn an_interval_of_integers_gives_the_numbers_that_stepping_by_one_gives() {
  let edge = i64::MAX;
  let cases = [
    ("1", "5"),
    ("5", "1"),
    ("-3", "3"),
    ("3", "-3"),
    ("0", "0"),
    ("-1", "0"),
    ("1", "0"),
    ("9007199254740990", "9007199254740994"),
    // held with a scale, or not integers: stepped as numbers
    ("1.0", "4"),
    ("1", "4.0"),
    ("1.5", "4"),
    ("1", "3.5"),
    ("4", "1.5"),
    ("1e2", "103"),
  ];
  let mut pairs: Vec<(FeelNumber, FeelNumber)> = cases
    .iter()
    .map(|(start, end)| (FeelNumber::from_str(start).unwrap(), FeelNumber::from_str(end).unwrap()))
    .collect();
  // the ends of the integer range: the count stops where a step would leave it
  pairs.push((FeelNumber::from(edge - 2), FeelNumber::from(edge)));
  pairs.push((FeelNumber::from(-edge + 2), FeelNumber::from(-edge)));
  for (start, end) in pairs {
    let mut sequence = Sequence::interval(Value::Number(start), Value::Number(end)).unwrap();
    let expected = stepped(start, end);
    let mut count = 0;
    while let Some(item) = sequence.next_item() {
      let Value::Number(item) = item else { panic!("an item of {start}..{end} is no number") };
      let wanted = expected.get(count).unwrap_or_else(|| panic!("{start}..{end} has more than {} items", expected.len()));
      assert!(item == *wanted, "{start}..{end}: item {count} is {item}, stepping gives {wanted}");
      // the same number in the same cohort
      assert_eq!(item.unscaled_i64(), wanted.unscaled_i64(), "{start}..{end}: item {count}");
      count += 1;
    }
    assert_eq!(count, expected.len(), "{start}..{end}");
  }
}
