//! CandDeltaEvent 载体层（#1180 D01 自 `recursive_tower.rs` 纯移动迁出，零行为）。
//!
//! 本文件只承载 CandDeltaEvent 投影族（CandDeltaEvent / CandDeltaCpEdge / CandDeltaEntryOrigin /
//! CandDeltaEntryEvent + relaxed_cand_delta_entries）。消费面经 `recursive_tower` / `cand_event`
//! 重导出保持原路径不变。

// ═══════════════════════════════════════════════════════════════════════════════════════
//  CandDeltaEvent 载体层（strict-nesting-divergence-plan-20260708 三裁决：① Cand^δ ≔ 背驰段
//  谓词（per-level A/C 定位配对，gauge 复用 divergence.rs MacdArea 默认路径，严格 curr <
//  prev）；② 盘整背驰不入链（单独诊断标志位）；③ confirm_src 为独立算法确认时点。P0
//  D_parent 复议后，完整 c 与局部 episode 分离；confirm_src 仅登记延迟）。
//
//  3b（ADR 0026 裁定三 / #1059 路线 A）：本层原 P1 谓词闭包（level_cand_delta 独立扫描 +
//  cp_event_objects 快照合成）退役——事件生产改由 scan.rs 纯投影
//  [`crate::theta_v0::classifier::scan::project_cand_delta_events`] 从生产合并扫描逐段素材 +
//  `LevelState.cp_ownership` 读回（判据单一，见 scan.rs 投影段头）；本层只保留 CandDeltaEvent
//  载体与 c_p 生命周期对象（advance_cp_lifecycles / relaxed_cand_delta_entries 等）。
// ═══════════════════════════════════════════════════════════════════════════════════════

use crate::theta_v0::classifier::chain_cert::{
    FullTrendCQualified, FullTrendQualificationEvidence,
};
use crate::theta_v0::classifier::diag::cp_recall_audit::{CpLifecycleStatus, CpScanOwnership};
use crate::theta_v0::classifier::recursive_tower::{
    CpStructureIdentity, ElementId, ParentCenterIdentity, ThirdClassInCp,
};
use crate::theta_v0::types::Side;

/// `CandDeltaEvent -> c_p` 的稳定归属边。这里只允许保存对象身份，不保存终态右端。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandDeltaCpEdge {
    pub b_center_id: ElementId,
    pub cp_departure_move_id: ElementId,
    pub cp_source_start: usize,
}

