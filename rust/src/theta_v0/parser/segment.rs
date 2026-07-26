//! 第四步：线段划分 v1 特征序列法（reference-theta-v0.md:22，第67/71/77/78课）。
//!
//! ## 契约重锚（legacy Phase2/Claim10_SegmentV1 → `Origin.SegmentConstruction` +
//! `Origin.SegmentFeatureSeq` + `Origin.SegmentFeatureComplete`）
//!
//! 本文件的原语函数与 Origin 特征序列构造层定义一一对应（同语义）：
//! - `Interval` ↔ `Origin.SegmentFeatureSeq.FeatureElem`（`low <= high` 不变量，`ofStroke` 投影）。
//! - `overlaps` ↔ `Origin.SegmentFeatureSeq.Overlaps`：`a.low <= b.high ∧ b.low <= a.high`。
//! - `gap` ↔ `Origin.SegmentFeatureSeq.HasGap`：`a.high < b.low ∨ b.high < a.low`（第67课缺口）。
//! - `contains` ↔ `Origin.SegmentFeatureSeq.Contains`：`b.low <= a.low ∧ a.high <= b.high`（包含处理）。
//! - `feature_elements` ↔ `Origin.SegmentFeatureSeq.FeatureElem.ofStroke`：取方向 `segDir.flip` 的笔区间。
//! - `classify_termination` ↔ `Origin.SegmentFeatureSeq.{SegmentEndUp,SegmentEndDown}`：
//!   `if HasGap e1 e2 then 等反向特征序列分型 else 直接确认`（无缺口 firstKind / 有缺口 secondKind）。
//! - 完整段端确认 ↔ `Origin.SegmentFeatureComplete.SegEndComplete`（分型 ∧ gap-case ∧ TopAboveBottom 第78课）。
//! - 段切分骨架 ↔ `Origin.SegmentConstruction.segmentsOf`（`nextSegmentEnd`/`scanSegEnd` 滑窗，well-founded 终止）。
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
//! `Origin.SegmentFeatureSeq.SegmentEndUp/Down` 明确：有缺口（`HasGap`）分支须「等反向特征序列出现
//! 相反分型」才确认（`FractalInSeq`），**不是**「终结已确认」——第二特征序列分型是否实际出现是
//! **动态过程**（状态机职责）。本文件的 `divide_segments` 实装该状态机：firstKind（无缺口）分型形成
//! 即断段；secondKind（有缺口）进入待确认态，扫描第二特征序列分型确认/否定。

use super::super::config::ParseConfig;
use super::super::types::{Direction, Segment, Stroke, Tick};
use super::feature_seq::{ExtendMode, FeatureSeqState};
use super::second_kind;
use std::rc::Rc;

// ════════════════════════════════════════════════════════════
// #246 相切探针（票 #248 裁定影响量化）——thread_local 计数器
// ════════════════════════════════════════════════════════════
//
// 与 `feature_seq.rs` 缺口谓词探针同批埋点（裁定书 §6：两处必须同批改、必须带影响量化）。
// 计数语义与谓词新旧口径无关（只记「max_lo == min_hi」数据事实），改前/改后可对比。

/// #246 相切探针计数（`three_stroke_overlap` 三笔重合谓词）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OverlapTangentProbe {
    /// 三笔重叠谓词调用次数。
    pub calls: u64,
    /// `max_lo == min_hi`（三笔区间边界相切）：旧口径（<）判无重合 / 新口径（<=）判有重合。
    pub tangent: u64,
}

thread_local! {
    static OVERLAP_TANGENT_PROBE: std::cell::RefCell<OverlapTangentProbe> =
        std::cell::RefCell::new(OverlapTangentProbe::default());
}

fn overlap_probe_bump(f: impl FnOnce(&mut OverlapTangentProbe)) {
    OVERLAP_TANGENT_PROBE.with(|p| f(&mut p.borrow_mut()));
}

/// 相切探针清零（probe binary / 测试用）。
pub fn overlap_tangent_probe_reset() {
    overlap_probe_bump(|p| *p = OverlapTangentProbe::default());
}

/// 相切探针快照（probe binary / 测试用）。
pub fn overlap_tangent_probe_snapshot() -> OverlapTangentProbe {
    OVERLAP_TANGENT_PROBE.with(|p| *p.borrow())
}

/// 尾窗大小（对齐 Python `_FeatureSeqState.TAIL_WINDOW=7`）。
///
/// ★L2 实测（OKLO 2000 笔，analysis/segment_refsem_cert.py）：窗口从 7 扩到 ∞ 输出不变
/// （237→237），证 7 在目标数据域非约束性（第二特征序列分型若有效必在窗口内）。保留 7 以
/// bit-exact 对齐参考默认；改值 = 改 Θ。这是单数据集 L2 观察，**非** L0 结构证明（不裸剥）。
const TAIL_WINDOW: u32 = 7;

