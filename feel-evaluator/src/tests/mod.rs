#![cfg(test)]

use crate::builders::build_evaluator;
use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, FeelScope};
use rheon_feel_parser::AstNode;
use rheon_feel_temporal::{Day, FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration, Month, Year};

mod addition;
mod arithmetic_negation;
mod bfeel;
mod bifs;
mod comments;
mod comparison_between;
mod comparison_eq;
mod comparison_ge;
mod comparison_gt;
mod comparison_in;
mod comparison_le;
mod comparison_lt;
mod comparison_ne;
mod conjunction;
mod context;
mod descendant;
mod disjunction;
mod division;
mod empty_input;
mod evaluation;
mod every_expression;
mod exponentiation;
mod external_functions;
mod filter;
mod for_expression;
mod formal_parameters;
mod function_definition;
mod function_invocation;
mod if_expression;
mod instance_of;
mod iterations;
mod join;
mod literal_at;
mod literal_boolean;
mod literal_number;
mod multiline;
mod multiplication;
mod name;
mod negation;
mod nested_lists;
mod out_operator;
mod parentheses;
mod path;
mod previews;
mod properties;
mod range;
mod satisfies;
mod send_sync;
mod shared_values;
mod some_expression;
mod subtraction;
mod types;
mod unary_tests;
mod values_used_again;
mod various;

const SECONDS_IN_DAY: i64 = 86_400;
const SECONDS_IN_HOUR: i64 = 3_600;
const SECONDS_IN_MINUTE: i64 = 60;

/// Evaluates `node` in its own session; a stopped evaluation fails the test with its abort.
pub fn eval(scope: &FeelScope, node: &AstNode) -> Value {
  crate::evaluate(scope, node).into_result().unwrap_or_else(|abort| panic!("evaluation stopped: {abort}"))
}

/// Evaluates `text` in an empty scope while collecting its diagnostics.
fn te_collect(text: &str) -> (Value, rheon_feel::diagnostics::Diagnostics) {
  let scope = FeelScope::default();
  let evaluator = build_evaluator(&rheon_feel_parser::parse_textual_expression(&scope, text, false).unwrap());
  rheon_feel::diagnostics::collect(|| evaluator(&scope))
}

/// Asserts that evaluating `text` gives a value whose text is `expected`, with exactly the
/// warnings `warnings` and no error.
pub fn te_warnings(text: &str, expected: &str, warnings: &[&str]) {
  let (value, diagnostics) = te_collect(text);
  assert_eq!(expected, value.to_string(), "{text}");
  assert_eq!(warnings, diagnostics.warnings(), "{text}");
  assert_eq!(0, diagnostics.count, "{text}");
}

/// Asserts that evaluating `text` gives a value whose text is `expected`, with exactly the
/// errors `errors` counted and listed (DMN 1.6 §7.3.8): a null that the specification defines
/// for its input is no error.
pub fn te_errors(text: &str, expected: &str, errors: &[&str]) {
  let (value, diagnostics) = te_collect(text);
  assert_eq!(expected, value.to_string(), "{text}");
  assert_eq!(errors, diagnostics.messages, "{text}");
  assert_eq!(errors.len(), diagnostics.count, "{text}");
}

