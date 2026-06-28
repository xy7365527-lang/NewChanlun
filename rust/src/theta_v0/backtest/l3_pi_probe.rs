//! **Phase-1 可行性探针**：`run_theta_v0_pi`（七链 π_Θ 生产 runner，σ_p=父容器方向 639 +
//! AncOK 持仓准入）的 substrate 时间标度 + 信号/交易计数实测。
//!
//! ## 为什么需要探针（probe-first，不盲跑）
//!
//! `run_theta_v0_pi` 的 substrate 是 **per-bar 前缀重分类**：fill loop 每 bar i 调
//! `classify_with_tower(parse_layer(&bars[..=i]))`（runner.rs:274-278）。`parse_layer`/
//! `classify_with_tower` 各自至少 O(i) ⟹ 全程 Σᵢ O(i) = **O(n²)**（runner.rs:439 自陈
//! "逐 bar 前缀重分类 = O(n²)"）。对比 v1 `run_theta_v0` 单趟 O(n)（signal.rs O(n²)→O(n)
//! 修复后全窗可行，见 l3_fullwindow.rs 头）——pi 即便单个 60K 截断窗也远慢于 v1。
//!
//! 真实 OOS 窗可达 ~10⁵–10⁶ bar（CL OOS≈数十万 1min bar）。**全窗 O(n²) 外推不可行**是
//! 待坐实的假设——本探针**实测**多窗口尺度的 wall-time，拟合标度指数，外推全窗总耗时，
//! 据此判定 Phase-2 用全窗还是可行子集。**严禁仅从代码读出 O(n²) 就声明**（memory
//! `signal-on2-two-independent-hotspots`：声明解 O(n²) 前须标度实测坐实）。
//!
//! ## 认识论等级
//!
//! 本探针测的是**管线时间复杂度 + 信号产出计数**（工程量），**非** Θ alpha（L2/L3 是
//! 下个测试）。时间/计数本身是 L1 工程观测；但"全窗是否可行"的判定直接决定 L2/L3 能否
//! 在全窗执行（有效域边界），故诚实记录。
//!
//! 跑法：`cargo test --release --lib theta_v0::backtest::l3_pi_probe -- --ignored --nocapture`

use super::data::{self, Dataset};
use super::runner::run_theta_v0_pi;
use super::super::config::ThetaConfig;

/// NAV 与品种价量级匹配（首可交易价×1000，下限 1e6）——与 l3_fullwindow 同口径。
fn nav_for(win: &Dataset, config: &ThetaConfig) -> f64 {
    let first_px = win
        .bars
        .iter()
        .find(|b| !b.untradable && b.close > 0)
        .map(|b| b.close as f64 * config.tick.tick_size)
        .unwrap_or(1.0);
    (first_px * 1000.0).max(1.0e6)
}

/// 取 OOS 窗前缀 n bar（source_index 已在 slice_date_window 重置为局部下标，前缀仍合法）。
fn prefix(oos: &Dataset, n: usize) -> Dataset {
    let n = n.min(oos.bars.len());
    Dataset {
        symbol: oos.symbol.clone(),
        bars: oos.bars[..n].to_vec(),
        dates: oos.dates[..n.min(oos.dates.len())].to_vec(),
    }
}

