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
    /// ★codex ad4afb58：生产 next_active 中 id 不在当前 elements[..tree_end] 的腿
    ///（来自 restore_ancestor_chain_from_registry 追加的 work 元素）。shadow 探针低估的量。
    active_restored: u64,
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
    for c in &buckets.open {
        let ci = candidate_start + c.gamma_index;
        if ci < elements.len() {
            let role = coverage::operation_role(&elements, ci);
            match role.v {
                Vertical::Ambient => diag.open_ambient += 1,
                Vertical::FollowParent => diag.open_followparent += 1,
                Vertical::ShortDiff => diag.open_shortdiff += 1,
            }
        }
    }

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

    // ★codex ad4afb58 修复：active_depth* 统计循环源从 shadow `post`（自重建 AncOK，不调
    // restore_ancestor_chain_from_registry、用索引 ancestors）改为**生产 next_active**。
    // 生产 next_active 是 coverage_step_classification 返回的真 A_{t+1}（经 ancestor_close_by_id
    // + restore_ancestor_chain_from_registry）。shadow post 系统性低估 active_depth1。
    // ponytail: ceiling=按 leg.id 在 elements[..tree_end] 查 idx 算 elem_depth；registry restored
    // 的腿（id 不在 elements）单独计 active_restored（其 depth 需遍历 work 元素才能算，非热路径）。
    for leg in next_active.iter() {
        let idx_opt = elements[..tree_end].iter().position(|e| e.id == leg.id);
        match idx_opt {
            Some(idx) => {
                let d = elem_depth(&elements, idx);
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
            None => {
                // 来自 restore_ancestor_chain_from_registry 的腿（id 不在当前 elements snapshot）。
                diag.active_restored += 1;
            }
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
        eprintln!("  ── (3) active 集深度贡献（AncOK 后净活动腿）──");
        eprintln!("  active_depth0 腿       : {}  units={:.2}", diag.active_depth0, diag.active_depth0_units);
        eprintln!("  active_depth1 腿       : {}  units={:.2}  (w=0.30)", diag.active_depth1, diag.active_depth1_units);
        eprintln!("  active_depth2 腿       : {}  units={:.2}  (w=0.10)", diag.active_depth2, diag.active_depth2_units);
        eprintln!("  ★active_restored     : {}  (registry 恢复祖先腿，shadow 探针漏计)", diag.active_restored);
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
        eprintln!("  [{:.1}s]", elapsed);

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
            eprintln!("  → (net-cancel) active depth>0 腿存在（{}）但 ΔSharpe=0：查 net_target_units 净额抵消（权重 0.30/0.10 太小或方向同向）。", depth_active);
        } else {
            eprintln!("  → (未分类) 人工裁定。");
        }
    }

    assert!(true, "诊断完成");
}
