//! #668（N4）事件↔BSP 稳定身份桥接对象（#666 四问四裁 + 2026-07-29 三轮 supersede 落地，
//! 修复轮 = #670 影子评审 FAIL 回炉，见 `chanlun/review-results/issue668-n4-fix-round1-20260729.md`）。
//!
//! ## 命名独立（#666 裁定①，同 #636/#540 先例）
//!
//! 老对象（[`super::cand_event::CandidateEvent`]/[`super::bsp::BspPoint`]）**不动**——本模块只
//! 新增一个关系户口：[`BspBridgeEdge`]。身份 = （N1 事件键，BSP 结构键）对（裁定②）：
//! - N1 事件键沿用 [`CandidateKey`]，不新造。
//! - BSP 结构键 = [`BspStructuralKey`]：v1（被破中枢指纹 + 方向 + 点类）被真值表证伪。
//!   v2 = v1 + **锚段坐标**——一类=背驰确认段（`seg_a`+`c_start`）、二类=一类锚身份（v1 退化，
//!   见 [`resolve_second_class_anchor`] 文档）、三类=离开段+回试段起点。
//!
//! ## 一类点身份 = episode（第三轮 supersede 裁定①，撤销「键唯一性」旧叙事）
//!
//! v2 锚（`seg_a`+`c_start`）标定的是**这一次破中枢的候选事件本身**（episode），不是某个孤立
//! 物理点。同一 episode 内可能存在**多个**物理一类点（多段递进背驰，教义必然，非 bug）——这些
//! 物理点是**同一候选身份的修订史**：`source_index`/pivot 是修订**载荷**，不入身份分量（与
//! [`BspBridgeEdge::bsp_source_index`] 头纪律一致）。「键唯一性」不是本对象的验收目标；「episode
//! 归属唯一」才是（一个一类点只能属于一个 episode——[`find_episode`] 的 `debug_assert`
//! 机器化此不变量，评审 #670 §五复核探针 300k 窗 `points_in_multiple_episodes=0` 实证）。
//!
//! ## 双向产出，主路径内联（裁定③）
//!
//! 「内联」= 直接消费 `classify_impl` 已经就地产出的两条主路径事实——BSP 六 bit
//! （[`super::LevelState::bsp`]）与候选事件流（[`CandidateStreams`]，`cand_event.rs` 头注「本模块
//! 只产出、存储候选生命史」同一条），**不重新扫描/重新判定**任何结构；不是外挂 sidecar 事后
//! 补扫（旧 `otherwise_domain_sidecar` 反面教材）。「双向」= 两个方向各有独立的、真实的遍历入口
//! （非同一遍历改名两次）：
//! - **事件侧 → 一类点**（[`resolve_first_class_episode_edges`]）：遍历 Trend 候选事件，对每个
//!   episode 回挂其区间 `[c_start, interval.1]` 覆盖的全部一类点（同 level/side/中枢指纹）——
//!   判据从「本点 `source_index` 恰好等于候选当前右端」改为「本点落在候选 episode 区间内」
//!   （右端会随 `as_of` 生长，[`super::cand_event`] 称为 `growth_revision`；区间左端 `c_start`
//!   已闭合不再增长，纪律同 [`CandidateKey`]）。
//! - **点侧 → 事件**（二/三类，[`resolve_bridge`]）：遍历 BSP 点，反查其所属候选事件——三类点的
//!   「离开段」= 同一 C 段候选事件（不要求背驰确认——`judge_third_cert` 的 `leave_seg` 与 N1
//!   候选扫描共用同一 `first_structural_gates`/`nearest_confirmed_center_idx` 定位）；二类点 =
//!   其一类锚（`OwnerRef::Type1Anchor`）所属 episode（同样按区间覆盖反查，见
//!   [`resolve_second_class_anchor`] 文档）。
//!
//! 「查簿命中才写」（N3 `extends_lineage_key` 先例）：找不到对应 N1 事件 ⟹ 不产边（[`BspBridgeBook::advance`]
//! 直接跳过该点，不写占位/不写「缺失」记录）——**Absent（查无）非证伪**（裁定⑤），与
//! [`BridgeStatus::Invalidated`]（N1 事件转 `Invalidated` 的正信号）严格区分。
//!
//! ## 端死边死（裁定⑤）+ append-only（裁定③ E2E-O 全套）
//!
//! 边的活死完全派生自 N1 事件侧状态（BSP 侧无独立状态机——一个 bit 一旦在某 `as_of` 置位即为
//! 冻结事实，`buy1`/`sell1` 严格背驰语义 + #551「一次写入不后移」保证不倒退）：事件转
//! `Invalidated` ⟹ 边转 [`BridgeStatus::Invalidated`]，留痕（revision 保留）、不复活（终态后
//! `advance` 直接跳过，同 `ChainCertificateBook::apply` 终态挡）。同一 `as_of` 重跑：投影
//! （[`BridgeProjection`]）未变 ⟹ 零 Delta（幂等跳过，不空转 append）。
//!
//! ## 零消费接线（裁定④，本票不动）
//!
//! p92 电池（`p92_nest_replay_postruling.rs`）与 π runner（`backtest/runner.rs`）的
//! `source_index` 拼缝本模块零触碰、零调用——本模块唯一调用点 = 单测 + 本票诊断/验收（对拍
//! cmp=0）。消费并轨归 N7（`chanlun/review-results/e2eo-lineage-synthesis-20260728.md`）。

