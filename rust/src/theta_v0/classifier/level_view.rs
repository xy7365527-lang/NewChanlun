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
    locate_pan_div_structure, locate_pan_div_structure_front_anchor, nearest_confirmed_center_idx,
    pan_div_structure_extreme, trend_third_class_in_c,
};
use std::collections::{BTreeSet, HashMap};

#[cfg(test)]
std::thread_local! {
    static CONFIRM_CORE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn reset_confirm_core_calls() {
    CONFIRM_CORE_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
fn confirm_core_calls() -> usize {
    CONFIRM_CORE_CALLS.with(std::cell::Cell::get)
}

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

/// #69 5a：趋势确认的已证状态。`TerminalFalse` 仅表示单调力度关系已经终假；
/// 结构或坐标仍不可验时必须保持 `Scanning`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmState {
    Confirmed(usize),
    TerminalFalse,
    Scanning,
}

impl ConfirmState {
    pub fn as_option(self) -> Option<usize> {
        match self {
            Self::Confirmed(t) => Some(t),
            Self::TerminalFalse | Self::Scanning => None,
        }
    }
}

/// 与 `LevelAsOfView::pairs` 同源的确认 sidecar；消费时必须按 `pair_id` 查找。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairConfirmState {
    pub pair_id: DivergencePairId,
    pub state: ConfirmState,
}

/// #69 5a：单个 divergence pair 在已封 lower-leg 前缀上的扫描累积。
#[derive(Debug, Clone)]
pub struct ConfirmCursor {
    /// 下一个尚未消费的 lower-leg 下标；只允许落在确认水线内。
    k0: usize,
    env: Option<(Tick, Tick)>,
    acc_hi: Option<usize>,
    area_c: f64,
    dif_max: f64,
    dif_min: f64,
    hist_max: f64,
    hist_min: f64,
    state: ConfirmState,
}

impl Default for ConfirmCursor {
    fn default() -> Self {
        Self {
            k0: 0,
            env: None,
            acc_hi: None,
            area_c: 0.0,
            dif_max: f64::NEG_INFINITY,
            dif_min: f64::INFINITY,
            hist_max: f64::NEG_INFINITY,
            hist_min: f64::INFINITY,
            state: ConfirmState::Scanning,
        }
    }
}

/// #69 5a：不含 `as_of` 与可增长 seg-c 末端的结构身份键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfirmKey {
    level: u32,
    run_window: CoordinateWindow,
    version: C2VersionTuple,
    pair_id: DivergencePairId,
    move_start: usize,
    seg_a: (usize, usize),
    c_start: usize,
    b_fingerprint: (usize, usize, Tick, Tick, Tick, Tick),
    structure_generation: u64,
}

