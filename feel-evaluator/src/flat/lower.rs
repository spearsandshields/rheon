//! # Lowering
//!
//! The instructions of a resolved expression. Every node becomes the instructions that compute
//! its value, emitted in the order in which the parts of the node are evaluated, so that errors,
//! warnings and limits are recorded in that order. A value known when the expression is built and whose evaluation records nothing is a
//! constant; any other value is computed into a temporary slot, which its one reader moves out.
//! A node of a shape that the parser never produces makes the whole tree malformed ([Malformed]).

use super::Malformed;
use super::program::{
  CallInfo, ClosureInfo, Compare, Domain, FilterInfo, Fold, Kernel1, Kernel2, ListKind, LoopInfo, LoopKind, NONE, Op, Opnd, Pc, ProbeInfo, Program, ProgramBuilder, Proto,
  SearchInfo, Slot,
};
use super::resolve::{Info, Place, Resolved, Stop, Target, variables};
use crate::Dialect;
use crate::filters::ITEM;
use crate::kernels::{self, Arithmetic};
use rheon_feel::Name;
use rheon_feel::diagnostics::WarningCode;
use rheon_feel::message::arg;
use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::sync::Arc;

/// The nodes whose values a request asks for: the number of each, None for a node that is none
/// of them.
pub(crate) type Targets<'t> = dyn Fn(&AstNode) -> Option<u32> + 't;

