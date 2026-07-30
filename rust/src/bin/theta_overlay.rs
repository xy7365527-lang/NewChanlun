//! # `theta_overlay` —— M5 声部执行层跑批 CLI（多空对冲.pdf p16 关卡10 / §M5）
//!
//! 驱动 `run_theta_v0_pi_overlay`：hedge-mode 逐声部账本 P^sep → N=Net(P^sep) → Order_t=ΔN，
//! 逐声部保 entry_v/exit_v/parent(v)/role(v)/pnl_v。产出：
//! - ΔN 守恒（fill loop 内 debug_assert 逐 bar；release 构建不 assert，但净额路径 bit-exact 保证）；
//! - Σpnl_v 对账净额价格 PnL（PDF §11 线性恒等 `Σσ_v q_v ΔP=N ΔP`，reconcile_residual < eps）；
//! - 逐声部归因表（按角色 role(v) 聚合 + 明细）。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin theta_overlay -- <SYMBOL> [START END]
//! 例：cargo run --release --features backtest_bin --bin theta_overlay -- BTC 2023-01-01 2023-02-28
//! ```
//! ★复杂度警告：per-bar 前缀因果重分类 O(n²)（run_theta_v0_pi 同）——全 OOS（2.5yr≈1.3M bar）
//! 不可行，用有界 OOS 切片（首轮执行层真实化，非全量 alpha 声明）。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::runner::run_theta_v0_pi_overlay;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::strategy::coverage::Vertical;
use std::collections::BTreeMap;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 4 {
        eprintln!(
            "用法: {} <SYMBOL> [START END]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO\n  日期窗(可选): ISO 闭区间",
            args.first().map(String::as_str).unwrap_or("theta_overlay")
        );
        return std::process::ExitCode::from(2);
    }
    let symbol = &args[1];
    let window = if args.len() == 4 {
        Some((args[2].as_str(), args[3].as_str()))
    } else {
        None
    };
    let config = ThetaConfig::default();

    let full = match load_by_symbol(symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let dataset = match window {
        Some((s, e)) => {
            if s > e {
                eprintln!("日期窗非法: {s} > {e}");
                return std::process::ExitCode::from(2);
            }
            full.slice_date_window(s, e)
        }
        None => full,
    };
    if dataset.bars.is_empty() {
        eprintln!("品种 {symbol} 窗内无 bar，无法回测。");
        return std::process::ExitCode::FAILURE;
    }

    let years = (dataset.bars.len() as f64
        / newchan_rust::theta_v0::backtest::data::bars_per_year(dataset.bar_seconds))
    .max(1e-9);
    let first_px = dataset.bars[0].close as f64 * config.tick.tick_size;
    let initial_nav = (first_px.abs() * 10_000.0).max(1.0e6);

    let r = run_theta_v0_pi_overlay(&dataset, &config, years, initial_nav);

    println!("=== M5 声部执行层跑批（run_theta_v0_pi_overlay）===");
    println!("品种            : {}", r.symbol);
    match window {
        Some((s, e)) => println!("日期窗          : {s} .. {e}"),
        None => println!("日期窗          : 全量"),
    }
    println!("bar 数          : {}", r.n_bars);
    // #313（#305 评审 LOW-2）：`net_result` 的口径随 env `VOICE_EXEC` 分叉——backtest/runner.rs 的
    // `net_result`/`voice_exec` 字段注释明载：VOICE_EXEC=1 时本结构承载**声部执行投影**
    // （n_orders = 声部 fill 事件数 ≡ `voice_exec.n_voice_fills`，equity/trade_pnls/r_decomp =
    // 声部账户），决策层（typed_ledger/TW/sep_legs）仍与净额臂逐字节一致；VOICE_EXEC=0（默认）
    // 时本段读数逐字节即净额执行层，与 run_theta_v0_pi bit-exact（该路径语义声明不变）。
    // 标签照实分列，两读数禁互相冒充（090 声明=能力）。
    println!(
        "--- 执行层读数（net_result：VOICE_EXEC=0 默认=净额执行层，与 run_theta_v0_pi bit-exact；=1 时为声部执行投影）---"
    );
    println!("净额/声部订单数 : {}", r.net_result.n_orders);
    println!("成交交易笔数    : {}", r.net_result.metrics.n_trades);
    println!("strat_return    : {:.6}", r.net_result.metrics.strat_return);
    println!("--- ★M5 overlay 逐声部账本（多空对冲.pdf p16 关卡10）---");
    println!("活动声部数      : {}", r.overlay.active_voices().count());
    println!("已离场声部数    : {}", r.overlay.closed_voices().len());
    println!("overlay 声部总数: {}", r.n_overlay_voices); // (#295 修字段名；#305 评审 LOW-1 口径注：本值≡活动+已离场声部数，恰与上两行之和相等；gap4/FIX §4.3 双字段方案（n_overlay_fill_events+补行）未采用——数值影响为零，选择单字段直观口径，登记在案）
    println!("终态净敞口 N    : {}", r.overlay.net());
    // ★LEE M1/M3 结果包（multi-level-native-execution-design-20260719 §D，#644 语义重放）：
    //   #289 影子评审 MED ① 要求恒等证据在 **release** 下非平凡可读——两组读数都在 fill loop
    //   内逐决策点累计（非 debug_assert，release 同样执行），「残差恒 0」必须配「量级 > 0」
    //   才算见证成立。
    //   ⚠#644 票面边界：LEE M2/M4（`level_order`/`level_risk`，物理订单量 Σ_ℓΔq_ℓ 生成 + 级别
    //   sizing/风险帽）**不在本票范围**（归 #755，接线会改变订单流本身，越过「只读」边界）。
    //   本 CLI 原有的「LEE M2 订单归因」打印段消费的 `OverlayRunResult::level_order` 字段随
    //   #614 合并已不存在于 main（main 侧只读接线只暴露 level_ledger/level_clock/归因诊断）
    //   ⟹ 该段一并移除，非本车新引入的能力削减。
    let lee = r.level_ledger.lee_net_witness();
    println!("--- ★LEE M1 级别账本（Σ_ℓ net_ℓ ≡ N 加性细化；§D M1，只读旁路）---");
    println!(
        "活动级别桶      : {:?}",
        r.level_ledger.levels().collect::<Vec<_>>()
    );
    println!(
        "LEE-Net 见证    : obs={} max|Σ_ℓ net_ℓ−N|={} max|N|={} ⟹ {}",
        lee.n_observations,
        lee.max_abs_residual,
        lee.max_abs_net,
        if lee.identity_witnessed() {
            "PASS（残差 0 且非平凡）"
        } else {
            "FAIL/平凡（残差≠0 或 max|N|=0）"
        }
    );
    println!("--- ★LEE M3 clock_ℓ 事件钟（§D M3，只读累计——不门控本臂订单流）---");
    println!(
        "决策点/结构钟/风控钟 : {} / {} / {} ⟹ {}",
        r.level_clock.n_decisions,
        r.level_clock.n_bars_with_structural_tick,
        r.level_clock.n_bars_with_risk_tick,
        if r.level_clock.sparsity_witnessed() {
            "PASS（结构钟响过且严格稀疏）"
        } else {
            "FAIL/平凡（恒不响或每 bar 都响）"
        }
    );
    println!("--- ★LEE 归因算子（`level_attrib::attribute_total`，#644 只读诊断）---");
    // ★#758 issue766 LOW-2：残差桶⊆缩放（attribute_total 的 Σbasis=0 分支同时置两标记，
    // 见 level_attrib.rs 文档）——「缩放」数不是「残差桶」数之外的独立计数，非零残差桶时
    // 两数不应被误读为可相加的互斥子集。
    println!(
        "决策点/残差桶/缩放(⊇残差桶) : {} / {} / {}",
        r.level_attrib_n_bars, r.level_attrib_n_residual_bars, r.level_attrib_n_rescaled_bars
    );
    println!("--- ★M7 treasury 层（三阶段 TW 账本终态，overlay 臂主 loop 内建）---");
    match &r.tw_final {
        Some(tw) => println!(
            "终Stage={:?} Q_T(notional_in)={} W_T(withdrawn)={} η_T(tw)={} free={} holding={} open_legacy={}",
            tw.stage, tw.notional_in, tw.withdrawn, tw.tw(), tw.free, tw.holding, tw.open_legacy_legs,
        ),
        None => println!("（tw_final=None——窗内无 bar）"),
    }
    println!("--- ★验收断言2：Σpnl_v 对账（PDF §11 线性恒等 Σσ_v q_v ΔP = N ΔP）---");
    println!("账户净额价格PnL : {:.6}", r.account_price_pnl);
    println!("Σ_v pnl_v       : {:.6}", r.total_voice_pnl);
    println!("对账残差        : {:.3e}", r.reconcile_residual);
    if r.reconcile_residual < 1e-3 {
        println!("对账: PASS（残差 < 1e-3，逐声部归因与净额价格 PnL 一致）");
    } else {
        println!("对账: FAIL（残差过大——检查步序/取整/结合律）");
    }

    // 逐声部归因按角色 role(v) 聚合（entry_v/exit_v/parent(v)/pnl_v 的 role 维汇总）。
    let mut by_role: BTreeMap<&str, (usize, f64)> = BTreeMap::new();
    for c in r.overlay.closed_voices() {
        let key = match c.role_v {
            Vertical::Ambient => "Ambient(根声部)",
            Vertical::FollowParent => "FollowParent(顺父)",
            Vertical::ReverseOpen => "ShortDiff(反向子/对冲)",
        };
        let e = by_role.entry(key).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += c.pnl_v;
    }
    println!("--- ★逐声部归因（按 role(v) 聚合，已离场声部）---");
    if by_role.is_empty() {
        println!("（无已离场声部——窗内无完整声部生命周期，或全部仍活动）");
    } else {
        for (role, (n, pnl)) in &by_role {
            println!("{:<26} 声部数={:>5}  Σpnl_v={:>14.4}", role, n, pnl);
        }
    }
    // 明细样本（前 10 条离场声部，PDF §M5 entry_v/exit_v/parent(v)/role(v)/pnl_v）。
    println!("--- 逐声部明细（前 10 条离场声部）---");
    for c in r.overlay.closed_voices().iter().take(10) {
        let parent = c
            .parent_id
            .map(|p| format!("L{}#{}", p.level, p.ordinal))
            .unwrap_or_else(|| "∂根".to_string());
        println!(
            "v=L{}#{:<4} side={:?} role={:?} parent={} entry_bar={} exit_bar={} entry_px={:.2} exit_px={:.2} pnl_v={:.4}",
            c.id.level, c.id.ordinal, c.side, c.role_v, parent, c.entry_bar, c.exit_bar, c.entry_px, c.exit_px, c.pnl_v
        );
    }
    println!("--- 认识论 L1（formalization-validity-domain 231号）---");
    println!("ΔN 守恒 + Σpnl_v 对账 = 结构恒等（构造性+线性代数），零信息增量。");
    println!(
        "★执行层首次真实化——不声明 alpha（PDF §9：声部生成层+净额可见层验收，经济有效层须另证）。"
    );

    std::process::ExitCode::SUCCESS
}
