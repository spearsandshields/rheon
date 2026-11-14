#![feature(test)]

extern crate test;

use rheon_feel_number::{FeelNumber, RoundingMode};
use test::Bencher;

#[bench]
fn bench_round_half_down_0001(b: &mut Bencher) {
  let x = FeelNumber::new(54, 1);
  assert_eq!("5", x.round_to(0, RoundingMode::HalfDown).unwrap().to_string());
  b.iter(|| x.round_to(0, RoundingMode::HalfDown).unwrap())
}

#[bench]
fn bench_round_half_down_0002(b: &mut Bencher) {
  let x = FeelNumber::new(55, 1);
  assert_eq!("5", x.round_to(0, RoundingMode::HalfDown).unwrap().to_string());
  b.iter(|| x.round_to(0, RoundingMode::HalfDown).unwrap())
}

#[bench]
fn bench_round_half_down_0003(b: &mut Bencher) {
  let x = FeelNumber::new(56, 1);
  assert_eq!("6", x.round_to(0, RoundingMode::HalfDown).unwrap().to_string());
  b.iter(|| x.round_to(0, RoundingMode::HalfDown).unwrap())
}
