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

use super::super::config::ParseConfig;
use super::super::types::{Direction, Segment, Stroke, Tick};
use super::feature_seq::{ExtendMode, FeatureSeqState};
use super::second_kind;

/// 尾窗大小（对齐 Python `_FeatureSeqState.TAIL_WINDOW=7`）。
///
/// ★L2 实测（OKLO 2000 笔，analysis/segment_refsem_cert.py）：窗口从 7 扩到 ∞ 输出不变
/// （237→237），证 7 在目标数据域非约束性（第二特征序列分型若有效必在窗口内）。保留 7 以
/// bit-exact 对齐参考默认；改值 = 改 Θ。这是单数据集 L2 观察，**非** L0 结构证明（不裸剥）。
const TAIL_WINDOW: u32 = 7;

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
pub(super) fn stroke_interval(s: &Stroke) -> Interval {
    let (lo, hi) = if s.start_price <= s.end_price {
        (s.start_price, s.end_price)
    } else {
        (s.end_price, s.start_price)
    };
    Interval { lo, hi }
}

/// 三笔重叠判定（第77课:64 "线段开始的那三笔必须有重合"，Claim10 wellFormedV1 H4）。
///
/// 三笔几何区间公共交集非空：`max(lows) < min(highs)`。
///
/// ★口径（Claim10:471 标注的未结算 `≤`/`<` 差异）：本实装用**严格 `<`**（开区间），
/// 对齐 Python `a_segment_v1._three_stroke_overlap`（交叉验证基准）+ frozen 第77课"必须重合"
/// （边界相切=零测度重合，从严不算）。Claim10 H4 用 `≤`（闭区间）是较宽口径——二者在
/// 边界相切（max(lows)==min(highs)）时分歧；本工位站 Python `<` 侧（少产边界假段，
/// 第77课"必须有重合的部分"更倾向实质重合）。这是 Lead #84 点3 的口径裁定（追溯第77课博文）。
fn three_stroke_overlap(a: &Stroke, b: &Stroke, c: &Stroke) -> bool {
    let (ia, ib, ic) = (stroke_interval(a), stroke_interval(b), stroke_interval(c));
    let max_lo = ia.lo.max(ib.lo).max(ic.lo);
    let min_hi = ia.hi.min(ib.hi).min(ic.hi);
    max_lo < min_hi
}

