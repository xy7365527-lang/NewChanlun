//! S3 门户补齐：备战档（未决只读）+ 短差档（判败盘背观测，亚型签 + 三锁）+ 失败处置通知
//! （票 #623；#574 契约裁定八）。
//!
//! # 与成立档（S1）的关系
//!
//! [`super::book::ThirdPointPack`] / [`super::book::CenterDeathCertificate`] 是**成立档**，S1
//! 已交付，本模块零改动。本模块只补另外两档：
//!
//! - **备战档**（[`StandbyWatch`] + [`RetraceLedger::standby`]）：未决候选只读枚举，素材全部
//!   现算自 S1 留下的钩子（`RetraceEntry::registration()`），本模块不新增任何账本状态；
//! - **短差档**（[`ShortRetraceRecord`] + [`ShortRetracePortal`]）：`NotConstituted{RetestReentered}`
//!   判败事件转盘背观测记录（027:16），亚型签固定 + 三锁；
//! - **失败处置通知**（[`FailureDisposalNotice`] + [`RetraceLedger::failure_disposal_notices`]）：
//!   短差档的伴生输出，名义「通知非信号」。
//!
//! # 备战档禁区（裁定八「不许被消费成买入信号」）
//!
//! 影子评审 #621 LOW-4 指出：`RetraceEntry` 全字段 `pub`、`entries()`/`registration()` 无差别
//! 放行任意状态，备战档素材在 S1 就已对外全开，**若 S3 直接包这层当备战档，禁区就只剩注释在
//! 守**。本模块因此不复用 `RetraceEntry`：[`StandbyWatch`] 是与 [`super::book::ThirdPointPack`]
//! 结构不同、字段不同（无 `retest_end`/`confirmed_as_of`/`death_certificate`——未决候选压根没有
//! 这些材料）、且**互不实现同一 trait、互无 `From`/`Into`** 的独立类型。类型面隔离的证明见
//! [`StandbyWatch`] 文档内的 `compile_fail` doctest（`cargo test --doc` 编译期强制，非事后描述）。

use super::book::{RetraceLedger, ThirdPointPack};
use super::{
    CenterFrame, NotConstitutedReason, RetraceEntry, RetraceKey, RetracePoint, RetraceSide,
};

// ═══════════════════════════════════════════════════════════════════════════
// 备战档（裁定八①）：未决只读 + 类型面隔离
// ═══════════════════════════════════════════════════════════════════════════

/// 可安全消费为「买入信号」的证据包类型才实现的标记 trait（禁区闸门）。
///
/// 仅 [`ThirdPointPack`]（成立档，Confirmed 已判胜 + 死亡证明齐）实现本 trait；
/// [`StandbyWatch`]（备战档，未决）**故意不实现**——任何要求 `T: TradableSignal` 的下游消费
/// 函数收到 `StandbyWatch` 一律编译失败，不依赖运行时状态检查。
pub trait TradableSignal: Copy {}

impl TradableSignal for ThirdPointPack {}

/// 备战档条目：未决候选的只读盯梢面（位置 + 四条边快照 + 出生钟）。
///
/// 名义「盯这里」不是「买这里」（024:36：次级别回切入点归交易层，扳机不在本账）——本类型因此
/// **没有** `retest_end`（回抽尚未走完，诚实缺席）、没有 `confirmed_as_of`、没有死亡证明：
/// 未决候选压根不具备这些材料，字段表本身就是禁区的第一道墙。
///
/// **类型面隔离证明**（编译期，`cargo test --doc` 覆盖）：
///
/// ```compile_fail
/// use newchan_rust::theta_v0::classifier::retrace_ledger::{
///     CenterFrame, RetraceKey, RetracePoint, RetraceSide,
/// };
/// use newchan_rust::theta_v0::classifier::retrace_ledger::portal::{StandbyWatch, TradableSignal};
///
/// fn place_order<T: TradableSignal>(_signal: T) {}
///
/// let watch = StandbyWatch {
///     identity: RetraceKey {
///         frame: CenterFrame { zd: 100, zg: 200, start_index: 0, end_index: 10 },
///         departure_move_index: 3,
///     },
///     side: RetraceSide::Buy,
///     frame: CenterFrame { zd: 100, zg: 200, start_index: 0, end_index: 10 },
///     leave_end: RetracePoint { index: 20, price: 250 },
///     registered_as_of: 500,
/// };
/// place_order(watch); // 编译失败：StandbyWatch 未实现 TradableSignal
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandbyWatch {
    pub identity: RetraceKey,
    pub side: RetraceSide,
    /// 四条边快照（临时右边——候选活着期间窗口不动，裁定一）。
    pub frame: CenterFrame,
    /// 「离开的边」= leave 笔终点。
    pub leave_end: RetracePoint,
    /// 出生知情时。
    pub registered_as_of: usize,
}

