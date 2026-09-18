//! #1394：Q私有缓存的可计量表示。全数据库页和原始账保留为拥有的字节；
//! 类型化审计索引保留原值，不存动态JSON树或跨请求SQLite事务。
//! 计量是实际Vec/String容量与类型大小，不是RSS，也不含查询期间的临时解码对象。
use super::*;
use newchan_rust::s_content::ContentRef;

pub(super) struct Packed {
    bytes: Vec<u8>,
    raw: Vec<Vec<u8>>,
    audited: Option<state::AuditedState>,
    digest: String,
    captured_bytes: u64,
    versions: [i64; 3],
    pub(super) retained_bytes: u64,
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
        let bytes = proof.db.serialize(rusqlite::MAIN_DB).map_err(err)?.to_vec();
        check(deadline, "Verify")?;
        let mut out = Self {
            bytes,
            raw: Vec::new(),
            audited: None,
            digest: proof.digest,
            captured_bytes: proof.captured_bytes,
            versions: proof.source_versions,
            retained_bytes: 0,
        };
        // 审计后的完整原始账逐条保留规范JSON字节；所有类型/字段/原文均保留。
        let decoded = std::mem::take(&mut proof.audited.raw);
        let mut remaining = max.saturating_sub(out.bytes.capacity() as u64);
        for record in decoded {
            check(deadline, "Verify")?;
            let mut sink = Bounded {
                bytes: Vec::new(),
                max: remaining,
                deadline,
            };
            serde_json::to_writer(&mut sink, &record).map_err(err)?;
            remaining = remaining.saturating_sub(sink.bytes.capacity() as u64);
            out.raw.push(sink.bytes);
        }
        out.audited = Some(proof.audited);
        out.retained_bytes = out.measure(max, deadline)?;
        Ok(out)
    }
    pub(super) fn checkout(&mut self, deadline: Instant) -> Result<VerifiedCapture, String> {
        check(deadline, "Verify")?;
        let mut db = Connection::open_in_memory().map_err(sql_err)?;
        // slice长度已确定，read_exact不会发生短读；只读deserialize不接受外部任意SQLite字节。
        db.deserialize_read_exact(
            rusqlite::MAIN_DB,
            self.bytes.as_slice(),
            self.bytes.len(),
            true,
        )
        .map_err(sql_err)?;
        db.pragma_update(None, "query_only", true)
            .map_err(sql_err)?;
        let mut raw = Vec::new();
        for bytes in &self.raw {
            check(deadline, "Verify")?;
            raw.push(newchan_rust::session_protocol::strict_json(bytes)?);
        }
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
        })
    }
    pub(super) fn restore(&mut self, mut proof: VerifiedCapture) {
        // 此模块的调用口不开放db写入或审计元数据修改；仅释放本次临时解码和私有句柄。
        proof.audited.raw = Vec::new();
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
        m.add(self.bytes.capacity())?;
        m.add(self.digest.capacity())?;
        m.vec(&self.raw)?;
        for row in &self.raw {
            m.add(row.capacity())?;
        }
        // audited本体已经在Option字段内；下面仅计各字段拥有的堆内存。
        m.vec(&a.raw)?;
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
            if let object_records::BatchRef::Existing(s) = &object.life.batch {
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
    fn vec<T>(&mut self, v: &Vec<T>) -> Result<(), String> {
        self.add(
            v.capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| err("保留容量溢出"))?,
        )
    }
    fn reference(&mut self, r: &ContentRef) -> Result<(), String> {
        self.add(r.format.capacity())?;
        self.add(r.root.capacity())
    }
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
