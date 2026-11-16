//! # Implementation of `for`, `some` and `every` expressions
//!
//! The iteration contexts of these expressions bind their variables in order: the domain of a
//! variable is evaluated with the variables before it bound (DMN 1.6 §10.3.2.14 lists `x..x+10`
//! among the valid iteration contexts), once for every combination of their values, and a domain
//! without items contributes no iteration.

use crate::errors::*;
use dsntk_common::Result;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::arg;
use rheon_feel::values::{FeelList, ListItems, TypeOf, Value};
use rheon_feel::{Evaluator, FEEL_TYPE_NAME_DATE, FEEL_TYPE_NAME_NUMBER, FeelNumber, FeelScope, IntervalType, Name, ScopeFrame, value_null};
use rheon_feel_temporal::FeelDate;
use std::ops::ControlFlow;
use std::sync::Arc;

/// The values one iteration variable takes, in order.
pub struct Sequence(Items);

/// The items of a [Sequence]: the remaining items of a list (taken one at a time, so a shared
/// list is not copied first), a single value, or the next number or date of an interval, which
/// are produced one at a time.
enum Items {
  List(ListItems),
  One(Option<Value>),
  Numbers { next: Option<FeelNumber>, end: FeelNumber, ascending: bool },
  Dates { next: Option<FeelDate>, end: FeelDate, ascending: bool },
}

impl Sequence {
  /// The items of a list; a value that is not a list is the only item.
  pub fn of(value: Value) -> Self {
    match value {
      Value::List(values) => Self(Items::List(values.into_iter())),
      single => Self(Items::One(Some(single))),
    }
  }

  /// No items.
  pub fn empty() -> Self {
    Self(Items::One(None))
  }

  /// The numbers from `start` to `end` by steps of one, or the dates by steps of one day, in
  /// either direction (an interval `start..end`).
  pub fn interval(start: Value, end: Value) -> Result<Self> {
    Self::between(start, end, false)
  }

  /// The numbers or the dates of a range literal `[start..end]`: closed at both ends, with its
  /// start not above its end.
  pub(crate) fn range(start: Value, end: Value) -> Result<Self> {
    match start {
      Value::IntervalStart(start, IntervalType::Closed) => match end {
        Value::IntervalEnd(end, IntervalType::Closed) => Self::between(*start, *end, true),
        other => Err(err_invalid_range_end(arg(&other))),
      },
      other => Err(err_invalid_range_start(arg(&other))),
    }
  }

  /// The numbers or the dates from `start` to `end`; `ascending_only` rejects a start above the end.
  fn between(start: Value, end: Value, ascending_only: bool) -> Result<Self> {
    match start {
      Value::Number(start) => match end {
        Value::Number(end) if ascending_only && start > end => Err(err_invalid_range()),
        Value::Number(end) => Ok(Self(Items::Numbers {
          next: Some(start),
          end,
          ascending: start <= end,
        })),
        other => Err(err_invalid_interval_end(FEEL_TYPE_NAME_NUMBER, arg(&TypeOf(&other)))),
      },
      Value::Date(start) => match end {
        Value::Date(end) if ascending_only && start > end => Err(err_invalid_range()),
        Value::Date(end) => {
          let ascending = start <= end;
          Ok(Self(Items::Dates {
            next: Some(start),
            end,
            ascending,
          }))
        }
        other => Err(err_invalid_interval_end(FEEL_TYPE_NAME_DATE, arg(&TypeOf(&other)))),
      },
      other => Err(err_invalid_interval_start(arg(&TypeOf(&other)))),
    }
  }

  /// The next item, `None` after the last one.
  ///
  /// Numbers have 34 significant digits, so for |n| >= 1e34 adding the step does not change the
  /// value (`1e34 + 1 = 1e34`); such a lost step also ends a number interval, otherwise
  /// `for i in 1e34..1e34 return i` would never end. A date interval ends at its end, or at the
  /// last or the first supported date, where no date follows.
  pub(crate) fn next_item(&mut self) -> Option<Value> {
    match &mut self.0 {
      Items::List(values) => values.next(),
      Items::One(value) => value.take(),
      Items::Numbers { next, end, ascending } => {
        let current = (*next)?;
        let following = if *ascending { current + FeelNumber::one() } else { current - FeelNumber::one() };
        let within = if *ascending { following <= *end } else { following >= *end };
        *next = (within && following != current).then_some(following);
        Some(Value::Number(current))
      }
      Items::Dates { next, end, ascending } => {
        let current = next.take()?;
        *next = if *ascending {
          current.add_days(1).filter(|date| *date <= *end)
        } else {
          current.sub_days(1).filter(|date| *date >= *end)
        };
        Some(Value::Date(current))
      }
    }
  }
}

