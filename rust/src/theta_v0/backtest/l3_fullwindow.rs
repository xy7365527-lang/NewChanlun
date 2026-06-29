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

// ════════════════════════════════════════════════════════════════════════════
//  #5 多声部对冲深度贡献根因诊断（231 诊断非 alpha）
//  诊断 max_depth=3 七链 vs depth=0 baseline ΔSharpe=0/ΔCalmar=0 根因——
//  区分 (a) ρ漂移保守剪枝 vs (b) 结构不产 depth>0 腿。
//  跑法：cargo test --release --lib l3_pi_depth_diag -- --ignored --nocapture
// ════════════════════════════════════════════════════════════════════════════
use super::super::strategy::coverage;
use super::super::strategy::interp;
use super::super::strategy::coverage::{CoverageElement, Vertical};
use super::super::strategy::interp::ActiveLeg;
use super::super::strategy::voice::depth_weight;
use super::super::classifier;
use super::super::parser;

/// #5 深度诊断 per-bar instrument 计数器。
#[derive(Default, Debug)]
struct DepthDiag {
    tower_depth0: u64,
    tower_depth1: u64,
    tower_depth2: u64,
    tower_depth_ge3: u64,
    bars_with_tower_ge2: u64,
    bars_with_empty_tree: u64,
    open_ambient: u64,
    open_followparent: u64,
    open_shortdiff: u64,
    open_total: u64,
    close_total: u64,
    record_total: u64,
    bars_with_any_open: u64,
    active_depth0: u64,
    active_depth1: u64,
    active_depth2: u64,
    active_depth_ge3: u64,
    active_depth0_units: f64,
    active_depth1_units: f64,
    active_depth2_units: f64,
    /// ★坐标系统一修复：restore overlay 腿（来自 restore_ancestor_chain_from_registry / Stale
    /// 追加，id 不在当前 elements[..tree_end] 且非本 bar open 候选）。**单独计数**，不混入
    /// active_depth* 桶——active_depth* 只反映真 1→2→3 AncOK 链准入（与 samebar_depth>0_admit /
    /// sd_parent_held / held_op_parent_alive 同语义，当前均应=0）。
    active_restored: u64,
    /// restore overlay 腿按 parent_id 链（生产同坐标系）算的 depth 分布——诊断 restore 腿真实深度。
    restored_depth0: u64,
    restored_depth1: u64,
    restored_depth2: u64,
    restored_depth_ge3: u64,
    /// ★ChatGPT §11 判据：parent-carrier accepted certificate 数（按 carrier 级别 ℓ+1 分桶）。
    /// = 本 bar open 候选**且被准入 next_active**、其 `parent_id` 非 None（区间套包装成父 carrier
    /// 开仓证书 γ=(c_{ℓ+1}; g_ℓ,...; δ)）的腿数。**区别于 raw_bsp_lvl**：raw BSP 是 classifier 终端
    /// 买卖点；accepted cert 是被解释器包装成父 carrier 证书并经 AncOK 准入的腿。
    /// 判 depth1/2=0 根因：accepted_cert_carrier(ℓ+1)≈0 ⟹ §9.1 正确剪枝(中间级证书天然稀疏 P)；
    /// raw_bsp 有但 accepted_cert=0 ⟹ 证书解释器缺口(区间套未生成 parent-carrier cert，可修)。
    accepted_cert_carrier1: u64,
    accepted_cert_carrier2: u64,
    accepted_cert_carrier_ge3: u64,
    /// 本 bar open 候选中 parent_id 非 None 的总数（**发出**侧分母，未必准入）——对照 accepted。
    open_cert_with_carrier: u64,
    raw_total: u64,
    post_ancok_total: u64,
    pruned_total: u64,
    pruned_depth1: u64,
    pruned_depth_ge2: u64,
    held_exact: u64,
    held_coord_drift: u64,
    held_stale: u64,
    held_total: u64,
    bars_processed: u64,
    // ★persistent overlay（anc.pdf §12）：三类 held 指标拆分。
    /// held_snapshot_exact：snapshot 中找到的 held 腿（旧 held_exact，可能永远不高）。
    held_snapshot_exact: u64,
    /// held_registry_alive：persistent registry 中存活的 held 腿（目标≈100%，§12）。
    held_registry_alive: u64,
    /// held_operation_parent_alive：op_parent 在 registry 中存活的 depth>0 腿（§12）。
    held_operation_parent_alive: u64,
    /// LiveDetached 腿计数（persistent overlay 修复后保留的非 stale detached 腿）。
    held_live_detached: u64,
    /// ★工位 G：shadow post（镜像 host 注入）中 depth>0 腿数——同 bar 容器+子共现 AncOK 准入。
    samebar_depth_gt0_admitted: u64,
    /// ★工位 G：open 候选 level 直方图（容器级买卖点共现诊断）。
    open_lvl0: u64,
    open_lvl1: u64,
    open_lvl_ge2: u64,
    /// ★工位 G：最后一 bar 全窗 classification 各级 bsp 数（自举前置：level>=1 须真有 bsp）。
    bsp_lvl0: u64,
    bsp_lvl1: u64,
    bsp_lvl_ge2: u64,
    n_levels: u64,
    /// ★工位 H 根因区分：ShortDiff 子候选的真 Compose 父（parent_id）是否在 prev_active（已持仓）。
    /// child_parent_held>0 ⟹ 父确实持仓但子仍被剪 = 真注入/对位 bug；
    /// child_parent_held=0 ⟹ 父从不持仓（容器 BSP 太稀疏）= 639(c) 正确剪枝（非 bug，是经验稀疏）。
    sd_parent_held: u64,
    /// ShortDiff 子候选的真父在 registry 中 live（含 LiveDetached，跨 bar 持久身份）。
    sd_parent_registry_alive: u64,
    /// ShortDiff 子候选总数（分母）。
    sd_total: u64,
    // ════════════════════════════════════════════════════════════════════════
    // ★工位 P（ChatGPT§5定理2+§11三层判据）：‖ΔN‖₁ 净头寸位移测量。
    // N_t = 生产净头寸 = Σ_legs sign(dir)·base_units·depth_weight(d)（= net_target_units 在
    // active set 上的有符号聚合，Long=+，Short=−，权重 depth_weight）。
    //   N_t^#5   = 所有 active 腿（含 depth>0）
    //   N_t^base = 只 depth0 腿
    //   ΔN_t = N_t^#5 − N_t^base = Σ_{d>0} sign(dir)·base_units·w(d)
    // 判定：‖ΔN‖₁=0 ∀t ⟹ b2（净额化后净头寸根本没变，NAV 原理上看不见）；
    //       ‖ΔN‖₁>0 但 ΔSharpe=0 ⟹ b1（净敞口被#5改变但经验无 alpha）。
    // ════════════════════════════════════════════════════════════════════════
    /// ‖ΔN‖₁ = Σ_t |ΔN_t|（净头寸位移 L1 范数，决定性 b1/b2 判据）。
    delta_n_l1: f64,
    /// ΔN_t ≠ 0 的 bar 数（净头寸被 depth>0 腿改变的 bar 数）。
    delta_n_nonzero_bars: u64,
    /// |ΔN_t| 量级分布桶（相对 base_units=1000：=0 / (0,1] / (1,100] / (100,1000] / >1000）。
    delta_n_mag_eq0: u64,
    delta_n_mag_le1: u64,
    delta_n_mag_le100: u64,
    delta_n_mag_le1000: u64,
    delta_n_mag_gt1000: u64,
    /// Σ_t N_t^#5 与 Σ_t N_t^base（净头寸序列的累加，诊断方向/抵消）。
    sum_n_full: f64,
    sum_n_base: f64,
}

