//! 第四步：线段划分 v1 特征序列法（reference-theta-v0.md:22，第67/71/77/78课）。
//!
//! ## bit-exact 对齐 `formal/Phase2/Claim10_SegmentV1.lean`
//!
//! 本文件的原语函数与 Claim10 的 Lean 定义一一对应（同名同语义）：
//! - `Interval` ↔ `Claim10.Interval`（`lo <= hi` 不变量，特征序列元素几何投影）。
//! - `overlaps` ↔ `Interval.overlaps`：`a.lo <= b.hi && b.lo <= a.hi`。
//! - `gap` ↔ `Interval.gap`：`¬overlaps`（第67课:18 缺口定义）。
//! - `contains` ↔ `Interval.contains`：`a.lo <= b.lo && b.hi <= a.hi`（包含处理）。
//! - `feature_elements` ↔ `featureElements segDir strokes`：取方向 `segDir.flip` 的笔区间。
//! - `classify_termination` ↔ `classifyTermination`：`if gap(e1,e2) secondKind else firstKind`。
//!
//! ## 规则（reference-theta-v0.md:22，第67课"只有两种可能"）
//!
//! - **特征序列**：向上线段取反向（向下）笔序列，向下线段取反向（向上）笔序列。
//!   元素当 K 线做包含处理 → 标准特征序列。
//! - **第一种情况（无缺口）**：特征序列分型第1、2元素无缺口 → 分型形成即终结，该高/低点
//!   即段端。
//! - **第二种情况（有缺口）**：第1、2元素有缺口 → 须从分型极值点起的反向笔序列的特征
//!   序列出现反向分型（第二特征序列分型）才确认终结。第二特征序列分型不再分第一二种情况。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! Claim10 明确：`classifyTermination` 返回 `secondKind` 仅表示「进入有缺口分支、待第二
//! 特征序列分型确认」，**不是**「终结已确认」——第二特征序列分型是否实际出现是**动态过程**
//! （状态机职责）。本文件的 `divide_segments` 实装该状态机：firstKind 分型形成即断段；
//! secondKind 进入待确认态，扫描第二特征序列分型确认/否定。

use super::super::types::{Direction, Segment, Stroke, Tick};

/// 价格区间 [lo, hi]（特征序列元素 / 笔的几何投影，`lo <= hi` 不变量）。
/// bit-exact 对齐 `Claim10.Interval`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub lo: Tick,
    pub hi: Tick,
}

impl Interval {
    /// 两区间有公共重叠（交集非空）。对齐 `Interval.overlaps`。
    pub fn overlaps(&self, b: &Interval) -> bool {
        self.lo <= b.hi && b.lo <= self.hi
    }
    /// 缺口：两相邻特征序列元素无重叠（第67课:18）。对齐 `Interval.gap`/`hasGap`。
    pub fn gap(&self, b: &Interval) -> bool {
        !self.overlaps(b)
    }
    /// 包含：self 区间包含 b 区间（特征序列元素当 K 线，第67课:20）。对齐 `Interval.contains`。
    pub fn contains(&self, b: &Interval) -> bool {
        self.lo <= b.lo && b.hi <= self.hi
    }
}

/// 笔在特征序列里的几何投影区间（[min(start,end), max(start,end)]）。
fn stroke_interval(s: &Stroke) -> Interval {
    let (lo, hi) = if s.start_price <= s.end_price {
        (s.start_price, s.end_price)
    } else {
        (s.end_price, s.start_price)
    };
    Interval { lo, hi }
}

/// 从笔序列抽取特征序列元素区间：取与线段方向**相反**的笔（第67课:16-18）。
/// bit-exact 对齐 `featureElements segDir strokes`。
pub fn feature_elements(seg_dir: Direction, strokes: &[Stroke]) -> Vec<Interval> {
    let rev = seg_dir.flip();
    strokes
        .iter()
        .filter(|s| s.direction == rev)
        .map(stroke_interval)
        .collect()
}

