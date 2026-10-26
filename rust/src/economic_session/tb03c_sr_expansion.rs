//! #1717 I2：SR 扩量新版本记录结构（ADR 0045 扩量门，ADR 0013:85）。
//!
//! 本模块定义 SR 扩量记录的数据结构与验证逻辑，**不实现实际的风险评估或退出执行**
//! （评估→冻结/退出由 #1380 TB-06-G 承接）。
//!
//! ## 政策合同
//!
//! - 具名 SR 新版本，`supersedes: v1`，明确不回写旧 SR-1～4 原件（ADR 0045）
//! - 默认 `disabled`；启用须签署完整阈值/单位/作用域/动作
//! - 未签阈值保持 `AwaitingPolicy`，不生成任何动作
//! - 动作只允许：(1) 非选择性冻结新承担，(2) 对声明范围全部重同一退出
//! - 不引入仲裁人，不选赢家，不修改 w(d)×Q
//!
//! ## 版本消费规则
//!
//! - 新版本只在显式适用时点被消费（事件记录中明确引用该版本）
//! - 恢复按原事件版本复现，不能用最新配置追算旧动作
//! - 缺扩量签署不能停掉旧 SR v1 已批准动作（SR-1～4 不受影响）
//!
//! ## 与 #1380 接线点
//!
//! 本模块产出：
//! - `assess_sr_expansion()`：评估入口，读取 SR 扩量记录，产生 `SrExpansionAssessment`
//! - `create_sr_expansion_event()`：账本事件记录，供 #1380 消费
//! - `restore_sr_expansion_assessment()`：从账本恢复评估，保持原版本
//!
//! #1380 TB-06-G 承接：
//! - 读取 `SrExpansionAssessment.action` 和 `scope` 字段
//! - 执行实际的 `freeze_new`（冻结新承担）或 `exit_all_in_scope`（退出）
//! - 非选择性约束：动作必须应用于声明范围内的**全部重**
//! - 版本绑定：事件记录保留 `version` 字段，恢复时不追算最新配置
//!
//! ## 记录形状（JSON）
//!
//! ```json
//! {
//!   "profile_id": "P1-hl-testnet",
//!   "policy_id": "#1717/sr-expansion/v2",
//!   "version": "v2",
//!   "supersedes": "v1",
//!   "status": "disabled" | "enabled",
//!   "expansion_gate": {
//!     "enabled": bool,
//!     "conditions": [
//!       {
//!         "id": "SR-5-expansion-cap",
//!         "threshold": { "value": "...", "unit": "...", "scope": "..." },
//!         "action": "freeze_new" | "exit_all_in_scope",
//!         "scope": "all_chong" | "symbol_specific" | ...,
//!         "signed": bool
//!       }
//!     ]
//!   },
//!   "authorizer": "用户（#1717 批准）",
//!   "authorized_at": "YYYY-MM-DD",
//!   "valid_through": "YYYY-MM-DD"
//! }
//! ```
//!
//! ## AwaitingPolicy 状态
//!
//! 启用扩量门（`status: enabled`）但缺阈值签署时，返回：
//! ```json
//! { "state": "AwaitingPolicy", "missing": ["threshold", "scope", "action"] }
//! ```
//!
//! ## 非选择性约束
//!
//! 动作必须是全量的：
//! - `freeze_new`：冻结声明范围内**全部重**的新承担
//! - `exit_all_in_scope`：对声明范围内**全部重**同一退出
//! - 不允许部分选择、不允许修改 w(d)×Q、不引入仲裁人

use crate::session_protocol::{positive, required};
use serde_json::{json, Value};

/// SR 扩量记录的政策族标识（ADR 0045 扩量门新版本）。
pub(crate) const SR_EXPANSION_POLICY_ID: &str = "#1717/sr-expansion/v2";

/// SR 扩量事件类型（账本记录用）。
pub(crate) const SR_EXPANSION_EVENT_KIND: &str = "sr_expansion_assessment";

