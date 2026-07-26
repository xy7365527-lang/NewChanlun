//! 中枢生命周期事件机（#291 / SPEC #274 T1，ADR 0001 修正案一·补充二「中枢=事件」裁定）。
//!
//! ## 教义口径（ADR 0001 修正案一·补充二，2026-07-26 用户逐题终审）
//!
//! - **出生** = 前三个次级别段重叠完成（第三段重叠完成 ⟹ 中枢出生事件）。判据复用
//!   [`super::center::center_from_segments`]（L0 完整判据：方向交替 ∧ 全三段核心非空，口径 B，
//!   637号）/ [`super::center::center_from_window`]（上级几何判据——上级单元无内在缠论方向，
//!   见 center.rs 诚实有效域声明），与塔 compose 同一对构造算子（禁第二套中枢判据）。
//! - **破坏（死亡）** = 三类买卖点（本级别确认的 buy3/sell3 点 ⟹ 在场中枢死亡）。只消费
//!   **已确认**的点（修6：候选→区间套确认才出生，定账只消费已确认的点）。
//! - **一类点同死** = 本级一类点（buy1/sell1）⟹ **段序列与在场中枢同死**，新走势类型的中枢
//!   从全新段计数（禁横跨走势类型生死边界拼中枢）。
//!
//! ## 中枢身份校验（#329 H1，2026-07-26）
//!
//! 破坏/重置**必须指名道姓杀哪个中枢**：[`CenterEventMachine::push_point`] 收触发点自带的载体
//! 身份 [`CenterId`]（`(si, zd, zg)`，由 [`super::bsp::BspPoint::center`] 的 `OwnerRef::Center`
//! 读出——一类 = 被破的最后中枢、三类 = 所离开回抽的中枢），与在场中枢身份比对；不符 ⟹
//! [`CenterMisKill`] 显式失败且**状态一动不动**（拒杀优先于错杀，理由见 [`CenterMisKill`]）。
//!
//! **为什么非加不可（wf8/BTC 产物级坐实）**：#329 步骤一带探针临时构建在 wf8 全窗测得——
//! 751 次实际杀中枢里仅 **83 次**（11%）载体身份与在场中枢一致，**668 次**（89%）不一致，
//! 其中 **300 次**触发点载体与在场中枢的源区间 `[si,ei]` **完全不相交**（铁证不同实例）；
//! ℓ≥1 的 41 次杀**无一例**身份一致。即：校验前的实装**已经在大规模杀错中枢**。
//!
//! ## 边界（票面 #291 范围）
//!
//! - **只产出事件，不产出动作**：本机是结构地基（狭义短差减补动作 = #292），事件不触发任何
//!   交易行为；默认零行为变化（事件产出经 env-gated 只读旁路外化，验收锚 = 既有轨迹逐位不变）。
//! - 中枢**延伸/升级不是本机事件**（塔 `tower_events` 的 extend/level_upgrade 域）：本机在场中枢
//!   保持出生时刻的 ZD/ZG 快照，直至破坏/同死。与塔口径差异在 wf8 对账中如实列出，不强行调和。
//! - 一类+三类 bit 同点：一类优先（同死吞没破坏——1B/3B 前提冲突互斥，bsp.rs `no_exclusive_
//!   trichotomy` 结构下理论不同位；防御性规定，照实标注）。
//!
//! ## 段计数口径
//!
//! - 段序列 = 自上次一类点同死（或机器构造）起喂入的段；`born_seg_ordinal` = 构成中枢的**第三段**
//!   （完成重叠那段）在当前段序列中的 1-based 序号。
//! - **三类破坏（R0′，#331/#330 用户裁定二）**：破坏后**已消费段不再参与新中枢计数**——旧中枢
//!   出生窗口及其之前的段（`segs[..born_seg_ordinal]`）随该中枢一并消费丢弃，新中枢自其后的段
//!   （`segs[born_seg_ordinal..]`）滑窗出生；`born_seg_ordinal` 由此变为相对「已消费段丢弃后的
//!   当前段序列」计数（旧口径下的读数一律作废）。塔消费语义的机器侧对应物 =
//!   [`super::recursive_tower`] `:793` 游标 `i=j*`（塔窗口扫描游标越过已吸收段后不倒回）。
//!   ⚠️ **旧口径（已作废，#331 修正）**：本行以下原文「三类破坏不清段序列（趋势延续，新中枢自
//!   后续段滑动窗口出生）」——2026-07-26 之前的对账读数（含「破坏后段序列不清零」「born_seg_ordinal
//!   跨破坏连续计数不清零」等假设）据此全部作废，须按新口径重跑。
//!   ⚠️ **有效域（#331 wf8 实测，如实登记）**：本条只改**计数口径**，**不改窗口选择**——下方滑窗
//!   恒测「尾 3 段」，尾窗由段序列**末端**定位，丢弃头部不改变任何被测窗口。因此 `born_seg_ordinal`
//!   的分母变了，出生时点/出生身份不变。wf8 全窗（264,960 bar）实测：`broken` 事件仅 **1 次**、
//!   `reset` **0 次** ⟹ 本条在该窗的最大爆炸半径 = 1 个事件，L0 时间线（609 born / 1490 broken /
//!   1502 born si=624）与改前逐条相同。若要让「游标推进」具备窗口调度上的操作内容（破坏后对存活
//!   后缀做完整前向重扫，而非只测尾窗——报告 §5 R0′ 标题所述），须另行教义裁决，本票未做。
//! - 一类同死**清零**段序列（不变，R0′ 未改此面）。
//! - 滑窗：在场中枢缺席时，每新段测试**尾 3 段**窗口（与塔窗口扫描同义——前 3 段不成立则
//!   第 2/3/4 段仍可成交）；在场中枢存在时不测出生（一中枢一场）。

use super::super::types::{BspBits, Center, Side, Tick};
use super::center::{center_from_segments, center_from_window, UnitRange};

/// 中枢**实例身份**（#329 H1）：`(出生坐标 si, 核心 ZD, 核心 ZG)` 三元组。
///
/// ## 为什么是这三元组（票面「择与既有结构最简一致者」）
///
/// 不引入新的实例 id 侧车——身份直接从既有 [`Center`] 读出：
/// - `start_index`：中枢首单元在 L0 原始 K 序的起点（[`UnitRange`] 文档：所有级别的单元坐标
///   统一在 L0 K 序）⟹ 跨级别可比，且在本机「一中枢一场」的时间线上唯一标定出生实例。
/// - `(zd, zg)`：核心区间。**延伸不改核心**（`recursive_tower.rs:265`）⟹ 同一实例被塔延伸后
///   核心仍相等，身份对「延伸」稳定；而不同实例的核心几乎必异。
///
/// 外缘 `dd/gg` 与 `end_index` **不进**身份：二者随延伸/窗口推进而变，进身份会把「同一中枢被
/// 延伸」误判成「不同中枢」（#291 对账中 si+core 同而 ei/dd/gg 异的 33 例即此面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CenterId {
    /// 中枢首单元在 L0 原始 K 序的起点。
    pub start_index: usize,
    /// 核心区间下沿 ZD。
    pub zd: Tick,
    /// 核心区间上沿 ZG。
    pub zg: Tick,
}

