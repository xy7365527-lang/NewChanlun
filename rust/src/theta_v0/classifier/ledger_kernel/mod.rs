//! 账本内核（票 #573 T1；#465 裁定 A）：对象无关责任点的泛型承载体。
//!
//! # 由来
//!
//! `nest_lifecycle` 把「可抽出的账本内核」与「活假设业务策略」写在同一模块同一 `advance`：
//! #465 调研（`chanlun/review-results/issue465-nest-lifecycle-reusability-20260728.md` §2）
//! 逐条计出 20 个责任点中 **11 个对象无关**。本模块承载那 11 个，域侧只留自己的判据。
//!
//! # 11 个责任点与落位
//!
//! | # | 责任点 | 落位 |
//! |---:|---|---|
//! | 1 | per-key 注册表 | [`LedgerBook`] 的 `entries: BTreeMap<K, Entry>` + `get/contains/len/keys` |
//! | 2 | 首次观察建项 | [`LedgerBook::open_on_observation`] |
//! | 3 | append-only 修订追加 | [`LedgerEntryCore::push_revision`]（内核默认实现，唯一追加点） |
//! | 4 | 修订计数与历史一致 | 同上：计数由留档长度现算；凡经 `push_revision`/`settle` 写入路径不会不一致，`assert_invariants` 调用点逮住绕过 |
//! | 5 | per-identity 倒退拒绝 | [`LedgerBook::admit`] / [`LedgerBook::reject_retrograde`] |
//! | 6 | 终态吸收 | [`LedgerState::is_terminal`] + `admit` 的 [`LedgerAdmission::TerminalAbsorbed`] |
//! | 7 | 钟首次写入不后移 | [`first_write_clock`] |
//! | 8 | 每次推进返回增量 | 全部产修订原语返回 [`LedgerRevision`] + [`LedgerDelta`] 收集器 |
//! | 9 | 身份迁移保留历史 | [`LedgerBook::migrate`] |
//! | 10 | 只读枚举/过滤门户 | [`LedgerBook::entries`]/`values`/[`LedgerBook::in_state`] |
//! | 11 | 不变量骨架 | [`LedgerBook::assert_core_invariants`] |
//!
//! # 泛化面：四组类型参数（spec `spec-kernel-and-retrace-ledger-20260728.md` T1 章）
//!
//! - **Key**（[`LedgerPolicy::Key`]）：等值 + 序；
//! - **Observation**（[`LedgerPolicy::Observation`]）：建项入口的输入，经
//!   [`LedgerPolicy::observation_key`] 投影到身份；
//! - **TransitionPolicy**：判据/拒绝/终态映射——判据留域侧，内核只提供**拒绝**
//!   （倒退 + 终态吸收，[`LedgerAdmission`]）与**终态落账**（[`LedgerSettlement`]）两个机制；
//! - **ReasonPayload**（[`LedgerPolicy::Reason`] + [`LedgerPolicy::Evidence`]）：原因码 +
//!   证据载荷，随终态落账与修订一并留存。
//!
//! 双消费方（防早产抽象，#465 resolution 命门条款）：nest 活假设（本仓已落地）与买卖点身份
//! 账本（#574 语义契约 §「对 T1 内核定形的含义」；#575 实装）。
//!
//! # 边界（不进内核的东西）
//!
//! 域概念一律不得出现在本模块的签名或命名中——身份的**判同规则**、业务成立/否证条件、
//! 具体原因码含义、域钟语义、观察产出方、力度/结构判据、结算策略全在域侧。内核只知道
//! 「有身份、有状态、有 append-only 留档、有时间单调纪律」。

use std::collections::BTreeMap;
use std::fmt;

#[cfg(test)]
mod tests;

// ═══════════════════════════════════════════════════════════════════════════
// 通用三态
// ═══════════════════════════════════════════════════════════════════════════

/// 账本通用三态：未决 / 成立 / 失效。
///
/// 三态是**账本纪律**而非域语义：域给出何时进入哪一态（判据）与该态的名分（原因码），
/// 内核只负责「终态吸收」——进入终态后同身份任何后续输入零输出、禁复活。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LedgerState {
    /// 未决：默认态，尚未落锤。
    Provisional,
    /// 成立：正向终态。
    Confirmed,
    /// 失效：负向终态族（具体名分由域的原因码承担）。
    Invalidated,
}