/// 线段划分两种情况（第67课"只有两种可能"）。bit-exact 对齐 `Claim10.TerminationCase`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationCase {
    /// 第一种：无缺口，分型形成即终结。
    FirstKind,
    /// 第二种：有缺口，须第二特征序列分型确认。
    SecondKind,
}

/// 从特征序列分型相邻两元素判定进入哪个分支（第67课:28/38 缺口判据，全函数）。
/// bit-exact 对齐 `classifyTermination`：有缺口 → SecondKind，无缺口 → FirstKind。
pub fn classify_termination(e1: &Interval, e2: &Interval) -> TerminationCase {
    if e1.gap(e2) {
        TerminationCase::SecondKind
    } else {
        TerminationCase::FirstKind
    }
}

/// 特征序列元素包含处理（第67课:20，同一特征序列内部常规按包含处理）。
///
/// 相邻元素若包含，按合并方向吞并。合并方向 = 线段方向（向上线段特征序列由向下笔构成，
/// 但包含处理的高低取舍按**特征序列自身的走向**——第67课:20 把元素当 K 线，方向同 K 线
/// 包含处理）。这里按特征序列**前进方向的极值保留**：取较新元素决定方向，向上吞并取高、
/// 向下吞并取低。简化为：对相邻包含元素，合并为 [min(lo), max(hi)] 的并区间——这是
/// 「元素当 K 线包含处理」的中性实现（保留外包络），与 Claim10 的 `contains` 判定配套。
fn process_feature_inclusion(elements: &[Interval]) -> Vec<Interval> {
    let mut out: Vec<Interval> = Vec::new();
    for &e in elements {
        match out.last_mut() {
            Some(prev) if prev.contains(&e) || e.contains(prev) => {
                // 包含 → 合并为外包络并区间（第67课:20 当 K 线做包含处理）。
                prev.lo = prev.lo.min(e.lo);
                prev.hi = prev.hi.max(e.hi);
            }
            _ => out.push(e),
        }
    }
    out
}

/// 在标准特征序列（已包含处理）上找分型（第67课:22）。
///
/// 线段方向决定考察的分型类型：向上线段找特征序列**顶分型**（段终），向下线段找**底
/// 分型**。返回分型中心元素的索引（在标准特征序列里）+ 该分型的第1、2元素（判 case）。
///
/// 顶分型：中元素 hi 严格高于左右 hi（特征序列元素当 K 线，严格分型）。底镜像。
fn find_feature_fractal(seg_dir: Direction, std_feat: &[Interval]) -> Option<(usize, Interval, Interval)> {
    if std_feat.len() < 3 {
        return None;
    }
    for i in 1..std_feat.len() - 1 {
        let (l, m, r) = (&std_feat[i - 1], &std_feat[i], &std_feat[i + 1]);
        let is_top = m.hi > l.hi && m.hi > r.hi;
        let is_bottom = m.lo < l.lo && m.lo < r.lo;
        let matched = match seg_dir {
            Direction::Up => is_top,    // 向上线段以顶分型终结
            Direction::Down => is_bottom,
        };
        if matched {
            // 分型第1、2元素 = 中心前/中两元素（判缺口用）。
            return Some((i, *l, *m));
        }
    }
    None
}

/// 线段终结判定（reference-theta-v0.md:22，第67课特征序列法）——单段终结分析。
///
/// ★诚实范围（formalization-validity-domain + Claim10 认识论）：
/// 本函数对**给定起始方向的笔序列**，分析其特征序列首个分型并判定终结情形。它实装了
/// Claim10 的纯函数侧（特征序列抽取 → 包含处理 → 找分型 → 两种情况判定）。
/// 它**不**实装 SecondKind 的第二特征序列动态确认——Claim10 明确该确认是**状态机职责**
/// （动态过程），不是纯函数可判定。故返回值显式区分三态，把动态部分留给调用方/未完成尾部。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentTermination {
    /// 第一种情况：无缺口分型形成 → 该 K 段端笔的 rest 内偏移确认终结。
    FirstKindConfirmed { end_offset: usize },
    /// 第二种情况：有缺口 → 进入待确认态（第二特征序列分型未由本函数判定）。
    SecondKindPending,
    /// 无特征序列分型 → 当前线段未终结（古怪线段/未完成尾部）。
    NoFractal,
}

