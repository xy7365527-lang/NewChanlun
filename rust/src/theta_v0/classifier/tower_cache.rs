//!  增量塔 API（task #93：per-bar substrate 塔构造 O(n²) → O(n)）
//!
//! ## 超线性根因（前序 aed4d5f5 实证：classify c_exp≈2.31 主导）
//!
//! `classify_impl` 的塔循环 `for level_idx in 0..=l_max` 每级调 `compose_level`（→
//! `detect_centers_windowed`）+ `classify_level`（→ `detect_centers_with`）——两次全量滑窗扫描。
//! per-bar substrate（每 bar 追加段）下，前级 confirmed 前缀稳定却重复全量扫描 ⟹ 超线性。
//!
//! ## 增量策略（严格有效域声明，formalization-validity-domain 231号）
//!
//! **有效域**：`tower_snapshots`（Vec<Vec<LeveledMove>>）+ `LevelState.centers` 的**中枢扫描构造**
//! 可增量——`detect_centers_windowed` 是确定性左折叠（见 recursive_tower.rs 增量证明），已产出
//! 中枢是不可变前缀，尾部追加续扫产出 bit-exact 尾部。
//!
//! **不在有效域**（诚实声明，no-声明膨胀）：
//! - `LevelState.moves`（走势分解）：`decompose(&centers)` 在完整 centers 序列上折块，
//!   尾部追加中枢可改变整体裁决 ⟹ 须每 bar 从累积 centers 全量裁决（非增量）。但裁决是 O(centers)
//!   单趟，不是超线性源。
//! - `LevelState.bsp`：依赖 MACD hist（每 bar 变）+ 完整 centers ⟹ 每 bar 全量重算。
//!
//! 故增量塔的 LevelState.centers/moves/bsp 由**累积的完整 centers 序列**经与全量同口径的裁决/
//! BSP 提取产出（bit-exact），仅**中枢扫描构造**走增量 resume。塔构造的超线性（双次全量扫描）
//! 被消除 ⟹ exp≈1。
//!
//! **641 谱系标注（有效域声明）**：incr_total exp 0.91 @16K ≈ 1.0 → 塔构造 O(n) **已达成**
//! （L2 真实数据验证，非 L0/L1）。有效域 = 中枢扫描构造（compose_level_resume）+ parse_layer
//! 增量合并（process_inclusion）。**不在有效域**：全引擎 per-bar 端到端 exp 须大规模验证（moves/
//! bsp 裁决每 bar 全量但 O(centers) 单趟，非超线性源；实测端到端 exp 见 benchmark）。
//! **注意（ab5f5a29d）**：O(n) 达成 ≠ 身份稳定→Stale 降根。增量塔 bit-exact 跨 bar 复用，但
//! held_leg_tree_index 值字段比较（level/ρ/eps/λ）非对象身份——bit-exact 不变 ⟹ Stale 不降。
//! "增量塔→身份连续→Stale 降根→ΔSharpe 可非零"路径被 L2 证伪（见 runner.rs 注释 + memory
//! newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass）。
//!
//! ## 跨级增量传播（严格不变量）
//!
//! 每级 units = `project_to_units(&upper_moves)`。上级 upper_moves 尾部追加时下级 units 尾部变，
//! 下级从自己的断点续扫。**前缀不变量链**：L0 段前缀不变 ⟹ L0 upper_moves 前缀不变（resume 已证）
//! ⟹ L1 units 前缀不变（project 逐元素，前缀同序同值）⟹ L1 扫描断点有效 ⟹ ... 逐级传播。
//!
//! bit-exact 充要：`cache.last_units_len` 与再次进入时的前缀严格增长（只追加，不修改前缀）。
//!
//! #748（C4）纯移动自 `classifier/mod.rs`（原内嵌 `TowerCache`/`LevelCache`）。

use super::*;
use recursive_tower::WindowScanCursor;


