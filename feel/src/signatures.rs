//! The signatures of the built-in functions: the names and types of their parameters, the type
//! of their result and one line that says what they give. For editors (completion, parameter
//! hints), and for the evaluator, which takes the names of an invocation only when they are
//! parameters of one signature. A test of the evaluator invokes every signature by position and
//! by name and checks that the function takes it.

use crate::FeelType;
use crate::bif::Bif;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// A parameter of a signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameter {
  /// The name, as a named parameter gives it.
  pub name: &'static str,
  /// The FEEL type, as text (`number`, `list<string>`, `date or date and time`).
  pub feel_type: &'static str,
  /// May be left out.
  pub optional: bool,
  /// Stands for any number of arguments, one or more.
  pub rest: bool,
}

/// One way to invoke a built-in function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature {
  pub parameters: &'static [Parameter],
  /// The FEEL type of the result, as text.
  pub result: &'static str,
  /// Whether the parameters can be given by name; otherwise by position only.
  pub named: bool,
}

/// A built-in function with its signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltIn {
  pub name: &'static str,
  /// One line that says what the function gives.
  pub summary: &'static str,
  pub signatures: &'static [Signature],
}

impl Signature {
  /// The signature as text: `substring(string, start position, length?): string`.
  pub fn text(&self, name: &str) -> String {
    let parameters: Vec<String> = self
      .parameters
      .iter()
      .map(|parameter| {
        format!(
          "{}{}",
          parameter.name,
          if parameter.rest {
            "..."
          } else if parameter.optional {
            "?"
          } else {
            ""
          }
        )
      })
      .collect();
    format!("{name}({}): {}", parameters.join(", "), self.result)
  }
}

/// Whether the built-in function `bif` is of the function type `parameters -> result` (DMN 1.7 Beta 1,
/// 10.3.2.9.2): one of its signatures, from the table of signatures, takes that many arguments by
/// position, every parameter type of the target conforms to the type of the parameter there, and
/// every type its result can have conforms to the target's result type. A result written as the
/// type of a parameter with several types (`abs(n: number or duration): number or duration`) is
/// one function for each of them: its result is of the types of that parameter that the target
/// gives it, so abs is a `function<number>->number`. A rest parameter stands for one argument or
/// more. With `binding` (a function bound to a parameter or a variable, not `instance of`), a
/// result of type Any is not held against the target's result type, as for a function literal,
/// whose result type is not inferred.
pub fn conforms(bif: &Bif, parameters: &[FeelType], result: &FeelType, binding: bool) -> bool {
  let Some(built_in) = of(bif) else {
    return false;
  };
  built_in.signatures.iter().any(|signature| {
    let declared = signature.parameters;
    let required = declared.iter().filter(|parameter| !parameter.optional).count();
    let rest = declared.last().filter(|parameter| parameter.rest);
    if parameters.len() < required || (rest.is_none() && parameters.len() > declared.len()) {
      return false;
    }
    let takes_target = |declared: &Option<FeelType>, target: &FeelType| match declared {
      Some(declared) => target.is_conformant(declared),
      None => matches!(target, FeelType::Function(..) | FeelType::Null),
    };
    let takes = parameters.iter().enumerate().all(|(place, target)| {
      let Some(parameter) = declared.get(place).or(rest) else {
        return false;
      };
      table_types(parameter.feel_type).any(|declared| takes_target(&declared, target))
    });
    // the parameter whose types the result follows, and the type the target gives it there
    let follows = signature.result.contains(" or ").then(|| {
      parameters
        .iter()
        .enumerate()
        .find(|(place, _)| declared.get(*place).or(rest).is_some_and(|parameter| parameter.feel_type == signature.result))
    });
    let result_conforms = |declared: Option<FeelType>| declared.is_some_and(|declared| (binding && declared == FeelType::Any) || declared.is_conformant(result));
    takes
      && match follows.flatten() {
        Some((_, target)) => table_types(signature.result).filter(|declared| takes_target(declared, target)).all(result_conforms),
        None => table_types(signature.result).all(result_conforms),
      }
  })
}

/// The types that a type of the table of signatures stands for, several joined by `or`; `None`
/// for `function`, which is every function type.
fn table_types(text: &'static str) -> impl Iterator<Item = Option<FeelType>> {
  text.split(" or ").flat_map(|text| {
    let types: Vec<Option<FeelType>> = match text {
      "duration" => vec![Some(FeelType::DaysAndTimeDuration), Some(FeelType::YearsAndMonthsDuration)],
      "function" => vec![None],
      text => vec![Some(table_type(text))],
    };
    types
  })
}

