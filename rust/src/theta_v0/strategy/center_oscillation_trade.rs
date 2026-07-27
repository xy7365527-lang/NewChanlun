//! 狭义短差动作本体（SPEC #274 T2，issue #292）。
//!
//! ADR 0001 修正案一/补充二裁定 + #292 前置约束五条（#337 评审论证，2026-07-26）：
//! - **A**：触发用主格——`alive_center()`（T1 [`center_lifecycle::CenterEventMachine`]）语义
//!   未变，本模块直接消费其结果，不另立第二套「在场」判据。
//! - **B**：superseded（[`center_lifecycle::DeathForm::ArenaTermination`]，被链推进取代）是
//!   挂起短差的终结触发源之一——在场终结**不新增出口**，仍只走「回补/终结」二分的终结支。
//! - **C**：终结动作**幂等**——主流程常态是同一实例先收在场终结、后收教义死亡两条信号
//!   （#337 容读法），第二条信号必须是 no-op，不得重复产出/panic。
//! - **D**：挂起按**中枢身份**（[`center_lifecycle::CenterId`]，经 `killed_center_id()`）匹配，
//!   不按当前在场——链已推进换代后，终结事件仍须命中它所指的那个挂起实例。
//! - **E**：接住首次教义死亡——即便某实例从未被记录过在场终结（无前置 Superseded），一旦
//!   收到教义死亡（Broken/Reset）也要正确终结，不依赖「先 superseded 才能终结」的隐含前提。
//! - **F**（issue #292 续修，二轮评审浮出）：链**重基**（[`center_lifecycle::ChainConsumed::Rebased`]）
//!   到达时，挂起按身份三元组核对重基后的新链——身份仍在链上⟹**跟随迁移**（挂起状态原样保留；
//!   本机挂起表键本身就是身份，不含链下标侧车，故迁移是保状态的 no-op）；身份从新链上消失⟹
//!   **终结**（[`SuspensionTerminationSource::RebaseVanished`]，与 `Superseded` 同形态：不回补，
//!   承诺作废）。禁悬空——[`CenterOscillationBook::on_chain_rebase`] 尾部机检断言：处理后任何
//!   仍挂起的身份都必须在新链上，不留「既非迁移又非终结」的第三态。
//!
//! ## 范围边界（与 T3/#293、T4/#294 分工）
//!
//! 本票只产出**动作决策**（Reduce/Replenish）与**挂起状态机**（出口二分），不触碰账本/成本基
//! ——「本仓成本基不动，短差往返盈亏记该级账内『短差盈亏』标签桶」的**记账实装**（TwEvent::
//! ShortDiff 成本基划转 + 恒仓断言）是 #293（T3）的职责；「每仓 campaign 粒度」是 #294（T4）
//! 的职责。本模块的 `CenterOscillationAction` 是这两票的输入契约，不是它们的实现。
//!
//! ## 命名纪律（票面条款 9）
//!
//! 机制名 = [`CenterOscillationAction`] / [`CenterOscillationTrigger`] / [`CenterOscillationBook`]
//! ——与 `ReverseOpen`/`OscillationBook`（#282 已删的 S6 配对子腿账面形态）严格区分；
//! 禁止任何形式复用「ShortDiff」作类型/字段命名（S6 已废止形态的名字不得回魂）。

use super::super::classifier::center_lifecycle::{CenterId, CenterLifecycleEvent};
use super::super::types::{Center, Side, Tick};
use super::oscillation::{BoundarySide, PanDivTrigger};
use super::voice::VoiceSide;
use std::collections::{BTreeMap, BTreeSet};

/// 触发构造的 typed 拒绝（无静默兜底）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerError {
    /// 本级中枢不在场（`alive_center()` 为 `None`）——A 裁定：无主格即无触发。
    CenterNotAlive,
    /// 次级别信号方向为 Flat——触发必须有向。
    FlatSignal,
    /// #292 B 裁定：信号方向圈定的候选边界侧与价格不符——次级别卖点但价格不在本级在场中枢
    /// 上半区，或次级别买点但价格不在下半区。信号方向不再单独决定边界侧；不满足价格判据
    /// 即整体拒绝构造（不是"仍构造但方向作废"）。
    PriceOutsideZone,
}

/// 触发事件：本级中枢在场（主格，A 裁定）+ 次级别买卖点（B 裁定：触发主信号源，盘背降为
/// 辅助参考、非必要条件）+ 边界侧（独立价格判据，B 裁定细化，2026-07-26）。不携任何账面
/// 语义（无数量、无成本基）——动作决策见 [`CenterOscillationBook::on_trigger`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterOscillationTrigger {
    level: u32,
    center: CenterId,
    boundary_side: BoundarySide,
    signal_side: VoiceSide,
    source_index: usize,
}

