//! Implementation of the lexer for `FEEL` grammar.

use crate::errors::*;
use crate::lalr::TokenType;
use crate::scope::{ParsingScope, first_word};
use dsntk_common::Result;
use rheon_feel::Name;
use rheon_feel::bif::is_built_in_date_time_function_name;
use rheon_feel::bip::{BUILT_IN_PROPERTY_NAMES, is_built_in_property_name};
use std::ops::Range;

/// Definition of a single space character.
const WS: char = ' ';

/// Buffer size for lexer input.
const BUF_SIZE: usize = 12;

/// Definition of decimal separator.
const DECIMAL_SEPARATOR: char = '.';

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
  YyState,
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
  StartSimpleExpression,
  StartSimpleExpressions,
  StartSimpleValue,
  StartTextualExpression,
  StartTextualExpressions,
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
  /// Flag indicating if the `between` keyword was encountered.
  /// When this flag is set, the next `and` token is returned as `band` keyword,
  /// otherwise it is returned as `and`. This allows to disambiguate the `and`
  /// operator used in between clause from conjunction.
  /// After consuming first `and` as `band` this flag is cleared by the lexer.
  between: bool,
  /// ???
  type_name: bool,
  /// ???
  till_in: bool,
  /// ??
  after_dot: bool,
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
  /// Start offset of the last token read from input: the hook of the token log of the parser
  /// (`Parser::with_token_log`), which the differential tests compare.
  #[cfg(test)]
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
      between: false,
      type_name: false,
      till_in: false,
      after_dot: false,
      name_buf: String::new(),
      name_tmp: String::new(),
      chain_parts: vec![],
      chain_end: 0,
      chain_last_empty: None,
      #[cfg(test)]
      token_start: 0,
    }
  }

  /// Returns the start offset of the last token read from input (a byte offset).
  #[cfg(test)]
  pub fn token_start(&self) -> usize {
    self.token_start
  }

  pub fn set_unary_tests(&mut self) {
    self.unary_tests = true;
  }

  pub fn set_between(&mut self) {
    self.between = true;
  }

  pub fn set_type_name(&mut self) {
    self.type_name = true;
  }

  pub fn set_till_in(&mut self) {
    self.till_in = true;
  }

  pub fn set_after_dot(&mut self) {
    self.after_dot = true;
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
        tt @ TokenType::StartSimpleExpression => (tt, TokenValue::StartSimpleExpression),
        tt @ TokenType::StartSimpleExpressions => (tt, TokenValue::StartSimpleExpressions),
        tt @ TokenType::StartSimpleValue => (tt, TokenValue::StartSimpleValue),
        tt @ TokenType::StartTextualExpression => (tt, TokenValue::StartTextualExpression),
        tt @ TokenType::StartTextualExpressions => (tt, TokenValue::StartTextualExpressions),
        tt @ TokenType::StartUnaryTests => (tt, TokenValue::StartUnaryTests),
        _ => (TokenType::YyError, TokenValue::YyError),
      });
    }
    let result = self.read_next_token();
    self.unary_tests = false;
    result
  }

  /// Reads the next token starting from current position.
  fn read_next_token(&mut self) -> Result<(TokenType, TokenValue)> {
    let chars = self.read_input();
    #[cfg(test)]
    {
      self.token_start = self.position;
    }
    match chars {
      ['s', 'a', 't', 'i', 's', 'f', 'i', 'e', 's', WS, _, _] => {
        self.position += 9;
        Ok((TokenType::Satisfies, TokenValue::Satisfies))
      }
      ['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', WS, _, _, _] => {
        self.position += 8;
        Ok((TokenType::External, TokenValue::External))
      }
      ['f', 'u', 'n', 'c', 't', 'i', 'o', 'n', _, _, _, _] if self.is_function_separator(8) => {
        self.position += 8;
        Ok((TokenType::Function, TokenValue::Function))
      }
      ['i', 'n', 's', 't', 'a', 'n', 'c', 'e', WS, _, _, _] => {
        self.position += 8;
        Ok((TokenType::Instance, TokenValue::Instance))
      }
      ['b', 'e', 't', 'w', 'e', 'e', 'n', WS, _, _, _, _] => {
        self.position += 7;
        Ok((TokenType::Between, TokenValue::Between))
      }
      ['c', 'o', 'n', 't', 'e', 'x', 't', _, _, _, _, _] if self.is_context_separator(7) => {
        self.position += 7;
        Ok((TokenType::Context, TokenValue::Context))
      }
      ['r', 'e', 't', 'u', 'r', 'n', WS, _, _, _, _, _] => {
        self.position += 6;
        Ok((TokenType::Return, TokenValue::Return))
      }
      ['e', 'v', 'e', 'r', 'y', WS, _, _, _, _, _, _] => {
        self.position += 5;
        Ok((TokenType::Every, TokenValue::Every))
      }
      ['f', 'a', 'l', 's', 'e', ch, _, _, _, _, _, _] if is_separator(ch) => {
        self.position += 5;
        Ok((TokenType::Boolean, TokenValue::Boolean(false)))
      }
      ['r', 'a', 'n', 'g', 'e', _h, _, _, _, _, _, _] if self.is_range_separator(5) => {
        self.position += 5;
        Ok((TokenType::Range, TokenValue::Range))
      }
      ['n', 'u', 'l', 'l', ch, _, _, _, _, _, _, _] if is_separator(ch) => {
        self.position += 4;
        Ok((TokenType::Null, TokenValue::Null))
      }
      ['e', 'l', 's', 'e', WS, _, _, _, _, _, _, _] => {
        self.position += 4;
        Ok((TokenType::Else, TokenValue::Else))
      }
      ['l', 'i', 's', 't', _, _, _, _, _, _, _, _] if self.is_list_separator(4) => {
        self.position += 4;
        Ok((TokenType::List, TokenValue::List))
      }
      ['s', 'o', 'm', 'e', WS, _, _, _, _, _, _, _] => {
        self.position += 4;
        Ok((TokenType::Some, TokenValue::Some))
      }
      ['t', 'h', 'e', 'n', WS, _, _, _, _, _, _, _] => {
        self.position += 4;
        Ok((TokenType::Then, TokenValue::Then))
      }
      ['t', 'r', 'u', 'e', ch, _, _, _, _, _, _, _] if is_separator(ch) => {
        self.position += 4;
        Ok((TokenType::Boolean, TokenValue::Boolean(true)))
      }
      ['a', 'n', 'd', WS, _, _, _, _, _, _, _, _] if !self.between => {
        self.position += 3;
        Ok((TokenType::And, TokenValue::And))
      }
      ['a', 'n', 'd', WS, _, _, _, _, _, _, _, _] if self.between => {
        self.between = false;
        self.position += 3;
        Ok((TokenType::BetweenAnd, TokenValue::BetweenAnd))
      }
      ['f', 'o', 'r', WS, _, _, _, _, _, _, _, _] => {
        self.position += 3;
        Ok((TokenType::For, TokenValue::For))
      }
      ['n', 'o', 't', ch, _, _, _, _, _, _, _, _] if self.unary_tests && is_keyword_not_separator(ch) => {
        self.position += 3;
        Ok((TokenType::Not, TokenValue::Not))
      }
      ['i', 'f', WS, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::If, TokenValue::If))
      }
      ['i', 'n', WS, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::In, TokenValue::In))
      }
      ['o', 'f', WS, _, _, _, _, _, _, _, _, _] => {
        self.position += 2;
        Ok((TokenType::Of, TokenValue::Of))
      }
      ['o', 'r', WS, _, _, _, _, _, _, _, _, _] => {
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
  /// to the next character after comment.
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
        self.position += 2;
        while let Some(ch) = self.char_at(0) {
          if ch == '*'
            && let Some('/') = self.char_at(1)
          {
            self.position += 2;
            return true;
          }
          self.position += ch.len_utf8();
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
        _ => return Ok((TokenType::YyEof, TokenValue::YyEof)),
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
          'e' | 'E' => {
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
          'e' | 'E' => {
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

    //------------------------------------------------------------------------------
    // Tweak with the name `item` used in filters.
    // If the first part is `item`, then treat it as a resulting name.
    //-------------------------------------------------------------------------------
    if self.part_text(first) == "item" {
      self.position = self.chain_parts[first].end;
      return Ok((TokenType::Name, TokenValue::Name("item".into())));
    }

    //-------------------------------------------------------------------------------
    // Tweak with the name in `for` and `quantified` expressions.
    // The variable's name is the name until the keyword `in` appears.
    // Return the whole name before the `in` keyword.
    // A name start is never a literal terminal symbol (DMN 1.6 §10.3.1.4), so a variable
    // can not start with `in`: then `in` is the keyword (and the variable name is missing).
    //-------------------------------------------------------------------------------
    if self.till_in
      && let Some(index) = (first..self.chain_parts.len()).position(|part| self.part_text(part) == "in")
    {
      self.till_in = false;
      if index == 0 {
        self.position = self.chain_parts[first].end;
        return Ok((TokenType::In, TokenValue::In));
      }
      self.position = self.chain_parts[first + index - 1].end;
      return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, first + index))));
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
      }
      // return to the input all characters that do not belong to the name that was found
      self.position = self.chain_parts[first + part_count - 1].end;
      // return the name that exists in the current context
      return Ok((TokenType::Name, TokenValue::Name(self.chain_name(first, first + part_count))));
    }

    //------------------------------------------------------------------------------
    // Build the name from name parts.
    //------------------------------------------------------------------------------
    let name = self.chain_name(first, self.chain_parts.len());

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
    //------------------------------------------------------------------------------
    // The current character on input is already a `name start` character, the first
    // character of the first part.
    //------------------------------------------------------------------------------
    let mut start = self.position;

    //------------------------------------------------------------------------------
    // Now parse the rest of the input using a state machine,
    // looking for characters belonging to this name.
    //------------------------------------------------------------------------------
    let mut state = 1;
    loop {
      match state {
        1 => {
          if self.is_next_name_part_char() {
            self.advance();
          } else {
            self.push_chain_part(start);
            state = 2;
          }
        }
        2 => {
          if self.is_next_name_part_char() {
            // the next part starts with the next character
            start = self.next_position();
            state = 1;
          } else if self.is_next_additional_name_symbol() {
            state = 4;
          } else if self.is_next_whitespace() {
            state = 5;
          } else {
            self.advance();
            break;
          }
        }
        4 => {
          if self.is_next_additional_name_symbol() {
            self.advance();
            self.push_chain_part(self.position);
          } else {
            state = 2;
          }
        }
        5 => {
          if self.is_next_whitespace() {
            self.advance();
          } else {
            state = 2;
          }
        }
        _ => {}
      }
    }
    self.chain_end = self.position;
  }

  /// Ends a part of the chain: from `start` to the current character, the last of the part.
  fn push_chain_part(&mut self, start: usize) {
    work!(Parts, 1);
    let end = self.next_position();
    let text = &self.input[start..end];
    let trim_start = start + (text.len() - text.trim_start().len());
    let trimmed = trim_start..trim_start + text.trim().len();
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
    while part_count > 0 {
      work!(Candidates, 1);
      // flatten the name parts to compare it with built-in names and keys in current scope
      let texts = parts[..part_count].iter().map(|part| &self.input[part.trimmed.clone()]);
      flatten_name_parts_into(texts, &mut self.name_buf, &mut self.name_tmp);
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

  /// Returns the position of the character after the current one.
  fn next_position(&self) -> usize {
    self.position + utf8_len(self.input.as_bytes()[self.position])
  }

  /// Moves the position to the next character.
  fn advance(&mut self) {
    self.position = self.next_position();
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

  /// Checks if the next value on input is whitespace character.
  fn is_next_whitespace(&self) -> bool {
    if let Some(ch) = self.char_at(1) { is_whitespace(ch) } else { false }
  }

  /// Checks if the next character is the name part character.
  fn is_next_name_part_char(&self) -> bool {
    if let Some(ch) = self.char_at(1) { is_name_part_char(ch) } else { false }
  }

  /// Returns **true* when the next character on input is the additional name symbol.
  fn is_next_additional_name_symbol(&self) -> bool {
    if let Some(ch) = self.char_at(1) { is_additional_name_symbol(ch) } else { false }
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

/// Returns `true` when the specified character is a separator (white space equivalent).
fn is_separator(ch: char) -> bool {
  matches!(ch, WS | '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '.' | ',' | ')' | '[' | ']' | '}')
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
    tmp.push_str(part.trim());
  }
  out.clear();
  out.push_str(tmp.trim());
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
