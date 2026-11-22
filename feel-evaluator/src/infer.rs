//! The type of an expression, found from its syntax tree without evaluating it: for editors,
//! which complete the entries of a value after a dot and show the types of what they offer.
//!
//! The answer is the type the value has when the expression gives one: an expression can
//! always give null instead (DMN 1.6 clause 10.3.2.1). Where the tree does not say the type
//! (the result of a function of the host, a name without a value, a list of items of several
//! types) the answer is `Any`. Nothing here is used by the evaluation.

use crate::flat::variables;
use rheon_feel::signatures::built_in;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, FeelType, Name};
use rheon_feel_parser::AstNode;
use std::collections::BTreeMap;

/// The deepest tree that is typed: below it the type is `Any`. The trees of the parser are not
/// deeper than its own limit; this bounds the stack of the walk for any tree.
const MAX_DEPTH: usize = 300;

/// The deepest value of the scope whose type is read: below it the type is `Any`.
const MAX_VALUE_DEPTH: usize = 16;

/// The most nodes that one inference types: the body of a function literal is typed again at
/// each invocation of it with the types of the arguments, and a text of many invocations of
/// functions that invoke each other would type their bodies without end. Beyond it a function
/// literal has the type its body has with the types of its parameters as written.
const MAX_STEPS: usize = 20_000;

/// Returns the type of the value of `node` in `scope`, where the names of `bound` (the
/// variables and parameters of the constructs around the expression, the innermost last) have
/// the types given with them.
pub fn infer_type(node: &AstNode, scope: &FeelScope, bound: &[(Name, FeelType)]) -> FeelType {
  infer_type_with(node, scope, bound, &[])
}

/// [infer_type] where names of `bound` are bound to function literals of the text: `literals`
/// gives the index of such a name in `bound` and the literal (an editor that types a piece of a
/// context literal gives the functions of the entries before it). An invocation of the name
/// types the body of the literal with the types of the arguments.
pub fn infer_type_with<'n>(node: &'n AstNode, scope: &FeelScope, bound: &[(Name, FeelType)], literals: &'n [(usize, AstNode)]) -> FeelType {
  let mut definitions: Vec<(usize, &AstNode)> = literals
    .iter()
    .filter(|(at, literal)| *at < bound.len() && matches!(literal, AstNode::FunctionDefinition(..)))
    .map(|(at, literal)| (*at, literal))
    .collect();
  definitions.sort_by_key(|(at, _)| *at);
  let mut typer = Typer {
    scope,
    bound: bound.to_vec(),
    definitions,
    active: vec![],
    depth: 0,
    steps: 0,
  };
  typer.node(node)
}

/// Returns the type of a value of the scope as an editor shows it: of a context the types of
/// its entries, of a list the type of its first item (the items of a list of a host are of one
/// kind; reading them all would cost what the list is long).
pub fn type_of_value(value: &Value) -> FeelType {
  value_type(value, 0)
}

fn value_type(value: &Value, depth: usize) -> FeelType {
  if depth >= MAX_VALUE_DEPTH {
    return FeelType::Any;
  }
  match value {
    Value::Context(context) => FeelType::Context(context.iter().map(|(name, value)| (name.clone(), value_type(value, depth + 1))).collect()),
    Value::List(items) => FeelType::List(Box::new(items.first().map_or(FeelType::Any, |item| value_type(item, depth + 1)))),
    Value::Null(_) => FeelType::Any,
    // a type in the place of a value says the type of the name (an outline, a DMN declaration)
    Value::FeelType(feel_type) => feel_type.clone(),
    other => other.type_of(),
  }
}

/// Returns the type that `text` writes as FEEL writes a type (`number`, `list<string>`,
/// `range<date>`, `context<a: number>`, `function<number>->string`), or None when it is no
/// type of FEEL.
pub fn type_of_text(text: &str) -> Option<FeelType> {
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_expression(&scope, &format!("null instance of {text}")).ok()?;
  match &node {
    AstNode::InstanceOf(value, written) if matches!(value.as_ref(), AstNode::Null) => type_written(written),
    _ => None,
  }
}

