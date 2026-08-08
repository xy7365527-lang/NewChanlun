//! # `p907_cost_gate_level` —— 成本门咬在哪一级（票 #907 事实底座，只读探针）
//!
//! ## 本探针回答什么
//!
//! `035-第35课.md:30`【正文】：「级别越小，平均的买卖点间波幅也越小，因此，那些太小的级别，
//! 不足以让交易成本、交易误差等相对买卖点间波幅足够小，这样的操作，从长期的角度看，是没有
//! 意义的。」#817 N-2 裁定把「下钻停止判据 = 成本门 ∧ 塔底」写成「判据 + 参数」，**参数就是
//! 本探针产的数**。
//!
//! 产三组读数（**只测不裁**——阈值归 #817）：
//! 1. 逐级「相邻买卖点间波幅」分布（p10/p25/p50/p75/p90 + 均值 + 样本数），单位 **bp**（相对幅度）。
//! 2. 成本侧：交易成本走 venue datum（本探针只**打印**档位，不自己估）；交易误差用
//!    `entry_delay_bars` 时延的**实测**价位差代理。
//! 3. 比值 `波幅 ÷ (成本 + 误差)` 逐级列表。
//!
//! ## 口径声明（**不同口径结论会翻，故逐条钉死**）
//!
//! ### 买卖点来源
//! 生产因果塔 `IncrementalClassifier::classify_at(i)` 的 `classification.levels[lvl].bsp`，
//! 六 bit（buy1/2/3 + sell1/2/3）任一为真即计一个买卖点。去重键 `(lvl, source_index)`
//! （与 `p105_cert_level_spectrum` / `p100_cert_bsp_recon` 同口径）。**塔级 k == classifier lvl k**
//! （p105 已考据，不做换算）。seen-set append-only diff 与 runner `newly_confirmed_step` 同语义
//! ⟹ 644 meta-rule：探针走生产路径，无影子分叉。
//!
//! ### 两个时间坐标，**必须分开**（本探针的关键分叉）
//! 每个买卖点有两个 bar 坐标：
//! - `source_index` = 该点的**结构坐标**（L0 原始 K 序，是过去的某根 K）；
//! - `confirm_bar`  = 该点**首次被塔确认**的 bar（因果可用的最早时刻，= 递增扫描的 i）。
//!
//! 因为塔要等结构长出来才能确认，`confirm_bar ≥ source_index`，二者可差很远。故两个幅度口径：
//! - **口径 EXEC（可执行，本票主口径）**：按 `confirm_bar` 排序，幅度 =
//!   `|close(confirm_{k+1}) − close(confirm_k)| / close(confirm_k)`。这是「你在看到买点时下手、
//!   在看到下一个买卖点时了结」实际能碰到的价差 —— **成本门问的就是这个**（成本按名义额收在
//!   你真实成交的那两个价上）。
//! - **口径 STRUCT（结构，参照用）**：按 `source_index` 排序，幅度取两个结构坐标的收盘价差。
//!   这是**上界**，不可执行（`source_index` 那根 K 在确认时已成过去，下不了单）。原文
//!   「买卖点间波幅」字面更像这个，故并列报出，供裁定方选。
//!
//! ### 幅度定义
//! **端点收盘价差的绝对值，除以起点收盘价，单位 bp**（1 bp = 0.01%）。
//! - 取 close 而非区间高低差：本引擎**按 bar close 市价撮合**（`venue_fee_provenance.md:50`），
//!   成本也按名义额（bp/side）收 ⟹ 分子分母同在「成交价」这一坐标上才可比。
//! - 取相对值（bp）而非绝对 $：费率是 bp/side ⟹ 比值才无量纲；且 BTC 全史价格跨两个数量级，
//!   绝对 $ 幅度会被后期高价段整体拉高（在案前例：跨品种原始 $ 拼接 = 尺度伪影）。
//! - `Tick` 是整数量化价，比值与 tick_size 无关 ⟹ 不需要反量化。
//!
//! ### 「同级相邻」定义
//! 同一 lvl 内、按上述时间坐标升序的**紧邻两个去重身份**。**不分买卖方向、不配对成一笔交易**
//! ——原文说的是「买卖点间波幅」，不是「一笔完整交易的盈亏」。方向配对属另一件事（会引入
//! 择时假设），本探针不做。
//!
//! ### 交易误差的可观测代理（**是代理，不是真实延迟实测**）
//! 本仓 `ExecConfig::slippage_bps = 2.0` 是**未标定常数**（`config.rs:259-262` 与
//! `venue_fee_provenance.md:46` 双处明文），**不能当实测用**。可观测的那一项是
//! `entry_delay_bars = 1`（`config.rs:252`）——「信号确认后延迟成交的基础 K 根数」，
//! 正是原文「你看见买点到你实际操作完成的时间差」的建模。故本探针实测
//! `|close(confirm_bar + d) − close(confirm_bar)| / close(confirm_bar)`（d = entry_delay_bars）
//! 的逐级分布 = **模型内 1 根 K 时延所致价位差**。
//!
//! **它是真实交易误差的下界**：不含网络延迟、盘口价差、排队与冲击。真实值本仓查不到
//! （无逐笔延迟/滑点记录，见报告）。
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态、不改任何生产行为；输出 `P907_*` 报表行到 stdout。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p907_cost_gate_level
//! # 可选：P907_MAX_BARS=500000 限长（跨窗对照用；不设 = 全史）
//! # 可选：P907_SYMBOL=BTC（默认 BTC）
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use std::collections::{BTreeMap, HashMap, HashSet};

