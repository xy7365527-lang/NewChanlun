//! #1373：只投影同次 parser 事实；比较与描述不参与结构/交易准入。
//!
//! #1394 2C：CC-007 的 24 比较单元按「精确分型参考价/源身份 × 后续 event 版本」缓存，
//! 每条 bar 的 4×6 单元只求值一次；同分型追加后续 bar 为有序追加，修订只重算受影响单元
//! 并按内容键核过，旧 cut 原件（已发布的不可变 Value）不动。
use super::*;
use newchan_rust::theta_v0::parser::inclusion::{
    InclusionDirectionEvidence, InclusionFacts, InclusionGroupFact, InclusionStepFact,
};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

fn bar(b: &Bar) -> Value {
    json!({"source_coord":b.source_index.to_string(),"open":b.open.to_string(),"high":b.high.to_string(),"low":b.low.to_string(),"close":b.close.to_string()})
}
fn direction(e: &Option<InclusionDirectionEvidence>) -> Value {
    e.as_ref().map(|e|json!({"direction":dir_str(e.direction),"established_at":e.incoming.source_index.to_string(),
        "previous_acc":bar(&e.previous_acc),"incoming":bar(&e.incoming),"source_coords":coords(&e.source_coords)})).unwrap_or(Value::Null)
}
fn coords(xs: &[usize]) -> Vec<String> {
    xs.iter().map(ToString::to_string).collect()
}
fn order(a: i64, b: i64) -> &'static str {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => "LT",
        std::cmp::Ordering::Equal => "EQ",
        std::cmp::Ordering::Greater => "GT",
    }
}

/// #1394 2C：单个 CC-007 后续 bar 的比较单元（`{"bar":…,"relations":[…]}` 的**精确原件**）。
pub(super) struct UnitEntry {
    /// 该 bar 的原始 OHLC 投影（与全量入口 `raw_bar` 逐字段相同）。
    bar: Value,
    /// 4 轴 × 参考价数（当前 6）= 24 个比较单元，顺序与全量入口相同。
    relations: Value,
    /// 精确内容键：源坐标 + revision + 4 价 + 参考集指纹。**不核此键不得复用**。
    key: String,
}

/// #1394 2C：CC-007 比较单元缓存（性能结构，不充任准入/证据权威）。
///
/// - 键：分型参考集指纹（精确参考价/源身份/名字）→ 源坐标 → 精确内容键。
/// - 命中要求**当前内容键逐字相等**；同源坐标异内容（修订/同 ID 改内容）一律重算并替换。
/// - 容量不足时整表清空（冷/暖/超限输出一致）；旧版本条目在容量内保留（供回退版本复用）。
pub(super) struct UnitsCache {
    /// 参考集指纹 → 源坐标 → **精确内容键分列的版本表**（旧版本保留，供回退版本复用）。
    by_refs: HashMap<String, BTreeMap<i64, Vec<Rc<UnitEntry>>>>,
    capacity: usize,
    entries: usize,
    computed: usize,
    reused: usize,
    invalidated: usize,
    evictions: usize,
}

impl Default for UnitsCache {
    fn default() -> Self {
        Self::new(250_000)
    }
}

impl UnitsCache {
    /// 同一源坐标保留的版本数上限（FIFO；旧版本保留是为了回退/返回时仍能命中原件）。
    const VERSIONS_PER_COORD: usize = 4;

    pub(super) fn new(capacity: usize) -> Self {
        UnitsCache {
            by_refs: HashMap::new(),
            capacity,
            entries: 0,
            computed: 0,
            reused: 0,
            invalidated: 0,
            evictions: 0,
        }
    }

    /// 实际完成 24 单元求值的次数（每 (参考集, bar 版本) 计 1）。
    #[cfg(test)]
    pub(super) fn computed(&self) -> usize {
        self.computed
    }

    /// 内容键逐字相等而直接复用的次数。
    #[cfg(test)]
    pub(super) fn reused(&self) -> usize {
        self.reused
    }

    /// 同源坐标内容键变化（真实修订）而重算的次数。
    #[cfg(test)]
    pub(super) fn invalidated(&self) -> usize {
        self.invalidated
    }

    #[cfg(test)]
    pub(super) fn evictions(&self) -> usize {
        self.evictions
    }

    /// 真实修订：源坐标 `coord` 起的条目失效（`>= coord` 的精确内容键不再可信）。
    /// 保留其它坐标与其它参考集的旧版本原件（旧 cut 原件不动）。
    pub(super) fn invalidate_from(&mut self, coord: i64) {
        for by_coord in self.by_refs.values_mut() {
            let dropped: usize = by_coord
                .range(coord..)
                .map(|(_, versions)| versions.len())
                .sum();
            by_coord.retain(|key, _| *key < coord);
            self.entries -= dropped;
        }
    }

    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        self.by_refs.clear();
        self.entries = 0;
    }

    fn clear_all(&mut self) {
        self.by_refs.clear();
        self.entries = 0;
        self.evictions += 1;
    }

    /// 取（或按精确内容键算一次）后续 bar 的比较单元。
    fn unit(
        &mut self,
        refs_key: &str,
        coord: i64,
        content_key: String,
        compute: impl FnOnce() -> UnitEntry,
    ) -> Rc<UnitEntry> {
        if let Some(versions) = self
            .by_refs
            .get(refs_key)
            .and_then(|by_coord| by_coord.get(&coord))
        {
            if let Some(entry) = versions.iter().find(|entry| entry.key == content_key) {
                self.reused += 1;
                return Rc::clone(entry);
            }
            if !versions.is_empty() {
                self.invalidated += 1;
            }
        }
        self.computed += 1;
        if self.entries >= self.capacity {
            self.clear_all();
        }
        let entry = Rc::new(compute());
        let versions = self
            .by_refs
            .entry(refs_key.to_string())
            .or_default()
            .entry(coord)
            .or_default();
        // 同一坐标的版本表 FIFO 封顶：容量有限时只失去复用，不改变判据与输出。
        if versions.len() >= Self::VERSIONS_PER_COORD {
            versions.remove(0);
            self.entries -= 1;
        }
        versions.push(Rc::clone(&entry));
        self.entries += 1;
        entry
    }
}