/// 单级增量缓存：已确认前缀 + 续扫断点。
///
/// 不变量（跨 bar 保持）：
/// - `scan_cursor.consumed`：本级窗口扫描退出断点（上次扫到此处，`units[..consumed]` 路径确定）。
/// - `upper_moves`：本级已 compose 的上级走势序列（前缀不可变；尾部续扫追加）。
/// - `last_input_len`：上次扫描时本级 `subs_moves`（= units）长度——再次进入时前缀须不大于此。
/// - `decompose_state`：增量走势分解冻结前缀（sealed 关系折出的块，O(1)/bar 续折）。
#[derive(Debug, Clone, Default)]
pub(super) struct LevelCache {
    pub(super) scan_cursor: WindowScanCursor,
    /// 已 compose 的上级走势序列（前缀不可变；尾部续扫追加）。每元素 `RMove::Compose` 携真 subs。
    /// ★O(n) 重构：`Rc` 共享塔——`moves_tower`/`tower_snapshots` 经 `Rc::clone`（O(1) 引用计数）取得，
    /// 消除 per-bar 全塔深拷贝（O(n²) 热点①）。`extend` 经 `Rc::make_mut`：caller 逐 bar drop 返回的
    /// snapshot ⟹ 下 bar extend 时 strong_count==1 ⟹ 原地追加 O(tail)，不触发写时复制。
    pub(super) upper_moves: Rc<Vec<LeveledMove>>,
    /// 已识别中枢序列（与 `upper_moves` 一一对应，每窗口一中枢；前缀不可变，尾部续扫追加）。
    pub(super) centers: Rc<Vec<Center>>,
    /// 与 `centers`/`upper_moves` 1:1 的完整 `c_p` pending/closed 生命周期对象。
    pub(super) cp_ownership: Rc<Vec<CpScanOwnership>>,
    pub(super) last_input_len: usize,
    /// 上次扫描的本级输入单元快照（前缀变异检测）。`detect_centers_windowed_resume` 充要条件 #2
    /// 要求 `units[..consumed]` 跨 bar 不可变；但 parser frontier 末段会**原地改写**（缠论古怪线段
    /// 重划，67/78课「顶高于底」+ 特征序列再分辨——定义层正确行为，非 parser bug）。仅长度回缩
    /// 守卫漏掉「长度不变/增长但末段值改写」的 frontier 变异。本快照逐值比对扫描区，变异 ⟹ 该级
    /// 缓存全量重置（anc.pdf §16：confirmed prefix immutable 可跳，frontier mutable 必须每 bar 重算）。
    pub(super) cached_units: Vec<UnitRange>,
    /// 增量走势分解状态（decompose.rs resume 单一来源；冻结不变量见其模块头——只折 sealed
    /// 关系，临时尾关系每 bar 重折 ⟹ frontier 中枢改写/一次多产无需额外失效钩子）。
    pub(super) decompose_state: decompose::DecomposeState,
    /// BSP memo 缓存：上次提取的 BSP 序列（与 `cached_bsp_key` 配对）。
    pub(super) cached_bsp: Rc<Vec<BspPoint>>,
    /// ★Q4（task #145）：盘整背驰证书 memo 缓存——与 `cached_bsp` 同一 extract 调用产出、同一
    /// `cached_bsp_key` 守卫（hit/miss/cascade 三态与 bsp 锁步 ⟹ 下游 Rc::ptr_eq(bsp) 蕴含 pan_div 同批）。
    pub(super) cached_pan_div: Rc<Vec<signal::PanDivCert>>,
    /// #550 双域候选 memo；与 BSP key 同失效边界，只在结构 tail 变化时重算。
    pub(super) cached_candidate_key: Option<(usize, usize, usize)>,
    pub(super) cached_candidate_observations: Vec<cand_event::CandidateObservation>,
    /// BSP memo guard key = (centers.len, upper_moves.len, segments.len[L0 only])。三者不变 ⟹
    /// BSP 纯函数同输入同输出（confirmed 元素区间在稳定前缀，尾 bar 不影响）⟹ 复用缓存 bit-exact。
    pub(super) cached_bsp_key: Option<(usize, usize, usize)>,
    /// ★O(n²) 真修（#106）：本级 `upper_moves` 投影缓存（= `project_to_units(&upper_moves)`，下一级输入）。
    /// `upper_moves` 前缀不变仅尾部 append（§16）⟹ 投影前缀不变（`fold_direction(prev)` 只依赖元素+前驱，
    /// 前缀稳定 ⟹ 前缀投影稳定）。每 bar 只对新 append 的 tail 投影（`projected_units.len()..`），避免
    /// per-bar 全量 `project_to_units` 递归 `rmove.lo()/hi()` 整棵子树 O(nodes)/bar=O(n²)。
    /// cascade_reset（前缀重排）⟹ 与 upper_moves 同步清空（line 850 旁）重投影。
    /// ★H7 Rc 化：stage 10 下一级 `units = Rc::clone`（O(1)）替代全量 `projected_units.clone()`
    /// （O(units_L)/bar=O(n²)）。truncate/resume 前经 `Rc::make_mut`：caller 逐 bar drop 上轮
    /// `units`（loop 尾/下 bar 重赋）⟹ strong_count==1 ⟹ 原地写（stage 09 O(tail) 不退化）；
    /// >1（理论跨 bar 持有）⟹ 写时复制（bit-exact）。同 A1 泳道 centers/bsp Rc 化。
    pub(super) projected_units: Rc<Vec<UnitRange>>,
    /// #69 5b（#614 并线自 kimi 线 `tower_cache.rs` 引入）：本级一/三类所用的 source 水位快照。
    /// **只写缓存**——生产分类/交易路径无任何读点，唯一读点是 [`TowerCache::freeze_boundary`]
    /// （诊断 bin `p123_fast_replay` 消费）。公式与 signal resume 单一同源。
    pub(super) last_freeze_boundary: usize,
    /// ★07b frontier 门控（A 泳道 resume 家族，与 A3/07c 同族）：confirmed 前缀 `upper_moves` 的第二类
    /// B2/S2 输出缓存。`extract_second_for_level` 每 memo-miss 全塔重扫 O(U)=O(n²)，但每个 parent 的 B2
    /// 只依赖该 parent（`c1`/subs）+ hist/close_src——confirmed 前缀 parent 的源区间落稳定前缀、hist 前缀
    /// append-only 稳定 ⟹ 其 B2 跨 bar 不变。缓存前缀 B2（稀疏：多数 parent 产 0 个 B2），每 bar 只重算
    /// frontier tail。cascade_reset（前缀重排）⟹ 同步清空（与 cached_bsp 一致，line 1119 旁）。
    pub(super) cached_second: Vec<BspPoint>,
    /// `cached_second` 已覆盖的 `upper_moves` 前缀数（推进锚，= 上次门控的 prefix_count）。
    pub(super) cached_second_count: usize,
    /// ★on2w3-07a frontier-resume（A 泳道 resume 家族，与 07b/07c 同族）：confirmed 前缀**段**的
    /// 一/三类 BspPoint 缓存（[`signal::extract_first_third_resume`]）。07a 每 memo-miss 全量重判全部
    /// S 段（第一/盘整/三类）跨 N bar = O(S·N)=O(n²)（on2w3 profile 坐实 BTC-1M 94%/exp≈2.20）。冻结
    /// 边界 `e_src` 之前的段的点跨 bar 不变（趋势门 sealed + A/C 后向窗口 + hist 前缀 append-only）⟹
    /// 缓存前缀点（push 序），每 bar 只重判 frontier tail。cascade（前缀重排）⟹ 同步清空（与
    /// cached_second/cached_bsp 一致纪律）。
    pub(super) cached_first_third: Vec<BspPoint>,
    /// `cached_first_third` 配套的盘整背驰证书缓存（同一 [`signal::extract_first_third_resume`] 产出、
    /// 同一 push 序、同一冻结边界锚——与点缓存锁步失效）。
    pub(super) cached_first_third_pan: Vec<signal::PanDivCert>,
    /// `cached_first_third{,_pan}` 已覆盖的 confirmed 段前缀数（推进锚 = 上次冻结边界 stable_seg）。
    pub(super) cached_first_third_count: usize,
    /// ★on2w2-cascade 读域侧车（设计 §4.1 解 A）：与 `centers`/`upper_moves` 1:1 对齐的每 center
    /// 窗口读域元数据（`WinMeta.read_end_src` 停止哨兵源坐标 + win_start/win_exit/emitted）。cascade
    /// 增量失效按 `read_end_src < e` 取保留前缀 P，用 `win_meta[P-1]` 重建 cursor（把 P-1 窗口当
    /// frontier 待 pop，回归常态 pop 语义）。前缀不可变 + 尾部续扫追加（同 centers 纪律）；frontier
    /// pop 时同步 pop、cascade 增量时同步 truncate(P)、cascade 全清（e=0）时 clear。
    pub(super) win_meta: Vec<WinMeta>,
    /// ★#93 水线证书（tower_confirmed_len 单一来源）：`upper_moves[..w]` **跨 bar bit-stable
    /// 下界**（保守）。维护站点（与塔写入站点一一对应，over-shrink 安全 / over-grow 禁止）：
    /// - 非 cascade bar 末：`w = upper_moves.len() - scan_cursor.last_window_emitted`
    ///   （frontier 整窗下 bar 会被 pop 重产，排除在水线外；其余已产出窗口只增不改）；
    /// - cascade bar：先 `min(P)`（保留前缀 partition_point——[P..] 本 bar 重扫可能改写），
    ///   末尾再 `min(w_nat)`（不越过本 bar frontier）；P=0 全清 ⟹ 0；
    /// - `LevelCache::default()`（血缘断裂/minparts truncate 重建）⟹ 0（消费方视为水线
    ///   回退，保守全量重比——恒正确退化，admission.rs LevelFingerprint 契约）。
    pub(super) confirmed_watermark: usize,
}

