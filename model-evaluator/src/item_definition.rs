//! Builder for item definition evaluators.

use crate::errors::*;
use crate::model_definitions::{DefDefinitions, DefItemDefinition, DefKey};
use dsntk_common::Result;
use rheon_feel::context::FeelContext;
use rheon_feel::message::arg;
use rheon_feel::values::{TypeOf, Value, Values};
use rheon_feel::{Evaluator, FeelScope, FeelType, Name, value_null};
use rheon_feel_parser::AstNode;
use rheon_model::ItemDefinitionType;
use std::collections::HashMap;
use std::sync::Arc;

/// Type of closure that evaluates input data conformant with item definition.
pub type ItemDefinitionEvaluatorFn = Box<dyn Fn(&Value, &ItemDefinitionEvaluator) -> Value + Send + Sync>;

/// Item definition evaluator.
#[derive(Default)]
pub struct ItemDefinitionEvaluator {
  evaluators: Arc<HashMap<DefKey, ItemDefinitionEvaluatorFn>>,
}

impl ItemDefinitionEvaluator {
  /// Creates new item definition evaluator based on provided definitions.
  pub fn new(definitions: &DefDefinitions) -> Result<Self> {
    let mut evaluators = HashMap::new();
    for item_definition in definitions.item_definitions() {
      let evaluator = build_item_definition_evaluator(item_definition)?;
      let namespace = item_definition.namespace();
      let type_ref = item_definition.name();
      let def_key = DefKey::new(namespace, type_ref);
      evaluators.insert(def_key, evaluator);
    }
    Ok(Self { evaluators: Arc::new(evaluators) })
  }

  /// Evaluates item definition with specified type reference name.
  pub fn eval(&self, def_key: &DefKey, value: &Value) -> Option<Value> {
    self.evaluators.get(def_key).map(|evaluator| evaluator(value, self))
  }
}

pub fn build_item_definition_evaluator(item_definition: &DefItemDefinition) -> Result<ItemDefinitionEvaluatorFn> {
  // prepare optional allowed values evaluator
  let av_evaluator = build_allowed_values_evaluator(item_definition)?;
  // build item definition evaluator
  match item_definition.item_definition_type()? {
    ItemDefinitionType::SimpleType(feel_type) => build_simple_type_evaluator(feel_type, av_evaluator),
    ItemDefinitionType::ReferencedType(namespace, type_ref) => build_referenced_type_evaluator(DefKey::new(&namespace, &type_ref), av_evaluator),
    ItemDefinitionType::ComponentType => build_component_type_evaluator(item_definition),
    ItemDefinitionType::CollectionOfSimpleType(feel_type) => build_collection_of_simple_type_evaluator(feel_type, av_evaluator),
    ItemDefinitionType::CollectionOfReferencedType(namespace, type_ref) => build_collection_of_referenced_type_evaluator(DefKey::new(&namespace, &type_ref), av_evaluator),
    ItemDefinitionType::CollectionOfComponentType => build_collection_of_component_type_evaluator(item_definition),
    ItemDefinitionType::FunctionType => build_function_type_evaluator(),
  }
}

fn build_allowed_values_evaluator(item_definition: &DefItemDefinition) -> Result<Option<Evaluator>> {
  let mut av_evaluator = None;
  if let Some(unary_tests) = item_definition.allowed_values()
    && let Some(text) = unary_tests.text()
  {
    let scope = FeelScope::default();
    let unary_tests_node = rheon_feel_parser::parse_unary_tests(&scope, text, false)?;
    let node = AstNode::In(Box::new(AstNode::Name("?".into())), Box::new(unary_tests_node));
    av_evaluator = Some(rheon_feel_evaluator::prepare(&node));
  }

  Ok(av_evaluator)
}

fn check_allowed_values(value: Value, av_evaluator: Option<&Evaluator>) -> Value {
  if let Some(evaluator) = av_evaluator {
    let scope = FeelScope::default();
    scope.set_value(&"?".into(), value.clone());
    if evaluator(&scope).is_true() {
      value
    } else {
      value_null!("value '{}' is not allowed", arg(&value))
    }
  } else {
    value
  }
}

