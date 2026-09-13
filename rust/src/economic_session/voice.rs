//! #1374：不可变 Voice 绑定。定义 n 是父 Voice 的嵌套代际，不是 campaign 重开计数。
use super::book::{decode, hash, integer};
use super::evidence::VerifiedGammaEvidence;
use super::store::Config;
use crate::session_protocol::{nonnegative, positive, required};
use serde::Serialize;
use serde_json::{json, Value};

pub(super) fn exact(v: &Value, keys: &[&str]) -> Result<(), String> {
    let o = v.as_object().ok_or("SchemaUnsupported：Voice 字段须对象")?;
    if o.len() != keys.len() || o.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err("SchemaUnsupported：Voice 字段不完整或多余".into());
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(super) struct EvidenceRef {
    authority_id: String,
    object_id: String,
    revision: String,
    content_hash: String,
}
impl EvidenceRef {
    fn parse(v: &Value) -> Result<Self, String> {
        exact(
            v,
            &["authority_id", "object_id", "revision", "content_hash"],
        )?;
        let content_hash = required(v, "content_hash")?;
        digest(content_hash)?;
        nonnegative(required(v, "revision")?, "evidence revision")?;
        Ok(Self {
            authority_id: required(v, "authority_id")?.into(),
            object_id: required(v, "object_id")?.into(),
            revision: required(v, "revision")?.into(),
            content_hash: content_hash.into(),
        })
    }
}
pub(super) fn digest(s: &str) -> Result<(), String> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("InvalidDomain：Voice 摘要须小写 SHA256".into());
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(super) struct VoiceIdentity {
    carrier_ref: EvidenceRef,
    gamma_ref: EvidenceRef,
    sigma: String,
    n: String,
}
pub(super) fn validate_request(p: &Value) -> Result<(), String> {
    exact(
        p,
        &[
            "op",
            "binding_id",
            "evidence_id",
            "carrier_ref",
            "gamma_ref",
            "parent_voice_id",
            "sigma",
            "n",
            "opening_basis",
        ],
    )?;
    if p["op"] != "BindVoice" {
        return Err("SchemaUnsupported：非 Voice 绑定请求".into());
    }
    required(p, "binding_id")?;
    required(p, "evidence_id")?;
    EvidenceRef::parse(&p["carrier_ref"])?;
    EvidenceRef::parse(&p["gamma_ref"])?;
    if !p["parent_voice_id"].is_null() {
        digest(required(p, "parent_voice_id")?)?;
    }
    if p["sigma"] != "Long" && p["sigma"] != "Short" {
        return Err("InvalidDomain：Voice sigma 须 Long/Short".into());
    }
    nonnegative(required(p, "n")?, "Voice n")?;
    exact(
        &p["opening_basis"],
        &["fact_key", "expected_fact_hash", "allocation_id"],
    )?;
    digest(required(&p["opening_basis"], "fact_key")?)?;
    digest(required(&p["opening_basis"], "expected_fact_hash")?)?;
    required(&p["opening_basis"], "allocation_id")?;
    Ok(())
}
pub(super) fn receipt_key(binding_id: &str) -> String {
    // 类型命名空间，实际 allocation key 冲突仍明确拒绝，不能覆盖另一业务。
    format!("voice-binding/1:{}", hash(&json!(binding_id)))
}
pub(super) fn records(image: &Value) -> Result<Vec<Value>, String> {
    let v = &image["book"]["voice"];
    if image["book"].is_null() || v["status"] == "MissingDependency" {
        return Ok(Vec::new());
    }
    exact(v, &["status", "records"])?;
    if v["status"] != "Bound" {
        return Err("StorageUnavailable：Voice 状态未知".into());
    }
    v["records"]
        .as_array()
        .filter(|a| !a.is_empty())
        .cloned()
        .ok_or_else(|| "StorageUnavailable：Bound Voice 集合为空/形状错误".into())
}
pub(super) fn find_binding(image: &Value, binding: &str) -> Result<Option<Value>, String> {
    Ok(records(image)?
        .into_iter()
        .find(|v| v["binding_id"] == binding))
}

/// 仅供 live 校验及同一强度的持久重放调用；证据类型不能由客户端反序列化产生。
pub(super) fn bind_image(
    image: &Value,
    config: &Config,
    request: &Value,
    verified: &VerifiedGammaEvidence,
    received_ns: &str,
    first_known_ns: &str,
    commit_ns: &str,
    seq: i64,
) -> Result<(Value, Value), String> {
    validate_request(request)?;
    let book = &image["book"];
    if book.is_null() {
        return Err("MissingDependency：Voice 绑定缺已核持久重".into());
    }
    let basis = &request["opening_basis"];
    let fact = image["recorded"]
        .as_array()
        .ok_or("StorageUnavailable：recorded 形状")?
        .iter()
        .find(|r| r["fact_key"] == basis["fact_key"])
        .ok_or("MissingDependency：Voice 开局原事实尚未在本 B 应用")?;
    let application = image["applied"]
        .as_array()
        .ok_or("StorageUnavailable：applied 形状")?
        .iter()
        .find(|a| a["allocation_id"] == basis["allocation_id"])
        .ok_or("MissingDependency：Voice 开局 allocation 尚未应用")?;
    if application["status"] != "Applied"
        || application["fact_key"] != fact["fact_key"]
        || fact["fact_hash"] != basis["expected_fact_hash"]
        || fact["event"]["allocation_id"] != basis["allocation_id"]
        || fact["target_domain"] != config.domain
    {
        return Err("IdentityConflict：Voice 开局事实/应用/目标不一致".into());
    }
    let (doc, event) = decode(required(fact, "raw_utf8")?, required(fact, "json_pointer")?)?;
    let quantity = if event.get("observed_existing_book").is_some() {
        integer(&event["observed_existing_book"], "units")?
    } else {
        integer(&event["deltas"], "units")?
            .checked_abs()
            .ok_or("InvalidDomain：开局数量溢出")?
    };
    positive(&quantity.to_string(), "Voice opening quantity")?;
    let opening_basis = json!({"fact_key":fact["fact_key"],"fact_hash":fact["fact_hash"],"allocation_id":basis["allocation_id"],"e_cut":fact["cut"],"raw_sha256":fact["raw_sha256"],"json_pointer":fact["json_pointer"],"quantity":quantity.to_string(),"units":doc["units"]});
    let trusted = verified.binding();
    let mut opening_source = opening_basis.clone();
    opening_source.as_object_mut().unwrap().remove("e_cut");
    // γ 有效并不自动证明 instrument、carrier 或开局。它们须属于受核验的同一绑定原件。
    for k in ["carrier_ref", "gamma_ref", "sigma", "n", "parent_voice_id"] {
        if request[k] != trusted[k] {
            return Err(format!("IdentityConflict：Voice {k} 不等于固定权威依据"));
        }
    }
    if trusted["instrument"] != book["symbol"]
        || trusted["operation_level"] != book["operation_level"]
        || trusted["chong_id"] != book["chong_id"]
        || trusted["opening_source"] != opening_source
        || trusted["evidence_id"] != request["evidence_id"]
    {
        return Err("IdentityConflict：γ/标的/重操作视图/开局量来源未绑定".into());
    }
    exact(
        &trusted["structure_view"],
        &[
            "profile_id",
            "execution_level",
            "confirmation_level",
            "nest_depth",
        ],
    )?;
    required(&trusted["structure_view"], "profile_id")?;
    let level = |k| nonnegative(required(&trusted["structure_view"], k)?, k);
    if level("execution_level")?.checked_add(level("nest_depth")?)
        != Some(level("confirmation_level")?)
    {
        return Err("InvalidDomain：γ 区间套坐标不满足确认层=执行层+Ndepth".into());
    }
    let mut voices = records(image)?;
    let n = nonnegative(required(request, "n")?, "Voice n")?;
    if request["parent_voice_id"].is_null() {
        if n != 0 {
            return Err("InvalidDomain：根 Voice n 必须为零".into());
        }
    } else {
        let parent = voices
            .iter()
            .find(|v| v["voice_id"] == request["parent_voice_id"])
            .ok_or("MissingDependency：真实父 Voice 尚未绑定")?;
        if nonnegative(required(&parent["identity"], "n")?, "parent Voice n")?.checked_add(1)
            != Some(n)
            || parent["identity"]["sigma"] == request["sigma"]
            || positive(required(parent, "commit_seq")?, "parent commit_seq")? >= seq
            || nonnegative(required(parent, "commit_ns")?, "parent commit_ns")?
                > nonnegative(first_known_ns, "child first_known_ns")?
            || nonnegative(required(parent, "first_known_ns")?, "parent first_known_ns")?
                > nonnegative(first_known_ns, "child first_known_ns")?
        {
            return Err("InvalidDomain：子 Voice 必须父 n+1 且方向相反".into());
        }
    }
    let identity = VoiceIdentity {
        carrier_ref: EvidenceRef::parse(&request["carrier_ref"])?,
        gamma_ref: EvidenceRef::parse(&request["gamma_ref"])?,
        sigma: required(request, "sigma")?.into(),
        n: n.to_string(),
    };
    let identity =
        serde_json::to_value(identity).map_err(|e| format!("InvalidDomain：Voice 编码 {e}"))?;
    let id = hash(
        &json!({"schema":"voice-identity/1","domain_id":config.domain,"session_id":config.session,"session_generation":config.generation,"chong_id":book["chong_id"],"identity":identity}),
    );
    if voices
        .iter()
        .any(|v| v["voice_id"] == id || v["binding_id"] == request["binding_id"])
    {
        return Err("IdentityConflict：Voice 身份或 binding_id 已建立，不可更换依据".into());
    }
    let r = nonnegative(received_ns, "Voice received_ns")?;
    let f = nonnegative(first_known_ns, "Voice first_known_ns")?;
    let c = nonnegative(commit_ns, "Voice commit_ns")?;
    if let Some(last) = image["history"].as_array().and_then(|a| a.last()) {
        if nonnegative(required(last, "commit_ns")?, "previous commit_ns")? > f {
            return Err("InvalidDomain：Voice 首次知晓早于本域前沿".into());
        }
    }
    if f < verified.observation_ns() {
        return Err("InvalidDomain：Voice 首次知晓早于显式映射的 γ 确认观察".into());
    }
    if r > f
        || f > c
        || seq <= 0
        || nonnegative(required(application, "commit_ns")?, "basis commit_ns")? > f
    {
        return Err("InvalidDomain：Voice 获知时间早于前件/时钟倒退".into());
    }
    let record = json!({"voice_id":id,"binding_id":request["binding_id"],"identity":identity,"instrument":book["symbol"],"operation_level":book["operation_level"],"structure_view":trusted["structure_view"],"parent_voice_id":request["parent_voice_id"],"opening_basis":opening_basis,"evidence":verified.reference(),"first_known_ns":first_known_ns,"commit_ns":commit_ns,"commit_seq":seq.to_string(),"business_lifecycle":{"status":"NotRecorded"},"responsibility_tail":{"status":"Unknown","reason":"no authoritative responsibility-retirement event"}});
    voices.push(record.clone());
    let mut next = image.clone();
    next["book"]["voice"] = json!({"status":"Bound","records":voices});
    Ok((next, record))
}
pub(super) fn receipt(config: &Config, record: &Value, cut: Value) -> Value {
    json!({"kind":"VoiceBound","domain_id":config.domain,"binding_id":record["binding_id"],"record":record,"first_known_ns":record["first_known_ns"],"commit_ns":record["commit_ns"],"cut":cut})
}

pub(super) fn history_event(
    request: &Value,
    record: &Value,
    received: &str,
    witness: Value,
) -> Value {
    json!({"kind":"BindVoice","record":record,"received_ns":received,"origin_message":{"source_namespace":request["source_namespace"],"source_epoch":request["source_epoch"],"message_id":request["message_id"],"payload_hash":request["payload_hash"]},"origin_request_hash":hash(request),"evidence_witness":witness,"commit_seq":record["commit_seq"],"commit_ns":record["commit_ns"]})
}
pub(super) fn original_request(
    c: &rusqlite::Connection,
    event: &Value,
    config: &Config,
) -> Result<Value, String> {
    use crate::session_protocol::{strict_json, validate_envelope};
    let origin = &event["origin_message"];
    exact(
        origin,
        &[
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
        ],
    )?;
    let key = hash(&json!([
        required(origin, "source_namespace")?,
        required(origin, "source_epoch")?,
        required(origin, "message_id")?
    ]));
    let (raw, h, rk): (String, String, String) = c
        .query_row(
            "SELECT request_json,payload_hash,receipt_key FROM messages WHERE k=?",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "StorageUnavailable：Voice 原消息绑定缺失".to_owned())?;
    let request = validate_envelope(strict_json(raw.as_bytes())?, super::store::SCHEMA)?;
    config.authorized(&request)?;
    let actual = json!({"source_namespace":request["source_namespace"],"source_epoch":request["source_epoch"],"message_id":request["message_id"],"payload_hash":request["payload_hash"]});
    if origin != &actual
        || request["payload_hash"] != h
        || event["origin_request_hash"] != hash(&request)
        || rk != receipt_key(required(&event["record"], "binding_id")?)
    {
        return Err("StorageUnavailable：Voice 原消息/回执绑定改变".into());
    }
    validate_request(&request["payload"])?;
    Ok(request)
}
pub(super) fn event_for<'a>(image: &'a Value, binding: &str) -> Result<&'a Value, String> {
    image["history"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|e| e["kind"] == "BindVoice" && e["record"]["binding_id"] == binding)
        })
        .ok_or_else(|| "StorageUnavailable：Voice 原绑定事件缺失".into())
}
pub(super) fn replay(
    c: &rusqlite::Connection,
    config: &Config,
    previous: &Value,
    event: &Value,
    seq: i64,
    ns: &str,
) -> Result<Value, String> {
    exact(
        event,
        &[
            "kind",
            "record",
            "received_ns",
            "origin_message",
            "origin_request_hash",
            "evidence_witness",
            "commit_seq",
            "commit_ns",
        ],
    )?;
    if !super::evidence::enabled(config)
        || config.domain == "E"
        || event["kind"] != "BindVoice"
        || event["commit_seq"] != seq.to_string()
        || event["commit_ns"] != ns
    {
        return Err("StorageUnavailable：Voice 事件类型/版本/提交身份不符".into());
    }
    let request = original_request(c, event, config)?;
    let verified = super::evidence::verify_witness(config, &event["evidence_witness"])?;
    let received = required(event, "received_ns")?;
    let first = required(&event["record"], "first_known_ns")?;
    let (mut next, record) = bind_image(
        previous,
        config,
        &request["payload"],
        &verified,
        received,
        first,
        ns,
        seq,
    )?;
    let expected = history_event(&request, &record, received, verified.witness());
    if event != &expected {
        return Err("StorageUnavailable：Voice 原件重放与绑定事件不一致".into());
    }
    next["history"]
        .as_array_mut()
        .ok_or("StorageUnavailable：history 形状")?
        .push(expected);
    Ok(next)
}