use std::collections::BTreeMap;

use super::bsp::OwnerRef;
use super::cand_event::{CandidateEvent, CandidateKey, CandidateKind, CandidateState, CandidateStreams, ParentFingerprint};
use super::{Classification, LevelState};
use super::super::types::Side;

/// 桥接判定规则版本（E2E-O「上游身份或规则版本变 ⟹ 新 key，禁复活旧 key」的版本分量，
/// 与 [`super::cand_event::CANDIDATE_RULE_VERSION`]/[`super::chain_cert::CHAIN_RULE_VERSION`]
/// 同精神独立计数——三者各自度量各自的判定规则，互不联动）。
pub const BRIDGE_RULE_VERSION: u32 = 1;

/// BSP 点类：六 bit 之一（一个 `BspPoint` 可能同时贡献多把键，如 2B/3B 共存，`bsp.rs`
/// `no_exclusive_trichotomy` 已证非互斥）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BspPointClass {
    Buy1,
    Buy2,
    Buy3,
    Sell1,
    Sell2,
    Sell3,
}

impl BspPointClass {
    pub fn side(self) -> Side {
        match self {
            BspPointClass::Buy1 | BspPointClass::Buy2 | BspPointClass::Buy3 => Side::Long,
            BspPointClass::Sell1 | BspPointClass::Sell2 | BspPointClass::Sell3 => Side::Short,
        }
    }

    /// 本点置位的全部点类，按 buy1/buy2/buy3/sell1/sell2/sell3 固定序（与 `class_index` 权重序一致）。
    fn set_classes(bits: &super::super::types::BspBits) -> Vec<BspPointClass> {
        [
            (bits.buy1, BspPointClass::Buy1),
            (bits.buy2, BspPointClass::Buy2),
            (bits.buy3, BspPointClass::Buy3),
            (bits.sell1, BspPointClass::Sell1),
            (bits.sell2, BspPointClass::Sell2),
            (bits.sell3, BspPointClass::Sell3),
        ]
        .into_iter()
        .filter_map(|(set, class)| set.then_some(class))
        .collect()
    }
}

/// BSP 结构身份键 v2（#666 裁定② + 2026-07-29 supersede）。
///
/// ★「右端/as_of 不入键」纪律（与 [`CandidateKey`]「C 右端与 as_of 均不在键中」同一纪律）：
/// `anchor` 只含**已闭合、不再随 as_of 增长**的历史段坐标——本点自身所在段的右端恒等于该点的
/// `source_index`（一/三类的 C 段/回试段右端），塞进锚会让键对 `source_index` 平凡单射、
/// 真值表恒判「唯一」（methodologically 空洞），故 `source_index` 不入 `anchor`（也不入本键任何
/// 分量）——查询/去重时按 `(level, source_index)` 另行核对（[`BspBridgeEdge::bsp_source_index`]/
/// [`BspBridgeEdge::bsp_level`]，二者是**载荷**，不是身份分量）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BspStructuralKey {
    pub rule_version: u32,
    pub level: u32,
    pub parent: ParentFingerprint,
    pub side: Side,
    pub class: BspPointClass,
    /// 锚段坐标，按点类语义不同长度（一类=`[seg_a, (c_start,c_start)]`；三类=
    /// `[leave_interval, (retest_start,retest_start)]`；二类=`[(anchor_idx,anchor_idx)]`，
    /// 诚实退化为 v1，见模块头 [`resolve_second_class_anchor`] 引用）。
    pub anchor: Vec<(usize, usize)>,
}