/// 后续 bar 的精确内容键（源坐标 + revision + 4 价）——修订即键变，杜绝同 ID 未核内容兜底。
fn unit_content_key(coord: i64, event: &Value, bar: &Value) -> String {
    canonical_json(&json!({
        "source_coord": coord.to_string(),
        "revision": event["revision"],
        "input_revision": event["input_revision"],
        "open": bar["open"],
        "high": bar["high"],
        "low": bar["low"],
        "close": bar["close"],
    }))
}
fn comparisons(a: Option<&Bar>, b: &Bar) -> (Value, Value) {
    let Some(a) = a else {
        return (json!([]), json!([]));
    };
    let comparisons = json!([{"axis":"high","acc":a.high.to_string(),"incoming":b.high.to_string(),"order":order(a.high,b.high)},
        {"axis":"low","acc":a.low.to_string(),"incoming":b.low.to_string(),"order":order(a.low,b.low)}]);
    let mut endpoints = vec![
        (a.low, "acc.low"),
        (a.high, "acc.high"),
        (b.low, "incoming.low"),
        (b.high, "incoming.high"),
    ];
    endpoints.sort();
    let mut classes: Vec<Vec<&str>> = Vec::new();
    let mut previous = None;
    for (price, label) in endpoints {
        if previous != Some(price) {
            classes.push(vec![]);
        }
        classes.last_mut().unwrap().push(label);
        previous = Some(price);
    }
    (comparisons, json!(classes))
}
pub(super) fn raw_ref(raw: &Value) -> Value {
    json!({"identity_key":raw["identity_key"],"payload_hash":raw["payload_hash"],"receipt_id":raw["receipt_id"],
    "event_id":raw["event_id"],"input_revision":raw["input_revision"],"revision":raw["revision"].as_i64().unwrap().to_string(),
    "seq":raw["seq"].as_i64().unwrap().to_string(),"source_coord":raw["source_coord"]})
}

pub(super) struct Builder<'a> {
    pub(super) raw: &'a BTreeMap<i64, Value>,
    pub(super) generation: i64,
    pub(super) cut: &'a str,
    pub(super) objects: Vec<Value>,
}
impl Builder<'_> {
    /// 纯构造：与 [`Builder::add`] 同一条内容寻址（object_id = kind+fact_key+payload+refs+档案+规则）。
    pub(super) fn build_object(
        &self,
        kind: &str,
        slot: Value,
        payload: Value,
        mut sources: Vec<usize>,
    ) -> Value {
        sources.sort_unstable();
        sources.dedup();
        let refs: Vec<Value> = sources
            .iter()
            .map(|s| {
                raw_ref(
                    self.raw
                        .get(&(*s as i64))
                        .expect("parser来源必须在当前原始账"),
                )
            })
            .collect();
        let key = canonical_json(&slot);
        let id = format!(
            "obj-{}",
            sha256_hex(
                canonical_json(
                    &json!({"kind":kind,"fact_key":key,"payload":payload,"input_refs":refs,
            "profile_id":tb02a::PROFILE,"rule_revision":RULE_REVISION})
                )
                .as_bytes()
            )
        );
        json!({"object_id":id,"object_revision":1,"kind":kind,"fact_key":key,
            "payload_json":canonical_json(&payload),"input_refs_json":canonical_json(&json!(refs)),"source_coords_json":canonical_json(&json!(coords(&sources))),
            "first_known_generation":self.generation,"first_known_cut":self.cut,"published_generation":self.generation})
    }

    pub(super) fn add(
        &mut self,
        kind: &str,
        slot: Value,
        payload: Value,
        sources: Vec<usize>,
    ) -> String {
        let object = self.build_object(kind, slot, payload, sources);
        let id = object["object_id"]
            .as_str()
            .expect("构造即带 object_id")
            .to_string();
        self.objects.push(object);
        id
    }
}

fn request(key: Value, id: Value, reasons: Vec<&str>, mut sources: Vec<usize>) -> Value {
    sources.sort_unstable();
    sources.dedup();
    let insufficient = reasons.iter().any(|r| {
        matches!(
            *r,
            "fewer_than_three_merged_bars" | "no_subsequent_bar" | "direction_not_yet_established"
        )
    });
    json!({"request_id":canonical_json(&key),"subject_id":id,
        "axes":{"input_quality":if insufficient{"INSUFFICIENT"}else{"SUFFICIENT"},
            "applicability":if reasons.is_empty(){"IN_DOMAIN"}else{"DOMAIN_PROOF_MISSING"},
            "computation":if reasons.is_empty(){"COMPLETE"}else{"INCOMPLETE"},"validity":"CURRENT"},
        "reasons":reasons,"source_coords":coords(&sources)})
}

// ────────────────────────────────────────────────────────────────────────────
// #1394 2A-S：CC-004 步骤 / CC-005 组的**内容构造**只有一份（两条调用路径共用），
// 路径差别只在「什么时候算」：全量 oracle 每代全算，增量路径只算新步/变版组、其余复用原件。
// ────────────────────────────────────────────────────────────────────────────

fn step_slot(step: &InclusionStepFact) -> Value {
    json!([
        "CC-004.inclusion_step",
        step.incoming.source_index.to_string()
    ])
}

fn step_payload(step: &InclusionStepFact) -> Value {
    let (cmp, endpoints) = comparisons(step.acc_before.as_ref(), &step.incoming);
    json!({"action":step.action,"incoming":bar(&step.incoming),"acc_before":step.acc_before.as_ref().map(bar),
        "acc_after":step.acc_after.as_ref().map(bar),"direction":step.direction_evidence.as_ref().map(|e|dir_str(e.direction)),
        "direction_evidence":direction(&step.direction_evidence),"comparisons":cmp,"endpoint_order":endpoints,"contains":step.contains,
        "high_sources":coords(&step.high_sources),"low_sources":coords(&step.low_sources),"waiting_reasons":step.waiting_reasons})
}

