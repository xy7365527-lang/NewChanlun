//! C2 趋势背驰确认核心：`ConfirmKey`/`ConfirmResidence` run 语境键 + 全合取扫描引擎
//!（#630 从 `level_view.rs` 拆出，纯移动零语义；来源票 #497 影子评审 MEDIUM-1）。

use super::super::super::types::{Center, Direction, Segment, Side, Tick};
use super::{C2VersionTuple, CoordinateWindow, LevelViewQuery};
use super::super::level_view_store::{ConfirmCursor, ConfirmCursorStore, ConfirmState};
use super::super::divergence::{
    dif_crosses_zero, move_range_envelope as range_envelope, same_color_area,
    same_dir_hist_peak, segment_dif_peak,
};
use super::super::recursive_tower::map_src_to_close_idx;
use super::super::signal::trend_third_class_in_c;

#[cfg(test)]
std::thread_local! {
    static CONFIRM_CORE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_confirm_core_calls() {
    CONFIRM_CORE_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
pub(super) fn confirm_core_calls() -> usize {
    CONFIRM_CORE_CALLS.with(std::cell::Cell::get)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DivergencePairId {
    pub level: u32,
    pub block_start_center: usize,
    pub block_end_center: usize,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivergencePair {
    pub id: DivergencePairId,
    pub move_start: usize,
    pub seg_a: (usize, usize),
    pub seg_c: (usize, usize),
}

/// 与 `LevelAsOfView::pairs` 同源的确认 sidecar；消费时必须按 `pair_id` 查找。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairConfirmState {
    pub pair_id: DivergencePairId,
    pub state: ConfirmState,
}

/// #69 5a：不含 `as_of` 与可增长 seg-c 末端的结构身份键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfirmKey {
    /// #630 修复轮（MEDIUM-2 目录化后 43 项收窄对照）：`level_view_store.rs` 的
    /// `retain_run_starts`/`retain_active_for_run` 直读这两个字段，而该文件不在本次
    /// 目录迁移范围内（仍是 classifier 直接子模块）——`pub(super)`（=level_view）不够，
    /// 须显式钉到 classifier，故用 `pub(in super::super)` 而非随迁移自动收紧。
    pub(in super::super) level: u32,
    pub(in super::super) run_window: CoordinateWindow,
    version: C2VersionTuple,
    pair_id: DivergencePairId,
    move_start: usize,
    seg_a: (usize, usize),
    c_start: usize,
    b_fingerprint: (usize, usize, Tick, Tick, Tick, Tick),
    /// `level_view::tests::confirm` 直接构造/读取本字段（结构代次剪枝断言），故留
    /// `pub(super)`（=level_view 子树）——不同于其余仅 `for_pair` 内部消费的字段。
    pub(super) structure_generation: u64,
}

impl ConfirmKey {
    pub(super) fn for_pair(
        query: LevelViewQuery,
        pair: &DivergencePair,
        last: &Center,
        structure_generation: u64,
    ) -> Self {
        Self {
            level: query.level,
            run_window: query.coordinate_window,
            version: query.version,
            pair_id: pair.id,
            move_start: pair.move_start,
            seg_a: pair.seg_a,
            c_start: pair.seg_c.0,
            b_fingerprint: (
                last.start_index,
                last.end_index,
                last.zd,
                last.zg,
                last.dd,
                last.gg,
            ),
            structure_generation,
        }
    }
}

/// 新 resident 入口的显式状态；`None` 即真冷路径。
pub struct ConfirmResidence<'a> {
    pub store: &'a mut ConfirmCursorStore,
    /// `TowerCache::tower_confirmed_len(level - 1)` 的逐字读数。
    pub stable_lower_len: usize,
    /// run 上层结构代次；变化即 key miss。
    pub structure_generation: u64,
}

/// #92 新路径的背驰段类型。盘整背驰保留独立类型，不冒充同级 B1/S1。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NestDivergenceKind {
    Trend,
    Consolidation,
}

