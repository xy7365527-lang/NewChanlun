//! #1394 C §5.4：从当前事务的独立出生索引许可本批占位，再定点解析。
//!
//! 调用顺序：核旧域完整原件 → 构造模板/manifest → 在同一 Commit 事务暂写独立索引
//! （真实 batch3 已知）→ 本模块 → 持久实际根、Delta、clock 和前沿 → commit。
//! 本模块不提交事务；任何错误必须使调用者回滚。返回值只证明本批占位和发布元数据，
//! 不能替代对象身份、来源、全部历史、Delta 方程或资源预算的完整业务核验。
//! S3 的 Delta 按已批准 §5.4 固定为真实变化；不调用旧域 `delta_storage::read_mode`，
//! 不从旧私有格式表推断协议。未接线的 S3 writer 必须生成此合同，不能复用 legacy-full 集合。
#![allow(dead_code)]

#[cfg(test)]
use super::meta_get_opt;
use super::{canonical_json, content_manifest, content_store, v2};
#[cfg(all(test, feature = "s_incremental"))]
use crate::staged_nodes;
use crate::v3::audit_store::HashReader;
use crate::v3::audit_store::{CodecRead, CodecStore, NativeAdmission};
use crate::v3::audit_store::{NativeSet, NativeTree};
use crate::v3::audit_store::{RefFields, RefValue};
use crate::v3::business_json;
use crate::v3::native_shared::NativeRc as Rc;
use content_manifest::Decimal;
#[cfg(test)]
use content_manifest::ManifestRoots;
use content_manifest::{
    ManifestMetadata, ManifestRootSource, MetadataView, RootsView, UnpublishedManifest,
};
use newchan_rust::s_content::{self as sc, path::PathStep, ContentRef, Kind, NodeDigest, NodeSink};
use newchan_rust::session_protocol::{parse_canonical_i64, strict_json, validate_envelope};
use newchan_rust::sqlite_checked::{CheckedRow, CheckedStatement};
use rusqlite::{OptionalExtension, Transaction};
use sc::capture_allocation::native_owned::NativeValue;
use sc::capture_allocation::{audit_json::JsonView, scratch::WorkScope};
#[cfg(test)]
use sc::Value as V;
use sc::ValueView as VV;
#[allow(
    unused_imports,
    reason = "#1488 再合 #1608：8组合实测无调用方，保留不删"
)]
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CStr;

#[derive(Debug)]
pub(super) struct ResolvedTemplates {
    pub snapshot: NativeValue<ContentRef>,
    pub delta: NativeValue<ContentRef>,
}

/// 同一占位许可核的构造结果，尚无Public闭包证明；不得用于发布或作为已核历史返回。
pub(super) struct PendingTemplates {
    pub snapshot: NativeValue<ContentRef>,
    pub delta: NativeValue<ContentRef>,
}

pub(super) fn resolve_current_births(
    tx: &Transaction<'_>,
    begin_token: &str,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
) -> Result<ResolvedTemplates, String> {
    resolve_current_births_inner(
        tx,
        begin_token,
        metadata,
        roots,
        manifest,
        None,
        #[cfg(all(test, feature = "s_incremental"))]
        None,
    )
}
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn resolve_current_births_staged(
    tx: &mut Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    stage: &staged_nodes::StageOwner<'_>,
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
    work: Option<&WorkScope>,
) -> Result<ResolvedTemplates, String> {
    stage.check_transaction(tx).map_err(err)?;
    if let Some(workspace) = workspace {
        // 冻结workspace只消费真实SQL。生成租约已经析构，再将本代模板闭包落库；
        // 之后才允许workspace取得独占publication租约，不以overlay冒充素材。
        // 租约内新写节点只进暂存，由 Commit 的 Final cut 只落可达闭包（与 cold staged 同字节）。
        let template_roots = [
            manifest.value.digest,
            roots.view().generated_state.digest().map_err(err)?,
            roots.view().publication_template.digest().map_err(err)?,
            roots.view().delta_template.digest().map_err(err)?,
            m.scope.digest().map_err(err)?,
        ];
        stage.flush_cut(tx, staged_nodes::FlushCut::Final(&template_roots))?;
        workspace.resolve_in_publication_staged(tx, token, m, roots, manifest, work, stage)
    } else {
        resolve_current_births_inner(tx, token, m, roots, manifest, work, Some(stage))
    }
}
#[cfg(feature = "s_incremental")]
pub(super) fn resolve_current_births_with_workspace(
    tx: &mut Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
) -> Result<ResolvedTemplates, String> {
    resolve_current_births_with_workspace_admitted(tx, token, m, roots, manifest, workspace, None)
}
#[cfg(feature = "s_incremental")]
pub(super) fn resolve_current_births_with_workspace_admitted(
    tx: &mut Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
    work: Option<&WorkScope>,
) -> Result<ResolvedTemplates, String> {
    let Some(workspace) = workspace else {
        return resolve_current_births_inner(
            tx,
            token,
            m,
            roots,
            manifest,
            work,
            #[cfg(test)]
            None,
        );
    };
    workspace.resolve_in_publication_admitted(tx, token, m, roots, manifest, work)
}
#[cfg(feature = "s_incremental")]
pub(super) fn current_publication_frontier(tx: &Transaction<'_>) -> Result<i64, String> {
    current_publication_frontier_admitted(tx, None)
}
#[cfg(feature = "s_incremental")]
pub(crate) fn current_publication_frontier_admitted(
    tx: &Transaction<'_>,
    work: Option<&WorkScope>,
) -> Result<i64, String> {
    parse_canonical_i64(
        &meta_admitted(tx, "last_advance_frontier", work)?,
        "last_advance_frontier",
    )
}

fn resolve_current_births_inner(
    tx: &Transaction<'_>,
    begin_token: &str,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    work: Option<&WorkScope>,
    #[cfg(all(test, feature = "s_incremental"))] stage: Option<&staged_nodes::StageOwner>,
) -> Result<ResolvedTemplates, String> {
    verify_current_begin_admitted(tx, begin_token, metadata.view(), work)?;
    #[cfg(all(test, feature = "s_incremental"))]
    let mut store = content_store::TxStore::new_with_stage(tx, stage)
        .map_err(err)?
        .with_work(work);
    #[cfg(not(all(test, feature = "s_incremental")))]
    let mut store = content_store::TxStore::new(tx)
        .map_err(err)?
        .with_work(work);
    let computed =
        content_manifest::assemble_unpublished(&mut store, metadata, roots).map_err(err)?;
    if computed.batch_id != manifest.batch_id
        || computed.value.digest != manifest.value.digest
        || computed.value.kind != manifest.value.kind
        || computed.value.extent != manifest.value.extent
    {
        return Err("IdentityConflict：manifest 与当前三个根/上下文不符".into());
    }
    let previous_frontier = parse_canonical_i64(
        &meta_admitted(tx, "last_advance_frontier", work)?,
        "last_advance_frontier",
    )?;
    resolve_indexed_templates(
        tx,
        &mut store,
        metadata,
        roots,
        &manifest.batch_id,
        previous_frontier,
    )
}

/// 发布和历史回读共用同一占位许可规则。历史调用方提供只读节点源及内存写层；
/// 独立索引按目标代投影，未来出生/撤回不得改变旧 cut。此口不授予新的 Commit 权限。
pub(super) fn resolve_indexed_templates(
    tx: &Transaction<'_>,
    store: &mut (impl CodecStore + ?Sized),
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    batch_id: &str,
    previous_frontier: i64,
) -> Result<ResolvedTemplates, String> {
    let result =
        resolve_indexed_templates_pending(tx, store, metadata, roots, batch_id, previous_frontier)?;
    store
        .verify_many_ref_fresh(
            [(&result.snapshot).into(), (&result.delta).into()],
            sc::Mode::Public,
        )
        .map_err(err)?;
    Ok(ResolvedTemplates {
        snapshot: result.snapshot,
        delta: result.delta,
    })
}

/// 历史只读调用可以把全部Pending根放入同一次Public核验；发布入口仍立即核两根。
pub(super) fn resolve_indexed_templates_pending(
    tx: &Transaction<'_>,
    store: &mut (impl CodecStore + ?Sized),
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    batch_id: &str,
    previous_frontier: i64,
) -> Result<PendingTemplates, String> {
    let result = resolve_indexed_templates_pending_view(
        tx,
        store,
        metadata.view(),
        roots.view(),
        batch_id,
        previous_frontier,
    )?;
    Ok(PendingTemplates {
        snapshot: store
            .native_plain_ref(result.snapshot.view())
            .map_err(err)?,
        delta: store.native_plain_ref(result.delta.view()).map_err(err)?,
    })
}
pub(crate) struct PendingRefTemplates {
    pub snapshot: RefValue,
    pub delta: RefValue,
}
#[cfg(test)]
fn normal_reference(r: sc::RefView<'_>) -> ContentRef {
    ContentRef {
        format: r.format().to_owned(),
        root: r.root().to_owned(),
        kind: r.kind(),
        extent: r.extent(),
    }
}
fn ref_value(store: &(impl CodecStore + ?Sized), r: sc::RefView<'_>) -> Result<RefValue, String> {
    // Storage choice is made before the work, never after an allocation failure.
    if store.allocation_budget().is_some() {
        #[cfg(feature = "s_allocation_json_testonly")]
        return Ok(RefValue::Owned(
            sc::Built {
                digest: r.digest().map_err(err)?,
                kind: r.kind(),
                extent: r.extent(),
            }
            .owned_ref(),
        ));
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        return Err(err("owned refs未启用TestOnly feature"));
    }
    store.native_plain_ref(r).map(RefValue::Plain).map_err(err)
}
pub(crate) fn resolve_indexed_templates_pending_view(
    tx: &Transaction<'_>,
    store: &mut (impl CodecStore + ?Sized),
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
    batch_id: &str,
    previous_frontier: i64,
) -> Result<PendingRefTemplates, String> {
    resolve_indexed_templates_pending_core(
        tx,
        store,
        metadata,
        roots,
        batch_id,
        previous_frontier,
        None,
        None,
    )
}

