use crate::values::Value;
use crate::{FeelScope, FunctionBody, value_number};
use std::sync::Arc;

#[test]
#[allow(clippy::redundant_clone)]
fn test_function_body_context() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Context(Arc::new(Box::new(|_: &FeelScope| value_number!(1))));
  assert_eq!(value_number!(1), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyContext", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Context(Arc::new(Box::new(|_: &FeelScope| value_number!(10))));
  assert_eq!(fun_body, fun_body_a);
  let fun_body_clone = fun_body.clone();
  assert_eq!(fun_body, fun_body_clone);
  assert_eq!(value_number!(1), fun_body_clone.evaluate(scope));
  assert_eq!("FunctionBodyContext", format!("{fun_body_clone:?}"))
}

#[test]
fn test_function_body_literal_expression() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(2))));
  assert_eq!(value_number!(2), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyLiteralExpression", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(20))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_decision_table() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::DecisionTable(Arc::new(Box::new(|_: &FeelScope| value_number!(3))));
  assert_eq!(value_number!(3), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyDecisionTable", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::DecisionTable(Arc::new(Box::new(|_: &FeelScope| value_number!(30))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_function_definition() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::FunctionDefinition(Arc::new(Box::new(|_: &FeelScope| value_number!(4))));
  assert_eq!(value_number!(4), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyFunctionDefinition", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::FunctionDefinition(Arc::new(Box::new(|_: &FeelScope| value_number!(40))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_invocation() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Invocation(Arc::new(Box::new(|_: &FeelScope| value_number!(5))));
  assert_eq!(value_number!(5), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyInvocation", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Invocation(Arc::new(Box::new(|_: &FeelScope| value_number!(50))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_relation() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Relation(Arc::new(Box::new(|_: &FeelScope| value_number!(6))));
  assert_eq!(value_number!(6), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyRelation", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Relation(Arc::new(Box::new(|_: &FeelScope| value_number!(60))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_decision_service() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::DecisionService(Arc::new(Box::new(|_: &FeelScope| value_number!(7))));
  assert_eq!(value_number!(7), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyDecisionService", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::DecisionService(Arc::new(Box::new(|_: &FeelScope| value_number!(70))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_external() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::External(Arc::new(Box::new(|_: &FeelScope| value_number!(8))));
  assert_eq!(value_number!(8), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyExternal", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::External(Arc::new(Box::new(|_: &FeelScope| value_number!(80))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_conditional() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Conditional(Arc::new(Box::new(|_: &FeelScope| value_number!(9))));
  assert_eq!(value_number!(9), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyConditional", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Conditional(Arc::new(Box::new(|_: &FeelScope| value_number!(90))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_every() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Every(Arc::new(Box::new(|_: &FeelScope| value_number!(10))));
  assert_eq!(value_number!(10), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyEvery", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Every(Arc::new(Box::new(|_: &FeelScope| value_number!(100))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_filter() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Filter(Arc::new(Box::new(|_: &FeelScope| value_number!(11))));
  assert_eq!(value_number!(11), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyFilter", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Filter(Arc::new(Box::new(|_: &FeelScope| value_number!(110))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_for() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::For(Arc::new(Box::new(|_: &FeelScope| value_number!(12))));
  assert_eq!(value_number!(12), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyFor", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::For(Arc::new(Box::new(|_: &FeelScope| value_number!(120))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_list() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::List(Arc::new(Box::new(|_: &FeelScope| value_number!(13))));
  assert_eq!(value_number!(13), fun_body.evaluate(scope));
  assert_eq!("FunctionBodyList", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::List(Arc::new(Box::new(|_: &FeelScope| value_number!(130))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

#[test]
fn test_function_body_some() {
  let scope = &FeelScope::default();
  let fun_body = FunctionBody::Some(Arc::new(Box::new(|_: &FeelScope| value_number!(14))));
  assert_eq!(value_number!(14), fun_body.evaluate(scope));
  assert_eq!("FunctionBodySome", format!("{fun_body:?}"));
  let fun_body_a = FunctionBody::Some(Arc::new(Box::new(|_: &FeelScope| value_number!(140))));
  assert_eq!(fun_body, fun_body_a);
  assert_eq!(fun_body, fun_body.clone());
}

/// Code whose bodies give a number plus their index.
struct Code(i32);

impl crate::FeelCode for Code {
  fn evaluate(self: Arc<Self>, body: u32, _: &FeelScope) -> Value {
    value_number!(self.0 + body as i32)
  }

  fn into_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
    self
  }
}

#[test]
fn a_feel_body_compares_and_prints_like_a_literal_expression() {
  let scope = &FeelScope::default();
  let feel = FunctionBody::Feel(Arc::new(Code(3)), 1);
  let literal = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(4))));
  assert_eq!(value_number!(4), feel.evaluate(scope));
  assert_eq!("FunctionBodyLiteralExpression", format!("{feel:?}"));
  assert_eq!(feel, literal);
  assert_eq!(literal, feel);
  assert_eq!(feel, FunctionBody::Feel(Arc::new(Code(5)), 0));
  assert_ne!(feel, FunctionBody::Context(Arc::new(Box::new(|_: &FeelScope| value_number!(3)))));
  let FunctionBody::Feel(code, _) = &feel else { unreachable!() };
  assert!(Arc::clone(code).into_any().downcast::<Code>().is_ok());
}

/// A home may hold a function that holds the home; the end of the evaluation empties the
/// homes it made, which frees them. An entry has its first value only.
#[test]
fn test_homes_are_emptied_when_their_evaluation_ends() {
  use crate::closure::Closure;
  use crate::context::FeelContext;
  use crate::{Evaluator, FeelType, FunctionDefinition, Home, Name};
  use std::sync::Arc;

  // (a thread of its own: the homes of an evaluation are those of its thread)
  std::thread::spawn(|| {
    let body = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &crate::FeelScope| Value::Null(None)) as Evaluator));
    let function = FunctionDefinition::new(vec![], body, false, Closure::default(), FeelContext::default(), FeelType::Any);
    let parent = Home::new(Box::new([Name::from("outer")]), None);
    let home = Home::new(Box::new([Name::from("f"), Name::from("k"), Name::from("later")]), Some(Arc::clone(&parent)));
    // a function with a home compares and prints as the one without
    let at_home = function.at_home(Arc::clone(&home));
    assert_eq!(function, at_home);
    assert_eq!(format!("{function:?}"), format!("{at_home:?}"));
    // the home holds the function, which holds the home
    home.set(0, Value::FunctionDefinition(Arc::new(at_home)));
    home.set(1, Value::Boolean(true));
    home.set(1, Value::Boolean(false));
    assert_eq!(Some(Value::Boolean(true)), home.entry(&Name::from("k")));
    assert!(matches!(home.entry(&Name::from("f")), Some(Value::FunctionDefinition(f)) if f.home.is_some()));
    assert_eq!(None, home.entry(&Name::from("later")));
    assert_eq!(None, home.entry(&Name::from("outer")));
    assert!(Arc::ptr_eq(&parent, home.parent().unwrap()));
    let (weak, weak_parent) = (Arc::downgrade(&home), Arc::downgrade(&parent));
    drop(home);
    drop(parent);
    // still alive: the home and its function hold each other
    assert!(weak.upgrade().is_some());
    Home::end_evaluation();
    assert!(weak.upgrade().is_none());
    assert!(weak_parent.upgrade().is_none());
    // a home that something still holds is emptied
    let home = Home::new(Box::new([Name::from("k")]), None);
    home.set(0, Value::Boolean(true));
    Home::end_evaluation();
    assert_eq!(None, home.entry(&Name::from("k")));
  })
  .join()
  .unwrap();
}
