//! #1420 / #1344 第四问（xianduan §5.1）：第二种候选的每候选折叠游标（K1）。
//!
//! 第二种候选 X（主序列分型极值 P）的语义终点——`CONFIRMED` / `KILLED_BY_NEW_EXTREME` /
//! `OPEN_AT_PREFIX_END`——是自 b 笔后逐笔的左折叠：每步只读已消费的笔与当前笔，同一笔内先做
//! 包含处理与分型检查、后做严格越界检查，先到者定终点（`078-第78课.md:52`、`067-第67课.md:38`）。
//! 本模块是这一折叠的唯一实现：批处理入口 [`super::feature_seq::second_sequence`] 与续扫共用
//! 同一步进函数，故无中断扫描、分批续扫、自游标恢复后续扫在同一完整输入上逐位相同。
//!
//! **游标字段（K1）**：输入代际（最近一次所见笔数 + 已封存消费笔数 + 其指纹）、规则与政策版本
//! （`second-candidate-terminal/1`、本次所读预算记录 `second-seq-scan-budget/<版本>` 与累计获批预算）、候选身份
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
};
use crate::theta_v0::policy_records::PolicyRef;

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
            window_members: vec![],
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
            window_members: vec![],
        };
        facts.standard.push(after.clone());
        facts.steps.push(FeatureStep {
            incoming,
            before,
            after,
            action: if merge { "merge" } else { "append" },
            merge_direction: merge.then_some(merge_direction),
            scope: "SECOND_SEQUENCE",
            tie_with: None,
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
    /// #1604：所读预算记录（`second-seq-scan-budget` 的 id/版本），由游标簿从 `ParseConfig::scan_budget`
    /// 带入；线形写 `label()`（例 `second-seq-scan-budget/2`）。不写代码常数。
    pub budget_policy: PolicyRef,
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
    /// 新游标：`scan_window` 0 = 无限预算；`budget_policy` = 该预算出自的记录版本。
    pub fn new(candidate: SecondCandidateId, scan_window: u32, budget_policy: PolicyRef) -> Self {
        SecondFoldCursor {
            schema: SECOND_FOLD_CURSOR_SCHEMA,
            rule_revision: SECOND_CANDIDATE_RULE_REVISION,
            budget_policy,
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
    ///
    /// 预算记录版本不在此比对：内存游标的 `budget_policy` 由游标簿按本次所读记录写入，线形恢复时
    /// 由 [`Self::from_wire`] 对照本次记录核验（#1604）。
    pub fn resume(&mut self, strokes: &[Stroke]) -> Result<SecondSequenceFacts, FoldResumeError> {
        if self.schema != SECOND_FOLD_CURSOR_SCHEMA
            || self.rule_revision != SECOND_CANDIDATE_RULE_REVISION
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

// ─── K1 线形与跨轮调度（#1420 tb02c 片）─────────────────────────────────────────────────────
//
// 线形只携 K1 字段（不携包含步骤与检查日志）。恢复时自 b 笔重折叠已封存的 `consumed_len` 步，
// 与线形逐字段比对（含指纹）：相符即采纳并重建单步撤销记录，不符（输入已修订、或线形被改）即
// 自 b 笔重折叠——无论哪条路，出具的事实都等于同一输入、同额预算的无中断折叠。累计预算是唯一
// 依赖历史的字段，恢复时照搬。

fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "UP",
        Direction::Down => "DOWN",
    }
}

fn wire_int(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Number(n) => n.as_i64(),
        _ => None,
    }
}

/// 数字一律转十进制文本（与 S 对象线形一致），便于逐字段比对。
fn normalized(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Number(n) => Value::String(n.to_string()),
        Value::Array(a) => Value::Array(a.into_iter().map(normalized).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, x)| (k, normalized(x))).collect()),
        x => x,
    }
}

/// #1419：K1 的只借用序列化视图；字段与 `to_wire` 完全同义，不拥有 JSON 树。
pub struct SecondFoldWire<'a>(&'a SecondFoldCursor);
impl serde::Serialize for SecondFoldWire<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        #[derive(serde::Serialize)]
        struct Candidate<'a> {
            seg_dir: &'a str,
            b_stroke: usize,
            candidate_extreme: Tick,
        }
        let c = self.0;
        let f = &c.state.facts;
        let mut hex = [b'0'; 16];
        for (i, byte) in hex.iter_mut().enumerate() {
            *byte =
                b"0123456789abcdef"[((c.state.consumed_digest >> ((15 - i) * 4)) & 15) as usize];
        }
        let digest = std::str::from_utf8(&hex).expect("十六进制ASCII");
        let mut out = serializer.serialize_struct("SecondFoldWire", 16)?;
        out.serialize_field("schema", &c.schema)?;
        out.serialize_field("rule_revision", &c.rule_revision)?;
        out.serialize_field("budget_policy", &c.budget_policy.label())?;
        out.serialize_field("budget_granted", &c.budget_granted)?;
        out.serialize_field(
            "candidate",
            &Candidate {
                seg_dir: dir_name(c.candidate.seg_dir),
                b_stroke: c.candidate.b_stroke,
                candidate_extreme: c.candidate.candidate_extreme,
            },
        )?;
        out.serialize_field("input_stroke_count", &c.input_stroke_count)?;
        out.serialize_field("from_stroke", &f.from_stroke)?;
        out.serialize_field("through_stroke", &f.through_stroke)?;
        out.serialize_field("consumed_len", &c.state.consumed_len)?;
        out.serialize_field("consumed_digest", digest)?;
        out.serialize_field("dir_state", &c.state.dir_state.map(dir_name))?;
        out.serialize_field("standard", &f.standard)?;
        out.serialize_field("breach_watermark", &f.breach_watermark)?;
        out.serialize_field("semantic_state", &f.semantic_state)?;
        out.serialize_field("killed_by", &f.killed_by)?;
        out.serialize_field("matched", &f.matched)?;
        out.end()
    }
}

