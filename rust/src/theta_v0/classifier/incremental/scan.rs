//! 增量塔单级扫描续接的构件（`incremental` 的扫描族）：frontier pop、cp 生命周期继承、
//! tail extend、水线推进与两条对齐/契约护栏。
//!
//! 编排在 [`super::tower::scan_and_extend_level`]，本模块只提供其逐步构件。

use super::*;

/// 快照本级输入（下 bar 比对 frontier 变异）。
///
/// ★A3 §3.2 证书化：`cached_units[..dirty_from] == units[..dirty_from]`（前缀可证不可变，
/// 上 bar 04 已写同值）⟹ 前缀无需重拷，truncate + extend tail = O(tail)。三重 min 保证
/// 终长 == `units.len()`（含 units 回缩）。
/// `debug_assert` = debug/test 护栏（release profile 按 Rust 语义剥离——省下的即此全前缀比较）。
pub(super) fn sync_cached_units(lc: &mut LevelCache, units: &[UnitRange], dirty_from: usize) {
    stage_profile::time("04_cached_units_copy", || {
        let keep = dirty_from.min(lc.cached_units.len()).min(units.len());
        debug_assert!(
            units[..keep] == lc.cached_units[..keep],
            "dirty_from 证书违反：cached_units[..{}] 应等于 units 前缀（bit-exact 护栏）",
            keep
        );
        lc.cached_units.truncate(keep);
        lc.cached_units.extend_from_slice(&units[keep..]);
    });
}

/// [`pop_frontier_window`] 的产出：被 pop 的 frontier 末窗口尾段 + 本级重扫起点。
pub(super) struct PoppedFrontier {
    /// pop 掉的 `upper_moves` 尾段（on2w2 E2/E2a 逐值判据的左端；无 pop ⟹ 空）。
    pub(super) upper: Vec<LeveledMove>,
    /// pop 掉的 `cp_ownership` 尾段（生命周期继承的候选源；无 pop ⟹ 空）。
    pub(super) cp: Vec<CpScanOwnership>,
    /// 本 bar 是否发生 pop（A3 `on_pop_rescan` 探针的门）。
    ///
    /// `allow(dead_code)`：唯一读点在 `#[cfg(test)]` 探针块内，release 构建下该块剥离 ⟹
    /// 字段未读。原码里它是循环内的 `had_emitted_window` 局部量（局部量无 dead_code 检查），
    /// 抽为结构体字段后才暴露；语义与读点均未变。
    #[allow(dead_code)]
    pub(super) had_window: bool,
    /// `compose_level_resume` 的续扫起点（`scan_cursor.resume_from`，pop 前读取）。
    pub(super) resume_start: usize,
}

