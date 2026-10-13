//! #1420 / #1344 第四问（xianduan §5.1）：第二种候选的每候选折叠游标（K1）。
//!
//! 第二种候选 X（主序列分型极值 P）的语义终点——`CONFIRMED` / `KILLED_BY_NEW_EXTREME` /
//! `OPEN_AT_PREFIX_END`——是自 b 笔后逐笔的左折叠：每步只读已消费的笔与当前笔，同一笔内先做
//! 包含处理与分型检查、后做严格越界检查，先到者定终点（`078-第78课.md:52`、`067-第67课.md:38`）。
//! 本模块是这一折叠的唯一实现：批处理入口 [`super::feature_seq::second_sequence`] 与续扫共用
//! 同一步进函数，故无中断扫描、分批续扫、自游标恢复后续扫在同一完整输入上逐位相同。
//!
//! **游标字段（K1）**：输入代际（最近一次所见笔数 + 已封存消费笔数 + 其指纹）、规则与政策版本
//! （`second-candidate-terminal/1`、`second-seq-scan-budget/1` 与累计获批预算）、候选身份
//! （方向、b 笔、P）、from/through、标准元素与成员笔、dir_state、越界水位、状态与原因。
//!
//! **等价条件**：已封存消费的笔逐字段不变。游标从不封存输入末笔（末笔可改写：`stroke.rs`
//! 单端点转移至多写旧尾一笔），末笔每次在游标之外暂算；输入回撤一笔、原末前一笔变成末笔时，
//! 游标凭最后一步的撤销记录回退一步（末笔可回退一步）。超出一步的改写返回
//! [`FoldResumeError::InputChanged`]，调用方须自 b 笔重折叠。
//!
//! **预算**：只限定可消费笔数（自 from 起，含反向笔），名分是调度参数、不是语义；耗尽只产
//! `INCOMPLETE_RESOURCE_BOUND`，语义为空。[`SecondFoldCursor::grant`] 追加预算后续扫，结果与
//! 一次给足同额预算的折叠相同（A-ST-048-02）。
//!
//! 有效域：本模块只管单个候选的折叠；候选身份由主特征序列状态机给出（`FeatureSeqState`），
//! 检查点安全边界（最早未封闭候选所在段起点）由 `segment::IncrSegments` 维护。

use super::super::types::{Direction, Stroke, Tick};
use super::feature_seq::{
    apply_inclusion, stroke_high_low, FeatureElement, FeatureStep, ScanCoverage, SecondCheck,
    SecondKill, SecondSequenceFacts, SecondTerminal, SECOND_CANDIDATE_RULE_REVISION,
    SECOND_SEQ_SCAN_BUDGET_POLICY,
};

/// K1 游标结构版本。
pub const SECOND_FOLD_CURSOR_SCHEMA: &str = "second-fold-cursor/1";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 候选身份：折叠结果只取决于它、输入笔与预算（段起点不进身份，归检查点边界）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondCandidateId {
    /// 原线段方向（第二特征序列取同向笔）。
    pub seg_dir: Direction,
    /// 主特征序列分型中心元素 b 的笔下标；第二序列自 `b_stroke + 1` 起。
    pub b_stroke: usize,
    /// P：b 的 high（向上段）/ low（向下段），冻结窗口优先。
    pub candidate_extreme: Tick,
}

/// 续扫拒绝原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldResumeError {
    /// 游标的结构、规则或预算政策版本与当前实现不同：按新版本重折叠。
    VersionMismatch,
    /// 已封存消费的笔被改写或截短超过一步：等价条件失效，自 b 笔重折叠。
    InputChanged,
}

/// 折叠状态：K1 的 from/through、标准元素与成员笔、dir_state、越界水位、状态与原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondFoldState {
    /// 已消费部分的事实。封存态的 `semantic_state` 只会是 `CONFIRMED` 或 `KILLED_BY_NEW_EXTREME`；
    /// `OPEN_AT_PREFIX_END` 与覆盖只在出具结果时按输入末端判定。
    pub facts: SecondSequenceFacts,
    /// 第二序列包含处理方向（独立作用域）。
    pub dir_state: Option<Direction>,
    /// 自 from 起已消费的笔数（含反向笔）。
    pub consumed_len: usize,
    /// 已消费笔的 FNV-1a 64 指纹（输入修订校验，非密码学）。
    pub consumed_digest: u64,
    elements: Vec<(Tick, Tick)>,
}

