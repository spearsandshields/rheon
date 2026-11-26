use super::*;

from_examples!(DMN_3_1130);

static_context!(CTX, "{}");

fn eq(invocable: &str, ctx: &FeelContext, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, ctx, expected);
}

#[test]
fn _0001() {
  eq(
    "during",
    &CTX,
    r#"{duringPRT: true, duringPRF: false, duringPRT2: true, duringPRT3: true, duringPRF2: false, duringPRF3: false, duringRRT: true, duringRRT2: true, duringRRT3: true, duringRRT4: true, duringRRT5: true, duringRRT6: true, duringRRT7: true, duringRRT8: true}"#,
  );
}

#[test]
fn _0002() {
  eq(
    "after",
    &CTX,
    r#"{afterPPT: true, afterPPF: false, afterPRT: true, afterPRT2: true, afterPRF: false, afterRPF: false, afterRPT: true, afterRPT2: true, afterRPF2: false, afterRRT: true, afterRRF: false, afterRRT2: true, afterRRT3: true}"#,
  );
}

#[test]
fn _0003() {
  eq(
    "started by",
    &CTX,
    r#"{startedbyRPT: true, startedbyRPF: false, startedbyRPF2: false, startedbyRRT: true, startedbyRRT2: true, startedbyRRF: false, startedbyRRF2: false, startedbyRRT3: true, startedbyRRT4: true, startedbyRRT5: true}"#,
  );
}

#[test]
fn _0004() {
  eq(
    "includes",
    &CTX,
    r#"{includesRPT: true, includesRPF: false, includesRPT2: true, includesRPT3: true, includesRPF2: false, includesRPF3: false, includesRRT: true, includesRRT2: true, includesRRT3: true, includesRRT4: true, includesRRT5: true, includesRRT6: true, includesRRT7: true, includesRRT8: true}"#,
  );
}

#[test]
fn _0005() {
  eq("met by", &CTX, r#"{metbyRRT: true, metbyRRF: false, metbyRRF2: false, metbyRRF3: false}"#);
}

#[test]
fn _0006() {
  eq(
    "before",
    &CTX,
    r#"{beforePPT: true, beforPPF: false, beforePRF: false, beforePRT: true, beforePRT2: true, beforeRPF: false, beforeRPT: true, beforeRPT2: true, beforeRRT: true, beforeRRF: false, beforeRRT2: true, beforeRRT3: true}"#,
  );
}

#[test]
fn _0007() {
  eq(
    "overlaps",
    &CTX,
    r#"{overlapsRRT: true, overlapsRRT2: true, overlapsRRT3: true, overlapsRRT4: true, overlapsRRF: false, overlapsRRF2: false, overlapsRRT5: true, overlapsRRF3: false, overlapsRRF4: false, overlapsRRF5: false, overlapsRRT6: true, overlapsRRF6: false, overlapsRRF7: false, overlapsRRF8: false}"#,
  );
}

#[test]
fn _0008() {
  eq(
    "overlaps before",
    &CTX,
    r#"{overlapsbeforeRRT: true, overlapsbeforeRRF: false, overlapsbeforeRRT2: true, overlapsbeforeRRF2: false, overlapsbeforeRRF3: false, overlapsbeforeRRT3: true, overlapsbeforeRRT4: true, overlapsbeforeRRF4: false, overlapsbeforeRRF5: false}"#,
  );
}

#[test]
fn _0009() {
  eq(
    "finishes",
    &CTX,
    r#"{finishesPRT: true, finishesPRF: false, finishesRRT: true, finishesRRF: false, finishesRRT2: true, finishesRRF2: true, finishesRRT3: true}"#,
  );
}

#[test]
fn _0010() {
  eq(
    "finished by",
    &CTX,
    r#"{finishedbyRPT: true, finishedbyRPF: false, finishedbyRRT: true, finishedbyRRF: false, finishedbyRRT2: true, finishedbyRRT3: true, finishedbyRRT4: true}"#,
  );
}

#[test]
fn _0011() {
  eq(
    "starts",
    &CTX,
    r#"{startsPRT: true, startsPRF: false, startsPRF2: false, startsRRT: true, startsRRT2: true, startsRRF: false, startsRRF2: false, startsRRT3: true, startsRRT4: true, startsRRT5: true}"#,
  );
}

#[test]
fn _0012() {
  eq(
    "coincides",
    &CTX,
    r#"{coincidesPPT: true, coincidesPPF: false, coincidesRRT: true, coincidesRRF: false, coincidesRRF2: false}"#,
  );
}

#[test]
fn _0013() {
  eq(
    "overlaps after",
    &CTX,
    r#"{overlapsafterRRT: true, overlapsafterRRF: false, overlapsafterRRT2: true, overlapsafterRRF2: false, overlapsafterRRF3: false, overlapsafterRRT3: true, overlapsafterRRT4: true, overlapsafterRRF4: false, overlapsafterRRF5: false}"#,
  );
}

#[test]
fn _0014() {
  eq("meets", &CTX, r#"{meetsRRT: true, meetsRRF: false, meetsRRF2: false, meetsRRF3: false}"#);
}