impl CenterOscillationTrigger {
    /// 唯一构造点：本级 `alive_center()` 身份（A 裁定）+ 次级别买卖点信号方向 + 该点价格 →
    /// 边界侧。★#292 B 裁定：边界侧不再单独由信号方向决定，而是信号方向圈定候选边界后，
    /// 再由价格相对本级在场中枢（`alive` 的 zd/zg）的半区位置独立判定：
    ///
    /// - 次级别卖点（`Short`）候选边界=`Above`（上沿高抛试探）——仅当 `price` 落在中枢
    ///   **上半区**（`price >= 中轴`，中轴=`(zd + zg) / 2`）才放行。
    /// - 次级别买点（`Long`）候选边界=`Below`（下沿回补试探）——仅当 `price` 落在中枢
    ///   **下半区**（`price <= 中轴`）才放行。
    /// - 不满足对应半区 ⟹ `Err(PriceOutsideZone)`，整体拒绝构造（不是"仍构造但方向作废"）。
    /// - 中轴恰等（`price == 中轴`）两侧均计入（`>=`/`<=` 含端点，非开区间）。
    ///
    /// 中轴取整数除法 `(zd + zg) / 2`（向零截断；price ticks 恒正故等价向下取整——实现决策，
    /// #292 票面「中轴阈值为实现决策」的落点，可复检）。
    ///
    /// **无门（机械断言）**：签名唯一输入 = 本级在场身份 + 次级别信号事实（方向+价格+坐标），
    /// 不接受、也无法接受任何次级别**账户/仓位状态**——编译期即杜绝互斥门的可能。
    pub fn new(
        level: u32,
        alive: Option<CenterId>,
        signal_side: VoiceSide,
        price: Tick,
        source_index: usize,
    ) -> Result<Self, TriggerError> {
        let center = alive.ok_or(TriggerError::CenterNotAlive)?;
        let boundary_side = match signal_side {
            VoiceSide::Long => BoundarySide::Below,
            VoiceSide::Short => BoundarySide::Above,
            VoiceSide::Flat => return Err(TriggerError::FlatSignal),
        };
        let midpoint = (center.zd + center.zg) / 2;
        let in_zone = match boundary_side {
            BoundarySide::Above => price >= midpoint,
            BoundarySide::Below => price <= midpoint,
        };
        if !in_zone {
            return Err(TriggerError::PriceOutsideZone);
        }
        Ok(Self::from_parts(level, center, boundary_side, signal_side, source_index))
    }

    /// 已核验字段的直接构造（跳过 `new` 的价格半区判据）——供 `from_pan_div_trigger` 使用，
    /// 因为 `PanDivTrigger` 不携价格，边界侧只能原样传导，不适用独立价格判据。
    fn from_parts(
        level: u32,
        center: CenterId,
        boundary_side: BoundarySide,
        signal_side: VoiceSide,
        source_index: usize,
    ) -> Self {
        Self { level, center, boundary_side, signal_side, source_index }
    }

    /// #292 接线点二：`oscillation::PanDivTrigger`（#282 保留触发链）→ 本触发的转换（票面
    /// 「五要素映射」：级别/信号方向/中枢身份/边界侧/盘背证据 → 本触发的四项构造输入）。
    ///
    /// - **级别**：`pan_div.level()` 原样传入。
    /// - **信号方向**：`pan_div.signal_side()` 原样传入。
    /// - **边界侧**：`pan_div.boundary_side()` 原样传入——`PanDivTrigger` 不携价格，无法套用
    ///   `new` 的独立价格判据（B 裁定细化只约束生产开启臂的直接构造点），故本转换绕过 `new`
    ///   经 `from_parts` 直接构造；两处仍是同一 `Long→Below`/`Short→Above` 方向约定
    ///   （`pan_div_conversion_boundary_side_matches_source` 核对相等），只是本路径不做价格校验。
    /// - **中枢身份**：**不**取 `pan_div.center()`（那只是 `OscillationCenterRef` 坐标投影，
    ///   缺 zd/zg 核心区间，不是 `CenterId`）——按 A 裁定，身份唯一源 = 调用方传入的本级
    ///   `alive_center()`（经 `CenterId::of` 投影），不复用 PanDiv 自带坐标另立第二套「在场」
    ///   判据。`alive=None` ⟹ `Err(CenterNotAlive)`（与 `new` 同一失败面）。
    /// - **盘背证据**：`pan_div.evidence()` 降格为 `source_index`（`reference().source_index()`）
    ///   ——本触发类型面不携证据类型本身，只留可追溯坐标。
    ///
    /// ★★#292 触发源改码（用户裁定 2026-07-26，票面 B）：生产开启臂的触发主信号源已改为
    /// **次级别买卖点**（`CenterOscillationTrigger::new` 直收次级别信号方向+价格，见上）——
    /// 盘背**非必要条件**，本方法降格为**可选辅助转换**（保留 API，供未来盘背过滤/标注等辅助
    /// 用途；`fill.rs::step_center_oscillation` 生产驱动路径已不再调用它，唯一驱动源是次级别
    /// 买卖点）。
    pub fn from_pan_div_trigger(
        pan_div: PanDivTrigger,
        alive: Option<CenterId>,
    ) -> Result<Self, TriggerError> {
        let center = alive.ok_or(TriggerError::CenterNotAlive)?;
        Ok(Self::from_parts(
            pan_div.level(),
            center,
            pan_div.boundary_side(),
            pan_div.signal_side(),
            pan_div.evidence().reference().source_index(),
        ))
    }

    pub const fn level(self) -> u32 {
        self.level
    }

    pub const fn center(self) -> CenterId {
        self.center
    }

    pub const fn boundary_side(self) -> BoundarySide {
        self.boundary_side
    }

    pub const fn signal_side(self) -> VoiceSide {
        self.signal_side
    }

    pub const fn source_index(self) -> usize {
        self.source_index
    }
}

/// 本级账动作（票面条款 9：与 `ReverseOpen` 族严格区分，禁 `ShortDiff` 命名）。
///
/// - `Reduce`（上沿高抛）：本级账记卖出，实现盈亏进短差盈亏标签桶（T3 接线）；本仓成本基不动。
/// - `Replenish`（下沿回补）：本级账记买入；本仓成本基不动（唯二出口=全平清零/加仓重算，本模块
///   两者都不触碰）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterOscillationAction {
    Reduce,
    Replenish,
}

