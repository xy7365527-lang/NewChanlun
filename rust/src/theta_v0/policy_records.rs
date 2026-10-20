//! #1603（#1586 审计 C-09/C-10，ADR 0041）：级别塔上界 `l_max` 与 MACD 力度参数的版本化记录。
//!
//! 两个量都是自由度（ADR 0041 裁定一）：塔高上界是资源预算，MACD 12/26/9 是原文给的一般取值
//! （`025-第25课.md:16`【正文】）。按 ADR 0041 裁定三，它们只从具名版本化记录读取：
//!
//! - `tower-level-budget`：[`policy_records/tower-level-budget.json`]，字段 `l_max`；
//! - `macd-force`：[`policy_records/macd-force.json`]，字段 `fast`/`slow`/`signal`。
//!
//! 记录随源码入库并编进二进制（`include_str!`）；数值只写在记录文件里，Rust 代码不再持有这些
//! 常数。改值 ＝ 改签新版本（`version` +1），按新值重推导；输出经 [`PolicyRef`] 写明所用记录
//! id 与版本（见 `classifier::pipeline::ClassifyOutput::policy`）。
//!
//! ## 缺值行为（ADR 0041 裁定二推论、#799）
//!
//! 记录缺失 ⟹ [`PolicyError::RecordMissing`]；字段缺失、`value` 为 `null` 或 `"AwaitingPolicy"`
//! ⟹ [`PolicyError::AwaitingPolicy`]；值非法 ⟹ [`PolicyError::Invalid`]。三者都是错误、
//! 调用方必须停止；本模块**没有**备用值，也不回退到任何旧常数。

use serde_json::Value;

use super::config::{LevelConfig, MacdConfig};

/// 级别塔资源预算记录 id（C-09）。
pub const TOWER_LEVEL_BUDGET_ID: &str = "tower-level-budget";
/// MACD 力度 policy 记录 id（C-10）。
pub const MACD_FORCE_ID: &str = "macd-force";

/// 当前生效的级别塔资源预算记录（编进二进制）。
pub const ACTIVE_TOWER_LEVEL_BUDGET_JSON: &str = include_str!("policy_records/tower-level-budget.json");
/// 当前生效的 MACD 力度 policy 记录（编进二进制）。
pub const ACTIVE_MACD_FORCE_JSON: &str = include_str!("policy_records/macd-force.json");

/// 记录版本。`Signed(n)` 来自记录文件的 `version` 字段；`ResearchOverride` 是研究/镜像工具在
/// 代码里改了字段值、没有对应签署版本的情形——输出照实写出，不冒充签署版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyVersion {
    Signed(u32),
    ResearchOverride,
}

/// 输出里携带的记录引用：「用的是哪份记录的哪一版」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PolicyRef {
    pub id: &'static str,
    pub version: PolicyVersion,
}

impl PolicyRef {
    /// `id/版本` 形式，例如 `tower-level-budget/1`；研究覆写写作 `tower-level-budget/research-override`。
    pub fn label(&self) -> String {
        match self.version {
            PolicyVersion::Signed(v) => format!("{}/{}", self.id, v),
            PolicyVersion::ResearchOverride => format!("{}/research-override", self.id),
        }
    }
}

impl std::fmt::Display for PolicyRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

