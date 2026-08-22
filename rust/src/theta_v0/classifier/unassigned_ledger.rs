//! 孤儿段「方案 C——显式追加式 `Unassigned` 账本」生产化（票 #1160；map #1094 图外实施票）。
//!
//! ## 语义来源（自包含）
//!
//! - Q5 方案 C 裁决：`chanlun/escalate/c-ruling-decision-20260713.md:15-19`——禁止沉默孤儿（方案 A）
//!   与无证据强制吸收（方案 B）；账本按 `(target_level, lower_id)` per-level 建账；assignment 与
//!   host supersede 用同一 correlation 原子追加、旧行不覆盖；区间套只消费 Assigned+Completed，
//!   coverage gap 显式返回；`ORPHAN_TOMBSTONED` 仅在 #61 §4.3 严格合取条件下允许。
//! - 33 口径验收：`chanlun/escalate/adopt-overlay-residual-ruling-20260713.md`——`adopt_v1` 生产验收
//!   `52→33；+19 获 host；其中 7 Completed；regressions=0`；旧 DP 重组口径 `residual=35 / regressions=2`
//!   仅保留为 `legacy_recompose_*` 兼容诊断字段。
//! - 生产化 spec（675 行草案）：`chanlun/review-results/unassigned-ledger-production-spec-20260713.md`
//!   （task #63 旧编号；已随 #504 归档出仓，正文见 git `1e4763a341`）——B0→B6 实现顺序、墓碑 T1–T5、
//!   10 条 prefix property、§2 行级不变量与 §3 状态机。
//!
//! ## Seam（零消费接线）
//!
//! 本模块是**纯产出、零消费接线**的只读账本内核（先例：#641 `chain_cert` / #550 `cand_event` /
//! #981 `consume_at`）：现役分类/信号/交易路径不调用本模块（spec §9.3 硬门 6——迁移后不得绕过
//! `LevelAsOfView` 自行拼 snapshot，迁移在后续票）。左半边现役对象（递归塔 / 分类器）**零改**：
//! 只读复用 `recursive_tower::ElementId`（`(level, ordinal)` 全量/增量跨 bar 稳定身份，锚
//! `recursive_tower.rs:101-108`），不 mock 现役对象内部、不改塔、不改 Lean。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 实装本身 = L1（bit-exact 一致性：账本归约确定性、完全分类守恒、前缀稳定与 append-only 纪律的
//! 管线正确性验证）。**不声称 L2/L3**——孤儿能否转正取决于组装器规则演化（#53/#55），本账本
//! 解决的是记账语义（消灭幽灵状态、计数可重放、可审计完整），不改变孤儿段的客观存在（裁决定位）。
//!
//! ## 本票交付边界（spec §9.2 B0/B1 + §6.2 prefix 面）
//!
//! - B0：冻结类型族、错误枚举与 fixture（`recursive_tower.rs` 零 diff）。
//! - B1：落地 `UnassignedEvent` reducer（不接生产 consumer）：状态机 property、previous/generation/
//!   reason exhaustive、T1–T5 逐项负测、`N=A+U+T` 守恒。
//! - 本票新增的 §6.2 十条 prefix property 全部落在测试子树，经公开 `reduce_unassigned` /
//!   `assemble_ledger` 两 seam 穿过。
//!
//! 尚未落地（后续票，按 B2→B6 顺序）：#58 组装器 seam 收敛（B2）、Move 账本全量 supersede 与
//! correlation 原子批次生产 adapter（B3，本票只落 `MoveEvent` 形状 + `validate_correlation_batches`
//! 两向完备校验）、coverage-gap 区间套只读迁移（B4）、#54/#60 重放 bridge（B5）、consumer 迁移（B6）。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use super::recursive_tower::ElementId;

// ═══════════════════════════════════════════════════════════════════════════
// 冻结常量
// ═══════════════════════════════════════════════════════════════════════════

/// 账本 schema 版本（wire/storage 名称锚，非序列化布局冻结）。
pub const SCHEMA_VERSION: &str = "unassigned-ledger-v1";
/// 冻结 fixture 的 rule version（#61 最小 schema 骨架的组装规则版）。
pub const LEDGER_RULE_VERSION: u32 = 1;
/// 冻结 fixture 的 as_of（BTC 1m 4613599 bar 全历史末端，与 p68 原型同源）。
pub const FIXTURE_AS_OF: usize = 4_613_598;
/// 旧 DP 重组口径残留（`legacy_recompose_residual`）：33 + 2（回退释放 L0#33574 / L0#37199）。
/// 锚：`chanlun/escalate/adopt-overlay-residual-ruling-20260713.md`。
pub const LEGACY_RECOMPOSE_RESIDUAL: usize = 35;
/// 旧 DP 重组口径回退数（`legacy_recompose_regressions`）。
pub const LEGACY_RECOMPOSE_REGRESSIONS: usize = 2;

// ═══════════════════════════════════════════════════════════════════════════
// 基础类型族（spec §2.1 草案定稿）
// ═══════════════════════════════════════════════════════════════════════════

/// 账本外层命名空间：隔离不同数据宇宙，但不改变裁决指定的逻辑键（spec §0.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerScope {
    pub instrument: &'static str,
    pub timeframe: &'static str,
    pub partition: &'static str,
    pub dataset: &'static str,
}

/// 冻结 BTC 1m 全历史 fixture 的 scope（p68/p72 原型同源）。
pub const BTC_SCOPE: LedgerScope = LedgerScope {
    instrument: "BTC",
    timeframe: "1m",
    partition: "full-history",
    dataset: "btc-1m-4613599",
};

/// 组装规则版本（spec §0.3：墓碑只对其事件所写版本成立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleVersion(pub u32);

/// Move 身份（opaque；最终编码由 Move 事件 spec 冻结——spec §2.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MoveId(pub u128);

/// 同一原子追加批次的 correlation 标识（spec §2.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorrelationId(pub u128);

/// source 坐标区间（`SourceIndex = usize`，spec §2.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}

/// 账本逻辑键：scope 内的 `(target_level, lower_id)`（spec §0.3）。
///
/// `rule_version` **不**入键——同一 key 跨版本重审必须显式 `REOPENED(generation+1)`，
/// 不能在键里另开一行（spec §3.2 转移表 TOMBSTONED→REOPENED 行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnassignedKey {
    pub scope: LedgerScope,
    pub target_level: u32,
    pub lower_id: ElementId,
}

/// `ElementId` 只派生 `Eq/Hash`（`recursive_tower.rs:101`，塔身份无全序需求），故本键手写
/// `Ord`：按 `(scope, target_level, lower_id.level, lower_id.ordinal)` 字典序，供 `BTreeMap` 定序。
impl PartialOrd for UnassignedKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UnassignedKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.scope,
            self.target_level,
            self.lower_id.level,
            self.lower_id.ordinal,
        )
            .cmp(&(
                other.scope,
                other.target_level,
                other.lower_id.level,
                other.lower_id.ordinal,
            ))
    }
}

/// 事件身份：key + 全生命周期追加序号（spec §0.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnassignedEventId {
    pub key: UnassignedKey,
    pub sequence: u32,
}

/// host 完成快照（spec §2.1：`Assigned` 不等于 `Completed`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HostCompletionAtEvent {
    Pending,
    Completed,
}

/// host 引用（spec §2.1）。`level == key.target_level`；membership proof 是未来 Move supersede
/// 账本的 opaque 类型（B3），本票只冻结 `level` 一致性 + `range` 有效性两个可机检字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostRef {
    pub move_id: MoveId,
    pub move_version: u32,
    pub level: u32,
    pub range: SourceRange,
    pub completion_at_event: HostCompletionAtEvent,
}

/// 四值封闭的事件类型（spec §3.1：持久化行枚举保持四值封闭，转移动作由 `reason` 命名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnassignedEventType {
    /// wire/storage name: `OPENED`
    Opened,
    /// wire/storage name: `ASSIGNED`
    Assigned,
    /// wire/storage name: `TOMBSTONED`
    Tombstoned,
    /// wire/storage name: `REOPENED`
    Reopened,
}

/// 十个 reason code 的封闭全集（spec §3.3）。没有 `Unknown`/自由文本 reason；新增 reason 必须
/// 提升 `rule_version`、补 reducer exhaustive match 和迁移测试（spec §2.2 行级不变量 10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReasonCode {
    CursorGapRecomposable,
    NoContainingSeedWindow,
    LeadingResidual,
    TerminalInsufficient,
    NoHostAtAsOf,
    LegalHostEstablished,
    HostSupersededAdopted,
    HostSupersededReleased,
    FinalizedNoLegalHost,
    RuleVersionReevaluation,
}

impl ReasonCode {
    fn code(self) -> &'static str {
        match self {
            Self::CursorGapRecomposable => "CURSOR_GAP_RECOMPOSABLE",
            Self::NoContainingSeedWindow => "NO_CONTAINING_SEED_WINDOW",
            Self::LeadingResidual => "LEADING_RESIDUAL",
            Self::TerminalInsufficient => "TERMINAL_INSUFFICIENT",
            Self::NoHostAtAsOf => "NO_HOST_AT_AS_OF",
            Self::LegalHostEstablished => "LEGAL_HOST_ESTABLISHED",
            Self::HostSupersededAdopted => "HOST_SUPERSEDED_ADOPTED",
            Self::HostSupersededReleased => "HOST_SUPERSEDED_RELEASED",
            Self::FinalizedNoLegalHost => "FINALIZED_NO_LEGAL_HOST",
            Self::RuleVersionReevaluation => "RULE_VERSION_REEVALUATION",
        }
    }

    fn is_open_reason(self) -> bool {
        matches!(
            self,
            Self::CursorGapRecomposable
                | Self::NoContainingSeedWindow
                | Self::LeadingResidual
                | Self::TerminalInsufficient
                | Self::NoHostAtAsOf
        )
    }
}

