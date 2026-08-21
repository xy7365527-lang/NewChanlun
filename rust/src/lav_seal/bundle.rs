//! Candidate Evidence Bundle 封存（ADR 0025 §2.2 精确最小 schema）。
//!
//! `Candidate Evidence Bundle = RFC 8785 canonical UTF-8 manifest + SHA-256
//! content-addressed blobs`。`bundle_id = SHA-256(canonical_manifest_bytes)`，不写回
//! manifest；`comparison_set_id` 也不进 bundle。manifest 只引用 blob 摘要 + 契约所需
//! 角色/媒体类型/字节数。
//!
//! 本模块实现：comparison_context_digest 的唯一构造式（§2.2）、bundle_id 计算、
//! 封存前不变量校验（§2.2 字段不变量 1–7 + §3 封存器职责）。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::digest;
use super::jcs;

/// ADR 0025 §2.2 的 `task` 块。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBlock {
    pub input_digest: String,
    pub spec_digest: String,
    pub gate_profile_digest: String,
}

/// ADR 0025 §2.2 的 `source` 块。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBlock {
    pub repository_digest: String,
    pub base_commit_digest: String,
    pub candidate_tree_digest: String,
    pub diff_digest: String,
}

/// ADR 0025 §2.2 的 `generation` 块（实际模型所有者/家族/精确 model_id，不得用别名）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationBlock {
    pub run_id: String,
    pub provider_id: String,
    pub owner_vendor: String,
    pub owner_family: String,
    pub model_id: String,
    pub prompt_digest: String,
    pub toolchain_digest: String,
    pub runtime_image_digest: String,
    pub dependency_lock_digest: String,
}

/// ADR 0025 §2.2 `gates[]` 元素。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRecord {
    pub gate_id: String,
    pub command_digest: String,
    pub status: String,
    pub exit_code: i64,
    pub stdout_blob: String,
    pub stderr_blob: String,
}

/// ADR 0025 §2.2 `artifacts[]` 元素（角色/媒体类型/字节数/SHA-256）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub role: String,
    pub media_type: String,
    pub bytes: u64,
    pub sha256: String,
}

/// ADR 0025 §2.2 的 `sealing` 块。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealingBlock {
    pub sealer_digest: String,
    pub seal_policy_digest: String,
    pub redaction_policy_digest: String,
}

/// ADR 0025 §2.2 的精确最小 manifest。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    pub schema_version: u64,
    pub candidate_id: String,
    pub task: TaskBlock,
    pub candidate_slot_policy_digest: String,
    pub comparison_context_digest: String,
    pub source: SourceBlock,
    pub generation: GenerationBlock,
    pub gates: Vec<GateRecord>,
    pub artifacts: Vec<ArtifactRecord>,
    pub ranking_view_digest: String,
    pub redaction_policy_digest: String,
    pub sealing: SealingBlock,
    pub state: String,
}

/// 封存器封出的 bundle：身份摘要 + 规范化 manifest。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub bundle_id: String,
    pub manifest: BundleManifest,
}

/// 封存/校验失败的错误（机器可判读，不吞不伪装）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// 摘要、schema 或同域条件不成立（ADR 0025 的 `INVALID_INPUT` 面）。
    Invalid(String),
    /// 封存器自身执行/存储故障（ADR 0025 的 `INFRA_FAILURE` 面）。
    Infra(String),
}

impl std::fmt::Display for SealError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SealError::Invalid(m) => write!(f, "INVALID_INPUT: {m}"),
            SealError::Infra(m) => write!(f, "INFRA_FAILURE: {m}"),
        }
    }
}

impl std::error::Error for SealError {}

/// `comparison_context_digest` 的唯一构造式（ADR 0025 §2.2）：对「三个 task 摘要 +
/// `candidate_slot_policy_digest`」的 JCS UTF-8 bytes 求 SHA-256。任何 set 身份、成员
/// 顺序、bundle/view/set ID 都不得进入此式。
pub fn comparison_context_digest(task: &TaskBlock, candidate_slot_policy_digest: &str) -> String {
    let v: Value = json!({
        "task": {
            "input_digest": task.input_digest,
            "spec_digest": task.spec_digest,
            "gate_profile_digest": task.gate_profile_digest,
        },
        "candidate_slot_policy_digest": candidate_slot_policy_digest,
    });
    digest::sha256_hex(&jcs::canonical_bytes(&v))
}

/// 把 manifest 规范化成 JSON 值（用于 JCS）。
pub fn manifest_to_value(manifest: &BundleManifest) -> Result<Value, SealError> {
    serde_json::to_value(manifest).map_err(|e| SealError::Infra(e.to_string()))
}

/// `bundle_id = SHA-256(canonical_manifest_bytes)`。
pub fn bundle_id(manifest: &BundleManifest) -> Result<String, SealError> {
    let bytes = canonical_manifest_bytes(manifest)?;
    Ok(digest::sha256_hex(&bytes))
}

