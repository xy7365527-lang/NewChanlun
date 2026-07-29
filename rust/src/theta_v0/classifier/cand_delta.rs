//! P1/P52/P53 Cand^δ 只读驱动器：`cand_delta_tower`（全量）/ `cand_delta_tower_cached`
//!（增量缓存序列）/ `cand_delta_entry_tower`（P53 放宽入口集合）/
//! `cp_recall_upper_bound_audit`（P52 召回上界）。
//!
//! ★纯增量只读层：不改 `classify` 任何行为；判据零分叉见 recursive_tower.rs P1 段头铁律。

use super::pipeline::{segment_to_unit, unit_to_segment};
use super::*;

/// P1 谓词闭包驱动器（strict-nesting-divergence-plan-20260708 §P1）：对既有分类输出逐级跑
/// [`recursive_tower::level_cand_delta`]（Cand^δ_ℓ 背驰段谓词，「级别→A/C 定位配对」层）。
///
/// 每级输入重建与 `classify_impl`（pipeline.rs）单一来源同构：
/// - ℓ0：`l0.segments`（L0 线段账本；anchor=None ⟹ 段方向即锚方向，L0 域定理）；
/// - ℓ≥1：`units = project_to_units(&tower_snapshots[ℓ], &levels[ℓ-1].moves)`
///   （`tower_snapshots[ℓ]` = 第 ℓ 级输入塔 = 第 ℓ-1 级 upper_moves，classify_impl 同步
///   index 不变量）+ `anchors[i] = center_own_dir_at(levels[ℓ-1].moves, i)`（Q7-#1 裁定C
///   同一 provenance）+ `unit_to_segment` 还原（与 `extract_first_third_for_level` 同口径）；
/// - hist/dif/closes_tick/close_src：与 classify_impl 同一 `compute_macd` 路径重建。
///
/// ★纯增量只读层：不改 classify 任何行为；判据零分叉见 recursive_tower.rs P1 段头铁律。
/// 全量/增量分类输出均适用（增量塔 bit-exact 于全量 ⟹ 重建输入逐值相同）。
pub fn cand_delta_tower(
    l0: &ParseLayer,
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
    config: &ThetaConfig,
) -> Vec<Vec<recursive_tower::CandDeltaEvent>> {
    let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
    let series = divergence::compute_macd(&closes, &config.macd);
    let closes_tick: Vec<Tick> = l0.merged_bars.iter().map(|b| b.close).collect();
    cand_delta_tower_with_series(
        l0,
        classification,
        tower_snapshots,
        config,
        &series.hist,
        &series.dif,
        &closes_tick,
        &close_src,
    )
}

/// [`cand_delta_tower`] 的缓存序列变体（校验 bin 每 bar 重放热路径，纯性能——判据零分叉）。
///
/// 全量版每 bar 3×O(merged_bars) collect + `compute_macd` 全量重算 ⟹ 因果重放 O(n²)（
/// strict_nest_check 实测分段耗时线性增长，外推 20h+）。本变体直接只读借用
/// [`TowerCache`] 的锁步增量序列（`closes_tick`/`close_src`/`macd_hist`/`macd_dif`——
/// 均 bit-exact 等价全量重算，见各字段文档与 `compute_macd_hist_incremental` 增量证明），
/// 消掉 per-bar O(n) 项。
///
/// ★缓存一致性守卫（over-invalidate 方向）：任一序列长度 ≠ `merged_bars.len()` 或
/// `macd_state_len` 不合覆盖契约（n≤1 ⟹ n；n≥2 ⟹ n-1，见 `cache_series_ok`）⟹
/// 退化调全量版（bit-exact，非增量）。`DIAG_CANDCACHE=1` 时
/// 每 bar 与全量版对拍断言（前缀验证用，同 `DIAG_L0UNITS` 模式）。
pub fn cand_delta_tower_cached(
    l0: &ParseLayer,
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
    config: &ThetaConfig,
    cache: &TowerCache,
) -> Vec<Vec<recursive_tower::CandDeltaEvent>> {
    let n = l0.merged_bars.len();
    if !cache_series_ok(cache, n) {
        return cand_delta_tower(l0, classification, tower_snapshots, config);
    }
    let out = cand_delta_tower_with_series(
        l0,
        classification,
        tower_snapshots,
        config,
        &cache.macd_hist,
        &cache.macd_dif,
        &cache.closes_tick,
        &cache.close_src,
    );
    if std::env::var("DIAG_CANDCACHE").is_ok() {
        let full = cand_delta_tower(l0, classification, tower_snapshots, config);
        assert_eq!(
            out, full,
            "DIAG_CANDCACHE：缓存序列版与全量重算版 Cand^δ 事件不一致（缓存序列漂移）"
        );
    }
    out
}

