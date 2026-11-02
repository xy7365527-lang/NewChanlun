//! #1647：严格全来源核之后导出本请求新增 raw 原件；非来源 Δ 准入。
//! 收据只活于请求内，不复用 NativeRawFacts 的赢家表冒充全部修订史。
//! raw 必须来自已校验的独立 read_raw_events 读集；不从生成态raw_events自证。
use super::*;
use sc::closure_delta::{self, EmptyFrontier, Probe};
#[path = "source_facts.rs"]
pub(crate) mod source_facts;

pub(crate) struct RawRow {
    pub(crate) seq: i64,
    pub(crate) event_digest: NodeDigest,
    pub(crate) bar_digest: NodeDigest,
}

/// 私有字段与唯一构造入口保证来源核失败时不产生部分收据。
/// raw_from 是调用方已发布前缀长度，不是未发布尾部的历史认证水位 R。
pub(crate) struct RawReceipt {
    token: NativeValue<String>,
    generation: i64,
    rule: NativeValue<String>,
    profile: NativeValue<String>,
    generated: NativeValue<ContentRef>,
    scope: NativeValue<ContentRef>,
    raw_from: usize,
    raw_to: usize,
    deadline: Option<std::time::Instant>,
    counts: Counts,
    rows: sc::CodecVec<RawRow>,
    /// 同次来源核导出的 observation 来源与 36M cover；raw-only 收据保持空导出。
    exports: source_facts::SourceExports,
    business: Option<source_facts::BusinessRows>,
    #[cfg(all(test, feature = "s_incremental"))]
    fee_probe: Option<(
        super::super::audit_owner::native::Budget,
        usize,
        bool,
        usize,
    )>,
}

/// Native 完整生成路径与对拍共用的严格来源核；成功后签发请求内收据，不复用跨代成功。
#[allow(dead_code)]
pub(crate) fn audit_raw_receipt(
    store: &mut dyn NodeStore,
    token: &str,
    generation: i64,
    generated: sc::RefView<'_>,
    raw: RawEventsView<'_>,
    raw_from: usize,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::super::audit_budget::Budget>,
) -> Result<RawReceipt, String> {
    audit_receipt(
        store, token, generation, generated, raw, raw_from, profile, rule, deadline, budget, false,
    )
}

pub(crate) fn audit_source_partitions_receipt(
    store: &mut dyn NodeStore,
    token: &str,
    generation: i64,
    generated: sc::RefView<'_>,
    raw: RawEventsView<'_>,
    raw_from: usize,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::super::audit_budget::Budget>,
) -> Result<RawReceipt, String> {
    audit_receipt(
        store, token, generation, generated, raw, raw_from, profile, rule, deadline, budget, true,
    )
}

fn audit_receipt(
    store: &mut dyn NodeStore,
    token: &str,
    generation: i64,
    generated: sc::RefView<'_>,
    raw: RawEventsView<'_>,
    raw_from: usize,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::super::audit_budget::Budget>,
    export_sources: bool,
) -> Result<RawReceipt, String> {
    if token.is_empty() || generation < 1 || raw_from > raw.len() {
        return Err(err("P6来源收据请求或raw区间无效"));
    }
    let mut s = super::super::audit_store::StoreView::generic(store);
    let mut exports = source_facts::SourceExports::default();
    let counts = audit_generated_in_store(
        &mut s,
        generated,
        raw,
        profile,
        rule,
        deadline,
        budget,
        None,
        None,
        None,
        None,
        None,
        if export_sources {
            Some(&mut exports)
        } else {
            None
        },
    )?;
    exports.finish();
    let mut receipt = export_raw_receipt(
        &mut s, token, generation, generated, raw, raw_from, profile, rule, deadline, budget,
        counts,
    )?;
    receipt.exports = exports;
    Ok(receipt)
}

