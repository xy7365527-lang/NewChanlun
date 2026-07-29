//! C2 typed provider 入口：`provide_nest_candidate_events*`/`provide_nest_candidate_events_ext*`
//!（#630 从 `level_view.rs` 拆出，纯移动零语义；来源票 #497 影子评审 MEDIUM-1）。
//! run 语境身份/memo 基础设施见 `level_view_pan.rs`。

use super::super::types::{Bar, Direction, Fractal, MoveKind, Side, Tick};
use super::decompose::{center_block_kind, MoveBlock};
use super::divergence::{move_range_envelope as range_envelope, segments_diverge_or};
use super::level_view::LevelAsOfView;
use super::level_view_confirm::{NestCandidateEvent, NestDivergenceKind};
use super::level_view_pan::{
    pan_block_triple, pan_owner_block_index, structural_block_span, structural_pair_span,
    PanCenterIdentity, PanEventCore, PanMemoKey, PanMemoValue, PanResidence, PanSegmentIdentity,
};
use super::level_view_projection::{leg_as_segment, ExactThreeProjection, LowerLeg};
use super::recursive_tower::map_src_to_close_idx;
use super::signal::{
    locate_pan_div_structure, locate_pan_div_structure_front_anchor, nearest_confirmed_center_idx,
    pan_div_structure_extreme,
};

/// #92 typed provider：把 strict C2 pair 映射为宽结构 Cand，并把力度确认留在独立字段。
///
/// Trend 与 Consolidation 共用同一输出类型但保留 `kind`；后者来自纯结构
/// [`super::signal::PanDivStructure`]，不会经 `PanDivCert` 的 Weak 门提前征税。
///
/// ★R1/R2/R3（2026-07-17 代理裁定）：Trend 分支 `divergence_confirmed` 改全合取扫描
/// （[`trend_confirm_time`]：T4 回拉0轴 ∧ T3 三买 ∧ T2 破极值 ∧ T5 力度或关系，确认时点 =
/// 首个全成立时点 t*，confirmed 事件的 `interval_b`/`turn_source` 收束到 t*）；Consolidation
/// 分支 A 锚加 R3 front-anchor 回退（061:28 中枢前最近同向段）、Weak 改 R2 力度或关系
/// （027:32：同色面积 ∨ 黄白线 ∨ 柱高）。
///
/// 事件视图（返回类型不含锚 sidecar）：T1 (#170) 锚供给传空集——锚载体解析为
/// `None`，不进事件本体/等值键/排序（事件集与 ext 形态逐字节同）。需锚的登记管道走
/// [`provide_nest_candidate_events_ext`] 并传真实包含层/分型供给。
#[allow(clippy::too_many_arguments)]
pub fn provide_nest_candidate_events(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Vec<NestCandidateEvent> {
    provide_nest_candidate_events_resident(
        level, projection, blocks, legs, view, hist, dif, close_src, None,
    )
}

/// #69 5b resident 事件视图入口。当前与旧入口共用同一 provider 核；显式 `None`
/// 是 shadow/legacy 的真冷 oracle。
#[allow(clippy::too_many_arguments)]
pub fn provide_nest_candidate_events_resident(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    residence: Option<PanResidence<'_>>,
) -> Vec<NestCandidateEvent> {
    provide_nest_candidate_events_ext_resident(
        level,
        projection,
        blocks,
        legs,
        view,
        hist,
        dif,
        close_src,
        &[],
        &[],
        residence,
    )
    .into_iter()
    .map(|ext| ext.event)
    .collect()
}

/// 事件 + T1 (#170) 锚 sidecar：事件本体不变（[`NestCandidateEvent`] 口径不动）。
///
/// T1（#170 键域重锚）锚 sidecar（`extreme_price` + `group_anchor`）：由 provider 在
/// 事件构造点经 [`resolve_triple_anchor`] 单一查法解析（gate 不二次推导，禁第二查法）。
/// `None` = 分型/包含层供给未命中（诚实缺锚——登记侧跳过新键域并入 `n_anchor_misses`
/// 计数，事件本体登记不受影响）。
/// T5a（#207 方向退役，ADR 20260723 裁定 1）：身份锚自三元组（方向, 极值价, 组锚）
/// 简化为**两元（极值价, 组锚）**——方向不参与身份；`event.side` 仍存事件本体供交易层
/// （入场裁决/Xzd 回退），不经本 sidecar 进身份键。
/// T5b（#208）：旧 `seg_c_full` 值桥载体（收束前全离开段坐标，供出场侧 `by_end`
/// 固定桥键）已删——#206 Q3 判删（出场迁链后生产侧写孤无读者；事件集/等值/排序
/// 历来不消费该载体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestCandidateEventExt {
    pub event: NestCandidateEvent,
    /// T1 (#170) 锚之**极值价**：拐点 x（= 离开段终点）处分型的 `Fractal.price`
    /// （整数 tick，精确等值无容差——v3 硬禁令）。跨级不变量：x 的源序号与级别无关
    /// （compose 端点 = 末子端点 = … = L0 线段端 = 笔尾分型中K），同一 x 在任意级别
    /// 查到同一分型 ⟹ 极值价逐值相同（跨型共点不产生价格二义，T5a 方向退役的依据）。
    pub extreme_price: Option<Tick>,
    /// T1 (#170) 锚之**组锚@ℓ**：x 所在合并组的锚（= 组内首根序号），该级包含层
    /// 单一来源（[`super::super::parser::inclusion::merged_group_anchor`]）。
    pub group_anchor: Option<usize>,
}

