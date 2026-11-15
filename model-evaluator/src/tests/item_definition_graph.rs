//! # Item definitions that refer to themselves, or whose types are too large or too deep

use super::assert_decision;
use crate::item_definition_type::ItemDefinitionTypeEvaluator;
use crate::model_definitions::DefDefinitions;
use crate::model_evaluator::ModelEvaluator;
use rheon_feel::FeelType;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
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

/// `t0` is a number and each `t<i>` a context of two entries of type `t<i-1>`, so the type of
/// `t<i>` has 2^(i+1) - 1 nodes.
fn doubling(levels: usize) -> String {
  let mut xml = reference("t0", "number");
  for i in 1..levels {
    let previous = format!("t{}", i - 1);
    xml.push_str(&components(&format!("t{i}"), &[("a", &previous), ("b", &previous)]));
  }
  xml
}

/// `t1` to `t<length - 1>` are each a context of one entry of the next type, and `t<length>` is a
/// number, so the type of `t1` nests `length` levels deep.
fn chain(length: usize) -> String {
  let mut xml = (1..length).map(|i| components(&format!("t{i}"), &[("x", &format!("t{}", i + 1))])).collect::<String>();
  xml.push_str(&reference(&format!("t{length}"), "number"));
  xml
}

#[test]
fn a_type_that_doubles_past_the_node_bound() {
  assert_eq!(
    "<ItemDefinitionError> item definition 't16' expands to more than 100000 type nodes",
    build(&[model(&doubling(17))])
  );
}

#[test]
fn a_type_that_doubles_thirty_times_is_rejected_without_being_built() {
  assert_eq!(
    "<ItemDefinitionError> item definition 't16' expands to more than 100000 type nodes",
    build(&[model(&doubling(30))])
  );
}

#[test]
fn a_type_nested_one_level_past_the_depth_bound() {
  assert_eq!(
    "<ItemDefinitionError> item definition 't1' nests types more than 256 levels deep",
    build(&[model(&chain(257))])
  );
}

#[test]
fn a_chain_of_a_hundred_thousand_definitions() {
  assert_eq!(
    "<ItemDefinitionError> item definition 't99744' nests types more than 256 levels deep",
    build(&[model(&chain(100_000))])
  );
}

/// The number of nodes and the depth of a built type.
fn nodes_and_depth(feel_type: &FeelType) -> (usize, usize) {
  let parts: Vec<&FeelType> = match feel_type {
    FeelType::Context(entries) => entries.values().collect(),
    FeelType::Function(parameters, result) => parameters.iter().chain([result.as_ref()]).collect(),
    FeelType::List(item) | FeelType::Range(item) => vec![item.as_ref()],
    _ => vec![],
  };
  parts
    .into_iter()
    .map(nodes_and_depth)
    .fold((1, 1), |(nodes, depth), (part_nodes, part_depth)| (nodes + part_nodes, depth.max(part_depth + 1)))
}

/// The type the type evaluator builds for the item definition `name` of the models.
fn built_type(models: &[Definitions], name: &str) -> FeelType {
  let mut definitions = DefDefinitions::default();
  models.iter().for_each(|model| definitions.add_model(model));
  ItemDefinitionTypeEvaluator::new(&definitions).unwrap().information_item_type(NAMESPACE, name).unwrap()
}

/// Item definitions of every kind: `tK` is a context (1 node) of a list of numbers (2), a list
/// of contexts of a number (3), a list of `t1` (1 + 3) and a function of `t1` and a number to a
/// type no definition has (1 + 3 + 1 + 1), 16 nodes in all.
fn every_kind() -> String {
  doubling(2)
    + r#"<itemDefinition name="tF"><functionItem outputTypeRef="unknown"><parameters name="p" typeRef="t1"/><parameters name="q" typeRef="number"/></functionItem></itemDefinition>"#
    + r#"<itemDefinition name="tK">
         <itemComponent name="a" isCollection="true"><typeRef>number</typeRef></itemComponent>
         <itemComponent name="b" isCollection="true"><itemComponent name="c"><typeRef>number</typeRef></itemComponent></itemComponent>
         <itemComponent name="d" isCollection="true"><typeRef>t1</typeRef></itemComponent>
         <itemComponent name="f"><typeRef>tF</typeRef></itemComponent>
       </itemDefinition>"#
}