/// The program of the resolved expression `resolved` in `dialect`: the code of the root
/// expression, then that of the body of each function definition in it.
pub(crate) fn lower<'a>(resolved: Resolved<'a>, dialect: Dialect, targets: Option<&Targets<'_>>) -> Result<Arc<Program>, Malformed<'a>> {
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
    targets,
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
      nearer: std::mem::take(&mut function.nearer),
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
  /// The nodes whose values a request asks for, each with its number: a recording instruction
  /// follows the value of such a node. None for a program that records nothing.
  targets: Option<&'r Targets<'r>>,
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

  /// The operand of the value of `node` ([Lower::lowered]), and for a node whose value a request
  /// asks for, the instruction that records it: the value is read there, not taken.
  fn value(&mut self, node: u32, dst: Option<Slot>) -> Result<Opnd, Malformed<'a>> {
    let opnd = self.lowered(node, dst)?;
    if let Some(targets) = self.targets
      && let Some(target) = targets(self.resolved.nodes[node as usize].ast)
    {
      let src = match opnd.decode() {
        super::program::Operand::Take(slot) => Opnd::slot(slot),
        _ => opnd,
      };
      self.builder.emit(Op::Record { src, target });
    }
    Ok(opnd)
  }

  /// The operand of the value of `node`: a constant, or a temporary that the instructions emitted
  /// here compute (`dst` when given).
  fn lowered(&mut self, node: u32, dst: Option<Slot>) -> Result<Opnd, Malformed<'a>> {
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
        _ => self.fail(dst, err_expected_name(entry)),
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
        let marked = self.diag_mark();
        self.builder.emit(Op::Compare { op, dst, a, b });
        self.boolean_default(dst, op == Compare::Ne, marked);
      }
      AstNode::Between(..) => {
        let args = self.temporary();
        self.temporary();
        self.temporary();
        let resolved = self.resolved;
        for (slot, &kid) in (args..).zip(resolved.kids(node)) {
          self.into(kid, slot)?;
        }
        let marked = self.diag_mark();
        self.builder.emit(Op::Between { dst, args });
        self.boolean_default(dst, false, marked);
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
        // and and or record no error of their own
        self.boolean_default(dst, false, false);
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
        let tests = matches!(kind, ListKind::ExpressionList | ListKind::NegatedList);
        for &item in items {
          let mut src = self.value(item, None)?;
          if tests && let Some(boolean) = asks(resolved.nodes[item as usize].ast) {
            // a boolean is the answer of the test, not a value to compare the tested value with
            let answer = self.temporary();
            self.builder.emit(Op::Kernel1 {
              kernel: Kernel1::TestAnswer(boolean),
              dst: answer,
              a: src,
            });
            src = Opnd::take(answer);
          }
          self.builder.emit(Op::ListPush { src });
          self.next = mark;
        }
        self.builder.emit(Op::ListEnd { kind, dst });
      }
      AstNode::AbstractType(..) => {
        // the mark takes off the error of a name that nothing holds
        self.builder.emit(Op::ReadMark);
        let (a, b) = self.operands(node)?;
        self.builder.emit(Op::TypeName { dst, a, b });
      }
      AstNode::In(..) | AstNode::Range(..) | AstNode::InstanceOf(..) | AstNode::As(..) | AstNode::ContextTypeEntry(..) | AstNode::FunctionType(..) => {
        let (a, b) = self.operands(node)?;
        let kernel = match ast {
          AstNode::In(..) => self.membership(),
          AstNode::Range(..) => Kernel2::Range,
          AstNode::InstanceOf(..) => Kernel2::InstanceOf,
          AstNode::As(..) => Kernel2::As,
          AstNode::ContextTypeEntry(..) => Kernel2::ContextTypeEntry,
          _ => Kernel2::FunctionType,
        };
        let membership = matches!(kernel, Kernel2::In | Kernel2::InTwoValued);
        let marked = membership && self.diag_mark();
        self.builder.emit(Op::Kernel2 { kernel, dst, a, b });
        if membership {
          self.boolean_default(dst, false, marked);
        }
      }
      AstNode::Out(..) => {
        // the whole `in` (never with the B-FEEL default), then the left operand again
        let (a, b) = self.operands(node)?;
        let test = self.temporary();
        self.builder.emit(Op::Kernel2 {
          kernel: self.membership(),
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
          _ => return Err(Malformed { at: ast }),
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
      _ => return Err(Malformed { at: ast }),
    }
    self.next = mark;
    Ok(Opnd::take(dst))
  }

  /// The values of the children of `node` added in order to a fold of `kind`, which stops at the
  /// first value that is not of its kind.
  fn fold(&mut self, node: u32, kind: Fold, dst: Slot) -> Result<(), Malformed<'a>> {
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
  fn function(&mut self, node: u32, dst: Slot) -> Result<(), Malformed<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let (Info::Function { proto: function }, AstNode::FunctionDefinition(_, body)) = (&rnode.info, rnode.ast) else {
      return Err(Malformed { at: rnode.ast });
    };
    if *function == NONE {
      // a definition whose parts are not of their shapes
      return Err(Malformed { at: rnode.ast });
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
  fn body(&mut self, proto: usize, function: u32) -> Result<(), Malformed<'a>> {
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
  fn invocation(&mut self, node: u32, arguments: &AstNode, dst: Slot) -> Result<(), Malformed<'a>> {
    let enter = self.builder.emit(Op::Enter { dst, skip: 0 });
    let names: Result<Option<Box<[Name]>>, String> = match arguments {
      AstNode::PositionalParameters(_) => Ok(None),
      AstNode::NamedParameters(items) => items
        .iter()
        .map(|item| match item {
          AstNode::NamedParameter(name, _) => match name.as_ref() {
            AstNode::ParameterName(name) => Ok(name.clone()),
            other => Err(err_expected_parameter_name(other)),
          },
          other => Err(err_msg_expected_node("NamedParameter", other)),
        })
        .collect::<Result<_, _>>()
        .map(Some),
      other => Err(err_msg_expected_positional_or_named_parameter(other)),
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
  fn context(&mut self, node: u32, dst: Slot) -> Result<(), Malformed<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let Info::Context { layout, stop } = rnode.info else {
      return Err(Malformed { at: rnode.ast });
    };
    let entries = resolved.kids(node);
    let bound = &resolved.layouts[layout as usize].entries;
    // the entries as the functions written in the literal see them
    let home = resolved.homes.contains(&layout);
    if home {
      self.builder.emit(Op::HomeOpen { layout });
    }
    for (index, (&entry, &(_, place))) in entries.iter().zip(bound.iter()).enumerate() {
      let Place::Slot(slot) = place else {
        return Err(Malformed { at: rnode.ast });
      };
      if resolved.circular.contains(&entry) {
        // an entry that needs its own value through other entries has none
        let name = &resolved.names[resolved.layouts[layout as usize].entries[index].0 as usize];
        self.fail(slot, kernels::err_circular_context_entry(name));
      } else {
        self.into(resolved.kids(entry)[1], slot)?;
      }
      self.builder.emit(Op::BoundDepth { s: slot });
      if home {
        self.builder.emit(Op::HomeSet { entry: index as u32, s: slot });
      }
    }
    if home {
      self.builder.emit(Op::HomeClose);
    }
    match stop {
      Stop::None => {
        self.builder.emit(Op::Context { dst, layout });
      }
      Stop::Duplicate(index) => {
        let entry = entries[index as usize];
        self.into(resolved.kids(entry)[1], dst)?;
        let AstNode::ContextEntry(key, _) = resolved.nodes[entry as usize].ast else {
          return Err(Malformed { at: rnode.ast });
        };
        let AstNode::ContextEntryKey(name) = key.as_ref() else {
          return Err(Malformed { at: rnode.ast });
        };
        self.fail(dst, kernels::err_duplicated_context_entry_key(name));
        self.builder.emit(Op::Clear { layout, visible: index });
      }
      // an entry whose value may bind any name
      Stop::Irregular(_) => return Err(Malformed { at: rnode.ast }),
    }
    Ok(())
  }

  /// A filter: the list, then the predicate, evaluated in the scope around the filter and then,
  /// unless it selected an item by its index, for each item.
  fn filter(&mut self, node: u32, dst: Slot) -> Result<(), Malformed<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let Info::Filter { record } = rnode.info else {
      return Err(Malformed { at: rnode.ast });
    };
    let kids = resolved.kids(node);
    let mark = self.next;
    let list = self.value(kids[0], None)?;
    let test = plain_test(resolved.nodes[kids[1] as usize].ast);
    let info = self.builder.filter(FilterInfo {
      dst,
      body: 0,
      close: 0,
      after: 0,
      test,
    });
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
  fn iteration(&mut self, node: u32, dst: Slot) -> Result<(), Malformed<'a>> {
    let resolved = self.resolved;
    let rnode = &resolved.nodes[node as usize];
    let (kind, contexts) = match rnode.ast {
      AstNode::For(contexts, _) => (LoopKind::For, contexts),
      AstNode::Some(contexts, _) => (LoopKind::Some, contexts),
      AstNode::Every(contexts, _) => (LoopKind::Every, contexts),
      _ => return Err(Malformed { at: rnode.ast }),
    };
    let Info::Iteration { layout, record, duplicate } = rnode.info else {
      self.malformed_quantified(rnode.ast, dst);
      return Ok(());
    };
    if duplicate != NONE {
      self.fail(dst, err_duplicate_iteration_variable(&resolved.names[duplicate as usize]));
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
        return Err(Malformed { at: rnode.ast });
      };
      levels.push((slot, 0));
    }
    let mark = self.next;
    let kids = resolved.kids(node);
    let mut parts = kids.iter().copied();
    let (domain, a, b) = self.domain(variables[0].1.len(), &mut parts)?;
    let info = self.builder.iteration(LoopInfo {
      kind,
      defaulted: self.dialect == Dialect::BFeel && kind != LoopKind::For,
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
      let (domain, a, b) = self.domain(domain_parts.len(), &mut parts)?;
      self.builder.emit(Op::LevelPush { record, domain, a, b });
      self.next = mark;
    }
    let last = levels.len() - 1;
    levels[last].1 = self.builder.here();
    let body = parts.next().ok_or(Malformed { at: rnode.ast })?;
    let src = self.value(body, None)?;
    self.builder.emit(Op::Visit { record, src });
    self.next = mark;
    let exit = self.builder.emit(Op::LoopClose { record, dst });
    // some and every are boolean expressions: B-FEEL answers false where FEEL answers null (11.2)
    if kind != LoopKind::For {
      self.boolean_default(dst, false, false);
    }
    let info = self.builder.iteration_mut(info);
    info.step = step;
    info.exit = exit;
    info.levels = levels.into();
    Ok(())
  }

  /// The domain of a variable whose domain has `count` parts, the next of `parts`: an interval
  /// from its two parts, or the items of a value (a range is none, [crate::Sequence::of_context]).
  fn domain(&mut self, count: usize, parts: &mut impl Iterator<Item = u32>) -> Result<(Domain, Opnd, Opnd), Malformed<'a>> {
    let resolved = self.resolved;
    let first = parts.next().ok_or(Malformed {
      at: resolved.nodes[resolved.root() as usize].ast,
    })?;
    if count == 2 {
      let second = parts.next().ok_or(Malformed {
        at: resolved.nodes[first as usize].ast,
      })?;
      let a = self.value(first, None)?;
      let b = self.value(second, None)?;
      return Ok((Domain::Interval, a, b));
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
      err_msg_expected_node("QuantifiedContexts", contexts)
    } else {
      let mut names = std::collections::HashSet::new();
      match variables(contexts, true).into_iter().find(|(name, _)| !names.insert(*name)) {
        Some((name, _)) => err_duplicate_iteration_variable(name),
        None => err_msg_expected_node("Satisfies", satisfies),
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
  fn into(&mut self, node: u32, dst: Slot) -> Result<(), Malformed<'a>> {
    let src = self.value(node, Some(dst))?;
    if src != Opnd::take(dst) {
      self.builder.emit(Op::Move { dst, src });
    }
    Ok(())
  }

  /// The operands of the two children of `node`, evaluated in order.
  fn operands(&mut self, node: u32) -> Result<(Opnd, Opnd), Malformed<'a>> {
    let kids = self.resolved.kids(node);
    let (lhs, rhs) = (kids[0], kids[1]);
    Ok((self.value(lhs, None)?, self.value(rhs, None)?))
  }

  /// The kernel of `in`: in B-FEEL a comparison is true or false, so its tests are too.
  fn membership(&self) -> Kernel2 {
    if self.dialect == Dialect::BFeel { Kernel2::InTwoValued } else { Kernel2::In }
  }

  /// B-FEEL: a boolean operator never gives null, but `ne` (DMN 1.6 chapter 11). With `marked`,
  /// the operation before was marked ([Lower::diag_mark]): its errors become warnings, as B-FEEL
  /// defines its answer.
  fn boolean_default(&mut self, dst: Slot, ne: bool, marked: bool) {
    if self.dialect == Dialect::BFeel {
      self.builder.emit(Op::BoolDefault { dst, ne, marked });
    }
  }

  /// B-FEEL: marks the diagnostics before an operation that records the error behind the null a
  /// [Lower::boolean_default] replaces (11.1: that error is no error in B-FEEL).
  fn diag_mark(&mut self) -> bool {
    let marked = self.dialect == Dialect::BFeel;
    if marked {
      self.builder.emit(Op::DiagMark);
    }
    marked
  }

  /// `dst` is null with the error `text`, recorded at every evaluation.
  fn fail(&mut self, dst: Slot, text: String) {
    let msg = self.builder.message(text);
    self.builder.emit(Op::Fail { dst, msg });
  }

  /// A name read into `dst`: in the root expression, from the scope of the host.
  fn read(&mut self, node: u32, dst: Slot) -> Result<(), Malformed<'a>> {
    let rnode = &self.resolved.nodes[node as usize];
    let Info::Read(read) = &rnode.info else {
      // a qualified name with a part that is no segment
      return Err(Malformed { at: rnode.ast });
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
      _ => return Err(Malformed { at: rnode.ast }),
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
        if let Some((code, warning)) = warning {
          let msg = self.builder.message(warning);
          self.builder.emit(Op::Warn { msg, code: code as u8 });
        }
        value
      }
      _ => fixed_test(ast)?,
    };
    Some(self.builder.konst(value))
  }
}

