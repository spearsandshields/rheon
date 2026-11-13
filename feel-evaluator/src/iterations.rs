//! # Implementation of `for`, `some` and `every` expressions

use crate::errors::*;
use dsntk_common::Result;
use rheon_feel::context::FeelContext;
use rheon_feel::values::{VALUE_FALSE, VALUE_TRUE, Value, Values};
use rheon_feel::{Evaluator, FEEL_TYPE_NAME_DATE, FEEL_TYPE_NAME_NUMBER, FeelNumber, FeelScope, IntervalType, Name, value_null};
use rheon_feel_temporal::FeelDate;
use std::ops::ControlFlow;

/// Common interface for all iteration state types.
trait IterationState {
  /// Assigns the iteration value pointed by binding variable.
  fn bind_value(&mut self, _ctx: &FeelContext) {}

  /// Sets the current value of the iteration in provided context.
  fn set_value(&mut self, _ctx: &mut FeelContext) {}

  /// Iterates to next value.
  fn next(&mut self) -> bool {
    false
  }

  /// Checks if there is a next value to iterate to (what [IterationState::next] returns; the
  /// reference of the iteration loop in tests).
  #[cfg(test)]
  fn has_next(&self) -> bool {
    false
  }

  /// Check if there are values to iterate over.
  fn is_empty(&self) -> bool {
    false
  }

  /// Checks if the iteration state is a bounded variable.
  fn is_variable(&self) -> bool {
    false
  }
}

/// Iteration stepping direction.
#[derive(Debug)]
enum IterationDirection {
  Ascending,
  Descending,
}

/// Iteration state for number intervals.
struct NumberIntervalState {
  /// Name of the bound variable in this iterator state.
  variable: Name,
  /// Iteration starting value.
  start: FeelNumber,
  /// Iteration ending value.
  end: FeelNumber,
  /// Iteration step.
  step: FeelNumber,
  /// Iteration step direction.
  direction: IterationDirection,
  /// Current iteration value.
  current: FeelNumber,
}

impl NumberIntervalState {
  fn init(variable: Name, start: FeelNumber, end: FeelNumber) -> Box<dyn IterationState> {
    Box::new(Self {
      variable,
      start,
      end,
      step: FeelNumber::one(),
      direction: if start <= end { IterationDirection::Ascending } else { IterationDirection::Descending },
      current: start,
    })
  }
}

impl NumberIntervalState {
  /// Returns the value that follows `current` in the iteration direction, or `None` when the
  /// end of the interval is passed.
  ///
  /// Numbers have 34 significant digits, so for |current| >= 1e34 adding the step does not
  /// change the value (`1e34 + 1 = 1e34`); such a lost step also ends the iteration, otherwise
  /// `for i in 1e34..1e34 return i` would never terminate.
  fn following(&self) -> Option<FeelNumber> {
    let next = match self.direction {
      IterationDirection::Ascending => {
        let next = self.current + self.step;
        if next <= self.end { next } else { return None }
      }
      IterationDirection::Descending => {
        let next = self.current - self.step;
        if next >= self.end { next } else { return None }
      }
    };
    if next == self.current { None } else { Some(next) }
  }
}

impl IterationState for NumberIntervalState {
  fn set_value(&mut self, ctx: &mut FeelContext) {
    ctx.assign_entry(&self.variable, Value::Number(self.current));
  }

  fn next(&mut self) -> bool {
    if let Some(next) = self.following() {
      self.current = next;
      true
    } else {
      self.current = self.start;
      false
    }
  }

  #[cfg(test)]
  fn has_next(&self) -> bool {
    self.following().is_some()
  }
}

/// Iteration state for date intervals.
struct DateIntervalState {
  /// Name of the bound variable in this iterator state.
  variable: Name,
  /// Iteration starting value.
  start: FeelDate,
  /// Iteration ending value.
  end: FeelDate,
  /// Iteration step.
  step: u64,
  /// Iteration step direction.
  direction: IterationDirection,
  /// Current iteration value.
  current: FeelDate,
}