fn step_extra_reasons(step: &InclusionStepFact) -> Vec<&'static str> {
    let mut step_reasons = step.waiting_reasons.clone();
    if step.direction_evidence.is_none() && step_reasons.is_empty() {
        step_reasons.push("direction_not_yet_established");
    }
    step_reasons
}

/// 组对象/请求的来源集合：成员 ∪ 方向证据 ∪ 封口证据（与全量入口同一条）。
fn group_sources(group: &InclusionGroupFact) -> Vec<usize> {
    let mut sources = group.members.clone();
    if let Some(e) = &group.direction_evidence {
        sources.extend(&e.source_coords);
    }
    if let Some(e) = &group.confirmation_evidence {
        sources.extend(&e.source_coords);
    }
    sources
}

fn group_reasons(group: &InclusionGroupFact) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if group.direction_evidence.is_none() {
        reasons.push("direction_not_yet_established");
    }
    if group.high_sources.len() > 1 || group.low_sources.len() > 1 {
        reasons.push("equal_extreme_identity");
    }
    reasons
}

fn group_payload(index: usize, group: &InclusionGroupFact, reasons: &[&'static str]) -> Value {
    json!({"group_index":index.to_string(),"group_anchor":group.bar.source_index.to_string(),"members":coords(&group.members),
        "open":group.bar.open.to_string(),"high":group.bar.high.to_string(),"low":group.bar.low.to_string(),"close":group.bar.close.to_string(),
        "high_sources":coords(&group.high_sources),"low_sources":coords(&group.low_sources),"confirmed":group.confirmed,"confirmation_evidence":direction(&group.confirmation_evidence),
        "direction":group.direction_evidence.as_ref().map(|e|dir_str(e.direction)),"direction_evidence":direction(&group.direction_evidence),"waiting_reasons":reasons})
}

/// CC-007 组装的**唯一**实现（全量 oracle 与跨代增量共用，含 2C 比较单元缓存）。
///
/// 输入是三组同次事实（left/mid/right）+ CC-006 形状原件的来源集合；输出 CC-007 对象与
/// 三条请求（`shape_description` / `descriptive_labels` / `subsequent_development`）。
#[allow(clippy::too_many_arguments)]
fn cc007_add(
    b: &mut Builder,
    raw: &BTreeMap<i64, Value>,
    shape: &Value,
    window: [usize; 3],
    groups: [&InclusionGroupFact; 3],
    base_sources: Vec<usize>,
    units: &mut UnitsCache,
) -> (Vec<Value>, Vec<&'static str>) {
    let branch = shape["branch"].as_str().unwrap();
    let mut references = Vec::new();
    for (label, group) in ["left", "mid", "right"].iter().zip(groups.iter()) {
        let anchor = group.bar.source_index;
        for (axis, value) in [("high", group.bar.high), ("low", group.bar.low)] {
            references.push(json!({"name":format!("{label}.{axis}"),"source_coord":anchor.to_string(),"axis":axis,"value":value.to_string()}));
        }
    }
    let shape_sources = base_sources.clone();
    let end = *groups[2].members.last().unwrap();
    let refs_key = canonical_json(&json!(references));
    let mut sources = base_sources;
    let mut observed: Vec<Value> = Vec::new();
    for (&coord, event) in raw.range((
        std::ops::Bound::Excluded(end as i64),
        std::ops::Bound::Unbounded,
    )) {
        let bar = raw_bar(event);
        let content_key = unit_content_key(coord, event, &bar);
        let unit = units.unit(&refs_key, coord, content_key.clone(), || {
            let mut relations = Vec::new();
            for axis in ["open", "high", "low", "close"] {
                let price = bar[axis].as_str().unwrap().parse::<i64>().unwrap();
                for r in &references {
                    let target = r["value"].as_str().unwrap().parse::<i64>().unwrap();
                    relations.push(json!({"axis":axis,"value":price.to_string(),"reference":r["name"],"reference_value":r["value"],"order":order(price,target)}));
                }
            }
            UnitEntry {
                bar: bar.clone(),
                relations: json!(relations),
                key: content_key,
            }
        });
        sources.push(coord as usize);
        observed.push(json!({"bar": unit.bar.clone(), "relations": unit.relations.clone()}));
    }
    let original: Vec<Value> = groups
        .iter()
        .flat_map(|group| group.members.iter())
        .map(|coord| raw_bar(&raw[&(*coord as i64)]))
        .collect();
    let label_sources: Vec<usize> = original
        .iter()
        .map(|bar| bar["source_coord"].as_str().unwrap().parse().unwrap())
        .collect();
    let no_future = observed.is_empty();
    let window: Vec<usize> = window.to_vec();
    let payload = json!({"shape_object_id":shape["object_id"],"branch":branch,"window":coords(&window),
        "description":if branch=="TOP"{"中组高点和低点均严格高于相邻两组"}else{"中组高点和低点均严格低于相邻两组"},
        "reference_prices":references,"descriptive_labels":{"status":"not_determined","reason":"no_settled_numeric_rule","labels":[],"source":"fenxing.md:64-74","raw_ohlc":original},
        "subsequent_development":{"status":if no_future{"insufficient_knowledge"}else{"observed"},"reason":if no_future{json!("no_subsequent_bar")}else{Value::Null},"bars":observed}});
    let slot = json!(["CC-007.fractal_description", coords(&window)]);
    let id = b.add(
        "CC-007.fractal_description",
        slot.clone(),
        payload,
        sources.clone(),
    );
    let reasons = if no_future {
        vec!["no_subsequent_bar"]
    } else {
        vec![]
    };
    let requests = vec![
        request(
            json!([slot.clone(), "shape_description"]),
            json!(id),
            vec![],
            shape_sources,
        ),
        request(
            json!([slot.clone(), "descriptive_labels"]),
            json!(id),
            vec!["no_settled_numeric_rule"],
            label_sources,
        ),
        request(
            json!([slot, "subsequent_development"]),
            json!(id),
            reasons.clone(),
            sources,
        ),
    ];
    (requests, reasons)
}

pub(super) fn build(
    trace: &InclusionFacts,
    raw: &BTreeMap<i64, Value>,
    shapes: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
) -> (Vec<Value>, Vec<Value>) {
    let mut units = UnitsCache::new(1);
    build_with_units(trace, raw, shapes, gen, cut, frontier, &mut units)
}

/// #1394 2C：携带跨代比较单元缓存的入口（与 [`build`] 输出逐字段相同）。
///
/// 调用方持有同一份 [`UnitsCache`] 跨代调用：分型未变时只有新后续 bar 需要求 24 单元，
/// 已算单元按精确内容键复用。
pub(super) fn build_with_units(
    trace: &InclusionFacts,
    raw: &BTreeMap<i64, Value>,
    shapes: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
    units: &mut UnitsCache,
) -> (Vec<Value>, Vec<Value>) {
    let mut b = Builder {
        raw,
        generation: gen,
        cut,
        objects: vec![],
    };
    let mut requests = Vec::new();
    let mut all_reasons: Vec<&str> = Vec::new();
    for step in &trace.steps {
        let payload = step_payload(step);
        let slot = step_slot(step);
        let id = b.add(
            "CC-004.inclusion_step",
            slot.clone(),
            payload,
            step.source_coords.clone(),
        );
        requests.push(request(
            slot,
            json!(id),
            step_extra_reasons(step),
            step.source_coords.clone(),
        ));
        all_reasons.extend(&step.waiting_reasons);
    }
    for (index, group) in trace.groups.iter().enumerate() {
        let reasons = group_reasons(group);
        let sources = group_sources(group);
        let payload = group_payload(index, group, &reasons);
        let slot = json!(["CC-005.inclusion_group", group.bar.source_index.to_string()]);
        let id = b.add(
            "CC-005.inclusion_group",
            slot.clone(),
            payload,
            sources.clone(),
        );
        requests.push(request(slot, json!(id), reasons.clone(), sources));
        all_reasons.extend(reasons);
    }
    for window in trace.groups.windows(3) {
        if window
            .iter()
            .any(|g| g.high_sources.len() != 1 || g.low_sources.len() != 1)
        {
            let anchors: Vec<usize> = window.iter().map(|g| g.bar.source_index).collect();
            let mut sources = Vec::new();
            for group in window {
                sources.extend(&group.members);
                if let Some(e) = &group.direction_evidence {
                    sources.extend(&e.source_coords);
                }
                if let Some(e) = &group.confirmation_evidence {
                    sources.extend(&e.source_coords);
                }
            }
            requests.push(request(
                json!(["CC-006.local_shape", coords(&anchors)]),
                Value::Null,
                vec!["equal_extreme_identity"],
                sources,
            ));
        }
    }
    for shape in shapes {
        let window: Vec<usize> = ["window_start", "window_mid", "window_end"]
            .iter()
            .map(|k| shape[*k].as_i64().unwrap() as usize)
            .collect();
        let sources_value: Value =
            serde_json::from_str(shape["source_coords_json"].as_str().unwrap()).unwrap();
        let sources: Vec<usize> = sources_value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect();
        requests.push(request(
            json!(["CC-006.local_shape", coords(&window)]),
            shape["object_id"].clone(),
            vec![],
            sources.clone(),
        ));
        let branch = shape["branch"].as_str().unwrap();
        if !matches!(branch, "TOP" | "BOTTOM") {
            continue;
        }
        let groups: Vec<&InclusionGroupFact> = window
            .iter()
            .map(|anchor| {
                trace
                    .groups
                    .iter()
                    .find(|g| g.bar.source_index == *anchor)
                    .unwrap()
            })
            .collect();
        let (cc007_requests, reasons) = cc007_add(
            &mut b,
            raw,
            shape,
            [window[0], window[1], window[2]],
            [groups[0], groups[1], groups[2]],
            sources.clone(),
            units,
        );
        requests.extend(cc007_requests);
        all_reasons.extend(reasons);
        all_reasons.push("no_settled_numeric_rule");
    }
    if trace.merged.len() < 3 {
        all_reasons.push("fewer_than_three_merged_bars");
        requests.push(request(
            json!(["CC-006.local_shape", "pending"]),
            Value::Null,
            vec!["fewer_than_three_merged_bars"],
            raw.keys().map(|x| *x as usize).collect(),
        ));
    }
    if !trace.steps.iter().any(|s| s.direction_evidence.is_some()) {
        all_reasons.push("direction_not_yet_established");
    }
    if trace.initial_direction_unsettled {
        all_reasons.push("initial_direction_unsettled");
    }
    all_reasons.sort_unstable();
    all_reasons.dedup();
    let sources: Vec<usize> = raw.keys().map(|x| *x as usize).collect();
    let observations=all_reasons.iter().map(|reason|json!({"kind":"awaiting_scope","reason":reason,"window_start":null,"window_mid":null,"window_end":null,
        "detail":{"tb02a":true,"source_coords":coords(&sources),"input_frontier":frontier.to_string(),"waiting_reasons":all_reasons}})).collect();
    b.add("CC-054.knowledge_state",json!(["CC-054.knowledge_state","TB-02-A"]),json!({"scope":"TB-02-A","input_frontier":frontier.to_string(),
        "known_facts":["raw_ohlc","inclusion_trace","group_provenance"],"waiting_reasons":all_reasons,
        "domain":"established_direction_without_extreme_identity_competition","requests":requests}),sources);
    b.objects.sort_by(|a, b| {
        (
            a["kind"].as_str(),
            a["fact_key"].as_str(),
            a["object_id"].as_str(),
        )
            .cmp(&(
                b["kind"].as_str(),
                b["fact_key"].as_str(),
                b["object_id"].as_str(),
            ))
    });
    (b.objects, observations)
}

// ────────────────────────────────────────────────────────────────────────────
// #1394 2A-S：跨代稳定对象复用（正式 S 的增量路径）
//
// 与 [`build_with_units`] **同一批内容构造函数**（step_payload/group_payload/cc007_add），
// 差别只在「什么时候算」：新步/变版组建对象，其余按（fact_key, 内容版本）复用**完整原件**
// 与已算 ID；元数据投影到本代（与全量路径逐字段相同）。
//
// 缓存是性能结构：未命中即重算，容量/命中状态不改变任何输出；真实修订按最早受影响坐标
// 失效（[`FactCache::invalidate_from`]），旧 cut 原件不动。
// ────────────────────────────────────────────────────────────────────────────

/// 一条被缓存的事实原件：完整对象 + 请求 + 内容版本。
struct CachedRecord {
    /// 该事实**依赖的最大源坐标**（修订失效扫描用）：任何 `from <= dep_max` 的真实修订
    /// 都可能改变这条事实的内容，故 `dep_max >= from` 一律失效重算。
    ///
    /// - 步骤：其内容只依赖 ≤ 自身源坐标的输入 ⟹ `dep_max = 自身源坐标`；
    /// - 组：内容依赖全部成员坐标与封口证据 ⟹ 未封口组 `dep_max = 末成员坐标`，
    ///   已封口组 `dep_max = 下一组锚`（封口步的 incoming，≥ 全部成员与证据坐标）。
    dep_max: i64,
    object: Value,
    request: Value,
}

/// #1394 2A-S：跨代事实缓存（步骤/组完整原件 + 请求 + 2C 比较单元）。
pub(super) struct FactCache {
    records: HashMap<String, CachedRecord>,
    pub(super) units: UnitsCache,
    hits: usize,
    misses: usize,
}

impl Default for FactCache {
    fn default() -> Self {
        Self::new()
    }
}

impl FactCache {
    pub(super) fn new() -> Self {
        FactCache {
            records: HashMap::new(),
            units: UnitsCache::default(),
            hits: 0,
            misses: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn hits(&self) -> usize {
        self.hits
    }

    #[cfg(test)]
    pub(super) fn misses(&self) -> usize {
        self.misses
    }

    /// 真实修订：从最早受影响源坐标起失效（`coord >= from` 的事实与其请求、比较单元）。
    pub(super) fn invalidate_from(&mut self, from: i64) {
        self.records.retain(|_, record| record.dep_max < from);
        self.units.invalidate_from(from);
    }

    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        self.records.clear();
        self.units.clear();
    }
}

/// 本代元数据投影：与全量路径每代重建的对象逐字段相同（`first_known/published = 本代`）。
fn project_object(cached: &Value, gen: i64, cut: &str) -> Value {
    let mut object = cached.clone();
    object["first_known_generation"] = json!(gen);
    object["first_known_cut"] = json!(cut);
    object["published_generation"] = json!(gen);
    object
}

/// #1394 2A-S：从**同代增量状态**构造 CC-004/CC-005/CC-007/CC-054（与 [`build_with_units`]
/// 输出逐字段相同）。
///
/// 只物化「本代新步」与「变版组」的事实；其余步骤/组直接复用缓存完整原件（不重新展开
/// 来源集合、不重算规范字节、不重算 ID）。缓存未命中一律回退到同一内容构造函数。
/// `group_facts` 复用同次 parser 已生成的完整组投影，避免每个分型窗口重新做全域稠密标注。
pub(super) fn build_incremental(
    state: &newchan_rust::theta_v0::parser::inclusion::IncrFactsState,
    group_facts: &[InclusionGroupFact],
    raw: &BTreeMap<i64, Value>,
    shapes: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
    cache: &mut FactCache,
) -> (Vec<Value>, Vec<Value>) {
    let mut b = Builder {
        raw,
        generation: gen,
        cut,
        objects: vec![],
    };
    let mut requests: Vec<Value> = Vec::new();
    let mut all_reasons: Vec<&str> = Vec::new();
    let groups = state.groups_slice();
    let mut anchor_index: BTreeMap<usize, usize> = BTreeMap::new();
    for (index, group) in groups.iter().enumerate() {
        anchor_index.insert(group.bar.source_index, index);
    }
    // 1) CC-004 步骤：不可变事实——旧步直接复用完整原件。
    for index in 0..state.step_count() {
        let coord = state.step_coord(index).expect("步骤索引在域内");
        all_reasons.extend(state.step_waiting_reasons(index).unwrap_or(&[]));
        let slot = json!(["CC-004.inclusion_step", coord.to_string()]);
        let key = canonical_json(&slot);
        if let Some(record) = cache.records.get(&key) {
            cache.hits += 1;
            b.objects.push(project_object(&record.object, gen, cut));
            requests.push(record.request.clone());
            continue;
        }
        cache.misses += 1;
        let fact = state.step_fact(index).expect("步骤索引在域内");
        let object = b.build_object(
            "CC-004.inclusion_step",
            slot.clone(),
            step_payload(&fact),
            fact.source_coords.clone(),
        );
        let request = request(
            slot,
            object["object_id"].clone(),
            step_extra_reasons(&fact),
            fact.source_coords.clone(),
        );
        cache.records.insert(
            key,
            CachedRecord {
                dep_max: coord as i64,
                object: object.clone(),
                request: request.clone(),
            },
        );
        b.objects.push(object);
        requests.push(request);
    }
    // 2) CC-005 组：内容版本驱动的稳定对象复用。
    for (index, compact) in groups.iter().enumerate() {
        let version = state.group_version(index).unwrap_or(0);
        let anchor = compact.bar.source_index;
        let mut reasons: Vec<&'static str> = Vec::new();
        if compact.direction_evidence.is_none() {
            reasons.push("direction_not_yet_established");
        }
        if compact.high_sources.len() > 1 || compact.low_sources.len() > 1 {
            reasons.push("equal_extreme_identity");
        }
        all_reasons.extend(reasons.iter().copied());
        let slot = json!(["CC-005.inclusion_group", anchor.to_string()]);
        let key = canonical_json(&json!([
            "CC-005.inclusion_group",
            anchor.to_string(),
            version
        ]));
        if let Some(record) = cache.records.get(&key) {
            cache.hits += 1;
            b.objects.push(project_object(&record.object, gen, cut));
            requests.push(record.request.clone());
            continue;
        }
        cache.misses += 1;
        let fact = &group_facts[index];
        let sources = group_sources(fact);
        let object = b.build_object(
            "CC-005.inclusion_group",
            slot.clone(),
            group_payload(index, fact, &reasons),
            sources.clone(),
        );
        let request = request(slot, object["object_id"].clone(), reasons, sources);
        // 组内容依赖全部成员与封口证据：已封口 = 下一组锚，未封口 = 末成员坐标。
        let dep_max = match (groups.get(index + 1), compact.members.last()) {
            (Some(next), _) => next.bar.source_index as i64,
            (None, Some(last)) => *last as i64,
            (None, None) => anchor as i64,
        };
        cache.records.insert(
            key,
            CachedRecord {
                dep_max,
                object: object.clone(),
                request: request.clone(),
            },
        );
        b.objects.push(object);
        requests.push(request);
    }
    // 3) 同角色极值竞争的三组窗口请求（与全量同序、同内容）。
    for window in groups.windows(3) {
        if window
            .iter()
            .any(|g| g.high_sources.len() != 1 || g.low_sources.len() != 1)
        {
            let anchors: Vec<usize> = window.iter().map(|g| g.bar.source_index).collect();
            let mut sources = Vec::new();
            for group in window {
                let fact = &group_facts[anchor_index[&group.bar.source_index]];
                sources.extend(&fact.members);
                if let Some(e) = &fact.direction_evidence {
                    sources.extend(&e.source_coords);
                }
                if let Some(e) = &fact.confirmation_evidence {
                    sources.extend(&e.source_coords);
                }
            }
            requests.push(request(
                json!(["CC-006.local_shape", coords(&anchors)]),
                Value::Null,
                vec!["equal_extreme_identity"],
                sources,
            ));
        }
    }
    // 4) CC-006 形状 → CC-007（继续交给同一 `cc007_add`，2C 单元缓存跨代复用）。
    for shape in shapes {
        let window: Vec<usize> = ["window_start", "window_mid", "window_end"]
            .iter()
            .map(|k| shape[*k].as_i64().unwrap() as usize)
            .collect();
        let sources_value: Value =
            serde_json::from_str(shape["source_coords_json"].as_str().unwrap()).unwrap();
        let sources: Vec<usize> = sources_value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect();
        requests.push(request(
            json!(["CC-006.local_shape", coords(&window)]),
            shape["object_id"].clone(),
            vec![],
            sources.clone(),
        ));
        let branch = shape["branch"].as_str().unwrap();
        if !matches!(branch, "TOP" | "BOTTOM") {
            continue;
        }
        let window_groups: Vec<&InclusionGroupFact> = window
            .iter()
            .map(|anchor| {
                let index = anchor_index
                    .get(anchor)
                    .copied()
                    .unwrap_or_else(|| panic!("形状窗口锚 {anchor} 不在当前组合集内"));
                &group_facts[index]
            })
            .collect();
        let (cc007_requests, reasons) = cc007_add(
            &mut b,
            raw,
            shape,
            [window[0], window[1], window[2]],
            [window_groups[0], window_groups[1], window_groups[2]],
            sources.clone(),
            &mut cache.units,
        );
        requests.extend(cc007_requests);
        all_reasons.extend(reasons);
        all_reasons.push("no_settled_numeric_rule");
    }
    // 5) 知识状态与观察（与全量同源：同一 all_reasons 词汇与同一排序去重）。
    if state.merged_slice().len() < 3 {
        all_reasons.push("fewer_than_three_merged_bars");
        requests.push(request(
            json!(["CC-006.local_shape", "pending"]),
            Value::Null,
            vec!["fewer_than_three_merged_bars"],
            raw.keys().map(|x| *x as usize).collect(),
        ));
    }
    if !state.has_direction_evidence() {
        all_reasons.push("direction_not_yet_established");
    }
    if state.unsettled() {
        all_reasons.push("initial_direction_unsettled");
    }
    all_reasons.sort_unstable();
    all_reasons.dedup();
    let sources: Vec<usize> = raw.keys().map(|x| *x as usize).collect();
    let observations = all_reasons
        .iter()
        .map(|reason| {
            json!({"kind":"awaiting_scope","reason":reason,"window_start":null,"window_mid":null,"window_end":null,
        "detail":{"tb02a":true,"source_coords":coords(&sources),"input_frontier":frontier.to_string(),"waiting_reasons":all_reasons}})
        })
        .collect();
    let knowledge = b.build_object(
        "CC-054.knowledge_state",
        json!(["CC-054.knowledge_state", "TB-02-A"]),
        json!({"scope":"TB-02-A","input_frontier":frontier.to_string(),
        "known_facts":["raw_ohlc","inclusion_trace","group_provenance"],"waiting_reasons":all_reasons,
        "domain":"established_direction_without_extreme_identity_competition","requests":requests}),
        sources,
    );
    b.objects.push(knowledge);
    b.objects.sort_by(|a, b| {
        (
            a["kind"].as_str(),
            a["fact_key"].as_str(),
            a["object_id"].as_str(),
        )
            .cmp(&(
                b["kind"].as_str(),
                b["fact_key"].as_str(),
                b["object_id"].as_str(),
            ))
    });
    (b.objects, observations)
}

fn raw_bar(raw: &Value) -> Value {
    json!({"source_coord":raw["source_coord"],"open":raw["open"],"high":raw["high"],"low":raw["low"],"close":raw["close"]})
}

/// #1394 整合 W-3：目录轴表随规则修订走——每个写者按自己发布的对象集声明目录。
///
/// v3（[`crate::V3_RULE_REVISION`]）只发布 TB-02-A/B，取 13 轴（与 FIN 线逐字同口径）；
/// v2 写者（[`crate::RULE_REVISION`]）另发布 TB-02-C/D，取 17 轴（main #1392/#1404 口径）。
/// 两张表各对应一个规则修订、由调用方按自己的修订选定，不存在「一张判不出再换另一张」
/// 的兜底（#799）。退场条件同 `V3_RULE_REVISION`：v3 承载 TB-02-C/D 后只剩一张表。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CatalogAxisTable {
    /// `s2-axis-quantifiers+new-bi/1`：CC-001…010、CC-054、CC-055、CC-056（13 轴）。
    NewBi,
    /// `s2-axis-quantifiers+new-bi/1+segment-second/2`：另加 CC-011…014（17 轴），含 #1420 续扫。
    SegmentSecond,
}

impl CatalogAxisTable {
    pub(super) fn for_rule_revision(rule_revision: &str) -> Option<Self> {
        match rule_revision {
            crate::V3_RULE_REVISION => Some(Self::NewBi),
            crate::RULE_REVISION => Some(Self::SegmentSecond),
            _ => None,
        }
    }

    /// 写入顺序即目录轴顺序。
    pub(super) fn axes(self) -> &'static [&'static str] {
        match self {
            Self::NewBi => &[
                "CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-008",
                "CC-009", "CC-010", "CC-054", "CC-055", "CC-056",
            ],
            Self::SegmentSecond => &[
                "CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-008",
                "CC-009", "CC-010", "CC-011", "CC-012", "CC-013", "CC-014", "CC-054", "CC-055",
                "CC-056",
            ],
        }
    }
}