/// The type that a type node of the parser writes, None for a name that is no type of FEEL.
fn type_written(node: &AstNode) -> Option<FeelType> {
  Some(match node {
    AstNode::FeelType(feel_type) => feel_type.clone(),
    AstNode::ListType(item) => FeelType::List(Box::new(type_written(item)?)),
    AstNode::RangeType(endpoint) => FeelType::Range(Box::new(type_written(endpoint)?)),
    AstNode::ContextType(entries) => {
      let mut types = BTreeMap::new();
      for entry in entries {
        let AstNode::ContextTypeEntry(key, entry_type) = entry else {
          return None;
        };
        let AstNode::ContextTypeEntryKey(name) = key.as_ref() else {
          return None;
        };
        types.insert(name.clone(), type_written(entry_type)?);
      }
      FeelType::Context(types)
    }
    AstNode::FunctionType(parameters, result) => {
      let AstNode::ParameterTypes(items) = parameters.as_ref() else {
        return None;
      };
      FeelType::Function(items.iter().map(type_written).collect::<Option<Vec<_>>>()?, Box::new(type_written(result)?))
    }
    _ => return None,
  })
}

/// Returns the members that a value of the type `feel_type` has after a dot, with their types:
/// the entries of a context, the properties of a date, a time, a duration and a range, and for
/// a list the members of its items, each as the list of its values.
pub fn members_of(feel_type: &FeelType) -> Vec<(String, FeelType)> {
  let properties = |names: &[&str]| names.iter().map(|name| (name.to_string(), FeelType::Number)).collect::<Vec<_>>();
  match feel_type {
    FeelType::Context(entries) => entries.iter().map(|(name, entry_type)| (name.to_string(), entry_type.clone())).collect(),
    FeelType::List(item_type) => members_of(item_type)
      .into_iter()
      .map(|(name, member_type)| (name, FeelType::List(Box::new(member_type))))
      .collect(),
    FeelType::Date => properties(&["year", "month", "day", "weekday", "value"]),
    FeelType::DateTime => {
      let mut members = properties(&["year", "month", "day", "weekday", "hour", "minute", "second", "value"]);
      members.push(("time offset".to_string(), FeelType::DaysAndTimeDuration));
      members.push(("timezone".to_string(), FeelType::String));
      members
    }
    FeelType::Time => {
      let mut members = properties(&["hour", "minute", "second", "value"]);
      members.push(("time offset".to_string(), FeelType::DaysAndTimeDuration));
      members.push(("timezone".to_string(), FeelType::String));
      members
    }
    FeelType::YearsAndMonthsDuration => properties(&["years", "months", "value"]),
    FeelType::DaysAndTimeDuration => properties(&["days", "hours", "minutes", "seconds", "value"]),
    FeelType::Range(endpoint) => vec![
      ("start".to_string(), endpoint.as_ref().clone()),
      ("end".to_string(), endpoint.as_ref().clone()),
      ("start included".to_string(), FeelType::Boolean),
      ("end included".to_string(), FeelType::Boolean),
    ],
    _ => vec![],
  }
}

/// The type that a type written as text in the table of signatures names, `Any` for a text
/// that names several types.
fn signature_type(text: &str) -> FeelType {
  match text {
    "boolean" => FeelType::Boolean,
    "number" => FeelType::Number,
    "string" => FeelType::String,
    "date" => FeelType::Date,
    "time" => FeelType::Time,
    "date and time" => FeelType::DateTime,
    "days and time duration" => FeelType::DaysAndTimeDuration,
    "years and months duration" => FeelType::YearsAndMonthsDuration,
    "list" => FeelType::List(Box::new(FeelType::Any)),
    "list<number>" => FeelType::List(Box::new(FeelType::Number)),
    "list<string>" => FeelType::List(Box::new(FeelType::String)),
    "list<context>" => FeelType::List(Box::new(FeelType::Context(BTreeMap::new()))),
    "context" => FeelType::Context(BTreeMap::new()),
    "range" => FeelType::Range(Box::new(FeelType::Any)),
    _ => FeelType::Any,
  }
}

