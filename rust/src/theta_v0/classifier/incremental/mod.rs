//! 增量塔入口 [`classify_with_tower_incremental`]（task #93）：塔构造的中枢扫描走增量
//! resume（前级 confirmed 前缀缓存，仅尾部续扫），解 per-bar substrate 的塔构造 O(n²) 根因。
//!
//! bit-exact 保证 / 硬契约 / 增量有效性 / 边界见函数文档；缓存本体见 [`super::tower_cache`]。

use super::pipeline::{build_level_projection, segment_to_unit};
use super::sublevel::extract_second_resume;
use super::tower_cache::{compute_macd_hist_incremental, update_closes_cache, AreaCache, LevelCache};
use super::*;

mod assemble;
mod invalidate;
mod scan;
mod tower;

// 兄弟子模块的 `pub(super)` 面在此 glob 汇聚——子模块各自 `use super::*` 即可互取
// （与 #576 的 classifier 子模块口径同构；glob import 不产生 unused 警告）。
use assemble::*;
use invalidate::*;
use scan::*;
use tower::*;

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
    if std::env::var(crate::theta_v0::env_registry::DIAG_L0UNITS).is_err() {
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
///
/// ## 序列面出借（T6–T9，函数体内不可重排）
///
/// MACD hist（背驰真算，增量递推——231号纯性能，解 aed4d5f5 实证的 c_exp≈2.31 主导根因）。
/// 增量策略（bit-exact 铁律，ac75d4b3 已证 append bit-exact）：
/// - `merged_bars.close` 前缀与 `macd_closes_prefix` 逐值一致 + 长度 >= 前缀 ⟹ 从 `macd_state`
///   增量 append 新 close，`current_point().hist` 追加到 `macd_hist`；
/// - merged_bars 回缩 / 前缀改写 / cache 空 ⟹ `macd_state_from_closes` 全量重建 state，
///   并逐 bar append 重建 hist 数组（bit-exact 退化，同塔 cache clear 逻辑）。
///
/// ★不调 `compute_macd`（全量）——增量路径用 `MacdState::append` O(1)/bar；退化路径用
/// `macd_state_from_closes` + 逐 bar `current_point`（与全量同 EMA 约简，bit-exact）。
/// closes/close_src 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写——同 MACD stable_prefix
/// 语义），不每 bar 全量重建（O(n)/bar → O(n²) 根因之一，工位 E profile 坐实 t=0.085s@16K）。
///
/// `mem::take` 取出四个缓存 `Vec`（`closes`/`close_src`/`closes_tick`/`area_cache`）——**唯一目的**
/// 是解除 `&cache.closes` 与下游 `&mut cache` 的别名（不 take 则 `compute_macd_hist_incremental`
/// 编译失败）；用毕在函数尾原样放回（缓冲复用，零额外分配）。`closes_tick`（整数 close）供
/// force_state 生产热路由（beta-route #115）的一类候选 A/C 段振幅/速度 proxy。
///
/// B3 #4 area-memo：`stable_len` = 本 bar hist 的确认边界（[`AreaCache`] 文档），**必须在 MACD
/// 增量之后读**（T8：该调用写 `macd_state_len`；先读则缓存 unstable tail 的 area，污染未来 bar）。
/// `RefCell` 包裹：`divergence_of` 闭包接口是 `impl Fn(&RMove) -> bool`
/// （`signal::extract_second_signals`），`Fn` 只给闭包体 `&self` 访问——捕获的可变缓存须走内部
/// 可变性（共享引用 + `borrow_mut`），不能捕获 `&mut AreaCache`（Long/Short 两侧各建一个闭包，
/// 同一 `&mut` 不能捕获两次）。
///
/// `hist`/`dif` 借 `cache.macd_{hist,dif}`（disjoint field 借用——[`build_level_tower`] 内的
/// `&mut cache.levels[i]` 借的是另一字段路径）。force DIF 峰 proxy 输入，bit-exact 等价全量。
pub fn classify_with_tower_incremental(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    // L0 侧序幕（回缩校验 → units 增量 → 空判 → 水线落账 → L0 塔增量）。内部五步次序受 T1–T5 约束。
    let Some(prelude) = prepare_l0_inputs(l0, cache) else {
        // 空 L0 边界：缓存已在序幕内清空（下次从头扫）。
        return (Classification::default(), Vec::new());
    };

    // 序列面出借（T6–T9，逐条见本函数文档「## 序列面出借」）。四个缓冲用毕在函数尾放回。
    stage_profile::time("00_update_closes_cache", || {
        update_closes_cache(&l0.merged_bars, l0.merged_confirmed_len, cache)
    });
    let closes: Vec<f64> = std::mem::take(&mut cache.closes);
    let close_src: Vec<usize> = std::mem::take(&mut cache.close_src);
    let closes_tick: Vec<Tick> = std::mem::take(&mut cache.closes_tick);
    stage_profile::time("02_macd_incremental", || {
        compute_macd_hist_incremental(&closes, l0.merged_confirmed_len, &config.macd, cache)
    });
    let stable_len = cache.macd_state_len; // T8：必须在 MACD 增量之后读
    let area_cache: RefCell<AreaCache> = RefCell::new(std::mem::take(&mut cache.area_cache));
    let series = LevelSeries {
        hist: &cache.macd_hist,
        dif: &cache.macd_dif,
        closes_tick: &closes_tick,
        close_src: &close_src,
        area_cache: &area_cache,
        stable_len,
    };

    let built = build_level_tower(l0, config, &mut cache.levels, prelude, &series);

    // closes/close_src 缓冲放回 cache（mem::take 取出的所有权归还，下 bar 复用，零额外分配）。
    cache.closes = closes;
    cache.close_src = close_src;
    cache.closes_tick = closes_tick; // force 价格振幅/速度 proxy 缓冲放回，下 bar 复用（同 closes 模式）。
    cache.area_cache = area_cache.into_inner(); // B3 #4 area-memo：缓冲放回，下 bar 复用（同上模式）。

    bump_tower_epochs(cache, &built);
    debug_assert_l0_units_in_sync(cache, &built.tower_snapshots);

    (Classification { levels: built.levels }, built.tower_snapshots)
}

/// [`build_level_tower`] 的产出：本 bar 的分级状态 + 塔快照 + 三个跨级累积判据。
struct TowerBuild {
    levels: Vec<LevelState>,
    tower_snapshots: Vec<Rc<Vec<LeveledMove>>>,
    /// 任一级检出 frontier 变异（含更高级无条件跟随）。驱动 `generation` bump + 水线只许收缩。
    cascade_reset: bool,
    /// 任一级 extend 非空 tail（含新级涌现首产 + 最高级 append）。驱动 `generation` bump。
    did_extend: bool,
    /// on2w2 E1-E3 折叠：塔实际字节变更。驱动 `forest_epoch` bump。
    forest_dirty: bool,
    /// E1 分量（L0 塔本 bar 重建），仅供 `on_forest_dirty` 探针分辨 E1 vs E2/E3。
    ///
    /// `allow(dead_code)`：唯一读点在 `#[cfg(test)]` 探针内（同 [`PoppedFrontier::had_window`]）。
    #[allow(dead_code)]
    forest_dirty_l0: bool,
}

/// ★工位 4g：本 bar 若有 cascade 重扫或任一级 extend 非空 tail ⟹ `extract_elements` 可观察树变更 ⟹
/// `+generation`（下游 `TreeCache` 据此 O(1) 跳过 `TreeKey::of`）。无变化 bar（~99.8%）generation
/// 不变 ⟹ 命中。soundness：保守过度计数（低级 extend 不动最高级输出时多算一次 TreeKey）安全，
/// 绝不假命中。
///
/// ★L0-root 边界（codex Q3 漏洞 + `gen_fastpath_bit_exact_debug` bar 345 坐实）：当最高非空级是 **L0**
/// （tower 无 compose 级，extract 直接读 `moves_tower_l0`），L0 走势塔每 bar 从 `l0.segments` 重建
/// （非缓存 Rc），其增长/同 len 古怪线段重划**不经** cascade/did_extend（那两者只覆盖各级 upper_moves）。
/// 故 L0-root 阶段**强制每 bar +generation**（走 TreeKey fallback）——此阶段 tree 极小（早期 bar），
/// TreeKey O(small) 不影响大 n 标度。一旦 L1+ 出现（extract 读 L1），L0 任何变化必经 cascade 传播至
/// L1（codex Q1：L0 frontier 改写→L1 units 投影变→L1 frontier_mutated→cascade），被完整捕获。
///
/// ★on2w2 forest_epoch：E1-E3 折叠（`forest_dirty`，含 E1 L0 塔重建 / E2 upper extend / E2a frontier
/// pop / E3 cascade clear）循环后统一 bump。E4（`clear()`）不经此（方法内已 bump）。与 `generation` 的
/// 关键区别：`generation` 靠 `l0_is_root` 每-bar blunt 兜底（bump 率 98.5%）；`forest_epoch` 只在塔实际
/// 字节变更时 bump（E1 直接捕获 L0 变更，无需 blunt 兜底）⟹ bump 率贴近 forest 真变率 ≈0.8%（on2w2 §2）。
/// over-invalidate：写入站点无条件 dirty，宁可多失效不可假命中（假命中不可能性证明 on2w2 §4）。
///
/// **T10 次序**：本函数必须在 closes/close_src/closes_tick/area_cache 四个缓冲放回之后调用
/// （放回与 bump 都要 `&mut cache`，而 `hist`/`dif` 的共享借用须先在放回处结束）。
fn bump_tower_epochs(cache: &mut TowerCache, built: &TowerBuild) {
    let highest_nonempty = built.tower_snapshots.iter().rposition(|s| !s.is_empty());
    let l0_is_root = highest_nonempty == Some(0);
    if built.cascade_reset || built.did_extend || l0_is_root {
        cache.generation += 1;
    }
    if built.forest_dirty {
        cache.forest_epoch += 1;
    }
    #[cfg(test)]
    oracle_probe::on_forest_dirty(
        built.forest_dirty_l0,
        built.forest_dirty && !built.forest_dirty_l0 && !built.cascade_reset,
        built.cascade_reset,
    );
}

/// ★#613（#609 F2）：[`TowerCache::l0_units`] 只读契约的不变式——**塔已产出 L0 级快照时**，
/// `l0_units_cache` 与 `tower[0]` 同长（同源同序由 `moves_tower_l0` 的构造保证：两者都是
/// `l0.segments` 的逐元素纯函数投影，且只有一处写入站点 [`build_l0_units_cache`]）。
///
/// 限定「非空」是结构事实而非放宽：段数不足以构造任何级别时 `tower_snapshots` 为空而
/// `l0_units_cache` 已有内容（BTC 100k 实测 278 个早期 bar），此时消费方
/// （`p123_fast_replay::recompute_lifecycle_window_stems`）走 `tower_level_absent` 显式原因码，
/// 根本不读 `l0_units` ⟹ 无契约面。
fn debug_assert_l0_units_in_sync(cache: &TowerCache, tower_snapshots: &[Rc<Vec<LeveledMove>>]) {
    debug_assert!(
        tower_snapshots
            .first()
            .is_none_or(|l0_tower| l0_tower.len() == cache.l0_units_cache.len()),
        "l0_units() 契约违反：l0_units_cache.len()={} 与 tower[0].len()={:?} 失步（#613/#609 F2）",
        cache.l0_units_cache.len(),
        tower_snapshots.first().map(|t| t.len())
    );
}
