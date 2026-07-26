//! 中枢生命周期事件机（#291 / SPEC #274 T1，ADR 0001 修正案一·补充二「中枢=事件」裁定；
//! ★**#336 R3 事件源改造：消费塔链，单一真相源**——用户裁定 2026-07-26）。
//!
//! ## ★R3 单一真相源（#336，本机现行口径）
//!
//! 本机**不再自建第二条中枢链**。在场中枢（「场」）= 本级塔链
//! `Classification::levels[level].centers` 的游标处实例——**与死亡请求载体取自同一张表**
//! （载体 = [`super::bsp::BspPoint::center`] 的 `OwnerRef::Center`，源表
//! `levels[level].centers`，见 `mod.rs` BSP 提取喂数）。故：
//!
//! - **出生（born）** = 该级塔链尾部新增一个中枢（塔窗口扫描成交 ⟹ 结构事件），事件在**该中枢
//!   出现在链上的那根 bar** 产出。判据仍是「前三个次级别段重叠完成」——只是**执行者**从本机
//!   自建的尾 3 段滑窗改为塔的窗口扫描（`super::recursive_tower::compose_level_resume`，本就是
//!   该判据的唯一生产实装）。禁第二套判据 ⟹ 禁第二套复算。
//! - **破坏（broken）** = 本级别确认的三类买卖点，且其载体**恰为在场实例**。三类点仍是死亡的
//!   唯一教义触发（塔本身**无破坏概念**——前缀因果塔单调增长，见 `opsem_dump.rs` 模块头
//!   「不输出 destroy 事件（诚实缺席，不伪造）」；故 broken 不可能「由塔结构事件推出」，
//!   R3 改的是**在场是谁**的真相源，不是死亡的触发源。此处如实区别于票面 #336 的概括措辞）。
//! - **一类点同死（reset）** = 本级一类点 ⟹ 在场实例死亡。★口径缩减（见下「段序列口径作废」）：
//!   「段序列清零」面随独立复算一并废止——新走势类型的中枢边界由塔链自身给出，本机不再执行
//!   「禁横跨走势类型生死边界拼中枢」（该约束的执行者移交塔；**塔是否真的执行该约束，本票未
//!   核验**，如实登记为未判定项）。
//!
//! ### 「一中枢一场」= 链游标单点（结构成立，非抑制规则）
//!
//! 旧口径靠**规则**成立：「在场中枢存在时不测出生」（`push_segment` 早退）——那是抑制，代价是
//! 在场实例只能靠死亡事件换人，一旦身份不可达即**永久吸收态锁死**（#330 §4.3 实测：264,960 bar
//! 里 `alive_si` 只有 3 个取值）。R3 下「一中枢一场」按**构造**成立：链是一个序列，游标是链上
//! 单点，同一时刻恰有一个实例在场，无需任何抑制规则。链推进 ⟹ 场自然换人。
//!
//! 链推进时若前一实例**从未收到死亡事件**，它被链推进**取代**（`superseded`）：本机
//! **不伪造** `Broken`（无三类点即无教义破坏），只如实计数 + 由旁路层落诊断行。
//!
//! ### 陈旧死亡请求（stale）与误杀拒绝（miskill）的分野（#329 校验口径收窄）
//!
//! 死亡请求载体身份与在场实例不符时，R3 下可分辨两种成因（旧实装无从分辨，一律记 miskill）：
//!
//! | 情形 | 判据 | 处置 |
//! |---|---|---|
//! | 载体 = 本级链上**已退场**实例（已被杀 / 已被链推进取代） | `target ∈ 已消费链前缀` | **陈旧请求**：不杀在场实例，不计误杀，落诊断行 |
//! | 载体**不在**本级链上（跨级错取 / 载体表被重切 / 载体缺席 `None`） | `target ∉ 已消费链前缀` | **误杀拒绝**（[`CenterMisKill`]）：状态一动不动 |
//!
//! 前者是时序滞后（载体是构造时快照，见 #330 §4.4 D6），后者是**真错位**——只有后者是回归信号。
//! **#329 校验保留为廉价看门狗**（每个已确认点一次身份比对），存废理由见 [`CenterMisKill`]。
//!
//! ## 已作废/已移除（谱系注记，tombstone 原位）
//!
//! - ★**独立复算路径已移除**（#336 R3）：`push_segment(UnitRange)` + `build` 构造算子指针
//!   （L0 [`super::center::center_from_segments`] / ℓ≥1 [`super::center::center_from_window`]）
//!   + `segs: Vec<UnitRange>` 尾 3 段滑窗 + `segments_since_reset()` **全部删除**。
//!   **建造错误 = 重造没接生产，同型事故**（#291 把塔既有的中枢构造重实现了一遍，产出一条
//!   塔链外的第二条中枢链；#330 §3 例 1 实测 `si=624` 身份在塔 720 条 `new_center` 中**零命中**）。
//!   R3 废止之。同型先例见 `.chanlun/review-results/built-but-unwired-pattern-20260723.md`。
//! - ★**#291 wf8 对账基线全部作废**（#336 票面裁定）：原对账读数（含 255/711、83/751 一致率、
//!   763 born / 733 broken / 59 reset 等）全部建立在**独立复算的第二条链**上，随该路径废止而
//!   **一并作废**，不得再作任何比较基准。新基线 = 本票 wf8 实测（报告
//!   `.chanlun/review-results/center-event-machine-r3-chain-consumption-20260726.md`）。
//! - ★**段序列口径全部作废**（#336 R3）：模块旧「段计数口径」整节（`born_seg_ordinal` 分母、
//!   「三类破坏不清段序列」旧口径、其 #331 R0′ 修正口径「已消费段不再参与新中枢计数」、
//!   一类同死清零段序列、`Reset::cleared_segments`）随段序列本身删除而作废。出生序号口径
//!   改为 **链下标**（`chain_index` = 该实例在 `levels[level].centers` 中的 0-based 下标）。
//! - ★**#331 R8（塔层索引对齐）退役**（口径吸收）：R8 修的是「事件机吃第几层塔单元」——R3 下
//!   本机**不再投影任何塔单元**（`project_to_units_resume` / `confirmed_lens` 水线 / `blocks`
//!   配对全部从本路径消失），级别对齐按定义成立：机器 level ℓ ⟺ `levels[ℓ].centers`。R8 的
//!   **结论**（事件机 level ℓ 应与 `levels[ℓ].centers` 同层）被 R3 吸收为构造性事实。
//! - ★**#331 R0′（破坏后段游标推进）退役**：其对象（段序列）已不存在；「已消费段不倒回」的
//!   执行者本就是塔窗口扫描游标 `i=j*`（`super::recursive_tower` 窗口扫描），R3 直接消费其结果。
//! - ★**#331 R2（拒杀逃逸阀）退役**：`alive_miskills` / `alive_mis_kills()` / 旁路层
//!   `MISKILL_ESCAPE_N` 逃逸阀全部删除。其对象——「在场实例只能靠死亡事件换人 ⟹ 吸收态锁死」
//!   ——已被 R3 的链游标推进消灭（无吸收态则无需逃逸）。留着反而重演「resync 后从前缀头回放
//!   ⟹ 确定性复生同一身份」的噪声（#331 §2.2 实测 51 次 born 只 2 个身份）。
//!
//! ## 边界（票面 #291 范围，不变）
//!
//! - **只产出事件，不产出动作**：本机是结构地基（狭义短差减补动作 = #292），事件不触发任何
//!   交易行为；默认零行为变化（事件产出经 env-gated 只读旁路外化，验收锚 = 既有轨迹逐位不变）。
//! - 中枢**延伸/升级不是本机事件**（塔 `tower_events` 的 extend/level_upgrade 域）：本机在场
//!   实例保持出生时刻的 ZD/ZG 快照（[`CenterId`] 对延伸稳定——延伸不改核心）。
//! - 一类+三类 bit 同点：一类优先（同死吞没破坏——1B/3B 前提冲突互斥，bsp.rs
//!   `no_exclusive_trichotomy` 结构下理论不同位；防御性规定，照实标注）。
//! - **链前缀分叉**（该级塔缓存全量重置 / 链回缩）⟹ 工程**重基**（[`ChainConsumed::Rebased`]），
//!   不伪造出生/死亡；重基把场重置到链尾实例——若该实例此前已被死亡事件杀掉，重基会让它
//!   重新在场（重基是工程再同步，非教义生死，条数如实计数）。

