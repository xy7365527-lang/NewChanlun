//! #1394 显式Native模式的来源Counts完整闭包复用；保留量受唯一S7预算约束。
//! 完整当前audit只跳过绑定成功旧cut的source业务核；仅整组保留Counts导航及三个完整闭包。
use super::{
    audit_store::{CodecRead, NativeAdmission},
    history::PublishedCut,
    request_audit::{ExactRows, SqlCell, SqlCellView},
    source_audit::Counts,
};
use newchan_rust::s_content::{self as sc, capture_allocation::scratch::WorkScope};
use rusqlite::Transaction;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq)]
struct Binding {
    frontier: i64,
    rows: Vec<ExactRows>,
}
impl Binding {
    #[cfg_attr(
        feature = "s_capture_native_testonly",
        allow(dead_code, reason = "#1608：s_capture_native_testonly 构建不触达此项")
    )]
    fn read(tx: &Transaction<'_>, frontier: i64) -> Result<Self, String> {
        Self::read_admitted(tx, frontier, None)
    }
    fn read_admitted(
        tx: &Transaction<'_>,
        frontier: i64,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        let mut meta = ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM meta ORDER BY rowid",
            &[],
            &[],
            work,
        )?;
        // 已知动态控制字段只由本次完整control/history审计消费；未知字段不悄悄排除。
        meta.rows.retain(|row| !matches!(row.get(1), Some(SqlCell::Text(key)) if [
            b"generation".as_slice(), b"structure_cut", b"index_frontier", b"last_advance_frontier",
            b"writer_epoch", b"begin_generation", b"begin_input_frontier", b"begin_token", b"advance_state",
        ].contains(&key.as_slice())));
        let mut rows = Vec::new();
        NativeAdmission(work)
            .native_vec_room(&mut rows, 6)
            .map_err(|e| e.to_string())?;
        rows.push(ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM sqlite_schema ORDER BY rowid",
            &[],
            &[],
            work,
        )?);
        rows.push(meta);
        rows.push(ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM s_content_meta ORDER BY rowid",
            &[],
            &[],
            work,
        )?);
        rows.push(ExactRows::read_admitted(tx, c"SELECT rowid,singleton,protocol_revision,session_generation,clock_plan_hash,clock_plan_json FROM s_protocol_meta ORDER BY rowid", &[], &[], work)?);
        let frontier_scope = [SqlCellView::Integer(frontier)];
        rows.push(ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM raw_events WHERE seq<=?1 ORDER BY seq,rowid",
            &frontier_scope,
            &frontier_scope,
            work,
        )?);
        // 保留独立OHLC负依赖；当前全账enrich/cardinality仍由原入口fresh核验。
        rows.push(ExactRows::read_admitted(tx, c"SELECT o.rowid,o.* FROM raw_ohlc o WHERE NOT EXISTS(SELECT 1 FROM raw_events r WHERE r.identity_key=o.identity_key AND r.revision=o.revision AND r.seq>?1) ORDER BY o.rowid", &frontier_scope, &frontier_scope, work)?);
        Ok(Self { frontier, rows })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    generation: i64,
    frontier: i64,
    state: sc::ContentRef,
    profile: String,
    rule: String,
}
impl Key {
    #[cfg(test)]
    fn of(cut: &PublishedCut) -> Self {
        let r = cut.roots.generated_state.view();
        Self {
            generation: cut.metadata.generation,
            frontier: cut.metadata.input_frontier,
            state: sc::ContentRef {
                format: r.format().into(),
                root: r.root().into(),
                kind: r.kind(),
                extent: r.extent(),
            },
            profile: cut.metadata.profile_id.as_str().into(),
            rule: cut.metadata.rule_revision.as_str().into(),
        }
    }
    fn of_admitted(cut: &PublishedCut, work: Option<&WorkScope>) -> Result<Self, String> {
        let admission = NativeAdmission(work);
        Ok(Self {
            generation: cut.metadata.generation,
            frontier: cut.metadata.input_frontier,
            state: admission
                .native_plain_ref(cut.roots.generated_state.view())
                .map_err(|e| e.to_string())?,
            profile: admission
                .native_string(cut.metadata.profile_id.as_str())
                .map_err(|e| e.to_string())?,
            rule: admission
                .native_string(cut.metadata.rule_revision.as_str())
                .map_err(|e| e.to_string())?,
        })
    }
    fn matches_cut(&self, cut: &PublishedCut) -> bool {
        let state = cut.roots.generated_state.view();
        self.generation == cut.metadata.generation
            && self.frontier == cut.metadata.input_frontier
            && self.state.format == state.format()
            && self.state.root == state.root()
            && self.state.kind == state.kind()
            && self.state.extent == state.extent()
            && self.profile == cut.metadata.profile_id.as_str()
            && self.rule == cut.metadata.rule_revision.as_str()
    }
}
struct Proof {
    key: Key,
    counts: Counts,
    dependency: Option<sc::NativeCountsSubsetCertificate>,
}
pub(super) struct Pool {
    pub(super) originals: sc::AuditCapture,
    pub(super) nodes: usize,
    binding: Binding,
    proofs: Vec<Proof>,
    source_cost: SourceCost,
    capacity: RetainedCapacity,
    subset_selection: Option<sc::NativeCountsSubsetReport>,
}

