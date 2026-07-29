//! 增量塔逐级 frontier 变异检测与 cascade 失效（`incremental` 的失效族）。
//!
//! 设计锚：on2w2-cascade 失效边界定理（§1.2/§1.3/§3）。全部函数只碰单个 [`LevelCache`]，
//! 不接触跨级状态——跨级累积量由调用方 [`super::tower`] 以 `&mut` 出参传入。

use super::*;

/// 放行条件3 falsification 探针启用开关（env `THETA_CASCADE_EPROBE`，仅 test 构建；
/// 测「保留前缀比例 P/len」判设计前提）。
///
/// `OnceLock::get_or_init` 幂等 ⟹ 每次调用读同一值，与原「循环外初始化一次的局部 `eprobe_on`」
/// 逐值等价（登记变换：局部量 → 同 static 的读函数，值域不变）。
#[cfg(test)]
fn cascade_eprobe_on() -> bool {
    *CASCADE_EPROBE.get_or_init(|| std::env::var("THETA_CASCADE_EPROBE").is_ok())
}

/// 本级前缀不变量校验 + cascade 失效（anc.pdf §16：confirmed prefix immutable / frontier mutable）。
///
/// 两类违反 resume 充要条件 #2（`units[..consumed]` 不可变）⟹ 该级缓存重置（bit-exact 退化）：
///   1. 长度回缩：`units.len() < last_input_len`（段账本前缩）。
///   2. frontier 末段原地改写：长度不变/增长但**扫描区**（`units[..consumed+2]`，已 build 读过
///      的单元）内某单元值变了——parser 古怪线段重划改写末段（定义层正确，非 bug）。仅长度
///      守卫漏此例（bar 1464 seg[9] end 1384→1170，段数不变）。扫描区外（未读尾部）的变化
///      无害（resume 从 consumed 续扫会读到新值），不触发重置。
///
/// `stable_bound`（#106 证书跳前缀，codex 裁决）：L0 units = segments 投影，
/// `segments[..confirmed_len]` 跨 bar bit-stable ⟹ `units[..stable]` 必等，只比 `[stable..scanned]`。
/// L1+ units 来自上级投影（无 segments 证书）⟹ A3 §3.3 从保守 0 抬升到父级 `prefix_count`
/// （`dirty_from`），前缀跳过不比较（可证不可变）——收益全在 L1+。证书**不**证明 `[stable..scanned]`
/// 没变（bar-1464 末段改写在 scanned frontier 内）⟹ 该区间仍逐值比，不漏 cascade。
///
/// 三个 `&mut` 出参都是**跨级累积量**（逐级 OR/min，§2.3 逐级恒定传播）：
/// - `cascade_reset`：任一级检出变异 ⟹ 该级及所有更高级无条件跟随（投影有损，上级不能仅靠
///   本级投影判断）。一旦置 true 不再复位。
/// - `dirty_e`：累积脏源下界（最小改变源坐标），驱动增量失效边界。
/// - `forest_dirty`：on2w2 E3——本级 `upper_moves` 尾段失效（tower[level+1] 字节变更）⟹ forest 变。
pub(super) fn apply_frontier_invalidation(
    lc: &mut LevelCache,
    units: &[UnitRange],
    stable_bound: usize,
    cascade_reset: &mut bool,
    dirty_e: &mut usize,
    forest_dirty: &mut bool,
) {
    let scanned = (lc.scan_cursor.consumed + 2).min(units.len()).min(lc.cached_units.len());
    let stable = stable_bound.min(scanned);
    let frontier_mutated = stage_profile::time("03_frontier_compare", || {
        units[stable..scanned] != lc.cached_units[stable..scanned]
    });
    let len_shrunk = units.len() < lc.last_input_len;
    if len_shrunk || frontier_mutated {
        *cascade_reset = true;
        *dirty_e = (*dirty_e).min(local_dirty_source(lc, units, stable, scanned, len_shrunk));
    }
    if *cascade_reset {
        *forest_dirty = true;
        invalidate_level_cache(lc, *dirty_e);
    }
}

/// ★on2w2-cascade 失效边界定理（设计 §1.3）：本级局部脏源坐标 `local_e`。
///
/// 区间 `[stable..scanned]` 首个改变下标 `j_min` ⟹ `units[j_min].start_index`
/// （取 min 保证 `units[..j_min]` 未变，§3.2）。len-shrink ⟹ 0 全清（设计 §5：回缩罕见，不做增量）。
fn local_dirty_source(
    lc: &LevelCache,
    units: &[UnitRange],
    stable: usize,
    scanned: usize,
    len_shrunk: bool,
) -> usize {
    if len_shrunk {
        return 0; // (A) len-shrink：退化全清（设计 §5）。
    }
    // (B) frontier_mutated：区间内首个改变下标 j_min ⟹ units[j_min].start_index。
    match (stable..scanned).find(|&j| units[j] != lc.cached_units[j]) {
        Some(j) => units[j].start_index,
        None => usize::MAX, // 理论不达（frontier_mutated 蕴含存在改变），保守不降 e。
    }
}

