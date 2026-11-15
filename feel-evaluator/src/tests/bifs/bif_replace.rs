use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_string(false, &scope!(), r#"replace("abcd","(ab)|(a)","[1=$1][2=$2]")"#, "[1=ab][2=]cd");
}

#[test]
fn _0002() {
  te_string(false, &scope!(), "replace(\"a\",\"[b-z]\",\"\")", "a");
}

#[test]
fn _0003() {
  te_string(false, &scope!(), "replace(\"a\",\"[a-z]\",\"#\")", "#");
}

#[test]
fn _0004() {
  te_string(false, &scope!(), "replace(\"abc\",\"def\",\"#\")", "abc");
}

#[test]
fn _0005() {
  te_string(false, &scope!(), "replace(\"abc\",\"e\",\"#\")", "abc");
}

#[test]
fn _0006() {
  te_string(false, &scope!(), "replace(\"foobar\",\"^fo*b*\",\"#\")", "#ar");
}

#[test]
fn _0007() {
  te_string(false, &scope!(), "replace(\"abc\",\".^[d-z]\",\"#\")", "abc");
}

#[test]
fn _0008() {
  te_string(false, &scope!(), r#"replace("abracadabra","bra","*")"#, "a*cada*");
}

#[test]
fn _0009() {
  te_string(false, &scope!(), r#"replace("abracadabra","a.*a","*")"#, "*");
}

#[test]
fn _0010() {
  te_string(false, &scope!(), r#"replace("abracadabra","a.*?a","*")"#, "*c*bra");
}

#[test]
fn _0011() {
  te_string(false, &scope!(), r#"replace("abracadabra","a","")"#, "brcdbr");
}

#[test]
fn _0012() {
  te_string(false, &scope!(), r#"replace("AAAA","A+","b")"#, "b");
}

#[test]
fn _0013() {
  te_string(false, &scope!(), r#"replace("AAAA","A+?","b")"#, "bbbb");
}

#[test]
fn _0014() {
  te_string(false, &scope!(), r#"replace("darted","^(.*?)d(.*)$","$1$2")"#, "arted");
}

#[test]
fn _0015() {
  te_string(false, &scope!(), r#"replace("darted","^(.*?)d(.*)$","$1c$2")"#, "carted");
}

#[test]
fn _0016() {
  te_string(false, &scope!(), r#"replace("reluctant","r.*?t","X")"#, r#"Xant"#);
}

#[test]
fn _0017() {
  te_string(false, &scope!(), r#"replace("0123456789","(\d{3})(\d{3})(\d{4})","($1) $2-$3")"#, r#"(012) 345-6789"#);
}

#[test]
fn _0018() {
  te_string(false, &scope!(), "replace(\"abc\",\"[a-z]\",\"#\",\"\")", "###");
}

#[test]
fn _0019() {
  te_string(false, &scope!(), "replace(\"a.b.c.\",\"[a-z]\",\"#\",\"s\")", "#.#.#.");
}

#[test]
fn _0020() {
  te_string(false, &scope!(), "replace(\"abc\",\"[A-Z]\",\"#\",\"i\")", "###");
}

#[test]
fn _0021() {
  te_string(false, &scope!(), "replace(\"abc\",\"[a-z]\",\"#\",\"s\")", "###");
}

#[test]
fn _0022() {
  te_string(false, &scope!(), "replace(\"a b c d \",\"[a-z]\",\"#\",\"x\")", "# # # # ");
}

#[test]
fn _0023() {
  te_string(false, &scope!(), "replace(\"a b c d \",\"[a-z]\",\"#\")", "# # # # ");
}

#[test]
fn _0024() {
  te_string(false, &scope!(), r#"replace("abc",".^[d-z]*","smix")"#, "abc");
}

#[test]
fn _0025() {
  te_string(false, &scope!(), "replace(input:\"abc\",pattern:\"[a-z]\",replacement:\"#\")", "###");
}

#[test]
fn _0026() {
  te_string(false, &scope!(), "replace(input:\"abc\",pattern:\"[A-Z]\",replacement:\"#\",flags:\"\")", "abc");
}

#[test]
fn _0027() {
  te_string(false, &scope!(), "replace(input:\"abc\",pattern:\"[A-Z]\",replacement:\"#\",flags:\"i\")", "###");
}

#[test]
fn _0028() {
  te_string(false, &scope!(), "replace(input:\"abc\",pattern:\".^[d-z]*\",replacement:\"#\",flags:\"smix\")", "abc");
}

#[test]
fn _0029() {
  te_string(false, &scope!(), r#"replace("a\b\c","\\","\\\\","q")"#, r"a\\b\\c");
}

#[test]
fn _0030() {
  te_string(false, &scope!(), r#"replace("a/b/c","/","$","q")"#, "a$b$c");
}

#[test]
fn _0031() {
  te_null(
    false,
    &scope!(),
    "replace(\"abc\",\"[A-Z]\",\"#\",\"all unknown but i\")",
    "<FeelRegexError> invalid regex flag: 'a'",
  );
}

#[test]
fn _0032() {
  te_string(false, &scope!(), r#"replace(replace("anbncnz","n","\u000A"),"[A-C]\n","u","i")"#, "uuuz");
}

#[test]
fn _0033() {
  te_string(false, &scope!(), r#"replace("a\u000Ab\u000Ac\u000A","[A-Z]\n","u","i")"#, "uuu");
}

#[test]
fn _0034() {
  te_null(false, &scope!(), r#"replace()"#, r#"expected 3,4 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0035() {
  te_null(false, &scope!(), r#"replace("abc")"#, r#"expected 3,4 parameters, actual number of parameters is 1"#);
}

#[test]
fn _0036() {
  te_null(false, &scope!(), r#"replace("abc","b")"#, r#"expected 3,4 parameters, actual number of parameters is 2"#);
}

#[test]
fn _0037() {
  te_null(
    false,
    &scope!(),
    r#"replace("abc","b","c","d","e")"#,
    r#"expected 3,4 parameters, actual number of parameters is 5"#,
  );
}

#[test]
fn _0038() {
  te_null(
    false,
    &scope!(),
    "replace(i:\"abc\",pattern:\".^[d-z]*\",replacement:\"#\",flags:\"smix\")",
    r#"parameter 'input' not found"#,
  );
}

#[test]
fn _0039() {
  te_null(
    false,
    &scope!(),
    "replace(input:\"abc\",p:\".^[d-z]*\",replacement:\"#\",flags:\"smix\")",
    r#"parameter 'pattern' not found"#,
  );
}

#[test]
fn _0040() {
  te_null(
    false,
    &scope!(),
    "replace(input: \"abc\", pattern: \".^[d-z]*\", r:\"\", flags: \"smix\")",
    r#"parameter 'replacement' not found"#,
  );
}

#[test]
fn _0041() {
  te_null(false, &scope!(), r#"replace(10,"[a-z]","A")"#, r#"replace: input must be a string"#);
}

#[test]
fn _0042() {
  te_null(false, &scope!(), r#"replace("abc1",10,"A")"#, r#"replace: pattern must be a string"#);
}

#[test]
fn _0043() {
  te_null(false, &scope!(), r#"replace("abc1","[a-z]",10)"#, r#"replace: replacement must be a string"#);
}

#[test]
fn _0044() {
  te_null(false, &scope!(), r#"replace("abc1","[a-z","A")"#, r#"replace: invalid pattern"#);
}

#[test]
fn an_engine_limit_stops_the_evaluation() {
  let compiled_size = r"replace: the pattern '\w{10000}' exceeds the regex engine's compiled size limit; evaluation stopped";
  crate::tests::te_aborted(r#"replace("a", "\\w{10000}", "b")"#, crate::Dialect::Feel, compiled_size);
  crate::tests::te_aborted(r#"replace("a", "\\w{10000}", "b", "i")"#, crate::Dialect::Feel, compiled_size);
  let nested = crate::tests::nested_groups(300);
  crate::tests::te_aborted(
    &format!(r#"replace("a", "{nested}", "b")"#),
    crate::Dialect::Feel,
    &format!(
      "replace: the pattern '{}… (601 characters)' exceeds the regex engine's nesting limit; evaluation stopped",
      &nested[..120]
    ),
  );
}

#[test]
fn every_character_of_the_flags_is_a_flag() {
  // XQuery 1.0 and XPath 2.0 F&O §7.6.3: the flags are interpreted as for `fn:matches`
  te_null(false, &scope!(), r#"replace("abc","b","x"," i")"#, "<FeelRegexError> invalid regex flag: ' '");
  te_null(false, &scope!(), r#"replace("abc","b","x","?")"#, "<FeelRegexError> invalid regex flag: '?'");
  te_string(false, &scope!(), r#"replace("abc","B","x","ii")"#, "axc");
}

#[test]
fn flags_that_are_no_string_are_null() {
  te_null(false, &scope!(), r#"replace("abc","b","x",1)"#, "replace: flags must be a string");
  te_null(false, &scope!(), r#"replace("abc","b","x",["i"])"#, "replace: flags must be a string");
  // null flags are no flags
  te_string(false, &scope!(), r#"replace("abc","b","x",null)"#, "axc");
}

#[test]
fn q_takes_every_character_of_the_pattern_as_itself() {
  te_string(false, &scope!(), r#"replace("a1d","d","!","q")"#, "a1!");
  te_string(false, &scope!(), r#"replace("abc","b","x","q")"#, "axc");
  te_string(false, &scope!(), r#"replace("a.c abc",".c","X","q")"#, "aX abc");
  te_string(false, &scope!(), r#"replace("A.B",".","!","qi")"#, "A!B");
  // with `m`, `s` or `x` beside it, `q` still applies and they have no effect
  te_string(false, &scope!(), r#"replace("a.b",".","!","qs")"#, "a!b");
  te_string(false, &scope!(), r#"replace("a b"," ","-","qx")"#, "a-b");
}

#[test]
fn x_removes_the_whitespace_outside_character_classes() {
  te_string(false, &scope!(), r##"replace("a#b","#","-","x")"##, "a-b");
  te_string(false, &scope!(), r#"replace("a b","a [ ] b","-","x")"#, "-");
  te_string(false, &scope!(), r#"replace("ab","a b","-","x")"#, "-");
}

#[test]
fn an_invalid_pattern_with_flags_is_null() {
  te_null(false, &scope!(), r#"replace("a","[a-z","b","i")"#, "replace: invalid pattern");
}

#[test]
fn a_group_reference_takes_the_digits_that_name_a_group() {
  // XQuery 1.0 and XPath 2.0 F&O §7.6.3: N is the number of all the digits after `$`
  te_string(false, &scope!(), r#"replace("abc","b","[$0x]")"#, "a[bx]c");
  te_string(false, &scope!(), r#"replace("abc","(b)","[$1]")"#, "a[b]c");
  // a group beyond the pattern's groups and below 10 is empty
  te_string(false, &scope!(), r#"replace("abc","(b)","[$2]")"#, "a[]c");
  te_string(false, &scope!(), r#"replace("abc","b","[$01]")"#, "a[]c");
  // beyond both, the last digit is a literal and the rule applies again
  te_string(false, &scope!(), r#"replace("abc","(b)","$10")"#, "ab0c");
  te_string(false, &scope!(), r#"replace("abcde","(a)(b)(c)(d)(e)","$23")"#, "b3");
  te_string(
    false,
    &scope!(),
    r#"replace("abc","(b)","$123456789012345678901234567890")"#,
    "ab23456789012345678901234567890c",
  );
  te_string(
    false,
    &scope!(),
    r#"replace("abcdefghijkl","(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)","$12$1$0")"#,
    "laabcdefghijkl",
  );
}

#[test]
fn a_backslash_escapes_only_a_dollar_or_a_backslash() {
  te_string(false, &scope!(), r#"replace("abc","b","\\$")"#, "a$c");
  te_string(false, &scope!(), r#"replace("abc","b","\\\\")"#, r"a\c");
  te_string(false, &scope!(), r#"replace("abc","(b)","\\\\$1")"#, r"a\bc");
  te_string(false, &scope!(), r#"replace("abc","(b)","\\$1")"#, "a$1c");
}

#[test]
fn an_invalid_replacement_string_is_null() {
  let dollar = "<FeelRegexError> invalid replacement string: a '$' that is not escaped must be followed by a digit";
  te_null(false, &scope!(), r#"replace("abc","b","$")"#, dollar);
  te_null(false, &scope!(), r#"replace("abc","b","$$")"#, dollar);
  te_null(false, &scope!(), r#"replace("abc","(b)","${1}")"#, dollar);
  te_null(false, &scope!(), r#"replace("abc","b","$a")"#, dollar);
  let backslash = r"<FeelRegexError> invalid replacement string: a '\' must be followed by '\' or '$'";
  te_null(false, &scope!(), r#"replace("abc","b","\\x")"#, backslash);
  te_null(false, &scope!(), r#"replace("abc","b","x\\")"#, backslash);
  // also where no match uses it
  te_null(false, &scope!(), r#"replace("abc","z","$")"#, dollar);
}

#[test]
fn q_uses_the_replacement_string_as_is() {
  // XPath and XQuery F&O 3.1 §5.6.4: "If the q flag is present, the replacement string is used as is"
  te_string(false, &scope!(), r#"replace("abc","b","$1","q")"#, "a$1c");
  te_string(false, &scope!(), r#"replace("abc","b","\\x$","q")"#, r"a\x$c");
}
