//! 第三步：新笔划分（reference-theta-v0.md:21，[缠论可导,77/81课]）。
//!
//! ## 规则（reference-theta-v0.md:21 逐字）
//!
//! - **启用新笔，旧笔禁用**（《忽闻台风可休市》/第81课新笔定义）。
//! - **顶/底分型不共用 K**：相邻笔端点分型不能是同一根 K。
//! - **间隔约束**：两极值 K 间按**原始 K 计数**排除两端至少 N 根（config
//!   `parse.new_stroke_min_gap`，default 3）。即两端点 source_index 之差 > N
//!   （中间至少 N 根独立 K）。
//! - **同类连续分型取舍**：相邻同类分型（连续两顶或两底，中间无异类），
//!   **顶保留更高、底保留更低、等价保留更早**（tie-break 设计选择）。
//!
//! ## bit-exact 注意点
//!
//! - 间隔用**原始 K 序号**（`source_index`），不是合并后序号——「按原始 K 计数」。
//! - 同类连续取舍是**先于**配对的预处理：先把分型序列规整为顶/底严格交替
//!   （同类连续只留极值/更早），再在交替序列上配对成笔。
//! - 「等价保留更早」：同类同价分型保留 `source_index` 更小者（平局裁决
//!   reference-theta-v0.md:16）。

use super::super::config::ParseConfig;
use super::super::types::{Direction, Fractal, FractalKind, Stroke};

/// 同类连续分型取舍 + 顶底交替规整（new 笔预处理）。
///
/// 扫描分型序列，把**连续同类**分型坍缩为一个代表：
/// - 连续顶：保留 price 最高者；等价（同 price）保留 source_index 更小者（更早）。
/// - 连续底：保留 price 最低者；等价保留更早。
/// 结果是顶/底**严格交替**的分型序列（相邻必异类）。
fn collapse_consecutive(fractals: &[Fractal]) -> Vec<Fractal> {
    let mut out: Vec<Fractal> = Vec::new();
    for &f in fractals {
        match out.last() {
            Some(prev) if prev.kind == f.kind => {
                // 同类连续：按 kind 取极值，等价取更早。
                let keep_new = match f.kind {
                    FractalKind::Top => {
                        f.price > prev.price
                            || (f.price == prev.price && f.source_index < prev.source_index)
                    }
                    FractalKind::Bottom => {
                        f.price < prev.price
                            || (f.price == prev.price && f.source_index < prev.source_index)
                    }
                };
                if keep_new {
                    *out.last_mut().unwrap() = f;
                }
                // 否则保留 prev（更极值，或等价时更早），丢弃 f。
            }
            _ => out.push(f),
        }
    }
    out
}

/// 两分型间是否满足新笔间隔约束（reference-theta-v0.md:21）。
///
/// 「两极值 K 间按原始 K 计数排除两端至少 N 根」——两端点 source_index 之差需 > N
/// （即中间至少 N 根独立原始 K，两端不共用 K 自动满足 diff>=1）。
fn gap_ok(a: &Fractal, b: &Fractal, min_gap: u32) -> bool {
    let diff = b.source_index.saturating_sub(a.source_index);
    diff as u32 > min_gap
}

