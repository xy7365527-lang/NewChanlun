//! #1394 C §5.4：按值边组装尚未发布的批次清单。
//!
//! 本模块只核码本闭包、三个根的顶层 schema 和跨根发布坐标；不授予出生位置许可，
//! 不写独立索引、clock 或 cut。输入标量须由调用者的同一个 Begin/Commit 读集取得。
//! 模板内 0x08 的业务许可、对象/来源/Delta 对拍仍由正式发布者核验。
#![allow(dead_code)]

use crate::v3::audit_store::{CodecRead, CodecStore};
use crate::v3::audit_store::{RefFields, RefValue};
use crate::v3::business_json::TextValue;
#[cfg_attr(
    not(feature = "s_incremental"),
    allow(
        unused_imports,
        reason = "#1488 再合 #1608：非 s_incremental 构建不触达"
    )
)]
use newchan_rust::s_content::capture_allocation::native_owned::NativeValue;
#[cfg(feature = "s_incremental")]
use newchan_rust::s_content::capture_allocation::scratch::WorkScope;
#[allow(
    unused_imports,
    reason = "#1488 再合 #1608：8组合实测无调用方，保留不删"
)]
use newchan_rust::s_content::{
    self, Built, CodecError, ContentRef, Kind, NodeDigest, NodeStore, NumberToken, Value as V,
};
use newchan_rust::session_protocol::{nonnegative, positive};
use s_content::{BorrowedNumberToken, ValueView as VV};
type Fields = RefFields<'static>;

const GENERATED_FIELDS: &[&str] = &[
    "generation",
    "structure_cut",
    "input_frontier",
    "catalog_revision",
    "profile_id",
    "profile_hash",
    "rule_revision",
    "raw_events",
    "effective_events",
    "objects",
    "witnesses",
    "relations",
    "observations",
    "scope",
];
pub(crate) const SNAPSHOT_FIELDS: &[&str] = &[
    "session_id",
    "generation",
    "structure_cut",
    "catalog_revision",
    "scope",
    "index_frontier",
    "profile_id",
    "profile_hash",
    "history_mode",
    "as_of_generation",
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "catalog_evidence",
];
pub(crate) const DELTA_FIELDS: &[&str] = &[
    "generation",
    "session_id",
    "catalog_revision",
    "base_cut",
    "next_cut",
    "seq_range",
    "index_frontier",
    "input_frontier",
    "catalog_run_status",
    "catalog_evidence",
    "upserts",
    "withdrawals",
    "replaces",
    "witnesses",
    "relations",
    "observations",
    "raw_history_added",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceCommit {
    pub source_namespace: String,
    pub source_epoch: String,
    pub message_id: String,
    pub payload_hash: String,
    pub producer_id: String,
    pub producer_epoch: String,
    pub clock_event_id: String,
    pub attempt_index: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ManifestMetadata {
    pub session_id: String,
    pub session_generation: String,
    pub writer_epoch: String,
    pub generation: i64,
    pub base_cut: String,
    pub structure_cut: String,
    pub input_frontier: i64,
    pub source_commit: SourceCommit,
    pub clock_plan_hash: String,
    pub semantic_commit_ns: String,
    pub profile_id: String,
    pub profile_hash: String,
    pub rule_revision: String,
    pub catalog_revision: String,
    pub scope: ContentRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ManifestRoots {
    pub generated_state: ContentRef,
    pub publication_template: ContentRef,
    pub delta_template: ContentRef,
}

/// 借用同一元数据字段；没有验证/来源成功token，不持有或制造文本。
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceCommitView<'a> {
    pub source_namespace: &'a str,
    pub source_epoch: &'a str,
    pub message_id: &'a str,
    pub payload_hash: &'a str,
    pub producer_id: &'a str,
    pub producer_epoch: &'a str,
    pub clock_event_id: &'a str,
    pub attempt_index: i64,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct MetadataView<'a> {
    pub session_id: &'a str,
    pub session_generation: &'a str,
    pub writer_epoch: &'a str,
    pub generation: i64,
    pub base_cut: &'a str,
    pub structure_cut: &'a str,
    pub input_frontier: i64,
    pub source_commit: SourceCommitView<'a>,
    pub clock_plan_hash: &'a str,
    pub semantic_commit_ns: &'a str,
    pub profile_id: &'a str,
    pub profile_hash: &'a str,
    pub rule_revision: &'a str,
    pub catalog_revision: &'a str,
    pub scope: s_content::RefView<'a>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct RootsView<'a> {
    pub generated_state: s_content::RefView<'a>,
    pub publication_template: s_content::RefView<'a>,
    pub delta_template: s_content::RefView<'a>,
}
/// 根拥有者仅借出字段；此trait不赋予核验、分配或发布资格。
pub(crate) trait ManifestRootSource {
    fn view(&self) -> RootsView<'_>;
}
impl ManifestRootSource for ManifestRoots {
    fn view(&self) -> RootsView<'_> {
        ManifestRoots::view(self)
    }
}
#[cfg(feature = "s_incremental")]
struct NativeRoots {
    generated_state: NativeValue<ContentRef>,
    publication_template: NativeValue<ContentRef>,
    delta_template: NativeValue<ContentRef>,
}
#[cfg(feature = "s_incremental")]
impl NativeRoots {
    fn from_view(roots: RootsView<'_>, work: Option<&WorkScope>) -> Result<Self, CodecError> {
        Ok(Self {
            generated_state: NativeValue::<ContentRef>::try_from_view(roots.generated_state, work)?,
            publication_template: NativeValue::<ContentRef>::try_from_view(
                roots.publication_template,
                work,
            )?,
            delta_template: NativeValue::<ContentRef>::try_from_view(roots.delta_template, work)?,
        })
    }
}
#[cfg(feature = "s_incremental")]
impl ManifestRootSource for NativeRoots {
    fn view(&self) -> RootsView<'_> {
        RootsView {
            generated_state: (&self.generated_state).into(),
            publication_template: (&self.publication_template).into(),
            delta_template: (&self.delta_template).into(),
        }
    }
}
#[cfg(feature = "s_incremental")]
fn same_reference(a: s_content::RefView<'_>, b: s_content::RefView<'_>) -> bool {
    a.format() == b.format()
        && a.root() == b.root()
        && a.kind() == b.kind()
        && a.extent() == b.extent()
}
#[cfg(feature = "s_incremental")]
fn same_roots(a: RootsView<'_>, b: RootsView<'_>) -> bool {
    same_reference(a.generated_state, b.generated_state)
        && same_reference(a.publication_template, b.publication_template)
        && same_reference(a.delta_template, b.delta_template)
}

impl SourceCommit {
    pub(crate) fn view(&self) -> SourceCommitView<'_> {
        SourceCommitView {
            source_namespace: &self.source_namespace,
            source_epoch: &self.source_epoch,
            message_id: &self.message_id,
            payload_hash: &self.payload_hash,
            producer_id: &self.producer_id,
            producer_epoch: &self.producer_epoch,
            clock_event_id: &self.clock_event_id,
            attempt_index: self.attempt_index,
        }
    }
}
impl ManifestMetadata {
    pub(crate) fn view(&self) -> MetadataView<'_> {
        MetadataView {
            session_id: &self.session_id,
            session_generation: &self.session_generation,
            writer_epoch: &self.writer_epoch,
            generation: self.generation,
            base_cut: &self.base_cut,
            structure_cut: &self.structure_cut,
            input_frontier: self.input_frontier,
            source_commit: self.source_commit.view(),
            clock_plan_hash: &self.clock_plan_hash,
            semantic_commit_ns: &self.semantic_commit_ns,
            profile_id: &self.profile_id,
            profile_hash: &self.profile_hash,
            rule_revision: &self.rule_revision,
            catalog_revision: &self.catalog_revision,
            scope: (&self.scope).into(),
        }
    }
}
impl ManifestRoots {
    pub(crate) fn view(&self) -> RootsView<'_> {
        RootsView {
            generated_state: (&self.generated_state).into(),
            publication_template: (&self.publication_template).into(),
            delta_template: (&self.delta_template).into(),
        }
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
use s_content::capture_allocation::{audit_json::TextOwner, OwnedBudget};
#[cfg(feature = "s_allocation_json_testonly")]
pub(crate) struct SourceCommitOwner {
    pub source_namespace: TextOwner,
    pub source_epoch: TextOwner,
    pub message_id: TextOwner,
    pub payload_hash: TextOwner,
    pub producer_id: TextOwner,
    pub producer_epoch: TextOwner,
    pub clock_event_id: TextOwner,
    pub attempt_index: i64,
}
#[cfg(feature = "s_allocation_json_testonly")]
pub(crate) struct MetaOwner {
    pub session_id: TextOwner,
    pub session_generation: TextOwner,
    pub writer_epoch: TextOwner,
    pub generation: i64,
    pub base_cut: TextOwner,
    pub structure_cut: TextOwner,
    pub input_frontier: i64,
    pub source_commit: SourceCommitOwner,
    pub clock_plan_hash: TextOwner,
    pub semantic_commit_ns: TextOwner,
    pub profile_id: TextOwner,
    pub profile_hash: TextOwner,
    pub rule_revision: TextOwner,
    pub catalog_revision: TextOwner,
    pub scope: s_content::RefOwner,
}
#[cfg(feature = "s_allocation_json_testonly")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct RootsOwner {
    pub generated_state: s_content::RefOwner,
    pub publication_template: s_content::RefOwner,
    pub delta_template: s_content::RefOwner,
}
#[cfg(feature = "s_allocation_json_testonly")]
fn copy_ref(view: s_content::RefView<'_>) -> Result<s_content::RefOwner, CodecError> {
    // 只复制已声明字段，复用normal digest/format判据；不提供F/闭包成功证明。
    Ok(Built {
        digest: view.digest()?,
        kind: view.kind(),
        extent: view.extent(),
    }
    .owned_ref())
}
#[cfg(feature = "s_allocation_json_testonly")]
impl SourceCommitOwner {
    fn from_view(v: SourceCommitView<'_>, b: &OwnedBudget) -> Result<Self, CodecError> {
        Ok(Self {
            source_namespace: TextOwner::copy_from(v.source_namespace, b)?,
            source_epoch: TextOwner::copy_from(v.source_epoch, b)?,
            message_id: TextOwner::copy_from(v.message_id, b)?,
            payload_hash: TextOwner::copy_from(v.payload_hash, b)?,
            producer_id: TextOwner::copy_from(v.producer_id, b)?,
            producer_epoch: TextOwner::copy_from(v.producer_epoch, b)?,
            clock_event_id: TextOwner::copy_from(v.clock_event_id, b)?,
            attempt_index: v.attempt_index,
        })
    }
    fn view(&self) -> SourceCommitView<'_> {
        SourceCommitView {
            source_namespace: &self.source_namespace,
            source_epoch: &self.source_epoch,
            message_id: &self.message_id,
            payload_hash: &self.payload_hash,
            producer_id: &self.producer_id,
            producer_epoch: &self.producer_epoch,
            clock_event_id: &self.clock_event_id,
            attempt_index: self.attempt_index,
        }
    }
    fn try_clone(&self) -> Result<Self, CodecError> {
        Ok(Self {
            source_namespace: self.source_namespace.try_clone()?,
            source_epoch: self.source_epoch.try_clone()?,
            message_id: self.message_id.try_clone()?,
            payload_hash: self.payload_hash.try_clone()?,
            producer_id: self.producer_id.try_clone()?,
            producer_epoch: self.producer_epoch.try_clone()?,
            clock_event_id: self.clock_event_id.try_clone()?,
            attempt_index: self.attempt_index,
        })
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
impl MetaOwner {
    /// 输入是调用方实际解码产生的UTF8借用；没有先造普通String再抄记账。
    /// 与normal DTO同样不在这里授予manifest/source/clock验证成功资格。
    pub(crate) fn from_view(v: MetadataView<'_>, b: &OwnedBudget) -> Result<Self, CodecError> {
        Ok(Self {
            session_id: TextOwner::copy_from(v.session_id, b)?,
            session_generation: TextOwner::copy_from(v.session_generation, b)?,
            writer_epoch: TextOwner::copy_from(v.writer_epoch, b)?,
            generation: v.generation,
            base_cut: TextOwner::copy_from(v.base_cut, b)?,
            structure_cut: TextOwner::copy_from(v.structure_cut, b)?,
            input_frontier: v.input_frontier,
            source_commit: SourceCommitOwner::from_view(v.source_commit, b)?,
            clock_plan_hash: TextOwner::copy_from(v.clock_plan_hash, b)?,
            semantic_commit_ns: TextOwner::copy_from(v.semantic_commit_ns, b)?,
            profile_id: TextOwner::copy_from(v.profile_id, b)?,
            profile_hash: TextOwner::copy_from(v.profile_hash, b)?,
            rule_revision: TextOwner::copy_from(v.rule_revision, b)?,
            catalog_revision: TextOwner::copy_from(v.catalog_revision, b)?,
            scope: copy_ref(v.scope)?,
        })
    }
    pub(crate) fn view(&self) -> MetadataView<'_> {
        MetadataView {
            session_id: &self.session_id,
            session_generation: &self.session_generation,
            writer_epoch: &self.writer_epoch,
            generation: self.generation,
            base_cut: &self.base_cut,
            structure_cut: &self.structure_cut,
            input_frontier: self.input_frontier,
            source_commit: self.source_commit.view(),
            clock_plan_hash: &self.clock_plan_hash,
            semantic_commit_ns: &self.semantic_commit_ns,
            profile_id: &self.profile_id,
            profile_hash: &self.profile_hash,
            rule_revision: &self.rule_revision,
            catalog_revision: &self.catalog_revision,
            scope: self.scope.view(),
        }
    }
    pub(crate) fn try_clone(&self) -> Result<Self, CodecError> {
        Ok(Self {
            session_id: self.session_id.try_clone()?,
            session_generation: self.session_generation.try_clone()?,
            writer_epoch: self.writer_epoch.try_clone()?,
            generation: self.generation,
            base_cut: self.base_cut.try_clone()?,
            structure_cut: self.structure_cut.try_clone()?,
            input_frontier: self.input_frontier,
            source_commit: self.source_commit.try_clone()?,
            clock_plan_hash: self.clock_plan_hash.try_clone()?,
            semantic_commit_ns: self.semantic_commit_ns.try_clone()?,
            profile_id: self.profile_id.try_clone()?,
            profile_hash: self.profile_hash.try_clone()?,
            rule_revision: self.rule_revision.try_clone()?,
            catalog_revision: self.catalog_revision.try_clone()?,
            scope: self.scope,
        })
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
impl std::fmt::Debug for SourceCommitOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceCommit")
            .field("source_namespace", &self.source_namespace)
            .field("source_epoch", &self.source_epoch)
            .field("message_id", &self.message_id)
            .field("payload_hash", &self.payload_hash)
            .field("producer_id", &self.producer_id)
            .field("producer_epoch", &self.producer_epoch)
            .field("clock_event_id", &self.clock_event_id)
            .field("attempt_index", &self.attempt_index)
            .finish()
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
impl std::fmt::Debug for MetaOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestMetadata")
            .field("session_id", &self.session_id)
            .field("session_generation", &self.session_generation)
            .field("writer_epoch", &self.writer_epoch)
            .field("generation", &self.generation)
            .field("base_cut", &self.base_cut)
            .field("structure_cut", &self.structure_cut)
            .field("input_frontier", &self.input_frontier)
            .field("source_commit", &self.source_commit)
            .field("clock_plan_hash", &self.clock_plan_hash)
            .field("semantic_commit_ns", &self.semantic_commit_ns)
            .field("profile_id", &self.profile_id)
            .field("profile_hash", &self.profile_hash)
            .field("rule_revision", &self.rule_revision)
            .field("catalog_revision", &self.catalog_revision)
            .field("scope", &RefValue::Owned(self.scope))
            .finish()
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
impl RootsOwner {
    pub(crate) fn from_view(v: RootsView<'_>) -> Result<Self, CodecError> {
        Ok(Self {
            generated_state: copy_ref(v.generated_state)?,
            publication_template: copy_ref(v.publication_template)?,
            delta_template: copy_ref(v.delta_template)?,
        })
    }
    pub(crate) fn view(&self) -> RootsView<'_> {
        RootsView {
            generated_state: self.generated_state.view(),
            publication_template: self.publication_template.view(),
            delta_template: self.delta_template.view(),
        }
    }
}

#[cfg(feature = "s_incremental")]
pub(crate) struct NativeMetadata {
    session_id: NativeValue<String>,
    session_generation: NativeValue<String>,
    writer_epoch: NativeValue<String>,
    base_cut: NativeValue<String>,
    structure_cut: NativeValue<String>,
    clock_plan_hash: NativeValue<String>,
    semantic_commit_ns: NativeValue<String>,
    profile_id: NativeValue<String>,
    profile_hash: NativeValue<String>,
    rule_revision: NativeValue<String>,
    catalog_revision: NativeValue<String>,
    source_namespace: NativeValue<String>,
    source_epoch: NativeValue<String>,
    message_id: NativeValue<String>,
    payload_hash: NativeValue<String>,
    producer_id: NativeValue<String>,
    producer_epoch: NativeValue<String>,
    clock_event_id: NativeValue<String>,
    generation: i64,
    input_frontier: i64,
    attempt_index: i64,
    scope: NativeValue<ContentRef>,
}
#[cfg(feature = "s_incremental")]
impl NativeMetadata {
    pub(crate) fn from_view(m: MetadataView<'_>, work: Option<&WorkScope>) -> Result<Self, CodecError> {
        Ok(Self {
            session_id: NativeValue::<String>::try_from_str(m.session_id, work)?,
            session_generation: NativeValue::<String>::try_from_str(m.session_generation, work)?,
            writer_epoch: NativeValue::<String>::try_from_str(m.writer_epoch, work)?,
            base_cut: NativeValue::<String>::try_from_str(m.base_cut, work)?,
            structure_cut: NativeValue::<String>::try_from_str(m.structure_cut, work)?,
            clock_plan_hash: NativeValue::<String>::try_from_str(m.clock_plan_hash, work)?,
            semantic_commit_ns: NativeValue::<String>::try_from_str(m.semantic_commit_ns, work)?,
            profile_id: NativeValue::<String>::try_from_str(m.profile_id, work)?,
            profile_hash: NativeValue::<String>::try_from_str(m.profile_hash, work)?,
            rule_revision: NativeValue::<String>::try_from_str(m.rule_revision, work)?,
            catalog_revision: NativeValue::<String>::try_from_str(m.catalog_revision, work)?,
            source_namespace: NativeValue::<String>::try_from_str(
                m.source_commit.source_namespace,
                work,
            )?,
            source_epoch: NativeValue::<String>::try_from_str(m.source_commit.source_epoch, work)?,
            message_id: NativeValue::<String>::try_from_str(m.source_commit.message_id, work)?,
            payload_hash: NativeValue::<String>::try_from_str(m.source_commit.payload_hash, work)?,
            producer_id: NativeValue::<String>::try_from_str(m.source_commit.producer_id, work)?,
            producer_epoch: NativeValue::<String>::try_from_str(
                m.source_commit.producer_epoch,
                work,
            )?,
            clock_event_id: NativeValue::<String>::try_from_str(
                m.source_commit.clock_event_id,
                work,
            )?,
            generation: m.generation,
            input_frontier: m.input_frontier,
            attempt_index: m.source_commit.attempt_index,
            scope: NativeValue::<ContentRef>::try_from_view(m.scope, work)?,
        })
    }
    pub(crate) fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + [
            &self.session_id, &self.session_generation, &self.writer_epoch,
            &self.base_cut, &self.structure_cut, &self.clock_plan_hash, &self.semantic_commit_ns,
            &self.profile_id, &self.profile_hash, &self.rule_revision, &self.catalog_revision,
            &self.source_namespace, &self.source_epoch, &self.message_id, &self.payload_hash,
            &self.producer_id, &self.producer_epoch, &self.clock_event_id,
        ].iter().map(|text| text.capacity()).sum::<usize>()
            + self.scope.format.capacity() + self.scope.root.capacity()
    }
    pub(crate) fn matches(&self, m: MetadataView<'_>) -> bool {
        self.session_id.as_str() == m.session_id
            && self.session_generation.as_str() == m.session_generation
            && self.writer_epoch.as_str() == m.writer_epoch
            && self.base_cut.as_str() == m.base_cut
            && self.structure_cut.as_str() == m.structure_cut
            && self.clock_plan_hash.as_str() == m.clock_plan_hash
            && self.semantic_commit_ns.as_str() == m.semantic_commit_ns
            && self.profile_id.as_str() == m.profile_id
            && self.profile_hash.as_str() == m.profile_hash
            && self.rule_revision.as_str() == m.rule_revision
            && self.catalog_revision.as_str() == m.catalog_revision
            && self.source_namespace.as_str() == m.source_commit.source_namespace
            && self.source_epoch.as_str() == m.source_commit.source_epoch
            && self.message_id.as_str() == m.source_commit.message_id
            && self.payload_hash.as_str() == m.source_commit.payload_hash
            && self.producer_id.as_str() == m.source_commit.producer_id
            && self.producer_epoch.as_str() == m.source_commit.producer_epoch
            && self.clock_event_id.as_str() == m.source_commit.clock_event_id
            && self.generation == m.generation
            && self.input_frontier == m.input_frontier
            && self.attempt_index == m.source_commit.attempt_index
            && same_reference((&self.scope).into(), m.scope)
    }
}

