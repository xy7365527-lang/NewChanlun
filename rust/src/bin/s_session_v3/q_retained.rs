//! #1394：Q私有缓存的可计量表示。全数据库页和原始账保留为拥有的字节；
//! 类型化审计索引保留原值，不存动态JSON树或跨请求SQLite事务。
//! 计量是实际Vec/String容量与类型大小，不是RSS，也不含查询期间的临时解码对象。
use super::super::business_json::RawEvents;
use super::*;
use newchan_rust::s_content::ContentRef;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecVec};

pub(super) struct Packed {
    bytes: CodecVec<u8>,
    raw: CodecVec<CodecVec<u8>>,
    audited: Option<state::AuditedState>,
    digest: CaptureDigest,
    captured_bytes: u64,
    versions: [i64; 3],
    pub(super) retained_bytes: u64,
    receipt: Option<audit_budget::Receipt>,
    // #1394: keep the same allocation ledger across packing and checkout.
    budget: Option<OwnedBudget>,
    allocation: Option<super::super::q_allocation::Profile>,
    q_source: Option<newchan_rust::s_content::AuditCapture>,
}

/// #1394：仅唯一鲜进程执行，安装真实 SQLite shim 后验证实际 Q 缓存所有权。
/// 不改变生产额度，也不把这里经过的普通 capture 临时容器声称为已完整计量。
#[cfg(all(
    test,
    feature = "s_allocation_json_testonly",
    feature = "s_allocation_sqlite_testonly"
))]
mod isolated_sqlite_owned_cache_tests {
    use super::*;
    use newchan_rust::s_content::capture_allocation::sqlite::TestOnlySqliteAllocator;

