//! # Tests for the bound on how deeply the elements of a model nest

use crate::parse;

const TOO_DEEP: &str = "<ModelParserError> elements are nested more than 100 levels deep";

/// A model whose deepest element is at `level` (the root element is level 1): an item definition
/// with item components nested in each other, the innermost one with a type reference.
/// `markup` is written into every item component before its child and nests no element deeper.
fn nested_item_components_with(level: usize, markup: &str) -> String {
  let components = level - 3;
  let mut xml = String::from(
    r#"<?xml version="1.0" encoding="UTF-8"?><definitions namespace="https://dsntk.io/element-depth" name="element-depth" xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"><itemDefinition name="tNested">"#,
  );
  for i in 0..components {
    xml.push_str(&format!(r#"<itemComponent name="c{i}">{markup}"#));
  }
  xml.push_str("<typeRef>number</typeRef>");
  xml.push_str(&"</itemComponent>".repeat(components));
  xml.push_str("</itemDefinition></definitions>");
  xml
}

fn nested_item_components(level: usize) -> String {
  nested_item_components_with(level, "")
}

#[test]
fn elements_nested_to_the_bound_are_parsed() {
  assert!(parse(&nested_item_components(100)).is_ok());
}

#[test]
fn elements_nested_one_level_past_the_bound_are_rejected() {
  assert_eq!(TOO_DEEP, parse(&nested_item_components(101)).unwrap_err().to_string());
}

#[test]
fn elements_nested_far_past_the_bound_are_rejected_before_any_recursion() {
  assert_eq!(TOO_DEEP, parse(&nested_item_components(100_000)).unwrap_err().to_string());
}

#[test]
fn markup_that_nests_no_element_deeper_does_not_count() {
  for markup in [
    "<!-- <a><b><c> -->",
    "<description><![CDATA[<a><b><c>]]></description>",
    "<?pi <a><b> ?>",
    "<description>a > b /> c</description>",
    r#"<description label="/> > />"/>"#,
    r#"<description label='"/>'></description>"#,
  ] {
    let result = parse(&nested_item_components_with(100, markup));
    assert!(result.is_ok(), "{markup}: {}", result.unwrap_err());
    assert_eq!(TOO_DEEP, parse(&nested_item_components_with(101, markup)).unwrap_err().to_string(), "{markup}");
  }
}

#[test]
fn a_document_type_declaration_is_left_to_the_xml_parser() {
  let xml = nested_item_components(100_000).replacen("?>", "?><!DOCTYPE definitions>", 1);
  assert_eq!(
    "<ModelParserError> parsing model from XML failed with reason: XML with DTD detected",
    parse(&xml).unwrap_err().to_string()
  );
}
