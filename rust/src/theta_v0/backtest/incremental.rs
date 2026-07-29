//! **增量 substrate orchestration**（per-bar 增量塔接入——跨 bar 身份稳定 + bit-exact）。
//!
//! ## 任务与结论
//!
//! 把 `run_theta_v0_pi` 的 substrate 从 per-bar 全量重分类（`classify_with_tower(parse_layer(bars[..=i]))`，
//! 实测 exp≈2.64，且每 bar 从零重建塔→跨 bar 身份断裂→held_leg 判 Stale→depth>0 腿被 AncOK 剪→
//! #5 多声部贡献为零）切换到**增量塔链**：
//!
//! 1. **增量 parse**：`ParseLayerIncr::append(bars[i])`（inclusion O(1)/bar + 下游 O(merged_i)）
//! 2. **增量塔**：`classify_with_tower_incremental(&l0_i, config, &mut tower_cache)`——
//!    `TowerCache` 跨 bar 复用（LevelCache.upper_moves/centers/scan_cursor 持久）→ **跨 bar 身份稳定**
//!
//! ## 跨 bar 身份稳定（核心修复——memory newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass）
//!
//! ★codex Q4 发现 B 归因修正：旧注释"λ/eps 漂移"归因错误——λ=start_index 在 confirmed 前缀不回写时
//! 不变。真因 = extract_elements 每 bar 重建 Vec + 更高级新出现时根结构重构索引重映射 + 旧 held_leg_tree_index
//! 值比较 (level,λ,eps) 非 spec §13 结构映射 p(g)。Q4 修复：确定性 ElementId 跨 bar 稳定（全量/增量
//! 产同 ID），held_leg_tree_index 按 ID 匹配非值比较；Stale 不伪造 parent:None（发现 A），非边界根
//! 父未解析 = prune（AncOK 严格 §13）⟹ depth>0 腿可准入 ⟹ ΔSharpe 可非零（待 L2 重测）。
//!
//! 全量重分类每 bar 从零重建塔→旧值比较 held_leg_tree_index 在新塔里找不到旧腿→判 Stale（90%+）
//! →`depth>0` 对冲腿全被 §13 AncOK 剪→#5 贡献为零→ΔSharpe=0。
//!
//! 增量塔的 `TowerCache.levels[k].upper_moves` 跨 bar **复用同一 Vec**（前缀不可变，尾部 append）→
//! `LeveledMove` 对象身份跨 bar 连续→Q4 确定性 ElementId 跨 bar 稳定→held_leg 按 ID 在新塔里找到
//! 同身份腿→Stale 降根减少→depth>0 腿准入→ΔSharpe 可非零。
//!
//! ## bit-exact 铁律
//!
//! 增量链产出的 `(Classification, tower)` 必须 == 全量 `classify_with_tower(parse_layer(bars[..=i]))`。
//! - `ParseLayerIncr::append` bit-exact == `parse_layer`（parser/mod.rs 逐 bar 断言验证）
//! - `classify_with_tower_incremental` bit-exact == `classify_with_tower`（classifier/mod.rs
//!   `incremental_tower_fresh_cache_equals_full` + `incremental_tower_per_segment_append_matches_full`）
//!
//! 接入后 runner 产出的订单/信号与全量版本的差异仅来自**身份稳定的预期改变**（Stale 减少→depth>0 腿
//! 准入→订单可能增多）——这是**预期非 bit-exact**，非增量 bug。
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! - 增量链 bit-exact = **L1**（管线正确性，零信息增量）
//! - 身份稳定→Stale 降根→ΔSharpe 可非零 = **L2 假设**（须真实数据否证/确认）

use super::super::config::ThetaConfig;
use super::super::types::Bar;
use super::super::{classifier, parser};

/// 增量分类器状态（跨 bar 持久）——增量 parse + 增量塔，**跨 bar 身份稳定**。
///
/// 每 bar 喂 [`classify_at(i)`](IncrementalClassifier::classify_at)：
/// - `ParseLayerIncr::append(bars[i])`（增量 inclusion O(1) + 下游重算 O(merged_i)）
/// - `classify_with_tower_incremental(&l0_i, config, &mut tower_cache)`（增量塔：前级 confirmed
///   前缀缓存 + 尾部续扫 + MACD 增量递推）
///
/// **TowerCache 跨 bar 复用**是身份稳定的根：`levels[k].upper_moves` 是同一 Vec 跨 bar append，
/// `LeveledMove` 对象身份连续——held_leg 在新塔里找到同身份腿，不判 Stale。
///
/// **bit-exact**：增量链 == 全量 `classify_with_tower(parse_layer(bars[..=i]))`（parser + classifier
/// 各自 bit-exact 已证，见模块文档）。
pub struct IncrementalClassifier<'a> {
    bars: &'a [Bar],
    config: &'a ThetaConfig,
    /// 增量 parser 状态（跨 bar append，inclusion O(1)/bar）。
    parser_incr: parser::ParseLayerIncr<'a>,
    /// 增量塔缓存（跨 bar 复用——身份稳定的根）。
    tower_cache: classifier::TowerCache,
}

impl<'a> IncrementalClassifier<'a> {
    /// 构造增量分类器（空缓存，首个 bar 从零起步）。
    pub fn new(bars: &'a [Bar], config: &'a ThetaConfig) -> Self {
        Self {
            bars,
            config,
            parser_incr: parser::ParseLayerIncr::new(config),
            tower_cache: classifier::TowerCache::new(),
        }
    }

    /// **per-bar 增量重分类（身份稳定）**：返回 bar i 的
    /// `(classification, tower)` == `classify_with_tower(parse_layer(&bars[..=i]))`，bit-exact。
    ///
    /// 增量链：`ParseLayerIncr::append(bars[i])` → `classify_with_tower_incremental(.., &mut cache)`。
    /// `tower_cache` 跨 bar 复用 → `LeveledMove` 身份连续 → held_leg 不判 Stale。
    ///
    /// **因果性**：`parse_layer(&bars[..=i])` 只用 ≤i 数据 ⟹ 输出因果（无 look-ahead，639）。
    pub fn classify_at_with_l0(&mut self, i: usize) -> (parser::ParseLayer, classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>) {
        debug_assert!(i < self.bars.len(), "classify_at({i}) 越界 bars.len={}", self.bars.len());
        // 增量 parse：append bar i（O(1) inclusion + O(merged_i) 下游）。
        let l0_i = self.parser_incr.append(self.bars[i]);
        // 增量塔：cache 跨 bar 复用（身份稳定），bit-exact == 全量 classify_with_tower。
        let (classification, tower) = classifier::classify_with_tower_incremental(&l0_i, self.config, &mut self.tower_cache);
        (l0_i, classification, tower)
    }

    pub fn classify_at(&mut self, i: usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>) {
        let (_, classification, tower) = self.classify_at_with_l0(i);
        (classification, tower)
    }

    /// strict-nest sidecar 只读复用 tower cache 中的增量 MACD/close 序列，避免开关打开后退回 O(n²)。
    pub fn tower_cache(&self) -> &classifier::TowerCache {
        &self.tower_cache
    }

    /// ★#93 水线证书批量读法（`classify_at(i)` 后同 bar 调用，单一来源禁第二查法）：
    /// `out[ℓ]` = `tower[ℓ][..out[ℓ]]` 跨 bar bit-stable 下界 =
    /// [`TowerCache::tower_confirmed_len(ℓ)`](classifier::TowerCache::tower_confirmed_len)。
    /// `n_levels` 取本 bar `tower.len()`；over-shrink 恒 sound（消费方多重比尾段）。
    pub fn tower_confirmed_lens(&self, n_levels: usize) -> Vec<usize> {
        (0..n_levels)
            .map(|l| self.tower_cache.tower_confirmed_len(l))
            .collect()
    }

    /// ★工位 4g：当前塔变更代次（`classify_at` 后读取）。下游 `TreeCache` 据此 O(1) 判断是否复用
    /// 缓存树，跳过每 bar O(tree) 的 `TreeKey::of`（exp≈2.0 真因）。同代次 ⟹ extract 输出不变。
    pub fn tower_generation(&self) -> u64 {
        self.tower_cache.generation()
    }

    /// ★on2w2：当前塔森林代次（`forest_epoch`）——下游 `TreeCache` 据此 O(1) 判断是否复用 K_i 森林
    /// （`extract_carrier_forest`，读全塔含 L0），跳过每 bar O(全塔) 的 `TreeKey::of_forest`（H6 O(n²)
    /// 真因）。同代次 ⟹ 森林逐字节不变（soundness 见 `TowerCache::forest_epoch` 字段文档）。
    pub fn forest_epoch(&self) -> u64 {
        self.tower_cache.forest_epoch()
    }
}

/// #345 自持缓冲区变体：见 [`classifier::streaming::OwnedIncrementalClassifier`]（无条件
/// 编译模块——`nautilus::strategy::ThetaCore`〈生产/实盘路径〉是其唯一消费方，`nautilus`
/// 模块不受本 `backtest` 模块的 `any(test, feature = "backtest_bin")` 门控约束，故该类型
/// **不能**定义在此处，只能重导出供本文件测试引用）。
pub use classifier::streaming::OwnedIncrementalClassifier;

