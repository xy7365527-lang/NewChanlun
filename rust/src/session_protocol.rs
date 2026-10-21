//! #1374：S、E、B 共用的规范字节与共同消息机械校验（SPEC #1340）。
//!
//! 保留 S 的规范序列化和错误语义；schema 由域入口显式给定。
//! 这里不授权 producer、不核账本/结构资格；目标、会话及写者权限由各域独立验证。
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt;

pub fn sha256_hex(bytes: &[u8]) -> String {
    #[cfg(feature = "s_capture_native_testonly")]
    note_hash(bytes.len());
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// 规范十进制整数：`0` 或 `-?[1-9][0-9]*`（无 `+`、无空白、无前导零、`-0` 非规范）。
pub fn is_canonical_integer(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    let (neg, rest) = if b[0] == b'-' {
        (true, &b[1..])
    } else {
        (false, b)
    };
    if rest.is_empty() {
        return false;
    }
    if rest[0] == b'0' {
        return rest.len() == 1 && !neg;
    }
    rest.iter().all(|c| c.is_ascii_digit())
}

/// 把规范十进制字符串解析为精确 i64（非规范或超范围报错，不静默截断）。
pub fn parse_canonical_i64(s: &str, what: &str) -> Result<i64, String> {
    parse_canonical_i64_typed(s, what).map_err(|e| e.to_string())
}
#[derive(Debug)]
pub enum EnvelopeFailure {
    Object,
    Fields,
    CausalArray,
    PayloadObject,
    HashConflict,
    HashUnavailable,
}
impl fmt::Display for EnvelopeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Object => "SchemaUnsupported：消息必须对象",
            Self::Fields => "SchemaUnsupported：公共头字段不完整/多余",
            Self::CausalArray => "SchemaUnsupported：causal_refs必须数组",
            Self::PayloadObject => "SchemaUnsupported：payload必须对象",
            Self::HashConflict => "IdentityConflict：payload_hash与规范业务内容不同",
            Self::HashUnavailable => "prototype_audit_allocation_incomplete：owned规范编码未启用",
        })
    }
}
#[derive(Debug)]
pub enum ProtocolError<'a> {
    Fixed(EnvelopeFailure),
    Required(&'a str),
    Schema(&'a str),
    Integer {
        input: &'a str,
        field: &'a str,
        overflow: Option<std::num::ParseIntError>,
    },
    Nonnegative(&'a str),
    Positive(&'a str),
    Allocation(crate::s_content::CodecError),
    Encoding(serde_json::Error),
}
impl fmt::Display for ProtocolError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(s) => fmt::Display::fmt(s, f),
            Self::Required(k) => write!(f, "SchemaUnsupported：缺非空字符串 {k}"),
            Self::Schema(s) => write!(f, "SchemaUnsupported：仅{s}"),
            Self::Integer {
                input,
                field,
                overflow: None,
            } => write!(
                f,
                "{field} `{input}` 不是规范十进制整数（要求唯一表示：无 +、无空白、无前导零）"
            ),
            Self::Integer {
                input,
                field,
                overflow: Some(e),
            } => write!(f, "{field} `{input}` 超出 i64 精确整数域：{e}"),
            Self::Nonnegative(k) => write!(f, "InvalidDomain：{k} 必须非负"),
            Self::Positive(k) => write!(f, "InvalidDomain：{k} 必须大于0"),
            Self::Allocation(e) => fmt::Display::fmt(e, f),
            Self::Encoding(e) => write!(f, "SchemaUnsupported：{e}"),
        }
    }
}
pub fn parse_canonical_i64_typed<'a>(s: &'a str, what: &'a str) -> Result<i64, ProtocolError<'a>> {
    if !is_canonical_integer(s) {
        return Err(ProtocolError::Integer {
            input: s,
            field: what,
            overflow: None,
        });
    }
    s.parse::<i64>().map_err(|e| ProtocolError::Integer {
        input: s,
        field: what,
        overflow: Some(e),
    })
}
pub fn nonnegative_typed<'a>(s: &'a str, field: &'a str) -> Result<i64, ProtocolError<'a>> {
    let n = parse_canonical_i64_typed(s, field)?;
    if n < 0 {
        return Err(ProtocolError::Nonnegative(field));
    }
    Ok(n)
}
pub fn positive_typed<'a>(s: &'a str, field: &'a str) -> Result<i64, ProtocolError<'a>> {
    let n = nonnegative_typed(s, field)?;
    if n == 0 {
        return Err(ProtocolError::Positive(field));
    }
    Ok(n)
}

