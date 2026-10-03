use super::*;

// #1394 fixed TestOnly instrument; this does not qualify whole-audit heap coverage.
#[cfg(feature = "s_allocation_json_testonly")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestOnlyOverlayRefusalReason {
    Slots,
    Body,
    Both,
    Overflow,
}

#[cfg(feature = "s_allocation_json_testonly")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TestOnlyOverlayRefusal {
    pub used_slots: usize,
    pub needed_slots: Option<usize>,
    pub slot_capacity: usize,
    pub used_body_bytes: usize,
    pub needed_body_bytes: Option<usize>,
    pub body_capacity: usize,
    pub reason: TestOnlyOverlayRefusalReason,
}

#[cfg(feature = "s_allocation_json_testonly")]
thread_local! {
    static FIRST_OVERLAY_REFUSAL: std::cell::Cell<Option<TestOnlyOverlayRefusal>> = const { std::cell::Cell::new(None) };
}

#[cfg(feature = "s_allocation_json_testonly")]
#[doc(hidden)]
pub fn reset_first_overlay_refusal_for_testonly() {
    FIRST_OVERLAY_REFUSAL.with(|first| first.set(None));
}

#[cfg(feature = "s_allocation_json_testonly")]
#[doc(hidden)]
pub fn take_first_overlay_refusal_for_testonly() -> Option<TestOnlyOverlayRefusal> {
    FIRST_OVERLAY_REFUSAL.with(|first| first.take())
}

#[cfg(feature = "s_allocation_json_testonly")]
fn note_overlay_refusal(
    s: &OverlayStorage,
    new_slot: bool,
    needed_body_bytes: Option<usize>,
    reason: TestOnlyOverlayRefusalReason,
) {
    FIRST_OVERLAY_REFUSAL.with(|first| {
        if first.get().is_none() {
            first.set(Some(TestOnlyOverlayRefusal {
                used_slots: s.used,
                needed_slots: s.used.checked_add(usize::from(new_slot)),
                slot_capacity: s.count,
                used_body_bytes: s.body_used,
                needed_body_bytes,
                body_capacity: s.body.layout.size(),
                reason,
            }));
        }
    });
}

// #1394 overlay appends bodies in write order while sorting its digest index.
// Its lengths must remain explicit; CaptureSlab's adjacent-offset rule does not apply.
#[derive(Clone, Copy)]
#[repr(C)]
struct OverlayRecord {
    digest: NodeDigest,
    offset: usize,
    len: usize,
    tag: u8,
}

struct OverlayStorage {
    index: OwnedLayout,
    body: OwnedLayout,
    count: usize,
    used: usize,
    body_used: usize,
}
impl OverlayStorage {
    fn requested_bytes(count: usize, bodies: usize) -> Result<usize, CodecError> {
        let index = Layout::array::<OverlayRecord>(count).map_err(|_| refused())?;
        Layout::array::<u8>(bodies).map_err(|_| refused())?;
        index.size().checked_add(bodies).ok_or_else(refused)
    }
    fn new(count: usize, bodies: usize, budget: &OwnedBudget) -> Result<Self, CodecError> {
        budget.preflight(Self::requested_bytes(count, bodies)?)?;
        Ok(Self {
            index: OwnedLayout::new(
                Layout::array::<OverlayRecord>(count).map_err(|_| refused())?,
                budget,
            )?,
            body: OwnedLayout::new(Layout::array::<u8>(bodies).map_err(|_| refused())?, budget)?,
            count,
            used: 0,
            body_used: 0,
        })
    }
    fn records(&self) -> &[OverlayRecord] {
        // SAFETY: index has count aligned, zero-initialized OverlayRecords, and only
        // the used <= count prefix published by successful put is exposed.
        unsafe { std::slice::from_raw_parts(self.index.pointer.as_ptr().cast(), self.used) }
    }
    fn original(&self, digest: &NodeDigest) -> Option<(u8, &[u8])> {
        let i = self
            .records()
            .binary_search_by_key(digest, |r| r.digest)
            .ok()?;
        let r = self.records()[i];
        // SAFETY: successful put checks arena bounds before publishing offset/len;
        // replacements compact bodies and repair offsets before returning. Shared
        // self prevents mutation, including for an empty body's valid zero-length slice.
        let body =
            unsafe { std::slice::from_raw_parts(self.body.pointer.as_ptr().add(r.offset), r.len) };
        Some((r.tag, body))
    }
}

