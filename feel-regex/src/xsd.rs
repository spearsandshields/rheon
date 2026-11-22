//! The front end of the regex built-ins: a pattern is read in the syntax of XML Schema Part 2
//! Appendix F (XSD 1.0 regular expressions) with the additions of XQuery 1.0 and XPath 2.0
//! Functions and Operators §7.6.1 (DMN 1.6 Table 75 note 2), and written in the syntax of the
//! engine with the meaning XSD gives it.
//!
//! Reading the pattern here, not by the engine, decides what a pattern is: a construct of the
//! engine's own syntax (`\b`, `\A`, `(?i)`, look-around, `\x41`, a `{` that is no quantifier) is
//! an invalid pattern (F&O FORX0002), and the classes have the XSD sets: `\s` is the four XML
//! space characters, `\w` every character but punctuation, separators and others, `\i` and `\c`
//! the XML name characters, `.` every character but the two line ends (with `s`, every
//! character). The input is matched as it is.
//!
//! The additions of F&O 2.0 are the anchors `^` and `$`, the reluctant quantifiers (`*?`, `{n,m}?`),
//! back-references (`\1`) and the escape `\$`. F&O 3.1 adds non-capturing groups `(?:...)`
//! ([Level::Fo31]).

use crate::unicode_blocks::block;

/// The syntax that a pattern is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Level {
  /// XQuery 1.0 and XPath 2.0 F&O §7.6.1, the syntax of DMN 1.6.
  #[default]
  Fo20,
  /// XPath and XQuery F&O 3.1 §5.6.1: also non-capturing groups.
  #[allow(dead_code)]
  Fo31,
}

/// A pattern written in the syntax of the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Translated {
  /// The engine's source.
  pub(crate) source: String,
}

/// Why a pattern is not one: the position of the character in the pattern, and what was
/// expected there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refused {
  pub(crate) at: usize,
  pub(crate) expected: &'static str,
  /// The pattern nests groups deeper than the engines read: a limit, not an invalid pattern.
  pub(crate) nesting: bool,
}

/// The most groups that may nest in a pattern: deeper is refused as the engines refuse it, so the
/// reading of a pattern never runs out of stack.
const MAX_NESTING: usize = 250;

/// `pattern` written for the engine, with `.` meaning every character when `dot_all` (the flag
/// `s`), or why it is no pattern.
pub(crate) fn translate(pattern: &str, dot_all: bool, level: Level) -> Result<Translated, Refused> {
  let mut reader = Reader {
    chars: pattern.chars().collect(),
    at: 0,
    out: String::with_capacity(pattern.len() + 8),
    dot_all,
    level,
    groups: 0,
    closed: Vec::new(),
    open: Vec::new(),
    classes: 0,
  };
  reader.reg_exp()?;
  if reader.at < reader.chars.len() {
    // only an unmatched `)` stops a regular expression before the end
    return Err(reader.refuse("an operand or the end of the pattern, not an unmatched ')'"));
  }
  Ok(Translated { source: reader.out })
}

struct Reader {
  chars: Vec<char>,
  at: usize,
  out: String,
  dot_all: bool,
  level: Level,
  /// The capturing groups opened so far.
  groups: usize,
  /// For each capturing group, by its number less one, whether it is closed: a back-reference names
  /// a closed one.
  closed: Vec<bool>,
  /// The groups that are open, innermost last: the number of a capturing one, 0 for another.
  open: Vec<usize>,
  /// The character classes that are open: a subtraction nests one in another.
  classes: usize,
}

impl Reader {
  fn peek(&self) -> Option<char> {
    self.chars.get(self.at).copied()
  }

  fn peek_at(&self, offset: usize) -> Option<char> {
    self.chars.get(self.at + offset).copied()
  }

  fn refuse(&self, expected: &'static str) -> Refused {
    Refused {
      at: self.at,
      expected,
      nesting: false,
    }
  }

  /// regExp ::= branch ( '|' branch )*
  fn reg_exp(&mut self) -> Result<(), Refused> {
    self.branch()?;
    while self.peek() == Some('|') {
      self.at += 1;
      self.out.push('|');
      self.branch()?;
    }
    Ok(())
  }

  /// branch ::= piece*
  fn branch(&mut self) -> Result<(), Refused> {
    while let Some(c) = self.peek() {
      if c == '|' || c == ')' {
        break;
      }
      self.piece()?;
    }
    Ok(())
  }

