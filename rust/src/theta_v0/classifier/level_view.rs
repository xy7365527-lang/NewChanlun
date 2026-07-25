//! C2 CompletedMove 消费 seam（tasks #73/#74）。
//!
//! 本模块是只读消费层，不改写递归塔。D1 要求调用方显式把当前扩展窗口投影为不可变
//! exact-three seed；D2 只消费 D3 已结裁的 `MoveBlock.dir: Option<Direction>`。四个旧 seam
//! provider 不在本模块出现，方向版本唯一绑定 `central-ggdd-v1`。

use super::super::types::{Bar, Center, Direction, Fractal, MoveKind, Segment, Side, Tick};
use super::center::{center_from_segments, center_from_window, compute_dd, compute_gg, UnitRange};
use super::decompose::{center_block_kind, MoveBlock, MoveStatus};
use super::divergence::{
    departure_move_c_start, dif_crosses_zero, locate_departure_move_a,
    move_range_envelope as range_envelope, same_color_area, same_dir_hist_peak, segment_dif_peak,
    segments_diverge_or, self_anchors,
};
use super::recursive_tower::{map_src_to_close_idx, ElementId, LeveledMove};
use super::signal::{
    locate_pan_div_structure, locate_pan_div_structure_front_anchor,
    nearest_confirmed_center_idx, pan_div_structure_extreme, trend_third_class_in_c,
};
use std::collections::BTreeSet;

/// #73-#75 独立激活门。默认关闭，现有分类/交易路径不调用本 seam。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C2LevelViewConfig {
    pub enabled: bool,
}

impl Default for C2LevelViewConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProviderVersion(pub &'static str);

impl ProviderVersion {
    pub const CENTRAL_GGDD_V1: Self = Self("central-ggdd-v1");
    /// 已废止（#90 结裁 `chanlun/escalate/silent-dual-core-c1-seam-ruling-20260715.md`）：
    /// V1 对 #148 重切子窗跑 offset-0 自核并静默采用（3,169 窗静默双核）。仅留作历史基准锚，
    /// `validate()` 不再接受。
    pub const EXTENDED_TO_EXACT_THREE_V1: Self = Self("extended-to-exact-three-v1");
    /// #90 结裁：seed [ZD,ZG] 改读塔 compose 携带核（准绳=继承核，定理一核心冻结在父窗），
    /// dd/gg/身份不动；两核不等显式打标 `SeedCoreProvenance::InheritedRecut`（codex :143 边界条件）。
    pub const EXTENDED_TO_EXACT_THREE_V2: Self = Self("extended-to-exact-three-v2-inherited-core");
    /// #95 版本化迁移（裁定 `chanlun/escalate/d1-invalidseed-carriedonly-migration-ruling-20260716.md`
    /// 选项 1，#90 条款 3 独立立项）：own offset-0 自核不存在（V2 InvalidSeed 域，#84 审计
    /// 215 窗全部 A3 定格分型核空）且塔 compose 携带核存在时，seed [ZD,ZG] = 携带核，
    /// 显式打标 `SeedCoreProvenance::CarriedOnly`；其余窗与 V2 逐位一致（p95 A/B 探针核验）。
    /// #96 结裁（2026-07-16 用户"按原文裁决"，原文回查记录见同名裁决文件）：Q1=选项 1 定稿，
    /// 生产 tuple（`auto_pairing`/`pairing_disabled`）钉 V3；V2 保留为历史基准锚与 A/B 参照。
    pub const EXTENDED_TO_EXACT_THREE_V3: Self = Self("extended-to-exact-three-v3-carried-only");
    pub const MOVE_BLOCK_AC_V1: Self = Self("move-block-ac-v1");
    /// 显式关闭也是一个完整版本值；它不是缺字段，且保持旧 Pending 行为。
    pub const DISABLED_V1: Self = Self("disabled-v1");
}

/// D1/D2 version tuple。三个字段保留 `Option`，使缺字段可以被 seam 明确拒绝而非靠默认补齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct C2VersionTuple {
    pub direction_provider_version: Option<ProviderVersion>,
    pub divergence_pair_provider_version: Option<ProviderVersion>,
    pub projection_provider_version: Option<ProviderVersion>,
}

