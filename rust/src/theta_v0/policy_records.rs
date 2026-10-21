//! #1603（#1586 审计 C-09/C-10，ADR 0041）：级别塔上界 `l_max` 与 MACD 力度参数的版本化记录。
//!
//! 两个量都是自由度（ADR 0041 裁定一）：塔高上界是资源预算，MACD 12/26/9 是原文给的一般取值
//! （`025-第25课.md:16`【正文】）。按 ADR 0041 裁定三，它们只从具名版本化记录读取：
//!
//! - `tower-level-budget`：[`policy_records/tower-level-budget.json`]，字段 `l_max`；
//! - `macd-force`：[`policy_records/macd-force.json`]，字段 `fast`/`slow`/`signal`。
//!
//! #1604（#1586 审计 C-04/A-12）沿同一形制再加两份：
//!
//! - `second-seq-scan-budget`（v2）：[`policy_records/second-seq-scan-budget.json`]，字段
//!   `second_seq_window`（v1 原有的第二序列窗口）与 `tail_window`（原代码常数 `TAIL_WINDOW = 7`）；
//! - `segment-second`（v2）：[`policy_records/segment-second.json`]，线段构造规则记录，字段
//!   `tangency ∈ {closed, open}`（xianduan §6.3）。`id/version` 就是现行规则 token
//!   `segment-second/2`（`parser::segment::CONSTRUCTION_RULE`），不新增 token。
//!
//! 记录随源码入库并编进二进制（`include_str!`）；数值只写在记录文件里，Rust 代码不再持有这些
//! 常数。改值 ＝ 改签新版本（`version` +1），按新值重推导；输出经 [`PolicyRef`] 写明所用记录
//! id 与版本（见 `classifier::pipeline::ClassifyOutput::policy`）。
//!
//! ## 缺值行为（ADR 0041 裁定二推论、#799）
//!
//! 记录缺失 ⟹ [`PolicyError::RecordMissing`]；字段缺失、字段直写或 `value` 为 `null` /
//! `"AwaitingPolicy"` ⟹ [`PolicyError::AwaitingPolicy`]；值非法 ⟹ [`PolicyError::Invalid`]。三者都是错误、
//! 调用方必须停止；本模块**没有**备用值，也不回退到任何旧常数。

use serde_json::Value;

use super::config::{LevelConfig, MacdConfig, Tangency};

/// 级别塔资源预算记录 id（C-09）。
pub const TOWER_LEVEL_BUDGET_ID: &str = "tower-level-budget";
/// MACD 力度 policy 记录 id（C-10）。
pub const MACD_FORCE_ID: &str = "macd-force";
/// 特征序列扫描预算记录 id（C-04，#1604）。
pub const SECOND_SEQ_SCAN_BUDGET_ID: &str = "second-seq-scan-budget";
/// 线段构造规则记录 id（A-12，#1604；xianduan §6.3 载体）。
pub const SEGMENT_CONSTRUCTION_ID: &str = "segment-second";

/// 当前生效的级别塔资源预算记录（编进二进制）。
pub const ACTIVE_TOWER_LEVEL_BUDGET_JSON: &str =
    include_str!("policy_records/tower-level-budget.json");
/// 当前生效的 MACD 力度 policy 记录（编进二进制）。
pub const ACTIVE_MACD_FORCE_JSON: &str = include_str!("policy_records/macd-force.json");
/// 当前生效的特征序列扫描预算记录（编进二进制）。
pub const ACTIVE_SECOND_SEQ_SCAN_BUDGET_JSON: &str =
    include_str!("policy_records/second-seq-scan-budget.json");