pub fn canonical_json(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical_json(&map[*k])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
    }
}

pub fn required<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    required_view(v.into(), k)
}
pub fn nonnegative(s: &str, field: &str) -> Result<i64, String> {
    nonnegative_typed(s, field).map_err(|e| e.to_string())
}
pub fn positive(s: &str, field: &str) -> Result<i64, String> {
    positive_typed(s, field).map_err(|e| e.to_string())
}

/// serde_json::Value 的普通解析会覆盖重复键；所有新协议/clock输入先经过此严格解码器。
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Strict>()? {
                    v.push(x.0)
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if v.contains_key(&k) {
                        return Err(serde::de::Error::custom(format!("duplicate key {k}")));
                    }
                    v.insert(k, a.next_value::<Strict>()?.0);
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_json(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice::<Strict>(bytes)
        .map(|s| s.0)
        .map_err(|e| format!("SchemaUnsupported：{e}"))
}

/// 校共同头、精确 epoch、因果引用形状和规范业务哈希；不替代域的权威身份核验。
pub fn validate_envelope(value: Value, schema_revision: &str) -> Result<Value, String> {
    validate_envelope_view((&value).into(), schema_revision)?;
    Ok(value)
}

use crate::s_content::capture_allocation::{audit_json::JsonView, OwnedBudget};
/// 同一字段/epoch/因果引用/哈希判据；返回本输入借用，不创建ordinary业务树。
pub fn validate_envelope_view<'a>(
    value: JsonView<'a>,
    schema_revision: &str,
) -> Result<JsonView<'a>, String> {
    validate_envelope_view_with_owner(value, schema_revision, None)
}
pub fn required_view<'a>(value: JsonView<'a>, key: &str) -> Result<&'a str, String> {
    required_view_typed(value, key).map_err(|e| e.to_string())
}
pub fn required_view_typed<'v, 'k>(
    value: JsonView<'v>,
    key: &'k str,
) -> Result<&'v str, ProtocolError<'k>> {
    value
        .get(key)
        .and_then(JsonView::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(ProtocolError::Required(key))
}
fn child<'a>(value: JsonView<'a>, key: &str) -> JsonView<'a> {
    value.get(key).unwrap_or(JsonView::Null)
}
/// owner只由实际调用上下文传入；它不作为schema/来源/哈希成功token。
pub fn validate_envelope_view_with_owner<'a>(
    value: JsonView<'a>,
    schema_revision: &str,
    owner: Option<&OwnedBudget>,
) -> Result<JsonView<'a>, String> {
    validate_envelope_typed(value, schema_revision, owner)
        .map(|_| value)
        .map_err(|e| e.to_string())
}
pub fn validate_envelope_typed<'a>(
    value: JsonView<'a>,
    schema_revision: &'a str,
    owner: Option<&OwnedBudget>,
) -> Result<JsonView<'a>, ProtocolError<'a>> {
    let allowed = [
        "schema_revision",
        "session_id",
        "session_generation",
        "source_namespace",
        "source_epoch",
        "message_id",
        "producer_id",
        "producer_epoch",
        "payload_hash",
        "causal_refs",
        "payload",
    ];
    let map = value
        .as_object()
        .ok_or(ProtocolError::Fixed(EnvelopeFailure::Object))?;
    if map.len() != allowed.len() || map.keys().any(|k| !allowed.contains(&k)) {
        return Err(ProtocolError::Fixed(EnvelopeFailure::Fields));
    }
    if required_view_typed(value, "schema_revision")? != schema_revision {
        return Err(ProtocolError::Schema(schema_revision));
    }
    for k in [
        "session_id",
        "source_namespace",
        "source_epoch",
        "message_id",
        "producer_id",
    ] {
        required_view_typed(value, k)?;
    }
    positive_typed(
        required_view_typed(value, "session_generation")?,
        "session_generation",
    )?;
    positive_typed(
        required_view_typed(value, "producer_epoch")?,
        "producer_epoch",
    )?;
    let refs = child(value, "causal_refs")
        .as_array()
        .ok_or(ProtocolError::Fixed(EnvelopeFailure::CausalArray))?;
    for r in refs.iter() {
        for k in [
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
        ] {
            required_view_typed(r, k)?;
        }
    }
    let payload = child(value, "payload");
    if !payload.is_object() {
        return Err(ProtocolError::Fixed(EnvelopeFailure::PayloadObject));
    }
    // 与原入口相同：先拒payload_hash缺失，再执行规范内容哈希，最后核op。
    let declared = required_view_typed(value, "payload_hash")?;
    if declared != canonical_hash_view_typed(payload, owner)?.as_str() {
        return Err(ProtocolError::Fixed(EnvelopeFailure::HashConflict));
    }
    required_view_typed(payload, "op")?;
    Ok(value)
}
/// 固定SHA256文本，无String或新账本；normal/Owned都由JsonView的canonical序列化规则供字节。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedHash([u8; 64]);
impl FixedHash {
    fn from_digest(bytes: &[u8]) -> Self {
        let mut hash = [0; 64];
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for (i, b) in bytes.iter().enumerate() {
            hash[2 * i] = HEX[(b >> 4) as usize];
            hash[2 * i + 1] = HEX[(b & 15) as usize];
        }
        Self(hash)
    }
    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}
