//! # Evaluator builders for FEEL expressions

use crate::FilterExpressionEvaluator;
use crate::bfeel::{self, Dialect};
use crate::equality::{Equality, eval_ternary_equality};
use crate::invocation::{Arguments, guarded, invoke};
use crate::iterations::{Domain, Iteration, Sequence};
use crate::macros::invalid_argument_type;
use rheon_feel::bif::Bif;
use rheon_feel::bip::*;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::arg;
use rheon_feel::values::{TypeOf, VALUE_FALSE, VALUE_TRUE, Value, truncated_fraction_warning};
use rheon_feel::{Evaluator, FeelNumber, FeelScope, FeelType, FunctionBody, FunctionDefinition, IntervalType, Name, value_null};
use rheon_feel_parser::{AstNode, ClosureBuilder};
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration, FeelZone};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::str::FromStr;
use std::sync::Arc;

/// Replaces non-finite numbers (decimal128 infinity or NaN, produced e.g. by `9e6144 * 10` or
/// `1 / 1e-6176`) with null: FEEL has no such numbers.
pub(crate) fn finite(value: Value) -> Value {
  match value {
    Value::Number(n) if !n.is_finite() => value_null!("result is not a finite number (decimal overflow)"),
    other => other,
  }
}

/// Wraps an arithmetic evaluator so that it never yields a non-finite number.
fn guard_finite(evaluator: Evaluator) -> Evaluator {
  Box::new(move |scope: &FeelScope| finite(evaluator(scope)))
}

/// Wraps a function invocation evaluator: the call depth limit, finite results and results nested
/// no deeper than values may nest ([guarded]).
fn guard_invocation(evaluator: Evaluator) -> Evaluator {
  Box::new(move |scope: &FeelScope| guarded(|| evaluator(scope)))
}

/// Builds an evaluator from provided AST node.
pub fn build_evaluator(node: &AstNode) -> Evaluator {
  let mut evaluator_builder = EvaluatorBuilder::default();
  evaluator_builder.build(node)
}

/// Builds an evaluator for the expression in the given dialect (FEEL or B-FEEL, DMN 1.6 chapter 11).
pub fn build_evaluator_with_dialect(node: &AstNode, dialect: Dialect) -> Evaluator {
  let mut evaluator_builder = EvaluatorBuilder { dialect };
  evaluator_builder.build(node)
}

/// Prepares null value with error message for second argument in `between` operator.
macro_rules! between_null2 {
  ($expected:literal, $actual:expr) => {
    value_null!(
      "expected {} as a second argument in 'between' operator, actual value type is {}",
      $expected,
      arg(&TypeOf($actual))
    )
  };
}

/// Prepares null value with error message for third argument in `between` operator.
macro_rules! between_null3 {
  ($expected:literal, $actual:expr) => {
    value_null!(
      "expected {} as a third argument in 'between' operator, actual value type is {}",
      $expected,
      arg(&TypeOf($actual))
    )
  };
}

/// Evaluator builder.
#[derive(Default)]
struct EvaluatorBuilder {
  /// FEEL or B-FEEL.
  dialect: Dialect,
}

impl EvaluatorBuilder {
  /// Builds and evaluator based on provided AST node.
  fn build(&mut self, node: &AstNode) -> Evaluator {
    // B-FEEL: arithmetic converts mismatched operands, boolean operators never return null
    if self.dialect == Dialect::BFeel {
      let operator = match node {
        AstNode::Add(lhs, rhs) => Some((bfeel::Operator::Add, lhs, rhs)),
        AstNode::Sub(lhs, rhs) => Some((bfeel::Operator::Sub, lhs, rhs)),
        AstNode::Mul(lhs, rhs) => Some((bfeel::Operator::Mul, lhs, rhs)),
        AstNode::Div(lhs, rhs) => Some((bfeel::Operator::Div, lhs, rhs)),
        AstNode::Exp(lhs, rhs) => Some((bfeel::Operator::Exp, lhs, rhs)),
        _ => None,
      };
      if let Some((operator, lhs, rhs)) = operator {
        let lhe = self.build(lhs);
        let rhe = self.build(rhs);
        // the FEEL operator applied to the operand values; built by this builder in the FEEL dialect, so
        // it already carries the non-finite guard added at the end of `build` (the early return below
        // skips that guard only for the B-FEEL wrapper, whose results all come from the FEEL operator)
        let feel_operator = bfeel::feel_operator_evaluator(operator);
        return Box::new(move |scope: &FeelScope| bfeel::arithmetic(operator, &feel_operator, scope, lhe(scope), rhe(scope)));
      }
    }
    // build the evaluator
    let evaluator = match node {
      AstNode::Add(lhs, rhs) => self.build_add(lhs, rhs),
      AstNode::And(lhs, rhs) => self.build_and(lhs, rhs),
      AstNode::At(rhs) => self.build_at(rhs),
      AstNode::Between(lhs, mhs, rhs) => self.build_between(lhs, mhs, rhs),
      AstNode::Boolean(lhs) => self.build_boolean(*lhs),
      AstNode::Context(lhs) => self.build_context(lhs),
      AstNode::ContextEntry(lhs, rhs) => self.build_context_entry(lhs, rhs),
      AstNode::ContextEntryKey(lhs) => self.build_context_entry_key(lhs),
      AstNode::ContextType(lhs) => self.build_context_type(lhs),
      AstNode::ContextTypeEntry(lhs, rhs) => self.build_context_type_entry(lhs, rhs),
      AstNode::ContextTypeEntryKey(lhs) => self.build_context_type_entry_key(lhs),
      AstNode::Div(lhs, rhs) => self.build_div(lhs, rhs),
      AstNode::Eq(lhs, rhs) => self.build_eq(lhs, rhs),
      AstNode::EvaluatedExpression(lhs) => self.build_evaluated_expression(lhs),
      AstNode::Every(lhs, rhs) => self.build_every(lhs, rhs),
      AstNode::Exp(lhs, rhs) => self.build_exp(lhs, rhs),
      AstNode::ExpressionList(lhs) => self.build_expression_list(lhs),
      AstNode::FeelType(lhs) => self.build_feel_type(lhs),
      AstNode::Filter(lhs, rhs) => self.build_filter(lhs, rhs),
      AstNode::For(lhs, rhs) => self.build_for(lhs, rhs),
      AstNode::FormalParameter(lhs, rhs) => self.build_formal_parameter(lhs, rhs),
      AstNode::FormalParameters(lhs) => self.build_formal_parameters(lhs),
      AstNode::FunctionBody(lhs, rhs) => self.build_function_body(lhs, rhs),
      AstNode::FunctionDefinition(lhs, rhs) => self.build_function_definition(lhs, rhs),
      AstNode::FunctionInvocation(lhs, rhs) => self.build_function_invocation(lhs, rhs),
      AstNode::FunctionType(lhs, rhs) => self.build_function_type(lhs, rhs),
      AstNode::Ge(lhs, rhs) => self.build_ge(lhs, rhs),
      AstNode::Gt(lhs, rhs) => self.build_gt(lhs, rhs),
      AstNode::If(lhs, mid, rhs) => self.build_if(lhs, mid, rhs),
      AstNode::In(lhs, rhs) => self.build_in(lhs, rhs),
      AstNode::InstanceOf(lhs, rhs) => self.build_instance_of(lhs, rhs),
      AstNode::IntervalEnd(lhs, rhs) => self.build_interval_end(lhs, rhs),
      AstNode::IntervalStart(lhs, rhs) => self.build_interval_start(lhs, rhs),
      AstNode::Irrelevant => self.build_irrelevant(),
      AstNode::Le(lhs, rhs) => self.build_le(lhs, rhs),
      AstNode::List(items) => self.build_list(items),
      AstNode::ListType(lhs) => self.build_list_type(lhs),
      AstNode::Lt(lhs, rhs) => self.build_lt(lhs, rhs),
      AstNode::Mul(lhs, rhs) => self.build_mul(lhs, rhs),
      AstNode::Name(name) => self.build_name(name.clone()),
      AstNode::Neg(rhs) => self.build_neg(rhs),
      AstNode::NegatedList(lhs) => self.build_negated_list(lhs),
      AstNode::Null => self.build_null(),
      AstNode::Numeric(before, after, sign, exponent) => self.build_number(before, after, *sign, exponent),
      AstNode::Nq(lhs, rhs) => self.build_nq(lhs, rhs),
      AstNode::Or(lhs, rhs) => self.build_or(lhs, rhs),
      AstNode::Out(lhs, rhs) => self.build_out(lhs, rhs),
      AstNode::ParameterName(lhs) => self.build_parameter_name(lhs),
      AstNode::ParameterTypes(lhs) => self.build_parameter_types(lhs),
      AstNode::Descendant(lhs, rhs) => self.build_descendant(lhs, rhs),
      AstNode::Path(lhs, rhs) => self.build_path(lhs, rhs),
      AstNode::QualifiedName(lhs) => self.build_qualified_name(lhs),
      AstNode::QualifiedNameSegment(lhs) => self.build_qualified_name_segment(lhs),
      AstNode::Range(lhs, rhs) => self.build_range(lhs, rhs),
      AstNode::RangeType(lhs) => self.build_range_type(lhs),
      AstNode::Some(lhs, rhs) => self.build_some(lhs, rhs),
      AstNode::String(lhs) => self.build_string(lhs),
      AstNode::Sub(lhs, rhs) => self.build_sub(lhs, rhs),
      AstNode::UnaryGe(lhs) => self.build_unary_ge(lhs),
      AstNode::UnaryGt(lhs) => self.build_unary_gt(lhs),
      AstNode::UnaryLe(lhs) => self.build_unary_le(lhs),
      AstNode::UnaryLt(lhs) => self.build_unary_lt(lhs),
      AstNode::UnaryEq(lhs) => self.build_unary_eq(lhs),
      AstNode::UnaryNe(lhs) => self.build_unary_ne(lhs),
      AstNode::CommaList { .. }
      | AstNode::IterationContexts { .. }
      | AstNode::IterationContextSingle { .. }
      | AstNode::IterationContextInterval { .. }
      | AstNode::NamedParameter { .. }
      | AstNode::NamedParameters { .. }
      | AstNode::PositionalParameters { .. }
      | AstNode::QuantifiedContext { .. }
      | AstNode::QuantifiedContexts { .. }
      | AstNode::Satisfies { .. } => build_err_msg(err_msg_unexpected_node(node)),
    };
    let evaluator = if self.dialect == Dialect::BFeel {
      match node {
        AstNode::Eq(..) | AstNode::Lt(..) | AstNode::Le(..) | AstNode::Gt(..) | AstNode::Ge(..) | AstNode::And(..) | AstNode::Or(..) | AstNode::In(..) | AstNode::Between(..) => {
          Box::new(move |scope: &FeelScope| bfeel::boolean_result(evaluator(scope), false)) as Evaluator
        }
        AstNode::Nq(..) => Box::new(move |scope: &FeelScope| bfeel::boolean_result(evaluator(scope), true)) as Evaluator,
        _ => evaluator,
      }
    } else {
      evaluator
    };
    // arithmetic and function calls must never yield non-finite numbers,
    // and function calls are subject to the call depth limit
    let evaluator = match node {
      AstNode::Add(..) | AstNode::Sub(..) | AstNode::Mul(..) | AstNode::Div(..) | AstNode::Neg(..) => guard_finite(evaluator),
      AstNode::FunctionInvocation(..) => guard_invocation(evaluator),
      _ => evaluator,
    };
    // return the evaluator
    evaluator
  }

