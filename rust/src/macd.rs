//! GUARD-ROLE: toplevel-loose-files——名分：现役（详见 `lib.rs` 头部 GUARD-ROLE 块，#764 C7-E5 核定；零删除/零移入 legacy/）。
//!
//! MACD 力度层 — 逐位等价移植自 src/newchan/a_macd.py + a_divergence_v1.py（MACD 维度）。
//!
//! 第七层。从原始 close 价格完整算出 MACD（EMA → DIF/DEA/hist → area/peak），
//! 复刻 Python 的完整管线，不接收预算数组。
//!
//! ## 两条 EMA 路径（Python 内部存在的递推形式分歧——逐位忠实移植，不弥合）
//!
//! Python 有两个产生 MACD 的入口，它们的 EMA 递推**浮点形式不同**，因此 bit-exact 结果
//! 不同（差异 ~1e-14）。这不是本移植引入的，是 Python 既有事实：
//!
//! | Python 入口 | 递推形式 | 生产用途 |
//! |-------------|---------|---------|
//! | `compute_macd`（pandas ewm adjust=False） | `prev + α·(x−prev)` | 批量（ab_bridge / nested_pipeline） |
//! | `OnlineMacdState.update` | `α·x + (1−α)·prev` | 在线 RTAS（orchestrator/recursive） |
//!
//! 经验验证（src/newchan/a_macd.py）：pandas `ewm(adjust=False).mean()` 的内部实现是
//! `weighted = prev + alpha*(cur - prev)`（不是代数等价的 `alpha*cur + (1-alpha)*prev`，
//! 二者末位 ULP 不同）。`OnlineMacdState` 用的是后者。本模块两条路径分别复刻，
//! `compute_macd_batch` ↔ Python `compute_macd`，`OnlineMacdState` ↔ Python `OnlineMacdState`。
//!
//! ## 浮点累加树（area 的 bit-exact 核心）
//!
//! Python `macd_area_for_range` 用 `pandas.Series.sum()`，经验证等于 `numpy.sum()`——
//! numpy 用 **pairwise summation**（块大小 128，块内 8 路展开）。naive 左到右累加不 bit-exact。
//! 本模块 `pairwise_sum` 精确复刻 numpy `pairwise_sum`（numpy/_core/src/umath/loops_utils.h.src）。
//!
//! ## 认识论等级
//! - EMA 递推：L0（数学恒等式，但浮点形式必须与 Python 对齐——见上）。
//! - pairwise 累加树：L0（复刻 numpy 算法），golden 测试在 L1/L2 数据上 bit-exact。

/// numpy pairwise summation —— 逐位复刻 numpy `pairwise_sum`（C 实现）。
///
/// 算法（numpy/_core/src/umath/loops_utils.h.src）：
/// - n < 8：朴素左到右累加。
/// - 8 ≤ n ≤ 128：8 个累加器展开，尾部（n%8）朴素累加，最后按 `((r0+r1)+(r2+r3))+((r4+r5)+(r6+r7))` 合并。
/// - n > 128：在 `n2 = (n/2) 向下取整到 8 的倍数` 处二分，递归相加。
///
/// `pandas.Series.sum()` == `numpy.sum()` == 本函数（已在 Python 端经验验证跨边界 7/8/128/129/256/...）。
fn pairwise_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0_f64;
        for &x in a {
            res += x;
        }
        res
    } else if n <= 128 {
        // 8 路累加器展开。
        let mut r0 = a[0];
        let mut r1 = a[1];
        let mut r2 = a[2];
        let mut r3 = a[3];
        let mut r4 = a[4];
        let mut r5 = a[5];
        let mut r6 = a[6];
        let mut r7 = a[7];
        let mut i = 8;
        let main_end = n - (n % 8);
        while i < main_end {
            r0 += a[i];
            r1 += a[i + 1];
            r2 += a[i + 2];
            r3 += a[i + 3];
            r4 += a[i + 4];
            r5 += a[i + 5];
            r6 += a[i + 6];
            r7 += a[i + 7];
            i += 8;
        }
        let mut res = ((r0 + r1) + (r2 + r3)) + ((r4 + r5) + (r6 + r7));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise_sum(&a[..n2]) + pairwise_sum(&a[n2..])
    }
}