/// B3 #4 area-memo（07b 残余 O(n²) 根治）：`(start,end)→segment_macd_area` 冻结缓存，跨 bar 持久
/// （挂 [`TowerCache`]，非 per-level——`hist` 全局单份，键值与 level 无关）。B3 profile 坐实：同一
/// (start,end) 被 [`sublevel_diverges`] 跨 bar 重复查询（calls=4.19M / distinct=2314 @ 300K，冗余
/// 99.94%）——07b 门控消除了「confirmed 前缀 parent 每 bar 重扫」，但 frontier parent 每 bar 仍对
/// 其固定 `prev_seg`（已稳定、远端）重新线性求和一次，O(range) 逐 bar 累积 = 残余 O(n²)。
///
/// 值一旦写入永久有效（hist 前缀 append-only 稳定，见 [`TowerCache::macd_hist`]）——**只对
/// `end < stable_len` 的查询读写缓存**（[`cached_segment_area`]），`stable_len` = 当前 bar 的
/// `TowerCache::macd_state_len`（`compute_macd_hist_incremental` 每 bar 末元素是 unstable tail，
/// 下 bar 可能被改写覆盖，见其函数头注释——绝不缓存该越界查询，防污染未来错值）。
pub(super) type AreaCache = HashMap<(usize, usize), f64>;

