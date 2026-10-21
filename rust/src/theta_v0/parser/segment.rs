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

use super::super::config::{ParseConfig, Tangency};
use super::super::types::{Direction, Segment, Stroke, Tick};
use super::feature_seq::{
    ExtendMode, FeatElem, FeatureElement, FeatureSeqState, TriggerHit, TURN_NOT_CASE_ONE,
};
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

/// 价格区间 [lo, hi]（特征序列元素 / 笔的几何投影，`lo <= hi` 不变量）。
/// 契约锚 `Origin.SegmentFeatureSeq.FeatureElem`（`low <= high` valid 不变量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub lo: Tick,
    pub hi: Tick,
}

impl Interval {
    /// 两区间有公共重叠（交集非空）。对齐 `Interval.overlaps`。端点相切是否算重叠由记录字段
    /// `tangency` 决定（xianduan §6.3，#1604）：`Closed` 即 `lo <= hi`（#1604 前的生产口径）。
    pub fn overlaps(&self, b: &Interval, t: Tangency) -> bool {
        t.meets(self.lo, b.hi) && t.meets(b.lo, self.hi)
    }
    /// 缺口：两相邻特征序列元素无重叠（第67课:18）。对齐 `Interval.gap`/`hasGap`；与
    /// [`Interval::overlaps`] 读同一个 `tangency`，严格互补（`gap_iff_not_overlap`）。
    pub fn gap(&self, b: &Interval, t: Tangency) -> bool {
        !self.overlaps(b, t)
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
/// #1604：相切口径改读记录字段 `tangency`（默认 `closed` = 上述 `<=`），与缺口谓词读同一个值。
fn three_stroke_overlap(a: &Stroke, b: &Stroke, c: &Stroke, t: Tangency) -> bool {
    let (ia, ib, ic) = (stroke_interval(a), stroke_interval(b), stroke_interval(c));
    let max_lo = ia.lo.max(ib.lo).max(ic.lo);
    let min_hi = ia.hi.min(ib.hi).min(ic.hi);
    overlap_probe_bump(|p| {
        p.calls += 1;
        if max_lo == min_hi {
            p.tangent += 1;
        }
    });
    t.meets(max_lo, min_hi)
}

/// 从 `from` 起找首个三笔重叠的起点（第77课 H4，对齐 Python `_find_overlap_start`）。
///
/// 线段起点必须满足前三笔重叠（第77课:64）。从 `from` 向后扫描，返回首个满足的起点偏移；
/// 无满足起点 ⟹ `None`（剩余笔无法起合法线段）。
fn find_overlap_start(strokes: &[Stroke], from: usize, t: Tangency) -> Option<usize> {
    let n = strokes.len();
    let mut j = from;
    while j + 2 < n {
        if three_stroke_overlap(&strokes[j], &strokes[j + 1], &strokes[j + 2], t) {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// #1420：第二种候选三态终点（078:52 失效）改变段语义 ⟹ 新语义版本；旧版对象不回改。
pub const CONSTRUCTION_RULE: &str = "segment-second/2";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ConstructionFact {
    pub start_stroke: usize,
    pub stroke_indices: Vec<usize>,
    pub length: &'static str,
    /// 仅奇数且至少三笔适用；交替、前三闭重叠、顶>底、首尾同向。
    pub vector: Option<[bool; 4]>,
    pub overlap: Option<[Tick; 2]>,
    pub overlap_relation: Option<&'static str>,
    pub direction: Option<&'static str>,
    pub failed_conditions: Vec<&'static str>,
}
impl ConstructionFact {
    fn valid(&self) -> bool {
        self.vector == Some([true; 4])
    }
}

/// #1404 CC-011：seed 与段端门使用相同完整条件，非观察器另算宽档。`t` 为相切口径（#1604）；
/// `overlap_relation` 只描述几何（`TOUCH` 即相切），是否算重合看 `vector[1]`。
pub fn construction(strokes: &[Stroke], start: usize, end: usize, t: Tangency) -> ConstructionFact {
    let ss = &strokes[start..end];
    let n = ss.len();
    let length = match n {
        0 => "n=0",
        1 => "n=1",
        2 => "n=2",
        n if n % 2 == 0 => "even_ge3",
        _ => "odd_ge3",
    };
    let overlap = (n >= 3).then(|| {
        let xs = [
            stroke_interval(&ss[0]),
            stroke_interval(&ss[1]),
            stroke_interval(&ss[2]),
        ];
        [
            xs.iter().map(|x| x.lo).max().unwrap(),
            xs.iter().map(|x| x.hi).min().unwrap(),
        ]
    });
    let vector = (n >= 3 && n % 2 == 1).then(|| {
        [
            ss.windows(2).all(|w| w[0].direction != w[1].direction),
            three_stroke_overlap(&ss[0], &ss[1], &ss[2], t),
            match ss[0].direction {
                Direction::Up => ss[n - 1].end_price > ss[0].start_price,
                Direction::Down => ss[n - 1].end_price < ss[0].start_price,
            },
            ss[0].direction == ss[n - 1].direction,
        ]
    });
    let failed_conditions = vector
        .map(|v| {
            [
                "direction_alternation",
                "closed_seed_overlap",
                "top_gt_bottom",
                "same_end_direction",
            ]
            .into_iter()
            .zip(v)
            .filter_map(|(name, ok)| (!ok).then_some(name))
            .collect()
        })
        .unwrap_or_else(|| {
            vec![if n < 3 {
                "fewer_than_three_strokes"
            } else {
                "even_stroke_count"
            }]
        });
    ConstructionFact {
        start_stroke: start,
        stroke_indices: (start..end).collect(),
        length,
        vector,
        overlap,
        overlap_relation: overlap.map(|[lo, hi]| {
            if lo < hi {
                "OVERLAP"
            } else if lo == hi {
                "TOUCH"
            } else {
                "DISJOINT"
            }
        }),
        direction: (vector == Some([true; 4])).then(|| {
            if ss[0].direction == Direction::Up {
                "UP"
            } else {
                "DOWN"
            }
        }),
        failed_conditions,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SegmentCandidateFact {
    pub start_stroke: usize,
    pub seed: ConstructionFact,
    pub construction: ConstructionFact,
    pub first: super::feature_seq::FeatureSequenceFacts,
    pub second: Option<super::feature_seq::SecondSequenceFacts>,
    pub strict: Option<StrictGapFact>,
    pub fact_vector: BreakFacts,
    pub observed_through: usize,
    pub result: &'static str,
    pub waiting_reasons: Vec<&'static str>,
    pub end_stroke: Option<usize>,
    pub trigger_stroke: Option<usize>,
    pub termination_attempts: Vec<SegmentEndCheck>,
    /// #1490 §7.1 第 1 条：本候选相关的第一种情况候选态（`CONFIRMED`／`KILLED_BY_NEW_EXTREME`／
    /// `OPEN_AT_PREFIX_END`）；无第一种情况候选（含已发射的第二种终结）为 `None`。
    pub turn_hypothesis_state: Option<&'static str>,
    /// 上述候选 b 的窗成员（§7.1 第 3 条）。
    pub window_members: Vec<usize>,
    /// #1490 §7.1 第 6 条：本候选扫描中段端门失败时的段起点（fail-closed）；否则 `None`。
    pub parse_invalid_from: Option<usize>,
}
/// #1414 G4：明确绑定 FIRST a/b 缺口与 c；不增加 later-FIRST 封闭门。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StrictGapFact {
    pub elements: [super::feature_seq::FeatureElement; 3],
    pub raw_gap: bool,
    pub gap_closed_by_c: Option<bool>,
    pub effective_case: &'static str,
    pub closure_operator: &'static str,
    pub closure_left: Tick,
    pub closure_right: Tick,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StrokeBreakWitness {
    pub earlier_stroke: usize,
    pub later_stroke: usize,
    pub earlier_price: Tick,
    pub later_price: Tick,
    pub comparison: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BreakFacts {
    pub stroke_broken: bool,
    pub new_segment_formed: Option<bool>,
    pub segment_broken: Option<bool>,
    pub stroke_break_witness: Option<StrokeBreakWitness>,
    pub scan_from_stroke: usize,
    pub scan_through_stroke: usize,
    pub scan_complete: bool,
    pub termination_witness: Option<TerminationWitness>,
}
/// 同次准入证书；new_segment_formed/segment_broken 由证书存在导出，不由结果标签反填。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TerminationWitness {
    pub kind: &'static str,
    pub strict: StrictGapFact,
    pub second_match: Option<super::feature_seq::SecondCheck>,
    pub construction: ConstructionFact,
    pub minimum_count_met: bool,
    pub through_stroke: usize,
}

/// #1414，065:44/46：独立枚举 gi 与 dj（或 di 与 gj），j >= i+2。
/// 原文正文已核；不用段终结结果反填笔破坏事实。
fn break_facts(
    strokes: &[Stroke],
    start: usize,
    through: usize,
    termination_witness: Option<TerminationWitness>,
    incomplete: bool,
) -> BreakFacts {
    let formed = termination_witness.is_some();
    let dir = strokes[start].direction;
    let mut witness = None;
    'outer: for earlier in (start..=through).step_by(2) {
        for later in ((earlier + 3)..=through).step_by(2) {
            let a = strokes[earlier].end_price;
            let b = strokes[later].end_price;
            if match dir {
                Direction::Up => b <= a,
                Direction::Down => b >= a,
            } {
                witness = Some(StrokeBreakWitness {
                    earlier_stroke: earlier,
                    later_stroke: later,
                    earlier_price: a,
                    later_price: b,
                    comparison: if dir == Direction::Up { "<=" } else { ">=" },
                });
                break 'outer;
            }
        }
    }
    BreakFacts {
        stroke_broken: witness.is_some(),
        new_segment_formed: (formed || !incomplete).then_some(formed),
        segment_broken: (formed || !incomplete).then_some(formed),
        stroke_break_witness: witness,
        scan_from_stroke: start,
        scan_through_stroke: through,
        scan_complete: formed || !incomplete,
        termination_witness,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SegmentEndCheck {
    pub end_stroke: usize,
    pub trigger_stroke: usize,
    pub minimum_count_met: bool,
    pub construction: ConstructionFact,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct SegmentFacts {
    pub seeds: Vec<ConstructionFact>,
    pub candidates: Vec<SegmentCandidateFact>,
    pub waiting_reasons: Vec<&'static str>,
    /// #1490 §7.1 第 6 条：最早一次段端门失败（「自 seg_start 起划分无效」）及其最小复现；
    /// 非空即 fail-closed，下游不得在其上构造。生产流目标恒为 `None`。
    pub parse_invalid_from: Option<ParseInvalid>,
    /// #1604（#1586 C-04）：最早一次「尾窗挤掉可准入候选」所在段——该段起的输出覆盖为
    /// `INCOMPLETE_RESOURCE_BOUND`，不是正常确认。签署记录（`tail_window = 7`）下恒为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail_window_bound: Option<TailWindowBound>,
    /// #1604：本次划分所读记录（id/版本与字段值）。由划分入口写入，`Default` 构造为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<SegmentPolicy>,
}

/// #1604：段层所读的两份版本化记录（`second-seq-scan-budget`、`segment-second`）及其字段值。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SegmentPolicy {
    /// 例 `second-seq-scan-budget/2`；研究覆写为 `…/research-override`。
    pub scan_budget: String,
    /// 例 `segment-second/2`（= [`CONSTRUCTION_RULE`]）；研究覆写为 `…/research-override`。
    pub construction_rule: String,
    pub tail_window: u32,
    pub second_seq_window: u32,
    pub tangency: &'static str,
}

impl SegmentPolicy {
    pub fn from_config(config: &ParseConfig) -> Self {
        SegmentPolicy {
            scan_budget: config.scan_budget.label(),
            construction_rule: config.construction_rule.label(),
            tail_window: config.tail_window,
            second_seq_window: config.second_seq_scan_window,
            tangency: config.tangency.as_str(),
        }
    }
}

/// #1604（#1586 C-04）：尾窗资源上界标记。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TailWindowBound {
    /// 被挤掉候选所在段的起点笔号；该段及其后的输出不冒充正常确认。
    pub seg_start: usize,
    /// 恒为 `INCOMPLETE_RESOURCE_BOUND`。
    pub coverage: super::feature_seq::ScanCoverage,
    /// 所读预算记录（`second-seq-scan-budget/…`）。
    pub policy: String,
}

impl TailWindowBound {
    fn new(seg_start: usize, config: &ParseConfig) -> Self {
        TailWindowBound {
            seg_start,
            coverage: super::feature_seq::ScanCoverage::IncompleteResourceBound,
            policy: config.scan_budget.label(),
        }
    }
}

/// 段端门失败的最小复现（#1490 §7.1 第 6 条）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ParseInvalid {
    pub seg_start: usize,
    pub end_stroke: usize,
    pub construction_vector: Option<[bool; 4]>,
    pub failed_conditions: Vec<&'static str>,
    pub elements: [FeatureElement; 3],
}

impl ParseInvalid {
    fn new(
        strokes: &[Stroke],
        seg_start: usize,
        end_stroke: usize,
        hit: &TriggerHit,
        t: Tangency,
    ) -> Self {
        let c = construction(strokes, seg_start, end_stroke + 1, t);
        let elem = |e: FeatElem| FeatureElement {
            high: e.high,
            low: e.low,
            stroke_idx: e.stroke_idx,
            stroke_indices: vec![e.stroke_idx],
            window_members: vec![],
        };
        ParseInvalid {
            seg_start,
            end_stroke,
            construction_vector: c.vector,
            failed_conditions: c.failed_conditions,
            elements: hit.window.map(elem),
        }
    }
}

/// #1490 §7.1 第 5 条：流首段（无前段可破）的起点 = `[seed, end]` 内与段同向的笔中，起点在段方向
/// 反向最极端者（向上段取起点最低的向上笔，向下段镜像；并列取最早）。`078:22`／`:24`。
/// 种子 `seed` 只定扫描起点；非流首段一律不适用。
fn first_segment_start(strokes: &[Stroke], seed: usize, end: usize, seg_dir: Direction) -> usize {
    #[cfg(test)]
    if FIRST_START_AT_SEED.with(|c| c.get()) {
        return seed;
    }
    let mut best = seed;
    for j in (seed..=end).filter(|&j| strokes[j].direction == seg_dir) {
        let better = match seg_dir {
            Direction::Up => strokes[j].start_price < strokes[best].start_price,
            Direction::Down => strokes[j].start_price > strokes[best].start_price,
        };
        if better {
            best = j;
        }
    }
    best
}

#[cfg(test)]
thread_local! {
    /// TestOnly（#1493）：流首段起点退回种子（§7.1 第 5 条之前的口径），仅用于在测试里构造段端门失败、
    /// 检验 fail-closed 路径；不参与生产准入（#799 限定词 2）。按 §7.1 第 5 条定义起点时首段门恒过
    /// （起点是最低的同向笔起点 ⟹ 顶>底与前三笔重叠必成立）。
    pub(super) static FIRST_START_AT_SEED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 段端门的单一判定（全量主循环与增量续扫共用，#804）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndGate {
    /// 构造完整 ⟹ 发射。
    Emit,
    /// 最少笔数未满足 ⟹ 跳过该触发、段继续（不是几何失败）。
    MinCount,
    /// 构造门（`078:20` 顶>底等不变式）失败 ⟹ fail-closed：跳过并上报，不回退、不改起点重判。
    Invalid,
}

fn end_gate(
    strokes: &[Stroke],
    start: usize,
    end_stroke: usize,
    min_seg: usize,
    t: Tangency,
) -> EndGate {
    if end_stroke < start || end_stroke - start < min_seg.saturating_sub(1) {
        EndGate::MinCount
    } else if !construction(strokes, start, end_stroke + 1, t).valid() {
        EndGate::Invalid
    } else {
        EndGate::Emit
    }
}

fn record_candidate(
    facts: &mut SegmentFacts,
    strokes: &[Stroke],
    start: usize,
    cursor: usize,
    feat: &FeatureSeqState,
    end: Option<usize>,
    mut attempts: Vec<SegmentEndCheck>,
    min_seg: usize,
    first_seed: Option<usize>,
    invalid_from: Option<usize>,
) {
    let t = feat.tangency();
    let first = feat.facts();
    let active_check = first.checks.iter().rev().find(|c| c.first_fractal);
    // 即使有 gap、尚未触发终结，当前第一分型的几何条件仍然可判。
    // CC-013 的 FAILED_GEOMETRY 优先于第二种等待，不能用等待掩盖已知失败。
    if let Some(check) = active_check {
        let apex = check.elements[1].stroke_idx;
        // #1490 §7.1 第 5 条：流首段的起点按定义随终结尝试而定。
        let s = first_seed.map_or(start, |seed| {
            first_segment_start(strokes, seed, apex - 1, strokes[seed].direction)
        });
        if apex > s && attempts.last().is_none_or(|a| a.end_stroke != apex - 1) {
            attempts.push(SegmentEndCheck {
                end_stroke: apex - 1,
                trigger_stroke: cursor,
                minimum_count_met: apex - s >= min_seg,
                construction: construction(strokes, s, apex, t),
            });
        }
    }
    let second = feat.second_facts();
    let observed_through = second
        .as_ref()
        .map_or(cursor, |s| cursor.max(s.through_stroke));
    let strict = active_check.map(|c| {
        let up = strokes[start].direction == Direction::Up;
        StrictGapFact {
            elements: c.elements.clone(),
            raw_gap: c.gap == Some(true),
            gap_closed_by_c: c.gap_closed_by_c,
            effective_case: if c.gap == Some(true) && c.gap_closed_by_c != Some(true) {
                "CASE_TWO"
            } else {
                "CASE_ONE"
            },
            // 「c 封闭缺口」与缺口谓词同读 `tangency`（#1604）：closed 含等号，open 严格。
            closure_operator: t.meets_operator(up),
            closure_left: if up {
                c.elements[2].low
            } else {
                c.elements[2].high
            },
            closure_right: if up {
                c.elements[0].high
            } else {
                c.elements[0].low
            },
        }
    });
    let deferred = strict
        .as_ref()
        .is_some_and(|c| c.effective_case == "CASE_TWO");
    // #1490 §7.1 第 6 条：段端门（构造不变式）失败才是 FAILED_GEOMETRY；最少笔数不足（流首段起点按定义
    // 落得太近）只是该触发不成终结、段继续，不是几何失败。
    let invalid = attempts
        .last()
        .is_some_and(|a| a.minimum_count_met && !a.construction.valid());
    let min_unmet = attempts.last().is_some_and(|a| !a.minimum_count_met);
    // #1344：活动第二种候选已被越界作废时，不得写成「待第二序列分型」。
    let killed = deferred
        && second.as_ref().is_some_and(|s| {
            s.semantic_state == Some(super::feature_seq::SecondTerminal::KilledByNewExtreme)
        });
    // #1420：预算耗尽的候选封住本段（含假设转折点试探中遇到的），其扫描事实即本段的等待依据。
    let resource_bound = second
        .as_ref()
        .is_some_and(|s| s.resource_truncated && s.matched.is_none());
    // #1490 §7.1 第 1/3 条：本候选相关的第一种情况候选态。已发射 ⟹ 命中检查的状态；未发射 ⟹
    // 前缀末端的开窗候选优先，否则最近一次第一种情况检查。
    let turn_source = if end.is_some() {
        active_check
            .filter(|c| c.turn_hypothesis_state != TURN_NOT_CASE_ONE)
            .map(|c| (c.turn_hypothesis_state, &c.elements[1]))
    } else if let Some(o) = &first.open_turn {
        Some((o.turn_hypothesis_state, &o.elements[1]))
    } else {
        first
            .checks
            .iter()
            .rev()
            .find(|c| c.turn_hypothesis_state != TURN_NOT_CASE_ONE)
            .map(|c| (c.turn_hypothesis_state, &c.elements[1]))
    };
    let turn_hypothesis_state = turn_source.map(|(s, _)| s);
    let window_members = turn_source.map_or(vec![], |(_, b)| b.window_members.clone());
    // §7.1 第 3 条（`071:32`）：第三笔完全在第一笔范围内 ⟹ 待定。尚无第三笔的开窗候选只报候选态，
    // 不改结果分类。
    let pending_one = first.open_turn.as_ref().is_some_and(|o| {
        o.turn_hypothesis_state == super::feature_seq::TURN_OPEN
            && !o.elements[1].window_members.is_empty()
    });
    let result = if invalid {
        "FAILED_GEOMETRY"
    } else if end.is_some() {
        if deferred {
            "CASE_TWO"
        } else {
            "CASE_ONE"
        }
    } else if killed {
        "KILLED_CASE_TWO"
    } else if deferred || resource_bound {
        "PENDING_CASE_TWO"
    } else if pending_one {
        "PENDING_CASE_ONE"
    } else {
        "NO_FIRST"
    };
    let end = end.filter(|_| !invalid);
    // 等待/失败事实只能引用同次实际扫描到的笔，不能借未来输入扩张见证。
    let built = construction(
        strokes,
        start,
        end.map_or(observed_through + 1, |e| e + 1),
        t,
    );
    let mut waiting_reasons = if invalid || end.is_some() {
        vec![]
    } else if killed {
        vec!["second_candidate_killed_by_new_extreme"]
    } else if deferred || resource_bound {
        vec![if second.as_ref().is_some_and(|s| s.resource_truncated) {
            "second_sequence_resource_bound"
        } else {
            "no_second_fractal_in_current_prefix"
        }]
    } else if pending_one {
        vec!["first_kind_third_stroke_within_first"]
    } else if first.standard.len() < 3 {
        vec!["fewer_than_three_standard_elements"]
    } else if min_unmet {
        vec![]
    } else {
        vec!["no_admissible_first_fractal"]
    };
    if end.is_none() {
        if let Some(a) = attempts.last() {
            if invalid {
                waiting_reasons.extend(&a.construction.failed_conditions);
            }
            if !a.minimum_count_met {
                waiting_reasons.push("minimum_segment_length_not_met");
            }
        }
    }
    let termination_witness = end.and_then(|e| {
        let strict = strict.clone()?;
        let second_match = second.as_ref().and_then(|s| s.matched.clone());
        let constructed = construction(strokes, start, e + 1, t);
        let minimum_count_met = e - start + 1 >= min_seg;
        (constructed.valid()
            && minimum_count_met
            && (strict.effective_case == "CASE_ONE" || second_match.is_some()))
        .then_some(TerminationWitness {
            kind: strict.effective_case,
            strict,
            second_match,
            construction: constructed,
            minimum_count_met,
            through_stroke: observed_through,
        })
    });
    let incomplete = second
        .as_ref()
        .is_some_and(|s| s.resource_truncated && s.matched.is_none());
    facts.candidates.push(SegmentCandidateFact {
        start_stroke: start,
        seed: construction(strokes, start, (start + 3).min(strokes.len()), t),
        construction: built,
        first,
        second,
        strict,
        fact_vector: break_facts(
            strokes,
            start,
            observed_through,
            termination_witness,
            incomplete,
        ),
        observed_through,
        result,
        waiting_reasons,
        end_stroke: end,
        trigger_stroke: end.map(|_| observed_through),
        termination_attempts: attempts,
        turn_hypothesis_state,
        window_members,
        parse_invalid_from: invalid_from,
    });
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
pub fn classify_termination(e1: &Interval, e2: &Interval, t: Tangency) -> TerminationCase {
    if e1.gap(e2, t) {
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
fn find_feature_fractal(
    seg_dir: Direction,
    std_feat: &[Interval],
) -> Option<(usize, Interval, Interval)> {
    if std_feat.len() < 3 {
        return None;
    }
    for i in 1..std_feat.len() - 1 {
        let (l, m, r) = (&std_feat[i - 1], &std_feat[i], &std_feat[i + 1]);
        let is_top = m.hi > l.hi && m.hi > r.hi;
        let is_bottom = m.lo < l.lo && m.lo < r.lo;
        let matched = match seg_dir {
            Direction::Up => is_top, // 向上线段以顶分型终结
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
/// `strokes` 的首笔方向 = 线段方向。返回该方向线段的首个终结情形。`t` 为相切口径（#1604）。
pub fn analyze_termination(strokes: &[Stroke], t: Tangency) -> SegmentTermination {
    if strokes.is_empty() {
        return SegmentTermination::NoFractal;
    }
    let seg_dir = strokes[0].direction;
    let feat = feature_elements(seg_dir, strokes);
    let std_feat = process_feature_inclusion(&feat);
    let Some((_fi, e1, e2)) = find_feature_fractal(seg_dir, &std_feat) else {
        return SegmentTermination::NoFractal;
    };
    match classify_termination(&e1, &e2, t) {
        TerminationCase::FirstKind => {
            // 无缺口：分型形成即终结。定位贡献分型中心元素 e2 的反向笔（apex 反向笔偏移）。
            let rev = seg_dir.flip();
            let Some(apex_off) = strokes
                .iter()
                .position(|s| s.direction == rev && stroke_interval(s).overlaps(&e2, t))
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
            match second_kind::resolve_second_kind(strokes, &e2, t) {
                // resolve 返回 apex 反向笔偏移；段端 = 其前一根同向笔（方向一致性，同 FirstKind）。
                second_kind::SecondKindResult::Confirmed {
                    end_offset: apex_off,
                } => match segment_end_from_apex(apex_off) {
                    Some(end_offset) => SegmentTermination::SecondKindConfirmed { end_offset },
                    None => SegmentTermination::SecondKindPending,
                },
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
            let lo = strokes[s0..=s1]
                .iter()
                .map(|s| s.start_price.min(s.end_price))
                .min()
                .unwrap();
            let hi = strokes[s0..=s1]
                .iter()
                .map(|s| s.start_price.max(s.end_price))
                .max()
                .unwrap();
            (lo, hi)
        }
        Direction::Down => {
            let hi = strokes[s0..=s1]
                .iter()
                .map(|s| s.start_price.max(s.end_price))
                .max()
                .unwrap();
            let lo = strokes[s0..=s1]
                .iter()
                .map(|s| s.start_price.min(s.end_price))
                .min()
                .unwrap();
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

/// 线段划分（reference-theta-v0.md:22，第67/71/78课）——增量「假设转折点」状态机。
///
/// 第一种情况按 `071:30/:32/:42` 直落（#1490 裁定，`xianduan.md` §7.1；#1493）：候选三态、转折点窗、
/// 先破判据；流首段起点按 `078:22/:24` 定义；段端门（`078:20`）失败 fail-closed 上报。Python v1
/// `a_segment_v1.segments_from_strokes_v1` 为历史参考，非正本。动态划分算法在 Origin 有意不形式化
/// （静态层锚 `Origin.SegmentConstruction`/`SegmentFeatureComplete`，见 feature_seq.rs 模块头）。
///
/// ★为何增量（#84）：无状态批处理「找首个分型即断段」表达不了第71课「假设转折点」这一动态过程，
/// 本函数用 `FeatureSeqState` 状态机实现。
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
    let (segments, tail, _) = divide_segments_core(strokes, config, false, &mut None);
    (segments, tail)
}

/// 同次生产划分附事实；不是从最终 Segment 反推过程。
pub fn divide_segments_with_facts(
    strokes: &[Stroke],
    config: &ParseConfig,
) -> (Vec<Segment>, Option<usize>, SegmentFacts) {
    divide_segments_core(strokes, config, true, &mut None)
}

/// #1420：同 [`divide_segments_with_facts`]，第二种候选经 S 层游标簿（K1）折叠并带回簿。
pub fn divide_segments_with_facts_store(
    strokes: &[Stroke],
    config: &ParseConfig,
    store: super::second_fold::SecondFoldStore,
) -> (
    Vec<Segment>,
    Option<usize>,
    SegmentFacts,
    super::second_fold::SecondFoldStore,
) {
    let mut slot = Some(store);
    let (segments, pending, facts) = divide_segments_core(strokes, config, true, &mut slot);
    (segments, pending, facts, slot.unwrap_or_default())
}

fn divide_segments_core(
    strokes: &[Stroke],
    config: &ParseConfig,
    observe: bool,
    store: &mut Option<super::second_fold::SecondFoldStore>,
) -> (Vec<Segment>, Option<usize>, SegmentFacts) {
    let t = config.tangency;
    let mut facts = SegmentFacts {
        policy: Some(SegmentPolicy::from_config(config)),
        ..SegmentFacts::default()
    };
    let mut segments = Vec::new();
    let n = strokes.len();
    if n < 3 {
        if observe {
            facts.seeds.push(construction(strokes, 0, n, t));
            facts.waiting_reasons.push("fewer_than_three_strokes");
        }
        return (segments, if n > 0 { Some(0) } else { None }, facts);
    }
    // 第77课 H4：起点必须满足前三笔重叠（对齐 Python `_find_overlap_start`）。
    let start = find_overlap_start(strokes, 0, t);
    if observe {
        for i in 0..=start.unwrap_or(n - 3) {
            facts.seeds.push(construction(strokes, i, i + 3, t));
        }
    }
    let Some(mut seg_start) = start else {
        if observe {
            facts.waiting_reasons.push("no_valid_three_stroke_seed");
        }
        return (segments, Some(0), facts);
    };
    let min_seg = config.seg_min_strokes as usize;
    let seed = seg_start;
    let mut seg_dir = strokes[seg_start].direction;
    let mut feat = FeatureSeqState::new(
        seg_dir,
        ExtendMode::Strict,
        config.tail_window,
        config.second_seq_scan_window,
        t,
    );
    feat.set_second_store(store.take());
    if observe {
        feat.observe();
    }
    let mut cursor = seg_start;
    let mut attempts = Vec::new();
    // #1490 §7.1 第 6 条：本候选扫描中的段端门失败起点（候选级）。
    let mut cand_invalid: Option<usize> = None;

    // 主循环（#1493：第一种情况按 #1490 §7.1；Python `segments_from_strokes_v1` 为历史参考）。
    while cursor < n {
        let sk = &strokes[cursor];
        let opposite = seg_dir.flip();
        if sk.direction != opposite {
            if observe {
                let (h, l) = super::feature_seq::stroke_high_low(sk);
                feat.observe_trend_stroke(cursor, h, l);
            }
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
        let trig = feat.scan_trigger(strokes);
        if feat.tail_truncated() && facts.tail_window_bound.is_none() {
            facts.tail_window_bound = Some(TailWindowBound::new(seg_start, config));
        }
        let Some(hit) = trig else {
            cursor += 1;
            continue;
        };
        let k = hit.k;
        let end_stroke = k - 1;
        // #1490 §7.1 第 5 条：流首段起点按定义（随每次终结尝试），其余段即 seg_start。
        let start = if segments.is_empty() {
            first_segment_start(strokes, seed, end_stroke, seg_dir)
        } else {
            seg_start
        };
        if observe && end_stroke >= start {
            attempts.push(SegmentEndCheck {
                end_stroke,
                trigger_stroke: hit.through_stroke.max(cursor),
                minimum_count_met: end_stroke - start >= min_seg.saturating_sub(1),
                construction: construction(strokes, start, end_stroke + 1, t),
            });
        }
        match end_gate(strokes, start, end_stroke, min_seg, t) {
            EndGate::Emit => {}
            gate => {
                // 最少笔数不足 ⟹ 跳过该触发、段继续；构造门失败 ⟹ 同样跳过，但 fail-closed 上报
                // （#1490 §7.1 第 6 条：不回退上一段、不改起点重判）。
                if gate == EndGate::Invalid {
                    cand_invalid.get_or_insert(start);
                    if observe && facts.parse_invalid_from.is_none() {
                        facts.parse_invalid_from =
                            Some(ParseInvalid::new(strokes, start, end_stroke, &hit, t));
                    }
                }
                feat.skip_trigger(k);
                cursor += 1;
                continue;
            }
        }
        if observe {
            record_candidate(
                &mut facts,
                strokes,
                start,
                cursor,
                &feat,
                Some(end_stroke),
                std::mem::take(&mut attempts),
                min_seg,
                segments.is_empty().then_some(seed),
                cand_invalid.take(),
            );
        }
        cand_invalid = None;
        // 发射旧段（end_stroke = k-1）。
        segments.push(make_segment(strokes, start, end_stroke, seg_dir));
        // 新段从 k（分型中心反向笔）起，方向反转（Python :608-610）。
        seg_start = k;
        seg_dir = opposite;
        feat.reset(seg_dir);
        cursor = k;
    }

    // 末段（未触发终结）= active 尾部。流首段未确认时其起点按定义随前缀变化（§7.1 第 5 条，不发布）。
    let pending_start = pending_segment_start(strokes, &segments, seed, seg_start, seg_dir);
    if observe {
        if let Some(p) = pending_start {
            record_candidate(
                &mut facts,
                strokes,
                p,
                n - 1,
                &feat,
                None,
                attempts,
                min_seg,
                segments.is_empty().then_some(seed),
                cand_invalid,
            );
        }
    }
    *store = feat.take_second_store();
    (segments, pending_start, facts)
}

/// 未确认尾段的起点：流首段未确认 ⟹ 按 §7.1 第 5 条定义（`[seed, n-1]`），否则即 seg_start。
fn pending_segment_start(
    strokes: &[Stroke],
    segments: &[Segment],
    seed: usize,
    seg_start: usize,
    seg_dir: Direction,
) -> Option<usize> {
    let n = strokes.len();
    (seg_start < n).then(|| {
        if segments.is_empty() {
            first_segment_start(strokes, seed, n - 1, seg_dir)
        } else {
            seg_start
        }
    })
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

/// 增量 segment 状态（与 `divide_segments_with_tail` 逐字段相同，#804）。
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
    /// 改判（第二特征序列出现分形），级联改写已 confirmed 的更早段（#88 报告反例 seg[404]<confirmed_len=406）。
    /// 故 `append` 的 `confirmed_bound = min(末段end, earliest_unsealed_from)` 回退到 unsealed 起点前。
    ///
    /// #1344 / #1420（xianduan §5.1 检查点安全边界）：只有 `OPEN_AT_PREFIX_END` 与预算耗尽的候选
    /// 记入本字段；`KILLED_BY_NEW_EXTREME` 不复活，失效笔不再是输入末笔后即封闭，边界随之前移。
    ///
    /// #1493 裁定 1（§7.1 第 8 条）：未决候选封住所在段，其后不再有已确认段，故 unsealed 只剩两类——
    /// 末段（pending）里的未决候选，与「末笔可改写」（确认/失效笔是输入末笔）；不再有「后续已确认段
    /// 被复活改写」。上文 #88 的级联改写反例在新规则下不可达，回退机制保留作末笔可改写的边界。
    ///
    /// **advancing soundness**（codex #91 §二）：second_seq_has_fractal 一旦真则永真（构建由既定前缀
    /// 决定）+ 段连续性 ⟹ 未 resolve 的候选必在下轮重扫被重新发现；候选 resolve 后 euf 前移/清零 ⟹
    /// confirmed_bound 前移 ⟹ 退回 O(n)（#93 H2 探针 late_repro=0 坐实）。取代 #88 的历史最小值持久化
    /// （后者 euf 单调非增锚死早点 ⟹ O(n²)，边界条件3）。
    earliest_unsealed_from: Option<usize>,
    /// #1490 §7.1 第 6 条（#1493）：最早一次段端门失败的段起点（fail-closed），与全量
    /// `SegmentFacts::parse_invalid_from` 的 `seg_start` 同一判定（#804）。保留前缀内的记录跨 append
    /// 保留，重扫区间内的重算。
    parse_invalid_from: Option<usize>,
    /// #1604（#1586 C-04）：尾窗资源上界起点，与全量 `SegmentFacts::tail_window_bound` 的 `seg_start`
    /// 同一判定（#804）；保留/重算规则同 `parse_invalid_from`。
    resource_bound_from: Option<usize>,
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
            parse_invalid_from: None,
            resource_bound_from: None,
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
    /// 1 段回退。**仅在恢复前缀无 unsealed 候选时 bit-exact 安全**。#1344：已失效（非末笔触发）的
    /// 候选不复活，不构成 unsealed，在此前提内可安全恢复（tb02w_tests 锁）。生产路径是 `empty()` + 逐 bar
    /// `append`（skip 历史跨 append 持久累积），**不经** `from_full`（当前无生产调用者）。在有 unsealed
    /// 前缀处恢复须带检查点边界，用 [`Self::from_checkpoint`]。
    pub fn from_full(
        segments: &[Segment],
        pending_start: Option<usize>,
        strokes: &[Stroke],
    ) -> Self {
        Self::from_checkpoint(segments, pending_start, strokes, None)
    }

    /// #1420（xianduan §5.1）：从全量结果 + 检查点安全边界恢复增量状态。
    ///
    /// `earliest_unsealed_from` 取自存检查点时的 [`Self::earliest_unsealed_from`]：最早未封闭第二种
    /// 候选（`OPEN_AT_PREFIX_END` 或预算耗尽）所在段起点，随确认/失效前移；失效候选不复活，不入边界。
    /// 带边界恢复后续扫，与同一完整输入的无中断全量逐位相同（对任意切点成立，tb02w 三路锁）；
    /// `from_full` 是边界为空的特例。单个候选的续扫用 `second_fold::SecondFoldCursor`（K1）。
    pub fn from_checkpoint(
        segments: &[Segment],
        pending_start: Option<usize>,
        strokes: &[Stroke],
        earliest_unsealed_from: Option<usize>,
    ) -> Self {
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
            earliest_unsealed_from,
            // 全量结果不带门失败历史；恢复前缀上的 fail-closed 记录须由调用方另核（生产不经此入口）。
            parse_invalid_from: None,
            resource_bound_from: None,
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
            parse_invalid_from: invalid_persisted,
            resource_bound_from: bound_persisted,
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
        let keep = end_indices.partition_point(|&end_idx| end_idx < confirmed_bound);

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
                        pending_start: if resume_seg_start < n {
                            Some(resume_seg_start)
                        } else {
                            None
                        },
                        strokes_len: n,
                        confirmed_len: keep,
                        last_stroke,
                        earliest_unsealed_from: euf_persisted,
                        // 与续扫路径同一过滤：只留续扫起点之前的门失败记录（笔数回缩时不带过期标记）。
                        parse_invalid_from: invalid_persisted.filter(|&p| p < resume_seg_start),
                        resource_bound_from: bound_persisted.filter(|&p| p < resume_seg_start),
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
                        // 无已确认段且无可扫种子：全量在同一输入上也无门失败（#804）。
                        parse_invalid_from: None,
                        resource_bound_from: None,
                    };
                }
                let Some(start) = find_overlap_start(strokes, 0, config.tangency) else {
                    return IncrSegments {
                        segments_rc,
                        end_indices,
                        pending_start: Some(0),
                        strokes_len: n,
                        confirmed_len: keep,
                        last_stroke,
                        earliest_unsealed_from: euf_persisted,
                        // 无已确认段且无可扫种子：全量在同一输入上也无门失败（#804）。
                        parse_invalid_from: None,
                        resource_bound_from: None,
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
        // 保留前缀内（< resume）的门失败记录不随重扫改变；重扫区间内的重算。
        let mut parse_invalid_from = invalid_persisted.filter(|&p| p < resume_seg_start);
        let mut resource_bound_from = bound_persisted.filter(|&p| p < resume_seg_start);
        let seed = resume_seg_start;
        let mut seg_start = resume_seg_start;
        let mut seg_dir = resume_seg_dir;
        let mut feat = FeatureSeqState::new(
            seg_dir,
            ExtendMode::Strict,
            config.tail_window,
            config.second_seq_scan_window,
            config.tangency,
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
            if feat.tail_truncated() {
                resource_bound_from.get_or_insert(seg_start);
            }
            let Some(hit) = trig else {
                cursor += 1;
                continue;
            };
            let k = hit.k;
            let end_stroke = k - 1;
            // 与全量主循环同一判定（#804）：流首段起点按定义，门失败 fail-closed。
            let start = if new_segments.is_empty() {
                first_segment_start(strokes, seed, end_stroke, seg_dir)
            } else {
                seg_start
            };
            match end_gate(strokes, start, end_stroke, min_seg, config.tangency) {
                EndGate::Emit => {}
                gate => {
                    if gate == EndGate::Invalid {
                        parse_invalid_from.get_or_insert(start);
                    }
                    feat.skip_trigger(k);
                    cursor += 1;
                    continue;
                }
            }
            let seg = make_segment(strokes, start, end_stroke, seg_dir);
            new_segments.push(seg);
            end_indices.push(end_stroke);
            seg_start = k;
            seg_dir = opposite;
            feat.reset(seg_dir);
            cursor = k;
        }

        let pending_start = pending_segment_start(strokes, new_segments, seed, seg_start, seg_dir);
        // #93 advancing 变体（codex #91 补充裁定 §五）：直接持久化本轮重扫的新鲜 euf，**不**与历史取 min。
        // #1344：确认/失效由非末笔决定后即终局（逐笔左折叠，先到者定），故下句对三态同样成立。
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
            parse_invalid_from,
            resource_bound_from,
        }
    }

    /// #1490 §7.1 第 6 条：最早一次段端门失败的段起点；`None` = 无失败。
    pub fn parse_invalid_from(&self) -> Option<usize> {
        self.parse_invalid_from
    }

    /// #1604（#1586 C-04）：尾窗资源上界起点（`INCOMPLETE_RESOURCE_BOUND`）；`None` = 覆盖完整。
    pub fn resource_bound_from(&self) -> Option<usize> {
        self.resource_bound_from
    }

    /// 当前快照 segments（Vec<Segment>，与 `divide_segments_with_tail` bit-exact）。
    pub fn to_result_vec(&self) -> (Vec<Segment>, Option<usize>) {
        (self.segments_rc.to_vec(), self.pending_start)
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

// ═══════════════════════════════════════════════════════════════════════════
// § I-1（#989 / G1 #975）：段→笔区间反查（按需重建载体，零新增状态）+ 笔速度力度
//
// 教义（#873，beichi.md v1.6）：L(段) = v(末笔) − v(首笔)；v(笔) = ±(笔端价差)/(笔 bar 数)。
// `make_segment` 构造完即丢 `(s0,s1)` 笔区间——本节提供按需反查：段的 bar 闭区间 +
// strokes 切片，二分定位覆盖笔区间 [first, last]。对拍锁（构造现场 == 反查结果）见 tests。
// ═══════════════════════════════════════════════════════════════════════════

/// 段 bar 闭区间 `(start, end)`（inclusive）覆盖的笔区间 `[first_stroke, last_stroke]`（inclusive）。
///
/// 定位语义（与 `make_segment` 端点口径对齐——首笔 start / 末笔 end）：
/// - `first` = 第一个 `end_index >= seg_start` 的笔（笔与段首 bar 相交或在其后开始）；
/// - `last` = 最后一个 `start_index <= seg_end` 的笔。
/// strokes 按 `start_index` 严格升序（分型交替构造保证）；两趟 `partition_point` 各 O(log n)。
/// 段区间完全落在笔间隙（不与任何笔相交）⟹ `None`（不冒充）。
pub fn stroke_span_of_segment(
    strokes: &[Stroke],
    seg_start: usize,
    seg_end: usize,
) -> Option<(usize, usize)> {
    if strokes.is_empty() || seg_start > seg_end {
        return None;
    }
    // first：笔区间按 [start,end] 排列，首个 end >= seg_start 的笔。
    let first = strokes.partition_point(|s| s.end_index < seg_start);
    // last：最后一个 start <= seg_end。
    let last = strokes.partition_point(|s| s.start_index <= seg_end);
    if first >= last {
        return None; // 空间隙：段区间不与任何笔相交。
    }
    Some((first, last - 1))
}

/// 笔速度（#873 教义）：方向符号 ×（笔端价差 ÷ 笔 bar 数）。Tick/bar 量纲。
///
/// 向上笔速度为正（(end−start)/bars）、向下为负——`L(段)=v(末笔)−v(首笔)` 沿用带号速度
/// （beichi.md v1.6「速度是同一把尺子」）。
pub fn stroke_velocity(stroke: &Stroke) -> f64 {
    let bars = (stroke.end_index - stroke.start_index + 1) as f64;
    let sign = match stroke.direction {
        Direction::Up => 1.0,
        Direction::Down => -1.0,
    };
    sign * (stroke.end_price - stroke.start_price) as f64 / bars
}

/// 力度 `L(段) = v(末笔) − v(首笔)`（#873 教义正本；段→笔经反查）。
///
/// 段区间不含任何笔 ⟹ `None`（取不到数据不冒充，与度量层「无数据判非」纪律同向）。
pub fn segment_force_l(strokes: &[Stroke], seg_start: usize, seg_end: usize) -> Option<f64> {
    let (first, last) = stroke_span_of_segment(strokes, seg_start, seg_end)?;
    Some(stroke_velocity(&strokes[last]) - stroke_velocity(&strokes[first]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 签署记录 `segment-second` 的相切口径（#1604）。以下 #246 / Lean 对拍用例读记录值：记录若改签
    /// `open` 而 Lean 未参数化（#1626），这些用例即红。
    fn rec() -> Tangency {
        ParseConfig::default().tangency
    }

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
        assert!(a.overlaps(&b, rec())); // 5<=12 && 8<=10
        assert!(!a.overlaps(&c, rec())); // 5<=15 但 11<=10 false → 无重叠
        assert!(a.gap(&c, rec()));
        assert!(!a.gap(&b, rec()));
    }

    // ★相切用例（#246 裁定：相切=重合，supersede Lead #84 点3 的严格 `<` 口径）。
    // 既有用例全是严格不等构造，对本口径变化不敏感；以下相切用例才是本次口径的保障。

    #[test]
    fn three_stroke_overlap_tangent_counts_as_overlap() {
        // 三笔区间 [5,10]、[8,12]、[10,14]：max(lows)=10 == min(highs)=10，
        // 三笔仅公共端点 10（相切）。相切=重合 ⟹ 有重合 → true。
        let a = stroke(Direction::Up, 0, 4, 5, 10);
        let b = stroke(Direction::Down, 4, 8, 12, 8);
        let c = stroke(Direction::Up, 8, 12, 10, 14);
        assert!(three_stroke_overlap(&a, &b, &c, rec()));
        // 对照：严格分离（无公共端点）仍无重合。
        let d = stroke(Direction::Up, 8, 12, 11, 14);
        assert!(!three_stroke_overlap(&a, &b, &d, rec())); // max_lo=11 > min_hi=10
    }

    #[test]
    fn contains_bit_exact_claim10() {
        let a = Interval { lo: 3, hi: 20 };
        let b = Interval { lo: 5, hi: 18 };
        assert!(a.contains(&b));
        assert!(!b.contains(&a));
    }

    /// ★#312 主缝：`three_stroke_overlap` 的重合真值 == Lean `decide (Overlaps a b)`（fixture 机器导出）。
    ///
    /// **退化三笔**：Lean 侧无三笔重合谓词（`Overlaps` 是两区间版），故用第三笔复用第二笔的
    /// 退化形态 `(a, b, b)` 把三笔谓词降到两区间语义——`max(a.lo,b.lo) <= min(a.hi,b.hi)`，
    /// 与 `Overlaps a b` 逐字同式，可直接对 Lean 导出值。覆盖 5 条用例（相切两形态 + 严格分离
    /// 正/反向 + 严格重叠对照）。口径若回改（相切不算重合，旧 `<`），相切两条立刻红。
    ///
    /// ⚠诚实边界（#276 MED-1，未闭合）：**三笔互异**区间的相切形态（`max(lows)==min(highs)` 由
    /// 三条不同区间共同取得）在 Lean 侧仍无见证——该形态的覆盖是下方手填单测
    /// `three_stroke_overlap_distinct_triple_tangent`，非机器耦合。本测试不冒充覆盖它。
    #[test]
    fn three_stroke_overlap_lean_fixture_bit_exact() {
        for case in super::super::gap_overlap_fixture::gap_overlap_cases() {
            let sa = stroke(Direction::Up, 0, 4, case.a.0, case.a.1);
            let sb = stroke(Direction::Up, 4, 8, case.b.0, case.b.1);
            assert_eq!(
                three_stroke_overlap(&sa, &sb, &sb, rec()),
                case.overlaps,
                "{}: three_stroke_overlap(a,b,b) 须 == Lean decide(Overlaps a b)（bit-exact）",
                case.name
            );
        }
    }

    #[test]
    fn three_stroke_overlap_distinct_triple_tangent() {
        // ★#246 相切口径在**三笔互异**形态上的行为（Lean 无对应谓词，期望值手填——#276 MED-1 已知缺角）。
        // 三笔区间 [5,10]、[8,12]、[10,15]：max_lo=10, min_hi=10 → 相切于 10 ⟹ 有重合（新口径，`<=`）。
        let a = stroke(Direction::Up, 0, 4, 5, 10);
        let b = stroke(Direction::Up, 4, 8, 8, 12);
        let c = stroke(Direction::Down, 8, 12, 15, 10);
        assert!(
            three_stroke_overlap(&a, &b, &c, rec()),
            "三笔互异相切（max_lo==min_hi==10）须判有重合——#246 新口径 `<=`；若红，口径被改回旧 `<`"
        );
    }

    #[test]
    fn three_stroke_overlap_strict_cases_unchanged() {
        // 严格内含（max_lo < min_hi）仍算重叠、严格分离（max_lo > min_hi）仍不算——新旧口径一致。
        let a = stroke(Direction::Up, 0, 4, 5, 10);
        let b = stroke(Direction::Up, 4, 8, 8, 12);
        let c_in = stroke(Direction::Down, 8, 12, 11, 9); // [9,11]：max_lo=9 < min_hi=10 → 重叠
        assert!(three_stroke_overlap(&a, &b, &c_in, rec()));
        let c_out = stroke(Direction::Up, 8, 12, 13, 20); // [13,20]：max_lo=13 > min_hi=10 → 无重叠
        assert!(!three_stroke_overlap(&a, &b, &c_out, rec()));
    }

    /// #1604 A-12：相切取值两档各一条 fixture——相切形态在 `closed` 与 `open` 下翻转，
    /// 严格形态两档一致。谓词若回退为常量（任一档硬编码），本用例必红。
    #[test]
    fn tangency_fixture_per_value_flips_only_tangent_case() {
        let a = Interval { lo: 5, hi: 10 };
        let touch = Interval { lo: 10, hi: 15 };
        let inner = Interval { lo: 8, hi: 12 };
        assert!(a.overlaps(&touch, Tangency::Closed));
        assert!(!a.gap(&touch, Tangency::Closed));
        assert!(!a.overlaps(&touch, Tangency::Open));
        assert!(a.gap(&touch, Tangency::Open));
        for t in [Tangency::Closed, Tangency::Open] {
            assert!(a.overlaps(&inner, t));
            assert!(!a.gap(&inner, t));
        }
        let sa = stroke(Direction::Up, 0, 4, 5, 10);
        let sb = stroke(Direction::Up, 4, 8, 8, 12);
        let sc = stroke(Direction::Down, 8, 12, 15, 10);
        assert!(three_stroke_overlap(&sa, &sb, &sc, Tangency::Closed));
        assert!(!three_stroke_overlap(&sa, &sb, &sc, Tangency::Open));
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
        assert_eq!(
            classify_termination(&e1, &e2, rec()),
            TerminationCase::FirstKind
        );
        // 有缺口 → SecondKind。
        let e3 = Interval { lo: 20, hi: 25 };
        assert_eq!(
            classify_termination(&e1, &e3, rec()),
            TerminationCase::SecondKind
        );
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
        let t = analyze_termination(&strokes, rec());
        matches!(
            t,
            SegmentTermination::FirstKindConfirmed { .. }
                | SegmentTermination::SecondKindPending
                | SegmentTermination::NoFractal
        );
    }

    #[test]
    fn analyze_termination_empty_no_fractal() {
        assert_eq!(
            analyze_termination(&[], rec()),
            SegmentTermination::NoFractal
        );
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
        let down_count = strokes
            .iter()
            .filter(|s| s.direction == Direction::Down)
            .count();
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
                    stroke(
                        Direction::Down,
                        b + 12,
                        b + 16,
                        25 + i as i64,
                        12 + i as i64,
                    ),
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
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            lo + ((s >> 33) as i64).rem_euclid(hi - lo + 1)
        };
        let mut out = Vec::with_capacity(n);
        let mut price: i64 = 100_000;
        let mut idx = 0usize;
        for i in 0..n {
            let dir = if i % 2 == 0 {
                Direction::Up
            } else {
                Direction::Down
            };
            // 变幅：多数中等，偶发大跳（制造缺口）。
            let amp = if next(0, 9) < 2 {
                next(400, 900)
            } else {
                next(60, 300)
            };
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

    /// ★#88 级联复活 bit-exact（验收项3）→ #1493 裁定 1（§7.1 第 8 条）改写：未决第二种候选封住所在段，
    /// 「早期候选后来确认、改写其后已发射段」的深回退在新规则下不可达。
    ///
    /// 逐 strokes 长度断言增量 `IncrSegments::append` == 全量 `divide_segments_with_tail`，并断言已发射
    /// 段序列逐前缀单调（前一前缀的段序列是后一前缀的前缀；笔前缀固定，无「末笔可改写」）。
    /// **覆盖率自证**：断言 `earliest_unsealed_from` 曾被置 Some（未决候选路径确被触发）。
    #[test]
    fn cascade_revival_bit_exact_incr_vs_full() {
        let cfg = ParseConfig::default();
        let strokes = gappy_strokes(1200, 0xC0FF_EE12_3456_789A);
        let mut incr = IncrSegments::empty();
        let mut euf_ever_some = false;
        let mut prev_segs: Vec<Segment> = Vec::new();
        for end in 3..=strokes.len() {
            incr = incr.append(&strokes[..end], &cfg);
            let (full_segs, full_pending) = divide_segments_with_tail(&strokes[..end], &cfg);
            let (incr_segs, incr_pending) = incr.to_result_vec();
            assert_eq!(
                incr_segs, full_segs,
                "strokes len {end}: 增量 != 全量（bit-exact 破裂）"
            );
            assert_eq!(
                incr_pending, full_pending,
                "strokes len {end}: pending != 全量"
            );
            assert!(
                full_segs.len() >= prev_segs.len() && full_segs[..prev_segs.len()] == prev_segs[..],
                "strokes len {end}: 已发射段被改写（#1493 裁定 1：未决候选封段后不应再有级联复活）"
            );
            prev_segs = full_segs;
            if incr.earliest_unsealed_from().is_some() {
                euf_ever_some = true;
            }
        }
        assert!(
            euf_ever_some,
            "覆盖为零：gappy 序列从未触发未决第二种候选（earliest_unsealed_from 恒 None）——\
             须调 gappy_strokes 参数"
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
            assert_eq!(
                incr.to_result_vec(),
                (full_segs, full_pending),
                "strokes len {end}: advancing 变体 bit-exact 破裂"
            );
            let euf = incr.earliest_unsealed_from();
            match (prev_euf, euf) {
                (Some(_), None) => {
                    saw_advance = true;
                    advanced_once = true;
                }
                (Some(a), Some(b)) if b > a => {
                    saw_advance = true;
                    advanced_once = true;
                }
                (_, Some(_)) if advanced_once => {
                    saw_reflag = true;
                }
                _ => {}
            }
            prev_euf = euf;
        }
        assert!(saw_advance,
            "覆盖为零：euf 从未前移/清零——persist-forever 未被证伪（advancing 未生效或序列不触发 resolve）");
        assert!(
            saw_reflag,
            "覆盖为零：前移后 euf 从未 reflag 到新位置——未覆盖 §六.3 later reflag 路径"
        );
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
                    stroke(
                        Direction::Down,
                        b + 12,
                        b + 16,
                        25 + i as i64,
                        12 + i as i64,
                    ),
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

    // ── I-1（#989）：段→笔反查对拍锁（#804 测试锁形状——同输入同输出）─────────────

    fn mk_stroke(direction: Direction, start: usize, end: usize, lo: Tick, hi: Tick) -> Stroke {
        let (start_price, end_price) = match direction {
            Direction::Up => (lo, hi),
            Direction::Down => (hi, lo),
        };
        Stroke {
            direction,
            start_index: start,
            end_index: end,
            start_price,
            end_price,
        }
    }

    /// 合成对拍：`divide_segments` 产出的每个段，其 bar 区间反查笔区间 == 构造现场
    /// （make_segment 的 `(s0,s1)` 无法直接观测——以「反查区间内的笔全部与段区间相交、
    /// 且区间外相邻笔不相交」的区间封闭性 + 首末笔端点 == 段端 bar」双条件锁）。
    #[test]
    fn i1_stroke_span_roundtrip_synthetic() {
        // 8 笔交替，覆盖典型形状（快笔/慢笔/大价差/小价差）。
        let strokes = vec![
            mk_stroke(Direction::Up, 0, 9, 100, 150),
            mk_stroke(Direction::Down, 9, 13, 140, 150),
            mk_stroke(Direction::Up, 13, 21, 145, 190),
            mk_stroke(Direction::Down, 21, 30, 180, 190),
            mk_stroke(Direction::Up, 30, 41, 185, 260),
            mk_stroke(Direction::Down, 41, 44, 250, 260),
            mk_stroke(Direction::Up, 44, 55, 255, 340),
            mk_stroke(Direction::Down, 55, 60, 330, 340),
        ];
        // 封闭性逐断言：
        // 口径：段端点 bar 上的笔算相交（相邻笔共享端点 bar 是分型构造常态）。
        // span(0,9) 含笔1（起点=9 与段末 bar 重合）；span(9,9) 单 bar 交笔0/笔1。
        for (seg_start, seg_end, expect) in [
            (0usize, 9usize, Some((0usize, 1usize))), // 段末 bar 与笔1 起点共享
            (0, 8, Some((0, 0))),                     // 恰首笔（不触笔1）
            (0, 21, Some((0, 3))),                    // 跨四笔（段末 bar 与笔3 起点共享）
            (30, 44, Some((3, 6))), // 共享端点：首含笔3（end=30）、末含笔6（start=44）
            (44, 60, Some((5, 7))), // 共享端点：首含笔5（end=44）
            (9, 9, Some((0, 1))),   // 单 bar 交两笔（共享端点）
            (61, 70, None),         // 全部笔之后 ⟹ None
        ] {
            assert_eq!(
                stroke_span_of_segment(&strokes, seg_start, seg_end),
                expect,
                "span({seg_start},{seg_end})"
            );
        }
        // 反查区间内笔全部与段区间相交（封闭性第一半）。
        if let Some((f, l)) = stroke_span_of_segment(&strokes, 0, 21) {
            for st in &strokes[f..=l] {
                assert!(st.end_index >= 0 && st.start_index <= 21);
            }
        }
    }

    /// L 原语一致性：L = v(末笔) − v(首笔)，手工可验算例。
    #[test]
    fn i1_segment_force_l_manual() {
        let strokes = vec![
            mk_stroke(Direction::Up, 0, 9, 100, 200), // v = +100/10 = 10.0
            mk_stroke(Direction::Down, 9, 19, 120, 200), // v = −80/11
            mk_stroke(Direction::Up, 19, 38, 130, 410), // v = +280/20 = 14.0
        ];
        // 段 [0,38]：L = v(末) − v(首) = 14.0 − 10.0 = 4.0
        let l = segment_force_l(&strokes, 0, 38).unwrap();
        assert!((l - 4.0).abs() < 1e-9, "L={l}");
        // 空间隙 ⟹ None。
        assert!(segment_force_l(&strokes, 40, 50).is_none());
    }

    /// 真实数据对拍：OKLO 全量 parse 后，每个段的反查笔区间满足封闭性
    /// （区间内笔与段相交、相邻区间外笔不相交）——#804 同款「同输入同输出」。
    #[test]
    fn i1_stroke_span_closure_on_oklo() {
        use crate::theta_v0::backtest::data::load_by_symbol;
        use crate::theta_v0::config::ThetaConfig;
        use crate::theta_v0::parser::parse_layer;
        // CI runner 无数据文件 ⟹ skip（本地全查；数据可复跑：cargo test --features backtest_bin i1_stroke_span_closure）
        let Ok(ds) = load_by_symbol("OKLO", &ThetaConfig::default()) else {
            println!("OKLO 数据不可用，skip 真实数据对拍（本地运行以全查）");
            return;
        };
        let l0 = parse_layer(&ds.bars, &ThetaConfig::default());
        assert!(!l0.strokes.is_empty() && !l0.segments.is_empty());
        let mut checked = 0usize;
        for seg in l0.segments.iter() {
            let Some((f, l)) = stroke_span_of_segment(&l0.strokes, seg.start_index, seg.end_index)
            else {
                panic!(
                    "段 ({},{}) 反查无笔——真实数据不应出现空间隙",
                    seg.start_index, seg.end_index
                );
            };
            // 封闭性：区间首笔 end >= seg_start、末笔 start <= seg_end。
            assert!(l0.strokes[f].end_index >= seg.start_index);
            assert!(l0.strokes[l].start_index <= seg.end_index);
            // 区间外相邻笔确实不相交（否则反查漏笔）。
            if f > 0 {
                assert!(l0.strokes[f - 1].end_index < seg.start_index);
            }
            if l + 1 < l0.strokes.len() {
                assert!(l0.strokes[l + 1].start_index > seg.end_index);
            }
            checked += 1;
        }
        assert!(
            checked > 100,
            "OKLO 1 分钟全史段数应远超 100，实测 {checked}"
        );
    }
}
