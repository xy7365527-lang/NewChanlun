//! P52 只读召回上界：不经过 Cand^δ 事件入口（旧 P1 `level_cand_delta` 已随 3b 退役，
//! 本审计自始不依赖事件面），直接在每级稳定
//! `B_p/c_p` 对象全集上重判 leave/retest 几何。
//!
//! #748（C4）纯移动自 `classifier/mod.rs`（原 `cp_recall_upper_bound_audit`）。
//! #1180（D01）类型归位：自 `recursive_tower.rs` 纯移动迁入 cp recall 审计类型族
//! （WindowScanCursor / WinMeta / CpScanOwnership / CpLifecycleStatus / CpRecallAtom /
//! CpRecallAuditCase + audit_cp_recall_upper_bound + cp_scan_ownership），零行为。

use std::rc::Rc;

use crate::theta_v0::classifier::center::UnitRange;
use crate::theta_v0::classifier::chain_cert::{
    FullTrendCQualified, FullTrendQualificationEvidence,
};
use crate::theta_v0::classifier::recursive_tower::{
    cp_unit_to_segment, project_to_units, CpStructureIdentity, ElementId, LeveledMove,
    ThirdClassInCp,
};
use crate::theta_v0::classifier::signal;
use crate::theta_v0::classifier::{decompose, segment_to_unit, Classification};
use crate::theta_v0::parser::ParseLayer;
use crate::theta_v0::types::{Center, Direction, Segment, Tick};

/// 扫描退出断点（增量续进的锚）：while 退出时的游标位置。
///
/// `consumed` 满足 `consumed + 2 >= units.len()`（while 终止条件）。尾部追加 units 后，
/// `consumed + 2 < new_len` 可能成立 ⟹ 从 `consumed` 续扫正确（见模块文档增量证明）。
///
/// ★frontier bug 修复（task #47/#21，区间套.pdf 六~十节裁决②）：`consumed` **不能**直接作
/// resume 起点——成立支消费整个窗口后 `consumed` 越过最后一个成立窗口，把它当 sealed prefix。
/// 但该窗口尾段可能是 frontier（未确认段），新 bar 到来后（后续新段使全量扫描在此窗口后续段落
/// 产出不同中枢，或古怪线段重划改写尾段）该中枢应重算。PDF：只有**完全结束于最后 sealed 边界
/// `b_t` 前**的窗口 sealed；`b_t` 之后（含最后一个成立窗口，因其可能依赖 frontier 段）必须重算。
/// 保守版（PDF §八）：`b_t = 当前活跃候选前最后稳定端点`，rollback 重算。
/// ★task #142 延伸语义后此协议从「保守正确」升为**必需**：末位中枢在未被 non-extension 单元
/// 终止前开放（新单元可延伸它），从 `consumed` 续进恒不合法——见 `detect_centers_windowed_resume`
/// 充要条件 #1。
///
/// `resume_from` = **最后一个成立窗口的起点**（`win[0]`），即保守 `b_t` 锚。resume 从 `resume_from`
/// 重扫（而非 `consumed`）⟹ 最后一个中枢每 bar 重算，其真正 sealed（后面又出现成立窗口把它推进
/// prefix）后自然稳定。无成立窗口 ⟹ `resume_from == start_i`（无中枢可回退，续进语义不变）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowScanCursor {
    /// 已扫描到的游标位置（退出点；`units[..consumed]` 的扫描路径已确定）。
    pub consumed: usize,
    /// ★frontier 修复锚：本次扫描最后一个成立窗口的起点（`win[0]`）。下次 resume 起点用此
    /// （而非 `consumed`）⟹ 最后一个（frontier）中枢重算。无成立窗口 ⟹ == 本次 `start_i`
    /// （无回退，续进不变）。
    pub resume_from: usize,
    /// ★#148 升级重切：最后一个成立窗口产出的中枢数（<9 段窗口 =1；≥9 段重切窗口 =⌊n/3⌋）。
    /// frontier 回退域是**整窗产出**——调用方 pop 该数量（只 pop 1 会残留旧子中枢，与重扫
    /// 产出重复）。无成立窗口 ⟹ 0（与 `resume_from == consumed` 一致，guard 不触发 pop）。
    pub last_window_emitted: usize,
}

