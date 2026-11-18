//! The range functions on a range with an undefined endpoint.
//!
//! DMN 1.6 Table 42: a range written with one endpoint (`< 10`, `range("(..10)")`) has the other
//! endpoint undefined and not included. Table 79 gives each range function as a FEEL formula
//! over the endpoints of its arguments and whether they are included ("Evaluates to true if and
//! only if"). An undefined endpoint is null there (`(< 10).start`), so the formula is evaluated
//! with the three values of FEEL: an ordering comparison with an undefined endpoint is null, an
//! equality with it is true for two undefined endpoints and false otherwise, and `and`, `or` and
//! `not` are those of Table 51. The result is the value of the formula: true or false where the
//! endpoints that are defined decide it, and null with an error where they do not.

use crate::equality::{eval_ternary_equality, range_parts};
use crate::kernels::compare_lt;
use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::Value;

/// The range functions of Table 79.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relation {
  Before,
  After,
  Meets,
  MetBy,
  Overlaps,
  OverlapsBefore,
  OverlapsAfter,
  Finishes,
  FinishedBy,
  Includes,
  During,
  Starts,
  StartedBy,
  Coincides,
}

impl Relation {
  fn name(self) -> &'static str {
    match self {
      Relation::Before => "before",
      Relation::After => "after",
      Relation::Meets => "meets",
      Relation::MetBy => "met by",
      Relation::Overlaps => "overlaps",
      Relation::OverlapsBefore => "overlaps before",
      Relation::OverlapsAfter => "overlaps after",
      Relation::Finishes => "finishes",
      Relation::FinishedBy => "finished by",
      Relation::Includes => "includes",
      Relation::During => "during",
      Relation::Starts => "starts",
      Relation::StartedBy => "started by",
      Relation::Coincides => "coincides",
    }
  }
}

/// A truth value of FEEL: true, false, or null (None).
type Truth = Option<bool>;

fn and(a: Truth, b: Truth) -> Truth {
  match (a, b) {
    (Some(false), _) | (_, Some(false)) => Some(false),
    (Some(true), Some(true)) => Some(true),
    _ => None,
  }
}

fn or(a: Truth, b: Truth) -> Truth {
  match (a, b) {
    (Some(true), _) | (_, Some(true)) => Some(true),
    (Some(false), Some(false)) => Some(false),
    _ => None,
  }
}

/// An endpoint: a value, or None when it is undefined.
type Endpoint<'a> = Option<&'a Value>;

/// `a < b`: null with an undefined endpoint.
fn lt(a: Endpoint, b: Endpoint) -> Truth {
  match compare_lt(a?.clone(), b?.clone()) {
    Value::Boolean(result) => Some(result),
    _ => None,
  }
}

/// `a > b`.
fn gt(a: Endpoint, b: Endpoint) -> Truth {
  lt(b, a)
}

/// `a = b`: two undefined endpoints are equal, and an undefined one equals no value.
fn eq(a: Endpoint, b: Endpoint) -> Truth {
  match (a, b) {
    (None, None) => Some(true),
    (Some(a), Some(b)) => eval_ternary_equality(a, b),
    _ => Some(false),
  }
}

/// An argument of a range function: a point, or a range.
#[derive(Clone, Copy)]
enum Operand<'a> {
  Point(&'a Value),
  Range(Range<'a>),
}

/// The endpoints of a range and whether they are included.
#[derive(Clone, Copy)]
struct Range<'a> {
  start: Endpoint<'a>,
  start_in: bool,
  end: Endpoint<'a>,
  end_in: bool,
}

fn operand(value: &Value) -> Operand<'_> {
  match range_parts(value) {
    Some((start, start_in, end, end_in)) => Operand::Range(Range { start, start_in, end, end_in }),
    None => Operand::Point(value),
  }
}

/// The value of the range function `relation` on `value1` and `value2` when either is a range
/// with an undefined endpoint, and the values that are there are of one ordered type. None
/// otherwise: the function takes its arguments as for ranges with both endpoints.
pub(crate) fn relation(relation: Relation, value1: &Value, value2: &Value) -> Option<Value> {
  let (first, second) = (operand(value1), operand(value2));
  let open = |operand: &Operand| matches!(operand, Operand::Range(range) if range.start.is_none() || range.end.is_none());
  let named = if open(&first) {
    value1
  } else if open(&second) {
    value2
  } else {
    return None;
  };
  if !one_ordered_type(&[first, second]) {
    return None;
  }
  let truth = formula(relation, first, second)?;
  Some(match truth {
    Some(result) => Value::Boolean(result),
    None => value_null!(
      "[{}] expected endpoints that decide the result, found the range {} with an undefined endpoint",
      relation.name(),
      arg(named)
    ),
  })
}

/// Whether the points and the defined endpoints of `operands` are values of one ordered type.
fn one_ordered_type(operands: &[Operand]) -> bool {
  let mut kind = None;
  let mut values = operands.iter().flat_map(|operand| match operand {
    Operand::Point(point) => [Some(*point), None],
    Operand::Range(range) => [range.start, range.end],
  });
  values.all(|value| {
    let Some(value) = value else { return true };
    let ordered = matches!(
      value,
      Value::Number(_) | Value::String(_) | Value::Date(_) | Value::DateTime(_) | Value::Time(_) | Value::DaysAndTimeDuration(_) | Value::YearsAndMonthsDuration(_)
    );
    let this = std::mem::discriminant(value);
    ordered && *kind.get_or_insert(this) == this
  })
}

