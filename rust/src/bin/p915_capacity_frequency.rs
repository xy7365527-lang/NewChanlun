//! # `p915_capacity_frequency` —— 逐级容量／频率／波幅阶梯（票 #915 事实底座，只读探针）
//!
//! ## 本探针回答什么
//!
//! [ADR 0016](../../../docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md) 裁定三
//! （规模 ＝ 该操作级别的容量）与裁定四（开不开这个重看频率）的**落地前置**：两个量本仓从未
//! 逐级测过。#917 已把「容量的可测代理」定死成
//!
//! ```text
//! 容量(S, ℓ) = V_日(S) · [ x · A_ℓ(S) / ( Y · σ_日(S) ) ] ^ (1/δ)
//! w(d)       = ( A_{ℓ−d} / A_ℓ ) ^ (1/δ)        ← Y、σ、V、x 全部约掉
//! ```
//!
//! ⟹ 本探针只需产**三样原料**（第四条曲线 w(d) 由这三样在报告里算出，不在本探针内算，
//! 因为 δ、x 是裁定参数不是测量量）：
//!
//! 1. **`A_ℓ` 波幅阶梯** —— 逐级 ROUNDTRIP 中位波幅（**口径照抄 #907，不重裁**：进 → 同级
//!    最近反向信号，一趟往返；理由见 #907 resolution）。
//! 2. **逐级信号频率** —— 每级单位时间的买卖点数量。**必须能抓出「某级 0 次」**
//!    （`analysis/recursive_position_experiment.md` §5.2b 实证：配额把 51.6% 绑在一个从未发
//!    事件的级别上，单修此项 +96.1pp）⟹ 本探针**报到塔在该标的上长出的最高级为止，中间空级
//!    显式打 `n_bsp=0`**，并另报 `tower_max_levels`（塔结构上到过第几级）以区分「级别不存在」
//!    与「级别存在但零信号」。
//! 3. **`σ_日` 日波动率 + `V_日` 日成交量/成交额** —— 四个 OHLC 估计量（close-to-close /
//!    Parkinson / Garman-Klass / Rogers-Satchell）各报 **RMS 与中位日两个统计量**
//!    （#915 追加二：`σ_日` 取中位日还是 RMS 未裁，**本探针两个都报，不替裁定方选**）。
//!
//! ## 口径声明（不同口径结论会翻，逐条钉死）
//!
//! ### 买卖点来源（**与 #907 逐字同源**）
//! 生产因果塔 `IncrementalClassifier::classify_at(i)` 的 `classification.levels[lvl].bsp`，
//! 六 bit（buy1/2/3 + sell1/2/3）任一为真即计一个买卖点。去重键 `(lvl, source_index, bits)`
//! 进 seen-set，身份键 `(lvl, source_index)`。**提取循环逐字照抄 `p907_cost_gate_level.rs:237-266`**
//! （其自身照抄 `p105_cert_level_spectrum.rs:234-272`）⟹ 走生产路径，无影子分叉（644 meta-rule）。
//! **级别 ＝ 位置索引**：`BspPoint` 自身无 level 字段。
//!
//! ### 两个时间坐标（照抄 #907 §1.2）
//! `source_index` ＝ 结构坐标（过去的某根 K）；`confirm_bar` ＝ 首次被塔确认的 bar（因果上
//! 最早可下单时刻）。**本探针一律用 `confirm_bar`**（可执行坐标）。
//!
//! ### 幅度定义（照抄 #907 §1.4）
//! 两端 `close` 之差的绝对值 ÷ 起点 `close`，单位 **bp**。取 close 因引擎按 bar close 撮合；
//! 取相对值因跨标的绝对 $ 幅度不可比（在案：跨品种原始 $ 拼接 ＝ 尺度伪影）。
//!
//! ### 窗口（**本探针相对 #907 的唯一新增能力**）
//! #907 只支持前缀窗（`P907_MAX_BARS`）。本探针同一趟扫描内按 `confirm_bar` 切三个窗：
//! - `FULL` ＝ `[0, n)`
//! - `H1`   ＝ `[0, n/2)`
//! - `H2`   ＝ `[n/2, n)`
//!
//! **H1 与 H2 互不重叠**（按 `confirm_bar` 的半开区间二分，交集为空，构造即证明）。配对的
//! **两个端点都必须落在窗内**才计入该窗（否则窗边界会把跨窗的长往返算成窗内样本）。
//! 因为 `classify_at` 是严格增量因果扫描（`confirm_bar == i`），`H1` 的身份集合与「用
//! `P907_MAX_BARS=n/2` 重跑一遍」逐位相同 ⟹ H1 不是近似，是精确前缀。
//!
//! ### 日波动率与日成交额（**不走量化 tick，直读原始 JSON 浮点**）
//! `Bar.volume` 是 `v as i64` **截断**（`backtest/data.rs:280`，逐行核过）——BTC 分钟成交量
//! 常在个位数到几十 BTC，截断误差不可忽略 ⟹ 本相直接用 `serde_json` 读同一文件的原始 `f64`
//! 列，**不经 `load_symbol` 的量化路径**。这不是影子分叉（不重实现任何判据），是绕开一个已知
//! 的有损转换。坏 bar 判据与 `data.rs:253-258` 逐字同（缺 OHLC / `high<max(o,c,l)` /
//! `low>min(o,c,h)` / 非正价 / `volume<=0` ⟹ 该 bar 不计入日聚合，并计数上报）。
//!
//! **一条已知的残留脏数据**：该判据**拦不住「单根 K 的 low 是坏 tick 但仍 > 0」**——实测
//! BRN `2024-02-19` 全日 O/H/C ≈ 83.2/83.6/83.35 而 L = **0.080**，`low > min(o,c,h)` 为假、
//! 价格为正 ⟹ 通过校验，该日 σ_GK 被算成 **491.6%**。**这不是本探针的 bug，是数据集自带的**，
//! 但它对 `RMS` 口径是毁灭性的（见报告 §σ 口径一节），对中位口径无影响。
//!
//! 日聚合键 ＝ `dates[i][..10]`（ISO 日期串，与 `Dataset::slice_date_window` 同口径）。
//! 逐日 O ＝ 当日首个好 bar 的 open，H ＝ 当日 high 最大，L ＝ 当日 low 最小，
//! C ＝ 当日末个好 bar 的 close，V ＝ 当日好 bar 的 volume 之和，
//! **成交额 ＝ Σ_bar (volume × close)**（不是「日量 × 日收」）。
//!
//! 四个估计量（全部对 **log 价**，方差口径，再开方得当日 σ）：
//! ```text
//! CC: σ² = (ln C_t − ln C_{t−1})²                      （需前一日，首日不计）
//! PK: σ² = (ln H − ln L)² / (4 ln 2)
//! GK: σ² = 0.5 (ln H − ln L)² − (2 ln 2 − 1)(ln C − ln O)²
//! RS: σ² = (ln H − ln C)(ln H − ln O) + (ln L − ln C)(ln L − ln O)
//! ```
//! `RMS` ＝ `sqrt(mean_t σ²_t)`；`中位日` ＝ `median_t sqrt(σ²_t)`。**两者不是同一个量，
//! #917 实测 BTC 上差 56%（容量差 2.43 倍）** ⟹ 并列报，不合并。
//!
//! ### 频率
//! - `per_year_cal` ＝ `n_bsp / (日历跨度天数 / 365.25)` —— 日历时间口径（跨标的可比性差：
//!   QQQ/OKLO 每日 6.5 小时、期货约 23 小时、BTC 24 小时）。
//! - `per_1e6_bars` ＝ `n_bsp / n × 1e6` —— **结构时间口径**（每百万根 K 的信号数），
//!   跨标的可比。**两个都报，报告里以后者为主。**
//!
//! ## 只读纪律
//! 不动判据、不写主路径状态、零生产行为变更；只往 stdout 打 `P915_*` 报表行。
//!
//! ## 用法
//! ```text
//! cargo build --release --features backtest_bin --bin p915_capacity_frequency
//! P915_SYMBOL=ES ./target/release/p915_capacity_frequency
//! # 可选 P915_MAX_BARS=1200000  限长（前缀）
//! # 可选 P915_PHASE=daily|tower|both（默认 both）——daily 相秒级，tower 相是大头
//! ```