fn elem_depth(elements: &[CoverageElement], idx: usize) -> u32 {
    let mut d = 0u32;
    let mut cur = elements.get(idx).and_then(|e| e.parent);
    while let Some(p) = cur {
        d += 1;
        cur = elements.get(p).and_then(|e| e.parent);
    }
    d
}

/// ★坐标系统一（本工位修复核心）：在**生产 next_active 自身的 parent_id 链**上算 depth。
///
/// 旧 bug：depth 走另一棵当前 bar 重建的 `elements` 树的 `parent`（索引）链——restore overlay 腿
/// 的 id 若在新树命中深元素，被误算 depth≥3 落 active_depth_ge3 桶（伪证）。生产 AncOK
/// （`ancestor_close_by_id`）的祖先闭包用 `parent_id`（ElementId 结构映射）在 work/next_active
/// **同一集合**上闭合——故 depth 必须用同坐标系：沿 `leg.parent_id` 链，按 id 在 next_active 内反查。
///
/// AncOK 不变量 ⟹ next_active 中存活的 depth-d 腿，其 d 条 parent_id 祖先**也在 next_active**
/// （否则被剪）。故 depth_ge3>0 ⟺ 同集合内有 depth1+depth2 链节——结构上不可能凭空=14166。
/// 边界根（parent_id=None）depth=0；链断（祖先 id 不在集合）按已走步数返回（AncOK 后不应发生）。
fn leg_depth_by_parent_id(
    id_to_parent: &std::collections::HashMap<
        super::super::classifier::recursive_tower::ElementId,
        Option<super::super::classifier::recursive_tower::ElementId>,
    >,
    leg: &ActiveLeg,
) -> u32 {
    let mut d = 0u32;
    let mut cur = leg.parent_id;
    while let Some(pid) = cur {
        d += 1;
        // 按 id 反查父的 parent_id（生产同坐标系，spec §13 结构映射）。
        cur = match id_to_parent.get(&pid) {
            Some(&pp) => pp,
            None => break, // 祖先不在 next_active（AncOK 后不应发生；断链按已走步数）。
        };
    }
    d
}