use super::super::types::{BspBits, Center, Side, Tick};

/// 中枢**实例身份**（#329 H1）：`(出生坐标 si, 核心 ZD, 核心 ZG)` 三元组。
///
/// ## 为什么是这三元组（票面「择与既有结构最简一致者」）
///
/// 不引入新的实例 id 侧车——身份直接从既有 [`Center`] 读出：
/// - `start_index`：中枢首单元在 L0 原始 K 序的起点（所有级别的单元坐标统一在 L0 K 序）
///   ⟹ 跨级别可比，且在链上唯一标定实例。
/// - `(zd, zg)`：核心区间。**延伸不改核心**（`recursive_tower.rs:265`）⟹ 同一实例被塔延伸后
///   核心仍相等，身份对「延伸」稳定；而不同实例的核心几乎必异。
///
/// 外缘 `dd/gg` 与 `end_index` **不进**身份：二者随延伸/窗口推进而变，进身份会把「同一中枢被
/// 延伸」误判成「不同中枢」。
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

/// **误杀拒绝证据**（#329 H1；★#336 R3 口径收窄）：死亡请求载体身份**不在本级塔链上** ⟹
/// 显式失败，状态一动不动。
///
/// ## R3 后为什么还留着（票面 #336 第 4 项「存废二选一」的裁决：**保留为廉价看门狗**）
///
/// R3 让在场实例与载体取自**同一张表**（`levels[ℓ].centers`）⟹ 「两条链互校」这层语义确实
/// 消失，`target ∈ 链` 变成近恒真。但**近恒真不是恒真**，剩下的反例正是真回归信号：
///
/// - **载体不在本级链上**：跨级错取（#330 §3 例 3/4 那一面：ℓ≥1 载体高一层，96 条）、
///   载体表被重切后旧快照失效、`#148` 升级重切。这类错位在 R3 下**不再可能由链滞后造成**
///   ⟹ 一旦出现就是接线/口径回归，而非时序噪声。
/// - **载体缺席**（`target = None`）：点无中枢载体却要杀中枢——无从校验 ⟹ 一律拒（不猜）。
///
/// 代价 = 每个已确认点一次三元组比对（O(1) + 链上线性查找），可忽略。收益 = 上述两类回归
/// 立即显影。故**不退役**。载体命中链上**已退场**实例的情形**不再计入本证据**（那是时序滞后，
/// 见 [`StaleKillRequest`]）——口径收窄如实登记，miskill 读数与 R3 前**不可比**。
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
    /// 在场实例身份（链游标处）。
    pub alive: CenterId,
    /// 触发点声明要杀的中枢身份；`None` = 该点无中枢载体（二类锚 / 载体缺席）。
    pub target: Option<CenterId>,
}

/// ★**陈旧死亡请求**（#336 R3 新增）：载体命中本级塔链上**已退场**实例（已被死亡事件杀掉，
/// 或已被链推进取代）⟹ 不杀在场实例（拒杀优先于错杀），**不计误杀**，只落诊断行。
///
/// 成因是时序滞后而非错位：载体是买卖点**构造时**的快照（`newly_confirmed_step` 整体 clone
/// `BspPoint`），而点的确认可能晚出很多 bar（#330 §4.4 实测 `bar - src` 中位 56、最大 1595）
/// ⟹ 点确认时链早已推进过该实例。这不是接线错误，把它计进 miskill 会把噪声当回归。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleKillRequest {
    /// 事件机级别。
    pub level: u32,
    /// 触发类（一类同死 / 三类破坏）。
    pub trigger: KillTrigger,
    /// 触发点 source_index（确认坐标）。
    pub trigger_source_index: usize,
    /// 触发点方向。
    pub trigger_side: Side,
    /// 在场实例身份（链游标处）。
    pub alive: CenterId,
    /// 载体声明要杀的、链上已退场的实例身份。
    pub target: CenterId,
    /// 载体实例在链上的 0-based 下标（溯源：与在场下标之差即滞后深度）。
    pub target_chain_index: usize,
    /// 在场实例在链上的 0-based 下标。
    pub alive_chain_index: usize,
}

