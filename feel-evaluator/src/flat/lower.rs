//! # Lowering
//!
//! The instructions of a resolved expression. Every node becomes the instructions that compute
//! what the evaluator of closures computes for it, emitted in the order in which that evaluator
//! evaluates the parts of the node, so that errors, warnings and limits are recorded in the same
//! order. A value known when the expression is built and whose evaluation records nothing is a
//! constant; any other value is computed into a temporary slot, which its one reader moves out.
//! A node that is not lowered yet refuses the whole expression ([NotYet]).

use super::NotYet;
use super::program::{
  CallInfo, ClosureInfo, Compare, Domain, FilterInfo, Fold, Kernel1, Kernel2, ListKind, LoopInfo, LoopKind, NONE, Op, Opnd, Pc, ProbeInfo, Program, ProgramBuilder, Proto,
  SearchInfo, Slot,
};
use super::resolve::{Info, Place, Resolved, Stop, Target, variables};
use crate::filters::ITEM;
use crate::kernels::{self, Arithmetic};
use crate::{Dialect, builders};
use rheon_feel::Name;
use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::sync::Arc;

/// The program of the resolved expression `resolved` in `dialect`: the code of the root
/// expression, then that of the body of each function definition in it.
pub(crate) fn lower(resolved: Resolved<'_>, dialect: Dialect) -> Result<Arc<Program>, NotYet<'_>> {
  // the binders first, then the temporaries
  let binders = resolved.root.binders;
  let mut lower = Lower {
    resolved: &resolved,
    dialect,
    builder: ProgramBuilder::with_capacity(resolved.nodes.len()),
    next: binders,
    max: binders,
    protos: vec![],
    bodies: vec![],
    proto: NONE,
    cells: vec![],
    cell_names: vec![],
  };
  let result = lower.value(resolved.root(), None)?;
  lower.builder.emit(Op::Return { src: result });
  let root_cells = lower.cell_names.len() as u32;
  lower.builder.root_cells(root_cells);
  let root_end = lower.builder.here();
  let root_slots = lower.max;
  // the bodies one after the other, so that no body is lowered inside another
  let mut done = 0;
  while let Some(function) = lower.bodies.get(done).map(|body| body.function) {
    lower.body(done, function)?;
    done += 1;
  }
  let Lower { mut builder, bodies, .. } = lower;
  let Resolved {
    names,
    layouts,
    regions,
    mut functions,
    ..
  } = resolved;
  for (
    index,
    Body {
      function,
      entry,
      end,
      slots,
      cells,
    },
  ) in bodies.into_iter().enumerate()
  {
    let function = &mut functions[function as usize];
    let proto = Proto {
      entry,
      end,
      slots,
      cells,
      params: std::mem::take(&mut function.params),
      region: function.region,
      closure: std::mem::take(&mut function.closure),
      captures: std::mem::take(&mut function.captures),
    };
    builder.proto(index as u32, proto);
  }
  builder.names(names.into_iter().map(|name| name.into_owned()).collect());
  Ok(builder.finish(root_slots, root_end, dialect, layouts, regions))
}

/// No proto has read a free name yet.
const UNSET: u32 = NONE - 1;

struct Lower<'r, 'a> {
  resolved: &'r Resolved<'a>,
  dialect: Dialect,
  builder: ProgramBuilder,
  /// The first free temporary.
  next: Slot,
  /// The slots used so far.
  max: u32,
  /// The index in the program of the proto of each function of the resolution, [NONE] until its
  /// definition is lowered.
  protos: Vec<u32>,
  /// The protos of the program in order.
  bodies: Vec<Body>,
  /// The proto being lowered, [NONE] for the root expression.
  proto: u32,
  /// For each free name, the proto and the cell it has there once read there.
  cells: Vec<(u32, u32)>,
  /// The name of each cell of the proto being lowered.
  cell_names: Vec<u32>,
}

/// A proto of the program: the function of the resolution, and once its body is lowered, where
/// its code starts and ends, the slots of its frame, and the name and the cell of each free name
/// it reads, by name.
struct Body {
  function: u32,
  entry: Pc,
  end: Pc,
  slots: u32,
  cells: Box<[(u32, u32)]>,
}

impl<'a> Lower<'_, 'a> {
  fn temporary(&mut self) -> Slot {
    let slot = self.next;
    self.next += 1;
    self.max = self.max.max(self.next);
    slot
  }

  /// The operand of the value of `node`: a constant, or a temporary that the instructions emitted
  /// here compute (`dst` when given).
  fn value(&mut self, node: u32, dst: Option<Slot>) -> Result<Opnd, NotYet<'a>> {
    let ast = self.resolved.nodes[node as usize].ast;
    if let Some(constant) = self.constant(ast) {
      return Ok(constant);
    }
    if let Some(binder) = self.binder(node) {
      return Ok(binder);
    }
    let dst = dst.unwrap_or_else(|| self.temporary());
    let mark = self.next;
    match ast {
      // a literal without a value (one with a value is a constant)
      AstNode::Numeric(..) | AstNode::At(_) => {
        if let Some(Err(text)) = literal(ast) {
          self.fail(dst, text);
        }
      }
      AstNode::Name(_) | AstNode::QualifiedName(_) => self.read(node, dst)?,
      AstNode::Path(_, entry) | AstNode::Descendant(_, entry) => match self.resolved.nodes[node as usize].info {
        Info::Entry { name } => {
          let a = self.value(self.resolved.kids(node)[0], None)?;
          self.builder.emit(match ast {
            AstNode::Path(..) => Op::Path { dst, a, name },
            _ => Op::Descendant { dst, a, name },
          });
        }
        _ => self.fail(dst, crate::builders::err_expected_name(entry)),
      },
      AstNode::Add(..) | AstNode::Sub(..) | AstNode::Mul(..) | AstNode::Div(..) | AstNode::Exp(..) => {
        let op = match ast {
          AstNode::Add(..) => Arithmetic::Addition,
          AstNode::Sub(..) => Arithmetic::Subtraction,
          AstNode::Mul(..) => Arithmetic::Multiplication,
          AstNode::Div(..) => Arithmetic::Division,
          _ => Arithmetic::Exponentiation,
        };
        let (a, b) = self.operands(node)?;
        // B-FEEL arithmetic converts mismatched operands (DMN 1.6 chapter 11)
        self.builder.emit(match self.dialect {
          Dialect::Feel => Op::Arith { op, dst, a, b },
          Dialect::BFeel => Op::ArithB { op, dst, a, b },
        });
      }
      AstNode::Neg(_) => {
        let a = self.value(self.resolved.kids(node)[0], None)?;
        self.builder.emit(Op::Neg { dst, a });
      }
      AstNode::Eq(..) | AstNode::Nq(..) | AstNode::Lt(..) | AstNode::Le(..) | AstNode::Gt(..) | AstNode::Ge(..) => {
        let op = match ast {
          AstNode::Eq(..) => Compare::Eq,
          AstNode::Nq(..) => Compare::Ne,
          AstNode::Lt(..) => Compare::Lt,
          AstNode::Le(..) => Compare::Le,
          AstNode::Gt(..) => Compare::Gt,
          _ => Compare::Ge,
        };
        let (a, b) = self.operands(node)?;
        self.builder.emit(Op::Compare { op, dst, a, b });
        self.boolean_default(dst, op == Compare::Ne);
      }
      AstNode::Between(..) => {
        let args = self.temporary();
        self.temporary();
        self.temporary();
        let resolved = self.resolved;
        for (slot, &kid) in (args..).zip(resolved.kids(node)) {
          self.into(kid, slot)?;
        }
        self.builder.emit(Op::Between { dst, args });
        self.boolean_default(dst, false);
      }
      AstNode::And(..) | AstNode::Or(..) => {
        // the right operand only when the left one does not decide (DMN 1.6, Table 51)
        let kids = self.resolved.kids(node);
        let (lhs, rhs) = (kids[0], kids[1]);
        let a = self.value(lhs, None)?;
        let left = self.builder.emit(match ast {
          AstNode::And(..) => Op::AndLeft { dst, a, to: 0 },
          _ => Op::OrLeft { dst, a, to: 0 },
        });
        self.next = mark;
        let b = self.value(rhs, None)?;
        self.builder.emit(match ast {
          AstNode::And(..) => Op::AndRight { dst, b },
          _ => Op::OrRight { dst, b },
        });
        self.builder.patch(left);
        self.boolean_default(dst, false);
      }
      AstNode::If(..) => {
        let kids = self.resolved.kids(node);
        let (condition, then, otherwise) = (kids[0], kids[1], kids[2]);
        let c = self.value(condition, None)?;
        let test = self.builder.emit(Op::JumpUnlessTrue { c, to: 0 });
        self.next = mark;
        self.into(then, dst)?;
        self.next = mark;
        let skip = self.builder.emit(Op::Jump { to: 0 });
        self.builder.patch(test);
        self.into(otherwise, dst)?;
        self.builder.patch(skip);
      }
      AstNode::List(_) | AstNode::ExpressionList(_) | AstNode::NegatedList(_) | AstNode::ParameterTypes(_) => {
        let kind = match ast {
          AstNode::List(_) => ListKind::List,
          AstNode::ExpressionList(_) => ListKind::ExpressionList,
          AstNode::NegatedList(_) => ListKind::NegatedList,
          _ => ListKind::ParameterTypes,
        };
        let resolved = self.resolved;
        let items = resolved.kids(node);
        self.builder.emit(Op::ListNew { len: items.len() as u32 });
        for &item in items {
          let src = self.value(item, None)?;
          self.builder.emit(Op::ListPush { src });
          self.next = mark;
        }
        self.builder.emit(Op::ListEnd { kind, dst });
      }
      AstNode::In(..) | AstNode::Range(..) | AstNode::InstanceOf(..) | AstNode::ContextTypeEntry(..) | AstNode::FunctionType(..) => {
        let (a, b) = self.operands(node)?;
        let kernel = match ast {
          AstNode::In(..) => Kernel2::In,
          AstNode::Range(..) => Kernel2::Range,
          AstNode::InstanceOf(..) => Kernel2::InstanceOf,
          AstNode::ContextTypeEntry(..) => Kernel2::ContextTypeEntry,
          _ => Kernel2::FunctionType,
        };
        self.builder.emit(Op::Kernel2 { kernel, dst, a, b });
        if kernel == Kernel2::In {
          self.boolean_default(dst, false);
        }
      }
      AstNode::Out(..) => {
        // the whole `in` (never with the B-FEEL default), then the left operand again
        let (a, b) = self.operands(node)?;
        let test = self.temporary();
        self.builder.emit(Op::Kernel2 {
          kernel: Kernel2::In,
          dst: test,
          a,
          b,
        });
        let lhs = self.value(self.resolved.kids(node)[0], None)?;
        self.builder.emit(Op::Kernel2 {
          kernel: Kernel2::Out,
          dst,
          a: Opnd::take(test),
          b: lhs,
        });
      }
      AstNode::UnaryEq(_)
      | AstNode::UnaryNe(_)
      | AstNode::UnaryLt(_)
      | AstNode::UnaryLe(_)
      | AstNode::UnaryGt(_)
      | AstNode::UnaryGe(_)
      | AstNode::IntervalStart(..)
      | AstNode::IntervalEnd(..)
      | AstNode::ListType(_)
      | AstNode::RangeType(_) => {
        let kernel = match ast {
          AstNode::UnaryEq(_) => Kernel1::UnaryEq,
          AstNode::UnaryNe(_) => Kernel1::UnaryNe,
          AstNode::UnaryLt(_) => Kernel1::UnaryLt,
          AstNode::UnaryLe(_) => Kernel1::UnaryLe,
          AstNode::UnaryGt(_) => Kernel1::UnaryGt,
          AstNode::UnaryGe(_) => Kernel1::UnaryGe,
          AstNode::IntervalStart(_, closed) => Kernel1::IntervalStart(*closed),
          AstNode::IntervalEnd(_, closed) => Kernel1::IntervalEnd(*closed),
          AstNode::ListType(_) => Kernel1::ListType,
          AstNode::RangeType(_) => Kernel1::RangeType,
          _ => return Err(NotYet { at: ast }),
        };
        let a = self.value(self.resolved.kids(node)[0], None)?;
        self.builder.emit(Op::Kernel1 { kernel, dst, a });
      }
      // the entries or the parameters in order, up to the first that is none
      AstNode::ContextType(_) => self.fold(node, Fold::ContextType, dst)?,
      AstNode::FormalParameters(_) => self.fold(node, Fold::Parameters, dst)?,
      AstNode::FormalParameter(..) => {
        let (a, b) = self.operands(node)?;
        self.builder.emit(Op::Kernel2 {
          kernel: Kernel2::FormalParameter,
          dst,
          a,
          b,
        });
      }
      AstNode::FunctionBody(_, true) => {
        let a = self.value(self.resolved.kids(node)[0], None)?;
        self.builder.emit(Op::Kernel1 {
          kernel: Kernel1::ExternalBody,
          dst,
          a,
        });
      }
      AstNode::FunctionDefinition(..) => self.function(node, dst)?,
      AstNode::FunctionInvocation(_, arguments) => self.invocation(node, arguments, dst)?,
      AstNode::Context(_) => self.context(node, dst)?,
      AstNode::For(..) | AstNode::Some(..) | AstNode::Every(..) => self.iteration(node, dst)?,
      AstNode::Filter(..) => self.filter(node, dst)?,
      AstNode::EvaluatedExpression(_) => self.into(self.resolved.kids(node)[0], dst)?,
      _ => return Err(NotYet { at: ast }),
    }
    self.next = mark;
    Ok(Opnd::take(dst))
  }

  /// The values of the children of `node` added in order to a fold of `kind`, which stops at the
  /// first value that is not of its kind.
  fn fold(&mut self, node: u32, kind: Fold, dst: Slot) -> Result<(), NotYet<'a>> {
    let resolved = self.resolved;
    let mark = self.next;
    self.builder.emit(Op::FoldNew { kind });
    let mut fails = vec![];
    for &kid in resolved.kids(node) {
      let src = self.value(kid, None)?;
      fails.push(self.builder.emit(Op::FoldAdd { src, dst, fail: 0 }));
      self.next = mark;
    }
    self.builder.emit(Op::FoldEnd { dst });
    for fail in fails {
      self.builder.patch(fail);
    }
    Ok(())
  }

  /// A function definition: its formal parameters, then the mapping of an external body (even
  /// when a parameter failed), then the function; the body of FEEL is lowered after the root
  /// expression, once however often its definition is lowered.
  fn function(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let (Info::Function { proto: function }, AstNode::FunctionDefinition(_, body)) = (&rnode.info, rnode.ast) else {
      return Err(NotYet { at: rnode.ast });
    };
    if *function == NONE {
      // a definition whose parts are not of their shapes
      return Err(NotYet { at: rnode.ast });
    }
    let kids = resolved.kids(node);
    let parameters = self.value(kids[0], None)?;
    let body = match body.as_ref() {
      AstNode::FunctionBody(_, true) => Some(self.value(kids[1], None)?),
      _ => None,
    };
    if self.protos.is_empty() {
      self.protos = vec![NONE; resolved.functions.len()];
    }
    let proto = match self.protos[*function as usize] {
      NONE => {
        let proto = self.bodies.len() as u32;
        self.protos[*function as usize] = proto;
        self.bodies.push(Body {
          function: *function,
          entry: 0,
          end: 0,
          slots: 0,
          cells: Box::new([]),
        });
        proto
      }
      proto => proto,
    };
    let closure = self.builder.closure(ClosureInfo {
      proto,
      parameters,
      body,
      region: rnode.region,
    });
    self.builder.emit(Op::Closure { dst, closure });
    Ok(())
  }

  /// The code of the body of the proto `proto`, the function `function` of the resolution: its
  /// value, returned; an external body has no code but a return of null.
  fn body(&mut self, proto: usize, function: u32) -> Result<(), NotYet<'a>> {
    let resolved = self.resolved;
    let function = &resolved.functions[function as usize];
    self.next = function.binders;
    self.max = function.binders;
    self.proto = proto as u32;
    self.cell_names.clear();
    let entry = self.builder.here();
    let result = if function.body == NONE {
      self.builder.konst(Value::Null(None))
    } else {
      self.value(function.body, None)?
    };
    self.builder.emit(Op::Return { src: result });
    let mut cells: Vec<(u32, u32)> = self.cell_names.iter().copied().zip(0..).collect();
    cells.sort_unstable();
    let body = &mut self.bodies[proto];
    (body.entry, body.end, body.slots, body.cells) = (entry, self.builder.here(), self.max, cells.into());
    Ok(())
  }

  /// The cell of the free name `free` in the root expression or the proto being lowered.
  fn cell(&mut self, free: u32) -> u32 {
    if self.cells.is_empty() {
      self.cells = vec![(UNSET, NONE); self.resolved.frees.len()];
    }
    let (proto, cell) = &mut self.cells[free as usize];
    if *proto != self.proto {
      (*proto, *cell) = (self.proto, self.cell_names.len() as u32);
      self.cell_names.push(self.resolved.frees[free as usize].name);
    }
    *cell
  }

  /// An invocation, between the instructions that enter and leave it: the function, then the
  /// arguments in the order written; a malformed invocation is the builder's error, which a call
  /// beyond the call depth limit does not record either.
  fn invocation(&mut self, node: u32, arguments: &AstNode, dst: Slot) -> Result<(), NotYet<'a>> {
    let enter = self.builder.emit(Op::Enter { dst, skip: 0 });
    let names: Result<Option<Box<[Name]>>, String> = match arguments {
      AstNode::PositionalParameters(_) => Ok(None),
      AstNode::NamedParameters(items) => items
        .iter()
        .map(|item| match item {
          AstNode::NamedParameter(name, _) => match name.as_ref() {
            AstNode::ParameterName(name) => Ok(name.clone()),
            other => Err(builders::err_expected_parameter_name(other)),
          },
          other => Err(builders::err_msg_expected_node("NamedParameter", other)),
        })
        .collect::<Result<_, _>>()
        .map(Some),
      other => Err(builders::err_msg_expected_positional_or_named_parameter(other)),
    };
    match names {
      Ok(names) => {
        let resolved = self.resolved;
        let kids = resolved.kids(node);
        let function = self.value(kids[0], None)?;
        let args = self.next;
        let argc = kids.len() as u32 - 1;
        for _ in 0..argc {
          self.temporary();
        }
        for (slot, &kid) in (args..).zip(&kids[1..]) {
          self.into(kid, slot)?;
        }
        let names = match names {
          Some(names) => self.builder.name_list(names),
          None => NONE,
        };
        let region = self.resolved.nodes[node as usize].region;
        let call = self.builder.call(CallInfo {
          function,
          args,
          argc,
          names,
          region,
        });
        self.builder.emit(Op::Call { dst, call });
      }
      Err(text) => self.fail(dst, text),
    }
    self.builder.emit(Op::Leave { dst });
    self.builder.patch(enter);
    Ok(())
  }

  /// A context literal: the value of each entry in the slot of its binder, which the entries after
  /// it read, nested no deeper than values may nest; at a repeated key, after the value of its
  /// entry, the null of the repeated key, and the entries before it are dropped.
  fn context(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let Info::Context { layout, stop } = rnode.info else {
      return Err(NotYet { at: rnode.ast });
    };
    let entries = resolved.kids(node);
    let bound = &resolved.layouts[layout as usize].entries;
    for (&entry, &(_, place)) in entries.iter().zip(bound.iter()) {
      let Place::Slot(slot) = place else {
        return Err(NotYet { at: rnode.ast });
      };
      self.into(resolved.kids(entry)[1], slot)?;
      self.builder.emit(Op::BoundDepth { s: slot });
    }
    match stop {
      Stop::None => {
        self.builder.emit(Op::Context { dst, layout });
      }
      Stop::Duplicate(index) => {
        let entry = entries[index as usize];
        self.into(resolved.kids(entry)[1], dst)?;
        let AstNode::ContextEntry(key, _) = resolved.nodes[entry as usize].ast else {
          return Err(NotYet { at: rnode.ast });
        };
        let AstNode::ContextEntryKey(name) = key.as_ref() else {
          return Err(NotYet { at: rnode.ast });
        };
        self.fail(dst, kernels::err_duplicated_context_entry_key(name));
        self.builder.emit(Op::Clear { layout, visible: index });
      }
      // an entry whose value may bind any name
      Stop::Irregular(_) => return Err(NotYet { at: rnode.ast }),
    }
    Ok(())
  }

  /// A filter: the list, then the predicate, evaluated in the scope around the filter and then,
  /// unless it selected an item by its index, for each item.
  fn filter(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let Info::Filter { record } = rnode.info else {
      return Err(NotYet { at: rnode.ast });
    };
    let kids = resolved.kids(node);
    let mark = self.next;
    let list = self.value(kids[0], None)?;
    let info = self.builder.filter(FilterInfo { dst, body: 0, close: 0, after: 0 });
    self.builder.emit(Op::FilterOpen { record, info, list });
    self.next = mark;
    let body = self.builder.here();
    let c = self.value(kids[1], None)?;
    self.builder.emit(Op::FilterStep { record, c });
    self.next = mark;
    let close = self.builder.emit(Op::FilterClose { record });
    let after = self.builder.here();
    let info = self.builder.filter_mut(info);
    info.body = body;
    info.close = close;
    info.after = after;
    Ok(())
  }

  /// A `for`, `some` or `every` expression: the domain of the first variable, then the walk over
  /// the combinations of the values of the variables, in which each new value of a variable is
  /// followed by the domain of the next variable or, for the last, by the body.
  fn iteration(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let (kind, contexts) = match rnode.ast {
      AstNode::For(contexts, _) => (LoopKind::For, contexts),
      AstNode::Some(contexts, _) => (LoopKind::Some, contexts),
      AstNode::Every(contexts, _) => (LoopKind::Every, contexts),
      _ => return Err(NotYet { at: rnode.ast }),
    };
    let Info::Iteration { layout, record, duplicate } = rnode.info else {
      self.malformed_quantified(rnode.ast, dst);
      return Ok(());
    };
    if duplicate != NONE {
      self.fail(dst, builders::err_duplicate_iteration_variable(&resolved.names[duplicate as usize]));
      return Ok(());
    }
    let variables = variables(contexts, kind != LoopKind::For);
    if variables.is_empty() {
      // no iteration: nothing is evaluated
      let value = match kind {
        LoopKind::For => Value::List(Default::default()),
        LoopKind::Some => Value::Boolean(false),
        LoopKind::Every => Value::Boolean(true),
      };
      let value = self.builder.konst(value);
      self.builder.emit(Op::Move { dst, src: value });
      return Ok(());
    }
    let mut levels = vec![];
    for &(_, place) in &resolved.layouts[layout as usize].entries[..variables.len()] {
      let Place::Slot(slot) = place else {
        return Err(NotYet { at: rnode.ast });
      };
      levels.push((slot, 0));
    }
    let mark = self.next;
    let kids = resolved.kids(node);
    let mut parts = kids.iter().copied();
    let (domain, a, b) = self.domain(kind, variables[0].1.len(), &mut parts)?;
    let info = self.builder.iteration(LoopInfo {
      kind,
      step: 0,
      exit: 0,
      levels: Box::new([]),
    });
    self.builder.emit(Op::LoopOpen { record, info });
    self.builder.emit(Op::LevelPush { record, domain, a, b });
    self.next = mark;
    let step = self.builder.emit(Op::Step { record });
    for (level, (_, domain_parts)) in variables.iter().enumerate().skip(1) {
      levels[level - 1].1 = self.builder.here();
      let (domain, a, b) = self.domain(kind, domain_parts.len(), &mut parts)?;
      self.builder.emit(Op::LevelPush { record, domain, a, b });
      self.next = mark;
    }
    let last = levels.len() - 1;
    levels[last].1 = self.builder.here();
    let body = parts.next().ok_or(NotYet { at: rnode.ast })?;
    let src = self.value(body, None)?;
    self.builder.emit(Op::Visit { record, src });
    self.next = mark;
    let exit = self.builder.emit(Op::LoopClose { record, dst });
    let info = self.builder.iteration_mut(info);
    info.step = step;
    info.exit = exit;
    info.levels = levels.into();
    Ok(())
  }

  /// The domain of a variable whose domain has `count` parts, the next of `parts`: an interval
  /// from its two parts, a range literal of a `for` from its start and end, or the items of a
  /// value.
  fn domain(&mut self, kind: LoopKind, count: usize, parts: &mut impl Iterator<Item = u32>) -> Result<(Domain, Opnd, Opnd), NotYet<'a>> {
    let resolved = self.resolved;
    let first = parts.next().ok_or(NotYet {
      at: resolved.nodes[resolved.root() as usize].ast,
    })?;
    if count == 2 {
      let second = parts.next().ok_or(NotYet {
        at: resolved.nodes[first as usize].ast,
      })?;
      let a = self.value(first, None)?;
      let b = self.value(second, None)?;
      return Ok((Domain::Interval, a, b));
    }
    if kind == LoopKind::For
      && let AstNode::Range(..) = resolved.nodes[first as usize].ast
    {
      let kids = resolved.kids(first);
      let a = self.value(kids[0], None)?;
      let b = self.value(kids[1], None)?;
      return Ok((Domain::Range, a, b));
    }
    let a = self.value(first, None)?;
    Ok((Domain::Of, a, a))
  }

  /// A `some` or `every` expression whose parts are not of their shapes: the builder's error.
  fn malformed_quantified(&mut self, ast: &AstNode, dst: Slot) {
    let (AstNode::Some(contexts, satisfies) | AstNode::Every(contexts, satisfies)) = ast else {
      return;
    };
    let text = if !matches!(contexts.as_ref(), AstNode::QuantifiedContexts(_)) {
      builders::err_msg_expected_node("QuantifiedContexts", contexts)
    } else {
      let mut names = std::collections::HashSet::new();
      match variables(contexts, true).into_iter().find(|(name, _)| !names.insert(*name)) {
        Some((name, _)) => builders::err_duplicate_iteration_variable(name),
        None => builders::err_msg_expected_node("Satisfies", satisfies),
      }
    };
    self.fail(dst, text);
  }

  /// The operand of the binder that `node` reads, when it is a name that reads one in a slot.
  fn binder(&self, node: u32) -> Option<Opnd> {
    match &self.resolved.nodes[node as usize].info {
      Info::Read(read) if read.probes.is_empty() => match read.target {
        Target::Place(Place::Slot(slot)) => Some(Opnd::slot(slot)),
        _ => None,
      },
      _ => None,
    }
  }

  /// The value of `node` into the slot `dst`.
  fn into(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let src = self.value(node, Some(dst))?;
    if src != Opnd::take(dst) {
      self.builder.emit(Op::Move { dst, src });
    }
    Ok(())
  }

  /// The operands of the two children of `node`, evaluated in order.
  fn operands(&mut self, node: u32) -> Result<(Opnd, Opnd), NotYet<'a>> {
    let kids = self.resolved.kids(node);
    let (lhs, rhs) = (kids[0], kids[1]);
    Ok((self.value(lhs, None)?, self.value(rhs, None)?))
  }

  /// B-FEEL: a boolean operator never gives null, but `ne` (DMN 1.6 chapter 11).
  fn boolean_default(&mut self, dst: Slot, ne: bool) {
    if self.dialect == Dialect::BFeel {
      self.builder.emit(Op::BoolDefault { dst, ne });
    }
  }

  /// `dst` is null with the error `text`, recorded at every evaluation.
  fn fail(&mut self, dst: Slot, text: String) {
    let msg = self.builder.message(text);
    self.builder.emit(Op::Fail { dst, msg });
  }

  /// A name read into `dst`: in the root expression, from the scope of the host.
  fn read(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let rnode = &self.resolved.nodes[node as usize];
    let Info::Read(read) = &rnode.info else {
      // a qualified name with a part that is no segment
      return Err(NotYet { at: rnode.ast });
    };
    // the items of the filters around it first, innermost first: `item`, or an entry of the name
    let item = read.name != NONE && *self.resolved.names[read.name as usize] == *ITEM;
    let mut found = Vec::with_capacity(read.probes.len());
    for &record in read.probes.iter() {
      found.push(if item {
        self.builder.emit(Op::ItemSelf { dst, record, found: 0 })
      } else {
        let probe = self.builder.probe(ProbeInfo { record, name: read.name });
        self.builder.emit(Op::ProbeItem { dst, probe, found: 0 })
      });
    }
    match (rnode.ast, read.target) {
      (_, Target::Place(Place::Slot(slot))) => {
        self.builder.emit(Op::Move { dst, src: Opnd::slot(slot) });
      }
      (_, Target::Place(Place::Partial(record))) => {
        self.builder.emit(Op::LoadPartial { dst, record });
      }
      (AstNode::Name(_), Target::Free(free)) => {
        let name = self.resolved.frees[free as usize].name;
        let cell = self.cell(free);
        self.builder.emit(Op::LoadName { dst, name, cell });
      }
      (AstNode::QualifiedName(segments), Target::Free(_) | Target::Search) => {
        let segments = segments
          .iter()
          .filter_map(|segment| match segment {
            AstNode::QualifiedNameSegment(name) => Some(name.clone()),
            _ => None,
          })
          .collect();
        let segments = self.builder.name_list(segments);
        // a free name is in no scope of the body; a search may be in any
        let region = match read.target {
          Target::Search => rnode.region,
          _ => NONE,
        };
        let search = self.builder.search(SearchInfo {
          segments,
          first: read.name,
          region,
        });
        self.builder.emit(Op::Search { dst, search });
      }
      _ => return Err(NotYet { at: rnode.ast }),
    }
    for at in found {
      self.builder.patch(at);
    }
    Ok(())
  }

  /// The constant of `ast` when its value is known now: a literal whose evaluation records nothing
  /// but a warning, which is recorded where it is evaluated.
  fn constant(&mut self, ast: &AstNode) -> Option<Opnd> {
    let value = match ast {
      AstNode::Boolean(value) => Value::Boolean(*value),
      AstNode::Null => Value::Null(None),
      AstNode::String(text) => Value::String(text.as_str().into()),
      AstNode::Irrelevant => Value::Irrelevant,
      AstNode::FeelType(feel_type) => Value::FeelType(feel_type.clone()),
      AstNode::ContextTypeEntryKey(name) => Value::ContextTypeEntryKey(name.clone()),
      AstNode::ParameterName(name) => Value::ParameterName(name.clone()),
      AstNode::FormalParameter(name, parameter_type) => match (name.as_ref(), parameter_type.as_ref()) {
        (AstNode::ParameterName(name), AstNode::FeelType(parameter_type)) => Value::FormalParameter(name.clone(), parameter_type.clone()),
        _ => return None,
      },
      // parameters whose types are named types of the grammar
      AstNode::FormalParameters(items) => Value::FormalParameters(
        items
          .iter()
          .map(|item| match item {
            AstNode::FormalParameter(name, parameter_type) => match (name.as_ref(), parameter_type.as_ref()) {
              (AstNode::ParameterName(name), AstNode::FeelType(parameter_type)) => Some((name.clone(), parameter_type.clone())),
              _ => None,
            },
            _ => None,
          })
          .collect::<Option<_>>()?,
      ),
      AstNode::Numeric(..) | AstNode::At(_) => {
        let (value, warning) = literal(ast)?.ok()?;
        if let Some(warning) = warning {
          let msg = self.builder.message(warning);
          self.builder.emit(Op::Warn { msg });
        }
        value
      }
      _ => return None,
    };
    Some(self.builder.konst(value))
  }
}

/// The value of a number or an `@` literal, with the warning its evaluation records, or the error
/// of a literal that has no value; `None` for another node.
fn literal(ast: &AstNode) -> Option<Result<(Value, Option<String>), String>> {
  match ast {
    AstNode::Numeric(before, after, sign, exponent) => Some(kernels::number_literal(before, after, *sign, exponent).map(|number| (Value::Number(number), None))),
    AstNode::At(text) => Some(kernels::at_literal(text)),
    _ => None,
  }
}
