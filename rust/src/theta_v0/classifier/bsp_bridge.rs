//! #668（N4）事件↔BSP 稳定身份桥接对象（#666 四问四裁 + 2026-07-29 三轮 supersede 落地，
//! 修复轮 1 = #670 影子评审 FAIL 回炉，见 `chanlun/review-results/issue668-n4-fix-round1-20260729.md`；
//! 修复轮 2 = #670 影子评审第 2 轮 FAIL 回炉（两条新 HIGH），见
//! `chanlun/review-results/issue668-n4-fix-round2-20260729.md` + ADR「第四轮 supersede」段；
//! 修复轮 3 = #670 影子评审第 2 轮遗留（R2-HIGH-3/R2-MED-2/R2-MED-3），见
//! `chanlun/review-results/issue668-n4-fix-round3-20260729.md` + ADR「第五轮 supersede」段）。
//!
//! ## 命名独立（#666 裁定①，同 #636/#540 先例）
//!
//! 老对象（[`super::cand_event::CandidateEvent`]/[`super::bsp::BspPoint`]）**不动**——本模块只
//! 新增一个关系户口：[`BspBridgeEdge`]。身份 = （N1 事件键，BSP 结构键）对（裁定②）：
//! - N1 事件键沿用 [`CandidateKey`]，不新造。
//! - BSP 结构键 = [`BspStructuralKey`]：v1（被破中枢指纹 + 方向 + 点类）被真值表证伪。
//!   v2 = v1 + **锚段坐标**——一类=背驰确认段（`seg_a`+`c_start`）、二类=一类锚身份（v1 退化，
//!   见 [`resolve_second_class_anchor`] 文档）、三类=离开段+回试段起点（第四轮 supersede：判据
//!   迁移到与一类同构的 episode 区间覆盖，见 [`resolve_bridge`] 三类分支）。
//!
//! ## 一类/三类点身份 = episode（第三轮+第四轮 supersede，撤销「键唯一性」旧叙事）
//!
//! v2 锚（一类=`seg_a`+`c_start`）标定的是**这一次破中枢的候选事件本身**（episode），不是某个
//! 孤立物理点。同一 episode 内可能存在**多个**物理点共享同一 [`BridgeKey`]（一类=多段递进背驰、
//! 三类=同一离开段的多次回试，教义必然，非 bug）——这些物理点是**同一候选身份的修订史**：
//! `source_index`/pivot 是修订**载荷**，不入身份分量。
//!
//! **载荷形态（第四轮 supersede，修复 #670 R2-HIGH-1/R2-HIGH-2）**：一次 `observe()` 扫描先按
//! 物理点产出原始观察（[`RawPointObservation`]），再**按 [`BridgeKey`] 分组折叠**成一条
//! [`BridgeObservation`]——同 key 的全部物理点合并为一个有序去重的 `source_index` 集合
//! （[`BspBridgeEdge::bsp_source_indices`]）。这修掉了修复轮 1 的根因：旧实现把「同 episode k 个
//! 并存物理点」映射成「对同一个 key 顺序 `apply` k 次」，而 `apply` 的幂等/终态判据假定**每次
//! observe 每个 key 只产一条观察**——k 次 apply 会（a）无条件重复 append 已在案的点（幂等破，
//! 300k 窗每次重跑 +12 revision）、（b）终态挡在处理到第 2 个物理点时已提前生效（因为第 1 个
//! 物理点已把该 key 判「terminal」），导致失效只在遍历序首个物理点上留痕、其余物理点及查询入口
//! （[`BspBridgeBook::edges_for_bsp_point`]）永久丢失。折叠成一条观察后，`apply` 每次 observe
//! 每个 key 恰好被调用一次，幂等/终态挡的「prior vs next 二元比较」前提重新成立。
//!
//! 「episode 归属唯一」（一个物理点只能落入一个 episode）——[`find_episode`] 的 `debug_assert`
//! 机器化此不变量。**订正（第五轮 supersede，修复 #670 R2-HIGH-3）**：此前一类路径
//! （[`resolve_first_class_episode_points`]）绕过 [`find_episode`] 自行内联同款过滤，判据
//! 重复但机器保证覆盖不到——若一类点同时落入两个 episode，旧实现会静默产两条边、零信号。
//! 现改为逐点调用 [`find_episode`]，一/二/三类三条路径至此才真正共用同一个带 `debug_assert`
//! 的查找函数（此前「共用同一函数」是文书断言先于实现，见
//! `chanlun/review-results/shadow-668-review2-20260729.md` R2-HIGH-3）。
//!
//! ## 双向产出，主路径内联（裁定③）
//!
//! 「内联」= 直接消费 `classify_impl` 已经就地产出的两条主路径事实——BSP 六 bit
//! （[`super::LevelState::bsp`]）与候选事件流（[`CandidateStreams`]，`cand_event.rs` 头注「本模块
//! 只产出、存储候选生命史」同一条），**不重新扫描/重新判定**任何结构；不是外挂 sidecar 事后
//! 补扫（旧 `otherwise_domain_sidecar` 反面教材）。「双向」= 两个方向各有独立的、真实的遍历入口
//! （非同一遍历改名两次）：
//! - **事件侧 → 一类点**（[`resolve_first_class_episode_points`]）：遍历 Trend 候选事件，对每个
//!   episode 回挂其区间 `[c_start, interval.1]` 覆盖的全部一类点（同 level/side/中枢指纹）——
//!   判据从「本点 `source_index` 恰好等于候选当前右端」改为「本点落在候选 episode 区间内」
//!   （右端会随 `as_of` 生长，[`super::cand_event`] 称为 `growth_revision`；区间左端 `c_start`
//!   已闭合不再增长，纪律同 [`CandidateKey`]）。
//! - **点侧 → 事件**（二/三类，[`resolve_bridge`]）：遍历 BSP 点，反查其所属候选事件——三类点的
//!   「离开段」= 同一 C 段候选事件（不要求背驰确认——`judge_third_cert` 的 `leave_seg` 与 N1
//!   候选扫描共用同一 `first_structural_gates`/`nearest_confirmed_center_idx` 定位），配对判据
//!   同一类一样按 episode 区间覆盖反查 `leave_interval.1`（第四轮 supersede：替换旧的
//!   `leave_interval.1` 精确等值——精确等值在 Trend 候选 `growth_revision` 后会漏判，同一类
//!   HIGH-2 的失效模式）；二类点 = 其一类锚（`OwnerRef::Type1Anchor`）所属 episode（同样按区间
//!   覆盖反查，见 [`resolve_second_class_anchor`] 文档）。
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
//! （[`BridgeProjection`]）未变 ⟹ 零 Delta（幂等跳过，不空转 append）——第四轮 supersede 后此
//! 承诺覆盖多物理点 episode（原先只对单点 episode 成立）。
//!
//! ## 零消费接线（裁定④，本票不动）
//!
//! p92 电池（`p92_nest_replay_postruling.rs`）与 π runner（`backtest/runner.rs`）的
//! `source_index` 拼缝本模块零触碰、零调用——本模块唯一调用点 = 单测 + 本票诊断/验收（对拍
//! cmp=0）。消费并轨归 N7（`chanlun/review-results/e2eo-lineage-synthesis-20260728.md`）。