/// 增量塔缓存（跨 bar 跨级复用）：每级 `LevelCache` + L0 段账本快照长度 + MACD 增量状态。
///
/// **使用契约**（bit-exact 充要，违反则增量破裂）：
/// 1. 每 bar 喂 `classify_with_tower_incremental(l0_i, config, &mut cache)`，`l0_i.segments` 是
///    `parse_layer(&bars[..=i])`——段账本**只允许尾部增长**（前缀段不可变，parser 前缀稳定语义）。
/// 2. 若某 bar 段账本前缀**回缩或改写**（非单调追加），须 `cache.clear()` 重置（退化为全量）。
/// 3. `config.level`（l_max/min_parts）不可变——变则 `cache.clear()`。
/// 4. `merged_bars.close` 前缀须单调追加（前缀不变）——inclusion 合并可能改写尾部，
///    故 `macd_closes_prefix` 校验 `closes[..last_merged_len]` 逐值一致；不一致则全量重建。
#[derive(Debug, Clone, Default)]
pub struct TowerCache {
    /// 逐级缓存（下标 = 级别 idx，与 `tower_snapshots` 同构）。
    pub(super) levels: Vec<LevelCache>,
    /// 上次处理的 L0 段数（前缀不变量校验用）。
    pub(super) last_l0_segments_len: usize,
    /// ★#93：本 bar L0 塔（tower[0]）确认前缀 = `l0.segments_confirmed_len`（parser 证书，
    /// tower[0][i] 是 segments[i] 的纯函数 ⟹ 前缀 bit-stable 同传）。每 bar 覆写。
    pub(super) l0_confirmed_len: usize,
    /// MACD 增量递推状态（None=未初始化；Some=已处理至 `macd_state_len` 末）。
    pub(super) macd_state: Option<MacdState>,
    /// 已增量产出的 hist 前缀（不可变；尾部 append 续产）。bit-exact 等价于
    /// `compute_macd(closes[..macd_state_len]).hist`。
    pub(super) macd_hist: Vec<f64>,
    /// 已增量产出的 dif 前缀（黄白线，与 `macd_hist` **逐 bar 锁步**——同一 `MacdState::current_point`
    /// 派生，同一 truncate/push 边界）。bit-exact 等价于 `compute_macd(closes[..macd_state_len]).dif`
    /// （dif 是 hist 的子表达式 `hist=dif-dea`，hist 增量已证 bit-exact ⟹ dif 同证）。force_state 生产
    /// 热路由（一类候选 A/C 段 `segment_dif_peak`）消费——`extract_signals_with_hist` 收 dif 才算 force。
    pub(super) macd_dif: Vec<f64>,
    /// MACD state 实际消费的 close 数（state 表示 `closes[..macd_state_len]` 的累积）。
    /// #106：替代旧 `macd_closes_prefix: Vec<f64>`（每 bar O(n) 全量比较 + to_vec 克隆 = O(n²)）。
    /// 增量边界 = `macd_state_len`；前缀稳定性靠 parser `confirmed_len` 证书（codex：绑 state_len
    /// 而非 cached_len-1，修血缘断裂漏洞）。config 变更经 `clear()` 失效（ponytail: config 在 cache
    /// 生命周期固定，clear 已覆盖，无需独立 macd_epoch）。
    pub(super) macd_state_len: usize,
    /// #106：L0 走势塔增量缓存（前缀稳定，仅 segments 末段可古怪线段重划改写）。用 parser
    /// `segments_confirmed_len` 证书复用前缀，只 map 新尾段——替代每 bar 全量 `from_unit` 重建
    /// （O(segs)×n = O(n²)，profile 坐实 400K=5.8s）。bit-exact：from_unit 只依赖单 seg（无相邻
    /// 依赖，codex 确认），前缀稳定 ⟹ moves_tower_l0 前缀稳定；ordinal=全局索引（reuse+i）跨 bar 稳定。
    pub(super) moves_tower_l0: Rc<Vec<LeveledMove>>,
    /// #106：L0 输入单元（segment_to_unit 投影）增量缓存——同 moves_tower_l0 证书复用，消除每 bar
    /// 全量 `l0.segments.map(segment_to_unit).collect()`（O(segs)×n，profile 坐实 400K=0.6s）。
    /// segment_to_unit 只依赖单 seg ⟹ 前缀稳定 bit-exact。
    /// ★[H4] Rc 化（同 moves_tower_l0 先例）：消除每 bar 全量 `.clone()`（O(segments)×n=O(n²)，
    /// profile 坐实 400K=380ms）——消费点 `Rc::clone` O(1)，`units` 循环变量统一为 `Rc<Vec>`
    /// （L0=此缓存 Rc::clone，L1+=投影 Rc::new）。units 全程只读不原地改（仅整体重赋值）⟹ bit-exact。
    pub(super) l0_units_cache: Rc<Vec<UnitRange>>,
    /// merged_bars.close 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写）。每 bar mem::take 出借
    /// 给 BSP/MACD，用毕放回——避免每 bar 全量 `.map().collect()` 重建（O(n)/bar → O(n²) 根因）。
    pub(super) closes: Vec<f64>,
    /// merged_bars.source_index 增量缓存（与 `closes` 同步，坐标系映射用）。
    pub(super) close_src: Vec<usize>,
    /// merged_bars.close 的 **Tick（整数）** 增量缓存（与 `closes` 同步，force 价格振幅/速度 proxy 用）。
    /// `closes_tick[i] == merged_bars[i].close`（Tick 本身，非 f64 往返）——与 `extract_signals_force`
    /// 的 `closes as f64 as Tick` 逐值一致（整值 Tick 往返 f64 精确）。热路径 mem::take 出借，用毕放回。
    pub(super) closes_tick: Vec<Tick>,
    /// ★工位 4g：塔变更代次（generation）——下游 [`super::strategy::interp::TreeCache`] 用其 O(1) 判断
    /// 是否复用缓存树，**跳过每 bar O(tree) 的 `TreeKey::of(tower)` 全量重算**（exp≈2.0 真因）。
    ///
    /// **单调递增**，仅当 `extract_elements(tower)` **可观察输出可能变化**时 +1。维护点（codex 异质审
    /// soundness 全覆盖）：(a) 任一级 `cascade_reset`（frontier 改写/回缩传播至最高级重扫）；(b) 任一级
    /// `upper_moves` extend 非空 tail（含新级涌现首产 + 最高级 append）；(c) `clear()`（全量重扫）。
    ///
    /// soundness 充要（codex Q3）：generation 绑定"可观察树变更"非指针/len。低级 extend 不影响最高级
    /// 输出时**过度** +1（保守——多算一次 TreeKey，绝不假命中），sound 安全。`extract_elements` 只读最高
    /// 非空级，但 `sub_moves: Vec<LeveledMove>` 值拷贝（compose 时 `subs.to_vec()`）⟹ 低级静默变异必经
    /// cascade 重建父级才更新副本（codex Q1 确认无静默路径）。
    pub(super) generation: u64,
    /// ★forest_epoch（on2w2：K_i 判据，独立于 `generation`）——下游 [`super::strategy::interp::TreeCache`]
    /// 用其 O(1) 命中判断 `extract_carrier_forest(tower)`（K_i，读**全塔含 L0**）是否可复用，取代每 bar
    /// O(全塔) 的 [`super::strategy::interp::TreeKey::of_forest`] 指纹（H6 O(n²) 真因）。
    ///
    /// **与 `generation` 的本质区别**（不复用的理由，见 on2w2-epoch-design §2/§3）：`generation` 服务
    /// T_i（`extract_elements`，只读最高非空级），靠 L0→cascade→L1 **间接**传播 + `l0_is_root` 每-bar
    /// blunt 兜底覆盖 L0，bump 率 98.5%（过度 124×，无法修 K_i 的 O(n²)）。forest_epoch 在**塔实际发生
    /// 字节变更的写入站点直接 ++**（E1 L0 重建 / E2 upper extend / E2a frontier pop / E3 cascade clear /
    /// E4 全量 clear），bump 率贴近 forest 真变率（≈0.8%），且 E1 直接捕获 L0 变更（覆盖 L0-root ∧
    /// L0-尾段-重划-under-L1，无需 blunt 兜底）。
    ///
    /// **单调递增**，永不 reset（含 E4 clear 也是 ++，与 `generation` 同规格——下游缓存旧 epoch 可能恰
    /// 为 0 ⟹ reset 会造假命中）。**over-invalidate**：写入站点无条件 ++（即便重扫复现相同字节），宁可
    /// 多失效不可假命中。假命中不可能性证明见 on2w2-epoch-design §4。
    pub(super) forest_epoch: u64,
    /// B3 #4 area-memo：[`AreaCache`]（见其文档）——`sublevel_diverges` 的 `(start,end)→area`
    /// 冻结缓存，跨 bar 持久。
    pub(super) area_cache: AreaCache,
    /// #550：跨 bar 持久的候选事件簿；首见钟与 revision 只能由逐前缀推进产生。
    pub(super) candidate_book: cand_event::CandidateEventBook,
}

