//! #1647：严格全来源核之后导出本请求新增 raw 原件；非来源 Δ 准入。
//! 收据只活于请求内，不复用 NativeRawFacts 的赢家表冒充全部修订史。
//! raw 必须来自已校验的独立 read_raw_events 读集；不从生成态raw_events自证。
use super::*;
use sc::closure_delta::{self, EmptyFrontier, Probe};

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
    if token.is_empty() || generation < 1 || raw_from > raw.len() {
        return Err(err("P6来源收据请求或raw区间无效"));
    }
    let mut s = super::super::audit_store::StoreView::generic(store);
    let counts = audit_generated_in_store(
        &mut s, generated, raw, profile, rule, deadline, budget, None, None, None, None, None,
    )?;
    export_raw_receipt(
        &mut s, token, generation, generated, raw, raw_from, profile, rule, deadline, budget,
        counts,
    )
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
    let counts = qualified.consume(material, generated, raw, metadata, None)?;
    let result = export_raw_receipt(
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
    Ok(result)
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
        let (mut nodes, state, raw, rule) = super::super::tests::fixture();
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
                        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
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