pub(super) fn catalog_axes<'a>(
    table: CatalogAxisTable,
    objects: &[Value],
    raw: impl Into<crate::v3::business_json::RawEventsView<'a>>,
    previous_frontier: i64,
    withdrawals: &[Value],
    replaces: &[Value],
) -> Value {
    let raw = raw.into();
    let mut axes = serde_json::Map::new();
    let knowledge = objects
        .iter()
        .find(|o| o["kind"] == "CC-054.knowledge_state")
        .map(|o| serde_json::from_str::<Value>(o["payload_json"].as_str().unwrap()).unwrap());
    for &axis in table.axes() {
        let kind = match axis {
            "CC-001" | "CC-002" | "CC-003" | "CC-004" => "CC-004.inclusion_step",
            "CC-005" => "CC-005.inclusion_group",
            "CC-006" => "CC-006.local_shape",
            "CC-007" => "CC-007.fractal_description",
            "CC-054" => "CC-054.knowledge_state",
            "CC-008" => "CC-008.",
            "CC-009" => "CC-009.same_kind",
            "CC-010" => "CC-010.bi",
            "CC-011" => "CC-011.",
            "CC-012" => "CC-012.feature_sequence",
            "CC-013" | "CC-014" => "CC-013.segment",
            "CC-055" => "CC-055.",
            // 13 轴表只有 `CC-056.version`（FIN 口径）；17 轴表另收 `CC-056.segment_version`。
            "CC-056" if table == CatalogAxisTable::NewBi => "CC-056.version",
            "CC-056" => "CC-056.",
            _ => "",
        };
        let ids: Vec<Value> = objects
            .iter()
            .filter(|o| {
                o["kind"].as_str().is_some_and(|k| {
                    if kind.ends_with('.') {
                        k.starts_with(kind)
                    } else {
                        k == kind
                    }
                })
            })
            .filter(|o| {
                if matches!(axis, "CC-001" | "CC-002" | "CC-003") {
                    let p: Value =
                        serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
                    !p["comparisons"].as_array().unwrap().is_empty()
                } else {
                    true
                }
            })
            .map(|o| o["object_id"].clone())
            .collect();
        let mut waiting: Vec<Value> = Vec::new();
        if let Some(k) = &knowledge {
            if axis == "CC-054" {
                waiting = k["waiting_reasons"].as_array().unwrap().clone();
            } else if !matches!(axis, "CC-001" | "CC-002" | "CC-003" | "CC-056") {
                for request in k["requests"].as_array().unwrap() {
                    let key: Value =
                        serde_json::from_str(request["request_id"].as_str().unwrap()).unwrap();
                    if key[0] == kind || key[0][0] == kind {
                        waiting.extend(request["reasons"].as_array().unwrap().clone());
                    }
                }
            }
        }
        if matches!(axis, "CC-008" | "CC-009" | "CC-010" | "CC-055" | "CC-056") {
            for o in objects.iter().filter(|o| o["kind"] == "CC-056.version") {
                let p: Value = serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
                waiting.extend(p["data"]["waiting_reasons"].as_array().unwrap().clone());
            }
        }
        if matches!(axis, "CC-011" | "CC-012" | "CC-013" | "CC-014") {
            for o in objects
                .iter()
                .filter(|o| o["kind"] == "CC-056.segment_version")
            {
                let p: Value = serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
                waiting.extend(p["data"]["waiting_reasons"].as_array().unwrap().clone());
            }
        }
        waiting.sort_by_key(canonical_json);
        waiting.dedup();
        let mut evidence = json!({"scope":"TB-02-A","object_ids":ids,"waiting_reasons":waiting});
        if matches!(axis, "CC-011" | "CC-012" | "CC-013" | "CC-014") {
            evidence["scope"] = json!("TB-02-W");
            evidence["delivered_subobligations"] = json!([
                "finite_complete_prefix_strict_second",
                "independent_stroke_and_segment_break_facts",
                "general_scan_continuation"
            ]);
            evidence["deferred_subobligations"] =
                json!(["general_equal_identity", "global_G006_bridge"]);
        }
        let mut run = if ids.is_empty() {
            if waiting.is_empty() {
                "not_run"
            } else {
                "waiting"
            }
        } else {
            "run"
        };
        if axis == "CC-056" {
            let revisions = raw
                .iter()
                .filter(|r| {
                    crate::v3::business_json::field(*r, "seq").as_i64().unwrap() > previous_frontier
                        && crate::v3::business_json::field(*r, "revision")
                            .as_i64()
                            .unwrap()
                            > 1
                })
                .map(|event| {
                    crate::v3::business_json::RawReference::new(event, false)
                        .expect("原始账已核整数revision/seq")
                })
                .collect::<Vec<_>>();
            run = if revisions.is_empty() && withdrawals.is_empty() && replaces.is_empty() {
                "not_run"
            } else {
                "run"
            };
            evidence["raw_revisions"] = json!(revisions);
            evidence["withdrawals"] = json!(withdrawals
                .iter()
                .map(project_withdrawal_wire)
                .collect::<Result<Vec<_>, _>>()
                .unwrap());
            evidence["replaces"] = json!(replaces);
        }
        axes.insert(axis.into(),json!({"impl_status":if matches!(axis,"CC-012"|"CC-013"|"CC-014"){"not_implemented"}else{"implemented"},"proof_status":"not_proved","run_status":run,"evidence":evidence}));
    }
    Value::Object(axes)
}

