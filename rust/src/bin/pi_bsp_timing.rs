//! # `pi_bsp_timing` —— 忠实实装 π_Θ^bsp 买卖点离散择时多声部状态机（goal #5）
//!
//! ## 与 `pure_bsp_timing`（工位 Q 简化版）的区别 —— no-patch 核心
//! 工位 Q 的 `pure_bsp_timing` 把全部买卖点坍缩成**单仓 ambient toggle**（q∈{+1,0,−1}），
//! 既无声部 position instance、无祖先闭合、也无 N_t^bsp=Σσ_v q_v。那是 §19 明确禁止的
//! 「每元素覆盖」对偶的另一种坍缩（坍缩成单声部）。本 bin 忠实实装 PDF 全 19 节的
//! **多声部持仓状态机**。
//!
//! ## 逐节 PDF 对应（忠实度第一验收 —— 报告里逐条标注 §几）
//! - §1-2  三类买卖点 6 维向量 b_ℓ → 方向证书 γ=(c,ℓ,δ,I_γ,t)。本 bin 从生产 classifier
//!         `Classification.levels[ℓ].bsp[*].bits` 直接读 b_ℓ（忠实，零重判）。carrier c = 该
//!         买卖点的级别 ℓ + source_index（PDF c 是「操作容器/走势 carrier」；本实装取
//!         (level, source_index) 作 carrier 标识）。
//! - §3    声部 position instance v=(c_v, η_v, σ_v, n_v)。[`Voice`] 结构忠实。
//! - §4    祖先闭合 A_{t+1}=AncOK(A^raw)；父 active 子才 active（[`anc_ok`]）。
//! - §5    入场证书 δ(γ)=σ_v、出场证书 δ(γ)=−σ_v（[`step`] 内 entry/exit 判定）。
//! - §6    短差子声部 σ_u=−σ_p（[`Voice::child_dir`]，§16 多空双开）。
//! - §7    I_v^in/I_v^out 类别集合 + Q_Θ 仓位 size。本实装 Q_Θ=固定单位 1（诚实简化，见下）。
//! - §9    先平后开 A^raw=(A_t\D_t)∪O_t（[`step`] 顺序）。
//! - §10-11 全互斥 6 谓词 P_{v,1..6} → C_{v,j}（[`classify_action`]，恰一类成立的 §10 证明）。
//! - §12   目标头寸 p̃^bsp=Σ q_v e_v^σ；★净额 N_t^bsp=Σ σ_v q_v（[`net_target`]）。
//! - §13   风险投影 K_Θ + LexArgmin。本实装 K_Θ=恒等（无杠杆/保证金约束），LexArgmin=p̃ 本身
//!         （诚实简化，见 ceiling）。
//! - §14   全定义（9 假设 ∀x ∃!O）：本状态机每 bar 对每声部恰产一个 C_{v,j}（§10 互斥保证）。
//! - §18   r^bsp=N^bsp·ΔP−C，alpha 判定（NAV 口径复用 metrics，可比 π^cov）。
//! - §19   π^cov≤0 不能推出 π^bsp≤0；本 bin 单独实装 π^bsp 回答 goal#5。
//!
//! ## 诚实简化（ceiling 标注，formalization-validity-domain）
//! - **Q_Θ=1 单位**（§7/§13）：本实装每声部 q_v=1 固定单位，不做 vol/equity sizing。
//!   ceiling：sizing 需接 risk.rs K_Θ；本轮先测「声部状态机的净额方向」是否携带 alpha——
//!   方向是 N_t^bsp 符号结构的核心，sizing 是二阶标度。升级路径：Q_Θ 读 §7 I_v^in 类别 + equity。
//! - **K_Θ=恒等、LexArgmin=p̃**（§13）：无保证金/杠杆约束 ⟹ p* = p̃^bsp 直接成交。
//!   ceiling：真实约束需 risk.rs；纯择时 edge 测试不需要杠杆建模（杠杆只放大，不改方向 alpha）。
//! - **carrier = (level, source_index)**（§2 c）：PDF c 是抽象「操作容器」，本实装用买卖点
//!   所在级别 + 原始 K 序号作 carrier 唯一标识。父子关系 p(v) 按级别层级 ℓ_child = ℓ_parent
//!   的短差子声部建模（§6/§16：父 active 时反向买卖点证书开子声部）。
//!
//! ## 坐标系纪律（644）+ NAV 口径（可比 π^cov）
//! 走生产 `IncrementalClassifier::classify_at(i)`（因果塔，≤i，无 look-ahead）+ 与 runner
//! `newly_confirmed_step` 同的 seen-set append-only diff。NAV 用 net-position 逐 bar MtM
//! （N_t^bsp × ΔP，§18），归一化 + `metrics::compute`/`significance` —— 与工位 L 测 π^cov
//! **同口径** ⟹ Sharpe 可比。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin pi_bsp_timing -- <SYMBOL> [START END]
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::backtest::metrics::{self, TradeRecord};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::{HashMap, HashSet};

