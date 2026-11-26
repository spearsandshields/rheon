//! Implementation of the `FEEL` scope.

use crate::context::FeelContext;
use crate::values::Value;
use crate::{Name, QualifiedName};
use dsntk_common::Jsonify;
use std::cell::{Ref, RefCell};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// This macro creates a default scope.
#[macro_export]
macro_rules! scope {
  () => {{ FeelScope::default() }};
}

/// A context on the stack of a [FeelScope]: a context the scope owns (the parameters of a call,
/// the entries of a context literal, the variables of a loop), or a shared context value pushed
/// without a copy ([FeelScope::push_shared]), such as a context item of a filter.
///
/// A frame is read as its context ([Deref]). A write into a shared frame copies the context
/// first ([FeelScope::set_value], [FeelScope::set_name]), so it never shows through the value the
/// frame was pushed from.
pub struct ScopeFrame(Frame);

enum Frame {
  Owned(FeelContext),
  Shared(Arc<FeelContext>),
}

impl ScopeFrame {
  /// The context of this frame, to be changed: a copy of it, which the frame then holds alone,
  /// when it is shared.
  fn context_mut(&mut self) -> &mut FeelContext {
    match &mut self.0 {
      Frame::Owned(ctx) => ctx,
      Frame::Shared(ctx) => Arc::make_mut(ctx),
    }
  }

  /// True when this frame holds `context` itself, shared and not written into since it was
  /// pushed ([FeelScope::push_shared]): what is known of `context` is then true of the frame.
  pub fn shares(&self, context: &Arc<FeelContext>) -> bool {
    matches!(&self.0, Frame::Shared(held) if Arc::ptr_eq(held, context))
  }

  /// The context of this frame: moved out of a frame that holds it alone, copied otherwise.
  pub fn into_context(self) -> FeelContext {
    match self.0 {
      Frame::Owned(ctx) => ctx,
      Frame::Shared(ctx) => Arc::unwrap_or_clone(ctx),
    }
  }
}

impl Deref for ScopeFrame {
  type Target = FeelContext;
  fn deref(&self) -> &FeelContext {
    match &self.0 {
      Frame::Owned(ctx) => ctx,
      Frame::Shared(ctx) => ctx,
    }
  }
}

impl From<FeelContext> for ScopeFrame {
  /// A frame that owns the context.
  fn from(ctx: FeelContext) -> Self {
    Self(Frame::Owned(ctx))
  }
}

impl fmt::Debug for ScopeFrame {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&**self, f)
  }
}

/// The `FEEL` scope.
pub struct FeelScope {
  /// The stack of contexts.
  stack: RefCell<Vec<ScopeFrame>>,
}

impl Default for FeelScope {
  /// Creates a default [FeelScope] containing single default [FeelContext].
  fn default() -> Self {
    Self {
      stack: RefCell::new(vec![FeelContext::default().into()]),
    }
  }
}

impl From<FeelContext> for FeelScope {
  /// Creates a [FeelScope] from [FeelContext].
  fn from(ctx: FeelContext) -> Self {
    Self {
      stack: RefCell::new(vec![ctx.into()]),
    }
  }
}

impl fmt::Display for FeelScope {
  /// Converts [FeelScope] to text.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[{}]", self.stack.borrow_mut().iter().map(|ctx| ctx.to_string()).collect::<Vec<String>>().join(", "))
  }
}

impl Jsonify for FeelScope {
  /// The contexts of this [FeelScope], bottom first, as a JSON array of objects.
  fn jsonify(&self) -> String {
    format!("[{}]", self.stack.borrow_mut().iter().map(|ctx| ctx.jsonify()).collect::<Vec<String>>().join(", "))
  }
}

impl FeelScope {
  /// Returns a copy of the stack of contexts, bottom first.
  pub fn contexts(&self) -> Vec<FeelContext> {
    self.stack.borrow().iter().map(|frame| FeelContext::clone(frame)).collect()
  }

  /// Returns the stack of contexts, bottom first, without copying it.
  ///
  /// The scope can not be changed while the returned reference is alive (it would panic).
  pub fn frames(&self) -> Ref<'_, [ScopeFrame]> {
    Ref::map(self.stack.borrow(), Vec::as_slice)
  }

  /// Creates a new and empty [FeelScope].
  pub fn new() -> Self {
    Self { stack: RefCell::new(vec![]) }
  }

  /// Pushes a context on the top of the scope stack.
  pub fn push(&self, ctx: FeelContext) {
    self.stack.borrow_mut().push(ctx.into())
  }

  /// Pushes a shared context on the top of the scope stack, without a copy: a write into the
  /// frame copies it first ([ScopeFrame]).
  pub fn push_shared(&self, ctx: Arc<FeelContext>) {
    self.stack.borrow_mut().push(ScopeFrame(Frame::Shared(ctx)))
  }

  /// Appends the content of another scope at the end of this scope.
  pub fn append(&self, other: FeelScope) {
    self.stack.borrow_mut().append(&mut other.stack.borrow_mut());
  }

  /// Takes and returns a context from the top of the stack (a copy of it when it is shared).
  pub fn pop(&self) -> Option<FeelContext> {
    self.stack.borrow_mut().pop().map(ScopeFrame::into_context)
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

  /// Restores the stack to `depth` contexts like [FeelScope::truncate] and returns the frame
  /// above them, the one pushed when the stack had `depth` contexts; `None` when the stack has
  /// no more than `depth` contexts (more were popped than pushed since, which no evaluator does).
  ///
  /// Only the number of contexts is checked: a context that was popped and replaced by another
  /// one is not detected, and the other one is returned.
  pub fn pop_frame(&self, depth: usize) -> Option<ScopeFrame> {
    let mut stack = self.stack.borrow_mut();
    if stack.len() > depth {
      stack.truncate(depth + 1);
      stack.pop()
    } else {
      None
    }
  }

  /// Returns a copy of the context on the top of the stack, `None` when the stack is empty.
  pub fn peek(&self) -> Option<FeelContext> {
    self.stack.borrow().last().map(|frame| FeelContext::clone(frame))
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

  /// Sets a specified value for entry name in [FeelContext] placed on the top of the scope stack
  /// (in a copy of it when the frame is shared, see [ScopeFrame]).
  ///
  /// Evaluators write only into the frames they pushed (a context literal writes its entries
  /// into its own frame), except a boxed context, which writes each entry into the frame on the
  /// top of the scope, whatever pushed it: the one write into a frame that may be shared.
  pub fn set_value(&self, name: &Name, value: Value) {
    if let Some(frame) = self.stack.borrow_mut().last_mut() {
      frame.context_mut().set_entry(name, value);
    }
  }

  /// Sets a null value for entry name in [FeelContext] placed on the top of the scope stack (in a
  /// copy of it when the frame is shared, see [ScopeFrame]).
  pub fn set_name(&self, name: Name) {
    if let Some(frame) = self.stack.borrow_mut().last_mut() {
      frame.context_mut().set_null(name);
    }
  }
}