use std::collections::{BTreeMap, BTreeSet};

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
/// 分量）——查询/去重时按 `(level, source_index)` 另行核对（[`BspBridgeEdge::bsp_source_indices`]/
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
    /// 本 revision 观察到的、共享本 [`BridgeKey`] 的全部物理 BSP 点 `source_index`，升序去重
    /// （载荷，非身份分量。第四轮 supersede：单点 `bsp_source_index: usize` 升为集合——同 episode
    /// 并存多物理点是并发载荷，不是需要顺序 `apply` 多次的时间序修订，见模块头「载荷形态」段）。
    /// 二/三类单点场景退化为单元素集合。
    pub bsp_source_indices: Vec<usize>,
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

impl BspBridgeEdge {
    /// 链头物理点：集合内最大 `source_index`（目前所见最新递进点；单点集合退化为该点自身）。
    pub fn head_source_index(&self) -> usize {
        *self
            .bsp_source_indices
            .last()
            .expect("覆盖点集非空：observe() 从不产出空集合观察（见 observe() 分组逻辑）")
    }
}

/// [`BspBridgeEdge`] 的业务载荷投影——「边是什么」的唯一比较口径（不含生命史记账三只钟 +
/// revision 计数，同 `chain_cert::ChainProjection`/`cand_event::CandidateProjection` 精神：
/// 同一份载荷在不同 `as_of` 重跑必带不同钟，计入等价比较会让「载荷未变」永远判不成立）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct BridgeProjection {
    bsp_level: u32,
    bsp_source_indices: Vec<usize>,
    event_state: CandidateState,
    status: BridgeStatus,
}