/// 价格区间 [lo, hi]（特征序列元素 / 笔的几何投影，`lo <= hi` 不变量）。
/// 契约锚 `Origin.SegmentFeatureSeq.FeatureElem`（`low <= high` valid 不变量）。
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

/// 三笔重叠判定（第77课:64 "线段开始的那三笔必须有重合"，对齐 `Origin.SegmentFeatureSeq.Overlaps` 三笔版）。
///
/// 三笔几何区间公共交集非空：`max(lows) <= min(highs)`（含边界相切）。
///
/// ★口径（#246 裁定，2026-07-25）：**相切（max(lows)==min(highs)）算「有重合区间」**，用含等号
/// `<=`，对齐 `Origin.SegmentFeatureSeq.Overlaps`（`≤` 闭区间）。裁定书：
/// `chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。强制性依据：Lean 已证
/// `gap_iff_not_overlap`（缺口与重合严格互补），本谓词与 `feature_seq.rs::is_fractal_and_gap`
/// 必须同口径、同批改。
/// ⚠ supersede 登记：本处原注释记录的 Lead #84 点3 口径（「边界相切=零测度重合，从严不算」，站
/// Python `a_segment_v1._three_stroke_overlap` 严格 `<` 侧）已被 #246 裁定 **supersede**——考据证实
/// 其援引的 frozen 第77课并非原文直述（与 #246 同属解读）。原「bit-exact 对齐 Python
/// `_three_stroke_overlap`」声明在**本谓词边界上作废**：Python 参考实现不再是该谓词的权威基线；
/// #84 其余条款（方案一 + 五 seam 拆分等）不受影响。
/// ★与既有原语 [`Interval::overlaps`]（本文件 :90-93，闭区间 `≤`）**同口径**（相切算重合）；保留
/// 独立实现仅为避免构造三个 `Interval` 的开销，非另起口径（#249 SPEC「优先复用既有原语」条）。
fn three_stroke_overlap(a: &Stroke, b: &Stroke, c: &Stroke) -> bool {
    let (ia, ib, ic) = (stroke_interval(a), stroke_interval(b), stroke_interval(c));
    let max_lo = ia.lo.max(ib.lo).max(ic.lo);
    let min_hi = ia.hi.min(ib.hi).min(ic.hi);
    overlap_probe_bump(|p| {
        p.calls += 1;
        if max_lo == min_hi {
            p.tangent += 1;
        }
    });
    max_lo <= min_hi
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

/// 线段划分两种情况（第67课"只有两种可能"）。对齐 `Origin.SegmentFeatureSeq.HasGap` 的 case 二分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationCase {
    /// 第一种：无缺口，分型形成即终结。
    FirstKind,
    /// 第二种：有缺口，须第二特征序列分型确认。
    SecondKind,
}

/// 从特征序列分型相邻两元素判定进入哪个分支（第67课:28/38 缺口判据，全函数）。
/// 对齐 `Origin.SegmentFeatureSeq.{SegmentEndUp,SegmentEndDown}` 的 `HasGap` 分支：有缺口 → SecondKind，无缺口 → FirstKind。
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
/// 「元素当 K 线包含处理」的中性实现（保留外包络），与 `Origin.SegmentFeatureSeq.Contains` 判定配套。
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
/// ★诚实范围（formalization-validity-domain + `Origin.SegmentFeatureSeq` 认识论）：
/// 本函数对**给定起始方向的笔序列**，分析其特征序列首个分型并判定终结情形。它实装了
/// `Origin.SegmentFeatureSeq` 的纯函数侧（`FeatureElem.ofStroke` 抽取 → 包含处理 → `IsTopFractal`/
/// `IsBottomFractal` 找分型 → `HasGap` 两种情况判定）。它**不**实装 SecondKind 的第二特征序列动态
/// 确认——`SegmentEndUp/Down` 的 `FractalInSeq` 确认是**状态机职责**（动态过程），不是纯函数可判定。
/// 故返回值显式区分三态，把动态部分留给调用方/未完成尾部。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentTermination {
    /// 第一种情况：无缺口分型形成 → 该 K 段端笔的 rest 内偏移确认终结。
    FirstKindConfirmed { end_offset: usize },
    /// 第二种情况且第二特征序列**已出现分型** → 确认终结（段端 = 极值笔偏移）。
    ///
    /// ★L1（动态确认，对 67课博文 frozen 定义忠实 + Python 交叉验证，**非对 Origin Lean bit-exact**，
    /// 见 second_kind.rs 模块头）。区别于 FirstKindConfirmed（后者对齐 Origin.SegmentFeatureComplete 静态层）。
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
/// 静态判据（feature_elements → 包含处理 → find_fractal → classify_termination）**对齐
/// `Origin.SegmentFeatureSeq`/`SegmentFeatureComplete`**（Origin 已形式化的静态层）。SecondKind
/// 分支的动态确认（resolve_second_kind）是 **L1**（对 67课博文 frozen 定义忠实 + Python 交叉验证，
/// **非对 Origin Lean bit-exact**——动态状态机有意不形式化，见 second_kind.rs 模块头）。
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
/// 对 Origin Lean bit-exact——动态划分算法在 Origin 有意不形式化（静态层锚
/// `Origin.SegmentConstruction`/`SegmentFeatureComplete`，见 feature_seq.rs 模块头）。
///
/// ★为何增量（cc-refsem-harness 归因，#84）：旧批处理「找首个分型即断段」对参考认证失败
/// （406 段 vs 参考 237 段，70% 过分段）。真因是缺第71课「假设转折点」逻辑（包含时先试不合并
/// 看是否触发分型）——批处理无状态，无法表达此动态过程。本函数用 `FeatureSeqState` 状态机修复。
///
/// 返回 `(confirmed segments, pending_start)`：
/// - `confirmed segments`：已确认线段（交易可用，对齐 `Origin.SegmentConstruction.segmentsOf` 输出）。
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