/// 与 runner.rs `bsp_bits_disc` 同口径——6 类 bit 打包（seen-set 身份判别用）。
fn bsp_bits_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

fn has_any_bit(b: &BspBits) -> bool {
    b.buy1 || b.buy2 || b.buy3 || b.sell1 || b.sell2 || b.sell3
}

/// 线性插值分位数（sorted 非空）。
fn pct(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
    }
}

/// 一组幅度样本的摘要行。`bars` 与 `vals` 同索引（bars = 该样本的起点 bar，聚簇用）。
struct Summary {
    n: usize,
    mean: f64,
    p10: f64,
    p25: f64,
    p50: f64,
    p75: f64,
    p90: f64,
    /// 最大月度簇占比（样本按起点 bar 的日历月分桶，最大桶 / n）。
    max_month_share: f64,
    /// 最大 4K-bar 窗簇占比（滑窗宽 4000 bar，最密窗内样本数 / n）。
    max_4k_share: f64,
    /// 覆盖的日历月数。
    n_months: usize,
}

fn summarize(vals: &[f64], bars: &[usize], dates: &[String]) -> Summary {
    let mut s: Vec<f64> = vals.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    let mean = if n == 0 {
        f64::NAN
    } else {
        s.iter().sum::<f64>() / n as f64
    };
    // 月度簇
    let mut month: BTreeMap<String, usize> = BTreeMap::new();
    for &b in bars {
        let key = dates
            .get(b)
            .map(|d| d.chars().take(7).collect::<String>())
            .unwrap_or_else(|| "????-??".to_string());
        *month.entry(key).or_default() += 1;
    }
    let max_month = month.values().copied().max().unwrap_or(0);
    // 4K-bar 最密窗（bars 已按时间升序 ⟹ 双指针）
    let mut sorted_bars = bars.to_vec();
    sorted_bars.sort_unstable();
    let mut best = 0usize;
    let mut lo = 0usize;
    for hi in 0..sorted_bars.len() {
        while sorted_bars[hi].saturating_sub(sorted_bars[lo]) >= 4000 {
            lo += 1;
        }
        best = best.max(hi - lo + 1);
    }
    Summary {
        n,
        mean,
        p10: pct(&s, 0.10),
        p25: pct(&s, 0.25),
        p50: pct(&s, 0.50),
        p75: pct(&s, 0.75),
        p90: pct(&s, 0.90),
        max_month_share: if n == 0 {
            f64::NAN
        } else {
            max_month as f64 / n as f64
        },
        max_4k_share: if n == 0 {
            f64::NAN
        } else {
            best as f64 / n as f64
        },
        n_months: month.len(),
    }
}

fn print_summary(tag: &str, lvl: usize, s: &Summary) {
    println!(
        "P907_{tag} lvl={lvl} n={} mean_bp={:.1} p10_bp={:.1} p25_bp={:.1} p50_bp={:.1} \
         p75_bp={:.1} p90_bp={:.1} max_month_share={:.3} max_4kbar_share={:.4} n_months={}",
        s.n,
        s.mean,
        s.p10,
        s.p25,
        s.p50,
        s.p75,
        s.p90,
        s.max_month_share,
        s.max_4k_share,
        s.n_months
    );
}