/// Asserts that a null on either side of the binary arithmetic operator `symbol`, beside a value of
/// every kind, is an error (DMN 1.6, before Table 57 and before Table 59: "if input values are not
/// of the listed types, the result is null"; the official TCK expects an error result for every
/// such operation, compliance-level-3/0100-arithmetic): the value is null, its reason
/// `[<name>] incompatible types: <operation>` is counted once, and a failing operand counts its own
/// error before it.
pub fn te_null_operands(name: &str, symbol: &str) {
  for (value, shown) in [
    ("1", "1"),
    (r#""a""#, r#""a""#),
    (r#"@"2020-01-01""#, "2020-01-01"),
    (r#"@"2020-01-01T10:00:00""#, "2020-01-01T10:00:00"),
    (r#"@"10:00:00""#, "10:00:00"),
    (r#"@"P1D""#, "P1D"),
    (r#"@"P1Y""#, "P1Y"),
    ("true", "true"),
    ("[1]", "[1]"),
    ("{a: 1}", "{a: 1}"),
    ("[1..2]", "[1..2]"),
    ("null", "null"),
  ] {
    let reason = format!("[{name}] incompatible types: {shown} {symbol} null");
    te_errors(&format!("{value} {symbol} null"), &format!("null({reason})"), &[&reason]);
    let reason = format!("[{name}] incompatible types: null {symbol} {shown}");
    te_errors(&format!("null {symbol} {value}"), &format!("null({reason})"), &[&reason]);
  }
  let division_by_zero = "[division] division by zero";
  let reason = format!(r#"[{name}] incompatible types: "a" {symbol} null({division_by_zero})"#);
  te_errors(&format!(r#""a" {symbol} (1/0)"#), &format!("null({reason})"), &[division_by_zero, &reason]);
  let reason = format!(r#"[{name}] incompatible types: null({division_by_zero}) {symbol} "a""#);
  te_errors(&format!(r#"(1/0) {symbol} "a""#), &format!("null({reason})"), &[division_by_zero, &reason]);
}

/// The warning that the seconds of `literal` lost the fraction digits `dropped`.
pub fn truncation_warning(literal: &str, dropped: &str) -> String {
  format!("fractional seconds of '{literal}' truncated to nanoseconds; dropped digits: '{dropped}'")
}

/// Asserts that evaluating `text` in `dialect` stops the evaluation, with `message`.
pub fn te_aborted(text: &str, dialect: crate::Dialect, message: &str) {
  let scope = FeelScope::default();
  let evaluator = crate::prepare_with_dialect(&rheon_feel_parser::parse_textual_expression(&scope, text, false).unwrap(), dialect);
  let outcome = crate::Session::new().run(|| evaluator(&scope)).into_result();
  assert_eq!(
    Err(message.to_string()),
    outcome.map(|value| value.to_string()).map_err(|abort| abort.to_string()),
    "{text}"
  );
}

/// A pattern of `levels` nested groups around `a`.
pub fn nested_groups(levels: usize) -> String {
  format!("{}a{}", "(".repeat(levels), ")".repeat(levels))
}

/// Utility function that tests evaluation of boolean value.
pub fn te_bool(trace: bool, scope: &FeelScope, s: &str, expected: bool) {
  textual_expression(trace, scope, s, Value::Boolean(expected));
}

/// Utility function that tests evaluation of date value.
pub fn te_date(trace: bool, scope: &FeelScope, expression: &str, year: Year, month: Month, day: Day) {
  textual_expression(trace, scope, expression, Value::Date(FeelDate::new(year, month, day).unwrap()));
}

/// Utility function that tests evaluation of local date and time value.
pub fn te_date_time_local(trace: bool, scope: &FeelScope, s: &str, date: (Year, Month, Day), time: (u8, u8, u8, u64)) {
  let (year, month, day) = date;
  let (hour, min, sec, nano) = time;
  textual_expression(trace, scope, s, Value::DateTime(FeelDateTime::local(year, month, day, hour, min, sec, nano).unwrap()));
}

/// Utility function that tests evaluation of UTC date and time value.
pub fn te_date_time_utc(trace: bool, scope: &FeelScope, s: &str, date: (Year, Month, Day), time: (u8, u8, u8, u64)) {
  let (year, month, day) = date;
  let (hour, min, sec, nano) = time;
  textual_expression(trace, scope, s, Value::DateTime(FeelDateTime::utc(year, month, day, hour, min, sec, nano).unwrap()));
}

/// Utility function that tests evaluation of date and time value with explicit offset.
pub fn te_date_time_offset(trace: bool, scope: &FeelScope, s: &str, date: (Year, Month, Day), time: (u8, u8, u8, u64), offset: i32) {
  textual_expression(trace, scope, s, Value::DateTime(FeelDateTime::offset(date, time, offset).unwrap()));
}

/// Utility function that creates a scope from specified input.
pub fn te_scope(input: &str) -> FeelScope {
  let scope = FeelScope::default();
  match rheon_feel_parser::parse_context(&scope, input, false) {
    Ok(node) => match eval(&scope, &node) {
      Value::Context(ctx) => ctx.into(),
      other => {
        println!("ERROR (INVALID VALUE TYPE): {other}");
        panic!("te_scope failed");
      }
    },
    Err(reason) => {
      println!("ERROR (REASON): {reason}");
      panic!("te_scope failed");
    }
  }
}

/// Utility function that tests evaluation of numeric values.
pub fn te_number(trace: bool, scope: &FeelScope, s: &str, num: i64, scale: i32) {
  textual_expression(trace, scope, s, Value::Number(FeelNumber::new(num, scale)));
}

/// Utility function that tests evaluation of numeric values.
pub fn te_number_x(trace: bool, scope: &FeelScope, s: &str, num: &str) {
  textual_expression(trace, scope, s, Value::Number(num.parse::<FeelNumber>().unwrap()));
}

/// Utility function that tests evaluation to null value.
fn te_null(trace: bool, scope: &FeelScope, s: &str, t: &str) {
  textual_expression(trace, scope, s, if t.is_empty() { Value::Null(None) } else { Value::Null(Some(t.to_owned())) });
}

/// Utility function that tests evaluation to an error result.
pub fn te_none(trace: bool, scope: &FeelScope, s: &str) {
  assert!(rheon_feel_parser::parse_textual_expression(scope, s, trace).is_err());
}

/// Utility function that tests evaluation to string value.
fn te_string(trace: bool, scope: &FeelScope, s: &str, expected: &str) {
  textual_expression(trace, scope, s, Value::String(expected.into()));
}

/// Utility function that tests evaluation of year and months duration.
pub fn te_years_and_months_duration(trace: bool, scope: &FeelScope, s: &str, years: i64, months: i64) {
  textual_expression(trace, scope, s, Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_ym(years, months).unwrap()));
}

/// Utility function that tests evaluation of year and months duration.
pub fn te_years_and_months_duration_x(trace: bool, scope: &FeelScope, s: &str, expected: &str) {
  textual_expression(trace, scope, s, Value::YearsAndMonthsDuration(expected.try_into().unwrap()));
}

/// Utility function that tests evaluation of days and time duration.
pub fn te_days_and_time_duration(trace: bool, scope: &FeelScope, s: &str, neg: bool, sec: i64, nano: i64) {
  textual_expression(
    trace,
    scope,
    s,
    Value::DaysAndTimeDuration(if neg {
      -FeelDaysAndTimeDuration::from_sn(sec, nano).unwrap()
    } else {
      FeelDaysAndTimeDuration::from_sn(sec, nano).unwrap()
    }),
  );
}

/// Utility function that tests evaluation of days and time duration.
pub fn te_days_and_time_duration_x(trace: bool, scope: &FeelScope, s: &str, expected: &str) {
  textual_expression(trace, scope, s, Value::DaysAndTimeDuration(expected.try_into().unwrap()));
}

/// Utility function that tests evaluation of time.
pub fn te_time(trace: bool, scope: &FeelScope, s: &str, expected: FeelTime) {
  textual_expression(trace, scope, s, Value::Time(expected));
}

/// Utility function that tests evaluation of context.
pub fn te_context<Actual: ToString, Expected: ToString>(trace: bool, scope: &FeelScope, actual: Actual, expected: Expected) {
  match rheon_feel_parser::parse_context(scope, &expected.to_string(), trace) {
    Ok(node) => {
      let evaluator = build_evaluator(&node);
      textual_expression(trace, scope, &actual.to_string(), evaluator(scope));
    }
    Err(reason) => {
      println!("ERROR: {reason}");
      panic!("te_value failed");
    }
  }
}

/// Utility function that tests evaluation to specified value.
pub fn te_value(trace: bool, scope: &FeelScope, actual: &str, expected: &str) {
  match rheon_feel_parser::parse_expression(scope, expected, trace) {
    Ok(node) => {
      let evaluator = build_evaluator(&node);
      textual_expression(trace, scope, actual, evaluator(scope));
    }
    Err(reason) => {
      println!("ERROR: {reason}");
      panic!("te_value failed");
    }
  }
}

/// Utility function that tests evaluation to specified value represented by boxed expression.
pub fn te_be_value(trace: bool, scope: &FeelScope, actual: &str, expected: &str) {
  match rheon_feel_parser::parse_expression(scope, expected, false) {
    Ok(node) => {
      textual_expression(trace, scope, actual, eval(scope, &node));
    }
    Err(reason) => {
      println!("ERROR (REASON): {reason}");
      panic!("te_value failed");
    }
  }
}

/// Utility function that tests evaluation to specified value represented by boxed expression.
pub fn be_be_value(trace: bool, scope: &FeelScope, actual: &str, expected: &str) {
  match rheon_feel_parser::parse_boxed_expression(scope, expected, trace) {
    Ok(node) => {
      boxed_expression(trace, scope, actual, eval(scope, &node));
    }
    Err(reason) => {
      println!("ERROR (REASON): {reason}");
      panic!("te_value failed");
    }
  }
}

/// Utility function that takes a text parameter, evaluates the boxed expression
/// represented by this text and compares the result with provided expected value.
/// The result must be equal to expected value, otherwise an error is reported.
pub fn boxed_expression(trace: bool, scope: &FeelScope, text: &str, expected: Value) {
  match rheon_feel_parser::parse_boxed_expression(scope, text, trace) {
    Ok(node) => {
      assert_eq!(eval(scope, &node), expected);
    }
    Err(reason) => {
      println!("ERROR: {reason}");
      panic!("boxed_expression failed");
    }
  }
}

/// Utility function that takes a text parameter, evaluates the textual expression
/// represented by this text and compares the result with provided expected value.
/// The result must be equal to expected value, otherwise an error is reported.
fn textual_expression(trace: bool, scope: &FeelScope, text: &str, expected: Value) {
  match rheon_feel_parser::parse_textual_expression(scope, text, trace) {
    Ok(node) => {
      let evaluator = build_evaluator(&node);
      let actual = evaluator(scope);
      assert_eq!(actual, expected, "ERROR\nexpected: {expected}\n  actual: {actual}\n");
    }
    Err(reason) => {
      panic!("parsing textual expression failed with reason: {}", reason);
    }
  }
}

/// Utility function that checks if unary tests are correctly parsed.
pub fn valid_unary_tests(trace: bool, scope: &FeelScope, text: &str) {
  match rheon_feel_parser::parse_unary_tests(scope, text, trace) {
    Ok(node) => {
      let evaluator = build_evaluator(&node);
      if let v @ Value::Null(_) = evaluator(scope) {
        panic!("evaluating unary tests failed, value: {}", v)
      }
    }
    Err(reason) => {
      panic!("parsing unary tests failed with reason: {}", reason);
    }
  }
}

pub fn satisfies(trace: bool, scope: &FeelScope, input_expression: &str, input_values: &str, input_entry: &str, expected: bool) {
  let input_expression_node = rheon_feel_parser::parse_textual_expression(scope, input_expression, trace).unwrap();
  let input_entry_node = rheon_feel_parser::parse_unary_tests(scope, input_entry, trace).unwrap();
  let node = if !input_values.is_empty() {
    let input_values_node = rheon_feel_parser::parse_unary_tests(scope, input_values, trace).unwrap();
    let left = AstNode::In(Box::new(input_expression_node.clone()), Box::new(input_values_node));
    let right = AstNode::In(Box::new(input_expression_node), Box::new(input_entry_node));
    AstNode::And(Box::new(left), Box::new(right))
  } else {
    AstNode::In(Box::new(input_expression_node), Box::new(input_entry_node))
  };
  assert_eq!(eval(scope, &node), Value::Boolean(expected));
}

pub fn satisfies_null(trace: bool, scope: &FeelScope, input_expression: &str, input_values: &str, input_entry: &str, expected: &str) {
  let input_expression_node = rheon_feel_parser::parse_textual_expression(scope, input_expression, trace).unwrap();
  let input_entry_node = rheon_feel_parser::parse_unary_tests(scope, input_entry, trace).unwrap();
  let node = if !input_values.is_empty() {
    let input_values_node = rheon_feel_parser::parse_unary_tests(scope, input_values, trace).unwrap();
    let left = AstNode::In(Box::new(input_expression_node.clone()), Box::new(input_values_node));
    let right = AstNode::In(Box::new(input_expression_node), Box::new(input_entry_node));
    AstNode::And(Box::new(left), Box::new(right))
  } else {
    AstNode::In(Box::new(input_expression_node), Box::new(input_entry_node))
  };
  assert_eq!(eval(scope, &node), Value::Null(Some(expected.to_string())));
}

/// Utility function for testing if parsed `expression` evaluates to [FeelDateTime]
/// and the evaluated value is in the range (`date_time` .. `date_time + seconds`).
pub fn te_date_time_local_after(trace: bool, scope: &FeelScope, expression: &str, date_time: FeelDateTime, seconds: u8) {
  let (year, month, day) = date_time.date().as_tuple();
  let (hour, min, sec, nano, _) = date_time.time().as_tuple();
  let range_start = FeelDateTime::local(year, month, day, hour, min, sec, nano).unwrap();
  let range_end = FeelDateTime::local(year, month, day, hour, min, sec + seconds, nano).unwrap();
  match rheon_feel_parser::parse_textual_expression(scope, expression, trace) {
    Ok(node) => {
      let evaluator = build_evaluator(&node);
      let value = evaluator(scope);
      let Value::DateTime(actual) = value else {
        panic!("expected date and time, actual type is {}", value.type_of());
      };
      assert!(
        (actual > range_start && actual < range_end),
        "ERROR\nexpected: {range_start}..{range_end}\n  actual: {actual}\n"
      );
    }
    Err(reason) => {
      panic!("parsing date and time expression failed with reason: {}", reason);
    }
  }
}