/// 新笔划分主流程（reference-theta-v0.md:21）。
///
/// 1. `collapse_consecutive`：同类连续取舍 → 顶/底严格交替序列。
/// 2. 在交替序列上贪心配对成笔：相邻异类分型满足间隔约束则成笔（方向：底→顶=Up，
///    顶→底=Down），笔尾成为下一笔的笔头；间隔不足则越过该候选对（保持交替）。
///
/// 边界条件：
/// - 分型 < 2 ⟹ 无笔。
/// - 相邻分型不满足间隔 ⟹ 越过该对（不强行成笔，i+=2 回到与 a 异类的下一候选）。
pub fn build_strokes(fractals: &[Fractal], config: &ParseConfig) -> Vec<Stroke> {
    let alt = collapse_consecutive(fractals);
    let mut strokes = Vec::new();
    if alt.len() < 2 {
        return strokes;
    }

    let mut i = 0usize;
    while i + 1 < alt.len() {
        let a = &alt[i];
        let b = &alt[i + 1];
        // collapse 后必异类（严格交替），间隔满足才成笔。
        if gap_ok(a, b, config.new_stroke_min_gap) {
            let direction = match a.kind {
                FractalKind::Bottom => Direction::Up, // 底→顶
                FractalKind::Top => Direction::Down,  // 顶→底
            };
            strokes.push(Stroke {
                direction,
                start_index: a.source_index,
                end_index: b.source_index,
                start_price: a.price,
                end_price: b.price,
            });
            i += 1; // 笔尾 b 成为下一笔笔头。
        } else {
            // 间隔不足：越过 b 与其后同类候选，回到与 a 异类的下一候选（保持交替）。
            i += 2;
        }
    }
    strokes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frac(kind: FractalKind, idx: usize, price: i64) -> Fractal {
        Fractal {
            kind,
            source_index: idx,
            timestamp: idx as i64,
            price,
        }
    }
    fn cfg(min_gap: u32) -> ParseConfig {
        ParseConfig {
            new_stroke_min_gap: min_gap,
            ..ParseConfig::default()
        }
    }

    #[test]
    fn collapse_keeps_higher_top_and_lower_bottom() {
        // 连续两顶 [idx1 p10, idx3 p12] → 保留更高 p12(idx3)。
        let fs = vec![
            frac(FractalKind::Top, 1, 10),
            frac(FractalKind::Top, 3, 12),
        ];
        let c = collapse_consecutive(&fs);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].price, 12);
        assert_eq!(c[0].source_index, 3);
    }

    #[test]
    fn collapse_equal_keeps_earlier() {
        // 连续两顶等价 p10 → 保留更早 idx1。
        let fs = vec![
            frac(FractalKind::Top, 1, 10),
            frac(FractalKind::Top, 5, 10),
        ];
        let c = collapse_consecutive(&fs);
        assert_eq!(c[0].source_index, 1);
    }

    #[test]
    fn gap_constraint_enforced() {
        // min_gap=3：底idx0 → 顶idx4，diff=4>3 → 成笔。
        let fs = vec![
            frac(FractalKind::Bottom, 0, 5),
            frac(FractalKind::Top, 4, 15),
        ];
        let s = build_strokes(&fs, &cfg(3));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].direction, Direction::Up);
        assert_eq!((s[0].start_index, s[0].end_index), (0, 4));
    }

    #[test]
    fn gap_too_small_no_stroke() {
        // 底idx0 → 顶idx3，diff=3，非 >3 → 不成笔。
        let fs = vec![
            frac(FractalKind::Bottom, 0, 5),
            frac(FractalKind::Top, 3, 15),
        ];
        let s = build_strokes(&fs, &cfg(3));
        assert!(s.is_empty());
    }

    #[test]
    fn golden_bottom_top_bottom_two_strokes() {
        // 底idx0 → 顶idx4 → 底idx8：两笔 Up, Down，共用端点（笔尾=下笔笔头）。
        let fs = vec![
            frac(FractalKind::Bottom, 0, 5),
            frac(FractalKind::Top, 4, 15),
            frac(FractalKind::Bottom, 8, 3),
        ];
        let s = build_strokes(&fs, &cfg(3));
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].direction, Direction::Up);
        assert_eq!(s[1].direction, Direction::Down);
        assert_eq!(s[0].end_index, s[1].start_index); // 共用端点
    }

    /// property：相邻笔方向严格交替（new 笔顶底交替的必然结果）。
    #[test]
    fn property_adjacent_strokes_alternate_direction() {
        let fs = vec![
            frac(FractalKind::Bottom, 0, 5),
            frac(FractalKind::Top, 4, 15),
            frac(FractalKind::Bottom, 8, 3),
            frac(FractalKind::Top, 12, 18),
        ];
        let s = build_strokes(&fs, &cfg(3));
        for w in s.windows(2) {
            assert_ne!(w[0].direction, w[1].direction);
        }
    }
}