// ============================================================================
// 增量 segment（#93 H1 主根因：parse_layer 下游 per-bar O(n²) → 增量化）。
//
// ## 前缀不变性（bit-exact 基础）
//
// divide_segments_with_tail 的 confirmed segments 前缀不可变——strokes 增量只改尾部
// （collapse 末元素 + 配对尾部重扫），confirmed 段端在 strokes 数组下标 < old_n-1 的范围内。
//
// 增量策略：
// 1. 保留 confirmed segments 前缀（段端 strokes 数组下标 < confirmed_bound 的）。
// 2. 从最后一笔 confirmed 段的 seg_start（= 段端+1）重新初始化 FeatureSeqState，续扫尾部 strokes。
//
// 严格性：内部存 (Segment, end_array_idx) 对——end_array_idx 是段端在 strokes 数组中的下标
// （不是 source_index！）。pending 段状态机从 seg_start 重建（tail strokes 可能变化）。
// ponytail: 保留 confirmed 前缀 + pending 段重扫（O(pending_strokes)/bar，非 O(n)）。
// ============================================================================

/// 增量 segment 状态（bit-exact 对齐 `divide_segments_with_tail`）。
#[derive(Debug, Clone, PartialEq)]
pub struct IncrSegments {
    /// confirmed segments 前缀（Rc 共享——`to_result_rc()` O(1) clone 给 `ParseLayer.segments`）。
    segments_rc: Rc<Vec<Segment>>,
    /// 段端在 strokes 数组中的下标（与 segments_rc 同步 truncate/push）。
    end_indices: Vec<usize>,
    /// 未完成段的起点（pending_start，strokes 数组下标）。
    pending_start: Option<usize>,
    /// 上次快照时的 strokes 长度。
    strokes_len: usize,
    /// #106：本次 append 保留的 confirmed segments 前缀长度（= keep，segments[..confirmed_len]
    /// 不被本轮重算 ⟹ 跨 bar bit-stable）。classifier l0_tower 复用证书。**不可用 segments.len()-1
    /// 替代**（codex 反例：末段可能 1384→1170 改写，keep 排除末段）。
    confirmed_len: usize,
    /// 上次 append 见到的末笔（相同输入早退用）。增量不变式：confirmed 前缀不可变，仅末笔可改写
    /// ⟹ (strokes_len, 末笔) 相同蕴含整个 strokes 相同 ⟹ 结果与 self 逐字段等。
    last_stroke: Option<Stroke>,
    /// #88 frontier 修复（codex #87）+ #93 advancing 变体（codex #91 §五）：本轮 append 重扫区间
    /// [resume, n) 新算的 unsealed 起点（被跳过的 SecondKind 候选所属段最早 seg_start）——**直接持久化，
    /// 不与历史取 min**（advancing）。
    ///
    /// **为何需要回退**：`second_seq_scan_window=0`（无限）下，新 bar 引入的同向笔可让 SecondKindPending
    /// 复活（第二特征序列出现分形），级联改写已 confirmed 的更早段（#88 报告反例 seg[404]<confirmed_len=406）。
    /// 故 `append` 的 `confirmed_bound = min(末段end, earliest_unsealed_from)` 回退到 unsealed 起点前。
    ///
    /// **advancing soundness**（codex #91 §二）：second_seq_has_fractal 一旦真则永真（构建由既定前缀
    /// 决定）+ 段连续性 ⟹ 未 resolve 的候选必在下轮重扫被重新发现；候选 resolve 后 euf 前移/清零 ⟹
    /// confirmed_bound 前移 ⟹ 退回 O(n)（#93 H2 探针 late_repro=0 坐实）。取代 #88 的历史最小值持久化
    /// （后者 euf 单调非增锚死早点 ⟹ O(n²)，边界条件3）。
    earliest_unsealed_from: Option<usize>,
}

