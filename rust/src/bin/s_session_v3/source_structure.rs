//! #1419：新结构域来源与请求核。只读已发布事实，绝不重新运行分类器。
use super::*;
use sc::capture_allocation::audit_json::JsonView;

pub(super) fn is_kind(rule: &str, kind: &str) -> bool {
    crate::is_structure_rule(rule)
        && (crate::tb02c::KINDS.contains(&kind)
            || crate::tb02e::KINDS.contains(&kind)
            || crate::tb02f::KINDS.contains(&kind))
}
fn header(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if crate::tb02c::KINDS.contains(&kind) {
        Some((
            crate::tb02c::SCHEMA,
            newchan_rust::theta_v0::parser::segment::CONSTRUCTION_RULE,
            "seed_closed_strict_second",
        ))
    } else if crate::tb02e::KINDS.contains(&kind) {
        Some((
            crate::tb02e::SCHEMA_REVISION,
            "center-seed/1",
            "strict_seed_zn_outer_c_resume",
        ))
    } else if crate::tb02f::KINDS.contains(&kind) {
        Some((
            crate::tb02f::SCHEMA_REVISION,
            "center-pair/1",
            "zn_outer_five_leaf_pending_ge9",
        ))
    } else {
        None
    }
}
pub(super) fn payload(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
) -> Result<RefValue, String> {
    let p = object.fields.get_admitted(s, "payload_json")?;
    s.lookup_ref(p.view(), &[PathStep::JsonText]).map_err(err)
}
fn data(s: &mut (impl SourceFieldStore + ?Sized), object: &Object) -> Result<RefValue, String> {
    let p = payload(s, object)?;
    field(s, &p, "data")
}
fn same_slot(
    s: &mut (impl SourceFieldStore + ?Sized),
    slot: JsonView<'_>,
    want: &RefValue,
) -> Result<(), String> {
    if super::super::history::hash_json_in_store(s, slot, None, None)? != digest(want)? {
        return Err(err("结构槽位不一致"));
    }
    Ok(())
}
pub(super) fn audit_payload(
    s: &mut (impl SourceFieldStore + ?Sized),
    kind: &str,
    p: &RefValue,
    slot: JsonView<'_>,
    sources: &SourceCoords,
    raw: &(impl RawLookup + ?Sized),
    current_generation: i64,
) -> Result<(), String> {
    exact(p, Kind::Object, 7)?;
    let (schema, semantic, policy) = header(kind).ok_or_else(|| err("未知结构头"))?;
    for (k, v) in [
        ("schema_revision", schema),
        ("semantic_version", semantic),
        ("policy_version", policy),
        ("view_role", "main"),
    ] {
        if text(s, p, k)? != v {
            return Err(err("结构语义头不符"));
        }
    }
    if decimal(&text(s, p, "object_generation")?)? < 0 {
        return Err(err("结构代号为负"));
    }
    let payload_slot = field(s, p, "slot")?;
    same_slot(s, slot, &payload_slot)?;
    let d = field(s, p, "data")?;
    if d.kind() != Kind::Object {
        return Err(err("结构data不是对象"));
    }
    let version = match kind {
        "CC-056.segment_version" => Some(("TB-02-W", "seed_strict_first_and_second_terminal")),
        "CC-056.center_version" => Some(("TB-02-E", "l0_strict_seed_zn_roles_c_resume_assignment")),
        "CC-056.pair_version" => Some((
            "TB-02-F",
            "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position",
        )),
        _ => None,
    };
    if let Some((name, scope)) = version {
        if slot.as_str() != Some(name)
            || text(s, &d, "delivered_scope")? != scope
            || sources.len() != raw.len()
        {
            return Err(err("结构版本槽位/范围/全原始账来源不符"));
        }
        if field(s, &d, "waiting_reasons")?.kind() != Kind::Array
            || field(s, &d, "deferred_scope")?.kind() != Kind::Array
        {
            return Err(err("结构版本范围不是数组"));
        }
        decimal(&text(s, &d, "input_frontier")?)?;
    } else if kind.ends_with("_relation") {
        let arr = slot.as_array().ok_or_else(|| err("关系槽位不是数组"))?;
        if arr.len() != 3 {
            return Err(err("关系槽位长度不符"));
        }
        for (i, k) in ["source_id", "relation_kind", "target_id"]
            .iter()
            .enumerate()
        {
            if slot.index(i).and_then(|v| v.as_str()) != Some(text(s, &d, k)?.as_str()) {
                return Err(err("关系槽位与端点不符"));
            }
        }
        if decimal(&text(s, &d, "version")?)? < 1 {
            return Err(err("关系版本不符"));
        }
    } else if kind.ends_with("_change") {
        if slot.as_array().is_none_or(|a| a.len() != 3)
            || decimal(
                slot.index(0)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| err("结构变更代号非文本"))?,
            )? != current_generation
            || slot.index(1).and_then(|v| v.as_str()) != Some(text(s, &d, "entity_id")?.as_str())
            || slot.index(2).and_then(|v| v.as_str()) != Some(text(s, &d, "change")?.as_str())
        {
            return Err(err("结构变更槽位不符"));
        }
        let known = field(s, &d, "known_at")?;
        if decimal(&text(s, &known, "generation")?)? != current_generation {
            return Err(err("结构变更获知代际与当前cut不符"));
        }
        let after = field(s, &d, "after")?;
        let object = field(s, &d, "object_id")?;
        if (after.kind() == Kind::Null) != (object.kind() == Kind::Null) {
            return Err(err("结构变更after与对象缺失不一致"));
        }
    } else if slot.as_str() != Some(text(s, &d, "entity_id")?.as_str()) {
        return Err(err("结构实体槽位不符"));
    }
    Ok(())
}

pub(super) trait Lookup {
    fn object(&self, id: &str) -> Option<&Object>;
    fn entity(&self, _id: &str) -> Option<&Object> {
        None
    }
}
struct FullLookup<'a> {
    ids: BusinessMap<&'a str, &'a Object>,
    entities: BusinessMap<bj::TextValue, &'a Object>,
}
impl Lookup for FullLookup<'_> {
    fn entity(&self, id: &str) -> Option<&Object> {
        self.entities.get(id).copied()
    }
    fn object(&self, id: &str) -> Option<&Object> {
        self.ids.get(id).copied()
    }
}