pub(super) struct UnpublishedManifest {
    pub value: Built,
    pub batch_id: TextValue,
    #[cfg(feature = "s_incremental")]
    closure_units: Option<s_content::closure_delta::VerifiedUnits>,
}

// #1394 Native-only, transaction-local manifest workspace. The constructor below is the
// only issuer: successful unchanged manifest rules plus the complete actual read/write set.
// Changing those rules requires incrementing this revision. Modes and Flavor::Value remain
// fixed by assemble_manifest_body; no caller-provided "same" or root-only success token.
#[cfg(feature = "s_incremental")]
const MANIFEST_WORKSPACE_REVISION: u32 = 1;
#[cfg(feature = "s_incremental")]
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub(crate) struct ManifestWorkspaceStats {
    pub issued: u64,
    pub overflow: u64,
    pub rebound: u64,
    pub rebind_rows: u64,
    pub replay_puts: u64,
    pub publication_owner_scopes: u64,
    pub publication_reused_reads: u64,
    pub publication_fallback_sql_reads: u64,
    pub publication_put_sql_checks: u64,
    pub publication_unknown_write_invalidations: u64,
    pub publication_owner_drops: u64,
    pub recorded_entries: u64,
    pub recorded_body_len: u64,
    pub recorded_body_capacity: u64,
    pub recorded_put_capacity: u64,
    pub max_recording_body_capacity: u64,
    pub max_recording_entries: u64,
    pub max_recording_body_len: u64,
    pub max_recording_put_capacity: u64,
    pub last_workspace_source_cost: u64,
    pub max_recording_source_cost: u64,
}
#[cfg(feature = "s_incremental")]
thread_local! {
    static MANIFEST_WORKSPACE_STATS: std::cell::Cell<ManifestWorkspaceStats> =
        std::cell::Cell::new(ManifestWorkspaceStats::default());
}
#[cfg(feature = "s_incremental")]
pub(crate) fn manifest_workspace_stats() -> ManifestWorkspaceStats {
    MANIFEST_WORKSPACE_STATS.with(std::cell::Cell::get)
}
#[cfg(feature = "s_incremental")]
fn record_workspace_stats(f: impl FnOnce(&mut ManifestWorkspaceStats)) {
    MANIFEST_WORKSPACE_STATS.with(|stats| {
        let mut value = stats.get();
        f(&mut value);
        stats.set(value);
    });
}
/// Opt-in TestOnly stderr diagnostics. This does not grant workspace qualification or
/// alter protocol replies; failed diagnostic writes must not change the business result.
#[cfg(feature = "s_incremental")]
fn diagnose_manifest_workspace(
    stage: &str,
    generation: i64,
    budget: crate::v3::ManifestWorkspaceBudget,
    workspace_source_cost: usize,
    recording_peak_source_cost: usize,
) {
    if std::env::var_os("S_MANIFEST_WORKSPACE_DIAGNOSTICS").as_deref()
        != Some(std::ffi::OsStr::new("1"))
    {
        return;
    }
    use std::io::Write as _;
    let line = serde_json::json!({
        "diagnostic": "WriterRetentionWorkspace",
        "stage": stage,
        "generation": generation,
        "stats_scope": "thread_cumulative_sums_and_maxima",
        "stats": manifest_workspace_stats(),
        "budget": budget,
        "workspace_source_cost": workspace_source_cost,
        "recording_peak_source_cost": recording_peak_source_cost,
    });
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}
#[cfg(feature = "s_incremental")]
pub(crate) struct PreparedManifestWorkspace {
    revision: u32,
    format: &'static str,
    metadata: NativeMetadata,
    roots: NativeRoots,
    manifest_value: Built,
    batch_id: TextValue,
    budget: crate::v3::ManifestWorkspaceBudget,
    source_cost: usize,
    originals: std::collections::BTreeMap<NodeDigest, (u8, s_content::CodecVec<u8>)>,
    puts: s_content::CodecVec<NodeDigest>,
    // originals 先释放，树许可后归还；随 workspace move，不复制全池。
    tree_permit: Option<s_content::capture_allocation::scratch::NativeReservation>,
}
#[cfg(feature = "s_incremental")]
impl PreparedManifestWorkspace {
    /// Consume into a fixed, exclusive transaction phase. The private lease has no raw SQL,
    /// hook or connection access API. Nothing qualified is returned to the Commit caller.
    pub(crate) fn resolve_in_publication(
        self,
        tx: &mut rusqlite::Transaction<'_>,
        token: &str,
        metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized),
        manifest: &UnpublishedManifest,
    ) -> Result<super::content_publication::ResolvedTemplates, String> {
        self.resolve_in_publication_admitted(tx, token, metadata, roots, manifest, None)
    }
    pub(crate) fn resolve_in_publication_admitted(
        self,
        tx: &mut rusqlite::Transaction<'_>,
        token: &str,
        metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized),
        manifest: &UnpublishedManifest,
        work: Option<&s_content::capture_allocation::scratch::WorkScope>,
    ) -> Result<super::content_publication::ResolvedTemplates, String> {
        self.resolve_in_publication_inner(
            tx,
            token,
            metadata,
            roots,
            manifest,
            work,
            #[cfg(all(test, feature = "s_incremental"))]
            None,
        )
    }
    /// staged 臂：本租约新写节点只进暂存，由 Commit 的 Final cut 仅落可达闭包；
    /// 原件重绑仍只读真实 SQL，暂存层不冒充素材。
    #[cfg(all(test, feature = "s_incremental"))]
    pub(crate) fn resolve_in_publication_staged(
        self,
        tx: &mut rusqlite::Transaction<'_>,
        token: &str,
        metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized),
        manifest: &UnpublishedManifest,
        work: Option<&s_content::capture_allocation::scratch::WorkScope>,
        stage: &super::staged_nodes::StageOwner<'_>,
    ) -> Result<super::content_publication::ResolvedTemplates, String> {
        self.resolve_in_publication_inner(tx, token, metadata, roots, manifest, work, Some(stage))
    }
    fn resolve_in_publication_inner(
        self,
        tx: &mut rusqlite::Transaction<'_>,
        token: &str,
        metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized),
        manifest: &UnpublishedManifest,
        work: Option<&s_content::capture_allocation::scratch::WorkScope>,
        #[cfg(all(test, feature = "s_incremental"))] stage: Option<
            &super::staged_nodes::StageOwner<'_>,
        >,
    ) -> Result<super::content_publication::ResolvedTemplates, String> {
        // No qualified originals exist across these actual Begin/scope/frontier SELECTs.
        // The issuer itself performs them; callers cannot substitute a scope/frontier token.
        super::content_publication::verify_current_begin_admitted(
            tx,
            token,
            metadata.view(),
            work,
        )?;
        let previous_frontier =
            super::content_publication::current_publication_frontier_admitted(tx, work)?;
        NativePublicationLease::new(tx)
            .map_err(|e| format!("PublicationInvalid：{e}"))?
            .resolve(
                self,
                metadata,
                roots,
                manifest,
                previous_frontier,
                work,
                #[cfg(all(test, feature = "s_incremental"))]
                stage,
            )
    }
}