impl DateIntervalState {
  fn init(variable: Name, start: FeelDate, end: FeelDate) -> Box<dyn IterationState> {
    let direction = if start <= end { IterationDirection::Ascending } else { IterationDirection::Descending };
    let current = start.clone();
    Box::new(Self {
      variable,
      start,
      end,
      step: 1,
      direction,
      current,
    })
  }
}

impl IterationState for DateIntervalState {
  fn set_value(&mut self, ctx: &mut FeelContext) {
    ctx.assign_entry(&self.variable, Value::Date(self.current.clone()));
  }

  fn next(&mut self) -> bool {
    match self.direction {
      IterationDirection::Ascending => {
        let Some(date) = self.current.add_days(self.step) else { return false };
        if date <= self.end {
          self.current = date;
          true
        } else {
          self.current = self.start.clone();
          false
        }
      }
      IterationDirection::Descending => {
        let Some(date) = self.current.sub_days(self.step) else { return false };

        if date >= self.end {
          self.current = date;
          true
        } else {
          self.current = self.start.clone();
          false
        }
      }
    }
  }

  #[cfg(test)]
  fn has_next(&self) -> bool {
    match self.direction {
      IterationDirection::Ascending => self.current.add_days(self.step).is_some_and(|date| date <= self.end),
      IterationDirection::Descending => self.current.sub_days(self.step).is_some_and(|date| date >= self.end),
    }
  }
}

struct ListState {
  /// Name of the bound variable in this iterator state.
  variable: Name,
  /// Values to iterate over.
  values: Values,
  /// Iteration step.
  step: usize,
  /// Current iteration index.
  current: usize,
}

impl ListState {
  fn init(variable: Name, value: Value) -> Box<dyn IterationState> {
    Box::new(Self {
      variable,
      values: match value {
        Value::List(values) => values,
        single => vec![single],
      },
      step: 1,
      current: 0,
    })
  }
}

impl IterationState for ListState {
  fn set_value(&mut self, ctx: &mut FeelContext) {
    if !self.values.is_empty() {
      ctx.assign_entry(&self.variable, self.values[self.current].clone());
    }
  }

  fn next(&mut self) -> bool {
    if self.current + self.step < self.values.len() {
      self.current += self.step;
      true
    } else {
      self.current = 0;
      false
    }
  }

  #[cfg(test)]
  fn has_next(&self) -> bool {
    self.current < self.values.len().saturating_sub(1)
  }

  fn is_empty(&self) -> bool {
    self.values.is_empty()
  }
}

struct VariableState {
  /// Name of the bound variable in another iteration state.
  bound_variable: Name,
  /// List iteration state.
  list_state: ListState,
}

impl VariableState {
  fn init(variable: Name, bound_variable: Name) -> Box<dyn IterationState> {
    Box::new(Self {
      bound_variable,
      list_state: ListState {
        variable,
        values: vec![],
        step: 1,
        current: 0,
      },
    })
  }
}

impl IterationState for VariableState {
  fn bind_value(&mut self, ctx: &FeelContext) {
    if self.list_state.current == 0
      && let Some(value) = ctx.get(&self.bound_variable)
    {
      self.list_state.values = match value {
        Value::List(values) => values.clone(),
        single => vec![single.clone()],
      };
      self.list_state.step = 1;
      self.list_state.current = 0;
    }
  }

  fn set_value(&mut self, ctx: &mut FeelContext) {
    self.list_state.set_value(ctx);
  }

  fn next(&mut self) -> bool {
    self.list_state.next()
  }

  #[cfg(test)]
  fn has_next(&self) -> bool {
    self.list_state.has_next()
  }

  fn is_variable(&self) -> bool {
    true
  }
}

/// Iterator built from multiple iteration states.
#[derive(Default)]
pub struct FeelIterator {
  states: Vec<Box<dyn IterationState>>,
}

impl FeelIterator {
  /// Creates a new, empty iterator.
  pub fn new() -> Self {
    Self::default()
  }

