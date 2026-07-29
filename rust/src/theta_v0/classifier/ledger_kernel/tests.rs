//! 内核行为测试（票 #573 T1，TDD 接缝 = `LedgerBook` 公共面）。
//!
//! 探针域（`Probe*`）是**纯合成的对象无关消费方**：既非 nest 活假设、也非买卖点，
//! 只为证明 11 个内核责任点不依赖任何具体域。域侧语义（判据、原因含义、业务钟）
//! 一律不出现在本文件——出现即说明内核签名漏了域概念。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 探针域：四组类型参数的最小合成实例
// ═══════════════════════════════════════════════════════════════════════════

/// 探针身份键（等值 + 序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ProbeKey {
    anchor: u32,
    /// 可动分量：迁移判据（域侧）允许它变化而身份不变。
    tail: u32,
}

/// 探针修订词汇。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeKind {
    Opened,
    Migrated,
    Marked,
    Settled,
}

/// 探针原因码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeReason {
    Fulfilled,
    Withdrawn,
}

/// 探针证据载荷。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeEvidence {
    weight: i64,
}

/// 探针观察。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeObservation {
    key: ProbeKey,
}

/// 探针条目：域自选字段布局（终态钟拆两只、另有一只域钟 `marked_at`）——
/// 内核只经访问器读写，不假设布局。
#[derive(Debug, Clone, PartialEq)]
struct ProbeEntry {
    key: ProbeKey,
    state: LedgerState,
    revision: u32,
    opened_at: usize,
    last_as_of: usize,
    fulfilled_at: Option<usize>,
    withdrawn_at: Option<usize>,
    reason: Option<ProbeReason>,
    evidence: Option<ProbeEvidence>,
    migrated_from: Option<ProbeKey>,
    /// 域钟：验证 [`first_write_clock`] 的首写不后移纪律。
    marked_at: Option<usize>,
    revisions: Vec<LedgerRevision<ProbePolicy>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbePolicy;

impl LedgerPolicy for ProbePolicy {
    type Key = ProbeKey;
    type Observation = ProbeObservation;
    type RevisionKind = ProbeKind;
    type Reason = ProbeReason;
    type Evidence = ProbeEvidence;
    type Entry = ProbeEntry;

    fn observation_key(observation: &Self::Observation) -> Self::Key {
        observation.key
    }

    fn opened_revision_kind() -> Self::RevisionKind {
        ProbeKind::Opened
    }
}

impl LedgerEntryCore<ProbePolicy> for ProbeEntry {
    fn open(key: ProbeKey, as_of: usize) -> Self {
        Self {
            key,
            state: LedgerState::Provisional,
            revision: 0,
            opened_at: as_of,
            last_as_of: as_of,
            fulfilled_at: None,
            withdrawn_at: None,
            reason: None,
            evidence: None,
            migrated_from: None,
            marked_at: None,
            revisions: Vec::new(),
        }
    }

    fn key(&self) -> ProbeKey {
        self.key
    }

    fn set_key(&mut self, key: ProbeKey) {
        self.key = key;
    }

    fn state(&self) -> LedgerState {
        self.state
    }

    fn set_state(&mut self, state: LedgerState) {
        self.state = state;
    }

    fn revision_count(&self) -> u32 {
        self.revision
    }

    fn set_revision_count(&mut self, count: u32) {
        self.revision = count;
    }

    fn revisions(&self) -> &[LedgerRevision<ProbePolicy>] {
        &self.revisions
    }

    fn revisions_mut(&mut self) -> &mut Vec<LedgerRevision<ProbePolicy>> {
        &mut self.revisions
    }

    fn opened_at(&self) -> usize {
        self.opened_at
    }

    fn last_as_of(&self) -> usize {
        self.last_as_of
    }

    fn set_last_as_of(&mut self, as_of: usize) {
        self.last_as_of = as_of;
    }

    fn settled_at(&self) -> Option<usize> {
        self.fulfilled_at.or(self.withdrawn_at)
    }

    fn migrated_from(&self) -> Option<ProbeKey> {
        self.migrated_from
    }

    fn set_migrated_from(&mut self, from: ProbeKey) {
        self.migrated_from = Some(from);
    }