#[cfg(feature = "s_incremental")]
struct NativePublicationLease<'lease, 'conn> {
    tx: &'lease mut rusqlite::Transaction<'conn>,
    insert_can_preserve: bool,
}
#[cfg(feature = "s_incremental")]
impl<'lease, 'conn> NativePublicationLease<'lease, 'conn> {
    fn new(tx: &'lease mut rusqlite::Transaction<'conn>) -> Result<Self, CodecError> {
        let insert_can_preserve = super::content_store::publication_insert_gate(tx)?;
        Ok(Self {
            tx,
            insert_can_preserve,
        })
    }
    fn resolve(
        self,
        workspace: PreparedManifestWorkspace,
        metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized),
        manifest: &UnpublishedManifest,
        previous_frontier: i64,
        work: Option<&s_content::capture_allocation::scratch::WorkScope>,
        #[cfg(all(test, feature = "s_incremental"))] stage: Option<
            &super::staged_nodes::StageOwner<'_>,
        >,
    ) -> Result<super::content_publication::ResolvedTemplates, String> {
        let budget = workspace.budget;
        let source_cost = workspace.source_cost;
        let before = manifest_workspace_stats();
        let tx = &*self.tx;
        let mut store = PublicationMaterialStore::new(tx, self.insert_can_preserve).with_work(work);
        #[cfg(all(test, feature = "s_incremental"))]
        {
            store.stage = stage;
        }
        let mut issued_owner = false;
        let mut bound_originals = 0usize;
        let mut owner_body_capacity = 0usize;
        let result = (|| {
            let context_equal = workspace.revision == MANIFEST_WORKSPACE_REVISION
                && workspace.format == s_content::CONTENT_FORMAT
                && workspace.metadata.matches(metadata.view())
                && same_roots(workspace.roots.view(), roots.view())
                && workspace.manifest_value == manifest.value
                && workspace.batch_id == manifest.batch_id;
            // The private helper always reads *every* actual row with SQL + F and compares
            // all bytes, even after context/material mismatch. No caller can mint its owner.
            let equal = store
                .rebind_originals(&workspace.originals)
                .map_err(|e| format!("PublicationInvalid：{e}"))?
                && context_equal;
            if equal {
                record_workspace_stats(|stats| stats.rebound += 1);
                // Qualification has not been installed yet: replay owns these literal puts.
                // Each put still performs its original SQL/full-byte check. Unknown effects
                // forfeit the base before it can be installed; the strict final reads remain.
                for digest in &workspace.puts {
                    let (tag, body) = workspace
                        .originals
                        .get(digest)
                        .expect("recorded successful put");
                    s_content::NodeSink::put(&mut store, *digest, *tag, body)
                        .map_err(|e| format!("PublicationInvalid：{e}"))?;
                    record_workspace_stats(|stats| stats.replay_puts += 1);
                }
                if store.bind_after_replay {
                    // Move the single budgeted map; never clone a second complete pool.
                    store.originals = workspace.originals;
                    store.tree_permit = workspace.tree_permit;
                    // Even a real put which was a Noop retires its target. The next read is
                    // actual SQL/F, and remains cold here (no new unbudgeted full-node memo).
                    for digest in workspace.puts {
                        store.originals.remove(&digest);
                    }
                    bound_originals = store.originals.len();
                    owner_body_capacity = store
                        .originals
                        .values()
                        .map(|(_, body)| body.capacity())
                        .sum();
                    issued_owner = true;
                    record_workspace_stats(|stats| stats.publication_owner_scopes += 1);
                } else {
                    drop(workspace); // Unknown effects cannot carry qualified originals onward.
                }
            } else {
                drop(workspace);
                let computed = assemble_unpublished(&mut store, metadata, roots)
                    .map_err(|e| format!("PublicationInvalid：{e}"))?;
                if computed.batch_id != manifest.batch_id
                    || computed.value.content_ref() != manifest.value.content_ref()
                {
                    return Err("IdentityConflict：manifest 与当前三个根/上下文不符".into());
                }
            }
            // This fixed helper has only actual independent-index SELECTs and NodeStore
            // reads/puts. Both final Public roots still run the original fresh codec pass.
            super::content_publication::resolve_indexed_templates(
                tx,
                &mut store as &mut dyn NodeStore,
                metadata,
                roots,
                &manifest.batch_id,
                previous_frontier,
            )
        })();
        // Qualified bytes end here, before later raw SQL and before the new keeper exists.
        let remaining_originals = store.originals.len();
        drop(store);
        if issued_owner {
            record_workspace_stats(|stats| stats.publication_owner_drops += 1);
        }
        diagnose_manifest_publication_reuse(
            metadata.generation,
            budget,
            source_cost,
            bound_originals,
            remaining_originals,
            owner_body_capacity,
            before,
        );
        result
    }
}