  /// Returns an evaluator for `addition`.
  fn build_add(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    arithmetic(Arithmetic::Addition, lhe, rhe, |lhv, rhv| match lhv {
      Value::Number(lh) => match rhv {
        Value::Number(rh) => Value::Number(lh + rh),
        _ => value_null!(
          "incompatible types in addition: {}({}) + {}({})",
          arg(&lhv),
          arg(&TypeOf(&lhv)),
          arg(&rhv),
          arg(&TypeOf(&rhv))
        ),
      },
      Value::String(lh) => {
        if let Value::String(rh) = rhv {
          Value::String(lh.concat(&rh))
        } else {
          value_null!("expected string as a second argument in addition")
        }
      }
      Value::Date(lh) => match rhv {
        Value::DaysAndTimeDuration(rh) => {
          if let Some(result) = lh + rh {
            Value::Date(result)
          } else {
            value_null!("invalid result while adding days and time duration to date")
          }
        }
        Value::YearsAndMonthsDuration(rh) => {
          if let Some(a) = lh + rh {
            Value::Date(a)
          } else {
            value_null!("invalid result while adding years and months duration to date")
          }
        }
        other => invalid_argument_type!("add", "years and months duration", other),
      },
      Value::DateTime(lh) => match rhv {
        Value::DaysAndTimeDuration(rh) => {
          if let Some(a) = lh + rh {
            Value::DateTime(a)
          } else {
            value_null!("invalid result while adding days and time duration to date and time")
          }
        }
        Value::YearsAndMonthsDuration(rh) => {
          if let Some(a) = lh + rh {
            Value::DateTime(a)
          } else {
            value_null!("invalid result while adding years and months duration to date and time")
          }
        }
        other => invalid_argument_type!("add", "days and time duration, years and months duration", other),
      },
      Value::Time(lh) => match rhv {
        Value::DaysAndTimeDuration(rh) => Value::Time(lh + rh),
        other => invalid_argument_type!("add", "days and time duration", other),
      },
      Value::DaysAndTimeDuration(lh) => match rhv {
        Value::DaysAndTimeDuration(rh) => lh
          .checked_add(&rh)
          .map_or_else(|| value_null!("days and time duration addition overflow"), Value::DaysAndTimeDuration),
        Value::Date(rh) => {
          if let Some(result) = rh + lh {
            Value::Date(result)
          } else {
            value_null!("invalid result while adding date to days and time duration")
          }
        }
        Value::DateTime(rh) => {
          if let Some(a) = rh + lh {
            Value::DateTime(a)
          } else {
            value_null!("invalid result while adding date and time to days and time duration")
          }
        }
        Value::Time(rh) => Value::Time(rh + lh),
        other => invalid_argument_type!("add", "days and time duration, date and time", other),
      },
      Value::YearsAndMonthsDuration(lh) => match rhv {
        Value::Date(rh) => {
          if let Some(a) = rh + lh {
            Value::Date(a)
          } else {
            value_null!("invalid result while adding date to years and months duration")
          }
        }
        Value::DateTime(rh) => {
          if let Some(a) = rh + lh {
            Value::DateTime(a)
          } else {
            value_null!("invalid result while adding date and time to years and months duration")
          }
        }
        Value::YearsAndMonthsDuration(rh) => lh
          .checked_add(&rh)
          .map_or_else(|| value_null!("years and months duration addition overflow"), Value::YearsAndMonthsDuration),
        other => invalid_argument_type!("add", "years and months duration, date and time", other),
      },
      other => invalid_argument_type!("add", "number, string, date and time, days and time duration, years and months duration, null", other),
    })
  }

  /// Returns an evaluator of the temporal expression after `@` (at) literal.
  /// Builds an `@` literal. Its value is parsed once, here; a literal whose fraction digits
  /// beyond nanoseconds were truncated warns on each evaluation, since diagnostics are recorded
  /// while an evaluation runs (a model built once reports in every evaluation).
  fn build_at(&mut self, text: &str) -> Evaluator {
    if let Ok(date) = FeelDate::from_str(text) {
      Box::new(move |_: &FeelScope| Value::Date(date.clone()))
    } else if let Ok((date_time, dropped)) = FeelDateTime::parse_reporting_truncation(text) {
      truncating_at_literal(Value::DateTime(date_time), text, dropped)
    } else if let Ok((time, dropped)) = FeelTime::parse_reporting_truncation(text) {
      truncating_at_literal(Value::Time(time), text, dropped)
    } else if let Ok(ym_duration) = FeelYearsAndMonthsDuration::try_from(text) {
      Box::new(move |_: &FeelScope| Value::YearsAndMonthsDuration(ym_duration.clone()))
    } else if let Ok((dt_duration, dropped)) = FeelDaysAndTimeDuration::parse_reporting_truncation(text) {
      truncating_at_literal(Value::DaysAndTimeDuration(dt_duration), text, dropped)
    } else {
      let invalid_at_literal = format!("invalid @ literal: {}", arg(text));
      Box::new(move |_: &FeelScope| value_null!(invalid_at_literal))
    }
  }