/// 重放 held_leg_tree_index 二分（codex Q4：按 ElementId 结构映射匹配，spec §13）。
/// 0=Exact（ID 命中） 2=Stale（ID 未匹配）。旧 1=CoordDrift 已删（ID 确定性 ⟹ 父延伸同 ID ⟹ Exact 覆盖）。
fn held_branch(elements: &[CoverageElement], candidate_start: usize, leg: &ActiveLeg) -> u8 {
    let tree_end = candidate_start.min(elements.len());
    let tree = &elements[..tree_end];
    // codex Q4：按 leg.id（跨 bar 稳定的确定性 ElementId）查当前因果树元素。
    if tree.iter().position(|e| e.id == leg.id).is_some() {
        return 0;
    }
    2
}

fn close_indices_mirror(prev_active: &[ActiveLeg], close: &[ActiveLeg]) -> Vec<usize> {
    let mut claimed = vec![false; prev_active.len()];
    let mut idx = Vec::new();
    for d in close {
        for (i, leg) in prev_active.iter().enumerate() {
            if !claimed[i] && leg == d {
                claimed[i] = true;
                idx.push(i);
                break;
            }
        }
    }
    idx
}

/// per-bar instrument：调生产 coverage_step_classification 拿真实 next_active + 采集计数。
fn instrument_bar(
    diag: &mut DepthDiag,
    classification_i: &classifier::Classification,
    tower_i: &[Vec<classifier::recursive_tower::LeveledMove>],
    prev_active: &[ActiveLeg],
    base_units: f64,
    cfg: &ThetaConfig,
    registry: &super::super::strategy::persistent::PersistentRegistry,
) -> Vec<ActiveLeg> {
    let (next_active, _p_tilde) = coverage::coverage_step_classification(
        classification_i,
        tower_i,
        prev_active,
        base_units,
        &cfg.voice,
        registry,
    );
    let (elements, candidate_start) =
        interp::coverage_elements_with_tower(classification_i, tower_i);
    let gamma = interp::assemble_gamma_with_tower(classification_i, tower_i);
    let buckets = interp::interpret(&gamma, prev_active);

    diag.bars_processed += 1;

    let tree = &elements[..candidate_start.min(elements.len())];
    if tree.is_empty() {
        diag.bars_with_empty_tree += 1;
    }
    if tower_i.len() >= 2 {
        diag.bars_with_tower_ge2 += 1;
    }
    for (idx, _e) in tree.iter().enumerate() {
        match elem_depth(&elements, idx) {
            0 => diag.tower_depth0 += 1,
            1 => diag.tower_depth1 += 1,
            2 => diag.tower_depth2 += 1,
            _ => diag.tower_depth_ge3 += 1,
        }
    }

    diag.open_total += buckets.open.len() as u64;
    diag.close_total += buckets.close.len() as u64;
    diag.record_total += buckets.record.len() as u64;
    if !buckets.open.is_empty() {
        diag.bars_with_any_open += 1;
    }
    // ★工位 H：prev_active 持仓腿 id 集（父在场判据：父持仓才放行 depth>0，639(c)）。
    let held_ids: std::collections::HashSet<_> = prev_active.iter().map(|l| l.id).collect();
    for c in &buckets.open {
        let ci = candidate_start + c.gamma_index;
        if ci < elements.len() {
            // ★ChatGPT §11：发出侧分母——open 候选携真 parent_id（被包装成父 carrier 子证书）。
            if elements[ci].parent_id.is_some() {
                diag.open_cert_with_carrier += 1;
            }
            let role = coverage::operation_role(&elements, ci);
            match role.v {
                Vertical::Ambient => diag.open_ambient += 1,
                Vertical::FollowParent => diag.open_followparent += 1,
                Vertical::ShortDiff => {
                    diag.open_shortdiff += 1;
                    // ★根因区分：ShortDiff 子的真 Compose 父（parent_id）是否已持仓 / registry live。
                    diag.sd_total += 1;
                    if let Some(pid) = elements[ci].parent_id {
                        if held_ids.contains(&pid) {
                            diag.sd_parent_held += 1;
                        }
                        if registry.registry_live(&pid) {
                            diag.sd_parent_registry_alive += 1;
                        }
                    }
                }
            }
        }
        // ★工位 G：open 候选 level 直方图（level>=1 候选=容器级买卖点=自举注入源）。
        match c.level {
            0 => diag.open_lvl0 += 1,
            1 => diag.open_lvl1 += 1,
            _ => diag.open_lvl_ge2 += 1,
        }
    }
    // ★工位 G：各级 bsp 实测数量（classifier 输出，自举前置——level>=1 须真有 bsp）。
    // 每 bar 全量重提取 ⟹ 用**最后一 bar**的全窗 classification 计数（overwrite，非 sum，防累加膨胀）。
    diag.bsp_lvl0 = classification_i.levels.first().map(|l| l.bsp.len() as u64).unwrap_or(0);
    diag.bsp_lvl1 = classification_i.levels.get(1).map(|l| l.bsp.len() as u64).unwrap_or(0);
    diag.bsp_lvl_ge2 = classification_i.levels.iter().skip(2).map(|l| l.bsp.len() as u64).sum();
    diag.n_levels = classification_i.levels.len() as u64;

    for leg in prev_active {
        diag.held_total += 1;
        match held_branch(&elements, candidate_start, leg) {
            0 => diag.held_exact += 1,
            1 => diag.held_coord_drift += 1,
            _ => diag.held_stale += 1,
        }
        // ★persistent overlay（anc.pdf §12）：三类 held 指标拆分。
        // held_snapshot_exact：snapshot 中找到（旧 held_exact，可能永远不高）。
        if held_branch(&elements, candidate_start, leg) == 0 {
            diag.held_snapshot_exact += 1;
        }
        // held_registry_alive：persistent registry 中存活（目标≈100%，§12）。
        if registry.registry_live(&leg.id) {
            diag.held_registry_alive += 1;
            // LiveDetached：registry alive 但 snapshot 找不到。
            if held_branch(&elements, candidate_start, leg) != 0 {
                diag.held_live_detached += 1;
            }
        }
        // held_operation_parent_alive：op_parent 在 registry 中存活的 depth>0 腿（§12）。
        if let Some(op_pid) = leg.op_parent {
            if registry.registry_live(&op_pid) {
                diag.held_operation_parent_alive += 1;
            }
        }
    }

    let mut raw: Vec<usize> = Vec::new();
    let closed_idx = close_indices_mirror(prev_active, &buckets.close);
    let tree_end = candidate_start.min(elements.len());
    for (i, leg) in prev_active.iter().enumerate() {
        if closed_idx.contains(&i) {
            continue;
        }
        // ★codex Q4：按 leg.id（确定性 ElementId）匹配当前因果树元素（spec §13 结构映射）。
        let idx_opt = elements[..tree_end].iter().position(|e| e.id == leg.id);
        match idx_opt {
            Some(idx) => {
                if !raw.contains(&idx) {
                    raw.push(idx);
                }
            }
            None => {
                // Stale：★发现 A 修复——不伪造 parent:None。真边界根 ∂ 作根保留，非边界根 prune。
                if leg.is_boundary_root {
                    // 真边界根 ∂：保留（prune 逻辑在 ancestor_close_by_id 按 parent_id 闭包）。
                    // 此处 L3 探针只统计，不重建 work，故不计入 raw（与 coverage_step_from_buckets
                    // 的 prune 一致——非边界根 prune，边界根作根但 L3 探针无 work 追加路径）。
                }
                // 非边界根：prune（不入 raw）。
            }
        }
    }
    // ★工位 H：位置节点身份 = carrier id（级别容器.pdf §13，见 interp 候选 id），候选自身入 raw 即激活
    // carrier 位置节点。删除旧 G 的同级 host 叶子注入（PDF §6/§7 反证的"开叶子"错形式）。
    for c in &buckets.open {
        let idx = candidate_start + c.gamma_index;
        if idx < elements.len() && !raw.contains(&idx) {
            raw.push(idx);
        }
    }
    diag.raw_total += raw.len() as u64;

    let post: Vec<usize> = raw
        .iter()
        .copied()
        .filter(|&e_idx| {
            coverage::ancestors(&elements, e_idx)
                .iter()
                .all(|a| raw.contains(a))
        })
        .collect();
    // ★工位 G 同 bar 自举诊断：shadow post（已镜像 host 注入）中 depth>0 腿数（同 bar 容器+子共现）。
    for &idx in &post {
        match elem_depth(&elements, idx) {
            0 => {}
            _ => diag.samebar_depth_gt0_admitted += 1,
        }
    }
    diag.post_ancok_total += post.len() as u64;

    for &idx in &raw {
        if !post.contains(&idx) {
            match elem_depth(&elements, idx) {
                0 => {}
                1 => diag.pruned_depth1 += 1,
                _ => diag.pruned_depth_ge2 += 1,
            }
        }
    }
    diag.pruned_total += (raw.len() - post.len()) as u64;

    // ★坐标系统一修复（本工位核心）：active_depth* 与 active_restored 都用**生产 next_active 自身**
    // 的坐标系，消除旧 bug 的坐标系分叉——
    //   旧 bug：depth 走另一棵当前 bar 重建的 `elements` 树的 `parent` 索引链（elem_depth）。restore
    //   overlay 腿（restore_ancestor_chain_from_registry / Stale 追加，parent:None）的 id 若在新树命中
    //   深元素，被误算 depth≥3 → active_depth_ge3=14166 伪证（与 active_depth1=active_depth2=0 数学矛盾）。
    //
    // 修复：
    //   ① depth 沿 `leg.parent_id` 链（生产 AncOK ancestor_close_by_id 闭合用的同一字段、同一集合），
    //      用 next_active 内的 id→parent_id 反查（leg_depth_by_parent_id）。
    //   ② 来源标注：open（本 bar open 候选）/ carry（id 在当前因果树 elements[..tree_end]）/ restored
    //      （二者皆非 = restore overlay）。restore 腿**单独计数**（active_restored + restored_depth*），
    //      不混入 active_depth*——active_depth* 只反映真 1→2→3 AncOK 链准入（与 samebar_depth>0_admit /
    //      sd_parent_held / held_op_parent_alive 同语义，当前均应=0，640 反伪证一致性）。
    let next_id_to_parent: std::collections::HashMap<_, _> =
        next_active.iter().map(|l| (l.id, l.parent_id)).collect();
    let open_cand_ids: std::collections::HashSet<_> = buckets
        .open
        .iter()
        .filter_map(|c| elements.get(candidate_start + c.gamma_index).map(|e| e.id))
        .collect();
    // ★工位 P：per-bar 净头寸 N_t = Σ sign(dir)·base_units·w(d)（生产 net_target_units 口径）。
    let mut n_full = 0.0_f64; // 所有 active 腿（含 depth>0）
    let mut n_base = 0.0_f64; // 只 depth0 腿
    for leg in next_active.iter() {
        let d = leg_depth_by_parent_id(&next_id_to_parent, leg);
        // ★净头寸贡献（net_target_units 口径：Long=+，Short=−，Flat=0；units=base_units·w(d)）。
        let sign = match leg.dir {
            super::super::strategy::voice::VoiceSide::Long => 1.0,
            super::super::strategy::voice::VoiceSide::Short => -1.0,
            super::super::strategy::voice::VoiceSide::Flat => 0.0,
        };
        let signed_u = sign * base_units * depth_weight(d, &cfg.voice);
        n_full += signed_u;
        if d == 0 {
            n_base += signed_u;
        }
        let in_tree = elements[..tree_end].iter().any(|e| e.id == leg.id);
        let is_open = open_cand_ids.contains(&leg.id);
        if !in_tree && !is_open {
            // restore overlay 腿（生产 work 追加，非当前因果树/非本 bar 开仓）——单独计数。
            diag.active_restored += 1;
            match d {
                0 => diag.restored_depth0 += 1,
                1 => diag.restored_depth1 += 1,
                2 => diag.restored_depth2 += 1,
                _ => diag.restored_depth_ge3 += 1,
            }
            continue;
        }
        // ★ChatGPT §11：accepted parent-carrier cert——本 bar open 候选且被准入 next_active、
        // 携真 parent_id（包装成父 carrier 子证书 γ=(c_{ℓ+1};...））。按 carrier 级别 ℓ+1 分桶。
        if is_open && leg.parent_id.is_some() {
            match leg.level + 1 {
                1 => diag.accepted_cert_carrier1 += 1,
                2 => diag.accepted_cert_carrier2 += 1,
                _ => diag.accepted_cert_carrier_ge3 += 1,
            }
        }
        // 真 AncOK 链准入腿（open / carry）——按生产坐标系 depth 入 active_depth* 桶。
        let w = depth_weight(d, &cfg.voice);
        let u = base_units * w;
        match d {
            0 => {
                diag.active_depth0 += 1;
                diag.active_depth0_units += u;
            }
            1 => {
                diag.active_depth1 += 1;
                diag.active_depth1_units += u;
            }
            2 => {
                diag.active_depth2 += 1;
                diag.active_depth2_units += u;
            }
            _ => {
                diag.active_depth_ge3 += 1;
            }
        }
    }

    // ★工位 P：ΔN_t = N_t^#5 − N_t^base，累加 ‖ΔN‖₁ 与量级分布。
    let delta_n = n_full - n_base;
    diag.sum_n_full += n_full;
    diag.sum_n_base += n_base;
    diag.delta_n_l1 += delta_n.abs();
    let mag = delta_n.abs();
    if mag < 1e-9 {
        diag.delta_n_mag_eq0 += 1;
    } else {
        diag.delta_n_nonzero_bars += 1;
        if mag <= 1.0 {
            diag.delta_n_mag_le1 += 1;
        } else if mag <= 100.0 {
            diag.delta_n_mag_le100 += 1;
        } else if mag <= 1000.0 {
            diag.delta_n_mag_le1000 += 1;
        } else {
            diag.delta_n_mag_gt1000 += 1;
        }
    }

    next_active
}

