#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

mod boxed_expressions;
mod business_knowledge_model;
mod decision;
mod decision_service;
mod decision_table;
mod errors;
mod input_data;
mod input_data_context;
mod item_definition;
mod item_definition_context;
mod item_definition_graph;
mod item_definition_type;
mod model_builder;
mod model_definitions;
mod model_evaluator;
mod type_ref;
mod variable;

#[cfg(test)]
mod tests;

pub use decision_table::build_decision_table_evaluator;
pub use model_evaluator::ModelEvaluator;