/// ★增量失效边界（设计 §1.2/§3.4）：保留 `read_end_src < e` 的前缀 P（读域上界含停止哨兵，
/// 覆盖 codex §1.2.1 反例）。`win_meta` 与 `centers` 1:1 对齐、`read_end_src` 升序（源单调 S1）⟹
/// `partition_point` 定位 P。升级子中枢共享父窗口 `read_end_src` ⟹ 整窗保留或整窗失效（§3.5）。
///
/// `THETA_CASCADE_FULLCLEAR`（test-only，铁律 H1 神谕先例）：强制 P=0 退回整塔前缀清空，
/// A/B 计时 + bit-exact 对照面。默认 off ⟹ 增量路径。开启后 `bit_exact_per_bar` 仍须绿
/// （证 P>0 与全清逐字段相等）。
fn invalidate_level_cache(lc: &mut LevelCache, dirty_e: usize) {
    // `mut`：仅 `THETA_CASCADE_FULLCLEAR`（cfg(test)）会把 P 强置 0；release 构建下该分支剥离，
    // 保留既有的 unused_mut 警告（与基线警告集逐字一致，不在本票的零行为变化范围内清理）。
    let mut p = lc.win_meta.partition_point(|w| w.read_end_src < dirty_e);
    // ★放行条件3 keep_frac 探针（test+eprobe，release 剥离）：真实 P/len（不再用 end_index 代理）。
    #[cfg(test)]
    if cascade_eprobe_on() && !lc.centers.is_empty() {
        oracle_probe::on_cascade_event(p as f64 / lc.centers.len() as f64);
    }
    #[cfg(test)]
    if *CASCADE_FULLCLEAR.get_or_init(|| std::env::var("THETA_CASCADE_FULLCLEAR").is_ok()) {
        p = 0;
    }
    if p == 0 {
        clear_level_cache(lc);
    } else {
        retain_level_prefix(lc, p, dirty_e);
    }
}

/// P=0（无可保留前缀，含 e=0 全清 / e 坍缩到起点）：退化为原全清（bit-exact，与 #65 整塔前缀清空同）。
fn clear_level_cache(lc: &mut LevelCache) {
    lc.scan_cursor = WindowScanCursor::default();
    // make_mut：若上 bar snapshot 仍持引用则写时复制再 clear（退化 bit-exact）；否则原地清。
    Rc::make_mut(&mut lc.upper_moves).clear();
    Rc::make_mut(&mut lc.centers).clear();
    Rc::make_mut(&mut lc.cp_ownership).clear();
    lc.win_meta.clear();
    lc.decompose_state.reset();
    Rc::make_mut(&mut lc.cached_bsp).clear();
    Rc::make_mut(&mut lc.cached_pan_div).clear(); // Q4：与 cached_bsp 同批失效（同 key 守卫）。
    lc.cached_bsp_key = None;
    lc.cached_second.clear(); // 07b 门控：前缀重排 ⟹ 前缀 B2 缓存失效，重扫。
    lc.cached_second_count = 0;
    lc.confirmed_watermark = 0; // ★#93 全清 ⟹ 水线归零（消费方全量重比）。
    Rc::make_mut(&mut lc.projected_units).clear(); // #106：投影缓存失效，重投影。
}

/// P>0（增量失效，设计 §3）：保留 `[..P]` 前缀（读域<e，L1 bit-identical），失效 `[P..]` 后缀。
///
/// **内部次序硬约束**：`b2_cut_incl` 读 `upper_moves[b2_count-1].end_index`，必须在
/// `upper_moves.truncate(p)` **之前**取（原码同序；虽然 `b2_count <= p` 使 truncate 后取值相同，
/// 但保持原序才免于依赖该不等式的传递证明）。
fn retain_level_prefix(lc: &mut LevelCache, p: usize, dirty_e: usize) {
    let wm = rebuild_cursor_for_retained_prefix(lc, p, dirty_e);
    let (b2_count, b2_cut_incl) = second_cache_anchors(lc, p, wm.emitted);

    Rc::make_mut(&mut lc.centers).truncate(p);
    Rc::make_mut(&mut lc.upper_moves).truncate(p);
    Rc::make_mut(&mut lc.cp_ownership).truncate(p);
    lc.win_meta.truncate(p);
    // ★#93 水线收缩到保留前缀 P（[P..] 本 bar 重扫可能改写）；bar 末再 min(w_nat)。
    lc.confirmed_watermark = lc.confirmed_watermark.min(p);
    // decompose：保留 reset()（O(centers)=百级，非 05/09/07b 的 O(n²) 靶，设计 §3.2 scope）。
    // 输出恒等全折叠（decompose 模块头），reset+重折 bit-exact，仅不省非瓶颈的重折量。
    lc.decompose_state.reset();
    // cached_bsp memo：key=(centers.len,..) 变 ⟹ 必 miss ⟹ 部分保留零收益（设计 §3.2），维持全失效。
    Rc::make_mut(&mut lc.cached_bsp).clear();
    Rc::make_mut(&mut lc.cached_pan_div).clear();
    lc.cached_bsp_key = None;
    truncate_second_cache(lc, b2_count, b2_cut_incl);
    // 投影缓存：截到 P（前缀投影稳定，L2 §2.3）——stage 09 的 truncate(prefix_count) 会进一步
    // 截到 pop 后的 prefix_count（pop_prefix），从此续投影（O(tail)）。此处截 p 是保守上界。
    Rc::make_mut(&mut lc.projected_units).truncate(p);
}

