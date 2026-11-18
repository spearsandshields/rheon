//! The order in which the entries of a context literal are evaluated.
//!
//! DMN 1.6 clause 10.3.2.11.1: "All of the entries of m are in-scope for e, but the depends on
//! graph SHALL be acyclic. [...] Simply evaluate the context entries in depends on order." And
//! clause 10.3.2.6: "An expression in a context entry may not reference the key of the same
//! context entry". So an entry is evaluated after the entries its value reads, an entry never
//! reads its own key (the name is read outside the literal), and entries that need each other in
//! a circle have no value.
//!
//! A function definition needs nothing to be made: what its body reads is needed when it is
//! invoked. So an entry that is a function definition can stand in a circle (the functions of a
//! literal can invoke each other and themselves), and an entry that reads a function entry needs
//! what the body of that function reads.

use super::resolve::variables;
use rheon_feel::Name;
use rheon_feel_parser::AstNode;
use std::collections::BTreeMap;

/// The order of evaluation of the first `count` entries of a context literal, whose keys are
/// distinct: the indexes of the entries, each after the entries it needs and otherwise in the
/// order written; and for each entry in that order, whether it needs its own value (through other
/// entries), so that it has none.
pub(crate) fn order(entries: &[AstNode], count: usize) -> (Vec<usize>, Vec<bool>) {
  let mut keys: BTreeMap<&Name, usize> = BTreeMap::new();
  let mut values: Vec<Option<&AstNode>> = Vec::with_capacity(count);
  for (index, entry) in entries[..count].iter().enumerate() {
    match entry {
      AstNode::ContextEntry(key, value) => {
        if let AstNode::ContextEntryKey(name) = key.as_ref() {
          keys.insert(name, index);
        }
        values.push(Some(value));
      }
      _ => values.push(None),
    }
  }
  // the entries each entry reads, in ascending order, without itself
  let needs: Vec<Vec<usize>> = values
    .iter()
    .enumerate()
    .map(|(index, value)| {
      let mut walk = Walk {
        keys: &keys,
        bound: vec![],
        found: vec![],
      };
      if let Some(value) = value {
        walk.node(value);
      }
      walk.found.sort_unstable();
      walk.found.dedup();
      walk.found.retain(|&needed| needed != index);
      walk.found
    })
    .collect();
  if needs.iter().all(Vec::is_empty) {
    return ((0..count).collect(), vec![false; count]);
  }
  let function: Vec<bool> = values.iter().map(|value| matches!(value, Some(AstNode::FunctionDefinition(..)))).collect();
  // an entry that is no function definition and reaches itself has no value
  let circular: Vec<bool> = (0..count).map(|index| !function[index] && reaches_itself(index, &needs)).collect();
  // each entry after the entries it needs: a walk with its own stack, in the order written
  let mut state = vec![0u8; count];
  let mut ordered = Vec::with_capacity(count);
  let mut stack: Vec<(usize, usize)> = vec![];
  for first in 0..count {
    if state[first] != 0 {
      continue;
    }
    state[first] = 1;
    stack.push((first, 0));
    while let Some((index, next)) = stack.last_mut() {
      match needs[*index].get(*next) {
        Some(&needed) => {
          *next += 1;
          if state[needed] == 0 {
            state[needed] = 1;
            stack.push((needed, 0));
          }
        }
        None => {
          ordered.push(*index);
          stack.pop();
        }
      }
    }
  }
  let circular = ordered.iter().map(|&index| circular[index]).collect();
  (ordered, circular)
}

/// Whether the entry `start` needs itself through the entries it needs.
fn reaches_itself(start: usize, needs: &[Vec<usize>]) -> bool {
  let mut seen = vec![false; needs.len()];
  let mut stack = needs[start].clone();
  while let Some(index) = stack.pop() {
    if index == start {
      return true;
    }
    if !std::mem::replace(&mut seen[index], true) {
      stack.extend(needs[index].iter().copied());
    }
  }
  false
}

/// The walk over the value of an entry that finds the keys of the literal it reads.
struct Walk<'a, 'k> {
  keys: &'k BTreeMap<&'a Name, usize>,
  /// The names that a construct around the position binds: they are not the keys.
  bound: Vec<&'a Name>,
  found: Vec<usize>,
}

impl<'a> Walk<'a, '_> {
  fn read(&mut self, name: &'a Name) {
    if !self.bound.contains(&name)
      && let Some(&index) = self.keys.get(name)
    {
      self.found.push(index);
    }
  }

  fn node(&mut self, node: &'a AstNode) {
    match node {
      AstNode::Name(name) => self.read(name),
      AstNode::QualifiedName(segments) => {
        if let Some(AstNode::QualifiedNameSegment(name)) = segments.first() {
          self.read(name);
        }
      }
      // the name after the dot is the name of an entry of the value, and a type reads no name
      AstNode::Path(base, _) | AstNode::Descendant(base, _) | AstNode::InstanceOf(base, _) => self.node(base),
      AstNode::NamedParameter(_, value) => self.node(value),
      AstNode::For(contexts, body) => self.iteration(variables(contexts, false), body),
      AstNode::Some(contexts, body) | AstNode::Every(contexts, body) => self.iteration(variables(contexts, true), body),
      AstNode::FunctionDefinition(parameters, body) => {
        let mark = self.bound.len();
        if let AstNode::FormalParameters(items) = parameters.as_ref() {
          for item in items {
            if let AstNode::FormalParameter(name, _) = item
              && let AstNode::ParameterName(name) = name.as_ref()
            {
              self.bound.push(name);
            }
          }
        }
        if let AstNode::FunctionBody(expression, false) = body.as_ref() {
          self.node(expression);
        }
        self.bound.truncate(mark);
      }
      // the entries of a literal inside bind their keys for each other, not for their own value
      AstNode::Context(entries) => {
        let inner: Vec<Option<&'a Name>> = entries
          .iter()
          .map(|entry| match entry {
            AstNode::ContextEntry(key, _) => match key.as_ref() {
              AstNode::ContextEntryKey(name) => Some(name),
              _ => None,
            },
            _ => None,
          })
          .collect();
        for (index, entry) in entries.iter().enumerate() {
          if let AstNode::ContextEntry(_, value) = entry {
            let mark = self.bound.len();
            self
              .bound
              .extend(inner.iter().enumerate().filter(|(other, _)| *other != index).filter_map(|(_, name)| *name));
            self.node(value);
            self.bound.truncate(mark);
          }
        }
      }
      other => other.for_each_child(|child| self.node(child)),
    }
  }

  /// The domains of the variables, each seeing the variables before it, and then the body.
  fn iteration(&mut self, variables: Vec<(&'a Name, Vec<&'a AstNode>)>, body: &'a AstNode) {
    let mark = self.bound.len();
    for (name, domain) in variables {
      for part in domain {
        self.node(part);
      }
      self.bound.push(name);
    }
    self.node(body);
    self.bound.truncate(mark);
  }
}
