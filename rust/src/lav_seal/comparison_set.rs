//! 独立的 canonical Comparison Set Manifest（ADR 0025 §4.2）。
//!
//! Bundle 不反向引用集合；全部成员 bundle `SEALED` 后，封存器才构造并原子发布本
//! manifest。`comparison_set_id = SHA-256(canonical_manifest_bytes)`，不写回。
//!
//! 关键语义：
//! - `comparison_context_digest` 是成员 bundle 内同名必填字段的逐字重复，不是集合层新摘要；
//! - `member_bundle_ids` 与 `ranking_view_digests` 等长、按位置一一对应；
//! - `member_count >= 2` 且等于两数组长度；
//! - bundle ↔ view ↔ candidate 三者唯一双射（view digest 与 candidate_id 从成员 bundle 解出后逐字核对）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::bundle::Bundle;
use super::digest;
use super::jcs;

/// ADR 0025 §4.2 的 Comparison Set Manifest。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonSetManifest {
    pub schema_version: u64,
    pub comparison_context_digest: String,
    pub member_bundle_ids: Vec<String>,
    pub ranking_view_digests: Vec<String>,
    pub member_count: u64,
    pub set_policy_digest: String,
}

/// 封存器发布的 Comparison Set：身份摘要 + manifest。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonSet {
    pub comparison_set_id: String,
    pub manifest: ComparisonSetManifest,
}

/// Comparison Set 封存/校验错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetError {
    Invalid(String),
    Infra(String),
}

impl std::fmt::Display for SetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetError::Invalid(m) => write!(f, "INVALID_INPUT: {m}"),
            SetError::Infra(m) => write!(f, "INFRA_FAILURE: {m}"),
        }
    }
}

impl std::error::Error for SetError {}

/// 从「按位置与 manifest 数组成员一一对应的已封存成员」封存 Comparison Set。
///
/// `members` 的顺序即 manifest 两数组的顺序；该顺序只服务规范身份，不表达偏好。
pub fn seal_comparison_set(
    set_policy_digest: &str,
    members: &[Bundle],
) -> Result<ComparisonSet, SetError> {
    if members.len() < 2 {
        return Err(SetError::Invalid(format!(
            "comparison set requires at least 2 members, got {}",
            members.len()
        )));
    }
    let manifest = build_manifest(set_policy_digest, members)?;
    validate_comparison_set(&manifest, members)?;
    let id = comparison_set_id(&manifest)?;
    Ok(ComparisonSet {
        comparison_set_id: id,
        manifest,
    })
}

fn build_manifest(
    set_policy_digest: &str,
    members: &[Bundle],
) -> Result<ComparisonSetManifest, SetError> {
    let mut bundle_ids = Vec::with_capacity(members.len());
    let mut view_digests = Vec::with_capacity(members.len());
    let mut context: Option<String> = None;
    for member in members {
        bundle_ids.push(member.bundle_id.clone());
        view_digests.push(member.manifest.ranking_view_digest.clone());
        match &context {
            None => context = Some(member.manifest.comparison_context_digest.clone()),
            Some(c) => {
                if *c != member.manifest.comparison_context_digest {
                    return Err(SetError::Invalid(
                        "members must share identical comparison_context_digest".into(),
                    ));
                }
            }
        }
    }
    let context = context.expect("members non-empty");
    Ok(ComparisonSetManifest {
        schema_version: 1,
        comparison_context_digest: context,
        member_bundle_ids: bundle_ids,
        ranking_view_digests: view_digests,
        member_count: members.len() as u64,
        set_policy_digest: set_policy_digest.to_string(),
    })
}

/// `comparison_set_id = SHA-256(canonical_manifest_bytes)`。
pub fn comparison_set_id(manifest: &ComparisonSetManifest) -> Result<String, SetError> {
    let value: Value =
        serde_json::to_value(manifest).map_err(|e| SetError::Infra(e.to_string()))?;
    Ok(digest::sha256_hex(&jcs::canonical_bytes(&value)))
}

