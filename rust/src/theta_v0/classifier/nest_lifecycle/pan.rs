//! pan 活窗产出机与 L1 active C frontier（#1188 B01 职责块自 `nest_lifecycle.rs` 迁出，零行为）。

use super::key::side_tag;
use super::*;

/// pan 活窗产出机（卡 §3：中枢 + seg_a 定位沿用 `locate_pan_div_structure`——窄锚优先、
/// A′ 回退与 provider 同序同判（level_view.rs:826-839）；Extreme 预滤沿用
/// `pan_div_structure_extreme`，禁第二查法）。
///
/// 对每只 Consolidation 中枢取**末个**可定位离开段的结构锚，活窗 = `(seg_c.0, as_of)`
/// （c 窗右端 = prefix 边界，含行进中 bar）。本函数只做定位与产窗，不消费力度
/// （力度三值化在 advance 内现算）。#421 现由 `p123_fast_replay` 在生产每根 bar 调用
/// `feed_replay_bar`（活窗相先、完成相后）；本函数是其 `PanLiveWindow` 结构定位与初建的
/// 参照实装（T11/T14 真实夹具锚定）。
///
/// ★#523 根因登记 / 票 #527 分水岭：本函数的 C 取自**已完成** lower legs，与完成事件
/// provider 同源同判 ⟹ 它产的窗最早只能在完成候选也已可构造时出生（首见即完成）。
/// **生产 L1 活窗已改由 [`provide_active_pan_live_windows`] 从 active C frontier 产**；
/// 本函数保留为结构定位参照与 p409 反事实探针（`post-completion holding counterfactual`）
/// 的产窗口径，**不再**是生产「完成前活窗」的来源。
/// 可见性登记：保留 `pub`——本函数是交付「消费契约」的喂入参照（两轴评审发现项取
/// 登记分支）；`pub(crate)` 在非 test 构建无调用方会触发 dead_code 警告，违反零新增警告线。
pub fn provide_pan_live_windows(
    level: u32,
    centers: &[Center],
    kinds: &[Option<MoveKind>],
    segments: &[Segment],
    anchors_self: &[Option<Direction>],
    as_of: usize,
) -> Vec<PanLiveWindow> {
    // 同一身份（中枢/方向/seg_a/c_start）多只离开段可定位时末段覆盖——每身份活窗唯一；
    // BTreeMap 保序 ⟹ 产出确定性（v3 硬禁令）。
    let mut latest: BTreeMap<(usize, u8, (usize, usize), usize), PanLiveWindow> = BTreeMap::new();
    for segment in segments.iter().filter(|s| s.end_index <= as_of) {
        let Some(center_index) = nearest_confirmed_center_idx(centers, segment.start_index) else {
            continue;
        };
        if kinds.get(center_index) != Some(&Some(MoveKind::Consolidation)) {
            continue;
        }
        // ⚠#898 登记：本通道（探针/回放 sidecar，非准入门）的 `kinds` 入参未过滤
        // `level_lift`——扩展折出的高一级盘整块在此仍按 Consolidation 收。准入门
        // （signal.rs / level_view/pan_provider.rs）已按 lift==0 过滤；本通道若未来
        // 进准入路径，须先补 lift 过滤（须随调用方签名一并改，超出 #898 范围）。
        // #1230 裁定 a：先定位唯一 A（窄锚结构存在取窄锚，不存在才取 A′——061:28 中枢前最近同向段），
        // 再单判 Extreme 一次（判负即止，不换段重判）——与 provider pan 分支同序同判。
        let Some(structure) =
            locate_pan_div_structure(&centers[center_index], segment, segments, anchors_self)
                .or_else(|| {
                    locate_pan_div_structure_front_anchor(
                        &centers[center_index],
                        segment,
                        segments,
                        anchors_self,
                    )
                })
                .filter(|structure| pan_div_structure_extreme(structure, segments))
        else {
            continue;
        };
        latest.insert(
            (
                center_index,
                side_tag(structure.side),
                structure.seg_a,
                structure.seg_c.0,
            ),
            PanLiveWindow {
                level,
                side: structure.side,
                seg_a: structure.seg_a,
                // c 窗右端 = prefix 边界（含行进中 bar；卡 §3 设计内行为）。
                seg_c_live: (
                    structure.seg_c.0,
                    active_window_right_edge(structure.seg_c.0, as_of),
                ),
                b_center_start: centers[center_index].start_index,
                // 本函数的 C 恒取自已完成 segments（非 active frontier），无洞概念，恒 0。
                gap_len: 0,
            },
        );
    }
    latest.into_values().collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// L1 active C frontier（票 #527：完成前活窗可见）
// ═══════════════════════════════════════════════════════════════════════════

/// L1 的**行进中 C 段**（parser 未确认线段的当下状态；票 #527 / #523 §3 L1）。
///
/// 数据源纪律（票面永禁清单）：本载体只能由 parser 的 `tail`（`OpenTail.pendingSegment`）
/// + 其对应的未确认笔序列构造——**禁止**从 confirmed segments 回放重建。它表达
/// 「当下状态」（方向/起点/当前极值，parser/tail.rs 逐字口径），不预判该段将如何终结。
///
/// 与 confirmed 段的关系：`start_index` = pending 段首笔起点（= 上一 confirmed 段终点），
/// 一旦 parser 把该段 emit 进 `ParseLayer.segments`，本 frontier 即消失并由完成事件接手
/// （同一 `c_start` ⟹ 桥判同身份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSegmentFrontier {
    /// 未确认段方向（= 剩余笔首笔方向，第67课线段方向口径）。
    pub direction: Direction,
    /// 未确认段起点源坐标。
    pub start_index: usize,
    /// 未确认段起点价（剩余笔首笔起点价）。
    pub start_price: Tick,
    /// 段方向上当下触及的极值（与 `parser::tail` 的 `current_extreme` 同口径）。
    pub extreme: Tick,
    /// 该极值所在的**结构点**源坐标（极值所在笔端点；禁用 as_of 冒充结构点）。
    pub extreme_at: usize,
}

