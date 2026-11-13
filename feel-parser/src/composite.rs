//! Composite names `k1 . k2 . … . kn`: the separator of their keys, and their search in the trees
//! of names of the parsing scope (the contexts pushed while parsing and the indexed input contexts).

use std::collections::HashSet;

/// Separator of the keys in a composite name.
pub(crate) const COMPOSITE_SEPARATOR: &str = " . ";

/// Returns the length of the composite name prefix `p k . ` made of a prefix of length `prefix`,
/// the key `key` and the separator: the prefix of the composite names into the names nested in
/// the value of `key`.
pub(crate) fn nested_prefix(prefix: usize, key: &str) -> usize {
  prefix + key.len() + COMPOSITE_SEPARATOR.len()
}

/// A node of a tree of names: its keys are names, and the value of a key can have names
/// nested in it, a nested node.
pub(crate) trait NameTree: Copy {
  /// Returns `true` when `key` is a key of the node itself.
  fn contains_key(self, key: &str) -> bool;

  /// Calls `f` with the key and the nested node of every key of the node whose value has nested names.
  fn for_each_nested(self, f: impl FnMut(&str, Self));

  /// Returns the address of the node; nodes are never shared, so the address identifies the node.
  fn address(self) -> usize;
}

/// Returns `true` when `name` is one of the flattened keys of the tree `root`: a key at any
/// depth, or a composite key `k1 . k2 . … . kn` where every `k(i+1)` is a key at any depth below `ki`.
///
/// The set of flattened keys grows exponentially with the nesting depth, so it is not built:
/// this is a search over the states (node, offset in `name`), each visited at most once, with an
/// explicit stack, so that a deeply nested tree does not overflow the stack. In state (N, i),
/// `name[i..]` is a flattened key of N when it is a key of N; or a flattened key of a nested node
/// D of N; or `k . rest` for a key `k` of N whose nested node is D and `rest` is a flattened key of D.
pub(crate) fn contains_composite<T: NameTree>(root: T, name: &str) -> bool {
  let mut visited = HashSet::new();
  let mut stack = vec![(root, 0_usize)];
  while let Some((node, offset)) = stack.pop() {
    if !visited.insert((node.address(), offset)) {
      continue;
    }
    work!(CompositeStates, 1);
    let rest = &name[offset..];
    if node.contains_key(rest) {
      return true;
    }
    node.for_each_nested(|key, nested| {
      stack.push((nested, offset));
      if rest.strip_prefix(key).is_some_and(|tail| tail.starts_with(COMPOSITE_SEPARATOR)) {
        stack.push((nested, nested_prefix(offset, key)));
      }
    });
  }
  false
}