/// 读取记录失败。调用方必须停止，不得换值重试。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// 记录不存在（未提供记录文本）。
    RecordMissing { id: &'static str },
    /// 字段未签：字段缺失，或 `value` 为 `null` / `"AwaitingPolicy"`。
    AwaitingPolicy { id: &'static str, field: String },
    /// 记录或字段值非法。
    Invalid { id: &'static str, field: String, reason: String },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::RecordMissing { id } => write!(f, "RecordMissing{{{id}}}：未提供版本化记录"),
            PolicyError::AwaitingPolicy { id, field } => {
                write!(f, "AwaitingPolicy{{{id}.{field}}}：字段未签，不回退任何备用值")
            }
            PolicyError::Invalid { id, field, reason } => {
                write!(f, "InvalidPolicy{{{id}.{field}}}：{reason}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

fn invalid(id: &'static str, field: &str, reason: impl Into<String>) -> PolicyError {
    PolicyError::Invalid { id, field: field.to_string(), reason: reason.into() }
}

/// 解析记录头：`id` 必须等于期望 id，`version` 必须是正整数。返回 (`PolicyRef`, `fields` 对象)。
fn parse_header(
    id: &'static str,
    text: Option<&str>,
) -> Result<(PolicyRef, serde_json::Map<String, Value>), PolicyError> {
    let text = text.ok_or(PolicyError::RecordMissing { id })?;
    let root: Value =
        serde_json::from_str(text).map_err(|e| invalid(id, "<record>", format!("不是合法 JSON：{e}")))?;
    let obj = root.as_object().ok_or_else(|| invalid(id, "<record>", "记录必须是 JSON 对象"))?;
    match obj.get("id").and_then(Value::as_str) {
        Some(got) if got == id => {}
        Some(got) => return Err(invalid(id, "id", format!("记录 id `{got}` 与期望 `{id}` 不符"))),
        None => return Err(invalid(id, "id", "缺 id")),
    }
    let version = match obj.get("version") {
        None | Some(Value::Null) => {
            return Err(PolicyError::AwaitingPolicy { id, field: "version".to_string() })
        }
        Some(v) => v
            .as_u64()
            .filter(|&n| n >= 1 && n <= u64::from(u32::MAX))
            .ok_or_else(|| invalid(id, "version", format!("版本须为正整数，得 {v}")))?,
    };
    let fields = obj
        .get("fields")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| invalid(id, "fields", "缺 fields 对象"))?;
    Ok((PolicyRef { id, version: PolicyVersion::Signed(version as u32) }, fields))
}

/// 读一个非负整数字段（`fields.<name>.value`）。
fn read_u32(
    id: &'static str,
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<u32, PolicyError> {
    let awaiting = || PolicyError::AwaitingPolicy { id, field: name.to_string() };
    let value = fields.get(name).ok_or_else(awaiting)?.get("value").ok_or_else(awaiting)?;
    match value {
        Value::Null => Err(awaiting()),
        Value::String(s) if s == "AwaitingPolicy" => Err(awaiting()),
        v => v
            .as_u64()
            .filter(|&n| n <= u64::from(u32::MAX))
            .map(|n| n as u32)
            .ok_or_else(|| invalid(id, name, format!("须为非负整数，得 {v}"))),
    }
}

/// 解析 `tower-level-budget` 记录。`min_parts_per_level` 不在本记录里：它是中枢定义的三段下限
/// （#1586 C-13 判 O，原文定义），不是自由度。
pub fn parse_tower_level_budget(text: Option<&str>) -> Result<TowerLevelBudget, PolicyError> {
    let (record, fields) = parse_header(TOWER_LEVEL_BUDGET_ID, text)?;
    let l_max = read_u32(TOWER_LEVEL_BUDGET_ID, &fields, "l_max")?;
    Ok(TowerLevelBudget { record, l_max })
}

/// 解析 `macd-force` 记录。合法性：三者为正整数且 `fast < slow`。
pub fn parse_macd_force(text: Option<&str>) -> Result<MacdConfig, PolicyError> {
    let (record, fields) = parse_header(MACD_FORCE_ID, text)?;
    let fast = read_u32(MACD_FORCE_ID, &fields, "fast")?;
    let slow = read_u32(MACD_FORCE_ID, &fields, "slow")?;
    let signal = read_u32(MACD_FORCE_ID, &fields, "signal")?;
    for (name, v) in [("fast", fast), ("slow", slow), ("signal", signal)] {
        if v == 0 {
            return Err(invalid(MACD_FORCE_ID, name, "EMA 周期须 ≥ 1"));
        }
    }
    if fast >= slow {
        return Err(invalid(MACD_FORCE_ID, "slow", format!("须 fast < slow，得 {fast} ≥ {slow}")));
    }
    Ok(MacdConfig { fast, slow, signal, record })
}

/// `tower-level-budget` 记录的解析结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TowerLevelBudget {
    pub record: PolicyRef,
    pub l_max: u32,
}

/// 当前生效记录的 `tower-level-budget`。
pub fn active_tower_level_budget() -> Result<TowerLevelBudget, PolicyError> {
    parse_tower_level_budget(Some(ACTIVE_TOWER_LEVEL_BUDGET_JSON))
}

/// 当前生效记录的 MACD 力度参数。
pub fn active_macd_force() -> Result<MacdConfig, PolicyError> {
    parse_macd_force(Some(ACTIVE_MACD_FORCE_JSON))
}

/// 由预算记录 + 中枢三段下限组装 [`LevelConfig`]。
pub fn level_config_from(budget: TowerLevelBudget, min_parts_per_level: u32) -> LevelConfig {
    LevelConfig { l_max: budget.l_max, min_parts_per_level, budget: budget.record }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOWER_V2_L3: &str = r#"{"id":"tower-level-budget","version":2,"fields":{"l_max":{"value":3}}}"#;
    const MACD_V2: &str =
        r#"{"id":"macd-force","version":2,"fields":{"fast":{"value":5},"slow":{"value":35},"signal":{"value":5}}}"#;

    #[test]
    fn active_records_carry_current_values_and_version_1() {
        let t = active_tower_level_budget().expect("生效塔预算记录可解析");
        assert_eq!(t.l_max, 6);
        assert_eq!(t.record.label(), "tower-level-budget/1");
        let m = active_macd_force().expect("生效 MACD 记录可解析");
        assert_eq!((m.fast, m.slow, m.signal), (12, 26, 9));
        assert_eq!(m.record.label(), "macd-force/1");
    }

    #[test]
    fn other_legal_versions_parse_with_their_own_version() {
        let t = parse_tower_level_budget(Some(TOWER_V2_L3)).unwrap();
        assert_eq!((t.l_max, t.record.label().as_str()), (3, "tower-level-budget/2"));
        let m = parse_macd_force(Some(MACD_V2)).unwrap();
        assert_eq!((m.fast, m.slow, m.signal), (5, 35, 5));
        assert_eq!(m.record.label(), "macd-force/2");
    }

    /// 负控：缺记录 / 缺字段 / 字段未签 / 非法值各一例，均报错（无备用值）。
    #[test]
    fn negative_controls_missing_unsigned_invalid() {
        assert_eq!(
            parse_tower_level_budget(None),
            Err(PolicyError::RecordMissing { id: TOWER_LEVEL_BUDGET_ID })
        );
        assert_eq!(parse_macd_force(None), Err(PolicyError::RecordMissing { id: MACD_FORCE_ID }));

        let no_field = r#"{"id":"tower-level-budget","version":1,"fields":{}}"#;
        assert_eq!(
            parse_tower_level_budget(Some(no_field)),
            Err(PolicyError::AwaitingPolicy { id: TOWER_LEVEL_BUDGET_ID, field: "l_max".into() })
        );
        let unsigned = r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":{"value":"AwaitingPolicy"}}}"#;
        assert!(matches!(
            parse_tower_level_budget(Some(unsigned)),
            Err(PolicyError::AwaitingPolicy { .. })
        ));
        let null_signal =
            r#"{"id":"macd-force","version":1,"fields":{"fast":{"value":12},"slow":{"value":26},"signal":{"value":null}}}"#;
        assert_eq!(
            parse_macd_force(Some(null_signal)),
            Err(PolicyError::AwaitingPolicy { id: MACD_FORCE_ID, field: "signal".into() })
        );

        let negative = r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":{"value":-1}}}"#;
        assert!(matches!(parse_tower_level_budget(Some(negative)), Err(PolicyError::Invalid { .. })));
        let fast_ge_slow =
            r#"{"id":"macd-force","version":1,"fields":{"fast":{"value":26},"slow":{"value":12},"signal":{"value":9}}}"#;
        assert!(matches!(parse_macd_force(Some(fast_ge_slow)), Err(PolicyError::Invalid { .. })));
        let zero_period =
            r#"{"id":"macd-force","version":1,"fields":{"fast":{"value":0},"slow":{"value":26},"signal":{"value":9}}}"#;
        assert!(matches!(parse_macd_force(Some(zero_period)), Err(PolicyError::Invalid { .. })));
        let wrong_id = r#"{"id":"macd-force","version":1,"fields":{"l_max":{"value":6}}}"#;
        assert!(matches!(parse_tower_level_budget(Some(wrong_id)), Err(PolicyError::Invalid { .. })));
        let bad_version = r#"{"id":"macd-force","version":0,"fields":{}}"#;
        assert!(matches!(parse_macd_force(Some(bad_version)), Err(PolicyError::Invalid { .. })));
    }
}