/// 墓碑五合取证据（spec §7.2 T1–T5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TombstoneEvidence {
    /// T1/T5 输入之一：lower settled 时点。
    pub lower_settled_at: usize,
    /// T2/T5 输入之一：后继完成事件封闭时点。
    pub successor_closed_at: usize,
    /// T3/T5 输入之一：坐标最终化水位时点。
    pub coordinate_finalized_at: usize,
    /// T4/T5 输入之一：穷举无合法 host 检查时点。
    pub exhaustive_check_at: usize,
    /// T1：lower 已 settled。
    pub lower_settled: bool,
    /// T2：后继完成事件已封闭候选归属区间。
    pub successor_closed: bool,
    /// T3：显式坐标最终化水位越过全部候选边界。
    pub coordinate_finalized: bool,
    /// T4：穷举无合法 host 且无 PendingMove 跨越 lower。
    pub no_legal_or_pending_host: bool,
}

impl TombstoneEvidence {
    /// T5：judge_at 取证据最晚时点的精确最大值（`>=` 而非 `==` 也拒绝）。
    pub fn expected_judge_at(self) -> usize {
        self.lower_settled_at
            .max(self.successor_closed_at)
            .max(self.coordinate_finalized_at)
            .max(self.exhaustive_check_at)
    }

    /// T1 ∧ T2 ∧ T3 ∧ T4 ∧ T5 全真才放行（spec §7.2）。
    fn all_conjuncts(self, judge_at: usize) -> bool {
        self.lower_settled
            && self.successor_closed
            && self.coordinate_finalized
            && self.no_legal_or_pending_host
            && judge_at == self.expected_judge_at()
    }
}

/// 一条追加事实行（spec §2.1）。`evidence` 全量 payload（`EvidenceRef`）是未来 Move supersede
/// 账本的 opaque 类型（B3），本票冻结 reducer 可机检的 `tombstone_evidence`（T1–T5）与
/// `correlation_id`（原子批次标识）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnassignedEvent {
    pub key: UnassignedKey,
    pub sequence: u32,
    pub generation: u32,
    pub lower_range: SourceRange,
    pub event_type: UnassignedEventType,
    pub reason: ReasonCode,
    pub judge_at: usize,
    pub host: Option<HostRef>,
    pub previous_event_id: Option<UnassignedEventId>,
    pub correlation_id: Option<CorrelationId>,
    pub rule_version: RuleVersion,
    pub tombstone_evidence: Option<TombstoneEvidence>,
}

impl UnassignedEvent {
    pub fn event_id(&self) -> UnassignedEventId {
        UnassignedEventId {
            key: self.key,
            sequence: self.sequence,
        }
    }

    /// 单行规范编码（append-only 字节稳定校验用，spec §6.2 property 7）：确定性字段顺序，
    /// 无 HashMap 迭代，供测试记录既有行字节并在追加未来批次后逐位比对。
    pub fn canonical_row(&self) -> Vec<u8> {
        let host = match self.host {
            Some(h) => format!(
                "{}@{}:L{}:{}-{}:{}",
                h.move_id.0,
                h.move_version,
                h.level,
                h.range.start,
                h.range.end,
                match h.completion_at_event {
                    HostCompletionAtEvent::Pending => "PENDING",
                    HostCompletionAtEvent::Completed => "COMPLETED",
                }
            ),
            None => "-".to_owned(),
        };
        let previous = match self.previous_event_id {
            Some(prev) => format!(
                "{}:{}:{}:{}:{}",
                prev.key.scope.instrument,
                prev.key.target_level,
                prev.key.lower_id.level,
                prev.key.lower_id.ordinal,
                prev.sequence
            ),
            None => "-".to_owned(),
        };
        let correlation = self
            .correlation_id
            .map(|c| c.0.to_string())
            .unwrap_or_else(|| "-".to_owned());
        let tombstone = self
            .tombstone_evidence
            .map(|e| {
                format!(
                    "{},{},{},{},{},{},{},{}",
                    e.lower_settled_at,
                    e.successor_closed_at,
                    e.coordinate_finalized_at,
                    e.exhaustive_check_at,
                    e.lower_settled,
                    e.successor_closed,
                    e.coordinate_finalized,
                    e.no_legal_or_pending_host
                )
            })
            .unwrap_or_else(|| "-".to_owned());
        let event_type = match self.event_type {
            UnassignedEventType::Opened => "OPENED",
            UnassignedEventType::Assigned => "ASSIGNED",
            UnassignedEventType::Tombstoned => "TOMBSTONED",
            UnassignedEventType::Reopened => "REOPENED",
        };
        format!(
            "event|{}:{}:{}:{}|target={}|lower=L{}#{}|rule={}|range={}-{}|type={}|reason={}|seq={}|generation={}|judge_at={}|host={}|prev={}|correlation={}|tombstone={}",
            self.key.scope.instrument,
            self.key.scope.timeframe,
            self.key.scope.partition,
            self.key.scope.dataset,
            self.key.target_level,
            self.key.lower_id.level,
            self.key.lower_id.ordinal,
            self.rule_version.0,
            self.lower_range.start,
            self.lower_range.end,
            event_type,
            self.reason.code(),
            self.sequence,
            self.generation,
            self.judge_at,
            host,
            previous,
            correlation,
            tombstone,
        )
        .into_bytes()
    }
}

/// Move supersede 账本的最小事件形状（B3 seam；spec §2.1 的 `MoveEventId` 尚未冻结序列化布局）。
/// 本票只用于 correlation 两向完备校验（spec §2.2 行级不变量 9 / §9.3 硬门 4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveEventKind {
    Created,
    SupersedeAdopted,
    SupersedeReleased,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveEvent {
    pub correlation_id: Option<CorrelationId>,
    pub judge_at: usize,
    pub rule_version: RuleVersion,
    pub host: HostRef,
    pub kind: MoveEventKind,
}

// ═══════════════════════════════════════════════════════════════════════════
// 完全分类 disposition 与视图
// ═══════════════════════════════════════════════════════════════════════════

/// 三值完全分类（spec §1.2）：任一 visible lower 恰落一类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    Assigned,
    Unassigned,
    Tombstoned,
}

impl Disposition {
    fn code(self) -> &'static str {
        match self {
            Self::Assigned => "ASSIGNED",
            Self::Unassigned => "UNASSIGNED",
            Self::Tombstoned => "TOMBSTONED",
        }
    }
}

/// 单 key 的当前视图（派生、可重算；禁反写为历史事实——spec §0.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    pub disposition: Disposition,
    pub generation: u32,
    pub lower_range: SourceRange,
    pub host: Option<HostRef>,
    pub last_reason: ReasonCode,
    pub last_sequence: u32,
    pub last_judge_at: usize,
    pub last_rule_version: RuleVersion,
}

/// as-of 纯归约视图（spec §4.1 `LevelAsOfView` 的账本分量）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerView {
    pub scope: LedgerScope,
    pub as_of: usize,
    pub rule_version: RuleVersion,
    pub entries: BTreeMap<UnassignedKey, EntryView>,
    pub supersede_edges: BTreeMap<HostRef, HostRef>,
}

impl LedgerView {
    /// 完全分类计数：`(assigned, unassigned, tombstoned)`（spec §1.2 守恒 `N=A+U+T`）。
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut assigned = 0;
        let mut unassigned = 0;
        let mut tombstoned = 0;
        for entry in self.entries.values() {
            match entry.disposition {
                Disposition::Assigned => assigned += 1,
                Disposition::Unassigned => unassigned += 1,
                Disposition::Tombstoned => tombstoned += 1,
            }
        }
        (assigned, unassigned, tombstoned)
    }

    /// 规范终态编码：逐 key、逐 supersede 边稳定排序，无 HashMap 迭代顺序，供 SHA-256 bit-exact
    /// 双重放与 append-only 字节稳定校验（spec §6.2 property 7）。
    pub fn canonical_terminal_state(&self) -> Vec<u8> {
        let mut out = String::new();
        writeln!(
            &mut out,
            "schema={} scope={}:{}:{}:{} as_of={} rule_version={}",
            SCHEMA_VERSION,
            self.scope.instrument,
            self.scope.timeframe,
            self.scope.partition,
            self.scope.dataset,
            self.as_of,
            self.rule_version.0
        )
        .expect("write String");
        for (key, entry) in &self.entries {
            let host = match entry.host {
                Some(host) => format!(
                    "{}@{}:L{}:{}-{}:{}",
                    host.move_id.0,
                    host.move_version,
                    host.level,
                    host.range.start,
                    host.range.end,
                    match host.completion_at_event {
                        HostCompletionAtEvent::Completed => "COMPLETED",
                        HostCompletionAtEvent::Pending => "PENDING",
                    }
                ),
                None => "-".to_owned(),
            };
            writeln!(
                &mut out,
                "entry|{}|{}|{}|{}|target={}|lower=L{}#{}|rule={}|range={}-{}|state={}|generation={}|host={}|reason={}|last_seq={}|judge_at={}",
                key.scope.instrument,
                key.scope.timeframe,
                key.scope.partition,
                key.scope.dataset,
                key.target_level,
                key.lower_id.level,
                key.lower_id.ordinal,
                entry.last_rule_version.0,
                entry.lower_range.start,
                entry.lower_range.end,
                entry.disposition.code(),
                entry.generation,
                host,
                entry.last_reason.code(),
                entry.last_sequence,
                entry.last_judge_at,
            )
            .expect("write String");
        }
        for (old, new) in &self.supersede_edges {
            writeln!(
                &mut out,
                "supersede|{}@{}->{}@{}",
                old.move_id.0, old.move_version, new.move_id.0, new.move_version
            )
            .expect("write String");
        }
        out.into_bytes()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 错误与归约
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    /// PL-1：跳级/跨级键（`lower_id.level + 1 != target_level`）。
    CrossLevelKey,
    /// 行级不变量 3：range 非法。
    InvalidRange,
    /// PL-5：sequence 断链或 previous_event_id 不接续。
    BrokenEventChain,
    /// 表外转移（spec §3.2 转移表全集）。
    InvalidTransition,
    /// 行级不变量 6/7：ASSIGNED/REOPENED 缺 correlation_id。
    MissingAtomicCorrelation,
    /// 行级不变量 9：correlation 半批次（Move 侧或 Unassigned 侧缺失）——整批不可见。
    IncompleteAtomicCorrelation,
    /// PL-4：host.level != target_level 或 host range 非法。
    InvalidHostLevel,
    /// 墓碑合取门失败（T1–T5 任一项为假）。
    TombstoneGateFailed,
    /// 墓碑不可复活（TOMBSTONED 是终态）。
    TombstoneImmutable,
    /// 同一 (key, sequence) 事件内容冲突。
    IdempotencyConflict,
    /// B3 Move 侧对账：Move supersede 事件声明的 old host 与账本当前 host 不一致
    /// （本票 B1 归约旧 host 即当前态 `prev.host`，无独立声明面，故该码当前不可触发，
    /// 冻结给后续 Move 账本 reconciliation 用）。
    HostMismatch,
    /// supersede 边成环或 self-supersede。
    SupersedeCycle,
    /// 同一 host 已被 supersede。
    HostAlreadySuperseded,
    /// base record 键重复。
    DuplicateBaseKey,
    /// PL-6：同一查询混用未显式迁移的 rule version。
    RuleVersionMix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    DuplicateNoop,
}

