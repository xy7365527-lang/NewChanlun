//! 增量塔逐级循环骨架（`incremental` 的塔族）：循环携带量 [`TowerLoop`]、跨级累积判据
//! [`TowerCarry`]、逐级上下文 [`LevelCtx`]，以及三层循环体
//! （[`process_level`] → [`scan_and_extend_level`] / [`assemble_level_state`]）。

use super::*;

/// 逐级增量构造塔的**跨级累积量**（逐级 OR/min 传播，§2.3 逐级恒定）。
struct TowerCarry {
    /// 任一级检出 frontier 变异 ⟹ 该级及所有更高级无条件跟随（一旦置 true 不复位）。
    ///
    /// ★cascade reset（codex 异质审查裁决：option 2）根因：本级守卫只比对 `project_to_units`
    /// 投影（有损——丢弃 `sub_moves`/`rmove.subs`）。当 L0 末段内点改写使上级投影 bit-identical
    /// 但底层 sub_moves 变时，上级 `frontier_mutated=false` 会漏 reset ⟹ `upper_moves` 深嵌套
    /// sub_moves 陈旧 + BSP memo 复用陈旧（codex 反例 `cascade_reset_on_frontier_interior_rewrite`
    /// 坐实）。cascade 消除整类「投影是否捕获深字段变化」的易错判断（no-patch）：下级变异无条件
    /// 向上传播，上级不依赖投影完备性。代价：变异 bar（16K 中 ~120 次稀疏）该级+所有上级全量重扫，
    /// amortized 仍 O(n)。
    cascade_reset: bool,
    /// ★on2w2-cascade 失效边界定理（设计 §1）：本 bar 累积脏源下界 `e`（最小改变源坐标）。
    /// init `usize::MAX`（=+∞，无改变 ⟹ 无 cascade）。**生产逻辑**：cascade 命中时按
    /// `read_end_src < e` 取保留前缀 P，前缀 bit-identical 保留、后缀失效重扫（撤 #65「整塔前缀清空」）。
    dirty_e: usize,
    /// ★工位 4g：本 bar 是否有任一级 extend 非空 tail（含新级涌现首产 + 最高级 append）——驱动
    /// `generation` +1（与 `cascade_reset` 一起完整覆盖 extract 可观察树变更，codex Q3）。
    did_extend: bool,
    /// ★on2w2 `forest_epoch` dirty 累积器（E1-E3 折叠，循环后统一 bump——避开与循环内 `lc` 可变
    /// 别名，同 `did_extend`/`cascade_reset` 模式）。E1（L0 塔重建）在循环前置位；E2（upper extend）/
    /// E2a（frontier pop）/E3（cascade clear）在循环内 `|=`。E4（`clear()`）不经此，方法内直接 bump。
    forest_dirty: bool,
    /// ★A3 证书（per-level `dirty_from`，§2.4）：本级 units 的不可变前缀长度。L0 = `l0_dirty_from`
    /// （§2.3）；L≥1 = 父级 `prefix_count`（loop 尾更新）。驱动 03（L1+ stable 从 0 抬起）+
    /// 04（`cached_units` truncate+extend O(tail)）。
    dirty_from: usize,
}

/// 逐级循环的**全部携带量**（栈上局部，drop 时点与原码内联时逐点相同，T11/T12）。
///
/// 拆成独立字段（而非塞进一个大 tuple）是为了让 [`process_level`] 内的
/// `&st.units` / `&mut st.carry` 同时成立——两者路径不相交，borrowck 逐字段判定。
struct TowerLoop {
    levels: Vec<LevelState>,
    tower_snapshots: Vec<Rc<Vec<LeveledMove>>>,
    units: Rc<Vec<UnitRange>>,
    /// Q7-#1 裁定C：units 方向锚资格（级别-N 在投影点与 units 同步派生；L0 分支不消费）。
    units_anchors: Vec<Option<Direction>>,
    moves_tower: Rc<Vec<LeveledMove>>,
    carry: TowerCarry,
    forest_dirty_l0: bool,
}

/// 从 [`L0Prelude`] 起手：L0 级 units/塔直接 move 入循环携带量（★H4：无 `Rc::new` 双重包裹）。
fn new_tower_loop(prelude: L0Prelude) -> TowerLoop {
    let L0Prelude { l0_units, moves_tower_l0, l0_dirty_from, forest_dirty_l0 } = prelude;
    TowerLoop {
        levels: Vec::new(),
        tower_snapshots: Vec::new(),
        units: l0_units,
        units_anchors: Vec::new(),
        moves_tower: moves_tower_l0,
        carry: TowerCarry {
            cascade_reset: false,
            dirty_e: usize::MAX,
            did_extend: false,
            forest_dirty: forest_dirty_l0,
            dirty_from: l0_dirty_from,
        },
        forest_dirty_l0,
    }
}