/// 桥接身份：（N1 事件键，BSP 结构键）对（#666 裁定②）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BridgeKey {
    pub event: CandidateKey,
    pub bsp: BspStructuralKey,
}

/// 桥接边活死二态（#666 裁定⑤：端死边死）。无「Closed」——桥接边不像链证书那样有「不可再
/// 扩展」的终局语义，它只回答「这个 N1 事件与这个 BSP 点的配对现在还成立吗」，成立 = `Open`，
/// N1 侧转 `Invalidated` = 边同步终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BridgeStatus {
    Open,
    Invalidated,
}

impl BridgeStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Invalidated)
    }
}

/// 一等桥接对象的一次不可变 revision（append-only，`chain_cert::TowerChainCertificate` 同构）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BspBridgeEdge {
    pub key: BridgeKey,
    /// 本边所指 BSP 点的级别（载荷，非身份分量——见 [`BspStructuralKey`] 头纪律）。
    pub bsp_level: u32,
    /// 本边所指 BSP 点的 `source_index`（载荷，非身份分量）。查询/对拍入口。
    pub bsp_source_index: usize,
    /// N1 事件侧最新状态（边的活死单一来源）。
    pub event_state: CandidateState,
    pub status: BridgeStatus,
    /// 边首次入簿时的 `as_of`（一次写入不后移）。
    pub observed_at: usize,
    /// 首次进入 `Invalidated` 的 `as_of`（终态，一次写入）。
    pub invalidated_at: Option<usize>,
    pub revision: u32,
    pub supersedes_revision: Option<u32>,
    pub revision_at: usize,
}

/// [`BspBridgeEdge`] 的业务载荷投影——「边是什么」的唯一比较口径（不含生命史记账三只钟 +
/// revision 计数，同 `chain_cert::ChainProjection`/`cand_event::CandidateProjection` 精神：
/// 同一份载荷在不同 `as_of` 重跑必带不同钟，计入等价比较会让「载荷未变」永远判不成立）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct BridgeProjection {
    bsp_level: u32,
    bsp_source_index: usize,
    event_state: CandidateState,
    status: BridgeStatus,
}

impl BspBridgeEdge {
    fn projection(&self) -> BridgeProjection {
        BridgeProjection {
            bsp_level: self.bsp_level,
            bsp_source_index: self.bsp_source_index,
            event_state: self.event_state,
            status: self.status,
        }
    }
}

struct BridgeObservation {
    key: BridgeKey,
    bsp_level: u32,
    bsp_source_index: usize,
    event_state: CandidateState,
    status: BridgeStatus,
}

