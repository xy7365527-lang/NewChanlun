//! #1419/#1420：同次段事实直接构造 C/D 内容边；E/F 与生成态接线另行验收。
//! 不调用旧对象 JSON builder，不重算段判据；obj3 身份只调用共享工厂。
use crate::content_producer::{
    Cc004Producer, NativeStructureObject as TypedObject, RequestApplicability, RequestAxes,
    RequestComputation, RequestInputQuality, RequestValidity,
};
use newchan_rust::s_content::{self as sc, Built, NodeStore};
use newchan_rust::theta_v0::{
    parser::{
        feature_seq::SecondTerminal,
        second_fold::{SecondCandidateId, SecondFoldStore},
        segment::{ConstructionFact, SegmentFacts, CONSTRUCTION_RULE},
        stroke::StrokeFacts,
    },
    types::{Direction, Segment},
};
use serde::Serialize;
macro_rules! vec_in { ($nodes:expr; $($v:expr),* $(,)?) => { Vec::from_iter([$($v),*], $nodes)? }; }
use super::audit_store::CodecRead;
use super::structure_value::{
    self as sv, CloneIn, Map as BTreeMap, Set as BTreeSet, Text, Value as CV, Vec,
};
#[cfg(test)]
use serde_json::Value;

/// 调用者必须一次 parser 取得这些值；恢复先于 parser，不在投影器里恢复或重算。
pub(super) struct StructureEvidence<'a> {
    pub segments: &'a [Segment],
    pub segment_facts: &'a SegmentFacts,
    pub stroke_facts: &'a StrokeFacts,
    pub second_store: &'a SecondFoldStore,
}
/// 调用者从上一代已核活跃发布索引选择。内容自洽核验不替代发布态授权。
pub(super) struct StructurePrior<'a> {
    pub segment_records: &'a [Built],
    pub version_record: Option<&'a Built>,
}
pub(super) struct StructureContext<'a> {
    pub fact_generation: i64,
    pub generation: i64,
    pub cut: &'a str,
    pub frontier: i64,
    /// 真实 generation/frontier/receipt/received_at/semantic_commit_ns 的共享值根。
    pub known_at: &'a Built,
    pub all_sources: &'a [usize],
}
pub(super) struct SegmentObject {
    pub candidate_index: usize,
    pub entity_id: Text,
    pub object_id: Text,
    pub sources: Vec<usize>,
}
pub(super) struct StructureOutput {
    /// 本次实际分类请求的唯一集合；不要再从 TypedObject.request 重复收集。
    pub requests: Vec<Built>,
    pub objects: Vec<TypedObject>,
    pub relations: Vec<Built>,
    pub segments: Vec<SegmentObject>,
    pub waiting_reasons: Vec<&'static str>,
}