impl CenterId {
    /// 从既有 [`Center`] 读出实例身份（零新增字段）。
    pub fn of(c: &Center) -> Self {
        Self { start_index: c.start_index, zd: c.zd, zg: c.zg }
    }
}

/// 死亡触发类（#329：误杀证据里区分「三类破坏」与「一类同死」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillTrigger {
    /// 本级一类点（buy1/sell1）⟹ 同死。
    FirstClass,
    /// 本级三类点（buy3/sell3）⟹ 破坏。
    ThirdClass,
}

/// **误杀拒绝证据**（#329 H1）：触发点声明要杀的中枢身份 ≠ 在场中枢身份 ⟹ 显式失败。
///
/// 语义（票面「误杀显式失败」）：本机**不猜**。触发点自带载体（[`super::bsp::BspPoint::center`]
/// ——一类 = 被破的最后中枢，三类 = 所离开回抽的中枢）；载体身份与在场中枢身份不符时，本机
/// **不动任何状态**（不杀中枢、不清段序列）并返回本证据，由调用方裁决（诊断/告警/再同步）。
///
/// 为什么拒杀优先于错杀：#292 把破坏/重置事件翻译成狭义短差减补动作——**错杀 = 在错误语境
/// 开火**（真金白银的错误动作），漏动作只是不动。故不确定时一律拒。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterMisKill {
    /// 事件机级别。
    pub level: u32,
    /// 触发类（一类同死 / 三类破坏）。
    pub trigger: KillTrigger,
    /// 触发点 source_index（确认坐标）。
    pub trigger_source_index: usize,
    /// 触发点方向。
    pub trigger_side: Side,
    /// 在场中枢身份（本机认为「在场」的那个）。
    pub alive: CenterId,
    /// 触发点声明要杀的中枢身份；`None` = 该点无中枢载体（二类锚 / 载体缺席）。
    pub target: Option<CenterId>,
}

/// 中枢生命周期事件（#291 三类：born/broken/reset）。事件含级别、中枢区间（ZD/ZG）、出生段号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterLifecycleEvent {
    /// 出生：第三段重叠完成（`build` 判据接受尾 3 段窗口）。
    Born {
        level: u32,
        /// 出生时刻中枢（核心 [zd,zg] + 外缘 [dd,gg] + 源坐标 [start_index,end_index]）。
        center: Center,
        /// 出生段号：完成重叠的第三段在当前段序列（一类点同死后重新计数）中的 1-based 序号。
        born_seg_ordinal: usize,
    },
    /// 破坏：本级别确认的三类买卖点 ⟹ 在场中枢死亡（R0′，#331/#330 裁定二：**已消费段
    /// 丢弃**——`segs[..born_seg_ordinal]` 随死中枢消费，新中枢自 `segs[born_seg_ordinal..]`
    /// 滑窗出生；⚠️ 旧口径「段序列不清零」已作废，见模块头「段计数口径」）。
    Broken {
        level: u32,
        /// 死亡中枢（出生时刻快照）。
        center: Center,
        /// 该中枢出生时的段号（溯源）。
        born_seg_ordinal: usize,
        /// 触发破坏的三类点 source_index（确认坐标）。
        breaker_source_index: usize,
        /// 三类点方向（buy3 ⟹ Long / sell3 ⟹ Short，types::Side 买卖语境）。
        breaker_side: Side,
    },
    /// 一类点同死：本级一类点 ⟹ 段序列清零 + 在场中枢（若有）同死。新中枢从全新段计数。
    Reset {
        level: u32,
        /// 同死的在场中枢（出生快照）；无在场中枢 ⟹ None（段序列仍清零）。
        died_center: Option<Center>,
        /// 同死中枢的出生段号（无在场中枢 ⟹ None）。
        died_born_seg_ordinal: Option<usize>,
        /// 被清零的段序列长度（计数清零的证据）。
        cleared_segments: usize,
        /// 触发同死的一类点 source_index（确认坐标）。
        trigger_source_index: usize,
        /// 一类点方向（buy1 ⟹ Long / sell1 ⟹ Short）。
        trigger_side: Side,
    },
}

/// 中枢生命周期事件机（每级别一台；纯结构，L0 判据，不读经验参数）。
///
/// 构造算子 `build` 按级别锁定：L0 = [`center_from_segments`]（完整判据，方向交替）；
/// ℓ≥1 = [`center_from_window`]（几何判据——上级单元无 §6.1 方向维度，center.rs 有效域）。
pub struct CenterEventMachine {
    level: u32,
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    /// 段序列（自上次一类点同死/工程 resync 起）。
    segs: Vec<UnitRange>,
    /// 在场中枢：(出生快照, 出生段号)。Some = 已出生未死。
    alive: Option<(Center, usize)>,
    born_total: usize,
    broken_total: usize,
    reset_total: usize,
    /// ★#329：误杀拒绝累计（身份不符 ⟹ 拒杀，状态不动）。
    miskill_total: usize,
    /// ★#331 R2：**同一在场中枢实例上累计被拒的死亡请求数**（拒杀逃逸阀读数，根因报告 §5 R2
    /// 「累积拒杀 N 次 ⟹ 该机 `resync()`」）。`Err` 路径 +1；归零点 = 在场实例换人的四处
    /// （出生 / 破坏放行 / 一类同死 / `resync()`）。**无关点（二类点等）的 `Ok(None)` 不归零**
    /// ——在场实例没换，此前拒杀历史依旧适用。纯读数，不改任何生死语义。
    alive_miskills: usize,
}

impl CenterLifecycleEvent {
    /// ★#329：本事件**杀掉的中枢身份**（票面「破坏/重置事件携带中枢身份」）。
    ///
    /// `Broken` ⟹ 被破坏中枢；`Reset` ⟹ 同死的在场中枢（无在场中枢 ⟹ `None`）；`Born` ⟹ `None`
    /// （出生不杀）。下游（#292 减补动作）直读身份，不再从 `Center` 逐字段反推。
    pub fn killed_center_id(&self) -> Option<CenterId> {
        match self {
            CenterLifecycleEvent::Born { .. } => None,
            CenterLifecycleEvent::Broken { center, .. } => Some(CenterId::of(center)),
            CenterLifecycleEvent::Reset { died_center, .. } => died_center.as_ref().map(CenterId::of),
        }
    }
}