impl ActiveSegmentFrontier {
    /// 行进中 C 段的当下段形态（右端 = 极值结构点，不是 as_of——as_of 只进活窗右端）。
    pub fn as_segment(&self) -> Segment {
        Segment {
            direction: self.direction,
            start_index: self.start_index,
            end_index: self.extreme_at,
            start_price: self.start_price,
            end_price: self.extreme,
        }
    }
}

/// 从 parser 单层输出读出 L1 的 active C frontier（唯一构造点）。
///
/// 口径与 `parser::tail::pending_segment` 同源：pending 段 = `strokes[pending_start..]`，
/// 方向 = 首笔方向，极值 = 剩余笔在段方向上的极值。本函数额外解析**极值所在结构点**
/// （`current_extreme` 不携带坐标——tail 只存当下状态值），用于给行进中段一个真实右端。
///
/// 诚实边界：`tail` 无 `PendingSegment` / pending 首笔无法在 `strokes` 上定位 / 极值退化在
/// 段起点（该段尚未推进）⟹ `None`（无行进中 C 可定位，不造窗）。
pub fn active_segment_frontier(l0: &ParseLayer) -> Option<ActiveSegmentFrontier> {
    let (direction, start_index) = l0.tail.iter().find_map(|pending| match pending {
        PendingTail::PendingSegment {
            direction,
            start_index,
            ..
        } => Some((*direction, *start_index)),
        _ => None,
    })?;
    // pending 首笔定位（strokes 按 start_index 升序；坐标不等 ⟹ 定位失败，诚实 None）。
    let pending_start = l0.strokes.partition_point(|s| s.start_index < start_index);
    let rest = l0.strokes.get(pending_start..)?;
    let first = rest.first()?;
    if first.start_index != start_index {
        return None;
    }
    // 极值与其结构点：逐笔取两端，方向上取极值；平局保最早坐标（parser :16 同 discipline）。
    let mut extreme = first.start_price;
    let mut extreme_at = first.start_index;
    for stroke in rest {
        for (price, at) in [
            (stroke.start_price, stroke.start_index),
            (stroke.end_price, stroke.end_index),
        ] {
            let better = match direction {
                Direction::Up => price > extreme,
                Direction::Down => price < extreme,
            };
            if better {
                extreme = price;
                extreme_at = at;
            }
        }
    }
    if extreme_at <= start_index {
        return None; // 段起点即极值 ⟹ 行进中段尚未推进，无可定位的 C。
    }
    Some(ActiveSegmentFrontier {
        direction,
        start_index,
        start_price: first.start_price,
        extreme,
        extreme_at,
    })
}

