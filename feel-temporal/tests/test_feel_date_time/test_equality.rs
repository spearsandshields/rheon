use rheon_feel_temporal::FeelDateTime;

#[test]
fn _0001() {
  let a: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0002() {
  let a: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00@Etc/UTC".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0003() {
  let a: FeelDateTime = "2025-02-09T00:00:00@Etc/UTC".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0004() {
  let a: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00+00:00".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0005() {
  let a: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00+01:00".try_into().unwrap();
  assert_ne!(a, b);
}

#[test]
fn _0006() {
  let a: FeelDateTime = "2025-02-09T00:00:00+01:00".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00Z".try_into().unwrap();
  assert_ne!(a, b);
}

#[test]
fn _0007() {
  let a: FeelDateTime = "2025-02-09T00:00:00".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0008() {
  let a: FeelDateTime = "2025-02-09T00:00:00+01:00".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00+01:00".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0009() {
  let a: FeelDateTime = "2018-10-08T00:00:00+02:00".try_into().unwrap();
  let b: FeelDateTime = "2018-10-08T00:00:00@Europe/Paris".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0010() {
  let a: FeelDateTime = "2018-10-08T00:00:00@Europe/Paris".try_into().unwrap();
  let b: FeelDateTime = "2018-10-08T00:00:00+02:00".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0011() {
  let a: FeelDateTime = "2025-02-09T00:00:00+01:00".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00@Europe/Paris".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0012() {
  let a: FeelDateTime = "2025-02-09T00:00:00@Europe/Paris".try_into().unwrap();
  let b: FeelDateTime = "2025-02-09T00:00:00+01:00".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0013() {
  let a: FeelDateTime = "2021-04-02T23:00:00@Australia/Melbourne".try_into().unwrap();
  let b: FeelDateTime = "2021-04-02T23:00:00+11:00".try_into().unwrap();
  assert_eq!(a, b);
}

#[test]
fn _0014() {
  let a: FeelDateTime = "2021-10-02T23:00:00@Australia/Melbourne".try_into().unwrap();
  let b: FeelDateTime = "2021-10-02T23:00:00+10:00".try_into().unwrap();
  assert_eq!(a, b);
}

/// A date and time that its zone has no instant for (the hour that a clock change skips) or
/// two instants for (the hour that it repeats) equals itself, and no value written otherwise.
#[test]
fn a_date_and_time_without_one_instant_equals_itself() {
  let of = |text: &str| -> FeelDateTime { text.try_into().unwrap() };
  for text in [
    "2024-03-31T02:30:00@Europe/Paris",
    "2024-10-27T02:30:00@Europe/Paris",
    "2024-03-10T02:00:00@America/New_York",
    "2024-11-03T01:59:59@America/New_York",
  ] {
    assert_eq!(of(text), of(text), "{text}");
    assert_eq!(Some(std::cmp::Ordering::Equal), of(text).partial_cmp(&of(text)), "{text}");
  }
  let skipped = of("2024-03-31T02:30:00@Europe/Paris");
  for other in [
    "2024-03-31T02:30:01@Europe/Paris",
    "2024-03-31T03:30:00@Europe/Paris",
    "2024-03-30T02:30:00@Europe/Paris",
    "2024-03-31T02:30:00@Europe/Berlin",
    "2024-03-31T02:30:00+01:00",
    "2024-03-31T01:30:00Z",
    "2024-03-31T00:30:00Z",
    "2024-03-31T02:30:00",
  ] {
    assert_ne!(skipped, of(other), "{other}");
    assert_ne!(of(other), skipped, "{other}");
  }
  // a date and time that has one instant is compared by it, as before
  assert_eq!(of("2024-03-31T03:30:00@Europe/Paris"), of("2024-03-31T01:30:00Z"));
  assert_eq!(of("2024-03-31T01:30:00@Europe/Paris"), of("2024-03-31T00:30:00Z"));
}

/// A date and time without an offset has an order against one with an offset only when the
/// two are more than 14 hours apart, on one day or on two.
#[test]
fn a_local_date_and_time_is_ordered_against_a_zoned_one_only_beyond_14_hours() {
  use std::cmp::Ordering::{Greater, Less};
  let of = |text: &str| -> FeelDateTime { text.try_into().unwrap() };
  for (local, zoned, expected) in [
    ("2026-10-17T10:00:00", "2026-10-17T11:00:00Z", None),
    ("2026-10-17T10:00:00", "2026-10-18T00:00:00Z", None),
    ("2026-10-17T10:00:00", "2026-10-18T00:00:01Z", Some(Less)),
    ("2026-10-17T10:00:00", "2026-10-18T11:00:00Z", Some(Less)),
    ("2026-10-17T10:00:00", "2026-10-16T20:00:00Z", None),
    ("2026-10-17T10:00:00", "2026-10-16T19:59:59Z", Some(Greater)),
    ("2026-10-17T10:00:00", "2025-10-17T10:00:00+02:00", Some(Greater)),
    ("2026-10-17T00:00:00", "2026-10-17T23:59:59Z", Some(Less)),
  ] {
    assert_eq!(expected, of(local).partial_cmp(&of(zoned)), "{local} {zoned}");
    assert_eq!(expected.map(std::cmp::Ordering::reverse), of(zoned).partial_cmp(&of(local)), "{zoned} {local}");
  }
}
