//! # Item definitions that refer to themselves

use crate::model_evaluator::ModelEvaluator;
use rheon_model::Definitions;

const NAMESPACE: &str = "https://dsntk.io/item-definition-graph";

/// A model in `namespace` named `name` holding `content` (item definitions, imports).
fn model_in(namespace: &str, name: &str, content: &str) -> Definitions {
  rheon_model::parse(&format!(
    r#"<definitions namespace="{namespace}" name="{name}" xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/">{content}</definitions>"#
  ))
  .unwrap()
}

fn model(content: &str) -> Definitions {
  model_in(NAMESPACE, "graph", content)
}

/// The error of building the models together, or `built`.
fn build(models: &[Definitions]) -> String {
  match ModelEvaluator::new(models) {
    Ok(_) => "built".to_string(),
    Err(reason) => reason.to_string(),
  }
}

/// An item definition with one component of each given name and type reference.
fn components(name: &str, components: &[(&str, &str)]) -> String {
  let mut xml = format!(r#"<itemDefinition name="{name}">"#);
  for (component, type_ref) in components {
    xml.push_str(&format!(r#"<itemComponent name="{component}"><typeRef>{type_ref}</typeRef></itemComponent>"#));
  }
  xml.push_str("</itemDefinition>");
  xml
}

/// An item definition with a type reference.
fn reference(name: &str, type_ref: &str) -> String {
  format!(r#"<itemDefinition name="{name}"><typeRef>{type_ref}</typeRef></itemDefinition>"#)
}

#[test]
fn two_definitions_through_their_components() {
  let model = model(&(components("tA", &[("x", "tB")]) + &components("tB", &[("y", "tA")])));
  assert_eq!("<ItemDefinitionError> item definitions form a cycle: tA -> tB -> tA", build(&[model]));
}

#[test]
fn three_definitions_through_their_type_references() {
  let model = model(&(reference("tA", "tB") + &reference("tB", "tC") + &reference("tC", "tA")));
  assert_eq!("<ItemDefinitionError> item definitions form a cycle: tA -> tB -> tC -> tA", build(&[model]));
}

#[test]
fn a_function_returning_its_own_type() {
  let model = model(r#"<itemDefinition name="tF"><functionItem outputTypeRef="tF"/></itemDefinition>"#);
  assert_eq!("<ItemDefinitionError> item definitions form a cycle: tF -> tF", build(&[model]));
}

#[test]
fn a_function_taking_its_own_type() {
  let model = model(r#"<itemDefinition name="tF"><functionItem outputTypeRef="number"><parameters name="f" typeRef="tF"/></functionItem></itemDefinition>"#);
  assert_eq!("<ItemDefinitionError> item definitions form a cycle: tF -> tF", build(&[model]));
}

#[test]
fn a_nested_component() {
  let model = model(
    &(r#"<itemDefinition name="tA"><itemComponent name="x"><itemComponent name="y"><typeRef>tB</typeRef></itemComponent></itemComponent></itemDefinition>"#.to_string()
      + &components("tB", &[("z", "tA")])),
  );
  assert_eq!("<ItemDefinitionError> item definitions form a cycle: tA -> tB -> tA", build(&[model]));
}

#[test]
fn two_models_with_one_namespace() {
  let first = model_in(NAMESPACE, "first", &components("tA", &[("x", "tB")]));
  let second = model_in(NAMESPACE, "second", &components("tB", &[("y", "tA")]));
  assert_eq!("<ItemDefinitionError> item definitions form a cycle: tA -> tB -> tA", build(&[first, second]));
}

#[test]
fn a_ring_of_ten_thousand_definitions_is_named_within_the_message_bound() {
  let content = (0..10_000).map(|i| reference(&format!("t{i}"), &format!("t{}", (i + 1) % 10_000))).collect::<String>();
  let error = build(&[model(&content)]);
  let message = error.strip_prefix("<ItemDefinitionError> ").unwrap();
  assert!(message.starts_with("item definitions form a cycle: t0 -> t1 -> t2 -> "), "{message}");
  assert_eq!(1_000, message.chars().count());
  assert!(message.ends_with('…'));
}

#[test]
fn a_self_referencing_tree_node() {
  for xml in [dsntk_examples::DMN_N_0088, dsntk_examples::item_definition::DMN_1001] {
    let definitions = rheon_model::parse(xml).unwrap();
    assert_eq!("<ItemDefinitionError> item definitions form a cycle: iTreeNode -> iTreeNode", build(&[definitions]));
  }
}

#[test]
fn definitions_without_a_cycle_build() {
  let model = model(
    &(reference("tSimpleString", "string")
      + &components("tA", &[("x", "tB"), ("y", "tB")])
      + &components("tB", &[("z", "tSimpleString")])
      + r#"<itemDefinition name="tF"><functionItem outputTypeRef="tA"><parameters name="b" typeRef="tB"/></functionItem></itemDefinition>"#),
  );
  assert_eq!("built", build(&[model]));
}

#[test]
fn a_reference_into_an_imported_model_is_not_followed() {
  let first_namespace = "https://dsntk.io/item-definition-graph/first";
  let second_namespace = "https://dsntk.io/item-definition-graph/second";
  let import = |namespace: &str, name: &str| format!(r#"<import namespace="{namespace}" name="{name}" importType="https://www.omg.org/spec/DMN/20191111/MODEL/"/>"#);
  let first = model_in(first_namespace, "first", &(import(second_namespace, "a") + &reference("tY", "a.tX")));
  let second = model_in(second_namespace, "second", &(import(first_namespace, "b") + &reference("tX", "b.tY")));
  assert_eq!("built", build(&[first, second]));
}