  /// piece ::= atom quantifier?
  fn piece(&mut self) -> Result<(), Refused> {
    let start = self.out.len();
    let anchor = self.atom()?;
    if let Some(c) = self.peek()
      && matches!(c, '?' | '*' | '+' | '{')
    {
      // F&O §7.6.1 makes `^` and `$` atoms, which a quantifier may follow: grouped for the engine
      if anchor {
        self.out.insert_str(start, "(?:");
        self.out.push(')');
      }
      self.quantifier()?;
    }
    Ok(())
  }

  /// quantifier ::= ( [?*+] | '{' quantity '}' ) '?'?
  fn quantifier(&mut self) -> Result<(), Refused> {
    match self.peek() {
      Some(c @ ('?' | '*' | '+')) => {
        self.at += 1;
        self.out.push(c);
      }
      Some('{') => {
        self.at += 1;
        let min = self.number().ok_or_else(|| self.refuse("a number of repetitions after '{'"))?;
        let max = match self.peek() {
          Some('}') => Some(min.clone()),
          Some(',') => {
            self.at += 1;
            if self.peek() == Some('}') {
              None
            } else {
              Some(self.number().ok_or_else(|| self.refuse("a number of repetitions after ','"))?)
            }
          }
          _ => return Err(self.refuse("',' or '}' in a quantity")),
        };
        if self.peek() != Some('}') {
          return Err(self.refuse("'}' to end a quantity"));
        }
        self.at += 1;
        if let Some(max) = &max
          && compare_numbers(&min, max) == std::cmp::Ordering::Greater
        {
          return Err(self.refuse("a quantity {n,m} with n not above m"));
        }
        // the engine counts repetitions in 32 bits; XSD has no limit, and a larger count than the
        // engine takes is refused as a limit of the engine would be
        let small = |digits: &str| trimmed(digits).len() <= 9;
        if !small(&min) || max.as_deref().is_some_and(|max| !small(max)) {
          return Err(self.refuse("a quantity of at most 999999999 repetitions"));
        }
        self.out.push('{');
        self.out.push_str(trimmed(&min));
        match &max {
          Some(max) if *max == min => {}
          Some(max) => {
            self.out.push(',');
            self.out.push_str(trimmed(max));
          }
          None => self.out.push(','),
        }
        self.out.push('}');
      }
      _ => return Ok(()),
    }
    // F&O §7.6.1: a reluctant quantifier
    if self.peek() == Some('?') {
      self.at += 1;
      self.out.push('?');
    }
    // XSD: a quantifier quantifies an atom, not another quantifier
    if matches!(self.peek(), Some('?' | '*' | '+' | '{')) {
      return Err(self.refuse("an atom, not a second quantifier"));
    }
    Ok(())
  }