/// #1394 fixed-capacity derived-node owner, used only by the test-only captured path.
/// Requested allocation capacity stays charged through clear and until Drop.
#[doc(hidden)]
pub struct OverlaySlab {
    slab: OverlayStorage,
    peak_slots: usize,
    peak_body: usize,
}
impl OverlaySlab {
    pub fn requested_bytes(slots: usize, body_bytes: usize) -> Result<usize, CodecError> {
        OverlayStorage::requested_bytes(slots, body_bytes)
    }
    pub fn new(slots: usize, body_bytes: usize, budget: &OwnedBudget) -> Result<Self, CodecError> {
        Ok(Self {
            slab: OverlayStorage::new(slots, body_bytes, budget)?,
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
            .ok_or_else(|| {
                #[cfg(feature = "s_allocation_json_testonly")]
                note_overlay_refusal(
                    s,
                    old.is_none(),
                    None,
                    TestOnlyOverlayRefusalReason::Overflow,
                );
                refused()
            })?;
        if (old.is_none() && s.used == s.count) || needed > s.body.layout.size() {
            #[cfg(feature = "s_allocation_json_testonly")]
            note_overlay_refusal(
                s,
                old.is_none(),
                Some(needed),
                match (
                    old.is_none() && s.used == s.count,
                    needed > s.body.layout.size(),
                ) {
                    (true, true) => TestOnlyOverlayRefusalReason::Both,
                    (true, false) => TestOnlyOverlayRefusalReason::Slots,
                    (false, _) => TestOnlyOverlayRefusalReason::Body,
                },
            );
            return Err(CodecError::new(
                LIMIT,
                "overlay 槽数或正文 arena 已满，复制前拒绝",
            ));
        }
        // SAFETY: privately owned fixed arrays; all capacity/overflow checks above precede
        // writes. copy permits overlap for compaction and sorted-index insertion.
        unsafe {
            let records = s.index.pointer.as_ptr().cast::<OverlayRecord>();
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
            records.add(i).write(OverlayRecord {
                digest,
                tag,
                offset: s.body_used,
                len: body.len(),
            });
        }
        s.body_used = needed;
        s.body.budget.note_copy(body.len());
        self.peak_slots = self.peak_slots.max(s.used);
        self.peak_body = self.peak_body.max(s.body_used);
        Ok(())
    }
    pub fn clear(&mut self) {
        // SAFETY: reset only the initialized used-record/body prefixes within the
        // privately owned allocations. Capacity and Layout owners remain live.
        unsafe {
            std::ptr::write_bytes(
                self.slab.index.pointer.as_ptr().cast::<OverlayRecord>(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn first_refusal_reports_exact_capacities_without_store_or_budget_changes() {
        for (slots, bodies, digest, body, expected) in [
            (
                1,
                8,
                [2; 32],
                &b"x"[..],
                TestOnlyOverlayRefusal {
                    used_slots: 1,
                    needed_slots: Some(2),
                    slot_capacity: 1,
                    used_body_bytes: 3,
                    needed_body_bytes: Some(4),
                    body_capacity: 8,
                    reason: TestOnlyOverlayRefusalReason::Slots,
                },
            ),
            (
                2,
                3,
                [1; 32],
                &b"abcd"[..],
                TestOnlyOverlayRefusal {
                    used_slots: 1,
                    needed_slots: Some(1),
                    slot_capacity: 2,
                    used_body_bytes: 3,
                    needed_body_bytes: Some(4),
                    body_capacity: 3,
                    reason: TestOnlyOverlayRefusalReason::Body,
                },
            ),
            (
                1,
                3,
                [2; 32],
                &b"x"[..],
                TestOnlyOverlayRefusal {
                    used_slots: 1,
                    needed_slots: Some(2),
                    slot_capacity: 1,
                    used_body_bytes: 3,
                    needed_body_bytes: Some(4),
                    body_capacity: 3,
                    reason: TestOnlyOverlayRefusalReason::Both,
                },
            ),
        ] {
            reset_first_overlay_refusal_for_testonly();
            let bytes = OverlaySlab::requested_bytes(slots, bodies).unwrap();
            let budget = OwnedBudget::new(bytes);
            let mut overlay = OverlaySlab::new(slots, bodies, &budget).unwrap();
            overlay.put([1; 32], 1, b"abc").unwrap();
            assert_eq!(take_first_overlay_refusal_for_testonly(), None);
            let before = budget.snapshot();
            let error = overlay.put(digest, 2, body).unwrap_err();
            assert_eq!(error.code(), LIMIT);
            assert_eq!(error.detail(), "overlay 槽数或正文 arena 已满，复制前拒绝");
            assert_eq!(overlay.original(&[1; 32]), Some((1, &b"abc"[..])));
            assert_eq!(overlay.original(&[2; 32]), None);
            assert_eq!((overlay.slab.used, overlay.slab.body_used), (1, 3));
            assert_eq!(overlay.peak_used(), (1, 3));
            assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
            assert_eq!(
                budget.snapshot().copied_body_bytes,
                before.copied_body_bytes
            );
            assert_eq!(
                budget.snapshot().live_requested_layout_bytes,
                before.live_requested_layout_bytes
            );
            assert_eq!(
                budget.snapshot().allocation_attempts,
                before.allocation_attempts
            );
            assert_eq!(
                budget.snapshot().allocated_layouts,
                before.allocated_layouts
            );
            assert_eq!(budget.snapshot().rejected_layouts, before.rejected_layouts);
            assert_eq!(take_first_overlay_refusal_for_testonly(), Some(expected));
            assert_eq!(take_first_overlay_refusal_for_testonly(), None);
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn first_refusal_is_retained_until_taken_or_explicitly_reset() {
        reset_first_overlay_refusal_for_testonly();
        let budget = OwnedBudget::new(OverlaySlab::requested_bytes(1, 3).unwrap());
        let mut overlay = OverlaySlab::new(1, 3, &budget).unwrap();
        overlay.put([1; 32], 1, b"abc").unwrap();
        overlay.put([2; 32], 2, b"").unwrap_err();
        overlay.put([1; 32], 1, b"abc").unwrap();
        overlay.put([1; 32], 2, b"abcd").unwrap_err();
        assert_eq!(
            take_first_overlay_refusal_for_testonly(),
            Some(TestOnlyOverlayRefusal {
                used_slots: 1,
                needed_slots: Some(2),
                slot_capacity: 1,
                used_body_bytes: 3,
                needed_body_bytes: Some(3),
                body_capacity: 3,
                reason: TestOnlyOverlayRefusalReason::Slots,
            })
        );
        overlay.put([1; 32], 2, b"abcd").unwrap_err();
        reset_first_overlay_refusal_for_testonly();
        assert_eq!(take_first_overlay_refusal_for_testonly(), None);
        overlay.put([1; 32], 2, b"abcd").unwrap_err();
        assert_eq!(
            take_first_overlay_refusal_for_testonly().unwrap().reason,
            TestOnlyOverlayRefusalReason::Body
        );
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn arithmetic_overflow_keeps_the_original_error_and_no_copy() {
        reset_first_overlay_refusal_for_testonly();
        let budget = OwnedBudget::new(0);
        let mut overlay = OverlaySlab::new(0, 0, &budget).unwrap();
        // Synthetic internal counter exercises the arithmetic refusal without a
        // huge allocation or constructing an invalid body slice.
        overlay.slab.body_used = usize::MAX;
        let before = budget.snapshot();
        let error = overlay.put([1; 32], 1, b"x").unwrap_err();
        assert_eq!(error.code(), LIMIT);
        assert_eq!(error.detail(), "capture requested Layout 超过显式测试额度");
        assert_eq!(overlay.original(&[1; 32]), None);
        assert_eq!((overlay.slab.used, overlay.slab.body_used), (0, usize::MAX));
        assert_eq!(overlay.peak_used(), (0, 0));
        assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
        assert_eq!(
            budget.snapshot().copied_body_bytes,
            before.copied_body_bytes
        );
        assert_eq!(
            budget.snapshot().allocation_attempts,
            before.allocation_attempts
        );
        assert_eq!(
            take_first_overlay_refusal_for_testonly(),
            Some(TestOnlyOverlayRefusal {
                used_slots: 0,
                needed_slots: Some(1),
                slot_capacity: 0,
                used_body_bytes: usize::MAX,
                needed_body_bytes: None,
                body_capacity: 0,
                reason: TestOnlyOverlayRefusalReason::Overflow,
            })
        );
    }

    #[test]
    fn overlay_retains_old_layout_and_rejects_slots_as_bytes() {
        for slots in [0, 1, 2, 104_465] {
            assert_eq!(
                OverlaySlab::requested_bytes(slots, 3).unwrap(),
                Layout::array::<OverlayRecord>(slots).unwrap().size() + 3
            );
        }
        #[cfg(target_pointer_width = "64")]
        assert_eq!(OverlaySlab::requested_bytes(104_465, 0).unwrap(), 5_850_040);
        assert!(OverlaySlab::new(1, 0, &OwnedBudget::new(1)).is_err());
        assert!(OverlaySlab::requested_bytes(usize::MAX, 1).is_err());
        let budget = OwnedBudget::new(0);
        let empty = OverlaySlab::new(0, 0, &budget).unwrap();
        assert_eq!(empty.original(&[0; 32]), None);
        assert_eq!(budget.snapshot().allocated_layouts, 0);
    }

    #[test]
    fn descending_insert_and_replace_keep_explicit_lengths() {
        let bytes = OverlaySlab::requested_bytes(2, 4).unwrap();
        let budget = OwnedBudget::new(bytes);
        let mut overlay = OverlaySlab::new(2, 4, &budget).unwrap();
        overlay.put([2; 32], 2, b"abc").unwrap();
        overlay.put([1; 32], 1, b"x").unwrap();
        assert_eq!(overlay.slab.records()[0].offset, 3);
        assert_eq!(overlay.slab.records()[1].offset, 0);
        assert_eq!(overlay.original(&[1; 32]), Some((1, &b"x"[..])));
        assert_eq!(overlay.original(&[2; 32]), Some((2, &b"abc"[..])));
        let before = budget.snapshot();
        overlay.put([2; 32], 2, b"abc").unwrap();
        assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
        assert_eq!(
            budget.snapshot().copied_body_bytes,
            before.copied_body_bytes
        );
        overlay.put([2; 32], 3, b"yz").unwrap();
        assert_eq!(overlay.original(&[1; 32]), Some((1, &b"x"[..])));
        assert_eq!(overlay.original(&[2; 32]), Some((3, &b"yz"[..])));
        overlay.put([1; 32], 4, b"").unwrap();
        assert_eq!(overlay.original(&[1; 32]), Some((4, &b""[..])));
        assert_eq!(overlay.original(&[2; 32]), Some((3, &b"yz"[..])));
        let before = budget.snapshot();
        assert_eq!(overlay.put([3; 32], 5, b"").unwrap_err().code(), LIMIT);
        assert_eq!(overlay.put([2; 32], 5, b"12345").unwrap_err().code(), LIMIT);
        assert_eq!(overlay.original(&[1; 32]), Some((4, &b""[..])));
        assert_eq!(overlay.original(&[2; 32]), Some((3, &b"yz"[..])));
        assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
        assert_eq!(
            budget.snapshot().copied_body_bytes,
            before.copied_body_bytes
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes);
        drop(overlay);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn clear_keeps_capacity_charged_and_reuse_preserves_peaks() {
        let bytes = OverlaySlab::requested_bytes(2, 4).unwrap();
        let budget = OwnedBudget::new(bytes);
        let mut overlay = OverlaySlab::new(2, 4, &budget).unwrap();
        overlay.put([2; 32], 2, b"abc").unwrap();
        overlay.put([1; 32], 1, b"x").unwrap();
        assert_eq!(overlay.peak_used(), (2, 4));
        let before = budget.snapshot();
        overlay.clear();
        assert_eq!(overlay.original(&[1; 32]), None);
        assert_eq!(overlay.original(&[2; 32]), None);
        assert_eq!((overlay.slab.used, overlay.slab.body_used), (0, 0));
        assert_eq!(overlay.peak_used(), (2, 4));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, bytes);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts
        );
        assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
        overlay.put([0; 32], 0, b"new").unwrap();
        assert_eq!(overlay.original(&[0; 32]), Some((0, &b"new"[..])));
        assert_eq!(overlay.original(&[1; 32]), None);
        assert_eq!(overlay.peak_used(), (2, 4));
        drop(overlay);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(budget.snapshot().released_layouts, 2);
    }

    #[test]
    fn capture_clone_and_borrowed_bodies_are_isolated_from_overlay() {
        let capture_bytes = CaptureSlab::requested_bytes(1, 3).unwrap();
        let overlay_bytes = OverlaySlab::requested_bytes(2, 4).unwrap();
        let budget = OwnedBudget::new(2 * capture_bytes + overlay_bytes);
        let mut capture = CaptureSlab::new(1, 3, &budget).unwrap();
        capture.push([2; 32], 2, b"abc").unwrap();
        let capture = capture.finish().unwrap();
        let clone = capture.try_clone().unwrap();
        let borrowed = capture.original(&[2; 32]).unwrap().1;
        let clone_borrowed = clone.original(&[2; 32]).unwrap().1;
        let mut overlay = OverlaySlab::new(2, 4, &budget).unwrap();
        overlay.put([2; 32], 3, b"xyz").unwrap();
        overlay.put([1; 32], 1, b"x").unwrap();
        overlay.put([2; 32], 4, b"yz").unwrap();
        overlay.clear();
        assert_eq!(borrowed, b"abc");
        assert_eq!(clone_borrowed, b"abc");
        assert_eq!(capture.original(&[2; 32]), Some((2, &b"abc"[..])));
        assert_eq!(clone.original(&[2; 32]), Some((2, &b"abc"[..])));
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            2 * capture_bytes + overlay_bytes
        );
        assert_eq!(
            budget.snapshot().peak_requested_layout_bytes,
            2 * capture_bytes + overlay_bytes
        );
        drop(overlay);
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            2 * capture_bytes
        );
        drop(clone);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