/// MACD 三列结果（与 Python `compute_macd` 返回的 DataFrame 三列等价）。
#[derive(Debug, Clone)]
pub struct MacdSeries {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

/// pandas `ewm(span, adjust=False).mean()` 的逐位等价。
///
/// 逐位复刻 pandas Cython `ewm`（pandas/_libs/window/aggregations.pyx，adjust=False、
/// normalize=True、无 deltas、无 NaN 路径）：
/// ```text
///   alpha = 1/(1+com)，com = (span-1)/2，old_wt_factor = 1-alpha，new_wt = alpha
///   weighted = vals[0]；old_wt = 1.0
///   for cur in vals[1..]:
///       old_wt *= old_wt_factor
///       if weighted != cur:                    # 常数序列规避数值误差
///           weighted = old_wt*weighted + new_wt*cur
///           weighted /= (old_wt + new_wt)
///       old_wt = 1.0
/// ```
///
/// **关键 bit-exact 细节（经 Rust↔pandas 5000 点全管线零失配验证）**：
/// 1. alpha 用 `1/(1+(span-1)/2)`（与 `2/(span+1)` 在双精度下相等，但按源码写法保留）。
/// 2. `if weighted != cur` 守卫不可省（常数段跳过更新）。
/// 3. 分子 `old_wt*weighted + new_wt*cur` 是否收缩取决于 pandas wheel 的目标特征：
///    AArch64（以及显式启用 `fma` 的 x86）会融合为 FMA；manylinux 基线 x86_64
///    不含 `fma`，保留两次乘法再加法。Rust 必须跟随同一目标特征，否则第二个点
///    就会产生 ULP 分歧。
/// 4. `weighted /= (old_wt + new_wt)` 是独立的第二步除法（先乘加后除，不可合并）。
///
/// alpha = 2 / (span + 1) 数值上等于源码 `1/(1+com)`；此处按源码形式书写以杜绝歧义。
/// 空输入返回空。
fn ewm_adjust_false(values: &[f64], span: i64) -> Vec<f64> {
    const USE_FMA: bool = cfg!(any(
        target_arch = "aarch64",
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "fma"
        )
    ));

    let mut out = Vec::with_capacity(values.len());
    if values.is_empty() {
        return out;
    }
    let com = (span as f64 - 1.0) / 2.0;
    let alpha = 1.0 / (1.0 + com);
    let old_wt_factor = 1.0 - alpha;
    let new_wt = alpha;

    let mut weighted = values[0];
    let mut old_wt = 1.0_f64;
    out.push(weighted);

    for &cur in &values[1..] {
        old_wt *= old_wt_factor;
        if weighted != cur {
            let new_term = new_wt * cur;
            if USE_FMA {
                weighted = old_wt.mul_add(weighted, new_term);
            } else {
                weighted = old_wt * weighted + new_term;
            }
            weighted /= old_wt + new_wt;
        }
        old_wt = 1.0;
        out.push(weighted);
    }
    out
}

/// 批量 MACD —— 逐位等价于 Python `compute_macd(df_raw, fast, slow, signal)`。
///
/// close → ema_fast - ema_slow = macd；signal = ewm(macd)；hist = macd - signal。
/// 所有 EMA 走 pandas ewm(adjust=False) 形式。
pub fn compute_macd_batch(close: &[f64], fast: i64, slow: i64, signal: i64) -> MacdSeries {
    let ema_fast = ewm_adjust_false(close, fast);
    let ema_slow = ewm_adjust_false(close, slow);
    let macd_line: Vec<f64> = ema_fast
        .iter()
        .zip(ema_slow.iter())
        .map(|(f, s)| f - s)
        .collect();
    let signal_line = ewm_adjust_false(&macd_line, signal);
    let hist: Vec<f64> = macd_line
        .iter()
        .zip(signal_line.iter())
        .map(|(m, s)| m - s)
        .collect();
    MacdSeries {
        macd: macd_line,
        signal: signal_line,
        hist,
    }
}

/// 在线 MACD 状态机 —— 逐位等价于 Python `OnlineMacdState`。
///
/// 每 bar O(1) 更新。递推形式 `α·x + (1−α)·prev`（**与 batch 路径不同**——见模块文档）。
/// 保留完整 macd/signal/hist 历史，等价于 `to_dataframe()`。
#[derive(Debug, Clone)]
pub struct OnlineMacdState {
    alpha_fast: f64,
    alpha_slow: f64,
    alpha_sig: f64,
    ema_fast: Option<f64>,
    ema_slow: Option<f64>,
    signal_val: Option<f64>,
    macd_hist: Vec<f64>,
    signal_hist: Vec<f64>,
    hist_hist: Vec<f64>,
}

