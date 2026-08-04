//! ponytail: feature-gated——引用 theta_v0::backtest（#[cfg(any(test,feature="backtest_bin"))] 门控），
//! 集成测试构建须 --features backtest_bin 才见 backtest 模块，否则 E0433。
#![cfg(feature = "backtest_bin")]

//! 奇偶交替成因诊断（L2，真实数据；分析工位，不改核心逻辑，纯诊断输出）。
//!
//! 分离 team-lead 四假设 + ChatGPT parity-phase-consult 机制 A/B/C 识别：
//! - 机制A（几何反相真结构）：级别k买点≈级别k+1某段中点，方向反相是递归几何必然。
//! - 机制B（出场口径产物）：奇偶交替只是"持有到下一反向信号"这个口径的伪影。
//! - 机制C（regime/beta漂移）：奇偶交替是BTC单边趋势的方向漂移，非结构信息。
//!
//! **ChatGPT 12/12 不可识别性**：仅均值符号不能区分 A/B/C——必须反事实。本诊断实装：
//! 1. **多出场口径反事实**（`oddeven_causation_counterfactual`）：同一批信号，3 种出场口径。只在
//!    口径4(下一反向)交替=机制B；口径5/6也交替=机制A/C（几何/入场相位，非出场口径产物）。
//! 2. **随机置换方向标签**（ChatGPT 方法1，最干净剥离beta）：保持入场/出场/持有期/成本+买卖数量，
//!    只 shuffle δ。S_obs(真标签 Σ|μ̂|) vs S_perm 分布，显著大=方向标签携带结构信息超过漂移。
//!
//! `oddeven_holding_window_diagnosis`（原诊断）：per-class 持有期 + 持有窗净涨跌方向（假设2/3/4）。
//!
//! **恒等式基础**：actual_pnl = δ·(P[τout]−P[τin])−Ce ⟹ sign(μ̂)=sign(δ×持有窗净涨跌)。

use newchan_rust::theta_v0::backtest::data::{self, Dataset};
use newchan_rust::theta_v0::backtest::econ_positive::decompose_capturable_spread;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::classifier::{Classification, LevelState};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::strategy::interp::assemble_gamma_with_tower;
use newchan_rust::theta_v0::strategy::voice::VoiceSide;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::BTreeMap;

// ════════════════════════════════════════════════════════════════════════════
//  原诊断：per-class 持有期 + 持有窗净涨跌方向（假设2/3/4 恒等式）
// ════════════════════════════════════════════════════════════════════════════

