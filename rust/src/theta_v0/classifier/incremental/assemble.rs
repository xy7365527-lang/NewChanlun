//! 增量塔单级 [`LevelState`] 装配的构件（`incremental` 的装配族）：BSP memo/resume 提取、
//! #110 投影层 stamping、下一级 units 投影与方向锚派生。
//!
//! 另含跨级共享的序列面载体 [`LevelSeries`]（MACD/坐标/area-memo 借用面）。

use super::*;

/// 本 bar 跨级共享的 MACD / 坐标 / area-memo 序列面（全部是借用，逐级只读传递）。
///
/// `hist`/`dif` 借 `cache.macd_{hist,dif}`，与循环内 `&mut cache.levels[i]` 是**不相交字段借用**
/// （原码同构，见报告 §1.2）。`closes_tick`/`close_src` 借主函数 `mem::take` 出来的栈上缓冲，
/// `area_cache` 借主函数栈上的 `RefCell`（T9：必须活过整个循环，放回在循环之后）。
pub(super) struct LevelSeries<'a> {
    pub(super) hist: &'a [f64],
    pub(super) dif: &'a [f64],
    pub(super) closes_tick: &'a [Tick],
    pub(super) close_src: &'a [usize],
    pub(super) area_cache: &'a RefCell<AreaCache>,
    /// B3 #4 area-memo 的确认边界（本 bar `macd_state_len`）——只对 `end < stable_len` 的查询读写缓存。
    pub(super) stable_len: usize,
}

/// [`extract_level_bsp`] 的本级几何输入（`units` 承担线段角色的级别-N 判据面）。
pub(super) struct LevelBspInputs<'a> {
    pub(super) l0: &'a ParseLayer,
    pub(super) config: &'a ThetaConfig,
    pub(super) units: &'a [UnitRange],
    pub(super) units_anchors: &'a [Option<Direction>],
    pub(super) is_l0: bool,
}