#[derive(Default, Clone, Debug, Serialize)]
pub(super) struct SourceCost {
    node_body_bytes: usize,
    node_digest_tag_certificate_bytes: usize,
    exact_rows_bytes: usize,
    cut_key_counts_certificate_bytes: usize,
    codec_root_certificate_bytes: usize,
    counts_dependency_certificate_bytes: usize,
    pub(super) total: usize,
}
#[derive(Default, Clone, Debug, Serialize)]
pub(super) struct RetainedCapacity {
    capture_owner_bytes: usize,
    node_body_vec_capacity_bytes: usize,
    native_index_capacity_payload_bytes: usize,
    exact_rows_capacity_payload_bytes: usize,
    proof_vec_capacity_payload_bytes: usize,
    key_string_capacity_bytes: usize,
    codec_root_set_capacity_payload_bytes: usize,
    counts_dependency_string_capacity_bytes: usize,
    pub(super) total_payload_bytes: usize,
}
fn add(a: usize, b: usize) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| "SourceCachePrototypeCostOverflow".into())
}
fn mul(a: usize, b: usize) -> Result<usize, String> {
    a.checked_mul(b)
        .ok_or_else(|| "SourceCachePrototypeCostOverflow".into())
}
fn cell_cost(cell: &SqlCell) -> Result<usize, String> {
    add(
        1,
        match cell {
            SqlCell::Null => 0,
            SqlCell::Integer(_) | SqlCell::Real(_) => 8,
            SqlCell::Text(v) | SqlCell::Blob(v) => add(8, v.len())?,
        },
    )
}
fn cells_cost(cells: &[SqlCell]) -> Result<usize, String> {
    cells
        .iter()
        .try_fold(8, |sum, cell| add(sum, cell_cost(cell)?))
}
fn cells_capacity(cells: &Vec<SqlCell>) -> Result<usize, String> {
    cells.iter().try_fold(
        mul(cells.capacity(), std::mem::size_of::<SqlCell>())?,
        |sum, cell| {
            add(
                sum,
                match cell {
                    SqlCell::Text(v) | SqlCell::Blob(v) => v.capacity(),
                    _ => 0,
                },
            )
        },
    )
}
fn binding_source_bytes(binding: &Binding) -> Result<usize, String> {
    binding.rows.iter().try_fold(0, |sum, row| {
        let mut cost = add(8, row.sql.len())?;
        cost = add(cost, cells_cost(&row.scope)?)?;
        cost = add(cost, 8)?;
        for column in &row.columns {
            cost = add(cost, add(8, column.len())?)?;
        }
        cost = add(cost, 8)?;
        for cells in &row.rows {
            cost = add(cost, cells_cost(cells)?)?;
        }
        add(sum, cost)
    })
}
fn proof_source_bytes(proof: &Proof) -> Result<usize, String> {
    [
        &proof.key.state.format,
        &proof.key.state.root,
        &proof.key.profile,
        &proof.key.rule,
    ]
    .into_iter()
    .try_fold(
        16 + 1 + 9 + 64 + 6 * std::mem::size_of::<usize>(),
        |sum, text| add(sum, add(8, text.len())?),
    )
}
fn codec_root_source_bytes(roots: usize) -> Result<usize, String> {
    add(
        add(add(8, sc::CONTENT_FORMAT.len())?, 4)?,
        add(mul(2, 1 + 1 + 8)?, mul(roots, 32)?)?,
    )
}
impl Pool {
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    fn build_bounded_counts(
        originals: sc::AuditCapture,
        binding: Binding,
        proofs: Vec<Proof>,
        cap: usize,
    ) -> Result<Self, String> {
        Self::build_bounded_counts_admitted(originals, binding, proofs, cap, None)
    }
    fn build_bounded_counts_admitted(
        originals: sc::AuditCapture,
        binding: Binding,
        mut proofs: Vec<Proof>,
        cap: usize,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        if !proofs.is_sorted_by_key(|proof| proof.key.generation) {
            // 保留原stable sort及同代顺序。Rust 1.98.0 stable/mod.rs:113-116的
            // alloc_len=max(max(n-n/2,min(n,8MB/size)),48)，故max(n,48)是元素上界；
            // 仅真需排序时许可临时完整块，sort返回后释放，不用平方级替代算法。
            let sort_permit = work
                .map(|work| {
                    let bytes = proofs
                        .len()
                        .max(48)
                        .checked_mul(std::mem::size_of::<Proof>())
                        .ok_or_else(|| {
                            work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget)
                        })?;
                    work.reserve_native(bytes)
                })
                .transpose()
                .map_err(|e| e.to_string())?;
            proofs.sort_by_key(|proof| proof.key.generation);
            drop(sort_permit);
        }
        let admission = NativeAdmission(work);
        let fixed = add(
            add(8, binding_source_bytes(&binding)?)?,
            codec_root_source_bytes(0)?,
        )?;
        // history原严格升序；只反向尝试一次，接纳后仍恢复原binary_search代序。
        let mut groups = Vec::new();
        admission
            .native_vec_room(&mut groups, proofs.len())
            .map_err(|e| e.to_string())?;
        for proof in proofs.iter().rev() {
            groups.push((
                admission
                    .native_plain_ref((&proof.key.state).into())
                    .map_err(|e| e.to_string())?,
                proof_source_bytes(proof)?,
            ));
        }
        let (originals, certificates, selection) = originals
            .into_counts_original_subset_admitted_for_testonly(groups, fixed, cap, work)
            .map_err(|e| e.to_string())?;
        let remove = proofs
            .len()
            .checked_sub(certificates.len())
            .ok_or("CountsSubsetProofCountInvalid")?;
        proofs.drain(..remove);
        for (proof, certificate) in proofs.iter_mut().rev().zip(certificates) {
            if !certificate.matches_state(&proof.key.state) {
                return Err("CountsSubsetStateCertificateInvalid".into());
            }
            proof.dependency = Some(certificate);
        }
        if let Some(work) = work {
            if proofs.len() < proofs.capacity() {
                // 缩容也可能分配：旧块仍在时先许可完整新块，再只移动Proof。
                let bytes = proofs
                    .len()
                    .checked_mul(std::mem::size_of::<Proof>())
                    .ok_or_else(|| {
                        work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget)
                            .to_string()
                    })?;
                let permit = work.reserve_native(bytes).map_err(|e| e.to_string())?;
                let mut compact = Vec::new();
                compact.try_reserve_exact(proofs.len()).map_err(|_| {
                    work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
                        .to_string()
                })?;
                compact.append(&mut proofs);
                proofs = compact;
                permit.retain();
            }
        } else {
            proofs.shrink_to_fit();
        }
        let nodes = originals
            .native_retention_numbers_for_testonly()
            .ok_or("CountsSubsetNodeCountInvalid")?
            .nodes;
        let mut pool = Self::build(originals, nodes, binding, proofs)?;
        if !pool.proofs.is_empty()
            && (pool.source_cost.total != selection.retained_source_bytes
                || pool.source_cost.total > cap)
        {
            return Err("CountsSubsetExactFeeMismatch".into());
        }
        pool.subset_selection = Some(selection);
        Ok(pool)
    }
    fn build(
        originals: sc::AuditCapture,
        nodes: usize,
        binding: Binding,
        proofs: Vec<Proof>,
    ) -> Result<Self, String> {
        // Drop全部Public/Template/Frame memo，保留原件owner及两份纯codec成功根集合。
        let originals = originals
            .into_originals_and_codec_roots_for_testonly()
            .map_err(|e| e.to_string())?;
        let retained = originals
            .native_retention_numbers_for_testonly()
            .ok_or("SourceCachePrototypeRetentionInvalid")?;
        let mut source_cost = SourceCost {
            node_body_bytes: retained.body_len,
            // 原tuple digest/tag和唯一原件证书；无独立children/slots，不虚加收费。
            node_digest_tag_certificate_bytes: mul(nodes, 32 + 1 + 64)?,
            ..Default::default()
        };
        let mut capacity = RetainedCapacity {
            capture_owner_bytes: std::mem::size_of::<sc::AuditCapture>(),
            node_body_vec_capacity_bytes: retained.body_capacity,
            native_index_capacity_payload_bytes: retained.index_payload,
            exact_rows_capacity_payload_bytes: mul(
                binding.rows.capacity(),
                std::mem::size_of::<ExactRows>(),
            )?,
            proof_vec_capacity_payload_bytes: mul(proofs.capacity(), std::mem::size_of::<Proof>())?,
            ..Default::default()
        };
        let root_count = add(retained.public_roots, retained.template_roots)?;
        // #1394本原型新增源成本配方，非SPEC原有配方：format长度前缀+字节，
        // 判据版本u32；两个集合各含Mode/Flavor字节和u64长度前缀；每个根32字节。
        source_cost.codec_root_certificate_bytes = codec_root_source_bytes(root_count)?;
        capacity.codec_root_set_capacity_payload_bytes = mul(
            add(retained.public_capacity, retained.template_capacity)?,
            std::mem::size_of::<sc::NodeDigest>(),
        )?;
        for row in &binding.rows {
            let mut cost = add(8, row.sql.len())?;
            cost = add(cost, cells_cost(&row.scope)?)?;
            cost = add(cost, 8)?;
            for column in &row.columns {
                cost = add(cost, add(8, column.len())?)?;
            }
            cost = add(cost, 8)?;
            for cells in &row.rows {
                cost = add(cost, cells_cost(cells)?)?;
            }
            source_cost.exact_rows_bytes = add(source_cost.exact_rows_bytes, cost)?;
            let mut bytes = add(row.sql.capacity(), cells_capacity(&row.scope)?)?;
            bytes = add(
                bytes,
                mul(row.columns.capacity(), std::mem::size_of::<String>())?,
            )?;
            for column in &row.columns {
                bytes = add(bytes, column.capacity())?;
            }
            bytes = add(
                bytes,
                mul(row.rows.capacity(), std::mem::size_of::<Vec<SqlCell>>())?,
            )?;
            for cells in &row.rows {
                bytes = add(bytes, cells_capacity(cells)?)?;
            }
            capacity.exact_rows_capacity_payload_bytes =
                add(capacity.exact_rows_capacity_payload_bytes, bytes)?;
        }
        for proof in &proofs {
            let strings = [
                &proof.key.state.format,
                &proof.key.state.root,
                &proof.key.profile,
                &proof.key.rule,
            ];
            let mut bytes = 16 + 1 + 9 + 64 + 6 * std::mem::size_of::<usize>();
            for text in strings {
                bytes = add(bytes, add(8, text.len())?)?;
                capacity.key_string_capacity_bytes =
                    add(capacity.key_string_capacity_bytes, text.capacity())?;
            }
            source_cost.cut_key_counts_certificate_bytes =
                add(source_cost.cut_key_counts_certificate_bytes, bytes)?;
            if let Some(certificate) = &proof.dependency {
                source_cost.counts_dependency_certificate_bytes = add(
                    source_cost.counts_dependency_certificate_bytes,
                    certificate.source_bytes().map_err(|e| e.to_string())?,
                )?;
                capacity.counts_dependency_string_capacity_bytes = add(
                    capacity.counts_dependency_string_capacity_bytes,
                    certificate.string_capacity_bytes(),
                )?;
            }
        }
        source_cost.total = [
            source_cost.node_body_bytes,
            source_cost.node_digest_tag_certificate_bytes,
            source_cost.exact_rows_bytes,
            source_cost.cut_key_counts_certificate_bytes,
            source_cost.codec_root_certificate_bytes,
            source_cost.counts_dependency_certificate_bytes,
        ]
        .into_iter()
        .try_fold(8, add)?;
        // size_of<Pool>已含inline Capture/Binding/metrics；capture_owner_bytes仅披露，不重复相加。
        capacity.total_payload_bytes = [
            capacity.node_body_vec_capacity_bytes,
            capacity.native_index_capacity_payload_bytes,
            capacity.exact_rows_capacity_payload_bytes,
            capacity.proof_vec_capacity_payload_bytes,
            capacity.key_string_capacity_bytes,
            capacity.codec_root_set_capacity_payload_bytes,
            capacity.counts_dependency_string_capacity_bytes,
        ]
        .into_iter()
        .try_fold(std::mem::size_of::<Self>(), add)?;
        Ok(Self {
            originals,
            nodes,
            binding,
            proofs,
            source_cost,
            capacity,
            subset_selection: None,
        })
    }
}