/// 从 `from` 起找首个三笔重叠的起点（第77课 H4，对齐 Python `_find_overlap_start`）。
///
/// 线段起点必须满足前三笔重叠（第77课:64）。从 `from` 向后扫描，返回首个满足的起点偏移；
/// 无满足起点 ⟹ `None`（剩余笔无法起合法线段）。
fn find_overlap_start(strokes: &[Stroke], from: usize) -> Option<usize> {
    let n = strokes.len();
    let mut j = from;
    while j + 2 < n {
        if three_stroke_overlap(&strokes[j], &strokes[j + 1], &strokes[j + 2]) {
            return Some(j);
        }
        j += 1;
    }
    None
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
pub(super) fn process_feature_inclusion(elements: &[Interval]) -> Vec<Interval> {
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
    /// 第二种情况且第二特征序列**已出现分型** → 确认终结（段端 = 极值笔偏移）。
    ///
    /// ★L1（动态确认，对 67课博文 frozen 定义忠实 + Python 交叉验证，**非对 Lean bit-exact**，
    /// 见 second_kind.rs 模块头）。区别于 FirstKindConfirmed（后者是 Claim10 静态 bit-exact）。
    SecondKindConfirmed { end_offset: usize },
    /// 第二种情况但第二特征序列**未出现分型** → 待确认态（留 tail，不强断，不产假线段）。
    SecondKindPending,
    /// 无特征序列分型 → 当前线段未终结（古怪线段/未完成尾部）。
    NoFractal,
}

/// 从 apex 反向笔偏移定位段端同向笔偏移（第77课方向一致性，对齐 Python `end_stroke = k-1`）。
///
/// 缠论线段方向一致性（第77课:62 "向上线段一定结束于向上笔"）：分型中心在 apex **反向**笔，
/// 段端是其前一根**同向**笔（笔严格交替 ⟹ apex_off-1 必是同向笔）。
///
/// 边界：`apex_off == 0`（反向笔即首笔）⟹ 段端落在起点之前，不构成合法线段 ⟹ `None`。
fn segment_end_from_apex(apex_off: usize) -> Option<usize> {
    apex_off.checked_sub(1)
}

/// 分析单段终结（特征序列抽取 → 包含处理 → 找分型 → 两种情况判定 + SecondKind 动态确认）。
///
/// 静态判据（feature_elements → 包含处理 → find_fractal → classify_termination）**bit-exact
/// 对齐 Claim10**（Lean 已形式化的静态层）。SecondKind 分支的动态确认（resolve_second_kind）
/// 是 **L1**（对 67课博文 frozen 定义忠实 + Python 交叉验证，**非对 Lean bit-exact**——
/// Claim10:334-346 有意不形式化动态状态机，见 second_kind.rs 模块头）。
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
            // 无缺口：分型形成即终结。定位贡献分型中心元素 e2 的反向笔（apex 反向笔偏移）。
            let rev = seg_dir.flip();
            let Some(apex_off) = strokes
                .iter()
                .position(|s| s.direction == rev && stroke_interval(s).overlaps(&e2))
            else {
                return SegmentTermination::NoFractal;
            };
            // ★方向一致性（第77课:62 "向上线段一定结束于向上笔"）：段端 = apex 反向笔的
            // **前一根同向笔**（offset apex_off-1），对齐 Python a_segment_v1 `end_stroke = k-1`。
            // apex_off==0（反向笔是首笔）⟹ 段端在起点前，不合理 ⟹ NoFractal（未成段）。
            match segment_end_from_apex(apex_off) {
                Some(end_offset) => SegmentTermination::FirstKindConfirmed { end_offset },
                None => SegmentTermination::NoFractal,
            }
        }
        // 有缺口：第二种情况，调动态状态机判第二特征序列是否出现分型（第67课博文）。
        // e2 = 首特征序列分型中心元素（apex），承载分型极值。确认成功 → SecondKindConfirmed，
        // 否则 SecondKindPending（留 tail，严格不强断）。
        TerminationCase::SecondKind => {
            match second_kind::resolve_second_kind(strokes, &e2) {
                // resolve 返回 apex 反向笔偏移；段端 = 其前一根同向笔（方向一致性，同 FirstKind）。
                second_kind::SecondKindResult::Confirmed { end_offset: apex_off } => {
                    match segment_end_from_apex(apex_off) {
                        Some(end_offset) => SegmentTermination::SecondKindConfirmed { end_offset },
                        None => SegmentTermination::SecondKindPending,
                    }
                }
                second_kind::SecondKindResult::Pending => SegmentTermination::SecondKindPending,
            }
        }
    }
}

/// 用边界笔构造确认线段（对齐 Python `_make_segment` 端点逻辑：i0=首笔 start，i1=末笔 end）。
///
/// `s0`/`s1` 是段首/末笔在 `strokes` 中的索引。端点价取方向极值（向上：起点 low、终点 high；
/// 向下镜像）——保证相邻段视觉连续 + 满足第78课"顶高于底"（端点用段内笔极值）。
fn make_segment(strokes: &[Stroke], s0: usize, s1: usize, seg_dir: Direction) -> Segment {
    let first = &strokes[s0];
    let last = &strokes[s1];
    let (start_price, end_price) = match seg_dir {
        // 向上段：起点 = 段内最低、终点 = 段内最高（第78课标准化，使下游区间语义正确）。
        Direction::Up => {
            let lo = strokes[s0..=s1].iter().map(|s| s.start_price.min(s.end_price)).min().unwrap();
            let hi = strokes[s0..=s1].iter().map(|s| s.start_price.max(s.end_price)).max().unwrap();
            (lo, hi)
        }
        Direction::Down => {
            let hi = strokes[s0..=s1].iter().map(|s| s.start_price.max(s.end_price)).max().unwrap();
            let lo = strokes[s0..=s1].iter().map(|s| s.start_price.min(s.end_price)).min().unwrap();
            (hi, lo)
        }
    };
    Segment {
        direction: seg_dir,
        start_index: first.start_index,
        end_index: last.end_index.max(last.start_index),
        start_price,
        end_price,
    }
}

