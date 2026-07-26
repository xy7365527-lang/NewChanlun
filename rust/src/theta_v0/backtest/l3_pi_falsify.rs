//! **Phase-2 L2/L3 否证检验**：七链生产 runner `run_theta_v0_pi`（σ_p=父容器方向 639 +
//! AncOK 持仓准入）的真实数据双口径否证——**镜像** v1 harness
//! [`super::l3_fullwindow::l3_fullwindow_multi_symbol_significance`] 的全部口径与归因逻辑，
//! 但驱动 `run_theta_v0_pi`（非 v1 `run_theta_v0`/recognize）。**独立**否证（新七链 ≠ 已被否证
//! 的买卖点 v1，memory `newchanlun-v1-fullwindow-l3-falsified`）。
//!
//! ## 阶段1探针结论 → 本阶段用可行子集（非全窗）
//!
//! pi substrate = per-bar 前缀重分类 = **O(n²)**（探针实测 exp≈2.0–2.66；CL OOS 870K bar 全窗
//! 外推单次≈75h，8 品种≈600h，双口径≈1200h，walk-forward×N 远超）⟹ **全窗 L3 不可行**
//! （[`super::l3_pi_probe`] 坐实，非 conjecture）。故本阶段在**预注册 OOS 窗前缀截断到
//! `MAX_BARS_PI`** 的可行子集上跑——显式有效域边界（非静默截断）。
//!
//! ## 双口径门（接 Nautilus 前置，231号）
//!
//! - **①收益口径**：`n_beats_random`（[`metrics::significance`] block bootstrap + 操作语义
//!   随机对照 shift/indep，seed 冻结）。
//! - **②风险调整口径**：Sharpe + MaxDD + Calmar **对照 depth=0 baseline**（`voice.max_depth=1`
//!   = 仅 depth-0 根声部，无嵌套对冲腿）的**超额**（非绝对值）——隔离七链嵌套对冲深度的边际
//!   风险调整贡献。baseline 仅在 strategy 产订单时跑（strategy 零订单 ⟹ baseline 必零 ⟹ 超额退化）。
//!
//! ## 分层诊断（强制，memory `l2-engine-incompleteness-vs-theta-falsification`）
//!
//! 每品种归一类并计数：(a) **引擎产不出信号**（工程层，n_orders=0；探针坐实短窗前缀塔仅 L0 ⟹
//! 候选父=Ambient ⟹ 无订单，runner.rs:246 边界②）；(b) **Θ 产信号但不盈利**（经验否证，真
//! n_beats_random=0 且 real_trades≥30）；(c) **样本饥饿**（real_trades<30，O(n²) 截断致样本不足，
//! inconclusive **非否证**）；(d) 对照退化 inconclusive。**严禁** (a)/(c)/(d) 谎报为 (b)。
//!
//! ## ρ 漂移 caveat（前序 AncOK 工位披露，诚实标注）
//!
//! per-bar 重分类下父容器走势延展 ⟹ 持仓父腿坐标漂移 ⟹ hedge 子腿被**保守剪枝** ⟹ 策略是
//! **保守欠对冲版**（安全方向，不开 naked，但少 ReverseOpen 腿）。本检验测此保守版，非完整对冲七链。
//!
//! 跑法：`cargo test --release --lib theta_v0::backtest::l3_pi_falsify -- --ignored --nocapture`

use super::data::{self, Dataset};
use super::metrics;
use super::prereg_windows::PREREG_WINDOWS;
use super::runner::run_theta_v0_pi;
use super::super::config::ThetaConfig;

/// OOS 窗年跨（与 l3_fullwindow 同口径：朴素序数日差 / 365.25）。
fn oos_years(oos_start: &str, oos_end: &str) -> f64 {
    fn ymd(s: &str) -> (i64, i64, i64) {
        let p: Vec<i64> = s.split('-').map(|x| x.parse().unwrap_or(0)).collect();
        (p[0], *p.get(1).unwrap_or(&1), *p.get(2).unwrap_or(&1))
    }
    fn ord(y: i64, m: i64, d: i64) -> i64 {
        const CUM: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        y * 365 + y / 4 + CUM[(m as usize - 1).min(11)] + d
    }
    let (y0, m0, d0) = ymd(oos_start);
    let (y1, m1, d1) = ymd(oos_end);
    ((ord(y1, m1, d1) - ord(y0, m0, d0)) as f64 / 365.25).max(0.1)
}

