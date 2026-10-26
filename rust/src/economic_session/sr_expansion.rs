//! #1717 I2：SR 扩量新版本在 E 的正式事实比较、持久评估与恢复（SPEC #1340 增量第五节）。
//!
//! 事实只取 E 最新已提交 cut 上 ImportResponsibility/Reserve 已形成的责任；caller 只给出它
//! 所见的 cut（CAS）和政策原件，不能自报“条件已成立”。政策原件随评估事件持久，重放与恢复
//! 只用事件自带的原件，没有“最新配置”可追算旧评估。动作候选只登记为 NotApplied：
//! 冻结新承担与同一退出由共享的冻结/退出授权入口消费，本模块不应用、不另建执行器。
//! 本切片只认具名 TestOnly 政策与 TestOnly 合成签署；生产签署合同与生产事实指标未批准。
//!
//! 新语义是与旧合同并列的子合同 `sr-expansion/1`：manifest 顶层 `sr_expansion` 显式选择，E 在 init
//! 绑定、audit 核对，旧域不原地扩义。它不占财务版本号，声明在 `economic-session/1` 与内容域
//! `economic-session/2`（`tb03b-finance/2` 起的具名财务责任）上。`/2` 的评估与财务写口共用同一形状：
//! 一个统一 job 自首次依赖读取起覆盖审计、事实比较、本地闭包核验、最终物理预留、提交与对账，取
//! 该 job 与入口响应的最早绝对期限。签署引用只认该节冻结的授权原件；授权原件随 manifest_hash 冻结，
//! 同一具名政策版本只能对应一份判据，改值必须登记新版本。
use super::book::hash;
use super::store::*;
use crate::session_protocol::{
    nonnegative, positive, required, sha256_hex, strict_json, validate_envelope,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const OP: &str = "EvaluateSrExpansion";
pub(super) const QUERY: &str = "QuerySrEvaluation";
pub(super) const RECEIPT: &str = "SrExpansionEvaluated";
const POLICY_REVISION: &str = "sr-expansion-policy/1";
const EVALUATION_REVISION: &str = "sr-expansion-evaluation/1";
const POLICY_LIMIT: usize = 65_536;
const FACT_SOURCE: &str = "E:finance.responsibilities";
const FACT_METRIC: &str = "gross_responsibility_total";
const FACT_FRONTIER: &str = "latest_committed_cut";
pub(super) const CONTRACT_FIELD: &str = "sr_expansion";
pub(super) const CONTRACT_REVISION: &str = "sr-expansion/1";
pub(super) const META_KEY: &str = "sr_expansion_revision";
const AUTHORIZATION_KIND: &str = "TestOnlySynthetic";

#[derive(Clone, Copy)]
enum Comparison {
    AtLeast,
    Above,
    AtMost,
    Below,
}
impl Comparison {
    fn parse(v: &Value) -> Result<Self, String> {
        match v.as_str() {
            Some(">=") => Ok(Self::AtLeast),
            Some(">") => Ok(Self::Above),
            Some("<=") => Ok(Self::AtMost),
            Some("<") => Ok(Self::Below),
            _ => Err("SchemaUnsupported：SR 比较只认 >=、>、<=、<".into()),
        }
    }
    fn holds(self, fact: i64, threshold: i64) -> bool {
        match self {
            Self::AtLeast => fact >= threshold,
            Self::Above => fact > threshold,
            Self::AtMost => fact <= threshold,
            Self::Below => fact < threshold,
        }
    }
}

enum Scope {
    AllChongs,
    Symbol(String),
}

struct Entry {
    id: String,
    enabled: bool,
    unit: (String, String),
    comparison: Comparison,
    /// `None` 即 AwaitingPolicy：未签不带数值，也没有备用值。
    signed_threshold: Option<i64>,
    reference: Option<String>,
    scope: Scope,
    raw: Value,
}

struct Policy {
    identity: Value,
    policy_id: String,
    version: String,
    /// 条目按 entry_id 排序后的规范原文：条目换序不是新判据，任何取值变化都是。
    canonical: String,
    entries: Vec<Entry>,
}

struct Authorization {
    reference: String,
    policy_id: String,
    version: String,
    canonical: String,
}

struct Fact {
    value: i64,
    unit: (String, String),
    record: Value,
}

fn exact(v: &Value, keys: &[&str], what: &str) -> Result<(), String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("SchemaUnsupported：{what}须对象"))?;
    if o.len() != keys.len() || o.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err(format!("SchemaUnsupported：{what}字段不完整或多余"));
    }
    Ok(())
}