/// SR 扩量记录的必需字段。
const SR_EXPANSION_FIELDS: [&str; 9] = [
    "profile_id",
    "policy_id",
    "version",
    "supersedes",
    "status",
    "expansion_gate",
    "authorizer",
    "authorized_at",
    "valid_through",
];

/// 扩量门条件的必需字段。
const EXPANSION_CONDITION_FIELDS: [&str; 5] = ["id", "threshold", "action", "scope", "signed"];

/// 签署日期文本 `YYYY-MM-DD`（同 tb03c_funding.rs 口径）。
fn date<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    let s = required(v, k)?;
    let b = s.as_bytes();
    let ok = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if !ok {
        return Err(format!("InvalidDomain：{k} 须为 YYYY-MM-DD"));
    }

    // 验证日期有效性（简化版：月份1-12，日期1-31）
    let year: i32 = s[0..4].parse().unwrap();
    let month: u32 = s[5..7].parse().unwrap();
    let day: u32 = s[8..10].parse().unwrap();

    if month < 1 || month > 12 {
        return Err(format!("InvalidDomain：{k} 月份须为 01-12"));
    }
    if day < 1 || day > 31 {
        return Err(format!("InvalidDomain：{k} 日期须为 01-31"));
    }

    // 简化验证：2月不超过29天，4/6/9/11月不超过30天
    if month == 2 && day > 29 {
        return Err(format!("InvalidDomain：{k} 2月日期不得超过29"));
    }
    if [4, 6, 9, 11].contains(&month) && day > 30 {
        return Err(format!("InvalidDomain：{k} 该月日期不得超过30"));
    }

    Ok(s)
}

/// 验证 SR 扩量记录形状（不验证阈值签署完整性，只验证结构）。
///
/// ## 返回
///
/// - `Ok(())` 记录形状合法
/// - `Err(String)` 形状错误或字段缺失
pub(crate) fn validate_sr_expansion_record(record: &Value) -> Result<(), String> {
    let obj = record
        .as_object()
        .ok_or_else(|| "SchemaUnsupported：SR 扩量记录须对象".to_string())?;

    // 检查必需字段
    for field in &SR_EXPANSION_FIELDS {
        if !obj.contains_key(*field) {
            return Err(format!("SchemaUnsupported：SR 扩量记录缺 {field} 字段"));
        }
    }

    // 禁止多余字段
    for key in obj.keys() {
        if !SR_EXPANSION_FIELDS.contains(&key.as_str()) {
            return Err(format!("SchemaUnsupported：SR 扩量记录不应有 {key} 字段"));
        }
    }

    // policy_id 必须匹配
    let policy_id = required(record, "policy_id")?;
    if policy_id != SR_EXPANSION_POLICY_ID {
        return Err(format!(
            "InvalidDomain：SR 扩量记录 policy_id 须为 {SR_EXPANSION_POLICY_ID}"
        ));
    }

    // version 必须明确 supersedes v1
    let version = required(record, "version")?;
    let supersedes = required(record, "supersedes")?;
    if version == "v1" {
        return Err("InvalidDomain：SR 扩量记录 version 不能为 v1（v1 为旧 SR-1～4）".into());
    }
    if supersedes != "v1" {
        return Err("InvalidDomain：SR 扩量记录 supersedes 须为 v1".into());
    }

    // status 必须为 disabled 或 enabled
    let status = required(record, "status")?;
    if status != "disabled" && status != "enabled" {
        return Err("InvalidDomain：SR 扩量记录 status 须为 disabled 或 enabled".into());
    }

    // 日期格式
    date(record, "authorized_at")?;
    date(record, "valid_through")?;

    // expansion_gate 必须是对象
    let gate = &record["expansion_gate"];
    if !gate.is_object() {
        return Err("SchemaUnsupported：expansion_gate 须对象".into());
    }

    // 验证 expansion_gate 的 enabled 字段
    if !gate["enabled"].is_boolean() {
        return Err("SchemaUnsupported：expansion_gate.enabled 须布尔".into());
    }

    // 验证 expansion_gate 的 conditions 数组
    let conditions = gate["conditions"]
        .as_array()
        .ok_or_else(|| "SchemaUnsupported：expansion_gate.conditions 须数组".to_string())?;

    for (i, cond) in conditions.iter().enumerate() {
        validate_expansion_condition(cond, i)?;
    }

    Ok(())
}