/// 仅当前Native历史审计能构造；寿命绑原始capture，不借overlay临时体。
pub(crate) struct NativePublicationMemo<'capture> {
    capture: &'capture sc::AuditCapture,
    fields: NativeTree<
        (
            NativeValue<String>,
            NativeValue<String>,
            &'static str,
            Option<u64>,
        ),
        Rc<RefFields<'static>>,
    >,
}
impl<'capture> NativePublicationMemo<'capture> {
    pub(crate) fn new(store: &HashReader<'capture>) -> Option<Self> {
        store.native_projection_capture().map(|capture| Self {
            capture,
            fields: NativeTree::default(),
        })
    }
    fn check_store(&self, store: &HashReader<'_>) -> Result<(), String> {
        if !store
            .native_projection_capture()
            .is_some_and(|c| std::ptr::eq(c, self.capture))
        {
            return Err(err("Native publication memo 与当前capture/审计权限不符"));
        }
        Ok(())
    }
    fn read_fields(
        &mut self,
        store: &mut (impl CodecStore + ?Sized),
        record: sc::RefView<'_>,
    ) -> Result<PublicationFields, String> {
        // 来源在读取前分派：原capture之外的overlay记录按原口读取，不复用结论。
        if !self.capture.contains(&record.digest().map_err(err)?) {
            return read_publication_fields(store, record).map(PublicationFields::Direct);
        }
        // 检查完整原件绑定与声明；不是只凭根存在放行。capture内同上下文的核已成功
        // 结果可复用，但失败不保留，也不转overlay以另一套判据重判。
        self.capture
            .verify_refview(record, sc::Mode::Template)
            .map_err(err)?;
        let key = (
            store.native_string(record.format()).map_err(err)?,
            store.native_string(record.root()).map_err(err)?,
            record.kind().name(),
            record.extent(),
        );
        if let Some(fields) = self.fields.get(&key) {
            return Ok(PublicationFields::Shared(Rc::clone(fields)));
        }
        let fields = read_publication_fields(store, record)?;
        // 单次DFS成功即已提取全部十字段；不记业务判断/失败，每cut原位重跑。
        let fields = Rc::try_new_shallow(fields, store.codec_work()).map_err(err)?;
        self.fields
            .insert_admitted(key, Rc::clone(&fields), store)
            .map_err(err)?;
        Ok(PublicationFields::Shared(fields))
    }
}
const PUBLICATION_FIELD_NAMES: &[&str] = &[
    "object_id",
    "kind",
    "object_revision",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
    "withdrawn_generation",
    "withdrawal_reason",
    "superseded_by",
    "lifecycle",
];
enum PublicationFields {
    Ordinary(std::rc::Rc<RefFields<'static>>),
    Direct(RefFields<'static>),
    Shared(Rc<RefFields<'static>>),
}
impl PublicationFields {
    fn get_admitted(
        &self,
        store: &(impl CodecRead + ?Sized),
        name: &str,
    ) -> Result<Option<RefValue>, String> {
        match self {
            Self::Direct(f) => f.get_admitted(store, name),
            Self::Shared(f) => f.get_admitted(store, name),
            Self::Ordinary(f) => f.get_admitted(store, name),
        }
        .map_err(err)
    }
    fn get(&self, name: &str) -> Option<sc::RefView<'_>> {
        match self {
            Self::Direct(f) => f.get(name),
            Self::Shared(f) => f.get(name),
            Self::Ordinary(f) => f.get(name),
        }
    }
}
#[cfg(test)]
thread_local! {
    static NATIVE_PUBLICATION_FIELDS_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
fn read_publication_fields(
    store: &mut (impl CodecStore + ?Sized),
    record: sc::RefView<'_>,
) -> Result<RefFields<'static>, String> {
    #[cfg(test)]
    NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.set(v.get() + 1));
    store
        .lookup_subset_ref(record, PUBLICATION_FIELD_NAMES)
        .map_err(err)
}
pub(crate) fn resolve_native_indexed_templates_pending_view(
    tx: &Transaction<'_>,
    store: &mut HashReader<'_>,
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
    batch_id: &str,
    previous_frontier: i64,
    memo: &mut NativePublicationMemo<'_>,
) -> Result<PendingRefTemplates, String> {
    memo.check_store(store)?;
    resolve_indexed_templates_pending_core(
        tx,
        store,
        metadata,
        roots,
        batch_id,
        previous_frontier,
        Some(memo),
        None,
    )
}
fn check_resolution_header(
    store: &mut (impl CodecStore + ?Sized),
    snapshot: &RefValue,
    delta: &RefValue,
    metadata: MetadataView<'_>,
    previous_frontier: i64,
) -> Result<(), String> {
    match_field_view(
        store,
        snapshot.view(),
        "history_mode",
        VV::String("RecomputedWithRevision"),
    )?;
    match_field_view(
        store,
        snapshot.view(),
        "as_of_generation",
        VV::Json(sc::capture_allocation::audit_json::JsonView::Null),
    )?;
    let from = previous_frontier
        .checked_add(1)
        .ok_or_else(|| err("seq_range 起点溢出"))?;
    if previous_frontier < -1 || from > metadata.input_frontier {
        return Err(err("seq_range 无实际新增输入"));
    }
    let from_text = Decimal::new(from);
    let to_text = Decimal::new(metadata.input_frontier);
    let seq_fields = [
        ("from", VV::String(from_text.as_str())),
        ("to", VV::String(to_text.as_str())),
    ];
    for root in [snapshot, delta] {
        match_field_view(store, root.view(), "seq_range", VV::Object(&seq_fields))?;
    }
    Ok(())
}

/// Consumes only history's opaque complete-load cut projection; never used by writer publish.
pub(crate) fn resolve_native_dual_templates_pending_view(
    tx: &Transaction<'_>,
    store: &mut HashReader<'_>,
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
    batch: &str,
    previous: i64,
    projection: &crate::v3::object_index::NativeCutProjection<'_, '_, '_>,
) -> Result<PendingRefTemplates, String> {
    projection.check_scope(tx, store, metadata, roots, batch, previous)?;
    let capture = store
        .native_projection_capture()
        .ok_or_else(|| err("双投影缺Native capture"))?;
    store
        .native_verify_original(capture, roots.publication_template, sc::Mode::Template)
        .map_err(err)?;
    store
        .native_verify_original(capture, roots.delta_template, sc::Mode::Template)
        .map_err(err)?;
    let mut snapshot = ref_value(store, roots.publication_template)?;
    let mut delta = ref_value(store, roots.delta_template)?;
    check_resolution_header(store, &snapshot, &delta, metadata, previous)?;
    // Field sets share the unchanged manifest exact_object kernel's canonical lists.
    use content_manifest::{DELTA_FIELDS as DELTA, SNAPSHOT_FIELDS as SNAPSHOT};
    for (root, names, replacements) in [
        (
            &mut snapshot,
            SNAPSHOT,
            &[
                ("objects", 0usize),
                ("withdrawn_objects", 1),
                ("observations", 3),
            ][..],
        ),
        (
            &mut delta,
            DELTA,
            &[("upserts", 2usize), ("observations", 4)][..],
        ),
    ] {
        let fields = store.lookup_fields_ref(root.view(), names).map_err(err)?;
        for (name, ordinal) in replacements {
            let actual = fields
                .get_admitted(store, name)
                .map_err(err)?
                .ok_or_else(|| err("模板缺数组字段"))?;
            let expected = &projection.arrays()[*ordinal].0;
            store
                .native_verify_original(capture, actual.view(), sc::Mode::Template)
                .map_err(err)?;
            if actual.kind() != Kind::Array
                || actual.extent() != expected.extent
                || actual.view().format() != sc::CONTENT_FORMAT
                || actual.root() != sc::FixedRoot::new(&expected.digest).as_str()
                || actual.kind() != expected.kind
            {
                return Err(err("真实Template数组与独立有序双投影不同"));
            }
        }
        let mut values = sc::CodecVec::default();
        store
            .native_vec_room(&mut values, names.len())
            .map_err(err)?;
        for name in names {
            let digest = if let Some((_, ordinal)) = replacements.iter().find(|(k, _)| k == name) {
                projection.arrays()[*ordinal].1.digest
            } else {
                fields
                    .get_admitted(store, name)
                    .map_err(err)?
                    .ok_or_else(|| err("模板缺字段"))?
                    .digest()
                    .map_err(err)?
            };
            values.try_push((*name, VV::Edge(digest))).map_err(err)?;
        }
        let built = store
            .write_template_view(VV::Object(&values))
            .map_err(err)?;
        *root = store.native_built_ref(&built).map_err(err)?;
    }
    for root in [&mut snapshot, &mut delta] {
        let frontier =
            PublicationPath::new(store, [sc::path::PathStepView::Field("index_frontier")])?;
        let evidence = PublicationPath::new(
            store,
            [
                sc::path::PathStepView::Field("catalog_evidence"),
                sc::path::PathStepView::Field("batch_id"),
            ],
        )?;
        for path in [&frontier.steps[..], &evidence.steps[..]] {
            *root = store.replace_ref(root.view(), path, batch).map_err(err)?;
        }
    }
    Ok(PendingRefTemplates { snapshot, delta })
}

fn resolve_indexed_templates_pending_core(
    tx: &Transaction<'_>,
    store: &mut (impl CodecStore + ?Sized),
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
    batch_id: &str,
    previous_frontier: i64,
    mut memo: Option<&mut NativePublicationMemo<'_>>,
    mut projection: Option<&mut PublicationProjection<'_, '_>>,
) -> Result<PendingRefTemplates, String> {
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_births_objects");
    let objects = object_births(tx, &metadata, store);
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_births_objects", &objects);
    let objects = objects?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_births_observations");
    let observations = observation_births(tx, &metadata, store);
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_births_observations", &observations);
    let observations = observations?;
    let mut snapshot = ref_value(store, roots.publication_template)?;
    let mut delta = ref_value(store, roots.delta_template)?;
    check_resolution_header(store, &snapshot, &delta, metadata, previous_frontier)?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_resolve_active");
    let resolved_family = resolve_objects(
        store,
        &mut snapshot,
        "objects",
        &objects,
        &metadata,
        batch_id,
        ObjectFamily::Active,
        memo.as_deref_mut(),
        projection.as_deref_mut(),
    );
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_resolve_active", &resolved_family);
    resolved_family?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_resolve_withdrawn");
    let resolved_family = resolve_objects(
        store,
        &mut snapshot,
        "withdrawn_objects",
        &objects,
        &metadata,
        batch_id,
        ObjectFamily::Withdrawn,
        memo.as_deref_mut(),
        projection.as_deref_mut(),
    );
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_resolve_withdrawn", &resolved_family);
    resolved_family?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_resolve_born");
    let resolved_family = resolve_objects(
        store,
        &mut delta,
        "upserts",
        &objects,
        &metadata,
        batch_id,
        ObjectFamily::Born,
        memo.as_deref_mut(),
        projection.as_deref_mut(),
    );
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_resolve_born", &resolved_family);
    resolved_family?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_resolve_obs_snapshot");
    let resolved_family = resolve_observations(
        store,
        &mut snapshot,
        &observations,
        &metadata,
        batch_id,
        false,
    );
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_resolve_obs_snapshot", &resolved_family);
    resolved_family?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_resolve_obs_delta");
    let resolved_family =
        resolve_observations(store, &mut delta, &observations, &metadata, batch_id, true);
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_resolve_obs_delta", &resolved_family);
    resolved_family?;
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::begin(tx, "ord_pub_replace");
    for root in [&mut snapshot, &mut delta] {
        let frontier =
            PublicationPath::new(store, [sc::path::PathStepView::Field("index_frontier")])?;
        let evidence = PublicationPath::new(
            store,
            [
                sc::path::PathStepView::Field("catalog_evidence"),
                sc::path::PathStepView::Field("batch_id"),
            ],
        )?;
        for path in [&frontier.steps[..], &evidence.steps[..]] {
            *root = store
                .replace_ref(root.view(), path, batch_id)
                .map_err(err)?;
        }
    }
    #[cfg(test)]
    crate::v3::diagnostic_s4_read_trace::end(tx, "ord_pub_replace", &Ok::<(), ()>(()));
    Ok(PendingRefTemplates { snapshot, delta })
}

fn err(e: impl std::fmt::Display) -> String {
    format!("PublicationInvalid：{e}")
}
#[cfg(test)]
fn field(s: &str) -> PathStep {
    PathStep::Field(s.into())
}
#[cfg(test)]
fn meta(tx: &Transaction<'_>, key: &str) -> Result<String, String> {
    meta_get_opt(tx, key)?.ok_or_else(|| err(format!("缺 meta.{key}")))
}
struct HashOnly;
impl NodeSink for HashOnly {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
#[cfg(test)]
fn text(s: impl ToString) -> V {
    V::String(s.to_string())
}
// #1394 独立摘要仍使用调用方的临时工作域，不保存正文或改变业务所有权。
struct ScopedHashOnly<'a>(Option<&'a sc::capture_allocation::scratch::WorkScope>);
impl NodeSink for ScopedHashOnly<'_> {
    fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.0
    }
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
fn hash_view(store: &(impl CodecRead + ?Sized), v: VV<'_>) -> Result<NodeDigest, String> {
    let result = match store.allocation_budget() {
        #[cfg(feature = "s_allocation_json_testonly")]
        Some(b) => sc::write_value_view_for_allocation_prototype(&mut HashOnly, v, &b),
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        Some(_) => return Err(err("owned hash未启用TestOnly feature")),
        None => sc::write_value_view(&mut ScopedHashOnly(store.codec_work()), v),
    };
    result.map(|v| v.digest).map_err(err)
}
fn match_field_view(
    store: &mut (impl CodecStore + ?Sized),
    root: sc::RefView<'_>,
    key: &str,
    value: VV<'_>,
) -> Result<(), String> {
    let actual = store
        .lookup_field_ref(root, key)
        .map_err(err)?
        .digest()
        .map_err(err)?;
    if actual != hash_view(store, value)? {
        return Err(err(format!("{key} 与独立记录不同")));
    }
    Ok(())
}
// 固定 SELECT 和原列类型判据；checked 行先拒 NOMEM，业务文本随后才准入复制。
fn query_one<T>(
    tx: &Transaction<'_>,
    sql: &CStr,
    bind: impl FnOnce(&mut CheckedStatement<'_>) -> rusqlite::Result<()>,
    read: impl FnOnce(&mut CheckedRow<'_>) -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    let mut statement = CheckedStatement::prepare(tx, sql)?;
    bind(&mut statement)?;
    let value = {
        let mut row = statement
            .next()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        read(&mut row)?
    };
    statement.finalize()?;
    Ok(value)
}
fn row_text_admitted(
    row: &mut CheckedRow<'_>,
    index: usize,
    admission: &(impl CodecRead + ?Sized),
) -> rusqlite::Result<NativeValue<String>> {
    let text = row.text(index)?;
    admission.native_string(text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(e))
    })
}
fn row_optional_text_admitted(
    row: &mut CheckedRow<'_>,
    index: usize,
    admission: &(impl CodecRead + ?Sized),
) -> rusqlite::Result<Option<NativeValue<String>>> {
    row.optional_text(index)?
        .map(|text| {
            admission.native_string(text).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    index,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
        .transpose()
}
fn meta_optional_admitted(
    tx: &Transaction<'_>,
    key: &str,
    work: Option<&WorkScope>,
) -> Result<Option<NativeValue<String>>, String> {
    query_one(
        tx,
        c"SELECT value FROM meta WHERE key = ?1",
        |statement| statement.bind_text(1, key),
        |row| row_text_admitted(row, 0, &NativeAdmission(work)),
    )
    .optional()
    .map_err(|e| format!("读 meta.{key} 失败：{e}"))
}
fn meta_admitted(
    tx: &Transaction<'_>,
    key: &str,
    work: Option<&WorkScope>,
) -> Result<NativeValue<String>, String> {
    meta_optional_admitted(tx, key, work)?.ok_or_else(|| err(format!("缺 meta.{key}")))
}
fn generation_admitted(
    tx: &Transaction<'_>,
    store: &(impl CodecRead + ?Sized),
) -> Result<i64, String> {
    let value = meta_optional_admitted(tx, "generation", store.codec_work())?
        .ok_or_else(|| "StorageUnavailable：meta.generation 缺失".to_owned())?;
    parse_canonical_i64(&value, "meta.generation").map_err(|e| format!("StorageUnavailable：{e}"))
}
fn strict_json_admitted(
    raw: &str,
    admission: &NativeAdmission<'_>,
) -> Result<NativeValue<serde_json::Value>, String> {
    #[cfg(feature = "s_incremental")]
    let permit = admission
        .native_reservation(
            crate::v3::audit_owner::native::JsonShape::scan(raw.as_bytes()).parse_peak(),
        )
        .map_err(err)?;
    #[cfg(not(feature = "s_incremental"))]
    let permit: Option<sc::capture_allocation::scratch::NativeReservation> =
        if admission.0.is_some() {
            return Err(err(admission
                .0
                .unwrap()
                .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)));
        } else {
            None
        };
    let value = strict_json(raw.as_bytes())?;
    Ok(match permit {
        Some(permit) => permit.hold(value),
        None => NativeValue::without_reservation(value),
    })
}
fn canonical_admitted(
    value: &serde_json::Value,
    admission: &NativeAdmission<'_>,
) -> Result<NativeValue<String>, String> {
    #[cfg(feature = "s_incremental")]
    let permit = admission
        .native_reservation(
            crate::v3::audit_owner::native::JsonShape::of_value(value).canonical_peak(),
        )
        .map_err(err)?;
    #[cfg(not(feature = "s_incremental"))]
    let permit: Option<sc::capture_allocation::scratch::NativeReservation> =
        if admission.0.is_some() {
            return Err(err(admission
                .0
                .unwrap()
                .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)));
        } else {
            None
        };
    let value = canonical_json(value);
    Ok(match permit {
        Some(permit) => permit.hold(value),
        None => NativeValue::without_reservation(value),
    })
}
fn canonical_array_admitted(
    values: &[JsonView<'_>],
    admission: &NativeAdmission<'_>,
) -> Result<NativeValue<String>, String> {
    let owner = business_json::json_from_view_admitted(admission, JsonView::ArraySlice(values))?;
    match owner {
        business_json::JsonOwner::Plain(value) => canonical_admitted(&value, admission),
        #[cfg(feature = "s_allocation_json_testonly")]
        business_json::JsonOwner::Owned(_) => Err(err("Native准入不可擦除Owned JSON")),
    }
}

/// 冻结标量和当前 Begin 必须来自同一事务；不调用只认识 v2/旧 batch 的根审计器。
/// 旧可达历史与全部来源审计是调用者在本事务写入前的独立义务，本函数不为它们背书。
pub(super) fn verify_current_begin(
    tx: &Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
) -> Result<(), String> {
    verify_current_begin_view(tx, token, m.view())
}
pub(crate) fn verify_current_begin_view(
    tx: &Transaction<'_>,
    token: &str,
    m: MetadataView<'_>,
) -> Result<(), String> {
    verify_current_begin_admitted(tx, token, m, None)
}
pub(crate) fn verify_current_begin_admitted(
    tx: &Transaction<'_>,
    token: &str,
    m: MetadataView<'_>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let admission = NativeAdmission(work);
    for (key, expected) in [
        ("protocol_revision", "s-session/3"),
        ("session_id", m.session_id),
        ("writer_epoch", m.writer_epoch),
        ("structure_cut", m.base_cut),
        ("profile_id", m.profile_id),
        ("profile_hash", m.profile_hash),
        ("catalog_revision", m.catalog_revision),
        ("rule_revision", m.rule_revision),
        ("begin_token", token),
    ] {
        if meta_admitted(tx, key, work)?.as_str() != expected {
            return Err(err(format!("当前 {key} 不符")));
        }
    }
    if m.generation.checked_sub(1)
        != Some(parse_canonical_i64(
            &meta_admitted(tx, "generation", work)?,
            "generation",
        )?)
        || m.structure_cut
            != admission
                .native_format(format_args!("cut-{}", m.generation))
                .map_err(err)?
                .as_str()
        || meta_admitted(tx, "advance_state", work)?
            != admission
                .native_format(format_args!(
                    "begun:{token}:{}:{}",
                    m.generation, m.input_frontier
                ))
                .map_err(err)?
    {
        return Err("StaleWriter：当前 Begin 所有权/代号/cut 不符".into());
    }
    let scope_raw = meta_admitted(tx, "scope", work)?;
    let scope = strict_json_admitted(&scope_raw, &admission)?;
    if hash_view(&admission, VV::Json(scope.as_ref().into()))? != m.scope.digest().map_err(err)? {
        return Err(err("scope 与冻结域不同"));
    }
    let frontier = query_one(
        tx,
        c"SELECT COALESCE(MAX(seq),-1) FROM raw_events",
        |_| Ok(()),
        |r| r.integer(0),
    )
    .map_err(err)?;
    if frontier != m.input_frontier {
        return Err("StaleWriter：输入前沿已变化".into());
    }
    let (protocol,inc,clock_hash,clock_raw,next_ordinal,phase_frontier):(NativeValue<String>,NativeValue<String>,NativeValue<String>,NativeValue<String>,i64,NativeValue<String>)=query_one(tx,
        c"SELECT protocol_revision,session_generation,clock_plan_hash,clock_plan_json,next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1", |_| Ok(()),
        |r|Ok((row_text_admitted(r,0,&admission)?,row_text_admitted(r,1,&admission)?,row_text_admitted(r,2,&admission)?,row_text_admitted(r,3,&admission)?,r.integer(4)?,row_text_admitted(r,5,&admission)?))).map_err(err)?;
    let clock_value = strict_json_admitted(&clock_raw, &admission)?;
    // ClockPlan::parse 的规则核使用栈上 FixedHash，唯一拥有的摘要文本为 64 字节。
    let _clock_hash_permit = admission.native_reservation(64).map_err(err)?;
    let clock = clock_value.try_map(v2::ClockPlan::parse)?;
    if protocol.as_str() != "s-session/3"
        || inc.as_str() != m.session_generation
        || clock_hash.as_str() != m.clock_plan_hash
        || clock.hash != clock_hash.as_str()
        || canonical_admitted(&clock.value, &admission)?.as_str() != clock_raw.as_str()
    {
        return Err(err("协议/会话代号/clock 冻结原件不同"));
    }
    let s = &m.source_commit;
    let (ph,raw,accepted,status,first,cid,attempts,receipt):(NativeValue<String>,NativeValue<String>,i64,NativeValue<String>,Option<i64>,NativeValue<String>,i64,NativeValue<String>)=query_one(tx,
        c"SELECT payload_hash,canonical_envelope,accepted_seq,status,first_published_generation,clock_event_id,attempt_count,receipt_id FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",
        |statement| { statement.bind_text(1,s.source_namespace)?; statement.bind_text(2,s.source_epoch)?; statement.bind_text(3,s.message_id) },|r|Ok((row_text_admitted(r,0,&admission)?,row_text_admitted(r,1,&admission)?,r.integer(2)?,row_text_admitted(r,3,&admission)?,r.optional_integer(4)?,row_text_admitted(r,5,&admission)?,r.integer(6)?,row_text_admitted(r,7,&admission)?))).map_err(err)?;
    let accepted_receipt = query_one(
        tx,
        c"SELECT receipt_id FROM raw_events WHERE seq=?1",
        |statement| statement.bind_integer(1, accepted),
        |r| row_text_admitted(r, 0, &admission),
    )
    .map_err(err)?;
    if receipt != accepted_receipt {
        return Err(err("message 的接纳收据与实际源行不同"));
    }
    let envelope = strict_json_admitted(&raw, &admission)?
        .try_map(|value| validate_envelope(value, "s-session/3"))?;
    if canonical_admitted(&envelope, &admission)?.as_str() != raw.as_str()
        || ph.as_str() != s.payload_hash
        || cid.as_str() != s.clock_event_id
        || accepted > frontier
        || accepted < 0
        || status.as_str() != "AcceptedPending"
        || first.is_some()
        || attempts.checked_sub(1) != Some(s.attempt_index)
    {
        return Err(err("source_commit 不是当前已接纳未发布的 attempt"));
    }
    for (key, expected) in [
        ("session_id", &m.session_id),
        ("session_generation", &m.session_generation),
        ("source_namespace", &s.source_namespace),
        ("source_epoch", &s.source_epoch),
        ("message_id", &s.message_id),
        ("payload_hash", &s.payload_hash),
        ("producer_id", &s.producer_id),
        ("producer_epoch", &s.producer_epoch),
    ] {
        if envelope[key].as_str() != Some(*expected) {
            return Err(err(format!("封存 envelope.{key} 不符")));
        }
    }
    if envelope["payload"]["clock_event_id"].as_str() != Some(s.clock_event_id) {
        return Err(err("envelope clock_event_id 不符"));
    }
    let (begin_ns,message_key,ordinal,generation):(NativeValue<String>,NativeValue<String>,i64,i64)=query_one(tx,
        c"SELECT semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events WHERE clock_event_id=?1 AND phase='begin' AND attempt_index=?2",
        |statement| { statement.bind_text(1,&cid)?; statement.bind_integer(2,s.attempt_index) },|r|Ok((row_text_admitted(r,0,&admission)?,row_text_admitted(r,1,&admission)?,r.integer(2)?,r.integer(3)?))).map_err(err)?;
    let expected_key = canonical_array_admitted(
        &[
            JsonView::String(s.source_namespace),
            JsonView::String(s.source_epoch),
            JsonView::String(s.message_id),
        ],
        &admission,
    )?;
    let ordinal_text = Decimal::new(ordinal);
    let generation_text = Decimal::new(m.generation);
    let frontier_text = Decimal::new(m.input_frontier);
    let token_parts = [
        JsonView::String(m.session_id),
        JsonView::String(m.session_generation),
        JsonView::String(m.writer_epoch),
        JsonView::String(ordinal_text.as_str()),
        JsonView::String(m.base_cut),
        JsonView::String(generation_text.as_str()),
        JsonView::String(frontier_text.as_str()),
    ];
    // 共用协议规范字节/摘要核，借用数组与固定摘要不建立临时 JSON 树或字符串。
    let token_hash = newchan_rust::session_protocol::canonical_hash_view_with_owner(
        JsonView::ArraySlice(&token_parts),
        None,
    )?;
    let expected_token = admission
        .native_format(format_args!("begin-{}", token_hash.as_str()))
        .map_err(err)?;
    // #1769：System 的 Commit 读数在准备时取样一次并封进元数据；此处只核持久 Begin 与前沿。
    let commit_ns = if clock.is_system() {
        m.semantic_commit_ns
    } else {
        newchan_rust::session_protocol::clock_phase_view(
            (&clock.value).into(),
            &cid,
            "commit",
            s.attempt_index,
        )?
    };
    if ordinal < 1
        || ordinal.checked_add(1) != Some(next_ordinal)
        || generation != m.generation
        || message_key.as_str() != expected_key.as_str()
        || !newchan_rust::session_protocol::clock_persisted_matches(
            (&clock.value).into(),
            &cid,
            "begin",
            s.attempt_index,
            begin_ns.as_str(),
        )?
        || commit_ns != m.semantic_commit_ns
        || token != expected_token.as_str()
        || phase_frontier != begin_ns
        || parse_canonical_i64(&phase_frontier, "clock frontier")?
            > parse_canonical_i64(&commit_ns, "commit clock")?
    {
        return Err(err("Begin/clock/attempt 与本次发布不同"));
    }
    Ok(())
}

struct ObjectBirth {
    revision: i64,
    kind: NativeValue<String>,
    batch: NativeValue<String>,
    first: i64,
    cut: NativeValue<String>,
    published: i64,
    withdrawn: Option<i64>,
    reason: Option<NativeValue<String>>,
    superseded: Option<NativeValue<String>>,
}
fn object_births(
    tx: &Transaction<'_>,
    m: &MetadataView<'_>,
    store: &(impl CodecStore + ?Sized),
) -> Result<NativeTree<NodeDigest, ObjectBirth>, String> {
    let mut out = NativeTree::default();
    let latest = generation_admitted(tx, store)?.max(m.generation);
    {
        let mut stmt = CheckedStatement::prepare(tx, c"SELECT object_id,object_revision,kind,batch_id,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM s_content_objects").map_err(err)?;
        while let Some(mut r) = stmt.next().map_err(err)? {
            let id = row_text_admitted(&mut r, 0, store).map_err(err)?;
            let mut b = ObjectBirth {
                revision: r.integer(1).map_err(err)?,
                kind: row_text_admitted(&mut r, 2, store).map_err(err)?,
                batch: row_text_admitted(&mut r, 3, store).map_err(err)?,
                first: r.integer(4).map_err(err)?,
                cut: row_text_admitted(&mut r, 5, store).map_err(err)?,
                published: r.integer(6).map_err(err)?,
                withdrawn: r.optional_integer(7).map_err(err)?,
                reason: row_optional_text_admitted(&mut r, 8, store).map_err(err)?,
                superseded: row_optional_text_admitted(&mut r, 9, store).map_err(err)?,
            };
            if b.first < 1
                || b.first > latest
                || b.published < b.first
                || b.published > latest
                || b.cut
                    != store
                        .native_format(format_args!("cut-{}", b.first))
                        .map_err(err)?
                || b.withdrawn.is_some_and(|g| g <= b.first || g > latest)
            {
                return Err(err("独立对象出生/生命周期代号非法"));
            }
            if b.first > m.generation {
                continue;
            }
            if b.published > m.generation {
                return Err(err("对象出生后发布代号被改写"));
            }
            if b.withdrawn.is_some_and(|g| g > m.generation) {
                b.withdrawn = None;
                b.reason = None;
                b.superseded = None;
            }
            let digest = hash_view(store, VV::String(&id))?;
            if out
                .insert_admitted(digest, b, store)
                .map_err(err)?
                .is_some()
            {
                return Err(err("对象身份同时存在两个独立表"));
            }
        }
        stmt.finalize().map_err(err)?;
    }
    Ok(out)
}
struct ObservationBirth {
    kind: NativeValue<String>,
    batch: NativeValue<String>,
    first: i64,
}
fn observation_births(
    tx: &Transaction<'_>,
    m: &MetadataView<'_>,
    store: &(impl CodecStore + ?Sized),
) -> Result<NativeTree<NodeDigest, ObservationBirth>, String> {
    let mut out = NativeTree::default();
    let latest = generation_admitted(tx, store)?.max(m.generation);
    let mut stmt = CheckedStatement::prepare(
        tx,
        c"SELECT observation_id,kind,batch_id,published_generation FROM s_content_observations",
    )
    .map_err(err)?;
    while let Some(mut r) = stmt.next().map_err(err)? {
        let id = row_text_admitted(&mut r, 0, store).map_err(err)?;
        let b = ObservationBirth {
            kind: row_text_admitted(&mut r, 1, store).map_err(err)?,
            batch: row_text_admitted(&mut r, 2, store).map_err(err)?,
            first: r.integer(3).map_err(err)?,
        };
        if b.first < 1 || b.first > latest {
            return Err(err("独立观察首次发布代号非法"));
        }
        if b.first > m.generation {
            continue;
        }
        let digest = hash_view(store, VV::String(&id))?;
        out.insert_admitted(digest, b, store).map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    Ok(out)
}
#[derive(Clone, Copy)]
enum ObjectFamily {
    Active,
    Withdrawn,
    Born,
}
fn member(b: &ObjectBirth, m: &MetadataView<'_>, f: ObjectFamily) -> bool {
    match f {
        ObjectFamily::Active => b.withdrawn.is_none(),
        ObjectFamily::Withdrawn => b.withdrawn.is_some(),
        ObjectFamily::Born => b.first == m.generation,
    }
}
fn array_len(
    store: &mut (impl CodecStore + ?Sized),
    root: &RefValue,
    name: &str,
) -> Result<u64, String> {
    let r = store.lookup_field_ref(root.view(), name).map_err(err)?;
    array_reference_len(&r, name)
}
fn array_reference_len(r: &RefValue, name: &str) -> Result<u64, String> {
    if r.kind() != Kind::Array {
        return Err(err(format!("{name} 不是数组")));
    }
    r.extent().ok_or_else(|| err("缺数组长度"))
}

/// 路径文本及向量持有原准入许可；先析构路径，再归还文本许可。
/// 仍调用 CodecStore 的既有 replace_ref/lookup_ref，保留 Owned 分派。
struct PublicationPath {
    steps: sc::CodecVec<PathStep>,
    _permits: sc::CodecVec<sc::capture_allocation::scratch::NativeReservation>,
}
impl PublicationPath {
    fn new<'a>(
        admission: &(impl CodecRead + ?Sized),
        steps: impl IntoIterator<Item = sc::path::PathStepView<'a>>,
    ) -> Result<Self, String> {
        use sc::path::PathStepView;
        let mut result = Self {
            steps: sc::CodecVec::default(),
            _permits: sc::CodecVec::default(),
        };
        for step in steps {
            if let Some(work) = admission.codec_work() {
                result.steps.try_reserve_native(work, 1).map_err(err)?;
            }
            let step = match step {
                PathStepView::Field(key) => {
                    if let Some(work) = admission.codec_work() {
                        result._permits.try_reserve_native(work, 1).map_err(err)?;
                        result
                            ._permits
                            .try_push(work.reserve_native(key.len()).map_err(err)?)
                            .map_err(err)?;
                    }
                    let mut text = String::new();
                    text.try_reserve_exact(key.len()).map_err(|_| {
                        err(admission.codec_work().map_or_else(
                            || {
                                sc::CodecError::new(
                                    sc::capture_allocation::LIMIT,
                                    "路径文本分配失败",
                                )
                            },
                            |work| {
                                work.reject(
                                    sc::capture_allocation::scratch::ScratchFailure::Allocation,
                                )
                            },
                        ))
                    })?;
                    text.push_str(key);
                    PathStep::Field(text)
                }
                PathStepView::Index(index) => PathStep::Index(index),
                PathStepView::JsonText => PathStep::JsonText,
            };
            result.steps.try_push(step).map_err(err)?;
        }
        Ok(result)
    }
}

fn resolve_record_batch(
    store: &mut (impl CodecStore + ?Sized),
    root: &mut RefValue,
    path: &[PathStep],
    record: &RefValue,
    first: i64,
    stored_batch: &str,
    m: &MetadataView<'_>,
    batch: &str,
) -> Result<(), String> {
    resolve_record_batch_projected(
        store,
        root,
        path,
        record,
        first,
        stored_batch,
        m,
        batch,
        None,
    )
}
fn resolve_record_batch_projected(
    store: &mut (impl CodecStore + ?Sized),
    root: &mut RefValue,
    path: &[PathStep],
    record: &RefValue,
    first: i64,
    stored_batch: &str,
    m: &MetadataView<'_>,
    batch: &str,
    projection: Option<&mut PublicationProjection<'_, '_>>,
) -> Result<(), String> {
    let value = match projection {
        Some(p) => p.read_batch(store, record.view())?,
        None => store
            .lookup_field_ref(record.view(), "batch_id")
            .map_err(err)?,
    };
    if first == m.generation {
        if stored_batch != batch || value.kind() != Kind::TemplateBatch {
            return Err(err("本批出生索引/占位不符"));
        }
        let path = PublicationPath::new(
            store,
            path.iter()
                .map(|step| match step {
                    PathStep::Field(key) => sc::path::PathStepView::Field(key),
                    PathStep::Index(index) => sc::path::PathStepView::Index(*index),
                    PathStep::JsonText => sc::path::PathStepView::JsonText,
                })
                .chain(std::iter::once(sc::path::PathStepView::Field("batch_id"))),
        )?;
        *root = store
            .replace_ref(root.view(), &path.steps, batch)
            .map_err(err)?;
    } else {
        if stored_batch == batch {
            return Err(err("旧出生记录不得换成本批身份"));
        }
        if value.digest().map_err(err)? != hash_view(store, VV::String(stored_batch))? {
            return Err(err("batch_id 与独立记录不同"));
        }
    }
    Ok(())
}
#[cfg(test)]
thread_local! { static RECORD_RESOLVERS: std::cell::Cell<[u64;2]> = const {std::cell::Cell::new([0,0])}; }
#[cfg(test)]
pub(crate) fn reset_record_resolver_counts() {
    RECORD_RESOLVERS.with(|v| v.set([0, 0]));
    NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.set(0));
}
#[cfg(test)]
pub(crate) fn record_resolver_counts() -> [u64; 3] {
    let v = RECORD_RESOLVERS.with(std::cell::Cell::get);
    [
        v[0],
        v[1],
        NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(std::cell::Cell::get) as u64,
    ]
}
fn resolve_objects(
    store: &mut (impl CodecStore + ?Sized),
    root: &mut RefValue,
    name: &str,
    births: &BTreeMap<NodeDigest, ObjectBirth>,
    m: &MetadataView<'_>,
    batch: &str,
    family: ObjectFamily,
    mut memo: Option<&mut NativePublicationMemo<'_>>,
    mut projection: Option<&mut PublicationProjection<'_, '_>>,
) -> Result<(), String> {
    #[cfg(test)]
    RECORD_RESOLVERS.with(|v| {
        let mut n = v.get();
        n[0] += 1;
        v.set(n);
    });
    let mut expected = NativeSet::default();
    for (id, _) in births.iter().filter(|(_, b)| member(b, m, family)) {
        expected.insert_admitted(*id, store).map_err(err)?;
    }
    // 本轮只修改其它成员的batch标量；未处理成员仍来自同一不可变模板数组。
    let array = store.lookup_field_ref(root.view(), name).map_err(err)?;
    let n = array_reference_len(&array, name)?;
    if n != expected.len() as u64 {
        return Err(err(format!("{name} 与独立索引成员数不符")));
    }
    let mut seen = NativeSet::default();
    let mut cursor = store.array_cursor(array.view()).map_err(err)?;
    for i in 0..n {
        let path = PublicationPath::new(
            store,
            [
                sc::path::PathStepView::Field(name),
                sc::path::PathStepView::Index(i),
            ],
        )?;
        let record = if let Some(cursor) = &mut cursor {
            store.next_array_member(cursor).map_err(err)?
        } else {
            store
                .lookup_ref(array.view(), &[PathStep::Index(i)])
                .map_err(err)?
        };
        // 本模块通用于既有记录与S3记录，只取其共同控制字段，不裁载荷字段schema。
        let fields = match memo.as_deref_mut() {
            Some(memo) => memo.read_fields(store, record.view())?,
            None => match projection.as_deref_mut() {
                Some(p) => p.read_fields(store, record.view())?,
                None => PublicationFields::Direct(read_publication_fields(store, record.view())?),
            },
        };
        let id = fields
            .get_admitted(store, "object_id")?
            .ok_or_else(|| err("缺object_id"))?
            .digest()
            .map_err(err)?;
        if !expected.contains(&id) || !seen.insert_admitted(id, store).map_err(err)? {
            return Err(err(format!("{name} 存在越界/重复身份")));
        }
        let b = &births[&id];
        let revision = Decimal::new(b.revision);
        let first = Decimal::new(b.first);
        let published = Decimal::new(b.published);
        let withdrawn = b.withdrawn.map(Decimal::new);
        let null = VV::Json(sc::capture_allocation::audit_json::JsonView::Null);
        for (k, v) in [
            ("kind", VV::String(&b.kind)),
            ("object_revision", VV::String(revision.as_str())),
            ("first_known_generation", VV::String(first.as_str())),
            ("first_known_cut", VV::String(&b.cut)),
            ("published_generation", VV::String(published.as_str())),
            (
                "withdrawn_generation",
                withdrawn
                    .as_ref()
                    .map(|v| VV::String(v.as_str()))
                    .unwrap_or(null),
            ),
            (
                "withdrawal_reason",
                b.reason
                    .as_ref()
                    .map(|v| VV::String(v.as_str()))
                    .unwrap_or(null),
            ),
            (
                "superseded_by",
                b.superseded
                    .as_ref()
                    .map(|v| VV::String(v.as_str()))
                    .unwrap_or(null),
            ),
            (
                "lifecycle",
                VV::String(if b.withdrawn.is_some() {
                    "withdrawn"
                } else {
                    "active"
                }),
            ),
        ] {
            if fields
                .get_admitted(store, k)?
                .ok_or_else(|| err(format!("缺{k}")))?
                .digest()
                .map_err(err)?
                != hash_view(store, v)?
            {
                return Err(err(format!("{k} 与独立记录不同")));
            }
        }
        resolve_record_batch_projected(
            store,
            root,
            &path.steps,
            &record,
            b.first,
            &b.batch,
            m,
            batch,
            projection.as_deref_mut(),
        )?;
    }
    Ok(())
}
fn resolve_observations(
    store: &mut (impl CodecStore + ?Sized),
    root: &mut RefValue,
    births: &BTreeMap<NodeDigest, ObservationBirth>,
    m: &MetadataView<'_>,
    batch: &str,
    delta: bool,
) -> Result<(), String> {
    #[cfg(test)]
    RECORD_RESOLVERS.with(|v| {
        let mut n = v.get();
        n[1] += 1;
        v.set(n);
    });
    let mut expected = NativeSet::default();
    for (id, _) in births
        .iter()
        .filter(|(_, b)| !delta || b.first == m.generation)
    {
        expected.insert_admitted(*id, store).map_err(err)?;
    }
    let array = store
        .lookup_field_ref(root.view(), "observations")
        .map_err(err)?;
    let n = array_reference_len(&array, "observations")?;
    if n != expected.len() as u64 {
        return Err(err("观察与独立索引成员数不符"));
    }
    let mut seen = NativeSet::default();
    let mut cursor = store.array_cursor(array.view()).map_err(err)?;
    for i in 0..n {
        let path = PublicationPath::new(
            store,
            [
                sc::path::PathStepView::Field("observations"),
                sc::path::PathStepView::Index(i),
            ],
        )?;
        let record = if let Some(cursor) = &mut cursor {
            cursor
                .next_with_prefix(store.generic_source(), &path.steps[..1])
                .map_err(err)?
                .map(RefValue::Plain)
                .ok_or_else(|| err("数组游标提前结束"))?
        } else {
            store.lookup_ref(root.view(), &path.steps).map_err(err)?
        };
        let id = store
            .lookup_field_ref(record.view(), "observation_id")
            .map_err(err)?
            .digest()
            .map_err(err)?;
        if !expected.contains(&id) || !seen.insert_admitted(id, store).map_err(err)? {
            return Err(err("观察有越界/重复身份"));
        }
        let b = &births[&id];
        match_field_view(store, record.view(), "kind", VV::String(&b.kind))?;
        resolve_record_batch(
            store,
            root,
            &path.steps,
            &record,
            b.first,
            &b.batch,
            m,
            batch,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{canonical_json, json, meta_set, sha256_hex, tb02a, SCHEMA};
    use super::*;
    use content_manifest::SourceCommit;
    use rusqlite::{params, Connection};
    use sc::{NumberToken, Value as V};
    use serde_json::Value as J;

    #[cfg(feature = "s_incremental")]
    #[test]
    fn admitted_sql_text_and_publication_path_keep_their_own_permits() {
        use sc::path::PathStepView;
        let mut connection = Connection::open_in_memory().unwrap();
        let tx = connection.transaction().unwrap();
        let budget = crate::v3::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        let (text, path) = budget
            .codec_work()
            .unwrap()
            .run(|work| -> Result<_, String> {
                let admission = NativeAdmission(Some(work));
                let text = query_one(
                    &tx,
                    c"SELECT 'first-cut', NULL",
                    |_| Ok(()),
                    |row| {
                        assert!(row_optional_text_admitted(row, 1, &admission)?.is_none());
                        row_text_admitted(row, 0, &admission)
                    },
                )
                .map_err(err)?;
                let path = PublicationPath::new(
                    &admission,
                    [
                        PathStepView::Field("objects"),
                        PathStepView::Index(7),
                        PathStepView::Field("batch_id"),
                    ],
                )?;
                assert!(text.is_native_admitted());
                assert_eq!(text.as_str(), "first-cut");
                assert!(matches!(&path.steps[0], PathStep::Field(name) if name == "objects"));
                assert!(matches!(&path.steps[1], PathStep::Index(7)));
                Ok((text, path))
            })
            .unwrap();
        let held = budget.live();
        assert!(held > baseline);
        drop(text);
        assert!(budget.live() < held, "文本析构单独归还其许可");
        assert!(budget.live() > baseline, "路径离域仍持有原许可");
        drop(path);
        assert_eq!(budget.live(), baseline);
    }

    /// #1394：独立只读诊断；完整 fresh 验证两根，普通 capture 不授予资源或生产资格。
    #[cfg(feature = "s_incremental")]
    #[test]
    #[ignore = "仅主控新拥有的 owned.sqlite；两臂各执行一次"]
    fn native_publication_fresh_sql_vs_current_capture_diagnostic() {
        use std::path::PathBuf;
        use std::time::Instant;

        fn outcome(result: &Result<(), sc::CodecError>) -> J {
            match result {
                Ok(()) => json!({"ok": true, "mode": "Public", "roots_verified": 2}),
                Err(error) => json!({
                    "ok": false, "mode": "Public", "code": error.code(),
                    "detail": error.detail()
                }),
            }
        }
        let path = PathBuf::from(
            std::env::var_os("S1394_PUBLICATION_PROBE_DB")
                .expect("须显式提供主控新拥有的 S1394_PUBLICATION_PROBE_DB"),
        );
        assert!(path.is_absolute(), "诊断 DB 必须使用绝对路径");
        assert_eq!(
            path.file_name().and_then(|s| s.to_str()),
            Some("owned.sqlite")
        );
        // 路径任一层均不得借 symlink 绕回封存来源；这里只核文件元数据，不打开来源 DB。
        for component in path.ancestors() {
            assert!(
                !std::fs::symlink_metadata(component)
                    .expect("诊断路径必须存在")
                    .file_type()
                    .is_symlink(),
                "诊断路径不得含 symlink"
            );
        }
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(metadata.is_file(), "owned.sqlite 必须是普通文件");
        let mut db = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("仅以 READ_ONLY 打开 owned.sqlite");
        db.pragma_update(None, "query_only", "ON").unwrap();
        let query_only: i64 = db
            .query_row("PRAGMA query_only", [], |row| row.get(0))
            .unwrap();
        assert_eq!(query_only, 1);
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .unwrap();
        let (generation, snapshot_digest, delta_digest) = {
            let mut statement = tx.prepare(
                "SELECT generation,snapshot_root,delta_root FROM main.s_content_publications ORDER BY generation DESC LIMIT 1",
            ).unwrap();
            assert!(statement.readonly());
            assert_eq!(
                statement.column_names(),
                ["generation", "snapshot_root", "delta_root"]
            );
            let values: (i64, Vec<u8>, Vec<u8>) = statement
                .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .expect("必须存在类型正确的末代发布记录");
            statement.finalize().unwrap();
            let snapshot: NodeDigest = values.1.try_into().expect("snapshot_root 必须为32字节BLOB");
            let delta: NodeDigest = values.2.try_into().expect("delta_root 必须为32字节BLOB");
            (values.0, snapshot, delta)
        };
        assert!(generation >= 1);
        let header_start = Instant::now();
        let header_sql = content_store::sql_reads_on_thread();
        let header_frames = sc::frame_calls_on_thread();
        let (snapshot, delta) = {
            let mut store = content_store::TxStore::new(&tx).unwrap();
            (
                store
                    .content_ref_of(&sc::root_string(&snapshot_digest))
                    .unwrap(),
                store
                    .content_ref_of(&sc::root_string(&delta_digest))
                    .unwrap(),
            )
        };
        let headers = json!({
            "elapsed_ns": header_start.elapsed().as_nanos(),
            "sql_reads": content_store::sql_reads_on_thread() - header_sql,
            "frame_calls": sc::frame_calls_on_thread() - header_frames
        });

        let sql_start = Instant::now();
        let sql_before = content_store::sql_reads_on_thread();
        let sql_frames = sc::frame_calls_on_thread();
        let mut store = content_store::TxStore::new(&tx).unwrap();
        let sql_verify_start = Instant::now();
        let sql_result =
            store.verify_many_fresh([snapshot.clone(), delta.clone()], sc::Mode::Public);
        let sql_verify_ns = sql_verify_start.elapsed().as_nanos();
        drop(store);
        let sql_total_ns = sql_start.elapsed().as_nanos();
        let sql_reads = content_store::sql_reads_on_thread() - sql_before;
        let sql_frame_calls = sc::frame_calls_on_thread() - sql_frames;

        let capture_start = Instant::now();
        let capture_sql = content_store::sql_reads_on_thread();
        let capture_frames = sc::frame_calls_on_thread();
        let work_before = content_store::prototype_capture_work_on_thread();
        let (capture, node_count) = content_store::capture_for_request_prototype(&tx).unwrap();
        let scan_ns = capture_start.elapsed().as_nanos();
        let mut reader = capture.reader();
        let verify_sql = content_store::sql_reads_on_thread();
        let verify_frames = sc::frame_calls_on_thread();
        let capture_verify_start = Instant::now();
        let capture_result =
            sc::verify_many(&mut reader, [snapshot.clone(), delta.clone()]).map(|_| ());
        let capture_verify_ns = capture_verify_start.elapsed().as_nanos();
        let capture_verify_sql_reads = content_store::sql_reads_on_thread() - verify_sql;
        let capture_verify_frame_calls = sc::frame_calls_on_thread() - verify_frames;
        let drop_start = Instant::now();
        drop(reader);
        drop(capture);
        let drop_ns = drop_start.elapsed().as_nanos();
        let capture_total_ns = capture_start.elapsed().as_nanos();
        let capture_sql_reads = content_store::sql_reads_on_thread() - capture_sql;
        let capture_frame_calls = sc::frame_calls_on_thread() - capture_frames;
        let work_after = content_store::prototype_capture_work_on_thread();
        let work = json!({
            "capture_builds": work_after.capture_builds - work_before.capture_builds,
            "node_scans": work_after.node_scans - work_before.node_scans,
            "rebind_scans": work_after.rebind_scans - work_before.rebind_scans,
            "decoded_rows": work_after.decoded_rows - work_before.decoded_rows,
            "body_bytes": work_after.body_bytes - work_before.body_bytes
        });
        println!(
            "{}",
            json!({
                "diagnostic": "native_publication_fresh_sql_vs_current_capture", "issue": 1394,
                "db": path, "db_bytes": metadata.len(), "generation": generation,
                "snapshot": snapshot.to_transport_json(), "delta": delta.to_transport_json(),
                "transaction": "READ_ONLY/query_only/Deferred", "arm_runs": 1,
                "sql_reads_counter_scope": "content_store digest point SELECTs only; capture node_scans separately measured; format SELECTs excluded",
                "headers": headers,
                "sql_fresh": {"result": outcome(&sql_result), "verify_ns": sql_verify_ns,
                    "total_including_store_drop_ns": sql_total_ns, "sql_reads": sql_reads,
                    "frame_calls": sql_frame_calls},
                "current_capture_fresh": {"result": outcome(&capture_result),
                    "scan_ns": scan_ns, "verify_ns": capture_verify_ns, "drop_ns": drop_ns,
                    "total_including_scan_and_drop_ns": capture_total_ns, "node_count": node_count,
                    "sql_reads": capture_sql_reads, "frame_calls": capture_frame_calls,
                    "verify_sql_reads": capture_verify_sql_reads,
                    "verify_frame_calls": capture_verify_frame_calls, "work": work}
            })
        );
        assert_eq!(work_after.capture_builds - work_before.capture_builds, 1);
        assert_eq!(work_after.node_scans - work_before.node_scans, 1);
        assert_eq!(work_after.rebind_scans - work_before.rebind_scans, 0);
        assert_eq!(
            work_after.decoded_rows - work_before.decoded_rows,
            node_count as u64
        );
        assert_eq!(capture_verify_sql_reads, 0);
        assert_eq!(sql_result, capture_result);
        sql_result.expect("SQL 臂必须完整验证两根 Public 闭包");
        capture_result.expect("当前 capture 臂必须独立完整验证两根 Public 闭包");
        tx.rollback().unwrap();
    }

    fn cv(v: &J) -> V {
        match v {
            J::Null => V::Null,
            J::Bool(x) => V::Bool(*x),
            J::Number(x) => V::Number(NumberToken::parse(&x.to_string()).unwrap()),
            J::String(x) => V::String(x.clone()),
            J::Array(xs) => V::Array(xs.iter().map(cv).collect()),
            J::Object(xs) => V::Object(xs.iter().map(|(k, v)| (k.clone(), cv(v))).collect()),
        }
    }
    fn object(id: &str, generation: &str, batch: V) -> V {
        V::Object(vec![
            ("object_id".into(), V::String(id.into())),
            ("object_revision".into(), V::String("1".into())),
            ("kind".into(), V::String("CC-010.bi".into())),
            (
                "first_known_generation".into(),
                V::String(generation.into()),
            ),
            (
                "first_known_cut".into(),
                V::String(format!("cut-{generation}")),
            ),
            ("published_generation".into(), V::String(generation.into())),
            ("withdrawn_generation".into(), V::Null),
            ("withdrawal_reason".into(), V::Null),
            ("superseded_by".into(), V::Null),
            ("lifecycle".into(), V::String("active".into())),
            ("batch_id".into(), batch),
            (
                "payload".into(),
                cv(&json!({"batch_id":"原文里的同名字段"})),
            ),
        ])
    }
    fn field_set(v: &mut V, key: &str, value: V) {
        let V::Object(fields) = v else {
            panic!("fixture object")
        };
        fields.iter_mut().find(|(k, _)| k == key).unwrap().1 = value;
    }
    #[derive(Clone, Copy)]
    enum FixtureChange {
        None,
        PayloadPlaceholder,
        OldPlaceholder,
        CurrentBatchString,
        Duplicate,
        Observations,
        OldObservationPlaceholder,
        LegacyFullDelta,
        #[cfg(feature = "s_incremental")]
        LifeMismatch,
        #[cfg(feature = "s_incremental")]
        ForeignIdentity,
    }

    fn fixtures(tx: &Transaction<'_>) -> (ManifestMetadata, ManifestRoots, UnpublishedManifest) {
        fixtures_with(tx, FixtureChange::None)
    }

    fn fixtures_with(
        tx: &Transaction<'_>,
        change: FixtureChange,
    ) -> (ManifestMetadata, ManifestRoots, UnpublishedManifest) {
        tx.execute_batch(SCHEMA).unwrap();
        tx.execute_batch(v2::SCHEMA_V2).unwrap();
        tx.execute_batch(tb02a::SCHEMA).unwrap();
        let clock=v2::ClockPlan::parse(json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"birth-test","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{"event-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"24"}]}}})).unwrap();
        let payload = json!({"op":"ingest","clock_event_id":"event-2"});
        let ph = sha256_hex(canonical_json(&payload).as_bytes());
        let envelope = json!({"schema_revision":"s-session/3","session_id":"birth-test","session_generation":"1","source_namespace":"feed","source_epoch":"epoch","message_id":"message-2","producer_id":"producer","producer_epoch":"2","payload_hash":ph,"causal_refs":[],"payload":payload});
        for (k, v) in [
            ("protocol_revision", "s-session/3"),
            ("session_id", "birth-test"),
            ("writer_epoch", "2"),
            ("generation", "1"),
            ("structure_cut", "cut-1"),
            ("last_advance_frontier", "0"),
            ("begin_token", "token-2"),
            ("advance_state", "begun:token-2:2:1"),
            ("profile_id", tb02a::PROFILE),
            ("rule_revision", "fixture-rule"),
            ("catalog_revision", "fixture-catalog"),
        ] {
            meta_set(tx, k, v).unwrap();
        }
        meta_set(tx, "profile_hash", &"c".repeat(64)).unwrap();
        let scope = json!({"structure":"CompleteCut","economic":"not_started"});
        meta_set(tx, "scope", &canonical_json(&scope)).unwrap();
        tx.execute(
            "INSERT INTO s_protocol_meta VALUES(1,'s-session/3','1',?1,?2,2,'21')",
            params![clock.hash, canonical_json(&clock.value)],
        )
        .unwrap();
        tx.execute("INSERT INTO s_input_messages VALUES('feed','epoch','message-2',?1,?2,'receipt-2',1,'AcceptedPending',NULL,'event-2',1)",params![ph,canonical_json(&envelope)]).unwrap();
        tx.execute(
            "INSERT INTO s_clock_events VALUES('event-2','begin',0,'21',?1,1,2)",
            params![canonical_json(&json!(["feed", "epoch", "message-2"]))],
        )
        .unwrap();
        tx.execute("INSERT INTO raw_events VALUES('raw-2',1,'1','hash','receipt-2',1,'feed','epoch','fixture','event-2','now','original','1','1','1','1',NULL)",[]).unwrap();
        content_store::init_schema(tx).unwrap();
        crate::v3::aux_index::init_schema(tx).unwrap();
        crate::v3::object_index::init_schema(tx).unwrap();
        let mut store = content_store::TxStore::new(tx).unwrap();
        let m = ManifestMetadata {
            session_id: "birth-test".into(),
            session_generation: "1".into(),
            writer_epoch: "2".into(),
            generation: 2,
            base_cut: "cut-1".into(),
            structure_cut: "cut-2".into(),
            input_frontier: 1,
            source_commit: SourceCommit {
                source_namespace: "feed".into(),
                source_epoch: "epoch".into(),
                message_id: "message-2".into(),
                payload_hash: ph,
                producer_id: "producer".into(),
                producer_epoch: "2".into(),
                clock_event_id: "event-2".into(),
                attempt_index: 0,
            },
            clock_plan_hash: clock.hash,
            semantic_commit_ns: "24".into(),
            profile_id: tb02a::PROFILE.into(),
            profile_hash: "c".repeat(64),
            rule_revision: "fixture-rule".into(),
            catalog_revision: "fixture-catalog".into(),
            scope: sc::write_value(&mut store, &cv(&scope))
                .unwrap()
                .content_ref(),
        };
        // 这些夹具只测独立出生许可；不声称两个 fixture ID 通过对象身份核验。
        let mut old = object(
            "obj3-old",
            "1",
            V::String(format!("batch3-{}", "d".repeat(64))),
        );
        let mut new = object("obj3-new", "2", V::BatchSelf);
        if matches!(change, FixtureChange::PayloadPlaceholder) {
            field_set(
                &mut new,
                "payload",
                V::Object(vec![("batch_id".into(), V::BatchSelf)]),
            );
        }
        if matches!(change, FixtureChange::OldPlaceholder) {
            field_set(&mut old, "batch_id", V::BatchSelf);
        }
        if matches!(change, FixtureChange::CurrentBatchString) {
            field_set(
                &mut new,
                "batch_id",
                V::String(format!("batch3-{}", "d".repeat(64))),
            );
        }
        #[cfg(feature = "s_incremental")]
        match change {
            FixtureChange::LifeMismatch => field_set(&mut old, "lifecycle", text("withdrawn")),
            FixtureChange::ForeignIdentity => field_set(&mut old, "object_id", text("foreign-id")),
            _ => {}
        }
        let generated = cv(
            &json!({"generation":2,"structure_cut":"cut-2","input_frontier":1,"catalog_revision":"fixture-catalog","profile_id":tb02a::PROFILE,"profile_hash":"c".repeat(64),"rule_revision":"fixture-rule","raw_events":[],"effective_events":[],"objects":[],"witnesses":[],"relations":[],"observations":[],"scope":scope}),
        );
        let mut publication = cv(
            &json!({"session_id":"birth-test","generation":"2","structure_cut":"cut-2","catalog_revision":"fixture-catalog","scope":scope,"index_frontier":null,"profile_id":tb02a::PROFILE,"profile_hash":"c".repeat(64),"history_mode":"RecomputedWithRevision","as_of_generation":null,"objects":[],"withdrawn_objects":[],"witnesses":[],"relations":[],"observations":[],"raw_history":[],"input_frontier":"1","seq_range":{"from":"1","to":"1"},"catalog_run_status":"not_run","catalog_evidence":{"batch_id":null}}),
        );
        let old_for_delta = old.clone();
        let objects = if matches!(change, FixtureChange::Duplicate) {
            vec![old.clone(), old]
        } else {
            vec![old, new.clone()]
        };
        field_set(&mut publication, "objects", V::Array(objects));
        let mut delta = cv(
            &json!({"generation":"2","session_id":"birth-test","catalog_revision":"fixture-catalog","base_cut":"cut-1","next_cut":"cut-2","seq_range":{"from":"1","to":"1"},"index_frontier":null,"input_frontier":"1","catalog_run_status":"not_run","catalog_evidence":{"batch_id":null},"upserts":[],"withdrawals":[],"replaces":[],"witnesses":[],"relations":[],"observations":[],"raw_history_added":[]}),
        );
        field_set(
            &mut delta,
            "upserts",
            V::Array(if matches!(change, FixtureChange::LegacyFullDelta) {
                vec![old_for_delta, new]
            } else {
                vec![new]
            }),
        );
        let has_observations = matches!(
            change,
            FixtureChange::Observations | FixtureChange::OldObservationPlaceholder
        );
        if has_observations {
            let ob = |id: &str, batch: V| {
                V::Object(vec![
                    ("observation_id".into(), text(id)),
                    ("kind".into(), text("insufficient_knowledge")),
                    ("batch_id".into(), batch),
                    ("detail".into(), cv(&json!({"batch_id":"原样保留"}))),
                ])
            };
            let old_batch = if matches!(change, FixtureChange::OldObservationPlaceholder) {
                V::BatchSelf
            } else {
                text(format!("batch3-{}", "e".repeat(64)))
            };
            let new_ob = ob("ob-new", V::BatchSelf);
            field_set(
                &mut publication,
                "observations",
                V::Array(vec![ob("ob-old", old_batch), new_ob.clone()]),
            );
            field_set(&mut delta, "observations", V::Array(vec![new_ob]));
        }
        for root in [&mut publication, &mut delta] {
            field_set(root, "index_frontier", V::BatchSelf);
            field_set(
                root,
                "catalog_evidence",
                V::Object(vec![("batch_id".into(), V::BatchSelf)]),
            );
        }
        let roots = ManifestRoots {
            generated_state: sc::write_value(&mut store, &generated)
                .unwrap()
                .content_ref(),
            publication_template: sc::write_template(&mut store, &publication)
                .unwrap()
                .content_ref(),
            delta_template: sc::write_template(&mut store, &delta)
                .unwrap()
                .content_ref(),
        };
        let token = format!(
            "begin-{}",
            sha256_hex(
                canonical_json(&json!(["birth-test", "1", "2", "1", "cut-1", "2", "1"])).as_bytes()
            )
        );
        meta_set(tx, "begin_token", &token).unwrap();
        meta_set(tx, "advance_state", &format!("begun:{token}:2:1")).unwrap();
        let manifest = content_manifest::assemble_unpublished(&mut store, &m, &roots).unwrap();
        for (id, g, batch) in [
            ("obj3-old", 1, format!("batch3-{}", "d".repeat(64))),
            ("obj3-new", 2, manifest.batch_id.as_str().to_owned()),
        ] {
            let node = sc::write_value(&mut store, &V::Null).unwrap();
            tx.execute("INSERT INTO s_content_objects VALUES(?1,1,'CC-010.bi','key',NULL,NULL,NULL,?5,?5,?5,?5,?2,?3,?4,?3,NULL,NULL,NULL)",params![id,batch,g,format!("cut-{g}"),node.digest.as_slice()]).unwrap();
        }
        if has_observations {
            for (id, g, batch) in [
                ("ob-old", 1, format!("batch3-{}", "e".repeat(64))),
                ("ob-new", 2, manifest.batch_id.as_str().to_owned()),
            ] {
                // 本夹具只核出生许可，值根的原件语义在辅助索引测试单独核。
                let root = sc::write_value(&mut store, &V::Null).unwrap();
                tx.execute("INSERT INTO s_content_observations VALUES(?1,'insufficient_knowledge',NULL,?2,?3,?3,?4)",params![id,batch,root.digest.as_slice(),g]).unwrap();
            }
        }
        (m, roots, manifest)
    }
    fn rendered(store: &mut dyn sc::NodeSource, r: &ContentRef) -> J {
        let mut out = Vec::new();
        sc::render_canonical(store, r, &mut out).unwrap();
        serde_json::from_slice(&out).unwrap()
    }
    #[cfg(feature = "s_incremental")]
    fn native_publication_fields_nodes(tx: &Transaction<'_>) -> sc::MemoryNodes {
        let mut nodes = sc::MemoryNodes::new();
        let mut statement = tx
            .prepare("SELECT digest,tag,body FROM s_content_nodes")
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let digest: Vec<u8> = row.get(0).unwrap();
            nodes.insert_unchecked(
                digest.try_into().unwrap(),
                row.get(1).unwrap(),
                row.get(2).unwrap(),
            );
        }
        nodes
    }
    #[cfg(feature = "s_incremental")]
    struct OrdinaryFixture {
        path: std::path::PathBuf,
        db: Connection,
        metadata: ManifestMetadata,
        roots: ManifestRoots,
        manifest: UnpublishedManifest,
    }
    #[cfg(feature = "s_incremental")]
    impl OrdinaryFixture {
        fn new(change: FixtureChange, mutation: Option<&str>) -> Self {
            let path = std::env::temp_dir().join(format!(
                "s1640-projection-{}-{}.sqlite",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let mut writer = Connection::open(&path).unwrap();
            let tx = writer.transaction().unwrap();
            let (metadata, roots, manifest) = fixtures_with(&tx, change);
            if let Some(sql) = mutation {
                tx.execute_batch(sql).unwrap();
            }
            tx.commit().unwrap();
            drop(writer);
            let db = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
            Self {
                path,
                db,
                metadata,
                roots,
                manifest,
            }
        }
    }
    #[cfg(feature = "s_incremental")]
    impl Drop for OrdinaryFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn ordinary_proof_publication_complete_outputs_two_frontiers_and_release() {
        use crate::v3::audit_store::HashReader;
        let mut fixture = OrdinaryFixture::new(FixtureChange::Observations, None);
        let tx = fixture.db.transaction().unwrap();
        let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
        let mut reader = HashReader::ordinary_publication_reader_for_test(&mut source);
        let expected = resolve_indexed_templates_pending_view(
            &tx,
            &mut reader,
            fixture.metadata.view(),
            fixture.roots.view(),
            &fixture.manifest.batch_id,
            0,
        )
        .unwrap();
        let outputs = [
            rendered(&mut reader, &normal_reference(expected.snapshot.view())),
            rendered(&mut reader, &normal_reference(expected.delta.view())),
        ];
        reader.clear_overlay();
        let mut projection = PublicationProjection::new(&tx, &reader).unwrap();
        for _ in 0..2 {
            let actual = resolve_ordinary_indexed_templates_pending_view(
                &tx,
                &mut reader,
                fixture.metadata.view(),
                fixture.roots.view(),
                &fixture.manifest.batch_id,
                0,
                &mut projection,
            )
            .unwrap();
            assert_eq!(actual.snapshot, expected.snapshot);
            assert_eq!(actual.delta, expected.delta);
            assert_eq!(
                [
                    rendered(&mut reader, &normal_reference(actual.snapshot.view())),
                    rendered(&mut reader, &normal_reference(actual.delta.view()))
                ],
                outputs
            );
            projection.commit_cut();
            assert!(!projection.previous.is_empty());
            assert!(projection.current.is_empty());
            reader.clear_overlay();
        }
        let mut foreign_source = crate::content_store::ReadStore::new(&tx).unwrap();
        let foreign_reader = HashReader::ordinary_publication_reader_for_test(&mut foreign_source);
        assert!(projection.check(&tx, &foreign_reader).is_err());
        projection.commit_cut();
        assert!(projection.previous.is_empty() && projection.batch_previous.is_empty());
        drop(projection);
        assert_eq!(ORDINARY_PUBLICATION_BYTES.with(|s| s.get().0), 0);
        let heap = crate::v3::audit_owner::allocation_probe::Window::start();
        let live_before = crate::v3::audit_owner::allocation_probe::live_bytes();
        let (mut one_frontier, mut two_frontiers) = (0, 0);
        {
            let mut projection = PublicationProjection::new(&tx, &reader).unwrap();
            for round in 0..2 {
                resolve_ordinary_indexed_templates_pending_view(
                    &tx,
                    &mut reader,
                    fixture.metadata.view(),
                    fixture.roots.view(),
                    &fixture.manifest.batch_id,
                    0,
                    &mut projection,
                )
                .unwrap();
                reader.clear_overlay();
                if round == 1 {
                    two_frontiers =
                        crate::v3::audit_owner::allocation_probe::live_bytes() - live_before;
                }
                projection.commit_cut();
                if round == 0 {
                    one_frontier =
                        crate::v3::audit_owner::allocation_probe::live_bytes() - live_before;
                }
            }
        }
        let observed = heap.finish();
        eprintln!("ORDINARY_PROJECTION_FRONTIERS actual_one_frontier={one_frontier} actual_two_frontiers={two_frontiers}");
        eprintln!("ORDINARY_PROJECTION_HEAP {observed}");
        assert!(one_frontier > 0 && two_frontiers > one_frontier);
        assert_eq!(observed["live_before"], observed["live_after"]);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn ordinary_proof_publication_keeps_per_cut_rejection_matrix() {
        use crate::v3::audit_store::HashReader;
        let mut cases = vec![
            (FixtureChange::OldPlaceholder, None),
            (FixtureChange::CurrentBatchString, None),
            (FixtureChange::Duplicate, None),
            (FixtureChange::OldObservationPlaceholder, None),
            (FixtureChange::LegacyFullDelta, None),
            (FixtureChange::LifeMismatch, None),
            (FixtureChange::ForeignIdentity, None),
        ];
        for sql in [
            "UPDATE s_content_objects SET kind='other' WHERE object_id='obj3-old'",
            "UPDATE s_content_objects SET withdrawn_generation=2,withdrawal_reason='reason' WHERE object_id='obj3-old'",
            "DELETE FROM s_content_objects WHERE object_id='obj3-old'",
            "UPDATE s_content_objects SET batch_id='other' WHERE object_id='obj3-old'",
            "UPDATE s_content_objects SET first_known_generation=1,first_known_cut='cut-1' WHERE object_id='obj3-new'",
            "UPDATE s_content_observations SET published_generation=1 WHERE observation_id='ob-new'",
        ] { cases.push((FixtureChange::Observations, Some(sql))); }
        for (change, mutation) in cases {
            let mut fixture = OrdinaryFixture::new(change, mutation);
            let tx = fixture.db.transaction().unwrap();
            let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
            let mut reader = HashReader::ordinary_publication_reader_for_test(&mut source);
            let expected = resolve_indexed_templates_pending_view(
                &tx,
                &mut reader,
                fixture.metadata.view(),
                fixture.roots.view(),
                &fixture.manifest.batch_id,
                0,
            )
            .err()
            .unwrap();
            reader.clear_overlay();
            let mut projection = PublicationProjection::new(&tx, &reader).unwrap();
            for _ in 0..2 {
                let actual = resolve_ordinary_indexed_templates_pending_view(
                    &tx,
                    &mut reader,
                    fixture.metadata.view(),
                    fixture.roots.view(),
                    &fixture.manifest.batch_id,
                    0,
                    &mut projection,
                )
                .err()
                .unwrap();
                assert_eq!(actual, expected);
                reader.clear_overlay();
            }
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn ordinary_proof_publication_committed_active_fields_rejected_at_withdrawal_cut() {
        use crate::v3::audit_store::HashReader;
        let mut fixture = OrdinaryFixture::new(FixtureChange::None, Some(
            "UPDATE s_content_objects SET withdrawn_generation=3,withdrawal_reason='superseded' WHERE object_id='obj3-old'; UPDATE meta SET value='3' WHERE key='generation';"));
        let tx = fixture.db.transaction().unwrap();
        let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
        let mut reader = HashReader::ordinary_publication_reader_for_test(&mut source);
        let mut projection = PublicationProjection::new(&tx, &reader).unwrap();
        // 同一只读事务的 generation 2 看不到 generation 3 的撤回，整 cut 成功后提升。
        let warm = resolve_ordinary_indexed_templates_pending_view(
            &tx,
            &mut reader,
            fixture.metadata.view(),
            fixture.roots.view(),
            &fixture.manifest.batch_id,
            0,
            &mut projection,
        )
        .unwrap();
        let mut snapshot = rendered(&mut reader, &normal_reference(warm.snapshot.view()));
        let mut delta = rendered(&mut reader, &normal_reference(warm.delta.view()));
        projection.commit_cut();
        assert!(!projection.previous.is_empty() && projection.current.is_empty());

        // 下一 cut 保留原 old 记录字节，却将它放入正确的撤回成员集合。
        // 这样成员数/身份先过，实际拒绝必须来自逐 cut 的生命周期核对。
        let old = snapshot["objects"].as_array_mut().unwrap().remove(0);
        assert_eq!(old["object_id"], "obj3-old");
        snapshot["withdrawn_objects"] = json!([old]);
        snapshot["generation"] = json!("3");
        snapshot["structure_cut"] = json!("cut-3");
        delta["generation"] = json!("3");
        delta["base_cut"] = json!("cut-2");
        delta["next_cut"] = json!("cut-3");
        delta["upserts"] = json!([]);
        for value in [&mut snapshot, &mut delta] {
            value["input_frontier"] = json!("2");
            value["seq_range"] = json!({"from":"2","to":"2"});
        }
        let mut snapshot = cv(&snapshot);
        let mut delta = cv(&delta);
        for value in [&mut snapshot, &mut delta] {
            field_set(value, "index_frontier", V::BatchSelf);
            field_set(
                value,
                "catalog_evidence",
                V::Object(vec![("batch_id".into(), V::BatchSelf)]),
            );
        }
        let next_roots = ManifestRoots {
            generated_state: fixture.roots.generated_state.clone(),
            publication_template: sc::write_template(&mut reader, &snapshot)
                .unwrap()
                .content_ref(),
            delta_template: sc::write_template(&mut reader, &delta)
                .unwrap()
                .content_ref(),
        };
        fixture.metadata.generation = 3;
        fixture.metadata.base_cut = "cut-2".into();
        fixture.metadata.structure_cut = "cut-3".into();
        fixture.metadata.input_frontier = 2;
        let next_batch = format!("batch3-{}", "f".repeat(64));
        let expected = resolve_indexed_templates_pending_view(
            &tx,
            &mut reader,
            fixture.metadata.view(),
            next_roots.view(),
            &next_batch,
            1,
        )
        .err()
        .unwrap();
        assert!(
            expected.contains("withdrawn_generation 与独立记录不同"),
            "{expected}"
        );
        let actual = resolve_ordinary_indexed_templates_pending_view(
            &tx,
            &mut reader,
            fixture.metadata.view(),
            next_roots.view(),
            &next_batch,
            1,
            &mut projection,
        )
        .err()
        .unwrap();
        assert_eq!(actual, expected);
        // 旧字段确实从成功前沿复用，而不是只跑一个新的冷坏 fixture。
        assert!(projection.current.iter().any(|(key, value)| projection
            .previous
            .get(key)
            .is_some_and(|old| std::rc::Rc::ptr_eq(old, value))));
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn ordinary_proof_publication_exact_declarations_never_hit_good_root_entry() {
        use crate::v3::audit_store::HashReader;
        let mut fixture = OrdinaryFixture::new(FixtureChange::None, None);
        let tx = fixture.db.transaction().unwrap();
        let mut nodes = sc::MemoryNodes::new();
        let valid = sc::write_value(&mut nodes, &object("old", "1", text("batch-old")))
            .unwrap()
            .content_ref();
        let mut missing = object("missing", "1", text("batch-old"));
        let V::Object(fields) = &mut missing else {
            unreachable!()
        };
        fields.retain(|(key, _)| key != "lifecycle");
        let missing = sc::write_value(&mut nodes, &missing).unwrap().content_ref();
        let mut reader = HashReader::ordinary_publication_reader_for_test(&mut nodes);
        let mut projection = PublicationProjection::new(&tx, &reader).unwrap();
        projection
            .read_fields(&mut reader, (&valid).into())
            .unwrap();
        projection.commit_cut();
        for index in 0..3 {
            let mut invalid = valid.clone();
            match index {
                0 => invalid.kind = Kind::Array,
                1 => invalid.extent = Some(valid.extent.unwrap() + 1),
                _ => invalid.format = "wrong-format".into(),
            }
            let expected = read_publication_fields(&mut reader, (&invalid).into())
                .err()
                .unwrap();
            assert_eq!(
                projection
                    .read_fields(&mut reader, (&invalid).into())
                    .err()
                    .unwrap(),
                expected
            );
        }
        for _ in 0..2 {
            assert!(projection
                .read_fields(&mut reader, (&missing).into())
                .is_err());
        }
        assert!(projection.current.is_empty());
        assert_eq!(projection.previous.len(), 1);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_publication_fields_same_roots_output_and_fewer_real_lookups() {
        use crate::v3::audit_store::CodecRead;
        use crate::v3::audit_store::HashReader;
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::Observations);
        let capture = sc::AuditCapture::new(native_publication_fields_nodes(&tx));
        // 与生产相同：先核当前完整原件闭包，再开始该次历史循环的复用。
        capture
            .verify(&manifest.value.content_ref(), sc::Mode::Template)
            .unwrap();
        let mut control = HashReader::native_publication_fields_for_test(&capture);
        let before = NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.get());
        let expected = resolve_indexed_templates_pending_view(
            &tx,
            &mut control,
            m.view(),
            roots.view(),
            &manifest.batch_id,
            0,
        )
        .unwrap();
        let control_lookups = NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.get()) - before;
        control
            .verify_many_ref(
                [expected.snapshot.view(), expected.delta.view()],
                sc::Mode::Public,
            )
            .unwrap();
        let expected_output = [
            rendered(&mut control, &normal_reference(expected.snapshot.view())),
            rendered(&mut control, &normal_reference(expected.delta.view())),
        ];
        let mut reader = HashReader::native_publication_fields_for_test(&capture);
        let mut memo = NativePublicationMemo::new(&reader).unwrap();
        let before = NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.get());
        let actual = resolve_native_indexed_templates_pending_view(
            &tx,
            &mut reader,
            m.view(),
            roots.view(),
            &manifest.batch_id,
            0,
            &mut memo,
        )
        .unwrap();
        let first_lookups = NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.get()) - before;
        assert_eq!(actual.snapshot, expected.snapshot);
        assert_eq!(actual.delta, expected.delta);
        assert_eq!(control_lookups, 3);
        assert_eq!(
            first_lookups, 2,
            "同一新record在snapshot/delta只读取一次十字段"
        );
        reader
            .verify_many_ref(
                [actual.snapshot.view(), actual.delta.view()],
                sc::Mode::Public,
            )
            .unwrap();
        assert_eq!(
            [
                rendered(&mut reader, &normal_reference(actual.snapshot.view())),
                rendered(&mut reader, &normal_reference(actual.delta.view()))
            ],
            expected_output
        );
        reader.clear_overlay();
        let before = NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.get());
        let again = resolve_native_indexed_templates_pending_view(
            &tx,
            &mut reader,
            m.view(),
            roots.view(),
            &manifest.batch_id,
            0,
            &mut memo,
        )
        .unwrap();
        assert_eq!(again.snapshot, expected.snapshot);
        assert_eq!(again.delta, expected.delta);
        assert_eq!(
            NATIVE_PUBLICATION_FIELDS_LOOKUPS.with(|v| v.get()),
            before,
            "clear_overlay后仍复用原件，必须真的省掉读取"
        );
        let fresh = sc::AuditCapture::new(native_publication_fields_nodes(&tx));
        let mut foreign = HashReader::native_publication_fields_for_test(&fresh);
        assert!(NativePublicationMemo::new(&foreign)
            .unwrap()
            .fields
            .is_empty());
        assert!(resolve_native_indexed_templates_pending_view(
            &tx,
            &mut foreign,
            m.view(),
            roots.view(),
            &manifest.batch_id,
            0,
            &mut memo
        )
        .err()
        .unwrap()
        .contains("capture"));
        let gated = HashReader::native_publication_fields_for_test(&capture)
            .with_native_publication_deadline_for_test(
                std::time::Instant::now() + std::time::Duration::from_secs(10),
            );
        assert!(
            memo.check_store(&gated).is_err(),
            "已命中memo仍须核当前权限门"
        );
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_publication_fields_exact_declaration_failures_and_overlay_not_cached() {
        use crate::v3::audit_store::CodecRead;
        use crate::v3::audit_store::HashReader;
        let mut nodes = sc::MemoryNodes::new();
        let valid = sc::write_value(&mut nodes, &object("old", "1", text("batch-old")))
            .unwrap()
            .content_ref();
        let mut missing = object("missing", "1", text("batch-old"));
        let V::Object(fields) = &mut missing else {
            unreachable!()
        };
        fields.retain(|(k, _)| k != "lifecycle");
        let missing = sc::write_value(&mut nodes, &missing).unwrap().content_ref();
        let mut damaged = nodes.clone();
        damaged.insert_unchecked(valid.digest().unwrap(), sc::tag::OBJECT, Vec::new());
        let capture = sc::AuditCapture::new(nodes);
        let mut reader = HashReader::native_publication_fields_for_test(&capture);
        let mut memo = NativePublicationMemo::new(&reader).unwrap();
        memo.read_fields(&mut reader, (&valid).into()).unwrap();
        assert_eq!(memo.fields.len(), 1);
        for mut altered in [valid.clone(), valid.clone(), valid.clone()]
            .into_iter()
            .enumerate()
        {
            match altered.0 {
                0 => altered.1.kind = Kind::Array,
                1 => altered.1.extent = Some(valid.extent.unwrap() + 1),
                _ => altered.1.format = "wrong-format".into(),
            }
            assert!(memo.read_fields(&mut reader, (&altered.1).into()).is_err());
            assert_eq!(memo.fields.len(), 1);
        }
        for _ in 0..2 {
            assert!(memo.read_fields(&mut reader, (&missing).into()).is_err());
        }
        assert_eq!(memo.fields.len(), 1, "缺字段失败不缓存");
        let damaged = sc::AuditCapture::new(damaged);
        let mut damaged_reader = HashReader::native_publication_fields_for_test(&damaged);
        let mut damaged_memo = NativePublicationMemo::new(&damaged_reader).unwrap();
        for _ in 0..2 {
            assert!(damaged_memo
                .read_fields(&mut damaged_reader, (&valid).into())
                .is_err());
        }
        assert!(damaged_memo.fields.is_empty(), "坏原件失败不缓存");
        let empty = sc::AuditCapture::new(sc::MemoryNodes::new());
        let mut overlay = HashReader::native_publication_fields_for_test(&empty);
        let overlay_record =
            sc::write_value(&mut overlay, &object("overlay", "1", text("batch-old")))
                .unwrap()
                .content_ref();
        let mut overlay_memo = NativePublicationMemo::new(&overlay).unwrap();
        overlay_memo
            .read_fields(&mut overlay, (&overlay_record).into())
            .unwrap();
        overlay_memo
            .read_fields(&mut overlay, (&overlay_record).into())
            .unwrap();
        assert!(
            overlay_memo.fields.is_empty(),
            "明确overlay来源直接读取且不缓存"
        );
        overlay.clear_overlay();
        assert!(overlay_memo
            .read_fields(&mut overlay, (&overlay_record).into())
            .is_err());
        // 扩 schema 不属于本次十字段复用。
        assert_eq!(
            reader
                .lookup_subset_ref((&valid).into(), PUBLICATION_FIELD_NAMES)
                .unwrap()
                .len(),
            10
        );
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_publication_fields_retains_per_cut_rejection_matrix() {
        use crate::v3::audit_store::HashReader;
        for change in [
            FixtureChange::OldPlaceholder,
            FixtureChange::CurrentBatchString,
            FixtureChange::Duplicate,
            FixtureChange::OldObservationPlaceholder,
            FixtureChange::LegacyFullDelta,
            FixtureChange::LifeMismatch,
            FixtureChange::ForeignIdentity,
        ] {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            let (m, roots, manifest) = fixtures_with(&tx, change);
            let capture = sc::AuditCapture::new(native_publication_fields_nodes(&tx));
            let mut reader = HashReader::native_publication_fields_for_test(&capture);
            let expected = resolve_indexed_templates_pending_view(
                &tx,
                &mut reader,
                m.view(),
                roots.view(),
                &manifest.batch_id,
                0,
            )
            .err()
            .unwrap();
            reader.clear_overlay();
            let mut memo = NativePublicationMemo::new(&reader).unwrap();
            for _ in 0..2 {
                let actual = resolve_native_indexed_templates_pending_view(
                    &tx,
                    &mut reader,
                    m.view(),
                    roots.view(),
                    &manifest.batch_id,
                    0,
                    &mut memo,
                )
                .err()
                .unwrap();
                assert_eq!(actual, expected);
                reader.clear_overlay();
            }
        }
        // 已命中的精确record仍须在每cut核life、成员、birth、observations与frontier。
        for sql in ["UPDATE s_content_objects SET kind='other' WHERE object_id='obj3-old'", "UPDATE s_content_objects SET withdrawn_generation=2,withdrawal_reason='reason' WHERE object_id='obj3-old'", "DELETE FROM s_content_objects WHERE object_id='obj3-old'", "UPDATE s_content_objects SET batch_id='other' WHERE object_id='obj3-old'", "UPDATE s_content_objects SET first_known_generation=1,first_known_cut='cut-1' WHERE object_id='obj3-new'", "UPDATE s_content_observations SET published_generation=1 WHERE observation_id='ob-new'"] {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::Observations);
            let capture = sc::AuditCapture::new(native_publication_fields_nodes(&tx));
            let mut reader = HashReader::native_publication_fields_for_test(&capture);
            let mut memo = NativePublicationMemo::new(&reader).unwrap();
            resolve_native_indexed_templates_pending_view(&tx, &mut reader, m.view(), roots.view(), &manifest.batch_id, 0, &mut memo).unwrap();
            reader.clear_overlay();
            tx.execute(sql, []).unwrap();
            let expected = resolve_indexed_templates_pending_view(&tx, &mut reader, m.view(), roots.view(), &manifest.batch_id, 0).err().unwrap();
            reader.clear_overlay();
            assert_eq!(resolve_native_indexed_templates_pending_view(&tx, &mut reader, m.view(), roots.view(), &manifest.batch_id, 0, &mut memo).err().unwrap(), expected);
        }
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures(&tx);
        let capture = sc::AuditCapture::new(native_publication_fields_nodes(&tx));
        let mut reader = HashReader::native_publication_fields_for_test(&capture);
        let mut memo = NativePublicationMemo::new(&reader).unwrap();
        assert!(resolve_native_indexed_templates_pending_view(
            &tx,
            &mut reader,
            m.view(),
            roots.view(),
            &manifest.batch_id,
            m.input_frontier,
            &mut memo
        )
        .is_err());
        let invalid_snapshot = reader
            .replace_ref(
                (&roots.publication_template).into(),
                &[field("index_frontier")],
                &manifest.batch_id,
            )
            .unwrap();
        let mut invalid_roots = roots.view();
        invalid_roots.publication_template = invalid_snapshot.view();
        let expected = resolve_indexed_templates_pending_view(
            &tx,
            &mut reader,
            m.view(),
            invalid_roots,
            &manifest.batch_id,
            0,
        )
        .err()
        .unwrap();
        reader.clear_overlay();
        // 非capture临时根在清除后重建，再走同一判据，不缓存该来源。
        let invalid_snapshot = reader
            .replace_ref(
                (&roots.publication_template).into(),
                &[field("index_frontier")],
                &manifest.batch_id,
            )
            .unwrap();
        invalid_roots.publication_template = invalid_snapshot.view();
        assert_eq!(
            resolve_native_indexed_templates_pending_view(
                &tx,
                &mut reader,
                m.view(),
                invalid_roots,
                &manifest.batch_id,
                0,
                &mut memo
            )
            .err()
            .unwrap(),
            expected
        );
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_publication_same_real_indexed_template_roots_and_f() {
        use sc::capture_allocation::{CaptureSlab, OwnedBudget};
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures(&tx);
        let mut nodes = sc::MemoryNodes::new();
        let mut statement = tx
            .prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let digest: Vec<u8> = row.get(0).unwrap();
            let t: u8 = row.get(1).unwrap();
            let body: Vec<u8> = row.get(2).unwrap();
            nodes.insert_unchecked(digest.try_into().unwrap(), t, body);
        }
        drop(rows);
        drop(statement);
        let legacy = sc::AuditCapture::new(nodes.clone());
        let legacy_budget = OwnedBudget::new(2 * 1024 * 1024);
        let (normal, normal_f) = crate::v3::audit_store::with_owned_store_probe(
            &legacy,
            &legacy_budget,
            1024,
            65536,
            |store| {
                let f = sc::frame_calls_on_thread();
                let result = resolve_indexed_templates_pending(
                    &tx,
                    store,
                    &m,
                    &roots,
                    &manifest.batch_id,
                    0,
                )
                .unwrap();
                (result, sc::frame_calls_on_thread() - f)
            },
        );
        assert_eq!(legacy_budget.snapshot().live_requested_layout_bytes, 0);
        let count = nodes.len();
        let body_bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let base = CaptureSlab::requested_bytes(count, body_bytes).unwrap();
        let budget = OwnedBudget::new(base + 2 * 1024 * 1024);
        let mut slab = CaptureSlab::new(count, body_bytes, &budget).unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let meta_owner = content_manifest::MetaOwner::from_view(m.view(), &budget).unwrap();
        let roots_owner = content_manifest::RootsOwner::from_view(roots.view()).unwrap();
        let (owned, owned_f) = crate::v3::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            1024,
            65536,
            |store| {
                let f = sc::frame_calls_on_thread();
                let result = resolve_indexed_templates_pending_view(
                    &tx,
                    store,
                    meta_owner.view(),
                    roots_owner.view(),
                    &manifest.batch_id,
                    0,
                )
                .unwrap();
                (result, sc::frame_calls_on_thread() - f)
            },
        );
        assert_eq!(owned.snapshot.view().root(), normal.snapshot.root);
        assert_eq!(owned.delta.view().root(), normal.delta.root);
        assert_eq!(owned.snapshot.view().kind(), normal.snapshot.kind);
        assert_eq!(owned.delta.view().extent(), normal.delta.extent);
        assert!(matches!(owned.snapshot, RefValue::Owned(_)));
        assert!(matches!(owned.delta, RefValue::Owned(_)));
        assert_eq!(owned_f, normal_f);
        // The same invalid frontier rejects at the same source-independent gate.
        let expected = crate::v3::audit_store::with_owned_store_probe(
            &legacy,
            &legacy_budget,
            1024,
            65536,
            |store| {
                resolve_indexed_templates_pending(
                    &tx,
                    store,
                    &m,
                    &roots,
                    &manifest.batch_id,
                    m.input_frontier,
                )
                .err()
                .unwrap()
            },
        );
        let actual = crate::v3::audit_store::with_owned_store_probe(
            &capture,
            &budget,
            1024,
            65536,
            |store| {
                resolve_indexed_templates_pending_view(
                    &tx,
                    store,
                    meta_owner.view(),
                    roots_owner.view(),
                    &manifest.batch_id,
                    m.input_frontier,
                )
                .err()
                .unwrap()
            },
        );
        assert_eq!(actual, expected);
        println!(
            "OWNED_PUBLICATION_RECEIPT {}",
            serde_json::json!({"normal_f":normal_f,"owned_f":owned_f,"snapshot":owned.snapshot.root(),"delta":owned.delta.root(),"meter":budget.snapshot(),"error":actual,"whole_audit_allocation_admitted":false})
        );
        drop(owned);
        drop(meta_owner);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_publication_begin_and_consumers_share_parent_scope() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::Observations);
        let token = meta(&tx, "begin_token").unwrap();
        verify_current_begin(&tx, &token, &m).unwrap();
        let expected_frontier = current_publication_frontier(&tx).unwrap();
        let expected = resolve_indexed_templates(
            &tx,
            &mut content_store::TxStore::new(&tx).unwrap(),
            &m,
            &roots,
            &manifest.batch_id,
            expected_frontier,
        )
        .unwrap();
        let budget = crate::v3::audit_owner::native::Budget::new(64 * 1024 * 1024, 0).unwrap();
        let before = budget.live();
        let actual = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                verify_current_begin_admitted(&tx, &token, m.view(), Some(work))?;
                assert_eq!(
                    current_publication_frontier_admitted(&tx, Some(work))?,
                    expected_frontier
                );
                // premerge-0086 第十八处裁改（待负责人确认）：begin 校验的临时文本/解析值是实际
                // 载荷许可，按 P1c（control/1488-budget-full-chain-counterexample-fix-v2.md:9，
                // 959d860065 起 RAII）真实析构即还父账，不再保守持有到父预算结束；
                // 原断言 `begin_held > before + M` 改为：确曾在同一父账出生（峰值）且返回后只剩
                // 本工作域元数据 M。
                let begin_held = budget.live();
                let metadata = sc::capture_allocation::scratch::CodecWork::metadata_bytes();
                assert!(
                    budget.peak() > before + metadata,
                    "begin 校验沿同一父账出生"
                );
                assert_eq!(
                    begin_held,
                    before + metadata,
                    "begin 临时许可随真实析构释放"
                );
                let mut store = content_store::TxStore::new(&tx)
                    .map_err(err)?
                    .with_work(Some(work));
                let resolved = resolve_indexed_templates(
                    &tx,
                    &mut store,
                    &m,
                    &roots,
                    &manifest.batch_id,
                    expected_frontier,
                )?;
                assert!(
                    budget.live() > begin_held,
                    "出生表、集合、路径和引用沿同一父账持有"
                );
                let held = budget.live();
                drop(store);
                assert_eq!(budget.live(), held, "Native许可不得随借用store提前释放");
                Ok::<_, String>(resolved)
            })
            .unwrap();
        assert_eq!(actual.snapshot, expected.snapshot);
        assert_eq!(actual.delta, expected.delta);
        assert!(budget.peak() > before);
        // P1c RAII：结果许可持原父账直至真实析构，析构后全额还账（0088 P3-1）。
        assert!(budget.live() > before, "结果许可持原父账直至真实析构");
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
        drop(actual);
        assert_eq!(budget.live(), before, "结果真实析构后父账全额还原");
    }

    #[test]
    fn historical_template_resolution_ignores_future_births_and_withdrawals() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures(&tx);
        let mut store = content_store::TxStore::new(&tx).unwrap();
        let before =
            resolve_indexed_templates(&tx, &mut store, &m, &roots, &manifest.batch_id, 0).unwrap();
        meta_set(&tx, "generation", "3").unwrap();
        tx.execute("UPDATE s_content_objects SET withdrawn_generation=3,withdrawal_reason='superseded',superseded_by='future-object' WHERE object_id='obj3-old'",[]).unwrap();
        let future = sc::write_value(&mut store, &V::Null).unwrap();
        tx.execute("INSERT INTO s_content_objects VALUES('future-object',1,'CC-010.bi','future-key',NULL,NULL,NULL,?1,?1,?1,?1,'future-batch',3,'cut-3',3,NULL,NULL,NULL)",params![future.digest.as_slice()]).unwrap();
        let node = sc::write_value(&mut store, &V::Null).unwrap();
        tx.execute("INSERT INTO s_content_observations VALUES('future-observation','insufficient_knowledge',NULL,'future-batch',?1,?1,3)",params![node.digest.as_slice()]).unwrap();
        let after =
            resolve_indexed_templates(&tx, &mut store, &m, &roots, &manifest.batch_id, 0).unwrap();
        assert_eq!(before.snapshot, after.snapshot);
        assert_eq!(before.delta, after.delta);
    }
    #[test]
    fn content_publication_resolves_new_birth_and_preserves_old_batch_and_payload() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures(&tx);
        let token = meta(&tx, "begin_token").unwrap();
        let resolved = resolve_current_births(&tx, &token, &m, &roots, &manifest).unwrap();
        let mut store = content_store::ReadStore::new(&tx).unwrap();
        let snapshot = rendered(&mut store, &resolved.snapshot);
        let delta = rendered(&mut store, &resolved.delta);
        assert_eq!(
            snapshot["objects"][0]["batch_id"],
            format!("batch3-{}", "d".repeat(64))
        );
        assert_eq!(
            snapshot["objects"][1]["batch_id"],
            manifest.batch_id.as_str()
        );
        assert_eq!(
            snapshot["objects"][1]["payload"]["batch_id"],
            "原文里的同名字段"
        );
        assert_eq!(delta["upserts"][0], snapshot["objects"][1]);
        assert_eq!(snapshot["index_frontier"], manifest.batch_id.as_str());
        assert_eq!(
            delta["catalog_evidence"]["batch_id"],
            manifest.batch_id.as_str()
        );
        assert_eq!(
            super::super::meta_get_opt(&tx, "generation")
                .unwrap()
                .as_deref(),
            Some("1")
        );
    }

    #[test]
    fn content_publication_refuses_missing_or_changed_independent_authority() {
        for sql in [
            "DELETE FROM s_content_objects WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET first_known_generation=1 WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET first_known_cut='cut-1' WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET batch_id='other' WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET kind='CC-056.version' WHERE object_id='obj3-new'",
            "UPDATE s_content_objects SET published_generation=2 WHERE object_id='obj3-old'",
            "UPDATE meta SET value='3' WHERE key='writer_epoch'",
            "UPDATE meta SET value='other-token' WHERE key='begin_token'",
            "UPDATE s_protocol_meta SET next_attempt_ordinal=3",
            "UPDATE s_protocol_meta SET logical_phase_frontier='20'",
            "DELETE FROM s_clock_events WHERE phase='begin'",
            "UPDATE s_input_messages SET status='Committed',first_published_generation=2",
            "UPDATE s_input_messages SET attempt_count=2",
            "UPDATE s_input_messages SET receipt_id='other-receipt'",
            "UPDATE meta SET value='1' WHERE key='last_advance_frontier'",
            "UPDATE raw_events SET seq=2",
        ] {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            let (m, roots, manifest) = fixtures(&tx);
            let token = meta(&tx, "begin_token").unwrap();
            tx.execute(sql, []).unwrap();
            assert!(
                resolve_current_births(&tx, &token, &m, &roots, &manifest).is_err(),
                "必须拒绝：{sql}"
            );
        }
    }

    #[test]
    fn content_publication_refuses_self_asserted_or_out_of_schema_placeholders() {
        for change in [
            FixtureChange::PayloadPlaceholder,
            FixtureChange::OldPlaceholder,
            FixtureChange::CurrentBatchString,
            FixtureChange::Duplicate,
            FixtureChange::OldObservationPlaceholder,
        ] {
            let mut db = Connection::open_in_memory().unwrap();
            let tx = db.transaction().unwrap();
            let (m, roots, manifest) = fixtures_with(&tx, change);
            let token = meta(&tx, "begin_token").unwrap();
            let count = |tx: &Transaction<'_>| -> i64 {
                tx.query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
                    .unwrap()
            };
            let before = count(&tx);
            tx.execute_batch("SAVEPOINT rejected_publication").unwrap();
            let error = resolve_current_births(&tx, &token, &m, &roots, &manifest).unwrap_err();
            if matches!(change, FixtureChange::PayloadPlaceholder) {
                assert!(error.contains(sc::code::TEMPLATE_PLACEHOLDER), "{error}");
            }
            // 操作不自行提交；调用者回滚使包括部分祖先重写在内的所有节点回到原状。
            tx.execute_batch("ROLLBACK TO rejected_publication; RELEASE rejected_publication")
                .unwrap();
            assert_eq!(count(&tx), before);
            assert_eq!(meta(&tx, "generation").unwrap(), "1");
        }
    }

    #[test]
    fn content_publication_observation_birth_is_independent_and_old_batch_stays() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::Observations);
        let token = meta(&tx, "begin_token").unwrap();
        let resolved = resolve_current_births(&tx, &token, &m, &roots, &manifest).unwrap();
        let mut store = content_store::ReadStore::new(&tx).unwrap();
        let snapshot = rendered(&mut store, &resolved.snapshot);
        let delta = rendered(&mut store, &resolved.delta);
        assert_eq!(
            snapshot["observations"][0]["batch_id"],
            format!("batch3-{}", "e".repeat(64))
        );
        assert_eq!(
            snapshot["observations"][1]["batch_id"],
            manifest.batch_id.as_str()
        );
        assert_eq!(delta["observations"], json!([snapshot["observations"][1]]));
        assert_eq!(
            snapshot["observations"][1]["detail"]["batch_id"],
            "原样保留"
        );
        tx.execute(
            "UPDATE s_content_observations SET published_generation=1 WHERE observation_id='ob-new'",
            [],
        )
        .unwrap();
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest).is_err());
    }

    #[test]
    fn content_publication_rejects_unrelated_manifest_and_source_producer() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (mut m, roots, mut manifest) = fixtures(&tx);
        let token = meta(&tx, "begin_token").unwrap();
        let actual = manifest.batch_id.as_str().to_owned();
        manifest.batch_id = format!("batch3-{}", "f".repeat(64)).into();
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest)
            .unwrap_err()
            .contains("IdentityConflict"));
        manifest.batch_id = actual.into();
        m.source_commit.producer_id = "wrong-producer".into();
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest)
            .unwrap_err()
            .contains("producer_id"));
    }

    #[test]
    fn content_publication_s3_rejects_legacy_full_delta_without_private_mode_fallback() {
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        let (m, roots, manifest) = fixtures_with(&tx, FixtureChange::LegacyFullDelta);
        let token = meta(&tx, "begin_token").unwrap();
        let count: i64 = tx
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name='s_storage_format'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert!(resolve_current_births(&tx, &token, &m, &roots, &manifest)
            .unwrap_err()
            .contains("upserts 与独立索引成员数不符"));
    }
}