/// P53 放宽后的 Cand 入口集合。
///
/// 旧 [`cand_delta_tower`] 继续输出算法背驰事件并保持 `cand_delta` 的历史真值；本入口把每级
/// 生产生命周期已经由 `judge_third_cert` 闭合的稳定 `B_p/c_p` 对象全部事件化。两者分开可
/// 保留 P51 的 20 个事件侧分类复核，也不会为无背驰事件的对象伪造背驰字段。
pub fn cand_delta_entry_tower(
    classification: &Classification,
    legacy_events: &[Vec<recursive_tower::CandDeltaEvent>],
) -> Vec<Vec<recursive_tower::CandDeltaEntryEvent>> {
    classification
        .levels
        .iter()
        .enumerate()
        .map(|(level, state)| {
            recursive_tower::relaxed_cand_delta_entries(
                level as u32,
                &state.cp_ownership,
                legacy_events.get(level).map(Vec::as_slice).unwrap_or(&[]),
            )
        })
        .collect()
}

/// 缓存序列一致性守卫（parser BUG-04 修复：`macd_state_len` 期望值按
/// [`compute_macd_hist_incremental`] 的**真实覆盖契约**校验——n≤1 时 state 覆盖全部
/// n 个 close；n≥2 时 state 只覆盖稳定前缀 `n-1`（尾 bar 不稳定，hist/dif 才含尾 bar）。
/// 旧守卫 `macd_state_len == n` 在 n≥2 恒假 ⟹ 缓存分支死代码，每 bar 退化全量重算
/// O(n²)，逐 bar 校验热路径失效）。over-invalidate 方向不变：任一不齐 ⟹ 调用方退化
/// 全量版（bit-exact）。
pub(super) fn cache_series_ok(cache: &TowerCache, n: usize) -> bool {
    let expected_state_len = if n <= 1 { n } else { n - 1 };
    cache.macd_state_len == expected_state_len
        && cache.closes_tick.len() == n
        && cache.close_src.len() == n
        && cache.macd_hist.len() == n
        && cache.macd_dif.len() == n
}

/// L≥1 的 `level_cand_delta` 几何输入派生：上级塔投影 → units → (Segment 投影, 方向锚)。
///
/// units 承担线段角色（裁定 A），方向锚与投影同源（`center_own_dir_at`，与 `classify_impl` /
/// 增量塔的 `derive_units_anchors` 同一 provenance）。
fn level_ge1_cand_inputs(
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
    lvl: usize,
) -> (Vec<Segment>, Vec<Option<Direction>>) {
    let pb = &classification.levels[lvl - 1].moves;
    let units = project_to_units(&tower_snapshots[lvl], pb);
    let anchors: Vec<Option<Direction>> =
        (0..units.len()).map(|i| decompose::center_own_dir_at(pb, i)).collect();
    let segs: Vec<Segment> = units.iter().map(unit_to_segment).collect();
    (segs, anchors)
}

/// P1 驱动器核心（序列注入版）：`cand_delta_tower`（全量重算）与
/// `cand_delta_tower_cached`（增量缓存）共用单一事件提取路径（判据零分叉）。
#[allow(clippy::too_many_arguments)]
pub(super) fn cand_delta_tower_with_series(
    l0: &ParseLayer,
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
    config: &ThetaConfig,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
) -> Vec<Vec<recursive_tower::CandDeltaEvent>> {
    let mut out = Vec::with_capacity(classification.levels.len());
    for (lvl, ls) in classification.levels.iter().enumerate() {
        debug_assert!(
            lvl < tower_snapshots.len(),
            "tower_snapshots 与 levels 同构（classify_impl 不变量）"
        );
        let evs = if lvl == 0 {
            recursive_tower::level_cand_delta(
                0,
                &ls.centers[..],
                Some(&ls.cp_ownership[..]),
                &l0.segments,
                Some(&tower_snapshots[lvl]),
                None,
                hist,
                dif,
                closes_tick,
                close_src,
                config.divergence_gauge,
            )
        } else {
            let (segs, anchors) = level_ge1_cand_inputs(classification, tower_snapshots, lvl);
            recursive_tower::level_cand_delta(
                lvl as u32,
                &ls.centers[..],
                Some(&ls.cp_ownership[..]),
                &segs,
                Some(&tower_snapshots[lvl]),
                Some(&anchors),
                hist,
                dif,
                closes_tick,
                close_src,
                config.divergence_gauge,
            )
        };
        out.push(evs);
    }
    out
}

/// P52 只读召回上界：不经过 `level_cand_delta` / `cand_delta` 事件入口，直接在每级稳定
/// `B_p/c_p` 对象全集上重判 leave/retest 几何。
pub fn cp_recall_upper_bound_audit(
    l0: &ParseLayer,
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
) -> Vec<recursive_tower::CpRecallAuditCase> {
    let mut out = Vec::new();
    for (level, state) in classification.levels.iter().enumerate() {
        let Some(unit_moves) = tower_snapshots.get(level) else {
            continue;
        };
        if level == 0 {
            let units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
            out.extend(recursive_tower::audit_cp_recall_upper_bound(
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
            out.extend(recursive_tower::audit_cp_recall_upper_bound(
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
