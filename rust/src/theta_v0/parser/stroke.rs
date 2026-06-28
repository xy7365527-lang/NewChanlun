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

// ============================================================================
// 增量 stroke（#93 H1 主根因：parse_layer 下游 per-bar O(n²) → 增量化）。
//
// ## 前缀不变性（bit-exact 基础）
//
// collapse_consecutive 是左折叠：out.push(f) 后，前缀 out[0..len-1] 不可变，仅 out.last()
// 可能被同类连续修改。贪心配对在交替序列上扫描：一旦某笔成笔（i+=1），其端点 fractal 固定。
//
// 增量策略：
// 1. 增量 collapse：保留前缀交替序列（除末 1 个可能被修改），重算尾部 collapse。
// 2. 增量配对：保留 confirmed strokes 前缀（端点 fractal 已固定），从最后一笔笔尾在
//    新交替序列中的位置续扫。
//
// 严格性：存 (Fractal, fractal_idx) 对——fractal_idx 是在原始 fractals 输入中的下标，
// 用于精确过滤前缀。不依赖 source_index 代理。
// ============================================================================

/// 增量 stroke 状态（bit-exact 对齐 `build_strokes`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrStrokes {
    /// confirmed 交替序列前缀 `(Fractal, fractal_idx)`（除末 1 个可能被 collapse 修改）。
    alt_prefix: Vec<(Fractal, usize)>,
    /// confirmed strokes 前缀（端点 fractal 已固定，不可变）。
    strokes: Vec<Stroke>,
    /// 上次快照时的 fractals 长度。
    fractals_len: usize,
}

impl Default for IncrStrokes {
    fn default() -> Self {
        IncrStrokes {
            alt_prefix: Vec::new(),
            strokes: Vec::new(),
            fractals_len: 0,
        }
    }
}

impl IncrStrokes {
    /// 空状态。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 从全量结果恢复增量状态（断点续算，一次性 O(n) 重建 alt_prefix 的 fractal_idx）。
    pub fn from_full(fractals: &[Fractal], strokes: &[Stroke]) -> Self {
        // 重建 (Fractal, fractal_idx)：重跑 collapse 逻辑记录 fractal_idx。
        let mut alt_prefix: Vec<(Fractal, usize)> = Vec::with_capacity(fractals.len());
        for (fi, &f) in fractals.iter().enumerate() {
            match alt_prefix.last() {
                Some(prev) if prev.0.kind == f.kind => {
                    let keep_new = match f.kind {
                        FractalKind::Top => {
                            f.price > prev.0.price
                                || (f.price == prev.0.price && f.source_index < prev.0.source_index)
                        }
                        FractalKind::Bottom => {
                            f.price < prev.0.price
                                || (f.price == prev.0.price && f.source_index < prev.0.source_index)
                        }
                    };
                    if keep_new {
                        *alt_prefix.last_mut().unwrap() = (f, fi);
                    }
                }
                _ => alt_prefix.push((f, fi)),
            }
        }
        IncrStrokes {
            alt_prefix,
            strokes: strokes.to_vec(),
            fractals_len: fractals.len(),
        }
    }

