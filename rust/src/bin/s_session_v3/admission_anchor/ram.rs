//! #1527 B/H1：单个准入 operation 的同锁双帽，不借用磁盘或请求额度。
use super::resources::NativeRamLimits;
use newchan_rust::s_content::capture_allocation::scratch::{
    CodecWork, ScratchCharge, ScratchFailure, ScratchParent,
};
use std::{alloc::Layout, sync::{Arc, Mutex, atomic::{AtomicU64, AtomicUsize, Ordering}}};

static NEXT_ORIGIN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RamSnapshot {
    pub job_live: usize,
    pub job_peak: usize,
    pub operation_live: usize,
    pub operation_peak: usize,
    pub parts: [usize; 3],
    pub active: bool,
    pub failed: Option<ScratchFailure>,
}
struct LedgerState {
    snapshot: RamSnapshot,
    max_live: usize,
    max_operation: usize,
    origin: u64,
}
struct OperationParent {
    ledger: Arc<Mutex<LedgerState>>,
}
/// 唯一operation owner；许可只持facade，不反持owner。析构禁新分配但保留真实载荷账。
pub(super) struct AdmissionRamLedger {
    parent: Arc<OperationParent>,
    origin: u64,
}
fn arc_layout<T>() -> Result<Layout, ScratchFailure> {
    Layout::new::<[AtomicUsize; 2]>().extend(Layout::new::<T>())
        .map(|(layout, _)| layout.pad_to_align()).map_err(|_| ScratchFailure::Ledger)
}
impl AdmissionRamLedger {
    pub(super) fn metadata_bytes() -> Result<usize, ScratchFailure> {
        arc_layout::<Mutex<LedgerState>>()?.size()
            .checked_add(arc_layout::<OperationParent>()?.size()).ok_or(ScratchFailure::Ledger)
    }
    pub(super) fn new(limits: NativeRamLimits) -> Result<Self, ScratchFailure> {
        let max_live = usize::try_from(limits.max_live_bytes).map_err(|_| ScratchFailure::Budget)?;
        let max_operation = usize::try_from(limits.max_operation_bytes).map_err(|_| ScratchFailure::Budget)?;
        if max_operation > max_live { return Err(ScratchFailure::Ledger); }
        // 在任何Arc/Mutex分配前检查合并固定Layout。固定元数据随最后parent自然释放。
        let fixed = Self::metadata_bytes()?;
        if fixed > max_live || fixed > max_operation { return Err(ScratchFailure::Budget); }
        let origin = NEXT_ORIGIN.fetch_update(Ordering::Relaxed, Ordering::Relaxed,
            |value| value.checked_add(1)).map_err(|_| ScratchFailure::Ledger)?;
        let ledger = Arc::new(Mutex::new(LedgerState {
            snapshot: RamSnapshot { job_live: fixed, job_peak: fixed,
                operation_live: fixed, operation_peak: fixed, parts: [fixed, 0, 0],
                active: true, failed: None }, max_live, max_operation, origin,
        }));
        Ok(Self { parent: Arc::new(OperationParent { ledger }), origin })
    }
    pub(super) fn snapshot(&self) -> Result<RamSnapshot, ScratchFailure> {
        self.parent.ledger.lock().map(|s| s.snapshot).map_err(|_| ScratchFailure::Ledger)
    }
    pub(super) fn codec_work(&self) -> Result<CodecWork, ScratchFailure> {
        let limit = self.parent.ledger.lock().map_err(|_| ScratchFailure::Ledger)?.max_operation;
        // 内部scratch limit只是额外保护；所有类型均由OperationParent同锁执法两帽。
        CodecWork::new_in_origin(self.parent.clone(), limit, self.origin)
    }
    pub(super) fn close(&self) -> Result<RamSnapshot, ScratchFailure> {
        let mut state = self.parent.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        state.snapshot.active = false;
        Ok(state.snapshot)
    }
}
impl Drop for AdmissionRamLedger { fn drop(&mut self) { let _ = self.close(); } }
fn index(kind: ScratchCharge) -> usize {
    match kind { ScratchCharge::Metadata => 0, ScratchCharge::Layout => 1, ScratchCharge::Native => 2 }
}
impl OperationParent {
    fn reserve_locked(&self, bytes: usize, kind: ScratchCharge, expected: u64) -> Result<u64, ScratchFailure> {
        let mut s = self.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        if let Some(failure) = s.snapshot.failed { return Err(failure); }
        if s.origin != expected || !s.snapshot.active { return Err(ScratchFailure::Ledger); }
        let next = (|| {
            let job = s.snapshot.job_live.checked_add(bytes).ok_or(ScratchFailure::Budget)?;
            let operation = s.snapshot.operation_live.checked_add(bytes).ok_or(ScratchFailure::Budget)?;
            let part = s.snapshot.parts[index(kind)].checked_add(bytes).ok_or(ScratchFailure::Ledger)?;
            if job > s.max_live || operation > s.max_operation { return Err(ScratchFailure::Budget); }
            Ok((job, operation, part))
        })();
        match next {
            Ok((job, operation, part)) => {
                s.snapshot.job_live = job; s.snapshot.operation_live = operation;
                s.snapshot.parts[index(kind)] = part;
                s.snapshot.job_peak = s.snapshot.job_peak.max(job);
                s.snapshot.operation_peak = s.snapshot.operation_peak.max(operation);
                Ok(s.origin)
            }
            Err(error) => { s.snapshot.failed = Some(error); Err(error) }
        }
    }
}
impl ScratchParent for OperationParent {
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
        self.reserve_locked(bytes, kind, self.origin())
    }
    fn reserve_in_origin(&self, bytes: usize, kind: ScratchCharge, expected: u64) -> Result<u64, ScratchFailure> {
        self.reserve_locked(bytes, kind, expected)
    }
    fn release(&self, bytes: usize, kind: ScratchCharge, origin: u64) -> Result<(), ScratchFailure> {
        let mut s = self.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        let next = (|| {
            if s.origin != origin { return Err(ScratchFailure::Ledger); }
            Ok((s.snapshot.job_live.checked_sub(bytes).ok_or(ScratchFailure::Ledger)?,
                s.snapshot.operation_live.checked_sub(bytes).ok_or(ScratchFailure::Ledger)?,
                s.snapshot.parts[index(kind)].checked_sub(bytes).ok_or(ScratchFailure::Ledger)?))
        })();
        match next {
            Ok((job, operation, part)) => { s.snapshot.job_live = job; s.snapshot.operation_live = operation;
                s.snapshot.parts[index(kind)] = part; Ok(()) }
            Err(error) => { s.snapshot.failed.get_or_insert(error); Err(error) }
        }
    }
    fn origin(&self) -> u64 { self.ledger.lock().map(|s| s.origin).unwrap_or(0) }
    fn health(&self) -> Result<(), ScratchFailure> {
        let s = self.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        s.snapshot.failed.map_or(Ok(()), Err)
    }
    fn invalidate(&self) {
        if let Ok(mut s) = self.ledger.lock() { s.snapshot.failed.get_or_insert(ScratchFailure::Ledger); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits(extra: usize) -> NativeRamLimits {
        let total = (AdmissionRamLedger::metadata_bytes().unwrap() + extra) as u64;
        NativeRamLimits { max_live_bytes: total, max_operation_bytes: total }
    }
    #[test]
    fn mixed_ram_all_charge_kinds_share_atomic_caps() {
        for kind in [ScratchCharge::Metadata, ScratchCharge::Layout, ScratchCharge::Native] {
            for operation_tight in [false, true] {
                let mut caps = limits(100);
                if operation_tight { caps.max_live_bytes += 100; }
                let owner = AdmissionRamLedger::new(caps).unwrap();
                let origin = owner.parent.reserve(100, kind).unwrap();
                let before = owner.snapshot().unwrap();
                assert_eq!(owner.parent.reserve(1, kind), Err(ScratchFailure::Budget));
                let after = owner.snapshot().unwrap();
                assert_eq!(before.job_live, after.job_live);
                assert_eq!(before.operation_live, after.operation_live);
                assert_eq!(before.parts, after.parts);
                owner.parent.release(100, kind, origin).unwrap();
                assert_eq!(owner.snapshot().unwrap().job_live, AdmissionRamLedger::metadata_bytes().unwrap());
            }
        }
    }
    #[test]
    fn mixed_ram_mixed_charge_origin_and_closed_retained_release() {
        let owner = AdmissionRamLedger::new(limits(4096)).unwrap();
        let baseline = owner.snapshot().unwrap().job_live;
        let retained = owner.codec_work().unwrap().run(|work| {
            let layout = work.bytes();
            let permit = work.reserve_native(17)?;
            drop(layout);
            Ok::<_, newchan_rust::s_content::CodecError>(permit.hold(String::from("retained")))
        }).unwrap();
        assert!(owner.snapshot().unwrap().job_live > baseline);
        owner.close().unwrap();
        assert!(owner.parent.health().is_ok(), "正常closed原件仍可读");
        assert_eq!(owner.parent.reserve(1, ScratchCharge::Native), Err(ScratchFailure::Ledger));
        drop(retained);
        assert_eq!(owner.snapshot().unwrap().job_live, baseline);
        assert_eq!(owner.parent.reserve_in_origin(0, ScratchCharge::Native, owner.origin + 1), Err(ScratchFailure::Ledger));
        assert!(owner.snapshot().unwrap().job_peak > baseline);
    }
    #[test]
    fn mixed_ram_fixed_metadata_must_fit_before_parent_exists() {
        let n = AdmissionRamLedger::metadata_bytes().unwrap() as u64;
        assert!(matches!(AdmissionRamLedger::new(NativeRamLimits {
            max_live_bytes: n, max_operation_bytes: n - 1 }), Err(ScratchFailure::Budget)));
    }
    fn job_limits() -> newchan_rust::s_content::job::JobLimits {
        newchan_rust::s_content::job::JobLimits::from_canonical_json(&serde_json::json!({
            "max_total_wire_bytes":"1000000", "max_total_node_bytes":"1000000", "max_total_nodes":"2",
            "max_total_records":"100", "max_total_requests":"100", "max_elapsed_ms":"60000",
            "max_staged_bytes":"1000000", "max_store_bytes":"1000000"
        })).unwrap()
    }
    #[test]
    fn mixed_ram_two_domain_job_acceptance_and_full_cap_duplicates() {
        use newchan_rust::s_content::{self as sc, job::{Job, Existing, CheckedNodeUsage, NodeAcceptance}};
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner.codec_work().unwrap().run(|work| {
            let mut job = Job::new_started_mixed(job_limits(), Existing { store_bytes: 0, staged_bytes: 0 },
                Box::new(std::time::Instant::now), std::time::Instant::now(), work)?;
            let bytes = b"content";
            let content = CheckedNodeUsage::checked(&sc::frame(sc::tag::BYTES_CHUNK, bytes), sc::tag::BYTES_CHUNK, bytes)?;
            let legacy = sc::legacy_evidence::CheckedLegacyEvidenceRow::verify(
                &sc::legacy_evidence::node_id(0, bytes), 0, bytes, bytes.len() as i64).unwrap();
            assert_eq!(job.accept_checked_usage_mixed(&content, work)?, NodeAcceptance::FirstAccept);
            assert_eq!(job.accept_legacy_evidence(&legacy, work)?, NodeAcceptance::FirstAccept);
            let before = job.snapshot();
            assert_eq!(before.nodes, 2);
            assert_eq!(job.accept_checked_usage_mixed(&content, work)?, NodeAcceptance::AlreadyAccepted);
            assert_eq!(job.accept_legacy_evidence(&legacy, work)?, NodeAcceptance::AlreadyAccepted);
            assert_eq!(job.snapshot().nodes, before.nodes);
            assert_eq!(job.snapshot().node_bytes, before.node_bytes);
            Ok::<_, sc::CodecError>(())
        }).unwrap();
        assert_eq!(owner.snapshot().unwrap().job_live, AdmissionRamLedger::metadata_bytes().unwrap());
    }
    #[test]
    fn mixed_ram_dedup_rejection_preserves_node_counts_and_wire() {
        use newchan_rust::s_content::{self as sc, job::{Job, Existing, CheckedNodeUsage}};
        for legacy in [false, true] {
            let owner = AdmissionRamLedger::new(limits(CodecWork::metadata_bytes() + 1)).unwrap();
            let mut observed = None;
            let result = owner.codec_work().unwrap().run(|work| {
                let mut job = Job::new_started_mixed(job_limits(), Existing { store_bytes: 0, staged_bytes: 0 },
                    Box::new(std::time::Instant::now), std::time::Instant::now(), work)?;
                job.charge_wire_bytes(23)?;
                let bytes = b"x";
                let accepted = if legacy {
                    let row = sc::legacy_evidence::CheckedLegacyEvidenceRow::verify(
                        &sc::legacy_evidence::node_id(0, bytes), 0, bytes, 1).unwrap();
                    job.accept_legacy_evidence(&row, work)
                } else {
                    let usage = CheckedNodeUsage::checked(&sc::frame(sc::tag::BYTES_CHUNK, bytes), sc::tag::BYTES_CHUNK, bytes)?;
                    job.accept_checked_usage_mixed(&usage, work)
                };
                assert!(accepted.is_err());
                observed = Some(job.snapshot());
                accepted.map(|_| ())
            });
            assert!(result.is_err());
            let observed = observed.unwrap();
            assert_eq!((observed.nodes, observed.node_bytes, observed.wire_bytes), (0, 0, 23));
        }
    }
    #[test]
    fn mixed_ram_parsers_run_without_native_feature_and_keep_original_grammar() {
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner.codec_work().unwrap().run(|work| {
            let duplicate = br#"{"key":1,"key":2}"#;
            assert!(crate::v3::business_json::mixed_strict_json_owner(duplicate, work).is_err());
            let legacy = crate::v3::business_json::mixed_legacy_json_owner(duplicate, work)?;
            assert_eq!(legacy.get("key").unwrap(), &serde_json::json!(2));
            let strict = crate::v3::business_json::mixed_strict_json_owner(br#"{"ok":[1,2]}"#, work)?;
            assert_eq!(strict.get("ok").unwrap(), &serde_json::json!([1,2]));
            Ok::<_, String>(())
        }).unwrap();
        assert_eq!(owner.snapshot().unwrap().job_live, AdmissionRamLedger::metadata_bytes().unwrap());
    }

}
