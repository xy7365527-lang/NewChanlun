//! 增量塔入口 [`classify_with_tower_incremental`]（task #93）：塔构造的中枢扫描走增量
//! resume（前级 confirmed 前缀缓存，仅尾部续扫），解 per-bar substrate 的塔构造 O(n²) 根因。
//!
//! bit-exact 保证 / 硬契约 / 增量有效性 / 边界见函数文档；缓存本体见 [`super::tower_cache`]。

use super::pipeline::segment_to_unit;
use super::sublevel::extract_second_resume;
use super::tower_cache::{compute_macd_hist_incremental, update_closes_cache, AreaCache, LevelCache};
use super::*;

/// on2w2-cascade 放行条件3 探针启用开关（env THETA_CASCADE_EPROBE，读一次缓存——热路径零 syscall）。
/// 仅 test 构建（探针本身 #[cfg(test)]），release 完全不编译。
#[cfg(test)]
static CASCADE_EPROBE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// on2w2-cascade 全清对照开关（env THETA_CASCADE_FULLCLEAR）：强制 P=0（退回 #65 整塔前缀清空），
/// 作 A/B 计时基线 + bit-exact 全量失效对照面（铁律 H1 神谕先例）。仅 test 构建，release 不编译
/// （默认 off = 增量失效路径）。开启后增量与全清应逐字段相等（bit_exact_per_bar 仍绿即证 P>0 sound）。
#[cfg(test)]
static CASCADE_FULLCLEAR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// [`prepare_l0_inputs`] 的产出：本 bar L0 侧的两个塔构造输入 + 两个证书/判据。
///
/// 各字段的生成次序受 T1–T5 次序点约束，见 `chanlun/review-results/issue633-incremental-fn-decomposition-20260729.md` §1.1。
struct L0Prelude {
    /// L0 输入单元（`l0_units_cache` 的 `Rc::clone`，逐级循环的首级 `units`）。
    l0_units: Rc<Vec<UnitRange>>,
    /// L0 走势塔（`moves_tower_l0` 的 `Rc::clone`，逐级循环的首级 `moves_tower`）。
    moves_tower_l0: Rc<Vec<LeveledMove>>,
    /// L0 units 复用前缀长度 = `dirty_from[0]`（A3 证书 §2.3）。
    l0_dirty_from: usize,
    /// on2w2 E1：L0 塔（tower[0]）本 bar 字节变更判据（循环 `forest_dirty` 的初值）。
    forest_dirty_l0: bool,
}

/// 前缀不变量校验：段账本回缩 ⟹ 清空重扫（bit-exact 退化，非增量）。
///
/// ★#613（#609 F2 根因收口）：本检测**必须**在 `l0_units_cache` 构建之前（T1）。它曾位于构建之后
/// （旧序：00_l0_units_build → Rc::clone 出借 → 回缩 clear），于是回缩 bar 上 [`TowerCache::clear`]
/// 把刚建好的 `l0_units_cache` 清空（该方法逐字段清塔缓存），而本 bar 的塔构造消费的是 clear
/// **之前** `Rc::clone` 出的局部 `l0_units`（写时复制 ⟹ 值完整）⟹ 函数返回后
/// [`TowerCache::l0_units`] 为空而 `tower[0]` 满载，只读契约「与 tower[0] 同序同长同源」被破。
/// BTC 100k 实测：回缩事件 1 次（`segments` 549→547 @ as_of=71040），失步 1 次，一一对应
/// （L2 等级：BTC 单标的单窗）。提前后 `l0_units_cache` 已清 ⟹ reuse=0 ⟹ 本 bar 全量重建，
/// 与同样在 clear 之后全量重建的 `moves_tower_l0` 口径归一，不变式成立
/// （见 [`classify_with_tower_incremental`] 末 `debug_assert`）。
///
/// 值不变（bit-exact）：复用前缀受 `segments_confirmed_len` 证书约束（parser 保证
/// `segments[..confirmed_len]` 跨 bar 逐字节稳定），回缩只发生在未确认尾部 ⟹ 全量重建 ==
/// 复用重建。仓内 `DIAG_L0UNITS` 对拍探针在 BTC 100k 全程零告警（含该回缩 bar），L2 等级实证。
/// 代价：回缩 bar 的 `l0_dirty_from` 从复用长度降为 0（保守全脏，A3 证书前缀护栏恒成立），
/// 100k 内 1 次，标度无影响。
fn reset_cache_on_segment_shrink(l0: &ParseLayer, cache: &mut TowerCache) {
    if l0.segments.len() < cache.last_l0_segments_len {
        cache.clear();
    }
}

/// L0 输入单元 = parser 线段账本。#106 证书增量（同 `moves_tower_l0`，消除每 bar 全量 collect）。
///
/// 返回 `reuse` = L0 units 复用前缀长度 = `dirty_from[0]`（§2.3：L0 不可变前缀，`units[..reuse]`
/// 逐字段等上 bar，`units[reuse..]` 本 bar 新 extend）。
///
/// `Rc::make_mut`：上 bar 的 `l0_units` Rc 已在循环内被投影重赋值 drop ⟹ `strong_count==1` ⟹ 原地
/// truncate+extend O(tail)；>1（caller 跨 bar 持有）⟹ 写时复制（bit-exact，同 `moves_tower_l0`）。
/// **本函数必须先于出借的 `Rc::clone`**（T2）——反序则 `strong_count==2`，每 bar 退化为写时复制。
fn build_l0_units_cache(l0: &ParseLayer, cache: &mut TowerCache) -> usize {
    stage_profile::time("00_l0_units_build", || {
        let reuse = l0.segments_confirmed_len.min(cache.l0_units_cache.len());
        let c = Rc::make_mut(&mut cache.l0_units_cache);
        c.truncate(reuse);
        c.extend(l0.segments[reuse..].iter().map(segment_to_unit));
        reuse
    })
}

/// DIAG(frontier-bit-exact)：对拍复用版 `l0_units_cache` vs 全量 `segment_to_unit`
/// （隔离 L0 units 前缀复用是否陈旧）。env `DIAG_L0UNITS` 门控，未开启时零开销直通。
fn diag_l0_units_parity(l0: &ParseLayer, l0_units: &[UnitRange], cache: &TowerCache) {
    if std::env::var("DIAG_L0UNITS").is_err() {
        return;
    }
    let full: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
    if l0_units != full.as_slice() {
        let m = l0_units.len().min(full.len());
        let first = (0..m).find(|&i| l0_units[i] != full[i]);
        eprintln!(
            "[DIAG-L0UNITS] segs={} confirmed_len={} cache.len(reuse前)={} ★l0_units陈旧 first_diff={:?} \
             lens=({},{})",
            l0.segments.len(), l0.segments_confirmed_len, cache.l0_units_cache.len(),
            first, l0_units.len(), full.len()
        );
    }
}

