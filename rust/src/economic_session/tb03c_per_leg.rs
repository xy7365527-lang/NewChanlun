//! #1406 C4b：per_leg 缺来源归属只能 Unmatched，不能改走净额比例。
use super::book::{array, hash};
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
    if !event["per_leg_report"]["leg_attribution"].is_null() {
        return Err("Unsupported：具备来源逐腿归属的 per_leg 成功域不在 C4b 实施范围".into());
    }
    let policy = &config.manifest["fill_allocation_policy"];
    let mut routes = Vec::new();
    for p in array(m, "participants")? {
        let id = format!(
            "per-leg:{}",
            hash(&json!([
                fact["fact_key"],
                event["allocation_id"],
                m["manifest_hash"],
                p
            ]))
        );
        routes.push(json!({"allocation_id":id,"chong_id":p["chong_id"],"family_id":p["family_id"],"command_id":p["command_id"],"revision":p["revision"],"unmatched_reason":"per_leg_attribution_missing","manifest_hash":m["manifest_hash"],"fill_allocation_policy_id":policy["fill_allocation_policy_id"],"policy_revision":policy["policy_revision"]}));
    }
    if next.get("fill_allocations").is_none() {
        next["fill_allocations"] = json!([]);
    }
    // 路由证明不带数量或金额，绝不将未知分配伪装为零。
    next["fill_allocations"].as_array_mut().unwrap().push(json!({"fact_key":fact["fact_key"],"fact_hash":fact["fact_hash"],"projection_kind":"per_leg","manifest_hash":m["manifest_hash"],"symbol":m["symbol"],"fill_allocation_policy_id":policy["fill_allocation_policy_id"],"policy_revision":policy["policy_revision"],"evidence_id":evidence_id,"allocations":routes}));
    Ok(())
}