/// T1 (#170) 锚解析（事件构造点单一查法）：拐点 x（离开段终点源序号）→
/// （极值价, 组锚）。T5a (#207)：锚为两元——方向不参与身份（ADR 20260723 裁定 1）。
///
/// - **极值价** = x 处 L0 分型的 `Fractal.price`（分型管单一来源
///   [`super::super::parser::fractal::fractal_at_source`]）。**不设分型 kind 对偶守卫**：
///   高级别走势可以次级别反向段收束（compose 窗口延伸吸收的末单元方向可与本级行程相反），
///   此时 x 在 L0 是反向分型——但 x 处实际打印价（= 末次级别段端价 = 该分型价）仍是跨级
///   不变量（同一 x 在任意级别查到同一分型同一价；同点跨型各级自为真，T5a 方向退役
///   正是建立在这一价格上不变之上）。
/// - **组锚** = x 所在合并组首根序号（该级包含层单一来源
///   [`super::super::parser::inclusion::merged_group_anchor`]）。
/// 分型供给未命中（x 处无 confirmed 分型）⟹ 极值价 `None`（诚实缺锚，禁降级第二查法）。
pub(super) fn resolve_triple_anchor(
    x: usize,
    fractals: &[Fractal],
    merged_bars: &[Bar],
) -> (Option<Tick>, Option<usize>) {
    let price = super::super::parser::fractal::fractal_at_source(fractals, x).map(|f| f.price);
    let anchor = super::super::parser::inclusion::merged_group_anchor(merged_bars, x);
    (price, anchor)
}

/// A/C source 闭区间到 MACD 前缀的完整映射。仅“有交集”不足以写 memo：
/// source 尾尚未到达或 hist/dif 未覆盖映射终点时返回 `None`。
fn complete_pan_span(
    close_src: &[usize],
    hist: &[f64],
    dif: &[f64],
    span: (usize, usize),
) -> Option<(usize, usize)> {
    if close_src.first().copied()? > span.0 || close_src.last().copied()? < span.1 {
        return None;
    }
    let mapped = map_src_to_close_idx(close_src, span.0, span.1)?;
    (mapped.1 < hist.len() && mapped.1 < dif.len()).then_some(mapped)
}

