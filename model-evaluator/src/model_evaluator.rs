//! # Decision model evaluator

use crate::business_knowledge_model::BusinessKnowledgeModelEvaluator;
use crate::decision::DecisionEvaluator;
use crate::decision_service::DecisionServiceEvaluator;
use crate::input_data::InputDataEvaluator;
use crate::item_definition::ItemDefinitionEvaluator;
use crate::model_builder::{EvaluatorBuilders, ModelBuilder};
use crate::model_definitions::{DefKey, InvocableType, Invocables};
use dsntk_common::Result;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics::{self, Diagnostics, ErrorMode};
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, Name, value_null};
use rheon_feel_evaluator::{Dialect, evaluate_invocation, reset_call_depth};
use rheon_model::Definitions;
use std::sync::Arc;

/// Model evaluator.
pub struct ModelEvaluator {
  /// Input data evaluator.
  input_data_evaluator: InputDataEvaluator,
  /// Item definition evaluator.
  item_definition_evaluator: ItemDefinitionEvaluator,
  /// Business knowledge model evaluator.
  business_knowledge_model_evaluator: BusinessKnowledgeModelEvaluator,
  /// Decision evaluator.
  decision_evaluator: DecisionEvaluator,
  /// Decision service evaluator.
  decision_service_evaluator: DecisionServiceEvaluator,
  /// Map of invocables indexed by invocable name.
  invocables: Invocables,
  /// Map of global information item types defined in this model evaluator.
  global_context: FeelContext,
}

impl From<ModelBuilder> for ModelEvaluator {
  /// Creates [ModelEvaluator] from provided [ModelBuilder].
  fn from(model_builder: ModelBuilder) -> Self {
    let builders: EvaluatorBuilders = model_builder.into();
    let mut global_context = FeelContext::default();
    for (def_key, feel_type) in builders.information_item_types {
      global_context.set_entry(&Name::from(def_key.id()), Value::FeelType(feel_type))
    }
    Self {
      input_data_evaluator: builders.input_data_evaluator,
      item_definition_evaluator: builders.item_definition_evaluator,
      business_knowledge_model_evaluator: builders.business_knowledge_model_evaluator,
      decision_evaluator: builders.decision_evaluator,
      decision_service_evaluator: builders.decision_service_evaluator,
      invocables: builders.invocables,
      global_context,
    }
  }
}

impl ModelEvaluator {
  /// Creates an instance of [ModelEvaluator] from parsed [Definitions].
  pub fn new(definitions: &[Definitions]) -> Result<Arc<Self>> {
    let mut model_builder = ModelBuilder::default();
    definitions.iter().for_each(|definitions| model_builder.add_model(definitions));
    model_builder.build()?;
    let model_evaluator: Arc<ModelEvaluator> = Arc::new(model_builder.into());
    model_evaluator.decision_service_evaluator.build_function_definitions(&Arc::clone(&model_evaluator));
    Ok(model_evaluator)
  }

  /// Returns a reference to input data evaluator.
  pub fn input_data_evaluator(&self) -> &InputDataEvaluator {
    &self.input_data_evaluator
  }

  /// Returns a reference to item definition evaluator.
  pub fn item_definition_evaluator(&self) -> &ItemDefinitionEvaluator {
    &self.item_definition_evaluator
  }

  /// Returns a reference to business knowledge model evaluator.
  pub fn business_knowledge_model_evaluator(&self) -> &BusinessKnowledgeModelEvaluator {
    &self.business_knowledge_model_evaluator
  }

  /// Returns a reference to decision evaluator.
  pub fn decision_evaluator(&self) -> &DecisionEvaluator {
    &self.decision_evaluator
  }

  /// Returns a reference to decision service evaluator.
  pub fn decision_service_evaluator(&self) -> &DecisionServiceEvaluator {
    &self.decision_service_evaluator
  }

  /// Returns a reference to invocables in model evaluator.
  pub fn invocables(&self) -> &Invocables {
    &self.invocables
  }

