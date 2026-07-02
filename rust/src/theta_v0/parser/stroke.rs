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
use std::rc::Rc;

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
//
// ## O(n²) 修复（a5a1bb70 子步隔离坐实 IncrStrokes::append 为 incr_total exp 2.9 真主导）
//
// ae0118c0 增量实现有两处 O(n²)/bar：
// 1. `take_while(|(_, fi)| *fi < confirmed_bound)` 遍历整个 alt_prefix O(n)/bar。
// 2. 嵌套循环 `for s in &self.strokes { for (ai,..) in new_alt.iter() }` O(strokes×alt)/bar。
//
// 修复：维护 confirmed_alt_len / confirmed_strokes_len / last_end_alt_idx 三个索引，
// append 入口 O(1) slice + O(1) 继承 resume_alt_idx，消除两处 O(n²)。出口 O(1) 更新
// confirmed 索引（new_alt.last() fi == new_confirmed_bound 判定 + 续扫尾端 alt 索引追踪）。
// ============================================================================

/// 计算 confirmed strokes 长度 + 最后一个 confirmed stroke 笔尾的 alt 索引。
///
/// confirmed strokes = 端点 source_index 对应的 alt 条目在 `alt_prefix[..confirmed_alt_len]` 内的笔。
/// alt 按 source_index 单调递增，strokes 按 end_index 单调递增 → partition_point O(log n)。
/// 返回 `(confirmed_strokes_len, last_end_alt_idx)`，无 confirmed 笔时 last_end_alt_idx = 0。
fn compute_confirmed_strokes(
    strokes: &[Stroke],
    alt_prefix: &[(Fractal, usize)],
    confirmed_alt_len: usize,
) -> (usize, usize) {
    if confirmed_alt_len == 0 || strokes.is_empty() {
        return (0, 0);
    }
    // confirmed 段最后一个 source_index（边界）。
    let boundary_si = alt_prefix[confirmed_alt_len - 1].0.source_index;
    // end_index <= boundary_si 的笔为 confirmed（端点在 confirmed alt 段内）。
    let confirmed_strokes_len = strokes
        .partition_point(|s| s.end_index <= boundary_si);
    let last_end_alt_idx = if confirmed_strokes_len == 0 {
        0
    } else {
        // 最后一个 confirmed stroke 笔尾在 alt_prefix 中的位置。
        let end_si = strokes[confirmed_strokes_len - 1].end_index;
        alt_prefix.partition_point(|(f, _)| f.source_index < end_si)
    };
    (confirmed_strokes_len, last_end_alt_idx)
}


/// 增量 stroke 状态（bit-exact 对齐 `build_strokes`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrStrokes {
    /// confirmed 交替序列前缀 `(Fractal, fractal_idx)`（除末 1 个可能被 collapse 修改）。
    alt_prefix: Vec<(Fractal, usize)>,
    /// confirmed strokes 前缀（端点 fractal 已固定，不可变）。
    /// Rc 共享——`to_result_rc()` O(1) clone 给 `ParseLayer.strokes`。
    strokes: Rc<Vec<Stroke>>,
    /// 上次快照时的 fractals 长度。
    fractals_len: usize,
    /// alt_prefix 中 `fi < fractals_len-1` 的条目数（confirmed alt 前缀长度）。
    // ponytail: O(1) slice 替代 O(n) take_while（hotspot 1，ae0118c0 增量 bug）。
    confirmed_alt_len: usize,
    /// strokes 中端点 `fi < fractals_len-1` 的笔数（confirmed strokes 前缀长度）。
    // ponytail: O(1) slice 替代 O(n²) 嵌套扫描（hotspot 2，ae0118c0 增量 bug）。
    confirmed_strokes_len: usize,
    /// 最后一个 confirmed stroke 笔尾在 alt_prefix 中的索引（续扫起点）。
    // ponytail: O(1) 继承替代 O(n²) 逐笔重扫 new_alt（hotspot 2，ae0118c0 增量 bug）。
    last_end_alt_idx: usize,
    /// 上次 append 见到的末分型（相同输入早退用）。confirmed 前缀不可变，仅末分型可被 collapse
    /// 改写 ⟹ (fractals_len, 末分型) 相同蕴含整个 fractals 相同 ⟹ 结果与 self 逐字段等。
    last_fractal: Option<Fractal>,
}