/// `#[ignore]`：需 BTC 全量 + O(n²) 逐 bar 重分类，`--release`。ECON_L2_MAX_BARS 覆盖窗口。
#[test]
#[ignore = "L2 奇偶交替持有窗诊断；需 BTC 全量；--release --ignored"]
fn oddeven_holding_window_diagnosis() {
    let config = ThetaConfig::default();
    let ds_full = data::load_by_symbol("BTC", &config).expect("BTC 加载（DATA BLOCKER 不伪造）");
    let n_full = ds_full.bars.len();
    let max_bars: usize = std::env::var("ECON_L2_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let ds = if n_full > max_bars {
        ds_full.slice_bar_range(n_full - max_bars, n_full)
    } else {
        ds_full
    };
    let n_bars = ds.bars.len();
    let tick = config.tick.tick_size;

    let (decomps, _agg) = decompose_capturable_spread(&ds, &config);

    // per-class 累积：(n, Σactual_pnl, Σ持有期bar, Σ净涨跌(无向), Σδ·净涨跌, n_hold_up)。
    type Bucket = (usize, f64, u64, f64, f64, usize);
    let mut buckets: BTreeMap<(u32, i8), Bucket> = BTreeMap::new();
    let px = |i: usize| ds.bars[i].close as f64 * tick;
    for d in &decomps {
        let hold_bars = (d.exit_bar - d.entry_bar) as u64;
        let net_move = px(d.exit_bar) - px(d.entry_bar);
        let e = buckets
            .entry((d.level, d.delta))
            .or_insert((0, 0.0, 0, 0.0, 0.0, 0));
        e.0 += 1;
        e.1 += d.actual_pnl;
        e.2 += hold_bars;
        e.3 += net_move;
        e.4 += d.actual_spread;
        if net_move > 0.0 {
            e.5 += 1;
        }
    }
    let market_net = px(n_bars - 1) - px(0);
    eprintln!(
        "\n===== 持有窗诊断（BTC bars={n_bars}，全窗净涨跌={market_net:+.1}={}）=====",
        if market_net > 0.0 { "涨" } else { "跌" }
    );
    eprintln!(
        "{:>2} {:>3} {:>6} {:>12} {:>12} {:>14} {:>14}",
        "L", "δ", "n", "μ̂", "avg持有bar", "avg净涨跌(无向)", "avg(δ·净涨跌)"
    );
    for ((lvl, dlt), (n, spnl, shold, snet, sspread, _)) in &buckets {
        let nn = *n as f64;
        eprintln!(
            "{:>2} {:>+3} {:>6} {:>12.2} {:>12.0} {:>14.2} {:>14.2}",
            lvl,
            dlt,
            n,
            spnl / nn,
            *shold as f64 / nn,
            snet / nn,
            sspread / nn
        );
    }

    eprintln!("\n--- 假设4：μ̂符号 vs δ·sign(avg净涨跌) 一致性 ---");
    let mut all_consistent = true;
    for ((lvl, dlt), (n, spnl, _, snet, _, _)) in &buckets {
        let consistent = ((spnl / *n as f64) > 0.0) == (((*dlt as f64) * (snet / *n as f64)) > 0.0);
        if !consistent {
            all_consistent = false;
        }
        eprintln!(
            "  L{lvl} δ{dlt:+}: μ̂={:+.1}, δ·avg净涨跌={:+.2} → {}",
            spnl / *n as f64,
            (*dlt as f64) * (snet / *n as f64),
            if consistent {
                "一致"
            } else {
                "★不一致(成本翻转)"
            }
        );
    }
    eprintln!("  全部一致={all_consistent}");

    eprintln!("\n--- 假设2/3：per-level 平均持有期（递增=级别持有窗尺度阶梯）---");
    let mut level_hold: BTreeMap<u32, (u64, usize)> = BTreeMap::new();
    for ((lvl, _), (n, _, shold, _, _, _)) in &buckets {
        let e = level_hold.entry(*lvl).or_insert((0, 0));
        e.0 += shold;
        e.1 += n;
    }
    let mut prev: Option<f64> = None;
    let mut monotone = true;
    for (lvl, (shold, n)) in &level_hold {
        let avg = *shold as f64 / *n as f64;
        let trend = match prev {
            Some(p) if avg < p => {
                monotone = false;
                "↓非单调"
            }
            Some(_) => "↑",
            None => "基准",
        };
        eprintln!("  L{lvl}: avg持有={avg:.0} bar n={n} {trend}");
        prev = Some(avg);
    }
    eprintln!("  持有期随级别单调递增={monotone}");
    assert!(!decomps.is_empty(), "decompose 非空");
}

// ════════════════════════════════════════════════════════════════════════════
//  反事实诊断：多出场口径（A vs B）+ 随机置换方向标签（剥离 beta，C）
// ════════════════════════════════════════════════════════════════════════════

/// 一条信号：(entry_bar=确认bar τin, δ, lambda_pivot_bar=入场pivot端点, level)。
type Signal = (usize, i8, usize, u32);

fn bsp_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// 信号收集（复用 decompose_capturable_spread 收集循环，一次跑=14min 重头）。收集顺序=因果时间序。
fn collect_signals(ds: &Dataset, config: &ThetaConfig) -> Vec<Signal> {
    let bars = &ds.bars;
    let n = bars.len();
    let mut incr = IncrementalClassifier::new(bars, config);
    let mut seen: std::collections::HashSet<(usize, usize, u8)> = std::collections::HashSet::new();
    let mut signals: Vec<Signal> = Vec::new();
    for i in 0..n {
        let bar = &bars[i];
        if bar.untradable || bar.close <= 0 {
            continue;
        }
        let (cls_i, tower_i) = incr.classify_at(i);
        for (lvl, ls) in cls_i.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                if !seen.insert((lvl, p.source_index, bsp_disc(&p.bits))) {
                    continue;
                }
                let pivot_bar = p.source_index;
                let single = Classification {
                    levels: cls_i
                        .levels
                        .iter()
                        .enumerate()
                        .map(|(l2, _)| LevelState {
                            moves: Vec::new(),
                            centers: Vec::new().into(),
                            cp_ownership: Vec::new().into(),
                            bsp: (if l2 == lvl {
                                vec![p.clone()]
                            } else {
                                Vec::new()
                            })
                            .into(),
                            pan_div: Vec::new().into(),
                            first_class_grades: Vec::new().into(),
                            level_projection: None,
                        })
                        .collect(),
                };
                for c in &assemble_gamma_with_tower(&single, &tower_i) {
                    let delta: i8 = match c.dir {
                        VoiceSide::Long => 1,
                        VoiceSide::Short => -1,
                        VoiceSide::Flat => continue,
                    };
                    signals.push((i, delta, pivot_bar, lvl as u32));
                }
            }
        }
    }
    signals
}

