//! #1406 C4b：per_leg 缺来源归属只能 Unmatched，不能改走净额比例。
use super::book::{array, hash, integer};
use super::store::Config;
use crate::session_protocol::{nonnegative, required};
use serde_json::{json, Value};

pub(super) const POLICY: &str = "per-leg-source-attribution/1";

pub(super) fn decode(
    event: &Value,
    pointer: &str,
    check: &dyn Fn(&str, &str, &Value) -> Result<(), String>,
) -> Result<(), String> {
    super::voice::exact(
        event,
        &[
            "event",
            "allocation_id",
            "source",
            "stable_point",
            "per_leg_report",
        ],
    )?;
    let r = &event["per_leg_report"];
    super::voice::exact(
        r,
        &[
            "manifest_hash",
            "symbol",
            "reported_quantity",
            "unit_price",
            "commission",
            "leg_attribution",
            "source_ref",
        ],
    )?;
    required(r, "manifest_hash")?;
    required(r, "symbol")?;
    for field in ["reported_quantity", "unit_price", "commission"] {
        nonnegative(required(r, field)?, field)?;
    }
    if !r["leg_attribution"].is_null() && !r["leg_attribution"].is_object() {
        return Err("SchemaUnsupported：来源逐腿归属须为对象或显式 null".into());
    }
    if let Some(attribution) = r["leg_attribution"].as_object() {
        for (chong_id, leg) in attribution {
            if chong_id.is_empty() {
                return Err("SchemaUnsupported：来源逐腿归属的重标识不得为空".into());
            }
            super::voice::exact(leg, &["units", "acquisition_cash", "commission"])?;
            for field in ["units", "acquisition_cash", "commission"] {
                nonnegative(required(leg, field)?, field)?;
            }
        }
    }
    check(
        required(r, "source_ref")?,
        &format!("{pointer}/per_leg_report"),
        r,
    )?;
    let sp = &event["stable_point"];
    super::voice::exact(sp, &["profile", "seq", "source_ref"])?;
    if sp["profile"] != "TestOnly" {
        return Err("SchemaUnsupported：稳定点仅 TestOnly".into());
    }
    nonnegative(required(sp, "seq")?, "stable_point.seq")?;
    check(
        required(sp, "source_ref")?,
        &format!("{pointer}/stable_point"),
        sp,
    )
}

pub(super) fn resolve(
    config: &Config,
    next: &mut Value,
    fact: &Value,
    m: &Value,
    evidence_id: &str,
) -> Result<(), String> {
    let event = &fact["event"];
    let policy = &config.manifest["fill_allocation_policy"];
    let report = &event["per_leg_report"];
    let participants = array(m, "participants")?;
    let attribution = &report["leg_attribution"];
    let attributed = attribution.as_object();
    if let Some(legs) = attributed {
        let expected = participants
            .iter()
            .map(|p| required(p, "chong_id"))
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        if legs.len() != expected.len() || legs.keys().any(|id| !expected.contains(id.as_str())) {
            return Err("IdentityConflict：来源逐腿归属必须恰好覆盖冻结参与重".into());
        }
    }
    let unit_price = integer(report, "unit_price")?;
    let mut total_units = 0i64;
    let mut total_cash = 0i64;
    let mut total_commission = 0i64;
    let mut routes = Vec::new();
    for p in participants {
        let id = format!(
            "per-leg:{}",
            hash(&json!([
                fact["fact_key"],
                event["allocation_id"],
                m["manifest_hash"],
                p
            ]))
        );
        let mut route = json!({"allocation_id":id,"chong_id":p["chong_id"],"family_id":p["family_id"],"command_id":p["command_id"],"revision":p["revision"],"manifest_hash":m["manifest_hash"],"fill_allocation_policy_id":policy["fill_allocation_policy_id"],"policy_revision":policy["policy_revision"]});
        if let Some(legs) = attributed {
            let leg = legs
                .get(required(p, "chong_id")?)
                .ok_or("IdentityConflict：来源逐腿归属缺冻结参与重")?;
            let units = integer(leg, "units")?;
            let cash = integer(leg, "acquisition_cash")?;
            let commission = integer(leg, "commission")?;
            let expected_cash = units
                .checked_mul(unit_price)
                .ok_or("InvalidDomain：逐腿成交金额溢出")?;
            if cash != expected_cash {
                return Err("IdentityConflict：逐腿归属金额与数量/单价不符".into());
            }
            if units == 0 && commission != 0 {
                return Err("IdentityConflict：零数量逐腿归属不得携带现金或费用".into());
            }
            total_units = total_units
                .checked_add(units)
                .ok_or("InvalidDomain：逐腿归属数量合计溢出")?;
            total_cash = total_cash
                .checked_add(cash)
                .ok_or("InvalidDomain：逐腿归属金额合计溢出")?;
            total_commission = total_commission
                .checked_add(commission)
                .ok_or("InvalidDomain：逐腿归属费用合计溢出")?;
            route["units"] = json!(units.to_string());
            route["acquisition_cash"] = json!(cash.to_string());
            route["commission"] = json!(commission.to_string());
        } else {
            route["unmatched_reason"] = json!("per_leg_attribution_missing");
        }
        routes.push(route);
    }
    if attributed.is_some()
        && (total_units != integer(report, "reported_quantity")?
            || total_cash
                != integer(report, "reported_quantity")?
                    .checked_mul(unit_price)
                    .ok_or("InvalidDomain：成交金额溢出")?
            || total_commission != integer(report, "commission")?)
    {
        return Err("IdentityConflict：逐腿归属合计与成交数量/金额/费用不闭合".into());
    }
    if next.get("fill_allocations").is_none() {
        next["fill_allocations"] = json!([]);
    }
    // 缺归属路由不得将未知分配伪装为零；成功路由显式携带逐腿数量、金额与费用。
    next["fill_allocations"].as_array_mut().unwrap().push(json!({"fact_key":fact["fact_key"],"fact_hash":fact["fact_hash"],"projection_kind":"per_leg","manifest_hash":m["manifest_hash"],"symbol":m["symbol"],"fill_allocation_policy_id":policy["fill_allocation_policy_id"],"policy_revision":policy["policy_revision"],"evidence_id":evidence_id,"allocations":routes}));
    Ok(())
}
