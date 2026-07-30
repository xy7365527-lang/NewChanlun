//! C2 D1 投影域：`ExactThreeProjection` seed 构造 + `LowerLeg` 下级走势腿投影（#630 从
//! `level_view.rs` 拆出，纯移动零语义；来源票 #497 影子评审 MEDIUM-1）。

use super::super::super::types::{Center, Direction, Segment, Tick};
use super::super::center::{
    center_from_segments, center_from_window, compute_dd, compute_gg, UnitRange,
};
use super::super::recursive_tower::{ElementId, LeveledMove};
use super::ProviderVersion;

/// seed 核来源显式打标（#90 结裁 `chanlun/escalate/silent-dual-core-c1-seam-ruling-20260715.md`，
/// 执行 codex-decide-20260704 :143 边界条件"不能伪装成普通 seed 中枢"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedCoreProvenance {
    /// offset-0 自核 ≡ compose 携带核（普通窗 + 重切首子窗；#89 审计零反例域）。
    SelfConsistent,
    /// #148 升级重切子窗：seed [ZD,ZG] = 父窗继承核（塔携带，准绳）；`self_core` 保留
    /// offset-0 自核值仅供审计，生产消费一律走 `ExactThreeSeed::center`。
    InheritedRecut { self_core: Center },
    /// #95（仅 `EXTENDED_TO_EXACT_THREE_V3` 产出）：offset-0 自核不存在（V2 InvalidSeed 域，
    /// A3 定格分型 `ZD>ZG` 核空或 L1 方向不交替），seed [ZD,ZG] = 塔 compose 携带核；
    /// 无自核值可留档，故无 `self_core` 字段——不伪装成普通 seed（codex :143 边界条件同源）。
    CarriedOnly,
}

