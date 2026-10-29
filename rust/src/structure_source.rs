//! #1769（ADR 0039 迁移步骤 1）：S 结构源声明——类别 + namespace 集合。
//!
//! 声明在 S init 时作为库的固定属性写进 `meta.structure_source`（与按库固定的
//! `rule_revision` 同型，运行中不切换）。S.AcceptInput 唯一入口逐批核对 namespace；
//! 不带声明的既有 TestOnly 库不受影响，生产 launcher 拒绝这类库。没有「校验失败改为
//! 不校验」的路径。结构源只认个股线美股 tick 或显式 TestOnly 结构输入（ADR 0020:151-154、
//! ADR 0023:9-34）；HL 等执行行情不进 S。
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// 声明在 S 库 `meta` 表中的键。
pub const META_KEY: &str = "structure_source";
/// 允许作为 S 结构源的类别（封闭集合）。
pub const CATEGORIES: [&str; 2] = ["UsEquityStockLineTick", "TestOnlyStructure"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub category: String,
    pub namespaces: BTreeSet<String>,
}

impl Declaration {
    /// 严格解析：恰好 `category` 与 `namespaces` 两键；类别在封闭集合内；namespace 非空、不重复。
    pub fn parse(value: &Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .filter(|o| o.len() == 2 && o.contains_key("category") && o.contains_key("namespaces"))
            .ok_or("InvalidDomain：结构源声明必须恰好含 category 与 namespaces")?;
        let category = object["category"]
            .as_str()
            .filter(|c| CATEGORIES.contains(c))
            .ok_or_else(|| {
                format!(
                    "InvalidDomain：结构源类别 `{}` 不在 {CATEGORIES:?} 内",
                    object["category"]
                )
            })?
            .to_string();
        let items = object["namespaces"]
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("InvalidDomain：结构源 namespaces 必须是非空数组")?;
        let mut namespaces = BTreeSet::new();
        for item in items {
            let ns = item
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("InvalidDomain：结构源 namespace 必须是非空字符串")?;
            if !namespaces.insert(ns.to_string()) {
                return Err(format!("InvalidDomain：结构源 namespace `{ns}` 重复"));
            }
        }
        Ok(Self {
            category,
            namespaces,
        })
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("InvalidDomain：读结构源声明 {} 失败：{e}", path.display()))?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| format!("InvalidDomain：结构源声明 JSON 解析失败：{e}"))?;
        Self::parse(&value)
    }

    /// 落库用的规范文本（键序固定、namespace 有序）。
    pub fn canonical(&self) -> String {
        json!({"category": self.category, "namespaces": self.namespaces}).to_string()
    }

    /// 结构源与执行能力 namespace 必须不相交：执行行情（如 HL）即使与美股 bar 同形、
    /// 类别标成个股线，也不得作为结构源。
    pub fn check_disjoint_from_execution(
        &self,
        execution_namespaces: &BTreeSet<String>,
    ) -> Result<(), String> {
        match self.namespaces.intersection(execution_namespaces).next() {
            Some(ns) => Err(format!(
                "InvalidDomain：结构源 namespace `{ns}` 属于执行能力 manifest，不得进入 S"
            )),
            None => Ok(()),
        }
    }

    pub fn admits(&self, namespace: &str) -> Result<(), String> {
        if self.namespaces.contains(namespace) {
            Ok(())
        } else {
            Err(format!(
                "InvalidDomain：source_namespace `{namespace}` 不在本库结构源声明 {:?} 内",
                self.namespaces
            ))
        }
    }
}

/// 读本库声明；`None` 表示不带声明的旧库。
pub fn stored(conn: &Connection) -> Result<Option<Declaration>, String> {
    let text: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key=?1", [META_KEY], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| format!("StorageUnavailable：读 meta.{META_KEY} 失败：{e}"))?;
    text.map(|t| {
        let value: Value = serde_json::from_str(&t)
            .map_err(|e| format!("StorageUnavailable：meta.{META_KEY} 损坏：{e}"))?;
        Declaration::parse(&value)
    })
    .transpose()
}

/// S.AcceptInput 的唯一一处声明校验：有声明的库只接纳声明内 namespace；旧库不变。
pub fn admit_input(conn: &Connection, namespace: &str) -> Result<(), String> {
    match stored(conn)? {
        Some(declaration) => declaration.admits(namespace),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_unknown_category_and_empty_set() {
        assert!(
            Declaration::parse(&json!({"category":"ExecutionFeed","namespaces":["a"]})).is_err()
        );
        assert!(
            Declaration::parse(&json!({"category":"TestOnlyStructure","namespaces":[]})).is_err()
        );
        assert!(Declaration::parse(
            &json!({"category":"TestOnlyStructure","namespaces":["a"],"x":1})
        )
        .is_err());
        let d = Declaration::parse(&json!({"category":"TestOnlyStructure","namespaces":["b","a"]}))
            .unwrap();
        assert_eq!(
            d.canonical(),
            r#"{"category":"TestOnlyStructure","namespaces":["a","b"]}"#
        );
        assert!(d.admits("a").is_ok());
        assert!(d
            .admits("hl.perp.BTC")
            .unwrap_err()
            .starts_with("InvalidDomain"));
    }
}