    /// 增量追加 fractals 序列，返回新状态（immutability）。
    ///
    /// bit-exact：结果笔序列 == `build_strokes(fractals, config)`。
    ///
    /// 保留 alt_prefix 中 fractal_idx < confirmed_bound 的（除末 1 个可能被 collapse 修改），
    /// 重算尾部 collapse + 保留 confirmed strokes 前缀并续扫配对。
    /// ponytail: collapse 尾部局部重算 + 配对续扫（O(尾部)/bar，非 O(n)）。
    pub fn append(&self, fractals: &[Fractal], config: &ParseConfig) -> IncrStrokes {
        let old_len = self.fractals_len;
        // collapse 末元素可能被新 fractal 修改（同类连续），故保留 fractal_idx < old_len-1 的，
        // 从 old_len-1 起重算（含重叠 1 个保边界）。
        let confirmed_bound = old_len.saturating_sub(1);

        // 保留 alt_prefix 中 fractal_idx < confirmed_bound 的。
        let mut new_alt: Vec<(Fractal, usize)> = self
            .alt_prefix
            .iter()
            .copied()
            .take_while(|(_, fi)| *fi < confirmed_bound)
            .collect();

        // 重算 collapse：从 fractal_idx = confirmed_bound 起扫描到末尾。
        for fi in confirmed_bound..fractals.len() {
            let f = fractals[fi];
            match new_alt.last() {
                Some(prev) if prev.0.kind == f.kind => {
                    let keep_new = match f.kind {
                        FractalKind::Top => {
                            f.price > prev.0.price
                                || (f.price == prev.0.price && f.source_index < prev.0.source_index)
                        }
                        FractalKind::Bottom => {
                            f.price < prev.0.price
                                || (f.price == prev.0.price && f.source_index < prev.0.source_index)
                        }
                    };
                    if keep_new {
                        *new_alt.last_mut().unwrap() = (f, fi);
                    }
                }
                _ => new_alt.push((f, fi)),
            }
        }

        // 增量配对：保留 confirmed strokes 前缀（笔尾 fractal_idx < confirmed_bound 的），
        // 从最后一笔笔尾在 new_alt 中的位置续扫。
        let mut new_strokes: Vec<Stroke> = Vec::new();
        let mut resume_alt_idx = 0usize;
        for s in &self.strokes {
            // 找该笔笔尾在 new_alt（fractal_idx < confirmed_bound 段）中的位置。
            let mut found_end_alt = None;
            for (ai, (f, fi)) in new_alt.iter().enumerate() {
                if *fi < confirmed_bound && f.source_index == s.end_index {
                    found_end_alt = Some(ai);
                    break;
                }
            }
            match found_end_alt {
                Some(ai) => {
                    new_strokes.push(*s);
                    resume_alt_idx = ai; // 下一笔从 ai+1 起扫（i+=1 后位置）
                }
                None => break, // 笔尾不在 confirmed 段——此笔及之后重扫
            }
        }

        // 从 resume_alt_idx 续扫配对（镜像 build_strokes 贪心逻辑）。
        let mut i = resume_alt_idx;
        while i + 1 < new_alt.len() {
            let a = &new_alt[i].0;
            let b = &new_alt[i + 1].0;
            if gap_ok(a, b, config.new_stroke_min_gap) {
                let direction = match a.kind {
                    FractalKind::Bottom => Direction::Up,
                    FractalKind::Top => Direction::Down,
                };
                new_strokes.push(Stroke {
                    direction,
                    start_index: a.source_index,
                    end_index: b.source_index,
                    start_price: a.price,
                    end_price: b.price,
                });
                i += 1;
            } else {
                i += 2;
            }
        }

        IncrStrokes {
            alt_prefix: new_alt,
            strokes: new_strokes,
            fractals_len: fractals.len(),
        }
    }

    /// 当前快照笔序列（与 `build_strokes` bit-exact）。
    pub fn to_result(&self) -> &[Stroke] {
        &self.strokes
    }
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

    /// ★bit-exact：增量 IncrStrokes::append 链 == 全量 build_strokes（逐 fractals 长度）。
    #[test]
    fn bit_exact_incr_strokes_per_bar() {
        // 合成分型序列：顶底交替 + 同类连续（触发 collapse）+ 间隔不足（触发 skip）。
        let fractals: Vec<Fractal> = (0..150usize)
            .flat_map(|i| {
                let base = i * 10;
                vec![
                    frac(FractalKind::Bottom, base, 100 + i as i64),
                    frac(FractalKind::Top, base + 4, 200 + i as i64),
                    frac(FractalKind::Top, base + 6, 210 + i as i64), // 同类连续（测 collapse）
                    frac(FractalKind::Bottom, base + 8, 90 + i as i64),
                ]
            })
            .collect();

        let cfg = cfg(3);
        let mut incr = IncrStrokes::empty();
        for end in 1..=fractals.len() {
            incr = incr.append(&fractals[..end], &cfg);
            let full = build_strokes(&fractals[..end], &cfg);
            assert_eq!(
                incr.to_result(),
                full.as_slice(),
                "fractals len {end}: 增量 stroke != 全量（bit-exact 破裂）"
            );
        }
    }
}
