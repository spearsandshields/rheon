use crate::errors::{err_empty_flags, err_invalid_flag, err_invalid_pattern};
use crate::unicode_blocks::replace_block_names;
use crate::utils::is_special_character;
use fancy_regex::Regex;

pub fn is_match(input: &str, pattern: &str) -> dsntk_common::Result<bool> {
  let Ok(re) = Regex::new(&fix_pattern(pattern, false, false)) else {
    return Err(err_invalid_pattern(pattern));
  };
  // A runtime failure (the backtracking limit for patterns with back-references or
  // look-around) is reported like an invalid pattern.
  re.is_match(&fix_input(input)).map_err(|_| err_invalid_pattern(pattern))
}

pub fn is_match_with_flags(input: &str, pattern: &str, flags: &str) -> dsntk_common::Result<bool> {
  let flags = flags.trim();
  if flags.is_empty() {
    return Err(err_empty_flags());
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
      _ => return Err(err_invalid_flag(ch)),
    }
  }
  if !regex_flags.is_empty() {
    regex_flags = format!("(?{})", regex_flags);
  }
  let pattern = format!("{}{}", regex_flags, fix_pattern(pattern, whitespaces, themselves));
  let Ok(re) = Regex::new(&pattern) else {
    return Err(err_invalid_pattern(&pattern));
  };
  re.is_match(&fix_input(input)).map_err(|_| err_invalid_pattern(&pattern))
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