    fn write_settlement(
        &mut self,
        state: LedgerState,
        reason: Option<ProbeReason>,
        as_of: usize,
        evidence: Option<ProbeEvidence>,
    ) {
        match state {
            LedgerState::Confirmed => self.fulfilled_at = Some(as_of),
            LedgerState::Invalidated => self.withdrawn_at = Some(as_of),
            LedgerState::Provisional => unreachable!("非终态不入终态落账"),
        }
        self.reason = reason;
        self.evidence = evidence;
    }
}

fn key(anchor: u32, tail: u32) -> ProbeKey {
    ProbeKey { anchor, tail }
}

fn observation(anchor: u32, tail: u32) -> ProbeObservation {
    ProbeObservation {
        key: key(anchor, tail),
    }
}

fn book() -> LedgerBook<ProbePolicy> {
    LedgerBook::new()
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 1：per-key 注册表
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn registry_routes_observations_per_key() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.open_on_observation(&observation(2, 0), 10);

    assert_eq!(ledger.len(), 2);
    assert!(!ledger.is_empty());
    assert!(ledger.contains(&key(1, 0)));
    assert!(ledger.contains(&key(2, 0)));
    assert!(!ledger.contains(&key(3, 0)));
    assert_eq!(ledger.get(&key(1, 0)).map(|entry| entry.key), Some(key(1, 0)));
    // BTreeMap 序：注册表枚举确定性（无平局歧义）。
    let keys: Vec<ProbeKey> = ledger.keys().copied().collect();
    assert_eq!(keys, vec![key(1, 0), key(2, 0)]);
}

#[test]
fn empty_registry_reports_empty() {
    let ledger = book();
    assert_eq!(ledger.len(), 0);
    assert!(ledger.is_empty());
    assert!(ledger.get(&key(1, 0)).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 2：首次观察建项
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn first_observation_opens_entry_with_opened_revision() {
    let mut ledger = book();
    let revision = ledger
        .open_on_observation(&observation(1, 0), 42)
        .expect("首次观察建项");

    assert_eq!(revision.key, key(1, 0));
    assert_eq!(revision.kind, ProbeKind::Opened);
    assert_eq!(revision.as_of, 42);
    assert!(revision.evidence.is_none());

    let entry = ledger.get(&key(1, 0)).expect("建项后可读");
    assert_eq!(entry.state, LedgerState::Provisional);
    assert_eq!(entry.opened_at, 42);
    assert_eq!(entry.last_as_of, 42);
    assert_eq!(entry.revision, 1);
    assert_eq!(entry.revisions.len(), 1);
}

#[test]
fn repeat_observation_never_rebuilds_entry() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 42);
    let repeat = ledger.open_on_observation(&observation(1, 0), 77);

    assert!(repeat.is_none(), "已建仓身份不得二次建项");
    let entry = ledger.get(&key(1, 0)).expect("条目仍在");
    assert_eq!(entry.opened_at, 42, "出生钟不因重复观察改写");
    assert_eq!(entry.revision, 1, "重复观察零修订");
    assert_eq!(entry.last_as_of, 42, "建项路径不推进门卫钟");
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 3/4：append-only 修订追加 + 修订计数与历史一致
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn revisions_are_append_only() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    entry.push_revision(ProbeKind::Marked, 11, Some(ProbeEvidence { weight: 3 }));
    entry.push_revision(ProbeKind::Marked, 12, None);

    let kinds: Vec<ProbeKind> = entry.revisions.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        vec![ProbeKind::Opened, ProbeKind::Marked, ProbeKind::Marked],
        "留档只追加、既有条目不改写不删除"
    );
    assert_eq!(entry.revisions[1].as_of, 11);
    assert_eq!(entry.revisions[1].evidence, Some(ProbeEvidence { weight: 3 }));
    assert_eq!(entry.revisions[2].evidence, None);
}

#[test]
fn revision_count_equals_history_length() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    for step in 0..5 {
        entry.push_revision(ProbeKind::Marked, 11 + step, None);
        assert_eq!(
            entry.revision as usize,
            entry.revisions.len(),
            "每次追加后计数与留档长度同步"
        );
    }
    assert_eq!(entry.revision, 6);
}