/// The formula of Table 79 for `relation` on its two arguments; None for a signature the
/// function does not have.
fn formula(relation: Relation, first: Operand, second: Operand) -> Option<Truth> {
  use Operand::{Point, Range};
  let point = |value| Some(value);
  Some(match (relation, first, second) {
    (Relation::Before, Point(p), Range(r)) => or(lt(point(p), r.start), and(eq(point(p), r.start), Some(!r.start_in))),
    (Relation::Before, Range(r), Point(p)) => or(lt(r.end, point(p)), and(eq(r.end, point(p)), Some(!r.end_in))),
    (Relation::Before, Range(a), Range(b)) => or(lt(a.end, b.start), and(Some(!a.end_in || !b.start_in), eq(a.end, b.start))),
    (Relation::After, Point(p), Range(r)) => or(gt(point(p), r.end), and(eq(point(p), r.end), Some(!r.end_in))),
    (Relation::After, Range(r), Point(p)) => or(gt(r.start, point(p)), and(eq(r.start, point(p)), Some(!r.start_in))),
    (Relation::After, Range(a), Range(b)) => or(gt(a.start, b.end), and(Some(!a.start_in || !b.end_in), eq(a.start, b.end))),
    (Relation::Meets, Range(a), Range(b)) => and(Some(a.end_in && b.start_in), eq(a.end, b.start)),
    (Relation::MetBy, Range(a), Range(b)) => and(Some(a.start_in && b.end_in), eq(a.start, b.end)),
    (Relation::Overlaps, Range(a), Range(b)) => and(
      or(gt(a.end, b.start), and(eq(a.end, b.start), Some(a.end_in && b.start_in))),
      or(lt(a.start, b.end), and(eq(a.start, b.end), Some(a.start_in && b.end_in))),
    ),
    (Relation::OverlapsBefore, Range(a), Range(b)) => overlaps_before(a, b),
    (Relation::OverlapsAfter, Range(a), Range(b)) => overlaps_before(b, a),
    (Relation::Finishes, Point(p), Range(r)) | (Relation::FinishedBy, Range(r), Point(p)) => and(Some(r.end_in), eq(r.end, point(p))),
    (Relation::Finishes, Range(a), Range(b)) => and(
      and(Some(a.end_in == b.end_in), eq(a.end, b.end)),
      or(gt(a.start, b.start), and(eq(a.start, b.start), Some(!a.start_in || b.start_in))),
    ),
    (Relation::FinishedBy, Range(a), Range(b)) => and(
      and(Some(a.end_in == b.end_in), eq(a.end, b.end)),
      or(lt(a.start, b.start), and(eq(a.start, b.start), Some(a.start_in || !b.start_in))),
    ),
    (Relation::Includes, Range(r), Point(p)) | (Relation::During, Point(p), Range(r)) => or(
      or(and(lt(r.start, point(p)), gt(r.end, point(p))), and(eq(r.start, point(p)), Some(r.start_in))),
      and(eq(r.end, point(p)), Some(r.end_in)),
    ),
    (Relation::Includes, Range(a), Range(b)) => includes(a, b),
    (Relation::During, Range(a), Range(b)) => includes(b, a),
    (Relation::Starts, Point(p), Range(r)) | (Relation::StartedBy, Range(r), Point(p)) => and(eq(r.start, point(p)), Some(r.start_in)),
    (Relation::Starts, Range(a), Range(b)) => starts(a, b),
    (Relation::StartedBy, Range(a), Range(b)) => starts(b, a),
    (Relation::Coincides, Range(a), Range(b)) => and(and(eq(a.start, b.start), Some(a.start_in == b.start_in)), and(eq(a.end, b.end), Some(a.end_in == b.end_in))),
    _ => return None,
  })
}

/// `overlaps before(a, b)`; `overlaps after(a, b)` is the same formula with the ranges exchanged.
fn overlaps_before(a: Range, b: Range) -> Truth {
  and(
    and(
      or(lt(a.start, b.start), and(eq(a.start, b.start), Some(a.start_in && !b.start_in))),
      or(gt(a.end, b.start), and(eq(a.end, b.start), Some(a.end_in && b.start_in))),
    ),
    or(lt(a.end, b.end), and(eq(a.end, b.end), Some(!a.end_in || b.end_in))),
  )
}

/// `includes(a, b)`; `during(a, b)` is the same formula with the ranges exchanged.
fn includes(a: Range, b: Range) -> Truth {
  and(
    or(lt(a.start, b.start), and(eq(a.start, b.start), Some(a.start_in || !b.start_in))),
    or(gt(a.end, b.end), and(eq(a.end, b.end), Some(a.end_in || !b.end_in))),
  )
}

/// `starts(a, b)`; `started by(a, b)` is the same formula with the ranges exchanged.
fn starts(a: Range, b: Range) -> Truth {
  and(
    and(eq(a.start, b.start), Some(a.start_in == b.start_in)),
    or(lt(a.end, b.end), and(eq(a.end, b.end), Some(!a.end_in || b.end_in))),
  )
}
