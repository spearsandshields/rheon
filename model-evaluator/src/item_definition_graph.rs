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
//! The type evaluator builds a definition's type as a tree, expanding a referenced definition at
//! each of its uses, so a few definitions that each use the next one twice build a type
//! exponentially larger than the model. A type with more than [MAX_TYPE_NODES] nodes, or nested
//! more than [MAX_TYPE_DEPTH] levels deep, is a build error that names the first definition, in
//! the order the search finishes them, whose type exceeds the bound. Both are computed from the
//! graph without building any type: a built-in type is one node; a context is one node and its
//! entries, a list one node and its item type, a function one node, its parameter types and its
//! result type; a referenced definition counts its whole type at each use, and a reference to no
//! definition counts as one node. The depth of a built-in type is 1, that of a context, a list
//! or a function one more than its deepest part. Every type the evaluators build afterwards is
//! within both bounds, so their recursion over types is bounded too. A type at the bounds is
//! built, and a value of it checked and evaluated, on a 2 MiB thread of an unoptimized build.
//!
//! The bounds hold per definition: a type is still expanded at each of its uses, so the work of a
//! build stays proportional to the size of the models times the node bound.
//!
//! The references are resolved by the rules the evaluators use
//! ([ItemDefinitionType](rheon_model::ItemDefinitionType) of each definition and component,
//! [referenced_item_definition] for function items), and a key names the last item definition
//! added under it, as in the evaluators, so the graph is the one the evaluators follow. The
//! search keeps its own stack, so a long chain or ring of definitions does not recurse.

use crate::errors::{err_item_definition_cycle, err_item_definition_too_deep, err_item_definition_too_large};
use crate::item_definition_type::{function_type_refs, referenced_item_definition};
use crate::model_definitions::{DefDefinitions, DefItemDefinition, DefKey};
use dsntk_common::Result;
use rheon_feel::message::arg;
use rheon_model::ItemDefinitionType;
use std::collections::HashMap;
use std::fmt;

/// The number of nodes the type of an item definition may have at most.
pub const MAX_TYPE_NODES: u64 = 100_000;

/// The number of levels the type of an item definition may nest at most.
pub const MAX_TYPE_DEPTH: u64 = 256;

/// Checks the item definitions of all models of a build: no item definition refers to itself,
/// directly or through others, and the type of each one is within [MAX_TYPE_NODES] and
/// [MAX_TYPE_DEPTH].
///
/// Every item definition must have a valid type (the evaluators report an invalid one first).
pub fn check(definitions: &DefDefinitions) -> Result<()> {
  let graph = Graph::new(definitions)?;
  let mut colours = vec![Colour::White; graph.nodes.len()];
  let mut sizes = vec![TypeSize::default(); graph.nodes.len()];
  for root in 0..graph.nodes.len() {
    if colours[root] != Colour::White {
      continue;
    }
    colours[root] = Colour::Grey;
    let mut stack = vec![Visit { node: root, next: 0 }];
    while let Some(visit) = stack.last_mut() {
      let node = visit.node;
      let Some(&(successor, _)) = graph.shapes[node].references.get(visit.next) else {
        // everything the definition refers to is done, so its size is known
        let size = graph.shapes[node].size(&sizes);
        let name = graph.nodes[node].name();
        if size.nodes > MAX_TYPE_NODES {
          return Err(err_item_definition_too_large(name, MAX_TYPE_NODES));
        }
        if size.depth > MAX_TYPE_DEPTH {
          return Err(err_item_definition_too_deep(name, MAX_TYPE_DEPTH));
        }
        sizes[node] = size;
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

/// The item definitions, one per key, and the shape of the type of each one.
struct Graph<'a> {
  nodes: Vec<&'a DefItemDefinition>,
  shapes: Vec<Shape>,
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
    let mut shapes = Vec::with_capacity(nodes.len());
    for item_definition in &nodes {
      let mut shape = Shape::default();
      shape.add(item_definition, 0, &index)?;
      shapes.push(shape);
    }
    Ok(Self { nodes, shapes })
  }
}

/// The number of nodes and the depth of a type.
#[derive(Clone, Copy, Default)]
struct TypeSize {
  nodes: u64,
  depth: u64,
}

/// The type of an item definition without the definitions it refers to: the size of its own
/// nodes, and each reference with the number of levels above it.
#[derive(Default)]
struct Shape {
  own: TypeSize,
  references: Vec<(usize, u64)>,
}

impl Shape {
  /// Adds the type of `item_definition`, `levels` below the root of the type. The recursion
  /// follows the components, so it is bounded by the nesting of the model's elements.
  fn add(&mut self, item_definition: &DefItemDefinition, levels: u64, index: &HashMap<DefKey, usize>) -> Result<()> {
    match item_definition.item_definition_type()? {
      ItemDefinitionType::SimpleType(_) => self.add_node(levels),
      ItemDefinitionType::CollectionOfSimpleType(_) => {
        self.add_node(levels);
        self.add_node(levels + 1);
      }
      ItemDefinitionType::ReferencedType(namespace, type_ref) => self.add_reference(Some(DefKey::new(&namespace, &type_ref)), levels, index),
      ItemDefinitionType::CollectionOfReferencedType(namespace, type_ref) => {
        self.add_node(levels);
        self.add_reference(Some(DefKey::new(&namespace, &type_ref)), levels + 1, index);
      }
      ItemDefinitionType::ComponentType => {
        self.add_node(levels);
        for component in item_definition.item_components() {
          self.add(component, levels + 1, index)?;
        }
      }
      ItemDefinitionType::CollectionOfComponentType => {
        self.add_node(levels);
        self.add_node(levels + 1);
        for component in item_definition.item_components() {
          self.add(component, levels + 2, index)?;
        }
      }
      ItemDefinitionType::FunctionType => {
        self.add_node(levels);
        let (parameters_type_ref, output_type_ref) = function_type_refs(item_definition);
        for type_ref in parameters_type_ref.iter().chain([&output_type_ref]) {
          self.add_reference(referenced_item_definition(item_definition.namespace(), type_ref), levels + 1, index);
        }
      }
    }
    Ok(())
  }

  /// Adds one node of the type, `levels` below its root.
  fn add_node(&mut self, levels: u64) {
    self.own.nodes = self.own.nodes.saturating_add(1);
    self.own.depth = self.own.depth.max(levels.saturating_add(1));
  }

  /// Adds a reference, `levels` below the root of the type; a built-in type, or a reference to
  /// no definition (which gives `Any` or no entry), is one node.
  fn add_reference(&mut self, def_key: Option<DefKey>, levels: u64, index: &HashMap<DefKey, usize>) {
    match def_key.and_then(|def_key| index.get(&def_key)) {
      Some(&node) => self.references.push((node, levels)),
      None => self.add_node(levels),
    }
  }

  /// The size of the whole type, given the sizes of the definitions it refers to.
  fn size(&self, sizes: &[TypeSize]) -> TypeSize {
    self.references.iter().fold(self.own, |size, &(node, levels)| TypeSize {
      nodes: size.nodes.saturating_add(sizes[node].nodes),
      depth: size.depth.max(levels.saturating_add(sizes[node].depth)),
    })
  }
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
