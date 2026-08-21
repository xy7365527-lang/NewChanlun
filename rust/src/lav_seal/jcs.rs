//! RFC 8785（JSON Canonicalization Scheme，JCS）规范 JSON 序列化。
//!
//! ADR 0025 的 bundle／Comparison Set Manifest 身份都是「JCS 规范化 UTF-8 bytes 求
//! SHA-256」得到的摘要，因此本模块是封存器身份机制的地基。实现要点：
//!
//! 1. 对象键按 Unicode code point 升序排序（Rust `String` 的 `Ord` 是逐字节字典序，
//!    UTF-8 保持 code point 序 ⟹ 与 RFC 8785 要求的排序一致；这里仍显式排序，
//!    不依赖 serde_json 的 Map 后端是 BTreeMap）。
//! 2. 无空白、无多余分隔符；数组元素 "," 连接；对象 "k":v 连接。
//! 3. 字符串按 JSON 字符串转义（serde_json 负责转义，输出最短且不丢信息）。
//! 4. 数字走 RFC 8785 的 ECMAScript 最短表示：整数不带小数点，`-0` 规范成 `0`。
//!    （本封存器的 manifest 只出现非负整数与字符串，浮点分支仅为防御性覆盖。）
//!
//! 禁止引入 `preserve_order` feature：JCS 本就要排序，插入序是噪音。

use serde_json::{Map, Number, Value};

/// 把任意 JSON 值规范化成 RFC 8785 的 UTF-8 字节串。
pub fn canonical_bytes(value: &Value) -> Vec<u8> {
    canonicalize(value).into_bytes()
}

/// 把任意 JSON 值规范化成 RFC 8785 的字符串（UTF-8）。
pub fn canonicalize(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&canonical_number(n)),
        Value::String(s) => {
            // serde_json 把 String 序列化为带引号、已转义的 JSON 字符串。
            out.push_str(&serde_json::to_string(s).expect("string serialize"));
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => write_object(map, out),
    }
}

fn write_object(map: &Map<String, Value>, out: &mut String) {
    out.push('{');
    let mut keys: Vec<&String> = map.keys().collect();
    // Unicode code point 升序（String 字节序 == code point 序，显式排序更自明）。
    keys.sort_unstable();
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::to_string(*key).expect("key serialize"));
        out.push(':');
        write_value(&map[*key], out);
    }
    out.push('}');
}

fn canonical_number(n: &Number) -> String {
    // serde_json::Number 内部三态：u64 / i64 / f64。先按整数取，保证整数不带小数点；
    // 只有真正的浮点才走最短浮点表示。`-0.0` 规范成 `0`（RFC 8785 §3.2.2.3）。
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(f) = n.as_f64() {
        if f == 0.0 {
            return "0".to_string();
        }
        // ryu 最短往返表示；本封存器不产生浮点 manifest 值，此分支为防御。
        return n.to_string();
    }
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jcs_sorts_object_keys() {
        let v = json!({"z": 1, "a": 2, "m": {"y": true, "b": false}});
        assert_eq!(
            canonicalize(&v),
            r#"{"a":2,"m":{"b":false,"y":true},"z":1}"#
        );
    }

    #[test]
    fn jcs_handles_nested_and_arrays() {
        let v = json!({"b": [1, 2, 3], "a": null});
        assert_eq!(canonicalize(&v), r#"{"a":null,"b":[1,2,3]}"#);
    }

    #[test]
    fn jcs_string_escaping_round_trips() {
        let v = json!({"k": "line\n\u{0000}\"quote\"\\"});
        let canonical = canonicalize(&v);
        assert_eq!(canonical, r#"{"k":"line\n\u0000\"quote\"\\"}"#);
    }

    #[test]
    fn jcs_numbers_are_integer_when_integral() {
        assert_eq!(canonicalize(&json!(0)), "0");
        assert_eq!(canonicalize(&json!(42)), "42");
        assert_eq!(canonicalize(&json!(-7)), "-7");
        assert_eq!(canonicalize(&json!(0.0)), "0");
        assert_eq!(canonicalize(&json!(-0.0)), "0");
        assert_eq!(canonicalize(&json!(1.5)), "1.5");
    }

    #[test]
    fn jcs_empty_object_and_array() {
        assert_eq!(canonicalize(&json!({})), "{}");
        assert_eq!(canonicalize(&json!([])), "[]");
    }
}