/// ★on2w2-cascade 读域侧车（设计 §4.1 解 A）：每个产出 center 一条，记录产出它的**窗口读域**，
/// 供 cascade 增量失效（按 `read_end_src < e` 取保留前缀 P）。与 `LevelCache.centers`/`upper_moves`
/// 1:1 对齐（同序同长，同前缀不可变 + 尾部续扫追加）。
///
/// **读域上界 `read_end_src`**（设计 §1.2.1，codex 二审终版）：detect 延伸循环停止时读了首个
/// non-extension 哨兵 `units[win.1+1]`，其源坐标才是 center 的完整读域上界（读域 `[win.0..win.1+1]`
/// ⊋ 输出区间 `[start,end]`）。`units[win.1+1]` 越界（`win.1+1 == units.len()`，窗口开放无哨兵）⟹
/// `read_end_src = usize::MAX`（+∞，永不进保留前缀，归 frontier pop 常态处理）。
///
/// **升级窗口共享**（设计 §3.5）：#148 升级重切窗口产 k 个子中枢，它们**共享同一父窗口**
/// `(win.0, win.1+1)` ⟹ `win_start`/`read_end_src` 对该窗口全部 k 个子中枢**逐值相等**。故按
/// `read_end_src < e` 的 `partition_point` 天然「整窗保留或整窗失效」——子中枢不会被从中部截断
/// （§3.5 blocker 由此自动解除，无需额外 snap 逻辑）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinMeta {
    /// 产出该 center 的窗口起点（`win.0`，unit 下标）。升级子中枢共享父窗口起点。
    pub win_start: usize,
    /// 窗口退出点（`win.1+1` = detect 的 `j`，unit 下标）。cascade cursor 重建的 `consumed`
    /// （had_emitted_window = win_start < win_exit 判据；仅瞬态用，compose 后被 new_cursor 覆盖）。
    pub win_exit: usize,
    /// 读域上界源坐标 = `units[win.1+1].start_index`（停止哨兵）；窗口开放（无哨兵）⟹ `usize::MAX`。
    pub read_end_src: usize,
    /// 该窗口产出的 center 数（#148 升级 = k，普通 = 1）——cursor 重建时的 `last_window_emitted`。
    pub emitted: usize,
}

/// 完整父级 `c` 的扫描期归属与生命周期对象（与产出的中枢/上级走势 1:1 对齐）。
///
/// `departure_move` 取自中枢窗口停止时读到的首个 non-extension 单元 `units[win_exit]`。
/// 该单元随后可以成为下一中枢 seed/延伸窗口的一部分；本侧车仍保存它最初作为 `B_p`
/// 离开单元的结构归属，因此事件端不需要、也禁止从最终 [`MoveBlock`] 反猜 `c_start_full`。
/// 之后每个新同级相邻单元对经 [`advance_cp_lifecycles`] 推进 Pending/Closed 状态；闭合不依赖
/// 新的 [`CandDeltaEvent`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpScanOwnership {
    /// `B_p` 在本级中枢序列中的确定性下标（全量/增量均等于 compose ordinal）。
    pub b_center_index: usize,
    /// 表示该中枢构成窗口的确定性上级元素 ID。
    pub b_center_id: ElementId,
    /// `B_p` 的核心、外缘与 source-index 首尾。
    pub b_center: Center,
    /// `B_p` 后首个 non-extension 单元的确定性递归元素 ID；开放窗口尚无离开时为 `None`。
    pub departure_move_id: Option<ElementId>,
    /// 上述离开单元的 source-index 闭区间；其左端是结构分解产出的 `c_start_full` 候选。
    pub departure_interval: Option<(usize, usize)>,
    /// `B_p/c_p` 对象的持续生命周期；只能从 Pending 单调闭合为 Closed。
    pub lifecycle: CpLifecycleStatus,
    /// 第三类/完整结构第一次可证的 source-index。与背驰事件确认时点严格分离。
    pub cp_certificate_confirm_src: Option<usize>,
    /// 终态完整 `c_p` 结构。Pending 时允许保存开放左端，右端必须为 `None`。
    pub c_structure: Option<CpStructureIdentity>,
    /// 终态第三类证书；Pending 时严格为 `None`。
    pub third_class_in_c: Option<ThirdClassInCp>,
    /// 第 20/22 行与完成分解的分量证据；Closed 后即使全合取失败也保留，供分类复核。
    pub full_trend_evidence: Option<FullTrendQualificationEvidence>,
    /// 终态第 37 课完整趋势 `c_p` 合取证书；第三类闭合但第 20/22 行或完成分解失败时为 `None`。
    pub full_trend_c_qualified: Option<FullTrendCQualified>,
}