/// 中枢生命周期事件（#291 三类：born/broken/reset）。★#336 R3：出生序号口径由「段号」
/// 改为**链下标**（`levels[level].centers` 的 0-based 下标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterLifecycleEvent {
    /// 出生：本级塔链尾部新增该中枢（塔窗口扫描成交 ⟹ 结构事件）。
    Born {
        level: u32,
        /// 出生时刻中枢（核心 [zd,zg] + 外缘 [dd,gg] + 源坐标 [start_index,end_index]）。
        center: Center,
        /// 该实例在本级塔链 `levels[level].centers` 中的 0-based 下标。
        chain_index: usize,
    },
    /// 破坏：本级别确认的三类买卖点，其载体恰为在场实例 ⟹ 在场实例死亡。
    Broken {
        level: u32,
        /// 死亡中枢（出生时刻快照）。
        center: Center,
        /// 该实例的链下标（溯源）。
        chain_index: usize,
        /// 触发破坏的三类点 source_index（确认坐标）。
        breaker_source_index: usize,
        /// 三类点方向（buy3 ⟹ Long / sell3 ⟹ Short，types::Side 买卖语境）。
        breaker_side: Side,
    },
    /// 一类点同死：本级一类点 ⟹ 在场实例（若有）死亡。★R3：「段序列清零」面已作废
    /// （本机无段序列——新走势类型的中枢边界由塔链自身给出，见模块头）。
    Reset {
        level: u32,
        /// 同死的在场实例（出生快照）；场为空 ⟹ None（一类点边界仍如实记录）。
        died_center: Option<Center>,
        /// 同死实例的链下标（场为空 ⟹ None）。
        died_chain_index: Option<usize>,
        /// 触发同死的一类点 source_index（确认坐标）。
        trigger_source_index: usize,
        /// 一类点方向（buy1 ⟹ Long / sell1 ⟹ Short）。
        trigger_side: Side,
    },
}

/// ★#336 R3：一次塔链消费的结果（本机唯一的**出生**事件来源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainConsumed {
    /// 前缀一致 ⟹ 正常推进。`events` = 按链序产出的 [`CenterLifecycleEvent::Born`]（可能空）；
    /// `superseded` = 本次推进中被链推进取代（从未收到死亡事件）的在场实例数。
    Advanced { events: Vec<CenterLifecycleEvent>, superseded: usize },
    /// **首次消费**（本机尚未与链同步）⟹ 静默采纳既有链前缀为历史，游标落链尾。
    /// 不伪造出生 bar——这些中枢是在本机开机（首个交易活跃 bar）之前就在链上的。
    /// 与 `tower_events` 同款纪律（旁路层首个活跃 bar 亦不重播既有 Compose）。
    Adopted { adopted: usize },
    /// **前缀分叉**（该级塔缓存全量重置 / 链回缩）⟹ 工程重基：静默采纳当前链，游标落链尾，
    /// 不伪造出生/死亡。`at` = 检出分叉的链下标；`len` = 重基后链长。
    Rebased { at: usize, len: usize },
}

/// 死亡请求对链解析的内部三态（[`CenterEventMachine::resolve_kill_target`]）。
enum KillResolution {
    /// 场为空：无可杀对象（校验不介入）。
    ArenaEmpty,
    /// 载体 == 在场身份：放行。
    Alive,
    /// 载体命中链上已退场实例：陈旧请求（不杀，不计误杀）。
    Stale(StaleKillRequest),
}

/// 一个已确认买卖点被喂入后的结果（★#336 R3：新增陈旧请求分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointOutcome {
    /// 无事件：无关点（二类点等），或场为空时的三类点诚实 no-op（不杀不存在的中枢）。
    Silent,
    /// 教义事件（broken / reset；born 不由本路产出）。
    Event(CenterLifecycleEvent),
    /// ★R3：陈旧死亡请求（载体命中链上已退场实例）⟹ 不杀在场实例，不计误杀。
    Stale(StaleKillRequest),
}

/// 中枢生命周期事件机（每级别一台）。★#336 R3：**塔链消费器**——不含任何中枢判据/复算，
/// 在场实例来自 `Classification::levels[level].centers`（唯一真相源）。
pub struct CenterEventMachine {
    level: u32,
    /// ★R3：已消费的塔链前缀身份（下标 = 链上 0-based 中枢序号）。前缀一致性守卫 + 陈旧请求判据。
    consumed: Vec<CenterId>,
    /// 是否已与链同步过（`false` ⟹ 下次消费走静默采纳，见 [`ChainConsumed::Adopted`]）。
    synced: bool,
    /// 在场实例（「场」）：(出生快照, 链下标)。None = 链空 或 在场实例已被死亡事件杀掉。
    alive: Option<(Center, usize)>,
    born_total: usize,
    broken_total: usize,
    reset_total: usize,
    /// ★#329：误杀拒绝累计（载体不在本级链上 ⟹ 拒杀，状态不动）。
    miskill_total: usize,
    /// ★R3：陈旧死亡请求累计（载体命中链上已退场实例；时序滞后，非错位）。
    stale_total: usize,
    /// ★R3：被链推进取代（从未收到死亡事件）的在场实例累计。
    superseded_total: usize,
}