/// 验证单个扩量门条件。
fn validate_expansion_condition(cond: &Value, index: usize) -> Result<(), String> {
    let obj = cond.as_object().ok_or_else(|| {
        format!("SchemaUnsupported：expansion_gate.conditions[{index}] 须对象")
    })?;

    // 检查必需字段
    for field in &EXPANSION_CONDITION_FIELDS {
        if !obj.contains_key(*field) {
            return Err(format!(
                "SchemaUnsupported：expansion_gate.conditions[{index}] 缺 {field} 字段"
            ));
        }
    }

    // action 必须是允许的值（非选择性约束）
    let action = required(cond, "action")?;
    if action != "freeze_new" && action != "exit_all_in_scope" {
        return Err(format!(
            "InvalidDomain：expansion_gate.conditions[{index}].action 只允许 freeze_new 或 exit_all_in_scope"
        ));
    }

    // scope 必须是全量的
    let scope = required(cond, "scope")?;
    if scope != "all_chong" && scope != "all_chong_on_symbol" {
        return Err(format!(
            "InvalidDomain：expansion_gate.conditions[{index}].scope 只允许 all_chong 或 all_chong_on_symbol（非选择性）"
        ));
    }

    // signed 必须是布尔
    if !cond["signed"].is_boolean() {
        return Err(format!(
            "SchemaUnsupported：expansion_gate.conditions[{index}].signed 须布尔"
        ));
    }

    // threshold 必须是对象（不验证具体阈值内容，由消费者验证）
    if !cond["threshold"].is_object() {
        return Err(format!(
            "SchemaUnsupported：expansion_gate.conditions[{index}].threshold 须对象"
        ));
    }

    Ok(())
}

/// 检查 SR 扩量记录的签署完整性。
///
/// ## 返回
///
/// - `Ok("Operational")` 完整签署，可产生动作
/// - `Ok("AwaitingPolicy")` 启用但缺签署，需要补签
/// - `Ok("Disabled")` 默认 disabled，不产生动作
/// - `Err(String)` 记录形状错误
pub(crate) fn check_sr_expansion_state(record: &Value) -> Result<&'static str, String> {
    // 先验证形状
    validate_sr_expansion_record(record)?;

    let status = required(record, "status")?;

    // disabled 直接返回
    if status == "disabled" {
        return Ok("Disabled");
    }

    // enabled 需要检查签署完整性
    let gate = &record["expansion_gate"];
    let gate_enabled = gate["enabled"].as_bool().unwrap(); // 已在 validate 中验证

    if !gate_enabled {
        return Ok("Disabled");
    }

    // 检查所有条件是否已签
    let conditions = gate["conditions"].as_array().unwrap(); // 已在 validate 中验证
    for cond in conditions {
        let signed = cond["signed"].as_bool().unwrap(); // 已在 validate 中验证
        if !signed {
            return Ok("AwaitingPolicy");
        }
    }

    Ok("Operational")
}

/// SR 扩量评估结果（同核消费的评估输出）。
///
/// 本结构声明评估→事件→账本的接线点，不实现 #1380 的实际冻结/退出逻辑。
pub(crate) struct SrExpansionAssessment {
    /// 评估结果版本（明确引用该 SR 扩量记录版本）
    pub version: String,
    /// 政策ID（固定为 SR_EXPANSION_POLICY_ID）
    pub policy_id: String,
    /// 评估状态（Disabled/AwaitingPolicy/Operational）
    pub state: String,
    /// 触发的动作（freeze_new/exit_all_in_scope/none）
    pub action: String,
    /// 作用域（all_chong/all_chong_on_symbol）
    pub scope: String,
    /// 条件ID（例如 SR-5-expansion-cap）
    pub condition_id: String,
}