/// The FEEL type of one type of the table: `list` and `range` take items of any type, a context
/// any entries.
fn table_type(text: &str) -> FeelType {
  match text {
    "number" => FeelType::Number,
    "string" => FeelType::String,
    "boolean" => FeelType::Boolean,
    "date" => FeelType::Date,
    "time" => FeelType::Time,
    "date and time" => FeelType::DateTime,
    "days and time duration" => FeelType::DaysAndTimeDuration,
    "years and months duration" => FeelType::YearsAndMonthsDuration,
    "context" => FeelType::Context(BTreeMap::new()),
    "range" => FeelType::Range(Box::new(FeelType::Any)),
    "list" => FeelType::List(Box::new(FeelType::Any)),
    text => match text.strip_prefix("list<").and_then(|text| text.strip_suffix('>')) {
      Some(item) => FeelType::List(Box::new(table_type(item))),
      None => FeelType::Any,
    },
  }
}

/// The built-in function of this name with its signatures.
pub fn built_in(name: &str) -> Option<&'static BuiltIn> {
  BUILT_INS.iter().find(|built_in| built_in.name == name)
}

/// The built-in function `bif` with its signatures.
pub fn of(bif: &Bif) -> Option<&'static BuiltIn> {
  /// The entry of each function at the place of its variant.
  static BY_VARIANT: LazyLock<Vec<Option<&'static BuiltIn>>> = LazyLock::new(|| {
    let mut entries = vec![None; BUILT_INS.len()];
    for built_in in &BUILT_INS {
      if let Ok(bif) = built_in.name.parse::<Bif>()
        && let Some(entry) = entries.get_mut(bif as usize)
      {
        *entry = Some(built_in);
      }
    }
    entries
  });
  BY_VARIANT.get(bif.clone() as usize).copied().flatten()
}

const fn p(name: &'static str, feel_type: &'static str) -> Parameter {
  Parameter {
    name,
    feel_type,
    optional: false,
    rest: false,
  }
}

const fn optional(name: &'static str, feel_type: &'static str) -> Parameter {
  Parameter {
    name,
    feel_type,
    optional: true,
    rest: false,
  }
}

const fn rest(name: &'static str, feel_type: &'static str) -> Parameter {
  Parameter {
    name,
    feel_type,
    optional: false,
    rest: true,
  }
}

const fn named(parameters: &'static [Parameter], result: &'static str) -> Signature {
  Signature { parameters, result, named: true }
}

const fn positional(parameters: &'static [Parameter], result: &'static str) -> Signature {
  Signature { parameters, result, named: false }
}

const fn built(name: &'static str, summary: &'static str, signatures: &'static [Signature]) -> BuiltIn {
  BuiltIn { name, summary, signatures }
}