impl RetraceLedger {
    /// 备战档：全部未决候选的只读盯梢面（按身份键序，与 [`RetraceLedger::established`] 同一
    /// 确定性口径）。
    pub fn standby(&self) -> Vec<StandbyWatch> {
        self.entries().filter_map(Self::watch_of).collect()
    }

    /// 单个身份的备战档条目；非 `Provisional` 或前缀态缺注册快照 ⟹ `None`。
    pub fn standby_watch(&self, key: &RetraceKey) -> Option<StandbyWatch> {
        self.entry(key).and_then(Self::watch_of)
    }

    fn watch_of(entry: &RetraceEntry) -> Option<StandbyWatch> {
        if entry.state != super::RetraceState::Provisional {
            return None;
        }
        let evidence = entry.registration()?;
        Some(StandbyWatch {
            identity: entry.key,
            side: evidence.side,
            frame: evidence.frame,
            leave_end: evidence.leave_end,
            registered_as_of: entry.registered_as_of,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 短差档（裁定八③）：判败盘背观测记录，亚型签固定
// ═══════════════════════════════════════════════════════════════════════════

/// 短差档亚型签：判败盘背观测记录的固定类别标记。
///
/// 短差通道（027:16 盘整背驰技术含义）只消费竞选失败一族；引擎改口
/// （[`NotConstitutedReason::CenterRebased`]）走 S2 警报桶，**不进本门户**——枚举只留一个变体，
/// 把「固定」写进类型而非注释：新增变体是显式的破坏性改动，不是静默扩权。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanDivSubtype {
    /// 回抽重回快照框内（#574 裁定三判败四行语义：候选从未成立/中枢未破坏/无中枢事件/盘背观测源）。
    RetestReentered,
}

/// 短差档记录：一条判败盘背观测（裁定八③）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortRetraceRecord {
    pub identity: RetraceKey,
    pub subtype: PanDivSubtype,
    pub side: RetraceSide,
    /// 四条边快照（判败：中枢未破坏，快照未转正、仍是原框）。
    pub frame: CenterFrame,
    pub leave_end: RetracePoint,
    /// 判败位置 = 回抽笔终点（回抽已重回框内，位置必齐）。
    pub retest_end: RetracePoint,
    pub registered_as_of: usize,
    /// 落锤知情时（判败知情时）。
    pub judged_as_of: usize,
}

/// 已入场者失败处置通知（裁定八③「通知非信号」）。
///
/// **名义边界**：本通知对**任一**判败候选都发出，不判断是否真的已有仓位——账本不进口外部
/// 状态做消费过滤（总禁区）。是否有仓位、是否需要处置，由交易层自己核对身份 + 位置后决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureDisposalNotice {
    pub identity: RetraceKey,
    pub side: RetraceSide,
    /// 位置 = 判败回抽笔终点。
    pub position: RetracePoint,
    /// 知情时 = 落锤知情时。
    pub known_as_of: usize,
}

impl RetraceLedger {
    /// 短差档源：全部判败身份的盘背观测记录（按身份键序）。
    pub fn short_retrace_records(&self) -> Vec<ShortRetraceRecord> {
        self.entries().filter_map(Self::short_retrace_record_of).collect()
    }