#[test]
fn pushed_revision_carries_current_key() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.migrate(key(1, 0), key(1, 9), ProbeKind::Migrated, 11);
    let entry = ledger.get_mut(&key(1, 9)).expect("迁移后条目在新键");
    let revision = entry.push_revision(ProbeKind::Marked, 12, None);
    assert_eq!(revision.key, key(1, 9), "修订记录取追加时的当前键");
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 5：per-identity 倒退拒绝
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn retrograde_admission_rejected_with_zero_mutation() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 20);
    ledger.admit(&key(1, 0), 30);
    let before = ledger.get(&key(1, 0)).expect("条目已建仓").clone();

    let admission = ledger.admit(&key(1, 0), 25);

    assert_eq!(admission, LedgerAdmission::RetrogradeRejected);
    assert_eq!(
        ledger.get(&key(1, 0)).expect("条目仍在"),
        &before,
        "倒退喂入零修订、零状态改写"
    );
    assert_eq!(
        ledger.retrograde_rejections(),
        &[LedgerRetrogradeRejection {
            key: key(1, 0),
            last_as_of: 30,
            rejected_as_of: 25,
        }],
        "拒绝显式记注记，不静默吸收"
    );
}

#[test]
fn non_retrograde_admission_advances_gate_clock() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 20);

    assert_eq!(ledger.admit(&key(1, 0), 20), LedgerAdmission::Accepted);
    assert_eq!(ledger.get(&key(1, 0)).expect("条目在").last_as_of, 20);
    assert_eq!(ledger.admit(&key(1, 0), 33), LedgerAdmission::Accepted);
    assert_eq!(ledger.get(&key(1, 0)).expect("条目在").last_as_of, 33);
    assert!(ledger.retrograde_rejections().is_empty());
}

#[test]
fn retrograde_rejection_recordable_before_entry_exists() {
    // 建仓前的判同分支也要能记注记（被拒键本身尚未建仓）。
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 20);
    ledger.reject_retrograde(key(1, 7), 20, 15);

    assert_eq!(ledger.len(), 1, "拒绝不建仓");
    assert_eq!(ledger.retrograde_rejections().len(), 1);
    assert_eq!(ledger.retrograde_rejections()[0].key, key(1, 7));
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 6：终态吸收
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn terminal_state_absorbs_further_admission() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 20);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    entry.settle(
        LedgerSettlement {
            state: LedgerState::Invalidated,
            kind: ProbeKind::Settled,
            reason: Some(ProbeReason::Withdrawn),
            evidence: Some(ProbeEvidence { weight: -1 }),
        },
        21,
    );

    assert_eq!(ledger.admit(&key(1, 0), 22), LedgerAdmission::TerminalAbsorbed);
    let entry = ledger.get(&key(1, 0)).expect("条目仍在");
    assert_eq!(entry.state, LedgerState::Invalidated, "禁复活");
    assert_eq!(entry.revision, 2, "终态后续观察零修订");
    assert_eq!(entry.withdrawn_at, Some(21), "终态钟只写一次");
    assert_eq!(entry.reason, Some(ProbeReason::Withdrawn));
    assert_eq!(entry.evidence, Some(ProbeEvidence { weight: -1 }));
}

#[test]
fn terminal_predicate_covers_both_terminal_states() {
    assert!(!LedgerState::Provisional.is_terminal());
    assert!(LedgerState::Confirmed.is_terminal());
    assert!(LedgerState::Invalidated.is_terminal());
}