pub fn canonical_hash_view_with_owner(
    value: JsonView<'_>,
    owner: Option<&OwnedBudget>,
) -> Result<FixedHash, String> {
    canonical_hash_view_typed(value, owner).map_err(|e| e.to_string())
}
pub fn canonical_hash_view_typed(
    value: JsonView<'_>,
    owner: Option<&OwnedBudget>,
) -> Result<FixedHash, ProtocolError<'static>> {
    #[cfg(feature = "s_capture_native_testonly")]
    let mut hashed_bytes = 0usize;
    let mut hash = Sha256::new();
    if let Some(owner) = owner {
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            let bytes =
                crate::s_content::capture_allocation::audit_json::canonical_bytes(value, owner)
                    .map_err(ProtocolError::Allocation)?;
            hash.update(&*bytes);
            #[cfg(feature = "s_capture_native_testonly")]
            {
                hashed_bytes = bytes.len();
            }
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = owner;
            return Err(ProtocolError::Fixed(EnvelopeFailure::HashUnavailable));
        }
    } else {
        struct HashOut<'a>(
            &'a mut Sha256,
            #[cfg(feature = "s_capture_native_testonly")] &'a mut usize,
        );
        impl std::io::Write for HashOut<'_> {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.update(bytes);
                #[cfg(feature = "s_capture_native_testonly")]
                {
                    *self.1 += bytes.len();
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer(
            HashOut(
                &mut hash,
                #[cfg(feature = "s_capture_native_testonly")]
                &mut hashed_bytes,
            ),
            &value,
        )
        .map_err(ProtocolError::Encoding)?;
    }
    #[cfg(feature = "s_capture_native_testonly")]
    note_hash(hashed_bytes);
    Ok(FixedHash::from_digest(&hash.finalize()))
}