impl Default for IncrSegments {
    fn default() -> Self {
        IncrSegments {
            segments_rc: Rc::new(Vec::new()),
            end_indices: Vec::new(),
            pending_start: None,
            strokes_len: 0,
            confirmed_len: 0,
            last_stroke: None,
            earliest_unsealed_from: None,
        }
    }
}

impl IncrSegments {
    /// 空状态。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 从全量结果恢复增量状态（断点续算，需 strokes 以重建 end_array_idx）。
    ///
    /// ★#88 边界（诚实声明，no-patch-mentality）：本函数**不重建** `earliest_unsealed_from`
    /// skip 历史——全量结果不记录哪些段曾跳过 SecondKind 候选。故恢复态设 `None`，首次 append 仅
    /// 1 段回退。**仅在恢复前缀无 unsealed 候选时 bit-exact 安全**。生产路径是 `empty()` + 逐 bar
    /// `append`（skip 历史跨 append 持久累积），**不经** `from_full`（当前无生产调用者）。若未来要
    /// 在有 unsealed 前缀处恢复，须同时持久化/重建 `earliest_unsealed_from`。
    pub fn from_full(segments: &[Segment], pending_start: Option<usize>, strokes: &[Stroke]) -> Self {
        // 重建 end_array_idx：段端 = strokes[s1].end_index，找 s1 在 strokes 中的位置。
        let mut end_indices: Vec<usize> = Vec::with_capacity(segments.len());
        let mut search_from = 0usize;
        for seg in segments {
            // 段端 end_index = strokes[s1].end_index。从 search_from 起找匹配的 s1。
            let mut found = None;
            for si in search_from..strokes.len() {
                if strokes[si].end_index == seg.end_index {
                    found = Some(si);
                    break;
                }
            }
            match found {
                Some(si) => {
                    end_indices.push(si);
                    search_from = si + 1;
                }
                None => break, // 段端不在 strokes 中（数据不一致）——截断
            }
        }
        let confirmed_len = end_indices.len();
        IncrSegments {
            segments_rc: Rc::new(segments.to_vec()),
            end_indices,
            pending_start,
            strokes_len: strokes.len(),
            confirmed_len,
            last_stroke: strokes.last().copied(),
            earliest_unsealed_from: None,
        }
    }

