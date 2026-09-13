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
    if !is_canonical_integer(s) {
        return Err(format!(
            "{what} `{s}` 不是规范十进制整数（要求唯一表示：无 +、无空白、无前导零）"
        ));
    }
    s.parse::<i64>()
        .map_err(|e| format!("{what} `{s}` 超出 i64 精确整数域：{e}"))
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
    v.get(k)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("SchemaUnsupported：缺非空字符串 {k}"))
}
pub fn nonnegative(s: &str, field: &str) -> Result<i64, String> {
    let n = parse_canonical_i64(s, field)?;
    if n < 0 {
        return Err(format!("InvalidDomain：{field} 必须非负"));
    }
    Ok(n)
}
pub fn positive(s: &str, field: &str) -> Result<i64, String> {
    let n = nonnegative(s, field)?;
    if n == 0 {
        return Err(format!("InvalidDomain：{field} 必须大于0"));
    }
    Ok(n)
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
    let map = value.as_object().ok_or("SchemaUnsupported：消息必须对象")?;
    if map.len() != allowed.len() || map.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("SchemaUnsupported：公共头字段不完整/多余".into());
    }
    if required(&value, "schema_revision")? != schema_revision {
        return Err(format!("SchemaUnsupported：仅{schema_revision}"));
    }
    for k in [
        "session_id",
        "source_namespace",
        "source_epoch",
        "message_id",
        "producer_id",
    ] {
        required(&value, k)?;
    }
    positive(
        required(&value, "session_generation")?,
        "session_generation",
    )?;
    positive(required(&value, "producer_epoch")?, "producer_epoch")?;
    let refs = value["causal_refs"]
        .as_array()
        .ok_or("SchemaUnsupported：causal_refs必须数组")?;
    for r in refs {
        for k in [
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
        ] {
            required(r, k)?;
        }
    }
    if !value["payload"].is_object() {
        return Err("SchemaUnsupported：payload必须对象".into());
    }
    if required(&value, "payload_hash")? != sha256_hex(canonical_json(&value["payload"]).as_bytes())
    {
        return Err("IdentityConflict：payload_hash与规范业务内容不同".into());
    }
    required(&value["payload"], "op")?;
    Ok(value)
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
        let expected = r#"{"causal_refs":[{"message_id":"m-1","payload_hash":"source-hash","source_epoch":"1","source_namespace":"test.feed"}],"message_id":"reply-10a72419b7415c6727126b0ed6cef4ee658b28e620a5f909f280ab461f82ff9c","payload":{"kind":"Ready","nested":{"exact":"9007199254740993","unicode":"重/γ"},"ok":true},"payload_hash":"e5293b9e9666346d23656a958821e6e33cf4fff30f1fd0513e54835a3728583b","producer_epoch":"3","producer_id":"S:test-c","schema_revision":"s-session/2","session_generation":"1","session_id":"test-c","source_epoch":"3","source_namespace":"s-session/replies"}"#;
        assert_eq!(canonical_json(&reply), expected);
    }
}
