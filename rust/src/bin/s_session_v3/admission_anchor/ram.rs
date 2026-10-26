//! #1527 B/H1：单个准入 operation 的同锁双帽，不借用磁盘或请求额度。
use super::resources::NativeRamLimits;
use newchan_rust::s_content::capture_allocation::scratch::{
    CodecWork, ScratchCharge, ScratchFailure, ScratchParent,
};
use std::{
    alloc::Layout,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

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
    Layout::new::<[AtomicUsize; 2]>()
        .extend(Layout::new::<T>())
        .map(|(layout, _)| layout.pad_to_align())
        .map_err(|_| ScratchFailure::Ledger)
}
impl AdmissionRamLedger {
    pub(super) fn metadata_bytes() -> Result<usize, ScratchFailure> {
        arc_layout::<Mutex<LedgerState>>()?
            .size()
            .checked_add(arc_layout::<OperationParent>()?.size())
            .ok_or(ScratchFailure::Ledger)
    }
    pub(super) fn new(limits: NativeRamLimits) -> Result<Self, ScratchFailure> {
        Self::new_with_fixed(limits, 0)
    }
    pub(super) fn new_with_fixed(
        limits: NativeRamLimits,
        other_fixed: usize,
    ) -> Result<Self, ScratchFailure> {
        let max_live =
            usize::try_from(limits.max_live_bytes).map_err(|_| ScratchFailure::Budget)?;
        let max_operation =
            usize::try_from(limits.max_operation_bytes).map_err(|_| ScratchFailure::Budget)?;
        if max_operation > max_live {
            return Err(ScratchFailure::Ledger);
        }
        // 在任何Arc/Mutex分配前检查合并固定Layout。固定元数据随最后parent自然释放。
        let fixed = Self::metadata_bytes()?
            .checked_add(other_fixed)
            .ok_or(ScratchFailure::Ledger)?;
        if fixed > max_live || fixed > max_operation {
            return Err(ScratchFailure::Budget);
        }
        let origin = NEXT_ORIGIN
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ScratchFailure::Ledger)?;
        let ledger = Arc::new(Mutex::new(LedgerState {
            snapshot: RamSnapshot {
                job_live: fixed,
                job_peak: fixed,
                operation_live: fixed,
                operation_peak: fixed,
                parts: [fixed, 0, 0],
                active: true,
                failed: None,
            },
            max_live,
            max_operation,
            origin,
        }));
        Ok(Self {
            parent: Arc::new(OperationParent { ledger }),
            origin,
        })
    }
    pub(super) fn snapshot(&self) -> Result<RamSnapshot, ScratchFailure> {
        self.parent
            .ledger
            .lock()
            .map(|s| s.snapshot)
            .map_err(|_| ScratchFailure::Ledger)
    }
    pub(super) fn codec_work(&self) -> Result<CodecWork, ScratchFailure> {
        let limit = self
            .parent
            .ledger
            .lock()
            .map_err(|_| ScratchFailure::Ledger)?
            .max_operation;
        // 内部scratch limit只是额外保护；所有类型均由OperationParent同锁执法两帽。
        CodecWork::new_in_origin(self.parent.clone(), limit, self.origin)
    }
    pub(super) fn close(&self) -> Result<RamSnapshot, ScratchFailure> {
        let mut state = self
            .parent
            .ledger
            .lock()
            .map_err(|_| ScratchFailure::Ledger)?;
        state.snapshot.active = false;
        Ok(state.snapshot)
    }
}
impl Drop for AdmissionRamLedger {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
fn index(kind: ScratchCharge) -> usize {
    match kind {
        ScratchCharge::Metadata => 0,
        ScratchCharge::Layout => 1,
        ScratchCharge::Native => 2,
    }
}
impl OperationParent {
    fn reserve_locked(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        expected: u64,
    ) -> Result<u64, ScratchFailure> {
        let mut s = self.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        if let Some(failure) = s.snapshot.failed {
            return Err(failure);
        }
        if s.origin != expected || !s.snapshot.active {
            return Err(ScratchFailure::Ledger);
        }
        let next = (|| {
            let job = s
                .snapshot
                .job_live
                .checked_add(bytes)
                .ok_or(ScratchFailure::Budget)?;
            let operation = s
                .snapshot
                .operation_live
                .checked_add(bytes)
                .ok_or(ScratchFailure::Budget)?;
            let part = s.snapshot.parts[index(kind)]
                .checked_add(bytes)
                .ok_or(ScratchFailure::Ledger)?;
            if job > s.max_live || operation > s.max_operation {
                return Err(ScratchFailure::Budget);
            }
            Ok((job, operation, part))
        })();
        match next {
            Ok((job, operation, part)) => {
                s.snapshot.job_live = job;
                s.snapshot.operation_live = operation;
                s.snapshot.parts[index(kind)] = part;
                s.snapshot.job_peak = s.snapshot.job_peak.max(job);
                s.snapshot.operation_peak = s.snapshot.operation_peak.max(operation);
                Ok(s.origin)
            }
            Err(error) => {
                s.snapshot.failed = Some(error);
                Err(error)
            }
        }
    }
}
impl ScratchParent for OperationParent {
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
        self.reserve_locked(bytes, kind, self.origin())
    }
    fn reserve_in_origin(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        expected: u64,
    ) -> Result<u64, ScratchFailure> {
        self.reserve_locked(bytes, kind, expected)
    }
    fn release(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        origin: u64,
    ) -> Result<(), ScratchFailure> {
        let mut s = self.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        let next = (|| {
            if s.origin != origin {
                return Err(ScratchFailure::Ledger);
            }
            Ok((
                s.snapshot
                    .job_live
                    .checked_sub(bytes)
                    .ok_or(ScratchFailure::Ledger)?,
                s.snapshot
                    .operation_live
                    .checked_sub(bytes)
                    .ok_or(ScratchFailure::Ledger)?,
                s.snapshot.parts[index(kind)]
                    .checked_sub(bytes)
                    .ok_or(ScratchFailure::Ledger)?,
            ))
        })();
        match next {
            Ok((job, operation, part)) => {
                s.snapshot.job_live = job;
                s.snapshot.operation_live = operation;
                s.snapshot.parts[index(kind)] = part;
                Ok(())
            }
            Err(error) => {
                s.snapshot.failed.get_or_insert(error);
                Err(error)
            }
        }
    }
    fn origin(&self) -> u64 {
        self.ledger.lock().map(|s| s.origin).unwrap_or(0)
    }
    fn health(&self) -> Result<(), ScratchFailure> {
        let s = self.ledger.lock().map_err(|_| ScratchFailure::Ledger)?;
        s.snapshot.failed.map_or(Ok(()), Err)
    }
    fn invalidate(&self) {
        if let Ok(mut s) = self.ledger.lock() {
            s.snapshot.failed.get_or_insert(ScratchFailure::Ledger);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits(extra: usize) -> NativeRamLimits {
        let total = (AdmissionRamLedger::metadata_bytes().unwrap() + extra) as u64;
        NativeRamLimits {
            max_live_bytes: total,
            max_operation_bytes: total,
        }
    }
    #[test]
    fn mixed_ram_all_charge_kinds_share_atomic_caps() {
        for kind in [
            ScratchCharge::Metadata,
            ScratchCharge::Layout,
            ScratchCharge::Native,
        ] {
            for operation_tight in [false, true] {
                let mut caps = limits(100);
                if operation_tight {
                    caps.max_live_bytes += 100;
                }
                let owner = AdmissionRamLedger::new(caps).unwrap();
                let origin = owner.parent.reserve(100, kind).unwrap();
                let before = owner.snapshot().unwrap();
                assert_eq!(owner.parent.reserve(1, kind), Err(ScratchFailure::Budget));
                let after = owner.snapshot().unwrap();
                assert_eq!(before.job_live, after.job_live);
                assert_eq!(before.operation_live, after.operation_live);
                assert_eq!(before.parts, after.parts);
                owner.parent.release(100, kind, origin).unwrap();
                assert_eq!(
                    owner.snapshot().unwrap().job_live,
                    AdmissionRamLedger::metadata_bytes().unwrap()
                );
            }
        }
    }
    #[test]
    fn mixed_ram_mixed_charge_origin_and_closed_retained_release() {
        let owner = AdmissionRamLedger::new(limits(4096)).unwrap();
        let baseline = owner.snapshot().unwrap().job_live;
        let retained = owner
            .codec_work()
            .unwrap()
            .run(|work| {
                let layout = work.bytes();
                let permit = work.reserve_native(17)?;
                drop(layout);
                Ok::<_, newchan_rust::s_content::CodecError>(permit.hold(String::from("retained")))
            })
            .unwrap();
        assert!(owner.snapshot().unwrap().job_live > baseline);
        owner.close().unwrap();
        assert!(owner.parent.health().is_ok(), "正常closed原件仍可读");
        assert_eq!(
            owner.parent.reserve(1, ScratchCharge::Native),
            Err(ScratchFailure::Ledger)
        );
        drop(retained);
        assert_eq!(owner.snapshot().unwrap().job_live, baseline);
        assert_eq!(
            owner
                .parent
                .reserve_in_origin(0, ScratchCharge::Native, owner.origin + 1),
            Err(ScratchFailure::Ledger)
        );
        assert!(owner.snapshot().unwrap().job_peak > baseline);
    }
    #[test]
    fn mixed_ram_fixed_metadata_must_fit_before_parent_exists() {
        let n = AdmissionRamLedger::metadata_bytes().unwrap() as u64;
        assert!(matches!(
            AdmissionRamLedger::new(NativeRamLimits {
                max_live_bytes: n,
                max_operation_bytes: n - 1
            }),
            Err(ScratchFailure::Budget)
        ));
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
        use newchan_rust::s_content::{
            self as sc,
            job::{CheckedNodeUsage, Existing, Job, NodeAcceptance},
        };
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut job = Job::new_started_mixed(
                    job_limits(),
                    Existing {
                        store_bytes: 0,
                        staged_bytes: 0,
                    },
                    Box::new(std::time::Instant::now),
                    std::time::Instant::now(),
                    work,
                )?;
                let bytes = b"content";
                let content = CheckedNodeUsage::checked(
                    &sc::frame(sc::tag::BYTES_CHUNK, bytes),
                    sc::tag::BYTES_CHUNK,
                    bytes,
                )?;
                let legacy = sc::legacy_evidence::CheckedLegacyEvidenceRow::verify(
                    &sc::legacy_evidence::node_id(0, bytes),
                    0,
                    bytes,
                    bytes.len() as i64,
                )
                .unwrap();
                assert_eq!(
                    job.accept_checked_usage_mixed(&content, work)?,
                    NodeAcceptance::FirstAccept
                );
                assert_eq!(
                    job.accept_legacy_evidence(&legacy, work)?,
                    NodeAcceptance::FirstAccept
                );
                let before = job.snapshot();
                assert_eq!(before.nodes, 2);
                assert_eq!(
                    job.accept_checked_usage_mixed(&content, work)?,
                    NodeAcceptance::AlreadyAccepted
                );
                assert_eq!(
                    job.accept_legacy_evidence(&legacy, work)?,
                    NodeAcceptance::AlreadyAccepted
                );
                assert_eq!(job.snapshot().nodes, before.nodes);
                assert_eq!(job.snapshot().node_bytes, before.node_bytes);
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(
            owner.snapshot().unwrap().job_live,
            AdmissionRamLedger::metadata_bytes().unwrap()
        );
    }
    #[test]
    fn mixed_ram_dedup_rejection_preserves_node_counts_and_wire() {
        use newchan_rust::s_content::{
            self as sc,
            job::{CheckedNodeUsage, Existing, Job},
        };
        for legacy in [false, true] {
            let owner = AdmissionRamLedger::new(limits(CodecWork::metadata_bytes() + 1)).unwrap();
            let mut observed = None;
            let result = owner.codec_work().unwrap().run(|work| {
                let mut job = Job::new_started_mixed(
                    job_limits(),
                    Existing {
                        store_bytes: 0,
                        staged_bytes: 0,
                    },
                    Box::new(std::time::Instant::now),
                    std::time::Instant::now(),
                    work,
                )?;
                job.charge_wire_bytes(23)?;
                let bytes = b"x";
                let accepted = if legacy {
                    let row = sc::legacy_evidence::CheckedLegacyEvidenceRow::verify(
                        &sc::legacy_evidence::node_id(0, bytes),
                        0,
                        bytes,
                        1,
                    )
                    .unwrap();
                    job.accept_legacy_evidence(&row, work)
                } else {
                    let usage = CheckedNodeUsage::checked(
                        &sc::frame(sc::tag::BYTES_CHUNK, bytes),
                        sc::tag::BYTES_CHUNK,
                        bytes,
                    )?;
                    job.accept_checked_usage_mixed(&usage, work)
                };
                assert!(accepted.is_err());
                observed = Some(job.snapshot());
                accepted.map(|_| ())
            });
            assert!(result.is_err());
            let observed = observed.unwrap();
            assert_eq!(
                (observed.nodes, observed.node_bytes, observed.wire_bytes),
                (0, 0, 23)
            );
        }
    }
    #[test]
    fn mixed_ram_parsers_run_without_native_feature_and_keep_original_grammar() {
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner
            .codec_work()
            .unwrap()
            .run(|work| {
                let duplicate = br#"{"key":1,"key":2}"#;
                assert!(
                    crate::v3::business_json::mixed_strict_json_owner(duplicate, work).is_err()
                );
                let legacy = crate::v3::business_json::mixed_legacy_json_owner(duplicate, work)?;
                assert_eq!(legacy.get("key").unwrap(), &serde_json::json!(2));
                let strict =
                    crate::v3::business_json::mixed_strict_json_owner(br#"{"ok":[1,2]}"#, work)?;
                assert_eq!(strict.get("ok").unwrap(), &serde_json::json!([1, 2]));
                Ok::<_, crate::v3::business_json::MixedJsonError>(())
            })
            .unwrap();
        assert_eq!(
            owner.snapshot().unwrap().job_live,
            AdmissionRamLedger::metadata_bytes().unwrap()
        );
    }

    #[test]
    fn mixed_ram_parse_refusal_precedes_owned_json_and_shape_kernel_stays_original() {
        let owner = AdmissionRamLedger::new(limits(CodecWork::metadata_bytes() + 1)).unwrap();
        let result = owner.codec_work().unwrap().run(|work| {
            crate::v3::business_json::mixed_legacy_json_owner(
                br#"{"many":[1,2,3],"broken":}"#,
                work,
            )
        });
        assert!(matches!(
            result,
            Err(
                newchan_rust::s_content::capture_allocation::scratch::WorkError::Resource(
                    ScratchFailure::Budget
                )
            )
        ));
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner
            .codec_work()
            .unwrap()
            .run(|work| {
                for text in ["not json", "[]", r#"{"key":1,"key":2}"#] {
                    let original =
                        crate::json_shape_with_owner(text, crate::JsonShape::Object, None);
                    let mixed = crate::json_shape_mixed(text, crate::JsonShape::Object, work);
                    match (original, mixed) {
                        (Ok(crate::v3::business_json::JsonOwner::Plain(original)), Ok(mixed)) => {
                            assert_eq!(&*original, &*mixed)
                        }
                        (
                            Err(original),
                            Err(crate::v3::business_json::MixedJsonError::Business(mixed)),
                        ) => assert_eq!(original, mixed),
                        _ => panic!("mixed改变原legacy形状核"),
                    }
                }
                Ok::<_, crate::v3::business_json::MixedJsonError>(())
            })
            .unwrap();
    }
    #[test]
    fn mixed_ram_grant_tree_is_metered_and_failed_growth_keeps_disk_accounting() {
        use newchan_rust::s_content::{
            self as sc,
            job::{Existing, Job},
        };
        for enough in [false, true] {
            let extra = if enough {
                1 << 20
            } else {
                CodecWork::metadata_bytes() + 1
            };
            let owner = AdmissionRamLedger::new(limits(extra)).unwrap();
            let baseline = owner.snapshot().unwrap().job_live;
            let result = owner.codec_work().unwrap().run(|work| {
                let mut job = Job::new_started_mixed(
                    job_limits(),
                    Existing {
                        store_bytes: 0,
                        staged_bytes: 0,
                    },
                    Box::new(std::time::Instant::now),
                    std::time::Instant::now(),
                    work,
                )?;
                job.charge_wire_bytes(19)?;
                assert!(
                    job.reserve_staged(10).is_err(),
                    "普通入口不能使mixed预留免账"
                );
                assert_eq!(job.snapshot().open_reservations, 0);
                let before = job.snapshot();
                let grant = job.reserve_staged_mixed(10, work);
                if enough {
                    let grant = grant?;
                    assert_eq!(job.snapshot().staged.pending, 10);
                    let stored = job.reserve_store_mixed(7, work)?;
                    assert_eq!(job.snapshot().store.pending, 7);
                    assert_eq!(job.snapshot().open_reservations, 2);
                    job.reconcile(&grant, 6)?;
                    job.release(&stored)?;
                    assert_eq!(job.snapshot().open_reservations, 0);
                    assert_eq!(job.snapshot().staged.pending, 0);
                    assert_eq!(job.snapshot().store.pending, 0);
                    job.finish_check()?;
                } else {
                    assert!(grant.is_err());
                    let after = job.snapshot();
                    assert_eq!(after.staged, before.staged);
                    assert_eq!(after.store, before.store);
                    assert_eq!(after.open_reservations, 0);
                    assert_eq!(after.wire_bytes, 19);
                }
                Ok::<_, sc::CodecError>(())
            });
            assert_eq!(result.is_ok(), enough);
            assert_eq!(owner.snapshot().unwrap().job_live, baseline);
        }
    }
    #[test]
    fn mixed_ram_rejected_parse_does_not_invoke_parser() {
        use crate::v3::business_json::{mixed_parse_admitted, MixedJsonError};
        let owner = AdmissionRamLedger::new(limits(CodecWork::metadata_bytes() + 1)).unwrap();
        let mut entered = false;
        let result = owner.codec_work().unwrap().run(|work| {
            let result = mixed_parse_admitted(br#"{"large":["value"]}"#, work, || {
                entered = true;
                Err(MixedJsonError::Business("parser invoked".into()))
            });
            match &result {
                Err(MixedJsonError::Resource(error)) => {
                    assert_eq!(error.scratch_failure(), Some(ScratchFailure::Budget))
                }
                _ => panic!("拒绝未保留结构化资源错误"),
            }
            result
        });
        assert!(!entered);
        assert!(result.is_err());
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn mixed_ram_parser_refusal_allocates_no_error_string() {
        use crate::v3::{
            audit_owner::allocation_probe::Window,
            business_json::{mixed_legacy_json_owner, mixed_strict_json_owner, MixedJsonError},
        };
        const TEXT: &str = r#"{"key":["a string that must allocate","another allocated string"]}"#;
        for route in 0..3 {
            for enough in [false, true] {
                let extra = if enough {
                    1 << 20
                } else {
                    CodecWork::metadata_bytes() + 1
                };
                let owner = AdmissionRamLedger::new(limits(extra)).unwrap();
                let result = owner.codec_work().unwrap().run(|work| {
                    let window = Window::start();
                    let parsed = match route {
                        0 => mixed_strict_json_owner(TEXT.as_bytes(), work),
                        1 => mixed_legacy_json_owner(TEXT.as_bytes(), work),
                        _ => crate::json_shape_mixed(TEXT, crate::JsonShape::Object, work),
                    };
                    let observed = window.finish();
                    let bytes = observed["allocated_bytes"].as_u64().unwrap();
                    if enough {
                        assert!(parsed.is_ok());
                        assert!(bytes > 0, "同一路径的成功正控确实经过实际分配器");
                    } else {
                        assert!(matches!(&parsed, Err(MixedJsonError::Resource(error))
                            if error.scratch_failure() == Some(ScratchFailure::Budget)));
                        assert_eq!(bytes, 0, "资源拒绝到跨层返回之间不得分配错误String或JSON");
                    }
                    parsed.map(|_| ())
                });
                assert_eq!(result.is_ok(), enough);
            }
        }
    }
    #[test]
    fn mixed_ram_sql_raw_projection_preserves_values_and_type_error_order() {
        use crate::v3::business_json::scoped_legacy::{project_raw_history_row, MixedSqlError};
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        for head in ["'id'", "42", "CAST(X'ff' AS TEXT)"] {
            for optional in ["NULL", "-9223372036854775808"] {
                let sql = format!("SELECT {head}, 9223372036854775807, 'input', 'hash', 'receipt', 7, 'ns', 'epoch', 'btc', 'event', 'time', 'raw', '12.3', '123', '0.2', 'coord', {optional}");
                let mut stmt = conn.prepare(&sql).unwrap();
                let mut rows = stmt.query([]).unwrap();
                let row = rows.next().unwrap().unwrap();
                let original = crate::project_raw_history_row(row);
                let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
                owner
                    .codec_work()
                    .unwrap()
                    .run(|work| {
                        match (original, project_raw_history_row(row, work)) {
                            (Ok(original), Ok(mixed)) => assert_eq!(original, *mixed),
                            (Err(original), Err(MixedSqlError::Sql(mixed))) => {
                                assert_eq!(original, mixed)
                            }
                            other => panic!("SQL投影结果不一致: {other:?}"),
                        }
                        Ok::<_, newchan_rust::s_content::CodecError>(())
                    })
                    .unwrap();
                assert_eq!(
                    owner.snapshot().unwrap().job_live,
                    AdmissionRamLedger::metadata_bytes().unwrap()
                );
            }
        }
    }

    #[test]
    fn mixed_ram_projection_replacement_keeps_old_owner_until_new_permitted() {
        use crate::v3::business_json::scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
        use crate::v3::business_json::MixedJsonError;
        let source = serde_json::json!({"lifecycle":"active","unknown":{"preserved":[1,2,3]}});
        let next = serde_json::json!("withdrawn");
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut record = ScopedLegacyRecord::clone_projection(
                    ProjectionKind::AtPreviousLifecycle,
                    &source,
                    work,
                )?;
                record.replace_known_field(Field::Lifecycle, &next)?;
                let result = record.finish();
                assert_eq!(result["lifecycle"], next);
                assert_eq!(result["unknown"], source["unknown"]);
                assert_eq!(source["lifecycle"], "active");
                Ok::<_, MixedJsonError>(())
            })
            .unwrap();
        assert_eq!(
            owner.snapshot().unwrap().job_live,
            AdmissionRamLedger::metadata_bytes().unwrap()
        );
    }

    #[test]
    fn mixed_ram_sql_object_projection_keeps_lifecycle_before_json_and_optional_reads() {
        use crate::v3::business_json::scoped_legacy::{project_object_row, MixedSqlError};
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let good = r#"[{"merged_index":1,"raw_refs":[{"seq":2,"revision":3,"extra":true}],"untouched":"yes"}]"#;
        for (wg, comparisons, refs, as_of) in [
            (Some(1), "{bad", "[]", None),
            (Some(3), "[]", good, Some(2)),
            (Some(3), "[]", good, None),
            (None, "[]", good, None),
            (None, "{bad", "[]", None),
            (None, "[]", "{}", None),
            (
                None,
                "[]",
                r#"[{"merged_index":"bad","raw_refs":[]}]"#,
                None,
            ),
        ] {
            let mut stmt = conn
                .prepare(
                    "SELECT 'id',2,'k','b','up','up','down',1,2,3,?2,?3,'[]',1,'cut',2,?1,99,88",
                )
                .unwrap();
            let mut rows = stmt
                .query(rusqlite::params![wg, comparisons, refs])
                .unwrap();
            let row = rows.next().unwrap().unwrap();
            let original = crate::project_object_row(row, as_of);
            let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
            owner
                .codec_work()
                .unwrap()
                .run(|work| {
                    match (original, project_object_row(row, as_of, work)) {
                        (Ok(original), Ok(mixed)) => assert_eq!(original, *mixed),
                        (Err(original), Err(MixedSqlError::Sql(mixed))) => {
                            assert_eq!(
                                std::mem::discriminant(&original),
                                std::mem::discriminant(&mixed)
                            );
                            assert_eq!(original.to_string(), mixed.to_string());
                        }
                        other => panic!("对象投影或首错漂移：{other:?}"),
                    }
                    Ok::<_, newchan_rust::s_content::CodecError>(())
                })
                .unwrap();
            assert_eq!(
                owner.snapshot().unwrap().job_live,
                AdmissionRamLedger::metadata_bytes().unwrap()
            );
        }
    }

    #[test]
    fn mixed_ram_fact_collect_defers_json_and_project_preserves_first_error() {
        use crate::v3::business_json::scoped_legacy::{collect_fact_row, project_fact};
        use crate::v3::business_json::MixedJsonError;
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        for (kind, payload, refs, death) in [
            ("CC-004.inclusion_step", "{}", "[]", None),
            ("CC-004.inclusion_step", "{}", "[]", Some(3_i64)),
            ("unknown", "bad", "bad", None),
            ("CC-004.inclusion_step", "", "", None),
            ("CC-004.inclusion_step", "bad", "[]", None),
            ("CC-004.inclusion_step", "{}", "{}", None),
        ] {
            let mut stmt = conn
                .prepare("SELECT 'id',1,?1,'batch','key',?2,?3,'[]',1,'cut',2,?4,NULL,NULL")
                .unwrap();
            let mut rows = stmt
                .query(rusqlite::params![kind, payload, refs, death])
                .unwrap();
            let row = rows.next().unwrap().unwrap();
            let original = crate::tb02a::collect_fact_row(row).unwrap();
            let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
            owner
                .codec_work()
                .unwrap()
                .run(|work| {
                    let collected = collect_fact_row(row, work).unwrap();
                    assert_eq!(*collected, original); // graph empty placeholders and bad JSON remain raw text
                    let expected = crate::tb02a::project(&original);
                    match (expected, project_fact(&collected, work)) {
                        (Ok(a), Ok(b)) => assert_eq!(a, *b),
                        (Err(a), Err(MixedJsonError::Business(b))) => assert_eq!(a, b),
                        other => panic!("fact projection mismatch {other:?}"),
                    }
                    Ok::<_, newchan_rust::s_content::CodecError>(())
                })
                .unwrap();
            assert_eq!(
                owner.snapshot().unwrap().job_live,
                AdmissionRamLedger::metadata_bytes().unwrap()
            );
        }
    }

    #[test]
    fn mixed_ram_fact_asof_filter_never_parses_excluded_json() {
        use crate::v3::business_json::scoped_legacy::{filter_fact_lifecycle, project_fact};
        use crate::v3::business_json::{MixedJsonError, PlainJson};
        for (first, death, asof, skip, clear, error) in [
            (3, None, Some(2), true, false, false),
            (1, Some(4), Some(2), false, true, false),
            (3, Some(2), Some(1), false, false, true),
        ] {
            let source = serde_json::json!({"first_known_generation":first,"withdrawn_generation":death,
                "withdrawal_reason":"old","superseded_by":"next","payload_json":"bad"});
            let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
            owner.codec_work().unwrap().run(|work| {
                let row = PlainJson::try_clone_from(&source,Some(work))?;
                let filtered = filter_fact_lifecycle(row,asof,work);
                if error {
                    assert!(matches!(filtered,Err(MixedJsonError::Business(ref s)) if s == "StorageUnavailable：结构事实生命周期非法"));
                } else if skip { assert!(filtered.unwrap().is_none()); }
                else {
                    let kept = filtered.unwrap().unwrap();
                    assert_eq!(kept["payload_json"],"bad");
                    if clear {
                        for key in ["withdrawn_generation","withdrawal_reason","superseded_by"] { assert!(kept[key].is_null()); }
                    }
                    assert!(matches!(project_fact(&kept,work),Err(MixedJsonError::Business(ref s)) if s == "StorageUnavailable：事实kind缺失"));
                }
                Ok::<_,newchan_rust::s_content::CodecError>(())
            }).unwrap();
            assert_eq!(
                owner.snapshot().unwrap().job_live,
                AdmissionRamLedger::metadata_bytes().unwrap()
            );
        }
    }
    #[test]
    fn mixed_ram_cache_same_parent_shares_closed_parent_copies_failed_parent_rejects() {
        use crate::v3::native_shared::NativeArc;
        use newchan_rust::s_content::capture_allocation::native_owned::NativeValue;
        let old = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        let cached = old.codec_work().unwrap().run(|work| {
            let text = NativeValue::try_from_str("cached pure content",Some(work))?;
            let cached = NativeArc::try_new_shallow(text,Some(work))?;
            let shared = cached.share_or_copy_mixed(work,|_,_| panic!("same parent must not copy"))?;
            assert!(NativeArc::ptr_eq(&cached,&shared));
            Ok::<_,newchan_rust::s_content::CodecError>(cached)
        }).unwrap();
        old.close().unwrap();
        let current = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        let copy = current.codec_work().unwrap().run(|work| {
            cached.share_or_copy_mixed(work,|text,scope| text.try_clone_admitted(scope))
        }).unwrap();
        assert!(!NativeArc::ptr_eq(&cached,&copy));
        assert_eq!(cached.as_ref(),copy.as_ref());
        assert!(old.snapshot().unwrap().job_live > AdmissionRamLedger::metadata_bytes().unwrap());
        assert!(current.snapshot().unwrap().job_live > AdmissionRamLedger::metadata_bytes().unwrap());
        drop(copy);
        assert_eq!(current.snapshot().unwrap().job_live,AdmissionRamLedger::metadata_bytes().unwrap());
        drop(cached);
        assert_eq!(old.snapshot().unwrap().job_live,AdmissionRamLedger::metadata_bytes().unwrap());

        let failed = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        let retained = failed.codec_work().unwrap().run(|work| NativeArc::try_new_shallow(17_u64,Some(work))).unwrap();
        failed.parent.invalidate();
        let target = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        target.codec_work().unwrap().run(|work| {
            let before = target.snapshot().unwrap().job_live;
            assert!(retained.share_or_copy_mixed(work,|_,_| panic!("failed parent must not copy")).is_err());
            assert_eq!(target.snapshot().unwrap().job_live,before);
            Ok::<_,newchan_rust::s_content::CodecError>(())
        }).unwrap();
    }

    #[test]
    fn mixed_ram_sql_node_acceptance_is_fresh_and_rejects_corrupt_original() {
        use newchan_rust::s_content::{self as sc, job::{Job,Existing}};
        use crate::v3::audit_budget::LegacyObserver;
        use std::cell::RefCell;
        struct Observer<'a> { job: RefCell<Job>, work: &'a sc::capture_allocation::scratch::WorkScope }
        impl LegacyObserver for Observer<'_> {
            fn precheck(&self) -> Result<(),sc::CodecError> { self.job.borrow_mut().pre_io_check() }
            fn check_work(&self,work: &sc::capture_allocation::scratch::WorkScope) -> Result<(),sc::CodecError> {
                let token = self.work.reserve_native(0)?;
                if !token.belongs_to_parent(work) { return Err(work.reject(ScratchFailure::Ledger)); }
                Ok(())
            }
            fn accept_checked(&self,row: &sc::legacy_evidence::CheckedLegacyEvidenceRow) -> Result<(),sc::CodecError> {
                self.job.borrow_mut().accept_legacy_evidence(row,self.work).map(|_| ())
            }
        }
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::evidence_store::DDL).unwrap();
        let payload = b"abc";
        let id = sc::legacy_evidence::node_id(0,payload);
        conn.execute("INSERT INTO s_evidence_nodes VALUES(?1,0,?2,3)",rusqlite::params![&id[..],&payload[..]]).unwrap();
        let owner = AdmissionRamLedger::new(limits(1 << 20)).unwrap();
        owner.codec_work().unwrap().run(|work| {
            let job = Job::new_started_mixed(job_limits(),Existing {store_bytes:0,staged_bytes:0},
                Box::new(std::time::Instant::now),std::time::Instant::now(),work)?;
            let observer = Observer {job:RefCell::new(job),work};
            let first = crate::evidence_store::read_node_mixed(&conn,&id,work,&observer).unwrap();
            assert_eq!(first.payload(),payload);
            assert_eq!(first.checked().id(),&id);
            assert_eq!(observer.job.borrow().snapshot().nodes,1);
            let repeated = crate::evidence_store::read_node_mixed(&conn,&id,work,&observer).unwrap();
            assert_eq!(repeated.payload(),payload);
            assert_eq!(observer.job.borrow().snapshot().nodes,1);
            conn.execute("UPDATE s_evidence_nodes SET payload=?1",rusqlite::params![&b"bad"[..]]).unwrap();
            assert!(matches!(crate::evidence_store::read_node_mixed(&conn,&id,work,&observer),
                Err(crate::v3::business_json::MixedJsonError::Business(ref s)) if s.contains("同 ID 异内容")));
            assert_eq!(observer.job.borrow().snapshot().nodes,1);
            Ok::<_,sc::CodecError>(())
        }).unwrap();
    }

}