/// **★Phase-1 探针：单品种 pi substrate O(n²) 时间标度 + 信号/交易计数**。
///
/// `#[ignore]`，需 CL 数据缓存；`--release` 必须（O(n²) debug 不可测）。
#[test]
#[ignore = "Phase-1 probe: pi O(n²) substrate 时标；需数据缓存；--release"]
fn pi_probe_scaling_single_symbol() {
    let config = ThetaConfig::default();
    // 单品种探针：CL（核心池，OOS 历史充足；与 PREREG_WINDOWS OOS 起点一致防数据挖掘）。
    let symbol = "CL";
    let ds = match data::load_by_symbol(symbol, &config) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("DATA BLOCKER: 加载 {symbol} 失败：{e}");
            panic!("probe 需真实数据（不伪造合成冒充 L2）");
        }
    };
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    let avail = oos.bars.len();
    eprintln!("\n===== Phase-1 pi substrate 探针：symbol={symbol} OOS=[2023-01-01,2025-06-30] =====");
    eprintln!("OOS 全窗可用 bar 数 = {avail}（全窗 O(n²) 外推目标）");
    eprintln!(
        "{:>8} {:>10} {:>9} {:>7} {:>7} {:>9} {:>12}",
        "n_bars", "wall_s", "bars/s", "n_ord", "trd", "real_trd", "exp_vs_prev"
    );

    // 逐步增大窗口（O(n²) 下指数增长，从小起，超时预算即停——不盲跑到超时）。
    let sizes = [500usize, 1000, 2000, 4000, 8000, 16000];
    const BUDGET_S: f64 = 90.0; // 单窗墙钟预算；超即停增长（探针不盲跑）
    let mut prev: Option<(usize, f64)> = None;
    let mut measured: Vec<(usize, f64, usize, usize)> = Vec::new(); // (n, dt, real_trd, n_ord)

    for &n in &sizes {
        if n > avail {
            eprintln!("(n={n} > 可用 {avail}，停)");
            break;
        }
        let win = prefix(&oos, n);
        let nav = nav_for(&win, &config);
        let t0 = std::time::Instant::now();
        let res = run_theta_v0_pi(&win, &config, 1.0, nav);
        let dt = t0.elapsed().as_secs_f64();
        let n_real = res.trades.iter().filter(|t| !t.forced_close).count();
        let exp = prev.map(|(pn, pt)| {
            if pt > 1e-9 {
                (dt / pt).ln() / (n as f64 / pn as f64).ln()
            } else {
                f64::NAN
            }
        });
        eprintln!(
            "{:>8} {:>10.3} {:>9.0} {:>7} {:>7} {:>9} {:>12}",
            n,
            dt,
            n as f64 / dt.max(1e-9),
            res.n_orders,
            res.trade_pnls.len(),
            n_real,
            exp.map(|e| format!("{e:.2}")).unwrap_or_else(|| "—".into()),
        );
        measured.push((n, dt, n_real, res.n_orders));
        prev = Some((n, dt));
        if dt > BUDGET_S {
            eprintln!("(单窗 {dt:.1}s > 预算 {BUDGET_S}s，停止增长——避免盲跑超时)");
            break;
        }
    }

    // ── 标度拟合 + 全窗外推（用最后两个测点的幂律外推；O(n²) ⟹ exp≈2）──
    if measured.len() >= 2 {
        let (n1, t1, _, _) = measured[measured.len() - 2];
        let (n2, t2, _, _) = measured[measured.len() - 1];
        let exp = (t2 / t1).ln() / (n2 as f64 / n1 as f64).ln();
        let c = t2 / (n2 as f64).powf(exp); // t = c·n^exp
        let extrap_full = c * (avail as f64).powf(exp);
        eprintln!("\n----- 标度拟合（末两测点幂律）-----");
        eprintln!("拟合 exp ≈ {exp:.3}（O(n²) ⟹ ≈2.0）, 常数 c ≈ {c:.3e}");
        eprintln!(
            "外推全窗（n={avail}）单品种单次 pi ≈ {:.0}s ≈ {:.1}min ≈ {:.2}h",
            extrap_full,
            extrap_full / 60.0,
            extrap_full / 3600.0
        );
        eprintln!(
            "外推 8 品种全窗（粗略，各品种 OOS 量级不同，CL 量级代表核心池）≈ {:.1}h（单次）；\n  \
             双口径（含 depth=0 baseline 二次 pi）≈ {:.1}h；walk-forward 多窗 ×N ⟹ 远超",
            extrap_full * 8.0 / 3600.0,
            extrap_full * 8.0 * 2.0 / 3600.0,
        );
        // 信号密度（每 bar 真实交易率）——判样本饥饿（O(n²) 截断下样本量是否够 n≥30 统计门槛）
        let (ln, _lt, lreal, lord) = measured[measured.len() - 1];
        eprintln!("\n----- 信号密度（末测点 n={ln}）-----");
        eprintln!(
            "n_orders={lord} real_trades={lreal} ⟹ 每 1000 bar 真实交易≈{:.2}；\n  \
             外推全窗 real_trades≈{:.0}（统计门槛 n≥30：{}）",
            lreal as f64 / ln as f64 * 1000.0,
            lreal as f64 / ln as f64 * avail as f64,
            if (lreal as f64 / ln as f64 * avail as f64) >= 30.0 {
                "全窗可达"
            } else {
                "全窗或不达——样本饥饿风险"
            },
        );
    }

    // 探针不变量：至少跑通最小窗（管线在真实数据上不崩）。
    assert!(!measured.is_empty(), "探针至少完成最小窗（pi 管线真实数据跑通）");
    let (_, _, _, n_ord_min) = measured[0];
    eprintln!(
        "\n探针完成：{} 个尺度测点。最小窗 n_orders={n_ord_min}（>0 ⟹ pi 产订单流，L2 有输入）",
        measured.len()
    );
}

/// 单次全窗 classify（O(n) 单趟，非 per-bar O(n²)）的结构统计——塔层数 + 逐级 bsp/moves 计数。
fn classify_structure_stats(win: &Dataset, config: &ThetaConfig) -> (usize, usize, usize, Vec<usize>) {
    use super::super::{classifier, parser};
    let l0 = parser::parse_layer(&win.bars, config);
    let (classification, tower) = classifier::classify_with_tower(&l0, config);
    let tower_depth = tower.len();
    let total_bsp: usize = classification.levels.iter().map(|lv| lv.bsp.len()).sum();
    let total_moves: usize = classification.levels.iter().map(|lv| lv.moves.len()).sum();
    let per_level_bsp: Vec<usize> = classification.levels.iter().map(|lv| lv.bsp.len()).collect();
    (tower_depth, total_bsp, total_moves, per_level_bsp)
}