fn export_raw_receipt(
    s: &mut super::super::audit_store::StoreView<'_, '_>,
    token: &str,
    generation: i64,
    generated: sc::RefView<'_>,
    raw: RawEventsView<'_>,
    raw_from: usize,
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::super::audit_budget::Budget>,
    counts: Counts,
) -> Result<RawReceipt, String> {
    if token.is_empty() || generation < 1 || raw_from > raw.len() {
        return Err(err("P6来源收据请求或raw区间无效"));
    }
    super::super::history::check_deadline(deadline)?;
    let frontier = i64::try_from(raw.len())
        .map_err(err)?
        .checked_sub(1)
        .ok_or_else(|| err("P6原始前沿溢出"))?;
    if number(s, generated, "generation")? != generation
        || number(s, generated, "input_frontier")? != frontier
        || text(s, generated, "profile_id")?.as_str() != profile
        || text(s, generated, "rule_revision")?.as_str() != rule
    {
        return Err(err("P6来源收据与生成态元数据不一致"));
    }
    let scope = field_ref(s, generated, "scope")?;
    let scope = s.native_plain_ref(scope.view()).map_err(err)?;
    let allocation = s.allocation_budget();
    let mut rows = bj::new_work(allocation.as_ref());
    for ordinal in raw_from..raw.len() {
        super::super::history::check_deadline(deadline)?;
        let event = raw.get(ordinal).ok_or_else(|| err("P6原始账区间缺行"))?;
        let seq = bj::field(event, "seq")
            .as_i64()
            .ok_or_else(|| err("P6原始seq非i64"))?;
        if usize::try_from(seq).ok() != Some(ordinal) {
            return Err(err("P6原始seq与独立原件ordinal不连续"));
        }
        let event_digest = super::super::history::hash_json_in_store(s, event, deadline, budget)?;
        let bar_digest = source_pending::raw_bar(s, event, deadline, budget)?;
        s.native_codec_room(&mut rows, 1).map_err(err)?;
        rows.try_push(RawRow {
            seq,
            event_digest,
            bar_digest,
        })
        .map_err(err)?;
    }
    super::super::history::check_deadline(deadline)?;
    Ok(RawReceipt {
        token: s.native_string(token).map_err(err)?,
        generation,
        rule: s.native_string(rule).map_err(err)?,
        profile: s.native_string(profile).map_err(err)?,
        generated: s.native_plain_ref(generated).map_err(err)?,
        scope,
        raw_from,
        raw_to: raw.len(),
        deadline,
        counts,
        rows,
        exports: source_facts::SourceExports::default(),
        business: None,
        #[cfg(all(test, feature = "s_incremental"))]
        fee_probe: None,
    })
}

/// 同次 Native 来源核的结果，不能由 Counts 或调用方布尔值制造。
#[cfg(feature = "s_incremental")]
pub(crate) fn receipt_from_native(
    qualified: super::super::audit_owner::QualifiedSourceCut,
    material: &super::super::audit_owner::MaterialAccess<'_, '_, '_>,
    store: &mut dyn NodeStore,
    token: &str,
    metadata: &crate::content_manifest::ManifestMetadata,
    generated: sc::RefView<'_>,
    raw: &super::super::audit_owner::BoundRawAdvance<'_>,
    raw_from: usize,
) -> Result<RawReceipt, String> {
    let (counts, exports) = qualified.consume(material, generated, raw, metadata, None)?;
    let mut result = export_raw_receipt(
        &mut super::super::audit_store::StoreView::generic(store),
        token,
        metadata.generation,
        generated,
        raw.raw(),
        raw_from,
        &metadata.profile_id,
        &metadata.rule_revision,
        None,
        None,
        counts,
    )?;
    raw.ensure_current()?;
    result.exports = exports;
    Ok(result)
}

impl RawReceipt {
    pub(in crate::v3) fn export_business(
        &mut self,
        store: &mut dyn NodeStore,
        objects: &super::super::object_index::Plan,
        auxiliary: &super::super::aux_index::Plan,
    ) -> Result<(), String> {
        if objects.generation != self.generation
            || auxiliary.generation != self.generation
            || self.business.is_some()
        {
            return Err(err("P6业务分区代号不同或重复导出"));
        }
        #[cfg(all(test, feature = "s_incremental"))]
        let probe = fee_probe::begin(self.generation).map(|parent| {
            let live = parent.live();
            (parent, live)
        });
        let exported = source_facts::export(
            &mut super::super::audit_store::StoreView::generic(store),
            objects,
            auxiliary,
            &self.exports.observation_sources,
        );
        #[cfg(all(test, feature = "s_incremental"))]
        if let Some((parent, before)) = probe {
            let refused = fee_probe::refusal(parent.live());
            if let Some(released) = refused {
                assert!(exported.is_err());
                assert!(released > 0);
            }
            let held = if exported.is_ok() {
                parent
                    .live()
                    .checked_sub(before)
                    .expect("业务行持有同父账费用")
            } else {
                0
            };
            self.fee_probe = Some((parent, held, exported.is_err(), refused.unwrap_or(0)));
        }
        self.business = Some(exported?);
        Ok(())
    }
    pub(crate) fn business_rows(&self) -> Result<&source_facts::BusinessRows, String> {
        self.business
            .as_ref()
            .ok_or_else(|| err("P6缺完整业务分区来源"))
    }
    /// 36M cover 导出；只有带来源导出的收据（source-partitions 或完整台账生产）才持有。
    pub(crate) fn cover(&self) -> &source_facts::SourceExports {
        &self.exports
    }
}