  fn build_between(&mut self, lhs: &AstNode, mhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let mhe = self.build(mhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let mhv = mhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Number(lh) => {
          if let Value::Number(mh) = mhv {
            if let Value::Number(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("number", &rhv)
            }
          } else {
            between_null2!("number", &mhv)
          }
        }
        Value::String(lh) => {
          if let Value::String(mh) = mhv {
            if let Value::String(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("string", &rhv)
            }
          } else {
            between_null2!("string", &mhv)
          }
        }
        Value::Date(lh) => {
          if let Value::Date(mh) = mhv {
            if let Value::Date(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("date", &rhv)
            }
          } else {
            between_null2!("date", &mhv)
          }
        }
        Value::Time(lh) => {
          if let Value::Time(mh) = mhv {
            if let Value::Time(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("time", &rhv)
            }
          } else {
            between_null2!("time", &mhv)
          }
        }
        Value::DateTime(lh) => {
          if let Value::DateTime(mh) = mhv {
            if let Value::DateTime(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("date and time", &rhv)
            }
          } else {
            between_null2!("date and time", &mhv)
          }
        }
        Value::DaysAndTimeDuration(lh) => {
          if let Value::DaysAndTimeDuration(mh) = mhv {
            if let Value::DaysAndTimeDuration(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("days and time duration", &rhv)
            }
          } else {
            between_null2!("days and time duration", &mhv)
          }
        }
        Value::YearsAndMonthsDuration(lh) => {
          if let Value::YearsAndMonthsDuration(mh) = mhv {
            if let Value::YearsAndMonthsDuration(rh) = rhv {
              Value::Boolean(mh <= lh && lh <= rh)
            } else {
              between_null3!("years and months duration", &rhv)
            }
          } else {
            between_null2!("years and months duration", &mhv)
          }
        }
        other => {
          if other.is_null() {
            other
          } else {
            value_null!("unexpected value type in 'between' operator: {}", arg(&TypeOf(&other)))
          }
        }
      }
    })
  }

  fn build_boolean(&mut self, lhs: bool) -> Evaluator {
    Box::new(move |_: &FeelScope| Value::Boolean(lhs))
  }

  /// Returns an evaluator for `conjunction`.
  ///
  /// ```text
  /// left        right        result
  ///─────────────────────────────────
  /// true        true         true
  /// true        false        false
  /// true        otherwise    null
  /// false       true         false
  /// false       false        false
  /// false       otherwise    false
  /// otherwise   true         null
  /// otherwise   false        false
  /// otherwise   otherwise    null
  /// ```
  fn build_and(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Boolean(lh) => match rhv {
          Value::Boolean(rh) => Value::Boolean(lh && rh),
          _ => {
            if lh {
              value_null!()
            } else {
              Value::Boolean(false)
            }
          }
        },
        _ => match rhv {
          Value::Boolean(rh) => {
            if rh {
              value_null!()
            } else {
              Value::Boolean(false)
            }
          }
          _ => value_null!(),
        },
      }
    })
  }

  fn build_context(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut evaluated_ctx = FeelContext::default();
      // prepare special context in scope, used for already evaluated context entries
      let depth = scope.depth();
      scope.push(FeelContext::default());
      // evaluate context entries
      let failure = 'entries: {
        for evaluator in &evaluators {
          match evaluator(scope) {
            Value::ContextEntry(name, value) => {
              if evaluated_ctx.contains_entry(&name) {
                // duplicated context entry keys are not allowed
                break 'entries Some(value_null!("duplicated context entry key: {}", arg(&name)));
              } else {
                let value = diagnostics::bound_depth(*value);
                // add newly evaluated entry to evaluated context
                evaluated_ctx.set_entry(&name, value.clone());
                // add newly evaluated entry to special context
                scope.set_value(&name, value);
              }
            }
            value_null @ Value::Null(_) => break 'entries Some(value_null),
            other => break 'entries Some(value_null!("expected context entry, actual value type is {}", arg(&TypeOf(&other)))),
          }
        }
        None
      };
      // remove special context from scope, on every path
      scope.truncate(depth);
      failure.unwrap_or_else(|| diagnostics::bound_depth(evaluated_ctx.into()))
    })
  }

  fn build_context_entry(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::ContextEntryKey(name) => Value::ContextEntry(name, Box::new(rhv)),
        _ => value_null!("expected context entry key, actual value type is {}", arg(&TypeOf(&lhv))),
      }
    })
  }

  fn build_context_entry_key(&mut self, lhs: &Name) -> Evaluator {
    let name = lhs.clone();
    Box::new(move |_: &FeelScope| Value::ContextEntryKey(name.clone()))
  }

  fn build_context_type(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut entries = BTreeMap::new();
      for evaluator in &evaluators {
        match evaluator(scope) {
          Value::ContextTypeEntry(name, feel_type) => {
            entries.insert(name, feel_type.clone());
          }
          null @ Value::Null(_) => return null,
          other => return value_null!("expected context type entry, actual value type is {}", arg(&TypeOf(&other))),
        }
      }
      Value::FeelType(FeelType::Context(entries))
    })
  }

  fn build_context_type_entry(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      if let Value::ContextTypeEntryKey(name) = lhv {
        if let Value::FeelType(feel_type) = rhv {
          Value::ContextTypeEntry(name, feel_type)
        } else {
          value_null!("expected a type in context type entry")
        }
      } else {
        value_null!("expected a name in context type entry")
      }
    })
  }

  fn build_context_type_entry_key(&mut self, lhs: &Name) -> Evaluator {
    let name = lhs.clone();
    Box::new(move |_: &FeelScope| Value::ContextTypeEntryKey(name.clone()))
  }

  fn build_div(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    arithmetic(Arithmetic::Division, lhe, rhe, |lhv, rhv| match lhv {
      Value::Number(lh) => match rhv {
        Value::Number(rh) => {
          if rh.abs() == FeelNumber::zero() {
            value_null!("[division] division by zero")
          } else {
            Value::Number(lh / rh)
          }
        }
        _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
      },
      Value::DaysAndTimeDuration(ref lh) => match rhv {
        Value::Number(rh) => {
          if rh.is_zero() {
            value_null!("[division] division by zero")
          } else {
            let lv = FeelNumber::from(lh.as_nanos()) / rh;
            if let Ok(v) = FeelNumber::try_into(lv)
              && let Some(duration) = FeelDaysAndTimeDuration::from_n(v)
            {
              Value::DaysAndTimeDuration(duration)
            } else {
              value_null!("[division] error: {} / {}", arg(&lhv), arg(&rhv))
            }
          }
        }
        Value::DaysAndTimeDuration(rh) => {
          if rh.as_nanos() == 0 {
            value_null!("[division] division by zero")
          } else {
            let lvl = FeelNumber::from(lh.as_nanos());
            let rvl = FeelNumber::from(rh.as_nanos());
            Value::Number(lvl / rvl)
          }
        }
        _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
      },
      Value::YearsAndMonthsDuration(ref lh) => match rhv {
        Value::Number(rh) => {
          if rh.is_zero() {
            value_null!("[division] division by zero")
          } else {
            match temporal_value::ym_duration(FeelNumber::from(lh.as_months()) / rh, &lhv, '/', &rhv) {
              Some(duration) => Value::YearsAndMonthsDuration(duration),
              None => value_null!("[division] error: {} / {}", arg(&lhv), arg(&rhv)),
            }
          }
        }
        Value::YearsAndMonthsDuration(rh) => {
          if rh.as_months() == 0 {
            value_null!("[division] division by zero")
          } else {
            let lvl = FeelNumber::from(lh.as_months());
            let rvl = FeelNumber::from(rh.as_months());
            Value::Number(lvl / rvl)
          }
        }
        _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
      },
      _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
    })
  }

  fn build_expression_list(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut values = vec![];
      for evaluator in &evaluators {
        values.push(evaluator(scope))
      }
      Value::ExpressionList(values)
    })
  }

  fn build_exp(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    arithmetic(Arithmetic::Exponentiation, lhe, rhe, |lhv, rhv| {
      if let Value::Number(lh) = lhv {
        if let Value::Number(rh) = rhv {
          // DMN 1.6 Table 61: "value(e2) is a number in the range [-999,999,999..999,999,999]",
          // checked before any power, also where the power would not depend on the exponent
          if rh.abs() > FeelNumber::from(999_999_999) {
            value_null!("exponentiation exponent is out of range -999999999..999999999")
          } else if let Some(result) = lh.pow(&rh) {
            Value::Number(result)
          } else {
            value_null!("exponentiation result is not a finite number")
          }
        } else {
          value_null!("exponentiation exponent is not a number")
        }
      } else {
        value_null!("exponentiation base is not a number")
      }
    })
  }

  fn build_feel_type(&mut self, lhs: &FeelType) -> Evaluator {
    let feel_type = lhs.clone();
    Box::new(move |_: &FeelScope| Value::FeelType(feel_type.clone()))
  }

  fn build_filter(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let filter_expression_evaluator = FilterExpressionEvaluator::for_feel();
      filter_expression_evaluator.evaluate(scope, lhe(scope), &rhe)
    })
  }

  fn build_for(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let mut variables = vec![];
    if let AstNode::IterationContexts(items) = lhs {
      for item in items {
        match item {
          AstNode::IterationContextInterval(variable_name, start_node, end_node) => {
            if let AstNode::Name(name) = variable_name.borrow() {
              let start_evaluator = self.build(start_node);
              let end_evaluator = self.build(end_node);
              let domain: Domain = Box::new(move |scope: &FeelScope| Sequence::interval(start_evaluator(scope), end_evaluator(scope)));
              variables.push((name.clone(), domain));
            }
          }
          AstNode::IterationContextSingle(variable_name, expr_node) => {
            if let AstNode::Name(name) = variable_name.borrow() {
              variables.push((name.clone(), self.build_domain(expr_node, true)));
            }
          }
          _ => {}
        }
      }
    }
    let iteration = match iteration(variables) {
      Ok(iteration) => iteration,
      Err(evaluator) => return evaluator,
    };
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| iteration.collect(scope, &rhe))
  }

  /// The domain of an iteration variable: the items of the value of `node`. In a `for`
  /// expression (`range_literal`) a range literal `[a..b]` is the numbers or dates it contains.
  fn build_domain(&mut self, node: &AstNode, range_literal: bool) -> Domain {
    match node {
      AstNode::Range(start_node, end_node) if range_literal => {
        let start_evaluator = self.build(start_node);
        let end_evaluator = self.build(end_node);
        Box::new(move |scope: &FeelScope| Sequence::range(start_evaluator(scope), end_evaluator(scope)))
      }
      _ => {
        let evaluator = self.build(node);
        Box::new(move |scope: &FeelScope| Ok(Sequence::of(evaluator(scope))))
      }
    }
  }

  /// The iteration of a `some` or an `every` expression: its quantified contexts, in order.
  fn build_quantified_contexts(&mut self, lhs: &AstNode) -> std::result::Result<Iteration, Evaluator> {
    let AstNode::QuantifiedContexts(items) = lhs else {
      return Err(build_err_msg(err_msg_expected_node("QuantifiedContexts", lhs)));
    };
    let mut variables = vec![];
    for item in items {
      if let AstNode::QuantifiedContext(variable_name, expr_node) = item
        && let AstNode::Name(name) = variable_name.borrow()
      {
        variables.push((name.clone(), self.build_domain(expr_node, false)));
      }
    }
    iteration(variables)
  }

  fn build_formal_parameter(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      if let Value::ParameterName(parameter_name) = lhv {
        if let Value::FeelType(parameter_type) = rhv {
          Value::FormalParameter(parameter_name, parameter_type)
        } else {
          value_null!("expected type of the formal parameter")
        }
      } else {
        value_null!("expected name of the formal parameter")
      }
    })
  }

  fn build_formal_parameters(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut formal_parameters = vec![];
      for evaluator in &evaluators {
        match evaluator(scope) {
          Value::FormalParameter(parameter_name, parameter_type) => {
            formal_parameters.push((parameter_name, parameter_type));
          }
          null @ Value::Null(_) => return null,
          other => return value_null!("expected formal parameter, actual value type is: {}", arg(&TypeOf(&other))),
        }
      }
      Value::FormalParameters(formal_parameters)
    })
  }

  fn build_function_body(&mut self, lhs: &AstNode, rhs: &bool) -> Evaluator {
    if *rhs {
      self.build_external_function_body(lhs)
    } else {
      self.build_internal_function_body(lhs)
    }
  }

  fn build_internal_function_body(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = Arc::new(self.build(lhs));
    Box::new(move |_: &FeelScope| Value::FunctionBody(FunctionBody::LiteralExpression(lhe.clone()), false))
  }

  fn build_external_function_body(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| {
      let mapping_value = lhe(scope);
      match mapping_value {
        Value::Context(mapping_information) => {
          if let Some(Value::Context(java_mapping)) = mapping_information.get_entry(&"java".into()) {
            return if let Some(Value::String(class_name)) = java_mapping.get_entry(&"class".into()) {
              if let Some(Value::String(method_signature)) = java_mapping.get_entry(&"method signature".into()) {
                let java_class_name = class_name.as_str().to_owned();
                let java_method_signature = method_signature.as_str().to_owned();
                let java_evaluator = Box::new(move |_: &FeelScope| Value::ExternalJavaFunction(java_class_name.clone(), java_method_signature.clone())) as Evaluator;
                let lhe = Arc::new(java_evaluator);
                Value::FunctionBody(FunctionBody::External(lhe), true)
              } else {
                value_null!("invalid Java function mapping, no method signature entry in context {}", arg(&java_mapping))
              }
            } else {
              value_null!("invalid Java function mapping, no class name entry in context {}", arg(&java_mapping))
            };
          }
          if let Some(Value::Context(pmml_mapping)) = mapping_information.get_entry(&"pmml".into()) {
            return if let Some(Value::String(document)) = pmml_mapping.get_entry(&"document".into()) {
              if let Some(Value::String(model_name)) = pmml_mapping.get_entry(&"model".into()) {
                let pmml_document = document.as_str().to_owned();
                let pmml_model_name = model_name.as_str().to_owned();
                let pmml_evaluator = Box::new(move |_: &FeelScope| Value::ExternalPmmlFunction(pmml_document.clone(), pmml_model_name.clone())) as Evaluator;
                let lhe = Arc::new(pmml_evaluator);
                Value::FunctionBody(FunctionBody::External(lhe), true)
              } else {
                value_null!("invalid PMML function mapping, no model name entry in context {}", arg(&pmml_mapping))
              }
            } else {
              value_null!("invalid PMML function mapping, no document entry in context {}", arg(&pmml_mapping))
            };
          }
          value_null!(
            "invalid external function mapping, expected 'java' or 'pmml' entry in context {}",
            arg(&mapping_information)
          )
        }
        other => value_null!("invalid external function mapping, expected context, actual value is {}", arg(&other)),
      }
    })
  }

  fn build_function_definition(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let closure = ClosureBuilder::from_function_definition(lhs, rhs);
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::FormalParameters(parameters) => {
          if let Value::FunctionBody(body, external) = rhv {
            // evaluate closure context
            let mut closure_ctx = FeelContext::default();
            for closure_name in closure.iter() {
              if let Some(closure_value) = scope.search_entry(closure_name) {
                closure_ctx.create_entry(closure_name, closure_value);
              }
            }
            //TODO Check if `FeelType::Any` is always ok for function result type in function definition.
            diagnostics::bound_depth(FunctionDefinition::new(parameters, body, external, closure.clone(), closure_ctx, FeelType::Any).into())
          } else {
            value_null!("invalid body in function definition")
          }
        }
        null @ Value::Null(_) => null,
        other => value_null!("expected formal parameters, actual value type is {}", arg(&TypeOf(&other))),
      }
    })
  }

  fn build_eq(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| equal(&lhe(scope), &rhe(scope)))
  }

  fn build_evaluated_expression(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| lhe(scope))
  }

  fn build_every(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    self.build_quantified(lhs, rhs, Iteration::every)
  }

  /// Builds a `some` or an `every` expression: the iteration contexts of `lhs`, the `satisfies`
  /// body of `rhs`, and the `quantifier` that evaluates the body over the contexts.
  fn build_quantified(&mut self, lhs: &AstNode, rhs: &AstNode, quantifier: fn(&Iteration, &FeelScope, &Evaluator) -> Value) -> Evaluator {
    let iteration = match self.build_quantified_contexts(lhs) {
      Ok(iteration) => iteration,
      Err(evaluator) => return evaluator,
    };
    let AstNode::Satisfies(satisfies) = rhs else {
      return build_err_msg(err_msg_expected_node("Satisfies", rhs));
    };
    let satisfies_evaluator = self.build(satisfies);
    Box::new(move |scope: &FeelScope| quantifier(&iteration, scope, &satisfies_evaluator))
  }

  fn build_function_invocation(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    match rhs {
      AstNode::PositionalParameters(parameters) => self.build_function_invocation_with_positional_parameters(lhs, parameters),
      AstNode::NamedParameters(parameters) => self.build_function_invocation_with_named_parameters(lhs, parameters),
      _ => build_err_msg(err_msg_expected_positional_or_named_parameter(rhs)),
    }
  }

  fn build_function_invocation_with_positional_parameters(&mut self, lhs: &AstNode, rhs: &[AstNode]) -> Evaluator {
    let function_evaluator = self.build(lhs);
    let argument_evaluators = self.build_positional_argument_evaluators(lhs, rhs);
    let dialect = self.dialect;
    Box::new(move |scope: &FeelScope| {
      let function = function_evaluator(scope);
      let arguments = argument_evaluators.iter().map(|evaluator| evaluator(scope)).collect();
      invoke(scope, function, Arguments::Positional(arguments), dialect)
    })
  }

  fn build_positional_argument_evaluators(&mut self, lhs: &AstNode, rhs: &[AstNode]) -> Vec<Evaluator> {
    if let AstNode::Name(name) = lhs {
      match name.to_string().as_str() {
        "range" => {
          //------------------------------------------------------------------------------------------------------------
          // Tweak with the `range` function: range value is pre-parsed?
          // Left for performance optimization stage.
          //------------------------------------------------------------------------------------------------------------
        }
        "matches" => {
          //------------------------------------------------------------------------------------------------------------
          // Tweak with `matches` function: regular expressions are pre-parsed?
          // Left for performance optimization stage.
          //------------------------------------------------------------------------------------------------------------
        }
        "replace" => {
          //------------------------------------------------------------------------------------------------------------
          // Tweak with `replace` function: regular expressions are pre-parsed?
          // Left for performance optimization stage.
          //------------------------------------------------------------------------------------------------------------
        }
        _ => {}
      }
    }
    //------------------------------------------------------------------------------------------------------------------
    // Default argument processing.
    //------------------------------------------------------------------------------------------------------------------
    let mut argument_evaluators = vec![];
    for node in rhs {
      argument_evaluators.push(self.build(node));
    }
    argument_evaluators
  }

  fn build_function_invocation_with_named_parameters(&mut self, lhs: &AstNode, rhs: &[AstNode]) -> Evaluator {
    let function_evaluator = self.build(lhs);
    let argument_evaluators = match self.build_named_argument_evaluators(lhs, rhs) {
      Ok(argument_evaluators) => argument_evaluators,
      Err(err_msg) => return build_err_msg(err_msg),
    };
    let dialect = self.dialect;
    Box::new(move |scope: &FeelScope| {
      let function = function_evaluator(scope);
      let arguments = argument_evaluators.iter().map(|(name, evaluator)| (name.clone(), evaluator(scope))).collect();
      invoke(scope, function, Arguments::Named(arguments), dialect)
    })
  }

  fn build_named_argument_evaluators(&mut self, lhs: &AstNode, rhs: &[AstNode]) -> Result<Vec<(Name, Evaluator)>, String> {
    if let AstNode::Name(name) = lhs {
      match name.to_string().as_str() {
        "range" => {
          //------------------------------------------------------------------------------------------------------------
          // Tweak with the `range` function: range value is pre-parsed?
          // Left for performance optimization stage.
          //------------------------------------------------------------------------------------------------------------
        }
        "matches" => {
          //------------------------------------------------------------------------------------------------------------
          // Tweak with `matches` function: regular expressions are pre-parsed?
          // Left for performance optimization stage.
          //------------------------------------------------------------------------------------------------------------
        }
        "replace" => {
          //------------------------------------------------------------------------------------------------------------
          // Tweak with `replace` function: regular expressions are pre-parsed?
          // Left for performance optimization stage.
          //------------------------------------------------------------------------------------------------------------
        }
        _ => {}
      }
    }
    //------------------------------------------------------------------------------------------------------------------
    // Default argument processing.
    //------------------------------------------------------------------------------------------------------------------
    let mut argument_evaluators = vec![];
    for node in rhs {
      let AstNode::NamedParameter(name_node, expression) = node else {
        return Err(err_msg_expected_node("NamedParameter", node));
      };
      let AstNode::ParameterName(name) = name_node.borrow() else {
        return Err(err_expected_parameter_name(name_node));
      };
      argument_evaluators.push((name.clone(), self.build(expression)));
    }
    Ok(argument_evaluators)
  }

  fn build_function_type(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      if let Value::ParameterTypes(types) = lhv {
        if let Value::FeelType(result_type) = rhv {
          let parameter_types = types
            .iter()
            .filter_map(|value| if let Value::FeelType(feel_type) = value { Some(feel_type.clone()) } else { None })
            .collect();
          Value::FeelType(FeelType::Function(parameter_types, Box::new(result_type)))
        } else {
          value_null!("expected function's result type")
        }
      } else {
        value_null!("expected function's parameter types")
      }
    })
  }

  fn build_ge(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Number(lh) => match rhv {
          Value::Number(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_number"),
        },
        Value::String(lh) => match rhv {
          Value::String(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_string"),
        },
        Value::Date(lh) => match rhv {
          Value::Date(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_date"),
        },
        Value::DateTime(lh) => match rhv {
          Value::DateTime(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_date_time"),
        },
        Value::Time(lh) => match rhv {
          Value::Time(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_time"),
        },
        Value::DaysAndTimeDuration(lh) => match rhv {
          Value::DaysAndTimeDuration(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_days_and_time_duration"),
        },
        Value::YearsAndMonthsDuration(lh) => match rhv {
          Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh >= rh),
          _ => value_null!("eval_greater_or_equal_years_and_months_duration"),
        },
        _ => value_null!("eval_greater_or_equal"),
      }
    })
  }

  fn build_gt(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Number(lh) => match rhv {
          Value::Number(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_then_number"),
        },
        Value::String(lh) => match rhv {
          Value::String(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_then_string"),
        },
        Value::Date(lh) => match rhv {
          Value::Date(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_then_date"),
        },
        Value::DateTime(lh) => match rhv {
          Value::DateTime(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_then_date_time"),
        },
        Value::Time(lh) => match rhv {
          Value::Time(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_then_time"),
        },
        Value::DaysAndTimeDuration(lh) => match rhv {
          Value::DaysAndTimeDuration(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_days_and_time_duration"),
        },
        Value::YearsAndMonthsDuration(lh) => match rhv {
          Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh > rh),
          _ => value_null!("eval_greater_years_and_months_duration"),
        },
        _ => value_null!("eval_greater_then"),
      }
    })
  }

  /// Returns an evaluator for `if`: DMN 1.6 Table 50, "if FEEL(e1) is true then FEEL(e2) else
  /// FEEL(e3)", so every condition that is not `true` takes the else branch.
  fn build_if(&mut self, lhs: &AstNode, mhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let mhe = self.build(mhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| match lhe(scope) {
      Value::Boolean(true) => mhe(scope),
      _ => rhe(scope),
    })
  }

  fn build_in(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match rhv {
        inner @ Value::Null(_)
        | inner @ Value::Number(_)
        | inner @ Value::String(_)
        | inner @ Value::Boolean(_)
        | inner @ Value::Date(_)
        | inner @ Value::Time(_)
        | inner @ Value::DateTime(_)
        | inner @ Value::YearsAndMonthsDuration(_)
        | inner @ Value::DaysAndTimeDuration(_)
        | inner @ Value::Context(_) => eval_in_equal(&mut Equality::default(), &lhv, &inner),
        Value::Range(l, l_type, r, r_type) => eval_in_range(&lhv, &l, l_type, &r, r_type),
        Value::List(r_inner) => {
          if let Value::List(l_inner) = lhv {
            eval_in_list_in_list(&mut Equality::default(), &l_inner, &r_inner)
          } else {
            eval_in_list(&mut Equality::default(), &lhv, &r_inner)
          }
        }
        Value::ExpressionList(inner) => eval_in_list(&mut Equality::default(), &lhv, &inner),
        Value::NegatedCommaList(inner) => eval_in_negated_list(&mut Equality::default(), &lhv, &inner),
        Value::UnaryEqual(inner) => eval_in_equal(&mut Equality::default(), &lhv, &inner),
        Value::UnaryNotEqual(inner) => eval_in_not_equal(&mut Equality::default(), &lhv, &inner),
        Value::UnaryLess(inner) => eval_in_unary_less(&lhv, inner.borrow()),
        Value::UnaryLessOrEqual(inner) => eval_in_unary_less_or_equal(&lhv, inner.borrow()),
        Value::UnaryGreater(inner) => eval_in_unary_greater(&lhv, inner.borrow()),
        Value::UnaryGreaterOrEqual(inner) => eval_in_unary_greater_or_equal(&lhv, inner.borrow()),
        Value::Irrelevant => VALUE_TRUE,
        _ => value_null!("unexpected argument type in 'in' operator: {}", arg(&TypeOf(&rhv))),
      }
    })
  }

  fn build_interval_end(&mut self, lhs: &AstNode, rhs: &IntervalType) -> Evaluator {
    let lhe = self.build(lhs);
    let interval_type = *rhs;
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      Value::IntervalEnd(Box::new(lhv), interval_type)
    })
  }

  fn build_interval_start(&mut self, lhs: &AstNode, rhs: &IntervalType) -> Evaluator {
    let lhe = self.build(lhs);
    let interval_type = *rhs;
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      Value::IntervalStart(Box::new(lhv), interval_type)
    })
  }

  fn build_irrelevant(&mut self) -> Evaluator {
    Box::new(move |_: &FeelScope| Value::Irrelevant)
  }

  fn build_instance_of(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      if let Value::FeelType(feel_type) = rhv {
        match lhv {
          Value::Number { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::Number => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::String { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::String => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::Boolean { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::Boolean => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::Date { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::Date => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::DateTime { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::DateTime => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::Time { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::Time => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::YearsAndMonthsDuration { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::YearsAndMonthsDuration => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::DaysAndTimeDuration { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            FeelType::DaysAndTimeDuration => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          Value::Null { .. } => match feel_type {
            FeelType::Null => VALUE_TRUE,
            _ => VALUE_FALSE,
          },
          value @ Value::Range { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            expected => Value::Boolean(value.has_type(&expected)),
          },
          value @ Value::List { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            expected => Value::Boolean(value.type_instance_of(&expected)),
          },
          value @ Value::Context { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            expected => Value::Boolean(value.type_instance_of(&expected)),
          },
          value @ Value::FunctionDefinition { .. } => match feel_type {
            FeelType::Any => VALUE_TRUE,
            expected => Value::Boolean(value.has_type(&expected)),
          },
          other => value_null!("invalid value in 'instance of' operator: {}", arg(&other)),
        }
      } else {
        Value::Boolean(lhv.has_type_of(&rhv))
      }
    })
  }

  fn build_le(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Number(lh) => match rhv {
          Value::Number(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_number"),
        },
        Value::String(lh) => match rhv {
          Value::String(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_string"),
        },
        Value::Date(lh) => match rhv {
          Value::Date(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_date"),
        },
        Value::DateTime(lh) => match rhv {
          Value::DateTime(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_date_time"),
        },
        Value::Time(lh) => match rhv {
          Value::Time(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_time"),
        },
        Value::DaysAndTimeDuration(lh) => match rhv {
          Value::DaysAndTimeDuration(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_days_and_time_duration"),
        },
        Value::YearsAndMonthsDuration(lh) => match rhv {
          Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh <= rh),
          _ => value_null!("eval_less_or_equal_years_and_months_duration"),
        },
        _ => value_null!("eval_less_or_equal"),
      }
    })
  }

  fn build_lt(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Number(lh) => match rhv {
          Value::Number(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_number"),
        },
        Value::String(lh) => match rhv {
          Value::String(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_string"),
        },
        Value::Date(lh) => match rhv {
          Value::Date(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_date"),
        },
        Value::DateTime(lh) => match rhv {
          Value::DateTime(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_date_time"),
        },
        Value::Time(lh) => match rhv {
          Value::Time(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_time"),
        },
        Value::DaysAndTimeDuration(lh) => match rhv {
          Value::DaysAndTimeDuration(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_days_and_time_duration"),
        },
        Value::YearsAndMonthsDuration(lh) => match rhv {
          Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh < rh),
          _ => value_null!("eval_less_then_years_and_months_duration"),
        },
        _ => value_null!("eval_less_then"),
      }
    })
  }

  fn build_list(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut values = vec![];
      for evaluator in &evaluators {
        values.push(evaluator(scope))
      }
      diagnostics::bound_depth(Value::List(values.into()))
    })
  }

  fn build_list_type(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      if let Value::FeelType(feel_type) = lhv {
        Value::FeelType(FeelType::List(Box::new(feel_type)))
      } else {
        value_null!("expected a feel type")
      }
    })
  }

  fn build_mul(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    arithmetic(Arithmetic::Multiplication, lhe, rhe, |lhv, rhv| match lhv {
      Value::Number(lh) => match rhv {
        Value::Number(rh) => Value::Number(lh * rh),
        Value::DaysAndTimeDuration(ref rh) => {
          let val = lh * FeelNumber::from(rh.as_nanos());
          if let Ok(v) = FeelNumber::try_into(val)
            && let Some(duration) = FeelDaysAndTimeDuration::from_n(v)
          {
            Value::DaysAndTimeDuration(duration)
          } else {
            value_null!("multiplication result is out of range of days and time duration")
          }
        }
        Value::YearsAndMonthsDuration(ref rh) => ym_product(temporal_value::ym_duration(lh * FeelNumber::from(rh.as_months()), &lhv, '*', &rhv)),
        _ => Arithmetic::Multiplication.incompatible(&[&lhv, &rhv]),
      },
      Value::DaysAndTimeDuration(ref lh) => match rhv {
        Value::Number(rh) => {
          let val = FeelNumber::from(lh.as_nanos()) * rh;
          if let Ok(v) = FeelNumber::try_into(val)
            && let Some(duration) = FeelDaysAndTimeDuration::from_n(v)
          {
            Value::DaysAndTimeDuration(duration)
          } else {
            value_null!("multiplication result is out of range of days and time duration")
          }
        }
        _ => Arithmetic::Multiplication.incompatible(&[&lhv, &rhv]),
      },
      Value::YearsAndMonthsDuration(ref lh) => match rhv {
        Value::Number(rh) => ym_product(temporal_value::ym_duration(FeelNumber::from(lh.as_months()) * rh, &lhv, '*', &rhv)),
        _ => Arithmetic::Multiplication.incompatible(&[&lhv, &rhv]),
      },
      other => value_null!("unexpected value type in multiplication: {}", arg(&TypeOf(&other))),
    })
  }

  fn build_name(&mut self, name: Name) -> Evaluator {
    Box::new(move |scope: &FeelScope| {
      if let Some(value) = scope.get_value(&name) {
        value
      } else if let Ok(bif) = Bif::from_str(&name.to_string()) {
        Value::BuiltInFunction(bif)
      } else {
        value_null!("context has no value for key '{}'", arg(&name))
      }
    })
  }

  /// Returns an evaluator for the arithmetic negation `-e`, the one unary arithmetic operator,
  /// whose null operand is an error as those of the binary ones are ([arithmetic]).
  fn build_neg(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      if let Some(null) = Arithmetic::Negation.null_operand(&[&lhv]) {
        return null;
      }
      match lhv {
        Value::Number(lh) => Value::Number(-lh),
        Value::DaysAndTimeDuration(lh) => Value::DaysAndTimeDuration(-lh),
        Value::YearsAndMonthsDuration(lh) => Value::YearsAndMonthsDuration(-lh),
        other => value_null!("unexpected type in arithmetic negation: {}", arg(&TypeOf(&other))),
      }
    })
  }

  fn build_negated_list(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut values = vec![];
      for evaluator in &evaluators {
        values.push(evaluator(scope))
      }
      Value::NegatedCommaList(values)
    })
  }

  fn build_null(&mut self) -> Evaluator {
    Box::new(move |_: &FeelScope| Value::Null(None))
  }

  fn build_number(&mut self, before: &str, after: &str, sign: char, exponent: &str) -> Evaluator {
    let mut text = String::new();
    let _ = write!(&mut text, "{before}");
    if !after.is_empty() {
      let _ = write!(&mut text, ".{after}");
    }
    if !exponent.is_empty() {
      let _ = write!(&mut text, "e{sign}{exponent}");
    }
    if let Ok(num) = text.parse::<FeelNumber>() {
      Box::new(move |_: &FeelScope| Value::Number(num))
    } else {
      Box::new(move |_: &FeelScope| value_null!("failed to convert text '{}' into number", arg(&text)))
    }
  }

  /// Returns an evaluator for `!=`: DMN 1.6 §10.3.2.15, "By definition, FEEL(e1 != e2) is
  /// FEEL(not(e 1= e2))", so the operands that `=` cannot compare give its null and its error.
  fn build_nq(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| match equal(&lhe(scope), &rhe(scope)) {
      Value::Boolean(result) => Value::Boolean(!result),
      null => null,
    })
  }

  /// Returns an evaluator for `disjunction`.
  ///
  /// ```text
  /// left        right        result
  ///─────────────────────────────────
  /// true        true         true
  /// true        false        true
  /// true        otherwise    true
  /// false       true         true
  /// false       false        false
  /// false       otherwise    null
  /// otherwise   true         true
  /// otherwise   false        null
  /// otherwise   otherwise    null
  /// ```
  fn build_or(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      match lhv {
        Value::Boolean(lh) => match rhv {
          Value::Boolean(rh) => Value::Boolean(lh || rh),
          _ => {
            if lh {
              Value::Boolean(true)
            } else {
              value_null!()
            }
          }
        },
        _ => match rhv {
          Value::Boolean(rh) => {
            if rh {
              Value::Boolean(true)
            } else {
              value_null!()
            }
          }
          _ => value_null!(),
        },
      }
    })
  }

  fn build_out(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let ine = self.build_in(lhs, rhs);
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| {
      let inv = ine(scope);
      let lhv = lhe(scope);
      match inv {
        Value::Boolean(true) => lhv,
        _ => value_null!(),
      }
    })
  }

  fn build_parameter_name(&mut self, lhs: &Name) -> Evaluator {
    let name = lhs.to_owned();
    Box::new(move |_: &FeelScope| Value::ParameterName(name.clone()))
  }

  fn build_parameter_types(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut values = vec![];
      for evaluator in &evaluators {
        values.push(evaluator(scope))
      }
      Value::ParameterTypes(values)
    })
  }

  fn build_qualified_name(&mut self, lhs: &[AstNode]) -> Evaluator {
    let mut evaluators = vec![];
    for node in lhs {
      evaluators.push(self.build(node));
    }
    Box::new(move |scope: &FeelScope| {
      let mut names = vec![];
      for evaluator in &evaluators {
        if let Value::QualifiedNameSegment(name) = evaluator(scope) {
          names.push(name);
        }
      }
      scope.search(&names).unwrap_or_else(|| value_null!("no value for qualified name"))
    })
  }

  fn build_qualified_name_segment(&mut self, name: &Name) -> Evaluator {
    let name = name.clone();
    Box::new(move |_: &FeelScope| Value::QualifiedNameSegment(name.clone()))
  }

  /// Returns and evaluator for `path expression`.
  /// `e...name` (DMN 1.6, grammar rule 68, DMN16-72): the values of every entry named `name` in the
  /// context `e` and in all its nested contexts, as a list, each matching entry before the entries
  /// nested in its value. A non-context `e` is null.
  fn build_descendant(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let AstNode::Name(name) = rhs else {
      return build_err_msg(err_expected_name(rhs));
    };
    let name = name.clone();
    let lhe = self.build(lhs);
    fn collect(context: &FeelContext, name: &Name, result: &mut Vec<Value>) {
      for (key, value) in context.iter() {
        if key == name {
          result.push(value.clone());
        }
        if let Value::Context(nested) = value {
          collect(nested, name, result);
        }
      }
    }
    Box::new(move |scope: &FeelScope| match lhe(scope) {
      Value::Context(context) => {
        let mut result = vec![];
        collect(&context, &name, &mut result);
        Value::List(result.into())
      }
      other => value_null!("descendant expression: expected context, actual type is {}", arg(&TypeOf(&other))),
    })
  }

  fn build_path(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let AstNode::Name(name) = rhs else {
      return build_err_msg(err_expected_name(rhs));
    };
    let name = name.clone();
    let lhe = self.build(lhs);
    // prepare and return the evaluator
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      match lhv {
        Value::Context(context) => {
          if let Some(value) = context.get_entry(&name) {
            value.clone()
          } else {
            value_null!(note: "build_path: no entry {} in context: {}", arg(&name), arg(&context))
          }
        }
        Value::List(items) => {
          let mut result = vec![];
          for item in items {
            if let Value::Context(context) = item {
              if let Some(value) = context.get_entry(&name) {
                result.push(value.clone());
              } else {
                result.push(value_null!());
              }
            } else {
              result.push(get_property_from_value(item, &name));
            }
          }
          Value::List(result.into())
        }
        other => get_property_from_value(other, &name),
      }
    })
  }

  fn build_range(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      if let Value::IntervalStart(lhv, l_closed) = lhv {
        if let Value::IntervalEnd(rhv, r_closed) = rhv {
          diagnostics::bound_depth(Value::Range(lhv, l_closed, rhv, r_closed))
        } else {
          value_null!("expected interval end")
        }
      } else {
        value_null!("expected interval start")
      }
    })
  }

  fn build_range_type(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      if let Value::FeelType(feel_type) = lhv {
        Value::FeelType(FeelType::Range(Box::new(feel_type)))
      } else {
        value_null!("expected a feel type")
      }
    })
  }

  fn build_some(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    self.build_quantified(lhs, rhs, Iteration::some)
  }

  fn build_string(&mut self, lhs: &str) -> Evaluator {
    let value = Value::String(lhs.into());
    Box::new(move |_: &FeelScope| value.clone())
  }

  fn build_sub(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    arithmetic(Arithmetic::Subtraction, lhe, rhe, |lhv, rhv| {
      // the operands stay available for the message below: temporal values are copied only in
      // the arm that subtracts them
      match &lhv {
        Value::Number(lh) => {
          if let Value::Number(rh) = &rhv {
            return Value::Number(*lh - *rh);
          }
        }
        Value::Date(lh) => match &rhv {
          Value::Date(rh) => {
            let l = FeelDateTime::new(lh.clone(), FeelTime::midnight(FeelZone::Utc));
            let r = FeelDateTime::new(rh.clone(), FeelTime::midnight(FeelZone::Utc));
            if let Some(result) = l - r {
              return Value::DaysAndTimeDuration(result);
            }
          }
          Value::DateTime(rh) => {
            let l = FeelDateTime::new(lh.clone(), FeelTime::midnight(FeelZone::Utc));
            if let Some(result) = l - rh.clone() {
              return Value::DaysAndTimeDuration(result);
            }
          }
          Value::DaysAndTimeDuration(rh) => {
            if let Some(date) = lh.clone() - rh.clone() {
              return Value::Date(date);
            }
          }
          Value::YearsAndMonthsDuration(rh) => {
            if let Some(date) = lh.clone() - rh.clone() {
              return Value::Date(date);
            }
          }
          _ => {}
        },
        Value::Time(lh) => match &rhv {
          Value::Time(rh) => {
            if let Some(result) = lh.clone() - rh.clone() {
              return Value::DaysAndTimeDuration(result);
            }
          }
          Value::DaysAndTimeDuration(rh) => {
            return Value::Time(lh.clone() - rh.clone());
          }
          _ => {}
        },
        Value::DateTime(lh) => match &rhv {
          Value::Date(rh) => {
            let r = FeelDateTime::new(rh.clone(), FeelTime::midnight(FeelZone::Utc));
            if let Some(result) = lh.clone() - r {
              return Value::DaysAndTimeDuration(result);
            }
          }
          Value::DateTime(rh) => {
            if let Some(result) = lh.clone() - rh.clone() {
              return Value::DaysAndTimeDuration(result);
            }
          }
          Value::DaysAndTimeDuration(rh) => {
            if let Some(result) = lh.clone() - rh.clone() {
              return Value::DateTime(result);
            }
          }
          Value::YearsAndMonthsDuration(rh) => {
            if let Some(result) = lh.clone() - rh.clone() {
              return Value::DateTime(result);
            }
          }
          _ => {}
        },
        Value::DaysAndTimeDuration(lh) => {
          if let Value::DaysAndTimeDuration(rh) = &rhv {
            return lh
              .checked_sub(rh)
              .map_or_else(|| value_null!("days and time duration subtraction overflow"), Value::DaysAndTimeDuration);
          }
        }
        Value::YearsAndMonthsDuration(lh) => {
          if let Value::YearsAndMonthsDuration(rh) = &rhv {
            return lh
              .checked_sub(rh)
              .map_or_else(|| value_null!("years and months duration subtraction overflow"), Value::YearsAndMonthsDuration);
          }
        }
        _ => {}
      }
      // the operands as evaluated above: evaluating them again would repeat their diagnostics
      // and take exponential time in a chain of failing subtractions
      Arithmetic::Subtraction.incompatible(&[&lhv, &rhv])
    })
  }

  /// Returns an evaluator for unary `=` (equal) operator.
  fn build_unary_eq(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Value::UnaryEqual(lhe(scope).into()))
  }

  /// Returns an evaluator for unary `!=` (not equal) operator.
  fn build_unary_ne(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Value::UnaryNotEqual(lhe(scope).into()))
  }

  /// Returns an evaluator for unary `<` (less) operator.
  fn build_unary_lt(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Value::UnaryLess(lhe(scope).into()))
  }

  /// Returns an evaluator for unary `<=` (less or equal) operator.
  fn build_unary_le(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Value::UnaryLessOrEqual(lhe(scope).into()))
  }

  /// Returns an evaluator for unary `>` (greater) operator.
  fn build_unary_gt(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Value::UnaryGreater(lhe(scope).into()))
  }

  /// Returns an evaluator for unary `>=` (greater or equal) operator.
  fn build_unary_ge(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Value::UnaryGreaterOrEqual(lhe(scope).into()))
  }
}