use newchan_rust::theta_v0::backtest::data::{data_dir, load_by_symbol, SYMBOLS};
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::BspBits;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

// ────────────────────────────────────────────────────────────────────────────
// 相一：日波动率 / 日成交额（直读原始 JSON 浮点，绕开 volume 的 i64 截断）
// ────────────────────────────────────────────────────────────────────────────

/// 与 `backtest/data.rs` 的私有 `RawData` 同 schema（8 品种统一 parallel-array）。
#[derive(Deserialize, Default)]
struct RawCols {
    #[serde(default)]
    opens: Vec<Option<f64>>,
    #[serde(default)]
    highs: Vec<Option<f64>>,
    #[serde(default)]
    lows: Vec<Option<f64>>,
    #[serde(default)]
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
    #[serde(default)]
    dates: Vec<String>,
}

struct DayAgg {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    vol: f64,
    notional: f64,
    n_bars: usize,
}

/// 线性插值分位数（sorted 非空）。**与 `p907_cost_gate_level.rs:94-109` 逐字同源。**
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

fn median_of(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    pct(&v, 0.50)
}

fn mean_of(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

/// `RMS` ＝ `sqrt(mean σ²)`；`中位日` ＝ `median sqrt(σ²)`。两个统计量并列报（#915 追加二）。
fn report_sigma(tag: &str, var_per_day: &[f64]) {
    let vals: Vec<f64> = var_per_day
        .iter()
        .copied()
        .filter(|x| x.is_finite() && *x >= 0.0)
        .collect();
    if vals.is_empty() {
        println!("P915_SIGMA est={tag} n_days=0");
        return;
    }
    let rms = (vals.iter().sum::<f64>() / vals.len() as f64).sqrt();
    let mut sd: Vec<f64> = vals.iter().map(|v| v.sqrt()).collect();
    sd.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "P915_SIGMA est={tag} n_days={} rms_pct={:.4} median_pct={:.4} p25_pct={:.4} \
         p75_pct={:.4} mean_pct={:.4}",
        vals.len(),
        rms * 100.0,
        pct(&sd, 0.50) * 100.0,
        pct(&sd, 0.25) * 100.0,
        pct(&sd, 0.75) * 100.0,
        mean_of(&sd) * 100.0
    );
}