impl CenterEventMachine {
    /// 构造级别 `level` 的事件机（L0 完整判据 / ℓ≥1 几何判据，与塔 compose_level 同口径）。
    pub fn new(level: u32) -> Self {
        Self {
            level,
            build: if level == 0 {
                center_from_segments
            } else {
                center_from_window
            },
            segs: Vec::new(),
            alive: None,
            born_total: 0,
            broken_total: 0,
            reset_total: 0,
            miskill_total: 0,
            alive_miskills: 0,
        }
    }

    /// 喂入一段（段序列尾部追加）。在场中枢缺席且段数 ≥3 ⟹ 测尾 3 段窗口，成交 ⟹ `Born`。
    pub fn push_segment(&mut self, seg: UnitRange) -> Option<CenterLifecycleEvent> {
        self.segs.push(seg);
        // 在场中枢存在 ⟹ 不测出生（一中枢一场；延伸非本机事件，见模块头边界）。
        if self.alive.is_some() {
            return None;
        }
        let n = self.segs.len();
        if n < 3 {
            return None;
        }
        // 尾 3 段窗口（滑窗：前 3 段不成立则第 2/3/4…段仍可成交，与塔窗口扫描同义）。
        let (a, b, c) = (&self.segs[n - 3], &self.segs[n - 2], &self.segs[n - 1]);
        let center = (self.build)(a, b, c)?;
        let born_seg_ordinal = n; // 1-based 出生段号 = 完成重叠的第三段在当前段序列中的序号。
        self.alive = Some((center, born_seg_ordinal));
        self.born_total += 1;
        // ★#331 R2：新在场实例上场 ⟹ 每实例拒杀读数归零（防御性——拒杀只在有在场中枢时发生，
        // 而在场中枢下场时已归零，故此处理论恒为 0；显式写出以免归零点漏一处）。
        self.alive_miskills = 0;
        Some(CenterLifecycleEvent::Born { level: self.level, center, born_seg_ordinal })
    }

    /// 喂入一个**已确认**买卖点（修6：只消费已确认的点）。
    ///
    /// 一类 bit（buy1/sell1）⟹ `Reset`（段序列清零 + 在场中枢同死；一类优先于三类）；
    /// 否则三类 bit（buy3/sell3）且在场中枢存在 ⟹ `Broken`。二类点不产事件（不在三类事件教义内）。
    ///
    /// ★#329：`target` = 该点自带载体的中枢身份（[`super::bsp::BspPoint::center`] 的
    /// `OwnerRef::Center` ⟹ [`CenterId::of`]；非中枢载体 ⟹ `None`）。有在场中枢时 `target`
    /// 必须与之相符，否则 [`Err(CenterMisKill)`](CenterMisKill) 且状态不动（校验体
    /// `verify_kill_target`）。
    pub fn push_point(
        &mut self,
        bits: BspBits,
        source_index: usize,
        target: Option<CenterId>,
    ) -> Result<Option<CenterLifecycleEvent>, CenterMisKill> {
        // 一类优先（1B/3B 前提冲突互斥，理论不同位；防御性规定，模块头已标注）。
        let first = if bits.buy1 {
            Some(Side::Long)
        } else if bits.sell1 {
            Some(Side::Short)
        } else {
            None
        };
        if let Some(trigger_side) = first {
            // ★#329 身份校验：有在场中枢时，触发点声明的载体身份必须 == 在场中枢身份。
            self.verify_kill_target(KillTrigger::FirstClass, source_index, trigger_side, target)?;
            let cleared_segments = self.segs.len();
            let (died_center, died_born_seg_ordinal) = match self.alive.take() {
                Some((c, ord)) => (Some(c), Some(ord)),
                None => (None, None),
            };
            self.segs.clear(); // 段序列与在场中枢同死（新中枢从全新段计数）。
            self.reset_total += 1;
            self.alive_miskills = 0; // ★#331 R2：一类同死 ⟹ 在场实例下场，每实例拒杀读数归零。
            return Ok(Some(CenterLifecycleEvent::Reset {
                level: self.level,
                died_center,
                died_born_seg_ordinal,
                cleared_segments,
                trigger_source_index: source_index,
                trigger_side,
            }));
        }
        let third = if bits.buy3 {
            Some(Side::Long)
        } else if bits.sell3 {
            Some(Side::Short)
        } else {
            None
        };
        if let Some(breaker_side) = third {
            // ★#329 身份校验（先于取走在场中枢——拒杀时状态一动不动）。
            self.verify_kill_target(KillTrigger::ThirdClass, source_index, breaker_side, target)?;
            // 无在场中枢 ⟹ 诚实 no-op（不杀不存在的中枢）；段序列不动（无死亡，无消费可谈）。
            let Some((center, born_seg_ordinal)) = self.alive.take() else {
                // 无在场实例 ⟹ 无拒杀可累（`alive_miskills` 此时恒为 0），读数不动。
                return Ok(None);
            };
            // ★R0′（#331/#330 用户裁定二）：塔消费语义——已死中枢出生窗口及其之前的段
            // （`segs[..born_seg_ordinal]`）随该中枢一并消费丢弃，不再参与新中枢计数；只保留
            // 出生窗口之后（在场期新喂入）的段供后续滑窗（旧口径「不清段序列」已作废，见模块头）。
            self.segs.drain(..born_seg_ordinal);
            self.broken_total += 1;
            self.alive_miskills = 0; // ★#331 R2：破坏放行 ⟹ 在场实例下场，每实例拒杀读数归零。
            return Ok(Some(CenterLifecycleEvent::Broken {
                level: self.level,
                center,
                born_seg_ordinal,
                breaker_source_index: source_index,
                breaker_side,
            }));
        }
        // 无一/三类 bit（二类点等无关点）⟹ 在场实例没换，`alive_miskills` **不归零**
        // （★#331 R2：若在此归零，阀门会被无关点持续打断——wf8 实测 888 次拒杀只放行 4 次）。
        Ok(None)
    }