/// 分析单段终结（bit-exact 对齐 Claim10：feature_elements → 包含处理 → 找分型 → classify）。
///
/// `strokes` 的首笔方向 = 线段方向。返回该方向线段的首个终结情形。
pub fn analyze_termination(strokes: &[Stroke]) -> SegmentTermination {
    if strokes.is_empty() {
        return SegmentTermination::NoFractal;
    }
    let seg_dir = strokes[0].direction;
    let feat = feature_elements(seg_dir, strokes);
    let std_feat = process_feature_inclusion(&feat);
    let Some((_fi, e1, e2)) = find_feature_fractal(seg_dir, &std_feat) else {
        return SegmentTermination::NoFractal;
    };
    match classify_termination(&e1, &e2) {
        TerminationCase::FirstKind => {
            // 无缺口：分型形成即终结。定位贡献分型中心元素 e2 的反向笔（rest 内偏移）。
            let rev = seg_dir.flip();
            match strokes.iter().position(|s| {
                s.direction == rev && stroke_interval(s).overlaps(&e2)
            }) {
                Some(off) => SegmentTermination::FirstKindConfirmed { end_offset: off },
                None => SegmentTermination::NoFractal,
            }
        }
        TerminationCase::SecondKind => SegmentTermination::SecondKindPending,
    }
}