impl CenterLifecycleEvent {
    /// ★#329：本事件**杀掉的中枢身份**（票面「破坏/重置事件携带中枢身份」）。
    ///
    /// `Broken` ⟹ 被破坏中枢；`Reset` ⟹ 同死的在场实例（场为空 ⟹ `None`）；`Born` ⟹ `None`
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
    /// 构造级别 `level` 的事件机（★R3：无构造算子——判据由塔承担，本机只消费链）。
    pub fn new(level: u32) -> Self {
        Self {
            level,
            consumed: Vec::new(),
            synced: false,
            alive: None,
            born_total: 0,
            broken_total: 0,
            reset_total: 0,
            miskill_total: 0,
            stale_total: 0,
            superseded_total: 0,
        }
    }

    /// ★#336 R3 事件源：消费本级塔链 `Classification::levels[level].centers` 的当前状态。
    ///
    /// 每 bar 调一次（幂等：链未增长 ⟹ `Advanced{events:[], superseded:0}`）。语义三分支见
    /// [`ChainConsumed`]。
    pub fn consume_chain(&mut self, chain: &[Center]) -> ChainConsumed {
        // ① 首次消费：静默采纳既有链前缀（这些中枢在本机开机前就在链上，不伪造出生 bar）。
        if !self.synced {
            self.synced = true;
            self.adopt(chain);
            return ChainConsumed::Adopted { adopted: chain.len() };
        }
        // ② 前缀分叉守卫（O(1)/bar/级）：链回缩 或 已消费末条身份被改写 ⟹ 工程重基。
        //
        // **诚实的有效域**：只查「长度回缩」与「已消费末条身份」，**不做全前缀比对**——全比对
        // 是 O(链长)/bar/级（wf8 26 万 bar × 5 级 ⟹ 数量级 1e9 次比较，不可接受）。依据：
        // `LevelCache.centers` 文档「前缀不可变，尾部经 Rc::make_mut 追加」，唯一的前缀变异源是
        // 该级缓存**全量重置**（frontier 变异检测触发，见 `mod.rs` `cached_units` 文档），而全量
        // 重置几乎必然改写末条（frontier 就在末端）。**比这更深的前缀改写本守卫检不出**，如实
        // 登记为已知限。延伸不改核心 ⟹ [`CenterId`] 对延伸稳定 ⟹ 本守卫不会被延伸误触发。
        let k = self.consumed.len();
        if k > 0 {
            let tail_same = chain
                .get(k - 1)
                .map(|c| CenterId::of(c) == self.consumed[k - 1])
                .unwrap_or(false);
            if chain.len() < k || !tail_same {
                let at = if chain.len() < k { chain.len() } else { k - 1 };
                self.adopt(chain);
                return ChainConsumed::Rebased { at, len: chain.len() };
            }
        }
        // ③ 正常推进（尾部追加）⟹ 逐个新中枢产 Born，游标落链尾。
        let mut events = Vec::new();
        let mut superseded = 0usize;
        for (idx, c) in chain.iter().enumerate().skip(k) {
            // 前一实例从未收到死亡事件 ⟹ 被链推进取代（不伪造 Broken——三类点是死亡的唯一
            // 教义触发，塔无破坏概念，见模块头）。
            if self.alive.is_some() {
                superseded += 1;
                self.superseded_total += 1;
            }
            self.consumed.push(CenterId::of(c));
            self.alive = Some((*c, idx));
            self.born_total += 1;
            events.push(CenterLifecycleEvent::Born {
                level: self.level,
                center: *c,
                chain_index: idx,
            });
        }
        ChainConsumed::Advanced { events, superseded }
    }

    /// 静默采纳当前链为已消费前缀（首次消费 / 前缀分叉重基共用）：游标落链尾，不产任何事件。
    fn adopt(&mut self, chain: &[Center]) {
        self.consumed = chain.iter().map(CenterId::of).collect();
        self.alive = chain.last().map(|c| (*c, chain.len() - 1));
    }