#[test]
#[should_panic(expected = "终态落账要求终态")]
fn settlement_into_non_terminal_state_fails_loud() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 20);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    entry.settle(
        LedgerSettlement {
            state: LedgerState::Provisional,
            kind: ProbeKind::Settled,
            reason: None,
            evidence: None,
        },
        21,
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 7：钟首次写入不后移
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn first_clock_write_wins_and_never_moves() {
    let mut slot = None;
    assert!(first_write_clock(&mut slot, 30), "首写落笔");
    assert_eq!(slot, Some(30));
    assert!(!first_write_clock(&mut slot, 40), "已写不再落笔");
    assert_eq!(slot, Some(30), "钟不后移");
    assert!(!first_write_clock(&mut slot, 10), "已写不前移");
    assert_eq!(slot, Some(30));
}

#[test]
fn domain_clock_uses_first_write_discipline() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 20);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    assert!(first_write_clock(&mut entry.marked_at, 21));
    assert!(!first_write_clock(&mut entry.marked_at, 22));
    assert_eq!(entry.marked_at, Some(21));
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 8：每次推进返回增量
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn delta_carries_only_fresh_revisions() {
    let mut ledger = book();
    let mut first = LedgerDelta::new();
    first.record_opt(ledger.open_on_observation(&observation(1, 0), 10));
    assert_eq!(first.len(), 1);

    let mut second = LedgerDelta::new();
    second.record_opt(ledger.open_on_observation(&observation(1, 0), 11));
    assert!(second.is_empty(), "无新事实则增量为空，消费方不重扫全账");

    let mut third = LedgerDelta::new();
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    third.record(entry.push_revision(ProbeKind::Marked, 12, None));
    let fresh = third.into_vec();
    assert_eq!(fresh.len(), 1, "第三轮只返回本轮新增");
    assert_eq!(fresh[0].kind, ProbeKind::Marked);
    assert_eq!(
        ledger.get(&key(1, 0)).expect("条目在").revisions.len(),
        2,
        "全量留档仍在账本内，增量不替代留档"
    );
}

#[test]
fn delta_preserves_record_order() {
    let mut ledger = book();
    let mut delta = LedgerDelta::new();
    delta.record_opt(ledger.open_on_observation(&observation(1, 0), 10));
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    delta.record(entry.push_revision(ProbeKind::Marked, 11, None));
    delta.record(entry.push_revision(ProbeKind::Settled, 12, None));

    let kinds: Vec<ProbeKind> = delta.as_slice().iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        vec![ProbeKind::Opened, ProbeKind::Marked, ProbeKind::Settled]
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 9：身份迁移保留历史
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn migration_preserves_clocks_and_history() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    entry.push_revision(ProbeKind::Marked, 11, None);
    assert!(first_write_clock(&mut entry.marked_at, 11));

    let revision = ledger.migrate(key(1, 0), key(1, 5), ProbeKind::Migrated, 12);

    assert_eq!(revision.key, key(1, 5), "迁移修订记新键");
    assert_eq!(revision.kind, ProbeKind::Migrated);
    assert!(ledger.get(&key(1, 0)).is_none(), "旧键退出注册表");
    let migrated = ledger.get(&key(1, 5)).expect("新键接住条目");
    assert_eq!(migrated.key, key(1, 5));
    assert_eq!(migrated.opened_at, 10, "迁移不动出生钟");
    assert_eq!(migrated.marked_at, Some(11), "迁移不动域钟");
    assert_eq!(migrated.last_as_of, 10, "迁移本身不推进门卫钟");
    assert_eq!(migrated.migrated_from, Some(key(1, 0)), "来源链留痕");
    assert_eq!(
        migrated.revisions.iter().map(|r| r.kind).collect::<Vec<_>>(),
        vec![ProbeKind::Opened, ProbeKind::Marked, ProbeKind::Migrated],
        "旧修订全部保留，迁移追加一条"
    );
    assert_eq!(migrated.revision, 3);
    assert_eq!(ledger.len(), 1, "迁移是换键不是新建");
}

#[test]
#[should_panic(expected = "身份迁移两端不得同键")]
fn migration_to_same_key_fails_loud() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.migrate(key(1, 0), key(1, 0), ProbeKind::Migrated, 11);
}