/// Cand^δ_ℓ 事件：级别 ℓ 的背驰段谓词判定（一次破中枢结构候选的完整证据包）。
///
/// 产生条件 = `judge_first_cached` 返回 `Some`（破最后中枢几何 ∧ 037:20 破 b 包络极值 ∧ A/C 可
/// 配对 ∧ closes 可映射，P2-R2 候选口径 + p117 037:20 收缩）；`cand_delta` = D 背驰确认（默认
/// gauge 下 ≡ MACD 面积严格 C<A，= `bits.buy1 ∨ bits.sell1`——从产出派生，无第二套判据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandDeltaEvent {
    /// 级别 ℓ（L0=0）。
    pub level: u32,
    /// 破中枢方向侧（买侧向下破=Long，卖侧向上破=Short；= `BspPoint.struct_break_dir`）。
    pub side: Side,
    /// [新缠论] 算法确认时点（#37 P0 局部改判）：破中枢段端点 source_index。
    /// 与 `interval.1`（结构定位窗右端）是独立字段；settled 生产者上可数值相等，但不得互相派生。
    pub divergence_confirm_src: usize,
    /// 旧消费者兼容别名；规范字段是 `divergence_confirm_src`。
    pub confirm_src: usize,
    /// 旧重放消费者的兼容区间，值等于 `c_episode_interval`；不再承载规范 `D_parent`。
    /// 完整父区间只读 `c_interval_full`，右端无证明时为 `None`。
    pub interval: (usize, usize),
    /// I(A) = [λ_A, ρ_A]（跨相邻中枢配对的前一中枢离开 episode 区间，0016:62）。
    pub a_interval: (usize, usize),
    /// 当前 C 离开 episode 的局部起点；唯一 writer = [`departure_move_c_start`]。
    pub c_episode_start: usize,
    /// 当前 C 离开 episode 的局部区间。reentry 只切分本对象。
    pub c_episode_interval: (usize, usize),
    /// 完整 `c_p` 区间；结构证书不足时严格为 `None`。
    pub c_interval_full: Option<(usize, usize)>,
    /// `B_p` 确定性身份（生产塔扫描侧车来源）。
    pub b_parent: Option<ParentCenterIdentity>,
    /// 完整 `c_p` 结构身份；允许右端 `None` 表示尚不能证明完成。
    pub c_structure: Option<CpStructureIdentity>,
    /// `c_p` 内第三类离开/回试结构。
    pub third_class_in_c: Option<ThirdClassInCp>,
    /// 确认时快照内，完整 `c_p` 第一次可证的 source-index；当时不可证必须保持 `None`。
    pub cp_certificate_confirm_src: Option<usize>,
    /// 确认时快照内的 R3 完整趋势合取证书；不得从终态对象回填。
    pub full_trend_c_qualified: Option<FullTrendCQualified>,
    /// 确认时快照内的 R3 分量证据；全合取失败也不得丢弃已成立的第 20/22 行证书。
    pub full_trend_evidence: Option<FullTrendQualificationEvidence>,
    /// 事件到 `c_p` 对象的稳定归属边；不携带终态右端。
    pub cp_ownership: Option<CandDeltaCpEdge>,
    /// C 离开 episode 的首个同向段起点；不是确认时点，也不是完整 `c_p` 左端定义。
    /// 兼容旧消费者的别名；规范字段是 `c_episode_start`，不得与 `c_start_full` 建立相等公理。
    pub enter_src: usize,
    /// Cand^δ 真值：趋势背驰确认 D（默认 gauge ≡ 严格 C<A）。
    pub cand_delta: bool,
    /// 盘整背驰诊断标志（当前**不入谓词**＝实装态，非裁定态：0708「盘背不入链」裁决已被
    /// `chanlun/escalate/nest-migration-ruling-20260716.md` 裁决⑤「盘背入链」supersede，#250/#260 在案，
    /// 清理票 #726）——同段最近中枢 ownership 落 Consolidation 块
    /// 且 `judge_pan_div` 产证书（独立范畴，诊断并列不合取）。
    pub pan_div_diag: bool,
}

/// P53 放宽后的 Cand 入口事件。
///
/// 该事件只由已经在生产生命周期中经 [`signal::judge_third_cert`] 闭合的稳定
/// `B_p/c_p` 对象产生。它与 [`CandDeltaEvent`] 的算法背驰真值正交：没有历史背驰事件的
/// 稳定对象不需要伪造 `a_interval` 或 `divergence_confirm_src`，已有 `cand_delta=false`
/// 事件也不需要改写其历史真值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandDeltaEntryOrigin {
    /// 同一 `CpId` 已有 `cand_delta=true` 的算法背驰事件。
    ExistingCandDeltaTrue,
    /// 同一 `CpId` 已有事件，但算法背驰真值为 false；P53 仅放宽几何入口。
    ExistingCandDeltaFalse,
    /// 同一 `CpId` 没有历史事件，由稳定对象的第三类证书直接事件化。
    StableCpGeometry,
}

impl CandDeltaEntryOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExistingCandDeltaTrue => "EXISTING_CAND_DELTA_TRUE",
            Self::ExistingCandDeltaFalse => "EXISTING_CAND_DELTA_FALSE",
            Self::StableCpGeometry => "STABLE_CP_GEOMETRY",
        }
    }
}