impl TowerCache {
    /// 构造空缓存（首次调用全量扫，之后增量复用）。
    pub fn new() -> Self {
        Self::default()
    }

    /// ★工位 4g：当前塔变更代次（下游 `TreeCache` O(1) 命中判据）。同代次 ⟹ `extract_elements`
    /// 输出逐字节不变（soundness 见 `generation` 字段文档）。
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// ★on2w2：当前塔森林代次（`forest_epoch`）——下游 [`super::strategy::interp::TreeCache`] 用其 O(1)
    /// 判断 `extract_carrier_forest(tower)`（K_i）是否可复用，取代每 bar O(全塔) 的 `TreeKey::of_forest`。
    /// 同代次 ⟹ K_i 森林输出逐字节不变（soundness 见 `forest_epoch` 字段文档 + on2w2-epoch-design §4）。
    pub fn forest_epoch(&self) -> u64 {
        self.forest_epoch
    }

    /// #641：跨 bar 因果候选事件簿的**只读**快照（与 [`classify_with_tower_events_incremental`]
    /// 返回的第三元同一来源，同一 `Rc`）。
    ///
    /// 加这个取数口是为了让已经调 [`classify_with_tower_incremental`] 的既有驱动（p123）能在
    /// **不改既有调用点、不改既有返回值**的前提下读到事件流。只读、不改簿、零行为影响。
    pub fn candidate_streams(&self) -> cand_event::CandidateStreams {
        self.candidate_book.streams()
    }

    /// ★#93 水线证书（单一来源，禁第二查法）：`tower[level][..w]` **跨 bar bit-stable 下界**。
    ///
    /// 语义（与 `classify_with_tower_incremental` 返回的 `tower_snapshots` 同下标）：
    /// - `level == 0`：= `l0.segments_confirmed_len`（parser 证书，tower[0][i] 为 segments[i]
    ///   纯函数 ⟹ 前缀稳定同传）；
    /// - `level >= 1`：`tower[level]` 与 `levels[level-1].upper_moves` 共享 Rc ⟹
    ///   = `levels[level-1].confirmed_watermark`（维护站点见该字段文档）；
    /// - 越界级（本 bar 未产出缓存）⟹ 0（保守：视为无稳定前缀）。
    ///
    /// over-shrink 恒 sound（消费方多重比）；over-grow 禁止（w 内元素跨 bar 必须逐字节不变）。
    pub fn tower_confirmed_len(&self, level: usize) -> usize {
        if level == 0 {
            self.l0_confirmed_len
        } else {
            self.levels
                .get(level - 1)
                .map_or(0, |lc| lc.confirmed_watermark)
        }
    }