    #[test]
    #[ignore = "only this exact test in a fresh isolated SQLite process"]
    fn actual_owned_q_cache_sqlite_refusal_preserves_old_cache_and_retries() {
        // Install precedes Fixture/init_v3/open_db and every SQLite API in this process.
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        // SAFETY: the caller runs only this ignored exact test in a fresh process,
        // with no other thread/library using SQLite until finish below.
        let guard = unsafe { TestOnlySqliteAllocator::install(&budget) }.unwrap();
        let control = guard.control();
        let mut fixture = prepared::tests::Fixture::new();
        for n in 1..=2 {
            let id = format!("raw-{n}");
            let (event, token) =
                fixture.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
            fixture.publish(&event, &token).unwrap();
        }
        let after_fixture = (budget.snapshot(), budget.ledger_health());
        assert!(after_fixture.1.sqlite_live_layout_bytes > 0);
        // The existing Q capture-image limit stays separate from the TestOnly ledger limit.
        let max = 16 * 1024 * 1024;
        let proof = capture_verified_with_allocation(
            &fixture.db_path(),
            Limits {
                max_capture_bytes: max,
                capture_deadline_ms: 20000,
                verify_deadline_ms: 20000,
            },
            Some(super::super::super::q_allocation::Profile {
                budget: budget.clone(),
                overlay_slots: 32768,
                overlay_bytes: 1_000_000,
            }),
        )
        .unwrap();
        let expected_rows = serde_json::to_value(&proof.audited.raw).unwrap();
        let expected_digest = proof.digest;
        let expected_cuts = proof.audited.cuts.len();
        let expected_objects = proof.audited.objects.len();
        let after_capture = (budget.snapshot(), budget.ledger_health());
        assert!(matches!(&proof.audited.raw, RawEvents::Owned(_)));
        assert!(proof.audited.budget.as_ref().unwrap().same_ledger(&budget));
        assert!(matches!(
            &proof.audited.cuts[0].metadata.session_id,
            super::super::super::business_json::TextValue::Owned(_)
        ));
        assert!(matches!(
            &proof.audited.cuts[0].delta,
            super::super::super::audit_store::RefValue::Owned(_)
        ));
        let after_owned_audit = after_capture;
        let deadline = Instant::now() + std::time::Duration::from_secs(30);
        let mut packed = Packed::new(proof, max, deadline).unwrap();
        assert!(packed.budget.as_ref().unwrap().same_ledger(&budget));
        let retained = (budget.snapshot(), budget.ledger_health());
        let retained_rust = retained
            .0
            .live_requested_layout_bytes
            .checked_sub(retained.1.sqlite_live_layout_bytes)
            .unwrap();
        assert!(retained_rust > 0);
        assert!(retained.1.sqlite_live_layout_bytes > 0);
        assert!(!packed.bytes.is_empty() && !packed.raw.is_empty());
        let page_hash = sha256_hex(&packed.bytes);
        let raw_hashes: Vec<_> = packed.raw.iter().map(|row| sha256_hex(row)).collect();
        let old_audited = packed.audited.as_ref().unwrap();
        let old_cuts = old_audited.cuts.as_ptr();
        let old_objects = old_audited.objects.as_ptr();
        let old_session = old_audited.cuts[0].metadata.session_id.as_str().as_ptr();
        let old_delta = old_audited.cuts[0].delta.digest().unwrap();

        // Only SQLite allocations refuse: old Rust page/raw/typed owners remain live.
        // This deliberate fault is distinct from a numeric 50 MiB quota failure.
        control.refuse_sqlite_allocations_for_testonly();
        // Diagnose the exact linked rusqlite open failure before the outer String boundary.
        // The same SQLite-only injection remains armed for the actual checkout below.
        let actual_sqlite_error = Connection::open_in_memory()
            .err()
            .expect("fresh in-memory connection must fail under SQLite-only refusal");
        let actual_sqlite_metadata = match &actual_sqlite_error {
            rusqlite::Error::SqliteFailure(sqlite, message) => {
                assert_eq!(sqlite.code, rusqlite::ffi::ErrorCode::OutOfMemory);
                assert_eq!(sqlite.extended_code & 255, rusqlite::ffi::SQLITE_NOMEM);
                json!({"variant":"SqliteFailure", "code":format!("{:?}",sqlite.code),
                    "extended_code":sqlite.extended_code, "message":message,
                    "debug":format!("{actual_sqlite_error:?}")})
            }
            other => panic!("SQLite allocation refusal misclassified: {other:?}"),
        };
        let failure = packed
            .checkout(deadline)
            .err()
            .expect("fresh actual Q checkout must propagate SQLite allocation refusal");
        let after_refusal = (budget.snapshot(), budget.ledger_health());
        println!(
            "Q_SQLITE_REFUSAL_DIAGNOSTIC {}",
            json!({
                "actual_sqlite_error":actual_sqlite_metadata,
                "checkout_error":failure,
                "retained":retained,
                "after_refusal":after_refusal,
                "request":budget.allocation_request_probe(),
                "old_audited_present":packed.audited.is_some(),
                "old_page_sha256_preserved":sha256_hex(&packed.bytes)==page_hash,
                "whole_audit_allocation_admitted":false
            })
        );
        assert!(!failure.contains(newchan_rust::s_content::capture_allocation::LIMIT));
        assert!(after_refusal.1.sqlite_rejected_layouts > retained.1.sqlite_rejected_layouts);
        assert_eq!(
            after_refusal.0.allocation_attempts,
            retained.0.allocation_attempts
        );
        assert_eq!(
            after_refusal.0.live_requested_layout_bytes,
            retained.0.live_requested_layout_bytes
        );
        assert_eq!(
            after_refusal.1.sqlite_live_layout_bytes,
            retained.1.sqlite_live_layout_bytes
        );
        assert!(!after_refusal.1.invariant_broken);
        let still_audited = packed
            .audited
            .as_ref()
            .expect("old audit owner must not be taken");
        assert_eq!(still_audited.cuts.as_ptr(), old_cuts);
        assert_eq!(still_audited.objects.as_ptr(), old_objects);
        assert_eq!(
            still_audited.cuts[0].metadata.session_id.as_str().as_ptr(),
            old_session
        );
        assert_eq!(still_audited.cuts[0].delta.digest().unwrap(), old_delta);
        assert_eq!(sha256_hex(&packed.bytes), page_hash);
        assert_eq!(
            packed
                .raw
                .iter()
                .map(|row| sha256_hex(row))
                .collect::<Vec<_>>(),
            raw_hashes
        );
        let refusal_request = budget.allocation_request_probe();
        assert!(refusal_request.refused_kind.starts_with("SQLite "));
        assert!(refusal_request.refused_bytes > 0);
        // Preserve the original failing message assertion after checking old-cache integrity.
        assert!(
            failure.to_ascii_lowercase().contains("out of memory"),
            "{failure}"
        );

        control.clear_refusal_for_testonly();
        let borrowed = packed.checkout(deadline).unwrap();
        assert!(packed.audited.is_none());
        assert!(matches!(&borrowed.audited.raw, RawEvents::Owned(_)));
        assert!(borrowed
            .audited
            .budget
            .as_ref()
            .unwrap()
            .same_ledger(&budget));
        assert_eq!(
            serde_json::to_value(&borrowed.audited.raw).unwrap(),
            expected_rows
        );
        assert_eq!(borrowed.audited.cuts.len(), expected_cuts);
        assert_eq!(borrowed.audited.objects.len(), expected_objects);
        assert_eq!(borrowed.digest, expected_digest);
        let during_checkout = (budget.snapshot(), budget.ledger_health());
        assert!(during_checkout.1.sqlite_live_layout_bytes > retained.1.sqlite_live_layout_bytes);
        let checkout_rust = during_checkout
            .0
            .live_requested_layout_bytes
            .checked_sub(during_checkout.1.sqlite_live_layout_bytes)
            .unwrap();
        assert!(
            checkout_rust > retained_rust,
            "old bytes and newly decoded raw coexist"
        );
        assert_eq!(sha256_hex(&packed.bytes), page_hash);
        packed.restore(borrowed);
        let after_restore = (budget.snapshot(), budget.ledger_health());
        assert!(packed.audited.is_some());
        assert_eq!(
            after_restore.0.live_requested_layout_bytes - after_restore.1.sqlite_live_layout_bytes,
            retained_rust
        );
        assert_eq!(sha256_hex(&packed.bytes), page_hash);
        let second = packed.checkout(deadline).unwrap();
        assert_eq!(
            serde_json::to_value(&second.audited.raw).unwrap(),
            expected_rows
        );
        packed.restore(second);
        drop(packed);
        drop(fixture);
        let after_owners_drop = (budget.snapshot(), budget.ledger_health());
        // SAFETY: all actual Fixture/capture/checkout connections, transactions and
        // statements above are dropped; this exact fresh process has no other SQLite user.
        unsafe { guard.finish() }.unwrap();
        drop(control);
        let after_finish = (budget.snapshot(), budget.ledger_health());
        assert_eq!(after_finish.0.live_requested_layout_bytes, 0);
        assert_eq!(
            after_finish.0.allocated_layouts,
            after_finish.0.released_layouts
        );
        assert_eq!(after_finish.1.sqlite_live_layout_bytes, 0);
        assert_eq!(after_finish.1.sqlite_live_payload_bytes, 0);
        assert_eq!(
            after_finish.1.sqlite_allocated_layouts,
            after_finish.1.sqlite_released_layouts
        );
        assert!(!after_finish.1.invariant_broken);
        println!(
            "Q_OWNED_SQLITE_LIFECYCLE {}",
            json!({
                "scope":"actual two-publication Q capture, same-ledger owned state audit, Packed checkout/restore; SQLite Layout/header and retained Rust page/raw/typed owners",
                "testonly_limit_bytes":50 * 1024 * 1024,
                "existing_q_image_limit_bytes":max,
                "overlay_slots":32768,
                "overlay_body_bytes":1_000_000,
                "overlay_peak_used":"profile driven cold capture; no posthoc owner replacement",
                "after_fixture":after_fixture,
                "after_capture":after_capture,
                "after_owned_audit":after_owned_audit,
                "retained":retained,
                "after_sqlite_refusal":after_refusal,
                "refusal_request":refusal_request,
                "refusal_error":failure,
                "actual_sqlite_error":actual_sqlite_metadata,
                "old_cache_preserved":true,
                "during_checkout":during_checkout,
                "after_restore":after_restore,
                "after_all_owners_drop":after_owners_drop,
                "after_finish":after_finish,
                "page_sha256":page_hash,
                "capture_digest":expected_digest.as_str(),
                "cuts":expected_cuts,
                "objects":expected_objects,
                "raw_rows":expected_rows.as_array().unwrap().len(),
                "fault_injection":true,
                "numeric_quota_failure_claimed":false,
                "whole_audit_allocation_admitted":false,
                "production_capture_enabled":false,
                "uncovered":["outer errors and diagnostics", "allocator overhead and physical RSS/IO"]
            })
        );
    }
}