/// #1640 普通只读历史的字段投影，不保存任何逐 cut 业务结论。
pub(crate) struct PublicationProjection<'scope, 'tx> {
    transaction: &'scope Transaction<'tx>,
    reader: *const (),
    previous: std::collections::HashMap<PublicationKey, std::rc::Rc<RefFields<'static>>>,
    current: std::collections::HashMap<PublicationKey, std::rc::Rc<RefFields<'static>>>,
    batch_previous: std::collections::HashMap<PublicationKey, ContentRef>,
    batch_current: std::collections::HashMap<PublicationKey, ContentRef>,
    peak_bytes: usize,
}
type PublicationKey = (String, String, &'static str, Option<u64>);
fn publication_key(record: sc::RefView<'_>) -> PublicationKey {
    (
        record.format().into(),
        record.root().into(),
        record.kind().name(),
        record.extent(),
    )
}
impl<'scope, 'tx> PublicationProjection<'scope, 'tx> {
    pub(crate) fn new(
        tx: &'scope Transaction<'tx>,
        store: &HashReader<'_>,
    ) -> Result<Self, String> {
        if !tx.is_readonly(rusqlite::MAIN_DB).map_err(err)?
            || !store.ordinary_bound_births_qualified()
        {
            return Err(err("ordinary投影需要同事务只读原件"));
        }
        Ok(Self {
            transaction: tx,
            reader: store as *const _ as *const (),
            previous: Default::default(),
            current: Default::default(),
            batch_previous: Default::default(),
            batch_current: Default::default(),
            peak_bytes: 0,
        })
    }
    fn check(&self, tx: &Transaction<'_>, store: &HashReader<'_>) -> Result<(), String> {
        if !std::ptr::eq(tx, self.transaction)
            || self.reader != store as *const _ as *const ()
            || !store.ordinary_bound_births_qualified()
        {
            return Err(err("ordinary投影事务/reader不符"));
        }
        Ok(())
    }
    fn read_fields(
        &mut self,
        store: &mut (impl CodecStore + ?Sized),
        r: sc::RefView<'_>,
    ) -> Result<PublicationFields, String> {
        let k = publication_key(r);
        let fields = match self.current.get(&k).or_else(|| self.previous.get(&k)) {
            Some(fields) => std::rc::Rc::clone(fields),
            None => std::rc::Rc::new(read_publication_fields(store, r)?),
        };
        self.current.insert(k, std::rc::Rc::clone(&fields));
        Ok(PublicationFields::Ordinary(fields))
    }
    fn read_batch(
        &mut self,
        store: &mut (impl CodecStore + ?Sized),
        r: sc::RefView<'_>,
    ) -> Result<RefValue, String> {
        let k = publication_key(r);
        let value = match self
            .batch_current
            .get(&k)
            .or_else(|| self.batch_previous.get(&k))
        {
            Some(value) => value.clone(),
            None => {
                let value = store.lookup_field_ref(r, "batch_id").map_err(err)?;
                ContentRef {
                    format: value.view().format().into(),
                    root: value.root().into(),
                    kind: value.kind(),
                    extent: value.extent(),
                }
            }
        };
        self.batch_current.insert(k, value.clone());
        Ok(value.into())
    }
    pub(crate) fn commit_cut(&mut self) {
        self.measure();
        self.previous = std::mem::take(&mut self.current);
        self.batch_previous = std::mem::take(&mut self.batch_current);
        self.measure();
    }
    fn measure(&mut self) {
        let mut pointers = std::collections::HashSet::new();
        let mut bytes = 0usize;
        for map in [&self.previous, &self.current] {
            bytes += map.capacity()
                * (std::mem::size_of::<(PublicationKey, std::rc::Rc<RefFields<'static>>)>() + 1);
            for (k, fields) in map {
                bytes += k.0.capacity() + k.1.capacity();
                if pointers.insert(std::rc::Rc::as_ptr(fields)) {
                    if let RefFields::Plain(fields) = &**fields {
                        bytes += fields
                            .plain_retained_bytes()
                            .expect("ordinary retained fields")
                            + std::mem::size_of::<RefFields<'static>>()
                            + 2 * std::mem::size_of::<usize>();
                    }
                }
            }
        }
        for map in [&self.batch_previous, &self.batch_current] {
            bytes += map.capacity() * (std::mem::size_of::<(PublicationKey, ContentRef)>() + 1);
            bytes += map
                .iter()
                .map(|(k, r)| {
                    k.0.capacity() + k.1.capacity() + r.format.capacity() + r.root.capacity()
                })
                .sum::<usize>();
        }
        self.peak_bytes = self.peak_bytes.max(bytes);
        #[cfg(test)]
        ORDINARY_PUBLICATION_BYTES.with(|stats| stats.set((bytes, self.peak_bytes)));
    }
}
impl Drop for PublicationProjection<'_, '_> {
    fn drop(&mut self) {
        self.measure();
        #[cfg(test)]
        ORDINARY_PUBLICATION_BYTES.with(|stats| stats.set((0, self.peak_bytes)));
    }
}
#[cfg(test)]
thread_local! { static ORDINARY_PUBLICATION_BYTES: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0,0)) }; }

pub(crate) fn resolve_ordinary_indexed_templates_pending_view(
    tx: &Transaction<'_>,
    store: &mut HashReader<'_>,
    metadata: MetadataView<'_>,
    roots: RootsView<'_>,
    batch_id: &str,
    previous_frontier: i64,
    projection: &mut PublicationProjection<'_, '_>,
) -> Result<PendingRefTemplates, String> {
    projection.check(tx, store)?;
    resolve_indexed_templates_pending_core(
        tx,
        store,
        metadata,
        roots,
        batch_id,
        previous_frontier,
        None,
        Some(projection),
    )
}