impl BspBridgeEdge {
    fn projection(&self) -> BridgeProjection {
        BridgeProjection {
            bsp_level: self.bsp_level,
            bsp_source_indices: self.bsp_source_indices.clone(),
            event_state: self.event_state,
            status: self.status,
        }
    }
}

/// 单个物理点对某 [`BridgeKey`] 的一次原始观察（分组折叠前）——一类（事件侧驱动）、二/三类
/// （点侧驱动）三条路径的共同产出形状，由 `observe()` 统一按 key 折叠为 [`BridgeObservation`]。
struct RawPointObservation {
    key: BridgeKey,
    bsp_level: u32,
    source_index: usize,
    event_state: CandidateState,
    status: BridgeStatus,
}

/// 折叠后的一条观察：一个 [`BridgeKey`] 本次 observe 命中的全部物理点集合（第四轮 supersede
/// 核心——见模块头「载荷形态」段）。
struct BridgeObservation {
    key: BridgeKey,
    bsp_level: u32,
    bsp_source_indices: Vec<usize>,
    event_state: CandidateState,
    status: BridgeStatus,
}

fn event_status(state: CandidateState) -> BridgeStatus {
    if state == CandidateState::Invalidated {
        BridgeStatus::Invalidated
    } else {
        BridgeStatus::Open
    }
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
/// 按 dispatch「若仍有撞键，停手上报」处置，不静默择一。**唯一调用点**（第五轮 supersede 前）
/// 曾只有二/三类分支（[`resolve_bridge`]）——一类分支（[`resolve_first_class_episode_points`]）
/// 内联了同款过滤但绕过本函数，机器保证因此覆盖不到检验域里唯一有真实数据的点类（评审 #670
/// R2-HIGH-3）。第五轮 supersede 后一/二/三类三条路径均经本函数反查，`debug_assert` 覆盖面
/// 与生产判据覆盖面终于重合。
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
/// 不冒充真实回抽段坐标。
/// ~~issue668 v2 真值表已证：BTC 三窗在此退化下仍 `ambiguous_keys=0`。~~
/// **订正（R2-LOW-1，2026-07-29）**：该读数出自已撤回的「键唯一性」旧口径检验，现行真值表
/// bin（`ISSUE668_TRUTH_V3`）不再产出 `ambiguous_keys` 这个字段；且二类在生产数据上 0/280
/// 命中（`resolve_second_class_anchor` 的第二个条件——反查锚点自身是否持有一类 bit——恒假，
/// 见 `chanlun/review-results/shadow-668-review2-20260729.md` R2-MED-2），该正面断言当前无
/// 检验域支撑，未撤回时按未实测处置，本行不再作为「已证」引用。
///
/// **第五轮 supersede（R2-MED-2 处置，停手照实上报）**：下方的「反查同级 `source_index==anchor`
/// 且持有一类 bit 的点」*已经就是* dispatch 点名的「结构可查的那条路径」——本函数改动前后判据
/// 逻辑不变。三窗分解数据（`p_issue668_bsp_key_truth` 新增 `ISSUE668_MED2_ANCHOR_BREAKDOWN`）：
/// 条件一「本点携 `Type1Anchor`」100% 满足（20k:17/17、100k:88/88、300k:280/280）；条件二
/// 「该锚坐标确有点持有一类 bit」三窗均 **0**——即 `Type1Anchor(type1_src)` 载的是该走势 m1
/// 终点坐标（`signal.rs` `extract_second_signals`），而 m1 终点要过背驰确认门才是一类点，二者
/// 生产上不重合，结构性不可得，非本函数实现错误。按 dispatch「若生产数据上结构性不可得，停手
/// 照实上报，禁发明坐标、禁放宽判据假装修复」处置：本函数**不改动**，二类锚归属仍恒判 `None`；
/// 是否属候选域结构性错位（归 #688）还是二类键公式本身选错了锚，交编排者另裁。
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
/// [`resolve_first_class_episode_points`]（模块头「双向产出」段）。
fn resolve_bridge(
    level_idx: u32,
    level: &LevelState,
    idx_in_level: usize,
    class: BspPointClass,
    episodes: &[TrendEpisode],
) -> Option<(BspStructuralKey, CandidateKey)> {
    let point = &level.bsp[idx_in_level];
    let side = class.side();
    match class {
        BspPointClass::Buy1 | BspPointClass::Sell1 => {
            unreachable!("一类由 resolve_first_class_episode_points 事件侧产出，不经本函数")
        }
        BspPointClass::Buy3 | BspPointClass::Sell3 => {
            let parent = center_fingerprint(level, idx_in_level)?;
            let entry = point.bits.third_class_entry?;
            // episode 区间覆盖反查（第四轮 supersede，MED 修复：与一类同构，替换旧的
            // `leave_interval.1` 精确等值——精确等值在 Trend 候选 `growth_revision` 后会漏判
            // （离开段自身坐标不变，但索引键改按当前 `interval.1` 建，二者错位），同一类
            // HIGH-2 的失效模式）。
            let episode = find_episode(episodes, level_idx, side, parent, entry.leave_interval.1)?;
            let anchor = vec![
                entry.leave_interval,
                (entry.retest_interval.0, entry.retest_interval.0),
            ];
            Some((
                BspStructuralKey { rule_version: BRIDGE_RULE_VERSION, level: level_idx, parent, side, class, anchor },
                episode.key,
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

/// 事件侧 → 一类点原始观察（未折叠，见 `observe()` 分组）。按物理点遍历（第五轮 supersede，
/// 修复 #670 R2-HIGH-3：旧实现按 episode 外层遍历、点内层过滤，判据与 [`find_episode`] 重复
/// 但绕过它，`find_episode` 的「episode 归属唯一」`debug_assert` 因此覆盖不到一类路径——一个
/// 一类点若同时落在两个 episode 区间内，旧实现会安静产出两条边，零机器信号）。改为逐点调用
/// [`find_episode`] 反查其所属 episode，与二/三类（[`resolve_bridge`]）共享同一个附带
/// `debug_assert` 的查找函数，一/二/三类三条路径至此才真正共用同一机器保证（模块头「episode
/// 归属唯一」段此前的文书断言与实现不符，本次修复令其名实相符）。
fn resolve_first_class_episode_points(
    classification: &Classification,
    episodes: &[TrendEpisode],
    latest: &BTreeMap<CandidateKey, CandidateEvent>,
) -> Vec<RawPointObservation> {
    let mut raw = Vec::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for (idx, point) in level.bsp.iter().enumerate() {
            for (side, class, hit) in [
                (Side::Long, BspPointClass::Buy1, point.bits.buy1),
                (Side::Short, BspPointClass::Sell1, point.bits.sell1),
            ] {
                if !hit {
                    continue;
                }
                let Some(parent) = center_fingerprint(level, idx) else {
                    continue;
                };
                let Some(episode) =
                    find_episode(episodes, level_idx as u32, side, parent, point.source_index)
                else {
                    continue;
                };
                let bsp_key = first_class_structural_key(episode, class);
                let event_state = latest[&episode.key].state;
                raw.push(RawPointObservation {
                    key: BridgeKey { event: episode.key, bsp: bsp_key },
                    bsp_level: level_idx as u32,
                    source_index: point.source_index,
                    event_state,
                    status: event_status(event_state),
                });
            }
        }
    }
    raw
}

/// 点侧 → 事件原始观察（二/三类，未折叠）：遍历 BSP 点，反查其所属候选事件——一类不走本函数
/// （事件侧驱动，见 [`resolve_first_class_episode_points`]）。
fn resolve_point_driven_observations(
    classification: &Classification,
    episodes: &[TrendEpisode],
    latest: &BTreeMap<CandidateKey, CandidateEvent>,
) -> Vec<RawPointObservation> {
    let mut raw = Vec::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for idx_in_level in 0..level.bsp.len() {
            for class in BspPointClass::set_classes(&level.bsp[idx_in_level].bits) {
                if matches!(class, BspPointClass::Buy1 | BspPointClass::Sell1) {
                    continue; // 已由事件侧驱动产出，见上。
                }
                let Some((bsp_key, event_key)) =
                    resolve_bridge(level_idx as u32, level, idx_in_level, class, episodes)
                else {
                    continue;
                };
                // event_key 保证在 latest 中存在（episodes 由 latest 折叠而来）。
                let event_state = latest[&event_key].state;
                raw.push(RawPointObservation {
                    key: BridgeKey { event: event_key, bsp: bsp_key },
                    bsp_level: level_idx as u32,
                    source_index: level.bsp[idx_in_level].source_index,
                    event_state,
                    status: event_status(event_state),
                });
            }
        }
    }
    raw
}

/// 单次扫描：一类走事件侧驱动（[`resolve_first_class_episode_points`]），二/三类走点侧驱动
/// （[`resolve_point_driven_observations`]）——两个方向各自真实遍历，找不到 N1 事件的点诚实
/// 跳过（Absent）。**按 [`BridgeKey`] 分组折叠**（第四轮 supersede 核心，模块头「载荷形态」段）：
/// 同一 key 命中的全部物理点合并为一条 [`BridgeObservation`]，保证每个 key 每次 `observe()`
/// 恰好产出一条观察——`BspBridgeBook::apply` 的幂等/终态判据依赖这条前提。
fn observe(classification: &Classification, streams: &CandidateStreams) -> Vec<BridgeObservation> {
    let latest = latest_candidates(streams);
    let episodes = trend_episodes(&latest);

    let mut raw = resolve_first_class_episode_points(classification, &episodes, &latest);
    raw.extend(resolve_point_driven_observations(classification, &episodes, &latest));

    let mut grouped: BTreeMap<BridgeKey, (u32, BTreeSet<usize>, CandidateState, BridgeStatus)> = BTreeMap::new();
    for point in raw {
        let RawPointObservation { key, bsp_level, source_index, event_state, status } = point;
        let entry = grouped
            .entry(key)
            .or_insert_with(|| (bsp_level, BTreeSet::new(), event_state, status));
        entry.1.insert(source_index);
    }

    grouped
        .into_iter()
        .map(|(key, (bsp_level, points, event_state, status))| BridgeObservation {
            key,
            bsp_level,
            bsp_source_indices: points.into_iter().collect(),
            event_state,
            status,
        })
        .collect()
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
    /// 查无 N1 事件的点不出现——Absent 非证伪，见模块头）。第四轮 supersede（R2-HIGH-2 修复）：
    /// 按 `bsp_source_indices` 集合成员判断，而非单值相等——同 episode 内非链头的物理点此前
    /// 会从这个入口静默消失（300k 窗实测 7/29 点查无），集合语义下同一条边覆盖的全部物理点
    /// 均可查得。
    pub fn edges_for_bsp_point(&self, level: u32, source_index: usize) -> Vec<&BspBridgeEdge> {
        self.heads()
            .into_iter()
            .filter(|edge| edge.bsp_level == level && edge.bsp_source_indices.contains(&source_index))
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
        bsp_source_indices: observation.bsp_source_indices,
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