/// 单步撤销记录：步进只在尾部追加或改写末元素，故只备份尾部（同 `stroke.rs` checkpoint）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct FoldUndo {
    consumed_len: usize,
    consumed_digest: u64,
    through_stroke: usize,
    dir_state: Option<Direction>,
    breach_watermark: Option<Tick>,
    raw_len: usize,
    steps_len: usize,
    checks_len: usize,
    elements_len: usize,
    elements_last: Option<(Tick, Tick)>,
    standard_last: Option<FeatureElement>,
}

fn digest_stroke(mut h: u64, s: &Stroke) -> u64 {
    let dir: u8 = match s.direction {
        Direction::Up => 1,
        Direction::Down => 2,
    };
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(FNV_PRIME);
        }
    };
    eat(&[dir]);
    eat(&(s.start_index as u64).to_le_bytes());
    eat(&(s.end_index as u64).to_le_bytes());
    eat(&s.start_price.to_le_bytes());
    eat(&s.end_price.to_le_bytes());
    h
}

impl SecondFoldState {
    fn new(c: &SecondCandidateId) -> Self {
        SecondFoldState {
            facts: SecondSequenceFacts {
                raw_elements: vec![],
                standard: vec![],
                steps: vec![],
                checks: vec![],
                from_stroke: c.b_stroke.saturating_add(1),
                through_stroke: c.b_stroke,
                input_stroke_count: 0,
                resource_truncated: false,
                matched: None,
                candidate_extreme: c.candidate_extreme,
                breach_watermark: None,
                semantic_state: None,
                killed_by: None,
                coverage: ScanCoverage::Complete,
            },
            dir_state: match c.seg_dir {
                Direction::Up => Some(Direction::Down),
                Direction::Down => None,
            },
            consumed_len: 0,
            consumed_digest: FNV_OFFSET,
            elements: vec![],
        }
    }

    /// 已到语义终点（确认或失效）。
    pub fn decided(&self) -> bool {
        self.facts.semantic_state.is_some()
    }

    /// 下一根待消费笔的下标。
    fn next_index(&self) -> usize {
        self.facts.from_stroke + self.consumed_len
    }

    fn capture(&self) -> FoldUndo {
        FoldUndo {
            consumed_len: self.consumed_len,
            consumed_digest: self.consumed_digest,
            through_stroke: self.facts.through_stroke,
            dir_state: self.dir_state,
            breach_watermark: self.facts.breach_watermark,
            raw_len: self.facts.raw_elements.len(),
            steps_len: self.facts.steps.len(),
            checks_len: self.facts.checks.len(),
            elements_len: self.elements.len(),
            elements_last: self.elements.last().copied(),
            standard_last: self.facts.standard.last().cloned(),
        }
    }

    /// 撤销一步。只对未决态步进，故终点三字段一律回到空。
    fn restore(&mut self, u: FoldUndo) {
        self.consumed_len = u.consumed_len;
        self.consumed_digest = u.consumed_digest;
        self.facts.through_stroke = u.through_stroke;
        self.dir_state = u.dir_state;
        self.facts.breach_watermark = u.breach_watermark;
        self.facts.raw_elements.truncate(u.raw_len);
        self.facts.steps.truncate(u.steps_len);
        self.facts.checks.truncate(u.checks_len);
        self.elements.truncate(u.elements_len);
        self.facts.standard.truncate(u.elements_len);
        if let (Some(e), Some(last)) = (u.elements_last, self.elements.last_mut()) {
            *last = e;
        }
        if let (Some(e), Some(last)) = (u.standard_last, self.facts.standard.last_mut()) {
            *last = e;
        }
        self.facts.matched = None;
        self.facts.semantic_state = None;
        self.facts.killed_by = None;
    }