    // ── #614 并线：kimi 线 `tower_cache.rs` 的三个只读窗口/水位访问器 ────────────────
    // 本合并树取 main 线的内联 `TowerCache`（见 #614 合并报告 ⚠HIGH-2），kimi 线把这三个
    // 访问器放在其拆分出的 `classifier/tower_cache.rs`。消费方 `src/bin/p123_fast_replay.rs`
    // （kimi 线 #421/#527/#601 PanLive 探针，本合并树取 kimi 版）逐字调用它们。
    // 三者**全部只读**，不参与任何分类/交易/订单/风控分支。⚠待人工复核：#614 手工重放。
    //
    // ★#712 收 #645 LOW-1：三者对同一个 `level` 实参的下标口径互差 1（易错面，非错——kimi
    // 侧原样口径，`tower_cache.rs:314-330` 长文档承担互指角色，本合并树未随入长文档，故在
    // 此互指一句）：`freeze_boundary(level)` 直读 `levels.get(level)`；`level_scan_cursor(level)`
    // 读 `levels.get(level-1)`（`tower[level]` 由 `levels[level-1]` 扫描产出）；
    // `level_scan_units(level)` 读 `levels.get(level-2)`（level==1 特例为 `l0_units_cache`，
    // 即「产出 `tower[level-1]` 的那一级窗口扫描输入」）。三者同放一个 impl 块相邻位置，
    // 调用前须按各自文档核对 `level` 实参，不可假设三者同口径。

    /// 本级窗口扫描游标（level≥1；level 0 无上级窗口 ⟹ `None`）。
    ///
    /// `levels[level-1]` 的对应关系与 `level_scan_units` 同源（`levels` 下标 = 上级层号-1）。
    pub fn level_scan_cursor(&self, level: usize) -> Option<WindowScanCursor> {
        if level == 0 {
            return None;
        }
        self.levels.get(level - 1).map(|lc| lc.scan_cursor)
    }

    /// 本级窗口扫描的**输入** units 只读切片（level≥1；level 0 无上级窗口 ⟹ `None`）。
    ///
    /// level 1 的输入 = L0 单元投影（`l0_units_cache`，与 `tower[0]` 同序同长同源）；
    /// level≥2 的输入 = 下一级的 `projected_units`。只读切片，调用方不得跨下一次增量调用持有。
    pub fn level_scan_units(&self, level: usize) -> Option<&[UnitRange]> {
        match level {
            0 => None,
            1 => Some(&self.l0_units_cache[..]),
            _ => self.levels.get(level - 2).map(|lc| &lc.projected_units[..]),
        }
    }

    /// #69 5b：classifier level 最近一次使用的 e_src（source_index 量纲）。
    ///
    /// p123 的 target level `L` 以 `L-1` 的 lower legs 判 pan，因此读取 `freeze_boundary(L-1)`。
    /// 缺级返回 `None`；调用方须按稳定集为空处理，禁止猜值。
    pub fn freeze_boundary(&self, level: usize) -> Option<usize> {
        self.levels.get(level).map(|lc| lc.last_freeze_boundary)
    }

    /// #92 因果 prefix provider 的只读 MACD/坐标快照。
    ///
    /// 两个切片与当前 [`classify_with_tower_incremental`] 返回值同源、同 prefix；调用方只读，
    /// 不得跨下一次增量调用持有。该入口避免 replay 从终态序列回填或另跑第二套 MACD。
    pub fn causal_series(&self) -> (&[f64], &[usize]) {
        (&self.macd_hist, &self.close_src)
    }

    /// 当前增量产出的 MACD dif 前缀（黄白线，force_state 生产热路由输入；bit-exact 等价全量
    /// `compute_macd(closes).dif`）。与 [`Self::macd_hist_for_test`] 逐 bar 锁步、等长。
    pub fn macd_dif(&self) -> &[f64] {
        &self.macd_dif
    }

    /// 当前增量产出的 MACD hist 前缀（对拍锚：dif 增量正确性由 `dif==全量` ∧ `hist==全量` 双证）。
    pub fn macd_hist_for_test(&self) -> &[f64] {
        &self.macd_hist
    }

    /// 当前增量产出的 close(Tick) 前缀（force 价格振幅/速度 proxy 输入；`== merged_bars.close`）。
    pub fn closes_tick(&self) -> &[Tick] {
        &self.closes_tick
    }

    /// 重置可重导塔缓存（退化为下次全量重扫），但保留不可回填的候选因果事件簿。
    ///
    /// 调用时机：段账本前缀非单调追加（回缩/改写）、或 config 变更、或 merged_bars
    /// 前缀改写（inclusion 合并回退改写尾部）。
    pub fn clear(&mut self) {
        self.levels.clear();
        self.last_l0_segments_len = 0;
        self.l0_confirmed_len = 0; // ★#93：水线随 clear 归零（消费方保守全量重比）。
        self.macd_state = None;
        self.macd_hist.clear();
        self.macd_dif.clear(); // 与 macd_hist 锁步（同 truncate/rebuild 边界，见 compute_macd_hist_incremental）。
        self.macd_state_len = 0;
        Rc::make_mut(&mut self.moves_tower_l0).clear();
        Rc::make_mut(&mut self.l0_units_cache).clear(); // ★H4：Rc<Vec>，make_mut 后清（同 moves_tower_l0）。
        self.closes.clear();
        self.close_src.clear();
        self.closes_tick.clear(); // 与 closes 锁步（同 update_closes_cache 前缀复用）。
        self.area_cache.clear(); // hist 全量重扫 ⟹ 旧 (start,end)→area 键值可能不再对应新 hist。
                                 // ★工位 4g：clear=全量重扫 ⟹ extract 输出会变 ⟹ +generation（不 reset 为 0——下游缓存旧
                                 // generation 可能恰为 0 ⟹ 假命中复用陈旧树）。单调递增保 sound。
        self.generation += 1;
        // ★on2w2 E4：clear = moves_tower_l0.clear（tower[0] 字节变更）⟹ forest 变 ⟹ +forest_epoch
        // （不 reset，同 generation 规格）。clear() 是 public 方法，此 bump 独立于增量循环的 forest_dirty
        // 折叠——一个 bar 若既 clear() 又走增量循环可能 ++2，多失效恒 sound（见 on2w2-epoch-design §5.1）。
        self.forest_epoch += 1;
    }
}

