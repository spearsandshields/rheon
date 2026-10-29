mod common;

use dsntk_common::Jsonify;
use flowstone_feel_number::FeelNumber;

#[test]
fn test_jsonify_001() {
  eqs!("12345.6789", num!(12345.6789).jsonify());
}
