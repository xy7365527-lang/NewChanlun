//! 事件日志 golden 锚（票 #624 S4 ②）：JSONL 字节 + 指纹 + 重放一致。
//!
//! # 锚什么
//!
//! [`anchored`] 是一条覆盖**六个场景**的固定剧本——判胜 / 判败 / Restart 新档 / 改口处死 /
//! 迟到吸收 / 死人挂号——其修订日志逐字节冻结在
//! `rust/tests/fixtures/issue624_retrace_ledger_events.golden.jsonl`。
//!
//! 六场景中**只有四个进日志**：迟到吸收与死人挂号是「未被采纳的输入」，按 #574 裁定六走 audit
//! 流、不改状态、结构上不进真相路径（`audit.rs` 模块头）。golden 里因此看不到它们——这不是覆盖
//! 缺口，是裁定六的可观测证据，由 [`golden_scenario_covers_all_six_faces`] 正面钉死。
//!
//! # 三层锚
//!
//! 1. **字节**（[`golden_event_log_matches_the_repo_anchor`]）：活账日志经
//!    [`JsonlRetraceLogStore`] 外化后与仓内 golden 逐字节相等；
//! 2. **指纹**（[`golden_log_digest_is_pinned_and_tamper_evident`]）：`FNV-1a` 前缀指纹钉成常量，
//!    任一字段被改写即红；
//! 3. **重放**（[`golden_log_folds_back_to_the_anchored_ledger_state`]）：`fold(golden 日志)` 逐条
//!    复现锚定账本态（走真实 JSONL 读回边界，不是内存 journal 直传）。
//!
//! # golden 变更纪律（对齐 `tests/issue533_p123_byte_guardrail.rs` 先例）
//!
//! golden 只允许因**已审阅、故意的**行为变化而更新，且须在同一提交里说明原因；禁止为了让测试
//! 变绿而静默重新生成。重生成入口：
//!
//! ```sh
//! cargo test --lib retrace_ledger::tests::golden_log::regenerate_golden -- --ignored --nocapture
//! ```
//!
//! 变更登记（每次动 golden 追加一行）：
//!
//! | commit | 原因 |
//! |---|---|
//! | 本次（#624） | 首次锚定（S4 验收环 ②） |

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::log::digest_records;
use super::*;

/// 锚定日志的 FNV-1a 前缀指纹（由 [`regenerate_golden`] 打印，动 golden 必须同步更新）。
const GOLDEN_DIGEST: u64 = 0x808b_849b_9ba1_d112;

/// 锚定日志的记录条数（六场景中进真相路径的那四个共产出的修订条数）。
const GOLDEN_RECORD_COUNT: usize = 11;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "newchan-retrace-golden-{name}-{}-{}.jsonl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue624_retrace_ledger_events.golden.jsonl")
}

/// 六场景剧本（**动它就动 golden**，见模块头变更纪律）。
///
/// | 拍 | 知情时 | 场景 | 进日志？ |
/// |---|---:|---|---|
/// | ① | 500 | 注册（未决，向上离开 ⟹ 三类买点候选） | ✓ 建项 + 钉快照 |
/// | ② | 600 | 判败：回抽重回快照框内 | ✓ `NotConstituted{RetestReentered}` |
/// | ③ | 700 | Restart 新档（谱系载荷记前任） | ✓ 建项 + 钉快照 + `Restarted` |
/// | ④ | 800 | 引擎改口处死（显式核对通道报出新窗口） | ✓ `NotConstituted{CenterRebased}` |
/// | ⑤ | 900 | 判胜（快照转正 + 死亡证明） | ✓ 建项 + 钉快照 + `Restarted` + `Confirmed` |
/// | ⑥ | 1000 | 同身份迟到输入静默吸收 | ✗ audit 流 |
/// | ⑦ | 1100 | 死人挂号拒收（已死中枢的新 departure） | ✗ audit 流 |
fn anchored() -> RetraceLedger {
    let mut book = ledger();
    book.observe(&up_input(frame(1_200), 3, None, 500)).unwrap();
    book.observe(&up_input(frame(1_200), 3, Some(RetraceOutcome::RetestReenters), 600))
        .unwrap();
    book.observe(&up_input(frame(1_400), 5, None, 700)).unwrap();
    book.reconcile_window(CenterAnchor(ANCHOR_START), Some(frame(1_500)), 800)
        .unwrap()
        .expect("改口处死必产一拍");
    book.observe(&up_input(frame(1_500), 7, Some(RetraceOutcome::Success), 900))
        .unwrap();
    book.observe(&up_input(frame(1_500), 7, Some(RetraceOutcome::Success), 1_000))
        .unwrap();
    book.observe(&up_input(frame(1_600), 9, None, 1_100))
        .unwrap_err();
    book.assert_invariants();
    book
}

/// 把活账日志外化成 JSONL 字节（与落 golden 时同一条路径）。
fn externalize(book: &RetraceLedger, name: &str) -> String {
    let path = temp_path(name);
    JsonlRetraceLogStore::new(&path)
        .append_all(&provenance(), book.journal())
        .expect("外化必成功");
    let bytes = std::fs::read(&path).expect("读回外化产物");
    let _ = std::fs::remove_file(&path);
    String::from_utf8(bytes).expect("JSONL 必是 UTF-8")
}