/// 完整 `c_p` 对象生命周期。闭合只由合法第三类对象生成，不由后续 Cand 事件生成。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpLifecycleStatus {
    Pending,
    Closed,
}

/// P52 召回上界审计的逐对象原子结果。
///
/// 该枚举只描述稳定 `B_p/c_p` 对象上 leave/retest 几何的只读重判结果；它不产生
/// [`CandDeltaEvent`]，也不改变任何生命周期或 strict-chain 真值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CpRecallAtom {
    Success,
    NoDepartureMove,
    NoBOwnedAdjacentPair,
    LeaveAnchorNone,
    DirectionPairMismatch,
    LeaveNotStrictlyOutsideB,
    RetestReentersB,
}

impl CpRecallAtom {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::NoDepartureMove => "NO_DEPARTURE_MOVE",
            Self::NoBOwnedAdjacentPair => "NO_B_OWNED_ADJACENT_PAIR",
            Self::LeaveAnchorNone => "LEAVE_ANCHOR_NONE",
            Self::DirectionPairMismatch => "DIRECTION_PAIR_MISMATCH",
            Self::LeaveNotStrictlyOutsideB => "LEAVE_NOT_STRICTLY_OUTSIDE_B",
            Self::RetestReentersB => "RETEST_REENTERS_B",
        }
    }

    fn progress_rank(self) -> u8 {
        match self {
            Self::NoDepartureMove => 0,
            Self::NoBOwnedAdjacentPair => 1,
            Self::LeaveAnchorNone => 2,
            Self::DirectionPairMismatch => 3,
            Self::LeaveNotStrictlyOutsideB => 4,
            Self::RetestReentersB => 5,
            Self::Success => 6,
        }
    }
}

/// 稳定 `B_p/c_p` 对象全集上的一条 P52 只读召回审计记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpRecallAuditCase {
    pub level: u32,
    pub b_center_id: ElementId,
    pub b_source_interval: (usize, usize),
    pub b_core: (Tick, Tick),
    pub cp_source_start: Option<usize>,
    pub cp_departure_move_id: Option<ElementId>,
    pub departure_move_id: Option<ElementId>,
    pub retest_move_id: Option<ElementId>,
    pub departure_interval: Option<(usize, usize)>,
    pub retest_interval: Option<(usize, usize)>,
    pub atom: CpRecallAtom,
}

fn recall_failure_atom(
    center: &Center,
    leave: &Segment,
    leave_anchor: Option<Direction>,
    retest: &Segment,
) -> CpRecallAtom {
    match (leave_anchor, retest.direction) {
        (None, _) => CpRecallAtom::LeaveAnchorNone,
        (Some(Direction::Up), Direction::Down) => {
            if leave.end_price <= center.zg {
                CpRecallAtom::LeaveNotStrictlyOutsideB
            } else {
                CpRecallAtom::RetestReentersB
            }
        }
        (Some(Direction::Down), Direction::Up) => {
            if leave.end_price >= center.zd {
                CpRecallAtom::LeaveNotStrictlyOutsideB
            } else {
                CpRecallAtom::RetestReentersB
            }
        }
        _ => CpRecallAtom::DirectionPairMismatch,
    }
}