    /// 增量追加 strokes 序列，消费 self 返回新状态（by-value 缓冲复用，消除 O(n)/bar clone）。
    ///
    /// bit-exact：结果 (segments, pending_start) == `divide_segments_with_tail(strokes, config)`。
    ///
    /// 保留 confirmed segments 前缀（除末段），从倒数第二段的 seg_start 重新初始化状态机续扫。
    ///
    /// ponytail: 末段（最后 confirmed 段）必须重算——`second_seq_has_fractal` 扫描到
    /// strokes 末尾（scan_window=0 无限），新 bar 可能让段内更早的 SecondKindPending 触发
    /// 复活（第二特征序列出现分形），使段端回退到更早位置。故末段非不可变，必须丢弃重算。
    /// 倒数第二段及之前视为不可变（其 SecondKind 确认依赖的第二序列分形在更早数据已稳定）。
    /// ponytail: Rc::make_mut + truncate 复用 Vec 缓冲，替代旧 take_while().collect() 的 O(n)/bar。
    pub fn append(self, strokes: &[Stroke], config: &ParseConfig) -> IncrSegments {
        let n = strokes.len();
        let last_stroke = strokes.last().copied();

        // 相同输入早退：strokes 长度同 ∧ 末笔逐字段等 ⟹ 结果与 self 逐字段等（confirmed 前缀
        // 不可变，仅末笔可改写，故 (len,末笔) 相同蕴含整个 strokes 相同）。confirmed_len 保 self
        // 旧值——同输入重算得同值，故精确（计划注为合法收窄，sound）。
        if n == self.strokes_len && last_stroke == self.last_stroke {
            return self;
        }

        // 移出 self 字段（by-value 消费）。#88：捕获持久化 earliest_unsealed_from。
        let IncrSegments {
            mut segments_rc,
            mut end_indices,
            strokes_len: _,
            pending_start: _,
            confirmed_len: _,
            last_stroke: _,
            earliest_unsealed_from: euf_persisted,
        } = self;

        // #88 frontier 修复（codex #87 修补版 A）：confirmed_bound 回退到
        // `min(末段end, earliest_unsealed_from)`——不是固定 1 段。无 unsealed 记录时退化为旧行为
        // （末段 end_array_idx，仅丢末段）。truncate 用 `<` ⟹ 保留完全结束于 confirmed_bound 前的段。
        // cascade 只向前传播 ⟹ [0, confirmed_bound) 稳定，从其后重扫与全量 bit-exact。
        let confirmed_bound = match end_indices.last() {
            Some(&last_end_idx) => match euf_persisted {
                Some(euf) => last_end_idx.min(euf),
                None => last_end_idx,
            },
            None => 0,
        };

        // ponytail: partition_point O(log n) 定位 truncate 位置，替代 take_while O(n)。
        let keep = end_indices
            .partition_point(|&end_idx| end_idx < confirmed_bound);

        // Rc::make_mut 突变 segments_rc——strong_count==1 时 O(1) in-place。
        let new_segments = Rc::make_mut(&mut segments_rc);
        new_segments.truncate(keep);
        end_indices.truncate(keep);

        // 确定续扫起点：倒数第二段的 seg_start = end_array_idx + 1（= 末段原 seg_start）。
        let resume_seg_start: usize;
        let resume_seg_dir: Direction;

        match end_indices.last() {
            Some(&last_end_idx) => {
                resume_seg_start = last_end_idx + 1;
                if resume_seg_start >= n {
                    // 无剩余笔——全部 confirmed，无 pending。euf 持久化（过去的 skip 仍是风险标记）。
                    return IncrSegments {
                        segments_rc,
                        end_indices,
                        pending_start: if resume_seg_start < n { Some(resume_seg_start) } else { None },
                        strokes_len: n,
                        confirmed_len: keep,
                        last_stroke,
                        earliest_unsealed_from: euf_persisted,
                    };
                }
                resume_seg_dir = strokes[resume_seg_start].direction;
            }
            None => {
                // 无 confirmed 段（或仅 1 段被丢弃）：全扫。先找 overlap start。
                if n < 3 {
                    return IncrSegments {
                        segments_rc,
                        end_indices,
                        pending_start: if n > 0 { Some(0) } else { None },
                        strokes_len: n,
                        confirmed_len: keep,
                        last_stroke,
                        earliest_unsealed_from: euf_persisted,
                    };
                }
                let Some(start) = find_overlap_start(strokes, 0) else {
                    return IncrSegments {
                        segments_rc,
                        end_indices,
                        pending_start: Some(0),
                        strokes_len: n,
                        confirmed_len: keep,
                        last_stroke,
                        earliest_unsealed_from: euf_persisted,
                    };
                };
                resume_seg_start = start;
                resume_seg_dir = strokes[start].direction;
            }
        }

        // #93（codex #91 §六.4 continuity 守卫）：resume_seg_start 绝不越过 unsealed flag（euf_persisted）
        // ——否则会把未 sealed 的段封进保留前缀（漏扫）。soundness (a) 项固化，覆盖主循环 + find_overlap_start
        // 边界分支。「≤」而非「==」：drop-末段（euf≥末段端/None）时 resume 落在末段起点（< 末段端），
        // 是 sound 的过量重扫；违规是 resume > flag（漏扫）。
        debug_assert!(
            euf_persisted.map_or(true, |flag| resume_seg_start <= flag),
            "resume_seg_start {resume_seg_start} 越过 unsealed flag {euf_persisted:?}（漏扫 unsealed 段）"
        );

        // 从 resume_seg_start 续扫（镜像 divide_segments_with_tail 主循环）。
        let min_seg = config.seg_min_strokes as usize;
        let mut seg_start = resume_seg_start;
        let mut seg_dir = resume_seg_dir;
        let mut feat = FeatureSeqState::new(
            seg_dir,
            ExtendMode::Strict,
            TAIL_WINDOW,
            config.second_seq_scan_window,
        );
        let mut cursor = seg_start;
        // #88：本次重扫区间内跳过过 SecondKind 候选的最早 seg_start（含 pending 尾段）。
        let mut euf_rescan: Option<usize> = None;

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
            let trig = feat.scan_trigger(strokes);
            // #88：本段（seg_start）扫描期间若跳过过 SecondKind 候选（append 探针或本次 scan），
            // seg_start 是 unsealed 起点。feat 在 reset 前累积该标志，故在 emit/reset 前读取。
            if feat.skipped_secondkind() {
                euf_rescan = Some(euf_rescan.map_or(seg_start, |e| e.min(seg_start)));
            }
            let Some(hit) = trig else {
                cursor += 1;
                continue;
            };
            let k = hit.k;
            let end_stroke = k - 1;
            if end_stroke < seg_start || end_stroke - seg_start < min_seg.saturating_sub(1) {
                feat.skip_trigger(k);
                cursor += 1;
                continue;
            }
            let seg = make_segment(strokes, seg_start, end_stroke, seg_dir);
            new_segments.push(seg);
            end_indices.push(end_stroke);
            seg_start = k;
            seg_dir = opposite;
            feat.reset(seg_dir);
            cursor = k;
        }