/// The type both types are: the type itself when they are the same, `Any` otherwise. A type
/// and `Null` are the type (a value or null).
fn join(a: FeelType, b: FeelType) -> FeelType {
  match (a, b) {
    (FeelType::Null, other) | (other, FeelType::Null) => other,
    (a, b) if a == b => a,
    (FeelType::List(a), FeelType::List(b)) => FeelType::List(Box::new(join(*a, *b))),
    _ => FeelType::Any,
  }
}

struct Typer<'a, 'n> {
  scope: &'a FeelScope,
  /// The names bound around the node being typed, the innermost last, with their types.
  bound: Vec<(Name, FeelType)>,
  /// The function literals that names of `bound` are bound to: the index of the name in
  /// `bound`, and the literal. An invocation of such a name types the body of the literal with
  /// the types of the arguments.
  definitions: Vec<(usize, &'n AstNode)>,
  /// The function literals whose bodies are being typed for an invocation (a literal that
  /// invokes itself is not typed again inside itself).
  active: Vec<*const AstNode>,
  depth: usize,
  /// The nodes typed so far ([MAX_STEPS]).
  steps: usize,
}

impl<'n> Typer<'_, 'n> {
  /// Forgets the names bound from the index `mark` of `bound` on.
  fn unbind(&mut self, mark: usize) {
    self.bound.truncate(mark);
    while self.definitions.last().is_some_and(|(index, _)| *index >= mark) {
      self.definitions.pop();
    }
  }

  fn node(&mut self, node: &'n AstNode) -> FeelType {
    if self.depth >= MAX_DEPTH {
      return FeelType::Any;
    }
    self.steps += 1;
    self.depth += 1;
    let feel_type = self.typed(node);
    self.depth -= 1;
    feel_type
  }