fn materialize_pan_event(
    level: u32,
    core: PanEventCore,
    view: &LevelAsOfView,
    fractals: &[Fractal],
    merged_bars: &[Bar],
) -> NestCandidateEventExt {
    let event = NestCandidateEvent {
        level,
        side: core.structure.side,
        kind: NestDivergenceKind::Consolidation,
        seg_a: core.structure.seg_a,
        interval_b: core.structure.seg_c,
        interval_a: core.interval_a,
        divergence_confirmed: core.divergence_confirmed,
        turn_source: core.structure.source_index,
        judge_at: view.query.as_of,
        provider_window: (
            view.query.coordinate_window.start,
            view.query.coordinate_window.end,
        ),
        intake_fallback: core.intake_fallback,
        b_center_start: core.structure.center.start_index,
    };
    let (extreme_price, group_anchor) =
        resolve_triple_anchor(core.structure.seg_c.1, fractals, merged_bars);
    NestCandidateEventExt {
        event,
        extreme_price,
        group_anchor,
    }
}

/// [`provide_nest_candidate_events`] 的 ext 形态（同一扫描核，事件集/排序逐字节同；
/// 仅额外携带 T1 (#170) 锚 sidecar（T5a 起两元：极值价, 组锚））。消费方：gate 派生
/// （`derive_level_events`）。
///
/// `fractals`/`merged_bars`：两元锚供给（分型管 + 该级包含层，均为单一来源——
/// [`resolve_triple_anchor`]，事件构造点唯一查法，gate 不二次推导）。
#[allow(clippy::too_many_arguments)]
pub fn provide_nest_candidate_events_ext(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    fractals: &[Fractal],
    merged_bars: &[Bar],
) -> Vec<NestCandidateEventExt> {
    provide_nest_candidate_events_ext_resident(
        level,
        projection,
        blocks,
        legs,
        view,
        hist,
        dif,
        close_src,
        fractals,
        merged_bars,
        None,
    )
}