/// The arithmetic operators, named and written as their errors name and write them.
#[derive(Clone, Copy)]
enum Arithmetic {
  Addition,
  Subtraction,
  Multiplication,
  Division,
  Exponentiation,
  Negation,
}

impl Arithmetic {
  fn name(self) -> &'static str {
    match self {
      Arithmetic::Addition => "addition",
      Arithmetic::Subtraction => "subtraction",
      Arithmetic::Multiplication => "multiplication",
      Arithmetic::Division => "division",
      Arithmetic::Exponentiation => "exponentiation",
      Arithmetic::Negation => "negation",
    }
  }

  fn symbol(self) -> &'static str {
    match self {
      Arithmetic::Addition => "+",
      Arithmetic::Subtraction | Arithmetic::Negation => "-",
      Arithmetic::Multiplication => "*",
      Arithmetic::Division => "/",
      Arithmetic::Exponentiation => "**",
    }
  }

  /// The rule of a null operand, for every arithmetic operator and on either side: DMN 1.6, before
  /// Table 57 and before Table 59, "if input values are not of the listed types, the result is
  /// null", and the official TCK expects an error result for every arithmetic operation with a null
  /// operand (compliance-level-3/0100-arithmetic). With a null among `operands`, the result is
  /// [Arithmetic::incompatible], an error of the operator's own; a null that stands for an error
  /// of its own has counted it where it was created (§7.3.8). `None` when no operand is null.
  fn null_operand(self, operands: &[&Value]) -> Option<Value> {
    operands.iter().any(|operand| operand.is_null()).then(|| self.incompatible(operands))
  }

  /// The null of this operator on `operands` it does not take, with the error
  /// `[<name>] incompatible types: <operation>`, the operation written with its operands as
  /// evaluated (`1 - null`, `-null`).
  fn incompatible(self, operands: &[&Value]) -> Value {
    value_null!("[{}] incompatible types: {}", self.name(), Written(self, operands))
  }
}

