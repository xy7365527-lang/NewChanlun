//! 固定历史样本集（S1 #1167）与封存 driver。
//!
//! 样本构造规则**复用** #1162 v2 实证（GO 73.3%）的构造规则，不重造：
//! 「同任务『初版 vs 终审订正』互斥对」——候选是同一任务、同一判定轴上的先后评审产物，
//! 二者互斥竞争同一个判定，杜绝 v1 的「非互斥候选 + tie + 位置偏差」脏读数。
//!
//! 三类各 10（#1149 裁定「code review / CI 红绿 / Lead 终态」三类等权）：
//! - `Review`：同一 ticket+轴 的评审报告「初版 vs 终审订正」（`-final`／`recheck`／`reviewN` 递进）；
//! - `Lead`：Lead 终态实现报告「初版 vs fixround/impl-b/retest-round2」递进；
//! - `Ci`：CI 红→绿修复对（git log 的 `fix(ci)`/`#CI修` 提交 vs 其父提交，红=失败态、绿=修复态）。
//!
//! ground truth 全部 held-out（不进 verifier 输入），只做事后指标分母。`held_out` 一律
//! 指向「终审订正」一方。A/B 槽随机化与 seed=42 属于 S4（排序 run）的输入顺序随机化，
//! 不在这里的封存面（封存只钉死 opaque `candidate_id` 与 ground truth）。
//!
//! ## 方法学偏差登记（照 #1162 先例，如实）
//!
//! 1. #1162 v2 的 harness 骨架与 30 样本实例在 pilot 收口时按 #1149「即停即删 sidecar」
//!    已删除，本处 30 样本是按票面构造规则（互斥对）对仓内历史评审产物 + git log 的
//!    **确定性重建**，与已删实例不必逐字相同。
//! 2. 历史评审报告不记录其生成模型/provider ⟹ bundle `generation` 的
//!    `provider_id/owner_vendor/owner_family/model_id` 记 `unrecorded`，不伪造实际
//!    model_id（ADR 0025 §7.6 禁伪造；此缺口属 pilot 历史样本固有，随本次封存照实登记）。
//! 3. CI 类的候选证据 = 提交消息（红/绿态描述），是 git log 代理，非完整 CI run 输出。

use std::path::Path;

use super::bundle::{
    comparison_context_digest, seal_bundle, ArtifactRecord, Bundle, BundleManifest, GateRecord,
    GenerationBlock, SealError, SealingBlock, SourceBlock, TaskBlock,
};
use super::comparison_set::{seal_comparison_set, ComparisonSet, SetError};
use super::digest;
use super::jcs;
use super::lead_baseline::{seal_baseline, selected, BaselineError, SealedBaseline};
use serde_json::json;

/// 三类样本（#1149：code review / CI 红绿 / Lead 终态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleClass {
    Review,
    Lead,
    Ci,
}

impl SampleClass {
    pub fn as_str(self) -> &'static str {
        match self {
            SampleClass::Review => "review",
            SampleClass::Lead => "lead",
            SampleClass::Ci => "ci",
        }
    }
}

/// 候选证据来源：仓内文件（相对路径）或内联内容（CI 提交消息，封存时冻结在 spec 里）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    File(&'static str),
    Inline(&'static str),
}

/// 单个候选的封存规格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateSpec {
    pub candidate_id: &'static str,
    pub source: EvidenceSource,
}

/// 一个样本（problem, N candidates, held_out 终审标签）三元组（#1149）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleSpec {
    pub sample_id: &'static str,
    pub class: SampleClass,
    pub task_label: &'static str,
    pub candidates: &'static [CandidateSpec],
    pub held_out: &'static str,
}

// ── 封存政策常量（身份机制的一部分，digest 绑定到 manifest） ──

/// 脱敏政策（ranking_view 只发布此政策允许的 canonical projection）。
const REDACTION_POLICY: &str = "lav-redaction-v1: strip absolute paths (/tmp /home /Users /root /var /etc /workspace /private) and http(s) URLs; keep relative code references";
/// 封存器身份（确定性实现）。
const SEALER_ID: &str = "newchan-lav-seal-rs/v1 (#1167 S1)";
/// 封存规则。
const SEAL_POLICY: &str =
    "lav-seal-v1: JCS manifest + SHA-256 content-addressed blobs, atomic SEALED publish";