/// 按 key 取最新 revision（`cand_sub::latest_by_level`/`chain_cert::AliveIndex::build` 同方法学，
/// 纯折叠非新判据）——**不按状态过滤**（含 `Invalidated`），供本模块判断事件侧终态。
fn latest_candidates(streams: &CandidateStreams) -> BTreeMap<CandidateKey, CandidateEvent> {
    let mut latest = BTreeMap::new();
    for batch in streams.iter() {
        for event in batch.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    latest
}

/// Trend 域事件反查索引：`(level, side, parent, interval.1) → CandidateKey`。**仅供三类点**
/// （离开段右端是三类点自身已闭合的历史坐标，不随本 episode 后续生长而漂移——三类离开时刻
/// 记录的 `leave_interval.1` 是过去时；见 [`resolve_bridge`] 三类分支）。一类/二类改走
/// [`find_episode`]（评审 #670 HIGH-1/HIGH-2：右端等值对一类点是错判据——它会随 `as_of` 生长，
/// `cand_event` 称为 `growth_revision`，用它做一类点的配对键会漏掉同一 episode 内右端生长之前
/// 就已确认的物理点）。
///
/// 静默覆盖留痕（LOW-2）：同 `(level, side, parent, interval.1)` 理论上不应有两个 Trend 候选
/// （interval.1 在教义上标定「这一次」破中枢的因果触发坐标），`debug_assert` 机器化此假设，
/// 复核探针 300k 窗 `idx_all_interval_end_collisions=0` 实证当前无实害。
fn trend_index_by_interval_end(
    latest: &BTreeMap<CandidateKey, CandidateEvent>,
) -> BTreeMap<(u32, Side, ParentFingerprint, usize), CandidateKey> {
    let mut index = BTreeMap::new();
    for event in latest.values() {
        if event.kind != CandidateKind::Trend {
            continue;
        }
        let idx_key = (event.event_level, event.key.side, event.key.parent, event.interval.1);
        let prior = index.insert(idx_key, event.key);
        debug_assert!(
            prior.is_none() || prior == Some(event.key),
            "trend_index_by_interval_end 静默覆盖：{idx_key:?} 已有 {prior:?}，被 {:?} 覆盖",
            event.key,
        );
    }
    index
}

/// 一次「破中枢」episode 的区间身份：候选 [`CandidateKey`]（含 `seg_a`/`c_start`，已闭合、
/// 不随 `as_of` 生长）+ 当前区间右端（会生长，只用作区间上界，不入身份，见模块头「一类点身份
/// = episode」段）。
struct TrendEpisode {
    key: CandidateKey,
    level: u32,
    side: Side,
    parent: ParentFingerprint,
    c_start: usize,
    interval_end: usize,
}

fn trend_episodes(latest: &BTreeMap<CandidateKey, CandidateEvent>) -> Vec<TrendEpisode> {
    latest
        .values()
        .filter(|event| event.kind == CandidateKind::Trend)
        .map(|event| TrendEpisode {
            key: event.key,
            level: event.event_level,
            side: event.key.side,
            parent: event.key.parent,
            c_start: event.key.c_start,
            interval_end: event.interval.1,
        })
        .collect()
}

/// episode 区间覆盖反查（评审 #670 HIGH-1/HIGH-2 修复核心）：给定一个坐标，找它落在哪个
/// Trend episode 的 `[c_start, interval_end]` 闭区间内（同 level/side/parent）。
///
/// `debug_assert` 机器化「episode 归属唯一」（评审 #670 §五复核探针 300k 窗
/// `points_in_multiple_episodes=0` 实证；100k 窗同）——若失守，说明该不变量在新数据上不再成立，
/// 按 dispatch「若仍有撞键，停手上报」处置，不静默择一。
fn find_episode<'a>(
    episodes: &'a [TrendEpisode],
    level: u32,
    side: Side,
    parent: ParentFingerprint,
    source_index: usize,
) -> Option<&'a TrendEpisode> {
    let mut hits = episodes
        .iter()
        .filter(|ep| ep.level == level && ep.side == side && ep.parent == parent)
        .filter(|ep| ep.c_start <= source_index && source_index <= ep.interval_end);
    let first = hits.next()?;
    debug_assert!(
        hits.next().is_none(),
        "episode 归属应唯一：source_index={source_index} 在 level={level} side={side:?} \
         parent={parent:?} 下落入多个 episode——评审 #670 复核探针未观测到此路径，需上报"
    );
    Some(first)
}

/// 一类/三类点的破/离中枢指纹（`OwnerRef::Center` 载体，`make_first_point`/`make_third_point`
/// 恒填）。
fn center_fingerprint(level: &LevelState, idx_in_level: usize) -> Option<ParentFingerprint> {
    match level.bsp[idx_in_level].center {
        Some(OwnerRef::Center(c)) => Some(ParentFingerprint {
            center_start: c.start_index,
            zd: c.zd,
            zg: c.zg,
        }),
        _ => None,
    }
}