/// Private to the fixed Native publication lease. The actual SQL rebind is the sole issuer;
/// no constructor, serialization, transaction getter or caller-provided bool grants a base.
#[cfg(feature = "s_incremental")]
struct PublicationMaterialStore<'a, 'conn> {
    tx: &'a rusqlite::Transaction<'conn>,
    originals: std::collections::BTreeMap<NodeDigest, (u8, s_content::CodecVec<u8>)>,
    insert_can_preserve: bool,
    bind_after_replay: bool,
    poisoned: bool,
    work: Option<&'a s_content::capture_allocation::scratch::WorkScope>,
    tree_permit: Option<s_content::capture_allocation::scratch::NativeReservation>,
    #[cfg(all(test, feature = "s_incremental"))]
    stage: Option<&'a super::staged_nodes::StageOwner<'a>>,
    #[cfg(all(test, feature = "s_incremental"))]
    rebinding: bool,
    #[cfg(all(test, feature = "s_incremental"))]
    staged_puts: std::collections::BTreeSet<NodeDigest>,
}
#[cfg(feature = "s_incremental")]
impl<'a, 'conn> PublicationMaterialStore<'a, 'conn> {
    fn new(tx: &'a rusqlite::Transaction<'conn>, insert_can_preserve: bool) -> Self {
        Self {
            tx,
            originals: Default::default(),
            insert_can_preserve,
            bind_after_replay: true,
            poisoned: false,
            work: None,
            tree_permit: None,
            #[cfg(all(test, feature = "s_incremental"))]
            stage: None,
            #[cfg(all(test, feature = "s_incremental"))]
            rebinding: false,
            #[cfg(all(test, feature = "s_incremental"))]
            staged_puts: Default::default(),
        }
    }
    fn with_work(
        mut self,
        work: Option<&'a s_content::capture_allocation::scratch::WorkScope>,
    ) -> Self {
        self.work = work;
        self
    }
    fn check_live(&mut self) -> Result<(), CodecError> {
        if self.poisoned || self.tx.is_autocommit() {
            self.originals.clear();
            self.poisoned = true;
            return Err(CodecError::new(
                s_content::code::SOURCE,
                "Native发布租约已失败或事务已结束；不得重试/回落",
            ));
        }
        Ok(())
    }
    fn rebind_originals(
        &mut self,
        originals: &std::collections::BTreeMap<NodeDigest, (u8, s_content::CodecVec<u8>)>,
    ) -> Result<bool, CodecError> {
        // 重绑只认真实 SQL 原件，暂存层新节点不冒充素材。
        #[cfg(all(test, feature = "s_incremental"))]
        {
            self.rebinding = true;
        }
        let result = self.rebind_originals_inner(originals);
        #[cfg(all(test, feature = "s_incremental"))]
        {
            self.rebinding = false;
        }
        result
    }
    fn rebind_originals_inner(
        &mut self,
        originals: &std::collections::BTreeMap<NodeDigest, (u8, s_content::CodecVec<u8>)>,
    ) -> Result<bool, CodecError> {
        let mut equal = true;
        for (digest, (tag, body)) in originals {
            record_workspace_stats(|stats| stats.rebind_rows += 1);
            match s_content::NodeSource::checked_read_scoped(self, digest)? {
                Some(frame) => equal &= frame.tag() == *tag && frame.body() == &body[..],
                None => equal = false,
            }
        }
        Ok(equal)
    }
    fn current_checked(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        self.current_scoped(digest)
            .and_then(|f| f.map(s_content::job::ScopedFrame::into_frame).transpose())
    }
    fn current_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
        self.check_live()?;
        let result = (|| {
            if let Some((tag, body)) = self.originals.get(digest) {
                record_workspace_stats(|stats| stats.publication_reused_reads += 1);
                let (body, permit) = super::content_store::native_body_reserved(self.work, body)?;
                let frame = s_content::job::CheckedFrame::checked(digest, *tag, body)?;
                Ok(Some(s_content::job::ScopedFrame::new(frame, permit)))
            } else {
                record_workspace_stats(|stats| stats.publication_fallback_sql_reads += 1);
                #[cfg(all(test, feature = "s_incremental"))]
                if let Some(stage) = self.stage {
                    stage.check_transaction(self.tx)?;
                }
                let current =
                    super::content_store::publication_read_checked(self.tx, digest, self.work)?;
                #[cfg(all(test, feature = "s_incremental"))]
                // 只回读本租约自己 put 进暂存的新节点；其余暂存不冒充素材。
                if current.is_none() && !self.rebinding && self.staged_puts.contains(digest) {
                    if let Some(stage) = self.stage {
                        return stage.read_scoped(digest);
                    }
                }
                Ok(current)
            }
        })();
        if result.is_err() {
            self.originals.clear();
            self.poisoned = true;
        }
        result
    }
}
#[cfg(feature = "s_incremental")]
impl s_content::NodeSource for PublicationMaterialStore<'_, '_> {
    fn checked_read_scoped(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
        self.current_scoped(d)
    }
    fn codec_work(&self) -> Option<&s_content::capture_allocation::scratch::WorkScope> {
        self.work
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<s_content::NodeRead, CodecError> {
        self.current_checked(digest).map(|frame| match frame {
            None => s_content::NodeRead::Missing,
            Some(frame) => {
                let (tag, body) = frame.into_parts();
                s_content::NodeRead::Found { tag, body }
            }
        })
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        self.current_checked(digest)
    }
}
#[cfg(feature = "s_incremental")]
impl s_content::NodeSink for PublicationMaterialStore<'_, '_> {
    fn codec_sink_work(&self) -> Option<&s_content::capture_allocation::scratch::WorkScope> {
        self.work
    }
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        self.check_live()?;
        // Retire the target before every real put, even Noop. Its first subsequent read
        // cannot reuse the old owner. The original SQL and byte/digest checks are mandatory.
        self.originals.remove(&digest);
        record_workspace_stats(|stats| stats.publication_put_sql_checks += 1);
        let result = super::content_store::publication_put(
            self.tx,
            self.insert_can_preserve,
            digest,
            tag,
            body,
            self.work,
            #[cfg(all(test, feature = "s_incremental"))]
            self.stage,
        );
        #[cfg(all(test, feature = "s_incremental"))]
        if self.stage.is_some()
            && matches!(result, Ok(super::content_store::PublicationPutEffect::Noop))
        {
            self.staged_puts.insert(digest);
        }
        match result {
            Ok(
                super::content_store::PublicationPutEffect::Noop
                | super::content_store::PublicationPutEffect::InsertPreservingBase,
            ) => Ok(()),
            Ok(super::content_store::PublicationPutEffect::Unknown) => {
                self.originals.clear();
                self.bind_after_replay = false;
                record_workspace_stats(|stats| stats.publication_unknown_write_invalidations += 1);
                Ok(())
            }
            Err(error) => {
                self.originals.clear();
                self.poisoned = true;
                Err(error)
            }
        }
    }
}
#[cfg(feature = "s_incremental")]
fn diagnose_manifest_publication_reuse(
    generation: i64,
    budget: crate::v3::ManifestWorkspaceBudget,
    source_cost: usize,
    bound_originals: usize,
    remaining_originals: usize,
    owner_body_capacity: usize,
    before: ManifestWorkspaceStats,
) {
    if std::env::var_os("S_MANIFEST_WORKSPACE_DIAGNOSTICS").as_deref()
        != Some(std::ffi::OsStr::new("1"))
    {
        return;
    }
    use std::io::Write as _;
    let after = manifest_workspace_stats();
    let line = serde_json::json!({
        "diagnostic": "WriterRetentionWorkspace",
        "stage": "publish-material-owner-dropped",
        "generation": generation,
        "budget": budget,
        "workspace_source_cost": source_cost,
        "scope_bound_originals": bound_originals,
        "scope_remaining_originals": remaining_originals,
        "scope_owner_body_capacity": owner_body_capacity,
        "scope_rebind_sql_rows": after.rebind_rows - before.rebind_rows,
        "scope_reused_original_reads": after.publication_reused_reads - before.publication_reused_reads,
        "scope_fallback_sql_reads": after.publication_fallback_sql_reads - before.publication_fallback_sql_reads,
        "scope_put_sql_checks": after.publication_put_sql_checks - before.publication_put_sql_checks,
        "scope_unknown_write_invalidations": after.publication_unknown_write_invalidations - before.publication_unknown_write_invalidations,
        "scope_owner_drops": after.publication_owner_drops - before.publication_owner_drops,
        "stats": after,
    });
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}
// Source-cost encoding, not heap accounting: text = u64 length + UTF-8 bytes;
// ContentRef = both texts + kind/extent-present bytes + optional u64 extent. Numeric
// metadata fields are i64, budget fields u64; Built is digest/kind/presence/extent.
#[cfg(feature = "s_incremental")]
fn workspace_certificate_source_cost(
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    value: Option<&Built>,
    batch: Option<&str>,
) -> Option<usize> {
    fn text(sum: &mut usize, value: &str) -> Option<()> {
        *sum = sum.checked_add(8)?.checked_add(value.len())?;
        Some(())
    }
    fn reference(sum: &mut usize, r: s_content::RefView<'_>) -> Option<()> {
        text(sum, r.format())?;
        text(sum, r.root())?;
        *sum = sum.checked_add(2 + if r.extent().is_some() { 8 } else { 0 })?;
        Some(())
    }
    // revision u32 + four fixed Mode/Flavor pairs + frozen budget's four u64 values
    // + generation/input_frontier/source attempt i64 values + two u64 collection lengths.
    let mut cost = 4usize + 8 + 32 + 24 + 16;
    text(&mut cost, s_content::CONTENT_FORMAT)?;
    for value in [
        &m.session_id,
        &m.session_generation,
        &m.writer_epoch,
        &m.base_cut,
        &m.structure_cut,
        &m.clock_plan_hash,
        &m.semantic_commit_ns,
        &m.profile_id,
        &m.profile_hash,
        &m.rule_revision,
        &m.catalog_revision,
        &m.source_commit.source_namespace,
        &m.source_commit.source_epoch,
        &m.source_commit.message_id,
        &m.source_commit.payload_hash,
        &m.source_commit.producer_id,
        &m.source_commit.producer_epoch,
        &m.source_commit.clock_event_id,
    ] {
        text(&mut cost, value)?
    }
    let roots = roots.view();
    for r in [
        (&m.scope).into(),
        roots.generated_state,
        roots.publication_template,
        roots.delta_template,
    ] {
        reference(&mut cost, r)?;
    }
    // Before construction reserve the maximum Built representation. Final accounting
    // uses actual kind/extent and actual batch length before issuing any workspace.
    cost = cost.checked_add(value.map_or(42, |v| 34 + if v.extent.is_some() { 8 } else { 0 }))?;
    let reserved_batch = batch_text(&[0; 32]);
    if let Some(batch) = batch {
        text(&mut cost, batch)?
    } else {
        text(&mut cost, reserved_batch.as_str())?
    }
    Some(cost)
}
#[cfg(feature = "s_incremental")]
struct ManifestRecordingStore<'a> {
    inner: &'a mut dyn NodeStore,
    allowance: usize,
    certificate_cost: usize,
    source_cost: usize,
    originals: Option<std::collections::BTreeMap<NodeDigest, (u8, s_content::CodecVec<u8>)>>,
    retained_capacity: usize,
    accepted_entries: usize,
    accepted_body_len: usize,
    puts: s_content::CodecVec<NodeDigest>,
    tree_permit: Option<s_content::capture_allocation::scratch::NativeReservation>,
}
#[cfg(feature = "s_incremental")]
impl ManifestRecordingStore<'_> {
    fn record(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        let Some(originals) = &mut self.originals else {
            return Ok(());
        };
        if let Some((old_tag, old_body)) = originals.get(&digest) {
            if *old_tag == tag && &old_body[..] == body {
                return Ok(());
            }
            self.originals = None;
            self.puts.clear();
            return Ok(());
        }
        // Same canonical node source recipe as Keeper: body + digest/tag/proof 97B.
        let next = self
            .source_cost
            .checked_add(body.len())
            .and_then(|v| v.checked_add(97));
        let Some(next) = next.filter(|v| *v <= self.allowance) else {
            self.originals = None;
            self.puts.clear();
            return Ok(());
        };
        let admission = crate::v3::audit_store::NativeAdmission(self.inner.codec_work());
        originals
            .len()
            .checked_add(1)
            .ok_or_else(|| admission.native_overflow())?;
        // #1488 0035：增量许可——只为新增条目取一份树槽上界并入已持许可；
        // 不再整树重许可，旧许可不与整树新许可并存（单条上界与整树上界逐条相加相等）。
        let mut tree_step = super::content_store::native_tree_storage::<
            NodeDigest,
            (u8, s_content::CodecVec<u8>),
        >(self.inner.codec_work(), 1)?;
        let mut owned_body = s_content::CodecVec::default();
        admission.native_codec_room(&mut owned_body, body.len())?;
        owned_body.try_extend_bytes(body)?;
        let body = owned_body;
        let totals = self
            .retained_capacity
            .checked_add(body.capacity())
            .zip(self.accepted_entries.checked_add(1))
            .zip(self.accepted_body_len.checked_add(body.len()));
        let Some(((capacity, entries), len)) = totals else {
            self.originals = None;
            self.puts.clear();
            return Ok(());
        };
        self.source_cost = next;
        self.retained_capacity = capacity;
        self.accepted_entries = entries;
        self.accepted_body_len = len;
        originals.insert(digest, (tag, body));
        if let Some(mut step) = tree_step.take() {
            if let Some(held) = self.tree_permit.as_mut() {
                if let Err(failure) = held.merge(&mut step) {
                    let work = self.inner.codec_work().expect("树许可来自当前操作域");
                    return Err(work.reject(failure));
                }
            } else {
                self.tree_permit = Some(step);
            }
        }
        Ok(())
    }
}
#[cfg(feature = "s_incremental")]
impl s_content::NodeSource for ManifestRecordingStore<'_> {
    fn checked_read_scoped(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<s_content::job::ScopedFrame>, CodecError> {
        let frame = self.inner.checked_read_scoped(d)?;
        if let Some(frame) = &frame {
            self.record(*d, frame.tag(), frame.body())?;
        }
        Ok(frame)
    }
    fn codec_work(&self) -> Option<&s_content::capture_allocation::scratch::WorkScope> {
        self.inner.codec_work()
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<s_content::NodeRead, CodecError> {
        let result = self.inner.read(digest)?;
        match &result {
            s_content::NodeRead::Found { tag, body } => self.record(*digest, *tag, body)?,
            s_content::NodeRead::Missing => {
                self.originals = None;
                self.puts.clear();
            }
        }
        Ok(result)
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<s_content::job::CheckedFrame>, CodecError> {
        let result = self.inner.checked_read(digest)?;
        if let Some(frame) = &result {
            self.record(*digest, frame.tag(), frame.body())?
        }
        Ok(result)
    }
}
#[cfg(feature = "s_incremental")]
impl s_content::NodeSink for ManifestRecordingStore<'_> {
    fn codec_sink_work(&self) -> Option<&s_content::capture_allocation::scratch::WorkScope> {
        self.inner.codec_sink_work()
    }
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        self.inner.put(digest, tag, body)?;
        self.record(digest, tag, body)?;
        if self.originals.is_some() {
            if let Some(next) = self
                .source_cost
                .checked_add(32)
                .filter(|v| *v <= self.allowance)
            {
                crate::v3::audit_store::NativeAdmission(self.inner.codec_work())
                    .native_codec_room(&mut self.puts, 1)?;
                self.puts.try_push(digest)?;
                self.source_cost = next;
            } else {
                self.originals = None;
                self.puts.clear();
            }
        }
        Ok(())
    }
}
#[cfg(feature = "s_incremental")]
pub(crate) fn assemble_unpublished_with_workspace(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    budget: crate::v3::ManifestWorkspaceBudget,
) -> Result<(UnpublishedManifest, Option<PreparedManifestWorkspace>), CodecError> {
    assemble_workspace_units(store, metadata, roots, budget, false)
}
#[cfg(feature = "s_incremental")]
fn assemble_workspace_units(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    budget: crate::v3::ManifestWorkspaceBudget,
    collect_units: bool,
) -> Result<(UnpublishedManifest, Option<PreparedManifestWorkspace>), CodecError> {
    let reserved_certificate = workspace_certificate_source_cost(metadata, roots, None, None)
        .filter(|cost| *cost <= budget.allowance());
    let mut recording = ManifestRecordingStore {
        inner: store,
        allowance: budget.allowance(),
        certificate_cost: reserved_certificate.unwrap_or(0),
        source_cost: reserved_certificate.unwrap_or(0),
        originals: reserved_certificate.map(|_| Default::default()),
        retained_capacity: 0,
        accepted_entries: 0,
        accepted_body_len: 0,
        puts: Default::default(),
        tree_permit: None,
    };
    let mut view = crate::v3::audit_store::StoreView::generic(&mut recording);
    view.collect_manifest_units = collect_units;
    let manifest = assemble_unpublished_in_store(&mut view, metadata, roots)?;
    drop(view);
    let recording_peak_source_cost = recording.source_cost;
    if recording.originals.is_some() {
        let actual = workspace_certificate_source_cost(
            metadata,
            roots,
            Some(&manifest.value),
            Some(&manifest.batch_id),
        );
        let cost = actual
            .and_then(|fixed| {
                recording
                    .source_cost
                    .checked_sub(recording.certificate_cost)
                    .and_then(|variable| variable.checked_add(fixed))
            })
            .filter(|cost| *cost <= budget.allowance());
        if let Some(cost) = cost {
            recording.source_cost = cost
        } else {
            recording.originals = None;
            recording.puts.clear();
        }
    }
    let workspace_source_cost = if recording.originals.is_some() {
        recording.source_cost
    } else {
        0
    };
    record_workspace_stats(|stats| {
        stats.last_workspace_source_cost = workspace_source_cost as u64;
        stats.max_recording_source_cost = stats
            .max_recording_source_cost
            .max(recording_peak_source_cost as u64);
        stats.max_recording_body_capacity = stats
            .max_recording_body_capacity
            .max(recording.retained_capacity as u64);
        stats.max_recording_entries = stats
            .max_recording_entries
            .max(recording.accepted_entries as u64);
        stats.max_recording_body_len = stats
            .max_recording_body_len
            .max(recording.accepted_body_len as u64);
        // clear() retains Vec capacity after overflow, so count it even without originals.
        let put_capacity = (recording.puts.capacity() * std::mem::size_of::<NodeDigest>()) as u64;
        stats.recorded_put_capacity += put_capacity;
        stats.max_recording_put_capacity = stats.max_recording_put_capacity.max(put_capacity);
        if let Some(originals) = &recording.originals {
            stats.recorded_entries += originals.len() as u64;
            stats.recorded_body_len += originals
                .values()
                .map(|(_, body)| body.len() as u64)
                .sum::<u64>();
            stats.recorded_body_capacity += recording.retained_capacity as u64;
        }
    });
    let workspace = if let Some(originals) = recording.originals {
        let work = recording.inner.codec_work();
        Some(PreparedManifestWorkspace {
            revision: MANIFEST_WORKSPACE_REVISION,
            format: s_content::CONTENT_FORMAT,
            metadata: NativeMetadata::from_view(metadata.view(), work)?,
            roots: NativeRoots::from_view(roots.view(), work)?,
            manifest_value: manifest.value.clone(),
            batch_id: TextValue::Plain(NativeValue::<String>::try_from_str(
                &manifest.batch_id,
                work,
            )?),
            budget,
            source_cost: workspace_source_cost,
            originals,
            puts: recording.puts,
            tree_permit: recording.tree_permit,
        })
    } else {
        None
    };
    record_workspace_stats(|stats| {
        if workspace.is_some() {
            stats.issued += 1
        } else {
            stats.overflow += 1
        }
    });
    diagnose_manifest_workspace(
        if workspace.is_some() {
            "prepare-issued"
        } else {
            "prepare-overflow"
        },
        metadata.generation,
        budget,
        workspace_source_cost,
        recording_peak_source_cost,
    );
    Ok((manifest, workspace))
}