  /// The decimal digits at the reading position.
  fn number(&mut self) -> Option<String> {
    let start = self.at;
    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
      self.at += 1;
    }
    (self.at > start).then(|| self.chars[start..self.at].iter().collect())
  }

  /// atom ::= NormalChar | charClass | '(' regExp ')' | backReference | '^' | '$'; whether the atom
  /// is an anchor.
  fn atom(&mut self) -> Result<bool, Refused> {
    let Some(c) = self.peek() else {
      return Err(self.refuse("an atom"));
    };
    match c {
      '(' => {
        self.at += 1;
        if self.peek() == Some('?') {
          if self.level == Level::Fo31 && self.peek_at(1) == Some(':') {
            self.at += 2;
            self.out.push_str("(?:");
            self.open.push(0);
          } else {
            return Err(self.refuse("a group, not the engine syntax '(?'"));
          }
        } else {
          self.groups += 1;
          self.closed.push(false);
          self.open.push(self.groups);
          self.out.push('(');
        }
        if self.open.len() > MAX_NESTING {
          return Err(Refused {
            at: self.at,
            expected: "groups nested at most 250 deep",
            nesting: true,
          });
        }
        self.reg_exp()?;
        if self.peek() != Some(')') {
          return Err(self.refuse("')' to close a group"));
        }
        self.at += 1;
        self.out.push(')');
        let group = self.open.pop().unwrap_or(0);
        if group != 0 {
          self.closed[group - 1] = true;
        }
        Ok(false)
      }
      '^' | '$' => {
        self.at += 1;
        self.out.push(c);
        Ok(true)
      }
      '.' => {
        self.at += 1;
        self.out.push_str(if self.dot_all { "(?s:.)" } else { r"[^\n\r]" });
        Ok(false)
      }
      '[' => {
        let class = self.class_expr()?;
        self.out.push_str(&class);
        Ok(false)
      }
      '\\' => {
        if self.peek_at(1).is_some_and(|d| d.is_ascii_digit() && d != '0') {
          self.back_reference()?;
          return Ok(false);
        }
        let escape = self.escape(false)?;
        self.out.push_str(&escape);
        Ok(false)
      }
      // XSD 1.0: these are no normal characters
      '?' | '*' | '+' | '{' | '}' | ']' | ')' | '|' => Err(self.refuse("an atom, not a metacharacter")),
      c => {
        self.at += 1;
        push_literal(&mut self.out, c);
        Ok(false)
      }
    }
  }

  /// backReference ::= '\' [1-9][0-9]*: F&O §7.6.1, as many digits as make the number of a group
  /// that is closed before it; a group that is not is refused.
  fn back_reference(&mut self) -> Result<(), Refused> {
    self.at += 1;
    let start = self.at;
    let mut number = 0_usize;
    let mut end = start;
    let mut digits = 0;
    while let Some(d) = self.peek_at(digits).and_then(|d| d.to_digit(10)) {
      let candidate = number * 10 + d as usize;
      digits += 1;
      if candidate > self.groups {
        break;
      }
      number = candidate;
      end = start + digits;
    }
    if end == start || !self.closed.get(number.wrapping_sub(1)).copied().unwrap_or(false) {
      return Err(self.refuse("a back-reference to a group closed before it"));
    }
    self.at = end;
    // F&O §7.6.1: a back-reference to a group that matched nothing matches the zero-length
    // string; the engine's back-reference fails there, so it is asked only when the group took
    // part (a conditional), and the group around it keeps a digit after it out of its number
    self.out.push_str(&format!("(?:(?({number})\\{number}))"));
    Ok(())
  }

  /// An escape outside or inside a class: a single character escape, a multi-character escape or
  /// a category escape, written for the engine (inside a class, as a class item).
  fn escape(&mut self, in_class: bool) -> Result<String, Refused> {
    self.at += 1;
    let Some(c) = self.peek() else {
      return Err(self.refuse("a character after '\\'"));
    };
    self.at += 1;
    Ok(match c {
      // SingleCharEsc, with F&O's \$
      'n' => r"\n".to_string(),
      'r' => r"\r".to_string(),
      't' => r"\t".to_string(),
      '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{' | '}' | '-' | '[' | ']' | '^' | '$' => {
        let mut out = String::new();
        push_literal(&mut out, c);
        out
      }
      // MultiCharEsc, with the XSD sets
      's' => set(in_class, false, SPACE),
      'S' => set(in_class, true, SPACE),
      'd' => set(in_class, false, r"\p{Nd}"),
      'D' => set(in_class, true, r"\p{Nd}"),
      'w' => set(in_class, true, r"\p{P}\p{Z}\p{C}"),
      'W' => set(in_class, false, r"\p{P}\p{Z}\p{C}"),
      'i' => set(in_class, false, NAME_START),
      'I' => set(in_class, true, NAME_START),
      'c' => set(in_class, false, NAME_CHAR),
      'C' => set(in_class, true, NAME_CHAR),
      'p' | 'P' => {
        let property = self.property()?;
        set(in_class, c == 'P', &property)
      }
      _ => {
        self.at -= 1;
        return Err(self.refuse("an escape of XML Schema, not one of the engine"));
      }
    })
  }

  /// charProp ::= IsCategory | IsBlock, between '{' and '}': the class items of the engine.
  fn property(&mut self) -> Result<String, Refused> {
    if self.peek() != Some('{') {
      return Err(self.refuse("'{' after \\p or \\P"));
    }
    self.at += 1;
    let start = self.at;
    while self.peek().is_some_and(|c| c != '}') {
      self.at += 1;
    }
    if self.peek() != Some('}') {
      return Err(self.refuse("'}' to end a property"));
    }
    let name: String = self.chars[start..self.at].iter().collect();
    let at = start;
    self.at += 1;
    if let Some(block_name) = name.strip_prefix("Is") {
      return match block(&name) {
        // the block's ranges, without the brackets of its class
        // the surrogate blocks hold no character of a string: an empty set
        Some(_) if name.contains("Surrogate") => Ok(r"\P{any}".to_string()),
        Some(ranges) => Ok(ranges.trim_start_matches('[').trim_end_matches(']').to_string()),
        None if block_name.is_empty() => Err(Refused {
          at,
          expected: "the name of a block after Is",
          nesting: false,
        }),
        None => Err(Refused {
          at,
          expected: "the name of a Unicode block known to XML Schema",
          nesting: false,
        }),
      };
    }
    if CATEGORIES.contains(&name.as_str()) {
      Ok(format!(r"\p{{{name}}}"))
    } else {
      Err(Refused {
        at,
        expected: "a general category of XML Schema (L, Lu, Nd, ...)",
        nesting: false,
      })
    }
  }

  /// charClassExpr ::= '[' charGroup ']', charGroup ::= ( posCharGroup | negCharGroup ) ( '-' charClassExpr )?
  fn class_expr(&mut self) -> Result<String, Refused> {
    self.classes += 1;
    if self.classes > MAX_NESTING {
      return Err(Refused {
        at: self.at,
        expected: "classes nested at most 250 deep",
        nesting: true,
      });
    }
    let class = self.class_group();
    self.classes -= 1;
    class
  }

  /// [Reader::class_expr] within the depth limit.
  fn class_group(&mut self) -> Result<String, Refused> {
    self.at += 1;
    let negated = self.peek() == Some('^');
    if negated {
      self.at += 1;
    }
    let mut items = String::new();
    let mut count = 0;
    loop {
      match self.peek() {
        None => return Err(self.refuse("']' to close a character class")),
        Some(']') if count > 0 => {
          self.at += 1;
          break;
        }
        Some(']') => return Err(self.refuse("a character or a range in a character class")),
        // a subtraction: '-' followed by a class, as the last part of the group
        Some('-') if self.peek_at(1) == Some('[') && count > 0 => {
          self.at += 1;
          let subtracted = self.class_expr()?;
          if self.peek() != Some(']') {
            return Err(self.refuse("']' after a subtracted class"));
          }
          self.at += 1;
          let base = format!("[{}{items}]", if negated { "^" } else { "" });
          return Ok(format!("[{base}--{subtracted}]"));
        }
        Some('[') => return Err(self.refuse("a character, not '[' (a class is subtracted with '-[')")),
        Some(_) => {
          self.class_item(&mut items, count == 0)?;
          count += 1;
        }
      }
    }
    Ok(format!("[{}{items}]", if negated { "^" } else { "" }))
  }

  /// A charRange or a charClassEsc of a group, written to `items`.
  fn class_item(&mut self, items: &mut String, first: bool) -> Result<(), Refused> {
    let start_at = self.at;
    // `--` is the engine's subtraction, no syntax of XML Schema (a class is subtracted with `-[`)
    if !first && self.peek() == Some('-') && self.peek_at(1) == Some('-') {
      return Err(self.refuse("a character or a range, not '--' (a class is subtracted with '-[')"));
    }
    let Some(start) = self.class_char(first)? else {
      // a multi-character or category escape
      let escape = self.escape(true)?;
      items.push_str(&escape);
      return self.no_range_after_a_set(start_at);
    };
    // seRange ::= charOrEsc '-' charOrEsc, where a charOrEsc is no unescaped '-'
    if self.peek() == Some('-') && !matches!(self.peek_at(1), Some(']') | Some('[') | None) {
      if self.chars[start_at] == '-' || self.peek_at(1) == Some('-') {
        return Err(Refused {
          at: start_at,
          expected: "a range of two characters other than '-' (write \\- for a hyphen)",
          nesting: false,
        });
      }
      self.at += 1;
      let end_at = self.at;
      let Some(end) = self.class_char(false)? else {
        return Err(Refused {
          at: end_at,
          expected: "a character to end a range, not a class escape",
          nesting: false,
        });
      };
      if end < start {
        return Err(Refused {
          at: start_at,
          expected: "a range whose start is not above its end",
          nesting: false,
        });
      }
      push_class_char(items, start);
      items.push('-');
      push_class_char(items, end);
      return Ok(());
    }
    push_class_char(items, start);
    Ok(())
  }

  /// A class escape is no end of a range.
  fn no_range_after_a_set(&self, start_at: usize) -> Result<(), Refused> {
    if self.peek() == Some('-') && !matches!(self.peek_at(1), Some(']') | Some('[') | None) {
      return Err(Refused {
        at: start_at,
        expected: "a character to start a range, not a class escape",
        nesting: false,
      });
    }
    Ok(())
  }

  /// A character of a group (XmlChar or SingleCharEsc), or `None` before a multi-character or
  /// category escape, which is not read. A '-' that starts no range is a character, as XML Schema
  /// 1.1 reads it and the TCK has it (`[0-9-.]`, TCK 1111): XSD 1.0 is not clear there.
  fn class_char(&mut self, _first: bool) -> Result<Option<char>, Refused> {
    let Some(c) = self.peek() else {
      return Err(self.refuse("']' to close a character class"));
    };
    match c {
      '\\' => {
        let Some(escaped) = self.peek_at(1) else {
          return Err(self.refuse("a character after '\\'"));
        };
        let single = match escaped {
          'n' => '\n',
          'r' => '\r',
          't' => '\t',
          '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{' | '}' | '-' | '[' | ']' | '^' | '$' => escaped,
          's' | 'S' | 'd' | 'D' | 'w' | 'W' | 'i' | 'I' | 'c' | 'C' | 'p' | 'P' => return Ok(None),
          _ => {
            self.at += 1;
            return Err(self.refuse("an escape of XML Schema, not one of the engine"));
          }
        };
        self.at += 2;
        Ok(Some(single))
      }
      '[' => Err(self.refuse("a character, not '['")),
      c => {
        self.at += 1;
        Ok(Some(c))
      }
    }
  }
}