#[test]
#[should_panic(expected = "迁移源键存在")]
fn migration_from_absent_key_fails_loud() {
    let mut ledger = book();
    ledger.migrate(key(1, 0), key(1, 5), ProbeKind::Migrated, 11);
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 10：只读枚举/过滤门户
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn readonly_portals_enumerate_and_filter_by_state() {
    let mut ledger = book();
    for anchor in 1..=3 {
        ledger.open_on_observation(&observation(anchor, 0), 10);
    }
    ledger
        .get_mut(&key(2, 0))
        .expect("条目已建仓")
        .settle(
            LedgerSettlement {
                state: LedgerState::Confirmed,
                kind: ProbeKind::Settled,
                reason: Some(ProbeReason::Fulfilled),
                evidence: None,
            },
            11,
        );

    let all: Vec<ProbeKey> = ledger.values().map(|entry| entry.key).collect();
    assert_eq!(all, vec![key(1, 0), key(2, 0), key(3, 0)], "全量枚举留档不删");

    let confirmed: Vec<ProbeKey> = ledger
        .in_state(LedgerState::Confirmed)
        .iter()
        .map(|entry| entry.key)
        .collect();
    assert_eq!(confirmed, vec![key(2, 0)], "按态过滤只放该态");

    let provisional: Vec<ProbeKey> = ledger
        .in_state(LedgerState::Provisional)
        .iter()
        .map(|entry| entry.key)
        .collect();
    assert_eq!(provisional, vec![key(1, 0), key(3, 0)]);
    assert!(ledger.in_state(LedgerState::Invalidated).is_empty());

    let pairs: Vec<(ProbeKey, LedgerState)> = ledger
        .entries()
        .map(|(k, entry)| (*k, entry.state))
        .collect();
    assert_eq!(pairs.len(), 3);
    assert!(pairs
        .iter()
        .all(|(k, _)| ledger.get(k).is_some_and(|entry| entry.key == *k)));
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 11：不变量骨架
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn core_invariants_hold_across_full_lifecycle() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.assert_core_invariants();

    ledger.admit(&key(1, 0), 11);
    ledger.migrate(key(1, 0), key(1, 4), ProbeKind::Migrated, 11);
    ledger.assert_core_invariants();

    ledger.admit(&key(1, 4), 12);
    ledger
        .get_mut(&key(1, 4))
        .expect("条目已建仓")
        .settle(
            LedgerSettlement {
                state: LedgerState::Invalidated,
                kind: ProbeKind::Settled,
                reason: Some(ProbeReason::Withdrawn),
                evidence: None,
            },
            12,
        );
    ledger.assert_core_invariants();

    ledger.open_on_observation(&observation(2, 0), 13);
    ledger.assert_core_invariants();
}

#[test]
#[should_panic(expected = "修订计数 == 留档长度")]
fn tampered_revision_count_trips_invariant() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.get_mut(&key(1, 0)).expect("条目已建仓").revision = 7;
    ledger.assert_core_invariants();
}

#[test]
#[should_panic(expected = "终态 ⟺ 终态钟")]
fn state_without_terminal_clock_trips_invariant() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    // 绕过 `settle` 直接改态：终态钟未写 ⟹ 不变量必须逮住。
    ledger.get_mut(&key(1, 0)).expect("条目已建仓").state = LedgerState::Confirmed;
    ledger.assert_core_invariants();
}

#[test]
#[should_panic(expected = "注册表键 == 条目键")]
fn key_desync_trips_invariant() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.get_mut(&key(1, 0)).expect("条目已建仓").key = key(9, 9);
    ledger.assert_core_invariants();
}

#[test]
#[should_panic(expected = "留档时序非降")]
fn retrograde_revision_history_trips_invariant() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    let entry = ledger.get_mut(&key(1, 0)).expect("条目已建仓");
    entry.push_revision(ProbeKind::Marked, 9, None);
    entry.last_as_of = 10;
    ledger.assert_core_invariants();
}

#[test]
#[should_panic(expected = "出生钟 ≤ 门卫钟")]
fn gate_clock_behind_birth_clock_trips_invariant() {
    let mut ledger = book();
    ledger.open_on_observation(&observation(1, 0), 10);
    ledger.get_mut(&key(1, 0)).expect("条目已建仓").last_as_of = 9;
    ledger.assert_core_invariants();
}

// ═══════════════════════════════════════════════════════════════════════════
// 账本容器面：Default / Clone / PartialEq（域侧 derive 依赖）
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn book_container_traits_available_for_domain_derive() {
    let mut ledger: LedgerBook<ProbePolicy> = LedgerBook::default();
    ledger.open_on_observation(&observation(1, 0), 10);
    let cloned = ledger.clone();
    assert_eq!(cloned, ledger);

    let mut other = book();
    other.open_on_observation(&observation(1, 0), 11);
    assert_ne!(other, ledger, "出生钟不同 ⟹ 账本不等");
    assert!(format!("{ledger:?}").contains("LedgerBook"));
}
