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
/// counts one. A text that is nested deeper is a syntax error.
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
  /// The expression being parsed ends the middle operand of a `between`: an `and` after a
  /// complete operand is the `and` of that `between`.
  between: bool,
  /// The number of expressions that are open.
  nesting: usize,
  /// The token log: every token read from the lexer, with its start offset in the input.
  #[cfg(test)]
  token_log: Option<Vec<(TokenType, TokenValue, usize)>>,
}

type Parsed = Result<(AstNode, Made)>;

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
  fn error<V>(&self) -> Result<V> {
    Err(err_syntax_error(self.input))
  }

  /// Looks at the next token: reads it from the lexer unless that was done.
  fn look(&mut self, at: At) -> Result<T> {
    if self.token.is_none() {
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
        log.push((token_type, token_value.clone(), self.lexer.token_start()));
      }
      let _ = token_type;
      self.token = Some(token_value);
    }
    Ok(self.kind)
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
      T::LeftBracket => self.list_or_interval(false)?.0,
      T::Function => self.function_definition()?,
      T::LeftBrace => self.context()?,
      _ => return self.error(),
    };
    self.nesting -= 1;
    Ok(node)
  }

  /// A textual expression: any expression but a boxed expression alone.
  fn textual_expression(&mut self) -> Result<AstNode> {
    let (node, made) = self.operand_and_operators(LOWEST, false)?;
    if made == Made::Boxed { self.error() } else { Ok(node) }
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
          let (node, made) = self.infix(AstNode::Name(name), Made::Textual, LOWEST, false)?;
          self.nesting -= 1;
          if matches!(made, Made::Arithmetic | Made::Negation) { Ok(node) } else { self.error() }
        }
      };
    }
    let (node, made) = self.operand_and_operators(LOWEST, false)?;
    if matches!(made, Made::Arithmetic | Made::Negation | Made::SimpleLiteral) {
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
        let node = self.operand()?.0;
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
        let operand = self.expression(NEGATION)?;
        let node = self.infix(AstNode::Neg(Box::new(operand)), Made::Negation, LOWEST, false)?.0;
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
    if self.nesting > MAX_NESTING { self.error() } else { Ok(()) }
  }

  /// An expression whose operators bind tighter than the level `level` (as tight, for a level
  /// that is right associative).
  fn expression(&mut self, level: u8) -> Result<AstNode> {
    Ok(self.operand_and_operators(level, false)?.0)
  }

  /// An expression that is no part of the expressions around it: it is not the end of the
  /// middle operand of a `between`.
  fn inner(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let node = self.expression(LOWEST);
    self.between = between;
    node
  }

  /// An operand and the operators after it (see [Parser::infix]).
  fn operand_and_operators(&mut self, level: u8, end_endpoint: bool) -> Parsed {
    self.enter()?;
    let (operand, made) = self.operand()?;
    let parsed = self.infix(operand, made, level, end_endpoint);
    self.nesting -= 1;
    parsed
  }

  /// The operators after the operand `lhs` that bind tighter than the level `level`, with their
  /// operands. With `end_endpoint` the expression is the end of an interval, which a `[` after
  /// it closes.
  fn infix(&mut self, mut lhs: AstNode, mut made: Made, level: u8, end_endpoint: bool) -> Parsed {
    loop {
      let binary: fn(Box<AstNode>, Box<AstNode>) -> AstNode;
      let operator_level;
      match self.look(At::AfterOperand)? {
        T::Dot | T::Descendant => {
          let descendant = self.kind == T::Descendant;
          self.bump();
          self.lexer.set_after_dot();
          let name = Box::new(AstNode::Name(self.name(At::Start)?));
          lhs = if descendant {
            AstNode::Descendant(Box::new(lhs), name)
          } else {
            AstNode::Path(Box::new(lhs), name)
          };
          made = Made::Textual;
          continue;
        }
        T::LeftBracket => {
          if end_endpoint {
            break;
          }
          self.bump();
          let filter = self.inner()?;
          self.expect(T::RightBracket, At::AfterOperand)?;
          lhs = AstNode::Filter(Box::new(lhs), Box::new(filter));
          made = Made::Textual;
          continue;
        }
        T::LeftParen => {
          self.bump();
          let parameters = self.parameters()?;
          lhs = AstNode::FunctionInvocation(Box::new(lhs), Box::new(parameters));
          made = Made::Textual;
          continue;
        }
        T::Instance => {
          self.bump();
          self.expect(T::Of, At::Start)?;
          self.lexer.set_type_name();
          let expected_type = self.type_expression()?;
          lhs = AstNode::InstanceOf(Box::new(lhs), Box::new(expected_type));
          made = Made::Textual;
          continue;
        }
        T::Between => {
          if BETWEEN <= level {
            break;
          }
          self.bump();
          let between = std::mem::replace(&mut self.between, true);
          let middle = self.expression(LOWEST);
          self.between = between;
          let middle = middle?;
          self.expect(T::BetweenAnd, At::AfterOperand)?;
          let rhs = self.expression(BETWEEN_AND)?;
          lhs = AstNode::Between(Box::new(lhs), Box::new(middle), Box::new(rhs));
          made = Made::Textual;
          continue;
        }
        T::In => {
          // right associative
          if IN < level {
            break;
          }
          self.bump();
          let rhs = if self.look(At::Start)? == T::LeftParen {
            self.bump();
            let between = std::mem::replace(&mut self.between, false);
            let first = self.expression(LOWEST)?;
            match self.kind {
              T::Comma => {
                let mut items = vec![first];
                while self.kind == T::Comma {
                  self.bump();
                  items.push(self.expression(LOWEST)?);
                }
                self.expect(T::RightParen, At::AfterOperand)?;
                self.between = between;
                lhs = AstNode::In(Box::new(lhs), Box::new(AstNode::ExpressionList(items)));
                made = Made::Textual;
                continue;
              }
              T::RightParen => {
                self.bump();
                self.between = between;
                self.infix(first, Made::Textual, IN, false)?.0
              }
              T::Ellipsis => {
                let interval = self.interval_end(first, IntervalType::Opened)?;
                self.between = between;
                self.infix(interval, Made::Textual, IN, false)?.0
              }
              _ => return self.error(),
            }
          } else {
            self.expression(IN)?
          };
          lhs = AstNode::In(Box::new(lhs), Box::new(rhs));
          made = Made::Textual;
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
      self.bump();
      let rhs = self.expression(operator_level)?;
      lhs = binary(Box::new(lhs), Box::new(rhs));
      made = if operator_level >= ADDITION { Made::Arithmetic } else { Made::Textual };
    }
    Ok((lhs, made))
  }

  /// An operand: a literal, a name, a parenthesized expression, an interval, a boxed
  /// expression, an expression that starts with a keyword, or an operand after a sign or a
  /// comparison operator.
  fn operand(&mut self) -> Parsed {
    let unary: fn(Box<AstNode>) -> AstNode;
    match self.look(At::Start)? {
      T::Numeric => return Ok((self.numeric()?, Made::SimpleLiteral)),
      T::String => return Ok((AstNode::String(self.string()?), Made::SimpleLiteral)),
      T::Boolean => {
        let TokenValue::Boolean(value) = self.take() else { return self.error() };
        return Ok((AstNode::Boolean(value), Made::SimpleLiteral));
      }
      T::At => {
        self.bump();
        if self.look(At::Start)? != T::String {
          return self.error();
        }
        return Ok((AstNode::At(self.string()?), Made::SimpleLiteral));
      }
      T::Null => {
        self.bump();
        return Ok((AstNode::Null, Made::Textual));
      }
      T::Name => {
        let TokenValue::Name(name) = self.take() else { return self.error() };
        return Ok((AstNode::Name(name), Made::Textual));
      }
      T::NameDateTime => {
        let TokenValue::NameDateTime(name) = self.take() else { return self.error() };
        self.expect(T::LeftParen, At::Start)?;
        let parameters = self.parameters()?;
        return Ok((AstNode::FunctionInvocation(Box::new(AstNode::Name(name)), Box::new(parameters)), Made::SimpleLiteral));
      }
      T::LeftParen => {
        self.bump();
        let between = std::mem::replace(&mut self.between, false);
        let first = self.expression(LOWEST)?;
        let node = match self.kind {
          T::RightParen => {
            self.bump();
            first
          }
          T::Ellipsis => self.interval_end(first, IntervalType::Opened)?,
          _ => return self.error(),
        };
        self.between = between;
        return Ok((node, Made::Textual));
      }
      T::LeftBracket => return self.list_or_interval(true),
      T::RightBracket => {
        self.bump();
        let between = std::mem::replace(&mut self.between, false);
        let interval = self.interval_after_right_bracket()?;
        self.between = between;
        return Ok((interval, Made::Textual));
      }
      T::LeftBrace => return Ok((self.context()?, Made::Boxed)),
      T::Function => return Ok((self.function_definition()?, Made::Boxed)),
      T::If => {
        self.bump();
        let between = std::mem::replace(&mut self.between, false);
        let condition = self.expression(LOWEST)?;
        self.expect(T::Then, At::AfterOperand)?;
        let then = self.expression(LOWEST)?;
        self.expect(T::Else, At::AfterOperand)?;
        self.between = between;
        let otherwise = self.expression(LOWEST)?;
        return Ok((AstNode::If(Box::new(condition), Box::new(then), Box::new(otherwise)), Made::Textual));
      }
      T::For => return Ok((self.for_expression()?, Made::Textual)),
      T::Some | T::Every => return Ok((self.quantified_expression()?, Made::Textual)),
      T::Minus => {
        self.bump();
        let operand = self.expression(NEGATION)?;
        return Ok((AstNode::Neg(Box::new(operand)), Made::Negation));
      }
      T::Lt => unary = AstNode::UnaryLt,
      T::Le => unary = AstNode::UnaryLe,
      T::Gt => unary = AstNode::UnaryGt,
      T::Ge => unary = AstNode::UnaryGe,
      T::Eq => unary = AstNode::UnaryEq,
      T::Ne => unary = AstNode::UnaryNe,
      _ => return self.error(),
    }
    // a unary comparison: its operand takes the arithmetic and the `in` after it
    self.bump();
    let operand = self.expression(IN)?;
    Ok((unary(Box::new(operand)), Made::Textual))
  }

  /// Takes the number the parser looks at.
  fn numeric(&mut self) -> Result<AstNode> {
    let TokenValue::Numeric(before, after, sign, exponent) = self.take() else {
      return self.error();
    };
    Ok(AstNode::Numeric(before, after, sign, exponent))
  }

  /// Takes the string the parser looks at.
  fn string(&mut self) -> Result<String> {
    let TokenValue::String(value) = self.take() else { return self.error() };
    Ok(value)
  }

  /// The rest of an interval after its start endpoint: `..`, the end endpoint and the closing
  /// bracket. The parser looks at the `..`.
  fn interval_end(&mut self, start: AstNode, start_type: IntervalType) -> Result<AstNode> {
    self.expect(T::Ellipsis, At::AfterOperand)?;
    let end = self.operand_and_operators(LOWEST, true)?.0;
    let end_type = match self.kind {
      T::RightParen | T::LeftBracket => IntervalType::Opened,
      T::RightBracket => IntervalType::Closed,
      _ => return self.error(),
    };
    self.bump();
    Ok(AstNode::Range(
      Box::new(AstNode::IntervalStart(Box::new(start), start_type)),
      Box::new(AstNode::IntervalEnd(Box::new(end), end_type)),
    ))
  }

  /// An interval after the `]` that starts it. A `[` after its start endpoint is no filter, as
  /// after an end endpoint.
  fn interval_after_right_bracket(&mut self) -> Result<AstNode> {
    let start = self.operand_and_operators(LOWEST, true)?.0;
    if self.kind != T::Ellipsis {
      return self.error();
    }
    self.interval_end(start, IntervalType::Opened)
  }

  /// A list or an interval that starts with `[`; only a list without `interval`.
  fn list_or_interval(&mut self, interval: bool) -> Parsed {
    self.bump();
    let first = if self.look(At::Start)? == T::RightBracket {
      self.bump();
      // an operand after `[]`: the `]` starts an interval, the first item of the list
      match self.look(At::AfterOperand)? {
        T::At | T::Every | T::For | T::LeftBrace | T::Null | T::Function | T::If | T::Some | T::Numeric | T::String | T::Boolean | T::Name | T::NameDateTime => {
          let between = std::mem::replace(&mut self.between, false);
          let start = self.interval_after_right_bracket()?;
          let first = self.infix(start, Made::Textual, LOWEST, false)?.0;
          self.between = between;
          first
        }
        _ => return Ok((AstNode::List(vec![]), Made::Boxed)),
      }
    } else {
      self.inner()?
    };
    let between = std::mem::replace(&mut self.between, false);
    let parsed = match self.kind {
      T::Ellipsis if interval => (self.interval_end(first, IntervalType::Closed)?, Made::Textual),
      T::Comma | T::RightBracket => {
        let mut items = vec![first];
        while self.kind == T::Comma {
          self.bump();
          items.push(self.expression(LOWEST)?);
        }
        self.expect(T::RightBracket, At::AfterOperand)?;
        (AstNode::List(items), Made::Boxed)
      }
      _ => return self.error(),
    };
    self.between = between;
    Ok(parsed)
  }

  /// The parameters of an invocation after its `(`.
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
          let mut name = name;
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
          AstNode::NamedParameters(items)
        } else {
          self.enter()?;
          let first = self.infix(AstNode::Name(name), Made::Textual, LOWEST, false)?.0;
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

  /// The positional parameters after the first one.
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
        let value = self.expression(LOWEST)?;
        self.lexer.add_parsed_name(name.clone());
        entries.push(AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(name)), Box::new(value)));
        match self.kind {
          T::Comma => self.bump(),
          T::RightBrace => break self.bump(),
          _ => return self.error(),
        }
      }
    }
    self.lexer.pop_from_scope();
    self.between = between;
    Ok(AstNode::Context(entries))
  }

  /// A function definition. The parser looks at its `function`.
  fn function_definition(&mut self) -> Result<AstNode> {
    self.bump();
    self.expect(T::LeftParen, At::Start)?;
    self.scope.push_default();
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
    let external = self.look(At::Start)? == T::External;
    if external {
      self.bump();
    }
    let body = self.expression(LOWEST)?;
    self.scope.pop();
    Ok(AstNode::FunctionDefinition(
      Box::new(AstNode::FormalParameters(parameters)),
      Box::new(AstNode::FunctionBody(Box::new(body), external)),
    ))
  }

  /// A `for` expression. The parser looks at its `for`.
  fn for_expression(&mut self) -> Result<AstNode> {
    self.bump();
    self.lexer.push_to_scope();
    self.lexer.add_name_to_scope(Name::from("partial"));
    let between = std::mem::replace(&mut self.between, false);
    let mut contexts = vec![];
    loop {
      self.lexer.set_till_in();
      let name = self.name(At::Start)?;
      let variable = Box::new(AstNode::Name(name.clone()));
      self.lexer.add_name_to_scope(name);
      self.expect(T::In, At::Start)?;
      let start = self.expression(LOWEST)?;
      contexts.push(if self.kind == T::Ellipsis {
        self.bump();
        let end = self.expression(LOWEST)?;
        AstNode::IterationContextInterval(variable, Box::new(start), Box::new(end))
      } else {
        AstNode::IterationContextSingle(variable, Box::new(start))
      });
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    self.expect(T::Return, At::AfterOperand)?;
    self.between = between;
    let body = self.expression(LOWEST)?;
    self.lexer.pop_from_scope();
    Ok(AstNode::For(
      Box::new(AstNode::IterationContexts(contexts)),
      Box::new(AstNode::EvaluatedExpression(Box::new(body))),
    ))
  }

  /// A `some` or `every` expression. The parser looks at its first keyword.
  fn quantified_expression(&mut self) -> Result<AstNode> {
    let every = self.kind == T::Every;
    self.bump();
    self.lexer.push_to_scope();
    let between = std::mem::replace(&mut self.between, false);
    let mut contexts = vec![];
    loop {
      self.lexer.set_till_in();
      let name = self.name(At::Start)?;
      let variable = Box::new(AstNode::Name(name.clone()));
      self.lexer.add_name_to_scope(name);
      self.expect(T::In, At::Start)?;
      let domain = self.expression(LOWEST)?;
      contexts.push(AstNode::QuantifiedContext(variable, Box::new(domain)));
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    self.expect(T::Satisfies, At::AfterOperand)?;
    self.between = between;
    let body = self.expression(LOWEST)?;
    self.lexer.pop_from_scope();
    let contexts = Box::new(AstNode::QuantifiedContexts(contexts));
    let satisfies = Box::new(AstNode::Satisfies(Box::new(body)));
    Ok(if every {
      AstNode::Every(contexts, satisfies)
    } else {
      AstNode::Some(contexts, satisfies)
    })
  }

  /// A type. The lexer was told that a type is expected.
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
      T::List | T::Range => {
        let list = self.kind == T::List;
        self.bump();
        self.expect(T::Lt, At::Start)?;
        self.lexer.set_type_name();
        let between = std::mem::replace(&mut self.between, false);
        let item_type = Box::new(self.type_expression()?);
        self.expect(T::Gt, At::AfterOperand)?;
        self.between = between;
        if list { AstNode::ListType(item_type) } else { AstNode::RangeType(item_type) }
      }
      T::Context => {
        self.bump();
        self.expect(T::Lt, At::Start)?;
        let between = std::mem::replace(&mut self.between, false);
        let mut entries = vec![];
        loop {
          let name = self.name(At::Start)?;
          self.expect(T::Colon, At::Start)?;
          self.lexer.set_type_name();
          let entry_type = self.type_expression()?;
          entries.push(AstNode::ContextTypeEntry(Box::new(AstNode::ContextTypeEntryKey(name)), Box::new(entry_type)));
          match self.look(At::AfterOperand)? {
            T::Comma => self.bump(),
            T::Gt => break self.bump(),
            _ => return self.error(),
          }
        }
        self.between = between;
        AstNode::ContextType(entries)
      }
      T::Function => {
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
        let result_type = self.type_expression()?;
        AstNode::FunctionType(Box::new(AstNode::ParameterTypes(parameter_types)), Box::new(result_type))
      }
      _ => return self.error(),
    };
    self.nesting -= 1;
    Ok(node)
  }
}