/// per-symbol instrument loop（只跑结构变换，不跑成交——depth/role/AncOK 计数与 NAV 协变无关）。
fn instrument_loop(bars: &[super::super::types::Bar], cfg: &ThetaConfig) -> DepthDiag {
    let mut diag = DepthDiag::default();
    let mut prev_active: Vec<ActiveLeg> = Vec::new();
    let base_units = 1000.0_f64;
    // ★persistent overlay（anc.pdf §4-§9）：跨 bar 持久元素注册表。
    let mut registry = super::super::strategy::persistent::PersistentRegistry::new();

    for i in 0..bars.len() {
        let bar = &bars[i];
        if bar.untradable || bar.close == 0 {
            continue;
        }
        let l0_prefix = parser::parse_layer(&bars[..=i], cfg);
        let (classification_i, tower_i) = classifier::classify_with_tower(&l0_prefix, cfg);
        // 诊断用全量 classification_i（非 newly_confirmed_step diff——该函数私有）。
        // 诚实标注：open_*/active_* 是 per-bar 全量候选/活动角色，非新增订单 diff。结构诊断用。
        prev_active = instrument_bar(
            &mut diag,
            &classification_i,
            &tower_i,
            &prev_active,
            base_units,
            cfg,
            &registry,
        );
        // ★persistent overlay merge：Pi+1 = merge(Pi, Ei+1, held legs)。
        let (elements_ref, _cstart) =
            interp::coverage_elements_with_tower(&classification_i, &tower_i);
        registry = registry.merge(&elements_ref, &prev_active);
    }

    diag
}