/// manifest 的 JCS 规范化 UTF-8 bytes。
pub fn canonical_manifest_bytes(manifest: &BundleManifest) -> Result<Vec<u8>, SealError> {
    Ok(jcs::canonical_bytes(&manifest_to_value(manifest)?))
}

/// 封存器职责里的不变量校验（ADR 0025 §2.2 字段不变量 + §3.1/§4.1）。
///
/// 返回 `Ok(())` 表示可封成 `SEALED`；返回 `SealError::Invalid` 表示同域/摘要/绑定不成立。
pub fn validate_bundle(manifest: &BundleManifest) -> Result<(), SealError> {
    // 不变量 1：schema 固定解释。
    if manifest.schema_version != 1 {
        return Err(SealError::Invalid(format!(
            "schema_version must be 1, got {}",
            manifest.schema_version
        )));
    }
    if manifest.state != "SEALED" {
        return Err(SealError::Invalid(format!(
            "state must be SEALED, got {}",
            manifest.state
        )));
    }
    // 不变量 2：candidate_id 是不透明槽位标识，须非空。
    if manifest.candidate_id.trim().is_empty() {
        return Err(SealError::Invalid("candidate_id must be non-empty".into()));
    }
    // 不变量 6：顶层与 sealing 内的 redaction_policy_digest 逐字相同。
    if manifest.redaction_policy_digest != manifest.sealing.redaction_policy_digest {
        return Err(SealError::Invalid(
            "top-level and sealing.redaction_policy_digest must be identical".into(),
        ));
    }
    // 不变量 3：comparison_context_digest 必须严格按唯一构造式重算一致。
    let recomputed =
        comparison_context_digest(&manifest.task, &manifest.candidate_slot_policy_digest);
    if recomputed != manifest.comparison_context_digest {
        return Err(SealError::Invalid(format!(
            "comparison_context_digest mismatch: manifest {} != recomputed {}",
            manifest.comparison_context_digest, recomputed
        )));
    }
    // 不变量 5 + §4.1：Gate Profile 每个 required gate 必须 PASS；被引用 blob 须能在
    // artifacts 中按角色对上，且 ranking_view_digest 必须落到 role="ranking_view" 的 artifact。
    if manifest.gates.iter().any(|g| g.status != "PASS") {
        return Err(SealError::Invalid(
            "all gates must be PASS for a SEALED bundle".into(),
        ));
    }
    validate_artifact_references(manifest)?;
    Ok(())
}

/// 校验 artifacts 与 blob 引用的对应关系（§2.2 不变量 5）。
fn validate_artifact_references(manifest: &BundleManifest) -> Result<(), SealError> {
    let mut ranking_view_matched = false;
    for artifact in &manifest.artifacts {
        if artifact.sha256.is_empty() || artifact.media_type.is_empty() {
            return Err(SealError::Invalid(
                "artifact sha256/media_type required".into(),
            ));
        }
        if artifact.role == "ranking_view" {
            if artifact.sha256 != manifest.ranking_view_digest {
                return Err(SealError::Invalid(
                    "ranking_view artifact sha256 must equal ranking_view_digest".into(),
                ));
            }
            ranking_view_matched = true;
        }
    }
    if !ranking_view_matched {
        return Err(SealError::Invalid(
            "artifacts must contain a role=ranking_view artifact matching ranking_view_digest"
                .into(),
        ));
    }
    // gate stdout/stderr blob 摘要必须在 artifacts 中也能对上（§2.2 不变量 5）。
    let known: std::collections::HashSet<&str> = manifest
        .artifacts
        .iter()
        .map(|a| a.sha256.as_str())
        .collect();
    for gate in &manifest.gates {
        if !known.contains(gate.stdout_blob.as_str()) {
            return Err(SealError::Invalid(format!(
                "gate {} stdout_blob not present in artifacts",
                gate.gate_id
            )));
        }
        if !known.contains(gate.stderr_blob.as_str()) {
            return Err(SealError::Invalid(format!(
                "gate {} stderr_blob not present in artifacts",
                gate.gate_id
            )));
        }
    }
    Ok(())
}

