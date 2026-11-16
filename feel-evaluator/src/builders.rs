//! # Evaluator builders for FEEL expressions

use crate::FilterExpressionEvaluator;
use crate::bfeel::{self, Dialect};
use crate::invocation::{Arguments, guarded, invoke};
use crate::iterations::{Domain, Iteration, Sequence};
use crate::kernels::{self, Arithmetic};
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::arg;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, FeelType, FunctionBody, IntervalType, Name, value_null};
use rheon_feel_parser::{AstNode, ClosureBuilder};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// Wraps a function invocation evaluator: the call depth limit, finite results and results nested
/// no deeper than values may nest ([guarded]).
fn guard_invocation(evaluator: Evaluator) -> Evaluator {
  Box::new(move |scope: &FeelScope| guarded(|| evaluator(scope)))
}

/// Builds an evaluator for the expression in the given dialect (FEEL or B-FEEL, DMN 1.6 chapter 11).
pub fn build_evaluator_with_dialect(node: &AstNode, dialect: Dialect) -> Evaluator {
  let mut evaluator_builder = EvaluatorBuilder { dialect };
  evaluator_builder.build(node)
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
        AstNode::Add(lhs, rhs) => Some((Arithmetic::Addition, lhs, rhs)),
        AstNode::Sub(lhs, rhs) => Some((Arithmetic::Subtraction, lhs, rhs)),
        AstNode::Mul(lhs, rhs) => Some((Arithmetic::Multiplication, lhs, rhs)),
        AstNode::Div(lhs, rhs) => Some((Arithmetic::Division, lhs, rhs)),
        AstNode::Exp(lhs, rhs) => Some((Arithmetic::Exponentiation, lhs, rhs)),
        _ => None,
      };
      if let Some((operator, lhs, rhs)) = operator {
        let lhe = self.build(lhs);
        let rhe = self.build(rhs);
        return Box::new(move |scope: &FeelScope| {
          let lhv = lhe(scope);
          let rhv = rhe(scope);
          bfeel::arithmetic(operator, lhv, rhv)
        });
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
    // function calls never yield non-finite numbers and are subject to the call depth limit
    let evaluator = match node {
      AstNode::FunctionInvocation(..) => guard_invocation(evaluator),
      _ => evaluator,
    };
    // return the evaluator
    evaluator
  }

  /// Returns an evaluator for `addition`.
  fn build_add(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    binary(Arithmetic::Addition, self.build(lhs), self.build(rhs))
  }

  /// Returns an evaluator of the temporal expression after `@` (at) literal.
  /// Builds an `@` literal. Its value is parsed once, here; a literal whose fraction digits
  /// beyond nanoseconds were truncated warns on each evaluation, since diagnostics are recorded
  /// while an evaluation runs (a model built once reports in every evaluation).
  fn build_at(&mut self, text: &str) -> Evaluator {
    match kernels::at_literal(text) {
      Ok((value, None)) => Box::new(move |_: &FeelScope| value.clone()),
      Ok((value, Some(warning))) => Box::new(move |_: &FeelScope| {
        diagnostics::warn(&warning);
        value.clone()
      }),
      Err(invalid_at_literal) => Box::new(move |_: &FeelScope| value_null!(invalid_at_literal)),
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
      kernels::between(lhv, mhv, rhv)
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
  ///
  /// The right operand is evaluated only when the left one is not `false`, which decides the
  /// result whatever the right one is: the errors and warnings of a right operand that cannot
  /// change the result are not recorded.
  fn build_and(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| match lhe(scope) {
      Value::Boolean(false) => Value::Boolean(false),
      Value::Boolean(true) => match rhe(scope) {
        rhv @ Value::Boolean(_) => rhv,
        _ => value_null!(),
      },
      _ => match rhe(scope) {
        Value::Boolean(false) => Value::Boolean(false),
        _ => value_null!(),
      },
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
          match kernels::context_literal_entry(&evaluated_ctx, evaluator(scope)) {
            Ok((name, value)) => {
              // add newly evaluated entry to evaluated context
              evaluated_ctx.set_entry(&name, value.clone());
              // add newly evaluated entry to special context
              scope.set_value(&name, value);
            }
            Err(null) => break 'entries Some(null),
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
      kernels::context_entry(lhv, rhv)
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
        if let Err(null) = kernels::add_context_type_entry(&mut entries, evaluator(scope)) {
          return null;
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
      kernels::context_type_entry(lhv, rhv)
    })
  }

  fn build_context_type_entry_key(&mut self, lhs: &Name) -> Evaluator {
    let name = lhs.clone();
    Box::new(move |_: &FeelScope| Value::ContextTypeEntryKey(name.clone()))
  }

  fn build_div(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    binary(Arithmetic::Division, self.build(lhs), self.build(rhs))
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
    binary(Arithmetic::Exponentiation, self.build(lhs), self.build(rhs))
  }

  fn build_feel_type(&mut self, lhs: &FeelType) -> Evaluator {
    let feel_type = lhs.clone();
    Box::new(move |_: &FeelScope| Value::FeelType(feel_type.clone()))
  }

  fn build_filter(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let filter_expression_evaluator = FilterExpressionEvaluator::new();
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
      kernels::formal_parameter(lhv, rhv)
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
        if let Err(null) = kernels::add_formal_parameter(&mut formal_parameters, evaluator(scope)) {
          return null;
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
    Box::new(move |scope: &FeelScope| kernels::external_body(lhe(scope)))
  }

  fn build_function_definition(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let closure = ClosureBuilder::from_function_definition(lhs, rhs);
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      kernels::function_definition(lhv, rhv, &closure, |name| scope.search_entry(name))
    })
  }

  fn build_eq(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| kernels::equal(&lhe(scope), &rhe(scope)))
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
      kernels::function_type(lhv, rhv)
    })
  }

  fn build_ge(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      kernels::compare_ge(lhv, rhv)
    })
  }

  fn build_gt(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      kernels::compare_gt(lhv, rhv)
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
      kernels::in_operator(lhv, rhv)
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
      kernels::instance_of(lhv, rhv)
    })
  }

  fn build_le(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      kernels::compare_le(lhv, rhv)
    })
  }

  fn build_lt(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      kernels::compare_lt(lhv, rhv)
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
      kernels::list_of(values)
    })
  }

  fn build_list_type(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| kernels::list_type(lhe(scope)))
  }

  fn build_mul(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    binary(Arithmetic::Multiplication, self.build(lhs), self.build(rhs))
  }

  fn build_name(&mut self, name: Name) -> Evaluator {
    Box::new(move |scope: &FeelScope| scope.get_value(&name).unwrap_or_else(|| kernels::free_name(&name)))
  }

  /// Returns an evaluator for the arithmetic negation `-e`, the one unary arithmetic operator,
  /// whose null operand is an error as those of the binary ones are ([Arithmetic::negation]).
  fn build_neg(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| Arithmetic::negation(lhe(scope)))
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
    match kernels::number_literal(before, after, sign, exponent) {
      Ok(num) => Box::new(move |_: &FeelScope| Value::Number(num)),
      Err(text) => Box::new(move |_: &FeelScope| value_null!(text)),
    }
  }

  /// Returns an evaluator for `!=`: DMN 1.6 §10.3.2.15, "By definition, FEEL(e1 != e2) is
  /// FEEL(not(e 1= e2))", so the operands that `=` cannot compare give its null and its error.
  fn build_nq(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| kernels::not_equal(&lhe(scope), &rhe(scope)))
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
  ///
  /// The right operand is evaluated only when the left one is not `true`, which decides the
  /// result whatever the right one is: the errors and warnings of a right operand that cannot
  /// change the result are not recorded.
  fn build_or(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| match lhe(scope) {
      Value::Boolean(true) => Value::Boolean(true),
      Value::Boolean(false) => match rhe(scope) {
        rhv @ Value::Boolean(_) => rhv,
        _ => value_null!(),
      },
      _ => match rhe(scope) {
        Value::Boolean(true) => Value::Boolean(true),
        _ => value_null!(),
      },
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
      scope.search(&names).unwrap_or_else(kernels::free_qualified_name)
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
    Box::new(move |scope: &FeelScope| kernels::descendants(lhe(scope), &name))
  }

  fn build_path(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let AstNode::Name(name) = rhs else {
      return build_err_msg(err_expected_name(rhs));
    };
    let name = name.clone();
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| kernels::path(lhe(scope), &name))
  }

  fn build_range(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    let rhe = self.build(rhs);
    Box::new(move |scope: &FeelScope| {
      let lhv = lhe(scope);
      let rhv = rhe(scope);
      kernels::range_of(lhv, rhv)
    })
  }

  fn build_range_type(&mut self, lhs: &AstNode) -> Evaluator {
    let lhe = self.build(lhs);
    Box::new(move |scope: &FeelScope| kernels::range_type(lhe(scope)))
  }

  fn build_some(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    self.build_quantified(lhs, rhs, Iteration::some)
  }

  fn build_string(&mut self, lhs: &str) -> Evaluator {
    let value = Value::String(lhs.into());
    Box::new(move |_: &FeelScope| value.clone())
  }

  fn build_sub(&mut self, lhs: &AstNode, rhs: &AstNode) -> Evaluator {
    binary(Arithmetic::Subtraction, self.build(lhs), self.build(rhs))
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

/// An evaluator of the binary arithmetic `operator` on the values of `lhe` and `rhe`, evaluated in
/// that order ([Arithmetic::apply]).
fn binary(operator: Arithmetic, lhe: Evaluator, rhe: Evaluator) -> Evaluator {
  Box::new(move |scope: &FeelScope| {
    let lhv = lhe(scope);
    let rhv = rhe(scope);
    operator.apply(lhv, rhv)
  })
}

/// The iteration of a `for`, `some` or `every` expression over `variables`, in order. A variable
/// named twice builds an evaluator of null instead: the later binding would hide the earlier one.
fn iteration(variables: Vec<(Name, Domain)>) -> std::result::Result<Iteration, Evaluator> {
  let mut names = HashSet::new();
  if let Some((name, _)) = variables.iter().find(|(name, _)| !names.insert(name)) {
    let name = name.clone();
    return Err(Box::new(move |_: &FeelScope| value_null!("duplicate iteration variable '{}'", arg(&name))));
  }
  Ok(Iteration::new(variables))
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