/// 活窗产出机（票 #527 立、票 #601 泛化）：**confirmed A/B 锚 + active C 腿**。
///
/// 与 [`provide_pan_live_windows`] 的分水岭（#523 根因）：后者遍历**已完成** lower legs 找 C，
/// 故最早只能在完成候选也已可构造时出生（首见即完成）；本函数的 C **只能**是行进中的下级腿
/// （`active`），A 锚与 B 中枢仍取 confirmed 侧 ⟹ 活窗在完成前即可见。
///
/// **级别中立**（票 #601）：本函数从不读 `level` 以外的级别信息，判据全部作用在传入的
/// `centers`/`kinds`/`confirmed_segments`/`active` 四元组上——`level` 只随窗口带出，不参与判定。
/// 故 L1 与 L2 共用同一实装（禁第二查法）：
/// - **L1**：`active` = parser 行进中段（[`active_segment_frontier`] → [`ActiveSegmentFrontier::as_segment`]），
///   `confirmed_segments` = `tower[0]` 投影；
/// - **L2**：`active` = 行进中的 L1 窗口单元（[`active_l1_window_frontier`] →
///   [`ActiveWindowFrontier::as_segment`]），`confirmed_segments` = `tower[1]` 投影。
///
/// 调用方**必须**保证 `active` 是塔上尚不存在的行进中腿——从已完成 tower unit 外推会重演
/// #523 的同源恒等式（该守卫在两个 frontier 构造函数内，不在本函数内；本函数不校验来源）。
///
/// 结构判据一律复用同一套单一来源（禁第二查法）：`nearest_confirmed_center_idx` 取 B、
/// `locate_pan_div_structure`（窄锚）→ `locate_pan_div_structure_front_anchor`（A′ 回退）、
/// `pan_div_structure_extreme` 预滤——与完成事件 provider（level_view.rs pan 分支）同序同判。
///
/// **措辞限定**（票 #591 裁定）：「同序同判」指**同一输入序列 ⟹ 同一判定**——两路径调用的
/// `nearest_confirmed_center_idx` 是同一纯函数、逐字相同的切片构造算法。这**不**蕴含「任一
/// 时刻两路径的查询结果一致」：活窗路径按当前全局唯一 pending frontier 稀疏采样（仅在
/// `(forest_epoch, frontier)` 变化时重算），完成路径在结构已完全确认后对已固定的
/// `segment.start_index` 做一次性事后查询——中枢确认可落在活窗路径的稀疏重算盲区内，
/// 此时两路径在**同一 bar** 查询同一纯函数会因**入参切片不同步**（活窗侧尚未看见刚确认的
/// 中枢）而给出不同结果，这是时序差（无害），不是判据分歧（32893 现场分析）。
///
/// **c_start 稳定性**（#523 遗留问题 2；票 #559 条件 C2 订正——原文写成无限定的恒等断言，
/// BTC 100k 实测 5 例反例，故收窄为下述限定表述）：
///
/// 限定成立的是「**同一** C 段」上的恒等：活窗左端 = `structure.seg_c.0` = λ_C，由
/// `departure_move_c_start` 在 `[B.end_index, C.start_index]` 窗口上定界；该窗口只含衔接连续的
/// confirmed 段与行进中段自身（衔接由上面的守卫钉死），行进中段 emit 为 confirmed 后
/// `start_index`/`direction` 不变 ⟹ 同一 λ_C ⟹ **该 C 段的**完成事件与**该 C 段的**活窗
/// `seg_c.0` 恒等，桥（除右端外全等）判同身份。
///
/// **不成立的是跨 C 段的恒等**（反例来源）：完成事件的首次可见 bar 晚于 lower unit 物理完成
/// bar（BTC 100k 滞后中位 57.5、最大 5190）。若该滞后超过 C 段活窗的存活期，完成事件到账时
/// parser 的 pending 段早已换成**下一个** C ⟹ 当下活窗的 `seg_c.0` 与到账完成事件的
/// `seg_c.0` 属于两个不同的 C，桥不判同身份 ⟹ 旧活窗记
/// `IdentityVanished{ObservationSeam}`、完成事件另起闪现。这是**观测接缝**，不是 λ_C 不稳；
/// 两类成因的账本区分见 [`VanishCause`]（票 #559 裁定）。
///
/// 活窗右端 = `as_of`（卡 §3 设计内行为，随 bar 前进；不进身份键）。
///
/// 返回 [`PanLiveOutcome`]——**未产窗时给出可审计的原因码**（验收 1 第二分支要求「明确、
/// 可审计」；诊断只写不判，不进任何真值路径）。
pub fn provide_active_pan_live_windows(
    level: u32,
    centers: &[Center],
    kinds: &[Option<MoveKind>],
    confirmed_segments: &[Segment],
    active: Segment,
    as_of: usize,
) -> PanLiveOutcome {
    // 诊断字段（票 #592）：frontier 起点与最近 confirmed 段末端的洞长——与下方
    // `frontier_not_after_confirmed` 判定同源同值（同一个 `confirmed_segments.last()`）。
    // 无 confirmed 段（B 锚尚未建立）时无洞可定义，记 0；产窗必经 `NoConfirmedCenterBefore`
    // 分支拒绝，该 0 值不会流入下游 Window 载荷。
    let gap_len = confirmed_segments
        .last()
        .map_or(0, |last| active.start_index.saturating_sub(last.end_index));
    // 行进中段必须严格晚于全部 confirmed 段（否则不是 frontier ⟹ 拒绝，诚实空产出）。
    //
    // **仍只查 `<` 不查 `==`**（票 #578 复核，推翻本注记曾经的「`>` 分支生产不可达」断言）：
    // 票 #559 条件 C2 曾主张「parser 笔首尾相接 ⟹ 末段 end_index == frontier.start_index 恒真，
    // `>` 分支不可达」，并引用「BTC 100k 实测 frontier_gap 分布逐 bar 恒 0」为据——但该数字
    // **从未由任何真做 `!=`/`>` 判别的探针实际测过**：旧夹具坐标（段间 +1 不共端点）令 `==`
    // 守卫在测试里恒拒，从来没人能在不改夹具的前提下把 `>` 分支接上真实数据跑一遍。
    // 票 #578 把夹具坐标对齐生产共端点约定后，**首次**具备条件真做这个探针：改 `<`→`!=`
    // 编译通过、单测全绿后，用同一 BTC 数据跑 p123 20k bar 对拍，`frontier_not_after_confirmed`
    // 从 0 跳到 **210**（`window` 命中同时从 101 降到 58）——即 `active.start_index >
    // last.end_index`（有洞，非倒灌）在真实数据里频繁发生，C2 的「恒可达」断言是未经验证的
    // 声明膨胀（090 号语法禁令）。故**不收紧**本守卫：`<`→`!=` 会真实拒绝当下被接受的活窗，
    // 不是生产零行为变化，是否应该拒绝这些「有洞」frontier 需要教义/架构裁决，非本票 Scope
    // （见 §7 遗留 6）。
    if confirmed_segments
        .last()
        .is_some_and(|last| active.start_index < last.end_index)
    {
        return PanLiveOutcome::FrontierNotAfterConfirmed;
    }
    if active.end_index > as_of {
        return PanLiveOutcome::FrontierAheadOfClock; // 禁前视：结构点尚未到达当前 bar。
    }
    // 与完成时同构：行进中段并入段序列尾（λ_C/包络与完成后同一算式，见函数文档）。
    let mut segments = confirmed_segments.to_vec();
    segments.push(active);
    let anchors_self: Vec<Option<Direction>> = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    let Some(center_index) = nearest_confirmed_center_idx(centers, active.start_index) else {
        return PanLiveOutcome::NoConfirmedCenterBefore;
    };
    if kinds.get(center_index) != Some(&Some(MoveKind::Consolidation)) {
        return PanLiveOutcome::CenterNotConsolidation;
    }
    let Some(structure) =
        locate_pan_div_structure(&centers[center_index], &active, &segments, &anchors_self)
            .or_else(|| {
                locate_pan_div_structure_front_anchor(
                    &centers[center_index],
                    &active,
                    &segments,
                    &anchors_self,
                )
            })
            .filter(|structure| pan_div_structure_extreme(structure, &segments))
    else {
        return PanLiveOutcome::StructureNotLocatable;
    };
    PanLiveOutcome::Window(PanLiveWindow {
        level,
        side: structure.side,
        seg_a: structure.seg_a,
        seg_c_live: (
            structure.seg_c.0,
            active_window_right_edge(structure.seg_c.0, as_of),
        ),
        b_center_start: centers[center_index].start_index,
        gap_len,
    })
}