/// #69 5b resident ext 入口；trend 分支保持冷核，只有 pan 分支可消费显式 run memo。
#[allow(clippy::too_many_arguments)]
pub fn provide_nest_candidate_events_ext_resident(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    fractals: &[Fractal],
    merged_bars: &[Bar],
    residence: Option<PanResidence<'_>>,
) -> Vec<NestCandidateEventExt> {
    let mut residence = residence;
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let anchors_self: Vec<_> = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    let mut out = Vec::new();

    for pair in &view.pairs {
        let Some(leave_index) = blocks.iter().position(|block| {
            block.start_center == pair.id.block_start_center
                && block.end_center == pair.id.block_end_center
                && block.kind == MoveKind::Trend
                && block.dir == Some(pair.id.direction)
        }) else {
            continue;
        };
        let Some(interval_a) = structural_pair_span(projection, blocks, leave_index) else {
            continue;
        };
        let side = match pair.id.direction {
            Direction::Down => Side::Long,
            Direction::Up => Side::Short,
        };
        // Cand 的 Extreme 预滤（037:20）：seg_c = 全离开段（R1）的包络破 b 包络——宽窗口预滤，
        // 精确 T2（[c_start, t*] 窗口）在 trend_confirm_time 内随扫描判定。
        let (Some(a), Some(c)) = (
            range_envelope(&segments, pair.seg_a),
            range_envelope(&segments, pair.seg_c),
        ) else {
            continue;
        };
        let extreme = match side {
            Side::Long => c.0 < a.0,
            Side::Short => c.1 > a.1,
        };
        if !extreme {
            continue;
        }
        // ★R1：全合取确认（T4 回拉0轴 ∧ T3 三买 ∧ T2 破极值 ∧ T5 力度或关系）。
        // confirmed ⟹ 确认时点 t* = 首个全成立时点，interval_b/turn_source 收束到 [c_start, t*]
        // （禁前视：在 t* 之前全合取不成立，坐标不得早于确认时点）；
        // 未确认 ⟹ 保持全离开段结构坐标（诚实结构，力度/三买未成立）。
        let last_center = &projection.seeds[pair.id.block_end_center].center;
        let confirm_t = view
            .pair_confirmations
            .iter()
            .find(|confirmation| confirmation.pair_id == pair.id)
            .and_then(|confirmation| confirmation.state.as_option());
        let (interval_b, turn_source, divergence_confirmed) = match confirm_t {
            Some(t) => ((pair.seg_c.0, t), t, true),
            None => (pair.seg_c, pair.seg_c.1, false),
        };
        // 票 #427 等价性加锁（裁定 #402 题二）：活假设账本的身份键 `seg_c_full` 走工程桥取
        // `interval_b`，其与已退役的 `seg_c_full` 字段**仅差右端**，而桥判同（`bridge_identity`）
        // **只比左端** ⟹ 「维持工程桥而不复活字段」这一裁定完全建立在「收束只截右端、左端恒等」
        // 上。此前提一旦被数据源改动打破，等价性会**静默失效**（桥不再判同 ⟹ 同一身份被记成
        // 「身份消失 + 新建仓」，而所有既有测试照绿）。故在唯一写入点钉死，且**非 debug 门控**
        // ——release 构建同样执行（影子评审「debug-only 断言不算行为护栏」判据）。
        assert_eq!(
            interval_b.0, pair.seg_c.0,
            "左端恒等：确认收束只许截右端（票 #427 / 裁定 #402 题二等价性前提）"
        );
        // T1 (#170)：两元锚（极值价, 组锚）在事件构造点解析（单一查法；方向分量
        // 已随 T5a (#207) 退役，#206 Q1 裁定）。
        let (extreme_price, group_anchor) =
            resolve_triple_anchor(pair.seg_c.1, fractals, merged_bars);
        out.push(NestCandidateEventExt {
            event: NestCandidateEvent {
                level,
                side,
                kind: NestDivergenceKind::Trend,
                seg_a: pair.seg_a,
                interval_b,
                interval_a,
                divergence_confirmed,
                turn_source,
                judge_at: view.query.as_of,
                provider_window: (
                    view.query.coordinate_window.start,
                    view.query.coordinate_window.end,
                ),
                intake_fallback: false,
                // 关③ P3：B = 被离开的最后中枢（`block_end_center` seed，trend_confirm_time 同一
                // 中枢入参）——prefix 首次观察快照写入，延伸不改写 start_index。
                b_center_start: last_center.start_index,
            },
            // 值桥载体 seg_c_full 已随 T5b (#208) 删除（#206 Q3 判删）。
            extreme_price,
            group_anchor,
        });
    }

    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let kinds = center_block_kind(centers.len(), blocks);
    let provider_window = (
        view.query.coordinate_window.start,
        view.query.coordinate_window.end,
    );
    if let Some(residence) = residence.as_mut() {
        residence.memo.prepare(
            level,
            provider_window,
            projection.version,
            projection,
            blocks,
            &segments,
            &centers,
            residence.freeze_boundary_src,
        );
    }
    for (segment_index, segment) in segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| segment.end_index <= view.query.as_of)
    {
        let Some(center_index) = nearest_confirmed_center_idx(&centers, segment.start_index) else {
            continue;
        };
        if kinds.get(center_index) != Some(&Some(MoveKind::Consolidation)) {
            continue;
        }
        let Some(leave_index) = pan_owner_block_index(blocks, center_index) else {
            continue;
        };
        let memo_key =
            pan_block_triple(projection, blocks, leave_index).map(|block_triple| PanMemoKey {
                level,
                provider_window,
                projection_version: projection.version,
                segment_index,
                segment: PanSegmentIdentity::from(segment),
                center_index,
                center: PanCenterIdentity::from(&centers[center_index]),
                block_index: leave_index,
                blocks: block_triple,
            });
        let segment_is_stable = residence.as_ref().is_some_and(|residence| {
            segments[..=segment_index]
                .iter()
                .all(|read| read.end_index < residence.freeze_boundary_src)
                && centers[center_index].end_index < residence.freeze_boundary_src
        });
        let cached = if segment_is_stable {
            memo_key.and_then(|memo_key| {
                residence
                    .as_mut()
                    .and_then(|residence| residence.memo.lookup(&memo_key))
            })
        } else {
            None
        };
        if let Some(value) = cached {
            match value {
                PanMemoValue::NoEvent => continue,
                PanMemoValue::Event(core) => {
                    let ext = materialize_pan_event(level, core, view, fractals, merged_bars);
                    if !out.iter().any(|candidate| candidate.event == ext.event) {
                        out.push(ext);
                    }
                    continue;
                }
            }
        }
        // ★R3：窄锚（中枢后前次同向破核心段）locate∧Extreme 优先；任一失败回退 A′（中枢前
        // 最近同向段，061:28 中枢两头比较，回中枢要件由中枢本身满足）重判 Extreme（044:234 维持）。
        let Some(structure) =
            locate_pan_div_structure(&centers[center_index], segment, &segments, &anchors_self)
                .filter(|structure| pan_div_structure_extreme(structure, &segments))
                .or_else(|| {
                    locate_pan_div_structure_front_anchor(
                        &centers[center_index],
                        segment,
                        &segments,
                        &anchors_self,
                    )
                    .filter(|structure| pan_div_structure_extreme(structure, &segments))
                })
        else {
            if let Some(memo_key) = memo_key.filter(|_| segment_is_stable) {
                residence
                    .as_mut()
                    .expect("stable 资格来自 resident")
                    .memo
                    .insert(
                        memo_key,
                        segment.end_index.max(centers[center_index].end_index),
                        segment_index + 1,
                        PanMemoValue::NoEvent,
                    );
            }
            continue;
        };
        // #97 进料口（⑤「盘背入链」落地缺口补齐）：leave→retest 对不可用（如盘整块为末块、
        // 离开块未 Completed）时不再丢弃候选；interval_a 仅供 A 口径诊断，回填为盘整块自身
        // 结构跨度（再兜底 seg_a.0..seg_c.1），B 生产口径（interval_b）不受影响。
        let (interval_a, intake_fallback) =
            match structural_pair_span(projection, blocks, leave_index) {
                Some(span) => (span, false),
                None => (
                    blocks
                        .get(leave_index)
                        .and_then(|block| structural_block_span(projection, block))
                        .unwrap_or((structure.seg_a.0, structure.seg_c.1)),
                    true,
                ),
            };
        // ★R2：力度或关系（027:32「只要其中一个符合就可以」）——同色柱面积 ∨ 黄白线峰 ∨
        // 同向柱峰，替代旧混合柱面积单通道必要门（p113 实测 30.4% 聋度）。
        let mapped_spans = complete_pan_span(close_src, hist, dif, structure.seg_a)
            .zip(complete_pan_span(close_src, hist, dif, structure.seg_c));
        let divergence_confirmed = mapped_spans
            .map(|(a, c)| segments_diverge_or(hist, dif, structure.side, a, c))
            .unwrap_or(false);
        let core = PanEventCore {
            structure,
            interval_a,
            intake_fallback,
            divergence_confirmed,
        };
        let ext = materialize_pan_event(level, core, view, fractals, merged_bars);
        let read_end_src = segment
            .end_index
            .max(centers[center_index].end_index)
            .max(structure.seg_a.1)
            .max(structure.seg_c.1);
        if segment_is_stable
            && memo_key.is_some()
            && read_end_src
                < residence
                    .as_ref()
                    .expect("stable 资格来自 resident")
                    .freeze_boundary_src
            && mapped_spans.is_some()
        {
            residence
                .as_mut()
                .expect("stable 资格来自 resident")
                .memo
                .insert(
                    memo_key.expect("链②资格已核"),
                    read_end_src,
                    segment_index + 1,
                    PanMemoValue::Event(core),
                );
        }
        // 去重键 = 事件本体（与旧 `out.contains(&event)` 逐字同语义；锚 sidecar 不进键）。
        if !out.iter().any(|candidate| candidate.event == ext.event) {
            out.push(ext);
        }
    }
    out.sort_by_key(|ext| {
        (
            ext.event.turn_source,
            ext.event.interval_b,
            ext.event.kind,
            matches!(ext.event.side, Side::Short),
        )
    });
    out
}
