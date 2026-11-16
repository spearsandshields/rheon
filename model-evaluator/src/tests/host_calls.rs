//! # Business knowledge models invoked from FEEL binders
//!
//! The body of a business knowledge model reads a name that it does not bind through the scope
//! of its invocation: the variables of a `for` or a `some`, the results so far of a `for`
//! (`partial`), and the entries of a context literal before the invocation.

use super::*;

const NAMESPACE: &str = "https://dsntk.io/host-calls";
const NAME: &str = "host calls";

/// A model with the business knowledge models `plus i` (`x + i`) and `count partial`
/// (`count(partial)`), and a decision of each of `decisions`, named by its text.
fn model(decisions: &[&str]) -> String {
  let mut xml = format!(
    r#"<definitions namespace="{NAMESPACE}" name="{NAME}" id="_host_calls" xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/">
  <businessKnowledgeModel name="plus i" id="_plus_i">
    <variable name="plus i"/>
    <encapsulatedLogic><formalParameter name="x" typeRef="number"/><literalExpression><text>x + i</text></literalExpression></encapsulatedLogic>
  </businessKnowledgeModel>
  <businessKnowledgeModel name="count partial" id="_count_partial">
    <variable name="count partial"/>
    <encapsulatedLogic><formalParameter name="x"/><literalExpression><text>count(partial)</text></literalExpression></encapsulatedLogic>
  </businessKnowledgeModel>"#
  );
  for (index, text) in decisions.iter().enumerate() {
    xml.push_str(&format!(
      r##"
  <decision name="d{index}" id="_d{index}">
    <variable name="d{index}"/>
    <knowledgeRequirement><requiredKnowledge href="#_plus_i"/></knowledgeRequirement>
    <knowledgeRequirement><requiredKnowledge href="#_count_partial"/></knowledgeRequirement>
    <literalExpression><text>{text}</text></literalExpression>
  </decision>"##
    ));
  }
  xml.push_str("\n</definitions>");
  xml
}

#[test]
fn a_business_knowledge_model_reads_the_binders_of_its_invocation() {
  let cases = [
    ("for i in 1..3 return plus i(10)", "[11, 12, 13]"),
    ("for i in 1..3 return count partial(0)", "[0, 1, 2]"),
    ("some i in [1, 2, 3] satisfies plus i(i) = 4", "true"),
    ("{i: 5, r: plus i(1)}.r", "6"),
    ("{r: plus i(1), i: 5}.r", "null([addition] incompatible types: 1 + null(context has no value for key 'i'))"),
    ("for j in 1..2, i in [j * 10] return plus i(j)", "[11, 22]"),
  ];
  let xml = model(&cases.map(|(text, _)| text));
  let model_evaluator = build_model_evaluator(&xml);
  for (index, (text, expected)) in cases.iter().enumerate() {
    let actual = evaluated(&model_evaluator, NAMESPACE, NAME, &format!("d{index}"), &context("{}"));
    assert_eq!(*expected, actual, "{text}");
  }
}