/// The value of a unary comparison, a range or a list of tests whose parts are all literals that
/// record nothing: what its evaluation builds, built once, so a test against it builds nothing.
fn fixed_test(ast: &AstNode) -> Option<Value> {
  match ast {
    AstNode::ExpressionList(items) => items.iter().map(fixed_item).collect::<Option<_>>().map(Value::ExpressionList),
    AstNode::NegatedList(items) => items.iter().map(fixed_item).collect::<Option<_>>().map(Value::NegatedCommaList),
    _ => fixed_single(ast),
  }
}

/// [fixed_test] for an item of a list of tests: a literal, a unary comparison or a range.
fn fixed_item(ast: &AstNode) -> Option<Value> {
  fixed_scalar(ast).or_else(|| fixed_single(ast))
}

/// [fixed_test] for a unary comparison or a range.
fn fixed_single(ast: &AstNode) -> Option<Value> {
  let boxed = |ast: &AstNode| fixed_scalar(ast).map(Box::new);
  Some(match ast {
    AstNode::UnaryEq(value) => Value::UnaryEqual(boxed(value)?),
    AstNode::UnaryNe(value) => Value::UnaryNotEqual(boxed(value)?),
    AstNode::UnaryLt(value) => Value::UnaryLess(boxed(value)?),
    AstNode::UnaryLe(value) => Value::UnaryLessOrEqual(boxed(value)?),
    AstNode::UnaryGt(value) => Value::UnaryGreater(boxed(value)?),
    AstNode::UnaryGe(value) => Value::UnaryGreaterOrEqual(boxed(value)?),
    AstNode::Range(start, end) => match (start.as_ref(), end.as_ref()) {
      // its bounds are not nested, so the range is within the depth that values may have
      (AstNode::IntervalStart(start, start_type), AstNode::IntervalEnd(end, end_type)) => Value::Range(boxed(start)?, *start_type, boxed(end)?, *end_type),
      _ => return None,
    },
    _ => return None,
  })
}