/// ClockPlan的借用规则核；bin保留原DTO或持自己的JsonOwner，不把库依赖反向指向bin。
#[derive(Clone, Copy)]
pub struct ClockView<'a> {
    value: JsonView<'a>,
}
impl<'a> ClockView<'a> {
    /// 本口接已经解码的值，不负责重复键策略；文件/SQL入口继续独立使用原strict解码器。
    pub fn parse(value: JsonView<'a>) -> Result<Self, String> {
        let keys = [
            "schema_revision",
            "clock_plan_id",
            "origin_utc",
            "unit",
            "events",
        ];
        if value.as_object().map(|m| m.len()) != Some(keys.len())
            || value
                .as_object()
                .unwrap()
                .keys()
                .any(|k| !keys.contains(&k))
        {
            return Err("SchemaUnsupported：clock计划字段不完整/多余".into());
        }
        if required_view(value, "schema_revision")? != "s-clock-plan/1"
            || required_view(value, "unit")? != "ns"
        {
            return Err("SchemaUnsupported：需要 s-clock-plan/1 与 ns 单位".into());
        }
        required_view(value, "clock_plan_id")?;
        required_view(value, "origin_utc")?;
        let events = child(value, "events")
            .as_object()
            .ok_or("SchemaUnsupported：clock events必须对象")?;
        for (id, e) in events.iter() {
            if id.is_empty() {
                return Err("InvalidDomain：空clock_event_id".into());
            }
            if let Some(t) = e.get("recover_ns") {
                nonnegative(
                    t.as_str().ok_or("InvalidDomain：recover_ns非文本")?,
                    "recover_ns",
                )?;
                if e.as_object().map(|m| m.len()) != Some(1) {
                    return Err("InvalidDomain：recover事件字段不唯一".into());
                }
            } else {
                let accept = nonnegative(required_view(e, "accept_ns")?, "accept_ns")?;
                let attempts = child(e, "attempts")
                    .as_array()
                    .filter(|a| !a.is_empty())
                    .ok_or("MissingDependency：clock缺非空attempts")?;
                if e.as_object().map(|m| m.len()) != Some(2) {
                    return Err("InvalidDomain：输入clock事件字段不完整/多余".into());
                }
                let mut prev = accept;
                for phase in attempts.iter() {
                    let b = nonnegative(required_view(phase, "begin_ns")?, "begin_ns")?;
                    let c = nonnegative(required_view(phase, "commit_ns")?, "commit_ns")?;
                    if b < prev || c < b || phase.as_object().map(|m| m.len()) != Some(2) {
                        return Err("InvalidDomain：clock attempt时间倒退或字段错误".into());
                    }
                    prev = c;
                }
            }
        }
        Ok(Self { value })
    }
    pub fn view(self) -> JsonView<'a> {
        self.value
    }
    pub fn hash_fixed(self) -> Result<FixedHash, String> {
        self.hash_fixed_with_owner(None)
    }
    pub fn hash_fixed_with_owner(self, owner: Option<&OwnedBudget>) -> Result<FixedHash, String> {
        canonical_hash_view_with_owner(self.value, owner)
    }
    pub fn phase(self, id: &str, phase: &str, attempt: i64) -> Result<&'a str, String> {
        clock_phase_view(self.value, id, phase, attempt)
    }
}
/// 原phase查找顺序及任意“phase_ns”键语义；字段名只借用真实key，不format分配临时String。
pub fn clock_phase_view<'a>(
    value: JsonView<'a>,
    id: &str,
    phase: &str,
    attempt: i64,
) -> Result<&'a str, String> {
    let event = child(value, "events")
        .get(id)
        .ok_or_else(|| format!("MissingDependency：clock事件 {id}"))?;
    let record = if attempt < 0 {
        Some(event)
    } else {
        child(event, "attempts")
            .as_array()
            .and_then(|a| a.get(attempt as usize))
    };
    let found = record
        .and_then(JsonView::as_object)
        .and_then(|o| o.iter().find(|(k, _)| k.strip_suffix("_ns") == Some(phase)));
    let (key, value) =
        found.ok_or_else(|| format!("MissingDependency：clock {id}/{phase}/{attempt}"))?;
    let text = value
        .as_str()
        .ok_or_else(|| format!("MissingDependency：clock {id}/{phase}/{attempt}"))?;
    nonnegative(text, key)?;
    Ok(text)
}