/// 循环收束：把携带量收成 [`TowerBuild`]（丢弃只在循环内有意义的 units/moves_tower/dirty_*）。
fn tower_loop_into_build(st: TowerLoop) -> TowerBuild {
    TowerBuild {
        levels: st.levels,
        tower_snapshots: st.tower_snapshots,
        cascade_reset: st.carry.cascade_reset,
        did_extend: st.carry.did_extend,
        forest_dirty: st.carry.forest_dirty,
        forest_dirty_l0: st.forest_dirty_l0,
    }
}

/// 自然终止（`units.len() < min_parts`）时截断不可达的更高级缓存槽。
///
/// ★§2.6：`level_idx` 及所有更高级本 bar 全跳过（循环顶部 break，`lc` 未触碰）——truncate 掉
/// 这段不可达尾巴的陈旧 [`LevelCache`]，令其恢复时走 `LevelCache::default()` 全扫（血缘自洽，
/// 防 `stable = dirty_from.min(scanned)` 坍缩假阴）。
fn truncate_levels_on_min_parts_break(levels_cache: &mut Vec<LevelCache>, level_idx: usize) {
    #[cfg(test)]
    oracle_probe::on_minparts_break(level_idx < levels_cache.len());
    levels_cache.truncate(level_idx);
}

/// 本级投影出空 units 时截断不可达的更高级缓存槽（★§2.6：`level_idx` 已处理完，尾部 break）。
fn truncate_levels_on_empty_break(levels_cache: &mut Vec<LevelCache>, level_idx: usize) {
    #[cfg(test)]
    oracle_probe::on_empty_break(level_idx + 1 < levels_cache.len());
    levels_cache.truncate(level_idx + 1);
}

/// 本级不变的上下文（逐级构造，全是借用；把 5 个逐级只读量从参数表折进一个载体）。
struct LevelCtx<'a, 'b> {
    l0: &'a ParseLayer,
    config: &'a ThetaConfig,
    series: &'a LevelSeries<'b>,
    level_idx: usize,
    is_l0: bool,
}

/// 逐级增量构造塔（`for level_idx in 0..=l_max` 主循环骨架）。
///
/// 按字段接参而非 `&mut TowerCache`：`series` 持 `&cache.macd_{hist,dif}`，本函数改 `cache.levels`
/// ——两个借用路径不相交（报告 §1.2）。返回类型不含借用 ⟹ 调用结束即释放，主函数随后可写 cache。
///
/// 循环携带量（`units`/`moves_tower`/`units_anchors`/[`TowerCarry`]）全部是本函数的栈上局部量，
/// drop 时点与原码内联时逐点相同（T11/T12）。
///
/// ★H7 Rc 化：loop 内 `units` 只读（读 len/切片/传 `&[]`，从不原地改），下一级由 stage 10
/// `Rc::clone(&lc.projected_units)` O(1) 重赋，替代全量 clone。
/// ★H4：L0 首级 `units` = `l0_units`（本身已是 `Rc<Vec<UnitRange>>`，00b 阶段 `Rc::clone` 出借
/// 缓存），直接 move 入 `units`（无 `Rc::new` 双重包裹）。
/// ★O(n) 重构：`tower_snapshots` 存 `Rc`——L≥1 级 push `Rc::clone(&lc.upper_moves)`（O(1)）；
/// L0 级 push `moves_tower_l0`（Rc）。下游（runner/interp/l3）只读借 `&[Rc<Vec<LeveledMove>>]`。
/// loop 尾 `moves_tower = Rc::clone(&levels_cache[level_idx].upper_moves)` 是 O(1) 引用计数，
/// 消除 per-bar 全塔深拷贝（旧 `lc.upper_moves.clone()` 是 O(n²) 热点①根因）。
pub(super) fn build_level_tower(
    l0: &ParseLayer,
    config: &ThetaConfig,
    levels_cache: &mut Vec<LevelCache>,
    prelude: L0Prelude,
    series: &LevelSeries<'_>,
) -> TowerBuild {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;
    let mut st = new_tower_loop(prelude);

    for level_idx in 0..=l_max {
        // 自然终止：单元数 < min_parts ⟹ 停止（与 classify_impl 同口径）。
        if st.units.len() < min_parts {
            truncate_levels_on_min_parts_break(levels_cache, level_idx);
            break;
        }
        // 缓存槽按需扩展（首次到达该级 ⟹ 新建空 LevelCache，start_i=0 全量扫）。
        if levels_cache.len() <= level_idx {
            levels_cache.push(LevelCache::default());
        }
        let ctx = LevelCtx { l0, config, series, level_idx, is_l0: level_idx == 0 };
        process_level(&mut levels_cache[level_idx], &ctx, &mut st);

        if st.units.is_empty() {
            truncate_levels_on_empty_break(levels_cache, level_idx);
            break;
        }
        st.moves_tower = Rc::clone(&levels_cache[level_idx].upper_moves);
    }
    tower_loop_into_build(st)
}

