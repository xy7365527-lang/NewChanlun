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
use serde_json::Value;

/// SR 扩量记录的政策族标识（ADR 0045 扩量门新版本）。
pub(crate) const SR_EXPANSION_POLICY_ID: &str = "#1717/sr-expansion/v2";

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
}