    /// ★#329 H1 中枢身份校验：死亡请求必须指名道姓杀哪个中枢。
    ///
    /// - **无在场中枢** ⟹ 校验不介入（`Ok(())`）：没有可杀对象，谈不上误杀。三类点走诚实
    ///   no-op，一类点仍清段序列（同死语义覆盖段序列本身，#291 口径不动）。
    /// - **有在场中枢** ⟹ `target` 必须恰为在场中枢的 [`CenterId`]。不符（含 `target=None`
    ///   ——点无中枢载体、无从校验）⟹ [`CenterMisKill`]，计数 +1，**不改任何状态**。
    ///
    /// 不猜的理由见 [`CenterMisKill`]：#292 把本机事件翻译成减补动作，错杀 = 在错误语境开火。
    fn verify_kill_target(
        &mut self,
        trigger: KillTrigger,
        source_index: usize,
        trigger_side: Side,
        target: Option<CenterId>,
    ) -> Result<(), CenterMisKill> {
        let Some((alive_center, _)) = self.alive else {
            return Ok(());
        };
        let alive = CenterId::of(&alive_center);
        if target == Some(alive) {
            return Ok(());
        }
        self.miskill_total += 1;
        self.alive_miskills += 1; // ★#331 R2：本在场实例上的累计拒杀（归零点见字段文档）。
        Err(CenterMisKill {
            level: self.level,
            trigger,
            trigger_source_index: source_index,
            trigger_side,
            alive,
            target,
        })
    }

    /// 工程性再同步（旁路喂数层专用：塔 cascade 失效/水线回缩致已喂前缀不可信时调用）。
    /// 清空段序列与在场中枢，**不产事件**（非教义生死，照实区别于 Reset）。
    /// ★#331 R2：同批归零每实例拒杀读数（resync 后在场身份已重置，旧拒杀历史不再适用）。
    pub fn resync(&mut self) {
        self.segs.clear();
        self.alive = None;
        self.alive_miskills = 0;
    }

    /// 在场中枢（出生快照 + 出生段号）；None = 当前无在场中枢。
    pub fn alive_center(&self) -> Option<(Center, usize)> {
        self.alive
    }

    /// 当前段序列长度（自上次一类点同死/resync 起计数）。
    pub fn segments_since_reset(&self) -> usize {
        self.segs.len()
    }