/// 单级的完整增量步：输入校验/失效 → 扫描续接 → 塔快照 → 装配 `LevelState` → 派生下一级输入。
///
/// 就地推进 [`TowerLoop`] 的携带量（`units`/`units_anchors`/`carry.dirty_from` 换成下一级的值，
/// `levels`/`tower_snapshots` 各追加一项）。
///
/// 四步的相对次序是硬约束：
/// - `tower_snapshots.push(mem::take(moves_tower))` 必须在 [`scan_and_extend_level`] **之后**
///   （T11：该函数内的 `advance_cp_lifecycles` 是本级对 `moves_tower` 的最后一次读）、
///   在 [`assemble_level_state`] **之前**（快照语义是「compose 前的本级输入塔」）；
/// - `levels.push` 必须在读 `levels.last().moves` **之前**（T13：`moves` 已 move 进 `LevelState`）。
fn process_level(lc: &mut LevelCache, ctx: &LevelCtx<'_, '_>, st: &mut TowerLoop) {
    // #106/A3 §3.3 证书跳前缀：L0 用 parser `segments_confirmed_len`，L1+ 用父级 `dirty_from`。
    let stable_bound = if ctx.is_l0 { ctx.l0.segments_confirmed_len } else { st.carry.dirty_from };
    apply_frontier_invalidation(
        lc, &st.units, stable_bound,
        &mut st.carry.cascade_reset, &mut st.carry.dirty_e, &mut st.carry.forest_dirty,
    );
    lc.last_input_len = st.units.len();
    sync_cached_units(lc, &st.units, st.carry.dirty_from);

    let prefix_count = scan_and_extend_level(
        lc, ctx, &st.units, &st.moves_tower[..], &st.units_anchors, &mut st.carry,
    );

    // 本级输入塔快照（compose 前）。★O(1) 优化：`moves_tower` 是 Rc——move 入 snapshots
    // （所有权转移，零拷贝）。下一级由 loop 尾 `Rc::clone(&lc.upper_moves)` 重置，故此处 move 合法。
    // bit-exact：snapshots 内容 == 全量版（Rc 指向的 Vec 值不变，仅所有权/引用计数变）。
    st.tower_snapshots.push(std::mem::take(&mut st.moves_tower));

    let level = assemble_level_state(
        lc, ctx, &st.units, &st.units_anchors, prefix_count, st.carry.dirty_e, st.levels.len(),
    );
    st.levels.push(level);

    // 下一级输入 = 上级走势塔投影 + 同源方向锚（`levels` 尾 = 本级刚 push 的 LevelState）。
    let parent_blocks = &st.levels.last().expect("本级 LevelState 已 push").moves;
    st.units = project_next_level_units(lc, parent_blocks, prefix_count);
    st.units_anchors = derive_units_anchors(parent_blocks, st.units.len());
    // ★A3 §2.4：本级 prefix_count 是下一级 units 的不可变前缀（父 confirmed 前缀投影稳定）。
    st.carry.dirty_from = prefix_count;
}