impl SecondFoldCursor {
    pub fn wire_view(&self) -> SecondFoldWire<'_> {
        SecondFoldWire(self)
    }
    /// K1 线形（字段见模块头）。
    pub fn to_wire(&self) -> serde_json::Value {
        let f = &self.state.facts;
        serde_json::json!({
            "schema": self.schema,
            "rule_revision": self.rule_revision,
            "budget_policy": self.budget_policy.label(),
            "budget_granted": self.budget_granted,
            "candidate": {
                "seg_dir": dir_name(self.candidate.seg_dir),
                "b_stroke": self.candidate.b_stroke,
                "candidate_extreme": self.candidate.candidate_extreme,
            },
            "input_stroke_count": self.input_stroke_count,
            "from_stroke": f.from_stroke,
            "through_stroke": f.through_stroke,
            "consumed_len": self.state.consumed_len,
            "consumed_digest": format!("{:016x}", self.state.consumed_digest),
            "dir_state": self.state.dir_state.map(dir_name),
            "standard": f.standard,
            "breach_watermark": f.breach_watermark,
            "semantic_state": f.semantic_state,
            "killed_by": f.killed_by,
            "matched": f.matched,
        })
    }

    /// 本轮预算已耗尽：未决且封存步数恰等于累计获批预算。
    pub fn exhausted(&self) -> bool {
        !self.state.decided()
            && self
                .budget_granted
                .is_some_and(|g| self.state.consumed_len == g)
    }

    /// 自线形恢复：身份与版本取线形；已封存部分在 `strokes` 上重折叠核验，相符即采纳。
    /// 返回 `(游标, 已核验, 上一轮已耗尽)`；未核验时游标为自 b 笔的空封存态（保留累计预算），
    /// 续扫即重折叠。「上一轮已耗尽」是调度事实，按线形判（未决且封存步数等于累计预算）。
    ///
    /// `budget_policy` = 本次所读预算记录；线形所带版本与之不同即不可读（返回 `None`，调用方新建游标、
    /// 自 b 笔重折叠），不沿用旧版本的累计预算（#1604）。
    pub fn from_wire(
        wire: &serde_json::Value,
        strokes: &[Stroke],
        budget_policy: PolicyRef,
    ) -> Option<(Self, bool, bool)> {
        if wire["schema"] != SECOND_FOLD_CURSOR_SCHEMA
            || wire["rule_revision"] != SECOND_CANDIDATE_RULE_REVISION
            || wire["budget_policy"].as_str() != Some(budget_policy.label().as_str())
        {
            return None;
        }
        let c = &wire["candidate"];
        let seg_dir = match c["seg_dir"].as_str()? {
            "UP" => Direction::Up,
            "DOWN" => Direction::Down,
            _ => return None,
        };
        let candidate = SecondCandidateId {
            seg_dir,
            b_stroke: usize::try_from(wire_int(&c["b_stroke"])?).ok()?,
            candidate_extreme: wire_int(&c["candidate_extreme"])?,
        };
        let budget_granted = match &wire["budget_granted"] {
            serde_json::Value::Null => None,
            v => Some(usize::try_from(wire_int(v)?).ok()?),
        };
        let consumed = usize::try_from(wire_int(&wire["consumed_len"])?).ok()?;
        let exhausted = wire["semantic_state"].is_null() && budget_granted == Some(consumed);
        let mut cursor = SecondFoldCursor::new(candidate, 0, budget_policy);
        cursor.budget_granted = budget_granted;
        cursor.input_stroke_count = usize::try_from(wire_int(&wire["input_stroke_count"])?).ok()?;
        let from = cursor.state.facts.from_stroke;
        if from + consumed > strokes.len() {
            return Some((cursor, false, exhausted));
        }
        let mut probe = cursor.clone();
        for k in 0..consumed {
            if probe.state.decided() {
                return Some((cursor, false, exhausted));
            }
            if k + 1 == consumed {
                probe.undo = Some(probe.state.capture());
            }
            probe.state.step(strokes, &candidate);
        }
        let verified = normalized(probe.to_wire()) == normalized(wire.clone());
        Some((if verified { probe } else { cursor }, verified, exhausted))
    }
}