impl LedgerState {
    /// 终态（`Confirmed`/`Invalidated`）⟹ 吸收：同身份后续输入零输出。
    pub fn is_terminal(self) -> bool {
        matches!(self, LedgerState::Confirmed | LedgerState::Invalidated)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 泛化面：四组类型参数
// ═══════════════════════════════════════════════════════════════════════════

/// 账本泛化面：域侧实现本 trait 即获得全部 11 个内核责任点。
pub trait LedgerPolicy: Sized {
    /// 身份键：等值 + 序（注册表定序 ⟹ 枚举确定性，无平局歧义）。
    type Key: Copy + Ord + fmt::Debug;
    /// 观察：建项入口的输入。
    type Observation;
    /// 修订词汇（域）。
    type RevisionKind: Copy + PartialEq + fmt::Debug;
    /// 原因码（ReasonPayload 之一半）。
    type Reason: Copy + PartialEq + fmt::Debug;
    /// 证据载荷（ReasonPayload 之另一半）。
    type Evidence: Copy + PartialEq + fmt::Debug;
    /// 账本条目：域自选字段布局，经 [`LedgerEntryCore`] 暴露内核所需的通用面。
    type Entry: LedgerEntryCore<Self>;

    /// 观察 → 身份键（per-key 路由的唯一入口）。
    fn observation_key(observation: &Self::Observation) -> Self::Key;

    /// 首建条目所记的修订词汇。
    fn opened_revision_kind() -> Self::RevisionKind;
}

/// 终态落账载荷：终态 + 修订词汇 + 原因码 + 证据。
pub struct LedgerSettlement<P: LedgerPolicy> {
    /// 目标终态；非终态 ⟹ [`LedgerEntryCore::settle`] fail-loud。
    pub state: LedgerState,
    /// 该次落锤所记的修订词汇。
    pub kind: P::RevisionKind,
    /// 原因码（正向终态可无原因）。
    pub reason: Option<P::Reason>,
    /// 证据载荷（材料缺则诚实 `None`）。
    pub evidence: Option<P::Evidence>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 修订记录 / 倒退注记 / 准入裁决
// ═══════════════════════════════════════════════════════════════════════════

/// 一条修订：留档 + 增量返回双通道；只追加，无删除路径。
pub struct LedgerRevision<P: LedgerPolicy> {
    /// 追加时条目的当前身份键（迁移后为新键）。
    pub key: P::Key,
    pub kind: P::RevisionKind,
    /// 产生该修订的知情时。
    pub as_of: usize,
    pub evidence: Option<P::Evidence>,
}

/// 倒退喂入的显式拒绝注记（禁静默吸收）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerRetrogradeRejection<K> {
    pub key: K,
    /// 该身份已见最大知情时。
    pub last_as_of: usize,
    /// 被拒绝的倒退知情时。
    pub rejected_as_of: usize,
}

/// 已建仓身份的推进准入裁决（责任点 5 + 6 的合成出口）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerAdmission {
    /// 准入：门卫钟已前移，域可继续推进。
    Accepted,
    /// 时间倒退：显式拒绝并记注记，条目零改写。
    RetrogradeRejected,
    /// 终态吸收：门卫钟已前移，但禁复活 ⟹ 零输出。
    TerminalAbsorbed,
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 7：钟首次写入不后移
// ═══════════════════════════════════════════════════════════════════════════

/// 钟首写纪律：空槽落笔返回 `true`；已写则**既不后移也不前移**，返回 `false`。
///
/// 通用账本纪律——出生钟/落锤钟一经写入即为历史事实，改写等于篡改历史。
pub fn first_write_clock(slot: &mut Option<usize>, as_of: usize) -> bool {
    if slot.is_some() {
        return false;
    }
    *slot = Some(as_of);
    true
}

// ═══════════════════════════════════════════════════════════════════════════
// 责任点 8：推进增量
// ═══════════════════════════════════════════════════════════════════════════

/// 推进增量累加器：消费方按增量消费，不重扫全账（留档仍在账本内，增量不替代留档）。
pub struct LedgerDelta<P: LedgerPolicy> {
    revisions: Vec<LedgerRevision<P>>,
}

impl<P: LedgerPolicy> LedgerDelta<P> {
    pub fn new() -> Self {
        Self {
            revisions: Vec::new(),
        }
    }

    /// 记一条本轮新增修订（记录序即产生序）。
    pub fn record(&mut self, revision: LedgerRevision<P>) {
        self.revisions.push(revision);
    }

    /// 记一条可选修订（`None` = 本轮该处无新事实）。
    pub fn record_opt(&mut self, revision: Option<LedgerRevision<P>>) {
        if let Some(revision) = revision {
            self.record(revision);
        }
    }

    pub fn len(&self) -> usize {
        self.revisions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.revisions.is_empty()
    }

    pub fn as_slice(&self) -> &[LedgerRevision<P>] {
        &self.revisions
    }

    pub fn into_vec(self) -> Vec<LedgerRevision<P>> {
        self.revisions
    }
}

impl<P: LedgerPolicy> Default for LedgerDelta<P> {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 条目通用面（责任点 3/4 的内核默认实现在此）
// ═══════════════════════════════════════════════════════════════════════════

/// 条目通用面：域自选字段布局，经本 trait 暴露内核所需的读写口。
///
/// 追加与落锤两条写入路径由内核默认实现独占（[`push_revision`](Self::push_revision) /
/// [`settle`](Self::settle)）——凡经这两条路径写入的修订与计数不会不一致；`assert_invariants`
/// 调用点负责逮住任何绕过路径造成的不一致。
pub trait LedgerEntryCore<P: LedgerPolicy>: Sized {
    /// 建空白条目：状态 `Provisional`、两钟同取 `as_of`、修订计数 0、留档空、无来源链。
    fn open(key: P::Key, as_of: usize) -> Self;

    fn key(&self) -> P::Key;
    fn set_key(&mut self, key: P::Key);
    fn state(&self) -> LedgerState;
    fn set_state(&mut self, state: LedgerState);
    fn revision_count(&self) -> u32;
    fn set_revision_count(&mut self, count: u32);
    fn revisions(&self) -> &[LedgerRevision<P>];
    fn revisions_mut(&mut self) -> &mut Vec<LedgerRevision<P>>;
    /// 出生钟（首写不改）。
    fn opened_at(&self) -> usize;
    /// 门卫钟：该身份已见最大知情时。
    fn last_as_of(&self) -> usize;
    fn set_last_as_of(&mut self, as_of: usize);
    /// 落锤钟（域可按终态分列多只字段，此处给合并读数）。
    fn settled_at(&self) -> Option<usize>;
    fn migrated_from(&self) -> Option<P::Key>;
    fn set_migrated_from(&mut self, from: P::Key);
    /// 终态载荷落账：域按自身字段布局写落锤钟、原因码与证据。
    fn write_settlement(
        &mut self,
        state: LedgerState,
        reason: Option<P::Reason>,
        as_of: usize,
        evidence: Option<P::Evidence>,
    );

    /// 责任点 3/4：append-only 追加一条修订，计数由留档长度现算。**全内核唯一追加点。**
    fn push_revision(
        &mut self,
        kind: P::RevisionKind,
        as_of: usize,
        evidence: Option<P::Evidence>,
    ) -> LedgerRevision<P> {
        let revision = LedgerRevision {
            key: self.key(),
            kind,
            as_of,
            evidence,
        };
        self.revisions_mut().push(revision);
        let count = u32::try_from(self.revisions().len()).expect("修订留档长度溢出 u32");
        self.set_revision_count(count);
        revision
    }

    /// 终态落账：置终态 → 域写落锤钟/原因/证据 → 追加一条修订。**全内核唯一转终态点。**
    fn settle(&mut self, settlement: LedgerSettlement<P>, as_of: usize) -> LedgerRevision<P> {
        assert!(!self.state().is_terminal(), "终态禁再落账（禁复活）");
        assert!(
            settlement.state.is_terminal(),
            "终态落账要求终态：{:?}",
            settlement.state
        );
        self.set_state(settlement.state);
        self.write_settlement(
            settlement.state,
            settlement.reason,
            as_of,
            settlement.evidence,
        );
        self.push_revision(settlement.kind, as_of, settlement.evidence)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 账本容器
// ═══════════════════════════════════════════════════════════════════════════

/// per-key 注册表 + 倒退注记面。
pub struct LedgerBook<P: LedgerPolicy> {
    entries: BTreeMap<P::Key, P::Entry>,
    retrograde_rejections: Vec<LedgerRetrogradeRejection<P::Key>>,
}

impl<P: LedgerPolicy> LedgerBook<P> {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            retrograde_rejections: Vec::new(),
        }
    }

    // ── 责任点 1/10：注册表与只读枚举/过滤门户 ──

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, key: &P::Key) -> bool {
        self.entries.contains_key(key)
    }

    pub fn get(&self, key: &P::Key) -> Option<&P::Entry> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &P::Key) -> Option<&mut P::Entry> {
        self.entries.get_mut(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &P::Key> {
        self.entries.keys()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&P::Key, &P::Entry)> {
        self.entries.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &P::Entry> {
        self.entries.values()
    }

    /// 按态过滤门户：只放该态的条目（留档全部仍在账本内，过滤不删除）。
    pub fn in_state(&self, state: LedgerState) -> Vec<&P::Entry> {
        self.entries
            .values()
            .filter(|entry| entry.state() == state)
            .collect()
    }

    /// 倒退拒绝注记面（审计只读）。
    pub fn retrograde_rejections(&self) -> &[LedgerRetrogradeRejection<P::Key>] {
        &self.retrograde_rejections
    }

    // ── 责任点 2：首次观察建项 ──

    /// 首次观察建项：已建仓 ⟹ `None` 且零改写；否则建仓并记一条建项修订。
    pub fn open_on_observation(
        &mut self,
        observation: &P::Observation,
        as_of: usize,
    ) -> Option<LedgerRevision<P>> {
        let key = P::observation_key(observation);
        if self.entries.contains_key(&key) {
            return None;
        }
        let mut entry = P::Entry::open(key, as_of);
        let revision = entry.push_revision(P::opened_revision_kind(), as_of, None);
        self.entries.insert(key, entry);
        Some(revision)
    }

    // ── 责任点 5/6：倒退拒绝 + 终态吸收 ──

    /// 记一条倒退拒绝注记（被拒键可尚未建仓——判同分支在建仓前即可拒）。
    pub fn reject_retrograde(&mut self, key: P::Key, last_as_of: usize, rejected_as_of: usize) {
        self.retrograde_rejections.push(LedgerRetrogradeRejection {
            key,
            last_as_of,
            rejected_as_of,
        });
    }

    /// 已建仓身份的推进准入：倒退拒绝（零改写 + 注记）→ 门卫钟前移 → 终态吸收。
    pub fn admit(&mut self, key: &P::Key, as_of: usize) -> LedgerAdmission {
        let last_as_of = self
            .entries
            .get(key)
            .expect("准入要求条目已建仓")
            .last_as_of();
        if as_of < last_as_of {
            self.reject_retrograde(*key, last_as_of, as_of);
            return LedgerAdmission::RetrogradeRejected;
        }
        let entry = self.entries.get_mut(key).expect("准入要求条目已建仓");
        entry.set_last_as_of(as_of);
        if entry.state().is_terminal() {
            return LedgerAdmission::TerminalAbsorbed;
        }
        LedgerAdmission::Accepted
    }

    // ── 责任点 9：身份迁移保留历史 ──

    /// 同一身份换键：旧键退出注册表 → 改键 → 来源链留痕 → 追加迁移修订 → 新键入表。
    ///
    /// **一切钟与既有留档一个 bit 不动**——迁移是「同对象换键留史」，不是新建对象。
    /// 判同规则属域侧：内核只在域已判定同身份后执行搬运。
    pub fn migrate(
        &mut self,
        from: P::Key,
        to: P::Key,
        kind: P::RevisionKind,
        as_of: usize,
    ) -> LedgerRevision<P> {
        assert!(from != to, "身份迁移两端不得同键：{from:?}");
        assert!(
            !self.entries.contains_key(&to),
            "身份迁移目标键须空闲：{to:?}"
        );
        let mut entry = self.entries.remove(&from).expect("迁移源键存在");
        entry.set_migrated_from(from);
        entry.set_key(to);
        let revision = entry.push_revision(kind, as_of, None);
        self.entries.insert(to, entry);
        revision
    }

    // ── 责任点 11：不变量骨架 ──

    /// 账本级不变量（对象无关部分）：注册表键一致、计数 == 留档、钟序、终态 ⟺ 终态钟、
    /// 留档时序非降、首条修订为建项词汇、来源链不自环。
    ///
    /// 域侧在此之上加自己的不变量（原因码互补、域钟序等），两层互不替代。
    pub fn assert_core_invariants(&self) {
        for (key, entry) in &self.entries {
            assert!(*key == entry.key(), "注册表键 == 条目键：{key:?}");
            assert_eq!(
                entry.revision_count() as usize,
                entry.revisions().len(),
                "修订计数 == 留档长度：{key:?}"
            );
            assert!(
                entry.opened_at() <= entry.last_as_of(),
                "出生钟 ≤ 门卫钟：{key:?}"
            );
            assert_eq!(
                entry.state().is_terminal(),
                entry.settled_at().is_some(),
                "终态 ⟺ 终态钟：{key:?}"
            );
            if let Some(settled) = entry.settled_at() {
                assert!(entry.opened_at() <= settled, "出生钟 ≤ 终态钟：{key:?}");
            }
            if let Some(from) = entry.migrated_from() {
                assert!(from != entry.key(), "身份来源链不自环：{key:?}");
            }
            Self::assert_history_invariants(key, entry);
        }
    }

    /// 单条留档链的不变量：非空、首条为建项词汇、知情时非降（append-only 时间纪律）。
    fn assert_history_invariants(key: &P::Key, entry: &P::Entry) {
        let history = entry.revisions();
        let first = history.first().expect("建仓即有一条建项修订：{key:?}");
        assert!(
            first.kind == P::opened_revision_kind(),
            "首条修订为建项词汇：{key:?}"
        );
        let mut prior = entry.opened_at();
        for revision in history {
            assert!(
                revision.as_of >= prior,
                "留档时序非降：{key:?} {prior} → {}",
                revision.as_of
            );
            prior = revision.as_of;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 容器 trait 手写实现
// ═══════════════════════════════════════════════════════════════════════════
//
// `derive` 会给泛型参数 `P` 加 bound（要求 `P: Clone` 等），但 `P` 是零尺寸的策略标记，
// 真正需要约束的是关联类型。故逐个手写，bound 只落在 `P::Key`/`P::Entry` 等实际字段上。

impl<P: LedgerPolicy> Clone for LedgerRevision<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: LedgerPolicy> Copy for LedgerRevision<P> {}

impl<P: LedgerPolicy> PartialEq for LedgerRevision<P> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.kind == other.kind
            && self.as_of == other.as_of
            && self.evidence == other.evidence
    }
}

impl<P: LedgerPolicy> fmt::Debug for LedgerRevision<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LedgerRevision")
            .field("key", &self.key)
            .field("kind", &self.kind)
            .field("as_of", &self.as_of)
            .field("evidence", &self.evidence)
            .finish()
    }
}

impl<P: LedgerPolicy> Clone for LedgerSettlement<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: LedgerPolicy> Copy for LedgerSettlement<P> {}

impl<P: LedgerPolicy> fmt::Debug for LedgerSettlement<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LedgerSettlement")
            .field("state", &self.state)
            .field("kind", &self.kind)
            .field("reason", &self.reason)
            .field("evidence", &self.evidence)
            .finish()
    }
}

impl<P: LedgerPolicy> Clone for LedgerBook<P>
where
    P::Entry: Clone,
{
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            retrograde_rejections: self.retrograde_rejections.clone(),
        }
    }
}

impl<P: LedgerPolicy> PartialEq for LedgerBook<P>
where
    P::Entry: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries && self.retrograde_rejections == other.retrograde_rejections
    }
}

impl<P: LedgerPolicy> fmt::Debug for LedgerBook<P>
where
    P::Entry: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LedgerBook")
            .field("entries", &self.entries)
            .field("retrograde_rejections", &self.retrograde_rejections)
            .finish()
    }
}

impl<P: LedgerPolicy> Default for LedgerBook<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: LedgerPolicy> Clone for LedgerDelta<P> {
    fn clone(&self) -> Self {
        Self {
            revisions: self.revisions.clone(),
        }
    }
}

impl<P: LedgerPolicy> fmt::Debug for LedgerDelta<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LedgerDelta")
            .field("revisions", &self.revisions)
            .finish()
    }
}
