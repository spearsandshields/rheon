//! Implementation of the lexer for `FEEL` grammar.

use crate::errors::*;
use crate::scope::{ParsingScope, first_word};
use crate::token::TokenType;
use dsntk_common::Result;
use rheon_feel::Name;
use rheon_feel::bif::{is_built_in_date_time_function_name, is_built_in_function_name};
use rheon_feel::bip::{BUILT_IN_PROPERTY_NAMES, is_built_in_property_name};
use std::ops::Range;

/// Definition of a single space character.
const WS: char = ' ';

/// Buffer size for lexer input.
const BUF_SIZE: usize = 12;

/// Definition of decimal separator.
const DECIMAL_SEPARATOR: char = '.';

/// The state of the flattened text of a chain (`Lexer::chain_flat`).
#[derive(Clone, Copy, PartialEq)]
enum FlatChain {
  Unknown,
  Written,
  NotRegular,
}

/// The value of the remembered search for a part `in` (`Lexer::chain_in`) of a chain that was
/// not searched: no part is in its range.
const NOT_SEARCHED: (usize, usize) = (usize::MAX, 0);

/// The keywords that can follow an operand. A name that is not in scope ends before a word of
/// its chain that is one of them: `for s in x return s` and `x and y` have their keywords also
/// when `x` is no name in scope. `as` (DMN 1.7 Beta 1, grammar rule 51) ends such a name only
/// when a type follows it that ends the name ([type_ends_name]), and never a name being declared
/// ([Lexer::set_declared_name]): `item as number` in a filter converts `item`, while a name of
/// FEEL 1.6 that holds the word keeps it (`p.Cost as percent`, `p.Count as number of items`,
/// entries of a parameter; `function(cost as string)`).
/// A name in scope is matched first, so it may hold these words (`tax return`, `cost as
/// percent`); names are resolved by the scope (10.3.1.2). The other keywords never follow an
/// operand, so a name ending before them would only be a syntax error.
const NAME_ENDING_KEYWORDS: [&str; 10] = ["and", "as", "between", "else", "in", "instance", "or", "return", "satisfies", "then"];

/// The additional name symbols that are operators or punctuation of an expression. A name that
/// is not in scope ends before one of them: `a + b` is a sum and `x.y` a path also when `a` and
/// `x` are no names in scope (DMN 1.6 §10.3.1.6: a symbol is part of a name when the names in
/// scope say so). The apostrophe is no such symbol: a name would end before it only to leave a
/// text that is no expression.
const NAME_ENDING_SYMBOLS: [&str; 5] = [".", "/", "-", "+", "*"];

/// The built-in names that have one of the [NAME_ENDING_KEYWORDS], as their words. They are
/// names of functions and types, which are never in the parsing scope, so a name that starts
/// with their words holds them whole.
/// The most words of a built-in function or type name (`years and months duration`).
const MAX_BUILT_IN_NAME_WORDS: usize = 4;

const BUILT_IN_NAMES_WITH_KEYWORDS: [&[&str]; 3] = [&["date", "and", "time"], &["years", "and", "months", "duration"], &["days", "and", "time", "duration"]];

/// Length in bytes of the longest built-in property name: after a dot, a candidate name that is
/// longer than this and than every name in scope can match nothing.
const MAX_BUILT_IN_PROPERTY_NAME_LEN: usize = longest(&BUILT_IN_PROPERTY_NAMES);

/// Semantic value associated with token type.
#[derive(Debug, Clone)]
pub enum TokenValue {
  YyEmpty,
  YyEof,
  YyError,
  YyUndef,
  And,
  At,
  Between,
  BetweenAnd,
  Boolean(bool),
  BuiltInTypeName(Name),
  Colon,
  Comma,
  Context,
  Div,
  Descendant,
  Dot,
  Ellipsis,
  Else,
  Eq,
  Every,
  Exp,
  External,
  For,
  Function,
  LeftBrace,
  LeftBracket,
  LeftParen,
  Le,
  Lt,
  Ge,
  Gt,
  If,
  In,
  As,
  Instance,
  List,
  Minus,
  Mul,
  Not,
  Name(Name),
  NameDateTime(Name),
  Nq,
  Null,
  Numeric(String, String, char, String),
  Of,
  Or,
  Plus,
  Range,
  Return,
  RightArrow,
  RightBrace,
  RightBracket,
  RightParen,
  Satisfies,
  Some,
  StartExpression,
  StartBoxedExpression,
  StartRangeLiteral,
  StartTextualExpression,
  StartUnaryTests,
  String(String),
  Then,
}

/// FEEL lexer.
pub struct Lexer<'lexer, 'scope> {
  /// Parsing scope.
  scope: &'lexer ParsingScope<'scope>,
  /// Starting token type, returned before the first token.
  start_token_type: Option<TokenType>,
  /// Input text.
  input: &'lexer str,
  /// Current cursor position in the input text (a byte offset at a character boundary).
  position: usize,
  /// Flag indicating if the unary tests rule is the starting point.
  /// This flag is used to identify `not` keyword,
  /// which otherwise would be recognized as a name.
  /// Token `not` is a keyword at the very beginning of the unary tests rule,
  /// in all other contexts it is just a name.
  unary_tests: bool,
  /// A type is expected: the next token may be the name of a built-in type or one of the
  /// keywords `list`, `range` and `context`. Set by the parser before every type, and over
  /// with the token read next.
  type_name: bool,
  /// A complete operand was read: the next token may be the operator `as` (DMN 1.7 Beta 1,
  /// grammar rule 51). `as` is no keyword elsewhere, so a name may start with it (`as of date`),
  /// and a name in scope that holds it is read whole before (`price as of today`). Set by the
  /// parser before every token, as where it reads it.
  after_operand: bool,
  /// ???
  till_in: bool,
  /// The text is a name that is being declared ([crate::parse_name]): a name that is not in
  /// scope is the whole chain of its parts, keywords and symbols included.
  whole_names: bool,
  /// The next token, when it is a name, is a name being declared ([Lexer::set_declared_name]).
  declared_name: bool,
  /// The token follows a dot: its name may be a built-in property (`year`, `time offset`,
  /// `start included`). Set by the parser after every dot, and over with the token read next.
  after_dot: bool,
  /// How many predicates of filters are open: the names in them can be entries of an item.
  filters: usize,
  /// Buffer for the flattened candidate names of [Lexer::longest_name_in_scope].
  name_buf: String,
  /// Second buffer for flattening names (the replacements alternate between the buffers).
  name_tmp: String,
  /// Parts of the last name chain scanned, see [ChainPart].
  chain_parts: Vec<ChainPart>,
  /// Position of the first character after the last name chain scanned.
  chain_end: usize,
  /// Index of the last part of the chain that is empty after trimming (only U+1680), if any.
  chain_last_empty: Option<usize>,
  /// The last search for a part `in` of the chain: the part it started at, and the first part
  /// `in` at or after it (the number of parts when there is none). See [Lexer::next_in_part].
  chain_in: (usize, usize),
  /// The flattened text of the whole chain, written once, when the chain is regular (see
  /// [Lexer::flat_chain]): the flattened name of the parts `a..=b` is a piece of it.
  chain_flat: String,
  /// For each part of the chain, where its text starts and ends in `chain_flat`.
  chain_flat_parts: Vec<(usize, usize)>,
  /// Whether `chain_flat` is written for the chain: not yet, written, or the chain is not regular.
  chain_flat_state: FlatChain,
  /// The index of the last part of the chain that is not regular (see
  /// [Lexer::has_irregular_parts]), once it was looked for.
  chain_last_irregular: Option<Option<usize>>,
  /// Start offset of the last token read from input: the hook of the token log of the parser
  /// (`Parser::with_token_log`), which the differential tests compare, and the place of a
  /// token for the hand-written parser.
  token_start: usize,
}

/// A part of a name chain: the name parts (words and additional name symbols) and the white space
/// between them that follow a `name start` character, up to the first character of none of
/// these kinds. The chain is scanned once per parse: a following name token that starts at
/// one of its parts (the token before took only some of the parts) reads the parts from there.
struct ChainPart {
  /// Position of the first character of the part.
  start: usize,
  /// Position after the last character of the part.
  end: usize,
  /// The text of the part without white space at both ends, in the input.
  trimmed: Range<usize>,
}