struct DeadlineSource<'a> {
    source: &'a mut dyn NodeStore,
    deadline: Option<std::time::Instant>,
}
impl DeadlineSource<'_> {
    fn check(&self) -> Result<(), sc::CodecError> {
        super::super::history::check_deadline(self.deadline)
            .map_err(|e| sc::CodecError::new("verify_deadline", e))
    }
}
impl sc::NodeSource for DeadlineSource<'_> {
    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source.codec_work()
    }
    fn max_render_bytes(&self) -> Option<u64> {
        self.source.max_render_bytes()
    }
    fn audit_full_array_scan(&self) -> bool {
        self.source.audit_full_array_scan()
    }
    fn preserve_sequence_read_trace(&self) -> bool {
        self.source.preserve_sequence_read_trace()
    }
    fn before_path_index(
        &mut self,
        prefix: &[sc::path::PathStep],
        index: u64,
    ) -> Result<(), sc::CodecError> {
        self.check()?;
        self.source.before_path_index(prefix, index)?;
        self.check()
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        self.read_scoped(digest)?.into_plain()
    }
    fn read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<sc::job::ScopedNodeRead, sc::CodecError> {
        self.check()?;
        let read = self.source.read_scoped(digest)?;
        self.check()?;
        Ok(read)
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        self.checked_read_scoped(digest)?
            .map(sc::job::ScopedFrame::into_frame)
            .transpose()
    }
    fn checked_read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        self.check()?;
        let frame = self.source.checked_read_scoped(digest)?;
        self.check()?;
        Ok(frame)
    }
}

impl sc::NodeSink for DeadlineSource<'_> {
    fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source.codec_sink_work()
    }
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.check()?;
        self.source.put(digest, tag, body)?;
        self.check()
    }
}

fn same_ref(a: sc::RefView<'_>, b: sc::RefView<'_>) -> bool {
    a.format() == b.format()
        && a.root() == b.root()
        && a.kind() == b.kind()
        && a.extent() == b.extent()
}