/// 共同回复的规范哈希、原请求因果引用和回复身份；域身份均由已核启动配置传入。
#[allow(clippy::too_many_arguments)]
pub fn reply_envelope(
    e: &Value,
    payload: Value,
    schema_revision: &str,
    session_id: &str,
    session_generation: &str,
    reply_namespace: &str,
    producer_id: &str,
    producer_epoch: &str,
) -> Value {
    let hash = sha256_hex(canonical_json(&payload).as_bytes());
    let id = format!(
        "reply-{}",
        sha256_hex(
            canonical_json(&json!([
                e["source_namespace"],
                e["source_epoch"],
                e["message_id"],
                hash
            ]))
            .as_bytes()
        )
    );
    let reply = json!({"schema_revision":schema_revision,"session_id":session_id,"session_generation":session_generation,"source_namespace":reply_namespace,"source_epoch":producer_epoch,"message_id":id,"producer_id":producer_id,"producer_epoch":producer_epoch,"payload_hash":hash,"causal_refs":[{"source_namespace":e["source_namespace"],"source_epoch":e["source_epoch"],"message_id":e["message_id"],"payload_hash":e["payload_hash"]}],"payload":payload});
    reply
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_integer_contract_does_not_round_or_normalize_input() {
        for value in ["", "-0", "+1", "01", "-01", " 1", "1.0", "1e3"] {
            assert!(parse_canonical_i64(value, "quantity").is_err(), "{value}");
        }
        for value in ["9223372036854775808", "-9223372036854775809"] {
            assert!(parse_canonical_i64(value, "quantity")
                .unwrap_err()
                .contains("超出 i64"));
        }
        assert_eq!(
            parse_canonical_i64("9007199254740993", "quantity").unwrap(),
            9007199254740993
        );
        assert_eq!(
            parse_canonical_i64("-9223372036854775808", "quantity").unwrap(),
            i64::MIN
        );
        assert_eq!(
            parse_canonical_i64("9223372036854775807", "quantity").unwrap(),
            i64::MAX
        );
        assert_eq!(
            canonical_json(&json!({"quantity":"9007199254740993","unit":"TEST-UNIT"})),
            r#"{"quantity":"9007199254740993","unit":"TEST-UNIT"}"#
        );
    }

    #[test]
    fn strict_decode_rejects_duplicates_at_all_depths_and_trailing_data() {
        for raw in [
            r#"{"a":1,"a":2}"#,
            r#"{"payload":{"amount":"1","amount":"2"}}"#,
            r#"{"refs":[{"id":"a","id":"b"}]}"#,
            r#"{"valid":true} {}"#,
        ] {
            assert!(strict_json(raw.as_bytes()).is_err(), "{raw}");
        }
        assert_eq!(
            strict_json(br#"{"a":[true,null,{"n":"0"}]}"#).unwrap(),
            json!({"a":[true,null,{"n":"0"}]})
        );
    }

    #[test]
    fn schema_parameter_preserves_common_validation_for_each_domain() {
        for schema in ["s-session/2", "economic-session/1"] {
            let payload = json!({"op":"QueryFact","quantity":"9007199254740993"});
            let request = json!({"schema_revision":schema,"session_id":"test-c","session_generation":"1","source_namespace":"test.feed","source_epoch":"1","message_id":"m-1","producer_id":"test-driver","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
            assert_eq!(validate_envelope(request.clone(), schema).unwrap(), request);
            let mut changed = request.clone();
            changed["payload"]["quantity"] = json!("9007199254740992");
            assert_eq!(
                validate_envelope(changed, schema).unwrap_err(),
                "IdentityConflict：payload_hash与规范业务内容不同"
            );
            let mut changed = request.clone();
            changed["session_generation"] = json!("01");
            assert!(validate_envelope(changed, schema).is_err());
            let mut changed = request.clone();
            changed["extra"] = Value::Null;
            assert_eq!(
                validate_envelope(changed, schema).unwrap_err(),
                "SchemaUnsupported：公共头字段不完整/多余"
            );
            assert_eq!(
                validate_envelope(request, "unsupported/1").unwrap_err(),
                "SchemaUnsupported：仅unsupported/1"
            );
        }
    }

    #[test]
    fn reply_bytes_match_independent_python_wire_vector() {
        let request = json!({"source_namespace":"test.feed","source_epoch":"1","message_id":"m-1","payload_hash":"source-hash"});
        let payload = json!({"ok":true,"kind":"Ready","nested":{"unicode":"重/γ","exact":"9007199254740993"}});
        let reply = reply_envelope(
            &request,
            payload,
            "s-session/2",
            "test-c",
            "1",
            "s-session/replies",
            "S:test-c",
            "3",
        );
        // Python 独立构造规范字节与两个 SHA-256；不是由 Rust 受测函数生成期望值。
        let expected = r#"{"causal_refs":[{"message_id":"m-1","payload_hash":"source-hash","source_epoch":"1","source_namespace":"test.feed"}],"message_id":"reply-10a72419b7415c6727126b0ed6cef4ee658b28e620a5f909f280ab461f82ff9c","payload":{"kind":"Ready","nested":{"exact":"9007199254740993","unicode":"重/γ"},"ok":true},"payload_hash":"e5293b9e9666346d23656a958821e6e33cf4fff30f1fd0513e54835a3728583b","producer_epoch":"3","producer_id":"S:test-c","schema_revision":"s-session/2","session_generation":"1","session_id":"test-c","source_epoch":"3","source_namespace":"s-session/replies"}"#; // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
        assert_eq!(canonical_json(&reply), expected);
    }
}

/// #1394 非 default 实验入口：全部 JSON 结果与解码工作集继续持有调用者的 Layout 账本。
/// ordinary strict_json 的公开接口不变；静态实验错误不会通过 serde 的 boxed Error 分配。
#[cfg(feature = "s_allocation_json_testonly")]
pub fn strict_json_owned(
    bytes: &[u8],
    budget: &crate::s_content::capture_allocation::OwnedBudget,
) -> Result<crate::s_content::capture_allocation::audit_json::OwnedJson, crate::s_content::CodecError>
{
    crate::s_content::capture_allocation::audit_json::OwnedJson::parse(bytes, budget)
}

/// #1394 非 default 实验入口：借用 ordinary/owned 统一视图，编码缓冲及遍历栈同账。
#[cfg(feature = "s_allocation_json_testonly")]
pub fn canonical_json_owned(
    value: crate::s_content::capture_allocation::audit_json::JsonView<'_>,
    budget: &crate::s_content::capture_allocation::OwnedBudget,
) -> Result<crate::s_content::CodecVec<u8>, crate::s_content::CodecError> {
    crate::s_content::capture_allocation::audit_json::canonical_bytes(value, budget)
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod borrowed_control_tests {
    use super::*;
    use crate::s_content::capture_allocation::audit_json::{tests::observed, OwnedJson};
    fn request() -> Value {
        let payload = json!({"op":"Commit","unicode":"汉\n😀","number":1.25e-9});
        json!({"schema_revision":"s-session/3","session_id":"same","session_generation":"1","source_namespace":"feed","source_epoch":"old","message_id":"m1","producer_id":"p1","producer_epoch":"2","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[{"source_namespace":"a","source_epoch":"1","message_id":"m0","payload_hash":"h0","extra":"allowed"}],"payload":payload})
    }
    fn clock() -> Value {
        json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"c1","origin_utc":"not-an-ISO-parser-contract","unit":"ns","events":{"input":{"accept_ns":"0","attempts":[{"begin_ns":"0","commit_ns":"9007199254740993"},{"begin_ns":"9007199254740993","commit_ns":"9223372036854775807"}]},"recover":{"recover_ns":"3"}}})
    }
    #[test]
    fn envelope_borrowed_plain_owned_same_rules_errors_and_canonical_hash() {
        for case in 0..8 {
            let mut value = request();
            match case {
                1 => value["extra"] = Value::Null,
                2 => value["schema_revision"] = json!("wrong"),
                3 => value["producer_epoch"] = json!("01"),
                4 => value["causal_refs"][0]["message_id"] = json!(""),
                5 => value["payload"] = Value::Null,
                6 => value["payload_hash"] = json!("bad"),
                7 => {
                    value["payload"].as_object_mut().unwrap().remove("op");
                    value["payload_hash"] =
                        json!(sha256_hex(canonical_json(&value["payload"]).as_bytes()));
                }
                _ => {}
            }
            let normal = validate_envelope(value.clone(), "s-session/3").map(|_| ());
            let budget = OwnedBudget::new(100000);
            let tree = strict_json_owned(canonical_json(&value).as_bytes(), &budget).unwrap();
            let owned =
                validate_envelope_view_with_owner(tree.view(), "s-session/3", Some(&budget))
                    .map(|_| ());
            assert_eq!(owned, normal, "case {case}");
            drop(tree);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        let value = request();
        let budget = OwnedBudget::new(100000);
        let tree = OwnedJson::parse(canonical_json(&value).as_bytes(), &budget).unwrap();
        let before = budget.snapshot().allocation_attempts;
        let (result, calls) = observed(|| {
            validate_envelope_view_with_owner(tree.view(), "s-session/3", Some(&budget))
        });
        result.unwrap();
        assert_eq!(calls, budget.snapshot().allocation_attempts - before);
        let (_, calls) =
            observed(|| validate_envelope_view((&value).into(), "s-session/3").unwrap());
        assert_eq!(calls, 0);
        drop(tree);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn clock_borrowed_validation_hash_phase_and_unknown_key_semantics_match() {
        let value = clock();
        let budget = OwnedBudget::new(100000);
        let tree = OwnedJson::parse(canonical_json(&value).as_bytes(), &budget).unwrap();
        let plain = ClockView::parse((&value).into()).unwrap();
        let owned = ClockView::parse(tree.view()).unwrap();
        assert_eq!(
            plain.hash_fixed().unwrap().as_str(),
            sha256_hex(canonical_json(&value).as_bytes())
        );
        assert_eq!(
            owned.hash_fixed_with_owner(Some(&budget)).unwrap(),
            plain.hash_fixed().unwrap()
        );
        for (id, phase, attempt) in [
            ("input", "accept", -1),
            ("input", "begin", 0),
            ("input", "commit", 1),
            ("recover", "recover", -7),
            ("input", "missing", 0),
            ("input", "begin", 2),
            ("missing", "begin", 0),
        ] {
            assert_eq!(
                plain.phase(id, phase, attempt),
                owned.phase(id, phase, attempt)
            );
        }
        assert_eq!(
            owned.phase("input", "commit", 1).unwrap(),
            "9223372036854775807"
        );
        let raw = json!({"events":{"x":{"custom_ns":"2","attempts":[{"odd phase_ns":"4"}]}}});
        assert_eq!(
            clock_phase_view((&raw).into(), "x", "custom", -1).unwrap(),
            "2"
        );
        assert_eq!(
            clock_phase_view((&raw).into(), "x", "odd phase", 0).unwrap(),
            "4"
        );
        let ((), calls) = observed(|| {
            let c = ClockView::parse((&value).into()).unwrap();
            c.phase("input", "commit", 0).unwrap();
            c.hash_fixed().unwrap();
        });
        assert_eq!(calls, 0);
        drop(tree);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn clock_error_priority_strict_inputs_and_owned_encoder_quota_stay_closed() {
        for case in 0..8 {
            let mut v = clock();
            match case {
                0 => v["extra"] = Value::Null,
                1 => v["unit"] = json!("bad"),
                2 => v["events"] = Value::Null,
                3 => v["events"]["recover"]["recover_ns"] = json!("-1"),
                4 => v["events"]["recover"]["extra"] = Value::Null,
                5 => v["events"]["input"]["attempts"] = json!([]),
                6 => v["events"]["input"]["attempts"][1]["begin_ns"] = json!("1"),
                _ => v["events"]["input"]["attempts"][0]["begin_ns"] = json!("01"),
            };
            let expected = ClockView::parse((&v).into()).err().unwrap();
            let b = OwnedBudget::new(100000);
            let tree = OwnedJson::parse(canonical_json(&v).as_bytes(), &b).unwrap();
            assert_eq!(ClockView::parse(tree.view()).err().unwrap(), expected);
            drop(tree);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
        let raw = br#"{"schema_revision":"s-clock-plan/1","events":{},"events":{}}"#;
        assert!(strict_json(raw).is_err());
        let b = OwnedBudget::new(100000);
        assert!(strict_json_owned(raw, &b).is_err());
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        let value = clock();
        let room = OwnedBudget::new(100000);
        let tree = OwnedJson::parse(canonical_json(&value).as_bytes(), &room).unwrap();
        let limit = OwnedBudget::new(0);
        let before = room.snapshot().live_requested_layout_bytes;
        let error = ClockView::parse(tree.view())
            .unwrap()
            .hash_fixed_with_owner(Some(&limit))
            .unwrap_err();
        assert!(error.contains(crate::s_content::capture_allocation::LIMIT));
        assert_eq!(limit.snapshot().allocated_layouts, 0);
        assert_eq!(room.snapshot().live_requested_layout_bytes, before);
        drop(tree);
        assert_eq!(room.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(feature = "s_capture_native_testonly")]
thread_local! { static HASH_WORK:std::cell::Cell<(u64,u64)> = const { std::cell::Cell::new((0,0)) }; }
#[cfg(feature = "s_capture_native_testonly")]
fn note_hash(bytes: usize) {
    HASH_WORK.with(|c| {
        let (n, b) = c.get();
        c.set((n.saturating_add(1), b.saturating_add(bytes as u64)));
    });
}
/// Native 正常诊断：实际执行的协议/clock/raw 入参 SHA256 次数及输入字节。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
pub fn hash_work_on_thread() -> (u64, u64) {
    HASH_WORK.with(std::cell::Cell::get)
}