/// 仅当前对象边；before/previous 是历史值快照，不冒充当前对象边。
/// 收集与校验共用路径，以便 Native 删除/修改边时撤销同一组逆依赖。
pub(super) fn references(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
) -> Result<sc::CodecVec<bj::TextValue>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    let d = data(s, object)?;
    fn walk(
        s: &mut (impl SourceFieldStore + ?Sized),
        r: &RefValue,
        out: &mut sc::CodecVec<bj::TextValue>,
    ) -> Result<(), String> {
        match r.kind() {
            Kind::Array => {
                for e in elements(s, r)? {
                    walk(s, &e, out)?;
                }
            }
            Kind::Object => {
                for key in [
                    "sequence_object_id",
                    "second_sequence_object_id",
                    "unit_object_id",
                    "witness_object_id",
                    "object_id",
                ] {
                    if let Some(value) = s.lookup_optional_field_ref(r.view(), key).map_err(err)? {
                        if value.kind() != Kind::Null {
                            let scalar = scalar_value(s, value.view())?;
                            let id = scalar
                                .view()
                                .as_str()
                                .ok_or_else(|| err("结构对象边不是文本"))?;
                            if !id.starts_with("obj3-") {
                                return Err(err("结构对象边不是obj3身份"));
                            }
                            s.native_codec_room(out, 1).map_err(err)?;
                            out.try_push(s.native_text(id)?).map_err(err)?;
                        }
                    }
                }
                for key in ["unit", "units", "after"] {
                    if let Some(value) = s.lookup_optional_field_ref(r.view(), key).map_err(err)? {
                        walk(s, &value, out)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    walk(s, &d, &mut out)?;
    Ok(out)
}
pub(super) fn audit_links(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
    lookup: &impl Lookup,
) -> Result<(), String> {
    for id in references(s, object)? {
        let child = lookup
            .object(id.as_str())
            .ok_or_else(|| err("结构对象子边悬空"))?;
        if child.id == object.id {
            return Err(err("结构对象自引用"));
        }
        // 关系对象的来源是见证的子集；其他对象可能引用更窄或更宽的已核事实。
        if object.kind.ends_with("_relation")
            && object.sources.iter().any(|c| !child.sources.contains(c))
        {
            return Err(err("结构关系来源不在见证中"));
        }
    }
    let d = data(s, object)?;
    if object.kind == "CC-013.segment" {
        for (id_field, entity_field) in [
            ("sequence_object_id", "sequence_id"),
            ("second_sequence_object_id", "second_sequence_id"),
        ] {
            let id = field(s, &d, id_field)?;
            let entity = field(s, &d, entity_field)?;
            if (id.kind() == Kind::Null) != (entity.kind() == Kind::Null) {
                return Err(err("段序列对象/实体边缺失不一致"));
            }
            if id.kind() != Kind::Null {
                let id = scalar_value(s, id.view())?;
                let child = lookup
                    .object(
                        id.view()
                            .as_str()
                            .ok_or_else(|| err("段序列对象边非文本"))?,
                    )
                    .ok_or_else(|| err("段序列对象悬空"))?;
                if child.kind != "CC-012.feature_sequence" {
                    return Err(err("段序列对象种类不符"));
                }
                let child_data = data(s, child)?;
                if digest(&field(s, &child_data, "entity_id")?)? != digest(&entity)?
                    || text(s, &child_data, "segment_id")? != text(s, &d, "entity_id")?
                {
                    return Err(err("段序列实体边错绑"));
                }
                let role = if id_field == "sequence_object_id" {
                    "FIRST_SEQUENCE"
                } else {
                    "SECOND_SEQUENCE"
                };
                if text(s, &child_data, "role")? != role {
                    return Err(err("段序列角色错绑"));
                }
            }
        }
    }
    fn unit(
        s: &mut (impl SourceFieldStore + ?Sized),
        unit: &RefValue,
        lookup: &impl Lookup,
    ) -> Result<(), String> {
        let id = text(s, unit, "unit_object_id")?;
        let child = lookup
            .object(id.as_str())
            .ok_or_else(|| err("单元段对象悬空"))?;
        let d = data(s, child)?;
        if child.kind != "CC-013.segment" || text(s, unit, "unit_id")? != text(s, &d, "entity_id")?
        {
            return Err(err("单元实体/对象错绑"));
        }
        Ok(())
    }
    if let Some(u) = s.lookup_optional_field_ref(d.view(), "unit").map_err(err)? {
        unit(s, &u, lookup)?;
    }
    if let Some(units) = s
        .lookup_optional_field_ref(d.view(), "units")
        .map_err(err)?
    {
        for u in elements(s, &units)? {
            unit(s, &u, lookup)?;
        }
    }
    if object.kind.ends_with("_relation") {
        audit_relation(s, object, &d, lookup)?;
    }
    if object.kind == "CC-020.center_pair" {
        audit_pair_sides(s, &d, lookup)?;
    }

    if object.kind.ends_with("_change") {
        let id = field(s, &d, "object_id")?;
        if id.kind() != Kind::Null {
            let id = scalar_value(s, id.view())?;
            let child = lookup
                .object(id.view().as_str().ok_or_else(|| err("变更对象非文本"))?)
                .ok_or_else(|| err("变更对象悬空"))?;
            let child_data = data(s, child)?;
            let expected_kind = match object.kind.as_str() {
                "CC-055.segment_change" => "CC-013.segment",
                "CC-055.center_change" if text(s, &d, "change")? == "roles_updated" => {
                    "CC-017.center_roles"
                }
                "CC-055.center_change" => "CC-016.center",
                "CC-055.pair_change" => "CC-020.center_pair",
                _ => return Err(err("未知结构change种类")),
            };
            let entity = text(s, &d, "entity_id")?;
            if child.kind != expected_kind
                || text(s, &child_data, "entity_id")? != entity
                || lookup
                    .entity(entity.as_str())
                    .is_none_or(|o| o.id != child.id)
            {
                return Err(err("结构change实体/种类错绑"));
            }
            if digest(&field(s, &d, "after")?)? != digest(&child_data)? {
                return Err(err("变更after与真实对象原件不符"));
            }
        } else {
            let entity = text(s, &d, "entity_id")?;
            let before = field(s, &d, "before")?;
            if text(s, &d, "change")? != "withdrawn"
                || before.kind() == Kind::Null
                || text(s, &before, "entity_id")? != entity
                || lookup.entity(entity.as_str()).is_some()
            {
                return Err(err("withdrawn实体/历史快照/当前缺席不符"));
            }
        }
    }
    Ok(())
}
pub(super) fn audit_full(
    s: &mut (impl SourceFieldStore + ?Sized),
    objects: &[Object],
    rule: &str,
) -> Result<(), String> {
    if !crate::is_structure_rule(rule) {
        return Ok(());
    }
    let allocation = source_allocation(s)?;
    let mut ids = bj::new_map(allocation.as_ref());
    for o in objects {
        s.native_map_room(&mut ids).map_err(err)?;
        ids.insert(o.id.as_str(), o).map_err(err)?;
    }
    let mut entities = bj::new_map(allocation.as_ref());
    for o in objects {
        if let Some(id) = entity_id(s, o)? {
            s.native_map_room(&mut entities).map_err(err)?;
            if entities.insert(id, o).map_err(err)?.is_some() {
                return Err(err("当前结构实体身份重复"));
            }
        }
    }
    let lookup = FullLookup { ids, entities };
    for kind in [
        "CC-056.segment_version",
        "CC-056.center_version",
        "CC-056.pair_version",
    ] {
        if objects.iter().filter(|o| o.kind.as_str() == kind).count() != 1 {
            return Err(err("结构域版本根缺失或重复"));
        }
    }
    for o in objects {
        if is_kind(rule, o.kind.as_str()) {
            audit_links(s, o, &lookup)?;
        }
    }
    Ok(())
}

/// 具名请求必须指向当前同一 obj3、同一 fact_key 和来源；四轴直接读取该对象的证据状态。
pub(super) fn audit_request(
    s: &mut (impl SourceFieldStore + ?Sized),
    request: &RefValue,
    key: JsonView<'_>,
    lookup: &impl Lookup,
) -> Result<(), String> {
    let address = key.index(1).ok_or_else(|| err("结构请求缺地址"))?;
    if address.as_array().is_none_or(|a| a.len() != 5)
        || address.index(2).and_then(|v| v.as_str()) != Some("structure-classification/1")
    {
        return Err(err("结构请求版本/地址不符"));
    }
    let subject = text(s, request, "subject_id")?;
    if address.index(4).and_then(|v| v.as_str()) != Some(subject.as_str()) {
        return Err(err("结构请求主体与键不符"));
    }
    let object = lookup
        .object(subject.as_str())
        .ok_or_else(|| err("结构请求主体不在当前cut"))?;
    if !is_kind(crate::V3_STRUCTURE_RULE_REVISION, object.kind.as_str()) {
        return Err(err("结构请求主体域不符"));
    }
    let fact = object.fields.text(s, "fact_key")?;
    let fact = parsed_json(s, fact.as_bytes())?;
    if super::super::history::hash_json_in_store(s, key.index(0).unwrap(), None, None)?
        != super::super::history::hash_json_in_store(s, fact.view(), None, None)?
    {
        return Err(err("结构请求fact_key错绑"));
    }
    let source = field(s, request, "source_coords")?;
    if &*coords(s, &source)? != &*object.sources {
        return Err(err("结构请求来源错绑"));
    }
    let cc = address
        .index(0)
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("结构请求CC非文本"))?;
    let axis = address
        .index(1)
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("结构请求轴非文本"))?;
    let path = address.index(3).ok_or_else(|| err("结构请求缺路径"))?;
    let d = data(s, object)?;
    // 0=完整，1=域外，2=证据不足，3=计算未完；false 已算出仍是完整。
    let mut state = 0;
    let mut reason = None;
    let kind = object.kind.as_str();
    match (cc, axis) {
        ("CC-011", "length" | "construction_conditions" | "direction")
            if [
                "CC-011.segment_seed",
                "CC-011.seed_candidate",
                "CC-013.segment",
            ]
            .contains(&kind) =>
        {
            let c = if path.as_str() == Some("construction") {
                field(s, &d, "construction")?
            } else {
                if kind != "CC-013.segment"
                    || path.as_array().is_none_or(|a| a.len() != 3)
                    || path.index(0).and_then(|v| v.as_str()) != Some("termination_attempts")
                    || path.index(2).and_then(|v| v.as_str()) != Some("construction")
                {
                    return Err(err("结构construction请求路径不符"));
                }
                let index = decimal(
                    path.index(1)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| err("请求attempt索引非文本"))?,
                )?;
                let index = usize::try_from(index).map_err(|_| err("请求attempt索引负数"))?;
                let attempts = field(s, &d, "termination_attempts")?;
                let entries = elements(s, &attempts)?;
                let a = entries
                    .get(index)
                    .ok_or_else(|| err("请求attempt索引越界"))?;
                field(s, a, "construction")?
            };
            let vector = field(s, &c, "vector")?;
            let odd = text(s, &c, "length")? == "odd_ge3";
            if axis != "length" {
                if vector.kind() == Kind::Null {
                    if odd {
                        state = 2;
                        reason = Some("construction_conditions_not_evaluated");
                    } else {
                        state = 1;
                        reason = Some("construction_length_outside_domain");
                    }
                } else if axis == "direction" {
                    exact(&vector, Kind::Array, 4)?;
                    let mut all = true;
                    for bit in elements(s, &vector)? {
                        all &= scalar_value(s, bit.view())?
                            .view()
                            .as_bool()
                            .ok_or_else(|| err("construction向量非布尔"))?;
                    }
                    if !all {
                        state = 1;
                        reason = Some("construction_conditions_failed");
                    } else if field(s, &c, "direction")?.kind() == Kind::Null {
                        state = 2;
                        reason = Some("construction_direction_not_evaluated");
                    }
                }
            }
        }
        ("CC-013", "semantic_termination_result")
            if kind == "CC-013.segment" && path.as_str() == Some("result") =>
        {
            let result = text(s, &d, "result")?;
            let second = field(s, &d, "second_sequence_object_id")?;
            if second.kind() != Kind::Null
                && !["FAILED_GEOMETRY", "CASE_ONE", "CASE_TWO", "KILLED_CASE_TWO"]
                    .contains(&result.as_str())
            {
                let id = scalar_value(s, second.view())?;
                let child = lookup
                    .object(
                        id.view()
                            .as_str()
                            .ok_or_else(|| err("SECOND对象边非文本"))?,
                    )
                    .ok_or_else(|| err("SECOND对象缺失"))?;
                let child = data(s, child)?;
                let scan = field(s, &child, "scan")?;
                let facts = field(s, &child, "facts")?;
                if scalar(s, &scan, "resource_truncated")?.view().as_bool() == Some(true)
                    && field(s, &facts, "matched")?.kind() == Kind::Null
                {
                    state = 3;
                    reason = Some("second_sequence_resource_bound");
                }
            }
        }
        ("CC-014", "known_fact_vector" | "scan_coverage")
            if kind == "CC-013.segment" && path.as_str() == Some("fact_vector") =>
        {
            let facts = field(s, &d, "fact_vector")?;
            if axis == "known_fact_vector"
                && (field(s, &facts, "new_segment_formed")?.kind() == Kind::Null
                    || field(s, &facts, "segment_broken")?.kind() == Kind::Null)
            {
                state = 3;
                reason = Some("break_facts_scan_incomplete");
            }
        }
        ("CC-016", "seed_condition_vector")
            if kind == "CC-016.center_seed_attempt" && path.as_str() == Some("vector") =>
        {
            let vector = field(s, &d, "vector")?;
            if field(s, &vector, "strict")?.kind() == Kind::Null {
                state = 2;
                reason = Some("seed_strict_not_evaluated");
            }
        }
        ("CC-016", "core_order")
            if kind == "CC-016.center_seed_attempt" && path.as_str() == Some("core_order") =>
        {
            field(s, &d, "core_order")?;
        }
        ("CC-018", "unit_core_position")
            if kind == "CC-018.unit_core_position" && path.as_str() == Some("position") =>
        {
            field(s, &d, "position")?;
        }
        ("CC-020", "center_pair_development")
            if kind == "CC-020.center_pair" && path.as_str() == Some("leaf") =>
        {
            if field(s, &d, "leaf")?.kind() == Kind::Null {
                if text(s, &d, "status")? != crate::tb02f::PENDING {
                    return Err(err("空pair叶没有pending证据"));
                }
                state = 2;
                reason = Some(crate::tb02f::PENDING);
            }
        }
        ("CC-022", "assignment_status")
            if kind == "CC-022.unit_assignment" && path.as_str() == Some("status") =>
        {
            field(s, &d, "status")?;
        }
        _ => return Err(err("结构请求未声明的主体/CC/轴/路径组合")),
    }
    let axes = field(s, request, "axes")?;
    exact(&axes, Kind::Object, 4)?;
    for (k, v) in [
        (
            "input_quality",
            if state == 2 {
                "INSUFFICIENT"
            } else {
                "SUFFICIENT"
            },
        ),
        (
            "applicability",
            match state {
                1 => "OUTSIDE_DOMAIN",
                2 => "DOMAIN_PROOF_MISSING",
                _ => "IN_DOMAIN",
            },
        ),
        (
            "computation",
            if state >= 2 { "INCOMPLETE" } else { "COMPLETE" },
        ),
        ("validity", "CURRENT"),
    ] {
        if text(s, &axes, k)? != v {
            return Err(err("结构请求四轴与事实不符"));
        }
    }
    let reasons = field(s, request, "reasons")?;
    exact(&reasons, Kind::Array, u64::from(reason.is_some()))?;
    if let Some(want) = reason {
        let values = elements(s, &reasons)?;
        if scalar_value(s, values[0].view())?.view().as_str() != Some(want) {
            return Err(err("结构请求原因与事实不符"));
        }
    }
    Ok(())
}

pub(super) fn expected_requests(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
) -> Result<usize, String> {
    Ok(match object.kind.as_str() {
        "CC-011.segment_seed" | "CC-011.seed_candidate" => 3,
        "CC-013.segment" => {
            let d = data(s, object)?;
            let attempts = field(s, &d, "termination_attempts")?;
            if attempts.kind() != Kind::Array {
                return Err(err("段termination_attempts不是数组"));
            }
            usize::try_from(attempts.extent().ok_or_else(|| err("段attempts无长度"))?)
                .ok()
                .and_then(|n| n.checked_mul(3))
                .and_then(|n| n.checked_add(6))
                .ok_or_else(|| err("结构请求数量溢出"))?
        }
        "CC-016.center_seed_attempt" => 2,
        "CC-018.unit_core_position" | "CC-020.center_pair" | "CC-022.unit_assignment" => 1,
        _ => 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn put(nodes: &mut sc::MemoryNodes, value: &Value) -> RefValue {
        RefValue::Plain(
            sc::write_value(nodes, &cv(value).unwrap())
                .unwrap()
                .content_ref(),
        )
    }
    pub(super) fn object(
        nodes: &mut sc::MemoryNodes,
        kind: &str,
        id: &str,
        slot: Value,
        d: Value,
    ) -> Object {
        let (schema, semantic, policy) = header(kind).unwrap_or((
            tb02b::CURRENT_CONTRACT.schema,
            tb02b::CURRENT_CONTRACT.semantic,
            tb02b::CURRENT_CONTRACT.policy,
        ));
        let p = json!({"schema_revision":schema,"semantic_version":semantic,"policy_version":policy,"view_role":"main","object_generation":"1","slot":slot,"data":d});
        let key = canonical_json(&json!([kind, slot]));
        let record = put(
            nodes,
            &json!({"payload_json":canonical_json(&p),"fact_key":key}),
        );
        let refs = put(nodes, &json!([]));
        Object {
            id: bj::TextValue::Plain(id.into()),
            kind: bj::TextValue::Plain(kind.into()),
            fields: SourceFields::read_subset(nodes, record.view(), &["payload_json", "fact_key"])
                .unwrap(),
            refs,
            sources: SourceCoords::Original(sc::CodecVec::from_plain(vec![])),
        }
    }
    struct One<'a>(&'a Object);
    impl Lookup for One<'_> {
        fn object(&self, id: &str) -> Option<&Object> {
            (id == self.0.id.as_str()).then_some(self.0)
        }
    }
    fn request(kind: &str, id: &str, slot: Value, cc: &str, axis: &str, path: Value) -> Value {
        let key = json!([
            [kind, slot],
            [cc, axis, "structure-classification/1", path, id]
        ]);
        json!({"request_id":canonical_json(&key),"subject_id":id,"axes":{"input_quality":"SUFFICIENT","applicability":"IN_DOMAIN","computation":"COMPLETE","validity":"CURRENT"},"reasons":[],"source_coords":[]})
    }
    fn check(nodes: &mut sc::MemoryNodes, object: &Object, r: &Value) -> Result<(), String> {
        let original = put(nodes, r);
        let key: Value = serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
        audit_request(nodes, &original, (&key).into(), &One(object))
    }
    #[test]
    fn exact_structure_domain_and_header_slot_are_required() {
        assert!(is_kind(
            crate::V3_STRUCTURE_RULE_REVISION,
            "CC-056.center_version"
        ));
        assert!(!is_kind(crate::V3_RULE_REVISION, "CC-056.center_version"));
        assert!(!is_kind(
            crate::V3_STRUCTURE_RULE_REVISION,
            "CC-056.future_version"
        ));
        let mut n = sc::MemoryNodes::default();
        let o = object(
            &mut n,
            "CC-011.seed_candidate",
            "obj3-subject",
            json!("seed:1"),
            json!({"entity_id":"seed:1","construction":{},"stroke_refs":[]}),
        );
        let p = payload(&mut n, &o).unwrap();
        let raw = BusinessMap::<i64, Source>::plain();
        audit_payload(
            &mut n,
            o.kind.as_str(),
            &p,
            (&json!("seed:1")).into(),
            &o.sources,
            &raw,
            1,
        )
        .unwrap();
        assert!(audit_payload(
            &mut n,
            o.kind.as_str(),
            &p,
            (&json!("seed:wrong")).into(),
            &o.sources,
            &raw,
            1,
        )
        .is_err());
    }
    #[test]
    fn computed_false_is_complete_but_direction_is_outside() {
        let mut n = sc::MemoryNodes::default();
        let kind = "CC-011.seed_candidate";
        let o = object(
            &mut n,
            kind,
            "obj3-seed",
            json!("seed"),
            json!({"entity_id":"seed","construction":{"length":"odd_ge3","vector":[true,false,true,true],"direction":null}}),
        );
        let mut r = request(
            kind,
            "obj3-seed",
            json!("seed"),
            "CC-011",
            "construction_conditions",
            json!("construction"),
        );
        check(&mut n, &o, &r).unwrap();
        r["axes"]["computation"] = json!("INCOMPLETE");
        assert!(check(&mut n, &o, &r).is_err());
        let mut r = request(
            kind,
            "obj3-seed",
            json!("seed"),
            "CC-011",
            "direction",
            json!("construction"),
        );
        assert!(check(&mut n, &o, &r).is_err());
        r["axes"]["applicability"] = json!("OUTSIDE_DOMAIN");
        r["reasons"] = json!(["construction_conditions_failed"]);
        check(&mut n, &o, &r).unwrap();
        assert_eq!(expected_requests(&mut n, &o).unwrap(), 3);
    }
    #[test]
    fn request_subject_slot_sources_and_named_axis_cannot_be_rebound() {
        let mut n = sc::MemoryNodes::default();
        let kind = "CC-022.unit_assignment";
        let o = object(
            &mut n,
            kind,
            "obj3-assignment",
            json!("assignment"),
            json!({"entity_id":"assignment","status":"NOT_YET_ASSIGNED"}),
        );
        let valid = request(
            kind,
            "obj3-assignment",
            json!("assignment"),
            "CC-022",
            "assignment_status",
            json!("status"),
        );
        check(&mut n, &o, &valid).unwrap();
        for broken in [
            request(
                kind,
                "obj3-assignment",
                json!("other"),
                "CC-022",
                "assignment_status",
                json!("status"),
            ),
            request(
                kind,
                "obj3-other",
                json!("assignment"),
                "CC-022",
                "assignment_status",
                json!("status"),
            ),
            request(
                kind,
                "obj3-assignment",
                json!("assignment"),
                "CC-020",
                "center_pair_development",
                json!("leaf"),
            ),
        ] {
            assert!(check(&mut n, &o, &broken).is_err());
        }
        let mut broken = valid;
        broken["source_coords"] = json!(["9"]);
        assert!(check(&mut n, &o, &broken).is_err());
    }
    #[test]
    fn pending_pair_requires_missing_evidence_axes() {
        let mut n = sc::MemoryNodes::default();
        let kind = "CC-020.center_pair";
        let o = object(
            &mut n,
            kind,
            "obj3-pair",
            json!("pair"),
            json!({"entity_id":"pair","status":crate::tb02f::PENDING,"leaf":null}),
        );
        let mut r = request(
            kind,
            "obj3-pair",
            json!("pair"),
            "CC-020",
            "center_pair_development",
            json!("leaf"),
        );
        assert!(check(&mut n, &o, &r).is_err());
        r["axes"]["input_quality"] = json!("INSUFFICIENT");
        r["axes"]["applicability"] = json!("DOMAIN_PROOF_MISSING");
        r["axes"]["computation"] = json!("INCOMPLETE");
        r["reasons"] = json!([crate::tb02f::PENDING]);
        check(&mut n, &o, &r).unwrap();
    }
    #[test]
    fn child_object_reference_cannot_dangle_or_use_old_identity() {
        let mut n = sc::MemoryNodes::default();
        for id in ["obj3-missing", "obj-old"] {
            let o = object(
                &mut n,
                "CC-055.segment_relation",
                "obj3-relation",
                json!(["a", "derived_from", "b"]),
                json!({"source_id":"a","target_id":"b","relation_kind":"derived_from","version":"1","witness_object_id":id}),
            );
            assert!(audit_links(&mut n, &o, &One(&o)).is_err());
        }
    }
}

/// 当前实体索引不纳入事件、关系或版本根；这些会合法复用主体的 entity_id。
pub(super) fn entity_id(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
) -> Result<Option<bj::TextValue>, String> {
    if object.kind == "CC-010.bi"
        || (is_kind(crate::V3_STRUCTURE_RULE_REVISION, object.kind.as_str())
            && !object.kind.starts_with("CC-055.")
            && !object.kind.starts_with("CC-056."))
    {
        let d = data(s, object)?;
        Ok(Some(text(s, &d, "entity_id")?))
    } else {
        Ok(None)
    }
}
pub(super) fn entity_references(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
) -> Result<sc::CodecVec<bj::TextValue>, String> {
    let allocation = source_allocation(s)?;
    let mut out = bj::new_work(allocation.as_ref());
    let fields: &[&str] = if object.kind.ends_with("_change") {
        &["entity_id"]
    } else if object.kind.ends_with("_relation") {
        &["source_id", "target_id"]
    } else if object.kind == "CC-020.center_pair" {
        &["a_center_id", "b_center_id"]
    } else if object.kind == "CC-022.unit_assignment" {
        &["left_center_id", "right_center_id"]
    } else if object.kind == "CC-018.unit_core_position" {
        &["center_id"]
    } else {
        &[]
    };
    if !fields.is_empty() {
        let d = data(s, object)?;
        for name in fields {
            let value = field(s, &d, name)?;
            if value.kind() != Kind::Null {
                let value = text_value(s, value.view(), name)?;
                s.native_codec_room(&mut out, 1).map_err(err)?;
                out.try_push(value).map_err(err)?;
            }
        }
    }
    Ok(out)
}

fn current_entity<'a>(lookup: &'a impl Lookup, id: &str, kind: &str) -> Result<&'a Object, String> {
    let object = lookup
        .entity(id)
        .ok_or_else(|| err("结构关系实体端点悬空"))?;
    if object.kind != kind {
        return Err(err("结构关系实体端点种类错绑"));
    }
    Ok(object)
}
fn listed(
    s: &mut (impl SourceFieldStore + ?Sized),
    array: &RefValue,
    want: &str,
) -> Result<bool, String> {
    for item in elements(s, array)? {
        if scalar_value(s, item.view())?.view().as_str() == Some(want) {
            return Ok(true);
        }
    }
    Ok(false)
}
fn require_sources(object: &Object, expected: &[i64]) -> Result<(), String> {
    if &*object.sources != expected {
        return Err(err("结构关系来源与见证投影不同"));
    }
    Ok(())
}
fn revision(
    s: &mut (impl SourceFieldStore + ?Sized),
    d: &RefValue,
    name: &str,
) -> Result<i64, String> {
    let rev = decimal(&text(s, d, name)?)?;
    if rev < 1 {
        return Err(err("结构关系版本不是正数"));
    }
    Ok(rev)
}
/// 从已核事实投影边，不计算几何、种子或分类。
fn audit_relation(
    s: &mut (impl SourceFieldStore + ?Sized),
    object: &Object,
    d: &RefValue,
    lookup: &impl Lookup,
) -> Result<(), String> {
    let source = text(s, d, "source_id")?;
    let target = text(s, d, "target_id")?;
    let relation = text(s, d, "relation_kind")?;
    let version = revision(s, d, "version")?;
    let witness_id = text(s, d, "witness_object_id")?;
    let witness = lookup
        .object(witness_id.as_str())
        .ok_or_else(|| err("结构关系见证悬空"))?;
    let w = data(s, witness)?;
    let entity = text(s, &w, "entity_id")?;
    match (
        object.kind.as_str(),
        relation.as_str(),
        witness.kind.as_str(),
    ) {
        ("CC-055.segment_relation", "derived_from", "CC-013.segment") => {
            let sequence = current_entity(lookup, source.as_str(), "CC-012.feature_sequence")?;
            let segment = current_entity(lookup, target.as_str(), "CC-013.segment")?;
            if segment.id != witness.id
                || target != entity
                || version != revision(s, &w, "entity_revision")?
            {
                return Err(err("段derived_from目标/版本错绑"));
            }
            let mut matched = false;
            for (name, idname) in [
                ("sequence_id", "sequence_object_id"),
                ("second_sequence_id", "second_sequence_object_id"),
            ] {
                let r = field(s, &w, name)?;
                if r.kind() != Kind::Null
                    && scalar_value(s, r.view())?.view().as_str() == Some(source.as_str())
                {
                    matched = text(s, &w, idname)? == sequence.id;
                }
            }
            if !matched {
                return Err(err("段derived_from源序列错绑"));
            }
            require_sources(object, &witness.sources)?;
        }
        ("CC-055.segment_relation", "member_of", "CC-013.segment") => {
            let sequence = current_entity(lookup, target.as_str(), "CC-012.feature_sequence")?;
            let bi = current_entity(lookup, source.as_str(), "CC-010.bi")?;
            let seq = data(s, sequence)?;
            if text(s, &seq, "segment_id")? != entity
                || version != revision(s, &w, "entity_revision")?
            {
                return Err(err("笔成员关系目标段/版本错绑"));
            }
            let mut matched_sequence = false;
            for name in ["sequence_object_id", "second_sequence_object_id"] {
                let r = field(s, &w, name)?;
                if r.kind() != Kind::Null
                    && scalar_value(s, r.view())?.view().as_str() == Some(sequence.id.as_str())
                {
                    matched_sequence = true;
                }
            }
            if !matched_sequence {
                return Err(err("笔成员序列不是见证段的序列"));
            }
            let stroke_refs = field(s, &seq, "stroke_refs")?;
            let facts = field(s, &seq, "facts")?;
            let raw = field(s, &facts, "raw_elements")?;
            let mut matched = false;
            for stroke in elements(s, &stroke_refs)? {
                if text(s, &stroke, "entity_id")? == source {
                    let index = decimal(&text(s, &stroke, "stroke_index")?)?;
                    for element in elements(s, &raw)? {
                        if decimal(&text(s, &element, "stroke_idx")?)? == index {
                            matched = true;
                        }
                    }
                }
            }
            if !matched {
                return Err(err("笔成员未出现在序列真实raw_elements"));
            }
            let bi_data = data(s, bi)?;
            let start = field(s, &bi_data, "start")?;
            let end = field(s, &bi_data, "end")?;
            let a = field(s, &start, "source_coords")?;
            let b = field(s, &end, "source_coords")?;
            let a = coords(s, &a)?;
            let b = coords(s, &b)?;
            let sources = union_coords(s, &a, &b)?;
            require_sources(object, &sources)?;
        }
        ("CC-055.center_relation", "attempted_from", "CC-016.center_seed_attempt") => {
            let from = current_entity(lookup, source.as_str(), "CC-016.center_seed_attempt")?;
            current_entity(lookup, target.as_str(), "CC-013.segment")?;
            let units = field(s, &w, "units")?;
            let units = elements(s, &units)?;
            let first = units.first().ok_or_else(|| err("seed attempt缺首单元"))?;
            if from.id != witness.id
                || source != entity
                || text(s, first, "unit_id")? != target
                || version != 1
            {
                return Err(err("attempted_from端点/版本错绑"));
            }
            require_sources(object, &witness.sources)?;
        }
        ("CC-055.center_relation", "member_of", "CC-016.center") => {
            let unit = current_entity(lookup, source.as_str(), "CC-013.segment")?;
            let center = current_entity(lookup, target.as_str(), "CC-016.center")?;
            let members = field(s, &w, "members")?;
            if center.id != witness.id
                || target != entity
                || !listed(s, &members, source.as_str())?
                || text(s, &w, "status")? == "UPGRADE_PENDING_TB02H"
                || version != revision(s, &w, "entity_revision")?
            {
                return Err(err("中枢member_of端点/状态/版本错绑"));
            }
            require_sources(object, &unit.sources)?;
        }
        (
            "CC-055.center_relation",
            "connects_left" | "connects_right",
            "CC-022.unit_assignment",
        ) => {
            let unit = current_entity(lookup, source.as_str(), "CC-013.segment")?;
            let center = current_entity(lookup, target.as_str(), "CC-016.center")?;
            let u = field(s, &w, "unit")?;
            let side = if relation == "connects_left" {
                "left_center_id"
            } else {
                "right_center_id"
            };
            let center_data = data(s, center)?;
            if text(s, &u, "unit_id")? != source
                || text(s, &u, "unit_object_id")? != unit.id
                || text(s, &w, side)? != target
                || version != revision(s, &center_data, "entity_revision")?
            {
                return Err(err("连接关系端点/当前中心版本错绑"));
            }
            require_sources(object, &unit.sources)?;
        }
        (
            "CC-055.pair_relation",
            "position_of" | "position_against",
            "CC-018.unit_core_position",
        ) => {
            let from = current_entity(lookup, source.as_str(), "CC-018.unit_core_position")?;
            if from.id != witness.id
                || source != entity
                || version != revision(s, &w, "center_revision")?
            {
                return Err(err("位置关系主体/版本错绑"));
            }
            if relation == "position_of" {
                let unit = current_entity(lookup, target.as_str(), "CC-013.segment")?;
                let u = field(s, &w, "unit")?;
                if text(s, &u, "unit_id")? != target || text(s, &u, "unit_object_id")? != unit.id {
                    return Err(err("position_of目标错绑"));
                }
                require_sources(object, &unit.sources)?;
            } else {
                current_entity(lookup, target.as_str(), "CC-016.center")?;
                if text(s, &w, "center_id")? != target {
                    return Err(err("position_against目标错绑"));
                }
                require_sources(object, &witness.sources)?;
            }
        }
        ("CC-055.pair_relation", "pair_from" | "pair_to", "CC-020.center_pair") => {
            let from = current_entity(lookup, source.as_str(), "CC-020.center_pair")?;
            let side = if relation == "pair_from" {
                "a_center_id"
            } else {
                "b_center_id"
            };
            if from.id != witness.id
                || source != entity
                || text(s, &w, side)? != target
                || version != revision(s, &w, "entity_revision")?
            {
                return Err(err("pair关系端点/版本错绑"));
            }
            audit_pair_sides(s, &w, lookup)?;
            require_sources(object, &witness.sources)?;
        }
        _ => return Err(err("结构关系类型/见证组合未声明")),
    }
    Ok(())
}
fn audit_pair_sides(
    s: &mut (impl SourceFieldStore + ?Sized),
    d: &RefValue,
    lookup: &impl Lookup,
) -> Result<(), String> {
    for (idname, revname) in [("a_center_id", "a_revision"), ("b_center_id", "b_revision")] {
        let id = text(s, d, idname)?;
        let side = current_entity(lookup, id.as_str(), "CC-016.center")?;
        let side = data(s, side)?;
        if revision(s, d, revname)? != revision(s, &side, "entity_revision")? {
            return Err(err("pair端点版本与当前中心不符"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod repair_tests {
    use super::tests::object;
    use super::*;
    struct Objects<'a>(&'a [Object]);
    impl Lookup for Objects<'_> {
        fn object(&self, id: &str) -> Option<&Object> {
            self.0.iter().find(|o| o.id.as_str() == id)
        }
        fn entity(&self, id: &str) -> Option<&Object> {
            self.0
                .iter()
                .find(|o| o.id.as_str().strip_prefix("obj3-") == Some(id))
        }
    }
    fn fixture(n: &mut sc::MemoryNodes) -> Vec<Object> {
        let unit = json!({"unit_id":"segment","unit_object_id":"obj3-segment"});
        vec![
            object(
                n,
                "CC-010.bi",
                "obj3-bi",
                json!("bi"),
                json!({"entity_id":"bi","start":{"source_coords":[]},"end":{"source_coords":[]}}),
            ),
            object(
                n,
                "CC-012.feature_sequence",
                "obj3-sequence",
                json!("sequence"),
                json!({"entity_id":"sequence","segment_id":"segment","role":"FIRST_SEQUENCE","facts":{"raw_elements":[{"stroke_idx":"0"}]},"stroke_refs":[{"stroke_index":"0","entity_id":"bi"}]}),
            ),
            object(
                n,
                "CC-013.segment",
                "obj3-segment",
                json!("segment"),
                json!({"entity_id":"segment","entity_revision":"3","sequence_id":"sequence","sequence_object_id":"obj3-sequence","second_sequence_id":null,"second_sequence_object_id":null}),
            ),
            object(
                n,
                "CC-016.center_seed_attempt",
                "obj3-attempt",
                json!("attempt"),
                json!({"entity_id":"attempt","units":[unit.clone()]}),
            ),
            object(
                n,
                "CC-016.center",
                "obj3-center",
                json!("center"),
                json!({"entity_id":"center","entity_revision":"5","status":"EXTENDING","members":["segment"]}),
            ),
            object(
                n,
                "CC-016.center",
                "obj3-center2",
                json!("center2"),
                json!({"entity_id":"center2","entity_revision":"7","status":"EXTENDING","members":["segment"]}),
            ),
            object(
                n,
                "CC-022.unit_assignment",
                "obj3-assignment",
                json!("assignment"),
                json!({"entity_id":"assignment","unit":unit.clone(),"left_center_id":"center","right_center_id":"center2"}),
            ),
            object(
                n,
                "CC-018.unit_core_position",
                "obj3-position",
                json!("position"),
                json!({"entity_id":"position","unit":unit,"center_id":"center","center_revision":"5"}),
            ),
            object(
                n,
                "CC-020.center_pair",
                "obj3-pair",
                json!("pair"),
                json!({"entity_id":"pair","entity_revision":"2","a_center_id":"center","a_revision":"5","b_center_id":"center2","b_revision":"7"}),
            ),
        ]
    }
    fn relation(
        n: &mut sc::MemoryNodes,
        kind: &str,
        source: &str,
        rel: &str,
        target: &str,
        version: &str,
        witness: &str,
    ) -> Object {
        object(
            n,
            kind,
            "obj3-edge",
            json!([source, rel, target]),
            json!({"source_id":source,"relation_kind":rel,"target_id":target,"version":version,"witness_object_id":witness}),
        )
    }
    #[test]
    fn all_emitted_relation_projections_reject_wrong_endpoint_version_kind_and_witness() {
        let mut n = sc::MemoryNodes::default();
        let objects = fixture(&mut n);
        let lookup = Objects(&objects);
        for (kind, src, rel, dst, version, witness) in [
            (
                "CC-055.segment_relation",
                "sequence",
                "derived_from",
                "segment",
                "3",
                "obj3-segment",
            ),
            (
                "CC-055.segment_relation",
                "bi",
                "member_of",
                "sequence",
                "3",
                "obj3-segment",
            ),
            (
                "CC-055.center_relation",
                "attempt",
                "attempted_from",
                "segment",
                "1",
                "obj3-attempt",
            ),
            (
                "CC-055.center_relation",
                "segment",
                "member_of",
                "center",
                "5",
                "obj3-center",
            ),
            (
                "CC-055.center_relation",
                "segment",
                "connects_left",
                "center",
                "5",
                "obj3-assignment",
            ),
            (
                "CC-055.center_relation",
                "segment",
                "connects_right",
                "center2",
                "7",
                "obj3-assignment",
            ),
            (
                "CC-055.pair_relation",
                "position",
                "position_of",
                "segment",
                "5",
                "obj3-position",
            ),
            (
                "CC-055.pair_relation",
                "position",
                "position_against",
                "center",
                "5",
                "obj3-position",
            ),
            (
                "CC-055.pair_relation",
                "pair",
                "pair_from",
                "center",
                "2",
                "obj3-pair",
            ),
            (
                "CC-055.pair_relation",
                "pair",
                "pair_to",
                "center2",
                "2",
                "obj3-pair",
            ),
        ] {
            let valid = relation(&mut n, kind, src, rel, dst, version, witness);
            audit_links(&mut n, &valid, &lookup).unwrap();
            for bad in [
                relation(&mut n, kind, src, rel, "missing", version, witness),
                relation(
                    &mut n,
                    kind,
                    src,
                    rel,
                    if dst == "center2" {
                        "center"
                    } else {
                        "center2"
                    },
                    version,
                    witness,
                ),
                relation(&mut n, kind, src, "undeclared", dst, version, witness),
                relation(&mut n, kind, src, rel, dst, "999", witness),
                relation(&mut n, kind, src, rel, dst, version, "obj3-sequence"),
            ] {
                assert!(audit_links(&mut n, &bad, &lookup).is_err(), "{kind}/{rel}");
            }
        }
    }
    #[test]
    fn missing_current_entity_rejects_relation_even_when_witness_remains() {
        let mut n = sc::MemoryNodes::default();
        let objects = fixture(&mut n);
        let edge = relation(
            &mut n,
            "CC-055.center_relation",
            "segment",
            "connects_left",
            "center",
            "5",
            "obj3-assignment",
        );
        audit_links(&mut n, &edge, &Objects(&objects)).unwrap();
        let without_center: Vec<_> = objects
            .into_iter()
            .filter(|o| o.id != "obj3-center")
            .collect();
        assert!(audit_links(&mut n, &edge, &Objects(&without_center)).is_err());
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_entity_removal_dirties_relation_and_rejects_dangling_endpoint() {
        let mut n = sc::MemoryNodes::default();
        let objects = fixture(&mut n);
        let edge = relation(
            &mut n,
            "CC-055.center_relation",
            "segment",
            "connects_left",
            "center",
            "5",
            "obj3-assignment",
        );
        super::super::source_contributions::entity_removal_control_for_test(
            &mut n, objects, edge, "center",
        )
        .unwrap();
    }
    #[test]
    fn changes_bind_event_generation_not_fact_generation_for_all_domains() {
        let mut n = sc::MemoryNodes::default();
        let raw = BusinessMap::<i64, Source>::plain();
        for kind in [
            "CC-055.segment_change",
            "CC-055.center_change",
            "CC-055.pair_change",
        ] {
            for event in ["formed", "updated", "withdrawn"] {
                let slot = json!(["28", "entity", event]);
                let after = if event == "withdrawn" {
                    Value::Null
                } else {
                    json!({"entity_id":"entity"})
                };
                let object_id = if event == "withdrawn" {
                    Value::Null
                } else {
                    json!("obj3-entity")
                };
                let o = object(
                    &mut n,
                    kind,
                    "obj3-change",
                    slot.clone(),
                    json!({"entity_id":"entity","change":event,"before":{"entity_id":"entity"},"after":after.clone(),"object_id":object_id.clone(),"known_at":{"generation":"28"}}),
                );
                let p = payload(&mut n, &o).unwrap();
                assert_eq!(text(&mut n, &p, "object_generation").unwrap(), "1");
                audit_payload(&mut n, kind, &p, (&slot).into(), &o.sources, &raw, 28).unwrap();
                assert!(
                    audit_payload(&mut n, kind, &p, (&slot).into(), &o.sources, &raw, 1).is_err()
                );
                assert!(
                    audit_payload(&mut n, kind, &p, (&slot).into(), &o.sources, &raw, 29).is_err()
                );
                let broken = object(
                    &mut n,
                    kind,
                    "obj3-change",
                    slot.clone(),
                    json!({"entity_id":"entity","change":event,"before":{"entity_id":"entity"},"after":after,"object_id":object_id,"known_at":{"generation":"27"}}),
                );
                let p = payload(&mut n, &broken).unwrap();
                assert!(
                    audit_payload(&mut n, kind, &p, (&slot).into(), &broken.sources, &raw, 28)
                        .is_err()
                );
            }
        }
    }
    #[test]
    fn change_cannot_attribute_real_after_to_another_entity() {
        let mut n = sc::MemoryNodes::default();
        let objects = fixture(&mut n);
        let lookup = Objects(&objects);
        let child = lookup.object("obj3-center").unwrap();
        let d = data(&mut n, child).unwrap();
        let mut bytes = Vec::new();
        n.render_ref(d.view(), &mut bytes).unwrap();
        let after: Value = serde_json::from_slice(&bytes).unwrap();
        for entity in ["center", "other"] {
            let c = object(
                &mut n,
                "CC-055.center_change",
                "obj3-change",
                json!(["28", entity, "formed"]),
                json!({"entity_id":entity,"change":"formed","before":null,"after":after.clone(),"object_id":"obj3-center","known_at":{"generation":"28"}}),
            );
            assert_eq!(audit_links(&mut n, &c, &lookup).is_ok(), entity == "center");
        }
    }
}

#[cfg(test)]
mod real_factory_repair_tests {
    use super::*;
    use crate::v3::content_structure::{
        build_cd, StructureContext, StructureEvidence, StructurePrior,
    };
    use newchan_rust::theta_v0::{
        config::ThetaConfig,
        parser::{parse_layer_with_inclusion_facts_store, second_fold::SecondFoldStore},
        types::Bar,
    };
    #[test]
    fn real_cd_factory_fact_one_event_28_then_warm_29_does_not_carry_old_changes() {
        let ledger: Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02d/raw-ledger.json"
        ))
        .unwrap();
        let mut seen = 0;
        for rows in ledger["cases"].as_object().unwrap().values() {
            let rows = rows.as_array().unwrap();
            let bars: Vec<_> = rows
                .iter()
                .enumerate()
                .map(|(i, r)| Bar {
                    source_index: i,
                    timestamp: i as i64,
                    open: r["open"].as_i64().unwrap(),
                    high: r["high"].as_i64().unwrap(),
                    low: r["low"].as_i64().unwrap(),
                    close: r["close"].as_i64().unwrap(),
                    volume: 1.0,
                    untradable: false,
                })
                .collect();
            let (layer, facts, fold) = parse_layer_with_inclusion_facts_store(
                &bars,
                &ThetaConfig::default(),
                crate::tb02c::second_store(&ThetaConfig::default().parse),
            );
            let evidence = StructureEvidence {
                segments: &layer.segments,
                segment_facts: facts.segment_facts.as_ref().unwrap(),
                stroke_facts: facts.stroke_facts.as_ref().unwrap(),
                second_store: &fold,
            };
            let mut n = sc::MemoryNodes::default();
            let mut producer =
                Cc004Producer::new(tb02a::PROFILE, crate::V3_STRUCTURE_RULE_REVISION);
            for i in 0..rows.len() {
                producer
                    .bind_source(
                        &mut n,
                        i as i64,
                        content_producer::RawRefSpec {
                            identity_key: format!("raw-{i}"),
                            payload_hash: "h".into(),
                            receipt_id: format!("receipt-{i}"),
                            event_id: format!("bar-{i}"),
                            input_revision: "1".into(),
                            revision: 1,
                            seq: i as i64,
                            source_coord: i.to_string(),
                        },
                    )
                    .unwrap();
            }
            let known=sc::write_value(&mut n,&cv(&json!({"generation":"28","input_frontier":"27","receipt_id":null,"received_at":null,"semantic_commit_ns":null})).unwrap()).unwrap();
            let sources: Vec<_> = (0..rows.len()).collect();
            let context = StructureContext {
                fact_generation: 1,
                generation: 28,
                cut: "cut-28",
                frontier: 27,
                known_at: &known,
                all_sources: &sources,
            };
            let output = build_cd(
                &mut producer,
                &mut n,
                &evidence,
                &StructurePrior {
                    segment_records: &[],
                    version_record: None,
                },
                &context,
            )
            .unwrap();
            let mut segments = Vec::new();
            let mut version = None;
            let raw = BusinessMap::<i64, Source>::plain();
            for o in &output.objects {
                let key: Value = serde_json::from_str(&o.fact_key).unwrap();
                match key[0].as_str().unwrap() {
                    "CC-013.segment" => segments.push(o.record.clone()),
                    "CC-056.segment_version" => version = Some(o.record.clone()),
                    "CC-055.segment_change" => {
                        seen += 1;
                        let p = RefValue::Plain(o.payload.content_ref());
                        let source = RefValue::Plain(o.source_coords.content_ref());
                        let sources = SourceCoords::Original(coords(&mut n, &source).unwrap());
                        assert_eq!(text(&mut n, &p, "object_generation").unwrap(), "1");
                        audit_payload(
                            &mut n,
                            "CC-055.segment_change",
                            &p,
                            (&key[1]).into(),
                            &sources,
                            &raw,
                            28,
                        )
                        .unwrap();
                        assert!(audit_payload(
                            &mut n,
                            "CC-055.segment_change",
                            &p,
                            (&key[1]).into(),
                            &sources,
                            &raw,
                            29
                        )
                        .is_err());
                    }
                    _ => {}
                }
            }
            let known=sc::write_value(&mut n,&cv(&json!({"generation":"29","input_frontier":"27","receipt_id":null,"received_at":null,"semantic_commit_ns":null})).unwrap()).unwrap();
            let warm = build_cd(
                &mut producer,
                &mut n,
                &evidence,
                &StructurePrior {
                    segment_records: &segments,
                    version_record: version.as_ref(),
                },
                &StructureContext {
                    generation: 29,
                    cut: "cut-29",
                    known_at: &known,
                    ..context
                },
            )
            .unwrap();
            assert!(
                warm.objects
                    .iter()
                    .all(|o| serde_json::from_str::<Value>(&o.fact_key).unwrap()[0]
                        != "CC-055.segment_change"),
                "同输入暖续跑不复制旧change"
            );
        }
        assert!(seen > 0, "必须实际覆盖工厂生成的change，不能空真");
    }
}
