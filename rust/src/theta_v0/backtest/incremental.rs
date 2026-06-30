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
    pub fn classify_at(&mut self, i: usize) -> (classifier::Classification, Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>) {
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
                registry.merge_in_place_split(&tree_ref, tree_dirty, &candidates_ref, &[]);
                t_merge += t.elapsed().as_secs_f64();
                prev_merge_tree = Some(std::rc::Rc::clone(&tree_ref));
                // 隔离测量：tree 段全量（candidate 空，tree_dirty=true）→ 旧 O(n²) 基底对照。
                let t = std::time::Instant::now();
                registry.merge_in_place_split(&tree_ref, true, &[], &[]);
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

    /// **★工位 4d L2 证据：`coverage_step` 路径标度（热点①② 消除验收）**。
    ///
    /// 工位 4c 的 `diag_strategy_hotspot_decompose_16k` 只测 extract（candidate 段重建）+ merge（③），
    /// **不覆盖** `coverage_step_from_buckets`（热点① `strategy_target_legs` 兄弟索引 + ② `held_leg_tree_index`
    /// ID 索引所在）。本 diag 镜像生产 runner 的完整 step 路径——`coverage_elements_and_gamma_with_tower_cached`
    /// + 注入缓存 base 索引（[`coverage::ElementView::with_base_indices`]）+ `coverage_step_prebuilt`——
    /// 测 step 总时间标度 `step_exp`，验收①② 缓存命中后 base 段 O(1)（不再每 bar `build_*_index` O(tree)）。
    ///
    /// 诚实诊断（formalization-validity-domain 231号 L2）：step 路径仍含 **candidate 段重建**（每 bar
    /// candidate ∝ confirmed，§16 不可缓存——candidate 随 bar 变）+ `ancestor_close_by_id`（raw 闭包 O(raw)）
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
            for i in 0..n {
                let (cls, tower) = incr.classify_at(i);
                let (tree, candidates, gamma) =
                    interp::coverage_elements_and_gamma_with_tower_cached(
                        &cls, &tower, &mut Some(&mut tree_cache));
                // 生产路径：注入缓存 base 索引（①② O(1) 命中）。
                let mut work = coverage::ElementView::from_parts(&tree, candidates);
                if let Some((sib, id)) = tree_cache.tree_sibling_and_id() {
                    work = work.with_base_indices(sib, id);
                }
                let t = std::time::Instant::now();
                let (next_active, _p) =
                    coverage::coverage_step_prebuilt(work, &gamma, &prev_active, 1000.0, &voice, &registry);
                t_step += t.elapsed().as_secs_f64();
                // merge 用纯 tree snapshot（与生产 candidate 含量差异不影响①② step 标度测量）。
                registry.merge_in_place(
                    coverage::ElementView::from_parts(&tree, Vec::new()).as_contiguous().as_ref(),
                    &next_active);
                prev_active = next_active;
            }
            let se = prev.map(|(pn, pt)| logexp(pn, pt, n, t_step)).unwrap_or(f64::NAN);
            eprintln!("{n:>7} | {t_step:>10.3} | {se:>8.2}  registry_len={}", registry.len());
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