/// FEEL lexer implementation.
impl<'lexer, 'scope> Lexer<'lexer, 'scope> {
  /// Creates a new lexer for specified input text.
  pub fn new(scope: &'lexer ParsingScope<'scope>, start_token_type: TokenType, input: &'lexer str) -> Self {
    Self {
      scope,
      start_token_type: Some(start_token_type),
      input,
      position: 0,
      unary_tests: false,
      type_name: false,
      after_operand: false,
      till_in: false,
      whole_names: false,
      declared_name: false,
      after_dot: false,
      filters: 0,
      name_buf: String::new(),
      name_tmp: String::new(),
      chain_parts: vec![],
      chain_end: 0,
      chain_last_empty: None,
      chain_in: NOT_SEARCHED,
      chain_flat: String::new(),
      chain_flat_parts: vec![],
      chain_flat_state: FlatChain::Unknown,
      chain_last_irregular: None,
      token_start: 0,
    }
  }

  /// Returns the start offset of the last token read from input (a byte offset).
  pub fn token_start(&self) -> usize {
    self.token_start
  }

  /// Returns the position after the last token read from input (a byte offset).
  pub fn position(&self) -> usize {
    self.position
  }

  /// Moves the position: the next token is read from there.
  pub fn set_position(&mut self, position: usize) {
    self.position = position;
  }

  pub fn set_unary_tests(&mut self) {
    self.unary_tests = true;
  }

  pub fn set_type_name(&mut self) {
    self.type_name = true;
  }

  /// Tells whether the next token follows a complete operand ([Lexer::after_operand]).
  pub fn set_after_operand(&mut self, after_operand: bool) {
    self.after_operand = after_operand;
  }

  pub fn set_till_in(&mut self) {
    self.till_in = true;
  }

  /// Tells that the next token, when it is a name, is one being declared (a parameter of a
  /// function definition): `as` and a type do not end it, as they end a name that is not in
  /// scope where it is read ([NAME_ENDING_KEYWORDS]). For the next token only.
  pub fn set_declared_name(&mut self) {
    self.declared_name = true;
  }

  pub fn set_whole_names(&mut self) {
    self.whole_names = true;
  }

  pub fn set_after_dot(&mut self) {
    self.after_dot = true;
  }

  /// The predicate of a filter starts.
  pub fn enter_filter(&mut self) {
    self.filters += 1;
  }

  /// The predicate of a filter ends.
  pub fn leave_filter(&mut self) {
    self.filters = self.filters.saturating_sub(1);
  }

  pub fn push_to_scope(&mut self) {
    self.scope.push_default();
  }

  pub fn pop_from_scope(&mut self) {
    self.scope.pop();
  }

  pub fn add_name_to_scope(&mut self, name: Name) {
    self.scope.set_entry_name(name);
  }

  pub fn remove_name_from_scope(&mut self, name: &Name) {
    self.scope.remove_entry_name(name);
  }

  pub fn add_parsed_name(&mut self, name: Name) {
    self.scope.add_parsed_name(name);
  }

  /// Returns the next token from input.
  pub fn next_token(&mut self) -> Result<(TokenType, TokenValue)> {
    if let Some(start_token_type) = self.start_token_type.clone() {
      self.start_token_type = None;
      return Ok(match start_token_type {
        tt @ TokenType::StartExpression => (tt, TokenValue::StartExpression),
        tt @ TokenType::StartBoxedExpression => (tt, TokenValue::StartBoxedExpression),
        tt @ TokenType::StartRangeLiteral => (tt, TokenValue::StartRangeLiteral),
        tt @ TokenType::StartTextualExpression => (tt, TokenValue::StartTextualExpression),
        tt @ TokenType::StartUnaryTests => (tt, TokenValue::StartUnaryTests),
        _ => (TokenType::YyError, TokenValue::YyError),
      });
    }
    // a type, or a name after a dot, is expected for one token: the parser asks again before
    // every type and after every dot
    let (type_name, after_dot) = (self.type_name, self.after_dot);
    self.scope.set_members(after_dot || self.filters > 0);
    let result = self.read_next_token();
    self.declared_name = false;
    if type_name {
      self.type_name = false;
    }
    if after_dot {
      self.after_dot = false;
    }
    self.unary_tests = false;
    result
  }

