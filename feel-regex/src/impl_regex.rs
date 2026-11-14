use crate::errors::{Limit, RegexError, err_empty_flags, err_invalid_flag, err_invalid_pattern};
use crate::unicode_blocks::replace_block_names;
use crate::utils::is_special_character;
use fancy_regex::Regex;
use regex_syntax::ast;
use std::sync::LazyLock;

/// Whether `input` matches `pattern` (XQuery F&O 3.1 `fn:matches`, back-references included).
pub fn is_match(input: &str, pattern: &str) -> Result<bool, RegexError> {
  search(input, &fix_pattern(pattern, false, false), pattern)
}

/// Whether `input` matches `pattern` with `flags` (`s`, `m`, `i`, `x` and `q`).
pub fn is_match_with_flags(input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
  let flags = flags.trim();
  if flags.is_empty() {
    return Err(RegexError::Invalid(err_empty_flags()));
  }
  let mut regex_flags = String::new();
  let mut whitespaces = false;
  let mut themselves = false;
  for ch in flags.chars() {
    match ch {
      'i' | 'm' | 's' => regex_flags.push(ch),
      'x' => {
        whitespaces = true;
        regex_flags.push(ch);
      }
      'q' => themselves = true,
      _ => return Err(RegexError::Invalid(err_invalid_flag(ch))),
    }
  }
  if !regex_flags.is_empty() {
    regex_flags = format!("(?{})", regex_flags);
  }
  let pattern = format!("{}{}", regex_flags, fix_pattern(pattern, whitespaces, themselves));
  search(input, &pattern, &pattern)
}

/// Whether `input` matches the backtracking engine's `pattern` (back-references and look-around
/// included), or which limit the engine hit, or that the pattern is invalid (quoting `quoted`).
fn search(input: &str, pattern: &str, quoted: &str) -> Result<bool, RegexError> {
  let classify = |error: fancy_regex::Error| match error {
    fancy_regex::Error::ParseError(_, fancy_regex::ParseError::RecursionExceeded) => RegexError::LimitExceeded(Limit::Nesting),
    fancy_regex::Error::CompileError(error) => match *error {
      fancy_regex::CompileError::InnerError(error) if error.size_limit().is_some() => RegexError::LimitExceeded(Limit::CompiledSize),
      fancy_regex::CompileError::InnerError(error) if error.syntax_error().is_some_and(is_nest_limit) => RegexError::LimitExceeded(Limit::Nesting),
      _ => RegexError::Invalid(err_invalid_pattern(quoted)),
    },
    fancy_regex::Error::RuntimeError(fancy_regex::RuntimeError::StackOverflow) => RegexError::LimitExceeded(Limit::BacktrackingStack),
    // a search that fails at run time always ran out of its backtracking budget
    fancy_regex::Error::RuntimeError(_) => RegexError::LimitExceeded(Limit::Backtracking),
    _ => RegexError::Invalid(err_invalid_pattern(quoted)),
  };
  Regex::new(pattern).map_err(classify)?.is_match(&fix_input(input)).map_err(classify)
}

/// Whether a pattern nests groups deeper than the parser's limit.
fn is_nest_limit(error: &regex_syntax::Error) -> bool {
  matches!(error, regex_syntax::Error::Parse(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)))
}

/// The pattern compiled by the automata engine (no back-references), or which limit it hit, or
/// that it is invalid.
fn compile(pattern: &str) -> Result<regex::Regex, RegexError> {
  regex::Regex::new(pattern).map_err(|error| match error {
    regex::Error::CompiledTooBig(_) => RegexError::LimitExceeded(Limit::CompiledSize),
    // the error names no kind; the parser with the engine's defaults tells a nesting limit apart,
    // and it runs only on this error path
    regex::Error::Syntax(_) => match ast::parse::Parser::new().parse(pattern) {
      Err(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)) => RegexError::LimitExceeded(Limit::Nesting),
      _ => RegexError::Invalid(err_invalid_pattern(pattern)),
    },
    _ => RegexError::Invalid(err_invalid_pattern(pattern)),
  })
}

