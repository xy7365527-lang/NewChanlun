//! 日志重放一致性（AC②前半）：`fold(log) = state`、prefix 一致、缓存溯源校验失败回退重放、
//! JSONL 正式边界（只追加 / 重放幂等 / 冲突拒绝）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "newchan-retrace-{name}-{}-{}.jsonl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// 一条覆盖注册 / 判败 / Restart / 判胜 / 迟到吸收 / 倒退拒收的完整剧本。
fn scripted() -> RetraceLedger {
    let mut book = ledger();
    let first = frame(1_200);
    book.observe(&up_input(first, 3, None, 500)).unwrap();
    book.observe(&up_input(
        first,
        3,
        Some(RetraceOutcome::RetestReenters),
        600,
    ))
    .unwrap();
    let second = frame(1_400);
    book.observe(&up_input(second, 5, None, 700)).unwrap();
    book.observe(&up_input(second, 5, Some(RetraceOutcome::Success), 800))
        .unwrap();
    // 迟到与倒退都不进日志（不改状态，另记 audit 流）。
    book.observe(&up_input(second, 5, Some(RetraceOutcome::Success), 900))
        .unwrap();
    book.observe(&up_input(first, 3, None, 10)).unwrap();
    let other = other_frame(300);
    book.observe(&down_input(other, 11, None, 950)).unwrap();
    book.assert_invariants();
    book
}

#[test]
fn fold_of_log_equals_state() {
    let live = scripted();
    let replayed = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    assert_eq!(replayed.journal(), live.journal(), "日志逐条相同");
    for entry in live.entries() {
        let folded = replayed.entry(&entry.key).unwrap();
        assert_eq!(folded.state, entry.state, "三态：{:?}", entry.key);
        assert_eq!(folded.revisions, entry.revisions, "留档：{:?}", entry.key);
        assert_eq!(folded.registered_as_of, entry.registered_as_of);
        assert_eq!(folded.terminal_as_of, entry.terminal_as_of);
        assert_eq!(folded.registration(), entry.registration());
        assert_eq!(folded.restarted_from(), entry.restarted_from());
        assert_eq!(
            folded.not_constituted_reason(),
            entry.not_constituted_reason()
        );
    }
    assert_eq!(replayed.len(), live.len());
    assert_eq!(replayed.established(), live.established(), "成立档重放即得");
    settled(&replayed);
}

/// 门卫钟不是身份事实：重放后退回留档下界（log.rs §「门卫钟的可恢复性」）。
#[test]
fn gate_clock_recovers_to_the_bound_the_log_supports() {
    let mut live = ledger();
    let center = frame(1_200);
    live.observe(&up_input(center, 3, None, 500)).unwrap();
    live.observe(&up_input(center, 3, None, 4_000)).unwrap();
    let key = key_of(center, 3);
    assert_eq!(
        live.entry(&key).unwrap().last_as_of,
        4_000,
        "活值随观察前移"
    );
    assert_eq!(live.entry(&key).unwrap().log_supported_gate(), 500);

    let replayed = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    assert_eq!(
        replayed.entry(&key).unwrap().last_as_of,
        500,
        "日志对 500→4000 之间一无所知，账本不编造"
    );
    settled(&replayed);
}

#[test]
fn alarms_are_audit_only_and_never_enter_the_truth_path() {
    let live = scripted();
    assert_eq!(live.alarms().late_absorbed, 1);
    assert_eq!(live.alarms().retrograde_rejected, 1);
    let replayed = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    assert_eq!(
        replayed.alarms(),
        RetraceAlarms::default(),
        "警报不进真相恢复路径（裁定六）"
    );
}

#[test]
fn every_prefix_folds_consistently() {
    let live = scripted();
    let journal = live.journal().to_vec();
    let deeper = RetraceLedger::fold(provenance(), &journal).unwrap();
    for cut in 0..=journal.len() {
        let partial = RetraceLedger::fold(provenance(), &journal[..cut]).unwrap();
        partial.assert_invariants();
        assert_eq!(partial.journal().len(), cut, "截到哪是哪（裁定七）");
        for entry in partial.entries() {
            let full = deeper.entry(&entry.key).unwrap();
            assert_eq!(
                &full.revisions[..entry.revisions.len()],
                &entry.revisions[..],
                "前缀留档是全量留档的前缀：{:?}",
                entry.key
            );
        }
    }
}

