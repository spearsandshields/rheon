use crate::{FeelDate, FeelYearsAndMonthsDuration};

#[test]
fn _0001() {
  let date_a = &FeelDate::new(2023, 2, 8).unwrap();
  let date_b = &FeelDate::new(2021, 2, 8).unwrap();
  let result = (date_a - date_b).unwrap();
  assert_eq!("P2Y", result.to_string());
}

#[test]
fn _0002() {
  let date_a = &FeelDate::new(2021, 2, 8).unwrap();
  let date_b = &FeelDate::new(2023, 2, 8).unwrap();
  let result = (date_a - date_b).unwrap();
  assert_eq!("-P2Y", result.to_string());
}

#[test]
fn _0003() {
  let date_a = &FeelDate::new(2021, 2, 9).unwrap();
  let date_b = &FeelDate::new(2023, 3, 8).unwrap();
  let result = (date_a - date_b).unwrap();
  assert_eq!("-P2Y", result.to_string());
}

/// The months between two dates within one year, earlier minus later, follow the rule of every
/// other span: whole months, one fewer when the day of the later date is below the day of the
/// earlier one.
#[test]
fn a_negative_span_inside_one_year() {
  let months = |a: (i32, u32, u32), b: (i32, u32, u32)| {
    let (a, b) = (FeelDate::new(a.0, a.1, a.2).unwrap(), FeelDate::new(b.0, b.1, b.2).unwrap());
    (&a - &b).unwrap().to_string()
  };
  assert_eq!("-P2M", months((2020, 3, 1), (2020, 5, 15)));
  assert_eq!("P0M", months((2020, 5, 1), (2020, 5, 15)));
  assert_eq!("-P1M", months((2020, 3, 15), (2020, 5, 1)));
  assert_eq!("-P1M", months((2020, 3, 20), (2020, 5, 15)));
  assert_eq!("P2M", months((2020, 5, 15), (2020, 3, 1)));
}

/// Over every pair of a grid of dates (two years, every month, the days 1, 15, 28 and 31 where
/// they exist), the months between two dates are antisymmetric, and for an earlier date `a` with
/// a day that no month lacks, `a` plus the months between `a` and `b` is the last such date not
/// after `b`.
#[test]
fn the_months_between_two_dates_have_one_rule_for_both_signs() {
  let mut dates = vec![];
  for year in [2020, 2021] {
    for month in 1..=12 {
      for day in [1, 15, 28, 31] {
        if let Some(date) = FeelDate::new(year, month, day) {
          dates.push(date);
        }
      }
    }
  }
  for a in &dates {
    for b in &dates {
      let forward = (b - a).unwrap();
      let backward = (a - b).unwrap();
      assert_eq!(forward.as_months(), -backward.as_months(), "{b} - {a} and {a} - {b}");
      if a <= b && a.day() <= 28 {
        let months = forward.as_months();
        let at = |m: i64| a.clone() + FeelYearsAndMonthsDuration::from_m(m).unwrap();
        assert!(at(months).unwrap() <= *b, "{a} + {months} months is after {b}");
        assert!(at(months + 1).unwrap() > *b, "{a} + {} months is not after {b}", months + 1);
      }
    }
  }
}