impl Packed {
    #[cfg(test)]
    #[cfg_attr(
        not(feature = "s_allocation_json_testonly"),
        allow(
            dead_code,
            reason = "#1608：测试辅助，仅 s_allocation_json_testonly 测试使用"
        )
    )]
    pub(super) fn page_digest_for_testonly(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(&self.bytes).into()
    }

    pub(super) fn new(
        mut proof: VerifiedCapture,
        max: u64,
        deadline: Instant,
    ) -> Result<Self, String> {
        check(deadline, "Verify")?;
        // #1488：当前capture构造只传work=None；StageOwner的Native Q不经Packed。
        // 未来capture Native化须单独迁移字节owner，禁止在这里脱许可编码。
        if matches!(&proof.audited.raw, RawEvents::Native(_)) {
            return Err(err(
                "prototype_audit_allocation_incomplete：Packed尚未接入Native捕获",
            ));
        }
        let pages = scalar(&proof.db, c"PRAGMA page_count")?;
        let page_size = scalar(&proof.db, c"PRAGMA page_size")?;
        if pages < 0
            || page_size <= 0
            || pages.checked_mul(page_size).is_none_or(|n| n as u64 > max)
        {
            return Err(err(
                "Incomplete/RetainedBytes：私有数据库页超过保留图像上限",
            ));
        }
        let allocation = proof.allocation.clone();
        let budget = allocation.as_ref().map(|p| p.budget.clone());
        match (&budget, &proof.audited.budget) {
            (None, None) => {}
            (Some(a), Some(b)) if a.same_ledger(b) => {}
            _ => {
                return Err(err(
                    "prototype_audit_allocation_incomplete：捕获原预算与真实audit owner不同",
                ))
            }
        }
        let serialized = proof.db.serialize(rusqlite::MAIN_DB).map_err(err)?;
        let bytes = copy_bytes(&serialized, budget.as_ref())?;
        drop(serialized);
        check(deadline, "Verify")?;
        let mut out = Self {
            bytes,
            raw: result_vec(budget.as_ref())?,
            audited: None,
            digest: proof.digest,
            captured_bytes: proof.captured_bytes,
            versions: proof.source_versions,
            retained_bytes: 0,
            receipt: proof.audit_receipt,
            budget,
            allocation,
            q_source: proof.q_source.take(),
        };
        // 审计后的完整原始账逐条保留规范JSON字节；所有类型/字段/原文均保留。
        let decoded = std::mem::take(&mut proof.audited.raw);
        let mut remaining = max.saturating_sub(out.bytes.capacity() as u64);
        for record in decoded.iter() {
            check(deadline, "Verify")?;
            let bytes = encode_record(record, out.budget.as_ref(), remaining, deadline)?;
            remaining = remaining.saturating_sub(bytes.capacity() as u64);
            out.raw.try_push(bytes).map_err(err)?;
        }
        drop(decoded);
        out.audited = Some(proof.audited);
        out.retained_bytes = out.measure(max, deadline)?;
        Ok(out)
    }
    pub(super) fn checkout(&mut self, deadline: Instant) -> Result<VerifiedCapture, String> {
        check(deadline, "Verify")?;
        let mut db = Connection::open_in_memory().map_err(sql_err)?;
        // slice长度已确定，read_exact不会发生短读；只读deserialize不接受外部任意SQLite字节。
        db.deserialize_read_exact(rusqlite::MAIN_DB, &self.bytes[..], self.bytes.len(), true)
            .map_err(sql_err)?;
        db.pragma_update(None, "query_only", true)
            .map_err(sql_err)?;
        let raw = decode_records(&self.raw, self.budget.as_ref(), deadline)?;
        check(deadline, "Verify")?;
        let mut audited = self.audited.take().ok_or_else(|| err("保留图像已借出"))?;
        audited.raw = raw;
        Ok(VerifiedCapture {
            db,
            audited,
            digest: self.digest.clone(),
            captured_bytes: self.captured_bytes,
            source_versions: self.versions,
            verify_deadline: deadline,
            // 用量收据仍由Packed持有，不能通过借出方法复制成第二份授权。
            audit_receipt: None,
            allocation: self.allocation.clone(),
            q_source: self.q_source.take(),
        })
    }
    pub(super) fn replay(
        &self,
        job: &mut dyn newchan_rust::s_content::access::Access,
    ) -> Result<(), String> {
        self.receipt
            .as_ref()
            .ok_or_else(|| err("未计费的legacy捕获不得注册job缓存"))?
            .replay(job)
    }
    pub(super) fn restore(&mut self, mut proof: VerifiedCapture) {
        debug_assert!(match (&self.allocation, &proof.allocation) {
            (None, None) => true,
            (Some(a), Some(b)) => a.budget.same_ledger(&b.budget),
            _ => false,
        });
        // 此模块的调用口不开放db写入或审计元数据修改；仅释放本次临时解码和私有句柄。
        proof.audited.raw = RawEvents::default();
        self.q_source = proof.q_source.take();
        self.audited = Some(proof.audited);
    }
    pub(super) fn measure(&self, max: u64, deadline: Instant) -> Result<u64, String> {
        let a = self.audited.as_ref().ok_or_else(|| err("保留图像已借出"))?;
        let mut m = Meter {
            n: 0,
            max,
            deadline,
        };
        m.add(size_of::<Self>())?;
        if let Some(receipt) = &self.receipt {
            m.add(usize::try_from(receipt.retained_heap_bytes()?).map_err(err)?)?;
        }
        if let Some(source) = &self.q_source {
            m.add(usize::try_from(source.retained_layout_bytes().map_err(err)?).map_err(err)?)?;
        }
        m.add(self.bytes.capacity())?;
        m.add(self.digest.capacity())?;
        m.codec_vec(&self.raw)?;
        for row in &self.raw {
            m.add(row.capacity())?;
        }
        // audited本体已经在Option字段内；下面仅计各字段拥有的堆内存。
        // Packed has moved raw into owned encoded records; this is an empty placeholder.
        match &a.raw {
            RawEvents::Plain(v) => m.vec(v)?,
            RawEvents::Native(_) => {
                return Err(err(
                    "Incomplete/RetainedBytes：保留图像未移出Native原始账树",
                ))
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            RawEvents::Owned(_) => {
                return Err(err("Incomplete/RetainedBytes：保留图像未移出原始账树"))
            }
        }
        m.vec(&a.cuts)?;
        for cut in &a.cuts {
            let h = &cut.metadata;
            let s = &h.source_commit;
            for text in [
                &h.session_id,
                &h.session_generation,
                &h.writer_epoch,
                &h.base_cut,
                &h.structure_cut,
                &h.clock_plan_hash,
                &h.semantic_commit_ns,
                &h.profile_id,
                &h.profile_hash,
                &h.rule_revision,
                &h.catalog_revision,
                &s.source_namespace,
                &s.source_epoch,
                &s.message_id,
                &s.payload_hash,
                &s.producer_id,
                &s.producer_epoch,
                &s.clock_event_id,
                &cut.batch_id,
            ] {
                m.add(text.capacity())?;
            }
            for r in [
                &h.scope,
                &cut.roots.generated_state,
                &cut.roots.publication_template,
                &cut.roots.delta_template,
                &cut.snapshot,
                &cut.delta,
            ] {
                m.reference(r)?;
            }
        }
        m.vec(&a.objects)?;
        for object in &a.objects {
            for s in [&object.id, &object.kind, &object.life.first_known_cut] {
                m.add(s.capacity())?;
            }
            if let object_index::Slot::Typed { kind, fact_key } = &object.slot {
                m.add(kind.capacity())?;
                m.add(fact_key.capacity())?;
            }
            for s in [&object.life.withdrawal_reason, &object.life.superseded_by]
                .into_iter()
                .flatten()
            {
                m.add(s.capacity())?;
            }
            if let object_records::BatchOwner::Existing(s) = &object.life.batch {
                m.add(s.capacity())?;
            }
            m.reference(&object.birth_wire)?;
        }
        m.vec(&a.auxiliary.witnesses)?;
        for row in &a.auxiliary.witnesses {
            m.add(row.id.capacity())?;
            m.add(row.object_id.capacity())?;
            m.reference(&row.generated)?;
            m.reference(&row.wire)?;
        }
        m.vec(&a.auxiliary.relations)?;
        for row in &a.auxiliary.relations {
            for s in [&row.id, &row.subject, &row.relation_type, &row.object] {
                m.add(s.capacity())?;
            }
            m.reference(&row.generated)?;
        }
        m.vec(&a.auxiliary.observations)?;
        for row in &a.auxiliary.observations {
            m.add(row.id.capacity())?;
            m.add(row.kind.capacity())?;
            if let Some(s) = &row.batch {
                m.add(s.capacity())?;
            }
            m.reference(&row.generated)?;
            m.reference(&row.wire)?;
        }
        Ok(m.n)
    }
}
struct Meter {
    n: u64,
    max: u64,
    deadline: Instant,
}
trait RetainedCapacity<T> {
    fn retained_capacity(&self) -> usize;
}
impl<T> RetainedCapacity<T> for Vec<T> {
    fn retained_capacity(&self) -> usize {
        self.capacity()
    }
}
impl<T> RetainedCapacity<T> for CodecVec<T> {
    fn retained_capacity(&self) -> usize {
        self.capacity()
    }
}
trait RetainedReference {
    fn retained_text_capacity(&self) -> usize;
}
impl RetainedReference for ContentRef {
    fn retained_text_capacity(&self) -> usize {
        self.format.capacity() + self.root.capacity()
    }
}
impl RetainedReference for super::super::audit_store::RefValue {
    fn retained_text_capacity(&self) -> usize {
        match self {
            Self::Plain(r) => r.retained_text_capacity(),
            Self::Owned(_) => 0,
        }
    }
}
impl Meter {
    fn add(&mut self, n: usize) -> Result<(), String> {
        check(self.deadline, "Verify")?;
        self.n = self
            .n
            .checked_add(n as u64)
            .ok_or_else(|| err("保留容量溢出"))?;
        if self.n > self.max {
            return Err(err(
                "Incomplete/RetainedBytes：完整保留图像超过max_capture_bytes",
            ));
        }
        Ok(())
    }
    fn vec<T>(&mut self, v: &impl RetainedCapacity<T>) -> Result<(), String> {
        self.add(
            v.retained_capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| err("保留容量溢出"))?,
        )
    }
    fn codec_vec<T>(&mut self, v: &CodecVec<T>) -> Result<(), String> {
        self.add(
            v.capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| err("保留容量溢出"))?,
        )
    }
    fn reference(&mut self, r: &impl RetainedReference) -> Result<(), String> {
        self.add(r.retained_text_capacity())
    }
}