  /// Reads the next token starting from current position.
  fn read_next_token(&mut self) -> Result<(TokenType, TokenValue)> {
    let chars = self.read_input();
    {
      self.token_start = self.position;
    }
    match chars {
      ['s', 'a', 't', 'i', 's', 'f', 'i', 'e', 's', ch, _, _] if ends_word(ch) => {
        self.position += 9;
        Ok((TokenType::Satisfies, TokenValue::Satisfies))
      }
      ['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', ch, _, _, _] if ends_word(ch) => {
        self.position += 8;
        Ok((TokenType::External, TokenValue::External))
      }
      ['f', 'u', 'n', 'c', 't', 'i', 'o', 'n', _, _, _, _] if self.is_function_separator(8) => {
        self.position += 8;
        Ok((TokenType::Function, TokenValue::Function))
      }
      ['a', 's', ch, _, _, _, _, _, _, _, _, _] if self.after_operand && ends_word(ch) => {
        self.position += 2;
        Ok((TokenType::As, TokenValue::As))
      }
      ['i', 'n', 's', 't', 'a', 'n', 'c', 'e', ch, _, _, _] if ends_word(ch) => {
        self.position += 8;
        Ok((TokenType::Instance, TokenValue::Instance))
      }
      ['b', 'e', 't', 'w', 'e', 'e', 'n', ch, _, _, _, _] if ends_word(ch) => {
        self.position += 7;
        Ok((TokenType::Between, TokenValue::Between))
      }
      ['c', 'o', 'n', 't', 'e', 'x', 't', _, _, _, _, _] if self.type_name && self.is_context_separator(7) => {
        self.position += 7;
        Ok((TokenType::Context, TokenValue::Context))
      }
      ['r', 'e', 't', 'u', 'r', 'n', ch, _, _, _, _, _] if ends_word(ch) => {
        self.position += 6;
        Ok((TokenType::Return, TokenValue::Return))
      }
      ['e', 'v', 'e', 'r', 'y', ch, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 5;
        Ok((TokenType::Every, TokenValue::Every))
      }
      ['f', 'a', 'l', 's', 'e', ch, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 5;
        Ok((TokenType::Boolean, TokenValue::Boolean(false)))
      }
      ['r', 'a', 'n', 'g', 'e', _h, _, _, _, _, _, _] if self.type_name && self.is_range_separator(5) => {
        self.position += 5;
        Ok((TokenType::Range, TokenValue::Range))
      }
      ['n', 'u', 'l', 'l', ch, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 4;
        Ok((TokenType::Null, TokenValue::Null))
      }
      ['e', 'l', 's', 'e', ch, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 4;
        Ok((TokenType::Else, TokenValue::Else))
      }
      ['l', 'i', 's', 't', _, _, _, _, _, _, _, _] if self.type_name && self.is_list_separator(4) => {
        self.position += 4;
        Ok((TokenType::List, TokenValue::List))
      }
      ['s', 'o', 'm', 'e', ch, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 4;
        Ok((TokenType::Some, TokenValue::Some))
      }
      ['t', 'h', 'e', 'n', ch, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 4;
        Ok((TokenType::Then, TokenValue::Then))
      }
      ['t', 'r', 'u', 'e', ch, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 4;
        Ok((TokenType::Boolean, TokenValue::Boolean(true)))
      }
      // the parser reads it as the `and` of a `between` where one is open
      ['a', 'n', 'd', ch, _, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 3;
        Ok((TokenType::And, TokenValue::And))
      }
      ['f', 'o', 'r', ch, _, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 3;
        Ok((TokenType::For, TokenValue::For))
      }
      ['n', 'o', 't', ch, _, _, _, _, _, _, _, _] if self.unary_tests && is_keyword_not_separator(ch) => {
        self.position += 3;
        Ok((TokenType::Not, TokenValue::Not))
      }
      ['i', 'f', ch, _, _, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 2;
        Ok((TokenType::If, TokenValue::If))
      }
      ['i', 'n', ch, _, _, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 2;
        Ok((TokenType::In, TokenValue::In))
      }
      ['o', 'f', ch, _, _, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 2;
        Ok((TokenType::Of, TokenValue::Of))
      }
      ['o', 'r', ch, _, _, _, _, _, _, _, _, _] if ends_word(ch) => {
        self.position += 2;
        Ok((TokenType::Or, TokenValue::Or))
      }
      // `...` is the descendant operator (DMN 1.6, grammar rule 68), checked before `..`
      ['.', '.', '.', _, _, _, _, _, _, _, _, _] => {
        self.position += 3;
        Ok((TokenType::Descendant, TokenValue::Descendant))
      }
      ['.', '.', _, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::Ellipsis, TokenValue::Ellipsis))
      }
      ['*', '*', _, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::Exp, TokenValue::Exp))
      }
      ['!', '=', _, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::Ne, TokenValue::Nq))
      }
      ['<', '=', _, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::Le, TokenValue::Le))
      }
      ['>', '=', _, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::Ge, TokenValue::Ge))
      }
      ['-', '>', _, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::RightArrow, TokenValue::RightArrow))
      }
      // every closed comment was consumed before the token, so this one is not closed
      ['/', '*', _, _, _, _, _, _, _, _, _, _] => Ok((TokenType::YyUndef, TokenValue::YyUndef)),
      ['.', ch, _, _, _, _, _, _, _, _, _, _] if is_digit(ch) => self.consume_number(),
      ['.', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Dot, TokenValue::Dot))
      }
      [',', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Comma, TokenValue::Comma))
      }
      [':', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Colon, TokenValue::Colon))
      }
      ['"', _, _, _, _, _, _, _, _, _, _, _] => self.consume_string(),
      ['+', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Plus, TokenValue::Plus))
      }
      ['-', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Minus, TokenValue::Minus))
      }
      ['*', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Mul, TokenValue::Mul))
      }
      ['/', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Div, TokenValue::Div))
      }
      ['=', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Eq, TokenValue::Eq))
      }
      ['<', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Lt, TokenValue::Lt))
      }
      ['>', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::Gt, TokenValue::Gt))
      }
      ['(', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::LeftParen, TokenValue::LeftParen))
      }
      [')', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::RightParen, TokenValue::RightParen))
      }
      ['[', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::LeftBracket, TokenValue::LeftBracket))
      }
      [']', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::RightBracket, TokenValue::RightBracket))
      }
      ['{', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::LeftBrace, TokenValue::LeftBrace))
      }
      ['}', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::RightBrace, TokenValue::RightBrace))
      }
      ['@', _, _, _, _, _, _, _, _, _, _, _] => {
        self.position += 1;
        Ok((TokenType::At, TokenValue::At))
      }
      [ch, _, _, _, _, _, _, _, _, _, _, _] if is_digit(ch) => self.consume_number(),
      [ch, _, _, _, _, _, _, _, _, _, _, _] if is_name_start_char(ch) => self.consume_name(),
      [WS, WS, WS, WS, WS, WS, WS, WS, WS, WS, WS, WS] => Ok((TokenType::YyEof, TokenValue::YyEof)),
      _ => Ok((TokenType::YyUndef, TokenValue::YyUndef)),
    }
  }

  /// Reads characters from input.
  fn read_input(&mut self) -> [char; BUF_SIZE] {
    self.consume_whitespace();
    loop {
      let consumed_comment = self.consume_comment();
      self.consume_whitespace();
      if !consumed_comment {
        break;
      }
    }
    let mut buffer: [char; BUF_SIZE] = [WS; BUF_SIZE];
    // only the keywords look beyond the first three characters
    let len = if self.char_at(0).is_some_and(is_keyword_initial) { BUF_SIZE } else { 3 };
    for (value, ch) in buffer[..len].iter_mut().zip(self.input[self.position..].chars()) {
      if !is_whitespace(ch) {
        *value = ch
      };
    }
    buffer
  }

  /// Consumes all whitespace characters starting from the current position.
  /// After consuming a whitespace character the current position is advanced.
  fn consume_whitespace(&mut self) {
    while let Some(ch) = self.char_at(0) {
      if is_whitespace(ch) {
        self.position += ch.len_utf8();
      } else {
        break;
      }
    }
  }

  /// Consumes single comment starting from the current position.
  /// After consuming a comment, the current position is advanced
  /// to the next character after comment. Returns `true` when more
  /// white space or another comment can follow.
  fn consume_comment(&mut self) -> bool {
    let pair = (self.char_at(0), self.char_at(1));
    match pair {
      (Some('/'), Some('/')) => {
        self.position += 2;
        while let Some(ch) = self.char_at(0) {
          if ch == '\n' {
            return true;
          }
          self.position += ch.len_utf8();
        }
      }
      (Some('/'), Some('*')) => {
        // a comment that the input does not close is no comment: the position stays at its
        // `/*`, which the next token reports
        if let Some(len) = self.input[self.position + 2..].find("*/") {
          self.position += 2 + len + 2;
          return true;
        }
      }
      _ => {}
    }
    false
  }

  /// Consumes the string literal.
  fn consume_string(&mut self) -> Result<(TokenType, TokenValue)> {
    self.position += 1;
    // a literal without escapes and vertical space is its characters up to the closing quote:
    // taken in one allocation of the exact size
    let rest = &self.input[self.position..];
    if let Some(len) = rest.find(|ch| matches!(ch, '"' | '\\') || is_vertical_space(ch))
      && rest[len..].starts_with('"')
    {
      self.position += len + 1;
      return Ok((TokenType::String, TokenValue::String(rest[..len].to_string())));
    }
    let mut string = "".to_string();
    loop {
      let first = self.char_at(0);
      let second = self.char_at(1);
      match (first, second) {
        (Some('\\'), Some('\'')) => {
          string.push('\'');
          self.position += 2;
        }
        (Some('\\'), Some('"')) => {
          string.push('"');
          self.position += 2;
        }
        (Some('\\'), Some('\\')) => {
          string.push('\\');
          self.position += 2;
        }
        (Some('\\'), Some('n')) => {
          string.push('\n');
          self.position += 2;
        }
        (Some('\\'), Some('r')) => {
          string.push('\r');
          self.position += 2;
        }
        (Some('\\'), Some('t')) => {
          string.push('\t');
          self.position += 2;
        }
        (Some('\\'), Some('u')) => {
          string.push(self.consume_unicode()?);
        }
        (Some('\\'), Some('U')) => {
          string.push(self.consume_unicode()?);
        }
        (Some('"'), _) => {
          self.position += 1;
          break;
        }
        (Some(ch), _) if is_vertical_space(ch) => {
          return Ok((TokenType::YyUndef, TokenValue::YyUndef));
        }
        (Some(ch), _) => {
          string.push(ch);
          self.position += ch.len_utf8();
        }
        // the input ends inside the literal
        _ => return Ok((TokenType::YyUndef, TokenValue::YyUndef)),
      }
    }
    Ok((TokenType::String, TokenValue::String(string)))
  }

  /// Consumes a number available on input starting from the current position.
  /// When the number is consumed, the current position is incremented accordingly.
  fn consume_number(&mut self) -> Result<(TokenType, TokenValue)> {
    // storage for digits before decimal separator
    let mut digits_before = String::new();
    // storage for digits after decimal separator
    let mut digits_after = String::new();
    // storage for the sign of the exponent ('+' is the default)
    let mut exponent_sign: char = '+';
    // storage for digits of the exponent
    let mut digits_exponent = String::new();
    //------------------------------------------------------------------------------
    // Parse number using a state machine.
    //------------------------------------------------------------------------------
    let mut state = 1;
    while let Some(ch) = self.char_at(0) {
      match state {
        1 => match ch {
          '0'..='9' => {
            // consume a digit before decimal separator
            self.reserve_digits(&mut digits_before);
            digits_before.push(ch);
            self.position += 1;
          }
          DECIMAL_SEPARATOR if self.is_digit_at(1) => {
            // consume decimal separator
            self.position += 1;
            state = 2
          }
          'e' | 'E' if self.exponent_follows() => {
            // consume exponent letter
            self.position += 1;
            state = 3
          }
          _ => break,
        },
        2 => match ch {
          '0'..='9' => {
            // consume a digit after decimal separator
            self.reserve_digits(&mut digits_after);
            digits_after.push(ch);
            self.position += 1;
          }
          'e' | 'E' if self.exponent_follows() => {
            // consume exponent letter
            self.position += 1;
            state = 3
          }
          _ => break,
        },
        3 => match ch {
          '+' => {
            // consume plus - this is the default exponent sign
            self.position += 1;
            state = 4;
          }
          '-' => {
            // consume minus - change the exponent sign
            exponent_sign = '-';
            self.position += 1;
            state = 4;
          }
          '0'..='9' => {
            // consume the first digit of the exponent
            self.reserve_digits(&mut digits_exponent);
            digits_exponent.push(ch);
            self.position += 1;
            state = 4;
          }
          _ => break,
        },
        4 => match ch {
          '0'..='9' => {
            // consume digits of the exponent
            self.reserve_digits(&mut digits_exponent);
            digits_exponent.push(ch);
            self.position += 1;
            state = 4;
          }
          _ => break,
        },
        _ => break,
      }
    }
    if digits_before.is_empty() {
      digits_before.push('0');
    }
    Ok((TokenType::Numeric, TokenValue::Numeric(digits_before, digits_after, exponent_sign, digits_exponent)))
  }

  /// Returns `true` when the digits of an exponent follow the exponent letter at the position,
  /// after an optional sign: only then the letter belongs to the number (DMN 1.6 grammar rule
  /// 35). Otherwise the number ends before the letter.
  fn exponent_follows(&self) -> bool {
    match self.char_at(1) {
      Some('+' | '-') => self.is_digit_at(2),
      Some(ch) => is_digit(ch),
      None => false,
    }
  }

  /// Consumes a name.
  fn consume_name(&mut self) -> Result<(TokenType, TokenValue)> {
    //------------------------------------------------------------------------------
    // The name parts that follow the current position: the parts of the chain scanned
    // for an earlier name token when this token starts at one of its parts (the scan
    // reads only the input, so it would give the same parts), or a new scan.
    //------------------------------------------------------------------------------
    let first = match self.chain_parts.binary_search_by_key(&self.position, |part| part.start) {
      Ok(index) => index,
      Err(_) => {
        self.scan_chain();
        0
      }
    };
    // the position after the whole chain, as after the scan
    self.position = self.chain_end;

    //==============================================================================
    // Now the parts `first..` of the chain are all parts of the longest possible name.
    // There is at least one part. Now decide, what kind of name it is.
    //==============================================================================

    //-------------------------------------------------------------------------------
    // Tweak with the name in `for` and `quantified` expressions.
    // The variable's name is the name until the keyword `in` appears.
    // Return the whole name before the `in` keyword.
    // A name start is never a literal terminal symbol (DMN 1.6 §10.3.1.4), so a variable
    // can not start with `in`: then `in` is the keyword (and the variable name is missing).
    //-------------------------------------------------------------------------------
    if self.till_in
      && let Some(part) = self.next_in_part(first)
    {
      self.till_in = false;
      if part == first {
        self.position = self.chain_parts[first].end;
        return Ok((TokenType::In, TokenValue::In));
      }
      self.position = self.chain_parts[part - 1].end;
      return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, part))));
    }

    //-------------------------------------------------------------------------------
    // Tweak with the context keys or named parameters.
    // In such cases, the name is followed by a colon.
    // Return the whole name before the `:`.
    //-------------------------------------------------------------------------------
    if self.is_next_character(&[':'], 0) {
      return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, self.chain_parts.len()))));
    }

    //-------------------------------------------------------------------------------
    // A name of one part is the part, whether it is in scope or not, so it needs no
    // lookup: except a built-in property name after a dot, and the names that the two
    // tweaks after the lookup turn into other tokens when they are not in scope (a new
    // tweak there must be excluded here too). The token type and value do not depend on
    // the lookup, only the position after the token does: a name found by the lookup
    // ends at the end of its part, this one after the white space that follows the part,
    // which the next read skips anyway.
    //-------------------------------------------------------------------------------
    if self.chain_parts.len() - first == 1 {
      let word = &self.input[self.chain_parts[first].trimmed.clone()];
      if self.after_dot && is_built_in_property_name(word) {
        self.after_dot = false;
        self.position = self.chain_parts[first].end;
        return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, first + 1))));
      }
      let decided_by_scope = (self.type_name && is_built_in_type_name(word)) || (is_built_in_date_time_function_name(word) && self.is_next_character(&['('], 0));
      if !decided_by_scope {
        return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, first + 1))));
      }
    }

    //-------------------------------------------------------------------------------
    // begin with with the longest name containing all parts
    //-------------------------------------------------------------------------------
    if let Some((part_count, built_in_property)) = self.longest_name_in_scope(first) {
      if built_in_property {
        self.after_dot = false;
      } else if let Some((part_count, token_type)) = self.longer_built_in_name(first, part_count) {
        // a built-in name that starts here is longer than the name in scope
        self.position = self.chain_parts[first + part_count - 1].end;
        let name = self.chain_name(first, first + part_count);
        return Ok(match token_type {
          TokenType::BuiltInTypeName => (token_type, TokenValue::BuiltInTypeName(name)),
          TokenType::NameDateTime => (token_type, TokenValue::NameDateTime(name)),
          _ => (token_type, TokenValue::Name(name)),
        });
      }
      // return to the input all characters that do not belong to the name that was found
      self.position = self.chain_parts[first + part_count - 1].end;
      // return the name that exists in the current context
      return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, first + part_count))));
    }

    //------------------------------------------------------------------------------
    // No name in scope starts here. Build the name from the name parts up to the first
    // word that is a keyword which can follow an operand, or the first symbol that is an
    // operator: the rest of the chain is read as the following tokens, as it is after a
    // name in scope.
    //------------------------------------------------------------------------------
    let mut end = if self.whole_names { self.chain_parts.len() } else { self.unknown_name_end(first) };
    // A name in scope that is written with symbols side by side (`fr**n*s`) is not found by its
    // flattened text, which keeps a space between them: where the parts are of this kind, the
    // longest run of them whose name is in scope is the name.
    if end < self.chain_parts.len()
      && self.has_irregular_parts(first)
      && let Some(longer) = (end + 1..=self.chain_parts.len())
        .rev()
        .find(|&to| self.scope.contains(self.chain_name(first, to).as_str()))
    {
      end = longer;
    }
    if end < self.chain_parts.len() {
      self.position = self.chain_parts[end - 1].end;
    }
    let name = self.chain_name(first, end);

    //------------------------------------------------------------------------------
    // Tweak with the names of built-in types.
    // If the name is a built-in type name,
    // then return a type name instead of regular name.
    //------------------------------------------------------------------------------
    if self.type_name && is_built_in_type_name(name.as_str()) {
      self.type_name = false;
      return Ok((TokenType::BuiltInTypeName, TokenValue::BuiltInTypeName(name)));
    }

    //------------------------------------------------------------------------------
    // Tweak with `date`, `time`, `date and time` and `duration` literals.
    // When these names are encountered and are followed by `(`,
    // then treat them as names of temporal functions (date literals).
    //------------------------------------------------------------------------------
    if is_built_in_date_time_function_name(name.as_str()) && self.is_next_character(&['('], 0) {
      return Ok((TokenType::NameDateTime, TokenValue::NameDateTime(name)));
    }

    //------------------------------------------------------------------------------
    // Return the name as it appears on input.
    //------------------------------------------------------------------------------
    Ok((TokenType::Name, TokenValue::Name(name)))
  }

  /// Scans the name chain that starts at the current position (a `name start` character) into
  /// the chain parts, and moves the position to the first character after the chain.
  fn scan_chain(&mut self) {
    self.chain_parts.clear();
    self.chain_last_empty = None;
    self.chain_in = NOT_SEARCHED;
    self.chain_flat_state = FlatChain::Unknown;
    self.chain_last_irregular = None;
    let bytes = self.input.as_bytes();
    //------------------------------------------------------------------------------
    // The current character on input is already a `name start` character, the first
    // character of the first part. `position` is the last character read; the state
    // machine looks at the character after it.
    //------------------------------------------------------------------------------
    let mut position = self.position;
    let mut start = position;
    let mut state = 1;
    loop {
      let next = position + utf8_len(bytes[position]);
      let ch = match bytes.get(next) {
        Some(&byte) if byte.is_ascii() => Some(byte as char),
        Some(_) => self.input[next..].chars().next(),
        None => None,
      };
      match state {
        // in a word
        1 => {
          if ch.is_some_and(is_name_part_char) {
            position = next;
          } else {
            self.push_chain_part(start, next);
            state = 2;
          }
        }
        // after a part
        2 => {
          if ch.is_some_and(is_name_part_char) {
            // the next part starts with the next character
            start = next;
            state = 1;
          } else if ch.is_some_and(is_additional_name_symbol) {
            state = 4;
          } else if ch.is_some_and(is_whitespace) {
            state = 5;
          } else {
            position = next;
            break;
          }
        }
        // additional name symbols, each a part
        4 => {
          if ch.is_some_and(is_additional_name_symbol) {
            position = next;
            self.push_chain_part(position, position + 1);
          } else {
            state = 2;
          }
        }
        // white space
        _ => {
          if ch.is_some_and(is_whitespace) {
            position = next;
          } else {
            state = 2;
          }
        }
      }
    }
    self.position = position;
    self.chain_end = position;
  }

  /// Returns the index of the first part `in` of the chain at or after the part `first`. The
  /// last search is remembered, so the name tokens of a chain compare each of its parts once.
  fn next_in_part(&mut self, first: usize) -> Option<usize> {
    let (from, found) = self.chain_in;
    if !(from..=found).contains(&first) {
      let found = (first..self.chain_parts.len())
        .find(|&part| {
          work!(InParts, 1);
          self.part_text(part) == "in"
        })
        .unwrap_or(self.chain_parts.len());
      self.chain_in = (first, found);
    }
    Some(self.chain_in.1).filter(|&part| part < self.chain_parts.len())
  }

  /// Adds the part `start..end` of the input to the chain.
  fn push_chain_part(&mut self, start: usize, end: usize) {
    work!(Parts, 1);
    let text = &self.input[start..end];
    // only a part with other characters than ASCII can have white space at an end (U+1680 is
    // a name part character too)
    let trimmed = if text.is_ascii() {
      start..end
    } else {
      let trim_start = start + (text.len() - text.trim_start_matches(is_whitespace).len());
      trim_start..trim_start + text.trim_matches(is_whitespace).len()
    };
    if trimmed.is_empty() {
      self.chain_last_empty = Some(self.chain_parts.len());
    }
    self.chain_parts.push(ChainPart { start, end, trimmed });
  }

  /// Returns the text of a part of the chain.
  fn part_text(&self, part: usize) -> &str {
    let part = &self.chain_parts[part];
    &self.input[part.start..part.end]
  }

  /// Returns the name made of the parts `from..to` of the chain.
  fn chain_name(&self, from: usize, to: usize) -> Name {
    Name::from_parts(self.chain_parts[from..to].iter().map(|part| &self.input[part.trimmed.clone()]))
  }

  /// Returns the index after the last part of the name that starts at part `first` of the chain
  /// when no name in scope starts there: the index of the first later part that ends such a name
  /// ([Lexer::ends_unknown_name]), or the length of the chain. The words of one of the
  /// [BUILT_IN_NAMES_WITH_KEYWORDS] at the start of the name are skipped.
  fn unknown_name_end(&self, first: usize) -> usize {
    (first + self.built_in_words(first)..self.chain_parts.len())
      .find(|&index| self.ends_unknown_name(index))
      .unwrap_or(self.chain_parts.len())
  }

  /// Returns `true` when the parts of the chain from `first` on are not all words and single
  /// symbols between them: a symbol follows a symbol, or a part is empty or has white space at
  /// an end. The last such part of the chain is found once.
  fn has_irregular_parts(&mut self, first: usize) -> bool {
    let last = *self.chain_last_irregular.get_or_insert_with(|| {
      let mut after_symbol = false;
      let mut last = None;
      for (index, part) in self.chain_parts.iter().enumerate() {
        let text = &self.input[part.trimmed.clone()];
        let symbol = is_symbol_part(text);
        if text.is_empty() || part.trimmed != (part.start..part.end) || (symbol && after_symbol) {
          last = Some(index);
        }
        after_symbol = symbol;
      }
      last
    });
    // a name starts at a word, so a symbol at `first` is no part of it
    last.is_some_and(|last| last > first)
  }

  /// Returns `true` when the part `index` of the chain ends a name that is not in scope before
  /// it: one of the [NAME_ENDING_KEYWORDS] as a word of its own, or one of the
  /// [NAME_ENDING_SYMBOLS].
  fn ends_unknown_name(&self, index: usize) -> bool {
    NAME_ENDING_SYMBOLS.contains(&&self.input[self.chain_parts[index].trimmed.clone()]) || self.is_name_ending_keyword(index)
  }

  /// Returns the number of parts from part `first` of the chain that are no keywords whatever
  /// they are: the words of one of the [BUILT_IN_NAMES_WITH_KEYWORDS] when the parts start with
  /// them, otherwise the first part.
  fn built_in_words(&self, first: usize) -> usize {
    let parts = &self.chain_parts[first..];
    BUILT_IN_NAMES_WITH_KEYWORDS
      .iter()
      .find(|words| parts.len() >= words.len() && words.iter().zip(parts).all(|(word, part)| &self.input[part.trimmed.clone()] == *word))
      .map_or(1, |words| words.len())
  }

  /// Returns `true` when the part `index` of the chain is one of the [NAME_ENDING_KEYWORDS] as a
  /// word of its own: white space before it, and white space or the end of the input after it,
  /// as where the keyword starts a token.
  fn is_name_ending_keyword(&self, index: usize) -> bool {
    let start = self.chain_parts[index].start;
    self.input[..start].chars().next_back().is_some_and(is_whitespace)
      && NAME_ENDING_KEYWORDS.iter().any(|keyword| {
        self.input[start..]
          .strip_prefix(keyword)
          .is_some_and(|rest| rest.chars().next().is_none_or(is_whitespace) && (*keyword != "as" || (!self.declared_name && !self.type_name && type_ends_name(rest))))
      })
  }

  /// Returns `true` when the name that starts at part `first` of the chain is this part alone
  /// when no name in scope starts there: a keyword or a symbol that ends such a name follows
  /// it. A name in scope of this one part is then the same token, so the scope need not be
  /// asked for it.
  fn is_one_part_before_keyword(&mut self, first: usize) -> bool {
    // parts that are not regular may be a longer name in scope that its flattened text misses
    !self.whole_names && first + 1 < self.chain_parts.len() && self.built_in_words(first) == 1 && self.ends_unknown_name(first + 1) && !self.has_irregular_parts(first)
  }

  /// Returns the number of leading name parts (from part `first` of the chain) that form a
  /// built-in name of more parts than `in_scope`, the parts of the name in scope that starts
  /// there, and the type of its token: the name of a built-in function, or where a type is
  /// expected the name of a built-in type, which also wins with as many parts (a variable is
  /// no type). A function name with a keyword in it (`date and time`) counts only before the
  /// `(` of its call: elsewhere the keyword is the operator after the name in scope.
  ///
  /// The built-in names are words with one space between them, of at most
  /// [MAX_BUILT_IN_NAME_WORDS] words, so a few prefixes of the parts are looked at, and none
  /// when the part after the name in scope is not a word.
  fn longer_built_in_name(&mut self, first: usize, in_scope: usize) -> Option<(usize, TokenType)> {
    let parts = self.chain_parts.len() - first;
    let word = |lexer: &Self, index: usize| {
      let part = &lexer.chain_parts[first + index];
      let text = &lexer.input[part.trimmed.clone()];
      !text.is_empty() && !is_symbol_part(text)
    };
    let longer = in_scope < parts && word(self, in_scope);
    if !longer && !self.type_name {
      return None;
    }
    let most = if longer { parts.min(MAX_BUILT_IN_NAME_WORDS) } else { in_scope };
    let least = if self.type_name { in_scope } else { in_scope + 1 };
    for part_count in (least..=most).rev() {
      if !(0..part_count).all(|index| word(self, index)) {
        continue;
      }
      let texts = self.chain_parts[first..first + part_count].iter().map(|part| &self.input[part.trimmed.clone()]);
      flatten_name_parts_into(texts, &mut self.name_buf, &mut self.name_tmp);
      if self.type_name && is_built_in_type_name(&self.name_buf) {
        return Some((part_count, TokenType::BuiltInTypeName));
      }
      if part_count > in_scope && is_built_in_function_name(&self.name_buf) {
        // the character after the name: the position is the end of the chain now
        let end_of_chain = std::mem::replace(&mut self.position, self.chain_parts[first + part_count - 1].end);
        let called = self.is_next_character(&['('], 0);
        self.position = end_of_chain;
        let keyword = (1..part_count).any(|index| NAME_ENDING_KEYWORDS.contains(&&self.input[self.chain_parts[first + index].trimmed.clone()]));
        if keyword && !called {
          continue;
        }
        let date_time = called && is_built_in_date_time_function_name(&self.name_buf);
        return Some((part_count, if date_time { TokenType::NameDateTime } else { TokenType::Name }));
      }
    }
    None
  }

  /// Returns the number of leading name parts (from part `first` of the chain) that form the
  /// longest name in scope (a key in the parsing scope, or a built-in property name right after
  /// a dot), and whether it is a built-in property name. The candidates are the flattened
  /// prefixes of the parts, longest first.
  fn longest_name_in_scope(&mut self, first: usize) -> Option<(usize, bool)> {
    // A prefix can only match a name in scope (or a built-in property name) when its flattened
    // text is not longer than the longest such name, and the flattened text is never shorter
    // than the trimmed parts it is made of. Skip the prefixes that are too long to match, so
    // that a long chain like `a.a.a...` (every `.` is an additional name symbol, so the longest
    // candidate name spans the whole chain) costs linear instead of quadratic time per token.
    // The candidates contain the composite separator `" . "` only when one of their parts is
    // empty after trimming, so the composite keys count only then.
    let has_empty_part = self.chain_last_empty.is_some_and(|last| last >= first);
    let parts = &self.chain_parts[first..];
    let mut bound = self.scope.bound(has_empty_part);
    if self.after_dot {
      bound = bound.max(MAX_BUILT_IN_PROPERTY_NAME_LEN);
    }
    let mut part_count = admitted_parts(parts, bound);
    // Without an empty part every candidate starts with the first part, followed by nothing, a
    // space or a symbol (no step of the flattening touches it), so it can only be a name with this
    // first word: the bound of these names is tighter, and worth looking up for two or more candidates.
    if part_count > 1 && !has_empty_part {
      let word = &self.input[parts[0].trimmed.clone()];
      let limit = parts[..part_count].iter().map(|part| part.trimmed.len()).sum();
      let mut group_bound = self.scope.group_bound(word, limit);
      if self.after_dot && group_bound < limit {
        group_bound = group_bound.max(property_group_bound(word));
      }
      part_count = part_count.min(admitted_parts(parts, group_bound));
    }
    let regular = part_count > 1 && !has_empty_part && self.flat_chain(first);
    while part_count > 0 {
      // The last candidate, the first part alone, is the token whether it is a name in scope or
      // not when a keyword ends the name after it (as the name of a chain of one part is), so
      // it is not looked up: except a built-in property name after a dot, and a built-in type
      // name where a type is read, which is another token when it is no name in scope.
      if part_count == 1 && !has_empty_part {
        let word = &self.input[self.chain_parts[first].trimmed.clone()];
        if self.after_dot && is_built_in_property_name(word) {
          return Some((1, true));
        }
        if !(self.type_name && is_built_in_type_name(word)) && self.is_one_part_before_keyword(first) {
          return Some((1, false));
        }
      }
      if regular {
        // The candidate is a piece of the flattened chain, and its length is known before it is
        // written: a candidate of a length that no name in scope has is not looked up, so a
        // long chain costs a step per candidate, not a text.
        let (start, last) = (self.chain_flat_parts[first].0, first + part_count - 1);
        let symbol = is_symbol_part(&self.input[self.chain_parts[last].trimmed.clone()]);
        // a symbol at the end of a name stands after a space: no part follows it
        let end = if symbol { self.chain_flat_parts[last - 1].1 } else { self.chain_flat_parts[last].1 };
        let len = end - start + if symbol { 2 } else { 0 };
        let possible = self.scope.may_contain_length(len) || (self.after_dot && BUILT_IN_PROPERTY_NAMES.iter().any(|name| name.len() == len));
        if !possible {
          part_count -= 1;
          continue;
        }
        self.name_buf.clear();
        self.name_buf.push_str(&self.chain_flat[start..end]);
        if symbol {
          self.name_buf.push(WS);
          self.name_buf.push_str(&self.input[self.chain_parts[last].trimmed.clone()]);
        }
        #[cfg(test)]
        {
          // the piece is the flattened name of these parts
          let texts = self.chain_parts[first..first + part_count].iter().map(|part| &self.input[part.trimmed.clone()]);
          let mut flattened = String::new();
          flatten_name_parts_into(texts, &mut flattened, &mut self.name_tmp);
          assert_eq!(flattened, self.name_buf, "the flattened chain {:?}", self.chain_flat);
        }
      } else {
        // flatten the name parts to compare it with built-in names and keys in current scope
        let texts = self.chain_parts[first..first + part_count].iter().map(|part| &self.input[part.trimmed.clone()]);
        flatten_name_parts_into(texts, &mut self.name_buf, &mut self.name_tmp);
      }
      work!(Candidates, 1);
      // check if the name is a built-in property name
      if self.after_dot && is_built_in_property_name(&self.name_buf) {
        return Some((part_count, true));
      }
      // check if the flattened name exists as a key in the current scope; it can be a composite
      // key only with an empty part
      let found = if has_empty_part {
        self.scope.contains(&self.name_buf)
      } else {
        self.scope.contains_plain(&self.name_buf)
      };
      if found {
        return Some((part_count, false));
      }
      part_count -= 1;
    }
    None
  }

  /// Writes the flattened text of the whole chain once, when the chain is regular, and returns
  /// whether the flattened name of parts from `first` on is a piece of it.
  ///
  /// A chain is regular when its parts are words and single additional name symbols, no part
  /// has white space at an end or is empty, the first part is a word and no symbol follows a
  /// symbol. The flattened name of some of its parts is then their texts, with a space between
  /// two words and none around a symbol that parts stand on both sides of: the spaces around
  /// such a symbol are the ones the flattening removes, and no other text of a part has a space.
  fn flat_chain(&mut self, first: usize) -> bool {
    if self.chain_flat_state == FlatChain::Unknown {
      self.chain_flat.clear();
      self.chain_flat_parts.clear();
      let mut regular = true;
      let mut after_symbol = true;
      for (index, part) in self.chain_parts.iter().enumerate() {
        let text = &self.input[part.trimmed.clone()];
        let symbol = is_symbol_part(text);
        if text.is_empty() || part.trimmed != (part.start..part.end) || (symbol && after_symbol) {
          regular = false;
          break;
        }
        if index > 0 && !symbol && !after_symbol {
          self.chain_flat.push(WS);
        }
        let start = self.chain_flat.len();
        self.chain_flat.push_str(text);
        self.chain_flat_parts.push((start, self.chain_flat.len()));
        after_symbol = symbol;
      }
      self.chain_flat_state = if regular { FlatChain::Written } else { FlatChain::NotRegular };
    }
    // a name starts at a word
    self.chain_flat_state == FlatChain::Written && !is_symbol_part(&self.input[self.chain_parts[first].trimmed.clone()])
  }

  /// Consumes a HEX digit from input or reports an error.
  fn consume_hex_digit(&mut self) -> Result<u64> {
    if let Some(ch) = self.char_at(0) {
      if ch.is_ascii_hexdigit() {
        self.position += 1;
        hex_to_decimal(ch)
      } else {
        Err(err_expected_hex_digit(ch))
      }
    } else {
      Err(err_unexpected_eof())
    }
  }

  /// Consumes Unicode literal in one of the following forms:
  /// - \u0000 ('\' + 'u' + four hexadecimal characters), or
  /// - \U000000 ('\' + 'U' + six hexadecimal characters).
  fn consume_unicode_literal(&mut self) -> Result<u64> {
    // A Unicode literal must start with '\u' or '\U'; report a lexer error instead of panicking
    // when it does not (e.g. a high surrogate '\uD800' that is not followed by a low surrogate).
    let u = match (self.char_at(0), self.char_at(1)) {
      (Some('\\'), Some(u @ ('u' | 'U'))) => u,
      (Some('\\'), Some(other)) => return Err(err_expected_unicode_escape(Some(other))),
      (Some(other), _) => return Err(err_expected_unicode_escape(Some(other))),
      _ => return Err(err_unexpected_eof()),
    };
    self.position += 2;
    let mut value = 0_u64;
    if u == 'U' {
      value = 1048576 * self.consume_hex_digit()?;
      value += 65536 * self.consume_hex_digit()?;
    }
    value += 4096 * self.consume_hex_digit()?;
    value += 256 * self.consume_hex_digit()?;
    value += 16 * self.consume_hex_digit()?;
    value += self.consume_hex_digit()?;
    Ok(value)
  }

  /// Consumes the UTF-8 encoded character given in one of the following form:
  /// - \u0000 ('\' + 'u' + four hexadecimal characters), or
  /// - \U000000 ('\' + 'U' + six hexadecimal characters).
  ///
  /// A UTF-16 surrogate pair (\uD83D\uDE40) is combined into one character.
  fn consume_unicode(&mut self) -> Result<char> {
    let value = self.consume_unicode_literal()?;
    match value {
      // value is the high surrogate of UTF-16, it must be followed by a low surrogate
      0xD800..=0xDBFF => {
        let low_surrogate = self.consume_unicode_literal()?;
        match low_surrogate {
          0xDC00..=0xDFFF => {
            let code_point = 0x10000 + ((value - 0xD800) * 0x400) + (low_surrogate - 0xDC00);
            u32::try_from(code_point)
              .ok()
              .and_then(char::from_u32)
              .ok_or_else(|| err_unicode_value_out_of_range(code_point))
          }
          other => Err(err_unicode_surrogate_out_of_range(other)),
        }
      }
      // a lone low surrogate is not a character
      0xDC00..=0xDFFF => Err(err_unicode_surrogate_out_of_range(value)),
      other => u32::try_from(other).ok().and_then(char::from_u32).ok_or_else(|| err_unicode_value_out_of_range(other)),
    }
  }

  /// Returns the character at the current position advanced with specified offset (in characters).
  fn char_at(&self, offset: usize) -> Option<char> {
    let bytes = self.input.as_bytes();
    let mut position = self.position;
    for _ in 0..offset {
      position += utf8_len(*bytes.get(position)?);
    }
    match *bytes.get(position)? {
      byte if byte.is_ascii() => Some(byte as char),
      _ => self.input[position..].chars().next(),
    }
  }

  /// Reserves the exact space for the run of digits that starts at the current position, before
  /// its first digit is pushed to `digits`.
  fn reserve_digits(&self, digits: &mut String) {
    if digits.is_empty() {
      digits.reserve_exact(self.input[self.position..].bytes().take_while(u8::is_ascii_digit).count());
    }
  }

  /// Returns `true` when the character at the current
  /// position advanced by the offset is a digit.
  fn is_digit_at(&self, offset: usize) -> bool {
    if let Some(ch) = self.char_at(offset) { is_digit(ch) } else { false }
  }

  /// Returns `true` when the specified character is a function keyword separator,
  /// that is a character that is allowed after keyword `function`.
  fn is_function_separator(&self, offset: usize) -> bool {
    self.is_next_character(&['(', '<'], offset)
  }

  /// Returns `true` when the specified character is a context keyword separator,
  /// that is a character that is allowed after keyword `context`.
  fn is_context_separator(&self, offset: usize) -> bool {
    self.is_next_character(&['<'], offset)
  }

  /// Returns `true` when the specified character is a range keyword separator,
  /// that is a character that is allowed after keyword `range`.
  fn is_range_separator(&self, offset: usize) -> bool {
    self.is_next_character(&['<'], offset)
  }

  /// Returns `true` when the next character on input is a `list` keyword separator,
  /// that is, a character that is allowed after keyword `list`.
  /// Allowed characters after keyword `list` are whitespace and `<`.
  fn is_list_separator(&self, offset: usize) -> bool {
    self.is_next_character(&['<'], offset)
  }

  /// Returns `true` when the next character on input is a character
  /// from specified `chars` list.
  fn is_next_character(&self, chars: &[char], offset: usize) -> bool {
    for ch in self.input[self.position..].chars().skip(offset) {
      if chars.contains(&ch) {
        return true;
      } else if !is_whitespace(ch) {
        return false;
      }
    }
    false
  }
}