/// Evaluates what an iteration variable iterates over, with the variables before it in scope.
pub type Domain = Box<dyn Fn(&FeelScope) -> Result<Sequence> + Send + Sync>;

/// Evaluates `evaluate` with the iteration context `ctx` on the top of the scope, pushed shared:
/// what the evaluation writes into the context on the top of the scope (a boxed context does,
/// model-evaluator) goes into a copy of it ([ScopeFrame]), which is dropped with the frame, so it
/// does not reach the next iteration. Returns the value and the frame the evaluation ran in, with
/// what the evaluation left in it; the caller drops the frame before it changes `ctx` again, so
/// that `ctx` is held alone then and changed without a copy.
///
/// `None` when the evaluation removed the iteration context from the scope (it popped more
/// contexts than it pushed, which no evaluator does): the iteration can not go on correctly, and
/// the caller returns null instead of a result that misses values.
fn evaluate_in_iteration_context<T>(scope: &FeelScope, ctx: &Arc<FeelContext>, evaluate: impl FnOnce(&FeelScope) -> T) -> Option<(T, ScopeFrame)> {
  let depth = scope.depth();
  scope.push_shared(Arc::clone(ctx));
  let value = evaluate(scope);
  let frame = scope.pop_frame(depth)?;
  Some((value, frame))
}

/// The null returned when an evaluation removed from the scope the context that a `for`, `some`
/// or `every` expression or a filter pushed for it (see [evaluate_in_iteration_context]):
/// `context` names that context. No correct result can be given then.
pub(super) fn lost_context(context: &str) -> Value {
  value_null!("internal error: the evaluation removed the {}", arg(&context))
}

/// The iteration contexts of a `for`, `some` or `every` expression: its variables, each with its
/// domain, bound in order.
pub struct Iteration {
  variables: Vec<(Name, Domain)>,
}

impl Iteration {
  /// The iteration contexts of a FEEL expression: `variables`, bound in this order.
  pub(crate) fn new(variables: Vec<(Name, Domain)>) -> Self {
    Self { variables }
  }

  /// The one iteration context of a boxed `for`, `some` or `every` expression.
  pub fn boxed(variable: Name, domain: Domain) -> Self {
    Self::new(vec![(variable, domain)])
  }

  /// Visits every combination of the values of the variables, the last variable advancing
  /// fastest, with the variables bound in the iteration context. The domain of the first variable
  /// is evaluated in the scope of the expression; the domain of each later one in every iteration
  /// of the variables before it, with them bound. A domain without items contributes no
  /// iteration.
  ///
  /// The walk keeps one open sequence per bound variable, so its depth is the number of variables
  /// and no recursion is involved. When a sequence ends, its variable is unbound, so that a domain
  /// sees the variables before it only; `hidden` (the `partial` of a `for`) is unbound as well
  /// while a domain is evaluated. `Break` ends the walk with its value: the null of a domain that
  /// fails, the value `visit` breaks with, or a null when the evaluation was stopped
  /// ([diagnostics::stopped]), which the walk reads before each further item.
  ///
  /// The iteration context is one context for the whole walk, which `visit` gets to evaluate the
  /// body in ([evaluate_in_iteration_context]); a domain is evaluated in it the same way.
  fn walk(&self, scope: &FeelScope, expression: &str, hidden: Option<&Name>, mut visit: impl FnMut(&mut Arc<FeelContext>) -> ControlFlow<Value>) -> ControlFlow<Value> {
    let Some((_, first)) = self.variables.first() else {
      return ControlFlow::Continue(());
    };
    let mut ctx = Arc::new(FeelContext::default());
    let mut levels = Vec::with_capacity(self.variables.len());
    match first(scope) {
      Ok(sequence) => levels.push(sequence),
      Err(reason) => return ControlFlow::Break(value_null!(reason)),
    }
    while let Some(depth) = levels.len().checked_sub(1) {
      if diagnostics::stopped() {
        return ControlFlow::Break(Value::Null(None));
      }
      let name = &self.variables[depth].0;
      let Some(value) = levels[depth].next_item() else {
        levels.pop();
        Arc::make_mut(&mut ctx).remove_entry(name);
        continue;
      };
      Arc::make_mut(&mut ctx).assign_entry(name, value);
      match self.variables.get(depth + 1) {
        Some((_, domain)) => {
          if let Some(hidden) = hidden {
            Arc::make_mut(&mut ctx).remove_entry(hidden);
          }
          match evaluate_in_iteration_context(scope, &ctx, |scope| domain(scope)).map(|(sequence, _)| sequence) {
            Some(Ok(sequence)) => levels.push(sequence),
            Some(Err(reason)) => return ControlFlow::Break(value_null!(reason)),
            None => return ControlFlow::Break(lost_context(expression)),
          }
        }
        None => visit(&mut ctx)?,
      }
    }
    ControlFlow::Continue(())
  }

