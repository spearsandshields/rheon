//! `FEEL` or `DMN` function definitions and bodies.

use crate::closure::Closure;
use crate::context::FeelContext;
use crate::values::Value;
use crate::{Evaluator, FeelScope, FeelType, Name};
use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

/// A function: a FEEL function definition or a function built from a DMN model
/// (the content of [Value::FunctionDefinition], shared: copying the value copies a pointer).
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDefinition {
  /// Formal parameters of the function.
  pub parameters: Vec<(Name, FeelType)>,
  /// Body of the function.
  pub body: FunctionBody,
  /// Flag indicating if the function's body is an external function, `true` == external.
  pub external: bool,
  /// Closed names from function context (closure names).
  pub closure: Closure,
  /// Values of the closed names (closure values).
  pub closure_ctx: FeelContext,
  /// Return type of the function.
  pub result_type: FeelType,
}

impl FunctionDefinition {
  /// Creates a function definition.
  pub fn new(parameters: Vec<(Name, FeelType)>, body: FunctionBody, external: bool, closure: Closure, closure_ctx: FeelContext, result_type: FeelType) -> Self {
    Self {
      parameters,
      body,
      external,
      closure,
      closure_ctx,
      result_type,
    }
  }
}

impl From<FunctionDefinition> for Value {
  /// Converts a function definition into a function value.
  fn from(function: FunctionDefinition) -> Self {
    Value::FunctionDefinition(Arc::new(function))
  }
}

/// Type alias of the closure that evaluates FEEL or DMN function body into [Value].
pub type FunctionBodyEvaluator = Arc<Evaluator>;

/// Function body may be defined multiple ways using FEEL or DMN.
/// This enum is the representation of all of these cases.
#[derive(Clone)]
pub enum FunctionBody {
  /// Function body created from boxed conditional (`if`) expression defined in DMN model.
  Conditional(FunctionBodyEvaluator),
  /// Function body created from context defined in DMN model.
  Context(FunctionBodyEvaluator),
  /// Function body created from decision service defined in DMN model.
  DecisionService(FunctionBodyEvaluator),
  /// Function body created from decision table defined in DMN model.
  DecisionTable(FunctionBodyEvaluator),
  /// Function body created from boxed `every` expression defined in DMN model.
  Every(FunctionBodyEvaluator),
  /// Function body created from externally defined function in Java or PMML.
  External(FunctionBodyEvaluator),
  /// Function body created from boxed `filter` expression defined in DMN model.
  Filter(FunctionBodyEvaluator),
  /// Function body created from boxed `for` loop defined in DMN model.
  For(FunctionBodyEvaluator),
  /// Function body created from function definition defined in DMN model.
  FunctionDefinition(FunctionBodyEvaluator),
  /// Function body created from invocation defined in DMN model.
  Invocation(FunctionBodyEvaluator),
  /// Function body created from list defined in DMN model.
  List(FunctionBodyEvaluator),
  /// Function body created from `FEEL` textual expression defined in DMN model.
  LiteralExpression(FunctionBodyEvaluator),
  /// Function body created from relation defined in DMN model.
  Relation(FunctionBodyEvaluator),
  /// Function body created from boxed `some` expression defined in DMN model.
  Some(FunctionBodyEvaluator),
}

impl FunctionBody {
  /// Evaluates the body in a scope that already holds the bound arguments; functions are invoked
  /// by the invocation of rheon-feel-evaluator, which binds them.
  pub fn evaluate(&self, scope: &FeelScope) -> Value {
    match self {
      FunctionBody::Conditional(evaluator) => evaluator(scope),
      FunctionBody::Context(evaluator) => evaluator(scope),
      FunctionBody::DecisionService(evaluator) => evaluator(scope),
      FunctionBody::DecisionTable(evaluator) => evaluator(scope),
      FunctionBody::Every(evaluator) => evaluator(scope),
      FunctionBody::External(evaluator) => evaluator(scope),
      FunctionBody::Filter(evaluator) => evaluator(scope),
      FunctionBody::For(evaluator) => evaluator(scope),
      FunctionBody::FunctionDefinition(evaluator) => evaluator(scope),
      FunctionBody::Invocation(evaluator) => evaluator(scope),
      FunctionBody::List(evaluator) => evaluator(scope),
      FunctionBody::LiteralExpression(evaluator) => evaluator(scope),
      FunctionBody::Relation(evaluator) => evaluator(scope),
      FunctionBody::Some(evaluator) => evaluator(scope),
    }
  }
}

impl Debug for FunctionBody {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FunctionBody::Conditional(_) => write!(f, "FunctionBodyConditional"),
      FunctionBody::Context(_) => write!(f, "FunctionBodyContext"),
      FunctionBody::DecisionService(_) => write!(f, "FunctionBodyDecisionService"),
      FunctionBody::DecisionTable(_) => write!(f, "FunctionBodyDecisionTable"),
      FunctionBody::Every(_) => write!(f, "FunctionBodyEvery"),
      FunctionBody::External(_) => write!(f, "FunctionBodyExternal"),
      FunctionBody::Filter(_) => write!(f, "FunctionBodyFilter"),
      FunctionBody::For(_) => write!(f, "FunctionBodyFor"),
      FunctionBody::FunctionDefinition(_) => write!(f, "FunctionBodyFunctionDefinition"),
      FunctionBody::Invocation(_) => write!(f, "FunctionBodyInvocation"),
      FunctionBody::List(_) => write!(f, "FunctionBodyList"),
      FunctionBody::LiteralExpression(_) => write!(f, "FunctionBodyLiteralExpression"),
      FunctionBody::Relation(_) => write!(f, "FunctionBodyRelation"),
      FunctionBody::Some(_) => write!(f, "FunctionBodySome"),
    }
  }
}

impl PartialEq for FunctionBody {
  fn eq(&self, other: &Self) -> bool {
    match self {
      FunctionBody::Conditional(_) => matches!(other, FunctionBody::Conditional(_)),
      FunctionBody::Context(_) => matches!(other, FunctionBody::Context(_)),
      FunctionBody::DecisionService(_) => matches!(other, FunctionBody::DecisionService(_)),
      FunctionBody::DecisionTable(_) => matches!(other, FunctionBody::DecisionTable(_)),
      FunctionBody::Every(_) => matches!(other, FunctionBody::Every(_)),
      FunctionBody::External(_) => matches!(other, FunctionBody::External(_)),
      FunctionBody::Filter(_) => matches!(other, FunctionBody::Filter(_)),
      FunctionBody::For(_) => matches!(other, FunctionBody::For(_)),
      FunctionBody::FunctionDefinition(_) => matches!(other, FunctionBody::FunctionDefinition(_)),
      FunctionBody::Invocation(_) => matches!(other, FunctionBody::Invocation(_)),
      FunctionBody::List(_) => matches!(other, FunctionBody::List(_)),
      FunctionBody::LiteralExpression(_) => matches!(other, FunctionBody::LiteralExpression(_)),
      FunctionBody::Relation(_) => matches!(other, FunctionBody::Relation(_)),
      FunctionBody::Some(_) => matches!(other, FunctionBody::Some(_)),
    }
  }
}