#[cfg(test)]
mod units_cache_tests {
    use super::*;

    #[test]
    fn tb02w_catalog_scope_is_bound_to_exact_rule_without_completing_parent_axes() {
        assert_eq!(
            CatalogAxisTable::for_rule_revision(crate::RULE_REVISION),
            Some(CatalogAxisTable::SegmentSecond)
        );
        assert_eq!(
            CatalogAxisTable::for_rule_revision(crate::V3_RULE_REVISION),
            Some(CatalogAxisTable::NewBi)
        );
        assert_eq!(CatalogAxisTable::for_rule_revision("unknown-rule"), None);
        let raw = Vec::<Value>::new();
        let axes = catalog_axes(CatalogAxisTable::SegmentSecond, &[], &raw, -1, &[], &[]);
        assert_eq!(axes.as_object().unwrap().len(), 17);
        for axis in ["CC-011", "CC-012", "CC-013", "CC-014"] {
            let evidence = &axes[axis]["evidence"];
            assert_eq!(evidence["scope"], "TB-02-W");
            assert!(evidence["delivered_subobligations"]
                .as_array()
                .unwrap()
                .contains(&json!("general_scan_continuation")));
            assert_eq!(
                evidence["deferred_subobligations"],
                json!(["general_equal_identity", "global_G006_bridge"])
            );
            if axis != "CC-011" {
                assert_eq!(axes[axis]["impl_status"], "not_implemented");
            }
        }
        let v3 = catalog_axes(CatalogAxisTable::NewBi, &[], &raw, -1, &[], &[]);
        assert_eq!(v3.as_object().unwrap().len(), 13);
        assert!(v3.get("CC-011").is_none());
    }