/// 绕过 `cand_delta` 事件入口，在稳定 `B_p` 对象全集上直接枚举相邻 leave/retest，并调用
/// [`signal::judge_third_cert`] 同一真值函数构造召回上界。
///
/// 每个稳定对象只返回一行：有合法第三类时取首次成功 pair；否则返回所有可归属 pair 中推进最深的
/// 失败原子。函数不写对象、不回填快照，也不读取 `CandDeltaEvent`。
pub fn audit_cp_recall_upper_bound(
    level: u32,
    centers: &[Center],
    objects: &[CpScanOwnership],
    units: &[UnitRange],
    unit_moves: &[LeveledMove],
    anchor_dirs: Option<&[Option<Direction>]>,
) -> Vec<CpRecallAuditCase> {
    debug_assert_eq!(units.len(), unit_moves.len());
    if let Some(anchors) = anchor_dirs {
        debug_assert_eq!(anchors.len(), units.len());
    }

    let mut results: Vec<CpRecallAuditCase> = objects
        .iter()
        .map(|object| CpRecallAuditCase {
            level,
            b_center_id: object.b_center_id,
            b_source_interval: (object.b_center.start_index, object.b_center.end_index),
            b_core: (object.b_center.zd, object.b_center.zg),
            cp_source_start: object.departure_interval.map(|interval| interval.0),
            cp_departure_move_id: object.departure_move_id,
            departure_move_id: None,
            retest_move_id: None,
            departure_interval: None,
            retest_interval: None,
            atom: if object.departure_move_id.is_some() && object.departure_interval.is_some() {
                CpRecallAtom::NoBOwnedAdjacentPair
            } else {
                CpRecallAtom::NoDepartureMove
            },
        })
        .collect();

    // 与 advance_cp_lifecycles 同复杂度：每个相邻 pair 只路由到唯一最近 B_p，避免逐对象扫全塔。
    for retest_idx in 1..units.len() {
        let leave_idx = retest_idx - 1;
        let leave_unit = &units[leave_idx];
        let Some(center_idx) =
            signal::nearest_confirmed_center_idx(centers, leave_unit.start_index)
        else {
            continue;
        };
        let (Some(object), Some(result)) = (objects.get(center_idx), results.get_mut(center_idx))
        else {
            continue;
        };
        if result.atom == CpRecallAtom::Success
            || object.b_center_index != center_idx
            || object.b_center != centers[center_idx]
        {
            continue;
        }
        let (Some(cp_departure_move_id), Some((cp_start, _))) =
            (object.departure_move_id, object.departure_interval)
        else {
            continue;
        };
        if leave_unit.start_index < cp_start {
            continue;
        }
        let (Some(leave_move), Some(retest_move)) =
            (unit_moves.get(leave_idx), unit_moves.get(retest_idx))
        else {
            continue;
        };
        if leave_move.id.level != cp_departure_move_id.level
            || retest_move.id.level != cp_departure_move_id.level
            || leave_move.id.ordinal < cp_departure_move_id.ordinal
            || retest_move.id.ordinal < leave_move.id.ordinal
        {
            continue;
        }

        let leave = cp_unit_to_segment(leave_unit);
        let retest = cp_unit_to_segment(&units[retest_idx]);
        let leave_anchor = match anchor_dirs {
            Some(anchors) => anchors.get(leave_idx).copied().unwrap_or(None),
            None => Some(leave.direction),
        };
        let atom = if signal::judge_third_cert(&object.b_center, &leave, leave_anchor, &retest)
            .is_some()
        {
            CpRecallAtom::Success
        } else {
            recall_failure_atom(&object.b_center, &leave, leave_anchor, &retest)
        };
        if atom.progress_rank() > result.atom.progress_rank() {
            result.atom = atom;
            result.departure_move_id = Some(leave_move.id);
            result.retest_move_id = Some(retest_move.id);
            result.departure_interval = Some((leave.start_index, leave.end_index));
            result.retest_interval = Some((retest.start_index, retest.end_index));
        }
    }
    results
}

pub(crate) fn cp_scan_ownership(
    center: Center,
    meta: WinMeta,
    units: &[UnitRange],
    subs_moves: &[LeveledMove],
    b_center_id: ElementId,
) -> CpScanOwnership {
    let departure = units.get(meta.win_exit).zip(subs_moves.get(meta.win_exit));
    let departure_move_id = departure.map(|(_, m)| m.id);
    let departure_interval = departure.map(|(u, _)| (u.start_index, u.end_index));
    CpScanOwnership {
        b_center_index: b_center_id.ordinal as usize,
        b_center_id,
        b_center: center,
        departure_move_id,
        departure_interval,
        lifecycle: CpLifecycleStatus::Pending,
        cp_certificate_confirm_src: None,
        c_structure: departure_move_id
            .zip(departure_interval)
            .map(|(move_id, interval)| CpStructureIdentity {
                level: move_id.level,
                b_center_id,
                departure_move_id: move_id,
                terminal_move_id: None,
                source_start: interval.0,
                source_end: None,
            }),
        third_class_in_c: None,
        full_trend_evidence: None,
        full_trend_c_qualified: None,
    }
}