/// BSP 提取（同 `classify_impl`：L0 线段层 + 递归组装层）。
///
/// ★增量接入：传预计算 hist（从 cache 增量产出），避免 `extract_signals` 内部全量 `compute_macd`。
///
/// ★memo 缓存（工位 E，O(n²) 主导根因——profile 坐实 bsp t=0.279s@16K，53% of tower）：
/// BSP 是 (centers, segments, upper_moves, hist, close_src) 的纯函数。其中 centers/segments/
/// upper_moves 跨 bar 单调追加（前缀不可变）；hist/close_src 仅尾 bar（不稳定，inclusion 可改写）
/// 变化。但 **confirmed 线段/中枢/上级走势的 source_index 区间全部落在稳定前缀**——尾 bar 在
/// 所有 confirmed 元素区间之后，故不影响其 BSP 判定。⟹ 当 (centers.len, segments.len,
/// upper_moves.len) 三者与上次一致时，BSP 逐字段 bit-identical（纯函数同输入同输出 + 尾 bar 不
/// 触及 confirmed 区间）。实测 16K bar 中 L0 segments 仅变 120 次（maxseg=134），其余 ~99.2%
/// bar 全量重算是冗余——memo 把 16K 次重算降为 O(段变化次数) 次，每次 O(S)，BSP 总成本坍缩近常数。
///
/// bit-exact 铁律：guard 命中 ⟹ 复用上次输出（纯函数同输入）；guard miss ⟹ 全量重算并刷新
/// 缓存。与无 memo 版逐字段相等（同 `extract_signals_with_hist`/`extract_second_for_level` 代码路径）。
///
/// ★裁定 A memo soundness：level≥1 的一/三类依赖本级输入 `units`（不止 upper_moves）——units
/// 尾部 append（新级别-N 单元，可能破最后中枢=新一类 C 段 / 与前段构成新三类回试对）会改变
/// 一/三类输出却**不**改 centers.len/upper_moves.len（新单元未凑齐三段窗口 ⟹ 无新中枢/上级走势）。
/// 故 level≥1 的结构长度键用 `units.len()`（L0 用 `segments.len()`）——append 变长即触 miss 重算。
/// frontier **同长改写**由 `cascade_reset`（frontier_mutated 比对，scan 窗覆盖全 units 因
/// `consumed+2 >= units.len()`）清 `cached_bsp_key` 兜底；回缩由 `last_input_len` 守卫触 cascade。
/// 三情形全覆盖。
pub(super) fn extract_level_bsp(
    lc: &mut LevelCache,
    inputs: &LevelBspInputs<'_>,
    moves: &[MoveBlock],
    prefix_count: usize,
    dirty_e: usize,
    series: &LevelSeries<'_>,
) -> (Rc<Vec<BspPoint>>, Rc<Vec<signal::PanDivCert>>) {
    let struct_len = if inputs.is_l0 {
        inputs.l0.segments.len()
    } else {
        inputs.units.len()
    };
    let bsp_key = (lc.centers.len(), lc.upper_moves.len(), struct_len);
    if lc.cached_bsp_key == Some(bsp_key) {
        // 07c：memo 命中 ⟹ `Rc::clone`（引用计数 O(1)），替代全量 `cached_bsp.clone()`。
        // Q4：pan_div 同批命中（同 key 守卫 ⟹ 同一 extract 产出的两半锁步复用）。
        return stage_profile::time("07c_bsp_memo_clone", || {
            (Rc::clone(&lc.cached_bsp), Rc::clone(&lc.cached_pan_div))
        });
    }
    let (mut b, pan) = extract_first_third_tail(lc, inputs, moves, prefix_count, dirty_e, series);
    let second = stage_profile::time("07b_extract_second", || {
        // ★07b frontier 门控：confirmed 前缀 parent 的 B2 缓存复用（跳过其重复背驰扫描），
        // 只对 frontier tail 每 bar 重算。消 confirmed-parent 全塔重扫 O(U²)。
        extract_second_resume(
            &mut lc.cached_second,
            &mut lc.cached_second_count,
            &lc.upper_moves[..],
            prefix_count,
            series.hist,
            series.close_src,
            series.area_cache,
            series.stable_len,
        )
    });
    b.extend(second);
    b.sort_by_key(|p| p.source_index);
    // miss 路径：`Rc::new` 一次，cache 与 LevelState 共享同一 buffer（消除旧 `b.clone()`）。
    let rc = Rc::new(b);
    let rc_pan = Rc::new(pan);
    lc.cached_bsp = Rc::clone(&rc);
    lc.cached_pan_div = Rc::clone(&rc_pan); // Q4：与 bsp 同批缓存（同 key）。
    lc.cached_bsp_key = Some(bsp_key);
    (rc, rc_pan)
}

/// ★on2w3-07a frontier-resume：confirmed 前缀段的一/三类点缓存复用，只重判 frontier tail
/// （消 07a O(n²) 主导项）。冻结边界锚 = `min(centers[prefix_count-2].end_index, dirty_e)`——
/// `moves`（= `decompose_resume` 输出，本级增量续折）作 blocks 单一来源（不重 decompose）。
///
/// segments 来源：L0 = `l0.segments`（有序）；L≥1 = `units → unit_to_segment` 投影（几何衰减，
/// resume 内 `debug_assert` 守 `end_index` 严格递增）+ anchors 平行传入（裁定 A：units 承担线段
/// 角色，复用 L0 判据）。cascade 清 `cached_first_third` 见 [`clear_level_cache`]/[`retain_level_prefix`]。
fn extract_first_third_tail(
    lc: &mut LevelCache,
    inputs: &LevelBspInputs<'_>,
    moves: &[MoveBlock],
    prefix_count: usize,
    dirty_e: usize,
    series: &LevelSeries<'_>,
) -> (Vec<BspPoint>, Vec<signal::PanDivCert>) {
    if inputs.is_l0 {
        stage_profile::time("07a_extract_signals_l0", || {
            signal::extract_first_third_resume(
                &mut lc.cached_first_third, &mut lc.cached_first_third_pan,
                &mut lc.cached_first_third_count, &lc.centers, &inputs.l0.segments, None, moves,
                prefix_count, dirty_e, series.hist, series.dif, series.closes_tick, series.close_src,
                inputs.config.divergence_gauge,
            )
        })
    } else {
        stage_profile::time("07a_extract_first_third_ln", || {
            let segs: Vec<Segment> = inputs.units.iter().map(unit_to_segment).collect();
            signal::extract_first_third_resume(
                &mut lc.cached_first_third, &mut lc.cached_first_third_pan,
                &mut lc.cached_first_third_count, &lc.centers, &segs, Some(inputs.units_anchors),
                moves, prefix_count, dirty_e, series.hist, series.dif, series.closes_tick,
                series.close_src, inputs.config.divergence_gauge,
            )
        })
    }
}

