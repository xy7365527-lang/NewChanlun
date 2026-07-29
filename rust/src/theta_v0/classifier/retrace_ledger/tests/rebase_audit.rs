//! Audit 流（票 #622；#574 裁定六）：四类警报（残废拒收/死人挂号/倒退拒收/迟到吸收）落 append-only
//! 流、JSONL 正式持久化边界（对标 `CompletedFreezeReducer`）、且**篡改 audit 流不影响
//! `fold(journal) == book`**——审计归审计，不进真相恢复路径。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "newchan-retrace-audit-{name}-{}-{}.jsonl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

// ═══════════════════════════════════════════════════════════════════════════
// 四类事件落流
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn residual_rejection_lands_in_audit_log() {
    let mut book = ledger();
    let broken = CenterFrame {
        zd: ZG,
        zg: ZD,
        start_index: ANCHOR_START,
        end_index: 1_200,
    };
    assert!(book.observe(&up_input(broken, 3, None, 500)).is_err());

    assert_eq!(book.audit_log().len(), 1);
    let record = book.audit_log()[0];
    assert_eq!(record.sequence, 0);
    assert_eq!(
        record.event,
        RetraceAuditEvent::ResidualRejected {
            as_of: 500,
            code: RetraceRejectionCode::MalformedFrame,
        }
    );
}

#[test]
fn dead_center_rejection_lands_in_audit_log() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let certificate = book.death_certificate(center.anchor()).unwrap();

    let attempted = key_of(frame(1_400), 5);
    assert!(book.observe(&up_input(frame(1_400), 5, None, 700)).is_err());

    let record = book.audit_log().last().copied().unwrap();
    assert_eq!(
        record.event,
        RetraceAuditEvent::DeadCenterRejected {
            anchor: center.anchor(),
            attempted,
            death_issued_as_of: certificate.issued_as_of,
            as_of: 700,
        }
    );
}

#[test]
fn retrograde_rejection_lands_in_audit_log() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 400))
        .unwrap();
    assert!(step.retrograde_rejected());

    let record = book.audit_log().last().copied().unwrap();
    assert_eq!(
        record.event,
        RetraceAuditEvent::RetrogradeRejected {
            key: key_of(center, 3),
            last_as_of: 500,
            rejected_as_of: 400,
        }
    );
}

#[test]
fn late_absorption_lands_in_audit_log() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 500))
        .unwrap();
    let step = book
        .observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap();
    assert!(step.absorbed_late());

    let record = book.audit_log().last().copied().unwrap();
    assert_eq!(
        record.event,
        RetraceAuditEvent::LateAbsorbed {
            key: key_of(center, 3),
            terminal_as_of: 500,
            late_as_of: 600,
        }
    );
}

/// 引擎改口处死**不进** audit 流——它是真实终态事件，走日志本体，不是四类警报之一。
#[test]
fn center_rebased_does_not_enter_audit_log() {
    let mut book = ledger();
    let old = frame(1_200);
    book.observe(&up_input(old, 3, None, 500)).unwrap();
    book.observe(&up_input(frame(1_400), 3, None, 700)).unwrap();

    assert!(book.audit_log().is_empty(), "改口走日志本体，不是四类警报");
    assert_eq!(book.alarms().center_rebased, 1, "但计数仍照常累加");
}

// ═══════════════════════════════════════════════════════════════════════════
// append-only：序号即产生序，逐类混合仍保持全局单调
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn audit_log_sequence_is_globally_monotonic_across_mixed_event_types() {
    let mut book = ledger();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap(); // 注册，无事件
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 600))
        .unwrap(); // 首次判胜，无事件

    let broken = CenterFrame {
        zd: ZG,
        zg: ZD,
        start_index: ANCHOR_START + 9_000,
        end_index: 1,
    };
    book.observe(&up_input(broken, 1, None, 500)).ok(); // ResidualRejected #0
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 400)).ok(); // RetrogradeRejected #1（400 < 600）
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 700)).unwrap(); // LateAbsorbed #2（终态吸收）
    book.observe(&up_input(frame(1_400), 5, None, 900)).ok(); // DeadCenterRejected #3（同锚已死）

    let sequences: Vec<u64> = book.audit_log().iter().map(|record| record.sequence).collect();
    assert_eq!(sequences, vec![0, 1, 2, 3], "全局序号单调、不因事件类型分道");
}

