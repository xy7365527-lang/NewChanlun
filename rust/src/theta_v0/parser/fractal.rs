//! 第二步：分型识别（reference-theta-v0.md:20，[缠论可导,62课]）。
//!
//! ## 规则（reference-theta-v0.md:20 逐字）
//!
//! - **输入**：包含处理后的 K 序列（inclusion.rs 输出的 merged bars）。
//! - **顶分型**：中 K 的 high 和 low 都**严格**高于左右相邻 K（`等价不成立`）。
//! - **底分型**：中 K 的 high 和 low 都**严格**低于左右相邻 K。
//! - **确认**：第三根 K 收盘确认（即三根 K 构成分型，分型在第三根确认）。
//!
//! ## bit-exact 注意点
//!
//! - **严格不等号**（`>` / `<`，非 `>=`/`<=`）：reference-theta-v0.md:20 明确「等价不成立」。
//!   这是 `[缠论可导,62课；严格等号处理设计选择]`——严格性是规则核心，不可松为非严格。
//! - 顶分型要求中 K high>左右 high **且** low>左右 low（两个极值都严格高）；底镜像。
//! - 包含处理后相邻 K 两两非包含（inclusion 不动点），故中 K 与左右的 high/low 严格序
//!   关系良定义（不会同时 high 高而 low 低）。
//! - 分型的 `source_index`/`timestamp`/极值价取**中 K**（分型顶点所在 K）。

use super::super::types::{Fractal, FractalKind};
use super::super::types::Bar;

/// 在包含处理后的 merged K 序列上识别分型（reference-theta-v0.md:20）。
///
/// 扫描每个内部三元组 `(left, mid, right)`：
/// - 顶分型：`mid.high > left.high && mid.high > right.high && mid.low > left.low
///   && mid.low > right.low`（严格，等价不成立）。
/// - 底分型：四个严格 `<` 镜像。
///
/// 第三根 K 确认：分型在 `right`（第三根）出现时确认，故输出顺序按 mid 的位置升序。
///
/// 边界条件：
/// - `merged.len() < 3` ⟹ 无分型（无完整三元组）。
/// - 中 K 既非严格全高也非严格全低 ⟹ 非分型（跳过；含包含处理残留的非严格情形）。
pub fn detect_fractals(merged: &[Bar]) -> Vec<Fractal> {
    let mut out = Vec::new();
    if merged.len() < 3 {
        return out;
    }
    for i in 1..merged.len() - 1 {
        let left = &merged[i - 1];
        let mid = &merged[i];
        let right = &merged[i + 1];

        let is_top = mid.high > left.high
            && mid.high > right.high
            && mid.low > left.low
            && mid.low > right.low;
        let is_bottom = mid.high < left.high
            && mid.high < right.high
            && mid.low < left.low
            && mid.low < right.low;

        if is_top {
            out.push(Fractal {
                kind: FractalKind::Top,
                source_index: mid.source_index,
                timestamp: mid.timestamp,
                price: mid.high,
            });
        } else if is_bottom {
            out.push(Fractal {
                kind: FractalKind::Bottom,
                source_index: mid.source_index,
                timestamp: mid.timestamp,
                price: mid.low,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::Tick;

    fn bar(i: usize, high: Tick, low: Tick) -> Bar {
        Bar {
            source_index: i,
            timestamp: i as i64,
            open: low,
            high,
            low,
            close: high,
            volume: 1,
            untradable: false,
        }
    }

    #[test]
    fn detects_top_fractal_strict() {
        // 中 K [15,12] 严格高于左 [10,5] 右 [11,6] 的 high 和 low → 顶分型。
        let merged = vec![bar(0, 10, 5), bar(1, 15, 12), bar(2, 11, 6)];
        let f = detect_fractals(&merged);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, FractalKind::Top);
        assert_eq!(f[0].source_index, 1);
        assert_eq!(f[0].price, 15);
    }

    #[test]
    fn detects_bottom_fractal_strict() {
        // 中 K [6,2] 严格低于左 [12,8] 右 [11,7] → 底分型，price=low=2。
        let merged = vec![bar(0, 12, 8), bar(1, 6, 2), bar(2, 11, 7)];
        let f = detect_fractals(&merged);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, FractalKind::Bottom);
        assert_eq!(f[0].price, 2);
    }

    #[test]
    fn equality_does_not_form_fractal() {
        // 中 K high 等于右 K high（非严格）→ 不成立（reference-theta-v0.md:20 等价不成立）。
        let merged = vec![bar(0, 10, 5), bar(1, 15, 12), bar(2, 15, 6)];
        let f = detect_fractals(&merged);
        assert!(f.is_empty(), "非严格高低不应形成分型");
    }

    #[test]
    fn fewer_than_three_bars_no_fractal() {
        assert!(detect_fractals(&[bar(0, 10, 5), bar(1, 15, 12)]).is_empty());
        assert!(detect_fractals(&[]).is_empty());
    }

    #[test]
    fn golden_alternating_top_then_bottom() {
        // 五根：底→顶→底交替（每个内部三元组各产一个分型）。
        // [10,5] [15,12] [11,6] [4,1] [8,3]
        // i=1: mid[15,12] > 左[10,5] 右[11,6] → 顶
        // i=2: mid[11,6]：左[15,12] 右[4,1]，high 11<15 非全高；11>4 但 low 6>1，非全低 → 非分型
        // i=3: mid[4,1] < 左[11,6] 右[8,3] → 底
        let merged = vec![
            bar(0, 10, 5),
            bar(1, 15, 12),
            bar(2, 11, 6),
            bar(3, 4, 1),
            bar(4, 8, 3),
        ];
        let f = detect_fractals(&merged);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kind, FractalKind::Top);
        assert_eq!(f[1].kind, FractalKind::Bottom);
    }

    /// property：分型的极值价就是中 K 的对应极值（顶=high，底=low）。
    #[test]
    fn property_fractal_price_is_mid_extreme() {
        let merged = vec![bar(0, 10, 5), bar(1, 15, 12), bar(2, 11, 6)];
        let f = detect_fractals(&merged);
        assert_eq!(f[0].price, merged[1].high); // 顶 → 中 K high
    }
}