fn ce_unit(px_in: f64, px_out: f64, fee: f64) -> f64 {
    (px_in + px_out) * fee
}

/// 口径4（基准，现口径）：持有到下一反向新确认信号，价用确认bar close。
fn pnl_next_reverse(
    sig: &[Signal],
    px: &dyn Fn(usize) -> f64,
    fee: f64,
) -> BTreeMap<(u32, i8), (usize, f64)> {
    let mut out: BTreeMap<(u32, i8), (usize, f64)> = BTreeMap::new();
    for (idx, &(entry_bar, delta, _, level)) in sig.iter().enumerate() {
        let opp = -delta;
        let Some(&(exit_bar, _, _, _)) = sig[idx + 1..]
            .iter()
            .find(|(eb, d, _, _)| *eb > entry_bar && *d == opp)
        else {
            continue;
        };
        if exit_bar <= entry_bar {
            continue;
        }
        let (pin, pout) = (px(entry_bar), px(exit_bar));
        if pin <= 0.0 || pout <= 0.0 {
            continue;
        }
        let e = out.entry((level, delta)).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += delta as f64 * (pout - pin) - ce_unit(pin, pout, fee);
    }
    out
}

/// 口径6（固定持有期，出场口径无关）：入场后固定持有 hold bar 出场。纯测"入场时点后市场走向×δ"。
/// 若这个也奇偶交替 ⟹ 交替来自入场相位（滞后+级别），非出场口径（排除机制B）。
fn pnl_fixed_hold(
    sig: &[Signal],
    px: &dyn Fn(usize) -> f64,
    n_bars: usize,
    hold: usize,
    fee: f64,
) -> BTreeMap<(u32, i8), (usize, f64)> {
    let mut out: BTreeMap<(u32, i8), (usize, f64)> = BTreeMap::new();
    for &(entry_bar, delta, _, level) in sig {
        let exit_bar = entry_bar + hold;
        if exit_bar >= n_bars {
            continue;
        }
        let (pin, pout) = (px(entry_bar), px(exit_bar));
        if pin <= 0.0 || pout <= 0.0 {
            continue;
        }
        let e = out.entry((level, delta)).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += delta as f64 * (pout - pin) - ce_unit(pin, pout, fee);
    }
    out
}

/// 口径5（终点pivot，无确认滞后）：入场价=λ pivot端点，出场价=配对反向信号ρ pivot端点（=econ a_b）。
/// 测"若确认无滞后（直接用pivot价）"是否还交替。
fn pnl_pivot(
    sig: &[Signal],
    px: &dyn Fn(usize) -> f64,
    fee: f64,
) -> BTreeMap<(u32, i8), (usize, f64)> {
    let mut out: BTreeMap<(u32, i8), (usize, f64)> = BTreeMap::new();
    for (idx, &(entry_bar, delta, lambda, level)) in sig.iter().enumerate() {
        let opp = -delta;
        let Some(&(_, _, rho, _)) = sig[idx + 1..]
            .iter()
            .find(|(eb, d, _, _)| *eb > entry_bar && *d == opp)
        else {
            continue;
        };
        let (pin, pout) = (px(lambda), px(rho));
        if pin <= 0.0 || pout <= 0.0 {
            continue;
        }
        let e = out.entry((level, delta)).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += delta as f64 * (pout - pin) - ce_unit(pin, pout, fee);
    }
    out
}

/// 奇偶交替匹配：sign(μ̂(ℓ,δ)) == sign(δ·(−1)^ℓ) 的类占比（ChatGPT 严格含义）。
fn parity_match_ratio(b: &BTreeMap<(u32, i8), (usize, f64)>) -> (usize, usize) {
    let (mut matched, mut total) = (0, 0);
    for (&(lvl, dlt), &(n, spnl)) in b {
        if n == 0 {
            continue;
        }
        total += 1;
        let expected = (dlt as f64) * if lvl % 2 == 0 { 1.0 } else { -1.0 };
        if ((spnl / n as f64) > 0.0) == (expected > 0.0) {
            matched += 1;
        }
    }
    (matched, total)
}