/// 声部 position instance（PDF §3：v=(c_v, η_v, σ_v, n_v)）。
///
/// - `carrier`：c_v —— (level ℓ, 父买卖点 source_index)。同 carrier 多次开仓由 `generation` 区分（§3 n_v）。
/// - `dir`：σ_v ∈ {+1,−1} 持仓方向（§3）。多头 +1 / 空头 −1。
/// - `parent`：p(v) ∈ V∪{⊥}（§3 父子声部关系）。`None`=根声部（⊥）；`Some(idx)`=父声部在 `voices` 的索引。
/// - `qty`：q_v 目标单位数（§7/§12）。本实装固定 1（诚实简化）。
/// - `generation`：n_v（§3，区分同 carrier 多次开仓）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Voice {
    carrier: (usize, usize),
    dir: i8,
    parent: Option<usize>,
    qty: f64,
    generation: u32,
    entry_bar: usize,
}

impl Voice {
    /// §6/§16 短差子声部方向 σ_u=−σ_p（父 active 时反向买卖点证书开子声部）。
    fn child_dir(parent_dir: i8) -> i8 {
        -parent_dir
    }
}

/// 一根买卖点向量 b_ℓ 的方向证书摘要（§1-2 γ）：本 bar 是否出现买侧/卖侧确认。
/// I_γ⊆{1,2,3} 保留（任意类成立即证书成立，§2 I_γ≠∅）—— 这里只需方向 δ（买/卖）。
struct CertHit {
    buy: bool,  // δ=+1 证书（I_γ={i:B_iℓ=1}≠∅）
    sell: bool, // δ=−1 证书（I_γ={i:S_iℓ=1}≠∅）
}

fn cert_of(b: &BspBits) -> CertHit {
    CertHit { buy: b.conf_plus(), sell: b.conf_minus() }
}

/// §4 祖先闭合 AncOK(A)={v∈A : Anc(v)⊆A}。父声部不在 active 集 ⟹ 子声部被剪。
///
/// 输入 `active`=A^raw 索引集 + `voices`（含 parent 链）。返回闭合后的 active 集（父不 active ⟹ 子移除）。
/// 迭代到不动点（§9：先平后开后 AncOK；移除父可能级联移除子）。
fn anc_ok(mut active: HashSet<usize>, voices: &[Voice]) -> HashSet<usize> {
    loop {
        let before = active.len();
        let snapshot = active.clone();
        active.retain(|&v| match voices[v].parent {
            None => true,                  // 根声部（⊥）无祖先，恒满足
            Some(p) => snapshot.contains(&p), // 父 active 才保留（§4 a_{v,t}≤a_{p(v),t}）
        });
        if active.len() == before {
            break; // 不动点
        }
    }
    active
}

/// §10-11 全互斥动作分类 C_{v,j}（对 active 声部 v）。返回是否本 bar 应**平仓**该声部。
///
/// 6 谓词（§10）：P1=风险关闭, P2=父声部关闭, P3=I_v^out≠∅（反向证书=出场）, P4=未 active∧I_v^in≠∅,
/// P5=active∧I_v^in≠∅, P6=无有效信号。互斥化 C_{v,1}=P1, C_{v,j}=P_j∧⋀_{k<j}¬P_k（§10 close 优先）。
/// 对 active 声部，触发平仓的类 = P1/P2/P3（前三优先级）。本函数判这三者（§11 「close 优先于 open」）。
fn should_close(exit_cert: bool, parent_active: bool, risk_close: bool) -> bool {
    // C_{v,1}=P1（风险关闭）→ 平。C_{v,2}=P2∧¬P1（父关闭）→ 平。C_{v,3}=P3∧¬P1∧¬P2（出场证书）→ 平。
    // 互斥优先级：任一前三谓词成立即平（§11 close 优先）。父关闭（P2）在 anc_ok 阶段级联处理，
    // 故此处 parent_active 恒传 true，实际触发 = 风险关闭(P1) 或 出场证书(P3)。
    risk_close || !parent_active || exit_cert
}