// ════════════════════════════════════════════════════════════════════════════
// 测试 + profile（cfg(test) 门控，backtest 整模块本就 cfg(test)，此处显式标注）
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// **★bit-exact 合成数据验证（`OwnedIncrementalClassifier`，#345 always-run）**：
    /// 逐 bar 断言自持缓冲区变体 `append_bar` 输出 == legacy 全量
    /// `classify_with_tower(parse_layer(&bars[..=i]))`，bit-identical。
    ///
    /// 与既有 `bit_exact_synthetic`（`IncrementalClassifier<'a>`，借用切片变体）互补：
    /// 同一合成序列，验证**两种所有权模型**共享的 `parser::append_incr_layer` 步进
    /// 产同一 bit-exact 结果——非平行第二份实现。
    #[test]
    fn owned_bit_exact_synthetic() {
        let bars: Vec<Bar> = (0..2000usize)
            .map(|i| {
                let base = 1000i64 + (i as i64) * 2;
                // 振幅/周期须使 cycle 的逐 bar 斜率能超过 base 斜率（2/bar）且波形非单一频率
                // （否则特征序列过于规则，线段划分状态机——67课——永不出顶底分型，segments 恒空）：
                // 双频叠加（主周期 22 + 短周期 6）打破规则性，实测末 bar strokes=85 / segments=8。
                let cycle = ((((i as f64) / 22.0).sin() * 60.0) + (((i as f64) / 6.0).sin() * 25.0)) as i64;
                let close = base + cycle;
                Bar {
                    source_index: i,
                    timestamp: i as i64,
                    open: close - 1,
                    high: close + 5,
                    low: close - 5,
                    close,
                    volume: 1000,
                    untradable: false,
                }
            })
            .collect();

        let config = ThetaConfig::default();
        let mut owned = OwnedIncrementalClassifier::new(config.clone());
        let mut final_strokes: usize = 0;
        let mut final_segments: usize = 0;
        for i in 0..bars.len() {
            let (owned_cls, owned_tower) = owned.append_bar(bars[i]);

            // legacy 对照：每 bar 从头全量重跑（非增量，ground truth）。
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (leg_cls, leg_tower) = classifier::classify_with_tower(&l0, &config);
            assert_eq!(owned_cls, leg_cls, "owned synthetic bar {i}: classification bit-exact 破裂");
            assert_eq!(owned_tower.len(), leg_tower.len(), "owned synthetic bar {i}: tower 层数破裂");
            for (lvl, (ol, ll)) in owned_tower.iter().zip(leg_tower.iter()).enumerate() {
                assert_eq!(ol, ll, "owned synthetic bar {i} lvl {lvl}: tower 级 LeveledMove 破裂");
            }
            if i == bars.len() - 1 {
                final_strokes = l0.strokes.len();
                final_segments = l0.segments.len();
            }
        }
        // #346 MED-1：合成序列必须产生非退化结构（笔/段非空），否则 bit-exact 对照的是两条退化的
        // 单调直线空结构（`.sin() as i64` 优先级截断使 cycle 恒为 0）——本断言防止该退化复发。
        assert!(
            final_strokes > 1 && final_segments >= 1,
            "owned synthetic: 末 bar strokes={final_strokes} segments={final_segments}，\
             合成序列疑似退化为单调直线（无笔/段结构）"
        );
        eprintln!(
            "\n===== owned bit-exact 合成验证通过：{} bars，末 bar strokes={final_strokes} segments={final_segments} =====\n  \
             OwnedIncrementalClassifier::append_bar == 全量，bit-identical，非退化。",
            bars.len()
        );
    }

    /// **★两变体互证（#345）**：`OwnedIncrementalClassifier`（owned config，无生命周期）
    /// 与 `IncrementalClassifier<'a>`（借用切片，既有批量变体）在同一合成序列上逐 bar
    /// bit-exact 相等——证明"自持缓冲区"重构未改变增量算法本身，只改了所有权模型。
    #[test]
    fn owned_matches_borrowed_variant_synthetic() {
        let bars: Vec<Bar> = (0..1500usize)
            .map(|i| {
                let base = 1000i64 + (i as i64);
                let cycle = (((i as f64) / 23.0).sin() * 40.0) as i64;
                let close = base + cycle;
                Bar {
                    source_index: i,
                    timestamp: i as i64,
                    open: close - 1,
                    high: close + 6,
                    low: close - 6,
                    close,
                    volume: 1000,
                    untradable: false,
                }
            })
            .collect();

        let config = ThetaConfig::default();
        let mut owned = OwnedIncrementalClassifier::new(config.clone());
        let mut borrowed = IncrementalClassifier::new(&bars, &config);
        for i in 0..bars.len() {
            let (owned_cls, owned_tower) = owned.append_bar(bars[i]);
            let (borrowed_cls, borrowed_tower) = borrowed.classify_at(i);
            assert_eq!(owned_cls, borrowed_cls, "bar {i}: owned != borrowed classification");
            assert_eq!(owned_tower.len(), borrowed_tower.len(), "bar {i}: owned/borrowed tower 层数不同");
            for (lvl, (ol, bl)) in owned_tower.iter().zip(borrowed_tower.iter()).enumerate() {
                assert_eq!(ol, bl, "bar {i} lvl {lvl}: owned/borrowed LeveledMove 不同");
            }
        }
    }

    /// **★真实数据 bit-exact（OKLO/BTC，#345 验收要求，需数据）**：逐 bar 断言
    /// `OwnedIncrementalClassifier::append_bar` == legacy 全量重跑，OKLO + BTC 各截一段窗口。
    /// `#[ignore]`（O(n²) 双跑对照 + 需真实数据，同既有 `bit_exact_per_bar` 口径）。
    #[test]
    #[ignore = "bit-exact 验证：需 OKLO/BTC 数据；--release（O(n²) 全 bar 双跑对照）"]
    fn owned_bit_exact_per_bar_real_symbols() {
        let config = ThetaConfig::default();
        let cap: usize = std::env::var("BITEXACT_BARS").ok().and_then(|s| s.parse().ok()).unwrap_or(8_000);
        for symbol in ["OKLO", "BTC"] {
            let ds = match super::super::data::load_by_symbol(symbol, &config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("DATA BLOCKER [{symbol}]: {e}");
                    panic!("需真实数据: {symbol}");
                }
            };
            let n = cap.min(ds.bars.len());
            let bars = &ds.bars[..n];
            let mut owned = OwnedIncrementalClassifier::new(config.clone());
            let t0 = std::time::Instant::now();
            for i in 0..n {
                let (owned_cls, owned_tower) = owned.append_bar(bars[i]);
                let l0 = parser::parse_layer(&bars[..=i], &config);
                let (leg_cls, leg_tower) = classifier::classify_with_tower(&l0, &config);
                assert_eq!(owned_cls, leg_cls, "[{symbol}] bar {i}: owned classification != legacy");
                assert_eq!(owned_tower.len(), leg_tower.len(), "[{symbol}] bar {i}: tower 层数 != legacy");
                for (lvl, (ol, ll)) in owned_tower.iter().zip(leg_tower.iter()).enumerate() {
                    assert_eq!(ol, ll, "[{symbol}] bar {i} lvl {lvl}: LeveledMove != legacy");
                }
            }
            let dt = t0.elapsed().as_secs_f64();
            eprintln!("[{symbol}] owned bit-exact n={n} 通过，{dt:.1}s（双跑对照）。");
        }
    }

    /// **★bit-exact 硬指标：逐 bar 断言增量 classify_at(i) == legacy
    /// `classify_with_tower(parse_layer(&bars[..=i]))`**。
    ///
    /// 增量链（ParseLayerIncr + classify_with_tower_incremental）若非 bit-exact，此断言立即捕获。
    /// **正确性铁律**。
    #[test]
    #[ignore = "bit-exact 验证：需 CL 数据；--release（O(n²) 全 bar 双跑对照）"]
    fn bit_exact_per_bar() {
        let config = ThetaConfig::default();
        let ds = match super::super::data::load_by_symbol("CL", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        // 8K 控时（O(n²) 双跑：增量 + legacy 对照，各一倍；8K≈2s/跑，~4s 总）。env BITEXACT_BARS
        // 可上调窗口做 cascade/升级窗口/哨兵翻转深覆盖（on2w2-cascade 验证：50K 命中更多 P>0 场景）。
        let cap: usize = std::env::var("BITEXACT_BARS").ok().and_then(|s| s.parse().ok()).unwrap_or(8_000);
        let n = cap.min(oos.bars.len());
        let bars = &oos.bars[..n];

        let mut incr = IncrementalClassifier::new(bars, &config);
        let t0 = std::time::Instant::now();
        for i in 0..n {
            let (incr_cls, incr_tower) = incr.classify_at(i);
            // legacy 对照（全量重算，非增量）。
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (leg_cls, leg_tower) = classifier::classify_with_tower(&l0, &config);

            // ★逐 bar bit-exact 断言（铁律）。
            assert_eq!(
                incr_cls, leg_cls,
                "bar {i}: 增量 classification != legacy（bit-exact 破裂）"
            );
            assert_eq!(
                incr_tower.len(), leg_tower.len(),
                "bar {i}: 增量 tower 层数 != legacy"
            );
            for (lvl, (il, ll)) in incr_tower.iter().zip(leg_tower.iter()).enumerate() {
                assert_eq!(
                    il, ll,
                    "bar {i} level {lvl}: 增量 tower 级 LeveledMove != legacy（身份稳定破裂）"
                );
            }
        }
        let dt = t0.elapsed().as_secs_f64();
        eprintln!(
            "\n===== bit-exact 验证通过：n={n} bars，{dt:.1}s（双跑对照）=====\n  \
             每 bar 增量输出 == legacy classify_with_tower(parse_layer(..=i))，bit-identical。\n  \
             TowerCache 跨 bar 复用 → LeveledMove 身份连续。"
        );
    }

    /// ★on2w2-cascade 放行条件3 falsification gate（L1 管线度量）：在真实 CL 上测 cascade 事件的
    /// 脏源下界 `e` 是否常态坍缩到 units 起点（保留前缀 P==0）。设计前提 = frontier 改写局部化 ⟹
    /// e 高位 ⟹ 保留前缀比例高。若 e 常态坍缩（e0_frac≈1 / keep_frac≈0）⟹ 前提证伪 ⟹ NO-SHIP，
    /// 不值得实装 ~500 行 bit-exact 高危改动。**先于实装跑**（ponytail：falsification gate 前置）。
    ///
    /// 走 THETA_CASCADE_EPROBE 探针（不改任何失效逻辑，只测 e 分布）。手动运行：
    /// `THETA_CASCADE_EPROBE=1 cargo test --release --features backtest_bin cascade_e_falsification -- --ignored --nocapture`
    #[test]
    #[ignore = "L1 falsification gate：需 CL 数据 + THETA_CASCADE_EPROBE=1；--release"]
    fn cascade_e_falsification_gate() {
        assert!(
            std::env::var("THETA_CASCADE_EPROBE").is_ok(),
            "须设 THETA_CASCADE_EPROBE=1 启用探针（否则 cascade_events=0，无数据）"
        );
        let config = ThetaConfig::default();
        let ds = match super::super::data::load_by_symbol("CL", &config) {
            Ok(d) => d,
            Err(e) => panic!("需真实数据: {e}"),
        };
        // 300K 窗（设计 §6.3 计时验收窗；e 分布不需双跑对照 ⟹ 单跑增量即可，快）。
        let n = 300_000.min(ds.bars.len());
        let bars = &ds.bars[..n];
        classifier::oracle_probe::reset();
        let mut incr = IncrementalClassifier::new(bars, &config);
        let t0 = std::time::Instant::now();
        for i in 0..n {
            let _ = incr.classify_at(i);
        }
        let dt = t0.elapsed().as_secs_f64();
        let p = classifier::oracle_probe::snapshot();
        let events = p.cascade_events.max(1);
        let e0_frac = p.cascade_e0 as f64 / events as f64;
        let keep_frac = p.cascade_keep_ppm_sum as f64 / events as f64 / 1e6;
        eprintln!(
            "\n===== on2w2-cascade 放行条件3 falsification gate（CL n={n}, {dt:.1}s）=====\n  \
             cascade_events   = {}\n  \
             cascade_e0(P==0) = {} ({:.2}%)\n  \
             mean keep_frac(P/len, end_index<e 宽松代理) = {:.4}\n  \
             判据：e0_frac→1 且 keep_frac→0 ⟹ e 常态坍缩 ⟹ 设计前提证伪 ⟹ NO-SHIP\n\
             ==============================================================",
            p.cascade_events, p.cascade_e0, e0_frac * 100.0, keep_frac
        );
    }

    /// ★on2w2-cascade O1 always-run（无需 CL）：合成流逐 bar 断言 **增量失效路径 == legacy 全量** 逐字段
    /// bit-exact（含 cascade 分支）。平滑合成流罕触发 P>0 局部失效（frontier 古怪线段重划需非规则数据）
    /// ——P>0 分支的真实覆盖由 CL falsification gate（`cascade_e_falsification_gate`，实测 3879 事件 96%
    /// 保留）+ 150K CL `bit_exact_per_bar` 提供；本测试作 always-run 回归网（cascade 全清路径 bit-exact）。
    /// 探针数（cascade_events/keep）仅在 THETA_CASCADE_EPROBE=1 时打印（信息性，不断言——合成流无 P>0）。
    #[test]
    fn cascade_incremental_eq_full_clear_synthetic() {
        // 合成 3000 bar：趋势 + 多频回撤（产多级塔 + 频繁 frontier 古怪线段重划 ⟹ cascade）。
        let bars: Vec<Bar> = (0..3000usize)
            .map(|i| {
                let base = 1000i64 + (i as i64) * 2;
                let c = base
                    + (((i as f64) / 50.0).sin() * 30.0) as i64
                    + (((i as f64) / 13.0).sin() * 12.0) as i64;
                Bar {
                    source_index: i, timestamp: i as i64,
                    open: c - 1, high: c + 5, low: c - 5, close: c,
                    volume: 1000, untradable: false,
                }
            })
            .collect();
        let config = ThetaConfig::default();

        // 探针跑（增量 P>0 路径，THETA_CASCADE_EPROBE 需在进程级设）。此处核心：增量 == legacy 全量逐 bar。
        classifier::oracle_probe::reset();
        let mut incr = IncrementalClassifier::new(&bars, &config);
        for i in 0..bars.len() {
            let (inc_cls, inc_tower) = incr.classify_at(i);
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (leg_cls, leg_tower) = classifier::classify_with_tower(&l0, &config);
            assert_eq!(inc_cls, leg_cls, "cascade-O1 bar {i}: 增量(P>0) != 全量");
            assert_eq!(inc_tower.len(), leg_tower.len(), "cascade-O1 bar {i}: tower 层数");
            for (lvl, (il, ll)) in inc_tower.iter().zip(leg_tower.iter()).enumerate() {
                assert_eq!(il, ll, "cascade-O1 bar {i} lvl {lvl}: LeveledMove 破裂");
            }
        }
        // 探针信息性打印（仅 THETA_CASCADE_EPROBE=1）——合成平滑流通常 events=0（无古怪线段重划）。
        // P>0 局部失效的真实覆盖在 CL 门（见函数头）；此处不断言探针数，避免对合成数据形态的隐式依赖。
        if std::env::var("THETA_CASCADE_EPROBE").is_ok() {
            let p = classifier::oracle_probe::snapshot();
            eprintln!(
                "cascade-O1（合成，信息性）：events={} e0={} keep_ppm_sum={}",
                p.cascade_events, p.cascade_e0, p.cascade_keep_ppm_sum
            );
        }
    }

    /// **★bit-exact 合成数据验证（无需 CL，always-run）**：合成 bar 序列逐 bar 断言增量 == legacy。
    #[test]
    fn bit_exact_synthetic() {
        // 合成 2000 bar：缓慢上升趋势 + 周期性回撤（产足够段/中枢）。
        let bars: Vec<Bar> = (0..2000usize)
            .map(|i| {
                let base = 1000i64 + (i as i64) * 2;
                let cycle = (((i as f64) / 50.0).sin() * 30.0) as i64;
                let close = base + cycle;
                Bar {
                    source_index: i,
                    timestamp: i as i64,
                    open: close - 1,
                    high: close + 5,
                    low: close - 5,
                    close,
                    volume: 1000,
                    untradable: false,
                }
            })
            .collect();

        let config = ThetaConfig::default();
        let mut incr = IncrementalClassifier::new(&bars, &config);
        for i in 0..bars.len() {
            let (incr_cls, incr_tower) = incr.classify_at(i);

            // legacy 对照。
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (leg_cls, leg_tower) = classifier::classify_with_tower(&l0, &config);
            assert_eq!(incr_cls, leg_cls, "synthetic bar {i}: classification bit-exact 破裂");
            assert_eq!(incr_tower.len(), leg_tower.len(), "synthetic bar {i}: tower 层数破裂");
            for (lvl, (il, ll)) in incr_tower.iter().zip(leg_tower.iter()).enumerate() {
                assert_eq!(il, ll, "synthetic bar {i} lvl {lvl}: tower 级 LeveledMove 破裂");
            }
        }
        eprintln!(
            "\n===== bit-exact 合成验证通过：{} bars =====\n  \
             增量链（ParseLayerIncr + classify_with_tower_incremental）== 全量，bit-identical。",
            bars.len()
        );
    }

    /// **★#106 confirmed_len 证书 bit-exact（only_open_tail 前缀改写 + 相 A→B 迁移覆盖）**。
    ///
    /// 针对 merged_confirmed_len 证书路径：开头 N 根**互相包含**的 bar ⟹ 长时间停留相 A
    /// （only_open_tail，confirmed_len=0）；随后方向出现触发相 A→B fold_all（整段重折叠，证书仍 0）；
    /// 再后续相 B 稳态（证书 = len-1）。逐 bar 断言增量（走 confirmed_len 路径）== 全量（无证书，
    /// 每 bar 从头重算）。若证书在 only_open_tail / 迁移边界给错值（非 0），update_closes/macd 会
    /// 错误复用陈旧前缀 ⟹ classification bit-exact 破裂，此测试捕获。
    #[test]
    fn bit_exact_confirmed_len_open_tail() {
        // 开头 12 根逐步收窄的互相包含 bar（无非包含严格对 ⟹ 相 A only_open_tail）；
        // 随后突破上沿引入方向（相 A→B fold_all）；再正弦波动产段/中枢（相 B 稳态 + 古怪线段重划）。
        let mut bars: Vec<Bar> = Vec::new();
        for i in 0..12usize {
            // 区间逐根收窄（后包含于前）⟹ 持续包含，无方向。
            let half = 50i64 - (i as i64) * 3;
            let close = 1000;
            bars.push(Bar {
                source_index: i, timestamp: i as i64,
                open: close, high: close + half, low: close - half,
                close, volume: 1000, untradable: false,
            });
        }
        for i in 12..1500usize {
            let base = 1000i64 + (i as i64);
            let cycle = (((i as f64) / 23.0).sin() * 40.0) as i64;
            let close = base + cycle;
            bars.push(Bar {
                source_index: i, timestamp: i as i64,
                open: close - 1, high: close + 6, low: close - 6,
                close, volume: 1000, untradable: false,
            });
        }

        let config = ThetaConfig::default();
        let mut incr = IncrementalClassifier::new(&bars, &config);
        for i in 0..bars.len() {
            let (incr_cls, incr_tower) = incr.classify_at(i);
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (leg_cls, leg_tower) = classifier::classify_with_tower(&l0, &config);
            assert_eq!(incr_cls, leg_cls, "confirmed_len bar {i}: classification bit-exact 破裂");
            assert_eq!(incr_tower.len(), leg_tower.len(), "confirmed_len bar {i}: tower 层数破裂");
            for (lvl, (il, ll)) in incr_tower.iter().zip(leg_tower.iter()).enumerate() {
                assert_eq!(il, ll, "confirmed_len bar {i} lvl {lvl}: tower LeveledMove 破裂");
            }
        }
    }

    // ================= A3 证书半边 always-run 稀疏变异 oracle（codex 审计第6条根修）=================
    //
    // 逐 bar 对拍「证书增量路径」（classify_with_tower_incremental，走 03 dirty_from / 04 truncate+
    // extend / 09 truncate(prefix_count)）vs「强制全量路径」（classify_with_tower(parse_layer(..=i))，
    // 无证书，每 bar 从头全量重算），断言 (Classification, tower) 逐字段 bit-identical。cached_units==
    // units 与 projected_units==project_to_units(upper_moves) 两条内不变量由 classify_with_tower_
    // incremental 内 debug_assert（04 前缀证书 + 09 投影证书）逐 bar 在 test/debug 构建自动护栏。
    //
    // 两独立 fixture（re-audit 精修：两处早停边界 level_idx vs level_idx+1 不同，须拆开）：
    // - fixture1：`units.len() < min_parts` 早停 break（truncate(level_idx) 代码路径，§2.6 路径1）。
    // - fixture2：`units.is_empty()` 早停 break（truncate(level_idx+1) 代码路径，§2.6 路径2）
    //   + had_emitted_window pop-and-rescan，覆盖 T==1（重扫仅复现被 pop 窗口，did_extend 证伪
    //   正向锁）与 T>1（重扫产出多窗口，frontier 值改写，bar-1464 型 cascade）。
    //
    // ★实施期发现（覆盖边界修正，非设计缺陷）：§2.6 的 **removal 子例**——早停 truncate 实际删掉
    // 已建级（塔深下降、级别被移除后重入）——经实测**不可达**：600K 真实 CL bar（塔深至 l_max=6
    // 上限）+ synthetic 9000 bar，reentry_minparts=reentry_empty=**0**。根因：中枢计数单调非降
    // （一级越过 min_parts 后 frontier 重划只改末中枢的值不减其计数，cascade 重扫复现同数或更多），
    // 故任何级越过 min_parts 后不回落，早停 break 恒为「塔生长边界」而非「已建级移除」。truncate
    // 语句仍每终止 bar 执行（no-op 分支），其 removal 语义正确性由 re-audit LevelCache::default
    // 重建等价论证（代码不变量）保证，**非**测试覆盖——不可达路径无法 always-run 覆盖，此为诚实
    // 边界，reentry_* 计数打印留证但不作断言。证据见 `a3_oracle_probe_cl_diag`（#[ignore]，需 CL）。
    //
    // 覆盖证明：probe 断言 pop-rescan T==1/T>1（第6条核心）+ 两处早停 break 代码路径确执行
    // （防「always-run 但覆盖为零」陷阱）。逐 bar cached_units==units / projected_units==
    // project_to_units(upper_moves) 两内不变量由 classify_with_tower_incremental 内 debug_assert
    // 自动护栏（04 前缀证书 + 09 投影证书），本 oracle 的每 bar 运行即触发。

    fn run_oracle(bars: &[Bar], label: &str) -> classifier::oracle_probe::Probe {
        let config = ThetaConfig::default();
        classifier::oracle_probe::reset();
        let mut incr = IncrementalClassifier::new(bars, &config);
        let mut max_depth = 0usize;
        for i in 0..bars.len() {
            let (incr_cls, incr_tower) = incr.classify_at(i);
            max_depth = max_depth.max(incr_tower.len());
            // 强制全量路径（无证书，每 bar 从头重算）——ground truth。
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (full_cls, full_tower) = classifier::classify_with_tower(&l0, &config);
            assert_eq!(incr_cls, full_cls, "[{label}] bar {i}: 证书增量 classification != 强制全量（bit-exact 破裂）");
            assert_eq!(incr_tower.len(), full_tower.len(), "[{label}] bar {i}: 证书增量 tower 层数 != 全量");
            for (lvl, (il, fl)) in incr_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(il, fl, "[{label}] bar {i} lvl {lvl}: 证书增量 tower LeveledMove != 全量（身份/值破裂）");
            }
        }
        let p = classifier::oracle_probe::snapshot();
        eprintln!(
            "[oracle:{label}] n={} max_depth={} T==1={} T>1={} minparts_break={} empty_break={} \
             reentry_minparts={} reentry_empty={}",
            bars.len(), max_depth, p.t_eq1, p.t_gt1, p.minparts_break, p.empty_break,
            p.reentry_minparts, p.reentry_empty
        );
        p
    }

    fn synth_bar(i: usize, close: i64) -> Bar {
        Bar {
            source_index: i,
            timestamp: i as i64,
            open: close - 1,
            high: close + 7,
            low: close - 7,
            close,
            volume: 1000,
            untradable: false,
        }
    }

    /// 确定性伪随机游走（反射边界保持区间）：制造真实数据式的不规则多尺度结构——高级中枢
    /// 频繁在 min_parts 边界附近徘徊 + frontier 古怪线段重划频发，是触发早停缓存血缘失效
    /// （§2.6：某级恰在 min_parts、重划夺走一个中枢 → 跌破 → truncate 已建高级）的现实条件。
    /// LCG（Numerical Recipes 常数）纯整数，跨平台确定性。
    fn pseudo_walk(n: usize, seed: u64, step_span: i64, lo: i64, hi: i64) -> Vec<Bar> {
        let mut s = seed;
        let mut p: i64 = (lo + hi) / 2;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let step = ((s >> 33) as i64).rem_euclid(2 * step_span + 1) - step_span;
            p += step;
            if p > hi {
                p = hi - (p - hi);
            }
            if p < lo {
                p = lo + (lo - p);
            }
            out.push(synth_bar(i, p));
        }
        out
    }

    /// §2.6 removal 子例不可达性的证据（记录用）：CL 真实数据逐窗口探针——2K~9K 小窗全量对拍
    /// bit-exact + 50K~600K O(n) 探针。实测所有窗口 reentry_minparts=reentry_empty=0（塔深至
    /// l_max=6 上限仍无级别移除），坐实「中枢计数单调非降 ⟹ 早停 removal 不可达」。
    #[test]
    #[ignore = "诊断：CL 真实数据早停 removal 不可达性证据（需 CL 数据，O(n²) 小窗对拍 + O(n) 大窗探针）"]
    fn a3_oracle_probe_cl_diag() {
        let config = ThetaConfig::default();
        let ds = super::super::data::load_by_symbol("CL", &config).expect("CL 数据");
        for n in [2000usize, 4000, 6000, 9000] {
            let bars = &ds.bars[..n.min(ds.bars.len())];
            let p = run_oracle(bars, &format!("CL_{n}"));
            let _ = p;
        }
        // O(n) 大规模探针（仅增量路径，无全量对拍）——测早停重入是否在深塔规模才触发。
        for n in [50_000usize, 200_000, 600_000] {
            let n = n.min(ds.bars.len());
            classifier::oracle_probe::reset();
            let mut incr = IncrementalClassifier::new(&ds.bars[..n], &config);
            let mut max_depth = 0usize;
            for i in 0..n {
                let (_, tower) = incr.classify_at(i);
                max_depth = max_depth.max(tower.len());
            }
            let p = classifier::oracle_probe::snapshot();
            eprintln!(
                "[probe-only:CL_{n}] max_depth={} T==1={} T>1={} reentry_minparts={} reentry_empty={}",
                max_depth, p.t_eq1, p.t_gt1, p.reentry_minparts, p.reentry_empty
            );
        }
    }

    /// fixture1：伪随机游走——每 bar 塔在某级经 `units.len() < min_parts` 早停终止，执行
    /// `cache.levels.truncate(level_idx)`（§2.6 路径1 代码路径）。断言该早停 break 确实触发
    /// （truncate 语句每终止 bar 执行）。**注**：其 removal 子例（depth 下降、truncate 删掉已建级）
    /// 经 600K 真实 CL bar（深至 l_max=6）+ synthetic 实测 = 0，因中枢计数单调非降（一级越过
    /// min_parts 后不回落）——见 `a3_oracle_probe_cl_diag` 证据。故 reentry_minparts 不作断言
    /// （不可达路径，truncate 为 bit-exact 安全的防御性护栏，正确性由 re-audit LevelCache::default
    /// 重建等价论证保证，非测试覆盖）。
    #[test]
    fn a3_oracle_minparts_reentry() {
        let bars = pseudo_walk(9000, 0x9E37_79B9_7F4A_7C15, 34, 700, 3300);
        let p = run_oracle(&bars, "minparts_reentry");
        assert!(
            p.minparts_break > 0,
            "fixture1 未触发 min_parts 早停 break（truncate(level_idx) 代码路径未执行）"
        );
    }

    /// fixture2：多尺度锐锯齿——快尺度制造密集中枢 + 频繁 pop-and-rescan，慢尺度偶发簇发段完成
    /// 产 T>1；同时高级 units 偶尔归零触发 units.is_empty 早停（§2.6 路径2 + 第6条 pop 覆盖）。
    #[test]
    fn a3_oracle_pop_rescan_empty() {
        let bars = pseudo_walk(9000, 0xD1B5_4A32_D192_ED03, 46, 500, 3500);
        let p = run_oracle(&bars, "pop_rescan_empty");
        // ★核心覆盖（第6条 refuted 根修）：pop-and-rescan 两分支都命中——T==1（重扫仅复现被 pop
        // 窗口，did_extend 恒 false 而尾部改写，did_extend 证伪正向锁）+ T>1（frontier 值改写，
        // bar-1464 型 cascade 路径）。re-audit：T = tail_upper.len() 精确定义，两分支都断言。
        assert!(p.t_eq1 > 0, "fixture2 未触发 had_emitted_window pop T==1（did_extend 证伪正向锁覆盖为零）");
        assert!(p.t_gt1 > 0, "fixture2 未触发 had_emitted_window pop T>1（frontier 值改写路径覆盖为零）");
        // units.is_empty 早停 truncate(level_idx+1) 代码路径（§2.6 路径2）。removal 子例同 fixture1
        // 不可达（reentry_empty 不作断言，见 a3_oracle_minparts_reentry doc + CL diag 证据）。
        assert!(
            p.empty_break > 0,
            "fixture2 未触发 units.is_empty 早停 break（truncate(level_idx+1) 代码路径未执行）"
        );
    }
}