    /// 喂入一个**已确认**买卖点（修6：只消费已确认的点）。
    ///
    /// 一类 bit（buy1/sell1）⟹ `Reset`（在场实例同死；一类优先于三类）；否则三类 bit
    /// （buy3/sell3）且场非空 ⟹ `Broken`。二类点不产事件。
    pub fn push_point(
        &mut self,
        bits: BspBits,
        source_index: usize,
        target: Option<CenterId>,
    ) -> Result<PointOutcome, CenterMisKill> {
        // 一类优先（1B/3B 前提冲突互斥，理论不同位；防御性规定，模块头已标注）。
        let first = if bits.buy1 {
            Some(Side::Long)
        } else if bits.sell1 {
            Some(Side::Short)
        } else {
            None
        };
        if let Some(trigger_side) = first {
            match self.resolve_kill_target(KillTrigger::FirstClass, source_index, trigger_side, target)? {
                KillResolution::Stale(st) => return Ok(PointOutcome::Stale(st)),
                KillResolution::ArenaEmpty | KillResolution::Alive => {}
            }
            let (died_center, died_chain_index) = match self.alive.take() {
                Some((c, idx)) => (Some(c), Some(idx)),
                None => (None, None),
            };
            self.reset_total += 1;
            return Ok(PointOutcome::Event(CenterLifecycleEvent::Reset {
                level: self.level,
                died_center,
                died_chain_index,
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
            match self.resolve_kill_target(KillTrigger::ThirdClass, source_index, breaker_side, target)? {
                KillResolution::Stale(st) => return Ok(PointOutcome::Stale(st)),
                // 场为空 ⟹ 诚实 no-op（不杀不存在的中枢）。
                KillResolution::ArenaEmpty => return Ok(PointOutcome::Silent),
                KillResolution::Alive => {}
            }
            let (center, chain_index) = self.alive.take().expect("Alive 分支蕴含场非空");
            self.broken_total += 1;
            return Ok(PointOutcome::Event(CenterLifecycleEvent::Broken {
                level: self.level,
                center,
                chain_index,
                breaker_source_index: source_index,
                breaker_side,
            }));
        }
        // 无一/三类 bit（二类点等无关点）⟹ 无事件，不进身份校验。
        Ok(PointOutcome::Silent)
    }

    /// 死亡请求对链解析（★#329 校验的 R3 口径，见 [`CenterMisKill`] / [`StaleKillRequest`]）。
    ///
    /// - **场为空** ⟹ [`KillResolution::ArenaEmpty`]：无可杀对象，谈不上误杀（口径不变）。
    /// - 载体 == 在场身份 ⟹ [`KillResolution::Alive`]：放行。
    /// - 载体命中**已消费链前缀里的其它实例** ⟹ [`KillResolution::Stale`]：时序滞后，不计误杀。
    /// - 其余（载体不在本级链上，含 `None`）⟹ [`Err(CenterMisKill)`](CenterMisKill)，状态不动。
    fn resolve_kill_target(
        &mut self,
        trigger: KillTrigger,
        source_index: usize,
        trigger_side: Side,
        target: Option<CenterId>,
    ) -> Result<KillResolution, CenterMisKill> {
        let Some((alive_center, alive_chain_index)) = self.alive else {
            return Ok(KillResolution::ArenaEmpty);
        };
        let alive = CenterId::of(&alive_center);
        if target == Some(alive) {
            return Ok(KillResolution::Alive);
        }
        if let Some(t) = target {
            // 链上线性查找（自尾向头——陈旧请求多命中靠近游标处）。
            if let Some(target_chain_index) = self.consumed.iter().rposition(|c| *c == t) {
                self.stale_total += 1;
                return Ok(KillResolution::Stale(StaleKillRequest {
                    level: self.level,
                    trigger,
                    trigger_source_index: source_index,
                    trigger_side,
                    alive,
                    target: t,
                    target_chain_index,
                    alive_chain_index,
                }));
            }
        }
        self.miskill_total += 1;
        Err(CenterMisKill {
            level: self.level,
            trigger,
            trigger_source_index: source_index,
            trigger_side,
            alive,
            target,
        })
    }

    /// 在场实例（出生快照 + 链下标）；None = 场为空。
    pub fn alive_center(&self) -> Option<(Center, usize)> {
        self.alive
    }

    /// 已消费的链长度（= 本机认为该级链上有多少中枢）。
    pub fn chain_len(&self) -> usize {
        self.consumed.len()
    }

    /// (born, broken, reset) 累计事件计数（wf8 自证读数）。
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.born_total, self.broken_total, self.reset_total)
    }

    /// ★#329：误杀拒绝累计（载体不在本级链上被拒的死亡请求数；口径见 [`CenterMisKill`]）。
    pub fn mis_kills(&self) -> usize {
        self.miskill_total
    }

    /// ★R3：陈旧死亡请求累计（载体命中链上已退场实例；时序滞后，不计误杀）。
    pub fn stale_requests(&self) -> usize {
        self.stale_total
    }

    /// ★R3：被链推进取代（从未收到死亡事件）的在场实例累计。
    pub fn superseded(&self) -> usize {
        self.superseded_total
    }
}

#[cfg(test)]
mod tests {
    //! ★#336 R3 测试口径重写（谱系注记）：本模块原有 14 条单测**全部**写在已删除的独立复算
    //! 路径（`push_segment` 尾 3 段滑窗 + `center_from_segments/center_from_window` 复算 +
    //! 段序列口径 + #331 R0′/R2）上，随该路径一并删除——其固化的读数（`born_seg_ordinal`
    //! 计数、破坏后段序列长度、`alive_mis_kills` 归零点）在 R3 下**无对应物**，不是回归而是
    //! 口径消失。新测试的接缝（seam）= [`CenterEventMachine`] 公开面：
    //! `consume_chain` / `push_point` / `alive_center` / `chain_len` / `counts` /
    //! `mis_kills` / `stale_requests` / `superseded`。

    use super::*;

    /// 造一个中枢（核心 [zd,zg]，外缘 [dd,gg]，源区间 [si,ei]）。
    fn center(si: usize, ei: usize, zd: Tick, zg: Tick) -> Center {
        Center { zd, zg, dd: zd - 2, gg: zg + 2, start_index: si, end_index: ei }
    }

    fn bits_1b() -> BspBits { BspBits { buy1: true, ..Default::default() } }
    fn bits_1s() -> BspBits { BspBits { sell1: true, ..Default::default() } }
    fn bits_3b() -> BspBits { BspBits { buy3: true, ..Default::default() } }
    fn bits_3s() -> BspBits { BspBits { sell3: true, ..Default::default() } }
    fn bits_2b() -> BspBits { BspBits { buy2: true, ..Default::default() } }

    // ──────────────────────────────────────────────────────────────────────
    //  ★R3 片一：事件源 = 塔链消费（born 由链推进直接推出）
    // ──────────────────────────────────────────────────────────────────────

    /// ★首次消费静默采纳：本机开机（首个交易活跃 bar）时链上已有的中枢**不伪造出生**
    /// （它们的出生 bar 在开机之前）；游标落链尾 ⟹ 场 = 链尾实例。
    #[test]
    fn first_consume_adopts_existing_chain_without_fabricating_births() {
        let mut m = CenterEventMachine::new(0);
        let chain = vec![center(5, 260, 100, 200), center(300, 600, 150, 250)];
        assert_eq!(
            m.consume_chain(&chain),
            ChainConsumed::Adopted { adopted: 2 },
            "首次消费 ⟹ 静默采纳既有链前缀（不产 born）"
        );
        assert_eq!(m.counts(), (0, 0, 0), "采纳不计出生（不伪造出生 bar）");
        assert_eq!(m.chain_len(), 2);
        assert_eq!(m.alive_center(), Some((chain[1], 1)), "场 = 链尾实例（游标单点）");
    }

