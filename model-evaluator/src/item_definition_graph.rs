//! # The graph of item definition references
//!
//! The type of an item definition refers to other item definitions: through its type reference,
//! the type references of its components at any depth, and the parameter and output types of its
//! function item. The type, value and context evaluators of item definitions follow these
//! references at every use, recursively, so the graph they follow is checked once, over the item
//! definitions of every model of the build, before anything follows a reference.
//!
//! A cycle has no finite type: DMN 1.6 §7.3.3 lets a type reference name an item definition, and
//! defines no recursive types. A cycle is a build error that names it.
//!
//! The references are resolved by the rules the evaluators use
//! ([ItemDefinitionType](rheon_model::ItemDefinitionType) of each definition and component,
//! [referenced_item_definition] for function items), and a key names the last item definition
//! added under it, as in the evaluators, so the graph is the one the evaluators follow. The
//! search keeps its own stack, so a long chain or ring of definitions does not recurse.

use crate::errors::err_item_definition_cycle;
use crate::item_definition_type::{function_type_refs, referenced_item_definition};
use crate::model_definitions::{DefDefinitions, DefItemDefinition, DefKey};
use dsntk_common::Result;
use rheon_feel::message::arg;
use rheon_model::ItemDefinitionType;
use std::collections::HashMap;
use std::fmt;

/// Checks the item definitions of all models of a build: no item definition refers to itself,
/// directly or through others.
///
/// Every item definition must have a valid type (the evaluators report an invalid one first).
pub fn check(definitions: &DefDefinitions) -> Result<()> {
  let graph = Graph::new(definitions)?;
  let mut colours = vec![Colour::White; graph.nodes.len()];
  for root in 0..graph.nodes.len() {
    if colours[root] != Colour::White {
      continue;
    }
    colours[root] = Colour::Grey;
    let mut stack = vec![Visit { node: root, next: 0 }];
    while let Some(visit) = stack.last_mut() {
      let node = visit.node;
      let Some(&successor) = graph.edges[node].get(visit.next) else {
        colours[node] = Colour::Black;
        stack.pop();
        continue;
      };
      visit.next += 1;
      match colours[successor] {
        Colour::White => {
          colours[successor] = Colour::Grey;
          stack.push(Visit { node: successor, next: 0 });
        }
        Colour::Grey => {
          let start = stack.iter().position(|visit| visit.node == successor).unwrap_or_default();
          let names = stack[start..]
            .iter()
            .chain([&Visit { node: successor, next: 0 }])
            .map(|visit| graph.nodes[visit.node].name())
            .collect::<Vec<&str>>();
          return Err(err_item_definition_cycle(Cycle(&names)));
        }
        Colour::Black => {}
      }
    }
  }
  Ok(())
}

/// The state of a definition in the depth-first search.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Colour {
  /// Not reached yet.
  White,
  /// On the current path.
  Grey,
  /// Done, with everything it refers to.
  Black,
}

/// A definition on the current path, with the index of its next reference to follow.
struct Visit {
  node: usize,
  next: usize,
}

/// The item definitions, one per key, and the definitions each one refers to.
struct Graph<'a> {
  nodes: Vec<&'a DefItemDefinition>,
  edges: Vec<Vec<usize>>,
}

impl<'a> Graph<'a> {
  fn new(definitions: &'a DefDefinitions) -> Result<Self> {
    let mut index = HashMap::new();
    let mut nodes: Vec<&DefItemDefinition> = vec![];
    for item_definition in definitions.item_definitions() {
      let def_key = DefKey::new(item_definition.namespace(), item_definition.name());
      match index.get(&def_key) {
        Some(&node) => nodes[node] = item_definition,
        None => {
          index.insert(def_key, nodes.len());
          nodes.push(item_definition);
        }
      }
    }
    let mut edges = Vec::with_capacity(nodes.len());
    for item_definition in &nodes {
      let mut references = vec![];
      collect_references(item_definition, &mut references)?;
      edges.push(references.iter().filter_map(|def_key| index.get(def_key).copied()).collect());
    }
    Ok(Self { nodes, edges })
  }
}

/// Collects the item definitions the type of `item_definition` refers to, its components included.
/// The recursion follows the components, so it is bounded by the nesting of the model's elements.
fn collect_references(item_definition: &DefItemDefinition, references: &mut Vec<DefKey>) -> Result<()> {
  match item_definition.item_definition_type()? {
    ItemDefinitionType::SimpleType(_) | ItemDefinitionType::CollectionOfSimpleType(_) => {}
    ItemDefinitionType::ReferencedType(namespace, type_ref) | ItemDefinitionType::CollectionOfReferencedType(namespace, type_ref) => {
      references.push(DefKey::new(&namespace, &type_ref));
    }
    ItemDefinitionType::ComponentType | ItemDefinitionType::CollectionOfComponentType => {
      for component in item_definition.item_components() {
        collect_references(component, references)?;
      }
    }
    ItemDefinitionType::FunctionType => {
      let (parameters_type_ref, output_type_ref) = function_type_refs(item_definition);
      for type_ref in parameters_type_ref.iter().chain([&output_type_ref]) {
        references.extend(referenced_item_definition(item_definition.namespace(), type_ref));
      }
    }
  }
  Ok(())
}

/// The names of a cycle of item definitions, the first one repeated at the end, as `tA -> tB -> tA`.
struct Cycle<'a>(&'a [&'a str]);

impl fmt::Display for Cycle<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, name) in self.0.iter().enumerate() {
      if i > 0 {
        f.write_str(" -> ")?;
      }
      write!(f, "{}", arg(*name))?;
    }
    Ok(())
  }
}