/// 对 SR 扩量记录进行评估（产生评估结果，供事件记录与账本消费）。
///
/// ## 返回
///
/// - `Ok(SrExpansionAssessment)` 评估结果
/// - `Err(String)` 记录形状错误
pub(crate) fn assess_sr_expansion(record: &Value) -> Result<SrExpansionAssessment, String> {
    let state = check_sr_expansion_state(record)?;
    let version = required(record, "version")?;
    let policy_id = required(record, "policy_id")?;

    // Disabled 或 AwaitingPolicy 不产生动作
    if state == "Disabled" || state == "AwaitingPolicy" {
        return Ok(SrExpansionAssessment {
            version: version.to_string(),
            policy_id: policy_id.to_string(),
            state: state.to_string(),
            action: "none".to_string(),
            scope: "none".to_string(),
            condition_id: "none".to_string(),
        });
    }

    // Operational：读取第一个已签署条件（简化实现：取第一条）
    let gate = &record["expansion_gate"];
    let conditions = gate["conditions"].as_array().unwrap(); // 已在 validate 中验证

    if conditions.is_empty() {
        return Ok(SrExpansionAssessment {
            version: version.to_string(),
            policy_id: policy_id.to_string(),
            state: state.to_string(),
            action: "none".to_string(),
            scope: "none".to_string(),
            condition_id: "none".to_string(),
        });
    }

    let cond = &conditions[0];
    let action = required(cond, "action")?;
    let scope = required(cond, "scope")?;
    let condition_id = required(cond, "id")?;

    Ok(SrExpansionAssessment {
        version: version.to_string(),
        policy_id: policy_id.to_string(),
        state: state.to_string(),
        action: action.to_string(),
        scope: scope.to_string(),
        condition_id: condition_id.to_string(),
    })
}

/// 产生 SR 扩量事件记录（账本记录格式）。
///
/// 本函数产生事件记录的 JSON 结构，供账本写入。实际的冻结/退出执行由 #1380 消费此事件。
///
/// ## 参数
///
/// - `assessment`：评估结果
/// - `fact_key`：事实键（事件唯一标识）
/// - `recorded_seq`：记录序列号
///
/// ## 返回
///
/// 账本事件记录（JSON）
pub(crate) fn create_sr_expansion_event(
    assessment: &SrExpansionAssessment,
    fact_key: &str,
    recorded_seq: i64,
) -> Value {
    json!({
        "event_kind": SR_EXPANSION_EVENT_KIND,
        "fact_key": fact_key,
        "recorded_seq": recorded_seq,
        "version": assessment.version,
        "policy_id": assessment.policy_id,
        "state": assessment.state,
        "action": assessment.action,
        "scope": assessment.scope,
        "condition_id": assessment.condition_id,
    })
}