/// 按 ADR 0025 §3 的职责封存一个 bundle：重算 comparison_context_digest → 校验 →
/// 算 bundle_id。封存器不调用模型、不解释测试结果，只做确定性校验。
pub fn seal_bundle(manifest: BundleManifest) -> Result<Bundle, SealError> {
    validate_bundle(&manifest)?;
    let id = bundle_id(&manifest)?;
    Ok(Bundle {
        bundle_id: id,
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> TaskBlock {
        TaskBlock {
            input_digest: digest::sha256_hex(b"task input"),
            spec_digest: digest::sha256_hex(b"spec"),
            gate_profile_digest: digest::sha256_hex(b"gate profile"),
        }
    }

    fn sample_manifest() -> BundleManifest {
        let task = sample_task();
        let slot = digest::sha256_hex(b"slot policy v1");
        let ctx = comparison_context_digest(&task, &slot);
        let redaction = digest::sha256_hex(b"redaction policy v1");
        let sealer = digest::sha256_hex(b"lav-seal-rs v1");
        let seal_policy = digest::sha256_hex(b"seal policy v1");
        let empty = digest::sha256_hex(b"");
        let view = digest::sha256_hex(b"redacted ranking view");

        BundleManifest {
            schema_version: 1,
            candidate_id: "cand-A".into(),
            task,
            candidate_slot_policy_digest: slot,
            comparison_context_digest: ctx,
            source: SourceBlock {
                repository_digest: digest::sha256_hex(b"repo"),
                base_commit_digest: digest::sha256_hex(b"base"),
                candidate_tree_digest: digest::sha256_hex(b"tree"),
                diff_digest: digest::sha256_hex(b"diff"),
            },
            generation: GenerationBlock {
                run_id: "run-1".into(),
                provider_id: "deepseek".into(),
                owner_vendor: "deepseek".into(),
                owner_family: "deepseek-v4".into(),
                model_id: "deepseek-v4-pro".into(),
                prompt_digest: digest::sha256_hex(b"prompt"),
                toolchain_digest: digest::sha256_hex(b"toolchain"),
                runtime_image_digest: digest::sha256_hex(b"runtime"),
                dependency_lock_digest: digest::sha256_hex(b"lock"),
            },
            gates: vec![GateRecord {
                gate_id: "g1".into(),
                command_digest: digest::sha256_hex(b"cargo test"),
                status: "PASS".into(),
                exit_code: 0,
                stdout_blob: empty.clone(),
                stderr_blob: empty.clone(),
            }],
            artifacts: vec![
                ArtifactRecord {
                    role: "evidence".into(),
                    media_type: "text/markdown".into(),
                    bytes: 10,
                    sha256: digest::sha256_hex(b"0123456789"),
                },
                ArtifactRecord {
                    role: "ranking_view".into(),
                    media_type: "text/markdown".into(),
                    bytes: view.len() as u64,
                    sha256: view.clone(),
                },
                ArtifactRecord {
                    role: "stdout".into(),
                    media_type: "text/plain".into(),
                    bytes: 0,
                    sha256: empty.clone(),
                },
                ArtifactRecord {
                    role: "stderr".into(),
                    media_type: "text/plain".into(),
                    bytes: 0,
                    sha256: empty.clone(),
                },
            ],
            ranking_view_digest: view,
            redaction_policy_digest: redaction.clone(),
            sealing: SealingBlock {
                sealer_digest: sealer,
                seal_policy_digest: seal_policy,
                redaction_policy_digest: redaction,
            },
            state: "SEALED".into(),
        }
    }

    #[test]
    fn comparison_context_digest_is_exactly_four_inputs() {
        let task = sample_task();
        let slot = "s".to_string();
        // 与手工构造的 JCS bytes 一致：只含 task 三摘要 + slot。
        let expected_value = json!({
            "task": {
                "input_digest": task.input_digest,
                "spec_digest": task.spec_digest,
                "gate_profile_digest": task.gate_profile_digest,
            },
            "candidate_slot_policy_digest": slot,
        });
        let expected = digest::sha256_hex(&jcs::canonical_bytes(&expected_value));
        assert_eq!(comparison_context_digest(&task, &slot), expected);
    }

    #[test]
    fn seal_valid_bundle_yields_stable_id() {
        let manifest = sample_manifest();
        let bundle = seal_bundle(manifest.clone()).unwrap();
        // bundle_id 稳定：同 manifest 两次封存得到同一身份。
        let again = seal_bundle(manifest.clone()).unwrap();
        assert_eq!(bundle.bundle_id, again.bundle_id);
        // bundle_id == SHA-256(canonical bytes)。
        let canonical = canonical_manifest_bytes(&manifest).unwrap();
        assert_eq!(bundle.bundle_id, digest::sha256_hex(&canonical));
        // bundle_id 不等于 ranking_view_digest / candidate_id（身份图单向）。
        assert_ne!(bundle.bundle_id, manifest.ranking_view_digest);
        assert_ne!(bundle.bundle_id, manifest.candidate_id);
    }

    #[test]
    fn rejects_redaction_policy_mismatch() {
        let mut manifest = sample_manifest();
        manifest.redaction_policy_digest = digest::sha256_hex(b"other");
        assert!(matches!(seal_bundle(manifest), Err(SealError::Invalid(_))));
    }

    #[test]
    fn rejects_bad_comparison_context() {
        let mut manifest = sample_manifest();
        manifest.comparison_context_digest = digest::sha256_hex(b"tampered");
        assert!(matches!(seal_bundle(manifest), Err(SealError::Invalid(_))));
    }

    #[test]
    fn rejects_non_sealed_state() {
        let mut manifest = sample_manifest();
        manifest.state = "DRAFT".into();
        assert!(matches!(seal_bundle(manifest), Err(SealError::Invalid(_))));
    }

    #[test]
    fn rejects_failed_gate() {
        let mut manifest = sample_manifest();
        manifest.gates[0].status = "FAIL".into();
        assert!(matches!(seal_bundle(manifest), Err(SealError::Invalid(_))));
    }
}