impl Default for IncrStrokes {
    fn default() -> Self {
        IncrStrokes {
            alt_prefix: Vec::new(),
            strokes: Rc::new(Vec::new()),
            fractals_len: 0,
            confirmed_alt_len: 0,
            confirmed_strokes_len: 0,
            last_end_alt_idx: 0,
            last_fractal: None,
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
        // 重建 confirmed 索引（O(n) 一次性，断点续算非热路径）。
        let confirmed_bound = fractals.len().saturating_sub(1);
        let confirmed_alt_len = alt_prefix
            .partition_point(|(_, fi)| *fi < confirmed_bound);
        // confirmed strokes：端点 source_index < alt_prefix[confirmed_alt_len] 的 source_index
        //（alt 按 source_index 单调递增，strokes 按 end_index 单调递增）。
        let (confirmed_strokes_len, last_end_alt_idx) = compute_confirmed_strokes(
            strokes,
            &alt_prefix,
            confirmed_alt_len,
        );
        IncrStrokes {
            alt_prefix,
            strokes: Rc::new(strokes.to_vec()),
            fractals_len: fractals.len(),
            confirmed_alt_len,
            confirmed_strokes_len,
            last_end_alt_idx,
            last_fractal: fractals.last().copied(),
        }
    }

    /// 增量追加 fractals 序列，返回新状态（by-value 缓冲复用，消除 O(n)/bar clone）。
    ///
    /// bit-exact：结果笔序列 == `build_strokes(fractals, config)`。
    ///
    /// 保留 alt_prefix 中 fractal_idx < confirmed_bound 的（除末 1 个可能被 collapse 修改），
    /// 重算尾部 collapse + 保留 confirmed strokes 前缀并续扫配对。
    /// ponytail: collapse 尾部局部重算 + 配对续扫（O(尾部)/bar，非 O(n)）。
    pub fn append(self, fractals: &[Fractal], config: &ParseConfig) -> IncrStrokes {
        // 相同输入早退：fractals 长度同 ∧ 末分型逐字段等 ⟹ 结果与 self 逐字段等（confirmed 前缀
        // 不可变，仅末分型可被 collapse 改写，故 (len,末分型) 相同蕴含整个 fractals 相同）。
        if fractals.len() == self.fractals_len && fractals.last().copied() == self.last_fractal {
            return self;
        }
        let old_len = self.fractals_len;
        // collapse 末元素可能被新 fractal 修改（同类连续），故保留 fractal_idx < old_len-1 的，
        // 从 old_len-1 起重算（含重叠 1 个保边界）。
        let confirmed_bound = old_len.saturating_sub(1);

        // 保留 alt_prefix 中 fractal_idx < confirmed_bound 的。
        // ponytail: truncate 复用 Vec 缓冲（O(尾部) drop），替代 O(n) take_while + to_vec
        //（hotspot 1，a5a1bb70 坐实 exp 2.9 主导）。前缀不变性：alt_prefix[..confirmed_alt_len]
        // 的 fi 均 < confirmed_bound（由上次 append 保证）。
        let mut new_alt: Vec<(Fractal, usize)> = self.alt_prefix;
        new_alt.truncate(self.confirmed_alt_len);

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
        // ponytail: truncate 复用 Vec 缓冲 + O(1) 继承 resume_alt_idx，替代 O(n²) 嵌套扫描
        //（hotspot 2，a5a1bb70 坐实 exp 2.9 主导）。confirmed_strokes_len 笔的笔尾 fi < confirmed_bound，
        // 且其 alt 索引 = last_end_alt_idx（由上次 append 保证，new_alt 前缀不变）。
        // Rc::make_mut——strong_count==1（self 消费，旧 ParseLayer 已 drop）时 O(1) in-place。
        let mut strokes_rc = self.strokes;
        let new_strokes = Rc::make_mut(&mut strokes_rc);
        new_strokes.truncate(self.confirmed_strokes_len);
        let resume_alt_idx = self.last_end_alt_idx;

        // 从 resume_alt_idx 续扫配对（镜像 build_strokes 贪心逻辑）。
        // 追踪续扫产出的每笔笔尾 alt 索引，用于最后 O(1) 更新 confirmed 索引。
        let mut scan_end_alt_indices: Vec<usize> = Vec::new();
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
                i += 1; // 笔尾 b 成为下一笔笔头。
                scan_end_alt_indices.push(i); // 笔尾 alt 索引 = i（i+=1 后）。
            } else {
                i += 2;
            }
        }

