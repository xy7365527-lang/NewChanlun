//! #1392 → #1405：生产新笔的唯一判据（bi.md §2/§4/§7/§7.1）。
//!
//! 输入必须携带包含后的中心位置、完整 raw 成员及实际极值根；缺项不换档。
//! 同价按 #1343 裁定（bi.md §7.1，策略版本 [`SAME_PRICE_POLICY`]）：相邻同型同价保留在先者
//! （`077-第77课.md:54/:58`），端点的实际极值身份是极值根**集合**，raw 间隔按极值区计；
//! 同价域不再有任何未定或冻结态，扫描从不弹出已成立的笔。
use super::super::config::ParseConfig;
use super::super::types::{Bar, Direction, FractalKind, Stroke};
#[cfg(test)]
use super::inclusion::process_inclusion_with_facts;
use super::inclusion::InclusionFacts;
use serde::Serialize;
use std::collections::BTreeMap;
use std::rc::Rc;

pub const RULE: &str = "new-bi-dual-coordinate/2";
/// #1343 Q5：同价策略版本。进 [`StrokeFacts::policy`] 与 S 发布的 `policy_version`；
/// 任何改动开新版本号，旧代际对象 AsKnown 保留、不回填（AT-14）。
pub const SAME_PRICE_POLICY: &str = "same-price/keep-earlier+extreme-region/1";
/// #1343 Q1：同型同价保留在先者时记入 [`PairFact::evidence`] 的证据标签（证据，不是等待原因）。
pub const SAME_KIND_EQUAL_PRICE_KEEP_EARLIER: &str = "same_kind_equal_price_keep_earlier";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Endpoint {
    pub kind: &'static str,
    pub merged_index: usize,
    /// 组锚（组内首根 `source_index`）：笔坐标与身份锚，端点键 = `(kind, group_anchor)`。
    pub group_anchor: usize,
    pub price: i64,
    /// 实际极值根集合（CC-005 `ACTUAL_EXTREMUM_MEMBER`，到达序）。#1343 Q2：端点的实际极值身份
    /// 是整个集合，不是其中某一根；多根组没有平局键。
    pub extreme_roots: Vec<usize>,
    /// 极值根集在 raw 序位域（`positions`）上的首/末序位；单根时二者相等。
    /// 只进条件 2（极值区 raw 间隔）与追溯，不作笔坐标、不作身份。
    pub raw_first: usize,
    pub raw_last: usize,
    pub source_coords: Vec<usize>,
    pub sealed_at: Option<usize>,
    pub waiting_reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Conditions {
    pub merged_gap: usize,
    /// #1343 Q2/Q3：两端点极值区之间的 raw 根数 `b.raw_first − a.raw_last − 1`，恒可算。
    pub raw_between_actual_extrema: usize,
    pub top_price: i64,
    pub bottom_price: i64,
    /// 三条件恒为确定值（同价域无未知位，#1343 Q3）；保留 `Option` 只为不混同 false 与未知。
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
    /// 同型取舍只有 `KEEP` / `REPLACE` 两值（#1343 Q1）。
    pub selection: Option<&'static str>,
    pub retained_anchors: Vec<usize>,
    /// 判定证据（非等待态），如 [`SAME_KIND_EQUAL_PRICE_KEEP_EARLIER`]。
    pub evidence: Vec<&'static str>,
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
            // 它继续独立供 `raw_first`/`raw_last`（极值区 raw 间隔，#1343）与追溯使用，
            // 不充当笔端点坐标。
            start_index: self.start.group_anchor,
            end_index: self.end.group_anchor,
            start_price: self.start.price,
            end_price: self.end.price,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StrokeFacts {
    /// 产出本批事实的同价策略版本（#1343 Q5）。
    pub policy: &'static str,
    pub endpoints: Vec<Endpoint>,
    pub pairs: Vec<PairFact>,
    pub strokes: Vec<BiFact>,
    pub waiting_reasons: Vec<&'static str>,
}
impl Default for StrokeFacts {
    fn default() -> Self {
        Self {
            policy: SAME_PRICE_POLICY,
            endpoints: Vec::new(),
            pairs: Vec::new(),
            strokes: Vec::new(),
            waiting_reasons: Vec::new(),
        }
    }
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
/// 这是 `Endpoint.raw_first/raw_last` 的唯一来源——源坐标可以稀疏，不能拿坐标相减冒充根数。
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

/// #1343 Q2/Q3：极值根集在 raw 序位域上的首/末序位（集合语义：与根的到达序、与指定哪一根无关）。
///
/// 根集非空且每根都登记了 raw 序位，是包含层的构造不变式（建组即以首根为极值根，归并只替换
/// 不清空；`positions` 逐 raw 登记）——它不是等待态。违反即实现缺陷：调试构建由
/// `debug_assert!` 先行定位，任何构建都 fail-loud，不以回退值继续判。
fn extreme_region(roots: &[usize], positions: &BTreeMap<usize, usize>) -> (usize, usize) {
    debug_assert!(
        !roots.is_empty(),
        "#1343 Q3 不变式：已成组端点的极值根集非空"
    );
    let mut span: Option<(usize, usize)> = None;
    for root in roots {
        let p = *positions
            .get(root)
            .unwrap_or_else(|| panic!("#1343 Q3 不变式破坏：极值根 {root} 无 raw 序位"));
        span = Some(span.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p))));
    }
    span.unwrap_or_else(|| panic!("#1343 Q3 不变式破坏：已成组端点的极值根集为空"))
}

