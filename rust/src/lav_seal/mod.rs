//! LAV 旁路排序器接入的**证据封存层**（S1：固定历史样本集与 Lead 基线封存，票 #1167）。
//!
//! 本模块实现 ADR 0025（`docs/adr/0025-lav-shadow-ranking-evidence-contract.md`）的证据
//! 封存契约：RFC 8785（JCS）规范化 + SHA-256 内容寻址的身份机制、Candidate Evidence
//! Bundle（§2.2 精确最小 schema）、独立 Comparison Set Manifest（§4.2）、Lead 基线裁决
//! 先封存后揭示（§10.2）。**不调用任何模型、不做任何排序**——封存器是确定性非模型角色，
//! 排序器（五态 fail-closed）在 S2（票 #1168）实现。
//!
//! 本模块是旁路 sidecar，不进入生产交易判定路径；默认 feature 下不编译（`cdylib` 零膨胀），
//! 仅在 `lav_seal` feature 或 `cargo test` 下编译。
//!
//! # 身份机制一览
//!
//! ```text
//! bundle_id          = SHA-256(JCS(canonical bundle manifest bytes))
//! comparison_set_id  = SHA-256(JCS(canonical Comparison Set Manifest bytes))
//! baseline_digest    = SHA-256(JCS(canonical LeadBaselineDecision bytes))
//! ```
//!
//! 三者都不写回自身 manifest；bundle 不含 `comparison_set_id`；Comparison Set Manifest 只
//! 引用既成 `bundle_id` 并重复其非集合派生的上下文摘要。身份图是
//! `bundle_id → Comparison Set Manifest → comparison_set_id` 的单向闭包。

pub mod bundle;
pub mod comparison_set;
pub mod digest;
pub mod jcs;
pub mod lead_baseline;
pub mod samples;

pub use bundle::{Bundle, BundleManifest, SealError};
pub use comparison_set::{ComparisonSet, ComparisonSetManifest, SetError};
pub use lead_baseline::{Decision, LeadBaselineDecision, SealedBaseline};