  fn typed(&mut self, node: &'n AstNode) -> FeelType {
    use AstNode as A;
    match node {
      A::Numeric(..) => FeelType::Number,
      A::String(_) => FeelType::String,
      A::Boolean(_) => FeelType::Boolean,
      A::Null => FeelType::Null,
      A::At(text) => at_literal(text),
      A::Name(name) => self.name(name),
      A::QualifiedName(segments) => {
        let mut names = segments.iter().filter_map(|segment| match segment {
          A::QualifiedNameSegment(name) => Some(name),
          _ => None,
        });
        let Some(first) = names.next() else {
          return FeelType::Any;
        };
        let mut feel_type = self.name(first);
        for name in names {
          feel_type = member(&feel_type, name.as_str());
        }
        feel_type
      }
      A::Path(base, name) => match name.as_ref() {
        A::Name(name) => {
          let base = self.node(base);
          member(&base, name.as_str())
        }
        _ => FeelType::Any,
      },
      A::Descendant(..) => FeelType::List(Box::new(FeelType::Any)),
      A::Filter(list, predicate) => {
        let list = self.node(list);
        // the entries of an item are names in the predicate; they are not typed here
        let predicate = self.node(predicate);
        let (item, is_list) = match list {
          FeelType::List(item) => (*item, true),
          other => (other, false),
        };
        match predicate {
          // an index selects one item
          FeelType::Number => item,
          _ if is_list || item != FeelType::Any => FeelType::List(Box::new(item)),
          _ => FeelType::Any,
        }
      }
      A::List(items) => {
        let mut item_type: Option<FeelType> = None;
        for item in items {
          let next = self.node(item);
          item_type = Some(match item_type {
            Some(current) => join(current, next),
            None => next,
          });
        }
        FeelType::List(Box::new(match item_type {
          Some(FeelType::Null) | None => FeelType::Any,
          Some(item_type) => item_type,
        }))
      }
      A::Context(entries) => self.context(entries),
      A::Range(start, end) => {
        let endpoint = |typer: &mut Self, node: &'n AstNode| match node {
          A::IntervalStart(value, _) | A::IntervalEnd(value, _) => typer.node(value),
          other => typer.node(other),
        };
        let start = endpoint(self, start);
        let end = endpoint(self, end);
        FeelType::Range(Box::new(join(start, end)))
      }
      // `e as T` is of the type T, or null (DMN 1.7 Beta 1, Table 62)
      A::As(_, written) => type_written(written).unwrap_or(FeelType::Any),
      A::UnaryLt(end) | A::UnaryLe(end) | A::UnaryGt(end) | A::UnaryGe(end) | A::UnaryEq(end) | A::UnaryNe(end) => FeelType::Range(Box::new(self.node(end))),
      A::Eq(..)
      | A::Nq(..)
      | A::Lt(..)
      | A::Le(..)
      | A::Gt(..)
      | A::Ge(..)
      | A::And(..)
      | A::Or(..)
      | A::In(..)
      | A::Out(..)
      | A::Between(..)
      | A::InstanceOf(..)
      | A::Some(..)
      | A::Every(..) => FeelType::Boolean,
      A::ExpressionList(_) | A::NegatedList(_) => FeelType::Boolean,
      A::Neg(operand) => match self.node(operand) {
        feel_type @ (FeelType::Number | FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration) => feel_type,
        _ => FeelType::Any,
      },
      A::Add(a, b) => {
        let (a, b) = (self.node(a), self.node(b));
        addition(&a, &b)
      }
      A::Sub(a, b) => {
        let (a, b) = (self.node(a), self.node(b));
        subtraction(&a, &b)
      }
      A::Mul(a, b) => {
        let (a, b) = (self.node(a), self.node(b));
        match (&a, &b) {
          (FeelType::Number, FeelType::Number) => FeelType::Number,
          // a duration is multiplied only by a number, a number by a number or a duration (Table 60)
          (duration @ (FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration), FeelType::Number | FeelType::Any)
          | (FeelType::Number | FeelType::Any, duration @ (FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration)) => duration.clone(),
          _ => FeelType::Any,
        }
      }
      A::Div(a, b) => {
        let (a, b) = (self.node(a), self.node(b));
        match (&a, &b) {
          // a number is divided only by a number (Table 60)
          (FeelType::Number, FeelType::Number | FeelType::Any) => FeelType::Number,
          (duration @ (FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration), FeelType::Number) => duration.clone(),
          (FeelType::DaysAndTimeDuration, FeelType::DaysAndTimeDuration) | (FeelType::YearsAndMonthsDuration, FeelType::YearsAndMonthsDuration) => FeelType::Number,
          _ => FeelType::Any,
        }
      }
      // only numbers have a power (Table 61): a power that is not null is a number
      A::Exp(a, b) => {
        let (a, b) = (self.node(a), self.node(b));
        let number = |feel_type: &FeelType| matches!(feel_type, FeelType::Number | FeelType::Any);
        if number(&a) && number(&b) { FeelType::Number } else { FeelType::Any }
      }
      A::If(_, then, otherwise) => {
        let then = self.node(then);
        let otherwise = self.node(otherwise);
        join(then, otherwise)
      }
      A::For(contexts, body) => {
        let mark = self.bound.len();
        self.bind(variables(contexts, false));
        let body = self.node(body);
        self.unbind(mark);
        FeelType::List(Box::new(if body == FeelType::Null { FeelType::Any } else { body }))
      }
      A::EvaluatedExpression(inner) | A::Satisfies(inner) => self.node(inner),
      A::FunctionDefinition(parameters, body) => {
        let mark = self.bound.len();
        let mut parameter_types = vec![];
        if let A::FormalParameters(items) = parameters.as_ref() {
          for item in items {
            if let A::FormalParameter(name, parameter_type) = item
              && let A::ParameterName(name) = name.as_ref()
            {
              let parameter_type = written_type(parameter_type);
              parameter_types.push(parameter_type.clone());
              self.bound.push((name.clone(), parameter_type));
            }
          }
        }
        let result = match body.as_ref() {
          A::FunctionBody(expression, false) => self.node(expression),
          _ => FeelType::Any,
        };
        self.unbind(mark);
        FeelType::Function(parameter_types, Box::new(result))
      }
      A::FunctionInvocation(callee, arguments) => self.invocation(callee, arguments),
      _ => FeelType::Any,
    }
  }

  /// The type of what `name` reads: a variable or parameter around the expression, a value of
  /// the scope, or a built-in function.
  fn name(&self, name: &Name) -> FeelType {
    if let Some((_, feel_type)) = self.bound.iter().rev().find(|(bound, _)| bound == name) {
      return feel_type.clone();
    }
    if let Some(value) = self.scope.frames().iter().rev().find_map(|frame| frame.get_entry(name).map(type_of_value)) {
      return value;
    }
    FeelType::Any
  }

  /// Binds the variables of an iteration to the types of the items of their domains.
  fn bind(&mut self, variables: Vec<(&'n Name, Vec<&'n AstNode>)>) {
    for (name, domain) in variables {
      let variable_type = match domain.as_slice() {
        // the numbers of an interval
        [_, _] => FeelType::Number,
        [values] => match self.node(values) {
          FeelType::List(item) => *item,
          FeelType::Range(endpoint) => *endpoint,
          FeelType::Any | FeelType::Null => FeelType::Any,
          // a value that is no list is a list of itself
          other => other,
        },
        _ => FeelType::Any,
      };
      self.bound.push((name.clone(), variable_type));
    }
  }

  /// A context literal: each entry with the type of its value; an entry reads the entries
  /// written before it with their types (an entry written after it reads as `Any`).
  fn context(&mut self, entries: &'n [AstNode]) -> FeelType {
    let mark = self.bound.len();
    let mut types = BTreeMap::new();
    for (position, entry) in entries.iter().enumerate() {
      if let AstNode::ContextEntry(key, value) = entry
        && let AstNode::ContextEntryKey(name) = key.as_ref()
      {
        // the body of a function literal runs when it is invoked, and sees the entries written
        // after it too: their names hide the names outside, of no type known here
        let shadows = self.bound.len();
        if let AstNode::FunctionDefinition(..) = value.as_ref() {
          for later in &entries[position + 1..] {
            if let AstNode::ContextEntry(key, _) = later
              && let AstNode::ContextEntryKey(later) = key.as_ref()
            {
              self.bound.push((later.clone(), FeelType::Any));
            }
          }
        }
        let value_type = self.node(value);
        self.unbind(shadows);
        let value_type = match value_type {
          FeelType::Null => FeelType::Any,
          value_type => value_type,
        };
        types.insert(name.clone(), value_type.clone());
        if let AstNode::FunctionDefinition(..) = value.as_ref() {
          self.definitions.push((self.bound.len(), value));
        }
        self.bound.push((name.clone(), value_type));
      }
    }
    self.unbind(mark);
    FeelType::Context(types)
  }

  fn invocation(&mut self, callee: &'n AstNode, arguments: &'n AstNode) -> FeelType {
    let argument_types: Vec<FeelType> = match arguments {
      AstNode::PositionalParameters(items) => items.iter().map(|item| self.node(item)).collect(),
      AstNode::NamedParameters(items) => items
        .iter()
        .map(|item| match item {
          AstNode::NamedParameter(_, value) => self.node(value),
          other => self.node(other),
        })
        .collect(),
      _ => vec![],
    };
    let positional = matches!(arguments, AstNode::PositionalParameters(_));
    if let AstNode::Name(name) = callee {
      // a variable, a parameter or a value of the scope of this name is what is invoked
      let known = self.name(name);
      if let FeelType::Function(_, result) = known {
        // a function literal of the text: its body with the types of the arguments
        return match self.typed_call(name, &argument_types, arguments) {
          Some(called) if called != FeelType::Any => called,
          _ => *result,
        };
      }
      if known == FeelType::Any
        && let Some(built_in) = built_in(name.as_str())
      {
        return built_in_result(
          built_in.name,
          built_in.signatures.iter().map(|signature| signature.result),
          &argument_types,
          positional,
          arguments,
        );
      }
      return FeelType::Any;
    }
    match self.node(callee) {
      FeelType::Function(_, result) => *result,
      _ => FeelType::Any,
    }
  }
}