/// ★frontier 修复（task #47/#21，区间套.pdf 六~十节裁决②）：resume 起点用 `resume_from`
/// （上次扫描最后一个成立窗口的起点）**而非** `consumed`——`consumed` 越过了最后一个成立
/// 窗口，把它当 sealed prefix，但该窗口第三段可能是 frontier（新 bar 后其后续段落使全量非
/// 重叠扫描产出不同中枢）。回退：从 `resume_from` 重扫 ⟹ 最后一个（frontier）中枢每 bar 重算，
/// 真正 sealed（后面又出现成立窗口）后自然稳定。对应地 **pop 最后一个 center/upper_move**
/// （它会被重扫重新产出），`prefix_count` 用 pop 后的长度（ordinal 接续，全量/增量同 ID）。
///
/// 保守正确性（PDF §九增量等价定理）：`resume_from <= consumed` 恒成立（窗口起点 ≤ 退出点），
/// 从更早处重扫产出的 tail ⊇ 从 consumed 重扫的 tail（多含重算的末窗口）。cascade_reset 后
/// `scan_cursor = default`（resume_from=0=consumed）⟹ 无回退，从 0 全扫（bit-exact 退化）。
/// 无成立窗口时 `resume_from == 上次 start_i`（无中枢可 pop），guard `resume_from < consumed`
/// 为假 ⟹ 不 pop、从 `resume_from`(=上次 start_i) 续扫（续进语义，仅不成立支推进过的区间）。
///
/// ★on2w2：本级 `upper_moves`（=tower[level+1]，forest 输入）字节变更判据。`upper_moves` 前缀
/// `[..prefix_count]` 不可变（§16 confirmed），本 bar 只改尾部：pop 掉 [`PoppedFrontier::upper`]
/// （末窗产出）后 extend `tail_upper`（重扫产出）⟹ **变更 ⟺ 两者不逐值相等**。无 pop 时
/// `upper` 空 ⟹ 变更 ⟺ `tail_upper` 非空（纯追加，=E2）。故此处在 pop 前捕获尾段以供比对。
pub(super) fn pop_frontier_window(lc: &mut LevelCache) -> PoppedFrontier {
    let resume_start = lc.scan_cursor.resume_from;
    let had_window = lc.scan_cursor.resume_from < lc.scan_cursor.consumed;
    let mut upper: Vec<LeveledMove> = Vec::new();
    let mut cp_popped: Vec<CpScanOwnership> = Vec::new();
    if had_window {
        // pop 最后成立窗口的**全部**产出（frontier 域 = 整窗，重扫从窗口起点重产）——
        // ★#148 升级重切后一窗可产 k 个子中枢（`last_window_emitted`），只 pop 1 会残留旧
        // 子中枢与重扫产出重复。前缀不变量不破（pop 的是尾部整窗）。
        let pop_n = lc.scan_cursor.last_window_emitted;
        debug_assert!(
            pop_n >= 1 && lc.centers.len() >= pop_n && lc.upper_moves.len() >= pop_n,
            "had_emitted_window ⟹ 末窗口产出（pop_n={pop_n}）可回退"
        );
        let cs = Rc::make_mut(&mut lc.centers);
        cs.truncate(cs.len().saturating_sub(pop_n));
        let cp = Rc::make_mut(&mut lc.cp_ownership);
        let cp_keep = cp.len().saturating_sub(pop_n);
        cp_popped = cp[cp_keep..].to_vec();
        cp.truncate(cp_keep);
        let um = Rc::make_mut(&mut lc.upper_moves);
        let keep = um.len().saturating_sub(pop_n);
        upper = um[keep..].to_vec(); // on2w2：pop 前捕获（与重扫 tail_upper 逐值比）。
        um.truncate(keep);
        // ★on2w2-cascade：win_meta 与 centers/upper_moves 1:1 同步 pop（末窗整窗，重扫重产）。
        lc.win_meta.truncate(lc.win_meta.len().saturating_sub(pop_n));
    }
    PoppedFrontier { upper, cp: cp_popped, had_window, resume_start }
}

/// frontier pop/recompose 若产出同一个 `B_p`/`c_p` 身份，继承已扫描对象态，只从 `dirty_from` 推进。
///
/// Closed 证书若落入 dirty 后缀则不可继承，必须从 departure 重判；证书完全位于稳定前缀才保留。
/// 返回本级 `advance_cp_lifecycles` 的推进起点 `lifecycle_scan_from`。
pub(super) fn reinherit_cp_lifecycles(
    lc: &mut LevelCache,
    tail_cp: &mut [CpScanOwnership],
    popped_cp: &[CpScanOwnership],
    dirty_from: usize,
    level_idx: usize,
) -> usize {
    let dirty_invalidation = recursive_tower::invalidate_cp_lifecycle_dirty_dependencies(
        Rc::make_mut(&mut lc.cp_ownership).as_mut_slice(),
        dirty_from,
    );
    cp_replay_diagnostics::record_dirty_invalidation(
        level_idx,
        dirty_invalidation.pending_fallbacks,
        dirty_invalidation.certificate_clear_recomputes,
    );
    let mut lifecycle_scan_from = dirty_invalidation.scan_from;
    for object in tail_cp.iter_mut() {
        let prior = popped_cp.iter().find(|prior| {
            prior.b_center_id == object.b_center_id
                && prior.b_center == object.b_center
                && prior.departure_move_id == object.departure_move_id
                && prior.departure_interval == object.departure_interval
        });
        let prior_is_stable = prior.is_some_and(|prior| {
            recursive_tower::cp_lifecycle_dependencies_stable_before(prior, dirty_from)
        });
        if prior_is_stable {
            let prior = prior.expect("prior_is_stable 蕴含 prior Some");
            cp_replay_diagnostics::record_tail_reinherit(level_idx);
            object.lifecycle = prior.lifecycle;
            object.cp_certificate_confirm_src = prior.cp_certificate_confirm_src;
            object.c_structure = prior.c_structure;
            object.third_class_in_c = prior.third_class_in_c;
            object.full_trend_evidence = prior.full_trend_evidence.clone();
            object.full_trend_c_qualified = prior.full_trend_c_qualified.clone();
        } else if let Some(departure) = object.departure_move_id {
            lifecycle_scan_from = lifecycle_scan_from.min(departure.ordinal as usize + 1);
        }
    }
    lifecycle_scan_from
}

