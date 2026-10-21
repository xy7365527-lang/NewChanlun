//! #1415 TB-03-D：B 侧依据使用关系的只读派生视图（SPEC #1340 C12 / Q.ReadEconomicView）。
//! 只从本 B 已持久的固定 cut 图像派生：Voice、Prepared/Committed 决定、事实应用各自所用的
//! 结构/操作/经营依据身份，及本重的实际量与未决潜量。不读 S、不写任何域，结果不回写 S。
//! 结构级别、操作级别、Voice 代际、下钻深度与方向分字段原样转出，不合并成单一“级别”。
use super::book::{add, hash, integer};
use super::store::Config;
use serde_json::{json, Value};

fn array<'a>(v: &'a Value, k: &str) -> Result<&'a [Value], String> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(a)) => Ok(a),
        Some(_) => Err(format!("StorageUnavailable：{k} 形状")),
    }
}

/// 结构证据只转出身份与摘要；完整原件仍由 QueryDecision 按原身份取回。
fn structure_ref(v: &Value) -> Value {
    if v.is_null() {
        return Value::Null;
    }
    json!({"evidence_hash":hash(v),"kind":v["kind"],"session_id":v["session_id"],"session_generation":v["session_generation"],"writer_epoch":v["writer_epoch"],"commit_ns":v["commit_ns"]})
}

/// 等待原因只陈述本 B 已知的前件缺口；B 不知 E/X 的后续状态，不据此推断已释放或已执行。
fn decision_waiting(d: &Value) -> Value {
    match d["state"].as_str() {
        Some("Prepared") => {
            json!({"reason_code":"AwaitingEReserveGrant","detail":"本地 Commit 须先有 E 增责任后的 grant"})
        }
        Some("Committed") => {
            json!({"reason_code":"AwaitingExecutionEligibility","detail":"本地已提交；E.Release/X 调用资格不在 B","execution_eligibility":d["local_commit_receipt"]["execution_eligibility"]})
        }
        Some("AbortedBeforeCommit") => {
            json!({"reason_code":"PotentialRetainedAfterAbort","detail":"Abort 不证明 E/尾责可释放，潜量与私有锁保留","abort_reason":d["abort_reason"]})
        }
        other => json!({"reason_code":"UnknownState","state":other}),
    }
}

fn voice_usage(v: &Value) -> Value {
    let sv = &v["structure_view"];
    json!({
        "voice_id":v["voice_id"],"binding_id":v["binding_id"],
        "direction":v["identity"]["sigma"],
        "voice_generation":v["identity"]["n"],
        "parent_voice_id":v["parent_voice_id"],
        "operation_level":v["operation_level"],
        "structure_level":{"profile_id":sv["profile_id"],"execution_level":sv["execution_level"],"confirmation_level":sv["confirmation_level"]},
        "drill_depth":sv["nest_depth"],
        "carrier_ref":v["identity"]["carrier_ref"],"gamma_ref":v["identity"]["gamma_ref"],
        "evidence":v["evidence"],
        "opening_basis":{"fact_key":v["opening_basis"]["fact_key"],"fact_hash":v["opening_basis"]["fact_hash"],"allocation_id":v["opening_basis"]["allocation_id"],"quantity":v["opening_basis"]["quantity"]},
        "first_known_ns":v["first_known_ns"],"commit_seq":v["commit_seq"]
    })
}

pub(super) fn view(
    config: &Config,
    image: &Value,
    cut: Value,
    commit_ns: &str,
) -> Result<Value, String> {
    if config.domain == "E" {
        return Err("SchemaUnsupported：E 不拥有 B 依据使用关系".into());
    }
    let book = &image["book"];
    let voices = super::voice::records(image)?;
    let mut decisions = vec![];
    let mut potential = 0i64;
    for d in array(&image["finance"], "decisions")? {
        let plan = &d["prepared"]["plan"];
        if plan["owner"] != config.domain {
            return Err("StorageUnavailable：决定不属于本重".into());
        }
        // 与 finance::own_lock 同一保守口径：未见 E 降责证据前，每个已 Prepared 的计划量均计潜量。
        potential = add(potential, integer(plan, "quantity")?)?;
        decisions.push(json!({
            "decision_id":d["decision_id"],"state":d["state"],"plan_hash":d["prepared"]["plan_hash"],
            "voice_binding_id":plan["voice_binding_id"],"voice_id":d["prepared"]["voice"]["voice_id"],
            "direction":d["prepared"]["voice"]["identity"]["sigma"],
            "quantity":plan["quantity"],"quantity_source":plan["quantity_source"],
            "s_ref":plan["s_ref"],"o_ref":plan["o_ref"],
            "structure_evidence":structure_ref(&d["structure_evidence"]),
            "right_id":plan["right_id"],"rights_version_after":d["prepared"]["rights_version_after"],
            "book_version_before":d["prepared"]["book_version_before"],
            "grant_id":d["grant"]["original"]["grant_id"],
            "waiting":decision_waiting(d)
        }));
    }
    let unmatched = array(image, "unmatched")?;
    let mut applications = vec![];
    for a in array(image, "applied")? {
        let used_by: Vec<&Value> = voices
            .iter()
            .filter(|v| v["opening_basis"]["allocation_id"] == a["allocation_id"])
            .map(|v| &v["voice_id"])
            .collect();
        let reason = unmatched
            .iter()
            .find(|u| u["allocation_id"] == a["allocation_id"] && u["fact_key"] == a["fact_key"])
            .map(|u| json!({"reason_code":u["reason_code"],"reason":u["reason"]}))
            .unwrap_or(Value::Null);
        applications.push(json!({"fact_key":a["fact_key"],"fact_hash":a["fact_hash"],"allocation_id":a["allocation_id"],"status":a["status"],"commit_seq":a["commit_seq"],"used_by_voices":used_by,"unmatched":reason}));
    }
    let quantity = if book.is_null() {
        json!({"status":"MissingDependency","reason":"实际非空重尚未建立","unresolved_potential":potential.to_string()})
    } else {
        let actual = integer(book, "actual_quantity")?;
        let fixed = integer(book, "fixed_q")?;
        let worst = add(actual, potential)?;
        json!({"status":"Known","chong_id":book["chong_id"],"side":book["side"],"units":book["units"]["quantity"],"actual_quantity":actual.to_string(),"unresolved_potential":potential.to_string(),"worst_quantity":worst.to_string(),"fixed_q":fixed.to_string(),"headroom":fixed.checked_sub(worst).ok_or("InvalidDomain：余量溢出")?.to_string(),"scope":"本重独立；不同重的方向与潜量互不抵消"})
    };
    let book_ref = if book.is_null() {
        Value::Null
    } else {
        json!({"chong_id":book["chong_id"],"symbol":book["symbol"],"operation_level":book["operation_level"],"opening_fact_ref":book["opening_fact_ref"]})
    };
    let voices: Vec<Value> = voices.iter().map(voice_usage).collect();
    Ok(json!({
        "kind":"EconomicBasisUsage","domain_id":config.domain,"cut":cut,"commit_ns":commit_ns,
        "book":book_ref,"voices":voices,
        "decisions":decisions,"applications":applications,"quantity":quantity,
        "structure_boundary":{"s_writes":"None","relation":"B 只按固定证据读 S 依据；本域无 S 写操作，经营结果不回写 S"}
    }))
}