    /// 消费下一根笔：先包含处理与分型检查，后严格越界检查（到达终点见 `decided`）。
    fn step(&mut self, strokes: &[Stroke], c: &SecondCandidateId) {
        let i = self.next_index();
        let sk = &strokes[i];
        self.consumed_len += 1;
        self.consumed_digest = digest_stroke(self.consumed_digest, sk);
        let facts = &mut self.facts;
        facts.through_stroke = i;
        if sk.direction != c.seg_dir {
            return;
        }
        let (high, low) = stroke_high_low(sk);
        // #1344：越界看该笔原始极值（不是包含合并后的元素），水位只作记账。
        let reach = match c.seg_dir {
            Direction::Up => high,
            Direction::Down => low,
        };
        facts.breach_watermark = Some(match (facts.breach_watermark, c.seg_dir) {
            (None, _) => reach,
            (Some(w), Direction::Up) => w.max(reach),
            (Some(w), Direction::Down) => w.min(reach),
        });
        let incoming = FeatureElement {
            high,
            low,
            stroke_idx: i,
            stroke_indices: vec![i],
        };
        let before = facts.standard.last().cloned();
        facts.raw_elements.push(incoming.clone());
        let previous_len = self.elements.len();
        let merge_direction = if self.dir_state == Some(Direction::Down) {
            "DOWN"
        } else {
            "UP"
        };
        if self.elements.is_empty() {
            self.elements.push((high, low));
        } else {
            self.dir_state = apply_inclusion(&mut self.elements, high, low, self.dir_state);
        }
        let merge = previous_len == self.elements.len();
        let mut stroke_indices = if merge {
            facts.standard.pop().unwrap().stroke_indices
        } else {
            vec![]
        };
        stroke_indices.push(i);
        let (high, low) = *self.elements.last().unwrap();
        let after = FeatureElement {
            high,
            low,
            stroke_idx: i,
            stroke_indices,
        };
        facts.standard.push(after.clone());
        facts.steps.push(FeatureStep {
            incoming,
            before,
            after,
            action: if merge { "merge" } else { "append" },
            merge_direction: merge.then_some(merge_direction),
            scope: "SECOND_SEQUENCE",
        });
        let n = facts.standard.len();
        if n >= 3 {
            let es = [
                facts.standard[n - 3].clone(),
                facts.standard[n - 2].clone(),
                facts.standard[n - 1].clone(),
            ];
            let check = SecondCheck {
                top_fractal: es[1].high > es[0].high && es[1].high > es[2].high,
                bottom_fractal: es[1].low < es[0].low && es[1].low < es[2].low,
                elements: es,
                through_stroke: i,
            };
            facts.checks.push(check.clone());
            if check.top_fractal || check.bottom_fractal {
                facts.matched = Some(check);
                facts.semantic_state = Some(SecondTerminal::Confirmed);
                return;
            }
        }
        // #1344 第二问：分型检查之后才看越界；被包含并入前一元素的笔不是独立第三元素，
        // 但它的新高/新低照样触发失效（078:52「直接新高或新低」）。
        let breached = match c.seg_dir {
            Direction::Up => reach > c.candidate_extreme,
            Direction::Down => reach < c.candidate_extreme,
        };
        if breached {
            facts.killed_by = Some(SecondKill {
                stroke_idx: i,
                price: reach,
                candidate_extreme: c.candidate_extreme,
                comparison: match c.seg_dir {
                    Direction::Up => ">",
                    Direction::Down => "<",
                },
            });
            facts.semantic_state = Some(SecondTerminal::KilledByNewExtreme);
        }
    }
}

/// 预算终点（不含）：`None` = 无限。
fn budget_end(from: usize, granted: Option<usize>, n: usize) -> usize {
    match granted {
        None => n,
        Some(w) => from.saturating_add(w).min(n),
    }
}

/// 按输入末端出具结果：未决时，预算先于输入末端耗尽 ⟹ 覆盖不完整、语义为空；否则 OPEN。
fn finish(mut facts: SecondSequenceFacts, n: usize, end: usize) -> SecondSequenceFacts {
    facts.input_stroke_count = n;
    if facts.semantic_state.is_none() {
        facts.resource_truncated = end < n;
        if facts.resource_truncated {
            facts.coverage = ScanCoverage::IncompleteResourceBound;
        } else {
            facts.semantic_state = Some(SecondTerminal::OpenAtPrefixEnd);
        }
    }
    facts
}

/// 无中断折叠（批处理入口）：`scan_window` 0 = 无限。
pub(super) fn fold_uninterrupted(
    strokes: &[Stroke],
    candidate: SecondCandidateId,
    scan_window: u32,
) -> SecondSequenceFacts {
    let mut state = SecondFoldState::new(&candidate);
    let n = strokes.len();
    let end = budget_end(state.facts.from_stroke, window_grant(scan_window), n);
    while !state.decided() && state.next_index() < end {
        state.step(strokes, &candidate);
    }
    finish(state.facts, n, end)
}

fn window_grant(scan_window: u32) -> Option<usize> {
    (scan_window != 0).then_some(scan_window as usize)
}