/// 同请求四根及来源结果绑定；不签发 U-A，不允许历史 frontier 输入。
/// 四根核任一失败，来源收据和已核单位随返回错误一起释放。
pub(crate) struct PreparedReceipt {
    source: RawReceipt,
    roots: [NativeValue<ContentRef>; 4],
    closure: Probe,
}
#[allow(dead_code)]
impl RawReceipt {
    pub(crate) fn with_store<T>(
        &self,
        store: &mut dyn NodeStore,
        f: impl FnOnce(&mut dyn NodeStore) -> Result<T, String>,
    ) -> Result<T, String> {
        super::super::history::check_deadline(self.deadline)?;
        let result = f(&mut DeadlineSource {
            source: store,
            deadline: self.deadline,
        })?;
        super::super::history::check_deadline(self.deadline)?;
        Ok(result)
    }
    pub(crate) fn check_request(
        &self,
        token: &str,
        generation: i64,
        rule: &str,
        raw_range: std::ops::Range<usize>,
        scope: sc::RefView<'_>,
        generated: sc::RefView<'_>,
    ) -> Result<(), String> {
        super::super::history::check_deadline(self.deadline)?;
        if self.token.as_str() != token
            || self.generation != generation
            || self.rule.as_str() != rule
            || self.raw_from != raw_range.start
            || self.raw_to != raw_range.end
            || !same_ref((&self.scope).into(), scope)
            || !same_ref((&self.generated).into(), generated)
        {
            return Err(err("P6来源收据与发布请求不符"));
        }
        Ok(())
    }
    pub(crate) fn check_manifest(
        &self,
        generation: i64,
        profile: &str,
        rule: &str,
        input_frontier: i64,
        scope: sc::RefView<'_>,
        generated: sc::RefView<'_>,
    ) -> Result<(), String> {
        let raw_to = input_frontier
            .checked_add(1)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| err("P6发布前沿无效"))?;
        if self.profile.as_str() != profile {
            return Err(err("P6来源profile与发布不同"));
        }
        self.check_request(
            &self.token,
            generation,
            rule,
            self.raw_from..raw_to,
            scope,
            generated,
        )
    }
    pub(crate) fn check_raw_partition(
        &self,
        token: &str,
        metadata: &crate::content_manifest::ManifestMetadata,
        roots: &impl crate::content_manifest::ManifestRootSource,
    ) -> Result<std::ops::Range<usize>, String> {
        let roots = roots.view();
        #[cfg(all(test, feature = "s_incremental"))]
        let token = if self.business.is_some() {
            fee_probe::binding_token(self.generation, token)
        } else {
            token
        };
        self.check_manifest(
            metadata.generation,
            &metadata.profile_id,
            &metadata.rule_revision,
            metadata.input_frontier,
            (&metadata.scope).into(),
            roots.generated_state,
        )?;
        self.check_request(
            token,
            metadata.generation,
            &metadata.rule_revision,
            self.raw_from..self.raw_to,
            (&metadata.scope).into(),
            roots.generated_state,
        )?;
        Ok(self.raw_from..self.raw_to)
    }
    #[cfg(all(test, feature = "s_incremental"))]
    pub(crate) fn binding_root_for_test(&self) -> Option<usize> {
        self.business.as_ref()?;
        fee_probe::binding_root(self.generation)
    }
    pub(crate) fn request_token(&self) -> &str {
        self.token.as_str()
    }
    pub(crate) fn counts(&self) -> &Counts {
        &self.counts
    }
    pub(crate) fn rows(&self) -> &[RawRow] {
        &self.rows
    }
    pub(crate) fn bind_four_roots(
        self,
        store: &mut dyn NodeStore,
        roots: [sc::RefView<'_>; 4],
    ) -> Result<PreparedReceipt, String> {
        super::super::history::check_deadline(self.deadline)?;
        if !same_ref((&self.scope).into(), roots[0])
            || !same_ref((&self.generated).into(), roots[1])
        {
            return Err(err("P6四根中的scope/generated与已核来源不同"));
        }
        let owned = {
            let s = super::super::audit_store::StoreView::generic(store);
            [
                s.native_plain_ref(roots[0]).map_err(err)?,
                s.native_plain_ref(roots[1]).map_err(err)?,
                s.native_plain_ref(roots[2]).map_err(err)?,
                s.native_plain_ref(roots[3]).map_err(err)?,
            ]
        };
        let mut bounded = DeadlineSource {
            source: store,
            deadline: self.deadline,
        };
        let closure =
            closure_delta::probe_four_roots(&mut bounded, roots, &EmptyFrontier).map_err(err)?;
        super::super::history::check_deadline(self.deadline)?;
        Ok(PreparedReceipt {
            source: self,
            roots: owned,
            closure,
        })
    }
}

