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
}
impl Packed {
    pub(super) fn new(
        mut proof: VerifiedCapture,
        max: u64,
        deadline: Instant,
    ) -> Result<Self, String> {
        check(deadline, "Verify")?;
        let pages: i64 = proof
            .db
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .map_err(err)?;
        let page_size: i64 = proof
            .db
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .map_err(err)?;
        if pages < 0
            || page_size <= 0
            || pages.checked_mul(page_size).is_none_or(|n| n as u64 > max)
        {
            return Err(err(
                "Incomplete/RetainedBytes：私有数据库页超过保留图像上限",
            ));
        }
        let budget = proof.audited.budget.clone();
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
        // 此模块的调用口不开放db写入或审计元数据修改；仅释放本次临时解码和私有句柄。
        proof.audited.raw = RawEvents::default();
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
        let mut proof = capture_verified(
            &fixture.db_path(),
            Limits {
                max_capture_bytes: max,
                capture_deadline_ms: 20000,
                verify_deadline_ms: 20000,
            },
        )
        .unwrap();
        let expected_rows = serde_json::to_value(&proof.audited.raw).unwrap();
        let expected_digest = proof.digest;
        let expected_cuts = proof.audited.cuts.len();
        let expected_objects = proof.audited.objects.len();
        let budget = OwnedBudget::new(max as usize);
        let tx = proof.db.transaction().unwrap();
        let owned = history::with_overlay_prototype(
            history::OverlayPrototype {
                slots: 4096,
                bytes: 2 * 1024 * 1024,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || state::audit_owned_capture_for_testonly(&tx, &budget),
        )
        .unwrap();
        drop(tx);
        assert!(matches!(&owned.raw, RawEvents::Owned(_)));
        assert!(owned.budget.as_ref().unwrap().same_ledger(&budget));
        proof.audited = owned;
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