use sv::{object as obj, put, string as s};
fn edge(root: &Built) -> CV {
    CV::Edge(root.digest)
}
fn field(nodes: &mut dyn NodeStore, root: &Built, name: &str) -> Result<Built, String> {
    let root = nodes.native_built_ref(root).map_err(|e| e.to_string())?;
    let value = nodes
        .lookup_field_ref(root.view(), name)
        .map_err(|e| e.to_string())?;
    Ok(Built {
        digest: value.view().digest().map_err(|e| e.to_string())?,
        kind: value.kind(),
        extent: value.extent(),
    })
}
fn text(nodes: &mut dyn NodeStore, root: &Built) -> Result<Text, String> {
    if root.kind != sc::Kind::String {
        return Err("结构字段不是文本".into());
    }
    let root = nodes.native_built_ref(root).map_err(|e| e.to_string())?;
    let owner = super::business_json::render_json(nodes, root.view())?;
    Text::new(owner.view().as_str().ok_or("结构字段不是文本")?, nodes)
}
fn field_text(nodes: &mut dyn NodeStore, root: &Built, name: &str) -> Result<Text, String> {
    let value = field(nodes, root, name)?;
    text(nodes, &value)
}
pub(crate) fn canonical_root_text(nodes: &mut dyn NodeStore, root: &Built) -> Result<Text, String> {
    enum Bytes {
        Native(sc::capture_allocation::scratch::ScratchBytes),
        Plain(std::vec::Vec<u8>),
    }
    impl std::io::Write for Bytes {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            match self {
                Self::Native(v) => v.try_extend_bytes(b).map_err(std::io::Error::other)?,
                Self::Plain(v) => v.extend_from_slice(b),
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut bytes = match nodes.codec_work() {
        Some(w) => Bytes::Native(w.bytes()),
        None => Bytes::Plain(std::vec::Vec::new()),
    };
    let reference = nodes.native_built_ref(root).map_err(|e| e.to_string())?;
    nodes
        .render_ref(reference.view(), &mut bytes)
        .map_err(|e| e.to_string())?;
    let bytes: &[u8] = match &bytes {
        Bytes::Native(v) => v,
        Bytes::Plain(v) => v,
    };
    Text::new(
        std::str::from_utf8(bytes).map_err(|e| e.to_string())?,
        nodes,
    )
}
fn canonical_key(value: CV, nodes: &mut dyn NodeStore) -> Result<Text, String> {
    let root = put(&value, nodes)?;
    canonical_root_text(nodes, &root)
}
fn sorted_sources(mut sources: Vec<usize>) -> Vec<usize> {
    sources.sort_unstable();
    sources.dedup();
    sources
}
fn source_coords(sources: &[usize], nodes: &mut dyn NodeStore) -> Result<Built, String> {
    fact(nodes, sources)
}
fn input_refs(
    producer: &Cc004Producer,
    sources: &[usize],
    nodes: &mut dyn NodeStore,
) -> Result<Built, String> {
    let mut values = Vec::new();
    for &coord in sources {
        let coord = i64::try_from(coord).map_err(|_| "来源坐标溢出")?;
        let digest = producer.source_root(coord).ok_or("结构来源未登记")?;
        values.push(CV::Edge(digest), nodes)?;
    }
    put(&CV::Array(values), nodes)
}

fn complete_axes() -> RequestAxes {
    RequestAxes {
        input_quality: RequestInputQuality::Sufficient,
        applicability: RequestApplicability::InDomain,
        computation: RequestComputation::Complete,
        validity: RequestValidity::Current,
    }
}
fn missing_axes() -> RequestAxes {
    RequestAxes {
        input_quality: RequestInputQuality::Insufficient,
        applicability: RequestApplicability::DomainProofMissing,
        computation: RequestComputation::Incomplete,
        validity: RequestValidity::Current,
    }
}
fn computation_pending_axes() -> RequestAxes {
    RequestAxes {
        computation: RequestComputation::Incomplete,
        ..complete_axes()
    }
}
fn outside_axes() -> RequestAxes {
    RequestAxes {
        applicability: RequestApplicability::OutsideDomain,
        ..complete_axes()
    }
}
fn classification_request(
    producer: &mut Cc004Producer,
    requests: &mut Vec<Built>,
    subject: &TypedObject,
    cc: &str,
    axis: &str,
    path: CV,
    axes: RequestAxes,
    reasons: &[&str],
    sources: &[usize],
    nodes: &mut dyn NodeStore,
) -> Result<(), String> {
    // 主体 obj3 已绑定完整事实和源版本；固定请求版本与观察路径隔离同对象内的多次判定。
    // 与 source_pending 的规范二元槽位协议相同：对象 slot + 具名请求 slot。
    let kind = field_text(nodes, &subject.record, "kind")?;
    let slot = field(nodes, &subject.payload, "slot")?;
    let key = CV::Array(vec_in![nodes;
        CV::Array(vec_in![nodes;s(kind, nodes)?, edge(&slot)]),
        CV::Array(vec_in![nodes;
            s(cc, nodes)?,
            s(axis, nodes)?,
            s("structure-classification/1", nodes)?,
            path,
            s(&subject.object_id, nodes)?,
        ]),
    ]);
    let key = put(&key, nodes)?;
    let coords = source_coords(sources, nodes)?;
    let request =
        producer.explicit_request_root(nodes, &key, &subject.object_id, axes, reasons, &coords)?;
    requests.push(request, nodes)?;
    Ok(())
}
fn construction_requests(
    producer: &mut Cc004Producer,
    requests: &mut Vec<Built>,
    subject: &TypedObject,
    path: CV,
    c: &ConstructionFact,
    sources: &[usize],
    nodes: &mut dyn NodeStore,
) -> Result<(), String> {
    classification_request(
        producer,
        requests,
        subject,
        "CC-011",
        "length",
        path.clone_in(nodes)?,
        complete_axes(),
        &[],
        sources,
        nodes,
    )?;
    let (axes, reasons): (_, &[&str]) = if c.vector.is_some() {
        (complete_axes(), &[])
    } else if c.length != "odd_ge3" {
        (outside_axes(), &["construction_length_outside_domain"])
    } else {
        (missing_axes(), &["construction_conditions_not_evaluated"])
    };
    classification_request(
        producer,
        requests,
        subject,
        "CC-011",
        "construction_conditions",
        path.clone_in(nodes)?,
        axes,
        reasons,
        sources,
        nodes,
    )?;
    let (axes, reasons): (_, &[&str]) = match c.vector {
        Some([true, true, true, true]) if c.direction.is_some() => (complete_axes(), &[]),
        Some([true, true, true, true]) => {
            (missing_axes(), &["construction_direction_not_evaluated"])
        }
        Some(_) => (outside_axes(), &["construction_conditions_failed"]),
        None if c.length != "odd_ge3" => (outside_axes(), &["construction_length_outside_domain"]),
        None => (missing_axes(), &["construction_conditions_not_evaluated"]),
    };
    classification_request(
        producer,
        requests,
        subject,
        "CC-011",
        "direction",
        path,
        axes,
        reasons,
        sources,
        nodes,
    )
}

/// 从已核事实直接序列化为内容值，整数遵从 s-segment/3 的十进制文本规则。
/// 不经 serde_json::Value、JSON 文本或旧 typed object；每份事实根只构造一次并跨对象接边。
pub(crate) fn fact<T: Serialize + ?Sized>(
    nodes: &mut dyn NodeStore,
    value: &T,
) -> Result<Built, String> {
    put(
        &(value
            .serialize(WireSerializer(nodes))
            .map_err(|e| e.to_string())?),
        nodes,
    )
}

struct PriorSegment {
    data: Built,
    revision: i64,
    result: Text,
}
fn verify_prior(
    nodes: &mut dyn NodeStore,
    producer: &Cc004Producer,
    record: &Built,
    kind: &str,
) -> Result<Built, String> {
    let record_ref = nodes.native_built_ref(record).map_err(|e| e.to_string())?;
    nodes
        .verify_ref(record_ref.view(), sc::Mode::Public)
        .map_err(|e| e.to_string())?;
    if field_text(nodes, record, "kind")? != kind {
        return Err("prior kind 不符".into());
    }
    let key = field_text(nodes, record, "fact_key")?;
    let payload = field(nodes, record, "payload")?;
    let refs = field(nodes, record, "input_refs")?;
    if payload.kind != sc::Kind::Object || refs.kind != sc::Kind::Array {
        return Err("prior 内容边类型不符".into());
    }
    let identity = producer
        .typed_identity_scoped(kind, &key, &payload, &refs, nodes.codec_work())
        .map_err(|e| e.to_string())?;
    let derived = Text::format(
        format_args!("{}", crate::content_producer::object_id_display(&identity)),
        nodes,
    )?;
    if field_text(nodes, record, "object_id")? != derived {
        return Err("prior obj3 身份不符".into());
    }
    for (key, want) in [
        ("schema_revision", header(kind).0),
        ("semantic_version", header(kind).1),
        ("policy_version", header(kind).2),
        ("view_role", "main"),
    ] {
        if field_text(nodes, &payload, key)? != want {
            return Err(format!("prior {key} 不符"));
        }
    }
    let slot = field(nodes, &payload, "slot")?;
    if canonical_key(
        CV::Array(vec_in![nodes;s(kind, nodes)?, edge(&slot)]),
        nodes,
    )? != key
    {
        return Err("prior fact_key 与 slot 不符".into());
    }
    let data = field(nodes, &payload, "data")?;
    if kind == crate::tb02c::KINDS[2]
        || crate::tb02e::KINDS[..4].contains(&kind)
        || crate::tb02f::KINDS[..2].contains(&kind)
    {
        if text(nodes, &slot)? != field_text(nodes, &data, "entity_id")? {
            return Err("prior slot 与实体身份不符".into());
        }
    }
    Ok(data)
}
fn read_prior(
    nodes: &mut dyn NodeStore,
    producer: &Cc004Producer,
    prior: &StructurePrior<'_>,
) -> Result<BTreeMap<Text, PriorSegment>, String> {
    if !prior.segment_records.is_empty() && prior.version_record.is_none() {
        return Err("prior 段记录缺本 cut 段版本根".into());
    }
    let mut out = BTreeMap::new();
    for record in prior.segment_records {
        let data = verify_prior(nodes, producer, record, crate::tb02c::KINDS[2])?;
        let entity = field_text(nodes, &data, "entity_id")?;
        let revision_text = field_text(nodes, &data, "entity_revision")?;
        let revision = revision_text.parse::<i64>().map_err(|e| e.to_string())?;
        if revision < 1 || Text::format(format_args!("{revision}"), nodes)? != revision_text {
            return Err("prior 非法修订".into());
        }
        let result = field_text(nodes, &data, "result")?;
        if out
            .insert(
                entity,
                PriorSegment {
                    data,
                    revision,
                    result,
                },
                nodes,
            )?
            .is_some()
        {
            return Err("prior 重复段实体".into());
        }
    }
    if let Some(record) = prior.version_record {
        verify_prior(nodes, producer, record, crate::tb02c::KINDS[5])?;
    }
    Ok(out)
}

/// 恢复交给 parser 前使用；只读真实已授权版本根，游标仍由 parser 自身验证。
/// checkpoint 是 parser 的既有权威 wire，允许还原它；不还原整个业务对象。
pub(super) fn restore_second_cursors(
    nodes: &mut dyn NodeStore,
    producer: &Cc004Producer,
    prior: &StructurePrior<'_>,
    fold: &mut SecondFoldStore,
) -> Result<(), String> {
    let Some(record) = prior.version_record else {
        return Ok(());
    };
    let data = verify_prior(nodes, producer, record, crate::tb02c::KINDS[5])?;
    let cursors = field(nodes, &data, "scan_cursors")?;
    let cursors = nodes
        .native_built_ref(&cursors)
        .map_err(|e| e.to_string())?;
    let items = nodes
        .lookup_array_ref(cursors.view())
        .map_err(|e| e.to_string())?;
    for item in items {
        let wire = super::business_json::render_json(nodes, item.view())?;
        match &wire {
            super::business_json::JsonOwner::Plain(value) => fold.restore(value.as_ref()),
            #[cfg(feature = "s_allocation_json_testonly")]
            super::business_json::JsonOwner::Owned(_) => {
                return Err("K1恢复只接受Native游标线形".into())
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit(
    producer: &mut Cc004Producer,
    ctx: &StructureContext<'_>,
    kind: &str,
    slot: CV,
    data: &Built,
    sources: &[usize],
    nodes: &mut dyn NodeStore,
) -> Result<TypedObject, String> {
    let slot = put(&(slot), nodes)?;
    let key = canonical_key(
        CV::Array(vec_in![nodes;s(kind, nodes)?, edge(&slot)]),
        nodes,
    )?;
    let payload = obj(
        [
            ("schema_revision", s(header(kind).0, nodes)?),
            ("semantic_version", s(header(kind).1, nodes)?),
            ("policy_version", s(header(kind).2, nodes)?),
            ("view_role", s("main", nodes)?),
            ("object_generation", s(ctx.fact_generation, nodes)?),
            ("slot", edge(&slot)),
            ("data", edge(data)),
        ],
        nodes,
    )?;
    let refs = input_refs(producer, sources, nodes)?;
    let coords = source_coords(sources, nodes)?;
    let payload = put(&payload, nodes)?;
    producer
        .build_structure_object(
            nodes,
            kind,
            &key,
            payload,
            refs,
            coords,
            ctx.generation,
            ctx.cut,
        )
        .map_err(|e| e.to_string())
}

fn stroke_refs(
    nodes: &mut dyn NodeStore,
    strokes: &StrokeFacts,
    generation: i64,
    from: usize,
    through: usize,
) -> Result<(Built, Vec<usize>), String> {
    let mut values = Vec::new();
    let mut sources = Vec::new();
    // Seed 为空时旧合同允许空序列；候选的索引由主循环显式核验。
    for (i, bi) in strokes
        .strokes
        .iter()
        .enumerate()
        .take(through.saturating_add(1))
        .skip(from)
    {
        sources.extend(bi.start.source_coords.iter().copied(), nodes)?;
        sources.extend(bi.end.source_coords.iter().copied(), nodes)?;
        let a = fact(nodes, &bi.start.extreme_roots)?;
        let b = fact(nodes, &bi.end.extreme_roots)?;
        let value = put(
            &(obj(
                [
                    ("stroke_index", s(i, nodes)?),
                    (
                        "entity_id",
                        s(
                            Text::format(
                                format_args!("bi:{generation}:{}", bi.identity_anchor),
                                nodes,
                            )?,
                            nodes,
                        )?,
                    ),
                    ("start_anchor", s(bi.start.group_anchor, nodes)?),
                    ("end_anchor", s(bi.end.group_anchor, nodes)?),
                    ("start_price", s(bi.start.price, nodes)?),
                    ("end_price", s(bi.end.price, nodes)?),
                    ("start_roots", edge(&a)),
                    ("end_roots", edge(&b)),
                ],
                nodes,
            )?),
            nodes,
        )?;
        values.push(edge(&value), nodes)?;
    }
    Ok((put(&(CV::Array(values)), nodes)?, sorted_sources(sources)))
}
fn second_status(state: Option<SecondTerminal>) -> &'static str {
    match state {
        Some(SecondTerminal::Confirmed) => "CONFIRMED",
        Some(SecondTerminal::KilledByNewExtreme) => "KILLED",
        _ => "PENDING",
    }
}
fn replace_value(fields: &mut [(Text, CV)], key: &str, value: CV) -> Result<(), String> {
    let (_, slot) = fields
        .iter_mut()
        .find(|(k, _)| k == key)
        .ok_or_else(|| format!("结构字段缺 {key}"))?;
    *slot = value;
    Ok(())
}

/// C/D 直接构造核。E/F 消费返回的真实段对象映射，不能以完成段的位置造身份。
pub(super) fn build_cd(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    evidence: &StructureEvidence<'_>,
    prior: &StructurePrior<'_>,
    ctx: &StructureContext<'_>,
) -> Result<StructureOutput, String> {
    let known_ref = nodes
        .native_built_ref(ctx.known_at)
        .map_err(|e| e.to_string())?;
    nodes
        .verify_ref(known_ref.view(), sc::Mode::Public)
        .map_err(|e| e.to_string())?;
    if ctx.known_at.kind != sc::Kind::Object {
        return Err("known_at 必须为真实对象根".into());
    }
    let old = read_prior(nodes, producer, prior)?;
    let facts = evidence.segment_facts;
    let strokes = evidence.stroke_facts;
    // #1604（#1586 C-04，与 v2 `tb02c::build` 同一口径）：尾窗与预算读段事实所带记录，不读代码常数；
    // 记录缺失即拒写（不回退）；游标簿与段事实须读同一份预算记录，不同即拒写。
    let budget = facts
        .policy
        .as_ref()
        .ok_or("TB-02-C：段事实未携带所读记录（#1604）")?;
    if evidence.second_store.policy.label() != budget.scan_budget {
        return Err(format!(
            "TB-02-C：游标簿预算记录 `{}` 与段事实所读 `{}` 不同（#1604）",
            evidence.second_store.policy.label(),
            budget.scan_budget
        ));
    }
    let truncated_from = facts.tail_window_bound.as_ref().map(|b| b.seg_start);
    let mut out = StructureOutput {
        requests: Vec::new(),
        objects: Vec::new(),
        relations: Vec::new(),
        segments: Vec::new(),
        waiting_reasons: Vec::from_iter(facts.waiting_reasons.iter().copied(), nodes)?,
    };
    let mut seeds = Vec::from_iter(facts.seeds.iter(), nodes)?;
    for c in &facts.candidates {
        if !seeds.iter().any(|s| s.start_stroke == c.seed.start_stroke) {
            seeds.push(&c.seed, nodes)?;
        }
    }
    for seed in seeds {
        let anchor = match strokes.strokes.get(seed.start_stroke) {
            Some(v) => Text::format(format_args!("{}", v.identity_anchor), nodes)?,
            None => Text::new("empty", nodes)?,
        };
        let entity = Text::format(format_args!("seed:{}:{anchor}", ctx.fact_generation), nodes)?;
        let (refs, sources) = stroke_refs(
            nodes,
            strokes,
            ctx.fact_generation,
            seed.start_stroke,
            seed.stroke_indices
                .last()
                .copied()
                .unwrap_or(seed.start_stroke),
        )?;
        let construction = fact(nodes, seed)?;
        let data = put(
            &(obj(
                [
                    ("entity_id", s(&entity, nodes)?),
                    ("construction", edge(&construction)),
                    ("stroke_refs", edge(&refs)),
                ],
                nodes,
            )?),
            nodes,
        )?;
        let kind = crate::tb02c::KINDS[if seed.vector == Some([true; 4]) { 0 } else { 6 }];
        let object = emit(
            producer,
            ctx,
            kind,
            s(entity, nodes)?,
            &data,
            &sources,
            nodes,
        )?;
        construction_requests(
            producer,
            &mut out.requests,
            &object,
            s("construction", nodes)?,
            seed,
            &sources,
            nodes,
        )?;
        out.objects.push(object, nodes)?;
    }
    let mut active = BTreeSet::new();
    for (candidate_index, c) in facts.candidates.iter().enumerate() {
        let bi = strokes.strokes.get(c.start_stroke).ok_or("候选起笔越界")?;
        if c.observed_through >= strokes.strokes.len() {
            return Err("候选 observed_through 越界".into());
        }
        let up = bi.start.kind == "BOTTOM";
        let dir = if up { "UP" } else { "DOWN" };
        let opposite = if up { "DOWN" } else { "UP" };
        let entity = Text::format(
            format_args!("segment:{}:{}", ctx.fact_generation, bi.identity_anchor),
            nodes,
        )?;
        if !active.insert(entity.clone_in(nodes)?, nodes)? {
            return Err("重复本次段实体".into());
        }
        let sequence = Text::format(format_args!("first:{entity}"), nodes)?;
        let (refs, sources) = stroke_refs(
            nodes,
            strokes,
            ctx.fact_generation,
            c.start_stroke,
            c.observed_through,
        )?;
        let first = fact(nodes, &c.first)?;
        let scan = put(
            &(obj(
                [
                    ("from_stroke", s(c.start_stroke, nodes)?),
                    (
                        "through_stroke",
                        s(
                            c.first
                                .raw_elements
                                .last()
                                .map_or(c.start_stroke, |e| e.stroke_idx),
                            nodes,
                        )?,
                    ),
                    (
                        "input_stroke_count",
                        s(
                            c.first
                                .raw_elements
                                .last()
                                .map_or(c.start_stroke + 1, |e| e.stroke_idx + 1),
                            nodes,
                        )?,
                    ),
                    (
                        "resource_truncated",
                        CV::Bool(truncated_from.is_some_and(|f| c.start_stroke >= f)),
                    ),
                    ("tail_cache_window", s(budget.tail_window, nodes)?),
                ],
                nodes,
            )?),
            nodes,
        )?;
        let data = put(
            &(obj(
                [
                    ("entity_id", s(&sequence, nodes)?),
                    ("segment_id", s(&entity, nodes)?),
                    ("role", s("FIRST_SEQUENCE", nodes)?),
                    ("direction", s(dir, nodes)?),
                    ("member_direction", s(opposite, nodes)?),
                    ("facts", edge(&first)),
                    ("stroke_refs", edge(&refs)),
                    ("scan", edge(&scan)),
                    (
                        "second_sequence_status",
                        s(
                            c.second
                                .as_ref()
                                .map_or("NOT_APPLICABLE", |v| second_status(v.semantic_state)),
                            nodes,
                        )?,
                    ),
                ],
                nodes,
            )?),
            nodes,
        )?;
        let first_object = emit(
            producer,
            ctx,
            crate::tb02c::KINDS[1],
            s(&sequence, nodes)?,
            &data,
            &sources,
            nodes,
        )?;
        let first_id = Text::new(&first_object.object_id, nodes)?;
        out.objects.push(first_object, nodes)?;
        let second_sequence = c
            .second
            .as_ref()
            .map(|_| Text::format(format_args!("second:{entity}"), nodes))
            .transpose()?;
        let second_id = if let Some(second) = &c.second {
            let b_stroke = second
                .from_stroke
                .checked_sub(1)
                .ok_or("第二种 from_stroke 为零")?;
            let cursor = evidence
                .second_store
                .cursor(&SecondCandidateId {
                    seg_dir: if up { Direction::Up } else { Direction::Down },
                    b_stroke,
                    candidate_extreme: second.candidate_extreme,
                })
                .ok_or("IdentityConflict：第二种候选不在同次游标簿")?;
            let cursor_root = fact(nodes, &cursor.wire_view())?;
            let second_root = fact(nodes, second)?;
            let semantic = fact(nodes, &second.semantic_state)?;
            let coverage = fact(nodes, &second.coverage)?;
            let granted = fact(nodes, &cursor.budget_granted)?;
            let scan = put(
                &(obj(
                    [
                        ("from_stroke", s(second.from_stroke, nodes)?),
                        ("through_stroke", s(second.through_stroke, nodes)?),
                        ("input_stroke_count", s(second.input_stroke_count, nodes)?),
                        ("resource_truncated", CV::Bool(second.resource_truncated)),
                        ("tail_cache_window", s(budget.second_seq_window, nodes)?),
                        ("semantic_state", edge(&semantic)),
                        ("coverage", edge(&coverage)),
                        (
                            "budget",
                            obj(
                                [
                                    ("policy", s(cursor.budget_policy.label(), nodes)?),
                                    ("granted", edge(&granted)),
                                ],
                                nodes,
                            )?,
                        ),
                        ("cursor", edge(&cursor_root)),
                    ],
                    nodes,
                )?),
                nodes,
            )?;
            let name = second_sequence.as_ref().ok_or("缺第二序列实体")?;
            let data = put(
                &(obj(
                    [
                        ("entity_id", s(name, nodes)?),
                        ("segment_id", s(&entity, nodes)?),
                        ("role", s("SECOND_SEQUENCE", nodes)?),
                        ("direction", s(dir, nodes)?),
                        ("member_direction", s(dir, nodes)?),
                        ("facts", edge(&second_root)),
                        ("stroke_refs", edge(&refs)),
                        ("scan", edge(&scan)),
                        (
                            "second_sequence_status",
                            s(second_status(second.semantic_state), nodes)?,
                        ),
                    ],
                    nodes,
                )?),
                nodes,
            )?;
            let object = emit(
                producer,
                ctx,
                crate::tb02c::KINDS[1],
                s(name, nodes)?,
                &data,
                &sources,
                nodes,
            )?;
            let id = Text::new(&object.object_id, nodes)?;
            out.objects.push(object, nodes)?;
            Some(id)
        } else {
            None
        };
        let previous = old.get(&entity);
        let geometric = if let Some(i) = c.end_stroke {
            let end = &strokes.strokes.get(i).ok_or("候选段端越界")?.end;
            let roots = fact(nodes, &end.extreme_roots)?;
            put(
                &(obj(
                    [
                        ("stroke_index", s(i, nodes)?),
                        ("group_anchor", s(end.group_anchor, nodes)?),
                        ("extreme_roots", edge(&roots)),
                        ("price", s(end.price, nodes)?),
                    ],
                    nodes,
                )?),
                nodes,
            )?
        } else {
            put(&(CV::Null), nodes)?
        };
        let formed = match previous {
            Some(p) => field(nodes, &p.data, "formed_known_at")?,
            None => ctx.known_at.clone(),
        };
        let terminated = if matches!(c.result, "CASE_ONE" | "CASE_TWO") {
            match previous {
                Some(p)
                    if p.result == c.result
                        && field(nodes, &p.data, "geometric_end")?.digest == geometric.digest =>
                {
                    field(nodes, &p.data, "terminated_known_at")?
                }
                _ => ctx.known_at.clone(),
            }
        } else {
            put(&(CV::Null), nodes)?
        };
        let strict = fact(nodes, &c.strict)?;
        let vector = fact(nodes, &c.fact_vector)?;
        let construction = fact(nodes, &c.construction)?;
        let waiting = fact(nodes, &c.waiting_reasons)?;
        let attempts = fact(nodes, &c.termination_attempts)?;
        let trigger = fact(nodes, &c.trigger_stroke)?;
        // #1493 §7.1 第 7 条（与 v2 `tb02c::build` 同一字段集）：第一种情况候选态、窗成员与段端门 fail-closed。
        let turn_state = fact(nodes, &c.turn_hypothesis_state)?;
        let window_members = fact(nodes, &c.window_members)?;
        let parse_invalid_from = fact(nodes, &c.parse_invalid_from)?;
        let parse_invalid = fact(
            nodes,
            &facts
                .parse_invalid_from
                .as_ref()
                .filter(|p| Some(p.seg_start) == c.parse_invalid_from),
        )?;
        let CV::Object(mut fields) = obj(
            [
                ("entity_id", s(&entity, nodes)?),
                ("entity_revision", s(1, nodes)?),
                (
                    "seed_id",
                    s(
                        Text::format(
                            format_args!("seed:{}:{}", ctx.fact_generation, bi.identity_anchor),
                            nodes,
                        )?,
                        nodes,
                    )?,
                ),
                ("sequence_id", s(&sequence, nodes)?),
                ("sequence_object_id", s(&first_id, nodes)?),
                (
                    "second_sequence_id",
                    second_sequence
                        .as_ref()
                        .map(|v| s(v, nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                (
                    "second_sequence_object_id",
                    second_id
                        .as_ref()
                        .map(|v| s(v, nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                ("strict", edge(&strict)),
                ("fact_vector", edge(&vector)),
                ("construction", edge(&construction)),
                ("result", s(c.result, nodes)?),
                ("waiting_reasons", edge(&waiting)),
                ("termination_attempts", edge(&attempts)),
                ("geometric_end", edge(&geometric)),
                ("trigger_stroke", edge(&trigger)),
                ("formed_known_at", edge(&formed)),
                ("terminated_known_at", edge(&terminated)),
                ("turn_hypothesis_state", edge(&turn_state)),
                ("window_members", edge(&window_members)),
                ("parse_invalid_from", edge(&parse_invalid_from)),
                ("parse_invalid", edge(&parse_invalid)),
                ("stroke_refs", edge(&refs)),
            ],
            nodes,
        )?
        else {
            unreachable!()
        };
        let normalized = put(&(CV::Object(fields.clone_in(nodes)?)), nodes)?;
        let changed = if let Some(p) = previous {
            let names = Vec::from_iter(fields.iter().map(|(k, _)| k.as_str()), nodes)?;
            let r = nodes.native_built_ref(&p.data).map_err(|e| e.to_string())?;
            let prior_fields = nodes
                .lookup_fields_ref(r.view(), &names)
                .map_err(|e| e.to_string())?;
            let mut normalized_prior = Vec::new();
            for key in names.iter().copied() {
                let reference = prior_fields.get(key).ok_or("旧对象缺事实字段")?;
                normalized_prior.push(
                    (
                        Text::new(key, nodes)?,
                        if key == "entity_revision" {
                            s(1, nodes)?
                        } else {
                            CV::Edge(reference.digest().map_err(|e| e.to_string())?)
                        },
                    ),
                    nodes,
                )?;
            }
            put(&(CV::Object(normalized_prior)), nodes)?.digest != normalized.digest
        } else {
            true
        };
        let revision = previous
            .map_or(0, |p| p.revision)
            .checked_add(i64::from(changed))
            .ok_or("段修订溢出")?;
        replace_value(&mut fields, "entity_revision", s(revision, nodes)?)?;
        let data = put(&(CV::Object(fields)), nodes)?;
        let object = emit(
            producer,
            ctx,
            crate::tb02c::KINDS[2],
            s(&entity, nodes)?,
            &data,
            &sources,
            nodes,
        )?;
        let object_id = Text::new(&object.object_id, nodes)?;
        construction_requests(
            producer,
            &mut out.requests,
            &object,
            s("construction", nodes)?,
            &c.construction,
            &sources,
            nodes,
        )?;
        for (index, attempt) in c.termination_attempts.iter().enumerate() {
            construction_requests(
                producer,
                &mut out.requests,
                &object,
                CV::Array(
                    vec_in![nodes;s("termination_attempts", nodes)?, s(index, nodes)?, s("construction", nodes)?],
                ),
                &attempt.construction,
                &sources,
                nodes,
            )?;
        }
        // 完整当前前缀的 NO_FIRST / PENDING / KILLED 是已计算的语义结果；资源截断才缺计算。
        let resource_missing = c
            .second
            .as_ref()
            .is_some_and(|second| second.resource_truncated && second.matched.is_none())
            && !matches!(
                c.result,
                "FAILED_GEOMETRY" | "CASE_ONE" | "CASE_TWO" | "KILLED_CASE_TWO"
            );
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-013",
            "semantic_termination_result",
            s("result", nodes)?,
            if resource_missing {
                computation_pending_axes()
            } else {
                complete_axes()
            },
            if resource_missing {
                &["second_sequence_resource_bound"]
            } else {
                &[]
            },
            &sources,
            nodes,
        )?;
        let facts_complete =
            c.fact_vector.new_segment_formed.is_some() && c.fact_vector.segment_broken.is_some();
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-014",
            "known_fact_vector",
            s("fact_vector", nodes)?,
            if facts_complete {
                complete_axes()
            } else {
                computation_pending_axes()
            },
            if facts_complete {
                &[]
            } else {
                &["break_facts_scan_incomplete"]
            },
            &sources,
            nodes,
        )?;
        // 扫描范围与覆盖事实均由同次 parser 明确给出；覆盖不足也是已求出的覆盖分类。
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-014",
            "scan_coverage",
            s("fact_vector", nodes)?,
            complete_axes(),
            &[],
            &sources,
            nodes,
        )?;
        out.objects.push(object, nodes)?;
        out.segments.push(
            SegmentObject {
                candidate_index,
                entity_id: entity.clone_in(nodes)?,
                object_id: object_id.clone_in(nodes)?,
                sources: sources.clone_in(nodes)?,
            },
            nodes,
        )?;
        let mut relations = vec_in![nodes;(
            sequence.clone_in(nodes)?,
            "derived_from",
            entity.clone_in(nodes)?,
            sources.clone_in(nodes)?,
        )];
        if let Some(second) = &second_sequence {
            relations.push(
                (
                    second.clone_in(nodes)?,
                    "derived_from",
                    entity.clone_in(nodes)?,
                    sources.clone_in(nodes)?,
                ),
                nodes,
            )?;
        }
        for (elements, seq) in std::iter::once((&c.first.raw_elements, &sequence)).chain(
            c.second
                .as_ref()
                .zip(second_sequence.as_ref())
                .map(|(second, name)| (&second.raw_elements, name)),
        ) {
            for element in elements {
                let member = strokes
                    .strokes
                    .get(element.stroke_idx)
                    .ok_or("成员笔越界")?;
                let deps = sorted_sources(Vec::from_iter(
                    member
                        .start
                        .source_coords
                        .iter()
                        .chain(&member.end.source_coords)
                        .copied(),
                    nodes,
                )?);
                relations.push(
                    (
                        Text::format(
                            format_args!("bi:{}:{}", ctx.fact_generation, member.identity_anchor),
                            nodes,
                        )?,
                        "member_of",
                        seq.clone_in(nodes)?,
                        deps,
                    ),
                    nodes,
                )?;
            }
        }
        for (source, relation, target, deps) in relations {
            let data = put(
                &(obj(
                    [
                        ("source_id", s(&source, nodes)?),
                        ("relation_kind", s(relation, nodes)?),
                        ("target_id", s(&target, nodes)?),
                        ("version", s(revision, nodes)?),
                        ("witness_object_id", s(&object_id, nodes)?),
                    ],
                    nodes,
                )?),
                nodes,
            )?;
            out.objects.push(
                emit(
                    producer,
                    ctx,
                    crate::tb02c::KINDS[3],
                    CV::Array(
                        vec_in![nodes;s(&source, nodes)?, s(relation, nodes)?, s(&target, nodes)?],
                    ),
                    &data,
                    &deps,
                    nodes,
                )?,
                nodes,
            )?;
            out.relations.push(
                put(
                    &(obj(
                        [
                            ("subject", s(source, nodes)?),
                            ("relation_type", s(relation, nodes)?),
                            ("object", s(target, nodes)?),
                        ],
                        nodes,
                    )?),
                    nodes,
                )?,
                nodes,
            )?;
        }
        if changed {
            let change = match previous {
                None => "formed",
                Some(p) if matches!(c.result, "CASE_ONE" | "CASE_TWO") && p.result != c.result => {
                    "terminated"
                }
                _ => "updated",
            };
            let change_data = put(
                &(obj(
                    [
                        ("entity_id", s(&entity, nodes)?),
                        ("change", s(change, nodes)?),
                        ("before", previous.map_or(CV::Null, |p| edge(&p.data))),
                        ("after", edge(&data)),
                        ("known_at", edge(ctx.known_at)),
                        ("object_id", s(&object_id, nodes)?),
                    ],
                    nodes,
                )?),
                nodes,
            )?;
            out.objects.push(emit(producer, ctx, crate::tb02c::KINDS[4], CV::Array(vec_in![nodes;s(ctx.generation, nodes)?, s(&entity, nodes)?, s(change, nodes)?]), &change_data, &sources, nodes)?, nodes)?;
        }
        out.waiting_reasons
            .extend(c.waiting_reasons.iter().copied(), nodes)?;
    }
    let all_sources = sorted_sources(Vec::from_iter(ctx.all_sources.iter().copied(), nodes)?);
    for (entity, prior) in &old {
        if !active.contains(entity) {
            let data = put(
                &(obj(
                    [
                        ("entity_id", s(entity, nodes)?),
                        ("change", s("withdrawn", nodes)?),
                        ("before", edge(&prior.data)),
                        ("after", CV::Null),
                        ("known_at", edge(ctx.known_at)),
                        ("object_id", CV::Null),
                    ],
                    nodes,
                )?),
                nodes,
            )?;
            out.objects.push(emit(producer, ctx, crate::tb02c::KINDS[4], CV::Array(vec_in![nodes;s(ctx.generation, nodes)?, s(entity, nodes)?, s("withdrawn", nodes)?]), &data, &all_sources, nodes)?, nodes)?;
        }
    }
    if facts.parse_invalid_from.is_some() {
        out.waiting_reasons
            .push(crate::tb02c::PARSE_INVALID_REASON, nodes)?;
    }
    out.waiting_reasons.sort_unstable();
    out.waiting_reasons.dedup();
    let waiting = fact(nodes, &out.waiting_reasons)?;
    // 唯一公开 checkpoint 枚举口的权威 wire；选择/排序仍由 SecondFoldStore 决定。
    let mut checkpoint_values = Vec::new();
    for cursor in evidence.second_store.checkpoint_cursors() {
        let cursor = fact(nodes, &cursor.wire_view())?;
        checkpoint_values.push(edge(&cursor), nodes)?;
    }
    let checkpoint = put(&CV::Array(checkpoint_values), nodes)?;
    let data = put(
        &(obj(
            [
                ("input_frontier", s(ctx.frontier, nodes)?),
                ("waiting_reasons", edge(&waiting)),
                ("known_at", edge(ctx.known_at)),
                (
                    "delivered_scope",
                    s("seed_strict_first_and_second_terminal", nodes)?,
                ),
                (
                    "deferred_scope",
                    CV::Array(
                        vec_in![nodes;s("general_equal_identity", nodes)?, s("global_G006_bridge", nodes)?],
                    ),
                ),
                (
                    "scan_budget",
                    obj(
                        [
                            ("policy", s(evidence.second_store.policy.label(), nodes)?),
                            (
                                "round_strokes",
                                s(evidence.second_store.round_budget, nodes)?,
                            ),
                        ],
                        nodes,
                    )?,
                ),
                ("scan_cursors", edge(&checkpoint)),
            ],
            nodes,
        )?),
        nodes,
    )?;
    out.objects.push(
        emit(
            producer,
            ctx,
            crate::tb02c::KINDS[5],
            s("TB-02-W", nodes)?,
            &data,
            &all_sources,
            nodes,
        )?,
        nodes,
    )?;
    Ok(out)
}

/// E/F 的已授权上一代活跃对象根。允许 seed/center/roles/assignment/position/pair 与版本，
/// 不接受任意历史 JSON 或旧 obj-；版本虽不参与实体比较，也必须验证。
pub(super) struct EfPrior<'a> {
    pub records: &'a [Built],
}
fn header(kind: &str) -> (&'static str, &'static str, &'static str) {
    if crate::tb02e::KINDS.contains(&kind) {
        (
            crate::tb02e::SCHEMA_REVISION,
            "center-seed/1",
            "strict_seed_zn_outer_c_resume",
        )
    } else if crate::tb02f::KINDS.contains(&kind) {
        (
            crate::tb02f::SCHEMA_REVISION,
            "center-pair/1",
            "zn_outer_five_leaf_pending_ge9",
        )
    } else {
        (
            crate::tb02c::SCHEMA,
            CONSTRUCTION_RULE,
            "seed_closed_strict_second",
        )
    }
}
fn direction(dir: Direction) -> &'static str {
    match dir {
        Direction::Up => "UP",
        Direction::Down => "DOWN",
    }
}
fn anchor(id: &str) -> &str {
    id.strip_prefix("segment:").unwrap_or(id)
}
fn center_anchor(id: &str) -> &str {
    id.strip_prefix("center:").unwrap_or(id)
}
fn unit_value(u: &crate::tb02e::SegUnit, nodes: &dyn sc::NodeSource) -> Result<CV, String> {
    Ok(match u.range {
        Some(r) => obj(
            [
                ("unit_id", s(&u.unit_id, nodes)?),
                ("unit_object_id", s(&u.object_id, nodes)?),
                ("direction", s(direction(r.direction), nodes)?),
                ("lo", s(r.lo, nodes)?),
                ("hi", s(r.hi, nodes)?),
            ],
            nodes,
        )?,
        None => obj(
            [
                ("unit_id", s(&u.unit_id, nodes)?),
                ("unit_object_id", s(&u.object_id, nodes)?),
                ("completed", CV::Bool(false)),
            ],
            nodes,
        )?,
    })
}
fn unit_sources<'a>(
    units: impl IntoIterator<Item = &'a crate::tb02e::SegUnit>,
    nodes: &dyn sc::NodeSource,
) -> Result<Vec<usize>, String> {
    Ok(sorted_sources(Vec::from_iter(
        units.into_iter().flat_map(|u| u.sources.iter().copied()),
        nodes,
    )?))
}
fn vec_fields(value: CV) -> Result<Vec<(Text, CV)>, String> {
    match value {
        CV::Object(v) => Ok(v),
        _ => Err("内部字段非对象".into()),
    }
}
fn root_eq(old: &Built, key: &str, new: CV, nodes: &mut dyn NodeStore) -> Result<bool, String> {
    Ok(field(nodes, old, key)?.digest == put(&(new), nodes)?.digest)
}
fn revision_of(nodes: &mut dyn NodeStore, old: Option<&Built>) -> Result<i64, String> {
    match old {
        None => Ok(0),
        Some(old) => {
            let value = field_text(nodes, old, "entity_revision")?;
            let n = value.parse::<i64>().map_err(|e| e.to_string())?;
            if n < 1 || Text::format(format_args!("{n}"), nodes)? != value {
                return Err("非法旧修订".into());
            }
            Ok(n)
        }
    }
}
fn next_revision(nodes: &mut dyn NodeStore, old: Option<&Built>) -> Result<i64, String> {
    revision_of(nodes, old)?
        .checked_add(1)
        .ok_or_else(|| "修订溢出".into())
}
/// 比较真实值边；允许的簿记键显式列出，不渲染完整旧载荷。
fn same_fields(
    nodes: &mut dyn NodeStore,
    old: &Built,
    fresh: &[(Text, CV)],
    bookkeeping: &[&str],
) -> Result<bool, String> {
    let mut names = Vec::from_iter(fresh.iter().map(|(k, _)| k.as_str()), nodes)?;
    for k in bookkeeping {
        if !names.contains(k) {
            names.push(k, nodes)?;
        }
    }
    let r = nodes.native_built_ref(old).map_err(|e| e.to_string())?;
    let fields = nodes
        .lookup_fields_ref(r.view(), &names)
        .map_err(|e| e.to_string())?;
    for (key, value) in fresh {
        if !bookkeeping.contains(&key.as_str()) {
            let old = fields.get(key.as_str()).ok_or("旧对象缺事实字段")?;
            if old.digest().map_err(|e| e.to_string())? != put(value, nodes)?.digest {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn keep_known_node(
    old: Option<&Built>,
    value: CV,
    nodes: &mut dyn NodeStore,
) -> Result<Built, String> {
    let fields = vec_fields(value)?;
    if let Some(old) = old {
        if same_fields(nodes, old, &fields, &["known_at"])? {
            return Ok(old.clone());
        }
    }
    put(&(CV::Object(fields)), nodes)
}
fn revise_node(
    old: Option<&Built>,
    value: CV,
    nodes: &mut dyn NodeStore,
) -> Result<(Built, i64, bool), String> {
    let mut fields = vec_fields(value)?;
    let changed = match old {
        Some(old) => !same_fields(nodes, old, &fields, &["entity_revision"])?,
        None => true,
    };
    let revision = revision_of(nodes, old)?
        .checked_add(i64::from(changed))
        .ok_or("修订溢出")?;
    replace_value(&mut fields, "entity_revision", s(revision, nodes)?)?;
    Ok((put(&(CV::Object(fields)), nodes)?, revision, changed))
}
type StructureEdge = (Text, &'static str, Text, Text, Text, Vec<usize>);
#[allow(clippy::too_many_arguments)]
fn change_object(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    ctx: &StructureContext<'_>,
    kind: &str,
    entity: &str,
    change: &str,
    before: Option<&Built>,
    after: Option<&Built>,
    object_id: Option<&str>,
    sources: &[usize],
) -> Result<TypedObject, String> {
    let data = put(
        &(obj(
            [
                ("entity_id", s(entity, nodes)?),
                ("change", s(change, nodes)?),
                ("before", before.map_or(CV::Null, edge)),
                ("after", after.map_or(CV::Null, edge)),
                ("known_at", edge(ctx.known_at)),
                (
                    "object_id",
                    object_id
                        .map(|v| s(v, nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
            ],
            nodes,
        )?),
        nodes,
    )?;
    emit(
        producer,
        ctx,
        kind,
        CV::Array(vec_in![nodes;s(ctx.generation, nodes)?, s(entity, nodes)?, s(change, nodes)?]),
        &data,
        sources,
        nodes,
    )
}
fn emit_edges(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    ctx: &StructureContext<'_>,
    kind: &str,
    edges: Vec<StructureEdge>,
    out: &mut StructureOutput,
) -> Result<(), String> {
    for (source, relation, target, version, witness, sources) in edges {
        let data = put(
            &(obj(
                [
                    ("source_id", s(&source, nodes)?),
                    ("relation_kind", s(relation, nodes)?),
                    ("target_id", s(&target, nodes)?),
                    ("version", s(version, nodes)?),
                    ("witness_object_id", s(witness, nodes)?),
                ],
                nodes,
            )?),
            nodes,
        )?;
        out.objects.push(
            emit(
                producer,
                ctx,
                kind,
                CV::Array(
                    vec_in![nodes;s(&source, nodes)?, s(relation, nodes)?, s(&target, nodes)?],
                ),
                &data,
                &sources,
                nodes,
            )?,
            nodes,
        )?;
        out.relations.push(
            put(
                &(obj(
                    [
                        ("subject", s(source, nodes)?),
                        ("relation_type", s(relation, nodes)?),
                        ("object", s(target, nodes)?),
                    ],
                    nodes,
                )?),
                nodes,
            )?,
            nodes,
        )?;
    }
    Ok(())
}
/// 同次 C/D 对象映射 → 已提取的共享输入合同；不按数组位置合成对象身份。
/// donor tb02e::units_from_segment_rows 的同一输入合同，直接借同次事实逐项核。
/// 只把通过核验的成员极值映射到塔 L0；未完成的内部段、断链、方向或范围不符均拒绝。
fn segment_units(
    evidence: &StructureEvidence<'_>,
    segments: &[SegmentObject],
    nodes: &mut dyn NodeStore,
) -> Result<crate::tb02e::UnitInputs, String> {
    use crate::tb02e::{SegUnit, UnitInputs};
    fn miss(
        check: &str,
        position: CV,
        expected: CV,
        actual: CV,
        out: &mut Vec<CV>,
        nodes: &dyn sc::NodeSource,
    ) -> Result<(), String> {
        out.push(
            obj(
                [
                    ("check", s(check, nodes)?),
                    ("position", position),
                    ("expected", expected),
                    ("actual", actual),
                ],
                nodes,
            )?,
            nodes,
        )
    }
    if segments.len() != evidence.segment_facts.candidates.len() {
        return Err("C/D 段映射未完整覆盖同次候选".into());
    }
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for p in segments {
        if !seen.insert(p.candidate_index, nodes)? {
            return Err("重复段候选映射".into());
        }
        let c = evidence
            .segment_facts
            .candidates
            .get(p.candidate_index)
            .ok_or("段映射候选越界")?;
        rows.push((p, c), nodes)?;
    }
    rows.sort_unstable_by_key(|(p, c)| (c.start_stroke, p.candidate_index));
    let tower = newchan_rust::theta_v0::classifier::center_scan::l0_units(evidence.segments);
    let mut out = UnitInputs {
        completed: Vec::new(),
        in_progress: None,
    };
    let mut mismatches = Vec::new();
    let mut completed_count = 0usize;
    let mut previous_end = None;
    for (i, (p, c)) in rows.iter().enumerate() {
        let position = || {
            obj(
                [
                    ("candidate_index", s(i, nodes)?),
                    ("unit_id", s(&p.entity_id, nodes)?),
                ],
                nodes,
            )
        };
        let completed = matches!(c.result, "CASE_ONE" | "CASE_TWO");
        if completed || i + 1 == rows.len() {
            if previous_end.is_some_and(|end: usize| end.checked_add(1) != Some(c.start_stroke)) {
                miss(
                    "contiguity",
                    position()?,
                    s(previous_end.unwrap() as i128 + 1, nodes)?,
                    s(c.start_stroke, nodes)?,
                    &mut mismatches,
                    nodes,
                )?;
            }
        } else {
            miss(
                "interior_incomplete",
                position()?,
                s("CASE_ONE|CASE_TWO", nodes)?,
                s("incomplete", nodes)?,
                &mut mismatches,
                nodes,
            )?;
            continue;
        }
        let range = if completed {
            let end = c.end_stroke.ok_or("完成段缺端笔")?;
            let start = evidence
                .stroke_facts
                .strokes
                .get(c.start_stroke)
                .ok_or("段起笔越界")?;
            let finish = evidence.stroke_facts.strokes.get(end).ok_or("段端越界")?;
            let direction = c.construction.direction.unwrap_or("");
            let price_direction = if finish.end.price > start.start.price {
                "UP"
            } else {
                "DOWN"
            };
            if direction != price_direction {
                miss(
                    "direction",
                    position()?,
                    s(price_direction, nodes)?,
                    s(direction, nodes)?,
                    &mut mismatches,
                    nodes,
                )?;
            }
            let mut extremes: Option<(i64, i64)> = None;
            for &m in &c.construction.stroke_indices {
                if m < c.start_stroke || m > c.observed_through {
                    return Err(format!(
                        "TB02E_INPUT_CONTRACT：成员笔 {m} 不在 CC-013 stroke_refs 中"
                    ));
                }
                let bi = evidence.stroke_facts.strokes.get(m).ok_or("成员笔越界")?;
                let (s, e) = (bi.start.price, bi.end.price);
                let (lo, hi) = extremes.unwrap_or((s.min(e), s.max(e)));
                extremes = Some((lo.min(s).min(e), hi.max(s).max(e)));
            }
            let (lo, hi) = extremes.ok_or("TB02E_INPUT_CONTRACT：完成段没有成员笔")?;
            let k = completed_count;
            completed_count += 1;
            previous_end = Some(end);
            if let Some(u) = tower.get(k) {
                let position = || {
                    obj(
                        [
                            ("completed_index", s(k, nodes)?),
                            ("unit_id", s(&p.entity_id, nodes)?),
                        ],
                        nodes,
                    )
                };
                for (check, expected, actual) in [("lo", u.lo, lo), ("hi", u.hi, hi)] {
                    if expected != actual {
                        miss(
                            check,
                            position()?,
                            s(expected, nodes)?,
                            s(actual, nodes)?,
                            &mut mismatches,
                            nodes,
                        )?;
                    }
                }
                if direction != self::direction(u.direction) {
                    miss(
                        "direction",
                        position()?,
                        s(self::direction(u.direction), nodes)?,
                        s(direction, nodes)?,
                        &mut mismatches,
                        nodes,
                    )?;
                }
                Some(*u)
            } else {
                None
            }
        } else {
            None
        };
        let unit = SegUnit {
            unit_id: p.entity_id.clone_in(nodes)?,
            object_id: p.object_id.clone_in(nodes)?,
            range,
            sources: p.sources.clone_in(nodes)?,
        };
        if completed {
            out.completed.push(unit, nodes)?;
        } else {
            out.in_progress = Some(unit);
        }
    }
    if completed_count != tower.len() {
        miss(
            "count",
            obj([("completed_units", s(completed_count, nodes)?)], nodes)?,
            s(tower.len(), nodes)?,
            s(completed_count, nodes)?,
            &mut mismatches,
            nodes,
        )?;
    }
    if !mismatches.is_empty() {
        let root = put(&CV::Array(mismatches), nodes)?;
        return Err(format!(
            "TB02E_INPUT_CONTRACT：段对象与塔 L0 单元不符，本 cut 拒绝提交：{}",
            canonical_root_text(nodes, &root)?
        ));
    }
    Ok(out)
}

/// E/F 同一中心扫描的直接内容投影；所有判据来自 center_scan/center 共享核。
pub(super) fn build_ef(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    evidence: &StructureEvidence<'_>,
    cd_segments: &[SegmentObject],
    prior: &EfPrior<'_>,
    ctx: &StructureContext<'_>,
) -> Result<StructureOutput, String> {
    let inputs = segment_units(evidence, cd_segments, nodes)?;
    build_ef_units(producer, nodes, &inputs, prior, ctx)
}
fn build_ef_units(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    inputs: &crate::tb02e::UnitInputs,
    prior: &EfPrior<'_>,
    ctx: &StructureContext<'_>,
) -> Result<StructureOutput, String> {
    use newchan_rust::theta_v0::classifier::{
        center::{
            center_from_segments, CenterRoles, PairEndpoint, PairLeaf, RoleVar, SeedExclusion,
            SeedInput, SeedRule,
        },
        center_scan::{
            scan_center_facts_full, PairOutcome, ScanState, SeedFromSelf, UnitAssignment,
        },
    };
    let units = Vec::try_from_iter(
        inputs
            .completed
            .iter()
            .map(|u| u.range.ok_or_else(|| "完成单元缺区间".to_string())),
        nodes,
    )?;
    let facts = scan_center_facts_full(&units, center_from_segments);
    if facts
        .attempts
        .iter()
        .any(|a| a.facts.exclusion(SeedRule::Complete) == Some(SeedExclusion::NotAlternating))
    {
        return Err("TB02E_INPUT_CONTRACT：完成单元方向未交替".into());
    }
    let windows = facts.window_facts(&units);
    let ids = Vec::from_iter(inputs.completed.iter().map(|u| u.unit_id.as_str()), nodes)?;
    let center_ids = Vec::try_from_iter(
        windows
            .iter()
            .map(|w| Text::format(format_args!("center:{}", anchor(ids[w.first])), nodes)),
        nodes,
    )?;
    let mut previous = BTreeMap::<(Text, Text), Built>::new();
    let mut prior_versions = BTreeSet::new();
    for record in prior.records {
        let kind = field_text(nodes, record, "kind")?;
        if !crate::tb02e::KINDS[..4].contains(&kind.as_str())
            && !crate::tb02f::KINDS[..2].contains(&kind.as_str())
            && kind != crate::tb02e::KINDS[6]
            && kind != crate::tb02f::KINDS[4]
        {
            return Err(format!("EF prior kind 不允许: {kind}"));
        }
        let data = verify_prior(nodes, producer, record, &kind)?;
        if kind == crate::tb02e::KINDS[6] || kind == crate::tb02f::KINDS[4] {
            if !prior_versions.insert(kind, nodes)? {
                return Err("EF prior 重复版本根".into());
            }
            continue;
        }
        let entity = field_text(nodes, &data, "entity_id")?;
        if previous.insert((kind, entity), data, nodes)?.is_some() {
            return Err("EF prior 重复实体".into());
        }
    }
    if !prior.records.is_empty()
        && (!prior_versions.contains(crate::tb02e::KINDS[6])
            || !prior_versions.contains(crate::tb02f::KINDS[4]))
    {
        return Err("EF prior 缺中心或中枢对版本根".into());
    }
    let old = |kind: &str, entity: &str| {
        previous
            .iter()
            .find(|((k, e), _)| k.as_str() == kind && e.as_str() == entity)
            .map(|(_, v)| v)
    };
    let mut out = StructureOutput {
        requests: Vec::new(),
        objects: Vec::new(),
        relations: Vec::new(),
        segments: Vec::new(),
        waiting_reasons: Vec::new(),
    };
    let mut edges: Vec<StructureEdge> = Vec::new();
    let mut attempt_ids = BTreeMap::new();
    for a in &facts.attempts {
        let seed = &inputs.completed[a.at..a.at + 3];
        let seed_units = &units[a.at..a.at + 3];
        let roles = CenterRoles::of_members(seed_units);
        let extremum = |var: RoleVar| -> Result<CV, String> {
            let contributors = Vec::try_from_iter(
                roles
                    .contributors(seed_units, var)
                    .into_iter()
                    .map(|k| s(ids[a.at + k], nodes)),
                nodes,
            )?;
            let witness = if contributors.len() >= 2 {
                contributors.clone_in(nodes)?
            } else {
                Vec::new()
            };
            obj(
                [
                    ("value", s(roles.value(var), nodes)?),
                    ("contributors", CV::Array(contributors)),
                    ("same_price_witness", CV::Array(witness)),
                ],
                nodes,
            )
        };
        let v = a.facts.condition_vector(SeedInput::TOWER);
        let entity = Text::format(format_args!("seed_attempt:{}", anchor(ids[a.at])), nodes)?;
        let center = facts
            .windows
            .iter()
            .position(|w| w.seed_at == a.at)
            .map(|w| s(&center_ids[w], nodes))
            .transpose()?
            .unwrap_or(CV::Null);
        let value = obj(
            [
                ("entity_id", s(&entity, nodes)?),
                (
                    "units",
                    CV::Array(Vec::try_from_iter(
                        seed.iter().map(|u| unit_value(u, nodes)),
                        nodes,
                    )?),
                ),
                (
                    "vector",
                    obj(
                        [
                            ("completed", CV::Bool(v.completed)),
                            ("consecutive", CV::Bool(v.consecutive)),
                            ("same_level", CV::Bool(v.same_level)),
                            ("alternating", CV::Bool(v.alternating)),
                            ("strict", v.strict.map_or(CV::Null, CV::Bool)),
                        ],
                        nodes,
                    )?,
                ),
                ("ZD", extremum(RoleVar::Zd)?),
                ("ZG", extremum(RoleVar::Zg)?),
                ("core_order", s(a.facts.core_order.as_str(), nodes)?),
                (
                    "result",
                    s(if a.formed { "FORMED" } else { "EXCLUDED" }, nodes)?,
                ),
                (
                    "exclusion_reason",
                    a.facts
                        .exclusion(SeedRule::Complete)
                        .map(|r| s(r.as_str(), nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                ("center_id", center),
                ("known_at", edge(ctx.known_at)),
            ],
            nodes,
        )?;
        let data = keep_known_node(old(crate::tb02e::KINDS[0], &entity), value, nodes)?;
        let sources = unit_sources(seed, nodes)?;
        let object = emit(
            producer,
            ctx,
            crate::tb02e::KINDS[0],
            s(&entity, nodes)?,
            &data,
            &sources,
            nodes,
        )?;
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-016",
            "seed_condition_vector",
            s("vector", nodes)?,
            if v.strict.is_some() {
                complete_axes()
            } else {
                missing_axes()
            },
            if v.strict.is_some() {
                &[]
            } else {
                &["seed_strict_not_evaluated"]
            },
            &sources,
            nodes,
        )?;
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-016",
            "core_order",
            s("core_order", nodes)?,
            complete_axes(),
            &[],
            &sources,
            nodes,
        )?;
        edges.push(
            (
                entity.clone_in(nodes)?,
                "attempted_from",
                Text::new(ids[a.at], nodes)?,
                Text::new("1", nodes)?,
                Text::new(&object.object_id, nodes)?,
                sources,
            ),
            nodes,
        )?;
        out.objects.push(object, nodes)?;
        attempt_ids.insert(a.at, entity, nodes)?;
    }
    let mut revisions = Vec::new();
    for (w, wf) in windows.iter().enumerate() {
        let entity = &center_ids[w];
        let members = &inputs.completed[wf.first..=wf.last];
        let member_ids = Vec::try_from_iter((wf.first..=wf.last).map(|k| s(ids[k], nodes)), nodes)?;
        let roles_entity = Text::format(format_args!("roles:{entity}"), nodes)?;
        let status = if wf.upgrade_pending {
            "UPGRADE_PENDING_TB02H"
        } else if wf.settled {
            "SETTLED"
        } else {
            "EXTENDING"
        };
        let next = wf
            .next_tested_unit
            .filter(|&k| k == units.len())
            .and(inputs.in_progress.as_ref())
            .map(|u| s(&u.unit_id, nodes))
            .transpose()?
            .unwrap_or(CV::Null);
        let break_unit = wf
            .break_unit
            .map(|k| s(ids[k], nodes))
            .transpose()?
            .unwrap_or(CV::Null);
        let prior = old(crate::tb02e::KINDS[1], entity);
        let settled = if wf.settled {
            match prior {
                Some(p)
                    if root_eq(p, "settled", CV::Bool(true), nodes)?
                        && root_eq(p, "break_unit", break_unit.clone_in(nodes)?, nodes)? =>
                {
                    field(nodes, p, "settled_known_at")?
                }
                _ => ctx.known_at.clone(),
            }
        } else {
            put(&(CV::Null), nodes)?
        };
        let formed = match prior {
            Some(p) => field(nodes, p, "formed_known_at")?,
            None => ctx.known_at.clone(),
        };
        let value = obj(
            [
                ("entity_id", s(entity, nodes)?),
                ("entity_revision", s(1, nodes)?),
                (
                    "seed_attempt_id",
                    s(
                        attempt_ids.get(&wf.first).ok_or("中心缺真实seed attempt")?,
                        nodes,
                    )?,
                ),
                (
                    "seed_units",
                    CV::Array(Vec::try_from_iter(
                        member_ids[..3].iter().map(|v| v.clone_in(nodes)),
                        nodes,
                    )?),
                ),
                (
                    "zn_direction",
                    s(direction(units[wf.first].direction), nodes)?,
                ),
                ("members", CV::Array(member_ids.clone_in(nodes)?)),
                (
                    "zn_members",
                    CV::Array(Vec::try_from_iter(
                        wf.zn_members.iter().map(|&k| s(ids[k], nodes)),
                        nodes,
                    )?),
                ),
                ("status", s(status, nodes)?),
                ("settled", CV::Bool(wf.settled)),
                ("break_unit", break_unit),
                ("break_up", CV::Bool(wf.break_up.unwrap_or(false))),
                ("break_up_defined", CV::Bool(wf.break_up.is_some())),
                (
                    "resume_anchor",
                    if wf.settled {
                        obj(
                            [
                                ("after_unit_id", s(ids[wf.last], nodes)?),
                                ("offset", s(2, nodes)?),
                            ],
                            nodes,
                        )?
                    } else {
                        CV::Null
                    },
                ),
                ("next_tested_unit", next),
                ("roles_id", s(&roles_entity, nodes)?),
                ("formed_known_at", edge(&formed)),
                ("settled_known_at", edge(&settled)),
            ],
            nodes,
        )?;
        let (data, revision, changed) = revise_node(prior, value, nodes)?;
        let mut sources = unit_sources(members, nodes)?;
        if let Some(k) = wf.break_unit {
            sources.extend(inputs.completed[k].sources.iter().copied(), nodes)?;
        }
        let sources = sorted_sources(sources);
        let object = emit(
            producer,
            ctx,
            crate::tb02e::KINDS[1],
            s(entity, nodes)?,
            &data,
            &sources,
            nodes,
        )?;
        let oid = Text::new(&object.object_id, nodes)?;
        out.objects.push(object, nodes)?;
        let role = |var: RoleVar| -> Result<CV, String> {
            let r = wf.role(var);
            let hidden = wf.upgrade_pending && !var.frozen();
            let contributors: Vec<_> = if hidden {
                Vec::new()
            } else {
                Vec::try_from_iter(r.contributors.iter().map(|&k| s(ids[k], nodes)), nodes)?
            };
            let witness = if contributors.len() >= 2 {
                contributors.clone_in(nodes)?
            } else {
                Vec::new()
            };
            obj(
                [
                    ("value", if hidden { CV::Null } else { s(r.value, nodes)? }),
                    ("contributors", CV::Array(contributors)),
                    ("same_price_witness", CV::Array(witness)),
                    ("frozen", CV::Bool(var.frozen())),
                    (
                        "null_reason",
                        if hidden {
                            s("UPGRADE_PENDING_TB02H", nodes)?
                        } else {
                            CV::Null
                        },
                    ),
                ],
                nodes,
            )
        };
        let roles = put(
            &(obj(
                [
                    (
                        "CORE",
                        obj(
                            [("ZD", role(RoleVar::Zd)?), ("ZG", role(RoleVar::Zg)?)],
                            nodes,
                        )?,
                    ),
                    (
                        "OUTER",
                        obj(
                            [("DD", role(RoleVar::Dd)?), ("GG", role(RoleVar::Gg)?)],
                            nodes,
                        )?,
                    ),
                    (
                        "INNER_EXTREMA",
                        obj([("D", role(RoleVar::D)?), ("G", role(RoleVar::G)?)], nodes)?,
                    ),
                ],
                nodes,
            )?),
            nodes,
        )?;
        let old_roles = old(crate::tb02e::KINDS[2], &roles_entity);
        let roles_data = match old_roles {
            Some(p)
                if root_eq(p, "roles", edge(&roles), nodes)?
                    && root_eq(p, "center_id", s(entity, nodes)?, nodes)? =>
            {
                p.clone()
            }
            _ => {
                let revision = next_revision(nodes, old_roles)?;
                let prev = match old_roles {
                    Some(p) => obj(
                        [
                            (
                                "entity_revision",
                                edge(&field(nodes, p, "entity_revision")?),
                            ),
                            ("roles", edge(&field(nodes, p, "roles")?)),
                        ],
                        nodes,
                    )?,
                    None => CV::Null,
                };
                put(
                    &(obj(
                        [
                            ("entity_id", s(&roles_entity, nodes)?),
                            ("entity_revision", s(revision, nodes)?),
                            ("center_id", s(entity, nodes)?),
                            ("roles", edge(&roles)),
                            ("previous", prev),
                            ("known_at", edge(ctx.known_at)),
                        ],
                        nodes,
                    )?),
                    nodes,
                )?
            }
        };
        let roles_changed = old_roles.is_some_and(|p| p.digest != roles_data.digest);
        let roles_object = emit(
            producer,
            ctx,
            crate::tb02e::KINDS[2],
            s(&roles_entity, nodes)?,
            &roles_data,
            &sources,
            nodes,
        )?;
        let roles_oid = Text::new(&roles_object.object_id, nodes)?;
        out.objects.push(roles_object, nodes)?;
        if !wf.upgrade_pending {
            for k in wf.first..=wf.last {
                edges.push(
                    (
                        Text::new(ids[k], nodes)?,
                        "member_of",
                        entity.clone_in(nodes)?,
                        Text::format(format_args!("{revision}"), nodes)?,
                        oid.clone_in(nodes)?,
                        inputs.completed[k].sources.clone_in(nodes)?,
                    ),
                    nodes,
                )?;
            }
        }
        let mut changes = Vec::new();
        match prior {
            None => changes.push("formed", nodes)?,
            Some(p) if changed => {
                let count = field(nodes, p, "members")?
                    .extent
                    .ok_or("旧members缺长度")?;
                if count < member_ids.len() as u64 {
                    changes.push("member_added", nodes)?;
                }
                if !root_eq(p, "settled", CV::Bool(true), nodes)? && wf.settled {
                    changes.push("settled", nodes)?;
                }
                if field_text(nodes, p, "status")? != "UPGRADE_PENDING_TB02H" && wf.upgrade_pending
                {
                    changes.push("upgrade_pending", nodes)?;
                }
            }
            _ => {}
        }
        for change in changes {
            out.objects.push(
                change_object(
                    producer,
                    nodes,
                    ctx,
                    crate::tb02e::KINDS[5],
                    entity,
                    change,
                    prior,
                    Some(&data),
                    Some(&oid),
                    &sources,
                )?,
                nodes,
            )?;
        }
        if roles_changed {
            out.objects.push(
                change_object(
                    producer,
                    nodes,
                    ctx,
                    crate::tb02e::KINDS[5],
                    &roles_entity,
                    "roles_updated",
                    old_roles,
                    Some(&roles_data),
                    Some(&roles_oid),
                    &sources,
                )?,
                nodes,
            )?;
        }
        revisions.push(revision, nodes)?;
    }
    let all_sources = sorted_sources(Vec::from_iter(ctx.all_sources.iter().copied(), nodes)?);
    for ((kind, entity), p) in &previous {
        if kind == crate::tb02e::KINDS[1] && !center_ids.contains(entity) {
            out.objects.push(
                change_object(
                    producer,
                    nodes,
                    ctx,
                    crate::tb02e::KINDS[5],
                    entity,
                    "withdrawn",
                    Some(p),
                    None,
                    None,
                    &all_sources,
                )?,
                nodes,
            )?;
        }
    }
    let unit_facts = facts.unit_facts();
    for (unit, f) in unit_facts
        .iter()
        .enumerate()
        .map(|(k, f)| (&inputs.completed[k], Some(*f)))
        .chain(inputs.in_progress.iter().map(|u| (u, None)))
    {
        let entity = Text::format(format_args!("assign:{}", unit.unit_id), nodes)?;
        let (status, center, left, right, pending, reason, seed) = match f {
            None => (
                "NOT_YET_ASSIGNED",
                None,
                None,
                None,
                false,
                Some("UNIT_NOT_COMPLETED"),
                SeedFromSelf::NotEvaluated,
            ),
            Some(f) => match f.assignment {
                UnitAssignment::Member { center } => (
                    "CENTER_MEMBER",
                    Some(center),
                    None,
                    None,
                    false,
                    None,
                    f.seed_from_self,
                ),
                UnitAssignment::Connector { left, right } => (
                    "CONNECTOR",
                    None,
                    Some(left),
                    right,
                    right.is_none(),
                    None,
                    f.seed_from_self,
                ),
                UnitAssignment::NotYetAssigned(r) => (
                    "NOT_YET_ASSIGNED",
                    None,
                    None,
                    None,
                    false,
                    Some(r.as_str()),
                    f.seed_from_self,
                ),
            },
        };
        if let Some(reason) = reason {
            out.waiting_reasons.push(reason, nodes)?;
        }
        let value = obj(
            [
                ("entity_id", s(&entity, nodes)?),
                ("unit", unit_value(unit, nodes)?),
                ("status", s(status, nodes)?),
                (
                    "center_id",
                    center
                        .map(|w| s(&center_ids[w], nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                (
                    "left_center_id",
                    left.map(|w| s(&center_ids[w], nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                (
                    "right_center_id",
                    right
                        .map(|w| s(&center_ids[w], nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                ("right_pending", CV::Bool(pending)),
                (
                    "reason",
                    reason.map(|v| s(v, nodes)).transpose()?.unwrap_or(CV::Null),
                ),
                ("seed_from_self", s(seed.as_str(), nodes)?),
                ("known_at", edge(ctx.known_at)),
            ],
            nodes,
        )?;
        let data = keep_known_node(old(crate::tb02e::KINDS[3], &entity), value, nodes)?;
        let object = emit(
            producer,
            ctx,
            crate::tb02e::KINDS[3],
            s(&entity, nodes)?,
            &data,
            &unit.sources,
            nodes,
        )?;
        let oid = Text::new(&object.object_id, nodes)?;
        // 全读法扫描同时给出 member/connector/not-yet；未完成单元的已知未归属状态不是 false 或漏算。
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-022",
            "assignment_status",
            s("status", nodes)?,
            complete_axes(),
            &[],
            &unit.sources,
            nodes,
        )?;
        out.objects.push(object, nodes)?;
        for (relation, w) in [("connects_left", left), ("connects_right", right)] {
            if let Some(w) = w {
                edges.push(
                    (
                        unit.unit_id.clone_in(nodes)?,
                        relation,
                        center_ids[w].clone_in(nodes)?,
                        Text::format(format_args!("{}", revisions[w]), nodes)?,
                        oid.clone_in(nodes)?,
                        unit.sources.clone_in(nodes)?,
                    ),
                    nodes,
                )?;
            }
        }
    }
    emit_edges(
        producer,
        nodes,
        ctx,
        crate::tb02e::KINDS[4],
        edges,
        &mut out,
    )?;
    out.waiting_reasons.sort_unstable();
    out.waiting_reasons.dedup();
    let len = inputs.completed.len();
    let id_at = |k: usize| {
        if k < len {
            Some(inputs.completed[k].unit_id.as_str())
        } else if k == len {
            inputs.in_progress.as_ref().map(|u| u.unit_id.as_str())
        } else {
            None
        }
    };
    let awaiting = |k: usize| -> Result<(CV, CV, CV), String> {
        Ok(match id_at(k) {
            Some(id) => (s(id, nodes)?, CV::Null, CV::Null),
            None => (
                CV::Null,
                len.checked_sub(1)
                    .map(|i| s(&inputs.completed[i].unit_id, nodes))
                    .transpose()?
                    .unwrap_or(CV::Null),
                s(k as i128 - (len as i128 - 1), nodes)?,
            ),
        })
    };
    let cursor = match facts.scan_state() {
        ScanState::Extending { center, next_unit } => {
            let (unit, after, offset) = awaiting(next_unit)?;
            obj(
                [
                    ("state", s("EXTENDING", nodes)?),
                    ("center_id", s(&center_ids[center], nodes)?),
                    ("seed_start_unit_id", CV::Null),
                    ("awaiting_unit_id", unit),
                    ("awaiting_after_unit_id", after),
                    ("awaiting_offset", offset),
                ],
                nodes,
            )?
        }
        ScanState::SeekingSeed {
            seed_start,
            awaiting_unit,
        } => {
            let (unit, after, offset) = awaiting(awaiting_unit)?;
            obj(
                [
                    ("state", s("SEEKING_SEED", nodes)?),
                    ("center_id", CV::Null),
                    (
                        "seed_start_unit_id",
                        id_at(seed_start)
                            .map(|v| s(v, nodes))
                            .transpose()?
                            .unwrap_or(CV::Null),
                    ),
                    ("awaiting_unit_id", unit),
                    ("awaiting_after_unit_id", after),
                    ("awaiting_offset", offset),
                ],
                nodes,
            )?
        }
    };
    let waiting = fact(nodes, &out.waiting_reasons)?;
    let deferred = fact(nodes, &crate::tb02e::DEFERRED_SCOPE)?;
    let data = put(
        &(obj(
            [
                ("input_frontier", s(ctx.frontier, nodes)?),
                ("known_at", edge(ctx.known_at)),
                ("scan_cursor", cursor),
                ("waiting_reasons", edge(&waiting)),
                (
                    "delivered_scope",
                    s("l0_strict_seed_zn_roles_c_resume_assignment", nodes)?,
                ),
                ("deferred_scope", edge(&deferred)),
            ],
            nodes,
        )?),
        nodes,
    )?;
    out.objects.push(
        emit(
            producer,
            ctx,
            crate::tb02e::KINDS[6],
            s("TB-02-E", nodes)?,
            &data,
            &all_sources,
            nodes,
        )?,
        nodes,
    )?;
    // F: consume this exact facts instance; no second scan and no reconstructed center geometry.
    let mut edges = Vec::new();
    for (w, tested) in facts.tested_positions(&units).iter().enumerate() {
        let center = &center_ids[w];
        let wf = &windows[w];
        let core = facts.window_center(&units, w);
        let seed = &inputs.completed[wf.first..wf.first + 3];
        for t in tested {
            let unit = &inputs.completed[t.unit];
            let entity = Text::format(
                format_args!("ucp:{}:{}", center_anchor(center), anchor(&unit.unit_id)),
                nodes,
            )?;
            let value = obj(
                [
                    ("entity_id", s(&entity, nodes)?),
                    ("center_id", s(center, nodes)?),
                    ("unit", unit_value(unit, nodes)?),
                    (
                        "core",
                        obj(
                            [("ZD", s(core.zd, nodes)?), ("ZG", s(core.zg, nodes)?)],
                            nodes,
                        )?,
                    ),
                    ("position", s(t.position.as_str(), nodes)?),
                    ("closed_overlap", CV::Bool(t.position.closed_overlap())),
                    ("strict_leave", CV::Bool(t.position.leave_side().is_some())),
                    (
                        "leave_side",
                        t.position
                            .leave_side()
                            .map(|d| s(direction(d), nodes))
                            .transpose()?
                            .unwrap_or(CV::Null),
                    ),
                    ("construction_role", s(t.role.as_str(), nodes)?),
                    ("destruction", s(crate::tb02f::DESTRUCTION, nodes)?),
                ],
                nodes,
            )?;
            let mut fields = vec_fields(value)?;
            let prior = old(crate::tb02f::KINDS[0], &entity);
            let data = match prior {
                Some(p) if same_fields(nodes, p, &fields, &["center_revision", "known_at"])? => {
                    p.clone()
                }
                _ => {
                    fields.push(
                        (
                            Text::new("center_revision", nodes)?,
                            s(revisions[w], nodes)?,
                        ),
                        nodes,
                    )?;
                    fields.push((Text::new("known_at", nodes)?, edge(ctx.known_at)), nodes)?;
                    put(&(CV::Object(fields)), nodes)?
                }
            };
            let version = field_text(nodes, &data, "center_revision")?;
            let sources = unit_sources(seed.iter().chain(std::iter::once(unit)), nodes)?;
            let object = emit(
                producer,
                ctx,
                crate::tb02f::KINDS[0],
                s(&entity, nodes)?,
                &data,
                &sources,
                nodes,
            )?;
            let oid = Text::new(&object.object_id, nodes)?;
            classification_request(
                producer,
                &mut out.requests,
                &object,
                "CC-018",
                "unit_core_position",
                s("position", nodes)?,
                complete_axes(),
                &[],
                &sources,
                nodes,
            )?;
            out.objects.push(object, nodes)?;
            edges.push(
                (
                    entity.clone_in(nodes)?,
                    "position_of",
                    unit.unit_id.clone_in(nodes)?,
                    version.clone_in(nodes)?,
                    oid.clone_in(nodes)?,
                    unit.sources.clone_in(nodes)?,
                ),
                nodes,
            )?;
            edges.push(
                (
                    entity,
                    "position_against",
                    center.clone_in(nodes)?,
                    version,
                    oid,
                    sources,
                ),
                nodes,
            )?;
        }
    }
    let mut pair_ids = Vec::new();
    let mut pending = 0;
    for pair in facts.pair_facts(&units) {
        let (a, b) = (&center_ids[pair.left], &center_ids[pair.right]);
        let entity = Text::format(
            format_args!("pair:{}:{}", center_anchor(a), center_anchor(b)),
            nodes,
        )?;
        let side = |w: usize| -> Result<CV, String> {
            let center = facts.window_center(&units, w);
            let outer = !windows[w].upgrade_pending;
            obj(
                [
                    ("ZD", s(center.zd, nodes)?),
                    ("ZG", s(center.zg, nodes)?),
                    (
                        "DD",
                        if outer {
                            s(center.dd, nodes)?
                        } else {
                            CV::Null
                        },
                    ),
                    (
                        "GG",
                        if outer {
                            s(center.gg, nodes)?
                        } else {
                            CV::Null
                        },
                    ),
                ],
                nodes,
            )
        };
        let (status, leaf, matrix, witnesses, null_reason) = match pair.outcome {
            PairOutcome::Evaluated(f) => {
                let mut matrix = Vec::new();
                for x in PairEndpoint::ALL {
                    for y in PairEndpoint::ALL {
                        matrix.push(
                            (
                                Text::format(
                                    format_args!("B.{}?A.{}", x.as_str(), y.as_str()),
                                    nodes,
                                )?,
                                s(
                                    match f.cmp(x, y) {
                                        std::cmp::Ordering::Less => "<",
                                        std::cmp::Ordering::Equal => "=",
                                        std::cmp::Ordering::Greater => ">",
                                    },
                                    nodes,
                                )?,
                            ),
                            nodes,
                        )?;
                    }
                }
                (
                    "EVALUATED",
                    Some(f.leaf),
                    CV::Object(matrix),
                    obj(
                        [
                            ("core_tangent", CV::Bool(f.core_tangent())),
                            ("outer_touch", CV::Bool(f.outer_touch())),
                            (
                                "outer_containment",
                                s(f.outer_containment().as_str(), nodes)?,
                            ),
                            ("core_containment", s(f.core_containment().as_str(), nodes)?),
                        ],
                        nodes,
                    )?,
                    CV::Null,
                )
            }
            PairOutcome::PendingUpgrade => {
                pending += 1;
                (
                    crate::tb02f::PENDING,
                    None,
                    CV::Null,
                    CV::Null,
                    s(crate::tb02f::PENDING, nodes)?,
                )
            }
        };
        let value = obj(
            [
                ("entity_id", s(&entity, nodes)?),
                ("a_center_id", s(a, nodes)?),
                ("a_revision", s(revisions[pair.left], nodes)?),
                ("b_center_id", s(b, nodes)?),
                ("b_revision", s(revisions[pair.right], nodes)?),
                (
                    "connector_units",
                    CV::Array(Vec::try_from_iter(
                        pair.connectors.clone().map(|k| s(ids[k], nodes)),
                        nodes,
                    )?),
                ),
                ("status", s(status, nodes)?),
                (
                    "leaf",
                    leaf.map(|l| s(l.as_str(), nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                (
                    "direction",
                    leaf.and_then(PairLeaf::direction)
                        .map(|d| s(direction(d), nodes))
                        .transpose()?
                        .unwrap_or(CV::Null),
                ),
                ("provisional", CV::Bool(!windows[pair.right].settled)),
                (
                    "inputs",
                    obj([("A", side(pair.left)?), ("B", side(pair.right)?)], nodes)?,
                ),
                ("matrix", matrix),
                ("witnesses", witnesses),
                (
                    "realization",
                    obj(
                        [
                            ("upper_center", s(crate::tb02f::REALIZATION_UPPER, nodes)?),
                            ("trend", s(crate::tb02f::REALIZATION_TREND, nodes)?),
                        ],
                        nodes,
                    )?,
                ),
                ("null_reason", null_reason),
            ],
            nodes,
        )?;
        let mut fields = vec_fields(value)?;
        let prior = old(crate::tb02f::KINDS[1], &entity);
        let (data, change) = match prior {
            Some(p)
                if same_fields(
                    nodes,
                    p,
                    &fields,
                    &["entity_revision", "previous", "formed_known_at", "known_at"],
                )? =>
            {
                (p.clone(), None)
            }
            _ => {
                let revision = next_revision(nodes, prior)?;
                let previous = match prior {
                    Some(p) => obj(
                        [
                            (
                                "entity_revision",
                                edge(&field(nodes, p, "entity_revision")?),
                            ),
                            ("status", edge(&field(nodes, p, "status")?)),
                            ("leaf", edge(&field(nodes, p, "leaf")?)),
                        ],
                        nodes,
                    )?,
                    None => CV::Null,
                };
                let formed = match prior {
                    Some(p) => field(nodes, p, "formed_known_at")?,
                    None => ctx.known_at.clone(),
                };
                fields.extend(
                    [
                        (Text::new("entity_revision", nodes)?, s(revision, nodes)?),
                        (Text::new("previous", nodes)?, previous),
                        (Text::new("formed_known_at", nodes)?, edge(&formed)),
                        (Text::new("known_at", nodes)?, edge(ctx.known_at)),
                    ],
                    nodes,
                )?;
                let leaf_value = leaf
                    .map(|l| s(l.as_str(), nodes))
                    .transpose()?
                    .unwrap_or(CV::Null);
                let change = match prior {
                    None => "formed",
                    Some(p)
                        if field_text(nodes, p, "status")? != status
                            && status == crate::tb02f::PENDING =>
                    {
                        "pending_upgrade"
                    }
                    Some(p) if !root_eq(p, "leaf", leaf_value, nodes)? => "leaf_revised",
                    _ => "inputs_revised",
                };
                (put(&(CV::Object(fields)), nodes)?, Some(change))
            }
        };
        let sources = unit_sources(
            &inputs.completed[windows[pair.left].first..=windows[pair.right].last],
            nodes,
        )?;
        let object = emit(
            producer,
            ctx,
            crate::tb02f::KINDS[1],
            s(&entity, nodes)?,
            &data,
            &sources,
            nodes,
        )?;
        let oid = Text::new(&object.object_id, nodes)?;
        // ≥9 的 pending 缺有效外缘，分类域尚未证成；上级 realization 尚未构造不影响已求值的 pair。
        classification_request(
            producer,
            &mut out.requests,
            &object,
            "CC-020",
            "center_pair_development",
            s("leaf", nodes)?,
            if leaf.is_some() {
                complete_axes()
            } else {
                missing_axes()
            },
            if leaf.is_some() {
                &[]
            } else {
                &[crate::tb02f::PENDING]
            },
            &sources,
            nodes,
        )?;
        out.objects.push(object, nodes)?;
        let version = field_text(nodes, &data, "entity_revision")?;
        for (relation, target) in [("pair_from", a), ("pair_to", b)] {
            edges.push(
                (
                    entity.clone_in(nodes)?,
                    relation,
                    target.clone_in(nodes)?,
                    version.clone_in(nodes)?,
                    oid.clone_in(nodes)?,
                    sources.clone_in(nodes)?,
                ),
                nodes,
            )?;
        }
        if let Some(change) = change {
            out.objects.push(
                change_object(
                    producer,
                    nodes,
                    ctx,
                    crate::tb02f::KINDS[3],
                    &entity,
                    change,
                    prior,
                    Some(&data),
                    Some(&oid),
                    &sources,
                )?,
                nodes,
            )?;
        }
        pair_ids.push(entity, nodes)?;
    }
    for ((kind, entity), p) in &previous {
        if kind == crate::tb02f::KINDS[1] && !pair_ids.contains(entity) {
            out.objects.push(
                change_object(
                    producer,
                    nodes,
                    ctx,
                    crate::tb02f::KINDS[3],
                    entity,
                    "withdrawn",
                    Some(p),
                    None,
                    None,
                    &all_sources,
                )?,
                nodes,
            )?;
        }
    }
    emit_edges(
        producer,
        nodes,
        ctx,
        crate::tb02f::KINDS[2],
        edges,
        &mut out,
    )?;
    let pair_waiting: Vec<&str> = if pending > 0 {
        vec_in![nodes;crate::tb02f::PENDING]
    } else {
        Vec::new()
    };
    let waiting = fact(nodes, &pair_waiting)?;
    let deferred = fact(nodes, &crate::tb02f::DEFERRED_SCOPE)?;
    let data = put(
        &(obj(
            [
                ("input_frontier", s(ctx.frontier, nodes)?),
                ("known_at", edge(ctx.known_at)),
                ("pair_count", s(pair_ids.len(), nodes)?),
                ("pending_pairs", s(pending, nodes)?),
                ("waiting_reasons", edge(&waiting)),
                (
                    "delivered_scope",
                    s(
                        "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position",
                        nodes,
                    )?,
                ),
                ("deferred_scope", edge(&deferred)),
            ],
            nodes,
        )?),
        nodes,
    )?;
    out.objects.push(
        emit(
            producer,
            ctx,
            crate::tb02f::KINDS[4],
            s("TB-02-F", nodes)?,
            &data,
            &all_sources,
            nodes,
        )?,
        nodes,
    )?;
    out.waiting_reasons.extend(pair_waiting, nodes)?;
    out.waiting_reasons.sort_unstable();
    out.waiting_reasons.dedup();
    Ok(out)
}

// 与 donor WireSerializer 相同的整数文本合同；缓冲区均在当前父账先准入。
struct WireSerializer<'a>(&'a dyn sc::NodeSource);
struct WireSeq<'a> {
    values: Vec<CV>,
    nodes: &'a dyn sc::NodeSource,
}
struct WireMap<'a> {
    fields: Vec<(Text, CV)>,
    key: Option<Text>,
    nodes: &'a dyn sc::NodeSource,
}
struct WireVariant<'a> {
    name: Text,
    fields: WireMap<'a>,
}
struct WireTupleVariant<'a> {
    name: Text,
    fields: WireSeq<'a>,
}
#[derive(Debug)]
struct SerError(String);
impl std::fmt::Display for SerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SerError {}
impl serde::ser::Error for SerError {
    fn custom<T: std::fmt::Display>(v: T) -> Self {
        Self(v.to_string())
    }
}
impl From<String> for SerError {
    fn from(v: String) -> Self {
        Self(v)
    }
}
macro_rules! wire_number {($($method:ident:$ty:ty),*)=>{$(fn $method(self,v:$ty)->Result<CV,SerError>{Ok(s(v,self.0)?)})*};}
impl<'a> serde::Serializer for WireSerializer<'a> {
    type Ok = CV;
    type Error = SerError;
    type SerializeSeq = WireSeq<'a>;
    type SerializeTuple = WireSeq<'a>;
    type SerializeTupleStruct = WireSeq<'a>;
    type SerializeTupleVariant = WireTupleVariant<'a>;
    type SerializeMap = WireMap<'a>;
    type SerializeStruct = WireMap<'a>;
    type SerializeStructVariant = WireVariant<'a>;
    wire_number!(serialize_i8:i8,serialize_i16:i16,serialize_i32:i32,serialize_i64:i64,serialize_i128:i128,serialize_u8:u8,serialize_u16:u16,serialize_u32:u32,serialize_u64:u64,serialize_u128:u128);
    fn serialize_f32(self, _: f32) -> Result<CV, SerError> {
        Err(SerError("段事实不允许浮点数".into()))
    }
    fn serialize_f64(self, _: f64) -> Result<CV, SerError> {
        Err(SerError("段事实不允许浮点数".into()))
    }
    fn serialize_bool(self, v: bool) -> Result<CV, SerError> {
        Ok(CV::Bool(v))
    }
    fn serialize_char(self, v: char) -> Result<CV, SerError> {
        Ok(s(v, self.0)?)
    }
    fn serialize_str(self, v: &str) -> Result<CV, SerError> {
        Ok(s(v, self.0)?)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<CV, SerError> {
        Ok(CV::Array(Vec::try_from_iter(
            v.iter().map(|v| s(v, self.0)),
            self.0,
        )?))
    }
    fn serialize_none(self) -> Result<CV, SerError> {
        Ok(CV::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<CV, SerError> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<CV, SerError> {
        Ok(CV::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<CV, SerError> {
        Ok(CV::Null)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        v: &'static str,
    ) -> Result<CV, SerError> {
        Ok(s(v, self.0)?)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<CV, SerError> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        v: &T,
    ) -> Result<CV, SerError> {
        let n = self.0;
        Ok(obj([(k, v.serialize(self)?)], n)?)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<WireSeq<'a>, SerError> {
        Ok(WireSeq {
            values: Vec::new(),
            nodes: self.0,
        })
    }
    fn serialize_tuple(self, n: usize) -> Result<WireSeq<'a>, SerError> {
        self.serialize_seq(Some(n))
    }
    fn serialize_tuple_struct(self, _: &'static str, n: usize) -> Result<WireSeq<'a>, SerError> {
        self.serialize_seq(Some(n))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        _: usize,
    ) -> Result<WireTupleVariant<'a>, SerError> {
        Ok(WireTupleVariant {
            name: Text::new(k, self.0)?,
            fields: WireSeq {
                values: Vec::new(),
                nodes: self.0,
            },
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<WireMap<'a>, SerError> {
        Ok(WireMap {
            fields: Vec::new(),
            key: None,
            nodes: self.0,
        })
    }
    fn serialize_struct(self, _: &'static str, n: usize) -> Result<WireMap<'a>, SerError> {
        self.serialize_map(Some(n))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        _: usize,
    ) -> Result<WireVariant<'a>, SerError> {
        Ok(WireVariant {
            name: Text::new(k, self.0)?,
            fields: WireMap {
                fields: Vec::new(),
                key: None,
                nodes: self.0,
            },
        })
    }
}
impl serde::ser::SerializeSeq for WireSeq<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        self.values
            .push(v.serialize(WireSerializer(self.nodes))?, self.nodes)?;
        Ok(())
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Array(self.values))
    }
}
impl serde::ser::SerializeTuple for WireSeq<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<CV, SerError> {
        serde::ser::SerializeSeq::end(self)
    }
}
impl serde::ser::SerializeTupleStruct for WireSeq<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<CV, SerError> {
        serde::ser::SerializeSeq::end(self)
    }
}
impl serde::ser::SerializeMap for WireMap<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        let CV::String(k) = v.serialize(WireSerializer(self.nodes))? else {
            return Err(SerError("非文本对象键".into()));
        };
        self.key = Some(k);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        let k = self.key.take().ok_or_else(|| SerError("缺对象键".into()))?;
        self.fields
            .push((k, v.serialize(WireSerializer(self.nodes))?), self.nodes)?;
        Ok(())
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Object(self.fields))
    }
}
impl serde::ser::SerializeStruct for WireMap<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        k: &'static str,
        v: &T,
    ) -> Result<(), SerError> {
        self.fields.push(
            (
                Text::new(k, self.nodes)?,
                v.serialize(WireSerializer(self.nodes))?,
            ),
            self.nodes,
        )?;
        Ok(())
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Object(self.fields))
    }
}
impl serde::ser::SerializeTupleVariant for WireTupleVariant<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(&mut self.fields, v)
    }
    fn end(self) -> Result<CV, SerError> {
        let n = self.fields.nodes;
        Ok(CV::Object(Vec::from_iter(
            [(self.name, CV::Array(self.fields.values))],
            n,
        )?))
    }
}
impl serde::ser::SerializeStructVariant for WireVariant<'_> {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        k: &'static str,
        v: &T,
    ) -> Result<(), SerError> {
        serde::ser::SerializeStruct::serialize_field(&mut self.fields, k, v)
    }
    fn end(self) -> Result<CV, SerError> {
        let n = self.fields.nodes;
        Ok(CV::Object(Vec::from_iter(
            [(self.name, CV::Object(self.fields.fields))],
            n,
        )?))
    }
}

#[cfg(test)]
mod native_wire_tests {
    use super::*;
    #[test]
    fn integer_text_bool_null_and_array_order_share_codec_kernel() {
        let mut nodes = sc::MemoryNodes::new();
        let value = serde_json::json!({"z":[2,-3,null,true],"a":"x"});
        let actual = fact(&mut nodes, &value).unwrap();
        let expected = sc::write_value(
            &mut nodes,
            &sc::Value::Object(vec![
                ("a".into(), sc::Value::String("x".into())),
                (
                    "z".into(),
                    sc::Value::Array(vec![
                        sc::Value::String("2".into()),
                        sc::Value::String("-3".into()),
                        sc::Value::Null,
                        sc::Value::Bool(true),
                    ]),
                ),
            ]),
        )
        .unwrap();
        assert_eq!(actual, expected);
        let before = nodes.iter().count();
        assert!(fact(&mut nodes, &1.25f64)
            .unwrap_err()
            .contains("不允许浮点"));
        assert_eq!(nodes.iter().count(), before, "编码拒绝不能先写节点");
    }
    #[test]
    fn k1_borrowed_view_matches_existing_wire_after_integer_normalization() {
        use newchan_rust::theta_v0::parser::second_fold::SecondFoldCursor;
        let c = SecondFoldCursor::new(
            SecondCandidateId {
                seg_dir: Direction::Up,
                b_stroke: 2,
                candidate_extreme: 99,
            },
            3,
        );
        let mut nodes = sc::MemoryNodes::new();
        assert_eq!(
            fact(&mut nodes, &c.wire_view()).unwrap(),
            fact(&mut nodes, &c.to_wire()).unwrap()
        );
    }
}