  pub fn add_interval(&mut self, variable: Name, start: Value, end: Value, strict: bool) -> Result<()> {
    match start {
      Value::Number(start) => match end {
        Value::Number(end) => {
          if strict && start > end {
            Err(err_invalid_range())
          } else {
            self.states.push(NumberIntervalState::init(variable, start, end));
            Ok(())
          }
        }
        other => Err(err_invalid_interval_end(FEEL_TYPE_NAME_NUMBER, &other.type_of().to_string())),
      },
      Value::Date(start) => match end {
        Value::Date(end) => {
          if strict && start > end {
            Err(err_invalid_range())
          } else {
            self.states.push(DateIntervalState::init(variable, start, end));
            Ok(())
          }
        }
        other => Err(err_invalid_interval_end(FEEL_TYPE_NAME_DATE, &other.type_of().to_string())),
      },
      other => Err(err_invalid_interval_start(&other.type_of().to_string())),
    }
  }

  pub fn add_range(&mut self, variable: Name, start: Value, end: Value) -> Result<()> {
    match start {
      Value::IntervalStart(start, IntervalType::Closed) => match end {
        Value::IntervalEnd(end, IntervalType::Closed) => self.add_interval(variable, *start, *end, true),
        other => Err(err_invalid_range_end(&other.to_string())),
      },
      other => Err(err_invalid_range_start(&other.to_string())),
    }
  }

  pub fn add_list(&mut self, variable: Name, value: Value) {
    self.states.push(ListState::init(variable, value));
  }

  pub fn add_variable(&mut self, variable: Name, bound_variable: Name) {
    self.states.push(VariableState::init(variable, bound_variable));
  }

  /// Iterates over all iteration states (tests; the evaluators use [FeelIterator::iterate_while]).
  #[cfg(test)]
  pub fn iterate<F>(&mut self, mut handler: F)
  where
    F: FnMut(&FeelContext),
  {
    self.iterate_while(|ctx| {
      handler(ctx);
      ControlFlow::Continue(())
    });
  }

  /// Iterates over all iteration states; `handler` gets the iteration context of every
  /// iteration and can stop the iteration with [ControlFlow::Break].
  ///
  /// The iteration context is the same object in every iteration (the states set their values
  /// in it), so the handler may move it out and back, but must put it back.
  pub fn iterate_while<F>(&mut self, mut handler: F)
  where
    F: FnMut(&mut FeelContext) -> ControlFlow<()>,
  {
    if self.states.is_empty() {
      return;
    }
    let mut ctx = FeelContext::new();
    let last_state_index = self.states.len() - 1;
    loop {
      for state in self.states.iter_mut().rev().filter(|state| !state.is_variable()) {
        state.set_value(&mut ctx);
      }
      for state in self.states.iter_mut().filter(|state| state.is_variable()) {
        state.bind_value(&ctx);
        state.set_value(&mut ctx);
      }
      if !self.states.iter().any(|state| state.is_empty()) && handler(&mut ctx).is_break() {
        return;
      }
      // advance like an odometer, the last state fastest; `next` returns what `has_next` would
      // (tests: `iterate_equals_previous_loop`), so the outermost state ends the iteration
      for (state_index, state) in self.states.iter_mut().rev().enumerate() {
        if state.next() {
          break;
        } else if state_index == last_state_index {
          return;
        }
      }
    }
  }
}

/// How the body of a `for`, `some` or `every` expression gets the iteration context.
#[derive(Clone, Copy, PartialEq)]
enum IterationContext {
  /// A copy in every iteration: what the body writes into the context on the top of the scope
  /// (boxed contexts do, model-evaluator) does not reach the next iteration.
  Copied,
  /// The iteration context itself, moved into the scope and back: for FEEL expressions, whose
  /// bodies never write into the context on the top of the scope.
  Moved,
}

