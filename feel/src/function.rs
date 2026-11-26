//! `FEEL` or `DMN` function definitions and bodies.

use crate::closure::Closure;
use crate::context::FeelContext;
use crate::values::Value;
use crate::{Evaluator, FeelScope, FeelType, Name};
use std::cell::RefCell;
use std::fmt;
use std::fmt::Debug;
use std::sync::{Arc, RwLock, Weak};

/// The entries of a context literal as the functions written in it see them: every entry of
/// the context is in scope for the body of such a function (DMN 1.6 §10.3.2.11.1), also one
/// that comes after the function, from the moment it has its value. One per evaluation of the
/// literal; the functions hold it.
///
/// A home can hold a value that holds the home (a function written in the literal, or a value
/// with one inside), so homes are not freed by their counts alone: every home is emptied when
/// the evaluation that made it ends ([Home::end_evaluation]), which frees them all. A function
/// that outlives its evaluation keeps what it closed over where it was written, and no longer
/// sees the entries that came after it.
pub struct Home {
  names: Box<[Name]>,
  /// The value of each entry once it has one.
  values: RwLock<Box<[Option<Value>]>>,
  /// The home of the context literal or of the function that this literal is written in.
  parent: Option<Arc<Home>>,
}

thread_local! {
  /// The homes made by the evaluation that runs on this thread.
  static HOMES: RefCell<Vec<Weak<Home>>> = const { RefCell::new(Vec::new()) };
}

impl Home {
  /// A home of the entries `names`, none of which has a value yet.
  pub fn new(names: Box<[Name]>, parent: Option<Arc<Home>>) -> Arc<Self> {
    let values = names.iter().map(|_| None).collect();
    let home = Arc::new(Self {
      names,
      values: RwLock::new(values),
      parent,
    });
    HOMES.with(|homes| homes.borrow_mut().push(Arc::downgrade(&home)));
    home
  }

  /// Gives the entry `index` its value, once.
  pub fn set(&self, index: usize, value: Value) {
    if let Ok(mut values) = self.values.write()
      && let Some(slot) = values.get_mut(index)
      && slot.is_none()
    {
      *slot = Some(value);
    }
  }

  /// The value of the entry `name`, when it is an entry that has its value.
  pub fn entry(&self, name: &Name) -> Option<Value> {
    let index = self.names.iter().position(|entry| entry == name)?;
    self.values.read().ok()?.get(index)?.clone()
  }

  /// The home around this one.
  pub fn parent(&self) -> Option<&Arc<Home>> {
    self.parent.as_ref()
  }

  /// Empties the homes that the evaluation of this thread made: called when it ends.
  pub fn end_evaluation() {
    let homes = HOMES.with(|homes| std::mem::take(&mut *homes.borrow_mut()));
    for home in homes {
      if let Some(home) = home.upgrade() {
        // the values are dropped after the lock: one of them may hold this home
        let values = home.values.write().map(|mut values| std::mem::take(&mut *values));
        drop(values);
      }
    }
  }
}

/// A function: a FEEL function definition or a function built from a DMN model
/// (the content of [Value::FunctionDefinition], shared: copying the value copies a pointer).
#[derive(Clone)]
pub struct FunctionDefinition {
  /// Formal parameters of the function.
  parameters: Vec<(Name, FeelType)>,
  /// Body of the function.
  pub body: FunctionBody,
  /// Flag indicating if the function's body is an external function, `true` == external.
  pub external: bool,
  /// Closed names from function context (closure names).
  pub closure: Closure,
  /// Values of the closed names (closure values), shared: a call puts them in scope without a
  /// copy, and a change copies them first when they are shared ([Arc::make_mut]).
  pub closure_ctx: Arc<FeelContext>,
  /// Return type of the function.
  result_type: FeelType,
  /// The entries of the context literal that the function is written in, for a FEEL function
  /// written in one. No part of what the function is: two functions that differ only in it
  /// are equal and print alike.
  pub home: Option<Arc<Home>>,
}

impl PartialEq for FunctionDefinition {
  fn eq(&self, other: &Self) -> bool {
    self.parameters == other.parameters
      && self.body == other.body
      && self.external == other.external
      && self.closure == other.closure
      && self.closure_ctx == other.closure_ctx
      && self.result_type == other.result_type
  }
}

impl Debug for FunctionDefinition {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FunctionDefinition")
      .field("parameters", &self.parameters)
      .field("body", &self.body)
      .field("external", &self.external)
      .field("closure", &self.closure)
      .field("closure_ctx", &self.closure_ctx)
      .field("result_type", &self.result_type)
      .finish()
  }
}

