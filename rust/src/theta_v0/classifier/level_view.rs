//! C2 CompletedMove 消费 seam（tasks #73/#74）。
//!
//! 本模块是只读消费层，不改写递归塔。D1 要求调用方显式把当前扩展窗口投影为不可变
//! exact-three seed；D2 只消费 D3 已结裁的 `MoveBlock.dir: Option<Direction>`。四个旧 seam
//! provider 不在本模块出现，方向版本唯一绑定 `central-ggdd-v1`。

use super::super::types::{Center, Direction, MoveKind, Segment, Side, Tick};
use super::center::{center_from_segments, center_from_window, UnitRange};
use super::decompose::{center_block_kind, MoveBlock, MoveStatus};
use super::divergence::{
    departure_move_c_start, locate_departure_move_a, segments_diverge, self_anchors,
};
use super::recursive_tower::{map_src_to_close_idx, ElementId, LeveledMove};
use super::signal::{locate_pan_div_structure, nearest_confirmed_center_idx, pan_div_structure_extreme};
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
    pub const fn auto_pairing() -> Self {
        Self {
            direction_provider_version: Some(ProviderVersion::CENTRAL_GGDD_V1),
            divergence_pair_provider_version: Some(ProviderVersion::MOVE_BLOCK_AC_V1),
            projection_provider_version: Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V2),
        }
    }

    pub const fn pairing_disabled() -> Self {
        Self {
            direction_provider_version: Some(ProviderVersion::CENTRAL_GGDD_V1),
            divergence_pair_provider_version: Some(ProviderVersion::DISABLED_V1),
            projection_provider_version: Some(ProviderVersion::EXTENDED_TO_EXACT_THREE_V2),
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
        if projection != ProviderVersion::EXTENDED_TO_EXACT_THREE_V2 {
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

/// D1 选项 A：显式把扩展窗口投影到 immutable exact-three seed。
pub fn project_extended_windows(
    windows: &[LeveledMove],
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
        // 条款 3（#90 结裁）：offset-0 自核不成立 ⟹ InvalidSeed 维持 D1 选项 0 fail-closed，
        // 不以携带核收复（收复须独立立项走版本化迁移）。
        let own_center = if window.id.level == 1 {
            center_from_segments(&a, &b, &c)
        } else {
            center_from_window(&a, &b, &c)
        }
        .ok_or(ProjectionError::InvalidSeed { index })?;
        // 条款 1（#90 结裁）：准绳 = 塔 compose 携带核（#89 已证与重算 detect 逐窗 bit-equal）。
        let carried = match &window.rmove {
            super::descend::RMove::Compose { centers, .. } => centers.first().copied(),
            super::descend::RMove::Segment { .. } => None,
        }
        .ok_or(ProjectionError::MissingCarriedCenter { index })?;
        // 条款 2（#90 结裁）：两核不等禁静默——seed [ZD,ZG] 取继承核，自核值显式留档。
        let (center, core_provenance) = if own_center.zd == carried.zd && own_center.zg == carried.zg
        {
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
    Ok(ExactThreeProjection {
        version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V2,
        seeds,
    })
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
}

fn range_envelope(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
    segments
        .iter()
        .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
        .fold(None, |acc, segment| {
            let lo = segment.start_price.min(segment.end_price);
            let hi = segment.start_price.max(segment.end_price);
            Some(match acc {
                None => (lo, hi),
                Some((old_lo, old_hi)) => (old_lo.min(lo), old_hi.max(hi)),
            })
        })
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

/// #92 typed provider：把 strict C2 pair 映射为宽结构 Cand，并把力度确认留在独立字段。
///
/// Trend 与 Consolidation 共用同一输出类型但保留 `kind`；后者来自纯结构
/// [`super::signal::PanDivStructure`]，不会经 `PanDivCert` 的 Weak 门提前征税。
pub fn provide_nest_candidate_events(
    level: u32,
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    close_src: &[usize],
) -> Vec<NestCandidateEvent> {
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
        let divergence_confirmed = match (
            map_src_to_close_idx(close_src, pair.seg_a.0, pair.seg_a.1),
            map_src_to_close_idx(close_src, pair.seg_c.0, pair.seg_c.1),
        ) {
            (Some(a), Some(c)) => segments_diverge(hist, a, c),
            _ => false,
        };
        out.push(NestCandidateEvent {
            level,
            side,
            kind: NestDivergenceKind::Trend,
            seg_a: pair.seg_a,
            interval_b: pair.seg_c,
            interval_a,
            divergence_confirmed,
            turn_source: pair.seg_c.1,
            judge_at: view.query.as_of,
            provider_window: (view.query.coordinate_window.start, view.query.coordinate_window.end),
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
        let Some(structure) = locate_pan_div_structure(
            &centers[center_index], segment, &segments, &anchors_self,
        ) else {
            continue;
        };
        if !pan_div_structure_extreme(&structure, &segments) {
            continue;
        }
        let Some(leave_index) = blocks.iter().position(|block| {
            block.kind == MoveKind::Consolidation
                && block.start_center <= center_index
                && block.end_center >= center_index
        }) else {
            continue;
        };
        let Some(interval_a) = structural_pair_span(projection, blocks, leave_index) else {
            continue;
        };
        let divergence_confirmed = match (
            map_src_to_close_idx(close_src, structure.seg_a.0, structure.seg_a.1),
            map_src_to_close_idx(close_src, structure.seg_c.0, structure.seg_c.1),
        ) {
            (Some(a), Some(c)) => segments_diverge(hist, a, c),
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
        };
        if !out.contains(&event) {
            out.push(event);
        }
    }
    out.sort_by_key(|event| (
        event.turn_source,
        event.interval_b,
        event.kind,
        matches!(event.side, Side::Short),
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
        let Some(c_terminal) = segments.iter().rev().find(|segment| {
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
        pairs.push(DivergencePair {
            id: DivergencePairId {
                level,
                block_start_center: block.start_center,
                block_end_center: block.end_center,
                direction,
            },
            move_start: projection.seeds[block.start_center].start_index,
            seg_a,
            seg_c: (c_start, c_terminal.end_index),
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
    if projection.version != ProviderVersion::EXTENDED_TO_EXACT_THREE_V2 {
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
                        let a =
                            map_src_to_close_idx(material.close_src, pair.seg_a.0, pair.seg_a.1);
                        let c =
                            map_src_to_close_idx(material.close_src, pair.seg_c.0, pair.seg_c.1);
                        match (a, c) {
                            (Some(a), Some(c)) if segments_diverge(material.hist, a, c) => {
                                CompletionStatus::Completed {
                                    as_of: query.as_of,
                                    evidence: CompletionEvidence::TerminalDivergence {
                                        pair_id: pair.id,
                                    },
                                }
                            }
                            (Some(_), Some(_)) => CompletionStatus::Pending {
                                as_of: query.as_of,
                                reason: PendingReason::TerminalLegNotDivergent,
                            },
                            _ => CompletionStatus::Pending {
                                as_of: query.as_of,
                                reason: PendingReason::MissingMacdCoordinates,
                            },
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
    use super::super::super::types::MoveKind;
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

    #[test]
    fn opposite_legs_pair_stably_and_disabled_provider_keeps_pending() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows(&windows).unwrap();
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

    #[test]
    fn auto_pairing_completes_only_after_real_macd_divergence() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let block = trend_block(Some(Direction::Up));
        let mut hist = vec![0.0; 130];
        hist[80..110].fill(2.0);
        hist[120..130].fill(0.1);
        let close_src: Vec<_> = (0..130).collect();
        let view = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query(C2VersionTuple::auto_pairing()),
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &[block],
                lower_legs: &legs,
                hist: &hist,
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
            .contains("projection=extended-to-exact-three-v2-inherited-core"));
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
