use super::*;
use crate::bif::Bif;
use crate::closure::Closure;
use crate::context::FeelContext;
use crate::function::{FunctionBody, FunctionDefinition};
use crate::values::{Value, Values};
use crate::{FeelNumber, FeelScope, IntervalType, ToFeelString, value_null, value_number};
use dsntk_common::Jsonify;
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration};
use std::collections::BTreeMap;
use std::sync::Arc;

macro_rules! eq_dbg {
  ($expected:literal, $value: expr) => {
    assert_eq!($expected, format!("{:?}", $value));
  };
}

macro_rules! eq_dsp {
  ($expected:literal, $value: expr) => {
    assert_eq!($expected, format!("{}", $value));
  };
}

macro_rules! eq_typ {
  ($expected:expr, $value: expr) => {
    assert_eq!($expected, $value.type_of());
  };
}

#[test]
fn test_debug() {
  let name = Name::from("a");
  let t_number = FeelType::Number;
  let v_number = Value::Number(FeelNumber::new(1, 0));
  let b_number = Box::new(v_number);
  let v_date = FeelDate::new(2022, 9, 27).unwrap();
  let v_time = FeelTime::local_opt(12, 13, 23, 0).unwrap();
  let v_date_time = FeelDateTime::new(v_date.clone(), v_time.clone());
  let v_function_body = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(2))));
  let v_external = false;
  let v_days_and_time_duration = FeelDaysAndTimeDuration::from_s(100);
  let v_years_and_months_duration = FeelYearsAndMonthsDuration::from_ym(3, 2);
  let v_closure = Closure::default();
  let v_closure_ctx = FeelContext::default();
  eq_dbg!(r#"Boolean(false)"#, Value::Boolean(false));
  eq_dbg!(r#"BuiltInFunction(Time)"#, Value::BuiltInFunction(Bif::Time));
  eq_dbg!(r#"Context(FeelContext({}))"#, Value::Context(FeelContext::default()));
  eq_dbg!(r#"ContextEntry(Name("a"), Number(+1E+0))"#, Value::ContextEntry(name.clone(), b_number.clone()));
  eq_dbg!(r#"ContextEntryKey(Name("a"))"#, Value::ContextEntryKey(name.clone()));
  eq_dbg!(r#"ContextType(Number)"#, Value::ContextType(t_number.clone()));
  eq_dbg!(r#"ContextTypeEntry(Name("a"), Number)"#, Value::ContextTypeEntry(name.clone(), t_number.clone()));
  eq_dbg!(r#"ContextTypeEntryKey(Name("a"))"#, Value::ContextTypeEntryKey(name.clone()));
  eq_dbg!(r#"Date(FeelDate(2022, 9, 27))"#, Value::Date(v_date));
  eq_dbg!(
    r#"DateTime(FeelDateTime(FeelDate(2022, 9, 27), FeelTime(12, 13, 23, 0, Local)))"#,
    Value::DateTime(v_date_time)
  );
  eq_dbg!(
    r#"DaysAndTimeDuration(FeelDaysAndTimeDuration(100000000000))"#,
    Value::DaysAndTimeDuration(v_days_and_time_duration)
  );
  eq_dbg!(r#"ExpressionList([])"#, Value::ExpressionList(Values::default()));
  eq_dbg!(
    r#"ExternalJavaFunction("class", "method")"#,
    Value::ExternalJavaFunction("class".to_string(), "method".to_string())
  );
  eq_dbg!(
    r#"ExternalPmmlFunction("document", "model")"#,
    Value::ExternalPmmlFunction("document".to_string(), "model".to_string())
  );
  eq_dbg!(r#"FeelType(Number)"#, Value::FeelType(t_number.clone()));
  eq_dbg!(r#"FormalParameter(Name("a"), Number)"#, Value::FormalParameter(name.clone(), t_number.clone()));
  eq_dbg!(r#"FormalParameters([])"#, Value::FormalParameters(vec![]));
  eq_dbg!(
    r#"FunctionBody(FunctionBodyLiteralExpression, false)"#,
    Value::FunctionBody(v_function_body.clone(), v_external)
  );
  eq_dbg!(
    r#"FunctionDefinition(FunctionDefinition { parameters: [(Name("a"), Number)], body: FunctionBodyLiteralExpression, external: false, closure: Closure({}), closure_ctx: FeelContext({}), result_type: Number })"#,
    Value::from(FunctionDefinition::new(
      vec![(name.clone(), t_number.clone())],
      v_function_body,
      v_external,
      v_closure,
      v_closure_ctx,
      t_number
    ))
  );
  eq_dbg!(r#"IntervalEnd(Number(+1E+0), Opened)"#, Value::IntervalEnd(b_number.clone(), IntervalType::Opened));
  eq_dbg!(r#"IntervalStart(Number(+1E+0), Opened)"#, Value::IntervalStart(b_number.clone(), IntervalType::Opened));
  eq_dbg!(r#"Irrelevant"#, Value::Irrelevant);
  eq_dbg!(r#"List([])"#, Value::List(Values::default()));
  eq_dbg!(
    r#"NamedParameter(ParameterName(Name("a")), Number(+1E+0))"#,
    Value::NamedParameter(Box::new(Value::ParameterName(name.clone())), b_number.clone())
  );
  eq_dbg!(r#"NamedParameters({})"#, Value::NamedParameters(BTreeMap::new()));
  eq_dbg!(r#"NegatedCommaList([])"#, Value::NegatedCommaList(Values::default()));
  eq_dbg!(r#"Number(+1E+0)"#, Value::Number(FeelNumber::new(1, 0)));
  eq_dbg!(r#"Null(None)"#, Value::Null(None));
  eq_dbg!(r#"ParameterName(Name("a"))"#, Value::ParameterName(name.clone()));
  eq_dbg!(r#"ParameterTypes([])"#, Value::ParameterTypes(vec![]));
  eq_dbg!(r#"PositionalParameters([])"#, Value::PositionalParameters(Values::default()));
  eq_dbg!(r#"QualifiedNameSegment(Name("a"))"#, Value::QualifiedNameSegment(name));
  eq_dbg!(
    r#"Range(Number(+1E+0), Opened, Number(+1E+0), Closed)"#,
    Value::Range(b_number.clone(), IntervalType::Opened, b_number.clone(), IntervalType::Closed)
  );
  eq_dbg!(r#"String("beta")"#, Value::String("beta".into()));
  eq_dbg!(r#"Time(FeelTime(12, 13, 23, 0, Local))"#, Value::Time(v_time));
  eq_dbg!(r#"UnaryGreater(Number(+1E+0))"#, Value::UnaryGreater(b_number.clone()));
  eq_dbg!(r#"UnaryGreaterOrEqual(Number(+1E+0))"#, Value::UnaryGreaterOrEqual(b_number.clone()));
  eq_dbg!(r#"UnaryLess(Number(+1E+0))"#, Value::UnaryLess(b_number.clone()));
  eq_dbg!(r#"UnaryLessOrEqual(Number(+1E+0))"#, Value::UnaryLessOrEqual(b_number.clone()));
  eq_dbg!(r#"UnaryEqual(Number(+1E+0))"#, Value::UnaryEqual(b_number.clone()));
  eq_dbg!(r#"UnaryNotEqual(Number(+1E+0))"#, Value::UnaryNotEqual(b_number.clone()));
  eq_dbg!(r#"Transparent(Number(+1E+0))"#, Value::Transparent(b_number));
  eq_dbg!(
    r#"YearsAndMonthsDuration(FeelYearsAndMonthsDuration(38))"#,
    Value::YearsAndMonthsDuration(v_years_and_months_duration)
  );
}

#[test]
fn test_display() {
  let name = Name::from("a");
  let t_number = FeelType::Number;
  let v_number = Value::Number(FeelNumber::new(1, 0));
  let b_number = Box::new(v_number);
  let v_date = FeelDate::new(2022, 9, 27).unwrap();
  let v_time = FeelTime::local_opt(12, 13, 23, 0).unwrap();
  let v_date_time = FeelDateTime::new(v_date.clone(), v_time.clone());
  let v_function_body = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(2))));
  let v_external = false;
  let v_days_and_time_duration = FeelDaysAndTimeDuration::from_s(100);
  let v_years_and_months_duration = FeelYearsAndMonthsDuration::from_ym(3, 2);
  let v_closure = Closure::default();
  let v_closure_ctx = FeelContext::default();
  eq_dsp!(r#"false"#, Value::Boolean(false));
  eq_dsp!(r#"BuiltInFunction"#, Value::BuiltInFunction(Bif::Time));
  eq_dsp!(r#"{}"#, Value::Context(FeelContext::default()));
  eq_dsp!(r#"ContextEntry"#, Value::ContextEntry(name.clone(), b_number.clone()));
  eq_dsp!(r#"a"#, Value::ContextEntryKey(name.clone()));
  eq_dsp!(r#"ContextType"#, Value::ContextType(t_number.clone()));
  eq_dsp!(r#"a: number"#, Value::ContextTypeEntry(name.clone(), t_number.clone()));
  eq_dsp!(r#"a"#, Value::ContextTypeEntryKey(name.clone()));
  eq_dsp!(r#"2022-09-27"#, Value::Date(v_date));
  eq_dsp!(r#"2022-09-27T12:13:23"#, Value::DateTime(v_date_time));
  eq_dsp!(r#"PT1M40S"#, Value::DaysAndTimeDuration(v_days_and_time_duration));
  eq_dsp!(r#"[]"#, Value::ExpressionList(Values::default()));
  eq_dsp!(
    r#"ExternalJavaFunction(class, method)"#,
    Value::ExternalJavaFunction("class".to_string(), "method".to_string())
  );
  eq_dsp!(
    r#"ExternalPmmlFunction(document, model)"#,
    Value::ExternalPmmlFunction("document".to_string(), "model".to_string())
  );
  eq_dsp!(r#"type(number)"#, Value::FeelType(t_number.clone()));
  eq_dsp!(r#"FormalParameter"#, Value::FormalParameter(name.clone(), t_number.clone()));
  eq_dsp!(r#"FormalParameters"#, Value::FormalParameters(vec![]));
  eq_dsp!(r#"FunctionBody"#, Value::FunctionBody(v_function_body.clone(), v_external));
  eq_dsp!(
    r#"FunctionDefinition([(Name("a"), Number)],_,false,[],{},number)"#,
    Value::from(FunctionDefinition::new(
      vec![(name.clone(), t_number.clone())],
      v_function_body,
      v_external,
      v_closure,
      v_closure_ctx,
      t_number
    ))
  );
  eq_dsp!(r#"IntervalEnd"#, Value::IntervalEnd(b_number.clone(), IntervalType::Opened));
  eq_dsp!(r#"IntervalStart"#, Value::IntervalStart(b_number.clone(), IntervalType::Opened));
  eq_dsp!(r#"Irrelevant"#, Value::Irrelevant);
  eq_dsp!(r#"[]"#, Value::List(Values::default()));
  eq_dsp!(r#"NamedParameter"#, Value::NamedParameter(Box::new(Value::ParameterName(name.clone())), b_number.clone()));
  eq_dsp!(r#"NamedParameters"#, Value::NamedParameters(BTreeMap::new()));
  eq_dsp!(r#"NegatedCommaList"#, Value::NegatedCommaList(Values::default()));
  eq_dsp!(r#"1"#, Value::Number(FeelNumber::new(1, 0)));
  eq_dsp!(r#"null"#, Value::Null(None));
  eq_dsp!(r#"ParameterName"#, Value::ParameterName(name.clone()));
  eq_dsp!(r#"ParameterTypes"#, Value::ParameterTypes(vec![]));
  eq_dsp!(r#"PositionalParameters"#, Value::PositionalParameters(Values::default()));
  eq_dsp!(r#"QualifiedNameSegment"#, Value::QualifiedNameSegment(name));
  eq_dsp!(r#"(1..1]"#, Value::Range(b_number.clone(), IntervalType::Opened, b_number.clone(), IntervalType::Closed));
  eq_dsp!(r#""beta""#, Value::String("beta".into()));
  eq_dsp!(r#"12:13:23"#, Value::Time(v_time));
  eq_dsp!(r#"UnaryGreater(1)"#, Value::UnaryGreater(b_number.clone()));
  eq_dsp!(r#"UnaryGreaterOrEqual(1)"#, Value::UnaryGreaterOrEqual(b_number.clone()));
  eq_dsp!(r#"UnaryLess(1)"#, Value::UnaryLess(b_number.clone()));
  eq_dsp!(r#"UnaryLessOrEqual(1)"#, Value::UnaryLessOrEqual(b_number.clone()));
  eq_dsp!(r#"UnaryEqual(1)"#, Value::UnaryEqual(b_number.clone()));
  eq_dsp!(r#"UnaryNotEqual(1)"#, Value::UnaryNotEqual(b_number.clone()));
  eq_dsp!(r#"Transparent(1)"#, Value::Transparent(b_number));
  eq_dsp!(r#"P3Y2M"#, Value::YearsAndMonthsDuration(v_years_and_months_duration));
}

#[test]
fn test_type_of() {
  let name = Name::from("a");
  let t_number = FeelType::Number;
  let v_number = Value::Number(FeelNumber::new(1, 0));
  let b_number = Box::new(v_number.clone());
  let v_boolean = Value::Boolean(false);
  let b_boolean = Box::new(v_boolean.clone());
  let v_date = FeelDate::new(2022, 9, 27).unwrap();
  let v_time = FeelTime::local_opt(12, 13, 23, 0).unwrap();
  let v_date_time = FeelDateTime::new(v_date.clone(), v_time.clone());
  let v_function_body = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(2))));
  let v_external = false;
  let v_days_and_time_duration = FeelDaysAndTimeDuration::from_s(100);
  let v_years_and_months_duration = FeelYearsAndMonthsDuration::from_ym(3, 2);
  let v_closure = Closure::default();
  let v_closure_ctx = FeelContext::default();
  eq_typ!(FeelType::Boolean, Value::Boolean(false));
  eq_typ!(FeelType::Any, Value::BuiltInFunction(Bif::Time));
  eq_typ!(FeelType::Context(BTreeMap::new()), Value::Context(FeelContext::default()));
  eq_typ!(FeelType::Any, Value::ContextEntry(name.clone(), b_number.clone()));
  eq_typ!(FeelType::Any, Value::ContextEntryKey(name.clone()));
  eq_typ!(FeelType::Number, Value::ContextType(t_number.clone()));
  eq_typ!(FeelType::Number, Value::ContextTypeEntry(name.clone(), t_number.clone()));
  eq_typ!(FeelType::Any, Value::ContextTypeEntryKey(name.clone()));
  eq_typ!(FeelType::Date, Value::Date(v_date));
  eq_typ!(FeelType::DateTime, Value::DateTime(v_date_time));
  eq_typ!(FeelType::DaysAndTimeDuration, Value::DaysAndTimeDuration(v_days_and_time_duration));
  eq_typ!(FeelType::Any, Value::ExpressionList(Values::default()));
  eq_typ!(FeelType::Any, Value::ExternalJavaFunction("class".to_string(), "method".to_string()));
  eq_typ!(FeelType::Any, Value::ExternalPmmlFunction("document".to_string(), "model".to_string()));
  eq_typ!(FeelType::Number, Value::FeelType(t_number.clone()));
  eq_typ!(FeelType::Number, Value::FormalParameter(name.clone(), t_number.clone()));
  eq_typ!(FeelType::Any, Value::FormalParameters(vec![]));
  eq_typ!(FeelType::Any, Value::FunctionBody(v_function_body.clone(), v_external));
  eq_typ!(
    FeelType::Function(vec![FeelType::Number], Box::new(FeelType::Number)),
    Value::from(FunctionDefinition::new(
      vec![(name.clone(), t_number.clone())],
      v_function_body,
      v_external,
      v_closure,
      v_closure_ctx,
      t_number.clone()
    ))
  );
  eq_typ!(FeelType::Number, Value::IntervalEnd(b_number.clone(), IntervalType::Opened));
  eq_typ!(FeelType::Number, Value::IntervalStart(b_number.clone(), IntervalType::Opened));
  eq_typ!(FeelType::Any, Value::Irrelevant);
  eq_typ!(FeelType::List(Box::new(FeelType::Null)), Value::List(vec![]));
  eq_typ!(FeelType::List(Box::new(t_number)), Value::List(vec![v_number.clone()]));
  eq_typ!(FeelType::List(Box::new(FeelType::Any)), Value::List(vec![v_number, v_boolean]));
  eq_typ!(FeelType::Any, Value::NamedParameter(Box::new(Value::ParameterName(name.clone())), b_number.clone()));
  eq_typ!(FeelType::Any, Value::NamedParameters(BTreeMap::new()));
  eq_typ!(FeelType::Any, Value::NegatedCommaList(Values::default()));
  eq_typ!(FeelType::Number, Value::Number(FeelNumber::new(1, 0)));
  eq_typ!(FeelType::Null, Value::Null(None));
  eq_typ!(FeelType::Any, Value::ParameterName(name.clone()));
  eq_typ!(FeelType::Any, Value::ParameterTypes(vec![]));
  eq_typ!(FeelType::Any, Value::PositionalParameters(Values::default()));
  eq_typ!(FeelType::Any, Value::QualifiedNameSegment(name));
  eq_typ!(
    FeelType::Range(Box::new(FeelType::Number)),
    Value::Range(b_number.clone(), IntervalType::Opened, b_number.clone(), IntervalType::Closed)
  );
  eq_typ!(
    FeelType::Range(Box::new(FeelType::Any)),
    Value::Range(b_number.clone(), IntervalType::Opened, b_boolean, IntervalType::Closed)
  );
  eq_typ!(FeelType::String, Value::String("beta".into()));
  eq_typ!(FeelType::Time, Value::Time(v_time));
  eq_typ!(FeelType::Boolean, Value::UnaryGreater(b_number.clone()));
  eq_typ!(FeelType::Boolean, Value::UnaryGreaterOrEqual(b_number.clone()));
  eq_typ!(FeelType::Boolean, Value::UnaryLess(b_number.clone()));
  eq_typ!(FeelType::Boolean, Value::UnaryLessOrEqual(b_number.clone()));
  eq_typ!(FeelType::Boolean, Value::UnaryEqual(b_number.clone()));
  eq_typ!(FeelType::Boolean, Value::UnaryNotEqual(b_number.clone()));
  eq_typ!(FeelType::Number, Value::Transparent(b_number));
  eq_typ!(FeelType::YearsAndMonthsDuration, Value::YearsAndMonthsDuration(v_years_and_months_duration));
}

#[test]
fn test_type_of_list_with_context() {
  let mut ctx_a = FeelContext::default();
  ctx_a.set_entry(&"a".into(), Value::String("alfa".into()));
  ctx_a.set_entry(&"b".into(), Value::String("beta".into()));
  let mut ctx_b = FeelContext::default();
  ctx_b.set_entry(&"a".into(), Value::String("gamma".into()));
  ctx_b.set_entry(&"b".into(), Value::String("gamma".into()));
  let list = Value::List(vec![Value::Context(ctx_a), Value::Context(ctx_b)]);
  assert_eq!("list<context<a: string, b: string>>", list.type_of().to_string());
}

#[test]
fn test_type_of_list_with_context_and_nulls() {
  let mut ctx_a = FeelContext::default();
  ctx_a.set_entry(&"a".into(), Value::String("alfa".into()));
  ctx_a.set_entry(&"b".into(), Value::String("beta".into()));
  let mut ctx_b = FeelContext::default();
  ctx_b.set_entry(&"a".into(), Value::String("gamma".into()));
  ctx_b.set_entry(&"b".into(), value_null!());
  let list = Value::List(vec![Value::Context(ctx_a), Value::Context(ctx_b)]);
  assert_eq!("list<context<a: string, b: string>>", list.type_of().to_string());
}

#[test]
fn test_is_number() {
  assert!(value_number!(10).is_number());
  assert!(value_number!(10123, 3).is_number());
  assert!(!Value::Boolean(true).is_number());
  assert!(!Value::Boolean(false).is_number());
}

#[test]
fn test_is_null() {
  assert!(Value::Null(None).is_null());
  assert!(Value::Null(Some("message".to_string())).is_null());
  assert!(!value_number!(10).is_null());
  assert!(!value_number!(10123, 3).is_null());
  assert!(!Value::Boolean(true).is_null());
  assert!(!Value::Boolean(false).is_null());
}

#[test]
fn test_is_true() {
  assert!(Value::Boolean(true).is_true());
  assert!(!Value::Boolean(false).is_true());
  assert!(!Value::Null(None).is_true());
  assert!(!value_number!(10).is_true());
}

#[test]
fn test_is_false() {
  assert!(Value::Boolean(false).is_false());
  assert!(!Value::Boolean(true).is_false());
  assert!(!Value::Null(None).is_false());
  assert!(!value_number!(10).is_false());
}

#[test]
fn test_comparison() {
  assert_eq!(value_number!(10), value_number!(10));
  assert_ne!(value_number!(10, 0), value_number!(10, 1));
  assert_eq!(Value::Boolean(true), Value::Boolean(true));
  assert_eq!(Value::Boolean(false), Value::Boolean(false));
  assert_ne!(Value::Boolean(true), Value::Boolean(false));
  assert_ne!(Value::Boolean(false), Value::Boolean(true));
  assert_eq!(Value::Context(FeelContext::default()), Value::Context(FeelContext::default()));
  let fun_body_a = FunctionBody::Context(Arc::new(Box::new(|_: &FeelScope| value_number!(1))));
  let fun_body_b = FunctionBody::Context(Arc::new(Box::new(|_: &FeelScope| value_number!(2))));
  let fun_body_c = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(3))));
  let fun_body_d = FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(4))));
  let fun_body_e = FunctionBody::DecisionTable(Arc::new(Box::new(|_: &FeelScope| value_number!(4))));
  let fun_body_f = FunctionBody::DecisionTable(Arc::new(Box::new(|_: &FeelScope| value_number!(5))));
  let fun_body_g = FunctionBody::External(Arc::new(Box::new(|_: &FeelScope| value_number!(6))));
  let fun_body_h = FunctionBody::External(Arc::new(Box::new(|_: &FeelScope| value_number!(7))));
  assert_eq!(fun_body_a, fun_body_b);
  assert_ne!(fun_body_a, fun_body_c);
  assert_ne!(fun_body_a, fun_body_e);
  assert_ne!(fun_body_a, fun_body_g);
  assert_eq!(fun_body_c, fun_body_d);
  assert_ne!(fun_body_c, fun_body_a);
  assert_ne!(fun_body_c, fun_body_e);
  assert_ne!(fun_body_c, fun_body_g);
  assert_eq!(fun_body_e, fun_body_f);
  assert_ne!(fun_body_e, fun_body_a);
  assert_ne!(fun_body_e, fun_body_c);
  assert_ne!(fun_body_e, fun_body_g);
  assert_eq!(fun_body_g, fun_body_h);
  assert_ne!(fun_body_g, fun_body_a);
  assert_ne!(fun_body_g, fun_body_c);
  assert_ne!(fun_body_g, fun_body_e);
}

#[test]
fn test_value_to_feel_string() {
  assert_eq!(r#""foo""#, Value::String("foo".into()).to_feel_string());
  assert_eq!(r#""\"bar\"""#, Value::String("\"bar\"".into()).to_feel_string());
  assert_eq!(r#"{}"#, Value::Context(FeelContext::default()).to_feel_string());
  assert_eq!(r#"[]"#, Value::List(Values::default()).to_feel_string());
  assert_eq!(
    r#"[1.2, 2.1]"#,
    Value::List(vec![Value::Number(FeelNumber::new(12, 1)), Value::Number(FeelNumber::new(21, 1))]).to_feel_string()
  );
  assert_eq!(r#"true"#, Value::Boolean(true).to_feel_string());
}

#[test]
fn test_jsonify() {
  assert_eq!(r#"true"#, Value::Boolean(true).jsonify());
  assert_eq!(r#"false"#, Value::Boolean(false).jsonify());
  assert_eq!(r#"1.23"#, Value::Number(FeelNumber::new(123, 2)).jsonify());
  assert_eq!(r#""beta""#, Value::String("beta".into()).jsonify());
  assert_eq!(r#""2023-02-24""#, Value::Date(FeelDate::new(2023, 2, 24).unwrap()).jsonify());
  assert_eq!(r#""18:35:12.000598677Z""#, Value::Time(FeelTime::utc(18, 35, 12, 598677)).jsonify());
  assert_eq!(
    r#""2023-02-24T18:35:12.000598677Z""#,
    Value::DateTime(FeelDateTime::new(FeelDate::new(2023, 2, 24).unwrap(), FeelTime::utc_opt(18, 35, 12, 598677).unwrap())).jsonify()
  );
  assert_eq!(r#""P10DT13H59M18S""#, Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(914358)).jsonify());
  assert_eq!(r#""P3Y2M""#, Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_ym(3, 2)).jsonify());
  assert_eq!(r#"[]"#, Value::ExpressionList(Values::default()).jsonify());
  assert_eq!(r#"{}"#, Value::Context(FeelContext::default()).jsonify());
  assert_eq!(r#"alpha"#, Value::ContextEntryKey("alpha".into()).jsonify());
  assert_eq!(r#"[]"#, Value::List(Values::default()).jsonify());
  assert_eq!(
    r#"[1.2, 2.1]"#,
    Value::List(vec![Value::Number(FeelNumber::new(12, 1)), Value::Number(FeelNumber::new(21, 1))]).jsonify()
  );
  assert_eq!(r#"null"#, Value::Null(None).jsonify());
  assert_eq!(r#""null(error details)""#, Value::Null(Some("error details".to_string())).jsonify());
  let n_start = Box::new(Value::Number(FeelNumber::new(1, 0)));
  let n_end = Box::new(Value::Number(FeelNumber::new(12, 0)));
  assert_eq!(r#""(1..12]""#, Value::Range(n_start, IntervalType::Opened, n_end, IntervalType::Closed).jsonify());
  assert_eq!(r#"jsonify trait not implemented for value: Irrelevant"#, Value::Irrelevant.jsonify());
}

#[test]
fn test_try_from() {
  // xsd::integer
  assert!(Value::try_from_xsd_integer("1").is_ok());
  assert!(Value::try_from_xsd_integer("1a").is_err());
  assert_eq!(
    "<ValueError> '1a' is not valid xsd:integer representation",
    Value::try_from_xsd_integer("1a").err().unwrap().to_string()
  );
  // xsd::decimal
  assert!(Value::try_from_xsd_decimal("1").is_ok());
  assert!(Value::try_from_xsd_decimal("1a").is_err());
  assert_eq!(
    "<ValueError> '1a' is not valid xsd:decimal representation",
    Value::try_from_xsd_decimal("1a").err().unwrap().to_string()
  );
  // xsd::double
  assert!(Value::try_from_xsd_double("1.2").is_ok());
  assert!(Value::try_from_xsd_double("1.2a").is_err());
  assert_eq!(
    "<ValueError> '1.2a' is not valid xsd:double representation",
    Value::try_from_xsd_double("1.2a").err().unwrap().to_string()
  );
  // xsd::boolean
  assert!(Value::try_from_xsd_boolean("true").is_ok());
  assert!(Value::try_from_xsd_boolean("false").is_ok());
  assert!(Value::try_from_xsd_boolean("1").is_ok());
  assert!(Value::try_from_xsd_boolean("0").is_ok());
  assert!(Value::try_from_xsd_boolean("5").is_err());
  assert!(Value::try_from_xsd_boolean("False").is_err());
  assert!(Value::try_from_xsd_boolean("True").is_err());
  assert_eq!(
    "<ValueError> 'TRUE' is not valid xsd:boolean representation",
    Value::try_from_xsd_boolean("TRUE").err().unwrap().to_string()
  );
  // xsd::date
  assert!(Value::try_from_xsd_date("2022-09-27").is_ok());
  assert!(Value::try_from_xsd_date("2022-02-31").is_err());
  assert_eq!(
    "<ValueError> '2022-02-31' is not valid xsd:date representation",
    Value::try_from_xsd_date("2022-02-31").err().unwrap().to_string()
  );
  // xsd::time
  assert!(Value::try_from_xsd_time("13:29:23").is_ok());
  assert!(Value::try_from_xsd_time("26:12:34").is_err());
  assert_eq!(
    "<ValueError> '26:12:34' is not valid xsd:time representation",
    Value::try_from_xsd_time("26:12:34").err().unwrap().to_string()
  );
  // xsd::dateTime
  assert!(Value::try_from_xsd_date_time("2022-09-27T13:29:23").is_ok());
  assert!(Value::try_from_xsd_date_time("2022-02-31T23:12:34").is_err());
  assert_eq!(
    "<ValueError> '2022-02-31T23:12:34' is not valid xsd:dateTime representation",
    Value::try_from_xsd_date_time("2022-02-31T23:12:34").err().unwrap().to_string()
  );
  // xsd::duration
  assert!(Value::try_from_xsd_duration("P1Y2M").is_ok());
  assert!(Value::try_from_xsd_duration("P2DT2H").is_ok());
  assert!(Value::try_from_xsd_duration("PYD").is_err());
  assert_eq!(
    "<ValueError> 'PYD' is not valid xsd:duration representation",
    Value::try_from_xsd_duration("PYD").err().unwrap().to_string()
  );
}

#[test]
fn test_coerced() {
  let v_number = value_number!(10);
  let v_string = Value::String("a".into());
  let v_list_number_1 = Value::List(vec![value_number!(1, 0)]);
  let v_list_number_2 = Value::List(vec![value_number!(1, 0), value_number!(2, 0)]);
  let v_list_string_1 = Value::List(vec![Value::String("A".into())]);
  let v_function_a = Value::from(FunctionDefinition::new(
    vec![(NAME_A.clone(), T_NUMBER.clone())],
    FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(1)))),
    false,
    Closure::default(),
    FeelContext::default(),
    T_NUMBER.clone(),
  ));
  let v_irrelevant = Value::Irrelevant;
  let mut ctx_d = FeelContext::default();
  ctx_d.set_entry(&NAME_A, v_string.clone());
  let v_context_d = Value::Context(ctx_d);
  let v_date = Value::Date(FeelDate::new(2022, 9, 27).unwrap());
  let v_list_date_1 = Value::List(vec![v_date.clone()]);
  let v_bif = Value::BuiltInFunction(Bif::Time);
  // the coerced value; the owned forms (`try_into_coerced`, `into_coerced`) give the same results
  let coerced = |value: &Value, target_type: &FeelType| {
    assert_eq!(value.try_coerced(target_type), value.clone().try_into_coerced(target_type), "{value} : {target_type}");
    let coerced = value.coerced(target_type);
    assert_eq!(coerced, value.clone().into_coerced(target_type), "{value} : {target_type}");
    coerced.to_string()
  };
  assert_eq!(r#"[10]"#, coerced(&v_number, &T_LIST_A));
  assert_eq!(r#"[10]"#, coerced(&v_number, &T_LIST_D));
  assert_eq!(r#"1"#, coerced(&v_list_number_1, T_NUMBER));
  assert_eq!(r#"null(after coercion)"#, coerced(&v_list_number_2, T_NUMBER));
  assert_eq!(r#"null(after coercion)"#, coerced(&v_list_string_1, T_NUMBER));
  assert_eq!(r#"["a"]"#, coerced(&v_string, &T_LIST_C));
  assert_eq!(r#"["a"]"#, coerced(&v_string, &T_LIST_D));
  assert_eq!(r#""A""#, coerced(&v_list_string_1, T_STRING));
  assert_eq!(r#"10"#, coerced(&v_number, T_NUMBER));
  assert_eq!(r#"FunctionDefinition([(Name("a"), Number)],_,false,[],{},number)"#, coerced(&v_function_a, T_NUMBER));
  assert_eq!(r#"null(after coercion)"#, coerced(&v_irrelevant, T_NUMBER));
  assert_eq!(r#"null(after coercion)"#, coerced(&v_context_d, &T_CONTEXT_A));
  assert_eq!(r#"2022-09-27T00:00:00Z"#, coerced(&v_date, T_DATE_TIME));
  assert_eq!(r#"2022-09-27T00:00:00Z"#, coerced(&v_list_date_1, T_DATE_TIME));
  assert_eq!(r#"2022-09-27"#, coerced(&v_list_date_1, T_DATE));
  assert_eq!(r#"null(after coercion)"#, coerced(&v_date, T_TIME));
  assert_eq!(r#"BuiltInFunction"#, coerced(&v_bif, T_ANY));
  assert_eq!(r#"null(after coercion)"#, coerced(&v_bif, T_NUMBER));
}

#[test]
fn test_type_of_list_items() {
  let ctx = |entries: &[(&str, Value)]| {
    let mut ctx = FeelContext::default();
    for (name, value) in entries {
      ctx.set_entry(&Name::from(*name), value.clone());
    }
    Value::Context(ctx)
  };
  let list_of = |item_type: FeelType| FeelType::List(Box::new(item_type));
  let context_a = FeelType::Context(BTreeMap::from([(NAME_A.clone(), FeelType::Number)]));
  // fixed cases: a null item (first, its type is conformed to only by null; later, it conforms to
  // any item type), a missing entry, extra entries, nested lists
  eq_typ!(list_of(FeelType::Any), Value::List(vec![value_null!(), value_number!(1)]));
  eq_typ!(list_of(context_a.clone()), Value::List(vec![ctx(&[("a", value_number!(1))]), value_null!()]));
  eq_typ!(list_of(FeelType::Any), Value::List(vec![ctx(&[("a", value_number!(1))]), ctx(&[("b", value_number!(1))])]));
  eq_typ!(
    list_of(context_a.clone()),
    Value::List(vec![ctx(&[("a", value_number!(1))]), ctx(&[("a", value_number!(2)), ("b", Value::String("x".into()))])])
  );
  eq_typ!(list_of(list_of(context_a)), Value::List(vec![Value::List(vec![ctx(&[("a", value_number!(1))])])]));
}

/// `true` when the randomized tests run their full size (`RHEON_FULL_TESTS` set).
fn full() -> bool {
  std::env::var_os("RHEON_FULL_TESTS").is_some()
}

/// The type of a value by the list typing rule of [Value::type_of], applied at every level and
/// with [FeelType::is_conformant] on the built type of every item: a list has the type of its
/// first item when the type of every other item conforms to it, `Any` otherwise.
fn reference_type(value: &Value) -> FeelType {
  match value {
    Value::List(items) => match items.split_first() {
      None => FeelType::List(Box::new(FeelType::Null)),
      Some((first, others)) => {
        let item_type = reference_type(first);
        if others.iter().all(|item| reference_type(item).is_conformant(&item_type)) {
          FeelType::List(Box::new(item_type))
        } else {
          FeelType::List(Box::new(FeelType::Any))
        }
      }
    },
    Value::Context(ctx) => FeelType::Context(ctx.iter().map(|(name, value)| (name.clone(), reference_type(value))).collect()),
    other => other.type_of(),
  }
}

/// The type of a list equals [reference_type] on random lists: lists of contexts like a template
/// context, with nulls, missing, extra and changed entries at any depth, and lists of random
/// values (2,000 lists, 20,000 with `RHEON_FULL_TESTS`).
#[test]
fn test_type_of_list_equals_reference_rule() {
  struct Rng(u64);
  impl Rng {
    fn below(&mut self, n: u64) -> u64 {
      self.0 ^= self.0 << 13;
      self.0 ^= self.0 >> 7;
      self.0 ^= self.0 << 17;
      self.0 % n
    }
    fn name(&mut self) -> Name {
      ["a", "b", "c", "d"][self.below(4) as usize].into()
    }
  }
  fn value(rng: &mut Rng, depth: u32) -> Value {
    match if depth > 3 { rng.below(4) } else { rng.below(9) } {
      0 => value_null!(),
      1 => value_number!(1),
      2 => Value::String("s".into()),
      3 => Value::Boolean(true),
      4..=6 => Value::Context(context(rng, depth)),
      7 => Value::List((0..rng.below(4)).map(|_| value(rng, depth + 1)).collect()),
      _ => Value::List(vec![value_null!(), value_number!(1)]),
    }
  }
  fn context(rng: &mut Rng, depth: u32) -> FeelContext {
    let mut ctx = FeelContext::default();
    for _ in 0..rng.below(4) {
      ctx.set_entry(&rng.name(), value(rng, depth + 1));
    }
    ctx
  }
  /// A value like the template: the template, the template with one entry removed, set to null,
  /// added or replaced, or changed in the same way one level deeper; or null.
  fn variant(rng: &mut Rng, template: &FeelContext) -> Value {
    let mut ctx = template.clone();
    let name = rng.name();
    match rng.below(7) {
      0 => {}
      1 => {
        ctx.remove_entry(&name);
      }
      2 => ctx.set_entry(&name, value_null!()),
      3 => ctx.set_entry(&name, value(rng, 2)),
      4 | 5 => {
        if let Some(Value::Context(inner)) = template.get_entry(&name) {
          ctx.set_entry(&name, variant(rng, inner));
        }
      }
      _ => return value_null!(),
    }
    Value::Context(ctx)
  }
  let mut rng = Rng(0x2545_F491_4F6C_DD1D);
  let (mut contexts, mut mixed, mut total) = (0, 0, 0);
  for _ in 0..if full() { 20_000 } else { 2_000 } {
    let items = if rng.below(2) == 0 {
      (0..rng.below(5)).map(|_| value(&mut rng, 1)).collect()
    } else {
      let template = context(&mut rng, 0);
      (0..1 + rng.below(4)).map(|_| variant(&mut rng, &template)).collect()
    };
    let list = Value::List(items);
    let expected = reference_type(&list);
    assert_eq!(expected, list.type_of(), "{list}");
    if let (FeelType::List(item_type), Value::List(items)) = (&expected, &list)
      && items.len() > 1
    {
      contexts += usize::from(matches!(item_type.as_ref(), FeelType::Context(_)));
      mixed += usize::from(matches!(item_type.as_ref(), FeelType::Any));
      total += 1;
    }
  }
  // both outcomes of the rule are frequent among the lists with several items
  assert!(contexts > total / 5 && mixed > total / 5, "{contexts} lists of contexts, {mixed} mixed lists, of {total}");
}