pub(super) fn receipt_key(evaluation_id: &str) -> String {
    format!("{EVALUATION_REVISION}:{}", hash(&json!(evaluation_id)))
}

fn validate_request(p: &Value) -> Result<(), String> {
    exact(
        p,
        &[
            "op",
            "evaluation_id",
            "policy_raw_utf8",
            "fact_cut",
            "writer_epoch",
        ],
        "SR 评估请求",
    )?;
    if p["op"] != OP {
        return Err("SchemaUnsupported：不是 SR 扩量评估请求".into());
    }
    required(p, "evaluation_id")?;
    if required(p, "policy_raw_utf8")?.len() > POLICY_LIMIT {
        return Err("InvalidDomain：SR 政策原件超限".into());
    }
    exact(
        &p["fact_cut"],
        &["domain_id", "commit_seq", "root_hash"],
        "SR 事实 cut",
    )?;
    positive(required(p, "writer_epoch")?, "SR writer_epoch")?;
    Ok(())
}

fn entry(v: &Value) -> Result<Entry, String> {
    exact(
        v,
        &[
            "entry_id",
            "status",
            "fact",
            "comparison",
            "threshold",
            "scope",
            "action",
        ],
        "SR 扩量条目",
    )?;
    let enabled = match v["status"].as_str() {
        Some("disabled") => false,
        Some("enabled") => true,
        _ => return Err("SchemaUnsupported：SR 扩量条目状态只认 disabled/enabled".into()),
    };
    let f = &v["fact"];
    exact(
        f,
        &["source", "metric", "frontier", "currency", "money_unit"],
        "SR 事实声明",
    )?;
    if f["source"] != FACT_SOURCE || f["metric"] != FACT_METRIC || f["frontier"] != FACT_FRONTIER {
        return Err(
            "UnsupportedCapability：SR 扩量事实只支持 E 最新已提交 cut 的毛责任合计（TestOnly）"
                .into(),
        );
    }
    let (signed_threshold, reference) = match &v["threshold"] {
        Value::String(s) if s == "AwaitingPolicy" => (None, None),
        t @ Value::Object(_) => {
            exact(t, &["value", "signature"], "SR 已签阈值")?;
            exact(&t["signature"], &["kind", "reference"], "SR 签署")?;
            if t["signature"]["kind"] != AUTHORIZATION_KIND {
                return Err("UnsupportedCapability：本切片只认 TestOnly 合成签署".into());
            }
            (
                Some(nonnegative(required(t, "value")?, "SR threshold")?),
                Some(required(&t["signature"], "reference")?.to_owned()),
            )
        }
        _ => return Err("SchemaUnsupported：SR 阈值只能是 AwaitingPolicy 或带签署的数值".into()),
    };
    let s = &v["scope"];
    let scope = match s["kind"].as_str() {
        Some("all_chongs") => {
            exact(s, &["kind"], "SR 作用域")?;
            Scope::AllChongs
        }
        Some("symbol") => {
            exact(s, &["kind", "symbol"], "SR 作用域")?;
            Scope::Symbol(required(s, "symbol")?.to_owned())
        }
        _ => return Err("InvalidDomain：SR 作用域只允许全部重或某标的全部重（非选择性）".into()),
    };
    if !matches!(
        v["action"].as_str(),
        Some("freeze_new" | "exit_all_in_scope")
    ) {
        return Err("InvalidDomain：SR 动作只允许 freeze_new 或 exit_all_in_scope".into());
    }
    Ok(Entry {
        id: required(v, "entry_id")?.to_owned(),
        enabled,
        unit: (
            required(f, "currency")?.to_owned(),
            required(f, "money_unit")?.to_owned(),
        ),
        comparison: Comparison::parse(&v["comparison"])?,
        signed_threshold,
        reference,
        scope,
        raw: v.clone(),
    })
}

fn policy(raw: &str) -> Result<Policy, String> {
    let mut parsed = parse_policy(&strict_json(raw.as_bytes())?)?;
    parsed.identity["raw_sha256"] = json!(sha256_hex(raw.as_bytes()));
    Ok(parsed)
}