/// ★on2w2 E1：L0 塔（tower[0]）字节变更判据。`moves_tower_l0` 是 `l0.segments` 的纯函数
/// （`from_unit∘segment_to_unit`，ordinal=index）⟹ 内容变更 ⟺ segments 变更。parser 证书保
/// `segments[..confirmed_len]` 跨 bar bit-stable ⟹ 只需比 tail `[reuse..]`。
///
/// **必须先于 [`rebuild_l0_tower`]**（T4）：本函数读的是 `cache.moves_tower_l0` 的**旧值**，
/// 重建后再比恒为 false ⟹ E1 判据失效、`forest_epoch` 漏 bump ⟹ 下游 `TreeCache` 假命中陈旧森林。
///
/// ★实证订正（on2w2 实装，CL 8K）：初版设计用「reuse<len ∨ segments[reuse..]非空」长度判据，
/// 但 frontier resume 使 truncate+repush 每 bar 发生（segments 有未确认尾段 ⟹ reuse<len 恒真），
/// 而 repush 的尾段字节 99.2% 与旧值相同（inclusion-only 不改段）⟹ 长度判据 bump 率 100%，
/// O(n²) 未消除。故改为**逐值比对 tail**（O(未确认尾段)=O(1) 摊还，非全塔）——精确匹配
/// of_forest 真变率 0.78%。over-invalidate 方向保留：长度不等或任一尾段值不等即 dirty。
fn l0_tower_tail_changed(l0: &ParseLayer, cache: &TowerCache) -> bool {
    let old = &cache.moves_tower_l0;
    let old_len = old.len();
    if old_len != l0.segments.len() {
        return true;
    }
    let reuse = l0.segments_confirmed_len.min(old_len);
    (reuse..old_len).any(|i| {
        let u = segment_to_unit(&l0.segments[i]);
        old[i] != LeveledMove::from_unit(&u, ElementId { level: 0, ordinal: i as u64 })
    })
}

/// L0 走势塔 = 携坐标的 `RMove::Segment`（递归底）。
///
/// ★#106 证书增量（替代每 bar 全量 `from_unit` 重建 = O(segs)×n，profile 坐实 400K=5.8s）：
/// parser `segments_confirmed_len` 保证 `segments[..confirmed_len]` 跨 bar bit-stable（仅末段可古怪
/// 线段重划，codex 确认）⟹ `moves_tower_l0[..reuse]` 复用（`from_unit` 只依赖单 seg，无相邻依赖）。
/// `ordinal=reuse+i` 全局索引（前缀 `reuse<=confirmed_len` 时 ordinal 不变 = 全量 enumerate，bit-exact）。
/// `Rc::make_mut`：caller 逐 bar drop 上轮 `tower_snapshots[0]` ⟹ `strong_count==1` ⟹ 原地 O(tail)；
/// >1（理论 caller 跨 bar 持有）⟹ 写时复制（仍 bit-exact）。`clear()` 已同步清空（退化全量）。
fn rebuild_l0_tower(l0: &ParseLayer, cache: &mut TowerCache) {
    stage_profile::time("01_l0_tower_rebuild", || {
        let reuse = l0.segments_confirmed_len.min(cache.moves_tower_l0.len());
        let m = Rc::make_mut(&mut cache.moves_tower_l0);
        m.truncate(reuse);
        for (off, seg) in l0.segments[reuse..].iter().enumerate() {
            let i = reuse + off;
            let u = segment_to_unit(seg);
            m.push(LeveledMove::from_unit(&u, ElementId { level: 0, ordinal: i as u64 }));
        }
    });
}

/// 本 bar 的 L0 侧序幕：回缩校验 → units 增量重建 → 空判 → 水线落账 → L0 塔增量重建。
///
/// 返回 `None` ⟺ 空 L0（无线段）：缓存已清空，调用方须直接返回
/// `(Classification::default(), Vec::new())`（原边界语义，T5）。
///
/// 内部五步的相对次序全部是硬约束（T1–T5），逐条见各步函数文档。
fn prepare_l0_inputs(l0: &ParseLayer, cache: &mut TowerCache) -> Option<L0Prelude> {
    reset_cache_on_segment_shrink(l0, cache); // T1：必须先于 units 构建
    let l0_dirty_from = build_l0_units_cache(l0, cache);
    // ★[H4] Rc::clone（引用计数 O(1)）替代全量 `.clone()`（O(segments)/bar × n = O(n²)，profile 坐实
    // 400K=380ms）。下游 `units` 只读消费（compose/extract 借 &[UnitRange]），bit-exact：值不变仅所有权。
    let l0_units: Rc<Vec<UnitRange>> =
        stage_profile::time("00b_l0_units_clone", || Rc::clone(&cache.l0_units_cache));
    diag_l0_units_parity(l0, &l0_units, cache);

    // 空 L0：无可构造级别 + 清空缓存（下次从头扫）。
    if l0_units.is_empty() {
        cache.clear();
        return None;
    }

    cache.last_l0_segments_len = l0.segments.len();
    // ★#93：落账本 bar L0 确认前缀（tower[0] 水线，见字段文档）。min 防御 parser 古怪末段计数。
    cache.l0_confirmed_len = l0.segments_confirmed_len.min(l0.segments.len());

    let forest_dirty_l0 = l0_tower_tail_changed(l0, cache); // T4：读旧值，必须先于重建
    rebuild_l0_tower(l0, cache);
    let moves_tower_l0: Rc<Vec<LeveledMove>> = Rc::clone(&cache.moves_tower_l0);

    Some(L0Prelude { l0_units, moves_tower_l0, l0_dirty_from, forest_dirty_l0 })
}