/// #292 门控接线可观测轨迹（fill.rs 开启臂产出）：一条「触发 → 减补动作」的可见记录。
/// 只是接线验收证据——真实记账（本仓成本基/短差盈亏标签桶）是 T3（#293）的职责，本记录
/// 不携任何账面语义。门关（`center_oscillation.enabled=false`）⟹ 本类型全程不构造（诚实空）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterOscillationActionRecord {
    pub bar: usize,
    pub level: u32,
    pub center: CenterId,
    pub action: CenterOscillationAction,
}

/// 挂起短差的终结来源（ADR 补充二 + #292 前置约束五条 + #292 续修）。五源同走「终结」出口，
/// 互不重叠。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionTerminationSource {
    /// 本级三类**买**点破坏（`Broken{breaker_side: Long}`）：终结伴随一次收手回补——
    /// 「高抛后出三类买点则于三类买点处回补」（049/068），回补动作来自这条真实证书本身，
    /// 不是凭空生成（幽灵回补=0 反例锚的正面形态）。
    BrokenByThirdClassBuy,
    /// 本级三类**卖**点破坏（`Broken{breaker_side: Short}`）：终结不回补——
    /// 「出三类卖点则不回补」，仓位留待新证书才可能再开（SPEC Out of Scope 已如实标注为
    /// 未细化口径，本机按此执行）。
    BrokenByThirdClassSell,
    /// 本级一类点全平（该级走势类型终结）。★暂定口径（票面项 8，待编排者裁）：清空**整场**
    /// 全部挂起身份，而非仅同死的那一个中枢——与该级走势类型终结同精神（Q2 同款先例）。
    Reset,
    /// 被链推进取代（[`center_lifecycle::DeathForm::ArenaTermination`]，B 裁定）：终结不回补
    /// ——承诺作废，禁任何形式复活（无本级买点证书不得开仓）。
    Superseded,
    /// ★#292 续修（二轮评审浮出的挂起悬空泄漏修复）：链重基（[`center_lifecycle::ChainConsumed::Rebased`]）
    /// 后，挂起对应的中枢身份已不在新链上——该实例连「被取代」的记录都没有，是工程重基这一
    /// 侧信道的失踪，与 `Superseded` 同形态终结：不回补，承诺作废，禁任何形式复活。判据 =
    /// [`CenterOscillationBook::on_chain_rebase`] 逐挂起身份核对重基后的新链。
    RebaseVanished,
}

/// 一次终结的产出：身份 + 来源 + 是否伴随一次收手回补动作（仅 [`SuspensionTerminationSource::BrokenByThirdClassBuy`]
/// 可能非 `None`，且仅当该身份此前确在挂起中）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuspensionOutcome {
    pub center: CenterId,
    pub source: SuspensionTerminationSource,
    pub cover_action: Option<CenterOscillationAction>,
}

/// 挂起短差状态机（每级别一台，与 T1 [`center_lifecycle::CenterEventMachine`] 同粒度）。
///
/// 身份 → 是否挂起：只存 `Suspended` 身份；不存在于表中 = 未挂起（含「从未挂起过」与
/// 「已终结」两种情形——两者对外行为相同：回补请求一律拒绝，终结信号一律 no-op，天然满足
/// C 裁定的幂等要求，无需额外的 `Terminated` 哨兵态）。
///
/// ★#292 H1（域层评审）：`BTreeMap` 非 `HashMap`——`on_lifecycle_event` 的 `Reset` 分支需要
/// 把「当前挂起的全部身份」投影成 `Vec` 输出（清空整场，票面项 8），`HashMap::keys()` 迭代序
/// 依赖默认哈希（跨进程/跨版本不确定）；`BTreeMap` 按 `CenterId` 派生序确定性迭代，wf8
/// bit-exact 回归不受哈希实现变化影响（exit.rs `step_active_set_with_subtree_close` 的
/// `HashSet` 反例：那里只做 `.contains()` membership 查询、从不迭代输出，故哈希序无关；
/// 本处迭代序直接进产出 `Vec` 顺序，二者边界正在于「迭代是否进输出」）。
#[derive(Debug, Default)]
pub struct CenterOscillationBook {
    level: u32,
    suspended: BTreeMap<CenterId, ()>,
}

impl CenterOscillationBook {
    pub fn new(level: u32) -> Self {
        Self { level, suspended: BTreeMap::new() }
    }

    pub const fn level(&self) -> u32 {
        self.level
    }

    /// 当前挂起的身份数（诊断/测试用）。
    pub fn suspended_count(&self) -> usize {
        self.suspended.len()
    }

    pub fn is_suspended(&self, center: CenterId) -> bool {
        self.suspended.contains_key(&center)
    }

    /// 处理一次触发 ⟹ 减/补动作二选一，或幽灵回补拒绝（`None`）。
    ///
    /// - 上沿高抛（`Above`）：无论此前是否已挂起，均记一次 `Reduce`（重复触碰上沿=继续高抛，
    ///   挂起态置位/保持）。
    /// - 下沿回补（`Below`）：**仅当**该中枢身份当前处于挂起中才放行 `Replenish`（回补出口之一）
    ///   ——否则返回 `None`（P6 反例锚：全平/毁中枢后无本级买点证书不许回补，即「幽灵回补」）。
    ///
    /// **无门**：本函数不读取、也无法读取任何次级别账户状态——签名唯一输入是触发事实本身。
    pub fn on_trigger(&mut self, trigger: CenterOscillationTrigger) -> Option<CenterOscillationAction> {
        assert_eq!(
            trigger.level(),
            self.level,
            "触发级别必须匹配本机级别（跨级误喂是接线错误，不是本机决策范围）"
        );
        match trigger.boundary_side() {
            BoundarySide::Above => {
                self.suspended.insert(trigger.center(), ());
                Some(CenterOscillationAction::Reduce)
            }
            BoundarySide::Below => {
                if self.suspended.remove(&trigger.center()).is_some() {
                    Some(CenterOscillationAction::Replenish)
                } else {
                    None
                }
            }
        }
    }