#[cfg(test)]
mod profile {
    use super::IncrementalClassifier;
    use super::super::data;
    use super::super::super::config::ThetaConfig;
    use super::super::super::{classifier, parser};

    /// **★全引擎大规模标度（acceptance[4]）：parser+classifier+strategy per-bar exp@16K（L2）**。
    ///
    /// goal `g-sigma-complete-l2-nautilus` acceptance[4] 要求**全引擎** per-bar exp≈1.0 @16K，
    /// 证明 parser + classifier + strategy(coverage/interp) 全热路径线性。本 profile 在**真实** CL
    /// OOS 数据上对 n∈{2K,4K,8K,16K} 测三个 cost center：
    ///
    /// 1. **classify**：`IncrementalClassifier::classify_at(i)` 累计（= 增量 parse + 增量塔；
    ///    parser+classifier 合并测，因增量塔与增量 parse 在 classify_at 内串联，外部无法零成本拆）。
    /// 2. **engine_full**：`run_theta_v0_pi(&prefix_dataset)` 端到端（= classify + strategy/coverage/interp
    ///    + fill + closed_loop）——production per-bar 引擎入口。
    /// 3. **closed_loop**：`run_closed_loop(bars)` 单独测（engine_full 含此项，须扣除才得 strategy 净额）。
    ///
    /// **strategy 净额** = engine_full − classify − closed_loop（pi_theta_fill_loop 的 coverage/interp/
    /// registry/risk_gate 部分）。各 cost center 在相邻 n 上算 log-log 局部 exp，n=8K→16K 的 exp 是
    /// **acceptance[4] 的判定值**（16K 目标点）。
    ///
    /// ## 认识论等级：**L2**（真实 CL 数据 + 16K 大规模标度 → 可否证 O(n) 声明）
    ///
    /// formalization-validity-domain 231号：@16K 真实标度 = L2（可否证）；@n=1000 小窗 = L1 不足以
    /// 坐实大规模线性。每个 exp 的 16K 判定值是本工位的可否证产出——exp≈1.0 确认 O(n)，exp≈2.0
    /// 否证（暴露残留 O(n²) 热点）。
    ///
    /// 运行：`cargo test --release -p newchan_rust --lib \
    ///   backtest::incremental::profile::profile_full_engine_scaling_16k -- --ignored --nocapture`
    #[test]
    #[ignore = "全引擎大规模标度 acceptance[4]；需 CL 数据；--release"]
    fn profile_full_engine_scaling_16k() {
        use super::super::runner::{run_closed_loop, run_theta_v0_pi};
        use super::super::data::Dataset;

        let config = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        eprintln!("\n===== 全引擎大规模标度（CL OOS，acceptance[4]，L2 真实数据）=====");
        eprintln!(
            "{:>7} | {:>9} {:>9} {:>9} {:>10} | {:>6} {:>6} {:>6} {:>6}",
            "n", "classify", "clloop", "stratgy", "engine", "cls_e", "cll_e", "str_e", "eng_e"
        );

        let sizes = [2000usize, 4000, 8000, 16000];
        // 相邻点 log-log exp：exp = ln(t1/t0)/ln(n1/n0)。≈1.0=O(n)，≈2.0=O(n²)。
        let logexp =
            |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        // (n, classify, closed_loop, strategy, engine)
        let mut rows: Vec<(usize, f64, f64, f64, f64)> = Vec::new();
        let mut prev: Option<(usize, f64, f64, f64, f64)> = None;

        for &n in &sizes {
            if n > oos.bars.len() {
                eprintln!("(n={n} > 可用 {}，跳过)", oos.bars.len());
                break;
            }
            let bars = &oos.bars[..n];

            // ① classify 累计：per-bar 增量 classify_at（parser 增量 + 增量塔）。
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let t = std::time::Instant::now();
            for i in 0..n {
                let _ = incr.classify_at(i);
            }
            let t_classify = t.elapsed().as_secs_f64();

            // ② closed_loop（engine_full 含此项，须扣除）。
            let t = std::time::Instant::now();
            let _ = run_closed_loop(bars, 1.0);
            let t_clloop = t.elapsed().as_secs_f64();

            // ③ engine_full：production per-bar 引擎入口（prefix Dataset，source_index 已=局部下标）。
            let prefix = Dataset {
                symbol: oos.symbol.clone(),
                bars: bars.to_vec(),
                dates: oos.dates[..n].to_vec(),
                bar_seconds: 60,
            };
            let years = (n as f64) / (252.0 * 390.0); // 名义年数（仅 metrics 用，不影响标度）
            let t = std::time::Instant::now();
            let _ = run_theta_v0_pi(&prefix, &config, years, 1.0);
            let t_engine = t.elapsed().as_secs_f64();

            // strategy 净额 = engine − classify − closed_loop（coverage/interp/registry/risk_gate/fill）。
            let t_strategy = (t_engine - t_classify - t_clloop).max(0.0);

            let (ce, cle, se, ee) = prev
                .map(|(pn, pc, pcl, ps, pe)| {
                    (
                        logexp(pn, pc, n, t_classify),
                        logexp(pn, pcl, n, t_clloop),
                        logexp(pn, ps.max(1e-9), n, t_strategy.max(1e-9)),
                        logexp(pn, pe, n, t_engine),
                    )
                })
                .unwrap_or((f64::NAN, f64::NAN, f64::NAN, f64::NAN));

            eprintln!(
                "{n:>7} | {t_classify:>9.3} {t_clloop:>9.3} {t_strategy:>9.3} {t_engine:>10.3} | \
                 {ce:>6.2} {cle:>6.2} {se:>6.2} {ee:>6.2}"
            );
            use std::io::Write;
            std::io::stderr().flush().ok();

            rows.push((n, t_classify, t_clloop, t_strategy, t_engine));
            prev = Some((n, t_classify, t_clloop, t_strategy, t_engine));
        }

        // 16K 判定（acceptance[4] 目标点：8K→16K 的 exp）。
        if rows.len() >= 2 {
            let (n0, c0, cl0, s0, e0) = rows[rows.len() - 2];
            let (n1, c1, cl1, s1, e1) = rows[rows.len() - 1];
            let verdict = |e: f64| {
                if e < 1.3 {
                    "O(n) ✓"
                } else if e < 1.7 {
                    "亚二次(超线性)"
                } else {
                    "O(n²) ✗ 残留热点"
                }
            };
            let ce = logexp(n0, c0, n1, c1);
            let cle = logexp(n0, cl0, n1, cl1);
            let se = logexp(n0, s0.max(1e-9), n1, s1.max(1e-9));
            let ee = logexp(n0, e0, n1, e1);
            eprintln!(
                "\n★acceptance[4] 判定（{n0}→{n1} 16K 目标点 exp，L2 真实数据）：\n  \
                 classify   exp={ce:.2}  {}\n  \
                 closed_loop exp={cle:.2}  {}\n  \
                 strategy   exp={se:.2}  {}\n  \
                 engine_full exp={ee:.2}  {}",
                verdict(ce),
                verdict(cle),
                verdict(se),
                verdict(ee),
            );
            eprintln!(
                "  判读：engine_full exp≈1.0 ⟹ 全引擎 O(n) 达成（acceptance[4] 确认）；\n  \
                 某 cost center exp≈2.0 ⟹ 该段残留 O(n²)，owner 见 cost center 名。\n  \
                 ★L2：真实数据 + 16K 大规模，可否证（231号）。"
            );
        }
        assert!(!rows.is_empty(), "至少 profile 一个窗口（n=2000 应可用）");
    }