    /// 失败处置通知：短差档的伴生输出，逐条判败各发一份（名义「通知非信号」）。
    pub fn failure_disposal_notices(&self) -> Vec<FailureDisposalNotice> {
        self.short_retrace_records().iter().map(FailureDisposalNotice::from_record).collect()
    }

    fn short_retrace_record_of(entry: &RetraceEntry) -> Option<ShortRetraceRecord> {
        if entry.not_constituted_reason() != Some(NotConstitutedReason::RetestReentered) {
            return None;
        }
        let evidence = entry.terminal_evidence()?;
        Some(ShortRetraceRecord {
            identity: entry.key,
            subtype: PanDivSubtype::RetestReentered,
            side: evidence.side,
            frame: evidence.frame,
            leave_end: evidence.leave_end,
            retest_end: evidence
                .retest_end
                .expect("判败必带回抽位置：重回快照框内已发生（适配器 missing 桶已挡）"),
            registered_as_of: entry.registered_as_of,
            judged_as_of: entry.terminal_as_of.expect("Invalidated 必有落锤钟"),
        })
    }
}

impl FailureDisposalNotice {
    fn from_record(record: &ShortRetraceRecord) -> Self {
        Self {
            identity: record.identity,
            side: record.side,
            position: record.retest_end,
            known_as_of: record.judged_as_of,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 短差档三锁（#606 S1 先例：键唯一 / 一一对应去重 / 账平断言）
// ═══════════════════════════════════════════════════════════════════════════

/// 短差档登记冲突（三锁之一：键唯一）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortRetraceRejection {
    /// 同一身份已登记过短差记录。
    DuplicateIdentity(RetraceKey),
}

/// 短差档接收器：判败盘背观测记录的键唯一登记 + 幂等消费。
///
/// **三锁**：
/// 1. **键唯一**——[`Self::admit`] 对已登记身份冲突拒绝（[`ShortRetraceRejection::DuplicateIdentity`]）；
/// 2. **一一对应去重**——[`Self::sync`] 只登记账本判败集合中尚未登记的身份，账本自身按 `RetraceKey`
///    去重（一个身份至多一条判败记录，内核终态禁复活保证），故同步天然一一对应；
/// 3. **账平断言**——[`Self::balances_with`] 供验收行使用：本档条数须等于账本判败条数。
#[derive(Debug, Clone, Default)]
pub struct ShortRetracePortal {
    records: std::collections::BTreeMap<RetraceKey, ShortRetraceRecord>,
    consumed: std::collections::BTreeSet<RetraceKey>,
}

impl ShortRetracePortal {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记单条短差记录；同一身份重复登记 = 冲突拒绝（锁一）。
    pub fn admit(&mut self, record: ShortRetraceRecord) -> Result<(), ShortRetraceRejection> {
        if self.records.contains_key(&record.identity) {
            return Err(ShortRetraceRejection::DuplicateIdentity(record.identity));
        }
        self.records.insert(record.identity, record);
        Ok(())
    }

    /// 与账本判败集合同步：只登记尚未登记过的身份（锁二，一一对应去重）。
    pub fn sync(&mut self, ledger: &RetraceLedger) {
        for record in ledger.short_retrace_records() {
            self.records.entry(record.identity).or_insert(record);
        }
    }

    /// 幂等消费：同一身份首次消费返回记录并标记已消费；重复消费返回 `None`（不重复处置）。
    pub fn consume(&mut self, identity: &RetraceKey) -> Option<ShortRetraceRecord> {
        if !self.consumed.insert(*identity) {
            return None;
        }
        self.records.get(identity).copied()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// 账平断言（锁三）：本档条数 == 账本判败条数。
    pub fn balances_with(&self, ledger: &RetraceLedger) -> bool {
        self.records.len() == ledger.short_retrace_records().len()
    }

    /// 失败处置通知：已登记的每条记录各发一份（名义「通知非信号」）。
    pub fn disposal_notices(&self) -> Vec<FailureDisposalNotice> {
        self.records.values().map(FailureDisposalNotice::from_record).collect()
    }
}