/// 增量 MACD hist 计算（231号纯性能，bit-exact 铁律）。
///
/// 返回完整 hist 序列（等长 `closes`），与 `divergence::compute_macd(closes, cfg).hist`
/// 逐元素 bit-identical（ac75d4b3 已证 `MacdState::append` bit-exact）。
///
/// ## per-bar substrate 语义（inclusion 合并的尾部不稳定性）
///
/// `parse_layer(&bars[..i])` 的 `merged_bars` = `merged_prefix`（confirmed 稳定前缀）+
/// `acc`（当前未定稿合并段，可能被后续 bar 吸收改写）。故跨 bar：
/// - `merged_bars[..len-1]` 稳定（confirmed 前缀不动）
/// - `merged_bars[len-1]`（= acc）可能改写
///
/// 增量 state 表示 `closes[..stable_prefix]`（stable_prefix = `closes.len() - 1`，排除
/// 不稳定的尾 bar）。尾 bar 的 hist 由 state + 尾 close O(1) 派生。每 bar 追加：
/// - 长度相同（inclusion 吸收）：stable_prefix 不变 ⟹ state 有效，尾 bar hist 重算。
/// - 长度 +1（非包含定稿）：旧尾 bar 现稳定 ⟹ state append 旧尾 close，新尾 bar hist 派生。
///
/// ## 退化路径（cache 空 / 前缀改写 / 长度回缩）
///
/// 逐 bar `append` 重建 state + hist 数组（与全量同 EMA 约简顺序，bit-exact）。O(n) 单次，
/// 非 fallback——是 inclusion 回退的合法 bit-exact 退化（同塔 cache clear 逻辑）。
///
/// ## 认识论（formalization-validity-domain 231号）
///
/// L1：bit-exact 等价于全量 `compute_macd`（管线正确性，零 alpha 信息增量）。
/// 增量只把同一浮点约简从「全量重算」改成「逐 bar 延伸」，数值不变。
/// closes/close_src 增量缓存更新（231号纯性能，bit-exact 铁律）。
///
/// 维护 `cache.closes == merged_bars.iter().map(|b| b.close as f64).collect()` 与
/// `cache.close_src == merged_bars.iter().map(|b| b.source_index).collect()`，逐元素 bit-identical。
///
/// ## per-bar substrate 语义（inclusion 尾部不稳定性，同 `compute_macd_hist_incremental`）
///
/// `merged_bars` = confirmed 稳定前缀 + 当前未定稿合并段 `acc`（尾元素，可能被后续 bar 吸收改写）。
/// 故 `merged_bars[..len-1]` 跨 bar 稳定（confirmed 前缀不动），仅 `merged_bars[len-1]`（acc）可能改写。
/// ⟹ 缓存只需：截掉尾元素（重算的边界），从稳定前缀末续推 close/source_index。
///
/// - 长度 +k（新 bar 定稿）：旧尾现稳定 ⟹ 续 push 新元素。
/// - 长度不变（inclusion 吸收）：尾元素可能改写 ⟹ 截尾重 push。
/// - 长度回缩 / 前缀改写：全量重建（bit-exact 退化，同 cache clear 逻辑）。
pub(super) fn update_closes_cache(
    merged_bars: &[super::super::types::Bar],
    confirmed_len: usize,
    cache: &mut TowerCache,
) {
    let n = merged_bars.len();
    let cached = cache.closes.len();

    // ★#106 O(1) 证书路径（替代旧每 bar O(n) 前缀全量比较 = O(n²) 主导根因，profile 坐实
    // 200K=15.3s/67%）：parser `merged_confirmed_len` 保证 `merged_bars[..confirmed_len]` 跨 bar
    // **物理不变**（相 B append_folded 仅 Rc::make_mut pop/push 末根，前缀字节不动；codex 锚定）。
    // ⟹ `cache.closes[..confirmed_len]` 与 `merged_bars[..confirmed_len]` 逐值一致（close + source_index
    // 双稳定，codex 3a/3b 满足——同一 Bar 物理不变两字段都不变），无需 O(n) 比较。
    //
    // bit-exact 前提（codex 血缘）：cache 由 classifier 每 bar 同源维护（生产路径每 bar 调 classify_at），
    // confirmed_len 单调 ⟹ cache[..confirmed_len] 是上轮前缀。前提失守（cached < confirmed_len，
    // 跨 bar 漏调 / 血缘断裂）⟹ reuse = min(cached, confirmed_len)，余下全量重扫（保守 bit-exact）。
    // confirmed_len=0（相 A / 相 A→B fold_all 整段重写 / 全量 parse_layer 无血缘）⟹ reuse=0 = 全量重建。
    let reuse = confirmed_len
        .min(cached)
        .min(cache.close_src.len())
        .min(cache.closes_tick.len());
    cache.closes.truncate(reuse);
    cache.close_src.truncate(reuse);
    cache.closes_tick.truncate(reuse);
    cache.closes.reserve(n.saturating_sub(reuse));
    cache.close_src.reserve(n.saturating_sub(reuse));
    cache.closes_tick.reserve(n.saturating_sub(reuse));
    for b in &merged_bars[reuse..] {
        cache.closes.push(b.close as f64);
        cache.close_src.push(b.source_index);
        cache.closes_tick.push(b.close); // Tick 本身（整数），force 价格振幅/速度 proxy 用（与 closes 锁步）。
    }
}