/// Evaluates `evaluator` with the iteration context `ctx` on the top of the scope.
/// `prepare` runs on the context pushed, `finish` on the context popped.
///
/// `None` when the evaluation removed the iteration context from the scope (it popped more
/// contexts than it pushed, which no evaluator does): the iteration can not go on correctly, and
/// the caller returns null instead of a result that misses values.
fn evaluate_in_iteration_context(
  scope: &FeelScope,
  ctx: &mut FeelContext,
  mode: IterationContext,
  evaluator: &Evaluator,
  prepare: impl FnOnce(&mut FeelContext),
  finish: impl FnOnce(&mut FeelContext),
) -> Option<Value> {
  let mut iteration_context = match mode {
    IterationContext::Copied => ctx.clone(),
    IterationContext::Moved => std::mem::take(ctx),
  };
  prepare(&mut iteration_context);
  let depth = scope.depth();
  scope.push(iteration_context);
  let value = evaluator(scope);
  let mut iteration_context = scope.pop_frame(depth)?;
  finish(&mut iteration_context);
  if mode == IterationContext::Moved {
    *ctx = iteration_context;
  }
  Some(value)
}

/// The null returned when an iteration context was removed from the scope (see [evaluate_in_iteration_context]).
#[cold]
#[inline(never)]
fn lost_iteration_context(expression: &str) -> Value {
  value_null!("internal error: the evaluation removed the iteration context of a `{}` expression", expression)
}

/// `for` expression evaluator.
pub struct ForExpressionEvaluator {
  iterator: FeelIterator,
  partial: Name,
  mode: IterationContext,
}

impl Default for ForExpressionEvaluator {
  /// Implements [Default] trait for [ForExpressionEvaluator].
  fn default() -> Self {
    Self::new()
  }
}

impl ForExpressionEvaluator {
  /// Creates a new `for` expression evaluator; its body gets a copy of the iteration context
  /// in every iteration (see [ForExpressionEvaluator::for_feel]).
  pub fn new() -> Self {
    Self {
      iterator: FeelIterator::default(),
      partial: "partial".into(),
      mode: IterationContext::Copied,
    }
  }

  /// Creates a `for` expression evaluator for a FEEL `for` expression: the iteration context is
  /// moved into the scope and back in every iteration instead of copied, because a FEEL body never
  /// writes into the context on the top of the scope.
  pub fn for_feel() -> Self {
    Self {
      mode: IterationContext::Moved,
      ..Self::new()
    }
  }

  /// Adds an interval of values to iterate over.
  pub fn add_interval(&mut self, name: Name, start: Value, end: Value) -> Result<()> {
    self.iterator.add_interval(name, start, end, false)
  }

  /// Adds a range of values to iterate over.
  pub fn add_range(&mut self, name: Name, start: Value, end: Value) -> Result<()> {
    self.iterator.add_range(name, start, end)
  }

  /// Adds a list of elements to iterate over.
  pub fn add_list(&mut self, name: Name, value: Value) {
    self.iterator.add_list(name, value);
  }

  pub fn add_variable(&mut self, name: Name, variable: Name) {
    self.iterator.add_variable(name, variable);
  }

  /// Evaluates the `for` expression: the list of the values of `evaluator` in every iteration.
  pub fn evaluate(&mut self, scope: &FeelScope, evaluator: &Evaluator) -> Value {
    let mut results = vec![];
    let mut complete = true;
    let (partial, mode) = (&self.partial, self.mode);
    self.iterator.iterate_while(|ctx| {
      // The implicit `partial` variable holds the results of all previous iterations.
      // Move them into the iteration context and back out afterwards instead of
      // cloning the whole list on every iteration, which made `for` quadratic.
      let mut previous_results = None;
      let Some(iteration_value) = evaluate_in_iteration_context(
        scope,
        ctx,
        mode,
        evaluator,
        |iteration_context| iteration_context.assign_entry(partial, Value::List(std::mem::take(&mut results))),
        |iteration_context| {
          // the entry stays (with a placeholder) in a moved context; the next iteration sets it again
          if let Some(entry) = iteration_context.get_entry_mut(partial)
            && let Value::List(values) = std::mem::replace(entry, Value::Null(None))
          {
            previous_results = Some(values);
          }
        },
      ) else {
        complete = false;
        return ControlFlow::Break(());
      };
      if let Some(values) = previous_results {
        results = values;
      }
      results.push(iteration_value);
      ControlFlow::Continue(())
    });
    if complete { Value::List(results) } else { lost_iteration_context("for") }
  }
}