#[test]
fn replay_is_idempotent() {
    let live = scripted();
    let once = RetraceLedger::fold(provenance(), live.journal()).unwrap();
    let twice = RetraceLedger::fold(provenance(), once.journal()).unwrap();
    assert_eq!(twice.journal(), once.journal());
    assert_eq!(twice.len(), once.len());
}

// ═══════════════════════════════════════════════════════════════════════════
// 派生缓存快照：带溯源、校验失败即扔
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn snapshot_cache_is_adopted_when_provenance_and_digest_match() {
    let live = scripted();
    let journal = live.journal().to_vec();
    let cache = RetraceLedger::fold(provenance(), &journal[..3])
        .unwrap()
        .snapshot();
    assert_eq!(cache.folded_through(), 3);

    let (restored, route) = RetraceLedger::restore(provenance(), Some(&cache), &journal).unwrap();
    assert_eq!(route, RestoreRoute::CacheAdopted { folded_through: 3 });
    assert_eq!(restored.journal(), live.journal());
    settled(&restored);
}

#[test]
fn snapshot_cache_with_tampered_prefix_falls_back_to_full_replay() {
    let live = scripted();
    let mut journal = live.journal().to_vec();
    let cache = RetraceLedger::fold(provenance(), &journal[..3])
        .unwrap()
        .snapshot();

    // 改写前缀内一条记录的知情时 ⟹ 指纹对不上。
    journal[2].revision.as_of += 1;
    let (restored, route) = RetraceLedger::restore(provenance(), Some(&cache), &journal).unwrap();
    assert_eq!(
        route,
        RestoreRoute::FullReplay {
            cause: Some(SnapshotRejection::DigestMismatch)
        },
        "缓存溯源校验失败 ⟹ 扔掉缓存全量重放（裁定六）"
    );
    assert_eq!(restored.journal().len(), journal.len());
}

#[test]
fn snapshot_cache_from_another_run_falls_back_to_full_replay() {
    let live = scripted();
    let cache = live.snapshot();
    let mut other = provenance();
    other.data_basis = "synthetic-other-run".to_owned();

    let (_, route) = RetraceLedger::restore(other, Some(&cache), live.journal()).unwrap();
    assert_eq!(
        route,
        RestoreRoute::FullReplay {
            cause: Some(SnapshotRejection::ProvenanceMismatch)
        }
    );
}

#[test]
fn snapshot_cache_longer_than_log_falls_back_to_full_replay() {
    let live = scripted();
    let cache = live.snapshot();
    let truncated = &live.journal()[..2];
    let (restored, route) = RetraceLedger::restore(provenance(), Some(&cache), truncated).unwrap();
    assert_eq!(
        route,
        RestoreRoute::FullReplay {
            cause: Some(SnapshotRejection::PrefixTooLong)
        }
    );
    assert_eq!(restored.journal().len(), 2);
}

#[test]
fn restore_without_cache_is_a_full_replay() {
    let live = scripted();
    let (restored, route) = RetraceLedger::restore(provenance(), None, live.journal()).unwrap();
    assert_eq!(route, RestoreRoute::FullReplay { cause: None });
    assert_eq!(restored.journal(), live.journal());
}

// ═══════════════════════════════════════════════════════════════════════════
// 折叠的 fail-loud 面（边界输入不信任）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn fold_rejects_non_append_sequence() {
    let live = scripted();
    let mut journal = live.journal().to_vec();
    journal[2].sequence = 9;
    assert_eq!(
        RetraceLedger::fold(provenance(), &journal),
        Err(RetraceLogError::NonAppendSequence {
            expected: 2,
            actual: 9
        })
    );
}

