//! segment.rs — 线段 v1 特征序列法，逐位等价移植自 `src/newchan/a_segment_v1.py`
//!
//! 契约：批量等价。给定同一笔列表（`Vec<Stroke>`，逐位等价于 Python `Stroke`），
//! `segments_from_strokes_v1` 的输出与 Python `segments_from_strokes_v1(strokes,
//! min_seg_strokes, extend_mode, _resume=None)` 逐字段相等。
//!
//! ## 逐位等价要点
//!
//! 1. 线段构造**不做任何浮点算术**——只有比较与 min/max **选择**。故无浮点约简顺序
//!    问题（不同于 bi 的 `slice_max`）。
//! 2. Python `min/max(key=...)` 在并列时返回**首个**极值元素；Rust `Iterator::max_by`
//!    并列时返回**末个**。`_standardize_endpoints` 用 first-extreme，故手写严格不等扫描。
//! 3. 有符号下标：`n - TAIL_WINDOW`、`skip_until = -1`、`max(0, i-1)`、`min_seg_strokes-1`
//!    用 i64 复刻 Python int 语义，避免 usize 下溢。
//! 4. `append()` 内**预演式**调用 `scan_trigger`，主循环 `_try_trigger_segment` 再调一次——
//!    每个含包含关系的笔上 scan_trigger 跑两遍。两处调用点逐行复刻，`last_checked`
//!    副作用序列一致。
//! 5. `_resume` 增量快路径是 Python 内部优化（已单独验证等于批量），不属跨语言契约；
//!    本模块移植批量路径（`_resume=None`）。

use crate::stroke::{Direction, Stroke};

// ════════════════════════════════════════════════════════════
// 数据类型（对应 a_segment_v0.py 的 Segment / BreakEvidence）
// ════════════════════════════════════════════════════════════

/// 分型类型。对应 Python Literal["top", "bottom"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalType {
    Top,
    Bottom,
}

impl FractalType {
    pub fn as_str(self) -> &'static str {
        match self {
            FractalType::Top => "top",
            FractalType::Bottom => "bottom",
        }
    }
}

/// 缺口类型。对应 Python Literal["none", "second"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapType {
    None,
    Second,
}

impl GapType {
    pub fn as_str(self) -> &'static str {
        match self {
            GapType::None => "none",
            GapType::Second => "second",
        }
    }
}

/// 线段类型。对应 Python Literal["candidate", "settled"]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegKind {
    Candidate,
    Settled,
}

impl SegKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SegKind::Candidate => "candidate",
            SegKind::Settled => "settled",
        }
    }
}

/// 断段证据。对应 Python `BreakEvidence` frozen dataclass。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakEvidence {
    pub trigger_stroke_k: usize,
    pub fractal_abc: (usize, usize, usize),
    pub gap_type: GapType,
}

/// 一段线段。对应 Python `Segment` frozen dataclass。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub s0: usize,
    pub s1: usize,
    pub i0: usize,
    pub i1: usize,
    pub direction: Direction,
    pub high: f64,
    pub low: f64,
    pub confirmed: bool,
    pub kind: SegKind,
    pub ep0_i: usize,
    pub ep0_price: f64,
    pub ep0_type: FractalType,
    pub ep1_i: usize,
    pub ep1_price: f64,
    pub ep1_type: FractalType,
    pub p0: f64,
    pub p1: f64,
    pub break_evidence: Option<BreakEvidence>,
}

// ════════════════════════════════════════════════════════════
// 内部工具（移植自 a_segment_v1.py 顶部函数）
// ════════════════════════════════════════════════════════════

/// 三笔交集重叠判定。移植自 `_three_stroke_overlap`。
#[inline]
fn three_stroke_overlap(s1: &Stroke, s2: &Stroke, s3: &Stroke) -> bool {
    let lo = s1.low.max(s2.low).max(s3.low);
    let hi = s1.high.min(s2.high).min(s3.high);
    lo < hi
}