/// ★增量塔入口：返回 `(Classification, tower_snapshots)` bit-exact 等价于
/// `classify_with_tower(l0, config)`，但塔构造的中枢扫描走增量 resume（前级 confirmed 前缀缓存，
/// 仅尾部续扫），解 per-bar substrate 的塔构造 O(n²) 根因。
///
/// ## bit-exact 保证（#93 铁律）
///
/// 输出 `(Classification, Vec<Vec<LeveledMove>>)` 与 `classify_with_tower(l0, config)` 逐字段
/// bit-identical：
/// - `Classification.levels[k].centers`：增量累积的中枢序列 == 全量 `detect_centers`（resume bit-exact，
///   见 recursive_tower.rs 证明）。
/// - `Classification.levels[k].moves`：从累积 centers 经 `decompose_resume` 续折 == 全量分解（resume 单一来源）。
/// - `Classification.levels[k].bsp`：从累积 centers + segments/hist 经同口径提取 == 全量提取。
/// - `tower_snapshots[k]`：本级 compose 前的 `moves_tower`，前缀来自缓存 + 尾部续扫 == 全量 compose。
///
/// ## 硬契约（#106 证书路径，codex 双轮审计锚定）
///
/// `cache: &mut TowerCache` 必须与 `l0` **同源逐 bar 推进**——即同一 `ParseLayerIncr` 血缘、同一
/// `ThetaConfig`、每 bar 调用一次（无跳 bar、无跨数据流复用、config 不变）。`merged_confirmed_len`/
/// `segments_confirmed_len` 证书只保证「本 parser 自己的前缀稳定」，不验证 cache 内旧前缀与本轮 l0 同源。
/// 违反（同 cache 跑两条流不 clear / 中途换 config）⟹ MACD/L0 tower/closes/frontier 复用陈旧前缀
/// （bit-exact 破裂）。生产路径满足：`IncrementalClassifier::new` 每实例新建 TowerCache + 同源逐 bar。
/// 换流/换 config 的调用方须先 `cache.clear()`。
/// ponytail: 不加运行时 lineage epoch——生产 IncrementalClassifier 结构保证同源，epoch 是为不存在的
/// 滥用场景加防御（YAGNI）；契约由本文档 + clear() 入口声明。
/// ## 增量有效性（exp≈1 前提）
///
/// 段账本单调追加（前缀稳定）时，每级扫描从 `consumed` 续扫 O(tail) 而非 O(units) ⟹ 塔构造总扫描
/// O(Σ tail) = O(n)（amortized）。段账本前缀回缩时自动退化为全量（`clear` + 重扫），仍 bit-exact。
///
/// ## 边界
///
/// - 空 ParseLayer（无线段）⟹ `Classification::default()` + 空 tower，cache 清空。
/// - 段账本前缩（`segments.len() < last_l0_segments_len`）⟹ cache 清空 + 全量重扫（bit-exact 退化）。
/// - merged_bars 前缀改写（inclusion 合并回退）⟹ MACD cache 局部重建（bit-exact 退化）。
pub fn classify_with_tower_incremental(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 侧序幕（回缩校验 → units 增量 → 空判 → 水线落账 → L0 塔增量）。内部五步次序受 T1–T5 约束。
    let Some(prelude) = prepare_l0_inputs(l0, cache) else {
        // 空 L0 边界：缓存已在序幕内清空（下次从头扫）。
        return (Classification::default(), Vec::new());
    };
    let L0Prelude { l0_units, moves_tower_l0, l0_dirty_from, forest_dirty_l0 } = prelude;

    // MACD hist（背驰真算，增量递推——231号纯性能，解 aed4d5f5 实证的 c_exp≈2.31 主导根因）。
    //
    // 增量策略（bit-exact 铁律，ac75d4b3 已证 append bit-exact）：
    // - merged_bars.close 前缀与 `macd_closes_prefix` 逐值一致 + 长度 >= 前缀 ⟹ 从
    //   `macd_state` 增量 append 新 close，`current_point().hist` 追加到 `macd_hist`。
    // - merged_bars 回缩 / 前缀改写 / cache 空 ⟹ `macd_state_from_closes` 全量重建
    //   state，并逐 bar append 重建 hist 数组（bit-exact 退化，同塔 cache clear 逻辑）。
    //
    // ★不调 `compute_macd`（全量）——增量路径用 `MacdState::append` O(1)/bar；退化路径
    //   用 `macd_state_from_closes` + 逐 bar `current_point`（与全量同 EMA 约简，bit-exact）。
    // closes/close_src 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写——同 MACD stable_prefix 语义）。
    // 不每 bar 全量重建（O(n)/bar → O(n²) 根因之一，工位 E profile 坐实 t=0.085s@16K）。
    // mem::take 取出缓存 Vec（避免 &cache.closes 与下游 &mut cache 别名），用毕放回（缓冲复用，零额外分配）。
    stage_profile::time("00_update_closes_cache", || {
        update_closes_cache(&l0.merged_bars, l0.merged_confirmed_len, cache)
    });
    let closes: Vec<f64> = std::mem::take(&mut cache.closes);
    let close_src: Vec<usize> = std::mem::take(&mut cache.close_src);
    // ★force_state 生产热路由（beta-route #115）：closes_tick（整数 close）mem::take 出借（同 closes
    // 模式，避免与下游 &mut cache 别名），供一类候选 A/C 段振幅/速度 proxy。用毕放回（下同 closes）。
    let closes_tick: Vec<Tick> = std::mem::take(&mut cache.closes_tick);
    // 增量 MACD：更新 cache.macd_hist + cache.macd_dif（锁步；不返回克隆，直接借用缓存避免 O(n) 拷贝）。
    stage_profile::time("02_macd_incremental", || {
        compute_macd_hist_incremental(&closes, l0.merged_confirmed_len, &config.macd, cache)
    });
    let hist: &[f64] = &cache.macd_hist;
    // dif 借用（与 hist 同——disjoint field 借用；force DIF 峰 proxy 输入，bit-exact 等价全量）。
    let dif: &[f64] = &cache.macd_dif;
    // B3 #4 area-memo：`stable_len` = 本 bar hist 的确认边界（[`AreaCache`] 文档）——`area_cache`
    // 跨 bar 持久（mem::take 出借，用毕放回，同 closes/close_src 模式）。`RefCell` 包裹：
    // `divergence_of` 闭包接口是 `impl Fn(&RMove) -> bool`（`signal::extract_second_signals`），
    // `Fn` 只给闭包体 `&self` 访问——捕获的可变缓存须走内部可变性（共享引用 + `borrow_mut`），
    // 不能捕获 `&mut AreaCache`（Long/Short 两侧各建一个闭包，同一 `&mut` 不能捕获两次）。
    let stable_len = cache.macd_state_len;
    let area_cache: RefCell<AreaCache> = RefCell::new(std::mem::take(&mut cache.area_cache));

    let mut levels: Vec<LevelState> = Vec::new();
    // ★O(n) 重构：snapshots 存 Rc——L≥1 级 push Rc::clone(&lc.upper_moves)（O(1)）；L0 级 push
    // moves_tower_l0（Rc）。下游（runner/interp/l3）只读借 &[Rc<Vec<LeveledMove>>]。
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();

    // 逐级增量构造。`units`/`moves_tower` 是本级输入（前缀来自缓存，尾部新增）。
    // ★H7 Rc 化：loop 内 units 只读（读 len/切片/传 &[]，从不原地改），下一级由 stage 10
    // `Rc::clone(&lc.projected_units)` O(1) 重赋，替代全量 clone。
    // ★H4：L0 首级 units = l0_units（本身已是 Rc<Vec<UnitRange>>，00b 阶段 Rc::clone 出借缓存），
    // 直接 move 入 units（无 Rc::new 双重包裹）。
    let mut units: Rc<Vec<UnitRange>> = l0_units;
    // Q7-#1 裁定C：units 方向锚资格（循环携带，级别-N 在投影点与 units 同步派生；L0 分支不消费）。
    let mut units_anchors: Vec<Option<Direction>> = Vec::new();
    let mut moves_tower: Rc<Vec<LeveledMove>> = moves_tower_l0;

    // ★cascade reset（codex 异质审查裁决：option 2）：任一级检出 frontier 变异 ⟹ 该级**及所有更高级**
    // 无条件 reset。根因：本级守卫只比对 `project_to_units` 投影（有损——丢弃 `sub_moves`/`rmove.subs`）。
    // 当 L0 末段内点改写使上级投影 bit-identical 但底层 sub_moves 变时，上级 frontier_mutated=false 会
    // 漏 reset ⟹ `upper_moves` 深嵌套 sub_moves 陈旧 + BSP memo 复用陈旧（codex 反例
    // `cascade_reset_on_frontier_interior_rewrite` 坐实）。cascade 消除整类"投影是否捕获深字段变化"
    // 的易错判断（no-patch）：下级变异无条件向上传播，上级不依赖投影完备性。代价：变异 bar（16K 中
    // ~120 次稀疏）该级+所有上级全量重扫，amortized 仍 O(n)。
    let mut cascade_reset = false;
    // ★on2w2-cascade 失效边界定理（设计 §1）：本 bar 累积脏源下界 `e`（最小改变源坐标，逐级恒定
    // 传播）。init usize::MAX（=+∞，无改变 ⟹ 无 cascade）。**生产逻辑**：cascade 命中时按
    // `read_end_src < e` 取保留前缀 P，前缀 bit-identical 保留、后缀失效重扫（撤 #65「整塔前缀清空」）。
    let mut dirty_e: usize = usize::MAX;
    // ★工位 4g：本 bar 是否有任一级 extend 非空 tail（含新级涌现首产 + 最高级 append）——驱动
    // generation +1（与 cascade_reset 一起完整覆盖 extract 可观察树变更，codex Q3）。
    let mut did_extend = false;
    // ★on2w2 forest_epoch dirty 累积器（E1-E3 折叠，循环后统一 bump——避开与循环内 `lc` 可变别名，
    // 同 did_extend/cascade_reset 模式）。E1（L0 塔重建）在循环前置位；E2（upper extend）/E2a
    // （frontier pop）/E3（cascade clear）在循环内 `|=`。E4（clear()）不经此，方法内直接 bump。
    let mut forest_dirty = forest_dirty_l0;
    // ★A3 证书（per-level dirty_from，§2.4）：本级 units 的不可变前缀长度。L0 = l0_dirty_from
    // （§2.3）；L≥1 = 父级 prefix_count（loop 尾 `dirty_from = prefix_count`）。驱动 03（L1+ stable
    // 从 0 抬起）+ 04（cached_units truncate+extend O(tail)）。
    let mut dirty_from: usize = l0_dirty_from;

    for level_idx in 0..=l_max {
        // 自然终止：单元数 < min_parts ⟹ 停止（与 classify_impl 同口径）。
        if units.len() < min_parts {
            // ★§2.6：level_idx 及所有更高级本 bar 全跳过（循环顶部 break，lc 未触碰）——truncate 掉
            // 这段不可达尾巴的陈旧 LevelCache，令其恢复时走 LevelCache::default() 全扫（血缘自洽，
            // 防 stable=dirty_from.min(scanned) 坍缩假阴）。
            #[cfg(test)]
            oracle_probe::on_minparts_break(level_idx < cache.levels.len());
            cache.levels.truncate(level_idx);
            break;
        }

        let is_l0 = level_idx == 0;

        // 缓存槽按需扩展（首次到达该级 ⟹ 新建空 LevelCache，start_i=0 全量扫）。
        if cache.levels.len() <= level_idx {
            cache.levels.push(LevelCache::default());
        }
        let lc = &mut cache.levels[level_idx];

        // #106/A3 §3.3 证书跳前缀：L0 用 parser `segments_confirmed_len`，L1+ 用父级 `dirty_from`。
        let stable_bound = if is_l0 { l0.segments_confirmed_len } else { dirty_from };
        apply_frontier_invalidation(
            lc, &units, stable_bound,
            &mut cascade_reset, &mut dirty_e, &mut forest_dirty,
        );
        lc.last_input_len = units.len();
        // 快照本级输入（下 bar 比对 frontier 变异）。★A3 §3.2 证书化：cached_units[..dirty_from]
        // == units[..dirty_from]（前缀可证不可变，上 bar 04 已写同值）⟹ 前缀无需重拷，truncate
        // + extend tail = O(tail)。三重 min 保证终长 == units.len()（含 units 回缩）。
        // debug_assert = debug/test 护栏（release profile 按 Rust 语义剥离——省下的即此全前缀比较）。
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

        // ★frontier 修复（task #47/#21，区间套.pdf 六~十节裁决②）：resume 起点用 `resume_from`
        // （上次扫描最后一个成立窗口的起点）**而非** `consumed`——`consumed` 越过了最后一个成立
        // 窗口，把它当 sealed prefix，但该窗口第三段可能是 frontier（新 bar 后其后续段落使全量非
        // 重叠扫描产出不同中枢）。回退：从 `resume_from` 重扫 ⟹ 最后一个（frontier）中枢每 bar 重算，
        // 真正 sealed（后面又出现成立窗口）后自然稳定。对应地 **pop 最后一个 center/upper_move**
        // （它会被重扫重新产出），`prefix_count` 用 pop 后的长度（ordinal 接续，全量/增量同 ID）。
        //
        // 保守正确性（PDF §九增量等价定理）：`resume_from <= consumed` 恒成立（窗口起点 ≤ 退出点），
        // 从更早处重扫产出的 tail ⊇ 从 consumed 重扫的 tail（多含重算的末窗口）。cascade_reset 后
        // `scan_cursor = default`（resume_from=0=consumed）⟹ 无回退，从 0 全扫（bit-exact 退化）。
        // 无成立窗口时 `resume_from == 上次 start_i`（无中枢可 pop），guard `resume_from < consumed`
        // 为假 ⟹ 不 pop、从 resume_from(=上次start_i) 续扫（续进语义，仅不成立支推进过的区间）。
        let resume_start = lc.scan_cursor.resume_from;
        let had_emitted_window = lc.scan_cursor.resume_from < lc.scan_cursor.consumed;
        // ★on2w2：本级 upper_moves（=tower[level+1]，forest 输入）字节变更判据。upper_moves 前缀
        // [..prefix_count] 不可变（§16 confirmed），本 bar 只改尾部：pop 掉 `popped_upper`（末窗产出）
        // 后 extend `tail_upper`（重扫产出）⟹ **变更 ⟺ popped_upper != tail_upper**（逐值）。无 pop
        // 时 popped_upper 空 ⟹ 变更 ⟺ tail_upper 非空（纯追加，=E2）。捕获 pop 掉的 upper 尾段以供比对。
        let mut popped_upper: Vec<LeveledMove> = Vec::new();
        let mut popped_cp: Vec<CpScanOwnership> = Vec::new();
        if had_emitted_window {
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
            popped_cp = cp[cp_keep..].to_vec();
            cp.truncate(cp_keep);
            let um = Rc::make_mut(&mut lc.upper_moves);
            let keep = um.len().saturating_sub(pop_n);
            popped_upper = um[keep..].to_vec(); // on2w2：pop 前捕获（与重扫 tail_upper 逐值比）。
            um.truncate(keep);
            // ★on2w2-cascade：win_meta 与 centers/upper_moves 1:1 同步 pop（末窗整窗，重扫重产）。
            lc.win_meta.truncate(lc.win_meta.len().saturating_sub(pop_n));
        }
        // ★codex Q4：prefix_count = lc.upper_moves.len()（pop 后的已产出前缀数），tail ordinal 接续
        // 前缀 ⟹ 全量/增量产同 ElementId（跨 bar 稳定身份）。★A3 §2.2：prefix_count 同时是本级
        // projected_units 的不可变前缀（§2.5 truncate 锚）+ 下一级 units 的 dirty_from（§2.4 loop 尾）。
        let prefix_count = lc.upper_moves.len();
        // ★on2w2-cascade §3.1 契约：cached_second_count（保留 parent 前缀数）须 <= pop 后 prefix_count
        // （否则缓存越过 confirmed 边界，extract_second_resume 单调守卫会重置）。cascade truncate 设
        // cached_second_count=P=win_meta 保留数 <= upper_moves 保留数 == prefix_count（同一 truncate(p)）。
        debug_assert!(
            lc.cached_second_count <= prefix_count,
            "§3.1 契约违反：cached_second_count={} > prefix_count={}",
            lc.cached_second_count, prefix_count
        );
        let (tail_centers, tail_upper, mut tail_cp, tail_metas, new_cursor) =
            stage_profile::time("05_compose_resume", || {
                compose_level_resume(
                    &units,
                    &moves_tower[..],
                    is_l0,
                    level_idx as u32 + 1,
                    resume_start,
                    prefix_count,
                )
            });
        // frontier pop/recompose 若产出同一个 B_p/c_p 身份，继承已扫描对象态，只从 dirty_from 推进。
        // Closed 证书若落入 dirty 后缀则不可继承，必须从 departure 重判；证书完全位于稳定前缀才保留。
        let dirty_invalidation =
            recursive_tower::invalidate_cp_lifecycle_dirty_dependencies(
                Rc::make_mut(&mut lc.cp_ownership).as_mut_slice(),
                dirty_from,
            );
        cp_replay_diagnostics::record_dirty_invalidation(
            level_idx,
            dirty_invalidation.pending_fallbacks,
            dirty_invalidation.certificate_clear_recomputes,
        );
        let mut lifecycle_scan_from = dirty_invalidation.scan_from;
        for object in &mut tail_cp {
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
                lifecycle_scan_from =
                    lifecycle_scan_from.min(departure.ordinal as usize + 1);
            }
        }

        // ★A3 oracle 探针：had_emitted_window pop 后 T = tail_upper.len()（本 bar 本级重扫产出窗口数）。
        // T==1 = did_extend 证伪正向锁（重扫仅复现被 pop 窗口，tail_upper 恰 1）；T>1 = frontier 值改写。
        #[cfg(test)]
        if had_emitted_window {
            oracle_probe::on_pop_rescan(tail_upper.len());
        }

        // ★task #143：走势分解增量无需 frontier 失效钩子——decompose_resume 只冻结 sealed
        // 关系（i < m-2，两端中枢均有后继），触 frontier 中枢的临时尾关系每 bar 重折。frontier
        // pop 后重扫改值 / 一次重扫多产两种破口均被冻结不变量覆盖（decompose.rs 模块头 + 随机
        // 事件流 parity 测试）。
        // 追加到已缓存前缀（前缀不可变，仅尾部追加）⟹ 累积 centers/upper == 全量扫描结果。
        did_extend |= !tail_upper.is_empty();
        // ★on2w2 E2+E2a（合并逐值判据）：本级 upper_moves 尾部从 popped_upper 换成 tail_upper。
        // 变更 ⟺ 两者不逐值相等（含长度）。frontier resume 每 bar pop+重扫复现相同窗口（tail_upper
        // == popped_upper）时**不置 dirty**——这是把 bump 率从 did_extend 的 ~96% 压回 forest 真变率
        // 0.78% 的机制（实证订正：初版 `|=!tail_upper.is_empty()` 对每 bar 重扫复现的相同窗口误 bump）。
        // over-invalidate 保留：长度或任一值不等即 dirty。O(tail) 比对，非全塔。
        forest_dirty |= tail_upper != popped_upper;
        stage_profile::time("06_extend_centers_upper", || {
            // make_mut：strong_count==1 ⟹ 原地 extend O(tail)；>1 ⟹ 写时复制（bit-exact）。
            Rc::make_mut(&mut lc.centers).extend(tail_centers);
            Rc::make_mut(&mut lc.upper_moves).extend(tail_upper);
            Rc::make_mut(&mut lc.cp_ownership).extend(tail_cp);
            // ★on2w2-cascade：win_meta 与 centers/upper_moves 同步 extend（1:1 对齐不变量维持）。
            lc.win_meta.extend(tail_metas);
        });
        let cp_objects = Rc::make_mut(&mut lc.cp_ownership);
        recursive_tower::advance_cp_lifecycles(
            cp_objects.as_mut_slice(),
            &lc.centers,
            &units,
            &moves_tower,
            (!is_l0).then_some(&units_anchors[..]),
            lifecycle_scan_from,
        );
        lc.scan_cursor = new_cursor;
        // ★#93 水线推进（字段文档见 LevelCache::confirmed_watermark）：
        // w_nat = len - last_window_emitted——本 bar frontier 末窗口（下 bar pop 重产）排除。
        // cascade bar 只允许收缩（前缀 min(P) 已在 cascade 分支落账），非 cascade bar 可增长。
        {
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
        debug_assert!(
            lc.centers.len() == lc.upper_moves.len()
                && lc.centers.len() == lc.cp_ownership.len()
                && lc.centers.len() == lc.win_meta.len(),
            "增量塔：centers/upper_moves/cp_ownership/win_meta 一一对应"
        );

        // 本级输入塔快照（compose 前）。
        // ★O(1) 优化：moves_tower 是 Rc——move 入 snapshots（所有权转移，零拷贝）。下一级用
        // Rc::clone(&lc.upper_moves) 重置 moves_tower（line 912），故此处 move 后 moves_tower 失效合法。
        // bit-exact：snapshots 内容 == 全量版（Rc 指向的 Vec 值不变，仅所有权/引用计数变）。
        tower_snapshots.push(std::mem::take(&mut moves_tower));

        // 走势分解（增量续折：resume 单一来源 ⟹ 与全量 decompose 定义性 bit-exact）。
        // ★#148：链尾可变中枢数 = 本轮末窗口产出数（升级重切窗口的全部子中枢在窗口 sealed 前
        // 均可变——外缘随延伸改写、数量随段数增长改变），冻结边界随之后移（decompose.rs 文档）。
        let moves = decompose_resume(
            &lc.centers,
            &mut lc.decompose_state,
            lc.scan_cursor.last_window_emitted.max(1),
        );
        // #69 5b：无条件登记本级一/三类所用 source 水位；不得挂在 BSP memo miss 分支，
        // 否则 hit bar 会暴露陈旧 e_src。公式与 signal resume 单一同源。
        lc.last_freeze_boundary = signal::freeze_boundary_src(&lc.centers, prefix_count, dirty_e);

        // BSP 提取（同 classify_impl：L0 线段层 + 递归组装层）。
        // ★增量接入：传预计算 hist（从 cache 增量产出），避免 extract_signals 内部全量 compute_macd。
        //
        // ★memo 缓存（工位 E，O(n²) 主导根因——profile 坐实 bsp t=0.279s@16K，53% of tower）：
        // BSP 是 (centers, segments, upper_moves, hist, close_src) 的纯函数。其中 centers/segments/
        // upper_moves 跨 bar 单调追加（前缀不可变）；hist/close_src 仅尾 bar（不稳定，inclusion 可改写）
        // 变化。但 **confirmed 线段/中枢/上级走势的 source_index 区间全部落在稳定前缀**——尾 bar 在
        // 所有 confirmed 元素区间之后，故不影响其 BSP 判定。⟹ 当 (centers.len, segments.len,
        // upper_moves.len) 三者与上次一致时，BSP 逐字段 bit-identical（纯函数同输入同输出 + 尾 bar 不
        // 触及 confirmed 区间）。实测 16K bar 中 L0 segments 仅变 120 次（maxseg=134），其余 ~99.2%
        // bar 全量重算是冗余——memo 把 16K 次重算降为 O(段变化次数) 次，每次 O(S)，BSP 总成本坍缩近常数。
        //
        // bit-exact 铁律：guard 命中 ⟹ 复用上次输出（纯函数同输入）；guard miss ⟹ 全量重算并刷新
        // 缓存。与无 memo 版逐字段相等（同 extract_signals_with_hist/extract_second_for_level 代码路径）。
        // ★裁定 A memo soundness：level≥1 的一/三类依赖本级输入 `units`（不止 upper_moves）——units
        // 尾部 append（新级别-N 单元，可能破最后中枢=新一类 C 段 / 与前段构成新三类回试对）会改变
        // 一/三类输出却**不**改 centers.len/upper_moves.len（新单元未凑齐三段窗口 ⟹ 无新中枢/上级走势）。
        // 故 level≥1 的结构长度键用 `units.len()`（L0 用 segments.len()）——append 变长即触 miss 重算。
        // frontier **同长改写**由 cascade_reset（frontier_mutated 比对，scan 窗覆盖全 units 因
        // consumed+2≥units.len()）清 cached_bsp_key 兜底；回缩由 last_input_len 守卫触 cascade。三情形全覆盖。
        let struct_len = if is_l0 { l0.segments.len() } else { units.len() };
        let bsp_key = (lc.centers.len(), lc.upper_moves.len(), struct_len);
        let (bsp, pan_div): (Rc<Vec<BspPoint>>, Rc<Vec<signal::PanDivCert>>) = if lc.cached_bsp_key
            == Some(bsp_key)
        {
            // 07c：memo 命中 ⟹ `Rc::clone`（引用计数 O(1)），替代全量 `cached_bsp.clone()`。
            // Q4：pan_div 同批命中（同 key 守卫 ⟹ 同一 extract 产出的两半锁步复用）。
            stage_profile::time("07c_bsp_memo_clone", || {
                (Rc::clone(&lc.cached_bsp), Rc::clone(&lc.cached_pan_div))
            })
        } else {
            // ★on2w3-07a frontier-resume：confirmed 前缀段的一/三类点缓存复用，只重判 frontier tail
            // （消 07a O(n²) 主导项）。冻结边界锚 = min(centers[prefix_count-2].end_index, dirty_e)——
            // `moves`（= decompose_resume 输出，本级增量续折）作 blocks 单一来源（不重 decompose）。
            // segments 来源：L0=l0.segments（有序）；L≥1=units→unit_to_segment 投影（几何衰减，
            // resume 内 debug_assert 守 end_index 严格递增）。cascade 清 cached_first_third 见 §失效块。
            let (mut b, pan): (Vec<BspPoint>, Vec<signal::PanDivCert>) = if is_l0 {
                stage_profile::time("07a_extract_signals_l0", || {
                    signal::extract_first_third_resume(
                        &mut lc.cached_first_third, &mut lc.cached_first_third_pan,
                        &mut lc.cached_first_third_count, &lc.centers, &l0.segments, None, &moves,
                        prefix_count, dirty_e, hist, dif, &closes_tick, &close_src,
                        config.divergence_gauge,
                    )
                })
            } else {
                stage_profile::time("07a_extract_first_third_ln", || {
                    // 级别-N 一/三类（裁定 A）：units 承担线段角色，复用 L0 判据。units→Segment 投影
                    // （几何衰减 O(units_L)/miss）+ anchors 平行传入。resume 冻结边界同 L0 锚口径。
                    let segs: Vec<Segment> = units.iter().map(unit_to_segment).collect();
                    signal::extract_first_third_resume(
                        &mut lc.cached_first_third, &mut lc.cached_first_third_pan,
                        &mut lc.cached_first_third_count, &lc.centers, &segs, Some(&units_anchors),
                        &moves, prefix_count, dirty_e, hist, dif, &closes_tick, &close_src,
                        config.divergence_gauge,
                    )
                })
            };
            let second = stage_profile::time("07b_extract_second", || {
                // ★07b frontier 门控：confirmed 前缀 parent 的 B2 缓存复用（跳过其重复背驰扫描），
                // 只对 frontier tail 每 bar 重算。消 confirmed-parent 全塔重扫 O(U²)。
                extract_second_resume(
                    &mut lc.cached_second,
                    &mut lc.cached_second_count,
                    &lc.upper_moves[..],
                    prefix_count,
                    hist,
                    &close_src,
                    &area_cache,
                    stable_len,
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
        };

        // 08：`Rc::clone`（O(1)）投影增量塔 centers 到 LevelState，替代全量 `centers.clone()`。
        let level_centers = stage_profile::time("08_levels_centers_clone", || Rc::clone(&lc.centers));
        // #110 投影层 stamping（增量塔 memo-miss/命中终装点同口径；机制位关 = None 零开销）。
        // T3 (#172) 并门：机制位转派生（π 入口 `admission::chain_driven_level_projection`
        // 唯一生产写入点，#168 裁定 3）——链活 ⟹ 层必载，链死不载。
        // T2 (#171)：三元锚供给（`l0.fractals`/`l0.merged_bars`，ParseLayer `Rc` 共享只读
        // 借用，零拷贝）随门开分支引入——门关分支零新增读。
        let level_projection = if config.level_projection.enabled {
            Some(projection::LevelProjectionLayer::from_level(
                levels.len() as u32,
                &bsp,
                &l0.fractals,
                &l0.merged_bars,
            ))
        } else {
            None
        };
        levels.push(LevelState {
            moves,
            centers: level_centers,
            cp_ownership: Rc::clone(&lc.cp_ownership),
            bsp,
            pan_div,
            level_projection,
        });

        // 下一级输入 = 上级走势塔投影（前缀来自缓存 upper_moves 前缀，尾部来自续扫）。
        // ★O(n²) 真修（#106）：增量投影——只对 upper_moves 新 tail 投影（`rmove.lo()/hi()` 递归整棵
        // 子树 O(nodes) 仅算新元素），前缀复用 lc.projected_units。clone 给 units 是 O(level) memcpy
        // （UnitRange: Copy，无递归）。cascade_reset 已清空 projected_units（line 855 旁）⟹ 退化全量。
        // ★A3 §2.5 证书化加固：pop 非 cascade 时 append-only 投影不感知 pop（保留上 bar frontier
        // 投影，隐式依赖 cascade 兜底）。显式 truncate 到 prefix_count 丢弃 pop 掉的 frontier 投影，
        // 从 prefix_count 起重投影（O(tail)）——消除 did_extend 类漏判。cascade 时 prefix_count=0，
        // 与 §上 projected_units.clear() 一致（truncate(0)==clear，冗余无害）。
        stage_profile::time("09_project_to_units_resume", || {
            // make_mut：下一级 units 由 stage 10 `Rc::clone` 共享同一 buffer；本 bar 头部 truncate 前
            // 上轮 units 已被 loop 尾/下 bar 重赋 drop ⟹ strong_count==1 ⟹ 原地 O(tail)；>1 ⟹ 写时
            // 复制（bit-exact 退化）。resume 契约要求 cache.len()<=moves.len()，truncate(prefix_count) 保证。
            let proj = Rc::make_mut(&mut lc.projected_units);
            proj.truncate(prefix_count);
            // Q7（task #145）：方向源 = 本级中枢 ownership 块方向（levels 尾 = 本级刚 push 的
            // LevelState.moves，与 batch 同一 decompose 单一来源）。confirmed 前缀方向冻结
            // （R(i-1,i) sealed 后标签不变），frontier 由 truncate(prefix_count) 每 bar 重投影。
            recursive_tower::project_to_units_resume(
                &lc.upper_moves[..],
                &levels.last().expect("本级 LevelState 已 push").moves,
                proj,
            );
        });
        // ★A3 投影证书护栏（debug/test）：truncate(prefix_count)+resume 必逐字段等全量 project_to_units。
        debug_assert!(
            *lc.projected_units
                == recursive_tower::project_to_units(
                    &lc.upper_moves,
                    &levels.last().expect("本级 LevelState 已 push").moves
                ),
            "投影证书违反：projected_units != 全量 project_to_units（truncate(prefix_count)/resume 破裂）"
        );
        // 10：`Rc::clone`（O(1)）替代全量 `projected_units.clone()`（O(units_L)/bar）。下一级借
        // &units[..] 只读；stage 09 头部 make_mut 时本 Rc 已 drop（loop 尾重赋）⟹ 原地写不退化。
        units = stage_profile::time("10_projected_units_clone", || Rc::clone(&lc.projected_units));
        // Q7-#1 裁定C：锚资格与投影同源同步派生（deterministic 于 (moves, len) ⟹ resume bit-exact：
        // 全量与增量在同一 (pb, units.len()) 上得同一锚数组，无缓存陈旧面）。
        units_anchors = {
            let pb = &levels.last().expect("本级 LevelState 已 push").moves;
            (0..units.len()).map(|i| decompose::center_own_dir_at(pb, i)).collect()
        };

        // ★A3 §2.4：本级 prefix_count 是下一级 units 的不可变前缀（父 confirmed 前缀投影稳定）。
        dirty_from = prefix_count;

        if units.is_empty() {
            // ★§2.6：level_idx 已处理完（尾部 break），level_idx+1 及更高级本 bar 因空 units 全跳过——
            // truncate 掉这段不可达尾巴的陈旧 LevelCache（血缘自洽，恢复时全扫重建）。
            #[cfg(test)]
            oracle_probe::on_empty_break(level_idx + 1 < cache.levels.len());
            cache.levels.truncate(level_idx + 1);
            break;
        }
        // ★O(n) 重构：moves_tower = Rc::clone(&lc.upper_moves) —— O(1) 引用计数，消除 per-bar 全塔
        // 深拷贝（旧 `lc.upper_moves.clone()` 是 O(n²) 热点①根因）。下一级 line 832 借 &moves_tower[..]
        // 只读，line 854 move 入 snapshots。lc.upper_moves 跨 bar 持久于 cache；extend（line 843）经
        // make_mut，caller 逐 bar drop snapshot ⟹ strong_count==1 ⟹ 原地 O(tail)。
        // bit-exact：Rc 指向同一 Vec，逐字段与旧 clone 等价。
        moves_tower = Rc::clone(&lc.upper_moves);
    }

    // closes/close_src 缓冲放回 cache（mem::take 取出的所有权归还，下 bar 复用，零额外分配）。
    cache.closes = closes;
    cache.close_src = close_src;
    cache.closes_tick = closes_tick; // force 价格振幅/速度 proxy 缓冲放回，下 bar 复用（同 closes 模式）。
    cache.area_cache = area_cache.into_inner(); // B3 #4 area-memo：缓冲放回，下 bar 复用（同上模式）。

    // ★工位 4g：本 bar 若有 cascade 重扫或任一级 extend 非空 tail ⟹ extract_elements 可观察树变更 ⟹
    // +generation（下游 TreeCache 据此 O(1) 跳过 TreeKey::of）。无变化 bar（~99.8%）generation 不变 ⟹
    // 命中。soundness：保守过度计数（低级 extend 不动最高级输出时多算一次 TreeKey）安全，绝不假命中。
    //
    // ★L0-root 边界（codex Q3 漏洞 + gen_fastpath_bit_exact_debug bar 345 坐实）：当最高非空级是 **L0**
    // （tower 无 compose 级，extract 直接读 `moves_tower_l0`），L0 走势塔每 bar 从 `l0.segments` 重建
    // （非缓存 Rc），其增长/同 len 古怪线段重划**不经** cascade/did_extend（那两者只覆盖各级 upper_moves）。
    // 故 L0-root 阶段**强制每 bar +generation**（走 TreeKey fallback）——此阶段 tree 极小（早期 bar），
    // TreeKey O(small) 不影响大 n 标度。一旦 L1+ 出现（extract 读 L1），L0 任何变化必经 cascade 传播至
    // L1（codex Q1：L0 frontier 改写→L1 units 投影变→L1 frontier_mutated→cascade），被完整捕获。
    let highest_nonempty = tower_snapshots.iter().rposition(|s| !s.is_empty());
    let l0_is_root = highest_nonempty == Some(0);
    if cascade_reset || did_extend || l0_is_root {
        cache.generation += 1;
    }

    // ★on2w2 forest_epoch：E1-E3 折叠（forest_dirty，含 E1 L0 塔重建 / E2 upper extend / E2a frontier
    // pop / E3 cascade clear）循环后统一 bump。E4（clear()）不经此（方法内已 bump）。与 generation 的
    // 关键区别：generation 靠 l0_is_root 每-bar blunt 兜底（bump 率 98.5%）；forest_epoch 只在塔实际字节
    // 变更时 bump（E1 直接捕获 L0 变更，无需 blunt 兜底）⟹ bump 率贴近 forest 真变率 ≈0.8%（on2w2 §2）。
    // over-invalidate：写入站点无条件 dirty，宁可多失效不可假命中（假命中不可能性证明 on2w2 §4）。
    if forest_dirty {
        cache.forest_epoch += 1;
    }
    #[cfg(test)]
    oracle_probe::on_forest_dirty(forest_dirty_l0, forest_dirty && !forest_dirty_l0 && !cascade_reset, cascade_reset);

    // ★#613（#609 F2）：`l0_units()` 只读契约的不变式——**塔已产出 L0 级快照时**，
    // `l0_units_cache` 与 `tower[0]` 同长（同源同序由 `moves_tower_l0` 的构造保证：两者都是
    // `l0.segments` 的逐元素纯函数投影，且本函数内只有一处写入站点）。
    //
    // 限定「非空」是结构事实而非放宽：段数不足以构造任何级别时 `tower_snapshots` 为空而
    // `l0_units_cache` 已有内容（BTC 100k 实测 278 个早期 bar），此时消费方
    // （`p123_fast_replay::recompute_lifecycle_window_stems`）走 `tower_level_absent` 显式原因码，
    // 根本不读 `l0_units` ⟹ 无契约面。
    debug_assert!(
        tower_snapshots
            .first()
            .is_none_or(|l0_tower| l0_tower.len() == cache.l0_units_cache.len()),
        "l0_units() 契约违反：l0_units_cache.len()={} 与 tower[0].len()={:?} 失步（#613/#609 F2）",
        cache.l0_units_cache.len(),
        tower_snapshots.first().map(|t| t.len())
    );

    (Classification { levels }, tower_snapshots)
}

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
fn apply_frontier_invalidation(
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