impl ConfirmKey {
    fn for_pair(
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

/// #69 5a：per-level divergence-pair cursor 映射；实际持有者在 bin `LevelDerived`。
#[derive(Debug, Default)]
pub struct ConfirmCursorStore {
    cursors: HashMap<ConfirmKey, ConfirmCursor>,
}

impl ConfirmCursorStore {
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    /// 上层 run 分区变化时删除已消失 run，避免陈旧 key 永久驻留。
    pub fn retain_run_starts(&mut self, level: u32, run_starts: impl IntoIterator<Item = usize>) {
        let run_starts: BTreeSet<_> = run_starts.into_iter().collect();
        self.cursors
            .retain(|key, _| key.level != level || run_starts.contains(&key.run_window.start));
    }

    fn retain_active_for_run(&mut self, level: u32, run_start: usize, active: &[ConfirmKey]) {
        self.cursors.retain(|key, _| {
            key.level != level
                || key.run_window.start != run_start
                || active.iter().any(|candidate| candidate == key)
        });
    }

    fn cursor_mut(&mut self, key: ConfirmKey) -> &mut ConfirmCursor {
        self.cursors.entry(key).or_default()
    }

    #[cfg(test)]
    fn poison_for_test(&mut self, state: ConfirmState) {
        for cursor in self.cursors.values_mut() {
            cursor.state = state;
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

fn structural_block_span(
    projection: &ExactThreeProjection,
    block: &MoveBlock,
) -> Option<(usize, usize)> {
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
fn trend_confirm_state(
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
fn trend_confirm_state_core(
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
    trend_confirm_state(
        segments, last, direction, side, seg_a, c_start, as_of, hist, dif, close_src,
    )
    .as_option()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PanSegmentIdentity {
    up: bool,
    start_index: usize,
    end_index: usize,
    start_price: Tick,
    end_price: Tick,
}

impl From<&Segment> for PanSegmentIdentity {
    fn from(segment: &Segment) -> Self {
        Self {
            up: segment.direction == Direction::Up,
            start_index: segment.start_index,
            end_index: segment.end_index,
            start_price: segment.start_price,
            end_price: segment.end_price,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PanCenterIdentity {
    zd: Tick,
    zg: Tick,
    dd: Tick,
    gg: Tick,
    start_index: usize,
    end_index: usize,
}

impl From<&Center> for PanCenterIdentity {
    fn from(center: &Center) -> Self {
        Self {
            zd: center.zd,
            zg: center.zg,
            dd: center.dd,
            gg: center.gg,
            start_index: center.start_index,
            end_index: center.end_index,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PanBlockIdentity {
    start_center: usize,
    end_center: usize,
    trend: bool,
    direction: i8,
    completed: bool,
    start_source: usize,
    end_source: usize,
}

fn pan_block_identity(
    projection: &ExactThreeProjection,
    block: &MoveBlock,
    reads_status: bool,
) -> Option<PanBlockIdentity> {
    Some(PanBlockIdentity {
        start_center: block.start_center,
        end_center: block.end_center,
        trend: block.kind == MoveKind::Trend,
        direction: match block.dir {
            Some(Direction::Up) => 1,
            Some(Direction::Down) => -1,
            None => 0,
        },
        // target/第一后继的 status 进入 structural_pair_span；第二后继只作存在性封口门。
        // 忽略第二后继的 Active→Completed，保证纯追加第四块不清已证目标。
        completed: reads_status && block.status == MoveStatus::Completed,
        start_source: projection.seeds.get(block.start_center)?.start_index,
        end_source: projection.seeds.get(block.end_center)?.end_index,
    })
}

fn pan_owner_block_index(blocks: &[MoveBlock], center_index: usize) -> Option<usize> {
    if center_index == 0 {
        return (!blocks.is_empty()).then_some(0);
    }
    blocks
        .iter()
        .position(|block| block.start_center < center_index && center_index <= block.end_center)
}

fn pan_block_triple(
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    block_index: usize,
) -> Option<[PanBlockIdentity; 3]> {
    // 链②唯一门：目标块之后至少两个完整身份槽；不足时稳定资格不存在。
    (block_index + 2 < blocks.len()).then_some(())?;
    Some([
        pan_block_identity(projection, blocks.get(block_index)?, true)?,
        pan_block_identity(projection, blocks.get(block_index + 1)?, true)?,
        pan_block_identity(projection, blocks.get(block_index + 2)?, false)?,
    ])
}

/// run 语境键：level/window/version + segment/center + ownership block 与两个后继块。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PanMemoKey {
    level: u32,
    provider_window: (usize, usize),
    projection_version: ProviderVersion,
    segment_index: usize,
    segment: PanSegmentIdentity,
    center_index: usize,
    center: PanCenterIdentity,
    block_index: usize,
    blocks: [PanBlockIdentity; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PanEventCore {
    structure: super::signal::PanDivStructure,
    interval_a: (usize, usize),
    intake_fallback: bool,
    divergence_confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanMemoValue {
    NoEvent,
    Event(PanEventCore),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PanMemoEntry {
    /// 本 entry 实际读取的最大 source_index；复用要求严格 `< e_src`。
    read_end_src: usize,
    /// 定位窄锚/front-anchor/Extreme 实际可见的段前缀长度。
    read_segment_count: usize,
    value: PanMemoValue,
}

/// #69 5b memo 诊断计数；只描述 memo 行为，不参与事件语义。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PanMemoStats {
    pub hits: usize,
    pub misses: usize,
    pub writes: usize,
    pub invalidations: usize,
}

/// #69 5b：由调用方按 run 持有的盘整背驰 memo。
///
/// 状态只经显式 [`PanResidence`] 进入 provider；默认入口与 `None` 始终走真冷路径。
#[derive(Debug, Default)]
pub struct PanMemo {
    entries: HashMap<PanMemoKey, PanMemoEntry>,
    /// run 的逐项精确 lower-segment 快照；entry 只存读前缀长度，避免每项复制整段前缀。
    segment_snapshot: Vec<PanSegmentIdentity>,
    stats: PanMemoStats,
}

impl PanMemo {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn stats(&self) -> PanMemoStats {
        self.stats
    }

    fn prepare(
        &mut self,
        level: u32,
        provider_window: (usize, usize),
        projection_version: ProviderVersion,
        projection: &ExactThreeProjection,
        blocks: &[MoveBlock],
        segments: &[Segment],
        centers: &[Center],
        freeze_boundary_src: usize,
    ) {
        let current_segments: Vec<_> = segments.iter().map(PanSegmentIdentity::from).collect();
        let common_segment_prefix = self
            .segment_snapshot
            .iter()
            .zip(&current_segments)
            .take_while(|(old, current)| old == current)
            .count();
        let before = self.entries.len();
        self.entries.retain(|key, entry| {
            key.level == level
                && key.provider_window == provider_window
                && key.projection_version == projection_version
                && entry.read_end_src < freeze_boundary_src
                && entry.read_segment_count <= common_segment_prefix
                && segments
                    .get(key.segment_index)
                    .is_some_and(|segment| PanSegmentIdentity::from(segment) == key.segment)
                && centers
                    .get(key.center_index)
                    .is_some_and(|center| PanCenterIdentity::from(center) == key.center)
                && pan_block_triple(projection, blocks, key.block_index)
                    .is_some_and(|blocks| blocks == key.blocks)
        });
        self.stats.invalidations += before - self.entries.len();
        self.segment_snapshot = current_segments;
    }

    fn lookup(&mut self, key: &PanMemoKey) -> Option<PanMemoValue> {
        match self.entries.get(key).map(|entry| entry.value) {
            Some(entry) => {
                self.stats.hits += 1;
                Some(entry)
            }
            None => {
                self.stats.misses += 1;
                None
            }
        }
    }

    fn insert(
        &mut self,
        key: PanMemoKey,
        read_end_src: usize,
        read_segment_count: usize,
        value: PanMemoValue,
    ) {
        self.entries.insert(
            key,
            PanMemoEntry {
                read_end_src,
                read_segment_count,
                value,
            },
        );
        self.stats.writes += 1;
    }

    #[cfg(test)]
    fn poison_for_test(&mut self) {
        let entry = self
            .entries
            .values_mut()
            .find(|entry| matches!(entry.value, PanMemoValue::Event(_)))
            .expect("测试夹具须已有 event memo");
        let PanMemoValue::Event(mut core) = entry.value else {
            unreachable!("上方已筛 Event");
        };
        core.divergence_confirmed = !core.divergence_confirmed;
        entry.value = PanMemoValue::Event(core);
    }
}

/// #69 5b：pan memo 的显式 resident seam。`freeze_boundary_src` 是 source_index 量纲；
/// `None` 表示完全绕过 memo 的真冷路径。
pub struct PanResidence<'a> {
    pub memo: &'a mut PanMemo,
    pub freeze_boundary_src: usize,
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
fn resolve_triple_anchor(
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
    pub pair_confirmations: Vec<PairConfirmState>,
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
    assemble_level_view_impl(config, query, material, None)
}

/// #69 5a resident seam。旧入口签名不动并委托 `None`；调用方只有显式给出
/// `ConfirmResidence` 才能跨 view 复用已封 lower-leg 前缀。
pub fn assemble_level_view_resident(
    config: C2LevelViewConfig,
    query: LevelViewQuery,
    material: LevelViewMaterial<'_>,
    residence: Option<ConfirmResidence<'_>>,
) -> Result<LevelAsOfView, LevelViewError> {
    assemble_level_view_impl(config, query, material, residence)
}

fn assemble_level_view_impl(
    config: C2LevelViewConfig,
    query: LevelViewQuery,
    material: LevelViewMaterial<'_>,
    mut residence: Option<ConfirmResidence<'_>>,
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
    if let Some(residence) = residence.as_mut() {
        let active_keys: Vec<_> = pairs
            .iter()
            .map(|pair| {
                let last = &projection.seeds[pair.id.block_end_center].center;
                ConfirmKey::for_pair(query, pair, last, residence.structure_generation)
            })
            .collect();
        residence.store.retain_active_for_run(
            query.level,
            query.coordinate_window.start,
            &active_keys,
        );
    }

    // Trend 完成判定（R1 全合取扫描）消费的段序列——与 provide_nest_candidate_events 同一投影。
    let segments: Vec<_> = material.lower_legs.iter().map(leg_as_segment).collect();
    let pair_confirmations: Vec<_> = pairs
        .iter()
        .map(|pair| {
            let last = &projection.seeds[pair.id.block_end_center].center;
            let direction = pair.id.direction;
            let side = match direction {
                Direction::Down => Side::Long,
                Direction::Up => Side::Short,
            };
            let state = if let Some(residence) = residence.as_mut() {
                let key = ConfirmKey::for_pair(query, pair, last, residence.structure_generation);
                let stable_lower_len = residence.stable_lower_len;
                let cursor = residence.store.cursor_mut(key);
                trend_confirm_state_core(
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
                    Some((cursor, stable_lower_len)),
                )
            } else {
                trend_confirm_state(
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
                )
            };
            PairConfirmState {
                pair_id: pair.id,
                state,
            }
        })
        .collect();

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
                        let mappable =
                            map_src_to_close_idx(material.close_src, pair.seg_a.0, pair.seg_a.1)
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
                            let state = pair_confirmations
                                .iter()
                                .find(|confirmation| confirmation.pair_id == pair.id)
                                .map_or(ConfirmState::Scanning, |confirmation| confirmation.state);
                            match state {
                                ConfirmState::Confirmed(_) => CompletionStatus::Completed {
                                    as_of: query.as_of,
                                    evidence: CompletionEvidence::TerminalDivergence {
                                        pair_id: pair.id,
                                    },
                                },
                                ConfirmState::TerminalFalse | ConfirmState::Scanning => {
                                    CompletionStatus::Pending {
                                        as_of: query.as_of,
                                        reason: PendingReason::TerminalLegNotDivergent,
                                    }
                                }
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
        pair_confirmations,
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
            Fractal {
                kind: FractalKind::Bottom,
                source_index: 10,
                timestamp: 10,
                price: 100,
            },
            // W 底第二脚：同向同价（底, 100），与第一脚分属不同合并组。
            Fractal {
                kind: FractalKind::Bottom,
                source_index: 30,
                timestamp: 30,
                price: 100,
            },
            Fractal {
                kind: FractalKind::Top,
                source_index: 50,
                timestamp: 50,
                price: 190,
            },
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
        assert_eq!(
            again,
            (p1, a1),
            "同一 x 重复解析逐值相同（与级别无关 ⟹ 跨级不变）"
        );
        assert_eq!(
            p1,
            Some(100),
            "极值价口径 = 分型极值价（整数 tick，非腿包络/非原始 K 极值）"
        );
        // x 在 L0 是反向分型（高级别走势以次级别反向段收束的情形）：极值价照取 x 处实际
        // 打印价（190 = 顶分型 high）——键内方向由 event.side 携带，不经本供给。
        let (pt, at) = resolve_triple_anchor(50, &fractals, &merged);
        assert_eq!(
            (pt, at),
            (Some(190), Some(50)),
            "极值价 = x 处实际分型价（不设 kind 守卫）"
        );
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
        let fractals = vec![Fractal {
            kind: FractalKind::Top,
            source_index: 129,
            timestamp: 129,
            price: 190,
        }];
        let merged: Vec<Bar> = (0..140).map(mbar).collect();
        let exts = provide_nest_candidate_events_ext(
            1,
            &projection,
            &blocks,
            &legs,
            &view,
            &hist,
            &dif,
            &close_src,
            &fractals,
            &merged,
        );
        let ext = exts
            .iter()
            .find(|e| e.event.divergence_confirmed && e.event.kind == NestDivergenceKind::Trend)
            .expect("夹具应产 1 个 confirmed Trend 事件");
        assert_eq!(
            ext.extreme_price,
            Some(190),
            "极值价 = x=129 处分型极值（跨级不变量口径）"
        );
        assert_eq!(ext.group_anchor, Some(129), "组锚 = x 所在合并组首根序号");
        // 锚 sidecar 不进事件本体：事件集/排序与事件视图（无锚供给）逐字节同。
        let events_only = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &view,
            &hist,
            &dif,
            &close_src,
        );
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
        assert_eq!(
            pairs[0].seg_a,
            (80, 109),
            "seg_a 不动（b = 进入最后中枢的段）"
        );
        assert_eq!(
            pairs[0].seg_c,
            (120, 149),
            "R1：c_end = 末个同向段（腿14）终点——全离开段，不再是首腿终点 129"
        );
        // as_of 截断到 129 ⟹ 只看得到首腿：c_end 退化回首腿终点（禁前视，与 #105 逐位一致）。
        let truncated = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
        assert_eq!(
            truncated[0].seg_c,
            (120, 129),
            "as_of 截断 ⟹ 单腿窗口（禁前视）"
        );
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

    /// #69 5a / T0：先锁住冷核的首证钟与未决投影；resident 化不得改写现行
    /// `Option<usize>` 可观察结果。
    #[test]
    fn confirm_state_cold_projection_preserves_first_proof_and_scanning() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
        let last = &projection.seeds[2].center;
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();

        let confirmed = trend_confirm_state(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            139,
            &hist,
            &dif,
            &close_src,
        );
        assert_eq!(confirmed, ConfirmState::Confirmed(139));
        assert_eq!(
            confirmed.as_option(),
            trend_confirm_time(
                &segments,
                last,
                Direction::Up,
                Side::Short,
                (80, 109),
                120,
                139,
                &hist,
                &dif,
                &close_src,
            )
        );

        let scanning = trend_confirm_state(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            129,
            &hist,
            &dif,
            &close_src,
        );
        assert_eq!(scanning, ConfirmState::Scanning);
        assert_eq!(scanning.as_option(), None);
    }

    /// #69 5a / T1：只有已进入 T5 判定且 `force_ok` 单调转假才是终假；
    /// 坐标缺失等旧 `None` 必须仍是未决。
    #[test]
    fn confirm_state_distinguishes_terminal_false_from_unresolved_none() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
        let last = &projection.seeds[2].center;
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(5.0);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();

        let terminal = trend_confirm_state(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            139,
            &hist,
            &dif,
            &close_src,
        );
        assert_eq!(terminal, ConfirmState::TerminalFalse);
        assert_eq!(terminal.as_option(), None);

        let missing_coordinates = trend_confirm_state(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            139,
            &hist,
            &dif,
            &[],
        );
        assert_eq!(missing_coordinates, ConfirmState::Scanning);
    }

    /// #69 5a / T2：同一 pair 按 as_of 增长时，resident cursor 每一步必须与空 cursor
    /// 冷算同果；已封 lower-leg 下标只前进、不重加。
    #[test]
    fn confirm_cursor_incremental_matches_cold_at_every_boundary() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let segments: Vec<_> = legs.iter().map(leg_as_segment).collect();
        let last = &projection.seeds[2].center;
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();
        let mut cursor = ConfirmCursor::default();
        let mut previous_k0 = 0;

        for as_of in [119, 129, 139] {
            let confirmed_len = segments.partition_point(|segment| segment.end_index <= as_of);
            let resident = trend_confirm_state_core(
                &segments,
                last,
                Direction::Up,
                Side::Short,
                (80, 109),
                120,
                as_of,
                &hist,
                &dif,
                &close_src,
                Some((&mut cursor, confirmed_len)),
            );
            let cold = trend_confirm_state(
                &segments,
                last,
                Direction::Up,
                Side::Short,
                (80, 109),
                120,
                as_of,
                &hist,
                &dif,
                &close_src,
            );
            assert_eq!(resident, cold, "as_of={as_of} 热路必须等于空 cursor 冷路");
            assert!(cursor.k0 >= previous_k0, "已封 lower-leg 游标不得倒退");
            assert!(cursor.k0 <= confirmed_len, "cursor 只能落在已封水线内");
            previous_k0 = cursor.k0;
        }
        assert_eq!(cursor.state, ConfirmState::Confirmed(139));
        let sealed_k0 = cursor.k0;
        let repeated = trend_confirm_state_core(
            &segments,
            last,
            Direction::Up,
            Side::Short,
            (80, 109),
            120,
            139,
            &hist,
            &dif,
            &close_src,
            Some((&mut cursor, segments.len())),
        );
        assert_eq!(repeated, ConfirmState::Confirmed(139));
        assert_eq!(cursor.k0, sealed_k0, "终态重复查询不得重扫或推进 cursor");
    }

    /// #69 5a / T3：store 只持久化已封前缀；证书回退必须清 cursor 冷重算，
    /// 结构代次变化必须 key miss 且同 run 旧 key 被剪枝。
    #[test]
    fn confirm_store_resets_on_watermark_rollback_and_structure_change() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let blocks = [trend_block(Some(Direction::Up))];
        let mut hist = vec![0.0; 140];
        hist[80..110].fill(2.0);
        hist[120..140].fill(0.1);
        let mut dif = vec![0.0; 140];
        dif[80..=100].fill(-5.0);
        dif[101..140].fill(1.0);
        let close_src: Vec<_> = (0..140).collect();
        let resident_query = LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 139 },
            as_of: 139,
            version: C2VersionTuple::auto_pairing(),
        };
        let material = || LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        };
        let mut store = ConfirmCursorStore::default();

        let sealed = assemble_level_view_resident(
            C2LevelViewConfig { enabled: true },
            resident_query,
            material(),
            Some(ConfirmResidence {
                store: &mut store,
                stable_lower_len: legs.len(),
                structure_generation: 7,
            }),
        )
        .unwrap();
        assert!(matches!(
            sealed.moves[0].completion,
            CompletionStatus::Completed { .. }
        ));
        assert_eq!(store.cursors.len(), 1);
        assert_eq!(
            store.cursors.values().next().unwrap().state,
            ConfirmState::Confirmed(139)
        );

        let rolled = assemble_level_view_resident(
            C2LevelViewConfig { enabled: true },
            resident_query,
            material(),
            Some(ConfirmResidence {
                store: &mut store,
                stable_lower_len: 12,
                structure_generation: 7,
            }),
        )
        .unwrap();
        assert_eq!(
            rolled,
            assemble_level_view(
                C2LevelViewConfig { enabled: true },
                resident_query,
                material()
            )
            .unwrap()
        );
        let cursor = store.cursors.values().next().unwrap();
        assert_eq!(cursor.k0, 12, "水线回退后只可重建到新已封边界");
        assert_eq!(cursor.state, ConfirmState::Scanning, "可变尾结果不得回写");

        let regenerated = assemble_level_view_resident(
            C2LevelViewConfig { enabled: true },
            resident_query,
            material(),
            Some(ConfirmResidence {
                store: &mut store,
                stable_lower_len: legs.len(),
                structure_generation: 8,
            }),
        )
        .unwrap();
        assert_eq!(regenerated, sealed);
        assert_eq!(store.cursors.len(), 1, "同 run 的旧结构 key 必须被剪枝");
        assert!(store
            .cursors
            .keys()
            .all(|key| key.structure_generation == 8));
    }

    /// #69 5a / T4：assemble 与 provider 必须消费 view 内同一份 pair 状态；
    /// provider 不得二次进入确认核，且 sidecar 查找必须校验 `DivergencePairId`。
    #[test]
    fn assemble_and_provider_share_one_pair_confirmation() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
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
        reset_confirm_core_calls();
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
        assert_eq!(view.pair_confirmations.len(), 1);
        assert_eq!(
            view.pair_confirmations[0],
            PairConfirmState {
                pair_id: view.pairs[0].id,
                state: ConfirmState::Confirmed(139),
            }
        );
        assert_eq!(confirm_core_calls(), 1, "assemble 每 pair 只进核一次");

        let events = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &view,
            &hist,
            &dif,
            &close_src,
        );
        assert!(events
            .iter()
            .any(|event| event.kind == NestDivergenceKind::Trend && event.divergence_confirmed));
        assert_eq!(confirm_core_calls(), 1, "provider 必须只读 view sidecar");

        let mut mismatched = view.clone();
        mismatched.pair_confirmations[0].pair_id.level += 1;
        let mismatched_events = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &mismatched,
            &hist,
            &dif,
            &close_src,
        );
        assert!(mismatched_events
            .iter()
            .filter(|event| event.kind == NestDivergenceKind::Trend)
            .all(|event| !event.divergence_confirmed));
        assert_eq!(
            confirm_core_calls(),
            1,
            "身份错配必须 fail-closed，禁止回退重算"
        );
    }

    /// #69 5a / T5：即使 resident store 被污染，显式 `None` 仍必须走真冷核，
    /// 既不读取也不改写该 store。
    #[test]
    fn cold_none_oracle_is_isolated_from_poisoned_resident_store() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        let blocks = [trend_block(Some(Direction::Up))];
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
        let material = || LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        };
        let mut store = ConfirmCursorStore::default();
        assemble_level_view_resident(
            C2LevelViewConfig { enabled: true },
            query,
            material(),
            Some(ConfirmResidence {
                store: &mut store,
                stable_lower_len: legs.len(),
                structure_generation: 1,
            }),
        )
        .unwrap();
        store.poison_for_test(ConfirmState::TerminalFalse);

        let forced_cold = assemble_level_view_resident(
            C2LevelViewConfig { enabled: true },
            query,
            material(),
            None,
        )
        .unwrap();
        assert_eq!(
            forced_cold.pair_confirmations[0].state,
            ConfirmState::Confirmed(139)
        );
        assert!(matches!(
            forced_cold.moves[0].completion,
            CompletionStatus::Completed { .. }
        ));
        assert!(store
            .cursors
            .values()
            .all(|cursor| cursor.state == ConfirmState::TerminalFalse));
    }

    /// 票 #427 左端恒等（裁定 #402 题二「维持工程桥」的等价性前提）：trend 确认分支把
    /// `interval_b` 收束到 `[c_start, t*]`——**只截右端**。本用例跨两个独立公开面交叉核对：
    /// `provide_divergence_pairs` 给出的 `pair.seg_c` 与 `provide_nest_candidate_events`
    /// 给出的 `event.interval_b` 左端必须逐一相等。
    ///
    /// 非空转证明：同一夹具下**右端确实改变**，故「左端相等」不是「两端都没动」的顺带结论。
    #[test]
    fn trend_confirm_truncation_keeps_seg_c_left_anchor() {
        let (windows, lower) = extended_windows();
        let projection = project_extended_windows_carried_only(&windows).unwrap();
        let legs = lower_legs_from(&lower).unwrap();
        // 与 nest_lifecycle T5 同款两块布局：interval_a 需要 leave→retest 块对
        // （`structural_pair_span` 要求两块均 Completed），单块下 trend 事件不产出。
        let blocks = [
            MoveBlock {
                start_center: 0,
                end_center: 2,
                kind: MoveKind::Trend,
                dir: Some(Direction::Up),
                status: MoveStatus::Completed,
            },
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
        let pairs = provide_divergence_pairs(1, &projection, &blocks, &legs, 139);
        let events = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &view,
            &hist,
            &dif,
            &close_src,
        );
        let trend: Vec<_> = events
            .iter()
            .filter(|event| event.kind == NestDivergenceKind::Trend)
            .collect();
        assert_eq!(trend.len(), 1, "夹具产单只 trend 事件");
        assert_eq!(pairs.len(), 1, "同夹具产单只 pair");
        assert!(trend[0].divergence_confirmed, "前提：走的是确认收束分支");
        assert_eq!(
            trend[0].interval_b.0, pairs[0].seg_c.0,
            "左端恒等（票 #427 加锁的前提）"
        );
        // 非空转：同夹具下右端**确实改变**（实测 129 → 139，t* 晚于 seg_c 右端 ⟹ 本例是
        // 外扩不是截短——裁定 #402 题二的「收束」措辞只在 t* ≤ seg_c.1 时成立；桥判同不看
        // 右端方向（`bridge_identity` 三形态口径），故等价性结论不受影响）。
        assert_ne!(
            trend[0].interval_b.1, pairs[0].seg_c.1,
            "右端确实改变 ⟹ 「左端相等」不是「两端都没动」的顺带结论"
        );

        // 第二臂：**未确认分支**（as_of=129 截断回试腿 ⟹ c 内无三买 ⟹ 全合取不成立）。
        // 两个分支各写一次 `interval_b`，左端恒等须两边都成立——只测确认分支会留下
        // 未确认分支的静默失效面（变异实测 M10 曾无干净测试捕获）。
        let query_pending = LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 139 },
            as_of: 129,
            version: C2VersionTuple::auto_pairing(),
        };
        let view_pending = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query_pending,
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
        let pairs_pending = provide_divergence_pairs(1, &projection, &blocks, &legs, 129);
        let events_pending = provide_nest_candidate_events(
            1,
            &projection,
            &blocks,
            &legs,
            &view_pending,
            &hist,
            &dif,
            &close_src,
        );
        let trend_pending: Vec<_> = events_pending
            .iter()
            .filter(|event| event.kind == NestDivergenceKind::Trend)
            .collect();
        assert_eq!(trend_pending.len(), 1, "未确认分支同样产单只 trend 事件");
        assert!(
            !trend_pending[0].divergence_confirmed,
            "前提：走的是未确认分支（保持全离开段结构坐标）"
        );
        assert_eq!(pairs_pending.len(), 1);
        assert_eq!(
            trend_pending[0].interval_b.0, pairs_pending[0].seg_c.0,
            "未确认分支左端同样恒等"
        );
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

    struct PanProviderFixture {
        projection: ExactThreeProjection,
        blocks: Vec<MoveBlock>,
        legs: Vec<LowerLeg>,
        view: LevelAsOfView,
        hist: Vec<f64>,
        dif: Vec<f64>,
        close_src: Vec<usize>,
    }

    fn pan_seed(index: usize, center: Center) -> ExactThreeSeed {
        ExactThreeSeed {
            source_id: ElementId {
                level: 1,
                ordinal: index as u64,
            },
            source_sub_count: 3,
            start_index: center.start_index,
            end_index: center.end_index,
            center,
            core_provenance: SeedCoreProvenance::SelfConsistent,
        }
    }

    fn pan_provider_fixture() -> PanProviderFixture {
        let centers = [
            Center {
                zd: 100,
                zg: 200,
                dd: 90,
                gg: 210,
                start_index: 0,
                end_index: 2,
            },
            Center {
                zd: 300,
                zg: 400,
                dd: 290,
                gg: 410,
                start_index: 3,
                end_index: 5,
            },
            Center {
                zd: 350,
                zg: 450,
                dd: 250,
                gg: 460,
                start_index: 4,
                end_index: 8,
            },
            Center {
                zd: 500,
                zg: 550,
                dd: 490,
                gg: 560,
                start_index: 12,
                end_index: 14,
            },
            Center {
                zd: 600,
                zg: 650,
                dd: 590,
                gg: 660,
                start_index: 15,
                end_index: 17,
            },
            Center {
                zd: 580,
                zg: 640,
                dd: 570,
                gg: 670,
                start_index: 18,
                end_index: 20,
            },
            Center {
                zd: 700,
                zg: 750,
                dd: 690,
                gg: 760,
                start_index: 21,
                end_index: 23,
            },
        ];
        let projection = ExactThreeProjection {
            version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V3,
            seeds: centers
                .iter()
                .copied()
                .enumerate()
                .map(|(index, center)| pan_seed(index, center))
                .collect(),
        };
        let blocks = vec![
            MoveBlock {
                start_center: 0,
                end_center: 2,
                kind: MoveKind::Consolidation,
                dir: None,
                status: MoveStatus::Completed,
            },
            MoveBlock {
                start_center: 2,
                end_center: 4,
                kind: MoveKind::Trend,
                dir: Some(Direction::Up),
                status: MoveStatus::Completed,
            },
            MoveBlock {
                start_center: 4,
                end_center: 6,
                kind: MoveKind::Consolidation,
                dir: None,
                status: MoveStatus::Active,
            },
        ];
        let legs = vec![
            LowerLeg {
                id: ElementId {
                    level: 0,
                    ordinal: 0,
                },
                direction: Direction::Down,
                start_index: 1,
                end_index: 3,
                lo: 360,
                hi: 460,
            },
            LowerLeg {
                id: ElementId {
                    level: 0,
                    ordinal: 1,
                },
                direction: Direction::Down,
                start_index: 9,
                end_index: 11,
                lo: 300,
                hi: 380,
            },
        ];
        let mut hist = vec![0.0; 32];
        hist[1..=3].fill(-5.0);
        hist[9] = -1.0;
        hist[10] = 20.0;
        hist[11] = -1.0;
        let dif = vec![0.0; 32];
        let close_src: Vec<_> = (0..32).collect();
        let query = LevelViewQuery {
            level: 1,
            coordinate_window: CoordinateWindow { start: 0, end: 23 },
            as_of: 31,
            version: C2VersionTuple::auto_pairing(),
        };
        let view = LevelAsOfView {
            query,
            cache_key: C2CacheKey::from_query(&query).unwrap(),
            moves: Vec::new(),
            pairs: Vec::new(),
            pair_confirmations: Vec::new(),
        };
        PanProviderFixture {
            projection,
            blocks,
            legs,
            view,
            hist,
            dif,
            close_src,
        }
    }

    /// #69 5b / T0：先锁冷路径盘背事件的全部物理字段；resident `None` 必须逐字段等于旧入口。
    #[test]
    fn pan_memo_cold_path_characterization() {
        let fixture = pan_provider_fixture();
        let cold = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let resident_none = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            None,
        );
        assert_eq!(resident_none, cold, "None 必须是真冷旧核");
        let event = cold
            .iter()
            .find(|event| event.kind == NestDivergenceKind::Consolidation)
            .expect("夹具必须产真实盘背事件");
        assert_eq!(event.side, Side::Long);
        assert_eq!(event.seg_a, (1, 3));
        assert_eq!(event.interval_b, (9, 11));
        assert_eq!(event.interval_a, (0, 17));
        assert!(event.divergence_confirmed);
        assert_eq!(event.turn_source, 11);
        assert_eq!(event.judge_at, 31);
        assert_eq!(event.provider_window, (0, 23));
        assert!(!event.intake_fallback);
        assert_eq!(event.b_center_start, 4);
    }

    /// #69 5b / T0 补格：锁定窄锚优先、Extreme 淘汰、span fallback 与事件去重；
    /// 坐标未到格由 `pan_memo_incomplete_macd_does_not_negative_cache` 同时覆盖。
    #[test]
    fn pan_cold_path_narrow_extreme_fallback_and_dedup_grid() {
        let fixture = pan_provider_fixture();

        let mut narrow_legs = fixture.legs.clone();
        narrow_legs.push(LowerLeg {
            id: ElementId {
                level: 0,
                ordinal: 2,
            },
            direction: Direction::Up,
            start_index: 11,
            end_index: 13,
            lo: 300,
            hi: 430,
        });
        narrow_legs.push(LowerLeg {
            id: ElementId {
                level: 0,
                ordinal: 3,
            },
            direction: Direction::Down,
            start_index: 13,
            end_index: 15,
            lo: 280,
            hi: 430,
        });
        let narrow = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &narrow_legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        assert!(
            narrow.iter().any(|event| {
                event.kind == NestDivergenceKind::Consolidation
                    && event.seg_a == (9, 11)
                    && event.interval_b == (13, 15)
            }),
            "同一中枢第二次离开必须优先命中窄锚 A"
        );

        let mut no_extreme_legs = fixture.legs.clone();
        no_extreme_legs[1].lo = 370;
        let no_extreme = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &no_extreme_legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        assert!(no_extreme
            .iter()
            .all(|event| event.kind != NestDivergenceKind::Consolidation));

        let mut fallback_blocks = fixture.blocks.clone();
        fallback_blocks[1].status = MoveStatus::Active;
        let fallback = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fallback_blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let fallback_event = fallback
            .iter()
            .find(|event| event.kind == NestDivergenceKind::Consolidation)
            .expect("span 不可用时结构候选不得丢失");
        assert!(fallback_event.intake_fallback);
        assert_eq!(fallback_event.interval_a, (0, 8));

        let mut duplicate_legs = fixture.legs.clone();
        let mut duplicate = duplicate_legs[1];
        duplicate.id.ordinal += 10;
        duplicate_legs.push(duplicate);
        let deduped = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &duplicate_legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        assert_eq!(
            deduped
                .iter()
                .filter(|event| event.kind == NestDivergenceKind::Consolidation)
                .count(),
            1,
            "重复候选仍按事件本体去重"
        );
    }

    /// #69 5b / T2（链①）：水位增长后稳定 entry 一生一算；回退及严格等号边界立即失效。
    #[test]
    fn pan_memo_e_src_growth_rollback_and_equal_boundary() {
        let fixture = pan_provider_fixture();
        let cold = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let mut memo = PanMemo::default();

        let first = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        assert_eq!(first, cold);
        assert_eq!(memo.len(), 1);
        assert_eq!(memo.stats().writes, 1);
        assert_eq!(memo.stats().hits, 0);

        let grown = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert_eq!(grown, cold);
        assert_eq!(memo.len(), 1, "水位增长不得清已证前缀");
        assert_eq!(memo.stats().writes, 1, "已证 entry 不得重算回写");
        assert_eq!(memo.stats().hits, 1);

        let rolled = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 11,
            }),
        );
        assert_eq!(rolled, cold, "失效后必须退化为真冷同案同果");
        assert_eq!(memo.len(), 0, "segment.end == e_src 不得留 stable memo");
        assert_eq!(memo.stats().writes, 1, "可变尾冷算不得回写");
        assert!(memo.stats().invalidations >= 1);
    }

    /// #69 5b / T2（链①c）：坐标/MACD 前缀未覆盖 A/C 时只冷算，不得写入假阴性；
    /// 输入到齐后同一调用可产事件并开始缓存。
    #[test]
    fn pan_memo_incomplete_macd_does_not_negative_cache() {
        let fixture = pan_provider_fixture();
        let mut memo = PanMemo::default();
        let incomplete_src = &fixture.close_src[..10];
        let incomplete = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist[..10],
            &fixture.dif[..10],
            incomplete_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        let incomplete_pan = incomplete
            .iter()
            .find(|event| event.kind == NestDivergenceKind::Consolidation)
            .expect("结构候选仍须按冷路返回");
        assert!(!incomplete_pan.divergence_confirmed);
        assert_eq!(memo.len(), 0);
        assert_eq!(memo.stats().writes, 0);

        let complete = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        let complete_pan = complete
            .iter()
            .find(|event| event.kind == NestDivergenceKind::Consolidation)
            .expect("输入到齐后必须保留盘背事件");
        assert!(
            complete_pan.divergence_confirmed,
            "不得复用未到齐时的假阴性"
        );
        assert_eq!(memo.len(), 1);
        assert_eq!(memo.stats().writes, 1);
        assert_eq!(memo.stats().hits, 0);
    }

    /// #69 5b / T3（链②a）：目标 block 后 0/1 块仍属可变尾；恰有两个后继块才可驻留。
    #[test]
    fn pan_memo_requires_two_successor_blocks() {
        let fixture = pan_provider_fixture();
        for successor_count in 0..=2 {
            let blocks = &fixture.blocks[..=successor_count];
            let cold = provide_nest_candidate_events(
                1,
                &fixture.projection,
                blocks,
                &fixture.legs,
                &fixture.view,
                &fixture.hist,
                &fixture.dif,
                &fixture.close_src,
            );
            let mut memo = PanMemo::default();
            let resident = provide_nest_candidate_events_resident(
                1,
                &fixture.projection,
                blocks,
                &fixture.legs,
                &fixture.view,
                &fixture.hist,
                &fixture.dif,
                &fixture.close_src,
                Some(PanResidence {
                    memo: &mut memo,
                    freeze_boundary_src: 12,
                }),
            );
            assert_eq!(resident, cold);
            assert_eq!(
                memo.len(),
                usize::from(successor_count == 2),
                "{successor_count} 个后继块的驻留资格错误"
            );
        }
    }

    /// #69 5b / T3（链②b）：追加第四块不清已证三块；后继消失或身份重折须立即失效。
    #[test]
    fn pan_memo_block_shrink_rewrite_and_append_discipline() {
        let fixture = pan_provider_fixture();
        let mut memo = PanMemo::default();
        let first = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        assert_eq!(memo.len(), 1);

        let mut appended = fixture.blocks.clone();
        appended[2].status = MoveStatus::Completed;
        appended.push(MoveBlock {
            start_center: 6,
            end_center: 6,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Active,
        });
        let appended_out = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &appended,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert_eq!(appended_out, first);
        assert_eq!(memo.stats().hits, 1, "追加尾块不得清已证目标三块");
        assert_eq!(memo.stats().invalidations, 0);

        let shrunk = &fixture.blocks[..2];
        let cold_shrunk = provide_nest_candidate_events(
            1,
            &fixture.projection,
            shrunk,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let resident_shrunk = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            shrunk,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert_eq!(resident_shrunk, cold_shrunk);
        assert!(memo.is_empty(), "两个后继块门消失后不得残留 entry");
        assert_eq!(memo.stats().writes, 1, "回缩后的冷算不得回写");
        assert!(memo.stats().invalidations >= 1);

        let mut memo = PanMemo::default();
        let _ = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        let mut rewritten = fixture.blocks.clone();
        rewritten[1].kind = MoveKind::Consolidation;
        rewritten[1].dir = None;
        let cold_rewritten = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &rewritten,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let resident_rewritten = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &rewritten,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        assert_eq!(resident_rewritten, cold_rewritten);
        assert!(memo.stats().invalidations >= 1, "后继块身份重折必须失效");
        assert_eq!(memo.stats().writes, 2, "重折后须按新身份冷算回写");
    }

    /// #69 5b / T4：`judge_at` 每次按当前调用物化；pan 核所读 segment 前缀改写时，
    /// 即便候选末段身份未变也不得命中旧值。
    #[test]
    fn pan_memo_rematerializes_dynamic_fields_and_invalidates_read_prefix() {
        let fixture = pan_provider_fixture();
        let mut memo = PanMemo::default();
        let _ = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );

        let mut later_view = fixture.view.clone();
        later_view.query.as_of = 40;
        let later = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &later_view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert_eq!(memo.stats().hits, 1);
        assert_eq!(memo.stats().writes, 1);
        assert_eq!(
            later
                .iter()
                .find(|event| event.kind == NestDivergenceKind::Consolidation)
                .expect("缓存事件仍须按当前调用物化")
                .judge_at,
            40
        );

        let mut rewritten_legs = fixture.legs.clone();
        rewritten_legs[0].lo -= 10;
        let cold_rewritten = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &rewritten_legs,
            &later_view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let resident_rewritten = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &rewritten_legs,
            &later_view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert_eq!(resident_rewritten, cold_rewritten);
        assert!(memo.stats().invalidations >= 1, "A/C 读前缀改写必须失效");
        assert_eq!(memo.stats().writes, 2);
    }

    /// #69 5b / T4：锚 sidecar 命中时按当前供给重解；provider window 改变即视为另一 run
    /// 语境，不得共享旧 entry。
    #[test]
    fn pan_memo_rematerializes_anchor_and_separates_run_window() {
        let fixture = pan_provider_fixture();
        let mut memo = PanMemo::default();
        let first = provide_nest_candidate_events_ext_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            &[],
            &[],
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        let first_pan = first
            .iter()
            .find(|ext| ext.event.kind == NestDivergenceKind::Consolidation)
            .expect("夹具必须产 pan");
        assert_eq!(
            (first_pan.extreme_price, first_pan.group_anchor),
            (None, None)
        );

        let fractals = [Fractal {
            kind: FractalKind::Bottom,
            source_index: 11,
            timestamp: 11,
            price: 300,
        }];
        let merged: Vec<_> = (0..32).map(mbar).collect();
        let anchored = provide_nest_candidate_events_ext_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            &fractals,
            &merged,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        let anchored_pan = anchored
            .iter()
            .find(|ext| ext.event.kind == NestDivergenceKind::Consolidation)
            .expect("命中后事件仍须存在");
        assert_eq!(
            (anchored_pan.extreme_price, anchored_pan.group_anchor),
            (Some(300), Some(11))
        );
        assert_eq!(memo.stats().hits, 1);
        assert_eq!(memo.stats().writes, 1);

        let mut other_run_view = fixture.view.clone();
        other_run_view.query.coordinate_window.end += 1;
        other_run_view.cache_key = C2CacheKey::from_query(&other_run_view.query).unwrap();
        let other_run = provide_nest_candidate_events_ext_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &other_run_view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            &fractals,
            &merged,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert!(other_run.iter().any(|ext| {
            ext.event.kind == NestDivergenceKind::Consolidation
                && ext.event.provider_window == (0, 24)
        }));
        assert!(memo.stats().invalidations >= 1);
        assert_eq!(memo.stats().writes, 2, "新 run 语境须冷算后独立回写");
    }

    /// #69 5b / T4：完整执行后的稳定 force-false 可缓存；命中不得把 false 改写为猜测值。
    #[test]
    fn pan_memo_caches_complete_stable_force_false() {
        let fixture = pan_provider_fixture();
        let mut non_divergent_hist = fixture.hist.clone();
        non_divergent_hist[9] = -10.0;
        non_divergent_hist[10] = 20.0;
        non_divergent_hist[11] = -10.0;
        let mut memo = PanMemo::default();
        for expected_hits in 0..=1 {
            let events = provide_nest_candidate_events_resident(
                1,
                &fixture.projection,
                &fixture.blocks,
                &fixture.legs,
                &fixture.view,
                &non_divergent_hist,
                &fixture.dif,
                &fixture.close_src,
                Some(PanResidence {
                    memo: &mut memo,
                    freeze_boundary_src: 12,
                }),
            );
            let event = events
                .iter()
                .find(|event| event.kind == NestDivergenceKind::Consolidation)
                .expect("结构事件仍应存在");
            assert!(!event.divergence_confirmed);
            assert_eq!(memo.stats().hits, expected_hits);
        }
        assert_eq!(memo.stats().writes, 1);
        assert_eq!(memo.len(), 1);
    }

    /// #69 5b / T6：人为污染 resident 后热路必须显出差异，而显式 `None` 仍返回真冷 oracle，
    /// 且 forced 调用不读写该 memo。
    #[test]
    fn pan_memo_forced_none_is_true_cold_oracle() {
        let fixture = pan_provider_fixture();
        let cold = provide_nest_candidate_events(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let mut memo = PanMemo::default();
        let _ = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        memo.poison_for_test();
        let poisoned = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 13,
            }),
        );
        assert_ne!(poisoned, cold, "污染须能被 shadow 比对观察到");
        let stats_before_forced = memo.stats();
        let len_before_forced = memo.len();
        let forced = provide_nest_candidate_events_resident(
            1,
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            None,
        );
        assert_eq!(forced, cold);
        assert_eq!(memo.stats(), stats_before_forced);
        assert_eq!(memo.len(), len_before_forced);
    }
}