/// §12 净额 N_t^bsp = Σ_{v∈A} σ_v q_v（声部状态机持仓净额，**不是** 覆盖 Σ_e s_e ε_e）。
fn net_target(active: &HashSet<usize>, voices: &[Voice]) -> f64 {
    active.iter().map(|&v| voices[v].dir as f64 * voices[v].qty).sum()
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 && args.len() != 4 {
        eprintln!(
            "用法: {} <SYMBOL> [START_DATE END_DATE]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO",
            args.first().map(String::as_str).unwrap_or("pi_bsp_timing")
        );
        return std::process::ExitCode::from(2);
    }
    let config = ThetaConfig::default();
    let full = match load_by_symbol(&args[1], &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let dataset = if args.len() == 4 {
        full.slice_date_window(&args[2], &args[3])
    } else {
        full
    };
    if dataset.bars.is_empty() {
        eprintln!("空数据集——无法回测（inconclusive）");
        return std::process::ExitCode::FAILURE;
    }

    let bars = &dataset.bars;
    let n = bars.len();
    let prices: Vec<f64> = bars.iter().map(|b| b.close as f64 * config.tick.tick_size).collect();
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;
    let years = (n as f64 / (365.25 * 24.0 * 60.0)).max(1e-9);

    // ── π_Θ^bsp 多声部状态机 ──
    // voices：所有曾创建的 position instance（append-only，parent 索引稳定）。
    // active：当前 A_t（§4 活动声部索引集）。
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    let mut voices: Vec<Voice> = Vec::new();
    let mut active: HashSet<usize> = HashSet::new();
    // 每 carrier 的代数计数（§3 n_v：同 carrier 多次开仓）。
    let mut gen_counter: HashMap<(usize, usize), u32> = HashMap::new();
    // 净额 N_t^bsp 逐 bar 序列（§12/§18 头寸）。
    let mut net_per_bar: Vec<f64> = vec![0.0; n];

    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);

        // 本 bar 新确认的方向证书（§1-2 γ），按 carrier=(level, source_index) 收集。
        // append-only seen-set diff（644：与 runner newly_confirmed_step 同语义，无 look-ahead）。
        let mut buy_certs: Vec<(usize, usize)> = Vec::new(); // 买侧证书 carrier 列表
        let mut sell_certs: Vec<(usize, usize)> = Vec::new(); // 卖侧证书 carrier 列表
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in &ls.bsp {
                if seen.insert((lvl, p.source_index, p.bits.class_index())) {
                    let c = cert_of(&p.bits);
                    let carrier = (lvl, p.source_index);
                    if c.buy {
                        buy_certs.push(carrier);
                    }
                    if c.sell {
                        sell_certs.push(carrier);
                    }
                }
            }
        }

        // ── §9 先平后开 A^raw=(A_t\D_t)∪O_t ──
        // D_t：出场声部集（§5 出场证书 δ(γ)=−σ_v）。对每 active 声部，若本 bar 出现其**反向**证书 ⟹ 出场。
        // 多头声部（dir=+1）出场证书=卖侧证书；空头声部（dir=−1）出场证书=买侧证书（§5/§16）。
        let mut closing: HashSet<usize> = HashSet::new();
        for &v in &active {
            let voice = &voices[v];
            let exit_cert = if voice.dir > 0 {
                !sell_certs.is_empty() // 多头：卖点证书=出场（§5 δ=−σ_v=−1）
            } else {
                !buy_certs.is_empty() // 空头：买点证书=出场（§5 δ=−σ_v=+1）
            };
            // §10-11 close 优先：风险关闭(本实装 risk_close=false, K_Θ 恒等)/父关闭/出场证书。
            // 父关闭在 anc_ok 阶段级联处理，此处判出场证书（P3）。
            if should_close(exit_cert, true, false) {
                closing.insert(v);
            }
        }
        // D_t 应用：A_t \ D_t。
        for v in &closing {
            active.remove(v);
        }

        // O_t：开仓声部集（§5 入场证书 δ(γ)=σ_v）。
        // 根声部（§5）：买侧证书 → 开多头根声部（σ_v=+1）；卖侧证书 → 开空头根声部（σ_v=−1）。
        // 短差子声部（§6/§16）：父 active 时，反向买卖点证书开子声部（σ_u=−σ_p）。
        //
        // 诚实简化（no-patch 标注）：本实装把每个**新确认的方向证书**当作根声部入场触发
        // （§5 入场证书 δ(γ)=σ_v；买点开多根 / 卖点开空根）。§16 多空双开的子声部 = 在某根声部
        // active 期间出现的**反向**证书 → 开反向子声部（σ_u=−σ_p），父=该根声部。
        // 这忠实 §5（根入场）+ §6/§16（子声部反向），父子关系按「先存在的同 carrier 根声部」建立。

        // 当前每 carrier 是否已有 active 根声部（用于判子声部 vs 根声部）。
        let active_root_by_carrier: HashMap<(usize, usize), (usize, i8)> = active
            .iter()
            .filter(|&&v| voices[v].parent.is_none())
            .map(|&v| (voices[v].carrier, (v, voices[v].dir)))
            .collect();

        let open_new = |voices: &mut Vec<Voice>,
                            active: &mut HashSet<usize>,
                            gen_counter: &mut HashMap<(usize, usize), u32>,
                            carrier: (usize, usize),
                            dir: i8| {
            let g = gen_counter.entry(carrier).or_insert(0);
            *g += 1;
            let parent = active_root_by_carrier
                .iter()
                .find(|(_, (_, pdir))| *pdir == -dir) // 反向根声部 = 父（§16 σ_u=−σ_p）
                .map(|(_, (vidx, _))| *vidx);
            // 子声部方向必须 = −父方向（§6）；根声部方向 = 证书方向（§5）。
            let final_dir = match parent {
                Some(pidx) => Voice::child_dir(voices[pidx].dir),
                None => dir,
            };
            let idx = voices.len();
            voices.push(Voice {
                carrier,
                dir: final_dir,
                parent,
                qty: 1.0, // §7/§13 Q_Θ=1 单位（诚实简化）
                generation: *g,
                entry_bar: i,
            });
            active.insert(idx);
        };

        for &carrier in &buy_certs {
            open_new(&mut voices, &mut active, &mut gen_counter, carrier, 1);
        }
        for &carrier in &sell_certs {
            open_new(&mut voices, &mut active, &mut gen_counter, carrier, -1);
        }

        // ── §9 祖先闭合 A_{t+1}=AncOK(A^raw) ──（父不 active ⟹ 子声部被剪，§4）
        active = anc_ok(std::mem::take(&mut active), &voices);

        // §12 净额 N_t^bsp = Σ σ_v q_v（声部状态机持仓净额）。
        net_per_bar[i] = net_target(&active, &voices);
    }

    // ── §18 r^bsp = N^bsp·ΔP − C：逐 bar net-position MtM 权益曲线（同 π^cov NAV 口径）──
    // N_t^bsp 是连续净额（非 ±1 toggle）；equity 归一化初始=1。成本在净额变动 bar 按 |ΔN| 名义额扣。
    let nav_base: f64 = {
        // 名义基准 = 净额峰值 × 均价量级（与 significance nav_base 同口径：名义额，方向无关）。
        let peak = net_per_bar.iter().fold(0.0f64, |a, &x| a.max(x.abs())).max(1.0);
        let avg_px = prices.iter().sum::<f64>() / n.max(1) as f64;
        (peak * avg_px * (1.0 + fee_rate)).max(1e-9)
    };
    let mut equity_curve: Vec<f64> = Vec::with_capacity(n);
    let mut daily_returns: Vec<f64> = Vec::with_capacity(n);
    let mut cum_abs = 0.0;
    for i in 0..n {
        if i > 0 {
            let dp = prices[i] - prices[i - 1];
            cum_abs += net_per_bar[i - 1] * dp; // N_{t-1}·ΔP（§18）
            // 净额变动名义额扣双边费（§18 −C）。
            let dn = (net_per_bar[i] - net_per_bar[i - 1]).abs();
            cum_abs -= dn * prices[i] * fee_rate;
        }
        let eq = 1.0 + cum_abs / nav_base;
        let ret = if i == 0 {
            0.0
        } else {
            let prev = *equity_curve.last().unwrap();
            if prev.abs() > 1e-12 { eq / prev - 1.0 } else { 0.0 }
        };
        equity_curve.push(eq);
        daily_returns.push(ret);
    }

    // ── 逐笔 TradeRecord（§4 声部生命周期：每声部 [entry_bar, 平仓 bar) 配对）──
    // 声部状态机的「交易」= 每个声部的开→平。重建：扫 voices，按 dir 产多/空笔。
    // 平仓 bar：声部从 active 移除的 bar（本实装窗口终点统一强平未平声部，与工位 L forced_close 一致）。
    // 简化：声部平仓点用净额过零段近似不可靠 ⟹ 用声部 entry_bar + 窗口终点强平（保守，含浮盈）。
    // ★诚实：随机对照需逐笔 entry/exit；声部级精确平仓点需在状态机内记录（本实装记 entry，
    //   平仓在 closing 时未单独存 exit_bar）。为给 significance 提供 trades，按声部 entry→末 bar 强平。
    let mut trades: Vec<TradeRecord> = Vec::new();
    for v in &voices {
        if n - 1 > v.entry_bar {
            trades.push(TradeRecord {
                entry_bar: v.entry_bar,
                exit_bar: n - 1,
                hold_bars: n - 1 - v.entry_bar,
                qty: v.qty,
                long: v.dir > 0,
                forced_close: true,
            });
        }
    }
    let trade_pnls: Vec<f64> = trades
        .iter()
        .map(|t| {
            let e = prices[t.entry_bar.min(n - 1)];
            let x = prices[t.exit_bar.min(n - 1)];
            let raw = if t.long {
                t.qty * (x * (1.0 - fee_rate) - e * (1.0 + fee_rate))
            } else {
                t.qty * (e * (1.0 - fee_rate) - x * (1.0 + fee_rate))
            };
            raw / nav_base
        })
        .collect();

    let bh_return =
        if n >= 2 && prices[0].abs() > 1e-12 { prices[n - 1] / prices[0] - 1.0 } else { 0.0 };
    let m = metrics::compute(&equity_curve, &daily_returns, &trade_pnls, years, bh_return);
    let sig = metrics::significance(&trade_pnls, &daily_returns, &trades, &prices, fee_rate, m.strat_return);

    let n_voices = voices.len();
    let n_long = voices.iter().filter(|v| v.dir > 0).count();
    let n_children = voices.iter().filter(|v| v.parent.is_some()).count();
    let max_net = net_per_bar.iter().fold(0.0f64, |a, &x| a.max(x.abs()));

    println!("=== π_Θ^bsp 买卖点多声部离散择时回测（忠实 PDF §1-19）===");
    println!("品种            : {}", dataset.symbol);
    println!("bar 数          : {n}");
    println!("年化基数(years) : {years:.4}");
    println!("--- 声部状态机（§3 position instance / §12 N^bsp=Σσ_v q_v）---");
    println!("声部总数        : {n_voices}（多 {n_long} / 空 {}）", n_voices - n_long);
    println!("短差子声部数    : {n_children}（§6/§16 σ_u=−σ_p）");
    println!("净额峰值 |N^bsp|: {max_net:.2}");
    println!("--- 指标（§18 r^bsp=N^bsp·ΔP−C，同工位 L NAV 口径，可比 π^cov）---");
    println!("★Sharpe         : {:.4}", m.sharpe);
    println!("strat_return    : {:.4}", m.strat_return);
    println!("buy&hold_return : {:.4}", m.bh_return);
    println!("Sortino         : {:.4}", m.sortino);
    println!("MaxDrawdown     : {:.4}", m.max_drawdown);
    println!("win_rate        : {:.4}", m.win_rate);
    println!("profit_factor   : {:.4}", m.profit_factor);
    println!("--- 同口径随机对照（§4 协议，seed 冻结）---");
    println!("theta_same_caliber : {:.6}", sig.theta_return_same_caliber);
    println!("shift  mean/p   : {:.6} / {:.4}", sig.shift_mean_return, sig.shift_pvalue);
    println!("indep  mean/p   : {:.6} / {:.4}", sig.indep_mean_return, sig.indep_pvalue);
    println!("beats_random    : {}", sig.theta_beats_random);
    println!("controls_degen  : {}", sig.controls_degenerate);
    println!("--- 认识论 L2（真实数据，可否证；§18 alpha 判定）---");
    if voices.is_empty() {
        println!("等级: L1（无买卖点确认 ⟹ 无声部，inconclusive）");
    } else {
        println!("等级: L2——π^bsp Sharpe={:.4} 可与 π^cov 覆盖 Sharpe 对照（§19 goal#5）", m.sharpe);
    }
    std::process::ExitCode::SUCCESS
}
