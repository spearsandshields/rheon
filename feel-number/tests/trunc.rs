mod common;

use rheon_feel_number::FeelNumber;

#[test]
fn test_trunc_001() {
  eqs!("12345", num!(12345.6789).trunc());
}
