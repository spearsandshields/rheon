mod common;

use dsntk_common::Jsonify;
use rheon_feel_number::FeelNumber;

#[test]
fn test_jsonify_001() {
  eqs!("12345.6789", num!(12345.6789).jsonify());
}