impl C2VersionTuple {
    /// #96 结裁后生产钉版 = V3（CarriedOnly 收复域生效；其余窗与 V2 逐位一致，p95 已证）。
    pub const fn auto_pairing() -> Self {
        Self {
            direction_provider_version: Some(ProviderVersion::CENTRAL_GGDD_V1),
            divergence_pair_provider_version: Some(ProviderVersion::MOVE_BLOCK_AC_V1),
            projection_provider_version: Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V3),
        }
    }

    pub const fn pairing_disabled() -> Self {
        Self {
            direction_provider_version: Some(ProviderVersion::CENTRAL_GGDD_V1),
            divergence_pair_provider_version: Some(ProviderVersion::DISABLED_V1),
            projection_provider_version: Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V3),
        }
    }

    /// #95 CarriedOnly 迁移 tuple：仅 projection 升版 V3，方向/背驰 provider 不动。
    /// #96 结裁后与 `auto_pairing()` 等值，保留作显式命名入口（历史探针与文档引用它）。
    pub const fn carried_only() -> Self {
        Self {
            direction_provider_version: Some(ProviderVersion::CENTRAL_GGDD_V1),
            divergence_pair_provider_version: Some(ProviderVersion::MOVE_BLOCK_AC_V1),
            projection_provider_version: Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V3),
        }
    }

    pub fn validate(self) -> Result<(), VersionTupleError> {
        let direction = self
            .direction_provider_version
            .ok_or(VersionTupleError::Missing("direction_provider_version"))?;
        if direction != ProviderVersion::CENTRAL_GGDD_V1 {
            return Err(VersionTupleError::Mismatch {
                field: "direction_provider_version",
                expected: ProviderVersion::CENTRAL_GGDD_V1,
                actual: direction,
            });
        }
        let divergence =
            self.divergence_pair_provider_version
                .ok_or(VersionTupleError::Missing(
                    "divergence_pair_provider_version",
                ))?;
        if !matches!(
            divergence,
            ProviderVersion::MOVE_BLOCK_AC_V1 | ProviderVersion::DISABLED_V1
        ) {
            return Err(VersionTupleError::Mismatch {
                field: "divergence_pair_provider_version",
                expected: ProviderVersion::MOVE_BLOCK_AC_V1,
                actual: divergence,
            });
        }
        let projection = self
            .projection_provider_version
            .ok_or(VersionTupleError::Missing("projection_provider_version"))?;
        if !matches!(
            projection,
            ProviderVersion::EXTENDED_TO_EXACT_THREE_V2
                | ProviderVersion::EXTENDED_TO_EXACT_THREE_V3
        ) {
            return Err(VersionTupleError::Mismatch {
                field: "projection_provider_version",
                expected: ProviderVersion::EXTENDED_TO_EXACT_THREE_V2,
                actual: projection,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionTupleError {
    Missing(&'static str),
    Mismatch {
        field: &'static str,
        expected: ProviderVersion,
        actual: ProviderVersion,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoordinateWindow {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelViewQuery {
    pub level: u32,
    pub coordinate_window: CoordinateWindow,
    pub as_of: usize,
    pub version: C2VersionTuple,
}

/// 正式缓存键：同一查询与同一 version tuple 的规范串稳定；三 provider 字段全部入键。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct C2CacheKey(String);

impl C2CacheKey {
    pub fn from_query(query: &LevelViewQuery) -> Result<Self, VersionTupleError> {
        query.version.validate()?;
        let d = query
            .version
            .direction_provider_version
            .expect("validated")
            .0;
        let a = query
            .version
            .divergence_pair_provider_version
            .expect("validated")
            .0;
        let p = query
            .version
            .projection_provider_version
            .expect("validated")
            .0;
        Ok(Self(format!(
            "c2|L{}|{}..{}|asof={}|dir={}|pair={}|projection={}",
            query.level,
            query.coordinate_window.start,
            query.coordinate_window.end,
            query.as_of,
            d,
            a,
            p
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// CompletedFreeze 事件流键：跨 `as_of` 稳定，但仍完整 pin level/window 与三 provider 版本。
/// 单个 view 的结果缓存使用上面的 [`C2CacheKey`]（含 `as_of`），二者不可混用。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct C2PersistenceKey(String);

impl C2PersistenceKey {
    pub fn from_query(query: &LevelViewQuery) -> Result<Self, VersionTupleError> {
        query.version.validate()?;
        let d = query
            .version
            .direction_provider_version
            .expect("validated")
            .0;
        let a = query
            .version
            .divergence_pair_provider_version
            .expect("validated")
            .0;
        let p = query
            .version
            .projection_provider_version
            .expect("validated")
            .0;
        Ok(Self(format!(
            "c2-stream|L{}|{}..{}|dir={}|pair={}|projection={}",
            query.level, query.coordinate_window.start, query.coordinate_window.end, d, a, p
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
    TooShort { index: usize, sub_count: usize },
    InvalidSeed { index: usize },
    InvalidLowerLeg { index: usize },
    /// #90 结裁 fail-closed：窗口非 Compose 或 compose 未携带核——不得回退 offset-0 自核。
    MissingCarriedCenter { index: usize },
}

fn first_leaf_direction(value: &LeveledMove) -> Option<Direction> {
    let mut node = &value.rmove;
    loop {
        match node {
            super::descend::RMove::Segment { direction, .. } => return Some(*direction),
            super::descend::RMove::Compose { subs, .. } => match subs.first() {
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
            super::descend::RMove::Compose { centers, .. } => centers.first().copied(),
            super::descend::RMove::Segment { .. } => None,
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

fn leg_as_segment(value: &LowerLeg) -> Segment {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

fn structural_block_span(projection: &ExactThreeProjection, block: &MoveBlock) -> Option<(usize, usize)> {
    Some((
        projection.seeds.get(block.start_center)?.start_index,
        projection.seeds.get(block.end_center)?.end_index,
    ))
}

fn structural_pair_span(
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    leave_index: usize,
) -> Option<(usize, usize)> {
    let leave = blocks.get(leave_index)?;
    let retest = blocks.get(leave_index + 1)?;
    if leave.status != MoveStatus::Completed || retest.status != MoveStatus::Completed {
        return None;
    }
    let leave = structural_block_span(projection, leave)?;
    let retest = structural_block_span(projection, retest)?;
    Some((leave.0, retest.1))
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
fn trend_confirm_time(
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
    // T4（025:761）：B 中枢 span 内 DIF 变号或触 0（p112 主口径 cross_dif）。
    let (b_lo, b_hi) = map_src_to_close_idx(close_src, last.start_index, last.end_index)?;
    if !dif_crosses_zero(dif, b_lo, b_hi) {
        return None;
    }
    // b 段力度基准（031:883 趋势背驰 = c vs b 比较）。
    let a_idx = map_src_to_close_idx(close_src, seg_a.0, seg_a.1)?;
    let a_env = range_envelope(segments, seg_a)?;
    let area_a = same_color_area(hist, a_idx.0, a_idx.1, side);
    let dif_peak_a = segment_dif_peak(dif, a_idx.0, a_idx.1, direction);
    let hist_peak_a = same_dir_hist_peak(hist, a_idx.0, a_idx.1, side);
    // T3（037:18）：c 全离开段内含对 B 的三买；t3 = 首个命中回试段终点（c 确立时点）。
    let (_leave_end, t3) = trend_third_class_in_c(segments, last, direction, c_start, as_of)?;
    // 渐进扫描 t ≥ t3：T2/T5 窗口 [c_start, t] 随段端点增量扩展。
    let lo = segments.partition_point(|s| s.start_index < c_start.max(last.end_index));
    let hi = segments.partition_point(|s| s.end_index <= as_of);
    let mut env: Option<(Tick, Tick)> = None;
    let mut acc_hi: Option<usize> = None; // 已累入力度窗口的末个 close 下标
    let mut area_c = 0.0f64;
    let (mut dif_max, mut dif_min) = (f64::NEG_INFINITY, f64::INFINITY);
    let mut hist_max = f64::NEG_INFINITY;
    let mut hist_min = f64::INFINITY;
    for leg in &segments[lo..hi] {
        // 包络增量扩展（T2）。
        let leg_lo = leg.start_price.min(leg.end_price);
        let leg_hi = leg.start_price.max(leg.end_price);
        env = Some(match env {
            None => (leg_lo, leg_hi),
            Some((old_lo, old_hi)) => (old_lo.min(leg_lo), old_hi.max(leg_hi)),
        });
        // 力度窗口增量扩展（T5）：新 bar 区间 (acc_hi, cur_hi] 累入。
        if let Some((c_lo, cur_hi)) = map_src_to_close_idx(close_src, c_start, leg.end_index) {
            let from = acc_hi.map_or(c_lo, |prev| prev + 1);
            if from <= cur_hi {
                for t in from..=cur_hi {
                    match side {
                        Side::Long if hist[t] < 0.0 => area_c += hist[t].abs(),
                        Side::Short if hist[t] > 0.0 => area_c += hist[t],
                        _ => {}
                    }
                    hist_max = hist_max.max(hist[t]);
                    hist_min = hist_min.min(hist[t]);
                    dif_max = dif_max.max(dif[t]);
                    dif_min = dif_min.min(dif[t]);
                }
                acc_hi = Some(cur_hi);
            }
        }
        if leg.end_index < t3 {
            continue; // 三买未确立前不判（037:18 必要合取）。
        }
        // t3 时点力度窗口无 bar ⟹ 力度不可验 ⟹ 诚实判负（同旧 map None ⟹ false）。
        acc_hi?;
        // T5 力度或关系（027:32）：同色面积 ∨ 黄白线峰 ∨ 同向柱峰（增量量与
        // same_color_area / segment_dif_peak / same_dir_hist_peak 同口径）。
        let dif_peak_c = match direction {
            Direction::Up => dif_max.max(0.0),
            Direction::Down => dif_min.min(0.0).abs(),
        };
        let hist_peak_c = match side {
            Side::Long => hist_min.min(0.0).abs(),
            Side::Short => hist_max.max(0.0),
        };
        let force_ok =
            area_c < area_a || dif_peak_c < dif_peak_a || hist_peak_c < hist_peak_a;
        if !force_ok {
            return None; // 各 proxy 只增 ⟹ 真→假单调，转假即终假（033:26 无衰减即无背驰）。
        }
        // T2 破极值（037:20）：c 包络破 b 包络。
        let (env_lo, env_hi) = env.expect("扫描窗口非空（t3 已命中）");
        let extreme = match side {
            Side::Long => env_lo < a_env.0,
            Side::Short => env_hi > a_env.1,
        };
        if extreme {
            return Some(leg.end_index); // t* = 首个 T2∧T5-OR 同真时点。
        }
    }
    None
}

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
    provide_nest_candidate_events_ext(level, projection, blocks, legs, view, hist, dif, close_src, &[], &[])
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
fn resolve_triple_anchor(
    x: usize,
    fractals: &[Fractal],
    merged_bars: &[Bar],
) -> (Option<Tick>, Option<usize>) {
    let price = super::super::parser::fractal::fractal_at_source(fractals, x).map(|f| f.price);
    let anchor = super::super::parser::inclusion::merged_group_anchor(merged_bars, x);
    (price, anchor)
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
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let anchors_self: Vec<_> = segments.iter().map(|segment| Some(segment.direction)).collect();
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
        let confirm_t = trend_confirm_time(
            &segments,
            last_center,
            pair.id.direction,
            side,
            pair.seg_a,
            pair.seg_c.0,
            view.query.as_of,
            hist,
            dif,
            close_src,
        );
        let (interval_b, turn_source, divergence_confirmed) = match confirm_t {
            Some(t) => ((pair.seg_c.0, t), t, true),
            None => (pair.seg_c, pair.seg_c.1, false),
        };
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
                provider_window: (view.query.coordinate_window.start, view.query.coordinate_window.end),
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
    for segment in segments.iter().filter(|segment| segment.end_index <= view.query.as_of) {
        let Some(center_index) = nearest_confirmed_center_idx(&centers, segment.start_index) else {
            continue;
        };
        if kinds.get(center_index) != Some(&Some(MoveKind::Consolidation)) {
            continue;
        }
        // ★R3：窄锚（中枢后前次同向破核心段）locate∧Extreme 优先；任一失败回退 A′（中枢前
        // 最近同向段，061:28 中枢两头比较，回中枢要件由中枢本身满足）重判 Extreme（044:234 维持）。
        let Some(structure) = locate_pan_div_structure(
            &centers[center_index], segment, &segments, &anchors_self,
        )
        .filter(|structure| pan_div_structure_extreme(structure, &segments))
        .or_else(|| {
            locate_pan_div_structure_front_anchor(
                &centers[center_index], segment, &segments, &anchors_self,
            )
            .filter(|structure| pan_div_structure_extreme(structure, &segments))
        }) else {
            continue;
        };
        let Some(leave_index) = blocks.iter().position(|block| {
            block.kind == MoveKind::Consolidation
                && block.start_center <= center_index
                && block.end_center >= center_index
        }) else {
            continue;
        };
        // #97 进料口（⑤「盘背入链」落地缺口补齐）：leave→retest 对不可用（如盘整块为末块、
        // 离开块未 Completed）时不再丢弃候选；interval_a 仅供 A 口径诊断，回填为盘整块自身
        // 结构跨度（再兜底 seg_a.0..seg_c.1），B 生产口径（interval_b）不受影响。
        let (interval_a, intake_fallback) = match structural_pair_span(projection, blocks, leave_index) {
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
        let divergence_confirmed = match (
            map_src_to_close_idx(close_src, structure.seg_a.0, structure.seg_a.1),
            map_src_to_close_idx(close_src, structure.seg_c.0, structure.seg_c.1),
        ) {
            (Some(a), Some(c)) => segments_diverge_or(hist, dif, structure.side, a, c),
            _ => false,
        };
        let event = NestCandidateEvent {
            level,
            side: structure.side,
            kind: NestDivergenceKind::Consolidation,
            seg_a: structure.seg_a,
            interval_b: structure.seg_c,
            interval_a,
            divergence_confirmed,
            turn_source: structure.source_index,
            judge_at: view.query.as_of,
            provider_window: (view.query.coordinate_window.start, view.query.coordinate_window.end),
            intake_fallback,
            // 关③ P3：B = 盘背结构所对的最近确认中枢（locate_pan_div_structure 同一中枢
            // 入参）——同写归因用途，不作门（P2 盘背域无需 owner 合取）。
            b_center_start: centers[center_index].start_index,
        };
        // 去重键 = 事件本体（与旧 `out.contains(&event)` 逐字同语义；锚 sidecar 不进键）。
        if !out.iter().any(|ext| ext.event == event) {
            // T1 (#170)：两元锚（极值价, 组锚）在事件构造点解析（单一查法；方向分量
            // 已随 T5a (#207) 退役，#206 Q1 裁定）。
            let (extreme_price, group_anchor) =
                resolve_triple_anchor(structure.seg_c.1, fractals, merged_bars);
            out.push(NestCandidateEventExt { event, extreme_price, group_anchor });
        }
    }
    out.sort_by_key(|ext| (
        ext.event.turn_source,
        ext.event.interval_b,
        ext.event.kind,
        matches!(ext.event.side, Side::Short),
    ));
    out
}

/// D2 provider：只读 `MoveBlock.dir`。盘整 `None` 严格产零 pair；A/C 是同趋势方向、
/// 分属相邻两个中心锚后的离开 episode，定界复用既有 divergence episode helpers。
pub fn provide_divergence_pairs(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    as_of: usize,
) -> Vec<DivergencePair> {
    let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
    let anchors = self_anchors(&segments);
    let mut pairs = Vec::new();
    for block in blocks {
        let Some(direction) = block.dir else {
            continue;
        };
        if block.end_center <= block.start_center || block.end_center >= projection.seeds.len() {
            continue;
        }
        let prev = projection.seeds[block.end_center - 1].center;
        let last = projection.seeds[block.end_center].center;
        let Some(seg_a) = locate_departure_move_a(&segments, &anchors, &prev, &last, direction)
        else {
            continue;
        };
        // #104/#105 裁定修复（di-trend-c-terminal-forward-find-ruling-20260716）：
        // C 终段 = 离开最后中枢后的**第一个**同向段（正向 find）。此前 rev().find 取
        // 全域最后一个同向段，最终快照下 C 段被锚到数据末端，Trend 背驰恒 false。
        let Some(c_terminal) = segments.iter().find(|segment| {
            segment.direction == direction
                && segment.start_index >= last.end_index
                && segment.end_index <= as_of
        }) else {
            continue;
        };
        let Some(c_start) = departure_move_c_start(
            &segments,
            &anchors,
            &last,
            direction,
            c_terminal.start_index,
        ) else {
            continue;
        };
        // ★R1（2026-07-17 代理裁定，p112 实证）：seg_c 取段 = **全离开段**——c_end 从首个
        // 同向段终点扩展为 c_start 起、end ≤ as_of 的**末个同向段**终点（037:22 全离开走势
        // 口径，p112 T3_ext 窗口）。#105 的单腿窗口（win_legs==1 @154/154）使 037:18 三买
        // 构造性无处容身；全离开段下 c 内含回拉段，三买可判。单段离开的退化名与新名逐位一致
        // （c_end == c_terminal.end）。确认时点（t*）收束在 trend_confirm_time 消费侧完成。
        let Some(c_end) = segments
            .iter()
            .rev()
            .find(|segment| {
                segment.direction == direction
                    && segment.start_index >= c_start
                    && segment.end_index <= as_of
            })
            .map(|segment| segment.end_index)
        else {
            continue;
        };
        pairs.push(DivergencePair {
            id: DivergencePairId {
                level,
                block_start_center: block.start_center,
                block_end_center: block.end_center,
                direction,
            },
            move_start: projection.seeds[block.start_center].start_index,
            seg_a,
            seg_c: (c_start, c_end),
        });
    }
    pairs
}

#[derive(Debug, Clone, Copy)]
pub struct LevelViewMaterial<'a> {
    pub projection: ProjectionMaterial<'a>,
    pub move_blocks: &'a [MoveBlock],
    pub lower_legs: &'a [LowerLeg],
    pub hist: &'a [f64],
    /// ★R1（2026-07-17 裁定）：全合取确认需黄白线——T4 回拉 0 轴（025:761）与力度或关系的
    /// 黄白线分量（026:521）都读 dif。与 `hist` 同源同坐标系（`compute_macd` 的 dif 序列）。
    pub dif: &'a [f64],
    pub close_src: &'a [usize],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingReason {
    AwaitingSubsequentMove,
    MissingDivergencePair,
    MissingMacdCoordinates,
    TerminalLegNotDivergent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionEvidence {
    SubsequentMove,
    TerminalDivergence { pair_id: DivergencePairId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionStatus {
    Completed {
        as_of: usize,
        evidence: CompletionEvidence,
    },
    Pending {
        as_of: usize,
        reason: PendingReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledMove {
    pub start_index: usize,
    pub end_index: usize,
    pub direction: Option<Direction>,
    pub kind: MoveKind,
    pub completion: CompletionStatus,
    pub center_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelAsOfView {
    pub query: LevelViewQuery,
    pub cache_key: C2CacheKey,
    pub moves: Vec<AssembledMove>,
    pub pairs: Vec<DivergencePair>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelViewError {
    FeatureDisabled,
    InvalidVersionTuple(VersionTupleError),
    UnprojectedInput { window_count: usize },
    ProjectionVersionMismatch(ProviderVersion),
    BlockOutOfRange { block_index: usize },
}

/// C2 唯一公开 seam。任何错误都在构造输出前返回，故 fail-closed 时不可能泄漏 CompletedMove。
pub fn assemble_level_view(
    config: C2LevelViewConfig,
    query: LevelViewQuery,
    material: LevelViewMaterial<'_>,
) -> Result<LevelAsOfView, LevelViewError> {
    if !config.enabled {
        return Err(LevelViewError::FeatureDisabled);
    }
    query
        .version
        .validate()
        .map_err(LevelViewError::InvalidVersionTuple)?;
    let projection = match material.projection {
        ProjectionMaterial::Unprojected(windows) => {
            return Err(LevelViewError::UnprojectedInput {
                window_count: windows.len(),
            })
        }
        ProjectionMaterial::ExactThree(value) => value,
    };
    // #95：projection 版本须与 query tuple 声明的 projection provider 逐字一致
    // （validate() 已把合法域钉在 V2/V3）。V2 查询行为与迁移前逐位不变；V3 仅显式 opt-in，
    // 且 V2 查询 + V3 投影（或反之）在此被拒绝——不存在静默混版。
    if Some(projection.version) != query.version.projection_provider_version {
        return Err(LevelViewError::ProjectionVersionMismatch(
            projection.version,
        ));
    }
    for (block_index, block) in material.move_blocks.iter().enumerate() {
        if block.start_center > block.end_center || block.end_center >= projection.seeds.len() {
            return Err(LevelViewError::BlockOutOfRange { block_index });
        }
    }

    let cache_key = C2CacheKey::from_query(&query).map_err(LevelViewError::InvalidVersionTuple)?;
    let pairs = if query.version.divergence_pair_provider_version
        == Some(ProviderVersion::MOVE_BLOCK_AC_V1)
    {
        provide_divergence_pairs(
            query.level,
            projection,
            material.move_blocks,
            material.lower_legs,
            query.as_of,
        )
    } else {
        Vec::new()
    };

    // Trend 完成判定（R1 全合取扫描）消费的段序列——与 provide_nest_candidate_events 同一投影。
    let segments: Vec<_> = material.lower_legs.iter().map(leg_as_segment).collect();

    let mut moves = Vec::with_capacity(material.move_blocks.len());
    for block in material.move_blocks {
        let seed_slice = &projection.seeds[block.start_center..=block.end_center];
        let start_index = seed_slice.first().expect("validated nonempty").start_index;
        let end_index = seed_slice.last().expect("validated nonempty").end_index;
        let completion = match block.kind {
            MoveKind::Consolidation => {
                if block.status == MoveStatus::Completed {
                    CompletionStatus::Completed {
                        as_of: query.as_of,
                        evidence: CompletionEvidence::SubsequentMove,
                    }
                } else {
                    CompletionStatus::Pending {
                        as_of: query.as_of,
                        reason: PendingReason::AwaitingSubsequentMove,
                    }
                }
            }
            MoveKind::Trend => {
                let pair = pairs.iter().find(|pair| pair.move_start == start_index);
                match pair {
                    None => CompletionStatus::Pending {
                        as_of: query.as_of,
                        reason: PendingReason::MissingDivergencePair,
                    },
                    Some(pair) => {
                        // ★R1（2026-07-17 裁定）：TerminalDivergence 与 nest 趋势事件确认共用
                        // 同一全合取谓词（trend_confirm_time，单一来源——T4 回拉0轴 ∧ T3 三买
                        // ∧ T2 破极值 ∧ T5 力度或关系，确认时点 = 首个全成立时点，禁前视）。
                        // 旧面积单通道（segments_diverge on seg_c）随 seg_c 全离开段化废止。
                        let mappable = map_src_to_close_idx(
                            material.close_src,
                            pair.seg_a.0,
                            pair.seg_a.1,
                        )
                        .is_some()
                            && map_src_to_close_idx(
                                material.close_src,
                                pair.seg_c.0,
                                pair.seg_c.1,
                            )
                            .is_some();
                        if !mappable {
                            CompletionStatus::Pending {
                                as_of: query.as_of,
                                reason: PendingReason::MissingMacdCoordinates,
                            }
                        } else {
                            let direction = block
                                .dir
                                .expect("Trend 块必有 dir（decompose 不变量，pair 存在性已证）");
                            let side = match direction {
                                Direction::Down => Side::Long,
                                Direction::Up => Side::Short,
                            };
                            let last = &projection.seeds[block.end_center].center;
                            match trend_confirm_time(
                                &segments,
                                last,
                                direction,
                                side,
                                pair.seg_a,
                                pair.seg_c.0,
                                query.as_of,
                                material.hist,
                                material.dif,
                                material.close_src,
                            ) {
                                Some(_) => CompletionStatus::Completed {
                                    as_of: query.as_of,
                                    evidence: CompletionEvidence::TerminalDivergence {
                                        pair_id: pair.id,
                                    },
                                },
                                None => CompletionStatus::Pending {
                                    as_of: query.as_of,
                                    reason: PendingReason::TerminalLegNotDivergent,
                                },
                            }
                        }
                    }
                }
            }
        };
        moves.push(AssembledMove {
            start_index,
            end_index,
            direction: block.dir,
            kind: block.kind,
            completion,
            center_indices: (block.start_center..=block.end_center).collect(),
        });
    }
    Ok(LevelAsOfView {
        query,
        cache_key,
        moves,
        pairs,
    })
}

pub fn completed_move_starts(view: &LevelAsOfView) -> BTreeSet<usize> {
    view.moves
        .iter()
        .filter(|value| matches!(value.completion, CompletionStatus::Completed { .. }))
        .map(|value| value.start_index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::super::types::{FractalKind, MoveKind};
    use super::super::recursive_tower::LeveledMove;
    use super::*;

    fn unit(start: usize, dir: Direction, lo: Tick, hi: Tick, ordinal: u64) -> LeveledMove {
        LeveledMove::from_unit(
            &UnitRange {
                start_index: start,
                end_index: start + 9,
                direction: dir,
                lo,
                hi,
            },
            ElementId { level: 0, ordinal },
        )
    }

    fn extended_windows() -> (Vec<LeveledMove>, Vec<LeveledMove>) {
        use Direction::{Down, Up};
        let lower = vec![
            unit(0, Up, 90, 110, 0),
            unit(10, Down, 95, 115, 1),
            unit(20, Up, 98, 112, 2),
            unit(30, Down, 96, 116, 3),
            unit(40, Up, 130, 145, 4),
            unit(50, Down, 132, 148, 5),
            unit(60, Up, 135, 150, 6),
            unit(70, Down, 125, 140, 7),
            unit(80, Up, 155, 170, 8),
            unit(90, Down, 150, 165, 9),
            unit(100, Up, 160, 175, 10),
            unit(110, Down, 140, 155, 11),
            unit(120, Up, 180, 190, 12),
            // ★R1 夹具补腿：回试段（Down，低点 170 > w2.zg=165 不重回核心）——与腿12（Up，
            // 端点 190 > 165 破核心）构成对 w2 的第三类买点（037:18），供全合取确认测试；
            // 旧 as_of=129 的既有测试按 end ≤ as_of 过滤本腿，行为不变。
            unit(130, Down, 170, 195, 13),
        ];
        let w0 = LeveledMove::compose(
            &lower[0..4],
            Center {
                zd: 98,
                zg: 110,
                dd: 90,
                gg: 116,
                start_index: 0,
                end_index: 39,
            },
            1,
            ElementId {
                level: 1,
                ordinal: 0,
            },
        );
        let w1 = LeveledMove::compose(
            &lower[4..8],
            Center {
                zd: 135,
                zg: 140,
                dd: 125,
                gg: 150,
                start_index: 40,
                end_index: 79,
            },
            1,
            ElementId {
                level: 1,
                ordinal: 1,
            },
        );
        let w2 = LeveledMove::compose(
            &lower[8..12],
            Center {
                zd: 160,
                zg: 165,
                dd: 140,
                gg: 175,
                start_index: 80,
                end_index: 119,
            },
            1,
            ElementId {
                level: 1,
                ordinal: 2,
            },
        );
        (vec![w0, w1, w2], lower)
    }

    // ───────────── T1 (#170) 三元锚供给/携带测试（先红后绿） ─────────────

    fn mbar(source_index: usize) -> Bar {
        Bar {
            source_index,
            timestamp: source_index as i64,
            open: 0,
            high: 0,
            low: 0,
            close: 0,
            volume: 0,
            untradable: false,
        }
    }

    /// 锚解析口径锁：极值价 = 拐点处 L0 分型极值（`Fractal.price`，整数 tick 精确等值，
    /// 不设 kind 守卫——高级别走势可以次级别反向段收束，x 在 L0 的分型方向可与事件 side
    /// 不同，但 x 处实际打印价跨级不变）；组锚 = 该级包含层组内首根序号。
    #[test]
    fn triple_anchor_resolution_caliber() {
        let fractals = vec![
            Fractal { kind: FractalKind::Bottom, source_index: 10, timestamp: 10, price: 100 },
            // W 底第二脚：同向同价（底, 100），与第一脚分属不同合并组。
            Fractal { kind: FractalKind::Bottom, source_index: 30, timestamp: 30, price: 100 },
            Fractal { kind: FractalKind::Top, source_index: 50, timestamp: 50, price: 190 },
        ];
        // 合并组：g0 = raw [0,20)（锚 0）；g1 = raw [20,50)（锚 20）；g2 = raw [50,∞)（锚 50）。
        let merged = vec![mbar(0), mbar(20), mbar(50)];
        // W 底双脚：同向同价由合并组区分——锚不同 ⟹ 键不同（教义裁定 4）。
        let (p1, a1) = resolve_triple_anchor(10, &fractals, &merged);
        let (p2, a2) = resolve_triple_anchor(30, &fractals, &merged);
        assert_eq!((p1, a1), (Some(100), Some(0)));
        assert_eq!((p2, a2), (Some(100), Some(20)));
        assert_ne!(a1, a2, "同向同价碰撞由合并组区分（教义裁定 4）");
        // 跨级不变量构造锁：解析只读（x, 供给），不读级别几何——同一 x 在任意级别查到
        // 同一分型 ⟹ 极值价逐值相同（教义裁定 3）。
        let again = resolve_triple_anchor(10, &fractals, &merged);
        assert_eq!(again, (p1, a1), "同一 x 重复解析逐值相同（与级别无关 ⟹ 跨级不变）");
        assert_eq!(p1, Some(100), "极值价口径 = 分型极值价（整数 tick，非腿包络/非原始 K 极值）");
        // x 在 L0 是反向分型（高级别走势以次级别反向段收束的情形）：极值价照取 x 处实际
        // 打印价（190 = 顶分型 high）——键内方向由 event.side 携带，不经本供给。
        let (pt, at) = resolve_triple_anchor(50, &fractals, &merged);
        assert_eq!((pt, at), (Some(190), Some(50)), "极值价 = x 处实际分型价（不设 kind 守卫）");
        // 分型供给未命中 ⟹ 极值价 None；组锚仍可解（x=51 ∈ g2）。
        let (pm, am) = resolve_triple_anchor(51, &fractals, &merged);
        assert_eq!(pm, None);
        assert_eq!(am, Some(50), "组内后续根映射回组锚");
        // 空供给 ⟹ 双 None（事件视图包装路径——锚载体不进事件本体）。
        assert_eq!(resolve_triple_anchor(10, &[], &[]), (None, None));
    }

    /// provider ext 携带两元锚（T1 #170；T5a #207 方向退役后锚 = 极值价 + 组锚）：confirmed 事件的极值价/组锚来自分型管与包含层
    /// 单一来源；锚是 sidecar——事件集/排序与无锚供给路径逐字节同。
    #[test]
    fn provider_ext_carries_triple_anchor_sidecar() {
        // 夹具同 auto_pairing_completes_only_after_real_macd_divergence：confirmed 事件
        // 离开段 (120,129)，x=129，side=Short（Direction::Up 对）。
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        // provider 的 interval_a 需要 leave→retest 块对（structural_pair_span：两块均
        // Completed）；retest 块取 Consolidation（dir None ⟹ 不再产第二个 pair，夹具保单 pair）。
        let blocks = [
            trend_block(Some(Direction::Up)),
            MoveBlock {
                start_center: 1,
                end_center: 2,
                kind: MoveKind::Consolidation,
                dir: None,
                status: MoveStatus::Completed,
            },
        ];
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();
        let query = LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 139 },
            as_of: 139,
            version: C2VersionTuple::auto_pairing(),
        };
        let view = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query,
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &blocks,
                lower_legs: &legs,
                hist: &hist,
                dif: &dif,
                close_src: &close_src,
            },
        )
        .unwrap();
        // 供给：x=129 处顶分型（极值 190 = 腿12 hi）；包含层逐根成组（锚 = 序号自身）。
        let fractals =
            vec![Fractal { kind: FractalKind::Top, source_index: 129, timestamp: 129, price: 190 }];
        let merged: Vec<Bar> = (0..140).map(mbar).collect();
        let exts = provide_nest_candidate_events_ext(
            1, &projection, &blocks, &legs, &view, &hist, &dif, &close_src, &fractals, &merged,
        );
        let ext = exts
            .iter()
            .find(|e| e.event.divergence_confirmed && e.event.kind == NestDivergenceKind::Trend)
            .expect("夹具应产 1 个 confirmed Trend 事件");
        assert_eq!(ext.extreme_price, Some(190), "极值价 = x=129 处分型极值（跨级不变量口径）");
        assert_eq!(ext.group_anchor, Some(129), "组锚 = x 所在合并组首根序号");
        // 锚 sidecar 不进事件本体：事件集/排序与事件视图（无锚供给）逐字节同。
        let events_only =
            provide_nest_candidate_events(1, &projection, &blocks, &legs, &view, &hist, &dif, &close_src);
        assert_eq!(
            exts.iter().map(|e| e.event).collect::<Vec<_>>(),
            events_only,
            "锚 sidecar 不改变事件集/排序（逐字节同）"
        );
    }

    fn trend_block(dir: Option<Direction>) -> MoveBlock {
        MoveBlock {
            start_center: 0,
            end_center: 2,
            kind: if dir.is_some() {
                MoveKind::Trend
            } else {
                MoveKind::Consolidation
            },
            dir,
            status: MoveStatus::Completed,
        }
    }

    fn query(version: C2VersionTuple) -> LevelViewQuery {
        LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 129 },
            as_of: 129,
            version,
        }
    }

    #[test]
    fn unprojected_input_is_observable_and_produces_no_completed_move() {
        let (windows, lower) = extended_windows();
        let legs = lower_legs_from(&lower).unwrap();
        let result = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query(C2VersionTuple::auto_pairing()),
            LevelViewMaterial {
                projection: ProjectionMaterial::Unprojected(&windows),
                move_blocks: &[trend_block(Some(Direction::Up))],
                lower_legs: &legs,
                hist: &[],
                dif: &[],
                close_src: &[],
            },
        );
        assert_eq!(
            result,
            Err(LevelViewError::UnprojectedInput { window_count: 3 })
        );
    }

    #[test]
    fn projection_uses_only_first_three_and_is_immutable_from_extension_tail() {
        let (windows, _) = extended_windows();
        let projected = project_extended_windows(&windows).unwrap();
        assert!(projected
            .seeds
            .iter()
            .all(|seed| seed.source_sub_count == 4));
        assert_eq!(projected.seeds[0].end_index, 29);
        assert_eq!(
            projected.seeds[0].center.gg, 115,
            "扩展尾 [30,39] 的 116 不得污染 seed"
        );
    }

    #[test]
    fn version_tuple_missing_or_wrong_direction_fails_closed() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        for version in [
            C2VersionTuple {
                direction_provider_version: None,
                ..C2VersionTuple::auto_pairing()
            },
            C2VersionTuple {
                divergence_pair_provider_version: None,
                ..C2VersionTuple::auto_pairing()
            },
            C2VersionTuple {
                projection_provider_version: None,
                ..C2VersionTuple::auto_pairing()
            },
            C2VersionTuple {
                direction_provider_version: Some(ProviderVersion("legacy-seam")),
                ..C2VersionTuple::auto_pairing()
            },
            C2VersionTuple {
                divergence_pair_provider_version: Some(ProviderVersion("implicit-pair")),
                ..C2VersionTuple::auto_pairing()
            },
            C2VersionTuple {
                projection_provider_version: Some(ProviderVersion("mutable-window-v0")),
                ..C2VersionTuple::auto_pairing()
            },
        ] {
            let result = assemble_level_view(
                C2LevelViewConfig { enabled: true },
                query(version),
                LevelViewMaterial {
                    projection: ProjectionMaterial::ExactThree(&projection),
                    move_blocks: &[trend_block(Some(Direction::Up))],
                    lower_legs: &legs,
                    hist: &[],
                    dif: &[],
                    close_src: &[],
                },
            );
            assert!(matches!(
                result,
                Err(LevelViewError::InvalidVersionTuple(_))
            ));
        }
    }

    #[test]
    fn dir_none_produces_zero_pairs() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let pairs = provide_divergence_pairs(1, &projection, &[trend_block(None)], &legs, 129);
        assert!(pairs.is_empty());
    }

    /// ★R1（2026-07-17 裁定）：seg_c 取段 = 全离开段——c_end 收 c_start 起、end ≤ as_of 的
    /// 末个同向段终点（037:22 口径）。单段离开时与 #105 单腿口径逐位一致（退化兼容）。
    #[test]
    fn provide_divergence_pairs_seg_c_spans_full_departure() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let block = trend_block(Some(Direction::Up));
        // 夹具：离开腿12（Up，120-129）+ 回试腿13（Down，130-139）+ 延续腿14（Up，140-149）。
        let mut extended = lower.clone();
        extended.push(unit(140, Direction::Up, 168, 200, 14));
        let legs = lower_legs_from(&extended).unwrap();
        let pairs = provide_divergence_pairs(1, &projection, &[block], &legs, 149);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].seg_a, (80, 109), "seg_a 不动（b = 进入最后中枢的段）");
        assert_eq!(
            pairs[0].seg_c,
            (120, 149),
            "R1：c_end = 末个同向段（腿14）终点——全离开段，不再是首腿终点 129"
        );
        // as_of 截断到 129 ⟹ 只看得到首腿：c_end 退化回首腿终点（禁前视，与 #105 逐位一致）。
        let truncated = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
        assert_eq!(truncated[0].seg_c, (120, 129), "as_of 截断 ⟹ 单腿窗口（禁前视）");
    }

    #[test]
    fn opposite_legs_pair_stably_and_disabled_provider_keeps_pending() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let block = trend_block(Some(Direction::Up));
        let one = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
        let two = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
        assert_eq!(one, two, "同输入 pair 身份必须稳定");
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].seg_a, (80, 109));
        assert_eq!(one[0].seg_c, (120, 129));

        let view = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query(C2VersionTuple::pairing_disabled()),
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &[block],
                lower_legs: &legs,
                hist: &vec![0.0; 130],
                dif: &vec![0.0; 130],
                close_src: &(0..130).collect::<Vec<_>>(),
            },
        )
        .unwrap();
        assert!(view.pairs.is_empty());
        assert!(matches!(
            view.moves[0].completion,
            CompletionStatus::Pending {
                reason: PendingReason::MissingDivergencePair,
                ..
            }
        ));
    }

    /// ★R1 语义更新（2026-07-17 裁定）：TerminalDivergence = 全合取（T4 回拉0轴 ∧ T3 三买 ∧
    /// T2 破极值 ∧ T5 力度或关系）首个全成立时点——夹具腿13（回试不重回 w2 核心）与腿12
    /// （破核心离开）构成对 w2 的三买；dif 在 w2 span 内变号（T4）；同色面积/柱峰 C≪A（T5-OR）。
    #[test]
    fn auto_pairing_completes_only_after_real_macd_divergence() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let block = trend_block(Some(Direction::Up));
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0); // b 段（seg_a=[80,109]）红柱面积 60、柱峰 2.0
        hist[120..140].fill(0.1); // c_est=[120,139] 红柱面积 2.0、柱峰 0.1（T5-OR 成立）
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0); // w2 span（[80,119]）内 DIF 负区
        dif[101..140].fill(1.0); // 100→101 变号 ⟹ T4 回拉 0 轴成立
        let close_src: Vec<_> = (0..140).collect();
        let query = LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 139 },
            as_of: 139, // 覆盖回试腿终点（t3=139；旧 129 口径下回试腿被 as_of 截断）
            version: C2VersionTuple::auto_pairing(),
        };
        let view = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query,
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &[block],
                lower_legs: &legs,
                hist: &hist,
                dif: &dif,
                close_src: &close_src,
            },
        )
        .unwrap();
        assert_eq!(view.pairs.len(), 1);
        assert!(matches!(
            view.moves[0].completion,
            CompletionStatus::Completed {
                evidence: CompletionEvidence::TerminalDivergence { .. },
                ..
            }
        ));
    }

    /// ★R1 负例（037:18 必要合取）：同一夹具但 as_of=129 截断回试腿 ⟹ c 内无三买（T3 构造性
    /// 不成立），即便面积/T2 俱备也不得 Completed——锁定「全合取缺三买即未确认」边界。
    #[test]
    fn auto_pairing_pending_when_third_buy_not_established() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let block = trend_block(Some(Direction::Up));
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();
        let view = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query(C2VersionTuple::auto_pairing()), // as_of=129：回试腿（end=139）被截断
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &[block],
                lower_legs: &legs,
                hist: &hist,
                dif: &dif,
                close_src: &close_src,
            },
        )
        .unwrap();
        assert_eq!(view.pairs.len(), 1);
        assert!(
            matches!(
                view.moves[0].completion,
                CompletionStatus::Pending {
                    reason: PendingReason::TerminalLegNotDivergent,
                    ..
                }
            ),
            "c 内无三买（037:18）⟹ 全合取不成立 ⟹ 不得 TerminalDivergence"
        );
    }

    #[test]
    fn feature_default_false_and_cache_key_stable() {
        assert!(!C2LevelViewConfig::default().enabled);
        let q = query(C2VersionTuple::auto_pairing());
        let empty_windows = Vec::new();
        let disabled = assemble_level_view(
            C2LevelViewConfig::default(),
            q,
            LevelViewMaterial {
                projection: ProjectionMaterial::Unprojected(&empty_windows),
                move_blocks: &[],
                lower_legs: &[],
                hist: &[],
                dif: &[],
                close_src: &[],
            },
        );
        assert_eq!(disabled, Err(LevelViewError::FeatureDisabled));

        let a = C2CacheKey::from_query(&q).unwrap();
        let b = C2CacheKey::from_query(&q).unwrap();
        assert_eq!(a, b);
        assert!(a.as_str().contains("dir=central-ggdd-v1"));
        assert!(a.as_str().contains("pair=move-block-ac-v1"));
        assert!(a
            .as_str()
            .contains("projection=extended-to-exact-three-v3-carried-only"));
        let stream_a = C2PersistenceKey::from_query(&q).unwrap();
        let mut later = q;
        later.as_of += 1;
        let stream_b = C2PersistenceKey::from_query(&later).unwrap();
        assert_eq!(
            stream_a, stream_b,
            "freeze event stream key 必须跨 as_of 稳定"
        );
    }
}