/// cursor 重建为「把 P-1 窗口当 frontier 待 pop」——回归常态 frontier pop 语义（§3.4）：
/// `resume_from = win_meta[P-1].win_start`，从该起点重扫复现被 pop 窗口（哨兵翻转则重扫吸收，
/// 自动覆盖 §1.2.1）；从 `win_start` 重扫至 len 覆盖 None-支失败 seed 尾部（§2.2.1）。
/// 随后 [`pop_frontier_window`]（现有 bit-exact pop 机制）据此 cursor pop P-1 窗口整窗。
///
/// 返回保留末窗的 [`WinMeta`]（`emitted` 供 07b 分离锚推导）。
fn rebuild_cursor_for_retained_prefix(lc: &mut LevelCache, p: usize, dirty_e: usize) -> WinMeta {
    let wm = lc.win_meta[p - 1];
    lc.scan_cursor = WindowScanCursor {
        consumed: wm.win_exit,
        resume_from: wm.win_start,
        last_window_emitted: wm.emitted,
    };
    debug_assert!(
        wm.read_end_src < dirty_e,
        "O2 哨兵读域断言：保留末窗 P-1 读域上界 {} 必 < e={} （否则读了 dirty 单元，须左退）",
        wm.read_end_src, dirty_e
    );
    wm
}

/// 07b 门控（设计 §3.1 + 放行条件4 契约）：cascade 后 cursor 把 P-1 窗口当 frontier 待
/// pop（§3.4）⟹ [`pop_frontier_window`] 把 `prefix_count` pop 到 `p - emitted`。
///
/// ★覆盖锚（bar-10299 修复）：`cached_second` **只含由前 old_count 个 parent 产出的 B2**——不能
/// 声明超过 old_count 的覆盖（否则 `extract_second_resume` 跳过 `[old_count..)` 的重算 ⟹ 漏 B2）；
/// 也不能保留被失效前缀 `[p..old_count)` 的 B2。故新覆盖锚 = `min(old_count, pop_prefix)`：
/// 既 <= pop 后 confirmed 前缀（放行条件4 单调），又 <= 已实际缓存的 parent 数。
///
/// ★分离锚（bar-27947 修复）：`cached_second` 按 **parent 追加序**（非全局 source 排序——sort
/// 在合并端，非缓存内）。B2 `source_index` = parent 内某 sub_move 的 `end_index`，落 parent 源区间；
/// parent 源区间不重叠但**共享边界坐标**（`unit.end == 下一 unit.start`）。故分离锚须用「前一保留
/// parent 的 `end_index`，含界」：`parent[b2_count-1].end_index`。`parent[b2_count]` 的首个 B2
/// `source_index > 其 start_index >= 该 end_index` ⟹ `<=` 精确分离，不误丢边界 B2（用
/// `parent[b2_count].start_index` 的 `<` 会丢掉恰落共享边界的前 parent B2）。
fn second_cache_anchors(lc: &LevelCache, p: usize, emitted: usize) -> (usize, Option<usize>) {
    let pop_prefix = p - emitted;
    let b2_count = lc.cached_second_count.min(pop_prefix);
    // 含界上锚：前 b2_count 个 parent 中最后一个的 end_index（b2_count==0 ⟹ 无保留 ⟹ cut 前于全部）。
    let b2_cut_incl = if b2_count == 0 {
        None
    } else {
        Some(lc.upper_moves[b2_count - 1].end_index)
    };
    (b2_count, b2_cut_incl)
}

/// 按 [`second_cache_anchors`] 的两个锚裁剪 07b 第二类缓存（含界 `<=` 分离，见该函数文档）。
fn truncate_second_cache(lc: &mut LevelCache, b2_count: usize, b2_cut_incl: Option<usize>) {
    let b2_keep = match b2_cut_incl {
        None => 0,
        Some(cut) => lc.cached_second.partition_point(|b| b.source_index <= cut),
    };
    lc.cached_second.truncate(b2_keep);
    lc.cached_second_count = b2_count;
}