/// An arithmetic operation as its error writes it: the operands infix, or after the symbol for a
/// single one.
struct Written<'a>(Arithmetic, &'a [&'a Value]);

impl std::fmt::Display for Written<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Written(operator, operands) = self;
    let symbol = operator.symbol();
    if let [operand] = operands {
      return write!(f, "{symbol}{}", arg(*operand));
    }
    for (index, operand) in operands.iter().enumerate() {
      if index > 0 {
        write!(f, " {symbol} ")?;
      }
      write!(f, "{}", arg(*operand))?;
    }
    Ok(())
  }
}

/// An evaluator of the binary arithmetic `operator` on the values of `lhe` and `rhe`, evaluated in
/// that order: `operation` gets them when [Arithmetic::null_operand] gives no null.
fn arithmetic(operator: Arithmetic, lhe: Evaluator, rhe: Evaluator, operation: impl Fn(Value, Value) -> Value + Send + Sync + 'static) -> Evaluator {
  Box::new(move |scope: &FeelScope| {
    let lhv = lhe(scope);
    let rhv = rhe(scope);
    operate(operator, &operation, lhv, rhv)
  })
}

/// The null rule of [arithmetic] and the operation, out of line: the evaluator's frame is on the
/// stack while an operand nested in it evaluates, so it holds the two operands and no locals of
/// the operation, and a deep chain of operators keeps a small frame per level.
#[inline(never)]
fn operate(operator: Arithmetic, operation: &impl Fn(Value, Value) -> Value, lhv: Value, rhv: Value) -> Value {
  if let Some(null) = operator.null_operand(&[&lhv, &rhv]) {
    return null;
  }
  operation(lhv, rhv)
}

