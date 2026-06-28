//! **★全窗 L3 定论（task #75，goal 核心问题最终裁决）**——完整 v1 全 8 品种 OOS **全窗**否证。
//!
//! ## 与 [`runner::tests::l3_falsify_multi_symbol_significance`] 的关系
//!
//! l3_falsify 是**截断窗**（`MAX_BARS=60_000`）——全窗 CPU-bound 曾不可行（BTC 全窗 1.31M
//! bar，signal.rs 590× 加速前 >5min）。signal.rs O(n²)→O(n) 加速（task #72）后全窗可行。
//! 本测试**复现 l3_falsify 的全部口径与归因逻辑，唯一改动 `cut = oos_full.bars.len()`（全窗，
//! 不截断）**——产出全窗 L3 定论，回答"截断窗 8/8 否证是真定论，还是 ~4.5% 覆盖的假阴性"。
//!
//! ## 为何写在 lib 自身 `#[cfg(test)]` 而非 `tests/*.rs`（机器坐实，非选择）
//!
//! `theta_v0::backtest` 整模块被 `#[cfg(test)] pub mod backtest`（mod.rs:88）门控（理由：data
//! 依赖 dev-dep serde_json + 避免污染 cdylib/Python 扩展构建）。integration test（`tests/*.rs`）
//! 是独立 crate，链接 lib 正常编译产物（不带 `--test`）⟹ `cfg(test)=false` ⟹ backtest 模块
//! **不存在**（`use ...backtest::runner` 报 `E0433: cannot find backtest`，机器坐实）。`run_theta_v0`
//! 等虽 pub，但模块门控使 pub 失效。故全窗测试**只能**写在 lib 自身 test 编译内（本文件）。
//!
//! ## 认识论等级与诚实边界（formalization-validity-domain 231号）
//!
//! - **单标的全窗 = L2，全 8 品种全窗复现 = L3**：跨品种"是否复现同一方向（否证/确认）"是
//!   L3 鲁棒性结论。**全窗**消除截断窗的统计功效受限 caveat（覆盖 100% OOS 而非 ~4.5%）。
//! - **有效域分层归因（强制 caveat，grammar-audit 0c71a0f771 + 231号）**：本否证/确认的有效域
//!   = **「完整 v1 实装」**（type3 去重 + 做空腿 + 真背驰 A/B/C + 方向感知 metrics），**非
//!   「完整缠论语法」**。**#5 多声部 depth=0 未补**（次级别对冲 alpha 未启）——诚实标注。
//!   ⟹ 若 (b) 否证：严禁归因「缠论无 alpha」——只否定**这套 v1 实装 / 这些品种 / 这个尺度**。
//! - **否定性结果价值**（231号）：全窗否证跨标的复现 = 缠论择时（v1 实装）无 alpha 的 L3 鲁棒
//!   结论，缩小有效域边界，比确认更有价值。
//!
//! 跑法：`cargo test --release --lib theta_v0::backtest::l3_fullwindow -- --ignored --nocapture`

use super::data::{self, Dataset};
use super::metrics;
use super::prereg_windows::PREREG_WINDOWS;
use super::runner::run_theta_v0;
use super::super::config::ThetaConfig;

/// OOS 窗年跨（自 l3_falsify 内私有 `oos_years` 等价复现——纯日期算术，零概念，年化基数用）。
/// 朴素序数日差 / 365.25，闰年误差 ≤1 日对年化量级无影响（与 l3_falsify 同口径）。
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