pub(super) fn compute_macd_hist_incremental(
    closes: &[f64],
    confirmed_len: usize,
    cfg: &super::super::config::MacdConfig,
    cache: &mut TowerCache,
) {
    // 空 closes：无 MACD 可算，清空 cache MACD 域（防御性，classify 入口已防空 layer）。
    if closes.is_empty() {
        cache.macd_state = None;
        cache.macd_hist.clear();
        cache.macd_dif.clear();
        cache.macd_state_len = 0;
        return;
    }

    // 单 bar：state = init(closes[0])，hist = [0.0]（首 bar DIF=DEA=hist=0）。state 覆盖 closes[..1]。
    if closes.len() == 1 {
        let state = MacdState::init(closes[0], cfg);
        let p = state.current_point();
        cache.macd_hist = vec![p.hist];
        cache.macd_dif = vec![p.dif]; // 首 bar DIF=0（close-close），与 hist 同点派生。
        cache.macd_state = Some(state);
        cache.macd_state_len = 1;
        return;
    }

    // 稳定前缀长度（排除不稳定的尾 bar）。state 续推目标 = closes[..stable_prefix]。
    let stable_prefix = closes.len() - 1;

    // ★#106 O(1) 证书增量（替代旧每 bar O(n) 前缀比较 + `closes.to_vec()` 全量克隆 = O(n²)，
    // profile 坐实 200K=4.7s/21%）：复用边界 = `min(macd_state_len, confirmed_len)`。
    // - `macd_state_len`：state 已消费的 close 数（state 表示 closes[..macd_state_len] 累积）。
    // - `confirmed_len`：parser 证书——closes[..confirmed_len] 跨 bar 物理不变（前缀未被 inclusion
    //   改写）。取 min ⟹ state 覆盖的部分**全在稳定前缀内** ⟹ 续推不被改写污染（codex 血缘修复：
    //   绑 state_len 非 cached_len-1）。
    // resume_from > stable_prefix 不可能（resume_from <= macd_state_len <= 上轮 stable < 本轮 stable）；
    // 但 confirmed_len 收缩（相 A→B fold_all=0）⟹ resume_from=0 ⟹ 从头全量重推（bit-exact 退化）。
    let resume_from = cache.macd_state_len.min(confirmed_len).min(stable_prefix);

    // macd_dif 与 macd_hist **逐 bar 锁步**：同一 truncate/clear/push 边界，同一 `current_point` 派生
    // （dif 是 hist 的子表达式）。任一分支对 hist 的操作都对 dif 做同样操作 ⟹ len 恒等、bit-exact。
    let mut state = if resume_from > 0 && cache.macd_state.is_some() {
        // 增量：从 resume_from 的 state 续推。需要 state 恰好表示 closes[..resume_from]——
        // 若 macd_state_len > resume_from（confirmed_len 收缩截断），state 比 resume_from 多消费了
        // 已失效的 close ⟹ 不能直接用，须从头重推。故仅 macd_state_len == resume_from 时复用。
        if cache.macd_state_len == resume_from {
            cache.macd_hist.truncate(resume_from);
            cache.macd_dif.truncate(resume_from);
            cache.macd_state.clone().expect("is_some 已判")
        } else {
            cache.macd_hist.clear();
            cache.macd_dif.clear();
            rebuild_macd_state_to(
                closes,
                resume_from,
                cfg,
                &mut cache.macd_hist,
                &mut cache.macd_dif,
            )
        }
    } else {
        // 全量重建（resume_from=0 或 state 空）。
        cache.macd_hist.clear();
        cache.macd_dif.clear();
        rebuild_macd_state_to(closes, 0, cfg, &mut cache.macd_hist, &mut cache.macd_dif)
    };

    // 续推 closes[hist.len()..stable_prefix]（新稳定 bar）+ 尾 bar（不稳定）hist/dif。
    for &c in &closes[cache.macd_hist.len()..stable_prefix] {
        state = divergence::compute_macd_append(&state, c);
        let p = state.current_point();
        cache.macd_hist.push(p.hist);
        cache.macd_dif.push(p.dif);
    }
    let tail_state = divergence::compute_macd_append(&state, closes[stable_prefix]);
    let tail_p = tail_state.current_point();
    cache.macd_hist.push(tail_p.hist);
    cache.macd_dif.push(tail_p.dif);
    cache.macd_state = Some(state);
    cache.macd_state_len = stable_prefix;
}

/// MACD state 重建到 `closes[..target]`（target=0 ⟹ init(closes[0])，state_len=1）。
/// `hist`/`dif` 被 push 至 len==max(target,1)（首 bar hist=dif=0 + 续 bar），逐 bar 同点派生锁步。
/// 返回 closes[..hist.len()] 的 state。bit-exact：与全量 `compute_macd` 同 EMA 约简（逐 bar append）。
fn rebuild_macd_state_to(
    closes: &[f64],
    target: usize,
    cfg: &super::super::config::MacdConfig,
    hist: &mut Vec<f64>,
    dif: &mut Vec<f64>,
) -> MacdState {
    let mut state = MacdState::init(closes[0], cfg);
    let p0 = state.current_point();
    hist.push(p0.hist);
    dif.push(p0.dif);
    let end = target.max(1);
    for &c in &closes[1..end] {
        state = divergence::compute_macd_append(&state, c);
        let p = state.current_point();
        hist.push(p.hist);
        dif.push(p.dif);
    }
    state
}