    /// 消费一条 T1 中枢生命周期事件 ⟹ 0 或多条终结产出（`Reset` 的「清空整场」暂定口径可能
    /// 一次终结多个挂起身份；其余事件至多终结一个）。
    ///
    /// 身份匹配走 `event.killed_center_id()`（D 裁定：按身份，不按当前在场）；`Born` 事件与
    /// 未挂起的身份均 no-op（C 裁定：幂等——目标身份不在挂起表中，天然产出空 Vec）。
    pub fn on_lifecycle_event(&mut self, event: &CenterLifecycleEvent) -> Vec<SuspensionOutcome> {
        if matches!(event, CenterLifecycleEvent::Reset { .. }) {
            if self.suspended.is_empty() {
                return Vec::new();
            }
            let ids: Vec<CenterId> = self.suspended.keys().copied().collect();
            self.suspended.clear();
            return ids
                .into_iter()
                .map(|center| SuspensionOutcome {
                    center,
                    source: SuspensionTerminationSource::Reset,
                    cover_action: None,
                })
                .collect();
        }
        let Some(id) = event.killed_center_id() else {
            return Vec::new(); // Born，或场为空的 Reset（上面分支已处理非空 Reset）。
        };
        let source = match event {
            CenterLifecycleEvent::Broken { breaker_side: Side::Long, .. } => {
                SuspensionTerminationSource::BrokenByThirdClassBuy
            }
            CenterLifecycleEvent::Broken { breaker_side: Side::Short, .. } => {
                SuspensionTerminationSource::BrokenByThirdClassSell
            }
            CenterLifecycleEvent::Superseded { .. } => SuspensionTerminationSource::Superseded,
            CenterLifecycleEvent::Reset { .. } | CenterLifecycleEvent::Born { .. } => {
                return Vec::new();
            }
        };
        if self.suspended.remove(&id).is_none() {
            return Vec::new(); // 该身份未挂起（从未挂起 / 已终结过）⟹ 幂等 no-op。
        }
        let cover_action = matches!(source, SuspensionTerminationSource::BrokenByThirdClassBuy)
            .then_some(CenterOscillationAction::Replenish);
        vec![SuspensionOutcome { center: id, source, cover_action }]
    }

    /// 消费一次链**重基**（[`center_lifecycle::ChainConsumed::Rebased`]）⟹ 0 或多条终结产出
    /// （F 裁定，issue #292 续修）。
    ///
    /// 重基是工程再同步（前缀分叉/该级塔缓存全量重置），不是教义生死——本机不产 `Reset`/`Broken`/
    /// `Superseded` 之外的第三种教义事件，只按**身份**核对新链：
    /// - 挂起身份仍在新链上（任意下标，不要求仍是链尾/容读格）⟹ **跟随迁移**：挂起状态原样
    ///   保留（本机挂起表的键本身就是身份三元组，不含链下标侧车，迁移不改变任何字段，是
    ///   保状态的 no-op，故本函数不为它产出任何 `SuspensionOutcome`）。
    /// - 挂起身份不在新链上 ⟹ **终结**：与 `Superseded` 同形态（[`SuspensionTerminationSource::RebaseVanished`]），
    ///   不回补，承诺作废。
    ///
    /// **禁悬空（机检断言）**：处理后仍挂起的身份必须全部在新链上——不留「既非迁移又非终结」
    /// 的第三态；这是本函数的构造性不变量（逐身份要么留要么删），断言只是把它显式钉死。
    pub fn on_chain_rebase(&mut self, chain: &[Center]) -> Vec<SuspensionOutcome> {
        if self.suspended.is_empty() {
            return Vec::new();
        }
        let chain_ids: BTreeSet<CenterId> = chain.iter().map(CenterId::of).collect();
        let vanished: Vec<CenterId> =
            self.suspended.keys().copied().filter(|id| !chain_ids.contains(id)).collect();
        for id in &vanished {
            self.suspended.remove(id);
        }
        let outcomes: Vec<SuspensionOutcome> = vanished
            .into_iter()
            .map(|center| SuspensionOutcome {
                center,
                source: SuspensionTerminationSource::RebaseVanished,
                cover_action: None,
            })
            .collect();
        debug_assert!(
            self.suspended.keys().all(|id| chain_ids.contains(id)),
            "禁悬空：重基核对后任何仍挂起的身份都必须在新链上（迁移分支的机检不变量）"
        );
        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::types::{Center, Tick};

    fn cid(start_index: usize, zd: Tick, zg: Tick) -> CenterId {
        CenterId::of(&Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index, end_index: start_index + 50 })
    }

    fn broken(center_id: CenterId, breaker_side: Side) -> CenterLifecycleEvent {
        let c = Center {
            zd: center_id.zd,
            zg: center_id.zg,
            dd: center_id.zd - 2,
            gg: center_id.zg + 2,
            start_index: center_id.start_index,
            end_index: center_id.start_index + 50,
        };
        CenterLifecycleEvent::Broken { level: 0, center: c, chain_index: 0, breaker_source_index: 999, breaker_side }
    }

    fn superseded(center_id: CenterId) -> CenterLifecycleEvent {
        let c = Center {
            zd: center_id.zd,
            zg: center_id.zg,
            dd: center_id.zd - 2,
            gg: center_id.zg + 2,
            start_index: center_id.start_index,
            end_index: center_id.start_index + 50,
        };
        CenterLifecycleEvent::Superseded { level: 0, center: c, chain_index: 0, by_chain_index: 1 }
    }

