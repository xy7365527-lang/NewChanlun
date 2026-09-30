//! #1394：具名 TestOnly 现金买入域的 Prepared → Reserved → Committed。
//! 原始资金/量/政策/slot 输入须冻结；本模块无场所、Release 或资金排序权。
use super::book::hash;
use super::store::*;
use super::voice::{digest, exact};
use crate::s_content::job::Job;
use crate::session_protocol::canonical_json;
use crate::session_protocol::{
    nonnegative, positive, required, sha256_hex, strict_json, validate_envelope,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

const REV: &str = "tb03b-finance/1";
/// #1394 §5.7：新财务子合同。没有 `/2` 域，`/1` 的子合同也不得靠改名接管新域。
const REV_V2: &str = "tb03b-finance/2";
/// §5.7 `s_ref/2` 的 exact 四字段版本值。
const S_REF_V2: &str = "s-object-ref/2";
/// 财务子合同版本的唯一登记点：缺省 `/1`（旧 manifest 逐字仍可读），显式只认 `/2`。
///
/// 与 E/B 域版本（`store::domain_revision`）必须同版；未知值与混写当场拒绝（§5.8）。
fn revision(m: &Value) -> Result<&'static str, String> {
    let domain = super::store::domain_revision(m)?;
    let want = if domain == super::store::SCHEMA_V2 {
        REV_V2
    } else {
        REV
    };
    let Some(f) = m.get("finance").filter(|f| !f.is_null()) else {
        return Ok(want);
    };
    match f.get("schema_revision") {
        Some(Value::String(text)) if text == want => Ok(want),
        Some(Value::String(text)) if text == REV || text == REV_V2 => {
            Err("IdentityConflict：财务子合同版本与 E/B 域版本混写".into())
        }
        _ => Err("SchemaUnsupported：财务子合同只有 tb03b-finance/1 与 tb03b-finance/2".into()),
    }
}
const LIMIT: usize = 16 * 1024 * 1024;
pub(super) fn enabled(config: &Config) -> bool {
    config.manifest.get("finance").is_some_and(|v| !v.is_null())
}
pub(super) fn validate_config(m: &Value) -> Result<(), String> {
    let Some(f) = m.get("finance").filter(|f| !f.is_null()) else {
        return Ok(());
    };
    // 外层 `economic-manifest/1` 与 finance 顶层 exact 五字段不随版本变化（§5.7 冻结）。
    exact(
        f,
        &[
            "schema_revision",
            "input_path",
            "input_sha256",
            "coordinator_source",
            "structure",
        ],
    )?;
    let revision = revision(m)?;
    if m["clock"]["kind"] != "TestOnly" {
        return Err("UnsupportedCapability：仅显式 TestOnly 财务输入域".into());
    }
    digest(required(f, "input_sha256")?)?;
    if !Path::new(required(f, "input_path")?).is_absolute() {
        return Err("InvalidDomain：资金原件须绝对路径".into());
    }
    exact(
        &f["coordinator_source"],
        &[
            "source_namespace",
            "source_epoch",
            "producer_id",
            "producer_epoch",
        ],
    )?;
    // structure 的 exact 形状按子合同版本分派：`/1` 六字段逐字保持，`/2` 九字段。
    let structure: &[&str] = if revision == REV_V2 {
        &[
            "binary",
            "binary_sha256",
            "db",
            "session_id",
            "session_generation",
            "writer_epoch",
            "reader_revision",
            "content_format",
            "reader_resources",
        ]
    } else {
        &[
            "binary",
            "binary_sha256",
            "db",
            "session_id",
            "session_generation",
            "writer_epoch",
        ]
    };
    exact(&f["structure"], structure)?;
    for k in ["binary", "db"] {
        if !Path::new(required(&f["structure"], k)?).is_absolute() {
            return Err("InvalidDomain：S 固定路径须绝对".into());
        }
    }
    digest(required(&f["structure"], "binary_sha256")?)?;
    if f["structure"]["session_id"] != m["session_id"]
        || f["structure"]["session_generation"] != m["session_generation"]
    {
        return Err("IdentityConflict：S 与经济会话不同".into());
    }
    positive(required(&f["structure"], "writer_epoch")?, "S epoch")?;
    if revision == REV_V2 {
        if f["structure"]["reader_revision"] != "s-financial-basis/3" {
            return Err("SchemaUnsupported：结构 reader_revision 须 s-financial-basis/3".into());
        }
        if f["structure"]["content_format"] != crate::s_content::CONTENT_FORMAT {
            return Err("SchemaUnsupported：结构 content_format 须 s-content/1".into());
        }
        // §5.6.1 冻结的六字段资源合同（含八字段 job_limits）；本层只解析，不填默认值。
        super::financial_reader_resources::FinancialReaderResources::from_json(
            &f["structure"]["reader_resources"],
        )?;
    }
    Ok(())
}
/// 创世图像：财务子对象的版本登记与 `validate_config` 同源，不另立一份判断。
pub(super) fn empty(config: &Config) -> Result<Value, String> {
    let mut image = super::book::empty_image();
    if enabled(config) {
        let revision = revision(&config.manifest)?;
        image["finance"] = json!({"schema_revision":revision,"rights":[],"decisions":[],"slots":[],"responsibilities":[]});
    }
    Ok(image)
}
fn list<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    v[key]
        .as_array()
        .ok_or_else(|| format!("SchemaUnsupported：{key} 须数组"))
}
fn add(a: i64, b: i64) -> Result<i64, String> {
    a.checked_add(b)
        .ok_or("InvalidDomain：财务精确整数溢出".into())
}
fn number(v: &Value, k: &str) -> Result<i64, String> {
    nonnegative(required(v, k)?, k)
}
fn item<'a>(a: &'a [Value], k: &str, id: &str) -> Result<&'a Value, String> {
    a.iter()
        .find(|v| v[k] == id)
        .ok_or_else(|| format!("MissingDependency：{k}={id} 不存在"))
}
fn pinned_raw(config: &Config) -> Result<String, String> {
    let path = required(&config.manifest["finance"], "input_path")?;
    let mut bytes = Vec::new();
    let f =
        std::fs::File::open(path).map_err(|e| format!("MissingDependency：财务原件不可读：{e}"))?;
    if !f.metadata().map_err(err)?.is_file() {
        return Err("InvalidDomain：财务原件须普通文件".into());
    }
    f.take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > LIMIT {
        return Err("InvalidDomain：财务原件超限".into());
    }
    String::from_utf8(bytes).map_err(err)
}
fn input(config: &Config, raw: &str) -> Result<Value, String> {
    if !enabled(config) {
        return Err("MissingDependency：未配置具名财务输入".into());
    }
    if raw.len() > LIMIT || sha256_hex(raw.as_bytes()) != config.manifest["finance"]["input_sha256"]
    {
        return Err("IdentityConflict：财务原件不等于冻结摘要".into());
    }
    let d = strict_json(raw.as_bytes())?;
    exact(
        &d,
        &[
            "schema_revision",
            "kind",
            "source_id",
            "source_epoch",
            "policy",
            "baseline",
            "rights",
            "plans",
            "slots",
        ],
    )?;
    // 输入原件的版本与子合同同版：`/2` 的原始 SHA 必须重新冻结。
    if d["schema_revision"] != revision(&config.manifest)? || d["kind"] != "TestOnly" {
        return Err("UnsupportedCapability：财务输入非具名 TestOnly".into());
    }
    required(&d, "source_id")?;
    required(&d, "source_epoch")?;
    let p = &d["policy"];
    exact(
        p,
        &[
            "policy_id",
            "allocation_policy_id",
            "cap",
            "currency",
            "money_unit",
            "quantity_unit",
            "quantum",
            "capability_id",
            "model",
            "valid_through_ns",
        ],
    )?;
    for k in [
        "policy_id",
        "currency",
        "money_unit",
        "quantity_unit",
        "capability_id",
    ] {
        required(p, k)?;
    }
    if p["allocation_policy_id"] != "#1349/unresolved-common-cap-allocation"
        || p["model"] != "TestOnlyUnleveragedCashBuy"
        || p["quantum"] != "1"
    {
        return Err("UnsupportedCapability：资金模型/量化/分配政策不在本片".into());
    }
    number(p, "cap")?;
    number(p, "valid_through_ns")?;
    exact(
        &d["baseline"],
        &["responsibility_id", "amount", "source_ref", "covered_facts"],
    )?;
    required(&d["baseline"], "responsibility_id")?;
    required(&d["baseline"], "source_ref")?;
    number(&d["baseline"], "amount")?;
    let mut facts = BTreeSet::new();
    for f in list(&d["baseline"], "covered_facts")? {
        exact(f, &["fact_key", "fact_hash", "token"])?;
        for k in ["fact_key", "fact_hash"] {
            digest(required(f, k)?)?;
        }
        required(f, "token")?;
        if !facts.insert(required(f, "fact_key")?) {
            return Err("IdentityConflict：责任原件覆盖重复事实".into());
        }
    }
    let mut ids = BTreeSet::new();
    let mut owners = BTreeSet::new();
    for r in list(&d, "rights")? {
        exact(
            r,
            &[
                "right_id",
                "owner",
                "version",
                "amount",
                "currency",
                "source_ref",
            ],
        )?;
        required(r, "source_ref")?;
        positive(required(r, "version")?, "rights version")?;
        number(r, "amount")?;
        if r["currency"] != p["currency"]
            || !ids.insert(required(r, "right_id")?)
            || !owners.insert(required(r, "owner")?)
        {
            return Err("IdentityConflict：私有权利身份/币种重复或不同".into());
        }
        config.route(
            required(r, "owner")?
                .strip_prefix("B:")
                .ok_or("InvalidDomain：权利 owner 须 B")?,
        )?;
    }
    ids.clear();
    let mut attempts = BTreeSet::new();
    let mut plan_ids = BTreeSet::new();
    for q in list(&d, "plans")? {
        exact(
            q,
            &[
                "decision_id",
                "attempt_id",
                "plan_id",
                "owner",
                "voice_binding_id",
                "right_id",
                "expected_book_version",
                "expected_rights_version",
                "applicant_epoch",
                "applicant_generation",
                "slot_id",
                "quantity",
                "limit_price",
                "fee_tail",
                "quantity_source",
                "price_source",
                "fee_source",
                "s_ref",
                "o_ref",
            ],
        )?;
        for k in [
            "quantity_source",
            "price_source",
            "fee_source",
            "voice_binding_id",
        ] {
            required(q, k)?;
        }
        for k in [
            "expected_book_version",
            "expected_rights_version",
            "applicant_epoch",
            "applicant_generation",
            "quantity",
            "limit_price",
        ] {
            positive(required(q, k)?, k)?;
        }
        number(q, "fee_tail")?;
        if !ids.insert(required(q, "decision_id")?)
            || !attempts.insert((required(q, "owner")?, required(q, "attempt_id")?))
            || !plan_ids.insert(required(q, "plan_id")?)
        {
            return Err("IdentityConflict：决定/attempt/plan 身份重复".into());
        }
        // §5.7：`s_ref/2` 的 exact 四字段（新增 schema_revision）；`/1` 域逐字保持三字段。
        let s_ref2 = revision(&config.manifest)? == REV_V2;
        let s_ref: &[&str] = if s_ref2 {
            &["schema_revision", "object_id", "object_hash", "profile_id"]
        } else {
            &["object_id", "object_hash", "profile_id"]
        };
        exact(&q["s_ref"], s_ref)?;
        if s_ref2 && q["s_ref"]["schema_revision"] != S_REF_V2 {
            return Err(
                "SchemaUnsupported：s_ref/2 必须显式 schema_revision=s-object-ref/2".into(),
            );
        }
        digest(required(&q["s_ref"], "object_hash")?)?;
        required(&q["s_ref"], "object_id")?;
        required(&q["s_ref"], "profile_id")?;
        exact(
            &q["o_ref"],
            &[
                "operation_id",
                "kind",
                "source_ref",
                "s_object_id",
                "quantity",
            ],
        )?;
        if q["o_ref"]["kind"] != "TestOnlyCashBuy"
            || q["o_ref"]["s_object_id"] != q["s_ref"]["object_id"]
            || q["o_ref"]["quantity"] != q["quantity"]
        {
            return Err("IdentityConflict：O 动作/量未绑定 S 和原量".into());
        }
        required(&q["o_ref"], "operation_id")?;
        required(&q["o_ref"], "source_ref")?;
        let r = item(list(&d, "rights")?, "right_id", required(q, "right_id")?)?;
        if r["owner"] != q["owner"] || number(r, "version")? > number(q, "expected_rights_version")?
        {
            return Err("IdentityConflict：计划不属于原私有权利/版本".into());
        }
        bound(q)?;
    }
    ids.clear();
    let mut assigned = BTreeSet::new();
    for s in list(&d, "slots")? {
        exact(s, &["slot_id", "applicants", "input_end"])?;
        if !ids.insert(required(s, "slot_id")?) {
            return Err("IdentityConflict：slot 重复".into());
        }
        exact(
            &s["input_end"],
            &[
                "kind",
                "source_id",
                "source_epoch",
                "watermark",
                "complete_request_hash",
            ],
        )?;
        if s["input_end"]["kind"] != "ControlledInputEnd"
            || s["input_end"]["source_id"] != d["source_id"]
            || s["input_end"]["source_epoch"] != d["source_epoch"]
            || s["input_end"]["complete_request_hash"] != hash(&s["applicants"])
        {
            return Err("MissingDependency：slot 结束依据不覆盖完整请求集".into());
        }
        positive(required(&s["input_end"], "watermark")?, "slot watermark")?;
        let mut slot_owners = BTreeSet::new();
        let a = list(s, "applicants")?;
        if a.is_empty() {
            return Err("InvalidDomain：完整 slot 不得为空".into());
        }
        for id in a {
            let id = id.as_str().ok_or("SchemaUnsupported：申请身份须文本")?;
            let q = item(list(&d, "plans")?, "decision_id", id)?;
            if q["slot_id"] != s["slot_id"]
                || !assigned.insert(id)
                || !slot_owners.insert(required(q, "owner")?)
            {
                return Err("IdentityConflict：slot 请求重复/归属错误".into());
            }
        }
    }
    if assigned.len() != list(&d, "plans")?.len() || assigned.is_empty() {
        return Err("MissingDependency：计划未全部纳入非空关闭集合".into());
    }
    Ok(d)
}
fn bound(p: &Value) -> Result<i64, String> {
    number(p, "quantity")?
        .checked_mul(number(p, "limit_price")?)
        .and_then(|v| v.checked_add(number(p, "fee_tail").ok()?))
        .ok_or("InvalidDomain：完整责任算式溢出".into())
}
/// C06.3/C06.5：核 Pending 是否由固定 E 保守完整界（covered_facts）覆盖，并对应 E 实存原事实与 token。
/// 只认 E 已独立保存的界与原件，不读坏 B、不用缓存余额、不伪造 Applied。
fn covered_pending(previous: &Value, coverage: &[Value], pending: &Value) -> Result<(), String> {
    let covered = coverage
        .iter()
        .find(|x| x["fact_key"] == pending["fact_key"] && x["token"] == pending["token"])
        .ok_or("MissingDependency：相关 Pending 未被已核责任界覆盖")?;
    let fact = item(
        list(previous, "recorded")?,
        "fact_key",
        required(covered, "fact_key")?,
    )?;
    if fact["fact_hash"] != covered["fact_hash"] {
        return Err("IdentityConflict：共同责任原件覆盖的事实内容不同".into());
    }
    Ok(())
}
fn plan<'a>(d: &'a Value, p: &Value) -> Result<&'a Value, String> {
    item(
        list(d, "plans")?,
        "decision_id",
        required(p, "decision_id")?,
    )
}
/// 图像级版本判据：与 `Config::revision()` 同源，`/1` 与 `/2` 各有自己的一份，
/// 不靠「接受两版」放宽（§5.8 旧域不混写）。
fn finance<'a>(config: &Config, image: &'a Value) -> Result<&'a Value, String> {
    let f = &image["finance"];
    if f["schema_revision"] != revision(&config.manifest)? {
        Err("StorageUnavailable：财务图像版本不同".into())
    } else {
        Ok(f)
    }
}
fn decision<'a>(config: &Config, image: &'a Value, id: &str) -> Result<&'a Value, String> {
    item(
        list(finance(config, image)?, "decisions")?,
        "decision_id",
        id,
    )
}
fn push(v: &mut Value, k: &str, x: Value) {
    v[k].as_array_mut().unwrap().push(x)
}
fn receipt_key(op: &str, id: &str) -> String {
    format!("{REV}:{op}:{}", hash(&json!(id)))
}
fn receipt_key_for(config: &Config, op: &str, id: &str) -> Result<String, String> {
    if revision(&config.manifest)? == REV {
        Ok(receipt_key(op, id))
    } else {
        Ok(format!("{REV_V2}:{op}:{}", hash(&json!(id))))
    }
}
pub(super) fn is_event(v: &Value) -> bool {
    matches!(
        v["kind"].as_str(),
        Some(
            "ImportResponsibility"
                | "PrepareDecision"
                | "Reserve"
                | "CommitDecision"
                | "AbortDecision"
                | "CloseApplicants"
        )
    )
}
fn operation_id<'a>(p: &'a Value) -> Result<&'a str, String> {
    required(
        p,
        if p["op"] == "ImportResponsibility" {
            "responsibility_id"
        } else if matches!(p["op"].as_str(), Some("Reserve" | "CloseApplicants")) {
            "slot_id"
        } else {
            "decision_id"
        },
    )
}
fn key_for(config: &Config, p: &Value) -> Result<String, String> {
    receipt_key_for(config, required(p, "op")?, operation_id(p)?)
}
#[cfg(test)]
fn key(p: &Value) -> Result<String, String> {
    Ok(receipt_key(required(p, "op")?, operation_id(p)?))
}
fn validate_request(p: &Value) -> Result<(), String> {
    match required(p, "op")? {
        "ImportResponsibility" => exact(p, &["op", "responsibility_id", "writer_epoch"]),
        "PrepareDecision" => exact(p, &["op", "decision_id", "plan_hash", "writer_epoch"]),
        "Reserve" => exact(p, &["op", "slot_id", "request_set_hash", "writer_epoch"]),
        "CommitDecision" => exact(
            p,
            &["op", "decision_id", "plan_hash", "grant_id", "writer_epoch"],
        ),
        "AbortDecision" => exact(
            p,
            &["op", "decision_id", "plan_hash", "writer_epoch", "reason"],
        ),
        "CloseApplicants" => exact(p, &["op", "slot_id", "writer_epoch", "closed", "reason"]),
        _ => Err("SchemaUnsupported：未知财务操作".into()),
    }?;
    positive(required(p, "writer_epoch")?, "request writer_epoch")?;
    Ok(())
}
fn proof(config: &Config, domain: &str, key: &str) -> Result<Value, String> {
    proof_inner(config, domain, key, None, None)
}
fn proof_inner(
    config: &Config,
    domain: &str,
    key: &str,
    mut import: Option<(&mut super::content_store::Stage<'_>, &mut Job)>,
    deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    if let Some((_, job)) = import.as_mut() {
        job.charge_requests(1).map_err(|e| e.to_string())?;
    }
    let other = domain_config(config, domain)?;
    let (mut c, _) = open(&other.db, true)?;
    let tx = c.transaction().map_err(err)?;
    if let Some((_, job)) = import.as_mut() {
        audit_with_deadline(&tx, &other, Some(&mut **job), deadline)?;
    } else {
        audit(&tx, &other)?;
    }
    let receipt = row(&tx, "receipts", key)?.ok_or("MissingDependency：原身份尚未持久")?;
    let seq = positive(required(&receipt["cut"], "commit_seq")?, "proof seq")?;
    let (prev, ns, raw): (String, String, String) = tx
        .query_row(
            "SELECT prev_root,commit_ns,image FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(err)?;
    let (current_seq, current_root, current_ns, current_image) = latest(&tx)?;
    let current_prev: String = tx
        .query_row(
            "SELECT prev_root FROM commits WHERE seq=?",
            [current_seq],
            |r| r.get(0),
        )
        .map_err(err)?;
    let result = json!({"current_cut":cut(&other,current_seq,&current_root),"current_previous_root":current_prev,"current_commit_ns":current_ns,"domain_id":domain,"receipt":receipt,"previous_root":prev,"commit_ns":ns,"image":strict_json(raw.as_bytes())?,"current_image":current_image,"current_epoch":meta(&tx,"writer_epoch")?,"current_epoch_at_ns":tx.query_row("SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",[],|r|r.get::<_,String>(0)).map_err(err)?});
    if let Some((stage, job)) = import {
        check_operation_deadline(Some(&mut *job), deadline)?;
        job.charge_wire_bytes(u64::try_from(canonical_json(&result).len()).map_err(err)?)
            .map_err(|e| e.to_string())?;
        let mut refs =
            super::content_store::image_refs(&result["image"]).map_err(|e| e.to_string())?;
        refs.extend(
            super::content_store::image_refs(&result["current_image"])
                .map_err(|e| e.to_string())?,
        );
        super::content_store::import_peer_refs(&tx, &refs, stage, job)
            .map_err(|e| format!("MissingDependency：peer closed refs 导入失败：{e}"))?;
    }
    tx.commit().map_err(err)?;
    Ok(result)
}
fn domain_config(config: &Config, domain: &str) -> Result<Config, String> {
    let mut x = config.clone();
    let r = if domain == "E" {
        &config.manifest["e"]
    } else {
        config.route(
            domain
                .strip_prefix("B:")
                .ok_or("InvalidDomain：未知经济域")?,
        )?
    };
    x.domain = domain.into();
    x.producer = format!("{domain}:{}", config.session);
    x.db = PathBuf::from(required(r, "db")?);
    x.socket = PathBuf::from(required(r, "socket")?);
    Ok(x)
}
fn verify_proof(config: &Config, p: &Value, domain: &str) -> Result<(), String> {
    exact(
        p,
        &[
            "domain_id",
            "receipt",
            "previous_root",
            "commit_ns",
            "image",
            "current_image",
            "current_epoch",
            "current_epoch_at_ns",
            "current_cut",
            "current_previous_root",
            "current_commit_ns",
        ],
    )?;
    let other = domain_config(config, domain)?;
    let r = &p["receipt"];
    let seq = positive(required(&r["cut"], "commit_seq")?, "proof seq")?;
    if p["domain_id"] != domain
        || r["domain_id"] != domain
        || r["cut"]
            != cut(
                &other,
                seq,
                &root(
                    &other,
                    seq,
                    required(p, "previous_root")?,
                    required(p, "commit_ns")?,
                    &p["image"],
                ),
            )
    {
        return Err("IdentityConflict：原回执不属于固定 owner/实际提交根".into());
    }
    let cs = positive(
        required(&p["current_cut"], "commit_seq")?,
        "current proof seq",
    )?;
    if cs < seq
        || p["current_cut"]
            != cut(
                &other,
                cs,
                &root(
                    &other,
                    cs,
                    required(p, "current_previous_root")?,
                    required(p, "current_commit_ns")?,
                    &p["current_image"],
                ),
            )
    {
        return Err("IdentityConflict：当前权威图像证明不符".into());
    }
    let e = list(&p["image"], "history")?
        .last()
        .ok_or("MissingDependency：原提交事件缺失")?;
    if !is_event(e) || receipt(&other, e, r["cut"].clone())? != *r {
        return Err("IdentityConflict：原回执与原事件不等".into());
    }
    Ok(())
}
// 只读 S 正式入口（只服务 `/1` 子合同；`/2` 由 `write` 的版本门在到达这里之前拒绝）。
// 输出由 S 自己核完整可达根和当前 Begin 门。
fn structure(config: &Config) -> Result<Value, String> {
    let s = &config.manifest["finance"]["structure"];
    let binary = required(s, "binary")?;
    if sha256_hex(&std::fs::read(binary).map_err(err)?) != s["binary_sha256"] {
        return Err("IdentityConflict：S 构建摘要变化".into());
    }
    let v = super::finance_transport::reader(
        binary,
        &["financial-basis", "--db", required(s, "db")?],
        LIMIT,
    )?;
    verify_structure(config, &v, None)?;
    Ok(v)
}
fn verify_structure(config: &Config, v: &Value, p: Option<&Value>) -> Result<(), String> {
    number(v, "commit_ns")?;
    let expected = &config.manifest["finance"]["structure"];
    if v["kind"] != "FinancialBasis"
        || v["session_id"] != expected["session_id"]
        || v["session_generation"] != expected["session_generation"]
        || v["writer_epoch"] != expected["writer_epoch"]
    {
        return Err("StaleWriter：S 权威/会话/epoch 不符".into());
    }
    if v["pending_revision"] != false {
        return Err("MissingDependency：S 正在修订，普通准备不能假定有效".into());
    }
    if let Some(p) = p {
        let snapshot = &v["snapshot"];
        let sr = &p["s_ref"];
        let object = item(
            list(snapshot, "objects")?,
            "object_id",
            required(sr, "object_id")?,
        )?;
        if hash(object) != sr["object_hash"] || snapshot["profile_id"] != sr["profile_id"] {
            return Err("InvalidDomain：原 S 结构失效/被替换/内容改变".into());
        }
    }
    Ok(())
}
fn verify_structure_for_domain(
    c: &Connection,
    config: &Config,
    job: Option<&mut Job>,
    evidence: &Value,
    plan: Option<&Value>,
) -> Result<(), String> {
    if revision(&config.manifest)? == REV {
        return verify_structure(config, evidence, plan);
    }
    let job = job.ok_or("MissingDependency：新域缺统一证据 job")?;
    let mut source = super::content_store::LocalSource { c };
    super::financial_content::verify_evidence(
        &config.manifest["finance"]["structure"],
        evidence,
        &mut source,
        job,
        plan.map(|p| &p["s_ref"]),
    )?;
    Ok(())
}
fn knowledge_floor(witness: &Value, first: &str) -> Result<(), String> {
    let known = nonnegative(first, "first_known_ns")?;
    if witness.get("structure").is_some_and(|v| !v.is_null())
        && number(&witness["structure"], "commit_ns")? > known
    {
        return Err("InvalidDomain：获知时刻早于实际 S 提交".into());
    }
    let mut proofs = vec![];
    if let Some(p) = witness.get("grant_proof") {
        proofs.push(p);
    }
    if let Some(ps) = witness.get("prepared_proofs") {
        proofs.extend(
            ps.as_array()
                .ok_or("SchemaUnsupported：Prepared 证明须数组")?,
        );
    }
    for p in proofs {
        if number(p, "current_commit_ns")? > known || number(p, "current_epoch_at_ns")? > known {
            return Err("InvalidDomain：获知时刻早于实读对端的提交/恢复代际".into());
        }
    }
    Ok(())
}
fn own_lock(
    config: &Config,
    image: &Value,
    d: &Value,
    p: &Value,
    seq: i64,
    ns: &str,
) -> Result<Value, String> {
    if image["book"].is_null() {
        return Err("MissingDependency：实际非空重尚未建立".into());
    }
    let voice = super::voice::find_binding(image, required(p, "voice_binding_id")?)?
        .ok_or("MissingDependency：原 Voice 未绑定")?;
    if voice["identity"]["sigma"] != "Long" {
        return Err("UnsupportedCapability：受测现金买入仅 Long Voice".into());
    }
    if p["expected_book_version"] != (seq - 1).to_string() {
        return Err("InvalidDomain：账本 CAS 版本陈旧".into());
    }
    let r = item(list(d, "rights")?, "right_id", required(p, "right_id")?)?;
    let f = finance(config, image)?;
    let current_right = list(f, "rights")?
        .iter()
        .find(|x| x["right_id"] == r["right_id"]);
    let actual_version = if let Some(x) = current_right {
        number(x, "current_version")?
    } else {
        number(r, "version")?
    };
    if actual_version != number(p, "expected_rights_version")? {
        return Err("InvalidDomain：私有权利 CAS 版本陈旧".into());
    }
    let locked = list(f, "decisions")?
        .iter()
        .filter(|x| x["prepared"]["plan"]["right_id"] == r["right_id"])
        .try_fold(0i64, |a, x| {
            add(a, number(&x["prepared"], "locked_amount")?)
        })?;
    if add(locked, bound(p)?)? > number(r, "amount")? {
        return Err("InvalidDomain：真实自有权利不足或已锁".into());
    }
    let worst = add(
        number(&image["book"], "actual_quantity")?,
        number(p, "quantity")?,
    )?;
    let others = list(f, "decisions")?.iter().try_fold(0i64, |a, x| {
        add(a, number(&x["prepared"]["plan"], "quantity")?)
    })?;
    if add(worst, others)? > number(&image["book"], "fixed_q")? {
        return Err("InvalidDomain：最坏量超过固定 Q".into());
    }
    if image["book"]["units"]["money"]["currency"] != d["policy"]["currency"]
        || image["book"]["units"]["quantity"]["unit"] != d["policy"]["quantity_unit"]
        || image["book"]["units"]["money"]["unit"] != d["policy"]["money_unit"]
    {
        return Err("InvalidDomain：原账/权利/计划量纲不同".into());
    }
    Ok(
        json!({"plan":p,"plan_hash":hash(p),"voice":voice,"right_source":r,"rights_version_before":actual_version.to_string(),"rights_version_after":add(actual_version,1)?.to_string(),"locked_amount":bound(p)?.to_string(),"worst_quantity":add(worst,others)?.to_string(),"book_version_before":(seq-1).to_string(),"book_version_after":seq.to_string(),"prepared_ns":ns,"actual_quantity":image["book"]["actual_quantity"],"fixed_q":image["book"]["fixed_q"],"units":image["book"]["units"]}),
    )
}
/// 生产写与离线审计共用本地转换。oracle 在测试侧独立计算，不调用本函数。
fn reduce(
    c: &Connection,
    config: &Config,
    previous: &Value,
    request: &Value,
    w: &Value,
    seq: i64,
    ns: &str,
    mut job: Option<&mut Job>,
) -> Result<(Value, Value), String> {
    let p = &request["payload"];
    validate_request(p)?;
    let op = required(p, "op")?;
    let d = input(config, required(w, "input_raw")?)?;
    if !matches!(op, "AbortDecision" | "CloseApplicants")
        && number(&d["policy"], "valid_through_ns")? < nonnegative(ns, "commit_ns")?
    {
        return Err("MissingDependency：共同界/能力证据过期".into());
    }
    let mut next = previous.clone();
    finance(config, previous)?;
    let record = match op {
        "ImportResponsibility" => {
            if config.domain != "E" || p["responsibility_id"] != d["baseline"]["responsibility_id"]
            {
                return Err("IdentityConflict：既有责任原身份/owner 不符".into());
            }
            if !list(finance(config, previous)?, "responsibilities")?.is_empty() {
                return Err("IdentityConflict：既有责任已导入，不能重置公共余额".into());
            }
            let r = json!({"responsibility_id":d["baseline"]["responsibility_id"],"owner":"E:baseline","amount":d["baseline"]["amount"],"source":d["baseline"],"policy":d["policy"]});
            push(&mut next["finance"], "responsibilities", r.clone());
            json!({"responsibility_id":p["responsibility_id"],"state":"ResponsibilityImported","original":r})
        }
        "PrepareDecision" => {
            let plan = plan(&d, p)?;
            if plan["owner"] != config.domain || hash(plan) != p["plan_hash"] {
                return Err("IdentityConflict：原计划/owner 不符".into());
            }
            if plan["applicant_epoch"] != p["writer_epoch"] {
                return Err("StaleWriter：旧申请者 epoch 不得创建新 Prepared".into());
            }
            if list(finance(config, previous)?, "decisions")?
                .iter()
                .any(|x| {
                    x["decision_id"] == p["decision_id"]
                        || x["prepared"]["plan"]["attempt_id"] == plan["attempt_id"]
                })
            {
                return Err("IdentityConflict：决定或 attempt 已使用".into());
            }
            verify_structure_for_domain(
                c,
                config,
                job.as_deref_mut(),
                &w["structure"],
                Some(plan),
            )?;
            let prepared = own_lock(config, previous, &d, plan, seq, ns)?;
            let r = json!({"decision_id":p["decision_id"],"state":"Prepared","prepared":prepared,"structure_evidence":w["structure"],"grant":null,"local_commit_receipt":null,"execution_eligibility":"None"});
            let f = &mut next["finance"];
            if !list(f, "rights")?
                .iter()
                .any(|r| r["right_id"] == plan["right_id"])
            {
                let mut right = prepared["right_source"].clone();
                right["current_version"] = right["version"].clone();
                right["locked_amount"] = json!("0");
                push(f, "rights", right);
            }
            let right = f["rights"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["right_id"] == plan["right_id"])
                .unwrap();
            right["current_version"] = prepared["rights_version_after"].clone();
            right["locked_amount"] = json!(add(
                number(right, "locked_amount")?,
                number(&prepared, "locked_amount")?
            )?
            .to_string());
            push(f, "decisions", r.clone());
            r
        }
        "Reserve" => {
            if config.domain != "E" {
                return Err("InvalidDomain：Reserve 仅 E".into());
            }
            let slot = item(list(&d, "slots")?, "slot_id", required(p, "slot_id")?)?;
            if p["request_set_hash"] != hash(&slot["applicants"]) {
                return Err("IdentityConflict：完整 slot 请求内容不同".into());
            }
            let f = finance(config, previous)?;
            if list(f, "slots")?
                .iter()
                .any(|s| s["state"] == "AwaitingAllocationPolicy")
            {
                return Err(
                    "AwaitingPolicy：#1349/unresolved-common-cap-allocation；不得让其他 slot 插队"
                        .into(),
                );
            }
            let closed = f
                .get("closed_applicants")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let closed_for = |q: &Value| {
                closed.iter().any(|x| {
                    x["slot_id"] == slot["slot_id"]
                        && x["owner"] == q["owner"]
                        && x["applicant_epoch"] == q["applicant_epoch"]
                        && x["applicant_generation"] == q["applicant_generation"]
                })
            };
            let applicants = list(slot, "applicants")?;
            let active: Vec<&Value> = applicants
                .iter()
                .map(|id| {
                    item(
                        list(&d, "plans").unwrap(),
                        "decision_id",
                        id.as_str().unwrap(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter(|q| !closed_for(q))
                .collect();
            if active.is_empty() {
                return Err("StaleWriter：关闭集合没有可接纳申请者".into());
            }
            let proofs = list(w, "prepared_proofs")?;
            if proofs.len() != active.len() {
                return Err("MissingDependency：完整同 slot 原 Prepared 证据不齐".into());
            }
            let mut seen = BTreeSet::new();
            let mut grants = Vec::new();
            let mut additional = 0i64;
            for q in &active {
                let id = required(q, "decision_id")?;
                let pf = proofs
                    .iter()
                    .find(|x| x["receipt"]["record"]["decision_id"] == id)
                    .ok_or("MissingDependency：完整集合缺实际 Prepared")?;
                if !seen.insert(id) {
                    return Err("IdentityConflict：Prepared 重复".into());
                }
                verify_proof(config, pf, required(q, "owner")?)?;
                let original = &pf["receipt"]["record"];
                let current = decision(config, &pf["current_image"], id)?;
                if pf["receipt"]["kind"] != "PreparedReceipt"
                    || original["prepared"]["plan"] != **q
                    || current["state"] != "Prepared"
                    || current["prepared"] != original["prepared"]
                {
                    return Err("InvalidDomain：原 Prepared/计划/私有锁已失效".into());
                }
                if pf["current_epoch"] != q["applicant_epoch"] {
                    return Err("StaleWriter：申请者已换代，旧申请不得新授信".into());
                }
                let expected_version =
                    positive(required(q, "expected_book_version")?, "book version")? + 1;
                if latest_proof_seq(pf)? != expected_version {
                    return Err("InvalidDomain：Prepared 后本地账本版本已改变".into());
                }
                verify_structure_for_domain(
                    c,
                    config,
                    job.as_deref_mut(),
                    &w["structure"],
                    Some(q),
                )?;
                if bound(q)? != number(&original["prepared"], "locked_amount")? {
                    return Err("IdentityConflict：私有锁不是原计划全额".into());
                }
                additional = add(additional, bound(q)?)?;
                grants.push(json!({"grant_id":hash(&json!([REV,config.session,config.generation,slot["slot_id"],id,hash(q)])),"decision_id":id,"owner":q["owner"],"plan_hash":hash(q),"plan_id":q["plan_id"],"attempt_id":q["attempt_id"],"applicant_epoch":q["applicant_epoch"],"applicant_generation":q["applicant_generation"],"e_epoch":p["writer_epoch"],"amount":bound(q)?.to_string(),"quantity":q["quantity"],"units":original["prepared"]["units"],"policy":d["policy"],"prepared_cut":pf["receipt"]["cut"],"reserved_seq":seq.to_string(),"reserved_ns":ns,"execution_eligibility":"None"}));
            }
            // C06.3/C06.5：E 已独立保存的完整保守界覆盖该原事实与 token 时，E 保持该责任并核实清除
            // 对应 Pending，不等待参与 B 的 Applied（A 账不可读不挡健康 B 的新增）；
            // 未覆盖或原件内容不符仍封新增，不无条件忽略 Pending。
            let coverage = list(&d["baseline"], "covered_facts")?;
            let mut resolved = BTreeSet::new();
            for pending in list(previous, "pending")? {
                covered_pending(previous, coverage, pending)?;
                resolved.insert(required(pending, "fact_key")?.to_owned());
            }
            if !resolved.is_empty() {
                let kept: Vec<Value> = list(previous, "pending")?
                    .iter()
                    .filter(|p| !p["fact_key"].as_str().is_some_and(|k| resolved.contains(k)))
                    .cloned()
                    .collect();
                next["pending"] = json!(kept);
            }
            let responsibilities = list(f, "responsibilities")?;
            if responsibilities.is_empty() {
                return Err("MissingDependency：既有公共责任尚未规范导入 E".into());
            }
            let existing = responsibilities
                .iter()
                .try_fold(0i64, |a, r| add(a, number(r, "amount")?))?;
            let total = add(existing, additional)?;
            let all_fit = total <= number(&d["policy"], "cap")?;
            let r = json!({"slot_id":slot["slot_id"],"slot":slot,"state":if all_fit{"Reserved"}else{"AwaitingAllocationPolicy"},"policy_id":d["policy"]["policy_id"],"waiting_for":if all_fit{Value::Null}else{d["policy"]["allocation_policy_id"].clone()},"required_total":total.to_string(),"retained_total":if all_fit{total}else{existing}.to_string(),"grants":if all_fit{grants.clone()}else{vec![]},"closed_applicants":closed,"reserve_seq":seq.to_string(),"reserve_ns":ns});
            let f = &mut next["finance"];
            if all_fit {
                for g in grants {
                    push(
                        f,
                        "responsibilities",
                        json!({"responsibility_id":g["grant_id"],"owner":g["owner"],"amount":g["amount"],"grant":g}),
                    );
                }
            }
            push(f, "slots", r.clone());
            r
        }
        "CommitDecision" => {
            let q = plan(&d, p)?;
            let id = required(q, "decision_id")?;
            let old = decision(config, previous, id)?;
            if q["owner"] != config.domain
                || p["plan_hash"] != hash(q)
                || old["prepared"]["plan"] != *q
            {
                return Err("IdentityConflict：Commit 原计划/owner 不符".into());
            }
            if old["state"] != "Prepared" {
                return Err("InvalidDomain：本地状态不允许 Commit".into());
            }
            if number(&old["prepared"], "book_version_after")? != seq - 1 {
                return Err("InvalidDomain：Commit 本地版本 CAS 陈旧".into());
            }
            let gp = &w["grant_proof"];
            verify_proof(config, gp, "E")?;
            let slot = &gp["receipt"]["record"];
            let current_slot = item(
                list(finance(config, &gp["current_image"])?, "slots")?,
                "slot_id",
                required(q, "slot_id")?,
            )?;
            if slot["state"] != "Reserved" || current_slot != slot {
                return Err("InvalidDomain：实际 E grant 当前范围失效".into());
            }
            // C06.3/INTERFACE B.Commit：收到旧 grant 的首次 Commit 须核其当前范围。Reserve 后新增的
            // 未覆盖风险 token 使共同约束不可核，暂停受影响新增；已独立保存的界仍覆盖的责任保留。
            let coverage = list(&d["baseline"], "covered_facts")?;
            for pending in list(&gp["current_image"], "pending")? {
                covered_pending(&gp["current_image"], coverage, pending)?;
            }
            let closed = finance(config, &gp["current_image"])?
                .get("closed_applicants")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if closed.iter().any(|x| {
                x["slot_id"] == q["slot_id"]
                    && x["owner"] == q["owner"]
                    && x["applicant_epoch"] == q["applicant_epoch"]
                    && x["applicant_generation"] == q["applicant_generation"]
            }) {
                return Err("StaleWriter：原申请代际已关闭；全责任保留".into());
            }
            let g = item(list(slot, "grants")?, "grant_id", required(p, "grant_id")?)?;
            if g["decision_id"] != q["decision_id"]
                || g["plan_hash"] != hash(q)
                || g["owner"] != config.domain
                || g["amount"] != old["prepared"]["locked_amount"]
                || g["applicant_epoch"] != q["applicant_epoch"]
                || g["applicant_generation"] != q["applicant_generation"]
            {
                return Err("IdentityConflict：grant 不属于原 Prepared/量/代际".into());
            }
            let liability = item(
                list(finance(config, &gp["current_image"])?, "responsibilities")?,
                "responsibility_id",
                required(g, "grant_id")?,
            )?;
            if liability["grant"] != *g || liability["amount"] != g["amount"] {
                return Err("MissingDependency：E 未先保留完整责任".into());
            }
            if number(slot, "reserve_ns")? > nonnegative(ns, "commit_ns")? {
                return Err("InvalidDomain：B Commit 早于 E 增责".into());
            }
            verify_structure_for_domain(c, config, job.as_deref_mut(), &w["structure"], Some(q))?;
            let mut r = old.clone();
            r["state"] = json!("Committed");
            r["grant"] = json!({"original":g,"e_cut":gp["receipt"]["cut"]});
            r["local_commit_receipt"] = json!({"decision_id":id,"plan_hash":hash(q),"grant_id":g["grant_id"],"e_cut":gp["receipt"]["cut"],"book_version_before":(seq-1).to_string(),"book_version_after":seq.to_string(),"commit_ns":ns,"commit_hash":hash(&json!([old,g,gp["receipt"]["cut"],seq.to_string(),ns])),"execution_eligibility":"None"});
            let a = next["finance"]["decisions"].as_array_mut().unwrap();
            *a.iter_mut().find(|x| x["decision_id"] == id).unwrap() = r.clone();
            r
        }
        "AbortDecision" => {
            let q = plan(&d, p)?;
            let id = required(q, "decision_id")?;
            let old = decision(config, previous, id)?;
            if p["plan_hash"] != hash(q) || q["owner"] != config.domain {
                return Err("IdentityConflict：Abort 计划不符".into());
            }
            required(p, "reason")?;
            if old["state"] != "Prepared" {
                return Err("InvalidDomain：已提交历史不能 Abort".into());
            }
            let mut r = old.clone();
            r["state"] = json!("AbortedBeforeCommit");
            r["abort_reason"] = p["reason"].clone();
            r["abort_ns"] = json!(ns);
            // Abort 不证明 E/尾责可以释放，私有锁也不在本片擅自退款。
            *next["finance"]["decisions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|x| x["decision_id"] == id)
                .unwrap() = r.clone();
            r
        }
        "CloseApplicants" => {
            if config.domain != "E" {
                return Err("InvalidDomain：代际关闭仅 E".into());
            }
            required(p, "reason")?;
            let slot = item(list(&d, "slots")?, "slot_id", required(p, "slot_id")?)?;
            let a = list(p, "closed")?;
            if a.is_empty() {
                return Err("InvalidDomain：关闭申请者集合为空".into());
            }
            let mut owners = BTreeSet::new();
            for x in a {
                exact(x, &["owner", "applicant_epoch", "applicant_generation"])?;
                if !owners.insert(required(x, "owner")?)
                    || !list(&d, "plans")?.iter().any(|q| {
                        q["slot_id"] == slot["slot_id"]
                            && q["owner"] == x["owner"]
                            && q["applicant_epoch"] == x["applicant_epoch"]
                            && q["applicant_generation"] == x["applicant_generation"]
                    })
                {
                    return Err("StaleWriter：关闭对象不在原完整集合".into());
                }
                let mut x = x.clone();
                x["slot_id"] = slot["slot_id"].clone();
                if next["finance"].get("closed_applicants").is_none() {
                    next["finance"]["closed_applicants"] = json!([]);
                }
                push(&mut next["finance"], "closed_applicants", x);
            }
            json!({"slot_id":slot["slot_id"],"state":"ApplicantsClosed","closed":a,"reason":p["reason"],"commit_ns":ns,"responsibility_release":"None"})
        }
        _ => return Err("SchemaUnsupported：未支持财务写入".into()),
    };
    Ok((next, record))
}
fn latest_proof_seq(p: &Value) -> Result<i64, String> {
    let last = list(&p["current_image"], "history")?
        .last()
        .ok_or("MissingDependency：本地当前账无历史")?;
    positive(required(last, "commit_seq")?, "B current seq")
}
pub(super) fn receipt(config: &Config, event: &Value, cut: Value) -> Result<Value, String> {
    let kind = match required(event, "kind")? {
        "ImportResponsibility" => "ResponsibilityImportedReceipt",
        "PrepareDecision" => "PreparedReceipt",
        "Reserve" => "ReserveReceipt",
        "CommitDecision" => "LocalCommitReceipt",
        "AbortDecision" => "AbortReceipt",
        "CloseApplicants" => "ApplicantsClosedReceipt",
        _ => return Err("SchemaUnsupported：未知财务事件".into()),
    };
    Ok(
        json!({"kind":kind,"domain_id":config.domain,"record":event["record"],"first_known_ns":event["first_known_ns"],"commit_ns":event["commit_ns"],"cut":cut,"receipt_key":key_for(config, &event["request"]["payload"])?}),
    )
}
fn original(c: &Connection, event: &Value, config: &Config) -> Result<Value, String> {
    let e = validate_envelope(event["request"].clone(), config.revision()?)?;
    config.authorized(&e)?;
    let r =
        super::service::previous_message(c, &e)?.ok_or("StorageUnavailable：财务原消息绑定缺失")?;
    if r["receipt_key"] != key_for(config, &e["payload"])? {
        return Err("IdentityConflict：财务原身份/回执绑定不同".into());
    }
    Ok(e)
}
pub(super) fn replay(
    c: &Connection,
    config: &Config,
    previous: &Value,
    event: &Value,
    seq: i64,
    ns: &str,
    job: Option<&mut Job>,
) -> Result<Value, String> {
    exact(
        event,
        &[
            "kind",
            "record",
            "request",
            "witness",
            "received_ns",
            "first_known_ns",
            "commit_ns",
            "commit_seq",
        ],
    )?;
    let e = original(c, event, config)?;
    if event["kind"] != e["payload"]["op"]
        || event["commit_seq"] != seq.to_string()
        || event["commit_ns"] != ns
        || number(event, "received_ns")? > number(event, "first_known_ns")?
        || number(event, "first_known_ns")? > nonnegative(ns, "commit_ns")?
    {
        return Err("StorageUnavailable：财务事件阶段/时刻/身份不符".into());
    }
    let epoch = positive(required(&e["payload"], "writer_epoch")?, "event epoch")?;
    let at: String = c
        .query_row("SELECT at_ns FROM epochs WHERE epoch=?", [epoch], |r| {
            r.get(0)
        })
        .map_err(err)?;
    if number(event, "first_known_ns")? < nonnegative(&at, "epoch at_ns")?
        || list(previous, "history")?.last().is_some_and(|last| {
            number(last, "commit_ns")
                .is_ok_and(|n| n > number(event, "first_known_ns").unwrap_or(-1))
        })
    {
        return Err("StorageUnavailable：财务事件早于 writer/原账前沿".into());
    }
    knowledge_floor(&event["witness"], required(event, "first_known_ns")?)?;
    let (mut next, r) = reduce(c, config, previous, &e, &event["witness"], seq, ns, job)?;
    if r != event["record"] {
        return Err("StorageUnavailable：原财务事件重放不同".into());
    }
    push(&mut next, "history", event.clone());
    Ok(next)
}
pub(super) fn audit_receipt(c: &Connection, config: &Config, r: &Value) -> Result<(), String> {
    let seq = positive(required(&r["cut"], "commit_seq")?, "receipt seq")?;
    let (h, raw): (String, String) = c
        .query_row("SELECT root,image FROM commits WHERE seq=?", [seq], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(err)?;
    let image = strict_json(raw.as_bytes())?;
    let e = list(&image, "history")?
        .last()
        .ok_or("MissingDependency：财务事件缺失")?;
    if !is_event(e) || receipt(config, e, cut(config, seq, &h))? != *r {
        return Err("StorageUnavailable：财务回执不等于实际提交".into());
    }
    Ok(())
}
pub(super) fn audit_message(
    c: &Connection,
    config: &Config,
    request: &Value,
    r: &Value,
) -> Result<(), String> {
    validate_request(&request["payload"])?;
    if r["receipt_key"] != key_for(config, &request["payload"])? {
        return Err("StorageUnavailable：财务消息/回执键不符".into());
    }
    let seq = positive(required(&r["cut"], "commit_seq")?, "receipt seq")?;
    let raw: String = c
        .query_row("SELECT image FROM commits WHERE seq=?", [seq], |r| r.get(0))
        .map_err(err)?;
    let i = strict_json(raw.as_bytes())?;
    let e = list(&i, "history")?
        .last()
        .ok_or("MissingDependency：财务事件缺失")?;
    if e["request"]["payload"] != request["payload"] || e["kind"] != request["payload"]["op"] {
        return Err("IdentityConflict：重送改变财务内容".into());
    }
    config.authorized(request)?;
    Ok(())
}
pub(super) fn write(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
) -> Result<Value, String> {
    write_with_ingress(c, config, epoch, e, None)
}
pub(super) fn write_with_ingress(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    let p = &e["payload"];
    validate_request(p)?;
    let k = key_for(config, p)?;
    let mut shared_job = if revision(&config.manifest)? == REV_V2 {
        Some(new_content_job(c, config)?)
    } else {
        None
    };
    let deadline = if let Some(job) = shared_job.as_mut() {
        Some(operation_deadline(config, job, ingress_response_deadline)?)
    } else {
        None
    };
    // DeliveryUnknown 原身份优先查回，外部证据不可读不能推翻已提交结果。
    {
        let tx = immediate(c)?;
        check_epoch(&tx, epoch)?;
        // A queued retry may already be past its ingress response window.
        // Audit local committed truth and return its original receipt first;
        // only a genuinely new admission is subject to that deadline.
        audit_with_job(&tx, config, shared_job.as_mut())?;
        if let Some(r) = super::service::previous_message(&tx, e)? {
            return Ok(r);
        }
        if let Some(r) = row(&tx, "receipts", &k)? {
            audit_message(&tx, config, e, &r)?;
            super::service::bind_message(&tx, e, &k)?;
            tx.commit().map_err(err)?;
            return Ok(r);
        }
    }
    if p["writer_epoch"] != epoch.to_string() {
        return Err("StaleWriter：财务请求 writer_epoch 陈旧".into());
    }
    check_operation_deadline(shared_job.as_mut(), deadline)?;
    if revision(&config.manifest)? == REV_V2 {
        return write_v2(
            c,
            config,
            epoch,
            e,
            shared_job
                .as_mut()
                .ok_or("MissingDependency：新域操作 job 缺失")?,
            deadline.ok_or("MissingDependency：新域最早期限缺失")?,
        );
    }
    let raw = pinned_raw(config)?;
    let d = input(config, &raw)?;
    let mut w = json!({"input_raw":raw});
    match required(p, "op")? {
        "PrepareDecision" => {
            w["structure"] = structure(config)?;
        }
        "Reserve" => {
            let slot = item(list(&d, "slots")?, "slot_id", required(p, "slot_id")?)?;
            let (_, _, _, i) = latest(c)?;
            let closed = finance(config, &i)?
                .get("closed_applicants")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut proofs = vec![];
            for id in list(slot, "applicants")? {
                let q = item(list(&d, "plans")?, "decision_id", id.as_str().unwrap())?;
                if closed.iter().any(|x| {
                    x["slot_id"] == q["slot_id"]
                        && x["owner"] == q["owner"]
                        && x["applicant_epoch"] == q["applicant_epoch"]
                        && x["applicant_generation"] == q["applicant_generation"]
                }) {
                    continue;
                }
                proofs.push(proof(
                    config,
                    required(q, "owner")?,
                    &receipt_key_for(config, "PrepareDecision", required(q, "decision_id")?)?,
                )?);
            }
            w["prepared_proofs"] = json!(proofs);
            w["structure"] = structure(config)?;
        }
        "CommitDecision" => {
            let q = plan(&d, p)?;
            w["grant_proof"] = proof(
                config,
                "E",
                &receipt_key_for(config, "Reserve", required(q, "slot_id")?)?,
            )?;
            w["structure"] = structure(config)?;
        }
        "ImportResponsibility" | "AbortDecision" | "CloseApplicants" => (),
        _ => return Err("SchemaUnsupported：财务操作未知".into()),
    }
    persist(c, config, epoch, e, w)
}
fn write_v2(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    job: &mut Job,
    deadline: std::time::Instant,
) -> Result<Value, String> {
    // The initial published-history audit in `write` already succeeded under
    // this owner lock. A stopped/expired Job cannot gate cleanup, so failure
    // cleanup uses only the fixed carrier envelope and touches staging rows.
    match write_v2_inner(c, config, epoch, e, job, deadline) {
        Ok(receipt) => Ok(receipt),
        Err(error) => match super::content_store::discard_unpublished(c, &config.db) {
            Ok((removed, before, after)) => Err(format!("{error}; staged_cleanup rows={removed} allocated_before={before} allocated_after={after}")),
            Err(cleanup) => Err(format!("{error}; staged_cleanup_pending_recover={cleanup}")),
        },
    }
}
fn write_v2_inner(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    job: &mut Job,
    deadline: std::time::Instant,
) -> Result<Value, String> {
    let p = &e["payload"];
    let resources = super::financial_reader_resources::FinancialReaderResources::from_json(
        &config.manifest["finance"]["structure"]["reader_resources"],
    )?;
    check_operation_deadline(Some(&mut *job), Some(deadline))?;
    let raw = pinned_raw(config)?;
    let d = input(config, &raw)?;
    let mut witness = json!({"input_raw":raw});
    let mut peer_requests: Vec<(String, String)> = Vec::new();
    let mut object_ids = Vec::<String>::new();
    match required(p, "op")? {
        "PrepareDecision" => {
            let q = plan(&d, p)?;
            object_ids.push(required(&q["s_ref"], "object_id")?.to_owned());
        }
        "Reserve" => {
            let slot = item(list(&d, "slots")?, "slot_id", required(p, "slot_id")?)?;
            let (_, _, _, image) = latest(c)?;
            let closed = finance(config, &image)?
                .get("closed_applicants")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for id in list(slot, "applicants")? {
                let q = item(
                    list(&d, "plans")?,
                    "decision_id",
                    id.as_str().ok_or("SchemaUnsupported：applicant 非文本")?,
                )?;
                if closed.iter().any(|x| {
                    x["slot_id"] == q["slot_id"]
                        && x["owner"] == q["owner"]
                        && x["applicant_epoch"] == q["applicant_epoch"]
                        && x["applicant_generation"] == q["applicant_generation"]
                }) {
                    continue;
                }
                let object_id = required(&q["s_ref"], "object_id")?.to_owned();
                if !object_ids.contains(&object_id) {
                    object_ids.push(object_id);
                }
                peer_requests.push((
                    required(q, "owner")?.to_owned(),
                    receipt_key_for(config, "PrepareDecision", required(q, "decision_id")?)?,
                ));
            }
        }
        "CommitDecision" => {
            let q = plan(&d, p)?;
            object_ids.push(required(&q["s_ref"], "object_id")?.to_owned());
            peer_requests.push((
                "E".into(),
                receipt_key_for(config, "Reserve", required(q, "slot_id")?)?,
            ));
        }
        "ImportResponsibility" | "AbortDecision" | "CloseApplicants" => (),
        _ => return Err("SchemaUnsupported：财务操作未知".into()),
    }
    let token = format!(
        "stg-{}",
        hash(&json!([
            config.domain,
            epoch.to_string(),
            required(e, "message_id")?
        ]))
    );
    let mut stage =
        super::content_store::Stage::new(c, &config.db, resources.job_limits, token.clone())
            .map_err(|e| format!("Incomplete：E/B 暂存不可用：{e}"))?;
    if !peer_requests.is_empty() {
        let mut proofs = Vec::with_capacity(peer_requests.len());
        for (domain, receipt_key) in &peer_requests {
            proofs.push(proof_inner(
                config,
                domain,
                receipt_key,
                Some((&mut stage, job)),
                Some(deadline),
            )?);
        }
        if p["op"] == "Reserve" {
            witness["prepared_proofs"] = json!(proofs);
        } else {
            witness["grant_proof"] = proofs
                .into_iter()
                .next()
                .ok_or("MissingDependency：grant proof 缺失")?;
        }
    }
    if matches!(
        p["op"].as_str(),
        Some("PrepareDecision" | "Reserve" | "CommitDecision")
    ) {
        let basis = super::financial_content::read_current(
            &config.manifest["finance"]["structure"],
            &object_ids,
            job,
            deadline,
            &mut stage,
        )?;
        witness["structure"] = basis.evidence;
    }
    drop(stage);
    persist_with_job(
        c,
        config,
        epoch,
        e,
        witness,
        Some(job),
        Some(&token),
        Some(deadline),
    )
}
fn persist(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    w: Value,
) -> Result<Value, String> {
    persist_with_job(c, config, epoch, e, w, None, None, None)
}
fn persist_with_job(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    w: Value,
    mut job: Option<&mut Job>,
    stage_token: Option<&str>,
    deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    let final_budget = if revision(&config.manifest)? == REV_V2 {
        let shared = job
            .as_deref_mut()
            .ok_or("MissingDependency：新域缺统一证据 job")?;
        Some(
            super::content_store::reserve_final(c, &config.db, shared)
                .map_err(|e| format!("Incomplete：最终经济写入物理预留失败：{e}"))?,
        )
    } else {
        None
    };
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit_with_deadline(&tx, config, job.as_deref_mut(), deadline)?;
    if e["payload"]["writer_epoch"] != epoch.to_string() {
        return Err("StaleWriter：财务 writer 已换代".into());
    }
    if let Some(r) = super::service::previous_message(&tx, e)? {
        return Ok(r);
    }
    let (old, prev, last, image) = latest(&tx)?;
    let seq = add(old, 1)?;
    let recovered: String = tx
        .query_row(
            "SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let floor = nonnegative(&last, "last commit")?
        .max(nonnegative(&recovered, "recover_ns")?)
        .to_string();
    let (received, first, ns) = super::service::clocks(config, e, &floor, epoch)?;
    knowledge_floor(&w, &first)?;
    let (mut next, record) = reduce(&tx, config, &image, e, &w, seq, &ns, job.as_deref_mut())?;
    // C06.5：已核覆盖 token 的清除须与 E 本事务持久状态一致，不能只改返回 JSON。
    super::store::sync_pending(&tx, &next)?;
    let event = json!({"kind":e["payload"]["op"],"record":record,"request":e,"witness":w,"received_ns":received,"first_known_ns":first,"commit_ns":ns,"commit_seq":seq.to_string()});
    push(&mut next, "history", event.clone());
    if revision(&config.manifest)? == REV_V2 {
        let refs = super::content_store::image_refs(&next)
            .map_err(|e| format!("MissingDependency：经济图像引用路径非法：{e}"))?;
        super::content_store::publish(
            &tx,
            &refs,
            job.as_deref_mut()
                .ok_or("MissingDependency：新域缺统一证据 job")?,
        )
        .map_err(|e| format!("MissingDependency：本地内容闭包未完整持久：{e}"))?;
        job.as_deref_mut()
            .ok_or("MissingDependency：新域缺统一证据 job")?
            .pre_io_check()
            .map_err(|e| format!("Incomplete：经济提交前 job 期限：{e}"))?;
    }
    let cut = append(&tx, config, seq, &prev, &ns, &next)?;
    let r = receipt(config, &event, cut)?;
    let k = key_for(config, &e["payload"])?;
    put(&tx, "receipts", &k, &r)?;
    super::service::bind_message(&tx, e, &k)?;
    config.pause(required(e, "message_id")?, "before_commit")?;
    check_operation_deadline(job.as_deref_mut(), deadline)?;
    tx.commit().map_err(err)?;
    if let Some((grant, before, ceiling)) = final_budget {
        super::content_store::reconcile_final(
            c,
            &config.db,
            job.as_deref_mut()
                .ok_or("DeliveryUnknown：已提交但 job 丢失")?,
            &grant,
            before,
            ceiling,
        )
        .map_err(|e| format!("DeliveryUnknown：经济提交已发生，物理对账/回收待核：{e}"))?;
    }
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(r)
}
pub(super) fn query(c: &Connection, config: &Config, p: &Value) -> Result<Value, String> {
    let (_, _, _, image) = super::service::select_cut(c, config, p)?;
    let op = required(p, "op")?;
    if op == "QueryDecision" {
        exact(p, &["op", "decision_id", "cut", "as_known_ns"])?;
        let d = decision(config, &image, required(p, "decision_id")?)?;
        let mut receipts = vec![];
        for event in list(&image, "history")?
            .iter()
            .filter(|x| is_event(x) && x["record"]["decision_id"] == p["decision_id"])
        {
            receipts.push(
                row(
                    c,
                    "receipts",
                    &key_for(config, &event["request"]["payload"])?,
                )?
                .ok_or("StorageUnavailable：决定缺原回执")?,
            );
        }
        Ok(
            json!({"kind":"EconomicDecision","domain_id":config.domain,"decision":d,"receipts":receipts}),
        )
    } else {
        exact(p, &["op", "grant_id", "cut", "as_known_ns"])?;
        for slot in list(finance(config, &image)?, "slots")? {
            if let Some(g) = list(slot, "grants")?
                .iter()
                .find(|g| g["grant_id"] == p["grant_id"])
            {
                return Ok(
                    json!({"kind":"EconomicGrant","domain_id":config.domain,"grant":g,"receipt":row(c,"receipts",&receipt_key_for(config,"Reserve",required(slot,"slot_id")?)?)?.ok_or("StorageUnavailable：grant 缺原回执")?}),
                );
            }
        }
        Err("MissingDependency：该 cut 尚无原 grant".into())
    }
}
/// 业务方只给关闭 slot 的原身份。跨重协议排序由 E 模块持有，不由 UI 拼装。
pub(super) fn advance(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
) -> Result<Value, String> {
    let p = &e["payload"];
    exact(
        p,
        &[
            "op",
            "slot_id",
            "request_set_hash",
            "writer_epoch",
            "owner_epochs",
        ],
    )?;
    let epochs = p["owner_epochs"]
        .as_object()
        .ok_or("MissingDependency：advance 缺监督器核准的各 owner epoch 向量")?;
    if epochs.len() != config.manifest["chongs"].as_array().unwrap().len() + 1
        || epochs.get("E") != Some(&p["writer_epoch"])
    {
        return Err("IdentityConflict：advance epoch 向量不完整".into());
    }
    for route in config.manifest["chongs"].as_array().unwrap() {
        positive(
            required(
                &p["owner_epochs"],
                &format!("B:{}", required(route, "chong_id")?),
            )?,
            "approved owner epoch",
        )?;
    }
    if config.domain != "E" || p["writer_epoch"] != epoch.to_string() {
        return Err("StaleWriter：advance 仅当前 E".into());
    }
    let raw = pinned_raw(config)?;
    let d = input(config, &raw)?;
    let slot = item(list(&d, "slots")?, "slot_id", required(p, "slot_id")?)?;
    if p["request_set_hash"] != hash(&slot["applicants"]) {
        return Err("IdentityConflict：advance 请求集内容改变".into());
    }
    let reserved = row(
        c,
        "receipts",
        &receipt_key_for(config, "Reserve", required(slot, "slot_id")?)?,
    )?;
    if reserved.is_none() {
        for id in list(slot, "applicants")? {
            let q = item(list(&d, "plans")?, "decision_id", id.as_str().unwrap())?;
            let payload = json!({"op":"PrepareDecision","decision_id":id,"plan_hash":hash(q),"writer_epoch":p["owner_epochs"][required(q,"owner")?]});
            super::finance_transport::call(
                config,
                required(q, "owner")?,
                required(&p["owner_epochs"], required(q, "owner")?)?,
                internal(config, payload)?,
            )?;
        }
    }
    let reserve_request = internal(
        config,
        json!({"op":"Reserve","slot_id":slot["slot_id"],"request_set_hash":hash(&slot["applicants"]),"writer_epoch":epoch.to_string()}),
    )?;
    config.authorized(&reserve_request)?;
    let reserve = if let Some(r) = reserved {
        r
    } else {
        write(c, config, epoch, &reserve_request)?
    };
    if reserve["record"]["state"] == "AwaitingAllocationPolicy" {
        return Ok(reserve);
    }
    let mut commits = vec![];
    // 已增责后逐 B 独立完成；A 不可读不得挡 B，所有失败如实返回且保全 E 责任。
    for g in list(&reserve["record"], "grants")? {
        let owner = required(g, "owner")?;
        let expected = required(&p["owner_epochs"], owner)?;
        let result = (|| {
            let query = json!({"op":"QueryDecision","decision_id":g["decision_id"],"cut":null,"as_known_ns":null});
            let state =
                super::finance_transport::call(config, owner, expected, internal(config, query)?)?;
            if state["decision"]["state"] == "Committed" {
                let r = list(&state, "receipts")?
                    .iter()
                    .find(|r| r["kind"] == "LocalCommitReceipt")
                    .ok_or("MissingDependency：Committed 缺原回执")?;
                if r["record"]["grant"]["original"] != *g {
                    return Err("IdentityConflict：原 Commit 不属于此 grant".into());
                }
                return Ok(r.clone());
            }
            let payload = json!({"op":"CommitDecision","decision_id":g["decision_id"],"plan_hash":g["plan_hash"],"grant_id":g["grant_id"],"writer_epoch":expected});
            super::finance_transport::call(config, owner, expected, internal(config, payload)?)
        })();
        commits.push(match result {
            Ok(r) => json!({"owner":g["owner"],"status":"Committed","receipt":r}),
            Err(error) => json!({"owner":g["owner"],"status":"Unavailable","error":error}),
        });
    }
    Ok(
        json!({"kind":"ChongAdvance","domain_id":"E","slot_id":slot["slot_id"],"reserve_receipt":reserve,"commits":commits,"execution_eligibility":"None"}),
    )
}
fn internal(config: &Config, payload: Value) -> Result<Value, String> {
    let mut e = config.manifest["finance"]["coordinator_source"].clone();
    let m = e
        .as_object_mut()
        .ok_or("MissingDependency：模块协议身份缺失")?;
    for (k, v) in [
        ("schema_revision", json!(config.revision()?)),
        ("session_id", json!(config.session)),
        ("session_generation", json!(config.generation)),
        (
            "message_id",
            json!(format!(
                "tb03b-{}-{}",
                required(&payload, "op")?,
                operation_id(&payload)?
            )),
        ),
        ("payload_hash", json!(hash(&payload))),
        ("causal_refs", json!([])),
        ("payload", payload),
    ] {
        m.insert(k.into(), v);
    }
    validate_envelope(e, config.revision()?)
}

#[cfg(test)]
#[path = "finance_tests.rs"]
mod tests;