/// The keys of the context literal whose `{` is at or after `open` that are names of several tokens, found
/// before its entries are parsed: an entry can read an entry written after it (DMN 1.6 clause
/// 10.3.2.11.1), and a name of several tokens is read as one name only when it is in scope.
///
/// One pass over the text of the literal: a key is the text between the start of an entry (after
/// the `{`, or after a `,` of the literal itself) and its `:`. Strings, comments and what stands
/// in brackets are passed over. The commas of a `context<...>` or `function<...>` type are not
/// told from those of the literal, so a type can give a name that is no key; such a name only
/// adds a candidate.
pub(crate) fn literal_keys(input: &str, open: usize) -> Vec<Name> {
  let bytes = input.as_bytes();
  let mut keys = vec![];
  // white space can stand between `open` and the brace
  let Some(brace) = bytes.get(open..).and_then(|rest| rest.iter().position(|&byte| byte == b'{')) else {
    return keys;
  };
  let (mut braces, mut brackets) = (1usize, 0isize);
  let mut position = open + brace + 1;
  // the start of the entry whose key is not read yet
  let mut entry = Some(position);
  while position < bytes.len() {
    match bytes[position] {
      b'"' => {
        position += 1;
        while position < bytes.len() && bytes[position] != b'"' {
          position += if bytes[position] == b'\\' { 2 } else { 1 };
        }
      }
      b'/' if matches!(bytes.get(position + 1), Some(b'/' | b'*')) => {
        let comment = position;
        if bytes[position + 1] == b'/' {
          while position < bytes.len() && bytes[position] != b'\n' {
            position += 1;
          }
        } else {
          position += 2;
          while position < bytes.len() && !(bytes[position] == b'*' && bytes.get(position + 1) == Some(&b'/')) {
            position += 1;
          }
          position += 1;
        }
        // a comment before the key is no part of it
        if let Some(start) = entry
          && input[start..comment].chars().all(is_whitespace)
        {
          entry = Some((position + 1).min(bytes.len()));
        }
      }
      b'{' => braces += 1,
      b'}' => {
        braces -= 1;
        if braces == 0 {
          break;
        }
      }
      b'(' | b'[' => brackets += 1,
      b')' | b']' => brackets -= 1,
      b',' if braces == 1 && brackets == 0 => entry = Some(position + 1),
      b':' if braces == 1 && brackets == 0 => {
        if let Some(start) = entry.take() {
          let key = input[start..position].trim_matches(is_whitespace);
          // a key that is a string without an escape is read by the name its text is
          let key = match key.strip_prefix('"').and_then(|key| key.strip_suffix('"')) {
            Some(text) if !text.contains(['"', '\\']) => text,
            _ => key,
          };
          if let Some(name) = key_of_several_tokens(key) {
            keys.push(name);
          }
        }
      }
      _ => {}
    }
    position += 1;
  }
  keys
}