/// as-of 查询（spec §4.3：调用方必须显式传 `rule_version`，不得自动选最新）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerQuery {
    pub scope: LedgerScope,
    pub as_of: usize,
    pub rule_version: RuleVersion,
}

/// 单 key 的 per-key 归约当前态（`reduce_unassigned` 内部）。
#[derive(Debug, Clone, Copy)]
struct RunningEntry {
    disposition: Disposition,
    generation: u32,
    lower_range: SourceRange,
    host: Option<HostRef>,
    last_reason: ReasonCode,
    last_sequence: u32,
    last_judge_at: usize,
    last_rule_version: RuleVersion,
}

/// 核心 reducer（B1）：只消费 `UnassignedEvent`，correlation 单侧校验（ASSIGNED/REOPENED 必须
/// 携带 correlation_id）。两向原子批次完备校验在 [`validate_correlation_batches`]（B3 seam）。
#[derive(Debug, Default, Clone)]
pub struct LedgerReducer {
    events: Vec<UnassignedEvent>,
    move_events: Vec<MoveEvent>,
}

impl LedgerReducer {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            move_events: Vec::new(),
        }
    }

    /// 追加一条 Unassigned 事件；幂等重复零输出，内容冲突报错；失败回滚（不落半行）。
    pub fn apply(&mut self, event: UnassignedEvent) -> Result<ApplyOutcome, LedgerError> {
        let event_id = event.event_id();
        if let Some(existing) = self.events.iter().find(|e| e.event_id() == event_id) {
            return if existing == &event {
                Ok(ApplyOutcome::DuplicateNoop)
            } else {
                Err(LedgerError::IdempotencyConflict)
            };
        }
        self.events.push(event.clone());
        let query = LedgerQuery {
            scope: event.key.scope,
            as_of: usize::MAX,
            rule_version: event.rule_version,
        };
        if let Err(err) = reduce_unassigned(&self.events, query) {
            self.events.pop();
            return Err(err);
        }
        Ok(ApplyOutcome::Applied)
    }

    /// 追加一条 Move 事件（B3 seam 的输入侧；correlation 两向校验在 [`assemble_ledger`] 执行）。
    pub fn apply_move(&mut self, event: MoveEvent) {
        self.move_events.push(event);
    }

    /// 以 `as_of = usize::MAX` 归约当前账本（测试/fixture 便捷入口）。
    pub fn view(&self, rule_version: RuleVersion) -> Result<LedgerView, LedgerError> {
        let scope = self
            .events
            .first()
            .map(|e| e.key.scope)
            .or_else(|| {
                self.move_events.first().map(|_| {
                    // Move 事件不携带 scope（correlation 事实）；fallback 到 BTC_SCOPE 仅用于
                    // 无 Unassigned 事件的纯 Move 场景（测试 fixture）。
                    BTC_SCOPE
                })
            })
            .unwrap_or(BTC_SCOPE);
        reduce_unassigned(
            &self.events,
            LedgerQuery {
                scope,
                as_of: usize::MAX,
                rule_version,
            },
        )
    }
}

/// 两向 correlation 完备校验（B3 seam；spec §2.2 行级不变量 9 / §9.3 硬门 4）：
/// correlation_id 一旦在任一侧可见，两侧必须各有一条同 correlation、同 as_of 可见的事件；
/// 缺任一侧 ⟹ `IncompleteAtomicCorrelation`（整批不可见，fail-closed）。
pub fn validate_correlation_batches(
    move_events: &[MoveEvent],
    unassigned_events: &[UnassignedEvent],
    query: LedgerQuery,
) -> Result<(), LedgerError> {
    let move_corrs: BTreeSet<CorrelationId> = move_events
        .iter()
        .filter(|m| m.judge_at <= query.as_of && m.rule_version.0 <= query.rule_version.0)
        .filter_map(|m| m.correlation_id)
        .collect();
    let unassigned_corrs: BTreeSet<CorrelationId> = unassigned_events
        .iter()
        .filter(|e| e.judge_at <= query.as_of && e.rule_version.0 <= query.rule_version.0)
        .filter(|e| e.key.scope == query.scope)
        .filter_map(|e| e.correlation_id)
        .collect();
    if move_corrs != unassigned_corrs {
        return Err(LedgerError::IncompleteAtomicCorrelation);
    }
    Ok(())
}

/// 完整 as-of 纯归约 seam（spec §4.1 的账本分量）：两向 correlation 完备校验 → 核心归约。
/// 这是未来 `assemble_level_view` 内部对 Move 账本与 Unassigned 账本的组装步骤（B2/B3 收敛后）。
pub fn assemble_ledger(
    move_events: &[MoveEvent],
    unassigned_events: &[UnassignedEvent],
    query: LedgerQuery,
) -> Result<LedgerView, LedgerError> {
    validate_correlation_batches(move_events, unassigned_events, query)?;
    reduce_unassigned(unassigned_events, query)
}

/// 核心 as-of 纯归约（spec §4.2 归约算法 1/3/4/5/6）。
///
/// 只消费 `judge_at <= as_of` 且 `rule_version == query.rule_version` 的 Unassigned 事件；
/// previous_event_id 链在**全量**事件索引上解析（跨 rule version 迁移事件的 previous 在旧版本，
/// 见 spec §3.2 TOMBSTONED→REOPENED 行）。任何缺类/重类/冲突立即报错，不构造部分视图。
pub fn reduce_unassigned(
    events: &[UnassignedEvent],
    query: LedgerQuery,
) -> Result<LedgerView, LedgerError> {
    // 全量索引：event_id -> event（含未可见的旧版本 previous）。精确重复行幂等合并
    // （同 id 同内容 = noop）；同 id 异内容 = IdempotencyConflict（spec §4.3：合法顺序只由
    // per-key sequence/previous 与 correlation batch 决定，重复追加不改历史）。
    let mut by_id: BTreeMap<UnassignedEventId, &UnassignedEvent> = BTreeMap::new();
    for event in events {
        match by_id.insert(event.event_id(), event) {
            Some(existing) if existing != event => return Err(LedgerError::IdempotencyConflict),
            _ => {}
        }
    }

    // 可见事件：judge_at <= as_of && rule_version <= query.rule_version && scope == query.scope。
    // 迁移链口径（spec §4.2「rule_version participates in Q's explicit migration chain」）：
    // rule version 顺序递增，查询 R 看到版本 [1..=R] 的事件；跨版本推进只经显式
    // RULE_VERSION_REEVALUATION REOPENED（循环内 PL-6 校验）。按 (key, sequence) 稳定排序——
    // 物理行顺序不参与归约（spec §4.3）；sequence 是同一 key 全生命周期严格递增序号，
    // 故 (key, sequence) 定序同时覆盖版本推进序。去重索引迭代（非原始切片）⟹ 精确重复
    // 行不重复参与归约。
    let mut visible: Vec<&UnassignedEvent> = by_id
        .values()
        .copied()
        .filter(|e| {
            e.judge_at <= query.as_of
                && e.rule_version.0 <= query.rule_version.0
                && e.key.scope == query.scope
        })
        .collect();
    visible.sort_by_key(|e| (e.key, e.sequence));

    let mut running: BTreeMap<UnassignedKey, RunningEntry> = BTreeMap::new();
    let mut supersede_edges: BTreeMap<HostRef, HostRef> = BTreeMap::new();

    for event in visible {
        validate_row(event)?;
        let current = running.get(&event.key).copied();
        if let Some(prev) = current {
            // PL-6：跨版本推进只经显式 RULE_VERSION_REEVALUATION REOPENED；表外混版拒绝。
            if prev.last_rule_version != event.rule_version {
                let is_migration = matches!(event.event_type, UnassignedEventType::Reopened)
                    && event.reason == ReasonCode::RuleVersionReevaluation;
                if !is_migration {
                    return Err(LedgerError::RuleVersionMix);
                }
            }
            // 墓碑终态（spec §3.2）：同版本不可复活；仅新 rule version 的显式迁移 REOPENED 放行。
            if prev.disposition == Disposition::Tombstoned {
                let is_rule_migration = matches!(event.event_type, UnassignedEventType::Reopened)
                    && event.reason == ReasonCode::RuleVersionReevaluation
                    && event.rule_version != prev.last_rule_version;
                if !is_rule_migration {
                    return Err(LedgerError::TombstoneImmutable);
                }
            }
        }
        let mut edge_to_add = None;
        let next = transition(event, current, &by_id, &supersede_edges, &mut edge_to_add)?;
        if let Some((old, new)) = edge_to_add {
            supersede_edges.insert(old, new);
        }
        running.insert(event.key, next);
    }

    Ok(LedgerView {
        scope: query.scope,
        as_of: query.as_of,
        rule_version: query.rule_version,
        entries: running
            .into_iter()
            .map(|(key, entry)| {
                (
                    key,
                    EntryView {
                        disposition: entry.disposition,
                        generation: entry.generation,
                        lower_range: entry.lower_range,
                        host: entry.host,
                        last_reason: entry.last_reason,
                        last_sequence: entry.last_sequence,
                        last_judge_at: entry.last_judge_at,
                        last_rule_version: entry.last_rule_version,
                    },
                )
            })
            .collect(),
        supersede_edges,
    })
}

