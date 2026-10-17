//! #1419/#1420：同次段事实直接构造 C/D 内容边；E/F 与生成态接线另行验收。
//! 不调用旧对象 JSON builder，不重算段判据；obj3 身份只调用共享工厂。
use crate::content_producer::{
    Cc004Producer, RequestApplicability, RequestAxes, RequestComputation, RequestInputQuality,
    RequestValidity, TypedObject,
};
use newchan_rust::s_content::{self as sc, Built, ContentRef, NodeStore, Value as CV};
use newchan_rust::theta_v0::{
    parser::{
        feature_seq::{SecondTerminal, SECOND_SEQ_SCAN_BUDGET_POLICY},
        inclusion::CoordsRef,
        second_fold::{SecondCandidateId, SecondFoldStore},
        segment::{ConstructionFact, SegmentFacts, CONSTRUCTION_RULE, TAIL_WINDOW},
        stroke::StrokeFacts,
    },
    types::{Direction, Segment},
};
use serde::Serialize;
#[cfg(test)]
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

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
    pub entity_id: String,
    pub object_id: String,
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

fn s(value: impl ToString) -> CV {
    CV::String(value.to_string())
}
fn obj<const N: usize>(fields: [(&str, CV); N]) -> CV {
    CV::Object(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}
fn edge(root: &Built) -> CV {
    CV::Edge(root.digest)
}
fn built(reference: &ContentRef) -> Result<Built, String> {
    Ok(Built {
        digest: reference.digest().map_err(|e| e.to_string())?,
        kind: reference.kind,
        extent: reference.extent,
    })
}
fn put(nodes: &mut dyn NodeStore, value: CV) -> Result<Built, String> {
    sc::write_value(nodes, &value).map_err(|e| e.to_string())
}
fn field(nodes: &mut dyn NodeStore, root: &Built, name: &str) -> Result<Built, String> {
    built(
        &sc::path::lookup_path(
            nodes,
            &root.content_ref(),
            &[sc::path::PathStep::Field(name.to_owned())],
        )
        .map_err(|e| e.to_string())?,
    )
}
fn text(nodes: &mut dyn NodeStore, root: &Built) -> Result<String, String> {
    if root.kind != sc::Kind::String {
        return Err("结构字段不是文本".into());
    }
    let mut bytes = Vec::new();
    sc::render_canonical(nodes, &root.content_ref(), &mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| format!("结构文本: {e}"))
}
fn field_text(nodes: &mut dyn NodeStore, root: &Built, name: &str) -> Result<String, String> {
    let value = field(nodes, root, name)?;
    text(nodes, &value)
}
fn canonical_key(nodes: &mut dyn NodeStore, value: CV) -> Result<String, String> {
    let root = put(nodes, value)?;
    let mut bytes = Vec::new();
    sc::render_canonical(nodes, &root.content_ref(), &mut bytes).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn sorted_sources(mut sources: Vec<usize>) -> Vec<usize> {
    sources.sort_unstable();
    sources.dedup();
    sources
}
fn coord_tree(sources: &[usize]) -> CoordsRef {
    sources
        .iter()
        .fold(CoordsRef::empty(), |coords, &n| coords.append_greater(n))
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
    nodes: &mut dyn NodeStore,
    requests: &mut Vec<Built>,
    subject: &TypedObject,
    cc: &str,
    axis: &str,
    path: CV,
    axes: RequestAxes,
    reasons: &[&str],
    sources: &[usize],
) -> Result<(), String> {
    // 主体 obj3 已绑定完整事实和源版本；固定请求版本与观察路径隔离同对象内的多次判定。
    // 与 source_pending 的规范二元槽位协议相同：对象 slot + 具名请求 slot。
    let kind = field_text(nodes, &subject.record, "kind")?;
    let slot = field(nodes, &subject.payload, "slot")?;
    let key = CV::Array(vec![
        CV::Array(vec![s(kind), edge(&slot)]),
        CV::Array(vec![
            s(cc),
            s(axis),
            s("structure-classification/1"),
            path,
            s(&subject.object_id),
        ]),
    ]);
    requests.push(
        producer
            .explicit_request(
                nodes,
                key,
                &subject.object_id,
                axes,
                reasons,
                &coord_tree(sources),
            )
            .map_err(|e| e.to_string())?,
    );
    Ok(())
}
fn construction_requests(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    requests: &mut Vec<Built>,
    subject: &TypedObject,
    path: CV,
    c: &ConstructionFact,
    sources: &[usize],
) -> Result<(), String> {
    classification_request(
        producer,
        nodes,
        requests,
        subject,
        "CC-011",
        "length",
        path.clone(),
        complete_axes(),
        &[],
        sources,
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
        nodes,
        requests,
        subject,
        "CC-011",
        "construction_conditions",
        path.clone(),
        axes,
        reasons,
        sources,
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
        nodes,
        requests,
        subject,
        "CC-011",
        "direction",
        path,
        axes,
        reasons,
        sources,
    )
}

/// 从已核事实直接序列化为内容值，整数遵从 s-segment/3 的十进制文本规则。
/// 不经 serde_json::Value、JSON 文本或旧 typed object；每份事实根只构造一次并跨对象接边。
fn fact<T: Serialize + ?Sized>(nodes: &mut dyn NodeStore, value: &T) -> Result<Built, String> {
    put(
        nodes,
        value.serialize(WireSerializer).map_err(|e| e.to_string())?,
    )
}

struct PriorSegment {
    data: Built,
    revision: i64,
    result: String,
}
fn verify_prior(
    nodes: &mut dyn NodeStore,
    producer: &Cc004Producer,
    record: &Built,
    kind: &str,
) -> Result<Built, String> {
    sc::verify(nodes, &record.content_ref()).map_err(|e| e.to_string())?;
    if field_text(nodes, record, "kind")? != kind {
        return Err("prior kind 不符".into());
    }
    let key = field_text(nodes, record, "fact_key")?;
    let payload = field(nodes, record, "payload")?;
    let refs = field(nodes, record, "input_refs")?;
    if payload.kind != sc::Kind::Object || refs.kind != sc::Kind::Array {
        return Err("prior 内容边类型不符".into());
    }
    let derived = producer
        .typed_object_id(kind, &key, &payload, &refs)
        .map_err(|e| e.to_string())?;
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
    if canonical_key(nodes, CV::Array(vec![s(kind), edge(&slot)]))? != key {
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
) -> Result<BTreeMap<String, PriorSegment>, String> {
    if !prior.segment_records.is_empty() && prior.version_record.is_none() {
        return Err("prior 段记录缺本 cut 段版本根".into());
    }
    let mut out = BTreeMap::new();
    for record in prior.segment_records {
        let data = verify_prior(nodes, producer, record, crate::tb02c::KINDS[2])?;
        let entity = field_text(nodes, &data, "entity_id")?;
        let revision_text = field_text(nodes, &data, "entity_revision")?;
        let revision = revision_text.parse::<i64>().map_err(|e| e.to_string())?;
        if revision < 1 || revision.to_string() != revision_text {
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
            )
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
    let items =
        sc::path::lookup_array_items(nodes, &cursors.content_ref()).map_err(|e| e.to_string())?;
    for item in items {
        let mut bytes = Vec::new();
        sc::render_canonical(nodes, &item, &mut bytes).map_err(|e| e.to_string())?;
        let wire: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        fold.restore(&wire);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit(
    producer: &mut Cc004Producer,
    nodes: &mut dyn NodeStore,
    ctx: &StructureContext<'_>,
    kind: &str,
    slot: CV,
    data: &Built,
    sources: &[usize],
) -> Result<TypedObject, String> {
    let slot = put(nodes, slot)?;
    let key = canonical_key(nodes, CV::Array(vec![s(kind), edge(&slot)]))?;
    let payload = obj([
        ("schema_revision", s(header(kind).0)),
        ("semantic_version", s(header(kind).1)),
        ("policy_version", s(header(kind).2)),
        ("view_role", s("main")),
        ("object_generation", s(ctx.fact_generation)),
        ("slot", edge(&slot)),
        ("data", edge(data)),
    ]);
    let coords = coord_tree(sources);
    let input_refs = producer
        .source_refs_array(nodes, &coords)
        .map_err(|e| e.to_string())?;
    let source_coords = producer
        .coords_array(nodes, &coords)
        .map_err(|e| e.to_string())?;
    producer
        .build_object(
            nodes,
            kind,
            key,
            payload,
            input_refs,
            source_coords,
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
        sources.extend(&bi.start.source_coords);
        sources.extend(&bi.end.source_coords);
        let a = fact(nodes, &bi.start.extreme_roots)?;
        let b = fact(nodes, &bi.end.extreme_roots)?;
        let value = put(
            nodes,
            obj([
                ("stroke_index", s(i)),
                (
                    "entity_id",
                    s(format!("bi:{generation}:{}", bi.identity_anchor)),
                ),
                ("start_anchor", s(bi.start.group_anchor)),
                ("end_anchor", s(bi.end.group_anchor)),
                ("start_price", s(bi.start.price)),
                ("end_price", s(bi.end.price)),
                ("start_roots", edge(&a)),
                ("end_roots", edge(&b)),
            ]),
        )?;
        values.push(edge(&value));
    }
    Ok((put(nodes, CV::Array(values))?, sorted_sources(sources)))
}
fn second_status(state: Option<SecondTerminal>) -> &'static str {
    match state {
        Some(SecondTerminal::Confirmed) => "CONFIRMED",
        Some(SecondTerminal::KilledByNewExtreme) => "KILLED",
        _ => "PENDING",
    }
}
fn replace_value(fields: &mut [(String, CV)], key: &str, value: CV) -> Result<(), String> {
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
    sc::verify(nodes, &ctx.known_at.content_ref()).map_err(|e| e.to_string())?;
    if ctx.known_at.kind != sc::Kind::Object {
        return Err("known_at 必须为真实对象根".into());
    }
    let old = read_prior(nodes, producer, prior)?;
    let facts = evidence.segment_facts;
    let strokes = evidence.stroke_facts;
    let mut out = StructureOutput {
        requests: vec![],
        objects: vec![],
        relations: vec![],
        segments: vec![],
        waiting_reasons: facts.waiting_reasons.clone(),
    };
    let mut seeds: Vec<_> = facts.seeds.iter().collect();
    for c in &facts.candidates {
        if !seeds.iter().any(|s| s.start_stroke == c.seed.start_stroke) {
            seeds.push(&c.seed);
        }
    }
    for seed in seeds {
        let anchor = strokes
            .strokes
            .get(seed.start_stroke)
            .map(|s| s.identity_anchor.to_string())
            .unwrap_or_else(|| "empty".into());
        let entity = format!("seed:{}:{anchor}", ctx.fact_generation);
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
            nodes,
            obj([
                ("entity_id", s(&entity)),
                ("construction", edge(&construction)),
                ("stroke_refs", edge(&refs)),
            ]),
        )?;
        let kind = crate::tb02c::KINDS[if seed.vector == Some([true; 4]) { 0 } else { 6 }];
        let object = emit(producer, nodes, ctx, kind, s(entity), &data, &sources)?;
        construction_requests(
            producer,
            nodes,
            &mut out.requests,
            &object,
            s("construction"),
            seed,
            &sources,
        )?;
        out.objects.push(object);
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
        let entity = format!("segment:{}:{}", ctx.fact_generation, bi.identity_anchor);
        if !active.insert(entity.clone()) {
            return Err("重复本次段实体".into());
        }
        let sequence = format!("first:{entity}");
        let (refs, sources) = stroke_refs(
            nodes,
            strokes,
            ctx.fact_generation,
            c.start_stroke,
            c.observed_through,
        )?;
        let first = fact(nodes, &c.first)?;
        let scan = put(
            nodes,
            obj([
                ("from_stroke", s(c.start_stroke)),
                (
                    "through_stroke",
                    s(c.first
                        .raw_elements
                        .last()
                        .map_or(c.start_stroke, |e| e.stroke_idx)),
                ),
                (
                    "input_stroke_count",
                    s(c.first
                        .raw_elements
                        .last()
                        .map_or(c.start_stroke + 1, |e| e.stroke_idx + 1)),
                ),
                ("resource_truncated", CV::Bool(false)),
                ("tail_cache_window", s(TAIL_WINDOW)),
            ]),
        )?;
        let data = put(
            nodes,
            obj([
                ("entity_id", s(&sequence)),
                ("segment_id", s(&entity)),
                ("role", s("FIRST_SEQUENCE")),
                ("direction", s(dir)),
                ("member_direction", s(opposite)),
                ("facts", edge(&first)),
                ("stroke_refs", edge(&refs)),
                ("scan", edge(&scan)),
                (
                    "second_sequence_status",
                    s(c.second
                        .as_ref()
                        .map_or("NOT_APPLICABLE", |v| second_status(v.semantic_state))),
                ),
            ]),
        )?;
        let first_object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02c::KINDS[1],
            s(&sequence),
            &data,
            &sources,
        )?;
        let first_id = first_object.object_id.clone();
        out.objects.push(first_object);
        let second_sequence = c.second.as_ref().map(|_| format!("second:{entity}"));
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
            let cursor_root = fact(nodes, &cursor.to_wire())?;
            let second_root = fact(nodes, second)?;
            let semantic = fact(nodes, &second.semantic_state)?;
            let coverage = fact(nodes, &second.coverage)?;
            let budget = fact(nodes, &cursor.budget_granted)?;
            let scan = put(
                nodes,
                obj([
                    ("from_stroke", s(second.from_stroke)),
                    ("through_stroke", s(second.through_stroke)),
                    ("input_stroke_count", s(second.input_stroke_count)),
                    ("resource_truncated", CV::Bool(second.resource_truncated)),
                    ("tail_cache_window", s(0)),
                    ("semantic_state", edge(&semantic)),
                    ("coverage", edge(&coverage)),
                    (
                        "budget",
                        obj([
                            ("policy", s(SECOND_SEQ_SCAN_BUDGET_POLICY)),
                            ("granted", edge(&budget)),
                        ]),
                    ),
                    ("cursor", edge(&cursor_root)),
                ]),
            )?;
            let name = second_sequence.as_ref().ok_or("缺第二序列实体")?;
            let data = put(
                nodes,
                obj([
                    ("entity_id", s(name)),
                    ("segment_id", s(&entity)),
                    ("role", s("SECOND_SEQUENCE")),
                    ("direction", s(dir)),
                    ("member_direction", s(dir)),
                    ("facts", edge(&second_root)),
                    ("stroke_refs", edge(&refs)),
                    ("scan", edge(&scan)),
                    (
                        "second_sequence_status",
                        s(second_status(second.semantic_state)),
                    ),
                ]),
            )?;
            let object = emit(
                producer,
                nodes,
                ctx,
                crate::tb02c::KINDS[1],
                s(name),
                &data,
                &sources,
            )?;
            let id = object.object_id.clone();
            out.objects.push(object);
            Some(id)
        } else {
            None
        };
        let previous = old.get(&entity);
        let geometric = if let Some(i) = c.end_stroke {
            let end = &strokes.strokes.get(i).ok_or("候选段端越界")?.end;
            let roots = fact(nodes, &end.extreme_roots)?;
            put(
                nodes,
                obj([
                    ("stroke_index", s(i)),
                    ("group_anchor", s(end.group_anchor)),
                    ("extreme_roots", edge(&roots)),
                    ("price", s(end.price)),
                ]),
            )?
        } else {
            put(nodes, CV::Null)?
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
            put(nodes, CV::Null)?
        };
        let strict = fact(nodes, &c.strict)?;
        let vector = fact(nodes, &c.fact_vector)?;
        let construction = fact(nodes, &c.construction)?;
        let waiting = fact(nodes, &c.waiting_reasons)?;
        let attempts = fact(nodes, &c.termination_attempts)?;
        let trigger = fact(nodes, &c.trigger_stroke)?;
        let CV::Object(mut fields) = obj([
            ("entity_id", s(&entity)),
            ("entity_revision", s(1)),
            (
                "seed_id",
                s(format!(
                    "seed:{}:{}",
                    ctx.fact_generation, bi.identity_anchor
                )),
            ),
            ("sequence_id", s(&sequence)),
            ("sequence_object_id", s(&first_id)),
            (
                "second_sequence_id",
                second_sequence.as_ref().map_or(CV::Null, s),
            ),
            (
                "second_sequence_object_id",
                second_id.as_ref().map_or(CV::Null, s),
            ),
            ("strict", edge(&strict)),
            ("fact_vector", edge(&vector)),
            ("construction", edge(&construction)),
            ("result", s(c.result)),
            ("waiting_reasons", edge(&waiting)),
            ("termination_attempts", edge(&attempts)),
            ("geometric_end", edge(&geometric)),
            ("trigger_stroke", edge(&trigger)),
            ("formed_known_at", edge(&formed)),
            ("terminated_known_at", edge(&terminated)),
            ("stroke_refs", edge(&refs)),
        ]) else {
            unreachable!()
        };
        let normalized = put(nodes, CV::Object(fields.clone()))?;
        let changed = if let Some(p) = previous {
            let names: Vec<_> = fields.iter().map(|(k, _)| k.as_str()).collect();
            let prior_fields = sc::path::lookup_object_fields(nodes, &p.data.content_ref(), &names)
                .map_err(|e| e.to_string())?;
            let mut normalized_prior = Vec::new();
            for (key, reference) in prior_fields {
                normalized_prior.push((
                    key.clone(),
                    if key == "entity_revision" {
                        s(1)
                    } else {
                        edge(&built(&reference)?)
                    },
                ));
            }
            put(nodes, CV::Object(normalized_prior))?.digest != normalized.digest
        } else {
            true
        };
        let revision = previous
            .map_or(0, |p| p.revision)
            .checked_add(i64::from(changed))
            .ok_or("段修订溢出")?;
        replace_value(&mut fields, "entity_revision", s(revision))?;
        let data = put(nodes, CV::Object(fields))?;
        let object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02c::KINDS[2],
            s(&entity),
            &data,
            &sources,
        )?;
        let object_id = object.object_id.clone();
        construction_requests(
            producer,
            nodes,
            &mut out.requests,
            &object,
            s("construction"),
            &c.construction,
            &sources,
        )?;
        for (index, attempt) in c.termination_attempts.iter().enumerate() {
            construction_requests(
                producer,
                nodes,
                &mut out.requests,
                &object,
                CV::Array(vec![s("termination_attempts"), s(index), s("construction")]),
                &attempt.construction,
                &sources,
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
            nodes,
            &mut out.requests,
            &object,
            "CC-013",
            "semantic_termination_result",
            s("result"),
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
        )?;
        let facts_complete =
            c.fact_vector.new_segment_formed.is_some() && c.fact_vector.segment_broken.is_some();
        classification_request(
            producer,
            nodes,
            &mut out.requests,
            &object,
            "CC-014",
            "known_fact_vector",
            s("fact_vector"),
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
        )?;
        // 扫描范围与覆盖事实均由同次 parser 明确给出；覆盖不足也是已求出的覆盖分类。
        classification_request(
            producer,
            nodes,
            &mut out.requests,
            &object,
            "CC-014",
            "scan_coverage",
            s("fact_vector"),
            complete_axes(),
            &[],
            &sources,
        )?;
        out.objects.push(object);
        out.segments.push(SegmentObject {
            candidate_index,
            entity_id: entity.clone(),
            object_id: object_id.clone(),
            sources: sources.clone(),
        });
        let mut relations = vec![(
            sequence.clone(),
            "derived_from",
            entity.clone(),
            sources.clone(),
        )];
        if let Some(second) = &second_sequence {
            relations.push((
                second.clone(),
                "derived_from",
                entity.clone(),
                sources.clone(),
            ));
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
                let deps = sorted_sources(
                    member
                        .start
                        .source_coords
                        .iter()
                        .chain(&member.end.source_coords)
                        .copied()
                        .collect(),
                );
                relations.push((
                    format!("bi:{}:{}", ctx.fact_generation, member.identity_anchor),
                    "member_of",
                    seq.clone(),
                    deps,
                ));
            }
        }
        for (source, relation, target, deps) in relations {
            let data = put(
                nodes,
                obj([
                    ("source_id", s(&source)),
                    ("relation_kind", s(relation)),
                    ("target_id", s(&target)),
                    ("version", s(revision)),
                    ("witness_object_id", s(&object_id)),
                ]),
            )?;
            out.objects.push(emit(
                producer,
                nodes,
                ctx,
                crate::tb02c::KINDS[3],
                CV::Array(vec![s(&source), s(relation), s(&target)]),
                &data,
                &deps,
            )?);
            out.relations.push(put(
                nodes,
                obj([
                    ("subject", s(source)),
                    ("relation_type", s(relation)),
                    ("object", s(target)),
                ]),
            )?);
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
                nodes,
                obj([
                    ("entity_id", s(&entity)),
                    ("change", s(change)),
                    ("before", previous.map_or(CV::Null, |p| edge(&p.data))),
                    ("after", edge(&data)),
                    ("known_at", edge(ctx.known_at)),
                    ("object_id", s(&object_id)),
                ]),
            )?;
            out.objects.push(emit(
                producer,
                nodes,
                ctx,
                crate::tb02c::KINDS[4],
                CV::Array(vec![s(ctx.generation), s(&entity), s(change)]),
                &change_data,
                &sources,
            )?);
        }
        out.waiting_reasons.extend(&c.waiting_reasons);
    }
    let all_sources = sorted_sources(ctx.all_sources.to_vec());
    for (entity, prior) in &old {
        if !active.contains(entity) {
            let data = put(
                nodes,
                obj([
                    ("entity_id", s(entity)),
                    ("change", s("withdrawn")),
                    ("before", edge(&prior.data)),
                    ("after", CV::Null),
                    ("known_at", edge(ctx.known_at)),
                    ("object_id", CV::Null),
                ]),
            )?;
            out.objects.push(emit(
                producer,
                nodes,
                ctx,
                crate::tb02c::KINDS[4],
                CV::Array(vec![s(ctx.generation), s(entity), s("withdrawn")]),
                &data,
                &all_sources,
            )?);
        }
    }
    out.waiting_reasons.sort_unstable();
    out.waiting_reasons.dedup();
    let waiting = fact(nodes, &out.waiting_reasons)?;
    // 唯一公开 checkpoint 枚举口的权威 wire；选择/排序仍由 SecondFoldStore 决定。
    let checkpoint = fact(nodes, &evidence.second_store.checkpoint())?;
    let data = put(
        nodes,
        obj([
            ("input_frontier", s(ctx.frontier)),
            ("waiting_reasons", edge(&waiting)),
            ("known_at", edge(ctx.known_at)),
            (
                "delivered_scope",
                s("seed_strict_first_and_second_terminal"),
            ),
            (
                "deferred_scope",
                CV::Array(vec![s("general_equal_identity"), s("global_G006_bridge")]),
            ),
            (
                "scan_budget",
                obj([
                    ("policy", s(SECOND_SEQ_SCAN_BUDGET_POLICY)),
                    ("round_strokes", s(evidence.second_store.round_budget)),
                ]),
            ),
            ("scan_cursors", edge(&checkpoint)),
        ]),
    )?;
    out.objects.push(emit(
        producer,
        nodes,
        ctx,
        crate::tb02c::KINDS[5],
        s("TB-02-W"),
        &data,
        &all_sources,
    )?);
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
fn unit_value(u: &crate::tb02e::SegUnit) -> CV {
    match u.range {
        Some(r) => obj([
            ("unit_id", s(&u.unit_id)),
            ("unit_object_id", s(&u.object_id)),
            ("direction", s(direction(r.direction))),
            ("lo", s(r.lo)),
            ("hi", s(r.hi)),
        ]),
        None => obj([
            ("unit_id", s(&u.unit_id)),
            ("unit_object_id", s(&u.object_id)),
            ("completed", CV::Bool(false)),
        ]),
    }
}
fn unit_sources<'a>(units: impl IntoIterator<Item = &'a crate::tb02e::SegUnit>) -> Vec<usize> {
    sorted_sources(
        units
            .into_iter()
            .flat_map(|u| u.sources.iter().copied())
            .collect(),
    )
}
fn vec_fields(value: CV) -> Result<Vec<(String, CV)>, String> {
    match value {
        CV::Object(v) => Ok(v),
        _ => Err("内部字段非对象".into()),
    }
}
fn root_eq(nodes: &mut dyn NodeStore, old: &Built, key: &str, new: CV) -> Result<bool, String> {
    Ok(field(nodes, old, key)?.digest == put(nodes, new)?.digest)
}
fn revision_of(nodes: &mut dyn NodeStore, old: Option<&Built>) -> Result<i64, String> {
    match old {
        None => Ok(0),
        Some(old) => {
            let value = field_text(nodes, old, "entity_revision")?;
            let n = value.parse::<i64>().map_err(|e| e.to_string())?;
            if n < 1 || n.to_string() != value {
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
    fresh: &[(String, CV)],
    bookkeeping: &[&str],
) -> Result<bool, String> {
    let mut names: Vec<&str> = fresh.iter().map(|(k, _)| k.as_str()).collect();
    for k in bookkeeping {
        if !names.contains(k) {
            names.push(k);
        }
    }
    let fields = sc::path::lookup_object_fields(nodes, &old.content_ref(), &names)
        .map_err(|e| e.to_string())?;
    for (key, value) in fresh {
        if !bookkeeping.contains(&key.as_str()) {
            let old = fields.get(key).ok_or("旧对象缺事实字段")?;
            if old.digest().map_err(|e| e.to_string())? != put(nodes, value.clone())?.digest {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn keep_known_node(
    nodes: &mut dyn NodeStore,
    old: Option<&Built>,
    value: CV,
) -> Result<Built, String> {
    let fields = vec_fields(value)?;
    if let Some(old) = old {
        if same_fields(nodes, old, &fields, &["known_at"])? {
            return Ok(old.clone());
        }
    }
    put(nodes, CV::Object(fields))
}
fn revise_node(
    nodes: &mut dyn NodeStore,
    old: Option<&Built>,
    value: CV,
) -> Result<(Built, i64, bool), String> {
    let mut fields = vec_fields(value)?;
    let changed = match old {
        Some(old) => !same_fields(nodes, old, &fields, &["entity_revision"])?,
        None => true,
    };
    let revision = revision_of(nodes, old)?
        .checked_add(i64::from(changed))
        .ok_or("修订溢出")?;
    replace_value(&mut fields, "entity_revision", s(revision))?;
    Ok((put(nodes, CV::Object(fields))?, revision, changed))
}
type StructureEdge = (String, &'static str, String, String, String, Vec<usize>);
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
        nodes,
        obj([
            ("entity_id", s(entity)),
            ("change", s(change)),
            ("before", before.map_or(CV::Null, edge)),
            ("after", after.map_or(CV::Null, edge)),
            ("known_at", edge(ctx.known_at)),
            ("object_id", object_id.map_or(CV::Null, s)),
        ]),
    )?;
    emit(
        producer,
        nodes,
        ctx,
        kind,
        CV::Array(vec![s(ctx.generation), s(entity), s(change)]),
        &data,
        sources,
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
            nodes,
            obj([
                ("source_id", s(&source)),
                ("relation_kind", s(relation)),
                ("target_id", s(&target)),
                ("version", s(version)),
                ("witness_object_id", s(witness)),
            ]),
        )?;
        out.objects.push(emit(
            producer,
            nodes,
            ctx,
            kind,
            CV::Array(vec![s(&source), s(relation), s(&target)]),
            &data,
            &sources,
        )?);
        out.relations.push(put(
            nodes,
            obj([
                ("subject", s(source)),
                ("relation_type", s(relation)),
                ("object", s(target)),
            ]),
        )?);
    }
    Ok(())
}
/// 同次 C/D 对象映射 → 已提取的共享输入合同；不按数组位置合成对象身份。
fn segment_units(
    evidence: &StructureEvidence<'_>,
    segments: &[SegmentObject],
) -> Result<crate::tb02e::UnitInputs, String> {
    use crate::tb02e::{CompletedSegmentUnit, SegmentStrokePrices, SegmentUnitRow};
    if segments.len() != evidence.segment_facts.candidates.len() {
        return Err("C/D 段映射未完整覆盖同次候选".into());
    }
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    let index = |i: usize| i64::try_from(i).map_err(|_| "笔下标溢出".to_string());
    for published in segments {
        if !seen.insert(published.candidate_index) {
            return Err("重复段候选映射".into());
        }
        let c = evidence
            .segment_facts
            .candidates
            .get(published.candidate_index)
            .ok_or("段映射候选越界")?;
        let strokes = &evidence.stroke_facts.strokes;
        let start = strokes.get(c.start_stroke).ok_or("段起笔越界")?;
        let completed = if matches!(c.result, "CASE_ONE" | "CASE_TWO") {
            let end = c.end_stroke.ok_or("完成段缺端笔")?;
            let refs = strokes
                .iter()
                .enumerate()
                .take(c.observed_through.checked_add(1).ok_or("观察下标溢出")?)
                .skip(c.start_stroke)
                .map(|(i, b)| {
                    Ok(SegmentStrokePrices {
                        stroke_index: index(i)?,
                        start_price: b.start.price,
                        end_price: b.end.price,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Some(CompletedSegmentUnit {
                end_stroke: index(end)?,
                start_price: start.start.price,
                end_price: strokes.get(end).ok_or("段端越界")?.end.price,
                member_stroke_indices: c
                    .construction
                    .stroke_indices
                    .iter()
                    .map(|&i| index(i))
                    .collect::<Result<Vec<_>, _>>()?,
                stroke_refs: refs,
            })
        } else {
            None
        };
        rows.push(SegmentUnitRow {
            unit_id: published.entity_id.clone(),
            object_id: published.object_id.clone(),
            start_stroke: index(c.start_stroke)?,
            direction: c.construction.direction.unwrap_or("").to_owned(),
            sources: published.sources.clone(),
            completed,
        });
    }
    crate::tb02e::units_from_segment_rows(rows, evidence.segments)
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
    let inputs = segment_units(evidence, cd_segments)?;
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
    let units: Vec<_> = inputs
        .completed
        .iter()
        .map(|u| u.range.ok_or_else(|| "完成单元缺区间".to_owned()))
        .collect::<Result<_, _>>()?;
    let facts = scan_center_facts_full(&units, center_from_segments);
    if facts
        .attempts
        .iter()
        .any(|a| a.facts.exclusion(SeedRule::Complete) == Some(SeedExclusion::NotAlternating))
    {
        return Err("TB02E_INPUT_CONTRACT：完成单元方向未交替".into());
    }
    let windows = facts.window_facts(&units);
    let ids: Vec<_> = inputs
        .completed
        .iter()
        .map(|u| u.unit_id.as_str())
        .collect();
    let center_ids: Vec<_> = windows
        .iter()
        .map(|w| format!("center:{}", anchor(ids[w.first])))
        .collect();
    let mut previous = BTreeMap::<(String, String), Built>::new();
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
            if !prior_versions.insert(kind) {
                return Err("EF prior 重复版本根".into());
            }
            continue;
        }
        let entity = field_text(nodes, &data, "entity_id")?;
        if previous.insert((kind, entity), data).is_some() {
            return Err("EF prior 重复实体".into());
        }
    }
    if !prior.records.is_empty()
        && (!prior_versions.contains(crate::tb02e::KINDS[6])
            || !prior_versions.contains(crate::tb02f::KINDS[4]))
    {
        return Err("EF prior 缺中心或中枢对版本根".into());
    }
    let old = |kind: &str, entity: &str| previous.get(&(kind.to_owned(), entity.to_owned()));
    let mut out = StructureOutput {
        requests: vec![],
        objects: vec![],
        relations: vec![],
        segments: vec![],
        waiting_reasons: vec![],
    };
    let mut edges: Vec<StructureEdge> = vec![];
    let mut attempt_ids = BTreeMap::new();
    for a in &facts.attempts {
        let seed = &inputs.completed[a.at..a.at + 3];
        let seed_units = &units[a.at..a.at + 3];
        let roles = CenterRoles::of_members(seed_units);
        let extremum = |var: RoleVar| {
            let contributors: Vec<_> = roles
                .contributors(seed_units, var)
                .into_iter()
                .map(|k| s(ids[a.at + k]))
                .collect();
            let witness = if contributors.len() >= 2 {
                contributors.clone()
            } else {
                vec![]
            };
            obj([
                ("value", s(roles.value(var))),
                ("contributors", CV::Array(contributors)),
                ("same_price_witness", CV::Array(witness)),
            ])
        };
        let v = a.facts.condition_vector(SeedInput::TOWER);
        let entity = format!("seed_attempt:{}", anchor(ids[a.at]));
        let center = facts
            .windows
            .iter()
            .position(|w| w.seed_at == a.at)
            .map(|w| s(&center_ids[w]))
            .unwrap_or(CV::Null);
        let value = obj([
            ("entity_id", s(&entity)),
            ("units", CV::Array(seed.iter().map(unit_value).collect())),
            (
                "vector",
                obj([
                    ("completed", CV::Bool(v.completed)),
                    ("consecutive", CV::Bool(v.consecutive)),
                    ("same_level", CV::Bool(v.same_level)),
                    ("alternating", CV::Bool(v.alternating)),
                    ("strict", v.strict.map_or(CV::Null, CV::Bool)),
                ]),
            ),
            ("ZD", extremum(RoleVar::Zd)),
            ("ZG", extremum(RoleVar::Zg)),
            ("core_order", s(a.facts.core_order.as_str())),
            ("result", s(if a.formed { "FORMED" } else { "EXCLUDED" })),
            (
                "exclusion_reason",
                a.facts
                    .exclusion(SeedRule::Complete)
                    .map_or(CV::Null, |r| s(r.as_str())),
            ),
            ("center_id", center),
            ("known_at", edge(ctx.known_at)),
        ]);
        let data = keep_known_node(nodes, old(crate::tb02e::KINDS[0], &entity), value)?;
        let sources = unit_sources(seed);
        let object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02e::KINDS[0],
            s(&entity),
            &data,
            &sources,
        )?;
        classification_request(
            producer,
            nodes,
            &mut out.requests,
            &object,
            "CC-016",
            "seed_condition_vector",
            s("vector"),
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
        )?;
        classification_request(
            producer,
            nodes,
            &mut out.requests,
            &object,
            "CC-016",
            "core_order",
            s("core_order"),
            complete_axes(),
            &[],
            &sources,
        )?;
        edges.push((
            entity.clone(),
            "attempted_from",
            ids[a.at].to_owned(),
            "1".into(),
            object.object_id.clone(),
            sources,
        ));
        out.objects.push(object);
        attempt_ids.insert(a.at, entity);
    }
    let mut revisions = vec![];
    for (w, wf) in windows.iter().enumerate() {
        let entity = &center_ids[w];
        let members = &inputs.completed[wf.first..=wf.last];
        let member_ids: Vec<_> = (wf.first..=wf.last).map(|k| s(ids[k])).collect();
        let roles_entity = format!("roles:{entity}");
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
            .map_or(CV::Null, |u| s(&u.unit_id));
        let break_unit = wf.break_unit.map_or(CV::Null, |k| s(ids[k]));
        let prior = old(crate::tb02e::KINDS[1], entity);
        let settled = if wf.settled {
            match prior {
                Some(p)
                    if root_eq(nodes, p, "settled", CV::Bool(true))?
                        && root_eq(nodes, p, "break_unit", break_unit.clone())? =>
                {
                    field(nodes, p, "settled_known_at")?
                }
                _ => ctx.known_at.clone(),
            }
        } else {
            put(nodes, CV::Null)?
        };
        let formed = match prior {
            Some(p) => field(nodes, p, "formed_known_at")?,
            None => ctx.known_at.clone(),
        };
        let value = obj([
            ("entity_id", s(entity)),
            ("entity_revision", s(1)),
            (
                "seed_attempt_id",
                s(attempt_ids.get(&wf.first).ok_or("中心缺真实seed attempt")?),
            ),
            ("seed_units", CV::Array(member_ids[..3].to_vec())),
            ("zn_direction", s(direction(units[wf.first].direction))),
            ("members", CV::Array(member_ids.clone())),
            (
                "zn_members",
                CV::Array(wf.zn_members.iter().map(|&k| s(ids[k])).collect()),
            ),
            ("status", s(status)),
            ("settled", CV::Bool(wf.settled)),
            ("break_unit", break_unit),
            ("break_up", CV::Bool(wf.break_up.unwrap_or(false))),
            ("break_up_defined", CV::Bool(wf.break_up.is_some())),
            (
                "resume_anchor",
                if wf.settled {
                    obj([("after_unit_id", s(ids[wf.last])), ("offset", s(2))])
                } else {
                    CV::Null
                },
            ),
            ("next_tested_unit", next),
            ("roles_id", s(&roles_entity)),
            ("formed_known_at", edge(&formed)),
            ("settled_known_at", edge(&settled)),
        ]);
        let (data, revision, changed) = revise_node(nodes, prior, value)?;
        let mut sources = unit_sources(members);
        if let Some(k) = wf.break_unit {
            sources.extend(&inputs.completed[k].sources);
        }
        let sources = sorted_sources(sources);
        let object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02e::KINDS[1],
            s(entity),
            &data,
            &sources,
        )?;
        let oid = object.object_id.clone();
        out.objects.push(object);
        let role = |var: RoleVar| {
            let r = wf.role(var);
            let hidden = wf.upgrade_pending && !var.frozen();
            let contributors: Vec<_> = if hidden {
                vec![]
            } else {
                r.contributors.iter().map(|&k| s(ids[k])).collect()
            };
            let witness = if contributors.len() >= 2 {
                contributors.clone()
            } else {
                vec![]
            };
            obj([
                ("value", if hidden { CV::Null } else { s(r.value) }),
                ("contributors", CV::Array(contributors)),
                ("same_price_witness", CV::Array(witness)),
                ("frozen", CV::Bool(var.frozen())),
                (
                    "null_reason",
                    if hidden {
                        s("UPGRADE_PENDING_TB02H")
                    } else {
                        CV::Null
                    },
                ),
            ])
        };
        let roles = put(
            nodes,
            obj([
                (
                    "CORE",
                    obj([("ZD", role(RoleVar::Zd)), ("ZG", role(RoleVar::Zg))]),
                ),
                (
                    "OUTER",
                    obj([("DD", role(RoleVar::Dd)), ("GG", role(RoleVar::Gg))]),
                ),
                (
                    "INNER_EXTREMA",
                    obj([("D", role(RoleVar::D)), ("G", role(RoleVar::G))]),
                ),
            ]),
        )?;
        let old_roles = old(crate::tb02e::KINDS[2], &roles_entity);
        let roles_data = match old_roles {
            Some(p)
                if root_eq(nodes, p, "roles", edge(&roles))?
                    && root_eq(nodes, p, "center_id", s(entity))? =>
            {
                p.clone()
            }
            _ => {
                let revision = next_revision(nodes, old_roles)?;
                let prev = match old_roles {
                    Some(p) => obj([
                        (
                            "entity_revision",
                            edge(&field(nodes, p, "entity_revision")?),
                        ),
                        ("roles", edge(&field(nodes, p, "roles")?)),
                    ]),
                    None => CV::Null,
                };
                put(
                    nodes,
                    obj([
                        ("entity_id", s(&roles_entity)),
                        ("entity_revision", s(revision)),
                        ("center_id", s(entity)),
                        ("roles", edge(&roles)),
                        ("previous", prev),
                        ("known_at", edge(ctx.known_at)),
                    ]),
                )?
            }
        };
        let roles_changed = old_roles.is_some_and(|p| p.digest != roles_data.digest);
        let roles_object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02e::KINDS[2],
            s(&roles_entity),
            &roles_data,
            &sources,
        )?;
        let roles_oid = roles_object.object_id.clone();
        out.objects.push(roles_object);
        if !wf.upgrade_pending {
            for k in wf.first..=wf.last {
                edges.push((
                    ids[k].to_owned(),
                    "member_of",
                    entity.clone(),
                    revision.to_string(),
                    oid.clone(),
                    inputs.completed[k].sources.clone(),
                ));
            }
        }
        let mut changes = vec![];
        match prior {
            None => changes.push("formed"),
            Some(p) if changed => {
                let count = field(nodes, p, "members")?
                    .extent
                    .ok_or("旧members缺长度")?;
                if count < member_ids.len() as u64 {
                    changes.push("member_added");
                }
                if !root_eq(nodes, p, "settled", CV::Bool(true))? && wf.settled {
                    changes.push("settled");
                }
                if field_text(nodes, p, "status")? != "UPGRADE_PENDING_TB02H" && wf.upgrade_pending
                {
                    changes.push("upgrade_pending");
                }
            }
            _ => {}
        }
        for change in changes {
            out.objects.push(change_object(
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
            )?);
        }
        if roles_changed {
            out.objects.push(change_object(
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
            )?);
        }
        revisions.push(revision);
    }
    let all_sources = sorted_sources(ctx.all_sources.to_vec());
    for ((kind, entity), p) in &previous {
        if kind == crate::tb02e::KINDS[1] && !center_ids.contains(entity) {
            out.objects.push(change_object(
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
            )?);
        }
    }
    let unit_facts = facts.unit_facts();
    for (unit, f) in unit_facts
        .iter()
        .enumerate()
        .map(|(k, f)| (&inputs.completed[k], Some(*f)))
        .chain(inputs.in_progress.iter().map(|u| (u, None)))
    {
        let entity = format!("assign:{}", unit.unit_id);
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
            out.waiting_reasons.push(reason);
        }
        let value = obj([
            ("entity_id", s(&entity)),
            ("unit", unit_value(unit)),
            ("status", s(status)),
            ("center_id", center.map_or(CV::Null, |w| s(&center_ids[w]))),
            (
                "left_center_id",
                left.map_or(CV::Null, |w| s(&center_ids[w])),
            ),
            (
                "right_center_id",
                right.map_or(CV::Null, |w| s(&center_ids[w])),
            ),
            ("right_pending", CV::Bool(pending)),
            ("reason", reason.map_or(CV::Null, s)),
            ("seed_from_self", s(seed.as_str())),
            ("known_at", edge(ctx.known_at)),
        ]);
        let data = keep_known_node(nodes, old(crate::tb02e::KINDS[3], &entity), value)?;
        let object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02e::KINDS[3],
            s(&entity),
            &data,
            &unit.sources,
        )?;
        let oid = object.object_id.clone();
        // 全读法扫描同时给出 member/connector/not-yet；未完成单元的已知未归属状态不是 false 或漏算。
        classification_request(
            producer,
            nodes,
            &mut out.requests,
            &object,
            "CC-022",
            "assignment_status",
            s("status"),
            complete_axes(),
            &[],
            &unit.sources,
        )?;
        out.objects.push(object);
        for (relation, w) in [("connects_left", left), ("connects_right", right)] {
            if let Some(w) = w {
                edges.push((
                    unit.unit_id.clone(),
                    relation,
                    center_ids[w].clone(),
                    revisions[w].to_string(),
                    oid.clone(),
                    unit.sources.clone(),
                ));
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
    let awaiting = |k: usize| match id_at(k) {
        Some(id) => (s(id), CV::Null, CV::Null),
        None => (
            CV::Null,
            len.checked_sub(1)
                .map_or(CV::Null, |i| s(&inputs.completed[i].unit_id)),
            s(k as i128 - (len as i128 - 1)),
        ),
    };
    let cursor = match facts.scan_state() {
        ScanState::Extending { center, next_unit } => {
            let (unit, after, offset) = awaiting(next_unit);
            obj([
                ("state", s("EXTENDING")),
                ("center_id", s(&center_ids[center])),
                ("seed_start_unit_id", CV::Null),
                ("awaiting_unit_id", unit),
                ("awaiting_after_unit_id", after),
                ("awaiting_offset", offset),
            ])
        }
        ScanState::SeekingSeed {
            seed_start,
            awaiting_unit,
        } => {
            let (unit, after, offset) = awaiting(awaiting_unit);
            obj([
                ("state", s("SEEKING_SEED")),
                ("center_id", CV::Null),
                ("seed_start_unit_id", id_at(seed_start).map_or(CV::Null, s)),
                ("awaiting_unit_id", unit),
                ("awaiting_after_unit_id", after),
                ("awaiting_offset", offset),
            ])
        }
    };
    let waiting = fact(nodes, &out.waiting_reasons)?;
    let deferred = fact(nodes, &crate::tb02e::DEFERRED_SCOPE)?;
    let data = put(
        nodes,
        obj([
            ("input_frontier", s(ctx.frontier)),
            ("known_at", edge(ctx.known_at)),
            ("scan_cursor", cursor),
            ("waiting_reasons", edge(&waiting)),
            (
                "delivered_scope",
                s("l0_strict_seed_zn_roles_c_resume_assignment"),
            ),
            ("deferred_scope", edge(&deferred)),
        ]),
    )?;
    out.objects.push(emit(
        producer,
        nodes,
        ctx,
        crate::tb02e::KINDS[6],
        s("TB-02-E"),
        &data,
        &all_sources,
    )?);
    // F: consume this exact facts instance; no second scan and no reconstructed center geometry.
    let mut edges = vec![];
    for (w, tested) in facts.tested_positions(&units).iter().enumerate() {
        let center = &center_ids[w];
        let wf = &windows[w];
        let core = facts.window_center(&units, w);
        let seed = &inputs.completed[wf.first..wf.first + 3];
        for t in tested {
            let unit = &inputs.completed[t.unit];
            let entity = format!("ucp:{}:{}", center_anchor(center), anchor(&unit.unit_id));
            let value = obj([
                ("entity_id", s(&entity)),
                ("center_id", s(center)),
                ("unit", unit_value(unit)),
                ("core", obj([("ZD", s(core.zd)), ("ZG", s(core.zg))])),
                ("position", s(t.position.as_str())),
                ("closed_overlap", CV::Bool(t.position.closed_overlap())),
                ("strict_leave", CV::Bool(t.position.leave_side().is_some())),
                (
                    "leave_side",
                    t.position
                        .leave_side()
                        .map_or(CV::Null, |d| s(direction(d))),
                ),
                ("construction_role", s(t.role.as_str())),
                ("destruction", s(crate::tb02f::DESTRUCTION)),
            ]);
            let mut fields = vec_fields(value)?;
            let prior = old(crate::tb02f::KINDS[0], &entity);
            let data = match prior {
                Some(p) if same_fields(nodes, p, &fields, &["center_revision", "known_at"])? => {
                    p.clone()
                }
                _ => {
                    fields.push(("center_revision".into(), s(revisions[w])));
                    fields.push(("known_at".into(), edge(ctx.known_at)));
                    put(nodes, CV::Object(fields))?
                }
            };
            let version = field_text(nodes, &data, "center_revision")?;
            let sources = unit_sources(seed.iter().chain(std::iter::once(unit)));
            let object = emit(
                producer,
                nodes,
                ctx,
                crate::tb02f::KINDS[0],
                s(&entity),
                &data,
                &sources,
            )?;
            let oid = object.object_id.clone();
            classification_request(
                producer,
                nodes,
                &mut out.requests,
                &object,
                "CC-018",
                "unit_core_position",
                s("position"),
                complete_axes(),
                &[],
                &sources,
            )?;
            out.objects.push(object);
            edges.push((
                entity.clone(),
                "position_of",
                unit.unit_id.clone(),
                version.clone(),
                oid.clone(),
                unit.sources.clone(),
            ));
            edges.push((
                entity,
                "position_against",
                center.clone(),
                version,
                oid,
                sources,
            ));
        }
    }
    let mut pair_ids = vec![];
    let mut pending = 0;
    for pair in facts.pair_facts(&units) {
        let (a, b) = (&center_ids[pair.left], &center_ids[pair.right]);
        let entity = format!("pair:{}:{}", center_anchor(a), center_anchor(b));
        let side = |w: usize| {
            let center = facts.window_center(&units, w);
            let outer = !windows[w].upgrade_pending;
            obj([
                ("ZD", s(center.zd)),
                ("ZG", s(center.zg)),
                ("DD", if outer { s(center.dd) } else { CV::Null }),
                ("GG", if outer { s(center.gg) } else { CV::Null }),
            ])
        };
        let (status, leaf, matrix, witnesses, null_reason) = match pair.outcome {
            PairOutcome::Evaluated(f) => {
                let mut matrix = vec![];
                for x in PairEndpoint::ALL {
                    for y in PairEndpoint::ALL {
                        matrix.push((
                            format!("B.{}?A.{}", x.as_str(), y.as_str()),
                            s(match f.cmp(x, y) {
                                std::cmp::Ordering::Less => "<",
                                std::cmp::Ordering::Equal => "=",
                                std::cmp::Ordering::Greater => ">",
                            }),
                        ));
                    }
                }
                (
                    "EVALUATED",
                    Some(f.leaf),
                    CV::Object(matrix),
                    obj([
                        ("core_tangent", CV::Bool(f.core_tangent())),
                        ("outer_touch", CV::Bool(f.outer_touch())),
                        ("outer_containment", s(f.outer_containment().as_str())),
                        ("core_containment", s(f.core_containment().as_str())),
                    ]),
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
                    s(crate::tb02f::PENDING),
                )
            }
        };
        let value = obj([
            ("entity_id", s(&entity)),
            ("a_center_id", s(a)),
            ("a_revision", s(revisions[pair.left])),
            ("b_center_id", s(b)),
            ("b_revision", s(revisions[pair.right])),
            (
                "connector_units",
                CV::Array(pair.connectors.clone().map(|k| s(ids[k])).collect()),
            ),
            ("status", s(status)),
            ("leaf", leaf.map_or(CV::Null, |l| s(l.as_str()))),
            (
                "direction",
                leaf.and_then(PairLeaf::direction)
                    .map_or(CV::Null, |d| s(direction(d))),
            ),
            ("provisional", CV::Bool(!windows[pair.right].settled)),
            (
                "inputs",
                obj([("A", side(pair.left)), ("B", side(pair.right))]),
            ),
            ("matrix", matrix),
            ("witnesses", witnesses),
            (
                "realization",
                obj([
                    ("upper_center", s(crate::tb02f::REALIZATION_UPPER)),
                    ("trend", s(crate::tb02f::REALIZATION_TREND)),
                ]),
            ),
            ("null_reason", null_reason),
        ]);
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
                    Some(p) => obj([
                        (
                            "entity_revision",
                            edge(&field(nodes, p, "entity_revision")?),
                        ),
                        ("status", edge(&field(nodes, p, "status")?)),
                        ("leaf", edge(&field(nodes, p, "leaf")?)),
                    ]),
                    None => CV::Null,
                };
                let formed = match prior {
                    Some(p) => field(nodes, p, "formed_known_at")?,
                    None => ctx.known_at.clone(),
                };
                fields.extend([
                    ("entity_revision".into(), s(revision)),
                    ("previous".into(), previous),
                    ("formed_known_at".into(), edge(&formed)),
                    ("known_at".into(), edge(ctx.known_at)),
                ]);
                let leaf_value = leaf.map_or(CV::Null, |l| s(l.as_str()));
                let change = match prior {
                    None => "formed",
                    Some(p)
                        if field_text(nodes, p, "status")? != status
                            && status == crate::tb02f::PENDING =>
                    {
                        "pending_upgrade"
                    }
                    Some(p) if !root_eq(nodes, p, "leaf", leaf_value)? => "leaf_revised",
                    _ => "inputs_revised",
                };
                (put(nodes, CV::Object(fields))?, Some(change))
            }
        };
        let sources =
            unit_sources(&inputs.completed[windows[pair.left].first..=windows[pair.right].last]);
        let object = emit(
            producer,
            nodes,
            ctx,
            crate::tb02f::KINDS[1],
            s(&entity),
            &data,
            &sources,
        )?;
        let oid = object.object_id.clone();
        // ≥9 的 pending 缺有效外缘，分类域尚未证成；上级 realization 尚未构造不影响已求值的 pair。
        classification_request(
            producer,
            nodes,
            &mut out.requests,
            &object,
            "CC-020",
            "center_pair_development",
            s("leaf"),
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
        )?;
        out.objects.push(object);
        let version = field_text(nodes, &data, "entity_revision")?;
        for (relation, target) in [("pair_from", a), ("pair_to", b)] {
            edges.push((
                entity.clone(),
                relation,
                target.clone(),
                version.clone(),
                oid.clone(),
                sources.clone(),
            ));
        }
        if let Some(change) = change {
            out.objects.push(change_object(
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
            )?);
        }
        pair_ids.push(entity);
    }
    for ((kind, entity), p) in &previous {
        if kind == crate::tb02f::KINDS[1] && !pair_ids.contains(entity) {
            out.objects.push(change_object(
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
            )?);
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
        vec![crate::tb02f::PENDING]
    } else {
        vec![]
    };
    let waiting = fact(nodes, &pair_waiting)?;
    let deferred = fact(nodes, &crate::tb02f::DEFERRED_SCOPE)?;
    let data = put(
        nodes,
        obj([
            ("input_frontier", s(ctx.frontier)),
            ("known_at", edge(ctx.known_at)),
            ("pair_count", s(pair_ids.len())),
            ("pending_pairs", s(pending)),
            ("waiting_reasons", edge(&waiting)),
            (
                "delivered_scope",
                s("l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position"),
            ),
            ("deferred_scope", edge(&deferred)),
        ]),
    )?;
    out.objects.push(emit(
        producer,
        nodes,
        ctx,
        crate::tb02f::KINDS[4],
        s("TB-02-F"),
        &data,
        &all_sources,
    )?);
    out.waiting_reasons.extend(pair_waiting);
    out.waiting_reasons.sort_unstable();
    out.waiting_reasons.dedup();
    Ok(out)
}

// 直接 Serde → 内容值适配；仅数字线形不同，其余遵从事实类型自身 Serialize 实现。
struct WireSerializer;
struct WireSeq(Vec<CV>);
struct WireMap {
    fields: Vec<(String, CV)>,
    key: Option<String>,
}
struct WireVariant {
    name: String,
    fields: WireMap,
}
struct WireTupleVariant {
    name: String,
    fields: WireSeq,
}
type SerError = serde_json::Error;
macro_rules! wire_number {($($method:ident:$ty:ty),*)=>{$(fn $method(self,value:$ty)->Result<CV,SerError>{Ok(s(value))})*};}
impl serde::Serializer for WireSerializer {
    type Ok = CV;
    type Error = SerError;
    type SerializeSeq = WireSeq;
    type SerializeTuple = WireSeq;
    type SerializeTupleStruct = WireSeq;
    type SerializeTupleVariant = WireTupleVariant;
    type SerializeMap = WireMap;
    type SerializeStruct = WireMap;
    type SerializeStructVariant = WireVariant;
    wire_number!(serialize_i8:i8,serialize_i16:i16,serialize_i32:i32,serialize_i64:i64,serialize_i128:i128,serialize_u8:u8,serialize_u16:u16,serialize_u32:u32,serialize_u64:u64,serialize_u128:u128);
    fn serialize_f32(self, _: f32) -> Result<CV, SerError> {
        Err(serde::ser::Error::custom("段事实不允许浮点数"))
    }
    fn serialize_f64(self, _: f64) -> Result<CV, SerError> {
        Err(serde::ser::Error::custom("段事实不允许浮点数"))
    }
    fn serialize_bool(self, v: bool) -> Result<CV, SerError> {
        Ok(CV::Bool(v))
    }
    fn serialize_char(self, v: char) -> Result<CV, SerError> {
        Ok(s(v))
    }
    fn serialize_str(self, v: &str) -> Result<CV, SerError> {
        Ok(s(v))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<CV, SerError> {
        Ok(CV::Array(v.iter().map(s).collect()))
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
        Ok(s(v))
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
        Ok(obj([(k, v.serialize(self)?)]))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<WireSeq, SerError> {
        Ok(WireSeq(Vec::new()))
    }
    fn serialize_tuple(self, _: usize) -> Result<WireSeq, SerError> {
        Ok(WireSeq(Vec::new()))
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<WireSeq, SerError> {
        Ok(WireSeq(Vec::new()))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        _: usize,
    ) -> Result<WireTupleVariant, SerError> {
        Ok(WireTupleVariant {
            name: k.into(),
            fields: WireSeq(Vec::new()),
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<WireMap, SerError> {
        Ok(WireMap {
            fields: Vec::new(),
            key: None,
        })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<WireMap, SerError> {
        Ok(WireMap {
            fields: Vec::new(),
            key: None,
        })
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        k: &'static str,
        _: usize,
    ) -> Result<WireVariant, SerError> {
        Ok(WireVariant {
            name: k.into(),
            fields: WireMap {
                fields: Vec::new(),
                key: None,
            },
        })
    }
}
impl serde::ser::SerializeSeq for WireSeq {
    type Ok = CV;
    type Error = SerError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        self.0.push(v.serialize(WireSerializer)?);
        Ok(())
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Array(self.0))
    }
}
impl serde::ser::SerializeTuple for WireSeq {
    type Ok = CV;
    type Error = SerError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<CV, SerError> {
        serde::ser::SerializeSeq::end(self)
    }
}
impl serde::ser::SerializeTupleStruct for WireSeq {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<CV, SerError> {
        serde::ser::SerializeSeq::end(self)
    }
}
impl serde::ser::SerializeMap for WireMap {
    type Ok = CV;
    type Error = SerError;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), SerError> {
        let CV::String(k) = k.serialize(WireSerializer)? else {
            return Err(serde::ser::Error::custom("非文本对象键"));
        };
        self.key = Some(k);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        let k = self
            .key
            .take()
            .ok_or_else(|| <SerError as serde::ser::Error>::custom("缺对象键"))?;
        self.fields.push((k, v.serialize(WireSerializer)?));
        Ok(())
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Object(self.fields))
    }
}
impl serde::ser::SerializeStruct for WireMap {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        k: &'static str,
        v: &T,
    ) -> Result<(), SerError> {
        self.fields.push((k.into(), v.serialize(WireSerializer)?));
        Ok(())
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Object(self.fields))
    }
}
impl serde::ser::SerializeTupleVariant for WireTupleVariant {
    type Ok = CV;
    type Error = SerError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), SerError> {
        serde::ser::SerializeSeq::serialize_element(&mut self.fields, v)
    }
    fn end(self) -> Result<CV, SerError> {
        Ok(CV::Object(vec![(self.name, CV::Array(self.fields.0))]))
    }
}
impl serde::ser::SerializeStructVariant for WireVariant {
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
        Ok(CV::Object(vec![(
            self.name,
            CV::Object(self.fields.fields),
        )]))
    }
}

#[cfg(test)]
fn parse_json_field(o: &Value, key: &str) -> Result<Value, String> {
    let text = o[key].as_str().ok_or_else(|| format!("{key} missing"))?;
    serde_json::from_str(text).map_err(|e| format!("parse {key}: {e}"))
}

#[cfg(test)]
fn decode_prior(store: &mut dyn NodeStore, prior: &[Built]) -> Result<Vec<Value>, String> {
    prior
        .iter()
        .map(|built| decode_object(store, built))
        .collect()
}

#[cfg(test)]
fn decode_object(store: &mut dyn NodeStore, built: &Built) -> Result<Value, String> {
    let mut bytes = Vec::new();
    sc::render_canonical(store, &built.content_ref(), &mut bytes)
        .map_err(|e| format!("decode: {e}"))?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| format!("decode JSON: {e}"))?;
    if !value.is_object() {
        return Err("decode: expected object record".into());
    }
    Ok(value)
}

#[cfg(test)]
fn json_to_cvalue(v: &Value) -> Result<sc::Value, String> {
    match v {
        Value::Null => Ok(sc::Value::Null),
        Value::Bool(b) => Ok(sc::Value::Bool(*b)),
        Value::Number(n) => sc::NumberToken::parse(&n.to_string())
            .map(sc::Value::Number)
            .map_err(|e| format!("number: {e}")),
        Value::String(s) => Ok(sc::Value::String(s.clone())),
        Value::Array(a) => {
            let mut out = Vec::new();
            for item in a {
                out.push(json_to_cvalue(item)?);
            }
            Ok(sc::Value::Array(out))
        }
        Value::Object(m) => {
            let mut out = Vec::new();
            for (k, val) in m {
                out.push((k.clone(), json_to_cvalue(val)?));
            }
            Ok(sc::Value::Object(out))
        }
    }
}

#[cfg(test)]
fn json_to_built(store: &mut dyn NodeStore, v: &Value) -> Result<Built, String> {
    let cvalue = json_to_cvalue(v)?;
    sc::write_value(store, &cvalue).map_err(|e| format!("write relation: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_bridge_preserves_json_types_and_number_domain() {
        let value = serde_json::json!({
            "bools": [true, false], "null": null,
            "signed": i64::MIN, "unsigned": u64::MAX,
            "fraction": -1.25, "exponent": 1.25e100,
            "nested": [{"text": "quoted \" text"}]
        });
        let mut store = sc::MemoryNodes::new();
        let built = json_to_built(&mut store, &value).unwrap();
        assert_eq!(decode_object(&mut store, &built).unwrap(), value);
    }

    #[test]
    fn prior_decode_propagates_missing_node_after_valid_record() {
        let mut store = sc::MemoryNodes::new();
        let valid = json_to_built(&mut store, &serde_json::json!({"ok": true})).unwrap();
        let mut foreign_store = sc::MemoryNodes::new();
        let missing =
            json_to_built(&mut foreign_store, &serde_json::json!({"missing": true})).unwrap();
        let expected = decode_object(&mut store, &missing).unwrap_err();
        assert_eq!(
            decode_prior(&mut store, &[valid, missing]).unwrap_err(),
            expected
        );
    }

    #[test]
    fn object_decode_rejects_real_nonobject_and_invalid_payload() {
        let mut store = sc::MemoryNodes::new();
        let scalar = json_to_built(&mut store, &serde_json::json!(true)).unwrap();
        assert_eq!(
            decode_object(&mut store, &scalar).unwrap_err(),
            "decode: expected object record"
        );
        assert!(
            parse_json_field(&serde_json::json!({"payload_json": "{"}), "payload_json").is_err()
        );
    }

    fn fixture_bars(rows: &[Value]) -> Vec<newchan_rust::theta_v0::types::Bar> {
        rows.iter()
            .enumerate()
            .map(|(i, r)| newchan_rust::theta_v0::types::Bar {
                source_index: i,
                timestamp: i as i64,
                open: r["open"].as_i64().unwrap(),
                high: r["high"].as_i64().unwrap(),
                low: r["low"].as_i64().unwrap(),
                close: r["close"].as_i64().unwrap(),
                volume: 1.0,
                untradable: false,
            })
            .collect()
    }
    fn fixture_raw(rows: &[Value]) -> BTreeMap<i64, Value> {
        rows.iter().enumerate().map(|(i,_)|(i as i64,serde_json::json!({"identity_key":format!("raw-{i}"),"payload_hash":"h","receipt_id":format!("receipt-{i}"),"event_id":format!("bar-{i}"),"input_revision":"1","revision":1,"seq":i,"source_coord":i.to_string(),"received_at":"2000-01-01T00:00:00Z"}))).collect()
    }
    fn bind_fixture(
        producer: &mut Cc004Producer,
        nodes: &mut dyn NodeStore,
        raw: &BTreeMap<i64, Value>,
    ) {
        for (&i, r) in raw {
            producer
                .bind_source(
                    nodes,
                    i,
                    crate::content_producer::RawRefSpec {
                        identity_key: r["identity_key"].as_str().unwrap().into(),
                        payload_hash: r["payload_hash"].as_str().unwrap().into(),
                        receipt_id: r["receipt_id"].as_str().unwrap().into(),
                        event_id: r["event_id"].as_str().unwrap().into(),
                        input_revision: "1".into(),
                        revision: 1,
                        seq: i,
                        source_coord: i.to_string(),
                    },
                )
                .unwrap();
        }
    }
    fn normalize_ids(v: &mut Value, ids: &BTreeMap<String, String>) {
        match v {
            Value::String(s) => {
                if let Some(key) = ids.get(s) {
                    *s = key.clone();
                }
            }
            Value::Array(a) => {
                for v in a {
                    normalize_ids(v, ids)
                }
            }
            Value::Object(o) => {
                for v in o.values_mut() {
                    normalize_ids(v, ids)
                }
            }
            _ => {}
        }
    }
    fn records(nodes: &mut dyn NodeStore, objects: &[TypedObject]) -> Vec<Value> {
        objects
            .iter()
            .map(|o| decode_object(nodes, &o.record).unwrap())
            .collect()
    }

    /// 跨生产者/消费者合同：exact 五字段、二元规范键、真实主体、来源及具名轴。
    fn check_requests(nodes: &mut dyn NodeStore, output: &StructureOutput) -> Vec<Value> {
        let objects: BTreeMap<_, _> = output
            .objects
            .iter()
            .map(|o| (o.object_id.as_str(), o))
            .collect();
        let mut seen = BTreeSet::new();
        let mut requests = vec![];
        for root in &output.requests {
            let r = decode_object(nodes, root).unwrap();
            assert_eq!(
                r.as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([
                    "request_id",
                    "subject_id",
                    "axes",
                    "reasons",
                    "source_coords"
                ])
            );
            let key_text = r["request_id"].as_str().unwrap();
            assert!(seen.insert(key_text.to_owned()), "同 cut 重复请求");
            let key: Value = serde_json::from_str(key_text).unwrap();
            assert_eq!(
                key.as_array().unwrap().len(),
                2,
                "source_pending 二元键合同"
            );
            let subject = r["subject_id"].as_str().unwrap();
            let object = objects.get(subject).expect("请求主体必须是本次真实obj3");
            assert!(subject.starts_with("obj3-"));
            assert_eq!(
                key[0],
                serde_json::from_str::<Value>(&object.fact_key).unwrap()
            );
            assert_eq!(key[1][2], "structure-classification/1");
            assert_eq!(key[1][4], subject);
            assert_eq!(
                r["axes"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from(["input_quality", "applicability", "computation", "validity"])
            );
            assert_eq!(r["axes"]["validity"], "CURRENT");
            let mut source_bytes = vec![];
            sc::render_canonical(
                nodes,
                &object.source_coords.content_ref(),
                &mut source_bytes,
            )
            .unwrap();
            assert_eq!(
                r["source_coords"],
                serde_json::from_slice::<Value>(&source_bytes).unwrap()
            );
            assert!(object.request.is_none(), "唯一请求集合不可重复装入对象");
            requests.push(r);
        }
        requests
    }

    #[test]
    fn construction_requests_distinguish_false_outside_domain_and_missing_evidence() {
        let mut nodes = sc::MemoryNodes::new();
        let mut producer = Cc004Producer::new(crate::tb02a::PROFILE, crate::RULE_REVISION);
        let raw = fixture_raw(&[serde_json::json!({})]);
        bind_fixture(&mut producer, &mut nodes, &raw);
        let known=json_to_built(&mut nodes,&serde_json::json!({"generation":"1","input_frontier":"0","receipt_id":"receipt-0","received_at":"2000-01-01T00:00:00Z","semantic_commit_ns":null})).unwrap();
        let ctx = StructureContext {
            fact_generation: 1,
            generation: 1,
            cut: "cut-1",
            frontier: 0,
            known_at: &known,
            all_sources: &[0],
        };
        let mut c = ConstructionFact {
            start_stroke: 0,
            stroke_indices: vec![0, 1, 2],
            length: "odd_ge3",
            vector: Some([true, false, true, true]),
            overlap: Some([1, 0]),
            overlap_relation: Some(">"),
            direction: None,
            failed_conditions: vec!["closed_seed_overlap"],
        };
        for (case, expected_condition, expected_direction) in [
            ("known_false", "IN_DOMAIN", "OUTSIDE_DOMAIN"),
            ("known_even", "OUTSIDE_DOMAIN", "OUTSIDE_DOMAIN"),
            (
                "missing_vector",
                "DOMAIN_PROOF_MISSING",
                "DOMAIN_PROOF_MISSING",
            ),
        ] {
            if case == "known_even" {
                c.length = "even_ge3";
                c.vector = None;
                c.stroke_indices.push(3);
            }
            if case == "missing_vector" {
                c.length = "odd_ge3";
                c.stroke_indices.pop();
            }
            let data = fact(&mut nodes, &c).unwrap();
            let object = emit(
                &mut producer,
                &mut nodes,
                &ctx,
                crate::tb02c::KINDS[6],
                s(case),
                &data,
                &[0],
            )
            .unwrap();
            let mut output = StructureOutput {
                objects: vec![],
                relations: vec![],
                segments: vec![],
                requests: vec![],
                waiting_reasons: vec![],
            };
            construction_requests(
                &mut producer,
                &mut nodes,
                &mut output.requests,
                &object,
                s("construction"),
                &c,
                &[0],
            )
            .unwrap();
            output.objects.push(object);
            let reqs = check_requests(&mut nodes, &output);
            assert_eq!(reqs.len(), 3);
            let get = |axis: &str| {
                reqs.iter()
                    .find(|r| {
                        let key: Value =
                            serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                        key[1][1] == axis
                    })
                    .unwrap()
            };
            assert_eq!(get("length")["axes"]["computation"], "COMPLETE");
            assert_eq!(
                get("construction_conditions")["axes"]["applicability"],
                expected_condition
            );
            assert_eq!(
                get("direction")["axes"]["applicability"],
                expected_direction
            );
            assert_eq!(
                get("construction_conditions")["axes"]["computation"],
                if case == "missing_vector" {
                    "INCOMPLETE"
                } else {
                    "COMPLETE"
                }
            );
        }
    }

    #[test]
    fn cd_direct_frozen_cases_match_oracle_and_close_real_object_references() {
        let ledger: Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02d/raw-ledger.json"
        ))
        .unwrap();
        let mut segments = 0;
        let mut seconds = 0;
        for (name, rows) in ledger["cases"].as_object().unwrap() {
            let rows = rows.as_array().unwrap();
            let bars = fixture_bars(rows);
            let raw = fixture_raw(rows);
            let (layer, facts, fold) =
                newchan_rust::theta_v0::parser::parse_layer_with_inclusion_facts_store(
                    &bars,
                    &newchan_rust::theta_v0::config::ThetaConfig::default(),
                    SecondFoldStore::new(crate::tb02c::round_budget()),
                );
            let evidence = StructureEvidence {
                segments: &layer.segments,
                segment_facts: facts.segment_facts.as_ref().unwrap(),
                stroke_facts: facts.stroke_facts.as_ref().unwrap(),
                second_store: &fold,
            };
            let mut nodes = sc::MemoryNodes::new();
            let mut producer = Cc004Producer::new(crate::tb02a::PROFILE, crate::RULE_REVISION);
            bind_fixture(&mut producer, &mut nodes, &raw);
            let known=json_to_built(&mut nodes,&serde_json::json!({"generation":"28","input_frontier":"27","receipt_id":raw.values().last().map(|r|&r["receipt_id"]),"received_at":raw.values().last().map(|r|&r["received_at"]),"semantic_commit_ns":null})).unwrap();
            let sources: Vec<_> = (0..rows.len()).collect();
            let ctx = StructureContext {
                fact_generation: 1,
                generation: 28,
                cut: "cut-28",
                frontier: 27,
                known_at: &known,
                all_sources: &sources,
            };
            let output = build_cd(
                &mut producer,
                &mut nodes,
                &evidence,
                &StructurePrior {
                    segment_records: &[],
                    version_record: None,
                },
                &ctx,
            )
            .unwrap();
            let actual = records(&mut nodes, &output.objects);
            let requests = check_requests(&mut nodes, &output);
            for segment in &output.segments {
                let own: Vec<_> = requests
                    .iter()
                    .filter(|r| r["subject_id"] == segment.object_id)
                    .collect();
                for axis in [
                    "semantic_termination_result",
                    "known_fact_vector",
                    "scan_coverage",
                ] {
                    assert_eq!(
                        own.iter()
                            .filter(|r| {
                                let k: Value =
                                    serde_json::from_str(r["request_id"].as_str().unwrap())
                                        .unwrap();
                                k[1][1] == axis
                            })
                            .count(),
                        1
                    );
                }
                for r in own {
                    let key: Value =
                        serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                    if key[1][1] == "scan_coverage" {
                        assert_eq!(r["axes"]["computation"], "COMPLETE");
                    }
                    if key[1][1] == "known_fact_vector" {
                        let fact =
                            &evidence.segment_facts.candidates[segment.candidate_index].fact_vector;
                        assert_eq!(
                            r["axes"]["computation"],
                            if fact.new_segment_formed.is_some() {
                                "COMPLETE"
                            } else {
                                "INCOMPLETE"
                            }
                        );
                        assert_eq!(r["axes"]["input_quality"], "SUFFICIENT");
                    }
                }
            }
            if !sources.is_empty() {
                let mut unbound = Cc004Producer::new(crate::tb02a::PROFILE, crate::RULE_REVISION);
                assert!(
                    build_cd(
                        &mut unbound,
                        &mut nodes,
                        &evidence,
                        &StructurePrior {
                            segment_records: &[],
                            version_record: None
                        },
                        &ctx
                    )
                    .is_err(),
                    "未登记来源不得生成空来源对象"
                );
            }
            let (expected, _) = crate::tb02c::build(
                evidence.segment_facts,
                evidence.stroke_facts,
                &raw,
                &[],
                &[],
                28,
                "cut-28",
                27,
                None,
                &fold,
            )
            .unwrap();
            let normalize = |rows: &[Value]| -> BTreeMap<String, Value> {
                let ids: BTreeMap<_, _> = rows
                    .iter()
                    .map(|r| {
                        (
                            r["object_id"].as_str().unwrap().to_string(),
                            r["fact_key"].as_str().unwrap().to_string(),
                        )
                    })
                    .collect();
                rows.iter().map(|r|{let mut payload:Value=serde_json::from_str(r["payload_json"].as_str().unwrap()).unwrap();normalize_ids(&mut payload,&ids);(r["fact_key"].as_str().unwrap().to_string(),serde_json::json!({"payload":payload,"sources":r["source_coords_json"],"refs":r["input_refs_json"]}))}).collect()
            };
            assert_eq!(normalize(&actual), normalize(&expected), "{name}");
            let ids: BTreeSet<_> = output
                .objects
                .iter()
                .map(|o| o.object_id.as_str())
                .collect();
            for r in &actual {
                let p: Value = serde_json::from_str(r["payload_json"].as_str().unwrap()).unwrap();
                for key in [
                    "sequence_object_id",
                    "second_sequence_object_id",
                    "witness_object_id",
                    "object_id",
                ] {
                    if let Some(id) = p["data"][key].as_str() {
                        assert!(ids.contains(id), "{name} dangling {key}={id}");
                        assert!(id.starts_with("obj3-"));
                    }
                }
                if r["kind"] == crate::tb02c::KINDS[2] {
                    segments += 1;
                }
                if p["data"]["role"] == "SECOND_SEQUENCE" {
                    seconds += 1;
                }
            }
            let prior_segments: Vec<_> = output
                .objects
                .iter()
                .filter(|o| {
                    field_text(&mut nodes, &o.record, "kind").unwrap() == crate::tb02c::KINDS[2]
                })
                .map(|o| o.record.clone())
                .collect();
            let version = output
                .objects
                .iter()
                .find(|o| {
                    field_text(&mut nodes, &o.record, "kind").unwrap() == crate::tb02c::KINDS[5]
                })
                .unwrap()
                .record
                .clone();
            let replay = build_cd(
                &mut producer,
                &mut nodes,
                &evidence,
                &StructurePrior {
                    segment_records: &prior_segments,
                    version_record: Some(&version),
                },
                &ctx,
            )
            .unwrap();
            assert_eq!(
                output
                    .segments
                    .iter()
                    .map(|s| &s.object_id)
                    .collect::<Vec<_>>(),
                replay
                    .segments
                    .iter()
                    .map(|s| &s.object_id)
                    .collect::<Vec<_>>(),
                "稳定事实不前移修订 {name}"
            );
            assert!(
                !records(&mut nodes, &replay.objects)
                    .iter()
                    .any(|r| r["kind"] == crate::tb02c::KINDS[4]),
                "{name} 伪造change"
            );
            if evidence
                .segment_facts
                .candidates
                .iter()
                .any(|c| c.second.is_some())
            {
                let empty = SecondFoldStore::new(crate::tb02c::round_budget());
                let wrong = StructureEvidence {
                    second_store: &empty,
                    ..evidence
                };
                let error = match build_cd(
                    &mut producer,
                    &mut nodes,
                    &wrong,
                    &StructurePrior {
                        segment_records: &[],
                        version_record: None,
                    },
                    &ctx,
                ) {
                    Ok(_) => panic!("错误游标簿被接受"),
                    Err(e) => e,
                };
                assert!(error.contains("IdentityConflict"));
            }
        }
        assert!(segments > 0, "必须覆盖真实段对象");
        assert!(seconds > 0, "必须覆盖真实第二种候选");
    }

    #[test]
    fn ef_direct_prefix_revisions_pending_and_retained_history_match_oracle() {
        use newchan_rust::theta_v0::classifier::center::UnitRange;
        // 有界段层合同夹具；身份显式作为外部 C/D 输入，不宣称由此覆盖生产 parser 接线。
        let endpoints = [
            10, 40, 20, 35, 30, 38, 21, 25, 5, 9, 2, 8, 3, 7, 4, 7, 4, 7, 4, 7,
        ];
        let mut nodes = sc::MemoryNodes::new();
        let mut producer = Cc004Producer::new(crate::tb02a::PROFILE, crate::RULE_REVISION);
        let rows: Vec<_> = endpoints
            .iter()
            .map(|p| serde_json::json!({"open":p,"high":p,"low":p,"close":p}))
            .collect();
        let all_raw = fixture_raw(&rows);
        bind_fixture(&mut producer, &mut nodes, &all_raw);
        let mut prior_roots = Vec::new();
        let mut prior_oracle = Vec::new();
        let mut aliases = BTreeMap::<String, String>::new();
        let mut pending_seen = false;
        let mut evaluated_seen = false;
        for n in 1..endpoints.len() {
            let completed = (0..n)
                .map(|k| crate::tb02e::SegUnit {
                    unit_id: format!("segment:1:{}", 10 + 4 * k),
                    object_id: format!("test-external-cd-{k}"),
                    range: Some(UnitRange {
                        start_index: 4 * k,
                        end_index: 4 * k + 4,
                        direction: if endpoints[k + 1] > endpoints[k] {
                            Direction::Up
                        } else {
                            Direction::Down
                        },
                        lo: endpoints[k].min(endpoints[k + 1]),
                        hi: endpoints[k].max(endpoints[k + 1]),
                    }),
                    sources: vec![k],
                })
                .collect();
            let inputs = crate::tb02e::UnitInputs {
                completed,
                in_progress: Some(crate::tb02e::SegUnit {
                    unit_id: format!("segment:1:{}", 10 + 4 * n),
                    object_id: format!("test-external-cd-{n}"),
                    range: None,
                    sources: vec![n],
                }),
            };
            let raw: BTreeMap<_, _> = all_raw
                .iter()
                .take(n + 1)
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            let generation = n as i64 + 1;
            let cut = format!("cut-{generation}");
            let frontier = n as i64;
            let known=json_to_built(&mut nodes,&serde_json::json!({"generation":generation.to_string(),"input_frontier":frontier.to_string(),"receipt_id":raw.values().last().unwrap()["receipt_id"],"received_at":"2000-01-01T00:00:00Z","semantic_commit_ns":null})).unwrap();
            let sources: Vec<_> = (0..=n).collect();
            let ctx = StructureContext {
                fact_generation: 1,
                generation,
                cut: &cut,
                frontier,
                known_at: &known,
                all_sources: &sources,
            };
            let output = build_ef_units(
                &mut producer,
                &mut nodes,
                &inputs,
                &EfPrior {
                    records: &prior_roots,
                },
                &ctx,
            )
            .unwrap();
            let actual = records(&mut nodes, &output.objects);
            let requests = check_requests(&mut nodes, &output);
            let mut request_counts = BTreeMap::<String, usize>::new();
            for r in &requests {
                *request_counts
                    .entry(r["subject_id"].as_str().unwrap().to_owned())
                    .or_default() += 1;
                let key: Value = serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                let record = actual
                    .iter()
                    .find(|o| o["object_id"] == r["subject_id"])
                    .unwrap();
                let payload: Value =
                    serde_json::from_str(record["payload_json"].as_str().unwrap()).unwrap();
                if key[1][0] == "CC-020" && payload["data"]["status"] == crate::tb02f::PENDING {
                    assert_eq!(r["axes"]["computation"], "INCOMPLETE");
                    assert_eq!(r["axes"]["applicability"], "DOMAIN_PROOF_MISSING");
                    assert_eq!(r["reasons"], serde_json::json!([crate::tb02f::PENDING]));
                } else {
                    // 含 false 的 seed、进行中单元的已知未归属、尚未实现上级的已判 pair 都已完成。
                    assert_eq!(r["axes"]["computation"], "COMPLETE");
                    assert_eq!(r["axes"]["applicability"], "IN_DOMAIN");
                }
            }
            for r in &actual {
                let expected = match r["kind"].as_str().unwrap() {
                    "CC-016.center_seed_attempt" => 2,
                    "CC-018.unit_core_position"
                    | "CC-020.center_pair"
                    | "CC-022.unit_assignment" => 1,
                    _ => 0,
                };
                assert_eq!(
                    request_counts
                        .get(r["object_id"].as_str().unwrap())
                        .copied()
                        .unwrap_or(0),
                    expected
                );
            }
            let (mut expected, _, centers) = crate::tb02e::build(
                &inputs,
                &raw,
                &[],
                &prior_oracle,
                generation,
                &cut,
                frontier,
                None,
            )
            .unwrap();
            let (pairs, _) = crate::tb02f::build(
                &inputs,
                &centers,
                &raw,
                &prior_oracle,
                generation,
                &cut,
                frontier,
            );
            expected.extend(pairs);
            for r in actual.iter().chain(&expected) {
                aliases.insert(
                    r["object_id"].as_str().unwrap().to_owned(),
                    r["fact_key"].as_str().unwrap().to_owned(),
                );
            }
            let normalize = |rows: &[Value]| -> BTreeMap<String, Value> {
                rows.iter().map(|r|{let mut p:Value=serde_json::from_str(r["payload_json"].as_str().unwrap()).unwrap();normalize_ids(&mut p,&aliases);(r["fact_key"].as_str().unwrap().into(),serde_json::json!({"payload":p,"sources":r["source_coords_json"],"refs":r["input_refs_json"]}))}).collect()
            };
            assert_eq!(normalize(&actual), normalize(&expected), "EF cut={cut}");
            let ids: BTreeSet<_> = output
                .objects
                .iter()
                .map(|o| o.object_id.as_str())
                .collect();
            for r in &actual {
                let p: Value = serde_json::from_str(r["payload_json"].as_str().unwrap()).unwrap();
                if let Some(w) = p["data"]["witness_object_id"].as_str() {
                    assert!(ids.contains(w), "dangling witness {w}");
                }
                if r["kind"] == crate::tb02f::KINDS[1] {
                    pending_seen |= p["data"]["status"] == crate::tb02f::PENDING;
                    evaluated_seen |= p["data"]["status"] == "EVALUATED";
                }
            }
            prior_roots = output
                .objects
                .iter()
                .filter(|o| {
                    let k = field_text(&mut nodes, &o.record, "kind").unwrap();
                    crate::tb02e::KINDS[..4].contains(&k.as_str())
                        || crate::tb02f::KINDS[..2].contains(&k.as_str())
                        || k == crate::tb02e::KINDS[6]
                        || k == crate::tb02f::KINDS[4]
                })
                .map(|o| o.record.clone())
                .collect();
            prior_oracle = expected;
        }
        assert!(
            pending_seen && evaluated_seen,
            "必须经历对已成立→≥9 pending翻转"
        );
    }
}
