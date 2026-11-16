/// Builds `null` value with a message informing about invalid argument type.
/// The type of `$value` is quoted from the value ([TypeOf](rheon_feel::values::TypeOf)), so a
/// whole type is never built for the message.
///
/// The message names the module of the call site by its file stem (`[core::abs] …`); the form
/// `invalid_argument_type!(module "builders", …)` names the given module instead, so code that
/// moves to another file keeps its messages.
macro_rules! invalid_argument_type {
  (module $module:literal, $function:expr, $expected:expr, $value:expr) => {{
    use rheon_feel::message::arg;
    use rheon_feel::value_null;
    use rheon_feel::values::TypeOf;
    value_null!(
      $module,
      $function,
      "invalid argument type, expected {}, actual type is {}",
      $expected,
      arg(&TypeOf(&$value))
    )
  }};
  ($function:expr, $expected:expr, $value:expr) => {{
    use rheon_feel::message::arg;
    use rheon_feel::value_null;
    use rheon_feel::values::TypeOf;
    use std::file;
    use std::path::Path;
    value_null!(
      Path::new(file!()).file_stem().unwrap().to_string_lossy(),
      $function,
      "invalid argument type, expected {}, actual type is {}",
      $expected,
      arg(&TypeOf(&$value))
    )
  }};
}

pub(crate) use invalid_argument_type;

/// Builds `null` value with a message informing about invalid parameter count.
macro_rules! invalid_number_of_parameters {
  ($expected:expr, $actual:expr) => {{
    use rheon_feel::value_null;
    value_null!("expected {} parameters, actual number of parameters is {}", $expected, $actual)
  }};
}

pub(crate) use invalid_number_of_parameters;