/// **★Phase-1 GATE 探针（path-1 only，O(n) 单趟，避开 O(n²) path-2 盲跑）**：大窗全窗
/// `classify_with_tower` 数塔层数 + 逐级 bsp/centers/moves，判定 **(a) 引擎在真实数据空** vs
/// **(c) 结构饥饿（incremental 塔可解锁）**。
///
/// 判据（task 阶段1）：大窗 **total_bsp>0 ∧ tower_depth≥2** ⟹ **(c) 结构饥饿**——多级父容器
/// 存在 + 买卖点非空，per-bar 因果 σ_p + AncOK 剪枝是 0-orders 主因，incremental 持久身份塔
/// 能解锁 → 进阶段2/3。反之（bsp=0 ∨ depth<2）⟹ **(a) 分类器真实数据空**，建塔不解锁 → 停。
///
/// 只跑 O(n) path-1（结构统计），**不**跑 O(n²) per-bar pi（path-2，单 32K 窗可能 >10min；
/// gate 判定不需要它——大窗 pi 本就因 O(n²) 不可行，这正是要修的）。
#[test]
#[ignore = "Phase-1 GATE: (a)引擎空 vs (c)结构饥饿；path-1 only O(n)；需 CL+BTC 缓存；--release"]
fn pi_probe_phase1_gate_structure() {
    use super::super::classifier::{self, bsp::BspPoint};
    use super::super::parser;

    let config = ThetaConfig::default();
    eprintln!("\n========== Phase-1 GATE：(a)引擎空 vs (c)结构饥饿（path-1 O(n) 单趟）==========");

    /// 逐级 (centers, moves, bsp) + bsp 类别拆分（buy1/2/3, sell1/2/3）。
    fn rich_stats(win: &Dataset, config: &ThetaConfig) -> (usize, Vec<(usize, usize, usize)>, [usize; 6]) {
        let l0 = parser::parse_layer(&win.bars, config);
        let (classification, tower) = classifier::classify_with_tower(&l0, config);
        let per: Vec<(usize, usize, usize)> = classification
            .levels
            .iter()
            .map(|lv| (lv.centers.len(), lv.moves.len(), lv.bsp.len()))
            .collect();
        let mut cls = [0usize; 6]; // b1,b2,b3,s1,s2,s3
        let count = |b: &BspPoint, cls: &mut [usize; 6]| {
            if b.bits.buy1 { cls[0] += 1; }
            if b.bits.buy2 { cls[1] += 1; }
            if b.bits.buy3 { cls[2] += 1; }
            if b.bits.sell1 { cls[3] += 1; }
            if b.bits.sell2 { cls[4] += 1; }
            if b.bits.sell3 { cls[5] += 1; }
        };
        for lv in &classification.levels {
            for b in &lv.bsp { count(b, &mut cls); }
        }
        (tower.len(), per, cls)
    }

    let mut any_c = false;
    let mut verdicts: Vec<(String, usize, &'static str)> = Vec::new();

    for (sym, oos_win) in [("CL", ("2023-01-01", "2025-06-30")), ("BTC", ("2023-01-01", "2025-06-30"))] {
        let ds = match data::load_by_symbol(sym, &config) {
            Ok(d) => d,
            Err(e) => { eprintln!("{sym} 加载失败：{e}（跳过）"); continue; }
        };
        let oos = ds.slice_date_window(oos_win.0, oos_win.1);
        let avail = oos.bars.len();
        eprintln!("\n----- {sym} OOS=[{},{}] 可用 {avail} bar -----", oos_win.0, oos_win.1);
        eprintln!("{:>8} {:>6} {:>9} {:>9} {:>9} {:>9}  {}", "n_bars", "塔层", "tower_s", "Σcenters", "Σmoves", "Σbsp", "per_level(c,mv,bsp)");
        for &n in &[4_000usize, 16_000, 64_000, 256_000] {
            if n > avail { eprintln!("(n={n} > {avail}，停)"); break; }
            let win = prefix(&oos, n);
            let t0 = std::time::Instant::now();
            let (depth, per, cls) = rich_stats(&win, &config);
            let dt = t0.elapsed().as_secs_f64();
            let sum_c: usize = per.iter().map(|x| x.0).sum();
            let sum_mv: usize = per.iter().map(|x| x.1).sum();
            let sum_bsp: usize = per.iter().map(|x| x.2).sum();
            eprintln!("{n:>8} {depth:>6} {dt:>8.2}s {sum_c:>9} {sum_mv:>9} {sum_bsp:>9}  {per:?}");
            eprintln!("         bsp类别 b1={} b2={} b3={} s1={} s2={} s3={}", cls[0], cls[1], cls[2], cls[3], cls[4], cls[5]);
            // gate 判据：本窗 bsp>0 ∧ depth>=2 ⟹ (c) 候选（取最大窗的判定为准）。
            let verdict = if sum_bsp > 0 && depth >= 2 { "(c)结构饥饿" } else if sum_bsp == 0 { "(a)bsp空" } else { "(a)单级塔" };
            verdicts.push((format!("{sym}@{n}"), depth, verdict));
            if sum_bsp > 0 && depth >= 2 { any_c = true; }
        }
    }

    eprintln!("\n========== GATE 判定汇总 ==========");
    for (tag, depth, v) in &verdicts {
        eprintln!("  {tag:<12} depth={depth} → {v}");
    }
    eprintln!(
        "\n★最终判定：{}",
        if any_c {
            "(c) 结构饥饿——大窗 total_bsp>0 ∧ tower_depth≥2：多级父容器+买卖点存在。\n  \
             per-bar pi 0-orders 主因 = 因果 σ_p + AncOK 剪枝 + per-bar 重分类 ρ 漂移。\n  \
             ⟹ incremental 持久身份塔可解锁 → 进阶段2（增量原语勘查）/阶段3（建塔）。"
        } else {
            "(a) 引擎在真实数据空——大窗 bsp=0 或单级塔。incremental 塔不解锁交易\n  \
             （更深的分类器缺口，另立工位）。停。"
        }
    );
    assert!(!verdicts.is_empty(), "GATE 探针至少完成一个品种一个窗（真实数据跑通）");
}

/// **★Phase-2 可行性探针：theta_v0 parse+classify 管线的「稳定前缀」性质实测**。
///
/// incremental 塔（摊还 O(1) extend）的**充要前提**：classify(prefix[..=i]) 随 i 增长，其
/// confirmed 结构（bsp）只在**有界尾部**修订，confirmed 前缀**永久固定**（== legacy
/// `SegCheckpoint.stable_count` 的 theta_v0 对应物）。本探针实测此性质——**不从读码推测**
/// （memory signal-on2/escalate-requires-l2-evidence：声明结构性质前须机器坐实）。
///
/// 测法：逐步增窗 prefix[..=cut]（cut 递增），把 bsp 扁平为 `(level, source_index, bits)` 按
/// source_index 排序。比较相邻两窗：
/// - **stable_si**：最大 source_index 使 ≤ 它的全部 bsp 在两窗 bit-identical（永久固定边界）。
/// - **drift**：上一窗有但当前窗变了/没了的 bsp 数（source_index ≤ 上窗末候选——非新增尾）。
///
/// 判读：stable_si 单调逼近 cut−bounded_margin（drift 局限有界尾）⟹ **稳定前缀成立**，
/// incremental 可行。stable_si 远落后 cut 或 drift 无界 ⟹ 早段 bsp 随窗变（强 ρ 漂移）⟹
/// 简单 incremental（缓存前缀+扩尾）**不 bit-exact**，需更深算法重设计（报告缺口）。
///
/// `#[ignore]`，需 CL 缓存；`--release`。
#[test]
#[ignore = "Phase-2 可行性：parse+classify 稳定前缀性质实测；需 CL 数据；--release"]
fn pi_probe_phase2_stable_prefix() {
    use super::super::classifier;
    use super::super::parser;
    use super::super::types::BspBits;

    let config = ThetaConfig::default();
    eprintln!("\n========== Phase-2 可行性：parse+classify 稳定前缀性质（incremental 充要前提）==========");

    /// bsp 扁平为 (level, source_index, bits) 按 source_index 升序（同 si 多 level 保留）。
    fn flat_bsp(win: &Dataset, config: &ThetaConfig) -> Vec<(u32, usize, BspBits)> {
        let l0 = parser::parse_layer(&win.bars, config);
        let (cls, _) = classifier::classify_with_tower(&l0, config);
        let mut v: Vec<(u32, usize, BspBits)> = Vec::new();
        for (lvl, ls) in cls.levels.iter().enumerate() {
            for b in &ls.bsp {
                v.push((lvl as u32, b.source_index, b.bits));
            }
        }
        v.sort_by_key(|(lvl, si, _)| (*si, *lvl));
        v
    }

    let sym = "CL";
    let ds = match data::load_by_symbol(sym, &config) {
        Ok(d) => d,
        Err(e) => { eprintln!("DATA BLOCKER: {sym} 加载失败：{e}"); panic!("需真实数据"); }
    };
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    // 中等窗逐步增长（每步 classify 单趟 O(cut)，cut≤24K 单趟亚秒；测稳定性不需大窗）。
    let cuts: Vec<usize> = (2_000..=24_000).step_by(2_000).collect();
    eprintln!("{:>8} {:>8} {:>10} {:>10} {:>9} {:>10}", "cut", "n_bsp", "stable_si", "cut-stbl", "drift", "new_tail");
    let mut prev: Option<Vec<(u32, usize, BspBits)>> = None;
    let mut prev_cut = 0usize;
    let mut max_drift = 0usize;
    let mut max_margin = 0usize;
    for &cut in &cuts {
        if cut > oos.bars.len() { break; }
        let win = prefix(&oos, cut);
        let cur = flat_bsp(&win, &config);
        if let Some(p) = &prev {
            // stable_si：从小 source_index 起两序列逐元素相等的最长公共前缀的末 source_index。
            let mut i = 0usize;
            while i < p.len() && i < cur.len() && p[i] == cur[i] {
                i += 1;
            }
            let stable_si = if i == 0 { 0 } else { cur[i - 1].1 };
            // drift：上窗中 source_index ≤ prev_cut 但不在当前窗（同 level+si+bits）的 bsp 数
            //（= 早段被窗增长改写的候选数；理想为 0——confirmed 前缀永久固定）。
            // BspBits 非 Hash ⟹ 线性 contains（bsp 数百，可接受）。
            let drift = p
                .iter()
                .filter(|(_, si, _)| *si <= prev_cut)
                .filter(|t| !cur.contains(*t))
                .count();
            // new_tail：当前窗新增（source_index > prev_cut）的 bsp 数（合法增长，不算 drift）。
            let new_tail = cur.iter().filter(|(_, si, _)| *si > prev_cut).count();
            let margin = cut.saturating_sub(stable_si);
            max_drift = max_drift.max(drift);
            max_margin = max_margin.max(margin);
            eprintln!("{cut:>8} {:>8} {stable_si:>10} {margin:>10} {drift:>9} {new_tail:>10}", cur.len());
        } else {
            eprintln!("{cut:>8} {:>8} {:>10} {:>10} {:>9} {:>10}", cur.len(), "—", "—", "—", "—");
        }
        prev = Some(cur);
        prev_cut = cut;
    }
    eprintln!(
        "\n判读：max_drift={max_drift}（理想 0=confirmed 前缀永久固定），max(cut−stable_si)={max_margin} bar\n  \
         （有界=易变尾窗口大小）。\n  \
         drift≈0 ∧ margin 有界 ⟹ **稳定前缀成立 → incremental 摊还 O(1) extend 可行（bit-exact）**。\n  \
         drift 大/无界 ⟹ 早段随窗改写（强 ρ 漂移）⟹ 简单缓存+扩尾不 bit-exact，需重设计（缺口）。"
    );
}

/// **★Phase-2 根因探针：bsp→订单转化失败定位（slice source_index==i 候选饥饿假设）**。
///
/// 编排者警示：per-bar pi 在 64K（bsp 存在）仍 n_orders=0 ⟹ 失败是 **bsp→订单转化**（坐标/ρ漂移），
/// 非纯性能。本探针实测旧 runner slice（`source_index==i`）是否候选饥饿：
///
/// ★已修复（本探针坐实的方向已实装）：runner 现用 delta（`runner::newly_confirmed_step` 确认-bar
/// 部署），非 slice。本探针保留为诊断对照（局部构造 slice/delta，独立于 runner 现口径）。
///
/// 逐 bar i 算 classify(prefix-i)，对比两种候选定义：
/// - **slice**（**旧** substrate）：bsp where `source_index==i`（"当步触发"假设——候选饥饿根因）。
/// - **delta**（因果新确认，**现 substrate**）：bsp ∈ classify(prefix-i) ∖ classify(prefix-(i-1))（本步新确认）。
///
/// 稳定前缀探针已证 confirmed bsp 的 source_index 落在 cut−[2000,4500] 的**已结算区**（非 ==cut）。
/// 故假设：Σ slice≈0（饥饿，确认买卖点 source_index≪i），Σ delta>0（新确认信号真存在，lag=i−source_index）。
/// 若坐实 ⟹ bsp→订单转化的根因是 **候选坐标错位**（slice 错挂 source_index==i，应挂"本步新确认"）
/// ⟹ incremental 塔的 delta 自然就是因果候选（性能与正确性同根）。
///
/// `#[ignore]`，需 CL 缓存；`--release`（O(N²)，用小窗 8K 控时）。
#[test]
#[ignore = "Phase-2 根因：bsp→订单转化（slice 候选饥饿）实测；需 CL；--release O(N²) 小窗"]
fn pi_probe_phase2_candidate_starvation() {
    use super::super::classifier;
    use super::super::parser;
    use super::super::types::BspBits;

    let config = ThetaConfig::default();
    eprintln!("\n========== Phase-2 根因：bsp→订单转化（slice source_index==i 候选饥饿）==========");

    fn flat(win_bars: &[super::super::types::Bar], config: &ThetaConfig) -> Vec<(u32, usize, BspBits)> {
        let l0 = parser::parse_layer(win_bars, config);
        let (cls, _) = classifier::classify_with_tower(&l0, config);
        let mut v = Vec::new();
        for (lvl, ls) in cls.levels.iter().enumerate() {
            for b in &ls.bsp {
                v.push((lvl as u32, b.source_index, b.bits));
            }
        }
        v
    }

    let sym = "CL";
    let ds = match data::load_by_symbol(sym, &config) {
        Ok(d) => d,
        Err(e) => { eprintln!("DATA BLOCKER: {sym} {e}"); panic!("需真实数据"); }
    };
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    let n: usize = 8_000.min(oos.bars.len());
    let win = prefix(&oos, n);
    eprintln!("窗口 N={n}（O(N²) 逐 bar 前缀 classify，与 runner substrate 同口径）");

    let mut prev: Vec<(u32, usize, BspBits)> = Vec::new();
    let mut sum_slice = 0usize;       // Σ_i bsp(source_index==i)（当前候选）
    let mut sum_delta = 0usize;       // Σ_i 新确认 bsp（因果候选）
    let mut bars_with_slice = 0usize; // 有 source_index==i 候选的 bar 数
    let mut bars_with_delta = 0usize; // 有新确认候选的 bar 数
    let mut lag_sum = 0u64;           // Σ (i − source_index) over delta bsp
    let mut lag_max = 0usize;
    let t0 = std::time::Instant::now();
    for i in 0..n {
        let cur = flat(&win.bars[..=i], &config);
        let slice_i = cur.iter().filter(|(_, si, _)| *si == i).count();
        // delta：cur 中不在 prev 的（level,source_index,bits）。
        let delta: Vec<_> = cur.iter().filter(|t| !prev.contains(*t)).collect();
        if slice_i > 0 { bars_with_slice += 1; }
        if !delta.is_empty() { bars_with_delta += 1; }
        sum_slice += slice_i;
        sum_delta += delta.len();
        for (_, si, _) in &delta {
            let lag = i.saturating_sub(*si);
            lag_sum += lag as u64;
            lag_max = lag_max.max(lag);
        }
        prev = cur;
    }
    let dt = t0.elapsed().as_secs_f64();
    let total_full = prev.len();
    eprintln!("\n----- 候选计数（窗口 N={n}，{dt:.1}s）-----");
    eprintln!("全窗 prefix-N bsp 总数         = {total_full}");
    eprintln!("Σ slice(source_index==i)       = {sum_slice}（{bars_with_slice} 个 bar 有候选）← 当前 substrate");
    eprintln!("Σ delta(本步新确认 bsp)        = {sum_delta}（{bars_with_delta} 个 bar 有候选）← 因果候选");
    if sum_delta > 0 {
        eprintln!("delta 候选 lag(i−source_index)：mean={:.0} max={lag_max} bar（确认点落后信号点）", lag_sum as f64 / sum_delta as f64);
    }
    eprintln!(
        "\n判读：{}",
        if sum_slice == 0 && sum_delta > 0 {
            "★坐实候选饥饿——Σslice=0（无 bar 有 source_index==i 候选）但 Σdelta>0（新确认信号真存在）。\n  \
             bsp→订单转化根因 = slice 错挂 source_index==i（确认买卖点 source_index≪i，落后 lag bar）。\n  \
             修复方向：per-bar 候选 = 本步新确认 bsp（incremental 塔的 delta），保留 source_index 作\n  \
             pivot/stop，入场触发挂 bar i（确认点）。这与 incremental 塔同根（delta 是 extend 的自然产物）。"
        } else if sum_slice > 0 {
            "slice 非饥饿（部分 bar 有 source_index==i 候选）——0 订单另有根因（AncOK 剪枝/ρ漂移/sizing），\n  \
             需进一步隔离 pi_theta_step 内部。"
        } else {
            "Σslice=0 ∧ Σdelta=0——窗内无任何 bsp（与结构探针矛盾，检查窗口/坐标）。"
        }
    );
    assert!(total_full > 0, "窗内应有 bsp（结构探针已证大窗 bsp>0）");
}

/// **★Phase-2/3 解锁证明：delta 候选 vs slice 候选驱动真 `pi_theta_step` 的 n_orders 对比**。
///
/// 候选饥饿探针证 Σslice=0 / Σdelta>0。本探针进一步**驱动生产 `coverage::pi_theta_step`**
/// （非仅计数 bsp），证：**把 per-bar 候选从 slice(source_index==i) 换成 delta(本步新确认 bsp)，
/// n_orders 从 0 翻为 >0**——即编排者硬指标（bsp→订单转化）的根治方向在可行窗上坐实。
///
/// 同口径双线程（各自 prev_active + p_t）对比，唯一变量 = 候选定义。位置 threading 简化为
/// `p_t := p_star`（假设挂单成交，L1 计数口径——决定是否**产开仓单**，非精确权益）；gate=open()
/// （隔离候选效应，不叠风控门）；tower=per-bar 因果塔（与 runner 同 639 口径）。
///
/// ★认识论 L1（管线计数，非 alpha）：证候选定义修复使引擎**产订单**（bsp→订单转化通），**不**声明盈利。
///
/// `#[ignore]`，需 CL；`--release`（O(N²) per-bar 前缀重分类，用 12K 控时）。
#[test]
#[ignore = "Phase-2/3 解锁证明：delta vs slice 候选 → pi_theta_step n_orders；需 CL；--release O(N²) 12K"]
fn pi_probe_delta_candidates_unlock_orders() {
    use super::super::classifier::{self, bsp::BspPoint, LevelState, Classification};
    use super::super::parser;
    use super::super::strategy::coverage::{pi_theta_step, KThetaRiskGate, PiThetaWeights};
    use super::super::strategy::interp::ActiveLeg;

    let config = ThetaConfig::default();
    eprintln!("\n========== Phase-2/3 解锁证明：delta vs slice 候选 → pi_theta_step n_orders ==========");

    // slice 候选（**旧**口径，已被 runner::newly_confirmed_step 取代）：bsp where source_index==i，保级别结构。
    fn slice_step(cur: &Classification, i: usize) -> Classification {
        Classification {
            levels: cur.levels.iter().map(|ls| LevelState {
                moves: Vec::new(), centers: Vec::new(),
                bsp: ls.bsp.iter().filter(|b| b.source_index == i).cloned().collect(),
            }).collect(),
        }
    }
    // delta 候选（本步新确认）：bsp ∈ cur ∖ prev（按级别 + BspPoint 相等），保级别结构。
    fn delta_step(cur: &Classification, prev: &Classification) -> Classification {
        Classification {
            levels: cur.levels.iter().enumerate().map(|(lvl, ls)| {
                let pb: &[BspPoint] = prev.levels.get(lvl).map(|p| p.bsp.as_slice()).unwrap_or(&[]);
                LevelState {
                    moves: Vec::new(), centers: Vec::new(),
                    bsp: ls.bsp.iter().filter(|b| !pb.contains(b)).cloned().collect(),
                }
            }).collect(),
        }
    }

    let sym = "CL";
    let ds = match data::load_by_symbol(sym, &config) {
        Ok(d) => d, Err(e) => { eprintln!("DATA BLOCKER: {sym} {e}"); panic!("需真实数据"); }
    };
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    let n: usize = 12_000.min(oos.bars.len());
    let win = prefix(&oos, n);
    let nav = nav_for(&win, &config);
    let weights = PiThetaWeights::from_risk(&config.risk);
    eprintln!("窗口 N={n} NAV={nav:.0}（O(N²) per-bar 前缀重分类，与 runner substrate 同口径）");

    // 双线程独立态（唯一变量=候选定义）。
    let (mut slice_active, mut slice_pt): (Vec<ActiveLeg>, f64) = (Vec::new(), 0.0);
    let (mut delta_active, mut delta_pt): (Vec<ActiveLeg>, f64) = (Vec::new(), 0.0);
    let mut slice_orders = 0usize;
    let mut delta_orders = 0usize;
    let mut prev_full = Classification::default();
    let t0 = std::time::Instant::now();
    for i in 0..n {
        let bar = &win.bars[i];
        let px = bar.close as f64 * config.tick.tick_size;
        if bar.untradable || px <= 0.0 { continue; }
        let l0 = parser::parse_layer(&win.bars[..=i], &config);
        let (cur_full, tower_i) = classifier::classify_with_tower(&l0, &config);
        let base_units = nav / px;

        // 线程 A：slice 候选。
        let s_step = slice_step(&cur_full, i);
        let (s_next, _s_star_, s_order) = pi_theta_step(
            &s_step, &tower_i, &slice_active, slice_pt, i, base_units,
            &config.voice, &config.risk, weights, KThetaRiskGate::open(),
        );
        if s_order.qty > 0 { slice_orders += 1; slice_pt = _s_star_; }
        slice_active = s_next;

        // 线程 B：delta 候选（本步新确认）。
        let d_step = delta_step(&cur_full, &prev_full);
        let (d_next, d_star, d_order) = pi_theta_step(
            &d_step, &tower_i, &delta_active, delta_pt, i, base_units,
            &config.voice, &config.risk, weights, KThetaRiskGate::open(),
        );
        if d_order.qty > 0 { delta_orders += 1; delta_pt = d_star; }
        delta_active = d_next;

        prev_full = cur_full;
    }
    let dt = t0.elapsed().as_secs_f64();
    eprintln!("\n----- n_orders 对比（窗口 N={n}，{dt:.1}s）-----");
    eprintln!("slice(source_index==i) 候选 → n_orders = {slice_orders}  ← 当前 substrate（编排者实测 0）");
    eprintln!("delta(本步新确认 bsp)   候选 → n_orders = {delta_orders}  ← 候选坐标修复");
    eprintln!(
        "\n判读：{}",
        if delta_orders > 0 && slice_orders == 0 {
            "★解锁坐实——delta 候选使 pi_theta_step 产订单（slice 恒 0）。bsp→订单转化的根治 = 候选定义\n  \
             从 source_index==i 改为本步新确认 delta（incremental 塔 extend 的自然产物）。编排者硬指标\n  \
             （≥64K n_orders>0）的**机制**在 12K 可行窗坐实；全窗验证待 O(n) incremental 塔（未竟，见报告）。"
        } else if delta_orders > 0 {
            "delta 产订单且 slice 亦非 0——候选定义影响 n_orders（方向确认），细节见计数。"
        } else {
            "delta 仍 0 订单——候选非唯一阻塞，AncOK 剪枝/ρ漂移/sizing 需进一步隔离（codex Q4）。"
        }
    );
    // 不 assert delta>0（探针是观测，真实数据驱动；记录事实供报告）。
    eprintln!("（探针记录事实，不强制断言——真实数据驱动观测）");
}

/// **★验证 #2 硬指标（≥64K n_orders>0）：生产 `run_theta_v0_pi`（确认-bar 部署修复）实测**。
///
/// 编排者硬判据：大窗（≥64K bar）`run_theta_v0_pi` n_orders>0——确认-bar 部署
/// （[`super::runner::newly_confirmed_step`]）修复成败的判据。本测试在 CL 65536-bar 窗上跑**生产路径**
/// （fix 已实装），实测 n_orders 并断言 >0。O(n²) substrate ⟹ 64K 约数分钟（exp≈2.69）；全窗 O(n)
/// 待 incremental 塔（未竟）。
///
/// ★认识论 L1（管线计数，非 alpha）：证生产 substrate 在 ≥64K 真实数据产订单流，**不**声明盈利（231）。
#[test]
#[ignore = "验证#2 硬指标：生产 run_theta_v0_pi ≥64K n_orders>0；需 CL；--release O(n²) ~数分钟"]
fn pi_production_orders_at_64k() {
    let config = ThetaConfig::default();
    let ds = match data::load_by_symbol("CL", &config) {
        Ok(d) => d,
        Err(e) => { eprintln!("DATA BLOCKER: CL {e}"); panic!("需真实数据"); }
    };
    let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
    let n = 65_536.min(oos.bars.len());
    let win = prefix(&oos, n);
    let nav = nav_for(&win, &config);
    eprintln!("\n========== 验证#2：生产 run_theta_v0_pi（确认-bar 部署）CL N={n} ==========");
    let t0 = std::time::Instant::now();
    let res = run_theta_v0_pi(&win, &config, 1.0, nav);
    let dt = t0.elapsed().as_secs_f64();
    let real = res.trades.iter().filter(|t| !t.forced_close).count();
    eprintln!(
        "N={n} wall={dt:.1}s n_orders={} trades={} real_trades={} is_l2={}",
        res.n_orders, res.trades.len(), real, res.is_l2
    );
    eprintln!(
        "判读：{}",
        if res.n_orders > 0 {
            "★硬指标达成——生产 substrate 在 ≥64K 真实数据 n_orders>0（确认-bar 部署解锁交易，L1 管线通）。"
        } else {
            "n_orders=0——确认-bar 部署未解锁（与 16K 8617 单矛盾，检查窗口/坐标）。"
        }
    );
    assert!(res.n_orders > 0, "硬指标：≥64K n_orders>0（实得 {}）", res.n_orders);
}

/// **★Phase-1 诊断探针：区分 (a) 硬工程断流 vs (c) 结构饥饿**（zero-orders 根因定位）。
///
/// scaling 探针实测：feasible 窗（≤16K bar）pi `n_orders=0`。本诊断分两路：
/// 1. **单次全窗 classify**（O(n) 单趟，可行）在更大窗 [16K,64K,256K] 上数塔层数 + bsp——
///    结构是否随窗增长出现（多级塔 + 非空 bsp）。zero bsp ⟹ 分类器在真实数据空（更深工程缺口）；
///    非空 bsp 但 pi 仍 0 单 ⟹ per-bar 因果 σ_p + AncOK 准入剪枝（结构/ρ漂移）。
/// 2. **per-bar pi** 在可行边缘 [32K,64K] + 另一品种（BTC）测 n_orders 是否**曾**非零。
///
/// `#[ignore]`，需 CL+BTC 缓存；`--release`。
#[test]
#[ignore = "Phase-1 诊断：zero-orders 根因 (a)硬断流 vs (c)结构饥饿；需数据；--release"]
fn pi_probe_zero_orders_root_cause() {
    let config = ThetaConfig::default();
    eprintln!("\n===== Phase-1 诊断：pi zero-orders 根因 (a)硬工程断流 vs (c)结构饥饿 =====");

    // ── 路 1：单次全窗 classify 结构统计（O(n)，可行到大窗）──
    eprintln!("\n----- 路1：单趟全窗 classify_with_tower 结构（O(n) 单趟）-----");
    eprintln!("{:>8} {:>6} {:>11} {:>9} {:>11}  {}", "n_bars", "塔层", "tower_s", "total_bsp", "total_mv", "per_level_bsp");
    for (sym, oos_win) in [("CL", ("2023-01-01", "2025-06-30")), ("BTC", ("2023-01-01", "2025-06-30"))] {
        let ds = match data::load_by_symbol(sym, &config) {
            Ok(d) => d,
            Err(e) => { eprintln!("{sym} 加载失败：{e}"); continue; }
        };
        let oos = ds.slice_date_window(oos_win.0, oos_win.1);
        for &n in &[16_000usize, 64_000, 256_000] {
            if n > oos.bars.len() { continue; }
            let win = prefix(&oos, n);
            let t0 = std::time::Instant::now();
            let (depth, bsp, mv, per) = classify_structure_stats(&win, &config);
            let dt = t0.elapsed().as_secs_f64();
            eprintln!("{sym:<3} {n:>4} {depth:>6} {dt:>10.2}s {bsp:>9} {mv:>11}  {per:?}");
        }
    }

    // ── 路 2：per-bar pi 在可行边缘测 n_orders 是否曾非零（多品种）──
    eprintln!("\n----- 路2：per-bar pi n_orders 可行边缘（O(n²)，单窗预算内）-----");
    eprintln!("{:>4} {:>8} {:>9} {:>7} {:>9}", "sym", "n_bars", "wall_s", "n_ord", "real_trd");
    const EDGE_BUDGET_S: f64 = 300.0;
    for (sym, oos_win) in [("CL", ("2023-01-01", "2025-06-30")), ("BTC", ("2023-01-01", "2025-06-30"))] {
        let ds = match data::load_by_symbol(sym, &config) {
            Ok(d) => d,
            Err(e) => { eprintln!("{sym} 加载失败：{e}"); continue; }
        };
        let oos = ds.slice_date_window(oos_win.0, oos_win.1);
        for &n in &[32_000usize, 64_000] {
            if n > oos.bars.len() { continue; }
            let win = prefix(&oos, n);
            let nav = nav_for(&win, &config);
            let t0 = std::time::Instant::now();
            let res = run_theta_v0_pi(&win, &config, 1.0, nav);
            let dt = t0.elapsed().as_secs_f64();
            let n_real = res.trades.iter().filter(|t| !t.forced_close).count();
            eprintln!("{sym:<4} {n:>8} {dt:>8.1}s {:>7} {n_real:>9}", res.n_orders);
            if dt > EDGE_BUDGET_S { eprintln!("({sym} n={n} {dt:.0}s>预算，停)"); break; }
        }
    }
    eprintln!(
        "\n判读：路1 全窗 classify bsp>0 但路2 pi n_orders=0 ⟹ **per-bar 因果 σ_p+AncOK 剪枝**（结构/ρ漂移，\n  \
         (a)/(c) 边界）；路1 bsp=0 ⟹ 分类器真实数据空（硬 (a)）；路2 大窗 n_orders 转正 ⟹ (c) 结构饥饿\n  \
         （需大窗形成父容器，但 O(n²) 致大窗不可行 = 双重障碍 memory l2-falsify-dual-barrier）。"
    );
}