/// 二类点的一类锚（`OwnerRef::Type1Anchor`）：反查同级 `source_index==anchor` 且对应
/// 买/卖 1 位为真的点，取其中枢指纹与自身坐标。
///
/// ★v2 诚实退化为 v1（模块头 supersede 引用）：回抽段坐标 structurally 不可得——段右端按
/// 「右端不入键」纪律排除（即本点自身 `source_index`），段左端（`RMove` 次级别走势起点）在
/// `extract_second_signals` 入参层已坐标剥离（signal.rs 注释「坐标 still-MISSING」），生产
/// 路径只把回拉走势*终点*经 `index_of` 传出；反查需重跑 `find_second_type_structure`（新判定
/// 路径，违反本对象「不重算既有判据」的方法学）。故 `anchor` 只含一类锚坐标一段，如实登记，
/// 不冒充真实回抽段坐标（issue668 v2 真值表已证：BTC 三窗在此退化下仍 `ambiguous_keys=0`）。
fn resolve_second_class_anchor(
    level: &LevelState,
    class: BspPointClass,
    idx_in_level: usize,
) -> Option<(ParentFingerprint, usize)> {
    let anchor_idx = match level.bsp[idx_in_level].center {
        Some(OwnerRef::Type1Anchor(idx)) => idx,
        _ => return None,
    };
    let want_buy1 = matches!(class, BspPointClass::Buy2);
    level.bsp.iter().find_map(|p| {
        if p.source_index != anchor_idx {
            return None;
        }
        let hit = if want_buy1 { p.bits.buy1 } else { p.bits.sell1 };
        if !hit {
            return None;
        }
        match p.center {
            Some(OwnerRef::Center(c)) => Some((
                ParentFingerprint {
                    center_start: c.start_index,
                    zd: c.zd,
                    zg: c.zg,
                },
                anchor_idx,
            )),
            _ => None,
        }
    })
}

/// 单点单 bit（二/三类）的 BSP 结构键 + 对应 N1 事件键（找不到 N1 事件 ⟹ `None`，模块头
/// 「查簿命中才写」）。**一类不走本函数**——一类是事件侧驱动，见
/// [`resolve_first_class_episode_edges`]（模块头「双向产出」段）。
fn resolve_bridge(
    level_idx: u32,
    level: &LevelState,
    idx_in_level: usize,
    class: BspPointClass,
    trend_index: &BTreeMap<(u32, Side, ParentFingerprint, usize), CandidateKey>,
    episodes: &[TrendEpisode],
) -> Option<(BspStructuralKey, CandidateKey)> {
    let point = &level.bsp[idx_in_level];
    let side = class.side();
    match class {
        BspPointClass::Buy1 | BspPointClass::Sell1 => {
            unreachable!("一类由 resolve_first_class_episode_edges 事件侧产出，不经本函数")
        }
        BspPointClass::Buy3 | BspPointClass::Sell3 => {
            let parent = center_fingerprint(level, idx_in_level)?;
            let entry = point.bits.third_class_entry?;
            let event_key = *trend_index.get(&(level_idx, side, parent, entry.leave_interval.1))?;
            let anchor = vec![
                entry.leave_interval,
                (entry.retest_interval.0, entry.retest_interval.0),
            ];
            Some((
                BspStructuralKey { rule_version: BRIDGE_RULE_VERSION, level: level_idx, parent, side, class, anchor },
                event_key,
            ))
        }
        BspPointClass::Buy2 | BspPointClass::Sell2 => {
            let (parent, anchor_idx) = resolve_second_class_anchor(level, class, idx_in_level)?;
            // episode 区间覆盖反查（HIGH-2 修复：exact right-end 会在同 episode 后续生长后漏判
            // 一类锚，二类继承同一失效风险，改用与一类同款判据）。
            let episode = find_episode(episodes, level_idx, side, parent, anchor_idx)?;
            let anchor = vec![(anchor_idx, anchor_idx)];
            Some((
                BspStructuralKey { rule_version: BRIDGE_RULE_VERSION, level: level_idx, parent, side, class, anchor },
                episode.key,
            ))
        }
    }
}

/// 一类点的 BSP 结构键（episode 锚，不含任何物理点自身坐标——模块头「一类点身份 = episode」）。
fn first_class_structural_key(episode: &TrendEpisode, class: BspPointClass) -> BspStructuralKey {
    BspStructuralKey {
        rule_version: BRIDGE_RULE_VERSION,
        level: episode.level,
        parent: episode.parent,
        side: episode.side,
        class,
        anchor: vec![episode.key.seg_a, (episode.key.c_start, episode.key.c_start)],
    }
}

