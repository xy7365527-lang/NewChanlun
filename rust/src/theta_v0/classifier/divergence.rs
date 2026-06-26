//! 背驰度量（reference-theta-v0.md:37）——MACD 辅助 + 同向段面积严格变小。
//!
//! ## bit-exact 注意点（★浮点域隔离）
//!
//! MACD 是 v0 的**辅助**度量（结构前提优先）。MACD 浮点运算**隔离在本文件**，按固定
//! 约简顺序计算，**不漏入整数 tick 域**（types.rs 的结构判定全在 i64）。背驰输出是
//! bool（严格变小），bool 无浮点歧义——浮点只在内部面积比较时出现，且用严格 `<`。
//!
//! ## MACD(12,26,9) 算法（reference-theta-v0.md:37，固定约简顺序）
//!
//! - `EMA[0] = close[0]`（★首值取首 close，非 SMA 预热）。
//! - `EMA[i] = α*close[i] + (1-α)*EMA[i-1]`，`α = 2/(period+1)`。
//! - `DIF = EMA_fast - EMA_slow`（逐 bar）。
//! - `DEA = EMA(DIF, signal)`，DEA 首值取首 DIF。
//! - `hist = DIF - DEA`（逐 bar）。
//!
//! ## 背驰判据（reference-theta-v0.md:37）
//!
//! 「同向段面积**严格**变小才成立，等值不成立」。段面积 = 该段 bar 区间内 `|hist|` 之和。
//! 后一同向段面积 `<` 前一同向段面积 ⟹ 背驰成立（严格 `<`，等值返回 false）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! MACD 常数从 config 读（fast/slow/signal）。EMA 首值规则 + 面积规则标 `[L3经验待标定]`
//! （reference-theta-v0.md:37）——v0 默认占位，L2/L3 标定不归本工位。本文件证的是
//! 「给定 MACD 参数后面积比较确定」（L1 管线正确性），**不**证「MACD 背驰预测有效」。

use super::super::config::MacdConfig;

/// MACD 逐 bar 输出（DIF/DEA/hist，浮点域，隔离在本结构）。
#[derive(Debug, Clone, PartialEq)]
pub struct MacdSeries {
    pub dif: Vec<f64>,
    pub dea: Vec<f64>,
    pub hist: Vec<f64>,
}

