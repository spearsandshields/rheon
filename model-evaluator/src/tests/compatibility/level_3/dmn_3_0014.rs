use super::*;

from_examples!(DMN_3_0014);

#[test]
fn _0001() {
  let ctx = context(r#"{p: 1, r: 1, n: 1, pmt: 1}"#);
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "equity36Mo", &ctx, r#"1"#);
}

#[test]
fn _0002() {
  let ctx = context(r#"{p: 2, r: 1, n: 1, pmt: 1}"#);
  assert_business_knowledge_model(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "equity36Mo",
    &ctx,
    r#"2.083333333333333333333333333333333"#,
  );
}

#[test]
fn _0003() {
  let ctx = context(r#"{p: 1, r: 1, n: 1}"#);
  assert_business_knowledge_model(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "monthlyPayment",
    &ctx,
    r#"1.083333333333333333333333333333338"#,
  );
}

#[test]
fn _0004() {
  let ctx = context(r#"{p: 2, r: 1, n: 1}"#);
  assert_business_knowledge_model(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "monthlyPayment",
    &ctx,
    r#"2.166666666666666666666666666666676"#,
  );
}

#[test]
fn _0005() {
  let ctx = context(r#"{requestedAmt: 330000, product: {fee: 0, lenderName: "Oceans Capital", points: 0, rate: 0.03500}}"#);
  assert_business_knowledge_model(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "FinancialMetrics",
    &ctx,
    r#"{lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0, loanAmt: 330000, downPmtAmt: 66000, paymentAmt: 1481.847469769120902911415325410838, equity36moPct: 0.1229130806675864888391782030891032}"#,
  );
}

#[test]
fn _0006() {
  let ctx = context(r#"{}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "Bankrates",
    &ctx,
    r#"[{lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0}, {lenderName: "eClick Lending", rate: 0.032, points: 1.1, fee: 2700}, {lenderName: "eClickLending", rate: 0.03375, points: 0.1, fee: 1200}, {lenderName: "AimLoan", rate: 0.03, points: 1.1, fee: 3966}, {lenderName: "Home Loans Today", rate: 0.03125, points: 1.1, fee: 285}, {lenderName: "Sebonic", rate: 0.03125, points: 0.1, fee: 4028}, {lenderName: "AimLoan", rate: 0.03125, points: 0.1, fee: 4317}, {lenderName: "eRates Mortgage", rate: 0.03125, points: 1.1, fee: 2518}, {lenderName: "Home Loans Today", rate: 0.0325, points: 0.1, fee: 822}, {lenderName: "AimLoan", rate: 0.0325, points: 0, fee: 1995}]"#,
  );
}

#[test]
fn _0007() {
  let ctx = context(r#"{RequestedAmt: 330000}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "RankedProducts",
    &ctx,
    r#"{metricsTable: [{lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0, loanAmt: 330000, downPmtAmt: 66000, paymentAmt: 1481.847469769120902911415325410838, equity36moPct: 0.1229130806675864888391782030891032}, {lenderName: "eClick Lending", rate: 0.032, points: 1.1, fee: 2700, loanAmt: 336330, downPmtAmt: 67266, paymentAmt: 1454.515807764692215596585144120545, equity36moPct: 0.1137029731874587934446846162699216}, {lenderName: "eClickLending", rate: 0.03375, points: 0.1, fee: 1200, loanAmt: 331530, downPmtAmt: 66306, paymentAmt: 1465.681565899863510819264705552193, equity36moPct: 0.1219807467513805938540322263850624}, {lenderName: "AimLoan", rate: 0.03, points: 1.1, fee: 3966, loanAmt: 337596, downPmtAmt: 67519.2, paymentAmt: 1423.31835370927550454489017290607, equity36moPct: 0.1154298007315239099518616327317256}, {lenderName: "Home Loans Today", rate: 0.03125, points: 1.1, fee: 285, loanAmt: 333915, downPmtAmt: 66783, paymentAmt: 1430.409890005738806923767229688247, equity36moPct: 0.1219478687825970483711604225077008}, {lenderName: "Sebonic", rate: 0.03125, points: 0.1, fee: 4028, loanAmt: 334358, downPmtAmt: 66871.6, paymentAmt: 1432.307593257382315875048929769861, equity36moPct: 0.120782970248151726934352923794468}, {lenderName: "AimLoan", rate: 0.03125, points: 0.1, fee: 4317, loanAmt: 334647, downPmtAmt: 66929.4, paymentAmt: 1433.545598313194898464034056911139, equity36moPct: 0.1200230251545745307825755713607792}, {lenderName: "eRates Mortgage", rate: 0.03125, points: 1.1, fee: 2518, loanAmt: 336148, downPmtAmt: 67229.6, paymentAmt: 1439.975513845287239177067537323106, equity36moPct: 0.1160760438900092317382233014543176}, {lenderName: "Home Loans Today", rate: 0.0325, points: 0.1, fee: 822, loanAmt: 331152, downPmtAmt: 66230.4, paymentAmt: 1441.194429734569798816940611550485, equity36moPct: 0.1261025270361081947340170865041632}, {lenderName: "AimLoan", rate: 0.0325, points: 0, fee: 1995, loanAmt: 331995, downPmtAmt: 66399, paymentAmt: 1444.863219004349967260442933552276, equity36moPct: 0.1238778822515121156167560595555808}], rankByRate: [{lenderName: "AimLoan", rate: 0.03, points: 1.1, fee: 3966, loanAmt: 337596, downPmtAmt: 67519.2, paymentAmt: 1423.31835370927550454489017290607, equity36moPct: 0.1154298007315239099518616327317256}, {lenderName: "Home Loans Today", rate: 0.03125, points: 1.1, fee: 285, loanAmt: 333915, downPmtAmt: 66783, paymentAmt: 1430.409890005738806923767229688247, equity36moPct: 0.1219478687825970483711604225077008}, {lenderName: "Sebonic", rate: 0.03125, points: 0.1, fee: 4028, loanAmt: 334358, downPmtAmt: 66871.6, paymentAmt: 1432.307593257382315875048929769861, equity36moPct: 0.120782970248151726934352923794468}, {lenderName: "AimLoan", rate: 0.03125, points: 0.1, fee: 4317, loanAmt: 334647, downPmtAmt: 66929.4, paymentAmt: 1433.545598313194898464034056911139, equity36moPct: 0.1200230251545745307825755713607792}, {lenderName: "eRates Mortgage", rate: 0.03125, points: 1.1, fee: 2518, loanAmt: 336148, downPmtAmt: 67229.6, paymentAmt: 1439.975513845287239177067537323106, equity36moPct: 0.1160760438900092317382233014543176}, {lenderName: "eClick Lending", rate: 0.032, points: 1.1, fee: 2700, loanAmt: 336330, downPmtAmt: 67266, paymentAmt: 1454.515807764692215596585144120545, equity36moPct: 0.1137029731874587934446846162699216}, {lenderName: "Home Loans Today", rate: 0.0325, points: 0.1, fee: 822, loanAmt: 331152, downPmtAmt: 66230.4, paymentAmt: 1441.194429734569798816940611550485, equity36moPct: 0.1261025270361081947340170865041632}, {lenderName: "AimLoan", rate: 0.0325, points: 0, fee: 1995, loanAmt: 331995, downPmtAmt: 66399, paymentAmt: 1444.863219004349967260442933552276, equity36moPct: 0.1238778822515121156167560595555808}, {lenderName: "eClickLending", rate: 0.03375, points: 0.1, fee: 1200, loanAmt: 331530, downPmtAmt: 66306, paymentAmt: 1465.681565899863510819264705552193, equity36moPct: 0.1219807467513805938540322263850624}, {lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0, loanAmt: 330000, downPmtAmt: 66000, paymentAmt: 1481.847469769120902911415325410838, equity36moPct: 0.1229130806675864888391782030891032}], rankByDownPmt: [{lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0, loanAmt: 330000, downPmtAmt: 66000, paymentAmt: 1481.847469769120902911415325410838, equity36moPct: 0.1229130806675864888391782030891032}, {lenderName: "Home Loans Today", rate: 0.0325, points: 0.1, fee: 822, loanAmt: 331152, downPmtAmt: 66230.4, paymentAmt: 1441.194429734569798816940611550485, equity36moPct: 0.1261025270361081947340170865041632}, {lenderName: "eClickLending", rate: 0.03375, points: 0.1, fee: 1200, loanAmt: 331530, downPmtAmt: 66306, paymentAmt: 1465.681565899863510819264705552193, equity36moPct: 0.1219807467513805938540322263850624}, {lenderName: "AimLoan", rate: 0.0325, points: 0, fee: 1995, loanAmt: 331995, downPmtAmt: 66399, paymentAmt: 1444.863219004349967260442933552276, equity36moPct: 0.1238778822515121156167560595555808}, {lenderName: "Home Loans Today", rate: 0.03125, points: 1.1, fee: 285, loanAmt: 333915, downPmtAmt: 66783, paymentAmt: 1430.409890005738806923767229688247, equity36moPct: 0.1219478687825970483711604225077008}, {lenderName: "Sebonic", rate: 0.03125, points: 0.1, fee: 4028, loanAmt: 334358, downPmtAmt: 66871.6, paymentAmt: 1432.307593257382315875048929769861, equity36moPct: 0.120782970248151726934352923794468}, {lenderName: "AimLoan", rate: 0.03125, points: 0.1, fee: 4317, loanAmt: 334647, downPmtAmt: 66929.4, paymentAmt: 1433.545598313194898464034056911139, equity36moPct: 0.1200230251545745307825755713607792}, {lenderName: "eRates Mortgage", rate: 0.03125, points: 1.1, fee: 2518, loanAmt: 336148, downPmtAmt: 67229.6, paymentAmt: 1439.975513845287239177067537323106, equity36moPct: 0.1160760438900092317382233014543176}, {lenderName: "eClick Lending", rate: 0.032, points: 1.1, fee: 2700, loanAmt: 336330, downPmtAmt: 67266, paymentAmt: 1454.515807764692215596585144120545, equity36moPct: 0.1137029731874587934446846162699216}, {lenderName: "AimLoan", rate: 0.03, points: 1.1, fee: 3966, loanAmt: 337596, downPmtAmt: 67519.2, paymentAmt: 1423.31835370927550454489017290607, equity36moPct: 0.1154298007315239099518616327317256}], rankByMonthlyPmt: [{lenderName: "AimLoan", rate: 0.03, points: 1.1, fee: 3966, loanAmt: 337596, downPmtAmt: 67519.2, paymentAmt: 1423.31835370927550454489017290607, equity36moPct: 0.1154298007315239099518616327317256}, {lenderName: "Home Loans Today", rate: 0.03125, points: 1.1, fee: 285, loanAmt: 333915, downPmtAmt: 66783, paymentAmt: 1430.409890005738806923767229688247, equity36moPct: 0.1219478687825970483711604225077008}, {lenderName: "Sebonic", rate: 0.03125, points: 0.1, fee: 4028, loanAmt: 334358, downPmtAmt: 66871.6, paymentAmt: 1432.307593257382315875048929769861, equity36moPct: 0.120782970248151726934352923794468}, {lenderName: "AimLoan", rate: 0.03125, points: 0.1, fee: 4317, loanAmt: 334647, downPmtAmt: 66929.4, paymentAmt: 1433.545598313194898464034056911139, equity36moPct: 0.1200230251545745307825755713607792}, {lenderName: "eRates Mortgage", rate: 0.03125, points: 1.1, fee: 2518, loanAmt: 336148, downPmtAmt: 67229.6, paymentAmt: 1439.975513845287239177067537323106, equity36moPct: 0.1160760438900092317382233014543176}, {lenderName: "Home Loans Today", rate: 0.0325, points: 0.1, fee: 822, loanAmt: 331152, downPmtAmt: 66230.4, paymentAmt: 1441.194429734569798816940611550485, equity36moPct: 0.1261025270361081947340170865041632}, {lenderName: "AimLoan", rate: 0.0325, points: 0, fee: 1995, loanAmt: 331995, downPmtAmt: 66399, paymentAmt: 1444.863219004349967260442933552276, equity36moPct: 0.1238778822515121156167560595555808}, {lenderName: "eClick Lending", rate: 0.032, points: 1.1, fee: 2700, loanAmt: 336330, downPmtAmt: 67266, paymentAmt: 1454.515807764692215596585144120545, equity36moPct: 0.1137029731874587934446846162699216}, {lenderName: "eClickLending", rate: 0.03375, points: 0.1, fee: 1200, loanAmt: 331530, downPmtAmt: 66306, paymentAmt: 1465.681565899863510819264705552193, equity36moPct: 0.1219807467513805938540322263850624}, {lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0, loanAmt: 330000, downPmtAmt: 66000, paymentAmt: 1481.847469769120902911415325410838, equity36moPct: 0.1229130806675864888391782030891032}], rankByEquityPct: [{lenderName: "Home Loans Today", rate: 0.0325, points: 0.1, fee: 822, loanAmt: 331152, downPmtAmt: 66230.4, paymentAmt: 1441.194429734569798816940611550485, equity36moPct: 0.1261025270361081947340170865041632}, {lenderName: "AimLoan", rate: 0.0325, points: 0, fee: 1995, loanAmt: 331995, downPmtAmt: 66399, paymentAmt: 1444.863219004349967260442933552276, equity36moPct: 0.1238778822515121156167560595555808}, {lenderName: "Oceans Capital", rate: 0.035, points: 0, fee: 0, loanAmt: 330000, downPmtAmt: 66000, paymentAmt: 1481.847469769120902911415325410838, equity36moPct: 0.1229130806675864888391782030891032}, {lenderName: "eClickLending", rate: 0.03375, points: 0.1, fee: 1200, loanAmt: 331530, downPmtAmt: 66306, paymentAmt: 1465.681565899863510819264705552193, equity36moPct: 0.1219807467513805938540322263850624}, {lenderName: "Home Loans Today", rate: 0.03125, points: 1.1, fee: 285, loanAmt: 333915, downPmtAmt: 66783, paymentAmt: 1430.409890005738806923767229688247, equity36moPct: 0.1219478687825970483711604225077008}, {lenderName: "Sebonic", rate: 0.03125, points: 0.1, fee: 4028, loanAmt: 334358, downPmtAmt: 66871.6, paymentAmt: 1432.307593257382315875048929769861, equity36moPct: 0.120782970248151726934352923794468}, {lenderName: "AimLoan", rate: 0.03125, points: 0.1, fee: 4317, loanAmt: 334647, downPmtAmt: 66929.4, paymentAmt: 1433.545598313194898464034056911139, equity36moPct: 0.1200230251545745307825755713607792}, {lenderName: "eRates Mortgage", rate: 0.03125, points: 1.1, fee: 2518, loanAmt: 336148, downPmtAmt: 67229.6, paymentAmt: 1439.975513845287239177067537323106, equity36moPct: 0.1160760438900092317382233014543176}, {lenderName: "AimLoan", rate: 0.03, points: 1.1, fee: 3966, loanAmt: 337596, downPmtAmt: 67519.2, paymentAmt: 1423.31835370927550454489017290607, equity36moPct: 0.1154298007315239099518616327317256}, {lenderName: "eClick Lending", rate: 0.032, points: 1.1, fee: 2700, loanAmt: 336330, downPmtAmt: 67266, paymentAmt: 1454.515807764692215596585144120545, equity36moPct: 0.1137029731874587934446846162699216}]}"#,
  );
}