pub fn cp_recall_upper_bound_audit(
    l0: &ParseLayer,
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
) -> Vec<CpRecallAuditCase> {
    let mut out = Vec::new();
    for (level, state) in classification.levels.iter().enumerate() {
        let Some(unit_moves) = tower_snapshots.get(level) else {
            continue;
        };
        if level == 0 {
            let units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
            out.extend(audit_cp_recall_upper_bound(
                0,
                &state.centers,
                &state.cp_ownership,
                &units,
                unit_moves,
                None,
            ));
        } else {
            let parent_blocks = &classification.levels[level - 1].moves;
            let units = project_to_units(unit_moves, parent_blocks);
            let anchors: Vec<Option<Direction>> = (0..units.len())
                .map(|i| decompose::center_own_dir_at(parent_blocks, i))
                .collect();
            out.extend(audit_cp_recall_upper_bound(
                level as u32,
                &state.centers,
                &state.cp_ownership,
                &units,
                unit_moves,
                Some(&anchors),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange {
            start_index: si,
            end_index: ei,
            direction: dir,
            lo,
            hi,
        }
    }

    fn up() -> Direction {
        Direction::Up
    }
    fn down() -> Direction {
        Direction::Down
    }

    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    fn pending_cp_object(
        center: Center,
        b_center_id: ElementId,
        departure_move_id: ElementId,
        departure_interval: (usize, usize),
    ) -> CpScanOwnership {
        CpScanOwnership {
            b_center_index: 0,
            b_center_id,
            b_center: center,
            departure_move_id: Some(departure_move_id),
            departure_interval: Some(departure_interval),
            lifecycle: CpLifecycleStatus::Pending,
            cp_certificate_confirm_src: None,
            c_structure: Some(CpStructureIdentity {
                level: departure_move_id.level,
                b_center_id,
                departure_move_id,
                terminal_move_id: None,
                source_start: departure_interval.0,
                source_end: None,
            }),
            third_class_in_c: None,
            full_trend_evidence: None,
            full_trend_c_qualified: None,
        }
    }

    #[test]
    fn recall_upper_bound_finds_legal_third_without_cand_event_input() {
        let center = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 8,
        };
        let units = vec![unit(9, 11, down(), 80, 150), unit(11, 13, up(), 80, 90)];
        let moves = vec![
            LeveledMove::from_unit(&units[0], eid(1, 7)),
            LeveledMove::from_unit(&units[1], eid(1, 8)),
        ];
        let objects = vec![pending_cp_object(center, eid(2, 3), eid(1, 7), (9, 11))];
        let anchors = vec![Some(Direction::Down), Some(Direction::Up)];

        let cases =
            audit_cp_recall_upper_bound(1, &[center], &objects, &units, &moves, Some(&anchors));

        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].atom, CpRecallAtom::Success);
        assert_eq!(cases[0].b_center_id, eid(2, 3));
        assert_eq!(cases[0].departure_move_id, Some(eid(1, 7)));
        assert_eq!(cases[0].retest_move_id, Some(eid(1, 8)));
        assert_eq!(cases[0].departure_interval, Some((9, 11)));
        assert_eq!(cases[0].retest_interval, Some((11, 13)));
    }

    #[test]
    fn recall_upper_bound_reports_closest_failure_atom() {
        let center = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 8,
        };
        let units = vec![unit(9, 11, down(), 80, 150), unit(11, 13, up(), 80, 120)];
        let moves = vec![
            LeveledMove::from_unit(&units[0], eid(1, 7)),
            LeveledMove::from_unit(&units[1], eid(1, 8)),
        ];
        let objects = vec![pending_cp_object(center, eid(2, 3), eid(1, 7), (9, 11))];
        let anchors = vec![Some(Direction::Down), Some(Direction::Up)];

        let cases =
            audit_cp_recall_upper_bound(1, &[center], &objects, &units, &moves, Some(&anchors));

        assert_eq!(cases[0].atom, CpRecallAtom::RetestReentersB);
        assert_eq!(cases[0].departure_interval, Some((9, 11)));
        assert_eq!(cases[0].retest_interval, Some((11, 13)));
    }
}