/// **★#5 深度贡献根因诊断：CL + BTC 32K OOS instrument 计数（区分 (a) ρ漂移剪枝 vs (b) 结构不产）**。
///
/// `#[ignore]`，需 CL/BTC 数据缓存；O(n²) 慢测 `--release` 必须。
/// 跑法：`cargo test --release --lib l3_pi_depth_diag -- --ignored --nocapture`
#[test]
#[ignore = "#5 深度诊断；O(n²) CL/BTC 32K；需数据缓存；--release"]
fn l3_pi_depth_diag_cl_btc() {
    let config = ThetaConfig::default();
    const MAX_BARS: usize = 32_000;

    eprintln!("\n===== #5 深度贡献根因诊断：max_depth=3 七链 depth>0 腿为何 ΔSharpe=0 =====");
    eprintln!("★区分 (a) ρ漂移保守剪枝（CoordDrift 计数应低=修复未生效）vs (b) 结构不产（tower depth>=2 少/ShortDiff=0）");
    eprintln!("★诚实标注：open_*/active_* 是 per-bar 全量候选/活动角色（非新增订单 diff），结构诊断用。");

    for w in PREREG_WINDOWS.iter().filter(|w| w.symbol == "CL" || w.symbol == "BTC") {
        let ds = match data::load_by_symbol(w.symbol, &config) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{:<6} 加载失败：{e}（DATA BLOCKER）", w.symbol);
                continue;
            }
        };
        let oos_full = ds.slice_date_window(w.oos.0, w.oos.1);
        if oos_full.bars.is_empty() {
            continue;
        }
        let cut = MAX_BARS.min(oos_full.bars.len());
        let oos = Dataset {
            symbol: oos_full.symbol.clone(),
            bars: oos_full.bars[..cut].to_vec(),
            dates: oos_full.dates[..cut.min(oos_full.dates.len())].to_vec(),
        };

        eprintln!("\n──── {:<6} ({} bars, OOS {}→{}, cut={}) ────", w.symbol, oos.bars.len(), w.oos.0, w.oos.1, cut);

        let t0 = std::time::Instant::now();
        let diag = instrument_loop(&oos.bars, &config);
        let elapsed = t0.elapsed().as_secs_f64();

        eprintln!("  bars_processed         : {}", diag.bars_processed);
        eprintln!();
        eprintln!("  ── (1) 塔结构深度分布 ──");
        eprintln!("  bars_with_tower_ge2    : {}  ({:.1}% of bars)", diag.bars_with_tower_ge2, 100.0 * diag.bars_with_tower_ge2 as f64 / diag.bars_processed.max(1) as f64);
        eprintln!("  bars_with_empty_tree   : {}", diag.bars_with_empty_tree);
        eprintln!("  tower_depth0 (根)      : {}", diag.tower_depth0);
        eprintln!("  tower_depth1 (子)      : {}", diag.tower_depth1);
        eprintln!("  tower_depth2 (孙)      : {}", diag.tower_depth2);
        eprintln!("  tower_depth_ge3        : {}", diag.tower_depth_ge3);
        eprintln!();
        eprintln!("  ── (2) 候选角色（open 桶各 V，#5 alpha 源=ShortDiff）──");
        eprintln!("  bars_with_any_open     : {}", diag.bars_with_any_open);
        eprintln!("  open_total             : {}", diag.open_total);
        eprintln!("  open_ambient           : {}", diag.open_ambient);
        eprintln!("  open_followparent      : {}", diag.open_followparent);
        eprintln!("  ★open_shortdiff        : {}  (反向对冲腿源)", diag.open_shortdiff);
        eprintln!("  close_total            : {}", diag.close_total);
        eprintln!("  record_total           : {}", diag.record_total);
        eprintln!();
        eprintln!("  ── (3) active 集深度贡献（AncOK 后真链准入腿，按生产 next_active parent_id 链算 depth）──");
        eprintln!("  ★坐标系统一：depth 沿 leg.parent_id 链（生产 AncOK 同字段同集合），restore overlay 单独计");
        eprintln!("  active_depth0 腿       : {}  units={:.2}", diag.active_depth0, diag.active_depth0_units);
        eprintln!("  active_depth1 腿       : {}  units={:.2}  (w=0.30)", diag.active_depth1, diag.active_depth1_units);
        eprintln!("  active_depth2 腿       : {}  units={:.2}  (w=0.10)", diag.active_depth2, diag.active_depth2_units);
        eprintln!("  ★active_depth_ge3 腿  : {}  (真 1→2→3 AncOK 链准入；应=0 与 samebar/sd_parent_held 一致)", diag.active_depth_ge3);
        eprintln!("  ── restore overlay 腿（生产 work 追加，单独计，不混入 active_depth*）──");
        eprintln!("  ★active_restored     : {}  (restore_ancestor_chain / Stale 腿，非真 carry/open)", diag.active_restored);
        eprintln!("    restored depth0/1/2/>=3 : {} / {} / {} / {}  (restore 腿按 parent_id 链真实 depth)",
            diag.restored_depth0, diag.restored_depth1, diag.restored_depth2, diag.restored_depth_ge3);
        let total_u = diag.active_depth0_units + diag.active_depth1_units + diag.active_depth2_units;
        if total_u > 0.0 {
            eprintln!("  depth0 占比            : {:.1}%", 100.0 * diag.active_depth0_units / total_u);
            eprintln!("  depth1 占比            : {:.1}%", 100.0 * diag.active_depth1_units / total_u);
            eprintln!("  depth2 占比            : {:.1}%", 100.0 * diag.active_depth2_units / total_u);
        }
        eprintln!();
        eprintln!("  ── (4) AncOK 剪枝 ──");
        eprintln!("  raw_total              : {}", diag.raw_total);
        eprintln!("  post_ancok_total       : {}", diag.post_ancok_total);
        eprintln!("  pruned_total           : {}", diag.pruned_total);
        eprintln!("  ★pruned_depth1        : {}", diag.pruned_depth1);
        eprintln!("  ★pruned_depth_ge2     : {}", diag.pruned_depth_ge2);
        eprintln!();
        eprintln!("  ── (5) held_leg_tree_index 分支 ──");
        eprintln!("  held_total             : {}", diag.held_total);
        eprintln!("  held_exact (ρ未漂移)   : {}", diag.held_exact);
        eprintln!("  ★held_coord_drift     : {}  (ρ漂移=父延伸)", diag.held_coord_drift);
        eprintln!("  held_stale (父真失效)  : {}", diag.held_stale);
        eprintln!();
        eprintln!("  ── (5b) ★persistent overlay 指标（anc.pdf §12）──");
        eprintln!("  held_snapshot_exact    : {}  (snapshot 中找到，可能永远不高)", diag.held_snapshot_exact);
        eprintln!("  ★held_registry_alive  : {}  (目标≈100%——未显式关闭的腿)", diag.held_registry_alive);
        eprintln!("  ★held_op_parent_alive  : {}  (depth>0 腿的 op_parent 在 registry 存活)", diag.held_operation_parent_alive);
        eprintln!("  held_live_detached     : {}  (LiveDetached——修复后保留非 stale)", diag.held_live_detached);
        eprintln!("  ★samebar_depth>0_admit : {}  (工位G: 同 bar 容器BSP+子共现 AncOK 准入 depth>0)", diag.samebar_depth_gt0_admitted);
        eprintln!();
        eprintln!("  ── (6) ★工位 G：open 候选 level 直方图 + 全窗各级 bsp ──");
        eprintln!("  open_lvl0 / lvl1 / lvl>=2 : {} / {} / {}", diag.open_lvl0, diag.open_lvl1, diag.open_lvl_ge2);
        eprintln!("  全窗 n_levels            : {}", diag.n_levels);
        eprintln!("  全窗 bsp_lvl0 / lvl1 / >=2: {} / {} / {}", diag.bsp_lvl0, diag.bsp_lvl1, diag.bsp_lvl_ge2);
        eprintln!();
        eprintln!("  ── (6b) ★ChatGPT §11：raw BSP vs parent-carrier accepted certificate（判 depth1/2=0 根因）──");
        eprintln!("  open_cert_with_carrier  : {}  (发出侧：open 候选携真 parent_id，被包装成父 carrier 子证书)", diag.open_cert_with_carrier);
        eprintln!("  ★accepted_cert carrier ℓ+1=1 / 2 / >=3 : {} / {} / {}  (准入 next_active 的父 carrier 子证书，按 carrier 级别)",
            diag.accepted_cert_carrier1, diag.accepted_cert_carrier2, diag.accepted_cert_carrier_ge3);
        eprintln!("  判据：accepted_cert(ℓ+1)≈0 ⟹ §9.1 正确剪枝(中间级证书天然稀疏 P，接受)；");
        eprintln!("        open_cert_with_carrier>0 但 accepted_cert=0 ⟹ 区间套生成证书但 AncOK 全剪(查父持仓)；");
        eprintln!("        raw_bsp_lvl(ℓ+1)>0 但 open_cert_with_carrier=0 ⟹ 解释器未包装成 parent-carrier cert(可修缺口)。");
        eprintln!();
        eprintln!("  ── (7) ★工位 H 根因区分：ShortDiff 子的真父是否已持仓 ──");
        eprintln!("  sd_total               : {}  (ShortDiff 子候选总数)", diag.sd_total);
        eprintln!("  ★sd_parent_held       : {}  (>0=父持仓但子被剪=真bug；=0=父从不持仓=639(c)正确剪枝)", diag.sd_parent_held);
        eprintln!("  sd_parent_registry_alive: {}  (父在 registry live，含 LiveDetached)", diag.sd_parent_registry_alive);
        eprintln!("  [{:.1}s]", elapsed);

        eprintln!();
        eprintln!("  ── (8) ★工位 P：‖ΔN‖₁ 净头寸位移（ChatGPT§5定理2，决定性 b1/b2 判据）──");
        eprintln!("  N_t = Σ sign(dir)·base_units·w(d)（net_target_units 口径，base_units={:.0}）", 1000.0_f64);
        eprintln!("  ★‖ΔN‖₁ = Σ|N_t^#5 − N_t^base| : {:.4}", diag.delta_n_l1);
        eprintln!("  ΔN_t≠0 的 bar 数             : {} / {} bars", diag.delta_n_nonzero_bars, diag.bars_processed);
        eprintln!("  Σ N_t^#5 / Σ N_t^base        : {:.2} / {:.2}", diag.sum_n_full, diag.sum_n_base);
        eprintln!("  |ΔN_t| 量级分布 =0 / (0,1] / (1,100] / (100,1000] / >1000 : {} / {} / {} / {} / {}",
            diag.delta_n_mag_eq0, diag.delta_n_mag_le1, diag.delta_n_mag_le100,
            diag.delta_n_mag_le1000, diag.delta_n_mag_gt1000);
        if diag.delta_n_l1 < 1e-9 {
            eprintln!("  → ★b2：‖ΔN‖₁=0 ∀t — depth>0 腿存在但净额化后净头寸根本没变（多空腿压缩）。#5 alpha（若有）在毛分账本层，需 overlay IR/保证金归一化度量（非 standalone Sharpe）。");
        } else {
            eprintln!("  → ★b1：‖ΔN‖₁={:.4}>0 — 净敞口确被 #5 改变，但 ΔSharpe=0 ⟹ 经验上无净值收益（成本/方向/时点/权重抵消）。#5 overlay 在当前数据/1min/这些品种无 alpha。", diag.delta_n_l1);
        }

        eprintln!();
        eprintln!("  ── 裁定 ──");
        let depth_active = diag.active_depth1 + diag.active_depth2 + diag.active_depth_ge3;
        let pruned_depth_gt0 = diag.pruned_depth1 + diag.pruned_depth_ge2;
        if diag.open_shortdiff == 0 && depth_active == 0 {
            eprintln!("  → (b) 结构不产：open_shortdiff=0 + active depth>0=0。ShortDiff 候选根本不产生（塔缺真 Compose 父 / hostOf 恒根 / V 恒 Ambient）。");
        } else if diag.open_shortdiff > 0 && depth_active == 0 && pruned_depth_gt0 > 0 {
            eprintln!("  → (a) AncOK 剪枝：ShortDiff 产生但 depth>0 腿全剪。查 CoordDrift={}（低=ρ漂移未根治致父不在 raw）。", diag.held_coord_drift);
        } else if diag.open_shortdiff > 0 && depth_active == 0 && pruned_depth_gt0 == 0 {
            eprintln!("  → (b/候选未入桶) ShortDiff 产生 + 未被 AncOK 剪 + 但 active depth>0=0：候选未入 open 桶（interpret record/close）或 prev_active 未持父。");
        } else if depth_active > 0 {
            // ★codex 复审 N 报告 bug 修复：本分支原打印 "(net-cancel)"，忽略上方 (8)
            // 已基于 delta_n_l1 判出的 b1/b2，与 line 877-881 自相矛盾（工位 N 误采信此行）。
            // 净头寸是否真被改变由 ‖ΔN‖₁ 唯一判定，不由 "depth_active>0 && ΔSharpe=0" 推 net-cancel。
            if diag.delta_n_l1 < 1e-9 {
                eprintln!("  → (b2) active depth>0={} 但 ‖ΔN‖₁=0：净额化抹平，#5 净头寸未变（见上 b2）。", depth_active);
            } else {
                eprintln!("  → (b1) active depth>0={}，‖ΔN‖₁={:.0}>0：净敞口确被改变，ΔSharpe=0 是经验无 alpha（非 net-cancel 抵消，见上 b1）。", depth_active, diag.delta_n_l1);
            }
        } else {
            eprintln!("  → (未分类) 人工裁定。");
        }
    }

    assert!(true, "诊断完成");
}