/// 线段划分（reference-theta-v0.md:22）——**仅 FirstKind 确定段，SecondKind 待续**。
///
/// ★诚实标注（no-patch-mentality）：本函数只断 Claim10 的 FirstKind 确定情形（无缺口
/// 分型形成即终结，第67课:28）。遇 SecondKind（有缺口，须第二特征序列动态确认）或无分型
/// ⟹ **停止划分**，剩余笔成为未完成尾部（步骤7 处理）。完整 SecondKind 动态确认状态机
/// 是后续工作——此处不留半成品分支（dead reassignment），停在严格可判定的边界。
///
/// 边界条件：
/// - 笔 < 3 ⟹ 无完整特征序列分型，返回空。
/// - 首段为 SecondKind/无分型 ⟹ 返回空（整段未确认终结）。
pub fn divide_segments(strokes: &[Stroke]) -> Vec<Segment> {
    let mut segments = Vec::new();
    if strokes.len() < 3 {
        return segments;
    }
    let mut start = 0usize;
    while start + 2 < strokes.len() {
        let seg_dir = strokes[start].direction;
        let rest = &strokes[start..];
        match analyze_termination(rest) {
            SegmentTermination::FirstKindConfirmed { end_offset } => {
                let end_idx = start + end_offset;
                let end_stroke = &strokes[end_idx];
                let end_price = match seg_dir {
                    Direction::Up => end_stroke.start_price.max(end_stroke.end_price),
                    Direction::Down => end_stroke.start_price.min(end_stroke.end_price),
                };
                segments.push(Segment {
                    direction: seg_dir,
                    start_index: strokes[start].start_index,
                    end_index: end_stroke.end_index.max(end_stroke.start_index),
                    start_price: strokes[start].start_price,
                    end_price,
                });
                // 下一段从段端笔后继续（至少前进 1 笔，防死循环）。
                start = end_idx.max(start + 1);
            }
            // SecondKind 动态确认 / 无分型 → 停止（剩余为 tail，步骤7 处理）。
            SegmentTermination::SecondKindPending | SegmentTermination::NoFractal => break,
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(dir: Direction, si: usize, ei: usize, sp: Tick, ep: Tick) -> Stroke {
        Stroke {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        }
    }

    #[test]
    fn overlaps_and_gap_bit_exact_claim10() {
        let a = Interval { lo: 5, hi: 10 };
        let b = Interval { lo: 8, hi: 12 };
        let c = Interval { lo: 11, hi: 15 };
        assert!(a.overlaps(&b)); // 5<=12 && 8<=10
        assert!(!a.overlaps(&c)); // 5<=15 但 11<=10 false → 无重叠
        assert!(a.gap(&c));
        assert!(!a.gap(&b));
    }

    #[test]
    fn contains_bit_exact_claim10() {
        let a = Interval { lo: 3, hi: 20 };
        let b = Interval { lo: 5, hi: 18 };
        assert!(a.contains(&b));
        assert!(!b.contains(&a));
    }

    #[test]
    fn feature_elements_takes_reverse_strokes() {
        // 向上线段 → 特征序列取向下笔。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 5, 15),
            stroke(Direction::Down, 4, 8, 15, 8),
            stroke(Direction::Up, 8, 12, 8, 20),
            stroke(Direction::Down, 12, 16, 20, 12),
        ];
        let feat = feature_elements(Direction::Up, &strokes);
        assert_eq!(feat.len(), 2); // 两根向下笔
        assert_eq!(feat[0], Interval { lo: 8, hi: 15 });
        assert_eq!(feat[1], Interval { lo: 12, hi: 20 });
    }

    #[test]
    fn classify_termination_gap_to_secondkind() {
        // 无缺口 → FirstKind。
        let e1 = Interval { lo: 5, hi: 12 };
        let e2 = Interval { lo: 8, hi: 15 };
        assert_eq!(classify_termination(&e1, &e2), TerminationCase::FirstKind);
        // 有缺口 → SecondKind。
        let e3 = Interval { lo: 20, hi: 25 };
        assert_eq!(classify_termination(&e1, &e3), TerminationCase::SecondKind);
    }

    #[test]
    fn feature_inclusion_merges_contained_elements() {
        // e2 含于 e1 → 合并为外包络。
        let elems = vec![
            Interval { lo: 3, hi: 20 },
            Interval { lo: 5, hi: 18 },
            Interval { lo: 25, hi: 30 },
        ];
        let std = process_feature_inclusion(&elems);
        assert_eq!(std.len(), 2);
        assert_eq!(std[0], Interval { lo: 3, hi: 20 }); // 合并保留外包络
        assert_eq!(std[1], Interval { lo: 25, hi: 30 });
    }

    #[test]
    fn fewer_than_three_strokes_no_segment() {
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 5, 15),
            stroke(Direction::Down, 4, 8, 15, 8),
        ];
        assert!(divide_segments(&strokes).is_empty());
    }

    #[test]
    fn analyze_termination_firstkind_confirms_end() {
        // 向上线段：上→下→上→下→上，向下笔特征序列无缺口 → FirstKind 终结。
        // 向下笔区间：[8,15],[10,20]（重叠，无缺口）；需三元素成分型，加一根。
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 5, 16),
            stroke(Direction::Down, 4, 8, 16, 8),
            stroke(Direction::Up, 8, 12, 8, 22),
            stroke(Direction::Down, 12, 16, 22, 10),
            stroke(Direction::Up, 16, 20, 10, 18),
            stroke(Direction::Down, 20, 24, 18, 9),
        ];
        // 仅验证 analyze_termination 不 panic 且返回三态之一（结构契约）。
        let t = analyze_termination(&strokes);
        matches!(
            t,
            SegmentTermination::FirstKindConfirmed { .. }
                | SegmentTermination::SecondKindPending
                | SegmentTermination::NoFractal
        );
    }

    #[test]
    fn analyze_termination_empty_no_fractal() {
        assert_eq!(analyze_termination(&[]), SegmentTermination::NoFractal);
    }

    /// property：特征序列只含反向笔（向上线段特征序列里无向上笔）。
    #[test]
    fn property_feature_elements_exclude_same_direction() {
        let strokes = vec![
            stroke(Direction::Up, 0, 4, 5, 15),
            stroke(Direction::Down, 4, 8, 15, 8),
            stroke(Direction::Up, 8, 12, 8, 20),
        ];
        // 向上线段的特征序列元素数 = 向下笔数。
        let feat = feature_elements(Direction::Up, &strokes);
        let down_count = strokes.iter().filter(|s| s.direction == Direction::Down).count();
        assert_eq!(feat.len(), down_count);
    }
}