/// **★Phase-2 L2/L3 pi 否证：8 品种【可行子集截断窗】OOS + 双口径门 + 分层诊断**。
///
/// `#[ignore]`，需 8 品种数据缓存；O(n²) 慢测 `--release` 必须。窗口截断到 `MAX_BARS_PI`
/// （全窗 O(n²) 不可行，探针坐实）。
#[test]
#[ignore = "Phase-2 pi 否证；O(n²) 截断窗；需 8 品种数据；--release"]
fn l3_pi_falsify_multi_symbol_significance() {
    let config = ThetaConfig::default();
    // depth=0 baseline：仅 depth-0 根声部（within_max_depth(d)=d<max_depth ⟹ max_depth=1 仅 d=0 开）。
    let mut baseline_config = config.clone();
    baseline_config.voice.max_depth = 1;

    const MIN_TRADES_FOR_L2: usize = 30; // 协议 §5.5：real_trades<30 inconclusive（625 铁律）
    // 可行子集窗 bar 数（全窗 O(n²) 不可行，探针外推单品种全窗≈75h）。
    // 32K：诊断实测 CL 32K≈49s / BTC 32K≈66s（且 CL/BTC 至 64K 仍 0 单）⟹ 8 品种 ~7min 可控；
    // 显式有效域边界（非全窗结论）。诊断坐实零订单在 16K–64K 鲁棒，32K 为可行子集代表窗。
    const MAX_BARS_PI: usize = 32_000;

    eprintln!("\n===== Phase-2 pi 否证：8 品种【可行子集 {MAX_BARS_PI}bar 截断窗】OOS + 双口径门 =====");
    eprintln!("★全窗 O(n²) 不可行（探针坐实单品种全窗≈75h）⟹ 截断窗 = 显式有效域边界（非全窗结论）");
    eprintln!("★驱动 run_theta_v0_pi（七链 π_Θ，σ_p=父容器方向 639+AncOK 准入），非 v1 recognize（独立否证）");
    eprintln!("★ρ漂移 caveat：保守欠对冲版（hedge 子腿剪枝）——结果解读须带此 caveat");
    eprintln!(
        "{:<6} {:>9} {:>6} {:>6} {:>8} {:>8} {:>8} {:>8} {:>7} {:>6} {}",
        "symbol", "bars/T", "real", "trd", "Shrp", "ΔShrp", "MaxDD", "ΔCalmar", "boot_p", "beats?", "归因",
    );

    let mut n_falsify = 0usize; // (b) 经验否证
    let mut n_confirm = 0usize; // (c') 确认
    let mut n_inconclusive = 0usize; // (c)+(d) 样本饥饿/对照退化
    let mut n_engine_block = 0usize; // (a) 工程断流
    let mut n_beats_random = 0usize; // Θ 优于随机对照的品种数
    let mut n_done = 0usize;

    for w in PREREG_WINDOWS {
        let ds = match data::load_by_symbol(w.symbol, &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{:<6} 加载失败：{e}（DATA BLOCKER，不伪造合成）", w.symbol);
                continue;
            }
        };
        let oos_full = ds.slice_date_window(w.oos.0, w.oos.1);
        if oos_full.bars.is_empty() {
            eprintln!("{:<6} OOS 窗空（{}→{}）", w.symbol, w.oos.0, w.oos.1);
            continue;
        }
        // 截断到 MAX_BARS_PI（全窗 O(n²) 不可行）——显式有效域边界。
        let cut = MAX_BARS_PI.min(oos_full.bars.len());
        let truncated = cut < oos_full.bars.len();
        let oos = Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[..cut].to_vec(),
            dates: oos_full.dates[..cut.min(oos_full.dates.len())].to_vec(),
            bar_seconds: 60,
        };
        n_done += 1;

        let full_years = oos_years(w.oos.0, w.oos.1);
        let years = if truncated {
            (full_years * cut as f64 / oos_full.bars.len() as f64).max(0.1)
        } else {
            full_years
        };
        let first_px = oos
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);

        let t0 = std::time::Instant::now();
        let res = run_theta_v0_pi(&oos, &config, years, nav);
        let pnls = &res.trade_pnls;
        let n_trades = pnls.len();
        let n_real_trades = res.trades.iter().filter(|t| !t.forced_close).count();

        // ②风险调整 depth=0 baseline（仅 strategy 产订单时跑——零订单 baseline 必零，超额退化）。
        let (d_sharpe, d_calmar, base_sharpe) = if res.n_orders > 0 {
            let base = run_theta_v0_pi(&oos, &baseline_config, years, nav);
            (
                res.metrics.sharpe - base.metrics.sharpe,
                res.metrics.calmar - base.metrics.calmar,
                base.metrics.sharpe,
            )
        } else {
            (0.0, 0.0, 0.0) // 退化：strategy 零订单 ⟹ baseline 亦零 ⟹ 超额=0（degenerate）
        };
        let _ = base_sharpe;

        // ①收益口径 significance（block bootstrap + 操作语义随机对照）。
        let sig = metrics::significance(
            pnls,
            &res.daily_returns,
            &res.trades,
            &res.prices,
            res.fee_rate,
            res.theta_return_mtm,
        );
        if sig.theta_beats_random {
            n_beats_random += 1;
        }

        // 分层诊断（强制；(a)/(c)/(d) 严禁谎报 (b)）。
        let verdict = if res.n_orders == 0 || n_trades == 0 {
            n_engine_block += 1;
            "(a)工程断流(0单)"
        } else if n_real_trades < MIN_TRADES_FOR_L2 {
            n_inconclusive += 1;
            "(c)样本饥饿real<30"
        } else if sig.controls_degenerate {
            n_inconclusive += 1;
            "(d)对照退化"
        } else if sig.boot_pvalue_pnl_le_0 > 0.05 {
            n_falsify += 1;
            "(b)否证:收益不显著"
        } else if !sig.theta_beats_random {
            n_falsify += 1;
            "(b)否证:不优随机"
        } else {
            n_confirm += 1;
            "(c')确认:p≤.05且优随机"
        };

        let elapsed = t0.elapsed().as_secs_f64();
        eprintln!(
            "{:<6} {:>8}{} {:>6} {:>6} {:>8.3} {:>8.3} {:>8.4} {:>8.3} {:>7.4} {:>6} {}  [{:.1}s]",
            w.symbol,
            oos.bars.len(),
            if truncated { "T" } else { "F" },
            n_real_trades,
            n_trades,
            res.metrics.sharpe,
            d_sharpe,
            res.metrics.max_drawdown,
            d_calmar,
            sig.boot_pvalue_pnl_le_0,
            sig.theta_beats_random,
            verdict,
            elapsed,
        );

        // 不变量：管线不崩 + 检验值合法。
        assert!(res.metrics.sharpe.is_finite(), "{} sharpe 有限", w.symbol);
        assert!((0.0..=1.0).contains(&sig.boot_pvalue_pnl_le_0), "{} boot p∈[0,1]", w.symbol);
        assert_eq!(res.is_l2, res.n_orders > 0, "{} is_l2 ⟺ 订单非空", w.symbol);
    }

    // ── 跨标的聚合 + 诚实裁定（231号：否定/inconclusive 比确认更有价值）──
    let total = PREREG_WINDOWS.len();
    eprintln!("\n===== Phase-2 pi 跨标的聚合（8 品种 {MAX_BARS_PI}bar 可行子集 OOS）=====");
    eprintln!("完成品种数                          : {n_done}/{total}");
    eprintln!("(a) 工程断流(n_orders=0)            : {n_engine_block}");
    eprintln!("(c)/(d) inconclusive(样本饥饿/对照退化): {n_inconclusive}");
    eprintln!("(b) L2 否证(收益不显著/不优随机)     : {n_falsify}");
    eprintln!("(c') 确认(p≤.05 且优两随机对照)       : {n_confirm}");
    eprintln!("    其中 Θ 打败两随机对照的品种数      : {n_beats_random}/{total}");
    eprintln!(
        "\n★诚实裁定（formalization-validity-domain 231号 + l2-engine-incompleteness）：\n  \
         - 若多数 (a)/(c)：**非 Θ 否证**，是 substrate/工程缺口（O(n²) 致可行窗结构饥饿，\n    \
           runner.rs:246 边界②：短窗前缀塔仅 L0 ⟹ 候选父=Ambient ⟹ 无订单）⟹ inconclusive。\n  \
         - 仅当 (b) 且 real_trades≥30：才是真经验否证（n_beats_random 是择时信息含量直接计数）。\n  \
         - 双重障碍（memory l2-falsify-dual-barrier）：O(n²) ⟹ 大窗不可行；短窗 ⟹ 结构饥饿零信号。\n  \
         - 修复方向：incremental persize-identity 塔（复用 zhongshu::incremental_tests /\n    \
           segment_layers::incremental_tests 增量原语，substrate 重构非从零）⟹ O(n) per-bar ⟹ 全窗可行。\n  \
         - ρ漂移 caveat：本结论的有效域 = **保守欠对冲版 pi**（hedge 子腿剪枝），非完整对冲七链。"
    );

    // 分层完备 + acceptance：管线在多标的真实数据上跑通（≥1 品种产订单或全 (a) 诚实计数）。
    assert_eq!(
        n_falsify + n_confirm + n_inconclusive + n_engine_block,
        n_done,
        "分层完备：每完成品种恰归一类",
    );
    assert!(n_done >= 1, "≥1 品种完成（真实数据 harness 可执行）");
}
