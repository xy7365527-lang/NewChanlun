//! C2 CompletedMove 消费 seam（tasks #73/#74）。
//!
//! 本模块是只读消费层，不改写递归塔。D1 要求调用方显式把当前扩展窗口投影为不可变
//! exact-three seed；D2 只消费 D3 已结裁的 `MoveBlock.dir: Option<Direction>`。四个旧 seam
//! provider 不在本模块出现，方向版本唯一绑定 `central-ggdd-v1`。
//!
//! #630：生产段按域拆分为子模块（本文件保留查询/版本元类型与顶层 assemble seam）——
//! `projection`（D1 投影 + LowerLeg）、`confirm`（趋势背驰确认核心）、`pan` + `pan_provider`
//! （盘整/趋势候选事件 provider）。全部 `pub` 项经下方 `pub use` 原样再导出，外部消费路径
//! （`classifier::level_view::X`）零改动。
//!
//! #630 修复轮（影子评审 MEDIUM-2）：4 个子域由 classifier 兄弟文件改为本模块内部子模块
//! （目录模块，`level_view.rs` → `level_view/mod.rs`），43 项 `pub(super)` 的 `super` 由
//! classifier 变为 level_view——可见域从「classifier 24 个兄弟模块」收窄到「level_view 子树」
//! （先例 #633 批7 `incremental/`）。`level_view_store`（`ConfirmCursor`/`ConfirmCursorStore`）
//! 不在本次迁移范围内，仍是 classifier 兄弟模块。

mod confirm;
mod pan;
mod pan_provider;
mod projection;

use super::super::types::{Direction, MoveKind, Side};
use super::decompose::{MoveBlock, MoveStatus};
use super::recursive_tower::map_src_to_close_idx;
use std::collections::BTreeSet;

/// #497：`ConfirmCursor`/`ConfirmCursorStore`/`ConfirmState` 归位 `level_view_store`；
/// 本行保 pub 路径不变（`p123_fast_replay.rs` 等既有消费方零改动）。
pub use super::level_view_store::{ConfirmCursor, ConfirmCursorStore, ConfirmState};

use projection::leg_as_segment;
/// #630：`projection` 子模块的 pub 项原样再导出（D1 投影 + LowerLeg）。
pub use projection::{
    lower_legs_from, project_extended_windows, project_extended_windows_carried_only,
    ExactThreeProjection, ExactThreeSeed, LowerLeg, ProjectionError, ProjectionMaterial,
    SeedCoreProvenance,
};

#[cfg(test)]
use confirm::{confirm_core_calls, reset_confirm_core_calls, trend_confirm_time};
use confirm::{trend_confirm_state, trend_confirm_state_core};
/// #630：`confirm` 子模块的 pub 项原样再导出（趋势背驰确认核心）。
pub use confirm::{
    ConfirmKey, ConfirmResidence, DivergencePair, DivergencePairId, NestCandidateEvent,
    NestDivergenceKind, PairConfirmState,
};

/// #630：`pan` 子模块的 pub 项原样再导出（D2 provider + memo resident seam；
/// `PanMemo`/`PanResidence` 由 `p123_fast_replay.rs` 直接消费，必须保 `pub`）。
/// `PanMemoStats`（#630 修复轮 MEDIUM-1）：原拆分遗漏再导出，`classifier::level_view::
/// PanMemoStats` 路径曾静默断裂（无消费者未破编译，仍是四件套自设口径的反例）。
pub use pan::{provide_divergence_pairs, PanMemo, PanMemoStats, PanResidence};

/// #630：`pan_provider` 子模块的 pub 项原样再导出（typed 候选事件 provider 入口）。
pub use pan_provider::{
    provide_nest_candidate_events, provide_nest_candidate_events_ext,
    provide_nest_candidate_events_ext_resident, provide_nest_candidate_events_resident,
    NestCandidateEventExt,
};

/// #630：`level_view/tests/*` 经由本模块 `use super::*` 两级 glob 链（`tests/mod.rs` →
/// `tests/<submodule>.rs`）访问的私有/跨域符号——拆分前这些符号与测试同处一个模块，
/// 私有可见性天然覆盖后代模块；拆分后改为显式 `pub(super)` + 本处 `#[cfg(test)]` 再导入，
/// 使 glob 链零改动（`level_view/tests/` 为 #497 已验收面，非必要不动）。
#[cfg(test)]
use super::super::types::{Bar, Center, Fractal, Tick};
#[cfg(test)]
use super::center::UnitRange;
#[cfg(test)]
use super::recursive_tower::ElementId;
#[cfg(test)]
use pan_provider::resolve_triple_anchor;

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
mod tests;