/// 事件侧 → 一类点（HIGH-2 修复：裁定③「双向产出」缺的那一向）。遍历 Trend episode，对每个
/// episode 回挂其区间 `[c_start, interval_end]` 覆盖的全部一类点（同 level/side/中枢指纹）。
///
/// 同一 episode 若覆盖多个物理一类点（多段递进背驰，教义必然——见模块头「一类点身份 =
/// episode」），按 `source_index` 升序产出多条观察，全部共享同一 [`BridgeKey`]：
/// `BspBridgeBook::apply`（同 `as_of` 内顺序 apply）会把它们折叠成一条**修订链**——较早的物理点
/// 先落 revision 0，后续物理点依次追加 revision，链头（`latest_of`）落在**最后一个**（即
/// `source_index` 最大、语义上「目前所见最新递进」的物理点）。这不是「键碰撞被强行去重」，
/// 是 dispatch 第三轮 supersede 裁定①明文授权的语义：pivot/右端是修订载荷，不是身份分量。
fn resolve_first_class_episode_edges(
    classification: &Classification,
    episodes: &[TrendEpisode],
    latest: &BTreeMap<CandidateKey, CandidateEvent>,
) -> Vec<BridgeObservation> {
    let mut observations = Vec::new();
    for episode in episodes {
        let Some(level) = classification.levels.get(episode.level as usize) else {
            continue;
        };
        let class = match episode.side {
            Side::Long => BspPointClass::Buy1,
            Side::Short => BspPointClass::Sell1,
        };
        let mut covered: Vec<(usize, usize)> = level
            .bsp
            .iter()
            .enumerate()
            .filter(|(idx, point)| {
                let hit = match episode.side {
                    Side::Long => point.bits.buy1,
                    Side::Short => point.bits.sell1,
                };
                hit && center_fingerprint(level, *idx) == Some(episode.parent)
                    && episode.c_start <= point.source_index
                    && point.source_index <= episode.interval_end
            })
            .map(|(idx, point)| (idx, point.source_index))
            .collect();
        covered.sort_by_key(|(_, source_index)| *source_index);

        let bsp_key = first_class_structural_key(episode, class);
        let event_state = latest[&episode.key].state;
        let status = if event_state == CandidateState::Invalidated {
            BridgeStatus::Invalidated
        } else {
            BridgeStatus::Open
        };
        for (_, source_index) in covered {
            observations.push(BridgeObservation {
                key: BridgeKey { event: episode.key, bsp: bsp_key.clone() },
                bsp_level: episode.level,
                bsp_source_index: source_index,
                event_state,
                status,
            });
        }
    }
    observations
}

/// 单次扫描：一类走事件侧驱动（[`resolve_first_class_episode_edges`]），二/三类走点侧驱动
/// （[`resolve_bridge`]）——两个方向各自真实遍历，找不到 N1 事件的点诚实跳过（Absent）。
fn observe(classification: &Classification, streams: &CandidateStreams) -> Vec<BridgeObservation> {
    let latest = latest_candidates(streams);
    let trend_index = trend_index_by_interval_end(&latest);
    let episodes = trend_episodes(&latest);

    let mut observations = resolve_first_class_episode_edges(classification, &episodes, &latest);

    for (level_idx, level) in classification.levels.iter().enumerate() {
        for idx_in_level in 0..level.bsp.len() {
            for class in BspPointClass::set_classes(&level.bsp[idx_in_level].bits) {
                if matches!(class, BspPointClass::Buy1 | BspPointClass::Sell1) {
                    continue; // 已由事件侧驱动产出，见上。
                }
                let Some((bsp_key, event_key)) = resolve_bridge(
                    level_idx as u32,
                    level,
                    idx_in_level,
                    class,
                    &trend_index,
                    &episodes,
                ) else {
                    continue;
                };
                // event_key 保证在 latest 中存在（trend_index/episodes 均由 latest 折叠而来）。
                let event_state = latest[&event_key].state;
                let status = if event_state == CandidateState::Invalidated {
                    BridgeStatus::Invalidated
                } else {
                    BridgeStatus::Open
                };
                observations.push(BridgeObservation {
                    key: BridgeKey { event: event_key, bsp: bsp_key },
                    bsp_level: level_idx as u32,
                    bsp_source_index: level.bsp[idx_in_level].source_index,
                    event_state,
                    status,
                });
            }
        }
    }
    observations
}