/// P53 几何入口事件的完整对象证据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandDeltaEntryEvent {
    pub level: u32,
    pub side: Side,
    pub cp_ownership: CandDeltaCpEdge,
    pub c_structure: CpStructureIdentity,
    pub third_class_in_c: ThirdClassInCp,
    pub cp_certificate_confirm_src: usize,
    pub full_trend_evidence: Option<FullTrendQualificationEvidence>,
    pub full_trend_c_qualified: Option<FullTrendCQualified>,
    pub origin: CandDeltaEntryOrigin,
}

/// 将本级稳定第三类对象事件化，形成 P53 放宽后的入口集合。
///
/// 生产入口以 `CpLifecycleStatus::Closed` 为唯一资格；Closed 的对象证书字段是生命周期 writer
/// 的不变量，缺任一字段即 panic 暴露对象级实现错误，禁止静默吞例。历史
/// [`CandDeltaEvent`] 只用于标注来源，函数不修改它们。
pub fn relaxed_cand_delta_entries(
    level: u32,
    objects: &[CpScanOwnership],
    legacy_events: &[CandDeltaEvent],
) -> Vec<CandDeltaEntryEvent> {
    let mut entries = Vec::new();
    for object in objects
        .iter()
        .filter(|object| object.lifecycle == CpLifecycleStatus::Closed)
    {
        let cp_departure_move_id = object
            .departure_move_id
            .expect("P53 invariant: Closed B_p/c_p 缺 cp_departure_move_id");
        let cp_source_start = object
            .departure_interval
            .map(|interval| interval.0)
            .expect("P53 invariant: Closed B_p/c_p 缺 departure_interval");
        let c_structure = object
            .c_structure
            .expect("P53 invariant: Closed B_p/c_p 缺 c_structure");
        let third_class_in_c = object
            .third_class_in_c
            .expect("P53 invariant: Closed B_p/c_p 缺 third_class_in_c");
        let cp_certificate_confirm_src = object
            .cp_certificate_confirm_src
            .expect("P53 invariant: Closed B_p/c_p 缺 cp_certificate_confirm_src");
        assert_eq!(c_structure.b_center_id, object.b_center_id);
        assert_eq!(c_structure.departure_move_id, cp_departure_move_id);
        assert_eq!(third_class_in_c.b_center_id, object.b_center_id);
        assert_eq!(third_class_in_c.cp_departure_move_id, cp_departure_move_id);
        assert_eq!(
            third_class_in_c.point_source_index,
            cp_certificate_confirm_src
        );

        let matching_legacy = legacy_events.iter().filter(|event| {
            let legacy_identity = event
                .cp_ownership
                .map(|edge| {
                    (
                        edge.b_center_id,
                        edge.cp_departure_move_id,
                        edge.cp_source_start,
                    )
                })
                .or_else(|| {
                    event.c_structure.map(|structure| {
                        (
                            structure.b_center_id,
                            structure.departure_move_id,
                            structure.source_start,
                        )
                    })
                });
            event.level == level
                && legacy_identity
                    == Some((object.b_center_id, cp_departure_move_id, cp_source_start))
        });
        let (mut has_true, mut has_false) = (false, false);
        for event in matching_legacy {
            if event.cand_delta {
                has_true = true;
            } else {
                has_false = true;
            }
        }
        let origin = if has_true {
            CandDeltaEntryOrigin::ExistingCandDeltaTrue
        } else if has_false {
            CandDeltaEntryOrigin::ExistingCandDeltaFalse
        } else {
            CandDeltaEntryOrigin::StableCpGeometry
        };
        entries.push(CandDeltaEntryEvent {
            level,
            side: third_class_in_c.side,
            cp_ownership: CandDeltaCpEdge {
                b_center_id: object.b_center_id,
                cp_departure_move_id,
                cp_source_start,
            },
            c_structure,
            third_class_in_c,
            cp_certificate_confirm_src,
            full_trend_evidence: object.full_trend_evidence.clone(),
            full_trend_c_qualified: object.full_trend_c_qualified.clone(),
            origin,
        });
    }
    entries.sort_by_key(|entry| {
        (
            entry.level,
            entry.cp_ownership.b_center_id.level,
            entry.cp_ownership.b_center_id.ordinal,
            entry.cp_ownership.cp_departure_move_id.level,
            entry.cp_ownership.cp_departure_move_id.ordinal,
        )
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::classifier::center::UnitRange;
    use crate::theta_v0::classifier::recursive_tower::{advance_cp_lifecycles, LeveledMove};
    use crate::theta_v0::types::{Center, Direction, Tick};

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

    fn legacy_event_for_edge(
        level: u32,
        edge: CandDeltaCpEdge,
        cand_delta: bool,
    ) -> CandDeltaEvent {
        CandDeltaEvent {
            level,
            side: Side::Short,
            divergence_confirm_src: edge.cp_source_start,
            confirm_src: edge.cp_source_start,
            interval: (edge.cp_source_start, edge.cp_source_start),
            a_interval: (edge.cp_source_start, edge.cp_source_start),
            c_episode_start: edge.cp_source_start,
            c_episode_interval: (edge.cp_source_start, edge.cp_source_start),
            c_interval_full: None,
            b_parent: None,
            c_structure: None,
            third_class_in_c: None,
            cp_certificate_confirm_src: None,
            full_trend_c_qualified: None,
            full_trend_evidence: None,
            cp_ownership: Some(edge),
            enter_src: edge.cp_source_start,
            cand_delta,
            pan_div_diag: false,
        }
    }

    fn close_fixture_object(
        center: Center,
        b_center_id: ElementId,
        departure_move_id: ElementId,
        retest_move_id: ElementId,
        leave: UnitRange,
        retest: UnitRange,
    ) -> CpScanOwnership {
        let moves = vec![
            LeveledMove::from_unit(&leave, departure_move_id),
            LeveledMove::from_unit(&retest, retest_move_id),
        ];
        let mut objects = vec![pending_cp_object(
            center,
            b_center_id,
            departure_move_id,
            (leave.start_index, leave.end_index),
        )];
        advance_cp_lifecycles(
            &mut objects,
            &[center],
            &[leave, retest],
            &moves,
            Some(&[Some(Direction::Up), Some(Direction::Down)]),
            1,
        );
        assert_eq!(objects[0].lifecycle, CpLifecycleStatus::Closed);
        objects.remove(0)
    }

    /// P53 回归：P52 原 `NO_EVENT_FOR_CP_ID` 第 1 例（L2#31/L1#140）必须由稳定
    /// `judge_third_cert` 对象直接事件化，且不得伪造历史 CandDeltaEvent。
    #[test]
    fn p53_relaxes_original_no_event_l2_31_l1_140() {
        let center = Center {
            zd: 455_000_000_000,
            zg: 465_900_000_000,
            dd: 450_000_000_000,
            gg: 470_000_000_000,
            start_index: 75_281,
            end_index: 76_706,
        };
        let object = close_fixture_object(
            center,
            eid(2, 31),
            eid(1, 140),
            eid(1, 141),
            unit(76_749, 77_211, up(), 460_000_000_000, 480_000_000_000),
            unit(77_222, 77_473, down(), 470_000_000_000, 480_000_000_000),
        );

        let entries = relaxed_cand_delta_entries(1, &[object], &[]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].origin, CandDeltaEntryOrigin::StableCpGeometry);
        assert_eq!(entries[0].cp_ownership.b_center_id, eid(2, 31));
        assert_eq!(entries[0].cp_ownership.cp_departure_move_id, eid(1, 140));
        assert_eq!(entries[0].third_class_in_c.retest_move_id, eid(1, 141));
    }

    /// P53 回归：P52 原 `CAND_DELTA_FALSE` 的 L2#764/L1#3401 保留算法背驰 false，
    /// 但经独立几何入口纳入 E。
    #[test]
    fn p53_relaxes_original_cand_delta_false_l2_764_l1_3401() {
        let center = Center {
            zd: 1_133_149_000_000,
            zg: 1_142_200_000_000,
            dd: 1_130_000_000_000,
            gg: 1_150_000_000_000,
            start_index: 1_594_745,
            end_index: 1_596_439,
        };
        let object = close_fixture_object(
            center,
            eid(2, 764),
            eid(1, 3401),
            eid(1, 3402),
            unit(
                1_596_755,
                1_597_267,
                up(),
                1_140_000_000_000,
                1_160_000_000_000,
            ),
            unit(
                1_597_267,
                1_597_963,
                down(),
                1_150_000_000_000,
                1_160_000_000_000,
            ),
        );
        let edge = CandDeltaCpEdge {
            b_center_id: eid(2, 764),
            cp_departure_move_id: eid(1, 3401),
            cp_source_start: 1_596_755,
        };
        let mut legacy = legacy_event_for_edge(1, edge, false);
        legacy.cp_ownership = None;
        legacy.c_structure = Some(CpStructureIdentity {
            level: 1,
            b_center_id: edge.b_center_id,
            departure_move_id: edge.cp_departure_move_id,
            terminal_move_id: None,
            source_start: edge.cp_source_start,
            source_end: None,
        });

        let entries = relaxed_cand_delta_entries(1, &[object], std::slice::from_ref(&legacy));
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].origin,
            CandDeltaEntryOrigin::ExistingCandDeltaFalse
        );
        assert!(!legacy.cand_delta, "P53 不改写算法背驰历史真值");
    }

    /// P53 回归：放宽函数只读既有事件；原 E 中成功对象继续纳入，几何失败对象仍留在
    /// classification-review 侧，不删除、不改字段。
    #[test]
    fn p53_preserves_existing_cand_delta_event_records() {
        let center = Center {
            zd: 4_180_000_000_000,
            zg: 4_210_000_000_000,
            dd: 4_170_000_000_000,
            gg: 4_220_000_000_000,
            start_index: 3_303_342,
            end_index: 3_305_431,
        };
        let closed = close_fixture_object(
            center,
            eid(2, 1508),
            eid(1, 6703),
            eid(1, 6704),
            unit(
                3_305_536,
                3_306_324,
                up(),
                4_180_000_000_000,
                4_300_000_000_000,
            ),
            unit(
                3_306_330,
                3_306_426,
                down(),
                4_220_000_000_000,
                4_300_000_000_000,
            ),
        );
        let pending = pending_cp_object(center, eid(2, 1509), eid(1, 6705), (3_306_500, 3_306_600));
        let legacy = vec![
            legacy_event_for_edge(
                1,
                CandDeltaCpEdge {
                    b_center_id: eid(2, 1508),
                    cp_departure_move_id: eid(1, 6703),
                    cp_source_start: 3_305_536,
                },
                true,
            ),
            legacy_event_for_edge(
                1,
                CandDeltaCpEdge {
                    b_center_id: eid(2, 1509),
                    cp_departure_move_id: eid(1, 6705),
                    cp_source_start: 3_306_500,
                },
                true,
            ),
        ];
        let before = legacy.clone();

        let entries = relaxed_cand_delta_entries(1, &[closed, pending], &legacy);
        assert_eq!(legacy, before, "原 CandDeltaEvent 记录必须 bit-exact 保留");
        assert_eq!(entries.len(), 1, "只有几何闭合对象进入放宽后的 E");
        assert_eq!(
            entries[0].origin,
            CandDeltaEntryOrigin::ExistingCandDeltaTrue
        );
        assert_eq!(entries[0].cp_ownership.b_center_id, eid(2, 1508));
    }
}