  /// Evaluates a `for` expression: the list of the values of `body` in every iteration.
  pub fn collect(&self, scope: &FeelScope, body: &Evaluator) -> Value {
    const EXPRESSION: &str = "iteration context of a `for` expression";
    // the implicit `partial` variable holds the results of all previous iterations
    let partial = Name::from("partial");
    let mut results = FeelList::new();
    let walked = self.walk(scope, EXPRESSION, Some(&partial), |ctx| {
      // Move the results into the iteration context and back out afterwards instead of cloning
      // the whole list on every iteration, which made `for` quadratic.
      Arc::make_mut(ctx).assign_entry(&partial, Value::List(std::mem::take(&mut results)));
      let Some((value, frame)) = evaluate_in_iteration_context(scope, ctx, body) else {
        return ControlFlow::Break(lost_context(EXPRESSION));
      };
      // The results are read from the frame the body ran in. A body that wrote into it (an
      // entry `partial` replaces the results) left a copy of the iteration context there; one
      // that did not left the iteration context itself, from which they are taken out.
      let written = !std::ptr::eq::<FeelContext>(&*frame, &**ctx);
      let written_results = if written {
        match frame.get_entry(&partial) {
          Some(Value::List(values)) => Some(values.clone()),
          _ => None,
        }
      } else {
        None
      };
      drop(frame);
      // the entry stays (with a placeholder) in the iteration context, which then holds no copy
      // of the results; the next iteration sets it again
      let taken = Arc::make_mut(ctx).assign_entry(&partial, Value::Null(None));
      let previous_results = if written {
        written_results
      } else {
        match taken {
          Some(Value::List(values)) => Some(values),
          _ => None,
        }
      };
      if let Some(values) = previous_results {
        results = values;
      }
      results.push(value);
      // the results grow with every iteration, through `partial` by a level in each
      if let Err(null) = diagnostics::bound_list_depth(&results) {
        return ControlFlow::Break(null);
      }
      ControlFlow::Continue(())
    });
    match walked {
      ControlFlow::Continue(()) => Value::List(results),
      ControlFlow::Break(null) => null,
    }
  }

  /// Evaluates a `some` expression: `true` as soon as the body is `true`, null as soon as it is
  /// not a boolean, otherwise `false`.
  pub fn some(&self, scope: &FeelScope, body: &Evaluator) -> Value {
    self.quantified(scope, body, true, "iteration context of a `some` expression")
  }

  /// Evaluates an `every` expression: `false` as soon as the body is `false`, null as soon as it
  /// is not a boolean, otherwise `true`.
  pub fn every(&self, scope: &FeelScope, body: &Evaluator) -> Value {
    self.quantified(scope, body, false, "iteration context of a `every` expression")
  }

  /// Evaluates a `some` expression (`decisive` is `true`) or an `every` expression (`decisive` is
  /// `false`): the first value of the body that is `decisive` decides the result, and so does the
  /// first value that is not a boolean (null); without either, the result is `!decisive`.
  fn quantified(&self, scope: &FeelScope, body: &Evaluator, decisive: bool, expression: &str) -> Value {
    let walked = self.walk(scope, expression, None, |ctx| {
      match evaluate_in_iteration_context(scope, ctx, body).map(|(value, _)| value) {
        Some(Value::Boolean(value)) if value != decisive => ControlFlow::Continue(()),
        Some(Value::Boolean(_)) => ControlFlow::Break(Value::Boolean(decisive)),
        Some(_) => ControlFlow::Break(value_null!()),
        None => ControlFlow::Break(lost_context(expression)),
      }
    });
    match walked {
      ControlFlow::Continue(()) => Value::Boolean(!decisive),
      ControlFlow::Break(value) => value,
    }
  }
}