/// **★L3 全窗否证/确认：全 8 品种【全窗】OOS + §3.4/§4 统计检验（成交盈亏口径）**。
///
/// `#[ignore]`，需全 8 品种 `analysis/data_cache/*.json`；慢测，`--release` 必须。
/// 复现 l3_falsify 全部口径，唯一改动 `cut = oos_full.bars.len()`（全窗）。
#[test]
#[ignore = "全窗 CPU 重；需 8 品种数据缓存；--release 跑"]
fn l3_fullwindow_multi_symbol_significance() {
    let config = ThetaConfig::default();
    const MIN_TRADES_FOR_L2: usize = 30; // 协议 §5.5：n_trades<30 inconclusive（625 铁律）

    eprintln!("\n===== L3 多标的【全窗 OOS】否证：8 品种 + §3.4/§4 操作语义随机对照 =====");
    eprintln!("★全窗（cut=oos_full.bars.len()，不截断）⟹ 覆盖 100% OOS（非截断窗 ~4.5%），消统计功效受限 caveat");
    eprintln!("★口径：boot_p 用已实现 trade_pnls；随机对照（shift/indep）用含浮盈 total_return；beats?=两对照 p≤0.05");
    eprintln!(
        "{:<6} {:>12} {:>6} {:>6} {:>8} {:>9} {:>7} {:>7} {:>7} {:>6} {}",
        "symbol", "oos_bars/T", "real", "trd", "MtM%", "bh%",
        "boot_p", "shift_p", "indep_p", "beats?", "归因",
    );
    eprintln!("（real=非强平笔数(门槛)；trd=已实现笔数；MtM%=含浮盈≠成交盈亏；shift_p/indep_p=随机对照 p_upper）");

    // L3 聚合：跨品种否证/确认/inconclusive/工程断流计数。
    let mut n_falsify = 0usize; // (b) L2 否证
    let mut n_confirm = 0usize; // (c) L2 确认
    let mut n_inconclusive = 0usize; // (d) 样本不足
    let mut n_engine_block = 0usize; // (a) 工程断流（无订单/无平仓）
    let mut n_beats_random = 0usize; // Θ 优于随机择时的品种数（择时信息含量）
    let mut n_full = 0usize; // 实际全窗完成的品种数（no-patch：若某品种被迫截断则不计入）

    for w in PREREG_WINDOWS {
        let t0 = std::time::Instant::now();
        let ds = match data::load_by_symbol(w.symbol, &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{:<6} 加载失败：{e}", w.symbol);
                continue;
            }
        };
        // 预注册 OOS 窗（唯一真相源 = PREREG_WINDOWS，防数据挖掘）。
        let oos_full = ds.slice_date_window(w.oos.0, w.oos.1);
        if oos_full.bars.is_empty() {
            eprintln!("{:<6} OOS 窗空（{}→{}）", w.symbol, w.oos.0, w.oos.1);
            continue;
        }
        // ★全窗：cut = 全部 OOS bar（与 l3_falsify 唯一差异——后者 MAX_BARS=60_000 截断）。
        let cut = oos_full.bars.len();
        let oos = Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars.clone(),
            dates: oos_full.dates.clone(),
        };
        n_full += 1;

        // years：全窗按预注册 OOS 全跨（不缩放——这是全窗，覆盖整个预注册窗）。
        let years = oos_years(w.oos.0, w.oos.1);
        // NAV 与品种价量级匹配（首价×容量；与 l3_falsify 同口径）。
        let first_px = oos
            .bars
            .iter()
            .find(|b| !b.untradable && b.close > 0)
            .map(|b| b.close as f64 * config.tick.tick_size)
            .unwrap_or(1.0);
        let nav = (first_px * 1000.0).max(1.0e6);
        let res = run_theta_v0(&oos, &config, years, nav);

        let pnls = &res.trade_pnls; // 已实现口径（§3.4 bootstrap）
        let n_trades = pnls.len();
        // ★门槛计数 = 非强平笔数（codex：强平笔不偷过 n≥30 统计功效门槛，625 铁律）。
        let n_real_trades = res.trades.iter().filter(|t| !t.forced_close).count();
        let total_pnl: f64 = pnls.iter().sum();

        // §3.4 block bootstrap（已实现）+ §4 操作语义随机对照（含浮盈，seed=20260625 冻结）。
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

        // 分层归因（625 + §5.5 inconclusive 严格区分；门槛用非强平笔数）。
        let verdict = if res.n_orders == 0 || n_trades == 0 {
            n_engine_block += 1;
            "(a)工程断流"
        } else if n_real_trades < MIN_TRADES_FOR_L2 {
            n_inconclusive += 1;
            "(d)inconcl real<30"
        } else if sig.controls_degenerate {
            n_inconclusive += 1;
            "(d)inconcl 对照退化"
        } else if sig.boot_pvalue_pnl_le_0 > 0.05 {
            n_falsify += 1;
            "(b)否证:收益不显著"
        } else if !sig.theta_beats_random {
            n_falsify += 1;
            "(b)否证:不优于随机"
        } else {
            n_confirm += 1;
            "(c)确认:p≤.05且优随机"
        };

        let elapsed = t0.elapsed().as_secs_f64();
        eprintln!(
            "{:<6} {:>11}F {:>6} {:>6} {:>8.2} {:>9.2} {:>7.4} {:>7.4} {:>7.4} {:>6} {}  [{:.1}s]",
            w.symbol,
            oos.bars.len(),
            n_real_trades,
            n_trades,
            res.metrics.strat_return * 100.0,
            res.metrics.bh_return * 100.0,
            sig.boot_pvalue_pnl_le_0,
            sig.shift_pvalue,
            sig.indep_pvalue,
            sig.theta_beats_random,
            verdict,
            elapsed,
        );
        eprintln!(
            "       └ full_oos_bars={} cut={} (全窗) 已实现total={:.2} Θ_同口径={:.4} (MtM复利={:.4}仅参考) shift_mean={:.4} indep_mean={:.4} Sharpe={:.3} years={:.2}",
            oos_full.bars.len(),
            cut,
            total_pnl,
            sig.theta_return_same_caliber,
            sig.theta_return_mtm,
            sig.shift_mean_return,
            sig.indep_mean_return,
            sig.sharpe,
            years,
        );

        // 不变量（每品种）：管线不崩 + 检验值合法 + 标注一致 + 全窗确认。
        assert!(res.metrics.strat_return.is_finite(), "{} strat(MtM) 有限", w.symbol);
        assert!(res.metrics.bh_return.is_finite(), "{} bh 有限", w.symbol);
        assert!(
            (0.0..=1.0).contains(&sig.boot_pvalue_pnl_le_0),
            "{} boot p∈[0,1]",
            w.symbol
        );
        assert!((0.0..=1.0).contains(&sig.shift_pvalue), "{} shift p∈[0,1]", w.symbol);
        assert!((0.0..=1.0).contains(&sig.indep_pvalue), "{} indep p∈[0,1]", w.symbol);
        assert_eq!(res.is_l2, res.n_orders > 0, "{} is_l2 ⟺ 订单非空", w.symbol);
        assert_eq!(cut, oos_full.bars.len(), "{} cut=全窗（无截断）", w.symbol);
    }

    // ── L3 全窗跨标的结论（231号：否证跨标的复现 = 鲁棒否证，比确认更有价值）──
    let total = PREREG_WINDOWS.len();
    eprintln!("\n===== L3 跨标的否证聚合（8 品种【全窗】OOS，操作语义随机对照）=====");
    eprintln!("全窗完成品种数（no-patch 诚实计数）  : {n_full}/{total}");
    eprintln!("(a) 工程断流(无订单/无平仓)       : {n_engine_block}");
    eprintln!("(d) inconclusive(real_trades<30)  : {n_inconclusive}");
    eprintln!("(b) L2 否证(收益不显著/不优随机)  : {n_falsify}");
    eprintln!("(c) L2 确认(p≤.05 且优于两随机对照): {n_confirm}");
    eprintln!("    其中 Θ 含浮盈打败两随机对照的品种数 : {n_beats_random}/{total}");
    eprintln!(
        "\n★L3 全窗诚实结论（formalization-validity-domain 231号）：\n  \
         - 跨标的多数 (b) ⟹ **「完整 v1 实装的择时不贡献 alpha」** 是鲁棒否证（L3 全窗，有效域缩小）。\n  \
         - 各品种结论分散 ⟹ v1 有效性**品种依赖**（有效域 < 定义域，非全域有效）。\n  \
         - n_beats_random 是择时信息含量的直接计数：=0 ⟹ v1 实装全标的无择时信息。\n  \
         - 全窗（覆盖 100% OOS）⟹ 消除截断窗 ~4.5% 覆盖的统计功效受限 caveat；本结论是全窗定论。"
    );
    // ★★有效域分层归因（231号强制 caveat）：完整 v1 已修 #1/#2/方向感知/type3，但 #5 多声部未补。
    eprintln!(
        "\n★★有效域分层归因（完整 v1 实装；231号强制 caveat）：\n  \
         否证/确认的有效域 = 「完整 v1 实装」（type3 去重 + 做空腿 + 真背驰 A/B/C + 方向感知 metrics），\n  \
         **非「完整缠论语法」**。已修的退化维度（相对 Θ v0 退化实装）：\n  \
         - #1 long-only：v1 做空腿已补（track_position_transition 多空 + 强平 pos_sign 对称）。\n  \
         - #2 背驰退化：v1 真背驰 A/B/C 趋势/盘整框架已实装（修假背驰=假买卖点噪声）。\n  \
         - type3 去重 + metrics 方向感知：决策基础干净 + 做空 PnL 镜像对称。\n  \
         **仍未补的退化维度（诚实标注，有效域 caveat）**：\n  \
         - #5 多声部 depth=0：结构是 canonical §5 契约（正确），但放弃次级别对冲 alpha。\n    \
           ⟹ 若 (b) 否证：alpha 增量可能藏在未启的次级别对冲，本 v1 全窗结论不覆盖该维度。\n  \
         ⟹ 否证只否定**这套 v1 实装 / 这些品种 / 这个尺度**，**不否定「缠论/完整走势识别无 alpha」**。\n  \
         - 弱反向先验（不迁移）：谱系 553/557 做空 L3 测得 alpha 有效域≈空集，但跑在 recursive_t 引擎\n    \
           **非 theta_v0**，强牛 regime 单一——仅作弱先验，不影响本 v1 口径结论。"
    );

    // 不变量：分层完备（每全窗完成的品种恰归一类 b/c/d；无订单归 a）。
    assert_eq!(
        n_falsify + n_confirm + n_inconclusive + n_engine_block,
        n_full,
        "L3 分层完备：每全窗完成品种恰归一类（a ∪ b ∪ c ∪ d = 全窗完成集）",
    );
    // ★acceptance：≥1 品种端到端产订单流（管线在多标的真实数据上跑通）。
    assert!(
        n_falsify + n_confirm + n_inconclusive >= 1,
        "≥1 品种产订单流（多标的真实数据 L2 检验可执行），实测全部工程断流 ⟹ 接线回退",
    );
}