fn parse_policy(p: &Value) -> Result<Policy, String> {
    exact(
        p,
        &[
            "schema_revision",
            "kind",
            "policy_id",
            "version",
            "supersedes",
            "entries",
        ],
        "SR 政策原件",
    )?;
    if p["schema_revision"] != POLICY_REVISION {
        return Err("SchemaUnsupported：SR 政策原件只认 sr-expansion-policy/1".into());
    }
    if p["kind"] != "TestOnly" {
        return Err("UnsupportedCapability：SR 扩量生产签署合同未批准，只认具名 TestOnly".into());
    }
    let policy_id = required(p, "policy_id")?.to_owned();
    let version = required(p, "version")?.to_owned();
    if version == "v1" {
        return Err("InvalidDomain：v1 是旧 SR 原件，扩量只能是新版本".into());
    }
    if p["supersedes"] != "v1" {
        return Err("InvalidDomain：SR 扩量新版本须 supersedes v1".into());
    }
    let mut entries = p["entries"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("SchemaUnsupported：SR 扩量条目须非空数组")?
        .iter()
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    if entries.windows(2).any(|w| w[0].id == w[1].id) {
        return Err("IdentityConflict：SR 扩量条目 id 重复".into());
    }
    let mut canonical = p.clone();
    canonical["entries"] = Value::Array(entries.iter().map(|e| e.raw.clone()).collect());
    Ok(Policy {
        identity: json!({"schema_revision":p["schema_revision"],"kind":p["kind"],"policy_id":p["policy_id"],"version":p["version"],"supersedes":p["supersedes"]}),
        policy_id,
        version,
        canonical: crate::session_protocol::canonical_json(&canonical),
        entries,
    })
}

/// 子合同 `sr-expansion/1` 的冻结授权原件表；缺省或 null 即本域未选择新合同。
fn registry(manifest: &Value) -> Result<Option<Vec<Authorization>>, String> {
    let section = match manifest.get(CONTRACT_FIELD) {
        None | Some(Value::Null) => return Ok(None),
        Some(section) => section,
    };
    exact(
        section,
        &["schema_revision", "authorizations"],
        "SR 扩量子合同",
    )?;
    if section["schema_revision"] != CONTRACT_REVISION {
        return Err("SchemaUnsupported：SR 扩量子合同只认 sr-expansion/1".into());
    }
    if !manifest.get("finance").is_some_and(|v| !v.is_null()) {
        return Err("InvalidDomain：SR 扩量子合同须同时冻结具名财务责任".into());
    }
    let mut registry: Vec<Authorization> = Vec::new();
    for a in section["authorizations"]
        .as_array()
        .ok_or("SchemaUnsupported：SR 授权原件表须数组")?
    {
        exact(a, &["reference", "kind", "policy"], "SR 授权原件")?;
        if a["kind"] != AUTHORIZATION_KIND {
            return Err("UnsupportedCapability：本切片只认 TestOnly 合成授权原件".into());
        }
        let reference = required(a, "reference")?.to_owned();
        let p = parse_policy(&a["policy"])?;
        if p.entries
            .iter()
            .any(|e| e.reference.as_ref().is_some_and(|r| *r != reference))
        {
            return Err("IdentityConflict：SR 授权原件内的签署引用须指向本授权".into());
        }
        if registry.iter().any(|o| {
            o.reference == reference || (o.policy_id == p.policy_id && o.version == p.version)
        }) {
            return Err("IdentityConflict：SR 授权引用或具名政策版本重复".into());
        }
        registry.push(Authorization {
            reference,
            policy_id: p.policy_id,
            version: p.version,
            canonical: p.canonical,
        });
    }
    Ok(Some(registry))
}

pub(super) fn validate_config(manifest: &Value) -> Result<bool, String> {
    Ok(registry(manifest)?.is_some())
}

/// init 绑定、audit 核对的持久子合同版本：只有选择了本合同的 E 有此 meta 行。
pub(super) fn bound_revision(config: &Config) -> Result<Option<&'static str>, String> {
    Ok((config.domain == "E" && validate_config(&config.manifest)?).then_some(CONTRACT_REVISION))
}