impl OnlineMacdState {
    pub fn new(fast: i64, slow: i64, signal: i64) -> Self {
        OnlineMacdState {
            alpha_fast: 2.0 / (fast as f64 + 1.0),
            alpha_slow: 2.0 / (slow as f64 + 1.0),
            alpha_sig: 2.0 / (signal as f64 + 1.0),
            ema_fast: None,
            ema_slow: None,
            signal_val: None,
            macd_hist: Vec::new(),
            signal_hist: Vec::new(),
            hist_hist: Vec::new(),
        }
    }

    /// 摄入一根 bar 的 close，返回 (macd, signal, hist)。O(1)。
    ///
    /// 逐位复刻 Python `update`：首值 seed = close；其后 `a*close + (1-a)*prev`。
    pub fn update(&mut self, close: f64) -> (f64, f64, f64) {
        let a_f = self.alpha_fast;
        let a_s = self.alpha_slow;
        let a_g = self.alpha_sig;

        match (self.ema_fast, self.ema_slow) {
            (None, _) | (_, None) => {
                self.ema_fast = Some(close);
                self.ema_slow = Some(close);
            }
            (Some(ef), Some(es)) => {
                self.ema_fast = Some(a_f * close + (1.0 - a_f) * ef);
                self.ema_slow = Some(a_s * close + (1.0 - a_s) * es);
            }
        }

        let macd = self.ema_fast.unwrap() - self.ema_slow.unwrap();

        let signal_val = match self.signal_val {
            None => macd,
            Some(sv) => a_g * macd + (1.0 - a_g) * sv,
        };
        self.signal_val = Some(signal_val);

        let hist = macd - signal_val;

        self.macd_hist.push(macd);
        self.signal_hist.push(signal_val);
        self.hist_hist.push(hist);

        (macd, signal_val, hist)
    }

    /// 返回完整历史（等价于 Python `to_dataframe()` 的三列）。
    pub fn series(&self) -> MacdSeries {
        MacdSeries {
            macd: self.macd_hist.clone(),
            signal: self.signal_hist.clone(),
            hist: self.hist_hist.clone(),
        }
    }

    pub fn n_bars(&self) -> usize {
        self.macd_hist.len()
    }

    pub fn reset(&mut self) {
        self.ema_fast = None;
        self.ema_slow = None;
        self.signal_val = None;
        self.macd_hist.clear();
        self.signal_hist.clear();
        self.hist_hist.clear();
    }
}

/// MACD 区间面积结果 —— 对应 Python `macd_area_for_range` 返回 dict。
#[derive(Debug, Clone, Copy)]
pub struct MacdArea {
    pub area_total: f64,
    pub area_pos: f64,
    pub area_neg: f64,
    pub n_bars: usize,
}

/// `round(x, 6)` —— 复刻 Python 内置 `round`（banker's rounding，round-half-to-even）。
///
/// Python3 `round` 用 IEEE-754 round-half-to-even 并且正确处理十进制（基于 repr 的
/// 双精度正确舍入）。对 6 位小数，CPython 走 `_Py_dg_dtoa`/`double_round`，等价于
/// 「乘 1e6、round-half-to-even、除 1e6」在双精度下的正确舍入版本。
///
/// 为 bit-exact，本实现复刻 CPython `float.__round__`(ndigits) 的算法：用十进制字符串
/// 正确舍入。Rust 标准库无此原语，因此用 `format!("{:.6}")`（Grisu/Ryū 正确舍入到 6 位，
/// round-half-to-even）再 parse 回 f64。
fn round6(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    // Rust `{:.6}` 使用正确舍入（round-half-to-even），与 CPython round(x, 6) 一致。
    let s = format!("{x:.6}");
    s.parse::<f64>().unwrap()
}

/// 计算指定 raw bar 范围 [raw_i0, raw_i1]（闭区间）内的 MACD 面积。
///
/// 逐位等价于 Python `macd_area_for_range(df_macd, raw_i0, raw_i1)`：
/// - 范围钳制（raw_i0<0→0；raw_i1≥len→len-1；raw_i0>raw_i1→全 0）。
/// - area_total = pairwise_sum(hist)；area_pos = pairwise_sum(clip_lower0(hist))；
///   area_neg = pairwise_sum(clip_upper0(hist))。
/// - 三个面积 round(·, 6)。
///
/// `raw_i0`/`raw_i1` 用 i64 复刻 Python int（可为负）。
pub fn macd_area_for_range(hist: &[f64], raw_i0: i64, raw_i1: i64) -> MacdArea {
    let len = hist.len() as i64;
    let mut i0 = raw_i0;
    let mut i1 = raw_i1;
    if i0 < 0 {
        i0 = 0;
    }
    if i1 >= len {
        i1 = len - 1;
    }
    if i0 > i1 {
        return MacdArea {
            area_total: 0.0,
            area_pos: 0.0,
            area_neg: 0.0,
            n_bars: 0,
        };
    }
    let slice = &hist[i0 as usize..=i1 as usize];
    // clip(lower=0): 负数→0.0；clip(upper=0): 正数→0.0。
    let pos: Vec<f64> = slice
        .iter()
        .map(|&v| if v < 0.0 { 0.0 } else { v })
        .collect();
    let neg: Vec<f64> = slice
        .iter()
        .map(|&v| if v > 0.0 { 0.0 } else { v })
        .collect();

    MacdArea {
        area_total: round6(pairwise_sum(slice)),
        area_pos: round6(pairwise_sum(&pos)),
        area_neg: round6(pairwise_sum(&neg)),
        n_bars: slice.len(),
    }
}