/// 本级扫描续接：frontier pop → `compose_level_resume` → 生命周期继承 → tail extend →
/// cursor/水线推进。返回 `prefix_count`。
///
/// ★codex Q4：`prefix_count = lc.upper_moves.len()`（pop 后的已产出前缀数），tail ordinal 接续
/// 前缀 ⟹ 全量/增量产同 `ElementId`（跨 bar 稳定身份）。★A3 §2.2：`prefix_count` 同时是本级
/// `projected_units` 的不可变前缀（§2.5 truncate 锚）+ 下一级 units 的 `dirty_from`（§2.4）。
///
/// ★task #143：走势分解增量无需 frontier 失效钩子——`decompose_resume` 只冻结 sealed 关系
/// （i < m-2，两端中枢均有后继），触 frontier 中枢的临时尾关系每 bar 重折。frontier pop 后重扫
/// 改值 / 一次重扫多产两种破口均被冻结不变量覆盖（decompose.rs 模块头 + 随机事件流 parity 测试）。
/// 追加到已缓存前缀（前缀不可变，仅尾部追加）⟹ 累积 centers/upper == 全量扫描结果。
///
/// ★on2w2 E2+E2a（合并逐值判据）：本级 `upper_moves` 尾部从 `popped.upper` 换成 `tail_upper`。
/// 变更 ⟺ 两者不逐值相等（含长度）。frontier resume 每 bar pop+重扫复现相同窗口
/// （`tail_upper == popped.upper`）时**不置 dirty**——这是把 bump 率从 `did_extend` 的 ~96% 压回
/// forest 真变率 0.78% 的机制（实证订正：初版 `|= !tail_upper.is_empty()` 对每 bar 重扫复现的
/// 相同窗口误 bump）。over-invalidate 保留：长度或任一值不等即 dirty。O(tail) 比对，非全塔。
/// **次序**：该比较必须在 [`extend_level_tail`] 消费 `tail_upper` 之前。
fn scan_and_extend_level(
    lc: &mut LevelCache,
    ctx: &LevelCtx<'_, '_>,
    units: &[UnitRange],
    moves_tower: &[LeveledMove],
    units_anchors: &[Option<Direction>],
    carry: &mut TowerCarry,
) -> usize {
    let popped = pop_frontier_window(lc);
    let prefix_count = lc.upper_moves.len();
    debug_assert_second_cache_contract(lc, prefix_count);
    let (tail_centers, tail_upper, mut tail_cp, tail_metas, new_cursor) =
        stage_profile::time("05_compose_resume", || {
            compose_level_resume(
                units,
                moves_tower,
                ctx.is_l0,
                ctx.level_idx as u32 + 1,
                popped.resume_start,
                prefix_count,
            )
        });
    let lifecycle_scan_from =
        reinherit_cp_lifecycles(lc, &mut tail_cp, &popped.cp, carry.dirty_from, ctx.level_idx);

    // ★A3 oracle 探针：had_window pop 后 T = tail_upper.len()（本 bar 本级重扫产出窗口数）。
    // T==1 = did_extend 证伪正向锁（重扫仅复现被 pop 窗口，tail_upper 恰 1）；T>1 = frontier 值改写。
    #[cfg(test)]
    if popped.had_window {
        oracle_probe::on_pop_rescan(tail_upper.len());
    }

    carry.did_extend |= !tail_upper.is_empty();
    carry.forest_dirty |= tail_upper != popped.upper;
    extend_level_tail(lc, tail_centers, tail_upper, tail_cp, tail_metas);
    let cp_objects = Rc::make_mut(&mut lc.cp_ownership);
    recursive_tower::advance_cp_lifecycles(
        cp_objects.as_mut_slice(),
        &lc.centers,
        units,
        moves_tower,
        (!ctx.is_l0).then_some(units_anchors),
        lifecycle_scan_from,
    );
    lc.scan_cursor = new_cursor;
    advance_confirmed_watermark(lc, carry.cascade_reset);
    debug_assert_level_alignment(lc);
    prefix_count
}

/// 装配本级 [`LevelState`]：走势分解 → 冻结边界登记 → BSP 提取 → centers/投影层投影。
///
/// 走势分解（增量续折：resume 单一来源 ⟹ 与全量 `decompose` 定义性 bit-exact）。
/// ★#148：链尾可变中枢数 = 本轮末窗口产出数（升级重切窗口的全部子中枢在窗口 sealed 前均可变
/// ——外缘随延伸改写、数量随段数增长改变），冻结边界随之后移（decompose.rs 文档）。
///
/// #69 5b：**无条件**登记本级一/三类所用 source 水位；不得挂在 BSP memo miss 分支，否则 hit bar
/// 会暴露陈旧 `e_src`。公式与 signal resume 单一同源。
fn assemble_level_state(
    lc: &mut LevelCache,
    ctx: &LevelCtx<'_, '_>,
    units: &[UnitRange],
    units_anchors: &[Option<Direction>],
    prefix_count: usize,
    dirty_e: usize,
    level_ordinal: usize,
) -> LevelState {
    let moves = decompose_resume(
        &lc.centers,
        &mut lc.decompose_state,
        lc.scan_cursor.last_window_emitted.max(1),
    );
    lc.last_freeze_boundary = signal::freeze_boundary_src(&lc.centers, prefix_count, dirty_e);

    let bsp_inputs = LevelBspInputs {
        l0: ctx.l0,
        config: ctx.config,
        units,
        units_anchors,
        is_l0: ctx.is_l0,
    };
    let (bsp, pan_div) =
        extract_level_bsp(lc, &bsp_inputs, &moves, prefix_count, dirty_e, ctx.series);

    // 08：`Rc::clone`（O(1)）投影增量塔 centers 到 LevelState，替代全量 `centers.clone()`。
    let centers = stage_profile::time("08_levels_centers_clone", || Rc::clone(&lc.centers));
    let level_projection = build_level_projection(ctx.config, level_ordinal as u32, &bsp, ctx.l0);
    LevelState {
        moves,
        centers,
        cp_ownership: Rc::clone(&lc.cp_ownership),
        bsp,
        pan_div,
        level_projection,
    }
}