/// #92 `NestCertificate` 的 typed provider 事件。
///
/// `Cand` 已由 provider 固定为 `dir ∧ Comparable ∧ Extreme`；`divergence_confirmed`
/// 是独立的②力度层真值，不进入 Cand。`interval_b` 是生产判据使用的完整背驰段 C；
/// `interval_a` 仅供 leave→retest 旧口径并行诊断。`judge_at` 由 prefix 首次观察写入，
/// 不读取 `CompletedFreezeEvent.created_at` 或挂钟。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestCandidateEvent {
    pub level: u32,
    pub side: Side,
    pub kind: NestDivergenceKind,
    pub seg_a: (usize, usize),
    pub interval_b: (usize, usize),
    pub interval_a: (usize, usize),
    pub divergence_confirmed: bool,
    pub turn_source: usize,
    pub judge_at: usize,
    /// provider 合法 run 的源坐标窗；只用于 prefix replay 精确路由，不进入 N 真值。
    pub provider_window: (usize, usize),
    /// #97 进料口审计标记：盘背候选的 `interval_a`（诊断口径）在 leave→retest 对不可用时
    /// 由结构兜底跨度回填。`true` 仅表示旧进料口会丢弃该候选；不进入 N 真值与 B 生产口径。
    pub intake_fallback: bool,
    /// 关③ P3（pan-terminal-endorsement-ruling-20260718）：B 中枢身份快照 = B 的
    /// `start_index`，provider 构造时按 `judge_at` 同款 prefix 首次观察快照纪律写入
    ///（禁终态回填、禁 created_at，#91 裁定④）——延伸只改写 end/dd/gg，`start_index`
    /// 与 zd/zg 稳定（p117 §7 实测 37/37 同 start 同核）。#218 面 B 起：Trend 域终端
    /// 背书 owner=B 判同用它**只当查找键**（不当身份依据——在账本 centers 查出 B 的
    /// 核心区间 (zd,zg) 后按带判同，消费唯一落点 = `nest::terminal_bits_at_event`；
    /// 旧 `point.center.start_index == b_center_start` 序号判同已随 #218 退役）；
    /// Pan 域同写（归因用途），不作门（P2 无需 owner 合取）。
    pub b_center_start: usize,
}