fn build_simple_type_evaluator(feel_type: FeelType, av_evaluator: Option<Evaluator>) -> Result<ItemDefinitionEvaluatorFn> {
  fn build_any_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| check_allowed_values(value.to_owned(), av_evaluator.as_ref()))
  }

  fn build_null_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::Null(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      _ => value_null!("expected type 'Null', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_string_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::String(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!("expected type 'string', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_number_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::Number(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!("expected type 'number', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_boolean_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::Boolean(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!("expected type 'boolean', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_date_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::Date(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!("expected type 'date', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_time_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::Time(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!("expected type 'time', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_date_time_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::DateTime(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!("expected type 'date and time', actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value)),
    })
  }

  fn build_dt_duration_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::DaysAndTimeDuration(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!(
        "expected type 'days and time duration', actual type is '{}' in value '{}'",
        arg(&TypeOf(value)),
        arg(&value)
      ),
    })
  }

  fn build_ym_duration_evaluator(av_evaluator: Option<Evaluator>) -> ItemDefinitionEvaluatorFn {
    Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| match value {
      Value::YearsAndMonthsDuration(_) => check_allowed_values(value.to_owned(), av_evaluator.as_ref()),
      null @ Value::Null(_) => null.clone(),
      _ => value_null!(
        "expected type 'years and months duration', actual type is '{}' in value '{}'",
        arg(&TypeOf(value)),
        arg(&value)
      ),
    })
  }
  match feel_type {
    FeelType::Any => Ok(build_any_evaluator(av_evaluator)),
    FeelType::Null => Ok(build_null_evaluator(av_evaluator)),
    FeelType::String => Ok(build_string_evaluator(av_evaluator)),
    FeelType::Number => Ok(build_number_evaluator(av_evaluator)),
    FeelType::Boolean => Ok(build_boolean_evaluator(av_evaluator)),
    FeelType::Date => Ok(build_date_evaluator(av_evaluator)),
    FeelType::Time => Ok(build_time_evaluator(av_evaluator)),
    FeelType::DateTime => Ok(build_date_time_evaluator(av_evaluator)),
    FeelType::DaysAndTimeDuration => Ok(build_dt_duration_evaluator(av_evaluator)),
    FeelType::YearsAndMonthsDuration => Ok(build_ym_duration_evaluator(av_evaluator)),
    _ => Err(err_unsupported_feel_type(feel_type, "a")),
  }
}

fn build_referenced_type_evaluator(def_key: DefKey, av_evaluator: Option<Evaluator>) -> Result<ItemDefinitionEvaluatorFn> {
  Ok(Box::new(move |value: &Value, evaluators: &ItemDefinitionEvaluator| {
    match evaluators.eval(&def_key, value) {
      // a value the referenced definition rejects keeps its reason, as a simple type passes a null
      Some(null @ Value::Null(_)) => null,
      Some(value) => check_allowed_values(value, av_evaluator.as_ref()),
      None => value_null!("no evaluator"),
    }
  }))
}

fn build_component_type_evaluator(item_definition: &DefItemDefinition) -> Result<ItemDefinitionEvaluatorFn> {
  let mut component_evaluators: Vec<(Name, ItemDefinitionEvaluatorFn)> = vec![];
  for component_item_definition in item_definition.item_components() {
    component_evaluators.push((component_item_definition.feel_name().clone(), build_item_definition_evaluator(component_item_definition)?));
  }
  let av_evaluator = build_allowed_values_evaluator(item_definition)?;
  Ok(Box::new(move |value: &Value, evaluators: &ItemDefinitionEvaluator| {
    if let Value::Context(ctx) = value {
      let mut evaluated_ctx = FeelContext::default();
      for (component_name, component_evaluator) in &component_evaluators {
        if let Some(component_value) = ctx.get_entry(component_name) {
          evaluated_ctx.set_entry(component_name, component_evaluator(component_value, evaluators));
        } else {
          return value_null!("item definition evaluator (Component): name not found: {} in context: {}", arg(&component_name), arg(&ctx));
        }
      }
      check_allowed_values(Value::Context(evaluated_ctx), av_evaluator.as_ref())
    } else {
      value_null!("item definition evaluator (Component): expected context value, actual value is: {}", arg(&value))
    }
  }))
}

fn build_collection_of_simple_type_evaluator(feel_type: FeelType, av_evaluator: Option<Evaluator>) -> Result<ItemDefinitionEvaluatorFn> {
  // the test of each element and the name of the element type in the message
  let (is_element, element_type): (fn(&Value) -> bool, &'static str) = match feel_type {
    FeelType::Any => (|_| true, "Any"),
    FeelType::Null => (|value| matches!(value, Value::Null(_)), "null"),
    FeelType::String => (|value| matches!(value, Value::String(_)), "string"),
    FeelType::Number => (|value| matches!(value, Value::Number(_)), "number"),
    FeelType::Boolean => (|value| matches!(value, Value::Boolean(_)), "boolean"),
    FeelType::Date => (|value| matches!(value, Value::Date(_)), "date"),
    FeelType::Time => (|value| matches!(value, Value::Time(_)), "time"),
    FeelType::DateTime => (|value| matches!(value, Value::DateTime(_)), "date and time"),
    FeelType::DaysAndTimeDuration => (|value| matches!(value, Value::DaysAndTimeDuration(_)), "days and time duration"),
    FeelType::YearsAndMonthsDuration => (|value| matches!(value, Value::YearsAndMonthsDuration(_)), "months and years duration"),
    _ => return Err(err_unsupported_feel_type(feel_type, "b")),
  };
  // a value that is no list is reported as a list expected, except by a collection of nulls
  let expected_list = if element_type == "null" { "null" } else { "list" };
  Ok(Box::new(move |value: &Value, _: &ItemDefinitionEvaluator| {
    let Value::List(values) = value else {
      return value_null!("item definition evaluator (CollectionOfSimpleType): expected {}", expected_list);
    };
    if !values.iter().all(is_element) {
      return value_null!("item definition evaluator (CollectionOfSimpleType): expected {}", element_type);
    }
    check_allowed_values(Value::List(values.clone()), av_evaluator.as_ref())
  }))
}

fn build_collection_of_referenced_type_evaluator(def_key: DefKey, av_evaluator: Option<Evaluator>) -> Result<ItemDefinitionEvaluatorFn> {
  Ok(Box::new(move |value: &Value, evaluators: &ItemDefinitionEvaluator| {
    if let Value::List(values) = value {
      let mut evaluated_values = Values::default();
      for item_value in values {
        if let Some(evaluated_value) = evaluators.eval(&def_key, item_value) {
          evaluated_values.push(evaluated_value);
        } else {
          return value_null!("no evaluator defined for type reference '{}'", def_key);
        }
      }
      check_allowed_values(Value::List(evaluated_values), av_evaluator.as_ref())
    } else {
      value_null!("expected list, actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value))
    }
  }))
}

fn build_collection_of_component_type_evaluator(item_definition: &DefItemDefinition) -> Result<ItemDefinitionEvaluatorFn> {
  let mut component_evaluators: Vec<(Name, ItemDefinitionEvaluatorFn)> = vec![];
  for component_item_definition in item_definition.item_components() {
    component_evaluators.push((component_item_definition.feel_name().clone(), build_item_definition_evaluator(component_item_definition)?));
  }
  let av_evaluator = build_allowed_values_evaluator(item_definition)?;
  Ok(Box::new(move |value: &Value, evaluators: &ItemDefinitionEvaluator| {
    if let Value::List(values) = value {
      let mut evaluated_values = Values::default();
      for item_value in values {
        if let Value::Context(ctx) = item_value {
          let mut evaluated_ctx = FeelContext::default();
          for (component_name, component_evaluator) in &component_evaluators {
            if let Some(component_value) = ctx.get_entry(component_name) {
              evaluated_ctx.set_entry(component_name, component_evaluator(component_value, evaluators));
            } else {
              return value_null!("name '{}' not found in context '{}'", arg(&component_name), arg(&ctx));
            }
          }
          evaluated_values.push(Value::Context(evaluated_ctx))
        } else {
          return value_null!("expected context, actual type is '{}' in value '{}'", arg(&TypeOf(item_value)), arg(&item_value));
        }
      }
      check_allowed_values(Value::List(evaluated_values), av_evaluator.as_ref())
    } else {
      value_null!("expected list, actual type is '{}' in value '{}'", arg(&TypeOf(value)), arg(&value))
    }
  }))
}

fn build_function_type_evaluator() -> Result<ItemDefinitionEvaluatorFn> {
  Ok(Box::new(move |_: &Value, _: &ItemDefinitionEvaluator| {
    value_null!("function type evaluator not implemented yet")
  }))
}

#[cfg(test)]
mod tests {
  use crate::item_definition::ItemDefinitionEvaluator;
  use crate::model_definitions::{DefDefinitions, DefKey};
  use dsntk_examples::item_definition::*;
  use rheon_feel::context::FeelContext;
  use rheon_feel::values::Value;
  use rheon_feel::{Name, value_null, value_number};
  use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration};

  const NAMESPACE: &str = "https://dsntk.io";

  /// Utility function for building item definition evaluator from definitions.
  fn build_evaluator(xml: &str) -> ItemDefinitionEvaluator {
    let definitions = rheon_model::parse(xml).unwrap();
    let mut def_definitions = DefDefinitions::default();
    def_definitions.add_model(&definitions);
    ItemDefinitionEvaluator::new(&def_definitions).unwrap()
  }

  #[test]
  fn test_evaluate_input_data_0101_1() {
    let evaluator = build_evaluator(DMN_0101);
    let context_str = r#"{ Customer Name : "Whistler" }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Customer", "Name"])).unwrap();
    assert_eq!(Value::String("Whistler".into()), evaluator.eval(&DefKey::new(NAMESPACE, "tCustomerName"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0101_2() {
    let evaluator = build_evaluator(DMN_0101);
    let context_str = r#"{ Customer Name : 12000 }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Customer", "Name"])).unwrap();
    assert_eq!(
      value_null!("expected type 'string', actual type is 'number' in value '12000'"),
      evaluator.eval(&DefKey::new(NAMESPACE, "tCustomerName"), value).unwrap()
    );
  }

  #[test]
  fn a_long_value_of_the_wrong_type_is_quoted_as_a_preview() {
    let evaluator = build_evaluator(DMN_0101);
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), "{ Customer Name : for i in 1..1000 return i }").unwrap();
    let value = context.get_entry(&Name::new(&["Customer", "Name"])).unwrap();
    let list = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, ";
    let expected = format!("expected type 'string', actual type is 'list<number>' in value '{list}… (1000 items)'");
    assert_eq!(Value::Null(Some(expected)), evaluator.eval(&DefKey::new(NAMESPACE, "tCustomerName"), value).unwrap());
  }

  #[test]
  fn a_wide_context_of_the_wrong_type_is_quoted_as_a_preview() {
    let evaluator = build_evaluator(DMN_0101);
    let mut wide = FeelContext::default();
    for i in 0..10_000 {
      wide.set_entry(&Name::from(format!("entry{i}")), value_number!(i));
    }
    let actual_type = "context<entry0: number, entry1: number, entry10: number, entry100: number, entry1000: number, entry1001: number, entry10";
    let actual_value = "{entry0: 0, entry1: 1, entry10: 10, entry100: 100, entry1000: 1000, entry1001: 1001, entry1002: 1002, entry1003: 1003, e";
    let expected = format!("expected type 'string', actual type is '{actual_type}… (10000 entries)' in value '{actual_value}… (10000 entries)'");
    assert_eq!(
      Value::Null(Some(expected)),
      evaluator.eval(&DefKey::new(NAMESPACE, "tCustomerName"), &Value::Context(wide)).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0102_1() {
    let evaluator = build_evaluator(DMN_0102);
    let context_str = r#"{ Monthly Salary : 12000.00 }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Monthly", "Salary"])).unwrap();
    assert_eq!(value_number!(12_000), evaluator.eval(&DefKey::new(NAMESPACE, "tMonthlySalary"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0102_2() {
    let evaluator = build_evaluator(DMN_0102);
    let context_str = r#"{ Monthly Salary : true }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Monthly", "Salary"])).unwrap();
    assert_eq!(
      value_null!("expected type 'number', actual type is 'boolean' in value 'true'"),
      evaluator.eval(&DefKey::new(NAMESPACE, "tMonthlySalary"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0103_1() {
    let evaluator = build_evaluator(DMN_0103);
    let context_str = r#"{ Is Affordable : true }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Is", "Affordable"])).unwrap();
    assert_eq!(Value::Boolean(true), evaluator.eval(&DefKey::new(NAMESPACE, "tIsAffordable"), value).unwrap());
    let context_str = r#"{ Is Affordable : false }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Is", "Affordable"])).unwrap();
    assert_eq!(Value::Boolean(false), evaluator.eval(&DefKey::new(NAMESPACE, "tIsAffordable"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0103_2() {
    let evaluator = build_evaluator(DMN_0103);
    let context_str = r#"{ Is Affordable : "Yes" }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Is", "Affordable"])).unwrap();
    assert_eq!(
      value_null!(r#"expected type 'boolean', actual type is 'string' in value '"Yes"'"#),
      evaluator.eval(&DefKey::new(NAMESPACE, "tIsAffordable"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0104_1() {
    let evaluator = build_evaluator(DMN_0104);
    let context_str = r#"{ Birthday : date("1982-04-12") }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Birthday"])).unwrap();
    assert_eq!(
      Value::Date(FeelDate::new(1982, 4, 12).unwrap()),
      evaluator.eval(&DefKey::new(NAMESPACE, "tBirthday"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0105_1() {
    let evaluator = build_evaluator(DMN_0105);
    let context_str = r#"{ Delivery Time : time("18:35:23") }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Delivery", "Time"])).unwrap();
    assert_eq!(
      Value::Time(FeelTime::local_opt(18, 35, 23, 0).unwrap()),
      evaluator.eval(&DefKey::new(NAMESPACE, "tDeliveryTime"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0106_1() {
    let evaluator = build_evaluator(DMN_0106);
    let context_str = r#"{ Appointment : date and time("2021-10-12T18:35:23") }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Appointment"])).unwrap();
    assert_eq!(
      Value::DateTime(FeelDateTime::new(FeelDate::new(2021, 10, 12).unwrap(), FeelTime::local_opt(18, 35, 23, 0).unwrap())),
      evaluator.eval(&DefKey::new(NAMESPACE, "tAppointment"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0107_1() {
    let evaluator = build_evaluator(DMN_0107);
    let context_str = r#"{ Course Duration : duration("P2DT3H") }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Course", "Duration"])).unwrap();
    assert_eq!(
      Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(183600)),
      evaluator.eval(&DefKey::new(NAMESPACE, "tCourseDuration"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0108_1() {
    let evaluator = build_evaluator(DMN_0108);
    let context_str = r#"{ Growth Duration : duration("P2Y5M") }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Growth", "Duration"])).unwrap();
    assert_eq!(
      Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_ym(2, 5).unwrap()),
      evaluator.eval(&DefKey::new(NAMESPACE, "tGrowthDuration"), value).unwrap()
    );
  }

  #[test]
  fn test_evaluate_input_data_0201_1() {
    let evaluator = build_evaluator(DMN_0201);
    let context_str = r#"{ Customer Name : "Bloomberg" }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Customer", "Name"])).unwrap();
    assert_eq!(Value::String("Bloomberg".into()), evaluator.eval(&DefKey::new(NAMESPACE, "tCustomerName"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0202_1() {
    let evaluator = build_evaluator(DMN_0202);
    let context_str = r#"{ Monthly Salary : 12000.00 }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Monthly", "Salary"])).unwrap();
    assert_eq!(value_number!(12_000), evaluator.eval(&DefKey::new(NAMESPACE, "tMonthlySalary"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0301_1() {
    let evaluator = build_evaluator(DMN_0301);
    let context_str = r#"{ Loan : { principal: 10, rate: 60, termMonths: 28 } }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Loan"])).unwrap();
    let mut ctx = FeelContext::default();
    ctx.set_entry(&"principal".into(), value_number!(10));
    ctx.set_entry(&"rate".into(), value_number!(60));
    ctx.set_entry(&"termMonths".into(), value_number!(28));
    let expected = Value::Context(ctx);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tLoan"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0401_1() {
    let evaluator = build_evaluator(DMN_0401);
    let context_str = r#"{ Items : ["Mercury", "Venus", "Earth", "Mars"] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      Value::String("Mercury".into()),
      Value::String("Venus".into()),
      Value::String("Earth".into()),
      Value::String("Mars".into()),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0402_1() {
    let evaluator = build_evaluator(DMN_0402);
    let context_str = r#"{ Items : [9000.00, 10000.00, 11000.00, 12000.00, 13000.00] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      value_number!(9_000),
      value_number!(10_000),
      value_number!(11_000),
      value_number!(12_000),
      value_number!(13_000),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0403_1() {
    let evaluator = build_evaluator(DMN_0403);
    let context_str = r#"{ Items : [true, false, false, true, true] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      Value::Boolean(true),
      Value::Boolean(false),
      Value::Boolean(false),
      Value::Boolean(true),
      Value::Boolean(true),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0404_1() {
    let evaluator = build_evaluator(DMN_0404);
    let context_str = r#"{ Items : [date("2021-10-10"), date("2021-10-11"), date("2021-10-12")] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      Value::Date(FeelDate::new(2021, 10, 10).unwrap()),
      Value::Date(FeelDate::new(2021, 10, 11).unwrap()),
      Value::Date(FeelDate::new(2021, 10, 12).unwrap()),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0405_1() {
    let evaluator = build_evaluator(DMN_0405);
    let context_str = r#"{ Items : [time("12:21:35"), time("12:21:36"), time("12:21:37")] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      Value::Time(FeelTime::local_opt(12, 21, 35, 0).unwrap()),
      Value::Time(FeelTime::local_opt(12, 21, 36, 0).unwrap()),
      Value::Time(FeelTime::local_opt(12, 21, 37, 0).unwrap()),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0406_1() {
    let evaluator = build_evaluator(DMN_0406);
    let context_str = r#"{ Items : [date and time("2021-10-10T21:23:18"), date and time("2021-10-11T12:18:59")] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      Value::DateTime(FeelDateTime::new(FeelDate::new(2021, 10, 10).unwrap(), FeelTime::local_opt(21, 23, 18, 0).unwrap())),
      Value::DateTime(FeelDateTime::new(FeelDate::new(2021, 10, 11).unwrap(), FeelTime::local_opt(12, 18, 59, 0).unwrap())),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0407_1() {
    let evaluator = build_evaluator(DMN_0407);
    let context_str = r#"{ Items : [duration("P2DT3H")] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(183600))]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0408_1() {
    let evaluator = build_evaluator(DMN_0408);
    let context_str = r#"{ Items : [duration("P2Y3M"), duration("P2Y4M")] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![
      Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_ym(2, 3).unwrap()),
      Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_ym(2, 4).unwrap()),
    ]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0501_1() {
    let evaluator = build_evaluator(DMN_0501);
    let context_str = r#"{ Items : ["Mercury", "Venus", "Earth"] }"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let expected = Value::List(vec![Value::String("Mercury".into()), Value::String("Venus".into()), Value::String("Earth".into())]);
    assert_eq!(expected, evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value).unwrap());
  }

  #[test]
  fn test_evaluate_input_data_0601_1() {
    let evaluator = build_evaluator(DMN_0601);
    let context_str = r#"{Items:[{number:1,name:"One",manager:"John"},{number:2,name:"Two",manager:"Mike"},{number:3,name:"Three",manager:"Bob"}]}"#;
    let context = rheon_feel_evaluator::evaluate_context(&Default::default(), context_str).unwrap();
    let value = context.get_entry(&Name::new(&["Items"])).unwrap();
    let mut ctx_1 = FeelContext::default();
    ctx_1.set_entry(&"number".into(), value_number!(1));
    ctx_1.set_entry(&"name".into(), Value::String("One".into()));
    ctx_1.set_entry(&"manager".into(), Value::String("John".into()));
    let mut ctx_2 = FeelContext::default();
    ctx_2.set_entry(&"number".into(), value_number!(2));
    ctx_2.set_entry(&"name".into(), Value::String("Two".into()));
    ctx_2.set_entry(&"manager".into(), Value::String("Mike".into()));
    let mut ctx_3 = FeelContext::default();
    ctx_3.set_entry(&"number".into(), value_number!(3));
    ctx_3.set_entry(&"name".into(), Value::String("Three".into()));
    ctx_3.set_entry(&"manager".into(), Value::String("Bob".into()));
    let expected = Value::List(vec![Value::Context(ctx_1), Value::Context(ctx_2), Value::Context(ctx_3)]);
    assert_eq!(Some(expected), evaluator.eval(&DefKey::new(NAMESPACE, "tItems"), value));
  }

  /// A collection of a simple type names what it expected: a list for a value that is no list
  /// (the collection of nulls says null), and the element type for an element of another type;
  /// `Any` takes every element.
  #[test]
  fn a_collection_of_a_simple_type_names_what_it_expected() {
    use rheon_feel::FeelType;
    let evaluators = ItemDefinitionEvaluator::default();
    let expected = |text: &str| Value::Null(Some(format!("item definition evaluator (CollectionOfSimpleType): expected {text}")));
    let cases = [
      (FeelType::Any, "list", None),
      (FeelType::Null, "null", Some("null")),
      (FeelType::String, "list", Some("string")),
      (FeelType::Number, "list", Some("number")),
      (FeelType::Boolean, "list", Some("boolean")),
      (FeelType::Date, "list", Some("date")),
      (FeelType::Time, "list", Some("time")),
      (FeelType::DateTime, "list", Some("date and time")),
      (FeelType::DaysAndTimeDuration, "list", Some("days and time duration")),
      (FeelType::YearsAndMonthsDuration, "list", Some("months and years duration")),
    ];
    for (feel_type, not_a_list, element) in cases {
      let evaluator = super::build_collection_of_simple_type_evaluator(feel_type.clone(), None).unwrap();
      assert_eq!(expected(not_a_list), evaluator(&value_number!(1), &evaluators), "{feel_type:?}");
      let list = Value::List(vec![Value::Context(FeelContext::default())]);
      let result = evaluator(&list, &evaluators);
      match element {
        Some(element) => assert_eq!(expected(element), result, "{feel_type:?}"),
        None => assert_eq!(list, result, "{feel_type:?}"),
      }
      assert_eq!(Value::List(vec![]), evaluator(&Value::List(vec![]), &evaluators), "{feel_type:?}");
    }
    assert!(super::build_collection_of_simple_type_evaluator(FeelType::Context(Default::default()), None).is_err());
  }

  /// DMN 1.6 §7.3.3: "An ItemDefinition element may restrict the values that are allowed from
  /// typeRef, using the allowedValues attribute", also when the typeRef names another item
  /// definition. A value the referenced definition rejects keeps that definition's reason.
  #[test]
  fn an_item_definition_that_references_another_keeps_its_allowed_values() {
    let evaluator = build_evaluator(DMN_0701);
    let def_key = DefKey::new(NAMESPACE, "tSmall");
    assert_eq!(
      Some(Value::Null(Some("value '11' is not allowed".to_string()))),
      evaluator.eval(&def_key, &value_number!(11))
    );
    assert_eq!(Some(value_number!(5)), evaluator.eval(&def_key, &value_number!(5)));
    let base_reason = "expected type 'number', actual type is 'string' in value '\"x\"'";
    assert_eq!(Some(Value::Null(Some(base_reason.to_string()))), evaluator.eval(&def_key, &Value::String("x".into())));
  }
}