/// 同一个三组判据；紧凑路径只延迟展开纯追溯字段，不省略根集合、封口或 raw 序位。
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
    }
    if initial_direction_unsettled {
        waiting.push("initial_direction_unsettled");
    }
    // #1343 Q2：多根同价不再是等待态——身份是整个根集，条件 2 只读其首末序位。
    let (raw_first, raw_last) = extreme_region(roots, positions);
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
        raw_first,
        raw_last,
        source_coords: sorted(sources),
        sealed_at: groups[mid + 1]
            .confirmation_evidence
            .as_ref()
            .map(|e| e.incoming.source_index),
        waiting_reasons: waiting,
    })
}

/// #1343 Q2（bi.md §7 条件 2）：时间先后两端点**极值区之间**的 raw 根数。
///
/// 等于对两端任意指定并列极值根所得间隔的最小值，因此与指定哪一根无关（对任意指定取最严）。
/// 严格递增坐标域内 `b.raw_first > a.raw_last` 由构造保证；裸调用方违反坐标域时按 0 计
/// （条件 2 判 false），不另开等待态。
fn raw_between(a: &Endpoint, b: &Endpoint) -> usize {
    b.raw_first.saturating_sub(a.raw_last + 1)
}

/// CC-008/009 分域。同型走一个比较器的三输出：后者严格更强 ⟹ REPLACE；更弱或相等 ⟹ KEEP
/// （相等保留在先者，#1343 Q1，`077:54/:58`）；异型三条件全部确定地算出（无未知位）。
fn pair(a: &Endpoint, b: &Endpoint, config: &ParseConfig) -> PairFact {
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
    let selection = if stronger { "REPLACE" } else { "KEEP" };
    let mut waiting = if same {
        // 取舍已定：被舍弃端点的待定原因不参与后续判断。
        if stronger {
            b.waiting_reasons.clone()
        } else {
            a.waiting_reasons.clone()
        }
    } else {
        let mut w = a.waiting_reasons.clone();
        w.extend(&b.waiting_reasons);
        w
    };
    waiting.sort_unstable();
    waiting.dedup();
    let evidence = if same && comparison == "EQ" {
        // 在后的同价端点仍是独立身份（组锚区分），只记证据，不作笔端点。
        vec![SAME_KIND_EQUAL_PRICE_KEEP_EARLIER]
    } else {
        vec![]
    };
    let conditions = if same {
        None
    } else {
        let merged_gap = b.merged_index.saturating_sub(a.merged_index);
        let raw_between = raw_between(a, b);
        let (top, bottom) = if a.kind == "TOP" {
            (a.price, b.price)
        } else {
            (b.price, a.price)
        };
        let vector = [
            Some(merged_gap >= 3),
            Some(raw_between >= config.new_stroke_min_gap as usize),
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
        retained_anchors: if same {
            vec![if stronger {
                b.group_anchor
            } else {
                a.group_anchor
            }]
        } else {
            vec![]
        },
        evidence,
        waiting_reasons: waiting,
    }
}

/// 唯一生产入口。异型失败不出笔；同型取舍走 CC-009 的同一比较器（严格更强换端点并延末笔，
/// 否则保留在先者——含同价，#1343 Q1）。
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
/// 同一扫描状态机（current/pair/同型 KEEP（含同价取先者）/REPLACE/formed/确认冻结），
/// 不另写宽松新笔；同价域没有冻结位、不弹笔（#1343 Q4）。
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
    out: StrokeFacts,
}
impl ScanState {
    /// full 与增量的唯一单端点状态转移。写入至多触及转移前的最后一笔（延伸终点或冻结确认
    /// 证书），只追加不弹出（#1343 Q4：同价域没有任何未定或冻结态）。
    fn apply(&mut self, f: &Endpoint, config: &ParseConfig) {
        let Some(a) = self.current.clone() else {
            self.current = Some(f.clone());
            return;
        };
        let p = pair(&a, f, config);
        if a.kind == f.kind {
            // KEEP（更弱或同价，#1343 Q1）不写任何状态；REPLACE 换活动端点并延末笔。
            if p.selection == Some("REPLACE") {
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
        }
        // 异型不成笔（合取含 false）：确定不成笔，略过，不冻结后续候选（#1392）。
        self.out.pairs.push(p);
    }
}

/// 稳定端点前的扫描 checkpoint。单端点转移至多改写当时的末笔、从不弹笔，故只备份该尾，
/// 不复制任何历史 endpoints / pairs / strokes。
#[derive(Debug, Clone)]
struct ScanCheckpoint {
    next_mid: usize,
    current: Option<Endpoint>,
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
            endpoints_len: scan.out.endpoints.len(),
            pairs_len: scan.out.pairs.len(),
            strokes_len: scan.out.strokes.len(),
            waiting_len: scan.out.waiting_reasons.len(),
            last_stroke: scan.out.strokes.last().cloned(),
        }
    }
    fn restore(&self, scan: &mut ScanState) {
        scan.current = self.current.clone();
        scan.out.endpoints.truncate(self.endpoints_len);
        scan.out.pairs.truncate(self.pairs_len);
        scan.out.waiting_reasons.truncate(self.waiting_len);
        // 转移只追加新笔或改写当时的末笔，从不弹笔（#1343 Q4）：退回到 checkpoint 时的笔数，
        // 再恢复当时末笔的冻结版本即可。
        debug_assert!(
            scan.out.strokes.len() >= self.strokes_len,
            "#1343 Q4：checkpoint 之后不得弹出已成立的笔"
        );
        scan.out.strokes.truncate(self.strokes_len);
        if let (Some(saved), Some(last)) = (&self.last_stroke, scan.out.strokes.last_mut()) {
            *last = saved.clone();
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
        let was_unsettled = self.facts.unsettled();
        self.facts.append(bar);
        if was_unsettled && !self.facts.unsettled() {
            // #1393：W 激活重折 raw0；未定期的等待态/扫描缓存不得跨越该边界。
            self.config = None;
        }
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
        // #1393：未定初向没有正式组或可冻结端点；激活时已清空等待态。
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
    pub(super) fn without_expanded_sources(mut facts: StrokeFacts) -> StrokeFacts {
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

    pub(super) fn check_incremental_prefixes(raw: &[Bar], label: &str) -> IncrStrokes {
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
                "{label} prefix {}: 包括封口、极值根集与首末序位、条件、证据的全部结构字段",
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

    pub(super) fn hl_bars(hl: &[(i64, i64)], stride: usize) -> Vec<Bar> {
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
    fn tb02b_irrelevant_low_roots_do_not_block_unique_top() {
        // #1343：顶组 high 根唯一，low 同价不参与该端点的 raw 间隔。
        let hl = [
            (3, 1),
            (6, 4),
            (10, 5),
            (9, 7),
            (8, 7),
            (8, 5),
            (6, 3),
            (4, 1),
            (5, 2),
            (6, 3),
        ];
        for stride in [1, 7] {
            let raw = hl_bars(&hl, stride);
            let inclusion = process_inclusion_with_facts(&raw);
            assert_eq!(inclusion.groups[2].high_sources, vec![2 * stride]);
            assert_eq!(
                inclusion.groups[2].low_sources,
                vec![3 * stride, 4 * stride]
            );
            let facts = build_stroke_facts(&inclusion, &ParseConfig::default());
            assert_eq!(facts.strokes.len(), 1);
            let stroke = &facts.strokes[0];
            assert_eq!(stroke.start.group_anchor, 2 * stride);
            assert_eq!(stroke.end.group_anchor, 7 * stride);
            assert_eq!(stroke.formation.vector, [Some(true); 3]);
            assert_eq!(stroke.formation.raw_between_actual_extrema, 4);
            check_incremental_prefixes(&raw, "irrelevant_low_roots");
        }
    }

    #[test]
    fn tb02b_false_pair_allows_later_unique_stronger_endpoint() {
        // #1343：早期 pair 对多根顶组 {2,4} 按极值区确定地判 [F,F,T]（不再有未知位）；
        // 后来的严格更高顶换端点，不需要挑选旧同价根。
        let hl = [
            (3, 1),
            (6, 4),
            (10, 5),
            (9, 6),
            (10, 7),
            (8, 5),
            (9, 6),
            (11, 8),
            (14, 10),
            (12, 8),
            (10, 6),
            (8, 4),
            (6, 2),
            (7, 3),
            (8, 4),
        ];
        for stride in [1, 11] {
            let raw = hl_bars(&hl, stride);
            let facts =
                build_stroke_facts(&process_inclusion_with_facts(&raw), &ParseConfig::default());
            assert_eq!(
                facts.pairs[0].conditions.as_ref().unwrap().vector,
                [Some(false), Some(false), Some(true)]
            );
            assert_eq!(
                facts.pairs[0].old.extreme_roots,
                vec![2 * stride, 4 * stride]
            );
            assert_eq!(
                (facts.pairs[0].old.raw_first, facts.pairs[0].old.raw_last),
                (2, 4)
            );
            assert_eq!(facts.pairs[1].selection, Some("REPLACE"));
            assert!(facts.pairs[1].waiting_reasons.is_empty());
            assert_eq!(facts.strokes.len(), 1);
            assert_eq!(facts.strokes[0].start.group_anchor, 8 * stride);
            assert_eq!(facts.strokes[0].end.group_anchor, 12 * stride);
            assert_eq!(facts.strokes[0].formation.raw_between_actual_extrema, 3);
            check_incremental_prefixes(&raw, "false_pair_then_unique_extreme");
        }
    }

    #[test]
    fn tb02v_threshold_crossing_roots_use_extreme_region() {
        // #1343 Q2（极值区）：顶组根 raw1 / raw2 到底根 raw5 分别隔 3 / 2 根——任取最早根会放行，
        // 集合语义取极值区之间的 2 根 ⟹ 条件 2 确定为 false，不挑根、不等待。
        let raw = hl_bars(
            &[
                (3, 1),
                (8, 4),
                (8, 5),
                (6, 3),
                (5, 2),
                (4, 1),
                (5, 2),
                (6, 3),
            ],
            7,
        );
        let facts =
            build_stroke_facts(&process_inclusion_with_facts(&raw), &ParseConfig::default());
        assert!(facts.strokes.is_empty());
        assert_eq!(facts.pairs[0].old.extreme_roots, vec![7, 14]);
        assert_eq!(
            (facts.pairs[0].old.raw_first, facts.pairs[0].old.raw_last),
            (1, 2)
        );
        let conditions = facts.pairs[0].conditions.as_ref().unwrap();
        assert_eq!(conditions.raw_between_actual_extrema, 2);
        assert_eq!(conditions.vector, [Some(true), Some(false), Some(true)]);
        assert!(facts.waiting_reasons.is_empty());
        assert!(facts.pairs.iter().all(|p| p.waiting_reasons.is_empty()));
        check_incremental_prefixes(&raw, "threshold_crossing_roots");
    }

    #[test]
    fn tb02b_strict_replacement_before_first_stroke_keeps_all_roots() {
        // #1343：更强同型替换只确定组锚，不要求给新组的多个实际根任选其一。
        let hl = [
            (3, 1),
            (6, 4),
            (10, 5),
            (9, 6),
            (10, 7),
            (8, 5),
            (9, 6),
            (11, 8),
            (14, 10),
            (13, 11),
            (14, 12),
            (12, 8),
            (13, 9),
            (15, 11),
            (18, 14),
            (16, 12),
            (14, 10),
            (12, 8),
            (10, 6),
            (11, 7),
            (12, 8),
        ];
        for reflected in [false, true] {
            let hl: Vec<_> = hl
                .iter()
                .map(|&(h, l)| if reflected { (40 - l, 40 - h) } else { (h, l) })
                .collect();
            let raw = hl_bars(&hl, 7);
            let facts =
                build_stroke_facts(&process_inclusion_with_facts(&raw), &ParseConfig::default());
            assert_eq!(facts.pairs[1].new.extreme_roots, vec![56, 70]);
            assert_eq!(
                (facts.pairs[1].new.raw_first, facts.pairs[1].new.raw_last),
                (8, 10)
            );
            assert_eq!(facts.pairs[2].old.group_anchor, 56);
            assert!(facts.pairs.iter().all(|p| p.waiting_reasons.is_empty()));
            assert_eq!(facts.strokes.len(), 1);
            assert_eq!(facts.strokes[0].start.group_anchor, 98);
            assert_eq!(facts.strokes[0].end.group_anchor, 126);
            check_incremental_prefixes(&raw, "strict_replacement_keeps_all_roots");
        }
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
    fn tb02b_checkpoint_preserves_stroke_across_irrelevant_right_group_tie() {
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
            }
            if i == 7 {
                let inclusion = process_inclusion_with_facts(&raw[..=i]);
                assert_eq!(inclusion.groups.last().unwrap().high_sources, vec![42, 49]);
                assert_eq!(incr.to_result().len(), 1, "右组同价根不是顶端点根");
                assert_eq!(incr.scan.out.strokes[0].end.extreme_roots, vec![35]);
            }
        }
        assert_eq!(
            incr.to_result().len(),
            1,
            "活动右组 high 根 tie 出现及消失均不改变实际顶根唯一的笔"
        );
        check_incremental_prefixes(&raw, "irrelevant_right_group_tie_to_unique");
    }

    #[test]
    fn tb02v_checkpoint_equal_price_keeps_tail_then_replace_extends() {
        // #1343 Q1/Q4（syn1 同型）：同价底 KEEP 先者，不弹已成立的尾笔、不冻结；其后严格更低底
        // 走既有 REPLACE 延末笔。checkpoint 回放 EQ 前后都只改写当时的末笔。
        let endpoint = |kind, mid: usize, price| Endpoint {
            kind,
            merged_index: mid,
            group_anchor: mid * 7,
            price,
            extreme_roots: vec![mid * 7],
            raw_first: mid,
            raw_last: mid,
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
        let before = scan.out.strokes.clone();
        scan.apply(&endpoint("BOTTOM", 11, 15), &config);
        assert_eq!(
            scan.out.strokes, before,
            "同型同价 KEEP 先者：不弹笔、不改尾"
        );
        let eq = scan.out.pairs.last().unwrap();
        assert_eq!((eq.comparison, eq.selection), (Some("EQ"), Some("KEEP")));
        assert_eq!(eq.retained_anchors, vec![63]);
        assert_eq!(eq.evidence, vec![SAME_KIND_EQUAL_PRICE_KEEP_EARLIER]);
        assert!(eq.waiting_reasons.is_empty());
        assert_eq!(scan.current.as_ref().unwrap().group_anchor, 63);
        scan.apply(&endpoint("BOTTOM", 13, 12), &config);
        assert_eq!(scan.out.strokes.len(), 2);
        assert_eq!(scan.out.strokes[1].end.group_anchor, 91);
        assert_eq!(scan.out.strokes[1].end.price, 12);
        assert_eq!(
            scan.out.strokes[0], before[0],
            "前笔所有确认字段保持原冻结版本"
        );
        let extended = scan.out.strokes.clone();
        checkpoint.restore(&mut scan);
        assert_eq!(
            scan.out.strokes, before,
            "checkpoint 恢复当时末笔的冻结版本"
        );
        scan.apply(&endpoint("BOTTOM", 11, 15), &config);
        scan.apply(&endpoint("BOTTOM", 13, 12), &config);
        assert_eq!(scan.out.strokes, extended, "回放同一端点序列逐位相同");
    }

    #[test]
    fn tb02b_checkpoint_many_prefixes_linear_endpoint_work() {
        // 明确无包含的长锯齿：每个波段五根，产生大量真实笔和确认，非空结果。
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
                        conditions.raw_between_actual_extrema as u64,
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
                    if let Some(comparison) = expected["comparison"].as_str() {
                        assert_eq!(p.comparison, Some(comparison), "{id}");
                    }
                    if let Some(retained) = expected["retained_anchors"].as_array() {
                        let retained: Vec<usize> = retained
                            .iter()
                            .map(|a| a.as_u64().unwrap() as usize * stride)
                            .collect();
                        assert_eq!(p.retained_anchors, retained, "{id}");
                    }
                    let evidence: Vec<&str> = expected["evidence"]
                        .as_array()
                        .map(|e| e.iter().map(|x| x.as_str().unwrap()).collect())
                        .unwrap_or_default();
                    assert_eq!(p.evidence, evidence, "{id}");
                    assert!(p.waiting_reasons.is_empty(), "{id}：同价不是等待态");
                }
                if let Some(region) = expected["extreme_region"].as_object() {
                    let anchor = region["anchor"].as_u64().unwrap() as usize * stride;
                    let endpoint = facts
                        .endpoints
                        .iter()
                        .find(|e| e.group_anchor == anchor)
                        .unwrap();
                    let roots: Vec<usize> = region["roots"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r.as_u64().unwrap() as usize * stride)
                        .collect();
                    assert_eq!(endpoint.extreme_roots, roots, "{id}");
                    assert_eq!(
                        (endpoint.raw_first as u64, endpoint.raw_last as u64),
                        (
                            region["raw_first"].as_u64().unwrap(),
                            region["raw_last"].as_u64().unwrap()
                        ),
                        "{id}"
                    );
                    assert!(endpoint.waiting_reasons.is_empty(), "{id}");
                }
                assert!(facts.waiting_reasons.is_empty(), "{id}：同价域无冻结");
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
                assert_eq!((endpoint.raw_first, endpoint.raw_last), (6, 6));
                // raw 间隔按实际极值根算：oracle D = m3 / raw4 / 111。
                let p = sf.pairs.iter().find(|p| p.conditions.is_some()).unwrap();
                let conditions = p.conditions.as_ref().unwrap();
                assert_eq!(conditions.merged_gap, 3, "case {case} stride {stride}");
                assert_eq!(
                    conditions.raw_between_actual_extrema, 4,
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

#[cfg(test)]
#[path = "stroke_tb02v_tests.rs"]
mod tb02v_tests;
