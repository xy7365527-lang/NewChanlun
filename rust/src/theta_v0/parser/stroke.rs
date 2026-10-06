//! #1392：生产新笔的唯一判据（bi.md §2/§4/§7）。
//!
//! 输入必须携带包含后的中心位置、完整 raw 成员及实际极值根；缺项不换档。
//! 同价根与同型端点同价分别保留为未定事实（#1343/#1405）。
use super::super::config::ParseConfig;
use super::super::types::{Bar, Direction, FractalKind, Stroke};
#[cfg(test)]
use super::inclusion::process_inclusion_with_facts;
use super::inclusion::InclusionFacts;
use serde::Serialize;
use std::collections::BTreeMap;
use std::rc::Rc;

pub const RULE: &str = "new-bi-dual-coordinate/1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Endpoint {
    pub kind: &'static str,
    pub merged_index: usize,
    pub group_anchor: usize,
    pub price: i64,
    pub extreme_roots: Vec<usize>,
    pub raw_position: Option<usize>,
    pub source_coords: Vec<usize>,
    pub sealed_at: Option<usize>,
    pub waiting_reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Conditions {
    pub merged_gap: usize,
    pub raw_between_actual_extrema: Option<usize>,
    pub top_price: i64,
    pub bottom_price: i64,
    pub vector: [Option<bool>; 3],
    pub failed_conditions: Vec<&'static str>,
    pub waiting_reasons: Vec<&'static str>,
}
impl Conditions {
    pub fn formed(&self) -> bool {
        self.vector == [Some(true); 3] && self.waiting_reasons.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PairFact {
    pub old: Endpoint,
    pub new: Endpoint,
    pub endpoint_kind_pair: String,
    pub conditions: Option<Conditions>,
    pub comparison: Option<&'static str>,
    pub selection: Option<&'static str>,
    pub retained_anchors: Vec<usize>,
    pub waiting_reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Confirmation {
    pub successor_start: usize,
    pub successor_end: usize,
    pub successor_conditions: Conditions,
    pub right_group_sealed_at: usize,
    pub source_coords: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BiFact {
    /// 逻辑身份只锚形成时的起点；末端延伸不换身份。输入修订代际由 S 绑定。
    pub identity_anchor: usize,
    pub start: Endpoint,
    pub end: Endpoint,
    pub formation: Conditions,
    pub confirmation: Option<Confirmation>,
}
impl BiFact {
    fn stroke(&self) -> Stroke {
        Stroke {
            direction: if self.start.kind == "BOTTOM" {
                Direction::Up
            } else {
                Direction::Down
            },
            // #1392：Stroke 坐标与 `ParseLayer.fractals` 同域——都是分型坐标（组锚），
            // 现役 `anchor_resolver`/`resolve_triple_anchor`/`resolve_foot` 只按该坐标查。
            // 实际极值根（`extreme_roots`）可以落在组内其它 raw 上（D 夹具顶组锚 5、根 6），
            // 它继续独立供 `raw_position`/raw 间隔与追溯使用，不充当笔端点坐标。
            start_index: self.start.group_anchor,
            end_index: self.end.group_anchor,
            start_price: self.start.price,
            end_price: self.end.price,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct StrokeFacts {
    pub endpoints: Vec<Endpoint>,
    pub pairs: Vec<PairFact>,
    pub strokes: Vec<BiFact>,
    pub waiting_reasons: Vec<&'static str>,
}
impl StrokeFacts {
    pub fn production_strokes(&self) -> Vec<Stroke> {
        self.strokes.iter().map(BiFact::stroke).collect()
    }
}

fn sorted(mut xs: Vec<usize>) -> Vec<usize> {
    xs.sort_unstable();
    xs.dedup();
    xs
}

/// 全量 facts 的 raw 序位映射：`source_index → 第几根 raw`（同坐标后者覆盖）。
///
/// 这是 `Endpoint.raw_position` 的唯一来源——源坐标可以稀疏，不能拿坐标相减冒充根数。
/// 增量热路径不物化 `steps`，改由 [`IncrFactsState::positions`] 逐 bar 维护同一映射。
fn positions_of_steps(facts: &InclusionFacts) -> BTreeMap<usize, usize> {
    facts
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.incoming.source_index, i))
        .collect()
}

/// 端点扫描（唯一判据）：full 与增量共享本函数，差别只在 `positions` 的来源。
///
/// `merged`/`groups`/`initial_direction_unsettled` 是全部结构输入；`source_coords` 只进
/// 证据面，不进任何判据（见 [`Endpoint`]）。
fn endpoints_from(
    merged: &[Bar],
    groups: &[super::inclusion::InclusionGroupFact],
    initial_direction_unsettled: bool,
    positions: &BTreeMap<usize, usize>,
) -> Vec<Endpoint> {
    (1..merged.len().saturating_sub(1))
        .filter_map(|mid| {
            endpoint_at(
                merged,
                groups,
                initial_direction_unsettled,
                positions,
                mid,
                true,
            )
        })
        .collect()
}

/// 同一个三组判据；紧凑路径只延迟展开纯追溯字段，不省略根唯一性、封口或 raw 序位。
fn endpoint_at(
    merged: &[Bar],
    groups: &[super::inclusion::InclusionGroupFact],
    initial_direction_unsettled: bool,
    positions: &BTreeMap<usize, usize>,
    mid: usize,
    expand_sources: bool,
) -> Option<Endpoint> {
    let f = super::fractal::detect_fractal_at(merged, mid)?;
    let g = &groups[mid];
    let roots = if f.kind == FractalKind::Top {
        &g.high_sources
    } else {
        &g.low_sources
    };
    let mut waiting = vec![];
    let mut sources = vec![];
    for group in &groups[mid - 1..=mid + 1] {
        if expand_sources {
            sources.extend(&group.members);
            if let Some(e) = &group.direction_evidence {
                sources.extend(&e.source_coords);
            }
            if let Some(e) = &group.confirmation_evidence {
                sources.extend(&e.source_coords);
            }
        }
        if group.high_sources.len() != 1 || group.low_sources.len() != 1 {
            waiting.push("raw_extreme_identity_tie");
        }
    }
    if initial_direction_unsettled {
        waiting.push("initial_direction_unsettled");
    }
    if roots.is_empty() {
        waiting.push("actual_extreme_mapping_missing");
    }
    waiting.sort_unstable();
    waiting.dedup();
    Some(Endpoint {
        kind: if f.kind == FractalKind::Top {
            "TOP"
        } else {
            "BOTTOM"
        },
        merged_index: mid,
        group_anchor: g.bar.source_index,
        price: f.price,
        extreme_roots: roots.clone(),
        raw_position: if roots.len() == 1 {
            positions.get(&roots[0]).copied()
        } else {
            None
        },
        source_coords: sorted(sources),
        sealed_at: groups[mid + 1]
            .confirmation_evidence
            .as_ref()
            .map(|e| e.incoming.source_index),
        waiting_reasons: waiting,
    })
}

/// CC-008/009 分域；所有适用位都算，未知与 false 分开。
fn pair(a: &Endpoint, b: &Endpoint, config: &ParseConfig) -> PairFact {
    let mut waiting = a.waiting_reasons.clone();
    waiting.extend(&b.waiting_reasons);
    waiting.sort_unstable();
    waiting.dedup();
    let same = a.kind == b.kind;
    let comparison = match b.price.cmp(&a.price) {
        std::cmp::Ordering::Less => "LT",
        std::cmp::Ordering::Equal => "EQ",
        std::cmp::Ordering::Greater => "GT",
    };
    let stronger = if a.kind == "TOP" {
        b.price > a.price
    } else {
        b.price < a.price
    };
    let selection = if comparison == "EQ" {
        "UNDETERMINED"
    } else if stronger {
        "REPLACE"
    } else {
        "KEEP"
    };
    if same && comparison == "EQ" {
        waiting.push("same_kind_endpoint_identity_tie");
    }
    let conditions = if same {
        None
    } else {
        let merged_gap = b.merged_index.saturating_sub(a.merged_index);
        let raw_between = match (a.raw_position, b.raw_position) {
            (Some(x), Some(y)) if y > x => Some(y - x - 1),
            _ => None,
        };
        let (top, bottom) = if a.kind == "TOP" {
            (a.price, b.price)
        } else {
            (b.price, a.price)
        };
        let vector = [
            Some(merged_gap >= 3),
            raw_between.map(|n| n >= config.new_stroke_min_gap as usize),
            Some(top > bottom),
        ];
        let labels = [
            "merged_gap_below_3",
            "raw_between_below_minimum",
            "top_not_above_bottom",
        ];
        let failed = vector
            .iter()
            .zip(labels)
            .filter_map(|(v, l)| if *v == Some(false) { Some(l) } else { None })
            .collect();
        if raw_between.is_none() && !waiting.contains(&"actual_extreme_mapping_missing") {
            waiting.push("actual_extreme_mapping_missing");
        }
        Some(Conditions {
            merged_gap,
            raw_between_actual_extrema: raw_between,
            top_price: top,
            bottom_price: bottom,
            vector,
            failed_conditions: failed,
            waiting_reasons: waiting.clone(),
        })
    };
    PairFact {
        old: a.clone(),
        new: b.clone(),
        endpoint_kind_pair: format!("{}/{}", a.kind, b.kind),
        conditions,
        comparison: same.then_some(comparison),
        selection: same.then_some(selection),
        retained_anchors: if !same {
            vec![]
        } else if comparison == "EQ" {
            vec![a.group_anchor, b.group_anchor]
        } else {
            vec![if stronger {
                b.group_anchor
            } else {
                a.group_anchor
            }]
        },
        waiting_reasons: waiting,
    }
}

/// 唯一生产入口。异型失败不出笔；越过失败异型后，同型取舍仍走 CC-009。
/// 同价未定之后不选择任一分支继续扫描，保全后续所有原候选（#1405）。
pub fn build_stroke_facts(facts: &InclusionFacts, config: &ParseConfig) -> StrokeFacts {
    stroke_transition(
        &facts.merged,
        &facts.groups,
        facts.initial_direction_unsettled,
        &positions_of_steps(facts),
        config,
    )
}

/// #1392：full 与增量共享的**唯一**新笔 transition。
///
/// 输入只有结构面五件（merged/groups/未定标志/raw 序位映射/config）——增量热路径由
/// [`super::inclusion::IncrFactsState`] 直接喂入，不物化 `steps` 与历史 `source_coords`。
/// 同一扫描状态机（current/blocked/pair/同型 KEEP/REPLACE/EQ 未定/formed/确认冻结），
/// 不另写宽松新笔。
pub fn build_stroke_facts_with_positions(
    merged: &[Bar],
    groups: &[super::inclusion::InclusionGroupFact],
    initial_direction_unsettled: bool,
    positions: &BTreeMap<usize, usize>,
    config: &ParseConfig,
) -> StrokeFacts {
    stroke_transition(
        merged,
        groups,
        initial_direction_unsettled,
        positions,
        config,
    )
}

fn stroke_transition(
    merged: &[Bar],
    groups: &[super::inclusion::InclusionGroupFact],
    initial_direction_unsettled: bool,
    positions: &BTreeMap<usize, usize>,
    config: &ParseConfig,
) -> StrokeFacts {
    let mut scan = ScanState::default();
    if initial_direction_unsettled {
        scan.out.waiting_reasons.push("initial_direction_unsettled");
    }
    for endpoint in endpoints_from(merged, groups, initial_direction_unsettled, positions) {
        scan.apply(&endpoint, config);
        scan.out.endpoints.push(endpoint);
    }
    scan.out.waiting_reasons.sort_unstable();
    scan.out.waiting_reasons.dedup();
    scan.out
}

#[derive(Debug, Clone, Default)]
struct ScanState {
    current: Option<Endpoint>,
    blocked: bool,
    out: StrokeFacts,
}
impl ScanState {
    /// full 与增量的唯一单端点状态转移。所有写入至多触及转移前的最后一笔。
    fn apply(&mut self, f: &Endpoint, config: &ParseConfig) {
        let Some(a) = self.current.clone() else {
            self.current = Some(f.clone());
            return;
        };
        let mut p = pair(&a, f, config);
        if self.blocked {
            p.waiting_reasons
                .push("earlier_endpoint_identity_unsettled");
            if let Some(c) = &mut p.conditions {
                c.waiting_reasons
                    .push("earlier_endpoint_identity_unsettled");
            }
            self.out.pairs.push(p);
            return;
        }
        if a.kind == f.kind {
            if p.selection == Some("UNDETERMINED") {
                self.out
                    .waiting_reasons
                    .push("same_kind_endpoint_identity_tie");
                self.blocked = true;
                // #1405：已形成尾笔的旧版保留在 S 历史，当前不把任一同价端点当唯一笔尾。
                if self
                    .out
                    .strokes
                    .last()
                    .is_some_and(|s| s.end.group_anchor == a.group_anchor)
                {
                    self.out.strokes.pop();
                }
            } else if p.selection == Some("REPLACE") {
                if p.waiting_reasons.is_empty() {
                    if let Some(last) = self.out.strokes.last_mut() {
                        if last.end.group_anchor == a.group_anchor {
                            let extended = pair(&last.start, f, config);
                            if extended.conditions.as_ref().is_some_and(Conditions::formed) {
                                last.end = f.clone();
                                last.formation = extended.conditions.unwrap();
                            }
                        }
                    }
                    self.current = Some(f.clone());
                } else {
                    self.blocked = true;
                    self.out.waiting_reasons.extend(&p.waiting_reasons);
                }
            }
        } else if p.conditions.as_ref().is_some_and(Conditions::formed) {
            // #1392：在已选中后继笔的首次封口见证处冻结前笔证书。
            // 后继尾笔以后可延伸到尚未封口的新端点，不能因此丢掉既有前缀证据。
            if let (Some(sealed), Some(previous)) = (f.sealed_at, self.out.strokes.last_mut()) {
                let mut sources = a.source_coords.clone();
                sources.extend(&f.source_coords);
                previous.confirmation = Some(Confirmation {
                    successor_start: a.group_anchor,
                    successor_end: f.group_anchor,
                    successor_conditions: p.conditions.clone().unwrap(),
                    right_group_sealed_at: sealed,
                    source_coords: sorted(sources),
                });
            }
            self.out.strokes.push(BiFact {
                identity_anchor: a.group_anchor,
                start: a,
                end: f.clone(),
                formation: p.conditions.clone().unwrap(),
                confirmation: None,
            });
            self.current = Some(f.clone());
        } else if !p.waiting_reasons.is_empty() {
            self.blocked = true;
            self.out.waiting_reasons.extend(&p.waiting_reasons);
        }
        self.out.pairs.push(p);
    }
}

/// 稳定端点前的扫描 checkpoint。单端点转移至多写旧尾一笔，故只备份该尾，
/// 不复制任何历史 endpoints / pairs / strokes。
#[derive(Debug, Clone)]
struct ScanCheckpoint {
    next_mid: usize,
    current: Option<Endpoint>,
    blocked: bool,
    endpoints_len: usize,
    pairs_len: usize,
    strokes_len: usize,
    waiting_len: usize,
    last_stroke: Option<BiFact>,
}
impl Default for ScanCheckpoint {
    fn default() -> Self {
        Self::capture(&ScanState::default(), 1)
    }
}
impl ScanCheckpoint {
    fn capture(scan: &ScanState, next_mid: usize) -> Self {
        Self {
            next_mid,
            current: scan.current.clone(),
            blocked: scan.blocked,
            endpoints_len: scan.out.endpoints.len(),
            pairs_len: scan.out.pairs.len(),
            strokes_len: scan.out.strokes.len(),
            waiting_len: scan.out.waiting_reasons.len(),
            last_stroke: scan.out.strokes.last().cloned(),
        }
    }
    fn restore(&self, scan: &mut ScanState) {
        scan.current = self.current.clone();
        scan.blocked = self.blocked;
        scan.out.endpoints.truncate(self.endpoints_len);
        scan.out.pairs.truncate(self.pairs_len);
        scan.out.waiting_reasons.truncate(self.waiting_len);
        // 未定尾端点可能 pop 旧尾；先退至其前缀，再恢复冻结尾。
        scan.out
            .strokes
            .truncate(self.strokes_len.saturating_sub(1));
        if let Some(last) = &self.last_stroke {
            scan.out.strokes.push(last.clone());
        }
    }
}

pub fn build_strokes(facts: &InclusionFacts, config: &ParseConfig) -> Vec<Stroke> {
    build_stroke_facts(facts, config).production_strokes()
}

/// #1392：稳定三组端点 checkpoint + 可撤回的活动尾。
#[derive(Debug, Clone, Default)]
pub struct IncrStrokes {
    pub(super) raw: Vec<Bar>,
    strokes: Rc<Vec<Stroke>>,
    facts: super::inclusion::IncrFactsState,
    scan: ScanState,
    checkpoint: ScanCheckpoint,
    config: Option<ParseConfig>,
    stable_prefix: usize,
    last_source_index: Option<usize>,
    #[cfg(test)]
    endpoint_transitions: usize,
    #[cfg(test)]
    scanned_mids: usize,
}
impl IncrStrokes {
    pub fn empty() -> Self {
        Self::default()
    }

    /// 兼容整段 raw 入口；重建状态后仍通过同一增量核供后续逐根使用。
    pub fn append(self, raw: &[Bar], config: &ParseConfig) -> Self {
        let mut next = Self::empty();
        for bar in raw {
            next.append_bar_incremental(*bar);
            next.update_strokes_incremental(config);
        }
        next.raw = raw.to_vec();
        next.stable_prefix = self
            .strokes
            .iter()
            .zip(next.strokes.iter())
            .take_while(|(a, b)| a == b)
            .count();
        next
    }

    pub fn append_bar_incremental(&mut self, bar: Bar) {
        // 裸调用方若违反递增坐标域，positions 可能改历史根序位：撤销所有端点证书。
        if self
            .last_source_index
            .is_some_and(|previous| bar.source_index <= previous)
        {
            self.config = None;
        }
        self.last_source_index = Some(bar.source_index);
        self.facts.append(bar);
    }

    pub(crate) fn facts_state(&self) -> &super::inclusion::IncrFactsState {
        &self.facts
    }

    pub(crate) fn update_strokes_incremental(&mut self, config: &ParseConfig) -> Rc<Vec<Stroke>> {
        // 参数改变须重扫同一判据；常规逐 bar 路径保持 checkpoint。
        let reset = self.config.as_ref() != Some(config);
        if reset {
            self.scan = ScanState::default();
            self.checkpoint = ScanCheckpoint::default();
            self.config = Some(*config);
        }
        let dirty_stroke = self.checkpoint.strokes_len.saturating_sub(1);
        self.checkpoint.restore(&mut self.scan);
        let n = self.facts.merged_slice().len();
        // 当前末组 g=n-1 仍可变；只冻结 mid+1<g，含右组封口证据。
        let stable_mid_end = n.saturating_sub(2).max(1);
        for mid in self.checkpoint.next_mid..n.saturating_sub(1) {
            #[cfg(test)]
            {
                self.scanned_mids += 1;
            }
            if let Some(endpoint) = endpoint_at(
                self.facts.merged_slice(),
                self.facts.groups_slice(),
                self.facts.unsettled(),
                self.facts.positions(),
                mid,
                false,
            ) {
                self.scan.apply(&endpoint, config);
                self.scan.out.endpoints.push(endpoint);
                #[cfg(test)]
                {
                    self.endpoint_transitions += 1;
                }
            }
            if mid < stable_mid_end {
                self.checkpoint = ScanCheckpoint::capture(&self.scan, mid + 1);
            }
        }
        // 未定初向只在三组形成以前触发，此时没有可冻结端点。
        if self.facts.unsettled()
            && !self
                .scan
                .out
                .waiting_reasons
                .contains(&"initial_direction_unsettled")
        {
            self.scan
                .out
                .waiting_reasons
                .push("initial_direction_unsettled");
        }
        let new_len = self.scan.out.strokes.len();
        let mut same = if reset {
            0
        } else {
            dirty_stroke.min(self.strokes.len()).min(new_len)
        };
        while same < self.strokes.len().min(new_len)
            && self.strokes[same] == self.scan.out.strokes[same].stroke()
        {
            same += 1;
        }
        self.stable_prefix = same;
        if same != self.strokes.len() || same != new_len {
            let output = Rc::make_mut(&mut self.strokes);
            output.truncate(same);
            output.extend(self.scan.out.strokes[same..].iter().map(BiFact::stroke));
        }
        Rc::clone(&self.strokes)
    }

    pub fn stable_strokes_prefix(&self) -> usize {
        self.stable_prefix
    }
    pub fn to_result(&self) -> &[Stroke] {
        &self.strokes
    }
    pub fn to_result_rc(&self) -> Rc<Vec<Stroke>> {
        self.strokes.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn ledger() -> Value {
        serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02b/raw-ledger.json"
        ))
        .unwrap()
    }
    fn oracle() -> Value {
        serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02b/hand-oracle.json"
        ))
        .unwrap()
    }
    fn bars(case: &Value, stride: usize) -> Vec<Bar> {
        case["raw_bars"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| Bar {
                source_index: r["raw_index"].as_u64().unwrap() as usize * stride,
                timestamp: r["raw_index"].as_i64().unwrap(),
                open: r["open"].as_i64().unwrap(),
                high: r["high"].as_i64().unwrap(),
                low: r["low"].as_i64().unwrap(),
                close: r["close"].as_i64().unwrap(),
                volume: 1.0,
                untradable: false,
            })
            .collect()
    }
    fn without_expanded_sources(mut facts: StrokeFacts) -> StrokeFacts {
        for e in &mut facts.endpoints {
            e.source_coords.clear();
        }
        for p in &mut facts.pairs {
            p.old.source_coords.clear();
            p.new.source_coords.clear();
        }
        for s in &mut facts.strokes {
            s.start.source_coords.clear();
            s.end.source_coords.clear();
            if let Some(c) = &mut s.confirmation {
                c.source_coords.clear();
            }
        }
        facts.waiting_reasons.sort_unstable();
        facts.waiting_reasons.dedup();
        facts
    }

    fn check_incremental_prefixes(raw: &[Bar], label: &str) -> IncrStrokes {
        let config = ParseConfig::default();
        let mut incr = IncrStrokes::empty();
        for (i, bar) in raw.iter().enumerate() {
            let old = incr.to_result_rc();
            let scanned_before = incr.scanned_mids;
            incr.append_bar_incremental(*bar);
            let actual = incr.update_strokes_incremental(&config);
            let full = build_stroke_facts(&process_inclusion_with_facts(&raw[..=i]), &config);
            assert_eq!(
                *actual,
                full.production_strokes(),
                "{label} prefix {}",
                i + 1
            );
            assert_eq!(
                without_expanded_sources(incr.scan.out.clone()),
                without_expanded_sources(full),
                "{label} prefix {}: 包括封口、根、条件、blocked 的全部结构字段",
                i + 1
            );
            let same = old
                .iter()
                .zip(actual.iter())
                .take_while(|(a, b)| a == b)
                .count();
            assert_eq!(
                incr.stable_strokes_prefix(),
                same,
                "{label} prefix {}: 实际逐字段相同前缀",
                i + 1
            );
            assert_eq!(&old[..same], &actual[..same]);
            assert!(
                incr.scanned_mids - scanned_before <= 2,
                "{label}: 每 bar 至多重算两个 mid"
            );
        }
        incr
    }

    fn hl_bars(hl: &[(i64, i64)], stride: usize) -> Vec<Bar> {
        hl.iter()
            .enumerate()
            .map(|(i, &(high, low))| Bar {
                source_index: i * stride,
                timestamp: i as i64,
                open: low,
                high,
                low,
                close: high,
                volume: 1.0,
                untradable: false,
            })
            .collect()
    }

    #[test]
    fn tb02b_checkpoint_frozen_ledger_every_prefix_and_sparse_coordinates() {
        for stride in [1, 7] {
            for c in ledger()["cases"].as_array().unwrap() {
                check_incremental_prefixes(&bars(c, stride), c["case_id"].as_str().unwrap());
            }
        }
    }

    #[test]
    fn tb02b_checkpoint_sealing_ties_and_tail_retraction() {
        let cases: &[(&str, &[(i64, i64)])] = &[
            (
                "initial_direction_unsettled",
                &[(10, 1), (9, 2), (12, 5), (5, 0)],
            ),
            (
                "right_group_seal_then_successor_extension",
                &[
                    (10, 7),
                    (8, 5),
                    (12, 8),
                    (11, 9),
                    (14, 11),
                    (17, 14),
                    (15, 12),
                    (19, 16),
                    (16, 13),
                    (14, 10),
                    (12, 8),
                    (10, 6),
                    (13, 9),
                    (15, 11),
                    (11, 7),
                    (7, 3),
                    (9, 5),
                    (12, 8),
                ],
            ),
            // active right group 同价高根先变非唯一，再以更低高值消除 tie。
            (
                "active_right_root_tie_to_unique",
                &[
                    (10, 7),
                    (8, 5),
                    (12, 8),
                    (14, 10),
                    (16, 12),
                    (18, 14),
                    (15, 11),
                    (15, 10),
                    (14, 11),
                    (13, 8),
                    (17, 12),
                ],
            ),
            // 末组向下降包含合并，活动底分型可在该组三组判据改变时撤回。
            (
                "tail_geometry_retraction",
                &[
                    (12, 9),
                    (8, 5),
                    (10, 7),
                    (13, 10),
                    (16, 13),
                    (19, 16),
                    (17, 14),
                    (16, 12),
                    (15, 13),
                    (20, 11),
                    (14, 9),
                    (18, 15),
                ],
            ),
        ];
        for (name, hl) in cases {
            for stride in [1, 11] {
                check_incremental_prefixes(&hl_bars(hl, stride), name);
            }
        }
    }

    #[test]
    fn tb02b_checkpoint_rolls_back_blocked_when_active_root_becomes_unique() {
        let hl = [
            (10, 7),
            (8, 5),
            (12, 8),
            (14, 10),
            (16, 12),
            (18, 14),
            (15, 11),
            (15, 10),
            (14, 11),
        ];
        let raw = hl_bars(&hl, 7);
        let config = ParseConfig::default();
        let mut incr = IncrStrokes::empty();
        for (i, bar) in raw.iter().enumerate() {
            incr.append_bar_incremental(*bar);
            incr.update_strokes_incremental(&config);
            if i == 6 {
                assert_eq!(incr.to_result().len(), 1);
                assert!(!incr.scan.blocked);
            }
            if i == 7 {
                assert!(incr.scan.blocked);
                assert!(incr.to_result().is_empty());
            }
        }
        assert!(
            !incr.scan.blocked,
            "活动右组 high 根 tie 消失必须撤回 blocked"
        );
        assert_eq!(incr.to_result().len(), 1);
        check_incremental_prefixes(&raw, "observed_tie_to_unique");
    }

    #[test]
    fn tb02b_checkpoint_restores_popped_tail_before_extension() {
        let endpoint = |kind, mid, price| Endpoint {
            kind,
            merged_index: mid,
            group_anchor: mid * 7,
            price,
            extreme_roots: vec![mid * 7],
            raw_position: Some(mid),
            source_coords: vec![],
            sealed_at: Some((mid + 2) * 7),
            waiting_reasons: vec![],
        };
        let config = ParseConfig::default();
        let mut scan = ScanState::default();
        for f in [
            endpoint("BOTTOM", 1, 10),
            endpoint("TOP", 5, 30),
            endpoint("BOTTOM", 9, 15),
        ] {
            scan.apply(&f, &config);
            scan.out.endpoints.push(f);
        }
        assert_eq!(scan.out.strokes.len(), 2);
        let checkpoint = ScanCheckpoint::capture(&scan, 10);
        let first = scan.out.strokes[0].clone();
        scan.apply(&endpoint("BOTTOM", 11, 15), &config);
        assert!(scan.blocked);
        assert_eq!(scan.out.strokes.len(), 1, "同型EQ撤去原尾笔");
        checkpoint.restore(&mut scan);
        assert!(!scan.blocked);
        scan.apply(&endpoint("BOTTOM", 11, 12), &config);
        assert_eq!(scan.out.strokes.len(), 2);
        assert_eq!(scan.out.strokes[1].end.group_anchor, 77);
        assert_eq!(scan.out.strokes[1].end.price, 12);
        assert_eq!(scan.out.strokes[0], first, "前笔所有确认字段保持原冻结版本");
    }

    #[test]
    fn tb02b_checkpoint_many_prefixes_linear_endpoint_work() {
        // 明确无包含的长锯齿：每个波段五根，产生大量真实笔和确认，非 blocked 空结果。
        let hl: Vec<_> = (0..1200)
            .map(|i| {
                let phase = i % 10;
                let p = if phase <= 5 { phase } else { 10 - phase };
                (100 + p * 10, 95 + p * 10)
            })
            .collect();
        let incr = check_incremental_prefixes(&hl_bars(&hl, 13), "long_wave");
        assert!(incr.to_result().len() > 200);
        assert!(incr.endpoint_transitions <= 2 * hl.len());
        assert!(incr.scanned_mids <= 2 * hl.len());
        eprintln!(
            "checkpoint bars={} scanned_mids={} endpoint_transitions={} strokes={}",
            hl.len(),
            incr.scanned_mids,
            incr.endpoint_transitions,
            incr.to_result().len()
        );
    }

    #[test]
    fn tb02b_frozen_raw_ledger_and_sparse_source_coordinates() {
        let theta = super::super::super::config::ThetaConfig::default();
        for stride in [1, 7] {
            for c in ledger()["cases"].as_array().unwrap() {
                let id = c["case_id"].as_str().unwrap();
                let expected = &oracle()["cases"][id];
                let (layer, inclusion) =
                    super::super::parse_layer_with_inclusion_facts(&bars(c, stride), &theta);
                let facts = inclusion.stroke_facts.as_ref().unwrap();
                if let Some(pair) = expected["pair"].as_array() {
                    let p = facts
                        .pairs
                        .iter()
                        .find(|p| {
                            p.old.group_anchor == pair[0].as_u64().unwrap() as usize * stride
                                && p.new.group_anchor == pair[1].as_u64().unwrap() as usize * stride
                        })
                        .unwrap();
                    let conditions = p.conditions.as_ref().unwrap();
                    assert_eq!(
                        serde_json::to_value(conditions.vector).unwrap(),
                        expected["vector"],
                        "{id}"
                    );
                    assert_eq!(
                        conditions.merged_gap as u64,
                        expected["merged_gap"].as_u64().unwrap(),
                        "{id}"
                    );
                    assert_eq!(
                        conditions.raw_between_actual_extrema.unwrap() as u64,
                        expected["raw_between"].as_u64().unwrap(),
                        "{id}"
                    );
                }
                if let Some(strokes) = expected["strokes"].as_array() {
                    // 账簿（ORACLE.md）的笔端点记的是「实际极值根」（底根/顶根），不是组锚——
                    // `internal_root` 底组锚 1、底根 2 即此例。几何事实仍按实际极值根逐位对拍。
                    let roots: Vec<_> = facts
                        .strokes
                        .iter()
                        .map(|s| {
                            serde_json::json!([
                                s.start.extreme_roots[0] / stride,
                                s.end.extreme_roots[0] / stride
                            ])
                        })
                        .collect();
                    assert_eq!(&roots, strokes, "{id}");
                    // #1392 坐标契约：同一批笔的公开坐标 = 分型坐标（组锚），
                    // 必须经现役 anchor_resolver 闭合，实际极值根不得冒充笔端点坐标。
                    let resolve = crate::theta_v0::classifier::projection::anchor_resolver(
                        &layer.fractals,
                        &layer.merged_bars,
                    );
                    for s in facts.production_strokes() {
                        for (x, price) in
                            [(s.start_index, s.start_price), (s.end_index, s.end_price)]
                        {
                            let (resolved, anchor) = resolve(x).unwrap_or_else(|| {
                                panic!("{id}：笔端点 {x} 经 anchor_resolver 不可解")
                            });
                            assert_eq!(anchor, x, "{id}");
                            assert_eq!(resolved, price, "{id}");
                        }
                    }
                }
                if let Some(pair) = expected["same_pair"].as_array() {
                    let p = facts
                        .pairs
                        .iter()
                        .find(|p| {
                            p.old.group_anchor == pair[0].as_u64().unwrap() as usize * stride
                                && p.new.group_anchor == pair[1].as_u64().unwrap() as usize * stride
                        })
                        .unwrap();
                    assert!(p.conditions.is_none());
                    assert_eq!(
                        p.selection.unwrap(),
                        expected["selection"].as_str().unwrap(),
                        "{id}"
                    );
                    if p.selection == Some("UNDETERMINED") {
                        assert_eq!(p.retained_anchors, vec![5 * stride, 7 * stride]);
                        assert!(p
                            .waiting_reasons
                            .contains(&"same_kind_endpoint_identity_tie"));
                    }
                }
                if let Some(anchor) = expected["ambiguous_root_anchor"].as_u64() {
                    let endpoint = facts
                        .endpoints
                        .iter()
                        .find(|e| e.group_anchor == anchor as usize * stride)
                        .unwrap();
                    assert_eq!(endpoint.extreme_roots, vec![stride, 2 * stride]);
                    assert!(endpoint.raw_position.is_none());
                    assert!(endpoint
                        .waiting_reasons
                        .contains(&"raw_extreme_identity_tie"));
                }
                if let Some(confirmed) = expected["confirmed"].as_array() {
                    let actual: Vec<_> = facts
                        .strokes
                        .iter()
                        .map(|s| Value::Bool(s.confirmation.is_some()))
                        .collect();
                    assert_eq!(&actual, confirmed, "{id}");
                }
            }
        }
    }
    /// #1392 缺陷一回归：独立 oracle trace D / D_REFLECTED（逐根 H/L 由 ORACLE.json 抄录，
    /// 不取自产品 fixtures）。
    ///
    /// D 的顶组是合并组 `[raw5, raw6]`：组锚 5、达到 17 的实际极值根 6 —— 两者不同。
    /// 现役 `ParseLayer.fractals` = `detect_fractals(merged)` 的分型坐标（组锚），
    /// `classifier::projection::anchor_resolver` 也只按该坐标查。
    /// 生产笔端点必须落在同一坐标上才能闭合；实际极值根仍独立留在事实里算 raw 间隔。
    #[test]
    fn tb02b_d_fixture_production_endpoints_close_through_anchor_resolver() {
        // 独立 oracle：trace D / D_REFLECTED（H′ = 30 − L，L′ = 30 − H）。
        let cases: [[(i64, i64); 8]; 2] = [
            [
                (12, 9),
                (8, 5),
                (7, 6),
                (10, 8),
                (13, 10),
                (16, 14),
                (17, 13),
                (15, 12),
            ],
            [
                (21, 18),
                (25, 22),
                (24, 23),
                (22, 20),
                (20, 17),
                (16, 14),
                (17, 13),
                (18, 15),
            ],
        ];
        let cfg = super::super::super::config::ThetaConfig::default();
        for (case, hl) in cases.iter().enumerate() {
            for stride in [1usize, 7] {
                let raw: Vec<Bar> = hl
                    .iter()
                    .enumerate()
                    .map(|(i, (high, low))| Bar {
                        source_index: i * stride,
                        timestamp: i as i64,
                        open: (high + low) / 2,
                        high: *high,
                        low: *low,
                        close: (high + low) / 2,
                        volume: 1.0,
                        untradable: false,
                    })
                    .collect();
                let (layer, facts) = super::super::parse_layer_with_inclusion_facts(&raw, &cfg);
                let sf = facts.stroke_facts.as_ref().unwrap();
                // 分型账本坐标 = 组锚：底@1、顶@5（镜像为顶@1、底@5）。
                assert_eq!(
                    layer
                        .fractals
                        .iter()
                        .map(|f| f.source_index)
                        .collect::<Vec<_>>(),
                    vec![stride, 5 * stride],
                    "case {case} stride {stride}"
                );
                // 真实 production_strokes 端点必须经现役 anchor_resolver 闭合。
                let strokes = sf.production_strokes();
                assert_eq!(strokes.len(), 1, "case {case} stride {stride}");
                let s = strokes[0];
                let resolve = crate::theta_v0::classifier::projection::anchor_resolver(
                    &layer.fractals,
                    &layer.merged_bars,
                );
                for (x, price) in [(s.start_index, s.start_price), (s.end_index, s.end_price)] {
                    let (resolved, anchor) = resolve(x).unwrap_or_else(|| {
                        panic!("case {case} stride {stride}：端点 {x} 经 anchor_resolver 不可解")
                    });
                    assert_eq!(anchor, x);
                    assert_eq!(resolved, price);
                }
                // 实际极值根（非组锚）不能充当 Stroke 坐标：它不在分型账本上。
                assert!(
                    resolve(6 * stride).is_none(),
                    "case {case} stride {stride}：实际极值根不是分型坐标，不得当笔端点"
                );
                // 实际极值根仍独立保留在事实上（组锚 5、根 6）。
                let endpoint = sf
                    .endpoints
                    .iter()
                    .find(|e| e.group_anchor == 5 * stride)
                    .unwrap();
                assert_eq!(endpoint.extreme_roots, vec![6 * stride]);
                assert_eq!(endpoint.raw_position, Some(6));
                // raw 间隔按实际极值根算：oracle D = m3 / raw4 / 111。
                let p = sf.pairs.iter().find(|p| p.conditions.is_some()).unwrap();
                let conditions = p.conditions.as_ref().unwrap();
                assert_eq!(conditions.merged_gap, 3, "case {case} stride {stride}");
                assert_eq!(
                    conditions.raw_between_actual_extrema,
                    Some(4),
                    "case {case} stride {stride}"
                );
                assert_eq!(conditions.vector, [Some(true); 3]);
            }
        }
    }

    #[test]
    fn tb02b_confirmed_prefix_survives_successor_tail_extension() {
        // 独立 RIGHT_GROUP_SEALED_COMPONENT 的 14 根前缀，再追加更低的后继底。
        let hl = [
            (10, 7),
            (8, 5),
            (12, 8),
            (11, 9),
            (14, 11),
            (17, 14),
            (15, 12),
            (19, 16),
            (16, 13),
            (14, 10),
            (12, 8),
            (10, 6),
            (13, 9),
            (15, 11),
            (11, 7),
            (7, 3),
            (9, 5),
            (12, 8),
        ];
        let raw: Vec<Bar> = hl
            .iter()
            .enumerate()
            .map(|(i, &(high, low))| Bar {
                source_index: i,
                timestamp: i as i64,
                open: (high + low) / 2,
                high,
                low,
                close: (high + low) / 2,
                volume: 1.0,
                untradable: false,
            })
            .collect();
        let cfg = super::super::super::config::ThetaConfig::default();
        let frozen = build_stroke_facts(&process_inclusion_with_facts(&raw[..14]), &cfg.parse);
        let certificate = frozen.strokes[0].confirmation.clone().unwrap();
        assert_eq!(certificate.successor_end, 11);
        assert_eq!(certificate.right_group_sealed_at, 13);
        for n in 15..=raw.len() {
            let full = build_stroke_facts(&process_inclusion_with_facts(&raw[..n]), &cfg.parse);
            assert_eq!(full.strokes[0].end.group_anchor, 7);
            assert_eq!(
                full.strokes[0].confirmation.as_ref(),
                Some(&certificate),
                "prefix {n}"
            );
            if n >= 17 {
                assert_eq!(full.strokes[1].end.group_anchor, 15);
            }
        }
    }

    #[test]
    fn tb02b_raw_prefix_formation_confirmation_and_parser_parity() {
        let cfg = super::super::super::config::ThetaConfig::default();
        for c in ledger()["cases"].as_array().unwrap() {
            let raw = bars(c, 1);
            let mut incr = super::super::ParseLayerIncr::new(&cfg);
            for (index, bar) in raw.iter().enumerate() {
                let (full, facts) =
                    super::super::parse_layer_with_inclusion_facts(&raw[..=index], &cfg);
                assert_eq!(
                    incr.append(*bar),
                    full,
                    "{} prefix {}",
                    c["case_id"],
                    index + 1
                );
                assert_eq!(
                    *full.strokes,
                    facts.stroke_facts.as_ref().unwrap().production_strokes()
                );
                if c["case_id"] == "lifecycle" && index >= 6 {
                    let bi = &facts.stroke_facts.as_ref().unwrap().strokes[0];
                    assert_eq!(bi.identity_anchor, 1);
                    assert_eq!(bi.confirmation.is_some(), index >= 11);
                    if let Some(w) = &bi.confirmation {
                        assert_eq!(w.right_group_sealed_at, 11);
                    }
                }
            }
        }
    }
}