fn phase_daily(symbol: &str, max_bars: usize) {
    let Some((_, file, _)) = SYMBOLS.iter().find(|(s, _, _)| s.eq_ignore_ascii_case(symbol)) else {
        eprintln!("P915_DAILY_ERR 未知品种 {symbol}");
        return;
    };
    let path = data_dir().join(file);
    let txt = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("P915_DAILY_ERR 读文件失败 {path:?}: {e}");
            return;
        }
    };
    // Python json(allow_nan) 写出 NaN/Infinity（serde_json 硬拒）→ null。
    // **与 `backtest/data.rs:216-220` 逐字同源**（BRN/DX/QQQ 三个文件实测含 NaN，不做此替换会解析失败）。
    let txt = txt
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawCols = match serde_json::from_str(&txt) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("P915_DAILY_ERR JSON 解析失败 {path:?}: {e}");
            return;
        }
    };
    drop(txt);
    let n = raw.closes.len().min(max_bars);

    // 逐日聚合（键 = dates[i][..10]，与 slice_date_window 同口径）。
    let mut days: BTreeMap<String, DayAgg> = BTreeMap::new();
    let mut n_bad = 0usize;
    for i in 0..n {
        let (o, h, l, c) = (
            raw.opens.get(i).copied().flatten(),
            raw.highs.get(i).copied().flatten(),
            raw.lows.get(i).copied().flatten(),
            raw.closes.get(i).copied().flatten(),
        );
        let v = raw.volumes.get(i).copied().flatten().unwrap_or(0.0);
        let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) else {
            n_bad += 1;
            continue;
        };
        // 与 data.rs:250-258 同判据（坏 bar 不入日聚合）。
        let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
        let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
        if bad_range || bad_price || v <= 0.0 {
            n_bad += 1;
            continue;
        }
        let key = raw.dates[i].chars().take(10).collect::<String>();
        days.entry(key)
            .and_modify(|d| {
                d.high = d.high.max(h);
                d.low = d.low.min(l);
                d.close = c;
                d.vol += v;
                d.notional += v * c;
                d.n_bars += 1;
            })
            .or_insert(DayAgg {
                open: o,
                high: h,
                low: l,
                close: c,
                vol: v,
                notional: v * c,
                n_bars: 1,
            });
    }

    println!(
        "P915_DAILY_INPUT symbol={symbol} bars_used={n} bars_total={} bad_bars={n_bad} \
         n_days={} day_first={} day_last={}",
        raw.closes.len(),
        days.len(),
        days.keys().next().map(|s| s.as_str()).unwrap_or("?"),
        days.keys().next_back().map(|s| s.as_str()).unwrap_or("?")
    );

    let ln2 = std::f64::consts::LN_2;
    let (mut v_cc, mut v_pk, mut v_gk, mut v_rs) = (vec![], vec![], vec![], vec![]);
    let (mut vols, mut notionals, mut bars_per_day) = (vec![], vec![], vec![]);
    let mut prev_close: Option<f64> = None;
    for d in days.values() {
        let (lo_, lh, ll, lc) = (d.open.ln(), d.high.ln(), d.low.ln(), d.close.ln());
        if let Some(pc) = prev_close {
            let r = lc - pc.ln();
            v_cc.push(r * r);
        }
        prev_close = Some(d.close);
        let hl = lh - ll;
        v_pk.push(hl * hl / (4.0 * ln2));
        let co = lc - lo_;
        v_gk.push(0.5 * hl * hl - (2.0 * ln2 - 1.0) * co * co);
        v_rs.push((lh - lc) * (lh - lo_) + (ll - lc) * (ll - lo_));
        vols.push(d.vol);
        notionals.push(d.notional);
        bars_per_day.push(d.n_bars as f64);
    }

    report_sigma("CLOSE_CLOSE", &v_cc);
    report_sigma("PARKINSON", &v_pk);
    report_sigma("GARMAN_KLASS", &v_gk);
    report_sigma("ROGERS_SATCHELL", &v_rs);

    let mut vs = vols.clone();
    vs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut ns = notionals.clone();
    ns.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "P915_VOLUME n_days={} vol_native_median={:.4e} vol_native_mean={:.4e} \
         notional_median={:.4e} notional_mean={:.4e} notional_p25={:.4e} notional_p75={:.4e} \
         bars_per_day_median={:.1}",
        vols.len(),
        pct(&vs, 0.50),
        mean_of(&vols),
        pct(&ns, 0.50),
        mean_of(&notionals),
        pct(&ns, 0.25),
        pct(&ns, 0.75),
        median_of(bars_per_day)
    );
}

