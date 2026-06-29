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
    pub fn classify_at(&mut self, i: usize) -> (classifier::Classification, Vec<Vec<classifier::recursive_tower::LeveledMove>>) {
        debug_assert!(i < self.bars.len(), "classify_at({i}) 越界 bars.len={}", self.bars.len());
        // 增量 parse：append bar i（O(1) inclusion + O(merged_i) 下游）。
        let l0_i = self.parser_incr.append(self.bars[i]);
        // 增量塔：cache 跨 bar 复用（身份稳定），bit-exact == 全量 classify_with_tower。
        classifier::classify_with_tower_incremental(&l0_i, self.config, &mut self.tower_cache)
    }
}

// ════════════════════════════════════════════════════════════════════════════
// 测试 + profile（cfg(test) 门控，backtest 整模块本就 cfg(test)，此处显式标注）
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

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
        // 8K 控时（O(n²) 双跑：增量 + legacy 对照，各一倍；8K≈2s/跑，~4s 总）。
        let n = 8_000.min(oos.bars.len());
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

    /// **★bit-exact 合成数据验证（无需 CL，always-run）**：合成 bar 序列逐 bar 断言增量 == legacy。
    #[test]
    fn bit_exact_synthetic() {
        // 合成 2000 bar：缓慢上升趋势 + 周期性回撤（产足够段/中枢）。
        let bars: Vec<Bar> = (0..2000usize)
            .map(|i| {
                let base = 1000i64 + (i as i64) * 2;
                let cycle = ((i as f64) / 50.0).sin() as i64 * 30;
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
}

#[cfg(test)]
mod profile {
    use super::super::data;
    use super::super::super::config::ThetaConfig;
    use super::super::super::{classifier, parser};

    /// **★Profile：增量链 vs legacy 全量 标度对比**。
    ///
    /// 增量路径 = ParseLayerIncr::append + classify_with_tower_incremental（cache 跨 bar 复用）。
    /// 对比 legacy = parse_layer + classify_with_tower（每 bar 全算）。
    /// 增量总 exp 应低于 legacy（inclusion + 塔构造增量化），但下游 fractal/stroke/segment 仍全量
    /// 重算（parser 工位约束），故 exp 不会到 1.0。
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