/// EMA（首值取首元素，reference-theta-v0.md:37）。
///
/// `out[0]=src[0]`；`out[i]=α*src[i]+(1-α)*out[i-1]`，`α=2/(period+1)`。
/// bit-exact：α 与递推按固定顺序计算（先 `α*src[i]`，再 `(1-α)*prev`，最后相加）。
/// 边界条件：`src` 为空 ⟹ 空序列；`period=0` 在 config 校验层拒绝（此处假设 >=1）。
fn ema(src: &[f64], period: u32) -> Vec<f64> {
    let mut out = Vec::with_capacity(src.len());
    if src.is_empty() {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    out.push(src[0]); // 首值取首元素（非 SMA 预热）。
    for i in 1..src.len() {
        // 固定约简顺序：α*src[i] + (1-α)*prev。
        let prev = out[i - 1];
        let v = alpha * src[i] + (1.0 - alpha) * prev;
        out.push(v);
    }
    out
}

/// 计算 MACD 序列（reference-theta-v0.md:37，固定约简顺序）。
///
/// `closes` 是逐 bar 收盘价（已量化的 tick 转 f64——MACD 在浮点域，但输入来自整数 tick
/// 的确定转换，无额外浮点引入）。返回 DIF/DEA/hist 三序列（等长 = closes 长度）。
pub fn compute_macd(closes: &[f64], cfg: &MacdConfig) -> MacdSeries {
    let ema_fast = ema(closes, cfg.fast);
    let ema_slow = ema(closes, cfg.slow);
    // DIF = EMA_fast - EMA_slow（逐 bar，等长）。
    let dif: Vec<f64> = ema_fast
        .iter()
        .zip(ema_slow.iter())
        .map(|(f, s)| f - s)
        .collect();
    // DEA = EMA(DIF, signal)，首值取首 DIF。
    let dea = ema(&dif, cfg.signal);
    // hist = DIF - DEA。
    let hist: Vec<f64> = dif.iter().zip(dea.iter()).map(|(d, e)| d - e).collect();
    MacdSeries { dif, dea, hist }
}

/// 同向段 MACD 面积（reference-theta-v0.md:37）= 段 bar 区间 `[start,end]` 内 `|hist|` 之和。
///
/// `[start, end]` 是闭区间 bar 索引。越界（end>=len 或 start>end）⟹ 0.0（空段无面积）。
/// bit-exact：求和按 bar 索引升序累加（固定顺序），用 `f64::abs`。
pub fn segment_macd_area(hist: &[f64], start: usize, end: usize) -> f64 {
    if start > end || end >= hist.len() {
        return 0.0;
    }
    let mut area = 0.0;
    for &h in &hist[start..=end] {
        area += h.abs();
    }
    area
}

/// 背驰判定（reference-theta-v0.md:37）：后一同向段面积**严格**小于前一同向段。
///
/// `prev_area`/`curr_area` 是两个同向段的 MACD 面积。`curr < prev`（严格 `<`）⟹ 背驰
/// 成立；等值（`curr == prev`）或变大 ⟹ 不成立。等值不成立是 reference-theta-v0.md:37
/// 明确规则（不是浮点容差问题——段面积相等表示力度未衰减，非背驰）。
pub fn is_divergence(prev_area: f64, curr_area: f64) -> bool {
    curr_area < prev_area
}

/// 两段背驰判定（端到端）：从 hist 序列取两同向段面积并比较。
///
/// `prev_seg`/`curr_seg` 是 `(start,end)` 闭区间 bar 索引对。后段面积严格小于前段 ⟹ 背驰。
pub fn segments_diverge(
    hist: &[f64],
    prev_seg: (usize, usize),
    curr_seg: (usize, usize),
) -> bool {
    let prev_area = segment_macd_area(hist, prev_seg.0, prev_seg.1);
    let curr_area = segment_macd_area(hist, curr_seg.0, curr_seg.1);
    is_divergence(prev_area, curr_area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ema_first_value_is_first_close() {
        // EMA 首值取首 close（reference-theta-v0.md:37）。
        let src = vec![10.0, 20.0, 30.0];
        let out = ema(&src, 2);
        assert_eq!(out[0], 10.0); // 首值 = src[0]
        // α=2/3：out[1]=2/3*20+1/3*10=16.666...
        let alpha = 2.0 / 3.0;
        assert!((out[1] - (alpha * 20.0 + (1.0 - alpha) * 10.0)).abs() < 1e-12);
    }

    #[test]
    fn ema_empty_yields_empty() {
        assert_eq!(ema(&[], 12), Vec::<f64>::new());
    }

    #[test]
    fn macd_series_equal_length() {
        let closes: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64)).collect();
        let cfg = MacdConfig::default();
        let m = compute_macd(&closes, &cfg);
        assert_eq!(m.dif.len(), closes.len());
        assert_eq!(m.dea.len(), closes.len());
        assert_eq!(m.hist.len(), closes.len());
    }

    #[test]
    fn macd_hist_equals_dif_minus_dea() {
        // hist = DIF - DEA（reference-theta-v0.md:37）逐 bar 验证。
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + (i as f64 * 0.5)).collect();
        let cfg = MacdConfig::default();
        let m = compute_macd(&closes, &cfg);
        for i in 0..m.hist.len() {
            assert!((m.hist[i] - (m.dif[i] - m.dea[i])).abs() < 1e-12);
        }
    }

    #[test]
    fn segment_area_sums_abs_hist() {
        let hist = vec![1.0, -2.0, 3.0, -4.0];
        // [0,2]：|1|+|-2|+|3|=6。
        assert_eq!(segment_macd_area(&hist, 0, 2), 6.0);
        // [1,3]：2+3+4=9。
        assert_eq!(segment_macd_area(&hist, 1, 3), 9.0);
    }

    #[test]
    fn segment_area_out_of_bounds_zero() {
        let hist = vec![1.0, 2.0];
        assert_eq!(segment_macd_area(&hist, 0, 5), 0.0); // end 越界
        assert_eq!(segment_macd_area(&hist, 3, 1), 0.0); // start>end
    }

    #[test]
    fn divergence_strict_less_only() {
        // 严格变小 ⟹ 背驰（reference-theta-v0.md:37）。
        assert!(is_divergence(10.0, 5.0));
        // 等值 ⟹ 不成立（等值不成立的明确规则）。
        assert!(!is_divergence(10.0, 10.0));
        // 变大 ⟹ 不成立。
        assert!(!is_divergence(10.0, 15.0));
    }

    #[test]
    fn segments_diverge_end_to_end() {
        // 前段 [0,1] 面积大，后段 [2,3] 面积小 ⟹ 背驰。
        let hist = vec![5.0, -5.0, 1.0, -1.0];
        assert!(segments_diverge(&hist, (0, 1), (2, 3))); // 10 vs 2 → 严格小
        // 反向：后段面积大 ⟹ 不背驰。
        assert!(!segments_diverge(&hist, (2, 3), (0, 1)));
    }

    /// property：面积非负（|hist| 之和）。
    #[test]
    fn property_area_nonnegative() {
        let closes: Vec<f64> = (0..30).map(|i| 100.0 + ((i * 7 % 13) as f64)).collect();
        let cfg = MacdConfig::default();
        let m = compute_macd(&closes, &cfg);
        for start in 0..m.hist.len() {
            for end in start..m.hist.len() {
                assert!(segment_macd_area(&m.hist, start, end) >= 0.0);
            }
        }
    }
}