fn main() -> std::process::ExitCode {
    let symbol = std::env::var("P907_SYMBOL").unwrap_or_else(|_| "BTC".to_string());
    let config = ThetaConfig::default();
    let dataset = match load_by_symbol(&symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let max_bars: usize = std::env::var("P907_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let n_full = dataset.bars.len();
    let n = n_full.min(max_bars);
    let bars = &dataset.bars[..n];
    let dates = &dataset.dates;
    let delay = config.exec.entry_delay_bars as usize;

    println!(
        "P907_INPUT symbol={symbol} bars_used={n} bars_total={n_full} \
         date_first={} date_last={} entry_delay_bars={delay}",
        dates.first().map(|s| s.as_str()).unwrap_or("?"),
        dates.get(n - 1).map(|s| s.as_str()).unwrap_or("?")
    );
    // 成本侧：只打印在册档位与未标定项，**不在此处替裁定方合成阈值**。
    println!(
        "P907_COST_CFG commission_bps_uncal={} slippage_bps_UNCALIBRATED={} tax_bps={} \
         fee_schedule={}",
        config.exec.commission_bps,
        config.exec.slippage_bps,
        config.exec.tax_bps,
        if config.exec.fee_schedule.is_some() {
            "Some"
        } else {
            "None(未注入,见报告)"
        }
    );

    // ── 生产因果塔扫描：逐级收集去重买卖点身份 ───────────────────────────
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    // (lvl, source_index) → (首次确认 bar, 该身份的买/卖 bit 并集)
    let mut first_confirm: HashMap<(usize, usize), (usize, bool, bool)> = HashMap::new();
    let mut events_total = 0usize;
    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                if seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits)))
                    && has_any_bit(&p.bits)
                {
                    events_total += 1;
                    let b = &p.bits;
                    let buy = b.buy1 || b.buy2 || b.buy3;
                    let sell = b.sell1 || b.sell2 || b.sell3;
                    first_confirm
                        .entry((lvl, p.source_index))
                        .and_modify(|(_cb, bu, se)| {
                            *bu |= buy;
                            *se |= sell;
                        })
                        .or_insert((i, buy, sell));
                }
            }
        }
        if i % 500_000 == 0 && i > 0 {
            eprintln!("P907_PROGRESS bar={i}/{n} ids={}", first_confirm.len());
        }
    }

    // 逐级重组：lvl → [(source_index, confirm_bar, buy, sell)]
    let mut by_lvl: BTreeMap<usize, Vec<(usize, usize, bool, bool)>> = BTreeMap::new();
    for (&(lvl, src), &(cb, bu, se)) in first_confirm.iter() {
        by_lvl.entry(lvl).or_default().push((src, cb, bu, se));
    }
    let max_lvl = by_lvl.keys().copied().max().unwrap_or(0);
    println!(
        "P907_BSP ids_total={} events_total={events_total} max_lvl={max_lvl}",
        first_confirm.len()
    );

    let close = |b: usize| bars[b].close as f64;

    for lvl in 0..=max_lvl {
        let Some(pts) = by_lvl.get(&lvl) else {
            println!("P907_LVL_EMPTY lvl={lvl} n_bsp=0");
            continue;
        };
        println!("P907_COUNT lvl={lvl} n_bsp={}", pts.len());

        // ── 口径 EXEC：按 confirm_bar 升序 ────────────────────────────
        let mut ex: Vec<(usize, usize, bool, bool)> = pts.clone();
        ex.sort_by_key(|&(src, cb, _, _)| (cb, src));
        let mut ex_vals = Vec::new();
        let mut ex_bars = Vec::new();
        for w in ex.windows(2) {
            let (b0, b1) = (w[0].1, w[1].1);
            if b0 == b1 {
                continue; // 同 bar 同时确认 ⟹ 无价差可捕（幅度恒 0，不入样本，避免用同刻事件把分布往下拉）
            }
            let c0 = close(b0);
            if c0 <= 0.0 {
                continue;
            }
            ex_vals.push(((close(b1) - c0).abs() / c0) * 10_000.0);
            ex_bars.push(b0);
        }
        if !ex_vals.is_empty() {
            print_summary("AMP_EXEC", lvl, &summarize(&ex_vals, &ex_bars, dates));
        } else {
            println!("P907_AMP_EXEC lvl={lvl} n=0 (无相邻对)");
        }

        // ── 诊断：相邻对的**时间间隔**（bar 数），单位仍走 summarize 的 *_bp 列名 ──
        // 为什么必须报：若高级别的相邻对间隔反而**更短**，说明该级别的买卖点是**成簇爆发**
        // （同一转折点在该级别生成多个 source 身份、near-同刻确认），那么「相邻买卖点间波幅」
        // 测到的就不是该级别的特征摆动，而是簇内的near-零价差 ⟹ 幅度随级别下降是**伪影**。
        // 这一列是判断上面 AMP_EXEC 能不能信的**前置条件**，不是可选装饰。
        let mut gap_vals: Vec<f64> = Vec::new();
        let mut gap_bars: Vec<usize> = Vec::new();
        for w in ex.windows(2) {
            let (b0, b1) = (w[0].1, w[1].1);
            if b0 == b1 {
                continue;
            }
            gap_vals.push((b1 - b0) as f64);
            gap_bars.push(b0);
        }
        if !gap_vals.is_empty() {
            print_summary("GAPBARS_EXEC", lvl, &summarize(&gap_vals, &gap_bars, dates));
        }
        // 同刻确认对数（被 AMP_EXEC 排除的那些）——簇的直接计数。
        let same_bar = ex.windows(2).filter(|w| w[0].1 == w[1].1).count();
        println!(
            "P907_SAMEBAR lvl={lvl} same_confirm_bar_pairs={same_bar} of_adjacent={}",
            ex.len().saturating_sub(1)
        );

        // ── 口径 ROUNDTRIP：入场信号 → **同级最近的反向信号**（一趟完整往返）──
        // 为什么加这一档：成本是**按往返两次成交**收的（taker × 2），而 AMP_EXEC 的「相邻任意
        // 买卖点」被上面 GAPBARS 证明在高级别是簇内配对 ⟹ 测不到该级别的特征摆动。本档按
        // 「看到买点进、看到同级第一个卖点出」配对（反向亦然），是成本门语义上最接近的可执行口径。
        // 仍是**描述性读数**：不设阈值、不选级别（裁定归 #817）。
        let mut rt_vals: Vec<f64> = Vec::new();
        let mut rt_bars: Vec<usize> = Vec::new();
        let mut rt_gap: Vec<f64> = Vec::new();
        for (k, &(_s0, b0, bu0, se0)) in ex.iter().enumerate() {
            for want_sell in [true, false] {
                // want_sell=true ⟹ 从买点出发找卖点；false ⟹ 从卖点出发找买点
                let start_ok = if want_sell { bu0 } else { se0 };
                if !start_ok {
                    continue;
                }
                let hit = ex[k + 1..]
                    .iter()
                    .find(|&&(_s1, b1, bu1, se1)| b1 > b0 && if want_sell { se1 } else { bu1 });
                if let Some(&(_s1, b1, _, _)) = hit {
                    let c0 = close(b0);
                    if c0 <= 0.0 {
                        continue;
                    }
                    rt_vals.push(((close(b1) - c0).abs() / c0) * 10_000.0);
                    rt_bars.push(b0);
                    rt_gap.push((b1 - b0) as f64);
                }
            }
        }
        if !rt_vals.is_empty() {
            print_summary("AMP_ROUNDTRIP", lvl, &summarize(&rt_vals, &rt_bars, dates));
            print_summary(
                "GAPBARS_ROUNDTRIP",
                lvl,
                &summarize(&rt_gap, &rt_bars, dates),
            );
        } else {
            println!("P907_AMP_ROUNDTRIP lvl={lvl} n=0 (无反向配对)");
        }

        // ── 口径 STRUCT：按 source_index 升序 ─────────────────────────
        let mut st: Vec<usize> = pts.iter().map(|&(src, _, _, _)| src).collect();
        st.sort_unstable();
        let mut st_vals = Vec::new();
        let mut st_bars = Vec::new();
        for w in st.windows(2) {
            let (s0, s1) = (w[0], w[1]);
            if s0 >= n || s1 >= n || s0 == s1 {
                continue;
            }
            let c0 = close(s0);
            if c0 <= 0.0 {
                continue;
            }
            st_vals.push(((close(s1) - c0).abs() / c0) * 10_000.0);
            st_bars.push(s0);
        }
        if !st_vals.is_empty() {
            print_summary("AMP_STRUCT", lvl, &summarize(&st_vals, &st_bars, dates));
        } else {
            println!("P907_AMP_STRUCT lvl={lvl} n=0 (无相邻对)");
        }

        // ── 交易误差代理：entry_delay_bars 根 K 的价位差（实测，非估） ──
        let mut d_vals = Vec::new();
        let mut d_bars = Vec::new();
        for &(_src, cb, _, _) in pts.iter() {
            let t = cb + delay;
            if t >= n {
                continue;
            }
            let c0 = close(cb);
            if c0 <= 0.0 {
                continue;
            }
            d_vals.push(((close(t) - c0).abs() / c0) * 10_000.0);
            d_bars.push(cb);
        }
        if !d_vals.is_empty() {
            print_summary("DELAYERR", lvl, &summarize(&d_vals, &d_bars, dates));
        } else {
            println!("P907_DELAYERR lvl={lvl} n=0");
        }
    }

    eprintln!("P907_DONE");
    std::process::ExitCode::SUCCESS
}