/// 校验 Comparison Set Manifest 的不变量（ADR 0025 §4.2 精确语义）。
pub fn validate_comparison_set(
    manifest: &ComparisonSetManifest,
    members: &[Bundle],
) -> Result<(), SetError> {
    if manifest.schema_version != 1 {
        return Err(SetError::Invalid(format!(
            "schema_version must be 1, got {}",
            manifest.schema_version
        )));
    }
    if manifest.member_count < 2 {
        return Err(SetError::Invalid("member_count must be >= 2".into()));
    }
    if manifest.member_count as usize != members.len() {
        return Err(SetError::Invalid(format!(
            "member_count {} != members.len() {}",
            manifest.member_count,
            members.len()
        )));
    }
    if manifest.member_bundle_ids.len() != members.len()
        || manifest.ranking_view_digests.len() != members.len()
    {
        return Err(SetError::Invalid(
            "member_bundle_ids and ranking_view_digests must match member count".into(),
        ));
    }
    // 双射：bundle_id / view digest / candidate_id 均不得重复，且逐位对应成员 bundle。
    let mut seen_bundle = std::collections::HashSet::new();
    let mut seen_view = std::collections::HashSet::new();
    let mut seen_candidate = std::collections::HashSet::new();
    for (i, member) in members.iter().enumerate() {
        if manifest.member_bundle_ids[i] != member.bundle_id {
            return Err(SetError::Invalid(format!(
                "member_bundle_ids[{i}] must equal member bundle_id"
            )));
        }
        if manifest.ranking_view_digests[i] != member.manifest.ranking_view_digest {
            return Err(SetError::Invalid(format!(
                "ranking_view_digests[{i}] must equal member ranking_view_digest"
            )));
        }
        if !seen_bundle.insert(&member.bundle_id) {
            return Err(SetError::Invalid("duplicate bundle_id".into()));
        }
        if !seen_view.insert(&member.manifest.ranking_view_digest) {
            return Err(SetError::Invalid("duplicate ranking_view_digest".into()));
        }
        if !seen_candidate.insert(&member.manifest.candidate_id) {
            return Err(SetError::Invalid("duplicate candidate_id".into()));
        }
    }
    // comparison_context_digest 是成员 bundle 同名字段的逐字重复。
    let shared = &members[0].manifest.comparison_context_digest;
    if manifest.comparison_context_digest != *shared {
        return Err(SetError::Invalid(
            "manifest comparison_context_digest must repeat member value verbatim".into(),
        ));
    }
    for member in members {
        if member.manifest.comparison_context_digest != *shared {
            return Err(SetError::Invalid(
                "all members must share identical comparison_context_digest".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lav_seal::bundle::{seal_bundle, BundleManifest};
    use crate::lav_seal::digest;

    fn make_bundle(candidate_id: &str, view: &str) -> Bundle {
        make_bundle_with_input(candidate_id, view, "input")
    }

    fn make_bundle_with_input(candidate_id: &str, view: &str, input: &str) -> Bundle {
        let task = crate::lav_seal::bundle::TaskBlock {
            input_digest: digest::sha256_hex(input.as_bytes()),
            spec_digest: digest::sha256_hex(b"spec"),
            gate_profile_digest: digest::sha256_hex(b"gate"),
        };
        let slot = digest::sha256_hex(b"slot policy");
        let ctx = crate::lav_seal::bundle::comparison_context_digest(&task, &slot);
        let redaction = digest::sha256_hex(b"redaction policy");
        let empty = digest::sha256_hex(b"");
        let view_digest = digest::sha256_hex(view.as_bytes());
        let manifest = BundleManifest {
            schema_version: 1,
            candidate_id: candidate_id.into(),
            task,
            candidate_slot_policy_digest: slot,
            comparison_context_digest: ctx,
            source: crate::lav_seal::bundle::SourceBlock {
                repository_digest: digest::sha256_hex(b"repo"),
                base_commit_digest: digest::sha256_hex(b"base"),
                candidate_tree_digest: digest::sha256_hex(b"tree"),
                diff_digest: digest::sha256_hex(b"diff"),
            },
            generation: crate::lav_seal::bundle::GenerationBlock {
                run_id: "run".into(),
                provider_id: "p".into(),
                owner_vendor: "v".into(),
                owner_family: "f".into(),
                model_id: "m".into(),
                prompt_digest: digest::sha256_hex(b"prompt"),
                toolchain_digest: digest::sha256_hex(b"toolchain"),
                runtime_image_digest: digest::sha256_hex(b"runtime"),
                dependency_lock_digest: digest::sha256_hex(b"lock"),
            },
            gates: vec![crate::lav_seal::bundle::GateRecord {
                gate_id: "g".into(),
                command_digest: digest::sha256_hex(b"cmd"),
                status: "PASS".into(),
                exit_code: 0,
                stdout_blob: empty.clone(),
                stderr_blob: empty.clone(),
            }],
            artifacts: vec![
                crate::lav_seal::bundle::ArtifactRecord {
                    role: "ranking_view".into(),
                    media_type: "text/markdown".into(),
                    bytes: view.len() as u64,
                    sha256: view_digest.clone(),
                },
                crate::lav_seal::bundle::ArtifactRecord {
                    role: "stdout".into(),
                    media_type: "text/plain".into(),
                    bytes: 0,
                    sha256: empty.clone(),
                },
                crate::lav_seal::bundle::ArtifactRecord {
                    role: "stderr".into(),
                    media_type: "text/plain".into(),
                    bytes: 0,
                    sha256: empty.clone(),
                },
            ],
            ranking_view_digest: view_digest,
            redaction_policy_digest: redaction.clone(),
            sealing: crate::lav_seal::bundle::SealingBlock {
                sealer_digest: digest::sha256_hex(b"sealer"),
                seal_policy_digest: digest::sha256_hex(b"seal policy"),
                redaction_policy_digest: redaction,
            },
            state: "SEALED".into(),
        };
        seal_bundle(manifest).unwrap()
    }

    #[test]
    fn seals_set_and_computes_stable_id() {
        let a = make_bundle("cand-A", "view A");
        let b = make_bundle("cand-B", "view B");
        let set = seal_comparison_set(&digest::sha256_hex(b"set policy"), &[a, b]).unwrap();
        assert_eq!(set.manifest.member_count, 2);
        let id = comparison_set_id(&set.manifest).unwrap();
        assert_eq!(set.comparison_set_id, id);
    }

    #[test]
    fn rejects_single_member() {
        let a = make_bundle("cand-A", "view A");
        assert!(matches!(
            seal_comparison_set(&digest::sha256_hex(b"set policy"), &[a]),
            Err(SetError::Invalid(_))
        ));
    }

    #[test]
    fn rejects_mismatched_context() {
        // 两 bundle 的 task.input_digest 不同 ⟹ comparison_context_digest 不同 ⟹ 不能同组。
        let a = make_bundle_with_input("cand-A", "view A", "input A");
        let b = make_bundle_with_input("cand-B", "view B", "input B");
        assert_ne!(
            a.manifest.comparison_context_digest,
            b.manifest.comparison_context_digest
        );
        assert!(matches!(
            seal_comparison_set(&digest::sha256_hex(b"set policy"), &[a, b]),
            Err(SetError::Invalid(_))
        ));
    }

    #[test]
    fn rejects_duplicate_candidate_ids() {
        let a = make_bundle("cand-A", "view A");
        let a2 = make_bundle("cand-A", "view A2");
        assert!(matches!(
            seal_comparison_set(&digest::sha256_hex(b"set policy"), &[a, a2]),
            Err(SetError::Invalid(_))
        ));
    }
}