/// 签署引用必须解析到冻结授权原件，具名版本的判据必须与该原件逐值相同。
fn authorize<'a>(
    registry: &'a [Authorization],
    policy: &Policy,
) -> Result<&'a Authorization, String> {
    for reference in policy.entries.iter().filter_map(|e| e.reference.as_ref()) {
        if !registry.iter().any(|a| a.reference == *reference) {
            return Err("UnsupportedCapability：SR 签署引用未登记冻结授权原件".into());
        }
    }
    let authorization = registry
        .iter()
        .find(|a| a.policy_id == policy.policy_id && a.version == policy.version)
        .ok_or("UnsupportedCapability：SR 具名政策版本未登记冻结授权原件")?;
    if authorization.canonical != policy.canonical {
        return Err(
            "IdentityConflict：SR 具名政策版本已冻结为另一份判据，签署原件与请求内容不符".into(),
        );
    }
    Ok(authorization)
}

/// 已核事实：每笔责任都回到同一图像历史中实际提交它的 ImportResponsibility 或 Reserve 事件。
fn fact(config: &Config, image: &Value, at: &Value) -> Result<Fact, String> {
    if !super::finance::enabled(config) {
        return Err("MissingDependency：E 未配置具名财务责任".into());
    }
    let responsibilities = image["finance"]["responsibilities"]
        .as_array()
        .ok_or("StorageUnavailable：财务责任形状损坏")?;
    if responsibilities.is_empty() {
        return Err("MissingDependency：既有公共责任尚未规范导入 E，SR 事实不存在".into());
    }
    let history = image["history"]
        .as_array()
        .ok_or("StorageUnavailable：历史形状损坏")?;
    let mut value = 0i64;
    let mut unit: Option<(String, String)> = None;
    let mut components = Vec::new();
    for r in responsibilities {
        let id = &r["responsibility_id"];
        let (terms, origin) = if r["owner"] == "E:baseline" {
            let origin = history.iter().find(|e| {
                e["kind"] == "ImportResponsibility"
                    && e["record"]["original"]["responsibility_id"] == *id
            });
            (&r["policy"], origin)
        } else {
            let origin = history.iter().find(|e| {
                e["kind"] == "Reserve"
                    && e["commit_seq"] == r["grant"]["reserved_seq"]
                    && e["record"]["grants"]
                        .as_array()
                        .is_some_and(|g| g.iter().any(|g| g["grant_id"] == *id))
            });
            (&r["grant"]["policy"], origin)
        };
        let origin = origin.ok_or("StorageUnavailable：责任缺实际提交事件")?;
        let u = (
            required(terms, "currency")?.to_owned(),
            required(terms, "money_unit")?.to_owned(),
        );
        if unit.as_ref().is_some_and(|x| *x != u) {
            return Err("IdentityConflict：责任事实单位不一致，不能求和".into());
        }
        unit = Some(u);
        value = value
            .checked_add(nonnegative(
                required(r, "amount")?,
                "responsibility amount",
            )?)
            .ok_or("InvalidDomain：责任合计溢出")?;
        components.push(json!({"responsibility_id":id,"owner":r["owner"],"amount":r["amount"],"origin":{"event":origin["kind"],"commit_seq":origin["commit_seq"],"commit_ns":origin["commit_ns"]}}));
    }
    let unit = unit.ok_or("MissingDependency：SR 事实缺单位")?;
    Ok(Fact {
        value,
        record: json!({"source":FACT_SOURCE,"metric":FACT_METRIC,"frontier":FACT_FRONTIER,"cut":at,"value":value.to_string(),"currency":unit.0,"money_unit":unit.1,"components":components}),
        unit,
    })
}