/// spec §2.2 行级不变量（不含 transfer，转移在 [`transition`]）。
fn validate_row(event: &UnassignedEvent) -> Result<(), LedgerError> {
    // 不变量 2：PL-1 跳级/跨级键。
    if event.key.lower_id.level.checked_add(1) != Some(event.key.target_level) {
        return Err(LedgerError::CrossLevelKey);
    }
    // 不变量 3：range 合法。
    if event.lower_range.start > event.lower_range.end {
        return Err(LedgerError::InvalidRange);
    }
    // 不变量 4：sequence 与前序接续。
    match event.previous_event_id {
        None => {
            if event.sequence != 0 {
                return Err(LedgerError::BrokenEventChain);
            }
        }
        Some(prev) => {
            if prev.key != event.key || prev.sequence.checked_add(1) != Some(event.sequence) {
                return Err(LedgerError::BrokenEventChain);
            }
        }
    }
    // 不变量 6/7：host 与 correlation 的形态约束。
    match event.event_type {
        UnassignedEventType::Opened | UnassignedEventType::Tombstoned => {
            if event.host.is_some() {
                return Err(LedgerError::InvalidTransition);
            }
        }
        UnassignedEventType::Assigned => {
            if event.host.is_none() || event.correlation_id.is_none() {
                return Err(LedgerError::MissingAtomicCorrelation);
            }
        }
        UnassignedEventType::Reopened => {
            if event.host.is_some() || event.correlation_id.is_none() {
                return Err(LedgerError::InvalidTransition);
            }
        }
    }
    // PL-4：host.level == target_level 且 host range 合法。
    if let Some(host) = event.host {
        if host.level != event.key.target_level || host.range.start > host.range.end {
            return Err(LedgerError::InvalidHostLevel);
        }
    }
    Ok(())
}

/// spec §3.2 合法转移表全集；表外转移一律 `InvalidTransition`。
#[allow(clippy::too_many_arguments)]
fn transition(
    event: &UnassignedEvent,
    current: Option<RunningEntry>,
    by_id: &BTreeMap<UnassignedEventId, &UnassignedEvent>,
    supersede_edges: &BTreeMap<HostRef, HostRef>,
    edge_to_add: &mut Option<(HostRef, HostRef)>,
) -> Result<RunningEntry, LedgerError> {
    match event.event_type {
        UnassignedEventType::Opened => {
            if current.is_some()
                || event.sequence != 0
                || event.generation != 0
                || event.previous_event_id.is_some()
                || event.correlation_id.is_some()
            {
                return Err(LedgerError::InvalidTransition);
            }
            if !event.reason.is_open_reason() {
                return Err(LedgerError::InvalidTransition);
            }
            Ok(RunningEntry {
                disposition: Disposition::Unassigned,
                generation: 0,
                lower_range: event.lower_range,
                host: None,
                last_reason: event.reason,
                last_sequence: event.sequence,
                last_judge_at: event.judge_at,
                last_rule_version: event.rule_version,
            })
        }
        UnassignedEventType::Assigned => {
            let Some(prev) = current else {
                return Err(LedgerError::InvalidTransition);
            };
            let host = event.host.expect("row validated: ASSIGNED 必携 host");
            match event.reason {
                ReasonCode::LegalHostEstablished => {
                    if prev.disposition != Disposition::Unassigned
                        || event.generation != prev.generation
                    {
                        return Err(LedgerError::InvalidTransition);
                    }
                }
                ReasonCode::HostSupersededAdopted => {
                    if prev.disposition != Disposition::Assigned
                        || event.generation != prev.generation
                    {
                        return Err(LedgerError::InvalidTransition);
                    }
                    let Some(old_host) = prev.host else {
                        return Err(LedgerError::InvalidTransition);
                    };
                    if old_host == host {
                        return Err(LedgerError::SupersedeCycle);
                    }
                    validate_supersede_edge(old_host, host, supersede_edges)?;
                    *edge_to_add = Some((old_host, host));
                }
                _ => return Err(LedgerError::InvalidTransition),
            }
            Ok(RunningEntry {
                disposition: Disposition::Assigned,
                generation: event.generation,
                lower_range: event.lower_range,
                host: Some(host),
                last_reason: event.reason,
                last_sequence: event.sequence,
                last_judge_at: event.judge_at,
                last_rule_version: event.rule_version,
            })
        }
        UnassignedEventType::Tombstoned => {
            let Some(prev) = current else {
                return Err(LedgerError::InvalidTransition);
            };
            if prev.disposition != Disposition::Unassigned || event.generation != prev.generation {
                return Err(LedgerError::InvalidTransition);
            }
            if event.reason != ReasonCode::FinalizedNoLegalHost {
                return Err(LedgerError::InvalidTransition);
            }
            let Some(evidence) = event.tombstone_evidence else {
                return Err(LedgerError::TombstoneGateFailed);
            };
            if !evidence.all_conjuncts(event.judge_at) {
                return Err(LedgerError::TombstoneGateFailed);
            }
            Ok(RunningEntry {
                disposition: Disposition::Tombstoned,
                generation: event.generation,
                lower_range: event.lower_range,
                host: None,
                last_reason: event.reason,
                last_sequence: event.sequence,
                last_judge_at: event.judge_at,
                last_rule_version: event.rule_version,
            })
        }
        UnassignedEventType::Reopened => {
            let Some(prev) = current else {
                return Err(LedgerError::InvalidTransition);
            };
            match event.reason {
                ReasonCode::HostSupersededReleased => {
                    if prev.disposition != Disposition::Assigned
                        || event.generation != prev.generation.saturating_add(1)
                    {
                        return Err(LedgerError::InvalidTransition);
                    }
                }
                ReasonCode::RuleVersionReevaluation => {
                    if prev.disposition != Disposition::Tombstoned
                        || event.generation != prev.generation.saturating_add(1)
                    {
                        return Err(LedgerError::InvalidTransition);
                    }
                    // 新 rule version 必须与前一事件（旧版本墓碑）不同。
                    let Some(prev_id) = event.previous_event_id else {
                        return Err(LedgerError::InvalidTransition);
                    };
                    let Some(prev_event) = by_id.get(&prev_id) else {
                        return Err(LedgerError::BrokenEventChain);
                    };
                    if prev_event.rule_version == event.rule_version {
                        return Err(LedgerError::RuleVersionMix);
                    }
                }
                _ => return Err(LedgerError::InvalidTransition),
            }
            Ok(RunningEntry {
                disposition: Disposition::Unassigned,
                generation: event.generation,
                lower_range: event.lower_range,
                host: None,
                last_reason: event.reason,
                last_sequence: event.sequence,
                last_judge_at: event.judge_at,
                last_rule_version: event.rule_version,
            })
        }
    }
}

/// supersede 边校验：old 不得已被 supersede；old != new；new 沿既有边不可回达 old（成环）。
fn validate_supersede_edge(
    old: HostRef,
    new: HostRef,
    edges: &BTreeMap<HostRef, HostRef>,
) -> Result<(), LedgerError> {
    if edges.contains_key(&old) {
        return Err(LedgerError::HostAlreadySuperseded);
    }
    if old == new {
        return Err(LedgerError::SupersedeCycle);
    }
    let mut cursor = new;
    let mut visited = BTreeSet::new();
    while visited.insert(cursor) {
        if cursor == old {
            return Err(LedgerError::SupersedeCycle);
        }
        let Some(next) = edges.get(&cursor).copied() else {
            return Ok(());
        };
        cursor = next;
    }
    Err(LedgerError::SupersedeCycle)
}

// ═══════════════════════════════════════════════════════════════════════════
// adopt_v1 覆盖层（#67/#68 冻结 fixture 的 33 口径）
// ═══════════════════════════════════════════════════════════════════════════

/// adopt_v1 的四种采纳机制（诊断名分，非账本 reason code）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdoptMechanism {
    CenterExtension,
    TrendAssembly,
    CrossWindowRecompose,
    TerminalContinuation,
}