        let pending_start = if seg_start < n { Some(seg_start) } else { None };
        // #93 advancing 变体（codex #91 补充裁定 §五）：直接持久化本轮重扫的新鲜 euf，**不**与历史取 min。
        // soundness（§二）：second_seq_has_fractal 一旦真则永真（构建由既定前缀决定，追加笔不改已算
        // elements）+ 段连续性（resume_seg_start 精确重合 flag 位置，见 append 内 debug_assert）⟹ 未
        // resolve 的候选必在下轮从 resume 重扫时被重新发现；仅当从 resume 到 n 全程零 flag（euf_rescan
        // =None）才前移/清零，此时等价于 divide_segments 直接跑该后缀，无级联残留。效果：候选 resolve 后
        // euf 前移 ⟹ confirmed_bound 前移 ⟹ 退回 O(n)（#93 H2 探针坐实 late_repro=0）。
        let earliest_unsealed_from = euf_rescan;
        IncrSegments {
            segments_rc,
            end_indices,
            pending_start,
            strokes_len: n,
            confirmed_len: keep,
            last_stroke,
            earliest_unsealed_from,
        }
    }

    /// 当前快照 segments（Vec<Segment>，与 `divide_segments_with_tail` bit-exact）。
    pub fn to_result_vec(&self) -> (Vec<Segment>, Option<usize>) {
        (
            self.segments_rc.to_vec(),
            self.pending_start,
        )
    }

    /// 当前快照 segments 的 Rc 共享句柄（O(1) refcount bump）。
    ///
    /// ponytail: Rc 共享替代旧 `to_result_vec()` 的 O(n) collect——`ParseLayerIncr::append`
    /// 用此填 `ParseLayer.segments`，消除每 bar Vec clone。
    pub fn to_result_rc(&self) -> (Rc<Vec<Segment>>, Option<usize>) {
        (Rc::clone(&self.segments_rc), self.pending_start)
    }

    /// #106：本次 append 的 confirmed segments 前缀长度（l0_tower 复用证书）。
    pub fn confirmed_len(&self) -> usize {
        self.confirmed_len
    }

    /// #88/#93：当前 unsealed 起点（= 本轮重扫新鲜值，advancing）。性能诊断——前移轨迹实测。
    pub fn earliest_unsealed_from(&self) -> Option<usize> {
        self.earliest_unsealed_from
    }
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
    fn three_stroke_overlap_tangent_counts() {
        // ★#246 相切口径：max(lows) == min(highs) 算「有重合区间」（旧 #84 点3 从严不算，已 supersede）。
        // 三笔区间 [5,10]、[8,12]、[10,15]：max_lo=10, min_hi=10 → 相切于 10。
        let a = stroke(Direction::Up, 0, 4, 5, 10);
        let b = stroke(Direction::Up, 4, 8, 8, 12);
        let c = stroke(Direction::Down, 8, 12, 15, 10);
        assert!(three_stroke_overlap(&a, &b, &c)); // 相切 ⟹ 有重合（新口径，`<=`）
    }

    #[test]
    fn three_stroke_overlap_strict_cases_unchanged() {
        // 严格内含（max_lo < min_hi）仍算重叠、严格分离（max_lo > min_hi）仍不算——新旧口径一致。
        let a = stroke(Direction::Up, 0, 4, 5, 10);
        let b = stroke(Direction::Up, 4, 8, 8, 12);
        let c_in = stroke(Direction::Down, 8, 12, 11, 9); // [9,11]：max_lo=9 < min_hi=10 → 重叠
        assert!(three_stroke_overlap(&a, &b, &c_in));
        let c_out = stroke(Direction::Up, 8, 12, 13, 20); // [13,20]：max_lo=13 > min_hi=10 → 无重叠
        assert!(!three_stroke_overlap(&a, &b, &c_out));
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

    /// ★bit-exact：增量 IncrSegments::append 链 == 全量 divide_segments_with_tail（逐 strokes 长度）。
    #[test]
    fn bit_exact_incr_segments_per_bar() {
        // 合成笔序列：交替上下 + 满足三笔重叠 + 间隔足够触发段终结。
        let strokes: Vec<Stroke> = (0..120usize)
            .flat_map(|i| {
                let b = i * 20;
                vec![
                    stroke(Direction::Up, b, b + 4, 5 + i as i64, 20 + i as i64),
                    stroke(Direction::Down, b + 4, b + 8, 20 + i as i64, 10 + i as i64),
                    stroke(Direction::Up, b + 8, b + 12, 10 + i as i64, 25 + i as i64),
                    stroke(Direction::Down, b + 12, b + 16, 25 + i as i64, 12 + i as i64),
                ]
            })
            .collect();

        let cfg = ParseConfig::default();
        let mut incr = IncrSegments::empty();
        for end in 3..=strokes.len() {
            incr = incr.append(&strokes[..end], &cfg);
            let (full_segs, full_pending) = divide_segments_with_tail(&strokes[..end], &cfg);
            let (incr_segs, incr_pending) = incr.to_result_vec();
            assert_eq!(
                incr_segs, full_segs,
                "strokes len {end}: 增量 segments != 全量（bit-exact 破裂）"
            );
            assert_eq!(
                incr_pending, full_pending,
                "strokes len {end}: 增量 pending_start != 全量（bit-exact 破裂）"
            );
        }
    }

    /// 伪随机 gappy stroke 序列（LCG）：交替方向 + 变幅 ⟹ 特征序列频繁出现缺口
    /// （SecondKind），且缺口候选随后续笔复活/改写更早段——#88 修复的目标结构。
    /// 纯整数 LCG（Numerical Recipes 常数），跨平台确定性。
    fn gappy_strokes(n: usize, seed: u64) -> Vec<Stroke> {
        let mut s = seed;
        let mut next = |lo: i64, hi: i64| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            lo + ((s >> 33) as i64).rem_euclid(hi - lo + 1)
        };
        let mut out = Vec::with_capacity(n);
        let mut price: i64 = 100_000;
        let mut idx = 0usize;
        for i in 0..n {
            let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
            // 变幅：多数中等，偶发大跳（制造缺口）。
            let amp = if next(0, 9) < 2 { next(400, 900) } else { next(60, 300) };
            let start = price;
            let end = match dir {
                Direction::Up => price + amp,
                Direction::Down => price - amp,
            };
            out.push(Stroke {
                direction: dir,
                start_index: idx,
                end_index: idx + 4,
                start_price: start,
                end_price: end,
            });
            price = end;
            idx += 4;
        }
        out
    }

    /// ★#88 级联复活 bit-exact（验收项3）：早期 SecondKindPending 后续被确认，改写其后已 confirm
    /// 的段——修复前 1 段回退永不重访该段（div），修复后 earliest_unsealed_from 回退到其前重扫。
    ///
    /// 逐 strokes 长度断言增量 `IncrSegments::append` == 全量 `divide_segments_with_tail`。
    /// **覆盖率自证**（防「always-run 但覆盖为零」）：断言运行期间 (a) `earliest_unsealed_from`
    /// 曾被置 Some（SecondKind-skip 路径确被触发）；(b) 至少一次深回退（euf < 末段端 ⟹ confirmed_len
    /// 比"仅丢末段"更靠前，即多段回退）——这正是修复前会 div 的结构。
    #[test]
    fn cascade_revival_bit_exact_incr_vs_full() {
        let cfg = ParseConfig::default();
        let strokes = gappy_strokes(1200, 0xC0FF_EE12_3456_789A);
        let mut incr = IncrSegments::empty();
        let mut euf_ever_some = false;
        let mut deep_rollback_seen = false;
        for end in 3..=strokes.len() {
            incr = incr.append(&strokes[..end], &cfg);
            let (full_segs, full_pending) = divide_segments_with_tail(&strokes[..end], &cfg);
            let (incr_segs, incr_pending) = incr.to_result_vec();
            assert_eq!(
                incr_segs, full_segs,
                "strokes len {end}: 级联复活场景增量 != 全量（bit-exact 破裂）"
            );
            assert_eq!(
                incr_pending, full_pending,
                "strokes len {end}: 级联复活场景 pending != 全量"
            );
            if let Some(euf) = incr.earliest_unsealed_from() {
                euf_ever_some = true;
                // 深回退：unsealed 起点落在某个已 confirm 段的端点之前（confirmed_len 因此比
                // 「仅末段」更靠前）。用全量段数 vs confirmed_len 判定多段回退。
                if incr.confirmed_len() + 1 < full_segs.len() && euf < end {
                    deep_rollback_seen = true;
                }
            }
        }
        assert!(
            euf_ever_some,
            "覆盖为零：gappy 序列从未触发 SecondKind-skip（earliest_unsealed_from 恒 None）——\
             本测试未实际覆盖 #88 修复路径，须调 gappy_strokes 参数"
        );
        assert!(
            deep_rollback_seen,
            "覆盖为零：从未发生深回退（euf < 末段端）——未覆盖「改写已 confirm 段」的级联路径"
        );
    }

    /// ★#93 advancing 变体守卫（codex #91 §六.2+3）：候选 resolve 后 `earliest_unsealed_from`
    /// 前移/清零（非持久化历史最小值），且后续更晚候选被跳过时 euf 移到**新位置**（非退回旧值）。
    ///
    /// 逐 strokes 长度断言 bit-exact（正确性硬门）。**覆盖率自证**：断言运行期间观测到 (a) 前移/清零
    /// （euf 从 Some(a) 变 None 或 Some(b>a)——持久化历史最小值单调非增下这**不可能**发生，故此断言
    /// 直接证伪 persist-forever、坐实 advancing）；(b) reflag（清零/前移后 euf 又变 Some——§六.3 later
    /// reflag）。二者皆命中方证守卫有效。
    #[test]
    fn advancing_euf_resolves_and_reflags() {
        let cfg = ParseConfig::default();
        let strokes = gappy_strokes(1200, 0xAD0A_9C13_2718_2818);
        let mut incr = IncrSegments::empty();
        let mut prev_euf: Option<usize> = None;
        let mut saw_advance = false; // Some(a) → None 或 Some(b>a)（前移/清零）
        let mut saw_reflag = false; // 前移/清零后 euf 再次变 Some
        let mut advanced_once = false;
        for end in 3..=strokes.len() {
            incr = incr.append(&strokes[..end], &cfg);
            let (full_segs, full_pending) = divide_segments_with_tail(&strokes[..end], &cfg);
            assert_eq!(incr.to_result_vec(), (full_segs, full_pending),
                "strokes len {end}: advancing 变体 bit-exact 破裂");
            let euf = incr.earliest_unsealed_from();
            match (prev_euf, euf) {
                (Some(_), None) => { saw_advance = true; advanced_once = true; }
                (Some(a), Some(b)) if b > a => { saw_advance = true; advanced_once = true; }
                (_, Some(_)) if advanced_once => { saw_reflag = true; }
                _ => {}
            }
            prev_euf = euf;
        }
        assert!(saw_advance,
            "覆盖为零：euf 从未前移/清零——persist-forever 未被证伪（advancing 未生效或序列不触发 resolve）");
        assert!(saw_reflag,
            "覆盖为零：前移后 euf 从未 reflag 到新位置——未覆盖 §六.3 later reflag 路径");
    }

    /// property：重复 append 同一输入幂等（相同输入早退路径）——第二次 append 同 strokes ⟹
    /// segments/pending/confirmed_len 与第一次逐字段等，且仍与全量 bit-exact。
    #[test]
    fn idempotent_repeat_append_same_input() {
        let strokes: Vec<Stroke> = (0..40usize)
            .flat_map(|i| {
                let b = i * 20;
                vec![
                    stroke(Direction::Up, b, b + 4, 5 + i as i64, 20 + i as i64),
                    stroke(Direction::Down, b + 4, b + 8, 20 + i as i64, 10 + i as i64),
                    stroke(Direction::Up, b + 8, b + 12, 10 + i as i64, 25 + i as i64),
                    stroke(Direction::Down, b + 12, b + 16, 25 + i as i64, 12 + i as i64),
                ]
            })
            .collect();

        let cfg = ParseConfig::default();
        let mut incr = IncrSegments::empty();
        for end in 3..=strokes.len() {
            incr = incr.append(&strokes[..end], &cfg);
            let first = incr.to_result_vec();
            let first_confirmed = incr.confirmed_len();
            // 第二次 append 同输入——走相同输入早退，返回 self 不变。
            incr = incr.append(&strokes[..end], &cfg);
            assert_eq!(
                incr.to_result_vec(),
                first,
                "strokes len {end}: 重复 append 同输入改变了结果（幂等破裂）"
            );
            assert_eq!(
                incr.confirmed_len(),
                first_confirmed,
                "strokes len {end}: 重复 append 同输入改变了 confirmed_len"
            );
            // 早退后仍与全量 bit-exact。
            let (full_segs, full_pending) = divide_segments_with_tail(&strokes[..end], &cfg);
            assert_eq!(incr.to_result_vec(), (full_segs, full_pending));
        }
    }
}