/// `some` expression evaluator.
pub struct SomeExpressionEvaluator {
  iterator: FeelIterator,
  mode: IterationContext,
}

impl Default for SomeExpressionEvaluator {
  /// Implements [Default] trait for [SomeExpressionEvaluator].
  fn default() -> Self {
    Self::new()
  }
}

impl SomeExpressionEvaluator {
  /// Creates a new `some` expression evaluator; its body gets a copy of the iteration context
  /// in every iteration.
  pub fn new() -> Self {
    Self {
      iterator: FeelIterator::new(),
      mode: IterationContext::Copied,
    }
  }

  /// Creates a `some` expression evaluator for a FEEL `some` expression (see [ForExpressionEvaluator::for_feel]).
  pub fn for_feel() -> Self {
    Self {
      iterator: FeelIterator::new(),
      mode: IterationContext::Moved,
    }
  }

  pub fn add_list(&mut self, name: Name, value: Value) {
    self.iterator.add_list(name, value);
  }

  pub fn evaluate(&mut self, scope: &FeelScope, evaluator: &Evaluator) -> Value {
    let mut result = VALUE_FALSE;
    let mode = self.mode;
    // the result is decided by the first `true` or non-boolean value
    self
      .iterator
      .iterate_while(|ctx| match evaluate_in_iteration_context(scope, ctx, mode, evaluator, |_| {}, |_| {}) {
        Some(Value::Boolean(true)) => {
          result = VALUE_TRUE;
          ControlFlow::Break(())
        }
        Some(Value::Boolean(false)) => ControlFlow::Continue(()),
        Some(_) => {
          result = value_null!();
          ControlFlow::Break(())
        }
        None => {
          result = lost_iteration_context("some");
          ControlFlow::Break(())
        }
      });
    result
  }
}

/// `every` expression evaluator.
pub struct EveryExpressionEvaluator {
  iterator: FeelIterator,
  mode: IterationContext,
}

impl Default for EveryExpressionEvaluator {
  /// Implements [Default] trait for [EveryExpressionEvaluator].
  fn default() -> Self {
    Self::new()
  }
}

impl EveryExpressionEvaluator {
  /// Creates a new `every` expression evaluator; its body gets a copy of the iteration context
  /// in every iteration.
  pub fn new() -> Self {
    Self {
      iterator: FeelIterator::new(),
      mode: IterationContext::Copied,
    }
  }

  /// Creates an `every` expression evaluator for a FEEL `every` expression (see [ForExpressionEvaluator::for_feel]).
  pub fn for_feel() -> Self {
    Self {
      iterator: FeelIterator::new(),
      mode: IterationContext::Moved,
    }
  }

  pub fn add_list(&mut self, name: Name, value: Value) {
    self.iterator.add_list(name, value);
  }