/// The four XML space characters.
const SPACE: &str = r"\x20\t\n\r";

/// XML 1.0 (fifth edition) NameStartChar, the set of `\i`.
const NAME_START: &str = r":A-Z_a-z\x{C0}-\x{D6}\x{D8}-\x{F6}\x{F8}-\x{2FF}\x{370}-\x{37D}\x{37F}-\x{1FFF}\x{200C}-\x{200D}\x{2070}-\x{218F}\x{2C00}-\x{2FEF}\x{3001}-\x{D7FF}\x{F900}-\x{FDCF}\x{FDF0}-\x{FFFD}\x{10000}-\x{EFFFF}";

/// XML 1.0 (fifth edition) NameChar, the set of `\c`.
const NAME_CHAR: &str = r":A-Z_a-z\x{C0}-\x{D6}\x{D8}-\x{F6}\x{F8}-\x{2FF}\x{370}-\x{37D}\x{37F}-\x{1FFF}\x{200C}-\x{200D}\x{2070}-\x{218F}\x{2C00}-\x{2FEF}\x{3001}-\x{D7FF}\x{F900}-\x{FDCF}\x{FDF0}-\x{FFFD}\x{10000}-\x{EFFFF}\-.0-9\x{B7}\x{300}-\x{36F}\x{203F}-\x{2040}";

