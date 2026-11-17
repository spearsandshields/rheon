//! A hand-written parser for the `FEEL` grammar (prototype, beside the table-driven [crate::parser]).
//!
//! Recursive descent for the constructs, and one loop with the precedence levels of the
//! grammar for the operators ([Parser::infix]). It reads the tokens of today's [Lexer] and
//! tells it what it expects at the places where the table-driven parser does so through its
//! actions, and it gives the same [AstNode] for every text.
//!
//! A token is read from the lexer when the parser first looks at it, never earlier: the lexer
//! reads a name with the scope and the expectations of that moment.

use crate::AstNode;
use crate::errors::*;
use crate::lalr::TokenType;
use crate::lexer::{Lexer, TokenValue};
use crate::scope::ParsingScope;
use dsntk_common::Result;
use rheon_feel::{FeelType, IntervalType, Name};

/// The most expressions that are open at once while parsing: an operand inside an operand
/// counts one. A text that is nested deeper is an error.
pub const MAX_NESTING: usize = 640;

/// The precedence levels of the operators, lowest first (the `%left`, `%right`, `%nonassoc`
/// and `%precedence` lines of the grammar).
const LOWEST: u8 = 0;
const OR: u8 = 1;
const AND: u8 = 2;
const COMPARISON: u8 = 3;
const BETWEEN: u8 = 4;
const BETWEEN_AND: u8 = 5;
const IN: u8 = 6;
const ADDITION: u8 = 7;
const MULTIPLICATION: u8 = 8;
const EXPONENTIATION: u8 = 9;
const NEGATION: u8 = 10;

/// The kind of the token the parser looks at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum T {
  And,
  At,
  Between,
  BetweenAnd,
  Boolean,
  BuiltInTypeName,
  Colon,
  Comma,
  Context,
  Descendant,
  Div,
  Dot,
  Ellipsis,
  Else,
  Eof,
  Eq,
  Every,
  Exp,
  External,
  For,
  Function,
  Ge,
  Gt,
  If,
  In,
  Instance,
  Le,
  LeftBrace,
  LeftBracket,
  LeftParen,
  List,
  Lt,
  Minus,
  Mul,
  Name,
  NameDateTime,
  Ne,
  Not,
  Null,
  Numeric,
  Of,
  Or,
  Other,
  Plus,
  Range,
  Return,
  RightArrow,
  RightBrace,
  RightBracket,
  RightParen,
  Satisfies,
  Some,
  String,
  Then,
}

/// Returns the kind of a token.
fn kind_of(value: &TokenValue) -> T {
  match value {
    TokenValue::And => T::And,
    TokenValue::At => T::At,
    TokenValue::Between => T::Between,
    TokenValue::BetweenAnd => T::BetweenAnd,
    TokenValue::Boolean(_) => T::Boolean,
    TokenValue::BuiltInTypeName(_) => T::BuiltInTypeName,
    TokenValue::Colon => T::Colon,
    TokenValue::Comma => T::Comma,
    TokenValue::Context => T::Context,
    TokenValue::Descendant => T::Descendant,
    TokenValue::Div => T::Div,
    TokenValue::Dot => T::Dot,
    TokenValue::Ellipsis => T::Ellipsis,
    TokenValue::Else => T::Else,
    TokenValue::YyEof => T::Eof,
    TokenValue::Eq => T::Eq,
    TokenValue::Every => T::Every,
    TokenValue::Exp => T::Exp,
    TokenValue::External => T::External,
    TokenValue::For => T::For,
    TokenValue::Function => T::Function,
    TokenValue::Ge => T::Ge,
    TokenValue::Gt => T::Gt,
    TokenValue::If => T::If,
    TokenValue::In => T::In,
    TokenValue::Instance => T::Instance,
    TokenValue::Le => T::Le,
    TokenValue::LeftBrace => T::LeftBrace,
    TokenValue::LeftBracket => T::LeftBracket,
    TokenValue::LeftParen => T::LeftParen,
    TokenValue::List => T::List,
    TokenValue::Lt => T::Lt,
    TokenValue::Minus => T::Minus,
    TokenValue::Mul => T::Mul,
    TokenValue::Name(_) => T::Name,
    TokenValue::NameDateTime(_) => T::NameDateTime,
    TokenValue::Nq => T::Ne,
    TokenValue::Not => T::Not,
    TokenValue::Null => T::Null,
    TokenValue::Numeric(..) => T::Numeric,
    TokenValue::Of => T::Of,
    TokenValue::Or => T::Or,
    TokenValue::Plus => T::Plus,
    TokenValue::Range => T::Range,
    TokenValue::Return => T::Return,
    TokenValue::RightArrow => T::RightArrow,
    TokenValue::RightBrace => T::RightBrace,
    TokenValue::RightBracket => T::RightBracket,
    TokenValue::RightParen => T::RightParen,
    TokenValue::Satisfies => T::Satisfies,
    TokenValue::Some => T::Some,
    TokenValue::String(_) => T::String,
    TokenValue::Then => T::Then,
    _ => T::Other,
  }
}

/// How an expression was made, for the start rules that take only some expressions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Made {
  /// A list, a function definition or a context, alone.
  Boxed,
  /// A number, a string, a boolean, an `@` literal or a date and time literal, alone.
  SimpleLiteral,
  /// An addition, subtraction, multiplication, division or exponentiation.
  Arithmetic,
  /// An arithmetic negation.
  Negation,
  /// Any other textual expression.
  Textual,
}

/// Where the parser is when it reads a token.
#[derive(Clone, Copy, PartialEq, Eq)]
enum At {
  /// Where an operand or a certain token is expected.
  Start,
  /// After a complete operand: an operator or the end of the construct follows.
  AfterOperand,
}