/// `tTop`: a context of 6,249 entries of type `tK` and `numbers` entries of type number, so
/// 99,985 + `numbers` nodes.
fn uses_of_every_kind(numbers: usize) -> String {
  let mut xml = every_kind() + r#"<itemDefinition name="tTop">"#;
  for i in 0..6_249 {
    xml.push_str(&format!(r#"<itemComponent name="k{i}"><typeRef>tK</typeRef></itemComponent>"#));
  }
  for i in 0..numbers {
    xml.push_str(&format!(r#"<itemComponent name="n{i}"><typeRef>number</typeRef></itemComponent>"#));
  }
  xml.push_str("</itemDefinition>");
  xml
}

#[test]
fn the_bounds_count_the_type_the_evaluators_build() {
  assert_eq!((16, 4), nodes_and_depth(&built_type(&[model(&every_kind())], "tK")));
  assert_eq!((100_000, 5), nodes_and_depth(&built_type(&[model(&uses_of_every_kind(15))], "tTop")));
  assert_eq!((256, 256), nodes_and_depth(&built_type(&[model(&chain(256))], "t1")));
  assert_eq!((65_535, 16), nodes_and_depth(&built_type(&[model(&doubling(16))], "t15")));
}

#[test]
fn types_at_the_bounds_build() {
  assert_eq!("built", build(&[model(&uses_of_every_kind(15))]));
  assert_eq!("built", build(&[model(&chain(256))]));
}

#[test]
fn a_type_one_node_past_the_node_bound() {
  assert_eq!(
    "<ItemDefinitionError> item definition 'tTop' expands to more than 100000 type nodes",
    build(&[model(&uses_of_every_kind(16))])
  );
}

/// A model with `types`, an input typed `type_ref`, and two decisions: the input as a value of
/// that type, and whether the input is an instance of it.
fn model_with_input(types: &str, type_ref: &str) -> Definitions {
  model(&format!(
    r##"{types}
    <inputData name="Input" id="_input"><variable name="Input" typeRef="{type_ref}"/></inputData>
    <decision name="Output" id="_output">
      <variable name="Output" typeRef="{type_ref}"/>
      <informationRequirement><requiredInput href="#_input"/></informationRequirement>
      <literalExpression><text>Input</text></literalExpression>
    </decision>
    <decision name="Conforms" id="_conforms">
      <variable name="Conforms" typeRef="boolean"/>
      <informationRequirement><requiredInput href="#_input"/></informationRequirement>
      <literalExpression><text>Input instance of {type_ref}</text></literalExpression>
    </decision>"##
  ))
}

/// Builds the model, evaluates both decisions with `input` and checks that the input comes back
/// unchanged and conforms to its type. It runs on the default test thread of the debug build.
fn evaluate_at_the_bound(model: Definitions, input: Value) {
  let model_evaluator = ModelEvaluator::new(&[model]).unwrap();
  let mut input_data = FeelContext::default();
  input_data.set_entry(&"Input".into(), input.clone());
  assert_decision(&model_evaluator, NAMESPACE, "graph", "Output", &input_data, &input.to_string());
  assert_decision(&model_evaluator, NAMESPACE, "graph", "Conforms", &input_data, "true");
}

/// A value of the type of `t<level>` of [doubling].
fn doubling_value(level: usize) -> Value {
  if level == 0 {
    return Value::Number(1.into());
  }
  let item = doubling_value(level - 1);
  let mut context = FeelContext::default();
  context.set_entry(&"a".into(), item.clone());
  context.set_entry(&"b".into(), item);
  Value::Context(context)
}

#[test]
fn an_input_of_a_type_with_as_many_nodes_as_the_bound_is_checked_and_evaluated() {
  // 1 + 65,535 + 32,767 + 1,023 + 511 + 127 + 31 + 3 + 1 + 1 = 100,000 nodes
  let parts = [("a", 15), ("b", 14), ("c", 9), ("d", 8), ("e", 6), ("f", 4), ("g", 1), ("h", 0), ("i", 0)];
  let names = parts.iter().map(|(name, level)| (*name, format!("t{level}"))).collect::<Vec<(&str, String)>>();
  let entries = names.iter().map(|(name, type_ref)| (*name, type_ref.as_str())).collect::<Vec<(&str, &str)>>();
  let types = doubling(16) + &components("tBig", &entries);
  assert_eq!((100_000, 17), nodes_and_depth(&built_type(&[model(&types)], "tBig")));
  let mut input = FeelContext::default();
  for (name, level) in parts {
    input.set_entry(&name.into(), doubling_value(level));
  }
  evaluate_at_the_bound(model_with_input(&types, "tBig"), Value::Context(input));
}

#[test]
fn an_input_of_a_type_as_deep_as_the_bound_is_checked_and_evaluated() {
  let mut input = Value::Number(1.into());
  for _ in 1..256 {
    let mut context = FeelContext::default();
    context.set_entry(&"x".into(), input);
    input = Value::Context(context);
  }
  evaluate_at_the_bound(model_with_input(&chain(256), "t1"), input);
}