/// ★R1（2026-07-17 代理裁定，p112 实证 + doc-trend 总判定）：趋势背驰**全合取**确认的首个
/// 全成立时点 t*（确认时点语义；禁前视——一切结构/力度窗口以 `as_of` 为界）。
///
/// 合取项（doc-trend §1-§4 唯一谓词形式；p112 §2 机械化口径）：
/// - **T4 回拉 0 轴**（025:761/024:24）：B 中枢 span（close 下标）内 DIF 变号或触 0——静态项先判；
/// - **T3 三买**（037:18/051:314）：c 全离开段内含对 B 的第三类买卖点（`trend_third_class_in_c`，
///   去 anchor 门；无命中 ⟹ c 未确立，037:18 否则条款域）；
/// - **T2 破极值**（037:20/061:28）：c 包络破 b 包络（Long: c_lo<b_lo / Short: c_hi>b_hi），
///   窗口 [c_start, t] 随 t 渐进扩展（包络只扩 ⟹ 假→真单调）；
/// - **T5 力度或关系**（027:32/026:521/025:38）：同色柱面积 ∨ 黄白线峰 ∨ 同向柱峰，
///   C 窗口 [c_start, t] vs b 段（各 proxy 只增 ⟹ 真→假单调）。
///
/// 扫描语义：从 t3（三买确立时点）起逐段端点推进，**首个 T2∧T5-OR 同真**的段端点即 t*；
/// T5-OR 转假即终假（扫描终止，返回 None）。b 段/c_start 的 close 映射失败、或 t3 时点力度
/// 窗口仍无 bar ⟹ None（力度不可验，诚实判负——与旧 `_ => false` 口径一致）。
#[allow(clippy::too_many_arguments)]
pub(super) fn trend_confirm_state(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    side: Side,
    seg_a: (usize, usize),
    c_start: usize,
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> ConfirmState {
    trend_confirm_state_core(
        segments, last, direction, side, seg_a, c_start, as_of, hist, dif, close_src, None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn trend_confirm_state_core(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    side: Side,
    seg_a: (usize, usize),
    c_start: usize,
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    resident: Option<(&mut ConfirmCursor, usize)>,
) -> ConfirmState {
    #[cfg(test)]
    CONFIRM_CORE_CALLS.with(|calls| calls.set(calls.get() + 1));

    // T4（025:761）：B 中枢 span 内 DIF 变号或触 0（p112 主口径 cross_dif）。
    let Some((b_lo, b_hi)) = map_src_to_close_idx(close_src, last.start_index, last.end_index)
    else {
        return ConfirmState::Scanning;
    };
    if !dif_crosses_zero(dif, b_lo, b_hi) {
        return ConfirmState::Scanning;
    }
    // b 段力度基准（031:883 趋势背驰 = c vs b 比较）。
    let Some(a_idx) = map_src_to_close_idx(close_src, seg_a.0, seg_a.1) else {
        return ConfirmState::Scanning;
    };
    let Some(a_env) = range_envelope(segments, seg_a) else {
        return ConfirmState::Scanning;
    };
    let area_a = same_color_area(hist, a_idx.0, a_idx.1, side);
    let dif_peak_a = segment_dif_peak(dif, a_idx.0, a_idx.1, direction);
    let hist_peak_a = same_dir_hist_peak(hist, a_idx.0, a_idx.1, side);
    // T3（037:18）：c 全离开段内含对 B 的三买；t3 = 首个命中回试段终点（c 确立时点）。
    let t3 = trend_third_class_in_c(segments, last, direction, c_start, as_of)
        .map(|(_leave_end, t3)| t3);
    // 渐进扫描 t ≥ t3：T2/T5 窗口 [c_start, t] 随段端点增量扩展。
    let lo = segments.partition_point(|s| s.start_index < c_start.max(last.end_index));
    let hi = segments.partition_point(|s| s.end_index <= as_of);

    let scan = |cursor: &mut ConfirmCursor, range: std::ops::Range<usize>| {
        scan_confirm_cursor(
            cursor,
            segments,
            range,
            t3,
            direction,
            side,
            a_env,
            area_a,
            dif_peak_a,
            hist_peak_a,
            c_start,
            hist,
            dif,
            close_src,
        )
    };

    let Some((cursor, confirmed_len)) = resident else {
        let mut cold = ConfirmCursor {
            k0: lo,
            ..ConfirmCursor::default()
        };
        return scan(&mut cold, lo..hi);
    };

    let confirmed_len = confirmed_len.min(segments.len());
    if confirmed_len < cursor.k0 || hi < cursor.k0 {
        *cursor = ConfirmCursor::default();
    }
    if matches!(
        cursor.state,
        ConfirmState::Confirmed(_) | ConfirmState::TerminalFalse
    ) {
        return cursor.state;
    }

    let stable_hi = confirmed_len.min(hi);
    if stable_hi < lo {
        cursor.k0 = stable_hi;
    } else {
        if cursor.k0 < lo {
            cursor.k0 = lo;
        }
        // 坐标尚不可映射的已封腿不能跨 bar 持久化；本次结果仍由下方 tail 冷扫给出。
        let persist_hi = (cursor.k0..stable_hi)
            .find(|&index| {
                map_src_to_close_idx(close_src, c_start, segments[index].end_index).is_none()
            })
            .unwrap_or(stable_hi);
        let state = scan(cursor, cursor.k0..persist_hi);
        if matches!(
            state,
            ConfirmState::Confirmed(_) | ConfirmState::TerminalFalse
        ) {
            return state;
        }
    }

    // 可变尾只在本次局部副本上推进，未获水线证书的累积绝不回写 store。
    let mut tail = cursor.clone();
    let tail_start = lo.max(tail.k0);
    scan(&mut tail, tail_start..hi)
}

#[allow(clippy::too_many_arguments)]
fn scan_confirm_cursor(
    cursor: &mut ConfirmCursor,
    segments: &[Segment],
    range: std::ops::Range<usize>,
    t3: Option<usize>,
    direction: Direction,
    side: Side,
    a_env: (Tick, Tick),
    area_a: f64,
    dif_peak_a: f64,
    hist_peak_a: f64,
    c_start: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> ConfirmState {
    for index in range {
        let leg = &segments[index];
        // 包络增量扩展（T2）。
        let leg_lo = leg.start_price.min(leg.end_price);
        let leg_hi = leg.start_price.max(leg.end_price);
        cursor.env = Some(match cursor.env {
            None => (leg_lo, leg_hi),
            Some((old_lo, old_hi)) => (old_lo.min(leg_lo), old_hi.max(leg_hi)),
        });
        // 力度窗口增量扩展（T5）：新 bar 区间 (acc_hi, cur_hi] 累入。
        if let Some((c_lo, cur_hi)) = map_src_to_close_idx(close_src, c_start, leg.end_index) {
            let from = cursor.acc_hi.map_or(c_lo, |prev| prev + 1);
            if from <= cur_hi {
                for t in from..=cur_hi {
                    match side {
                        Side::Long if hist[t] < 0.0 => cursor.area_c += hist[t].abs(),
                        Side::Short if hist[t] > 0.0 => cursor.area_c += hist[t],
                        _ => {}
                    }
                    cursor.hist_max = cursor.hist_max.max(hist[t]);
                    cursor.hist_min = cursor.hist_min.min(hist[t]);
                    cursor.dif_max = cursor.dif_max.max(dif[t]);
                    cursor.dif_min = cursor.dif_min.min(dif[t]);
                }
                cursor.acc_hi = Some(cur_hi);
            }
        }
        cursor.k0 = index + 1;
        let Some(t3) = t3 else {
            continue;
        };
        if leg.end_index < t3 {
            continue; // 三买未确立前不判（037:18 必要合取）。
        }
        // t3 时点力度窗口无 bar ⟹ 力度不可验 ⟹ 诚实判负（同旧 map None ⟹ false）。
        if cursor.acc_hi.is_none() {
            return ConfirmState::Scanning;
        }
        // T5 力度或关系（027:32）：同色面积 ∨ 黄白线峰 ∨ 同向柱峰（增量量与
        // same_color_area / segment_dif_peak / same_dir_hist_peak 同口径）。
        let dif_peak_c = match direction {
            Direction::Up => cursor.dif_max.max(0.0),
            Direction::Down => cursor.dif_min.min(0.0).abs(),
        };
        let hist_peak_c = match side {
            Side::Long => cursor.hist_min.min(0.0).abs(),
            Side::Short => cursor.hist_max.max(0.0),
        };
        let force_ok =
            cursor.area_c < area_a || dif_peak_c < dif_peak_a || hist_peak_c < hist_peak_a;
        if !force_ok {
            // 各 proxy 只增 ⟹ 真→假单调，转假即终假（033:26 无衰减即无背驰）。
            cursor.state = ConfirmState::TerminalFalse;
            return cursor.state;
        }
        // T2 破极值（037:20）：c 包络破 b 包络。
        let (env_lo, env_hi) = cursor.env.expect("扫描窗口非空（t3 已命中）");
        let extreme = match side {
            Side::Long => env_lo < a_env.0,
            Side::Short => env_hi > a_env.1,
        };
        if extreme {
            // t* = 首个 T2∧T5-OR 同真时点。
            cursor.state = ConfirmState::Confirmed(leg.end_index);
            return cursor.state;
        }
    }
    cursor.state = ConfirmState::Scanning;
    cursor.state
}

#[allow(clippy::too_many_arguments)]
pub(super) fn trend_confirm_time(
    segments: &[Segment],
    last: &Center,
    direction: Direction,
    side: Side,
    seg_a: (usize, usize),
    c_start: usize,
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Option<usize> {
    trend_confirm_state(
        segments, last, direction, side, seg_a, c_start, as_of, hist, dif, close_src,
    )
    .as_option()
}