/// 扩展窗口投影后的不可变 seed；只复制前三个次级别走势的确定值，不保留可变尾引用。
/// #90 结裁：`center` 的 [ZD,ZG] 为塔 compose 携带核（继承核准绳）；dd/gg 与坐标仍取
/// 首三段窗（#142 设计内外缘语义不动）；来源见 `core_provenance`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactThreeSeed {
    pub source_id: ElementId,
    pub source_sub_count: usize,
    pub start_index: usize,
    pub end_index: usize,
    pub center: Center,
    pub core_provenance: SeedCoreProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactThreeProjection {
    pub version: ProviderVersion,
    pub seeds: Vec<ExactThreeSeed>,
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectionMaterial<'a> {
    /// 未经 D1 投影的当前扩展窗口；seam 必须拒绝。
    Unprojected(&'a [LeveledMove]),
    ExactThree(&'a ExactThreeProjection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    TooShort {
        index: usize,
        sub_count: usize,
    },
    InvalidSeed {
        index: usize,
    },
    InvalidLowerLeg {
        index: usize,
    },
    /// #90 结裁 fail-closed：窗口非 Compose 或 compose 未携带核——不得回退 offset-0 自核。
    MissingCarriedCenter {
        index: usize,
    },
}

fn first_leaf_direction(value: &LeveledMove) -> Option<Direction> {
    let mut node = &value.rmove;
    loop {
        match node {
            super::super::descend::RMove::Segment { direction, .. } => return Some(*direction),
            super::super::descend::RMove::Compose { subs, .. } => match subs.first() {
                Some(first) => node = first,
                None => return None,
            },
        }
    }
}

fn as_unit(value: &LeveledMove) -> Option<UnitRange> {
    Some(UnitRange {
        start_index: value.start_index,
        end_index: value.end_index,
        direction: first_leaf_direction(value)?,
        lo: value.rmove.lo(),
        hi: value.rmove.hi(),
    })
}

/// D1 选项 A：显式把扩展窗口投影到 immutable exact-three seed（钉 V2）。
/// #96 迁移后：生产 tuple 已切 V3（`auto_pairing()`/`carried_only()`/`pairing_disabled()`），
/// 本入口按裁决保留为**历史基准锚与 A/B 参照**（InvalidSeed fail-closed 语义不变）；
/// 与 V3 tuple 混用会在 query 侧被 `ProjectionVersionMismatch` 显式拒绝。
pub fn project_extended_windows(
    windows: &[LeveledMove],
) -> Result<ExactThreeProjection, ProjectionError> {
    project_extended_windows_impl(windows, ProviderVersion::EXTENDED_TO_EXACT_THREE_V2)
}

/// #95 D1 CarriedOnly 版本化迁移（V3）：与 V2 的**唯一**分歧点是 own offset-0 自核不存在时
/// 不再 InvalidSeed fail-closed，而是取塔 compose 携带核为 seed [ZD,ZG]（外缘 dd/gg 与坐标
/// 仍按 V2 同一公式取首三段窗），显式打标 `SeedCoreProvenance::CarriedOnly`。
/// own 自核存在的窗与 V2 逐位一致（p95 A/B 探针全量核验）。
pub fn project_extended_windows_carried_only(
    windows: &[LeveledMove],
) -> Result<ExactThreeProjection, ProjectionError> {
    project_extended_windows_impl(windows, ProviderVersion::EXTENDED_TO_EXACT_THREE_V3)
}

fn project_extended_windows_impl(
    windows: &[LeveledMove],
    version: ProviderVersion,
) -> Result<ExactThreeProjection, ProjectionError> {
    let mut seeds = Vec::with_capacity(windows.len());
    for (index, window) in windows.iter().enumerate() {
        let count = window.sub_moves.len();
        if count < 3 {
            return Err(ProjectionError::TooShort {
                index,
                sub_count: count,
            });
        }
        let units = [
            as_unit(&window.sub_moves[0]),
            as_unit(&window.sub_moves[1]),
            as_unit(&window.sub_moves[2]),
        ];
        let [Some(a), Some(b), Some(c)] = units else {
            return Err(ProjectionError::InvalidSeed { index });
        };
        let own_center = if window.id.level == 1 {
            center_from_segments(&a, &b, &c)
        } else {
            center_from_window(&a, &b, &c)
        };
        // 条款 3（#90 结裁）：V2 下 offset-0 自核不成立 ⟹ InvalidSeed 维持 D1 选项 0
        // fail-closed。#95 迁移裁定（选项 1）：V3 下该域改走携带核收复，见下方 CarriedOnly 分支。
        if own_center.is_none() && version != ProviderVersion::EXTENDED_TO_EXACT_THREE_V3 {
            return Err(ProjectionError::InvalidSeed { index });
        }
        // 条款 1（#90 结裁）：准绳 = 塔 compose 携带核（#89 已证与重算 detect 逐窗 bit-equal）。
        let carried = match &window.rmove {
            super::super::descend::RMove::Compose { centers, .. } => centers.first().copied(),
            super::super::descend::RMove::Segment { .. } => None,
        }
        .ok_or(ProjectionError::MissingCarriedCenter { index })?;
        let (center, core_provenance) = match own_center {
            // 条款 2（#90 结裁）：两核不等禁静默——seed [ZD,ZG] 取继承核，自核值显式留档。
            Some(own_center) => {
                if own_center.zd == carried.zd && own_center.zg == carried.zg {
                    (own_center, SeedCoreProvenance::SelfConsistent)
                } else {
                    (
                        Center {
                            zd: carried.zd,
                            zg: carried.zg,
                            ..own_center
                        },
                        SeedCoreProvenance::InheritedRecut {
                            self_core: own_center,
                        },
                    )
                }
            }
            // #95（V3 独占分支）：自核不存在，seed 核 = 携带核；外缘/坐标与 V2 同公式。
            None => (
                Center {
                    zd: carried.zd,
                    zg: carried.zg,
                    dd: compute_dd(&a, &b, &c),
                    gg: compute_gg(&a, &b, &c),
                    start_index: a.start_index,
                    end_index: c.end_index,
                },
                SeedCoreProvenance::CarriedOnly,
            ),
        };
        seeds.push(ExactThreeSeed {
            source_id: window.id,
            source_sub_count: count,
            start_index: a.start_index,
            end_index: c.end_index,
            center,
            core_provenance,
        });
    }
    Ok(ExactThreeProjection { version, seeds })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowerLeg {
    pub id: ElementId,
    pub direction: Direction,
    pub start_index: usize,
    pub end_index: usize,
    pub lo: Tick,
    pub hi: Tick,
}

pub fn lower_legs_from(values: &[LeveledMove]) -> Result<Vec<LowerLeg>, ProjectionError> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            Ok(LowerLeg {
                id: value.id,
                direction: first_leaf_direction(value)
                    .ok_or(ProjectionError::InvalidLowerLeg { index })?,
                start_index: value.start_index,
                end_index: value.end_index,
                lo: value.rmove.lo(),
                hi: value.rmove.hi(),
            })
        })
        .collect()
}

pub(super) fn leg_as_segment(value: &LowerLeg) -> Segment {
    let (start_price, end_price) = match value.direction {
        Direction::Up => (value.lo, value.hi),
        Direction::Down => (value.hi, value.lo),
    };
    Segment {
        direction: value.direction,
        start_index: value.start_index,
        end_index: value.end_index,
        start_price,
        end_price,
    }
}