/// 下一级输入 = 上级走势塔投影（前缀来自缓存 `upper_moves` 前缀，尾部来自续扫）。
///
/// ★O(n²) 真修（#106）：增量投影——只对 `upper_moves` 新 tail 投影（`rmove.lo()/hi()` 递归整棵
/// 子树 O(nodes) 仅算新元素），前缀复用 `lc.projected_units`。返回给 `units` 的是 `Rc::clone`
/// （O(1)，替代全量 `projected_units.clone()` = O(units_L)/bar）。cascade_reset 已清空
/// `projected_units` ⟹ 退化全量。
///
/// ★A3 §2.5 证书化加固：pop 非 cascade 时 append-only 投影不感知 pop（保留上 bar frontier
/// 投影，隐式依赖 cascade 兜底）。显式 `truncate(prefix_count)` 丢弃 pop 掉的 frontier 投影，
/// 从 `prefix_count` 起重投影（O(tail)）——消除 did_extend 类漏判。cascade 时 `prefix_count=0`，
/// 与失效块的 `projected_units.clear()` 一致（`truncate(0) == clear`，冗余无害）。
///
/// `Rc::make_mut`：下一级 units 由返回的 `Rc::clone` 共享同一 buffer；本 bar 头部 truncate 前
/// 上轮 units 已被 loop 尾/下 bar 重赋 drop ⟹ `strong_count==1` ⟹ 原地 O(tail)；>1 ⟹ 写时
/// 复制（bit-exact 退化）。resume 契约要求 `cache.len() <= moves.len()`，`truncate(prefix_count)` 保证。
///
/// Q7（task #145）：方向源 = 本级中枢 ownership 块方向（`parent_blocks` = 本级刚 push 的
/// `LevelState.moves`，与 batch 同一 decompose 单一来源）。confirmed 前缀方向冻结
/// （`R(i-1,i)` sealed 后标签不变），frontier 由 `truncate(prefix_count)` 每 bar 重投影。
pub(super) fn project_next_level_units(
    lc: &mut LevelCache,
    parent_blocks: &[MoveBlock],
    prefix_count: usize,
) -> Rc<Vec<UnitRange>> {
    stage_profile::time("09_project_to_units_resume", || {
        let proj = Rc::make_mut(&mut lc.projected_units);
        proj.truncate(prefix_count);
        recursive_tower::project_to_units_resume(&lc.upper_moves[..], parent_blocks, proj);
    });
    // ★A3 投影证书护栏（debug/test）：truncate(prefix_count)+resume 必逐字段等全量 project_to_units。
    debug_assert!(
        *lc.projected_units == recursive_tower::project_to_units(&lc.upper_moves, parent_blocks),
        "投影证书违反：projected_units != 全量 project_to_units（truncate(prefix_count)/resume 破裂）"
    );
    // 10：`Rc::clone`（O(1)）。下一级借 `&units[..]` 只读；stage 09 头部 make_mut 时本 Rc 已 drop
    // （loop 尾重赋）⟹ 原地写不退化。
    stage_profile::time("10_projected_units_clone", || Rc::clone(&lc.projected_units))
}

/// Q7-#1 裁定C：锚资格与投影同源同步派生（deterministic 于 `(parent_blocks, len)` ⟹ resume
/// bit-exact：全量与增量在同一 `(pb, units.len())` 上得同一锚数组，无缓存陈旧面）。
pub(super) fn derive_units_anchors(parent_blocks: &[MoveBlock], units_len: usize) -> Vec<Option<Direction>> {
    (0..units_len)
        .map(|i| decompose::center_own_dir_at(parent_blocks, i))
        .collect()
}
