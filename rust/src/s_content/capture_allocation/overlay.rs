use super::*;

/// #1394 fixed-capacity derived-node owner, used only by the test-only captured path.
/// Requested allocation capacity stays charged through clear and until Drop.
#[doc(hidden)]
pub struct OverlaySlab {
    slab: CaptureSlab,
    peak_slots: usize,
    peak_body: usize,
}
impl OverlaySlab {
    pub fn requested_bytes(slots: usize, body_bytes: usize) -> Result<usize, CodecError> {
        CaptureSlab::requested_bytes(slots, body_bytes)
    }
    pub fn new(slots: usize, body_bytes: usize, budget: &OwnedBudget) -> Result<Self, CodecError> {
        Ok(Self {
            slab: CaptureSlab::new(slots, body_bytes, budget)?,
            peak_slots: 0,
            peak_body: 0,
        })
    }
    pub fn original(&self, digest: &NodeDigest) -> Option<(u8, &[u8])> {
        self.slab.original(digest)
    }
    pub fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        let s = &mut self.slab;
        let position = s.records().binary_search_by_key(&digest, |r| r.digest);
        let old = position.ok().map(|i| s.records()[i]);
        if let Some((old_tag, old_body)) = s.original(&digest) {
            if old_tag == tag && old_body == body {
                return Ok(());
            }
        }
        let old_len = old.map_or(0, |r| r.len);
        let needed = s
            .body_used
            .checked_sub(old_len)
            .and_then(|n| n.checked_add(body.len()))
            .ok_or_else(refused)?;
        if (old.is_none() && s.used == s.count) || needed > s.body.layout.size() {
            return Err(CodecError::new(
                LIMIT,
                "overlay 槽数或正文 arena 已满，复制前拒绝",
            ));
        }
        // SAFETY: privately owned fixed arrays; all capacity/overflow checks above precede
        // writes. copy permits overlap for compaction and sorted-index insertion.
        unsafe {
            let records = s.index.pointer.as_ptr().cast::<Record>();
            if let Some(r) = old {
                let end = r.offset + r.len;
                std::ptr::copy(
                    s.body.pointer.as_ptr().add(end),
                    s.body.pointer.as_ptr().add(r.offset),
                    s.body_used - end,
                );
                for i in 0..s.used {
                    let record = &mut *records.add(i);
                    if record.offset > r.offset {
                        record.offset -= r.len;
                    }
                }
                s.body_used -= r.len;
            }
            let i = match position {
                Ok(i) => i,
                Err(i) => {
                    std::ptr::copy(records.add(i), records.add(i + 1), s.used - i);
                    s.used += 1;
                    i
                }
            };
            std::ptr::copy_nonoverlapping(
                body.as_ptr(),
                s.body.pointer.as_ptr().add(s.body_used),
                body.len(),
            );
            records.add(i).write(Record {
                digest,
                tag,
                offset: s.body_used,
                len: body.len(),
            });
        }
        s.body_used = needed;
        let m = &s.body.budget.0;
        m.copied_rows.set(m.copied_rows.get() + 1);
        m.copied_bytes.set(m.copied_bytes.get() + body.len() as u64);
        self.peak_slots = self.peak_slots.max(s.used);
        self.peak_body = self.peak_body.max(s.body_used);
        Ok(())
    }
    pub fn clear(&mut self) {
        // Reset initialized ranges only. Capacity and Layout owners remain live.
        unsafe {
            std::ptr::write_bytes(
                self.slab.index.pointer.as_ptr().cast::<Record>(),
                0,
                self.slab.used,
            );
            std::ptr::write_bytes(self.slab.body.pointer.as_ptr(), 0, self.slab.body_used);
        }
        self.slab.used = 0;
        self.slab.body_used = 0;
    }
    pub fn peak_used(&self) -> (usize, usize) {
        (self.peak_slots, self.peak_body)
    }
}