/// 候选槽位预分配政策（opaque A/B 槽，不携带质量/顺序）。
const SLOT_POLICY: &str = "lav-slot-v1: two opaque candidate slots per historical sample, assigned A/B, no quality/order signal";
/// Gate Profile：历史样本唯一的 required gate 是「已终审」。
const GATE_PROFILE_POLICY: &str =
    "lav-gate-v1: single required gate 'historical-finalized' must PASS";
/// 仓库身份（source.repository_digest 的输入）。
const REPO_ID: &str = "xy7365527-lang/NewChanlun";
/// 历史样本 run_id（本次封存的确定性 run 标识）。
const RUN_ID: &str = "hist-samples-v1";
/// 历史样本未记录字段的诚实哨兵。
const UNRECORDED: &str = "unrecorded";

fn unrecorded_digest() -> String {
    digest::sha256_hex(UNRECORDED.as_bytes())
}

fn empty_digest() -> String {
    digest::sha256_hex(b"")
}

/// 脱敏：把绝对路径与 URL 替换为 `[REDACTED]`（只删操作面，保留相对代码引用）。
pub fn redact(content: &str) -> String {
    const PREFIXES: [&str; 9] = [
        "https://",
        "http://",
        "/tmp/",
        "/home/",
        "/Users/",
        "/root/",
        "/var/",
        "/etc/",
        "/workspace/",
    ];
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    loop {
        let hit = PREFIXES
            .iter()
            .filter_map(|p| rest.find(p).map(|idx| (idx, *p)))
            .min_by_key(|(idx, _)| *idx);
        match hit {
            None => {
                out.push_str(rest);
                break;
            }
            Some((idx, prefix)) => {
                out.push_str(&rest[..idx]);
                out.push_str("[REDACTED]");
                let after = &rest[idx + prefix.len()..];
                let token_end = after.find(char::is_whitespace).unwrap_or(after.len());
                rest = &after[token_end..];
            }
        }
    }
    out
}

/// 解析候选证据内容。
pub fn resolve_evidence(source: &EvidenceSource, repo_root: &Path) -> Result<String, SampleError> {
    match source {
        EvidenceSource::File(path) => {
            let full = repo_root.join(path);
            std::fs::read_to_string(&full)
                .map_err(|e| SampleError::MissingEvidence(format!("{}: {e}", full.display())))
        }
        EvidenceSource::Inline(content) => Ok(content.to_string()),
    }
}

fn media_type_for(source: &EvidenceSource) -> &'static str {
    match source {
        EvidenceSource::File(p) if p.ends_with(".md") => "text/markdown",
        EvidenceSource::File(_) => "text/plain",
        EvidenceSource::Inline(_) => "text/plain",
    }
}

