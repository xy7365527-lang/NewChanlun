//! Lead 基线裁决封存（ADR 0025 §10.2 历史 pilot 盲法契约）。
//!
//! `LeadBaselineDecision` 必须在任何 LAV 结果揭示给 Lead／比较者之前先行封存，且封存后
//! 不可改写；它记录 `selected_candidate_id` 或 `REJECT_ALL`、理由与确定性证据 refs。
//! LAV 永远看不到 ground truth。
//!
//! 本模块提供「先封存、后揭示」的两段式 API：
//! 1. `seal_baseline`：先写封存面（规范化 manifest + 摘要），返回 `SealedBaseline`（含
//!    `baseline_digest`）；该摘要就是先行落盘的承诺。
//! 2. `reveal`：之后才把明文 baseline 暴露给比较方（S6 的 `ShadowComparison`）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::digest;
use super::jcs;

/// 判定枚举：选中一个候选 / 全部拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    Selected,
    RejectAll,
}

/// ADR 0025 §10.2 的 Lead 基线裁决。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeadBaselineDecision {
    pub schema_version: u64,
    /// 本 baseline 所对应的 Comparison Set（身份引用，非集合层新摘要）。
    pub comparison_set_id: String,
    pub decision: Decision,
    /// `Decision::Selected` 时的 `selected_candidate_id`；`RejectAll` 时为 `None`。
    pub selected_candidate_id: Option<String>,
    /// 理由（人读，确定性证据 refs 另列）。
    pub reason: String,
    /// 确定性证据引用（摘要，不复制被删内容）。
    pub evidence_refs: Vec<String>,
}

/// 封存后的 baseline：承诺摘要 + 明文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedBaseline {
    pub baseline_digest: String,
    pub baseline: LeadBaselineDecision,
}

/// 封存/校验错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaselineError {
    Invalid(String),
    Infra(String),
}

impl std::fmt::Display for BaselineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaselineError::Invalid(m) => write!(f, "INVALID_INPUT: {m}"),
            BaselineError::Infra(m) => write!(f, "INFRA_FAILURE: {m}"),
        }
    }
}

impl std::error::Error for BaselineError {}

/// `baseline_digest = SHA-256(canonical_baseline_bytes)`（先封存的承诺）。
pub fn baseline_digest(baseline: &LeadBaselineDecision) -> Result<String, BaselineError> {
    let value: Value =
        serde_json::to_value(baseline).map_err(|e| BaselineError::Infra(e.to_string()))?;
    Ok(digest::sha256_hex(&jcs::canonical_bytes(&value)))
}

/// 校验 baseline 不变量：decision 与 selected_candidate_id 一致；schema 固定。
pub fn validate_baseline(baseline: &LeadBaselineDecision) -> Result<(), BaselineError> {
    if baseline.schema_version != 1 {
        return Err(BaselineError::Invalid(format!(
            "schema_version must be 1, got {}",
            baseline.schema_version
        )));
    }
    if baseline.comparison_set_id.trim().is_empty() {
        return Err(BaselineError::Invalid(
            "comparison_set_id must be non-empty".into(),
        ));
    }
    match baseline.decision {
        Decision::Selected => {
            if baseline
                .selected_candidate_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err(BaselineError::Invalid(
                    "Selected decision requires selected_candidate_id".into(),
                ));
            }
        }
        Decision::RejectAll => {
            if baseline.selected_candidate_id.is_some() {
                return Err(BaselineError::Invalid(
                    "RejectAll decision must not carry selected_candidate_id".into(),
                ));
            }
        }
    }
    Ok(())
}

/// 先封存：校验 → 算摘要 → 返回 `SealedBaseline`（摘要先行落盘）。
pub fn seal_baseline(baseline: LeadBaselineDecision) -> Result<SealedBaseline, BaselineError> {
    validate_baseline(&baseline)?;
    let id = baseline_digest(&baseline)?;
    Ok(SealedBaseline {
        baseline_digest: id,
        baseline,
    })
}

/// 后揭示：只在「摘要已先行落盘」之后才允许调用方拿到明文（供 S6 比较）。
///
/// 这里把「揭示」建模成显式函数而非直接读字段，是为把「先封存后揭示」的顺序约束留在
/// 类型面上：调用方必须先持有 `SealedBaseline`（封存承诺），才能 `reveal` 出明文。
pub fn reveal(sealed: &SealedBaseline) -> &LeadBaselineDecision {
    &sealed.baseline
}

/// 构造一个 `RejectAll` 基线（无 selected_candidate_id）。
pub fn reject_all(
    comparison_set_id: String,
    reason: String,
    evidence_refs: Vec<String>,
) -> LeadBaselineDecision {
    LeadBaselineDecision {
        schema_version: 1,
        comparison_set_id,
        decision: Decision::RejectAll,
        selected_candidate_id: None,
        reason,
        evidence_refs,
    }
}

/// 构造一个 `Selected` 基线。
pub fn selected(
    comparison_set_id: String,
    selected_candidate_id: String,
    reason: String,
    evidence_refs: Vec<String>,
) -> LeadBaselineDecision {
    LeadBaselineDecision {
        schema_version: 1,
        comparison_set_id,
        decision: Decision::Selected,
        selected_candidate_id: Some(selected_candidate_id),
        reason,
        evidence_refs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_then_reveal_preserves_digest() {
        let baseline = selected(
            "set-id".into(),
            "cand-B".into(),
            "终审订正版本为当选候选".into(),
            vec![digest::sha256_hex(b"evidence-1")],
        );
        let sealed = seal_baseline(baseline).unwrap();
        // 封存后摘要即承诺；reveal 出的明文与封存时逐字一致。
        let revealed = reveal(&sealed);
        assert_eq!(baseline_digest(revealed).unwrap(), sealed.baseline_digest);
        assert_eq!(revealed.selected_candidate_id.as_deref(), Some("cand-B"));
    }

    #[test]
    fn reject_all_must_not_carry_selection() {
        let mut b = reject_all("set-id".into(), "全部拒绝".into(), vec![]);
        b.selected_candidate_id = Some("cand-X".into());
        assert!(matches!(seal_baseline(b), Err(BaselineError::Invalid(_))));
    }

    #[test]
    fn selected_requires_candidate() {
        let mut b = selected("set-id".into(), "cand-B".into(), "r".into(), vec![]);
        b.selected_candidate_id = None;
        assert!(matches!(seal_baseline(b), Err(BaselineError::Invalid(_))));
    }

    #[test]
    fn reject_all_ok() {
        let b = reject_all("set-id".into(), "全部拒绝".into(), vec![]);
        let sealed = seal_baseline(b).unwrap();
        assert_eq!(sealed.baseline.decision, Decision::RejectAll);
    }
}