fn dump_buckets(name: &str, b: &BTreeMap<(u32, i8), (usize, f64)>) {
    let (m, t) = parity_match_ratio(b);
    eprintln!("\n[{name}] 奇偶匹配 {m}/{t}（sign μ̂ == sign(δ·(−1)^ℓ)）");
    for (&(lvl, dlt), &(n, spnl)) in b {
        if n == 0 {
            continue;
        }
        eprintln!(
            "  L{lvl}({}) δ{dlt:+} n={n:>5} μ̂={:+.2}",
            if lvl % 2 == 0 { "偶" } else { "奇" },
            spnl / n as f64
        );
    }
}

#[test]
#[ignore = "L2 奇偶交替成因反事实（多口径+随机置换）；需 BTC 全量；--release --ignored"]
fn oddeven_causation_counterfactual() {
    let config = ThetaConfig::default();
    let ds_full = data::load_by_symbol("BTC", &config).expect("BTC 加载（DATA BLOCKER 不伪造）");
    let n_full = ds_full.bars.len();
    let max_bars: usize = std::env::var("ECON_L2_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let ds = if n_full > max_bars {
        ds_full.slice_bar_range(n_full - max_bars, n_full)
    } else {
        ds_full
    };
    let n_bars = ds.bars.len();
    let tick = config.tick.tick_size;
    let fee_rate =
        (config.exec.commission_bps + config.exec.slippage_bps + config.exec.tax_bps) / 10_000.0;

    let sig = collect_signals(&ds, &config);
    // px 借用 ds（非 move，ds 活到函数结束；collect 与 px 都只借用，无冲突）。
    let px = |i: usize| {
        ds.bars
            .get(i)
            .map(|b| b.close as f64 * tick)
            .filter(|&p| p > 0.0)
            .unwrap_or(0.0)
    };
    eprintln!(
        "\n===== 奇偶交替成因反事实（BTC bars={n_bars}，信号={}）=====",
        sig.len()
    );
    eprintln!("奇偶交替严格含义（ChatGPT）：sign μ̂(ℓ,δ)=sign(δ·(−1)^ℓ)。");

    // ── 反事实1：多出场口径（识别 A vs B）──
    let b_next = pnl_next_reverse(&sig, &px, fee_rate);
    let b_pivot = pnl_pivot(&sig, &px, fee_rate);
    let b_h500 = pnl_fixed_hold(&sig, &px, n_bars, 500, fee_rate);
    let b_h1000 = pnl_fixed_hold(&sig, &px, n_bars, 1000, fee_rate);
    let b_h2000 = pnl_fixed_hold(&sig, &px, n_bars, 2000, fee_rate);
    dump_buckets("口径4 下一反向(现口径)", &b_next);
    dump_buckets("口径5 终点pivot(无确认滞后)", &b_pivot);
    dump_buckets("口径6 固定持有500bar", &b_h500);
    dump_buckets("口径6 固定持有1000bar", &b_h1000);
    dump_buckets("口径6 固定持有2000bar", &b_h2000);

    eprintln!("\n--- 机制识别（ChatGPT A vs B）---");
    eprintln!("  只口径4交替 ⟹ 机制B（出场口径产物）；口径5/6也交替 ⟹ 机制A/C（几何/入场相位，非出场口径）");
    let (m4, t4) = parity_match_ratio(&b_next);
    let (m5, t5) = parity_match_ratio(&b_pivot);
    let (m6, t6) = parity_match_ratio(&b_h1000);
    eprintln!("  口径4={m4}/{t4}，口径5(pivot)={m5}/{t5}，口径6(H1000)={m6}/{t6}");

    // ── 反事实2：随机置换方向标签（ChatGPT 方法1，剥离 beta）──
    // 保持每条信号 entry/出场/持有期/成本 + 买卖总数，只 shuffle δ。S=Σ_class|μ̂|。
    // 真 S 显著大于置换分布 ⟹ 方向标签携结构信息（机制A），非纯 beta 漂移（机制C，漂移与δ标签无关⟹置换不改S）。
    let s_stat = |b: &BTreeMap<(u32, i8), (usize, f64)>| -> f64 {
        b.values()
            .filter(|(n, _)| *n > 0)
            .map(|(n, s)| (s / *n as f64).abs())
            .sum()
    };
    let s_obs = s_stat(&b_next);
    let deltas: Vec<i8> = sig.iter().map(|s| s.1).collect();
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut next = |seed: &mut u64, m: usize| -> usize {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 33) as usize) % m.max(1)
    };
    let n_perm = 200;
    let mut s_perm = Vec::with_capacity(n_perm);
    let mut perm_sig = sig.clone();
    for _ in 0..n_perm {
        let mut d = deltas.clone();
        for k in (1..d.len()).rev() {
            let j = next(&mut seed, k + 1);
            d.swap(k, j);
        }
        for (s, &nd) in perm_sig.iter_mut().zip(d.iter()) {
            s.1 = nd;
        }
        s_perm.push(s_stat(&pnl_next_reverse(&perm_sig, &px, fee_rate)));
    }
    let n_ge = s_perm.iter().filter(|&&s| s >= s_obs).count();
    let perm_p = (n_ge as f64 + 1.0) / (n_perm as f64 + 1.0);
    let perm_mean = s_perm.iter().sum::<f64>() / n_perm as f64;
    eprintln!("\n--- 随机置换方向标签剥离beta（ChatGPT 方法1，n_perm={n_perm} 固定种子）---");
    eprintln!(
        "  S_obs(真标签)={s_obs:.2}, S_perm均值={perm_mean:.2}, perm_p(#perm≥obs)={perm_p:.4}"
    );
    eprintln!("  perm_p<0.05 ⟹ 方向标签携结构信息显著超漂移（机制A）；perm_p≈1 ⟹ 奇偶交替=beta漂移伪影（机制C）");
    eprintln!("\n（诊断结束，纯 eprintln 无落盘；结论由分析工位综合）");
    assert!(!sig.is_empty(), "信号非空");
}