/// 为一个候选构造并封存 bundle（历史样本 provenance 政策）。
fn seal_candidate_bundle(
    sample: &SampleSpec,
    candidate: &CandidateSpec,
    raw: &str,
) -> Result<Bundle, SealError> {
    // ranking_view 必须是 JCS UTF-8 JSON，且含一个且仅一个顶层 opaque candidate_id（ADR 0025 §5）。
    let view_text = redact(raw);
    let view_value = json!({
        "candidate_id": candidate.candidate_id,
        "evidence_text": view_text,
    });
    let view_bytes = jcs::canonical_bytes(&view_value);
    let raw_digest = digest::sha256_hex(raw.as_bytes());
    let view_digest = digest::sha256_hex(&view_bytes);
    let task = TaskBlock {
        input_digest: digest::sha256_hex(format!("hist:{}", sample.task_label).as_bytes()),
        spec_digest: digest::sha256_hex(format!("{}:spec", sample.task_label).as_bytes()),
        gate_profile_digest: digest::sha256_hex(GATE_PROFILE_POLICY.as_bytes()),
    };
    let slot = digest::sha256_hex(SLOT_POLICY.as_bytes());
    let ctx = comparison_context_digest(&task, &slot);
    let redaction = digest::sha256_hex(REDACTION_POLICY.as_bytes());
    let empty = empty_digest();
    let un = unrecorded_digest();

    let manifest = BundleManifest {
        schema_version: 1,
        candidate_id: candidate.candidate_id.to_string(),
        task,
        candidate_slot_policy_digest: slot,
        comparison_context_digest: ctx,
        source: SourceBlock {
            repository_digest: digest::sha256_hex(REPO_ID.as_bytes()),
            base_commit_digest: un.clone(),
            candidate_tree_digest: un.clone(),
            diff_digest: un.clone(),
        },
        generation: GenerationBlock {
            run_id: RUN_ID.to_string(),
            provider_id: UNRECORDED.to_string(),
            owner_vendor: UNRECORDED.to_string(),
            owner_family: UNRECORDED.to_string(),
            model_id: UNRECORDED.to_string(),
            prompt_digest: un.clone(),
            toolchain_digest: un.clone(),
            runtime_image_digest: un.clone(),
            dependency_lock_digest: un,
        },
        gates: vec![GateRecord {
            gate_id: "historical-finalized".into(),
            command_digest: digest::sha256_hex(b"historical-finalized"),
            status: "PASS".into(),
            exit_code: 0,
            stdout_blob: empty.clone(),
            stderr_blob: empty.clone(),
        }],
        artifacts: vec![
            ArtifactRecord {
                role: "evidence".into(),
                media_type: media_type_for(&candidate.source).into(),
                bytes: raw.len() as u64,
                sha256: raw_digest,
            },
            ArtifactRecord {
                role: "ranking_view".into(),
                media_type: "application/json".into(),
                bytes: view_bytes.len() as u64,
                sha256: view_digest.clone(),
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
        ranking_view_digest: view_digest,
        redaction_policy_digest: redaction.clone(),
        sealing: SealingBlock {
            sealer_digest: digest::sha256_hex(SEALER_ID.as_bytes()),
            seal_policy_digest: digest::sha256_hex(SEAL_POLICY.as_bytes()),
            redaction_policy_digest: redaction,
        },
        state: "SEALED".into(),
    };
    seal_bundle(manifest)
}

/// 封存后的单个样本：bundle + Comparison Set + Lead 基线（先封存后揭示）。
#[derive(Debug, Clone)]
pub struct SealedSample {
    pub sample_id: &'static str,
    pub class: SampleClass,
    pub bundles: Vec<Bundle>,
    pub set: ComparisonSet,
    pub baseline: SealedBaseline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleError {
    MissingEvidence(String),
    Seal(String),
    Set(String),
    Baseline(String),
}

impl std::fmt::Display for SampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleError::MissingEvidence(m) => write!(f, "MISSING_EVIDENCE: {m}"),
            SampleError::Seal(m) => write!(f, "SEAL: {m}"),
            SampleError::Set(m) => write!(f, "SET: {m}"),
            SampleError::Baseline(m) => write!(f, "BASELINE: {m}"),
        }
    }
}

impl std::error::Error for SampleError {}

impl From<SealError> for SampleError {
    fn from(e: SealError) -> Self {
        SampleError::Seal(e.to_string())
    }
}

impl From<SetError> for SampleError {
    fn from(e: SetError) -> Self {
        SampleError::Set(e.to_string())
    }
}

impl From<BaselineError> for SampleError {
    fn from(e: BaselineError) -> Self {
        SampleError::Baseline(e.to_string())
    }
}

/// 封存一个样本：候选 bundle → Comparison Set → Lead 基线（先封存）。
pub fn seal_sample(spec: &SampleSpec, repo_root: &Path) -> Result<SealedSample, SampleError> {
    let mut bundles = Vec::with_capacity(spec.candidates.len());
    for candidate in spec.candidates {
        let raw = resolve_evidence(&candidate.source, repo_root)?;
        bundles.push(seal_candidate_bundle(spec, candidate, &raw)?);
    }
    let set_policy = digest::sha256_hex(
        format!(
            "lav-set-v1: membership ordered by spec, {} members",
            bundles.len()
        )
        .as_bytes(),
    );
    let set = seal_comparison_set(&set_policy, &bundles)?;
    let reason = format!("{}: 终审订正候选为 held_out", spec.sample_id);
    let evidence_refs: Vec<String> = bundles.iter().map(|b| b.bundle_id.clone()).collect();
    let baseline = seal_baseline(selected(
        set.comparison_set_id.clone(),
        spec.held_out.to_string(),
        reason,
        evidence_refs,
    ))?;
    Ok(SealedSample {
        sample_id: spec.sample_id,
        class: spec.class,
        bundles,
        set,
        baseline,
    })
}