    /// ★空链首次消费 ⟹ 采纳 0 条；此后链上每新增一个中枢都产 born（本机开机后出生的中枢
    /// 逐条可见，born bar = 该中枢出现在链上那根 bar）。
    #[test]
    fn chain_growth_after_sync_births_each_new_center() {
        let mut m = CenterEventMachine::new(0);
        assert_eq!(m.consume_chain(&[]), ChainConsumed::Adopted { adopted: 0 }, "空链采纳 0 条");
        let c0 = center(5, 260, 100, 200);
        assert_eq!(
            m.consume_chain(&[c0]),
            ChainConsumed::Advanced {
                events: vec![CenterLifecycleEvent::Born { level: 0, center: c0, chain_index: 0 }],
                superseded: 0,
            },
            "链新增 ⟹ born（链下标 0）"
        );
        assert_eq!(m.counts(), (1, 0, 0));
        assert_eq!(m.alive_center(), Some((c0, 0)));
        // 幂等：链未增长 ⟹ 无事件（每 bar 调用不重复产出）。
        assert_eq!(
            m.consume_chain(&[c0]),
            ChainConsumed::Advanced { events: Vec::new(), superseded: 0 },
            "链未增长 ⟹ 幂等无事件"
        );
        assert_eq!(m.counts(), (1, 0, 0));
    }

    /// ★「一中枢一场」按构造成立（非抑制规则）：一次消费喂入 3 个新中枢 ⟹ 3 个 born 按链序
    /// 产出，场恒为链尾单点；前两个实例从未收到死亡事件 ⟹ 被链推进**取代**（`superseded=2`），
    /// 本机**不伪造** Broken。
    #[test]
    fn one_arena_holds_by_chain_cursor_and_undead_predecessors_are_superseded() {
        let mut m = CenterEventMachine::new(0);
        m.consume_chain(&[]);
        let (c0, c1, c2) = (center(5, 260, 100, 200), center(300, 600, 150, 250), center(700, 900, 180, 280));
        let got = m.consume_chain(&[c0, c1, c2]);
        assert_eq!(
            got,
            ChainConsumed::Advanced {
                events: vec![
                    CenterLifecycleEvent::Born { level: 0, center: c0, chain_index: 0 },
                    CenterLifecycleEvent::Born { level: 0, center: c1, chain_index: 1 },
                    CenterLifecycleEvent::Born { level: 0, center: c2, chain_index: 2 },
                ],
                superseded: 2,
            },
            "3 个新中枢 ⟹ 3 born（按链序），前 2 个未收死亡事件 ⟹ superseded=2"
        );
        assert_eq!(m.counts(), (3, 0, 0), "取代不计 broken（无三类点即无教义破坏）");
        assert_eq!(m.superseded(), 2);
        assert_eq!(m.alive_center(), Some((c2, 2)), "场 = 链尾单点（一中枢一场按构造成立）");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★R3 片二：死亡事件（三类/一类点）对链解析——载体恰为在场实例 ⟹ 放行
    // ──────────────────────────────────────────────────────────────────────

    /// 建一台 level0 机器，先与空链同步，再消费 `chain`（⟹ 逐条 born，场 = 链尾实例）。
    fn machine_on_chain(chain: &[Center]) -> CenterEventMachine {
        let mut m = CenterEventMachine::new(0);
        m.consume_chain(&[]);
        m.consume_chain(chain);
        m
    }

    /// ★三类点破在场实例：载体 == 在场身份 ⟹ Broken（携链下标/触发坐标/方向），场清空；
    /// 之后链再推进 ⟹ 新实例出生接场（不再需要「一中枢一场」抑制规则）。
    #[test]
    fn third_class_point_breaks_alive_chain_instance() {
        let (c0, c1) = (center(5, 260, 100, 200), center(300, 600, 150, 250));
        let mut m = machine_on_chain(&[c0]);
        let ev = m.push_point(bits_3b(), 100, Some(CenterId::of(&c0)));
        assert_eq!(
            ev,
            Ok(PointOutcome::Event(CenterLifecycleEvent::Broken {
                level: 0,
                center: c0,
                chain_index: 0,
                breaker_source_index: 100,
                breaker_side: Side::Long,
            })),
            "载体 == 在场身份 ⟹ 破坏放行"
        );
        assert_eq!(m.alive_center(), None, "破坏后场为空");
        assert_eq!(m.counts(), (1, 1, 0));
        assert_eq!(m.mis_kills(), 0, "杀对不计误杀");
        assert_eq!(m.superseded(), 0, "被死亡事件杀掉 ≠ 被链推进取代");

        // 链推进 ⟹ 新实例出生接场（被杀实例不复活）。
        let got = m.consume_chain(&[c0, c1]);
        assert_eq!(
            got,
            ChainConsumed::Advanced {
                events: vec![CenterLifecycleEvent::Born { level: 0, center: c1, chain_index: 1 }],
                superseded: 0,
            },
            "场为空时链推进 ⟹ born 接场，superseded=0（前一实例已被合法杀掉）"
        );
        assert_eq!(m.alive_center(), Some((c1, 1)));
    }

    /// ★一类点同死：载体 == 在场身份 ⟹ Reset（携链下标）。★R3 口径：`cleared_segments`
    /// （段序列清零）面已随独立复算作废——本事件只记「在场实例同死」。
    #[test]
    fn first_class_point_kills_alive_chain_instance() {
        let c0 = center(5, 260, 100, 200);
        let mut m = machine_on_chain(&[c0]);
        let ev = m.push_point(bits_1s(), 200, Some(CenterId::of(&c0)));
        assert_eq!(
            ev,
            Ok(PointOutcome::Event(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: Some(c0),
                died_chain_index: Some(0),
                trigger_source_index: 200,
                trigger_side: Side::Short,
            })),
            "一类点 ⟹ reset（在场实例同死）"
        );
        assert_eq!(m.alive_center(), None);
        assert_eq!(m.counts(), (1, 0, 1));
    }