/// 桥接对象 append-only 修订簿。终态不复活；同一 `as_of` 重跑零 Delta（#666 裁定③ E2E-O 全套）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BspBridgeBook {
    edges: Vec<BspBridgeEdge>,
    latest: BTreeMap<BridgeKey, usize>,
}

impl BspBridgeBook {
    /// 全部 revision，落簿序（append-only，旧 revision 永不改写、永不删除）。
    pub fn edges(&self) -> &[BspBridgeEdge] {
        &self.edges
    }

    /// 按 [`BridgeKey`] 取最新 revision。
    pub fn latest_of(&self, key: &BridgeKey) -> Option<&BspBridgeEdge> {
        self.latest.get(key).map(|index| &self.edges[*index])
    }

    /// 每 key 最新 revision，key 升序。
    pub fn heads(&self) -> Vec<&BspBridgeEdge> {
        self.latest.values().map(|index| &self.edges[*index]).collect()
    }

    /// 查询入口（#666 裁定⑦ 验收对拍用）：给定 BSP 点坐标，返回其全部现存边（各点类各一条，
    /// 查无 N1 事件的点不出现——Absent 非证伪，见模块头）。
    pub fn edges_for_bsp_point(&self, level: u32, source_index: usize) -> Vec<&BspBridgeEdge> {
        self.heads()
            .into_iter()
            .filter(|edge| edge.bsp_level == level && edge.bsp_source_index == source_index)
            .collect()
    }

    /// 用当前 `(Classification, CandidateStreams)` 推进一步，返回本次追加的 Delta。
    ///
    /// 与 `ChainCertificateBook::advance` 同构，但**不需要 `resident` 补集**：BSP 点一旦在
    /// 某 `as_of` 置位即为冻结事实（append-only 单调，见模块头「端死边死」段），故每次全量重扫
    /// `classification` 天然覆盖此前观察过的全部点；唯一会变化的是 N1 事件侧状态（在 `latest`
    /// 折叠里天然读到最新值），无需额外携带簿内旧 key 补扫。
    pub fn advance(&mut self, classification: &Classification, streams: &CandidateStreams, as_of: usize) -> Vec<BspBridgeEdge> {
        let mut delta = Vec::new();
        for observation in observe(classification, streams) {
            if let Some(edge) = self.apply(observation, as_of) {
                delta.push(edge);
            }
        }
        delta
    }

    fn apply(&mut self, observation: BridgeObservation, as_of: usize) -> Option<BspBridgeEdge> {
        let prior = self.latest_of(&observation.key).cloned();
        if prior.as_ref().is_some_and(|edge| edge.status.is_terminal()) {
            return None; // 终态不复活。
        }
        let next = make_revision(prior.as_ref(), observation, as_of);
        if prior.as_ref().is_some_and(|edge| edge.projection() == next.projection()) {
            return None; // 同 as_of 重跑零 Delta（幂等跳过）。
        }
        self.append(next.clone());
        Some(next)
    }

    fn append(&mut self, edge: BspBridgeEdge) {
        let position = self.edges.len();
        self.latest.insert(edge.key.clone(), position);
        self.edges.push(edge);
    }
}

fn make_revision(prior: Option<&BspBridgeEdge>, observation: BridgeObservation, as_of: usize) -> BspBridgeEdge {
    // observed_at / invalidated_at 均为「一次写入不后移」：已在案的值优先。
    let observed_at = prior.map_or(as_of, |edge| edge.observed_at);
    let invalidated_at = prior
        .and_then(|edge| edge.invalidated_at)
        .or((observation.status == BridgeStatus::Invalidated).then_some(as_of));
    BspBridgeEdge {
        key: observation.key,
        bsp_level: observation.bsp_level,
        bsp_source_index: observation.bsp_source_index,
        event_state: observation.event_state,
        status: observation.status,
        observed_at,
        invalidated_at,
        revision: prior.map_or(0, |edge| edge.revision + 1),
        supersedes_revision: prior.map(|edge| edge.revision),
        revision_at: as_of,
    }
}

#[cfg(test)]
mod tests;