/// The name that `text` is when it is a name of several tokens: words and additional name
/// symbols, with white space between them or not.
fn key_of_several_tokens(text: &str) -> Option<Name> {
  let mut parts: Vec<&str> = vec![];
  let mut word: Option<usize> = None;
  for (at, ch) in text.char_indices() {
    if is_whitespace(ch) || is_additional_name_symbol(ch) {
      if let Some(start) = word.take() {
        parts.push(&text[start..at]);
      }
      if !is_whitespace(ch) {
        parts.push(&text[at..at + ch.len_utf8()]);
      }
    } else if is_name_part_char(ch) {
      if word.is_none() {
        if !parts.is_empty() || is_name_start_char(ch) {
          word = Some(at);
        } else {
          return None;
        }
      }
    } else {
      return None;
    }
  }
  if let Some(start) = word {
    parts.push(&text[start..]);
  }
  (parts.len() > 1 && parts.first().is_some_and(|part| part.chars().next().is_some_and(is_name_start_char))).then(|| Name::from_parts(parts))
}

/// Returns `true` when a word ends before the specified character: a word that is a keyword is
/// the keyword whatever follows it (DMN 1.6 clause 10.3.1.4: a name start is never a literal
/// terminal), and a longer word (`iffy`, `in1`) is none.
fn ends_word(ch: char) -> bool {
  !is_name_part_char(ch)
}