/// 同次来源成功和清单完整组装的请求内证明；无 Clone、无反序列化、无公开构造。
/// 原件在发布租约内仍须真实重绑，本类型不授予跨代跳过权限。
#[cfg(feature = "s_incremental")]
pub(crate) struct ManifestSourceReceipt {
    source: crate::v3::audit_owner::RawReceipt,
    metadata: NativeMetadata,
    roots: NativeRoots,
    manifest_value: Built,
    batch_id: TextValue,
    #[cfg(feature = "s_incremental")]
    closure_units: Option<s_content::closure_delta::VerifiedUnits>,
}
#[cfg(feature = "s_incremental")]
impl ManifestSourceReceipt {
    pub(crate) fn take_verified_units(
        &mut self, metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized), manifest: &UnpublishedManifest,
    ) -> Result<s_content::closure_delta::VerifiedUnits, String> {
        if !self.metadata.matches(metadata.view()) || !same_roots(self.roots.view(), roots.view())
            || self.manifest_value != manifest.value || self.batch_id != manifest.batch_id
        { return Err("IdentityConflict：P6单位导出与当前清单不同".into()); }
        self.source.check_manifest(metadata.generation, &metadata.profile_id, &metadata.rule_revision,
            metadata.input_frontier, (&metadata.scope).into(), roots.view().generated_state)?;
        self.closure_units.take().ok_or_else(|| "AuditLedgerStale：P6单位未收集或已消费".into())
    }
    pub(crate) fn consume(
        self,
        token: &str,
        metadata: &ManifestMetadata,
        roots: &(impl ManifestRootSource + ?Sized),
        manifest: &UnpublishedManifest,
        raw_range: std::ops::Range<usize>,
    ) -> Result<crate::v3::audit_owner::RawReceipt, String> {
        if !self.metadata.matches(metadata.view()) || !same_roots(self.roots.view(), roots.view())
            || self.manifest_value != manifest.value || self.batch_id != manifest.batch_id
        {
            return Err("IdentityConflict：P6组装证明与当前清单不同".into());
        }
        self.source.check_request(token, metadata.generation, &metadata.rule_revision,
            raw_range, (&metadata.scope).into(), roots.view().generated_state)?;
        Ok(self.source)
    }
}

/// 沿用既有workspace/无workspace存储选择，只组装一次；不另跑四根Probe。
#[cfg(feature = "s_incremental")]
pub(crate) fn assemble_with_source(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    source: crate::v3::audit_owner::RawReceipt,
    budget: Option<crate::v3::ManifestWorkspaceBudget>,
    collect_units: bool,
) -> Result<(UnpublishedManifest, Option<PreparedManifestWorkspace>, ManifestSourceReceipt), String> {
    source.check_manifest(metadata.generation, &metadata.profile_id, &metadata.rule_revision,
        metadata.input_frontier, (&metadata.scope).into(), roots.view().generated_state)?;
    let (mut manifest, workspace) = source.with_store(store, |bounded| {
        match budget {
            Some(budget) => assemble_workspace_units(bounded, metadata, roots, budget, collect_units)
                .map_err(|e| e.to_string()),
            None => {
                let mut view = crate::v3::audit_store::StoreView::generic(bounded);
                view.collect_manifest_units = collect_units;
                assemble_unpublished_in_store(&mut view, metadata, roots)
                    .map(|manifest| (manifest, None)).map_err(|e| e.to_string())
            }
        }
    })?;
    let work = store.codec_work();
    let proof = ManifestSourceReceipt {
        source,
        metadata: NativeMetadata::from_view(metadata.view(), work).map_err(|e| e.to_string())?,
        roots: NativeRoots::from_view(roots.view(), work).map_err(|e| e.to_string())?,
        manifest_value: manifest.value.clone(),
        closure_units: manifest.closure_units.take(),
        batch_id: TextValue::Plain(NativeValue::<String>::try_from_str(&manifest.batch_id, work)
            .map_err(|e| e.to_string())?),
    };
    Ok((manifest, workspace, proof))
}

pub(super) fn assemble_unpublished(
    store: &mut dyn NodeStore,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
) -> Result<UnpublishedManifest, CodecError> {
    assemble_unpublished_in_store(
        &mut crate::v3::audit_store::StoreView::generic(store),
        metadata,
        roots,
    )
}
pub(crate) fn assemble_unpublished_in_store(
    store: &mut crate::v3::audit_store::StoreView<'_, '_>,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
) -> Result<UnpublishedManifest, CodecError> {
    assemble_unpublished_view_in_store(store, metadata.view(), roots.view())
}
/// 明确normal输出适配器，借用相同输入核；不创建临时normal元数据DTO。
pub(crate) fn assemble_unpublished_view_in_store(
    store: &mut crate::v3::audit_store::StoreView<'_, '_>,
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
) -> Result<UnpublishedManifest, CodecError> {
    #[cfg(test)]
    let (sql, verify) = (
        super::content_store::sql_reads_on_thread(),
        ASSEMBLE_VERIFY_SQL_READS.with(std::cell::Cell::get),
    );
    let body = assemble_manifest_body(store, metadata, roots)?;
    #[cfg(test)]
    ASSEMBLE_CALL_READS.with(|calls| {
        calls.borrow_mut().push((
            super::content_store::sql_reads_on_thread() - sql,
            ASSEMBLE_VERIFY_SQL_READS.with(std::cell::Cell::get) - verify,
        ))
    });
    Ok(UnpublishedManifest {
        value: body.value,
        #[cfg(feature = "s_incremental")]
        closure_units: body.closure_units,
        batch_id: TextValue::Plain(store.native_string(batch_text(&body.identity).as_str())?),
    })
}
#[cfg(test)]
thread_local! {
    static ASSEMBLE_CALL_READS: std::cell::RefCell<Vec<(u64, u64)>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(test)]