/// True for a predicate of a filter that is a comparison, a conjunction or a disjunction of
/// names, paths, literals and arithmetic: its value is a boolean or null, never the index of an
/// item, and its evaluation visits no item of a list, calls nothing and so takes no step.
fn plain_test(ast: &AstNode) -> bool {
  /// What a plain test may hold below it.
  fn plain(ast: &AstNode) -> bool {
    match ast {
      AstNode::Name(_) | AstNode::QualifiedName(_) | AstNode::Numeric(..) | AstNode::String(_) | AstNode::Boolean(_) | AstNode::Null | AstNode::At(_) => true,
      AstNode::Neg(inner) => plain(inner),
      AstNode::Path(left, right) => plain(left) && matches!(right.as_ref(), AstNode::Name(_)),
      AstNode::Add(left, right) | AstNode::Sub(left, right) | AstNode::Mul(left, right) | AstNode::Div(left, right) => plain(left) && plain(right),
      test => plain_test(test),
    }
  }
  match ast {
    AstNode::Eq(left, right)
    | AstNode::Nq(left, right)
    | AstNode::Lt(left, right)
    | AstNode::Le(left, right)
    | AstNode::Gt(left, right)
    | AstNode::Ge(left, right)
    | AstNode::And(left, right)
    | AstNode::Or(left, right) => plain(left) && plain(right),
    AstNode::Between(value, low, high) => plain(value) && plain(low) && plain(high),
    _ => false,
  }
}