pub(super) struct Keeper {
    cap: usize,
    pool: Option<Pool>,
    pub(super) hits: usize,
    pub(super) cold: usize,
    pub(super) rejected: usize,
    pub(super) last_required: usize,
    pub(super) peak_two_pool_source_cost: usize,
    pub(super) peak_two_pool_capacity_payload: usize,
}
impl Keeper {
    // 单是frontier就收8字节，完整SQL证书必有额外成本。原S7=1因此完全走旧路径。
    pub(super) fn retention_possible(cap: usize) -> bool {
        cap > 8
    }
    pub(super) fn new(cap: usize) -> Self {
        Self {
            cap,
            pool: None,
            hits: 0,
            cold: 0,
            rejected: 0,
            last_required: 0,
            peak_two_pool_source_cost: 0,
            peak_two_pool_capacity_payload: 0,
        }
    }
    /// The caller has checked the shared S7 remainder. Cache retention never grants
    /// business permission; narrowing a completed keeper can only discard its pool.
    pub(super) fn set_retention_cap(&mut self, cap: usize) {
        self.cap = cap;
        if self.retained_source_bytes() > cap {
            self.clear()
        }
    }
    pub(super) fn clear(&mut self) {
        self.pool = None;
    }
    pub(super) fn take(&mut self) -> Option<Pool> {
        self.pool.take()
    }
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn retention_cap(&self) -> usize {
        self.cap
    }
    pub(super) fn retained_source_bytes(&self) -> usize {
        self.pool.as_ref().map_or(0, |pool| pool.source_cost.total)
    }
    pub(super) fn finish(
        &mut self,
        prior: Option<Pool>,
        next: Pool,
        hits: usize,
        cold: usize,
    ) -> Result<(), String> {
        self.last_required =
            next.subset_selection
                .as_ref()
                .map_or(next.source_cost.total, |selection| {
                    selection
                        .last_attempted_source_bytes
                        .max(next.source_cost.total)
                });
        self.peak_two_pool_source_cost = self.peak_two_pool_source_cost.max(add(
            prior.as_ref().map_or(0, |p| p.source_cost.total),
            next.source_cost.total,
        )?);
        self.peak_two_pool_capacity_payload = self.peak_two_pool_capacity_payload.max(add(
            prior.as_ref().map_or(0, |p| p.capacity.total_payload_bytes),
            next.capacity.total_payload_bytes,
        )?);
        self.hits += hits;
        self.cold += cold;
        // 新到旧完整组接纳；超额只舍弃最后尝试的整组，先前完整组继续保留。
        if next
            .subset_selection
            .as_ref()
            .is_some_and(|selection| selection.budget_stopped)
        {
            self.rejected = self.rejected.saturating_add(1);
        }
        if !next.proofs.is_empty() && next.source_cost.total <= self.cap {
            self.pool = Some(next);
        } else if next.subset_selection.is_none() {
            self.rejected = self.rejected.saturating_add(1);
        }
        drop(prior);
        Ok(())
    }
    /// 请求已成功提交后的可丢弃缓存收尾，不得把缓存失败变成业务失败。
    /// 只消费同次审计的原件和已封存Counts；导航及三闭包按原codec核重走一次。
    /// 不读取SQL、不生成新的业务审计资格；构造失败只丢弃可选缓存。
    pub(super) fn retain_completed(
        &mut self,
        capture: sc::AuditCapture,
        nodes: usize,
        pending: PendingPool,
    ) {
        if self
            .retain_completed_admitted(capture, nodes, pending, None)
            .is_err()
        {
            self.reject_completed();
        }
    }
    /// 与调用方独立CodecWork共核；内层错误必须离开run后才能按可选缓存失败处理。
    pub(super) fn retain_completed_admitted(
        &mut self,
        capture: sc::AuditCapture,
        nodes: usize,
        pending: PendingPool,
        work: Option<&WorkScope>,
    ) -> Result<(), String> {
        self.clear();
        // slot.node_count仍为全capture数；稀疏池nodes由裁剪后的owner单独计数。
        let _ = nodes;
        let next = Pool::build_bounded_counts_admitted(
            capture,
            pending.binding,
            pending.proofs,
            self.cap,
            work,
        )?;
        self.finish(None, next, pending.hits, pending.cold)
    }
    pub(super) fn reject_completed(&mut self) {
        self.clear();
        self.rejected = self.rejected.saturating_add(1);
    }
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({"cap":self.cap,"hits":self.hits,"cold":self.cold,"rejected":self.rejected,"last_required":self.last_required,
            "retained_source_cost":self.pool.as_ref().map_or(0,|p|p.source_cost.total),
            "source_cost_breakdown":self.pool.as_ref().map(|p|&p.source_cost),
            "retained_capacity_breakdown":self.pool.as_ref().map(|p|&p.capacity),
            "retained_capture":self.pool.as_ref().map(|p|p.originals.retention()),
            "counts_subset_selection":self.pool.as_ref().and_then(|p|p.subset_selection.as_ref()),
            "retained_counts_proofs":self.pool.as_ref().map_or(0,|p|p.proofs.len()),
            "peak_two_pool_source_cost":self.peak_two_pool_source_cost,"peak_two_pool_capacity_payload":self.peak_two_pool_capacity_payload,
            "whole_resource_qualified":false,"uncovered":["HashMap control bytes","allocator overhead","transient verifier/overlay","process heap/RSS/IO peak"]})
    }
}
/// 与请求slot共存的候选证书；SQL原件只在完整审计的同一事务中封存。
pub(super) struct PendingPool {
    binding: Binding,
    proofs: Vec<Proof>,
    hits: usize,
    cold: usize,
}
pub(super) struct Trial<'a> {
    prior: Option<&'a Pool>,
    bound: bool,
    proofs: Vec<Proof>,
    pub(super) hits: usize,
    pub(super) cold: usize,
}
impl<'a> Trial<'a> {
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn new(
        tx: &Transaction<'_>,
        prior: Option<&'a Pool>,
        same_nodes: bool,
    ) -> Result<Self, String> {
        Self::new_admitted(tx, prior, same_nodes, None)
    }
    pub(super) fn new_admitted(
        tx: &Transaction<'_>,
        prior: Option<&'a Pool>,
        same_nodes: bool,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        // 仍完整读取原始绑定；node mismatch不能吞掉后续SQL或资源读取错误。
        let bound = match prior {
            Some(pool) => {
                Binding::read_admitted(tx, pool.binding.frontier, work)? == pool.binding
                    && same_nodes
            }
            None => false,
        };
        Ok(Self {
            prior,
            bound,
            proofs: Vec::new(),
            hits: 0,
            cold: 0,
        })
    }
    pub(super) fn lookup(&mut self, cut: &PublishedCut) -> Option<Counts> {
        // proofs由成功history的严格代序构造；避免每cut线扫造成新的平方成本。
        let found = self.prior.filter(|_| self.bound).and_then(|pool| {
            pool.proofs
                .binary_search_by_key(&cut.metadata.generation, |proof| proof.key.generation)
                .ok()
                .and_then(|index| pool.proofs.get(index))
                .filter(|proof| proof.key.matches_cut(cut))
                .filter(|proof| {
                    proof
                        .dependency
                        .as_ref()
                        .is_some_and(|certificate| certificate.matches_state(&proof.key.state))
                })
        });
        if let Some(proof) = found {
            self.hits += 1;
            Some(proof.counts.clone())
        } else {
            self.cold += 1;
            None
        }
    }
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn record(&mut self, cut: &PublishedCut, counts: &Counts) -> Result<(), String> {
        self.record_admitted(cut, counts, None)
    }
    pub(super) fn record_admitted(
        &mut self,
        cut: &PublishedCut,
        counts: &Counts,
        work: Option<&WorkScope>,
    ) -> Result<(), String> {
        let key = Key::of_admitted(cut, work)?;
        NativeAdmission(work)
            .native_vec_room(&mut self.proofs, 1)
            .map_err(|e| e.to_string())?;
        self.proofs.push(Proof {
            key,
            counts: counts.clone(),
            dependency: None,
        });
        Ok(())
    }
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn complete(
        self,
        tx: &Transaction<'_>,
        capture: sc::AuditCapture,
        nodes: usize,
        frontier: i64,
        cap: usize,
    ) -> Result<(Pool, usize, usize), String> {
        self.complete_admitted(tx, capture, nodes, frontier, cap, None)
    }
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn complete_admitted(
        self,
        tx: &Transaction<'_>,
        capture: sc::AuditCapture,
        nodes: usize,
        frontier: i64,
        cap: usize,
        work: Option<&WorkScope>,
    ) -> Result<(Pool, usize, usize), String> {
        let _ = nodes; // 全capture计数不得变成稀疏原件计数。
        Ok((
            Pool::build_bounded_counts_admitted(
                capture,
                Binding::read_admitted(tx, frontier, work)?,
                self.proofs,
                cap,
                work,
            )?,
            self.hits,
            self.cold,
        ))
    }
    #[cfg_attr(
        all(not(test), feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
        )
    )]
    pub(super) fn seal(self, tx: &Transaction<'_>, frontier: i64) -> Result<PendingPool, String> {
        self.seal_admitted(tx, frontier, None)
    }
    pub(super) fn seal_admitted(
        self,
        tx: &Transaction<'_>,
        frontier: i64,
        work: Option<&WorkScope>,
    ) -> Result<PendingPool, String> {
        Ok(PendingPool {
            binding: Binding::read_admitted(tx, frontier, work)?,
            proofs: self.proofs,
            hits: self.hits,
            cold: self.cold,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{prepared::tests::Fixture, state};
    use super::*;

    fn publish(f: &mut Fixture, n: i64, row: (&str, i64, i64, i64, i64, i64), revision: i64) {
        let (envelope, token) = f.accept_begin(n, row, revision);
        f.publish(&envelope, &token).unwrap();
    }
    fn complete(state: &state::AuditedState) -> serde_json::Value {
        let cuts = state
            .cuts
            .iter()
            .map(|cut| {
                format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    cut.metadata, cut.roots, cut.batch_id, cut.snapshot, cut.delta
                )
            })
            .collect::<Vec<_>>();
        serde_json::json!({"raw":state.raw,"cuts":cuts,"objects":format!("{:?}",state.objects),"auxiliary":format!("{:?}",state.auxiliary)})
    }
    fn audit(f: &mut Fixture, keeper: &mut Keeper) -> Result<serde_json::Value, String> {
        let tx = f.conn.transaction().unwrap();
        let warm = state::audit_native_with_source_keeper_for_testonly(&tx, keeper)?;
        let cold = state::audit_native_current(&tx)?;
        let value = complete(&warm);
        assert_eq!(value, complete(&cold), "完整raw/cuts/index与cold不同");
        Ok(value)
    }
    fn seeded() -> (Fixture, Keeper) {
        let mut f = Fixture::new();
        publish(&mut f, 1, ("a", 0, 6, 12, 3, 7), 1);
        let mut keeper = Keeper::new(usize::MAX);
        audit(&mut f, &mut keeper).unwrap();
        (f, keeper)
    }

    #[test]
    fn r11_source_binding_new_and_seal_use_shared_scope() {
        use super::super::audit_owner::native::Budget;
        let (mut fixture, keeper) = seeded();
        let prior = keeper.pool.as_ref().unwrap();
        let tx = fixture.conn.transaction().unwrap();
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        let before = budget.live();
        let mut native_retained = 0;
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let scope_before = budget.live();
                let trial = Trial::new_admitted(&tx, Some(prior), true, Some(work))?;
                assert!(trial.bound);
                let after_new = budget.live();
                assert!(
                    after_new > scope_before,
                    "新trial完整原件绑定须消费父域许可"
                );
                let pending = trial.seal_admitted(&tx, prior.binding.frontier, Some(work))?;
                assert_eq!(pending.binding, prior.binding);
                assert!(budget.live() > after_new, "封存完整原件再次消费同一父域");
                drop(pending);
                native_retained = budget.live() - scope_before;
                Ok::<_, String>(())
            })
            .unwrap();
        assert!(budget.peak() > before);
        assert!(native_retained > 0);
        assert_eq!(
            budget.live(),
            before + native_retained,
            "Native保守费用随父Budget存活"
        );
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
        println!(
            "R11_SOURCE_BINDING {}",
            serde_json::json!({
                "limit_bytes":1024*1024,"peak_bytes":budget.peak(),
                "new_and_seal_same_scope":true,"binding_equal":true,"whole_heap_qualified":false
            })
        );
    }

    #[test]
    fn r14_source_pool_normal_growth_and_sparse_cap_match_none() {
        use super::super::audit_owner::native::Budget;
        use super::super::request_audit::RequestAuditContext;
        let mut fixture = Fixture::new();
        // 原Fixture仅定义四个clock事件；沿driver正常with_clock夹具的初始化方式，
        // 在任何输入之前为六组历史建立新库，不修改已建域的clock绑定。
        let mut clock = super::super::v2::stored_clock(&fixture.conn).unwrap().value;
        for n in 5..=6 {
            clock["events"][format!("input-{n}")] = serde_json::json!({
                "accept_ns":(n*10).to_string(),
                "attempts":[{"begin_ns":(n*10+1).to_string(),"commit_ns":(n*10+2).to_string()}]
            });
        }
        let clock_path = fixture.db_path().with_file_name("clock-six.json");
        std::fs::write(&clock_path, crate::canonical_json(&clock)).unwrap();
        let db = fixture.db_path().with_file_name("source-pool-six.sqlite");
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        super::super::init_v3(
            &db,
            "s-session-testonly-001",
            &base.join("catalog/signed-catalog.json"),
            &base.join("profiles/ohlc_integer_tb02a_v1.json"),
            "1",
            &clock_path,
            6,
        )
        .unwrap();
        fixture.conn = crate::open_db(&db).unwrap();
        for (index, identity) in ["a", "b", "c", "d", "e", "f"].into_iter().enumerate() {
            let n = index as i64 + 1;
            publish(
                &mut fixture,
                n,
                (identity, (n - 1) * 10, 6 + n, 12 + n, 3 + n, 7 + n),
                1,
            );
        }
        let tx = fixture.conn.transaction().unwrap();
        let mut whole_keeper = Keeper::new(usize::MAX);
        let baseline =
            state::audit_native_with_source_keeper_for_testonly(&tx, &mut whole_keeper).unwrap();
        let whole = whole_keeper.pool.as_ref().unwrap();
        assert_eq!(whole.proofs.len(), 6);
        let first_group_cap = whole
            .subset_selection
            .as_ref()
            .unwrap()
            .first_admitted_source_bytes;
        for cap in [usize::MAX, first_group_cap] {
            let prepare = || {
                let mut context = RequestAuditContext::with_source_keeper(Keeper::new(usize::MAX));
                let audited = state::audit_native_in_request(&tx, &mut context).unwrap();
                assert_eq!(complete(&audited), complete(&baseline));
                context.take().unwrap()
            };
            let oracle = prepare();
            let pending = oracle.source_pending.unwrap();
            let oracle =
                Pool::build_bounded_counts(oracle.capture, pending.binding, pending.proofs, cap)
                    .unwrap();
            let actual = prepare();
            let pending = actual.source_pending.unwrap();
            let pointers = pending
                .proofs
                .iter()
                .map(|proof| {
                    let digest = proof.key.state.digest().unwrap();
                    (
                        digest,
                        actual.capture.original(&digest).unwrap().1.as_ptr() as usize,
                    )
                })
                .collect::<Vec<_>>();
            let budget = Budget::new(32 * 1024 * 1024, 0).unwrap();
            let before = budget.live();
            let mut layout_peak = 0;
            let actual = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let pool = Pool::build_bounded_counts_admitted(
                        actual.capture,
                        pending.binding,
                        pending.proofs,
                        cap,
                        Some(work),
                    )?;
                    let snapshot = work.snapshot();
                    layout_peak = snapshot.peak;
                    assert!(
                        snapshot.attempts > 0 && snapshot.peak > 0,
                        "导航与完整三闭包须使用同scope真实Layout"
                    );
                    assert_eq!(snapshot.live, 0, "封存后不带走codec临时工作集");
                    assert!(budget.live() > before, "封存Plain容器许可须进入同父账");
                    Ok::<_, String>(pool)
                })
                .unwrap();
            assert_eq!(actual.binding, oracle.binding);
            assert_eq!(actual.nodes, oracle.nodes);
            assert!(actual
                .originals
                .exact_original_tuples_equal(&oracle.originals)
                .unwrap());
            assert_eq!(
                serde_json::to_value(&actual.source_cost).unwrap(),
                serde_json::to_value(&oracle.source_cost).unwrap()
            );
            assert_eq!(
                serde_json::to_value(&actual.subset_selection).unwrap(),
                serde_json::to_value(&oracle.subset_selection).unwrap()
            );
            assert_eq!(actual.proofs.len(), oracle.proofs.len());
            assert_eq!(actual.proofs.len(), if cap == usize::MAX { 6 } else { 1 });
            for (actual, expected) in actual.proofs.iter().zip(&oracle.proofs) {
                assert_eq!(actual.key, expected.key);
                assert_eq!(actual.counts, expected.counts);
                let cert = actual.dependency.as_ref().unwrap();
                assert!(cert.matches_state(&actual.key.state));
                assert_eq!(
                    cert.source_bytes().unwrap(),
                    expected
                        .dependency
                        .as_ref()
                        .unwrap()
                        .source_bytes()
                        .unwrap()
                );
            }
            for (digest, pointer) in pointers {
                if let Some((_, body)) = actual.originals.original(&digest) {
                    assert_eq!(body.as_ptr() as usize, pointer, "稀疏owner只移动原件正文");
                }
            }
            let actual_retention = actual
                .originals
                .native_retention_numbers_for_testonly()
                .unwrap();
            let expected_retention = oracle
                .originals
                .native_retention_numbers_for_testonly()
                .unwrap();
            assert_eq!(
                actual_retention.public_roots,
                expected_retention.public_roots
            );
            assert_eq!(actual_retention.template_roots, 0);
            assert_eq!(budget.scratch_observation().0, 0);
            assert_eq!(budget.scratch_observation().2, 0);
            assert!(budget.peak() > before && budget.peak() <= 32 * 1024 * 1024);
            println!(
                "R14_SOURCE_POOL {}",
                serde_json::json!({
                    "groups":actual.proofs.len(),"limit_bytes":32*1024*1024,
                    "parent_peak_bytes":budget.peak(),"codec_layout_peak_bytes":layout_peak,
                    "source_cap":cap,"source_bytes":actual.source_cost.total,
                    "all_original_tuples_binding_key_counts_equal":true,"whole_heap_qualified":false
                })
            );
        }
    }

    #[test]
    fn r13_source_trial_cold_hit_miss_record_and_seal_use_shared_scope() {
        use super::super::audit_owner::native::Budget;
        let (mut fixture, keeper) = seeded();
        publish(&mut fixture, 2, ("b", 10, 8, 14, 5, 9), 1);
        let tx = fixture.conn.transaction().unwrap();
        // 正常追加后的完整原核提供真实 Key/Counts；准备 oracle 不占待测子域。
        let mut oracle_keeper = Keeper::new(usize::MAX);
        let oracle =
            state::audit_native_with_source_keeper_for_testonly(&tx, &mut oracle_keeper).unwrap();
        let cold = state::audit_native_current(&tx).unwrap();
        assert_eq!(complete(&oracle), complete(&cold));
        let expected_pool = oracle_keeper.pool.as_ref().unwrap();
        assert_eq!(oracle.cuts.len(), 2);
        assert_eq!(expected_pool.proofs.len(), oracle.cuts.len());
        let retained = keeper.pool.as_ref().unwrap();
        assert_eq!(retained.proofs.len(), 1);
        assert!(retained.proofs[0]
            .dependency
            .as_ref()
            .unwrap()
            .matches_state(&retained.proofs[0].key.state));
        for prior in [None, Some(retained)] {
            let expected_hits = usize::from(prior.is_some());
            let expected_cold = oracle.cuts.len() - expected_hits;
            let mut plain = Trial::new(&tx, prior, true).unwrap();
            for (index, (cut, proof)) in oracle.cuts.iter().zip(&expected_pool.proofs).enumerate() {
                assert_eq!(proof.key, Key::of(cut));
                let hit = plain.lookup(cut);
                if index < expected_hits {
                    assert_eq!(hit.as_ref(), Some(&proof.counts));
                } else {
                    assert!(hit.is_none());
                }
                plain.record(cut, &proof.counts).unwrap();
            }
            let plain = plain.seal(&tx, expected_pool.binding.frontier).unwrap();
            assert_eq!(plain.binding, expected_pool.binding);
            assert_eq!((plain.hits, plain.cold), (expected_hits, expected_cold));

            let budget = Budget::new(2 * 1024 * 1024, 0).unwrap();
            let before = budget.live();
            let mut native_retained = 0;
            let pending = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let scope_before = budget.live();
                    let mut trial = Trial::new_admitted(&tx, prior, true, Some(work))?;
                    assert_eq!(trial.bound, prior.is_some());
                    if prior.is_some() {
                        assert!(budget.live() > scope_before, "完整绑定须消费同父域");
                    } else {
                        assert_eq!(budget.live(), scope_before, "cold 无 prior 绑定分配");
                    }
                    for (index, (cut, expected)) in
                        oracle.cuts.iter().zip(&expected_pool.proofs).enumerate()
                    {
                        let before_lookup = budget.live();
                        let before_work = work.snapshot();
                        let hit = trial.lookup(cut);
                        if index < expected_hits {
                            assert_eq!(hit.as_ref(), Some(&expected.counts));
                        } else {
                            assert!(hit.is_none());
                        }
                        assert_eq!(budget.live(), before_lookup, "借用 Key 查询不复制文本");
                        assert_eq!(work.snapshot().attempts, before_work.attempts);
                        assert_eq!(work.snapshot().live, before_work.live);

                        let old_capacity = trial.proofs.capacity();
                        trial.record_admitted(cut, &expected.counts, Some(work))?;
                        let proof = trial.proofs.last().unwrap();
                        assert_eq!(proof.key, expected.key);
                        assert_eq!(proof.counts, expected.counts);
                        assert!(proof.dependency.is_none());
                        let text_bytes = proof.key.state.format.len()
                            + proof.key.state.root.len()
                            + proof.key.profile.len()
                            + proof.key.rule.len();
                        let proof_bytes = if trial.proofs.capacity() > old_capacity {
                            trial.proofs.capacity() * std::mem::size_of::<Proof>()
                        } else {
                            0
                        };
                        assert!(
                            budget.live() - before_lookup >= text_bytes + proof_bytes,
                            "Key 全部 Plain 文本及 Proof Vec 新块须先计入同父域"
                        );
                    }
                    assert_eq!((trial.hits, trial.cold), (expected_hits, expected_cold));
                    let before_seal = budget.live();
                    let pending =
                        trial.seal_admitted(&tx, expected_pool.binding.frontier, Some(work))?;
                    assert!(budget.live() > before_seal, "seal 完整绑定消费同父域");
                    native_retained = budget.live() - scope_before;
                    Ok::<_, String>(pending)
                })
                .unwrap();
            assert_eq!(pending.binding, plain.binding);
            assert_eq!((pending.hits, pending.cold), (plain.hits, plain.cold));
            assert_eq!(pending.proofs.len(), plain.proofs.len());
            for (actual, expected) in pending.proofs.iter().zip(&plain.proofs) {
                assert_eq!(actual.key, expected.key);
                assert_eq!(actual.counts, expected.counts);
                assert!(actual.dependency.is_none());
                assert!(expected.dependency.is_none());
            }
            assert!(native_retained > 0);
            assert_eq!(budget.live(), before + native_retained);
            assert_eq!(budget.scratch_observation().0, 0);
            assert_eq!(budget.scratch_observation().2, 0);
            drop(pending);
            assert_eq!(
                budget.live(),
                before + native_retained,
                "保守费用保留在父账"
            );
            println!(
                "R13_SOURCE_TRIAL {}",
                serde_json::json!({
                    "limit_bytes":2*1024*1024,"peak_bytes":budget.peak(),
                    "native_retained_bytes":native_retained,"hits":expected_hits,"cold":expected_cold,
                    "binding_key_counts_equal":true,"whole_heap_qualified":false
                })
            );
        }
    }

    #[test]
    fn counts_subset_normal_multicut_bounded_groups_match_complete_cold() {
        let mut f = Fixture::new();
        for n in 1..=3 {
            let start = (n - 1) * 10;
            let identity = match n {
                1 => "a",
                2 => "b",
                _ => "c",
            };
            publish(&mut f, n, (identity, start, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        }
        let mut measured = Keeper::new(usize::MAX);
        audit(&mut f, &mut measured).unwrap();
        let full = measured.pool.as_ref().unwrap();
        assert_eq!(full.proofs.len(), 3);
        let selection = full.subset_selection.as_ref().unwrap();
        let one_group_cap = selection.first_admitted_source_bytes;
        assert!(
            one_group_cap < full.source_cost.total,
            "正常多个cut须具有正边际原件/Key费用"
        );
        assert!(full.proofs.iter().all(|p| p
            .dependency
            .as_ref()
            .unwrap()
            .matches_state(&p.key.state)));
        for pair in full.proofs.windows(2) {
            assert_ne!(pair[0].key.state, pair[1].key.state);
            assert!(!pair[0]
                .dependency
                .as_ref()
                .unwrap()
                .matches_state(&pair[1].key.state));
            assert!(!pair[1]
                .dependency
                .as_ref()
                .unwrap()
                .matches_state(&pair[0].key.state));
        }
        let mut bounded = Keeper::new(one_group_cap);
        let baseline = audit(&mut f, &mut bounded).unwrap();
        let pool = bounded.pool.as_ref().expect("整组接纳不应被后续超额组驱逐");
        assert_eq!(pool.proofs.len(), 1);
        assert_eq!(pool.proofs[0].key.generation, 3);
        let selection = pool.subset_selection.as_ref().unwrap();
        assert!(selection.budget_stopped);
        assert_eq!(selection.attempted_groups, 2);
        assert_eq!(selection.accepted_groups, 1);
        assert!(selection.rolled_back_nodes > 0);
        assert!(pool.source_cost.total <= one_group_cap);
        assert!(pool.source_cost.counts_dependency_certificate_bytes > 0);
        let public_roots = pool.originals.retention()["native_codec_root_certificates"]
            ["public_roots"]
            .as_u64()
            .unwrap() as usize;
        assert!((1..=3).contains(&public_roots));
        assert_eq!(
            pool.source_cost.codec_root_certificate_bytes,
            43 + 32 * public_roots
        );
        assert_eq!(pool.source_cost.counts_dependency_certificate_bytes, 428);
        assert_eq!(pool.source_cost.total, selection.retained_source_bytes);
        assert_eq!(
            pool.originals.retention()["native_codec_root_certificates"]["template_roots"],
            0
        );
        let (hits, cold) = (bounded.hits, bounded.cold);
        assert_eq!(audit(&mut f, &mut bounded).unwrap(), baseline);
        assert_eq!((bounded.hits - hits, bounded.cold - cold), (1, 2));
        // 合法append仍调用当前完整原source核，先前未接纳cut不具有资格。
        publish(&mut f, 4, ("d", 30, 10, 16, 7, 11), 1);
        audit(&mut f, &mut bounded).unwrap();
        assert!(bounded.retained_source_bytes() <= one_group_cap);
        bounded.clear();
        let mut restarted = Keeper::new(one_group_cap);
        audit(&mut f, &mut restarted).unwrap();
        assert_eq!((restarted.hits, restarted.cold), (0, 4));
    }

    #[test]
    fn counts_subset_normal_unrelated_legal_append_keeps_complete_counts() {
        let (mut f, mut keeper) = seeded();
        let baseline = audit(&mut f, &mut keeper).unwrap();
        let (hits, cold) = (keeper.hits, keeper.cold);
        // 合法完整F原件通过原写入口追加，但不在任何Counts依赖三子图中。
        let digest = {
            let tx = f.conn.transaction().unwrap();
            let value = sc::write_value(
                &mut super::super::content_store::TxStore::new(&tx).unwrap(),
                &sc::Value::String("unrelated-normal-original".repeat(400)),
            )
            .unwrap();
            tx.commit().unwrap();
            value.digest
        };
        assert_eq!(audit(&mut f, &mut keeper).unwrap(), baseline);
        assert_eq!((keeper.hits - hits, keeper.cold - cold), (1, 0));
        let pool = keeper.pool.as_ref().unwrap();
        assert!(pool.originals.original(&digest).is_none());
        assert_eq!(pool.proofs.len(), 1);
        assert!(pool.proofs[0]
            .dependency
            .as_ref()
            .unwrap()
            .matches_state(&pool.proofs[0].key.state));
        assert!(pool.source_cost.total <= keeper.retention_cap());
    }

    #[test]
    fn counts_subset_normal_owner_releases_unselected_originals_and_keeps_exact_tuples() {
        let mut f = Fixture::new();
        for n in 1..=3 {
            let identity = match n {
                1 => "a",
                2 => "b",
                _ => "c",
            };
            publish(
                &mut f,
                n,
                (identity, (n - 1) * 10, 6 + n, 12 + n, 3 + n, 7 + n),
                1,
            );
        }
        let tx = f.conn.transaction().unwrap();
        let mut context = super::super::request_audit::RequestAuditContext::with_source_keeper(
            Keeper::new(usize::MAX),
        );
        let warm = state::audit_native_in_request(&tx, &mut context).unwrap();
        let cold = state::audit_native_current(&tx).unwrap();
        assert_eq!(complete(&warm), complete(&cold));
        let slot = context.take().unwrap();
        let pending = slot.source_pending.unwrap();
        let all_nodes = slot.node_count;
        let all_body = slot.capture.retention()["body_len_bytes"].as_u64().unwrap();
        // 对原导航+三完整子图独立展开，只取正常原件的直接只读见证。
        let mut expected = std::collections::HashMap::new();
        struct ExactRecorder<'a> {
            capture: &'a sc::AuditCapture,
            tuples: &'a mut std::collections::HashMap<sc::NodeDigest, (u8, Vec<u8>, usize)>,
        }
        impl sc::NodeSource for ExactRecorder<'_> {
            fn read(&mut self, d: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                let got = sc::NodeSource::read(&mut self.capture.reader(), d)?;
                if let sc::NodeRead::Found { tag, body } = &got {
                    self.tuples.entry(*d).or_insert_with(|| {
                        (
                            *tag,
                            body.clone(),
                            self.capture.original(d).unwrap().1.as_ptr() as usize,
                        )
                    });
                }
                Ok(got)
            }
        }
        for proof in &pending.proofs {
            let mut recorder = ExactRecorder {
                capture: &slot.capture,
                tuples: &mut expected,
            };
            let fields = sc::path::lookup_fields_refview(
                &mut recorder,
                sc::RefView::from(&proof.key.state),
                &["objects", "witnesses", "observations"],
                true,
            )
            .unwrap();
            sc::audit_verify_many_refview(
                &mut recorder,
                ["objects", "witnesses", "observations"]
                    .iter()
                    .map(|name| sc::RefView::from(fields.get(name).unwrap())),
                sc::Mode::Public,
            )
            .unwrap();
        }
        let excluded = warm
            .cuts
            .iter()
            .map(|cut| cut.roots.publication_template.view())
            .map(|r| r.digest().unwrap())
            .find(|d| !expected.contains_key(d));
        let pool =
            Pool::build_bounded_counts(slot.capture, pending.binding, pending.proofs, usize::MAX)
                .unwrap();
        assert_eq!(pool.nodes, expected.len());
        assert!(pool.nodes < all_nodes);
        assert!(pool.source_cost.node_body_bytes < all_body as usize);
        for (digest, (tag, bytes, pointer)) in expected {
            let kept = pool.originals.original(&digest).unwrap();
            assert_eq!((kept.0, kept.1), (tag, bytes.as_slice()));
            assert_eq!(kept.1.as_ptr() as usize, pointer, "正文必须原Vec按值移动");
        }
        let digest = excluded.expect("正常模板根应不在Counts三子图闭包中");
        assert!(pool.originals.original(&digest).is_none());
        let retained = pool.originals.retention();
        assert_eq!(retained["captured_nodes"], pool.nodes);
        assert_eq!(
            retained["native_frame_reuse"]["retained_body_vec_capacity_bytes"],
            0
        );
        assert_eq!(
            pool.source_cost.node_digest_tag_certificate_bytes,
            97 * pool.nodes
        );
        let public_roots = retained["native_codec_root_certificates"]["public_roots"]
            .as_u64()
            .unwrap() as usize;
        assert!(public_roots > 0 && public_roots <= 3 * pool.proofs.len());
        assert_eq!(
            retained["native_codec_root_certificates"]["template_roots"],
            0
        );
        assert_eq!(
            pool.source_cost.codec_root_certificate_bytes,
            43 + 32 * public_roots
        );
        assert_eq!(
            pool.source_cost.counts_dependency_certificate_bytes,
            428 * pool.proofs.len()
        );
        assert_eq!(
            pool.capacity.codec_root_set_capacity_payload_bytes,
            retained["native_codec_root_certificates"]["public_capacity"]
                .as_u64()
                .unwrap() as usize
                * std::mem::size_of::<sc::NodeDigest>()
        );
        assert_eq!(
            pool.source_cost.total,
            pool.subset_selection
                .as_ref()
                .unwrap()
                .retained_source_bytes
        );
    }

    #[test]
    fn codec_root_success_handler_legal_growth_matches_complete_cold() {
        let mut f = Fixture::new();
        publish(&mut f, 1, ("a", 0, 6, 12, 3, 7), 1);
        let mut warm_context = super::super::request_audit::RequestAuditContext::default();
        {
            let tx = f.conn.transaction().unwrap();
            state::audit_native_in_request(&tx, &mut warm_context).unwrap();
        }
        publish(&mut f, 2, ("b", 10, 8, 16, 6, 12), 1);
        let mut cold_context = super::super::request_audit::RequestAuditContext::default();
        {
            let tx = f.conn.transaction().unwrap();
            let warm = state::audit_native_in_request(&tx, &mut warm_context).unwrap();
            let cold = state::audit_native_in_request(&tx, &mut cold_context).unwrap();
            assert_eq!(complete(&warm), complete(&cold));
        }
        assert_eq!((warm_context.cold, warm_context.hits), (2, 0));
        let warm = warm_context.take().unwrap().capture.retention();
        let cold = cold_context.take().unwrap().capture.retention();
        assert!(warm["capture_reads"].as_u64().unwrap() < cold["capture_reads"].as_u64().unwrap());
        assert_eq!(
            warm["native_codec_root_certificates"]["public_roots"],
            cold["native_codec_root_certificates"]["public_roots"]
        );
        assert_eq!(
            warm["native_codec_root_certificates"]["template_roots"],
            cold["native_codec_root_certificates"]["template_roots"]
        );
        eprintln!("codec_root_success handler_growth warm={warm} cold={cold}");
    }

    #[test]
    fn cross_request_source_cache_real_append_matches_complete_cold() {
        let (mut f, mut keeper) = seeded();
        assert_eq!((keeper.cold, keeper.hits), (1, 0));
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (1, 1));
        publish(&mut f, 2, ("b", 10, 8, 16, 6, 12), 1);
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (2, 2));
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (2, 4));
        let report = keeper.report();
        assert!(report["retained_source_cost"].as_u64().unwrap() > 0);
        assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
        let retention = &report["retained_capture"];
        for mode in ["public", "template"] {
            assert!(retention[mode]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v.as_u64() == Some(0)));
        }
        assert_eq!(retention["native_frame_reuse"]["retained_entries"], 0);
        assert_eq!(
            retention["native_frame_reuse"]["retained_body_vec_capacity_bytes"],
            0
        );
        keeper.clear();
        assert_eq!(keeper.report()["retained_source_cost"], 0);
        let mut restart = Keeper::new(usize::MAX);
        audit(&mut f, &mut restart).unwrap();
        assert_eq!((restart.cold, restart.hits), (2, 0));
    }

    #[test]
    fn cross_request_source_cache_later_revision_and_withdrawal_still_use_current_index() {
        let (mut f, mut keeper) = seeded();
        publish(&mut f, 2, ("b", 10, 8, 16, 6, 12), 1);
        audit(&mut f, &mut keeper).unwrap();
        publish(&mut f, 3, ("a", 0, 7, 13, 4, 8), 2);
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (3, 3));
        let deaths: i64 = f
            .conn
            .query_row(
                "SELECT count(*) FROM s_content_objects WHERE withdrawn_generation IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(deaths > 0, "真实修订夹具必须产生撤回");
        f.conn.execute("UPDATE s_content_objects SET withdrawal_reason='forged' WHERE withdrawn_generation IS NOT NULL", []).unwrap();
        let tx = f.conn.transaction().unwrap();
        assert!(state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper).is_err());
        assert!(keeper.pool.is_none());
    }

    #[test]
    fn cross_request_source_cache_measured_cap_exact_minus_one_and_one() {
        let (mut f, measured) = seeded();
        let required = measured.last_required;
        assert!(required > 1);
        drop(measured);
        for cap in [required, required - 1, 1] {
            let mut keeper = Keeper::new(cap);
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!(keeper.last_required, required);
            assert_eq!(keeper.pool.is_some(), cap == required);
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!(keeper.hits, usize::from(cap == required));
            assert!(keeper.report()["retained_source_cost"].as_u64().unwrap() <= cap as u64);
        }
    }

    #[test]
    fn cross_request_source_cache_complete_key_and_exact_rows_never_hash_only() {
        for mutation in 0..8 {
            let (mut f, mut keeper) = seeded();
            let key = &mut keeper.pool.as_mut().unwrap().proofs[0].key;
            match mutation {
                0 => key.generation += 1,
                1 => key.frontier += 1,
                2 => key.state.root.push('0'),
                3 => key.state.format.push('0'),
                4 => key.state.kind = sc::Kind::Array,
                5 => key.state.extent = Some(999),
                6 => key.profile.push('0'),
                _ => key.rule.push('0'),
            }
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!((keeper.cold, keeper.hits), (2, 0), "mutation={mutation}");
        }
        for sql in [
            "UPDATE raw_events SET rowid=rowid+100000",
            "UPDATE raw_ohlc SET rowid=rowid+100000",
            "INSERT INTO meta VALUES('unrecognized_source_context','x')",
        ] {
            let (mut f, mut keeper) = seeded();
            f.conn.execute(sql, []).unwrap();
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!((keeper.cold, keeper.hits), (2, 0), "{sql}");
        }
        assert_ne!(SqlCell::Text(b"1".to_vec()), SqlCell::Blob(b"1".to_vec()));
        assert_ne!(SqlCell::Text(b"1".to_vec()), SqlCell::Integer(1));
        assert_ne!(SqlCell::Null, SqlCell::Text(Vec::new()));
    }

    #[test]
    fn cross_request_source_cache_damaged_current_material_rejects_and_clears() {
        let faults = [
            "UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "UPDATE s_content_nodes SET tag=255 WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "DELETE FROM s_content_nodes WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "UPDATE raw_events SET raw_text='changed' WHERE seq=0",
            "UPDATE raw_events SET raw_text=CAST(raw_text AS BLOB) WHERE seq=0",
            "UPDATE raw_ohlc SET open='777'",
            "UPDATE raw_ohlc SET open=CAST(open AS BLOB)",
            "DELETE FROM s_content_objects WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "UPDATE catalog SET evidence_json='not-json' WHERE catalog_id='CC-005'",
            "UPDATE meta SET value='forged' WHERE key='profile_hash'",
            "UPDATE meta SET value='forged' WHERE key='rule_revision'",
        ];
        for sql in faults {
            let (mut f, mut keeper) = seeded();
            f.conn.pragma_update(None, "foreign_keys", false).unwrap();
            f.conn.execute(sql, []).unwrap();
            let tx = f.conn.transaction().unwrap();
            assert!(
                state::audit_native_current(&tx).is_err(),
                "fixture did not fail: {sql}"
            );
            assert!(
                state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper).is_err(),
                "{sql}"
            );
            assert!(keeper.pool.is_none(), "{sql}");
            assert_eq!(keeper.hits, 0, "失败不admit任何Counts：{sql}");
        }
    }

    #[test]
    fn cross_request_source_cache_tail_and_late_catalog_fault_cannot_be_hidden() {
        for sql in [
            "UPDATE raw_events SET raw_text='bad-tail' WHERE seq=1",
            "UPDATE raw_ohlc SET schema_revision='bad' WHERE rowid=(SELECT max(rowid) FROM raw_ohlc)",
            "INSERT INTO raw_ohlc SELECT 'orphan',revision,schema_revision,open,high,low,close FROM raw_ohlc LIMIT 1",
            "UPDATE catalog SET evidence_json='{}' WHERE catalog_id='CC-005'",
        ] {
            let (mut f,mut keeper)=seeded();
            f.accept_begin(2,("b",10,8,16,6,12),1);
            f.conn.execute(sql,[]).unwrap();
            let tx=f.conn.transaction().unwrap();
            assert!(state::audit_native_current(&tx).is_err(),"{sql}");
            assert!(state::audit_native_with_source_keeper_for_testonly(&tx,&mut keeper).is_err(),"{sql}");
            assert!(keeper.pool.is_none());
            assert_eq!(keeper.hits,0);
        }
    }

    /// #1394：原名 cross_request_source_cache_unrelated_original_change_is_conservative_miss（a60b26586d，
    /// 全原件超集池时代）。92eabc10f2 起池只保留 Counts 依赖三子图内的原件，无关原件不入池
    /// （同提交 counts_subset_normal_unrelated_legal_append_keeps_complete_counts 已锁），
    /// 绑定只逐 tuple 比对池内原件，故池外原件改变不使旧 proof 失效。按现设计改名并锁住：
    /// 无关原件始终不入池、不假称已验证；旧 proof 依完整闭包证书继续命中；warm 与 cold
    /// 完整 raw/cuts/index 逐项相同（audit 内断言）。
    #[test]
    fn cross_request_source_cache_unrelated_original_change_outside_counts_closure_still_hits() {
        let (mut f, mut keeper) = seeded();
        assert_eq!((keeper.cold, keeper.hits), (1, 0));
        let digest = {
            let tx = f.conn.transaction().unwrap();
            let value = sc::write_value(
                &mut super::super::content_store::TxStore::new(&tx).unwrap(),
                &sc::Value::String("unrelated-original".into()),
            )
            .unwrap();
            tx.commit().unwrap();
            value.digest
        };
        audit(&mut f, &mut keeper).unwrap(); // 新增原件允许旧proof命中，但不入稀疏池。
        assert_eq!((keeper.cold, keeper.hits), (1, 1));
        assert!(keeper
            .pool
            .as_ref()
            .unwrap()
            .originals
            .original(&digest)
            .is_none());
        f.conn
            .execute(
                "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
                [&digest[..]],
            )
            .unwrap();
        audit(&mut f, &mut keeper).unwrap(); // 池外原件改变不触及任何已保留原件。
        assert_eq!((keeper.cold, keeper.hits), (1, 2));
        let pool = keeper.pool.as_ref().unwrap();
        assert!(pool.originals.original(&digest).is_none());
        assert_eq!(pool.proofs.len(), 1);
        assert!(pool.proofs[0]
            .dependency
            .as_ref()
            .unwrap()
            .matches_state(&pool.proofs[0].key.state));
    }

    #[test]
    fn cross_request_source_cache_sql_interrupt_never_cold_retries() {
        let (mut f, mut keeper) = seeded();
        let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        {
            let tx = f.conn.transaction().unwrap();
            tx.progress_handler(
                1,
                Some(move || once.swap(false, std::sync::atomic::Ordering::SeqCst)),
            )
            .expect("安装故障注入 progress_handler");
            assert!(state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper).is_err());
        }
        f.conn
            .progress_handler(0, None::<fn() -> bool>)
            .expect("清除 progress_handler");
        assert!(keeper.pool.is_none());
        assert_eq!(keeper.hits, 0);
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (2, 0));
    }

    #[test]
    fn cross_request_source_cache_subset_requires_unique_complete_old_cover() {
        for (mutation,expected) in [
            ("",true),
            ("INSERT INTO s_content_nodes VALUES(zeroblob(32),7,x'010203')",true),
            ("DELETE FROM s_content_nodes WHERE rowid=2",false),
            ("UPDATE s_content_nodes SET body=x'ff' WHERE rowid=1",false),
            ("DELETE FROM s_content_nodes WHERE rowid=2; INSERT INTO s_content_nodes SELECT digest,tag,body FROM s_content_nodes WHERE rowid=1",false),
            ("INSERT INTO s_content_nodes SELECT digest,tag,body FROM s_content_nodes WHERE rowid=1",false),
        ] {
            let mut conn=rusqlite::Connection::open_in_memory().unwrap();
            conn.execute_batch("CREATE TABLE s_protocol_meta(singleton,protocol_revision); INSERT INTO s_protocol_meta VALUES(1,'s-session/3'); CREATE TABLE s_content_meta(singleton INTEGER,content_format TEXT); CREATE TABLE s_content_nodes(digest BLOB,tag INTEGER,body BLOB)").unwrap();
            conn.execute("INSERT INTO s_content_meta VALUES(1,?1)",[sc::CONTENT_FORMAT]).unwrap();
            for digest in [[1u8;32],[2u8;32]] { conn.execute("INSERT INTO s_content_nodes VALUES(?1,7,x'010203')",[&digest[..]]).unwrap(); }
            let capture={let tx=conn.transaction().unwrap(); super::super::content_store::capture_for_request_prototype(&tx).unwrap().0};
            conn.execute_batch(mutation).unwrap();
            let tx=conn.transaction().unwrap();
            let (_,_,same)=super::super::content_store::capture_with_original_subset_for_source_cache(&tx,Some((&capture,2))).unwrap();
            assert_eq!(same,expected,"{mutation}");
        }
        // mismatch后出现错误必须继续读并Err，不把部分匹配当正常cold资格。
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton,protocol_revision); INSERT INTO s_protocol_meta VALUES(1,'s-session/3'); CREATE TABLE s_content_meta(singleton INTEGER,content_format TEXT); CREATE TABLE s_content_nodes(digest,tag,body)").unwrap();
        conn.execute(
            "INSERT INTO s_content_meta VALUES(1,?1)",
            [sc::CONTENT_FORMAT],
        )
        .unwrap();
        for digest in [[1u8; 32], [2u8; 32]] {
            conn.execute(
                "INSERT INTO s_content_nodes VALUES(?1,7,x'010203')",
                [&digest[..]],
            )
            .unwrap();
        }
        let capture = {
            let tx = conn.transaction().unwrap();
            super::super::content_store::capture_for_request_prototype(&tx)
                .unwrap()
                .0
        };
        conn.execute_batch("UPDATE s_content_nodes SET body=x'ff' WHERE rowid=1; UPDATE s_content_nodes SET body='text' WHERE rowid=2").unwrap();
        let tx = conn.transaction().unwrap();
        assert!(
            super::super::content_store::capture_with_original_subset_for_source_cache(
                &tx,
                Some((&capture, 2))
            )
            .is_err()
        );
    }

    /// 只读打开根任务提供的新owned副本；绝不打开历史封存原件。
    #[test]
    #[ignore]
    fn cross_request_source_cache_actual_owned_snapshot_diagnostic() {
        let db = std::env::var("CROSS_REQUEST_SOURCE_CACHE_DB").unwrap();
        let report = std::env::var("CROSS_REQUEST_SOURCE_CACHE_REPORT").unwrap();
        let mut conn =
            rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        conn.pragma_update(None, "query_only", true).unwrap();
        let mut keeper = Keeper::new(usize::MAX);
        let tx = conn.transaction().unwrap();
        let mut phases = Vec::new();
        let mut baseline = None;
        for arm in [
            "fresh_cold_before",
            "keeper_cold",
            "keeper_warm",
            "fresh_cold_after",
        ] {
            let hits = keeper.hits;
            let cold = keeper.cold;
            let capture_before = super::super::content_store::prototype_capture_work_on_thread();
            let start = std::time::Instant::now();
            let audited = if arm.starts_with("fresh") {
                state::audit_native_current(&tx)
            } else {
                state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper)
            }
            .unwrap();
            let audit_elapsed = start.elapsed().as_nanos();
            let value = crate::canonical_json(&complete(&audited));
            if let Some(expected) = &baseline {
                assert_eq!(&value, expected, "{arm}");
            } else {
                baseline = Some(value.clone());
            }
            let after = super::super::content_store::prototype_capture_work_on_thread();
            let cuts = audited.cuts.len();
            let drop_started = std::time::Instant::now();
            drop(audited);
            let drop_elapsed = drop_started.elapsed().as_nanos();
            if arm == "keeper_cold" {
                keeper.cap = keeper.last_required;
            }
            phases.push(serde_json::json!({"arm":arm,"audit_return_elapsed_ns":audit_elapsed.to_string(),"caller_audited_drop_ns":drop_elapsed.to_string(),
                "audit_and_caller_drop_elapsed_ns":(audit_elapsed+drop_elapsed).to_string(),"stage_elapsed_ns":start.elapsed().as_nanos().to_string(),
                "complete_sha256":crate::sha256_hex(value.as_bytes()),"exact_equal":true,
                "source_calls":if arm.starts_with("fresh"){cuts}else{keeper.cold-cold},"source_hits":keeper.hits-hits,
                "capture_builds":after.capture_builds-capture_before.capture_builds,"node_scans":after.node_scans-capture_before.node_scans,
                "decoded_rows":after.decoded_rows-capture_before.decoded_rows,"body_bytes":after.body_bytes-capture_before.body_bytes,"keeper":keeper.report()}));
        }
        assert!(keeper.hits > 0);
        keeper.clear();
        let out = serde_json::json!({"diagnostic_only":true,"formal_S_9011_started":false,"whole_resource_qualified":false,"production_S7_unchanged":state::AUDIT_SOURCE_RETAINED_BYTES==0,
            "phases":phases,"after_clear":keeper.report(),"snapshot":"one current read-only transaction","cap_role":"test measured source cost only; no approved production value"});
        std::fs::write(report, serde_json::to_vec_pretty(&out).unwrap()).unwrap();
    }
}