    /// ★一类+三类 bit 同点：一类优先（同死吞没破坏；防御性规定，理论互斥）。
    #[test]
    fn first_class_wins_over_third_class_on_same_point() {
        let c0 = center(5, 260, 100, 200);
        let mut m = machine_on_chain(&[c0]);
        let both = BspBits { buy1: true, buy3: true, ..Default::default() };
        let ev = m.push_point(both, 60, Some(CenterId::of(&c0)));
        assert!(
            matches!(ev, Ok(PointOutcome::Event(CenterLifecycleEvent::Reset { .. }))),
            "一类+三类同点 ⟹ 一类优先"
        );
        assert_eq!(m.counts(), (1, 0, 1));
    }

    /// ★二类点不产事件（不在 born/broken/reset 三类教义事件内），且不动在场实例。
    #[test]
    fn second_class_point_produces_no_event() {
        let c0 = center(5, 260, 100, 200);
        let mut m = machine_on_chain(&[c0]);
        // 二类点载体是 Type1Anchor（无中枢身份）⟹ target=None；不产事件 ⟹ 身份校验不触发。
        assert_eq!(m.push_point(bits_2b(), 70, None), Ok(PointOutcome::Silent), "二类点不产事件");
        assert_eq!(m.counts(), (1, 0, 0));
        assert_eq!(m.mis_kills(), 0, "二类点不进身份校验");
        assert!(m.alive_center().is_some(), "二类点不动在场实例");
    }

    /// ★场为空 ⟹ 身份校验不介入（无可杀对象，不因载体不符而报误杀）：三类点诚实 no-op；
    /// 一类点仍如实记录边界（died=None）。
    #[test]
    fn identity_check_inert_when_arena_empty() {
        let stale_id = CenterId { start_index: 9999, zd: 1, zg: 2 };
        let mut m = CenterEventMachine::new(0);
        m.consume_chain(&[]);
        assert_eq!(m.alive_center(), None, "前置：链空 ⟹ 场为空");
        assert_eq!(m.push_point(bits_3b(), 10, Some(stale_id)), Ok(PointOutcome::Silent));
        assert_eq!(m.mis_kills(), 0, "场为空 ⟹ 不是误杀");
        let ev = m.push_point(bits_1b(), 11, Some(stale_id));
        assert_eq!(
            ev,
            Ok(PointOutcome::Event(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: None,
                died_chain_index: None,
                trigger_source_index: 11,
                trigger_side: Side::Long,
            })),
            "场为空 ⟹ 一类点边界仍如实记录（died=None）"
        );
        assert_eq!(m.mis_kills(), 0);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★R3 片三：stale（时序滞后）与 miskill（真错位）的分野——#329 校验口径收窄
    // ──────────────────────────────────────────────────────────────────────

    /// ★陈旧请求①「载体命中被链推进取代的实例」：链 [c0,c1]、场 = c1，点声明 c0
    /// ⟹ 陈旧（不杀 c1，不计误杀），携双方链下标（Δidx = target−alive 为负 = 滞后深度）。
    #[test]
    fn stale_request_when_target_is_superseded_chain_instance() {
        let (c0, c1) = (center(5, 260, 100, 200), center(300, 600, 150, 250));
        let mut m = machine_on_chain(&[c0, c1]);
        assert_eq!(m.alive_center(), Some((c1, 1)), "前置：场 = 链尾 c1");
        let got = m.push_point(bits_3s(), 500, Some(CenterId::of(&c0)));
        assert_eq!(
            got,
            Ok(PointOutcome::Stale(StaleKillRequest {
                level: 0,
                trigger: KillTrigger::ThirdClass,
                trigger_source_index: 500,
                trigger_side: Side::Short,
                alive: CenterId::of(&c1),
                target: CenterId::of(&c0),
                target_chain_index: 0,
                alive_chain_index: 1,
            })),
            "载体命中链上已退场实例 ⟹ 陈旧请求（时序滞后，非错位）"
        );
        assert_eq!(m.alive_center(), Some((c1, 1)), "陈旧请求不动在场实例（拒杀优先）");
        assert_eq!(m.counts(), (2, 0, 0), "陈旧请求不计 broken/reset");
        assert_eq!(m.stale_requests(), 1);
        assert_eq!(m.mis_kills(), 0, "★口径收窄：陈旧请求**不**计误杀");
    }

    /// ★陈旧请求②「载体命中已被死亡事件合法杀掉的实例」：c0 被三类点杀 → 链推进出生 c1
    /// ⟹ 再有点声明 c0 也是陈旧（已死实例不复活）。
    #[test]
    fn stale_request_when_target_was_already_legitimately_killed() {
        let (c0, c1) = (center(5, 260, 100, 200), center(300, 600, 150, 250));
        let mut m = machine_on_chain(&[c0]);
        assert!(m.push_point(bits_3b(), 100, Some(CenterId::of(&c0))).is_ok());
        m.consume_chain(&[c0, c1]);
        assert_eq!(m.alive_center(), Some((c1, 1)));
        let got = m.push_point(bits_1b(), 700, Some(CenterId::of(&c0)));
        assert!(
            matches!(got, Ok(PointOutcome::Stale(StaleKillRequest { target_chain_index: 0, .. }))),
            "已被合法杀掉的实例再被声明 ⟹ 陈旧请求，实得 {got:?}"
        );
        assert_eq!(m.counts(), (2, 1, 0), "陈旧的一类点不产 reset");
        assert_eq!(m.mis_kills(), 0);
    }