/// The value of a literal that is not nested and whose evaluation records nothing.
fn fixed_scalar(ast: &AstNode) -> Option<Value> {
  match ast {
    AstNode::Boolean(value) => Some(Value::Boolean(*value)),
    AstNode::Null => Some(Value::Null(None)),
    AstNode::String(text) => Some(Value::String(text.as_str().into())),
    AstNode::Numeric(..) | AstNode::At(_) => match literal(ast)?.ok()? {
      (value, None) => Some(value),
      _ => None,
    },
    // a negative number: the negation of a number literal is a number and records nothing
    AstNode::Neg(number) if matches!(number.as_ref(), AstNode::Numeric(..)) => match Arithmetic::negation(fixed_scalar(number)?) {
      number @ Value::Number(_) => Some(number),
      _ => None,
    },
    _ => None,
  }
}

/// The value of a literal, and the warning its evaluation records: its code and its message.
type Literal = (Value, Option<(WarningCode, String)>);

/// The value of a number or an `@` literal, with the warning its evaluation records, or the error
/// of a literal that has no value; `None` for another node.
fn literal(ast: &AstNode) -> Option<Result<Literal, String>> {
  match ast {
    AstNode::Numeric(before, after, sign, exponent) => {
      Some(kernels::number_literal(before, after, *sign, exponent).map(|(number, warning)| (Value::Number(number), warning.map(|warning| (WarningCode::NumberRounded, warning)))))
    }
    AstNode::At(text) => Some(kernels::at_literal(text)),
    _ => None,
  }
}

/// Returns an error message when other AST node was expected.
fn err_msg_expected_node(expected: &str, actual: &AstNode) -> String {
  format!("expected AST node {expected}, actual AST node is {actual:?}")
}

/// Returns an error message when expected only a named or positional parameter.
fn err_msg_expected_positional_or_named_parameter(actual: &AstNode) -> String {
  format!("expected positional or named parameter, actual AST node is {actual:?}")
}

/// Returns an error message when expected only a parameter name.
fn err_expected_parameter_name(actual: &AstNode) -> String {
  format!("expected AST node ParameterName, actual node is {actual:?}")
}

/// The error of an iteration whose variable `name` is named twice.
fn err_duplicate_iteration_variable(name: &Name) -> String {
  format!("duplicate iteration variable '{}'", arg(name))
}

/// Returns an error message when expected only a name.
fn err_expected_name(actual: &AstNode) -> String {
  format!("expected name node, actual node is: {actual:?}")
}