#[test]
fn fold_rejects_resurrection_record() {
    let live = scripted();
    let mut journal = live.journal().to_vec();
    let terminal = *journal
        .iter()
        .find(|record| record.revision.kind == RetraceRevisionKind::Confirmed)
        .unwrap();
    journal.push(RetraceRecord {
        sequence: journal.len() as u64,
        revision: terminal.revision,
    });
    let sequence = (journal.len() - 1) as u64;
    assert_eq!(
        RetraceLedger::fold(provenance(), &journal),
        Err(RetraceLogError::FoldViolation {
            sequence,
            detail: "终态条目禁再落锤（禁复活）"
        })
    );
}

#[test]
fn fold_rejects_revision_on_absent_identity() {
    let live = scripted();
    let mut journal = live.journal().to_vec();
    journal.remove(0);
    for (index, record) in journal.iter_mut().enumerate() {
        record.sequence = index as u64;
    }
    assert_eq!(
        RetraceLedger::fold(provenance(), &journal),
        Err(RetraceLogError::FoldViolation {
            sequence: 0,
            detail: "修订记录落在未建仓身份"
        })
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// JSONL 正式持久化边界
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn jsonl_round_trip_only_appends_and_replays_idempotently() {
    let live = scripted();
    let path = temp_path("roundtrip");
    let store = JsonlRetraceLogStore::new(&path);
    store
        .append_all(&provenance(), &live.journal()[..3])
        .unwrap();
    let before = std::fs::read(&path).unwrap();

    store
        .append_all(&provenance(), &live.journal()[3..])
        .unwrap();
    let after = std::fs::read(&path).unwrap();
    assert!(after.starts_with(&before), "第二段必须只追加在旧字节之后");

    let loaded = store.load(&provenance()).unwrap();
    assert_eq!(loaded, live.journal(), "读回逐条相同");
    let restored = RetraceLedger::fold(provenance(), &loaded).unwrap();
    assert_eq!(restored.len(), live.len());
    restored.assert_invariants();

    // 同一段重复落盘 ⟹ 读回仍是同一条日志（幂等）。
    store
        .append_all(&provenance(), &live.journal()[3..])
        .unwrap();
    assert_eq!(store.load(&provenance()).unwrap(), live.journal());
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_missing_file_loads_empty() {
    let store = JsonlRetraceLogStore::new(temp_path("absent"));
    assert_eq!(store.load(&provenance()).unwrap(), Vec::new());
}

#[test]
fn jsonl_provenance_mismatch_is_rejected() {
    let live = scripted();
    let path = temp_path("provenance");
    let store = JsonlRetraceLogStore::new(&path);
    store.append_all(&provenance(), live.journal()).unwrap();

    let mut other = provenance();
    other.level = 4;
    assert!(matches!(
        store.load(&other),
        Err(RetraceLogError::ProvenanceMismatch { .. })
    ));
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_sequence_conflict_is_rejected() {
    let live = scripted();
    let path = temp_path("conflict");
    let store = JsonlRetraceLogStore::new(&path);
    store.append_all(&provenance(), live.journal()).unwrap();

    let mut tampered = live.journal()[1];
    tampered.revision.as_of += 7;
    store.append_all(&provenance(), &[tampered]).unwrap();
    assert_eq!(
        store.load(&provenance()),
        Err(RetraceLogError::SequenceConflict { sequence: 1 })
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_without_header_is_rejected() {
    let path = temp_path("headerless");
    let live = scripted();
    let line = serde_json::to_string(&serde_json::json!({
        "schema_version": LOG_SCHEMA_VERSION,
        "record": "revision",
        "sequence": 0,
        "key": live.journal()[0].revision.key,
        "kind": "registered",
        "as_of": 500,
        "evidence": serde_json::Value::Null,
    }))
    .unwrap();
    std::fs::write(&path, format!("{line}\n")).unwrap();
    assert_eq!(
        JsonlRetraceLogStore::new(&path).load(&provenance()),
        Err(RetraceLogError::MissingProvenanceHeader)
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn jsonl_unsupported_schema_is_rejected() {
    let path = temp_path("schema");
    std::fs::write(&path, "{\"schema_version\":99,\"record\":\"provenance\"}\n").unwrap();
    assert_eq!(
        JsonlRetraceLogStore::new(&path).load(&provenance()),
        Err(RetraceLogError::UnsupportedSchema {
            line: 1,
            schema: 99
        })
    );
    let _ = std::fs::remove_file(path);
}