/// L1 自检（合成，秒级）：三口径 + 置换统计算术正确性（不需 BTC，验管线；formalization-validity-domain L1）。
#[test]
fn counterfactual_helpers_l1() {
    // 合成 3 信号：买@bar0(pivot0) 卖@bar2(pivot2) 买@bar4(pivot4)，价序列见下。
    let sig: Vec<Signal> = vec![(0, 1, 0, 0), (2, -1, 2, 0), (4, 1, 4, 0)];
    let prices = [10.0, 11.0, 14.0, 13.0, 12.0, 12.5, 13.0];
    let px = |i: usize| prices.get(i).copied().unwrap_or(0.0);

    // 口径4：买@0→卖@2 δ+1·(14−10)=+4；卖@2→买@4 δ−1·(12−14)=+2。
    let b = pnl_next_reverse(&sig, &px, 0.0);
    assert_eq!(b.get(&(0, 1)).unwrap().0, 1);
    assert!((b.get(&(0, 1)).unwrap().1 - 4.0).abs() < 1e-9, "买 pnl=+4");
    assert!((b.get(&(0, -1)).unwrap().1 - 2.0).abs() < 1e-9, "卖 pnl=+2");

    // 口径6 固定持有2bar：买@0→bar2(14)=+4，买@4→bar6(13)=+1 ⟹ 买Σ=5（2个）；卖@2→bar4(12) δ−1·(−2)=+2。
    let h = pnl_fixed_hold(&sig, &px, prices.len(), 2, 0.0);
    assert_eq!(h.get(&(0, 1)).unwrap().0, 2);
    assert!(
        (h.get(&(0, 1)).unwrap().1 - 5.0).abs() < 1e-9,
        "买固定持有 Σ=5"
    );

    // 口径5 pivot：买@pivot0(10)→配对卖 rho=pivot2(14) δ+1·4=+4。
    let p = pnl_pivot(&sig, &px, 0.0);
    assert!(
        (p.get(&(0, 1)).unwrap().1 - 4.0).abs() < 1e-9,
        "买 pivot pnl=+4"
    );

    // parity：L0(偶)买(δ+1) μ̂>0 ⟹ 匹配 sign(δ·(−1)^0)=+。
    let (m, t) = parity_match_ratio(&b);
    assert!(t == 2 && m >= 1, "L0 买正匹配奇偶模式");
}