/// 正常副本的离线来源体积测量；不接入业务、Keeper接纳或旧codec根播种。
#[cfg(all(test, feature = "s_capture_native_testonly", unix))]
mod counts_footprint_tests {
    use super::super::{request_audit::RequestAuditContext, state};
    use super::*;
    use std::collections::HashSet;
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;
    use std::time::Instant;

    const OWNED_DIR: &str = "/Volumes/AgentStorage/issue1394-counts-footprint-owned-20261005-r1";
    const S7: usize = 512 * 1024 * 1024;

    struct Recorder<'a, S> {
        inner: S,
        digests: &'a mut HashSet<sc::NodeDigest>,
    }
    impl<S: sc::NodeSource> sc::NodeSource for Recorder<'_, S> {
        fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            let result = self.inner.read(digest)?;
            if matches!(&result, sc::NodeRead::Found { .. }) {
                self.digests.insert(*digest);
            }
            Ok(result)
        }
        fn checked_read(
            &mut self,
            digest: &sc::NodeDigest,
        ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
            let result = self.inner.checked_read(digest)?;
            if result.is_some() {
                self.digests.insert(*digest);
            }
            Ok(result)
        }
    }

    fn measured(
        pool: &Pool,
        digests: &HashSet<sc::NodeDigest>,
    ) -> Result<serde_json::Value, String> {
        // 97N沿用现Pool::build的实测原配方，避免另写费用常数。
        if pool.nodes == 0 {
            return Err("CountsFootprintEmptyOriginalCapture".into());
        }
        let unit = pool.source_cost.node_digest_tag_certificate_bytes / pool.nodes;
        if mul(unit, pool.nodes)? != pool.source_cost.node_digest_tag_certificate_bytes {
            return Err("CountsFootprintOriginalFeeInvalid".into());
        }
        let body = digests.iter().try_fold(0, |sum, digest| {
            let (_tag, body) = pool.originals.original(digest).ok_or(
                "CountsFootprintScopeError: dependency is not a full capture original; no overlay synthesis",
            )?;
            add(sum, body.len())
        })?;
        let certificate = mul(digests.len(), unit)?;
        Ok(serde_json::json!({
            "unique_nodes":digests.len(),"node_body_bytes":body,
            "node_digest_tag_certificate_bytes":certificate,
            "source_bytes":add(body,certificate)?,
            "diagnostic_digest_set_capacity_payload_bytes":mul(digests.capacity(),std::mem::size_of::<sc::NodeDigest>())?,
            "scope":"full capture original tag/body; shared unique digest union; source fee excludes diagnostic containers and heap overhead"
        }))
    }

    fn existing_owned_db() -> (PathBuf, Vec<u8>) {
        let owner = PathBuf::from(std::env::var("COUNTS_FOOTPRINT_OWNER_DIR").unwrap());
        assert!(owner.is_absolute());
        assert_eq!(owner, PathBuf::from(OWNED_DIR), "仅接受本轮新owner目录");
        assert_eq!(owner.canonicalize().unwrap(), owner, "owner不得经symlink");
        let db = PathBuf::from(std::env::var("COUNTS_FOOTPRINT_DB").unwrap());
        assert!(db.is_absolute());
        assert_eq!(db.canonicalize().unwrap(), db, "DB不得经symlink");
        assert_eq!(db.parent(), Some(owner.as_path()));
        let metadata = std::fs::symlink_metadata(&db).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.nlink(), 1, "DB不得硬链到封存原件");
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", db.to_str().unwrap()));
            if sidecar.exists() {
                assert_eq!(sidecar.canonicalize().unwrap(), sidecar);
                let metadata = std::fs::symlink_metadata(&sidecar).unwrap();
                assert!(metadata.is_file());
                assert_eq!(metadata.nlink(), 1, "sidecar不得硬链到封存原件");
            }
        }
        // 根runner在开库前核stat/hash/来源链；本测试保留其证书摘要，不冒称复核整链。
        let certificate = std::fs::read(owner.join("OWNER-CERTIFICATE.json")).unwrap();
        let _: serde_json::Value = serde_json::from_slice(&certificate).unwrap();
        (db, certificate)
    }

    #[test]
    #[ignore]
    fn counts_footprint_actual_owned_normal_diagnostic() {
        let started = Instant::now();
        let (db, certificate) = existing_owned_db();
        let report = PathBuf::from(std::env::var("COUNTS_FOOTPRINT_REPORT").unwrap());
        assert!(report.is_absolute());
        assert!(!report.exists(), "不得覆盖既有报告或原料");
        assert_ne!(report, db);
        let legacy: usize = std::env::var("COUNTS_FOOTPRINT_LEGACY_BYTES")
            .unwrap()
            .parse()
            .unwrap();
        let measured_legacy = crate::cache::retained_bytes();
        assert_eq!(legacy, measured_legacy, "必须等于当前进程原legacy来源计量");
        let expected_cuts: usize = std::env::var("COUNTS_FOOTPRINT_EXPECTED_CUTS")
            .unwrap()
            .parse()
            .unwrap();
        let remainder = S7.checked_sub(legacy).expect("legacy已超过S7");
        let mut conn =
            rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        conn.pragma_update(None, "query_only", true).unwrap();
        // 不设immutable：本轮合法最新提交仍依赖WAL；只读主DB/WAL，SHM允许读者锁。
        let tx = conn.transaction().unwrap();
        assert_eq!(
            tx.pragma_query_value(None, "query_only", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        // 空Keeper只让既有完整Native原核记录真实Key/Counts/Binding，不接纳/播种旧证明。
        let mut context = RequestAuditContext::with_source_keeper(Keeper::new(S7));
        let audit_started = Instant::now();
        let audited = state::audit_native_in_request(&tx, &mut context).unwrap();
        let audit_ns = audit_started.elapsed().as_nanos().to_string();
        assert_eq!((context.cold, context.hits), (1, 0));
        assert!(audited.budget.is_none());
        assert_eq!(
            audited.cuts.len(),
            expected_cuts,
            "完整审核cut数量必须等于根绑定的正常副本"
        );
        let slot = context.take().expect("正常完整审计必须留下capture");
        let pending = slot.source_pending.expect("原核必须封存真实Counts");
        assert_eq!(pending.hits, 0);
        assert_eq!(pending.cold, audited.cuts.len());
        assert_eq!(pending.proofs.len(), audited.cuts.len());
        assert!(!pending.proofs.is_empty());
        // 原件owner保持活着；费用直接用当前原Pool配方，移出旧AuditMemo后不恢复任何旧memo。
        let pool = Pool::build(
            slot.capture,
            slot.node_count,
            pending.binding,
            pending.proofs,
        )
        .unwrap();
        assert!(pool.originals.native_frame_reuse_active_for_testonly());
        let full_retention = pool.originals.retention();
        assert_eq!(
            full_retention["captured_nodes"].as_u64().unwrap() as usize,
            pool.nodes
        );
        let measure_started = Instant::now();
        let names = ["objects", "witnesses", "observations"];
        let mut roots: [Vec<sc::ContentRef>; 3] = std::array::from_fn(|_| Vec::new());
        let mut navigation = HashSet::new();
        let navigation_started = Instant::now();
        {
            let mut recorder = Recorder {
                inner: pool.originals.reader(),
                digests: &mut navigation,
            };
            for (cut, proof) in audited.cuts.iter().zip(&pool.proofs) {
                assert_eq!(proof.key, Key::of(cut));
                // 现path原核只枚举顶层骨架/完整键及选中值header，不递归verify生成态。
                let fields = sc::path::lookup_fields_refview(
                    &mut recorder,
                    cut.roots.generated_state.view(),
                    &names,
                    true,
                )
                .unwrap();
                for (i, name) in names.iter().enumerate() {
                    roots[i].push(fields.get(name).expect("三字段必须存在").clone());
                }
            }
        }
        let navigation_ns = navigation_started.elapsed().as_nanos().to_string();
        let mut categories = Vec::new();
        let mut union = navigation.clone();
        for (name, refs) in names.iter().zip(&roots) {
            let category_started = Instant::now();
            let mut digests = HashSet::new();
            {
                let mut recorder = Recorder {
                    inner: pool.originals.reader(),
                    digests: &mut digests,
                };
                // 每类别调用各自创建Verifier::new与全新AuditMemo；跨cut在本类别内共享展开。
                sc::audit_verify_many_refview(
                    &mut recorder,
                    refs.iter().map(sc::RefView::from),
                    sc::Mode::Public,
                )
                .unwrap();
            }
            let root_count = refs
                .iter()
                .map(|r| r.digest().unwrap())
                .collect::<HashSet<_>>()
                .len();
            let cost = measured(&pool, &digests).unwrap();
            union.extend(digests);
            categories.push(
                serde_json::json!({"category":name,"root_occurrences":refs.len(),
                "unique_roots":root_count,"fresh_verifier":true,"closure":cost,
                "elapsed_ns":category_started.elapsed().as_nanos().to_string()}),
            );
        }
        let navigation_cost = measured(&pool, &navigation).unwrap();
        let union_cost = measured(&pool, &union).unwrap();
        let union_nodes_bytes =
            usize::try_from(union_cost["source_bytes"].as_u64().unwrap()).unwrap();
        let fixed = pool
            .source_cost
            .total
            .checked_sub(pool.source_cost.node_body_bytes)
            .unwrap()
            .checked_sub(pool.source_cost.node_digest_tag_certificate_bytes)
            .unwrap()
            .checked_sub(pool.source_cost.codec_root_certificate_bytes)
            .unwrap();
        let candidate_material = add(fixed, union_nodes_bytes).unwrap();
        let fits_material = candidate_material <= remainder;
        let exact_rows = pool.binding.rows.iter().map(|r|serde_json::json!({
            "sql":r.sql,"row_count":r.rows.len(),"column_count":r.columns.len(),
            "scope_cells":r.scope.len(),"fee_status":"原Pool配方逐storage-class及完整字节计费；不导出原行正文"
        })).collect::<Vec<_>>();
        let mut out = serde_json::Map::new();
        for part in [
            serde_json::json!({
            "diagnostic_only":true,"normal_complete_audit_success":true,
            "formal_S_9011_started":false,"whole_resource_qualified":false,
            "scope":"one new owned normal read-only transaction; original capture retained; navigation plus three independent fresh-verifier category closures; no per-cut independent proof measurement",
            "database":db,"owner_certificate_sha256":crate::sha256_hex(&certificate),
            "owner_source_identity_hash_validation":"root runner before open; test checks exclusive owner path and preserves certificate hash",
            "sqlite_read_mode":"READ_ONLY + query_only; WAL retained; normal owned SHM reader changes allowed; no immutable/checkpoint",
            "cut_count":audited.cuts.len(),"source_calls":pending.cold,"source_hits":pending.hits,
            "expected_cut_count":expected_cuts,"actual_cut_count":audited.cuts.len(),
            "audited_generation":audited.cuts.last().unwrap().metadata.generation,
            "raw_row_count":audited.raw.view().len(),"full_capture_unique_nodes":pool.nodes,
            }),
            serde_json::json!({
            "full_capture_source_cost":pool.source_cost,"full_capture_retention":full_retention,
            "navigation":navigation_cost,"navigation_elapsed_ns":navigation_ns,
            "navigation_scope":"top-level OBJECT/SEQ/FIELD_PAIR and complete key comparisons, selected value headers; no full generated_state verify",
            "categories":categories,"deduplicated_union":union_cost,
            "sql_binding":exact_rows,"counts_proof_count":pool.proofs.len(),
            "frontier_certificate_bytes":8,"sql_and_keys_fee_status":"actual original Binding/Key/Counts from successful Native audit; original Pool::build accounting unchanged",
            "codec_root_certificate_bytes_separate":pool.source_cost.codec_root_certificate_bytes,
            "codec_root_scope":"current full capture certificates only; not included in Counts material candidate; no cross-request seeding",
            "candidate_material_source_bytes_without_codec_roots":candidate_material,
            "persistent_dependency_metadata_bytes":null,
            "dependency_metadata_status":"not implemented/qualified; diagnostic digest set capacity is disclosed separately and cannot stand for candidate retained metadata",
            }),
            serde_json::json!({
            "s7_bytes":S7,"legacy_retained_source_bytes":legacy,"s7_remainder_bytes":remainder,
            "legacy_environment_bytes":legacy,"legacy_measured_bytes":measured_legacy,
            "all_recorded_digests_are_capture_originals":true,"overlay_dependencies":0,
            "original_fee_accounting_success":true,
            "all_measured_materials_fit_remainder":fits_material,
            "material_only_all_proof_count_if_fit":if fits_material {Some(pool.proofs.len())} else {None},
            "admissible_complete_proof_count":null,
            "budget_conclusion":if fits_material {"本副本全部测得来源原料低于原S7余量；依赖元数据及运行资源尚未资格化，不能据此宣布整份proof可接纳或9011可容纳"} else {"本副本全部来源原料已超过原S7余量；本轮无逐cut独立proof闭包，不能猜测可接纳数量"},
            "audit_elapsed_ns":audit_ns,"measurement_elapsed_ns":measure_started.elapsed().as_nanos().to_string(),
            "total_elapsed_ns":started.elapsed().as_nanos().to_string(),
            "elapsed_scope":"offline diagnostic audit and extra traversal; not service request time or complete 9011 estimate",
            "s7_scope":"declared original normal-domain source allowance; measurement issues no budget writes"
            }),
        ] {
            let serde_json::Value::Object(fields) = part else {
                unreachable!();
            };
            out.extend(fields);
        }
        let out = serde_json::Value::Object(out);
        drop(tx); // 只读事务rollback；不调用COMMIT、写表或业务handler。
        use std::io::Write;
        let bytes = serde_json::to_vec_pretty(&out).unwrap();
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&report)
            .unwrap();
        output.write_all(&bytes).unwrap();
        eprintln!(
            "counts_footprint_report={} sha256={}",
            report.display(),
            crate::sha256_hex(&bytes)
        );
    }
}