impl FunctionDefinition {
  /// This function with the home `home`.
  pub fn at_home(&self, home: Arc<Home>) -> Self {
    let mut function = self.clone();
    function.home = Some(home);
    function
  }

  /// Creates a function definition.
  pub fn new(parameters: Vec<(Name, FeelType)>, body: FunctionBody, external: bool, closure: Closure, closure_ctx: FeelContext, result_type: FeelType) -> Self {
    Self {
      parameters,
      body,
      external,
      closure,
      closure_ctx: Arc::new(closure_ctx),
      result_type,
      home: None,
    }
  }

  /// The formal parameters of the function, with their types, in their order.
  pub fn parameters(&self) -> &[(Name, FeelType)] {
    &self.parameters
  }

  /// The return type of the function.
  pub fn result_type(&self) -> &FeelType {
    &self.result_type
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

/// Code built by an evaluator of FEEL that holds the bodies of FEEL function definitions, each
/// by its index: the body of [FunctionBody::Feel]. The invocation of the evaluator that built it
/// runs a body directly; any other caller evaluates it like any other body.
pub trait FeelCode: std::any::Any + Send + Sync {
  /// Evaluates the body `body` in a scope that already holds the bound arguments, as
  /// [FunctionBody::evaluate] evaluates every body: the parameters are the values of their names
  /// in `scope`.
  fn evaluate(self: Arc<Self>, body: u32, scope: &FeelScope) -> Value;

  /// The code as [std::any::Any], so that the evaluator that built it can recognize it.
  fn into_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync>;
}

/// Function body may be defined multiple ways using FEEL or DMN.
/// This enum is the representation of all of these cases.
#[derive(Clone)]
pub enum FunctionBody {
  /// Function body created from boxed conditional (`if`) expression defined in DMN model.
  Conditional(FunctionBodyEvaluator),
  /// Function body created from context defined in DMN model.
  Context(FunctionBodyEvaluator),
  /// Function body created from decision service defined in DMN model. It holds only a weak
  /// reference to the model that holds the function value, so a value never holds a strong
  /// handle to the state that holds it; called after the model was dropped, it gives null.
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
  /// Function body of a FEEL function definition: code of an evaluator of FEEL and the index of
  /// the body in it. It compares and prints like [FunctionBody::LiteralExpression].
  Feel(Arc<dyn FeelCode>, u32),
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
      FunctionBody::Feel(code, body) => Arc::clone(code).evaluate(*body, scope),
    }
  }
}

impl FunctionBody {
  /// Whether two bodies are the same body: the same code, not only bodies of the same kind (the
  /// [PartialEq] of bodies compares their kinds). A FEEL body is the same when it is the same
  /// body of the same code; a body of a DMN model or an external function when it is the same
  /// evaluator.
  pub fn is_same(&self, other: &Self) -> bool {
    use FunctionBody as B;
    match (self, other) {
      (B::Feel(code1, body1), B::Feel(code2, body2)) => Arc::ptr_eq(code1, code2) && body1 == body2,
      (B::Conditional(e1), B::Conditional(e2))
      | (B::Context(e1), B::Context(e2))
      | (B::DecisionService(e1), B::DecisionService(e2))
      | (B::DecisionTable(e1), B::DecisionTable(e2))
      | (B::Every(e1), B::Every(e2))
      | (B::External(e1), B::External(e2))
      | (B::Filter(e1), B::Filter(e2))
      | (B::For(e1), B::For(e2))
      | (B::FunctionDefinition(e1), B::FunctionDefinition(e2))
      | (B::Invocation(e1), B::Invocation(e2))
      | (B::List(e1), B::List(e2))
      | (B::LiteralExpression(e1), B::LiteralExpression(e2))
      | (B::Relation(e1), B::Relation(e2))
      | (B::Some(e1), B::Some(e2)) => Arc::ptr_eq(e1, e2),
      _ => false,
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
      FunctionBody::LiteralExpression(_) | FunctionBody::Feel(..) => write!(f, "FunctionBodyLiteralExpression"),
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
      FunctionBody::LiteralExpression(_) | FunctionBody::Feel(..) => matches!(other, FunctionBody::LiteralExpression(_) | FunctionBody::Feel(..)),
      FunctionBody::Relation(_) => matches!(other, FunctionBody::Relation(_)),
      FunctionBody::Some(_) => matches!(other, FunctionBody::Some(_)),
    }
  }
}
