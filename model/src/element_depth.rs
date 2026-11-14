//! # The nesting bound of the elements of a model
//!
//! The XML parser, the schema validation, the model parser and everything built from the model
//! later follow the element tree recursively, one stack frame or more per level. The XML parser
//! does so while it reads the text, so the bound is checked on the text before it is parsed: a
//! model nested too deeply is an error instead of a stack overflow, and every recursion that
//! follows is bounded by it.
//!
//! The bound is set so that a model nested to it is parsed, built and evaluated on a 2 MiB
//! thread by an unoptimized build, the smallest native stack a host gives. Such a thread holds
//! 124 levels on aarch64; the bound keeps a margin of about a quarter below that for the targets
//! whose ceiling is not measured.

use crate::errors::err_elements_nested_too_deeply;
use dsntk_common::Result;

/// The deepest level at which the elements of a model may nest; the root element is level 1.
pub const MAX_ELEMENT_DEPTH: usize = 100;

/// Checks that no element of the XML text is nested deeper than [MAX_ELEMENT_DEPTH], without recursion.
///
/// The text is read once, the way the XML parser reads it: start tags (with their quoted
/// attribute values), end tags and empty-element tags change the depth, while comments, CDATA
/// sections, processing instructions and text do not. Where the XML parser stops with an error
/// (an attribute value or a tag containing `<`, a document type declaration, a truncated
/// construct), the check stops too and leaves the error to the parser, which recurses no
/// deeper than the check has counted.
pub fn check_element_depth(text: &str) -> Result<()> {
  let bytes = text.as_bytes();
  let mut depth = 0_usize;
  let mut position = 0_usize;
  while let Some(start) = find_byte(bytes, position, b'<') {
    let rest = &bytes[start..];
    let next = if rest.starts_with(b"<!--") {
      find(bytes, start + 4, b"-->").map(|end| end + 3)
    } else if rest.starts_with(b"<![CDATA[") {
      find(bytes, start + 9, b"]]>").map(|end| end + 3)
    } else if rest.starts_with(b"<!") {
      // a document type declaration, or no markup at all; the parser rejects both
      None
    } else if rest.starts_with(b"<?") {
      find(bytes, start + 2, b"?>").map(|end| end + 2)
    } else if rest.starts_with(b"</") {
      depth = depth.saturating_sub(1);
      find_byte(bytes, start + 2, b'>').map(|end| end + 1)
    } else {
      if depth == MAX_ELEMENT_DEPTH {
        return Err(err_elements_nested_too_deeply(MAX_ELEMENT_DEPTH));
      }
      depth += 1;
      start_tag_end(bytes, start + 1).map(|(end, empty)| {
        if empty {
          depth -= 1;
        }
        end + 1
      })
    };
    match next {
      Some(next) => position = next,
      None => break,
    }
  }
  Ok(())
}

/// Returns the position of the `>` that ends a start tag whose name begins at `position`, and
/// whether it is an empty-element tag (`/>`). Quoted attribute values are skipped; a `<` inside
/// the tag, which the XML parser rejects, or the end of the text gives `None`.
fn start_tag_end(bytes: &[u8], mut position: usize) -> Option<(usize, bool)> {
  while position < bytes.len() {
    match bytes[position] {
      b'>' => return Some((position, position > 0 && bytes[position - 1] == b'/')),
      b'<' => return None,
      quote @ (b'"' | b'\'') => {
        let end = bytes[position + 1..].iter().position(|&byte| byte == quote || byte == b'<')? + position + 1;
        if bytes[end] == b'<' {
          return None;
        }
        position = end + 1;
      }
      _ => position += 1,
    }
  }
  None
}

/// Returns the position of the first `byte` at or after `position`.
fn find_byte(bytes: &[u8], position: usize, byte: u8) -> Option<usize> {
  bytes.get(position..)?.iter().position(|&b| b == byte).map(|offset| position + offset)
}

/// Returns the position of the first occurrence of `pattern` at or after `position`.
fn find(bytes: &[u8], position: usize, pattern: &[u8]) -> Option<usize> {
  bytes
    .get(position..)?
    .windows(pattern.len())
    .position(|window| window == pattern)
    .map(|offset| position + offset)
}