fn result_vec<T>(budget: Option<&OwnedBudget>) -> Result<CodecVec<T>, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        return Ok(newchan_rust::s_content::capture_allocation::audit_json::owned_vec(b));
    }
    if budget.is_some() {
        return Err(err(
            "prototype_audit_allocation_incomplete：owned缓存缺所有者编译入口",
        ));
    }
    Ok(CodecVec::from_plain(Vec::new()))
}
fn copy_bytes(bytes: &[u8], budget: Option<&OwnedBudget>) -> Result<CodecVec<u8>, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        return newchan_rust::s_content::capture_allocation::audit_json::owned_bytes(bytes, b)
            .map_err(err);
    }
    if budget.is_some() {
        return Err(err(
            "prototype_audit_allocation_incomplete：owned缓存不可退回std字节",
        ));
    }
    Ok(CodecVec::from_plain(bytes.to_vec()))
}
fn encode_record(
    view: newchan_rust::s_content::capture_allocation::audit_json::JsonView<'_>,
    budget: Option<&OwnedBudget>,
    max: u64,
    deadline: Instant,
) -> Result<CodecVec<u8>, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        return newchan_rust::s_content::capture_allocation::audit_json::canonical_bytes_bounded(
            view,
            b,
            max,
            || {
                if Instant::now() >= deadline {
                    Err(
                        newchan_rust::s_content::capture_allocation::audit_json::static_error(
                            "Incomplete/VerifyTimeout",
                            "原绝对核验期限已到",
                        ),
                    )
                } else {
                    Ok(())
                }
            },
        )
        .map_err(err);
    }
    if budget.is_some() {
        return Err(err(
            "prototype_audit_allocation_incomplete：owned缓存不可退回std编码",
        ));
    }
    let mut sink = Bounded {
        bytes: Vec::new(),
        max,
        deadline,
    };
    serde_json::to_writer(&mut sink, &view).map_err(err)?;
    Ok(CodecVec::from_plain(sink.bytes))
}
fn decode_records(
    rows: &CodecVec<CodecVec<u8>>,
    budget: Option<&OwnedBudget>,
    deadline: Instant,
) -> Result<RawEvents, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        use newchan_rust::s_content::capture_allocation::audit_json::OwnedJson;
        let mut all = OwnedJson::array(b).map_err(err)?;
        for bytes in rows.iter() {
            check(deadline, "Verify")?;
            let row = OwnedJson::parse(bytes, b).map_err(err)?;
            all.push(row.view()).map_err(err)?;
        }
        return Ok(RawEvents::Owned(all));
    }
    if budget.is_some() {
        return Err(err(
            "prototype_audit_allocation_incomplete：owned缓存不可退回std解析",
        ));
    }
    let mut out = Vec::new();
    for bytes in rows.iter() {
        check(deadline, "Verify")?;
        out.push(newchan_rust::session_protocol::strict_json(bytes)?);
    }
    Ok(RawEvents::Plain(out))
}
struct Bounded {
    bytes: Vec<u8>,
    max: u64,
    deadline: Instant,
}
impl Write for Bounded {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        check(self.deadline, "Verify").map_err(std::io::Error::other)?;
        if self.bytes.len() as u128 + b.len() as u128 > u128::from(self.max) {
            return Err(std::io::Error::other(
                "Incomplete/RetainedBytes：原始账字节超限",
            ));
        }
        self.bytes.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod owned_cache_tests {
    use super::*;
    use newchan_rust::s_content::capture_allocation::audit_json::{JsonView, OwnedJson};

    #[test]
    fn actual_owned_capture_packing_checkout_restore_preserves_rows_and_ledger() {
        use newchan_rust::s_content::capture_allocation::CaptureSlab;
        let mut fixture = prepared::tests::Fixture::new();
        for n in 1..=2 {
            let id = format!("raw-{n}");
            let (event, token) =
                fixture.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
            fixture.publish(&event, &token).unwrap();
        }
        let max = 16 * 1024 * 1024;
        let budget = OwnedBudget::new(max as usize);
        let proof = capture_verified_with_allocation(
            &fixture.db_path(),
            Limits {
                max_capture_bytes: max,
                capture_deadline_ms: 20000,
                verify_deadline_ms: 20000,
            },
            Some(super::super::super::q_allocation::Profile {
                budget: budget.clone(),
                overlay_slots: 4096,
                overlay_bytes: 2 * 1024 * 1024,
            }),
        )
        .unwrap();
        let expected_rows = serde_json::to_value(&proof.audited.raw).unwrap();
        let expected_digest = proof.digest;
        let expected_cuts = proof.audited.cuts.len();
        let expected_objects = proof.audited.objects.len();
        assert!(matches!(&proof.audited.raw, RawEvents::Owned(_)));
        assert!(proof.audited.budget.as_ref().unwrap().same_ledger(&budget));
        let deadline = Instant::now() + std::time::Duration::from_secs(30);
        let mut packed = Packed::new(proof, max, deadline).unwrap();
        assert!(packed.budget.as_ref().unwrap().same_ledger(&budget));
        let retained = budget.snapshot().live_requested_layout_bytes;
        assert!(retained > 0);
        let page_hash = sha256_hex(&packed.bytes);
        let raw_hashes: Vec<_> = packed.raw.iter().map(|r| sha256_hex(r)).collect();
        assert_eq!(
            packed.measure(packed.retained_bytes, deadline).unwrap(),
            packed.retained_bytes
        );
        assert!(packed
            .measure(packed.retained_bytes - 1, deadline)
            .unwrap_err()
            .contains("RetainedBytes"));

        // 保留真实缓存，仅剩一字节可分配；checkout失败不得take旧审计状态或销掉旧字节。
        let ballast = CaptureSlab::new(0, max as usize - retained - 1, &budget).unwrap();
        let before_failure = budget.snapshot().live_requested_layout_bytes;
        let failure = packed
            .checkout(deadline)
            .err()
            .expect("真实checkout应被同账解码门拒绝");
        assert!(
            failure.contains(newchan_rust::s_content::capture_allocation::LIMIT),
            "{failure}"
        );
        assert!(packed.audited.is_some());
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before_failure
        );
        assert_eq!(sha256_hex(&packed.bytes), page_hash);
        assert_eq!(
            packed.raw.iter().map(|r| sha256_hex(r)).collect::<Vec<_>>(),
            raw_hashes
        );
        drop(ballast);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, retained);