  /// Evaluates an invocable.
  pub fn evaluate_invocable(&self, model_namespace: &str, model_name: &str, invocable_name: &str, input_data: &FeelContext) -> Value {
    reset_call_depth(); // the call depth limit is per evaluation
    let Some(invocable) = self.invocables.by_name(model_namespace, model_name, invocable_name) else {
      return value_null!("invocable '{}' not found in namespace '{}'", invocable_name, model_namespace);
    };
    match invocable {
      InvocableType::Decision(def_key) => {
        // evaluate a decision
        self.evaluate_decision(def_key, input_data)
      }
      InvocableType::BusinessKnowledgeModel(def_key, output_variable_name) => {
        // evaluate a business knowledge model
        self.evaluate_bkm(def_key, input_data, output_variable_name)
      }
      InvocableType::DecisionService(def_key) => {
        // evaluate a decision service
        self.evaluate_decision_service(def_key, input_data)
      }
    }
  }

  /// Evaluates an invocable in the given error handling mode (DMN 1.6 §7.3.8) and returns its
  /// value together with the errors recorded during the evaluation. Lenient (the default of
  /// [evaluate_invocable](Self::evaluate_invocable)) returns the value with the errors collected;
  /// strict returns null with the first error when any error occurred.
  pub fn evaluate_invocable_with_mode(
    &self,
    model_namespace: &str,
    model_name: &str,
    invocable_name: &str,
    input_data: &FeelContext,
    error_mode: ErrorMode,
  ) -> (Value, Diagnostics) {
    let (value, diagnostics) = diagnostics::collect(|| self.evaluate_invocable(model_namespace, model_name, invocable_name, input_data));
    match (error_mode, diagnostics.first()) {
      (ErrorMode::Strict, Some(first)) => (Value::Null(Some(format!("strict error mode: {first}"))), diagnostics),
      _ => (value, diagnostics),
    }
  }

  /// Evaluates a decision.
  fn evaluate_decision(&self, def_key: &DefKey, input_data: &FeelContext) -> Value {
    let mut evaluated_ctx = FeelContext::default();
    if let Some(output_variable_name) = self.decision_evaluator.evaluate(def_key, &self.global_context, input_data, self, &mut evaluated_ctx) {
      if let Some(output_value) = evaluated_ctx.get_entry(&output_variable_name) {
        output_value.clone()
      } else {
        value_null!()
      }
    } else {
      value_null!()
    }
  }

  /// Evaluates a business knowledge model.
  fn evaluate_bkm(&self, def_key: &DefKey, input_data: &FeelContext, output_variable_name: &Name) -> Value {
    let mut evaluated_ctx = FeelContext::default();
    self
      .business_knowledge_model_evaluator
      .evaluate(def_key, &self.global_context, input_data, self, &mut evaluated_ctx);
    let Some(function) = evaluated_ctx.get_entry(output_variable_name).cloned() else {
      return value_null!();
    };
    // the inputs of a business knowledge model are its parameters (DMN 1.6, §10.4), taken from the input data by name
    let Value::FunctionDefinition(function_definition) = &function else {
      return value_null!();
    };
    let arguments = function_definition
      .parameters
      .iter()
      .filter_map(|(name, _)| input_data.get_entry(name).map(|value| (name.clone(), value.clone())))
      .collect();
    // the requirements of the business knowledge model are in scope of its body, its parameters innermost
    evaluate_invocation(&FeelScope::from(evaluated_ctx), function, arguments, Dialect::default())
  }

  /// Evaluates a decision service.
  fn evaluate_decision_service(&self, def_key: &DefKey, input_data: &FeelContext) -> Value {
    let mut evaluated_ctx = FeelContext::default();
    if let Some(output_variable_name) = self
      .decision_service_evaluator
      .evaluate(def_key, &self.global_context, input_data, self, &mut evaluated_ctx)
    {
      if let Some(output_value) = evaluated_ctx.get_entry(&output_variable_name) {
        output_value.clone()
      } else {
        value_null!()
      }
    } else {
      value_null!()
    }
  }
}