/// The hand-written parser.
///
/// The functions that call each other for a nested expression keep their stack frames small:
/// a node is built in place (`&mut AstNode`), and every construct has its own function.
pub struct Parser<'parser, 'scope> {
  /// Parsing scope.
  scope: &'parser ParsingScope<'scope>,
  /// Parsed input.
  input: &'parser str,
  /// The lexer.
  lexer: Lexer<'parser, 'scope>,
  /// The start rule.
  start: Option<TokenType>,
  /// The token the parser looks at, read from the lexer and not taken yet.
  token: Option<TokenValue>,
  /// The kind of `token`.
  kind: T,
  /// How the expression parsed last was made.
  made: Made,
  /// The expression being parsed ends the middle operand of a `between`: an `and` after a
  /// complete operand is the `and` of that `between`.
  between: bool,
  /// The number of expressions that are open.
  nesting: usize,
  /// The token log: every token read from the lexer, with its start offset in the input.
  #[cfg(test)]
  token_log: Option<Vec<(TokenType, TokenValue, usize)>>,
}

#[cfg(test)]
thread_local! {
  /// The lowest address of the stack seen while parsing (a measurement of the tests).
  pub(crate) static STACK_LOW: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

/// Replaces the node `lhs` with a node made of it.
fn wrap(lhs: &mut AstNode, node: impl FnOnce(Box<AstNode>) -> AstNode) {
  let inner = Box::new(std::mem::replace(lhs, AstNode::Null));
  *lhs = node(inner);
}

impl<'parser, 'scope> Parser<'parser, 'scope> {
  /// Creates a new parser.
  pub fn new(scope: &'parser ParsingScope<'scope>, start_token_type: TokenType, input: &'parser str) -> Self {
    Self {
      scope,
      input,
      lexer: Lexer::new(scope, start_token_type.clone(), input),
      start: Some(start_token_type),
      token: None,
      kind: T::Other,
      made: Made::Textual,
      between: false,
      nesting: 0,
      #[cfg(test)]
      token_log: None,
    }
  }

  /// Reads the text as a name that is being declared (see [crate::parse_name]).
  pub fn with_whole_names(mut self) -> Self {
    self.lexer.set_whole_names();
    self
  }

  /// Records every token read from the lexer (see [Parser::token_log]).
  #[cfg(test)]
  pub fn with_token_log(mut self) -> Self {
    self.token_log = Some(vec![]);
    self
  }

  /// Returns the recorded tokens.
  #[cfg(test)]
  pub fn token_log(&mut self) -> Vec<(TokenType, TokenValue, usize)> {
    self.token_log.take().unwrap_or_default()
  }

  /// Parses the input.
  pub fn parse(&mut self) -> Result<AstNode> {
    // the start token of the lexer
    let (_token_type, _token_value) = self.lexer.next_token()?;
    #[cfg(test)]
    if let Some(log) = &mut self.token_log {
      log.push((_token_type, _token_value, self.lexer.token_start()));
    }
    let node = match self.start.take() {
      Some(TokenType::StartExpression) => self.expression(LOWEST)?,
      Some(TokenType::StartBoxedExpression) => self.start_boxed_expression()?,
      Some(TokenType::StartTextualExpression) => self.textual_expression()?,
      Some(TokenType::StartTextualExpressions) => {
        let mut items = vec![self.textual_expression()?];
        while self.kind == T::Comma {
          self.bump();
          items.push(self.textual_expression()?);
        }
        AstNode::ExpressionList(items)
      }
      Some(TokenType::StartUnaryTests) => self.start_unary_tests()?,
      Some(TokenType::StartSimpleExpression) => self.simple_expression()?,
      Some(TokenType::StartSimpleExpressions) => {
        let mut items = vec![self.simple_expression()?];
        while self.kind == T::Comma {
          self.bump();
          items.push(self.simple_expression()?);
        }
        AstNode::ExpressionList(items)
      }
      Some(TokenType::StartSimpleValue) => self.start_simple_value()?,
      Some(TokenType::StartRangeLiteral) => self.start_range_literal()?,
      _ => return self.error(),
    };
    // the end of the input
    self.look(At::AfterOperand)?;
    if self.kind == T::Eof { Ok(node) } else { self.error() }
  }

  //----------------------------------------------------------------------------------------------
  // Tokens
  //----------------------------------------------------------------------------------------------

  /// Returns the syntax error.
  #[cold]
  fn error<V>(&self) -> Result<V> {
    Err(err_syntax_error(self.input))
  }

  /// Looks at the next token: reads it from the lexer unless that was done.
  fn look(&mut self, at: At) -> Result<T> {
    if self.token.is_none() {
      self.read(at)?;
    }
    Ok(self.kind)
  }

  /// Reads the next token from the lexer.
  #[inline(never)]
  fn read(&mut self, at: At) -> Result<()> {
    #[allow(unused_mut)]
    let (mut token_type, mut token_value) = self.lexer.next_token()?;
    // the `and` after the middle operand of a `between`
    if at == At::AfterOperand && self.between && matches!(token_value, TokenValue::And) {
      token_type = TokenType::BetweenAnd;
      token_value = TokenValue::BetweenAnd;
    }
    self.kind = kind_of(&token_value);
    #[cfg(test)]
    if let Some(log) = &mut self.token_log {
      log.push((token_type.clone(), token_value.clone(), self.lexer.token_start()));
    }
    let _ = token_type;
    self.token = Some(token_value);
    Ok(())
  }

  /// Takes the token the parser looks at.
  fn take(&mut self) -> TokenValue {
    self.token.take().unwrap_or(TokenValue::YyEmpty)
  }

  /// Passes the token the parser looks at.
  fn bump(&mut self) {
    self.token = None;
  }

  /// Passes the next token, which must be of the kind `kind`.
  fn expect(&mut self, kind: T, at: At) -> Result<()> {
    if self.look(at)? == kind {
      self.bump();
      Ok(())
    } else {
      self.error()
    }
  }

  /// Takes the next token, which must be a name.
  #[inline(never)]
  fn name(&mut self, at: At) -> Result<Name> {
    if self.look(at)? == T::Name
      && let TokenValue::Name(name) = self.take()
    {
      Ok(name)
    } else {
      self.error()
    }
  }

  //----------------------------------------------------------------------------------------------
  // Start rules
  //----------------------------------------------------------------------------------------------

  /// A boxed expression alone: a list, a function definition or a context.
  fn start_boxed_expression(&mut self) -> Result<AstNode> {
    self.enter()?;
    let node = match self.look(At::Start)? {
      T::LeftBracket => self.list_or_interval(false)?,
      T::Function => self.function_definition()?,
      T::LeftBrace => self.context()?,
      _ => return self.error(),
    };
    self.nesting -= 1;
    Ok(node)
  }

  /// A textual expression: any expression but a boxed expression alone.
  fn textual_expression(&mut self) -> Result<AstNode> {
    let node = self.expression(LOWEST)?;
    if self.made == Made::Boxed { self.error() } else { Ok(node) }
  }

  /// A simple expression: an arithmetic expression, a simple literal or a qualified name.
  fn simple_expression(&mut self) -> Result<AstNode> {
    if self.look(At::Start)? == T::Name {
      let name = self.name(At::Start)?;
      return match self.look(At::AfterOperand)? {
        T::Dot => self.qualified_name(name),
        T::Eof | T::Comma => Ok(AstNode::QualifiedName(vec![AstNode::QualifiedNameSegment(name)])),
        _ => {
          self.enter()?;
          let mut node = AstNode::Name(name);
          self.made = Made::Textual;
          self.infix(&mut node, LOWEST, false)?;
          self.nesting -= 1;
          if matches!(self.made, Made::Arithmetic | Made::Negation) {
            Ok(node)
          } else {
            self.error()
          }
        }
      };
    }
    let node = self.expression(LOWEST)?;
    if matches!(self.made, Made::Arithmetic | Made::Negation | Made::SimpleLiteral) {
      Ok(node)
    } else {
      self.error()
    }
  }

  /// The segments of a qualified name after its first name, which a dot follows.
  fn qualified_name(&mut self, first: Name) -> Result<AstNode> {
    let mut segments = vec![AstNode::QualifiedNameSegment(first)];
    while self.look(At::AfterOperand)? == T::Dot {
      self.bump();
      self.lexer.set_after_dot();
      segments.push(AstNode::QualifiedNameSegment(self.name(At::Start)?));
    }
    Ok(AstNode::QualifiedName(segments))
  }

  /// A simple value: a qualified name or a simple literal.
  fn start_simple_value(&mut self) -> Result<AstNode> {
    match self.look(At::Start)? {
      T::Name => {
        let name = self.name(At::Start)?;
        self.qualified_name(name)
      }
      T::Numeric | T::String | T::Boolean | T::At | T::NameDateTime => {
        self.enter()?;
        let node = self.operand()?;
        self.nesting -= 1;
        Ok(node)
      }
      _ => self.error(),
    }
  }

  /// Unary tests: `-`, `not(` tests `)` or tests.
  fn start_unary_tests(&mut self) -> Result<AstNode> {
    self.lexer.set_unary_tests();
    let mut items = vec![];
    match self.look(At::Start)? {
      T::Not => {
        self.bump();
        self.expect(T::LeftParen, At::Start)?;
        loop {
          items.push(self.expression(LOWEST)?);
          match self.kind {
            T::Comma => self.bump(),
            T::RightParen => break self.bump(),
            _ => return self.error(),
          }
        }
        return Ok(AstNode::NegatedList(items));
      }
      T::Minus => {
        self.bump();
        if self.look(At::AfterOperand)? == T::Eof {
          return Ok(AstNode::Irrelevant);
        }
        self.enter()?;
        let mut node = AstNode::Neg(Box::new(self.expression(NEGATION)?));
        self.made = Made::Negation;
        self.infix(&mut node, LOWEST, false)?;
        self.nesting -= 1;
        items.push(node);
      }
      _ => items.push(self.expression(LOWEST)?),
    }
    while self.kind == T::Comma {
      self.bump();
      items.push(self.expression(LOWEST)?);
    }
    Ok(AstNode::ExpressionList(items))
  }

  /// A range literal.
  fn start_range_literal(&mut self) -> Result<AstNode> {
    let closed = match self.look(At::Start)? {
      T::LeftParen | T::RightBracket => false,
      T::LeftBracket => true,
      _ => return self.error(),
    };
    self.bump();
    let (start, has_start) = if self.look(At::Start)? == T::Ellipsis {
      if closed {
        return self.error();
      }
      (AstNode::IntervalStart(AstNode::Null.into(), IntervalType::OpenedUndef), false)
    } else {
      let endpoint = self.range_literal_endpoint()?;
      (
        AstNode::IntervalStart(Box::new(endpoint), if closed { IntervalType::Closed } else { IntervalType::Opened }),
        true,
      )
    };
    self.expect(T::Ellipsis, At::Start)?;
    let end = match self.look(At::Start)? {
      T::RightParen | T::LeftBracket if has_start => {
        self.bump();
        AstNode::IntervalEnd(AstNode::Null.into(), IntervalType::OpenedUndef)
      }
      _ => {
        let endpoint = self.range_literal_endpoint()?;
        let interval_type = match self.look(At::Start)? {
          T::RightParen | T::LeftBracket => IntervalType::Opened,
          T::RightBracket => IntervalType::Closed,
          _ => return self.error(),
        };
        self.bump();
        AstNode::IntervalEnd(Box::new(endpoint), interval_type)
      }
    };
    Ok(AstNode::Range(Box::new(start), Box::new(end)))
  }

  /// An endpoint of a range literal: a number with an optional sign, a string or a date and
  /// time literal.
  fn range_literal_endpoint(&mut self) -> Result<AstNode> {
    match self.look(At::Start)? {
      T::Numeric => self.numeric(),
      T::Minus => {
        self.bump();
        if self.look(At::Start)? != T::Numeric {
          return self.error();
        }
        Ok(AstNode::Neg(Box::new(self.numeric()?)))
      }
      T::String => self.string().map(AstNode::String),
      T::At => {
        self.bump();
        if self.look(At::Start)? != T::String {
          return self.error();
        }
        self.string().map(AstNode::At)
      }
      T::NameDateTime => {
        let TokenValue::NameDateTime(name) = self.take() else { return self.error() };
        self.expect(T::LeftParen, At::Start)?;
        let parameters = match self.look(At::Start)? {
          T::Name => {
            let parameter_name = self.name(At::Start)?;
            self.expect(T::Colon, At::Start)?;
            if self.look(At::Start)? != T::String {
              return self.error();
            }
            let value = self.string().map(AstNode::String)?;
            AstNode::NamedParameters(vec![AstNode::NamedParameter(Box::new(AstNode::ParameterName(parameter_name)), Box::new(value))])
          }
          T::String => AstNode::PositionalParameters(vec![self.string().map(AstNode::String)?]),
          _ => return self.error(),
        };
        self.expect(T::RightParen, At::Start)?;
        Ok(AstNode::FunctionInvocation(Box::new(AstNode::Name(name)), Box::new(parameters)))
      }
      _ => self.error(),
    }
  }

  //----------------------------------------------------------------------------------------------
  // Expressions
  //----------------------------------------------------------------------------------------------

  /// Counts an open expression.
  fn enter(&mut self) -> Result<()> {
    self.nesting += 1;
    #[cfg(test)]
    {
      let marker = 0u8;
      let address = &marker as *const u8 as usize;
      STACK_LOW.with(|low| low.set(low.get().min(address)));
    }
    if self.nesting > MAX_NESTING { Err(err_nested_too_deeply(MAX_NESTING)) } else { Ok(()) }
  }

  /// An expression whose operators bind tighter than the level `level` (as tight, for a level
  /// that is right associative).
  fn expression(&mut self, level: u8) -> Result<AstNode> {
    self.expression_until(level, false)
  }

  /// An expression (see [Parser::expression]). With `end_endpoint` it is an endpoint of an
  /// interval that a `[` after it ends.
  fn expression_until(&mut self, level: u8, end_endpoint: bool) -> Result<AstNode> {
    self.enter()?;
    let mut node = self.operand()?;
    self.infix(&mut node, level, end_endpoint)?;
    self.nesting -= 1;
    Ok(node)
  }

  /// An expression that is no part of the expressions around it: it is not the end of the
  /// middle operand of a `between`.
  fn inner(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let node = self.expression(LOWEST);
    self.between = between;
    node
  }

  /// The operators after the operand `lhs` that bind tighter than the level `level`, with their
  /// operands: `lhs` becomes the whole expression. With `end_endpoint` the expression is an
  /// endpoint of an interval that a `[` after it ends.
  #[inline(never)]
  fn infix(&mut self, lhs: &mut AstNode, level: u8, end_endpoint: bool) -> Result<()> {
    loop {
      let binary: fn(Box<AstNode>, Box<AstNode>) -> AstNode;
      let operator_level;
      match self.look(At::AfterOperand)? {
        T::Dot | T::Descendant => {
          self.path(lhs)?;
          continue;
        }
        T::LeftBracket => {
          if end_endpoint {
            break;
          }
          self.filter(lhs)?;
          continue;
        }
        T::LeftParen => {
          self.invocation(lhs)?;
          continue;
        }
        T::Instance => {
          self.instance_of(lhs)?;
          continue;
        }
        T::Between => {
          if BETWEEN <= level {
            break;
          }
          self.between(lhs)?;
          continue;
        }
        T::In => {
          // right associative
          if IN < level {
            break;
          }
          self.comparison_in(lhs)?;
          continue;
        }
        T::Or => (binary, operator_level) = (AstNode::Or, OR),
        T::And => (binary, operator_level) = (AstNode::And, AND),
        T::Eq => (binary, operator_level) = (AstNode::Eq, COMPARISON),
        T::Ne => (binary, operator_level) = (AstNode::Nq, COMPARISON),
        T::Lt => (binary, operator_level) = (AstNode::Lt, COMPARISON),
        T::Le => (binary, operator_level) = (AstNode::Le, COMPARISON),
        T::Gt => (binary, operator_level) = (AstNode::Gt, COMPARISON),
        T::Ge => (binary, operator_level) = (AstNode::Ge, COMPARISON),
        T::Plus => (binary, operator_level) = (AstNode::Add, ADDITION),
        T::Minus => (binary, operator_level) = (AstNode::Sub, ADDITION),
        T::Mul => (binary, operator_level) = (AstNode::Mul, MULTIPLICATION),
        T::Div => (binary, operator_level) = (AstNode::Div, MULTIPLICATION),
        T::Exp => (binary, operator_level) = (AstNode::Exp, EXPONENTIATION),
        _ => break,
      }
      // the binary operators are left associative; a comparison takes no comparison as an operand
      if operator_level < level {
        break;
      }
      if operator_level == level {
        if level == COMPARISON {
          return self.error();
        }
        break;
      }
      self.binary(lhs, binary, operator_level)?;
    }
    Ok(())
  }

  /// A binary operator and its right operand. The parser looks at the operator.
  #[inline(never)]
  fn binary(&mut self, lhs: &mut AstNode, binary: fn(Box<AstNode>, Box<AstNode>) -> AstNode, operator_level: u8) -> Result<()> {
    self.bump();
    let rhs = Box::new(self.expression(operator_level)?);
    wrap(lhs, |lhs| binary(lhs, rhs));
    self.made = if operator_level >= ADDITION { Made::Arithmetic } else { Made::Textual };
    Ok(())
  }

  /// A path or a descendant: `.` or `...` and a name.
  #[inline(never)]
  fn path(&mut self, lhs: &mut AstNode) -> Result<()> {
    let descendant = self.kind == T::Descendant;
    self.bump();
    self.lexer.set_after_dot();
    let name = Box::new(AstNode::Name(self.name(At::Start)?));
    wrap(lhs, |lhs| if descendant { AstNode::Descendant(lhs, name) } else { AstNode::Path(lhs, name) });
    self.made = Made::Textual;
    Ok(())
  }

  /// A filter: `[`, an expression and `]`.
  #[inline(never)]
  fn filter(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let filter = Box::new(self.inner()?);
    self.expect(T::RightBracket, At::AfterOperand)?;
    wrap(lhs, |lhs| AstNode::Filter(lhs, filter));
    self.made = Made::Textual;
    Ok(())
  }

  /// An invocation: `(` and the parameters.
  #[inline(never)]
  fn invocation(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let parameters = Box::new(self.parameters()?);
    wrap(lhs, |lhs| AstNode::FunctionInvocation(lhs, parameters));
    self.made = Made::Textual;
    Ok(())
  }

  /// `instance of` and a type.
  #[inline(never)]
  fn instance_of(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    self.expect(T::Of, At::Start)?;
    self.lexer.set_type_name();
    let expected_type = Box::new(self.type_expression()?);
    wrap(lhs, |lhs| AstNode::InstanceOf(lhs, expected_type));
    self.made = Made::Textual;
    Ok(())
  }

  /// `between`, the middle operand, `and` and the last operand.
  #[inline(never)]
  fn between(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let between = std::mem::replace(&mut self.between, true);
    let middle = self.expression(LOWEST);
    self.between = between;
    let middle = Box::new(middle?);
    self.expect(T::BetweenAnd, At::AfterOperand)?;
    let rhs = Box::new(self.expression(BETWEEN_AND)?);
    wrap(lhs, |lhs| AstNode::Between(lhs, middle, rhs));
    self.made = Made::Textual;
    Ok(())
  }

  /// `in` and its right side: an operand, or tests in parentheses.
  #[inline(never)]
  fn comparison_in(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let rhs = if self.look(At::Start)? == T::LeftParen {
      self.bump();
      let between = std::mem::replace(&mut self.between, false);
      let mut first = self.expression(LOWEST)?;
      match self.kind {
        T::Comma => {
          let mut items = vec![first];
          while self.kind == T::Comma {
            self.bump();
            items.push(self.expression(LOWEST)?);
          }
          self.expect(T::RightParen, At::AfterOperand)?;
          self.between = between;
          // the tests in parentheses are no operand: no operator of the right side follows them
          let tests = Box::new(AstNode::ExpressionList(items));
          wrap(lhs, |lhs| AstNode::In(lhs, tests));
          self.made = Made::Textual;
          return Ok(());
        }
        T::RightParen => self.bump(),
        T::Ellipsis => first = self.interval_end(first, IntervalType::Opened)?,
        _ => return self.error(),
      }
      self.between = between;
      self.infix(&mut first, IN, false)?;
      first
    } else {
      self.expression(IN)?
    };
    let rhs = Box::new(rhs);
    wrap(lhs, |lhs| AstNode::In(lhs, rhs));
    self.made = Made::Textual;
    Ok(())
  }

  /// An operand: a literal, a name, a parenthesized expression, an interval, a boxed
  /// expression, an expression that starts with a keyword, or an operand after a sign or a
  /// comparison operator.
  fn operand(&mut self) -> Result<AstNode> {
    self.made = Made::Textual;
    match self.look(At::Start)? {
      T::Numeric => {
        self.made = Made::SimpleLiteral;
        self.numeric()
      }
      T::String => {
        self.made = Made::SimpleLiteral;
        self.string().map(AstNode::String)
      }
      T::Boolean => {
        self.made = Made::SimpleLiteral;
        if let TokenValue::Boolean(value) = self.take() {
          Ok(AstNode::Boolean(value))
        } else {
          self.error()
        }
      }
      T::At => self.at_literal(),
      T::Null => {
        self.bump();
        Ok(AstNode::Null)
      }
      T::Name => self.name(At::Start).map(AstNode::Name),
      T::NameDateTime => self.date_time_literal(),
      T::LeftParen => self.parenthesized(),
      T::LeftBracket => self.list_or_interval(true),
      T::RightBracket => self.interval_from_right_bracket(),
      T::LeftBrace => self.context(),
      T::Function => self.function_definition(),
      T::If => self.if_expression(),
      T::For => self.for_expression(),
      T::Some | T::Every => self.quantified_expression(),
      T::Minus => self.negation(),
      T::Lt => self.unary_comparison(AstNode::UnaryLt),
      T::Le => self.unary_comparison(AstNode::UnaryLe),
      T::Gt => self.unary_comparison(AstNode::UnaryGt),
      T::Ge => self.unary_comparison(AstNode::UnaryGe),
      T::Eq => self.unary_comparison(AstNode::UnaryEq),
      T::Ne => self.unary_comparison(AstNode::UnaryNe),
      _ => self.error(),
    }
  }

  /// Takes the number the parser looks at.
  #[inline(never)]
  fn numeric(&mut self) -> Result<AstNode> {
    let TokenValue::Numeric(before, after, sign, exponent) = self.take() else {
      return self.error();
    };
    Ok(AstNode::Numeric(before, after, sign, exponent))
  }

  /// Takes the string the parser looks at.
  #[inline(never)]
  fn string(&mut self) -> Result<String> {
    let TokenValue::String(value) = self.take() else { return self.error() };
    Ok(value)
  }

  /// `@` and a string.
  #[inline(never)]
  fn at_literal(&mut self) -> Result<AstNode> {
    self.bump();
    if self.look(At::Start)? != T::String {
      return self.error();
    }
    let node = self.string().map(AstNode::At);
    self.made = Made::SimpleLiteral;
    node
  }

  /// A date and time literal: the name of its function, `(` and the parameters.
  #[inline(never)]
  fn date_time_literal(&mut self) -> Result<AstNode> {
    let TokenValue::NameDateTime(name) = self.take() else { return self.error() };
    self.expect(T::LeftParen, At::Start)?;
    let parameters = self.parameters()?;
    self.made = Made::SimpleLiteral;
    Ok(AstNode::FunctionInvocation(Box::new(AstNode::Name(name)), Box::new(parameters)))
  }

  /// An arithmetic negation: its operand takes only what binds tighter than the arithmetic.
  #[inline(never)]
  fn negation(&mut self) -> Result<AstNode> {
    self.bump();
    let operand = self.expression(NEGATION)?;
    self.made = Made::Negation;
    Ok(AstNode::Neg(Box::new(operand)))
  }

  /// A unary comparison: its operand takes the arithmetic and the `in` after it.
  #[inline(never)]
  fn unary_comparison(&mut self, unary: fn(Box<AstNode>) -> AstNode) -> Result<AstNode> {
    self.bump();
    let operand = self.expression(IN)?;
    self.made = Made::Textual;
    Ok(unary(Box::new(operand)))
  }

  /// An expression in parentheses, or an interval that starts with `(`.
  #[inline(never)]
  fn parenthesized(&mut self) -> Result<AstNode> {
    self.bump();
    let between = std::mem::replace(&mut self.between, false);
    let mut node = self.expression(LOWEST)?;
    match self.kind {
      T::RightParen => self.bump(),
      T::Ellipsis => node = self.interval_end(node, IntervalType::Opened)?,
      _ => return self.error(),
    }
    self.between = between;
    self.made = Made::Textual;
    Ok(node)
  }

  /// `if`, a condition, `then`, an expression, `else` and an expression.
  #[inline(never)]
  fn if_expression(&mut self) -> Result<AstNode> {
    self.bump();
    let between = std::mem::replace(&mut self.between, false);
    let condition = Box::new(self.expression(LOWEST)?);
    self.expect(T::Then, At::AfterOperand)?;
    let then = Box::new(self.expression(LOWEST)?);
    self.expect(T::Else, At::AfterOperand)?;
    self.between = between;
    let otherwise = Box::new(self.expression(LOWEST)?);
    self.made = Made::Textual;
    Ok(AstNode::If(condition, then, otherwise))
  }

  /// The rest of an interval after its start endpoint: `..`, the end endpoint and the closing
  /// bracket. The parser looks at the `..`.
  #[inline(never)]
  fn interval_end(&mut self, start: AstNode, start_type: IntervalType) -> Result<AstNode> {
    self.expect(T::Ellipsis, At::AfterOperand)?;
    let start = Box::new(AstNode::IntervalStart(Box::new(start), start_type));
    let end = Box::new(self.expression_until(LOWEST, true)?);
    let end_type = match self.kind {
      T::RightParen | T::LeftBracket => IntervalType::Opened,
      T::RightBracket => IntervalType::Closed,
      _ => return self.error(),
    };
    self.bump();
    Ok(AstNode::Range(start, Box::new(AstNode::IntervalEnd(end, end_type))))
  }

  /// An interval that starts with `]`. The parser looks at the `]`.
  #[inline(never)]
  fn interval_from_right_bracket(&mut self) -> Result<AstNode> {
    self.bump();
    let between = std::mem::replace(&mut self.between, false);
    let interval = self.interval_after_right_bracket()?;
    self.between = between;
    self.made = Made::Textual;
    Ok(interval)
  }

  /// An interval after the `]` that starts it. A `[` after its start endpoint is no filter, as
  /// after an end endpoint.
  fn interval_after_right_bracket(&mut self) -> Result<AstNode> {
    let start = self.expression_until(LOWEST, true)?;
    if self.kind != T::Ellipsis {
      return self.error();
    }
    self.interval_end(start, IntervalType::Opened)
  }

  /// A list or an interval that starts with `[`; only a list without `interval`.
  #[inline(never)]
  fn list_or_interval(&mut self, interval: bool) -> Result<AstNode> {
    self.bump();
    let first = if self.look(At::Start)? == T::RightBracket {
      self.bump();
      // an operand after `[]`: the `]` starts an interval, the first item of the list
      match self.look(At::AfterOperand)? {
        T::At | T::Every | T::For | T::LeftBrace | T::Null | T::Function | T::If | T::Some | T::Numeric | T::String | T::Boolean | T::Name | T::NameDateTime => {
          let between = std::mem::replace(&mut self.between, false);
          let mut first = self.interval_after_right_bracket()?;
          self.infix(&mut first, LOWEST, false)?;
          self.between = between;
          first
        }
        _ => {
          self.made = Made::Boxed;
          return Ok(AstNode::List(vec![]));
        }
      }
    } else {
      self.inner()?
    };
    let between = std::mem::replace(&mut self.between, false);
    let node = match self.kind {
      T::Ellipsis if interval => {
        let node = self.interval_end(first, IntervalType::Closed)?;
        self.made = Made::Textual;
        node
      }
      T::Comma | T::RightBracket => {
        let mut items = vec![first];
        while self.kind == T::Comma {
          self.bump();
          items.push(self.expression(LOWEST)?);
        }
        self.expect(T::RightBracket, At::AfterOperand)?;
        self.made = Made::Boxed;
        AstNode::List(items)
      }
      _ => return self.error(),
    };
    self.between = between;
    Ok(node)
  }

  /// The parameters of an invocation after its `(`.
  #[inline(never)]
  fn parameters(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let parameters = match self.look(At::Start)? {
      T::RightParen => {
        self.bump();
        AstNode::PositionalParameters(vec![])
      }
      T::Name => {
        let name = self.name(At::Start)?;
        if self.look(At::AfterOperand)? == T::Colon {
          self.named_parameters(name)?
        } else {
          self.enter()?;
          let mut first = AstNode::Name(name);
          self.infix(&mut first, LOWEST, false)?;
          self.nesting -= 1;
          self.positional_parameters(first)?
        }
      }
      _ => {
        let first = self.expression(LOWEST)?;
        self.positional_parameters(first)?
      }
    };
    self.between = between;
    Ok(parameters)
  }

  /// The named parameters after the name of the first one. The parser looks at the `:`.
  #[inline(never)]
  fn named_parameters(&mut self, mut name: Name) -> Result<AstNode> {
    let mut items = vec![];
    loop {
      self.expect(T::Colon, At::Start)?;
      let value = self.expression(LOWEST)?;
      items.push(AstNode::NamedParameter(Box::new(AstNode::ParameterName(name)), Box::new(value)));
      match self.kind {
        T::Comma => {
          self.bump();
          name = self.name(At::Start)?;
        }
        T::RightParen => break self.bump(),
        _ => return self.error(),
      }
    }
    Ok(AstNode::NamedParameters(items))
  }

  /// The positional parameters after the first one.
  #[inline(never)]
  fn positional_parameters(&mut self, first: AstNode) -> Result<AstNode> {
    let mut items = vec![first];
    while self.kind == T::Comma {
      self.bump();
      items.push(self.expression(LOWEST)?);
    }
    self.expect(T::RightParen, At::AfterOperand)?;
    Ok(AstNode::PositionalParameters(items))
  }

  /// A context. The parser looks at its `{`.
  #[inline(never)]
  fn context(&mut self) -> Result<AstNode> {
    self.bump();
    self.lexer.push_to_scope();
    let between = std::mem::replace(&mut self.between, false);
    let mut entries = vec![];
    if self.look(At::Start)? == T::RightBrace {
      self.bump();
    } else {
      loop {
        let name = match self.look(At::Start)? {
          T::Name => self.name(At::Start)?,
          T::String => Name::from(self.string()?),
          _ => return self.error(),
        };
        self.expect(T::Colon, At::Start)?;
        let value = Box::new(self.expression(LOWEST)?);
        self.lexer.add_parsed_name(name.clone());
        entries.push(AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(name)), value));
        match self.kind {
          T::Comma => self.bump(),
          T::RightBrace => break self.bump(),
          _ => return self.error(),
        }
      }
    }
    self.lexer.pop_from_scope();
    self.between = between;
    self.made = Made::Boxed;
    Ok(AstNode::Context(entries))
  }

  /// A function definition. The parser looks at its `function`.
  #[inline(never)]
  fn function_definition(&mut self) -> Result<AstNode> {
    self.bump();
    self.expect(T::LeftParen, At::Start)?;
    self.scope.push_default();
    let parameters = Box::new(self.formal_parameters()?);
    let external = self.look(At::Start)? == T::External;
    if external {
      self.bump();
    }
    let body = Box::new(self.expression(LOWEST)?);
    self.scope.pop();
    self.made = Made::Boxed;
    Ok(AstNode::FunctionDefinition(parameters, Box::new(AstNode::FunctionBody(body, external))))
  }

  /// The formal parameters of a function definition after their `(`.
  #[inline(never)]
  fn formal_parameters(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let mut parameters = vec![];
    if self.look(At::Start)? == T::RightParen {
      self.bump();
    } else {
      loop {
        let name = self.name(At::Start)?;
        let parameter_type = if self.look(At::AfterOperand)? == T::Colon {
          self.bump();
          self.lexer.set_type_name();
          self.type_expression()?
        } else {
          AstNode::FeelType(FeelType::Any)
        };
        self.scope.set_entry_name(name.clone());
        parameters.push(AstNode::FormalParameter(Box::new(AstNode::ParameterName(name)), Box::new(parameter_type)));
        match self.look(At::AfterOperand)? {
          T::Comma => self.bump(),
          T::RightParen => break self.bump(),
          _ => return self.error(),
        }
      }
    }
    self.between = between;
    Ok(AstNode::FormalParameters(parameters))
  }

  /// A `for` expression. The parser looks at its `for`.
  #[inline(never)]
  fn for_expression(&mut self) -> Result<AstNode> {
    self.bump();
    self.lexer.push_to_scope();
    self.lexer.add_name_to_scope(Name::from("partial"));
    let contexts = Box::new(self.iteration_contexts()?);
    let body = Box::new(self.expression(LOWEST)?);
    self.lexer.pop_from_scope();
    self.made = Made::Textual;
    Ok(AstNode::For(contexts, Box::new(AstNode::EvaluatedExpression(body))))
  }

  /// The iteration contexts of a `for` expression and its `return`.
  #[inline(never)]
  fn iteration_contexts(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let mut contexts = vec![];
    loop {
      self.lexer.set_till_in();
      let name = self.name(At::Start)?;
      let variable = Box::new(AstNode::Name(name.clone()));
      self.lexer.add_name_to_scope(name);
      self.expect(T::In, At::Start)?;
      let start = Box::new(self.expression(LOWEST)?);
      contexts.push(if self.kind == T::Ellipsis {
        self.bump();
        let end = Box::new(self.expression(LOWEST)?);
        AstNode::IterationContextInterval(variable, start, end)
      } else {
        AstNode::IterationContextSingle(variable, start)
      });
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    self.expect(T::Return, At::AfterOperand)?;
    self.between = between;
    Ok(AstNode::IterationContexts(contexts))
  }

  /// A `some` or `every` expression. The parser looks at its first keyword.
  #[inline(never)]
  fn quantified_expression(&mut self) -> Result<AstNode> {
    let every = self.kind == T::Every;
    self.bump();
    self.lexer.push_to_scope();
    let contexts = Box::new(self.quantified_contexts()?);
    let body = Box::new(self.expression(LOWEST)?);
    self.lexer.pop_from_scope();
    self.made = Made::Textual;
    let satisfies = Box::new(AstNode::Satisfies(body));
    Ok(if every {
      AstNode::Every(contexts, satisfies)
    } else {
      AstNode::Some(contexts, satisfies)
    })
  }

  /// The variables of a `some` or `every` expression with their lists, and its `satisfies`.
  #[inline(never)]
  fn quantified_contexts(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let mut contexts = vec![];
    loop {
      self.lexer.set_till_in();
      let name = self.name(At::Start)?;
      let variable = Box::new(AstNode::Name(name.clone()));
      self.lexer.add_name_to_scope(name);
      self.expect(T::In, At::Start)?;
      let domain = Box::new(self.expression(LOWEST)?);
      contexts.push(AstNode::QuantifiedContext(variable, domain));
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    self.expect(T::Satisfies, At::AfterOperand)?;
    self.between = between;
    Ok(AstNode::QuantifiedContexts(contexts))
  }

  /// A type. The lexer was told that a type is expected.
  #[inline(never)]
  fn type_expression(&mut self) -> Result<AstNode> {
    self.enter()?;
    let node = match self.look(At::Start)? {
      T::BuiltInTypeName => {
        let TokenValue::BuiltInTypeName(name) = self.take() else { return self.error() };
        AstNode::FeelType((&name).into())
      }
      T::Name => {
        let name = self.name(At::Start)?;
        self.qualified_name(name)?
      }
      T::List | T::Range => self.list_or_range_type()?,
      T::Context => self.context_type()?,
      T::Function => self.function_type()?,
      _ => return self.error(),
    };
    self.nesting -= 1;
    Ok(node)
  }

  /// `list<` or `range<`, a type and `>`.
  fn list_or_range_type(&mut self) -> Result<AstNode> {
    let list = self.kind == T::List;
    self.bump();
    self.expect(T::Lt, At::Start)?;
    self.lexer.set_type_name();
    let between = std::mem::replace(&mut self.between, false);
    let item_type = Box::new(self.type_expression()?);
    self.expect(T::Gt, At::AfterOperand)?;
    self.between = between;
    Ok(if list { AstNode::ListType(item_type) } else { AstNode::RangeType(item_type) })
  }

  /// `context<`, the entries with their types and `>`.
  fn context_type(&mut self) -> Result<AstNode> {
    self.bump();
    self.expect(T::Lt, At::Start)?;
    let between = std::mem::replace(&mut self.between, false);
    let mut entries = vec![];
    loop {
      let name = self.name(At::Start)?;
      self.expect(T::Colon, At::Start)?;
      self.lexer.set_type_name();
      let entry_type = Box::new(self.type_expression()?);
      entries.push(AstNode::ContextTypeEntry(Box::new(AstNode::ContextTypeEntryKey(name)), entry_type));
      match self.look(At::AfterOperand)? {
        T::Comma => self.bump(),
        T::Gt => break self.bump(),
        _ => return self.error(),
      }
    }
    self.between = between;
    Ok(AstNode::ContextType(entries))
  }

  /// `function<`, the types of the parameters, `>`, `->` and the type of the result.
  fn function_type(&mut self) -> Result<AstNode> {
    self.bump();
    self.expect(T::Lt, At::Start)?;
    self.lexer.set_type_name();
    let between = std::mem::replace(&mut self.between, false);
    let mut parameter_types = vec![];
    if self.look(At::Start)? == T::Gt {
      self.bump();
    } else {
      loop {
        parameter_types.push(self.type_expression()?);
        match self.look(At::AfterOperand)? {
          T::Comma => {
            self.bump();
            self.lexer.set_type_name();
          }
          T::Gt => break self.bump(),
          _ => return self.error(),
        }
      }
    }
    self.expect(T::RightArrow, At::Start)?;
    self.lexer.set_type_name();
    self.between = between;
    let result_type = Box::new(self.type_expression()?);
    Ok(AstNode::FunctionType(Box::new(AstNode::ParameterTypes(parameter_types)), result_type))
  }
}

#[cfg(test)]
mod stack_tests {
  use super::*;

  /// Prints the stack one level of nesting takes, for some kinds of nesting.
  #[test]
  fn stack_per_level() {
    let n = 200;
    let cases = [
      ("parentheses", format!("{}1{}", "(".repeat(n), ")".repeat(n)), n),
      ("sum in parentheses", format!("{}1{}", "(1+".repeat(n), ")".repeat(n)), 2 * n),
      ("list", format!("{}1{}", "[".repeat(n), "]".repeat(n)), n),
      ("context", format!("{}1{}", "{a:".repeat(n), "}".repeat(n)), n),
      ("negation", format!("{}1", "-".repeat(n)), n),
      ("if", format!("{}1", "if true then 1 else ".repeat(n)), n),
      ("invocation", format!("{}1{}", "f(".repeat(n), ")".repeat(n)), n),
      ("for", format!("{}x", "for x in [1] return ".repeat(n)), n),
      ("function", format!("{}1", "function(a) ".repeat(n)), n),
      ("filter", format!("{}1{}", "a[".repeat(n), "]".repeat(n)), n),
      ("between", format!("{}1", "1 between 0 and ".repeat(n)), n),
      ("in", format!("{}1", "1 in ".repeat(n)), n),
    ];
    for (name, input, levels) in cases {
      let handle = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
          let scope = ParsingScope::new(std::iter::empty());
          let top = 0u8;
          let top = &top as *const u8 as usize;
          STACK_LOW.with(|low| low.set(usize::MAX));
          let result = Parser::new(&scope, TokenType::StartExpression, &input).parse();
          assert!(result.is_ok(), "{name}");
          let low = STACK_LOW.with(|low| low.get());
          println!("STACK {name}: {} bytes per level of {levels}", (top - low) / levels);
        })
        .unwrap();
      handle.join().unwrap();
    }
  }
}
