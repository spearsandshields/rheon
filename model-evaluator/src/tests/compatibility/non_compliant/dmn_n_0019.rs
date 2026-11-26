use super::*;

from_examples!(DMN_N_0019);

static_context!(
  CTX,
  r#"{
    "Flight List": [
      { "Flight Number": "UA123",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T18:00:00", "Arrival": @"2017-01-01T19:00:00", "Capacity": 5, "Status": "cancelled" },
      { "Flight Number": "UA456",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T19:00:00", "Arrival": @"2017-01-01T20:00:00", "Capacity": 2, "Status": "scheduled" },
      { "Flight Number": "UA789",  "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T21:00:00", "Arrival": @"2017-01-01T23:00:00", "Capacity": 2, "Status": "scheduled" },
      { "Flight Number": "UA1001", "From": "SFO", "To": "SNA", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 0, "Status": "scheduled" },
      { "Flight Number": "UA1111", "From": "SFO", "To": "LAX", "Departure": @"2017-01-01T23:00:00", "Arrival": @"2017-01-02T05:00:00", "Capacity": 2, "Status": "scheduled" }
    ],
    "Passenger List": [
      { "Name": "Tom",   "Status": "bronze", "Miles": 10,     "Flight Number": "UA123" },
      { "Name": "Igor",  "Status": "gold",   "Miles": 50000,  "Flight Number": "UA123" },
      { "Name": "Jenny", "Status": "gold",   "Miles": 500000, "Flight Number": "UA123" },
      { "Name": "Harry", "Status": "gold",   "Miles": 100000, "Flight Number": "UA123" },
      { "Name": "Dick",  "Status": "silver", "Miles": 100,    "Flight Number": "UA123" }
    ]
  }"#
);

#[test]
fn _0001() {
  let invocable_name = "Rebooked Passengers";
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    invocable_name,
    &CTX,
    r#"[{Name: "Jenny", Status: "gold", Miles: 500000, Flight Number: "UA456"}, {Name: "Harry", Status: "gold", Miles: 100000, Flight Number: "UA456"}, {Name: "Igor", Status: "gold", Miles: 50000, Flight Number: "UA789"}, {Name: "Dick", Status: "silver", Miles: 100, Flight Number: "UA789"}, {Name: "Tom", Status: "bronze", Miles: 10, Flight Number: null}]"#,
  );
}

#[test]
fn _0002() {
  let invocable_name = "Prioritized Waiting List";
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    invocable_name,
    &CTX,
    r#"[{Name: "Jenny", Status: "gold", Miles: 500000, Flight Number: "UA123"}, {Name: "Harry", Status: "gold", Miles: 100000, Flight Number: "UA123"}, {Name: "Igor", Status: "gold", Miles: 50000, Flight Number: "UA123"}, {Name: "Dick", Status: "silver", Miles: 100, Flight Number: "UA123"}, {Name: "Tom", Status: "bronze", Miles: 10, Flight Number: "UA123"}]"#,
  );
}