impl<'n> Typer<'_, 'n> {
  /// The type of the result of an invocation of `name` with arguments of the types `arguments`
  /// (`written` as the text writes them), when the name is bound to a function literal of the
  /// text: the type of its body, where a parameter that has no type written has the type of its
  /// argument. The body sees the names bound where the literal is, not those of the
  /// invocation. None when the name is no such literal, the literal is being typed for an
  /// invocation already (it invokes itself), or the inference has typed [MAX_STEPS] nodes.
  fn typed_call(&mut self, name: &Name, arguments: &[FeelType], written: &AstNode) -> Option<FeelType> {
    let index = self.bound.iter().rposition(|(bound, _)| bound == name)?;
    let definition = self.definitions.iter().rev().find(|(at, _)| *at == index).map(|(_, definition)| *definition)?;
    let AstNode::FunctionDefinition(parameters, body) = definition else {
      return None;
    };
    let (AstNode::FormalParameters(items), AstNode::FunctionBody(expression, false)) = (parameters.as_ref(), body.as_ref()) else {
      return None;
    };
    if self.steps >= MAX_STEPS || self.active.contains(&(definition as *const AstNode)) {
      return None;
    }
    // the names of named arguments, in their order
    let named: Option<Vec<&Name>> = match written {
      AstNode::NamedParameters(named) => Some(
        named
          .iter()
          .filter_map(|argument| match argument {
            AstNode::NamedParameter(name, _) => match name.as_ref() {
              AstNode::ParameterName(name) => Some(name),
              _ => None,
            },
            _ => None,
          })
          .collect(),
      ),
      _ => None,
    };
    // the names bound after the literal are not those of its body: the parameters of the
    // constructs around the invocation are not seen there, and an entry of the context written
    // after the literal is seen with a value not typed here; a name of them is of no known type
    // in the body, so a name outside of the same text is not read in its place
    let mark = index + 1;
    let hidden_bound = self.bound.split_off(mark);
    let split = self.definitions.partition_point(|(at, _)| *at <= index);
    let hidden_definitions = self.definitions.split_off(split);
    self.active.push(definition);
    for (hidden, _) in &hidden_bound {
      self.bound.push((hidden.clone(), FeelType::Any));
    }
    for (position, item) in items.iter().enumerate() {
      if let AstNode::FormalParameter(parameter, parameter_type) = item
        && let AstNode::ParameterName(parameter) = parameter.as_ref()
      {
        let parameter_type = match written_type(parameter_type) {
          FeelType::Any => {
            let argument = match &named {
              Some(names) => names.iter().position(|name| *name == parameter).and_then(|at| arguments.get(at)),
              None => arguments.get(position),
            };
            match argument {
              Some(FeelType::Null) | None => FeelType::Any,
              Some(argument) => argument.clone(),
            }
          }
          written => written,
        };
        self.bound.push((parameter.clone(), parameter_type));
      }
    }
    let result = self.node(expression);
    self.unbind(mark);
    self.active.pop();
    self.bound.extend(hidden_bound);
    self.definitions.extend(hidden_definitions);
    Some(result)
  }
}