/// 完整候选不裁剪旧来源证明；原收费项全留，另加容器容量。
impl PendingPool {
    pub(super) fn work_counts(&self) -> (usize, usize) {
        (self.hits, self.cold)
    }
    pub(super) fn full_retention_charge(&self) -> usize {
        use super::audit_retention::{sum, Retained};
        let source = binding_source_bytes(&self.binding)
            .and_then(|base| {
                self.proofs.iter().try_fold(base, |total, proof| {
                    add(
                        add(total, proof_source_bytes(proof)?)?,
                        proof
                            .dependency
                            .as_ref()
                            .map_or(Ok(0), |c| c.source_bytes().map_err(|e| e.to_string()))?,
                    )
                })
            })
            .unwrap_or(usize::MAX);
        let proofs = self.proofs.iter().map(|p| {
            sum([
                p.key.state.format.capacity(),
                p.key.state.root.capacity(),
                p.key.profile.capacity(),
                p.key.rule.capacity(),
                p.dependency
                    .as_ref()
                    .map_or(0, |c| c.string_capacity_bytes()),
            ])
        });
        sum([
            source,
            self.binding.rows.heap_bytes(),
            super::audit_retention::vec_bytes::<Proof>(self.proofs.capacity()),
            sum(proofs),
        ])
    }
}