/// 封存全部 30 样本。
pub fn seal_all(repo_root: &Path) -> Result<Vec<SealedSample>, SampleError> {
    SAMPLES.iter().map(|s| seal_sample(s, repo_root)).collect()
}

/// 校验样本规格的静态不变量（每类 10、每样本 N=2、held_out 在候选内且唯一）。
pub fn validate_spec(samples: &[SampleSpec]) -> Result<(), String> {
    for class in [SampleClass::Review, SampleClass::Lead, SampleClass::Ci] {
        let count = samples.iter().filter(|s| s.class == class).count();
        if count != 10 {
            return Err(format!(
                "class {:?} must have 10 samples, got {}",
                class, count
            ));
        }
    }
    for s in samples {
        if s.candidates.len() < 2 {
            return Err(format!("{}: N must be >= 2", s.sample_id));
        }
        if s.candidates.len() > 4 {
            return Err(format!("{}: N must be <= 4", s.sample_id));
        }
        let mut held_count = 0;
        let mut ids = std::collections::HashSet::new();
        for c in s.candidates {
            if !ids.insert(c.candidate_id) {
                return Err(format!(
                    "{}: duplicate candidate_id {}",
                    s.sample_id, c.candidate_id
                ));
            }
            if c.candidate_id == s.held_out {
                held_count += 1;
            }
        }
        if held_count != 1 {
            return Err(format!(
                "{}: held_out must name exactly one candidate, got {}",
                s.sample_id, held_count
            ));
        }
    }
    Ok(())
}