/// The type of the result of a built-in function: of its signatures when they agree, and for
/// the functions whose result has the type of an argument, that type.
fn built_in_result<'s>(name: &str, mut results: impl Iterator<Item = &'s str>, arguments: &[FeelType], positional: bool, written: &AstNode) -> FeelType {
  let first = arguments.first().filter(|_| positional);
  match (name, first) {
    // a list of the items of the first list
    ("append" | "concatenate" | "insert before" | "remove" | "reverse" | "sublist" | "sort" | "distinct values" | "union" | "list replace", Some(list @ FeelType::List(_))) => {
      return list.clone();
    }
    // an item of the list, or one of the arguments
    ("min" | "max" | "mode" | "median", Some(FeelType::List(item))) if **item != FeelType::Any => {
      return if name == "mode" { FeelType::List(item.clone()) } else { item.as_ref().clone() };
    }
    ("min" | "max", Some(argument)) if !matches!(argument, FeelType::List(_) | FeelType::Any) => return argument.clone(),
    ("abs", Some(argument @ (FeelType::Number | FeelType::DaysAndTimeDuration | FeelType::YearsAndMonthsDuration))) => return argument.clone(),
    // the entry of a context that a string names
    ("get value", Some(context @ FeelType::Context(_))) => {
      if let AstNode::PositionalParameters(items) = written
        && let Some(AstNode::String(key)) = items.get(1)
      {
        return member(context, key);
      }
      return FeelType::Any;
    }
    ("range", _) => {
      return FeelType::Range(Box::new(FeelType::Any));
    }
    _ => {}
  }
  let Some(result) = results.next() else {
    return FeelType::Any;
  };
  if results.all(|other| other == result) { signature_type(result) } else { FeelType::Any }
}