    fn entry(key: &str) -> UnitEntry {
        UnitEntry {
            bar: json!({"source_coord":"1","open":"1","high":"2","low":"1","close":"2"}),
            relations: json!([{"axis":"high"}]),
            key: key.to_string(),
        }
    }

    #[test]
    fn units_are_computed_once_per_exact_content_key_and_reused_afterwards() {
        let mut cache = UnitsCache::new(1000);
        let first = cache.unit("refs-a", 7, "k1".into(), || entry("k1"));
        assert_eq!(cache.computed(), 1);
        assert_eq!(cache.reused(), 0);
        // 同内容键：直接复用（同一原件）
        let second = cache.unit("refs-a", 7, "k1".into(), || entry("k1"));
        assert_eq!(cache.computed(), 1);
        assert_eq!(cache.reused(), 1);
        assert_eq!(first.key, second.key);
        // 不同参考集：各自独立求值（每条分型的 24 单元各自算一次）
        cache.unit("refs-b", 7, "k1".into(), || entry("k1"));
        assert_eq!(cache.computed(), 2);
    }

    #[test]
    fn same_coord_with_changed_content_is_recomputed_and_old_version_kept() {
        let mut cache = UnitsCache::new(1000);
        cache.unit("refs-a", 7, "k1".into(), || entry("k1"));
        cache.unit("refs-a", 7, "k2".into(), || entry("k2"));
        assert_eq!(cache.computed(), 2, "同源坐标内容键变化必须重算");
        assert_eq!(cache.invalidated(), 1);
        // 回退到旧版本：按精确内容键命中旧原件（旧 cut 原件不动）
        cache.unit("refs-a", 7, "k1".into(), || entry("k1"));
        assert_eq!(cache.computed(), 2);
        assert_eq!(cache.reused(), 1);
    }

