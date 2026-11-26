//! # Models that are not well formed
//!
//! A model with a part that can not be built is built without that part: the part has a named
//! error, and the rest evaluates.

use super::*;

const NAMESPACE: &str = "https://rheon.dev/build-safety";
const NAME: &str = "build-safety";

/// A model of the elements `body`.
fn model(body: &str) -> String {
  format!(r#"<definitions namespace="{NAMESPACE}" name="{NAME}" id="_m" xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/">{body}</definitions>"#)
}

/// A decision `name` (id `_name`) of the literal expression `text` that requires the decisions
/// and the business knowledge models of the ids given.
fn decision(name: &str, text: &str, decisions: &[&str], knowledge: &[&str]) -> String {
  let mut xml = format!(r#"<decision name="{name}" id="_{name}"><variable name="{name}"/>"#);
  for id in decisions {
    xml.push_str(&format!(
      r##"<informationRequirement id="_{name}_{id}"><requiredDecision href="#{id}"/></informationRequirement>"##
    ));
  }
  for id in knowledge {
    xml.push_str(&format!(
      r##"<knowledgeRequirement id="_{name}_k{id}"><requiredKnowledge href="#{id}"/></knowledgeRequirement>"##
    ));
  }
  xml.push_str(&format!("<literalExpression><text>{text}</text></literalExpression></decision>"));
  xml
}

/// A business knowledge model `name` (id `_name`) of no parameters and the body `text` that
/// requires the business knowledge models of the ids given.
fn bkm(name: &str, text: &str, knowledge: &[&str]) -> String {
  let mut xml = format!(r#"<businessKnowledgeModel name="{name}" id="_{name}"><variable name="{name}"/>"#);
  for id in knowledge {
    xml.push_str(&format!(
      r##"<knowledgeRequirement id="_{name}_k{id}"><requiredKnowledge href="#{id}"/></knowledgeRequirement>"##
    ));
  }
  xml.push_str(&format!(
    "<encapsulatedLogic><literalExpression><text>{text}</text></literalExpression></encapsulatedLogic></businessKnowledgeModel>"
  ));
  xml
}

/// The model of the elements `body`, built: the errors of the build, each as `code message`.
fn built(body: &str) -> (Arc<ModelEvaluator>, Vec<String>) {
  let definitions = rheon_model::parse(&model(body)).unwrap_or_else(|e| panic!("{e}"));
  let evaluator = ModelEvaluator::new(&[definitions]).unwrap_or_else(|e| panic!("{e}"));
  let said = evaluator
    .build_diagnostics()
    .iter()
    .filter(|diagnostic| diagnostic.severity == crate::BuildSeverity::Error)
    .map(|diagnostic| format!("{} {}", diagnostic.code, diagnostic.message))
    .collect();
  (evaluator, said)
}

/// The value of the invocable `name` without input data.
fn value(evaluator: &ModelEvaluator, name: &str) -> String {
  evaluated(evaluator, NAMESPACE, NAME, name, &FeelContext::default())
}

#[test]
fn decisions_that_require_each_other_are_not_built_and_the_others_are() {
  let body = decision("A", "B + 1", &["_B"], &[]) + &decision("B", "A + 1", &["_A"], &[]) + &decision("C", "A + 1", &["_A"], &[]) + &decision("D", "1 + 1", &[], &[]);
  let (evaluator, said) = built(&body);
  assert_eq!(
    said,
    [
      "requirement-cycle decision 'A' is in a cycle of requirements: A -> B -> A",
      "requirement-cycle decision 'B' is in a cycle of requirements: A -> B -> A",
      "unusable-requirement decision 'C' requires the decision 'A', which can not be evaluated: decision 'A' is in a cycle of requirements: A -> B -> A",
    ]
  );
  assert_eq!(value(&evaluator, "D"), "2");
  assert_eq!(value(&evaluator, "A"), "null(decision 'A' is in a cycle of requirements: A -> B -> A)");
  assert_eq!(
    value(&evaluator, "C"),
    "null(decision 'C' requires the decision 'A', which can not be evaluated: decision 'A' is in a cycle of requirements: A -> B -> A)"
  );
}

#[test]
fn a_business_knowledge_model_that_requires_itself_is_not_built() {
  let (evaluator, said) = built(&(decision("A", "F()", &[], &["_F"]) + &bkm("F", "F()", &["_F"]) + &decision("D", "G()", &[], &["_G"]) + &bkm("G", "7", &[])));
  assert_eq!(
    said,
    [
      "unusable-requirement decision 'A' requires the business knowledge model 'F', which can not be evaluated: business knowledge model 'F' is in a cycle of requirements: F -> F",
      "requirement-cycle business knowledge model 'F' is in a cycle of requirements: F -> F",
    ]
  );
  assert_eq!(value(&evaluator, "D"), "7");
  assert_eq!(value(&evaluator, "F"), "null(business knowledge model 'F' is in a cycle of requirements: F -> F)");
}

#[test]
fn business_knowledge_models_that_require_each_other_are_not_built() {
  let (evaluator, said) = built(&(decision("A", "F()", &[], &["_F"]) + &bkm("F", "G()", &["_G"]) + &bkm("G", "F()", &["_F"])));
  assert_eq!(said.len(), 3, "{said:?}");
  assert_eq!(said[1], "requirement-cycle business knowledge model 'F' is in a cycle of requirements: F -> G -> F");
  assert!(value(&evaluator, "A").starts_with("null(decision 'A' requires the business knowledge model 'F', which can not be evaluated"));
}

#[test]
fn a_decision_service_that_a_decision_of_it_requires_is_not_built() {
  let service = r##"<decisionService name="S" id="_S"><variable name="S"/><outputDecision href="#_A"/></decisionService>"##;
  let (evaluator, said) = built(&(decision("A", "S()", &[], &["_S"]) + service + &decision("D", "1", &[], &[])));
  assert_eq!(
    said,
    [
      "requirement-cycle decision 'A' is in a cycle of requirements: A -> S -> A",
      "requirement-cycle decision service 'S' is in a cycle of requirements: A -> S -> A",
    ]
  );
  assert_eq!(value(&evaluator, "D"), "1");
  assert_eq!(value(&evaluator, "S"), "null(decision service 'S' is in a cycle of requirements: A -> S -> A)");
}

#[test]
fn a_reference_that_leads_nowhere_or_to_another_kind_is_an_error_of_its_element() {
  let service = r##"<decisionService name="S" id="_S"><variable name="S"/><outputDecision href="#_D"/><inputData href="#_none"/></decisionService>"##;
  let body =
    decision("A", "1", &["_none"], &[]) + &decision("B", "1", &["_F"], &[]) + &decision("C", "1", &[], &["_D"]) + &decision("D", "2", &[], &[]) + &bkm("F", "1", &[]) + service;
  let (evaluator, said) = built(&body);
  assert_eq!(
    said,
    [
      format!("unknown-reference decision 'A': expected its required decision '{NAMESPACE}#_none' to be a decision of the models, found no element of that id"),
      format!("unknown-reference decision 'B': expected its required decision '{NAMESPACE}#_F' to be a decision, found the business knowledge model 'F'"),
      format!("unknown-reference decision 'C': expected its required knowledge '{NAMESPACE}#_D' to be a business knowledge model or a decision service, found the decision 'D'"),
      format!("unknown-reference decision service 'S': expected its input data '{NAMESPACE}#_none' to be input data of the models, found no element of that id"),
    ]
  );
  assert_eq!(value(&evaluator, "D"), "2");
  assert_eq!(value(&evaluator, "F"), "1");
}

#[test]
fn a_decision_service_with_neither_encapsulated_nor_input_decisions_evaluates_with_a_warning() {
  let service = r##"<decisionService name="S" id="_S"><variable name="S"/><outputDecision href="#_D"/></decisionService>"##;
  let (evaluator, _) = built(&(decision("D", "2", &[], &[]) + service));
  let diagnostics = evaluator.build_diagnostics();
  assert_eq!(diagnostics.len(), 1);
  assert_eq!(
    (
      diagnostics[0].severity,
      diagnostics[0].code,
      diagnostics[0].kind,
      diagnostics[0].id.as_str(),
      diagnostics[0].message.as_str()
    ),
    (
      crate::BuildSeverity::Warning,
      "service-without-decisions",
      crate::ElementKind::DecisionService,
      "_S",
      "decision service 'S': expected an encapsulated decision or an input decision, found neither"
    )
  );
  assert_eq!(value(&evaluator, "S"), "2");
}

/// A decision `T` of a decision table of the attributes `attributes` with the inputs `1 + 0` (one
/// for each of `inputs`), the outputs given (each its attributes and children) and the rules,
/// each its input entries and its output entries.
fn table(attributes: &str, inputs: usize, outputs: &[&str], rules: &[(&[&str], &[&str])]) -> String {
  let mut xml = format!(r#"<decision name="T" id="_T"><variable name="T"/><decisionTable {attributes}>"#);
  for _ in 0..inputs {
    xml.push_str("<input><inputExpression><text>1 + 0</text></inputExpression></input>");
  }
  for output in outputs {
    xml.push_str(&format!("<output {output}</output>"));
  }
  for (input_entries, output_entries) in rules {
    xml.push_str("<rule>");
    for entry in *input_entries {
      xml.push_str(&format!("<inputEntry><text>{entry}</text></inputEntry>"));
    }
    for entry in *output_entries {
      xml.push_str(&format!("<outputEntry><text>{entry}</text></outputEntry>"));
    }
    xml.push_str("</rule>");
  }
  xml.push_str("</decisionTable></decision>");
  xml
}

/// What the build says of the table, with a decision beside it that evaluates.
fn table_error(attributes: &str, inputs: usize, outputs: &[&str], rules: &[(&[&str], &[&str])]) -> String {
  let (evaluator, said) = built(&(table(attributes, inputs, outputs, rules) + &decision("D", "1 + 1", &[], &[])));
  assert_eq!(value(&evaluator, "D"), "2");
  assert_eq!(said.len(), 1, "{said:?}");
  assert_eq!(value(&evaluator, "T"), format!("null({})", said[0].strip_prefix("invalid-element ").unwrap()));
  said[0]
    .strip_prefix("invalid-element decision 'T' can not be built: <DecisionTableError> decision table: ")
    .unwrap()
    .to_string()
}

#[test]
fn a_decision_table_whose_parts_do_not_make_a_table_is_an_error_of_its_decision() {
  let one = [">"];
  let two = [r#"name="a">"#, r#"name="b">"#];
  // a rule with fewer or more entries than the table has clauses
  assert_eq!(table_error("", 2, &one, &[(&["1"], &["1"])]), "expected 2 input entries in rule 1, found 1");
  assert_eq!(
    table_error("", 1, &one, &[(&["1"], &["1"]), (&["1", "2"], &["1"])]),
    "expected 1 input entry in rule 2, found 2"
  );
  assert_eq!(table_error("", 1, &one, &[(&["1"], &[])]), "expected 1 output entry in rule 1, found 0");
  assert_eq!(table_error("", 1, &two, &[(&["1"], &["1"])]), "expected 2 output entries in rule 1, found 1");
  // several outputs, one without a name
  assert_eq!(
    table_error(
      "",
      1,
      &[r#"name="a">"#, "><defaultOutputEntry><text>1</text></defaultOutputEntry>"],
      &[(&["1"], &["1", "2"])]
    ),
    "expected a name for each of its 2 outputs, found none for output 2"
  );
  // a hit policy that the outputs do not allow
  assert_eq!(
    table_error(r#"hitPolicy="COLLECT" aggregation="COUNT""#, 1, &two, &[(&["1"], &["1", "2"])]),
    "expected one output for the hit policy C#, found 2 outputs"
  );
  assert_eq!(
    table_error(r#"hitPolicy="PRIORITY""#, 1, &one, &[(&["1"], &["1"])]),
    "expected output values (they order the outputs) for the hit policy P, found none"
  );
  assert_eq!(
    table_error(r#"hitPolicy="FIRST" preferredOrientation="CrossTable""#, 1, &one, &[(&["1"], &["1"])]),
    "expected U for the hit policy of a crosstab table, found another"
  );
}

#[test]
fn a_rule_without_input_entries_is_of_every_input_and_a_table_without_rules_gives_no_value() {
  let (evaluator, said) = built(&table("", 1, &[">"], &[(&[], &["7"])]));
  assert_eq!(said, [] as [&str; 0]);
  assert_eq!(evaluated(&evaluator, NAMESPACE, NAME, "T", &context("{x: 1}")), "7");
  let (evaluator, said) = built(&table("", 1, &[">"], &[]));
  assert_eq!(said, [] as [&str; 0]);
  assert_eq!(
    evaluated(&evaluator, NAMESPACE, NAME, "T", &context("{x: 1}")),
    "null(no rules matched, no output value defined)"
  );
}

#[test]
fn the_output_entries_of_a_rule_that_does_not_match_are_not_evaluated() {
  // the second rule does not match; its output entry is an error, and would be the table's
  let rules: [(&[&str], &[&str]); 2] = [(&["1"], &["\"first\""]), (&["2"], &["1 + \"a\""])];
  let (evaluator, _) = built(&table("", 1, &[">"], &rules));
  let outcome = evaluator.evaluate_invocable_with_mode(NAMESPACE, NAME, "T", &context("{x: 1}"), rheon_feel::diagnostics::ErrorMode::Strict);
  let (value, diagnostics) = outcome.into_result().unwrap();
  assert_eq!((value.to_string().as_str(), diagnostics.first().is_none()), ("\"first\"", true));
}