    /// **★on2w3 candidate 残余定位：strategy 段 stage 拆解（THETA_PROFILE_STAGES=1）**。
    ///
    /// 全引擎 `run_theta_v0_pi` 在 CL 上跑，dump cand_build_step / cand_build_merge /
    /// cand_merge_consume 三 stage 计时 + cand_count 跨度。定位 strategy exp≈1.9 的 O(n²) 是
    /// candidate 构建（拼接/attach）还是 merge 消费（HashSet 全量重建）。
    /// 运行：`THETA_PROFILE_STAGES=1 CAND_PROFILE_BARS=16000 cargo test --release -p newchan_rust \
    ///   --lib backtest::incremental::profile::profile_cand_stages -- --ignored --nocapture`。
    #[test]
    #[ignore = "on2w3 candidate stage 拆解；需 CL；THETA_PROFILE_STAGES=1；--release"]
    fn profile_cand_stages() {
        use super::super::runner::run_theta_v0_pi;
        use super::super::data::Dataset;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("需 CL 数据");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        if std::env::var("THETA_PROFILE_STAGES").is_err() {
            eprintln!("★未设 THETA_PROFILE_STAGES=1 ⟹ dump 为空。");
        }
        for &n in &[8000usize, 16000] {
            let n = n.min(oos.bars.len());
            classifier::stage_profile::reset();
            let prefix = Dataset {
                symbol: oos.symbol.clone(),
                bars: oos.bars[..n].to_vec(),
                dates: oos.dates[..n].to_vec(),
                bar_seconds: 60,
            };
            let years = (n as f64) / (252.0 * 390.0);
            let t = std::time::Instant::now();
            let _ = run_theta_v0_pi(&prefix, &config, years, 1.0);
            eprintln!("\n===== profile_cand_stages n={n}（墙钟={:.2}s）=====", t.elapsed().as_secs_f64());
            classifier::stage_profile::dump();
        }
    }

    /// **★classify_at 内部拆解：parser-append 累计 vs classify_with_tower_incremental 累计（定位 O(n²) 段）**。
    ///
    /// `profile_full_engine_scaling_16k` 测出 classify(整) exp≈2.17 O(n²)，但 parser::profile
    /// 测出 ParseLayerIncr::append 累计 exp≈0.94 O(n)。本拆解坐实 O(n²) 在**增量塔**
    /// （`classify_with_tower_incremental`）而非 parser——逐 bar 分别计时 parser_incr.append 与
    /// 塔续算，各自累计后拟合 exp。
    ///
    /// **L2**（真实 CL，16K）。
    #[test]
    #[ignore = "classify_at 内部 O(n²) 段定位；需 CL；--release"]
    fn profile_classify_at_decompose_16k() {
        let config = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        eprintln!("\n===== classify_at 拆解：parser-append vs 增量塔（CL OOS，L2）=====");
        eprintln!("{:>7} | {:>10} {:>10} | {:>7} {:>7}", "n", "parse_s", "tower_s", "p_exp", "t_exp");
        let logexp =
            |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [2000usize, 4000, 8000, 16000];
        let mut prev: Option<(usize, f64, f64)> = None;
        for &n in &sizes {
            if n > oos.bars.len() {
                break;
            }
            let bars = &oos.bars[..n];
            let mut parser_incr = parser::ParseLayerIncr::new(&config);
            let mut tower_cache = classifier::TowerCache::default();
            let mut t_parse = 0.0f64;
            let mut t_tower = 0.0f64;
            for i in 0..n {
                let t = std::time::Instant::now();
                let l0_i = parser_incr.append(bars[i]);
                t_parse += t.elapsed().as_secs_f64();
                let t = std::time::Instant::now();
                let _ = classifier::classify_with_tower_incremental(&l0_i, &config, &mut tower_cache);
                t_tower += t.elapsed().as_secs_f64();
            }
            let (pe, te) = prev
                .map(|(pn, pp, pt)| (logexp(pn, pp, n, t_parse), logexp(pn, pt, n, t_tower)))
                .unwrap_or((f64::NAN, f64::NAN));
            eprintln!("{n:>7} | {t_parse:>10.3} {t_tower:>10.3} | {pe:>7.2} {te:>7.2}");
            use std::io::Write;
            std::io::stderr().flush().ok();
            prev = Some((n, t_parse, t_tower));
        }
        eprintln!(
            "\n判读：parser p_exp≈1.0 (O(n)) + tower t_exp≈2.0 (O(n²)) ⟹ O(n²) 根在增量塔\n  \
             classify_with_tower_incremental（mod.rs:634）——逐 bar 全前缀重算项（见报告）。L2。"
        );
    }

