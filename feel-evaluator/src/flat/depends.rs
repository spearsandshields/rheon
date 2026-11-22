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
  // the keys that are written in another form than FEEL reads a name (strings), by that name
  // (the first of the keys, in their order, that are one name)
  let mut aliases: BTreeMap<Name, usize> = BTreeMap::new();
  for (name, index) in keys.iter().filter(|(name, _)| !rheon_feel::names::is_in_name_form(name.as_str())) {
    aliases.entry(Name::from(rheon_feel::names::name_form(name.as_str()))).or_insert(*index);
  }
  // the entries each entry reads, in ascending order, without itself
  let needs: Vec<Vec<usize>> = values
    .iter()
    .enumerate()
    .map(|(index, value)| {
      let mut walk = Walk {
        keys: &keys,
        aliases: &aliases,
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
  let cycles = on_cycles(&needs);
  let circular: Vec<bool> = (0..count).map(|index| !function[index] && cycles[index]).collect();
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

/// For each entry, whether it needs itself through the entries it needs: whether it is one of
/// two or more entries that all need each other (no entry is among its own needs). One walk over
/// all entries finds these groups (Tarjan's strongly connected components, with a stack of its
/// own), so the time is that of the entries and their needs together.
fn on_cycles(needs: &[Vec<usize>]) -> Vec<bool> {
  const UNSEEN: usize = usize::MAX;
  let count = needs.len();
  let mut number = vec![UNSEEN; count];
  let mut low = vec![0; count];
  let mut waiting = vec![false; count];
  let mut group: Vec<usize> = vec![];
  let mut cycles = vec![false; count];
  let mut next_number = 0;
  let mut stack: Vec<(usize, usize)> = vec![];
  for first in 0..count {
    if number[first] != UNSEEN {
      continue;
    }
    stack.push((first, 0));
    while let Some(&mut (index, ref mut next)) = stack.last_mut() {
      if *next == 0 {
        number[index] = next_number;
        low[index] = next_number;
        next_number += 1;
        group.push(index);
        waiting[index] = true;
      }
      match needs[index].get(*next) {
        Some(&needed) => {
          *next += 1;
          if number[needed] == UNSEEN {
            stack.push((needed, 0));
          } else if waiting[needed] {
            low[index] = low[index].min(number[needed]);
          }
        }
        None => {
          stack.pop();
          if let Some(&(parent, _)) = stack.last() {
            low[parent] = low[parent].min(low[index]);
          }
          if low[index] == number[index] {
            // the entries of the group of `index` are those above it
            let from = group.iter().rposition(|&member| member == index).unwrap_or(0);
            let together = group.len() - from > 1;
            for member in group.drain(from..) {
              waiting[member] = false;
              cycles[member] = together;
            }
          }
        }
      }
    }
  }
  cycles
}

/// The walk over the value of an entry that finds the keys of the literal it reads.
struct Walk<'a, 'k> {
  keys: &'k BTreeMap<&'a Name, usize>,
  /// The keys written in another form, by the name they are.
  aliases: &'k BTreeMap<Name, usize>,
  /// The names that a construct around the position binds: they are not the keys.
  bound: Vec<&'a Name>,
  found: Vec<usize>,
}

impl<'a> Walk<'a, '_> {
  fn read(&mut self, name: &'a Name) {
    if self.bound.contains(&name) {
      return;
    }
    if let Some(&index) = self.keys.get(name) {
      self.found.push(index);
    } else if let Some(&index) = self.aliases.get(name) {
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
      AstNode::Path(base, _) | AstNode::Descendant(base, _) | AstNode::InstanceOf(base, _) | AstNode::As(base, _) | AstNode::AbstractType(base, _) => self.node(base),
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

#[cfg(test)]
mod tests {
  use super::*;

  /// Whether the entry `start` needs itself through the entries it needs, by a search from it.
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

  #[test]
  fn the_entries_on_cycles_are_those_that_reach_themselves() {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |below: usize| {
      state ^= state << 13;
      state ^= state >> 7;
      state ^= state << 17;
      (state % below as u64) as usize
    };
    let (mut on_a_cycle, mut off) = (0, 0);
    for _ in 0..20_000 {
      let count = 1 + next(12);
      // few needs, so that some graphs have no cycle, some one and some several
      let needs: Vec<Vec<usize>> = (0..count)
        .map(|index| {
          let mut needed: Vec<usize> = (0..next(3)).map(|_| next(count)).filter(|&other| other != index).collect();
          needed.sort_unstable();
          needed.dedup();
          needed
        })
        .collect();
      let expected: Vec<bool> = (0..count).map(|index| reaches_itself(index, &needs)).collect();
      assert_eq!(expected, on_cycles(&needs), "{needs:?}");
      on_a_cycle += expected.iter().filter(|&&cycle| cycle).count();
      off += expected.iter().filter(|&&cycle| !cycle).count();
    }
    assert!(on_a_cycle > 10_000 && off > 10_000, "{on_a_cycle} on a cycle, {off} not");
  }

  #[test]
  fn a_long_chain_of_entries_is_ordered_in_time_linear_in_its_length() {
    let count = 200_000;
    let needs: Vec<Vec<usize>> = (0..count).map(|index| if index == 0 { vec![] } else { vec![index - 1] }).collect();
    let started = std::time::Instant::now();
    assert!(on_cycles(&needs).iter().all(|&cycle| !cycle));
    // a search from every entry is 20,000,000,000 steps
    assert!(started.elapsed().as_secs() < 20, "took {:?}", started.elapsed());
  }
}