    /// ★误杀拒绝（R3 后**仅剩**这一面）：载体**不在**本级链上 ⟹ 显式失败，状态一动不动。
    /// 两个面各一：`Some(链外身份)`（跨级错取/载体表重切）与 `None`（载体缺席，无从校验 ⟹ 不猜）。
    #[test]
    fn mis_kill_rejected_when_target_absent_from_chain() {
        let (c0, c1) = (center(5, 260, 100, 200), center(300, 600, 150, 250));
        // 链外身份：核心与源坐标都不在 [c0,c1] 里（wf8 实测「target 核心在塔链查无」那一面）。
        let off_chain = CenterId { start_index: 90_000, zd: 777, zg: 888 };
        for (label, target, trigger, bits, side, src) in [
            ("三类×链外身份", Some(off_chain), KillTrigger::ThirdClass, bits_3b(), Side::Long, 100usize),
            ("三类×载体缺席", None, KillTrigger::ThirdClass, bits_3s(), Side::Short, 101),
            ("一类×链外身份", Some(off_chain), KillTrigger::FirstClass, bits_1b(), Side::Long, 102),
            ("一类×载体缺席", None, KillTrigger::FirstClass, bits_1s(), Side::Short, 103),
        ] {
            let mut m = machine_on_chain(&[c0, c1]);
            let got = m.push_point(bits, src, target);
            assert_eq!(
                got,
                Err(CenterMisKill {
                    level: 0,
                    trigger,
                    trigger_source_index: src,
                    trigger_side: side,
                    alive: CenterId::of(&c1),
                    target,
                }),
                "{label}：载体不在本级链上 ⟹ 误杀显式失败（不静默杀）"
            );
            assert_eq!(m.alive_center(), Some((c1, 1)), "{label}：拒杀后场不动");
            assert_eq!(m.counts(), (2, 0, 0), "{label}：拒杀不计 broken/reset");
            assert_eq!(m.mis_kills(), 1, "{label}：误杀单独计数");
            assert_eq!(m.stale_requests(), 0, "{label}：链外身份不是陈旧请求");
        }
    }

    /// ★身份对「延伸」稳定：`CenterId` 只取 (si,zd,zg)——同一实例外缘 dd/gg 与 ei 随塔延伸
    /// 变化时仍判同一中枢（延伸不改核心；那不是误杀，不得拒）。
    #[test]
    fn center_id_ignores_envelope_and_end_index() {
        let c0 = center(5, 260, 100, 200);
        let mut m = machine_on_chain(&[c0]);
        // 塔侧同一中枢被延伸：ei 推进、外缘放大，核心 (zd,zg) 与出生坐标 si 不变。
        let extended = Center { zd: 100, zg: 200, dd: 50, gg: 300, start_index: 5, end_index: 4444 };
        assert_eq!(CenterId::of(&extended), CenterId::of(&c0), "延伸不改身份（核心不变）");
        let got = m.push_point(bits_3b(), 300, Some(CenterId::of(&extended)));
        assert!(
            matches!(got, Ok(PointOutcome::Event(CenterLifecycleEvent::Broken { .. }))),
            "延伸后的同一中枢 ⟹ 身份仍匹配，破坏放行"
        );
        assert_eq!(m.mis_kills(), 0);
        assert_eq!(m.stale_requests(), 0);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  ★R3 片四：链前缀分叉 ⟹ 工程重基（不伪造出生/死亡）
    // ──────────────────────────────────────────────────────────────────────

    /// ★链回缩（该级塔缓存全量重置）⟹ 重基：静默采纳当前链，游标落链尾，**不产 born/broken**。
    #[test]
    fn chain_shrink_triggers_rebase_without_fabricating_events() {
        let (c0, c1, c2) = (center(5, 260, 100, 200), center(300, 600, 150, 250), center(700, 900, 180, 280));
        let mut m = machine_on_chain(&[c0, c1, c2]);
        assert_eq!(m.chain_len(), 3);
        assert_eq!(m.counts(), (3, 0, 0));
        let got = m.consume_chain(&[c0, c1]);
        assert_eq!(
            got,
            ChainConsumed::Rebased { at: 2, len: 2 },
            "链回缩 ⟹ 重基（at = 首个缺失下标）"
        );
        assert_eq!(m.counts(), (3, 0, 0), "重基不伪造 born/broken");
        assert_eq!(m.chain_len(), 2);
        assert_eq!(m.alive_center(), Some((c1, 1)), "重基后场 = 当前链尾实例");
    }

    /// ★链尾重切（长度不变/增长但已消费末条身份被改写）⟹ 重基。核心 (zd,zg) 变即身份变
    /// （区别于延伸——延伸不改核心，见 [`center_id_ignores_envelope_and_end_index`]）。
    #[test]
    fn chain_tail_recut_triggers_rebase() {
        let (c0, c1) = (center(5, 260, 100, 200), center(300, 600, 150, 250));
        let mut m = machine_on_chain(&[c0, c1]);
        // c1 被重切成核心不同的另一个实例 c1'（塔 frontier 重划）。
        let c1_recut = center(300, 640, 160, 240);
        let got = m.consume_chain(&[c0, c1_recut]);
        assert_eq!(
            got,
            ChainConsumed::Rebased { at: 1, len: 2 },
            "已消费末条身份被改写 ⟹ 重基（at = 检出分叉的下标）"
        );
        assert_eq!(m.counts(), (2, 0, 0), "重基不伪造 born");
        assert_eq!(m.alive_center(), Some((c1_recut, 1)));
    }

    /// ★事件自带中枢身份（票面「破坏/重置事件携带中枢身份」）：下游（#292 减补动作）直读。
    #[test]
    fn events_expose_killed_center_id() {
        let c0 = center(5, 260, 100, 200);
        let born_ev = CenterLifecycleEvent::Born { level: 0, center: c0, chain_index: 0 };
        assert_eq!(born_ev.killed_center_id(), None, "出生不杀中枢 ⟹ 无死亡身份");

        let mut m = machine_on_chain(&[c0]);
        let broken = match m.push_point(bits_3b(), 100, Some(CenterId::of(&c0))) {
            Ok(PointOutcome::Event(ev)) => ev,
            other => panic!("应放行破坏，实得 {other:?}"),
        };
        assert_eq!(broken.killed_center_id(), Some(CenterId::of(&c0)), "破坏事件携带死亡身份");

        let mut m2 = machine_on_chain(&[c0]);
        let reset = match m2.push_point(bits_1s(), 200, Some(CenterId::of(&c0))) {
            Ok(PointOutcome::Event(ev)) => ev,
            other => panic!("应放行同死，实得 {other:?}"),
        };
        assert_eq!(reset.killed_center_id(), Some(CenterId::of(&c0)), "同死事件携带死亡身份");
    }
}