    /// **★A0（YAGNI 重开门）：克隆簇阶段占比 profile（THETA_PROFILE_STAGES 全阶段拆解，BTC ≥1M bar）**。
    ///
    /// 泳道 A A0（algo-opt-plan-20260702.md）：`classify_with_tower_incremental` 逐 bar 驱动 BTC
    /// 前 N bar（默认 1M，env `A0_PROFILE_BARS` 可调），`THETA_PROFILE_STAGES=1` 时 `dump()` 打印
    /// 15 阶段耗时。克隆簇 = {00b_l0_units_clone, 04_cached_units_copy, 07c_bsp_memo_clone,
    /// 08_levels_centers_clone, 10_projected_units_clone}（A1 的 Rc/借用 目标段）占分类器总耗时之比
    /// = A1 是否值得重开的唯一合法证据。占比高 ⟹ A1 全量；占比低 ⟹ 缩水/撤项。
    ///
    /// ## 认识论等级：**L1**（CPU 度量，零信息增量，231号）——耗时可复现，不验证 Θ 市场有效。
    ///
    /// 运行：`THETA_PROFILE_STAGES=1 A0_PROFILE_BARS=1000000 cargo test --release -p newchan_rust \
    ///   --lib backtest::incremental::profile::profile_clone_cluster_a0 -- --ignored --nocapture`
    #[test]
    #[ignore = "A0 克隆簇占比 profile；需 BTC；THETA_PROFILE_STAGES=1；--release"]
    fn profile_clone_cluster_a0() {
        let config = ThetaConfig::default();
        let ds = match data::load_by_symbol("BTC", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需 BTC 数据");
            }
        };
        let n_avail = ds.bars.len();
        let n: usize = std::env::var("A0_PROFILE_BARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1_000_000)
            .min(n_avail);
        eprintln!(
            "\n===== A0 克隆簇占比 profile（BTC 前 {n}/{n_avail} bar，classify_with_tower_incremental）====="
        );
        if std::env::var("THETA_PROFILE_STAGES").is_err() {
            eprintln!("★未设 THETA_PROFILE_STAGES=1 ⟹ dump 为空（零开销直通）。设 env 后重跑才有数据。");
        }
        let bars = &ds.bars[..n];
        let mut parser_incr = parser::ParseLayerIncr::new(&config);
        let mut tower_cache = classifier::TowerCache::default();
        let t0 = std::time::Instant::now();
        for i in 0..n {
            let l0_i = parser_incr.append(bars[i]);
            let _ = classifier::classify_with_tower_incremental(&l0_i, &config, &mut tower_cache);
        }
        let wall = t0.elapsed().as_secs_f64();
        eprintln!("[A0] {n} bar 逐 bar classify_with_tower_incremental 墙钟={wall:.2}s");
        classifier::stage_profile::dump();
        eprintln!(
            "★A0 克隆簇 = {{00b_l0_units_clone, 04_cached_units_copy, 07c_bsp_memo_clone, \
             08_levels_centers_clone, 10_projected_units_clone}}；占比 = 克隆簇Σ / 全阶段Σ（见 dump）。L1。"
        );
    }