/// Numbered groups of a replacement in braces, `$1` as `${1}`: the engine reads the longest
/// name after `$`, so `$1a` would name the group `1a`.
static REPLACEMENT_GROUP: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new("\\$([1-9][0-9]*)").expect("a valid pattern"));

/// Replaces every match of `pattern` in `input` with `replacement` (`$1` names a group).
///
/// The flags `s`, `m`, `i` and `x` apply to the pattern; `q` quotes it, unless a flag other than
/// `i` comes with it. When the pattern with flags is invalid, the pattern without them is used.
/// An engine limit is reported at once.
pub fn replace(input: &str, pattern: &str, replacement: &str, flags: Option<&str>) -> Result<String, RegexError> {
  let replacement = REPLACEMENT_GROUP.replace_all(replacement, "$${${1}}");
  if let Some(flags_string) = flags {
    let mut flags = String::new();
    let mut flag_q = false;
    let mut clear_flag_q = false;
    for ch in flags_string.chars() {
      if ch == 'q' {
        flag_q = true;
      }
      if matches!(ch, 's' | 'm' | 'i' | 'x') {
        flags.push(ch);
        if ch != 'i' {
          clear_flag_q = true;
        }
      }
    }
    if clear_flag_q {
      flag_q = false;
    }
    let mut patt = String::new();
    for ch in pattern.chars() {
      if flag_q {
        patt.push('\\');
      }
      patt.push(ch);
    }
    let flagged = if flags.is_empty() { patt } else { format!("(?{flags}){patt}") };
    match compile(&flagged) {
      Ok(re) => return Ok(re.replace_all(input, replacement.as_ref()).into_owned()),
      Err(RegexError::LimitExceeded(limit)) => return Err(RegexError::LimitExceeded(limit)),
      Err(RegexError::Invalid(_)) => {}
    }
  }
  Ok(compile(pattern)?.replace_all(input, replacement.as_ref()).into_owned())
}

/// Splits `input` at every match of `delimiter`.
pub fn split(input: &str, delimiter: &str) -> Result<Vec<String>, RegexError> {
  Ok(compile(delimiter)?.split(input).map(String::from).collect())
}

// Nasty tricks on input.
fn fix_input(s: &str) -> String {
  s.replace("\r\n", "\n").replace('\r', "\n")
}

// Nasty tricks on pattern.
fn fix_pattern(pattern: &str, whitespaces: bool, themselves: bool) -> String {
  let mut pattern = replace_block_names(pattern, whitespaces);
  pattern = pattern.replace("-[", "--[");
  if whitespaces {
    pattern = pattern.replace("\\ ", "\\").replace("[ ]", "[\\ ]");
    // With the `x` flag whitespace is insignificant also inside `\p{...}` / `\P{...}`
    // (`\p{ IsLatin }`); the regex parser does not strip it there, so it is removed here.
    pattern = collapse_whitespace_in_property_names(&pattern);
  }
  if themselves {
    pattern = pattern
      .chars()
      .map(|ch| if is_special_character(ch) { format!(r#"\{}"#, ch) } else { format!("{}", ch) })
      .collect();
  }
  pattern
}

/// Removes whitespace inside Unicode property escapes `\p{...}` and `\P{...}`.
fn collapse_whitespace_in_property_names(pattern: &str) -> String {
  let mut output = String::with_capacity(pattern.len());
  let mut chars = pattern.chars().peekable();
  let mut escaped = false;
  while let Some(ch) = chars.next() {
    if escaped {
      escaped = false;
      output.push(ch);
      if (ch == 'p' || ch == 'P') && chars.peek() == Some(&'{') {
        // copy `{...}` without whitespace
        for inner in chars.by_ref() {
          if !inner.is_whitespace() {
            output.push(inner);
          }
          if inner == '}' {
            break;
          }
        }
      }
    } else if ch == '\\' {
      escaped = true;
      output.push(ch);
    } else {
      output.push(ch);
    }
  }
  output
}