/// 从 from_s 开始找第一个三笔交集重叠的起点。移植自 `_find_overlap_start`。
fn find_overlap_start(strokes: &[Stroke], from_s: usize) -> Option<usize> {
    let n = strokes.len();
    // Python: while j <= n - 3（n<3 时 n-3 为负，循环不执行）。j + 3 <= n 等价且避免下溢。
    let mut j = from_s;
    while j + 3 <= n {
        if three_stroke_overlap(&strokes[j], &strokes[j + 1], &strokes[j + 2]) {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// 段方向 → (起点分型类型, 终点分型类型)。移植自 `_segment_endpoint_types`。
#[inline]
fn segment_endpoint_types(direction: Direction) -> (FractalType, FractalType) {
    match direction {
        Direction::Up => (FractalType::Bottom, FractalType::Top),
        Direction::Down => (FractalType::Top, FractalType::Bottom),
    }
}

/// 从笔上按分型类型取端点（merged idx, price）。移植自 `_stroke_endpoint_by_type`。
#[inline]
fn stroke_endpoint_by_type(stroke: &Stroke, fractal_type: FractalType) -> (usize, f64) {
    match fractal_type {
        FractalType::Top => {
            if stroke.direction == Direction::Down {
                (stroke.i0, stroke.p0)
            } else {
                (stroke.i1, stroke.p1)
            }
        }
        FractalType::Bottom => {
            if stroke.direction == Direction::Down {
                (stroke.i1, stroke.p1)
            } else {
                (stroke.i0, stroke.p0)
            }
        }
    }
}

/// 第78课硬约束：一顶一底，顶肯定高于底。移植自 `_top_above_bottom`。
#[inline]
fn top_above_bottom(direction: Direction, ep0_price: f64, ep1_price: f64) -> bool {
    match direction {
        Direction::Up => ep1_price > ep0_price,
        Direction::Down => ep0_price > ep1_price,
    }
}

/// 段内首个 low 最小的笔下标（first-min，复刻 Python `min(key=low)` 并列取首）。
#[inline]
fn first_min_low_idx(seg: &[Stroke]) -> usize {
    let mut best = 0usize;
    let mut bv = seg[0].low;
    let mut k = 1usize;
    while k < seg.len() {
        if seg[k].low < bv {
            bv = seg[k].low;
            best = k;
        }
        k += 1;
    }
    best
}

/// 段内首个 high 最大的笔下标（first-max，复刻 Python `max(key=high)` 并列取首）。
#[inline]
fn first_max_high_idx(seg: &[Stroke]) -> usize {
    let mut best = 0usize;
    let mut bv = seg[0].high;
    let mut k = 1usize;
    while k < seg.len() {
        if seg[k].high > bv {
            bv = seg[k].high;
            best = k;
        }
        k += 1;
    }
    best
}

/// 第78课标准化：端点违反 L78 时用实际 high/low 替代。移植自 `_standardize_endpoints`。
fn standardize_endpoints(seg: &[Stroke], direction: Direction) -> (usize, f64, usize, f64) {
    if direction == Direction::Up {
        let low_s = &seg[first_min_low_idx(seg)];
        let high_s = &seg[first_max_high_idx(seg)];
        let ep0_i = if low_s.p0 <= low_s.p1 { low_s.i0 } else { low_s.i1 };
        let ep0_price = low_s.low;
        let ep1_i = if high_s.p0 >= high_s.p1 { high_s.i0 } else { high_s.i1 };
        let ep1_price = high_s.high;
        (ep0_i, ep0_price, ep1_i, ep1_price)
    } else {
        let high_s = &seg[first_max_high_idx(seg)];
        let low_s = &seg[first_min_low_idx(seg)];
        let ep0_i = if high_s.p0 >= high_s.p1 { high_s.i0 } else { high_s.i1 };
        let ep0_price = high_s.high;
        let ep1_i = if low_s.p0 <= low_s.p1 { low_s.i0 } else { low_s.i1 };
        let ep1_price = low_s.low;
        (ep0_i, ep0_price, ep1_i, ep1_price)
    }
}

/// 创建 Segment：端点从边界笔取，L78 违反时标准化。移植自 `_make_segment`。
fn make_segment(
    strokes: &[Stroke],
    s0: usize,
    s1: usize,
    direction: Direction,
    confirmed: bool,
    break_evidence: Option<BreakEvidence>,
    kind: SegKind,
) -> Segment {
    let seg = &strokes[s0..=s1];
    // seg_high = max(s.high ...) / seg_low = min(s.low ...)：从首元素顺序约简。
    let mut seg_high = seg[0].high;
    let mut seg_low = seg[0].low;
    let mut k = 1usize;
    while k < seg.len() {
        seg_high = seg_high.max(seg[k].high);
        seg_low = seg_low.min(seg[k].low);
        k += 1;
    }
    let (start_type, end_type) = segment_endpoint_types(direction);
    let (mut ep0_i, mut ep0_price) = stroke_endpoint_by_type(&strokes[s0], start_type);
    let (mut ep1_i, mut ep1_price) = stroke_endpoint_by_type(&strokes[s1], end_type);

    if !top_above_bottom(direction, ep0_price, ep1_price) {
        // 279号修复：L78 违反时应用第78课标准化（端点改为段内实际 high/low）。
        let (a, b, c, d) = standardize_endpoints(seg, direction);
        ep0_i = a;
        ep0_price = b;
        ep1_i = c;
        ep1_price = d;
    }

    Segment {
        s0,
        s1,
        i0: strokes[s0].i0,
        i1: strokes[s1].i1,
        direction,
        high: seg_high,
        low: seg_low,
        confirmed,
        kind,
        ep0_i,
        ep0_price,
        ep0_type: start_type,
        ep1_i,
        ep1_price,
        ep1_type: end_type,
        p0: ep0_price,
        p1: ep1_price,
        break_evidence,
    }
}

// ════════════════════════════════════════════════════════════
// 增量特征序列 + 包含处理 + 分型检测
// ════════════════════════════════════════════════════════════

/// 特征序列方向状态。对应 Python `dir_state: str | None`（None / "UP" / "DOWN"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirState {
    None,
    Up,
    Down,
}

/// 对 elements 尾部做包含处理或追加新元素，返回更新后的 dir_state。
/// 移植自 `_apply_inclusion`（用于第二特征序列，elements = (high, low) 对）。
fn apply_inclusion(elements: &mut Vec<(f64, f64)>, h: f64, l: f64, dir_state: DirState) -> DirState {
    let (last_h, last_l) = *elements.last().unwrap();
    let left_inc = last_h >= h && last_l <= l;
    let right_inc = h >= last_h && l <= last_l;

    if left_inc || right_inc {
        // effective_up = dir_state != "DOWN"（None/"UP" 均为 True）
        let effective_up = dir_state != DirState::Down;
        let last = elements.last_mut().unwrap();
        if effective_up {
            last.0 = last_h.max(h);
            last.1 = last_l.max(l);
        } else {
            last.0 = last_h.min(h);
            last.1 = last_l.min(l);
        }
        return dir_state;
    }

    let mut ds = dir_state;
    if h > last_h && l > last_l {
        ds = DirState::Up;
    } else if h < last_h && l < last_l {
        ds = DirState::Down;
    }
    elements.push((h, l));
    ds
}

/// 在元素序列中检测是否存在任意分型（顶或底）。移植自 `_has_any_fractal`。
fn has_any_fractal(elements: &[(f64, f64)]) -> bool {
    let n = elements.len();
    // Python: for j in range(1, n - 1)
    let mut j = 1usize;
    let upper = n.saturating_sub(1);
    while j < upper {
        let (b_h, b_l) = elements[j];
        let (a_h, a_l) = elements[j - 1];
        let (c_h, c_l) = elements[j + 1];
        if b_h > a_h && b_h > c_h {
            return true;
        }
        if b_l < a_l && b_l < c_l {
            return true;
        }
        j += 1;
    }
    false
}

/// 检测 (a,b,c) 是否构成目标分型，及 a-b 间是否有缺口。移植自 `_is_fractal_and_gap`。
#[inline]
fn is_fractal_and_gap(
    a_h: f64,
    a_l: f64,
    b_h: f64,
    b_l: f64,
    c_h: f64,
    c_l: f64,
    seg_direction: Direction,
) -> (bool, bool) {
    if seg_direction == Direction::Up {
        let is_fractal = b_h > a_h && b_h > c_h;
        let has_gap = if is_fractal { b_l >= a_h } else { false };
        (is_fractal, has_gap)
    } else {
        let is_fractal = b_l < a_l && b_l < c_l;
        let has_gap = if is_fractal { a_l >= b_h } else { false };
        (is_fractal, has_gap)
    }
}

const TAIL_WINDOW: i64 = 7;
/// 第二特征序列最大扫描窗口。移植自 `_FeatureSeqState.MAX_SECOND_SEQ_SCAN`。
/// 同时是 orchestrator 线段检查点稳定性判据的窗口边界（`trigger_k+1+MARGIN<=n`）。
pub const MAX_SECOND_SEQ_SCAN: usize = 50;

/// 检查第二特征序列是否存在分型。移植自 `_FeatureSeqState._second_seq_has_fractal`。
fn second_seq_has_fractal(
    strokes: &[Stroke],
    seg_dir: Direction,
    from_stroke_idx: usize,
    max_scan: usize,
) -> bool {
    let mut elements: Vec<(f64, f64)> = Vec::new();
    // Python: dir_state = "DOWN" if seg_dir == "up" else None
    let mut dir_state = if seg_dir == Direction::Up {
        DirState::Down
    } else {
        DirState::None
    };
    let end_idx = std::cmp::min(from_stroke_idx + 1 + max_scan, strokes.len());

    let mut i = from_stroke_idx + 1;
    while i < end_idx {
        let sk = &strokes[i];
        if sk.direction != seg_dir {
            i += 1;
            continue;
        }
        if elements.is_empty() {
            elements.push((sk.high, sk.low));
            i += 1;
            continue;
        }
        dir_state = apply_inclusion(&mut elements, sk.high, sk.low, dir_state);
        i += 1;
    }

    has_any_fractal(&elements)
}

/// 增量维护标准特征序列。移植自 `_FeatureSeqState`。
struct FeatureSeqState {
    /// 标准特征序列：每个元素 = (high, low, stroke_idx)。
    std: Vec<(f64, f64, usize)>,
    dir_state: DirState,
    last_checked: usize,
    /// 跳过 stroke_idx <= 此值的分型。初始 -1（i64 复刻 Python int）。
    skip_until_stroke: i64,
    extend_mode_strict: bool,
}

impl FeatureSeqState {
    fn new(seg_direction: Direction, extend_mode_strict: bool) -> Self {
        FeatureSeqState {
            std: Vec::new(),
            dir_state: if seg_direction == Direction::Down {
                DirState::Down
            } else {
                DirState::None
            },
            last_checked: 0,
            skip_until_stroke: -1,
            extend_mode_strict,
        }
    }

    fn reset(&mut self, seg_direction: Direction) {
        self.std.clear();
        self.dir_state = if seg_direction == Direction::Down {
            DirState::Down
        } else {
            DirState::None
        };
        self.last_checked = 0;
        self.skip_until_stroke = -1;
    }

    fn skip_trigger(&mut self, stroke_idx: usize) {
        self.skip_until_stroke = stroke_idx as i64;
    }

    /// 增量添加一个反向笔，包含处理遵循71课"假设转折点"规则。移植自 `append`。
    fn append(
        &mut self,
        stroke_idx: usize,
        high: f64,
        low: f64,
        seg_direction: Direction,
        strokes: &[Stroke],
    ) {
        if self.std.is_empty() {
            self.std.push((high, low, stroke_idx));
            return;
        }

        let (last_h, last_l, _) = *self.std.last().unwrap();
        let left_inc = last_h >= high && last_l <= low;
        let right_inc = high >= last_h && low <= last_l;
        let has_inclusion = left_inc || right_inc;

        if has_inclusion {
            // 71课：先尝试不合并（保持分离），检查是否形成分型 → 有分型则不合并。
            self.std.push((high, low, stroke_idx));
            if self.scan_trigger(seg_direction, strokes).is_some() {
                return;
            }
            self.std.pop();

            let effective_up = self.dir_state != DirState::Down;
            let last = self.std.last_mut().unwrap();
            if effective_up {
                last.0 = last_h.max(high);
                last.1 = last_l.max(low);
            } else {
                last.0 = last_h.min(high);
                last.1 = last_l.min(low);
            }
            last.2 = stroke_idx;
            // last_checked = max(0, len(std) - 3)
            self.last_checked = self.std.len().saturating_sub(3);
        } else {
            if high > last_h && low > last_l {
                self.dir_state = DirState::Up;
            } else if high < last_h && low < last_l {
                self.dir_state = DirState::Down;
            }
            self.std.push((high, low, stroke_idx));
        }
    }

    /// 从 last_checked 向后扫描（受尾窗限制），找第一个匹配分型。移植自 `scan_trigger`。
    fn scan_trigger(
        &mut self,
        seg_direction: Direction,
        strokes: &[Stroke],
    ) -> Option<(usize, (usize, usize, usize), GapType)> {
        let n = self.std.len();
        if n < 3 {
            return None;
        }
        // start = max(1, last_checked, n - TAIL_WINDOW)（n - TAIL_WINDOW 可能为负 → i64）
        let n_i = n as i64;
        let mut start_i = 1i64;
        if (self.last_checked as i64) > start_i {
            start_i = self.last_checked as i64;
        }
        if n_i - TAIL_WINDOW > start_i {
            start_i = n_i - TAIL_WINDOW;
        }
        let start = start_i as usize; // 保证 ≥ 1

        let mut i = start;
        while i < n - 1 {
            let b_stroke = self.std[i].2;
            if (b_stroke as i64) <= self.skip_until_stroke {
                i += 1;
                continue;
            }

            let (a_h, a_l) = (self.std[i - 1].0, self.std[i - 1].1);
            let (b_h, b_l) = (self.std[i].0, self.std[i].1);
            let (c_h, c_l) = (self.std[i + 1].0, self.std[i + 1].1);

            let (is_fractal, mut has_gap) =
                is_fractal_and_gap(a_h, a_l, b_h, b_l, c_h, c_l, seg_direction);
            if !is_fractal {
                i += 1;
                continue;
            }

            // 67课严格延续：缺口被 c 封闭 → 按第一种情况处理。
            if has_gap && self.extend_mode_strict {
                let gap_closed_by_c = if seg_direction == Direction::Up {
                    c_l <= a_h
                } else {
                    c_h >= a_l
                };
                if gap_closed_by_c {
                    has_gap = false;
                }
            }

            if has_gap
                && !second_seq_has_fractal(strokes, seg_direction, b_stroke, MAX_SECOND_SEQ_SCAN)
            {
                i += 1;
                continue;
            }

            let gap_type = if has_gap { GapType::Second } else { GapType::None };
            // last_checked = max(0, i - 1)；i ≥ start ≥ 1，故 i-1 ≥ 0。
            self.last_checked = i - 1;
            return Some((b_stroke, (i - 1, i, i + 1), gap_type));
        }

        None
    }
}

// ════════════════════════════════════════════════════════════
// v1 主函数
// ════════════════════════════════════════════════════════════

/// 尝试从特征序列触发断段。移植自 `_try_trigger_segment`。
fn try_trigger_segment(
    feat: &mut FeatureSeqState,
    seg_dir: Direction,
    strokes: &[Stroke],
    seg_start: usize,
    min_seg_strokes: usize,
) -> Option<(usize, BreakEvidence)> {
    let (k, fractal_abc, gap_type) = feat.scan_trigger(seg_dir, strokes)?;
    let end_stroke = k - 1; // k 为反向笔下标 ≥ seg_start+1 ≥ 1，故安全

    // 保证至少 min_seg_strokes 笔：if end_stroke - seg_start < min_seg_strokes - 1
    if (end_stroke as i64) - (seg_start as i64) < (min_seg_strokes as i64) - 1 {
        feat.skip_trigger(k);
        return None;
    }

    // L78 前置检查在 Python 中仅 debug 记录，不阻止触发 → 此处省略。
    let break_ev = BreakEvidence {
        trigger_stroke_k: k,
        fractal_abc,
        gap_type,
    };
    Some((k, break_ev))
}

/// 发射旧段。移植自 `_emit_segment`。
fn emit_segment(
    segments: &mut Vec<Segment>,
    strokes: &[Stroke],
    seg_start: usize,
    seg_dir: Direction,
    k: usize,
    break_ev: BreakEvidence,
) {
    let end_stroke = k - 1;
    segments.push(make_segment(
        strokes,
        seg_start,
        end_stroke,
        seg_dir,
        true,
        Some(break_ev),
        SegKind::Settled,
    ));
}

/// 处理最后一段（未确认）并追加。移植自 `_finalize_last_segment`。
fn finalize_last_segment(
    segments: &mut Vec<Segment>,
    strokes: &[Stroke],
    seg_start: usize,
    seg_dir: Direction,
    min_seg_strokes: usize,
    n: usize,
) {
    if seg_start >= n {
        return;
    }
    let last_end = n - 1;
    if (last_end as i64) - (seg_start as i64) >= (min_seg_strokes as i64) - 1 {
        let last_kind = if seg_start + 2 < n
            && three_stroke_overlap(
                &strokes[seg_start],
                &strokes[seg_start + 1],
                &strokes[seg_start + 2],
            ) {
            SegKind::Settled
        } else {
            SegKind::Candidate
        };
        segments.push(make_segment(
            strokes, seg_start, last_end, seg_dir, false, None, last_kind,
        ));
    } else if !segments.is_empty() {
        let prev = *segments.last().unwrap();
        let new_seg = make_segment(
            strokes,
            prev.s0,
            last_end,
            prev.direction,
            false,
            None,
            prev.kind,
        );
        *segments.last_mut().unwrap() = new_seg;
    } else {
        segments.push(make_segment(
            strokes,
            seg_start,
            last_end,
            seg_dir,
            false,
            None,
            SegKind::Candidate,
        ));
    }
}

/// 确保最后一段 confirmed=False。移植自 `_ensure_last_unconfirmed`。
fn ensure_last_unconfirmed(segments: &mut Vec<Segment>, strokes: &[Stroke]) {
    if let Some(last) = segments.last() {
        if last.confirmed {
            let l = *last;
            let new_seg = make_segment(strokes, l.s0, l.s1, l.direction, false, None, l.kind);
            *segments.last_mut().unwrap() = new_seg;
        }
    }
}

/// v1 线段构造：增量特征序列法批量入口。移植自 `segments_from_strokes_v1`（_resume=None）。
pub fn segments_from_strokes_v1(
    strokes: &[Stroke],
    min_seg_strokes: usize,
    extend_mode_strict: bool,
) -> Vec<Segment> {
    let n = strokes.len();
    if n < 3 {
        return Vec::new();
    }
    let seg_start = match find_overlap_start(strokes, 0) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let seg_dir = strokes[seg_start].direction;
    let mut segments: Vec<Segment> = Vec::new();
    segments_from_strokes_v1_into(
        &mut segments,
        strokes,
        min_seg_strokes,
        extend_mode_strict,
        seg_start,
        seg_dir,
    );
    segments
}

/// 原地续算线段：从 `(seg_start, seg_dir)` 起运行主循环，向 `segments` **追加**新段并收尾。
///
/// 调用契约（resume 场景）：`segments` 已含可复用的不可变前缀（调用方 truncate 到复用边界），
/// `seg_start` 为下一段（= 前缀**之后**第一个待重算段）的起始笔下标，`seg_dir` 为其方向，
/// 配 fresh `FeatureSeqState`。此条件与全量计算"发射前一段后、开始下一段时"的状态逐字段一致
/// （`seg_start = 前段.trigger_k`、`seg_dir = opposite`、feat.reset），故续算逐位等价于全量。
///
/// 全量入口传 `seg_start = find_overlap_start(strokes, 0)`、`seg_dir = strokes[seg_start].direction`、
/// 空 `segments`——退化为原批量路径。
///
/// **不可变前缀不被触碰**：主循环只追加（`emit_segment` push），`finalize_last_segment` /
/// `ensure_last_unconfirmed` 只改 `segments.last()`。当本次至少发射一段时 last 为新段；
/// 当零发射且尾部过短时 last 落在 `seg_start` 起始的重算段或调用方留下的边界段——由调用方
/// 保证 `seg_start` 段本身在重算范围内（见 orchestrator 复用 `stable_count-1` 段的设计）。
pub fn segments_from_strokes_v1_into(
    segments: &mut Vec<Segment>,
    strokes: &[Stroke],
    min_seg_strokes: usize,
    extend_mode_strict: bool,
    seg_start: usize,
    seg_dir: Direction,
) {
    let n = strokes.len();
    if n < 3 {
        return;
    }
    let mut seg_start = seg_start;
    let mut seg_dir = seg_dir;

    if seg_start >= n {
        // resume 起点越界：直接收尾（移植 Python `_resume` 的 `resume_start >= n` 分支）。
        finalize_last_segment(segments, strokes, seg_start, seg_dir, min_seg_strokes, n);
        ensure_last_unconfirmed(segments, strokes);
        return;
    }

    let mut feat = FeatureSeqState::new(seg_dir, extend_mode_strict);
    let mut cursor = seg_start;

    while cursor < n {
        let sk = &strokes[cursor];
        let opposite = if seg_dir == Direction::Up {
            Direction::Down
        } else {
            Direction::Up
        };
        if sk.direction != opposite {
            cursor += 1;
            continue;
        }

        feat.append(cursor, sk.high, sk.low, seg_dir, strokes);
        match try_trigger_segment(&mut feat, seg_dir, strokes, seg_start, min_seg_strokes) {
            None => {
                cursor += 1;
                continue;
            }
            Some((k, break_ev)) => {
                emit_segment(segments, strokes, seg_start, seg_dir, k, break_ev);
                seg_start = k;
                seg_dir = opposite;
                feat.reset(seg_dir);
                cursor = k;
            }
        }
    }

    finalize_last_segment(segments, strokes, seg_start, seg_dir, min_seg_strokes, n);
    ensure_last_unconfirmed(segments, strokes);
}