/// The type of the member `name` of a value of the type `base`, `Any` when the type has none
/// of this name.
fn member(base: &FeelType, name: &str) -> FeelType {
  members_of(base)
    .into_iter()
    .find(|(member, _)| member == name)
    .map_or(FeelType::Any, |(_, member_type)| member_type)
}

/// The type of an `@` literal, by the shape of its text.
fn at_literal(text: &str) -> FeelType {
  let text = text.trim();
  if text.starts_with('P') || text.starts_with("-P") {
    return if text.contains('T') || text.contains('D') {
      FeelType::DaysAndTimeDuration
    } else {
      FeelType::YearsAndMonthsDuration
    };
  }
  let date = text
    .as_bytes()
    .iter()
    .skip(1)
    .position(|&byte| byte == b'-')
    .is_some_and(|dash| text[..dash + 1].trim_start_matches('-').chars().all(|ch| ch.is_ascii_digit()));
  match (date, text.contains('T')) {
    (true, true) => FeelType::DateTime,
    (true, false) => FeelType::Date,
    (false, _) if text.contains(':') => FeelType::Time,
    _ => FeelType::Any,
  }
}

/// The type that the type written after a parameter is.
fn written_type(node: &AstNode) -> FeelType {
  match node {
    AstNode::FeelType(feel_type) => feel_type.clone(),
    AstNode::ListType(item) => FeelType::List(Box::new(written_type(item))),
    AstNode::RangeType(endpoint) => FeelType::Range(Box::new(written_type(endpoint))),
    AstNode::ContextType(entries) => FeelType::Context(
      entries
        .iter()
        .filter_map(|entry| match entry {
          AstNode::ContextTypeEntry(key, entry_type) => match key.as_ref() {
            AstNode::ContextTypeEntryKey(name) => Some((name.clone(), written_type(entry_type))),
            _ => None,
          },
          _ => None,
        })
        .collect(),
    ),
    AstNode::FunctionType(parameters, result) => FeelType::Function(
      match parameters.as_ref() {
        AstNode::ParameterTypes(items) => items.iter().map(written_type).collect(),
        _ => vec![],
      },
      Box::new(written_type(result)),
    ),
    _ => FeelType::Any,
  }
}

/// The type of `a + b`. An operand of a type not known (`Any`) beside a number or a string
/// gives a number or a string: FEEL adds a number only to a number and a string only to a
/// string (Tables 57 and 58), so a sum that is not null is of that type.
fn addition(a: &FeelType, b: &FeelType) -> FeelType {
  use FeelType::*;
  match (a, b) {
    (Number, Number | Any) | (Any, Number) => Number,
    (String, String | Any) | (Any, String) => String,
    (DaysAndTimeDuration, DaysAndTimeDuration) => DaysAndTimeDuration,
    (YearsAndMonthsDuration, YearsAndMonthsDuration) => YearsAndMonthsDuration,
    (temporal @ (Date | DateTime), DaysAndTimeDuration | YearsAndMonthsDuration) | (DaysAndTimeDuration | YearsAndMonthsDuration, temporal @ (Date | DateTime)) => temporal.clone(),
    (Time, DaysAndTimeDuration) | (DaysAndTimeDuration, Time) => Time,
    _ => Any,
  }
}

/// The type of `a - b`; a number and an operand of a type not known give a number, as for
/// [addition].
fn subtraction(a: &FeelType, b: &FeelType) -> FeelType {
  use FeelType::*;
  match (a, b) {
    (Number, Number | Any) | (Any, Number) => Number,
    (DaysAndTimeDuration, DaysAndTimeDuration) => DaysAndTimeDuration,
    (YearsAndMonthsDuration, YearsAndMonthsDuration) => YearsAndMonthsDuration,
    (Date | DateTime, Date | DateTime) | (Time, Time) => DaysAndTimeDuration,
    (temporal @ (Date | DateTime), DaysAndTimeDuration | YearsAndMonthsDuration) => temporal.clone(),
    (Time, DaysAndTimeDuration) => Time,
    _ => Any,
  }
}