    #[test]
    fn revision_invalidates_affected_coords_and_capacity_eviction_is_output_neutral() {
        let mut cache = UnitsCache::new(1000);
        for coord in 0..5i64 {
            cache.unit("refs-a", coord, format!("k{coord}"), || {
                entry(&format!("k{coord}"))
            });
        }
        assert_eq!(cache.computed(), 5);
        cache.invalidate_from(3);
        assert_eq!(cache.computed(), 5);
        for coord in 0..5i64 {
            cache.unit("refs-a", coord, format!("k{coord}"), || {
                entry(&format!("k{coord}"))
            });
        }
        assert_eq!(cache.computed(), 7, "修订坐标 3、4 必须重算");
        assert_eq!(
            cache.reused(),
            3,
            "修订起点之前的坐标 0-2 未受影响、直接复用"
        );
        // 容量不足：只失去复用（可重算），输出内容不变
        let mut tiny = UnitsCache::new(1);
        let a = tiny.unit("refs-a", 1, "k1".into(), || entry("k1"));
        let _b = tiny.unit("refs-a", 1, "k1".into(), || entry("k1"));
        let _c = tiny.unit("refs-a", 2, "k2".into(), || entry("k2"));
        let d = tiny.unit("refs-a", 1, "k1".into(), || entry("k1"));
        assert_eq!(
            a.relations, d.relations,
            "超限清空后重算必须给出逐字段相同的原件"
        );
        assert!(tiny.evictions() >= 1);
    }
}