/// FEEL `lhs = rhs` (DMN 1.6 Table 53, Table 54): a boolean, or null with an error when the operands
/// cannot be compared.
fn equal(lhs: &Value, rhs: &Value) -> Value {
  if let Some(result) = eval_ternary_equality(lhs, rhs) {
    Value::Boolean(result)
  } else {
    value_null!("equal err '{}' =?= '{}'", arg(lhs), arg(rhs))
  }
}

fn get_property_from_value(value: Value, name: &Name) -> Value {
  let property = Bip::new(name.as_str());
  match value {
    Value::Date(date) => match property {
      Bip::Year => Value::Number(date.year().into()),
      Bip::Month => Value::Number(date.month().into()),
      Bip::Day => Value::Number(date.day().into()),
      Bip::Weekday => {
        if let Some(day_of_week) = date.day_of_week() {
          Value::Number(day_of_week.1.into())
        } else {
          value_null!("could not retrieve weekday for date")
        }
      }
      Bip::Value => temporal_value::of_date(&date),
      other => value_null!("no such property in date: {}", arg(&other)),
    },
    Value::DateTime(date_time) => match property {
      Bip::Year => Value::Number(date_time.year().into()),
      Bip::Month => Value::Number(date_time.month().into()),
      Bip::Day => Value::Number(date_time.day().into()),
      Bip::Weekday => {
        if let Some(day_of_week) = date_time.day_of_week() {
          Value::Number(day_of_week.1.into())
        } else {
          value_null!("could not retrieve weekday for date and time")
        }
      }
      Bip::Hour => Value::Number(date_time.hour().into()),
      Bip::Minute => Value::Number(date_time.minute().into()),
      Bip::Second => temporal_value::second(date_time.second().into(), date_time.nanos()),
      Bip::TimeOffset => {
        if let Some(offset) = date_time.feel_time_offset() {
          Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(offset))
        } else {
          value_null!("could not retrieve time offset for date and time")
        }
      }
      Bip::Timezone => {
        if let Some(feel_time_zone) = date_time.feel_time_zone() {
          Value::String(feel_time_zone.into())
        } else {
          value_null!("could not retrieve timezone for date and time")
        }
      }
      Bip::Value => temporal_value::of_date_time(&date_time),
      other => value_null!("no such property in date and time: {}", arg(&other)),
    },
    Value::Time(time) => match property {
      Bip::Hour => Value::Number(time.hour().into()),
      Bip::Minute => Value::Number(time.minute().into()),
      Bip::Second => temporal_value::second(time.second().into(), time.nanos()),
      Bip::TimeOffset => {
        if let Some(offset) = time.feel_time_offset() {
          Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(offset))
        } else {
          value_null!("could not retrieve time offset for time")
        }
      }
      Bip::Timezone => {
        if let Some(feel_time_zone) = time.feel_time_zone() {
          Value::String(feel_time_zone.into())
        } else {
          value_null!("could not retrieve timezone for time")
        }
      }
      Bip::Value => temporal_value::of_time(&time),
      other => value_null!("no such property in time: {}", arg(&other)),
    },
    Value::DaysAndTimeDuration(dt_duration) => match property {
      Bip::Days => Value::Number(dt_duration.get_days().into()),
      Bip::Hours => Value::Number(dt_duration.get_hours().into()),
      Bip::Minutes => Value::Number(dt_duration.get_minutes().into()),
      Bip::Seconds => temporal_value::second(dt_duration.get_seconds() as u64, dt_duration.get_nanoseconds()),
      Bip::Value => temporal_value::of_dt_duration(&dt_duration),
      other => value_null!("no such property in days and time duration: {}", arg(&other)),
    },
    Value::YearsAndMonthsDuration(ym_duration) => match property {
      Bip::Years => Value::Number(ym_duration.years().into()),
      Bip::Months => Value::Number(ym_duration.months().into()),
      Bip::Value => Value::Number(ym_duration.as_months().into()),
      other => value_null!("no such property in years and months duration: {}", arg(&other)),
    },
    Value::Range(rs, cs, re, ce) => match property {
      Bip::Start => *rs,
      Bip::StartIncluded => Value::Boolean(cs.closed()),
      Bip::End => *re,
      Bip::EndIncluded => Value::Boolean(ce.closed()),
      other => value_null!("no such property in range: {}", arg(&other)),
    },
    Value::UnaryEqual(value) => match property {
      Bip::Start => *value,
      Bip::End => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(true),
      other => value_null!("no such property in unary equal: {}", arg(&other)),
    },
    Value::UnaryNotEqual(value) => match property {
      Bip::Start => *value,
      Bip::End => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(true),
      other => value_null!("no such property in unary not equal: {}", arg(&other)),
    },
    Value::UnaryLess(value) => match property {
      Bip::End => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(false),
      other => value_null!("no such property in unary less: {}", arg(&other)),
    },
    Value::UnaryLessOrEqual(value) => match property {
      Bip::End => *value,
      Bip::StartIncluded => Value::Boolean(false),
      Bip::EndIncluded => Value::Boolean(true),
      other => value_null!("no such property in unary less or equal: {}", arg(&other)),
    },
    Value::UnaryGreater(value) => match property {
      Bip::Start => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(false),
      other => value_null!("no such property in unary greater: {}", arg(&other)),
    },
    Value::UnaryGreaterOrEqual(value) => match property {
      Bip::Start => *value,
      Bip::StartIncluded => Value::Boolean(true),
      Bip::EndIncluded => Value::Boolean(false),
      other => value_null!("no such property in unary greater or equal: {}", arg(&other)),
    },
    v @ Value::Null(_) => v,
    other => value_null!("unexpected type: {}, for property: {}", arg(&TypeOf(&other)), arg(&property)),
  }
}