/// DIF（黄白线 = macd 列）区间峰值 —— 逐位等价于 Python `dif_peak_for_range`。
///
/// trend_direction "up" → max(0, max(dif))；"down" → abs(min(0, min(dif)))。
/// 非法范围返回 0.0。max/min 顺序约简（与 Python `Series.max()/min()` = numpy 一致，
/// numpy max/min 是逐元素比较，无重排，bit-exact）。
pub fn dif_peak_for_range(macd: &[f64], raw_i0: i64, raw_i1: i64, up: bool) -> f64 {
    let len = macd.len() as i64;
    if raw_i0 > raw_i1 || raw_i0 < 0 || raw_i1 >= len {
        return 0.0;
    }
    let slice = &macd[raw_i0 as usize..=raw_i1 as usize];
    if slice.is_empty() {
        return 0.0;
    }
    if up {
        let mx = series_max(slice);
        mx.max(0.0)
    } else {
        let mn = series_min(slice);
        mn.min(0.0).abs()
    }
}

/// HIST（柱子）区间峰值 —— 逐位等价于 Python `histogram_peak_for_range`。
pub fn histogram_peak_for_range(hist: &[f64], raw_i0: i64, raw_i1: i64, up: bool) -> f64 {
    let len = hist.len() as i64;
    if raw_i0 > raw_i1 || raw_i0 < 0 || raw_i1 >= len {
        return 0.0;
    }
    let slice = &hist[raw_i0 as usize..=raw_i1 as usize];
    if slice.is_empty() {
        return 0.0;
    }
    if up {
        series_max(slice).max(0.0)
    } else {
        series_min(slice).min(0.0).abs()
    }
}

/// pandas `Series.max()` 等价（= numpy max，逐元素比较，无重排）。slice 非空。
fn series_max(slice: &[f64]) -> f64 {
    let mut m = slice[0];
    for &v in &slice[1..] {
        if v > m {
            m = v;
        }
    }
    m
}

/// pandas `Series.min()` 等价。slice 非空。
fn series_min(slice: &[f64]) -> f64 {
    let mut m = slice[0];
    for &v in &slice[1..] {
        if v < m {
            m = v;
        }
    }
    m
}

/// B 段黄白线（DIF = macd 列）是否穿越 0 轴 —— 逐位等价于 Python `_b_segment_crosses_zero`。
///
/// 方案3：穿越 = 范围内同时存在 strictly positive 与 strictly negative 的 macd 值。
/// 范围钳制与 Python 一致（raw_i0<0→0；raw_i1≥len→len-1；raw_i0>raw_i1→False）。
pub fn b_segment_crosses_zero(macd: &[f64], raw_i0: i64, raw_i1: i64) -> bool {
    let len = macd.len() as i64;
    let mut i0 = raw_i0;
    let mut i1 = raw_i1;
    if i0 < 0 {
        i0 = 0;
    }
    if i1 >= len {
        i1 = len - 1;
    }
    if i0 > i1 {
        return false;
    }
    let slice = &macd[i0 as usize..=i1 as usize];
    let has_positive = slice.iter().any(|&v| v > 0.0);
    let has_negative = slice.iter().any(|&v| v < 0.0);
    has_positive && has_negative
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_small() {
        // n<8 朴素
        assert_eq!(pairwise_sum(&[1.0, 2.0, 3.0]), 6.0);
    }

    #[test]
    fn area_empty_range() {
        let a = macd_area_for_range(&[1.0, -1.0], 5, 10);
        // raw_i0=5 钳到... i1=1, i0=5>i1 → 全0
        assert_eq!(a.n_bars, 0);
    }

    #[test]
    fn round6_basic() {
        assert_eq!(round6(25.123940487230932), 25.12394);
    }
}