#[allow(dead_code)]
impl PreparedReceipt {
    pub(crate) fn for_each_unit(&self, visit: impl FnMut(closure_delta::Unit)) {
        self.closure.for_each_unit(visit);
    }
    /// 消费而非克隆；错请求/代/规则/区间/任一根声明都拒绝，不重新核验兜底。
    pub(crate) fn consume(
        self,
        token: &str,
        generation: i64,
        rule: &str,
        raw_range: std::ops::Range<usize>,
        roots: [sc::RefView<'_>; 4],
    ) -> Result<RawReceipt, String> {
        super::super::history::check_deadline(self.source.deadline)?;
        if self.source.token.as_str() != token
            || self.source.generation != generation
            || self.source.rule.as_str() != rule
            || self.source.raw_from != raw_range.start
            || self.source.raw_to != raw_range.end
            || !self
                .roots
                .iter()
                .zip(roots)
                .all(|(a, b)| same_ref(a.into(), b))
        {
            return Err(err("P6发布收据与本请求、来源区间或四根不一致"));
        }
        super::super::history::check_deadline(self.source.deadline)?;
        Ok(self.source)
    }
}

#[cfg(test)]
thread_local! {
    static PUBLICATION_CONSUMED: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(test)]
pub(crate) fn note_publication_consumed_for_test(rows: usize) {
    PUBLICATION_CONSUMED.with(|seen| seen.borrow_mut().push(rows));
}
#[cfg(test)]
pub(crate) fn take_publication_consumed_for_test() -> Vec<usize> {
    PUBLICATION_CONSUMED.with(|seen| std::mem::take(&mut *seen.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::capture_allocation::scratch::{
        CodecWork, ScratchCharge, ScratchFailure, ScratchParent, WorkScope,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::{Duration, Instant};

    struct Parent(AtomicUsize);
    impl ScratchParent for Parent {
        fn health(&self) -> Result<(), ScratchFailure> {
            Ok(())
        }
        fn invalidate(&self) {
            panic!("unexpected invalidation");
        }
        fn reserve(&self, n: usize, _: ScratchCharge) -> Result<u64, ScratchFailure> {
            self.0.fetch_add(n, Ordering::SeqCst);
            Ok(0)
        }
        fn release(&self, n: usize, _: ScratchCharge, _: u64) -> Result<(), ScratchFailure> {
            assert!(self.0.fetch_sub(n, Ordering::SeqCst) >= n);
            Ok(())
        }
    }
    struct ScopedStore<'a> {
        nodes: &'a mut sc::MemoryNodes,
        work: &'a WorkScope,
        reads: usize,
        expire_at_read: Option<(usize, Instant)>,
    }
    impl sc::NodeSource for ScopedStore<'_> {
        fn codec_work(&self) -> Option<&WorkScope> {
            Some(self.work)
        }
        fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.reads += 1;
            if let Some((at, deadline)) = self.expire_at_read {
                if self.reads == at {
                    std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
                }
            }
            self.nodes.read(digest)
        }
    }
    impl sc::NodeSink for ScopedStore<'_> {
        fn codec_sink_work(&self) -> Option<&WorkScope> {
            Some(self.work)
        }
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
            self.nodes.put(digest, tag, body)
        }
    }

    #[test]
    fn p6_business_source_coordinates_release_native_fees_on_success_and_refusal() {
        let parent = Arc::new(Parent(AtomicUsize::new(0)));
        let mut nodes = sc::MemoryNodes::default();
        CodecWork::new(parent.clone(), 1024 * 1024)
            .unwrap()
            .run(|work| {
                let mut store = ScopedStore {
                    nodes: &mut nodes,
                    work,
                    reads: 0,
                    expire_at_read: None,
                };
                let source = super::super::super::audit_store::StoreView::generic(&mut store);
                let before = parent.0.load(Ordering::SeqCst);
                let digest = source_facts::coordinates_digest(&source, &[0, 2, 9]).unwrap();
                assert_eq!(
                    digest,
                    super::super::super::history::hash_json_metered(
                        &json!(["0", "2", "9"]),
                        None,
                        None
                    )
                    .unwrap()
                );
                assert_eq!(parent.0.load(Ordering::SeqCst), before);
                Ok::<(), sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(parent.0.load(Ordering::SeqCst), 0);
        let refused = CodecWork::new(parent.clone(), 128).unwrap().run(|work| {
            let mut store = ScopedStore {
                nodes: &mut nodes,
                work,
                reads: 0,
                expire_at_read: None,
            };
            let source = super::super::super::audit_store::StoreView::generic(&mut store);
            assert!(source_facts::coordinates_digest(&source, &[0; 4096]).is_err());
            Ok::<(), sc::CodecError>(())
        });
        assert!(matches!(
            refused,
            Err(sc::capture_allocation::scratch::WorkError::Resource(
                sc::capture_allocation::scratch::ScratchFailure::Budget
            ))
        ));
        assert_eq!(parent.0.load(Ordering::SeqCst), 0);
    }

    fn roots(s: &mut sc::MemoryNodes, state: &ContentRef) -> [ContentRef; 4] {
        let scope = field_ref(s, state.into(), "scope").unwrap();
        let v = scope.view();
        let scope = ContentRef {
            format: v.format().into(),
            root: v.root().into(),
            kind: v.kind(),
            extent: v.extent(),
        };
        [scope, state.clone(), state.clone(), state.clone()]
    }

    fn deadline_failure(boundary: usize) {
        let (mut nodes, state, raw, rule) =
            super::super::tests::tb1405_pending_fixture(&[("deadline-bar", 0, 1, 2, 0, 1)]);
        assert_eq!(raw.len(), 1);
        let refs = roots(&mut nodes, &state);
        let parent = Arc::new(Parent(AtomicUsize::new(0)));
        CodecWork::new(parent.clone(), 8 * 1024 * 1024)
            .unwrap()
            .run(|work| {
                let mut store = ScopedStore {
                    nodes: &mut nodes,
                    work,
                    reads: 0,
                    expire_at_read: None,
                };
                let retained_before = parent.0.load(Ordering::SeqCst);
                let deadline = Instant::now() + Duration::from_secs(3);
                let source = audit_raw_receipt(
                    &mut store,
                    "request-a",
                    1,
                    (&state).into(),
                    (&raw).into(),
                    0,
                    tb02a::PROFILE,
                    &rule,
                    Some(deadline),
                    None,
                )
                .unwrap();
                assert_eq!(source.deadline, Some(deadline));
                assert!(parent.0.load(Ordering::SeqCst) > retained_before);
                store.reads = 0;
                let failure = match boundary {
                    0 => {
                        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
                        let error = source
                            .bind_four_roots(&mut store, refs.each_ref().map(Into::into))
                            .err()
                            .expect("expired source receipt must not bind");
                        assert_eq!(store.reads, 0);
                        error
                    }
                    1 => {
                        store.expire_at_read = Some((4, deadline));
                        let error = source
                            .bind_four_roots(&mut store, refs.each_ref().map(Into::into))
                            .err()
                            .expect("expiry during traversal must not issue a receipt");
                        assert_eq!(store.reads, 4);
                        error
                    }
                    2 => {
                        let receipt = source
                            .bind_four_roots(&mut store, refs.each_ref().map(Into::into))
                            .unwrap();
                        assert!(work.snapshot().live > 0);
                        assert_eq!(receipt.source.deadline, Some(deadline));
                        assert!(Instant::now() < deadline);
                        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
                        assert!(Instant::now() >= deadline);
                        receipt
                            .consume(
                                "request-a",
                                1,
                                &rule,
                                0..raw.len(),
                                refs.each_ref().map(Into::into),
                            )
                            .err()
                            .expect("expiry after binding must reject consumption")
                    }
                    _ => unreachable!(),
                };
                assert!(failure.contains("VerifyDeadline"), "{failure}");
                assert_eq!(work.snapshot().live, 0);
                assert_eq!(parent.0.load(Ordering::SeqCst), retained_before);
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(parent.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn p6_raw_receipt_expired_before_binding_releases_budget() {
        deadline_failure(0);
    }
    #[test]
    fn p6_raw_receipt_expired_during_closure_releases_budget() {
        deadline_failure(1);
    }
    #[test]
    fn p6_raw_receipt_expired_before_consumption_releases_budget() {
        deadline_failure(2);
    }

    #[test]
    fn p6_raw_partition_borrows_original_payload_and_releases_pending_parent_fees() {
        use super::super::super::{audit_ledger, prepared};
        let (fixture, _, _, mut metadata, _) = prepared::tests::p6_candidate();
        let db = fixture.db_path();
        let (mut nodes, generated, raw, rule) = super::super::tests::fixture();
        let refs = roots(&mut nodes, &generated);
        metadata.scope = refs[0].clone();
        metadata.generation = 1;
        metadata.input_frontier = raw.len() as i64 - 1;
        metadata.profile_id = tb02a::PROFILE.into();
        metadata.rule_revision = rule.clone();
        let manifest_roots = crate::content_manifest::ManifestRoots {
            generated_state: refs[1].clone(),
            publication_template: refs[2].clone(),
            delta_template: refs[3].clone(),
        };
        for wrong_token in [false, true] {
            let name = if wrong_token {
                "raw-budget-reject.sqlite"
            } else {
                "raw-budget.sqlite"
            };
            let mut ledger = audit_ledger::Ledger::fresh_raw_partition(
                &db.with_file_name(name),
                &db,
                audit_ledger::Identity {
                    rule_revision: rule.clone(),
                    content_identity: "TestOnly-raw-budget".into(),
                    binary_commit: "test-only".into(),
                },
                64 * 1024 * 1024,
            )
            .unwrap();
            let parent = Arc::new(Parent(AtomicUsize::new(0)));
            CodecWork::new(parent.clone(), 8 * 1024 * 1024)
                .unwrap()
                .run(|work| {
                    let mut store = ScopedStore {
                        nodes: &mut nodes,
                        work,
                        reads: 0,
                        expire_at_read: None,
                    };
                    let retained_before = parent.0.load(Ordering::SeqCst);
                    let source = audit_raw_receipt(
                        &mut store,
                        "raw-request",
                        1,
                        (&generated).into(),
                        (&raw).into(),
                        0,
                        tb02a::PROFILE,
                        &rule,
                        None,
                        None,
                    )
                    .unwrap();
                    let payload = source.rows().as_ptr();
                    // NativeReservation直接归父账，WorkSnapshot只计scratch Layout。
                    let live = parent.0.load(Ordering::SeqCst);
                    assert!(live > retained_before);
                    let token = if wrong_token {
                        "wrong-request"
                    } else {
                        "raw-request"
                    };
                    let result = ledger.stage_raw_partition(
                        token,
                        &metadata,
                        &manifest_roots,
                        &source,
                        Some(work),
                    );
                    assert_eq!(source.rows().as_ptr(), payload);
                    if wrong_token {
                        assert!(result.is_err());
                        assert!(ledger.raw_partition_through().is_err());
                    } else {
                        result.unwrap();
                        assert!(parent.0.load(Ordering::SeqCst) > live);
                        let reader = rusqlite::Connection::open(db.with_file_name(name)).unwrap();
                        assert_eq!(
                            reader
                                .query_row("SELECT COUNT(*) FROM raw", [], |r| r.get::<_, i64>(0))
                                .unwrap(),
                            0
                        );
                        ledger.bind(token, &metadata, &manifest_roots).unwrap();
                        ledger.content_committed().unwrap();
                        assert_eq!(ledger.raw_partition_through().unwrap(), 1);
                        assert!(ledger.committed_through().is_err());
                    }
                    assert_eq!(parent.0.load(Ordering::SeqCst), live);
                    drop(source);
                    assert_eq!(parent.0.load(Ordering::SeqCst), retained_before);
                    assert_eq!(work.snapshot().live, 0);
                    Ok::<_, sc::CodecError>(())
                })
                .unwrap();
            assert_eq!(parent.0.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn p6_raw_receipt_preserves_full_event_and_only_requested_suffix() {
        let (mut s, state, raw, rule) = super::super::tests::fixture();
        let from = raw.len() - 1;
        let receipt = audit_raw_receipt(
            &mut s,
            "request-a",
            1,
            (&state).into(),
            (&raw).into(),
            from,
            tb02a::PROFILE,
            &rule,
            None,
            None,
        )
        .unwrap();
        assert_eq!(receipt.rows().len(), 1);
        let row = &receipt.rows()[0];
        assert_eq!(row.seq, from as i64);
        assert_eq!(
            row.event_digest,
            super::super::super::history::hash_json_metered(&raw[from], None, None).unwrap()
        );
        let mut altered = raw[from].clone();
        altered["receipt_id"] = json!("different-original-receipt");
        assert_ne!(
            row.event_digest,
            super::super::super::history::hash_json_metered(&altered, None, None).unwrap()
        );
        assert_eq!(
            row.bar_digest,
            source_pending::raw_bar(&s, (&altered).into(), None, None).unwrap()
        );
        assert!(receipt.counts().objects > 0);
    }

    #[test]
    fn p6_raw_receipt_source_failure_issues_nothing() {
        let (mut s, state, mut raw, rule) = super::super::tests::fixture();
        raw[0]["high"] = json!("99999");
        let full = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap_err();
        let failure = audit_raw_receipt(
            &mut s,
            "request-a",
            1,
            (&state).into(),
            (&raw).into(),
            0,
            tb02a::PROFILE,
            &rule,
            None,
            None,
        )
        .err()
        .unwrap();
        assert_eq!(failure, full);
    }

    #[test]
    fn p6_raw_receipt_rejects_request_and_four_root_substitution() {
        let (mut s, state, raw, rule) = super::super::tests::fixture();
        let refs = roots(&mut s, &state);
        for changed in 0..8 {
            let source = audit_raw_receipt(
                &mut s,
                "request-a",
                1,
                (&state).into(),
                (&raw).into(),
                0,
                tb02a::PROFILE,
                &rule,
                None,
                None,
            )
            .unwrap();
            let receipt = source
                .bind_four_roots(&mut s, refs.each_ref().map(Into::into))
                .unwrap();
            let mut supplied = refs.clone();
            if changed < 4 {
                supplied[changed].extent = Some(u64::MAX);
            }
            let token = if changed == 4 {
                "request-b"
            } else {
                "request-a"
            };
            let generation = if changed == 5 { 2 } else { 1 };
            let revision = if changed == 6 { "other-rule" } else { &rule };
            let start = usize::from(changed == 7);
            assert!(receipt
                .consume(
                    token,
                    generation,
                    revision,
                    start..raw.len(),
                    supplied.each_ref().map(Into::into)
                )
                .is_err());
        }
    }

    #[test]
    fn p6_raw_receipt_fourth_root_failure_issues_no_prepared_receipt() {
        let (mut s, state, raw, rule) = super::super::tests::fixture();
        let source = audit_raw_receipt(
            &mut s,
            "request-a",
            1,
            (&state).into(),
            (&raw).into(),
            0,
            tb02a::PROFILE,
            &rule,
            None,
            None,
        )
        .unwrap();
        let mut refs = roots(&mut s, &state);
        refs[3].root = "not-a-content-root".into();
        assert!(source
            .bind_four_roots(&mut s, refs.each_ref().map(Into::into))
            .is_err());
    }
}

#[cfg(all(test, feature = "s_incremental"))]
pub(crate) mod fee_probe {
    use super::super::super::audit_owner::native::{Budget, Permit};
    #[derive(Default)]
    pub(crate) struct Reading {
        pub generation: i64,
        pub held: usize,
        pub raw_released: usize,
        pub projection_released: usize,
        pub business_released: usize,
        pub refused: bool,
        pub refusal_released: usize,
    }
    #[derive(Default)]
    struct Probe {
        enabled: bool,
        reject: i64,
        reject_binding: i64,
        reject_root: Option<(i64, usize)>,
        current: i64,
        refusal_live: Option<usize>,
        parents: Vec<Budget>,
        rows: Vec<Reading>,
    }
    thread_local! { static PROBE: std::cell::RefCell<Probe> = Default::default(); }
    pub(crate) fn start(reject: i64) {
        PROBE.with(|p| {
            *p.borrow_mut() = Probe {
                enabled: true,
                reject,
                ..Default::default()
            }
        });
    }
    pub(crate) fn reject_binding_at(generation: i64) {
        PROBE.with(|p| p.borrow_mut().reject_binding = generation);
    }
    pub(crate) fn reject_root_at(generation: i64, root: usize) {
        assert!(root < 4);
        PROBE.with(|p| p.borrow_mut().reject_root = Some((generation, root)));
    }
    pub(super) fn binding_root(generation: i64) -> Option<usize> {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if p.enabled && p.reject_root.is_some_and(|(g, _)| g == generation) {
                p.reject_root.take().map(|(_, root)| root)
            } else {
                None
            }
        })
    }
    pub(super) fn binding_token(generation: i64, token: &str) -> &str {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if p.enabled && p.reject_binding == generation {
                p.reject_binding = -1;
                "p6-wrong-request"
            } else {
                token
            }
        })
    }
    pub(crate) fn bind(parent: &Budget) {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if p.enabled {
                p.parents.push(parent.clone());
            }
        });
    }
    pub(super) fn begin(generation: i64) -> Option<Budget> {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if !p.enabled {
                return None;
            }
            p.current = generation;
            p.parents.last().cloned()
        })
    }
    pub(crate) fn block_after_object() -> Option<Permit> {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if !p.enabled || p.current != p.reject {
                return None;
            }
            p.reject = -1;
            let parent = p.parents.last().unwrap().clone();
            p.refusal_live = Some(parent.live());
            Some(
                parent
                    .reserve(parent.remaining())
                    .expect("测试预留同一父账剩余额度"),
            )
        })
    }
    pub(super) fn refusal(after: usize) -> Option<usize> {
        PROBE.with(|p| {
            p.borrow_mut()
                .refusal_live
                .take()
                .map(|before| before.checked_sub(after).expect("预算拒绝不能遗留新增载荷"))
        })
    }
    pub(super) fn record(reading: Reading) {
        eprintln!("P6_SOURCE_FEES generation={} held={} raw_released={} projection_released={} business_released={} refused={} refusal_released={}",
            reading.generation, reading.held, reading.raw_released, reading.projection_released,
            reading.business_released, reading.refused, reading.refusal_released);
        PROBE.with(|p| p.borrow_mut().rows.push(reading));
    }
    pub(crate) fn finish() -> Vec<Reading> {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            for parent in &p.parents {
                let (scratch, _, codec_metadata, _) = parent.scratch_observation();
                eprintln!(
                    "P6_SOURCE_PARENT live={} parent_metadata={} scratch={} codec_metadata={}",
                    parent.live(),
                    Budget::metadata_bytes(),
                    scratch,
                    codec_metadata
                );
                assert_eq!(
                    parent.live(),
                    Budget::metadata_bytes(),
                    "实际owner析构后仅存父账观察句柄元数据"
                );
                assert_eq!(scratch, 0);
                assert_eq!(codec_metadata, 0);
            }
            p.enabled = false;
            p.parents.clear();
            std::mem::take(&mut p.rows)
        })
    }
}
#[cfg(all(test, feature = "s_incremental"))]
impl Drop for RawReceipt {
    fn drop(&mut self) {
        if let Some((parent, held, refused, refusal_released)) = self.fee_probe.take() {
            let before = parent.live();
            drop(std::mem::take(&mut self.rows));
            let raw = parent.live();
            drop(std::mem::take(&mut self.exports));
            let projection = parent.live();
            drop(self.business.take());
            fee_probe::record(fee_probe::Reading {
                generation: self.generation,
                held,
                raw_released: before - raw,
                projection_released: raw - projection,
                business_released: projection - parent.live(),
                refused,
                refusal_released,
            });
        }
    }
}