impl AdoptMechanism {
    pub fn code(self) -> &'static str {
        match self {
            Self::CenterExtension => "CENTER_EXTENSION",
            Self::TrendAssembly => "TREND_ASSEMBLY",
            Self::CrossWindowRecompose => "CROSS_WINDOW_RECOMPOSE",
            Self::TerminalContinuation => "TERMINAL_CONTINUATION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseRecord {
    pub key: UnassignedKey,
    pub lower_range: SourceRange,
    pub settled_at: usize,
    pub base_host: Option<HostRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdoptionCandidate {
    pub key: UnassignedKey,
    pub lower_range: SourceRange,
    pub judge_at: usize,
    pub mechanism: AdoptMechanism,
    pub host: HostRef,
}

fn candidate_tie_key(candidate: &AdoptionCandidate) -> (AdoptMechanism, usize, usize, usize, u128) {
    (
        candidate.mechanism,
        candidate
            .host
            .range
            .end
            .saturating_sub(candidate.host.range.start),
        candidate.host.range.start,
        candidate.host.range.end,
        candidate.host.move_id.0,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationResult {
    pub base_assignments: BTreeMap<UnassignedKey, HostRef>,
    pub adopted: BTreeMap<UnassignedKey, AdoptionCandidate>,
    pub events: Vec<UnassignedEvent>,
    /// 与每个 ASSIGNED 同 correlation 的 MOVE_CREATED Move 侧（spec §3.2 OPENED→ASSIGNED
    /// 原子关联）；两向完备校验经 [`assemble_ledger`] 执行。
    pub move_events: Vec<MoveEvent>,
    pub ledger: LedgerView,
}

impl IntegrationResult {
    /// 旧 DP 重组口径残留（35 = 33 + 2 回退释放）。
    pub fn legacy_recompose_residual(&self) -> usize {
        LEGACY_RECOMPOSE_RESIDUAL
    }

    /// 旧 DP 重组口径回退数（2）。
    pub fn legacy_recompose_regressions(&self) -> usize {
        LEGACY_RECOMPOSE_REGRESSIONS
    }
}

/// adopt_v1 组装（#67 spec §0.2 冻结口径）：基座 host 不变（M1）、+19 获 host（M2）、
/// host⊕ledger 完备互斥（M3）。assignment 与 host supersede 同 correlation 原子追加。
pub fn integrate_adopt_v1(
    records: &[BaseRecord],
    candidates: &[AdoptionCandidate],
    as_of: usize,
) -> Result<IntegrationResult, LedgerError> {
    let mut visible = BTreeMap::new();
    for record in records.iter().filter(|record| record.settled_at <= as_of) {
        if visible.insert(record.key, *record).is_some() {
            return Err(LedgerError::DuplicateBaseKey);
        }
    }

    let mut candidates_by_key: BTreeMap<UnassignedKey, Vec<AdoptionCandidate>> = BTreeMap::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.judge_at <= as_of)
    {
        candidates_by_key
            .entry(candidate.key)
            .or_default()
            .push(*candidate);
    }

    let mut base_assignments = BTreeMap::new();
    let mut adopted = BTreeMap::new();
    let mut events = Vec::new();
    let mut move_events = Vec::new();
    for (key, record) in visible {
        if let Some(host) = record.base_host {
            base_assignments.insert(key, host);
            continue;
        }

        let opened = UnassignedEvent {
            key,
            sequence: 0,
            generation: 0,
            lower_range: record.lower_range,
            event_type: UnassignedEventType::Opened,
            reason: ReasonCode::NoContainingSeedWindow,
            judge_at: record.settled_at,
            host: None,
            previous_event_id: None,
            correlation_id: None,
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            tombstone_evidence: None,
        };
        let opened_id = opened.event_id();
        events.push(opened);

        let chosen = candidates_by_key.get_mut(&key).and_then(|options| {
            options.sort_by_key(candidate_tie_key);
            options.first().copied()
        });
        if let Some(candidate) = chosen {
            let correlation_id = CorrelationId(
                ((key.target_level as u128) << 96)
                    | ((key.lower_id.level as u128) << 64)
                    | key.lower_id.ordinal as u128,
            );
            let judge_at = candidate.judge_at.max(record.settled_at);
            events.push(UnassignedEvent {
                key,
                sequence: 1,
                generation: 0,
                lower_range: record.lower_range,
                event_type: UnassignedEventType::Assigned,
                reason: ReasonCode::LegalHostEstablished,
                judge_at,
                host: Some(candidate.host),
                previous_event_id: Some(opened_id),
                correlation_id: Some(correlation_id),
                rule_version: RuleVersion(LEDGER_RULE_VERSION),
                tombstone_evidence: None,
            });
            // 同 correlation 的 Move 侧（MOVE_CREATED）——原子批次两向完备。
            move_events.push(MoveEvent {
                correlation_id: Some(correlation_id),
                judge_at,
                rule_version: RuleVersion(LEDGER_RULE_VERSION),
                host: candidate.host,
                kind: MoveEventKind::Created,
            });
            adopted.insert(key, candidate);
        }
    }

    let query = LedgerQuery {
        scope: BTC_SCOPE,
        as_of,
        rule_version: RuleVersion(LEDGER_RULE_VERSION),
    };
    let ledger = assemble_ledger(&move_events, &events, query)?;
    Ok(IntegrationResult {
        base_assignments,
        adopted,
        events,
        move_events,
        ledger,
    })
}

fn fixture_key(level: u32, ordinal: u64) -> UnassignedKey {
    UnassignedKey {
        scope: BTC_SCOPE,
        target_level: level + 1,
        lower_id: ElementId { level, ordinal },
    }
}

fn fixture_host(level: u32, ordinal: u64, start: usize, end: usize, completed: bool) -> HostRef {
    HostRef {
        move_id: MoveId(((level as u128 + 1) << 56) | ordinal as u128),
        move_version: 1,
        level: level + 1,
        range: SourceRange { start, end },
        completion_at_event: if completed {
            HostCompletionAtEvent::Completed
        } else {
            HostCompletionAtEvent::Pending
        },
    }
}

/// 冻结 task #67 fixture：19 采纳候选 + 33 残留 = 52 基线未归属 lower。
pub fn task67_fixture() -> (Vec<BaseRecord>, Vec<AdoptionCandidate>) {
    let adopted = [
        (
            0,
            1126,
            137974,
            137996,
            136767,
            138631,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            3217,
            367919,
            367993,
            367527,
            368201,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            3457,
            391323,
            391355,
            390932,
            391355,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            8062,
            892963,
            892984,
            891823,
            893171,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            15685,
            1733210,
            1733340,
            1732375,
            1733617,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            15798,
            1746272,
            1746389,
            1745358,
            1747317,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            16192,
            1792727,
            1792891,
            1792356,
            1793002,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            20470,
            2306893,
            2306924,
            2305960,
            2306945,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            21613,
            2444538,
            2444617,
            2444220,
            2445064,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            23759,
            2703777,
            2703853,
            2703015,
            2704269,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            24012,
            2734544,
            2734670,
            2733227,
            2734862,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            27163,
            3102728,
            3102816,
            3101775,
            3104190,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            32353,
            3702311,
            3702494,
            3701504,
            3702699,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            33409,
            3827807,
            3827850,
            3825651,
            3827879,
            false,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            0,
            35909,
            4128452,
            4128611,
            4128007,
            4129083,
            true,
            AdoptMechanism::CenterExtension,
        ),
        (
            0,
            36752,
            4225895,
            4226074,
            4224623,
            4226074,
            false,
            AdoptMechanism::TrendAssembly,
        ),
        (
            0,
            39878,
            4598245,
            4598300,
            4597872,
            4598760,
            false,
            AdoptMechanism::TrendAssembly,
        ),
        (
            1,
            10846,
            4133078,
            4133432,
            4130475,
            4134490,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
        (
            2,
            1514,
            1864033,
            1865267,
            1859426,
            1865267,
            true,
            AdoptMechanism::CrossWindowRecompose,
        ),
    ];
    let residual = [
        (0, 2498, 294379, 294678),
        (0, 3705, 417456, 417565),
        (0, 4869, 542428, 542539),
        (0, 10581, 1169165, 1169293),
        (0, 11956, 1319311, 1319352),
        (0, 12211, 1349713, 1349876),
        (0, 13019, 1437139, 1437152),
        (0, 14718, 1621897, 1622008),
        (0, 20127, 2264911, 2264933),
        (0, 20299, 2286677, 2286887),
        (0, 22171, 2510715, 2510734),
        (0, 22729, 2576876, 2576984),
        (0, 24227, 2759962, 2760015),
        (0, 25368, 2903157, 2903192),
        (0, 27013, 3084709, 3084854),
        (0, 28272, 3229763, 3229845),
        (0, 30608, 3499553, 3499792),
        (0, 30760, 3516730, 3516953),
        (0, 31288, 3577921, 3577959),
        (0, 32469, 3716206, 3716484),
        (0, 33046, 3783351, 3783467),
        (0, 35944, 4132879, 4133078),
        (0, 36442, 4190140, 4190318),
        (1, 3278, 1183803, 1184381),
        (1, 3700, 1333681, 1334083),
        (1, 5304, 1939180, 1939671),
        (1, 6576, 2446451, 2446609),
        (1, 8479, 3192471, 3192734),
        (1, 9472, 3578942, 3579315),
        (1, 9573, 3616553, 3616977),
        (1, 10947, 4171769, 4172028),
        (1, 11518, 4397210, 4397392),
        (2, 107, 145672, 147050),
    ];

    let mut records = Vec::new();
    let mut candidates = Vec::new();
    for (level, ordinal, lower_start, lower_end, host_start, host_end, completed, mechanism) in
        adopted
    {
        let key = fixture_key(level, ordinal);
        let lower_range = SourceRange {
            start: lower_start,
            end: lower_end,
        };
        let host = fixture_host(level, ordinal, host_start, host_end, completed);
        records.push(BaseRecord {
            key,
            lower_range,
            settled_at: lower_end,
            base_host: None,
        });
        candidates.push(AdoptionCandidate {
            key,
            lower_range,
            judge_at: host_end,
            mechanism,
            host,
        });
    }
    for (level, ordinal, start, end) in residual {
        records.push(BaseRecord {
            key: fixture_key(level, ordinal),
            lower_range: SourceRange { start, end },
            settled_at: end,
            base_host: None,
        });
    }
    (records, candidates)
}

/// 冻结 fixture 全量重放（33 口径验收断言）。
pub fn run_task67_fixture() -> IntegrationResult {
    let (records, candidates) = task67_fixture();
    let result = integrate_adopt_v1(&records, &candidates, FIXTURE_AS_OF)
        .expect("frozen task67 fixture must reduce");
    let (assigned, unassigned, tombstoned) = result.ledger.counts();
    assert_eq!(records.len(), 52);
    assert_eq!(result.adopted.len(), 19);
    assert_eq!(
        result
            .adopted
            .values()
            .filter(|c| matches!(c.host.completion_at_event, HostCompletionAtEvent::Completed))
            .count(),
        7
    );
    assert_eq!((assigned, unassigned, tombstoned), (19, 33, 0));
    assert_eq!(assigned + unassigned + tombstoned, records.len());
    result
}

// ═══════════════════════════════════════════════════════════════════════════
// 测试：33 口径验收 + 5 条 property（task68 迁移）+ 10 条 prefix property（spec §6.2）
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    // ── 测试脚手架 ──

    fn key(level: u32, ordinal: u64) -> UnassignedKey {
        UnassignedKey {
            scope: BTC_SCOPE,
            target_level: level + 1,
            lower_id: ElementId { level, ordinal },
        }
    }

    fn range(seed: usize) -> SourceRange {
        SourceRange {
            start: seed * 10,
            end: seed * 10 + 9,
        }
    }

    fn host(level: u32, move_id: u64, version: u32, completed: bool) -> HostRef {
        HostRef {
            move_id: MoveId(move_id as u128),
            move_version: version,
            level: level + 1,
            range: SourceRange {
                start: move_id as usize * 10,
                end: move_id as usize * 10 + 19,
            },
            completion_at_event: if completed {
                HostCompletionAtEvent::Completed
            } else {
                HostCompletionAtEvent::Pending
            },
        }
    }

    fn opened(k: UnassignedKey, r: SourceRange, judge_at: usize) -> UnassignedEvent {
        UnassignedEvent {
            key: k,
            sequence: 0,
            generation: 0,
            lower_range: r,
            event_type: UnassignedEventType::Opened,
            reason: ReasonCode::NoHostAtAsOf,
            judge_at,
            host: None,
            previous_event_id: None,
            correlation_id: None,
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            tombstone_evidence: None,
        }
    }

    fn assigned(
        k: UnassignedKey,
        r: SourceRange,
        previous: UnassignedEventId,
        h: HostRef,
        sequence: u32,
        generation: u32,
    ) -> UnassignedEvent {
        UnassignedEvent {
            key: k,
            sequence,
            generation,
            lower_range: r,
            event_type: UnassignedEventType::Assigned,
            reason: ReasonCode::LegalHostEstablished,
            judge_at: r.end + sequence as usize,
            host: Some(h),
            previous_event_id: Some(previous),
            correlation_id: Some(CorrelationId(
                ((k.target_level as u128) << 96)
                    | ((k.lower_id.level as u128) << 64)
                    | k.lower_id.ordinal as u128,
            )),
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            tombstone_evidence: None,
        }
    }

    fn superseded_adopted(
        k: UnassignedKey,
        r: SourceRange,
        previous: UnassignedEventId,
        new_host: HostRef,
        sequence: u32,
        generation: u32,
        correlation: u128,
    ) -> UnassignedEvent {
        UnassignedEvent {
            key: k,
            sequence,
            generation,
            lower_range: r,
            event_type: UnassignedEventType::Assigned,
            reason: ReasonCode::HostSupersededAdopted,
            judge_at: r.end + sequence as usize,
            host: Some(new_host),
            previous_event_id: Some(previous),
            correlation_id: Some(CorrelationId(correlation)),
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            tombstone_evidence: None,
        }
    }

    fn superseded_released(
        k: UnassignedKey,
        r: SourceRange,
        previous: UnassignedEventId,
        sequence: u32,
        generation: u32,
        correlation: u128,
    ) -> UnassignedEvent {
        UnassignedEvent {
            key: k,
            sequence,
            generation,
            lower_range: r,
            event_type: UnassignedEventType::Reopened,
            reason: ReasonCode::HostSupersededReleased,
            judge_at: r.end + sequence as usize,
            host: None,
            previous_event_id: Some(previous),
            correlation_id: Some(CorrelationId(correlation)),
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            tombstone_evidence: None,
        }
    }

    fn tombstoned(
        k: UnassignedKey,
        r: SourceRange,
        previous: UnassignedEventId,
        sequence: u32,
        generation: u32,
        evidence: TombstoneEvidence,
        judge_at: usize,
    ) -> UnassignedEvent {
        UnassignedEvent {
            key: k,
            sequence,
            generation,
            lower_range: r,
            event_type: UnassignedEventType::Tombstoned,
            reason: ReasonCode::FinalizedNoLegalHost,
            judge_at,
            host: None,
            previous_event_id: Some(previous),
            correlation_id: None,
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            tombstone_evidence: Some(evidence),
        }
    }

    fn evidence_for(r: SourceRange) -> TombstoneEvidence {
        TombstoneEvidence {
            lower_settled_at: r.end,
            successor_closed_at: r.end + 2,
            coordinate_finalized_at: r.end + 3,
            exhaustive_check_at: r.end + 4,
            lower_settled: true,
            successor_closed: true,
            coordinate_finalized: true,
            no_legal_or_pending_host: true,
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    fn v1() -> RuleVersion {
        RuleVersion(LEDGER_RULE_VERSION)
    }

    fn query(as_of: usize) -> LedgerQuery {
        LedgerQuery {
            scope: BTC_SCOPE,
            as_of,
            rule_version: v1(),
        }
    }

    // ── 33 口径验收（冻结 fixture 全量重放 + 双 SHA-256 bit-exact） ──

    #[test]
    fn adopt_v1_fixture_acceptance_33_caliber() {
        let first = run_task67_fixture();
        let second = run_task67_fixture();

        // 52 → 33；+19 获 host；其中 7 Completed；regressions=0。
        let (assigned, unassigned, tombstoned) = first.ledger.counts();
        assert_eq!(first.adopted.len(), 19);
        assert_eq!(unassigned, 33);
        assert_eq!((assigned, unassigned, tombstoned), (19, 33, 0));
        assert_eq!(
            first
                .adopted
                .values()
                .filter(|c| matches!(c.host.completion_at_event, HostCompletionAtEvent::Completed))
                .count(),
            7
        );
        // 集合恒等式：52 = 19 + 33 + 0（M1 基座 host 不变 + M2 +19 + M3 host⊕ledger 完备互斥）。
        assert_eq!(assigned + unassigned + tombstoned, 52);

        // 双 SHA-256 bit-exact：两次独立重放规范终态编码逐位一致。
        let canonical_first = first.ledger.canonical_terminal_state();
        let canonical_second = second.ledger.canonical_terminal_state();
        assert_eq!(canonical_first, canonical_second);
        assert_eq!(sha256_hex(&canonical_first), sha256_hex(&canonical_second));

        // 旧 DP 重组口径仅作兼容诊断字段（35 / 2）。
        assert_eq!(first.legacy_recompose_residual(), LEGACY_RECOMPOSE_RESIDUAL);
        assert_eq!(
            first.legacy_recompose_regressions(),
            LEGACY_RECOMPOSE_REGRESSIONS
        );
        // 35 = 33（adopt_v1 残留）+ 2（旧 DP 回退释放）。
        assert_eq!(
            first.legacy_recompose_residual(),
            unassigned + LEGACY_RECOMPOSE_REGRESSIONS
        );

        // 幂等：同 fixture 双份事件重放零漂移。
        let mut doubled = first.events.clone();
        doubled.extend(first.events.clone());
        let duplicate = reduce_unassigned(&doubled, query(FIXTURE_AS_OF)).expect("idempotent");
        assert_eq!(duplicate, first.ledger);
    }

    // ── 5 条 domain property（task68 迁移） ──

    #[test]
    fn prop_tombstone_cannot_be_resurrected() {
        for seed in 0..64u64 {
            let k = key(0, seed + 1);
            let r = range(seed as usize + 1);
            let open = opened(k, r, r.end);
            let evidence = evidence_for(r);

            // T1..T5：每个合取项逐个翻 false 都必须独立拒绝。
            for missing in 0..5 {
                let mut reducer = LedgerReducer::new();
                reducer.apply(open.clone()).unwrap();
                let mut bad = evidence;
                match missing {
                    0 => bad.lower_settled = false,
                    1 => bad.successor_closed = false,
                    2 => bad.coordinate_finalized = false,
                    3 => bad.no_legal_or_pending_host = false,
                    4 => {}
                    _ => unreachable!(),
                }
                let judge_at = bad.expected_judge_at() + usize::from(missing == 4);
                let event = tombstoned(k, r, open.event_id(), 1, 0, bad, judge_at);
                assert_eq!(reducer.apply(event), Err(LedgerError::TombstoneGateFailed));
            }

            // 全真放行；重复零输出；同版本复活被拒。
            let mut reducer = LedgerReducer::new();
            reducer.apply(open.clone()).unwrap();
            let tomb = tombstoned(
                k,
                r,
                open.event_id(),
                1,
                0,
                evidence,
                evidence.expected_judge_at(),
            );
            reducer.apply(tomb.clone()).unwrap();
            assert_eq!(reducer.apply(tomb.clone()), Ok(ApplyOutcome::DuplicateNoop));
            let mut resurrection =
                assigned(k, r, tomb.event_id(), host(0, 50_000 + seed, 1, true), 2, 0);
            resurrection.judge_at = tomb.judge_at + 1;
            assert_eq!(
                reducer.apply(resurrection),
                Err(LedgerError::TombstoneImmutable)
            );
            let view = reducer.view(v1()).unwrap();
            assert_eq!(view.entries[&k].disposition, Disposition::Tombstoned);
        }
    }

    #[test]
    fn prop_supersede_chain_is_acyclic() {
        for chain_len in 2..33u32 {
            let k = key(0, 10_000 + chain_len as u64);
            let r = range(chain_len as usize);
            let open = opened(k, r, r.end);
            let first_host = host(0, 60_000 + chain_len as u64, 0, false);
            let assign = assigned(k, r, open.event_id(), first_host, 1, 0);
            let mut reducer = LedgerReducer::new();
            reducer.apply(open).unwrap();
            reducer.apply(assign.clone()).unwrap();
            let mut old = first_host;
            let mut previous = assign.event_id();
            let mut sequence = 2;
            for version in 1..chain_len {
                let new = host(0, first_host.move_id.0 as u64, version, version % 2 == 0);
                let event = superseded_adopted(k, r, previous, new, sequence, 0, sequence as u128);
                reducer.apply(event.clone()).unwrap();
                previous = event.event_id();
                old = new;
                sequence += 1;
            }
            let before = reducer.view(v1()).unwrap();
            let cycle = superseded_adopted(
                k,
                r,
                previous,
                first_host,
                sequence,
                0,
                u128::MAX - chain_len as u128,
            );
            assert_eq!(reducer.apply(cycle), Err(LedgerError::SupersedeCycle));
            assert_eq!(reducer.view(v1()).unwrap(), before);
            let _ = old;
        }
    }

    #[test]
    fn prop_adopt_orphan_is_add_only_and_preserves_base_hosts() {
        for seed in 0..256u64 {
            let mut state = seed;
            let mut records = Vec::new();
            let mut candidates = Vec::new();
            for ordinal in 0..64u64 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let k = key(0, ordinal);
                let r = range(ordinal as usize + 1);
                let base_host =
                    (state & 1 == 0).then(|| host(0, ordinal + 1_000, 1, state & 2 == 0));
                records.push(BaseRecord {
                    key: k,
                    lower_range: r,
                    settled_at: r.end,
                    base_host,
                });
                candidates.push(AdoptionCandidate {
                    key: k,
                    lower_range: r,
                    judge_at: r.end + 1,
                    mechanism: match state % 4 {
                        0 => AdoptMechanism::CenterExtension,
                        1 => AdoptMechanism::TrendAssembly,
                        2 => AdoptMechanism::CrossWindowRecompose,
                        _ => AdoptMechanism::TerminalContinuation,
                    },
                    host: host(0, ordinal + 10_000, 1, state & 4 == 0),
                });
            }
            let view = integrate_adopt_v1(&records, &candidates, usize::MAX).unwrap();
            for record in &records {
                if let Some(base_host) = record.base_host {
                    assert_eq!(view.base_assignments.get(&record.key), Some(&base_host));
                    assert!(!view.adopted.contains_key(&record.key));
                    assert!(!view.ledger.entries.contains_key(&record.key));
                }
            }
        }
    }

    #[test]
    fn prop_ledger_entries_are_conserved_and_disjoint() {
        for seed in 0..192u64 {
            let mut records = Vec::new();
            let mut candidates = Vec::new();
            for ordinal in 0..96u64 {
                let level = (ordinal % 3) as u32;
                let unique_ordinal = ordinal / 3 + seed * 100;
                let k = key(level, unique_ordinal);
                let r = range(ordinal as usize + 1);
                let base_host = ((ordinal + seed) % 5 == 0)
                    .then(|| host(level, 70_000 + ordinal + seed * 100, 1, false));
                records.push(BaseRecord {
                    key: k,
                    lower_range: r,
                    settled_at: r.end,
                    base_host,
                });
                if (ordinal.wrapping_mul(17) + seed) % 4 != 0 {
                    candidates.push(AdoptionCandidate {
                        key: k,
                        lower_range: r,
                        judge_at: r.end + 1,
                        mechanism: AdoptMechanism::CrossWindowRecompose,
                        host: host(level, 90_000 + ordinal + seed * 100, 1, ordinal % 7 == 0),
                    });
                }
            }
            let result = integrate_adopt_v1(&records, &candidates, usize::MAX).unwrap();
            let (assigned, unassigned, tombstoned) = result.ledger.counts();
            assert_eq!(
                records.len(),
                result.base_assignments.len() + assigned + unassigned + tombstoned
            );
            assert_eq!(
                result.ledger.entries.len(),
                assigned + unassigned + tombstoned
            );
            for k in result.base_assignments.keys() {
                assert!(!result.ledger.entries.contains_key(k));
            }
            for entry in result.ledger.entries.values() {
                assert_eq!(
                    entry.host.is_some(),
                    entry.disposition == Disposition::Assigned
                );
            }
        }
    }

    #[test]
    fn prop_full_replay_is_idempotent_and_sha256_bit_exact() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let fixture = run_task67_fixture();
        let fresh =
            assemble_ledger(&fixture.move_events, &fixture.events, query(FIXTURE_AS_OF)).unwrap();
        let mut doubled = fixture.events.clone();
        doubled.extend(fixture.events.clone());
        let mut doubled_moves = fixture.move_events.clone();
        doubled_moves.extend(fixture.move_events.clone());
        let duplicate_replay =
            assemble_ledger(&doubled_moves, &doubled, query(FIXTURE_AS_OF)).unwrap();

        // 按 key 分组后整体倒序（组内 sequence 序保持）——归约与物理行序无关。
        let mut groups: BTreeMap<UnassignedKey, Vec<UnassignedEvent>> = BTreeMap::new();
        for event in &fixture.events {
            groups.entry(event.key).or_default().push(event.clone());
        }
        let reordered: Vec<_> = groups.into_values().rev().flatten().collect();
        let reordered_replay =
            assemble_ledger(&fixture.move_events, &reordered, query(FIXTURE_AS_OF)).unwrap();

        assert_eq!(fresh, duplicate_replay);
        assert_eq!(fresh, reordered_replay);
        assert_eq!(
            sha256_hex(&fresh.canonical_terminal_state()),
            sha256_hex(&duplicate_replay.canonical_terminal_state())
        );
        assert_eq!(
            sha256_hex(&fresh.canonical_terminal_state()),
            sha256_hex(&reordered_replay.canonical_terminal_state())
        );
        assert_eq!(fresh.counts(), (19, 33, 0));
    }

    // ── 10 条 prefix property（spec §6.2） ──

    /// §6.2.1：固定 t 时，追加 judge_at>t 的后缀不得改变 t 的视图（bit-exact）。
    #[test]
    fn ledger_prefix_stability_ignores_future_events() {
        for seed in 0..32u64 {
            let k = key(0, 100_000 + seed);
            let r = range(seed as usize + 1);
            let open = opened(k, r, r.end);
            let h1 = host(0, 200_000 + seed, 1, false);
            let assign = assigned(k, r, open.event_id(), h1, 1, 0);
            let release = superseded_released(k, r, assign.event_id(), 2, 1, 1_000 + seed as u128);
            // t 落在 release 之后、readopt 之前：前缀只含 OPENED→ASSIGNED→REOPENED。
            let t = release.judge_at;
            let h2 = host(0, 200_000 + seed, 2, true);
            let readopt = UnassignedEvent {
                key: k,
                sequence: 3,
                generation: 1,
                lower_range: r,
                event_type: UnassignedEventType::Assigned,
                reason: ReasonCode::LegalHostEstablished,
                judge_at: release.judge_at + 1,
                host: Some(h2),
                previous_event_id: Some(release.event_id()),
                correlation_id: Some(CorrelationId(2_000 + seed as u128)),
                rule_version: RuleVersion(LEDGER_RULE_VERSION),
                tombstone_evidence: None,
            };
            let events = vec![open, assign, release, readopt];
            let prefix: Vec<UnassignedEvent> =
                events.iter().filter(|e| e.judge_at <= t).cloned().collect();
            let view_prefix = reduce_unassigned(&prefix, query(t)).unwrap();
            let view_full = reduce_unassigned(&events, query(t)).unwrap();
            assert_eq!(view_prefix, view_full, "未来 readopt 不得改写 t 视图");
            assert_eq!(
                view_full.entries[&k].disposition,
                Disposition::Unassigned,
                "t 时 REOPENED 仍为 Unassigned"
            );
        }
    }

    /// §6.2.2：t 时 OPENED，T 时新 host ASSIGNED；OPENED 原行内容不变、query(t) 恒 OPENED。
    #[test]
    fn future_assignment_creates_new_version_without_repainting_opened() {
        for seed in 0..32u64 {
            let k = key(0, 300_000 + seed);
            let r = range(seed as usize + 1);
            let open = opened(k, r, r.end);
            let open_row = open.clone();
            let t = open.judge_at;
            let h = host(0, 400_000 + seed, 1, true);
            let mut assign = assigned(k, r, open.event_id(), h, 1, 0);
            assign.judge_at = t + 1;
            let events = vec![open, assign];

            let at_t = reduce_unassigned(&events, query(t)).unwrap();
            assert_eq!(
                at_t.entries[&k].disposition,
                Disposition::Unassigned,
                "query(t) 恒 OPENED"
            );
            let at_t_plus = reduce_unassigned(&events, query(t + 1)).unwrap();
            assert_eq!(at_t_plus.entries[&k].disposition, Disposition::Assigned);
            assert_eq!(at_t_plus.entries[&k].host, Some(h));
            // 旧行字节不变（OPENED 原行未被未来 assignment 改写）。
            assert_eq!(open_row, events[0]);
        }
    }

    /// §6.2.3：t 时 ASSIGNED(old)，T 时 RELEASED；query(t) 旧 host、query(T) REOPENED(generation+1)。
    #[test]
    fn future_release_reopens_new_generation_only() {
        for seed in 0..32u64 {
            let k = key(0, 500_000 + seed);
            let r = range(seed as usize + 1);
            let open = opened(k, r, r.end);
            let old_host = host(0, 600_000 + seed, 1, false);
            let assign = assigned(k, r, open.event_id(), old_host, 1, 0);
            let t = assign.judge_at;
            let mut release =
                superseded_released(k, r, assign.event_id(), 2, 1, 3_000 + seed as u128);
            release.judge_at = t + 1;
            let events = vec![open, assign, release];

            let at_t = reduce_unassigned(&events, query(t)).unwrap();
            assert_eq!(at_t.entries[&k].disposition, Disposition::Assigned);
            assert_eq!(at_t.entries[&k].host, Some(old_host));
            let at_t_plus = reduce_unassigned(&events, query(t + 1)).unwrap();
            assert_eq!(at_t_plus.entries[&k].disposition, Disposition::Unassigned);
            assert_eq!(
                at_t_plus.entries[&k].generation, 1,
                "REOPENED generation = old+1"
            );
        }
    }

    /// §6.2.4：旧版本 t 时 TOMBSTONED，新版本 T 时 REOPENED；旧查询还原墓碑、新查询见迁移链。
    #[test]
    fn future_rule_migration_does_not_erase_tombstone() {
        for seed in 0..32u64 {
            let k = key(0, 700_000 + seed);
            let r = range(seed as usize + 1);
            let open = opened(k, r, r.end);
            let evidence = evidence_for(r);
            let tomb = tombstoned(
                k,
                r,
                open.event_id(),
                1,
                0,
                evidence,
                evidence.expected_judge_at(),
            );
            let t = tomb.judge_at;
            // 新 rule version 迁移：RULE_VERSION_REEVALUATION REOPENED（generation+1，version 2）。
            let mut reopen = UnassignedEvent {
                key: k,
                sequence: 2,
                generation: 1,
                lower_range: r,
                event_type: UnassignedEventType::Reopened,
                reason: ReasonCode::RuleVersionReevaluation,
                judge_at: t + 1,
                host: None,
                previous_event_id: Some(tomb.event_id()),
                correlation_id: Some(CorrelationId(4_000 + seed as u128)),
                rule_version: RuleVersion(2),
                tombstone_evidence: None,
            };
            reopen.judge_at = t + 1;
            let events = vec![open, tomb, reopen];

            let old_query = LedgerQuery {
                scope: BTC_SCOPE,
                as_of: t,
                rule_version: RuleVersion(1),
            };
            let old_view = reduce_unassigned(&events, old_query).unwrap();
            assert_eq!(old_view.entries[&k].disposition, Disposition::Tombstoned);

            let new_query = LedgerQuery {
                scope: BTC_SCOPE,
                as_of: t + 1,
                rule_version: RuleVersion(2),
            };
            let new_view = reduce_unassigned(&events, new_query).unwrap();
            assert_eq!(new_view.entries[&k].disposition, Disposition::Unassigned);
            assert_eq!(new_view.entries[&k].generation, 1);
            assert_eq!(new_view.entries[&k].last_rule_version, RuleVersion(2));
        }
    }

    /// §6.2.5：同 judge_at 多事件只由 per-key sequence/previous 定序，与物理行序无关；
    /// sequence 重复/断链稳定报错。
    #[test]
    fn same_judge_at_uses_sequence_not_input_order() {
        let k = key(0, 800_000);
        let r = range(1);
        let mut open = opened(k, r, 5);
        open.judge_at = 20;
        let mut assign = assigned(k, r, open.event_id(), host(0, 900_000, 1, true), 1, 0);
        assign.judge_at = 20;
        // 两条 ASSIGNED 同一 judge_at：sequence 决定顺序，且后者是非法转移（已 ASSIGNED 再
        // LEGAL_HOST_ESTABLISHED）→ 无论物理行序如何都稳定报错。
        let mut assign_dup = assigned(k, r, assign.event_id(), host(0, 900_000, 2, true), 2, 0);
        assign_dup.judge_at = 20;

        let shuffled = vec![assign_dup.clone(), open.clone(), assign.clone()];
        assert_eq!(
            reduce_unassigned(&shuffled, query(20)),
            Err(LedgerError::InvalidTransition)
        );
        let ordered = vec![open.clone(), assign.clone(), assign_dup.clone()];
        assert_eq!(
            reduce_unassigned(&ordered, query(20)),
            Err(LedgerError::InvalidTransition)
        );

        // 断链（sequence 跳号）稳定报错。
        let mut broken = assigned(k, r, open.event_id(), host(0, 900_000, 3, true), 5, 0);
        broken.judge_at = 20;
        assert_eq!(
            reduce_unassigned(&[open.clone(), broken], query(20)),
            Err(LedgerError::BrokenEventChain)
        );
        // sequence 重复（同 (key, sequence) 异内容）稳定报错。
        let mut dup_id = assigned(k, r, open.event_id(), host(0, 900_000, 4, true), 1, 0);
        dup_id.judge_at = 20;
        assert_eq!(
            reduce_unassigned(&[open.clone(), assign.clone(), dup_id], query(20)),
            Err(LedgerError::IdempotencyConflict)
        );
    }

    /// §6.2.6：Move 与 Unassigned 视图共享同一 as_of；XOR 完整才 Ok。
    #[test]
    fn move_and_unassigned_views_share_one_as_of() {
        let k = key(0, 1_000_000);
        let r = range(2);
        let open = opened(k, r, 10);
        let h = host(0, 1_100_000, 1, true);
        let assign = assigned(k, r, open.event_id(), h, 1, 0);
        let move_event = MoveEvent {
            correlation_id: assign.correlation_id,
            judge_at: assign.judge_at,
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            host: h,
            kind: MoveEventKind::Created,
        };
        let unassigned = vec![open.clone(), assign.clone()];
        let moves = vec![move_event];

        let view = assemble_ledger(&moves, &unassigned, query(assign.judge_at)).unwrap();
        assert_eq!(view.as_of, assign.judge_at, "三个视图分量共享同一 as_of");
        assert_eq!(view.entries[&k].disposition, Disposition::Assigned);

        // 同 as_of 下移除 host（删除 Assigned 行）→ 完全分类残缺 → 无 host 的 OPENED 键成为
        // Unassigned（合法），但 correlation 半批次不可见。
        assert_eq!(
            assemble_ledger(&moves, &[open], query(assign.judge_at)),
            Err(LedgerError::IncompleteAtomicCorrelation)
        );
    }

    /// §6.2.7：append-only——旧行编码字节不变，追加未来批次后无 update/delete。
    #[test]
    fn append_only_rows_are_byte_stable() {
        let k = key(0, 1_200_000);
        let r = range(3);
        let open = opened(k, r, 20);
        let assign = assigned(k, r, open.event_id(), host(0, 1_300_000, 1, true), 1, 0);
        let mut reducer = LedgerReducer::new();
        reducer.apply(open.clone()).unwrap();
        reducer.apply(assign.clone()).unwrap();

        let before_rows: Vec<Vec<u8>> = reducer.events.iter().map(|e| e.canonical_row()).collect();
        let before_len = reducer.events.len();
        let before_hash = sha256_hex(&before_rows.concat());

        // 追加未来批次（judge_at > 当前 as_of）。
        let future = superseded_released(k, r, assign.event_id(), 2, 1, 5_000);
        reducer.apply(future).unwrap();

        let after_rows: Vec<Vec<u8>> = reducer
            .events
            .iter()
            .take(before_len)
            .map(|e| e.canonical_row())
            .collect();
        assert_eq!(before_rows, after_rows, "旧行编码字节不变");
        assert_eq!(sha256_hex(&after_rows.concat()), before_hash);
        assert_eq!(
            reducer.events.len(),
            before_len + 1,
            "仅追加，无 update/delete"
        );
    }

    /// §6.2.8：correlation 原子批次 all-or-nothing——缺任一侧均不可见并报错，完整批次一次生效。
    #[test]
    fn atomic_correlation_is_all_or_nothing() {
        let k = key(0, 1_400_000);
        let r = range(4);
        let open = opened(k, r, 30);
        let h = host(0, 1_500_000, 1, true);
        let assign = assigned(k, r, open.event_id(), h, 1, 0);
        let move_event = MoveEvent {
            correlation_id: assign.correlation_id,
            judge_at: assign.judge_at,
            rule_version: RuleVersion(LEDGER_RULE_VERSION),
            host: h,
            kind: MoveEventKind::SupersedeAdopted,
        };

        // 完整批次：一次生效。
        let full = assemble_ledger(
            &[move_event.clone()],
            &[open.clone(), assign.clone()],
            query(assign.judge_at),
        )
        .unwrap();
        assert_eq!(full.entries[&k].disposition, Disposition::Assigned);

        // 缺 Move 侧：半批次不可见。
        assert_eq!(
            assemble_ledger(&[], &[open.clone(), assign.clone()], query(assign.judge_at)),
            Err(LedgerError::IncompleteAtomicCorrelation)
        );
        // 缺 Unassigned 侧：半批次不可见。
        assert_eq!(
            assemble_ledger(&[move_event], &[open], query(assign.judge_at)),
            Err(LedgerError::IncompleteAtomicCorrelation)
        );
    }

    /// §6.2.9：完全分类 total + disjoint——逐 key XOR 且 N=A+U+T；冲突状态稳定失败。
    #[test]
    fn coverage_partition_is_total_and_disjoint() {
        for seed in 0..96u64 {
            let mut events = Vec::new();
            for ordinal in 0..8u64 {
                let k = key(0, seed * 100 + ordinal);
                let r = range(ordinal as usize + 1);
                let open = opened(k, r, r.end);
                events.push(open.clone());
                if (ordinal + seed) % 3 == 0 {
                    events.push(assigned(
                        k,
                        r,
                        open.event_id(),
                        host(0, 1_600_000 + seed * 100 + ordinal, 1, ordinal % 2 == 0),
                        1,
                        0,
                    ));
                } else if (ordinal + seed) % 3 == 1 {
                    let ev = evidence_for(r);
                    events.push(tombstoned(
                        k,
                        r,
                        open.event_id(),
                        1,
                        0,
                        ev,
                        ev.expected_judge_at(),
                    ));
                }
            }
            let view = reduce_unassigned(&events, query(usize::MAX)).unwrap();
            let (a, u, t) = view.counts();
            assert_eq!(view.entries.len(), a + u + t);
            assert_eq!(view.entries.len(), 8);
            for entry in view.entries.values() {
                assert_eq!(
                    entry.host.is_some(),
                    entry.disposition == Disposition::Assigned
                );
            }
        }

        // 冲突状态：同一 key 两条 OPENED（sequence 冲突）稳定失败。
        let k = key(0, 2_000_000);
        let r = range(9);
        let open = opened(k, r, 10);
        let mut second_open = opened(k, r, 11);
        second_open.sequence = 0;
        assert_eq!(
            reduce_unassigned(&[open.clone(), second_open], query(11)),
            Err(LedgerError::IdempotencyConflict)
        );
    }

    /// §6.2.10：墓碑合取门——五个合取项逐个翻 false 均拒绝，仅全真接受。
    #[test]
    fn tombstone_gate_rejects_each_missing_conjunct() {
        let k = key(0, 3_000_000);
        let r = range(10);
        let open = opened(k, r, r.end);
        let evidence = evidence_for(r);
        for missing in 0..5 {
            let mut bad = evidence;
            match missing {
                0 => bad.lower_settled = false,
                1 => bad.successor_closed = false,
                2 => bad.coordinate_finalized = false,
                3 => bad.no_legal_or_pending_host = false,
                4 => {}
                _ => unreachable!(),
            }
            let judge_at = bad.expected_judge_at() + usize::from(missing == 4);
            let event = tombstoned(k, r, open.event_id(), 1, 0, bad, judge_at);
            assert_eq!(
                reduce_unassigned(&[open.clone(), event], query(judge_at)),
                Err(LedgerError::TombstoneGateFailed),
                "missing conjunct {missing}"
            );
        }
        let tomb = tombstoned(
            k,
            r,
            open.event_id(),
            1,
            0,
            evidence,
            evidence.expected_judge_at(),
        );
        let view = reduce_unassigned(&[open, tomb], query(evidence.expected_judge_at())).unwrap();
        assert_eq!(view.entries[&k].disposition, Disposition::Tombstoned);
    }
}