/// Returns the length in bytes of the UTF-8 encoded character that starts with `byte`.
fn utf8_len(byte: u8) -> usize {
  match byte {
    0x00..0x80 => 1,
    0xC0..0xE0 => 2,
    0xE0..0xF0 => 3,
    _ => 4,
  }
}

/// Returns `true` when the specified character is the first character of a keyword.
fn is_keyword_initial(ch: char) -> bool {
  matches!(ch, 'a' | 'b' | 'c' | 'e' | 'f' | 'i' | 'l' | 'n' | 'o' | 'r' | 's' | 't')
}

/// Returns **true** when the specified character is an ASCII digit.
fn is_digit(ch: char) -> bool {
  ch.is_ascii_digit()
}

/// Returns `true` when the specified character is a keyword `not` separator,
/// that is a character that is allowed after keyword `not` at the beginning of unary tests.
fn is_keyword_not_separator(ch: char) -> bool {
  matches!(ch, WS | '(')
}

/// Returns `true` when the specified character is an additional name symbol.
/// Specification: 10.3.1.2 Grammar rule, p.120, grammar rule 30.
fn is_additional_name_symbol(ch: char) -> bool {
  matches!(ch, '.' | '/' | '-' | '\'' | '+' | '*')
}

/// Returns `true` when the specified character is name start character.
fn is_name_start_char(ch: char) -> bool {
  matches!(ch, '?' | 'A'..='Z' | '_' | 'a'..='z' |
               '\u{00C0}'..='\u{00D6}' | '\u{00D8}'..='\u{00F6}' | '\u{00F8}'..='\u{02FF}' |
               '\u{0370}'..='\u{037D}' | '\u{037F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' |
               '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' |
               '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}

/// Returns `true` when the specified character is name start character.
fn is_name_part_char(ch: char) -> bool {
  is_name_start_char(ch) || is_digit(ch) || matches!(ch, '\u{00B7}' | '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

/// Returns `true` when the specified character is a whitespace character.
fn is_whitespace(ch: char) -> bool {
  is_vertical_space(ch)
    || matches!(
      ch,
      '\u{0009}' | '\u{0020}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'
        ..='\u{200B}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
    )
}

/// Returns `true` when the specified character is a vertical space.
fn is_vertical_space(ch: char) -> bool {
  matches!(ch, '\u{000A}'..='\u{000D}')
}

/// Return the decimal value that corresponds to hexadecimal digit, case-insensitive.
fn hex_to_decimal(ch: char) -> Result<u64> {
  match ch {
    '0' => Ok(0),
    '1' => Ok(1),
    '2' => Ok(2),
    '3' => Ok(3),
    '4' => Ok(4),
    '5' => Ok(5),
    '6' => Ok(6),
    '7' => Ok(7),
    '8' => Ok(8),
    '9' => Ok(9),
    'A' | 'a' => Ok(10),
    'B' | 'b' => Ok(11),
    'C' | 'c' => Ok(12),
    'D' | 'd' => Ok(13),
    'E' | 'e' => Ok(14),
    'F' | 'f' => Ok(15),
    other => Err(err_expected_hex_digit(other)),
  }
}

/// Returns `true` when the specified name is a built-in type name.
fn is_built_in_type_name(name: &str) -> bool {
  matches!(
    name,
    "Any" | "Null" | "boolean" | "number" | "string" | "date" | "date and time" | "time" | "years and months duration" | "days and time duration"
  )
}

/// Returns `true` when `text`, the text after the word `as` in a name chain, makes `as` the
/// operator of a name that is not in scope ([NAME_ENDING_KEYWORDS]): nothing follows (a text
/// being written, `item as`, which an editor completes with a type), or a whole type of FEEL
/// follows ([type_end]) that ends the name: the text ends after it, or a keyword that ends
/// names, or a symbol other than `:` and `'`. After `cost as string:` stands a name being given
/// a value or a type (`{cost as string: 1}`, `f(cost as string: 1)`), `p.Count as number of
/// items` is one name, and so is `p.Items as list` in `p.Items as list < 3`.
fn type_ends_name(text: &str) -> bool {
  if text.trim_start_matches(is_whitespace).is_empty() {
    return true;
  }
  let Some(end) = type_end(text, 0) else {
    return false;
  };
  let after = text[end..].trim_start_matches(is_whitespace);
  match after.chars().next() {
    None => true,
    Some(':' | '\'') => false,
    Some(ch) if is_name_part_char(ch) => {
      let (start, end) = leading_word(after);
      NAME_ENDING_KEYWORDS.contains(&&after[start..end])
    }
    Some(_) => true,
  }
}

/// The most nested types that [type_end] reads (`list<list<…>>`); a deeper one is no type to it.
const MAX_TYPE_DEPTH: usize = 16;

/// The start and end of the word at the start of `text`, after white space.
fn leading_word(text: &str) -> (usize, usize) {
  let start = text.len() - text.trim_start_matches(is_whitespace).len();
  let end = text[start..].find(|ch: char| !is_name_part_char(ch)).map_or(text.len(), |end| start + end);
  (start, end)
}

/// The end of the type of FEEL that `text` starts with, after white space (DMN 1.7 Beta 1,
/// grammar rule 54, with the abstract types of Figure 10-26): a built-in or abstract type
/// name, the multi-word ones whole, `list<T>`, `range<T>`, `context<>`, `context<name: T, …>`
/// or `function<T, …>->T`. None when it starts with none: a type that names an item definition
/// is a name like any other, which `as` does not end.
fn type_end(text: &str, depth: usize) -> Option<usize> {
  if depth > MAX_TYPE_DEPTH {
    return None;
  }
  // the end of the words `words` at the start of `text`
  let words_end = |words: &[&str]| -> Option<usize> {
    let mut at = 0;
    for expected in words {
      let (start, end) = leading_word(&text[at..]);
      if &text[at + start..at + end] != *expected {
        return None;
      }
      at += end;
    }
    Some(at)
  };
  // the end of `symbol` after white space at `at`
  // `<=` and `>=` are tokens of their own: a `<` or `>` before `=` is no bracket of a type
  // (`Kn as range<number>=1` compares, as in FEEL 1.6)
  let symbol = |at: usize, symbol: &str| -> Option<usize> {
    let rest = &text[at..];
    let start = rest.len() - rest.trim_start_matches(is_whitespace).len();
    let end = at + start + symbol.len();
    let comparison = matches!(symbol, "<" | ">") && text[end.min(text.len())..].starts_with('=');
    (rest[start..].starts_with(symbol) && !comparison).then_some(end)
  };
  let (start, end) = leading_word(text);
  match &text[start..end] {
    "Any" | "Null" | "boolean" | "number" | "string" | "time" | "comparable" | "temporal" => Some(end),
    "date" => Some(words_end(&["date", "and", "time"]).unwrap_or(end)),
    "years" => words_end(&["years", "and", "months", "duration"]),
    "days" => words_end(&["days", "and", "time", "duration"]),
    "duration" => Some(end),
    "list" | "range" => {
      let open = symbol(end, "<")?;
      let inner = open + type_end(&text[open..], depth + 1)?;
      symbol(inner, ">")
    }
    "context" => {
      let Some(mut at) = symbol(end, "<") else {
        // the context type of any entries
        return Some(end);
      };
      if let Some(close) = symbol(at, ">") {
        return Some(close);
      }
      loop {
        // a name of an entry: words, then `:`
        let (start, name_end) = leading_word(&text[at..]);
        if start == name_end {
          return None;
        }
        // the words of the name, white space between them
        let mut name_end = at + name_end;
        loop {
          let (start, end) = leading_word(&text[name_end..]);
          if start == 0 || start == end {
            break;
          }
          name_end += end;
        }
        let colon = symbol(name_end, ":")?;
        at = colon + type_end(&text[colon..], depth + 1)?;
        if let Some(close) = symbol(at, ">") {
          return Some(close);
        }
        at = symbol(at, ",")?;
      }
    }
    "function" => {
      let mut at = symbol(end, "<")?;
      if let Some(close) = symbol(at, ">") {
        at = close;
      } else {
        loop {
          at += type_end(&text[at..], depth + 1)?;
          if let Some(close) = symbol(at, ">") {
            at = close;
            break;
          }
          at = symbol(at, ",")?;
        }
      }
      let arrow = symbol(at, "->")?;
      Some(arrow + type_end(&text[arrow..], depth + 1)?)
    }
    _ => None,
  }
}

/// Returns `true` when the text of a part of a name chain is one additional name symbol.
fn is_symbol_part(text: &str) -> bool {
  let mut chars = text.chars();
  matches!((chars.next(), chars.next()), (Some(symbol), None) if is_additional_name_symbol(symbol))
}

/// Returns the length in bytes of the longest of `names`.
const fn longest(names: &[&str]) -> usize {
  let mut longest = 0;
  let mut index = 0;
  while index < names.len() {
    if names[index].len() > longest {
      longest = names[index].len();
    }
    index += 1;
  }
  longest
}

/// Returns the length in bytes of the longest built-in property name whose first word is `word`
/// (see [first_word]), or zero: after a dot, a bound of the candidate names with this first word
/// that can be built-in property names.
fn property_group_bound(word: &str) -> usize {
  BUILT_IN_PROPERTY_NAMES
    .iter()
    .filter(|name| name.starts_with(word) && first_word(name) == word)
    .map(|name| name.len())
    .max()
    .unwrap_or(0)
}

/// Returns the number of leading parts whose trimmed lengths add up to at most `bound`.
fn admitted_parts(parts: &[ChainPart], bound: usize) -> usize {
  let mut prefix_len = 0;
  parts
    .iter()
    .take_while(|part| {
      prefix_len += part.trimmed.len();
      prefix_len <= bound
    })
    .count()
}

/// The replacements that remove the spaces around additional name symbols in a flattened name,
/// applied in this order, each with the symbol it is about.
const NAME_SYMBOL_REPLACEMENTS: [(char, &str, &str); 6] = [
  ('.', " . ", "."),
  ('/', " / ", "/"),
  ('-', " - ", "-"),
  ('\'', " ' ", "'"),
  ('+', " + ", "+"),
  ('*', " * ", "*"),
];

/// Writes the flattened name of `parts` into `out`, in these steps: every part is trimmed of
/// white space at both ends; the parts are joined with single spaces; the result is trimmed;
/// then the spaces around the additional name symbols are removed, one replacement of
/// [NAME_SYMBOL_REPLACEMENTS] after the other, each scanning the text from the left for
/// non-overlapping occurrences. The buffers are reused; `tmp` is scratch space.
fn flatten_name_parts_into<'a>(parts: impl Iterator<Item = &'a str>, out: &mut String, tmp: &mut String) {
  tmp.clear();
  for (index, part) in parts.enumerate() {
    if index > 0 {
      tmp.push(WS);
    }
    tmp.push_str(part.trim_matches(is_whitespace));
  }
  out.clear();
  out.push_str(tmp.trim_matches(is_whitespace));
  for (symbol, from, to) in NAME_SYMBOL_REPLACEMENTS {
    // a replacement leaves a text without the symbol unchanged
    if out.contains(symbol) {
      replace_into(out, from, to, tmp);
      std::mem::swap(out, tmp);
    }
  }
}

/// Writes `text` with every non-overlapping occurrence of `from` replaced by `to` into `out`,
/// scanning from the left: the loop of [str::replace], into a reused buffer.
fn replace_into(text: &str, from: &str, to: &str, out: &mut String) {
  out.clear();
  let mut last_end = 0;
  for (start, part) in text.match_indices(from) {
    out.push_str(&text[last_end..start]);
    out.push_str(to);
    last_end = start + part.len();
  }
  out.push_str(&text[last_end..]);
}

#[cfg(test)]
mod tests;