/// 追加重扫尾段到已缓存前缀（前缀不可变，仅尾部追加）⟹ 累积 centers/upper == 全量扫描结果。
///
/// `Rc::make_mut`：`strong_count==1` ⟹ 原地 extend O(tail)；>1 ⟹ 写时复制（bit-exact）。
/// `win_meta` 与 `centers`/`upper_moves` 同步 extend（on2w2-cascade 1:1 对齐不变量维持）。
pub(super) fn extend_level_tail(
    lc: &mut LevelCache,
    tail_centers: Vec<Center>,
    tail_upper: Vec<LeveledMove>,
    tail_cp: Vec<CpScanOwnership>,
    tail_metas: Vec<WinMeta>,
) {
    stage_profile::time("06_extend_centers_upper", || {
        Rc::make_mut(&mut lc.centers).extend(tail_centers);
        Rc::make_mut(&mut lc.upper_moves).extend(tail_upper);
        Rc::make_mut(&mut lc.cp_ownership).extend(tail_cp);
        lc.win_meta.extend(tail_metas);
    });
}

/// ★#93 水线推进（字段文档见 [`LevelCache::confirmed_watermark`]）：
/// `w_nat = len - last_window_emitted`——本 bar frontier 末窗口（下 bar pop 重产）排除。
/// cascade bar 只允许收缩（前缀 `min(P)` 已在 cascade 分支落账），非 cascade bar 可增长。
///
/// **必须在 `lc.scan_cursor = new_cursor` 之后调用**（读的是本 bar 新 cursor 的 `last_window_emitted`）。
pub(super) fn advance_confirmed_watermark(lc: &mut LevelCache, cascade_reset: bool) {
    let w_nat = lc
        .upper_moves
        .len()
        .saturating_sub(lc.scan_cursor.last_window_emitted);
    lc.confirmed_watermark = if cascade_reset {
        lc.confirmed_watermark.min(w_nat)
    } else {
        w_nat
    };
}

/// 增量塔一一对应护栏：`centers`/`upper_moves`/`cp_ownership`/`win_meta` 同长（debug/test）。
pub(super) fn debug_assert_level_alignment(lc: &LevelCache) {
    debug_assert!(
        lc.centers.len() == lc.upper_moves.len()
            && lc.centers.len() == lc.cp_ownership.len()
            && lc.centers.len() == lc.win_meta.len(),
        "增量塔：centers/upper_moves/cp_ownership/win_meta 一一对应"
    );
}

/// ★on2w2-cascade §3.1 契约：`cached_second_count`（保留 parent 前缀数）须 <= pop 后 `prefix_count`
/// （否则缓存越过 confirmed 边界，`extract_second_resume` 单调守卫会重置）。cascade truncate 设
/// `cached_second_count = P = win_meta 保留数 <= upper_moves 保留数 == prefix_count`（同一 `truncate(p)`）。
pub(super) fn debug_assert_second_cache_contract(lc: &LevelCache, prefix_count: usize) {
    debug_assert!(
        lc.cached_second_count <= prefix_count,
        "§3.1 契约违反：cached_second_count={} > prefix_count={}",
        lc.cached_second_count, prefix_count
    );
}