// ═══════════════════════════════════════════════════════════════════════════
// 篡改 audit 流不影响 fold(journal) == book（裁定六核心断言）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn tampering_audit_log_never_affects_fold_journal_equals_book() {
    let mut book = ledger();
    let first = frame(1_200);
    book.observe(&up_input(first, 3, None, 500)).unwrap();
    book.observe(&up_input(first, 3, Some(RetraceOutcome::RetestReenters), 600))
        .unwrap();
    let second = frame(1_400);
    book.observe(&up_input(second, 5, Some(RetraceOutcome::Success), 700))
        .unwrap();
    // 触发全部四类警报，把 audit_log 填满非空内容。
    book.observe(&up_input(second, 5, Some(RetraceOutcome::Success), 800)).unwrap(); // 迟到吸收
    book.observe(&up_input(second, 5, None, 10)).unwrap(); // 倒退拒收
    book.observe(&up_input(frame(1_600), 9, None, 900)).ok(); // 死人挂号拒收
    assert!(!book.audit_log().is_empty(), "剧本须真的产生 audit 记录");

    let journal = book.journal().to_vec();
    let folded_before = RetraceLedger::fold(provenance(), &journal).unwrap();

    // 篡改：清空并塞入与真实历史无关的伪造事件（`audit_log` 字段 `pub(super)` 同树可见）。
    book.audit_log.clear();
    book.audit_log.push(RetraceAuditRecord {
        sequence: 9_999,
        event: RetraceAuditEvent::RetrogradeRejected {
            key: key_of(frame(1), 1),
            last_as_of: 1,
            rejected_as_of: 0,
        },
    });

    let folded_after = RetraceLedger::fold(provenance(), &journal).unwrap();
    assert_eq!(
        folded_after.journal(),
        folded_before.journal(),
        "fold 的入参只有 journal，audit_log 篡改无从传导"
    );
    for entry in book.entries() {
        let replayed = folded_after.entry(&entry.key).unwrap();
        assert_eq!(replayed.state, entry.state, "{:?}", entry.key);
        assert_eq!(replayed.revisions, entry.revisions, "{:?}", entry.key);
    }
    assert_eq!(folded_after.len(), book.len());
    folded_after.assert_invariants();
}

// ═══════════════════════════════════════════════════════════════════════════
// JSONL 正式持久化边界（对标 log.rs：只追加 / 幂等 / 冲突拒绝）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn jsonl_audit_round_trip_only_appends_and_replays_idempotently() {
    let mut book = ledger();
    let broken = CenterFrame {
        zd: ZG,
        zg: ZD,
        start_index: ANCHOR_START,
        end_index: 1_200,
    };
    book.observe(&up_input(broken, 3, None, 500)).ok();
    let center = frame(1_200);
    book.observe(&up_input(center, 3, None, 500)).unwrap();
    book.observe(&up_input(center, 3, Some(RetraceOutcome::Success), 400)).ok();

    let path = temp_path("roundtrip");
    let store = JsonlRetraceAuditStore::new(&path);
    store.append_all(&provenance(), book.audit_log()).unwrap();
    let before = std::fs::read(&path).unwrap();

    store.append_all(&provenance(), &[]).unwrap();
    let after = std::fs::read(&path).unwrap();
    assert_eq!(after, before, "追加空段不改字节");

    let loaded = store.load(&provenance()).unwrap();
    assert_eq!(loaded, book.audit_log().to_vec(), "读回逐条相同");

    // 重复落盘同一段 ⟹ 幂等。
    store.append_all(&provenance(), book.audit_log()).unwrap();
    assert_eq!(store.load(&provenance()).unwrap(), book.audit_log().to_vec());
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_audit_missing_file_loads_empty() {
    let store = JsonlRetraceAuditStore::new(temp_path("absent"));
    assert_eq!(store.load(&provenance()).unwrap(), Vec::new());
}

#[test]
fn jsonl_audit_provenance_mismatch_is_rejected() {
    let mut book = ledger();
    book.observe(&up_input(frame(1_200), 3, None, 500)).ok();
    let broken = CenterFrame {
        zd: ZG,
        zg: ZD,
        start_index: ANCHOR_START,
        end_index: 1_200,
    };
    book.observe(&up_input(broken, 3, None, 500)).ok();

    let path = temp_path("provenance");
    let store = JsonlRetraceAuditStore::new(&path);
    store.append_all(&provenance(), book.audit_log()).unwrap();

    let mut other = provenance();
    other.level = 4;
    assert!(matches!(
        store.load(&other),
        Err(RetraceAuditError::ProvenanceMismatch { .. })
    ));
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_audit_sequence_conflict_is_rejected() {
    let mut book = ledger();
    let broken = CenterFrame {
        zd: ZG,
        zg: ZD,
        start_index: ANCHOR_START,
        end_index: 1_200,
    };
    book.observe(&up_input(broken, 3, None, 500)).ok();

    let path = temp_path("conflict");
    let store = JsonlRetraceAuditStore::new(&path);
    store.append_all(&provenance(), book.audit_log()).unwrap();

    let mut tampered = book.audit_log()[0];
    tampered.sequence = 0;
    let RetraceAuditEvent::ResidualRejected { as_of, .. } = &mut tampered.event else {
        panic!("剧本第一条必为 ResidualRejected");
    };
    *as_of += 1;
    store.append_all(&provenance(), &[tampered]).unwrap();
    assert_eq!(
        store.load(&provenance()),
        Err(RetraceAuditError::SequenceConflict { sequence: 0 })
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_audit_non_append_sequence_is_rejected() {
    let path = temp_path("nonappend");
    let store = JsonlRetraceAuditStore::new(&path);
    let record = RetraceAuditRecord {
        sequence: 5,
        event: RetraceAuditEvent::RetrogradeRejected {
            key: key_of(frame(1_200), 3),
            last_as_of: 10,
            rejected_as_of: 5,
        },
    };
    store.append_all(&provenance(), &[record]).unwrap();
    assert_eq!(
        store.load(&provenance()),
        Err(RetraceAuditError::NonAppendSequence {
            expected: 0,
            actual: 5,
        })
    );
    let _ = std::fs::remove_file(path);
}