    /// (born, broken, reset) 累计事件计数（wf8 自证读数）。
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.born_total, self.broken_total, self.reset_total)
    }

    /// ★#329：误杀拒绝累计（身份不符被拒的死亡请求数；不含「无在场中枢」的诚实 no-op）。
    pub fn mis_kills(&self) -> usize {
        self.miskill_total
    }

    /// ★#331 R2：**同一在场中枢实例上累计被拒的死亡请求数**（`Err` 路径 +1；归零点 = 在场实例
    /// 换人的四处：出生 / 破坏放行 / 一类同死 / `resync()`；无关点的 `Ok(None)` 不归零）。
    /// 逃逸阀读数——旁路层（opsem_dump.rs）据此判断是否触发工程 resync，本机自身不因此值改变
    /// 任何生死语义（纯读数，见 [`CenterMisKill`] 拒杀优先于错杀）。
    pub fn alive_mis_kills(&self) -> usize {
        self.alive_miskills
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::{Direction, Tick};

    /// 方向交替单元（与 center.rs 测试同款构造器）。
    fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }

    fn up() -> Direction { Direction::Up }
    fn down() -> Direction { Direction::Down }

    fn bits_1b() -> BspBits { BspBits { buy1: true, ..Default::default() } }
    fn bits_1s() -> BspBits { BspBits { sell1: true, ..Default::default() } }
    fn bits_3b() -> BspBits { BspBits { buy3: true, ..Default::default() } }
    fn bits_3s() -> BspBits { BspBits { sell3: true, ..Default::default() } }
    fn bits_2b() -> BspBits { BspBits { buy2: true, ..Default::default() } }

    // ──────────────────────────────────────────────────────────────────────
    //  出生（3 段重叠 → born 事件，含级别/ZD/ZG/出生段号）
    // ──────────────────────────────────────────────────────────────────────

    /// ★出生正例（口径 B bit-exact，center.rs complete_center_confirmed_bit_exact 同构）：
    /// 上-下-上 a=[10,20] b=[12,20] c=[12,22]，全三段核心 [12,20] 非空 ⟹ 第三段完成时 Born。
    #[test]
    fn born_on_third_segment_overlap() {
        let mut m = CenterEventMachine::new(0);
        assert_eq!(m.push_segment(unit(0, 4, up(), 10, 20)), None, "第 1 段不出生");
        assert_eq!(m.push_segment(unit(4, 8, down(), 12, 20)), None, "第 2 段不出生");
        let ev = m.push_segment(unit(8, 12, up(), 12, 22));
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Born { level: 0, center, born_seg_ordinal: 3 }),
            "第三段重叠完成 ⟹ born 事件（含级别/ZD/ZG/出生段号）"
        );
        assert_eq!(m.alive_center(), Some((center, 3)), "出生后在场");
        assert_eq!(m.counts(), (1, 0, 0));
        // 在场后第 4 段不再产 born（一中枢一场，延伸非本机事件）。
        assert_eq!(m.push_segment(unit(12, 16, down(), 13, 21)), None, "在场中枢存在时不测出生");
        assert_eq!(m.counts(), (1, 0, 0));
    }

    /// ★滑窗出生：前 3 段核心空不出生，第 2/3/4…段窗口滑动后成交 ⟹ born（出生段号 = 完成段序号）。
    ///
    /// ★#321 裁定后重算（本用例固化的 (c,d,e) 分支曾是旧弱口径）：原版本在 (c,d,e) 处
    /// `zd=max(5,8,9)=9 ≤ zg=min(9,12,13)=9`（单点核心 `[9,9]`）判**成立**（born_seg_ordinal=5）。
    /// #321 裁定（2026-07-26 用户裁决，与 #290 裁定 B / Python 严格口径三方对齐）后单点核心
    /// `zd==zg` **不成立**——旧读数（born 于第 5 段）已作废：(c,d,e) 现判不出生，滑窗需再推进一段
    /// 到 (d,e,f) 才遇到真正非退化核心（`zd=9<zg=12`）成交，born_seg_ordinal 由 5 变为 6。
    #[test]
    fn born_sliding_window_after_empty_core() {
        let mut m = CenterEventMachine::new(0);
        // (a,b,c)：zd=max(0,10,5)=10 > zg=min(4,14,9)=4 ⟹ 核心空，不出生。
        assert_eq!(m.push_segment(unit(0, 4, up(), 0, 4)), None);
        assert_eq!(m.push_segment(unit(4, 8, down(), 10, 14)), None);
        assert_eq!(m.push_segment(unit(8, 12, up(), 5, 9)), None, "前 3 段核心空 ⟹ 无 born");
        // (b,c,d)：zd=max(10,5,8)=10 > zg=min(14,9,12)=9 ⟹ 仍空。
        assert_eq!(m.push_segment(unit(12, 16, down(), 8, 12)), None, "滑窗 (b,c,d) 核心空 ⟹ 无 born");
        // (c,d,e)：zd=max(5,8,9)=9，zg=min(9,12,13)=9 ⟹ 单点核心 [9,9]。#321 从严：zd==zg 不成立
        // ⟹ 无 born（旧口径判成立于此，已作废——见上方用例 docstring）。
        assert_eq!(
            m.push_segment(unit(16, 20, up(), 9, 13)),
            None,
            "#321 从严：(c,d,e) 单点核心 zd==zg ⟹ 不成立，无 born（旧读数已作废）"
        );
        // (d,e,f)：zd=max(8,9,9)=9 < zg=min(12,13,20)=12 ⟹ 非退化核心 [9,12] ⟹ 成交 born，
        // 出生段号 = 第 6 段（完成段，滑窗多推进一段才遇到真正非空核心）。
        let ev = m.push_segment(unit(20, 24, down(), 9, 20));
        let center = Center { zd: 9, zg: 12, dd: 8, gg: 20, start_index: 12, end_index: 24 };
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Born { level: 0, center, born_seg_ordinal: 6 }),
            "滑窗 (d,e,f) 成交 ⟹ born，出生段号 = 第 6 段（完成段，#321 从严后需多滑一段）"
        );
        assert_eq!(m.counts(), (1, 0, 0));
    }

    /// ★无重叠不出生：方向交替但三段两两分离 ⟹ 永不出生；同向三段（无方向交替）L0 完整判据拒绝。
    #[test]
    fn no_overlap_no_born() {
        let mut m = CenterEventMachine::new(0);
        // 方向交替但两两分离：zd=max(0,10,20)=20 > zg=min(4,14,24)=4 ⟹ 核心空。
        assert_eq!(m.push_segment(unit(0, 4, up(), 0, 4)), None);
        assert_eq!(m.push_segment(unit(4, 8, down(), 10, 14)), None);
        assert_eq!(m.push_segment(unit(8, 12, up(), 20, 24)), None, "三段两两分离 ⟹ 无 born");
        assert_eq!(m.alive_center(), None);
        assert_eq!(m.counts(), (0, 0, 0));
        assert_eq!(m.segments_since_reset(), 3, "未出生段序列保留计数（供滑窗）");

        // 同向三段（无方向交替）⟹ L0 完整判据拒绝（即使核心非空）。
        let mut m2 = CenterEventMachine::new(0);
        assert_eq!(m2.push_segment(unit(0, 4, up(), 10, 20)), None);
        assert_eq!(m2.push_segment(unit(4, 8, up(), 18, 25)), None);
        assert_eq!(m2.push_segment(unit(8, 12, up(), 18, 22)), None, "无方向交替 ⟹ L0 完整判据拒绝");
        assert_eq!(m2.counts(), (0, 0, 0));

        // 上级（ℓ≥1）几何判据不查方向：同构同向三段 ⟹ 几何成交 born（判据分域见证）。
        let mut m3 = CenterEventMachine::new(1);
        assert_eq!(m3.push_segment(unit(0, 4, up(), 10, 20)), None);
        assert_eq!(m3.push_segment(unit(4, 8, up(), 18, 25)), None);
        let ev = m3.push_segment(unit(8, 12, up(), 18, 22));
        assert!(
            matches!(ev, Some(CenterLifecycleEvent::Born { level: 1, born_seg_ordinal: 3, .. })),
            "ℓ≥1 几何判据不查方向交替 ⟹ born（与 L0 判据分域，center.rs 有效域）"
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    //  破坏（三类买卖点 → broken）
    // ──────────────────────────────────────────────────────────────────────

    /// ★三类点破在场中枢：born 后确认 buy3 ⟹ Broken（含死中枢/出生段号/触发坐标/方向）；
    /// **R0′（#331/#330 用户裁定二）**：破坏后已消费段（`segs[..born_seg_ordinal]`）随死中枢
    /// 丢弃，`segments_since_reset()` 应为 0；新中枢自后续段滑窗出生，出生段号相对「已消费段
    /// 丢弃后的当前段序列」重新计数（本例 = 3，非旧口径的 6）。
    ///
    /// ⚠️ **旧读数已作废**（#331 修正）：本用例修改前断言「破坏后 `segments_since_reset()==3`
    /// （不清零）」「新中枢 `born_seg_ordinal==6`（连续计数）」——2026-07-26 之前对账/wf8 产物
    /// 里依赖这两条断言的读数一律作废，须按 R0′ 口径重跑。
    #[test]
    fn broken_by_third_class_point() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        let born = m.push_segment(unit(8, 12, up(), 12, 22));
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        assert!(matches!(born, Some(CenterLifecycleEvent::Born { .. })));

        // 三类买点确认 ⟹ 在场中枢死亡。
        let ev = m.push_point(bits_3b(), 100, Some(CenterId::of(&center)));
        assert_eq!(
            ev,
            Ok(Some(CenterLifecycleEvent::Broken {
                level: 0,
                center,
                born_seg_ordinal: 3,
                breaker_source_index: 100,
                breaker_side: Side::Long,
            })),
            "三类买点 ⟹ broken（R0′：已消费段随之丢弃）"
        );
        assert_eq!(m.alive_center(), None, "破坏后无在场中枢");
        assert_eq!(m.counts(), (1, 1, 0));
        assert_eq!(
            m.segments_since_reset(),
            0,
            "R0′：破坏后已消费段（出生窗口及其之前）随死中枢丢弃，段序列归 0（旧口径断言 3 已作废）"
        );

        // 无在场中枢时三类点不产事件（不杀不存在的中枢，诚实 no-op；无在场 ⟹ 身份无可校验对象）。
        assert_eq!(m.push_point(bits_3s(), 200, None), Ok(None), "无在场中枢 ⟹ 三类点无事件");
        assert_eq!(m.counts(), (1, 1, 0));

        // 破坏后新中枢自「已消费段丢弃后的当前段序列」滑窗出生：前 2 段不成交，第 3 段（全新
        // 计数）完成重叠。
        assert_eq!(m.push_segment(unit(12, 16, down(), 30, 40)), None, "新序列第 1 段");
        assert_eq!(m.push_segment(unit(16, 20, up(), 32, 42)), None, "新序列第 2 段");
        let ev2 = m.push_segment(unit(20, 24, down(), 31, 41));
        // (d,e,f)：方向 down/up/down 交替；zd=max(30,32,31)=32 ≤ zg=min(40,42,41)=40 ⟹ 成交。
        assert_eq!(
            ev2,
            Some(CenterLifecycleEvent::Born {
                level: 0,
                center: Center { zd: 32, zg: 40, dd: 30, gg: 42, start_index: 12, end_index: 24 },
                born_seg_ordinal: 3,
            }),
            "R0′：新中枢出生段号 = 3（相对已消费段丢弃后的当前段序列重新计数，旧口径断言 6 已作废）"
        );
        assert_eq!(m.counts(), (2, 1, 0));
    }

    /// ★R0′ 片二（#331/#330 用户裁定二）：破坏时段序列**长于**出生窗口——中枢出生于第 3 段
    /// （`born_seg_ordinal=3`），在场期又续喂到第 5 段（第 4/5 段是趋势延续段，未参与出生判据）。
    /// 破坏后应只丢弃出生窗口内的段（`segs[..3]`），保留在场期新喂入的第 4/5 段（`segs[3..5]`）；
    /// 新中枢无需等满 3 个全新段——喂入第 3 个新段（全局第 6 段）即可用「保留的 2 段 + 新 1 段」
    /// 成交，`born_seg_ordinal` 相对丢弃后的段序列计数为 3。
    #[test]
    fn broken_after_alive_period_fed_beyond_birth_window_keeps_trailing_segments() {
        let mut m = CenterEventMachine::new(0);
        // 出生窗口（第 1..3 段）：上-下-上，核心 [12,20]。
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        let born = m.push_segment(unit(8, 12, up(), 12, 22));
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        assert_eq!(born, Some(CenterLifecycleEvent::Born { level: 0, center, born_seg_ordinal: 3 }));

        // 在场期续喂第 4/5 段（一中枢一场：不测出生，但段序列照常追加）。
        m.push_segment(unit(12, 16, down(), 30, 45));
        m.push_segment(unit(16, 20, up(), 33, 48));
        assert_eq!(m.segments_since_reset(), 5, "在场期第 4/5 段已喂入（未参与出生判据）");

        // 三类破坏：出生窗口 [第1..3段] 随死中枢丢弃，保留第 4/5 段。
        let ev = m.push_point(bits_3b(), 200, Some(CenterId::of(&center)));
        assert!(matches!(ev, Ok(Some(CenterLifecycleEvent::Broken { born_seg_ordinal: 3, .. }))));
        assert_eq!(
            m.segments_since_reset(),
            2,
            "R0′：只丢弃出生窗口内的段（前 3 段），在场期新喂的第 4/5 段保留"
        );

        // 第 6 个全局段（= 保留后的第 3 段）到达 ⟹ 与保留的第 4/5 段拼出新中枢，无需等满 3 个
        // 全新段。方向 down/up/down 交替；zd=max(30,33,31)=33 ≤ zg=min(45,48,41)=41 ⟹ 成交。
        let ev2 = m.push_segment(unit(20, 24, down(), 31, 41));
        assert_eq!(
            ev2,
            Some(CenterLifecycleEvent::Born {
                level: 0,
                center: Center { zd: 33, zg: 41, dd: 30, gg: 48, start_index: 12, end_index: 24 },
                born_seg_ordinal: 3,
            }),
            "R0′：保留段 + 1 新段即可成交，born_seg_ordinal=3（相对丢弃后的段序列）"
        );
        assert_eq!(m.counts(), (2, 1, 0));
    }

    // ──────────────────────────────────────────────────────────────────────
    //  一类点同死（本级一类点 → 段序列清零 + 在场中枢死亡，新中枢不从旧段拼出）
    // ──────────────────────────────────────────────────────────────────────

    /// ★一类卖点同死：born 后确认 sell1 ⟹ Reset（段序列清零 + 在场中枢同死）；
    /// 之后只喂 2 段不出生（即使旧段若参与可成交——新中枢**不从旧段拼出**）；
    /// 喂满全新 3 段 ⟹ 新 born，出生段号重新从 3 计数。
    #[test]
    fn reset_by_first_class_point_kills_segments_and_center() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        m.push_segment(unit(8, 12, up(), 12, 22)); // born（段 3）
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        // 在场期间第 4 段（旧走势段，若与新段拼可成交的诱饵）。
        assert_eq!(m.push_segment(unit(12, 16, down(), 30, 40)), None);
        assert_eq!(m.segments_since_reset(), 4);

        // 本级一类卖点确认 ⟹ 段序列与在场中枢同死。
        let ev = m.push_point(bits_1s(), 100, Some(CenterId::of(&center)));
        assert_eq!(
            ev,
            Ok(Some(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: Some(center),
                died_born_seg_ordinal: Some(3),
                cleared_segments: 4,
                trigger_source_index: 100,
                trigger_side: Side::Short,
            })),
            "一类卖点 ⟹ reset（段序列清零 + 在场中枢同死）"
        );
        assert_eq!(m.alive_center(), None);
        assert_eq!(m.segments_since_reset(), 0, "段计数清零");
        assert_eq!(m.counts(), (1, 0, 1));

        // 新走势类型只来 2 段：与旧第 4 段 [30,40]down 若拼 (d,e,f) 本可成交
        // （zd=max(30,32,31)=32 ≤ zg=min(40,42,41)=41），但旧段已死 ⟹ 不得参与。
        assert_eq!(m.push_segment(unit(16, 20, up(), 32, 42)), None, "新序列第 1 段");
        assert_eq!(m.push_segment(unit(20, 24, down(), 31, 41)), None, "新序列第 2 段——不从旧段拼出");
        assert_eq!(m.counts(), (1, 0, 1), "2 段不出生（旧段禁拼）");
        // 全新第 3 段完成重叠 ⟹ 新 born，段号重新计数 = 3。
        let ev2 = m.push_segment(unit(24, 28, up(), 33, 43));
        assert_eq!(
            ev2,
            Some(CenterLifecycleEvent::Born {
                level: 0,
                center: Center { zd: 33, zg: 41, dd: 31, gg: 43, start_index: 16, end_index: 28 },
                born_seg_ordinal: 3,
            }),
            "全新 3 段 ⟹ 新中枢出生，段号重新计数"
        );
        assert_eq!(m.counts(), (2, 0, 1));
    }

    /// ★无在场中枢时一类点仍清零段序列（同死语义覆盖段序列本身）。
    #[test]
    fn reset_without_alive_center_still_clears_segments() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 0, 4));
        m.push_segment(unit(4, 8, down(), 10, 14));
        assert_eq!(m.segments_since_reset(), 2);
        // 无在场中枢 ⟹ 无可校验对象，target=None 仍产 Reset（清段序列，不杀任何中枢）。
        let ev = m.push_point(bits_1b(), 50, None);
        assert_eq!(
            ev,
            Ok(Some(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: None,
                died_born_seg_ordinal: None,
                cleared_segments: 2,
                trigger_source_index: 50,
                trigger_side: Side::Long,
            })),
            "无在场中枢 ⟹ Reset 仍清零段序列（died=None）"
        );
        assert_eq!(m.segments_since_reset(), 0);
        assert_eq!(m.counts(), (0, 0, 1));
        // 一类+三类同点（防御：理论互斥）：一类优先，同死吞没破坏。
        let mut m2 = CenterEventMachine::new(0);
        m2.push_segment(unit(0, 4, up(), 10, 20));
        m2.push_segment(unit(4, 8, down(), 12, 20));
        m2.push_segment(unit(8, 12, up(), 12, 22));
        let both = BspBits { buy1: true, buy3: true, ..Default::default() };
        let m2_center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        let ev2 = m2.push_point(both, 60, Some(CenterId::of(&m2_center)));
        assert!(
            matches!(ev2, Ok(Some(CenterLifecycleEvent::Reset { .. }))),
            "一类+三类同点 ⟹ 一类优先（同死吞没破坏）"
        );
        assert_eq!(m2.counts(), (1, 0, 1));
    }

    /// ★二类点不产事件（不在 born/broken/reset 三类教义事件内）。
    #[test]
    fn second_class_point_produces_no_event() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        m.push_segment(unit(8, 12, up(), 12, 22));
        // 二类点载体是 Type1Anchor（无中枢身份）⟹ target=None；不产事件 ⟹ 身份校验不触发。
        assert_eq!(m.push_point(bits_2b(), 70, None), Ok(None), "二类点不产事件");
        assert_eq!(m.counts(), (1, 0, 0));
        assert!(m.alive_center().is_some(), "二类点不动在场中枢");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★#329 H1：中枢身份校验（多候选下「杀对」与「误杀拒绝」两态）
    // ──────────────────────────────────────────────────────────────────────

    /// 建一台 L0 机器并出生 A = `{zd:12, zg:20, si:0, ei:12}`（返回机器与 A）。
    fn machine_with_alive_a() -> (CenterEventMachine, Center) {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        m.push_segment(unit(8, 12, up(), 12, 22));
        let a = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        assert_eq!(m.alive_center(), Some((a, 3)), "前置：A 在场");
        (m, a)
    }

    /// ★多候选态①「杀对」：触发点载体身份 == 在场中枢身份 ⟹ 正常 Broken/Reset。
    ///
    /// 多候选面按 #291 对账实录构造：候选 B 与 A **si 同、区间异**（`si=0` 同，核心 `[13,19]` 异
    /// ——67/711 例那一面），候选 C 与 A 完全无关（源区间不相交，wf8 实测 300 例那一面）。
    /// 本用例证「A 在场、点声明 A」时校验放行且事件逐字段不变。
    #[test]
    fn kill_passes_when_point_owner_matches_alive_center() {
        // 三类破坏：声明 A ⟹ 放行。
        let (mut m, a) = machine_with_alive_a();
        let ev = m.push_point(bits_3b(), 100, Some(CenterId::of(&a)));
        assert_eq!(
            ev,
            Ok(Some(CenterLifecycleEvent::Broken {
                level: 0,
                center: a,
                born_seg_ordinal: 3,
                breaker_source_index: 100,
                breaker_side: Side::Long,
            })),
            "载体身份 == 在场身份 ⟹ 破坏放行（事件逐字段与 #291 口径不变）"
        );
        assert_eq!(m.alive_center(), None, "杀对 ⟹ 中枢确实死了");
        assert_eq!(m.counts(), (1, 1, 0));
        assert_eq!(m.mis_kills(), 0, "杀对不计误杀");

        // 一类同死：声明 A ⟹ 放行（段序列同清）。
        let (mut m2, a2) = machine_with_alive_a();
        let ev2 = m2.push_point(bits_1s(), 200, Some(CenterId::of(&a2)));
        assert_eq!(
            ev2,
            Ok(Some(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: Some(a2),
                died_born_seg_ordinal: Some(3),
                cleared_segments: 3,
                trigger_source_index: 200,
                trigger_side: Side::Short,
            })),
            "载体身份 == 在场身份 ⟹ 同死放行"
        );
        assert_eq!(m2.segments_since_reset(), 0, "放行的同死照常清段序列");
        assert_eq!(m2.mis_kills(), 0);
    }

    /// ★多候选态②「误杀拒绝」：触发点载体身份 ≠ 在场中枢身份 ⟹ 显式失败（Err），**状态不动**。
    ///
    /// 三个拒绝面各一：
    /// - B「si 同区间异」（#291 对账 67 例面）——最刁钻，只差核心；
    /// - C「源区间完全不相交」（wf8 实测 300 例面）——铁证不同实例；
    /// - `None`「点无中枢载体」——无从校验 ⟹ 一律拒（不猜）。
    #[test]
    fn mis_kill_rejected_when_point_owner_differs_from_alive_center() {
        let b = Center { zd: 13, zg: 19, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        let c = Center { zd: 90, zg: 99, dd: 88, gg: 100, start_index: 400, end_index: 460 };
        let a_id = CenterId {
            start_index: 0,
            zd: 12,
            zg: 20,
        };

        for (label, target, trigger, bits, side, src) in [
            ("三类×B(si同核心异)", Some(CenterId::of(&b)), KillTrigger::ThirdClass, bits_3b(), Side::Long, 100usize),
            ("三类×C(区间不相交)", Some(CenterId::of(&c)), KillTrigger::ThirdClass, bits_3s(), Side::Short, 101),
            ("三类×无载体", None, KillTrigger::ThirdClass, bits_3b(), Side::Long, 102),
            ("一类×B(si同核心异)", Some(CenterId::of(&b)), KillTrigger::FirstClass, bits_1b(), Side::Long, 103),
            ("一类×C(区间不相交)", Some(CenterId::of(&c)), KillTrigger::FirstClass, bits_1s(), Side::Short, 104),
            ("一类×无载体", None, KillTrigger::FirstClass, bits_1b(), Side::Long, 105),
        ] {
            let (mut m, a) = machine_with_alive_a();
            let got = m.push_point(bits, src, target);
            assert_eq!(
                got,
                Err(CenterMisKill {
                    level: 0,
                    trigger,
                    trigger_source_index: src,
                    trigger_side: side,
                    alive: a_id,
                    target,
                }),
                "{label}：载体身份 ≠ 在场身份 ⟹ 误杀显式失败（不静默杀）"
            );
            // 拒杀 ⟹ 状态一动不动（中枢仍在场、段序列不清、计数不进）。
            assert_eq!(m.alive_center(), Some((a, 3)), "{label}：拒杀后 A 仍在场");
            assert_eq!(m.segments_since_reset(), 3, "{label}：拒杀后段序列不清");
            assert_eq!(m.counts(), (1, 0, 0), "{label}：拒杀不计 broken/reset");
            assert_eq!(m.mis_kills(), 1, "{label}：误杀拒绝单独计数");
        }
    }

    /// ★R2 片三（#331）：拒杀逃逸阀读数 = **同一在场中枢实例上累计被拒的死亡请求数**
    /// （根因报告 §5 R2 原文「累积拒杀 N 次 ⟹ 该机 resync()」）。归零点恰为「在场实例换人」的
    /// 四处：出生（新实例上场）/ 破坏放行 / 一类同死 / `resync()`。**无关点（如二类点）走
    /// `Ok(None)` 不归零**——在场实例没换，此前的拒杀历史依旧适用（若被无关点打断即归零，
    /// 阀门在 wf8 实测 888 次拒杀里只放行 4 次 ≈ 形同虚设，见 #331 实测）。
    /// 纯读数，不改任何生死语义（`mis_kills()` 累计总数不受归零影响）。
    #[test]
    fn alive_mis_kills_accumulate_per_instance_and_reset_on_instance_change() {
        let (mut m, a) = machine_with_alive_a();
        let stale = CenterId { start_index: 400, zd: 90, zg: 99 };
        assert_eq!(m.alive_mis_kills(), 0, "前置：无拒杀");

        // 同一在场实例上 3 次误杀（身份不符）⟹ 累计。
        for (i, src) in [100usize, 101, 102].into_iter().enumerate() {
            let got = m.push_point(bits_3b(), src, Some(stale));
            assert!(got.is_err(), "误杀应拒绝");
            assert_eq!(m.alive_mis_kills(), i + 1, "第 {} 次误杀累加", i + 1);
        }
        assert_eq!(m.mis_kills(), 3, "误杀总计数同步累加");

        // 二类点（无载体，不触发身份校验）⟹ Ok(None)，在场实例没换 ⟹ **不归零**。
        let ev = m.push_point(bits_2b(), 103, None);
        assert_eq!(ev, Ok(None), "二类点不产事件");
        assert_eq!(m.alive_mis_kills(), 3, "无关点不换在场实例 ⟹ 累计拒杀不归零");
        assert_eq!(m.mis_kills(), 3, "总误杀计数不受影响");

        // 用正确目标杀成功 ⟹ 在场实例下场 ⟹ 归零。
        let ev2 = m.push_point(bits_3b(), 105, Some(CenterId::of(&a)));
        assert!(
            matches!(ev2, Ok(Some(CenterLifecycleEvent::Broken { .. }))),
            "载体身份匹配 ⟹ 破坏放行"
        );
        assert_eq!(m.alive_mis_kills(), 0, "杀对 ⟹ 在场实例换人，归零");
        assert_eq!(m.mis_kills(), 3, "归零只影响每实例读数，不影响累计误杀总数");

        // 一类同死归零。
        let (mut m1, a1) = machine_with_alive_a();
        m1.push_point(bits_3b(), 300, Some(stale)).unwrap_err();
        assert_eq!(m1.alive_mis_kills(), 1);
        assert!(matches!(
            m1.push_point(bits_1s(), 301, Some(CenterId::of(&a1))),
            Ok(Some(CenterLifecycleEvent::Reset { .. }))
        ));
        assert_eq!(m1.alive_mis_kills(), 0, "一类同死 ⟹ 归零");

        // resync 归零。
        let (mut m2, _a2) = machine_with_alive_a();
        m2.push_point(bits_3b(), 200, Some(stale)).unwrap_err();
        assert_eq!(m2.alive_mis_kills(), 1);
        m2.resync();
        assert_eq!(m2.alive_mis_kills(), 0, "resync 归零");
    }

    /// ★身份对「延伸」稳定：`CenterId` 只取 (si,zd,zg)——同一实例外缘 dd/gg 与 ei 随延伸变化时
    /// 仍判同一中枢（#291 对账「si+core 同、ei/dd/gg 异」33 例面：那不是误杀，不得拒）。
    #[test]
    fn center_id_ignores_envelope_and_end_index() {
        let (mut m, a) = machine_with_alive_a();
        // 塔侧同一中枢被延伸：ei 推进、外缘放大，核心 (zd,zg) 与出生坐标 si 不变。
        let extended = Center { zd: 12, zg: 20, dd: 5, gg: 30, start_index: 0, end_index: 44 };
        assert_eq!(CenterId::of(&extended), CenterId::of(&a), "延伸不改身份（核心不变）");
        let ev = m.push_point(bits_3b(), 300, Some(CenterId::of(&extended)));
        assert!(
            matches!(ev, Ok(Some(CenterLifecycleEvent::Broken { .. }))),
            "延伸后的同一中枢 ⟹ 身份仍匹配，破坏放行"
        );
        assert_eq!(m.mis_kills(), 0);
    }

    /// ★无在场中枢 ⟹ 身份校验不介入（无可杀对象，不因 target 不符而报误杀）。
    #[test]
    fn identity_check_inert_without_alive_center() {
        let stale = Center { zd: 90, zg: 99, dd: 88, gg: 100, start_index: 400, end_index: 460 };
        // 三类：诚实 no-op。
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 0, 4));
        assert_eq!(m.push_point(bits_3b(), 10, Some(CenterId::of(&stale))), Ok(None));
        assert_eq!(m.mis_kills(), 0, "无在场中枢 ⟹ 不是误杀");
        // 一类：仍清段序列，died=None。
        let ev = m.push_point(bits_1b(), 11, Some(CenterId::of(&stale)));
        assert!(
            matches!(ev, Ok(Some(CenterLifecycleEvent::Reset { died_center: None, .. }))),
            "无在场中枢 ⟹ Reset 仍清段序列且 died=None"
        );
        assert_eq!(m.mis_kills(), 0);
    }

    /// ★事件自带中枢身份（票面「破坏/重置事件携带中枢身份」）：三类事件的 `killed_center_id`
    /// 直读，下游（#292 减补动作）无需再从 Center 反推。
    #[test]
    fn events_expose_killed_center_id() {
        let (mut m, a) = machine_with_alive_a();
        let born_ev = CenterLifecycleEvent::Born { level: 0, center: a, born_seg_ordinal: 3 };
        assert_eq!(born_ev.killed_center_id(), None, "出生不杀中枢 ⟹ 无死亡身份");

        let broken = m.push_point(bits_3b(), 100, Some(CenterId::of(&a))).expect("放行").expect("有事件");
        assert_eq!(broken.killed_center_id(), Some(CenterId::of(&a)), "破坏事件携带死亡中枢身份");

        let (mut m2, a2) = machine_with_alive_a();
        let reset = m2.push_point(bits_1s(), 200, Some(CenterId::of(&a2))).expect("放行").expect("有事件");
        assert_eq!(reset.killed_center_id(), Some(CenterId::of(&a2)), "同死事件携带死亡中枢身份");

        let mut m3 = CenterEventMachine::new(0);
        m3.push_segment(unit(0, 4, up(), 0, 4));
        let reset_nodie = m3.push_point(bits_1b(), 5, None).expect("放行").expect("有事件");
        assert_eq!(reset_nodie.killed_center_id(), None, "无在场中枢的同死 ⟹ 无死亡身份");
    }
}