/// The general categories that XML Schema names (Part 2, F.1.1).
const CATEGORIES: [&str; 37] = [
  "L", "Lu", "Ll", "Lt", "Lm", "Lo", "M", "Mn", "Mc", "Me", "N", "Nd", "Nl", "No", "P", "Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po", "Z", "Zs", "Zl", "Zp", "S", "Sm", "Sc", "Sk",
  "So", "C", "Cc", "Cf", "Co", "Cn", "Cs",
];

/// A set of class items as an atom or as a class item: in a class, a nested class.
fn set(_in_class: bool, negated: bool, items: &str) -> String {
  format!("[{}{items}]", if negated { "^" } else { "" })
}

/// A character that stands for itself, escaped for the engine where it has a meaning there.
fn push_literal(out: &mut String, c: char) {
  if regex_syntax::is_meta_character(c) {
    out.push('\\');
  }
  out.push(c);
}

/// A character of a class, escaped for the engine where a class gives it a meaning.
fn push_class_char(out: &mut String, c: char) {
  match c {
    '\n' => out.push_str(r"\n"),
    '\r' => out.push_str(r"\r"),
    '\t' => out.push_str(r"\t"),
    c if regex_syntax::is_meta_character(c) => {
      out.push('\\');
      out.push(c);
    }
    c => out.push(c),
  }
}

/// The digits of a number without leading zeros (one zero for zero).
fn trimmed(digits: &str) -> &str {
  let trimmed = digits.trim_start_matches('0');
  if trimmed.is_empty() { "0" } else { trimmed }
}

/// The order of two numbers written in decimal digits.
fn compare_numbers(a: &str, b: &str) -> std::cmp::Ordering {
  let (a, b) = (trimmed(a), trimmed(b));
  a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}