/// K1 每候选折叠游标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondFoldCursor {
    pub schema: &'static str,
    pub rule_revision: &'static str,
    pub budget_policy: &'static str,
    /// 累计获批预算（笔数，自 from 起含反向笔）；`None` = 无限（窗口 0）。
    pub budget_granted: Option<usize>,
    pub candidate: SecondCandidateId,
    /// 输入代际：最近一次续扫所见的输入笔数（已封存部分的修订见 `state` 的指纹）。
    pub input_stroke_count: usize,
    /// 已封存的折叠状态（从不含输入末笔，见模块头）。
    pub state: SecondFoldState,
    undo: Option<FoldUndo>,
}

impl SecondFoldCursor {
    /// 新游标：`scan_window` 0 = 无限预算。
    pub fn new(candidate: SecondCandidateId, scan_window: u32) -> Self {
        SecondFoldCursor {
            schema: SECOND_FOLD_CURSOR_SCHEMA,
            rule_revision: SECOND_CANDIDATE_RULE_REVISION,
            budget_policy: SECOND_SEQ_SCAN_BUDGET_POLICY,
            budget_granted: window_grant(scan_window),
            candidate,
            input_stroke_count: 0,
            state: SecondFoldState::new(&candidate),
            undo: None,
        }
    }

    /// 追加预算（调度）：`extra` 0 = 改为无限。
    pub fn grant(&mut self, extra: u32) {
        self.budget_granted = match (self.budget_granted, extra) {
            (_, 0) | (None, _) => None,
            (Some(g), e) => Some(g.saturating_add(e as usize)),
        };
    }

    /// 校验已封存前缀；必要时回退一步。返回 false = 等价条件失效。
    fn reconcile(&mut self, strokes: &[Stroke]) -> bool {
        let len = self.state.consumed_len;
        if len == 0 {
            return true;
        }
        let from = self.state.facts.from_stroke;
        let n = strokes.len();
        if from > n {
            return false;
        }
        // 可封存上限：输入末笔之前（末笔从不封存）。
        let sealable = n.saturating_sub(1).saturating_sub(from);
        let available = n.saturating_sub(from);
        let mut h = FNV_OFFSET;
        let head = (len - 1).min(available);
        for sk in &strokes[from..from + head] {
            h = digest_stroke(h, sk);
        }
        let head_ok = head == len - 1;
        if head_ok
            && len <= sealable
            && digest_stroke(h, &strokes[from + len - 1]) == self.state.consumed_digest
        {
            return true;
        }
        // 末笔可回退一步：撤销最后一步，校验其前的已封存前缀。
        match self.undo.take() {
            Some(u) if head_ok && u.consumed_len <= sealable && h == u.consumed_digest => {
                self.state.restore(u);
                true
            }
            _ => false,
        }
    }

    /// 在 `strokes` 上续左折叠并出具结果；结果与在同一输入上无中断折叠（同额预算）逐位相同。
    pub fn resume(&mut self, strokes: &[Stroke]) -> Result<SecondSequenceFacts, FoldResumeError> {
        if self.schema != SECOND_FOLD_CURSOR_SCHEMA
            || self.rule_revision != SECOND_CANDIDATE_RULE_REVISION
            || self.budget_policy != SECOND_SEQ_SCAN_BUDGET_POLICY
        {
            return Err(FoldResumeError::VersionMismatch);
        }
        if !self.reconcile(strokes) {
            return Err(FoldResumeError::InputChanged);
        }
        let n = strokes.len();
        let end = budget_end(self.state.facts.from_stroke, self.budget_granted, n);
        // 封存到输入末笔之前（预算先尽则止于预算）。
        let seal_end = end.min(n.saturating_sub(1));
        while !self.state.decided() && self.state.next_index() < seal_end {
            self.undo = Some(self.state.capture());
            self.state.step(strokes, &self.candidate);
        }
        // 末笔暂算：步进、出具、撤销，不入封存态。
        let facts = if !self.state.decided() && self.state.next_index() < end {
            let u = self.state.capture();
            self.state.step(strokes, &self.candidate);
            let out = self.state.facts.clone();
            self.state.restore(u);
            out
        } else {
            self.state.facts.clone()
        };
        self.input_stroke_count = n;
        Ok(finish(facts, n, end))
    }

    /// 等价条件失效后自 b 笔重折叠（保留身份与累计预算）。
    pub fn refold(&mut self, strokes: &[Stroke]) -> SecondSequenceFacts {
        self.state = SecondFoldState::new(&self.candidate);
        self.undo = None;
        self.resume(strokes)
            .expect("空封存态无前缀可校验，版本取自当前常量")
    }
}