/// 固定 30 样本（三类各 10 × N=2），#1162 v2 互斥对构造规则。
pub static SAMPLES: &[SampleSpec] = &[
    SampleSpec {
        sample_id: "S001",
        class: SampleClass::Review,
        task_label: "issue641-spec-review",
        candidates: &[
            CandidateSpec { candidate_id: "S001-A", source: EvidenceSource::File("chanlun/review-results/code-review-issue641-spec-20260729.md") },
            CandidateSpec { candidate_id: "S001-B", source: EvidenceSource::File("chanlun/review-results/code-review-issue641-spec-final-20260729.md") },
        ],
        held_out: "S001-B",
    },
    SampleSpec {
        sample_id: "S002",
        class: SampleClass::Review,
        task_label: "issue641-standards-review",
        candidates: &[
            CandidateSpec { candidate_id: "S002-A", source: EvidenceSource::File("chanlun/review-results/code-review-issue641-standards-20260729.md") },
            CandidateSpec { candidate_id: "S002-B", source: EvidenceSource::File("chanlun/review-results/code-review-issue641-standards-final-20260729.md") },
        ],
        held_out: "S002-B",
    },
    SampleSpec {
        sample_id: "S003",
        class: SampleClass::Review,
        task_label: "shadow554-spec-review",
        candidates: &[
            CandidateSpec { candidate_id: "S003-A", source: EvidenceSource::File("chanlun/review-results/shadow-554-spec-20260728.md") },
            CandidateSpec { candidate_id: "S003-B", source: EvidenceSource::File("chanlun/review-results/shadow-554-spec-final-20260728.md") },
        ],
        held_out: "S003-B",
    },
    SampleSpec {
        sample_id: "S004",
        class: SampleClass::Review,
        task_label: "shadow554-standards-review",
        candidates: &[
            CandidateSpec { candidate_id: "S004-A", source: EvidenceSource::File("chanlun/review-results/shadow-554-standards-20260728.md") },
            CandidateSpec { candidate_id: "S004-B", source: EvidenceSource::File("chanlun/review-results/shadow-554-standards-final-20260728.md") },
        ],
        held_out: "S004-B",
    },
    SampleSpec {
        sample_id: "S005",
        class: SampleClass::Review,
        task_label: "shadow475-recheck",
        candidates: &[
            CandidateSpec { candidate_id: "S005-A", source: EvidenceSource::File("chanlun/review-results/shadow-review-475-recheck-20260727.md") },
            CandidateSpec { candidate_id: "S005-B", source: EvidenceSource::File("chanlun/review-results/shadow-review-475-recheck2-20260727.md") },
        ],
        held_out: "S005-B",
    },
    SampleSpec {
        sample_id: "S006",
        class: SampleClass::Review,
        task_label: "shadow481-recheck",
        candidates: &[
            CandidateSpec { candidate_id: "S006-A", source: EvidenceSource::File("chanlun/review-results/shadow-review-481-recheck-20260727.md") },
            CandidateSpec { candidate_id: "S006-B", source: EvidenceSource::File("chanlun/review-results/shadow-review-481-recheck2-20260727.md") },
        ],
        held_out: "S006-B",
    },
    SampleSpec {
        sample_id: "S007",
        class: SampleClass::Review,
        task_label: "shadow511-recheck",
        candidates: &[
            CandidateSpec { candidate_id: "S007-A", source: EvidenceSource::File("chanlun/review-results/shadow-review-511-recheck-20260728.md") },
            CandidateSpec { candidate_id: "S007-B", source: EvidenceSource::File("chanlun/review-results/shadow-review-511-recheck2-20260728.md") },
        ],
        held_out: "S007-B",
    },
    SampleSpec {
        sample_id: "S008",
        class: SampleClass::Review,
        task_label: "shadow668-review",
        candidates: &[
            CandidateSpec { candidate_id: "S008-A", source: EvidenceSource::File("chanlun/review-results/shadow-668-review-20260729.md") },
            CandidateSpec { candidate_id: "S008-B", source: EvidenceSource::File("chanlun/review-results/shadow-668-review4-final-20260729.md") },
        ],
        held_out: "S008-B",
    },
    SampleSpec {
        sample_id: "S009",
        class: SampleClass::Review,
        task_label: "shadow429-rereview",
        candidates: &[
            CandidateSpec { candidate_id: "S009-A", source: EvidenceSource::File("chanlun/review-results/shadow-429-issue421-20260727.md") },
            CandidateSpec { candidate_id: "S009-B", source: EvidenceSource::File("chanlun/review-results/shadow-429-rereview-20260728.md") },
        ],
        held_out: "S009-B",
    },
    SampleSpec {
        sample_id: "S010",
        class: SampleClass::Review,
        task_label: "shadow430-rereview",
        candidates: &[
            CandidateSpec { candidate_id: "S010-A", source: EvidenceSource::File("chanlun/review-results/shadow-430-issue421-20260727.md") },
            CandidateSpec { candidate_id: "S010-B", source: EvidenceSource::File("chanlun/review-results/shadow-430-rereview-20260728.md") },
        ],
        held_out: "S010-B",
    },
    SampleSpec {
        sample_id: "S011",
        class: SampleClass::Lead,
        task_label: "issue573-t1-kernel",
        candidates: &[
            CandidateSpec { candidate_id: "S011-A", source: EvidenceSource::File("chanlun/review-results/issue573-t1-kernel-expand-impl-20260728.md") },
            CandidateSpec { candidate_id: "S011-B", source: EvidenceSource::File("chanlun/review-results/issue573-t1-kernel-fixround-impl-20260728.md") },
        ],
        held_out: "S011-B",
    },
    SampleSpec {
        sample_id: "S012",
        class: SampleClass::Lead,
        task_label: "issue622-s2",
        candidates: &[
            CandidateSpec { candidate_id: "S012-A", source: EvidenceSource::File("chanlun/review-results/issue622-s2-impl-20260729.md") },
            CandidateSpec { candidate_id: "S012-B", source: EvidenceSource::File("chanlun/review-results/issue622-s2-fixround-impl-20260729.md") },
        ],
        held_out: "S012-B",
    },
    SampleSpec {
        sample_id: "S013",
        class: SampleClass::Lead,
        task_label: "issue623-s3",
        candidates: &[
            CandidateSpec { candidate_id: "S013-A", source: EvidenceSource::File("chanlun/review-results/issue623-s3-portal-impl-20260729.md") },
            CandidateSpec { candidate_id: "S013-B", source: EvidenceSource::File("chanlun/review-results/issue623-s3-fixround-impl-20260729.md") },
        ],
        held_out: "S013-B",
    },
    SampleSpec {
        sample_id: "S014",
        class: SampleClass::Lead,
        task_label: "issue668-n4-impl",
        candidates: &[
            CandidateSpec { candidate_id: "S014-A", source: EvidenceSource::File("chanlun/review-results/issue668-n4-impl-ticket668-20260729.md") },
            CandidateSpec { candidate_id: "S014-B", source: EvidenceSource::File("chanlun/review-results/issue668-n4-impl-ticket668b-20260729.md") },
        ],
        held_out: "S014-B",
    },
    SampleSpec {
        sample_id: "S015",
        class: SampleClass::Lead,
        task_label: "issue965-impl",
        candidates: &[
            CandidateSpec { candidate_id: "S015-A", source: EvidenceSource::File("chanlun/review-results/issue965-impl-20260814.md") },
            CandidateSpec { candidate_id: "S015-B", source: EvidenceSource::File("chanlun/review-results/issue965-impl-b-20260814.md") },
        ],
        held_out: "S015-B",
    },
    SampleSpec {
        sample_id: "S016",
        class: SampleClass::Lead,
        task_label: "issue870-retest",
        candidates: &[
            CandidateSpec { candidate_id: "S016-A", source: EvidenceSource::File("chanlun/review-results/issue870-retest-20260816.md") },
            CandidateSpec { candidate_id: "S016-B", source: EvidenceSource::File("chanlun/review-results/issue870-retest-round2-20260818.md") },
        ],
        held_out: "S016-B",
    },
    SampleSpec {
        sample_id: "S017",
        class: SampleClass::Lead,
        task_label: "issue641-n3-chain",
        candidates: &[
            CandidateSpec { candidate_id: "S017-A", source: EvidenceSource::File("chanlun/review-results/issue641-n3-chain-impl-20260729.md") },
            CandidateSpec { candidate_id: "S017-B", source: EvidenceSource::File("chanlun/review-results/issue641-n3-chain-impl-ticket641b-20260729.md") },
        ],
        held_out: "S017-B",
    },
    SampleSpec {
        sample_id: "S018",
        class: SampleClass::Lead,
        task_label: "issue668-n4-fix-round1-2",
        candidates: &[
            CandidateSpec { candidate_id: "S018-A", source: EvidenceSource::File("chanlun/review-results/issue668-n4-fix-round1-20260729.md") },
            CandidateSpec { candidate_id: "S018-B", source: EvidenceSource::File("chanlun/review-results/issue668-n4-fix-round2-20260729.md") },
        ],
        held_out: "S018-B",
    },
    SampleSpec {
        sample_id: "S019",
        class: SampleClass::Lead,
        task_label: "issue668-n4-fix-round2-3",
        candidates: &[
            CandidateSpec { candidate_id: "S019-A", source: EvidenceSource::File("chanlun/review-results/issue668-n4-fix-round2-20260729.md") },
            CandidateSpec { candidate_id: "S019-B", source: EvidenceSource::File("chanlun/review-results/issue668-n4-fix-round3-20260729.md") },
        ],
        held_out: "S019-B",
    },
    SampleSpec {
        sample_id: "S020",
        class: SampleClass::Lead,
        task_label: "issue573-t1-kernel-migrate",
        candidates: &[
            CandidateSpec { candidate_id: "S020-A", source: EvidenceSource::File("chanlun/review-results/issue573-t1-kernel-fixround-impl-20260728.md") },
            CandidateSpec { candidate_id: "S020-B", source: EvidenceSource::File("chanlun/review-results/issue573-t1-kernel-migrate-impl-20260728.md") },
        ],
        held_out: "S020-B",
    },
    SampleSpec {
        sample_id: "S021",
        class: SampleClass::Ci,
        task_label: "ci-1022-twstepctx",
        candidates: &[
            CandidateSpec { candidate_id: "S021-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 fix(backtest): #1011 评审 C1——opsem_dump 三处 centers.first()→last()") },
            CandidateSpec { candidate_id: "S021-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 25e7a43798 hotfix(ci): #1022 TwStepCtx re-export 门修 cherry 至 main 尖——CI 连续两轮 rust-check 红（run 31965652414 同因）") },
        ],
        held_out: "S021-B",
    },
    SampleSpec {
        sample_id: "S022",
        class: SampleClass::Ci,
        task_label: "ci-903-bsp-whitelist",
        candidates: &[
            CandidateSpec { candidate_id: "S022-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 refactor(econ): #399 截窗骨架八点收敛——共享 helper + 边界算术三态单测") },
            CandidateSpec { candidate_id: "S022-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 c9fa040dd3 fix(ci): bsp 白名单七夹具点行号重登记——#903 注释 +2 行位移") },
        ],
        held_out: "S022-B",
    },
    SampleSpec {
        sample_id: "S023",
        class: SampleClass::Ci,
        task_label: "ci-1141-disk-budget",
        candidates: &[
            CandidateSpec { candidate_id: "S023-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 fix(security): use HTTPS for arXiv API (#1118)") },
            CandidateSpec { candidate_id: "S023-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 d45c26a9dc fix(ci): keep rust-check within runner disk budget (#1141)") },
        ],
        held_out: "S023-B",
    },
    SampleSpec {
        sample_id: "S024",
        class: SampleClass::Ci,
        task_label: "ci-898-fixture-amplitude",
        candidates: &[
            CandidateSpec { candidate_id: "S024-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 test(theta_v0): #CI修——incremental_macd O(1) 标度锁 min-of-3 降噪") },
            CandidateSpec { candidate_id: "S024-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 ba987be5ba test(equivalence): #CI修——合成夹具慢趋势振幅 40→60（CI 2 红根因）") },
        ],
        held_out: "S024-B",
    },
    SampleSpec {
        sample_id: "S025",
        class: SampleClass::Ci,
        task_label: "ci-incremental-macd-scale",
        candidates: &[
            CandidateSpec { candidate_id: "S025-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 docs(review): #735——L69 A7 判据「裁量分离」挂账语义查明") },
            CandidateSpec { candidate_id: "S025-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 acc721de02 test(theta_v0): #CI修——incremental_macd O(1) 标度锁 min-of-3 降噪（CI 慢机单轮抖动 3.73x 误触 3.0 阈值）") },
        ],
        held_out: "S025-B",
    },
    SampleSpec {
        sample_id: "S026",
        class: SampleClass::Ci,
        task_label: "ci-probe-empty-commit",
        candidates: &[
            CandidateSpec { candidate_id: "S026-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 docs(adr): ADR 0024 总账口径——净额验收/毛账归因两套并记") },
            CandidateSpec { candidate_id: "S026-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 ec59fe1424 ci: 探针——空提交重触发 CI（9563ee23 三 job 零步骤全红）") },
        ],
        held_out: "S026-B",
    },
    SampleSpec {
        sample_id: "S027",
        class: SampleClass::Ci,
        task_label: "ci-probe2-startup",
        candidates: &[
            CandidateSpec { candidate_id: "S027-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 Merge branch 'issue-871-lean-soundness'") },
            CandidateSpec { candidate_id: "S027-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 f0e6391ea9 ci: 探针2——GitHub Actions startup 故障复检（空提交）") },
        ],
        held_out: "S027-B",
    },
    SampleSpec {
        sample_id: "S028",
        class: SampleClass::Ci,
        task_label: "ci-990-whitelist-skip",
        candidates: &[
            CandidateSpec { candidate_id: "S028-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 style(rust): cargo fmt 归零（I-1 测试注释对齐回潮）") },
            CandidateSpec { candidate_id: "S028-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 3215976776 fix(ci): 白名单八夹具点行号重登记（#990 接线位移）+ i1 真实数据对拍加 CI skip 守卫") },
        ],
        held_out: "S028-B",
    },
    SampleSpec {
        sample_id: "S029",
        class: SampleClass::Ci,
        task_label: "ci-p992-required-features",
        candidates: &[
            CandidateSpec { candidate_id: "S029-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 chore: roster I-0~I-4 实施链登记") },
            CandidateSpec { candidate_id: "S029-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 8cba466ab2 fix(ci): CI 双红修复——p992 探针补 required-features（同型第三撞）+ morse 裸名笔误") },
        ],
        held_out: "S029-B",
    },
    SampleSpec {
        sample_id: "S030",
        class: SampleClass::Ci,
        task_label: "ci-env-registry-5keys",
        candidates: &[
            CandidateSpec { candidate_id: "S030-A", source: EvidenceSource::Inline("CI 状态：红（失败）。提交 style(rust): p977 let-else 折行补零") },
            CandidateSpec { candidate_id: "S030-B", source: EvidenceSource::Inline("CI 状态：绿（修复后通过）。提交 2fc1680bd6 fix(ci): 两守卫红修复——env_registry 补登 5 键（#837/#841 探针）+ bsp 白名单行号重登记") },
        ],
        held_out: "S030-B",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_strips_absolute_paths_but_keeps_relative_refs() {
        let input = "工作面 /tmp/wt-641n @ ticket-641-n3。断言链头见 chain_cert/mod.rs:479。参考 https://github.com/x/y/issues/1 与 /home/agent/.env";
        let out = redact(input);
        assert!(!out.contains("/tmp/wt-641n"));
        assert!(!out.contains("/home/agent/.env"));
        assert!(!out.contains("https://github.com"));
        // 相对代码引用保留（评审证据本体）。
        assert!(out.contains("chain_cert/mod.rs:479"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn spec_has_three_classes_of_ten() {
        validate_spec(SAMPLES).expect("spec valid");
        assert_eq!(SAMPLES.len(), 30);
    }

    #[test]
    fn seal_sample_from_repo_file() {
        // 从 CARGO_MANIFEST_DIR 上溯到仓库根，读一个真实评审产物封存。
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let s001 = &SAMPLES[0];
        assert_eq!(s001.sample_id, "S001");
        let sealed = seal_sample(s001, repo_root).expect("seal S001");
        assert_eq!(sealed.bundles.len(), 2);
        assert_eq!(sealed.set.manifest.member_count, 2);
        // Lead 基线：held_out 指向终审订正一方，且摘要先于揭示可用。
        assert_eq!(
            sealed.baseline.baseline.selected_candidate_id.as_deref(),
            Some("S001-B")
        );
        assert_eq!(
            sealed.baseline.baseline.decision,
            super::super::lead_baseline::Decision::Selected
        );
    }

    #[test]
    fn ranking_view_is_jcs_json_with_top_level_candidate_id() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let s001 = &SAMPLES[0];
        let sealed = seal_sample(s001, repo_root).unwrap();
        for bundle in &sealed.bundles {
            // ranking_view_digest == SHA-256(JCS({"candidate_id","evidence_text"}))，
            // 且 candidate_id 顶层唯一、与 manifest 逐字一致（ADR 0025 §5）。
            let raw = resolve_evidence(
                &s001
                    .candidates
                    .iter()
                    .find(|c| c.candidate_id == bundle.manifest.candidate_id)
                    .unwrap()
                    .source,
                repo_root,
            )
            .unwrap();
            let view_value = json!({
                "candidate_id": bundle.manifest.candidate_id,
                "evidence_text": redact(&raw),
            });
            let expect = digest::sha256_hex(&jcs::canonical_bytes(&view_value));
            assert_eq!(bundle.manifest.ranking_view_digest, expect);
        }
    }

    #[test]
    fn seal_all_thirty_at_repo_root() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let sealed = seal_all(repo_root).expect("seal all 30");
        assert_eq!(sealed.len(), 30);
        // 每样本基线都指向一个真实存在的候选槽位（held_out 在 spec 内唯一，已由 validate_spec 锁）。
        let count = sealed
            .iter()
            .filter(|s| {
                s.baseline.baseline.decision == super::super::lead_baseline::Decision::Selected
            })
            .count();
        assert_eq!(count, 30);
    }
}