fn eval_in_list(equality: &mut Equality, left: &Value, items: &[Value]) -> Value {
  let mut unary_not_equal = vec![];
  for item in items {
    match item {
      inner @ Value::Null(_)
      | inner @ Value::String(_)
      | inner @ Value::Number(_)
      | inner @ Value::Boolean(_)
      | inner @ Value::Date(_)
      | inner @ Value::Time(_)
      | inner @ Value::DateTime(_)
      | inner @ Value::YearsAndMonthsDuration(_)
      | inner @ Value::DaysAndTimeDuration(_)
      | inner @ Value::Context(_) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryNotEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner.borrow()) {
          unary_not_equal.push(false);
        } else {
          unary_not_equal.push(true);
        }
      }
      Value::UnaryLess(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryLessOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less_or_equal(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryGreater(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryGreaterOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater_or_equal(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::List(inner) => {
        if let Value::Boolean(true) = eval_in_list(equality, left, inner) {
          return VALUE_TRUE;
        }
      }
      Value::Range(l, l_closed, r, r_closed) => {
        if let Value::Boolean(true) = eval_in_range(left, l, *l_closed, r, *r_closed) {
          return VALUE_TRUE;
        }
      }
      other => return value_null!("unexpected type in list: {}", arg(&TypeOf(other))),
    }
  }
  if !unary_not_equal.is_empty() && unary_not_equal.iter().all(|value| *value) {
    return VALUE_TRUE;
  }
  VALUE_FALSE
}

fn eval_in_negated_list(equality: &mut Equality, left: &Value, items: &[Value]) -> Value {
  let mut unary_not_equal = vec![];
  for item in items {
    match item {
      inner @ Value::Null(_)
      | inner @ Value::String(_)
      | inner @ Value::Number(_)
      | inner @ Value::Boolean(_)
      | inner @ Value::Date(_)
      | inner @ Value::Time(_)
      | inner @ Value::DateTime(_)
      | inner @ Value::DaysAndTimeDuration(_)
      | inner @ Value::YearsAndMonthsDuration(_) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryNotEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner.borrow()) {
          unary_not_equal.push(false);
        } else {
          unary_not_equal.push(true);
        }
      }
      Value::UnaryLess(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryLessOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less_or_equal(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryGreater(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryGreaterOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater_or_equal(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::List(inner) => {
        if let Value::Boolean(true) = eval_in_list(equality, left, inner) {
          return VALUE_FALSE;
        }
      }
      Value::Range(l, l_closed, r, r_closed) => {
        if let Value::Boolean(true) = eval_in_range(left, l, *l_closed, r, *r_closed) {
          return VALUE_FALSE;
        }
      }
      other => return value_null!("unexpected type in negated list: {}", arg(&TypeOf(other))),
    }
  }
  if !unary_not_equal.is_empty() && unary_not_equal.iter().all(|value| *value) {
    return VALUE_FALSE;
  }
  VALUE_TRUE
}

/// Checks if all elements from `list` are present in `items`.
fn eval_in_list_in_list(equality: &mut Equality, l_items: &[Value], r_items: &[Value]) -> Value {
  for item in r_items {
    if let Value::List(rhs) = item {
      let mut available: HashSet<usize> = (0..rhs.len()).collect();
      for l in l_items {
        let mut found = false;
        for (index, r) in rhs.iter().enumerate() {
          if available.contains(&index)
            && let Value::Boolean(true) = eval_in_equal(equality, l, r)
          {
            available.remove(&index);
            found = true;
            break;
          }
        }
        if !found {
          return VALUE_FALSE;
        }
      }
      return VALUE_TRUE;
    }
  }
  VALUE_FALSE
}

fn eval_in_range(lhv: &Value, start: &Value, start_type: IntervalType, end: &Value, end_type: IntervalType) -> Value {
  match lhv {
    Value::Number(value) => match start {
      Value::Number(lv) => match end {
        Value::Number(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.undefined() => {
          let start_in_range = if start_type.closed() { value >= lv } else { value > lv };
          Value::Boolean(start_in_range)
        }
        Value::Null(_) => value_null!(),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.undefined() => match end {
        Value::Number(rv) => {
          let end_in_range = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(end_in_range)
        }
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) => match end {
        Value::Number(_) => value_null!(),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::String(value) => match start {
      Value::String(lv) => match end {
        Value::String(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::String(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::Date(value) => match start {
      Value::Date(lv) => match end {
        Value::Date(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::Date(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::Time(value) => match start {
      Value::Time(lv) => match end {
        Value::Time(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::Time(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::DateTime(value) => match start {
      Value::DateTime(lv) => match end {
        Value::DateTime(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::DateTime(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::YearsAndMonthsDuration(value) => match start {
      Value::YearsAndMonthsDuration(lv) => match end {
        Value::YearsAndMonthsDuration(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::YearsAndMonthsDuration(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::DaysAndTimeDuration(value) => match start {
      Value::DaysAndTimeDuration(lv) => match end {
        Value::DaysAndTimeDuration(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::DaysAndTimeDuration(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    _ => value_null!("eval_in_range"),
  }
}

fn eval_in_equal(equality: &mut Equality, left: &Value, right: &Value) -> Value {
  if let Some(true) = equality.compare(left, right) { VALUE_TRUE } else { VALUE_FALSE }
}

fn eval_in_not_equal(equality: &mut Equality, left: &Value, right: &Value) -> Value {
  if let Some(false) = equality.compare(left, right) { VALUE_TRUE } else { VALUE_FALSE }
}

fn eval_in_unary_less(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l < *r);
      }
    }
    Value::String(r) => {
      if let Value::String(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l < r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_less")
}

fn eval_in_unary_less_or_equal(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l <= *r);
      }
    }
    Value::String(r) => {
      if let Value::String(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_less_or_equal")
}

fn eval_in_unary_greater(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l > *r);
      }
    }
    Value::String(r) => {
      if let Value::String(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l > r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_greater")
}

fn eval_in_unary_greater_or_equal(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l >= *r);
      }
    }
    Value::String(r) => {
      if let Value::String(left_value) = left {
        return Value::Boolean(left_value >= r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_greater_or_equal")
}

/// The iteration of a `for`, `some` or `every` expression over `variables`, in order. A variable
/// named twice builds an evaluator of null instead: the later binding would hide the earlier one.
fn iteration(variables: Vec<(Name, Domain)>) -> std::result::Result<Iteration, Evaluator> {
  let mut names = HashSet::new();
  if let Some((name, _)) = variables.iter().find(|(name, _)| !names.insert(name)) {
    let name = name.clone();
    return Err(Box::new(move |_: &FeelScope| value_null!("duplicate iteration variable '{}'", arg(&name))));
  }
  Ok(Iteration::for_feel(variables))
}

fn build_err_msg(err_msg: String) -> Evaluator {
  Box::new(move |_: &FeelScope| value_null!(err_msg))
}

/// Returns an error message for unexpected node in AST.
fn err_msg_unexpected_node(node: &AstNode) -> String {
  format!("unexpected AST node: {node:?}")
}

/// Returns an error message when other AST node was expected.
fn err_msg_expected_node(expected: &str, actual: &AstNode) -> String {
  format!("expected AST node {expected}, actual AST node is {actual:?}")
}

/// Returns an error message when expected only a named or positional parameter.
fn err_msg_expected_positional_or_named_parameter(actual: &AstNode) -> String {
  format!("expected positional or named parameter, actual AST node is {actual:?}")
}

/// Returns an error message when expected only a parameter name.
fn err_expected_parameter_name(actual: &AstNode) -> String {
  format!("expected AST node ParameterName, actual node is {actual:?}")
}

/// Returns an error message when expected only a parameter name.
fn err_expected_name(actual: &AstNode) -> String {
  format!("expected name node, actual node is: {actual:?}")
}

/// The value of a years and months duration times a number: the duration [temporal_value::ym_duration]
/// gives, or null beyond the range.
fn ym_product(duration: Option<FeelYearsAndMonthsDuration>) -> Value {
  match duration {
    Some(duration) => Value::YearsAndMonthsDuration(duration),
    None => value_null!("multiplication result is out of range of years and months duration"),
  }
}

/// The evaluator of an `@` literal of `value` with seconds: with no `dropped` digits it gives the
/// value, otherwise it also records the warning, built once, that `text` lost them.
fn truncating_at_literal(value: Value, text: &str, dropped: Option<&str>) -> Evaluator {
  match dropped {
    None => Box::new(move |_: &FeelScope| value.clone()),
    Some(dropped) => {
      let warning = truncated_fraction_warning(text, dropped);
      Box::new(move |_: &FeelScope| {
        diagnostics::warn(&warning);
        value.clone()
      })
    }
  }
}

/// The numbers of temporal values: the `value` property (DMN 1.6 §10.3.2.3.4–§10.3.2.3.8, Table
/// 66), `valuet`, `valuedt`, `valuedtd` and `valueymd` of the spec, and the `second` and `seconds`
/// properties, decimals in the interval [0..60) (Table 67).
mod temporal_value {
  use rheon_feel::diagnostics;
  use rheon_feel::message::{arg, compose};
  use rheon_feel::values::Value;
  use rheon_feel::{FeelNumber, value_null};
  use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration, FeelZone};

  const NANOS_IN_SECOND: i64 = 1_000_000_000;
  const SECONDS_IN_DAY: i64 = 86_400;

  /// Seconds (with the fractional nanoseconds) as an exact number.
  fn seconds(nanos: i64) -> Value {
    let whole = nanos.div_euclid(NANOS_IN_SECOND);
    let fraction = nanos.rem_euclid(NANOS_IN_SECOND);
    let n = FeelNumber::from(whole);
    Value::Number(if fraction == 0 { n } else { n + FeelNumber::new(fraction, 9) })
  }

  /// `valueymd⁻¹` (DMN 1.6 §10.3.2.3.8): the years and months duration of `months`, the product
  /// or the quotient of Table 60 as FEEL computes it, `None` beyond the range of a duration. A
  /// fraction of a month is truncated toward zero, as the DMN TCK expects, and the evaluation
  /// warns that the operation `left operator right` dropped it.
  pub fn ym_duration(months: FeelNumber, left: &Value, operator: char, right: &Value) -> Option<FeelYearsAndMonthsDuration> {
    let duration = i64::try_from(&months).ok().and_then(FeelYearsAndMonthsDuration::from_m)?;
    if !months.is_integer() {
      warn_dropped_months(months.frac(), left, operator, right);
    }
    Some(duration)
  }

  /// Records the warning that the operation `left operator right` dropped the fraction of a month
  /// `dropped`; a warning is the rare path, kept out of the arithmetic.
  #[cold]
  #[inline(never)]
  fn warn_dropped_months(dropped: FeelNumber, left: &Value, operator: char, right: &Value) {
    diagnostics::warn(&compose(format_args!(
      "fractional months of '{} {operator} {}' truncated to whole months; dropped fraction: '{}'",
      arg(left),
      arg(right),
      arg(&dropped)
    )));
  }

  /// The second of a minute with its fraction, from the whole seconds and the nanoseconds:
  /// `second` of a time or a date and time and `seconds` of a days and time duration.
  pub fn second(whole: u64, nanos: u64) -> Value {
    seconds((whole * NANOS_IN_SECOND as u64 + nanos) as i64)
  }

  /// `valuedtd`: a days and time duration as a number of seconds.
  pub fn of_dt_duration(duration: &FeelDaysAndTimeDuration) -> Value {
    seconds(duration.as_nanos())
  }

  /// `valuedt`: seconds since 1970-01-01T00:00:00Z of a date and time that has a UTC form (an
  /// offset or a time zone); a local date and time has none, so its value is null.
  pub fn of_date_time(date_time: &FeelDateTime) -> Value {
    if !date_time.time().zone().has_offset() {
      return value_null!("value: a local date and time has no UTC form");
    }
    let Some(epoch) = FeelDateTime::utc(1970, 1, 1, 0, 0, 0, 0) else {
      return value_null!("value: epoch");
    };
    match date_time.nanos_since(&epoch) {
      Some(nanos) => seconds(nanos),
      None => value_null!("value: date and time out of range"),
    }
  }

  /// A date is taken as the date and time at UTC midnight (§10.3.2.3.5).
  pub fn of_date(date: &FeelDate) -> Value {
    of_date_time(&FeelDateTime::new(date.clone(), FeelTime::midnight(FeelZone::Utc)))
  }

  /// `valuet`: seconds since midnight in [0..86400). A time with an offset is taken in its UTC
  /// form, which keeps `value` consistent with time subtraction; a local time uses its clock time,
  /// and a time in a zone whose offset is not known has no value (null), never the value of UTC.
  pub fn of_time(time: &FeelTime) -> Value {
    let clock = (time.hour() as i64 * 3600 + time.minute() as i64 * 60 + time.second() as i64) * NANOS_IN_SECOND + time.nanos() as i64;
    let offset = match (time.zone(), time.feel_time_offset()) {
      (FeelZone::Local, _) => 0,
      (_, Some(offset)) => offset as i64 * NANOS_IN_SECOND,
      (_, None) => return value_null!("value: the time zone of the time has no known offset"),
    };
    seconds((clock - offset).rem_euclid(SECONDS_IN_DAY * NANOS_IN_SECOND))
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    fn time(zone: FeelZone) -> FeelTime {
      FeelTime::zone_opt(10, 0, 0, 500_000_000, zone).unwrap()
    }

    /// DMN 1.6 §10.3.2.3.4: "If no time offset is specified, the time is interpreted as a local
    /// time of day", so a local time is valued by its clock time; a time in a zone is valued in
    /// its UTC form, and a zone whose offset cannot be resolved gives no value instead of the
    /// value of UTC. Such a zone is not reachable from an expression whose result does not depend
    /// on the date of the evaluation (a zone name is checked when it is read, and a time in a
    /// gap of daylight saving time resolves on the current date).
    #[test]
    fn the_value_of_a_time_needs_its_offset() {
      assert_eq!(Value::Number("36000.5".parse().unwrap()), of_time(&time(FeelZone::Local)));
      assert_eq!(Value::Number("36000.5".parse().unwrap()), of_time(&time(FeelZone::Utc)));
      let plus_one = FeelTime::offset_opt(10, 0, 0, 500_000_000, 3600).unwrap();
      assert_eq!(Value::Number("32400.5".parse().unwrap()), of_time(&plus_one));
      assert_eq!(Value::Number("32400.5".parse().unwrap()), of_time(&time(FeelZone::Zone("Etc/GMT-1".to_string()))));
      assert_eq!(
        Value::Null(Some("value: the time zone of the time has no known offset".to_string())),
        of_time(&time(FeelZone::Zone("No/Such_Zone".to_string())))
      );
    }
  }
}