/// 当前生效的线段构造规则记录（编进二进制；Python 侧读同一文件，#1604）。
pub const ACTIVE_SEGMENT_CONSTRUCTION_JSON: &str =
    include_str!("policy_records/segment-second.json");

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
    /// 字段未签：字段缺失；字段直写为 `null` / `"AwaitingPolicy"`；或 `value` 缺失、为 `null` /
    /// `"AwaitingPolicy"`。记录头 `version` 缺失或为 `null` 同判此类（`field = "version"`）。
    AwaitingPolicy { id: &'static str, field: String },
    /// 记录或字段值非法。
    Invalid {
        id: &'static str,
        field: String,
        reason: String,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::RecordMissing { id } => {
                write!(f, "RecordMissing{{{id}}}：未提供版本化记录")
            }
            PolicyError::AwaitingPolicy { id, field } => {
                write!(
                    f,
                    "AwaitingPolicy{{{id}.{field}}}：字段未签，不回退任何备用值"
                )
            }
            PolicyError::Invalid { id, field, reason } => {
                write!(f, "InvalidPolicy{{{id}.{field}}}：{reason}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

pub(super) fn invalid(id: &'static str, field: &str, reason: impl Into<String>) -> PolicyError {
    PolicyError::Invalid {
        id,
        field: field.to_string(),
        reason: reason.into(),
    }
}

/// 解析记录头：`id` 必须等于期望 id，`version` 必须是正整数。返回 (`PolicyRef`, `fields` 对象)。
pub(super) fn parse_header(
    id: &'static str,
    text: Option<&str>,
) -> Result<(PolicyRef, serde_json::Map<String, Value>), PolicyError> {
    let text = text.ok_or(PolicyError::RecordMissing { id })?;
    let root: Value = serde_json::from_str(text)
        .map_err(|e| invalid(id, "<record>", format!("不是合法 JSON：{e}")))?;
    let obj = root
        .as_object()
        .ok_or_else(|| invalid(id, "<record>", "记录必须是 JSON 对象"))?;
    match obj.get("id").and_then(Value::as_str) {
        Some(got) if got == id => {}
        Some(got) => {
            return Err(invalid(
                id,
                "id",
                format!("记录 id `{got}` 与期望 `{id}` 不符"),
            ))
        }
        None => return Err(invalid(id, "id", "缺 id")),
    }
    let version = match obj.get("version") {
        None | Some(Value::Null) => {
            return Err(PolicyError::AwaitingPolicy {
                id,
                field: "version".to_string(),
            })
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
    Ok((
        PolicyRef {
            id,
            version: PolicyVersion::Signed(version as u32),
        },
        fields,
    ))
}

/// 取已签字段的 `value`（`fields.<name>.value`）；缺失或未签 ⟹ `AwaitingPolicy`。
fn signed_value<'a>(
    id: &'static str,
    fields: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Result<&'a Value, PolicyError> {
    let awaiting = || PolicyError::AwaitingPolicy {
        id,
        field: name.to_string(),
    };
    let entry = fields.get(name).ok_or_else(awaiting)?;
    // 直接写 `"l_max": null` / `"l_max": "AwaitingPolicy"` 也按未签处理（ADR 0041 裁定三：签署门默认值
    // 就是 AwaitingPolicy）；其余非对象写法判格式错。规范写法为 `{"value": …}`。
    if entry.is_null() || entry.as_str() == Some("AwaitingPolicy") {
        return Err(awaiting());
    }
    let entry = entry
        .as_object()
        .ok_or_else(|| invalid(id, name, format!("字段须为带 value 的对象，得 {entry}")))?;
    match entry.get("value") {
        None | Some(Value::Null) => Err(awaiting()),
        Some(Value::String(s)) if s == "AwaitingPolicy" => Err(awaiting()),
        Some(v) => Ok(v),
    }
}

<<<<<<< HEAD
/// 读一个字符串字段（`fields.<name>.value`）；缺失与未签的口径同 [`read_u32`]，非字符串判非法。
fn read_str(
    id: &'static str,
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<String, PolicyError> {
    let awaiting = || PolicyError::AwaitingPolicy {
        id,
        field: name.to_string(),
    };
    let entry = fields.get(name).ok_or_else(awaiting)?;
    if entry.is_null() || entry.as_str() == Some("AwaitingPolicy") {
        return Err(awaiting());
    }
    let entry = entry
        .as_object()
        .ok_or_else(|| invalid(id, name, format!("字段须为带 value 的对象，得 {entry}")))?;
    match entry.get("value").ok_or_else(awaiting)? {
        Value::Null => Err(awaiting()),
        Value::String(s) if s == "AwaitingPolicy" => Err(awaiting()),
        Value::String(s) => Ok(s.clone()),
        v => Err(invalid(id, name, format!("须为字符串，得 {v}"))),
    }
=======
/// 读一个非负整数字段（`fields.<name>.value`）。
pub(super) fn read_u32(
    id: &'static str,
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<u32, PolicyError> {
    let v = signed_value(id, fields, name)?;
    v.as_u64()
        .filter(|&n| n <= u64::from(u32::MAX))
        .map(|n| n as u32)
        .ok_or_else(|| invalid(id, name, format!("须为非负整数，得 {v}")))
}

/// 读一个有限浮点字段（`fields.<name>.value`）。值域由调用方再校验。
pub(super) fn read_f64(
    id: &'static str,
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<f64, PolicyError> {
    let v = signed_value(id, fields, name)?;
    v.as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| invalid(id, name, format!("须为有限数，得 {v}")))
}

/// 读一个布尔字段（`fields.<name>.value`）。
pub(super) fn read_bool(
    id: &'static str,
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<bool, PolicyError> {
    let v = signed_value(id, fields, name)?;
    v.as_bool()
        .ok_or_else(|| invalid(id, name, format!("须为 true/false，得 {v}")))
}

/// 读一个定长有限浮点数组字段（`fields.<name>.value`）。
pub(super) fn read_f64_array<const N: usize>(
    id: &'static str,
    fields: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<[f64; N], PolicyError> {
    let v = signed_value(id, fields, name)?;
    let bad = || invalid(id, name, format!("须为 {N} 个有限数的数组，得 {v}"));
    let arr = v.as_array().filter(|a| a.len() == N).ok_or_else(bad)?;
    let mut out = [0.0; N];
    for (slot, x) in out.iter_mut().zip(arr) {
        *slot = x.as_f64().filter(|x| x.is_finite()).ok_or_else(bad)?;
    }
    Ok(out)
>>>>>>> 32d071d341 (feat(theta_v0): #1602——执行成本、Θ_risk、场所 tick 与坏 tick 闸落版本化记录)
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
        return Err(invalid(
            MACD_FORCE_ID,
            "slow",
            format!("须 fast < slow，得 {fast} ≥ {slow}"),
        ));
    }
    Ok(MacdConfig {
        fast,
        slow,
        signal,
        record,
    })
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

/// `second-seq-scan-budget` 记录的解析结果（C-04）。两个窗口都是 0 = 不设上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanBudget {
    pub record: PolicyRef,
    /// 第二特征序列扫描窗口（`ParseConfig::second_seq_scan_window`）。
    pub second_seq_window: u32,
    /// 主特征序列尾窗（`ParseConfig::tail_window`）。
    pub tail_window: u32,
}

/// 解析 `second-seq-scan-budget` 记录。v1 只是 S 线形里的 token、没有记录文件，故 `version < 2`
/// 的记录判非法（v1 没有 `tail_window` 字段，不能拿它补值）。
pub fn parse_second_seq_scan_budget(text: Option<&str>) -> Result<ScanBudget, PolicyError> {
    let (record, fields) = parse_header(SECOND_SEQ_SCAN_BUDGET_ID, text)?;
    if record.version == PolicyVersion::Signed(1) {
        return Err(invalid(
            SECOND_SEQ_SCAN_BUDGET_ID,
            "version",
            "v1 未承载 tail_window（#1604 起为 v2）",
        ));
    }
    let second_seq_window = read_u32(SECOND_SEQ_SCAN_BUDGET_ID, &fields, "second_seq_window")?;
    let tail_window = read_u32(SECOND_SEQ_SCAN_BUDGET_ID, &fields, "tail_window")?;
    Ok(ScanBudget {
        record,
        second_seq_window,
        tail_window,
    })
}

/// 线段构造规则记录的解析结果（A-12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentConstruction {
    pub record: PolicyRef,
    pub tangency: Tangency,
}

/// 解析 `segment-second` 记录。`tangency` 只认 `closed` / `open`，其余判非法。
pub fn parse_segment_construction(text: Option<&str>) -> Result<SegmentConstruction, PolicyError> {
    let (record, fields) = parse_header(SEGMENT_CONSTRUCTION_ID, text)?;
    let raw = read_str(SEGMENT_CONSTRUCTION_ID, &fields, "tangency")?;
    let tangency = Tangency::from_token(&raw).ok_or_else(|| {
        invalid(
            SEGMENT_CONSTRUCTION_ID,
            "tangency",
            format!("须为 closed 或 open，得 `{raw}`"),
        )
    })?;
    Ok(SegmentConstruction { record, tangency })
}

/// 当前生效记录的特征序列扫描预算。
pub fn active_second_seq_scan_budget() -> Result<ScanBudget, PolicyError> {
    parse_second_seq_scan_budget(Some(ACTIVE_SECOND_SEQ_SCAN_BUDGET_JSON))
}

/// 当前生效记录的线段构造规则。
pub fn active_segment_construction() -> Result<SegmentConstruction, PolicyError> {
    parse_segment_construction(Some(ACTIVE_SEGMENT_CONSTRUCTION_JSON))
}

/// 由预算记录 + 中枢三段下限组装 [`LevelConfig`]。
pub fn level_config_from(budget: TowerLevelBudget, min_parts_per_level: u32) -> LevelConfig {
    LevelConfig {
        l_max: budget.l_max,
        min_parts_per_level,
        budget: budget.record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOWER_V2_L3: &str =
        r#"{"id":"tower-level-budget","version":2,"fields":{"l_max":{"value":3}}}"#;
    const MACD_V2: &str = r#"{"id":"macd-force","version":2,"fields":{"fast":{"value":5},"slow":{"value":35},"signal":{"value":5}}}"#;

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
        assert_eq!(
            (t.l_max, t.record.label().as_str()),
            (3, "tower-level-budget/2")
        );
        let m = parse_macd_force(Some(MACD_V2)).unwrap();
        assert_eq!((m.fast, m.slow, m.signal), (5, 35, 5));
        assert_eq!(m.record.label(), "macd-force/2");
    }

    /// 就地改写守卫的字段（#1603 的 `level`/`macd` + #1604 的尾窗 / 第二序列窗口 / 相切）。
    const IN_PLACE_GUARDED: [&str; 7] = [
        ".level.l_max",
        ".macd.fast",
        ".macd.slow",
        ".macd.signal",
        ".tail_window",
        ".second_seq_scan_window",
        ".tangency",
    ];

    fn is_1604_field(pat: &str) -> bool {
        IN_PLACE_GUARDED[4..].contains(&pat)
    }

    /// 源码守卫（独审 note 1/2；#1604 扩展）：Rust 源码里不得再写这几个记录值的常数，也不得就地
    /// 改写记录字段（就地改写会让输出戳仍写签署版本）。研究工具须走 `research_override_*`。
    #[test]
    fn source_guard_no_constants_or_in_place_writes() {
        let config_src = include_str!("config.rs");
        for lit in [
            "l_max: 6",
            "fast: 12",
            "slow: 26",
            "signal: 9",
            "tail_window: 7",
            "second_seq_scan_window: 0",
            "tangency: Tangency::",
        ] {
            assert!(!config_src.contains(lit), "config.rs 又出现常数 `{lit}`");
        }
        let segment_src = include_str!("parser/segment.rs");
        assert!(
            !segment_src.lines().any(|l| l
                .split("//")
                .next()
                .unwrap_or("")
                .contains("TAIL_WINDOW")),
            "parser/segment.rs 又出现尾窗常数 `TAIL_WINDOW`（#1604）"
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![root];
        let mut offenders = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    for (i, line) in text.lines().enumerate() {
                        let code = line.split("//").next().unwrap_or("");
                        for pat in IN_PLACE_GUARDED {
                            for (pos, _) in code.match_indices(pat) {
                                // #1604 三字段：`research_override_*` 与状态机内部的 `self.x = …` 不算。
                                if is_1604_field(pat) && code[..pos].ends_with("self") {
                                    continue;
                                }
                                let rest = code[pos + pat.len()..].trim_start();
                                if rest.starts_with('=') && !rest.starts_with("==") {
                                    offenders.push(format!("{}:{}", path.display(), i + 1));
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(offenders.is_empty(), "就地改写记录字段：{offenders:?}");
    }

    /// 负控：缺记录 / 缺字段 / 字段未签 / 非法值各一例，均报错（无备用值）。
    #[test]
    fn negative_controls_missing_unsigned_invalid() {
        assert_eq!(
            parse_tower_level_budget(None),
            Err(PolicyError::RecordMissing {
                id: TOWER_LEVEL_BUDGET_ID
            })
        );
        assert_eq!(
            parse_macd_force(None),
            Err(PolicyError::RecordMissing { id: MACD_FORCE_ID })
        );

        let no_field = r#"{"id":"tower-level-budget","version":1,"fields":{}}"#;
        assert_eq!(
            parse_tower_level_budget(Some(no_field)),
            Err(PolicyError::AwaitingPolicy {
                id: TOWER_LEVEL_BUDGET_ID,
                field: "l_max".into()
            })
        );
        let unsigned = r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":{"value":"AwaitingPolicy"}}}"#;
        assert!(matches!(
            parse_tower_level_budget(Some(unsigned)),
            Err(PolicyError::AwaitingPolicy { .. })
        ));
        let null_signal = r#"{"id":"macd-force","version":1,"fields":{"fast":{"value":12},"slow":{"value":26},"signal":{"value":null}}}"#;
        assert_eq!(
            parse_macd_force(Some(null_signal)),
            Err(PolicyError::AwaitingPolicy {
                id: MACD_FORCE_ID,
                field: "signal".into()
            })
        );

        let negative = r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":{"value":-1}}}"#;
        assert!(matches!(
            parse_tower_level_budget(Some(negative)),
            Err(PolicyError::Invalid { .. })
        ));
        let fast_ge_slow = r#"{"id":"macd-force","version":1,"fields":{"fast":{"value":26},"slow":{"value":12},"signal":{"value":9}}}"#;
        assert!(matches!(
            parse_macd_force(Some(fast_ge_slow)),
            Err(PolicyError::Invalid { .. })
        ));
        let zero_period = r#"{"id":"macd-force","version":1,"fields":{"fast":{"value":0},"slow":{"value":26},"signal":{"value":9}}}"#;
        assert!(matches!(
            parse_macd_force(Some(zero_period)),
            Err(PolicyError::Invalid { .. })
        ));
        let wrong_id = r#"{"id":"macd-force","version":1,"fields":{"l_max":{"value":6}}}"#;
        assert!(matches!(
            parse_tower_level_budget(Some(wrong_id)),
            Err(PolicyError::Invalid { .. })
        ));
        let bare_unsigned =
            r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":"AwaitingPolicy"}}"#;
        assert!(matches!(
            parse_tower_level_budget(Some(bare_unsigned)),
            Err(PolicyError::AwaitingPolicy { .. })
        ));
        // #1653（#1603 尾审 LOW-1）：直写 `null` 独立负控——只删 `entry.is_null()` 分支会退成 Invalid。
        let bare_null = r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":null}}"#;
        assert_eq!(
            parse_tower_level_budget(Some(bare_null)),
            Err(PolicyError::AwaitingPolicy {
                id: TOWER_LEVEL_BUDGET_ID,
                field: "l_max".into()
            })
        );
        let bare = r#"{"id":"tower-level-budget","version":1,"fields":{"l_max":6}}"#;
        assert!(matches!(
            parse_tower_level_budget(Some(bare)),
            Err(PolicyError::Invalid { .. })
        ));
        let bad_version = r#"{"id":"macd-force","version":0,"fields":{}}"#;
        assert!(matches!(
            parse_macd_force(Some(bad_version)),
            Err(PolicyError::Invalid { .. })
        ));
    }

    /// #1604 跨层一致锁（#804）：相切取值只能来自记录。生产源码（`mod tests` 之前、非
    /// `*_tests.rs`）里除记录解析处（`config.rs` 定义、`policy_records.rs`）外不得点名
    /// `Tangency::Closed` / `Tangency::Open`——任一层回退成常量即红。
    #[test]
    fn source_guard_tangency_only_from_record() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![root];
        let mut offenders = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let skip = !name.ends_with(".rs")
                    || name.ends_with("_tests.rs")
                    || ((name == "config.rs" || name == "policy_records.rs")
                        && path.parent().is_some_and(|d| d.ends_with("theta_v0")));
                if skip {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let prod = text.split("#[cfg(test)]\nmod tests {").next().unwrap_or("");
                for (i, line) in prod.lines().enumerate() {
                    let code = line.split("//").next().unwrap_or("");
                    if code.contains("Tangency::Closed") || code.contains("Tangency::Open") {
                        offenders.push(format!("{}:{}", path.display(), i + 1));
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "生产路径硬编码相切取值：{offenders:?}"
        );
    }

    #[test]
    fn active_scan_budget_and_construction_records() {
        let b = active_second_seq_scan_budget().expect("生效扫描预算记录可解析");
        assert_eq!((b.second_seq_window, b.tail_window), (0, 7));
        assert_eq!(b.record.label(), "second-seq-scan-budget/2");
        let c = active_segment_construction().expect("生效线段构造记录可解析");
        assert_eq!(c.tangency, Tangency::Closed);
        assert_eq!(
            c.record.label(),
            super::super::parser::segment::CONSTRUCTION_RULE
        );
    }

    #[test]
    fn scan_budget_and_construction_other_versions() {
        let b3 = r#"{"id":"second-seq-scan-budget","version":3,"fields":{"second_seq_window":{"value":4},"tail_window":{"value":1}}}"#;
        let b = parse_second_seq_scan_budget(Some(b3)).unwrap();
        assert_eq!((b.second_seq_window, b.tail_window), (4, 1));
        assert_eq!(b.record.label(), "second-seq-scan-budget/3");
        let c3 = r#"{"id":"segment-second","version":3,"fields":{"tangency":{"value":"open"}}}"#;
        let c = parse_segment_construction(Some(c3)).unwrap();
        assert_eq!(c.tangency, Tangency::Open);
        assert_eq!(c.record.label(), "segment-second/3");
    }

    /// #1604 负控：缺记录 / 缺字段 / 未签 / 非法值 / v1 预算，均报错（无备用值）。
    #[test]
    fn negative_controls_scan_budget_and_tangency() {
        assert_eq!(
            parse_second_seq_scan_budget(None),
            Err(PolicyError::RecordMissing {
                id: SECOND_SEQ_SCAN_BUDGET_ID
            })
        );
        assert_eq!(
            parse_segment_construction(None),
            Err(PolicyError::RecordMissing {
                id: SEGMENT_CONSTRUCTION_ID
            })
        );
        let no_tail = r#"{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0}}}"#;
        assert_eq!(
            parse_second_seq_scan_budget(Some(no_tail)),
            Err(PolicyError::AwaitingPolicy {
                id: SECOND_SEQ_SCAN_BUDGET_ID,
                field: "tail_window".into()
            })
        );
        let no_tangency = r#"{"id":"segment-second","version":2,"fields":{}}"#;
        assert_eq!(
            parse_segment_construction(Some(no_tangency)),
            Err(PolicyError::AwaitingPolicy {
                id: SEGMENT_CONSTRUCTION_ID,
                field: "tangency".into()
            })
        );
        let unsigned = r#"{"id":"segment-second","version":2,"fields":{"tangency":{"value":"AwaitingPolicy"}}}"#;
        assert!(matches!(
            parse_segment_construction(Some(unsigned)),
            Err(PolicyError::AwaitingPolicy { .. })
        ));
        let null_tail = r#"{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":null}}}"#;
        assert!(matches!(
            parse_second_seq_scan_budget(Some(null_tail)),
            Err(PolicyError::AwaitingPolicy { .. })
        ));
        for bad in [
            r#"{"id":"segment-second","version":2,"fields":{"tangency":{"value":"half"}}}"#,
            r#"{"id":"segment-second","version":2,"fields":{"tangency":{"value":"Closed"}}}"#,
            r#"{"id":"segment-second","version":2,"fields":{"tangency":{"value":1}}}"#,
            r#"{"id":"segment-second","version":2,"fields":{"tangency":"closed"}}"#,
            r#"{"id":"macd-force","version":2,"fields":{"tangency":{"value":"closed"}}}"#,
        ] {
            assert!(
                matches!(
                    parse_segment_construction(Some(bad)),
                    Err(PolicyError::Invalid { .. })
                ),
                "{bad}"
            );
        }
        for bad in [
            r#"{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":-1}}}"#,
            r#"{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":7}}"#,
            r#"{"id":"second-seq-scan-budget","version":1,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":7}}}"#,
        ] {
            assert!(
                matches!(
                    parse_second_seq_scan_budget(Some(bad)),
                    Err(PolicyError::Invalid { .. })
                ),
                "{bad}"
            );
        }
    }
}