        let borrowed = packed.checkout(deadline).unwrap();
        assert!(packed.audited.is_none());
        assert!(borrowed
            .audited
            .budget
            .as_ref()
            .unwrap()
            .same_ledger(&budget));
        assert!(matches!(&borrowed.audited.raw, RawEvents::Owned(_)));
        assert!(budget.snapshot().live_requested_layout_bytes > retained);
        assert_eq!(
            serde_json::to_value(&borrowed.audited.raw).unwrap(),
            expected_rows
        );
        assert_eq!(borrowed.audited.cuts.len(), expected_cuts);
        assert_eq!(borrowed.audited.objects.len(), expected_objects);
        assert_eq!(borrowed.digest, expected_digest);
        packed.restore(borrowed);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, retained);
        assert_eq!(sha256_hex(&packed.bytes), page_hash);
        let second = packed.checkout(deadline).unwrap();
        assert_eq!(
            serde_json::to_value(&second.audited.raw).unwrap(),
            expected_rows
        );
        packed.restore(second);
        drop(packed);
        let after = budget.snapshot();
        assert_eq!(after.live_requested_layout_bytes, 0);
        assert_eq!(after.allocated_layouts, after.released_layouts);
        assert!(!budget.ledger_health().invariant_broken);
    }

    #[test]
    fn owned_encoded_rows_checkout_keep_original_ledger_and_failed_decode_keeps_bytes() {
        let deadline = Instant::now() + std::time::Duration::from_secs(30);
        let input = br#"{"key":"escaped\ntext","n":18446744073709551615,"value":[true,null,-0.0]}"#;
        let normal = newchan_rust::session_protocol::strict_json(input).unwrap();
        let budget = OwnedBudget::new(1024 * 1024);
        let tree = OwnedJson::parse(input, &budget).unwrap();
        let bytes = encode_record(tree.view(), Some(&budget), 8192, deadline).unwrap();
        let expected = newchan_rust::session_protocol::canonical_json(&normal);
        assert_eq!(&bytes[..], expected.as_bytes());
        let mut rows = result_vec(Some(&budget)).unwrap();
        rows.try_push(bytes).unwrap();
        drop(tree);
        let retained = budget.snapshot().live_requested_layout_bytes;
        assert!(retained > 0);
        let decoded = decode_records(&rows, Some(&budget), deadline).unwrap();
        assert!(matches!(&decoded, RawEvents::Owned(_)));
        assert_eq!(serde_json::to_value(&decoded).unwrap(), json!([normal]));
        assert!(budget.snapshot().live_requested_layout_bytes > retained);
        drop(decoded);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, retained);
        assert_eq!(&rows[0][..], expected.as_bytes());
        let encoded_len = rows[0].len();
        drop(rows);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);

        // 相同输入的保留字节先成功创建，只留1字节给新树，不能销掉旧缓存后再解码。
        let sizing = OwnedBudget::new(1024 * 1024);
        let mut measured_old = result_vec(Some(&sizing)).unwrap();
        measured_old
            .try_push(copy_bytes(expected.as_bytes(), Some(&sizing)).unwrap())
            .unwrap();
        let baseline = sizing.snapshot().live_requested_layout_bytes;
        drop(measured_old);
        assert_eq!(sizing.snapshot().live_requested_layout_bytes, 0);
        let tight = OwnedBudget::new(baseline + 1);
        let mut old = result_vec(Some(&tight)).unwrap();
        old.try_push(copy_bytes(expected.as_bytes(), Some(&tight)).unwrap())
            .unwrap();
        let old_live = tight.snapshot().live_requested_layout_bytes;
        assert_eq!(old_live, baseline);
        let error = decode_records(&old, Some(&tight), deadline).unwrap_err();
        assert!(
            error.contains(newchan_rust::s_content::capture_allocation::LIMIT),
            "{error}"
        );
        assert_eq!(tight.snapshot().live_requested_layout_bytes, old_live);
        assert_eq!(old[0].len(), encoded_len);
        assert_eq!(&old[0][..], expected.as_bytes());
        drop(old);
        assert_eq!(tight.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(
            tight.snapshot().allocated_layouts,
            tight.snapshot().released_layouts
        );

        let ample = OwnedBudget::new(65536);
        assert!(
            encode_record(JsonView::Plain(&normal), Some(&ample), 1, deadline)
                .unwrap_err()
                .contains("CaptureBytes")
        );
        assert!(
            encode_record(JsonView::Plain(&normal), Some(&ample), 8192, Instant::now())
                .unwrap_err()
                .contains("VerifyTimeout")
        );
        assert_eq!(ample.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod native_boundary_tests {
    use super::super::super::{audit_owner::native::Budget, business_json as bj};
    use super::*;

    #[test]
    fn plain_capture_roundtrip_stays_plain_and_unreachable_native_injection_is_rejected() {
        let mut fixture = prepared::tests::Fixture::new();
        let (event, token) = fixture.accept_begin(1, ("packed-original", 0, 6, 12, 3, 7), 1);
        fixture.publish(&event, &token).unwrap();
        let max = 16 * 1024 * 1024;
        let limits = Limits {
            max_capture_bytes: max,
            capture_deadline_ms: 20000,
            verify_deadline_ms: 20000,
        };
        let proof = capture_verified(&fixture.db_path(), limits).unwrap();
        assert!(matches!(&proof.audited.raw, RawEvents::Plain(_)));
        let expected = serde_json::to_value(&proof.audited.raw).unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(30);
        let mut packed = Packed::new(proof, max, deadline).unwrap();
        for _ in 0..2 {
            let borrowed = packed.checkout(deadline).unwrap();
            assert!(matches!(&borrowed.audited.raw, RawEvents::Plain(_)));
            assert_eq!(
                serde_json::to_value(&borrowed.audited.raw).unwrap(),
                expected
            );
            packed.restore(borrowed);
        }
        let mut proof = packed.checkout(deadline).unwrap();
        let budget = Budget::new(max as usize, 0).unwrap();
        let baseline = budget.live();
        // 仅测试强行注入：生产verify_image_metered没有WorkScope参数。
        proof.audited.raw = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut rows = bj::ScopedJsonArray::native(work)?;
                for row in expected.as_array().unwrap() {
                    rows.push(row.into())?;
                }
                bj::RawEvents::from_json_owner(rows.finish())
            })
            .unwrap();
        assert!(budget.live() > baseline);
        let error = Packed::new(proof, max, deadline)
            .err()
            .expect("Native捕获尚未获准");
        assert!(error.contains("Packed尚未接入Native捕获"));
        assert_eq!(budget.live(), baseline, "拒绝后实际owner归还原许可");
    }
}