    fn reset_with(died: Option<CenterId>) -> CenterLifecycleEvent {
        let died_center = died.map(|id| Center {
            zd: id.zd,
            zg: id.zg,
            dd: id.zd - 2,
            gg: id.zg + 2,
            start_index: id.start_index,
            end_index: id.start_index + 50,
        });
        CenterLifecycleEvent::Reset {
            level: 0,
            died_center,
            died_chain_index: died.map(|_| 0),
            trigger_source_index: 500,
            trigger_side: Side::Long,
        }
    }

    // ── 触发（A 裁定 + 边界映射） ──────────────────────────────────────────

    #[test]
    fn trigger_rejects_when_center_not_alive() {
        assert_eq!(
            CenterOscillationTrigger::new(0, None, VoiceSide::Long, 0, 10),
            Err(TriggerError::CenterNotAlive),
            "A 裁定：无主格即无触发"
        );
    }

    #[test]
    fn trigger_rejects_flat_signal() {
        let id = cid(5, 100, 200);
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Flat, 0, 10),
            Err(TriggerError::FlatSignal)
        );
    }

    #[test]
    fn trigger_maps_signal_side_to_boundary_same_convention_as_pan_div() {
        let id = cid(5, 100, 200);
        let buy = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 10).unwrap();
        assert_eq!(buy.boundary_side(), BoundarySide::Below, "次级别买点=下沿回补试探");
        let sell = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 11).unwrap();
        assert_eq!(sell.boundary_side(), BoundarySide::Above, "次级别卖点=上沿高抛试探");
    }

    // ── #292 B 裁定：边界侧独立价格判据（四态 + 中轴恰等） ──────────────────

    /// 状态①：上沿区卖点 ⟹ 高抛（`Above`）。
    #[test]
    fn sell_signal_in_upper_half_produces_above_boundary() {
        let id = cid(5, 100, 200); // 中轴 = 150
        let t = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, 151, 10).unwrap();
        assert_eq!(t.boundary_side(), BoundarySide::Above);
    }

    /// 状态②：非上沿区卖点 ⟹ 整体拒绝（不是"仍构造但方向作废"）。
    #[test]
    fn sell_signal_below_upper_half_is_rejected() {
        let id = cid(5, 100, 200); // 中轴 = 150
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, 149, 10),
            Err(TriggerError::PriceOutsideZone),
            "卖点但价格在中轴以下 ⟹ 不构成高抛触发"
        );
    }

    /// 状态③：下沿区买点 ⟹ 回补（`Below`）。
    #[test]
    fn buy_signal_in_lower_half_produces_below_boundary() {
        let id = cid(5, 100, 200); // 中轴 = 150
        let t = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, 149, 10).unwrap();
        assert_eq!(t.boundary_side(), BoundarySide::Below);
    }

    /// 状态④：非下沿区买点 ⟹ 整体拒绝。
    #[test]
    fn buy_signal_above_lower_half_is_rejected() {
        let id = cid(5, 100, 200); // 中轴 = 150
        assert_eq!(
            CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, 151, 10),
            Err(TriggerError::PriceOutsideZone),
            "买点但价格在中轴以上 ⟹ 不构成回补触发"
        );
    }

    /// 中轴恰等：`(zd+zg)/2` 本身同时计入两侧（`>=`/`<=` 均含端点，非开区间）。
    #[test]
    fn price_exactly_at_midpoint_counts_as_in_zone_for_both_sides() {
        let id = cid(5, 100, 200); // 中轴 = 150
        let sell = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, 150, 10).unwrap();
        assert_eq!(sell.boundary_side(), BoundarySide::Above, "中轴恰等计入上半区（>=）");
        let buy = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, 150, 11).unwrap();
        assert_eq!(buy.boundary_side(), BoundarySide::Below, "中轴恰等计入下半区（<=）");
    }

    // ── 减补动作 ──────────────────────────────────────────────────────────

    #[test]
    fn upper_boundary_touch_emits_reduce_and_opens_suspension() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let t = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        assert_eq!(book.on_trigger(t), Some(CenterOscillationAction::Reduce));
        assert!(book.is_suspended(id));
    }

    #[test]
    fn repeated_upper_boundary_touches_keep_emitting_reduce() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let t1 = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        let t2 = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 20).unwrap();
        assert_eq!(book.on_trigger(t1), Some(CenterOscillationAction::Reduce));
        assert_eq!(book.on_trigger(t2), Some(CenterOscillationAction::Reduce), "重复触碰上沿=继续高抛");
        assert!(book.is_suspended(id));
    }

    #[test]
    fn lower_boundary_touch_while_suspended_covers() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let reduce = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        let cover = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 20).unwrap();
        book.on_trigger(reduce);
        assert_eq!(book.on_trigger(cover), Some(CenterOscillationAction::Replenish));
        assert!(!book.is_suspended(id), "回补出口=挂起清空");
    }

    /// ★幽灵回补=0（原型 P6 反例锚）：未挂起时下沿信号不产任何动作。
    #[test]
    fn ghost_replenish_without_suspension_is_rejected() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let cover = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 20).unwrap();
        assert_eq!(book.on_trigger(cover), None, "无挂起证书 ⟹ 禁复活");
        assert_eq!(book.suspended_count(), 0);
    }

    // ── 挂起出口二分（5 场景，票面「验收」条款） ───────────────────────────

    /// 出口①：下沿回补（正常出口）——已在 `lower_boundary_touch_while_suspended_covers` 覆盖，
    /// 此处补一条端到端多轮（高抛→回补→再高抛→再回补）确认状态机可重复进出。
    #[test]
    fn suspension_can_reopen_after_a_full_cover_cycle() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        let reduce = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        let cover = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 20).unwrap();
        assert_eq!(book.on_trigger(reduce), Some(CenterOscillationAction::Reduce));
        assert_eq!(book.on_trigger(cover), Some(CenterOscillationAction::Replenish));
        assert_eq!(book.on_trigger(reduce), Some(CenterOscillationAction::Reduce), "回补后可再度高抛");
        assert_eq!(book.on_trigger(cover), Some(CenterOscillationAction::Replenish));
    }

    /// 出口②：三类买点处收手回补——终结伴随一次真实回补（幽灵回补=0 的正面形态：回补来自
    /// 这条破坏证书本身，不是凭空生成）。
    #[test]
    fn suspension_exit_third_class_buy_terminates_with_cover() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        let outcomes = book.on_lifecycle_event(&broken(id, Side::Long));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::BrokenByThirdClassBuy);
        assert_eq!(outcomes[0].cover_action, Some(CenterOscillationAction::Replenish));
        assert!(!book.is_suspended(id), "终结=挂起清空");
    }

    /// 出口③：全平终结（本级一类点，暂定口径=清空整场，票面项 8）——不回补。
    #[test]
    fn suspension_exit_full_close_reset_terminates_without_cover() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        let outcomes = book.on_lifecycle_event(&reset_with(Some(id)));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::Reset);
        assert_eq!(outcomes[0].cover_action, None, "全平终结不回补");
        assert!(!book.is_suspended(id));
    }

    /// ★暂定口径实证：Reset 清空**整场**——即便同死身份与挂起身份不是同一个（场为空的一类点
    /// 边界记录、或挂起身份早于当前在场中枢），挂起表仍被整体清空（该级走势类型终结）。
    #[test]
    fn reset_clears_entire_level_suspension_book_not_just_the_died_identity() {
        let stale_suspended = cid(5, 100, 200); // 早于当前在场的挂起遗留（D 裁定：按身份不按在场）
        let currently_alive = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(stale_suspended), VoiceSide::Short, stale_suspended.zg, 10).unwrap());
        book.on_trigger(CenterOscillationTrigger::new(0, Some(currently_alive), VoiceSide::Short, currently_alive.zg, 20).unwrap());
        assert_eq!(book.suspended_count(), 2);
        let outcomes = book.on_lifecycle_event(&reset_with(Some(currently_alive)));
        assert_eq!(outcomes.len(), 2, "清空整场=两个挂起身份同时终结，非仅同死的那一个");
        assert_eq!(book.suspended_count(), 0);
    }

    /// 出口④：三类卖点终结——不回补（承诺作废，「不回补」与「终结」是同一事件的两个描述）。
    #[test]
    fn suspension_exit_third_class_sell_terminates_without_cover() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        let outcomes = book.on_lifecycle_event(&broken(id, Side::Short));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::BrokenByThirdClassSell);
        assert_eq!(outcomes[0].cover_action, None);
        assert!(!book.is_suspended(id));
    }

    /// 出口⑤：中枢破坏终结的另一面——被链推进取代（superseded，B 裁定）同走终结出口，不回补。
    #[test]
    fn suspension_exit_superseded_arena_termination_without_cover() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        let outcomes = book.on_lifecycle_event(&superseded(id));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::Superseded);
        assert_eq!(outcomes[0].cover_action, None, "承诺作废，禁任何形式复活");
        assert!(!book.is_suspended(id));
    }

    /// 终结后幽灵回补=0：任何终结出口之后，同身份的下沿信号必须被拒绝（不得凭空复活仓位）。
    #[test]
    fn ghost_replenish_after_any_termination_source_is_always_zero() {
        for (label, event) in [
            ("third_class_sell", broken(cid(5, 100, 200), Side::Short)),
            ("superseded", superseded(cid(5, 100, 200))),
            ("reset", reset_with(Some(cid(5, 100, 200)))),
        ] {
            let id = cid(5, 100, 200);
            let mut book = CenterOscillationBook::new(0);
            book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
            book.on_lifecycle_event(&event);
            let cover_attempt = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Long, id.zd, 30).unwrap();
            assert_eq!(book.on_trigger(cover_attempt), None, "{label}: 终结后必须拒绝幽灵回补");
        }
    }

    // ── 幂等（C 裁定） ────────────────────────────────────────────────────

    /// 主流程常态：同一实例先收在场终结（superseded）、后收教义死亡（broken）——第二条信号
    /// 必须是 no-op（幂等），不得重复产出终结事件。
    #[test]
    fn termination_is_idempotent_across_duplicate_dual_signals() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        let first = book.on_lifecycle_event(&superseded(id));
        assert_eq!(first.len(), 1, "第一条信号（在场终结）产出终结");
        let second = book.on_lifecycle_event(&broken(id, Side::Long));
        assert!(second.is_empty(), "第二条信号（容读法放行的教义死亡）必须 no-op，不重复产出");
    }

    /// 同一 Broken 事件被重复喂入（同型防御性重放）也必须幂等。
    #[test]
    fn same_event_fed_twice_is_idempotent() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        let ev = broken(id, Side::Long);
        let first = book.on_lifecycle_event(&ev);
        assert_eq!(first.len(), 1);
        let second = book.on_lifecycle_event(&ev);
        assert!(second.is_empty());
    }

    // ── 身份匹配（D 裁定） ────────────────────────────────────────────────

    /// 挂起按中枢身份匹配，不按当前在场：链已推进（新中枢在场）后，终结事件仍须命中它所指的
    /// 旧挂起身份，不会误终结新在场的那一个。
    #[test]
    fn suspension_matches_by_center_identity_not_current_alive() {
        let old_id = cid(5, 100, 200);
        let new_id = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(old_id), VoiceSide::Short, old_id.zg, 10).unwrap());
        // 链推进：新中枢在场，也开一笔挂起。
        book.on_trigger(CenterOscillationTrigger::new(0, Some(new_id), VoiceSide::Short, new_id.zg, 100).unwrap());
        assert_eq!(book.suspended_count(), 2);
        // 终结事件声明身份 = old_id（即便当前在场早已是 new_id）。
        let outcomes = book.on_lifecycle_event(&broken(old_id, Side::Short));
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].center, old_id, "命中的是载体所指的旧身份");
        assert!(!book.is_suspended(old_id));
        assert!(book.is_suspended(new_id), "新在场身份不受影响（连坐=违拒杀优先于错杀）");
    }

    // ── 接住首次教义死亡（E 裁定） ────────────────────────────────────────

    /// 即便某实例从未收到过在场终结（Superseded）记录，一旦收到教义死亡（Broken）也要正确
    /// 终结——不依赖「先 superseded 才能终结」的隐含前提。
    #[test]
    fn first_doctrinal_death_without_prior_supersede_terminates_correctly() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        // 无任何 superseded 事件历史，直接喂教义死亡。
        let outcomes = book.on_lifecycle_event(&broken(id, Side::Short));
        assert_eq!(outcomes.len(), 1);
        assert!(!book.is_suspended(id));
    }

    // ── 链重基（F 裁定，issue #292 续修：禁悬空） ────────────────────────────

    fn center_of(id: CenterId) -> Center {
        Center {
            zd: id.zd,
            zg: id.zg,
            dd: id.zd - 2,
            gg: id.zg + 2,
            start_index: id.start_index,
            end_index: id.start_index + 50,
        }
    }

    /// 状态①跟随迁移：重基后新链仍含该挂起身份（哪怕不再是链尾/容读格，只要任意下标命中）
    /// ⟹ 挂起状态原样保留，不产任何终结。
    #[test]
    fn rebase_migrates_suspension_when_identity_still_on_new_chain() {
        let id = cid(5, 100, 200);
        let other = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        assert!(book.is_suspended(id));
        let new_chain = vec![center_of(id), center_of(other)];
        let outcomes = book.on_chain_rebase(&new_chain);
        assert!(outcomes.is_empty(), "身份仍在新链上⟹跟随迁移，不产终结");
        assert!(book.is_suspended(id), "迁移=挂起状态原样保留");
    }

    /// 状态②终结：重基后新链不再含该挂起身份 ⟹ 与 Superseded 同形态终结，不回补。
    #[test]
    fn rebase_terminates_suspension_when_identity_vanishes_from_new_chain() {
        let id = cid(5, 100, 200);
        let survivor = cid(700, 300, 400);
        let mut book = CenterOscillationBook::new(0);
        book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        book.on_trigger(CenterOscillationTrigger::new(0, Some(survivor), VoiceSide::Short, survivor.zg, 20).unwrap());
        assert_eq!(book.suspended_count(), 2);
        let new_chain = vec![center_of(survivor)];
        let outcomes = book.on_chain_rebase(&new_chain);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].center, id);
        assert_eq!(outcomes[0].source, SuspensionTerminationSource::RebaseVanished);
        assert_eq!(outcomes[0].cover_action, None, "重基悬空终结不回补，承诺作废");
        assert!(!book.is_suspended(id), "终结=挂起清空");
        assert!(book.is_suspended(survivor), "存活身份不受连坐");
    }

    /// 状态③禁悬空机检：混合场景（多身份，部分迁移/部分终结）——处理后任何仍挂起的身份都
    /// 必须在新链上，逐一核对不留第三态；同时验证终结产出恰好覆盖消失的那些身份，不多不少。
    #[test]
    fn rebase_reconciliation_leaves_no_dangling_identity() {
        let stays = cid(5, 100, 200);
        let vanishes_a = cid(50, 150, 250);
        let vanishes_b = cid(80, 160, 260);
        let new_arrival = cid(900, 500, 600);
        let mut book = CenterOscillationBook::new(0);
        for (id, seed) in [(stays, 10usize), (vanishes_a, 20), (vanishes_b, 30)] {
            book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, seed).unwrap());
        }
        assert_eq!(book.suspended_count(), 3);
        let new_chain = vec![center_of(stays), center_of(new_arrival)];
        let outcomes = book.on_chain_rebase(&new_chain);
        let terminated: std::collections::BTreeSet<CenterId> =
            outcomes.iter().map(|o| o.center).collect();
        assert_eq!(
            terminated,
            [vanishes_a, vanishes_b].into_iter().collect(),
            "终结产出恰好=消失的身份集合，不多不少"
        );
        for o in &outcomes {
            assert_eq!(o.source, SuspensionTerminationSource::RebaseVanished);
            assert_eq!(o.cover_action, None);
        }
        let chain_ids: std::collections::BTreeSet<CenterId> =
            new_chain.iter().map(CenterId::of).collect();
        assert!(book.is_suspended(stays));
        assert!(!book.is_suspended(vanishes_a));
        assert!(!book.is_suspended(vanishes_b));
        assert_eq!(book.suspended_count(), 1, "只剩迁移的那一个");
        assert!(chain_ids.contains(&stays), "迁移身份必须在新链上（否则是第三态：悬空未归因）");
    }

    /// 空挂起表上的重基是 no-op（不 panic，不产任何终结）——防御性边界。
    #[test]
    fn rebase_on_empty_suspension_book_is_noop() {
        let mut book = CenterOscillationBook::new(0);
        let new_chain = vec![center_of(cid(5, 100, 200))];
        let outcomes = book.on_chain_rebase(&new_chain);
        assert!(outcomes.is_empty());
        assert_eq!(book.suspended_count(), 0);
    }

    // ── 无门（机械断言） ──────────────────────────────────────────────────

    /// 无互斥门：两个级别的账各自独立，互不读对方状态——本级触发/终结只影响本级挂起表。
    /// （编译期证据见 `on_trigger`/`on_lifecycle_event` 签名不接受任何跨级/次级别账户参数；
    /// 本测试补充运行期证据：级别 0 的全部操作对级别 1 的挂起表零影响。）
    #[test]
    fn no_mutex_gate_between_levels_each_book_is_independent() {
        let id = cid(5, 100, 200);
        let mut book0 = CenterOscillationBook::new(0);
        let mut book1 = CenterOscillationBook::new(1);
        book0.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap());
        book0.on_lifecycle_event(&broken(id, Side::Long));
        assert_eq!(book1.suspended_count(), 0, "本级动作对另一级别挂起表零影响");
    }

    #[test]
    #[should_panic(expected = "触发级别必须匹配本机级别")]
    fn trigger_level_mismatch_is_a_wiring_error_not_silent() {
        let id = cid(5, 100, 200);
        let mut book = CenterOscillationBook::new(1);
        let t = CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, 10).unwrap();
        book.on_trigger(t);
    }

    // ── #292 接线点二：PanDivTrigger → CenterOscillationTrigger 转换 ─────────

    use super::super::oscillation::{ConsolidationDivergenceEvidence, OscillationCenterRef, OscillationEvidenceRef};

    fn pan_div(level: u32, signal_side: VoiceSide, source_index: usize) -> PanDivTrigger {
        PanDivTrigger::from_gated_pan_div(
            level,
            signal_side,
            OscillationCenterRef::new(10, 30), // 坐标投影：与 CenterId 无关（A 裁定不复用它）
            ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(source_index, 0)),
        )
        .unwrap()
    }

    /// 五要素映射：级别/信号方向/盘背证据(source_index) 原样传导；中枢身份取调用方传入的
    /// `alive`（不取 `pan_div.center()` 坐标投影——A 裁定单一在场判据）。
    #[test]
    fn pan_div_conversion_maps_five_elements() {
        let alive = cid(5, 100, 200);
        let pd = pan_div(3, VoiceSide::Short, 777);
        let t = CenterOscillationTrigger::from_pan_div_trigger(pd, Some(alive)).unwrap();
        assert_eq!(t.level(), 3, "级别原样传导");
        assert_eq!(t.signal_side(), VoiceSide::Short, "信号方向原样传导");
        assert_eq!(t.center(), alive, "中枢身份=调用方传入的 alive，非 pan_div 坐标投影");
        assert_eq!(t.source_index(), 777, "盘背证据降格为 source_index 原样传导");
    }

    /// 边界侧核对：转换产出的 boundary_side 与来源 `PanDivTrigger.boundary_side()` 必然相等
    /// （两处同一映射：Long→Below / Short→Above）。
    #[test]
    fn pan_div_conversion_boundary_side_matches_source() {
        let alive = cid(5, 100, 200);
        let buy = pan_div(1, VoiceSide::Long, 10);
        let buy_source_boundary = buy.boundary_side();
        let mapped_buy = CenterOscillationTrigger::from_pan_div_trigger(buy, Some(alive)).unwrap();
        assert_eq!(mapped_buy.boundary_side(), buy_source_boundary);
        assert_eq!(mapped_buy.boundary_side(), BoundarySide::Below);

        let sell = pan_div(1, VoiceSide::Short, 11);
        let sell_source_boundary = sell.boundary_side();
        let mapped_sell = CenterOscillationTrigger::from_pan_div_trigger(sell, Some(alive)).unwrap();
        assert_eq!(mapped_sell.boundary_side(), sell_source_boundary);
        assert_eq!(mapped_sell.boundary_side(), BoundarySide::Above);
    }

    /// A 裁定：本级无在场中枢（`alive=None`）⟹ 转换拒绝，与 `new` 同一失败面，不新增静默兜底。
    #[test]
    fn pan_div_conversion_rejects_when_no_alive_center() {
        let pd = pan_div(2, VoiceSide::Long, 20);
        assert_eq!(
            CenterOscillationTrigger::from_pan_div_trigger(pd, None),
            Err(TriggerError::CenterNotAlive)
        );
    }

    // ── H1 确定性回归：BTreeMap 迭代序 ────────────────────────────────────

    /// Reset「清空整场」的多身份终结产出必须按 `CenterId` 派生序确定性排列（BTreeMap 键序），
    /// 不依赖 HashMap 默认哈希（跨进程/跨版本不确定）。三身份刻意按乱序插入，产出必须升序。
    #[test]
    fn reset_multi_suspension_outcomes_are_deterministically_ordered_by_center_id() {
        let mid = cid(50, 100, 200);
        let low = cid(5, 10, 20);
        let high = cid(900, 500, 600);
        let mut book = CenterOscillationBook::new(0);
        // 刻意乱序插入（mid → high → low），核对输出与插入序无关，只与 CenterId 派生序有关。
        for (id, seed) in [(mid, 10usize), (high, 20), (low, 30)] {
            book.on_trigger(CenterOscillationTrigger::new(0, Some(id), VoiceSide::Short, id.zg, seed).unwrap());
        }
        let outcomes = book.on_lifecycle_event(&reset_with(None));
        let ids: Vec<CenterId> = outcomes.iter().map(|o| o.center).collect();
        assert_eq!(ids, vec![low, mid, high], "产出必须按 CenterId 升序（BTreeMap 派生序），非插入序");
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "确定性排序自证：与显式排序结果一致");
    }
}