  pub fn evaluate(&mut self, scope: &FeelScope, evaluator: &Evaluator) -> Value {
    let mut result = VALUE_TRUE;
    let mode = self.mode;
    // the result is decided by the first `false` or non-boolean value
    self
      .iterator
      .iterate_while(|ctx| match evaluate_in_iteration_context(scope, ctx, mode, evaluator, |_| {}, |_| {}) {
        Some(Value::Boolean(false)) => {
          result = VALUE_FALSE;
          ControlFlow::Break(())
        }
        Some(Value::Boolean(true)) => ControlFlow::Continue(()),
        Some(_) => {
          result = value_null!();
          ControlFlow::Break(())
        }
        None => {
          result = lost_iteration_context("every");
          ControlFlow::Break(())
        }
      });
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::str::FromStr;

  /// The iteration loop before `iterate` relied on `next` alone: `has_next` decided the end.
  fn iterate_previous(iterator: &mut FeelIterator, mut handler: impl FnMut(&FeelContext)) {
    if iterator.states.is_empty() {
      return;
    }
    let mut ctx = FeelContext::new();
    let last_state_index = iterator.states.len() - 1;
    loop {
      for state in iterator.states.iter_mut().rev().filter(|state| !state.is_variable()) {
        state.set_value(&mut ctx);
      }
      for state in iterator.states.iter_mut().filter(|state| state.is_variable()) {
        state.bind_value(&ctx);
        state.set_value(&mut ctx);
      }
      if !iterator.states.iter().any(|state| state.is_empty()) {
        handler(&ctx);
      }
      for (state_index, state) in iterator.states.iter_mut().rev().enumerate() {
        if state_index == last_state_index && !state.has_next() {
          return;
        }
        if state.next() {
          break;
        }
      }
    }
  }

  fn number(text: &str) -> Value {
    Value::Number(FeelNumber::from_str(text).unwrap())
  }

  fn date(text: &str) -> Value {
    Value::Date(FeelDate::from_str(text).unwrap())
  }

  fn list(items: &[i64]) -> Value {
    Value::List(items.iter().map(|i| Value::Number(FeelNumber::from(*i))).collect())
  }

  #[test]
  fn iterate_equals_previous_loop() {
    // one iteration state each: (variable, start, end) intervals, lists, variables bound to a previous list
    enum S {
      Interval(&'static str, Value, Value),
      List(&'static str, Value),
      Variable(&'static str, &'static str),
    }
    let single = || {
      vec![
        S::Interval("i", number("1"), number("5")),
        S::Interval("i", number("5"), number("1")),
        S::Interval("i", number("3"), number("3")),
        S::Interval("i", number("1.5"), number("4")),
        S::Interval("i", number("-1"), number("1")),
        S::Interval("i", number("1e34"), number("1e34")),
        S::Interval("i", number("9999999999999999999999999999999990"), number("1e34")),
        S::Interval("i", date("2020-02-27"), date("2020-03-02")),
        S::Interval("i", date("2020-01-03"), date("2020-01-01")),
        S::Interval("i", date("2020-01-01"), date("2020-01-01")),
        S::List("x", list(&[])),
        S::List("x", list(&[7])),
        S::List("x", list(&[1, 2, 3])),
        S::List("x", Value::List(vec![list(&[1, 2]), list(&[]), list(&[3])])),
        S::List("x", Value::String("single".to_string())),
      ]
    };
    let build = |states: &[&S]| {
      let mut iterator = FeelIterator::new();
      for state in states {
        match state {
          S::Interval(name, start, end) => iterator.add_interval(Name::from(*name), start.clone(), end.clone(), false).unwrap(),
          S::List(name, value) => iterator.add_list(Name::from(*name), value.clone()),
          S::Variable(name, bound) => iterator.add_variable(Name::from(*name), Name::from(*bound)),
        }
      }
      iterator
    };
    let configurations = single();
    let renamed = |name: &'static str| {
      single()
        .into_iter()
        .map(|s| match s {
          S::Interval(_, a, b) => S::Interval(name, a, b),
          S::List(_, v) => S::List(name, v),
          v => v,
        })
        .collect::<Vec<S>>()
    };
    let second = renamed("j");
    let variables = [S::Variable("y", "x"), S::Variable("z", "y")];
    let mut cases: Vec<Vec<&S>> = vec![];
    for a in &configurations {
      cases.push(vec![a]);
      for b in &second {
        cases.push(vec![a, b]);
        cases.push(vec![b, a]);
      }
      if let S::List(..) = a {
        cases.push(vec![a, &variables[0]]);
        cases.push(vec![a, &variables[0], &variables[1]]);
        for b in &second {
          cases.push(vec![a, &variables[0], b]);
        }
      }
    }
    let mut iterations = 0;
    for states in &cases {
      let mut expected = vec![];
      iterate_previous(&mut build(states), |ctx| expected.push(ctx.to_string()));
      let mut actual = vec![];
      build(states).iterate(|ctx| actual.push(ctx.to_string()));
      assert_eq!(expected, actual);
      iterations += actual.len();
      // stopping early stops after the iteration that returned Break
      let mut seen = 0;
      build(states).iterate_while(|_| {
        seen += 1;
        if seen == 2 { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
      });
      assert_eq!(expected.len().min(2), seen);
    }
    assert!(cases.len() > 400 && iterations > 3000, "{} cases, {iterations} iterations", cases.len());
  }
}