// ────────────────────────────────────────────────────────────────────────────
// 相二：塔扫描 → 逐级买卖点 → 频率 + ROUNDTRIP 波幅阶梯
// ────────────────────────────────────────────────────────────────────────────

/// 与 runner.rs `bsp_bits_disc` 同口径。**照抄 `p907_cost_gate_level.rs:80-87`。**
fn bsp_bits_disc(b: &BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// **照抄 `p907_cost_gate_level.rs:89-91`。**
fn has_any_bit(b: &BspBits) -> bool {
    b.buy1 || b.buy2 || b.buy3 || b.sell1 || b.sell2 || b.sell3
}

/// 一组样本的摘要（**照抄 `p907_cost_gate_level.rs:112-178` 的 `Summary`/`summarize`**，
/// 以保证与 #907 的读数逐位可比）。
struct Summary {
    n: usize,
    mean: f64,
    p10: f64,
    p25: f64,
    p50: f64,
    p75: f64,
    p90: f64,
    max_month_share: f64,
    max_4k_share: f64,
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
    let mut month: BTreeMap<String, usize> = BTreeMap::new();
    for &b in bars {
        let key = dates
            .get(b)
            .map(|d| d.chars().take(7).collect::<String>())
            .unwrap_or_else(|| "????-??".to_string());
        *month.entry(key).or_default() += 1;
    }
    let max_month = month.values().copied().max().unwrap_or(0);
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

fn print_summary(tag: &str, win: &str, lvl: usize, s: &Summary) {
    println!(
        "P915_{tag} win={win} lvl={lvl} n={} mean_bp={:.1} p10_bp={:.1} p25_bp={:.1} \
         p50_bp={:.1} p75_bp={:.1} p90_bp={:.1} max_month_share={:.3} max_4kbar_share={:.4} \
         n_months={}",
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

/// 一个买卖点身份（去重后）。
#[derive(Clone, Copy)]
struct Ident {
    confirm_bar: usize,
    buy: bool,
    sell: bool,
}

fn main() -> std::process::ExitCode {
    let symbol = std::env::var("P915_SYMBOL").unwrap_or_else(|_| "BTC".to_string());
    let phase = std::env::var("P915_PHASE").unwrap_or_else(|_| "both".to_string());
    let max_bars: usize = std::env::var("P915_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);

    if phase == "daily" || phase == "both" {
        phase_daily(&symbol, max_bars);
    }
    if phase == "daily" {
        eprintln!("P915_DONE symbol={symbol} phase=daily");
        return std::process::ExitCode::SUCCESS;
    }

    let config = ThetaConfig::default();
    let dataset = match load_by_symbol(&symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let n_full = dataset.bars.len();
    let n = n_full.min(max_bars);
    let bars = &dataset.bars[..n];
    let dates = &dataset.dates;

    println!(
        "P915_INPUT symbol={symbol} bars_used={n} bars_total={n_full} date_first={} date_last={}",
        dates.first().map(|s| s.as_str()).unwrap_or("?"),
        dates.get(n - 1).map(|s| s.as_str()).unwrap_or("?")
    );

    // ── 生产因果塔扫描（提取循环逐字照抄 p907_cost_gate_level.rs:237-266）──────
    let mut classifier = IncrementalClassifier::new(bars, &config);
    let mut seen: HashSet<(usize, usize, u8)> = HashSet::new();
    let mut first_confirm: HashMap<(usize, usize), (usize, bool, bool)> = HashMap::new();
    let mut events_total = 0usize;
    // 塔结构上到过第几级（区分「该级不存在」与「该级存在但零信号」——E-1 那个坑）。
    let mut tower_max_levels = 0usize;
    for i in 0..n {
        let (classification, _tower) = classifier.classify_at(i);
        tower_max_levels = tower_max_levels.max(classification.levels.len());
        for (lvl, ls) in classification.levels.iter().enumerate() {
            for p in ls.bsp.iter() {
                if seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))) && has_any_bit(&p.bits)
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
            eprintln!("P915_PROGRESS symbol={symbol} bar={i}/{n} ids={}", first_confirm.len());
        }
    }

    let mut by_lvl: BTreeMap<usize, Vec<Ident>> = BTreeMap::new();
    for (&(lvl, _src), &(cb, bu, se)) in first_confirm.iter() {
        by_lvl.entry(lvl).or_default().push(Ident {
            confirm_bar: cb,
            buy: bu,
            sell: se,
        });
    }
    let max_lvl_nonempty = by_lvl.keys().copied().max().unwrap_or(0);
    println!(
        "P915_BSP symbol={symbol} ids_total={} events_total={events_total} \
         max_lvl_nonempty={max_lvl_nonempty} tower_max_levels={tower_max_levels}",
        first_confirm.len()
    );

    let close = |b: usize| bars[b].close as f64;
    let span_days = if n >= 2 {
        // 日历跨度：首末 bar 的 ISO 日期差，按 (y,m,d) 折算儒略日近似（只用于频率归一）。
        fn jd(s: &str) -> f64 {
            let y: i64 = s.get(0..4).and_then(|x| x.parse().ok()).unwrap_or(0);
            let m: i64 = s.get(5..7).and_then(|x| x.parse().ok()).unwrap_or(1);
            let d: i64 = s.get(8..10).and_then(|x| x.parse().ok()).unwrap_or(1);
            let a = (14 - m) / 12;
            let y2 = y + 4800 - a;
            let m2 = m + 12 * a - 3;
            (d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045) as f64
        }
        (jd(&dates[n - 1]) - jd(&dates[0])).max(1.0)
    } else {
        1.0
    };
    println!("P915_SPAN symbol={symbol} span_cal_days={span_days:.0} bars_used={n}");

    // 三个窗（H1/H2 互不重叠：半开区间二分，交集为空）。
    let half = n / 2;
    let windows: [(&str, usize, usize); 3] = [("FULL", 0, n), ("H1", 0, half), ("H2", half, n)];

    // 报到「塔结构上到过的最高级」为止（不是「有信号的最高级」）——中间/末尾空级要显式打 0。
    let report_upto = tower_max_levels.max(max_lvl_nonempty + 1);
    for lvl in 0..report_upto {
        let empty: Vec<Ident> = Vec::new();
        let pts = by_lvl.get(&lvl).unwrap_or(&empty);

        for (win, w_lo, w_hi) in windows {
            let mut ex: Vec<Ident> = pts
                .iter()
                .copied()
                .filter(|p| p.confirm_bar >= w_lo && p.confirm_bar < w_hi)
                .collect();
            ex.sort_by_key(|p| p.confirm_bar);
            let n_bsp = ex.len();
            let n_buy = ex.iter().filter(|p| p.buy).count();
            let n_sell = ex.iter().filter(|p| p.sell).count();
            let w_bars = w_hi.saturating_sub(w_lo);
            let w_days = span_days * (w_bars as f64) / (n.max(1) as f64);
            println!(
                "P915_FREQ symbol={symbol} win={win} lvl={lvl} n_bsp={n_bsp} n_buy={n_buy} \
                 n_sell={n_sell} win_bars={w_bars} per_1e6_bars={:.2} per_year_cal={:.2} \
                 tower_has_level={}",
                if w_bars == 0 {
                    f64::NAN
                } else {
                    n_bsp as f64 / w_bars as f64 * 1e6
                },
                if w_days <= 0.0 {
                    f64::NAN
                } else {
                    n_bsp as f64 / (w_days / 365.25)
                },
                if lvl < tower_max_levels { 1 } else { 0 }
            );

            if n_bsp == 0 {
                println!("P915_AMP_ROUNDTRIP symbol={symbol} win={win} lvl={lvl} n=0 (无买卖点)");
                continue;
            }

            // ── ROUNDTRIP（照抄 p907_cost_gate_level.rs:341-364 的配对规则）─────
            // 从带买 bit 的点出发找同级 confirm_bar 严格更晚的第一个带卖 bit 的点；反向亦然。
            // **两端都必须在窗内**（`ex` 已按窗过滤 ⟹ 天然满足）。
            let mut rt_vals: Vec<f64> = Vec::new();
            let mut rt_bars: Vec<usize> = Vec::new();
            let mut rt_gap: Vec<f64> = Vec::new();
            for (k, p0) in ex.iter().enumerate() {
                let b0 = p0.confirm_bar;
                for want_sell in [true, false] {
                    let start_ok = if want_sell { p0.buy } else { p0.sell };
                    if !start_ok {
                        continue;
                    }
                    let hit = ex[k + 1..].iter().find(|p1| {
                        p1.confirm_bar > b0 && if want_sell { p1.sell } else { p1.buy }
                    });
                    if let Some(p1) = hit {
                        let c0 = close(b0);
                        if c0 <= 0.0 {
                            continue;
                        }
                        rt_vals.push(((close(p1.confirm_bar) - c0).abs() / c0) * 10_000.0);
                        rt_bars.push(b0);
                        rt_gap.push((p1.confirm_bar - b0) as f64);
                    }
                }
            }
            if rt_vals.is_empty() {
                println!(
                    "P915_AMP_ROUNDTRIP symbol={symbol} win={win} lvl={lvl} n=0 (无反向配对)"
                );
            } else {
                print_summary(
                    "AMP_ROUNDTRIP",
                    win,
                    lvl,
                    &summarize(&rt_vals, &rt_bars, dates),
                );
                print_summary(
                    "GAPBARS_ROUNDTRIP",
                    win,
                    lvl,
                    &summarize(&rt_gap, &rt_bars, dates),
                );
            }
        }
    }

    eprintln!("P915_DONE symbol={symbol}");
    std::process::ExitCode::SUCCESS
}
