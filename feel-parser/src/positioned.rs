//! A parse with places: the tree of a strict parse, and for its nodes where each stands in the
//! text. For the operations that say something about a part of an expression (which values it
//! reads, what is wrong with it, how its value came about).
//!
//! The parser records a node when it completes it, so the nodes inside a node come before it:
//! the order in which a walk of the finished tree leaves its nodes. The two are paired node by
//! node, and each pair is checked (the kind of the record is the kind of the node). Should they
//! ever not line up, the tree has no places at all ([Positioned::complete] is false) rather
//! than wrong ones.

use crate::ast::AstNode;
use crate::parser::SyntaxNode;
use std::collections::HashMap;

/// The tree of a text and the places of its nodes.
pub struct Positioned {
  /// The tree, as a strict parse gives it. In a box: the places are found by the addresses of
  /// the nodes, which stay while the tree is not changed.
  tree: Box<AstNode>,
  places: HashMap<usize, (u32, u32)>,
  complete: bool,
}

impl Positioned {
  pub(crate) fn new(tree: AstNode, records: Vec<SyntaxNode>) -> Self {
    let tree = Box::new(tree);
    let (places, complete) = match pair(&tree, &records) {
      Ok(places) => (places, true),
      Err(_) => (HashMap::new(), false),
    };
    Self { tree, places, complete }
  }

  /// The tree.
  pub fn tree(&self) -> &AstNode {
    &self.tree
  }

  /// Whether the nodes have their places: false when the records of the parser did not line up
  /// with the tree, and then no node has one.
  pub fn complete(&self) -> bool {
    self.complete
  }

  /// Where `node`, a node of [Positioned::tree], stands in the text: the byte offsets of its
  /// first token and of the end of its last, without parentheses around it. None for a node
  /// that has no place of its own (the list of the parameters of a function, a type).
  pub fn place(&self, node: &AstNode) -> Option<(usize, usize)> {
    self.places.get(&(node as *const AstNode as usize)).map(|&(start, end)| (start as usize, end as usize))
  }

  /// The number of nodes that have a place.
  pub fn len(&self) -> usize {
    self.places.len()
  }

  /// Whether no node has a place.
  pub fn is_empty(&self) -> bool {
    self.places.is_empty()
  }
}

/// The kinds of record that a node completes with: empty for a node that the parser does not
/// record.
fn kinds(node: &AstNode) -> &'static [&'static str] {
  match node {
    AstNode::Add(..) | AstNode::Sub(..) => &["Addition"],
    AstNode::Mul(..) | AstNode::Div(..) => &["Multiplication"],
    AstNode::Exp(..) => &["Exponentiation"],
    AstNode::And(..) => &["Conjunction"],
    AstNode::Or(..) => &["Disjunction"],
    AstNode::Eq(..) | AstNode::Nq(..) | AstNode::Lt(..) | AstNode::Le(..) | AstNode::Gt(..) | AstNode::Ge(..) => &["Comparison"],
    AstNode::Between(..) => &["Between"],
    AstNode::In(..) => &["In"],
    AstNode::InstanceOf(..) => &["InstanceOf"],
    AstNode::As(..) => &["As"],
    AstNode::Path(..) | AstNode::Descendant(..) => &["Path"],
    AstNode::Filter(..) => &["Filter"],
    AstNode::FunctionInvocation(..) => &["Invocation", "Temporal"],
    AstNode::Name(_) => &["Name"],
    AstNode::Numeric(..) => &["Number"],
    AstNode::String(_) => &["String"],
    AstNode::Boolean(_) => &["Boolean"],
    AstNode::Null => &["Null"],
    AstNode::At(_) => &["Temporal"],
    AstNode::Context(_) => &["Context"],
    AstNode::ContextEntry(..) => &["Entry"],
    AstNode::List(_) => &["List"],
    AstNode::Range(..) => &["Interval"],
    AstNode::FunctionDefinition(..) => &["Function"],
    AstNode::If(..) => &["If"],
    AstNode::For(..) => &["For"],
    AstNode::Some(..) | AstNode::Every(..) => &["Quantified"],
    AstNode::Neg(_) => &["Negation"],
    AstNode::UnaryLt(_) | AstNode::UnaryLe(_) | AstNode::UnaryGt(_) | AstNode::UnaryGe(_) | AstNode::UnaryEq(_) | AstNode::UnaryNe(_) => &["UnaryTest"],
    _ => &[],
  }
}

/// The records that stand for no node of the tree: parentheses are no node, and a type is
/// recorded as one piece.
fn passed(kind: &str) -> bool {
  matches!(kind, "Parentheses" | "Type")
}

/// What does not line up: for the tests.
#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct Mismatch {
  pub node: String,
  pub record: Option<&'static str>,
}

/// Pairs the nodes of `tree`, in the order a walk leaves them, with `records`.
pub(crate) fn pair(tree: &AstNode, records: &[SyntaxNode]) -> Result<HashMap<usize, (u32, u32)>, Mismatch> {
  let mut places = HashMap::with_capacity(records.len());
  let mut records = records.iter().filter(|record| !passed(record.kind));
  // a walk with its own stack: a tree can be deeper than the stack of a thread takes
  let mut stack: Vec<(&AstNode, bool)> = vec![(tree, false)];
  while let Some((node, left)) = stack.pop() {
    if !left {
      stack.push((node, true));
      let from = stack.len();
      node.for_each_child(|child| stack.push((child, false)));
      stack[from..].reverse();
      continue;
    }
    let kinds = kinds(node);
    if kinds.is_empty() {
      continue;
    }
    let name = |node: &AstNode| format!("{node:?}").chars().take_while(|ch| ch.is_alphanumeric()).collect::<String>();
    match records.next() {
      Some(record) if kinds.contains(&record.kind) => {
        places.insert(node as *const AstNode as usize, (record.start as u32, record.end as u32));
      }
      Some(record) => {
        return Err(Mismatch {
          node: name(node),
          record: Some(record.kind),
        });
      }
      None => return Err(Mismatch { node: name(node), record: None }),
    }
  }
  match records.next() {
    Some(record) => Err(Mismatch {
      node: "(none)".to_string(),
      record: Some(record.kind),
    }),
    None => Ok(places),
  }
}