thread_local! { static ASSEMBLE_VERIFY_SQL_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
/// #1488 S3 探针：取走本线程各次 assemble_manifest_body 的 SQL 节点读（总量, 其中 §5.4 三根全闭包核验）。
#[cfg(test)]
pub(crate) fn take_assemble_call_reads_for_test() -> Vec<(u64, u64)> {
    ASSEMBLE_CALL_READS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}
struct ManifestBody {
    value: Built,
    identity: NodeDigest,
    #[cfg(feature = "s_incremental")]
    closure_units: Option<s_content::closure_delta::VerifiedUnits>,
}
fn assemble_manifest_body(
    store: &mut crate::v3::audit_store::StoreView<'_, '_>,
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
) -> Result<ManifestBody, CodecError> {
    check_metadata_view(metadata)?;
    let generation = Decimal::new(metadata.generation);
    let frontier = Decimal::new(metadata.input_frontier);
    let attempt = Decimal::new(metadata.source_commit.attempt_index);
    // 同一次读集核完整闭包；路径算子只核触碰边界，不能替代这里的全闭包校验。
    #[cfg(test)]
    let sql = super::content_store::sql_reads_on_thread();
    let collect_units = {
        #[cfg(feature = "s_incremental")]
        { store.collect_manifest_units }
        #[cfg(not(feature = "s_incremental"))]
        { false }
    };
    #[cfg(feature = "s_incremental")]
    let closure_units = if collect_units {
        let units = s_content::closure_delta::verify_four_roots(store,
            [metadata.scope, roots.generated_state, roots.publication_template, roots.delta_template])?;
        require_kind(metadata.scope, Kind::Object, "scope")?;
        Some(units)
    } else { None };
    if !collect_units {
    #[cfg(test)]
    crate::v3::diagnostic_trace_event("manifest_verify_scope", None);
    store.verify_manifest_public(metadata.scope)?;
    require_kind(metadata.scope, Kind::Object, "scope")?;
    #[cfg(test)]
    crate::v3::diagnostic_trace_event("manifest_verify_generated", None);
    store.verify_manifest_public(roots.generated_state)?;
    #[cfg(test)]
    crate::v3::diagnostic_trace_event("manifest_verify_templates", None);
    store.verify_manifest_templates([roots.publication_template, roots.delta_template])?;
    }
    #[cfg(test)]
    crate::v3::diagnostic_trace_event("manifest_verify_complete", None);
    #[cfg(test)]
    ASSEMBLE_VERIFY_SQL_READS
        .with(|reads| reads.set(reads.get() + super::content_store::sql_reads_on_thread() - sql));
    let generated_fields = exact_object(store, roots.generated_state, GENERATED_FIELDS, true)?;
    let publication_fields =
        exact_object(store, roots.publication_template, SNAPSHOT_FIELDS, false)?;
    let delta_fields = exact_object(store, roots.delta_template, DELTA_FIELDS, false)?;

    let generated = &generated_fields;
    let publication = &publication_fields;
    let delta = &delta_fields;
    // 保留旧生成态的 JSON number；公开 snapshot/Delta 控制量则继续是规范十进制字符串。
    match_value(store, generated, "generation", number(&generation)?)?;
    match_value(store, generated, "input_frontier", number(&frontier)?)?;
    for root in [publication, delta] {
        match_value(store, root, "generation", text(generation.as_str()))?;
        match_value(store, root, "input_frontier", text(frontier.as_str()))?;
        match_value(store, root, "session_id", text(&metadata.session_id))?;
        require_kind(
            field(store, root, "seq_range")?.view(),
            Kind::Object,
            "seq_range",
        )?;
        require_kind(
            field(store, root, "catalog_run_status")?.view(),
            Kind::String,
            "catalog_run_status",
        )?;
        require_kind(
            field(store, root, "catalog_evidence")?.view(),
            Kind::Object,
            "catalog_evidence",
        )?;
        // 当前发布头由本批清单派生，不能预填旧批次或形成 hash 自引用。
        require_kind(
            field(store, root, "index_frontier")?.view(),
            Kind::TemplateBatch,
            "index_frontier",
        )?;
        let evidence = field(store, root, "catalog_evidence")?;
        let evidence_batch = store.lookup_field_ref(evidence.view(), "batch_id")?;
        require_kind(
            evidence_batch.view(),
            Kind::TemplateBatch,
            "catalog_evidence.batch_id",
        )?;
    }
    for root in [generated, publication, delta] {
        match_value(
            store,
            root,
            "catalog_revision",
            text(&metadata.catalog_revision),
        )?;
    }
    for root in [generated, publication] {
        match_value(store, root, "structure_cut", text(&metadata.structure_cut))?;
        match_value(store, root, "profile_id", text(&metadata.profile_id))?;
        match_value(store, root, "profile_hash", text(&metadata.profile_hash))?;
        match_digest(store, root, "scope", metadata.scope.digest()?)?;
    }
    match_value(
        store,
        generated,
        "rule_revision",
        text(&metadata.rule_revision),
    )?;
    match_value(store, delta, "base_cut", text(&metadata.base_cut))?;
    match_value(store, delta, "next_cut", text(&metadata.structure_cut))?;
    for key in ["seq_range", "catalog_run_status", "catalog_evidence"] {
        let expected = field(store, publication, key)?.digest()?;
        match_digest(store, delta, key, expected)?;
    }
    for (root, fields) in [
        (
            generated,
            &[
                "raw_events",
                "effective_events",
                "objects",
                "witnesses",
                "relations",
                "observations",
            ][..],
        ),
        (
            publication,
            &[
                "objects",
                "withdrawn_objects",
                "witnesses",
                "relations",
                "observations",
                "raw_history",
            ][..],
        ),
        (
            delta,
            &[
                "upserts",
                "withdrawals",
                "replaces",
                "witnesses",
                "relations",
                "observations",
                "raw_history_added",
            ][..],
        ),
    ] {
        for key in fields {
            require_kind(field(store, root, key)?.view(), Kind::Array, key)?;
        }
    }
    let source = &metadata.source_commit;
    let source_fields = [
        ("source_namespace", text(&source.source_namespace)),
        ("source_epoch", text(&source.source_epoch)),
        ("message_id", text(&source.message_id)),
        ("payload_hash", text(&source.payload_hash)),
        ("producer_id", text(&source.producer_id)),
        ("producer_epoch", text(&source.producer_epoch)),
        ("clock_event_id", text(&source.clock_event_id)),
        ("attempt_index", text(attempt.as_str())),
    ];
    let source_commit = VV::Object(&source_fields);
    let manifest_fields = [
        ("schema_revision", text("s-batch-manifest/1")),
        ("content_format", text(s_content::CONTENT_FORMAT)),
        ("protocol_revision", text("s-session/3")),
        ("session_id", text(&metadata.session_id)),
        ("session_generation", text(&metadata.session_generation)),
        ("writer_epoch", text(&metadata.writer_epoch)),
        ("generation", text(generation.as_str())),
        ("base_cut", text(&metadata.base_cut)),
        ("structure_cut", text(&metadata.structure_cut)),
        ("input_frontier", text(frontier.as_str())),
        ("source_commit", source_commit),
        ("clock_plan_hash", text(&metadata.clock_plan_hash)),
        ("semantic_commit_ns", text(&metadata.semantic_commit_ns)),
        ("profile_id", text(&metadata.profile_id)),
        ("profile_hash", text(&metadata.profile_hash)),
        ("rule_revision", text(&metadata.rule_revision)),
        ("catalog_revision", text(&metadata.catalog_revision)),
        ("scope", VV::Edge(metadata.scope.digest()?)),
        ("generated_state", VV::Edge(roots.generated_state.digest()?)),
        (
            "publication_template",
            VV::Edge(roots.publication_template.digest()?),
        ),
        ("delta_template", VV::Edge(roots.delta_template.digest()?)),
    ];
    let value = store.write_template_view(VV::Object(&manifest_fields))?;
    let identity = s_content::frame(0x21, &value.digest);
    Ok(ManifestBody { value, identity,
        #[cfg(feature = "s_incremental")]
        closure_units,
    })
}

fn text(value: &str) -> VV<'_> {
    VV::String(value)
}
fn number(value: &Decimal) -> Result<VV<'_>, CodecError> {
    Ok(VV::Number(BorrowedNumberToken::parse(value.as_str())?))
}
pub(crate) struct Decimal {
    bytes: [u8; 20],
    start: usize,
}
impl Decimal {
    pub(crate) fn new(value: i64) -> Self {
        let mut out = Self {
            bytes: [0; 20],
            start: 20,
        };
        let mut n = value.unsigned_abs();
        loop {
            out.start -= 1;
            out.bytes[out.start] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        if value < 0 {
            out.start -= 1;
            out.bytes[out.start] = b'-';
        }
        out
    }
    pub(crate) fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.bytes[self.start..]) }
    }
}
struct BatchText {
    bytes: [u8; 71],
}
fn batch_text(identity: &NodeDigest) -> BatchText {
    let mut bytes = [0; 71];
    bytes[..7].copy_from_slice(b"batch3-");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, b) in identity.iter().enumerate() {
        bytes[7 + 2 * i] = HEX[(b >> 4) as usize];
        bytes[8 + 2 * i] = HEX[(b & 15) as usize];
    }
    BatchText { bytes }
}
impl BatchText {
    fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
pub(crate) struct UnpublishedManifestOwner {
    pub value: Built,
    pub batch_id: TextOwner,
}
#[cfg(feature = "s_allocation_json_testonly")]
pub(crate) fn assemble_unpublished_owned_in_store(
    store: &mut crate::v3::audit_store::StoreView<'_, '_>,
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
) -> Result<UnpublishedManifestOwner, CodecError> {
    let budget = store.allocation_budget().ok_or_else(|| {
        s_content::capture_allocation::audit_json::static_error(
            s_content::capture_allocation::INCOMPLETE,
            "owned manifest必须来自实际capture预算",
        )
    })?;
    let body = assemble_manifest_body(store, metadata, roots)?;
    Ok(UnpublishedManifestOwner {
        value: body.value,
        batch_id: TextOwner::copy_from(batch_text(&body.identity).as_str(), &budget)?,
    })
}
fn context_error(detail: impl Into<String>) -> CodecError {
    CodecError::new("manifest_context", detail)
}

fn check_metadata(m: &ManifestMetadata) -> Result<(), CodecError> {
    check_metadata_view(m.view())
}
pub(crate) fn check_metadata_view(m: MetadataView<'_>) -> Result<(), CodecError> {
    if m.generation < 1 || m.input_frontier < -1 || m.source_commit.attempt_index < 0 {
        return Err(context_error(
            "代号须为正、输入前沿不得小于 -1、attempt_index 不得为负",
        ));
    }
    for (name, value) in [
        ("session_generation", &m.session_generation),
        ("writer_epoch", &m.writer_epoch),
        ("producer_epoch", &m.source_commit.producer_epoch),
    ] {
        positive(value, name).map_err(context_error)?;
    }
    nonnegative(&m.semantic_commit_ns, "semantic_commit_ns").map_err(context_error)?;
    for (name, value) in [
        ("session_id", &m.session_id),
        ("base_cut", &m.base_cut),
        ("structure_cut", &m.structure_cut),
        ("profile_id", &m.profile_id),
        ("rule_revision", &m.rule_revision),
        ("catalog_revision", &m.catalog_revision),
        ("source_namespace", &m.source_commit.source_namespace),
        ("source_epoch", &m.source_commit.source_epoch),
        ("message_id", &m.source_commit.message_id),
        ("producer_id", &m.source_commit.producer_id),
        ("clock_event_id", &m.source_commit.clock_event_id),
    ] {
        if value.is_empty() {
            return Err(context_error(format!("{name} 不得为空")));
        }
    }
    for (name, value) in [
        ("profile_hash", &m.profile_hash),
        ("clock_plan_hash", &m.clock_plan_hash),
        ("payload_hash", &m.source_commit.payload_hash),
    ] {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(context_error(format!(
                "{name} 必须是 SHA256 的 64 位小写十六进制"
            )));
        }
    }
    Ok(())
}

fn require_kind(value: s_content::RefView<'_>, kind: Kind, name: &str) -> Result<(), CodecError> {
    if value.kind() != kind {
        return Err(CodecError::new(
            "manifest_schema",
            format!("{name} 应为 {}", kind.name()),
        ));
    }
    Ok(())
}

fn field(
    store: &mut (impl CodecStore + ?Sized),
    root: &Fields,
    name: &str,
) -> Result<RefValue, CodecError> {
    root.get_admitted(store, name)?
        .ok_or_else(|| CodecError::new("manifest_schema", format!("根缺字段{name}")))
}

fn exact_object(
    store: &mut (impl CodecStore + ?Sized),
    root: s_content::RefView<'_>,
    fields: &'static [&'static str],
    allow_axes: bool,
) -> Result<Fields, CodecError> {
    require_kind(root, Kind::Object, "根")?;
    let n = fields.len() as u64;
    let has_axes = allow_axes && root.extent() == Some(n + 1);
    if root.extent() != Some(n) && !has_axes {
        return Err(CodecError::new(
            "manifest_schema",
            "顶层 exact 字段数量不符",
        ));
    }
    let mut names = [""; 21];
    names[..fields.len()].copy_from_slice(fields);
    let count = fields.len() + usize::from(has_axes);
    if has_axes {
        names[fields.len()] = "catalog_axes";
    }
    let values = store.lookup_fields_ref(root, &names[..count])?;
    if has_axes {
        require_kind(
            values
                .get_admitted(store, "catalog_axes")?
                .ok_or_else(|| context_error("根缺catalog_axes"))?
                .view(),
            Kind::Object,
            "catalog_axes",
        )?;
    }
    Ok(values)
}

fn match_digest(
    store: &mut (impl CodecStore + ?Sized),
    root: &Fields,
    key: &str,
    expected: NodeDigest,
) -> Result<(), CodecError> {
    if field(store, root, key)?.digest()? != expected {
        return Err(CodecError::new(
            "manifest_binding",
            format!("{key} 与当前发布上下文或相邻根不符"),
        ));
    }
    Ok(())
}