fn scope_members(config: &Config, scope: &Scope) -> Result<Vec<String>, String> {
    let members: BTreeSet<String> = config.manifest["chongs"]
        .as_array()
        .ok_or("InvalidDomain：重路由为空")?
        .iter()
        .filter(|c| match scope {
            Scope::AllChongs => true,
            Scope::Symbol(s) => c["symbol"] == s.as_str(),
        })
        .map(|c| required(c, "chong_id").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    if members.is_empty() {
        return Err("InvalidDomain：SR 作用域在本 session 没有任何重".into());
    }
    Ok(members.into_iter().collect())
}

/// 纯评估：同一 (cut 图像, 请求) 永远得到同一记录；写入与重放共用这一份。
fn evaluate(config: &Config, image: &Value, at: &Value, p: &Value) -> Result<Value, String> {
    validate_request(p)?;
    if p["fact_cut"] != *at {
        return Err("InvalidDomain：SR 事实前沿不是本域最新已提交 cut（CAS 陈旧或越界）".into());
    }
    let mut policy = policy(required(p, "policy_raw_utf8")?)?;
    let registry = registry(&config.manifest)?
        .ok_or("SchemaUnsupported：本域未选择 sr-expansion/1 合同，不接受 SR 扩量评估")?;
    let authorization = authorize(&registry, &policy)?;
    policy.identity["authorization"] = json!({"reference":authorization.reference,"kind":AUTHORIZATION_KIND,"content_sha256":sha256_hex(authorization.canonical.as_bytes())});
    let fact = fact(config, image, at)?;
    let mut entries = Vec::new();
    let mut actions: BTreeMap<(String, String), (Value, Vec<String>, Vec<String>)> =
        BTreeMap::new();
    for e in &policy.entries {
        if e.unit != fact.unit {
            return Err("IdentityConflict：SR 条目单位与已核事实单位不同".into());
        }
        let members = scope_members(config, &e.scope)?;
        let outcome = match (e.enabled, e.signed_threshold) {
            (false, _) => "Disabled",
            (true, None) => "AwaitingPolicy",
            (true, Some(t)) if e.comparison.holds(fact.value, t) => "Triggered",
            (true, Some(_)) => "NotTriggered",
        };
        if outcome == "Triggered" {
            let action = required(&e.raw, "action")?.to_owned();
            let key = (
                action,
                crate::session_protocol::canonical_json(&e.raw["scope"]),
            );
            actions
                .entry(key)
                .or_insert_with(|| (e.raw["scope"].clone(), members.clone(), Vec::new()))
                .2
                .push(e.id.clone());
        }
        entries.push(json!({"entry_id":e.id,"status":e.raw["status"],"outcome":outcome,"comparison":e.raw["comparison"],"threshold":e.raw["threshold"],"scope":e.raw["scope"],"scope_members":members,"action":e.raw["action"]}));
    }
    let actions: Vec<Value> = actions
        .into_iter()
        .map(|((action, _), (scope, members, ids))| json!({"action":action,"scope":scope,"scope_members":members,"entry_ids":ids,"application_state":"NotApplied"}))
        .collect();
    Ok(json!({
        "schema_revision":EVALUATION_REVISION,
        "evaluation_id":p["evaluation_id"],
        "policy":policy.identity,
        "fact":fact.record,
        "entries":entries,
        "result":if actions.is_empty(){"NoAction"}else{"ActionCandidates"},
        "actions":actions
    }))
}

fn receipt(config: &Config, event: &Value, cut: Value) -> Value {
    json!({"kind":RECEIPT,"domain_id":config.domain,"evaluation_id":event["record"]["evaluation_id"],"record":event["record"],"first_known_ns":event["first_known_ns"],"commit_ns":event["commit_ns"],"cut":cut})
}

/// 正式 E 写口，与财务写口同形：先按原身份取回已提交回执，再在一个 immediate 事务内审计、CAS
/// 事实前沿、评估、核本地闭包、追加提交、回执与消息绑定。`/2` 的统一 job 覆盖从首次依赖读取到
/// 提交后物理对账的全部读写，任何失败都发生在 COMMIT 之前，不留经济提交。
pub(super) fn write(
    c: &mut Connection,
    config: &Config,
    epoch: i64,
    e: &Value,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<Value, String> {
    let p = &e["payload"];
    validate_request(p)?;
    if config.domain != "E" {
        return Err("SchemaUnsupported：SR 扩量评估只属于 E".into());
    }
    let key = receipt_key(required(p, "evaluation_id")?);
    let mut job = if config.revision()? == SCHEMA_V2 {
        Some(new_content_job(c, config)?)
    } else {
        None
    };
    let deadline = match job.as_mut() {
        Some(job) => Some(operation_deadline(config, job, ingress_response_deadline)?),
        None => ingress_response_deadline,
    };
    {
        let tx = immediate(c)?;
        check_epoch(&tx, epoch)?;
        audit_with_job(&tx, config, job.as_mut())?;
        if let Some(r) = super::service::previous_message(&tx, e)? {
            return Ok(r);
        }
        if let Some(r) = row(&tx, "receipts", &key)? {
            audit_message(&tx, config, e, &r, &mut ReadBudget::new(job.as_mut(), None))?;
            super::service::bind_message(&tx, e, &key)?;
            tx.commit().map_err(err)?;
            return Ok(r);
        }
    }
    if p["writer_epoch"] != epoch.to_string() {
        return Err("StaleWriter：SR 评估 writer_epoch 陈旧".into());
    }
    check_operation_deadline(job.as_mut(), deadline)?;
    let final_budget = match job.as_mut() {
        Some(job) => Some(
            super::content_store::reserve_final(c, &config.db, job)
                .map_err(|e| format!("Incomplete：SR 评估最终物理预留失败：{e}"))?,
        ),
        None => None,
    };
    let tx = immediate(c)?;
    check_epoch(&tx, epoch)?;
    audit_with_deadline(&tx, config, job.as_mut(), deadline)?;
    if let Some(r) = super::service::previous_message(&tx, e)? {
        return Ok(r);
    }
    let (seq, root, last_ns, mut image) = latest(&tx)?;
    let record = evaluate(config, &image, &cut(config, seq, &root), p)?;
    let recovered: String = tx
        .query_row(
            "SELECT at_ns FROM epochs ORDER BY epoch DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let floor = nonnegative(&last_ns, "last commit")?
        .max(nonnegative(&recovered, "epoch at_ns")?)
        .to_string();
    let (received, first, ns) = super::service::clocks(config, e, &floor, epoch)?;
    let seq = seq.checked_add(1).ok_or("InvalidDomain：commit seq溢出")?;
    let event = json!({"kind":OP,"record":record,"request":e,"received_ns":received,"first_known_ns":first,"commit_ns":ns,"commit_seq":seq.to_string()});
    image["history"]
        .as_array_mut()
        .ok_or("StorageUnavailable：历史形状损坏")?
        .push(event.clone());
    if let Some(job) = job.as_mut() {
        // 评估事件不新增内容引用；新图像仍须整体闭包已在本 owner 持久，才可发布。
        let refs = super::content_store::image_refs(&image)
            .map_err(|e| format!("MissingDependency：SR 评估图像引用路径非法：{e}"))?;
        super::content_store::publish(&tx, &refs, job)
            .map_err(|e| format!("MissingDependency：本地内容闭包未完整持久：{e}"))?;
        job.pre_io_check()
            .map_err(|e| format!("Incomplete：SR 评估提交前 job 期限：{e}"))?;
    }
    let committed = append(&tx, config, seq, &root, &ns, &image)?;
    let r = receipt(config, &event, committed);
    put(&tx, "receipts", &key, &r)?;
    super::service::bind_message(&tx, e, &key)?;
    config.pause(required(e, "message_id")?, "before_commit")?;
    check_operation_deadline(job.as_mut(), deadline)?;
    tx.commit().map_err(err)?;
    if let Some((grant, before, ceiling)) = final_budget {
        super::content_store::reconcile_final(
            c,
            &config.db,
            job.as_mut()
                .ok_or("DeliveryUnknown：SR 评估已提交但 job 缺失")?,
            &grant,
            before,
            ceiling,
        )
        .map_err(|e| format!("DeliveryUnknown：SR 评估已提交，物理对账待核：{e}"))?;
    }
    config.pause(required(e, "message_id")?, "after_commit")?;
    Ok(r)
}

/// 审计重放：从原消息与前一提交图像重算，必须逐位等于已提交事件。
pub(super) fn replay(
    c: &Connection,
    config: &Config,
    previous: &Value,
    event: &Value,
    seq: i64,
    ns: &str,
    budget: &mut ReadBudget<'_>,
) -> Result<Value, String> {
    exact(
        event,
        &[
            "kind",
            "record",
            "request",
            "received_ns",
            "first_known_ns",
            "commit_ns",
            "commit_seq",
        ],
        "SR 评估事件",
    )?;
    let e = validate_envelope(event["request"].clone(), config.revision()?)?;
    config.authorized(&e)?;
    let bound = super::service::previous_message_metered(c, &e, budget)?
        .ok_or("StorageUnavailable：SR 评估原消息绑定缺失")?;
    let p = &e["payload"];
    validate_request(p)?;
    let first = nonnegative(required(event, "first_known_ns")?, "first_known_ns")?;
    if config.domain != "E"
        || bound["kind"] != RECEIPT
        || bound["evaluation_id"] != p["evaluation_id"]
        || event["commit_seq"] != seq.to_string()
        || event["commit_ns"] != ns
        || nonnegative(required(event, "received_ns")?, "received_ns")? > first
        || first > nonnegative(ns, "commit_ns")?
    {
        return Err("StorageUnavailable：SR 评估事件阶段/时刻/身份不符".into());
    }
    let writer = positive(required(p, "writer_epoch")?, "SR writer_epoch")?;
    let at: String = budget.one(c, "SELECT at_ns FROM epochs WHERE epoch=?", [writer], |r| {
        r.get(0)
    })?;
    let floor = previous["history"]
        .as_array()
        .and_then(|h| h.last())
        .map(|last| nonnegative(required(last, "commit_ns")?, "previous commit_ns"))
        .transpose()?
        .unwrap_or(0)
        .max(nonnegative(&at, "epoch at_ns")?);
    if first < floor {
        return Err("StorageUnavailable：SR 评估早于 writer/原账前沿".into());
    }
    let root: String = budget.one(c, "SELECT root FROM commits WHERE seq=?", [seq - 1], |r| {
        r.get(0)
    })?;
    if evaluate(config, previous, &cut(config, seq - 1, &root), p)? != event["record"] {
        return Err("StorageUnavailable：SR 评估重放不等于原提交".into());
    }
    let mut next = previous.clone();
    next["history"]
        .as_array_mut()
        .ok_or("StorageUnavailable：历史形状损坏")?
        .push(event.clone());
    Ok(next)
}

fn committed_event(
    c: &Connection,
    r: &Value,
    budget: &mut ReadBudget<'_>,
) -> Result<(i64, String, Value), String> {
    let seq = positive(required(&r["cut"], "commit_seq")?, "SR receipt seq")?;
    let (root, raw): (String, String) = budget.one(
        c,
        "SELECT root,image FROM commits WHERE seq=?",
        [seq],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let event = strict_json(raw.as_bytes())?["history"]
        .as_array()
        .and_then(|a| a.last().cloned())
        .ok_or("StorageUnavailable：SR 回执指向的提交缺事件")?;
    if event["kind"] != OP {
        return Err("StorageUnavailable：SR 回执指向的提交不是评估事件".into());
    }
    Ok((seq, root, event))
}

pub(super) fn audit_receipt(
    c: &Connection,
    config: &Config,
    r: &Value,
    budget: &mut ReadBudget<'_>,
) -> Result<(), String> {
    let (seq, root, event) = committed_event(c, r, budget)?;
    if receipt(config, &event, cut(config, seq, &root)) != *r {
        return Err("StorageUnavailable：SR 评估回执不等于实际提交".into());
    }
    Ok(())
}

pub(super) fn audit_message(
    c: &Connection,
    config: &Config,
    request: &Value,
    r: &Value,
    budget: &mut ReadBudget<'_>,
) -> Result<(), String> {
    validate_request(&request["payload"])?;
    config.authorized(request)?;
    if r["kind"] != RECEIPT || r["evaluation_id"] != request["payload"]["evaluation_id"] {
        return Err("StorageUnavailable：SR 消息/回执身份不符".into());
    }
    let (_, _, event) = committed_event(c, r, budget)?;
    if event["request"]["payload"] != request["payload"] {
        return Err("IdentityConflict：SR evaluation_id 已绑定其他评估内容".into());
    }
    Ok(())
}

/// 公开查询：按 evaluation_id 返回原评估、原政策原件与永久回执。
pub(super) fn query(c: &Connection, config: &Config, p: &Value) -> Result<Value, String> {
    exact(p, &["op", "evaluation_id"], "SR 评估查询")?;
    if config.domain != "E" {
        return Err("SchemaUnsupported：只有 E 持有 SR 扩量评估".into());
    }
    let r = row(c, "receipts", &receipt_key(required(p, "evaluation_id")?))?
        .ok_or("MissingDependency：SR 评估不存在")?;
    let (_, _, event) = committed_event(c, &r, &mut ReadBudget::new(None, None))?;
    Ok(
        json!({"kind":"EconomicSrEvaluation","domain_id":config.domain,"evaluation":event["record"],"policy_raw_utf8":event["request"]["payload"]["policy_raw_utf8"],"receipt":r}),
    )
}