/// Whether a unary test uses the special variable `?`, and then whether it is a boolean
/// expression by its form: a comparison, `and`, `or`, `between`, `in`, `instance of` or an
/// invocation. A test that uses `?` and gives a boolean is satisfied when it is true (DMN 1.6
/// clause 7.3.2 d and the last row of Table 56); another test is satisfied when the tested
/// value is equal to its value.
fn asks(test: &AstNode) -> Option<bool> {
  // `?` alone is the tested value: it is compared with itself
  if matches!(test, AstNode::Name(_)) {
    return None;
  }
  let mut nodes = vec![test];
  let mut uses = false;
  while let Some(node) = nodes.pop() {
    if matches!(node, AstNode::Name(name) if name.to_string() == "?") {
      uses = true;
      break;
    }
    node.for_each_child(|child| nodes.push(child));
  }
  uses.then_some(matches!(
    test,
    AstNode::Eq(..)
      | AstNode::Nq(..)
      | AstNode::Lt(..)
      | AstNode::Le(..)
      | AstNode::Gt(..)
      | AstNode::Ge(..)
      | AstNode::And(..)
      | AstNode::Or(..)
      | AstNode::Between(..)
      | AstNode::In(..)
      | AstNode::InstanceOf(..)
      | AstNode::As(..)
      | AstNode::FunctionInvocation(..)
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  use rheon_feel::{FeelScope, IntervalType};

  /// [fixed_test] of the right operand of `in` in `text`.
  fn fixed(text: &str) -> Option<Value> {
    let scope = FeelScope::default();
    scope.set_value(&"x".into(), Value::Null(None));
    match &rheon_feel_parser::parse_expression(&scope, text).unwrap() {
      AstNode::In(_, tests) => fixed_test(tests),
      other => panic!("no `in`: {other:?}"),
    }
  }

  #[test]
  fn tests_of_literals_are_built_once() {
    match fixed(r#"x in (< 10, [20..30), "a", true, null, >= @"2026-01-01")"#) {
      Some(Value::ExpressionList(tests)) => {
        assert_eq!(6, tests.len());
        assert!(matches!(tests[0], Value::UnaryLess(_)));
        assert!(matches!(tests[1], Value::Range(_, IntervalType::Closed, _, IntervalType::Opened)));
        assert!(matches!(tests[5], Value::UnaryGreaterOrEqual(_)));
      }
      other => panic!("no list of tests: {other:?}"),
    }
    assert!(matches!(fixed("x in [1..10]"), Some(Value::Range(..))));
    assert!(matches!(fixed("x in > 5"), Some(Value::UnaryGreater(_))));
    assert_eq!(Some("[-10..-5]".to_string()), fixed("x in [-10..-5]").map(|range| range.to_string()));
  }

  #[test]
  fn tests_with_another_part_are_built_when_evaluated() {
    // a name, an operation, and a literal whose evaluation records a warning
    for text in ["x in (< 10, x)", "x in [1..x]", "x in [1..2+3]", "x in < -x", r#"x in (< @"PT0.0000000001S")"#] {
      assert!(fixed(text).is_none(), "{text}");
    }
  }

  /// [plain_test] of the predicate of the filter `text`.
  fn test_only(text: &str) -> bool {
    let scope = FeelScope::default();
    for name in ["x", "rows", "f"] {
      scope.set_value(&name.into(), Value::Null(None));
    }
    match &rheon_feel_parser::parse_expression(&scope, text).unwrap() {
      AstNode::Filter(_, predicate) => plain_test(predicate),
      other => panic!("no filter: {other:?}"),
    }
  }

  #[test]
  fn a_predicate_that_only_tests_is_not_evaluated_around_the_filter() {
    for text in [
      "rows[item > 1]",
      "rows[x = \"a\" and item.x <= 2 * x + 1]",
      "rows[item != null or x < -1]",
      "rows[x between 1 and item.x]",
      "rows[item >= @\"2026-01-01\"]",
    ] {
      assert!(test_only(text), "{text}");
    }
  }

  #[test]
  fn a_predicate_that_may_be_an_index_or_take_a_step_is_evaluated_around_the_filter() {
    // an index, a name that may hold one, a call, a filter and an iteration inside, a power
    for text in [
      "rows[1]",
      "rows[x]",
      "rows[-1]",
      "rows[x + 1]",
      "rows[f(item) > 1]",
      "rows[rows[1] = item]",
      "rows[item > count(rows)]",
      "rows[(some y in rows satisfies y = item) = true]",
      "rows[item ** 2 > 4]",
      "rows[if item > 1 then true else false]",
      "rows[item in (1, 2)]",
    ] {
      assert!(!test_only(text), "{text}");
    }
  }
}