fn match_value(
    store: &mut (impl CodecStore + ?Sized),
    root: &Fields,
    key: &str,
    expected: VV<'_>,
) -> Result<(), CodecError> {
    let expected = store.write_value_view(expected)?.digest;
    match_digest(store, root, key, expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::{MemoryNodes, NumberToken, Value as V};
    use serde_json::{json, Value as J};

    #[cfg(feature = "s_incremental")]
    fn test_body(bytes: &[u8]) -> s_content::CodecVec<u8> {
        let mut body = s_content::CodecVec::default();
        body.try_extend_bytes(bytes).unwrap();
        body
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_manifest_metadata_and_roots_outlive_scope_and_drop_independently() {
        let (_, metadata, roots, _, _, _) = fixture();
        let budget = crate::v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let (owned_metadata, owned_roots) = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    Ok::<_, CodecError>((
                        NativeMetadata::from_view(metadata.view(), Some(work))?,
                        NativeRoots::from_view(roots.view(), Some(work))?,
                    ))
                })
                .unwrap();
            assert!(owned_metadata.matches(metadata.view()));
            assert!(same_roots(owned_roots.view(), roots.view()));
            let both = budget.live();
            assert!(both > baseline);
            drop(owned_metadata);
            assert!(budget.live() < both);
            assert!(budget.live() > baseline, "三个根仍持自己的原许可");
            assert!(same_roots(owned_roots.view(), roots.view()));
            drop(owned_roots);
            assert_eq!(budget.live(), baseline);
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_manifest_copy_failure_preserves_inputs_and_does_not_validate_early() {
        let (_, metadata, mut roots, _, _, _) = fixture();
        let budget = crate::v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let before = roots.clone();
        let failure = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let first = roots.generated_state.format.len() + roots.generated_state.root.len();
                let _held = budget
                    .reserve(1024 * 1024 - budget.live() - first - 1)
                    .unwrap();
                NativeRoots::from_view(roots.view(), Some(work)).map(|_| ())
            })
            .unwrap_err();
        assert!(matches!(
            failure,
            s_content::capture_allocation::scratch::WorkError::Resource(_)
        ));
        assert_eq!(roots, before);
        assert_eq!(budget.live(), baseline, "中途失败必须析构已复制的第一根");
        let failure = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let _held = budget
                    .reserve(1024 * 1024 - budget.live() - metadata.session_id.len() - 1)
                    .unwrap();
                NativeMetadata::from_view(metadata.view(), Some(work)).map(|_| ())
            })
            .unwrap_err();
        assert!(matches!(
            failure,
            s_content::capture_allocation::scratch::WorkError::Resource(_)
        ));
        assert_eq!(metadata.session_id, "manifest-test");
        assert_eq!(budget.live(), baseline);
        roots.generated_state.root = "invalid-transport-root".into();
        let copied = budget
            .codec_work()
            .unwrap()
            .run(|work| NativeRoots::from_view(roots.view(), Some(work)))
            .unwrap();
        assert!(
            same_roots(copied.view(), roots.view()),
            "复制阶段不新增digest验证"
        );
        assert!(copied.view().generated_state.digest().is_err());
        drop(copied);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_manifest_recorded_body_and_puts_keep_original_permits_after_scope() {
        use s_content::NodeSink;
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL); INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
        let tx = conn.transaction().unwrap();
        super::super::content_store::init_schema(&tx).unwrap();
        let budget = crate::v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let tag = s_content::tag::BYTES_CHUNK;
        let digest = s_content::frame(tag, &[7, 8, 9]);
        let (mut originals, puts, tree_permit) = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut inner =
                    super::super::content_store::TxStore::new(&tx)?.with_work(Some(work));
                let mut recording = ManifestRecordingStore {
                    inner: &mut inner,
                    allowance: 1024 * 1024,
                    certificate_cost: 0,
                    source_cost: 0,
                    originals: Some(Default::default()),
                    retained_capacity: 0,
                    accepted_entries: 0,
                    accepted_body_len: 0,
                    puts: Default::default(),
                    tree_permit: None,
                };
                recording.put(digest, tag, &[7, 8, 9])?;
                Ok::<_, CodecError>((
                    recording.originals.take().unwrap(),
                    recording.puts,
                    recording.tree_permit,
                ))
            })
            .unwrap();
        assert_eq!(&puts[..], &[digest]);
        let held = budget.live();
        assert!(held > baseline);
        let (_, body) = originals.remove(&digest).unwrap();
        drop(originals);
        drop(tree_permit);
        let remaining = budget.live();
        assert!(remaining < held);
        assert_eq!(&body[..], &[7, 8, 9]);
        drop(puts);
        assert!(budget.live() < remaining);
        assert!(budget.live() > baseline, "移出原件的正文仍持原许可");
        drop(body);
        assert_eq!(budget.live(), baseline);
    }

    // #1488 0035：originals 树增量许可——每个新条目只并入一份单条上界，峰值不含整树重复许可。
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_manifest_originals_tree_permit_is_incremental_without_whole_tree_coexistence() {
        use s_content::NodeSink;
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL); INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
        let tx = conn.transaction().unwrap();
        super::super::content_store::init_schema(&tx).unwrap();
        let budget = crate::v3::audit_owner::native::Budget::new(4 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let tag = s_content::tag::BYTES_CHUNK;
        let per =
            crate::v3::audit_retention::tree_slots::<NodeDigest, (u8, s_content::CodecVec<u8>)>(1);
        let entries = 8usize;
        let tree_permit = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut inner =
                    super::super::content_store::TxStore::new(&tx)?.with_work(Some(work));
                let mut recording = ManifestRecordingStore {
                    inner: &mut inner,
                    allowance: 1024 * 1024,
                    certificate_cost: 0,
                    source_cost: 0,
                    originals: Some(Default::default()),
                    retained_capacity: 0,
                    accepted_entries: 0,
                    accepted_body_len: 0,
                    puts: Default::default(),
                    tree_permit: None,
                };
                for index in 0..entries {
                    let body = [index as u8 + 10, 1, 2];
                    recording.put(s_content::frame(tag, &body), tag, &body)?;
                    let held = recording.tree_permit.as_ref().map_or(0, |p| p.bytes());
                    assert_eq!(held, (index + 1) * per, "树许可逐条累加");
                    assert!(
                        budget.peak() - budget.live() < per,
                        "峰值不得含整树重复许可：peak={} live={} per={per}",
                        budget.peak(),
                        budget.live()
                    );
                }
                assert_eq!(recording.originals.as_ref().unwrap().len(), entries);
                Ok::<_, CodecError>(recording.tree_permit.take())
            })
            .unwrap();
        assert_eq!(tree_permit.as_ref().map(|p| p.bytes()), Some(entries * per));
        drop(tree_permit);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_publication_material_owner_normal_puts_keep_base_and_read_targets_cold() {
        use s_content::{NodeSink, NodeSource};
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL); INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
        let mut tx = conn.transaction().unwrap();
        super::super::content_store::init_schema(&tx).unwrap();
        let tag = s_content::tag::BYTES_CHUNK;
        let a = s_content::frame(tag, &[7]);
        let b = s_content::frame(tag, &[8]);
        let c = s_content::frame(tag, &[9]);
        let mut seed = super::super::content_store::TxStore::new(&tx).unwrap();
        seed.put(a, tag, &[7]).unwrap();
        seed.put(b, tag, &[8]).unwrap();
        drop(seed);
        let lease = NativePublicationLease::new(&mut tx).unwrap();
        let mut store = PublicationMaterialStore::new(&*lease.tx, lease.insert_can_preserve);
        let originals = std::collections::BTreeMap::from([
            (a, (tag, test_body(&[7]))),
            (b, (tag, test_body(&[8]))),
        ]);
        let sql = super::super::content_store::sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        assert!(store.rebind_originals(&originals).unwrap());
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 2);
        assert_eq!(s_content::frame_calls_on_thread() - f, 2);
        store.originals = originals; // Test uses the same actual-SQL issuer as fixed resolve.
        let sql = super::super::content_store::sql_reads_on_thread();
        assert_eq!(store.checked_read(&a).unwrap().unwrap().body(), &[7]);
        assert_eq!(store.checked_read(&b).unwrap().unwrap().body(), &[8]);
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 0);
        let sql = super::super::content_store::sql_reads_on_thread();
        let f = s_content::frame_calls_on_thread();
        store.put(a, tag, &[7]).unwrap(); // Legal existing-node Noop is still SQL + two Fs.
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 1);
        assert_eq!(s_content::frame_calls_on_thread() - f, 2);
        let sql = super::super::content_store::sql_reads_on_thread();
        assert_eq!(store.checked_read(&a).unwrap().unwrap().body(), &[7]);
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 1);
        assert!(!store.originals.contains_key(&a));
        let sql = super::super::content_store::sql_reads_on_thread();
        store.put(c, tag, &[9]).unwrap(); // Legal controlled insert preserves the other base.
        assert!(store.originals.contains_key(&b));
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 1);
        let sql = super::super::content_store::sql_reads_on_thread();
        assert_eq!(store.checked_read(&c).unwrap().unwrap().body(), &[9]);
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 1);
        let sql = super::super::content_store::sql_reads_on_thread();
        assert_eq!(store.checked_read(&b).unwrap().unwrap().body(), &[8]);
        assert_eq!(super::super::content_store::sql_reads_on_thread() - sql, 0);
        drop(store);
        drop(lease);
        let sql = super::super::content_store::sql_reads_on_thread();
        let mut fresh = super::super::content_store::TxStore::new(&tx).unwrap();
        for (digest, body) in [(a, &[7][..]), (b, &[8][..]), (c, &[9][..])] {
            assert_eq!(fresh.checked_read(&digest).unwrap().unwrap().body(), body);
        }
        assert_eq!(
            super::super::content_store::sql_reads_on_thread() - sql,
            3,
            "租约结束后所有目标重新来自实际 SQL"
        );
    }

    fn cv(value: &J) -> V {
        match value {
            J::Null => V::Null,
            J::Bool(v) => V::Bool(*v),
            J::Number(v) => V::Number(NumberToken::parse(&v.to_string()).unwrap()),
            J::String(v) => V::String(v.clone()),
            J::Array(v) => V::Array(v.iter().map(cv).collect()),
            J::Object(v) => V::Object(v.iter().map(|(k, v)| (k.clone(), cv(v))).collect()),
        }
    }

    fn fixture() -> (MemoryNodes, ManifestMetadata, ManifestRoots, J, J, J) {
        let mut store = MemoryNodes::new();
        let scope = json!({"structure":"CompleteCut","economic":"not_started"});
        let scope_ref = s_content::write_value(&mut store, &cv(&scope))
            .unwrap()
            .content_ref();
        let metadata = ManifestMetadata {
            session_id: "manifest-test".into(),
            session_generation: "1".into(),
            writer_epoch: "2".into(),
            generation: 2,
            base_cut: "cut-1".into(),
            structure_cut: "cut-2".into(),
            input_frontier: 1,
            source_commit: SourceCommit {
                source_namespace: "fixture.feed".into(),
                source_epoch: "fixture-epoch".into(),
                message_id: "message-2".into(),
                payload_hash: "a".repeat(64),
                producer_id: "fixture-producer".into(),
                producer_epoch: "2".into(),
                clock_event_id: "event-2".into(),
                attempt_index: 0,
            },
            clock_plan_hash: "b".repeat(64),
            semantic_commit_ns: "24".into(),
            profile_id: "fixture-profile".into(),
            profile_hash: "c".repeat(64),
            rule_revision: "fixture-rule".into(),
            catalog_revision: "fixture-catalog".into(),
            scope: scope_ref,
        };
        let generated = json!({
            "generation":2,"structure_cut":"cut-2","input_frontier":1,
            "catalog_revision":"fixture-catalog","profile_id":"fixture-profile",
            "profile_hash":"c".repeat(64),"rule_revision":"fixture-rule", "scope":scope,
            "raw_events":[],"effective_events":[],"objects":[],"witnesses":[],
            "relations":[],"observations":[]
        });
        let publication = json!({
            "session_id":"manifest-test","generation":"2","structure_cut":"cut-2",
            "catalog_revision":"fixture-catalog","scope":scope,"index_frontier":null,
            "profile_id":"fixture-profile","profile_hash":"c".repeat(64),
            "history_mode":"RecomputedWithRevision","as_of_generation":null,
            "objects":[],"withdrawn_objects":[],"witnesses":[],"relations":[],
            "observations":[],"raw_history":[],"input_frontier":"1",
            "seq_range":{"from":"1","to":"1"},"catalog_run_status":"not_run",
            "catalog_evidence":{"batch_id":null}
        });
        let delta = json!({
            "generation":"2","session_id":"manifest-test","catalog_revision":"fixture-catalog",
            "base_cut":"cut-1","next_cut":"cut-2","seq_range":{"from":"1","to":"1"},
            "index_frontier":null,"input_frontier":"1","catalog_run_status":"not_run",
            "catalog_evidence":{"batch_id":null},"upserts":[],"withdrawals":[],
            "replaces":[],"witnesses":[],"relations":[],"observations":[],"raw_history_added":[]
        });
        let roots = roots(&mut store, &generated, &publication, &delta);
        (store, metadata, roots, generated, publication, delta)
    }

    fn template(value: &J) -> V {
        let V::Object(mut fields) = cv(value) else {
            panic!("fixture object")
        };
        for (key, value) in &mut fields {
            if key == "index_frontier" {
                *value = V::BatchSelf;
            }
            if key == "catalog_evidence" {
                *value = V::Object(vec![("batch_id".into(), V::BatchSelf)]);
            }
        }
        V::Object(fields)
    }

    fn roots(store: &mut MemoryNodes, generated: &J, publication: &J, delta: &J) -> ManifestRoots {
        ManifestRoots {
            generated_state: s_content::write_value(store, &cv(generated))
                .unwrap()
                .content_ref(),
            publication_template: s_content::write_template(store, &template(publication))
                .unwrap()
                .content_ref(),
            delta_template: s_content::write_template(store, &template(delta))
                .unwrap()
                .content_ref(),
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn expected_text_bytes(v: MetadataView<'_>) -> usize {
        let source = v.source_commit;
        [
            v.session_id,
            v.session_generation,
            v.writer_epoch,
            v.base_cut,
            v.structure_cut,
            v.clock_plan_hash,
            v.semantic_commit_ns,
            v.profile_id,
            v.profile_hash,
            v.rule_revision,
            v.catalog_revision,
            source.source_namespace,
            source.source_epoch,
            source.message_id,
            source.payload_hash,
            source.producer_id,
            source.producer_epoch,
            source.clock_event_id,
        ]
        .iter()
        .map(|v| v.len())
        .sum()
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_metadata_direct_utf8_clone_refusal_move_and_drop_preserve_same_ledger() {
        let (_nodes, metadata, roots, _, _, _) = fixture();
        let payload_bytes = expected_text_bytes(metadata.view());
        let calibration = OwnedBudget::new(10000);
        let measured = MetaOwner::from_view(metadata.view(), &calibration).unwrap();
        let bytes = calibration.snapshot().live_requested_layout_bytes;
        assert!(bytes >= payload_bytes);
        drop(measured);
        assert_eq!(calibration.snapshot().live_requested_layout_bytes, 0);
        let budget = OwnedBudget::new(bytes * 2);
        let owned = MetaOwner::from_view(metadata.view(), &budget).unwrap();
        assert_eq!(format!("{owned:?}"), format!("{metadata:?}"));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes);
        assert_eq!(budget.snapshot().allocated_layouts, 18);
        check_metadata_view(owned.view()).unwrap();
        let owned_roots = RootsOwner::from_view(roots.view()).unwrap();
        assert_eq!(
            owned_roots.generated_state.root(),
            roots.generated_state.root
        );
        assert_eq!(
            owned_roots.publication_template.root(),
            roots.publication_template.root
        );
        assert_eq!(owned_roots.delta_template.root(), roots.delta_template.root);
        let clone = owned.try_clone().unwrap();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes * 2);
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes);
        let moved = clone;
        drop(moved);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let budget = OwnedBudget::new(bytes * 2 - 1);
        let owned = MetaOwner::from_view(metadata.view(), &budget).unwrap();
        let original = format!("{owned:?}");
        assert_eq!(
            owned.try_clone().unwrap_err().code(),
            s_content::capture_allocation::LIMIT
        );
        assert_eq!(format!("{owned:?}"), original);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes);
        assert!(budget.snapshot().peak_requested_layout_bytes < bytes * 2);
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let budget = OwnedBudget::new(bytes - 1);
        assert_eq!(
            MetaOwner::from_view(metadata.view(), &budget)
                .unwrap_err()
                .code(),
            s_content::capture_allocation::LIMIT
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        println!(
            "OWNED_METADATA_RECEIPT {}",
            serde_json::json!({"text_fields":18,"payload_bytes":payload_bytes,"owned_layout_bytes":bytes,"ref_fixed":4,"last_refusal":budget.allocation_request_probe(),"after_drop":budget.snapshot()})
        );
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn metadata_views_keep_scalar_hash_epoch_validation_and_error_precedence() {
        let (_nodes, mut metadata, _roots, _, _, _) = fixture();
        for case in 0..6 {
            let mut m = metadata.clone();
            match case {
                0 => m.generation = 0,
                1 => m.session_generation = "01".into(),
                2 => m.source_commit.producer_epoch = "0".into(),
                3 => m.profile_hash = "A".repeat(64),
                4 => m.source_commit.payload_hash = "a".repeat(63),
                _ => m.semantic_commit_ns = "-1".into(),
            };
            let expected = check_metadata(&m).unwrap_err();
            let b = OwnedBudget::new(10000);
            let owned = MetaOwner::from_view(m.view(), &b).unwrap();
            assert_eq!(check_metadata_view(owned.view()).unwrap_err(), expected);
            drop(owned);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
        metadata.generation = 0;
        metadata.profile_hash = "wrong".into();
        assert!(check_metadata_view(metadata.view())
            .unwrap_err()
            .detail()
            .contains("代号须为正"));
        for n in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert_eq!(Decimal::new(n).as_str(), n.to_string());
        }
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn r9a_manifest_scope_preserves_identity_and_accounts_outputs() {
        let (mut nodes, metadata, roots, _, _, _) = fixture();
        let expected = assemble_unpublished(&mut nodes, &metadata, &roots).unwrap();
        let capture = s_content::AuditCapture::new(nodes);
        let (_, stats) = crate::v3::audit_store::with_native_store_probe(
            &capture,
            16 * 1024 * 1024,
            |store, live| {
                let before = live();
                let actual =
                    assemble_unpublished_view_in_store(store, metadata.view(), roots.view())?;
                assert_eq!(actual.value, expected.value);
                assert_eq!(actual.batch_id, expected.batch_id);
                assert!(live() > before + actual.batch_id.len());
                let reference = store.native_built_ref(&actual.value)?;
                assert!(matches!(reference, RefValue::Plain(_)));
                let fields = store.lookup_subset_ref(
                    reference.view(),
                    &["source_commit", "scope", "generated_state"],
                )?;
                for key in ["source_commit", "scope", "generated_state"] {
                    assert!(matches!(
                        fields.get_admitted(store, key)?,
                        Some(RefValue::Plain(_))
                    ));
                }
                Ok::<_, CodecError>(())
            },
        );
        assert_eq!((stats.0, stats.2), (0, 0));
        assert!(stats.1 > 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_manifest_real_capture_body_matches_normal_identity_and_f() {
        let (nodes, metadata, roots, _, _, _) = fixture();
        let legacy = s_content::AuditCapture::new(nodes.clone());
        let legacy_budget = OwnedBudget::new(512 * 1024);
        let (ordinary, normal_f) = crate::v3::audit_store::with_owned_store_probe(
            &legacy,
            &legacy_budget,
            512,
            32768,
            |store| {
                let f = s_content::frame_calls_on_thread();
                let result = assemble_unpublished_in_store(store, &metadata, &roots).unwrap();
                (result, s_content::frame_calls_on_thread() - f)
            },
        );
        assert_eq!(legacy_budget.snapshot().live_requested_layout_bytes, 0);
        // Capture exactly the original fixture nodes; ordinary's extra compatible emissions are immaterial.
        let count = nodes.len();
        let body_bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let base =
            s_content::capture_allocation::CaptureSlab::requested_bytes(count, body_bytes).unwrap();
        let budget = OwnedBudget::new(base + 512 * 1024);
        let mut slab =
            s_content::capture_allocation::CaptureSlab::new(count, body_bytes, &budget).unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        let capture =
            s_content::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let metadata_owner = MetaOwner::from_view(metadata.view(), &budget).unwrap();
        let roots_owner = RootsOwner::from_view(roots.view()).unwrap();
        let owned = crate::v3::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            512,
            32768,
            |store| {
                let f = s_content::frame_calls_on_thread();
                let result = assemble_unpublished_owned_in_store(
                    store,
                    metadata_owner.view(),
                    roots_owner.view(),
                )
                .unwrap();
                let owned_f = s_content::frame_calls_on_thread() - f;
                assert_eq!(owned_f, normal_f, "同captured mode下F/读取核不得漂移");
                (result, owned_f)
            },
        );
        assert_eq!(owned.0.value.digest, ordinary.value.digest);
        assert_eq!(owned.0.value.kind, ordinary.value.kind);
        assert_eq!(owned.0.value.extent, ordinary.value.extent);
        assert_eq!(owned.0.batch_id.as_str(), ordinary.batch_id.as_str());
        println!(
            "OWNED_MANIFEST_RECEIPT {}",
            serde_json::json!({"normal_f":normal_f,"owned_f":owned.1,"root":ordinary.value.root(),"batch":ordinary.batch_id,"meter":budget.snapshot(),"whole_audit_allocation_admitted":false})
        );
        drop(owned);
        drop(metadata_owner);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn content_manifest_binds_complete_template_edges_without_descriptor_hashing() {
        let (mut store, metadata, roots, _, _, _) = fixture();
        let built = assemble_unpublished(&mut store, &metadata, &roots).unwrap();
        assert_eq!(built.value.extent, Some(21));
        // 由独立 Python 按批准的 F/M/V 配方与上述显式 fixture 计算；不调用 Rust 产期望。
        assert_eq!(
            built.value.root(),
            "sc1-69b379691366b15839155bdbf66cb52e8d83db3742db97dd38a99397a9ce5fd6"
        );
        assert_eq!(
            built.batch_id,
            "batch3-cf784d122da0fe22e82c1e1162224416da8be5c676bd14d8d442e4c0ab8b259a"
        );
        s_content::verify_template(&mut store, &built.value.content_ref()).unwrap();
        // 模板尚有 0x08：组装成功不能被当作可公开的已发布切面。
        assert_eq!(
            s_content::verify(&mut store, &built.value.content_ref())
                .unwrap_err()
                .code(),
            s_content::code::TEMPLATE_PLACEHOLDER
        );
    }

    #[test]
    fn content_manifest_rejects_missing_extra_and_wrong_collection_fields() {
        let (mut store, metadata, _, generated, publication, delta) = fixture();
        let mut missing = generated.clone();
        missing.as_object_mut().unwrap().remove("observations");
        let input = roots(&mut store, &missing, &publication, &delta);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());

        let mut extra = generated.clone();
        extra["unapproved_field"] = json!([]);
        let input = roots(&mut store, &extra, &publication, &delta);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());
        let mut extra = publication.clone();
        extra["q_projection_root"] = json!("not-a-manifest-input");
        let input = roots(&mut store, &generated, &extra, &delta);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());
        let mut extra = delta.clone();
        extra["batch_id"] = json!("not-a-manifest-input");
        let input = roots(&mut store, &generated, &publication, &extra);
        assert!(assemble_unpublished(&mut store, &metadata, &input).is_err());

        let mut wrong = generated.clone();
        wrong["observations"] = json!({});
        let input = roots(&mut store, &wrong, &publication, &delta);
        assert_eq!(
            assemble_unpublished(&mut store, &metadata, &input)
                .err()
                .unwrap()
                .code(),
            "manifest_schema"
        );
        let mut ohlc = generated;
        ohlc["catalog_axes"] = json!({});
        let input = roots(&mut store, &ohlc, &publication, &delta);
        assemble_unpublished(&mut store, &metadata, &input).unwrap();
    }

    #[test]
    fn content_manifest_rejects_mixed_generation_profile_scope_and_delta_header() {
        let (mut store, metadata, _, generated, publication, delta) = fixture();
        for (family, key, value) in [
            (0, "generation", json!(3)),
            (0, "generation", json!("2")),
            (0, "rule_revision", json!("different-rule")),
            (1, "profile_id", json!("different-profile")),
            (
                1,
                "scope",
                json!({"structure":"PartialCut","economic":"not_started"}),
            ),
            (2, "base_cut", json!("cut-0")),
            (2, "next_cut", json!("cut-3")),
            (2, "seq_range", json!({"from":"0","to":"1"})),
            (2, "catalog_run_status", json!("run")),
        ] {
            let (mut g, mut p, mut d) = (generated.clone(), publication.clone(), delta.clone());
            let target = match family {
                0 => &mut g,
                1 => &mut p,
                _ => &mut d,
            };
            target[key] = value;
            let input = roots(&mut store, &g, &p, &d);
            assert_eq!(
                assemble_unpublished(&mut store, &metadata, &input)
                    .err()
                    .unwrap()
                    .code(),
                "manifest_binding",
                "{family}.{key}"
            );
        }
    }

    #[test]
    fn content_manifest_binds_all_source_commit_fields_and_clock_epoch() {
        let (mut store, metadata, roots, _, _, _) = fixture();
        let original = assemble_unpublished(&mut store, &metadata, &roots)
            .unwrap()
            .batch_id;
        for field in 0..12 {
            let mut changed = metadata.clone();
            match field {
                0 => changed.source_commit.source_namespace.push_str("-changed"),
                1 => changed.source_commit.source_epoch.push_str("-changed"),
                2 => changed.source_commit.message_id.push_str("-changed"),
                3 => changed.source_commit.payload_hash = "d".repeat(64),
                4 => changed.source_commit.producer_id.push_str("-changed"),
                5 => changed.source_commit.producer_epoch = "3".into(),
                6 => changed.source_commit.clock_event_id.push_str("-changed"),
                7 => changed.source_commit.attempt_index = 1,
                8 => changed.clock_plan_hash = "e".repeat(64),
                9 => changed.semantic_commit_ns = "25".into(),
                10 => changed.writer_epoch = "3".into(),
                11 => changed.session_generation = "2".into(),
                _ => unreachable!(),
            }
            assert_ne!(
                assemble_unpublished(&mut store, &changed, &roots)
                    .unwrap()
                    .batch_id,
                original,
                "field {field}"
            );
        }
        let mut invalid = metadata.clone();
        invalid.source_commit.attempt_index = -1;
        assert_eq!(
            assemble_unpublished(&mut store, &invalid, &roots)
                .err()
                .unwrap()
                .code(),
            "manifest_context"
        );
        invalid = metadata;
        invalid.session_generation = "01".into();
        assert_eq!(
            assemble_unpublished(&mut store, &invalid, &roots)
                .err()
                .unwrap()
                .code(),
            "manifest_context"
        );
    }

    #[test]
    fn content_manifest_rejects_missing_nodes_and_generated_placeholders() {
        let (mut store, metadata, mut input, generated, _, _) = fixture();
        input.generated_state.root = format!("sc1-{}", "f".repeat(64));
        assert_eq!(
            assemble_unpublished(&mut store, &metadata, &input)
                .err()
                .unwrap()
                .code(),
            s_content::code::NODE_MISSING
        );
        let V::Object(mut fields) = cv(&generated) else {
            unreachable!()
        };
        fields
            .iter_mut()
            .find(|(key, _)| key == "objects")
            .unwrap()
            .1 = V::Array(vec![V::BatchSelf]);
        input.generated_state = s_content::write_template(&mut store, &V::Object(fields))
            .unwrap()
            .content_ref();
        assert_eq!(
            assemble_unpublished(&mut store, &metadata, &input)
                .err()
                .unwrap()
                .code(),
            s_content::code::TEMPLATE_PLACEHOLDER
        );
    }
}