// ═══════════════════════════════════════════════════════════════════════════
// 锚一：字节
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn golden_event_log_matches_the_repo_anchor() {
    let live = anchored();
    assert_eq!(
        live.journal().len(),
        GOLDEN_RECORD_COUNT,
        "剧本产出的修订条数变了 ⟹ 先确认是不是故意的，再动 golden"
    );
    let golden = std::fs::read_to_string(golden_path())
        .unwrap_or_else(|error| panic!("读 golden {} 失败：{error}", golden_path().display()));
    assert_eq!(
        externalize(&live, "actual"),
        golden,
        "#624 DRIFT：事件日志与仓内 golden 不再逐字节相等"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 锚二：指纹（篡改即红）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn golden_log_digest_is_pinned_and_tamper_evident() {
    let live = anchored();
    assert_eq!(
        digest_records(live.journal()),
        GOLDEN_DIGEST,
        "锚定日志指纹变了 ⟹ 事件流变了"
    );

    for index in 0..live.journal().len() {
        let mut tampered = live.journal().to_vec();
        tampered[index].revision.as_of += 1;
        assert_ne!(
            digest_records(&tampered),
            GOLDEN_DIGEST,
            "第 {index} 条被改写却指纹不变 ⟹ 指纹形同虚设"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 锚三：重放一致（走真实 JSONL 读回边界）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn golden_log_folds_back_to_the_anchored_ledger_state() {
    let live = anchored();
    let loaded = JsonlRetraceLogStore::new(golden_path())
        .load(&provenance())
        .expect("golden 必可读回");
    assert_eq!(loaded, live.journal(), "golden 读回逐条 = 活账日志");

    let folded = RetraceLedger::fold(provenance(), &loaded).expect("golden 必可折叠");
    assert_eq!(folded.len(), live.len(), "身份集相同");
    for entry in live.entries() {
        let replayed = folded.entry(&entry.key).unwrap();
        assert_eq!(replayed.state, entry.state, "三态：{:?}", entry.key);
        assert_eq!(replayed.revisions, entry.revisions, "留档逐条：{:?}", entry.key);
        assert_eq!(replayed.registered_as_of, entry.registered_as_of);
        assert_eq!(replayed.terminal_as_of, entry.terminal_as_of);
        assert_eq!(replayed.restarted_from(), entry.restarted_from());
        assert_eq!(
            replayed.not_constituted_reason(),
            entry.not_constituted_reason()
        );
    }
    assert_eq!(folded.established(), live.established(), "成立档重放即得");
    assert_eq!(
        folded.short_retrace_records(),
        live.short_retrace_records(),
        "短差档重放即得"
    );
    settled(&folded);
}

// ═══════════════════════════════════════════════════════════════════════════
// 场景覆盖：六个面各自落在哪条流
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn golden_scenario_covers_all_six_faces() {
    let live = anchored();

    // ①②③⑤ 四个进真相路径的面：日志词汇里各自可见。
    let kinds: Vec<_> = live
        .journal()
        .iter()
        .map(|record| record.revision.kind)
        .collect();
    assert!(
        kinds.contains(&RetraceRevisionKind::Confirmed),
        "判胜进日志"
    );
    assert!(
        kinds.contains(&RetraceRevisionKind::NotConstituted {
            reason: NotConstitutedReason::RetestReentered
        }),
        "判败进日志"
    );
    assert!(
        kinds.iter().any(|kind| matches!(
            kind,
            RetraceRevisionKind::NotConstituted {
                reason: NotConstitutedReason::CenterRebased { .. }
            }
        )),
        "改口处死进日志"
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, RetraceRevisionKind::Restarted { .. })),
        "Restart 新档谱系载荷进日志"
    );

    // ⑥⑦ 两个不进真相路径的面：只在 audit 流与警报计数里可见。
    let alarms = live.alarms();
    assert_eq!(alarms.late_absorbed, 1, "迟到吸收计数");
    assert_eq!(alarms.dead_center_registrations, 1, "死人挂号拒收计数");
    assert_eq!(alarms.center_rebased, 1, "改口处死计数（现算自折叠投影）");
    assert!(live.audit_log().iter().any(|record| matches!(
        record.event,
        RetraceAuditEvent::LateAbsorbed { .. }
    )));
    assert!(live.audit_log().iter().any(|record| matches!(
        record.event,
        RetraceAuditEvent::DeadCenterRejected { .. }
    )));

    // 死亡证明：判胜那一档的快照转正（成立档消费面）。
    let pack = live.established();
    assert_eq!(pack.len(), 1, "六场景恰产一份成立档");
    assert_eq!(pack[0].death_certificate.issued_as_of, 900);
    assert_eq!(pack[0].frame, frame(1_500), "快照转正 = 注册拍那四条边");
}

// ═══════════════════════════════════════════════════════════════════════════
// golden 重生成入口（模块头变更纪律：只为已审阅的故意行为变化跑）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "golden 重生成入口：只为已审阅的故意行为变化跑（见模块头变更纪律）"]
fn regenerate_golden() {
    let live = anchored();
    let path = golden_path();
    let _ = std::fs::remove_file(&path);
    JsonlRetraceLogStore::new(&path)
        .append_all(&provenance(), live.journal())
        .expect("落 golden 必成功");
    eprintln!(
        "[#624] golden 已重落 {}；records={} digest=0x{:016x}",
        path.display(),
        live.journal().len(),
        digest_records(live.journal())
    );
}
