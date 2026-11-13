use crate::bip::{BUILT_IN_PROPERTY_NAMES, Bip, is_built_in_property_name};

#[test]
fn _0001() {
  // every name of the list is a property of the enumeration, and is displayed as itself
  for name in BUILT_IN_PROPERTY_NAMES {
    assert!(is_built_in_property_name(name), "{name}");
    assert!(!matches!(Bip::new(name), Bip::Unknown(_)), "{name}");
    assert_eq!(name, Bip::new(name).to_string());
  }
}

#[test]
fn _0002() {
  for name in ["", "Start", "start ", "startincluded", "time  offset", "values", "x"] {
    assert!(!is_built_in_property_name(name), "{name}");
    assert!(matches!(Bip::new(name), Bip::Unknown(_)), "{name}");
  }
}