/// 每候选游标的调度簿：S 每轮（cut）一份，跨轮的游标经已发布对象的 K1 线形带入。
///
/// 每轮获批预算 `round_budget`（笔数，0 = 无限）：新候选以它为初始预算；上一轮已耗尽的游标在本轮
/// 先 `grant(round_budget)` 再续扫——预算耗尽只让覆盖暂缺，下一轮接着扫，不会永久停在
/// `INCOMPLETE_RESOURCE_BOUND`。
#[derive(Debug, Clone)]
pub struct SecondFoldStore {
    pub round_budget: u32,
    /// #1604：`round_budget` 出自的预算记录（id/版本）；游标与 S 线形的 `budget_policy` 皆取此值。
    pub policy: PolicyRef,
    restored: std::collections::BTreeMap<(u8, usize, Tick), serde_json::Value>,
    cursors: std::collections::BTreeMap<(u8, usize, Tick), SecondFoldCursor>,
    verified: usize,
    granted: usize,
}

fn store_key(c: &SecondCandidateId) -> (u8, usize, Tick) {
    let d = match c.seg_dir {
        Direction::Up => 0,
        Direction::Down => 1,
    };
    (d, c.b_stroke, c.candidate_extreme)
}

impl SecondFoldStore {
    pub fn new(round_budget: u32, policy: PolicyRef) -> Self {
        SecondFoldStore {
            round_budget,
            policy,
            restored: Default::default(),
            cursors: Default::default(),
            verified: 0,
            granted: 0,
        }
    }

    /// #1604：每轮预算与其记录版本都取自 `ParseConfig`（即记录 `second-seq-scan-budget` 的
    /// `second_seq_window` 字段，与无游标簿路径的第二序列窗口同一个值），不另设常数。
    pub fn from_parse_config(config: &super::super::config::ParseConfig) -> Self {
        SecondFoldStore::new(config.second_seq_scan_window, config.scan_budget)
    }

    /// 登记上一轮的 K1 线形（本轮首次用到该候选时才在当轮输入上核验）。身份不可读的线形丢弃。
    pub fn restore(&mut self, wire: &serde_json::Value) {
        let c = &wire["candidate"];
        let dir = match c["seg_dir"].as_str() {
            Some("UP") => Direction::Up,
            Some("DOWN") => Direction::Down,
            _ => return,
        };
        let (Some(b), Some(p)) = (wire_int(&c["b_stroke"]), wire_int(&c["candidate_extreme"]))
        else {
            return;
        };
        let Ok(b) = usize::try_from(b) else { return };
        let key = store_key(&SecondCandidateId {
            seg_dir: dir,
            b_stroke: b,
            candidate_extreme: p,
        });
        self.restored.insert(key, wire.clone());
    }

    /// 本轮折叠该候选：已有游标即续扫；否则自上一轮线形恢复或新建。
    pub(super) fn fold(
        &mut self,
        candidate: SecondCandidateId,
        strokes: &[Stroke],
    ) -> SecondSequenceFacts {
        let key = store_key(&candidate);
        if !self.cursors.contains_key(&key) {
            let cursor = match self
                .restored
                .remove(&key)
                .and_then(|w| SecondFoldCursor::from_wire(&w, strokes, self.policy))
            {
                Some((mut cursor, verified, exhausted)) => {
                    self.verified += usize::from(verified);
                    if exhausted {
                        cursor.grant(self.round_budget);
                        self.granted += 1;
                    }
                    cursor
                }
                None => SecondFoldCursor::new(candidate, self.round_budget, self.policy),
            };
            self.cursors.insert(key, cursor);
        }
        let cursor = self.cursors.get_mut(&key).expect("上句已插入");
        match cursor.resume(strokes) {
            Ok(facts) => facts,
            Err(_) => cursor.refold(strokes),
        }
    }

    /// 本轮用到的该候选游标（供 S 写入 K1 线形）。
    pub fn cursor(&self, candidate: &SecondCandidateId) -> Option<&SecondFoldCursor> {
        self.cursors.get(&store_key(candidate))
    }

    /// 本 cut 的续扫检查点：只含调度相关的游标——累计预算大于每轮预算，或本轮预算已耗尽。其余
    /// 游标下一轮以每轮预算新建即与续扫逐位相同，不必存；预算 0（无限）时恒为空。
    pub fn checkpoint_cursors(&self) -> impl Iterator<Item = &SecondFoldCursor> {
        let round = self.round_budget as usize;
        self.cursors.values().filter(move |c| {
            c.budget_granted
                .is_some_and(|g| (round != 0 && g > round) || c.exhausted())
        })
    }

    pub fn checkpoint(&self) -> Vec<serde_json::Value> {
        self.checkpoint_cursors()
            .map(SecondFoldCursor::to_wire)
            .collect()
    }

    /// 诊断读数：(本轮自线形核验通过的游标数, 本轮因耗尽而追加预算的游标数)。
    pub fn round_counts(&self) -> (usize, usize) {
        (self.verified, self.granted)
    }
}