    /// **★A3 验收：证书半边 03/04 阶段计时（CL 1M bar，THETA_PROFILE_STAGES=1）**。
    ///
    /// A3 后 03_frontier_compare（L1+ stable 抬到 dirty_from）+ 04_cached_units_copy（truncate+
    /// extend O(tail)）应从 A0 基线（1M：03=2446ms/4.7%、04=4010ms/7.7%）降至近零占比。
    /// 运行：`THETA_PROFILE_STAGES=1 A3_PROFILE_BARS=1000000 cargo test --release -p newchan_rust \
    ///   profile_stage_a3_cl -- --ignored --nocapture`。
    #[test]
    #[ignore = "A3 stage 03/04 计时；需 CL；THETA_PROFILE_STAGES=1；--release"]
    fn profile_stage_a3_cl() {
        let config = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需 CL 数据");
            }
        };
        let n: usize = std::env::var("A3_PROFILE_BARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1_000_000)
            .min(ds.bars.len());
        eprintln!("\n===== A3 stage 计时（CL 前 {n} bar，classify_with_tower_incremental）=====");
        if std::env::var("THETA_PROFILE_STAGES").is_err() {
            eprintln!("★未设 THETA_PROFILE_STAGES=1 ⟹ dump 为空。");
        }
        let mut parser_incr = parser::ParseLayerIncr::new(&config);
        let mut tower_cache = classifier::TowerCache::default();
        let t0 = std::time::Instant::now();
        for i in 0..n {
            let l0_i = parser_incr.append(ds.bars[i]);
            let _ = classifier::classify_with_tower_incremental(&l0_i, &config, &mut tower_cache);
        }
        eprintln!("[A3] {n} bar 墙钟={:.2}s", t0.elapsed().as_secs_f64());
        classifier::stage_profile::dump();
    }

    /// **★诊断（工位 E 留档）：classifier 增量 vs 全量 bit-exact 隔离**。
    ///
    /// 同一 legacy `parse_layer(&bars[..=i])` 输入喂 `classify_with_tower_incremental`（持久 cache）
    /// 与 `classify_with_tower`（全量），隔离 **classifier 增量** vs parser 增量。坐实
    /// `classify_with_tower_incremental` 在真实 CL 上 **bar 1464 与全量发散**（第 3 个 L0 中枢
    /// `end_index`/`dd` 不同——增量 resume 续扫的 frontier 中枢比全量非重叠扫描多吸收段）。
    ///
    /// ★此发散**先于工位 E 的性能改动**（git stash 验证：pure-HEAD 同样 1464 发散，同值）——
    /// 是 `detect_centers_windowed_resume` 的 frontier 中枢不稳定性 bug（resume cursor 把未确认的
    /// 末窗口当作 immutable，违反 bit-exact 充要条件 #2），非性能 memo/cache 引入。属上浮矛盾。
    /// **L2**（真实 CL）。
    #[test]
    #[ignore = "工位 E 诊断：classifier 增量 frontier 中枢 bit-exact 发散（bar 1464，pre-existing）；需 CL"]
    fn diag_classifier_resume_frontier_divergence() {
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 1500.min(oos.bars.len());
        let bars = &oos.bars[..n];
        let mut cache = classifier::TowerCache::new();
        for i in 0..n {
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (inc_cls, _) = classifier::classify_with_tower_incremental(&l0, &config, &mut cache);
            let (full_cls, _) = classifier::classify_with_tower(&l0, &config);
            if inc_cls != full_cls {
                eprintln!("classifier-only divergence at bar {i}: segs={} merged={}",
                    l0.segments.len(), l0.merged_bars.len());
                for (lvl, (il, fl)) in inc_cls.levels.iter().zip(full_cls.levels.iter()).enumerate() {
                    if il.centers != fl.centers {
                        eprintln!("  L{lvl} centers DIFFER\n    inc ={:?}\n    full={:?}", il.centers, fl.centers);
                    }
                }
                return; // 诊断目的：报告首个发散点，不 panic（pre-existing 矛盾，已上浮）。
            }
        }
        eprintln!("no classifier-only divergence in 0..{n}");
    }

    /// **★诊断（frontier-bit-exact 根因隔离）：增量 parser segments vs 全量 parser segments**。
    ///
    /// `decisive_endpoint` 用 `IncrementalClassifier`（增量 parser + 增量塔）对拍 `parse_layer`（全量
    /// parser + 全量塔），~bar 15650 发散；但 `diag_classifier_resume_frontier_divergence`（**同一** 全量
    /// `parse_layer` 输入喂增量塔 vs 全量塔）到 20000 **无发散** ⟹ classifier 增量塔在同输入下 bit-exact。
    /// 两者唯一区别 = parser 路径。本测试逐 bar 对拍 `ParseLayerIncr::append(bars[i]).segments` vs
    /// `parse_layer(&bars[..=i]).segments`，定位发散是否源于 **parser 增量 bit-exact 破裂**（非 classifier）。
    /// **L2**（真实 CL）。
    #[test]
    #[ignore = "诊断：parser 增量 vs 全量 segments bit-exact（frontier 根因隔离）；需 CL"]
    fn diag_parser_incr_vs_full_segments() {
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 50000.min(oos.bars.len());
        let bars = &oos.bars[..n];
        let mut incr = parser::ParseLayerIncr::new(&config);
        let mut div_count = 0usize;
        let mut first_div: Option<usize> = None;
        for i in 0..n {
            let l0_incr = incr.append(bars[i]);
            let l0_full = parser::parse_layer(&bars[..=i], &config);
            let segs_i: &[_] = &l0_incr.segments;
            let segs_f: &[_] = &l0_full.segments;
            if segs_i != segs_f {
                div_count += 1;
                if first_div.is_none() {
                    first_div = Some(i);
                    eprintln!("★parser segments divergence at bar {i}: incr.len={} full.len={} confirmed_len={}",
                        segs_i.len(), segs_f.len(), l0_incr.segments_confirmed_len);
                    let m = segs_i.len().min(segs_f.len());
                    for k in 0..m {
                        if segs_i[k] != segs_f[k] {
                            eprintln!("  seg[{k}] incr={:?}\n          full={:?}", segs_i[k], segs_f[k]);
                            eprintln!("  发散段是末段? {} (segs.len-1={})", k == segs_i.len().saturating_sub(1).min(segs_f.len().saturating_sub(1)), segs_i.len().saturating_sub(1));
                            break;
                        }
                    }
                }
            }
        }
        eprintln!("\n★parser 发散统计 in 0..{n}：div_count={div_count} first={first_div:?} \
                   （持续发散⟹持久 bit-exact 破裂；单点⟹瞬时 frontier 波动）");
    }

    /// **★#88 性能计数器（codex #87 验收项2）：earliest_unsealed_from 前移轨迹 + frontier rescan
    /// 长度 + 墙钟标度**——判定修补版 A 是否退化为候选 C 的 O(n²)（边界条件3）。
    ///
    /// 逐 bar `ParseLayerIncr::append`（生产 parser 路径），记录：
    /// - **euf 轨迹**：euf 变化次数（advance=前移/regress=后退）；每 bar euf 单调非增 ⟹ regress=0 期望，
    ///   advance>0 说明 confirmed_bound 未被永久锚死在早点（否则退化 O(n²)）。
    /// - **rescan 长度**：`segments.len() - segments_confirmed_len`（重扫段数代理）；max/p95/均值。
    /// - **墙钟 exp**：相邻 n 的 log-log 斜率 ≈1.0=O(n)、≈2.0=O(n²)。cascade_count=N/A（未启用候选C）。
    ///
    /// **L1**（CPU/结构计数，零信息增量，231号）——不验证 Θ 市场有效，仅证性能红线。
    #[test]
    #[ignore = "#88 性能计数器：euf 前移轨迹 + rescan 长度 + 墙钟标度；需 CL；--release"]
    fn perf_frontier_rescan_counters_88() {
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2015-01-01", "2025-06-30");
        eprintln!("\n===== #88 frontier rescan 性能计数器（CL，ParseLayerIncr 生产路径）=====");
        eprintln!("{:>8} | {:>8} | {:>10} {:>8} {:>7} {:>7} | {:>9} {:>9} {:>6}",
            "n", "wall_s", "exp", "euf_adv", "rs_max", "rs_p95", "euf_fin", "euf_min", "adv_r");
        let logexp = |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [50_000usize, 150_000, 300_000];
        let mut prev: Option<(usize, f64)> = None;
        for &n in &sizes {
            if n > oos.bars.len() { eprintln!("(n={n} > {}，跳过)", oos.bars.len()); continue; }
            let bars = &oos.bars[..n];
            let mut incr = parser::ParseLayerIncr::new(&config);
            let mut rescans: Vec<usize> = Vec::with_capacity(n);
            let mut euf_prev: Option<usize> = None;
            let mut euf_adv = 0usize; // 前移（值增大——unsealed 起点右移，重扫区间缩小）
            let mut euf_reg = 0usize; // 后退（值减小——历史最小值被更早候选拉低）
            let mut euf_min_ever: Option<usize> = None;
            let mut euf_final: Option<usize> = None;
            let t0 = std::time::Instant::now();
            for i in 0..n {
                let l0 = incr.append(bars[i]);
                rescans.push(l0.segments.len().saturating_sub(l0.segments_confirmed_len));
                let euf = l0.segments_earliest_unsealed;
                if euf != euf_prev {
                    match (euf_prev, euf) {
                        (Some(a), Some(b)) if b > a => euf_adv += 1,
                        (Some(a), Some(b)) if b < a => euf_reg += 1,
                        (None, Some(_)) => euf_adv += 1,
                        _ => {}
                    }
                    euf_prev = euf;
                }
                if let Some(e) = euf { euf_min_ever = Some(euf_min_ever.map_or(e, |m| m.min(e))); }
                euf_final = euf;
            }
            let wall = t0.elapsed().as_secs_f64();
            rescans.sort_unstable();
            let rs_max = *rescans.last().unwrap_or(&0);
            let rs_p95 = rescans[(rescans.len() as f64 * 0.95) as usize];
            let exp = prev.map(|(pn, pt)| logexp(pn, pt, n, wall)).unwrap_or(f64::NAN);
            // adv_r = euf 前移次数 / 总变化次数（接近 1 ⟹ euf 主要在前移，未锚死 ⟹ 非 O(n²)）。
            let adv_r = if euf_adv + euf_reg > 0 { euf_adv as f64 / (euf_adv + euf_reg) as f64 } else { f64::NAN };
            eprintln!("{n:>8} | {wall:>8.2} | {exp:>10.2} {euf_adv:>8} {rs_max:>7} {rs_p95:>7} | {euf_final:>9?} {euf_min_ever:>9?} {adv_r:>6.2}",);
            use std::io::Write; std::io::stderr().flush().ok();
            prev = Some((n, wall));
        }
        eprintln!("\n判读（边界条件3）：wall exp≈1.0 + rs_max/p95 有界 ⟹ 修补版A 非退化 O(n²)（前移有效）；\n  \
             exp≈2.0 + euf 长期锚定早点（euf_min≈0、rs_max≈segs 全量）⟹ 退化候选C 性能，须上浮重评。");
    }

    /// **★诊断（frontier-bit-exact 首发散 bar 定位）：IncrementalClassifier 逐 bar vs 全量**。
    ///
    /// 精确复现生产路径（增量 parser + 增量塔 + cache 跨 bar 连续复用）的**首个** incr≠full bar，
    /// dump 该 bar 的 segments/merged 长度 + L0 首个发散 center。区别于 `diag_classifier_resume_frontier_divergence`
    /// （喂全量 parse_layer，confirmed_len=0）——本测试喂增量 parser（confirmed_len>0，走缓存复用分支）。
    /// **L2**（真实 CL）。
    #[test]
    #[ignore = "诊断：IncrementalClassifier 逐 bar vs 全量首发散定位；需 CL"]
    fn diag_incremental_classifier_first_divergence() {
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 50000.min(oos.bars.len());
        let bars = &oos.bars[..n];
        let mut incr = IncrementalClassifier::new(bars, &config);
        for i in 0..n {
            let (inc_cls, _) = incr.classify_at(i);
            let l0 = parser::parse_layer(&bars[..=i], &config);
            let (full_cls, _) = classifier::classify_with_tower(&l0, &config);
            if inc_cls != full_cls {
                eprintln!("★IncrementalClassifier divergence at bar {i}: segs={} merged={} confirmed_len={}",
                    l0.segments.len(), l0.merged_bars.len(), l0.segments_confirmed_len);
                for (lvl, (il, fl)) in inc_cls.levels.iter().zip(full_cls.levels.iter()).enumerate() {
                    if il.centers != fl.centers {
                        let m = il.centers.len().min(fl.centers.len());
                        let k = (0..m).find(|&k| il.centers[k] != fl.centers[k]);
                        eprintln!("  L{lvl} centers DIFFER (inc={} full={}) first_diff={:?}",
                            il.centers.len(), fl.centers.len(), k);
                        if let Some(k) = k {
                            eprintln!("    inc [{k}]={:?}\n    full[{k}]={:?}", il.centers[k], fl.centers[k]);
                        }
                        break;
                    }
                }
                return;
            }
        }
        eprintln!("no IncrementalClassifier divergence in 0..{n}");
    }

    /// **★决定性对拍（Task #10）：长历史终点 level 分布 增量生产路径 vs 全量 ground truth**。
    ///
    /// codex #8 攻击 acc-classification「窗口依赖非 bug」：用 L1 合成 bit-exact 排除 L2 真实 frontier
    /// 发散（bar 1464，`detect_centers_windowed_resume` resume cursor 把未确认末窗当 immutable）。翻转
    /// 条件 #2 = 增量 vs 全量对拍验证 frontier bug 在长历史/真实数据已修复。
    ///
    /// ## 判别设计（终点对拍，非每 bar——O(n) 可行到 300K）
    ///
    /// - **增量生产路径**：`IncrementalClassifier::new(bars[..n])` 逐 bar `classify_at` 到终点 → 最终
    ///   classification（`run_theta_v0_pi` 实际跑的 substrate 路径）。
    /// - **全量 ground truth**：`classify_with_tower(parse_layer(bars[..n]))` 单次全量重算（无增量）。
    /// - **逐 level 对拍**：`centers` / `bsp` bit-exact 比较，报每级数量 + 首个发散字段。
    ///
    /// ## 决定性判读
    ///
    /// - **终点 level 分布 bit-exact 相等** ⟹ 增量在长历史无 frontier 发散（cascade_reset 修复有效）
    ///   ⟹ 全历史 level2-4=0 **不是增量 bug 伪影**，窗口依赖判定升回坐实（H2 否证，alpha 不重开）。
    /// - **高级别（level≥2）centers/bsp 发散** ⟹ frontier bug 使高级别塔退化 ⟹ level2-4=0 是 bug 伪影
    ///   ⟹ H2 翻案，高级别信号可能复活，acc-alpha 对象重开。
    ///
    /// ## 认识论等级：**L2**（真实 CL/BTC 长历史，可否证 frontier bug 存否）
    ///
    /// 运行：`cargo test --release -p newchan_rust --lib \
    ///   backtest::incremental::profile::decisive_endpoint_tower_parity_longhistory -- --ignored --nocapture`
    #[test]
    #[ignore = "决定性对拍 Task #10：长历史终点 level 分布增量 vs 全量；需 CL+BTC；--release"]
    fn decisive_endpoint_tower_parity_longhistory() {
        let config = ThetaConfig::default();
        // CL：全历史切片（3.5M+）；BTC：全量（4.6M）。取递增窗口找发散点。
        let symbols: [(&str, Option<(&str, &str)>); 2] =
            [("CL", Some(("2015-01-01", "2025-06-30"))), ("BTC", None)];
        let sizes = [50_000usize, 150_000, 300_000];

        let mut any_divergence = false;
        for (sym, window) in symbols {
            let ds = match super::super::data::load_by_symbol(sym, &config) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[{sym}] DATA BLOCKER: {e}（跳过）");
                    continue;
                }
            };
            let bars_all = match window {
                Some((a, b)) => ds.slice_date_window(a, b).bars,
                None => ds.bars.clone(),
            };
            eprintln!("\n===== [{sym}] 决定性对拍（终点 level 分布，增量生产 vs 全量 GT）total={} =====",
                bars_all.len());

            for &n in &sizes {
                if n > bars_all.len() {
                    eprintln!("[{sym}] n={n} > 可用 {}，跳过", bars_all.len());
                    continue;
                }
                let bars = &bars_all[..n];

                // ① 增量生产路径：逐 bar classify_at 到终点。
                let mut incr = IncrementalClassifier::new(bars, &config);
                let mut last = classifier::Classification::default();
                for i in 0..n {
                    let (cls, _) = incr.classify_at(i);
                    last = cls;
                }

                // ② 全量 ground truth：单次全量重算。
                let l0 = parser::parse_layer(bars, &config);
                let (full, _) = classifier::classify_with_tower(&l0, &config);

                // ③ 逐 level 对拍。
                let n_lvl_i = last.levels.len();
                let n_lvl_f = full.levels.len();
                let levels_match = n_lvl_i == n_lvl_f;
                eprint!("[{sym}] n={n:>7}: 增量 lvls={n_lvl_i} 全量 lvls={n_lvl_f}");
                if !levels_match {
                    eprintln!("  ★★层数发散");
                    any_divergence = true;
                }
                let mut level_diverged = false;
                for lvl in 0..n_lvl_i.min(n_lvl_f) {
                    let li = &last.levels[lvl];
                    let lf = &full.levels[lvl];
                    let centers_eq = li.centers == lf.centers;
                    let bsp_eq = li.bsp == lf.bsp;
                    if !centers_eq || !bsp_eq {
                        if !level_diverged {
                            eprintln!();
                        }
                        level_diverged = true;
                        any_divergence = true;
                        eprintln!(
                            "  ★L{lvl} 发散: centers({}/{}) eq={centers_eq}  bsp({}/{}) eq={bsp_eq}",
                            li.centers.len(), lf.centers.len(), li.bsp.len(), lf.bsp.len()
                        );
                        // 首个 center 发散细节。
                        if !centers_eq {
                            for (ci, (a, b)) in li.centers.iter().zip(lf.centers.iter()).enumerate() {
                                if a != b {
                                    eprintln!("      center[{ci}] incr={a:?}\n              full={b:?}");
                                    break;
                                }
                            }
                            if li.centers.len() != lf.centers.len() {
                                eprintln!("      (center 数不同：incr={} full={})", li.centers.len(), lf.centers.len());
                            }
                        }
                    }
                }
                if !level_diverged && levels_match {
                    // 全等：报每级 bsp 数（level 分布）以佐证「非 bug」。
                    let dist: Vec<usize> = full.levels.iter().map(|l| l.bsp.len()).collect();
                    eprintln!("  bit-exact ✓  bsp/lvl={dist:?}");
                }
                use std::io::Write;
                std::io::stderr().flush().ok();
            }
        }

        eprintln!(
            "\n★决定性判读：\n  \
             全 bit-exact ✓ ⟹ 增量长历史无 frontier 发散 ⟹ level2-4=0 非 bug 伪影（窗口依赖坐实，H2 否证）。\n  \
             任一 level≥2 发散 ⟹ frontier bug 使高级别退化 ⟹ level2-4=0 是 bug 伪影（H2 翻案，alpha 重开）。"
        );
        assert!(
            !any_divergence,
            "★frontier bug 确认：增量生产路径与全量 ground truth 在长历史终点 level 分布发散——\
             高级别塔退化，level2-4=0 是 bug 伪影，H2 翻案。见上方 ★L 发散明细。"
        );
    }

    /// **★Profile：增量链 vs legacy 全量 标度对比**。
    ///
    /// 增量路径 = ParseLayerIncr::append + classify_with_tower_incremental（cache 跨 bar 复用）。
    /// 对比 legacy = parse_layer + classify_with_tower（每 bar 全算）。
    /// 增量总 exp 应低于 legacy（inclusion + 塔构造增量化），但下游 fractal/stroke/segment 仍全量
    /// 重算（parser 工位约束），故 exp 不会到 1.0。
    /// **★工位 K L2 证据：strategy fill-loop 热点分解（extract_elements + registry.merge 标度）**。
    ///
    /// 隔离 runner fill loop 的两段 per-bar 操作（tree-prefix 提取 + registry merge）标度，证 exp≈1.0。
    ///
    /// ★工位 4c 修正（exp 2.12 是测错路径，非生产仍坏）：旧 diag 用**非生产路径**——`coverage_elements_with_tower`
    /// （materialize `(*tree).clone()` O(tree)，**不传 TreeCache** ⟹ 每 bar 全量 extract）+ 手动双调
    /// `extract_elements` + 旧 `merge`（返新对象，clone O(registry)）。这套是被刻意保留的优化前路径，
    /// 测出 O(n²) 是必然——它不反映 runner 实际跑的路径。
    ///
    /// 生产热路径（runner.rs:577/608）：`coverage_elements_and_gamma_with_tower_cached`（命中返
    /// `Rc::clone` O(1)，§16 confirmed prefix 不变 ⟹ CL 16K 仅 26 次 miss）+ `merge_in_place`（原地增量，
    /// 消 clone+全扫）。本 diag 现镜像该路径——`x_exp`/`m_exp` 反映生产真实标度（应 ≈1.0）。
    #[test]
    #[ignore = "工位 4c L2：strategy 热点分解标度（镜像生产 cached+in_place）；需 CL；--release"]
    fn diag_strategy_hotspot_decompose_16k() {
        use super::super::super::strategy::{coverage, interp, persistent};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        eprintln!("\n===== strategy 热点分解（CL OOS，L2，镜像生产 cached+in_place）=====");
        eprintln!("{:>7} | {:>10} {:>10} | {:>7} {:>7}", "n", "extract_s", "merge_s", "x_exp", "m_exp");
        let logexp = |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [2000usize, 4000, 8000, 16000];
        let mut prev: Option<(usize, f64, f64)> = None;
        for &n in &sizes {
            if n > oos.bars.len() { break; }
            let bars = &oos.bars[..n];
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let mut registry = persistent::PersistentRegistry::new();
            let mut tree_cache = interp::TreeCache::new(); // 跨 bar 复用（§16，与 runner 同）
            let mut t_extract = 0.0f64;
            let mut t_merge = 0.0f64;
            let mut t_merge_tree = 0.0f64; // 仅 tree snapshot（无 candidate）→ 隔离 tree 段冗余 upsert 标度
            let mut last_cand = 0usize;
            // ★工位 4f：镜像生产 runner 的 tree Rc ptr_eq 脏检查（命中⟹tree 段跳过 step 1'/2'）。
            let mut prev_merge_tree: Option<std::rc::Rc<Vec<coverage::CoverageElement>>> = None;
            for i in 0..n {
                let (cls, tower) = incr.classify_at(i);
                // 生产 tree-prefix 提取：cached（命中 Rc::clone O(1)），candidate 段 overlay 不进树 clone。
                let t = std::time::Instant::now();
                let (tree_ref, candidates_ref, _gamma) =
                    interp::coverage_elements_and_gamma_with_tower_cached(
                        &cls, &tower, &mut Some(&mut tree_cache),
                    );
                t_extract += t.elapsed().as_secs_f64();
                last_cand = candidates_ref.len();
                // 生产 merge（runner split 口径）：tree_dirty=false（ptr_eq 命中）⟹ 跳过 tree 段。
                let tree_dirty = prev_merge_tree.as_ref()
                    .map(|p| !std::rc::Rc::ptr_eq(p, &tree_ref)).unwrap_or(true);
                let t = std::time::Instant::now();
                registry.merge_in_place_split(&tree_ref, tree_dirty, &candidates_ref, true, &[]);
                t_merge += t.elapsed().as_secs_f64();
                prev_merge_tree = Some(std::rc::Rc::clone(&tree_ref));
                // 隔离测量：tree 段全量（candidate 空，tree_dirty=true）→ 旧 O(n²) 基底对照。
                let t = std::time::Instant::now();
                registry.merge_in_place_split(&tree_ref, true, &[], true, &[]);
                t_merge_tree += t.elapsed().as_secs_f64();
            }
            let (xe, me) = prev.map(|(pn, px, pm)| (logexp(pn, px, n, t_extract), logexp(pn, pm, n, t_merge)))
                .unwrap_or((f64::NAN, f64::NAN));
            // tree_only=仅 tree snapshot merge（candidate 空）→ 隔离证：tree 段独立即 O(n²)，
            // candidate（cand_last）极小且系统性复用 tree id（实测 collide≈cand），∴ merge O(n²)
            // 根因=snapshot(tree prefix)∝confirmed 每 bar 全扫，与 extract 同根（§16 candidate 不可缓存 ceiling）。
            eprintln!("{n:>7} | {t_extract:>10.3} {t_merge:>10.3} | {xe:>7.2} {me:>7.2}  registry_len={} tree_only={t_merge_tree:.3} cand_last={last_cand}", registry.len());
            prev = Some((n, t_extract, t_merge));
        }
    }

    /// **★工位 4g 前置诊断：TreeKey-miss 频率 + 成本分布**（修复路径决策数据，L2）。
    ///
    /// #18 报告 extract x_exp≈2.0 但绝对值极小，归因 TreeKey-miss ceiling（miss 时全量
    /// `extract_elements` + 3×`build_*_index`）。修复（检测最高级变化 + 索引重映射重用前缀）是
    /// high bit-exact risk 重构——动手前必须先量化：miss 占多少 bar？miss 累积成本占 extract 总时
    /// 多少？miss 是否集中在大 n 端（O(Σtree_at_miss) 超线性的来源）？
    ///
    /// 测：逐 bar 记 hit/miss + miss 时 tree.len()。若 miss 累积成本 << extract 总时 ⟹ ceiling 不是
    /// exp≈2.0 的真因（exp 来自别处），修复无收益（ponytail：不修不需要存在的东西）。
    #[test]
    #[ignore = "工位 4g：TreeKey-miss 频率+成本分布；需 CL；--release --ignored"]
    fn diag_treekey_miss_cost_16k() {
        use super::super::super::strategy::{coverage, interp};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 16_000.min(oos.bars.len());
        let bars = &oos.bars[..n];
        let mut incr = super::IncrementalClassifier::new(bars, &config);
        let mut prev_key = interp::TreeKey::default();
        let mut first = true;
        let mut miss_count = 0usize;
        let mut miss_cost = 0.0f64;   // miss 时 extract_elements + 3 index 全量重建累计耗时
        let mut total_extract_work = 0.0f64; // 所有 bar 若无缓存的全量重建总时（基底对照）
        let mut miss_tree_sizes: Vec<usize> = Vec::new();
        for i in 0..n {
            let (_cls, tower) = incr.classify_at(i);
            let key = interp::TreeKey::of(&tower);
            let is_miss = first || key != prev_key;
            first = false;
            prev_key = key;
            // 量化一次全量重建成本（extract + 3 index，§16 ceiling 的真实工作量）。
            let t = std::time::Instant::now();
            let tree = coverage::extract_elements(&tower);
            let _e = coverage::build_tree_endpoint_index(&tree);
            let _s = coverage::build_prev_sibling_index(&tree);
            let _d = coverage::build_tree_id_index(&tree);
            let cost = t.elapsed().as_secs_f64();
            total_extract_work += cost;
            if is_miss {
                miss_count += 1;
                miss_cost += cost;
                miss_tree_sizes.push(tree.len());
            }
        }
        let avg_miss_tree = if miss_count > 0 {
            miss_tree_sizes.iter().sum::<usize>() as f64 / miss_count as f64
        } else { 0.0 };
        let max_miss_tree = miss_tree_sizes.iter().copied().max().unwrap_or(0);
        eprintln!("\n===== TreeKey-miss 成本分布（CL OOS 16K，L2）=====");
        eprintln!("miss={miss_count}/{n}（{:.2}%）", 100.0 * miss_count as f64 / n as f64);
        eprintln!("miss 累积重建成本={miss_cost:.4}s；全 bar 若无缓存总重建={total_extract_work:.4}s");
        eprintln!("miss 成本占全量重建={:.1}%（其余 {:.1}% 被缓存命中省下）",
            100.0 * miss_cost / total_extract_work, 100.0 * (1.0 - miss_cost / total_extract_work));
        eprintln!("miss tree.len：avg={avg_miss_tree:.0} max={max_miss_tree}");
        eprintln!("miss tree.len 末 10：{:?}", &miss_tree_sizes[miss_tree_sizes.len().saturating_sub(10)..]);
    }

    /// **★工位 4g 真因隔离：`TreeKey::of` per-bar 标度**（L2）。
    ///
    /// miss 成本仅 0.1%（diag_treekey_miss_cost_16k 坐实），∴ exp≈2.0 不来自 miss 重建。嫌疑：
    /// cached hit 路径**每 bar 无条件 `TreeKey::of(tower)`**（interp.rs:480 算 key 才能比较）——递归
    /// 发射全部 tree 节点 = O(confirmed tree)/bar，tree 单调增 ⟹ O(n²) 累积。这是 hit 路径里唯一随
    /// tree 增长的 O(tree) 工作（候选段 O(cand≤47)、role 查表 O(1)、tree/index 命中 Rc::clone O(1)）。
    /// 本 diag 测纯 `TreeKey::of` 累积标度——若 key_exp≈2 ⟹ 真因坐实（修复=指纹增量/缓存，非 extract 前缀重用）。
    #[test]
    #[ignore = "工位 4g：TreeKey::of per-bar 标度（真因隔离）；需 CL；--release --ignored"]
    fn diag_treekey_of_scaling_16k() {
        use super::super::super::strategy::interp;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let logexp = |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [2000usize, 4000, 8000, 16000];
        let mut prev: Option<(usize, f64)> = None;
        eprintln!("\n===== TreeKey::of per-bar 累积标度（CL OOS，L2）=====");
        eprintln!("{:>7} | {:>10} | {:>8}", "n", "key_s", "key_exp");
        for &n in &sizes {
            if n > oos.bars.len() { break; }
            let bars = &oos.bars[..n];
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let mut t_key = 0.0f64;
            let mut last_fp_len = 0usize;
            for i in 0..n {
                let (_cls, tower) = incr.classify_at(i);
                let t = std::time::Instant::now();
                let key = interp::TreeKey::of(&tower);
                t_key += t.elapsed().as_secs_f64();
                last_fp_len = key.fp_len();
            }
            let ke = prev.map(|(pn, pt)| logexp(pn, pt, n, t_key)).unwrap_or(f64::NAN);
            eprintln!("{n:>7} | {t_key:>10.4} | {ke:>8.2}  fp_len={last_fp_len}");
            prev = Some((n, t_key));
        }
    }

    /// **★工位 4g 验收：generation 快路 extract 标度（镜像生产 runner `_gen` 路径，L2）**。
    ///
    /// 镜像生产：`classify_at` + `tower_generation` → `_gen` 快路（代次未变跳过 `TreeKey::of`）。验收
    /// x_exp 从 ≈2.0（每 bar 全量 `TreeKey::of`）→ ≈1.0（代次命中 O(1)）。同时统计代次命中率
    /// （应 ≈99.8%，与 TreeKey-miss 26/16K 同源）。
    #[test]
    #[ignore = "工位 4g：generation 快路 extract 标度验收；需 CL；--release --ignored"]
    fn diag_gen_fastpath_extract_scaling_16k() {
        use super::super::super::strategy::interp;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let logexp = |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [2000usize, 4000, 8000, 16000];
        let mut prev: Option<(usize, f64)> = None;
        eprintln!("\n===== generation 快路 extract 标度（CL OOS，L2，镜像生产 _gen）=====");
        eprintln!("{:>7} | {:>10} | {:>8} {:>10}", "n", "extract_s", "x_exp", "gen_hit%");
        for &n in &sizes {
            if n > oos.bars.len() { break; }
            let bars = &oos.bars[..n];
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let mut tree_cache = interp::TreeCache::new();
            let mut t_extract = 0.0f64;
            let mut prev_gen: Option<u64> = None;
            let mut gen_hits = 0usize;
            for i in 0..n {
                let (cls, tower) = incr.classify_at(i);
                let gen = incr.tower_generation();
                let fe = incr.forest_epoch();
                if prev_gen == Some(fe) { gen_hits += 1; }
                prev_gen = Some(fe);
                let t = std::time::Instant::now();
                let (_tree, _cand, _gamma) =
                    interp::coverage_elements_and_gamma_with_tower_cached_gen(
                        &cls, &tower, &mut Some(&mut tree_cache), Some(gen), Some(fe),
                    );
                t_extract += t.elapsed().as_secs_f64();
            }
            let xe = prev.map(|(pn, pt)| logexp(pn, pt, n, t_extract)).unwrap_or(f64::NAN);
            eprintln!("{n:>7} | {t_extract:>10.4} | {xe:>8.2} {:>9.2}%", 100.0 * gen_hits as f64 / n as f64);
            prev = Some((n, t_extract));
        }
    }

    /// ★on2w2 诊断（临时）：forest_epoch bump 率 vs of_forest 真变率对拍。
    #[test]
    #[ignore = "on2w2 诊断：epoch bump 率 vs 真变率；需 CL；--release --ignored"]
    fn diag_forest_epoch_bump_vs_true_change() {
        use super::super::super::strategy::{coverage, interp::TreeKey};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 8000.min(oos.bars.len());
        let mut incr = super::IncrementalClassifier::new(&oos.bars[..n], &config);
        let mut prev_fe: Option<u64> = None;
        let mut prev_fp: Option<TreeKey> = None;
        let mut epoch_bumps = 0usize;
        let mut true_changes = 0usize;
        let mut false_hits = 0usize; // epoch 相等但森林真变（=假命中，soundness 破裂）
        for i in 0..n {
            let (_cls, tower) = incr.classify_at(i);
            let fe = incr.forest_epoch();
            let fp = TreeKey::of_forest(&tower);
            let _forest = coverage::extract_carrier_forest(&tower);
            let epoch_changed = prev_fe.map(|p| p != fe).unwrap_or(true);
            let fp_changed = prev_fp.as_ref().map(|p| *p != fp).unwrap_or(true);
            if epoch_changed { epoch_bumps += 1; }
            if fp_changed { true_changes += 1; }
            if !epoch_changed && fp_changed { false_hits += 1; }
            prev_fe = Some(fe);
            prev_fp = Some(fp);
        }
        // ★on2w2 forest 段隔离计时：只量 tree_segment（forest 命中/重建），排除 candidate/gamma 段。
        // before（of_forest 每 bar O(全塔)）= 传 None；after（epoch O(1) 命中）= 传 Some(fe)。
        {
            use super::super::super::strategy::interp::TreeCache;
            let mut before = super::IncrementalClassifier::new(&oos.bars[..n], &config);
            let mut tc_b = TreeCache::new();
            let t = std::time::Instant::now();
            for i in 0..n {
                let (cls, tower) = before.classify_at(i);
                // None ⟹ of_forest 指纹判据（实装前形态）。
                let _ = coverage::extract_carrier_forest(&tower); // 保底触达（None miss 时同）
                let _ = super::super::super::strategy::interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &cls, &tower, &mut Some(&mut tc_b), None, None);
            }
            let t_before = t.elapsed().as_secs_f64();
            let mut after = super::IncrementalClassifier::new(&oos.bars[..n], &config);
            let mut tc_a = TreeCache::new();
            let t = std::time::Instant::now();
            for i in 0..n {
                let (cls, tower) = after.classify_at(i);
                let fe = after.forest_epoch();
                let _ = super::super::super::strategy::interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &cls, &tower, &mut Some(&mut tc_a), None, Some(fe));
            }
            let t_after = t.elapsed().as_secs_f64();
            eprintln!("forest 段计时（含 candidate 段共同基底）：before(None/of_forest)={t_before:.4}s after(Some/epoch)={t_after:.4}s ({:.2}x)",
                t_before / t_after.max(1e-9));
        }
        let pr = super::super::super::classifier::oracle_probe::snapshot();
        eprintln!("\n===== on2w2 epoch bump vs 真变（CL {n}）=====");
        eprintln!("epoch_bumps={epoch_bumps} ({:.2}%) | true_changes(of_forest)={true_changes} ({:.2}%) | false_hits={false_hits}",
            100.0*epoch_bumps as f64/n as f64, 100.0*true_changes as f64/n as f64);
        eprintln!("E-site 分解：calls={} fd_any={} | E1(l0)={} E2(extend)={} E3(cascade)={}",
            pr.fd_calls, pr.fd_any, pr.fd_l0, pr.fd_extend, pr.fd_cascade);
    }

    /// **★工位 4g 候选段隔离：gen 快路命中后剩余工作（candidate 段）标度**（L2）。
    /// gen 快路 tree/index 全 O(1)，剩 candidate 段重建。测其单独标度——若 cand_exp≈1 则候选段非真因
    /// （extract exp=2 是测量噪声）；若 cand_exp≈2 则候选段内有隐藏 O(tree)。
    #[test]
    #[ignore = "工位 4g：候选段隔离标度；需 CL；--release --ignored"]
    fn diag_candidate_segment_scaling_16k() {
        use super::super::super::strategy::interp;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let logexp = |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [2000usize, 4000, 8000, 16000];
        let mut prev: Option<(usize, f64)> = None;
        eprintln!("\n===== 候选段隔离标度（gen 快路命中，CL OOS，L2）=====");
        eprintln!("{:>7} | {:>10} | {:>8} {:>10}", "n", "cand_s", "cand_exp", "cand_last");
        for &n in &sizes {
            if n > oos.bars.len() { break; }
            let bars = &oos.bars[..n];
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let mut tree_cache = interp::TreeCache::new();
            let mut t_cand = 0.0f64;
            let mut last_cand = 0usize;
            // 预热第一个 bar（填缓存），之后测命中路径的总时（含候选段）。
            for i in 0..n {
                let (cls, tower) = incr.classify_at(i);
                let gen = incr.tower_generation();
                let fe = incr.forest_epoch();
                let t = std::time::Instant::now();
                let (_t, cand, _g) = interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &cls, &tower, &mut Some(&mut tree_cache), Some(gen), Some(fe));
                t_cand += t.elapsed().as_secs_f64();
                last_cand = cand.len();
            }
            let ce = prev.map(|(pn, pt)| logexp(pn, pt, n, t_cand)).unwrap_or(f64::NAN);
            // 用 nocache None 模式对比（每 bar 全量 extract+index+候选，作 O(n²) 上界基底）。
            let mut incr2 = super::IncrementalClassifier::new(bars, &config);
            let mut t_full = 0.0f64;
            for i in 0..n {
                let (cls, tower) = incr2.classify_at(i);
                let t = std::time::Instant::now();
                let _ = interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &cls, &tower, &mut None, None, None);
                t_full += t.elapsed().as_secs_f64();
            }
            eprintln!("{n:>7} | {t_cand:>10.4} | {ce:>8.2} {last_cand:>9}  full_nocache={t_full:.4}");
            prev = Some((n, t_cand));
        }
    }

    /// **★工位 4g codex S(N) vs R(N)：candidate 段表示层 vs 数学下界判定**（L2）。
    /// codex 框架（为 tree 设计，此处套 candidate 真凶）：S=Σ每 bar 候选总数（全量重建工作量）；
    /// R=Σ每 bar 相对上 bar **新增/变化**的候选数（真增量工作量，按 (level,source_index,bits) 身份 diff）。
    /// S 大 R 小 ⟹ 表示层可修（candidate 前缀复用 O(R)）；R=Θ(S) ⟹ 候选每 bar 真变全部（定义冲突，escalate）。
    #[test]
    #[ignore = "工位 4g：candidate S(N) vs R(N) 表示层判定；需 CL；--release --ignored"]
    fn diag_candidate_s_vs_r_16k() {
        use std::collections::HashSet;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 16_000.min(oos.bars.len());
        let mut incr = super::IncrementalClassifier::new(&oos.bars[..n], &config);
        let mut s_total = 0u64;
        let mut r_total = 0u64;
        let mut prev: HashSet<(usize, usize, u8, u8)> = HashSet::new();
        for i in 0..n {
            let (cls, _tower) = incr.classify_at(i);
            let mut cur: HashSet<(usize, usize, u8, u8)> = HashSet::new();
            for (lvl, level) in cls.levels.iter().enumerate() {
                for p in level.bsp.iter() {
                    let b = &p.bits;
                    let buy = (b.buy1 as u8) | (b.buy2 as u8) << 1 | (b.buy3 as u8) << 2;
                    let sell = (b.sell1 as u8) | (b.sell2 as u8) << 1 | (b.sell3 as u8) << 2;
                    cur.insert((lvl, p.source_index, buy, sell));
                }
            }
            s_total += cur.len() as u64;
            r_total += cur.difference(&prev).count() as u64;
            prev = cur;
        }
        eprintln!("\n===== candidate S(N) vs R(N)（CL OOS 16K，L2，codex 框架）=====");
        eprintln!("S(N)=Σ候选总数={s_total}（全量重建工作量）");
        eprintln!("R(N)=Σ新增/变化候选={r_total}（真增量工作量）");
        eprintln!("R/S={:.4}（<<1 ⟹ 表示层可修前缀复用 O(R)；≈1 ⟹ 每 bar 真变全部=定义冲突）",
            r_total as f64 / s_total.max(1) as f64);
    }

    /// **★工位 4g bit-exact 守卫（debug 模式 debug_assert 生效）：generation 快路逐 bar == nocache**。
    ///
    /// 生产 `_gen` 路径的 debug_assert 在 release 不生效。本测试**默认 debug 构建**跑——代次快路命中时
    /// debug_assert_eq! 逐 bar 对比 cached 树 vs `extract_elements(tower)`，任何 generation 维护遗漏
    /// （假命中返陈旧树，codex Q3 列的漏洞）立即 panic。覆盖真实 CL frontier 古怪线段重划（bar 1464 类）。
    #[test]
    #[ignore = "工位 4g：generation 快路 debug bit-exact 守卫；需 CL；debug 构建 --ignored"]
    fn gen_fastpath_bit_exact_debug() {
        use super::super::super::strategy::{coverage, interp};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 4000.min(oos.bars.len()); // debug 慢，4K 含足够 cascade（古怪线段）覆盖
        let mut incr = super::IncrementalClassifier::new(&oos.bars[..n], &config);
        let mut tree_cache = interp::TreeCache::new();
        let mut gen_hits = 0usize;
        let mut prev_gen: Option<u64> = None;
        for i in 0..n {
            let (cls, tower) = incr.classify_at(i);
            let gen = incr.tower_generation();
            let fe = incr.forest_epoch();
            let prev_gen_snap = prev_gen;
            if prev_gen == Some(fe) { gen_hits += 1; }
            prev_gen = Some(fe);
            let _ = prev_gen_snap;
            // ★on2w2：K_i 命中判据 = forest_epoch。debug_assert_eq 内部对比 epoch 命中树 ==
            // extract_carrier_forest。此处再显式对 extract_elements（T_i）——注意 cached_gen 的 tree
            // 段现是 K_i（extract_carrier_forest），故显式对拍改用 carrier_forest（与生产判据同源）。
            let expect = coverage::extract_carrier_forest(&tower);
            let (tree, _cand, _gamma) =
                interp::coverage_elements_and_gamma_with_tower_cached_gen(
                    &cls, &tower, &mut Some(&mut tree_cache), Some(gen), Some(fe));
            // 显式再断一遍（不依赖 debug_assert，release 也保护本测试）。
            if tree.as_ref() != &expect {
                eprintln!("DIVERGE bar {i}：gen={gen} prev_gen={prev_gen_snap:?} cached_len={} expect_len={} tower_levels={}",
                    tree.len(), expect.len(), tower.len());
                for (j, (a, b)) in tree.iter().zip(expect.iter()).enumerate() {
                    if a != b { eprintln!("  first diff idx {j}: cached={a:?}\n             expect={b:?}"); break; }
                }
                panic!("bar {i}：generation 快路返陈旧树（gen 维护遗漏变异点，codex Q3）");
            }
        }
        eprintln!("gen_fastpath_bit_exact：{n} bars 全部 gen 快路==nocache，gen 命中 {gen_hits} bars");
    }

    /// **★工位 4d L2 证据：`coverage_step` 路径标度（热点①② 消除验收）**。
    ///
    /// 工位 4c 的 `diag_strategy_hotspot_decompose_16k` 只测 extract（candidate 段重建）+ merge（③），
    /// **不覆盖** `coverage_step_from_buckets`（热点① `strategy_target_legs` 兄弟索引 + ② `held_leg_tree_index`
    /// ID 索引所在）。本 diag 镜像生产 runner 的完整 step 路径——`coverage_elements_and_gamma_with_tower_cached`
    /// + 注入缓存 base 索引（[`coverage::ElementView::with_base_indices`]）+ `coverage_step_prebuilt`——
    /// 测 step 总时间标度 `step_exp`，验收①② 缓存命中后 base 段 O(1)（不再每 bar `build_*_index` O(tree)）。
    ///
    /// 诚实诊断（formalization-validity-domain 231号 L2）：step 路径仍含 **candidate 段重建**（每 bar
    /// candidate ∝ confirmed，§16 不可缓存——candidate 随 bar 变）+ 生产 AncOK 闭包（#183 归一后 =
    /// `exit::step_active_set_with_subtree_close`，raw 闭包 O(raw)）
    /// + `strategy_target_legs` 遍历 active（O(active)）。①②缓存只消除 base 段索引重建，candidate/active
    /// 遍历是 step 的内禀工作量（非重复重建）。step_exp 反映这些残留的真实标度——若仍 >1.5 诚实报告，
    /// 不强声明①② 已让 step≈1.0（candidate 重建是独立残留，非①②）。
    #[test]
    #[ignore = "工位 4d L2：coverage_step 路径标度（①② 缓存消除验收）；需 CL；--release"]
    fn diag_coverage_step_scaling_16k() {
        use super::super::super::strategy::{coverage, interp, persistent};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let voice = config.voice.clone();
        eprintln!("\n===== coverage_step 路径标度（CL OOS，L2，①② 缓存注入）=====");
        eprintln!("{:>7} | {:>10} | {:>8}", "n", "step_s", "step_exp");
        let logexp = |n0: usize, t0: f64, n1: usize, t1: f64| (t1 / t0).ln() / (n1 as f64 / n0 as f64).ln();
        let sizes = [2000usize, 4000, 8000, 16000];
        let mut prev: Option<(usize, f64)> = None;
        for &n in &sizes {
            if n > oos.bars.len() { break; }
            let bars = &oos.bars[..n];
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let mut registry = persistent::PersistentRegistry::new();
            let mut tree_cache = interp::TreeCache::new();
            let mut prev_active: Vec<interp::ActiveLeg> = Vec::new();
            let mut t_step = 0.0f64;
            let mut last_gamma = 0usize;
            let mut last_work_base = 0usize;
            for i in 0..n {
                let (cls, tower) = incr.classify_at(i);
                let (tree, candidates, gamma) =
                    interp::coverage_elements_and_gamma_with_tower_cached(
                        &cls, &tower, &mut Some(&mut tree_cache));
                last_gamma = gamma.len();
                last_work_base = tree.len();
                // 生产路径：注入缓存 base 索引（①② O(1) 命中）。
                let mut work = coverage::ElementView::from_parts(&tree, candidates);
                if let Some((sib, id)) = tree_cache.tree_sibling_and_id() {
                    work = work.with_base_indices(sib, id);
                }
                let t = std::time::Instant::now();
                let (next_active, _p) =
                    coverage::coverage_step_prebuilt(work, &gamma, &prev_active, 1000.0, &voice, None, &registry);
                t_step += t.elapsed().as_secs_f64();
                // merge 用纯 tree snapshot（与生产 candidate 含量差异不影响①② step 标度测量）。
                registry.merge_in_place(
                    coverage::ElementView::from_parts(&tree, Vec::new()).as_contiguous().as_ref(),
                    &next_active);
                prev_active = next_active;
            }
            let se = prev.map(|(pn, pt)| logexp(pn, pt, n, t_step)).unwrap_or(f64::NAN);
            eprintln!("{n:>7} | {t_step:>10.3} | {se:>8.2}  registry_len={} prev_active_last={} gamma_last={} work_base_last={}", registry.len(), prev_active.len(), last_gamma, last_work_base);
            prev = Some((n, t_step));
        }
    }

    #[test]
    #[ignore = "profile: 增量 vs legacy 标度；需 CL；--release"]
    fn profile_incremental_vs_legacy_scaling() {
        let config = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        eprintln!("\n===== Profile: 增量链 vs legacy 全量标度（CL OOS）=====");
        eprintln!(
            "{:>8} {:>10} {:>10} {:>8} {:>8}",
            "n_bars", "incr_s", "legacy_s", "i_exp", "l_exp"
        );
        let sizes = [1000usize, 2000, 4000, 8000];
        let mut prev: Option<(usize, f64, f64)> = None;
        for &n in &sizes {
            if n > oos.bars.len() {
                break;
            }
            let bars = &oos.bars[..n];

            // 增量路径。
            let mut incr = super::IncrementalClassifier::new(bars, &config);
            let t0 = std::time::Instant::now();
            for i in 0..n {
                let _ = incr.classify_at(i);
            }
            let incr_dt = t0.elapsed().as_secs_f64();

            // legacy 路径（parse + classify 每 bar 全算）。
            let t1 = std::time::Instant::now();
            for i in 0..n {
                let l0 = parser::parse_layer(&bars[..=i], &config);
                let _ = classifier::classify_with_tower(&l0, &config);
            }
            let leg_dt = t1.elapsed().as_secs_f64();

            let (i_exp, l_exp) = prev
                .map(|(pn, pi, pl)| {
                    let r = (n as f64 / pn as f64).ln();
                    ((incr_dt / pi).ln() / r, (leg_dt / pl).ln() / r)
                })
                .unwrap_or((f64::NAN, f64::NAN));
            eprintln!(
                "{n:>8} {incr_dt:>10.3} {leg_dt:>10.3} {i_exp:>8.2} {l_exp:>8.2}",
            );
            prev = Some((n, incr_dt, leg_dt));
        }
        eprintln!(
            "\n判读：i_exp（增量总标度）应低于 l_exp（legacy 全量）。\n  \
             增量收益 = inclusion O(1)/bar + 塔构造 resume + MACD 增量；下游 fractal/stroke/segment\n  \
             仍全量重算（parser 工位约束），故 i_exp 不到 1.0。\n  \
             ★身份稳定（非标度）：TowerCache 跨 bar 复用 → LeveledMove 身份连续 → Stale 降根。"
        );
    }

}