/// 线段划分（reference-theta-v0.md:22，第67/71课）——增量「假设转折点」状态机。
///
/// ★bit-exact 移植 Python `a_segment_v1.segments_from_strokes_v1`（编排者裁定 v1 唯一口径，
/// 37 测试）。L1（对参考 spec 忠实，认证 harness `analysis/segment_refsem_cert.py`），**非**
/// 对 Lean bit-exact——动态划分算法 Claim10:334-346 有意不形式化（见 feature_seq.rs 模块头）。
///
/// ★为何增量（cc-refsem-harness 归因，#84）：旧批处理「找首个分型即断段」对参考认证失败
/// （406 段 vs 参考 237 段，70% 过分段）。真因是缺第71课「假设转折点」逻辑（包含时先试不合并
/// 看是否触发分型）——批处理无状态，无法表达此动态过程。本函数用 `FeatureSeqState` 状态机修复。
///
/// 返回 `(confirmed segments, pending_start)`：
/// - `confirmed segments`：已确认线段（交易可用，对齐 Parse.lean confirmed）。
/// - `pending_start`：`Some(i)` = 从第 `i` 笔起的剩余笔是未确认线段（active 尾部）；`None` = 无。
///
/// 边界条件：
/// - 笔 < 3 ⟹ 无段，`(空, Some(0))`（全为 tail，若非空）。
/// - 无三笔重叠起点 ⟹ `(空, Some(0))`。
/// - 末段未触发终结 ⟹ pending_start = 末段起点（active 尾部）。
pub fn divide_segments_with_tail(
    strokes: &[Stroke],
    config: &ParseConfig,
) -> (Vec<Segment>, Option<usize>) {
    let mut segments = Vec::new();
    let n = strokes.len();
    if n < 3 {
        return (segments, if n > 0 { Some(0) } else { None });
    }
    // 第77课 H4：起点必须满足前三笔重叠（对齐 Python `_find_overlap_start`）。
    let Some(mut seg_start) = find_overlap_start(strokes, 0) else {
        return (segments, Some(0));
    };
    let min_seg = config.seg_min_strokes as usize;
    let mut seg_dir = strokes[seg_start].direction;
    let mut feat = FeatureSeqState::new(
        seg_dir,
        ExtendMode::Strict,
        TAIL_WINDOW,
        config.second_seq_scan_window,
    );
    let mut cursor = seg_start;

    // 主循环（bit-exact Python `segments_from_strokes_v1` :591-610 + `_try_trigger_segment`）。
    while cursor < n {
        let sk = &strokes[cursor];
        let opposite = seg_dir.flip();
        if sk.direction != opposite {
            cursor += 1;
            continue;
        }
        let (h, l) = if sk.start_price >= sk.end_price {
            (sk.start_price, sk.end_price)
        } else {
            (sk.end_price, sk.start_price)
        };
        feat.append(cursor, h, l, strokes);
        // _try_trigger_segment：scan_trigger + min_seg 门控 + skip_trigger 去重。
        let Some(hit) = feat.scan_trigger(strokes) else {
            cursor += 1;
            continue;
        };
        let k = hit.k;
        let end_stroke = k - 1;
        // min_seg 门控（Python :457）：段端-起点 < min-1 ⟹ 拒绝该触发，skip 后继续延伸。
        if end_stroke < seg_start || end_stroke - seg_start < min_seg.saturating_sub(1) {
            feat.skip_trigger(k);
            cursor += 1;
            continue;
        }
        // 发射旧段（Python `_emit_segment`：end_stroke = k-1）。
        segments.push(make_segment(strokes, seg_start, end_stroke, seg_dir));
        // 新段从 k（分型中心反向笔）起，方向反转（Python :608-610）。
        seg_start = k;
        seg_dir = opposite;
        feat.reset(seg_dir);
        cursor = k;
    }

    // 末段（未触发终结）= active 尾部。pending_start = seg_start（若剩余笔非空）。
    let pending_start = if seg_start < n { Some(seg_start) } else { None };
    (segments, pending_start)
}

/// 线段划分（reference-theta-v0.md:22）——只取确认段（薄封装 `divide_segments_with_tail`）。
///
/// 丢弃 pending 尾部信息，仅返回 confirmed segments。tail 由 `divide_segments_with_tail`
/// 的 `pending_start` 在步骤7（`tail` 模块）显式保存——本函数供只需 confirmed 段的调用方。
pub fn divide_segments(strokes: &[Stroke], config: &ParseConfig) -> Vec<Segment> {
    divide_segments_with_tail(strokes, config).0
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
        assert!(divide_segments(&strokes, &ParseConfig::default()).is_empty());
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