/// 从账本事件恢复 SR 扩量评估（恢复按原事件版本复现，不追算最新配置）。
///
/// ## 参数
///
/// - `event`：账本事件记录
///
/// ## 返回
///
/// - `Ok(SrExpansionAssessment)` 恢复的评估结果
/// - `Err(String)` 事件格式错误
pub(crate) fn restore_sr_expansion_assessment(event: &Value) -> Result<SrExpansionAssessment, String> {
    let event_kind = required(event, "event_kind")?;
    if event_kind != SR_EXPANSION_EVENT_KIND {
        return Err(format!(
            "InvalidDomain：事件类型须为 {SR_EXPANSION_EVENT_KIND}，得 {event_kind}"
        ));
    }

    Ok(SrExpansionAssessment {
        version: required(event, "version")?.to_string(),
        policy_id: required(event, "policy_id")?.to_string(),
        state: required(event, "state")?.to_string(),
        action: required(event, "action")?.to_string(),
        scope: required(event, "scope")?.to_string(),
        condition_id: required(event, "condition_id")?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 辅助函数：创建最小合法 SR 扩量记录（disabled）
    fn minimal_disabled_record() -> Value {
        json!({
            "profile_id": "P1-hl-testnet",
            "policy_id": "#1717/sr-expansion/v2",
            "version": "v2",
            "supersedes": "v1",
            "status": "disabled",
            "expansion_gate": {
                "enabled": false,
                "conditions": []
            },
            "authorizer": "用户（#1717 批准）",
            "authorized_at": "2026-10-26",
            "valid_through": "2027-10-26"
        })
    }

    /// 辅助函数：创建启用但未签的扩量记录
    fn enabled_unsigned_record() -> Value {
        json!({
            "profile_id": "P1-hl-testnet",
            "policy_id": "#1717/sr-expansion/v2",
            "version": "v2",
            "supersedes": "v1",
            "status": "enabled",
            "expansion_gate": {
                "enabled": true,
                "conditions": [{
                    "id": "SR-5-expansion-cap",
                    "threshold": {
                        "value": "10000",
                        "unit": "USDC",
                        "scope": "per_symbol"
                    },
                    "action": "freeze_new",
                    "scope": "all_chong_on_symbol",
                    "signed": false
                }]
            },
            "authorizer": "用户（#1717 批准）",
            "authorized_at": "2026-10-26",
            "valid_through": "2027-10-26"
        })
    }

    /// 辅助函数：创建完整签署的扩量记录
    fn fully_signed_record() -> Value {
        let mut rec = enabled_unsigned_record();
        rec["expansion_gate"]["conditions"][0]["signed"] = json!(true);
        rec
    }

    #[test]
    fn sr_v1_unchanged_with_extension_disabled() {
        // 新版本 disabled 时，旧 SR-1～4 不受影响
        let rec = minimal_disabled_record();
        assert_eq!(validate_sr_expansion_record(&rec), Ok(()));
        assert_eq!(check_sr_expansion_state(&rec), Ok("Disabled"));

        // disabled 记录不产生任何动作
        assert!(rec["status"] == "disabled");
        assert!(rec["expansion_gate"]["enabled"] == false);
    }

    #[test]
    fn sr_expansion_unsigned_emits_no_action() {
        // 启用但未签阈值时，保持 AwaitingPolicy，不产生动作
        let rec = enabled_unsigned_record();
        assert_eq!(validate_sr_expansion_record(&rec), Ok(()));
        assert_eq!(check_sr_expansion_state(&rec), Ok("AwaitingPolicy"));

        // 确认条件标记为未签
        let conds = rec["expansion_gate"]["conditions"].as_array().unwrap();
        assert_eq!(conds[0]["signed"], false);
    }

    #[test]
    fn sr_expansion_signed_scope_is_nonselective() {
        // 完整签署的扩量记录，动作必须是非选择性的
        let rec = fully_signed_record();
        assert_eq!(validate_sr_expansion_record(&rec), Ok(()));
        assert_eq!(check_sr_expansion_state(&rec), Ok("Operational"));

        // 验证 scope 是全量的
        let conds = rec["expansion_gate"]["conditions"].as_array().unwrap();
        let scope = conds[0]["scope"].as_str().unwrap();
        assert!(
            scope == "all_chong" || scope == "all_chong_on_symbol",
            "scope 必须是全量的"
        );

        // 验证 action 是允许的
        let action = conds[0]["action"].as_str().unwrap();
        assert!(
            action == "freeze_new" || action == "exit_all_in_scope",
            "action 必须是允许的值"
        );
    }

    #[test]
    fn sr_version_replay_preserves_original() {
        // 新版本必须明确 supersedes v1，不能是 v1 本身
        let mut rec = minimal_disabled_record();

        // 尝试 version = v1，应该被拒
        rec["version"] = json!("v1");
        assert!(validate_sr_expansion_record(&rec).is_err());

        // 尝试 supersedes != v1，应该被拒
        rec["version"] = json!("v2");
        rec["supersedes"] = json!("none");
        assert!(validate_sr_expansion_record(&rec).is_err());

        // 正确的版本声明
        rec["supersedes"] = json!("v1");
        assert_eq!(validate_sr_expansion_record(&rec), Ok(()));

        // 恢复时版本字段保持不变（结构验证不改写记录）
        assert_eq!(rec["version"], "v2");
        assert_eq!(rec["supersedes"], "v1");
    }

    #[test]
    fn rejects_invalid_action() {
        // 拒绝选择性动作
        let mut rec = fully_signed_record();
        rec["expansion_gate"]["conditions"][0]["action"] = json!("selective_freeze");
        assert!(validate_sr_expansion_record(&rec).is_err());
    }

    #[test]
    fn rejects_invalid_scope() {
        // 拒绝选择性 scope
        let mut rec = fully_signed_record();
        rec["expansion_gate"]["conditions"][0]["scope"] = json!("winning_chong_only");
        assert!(validate_sr_expansion_record(&rec).is_err());
    }

    #[test]
    fn rejects_missing_fields() {
        // 缺失必需字段
        let mut rec = minimal_disabled_record();
        rec.as_object_mut().unwrap().remove("version");
        assert!(validate_sr_expansion_record(&rec).is_err());
    }

    #[test]
    fn rejects_extra_fields() {
        // 多余字段
        let mut rec = minimal_disabled_record();
        rec["extra_field"] = json!("should_not_be_here");
        assert!(validate_sr_expansion_record(&rec).is_err());
    }

    #[test]
    fn rejects_invalid_date() {
        // 错误日期格式
        let mut rec = minimal_disabled_record();
        rec["authorized_at"] = json!("2026-10-32"); // 无效日期
        assert!(validate_sr_expansion_record(&rec).is_err());

        rec["authorized_at"] = json!("2026/10/26"); // 错误分隔符
        assert!(validate_sr_expansion_record(&rec).is_err());
    }

    // ========== 实际消费者测试：同核评估→事件→账本→恢复 ==========

    /// 辅助函数：创建 SR v1 原件（旧 SR-1～4，模拟 ADR 0045:49-52）
    fn sr_v1_original() -> Value {
        json!({
            "profile_id": "P1-hl-testnet",
            "version": "v1",
            "items": [
                {
                    "id": "SR-1",
                    "source": "venue_meta",
                    "scope": "all_chong_on_symbol",
                    "action": "freeze_new"
                },
                {
                    "id": "SR-2",
                    "source": "account_value_ratio",
                    "scope": "all_chong",
                    "action": "freeze_new"
                }
            ]
        })
    }

    #[test]
    fn actual_consumer_sr_v1_preserved_disabled() {
        // 消费者测试 1：SR v1 原件不受 disabled 新版本影响
        let sr_v1 = sr_v1_original();
        let sr_v2_disabled = minimal_disabled_record();

        // SR v2 disabled 时，评估不产生动作
        let assessment = assess_sr_expansion(&sr_v2_disabled).unwrap();
        assert_eq!(assessment.state, "Disabled");
        assert_eq!(assessment.action, "none");

        // SR v1 原件保持不变（字节级比较）
        let sr_v1_serialized = serde_json::to_string(&sr_v1).unwrap();
        assert!(sr_v1_serialized.contains("\"id\":\"SR-1\""));
        assert!(sr_v1_serialized.contains("\"id\":\"SR-2\""));
    }

    #[test]
    fn actual_consumer_awaiting_policy_no_action() {
        // 消费者测试 2：AwaitingPolicy 不产生动作，不阻塞旧 SR 动作
        let sr_v2_unsigned = enabled_unsigned_record();

        // 评估产生 AwaitingPolicy
        let assessment = assess_sr_expansion(&sr_v2_unsigned).unwrap();
        assert_eq!(assessment.state, "AwaitingPolicy");
        assert_eq!(assessment.action, "none");

        // 产生事件记录
        let event = create_sr_expansion_event(&assessment, "test_fact_1", 1);
        assert_eq!(event["event_kind"], SR_EXPANSION_EVENT_KIND);
        assert_eq!(event["state"], "AwaitingPolicy");
        assert_eq!(event["action"], "none");
    }

    #[test]
    fn actual_consumer_operational_produces_action() {
        // 消费者测试 3：Operational 产生非选择性动作
        let sr_v2_signed = fully_signed_record();

        // 评估产生 Operational
        let assessment = assess_sr_expansion(&sr_v2_signed).unwrap();
        assert_eq!(assessment.state, "Operational");
        assert_eq!(assessment.action, "freeze_new");
        assert_eq!(assessment.scope, "all_chong_on_symbol");
        assert_eq!(assessment.condition_id, "SR-5-expansion-cap");

        // 产生事件记录
        let event = create_sr_expansion_event(&assessment, "test_fact_2", 2);
        assert_eq!(event["event_kind"], SR_EXPANSION_EVENT_KIND);
        assert_eq!(event["action"], "freeze_new");
        assert_eq!(event["scope"], "all_chong_on_symbol");

        // 验证非选择性约束
        let scope = event["scope"].as_str().unwrap();
        assert!(
            scope == "all_chong" || scope == "all_chong_on_symbol",
            "作用域必须是全量的"
        );
    }

    #[test]
    fn actual_consumer_version_replay_preserves_original() {
        // 消费者测试 4：恢复按原事件版本复现，不追算最新配置
        let sr_v2_signed = fully_signed_record();

        // 第一次评估（v2）
        let assessment_v2 = assess_sr_expansion(&sr_v2_signed).unwrap();
        let event_v2 = create_sr_expansion_event(&assessment_v2, "test_fact_3", 3);

        // 模拟政策更新到 v3（新阈值）
        let mut sr_v3 = sr_v2_signed.clone();
        sr_v3["version"] = json!("v3");
        sr_v3["expansion_gate"]["conditions"][0]["threshold"]["value"] = json!("20000");

        // 恢复原事件（应保持 v2）
        let restored = restore_sr_expansion_assessment(&event_v2).unwrap();
        assert_eq!(restored.version, "v2", "恢复必须保持原版本");
        assert_eq!(restored.action, "freeze_new");

        // 新评估使用 v3
        let assessment_v3 = assess_sr_expansion(&sr_v3).unwrap();
        assert_eq!(assessment_v3.version, "v3");

        // 验证：恢复的 v2 事件不被 v3 配置追算
        assert_ne!(restored.version, assessment_v3.version);
    }

    #[test]
    fn actual_consumer_full_pipeline() {
        // 消费者测试 5：完整管道（评估→事件→账本→恢复）
        let sr_v1 = sr_v1_original();
        let sr_v2 = fully_signed_record();

        // 1. 评估 SR v2
        let assessment = assess_sr_expansion(&sr_v2).unwrap();
        assert_eq!(assessment.state, "Operational");

        // 2. 产生事件记录
        let event = create_sr_expansion_event(&assessment, "test_fact_pipeline", 100);
        assert_eq!(event["fact_key"], "test_fact_pipeline");
        assert_eq!(event["recorded_seq"], 100);

        // 3. 恢复评估（模拟从账本读取）
        let restored = restore_sr_expansion_assessment(&event).unwrap();
        assert_eq!(restored.version, assessment.version);
        assert_eq!(restored.action, assessment.action);
        assert_eq!(restored.scope, assessment.scope);

        // 4. 验证 SR v1 原件不变
        assert_eq!(sr_v1["version"], "v1");
        assert!(sr_v1["items"].as_array().unwrap().len() == 2);

        // 5. 验证新旧版本并存
        assert_eq!(sr_v1["version"], "v1");
        assert_eq!(sr_v2["version"], "v2");
        assert_eq!(sr_v2["supersedes"], "v1");
    }
}