/// The built-in functions, in the order of [crate::bif::BUILT_IN_FUNCTION_NAMES].
#[rustfmt::skip]
pub static BUILT_INS: [BuiltIn; 90] = [
  built("abs", "The absolute value.", &[
    named(&[p("n", "number or duration")], "number or duration"),
  ]),
  built("after", "Whether the first comes after the second.", &[
    named(&[p("point1", "Any"), p("point2", "Any")], "boolean"),
    named(&[p("point", "Any"), p("range", "range")], "boolean"),
    named(&[p("range", "range"), p("point", "Any")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("any", "True when an item is true.", &[
    named(&[p("list", "list<boolean>")], "boolean"),
    positional(&[rest("b", "boolean")], "boolean"),
  ]),
  built("all", "False when an item is false.", &[
    named(&[p("list", "list<boolean>")], "boolean"),
    positional(&[rest("b", "boolean")], "boolean"),
  ]),
  built("append", "The list with the items added at its end.", &[
    positional(&[p("list", "list"), rest("item", "Any")], "list"),
  ]),
  built("before", "Whether the first comes before the second.", &[
    named(&[p("point1", "Any"), p("point2", "Any")], "boolean"),
    named(&[p("point", "Any"), p("range", "range")], "boolean"),
    named(&[p("range", "range"), p("point", "Any")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("ceiling", "The number rounded towards positive infinity.", &[
    named(&[p("n", "number"), optional("scale", "number")], "number"),
  ]),
  built("coincides", "Whether the two are the same point or the same range.", &[
    named(&[p("point1", "Any"), p("point2", "Any")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("concatenate", "One list of the items of the lists.", &[
    positional(&[rest("list", "list")], "list"),
  ]),
  built("contains", "Whether the string contains the match.", &[
    named(&[p("string", "string"), p("match", "string")], "boolean"),
  ]),
  built("context", "A context from a list of entries with a key and a value each.", &[
    named(&[p("entries", "list<context>")], "context"),
  ]),
  built("context merge", "One context of the entries of the contexts; a later entry replaces an earlier one.", &[
    named(&[p("contexts", "list<context>")], "context"),
  ]),
  built("context put", "The context with an entry added or replaced.", &[
    named(&[p("context", "context"), p("key", "string"), p("value", "Any")], "context"),
    named(&[p("context", "context"), p("keys", "list<string>"), p("value", "Any")], "context"),
  ]),
  built("count", "The number of items.", &[
    named(&[p("list", "list")], "number"),
  ]),
  built("date", "A date from its text, from a date and time, or from a year, a month and a day.", &[
    named(&[p("from", "string or date and time")], "date"),
    named(&[p("year", "number"), p("month", "number"), p("day", "number")], "date"),
  ]),
  built("date and time", "A date and time from its text, or from a date and a time.", &[
    named(&[p("from", "string")], "date and time"),
    named(&[p("date", "date or date and time"), p("time", "time")], "date and time"),
    named(&[p("date", "date or date and time"), p("time", "time"), p("timezone", "string")], "date and time"),
  ]),
  built("day of week", "The name of the day of the week.", &[
    named(&[p("date", "date or date and time")], "string"),
  ]),
  built("day of year", "The number of the day in its year.", &[
    named(&[p("date", "date or date and time")], "number"),
  ]),
  built("days and time duration", "A days and time duration from its text.", &[
    named(&[p("from", "string")], "days and time duration"),
  ]),
  built("decimal", "The number rounded to a scale, half to even.", &[
    named(&[p("n", "number"), p("scale", "number")], "number"),
  ]),
  built("distinct values", "The list without its repeated items.", &[
    named(&[p("list", "list")], "list"),
  ]),
  built("duration", "A duration from its text.", &[
    named(&[p("from", "string")], "days and time duration or years and months duration"),
  ]),
  built("during", "Whether the first lies within the range.", &[
    named(&[p("point", "Any"), p("range", "range")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("ends with", "Whether the string ends with the match.", &[
    named(&[p("string", "string"), p("match", "string")], "boolean"),
  ]),
  built("even", "Whether the number is even.", &[
    named(&[p("number", "number")], "boolean"),
  ]),
  built("exp", "Euler's number raised to the number.", &[
    named(&[p("number", "number")], "number"),
  ]),
  built("finished by", "Whether the range ends with the second.", &[
    named(&[p("range", "range"), p("point", "Any")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("finishes", "Whether the first ends the range.", &[
    named(&[p("point", "Any"), p("range", "range")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("flatten", "The list with the items of its nested lists in place of them.", &[
    named(&[p("list", "list")], "list"),
  ]),
  built("floor", "The number rounded towards negative infinity.", &[
    named(&[p("n", "number"), optional("scale", "number")], "number"),
  ]),
  built("get entries", "The entries of the context as a list of contexts with a key and a value.", &[
    named(&[p("m", "context")], "list<context>"),
  ]),
  built("get value", "The value of an entry of the context.", &[
    named(&[p("m", "context"), p("key", "string")], "Any"),
  ]),
  built("includes", "Whether the range includes the second.", &[
    named(&[p("range", "range"), p("point", "Any")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("index of", "The positions of the items that equal the match.", &[
    named(&[p("list", "list"), p("match", "Any")], "list<number>"),
  ]),
  built("insert after", "The list with an item inserted after a position.", &[
    named(&[p("list", "list"), p("position", "number"), p("newItem", "Any")], "list"),
  ]),
  built("insert before", "The list with an item inserted before a position.", &[
    named(&[p("list", "list"), p("position", "number"), p("newItem", "Any")], "list"),
  ]),
  built("intersection", "The items that every list has.", &[
    positional(&[rest("list", "list")], "list"),
  ]),
  built("is", "Whether the two values are the same value of the same type.", &[
    named(&[p("value1", "Any"), p("value2", "Any")], "boolean"),
  ]),
  built("list contains", "Whether the list has an item that equals the element.", &[
    named(&[p("list", "list"), p("element", "Any")], "boolean"),
  ]),
  built("list replace", "The list with the item at a position, or every item a function matches, replaced.", &[
    named(&[p("list", "list"), p("position", "number"), p("newItem", "Any")], "list"),
    named(&[p("list", "list"), p("match", "function"), p("newItem", "Any")], "list"),
  ]),
  built("log", "The natural logarithm.", &[
    named(&[p("number", "number")], "number"),
  ]),
  built("lower case", "The string in lower case.", &[
    named(&[p("string", "string")], "string"),
  ]),
  built("matches", "Whether the input matches the regular expression.", &[
    named(&[p("input", "string"), p("pattern", "string"), optional("flags", "string")], "boolean"),
  ]),
  built("max", "The greatest item.", &[
    named(&[p("list", "list")], "Any"),
    positional(&[rest("c", "Any")], "Any"),
  ]),
  built("mean", "The arithmetic mean of the numbers.", &[
    named(&[p("list", "list<number>")], "number"),
    positional(&[rest("n", "number")], "number"),
  ]),
  built("median", "The median of the numbers.", &[
    named(&[p("list", "list<number>")], "number"),
    positional(&[rest("n", "number")], "number"),
  ]),
  built("meets", "Whether the first range ends where the second starts.", &[
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("met by", "Whether the first range starts where the second ends.", &[
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("min", "The least item.", &[
    named(&[p("list", "list")], "Any"),
    positional(&[rest("c", "Any")], "Any"),
  ]),
  built("mode", "The most frequent numbers.", &[
    named(&[p("list", "list<number>")], "list<number>"),
    positional(&[rest("n", "number")], "list<number>"),
  ]),
  built("modulo", "The remainder of the division, with the sign of the divisor.", &[
    named(&[p("dividend", "number"), p("divisor", "number")], "number"),
  ]),
  built("month of year", "The name of the month.", &[
    named(&[p("date", "date or date and time")], "string"),
  ]),
  built("not", "The negation of a boolean.", &[
    named(&[p("negand", "boolean")], "boolean"),
  ]),
  built("now", "The current date and time.", &[
    named(&[], "date and time"),
  ]),
  built("number", "A number from its text, with the separators it uses.", &[
    named(&[p("from", "string")], "number"),
    named(&[p("from", "string"), p("grouping separator", "string"), p("decimal separator", "string")], "number"),
  ]),
  built("odd", "Whether the number is odd.", &[
    named(&[p("number", "number")], "boolean"),
  ]),
  built("overlaps", "Whether the ranges have a point in common.", &[
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("overlaps after", "Whether the first range overlaps the second and ends after it.", &[
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("overlaps before", "Whether the first range overlaps the second and starts before it.", &[
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("product", "The product of the numbers.", &[
    named(&[p("list", "list<number>")], "number"),
    positional(&[rest("n", "number")], "number"),
  ]),
  built("range", "A range from its text.", &[
    named(&[p("from", "string")], "range"),
  ]),
  built("remove", "The list without the item at a position.", &[
    named(&[p("list", "list"), p("position", "number")], "list"),
  ]),
  built("replace", "The input with the matches of the regular expression replaced.", &[
    named(&[p("input", "string"), p("pattern", "string"), p("replacement", "string"), optional("flags", "string")], "string"),
  ]),
  built("reverse", "The list in reverse order.", &[
    named(&[p("list", "list")], "list"),
  ]),
  built("round down", "The number rounded towards zero.", &[
    named(&[p("n", "number"), optional("scale", "number")], "number"),
  ]),
  built("round half down", "The number rounded to the nearest, a tie towards zero.", &[
    named(&[p("n", "number"), optional("scale", "number")], "number"),
  ]),
  built("round half up", "The number rounded to the nearest, a tie away from zero.", &[
    named(&[p("n", "number"), optional("scale", "number")], "number"),
  ]),
  built("round up", "The number rounded away from zero.", &[
    named(&[p("n", "number"), optional("scale", "number")], "number"),
  ]),
  built("sort", "The list in the order of a function that says whether its first parameter precedes its second.", &[
    named(&[p("list", "list"), p("precedes", "function")], "list"),
  ]),
  built("split", "The parts of the string between the matches of the delimiter, a regular expression.", &[
    named(&[p("string", "string"), p("delimiter", "string")], "list<string>"),
  ]),
  built("sqrt", "The square root.", &[
    named(&[p("number", "number")], "number"),
  ]),
  built("started by", "Whether the range starts with the second.", &[
    named(&[p("range", "range"), p("point", "Any")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("starts", "Whether the first starts the range.", &[
    named(&[p("point", "Any"), p("range", "range")], "boolean"),
    named(&[p("range1", "range"), p("range2", "range")], "boolean"),
  ]),
  built("starts with", "Whether the string starts with the match.", &[
    named(&[p("string", "string"), p("match", "string")], "boolean"),
  ]),
  built("stddev", "The sample standard deviation of the numbers.", &[
    named(&[p("list", "list<number>")], "number"),
    positional(&[rest("n", "number")], "number"),
  ]),
  built("string", "The text of a value.", &[
    named(&[p("from", "Any")], "string"),
  ]),
  built("string join", "The strings joined, with a delimiter between them if one is given.", &[
    named(&[p("list", "list<string>"), optional("delimiter", "string")], "string"),
  ]),
  built("string length", "The number of characters.", &[
    named(&[p("string", "string")], "number"),
  ]),
  built("sublist", "The items from a position, as many as the length says or all that follow.", &[
    named(&[p("list", "list"), p("start position", "number"), optional("length", "number")], "list"),
  ]),
  built("substring", "The characters from a position, as many as the length says or all that follow.", &[
    named(&[p("string", "string"), p("start position", "number"), optional("length", "number")], "string"),
  ]),
  built("substring after", "The part of the string after the first match.", &[
    named(&[p("string", "string"), p("match", "string")], "string"),
  ]),
  built("substring before", "The part of the string before the first match.", &[
    named(&[p("string", "string"), p("match", "string")], "string"),
  ]),
  built("sum", "The sum of the numbers.", &[
    named(&[p("list", "list<number>")], "number"),
    positional(&[rest("n", "number")], "number"),
  ]),
  built("time", "A time from its text, from a date and time, or from an hour, a minute and a second.", &[
    named(&[p("from", "string or time or date and time")], "time"),
    named(&[p("hour", "number"), p("minute", "number"), p("second", "number"), optional("offset", "days and time duration")], "time"),
  ]),
  built("today", "The current date.", &[
    named(&[], "date"),
  ]),
  built("trim", "The string without the white space at its ends.", &[
    named(&[p("string", "string")], "string"),
  ]),
  built("union", "The items of the lists, each once.", &[
    positional(&[rest("list", "list")], "list"),
  ]),
  built("upper case", "The string in upper case.", &[
    named(&[p("string", "string")], "string"),
  ]),
  built("week of year", "The number of the week in its year.", &[
    named(&[p("date", "date or date and time")], "number"),
  ]),
  built("years and months duration", "A years and months duration from its text, or the years and months between two dates.", &[
    named(&[p("from", "string")], "years and months duration"),
    named(&[p("from", "date or date and time"), p("to", "date or date and time")], "years and months duration"),
  ]),
];

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bif::BUILT_IN_FUNCTION_NAMES;

  /// The table has every built-in function once, in the order of the names, and every
  /// function has a summary and a signature.
  #[test]
  fn every_built_in_function_has_signatures() {
    assert_eq!(BUILT_IN_FUNCTION_NAMES.to_vec(), BUILT_INS.iter().map(|built_in| built_in.name).collect::<Vec<_>>());
    for built_in in &BUILT_INS {
      assert!(!built_in.signatures.is_empty() && built_in.summary.ends_with('.'), "{}", built_in.name);
      for signature in built_in.signatures {
        // an optional parameter is followed by optional ones only, a rest parameter is the last
        let first_optional = signature.parameters.iter().position(|parameter| parameter.optional).unwrap_or(signature.parameters.len());
        assert!(signature.parameters[first_optional..].iter().all(|parameter| parameter.optional), "{}", built_in.name);
        assert!(signature.parameters.iter().rev().skip(1).all(|parameter| !parameter.rest), "{}", built_in.name);
        assert!(!signature.parameters.iter().any(|parameter| parameter.rest) || !signature.named, "{}", built_in.name);
      }
    }
    assert_eq!(
      "substring(string, start position, length?): string",
      built_in("substring").unwrap().signatures[0].text("substring")
    );
    assert_eq!("append(list, item...): list", built_in("append").unwrap().signatures[0].text("append"));
    assert_eq!("now(): date and time", built_in("now").unwrap().signatures[0].text("now"));
    assert!(built_in("nothing").is_none());
  }
}