/// L1 活窗定位的结果与**未产窗原因码**（票 #527；诊断面，不进真值路径）。
///
/// 原因码回答「这只完成身份为什么没有更早的 Live」——闪现若非 true-flash，必须能落到
/// 其中某一码上，禁以「不知道」结账。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanLiveOutcome {
    /// 定位成功：行进中 C 的活窗。
    Window(PanLiveWindow),
    /// 行进中段与 confirmed 段序不自洽（frontier 起点落在末 confirmed 段内部）。
    FrontierNotAfterConfirmed,
    /// 行进中段的极值结构点尚未到达当前 bar（禁前视）。
    FrontierAheadOfClock,
    /// 行进中段之前没有任何已确认中枢（B 锚不可见）。
    NoConfirmedCenterBefore,
    /// 最近已确认中枢不属 Consolidation 块（盘整域外）。
    CenterNotConsolidation,
    /// 窄锚与 A′ 回退都无法定位 A/C 结构，或 Extreme 预滤未过（C 尚未破 A 极值）。
    StructureNotLocatable,
}

impl PanLiveOutcome {
    /// 原因码标签（dump/统计口径单一来源）。
    pub fn reason_tag(&self) -> &'static str {
        match self {
            Self::Window(_) => "window",
            Self::FrontierNotAfterConfirmed => "frontier_not_after_confirmed",
            Self::FrontierAheadOfClock => "frontier_ahead_of_clock",
            Self::NoConfirmedCenterBefore => "no_confirmed_center_before",
            Self::CenterNotConsolidation => "center_not_consolidation",
            Self::StructureNotLocatable => "structure_not_locatable",
        }
    }

    pub fn window(&self) -> Option<PanLiveWindow> {
        match self {
            Self::Window(window) => Some(*window),
            _ => None,
        }
    }
}