        // 更新 confirmed 索引（O(1) amortized）。
        // 新 confirmed_bound = fractals.len()-1。new_alt 最后一个 fi 要么 = 新 bound（最后分型存活）
        // 要么 < 新 bound（被 collapse 吞）。confirmed_alt_len = 前者 ? len-1 : len。
        let new_confirmed_bound = fractals.len().saturating_sub(1);
        let new_confirmed_alt_len = match new_alt.last() {
            Some((_, fi)) if *fi == new_confirmed_bound => new_alt.len() - 1,
            _ => new_alt.len(),
        };
        // confirmed strokes = confirmed 前缀 + 续扫中 fi < new_confirmed_bound 的笔。
        // confirmed 前缀笔数 = self.confirmed_strokes_len（继承）。
        // 续扫笔的笔尾 alt 索引 < new_confirmed_alt_len ⟺ 笔尾 fi < new_confirmed_bound。
        let mut new_confirmed_strokes_len = self.confirmed_strokes_len;
        let mut new_last_end_alt_idx = self.last_end_alt_idx;
        for &end_ai in &scan_end_alt_indices {
            if end_ai < new_confirmed_alt_len {
                new_confirmed_strokes_len += 1;
                new_last_end_alt_idx = end_ai;
            } else {
                break; // alt 按序，首个未确认后续全未确认。
            }
        }

        IncrStrokes {
            alt_prefix: new_alt,
            strokes: strokes_rc,
            fractals_len: fractals.len(),
            confirmed_alt_len: new_confirmed_alt_len,
            confirmed_strokes_len: new_confirmed_strokes_len,
            last_end_alt_idx: new_last_end_alt_idx,
            last_fractal: fractals.last().copied(),
        }
    }

    /// 当前快照笔序列（与 `build_strokes` bit-exact）。
    pub fn to_result(&self) -> &[Stroke] {
        &self.strokes
    }

    /// 当前快照笔序列的 Rc 共享句柄（O(1) refcount bump）。
    ///
    /// ponytail: Rc 共享替代旧 `to_result().to_vec()` 的 O(n) clone——`ParseLayerIncr::append`
    /// 用此填 `ParseLayer.strokes`，消除每 bar Vec clone。
    pub fn to_result_rc(&self) -> Rc<Vec<Stroke>> {
        Rc::clone(&self.strokes)
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

    /// property：重复 append 同一输入幂等（相同输入早退路径）——第二次 append 同 fractals ⟹
    /// strokes 与第一次逐字段等，且仍与全量 build_strokes bit-exact。
    #[test]
    fn idempotent_repeat_append_same_input() {
        let fractals: Vec<Fractal> = (0..150usize)
            .flat_map(|i| {
                let base = i * 10;
                vec![
                    frac(FractalKind::Bottom, base, 100 + i as i64),
                    frac(FractalKind::Top, base + 4, 200 + i as i64),
                    frac(FractalKind::Top, base + 6, 210 + i as i64),
                    frac(FractalKind::Bottom, base + 8, 90 + i as i64),
                ]
            })
            .collect();

        let cfg = cfg(3);
        let mut incr = IncrStrokes::empty();
        for end in 1..=fractals.len() {
            incr = incr.append(&fractals[..end], &cfg);
            let first = incr.to_result().to_vec();
            // 第二次 append 同输入——走相同输入早退，返回 self 不变。
            incr = incr.append(&fractals[..end], &cfg);
            assert_eq!(
                incr.to_result(),
                first.as_slice(),
                "fractals len {end}: 重复 append 同输入改变了结果（幂等破裂）"
            );
            let full = build_strokes(&fractals[..end], &cfg);
            assert_eq!(incr.to_result(), full.as_slice());
        }
    }
}
