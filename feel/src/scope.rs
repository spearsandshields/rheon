//! Implementation of the `FEEL` scope.

use crate::context::FeelContext;
use crate::values::Value;
use crate::{Name, QualifiedName};
use dsntk_common::Jsonify;
use std::cell::{Ref, RefCell};
use std::fmt;

/// This macro creates a default scope.
#[macro_export]
macro_rules! scope {
  () => {{ FeelScope::default() }};
}

/// The `FEEL` scope.
pub struct FeelScope {
  /// The stack of contexts.
  stack: RefCell<Vec<FeelContext>>,
}

impl Default for FeelScope {
  /// Creates a default [FeelScope] containing single default [FeelContext].
  fn default() -> Self {
    Self {
      stack: RefCell::new(vec![FeelContext::default()]),
    }
  }
}

impl From<FeelContext> for FeelScope {
  /// Creates a [FeelScope] from [FeelContext].
  fn from(ctx: FeelContext) -> Self {
    Self { stack: RefCell::new(vec![ctx]) }
  }
}

impl fmt::Display for FeelScope {
  /// Converts [FeelScope] to text.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[{}]", self.stack.borrow_mut().iter().map(|ctx| ctx.to_string()).collect::<Vec<String>>().join(", "))
  }
}

impl Jsonify for FeelScope {
  /// Converts this [FeelScope] to JSON text.
  fn jsonify(&self) -> String {
    format!("[{}]", self.stack.borrow_mut().iter().map(|ctx| ctx.to_string()).collect::<Vec<String>>().join(", "))
  }
}

impl FeelScope {
  /// Returns the stack of contexts, bottom first, without copying it.
  ///
  /// The scope can not be changed while the returned reference is alive (it would panic).
  pub fn frames(&self) -> Ref<'_, Vec<FeelContext>> {
    self.stack.borrow()
  }

  /// Creates a new and empty [FeelScope].
  pub fn new() -> Self {
    Self { stack: RefCell::new(vec![]) }
  }

  /// Pushes a context on the top of the scope stack.
  pub fn push(&self, ctx: FeelContext) {
    self.stack.borrow_mut().push(ctx)
  }

  /// Appends the content of another scope at the end of this scope.
  pub fn append(&self, other: FeelScope) {
    self.stack.borrow_mut().append(&mut other.stack.borrow_mut());
  }

  /// Takes and returns a context from the top of the stack.
  pub fn pop(&self) -> Option<FeelContext> {
    self.stack.borrow_mut().pop()
  }

  /// Returns the number of contexts on the stack.
  pub fn depth(&self) -> usize {
    self.stack.borrow().len()
  }

  /// Restores the stack to `depth` contexts (a value [FeelScope::depth] returned): removes the
  /// contexts pushed since then and not popped, e.g. by an evaluation that returned early.
  ///
  /// Every evaluation that pushes a context restores the depth this way afterwards, instead of
  /// popping one context, so that a context left behind can not take the place of its own.
  pub fn truncate(&self, depth: usize) {
    self.stack.borrow_mut().truncate(depth);
  }

  /// Restores the stack to `depth` contexts like [FeelScope::truncate] and returns the context
  /// that was pushed when the stack had `depth` contexts; `None` when that context is gone
  /// (more contexts were popped than pushed since, which no evaluator does).
  pub fn pop_frame(&self, depth: usize) -> Option<FeelContext> {
    let mut stack = self.stack.borrow_mut();
    if stack.len() > depth {
      stack.truncate(depth + 1);
      stack.pop()
    } else {
      None
    }
  }

  /// Peeks a context from the top of the stack.
  /// If the stack is empty, the default context is returned.
  pub fn peek(&self) -> Option<FeelContext> {
    self.stack.borrow().last().cloned()
  }

  /// Returns a value of an entry with specified name.
  /// Entries are searched from the last to the first context,
  /// (from top to bottom of the stack).
  pub fn get_value(&self, name: &Name) -> Option<Value> {
    for context in self.stack.borrow().iter().rev() {
      if let Some(value) = context.get_entry(name) {
        return Some(value.clone());
      }
    }
    None
  }

  /// Searches for a value under so called `qualified` name build from
  /// multiple names passed as an argument.
  pub fn search(&self, names: &[Name]) -> Option<Value> {
    for context in self.stack.borrow().iter().rev() {
      if let Some(value) = context.search_deep(names) {
        return Some(value.clone());
      }
    }
    None
  }

  /// Searches for a value of an entry pointed by specified qualified name.
  pub fn search_entry(&self, qname: &QualifiedName) -> Option<Value> {
    for context in self.stack.borrow().iter().rev() {
      if let Some(value) = context.search_entry(qname) {
        return Some(value.clone());
      }
    }
    None
  }

  /// Sets a specified value for entry name in [FeelContext] placed on the top of the scope stack.
  pub fn set_value(&self, name: &Name, value: Value) {
    if let Some(context) = self.stack.borrow_mut().last_mut() {
      context.set_entry(name, value);
    }
  }

  /// Sets a null value for entry name in [FeelContext] placed on the top of the scope stack.
  pub fn set_name(&self, name: Name) {
    if let Some(context) = self.stack.borrow_mut().last_mut() {
      context.set_null(name);
    }
  }
}
