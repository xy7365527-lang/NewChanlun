//! #1394 C §5.2–5.4：绑定同 cut 来源原件、typed/CC006 身份和见证。
//! 只读事实及其引用，不重新运行包含、分型或新笔判定。原始账必须先经 read_raw_events 核验。
use crate::{
    canonical_json,
    content_producer::{
        self, Cc004Producer, ShapeIdentity, CC004_STEP_KIND, CC005_GROUP_KIND, CC006_SHAPE_KIND,
        CC007_FRACTAL_KIND, CC054_KNOWLEDGE_KIND,
    },
    read_effective_events, stored_source_coord, tb02a, tb02a_facts, tb02b,
};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, PathStep},
    Built, ContentRef, Kind, NodeDigest, NodeSink, NodeSource, NodeStore, Value as V,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
fn err(e: impl std::fmt::Display) -> String {
    format!("SourceAuditInvalid：{e}")
}
fn f(k: &str) -> PathStep {
    PathStep::Field(k.into())
}
fn field(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    lookup_path(s, r, &[f(k)]).map_err(err)
}
fn inner(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    lookup_path(s, r, &[f(k), PathStep::JsonText]).map_err(err)
}
fn scalar(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<Value, String> {
    let r = field(s, r, k)?;
    if !matches!(
        r.kind,
        Kind::Number | Kind::String | Kind::Bool | Kind::Null
    ) {
        return Err(err("预期标量"));
    }
    let mut b = Vec::new();
    sc::render_canonical(s, &r, &mut b).map_err(err)?;
    newchan_rust::session_protocol::strict_json(&b)
}
fn text(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<String, String> {
    scalar(s, r, k)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| err(format!("{k}不是文本")))
}
fn number(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<i64, String> {
    scalar(s, r, k)?
        .as_i64()
        .ok_or_else(|| err(format!("{k}不是整数")))
}
fn decimal(v: &str) -> Result<i64, String> {
    newchan_rust::session_protocol::parse_canonical_i64(v, "source coordinate").map_err(err)
}
fn digest(r: &ContentRef) -> Result<NodeDigest, String> {
    r.digest().map_err(err)
}
fn built(r: ContentRef) -> Result<Built, String> {
    Ok(Built {
        digest: digest(&r)?,
        kind: r.kind,
        extent: r.extent,
    })
}
fn cv(v: &Value) -> Result<V, String> {
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(x) => V::Bool(*x),
        Value::String(x) => V::String(x.clone()),
        Value::Number(n) => V::Number(sc::NumberToken::parse(&n.to_string()).map_err(err)?),
        Value::Array(a) => V::Array(a.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(o) => V::Object(
            o.iter()
                .map(|(k, v)| Ok((k.clone(), cv(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
struct HashOnly;
impl NodeSink for HashOnly {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
fn hash(v: &Value) -> Result<NodeDigest, String> {
    Ok(sc::write_value(&mut HashOnly, &cv(v)?).map_err(err)?.digest)
}
fn exact(r: &ContentRef, kind: Kind, n: u64) -> Result<(), String> {
    if r.kind != kind || r.extent != Some(n) {
        Err(err(format!("字段/成员形状不符：预期{kind:?}/{n}")))
    } else {
        Ok(())
    }
}
fn elements(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Vec<ContentRef>, String> {
    if r.kind != Kind::Array {
        return Err(err("预期数组"));
    }
    (0..r.extent.ok_or_else(|| err("数组缺长度"))?)
        .map(|i| lookup_path(s, r, &[PathStep::Index(i)]).map_err(err))
        .collect()
}
fn coords(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Vec<i64>, String> {
    let mut out = Vec::new();
    for item in elements(s, r)? {
        if item.kind != Kind::String {
            return Err(err("源坐标必须为文本"));
        }
        let mut bytes = Vec::new();
        sc::render_canonical(s, &item, &mut bytes).map_err(err)?;
        let v: Value = newchan_rust::session_protocol::strict_json(&bytes)?;
        let n = decimal(v.as_str().ok_or_else(|| err("坐标不是文本"))?)?;
        if out.last().is_some_and(|prev| *prev >= n) {
            return Err(err("源坐标不是严格升序"));
        }
        out.push(n);
    }
    Ok(out)
}
struct Source {
    wire: NodeDigest,
    numeric: NodeDigest,
    raw: NodeDigest,
}
struct Object {
    id: String,
    kind: String,
    record: ContentRef,
    refs: ContentRef,
    sources: Vec<i64>,
}
struct Group {
    payload: ContentRef,
    members: Vec<i64>,
    sources: Vec<i64>,
    anchor: i64,
}
struct Shape {
    groups: [i64; 3],
}
#[derive(Debug)]
pub(super) struct Counts {
    pub objects: usize,
    pub groups: usize,
    pub windows: usize,
    pub effective: usize,
    pub insufficient: usize,
    pub outside_domain: usize,
}
fn references(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    raw: &BTreeMap<i64, Source>,
    numeric: bool,
) -> Result<Vec<i64>, String> {
    let mut out = Vec::new();
    for item in elements(s, r)? {
        let coord = decimal(&text(s, &item, "source_coord")?)?;
        let expected = raw
            .get(&coord)
            .ok_or_else(|| err("引用不是本cut生效来源"))?;
        if digest(&item)?
            != if numeric {
                expected.numeric
            } else {
                expected.wire
            }
        {
            return Err(err("源引用不是本cut的真实完整八字段原件"));
        }
        if out.last().is_some_and(|prev| *prev >= coord) {
            return Err(err("源引用不是严格升序"));
        }
        out.push(coord);
    }
    Ok(out)
}
/// raw_prefix 是同一次读集、截至本代 input_frontier 的独立原始账，不是manifest自己提供的值。
pub(super) fn audit_generated(
    s: &mut dyn NodeStore,
    state: &ContentRef,
    raw_prefix: &[Value],
    profile: &str,
    rule: &str,
) -> Result<Counts, String> {
    audit_generated_until(s, state, raw_prefix, profile, rule, None)
}
pub(super) fn audit_generated_until(
    s: &mut dyn NodeStore,
    state: &ContentRef,
    raw_prefix: &[Value],
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
) -> Result<Counts, String> {
    super::history::check_deadline(deadline)?;
    sc::verify(s, state).map_err(err)?;
    if profile != tb02a::PROFILE {
        return Err(err(
            "当前来源审计仅实现OHLC档；其他档仍待接线，拒绝按此口径放行",
        ));
    }
    let producer = Cc004Producer::new(profile, rule);
    let effective = crate::read_effective_events_until(raw_prefix, deadline)?;
    let mut raw = BTreeMap::new();
    for event in &effective {
        super::history::check_deadline(deadline)?;
        let coord = stored_source_coord(event)?;
        let wire = tb02a_facts::raw_ref(event);
        let mut numeric = wire.clone();
        numeric["revision"] = event["revision"].clone();
        numeric["seq"] = event["seq"].clone();
        raw.insert(
            coord,
            Source {
                wire: super::history::hash_json_until(&wire, deadline)?,
                numeric: super::history::hash_json_until(&numeric, deadline)?,
                raw: super::history::hash_json_until(event, deadline)?,
            },
        );
    }
    let object_array = field(s, state, "objects")?;
    let mut objects = Vec::new();
    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::new();
    for record in elements(s, &object_array)? {
        super::history::check_deadline(deadline)?;
        let id = text(s, &record, "object_id")?;
        let kind = text(s, &record, "kind")?;
        if !ids.insert(id.clone()) || number(s, &record, "object_revision")? != 1 {
            return Err(err("对象身份重复或修订不符"));
        }
        let refs = inner(s, &record, "input_refs_json")?;
        let source_ref = inner(s, &record, "source_coords_json")?;
        let sources = coords(s, &source_ref)?;
        if kind == CC006_SHAPE_KIND {
            exact(&record, Kind::Object, 15)?;
        } else {
            exact(&record, Kind::Object, 10)?;
            if ![
                CC004_STEP_KIND,
                CC005_GROUP_KIND,
                CC007_FRACTAL_KIND,
                CC054_KNOWLEDGE_KIND,
            ]
            .contains(&kind.as_str())
                && !tb02b::KINDS.contains(&kind.as_str())
            {
                return Err(err("未声明typed事实种类"));
            }
            let payload = inner(s, &record, "payload_json")?;
            if payload.kind != Kind::Object {
                return Err(err("typed载荷不是对象"));
            }
            if references(s, &refs, &raw, false)? != sources {
                return Err(err("typed引用与source_coords不同"));
            }
            let key = text(s, &record, "fact_key")?;
            let parsed = newchan_rust::session_protocol::strict_json(key.as_bytes())?;
            if canonical_json(&parsed) != key
                || parsed.as_array().is_none_or(|a| a.len() != 2)
                || parsed[0] != kind
            {
                return Err(err("typed槽位种类/规范文本不符"));
            }
            let expected = producer
                .typed_object_id(&kind, &key, &built(payload.clone())?, &built(refs.clone())?)
                .map_err(err)?;
            if id != expected {
                return Err(err("typed obj3身份与来源/载荷原件不符"));
            }
            if tb02b::KINDS.contains(&kind.as_str()) {
                for (k, want) in [
                    ("schema_revision", "s-new-bi/1"),
                    (
                        "semantic_version",
                        newchan_rust::theta_v0::parser::stroke::RULE,
                    ),
                    ("policy_version", "standard_raw_gap_3"),
                    ("view_role", "main"),
                ] {
                    if text(s, &payload, k)? != want {
                        return Err(err("新笔载荷语义头不同"));
                    }
                }
                if digest(&field(s, &payload, "slot")?)?
                    != super::history::hash_json_until(&parsed[1], deadline)?
                {
                    return Err(err("新笔payload槽位与fact_key不同"));
                }
            }
            if kind == CC005_GROUP_KIND {
                let index = decimal(&text(s, &payload, "group_index")?)?;
                let anchor = decimal(&text(s, &payload, "group_anchor")?)?;
                let members_root = field(s, &payload, "members")?;
                let members = coords(s, &members_root)?;
                if index < 0
                    || members.is_empty()
                    || !members.contains(&anchor)
                    || members.iter().any(|c| !raw.contains_key(c))
                {
                    return Err(err("组成员/锚/序号不符"));
                }
                let mut union = members.iter().copied().collect::<BTreeSet<_>>();
                for key in ["direction_evidence", "confirmation_evidence"] {
                    super::history::check_deadline(deadline)?;
                    let evidence = field(s, &payload, key)?;
                    if evidence.kind != Kind::Null {
                        let list = field(s, &evidence, "source_coords")?;
                        union.extend(coords(s, &list)?);
                    }
                }
                if union.into_iter().collect::<Vec<_>>() != sources {
                    return Err(err("组来源不等于成员及方向/封口依据的并集"));
                }
                if groups
                    .insert(
                        index,
                        Group {
                            payload: payload.clone(),
                            members,
                            sources: sources.clone(),
                            anchor,
                        },
                    )
                    .is_some()
                {
                    return Err(err("同cut组合并序号重复"));
                }
            }
        }
        objects.push(Object {
            id,
            kind,
            record,
            refs,
            sources,
        });
    }
    let mut partition = BTreeSet::new();
    for (expected, (index, group)) in groups.iter().enumerate() {
        super::history::check_deadline(deadline)?;
        if *index != expected as i64 {
            return Err(err("组合并序号不连续"));
        }
        for c in &group.members {
            super::history::check_deadline(deadline)?;
            if !partition.insert(*c) {
                return Err(err("同cut组成员重叠"));
            }
        }
    }
    if partition != raw.keys().copied().collect() {
        return Err(err("同cut组未完整覆盖生效原始账"));
    }
    let mut shapes = BTreeMap::new();
    for object in objects.iter().filter(|o| o.kind == CC006_SHAPE_KIND) {
        super::history::check_deadline(deadline)?;
        exact(&object.refs, Kind::Array, 3)?;
        let mut indices = [0; 3];
        let mut highs: [String; 3] = Default::default();
        let mut lows: [String; 3] = Default::default();
        let mut windows: [String; 3] = Default::default();
        let mut source_union = BTreeSet::new();
        for (slot, part) in elements(s, &object.refs)?.into_iter().enumerate() {
            super::history::check_deadline(deadline)?;
            exact(&part, Kind::Object, 3)?;
            let index = number(s, &part, "merged_index")?;
            indices[slot] = index;
            if slot > 0 && index != indices[slot - 1] + 1 {
                return Err(err("形态来源组不是连续三组"));
            }
            let group = groups
                .get(&index)
                .ok_or_else(|| err("形态引用没有同cut组事实"))?;
            let raw_refs = field(s, &part, "raw_refs")?;
            let members = references(s, &raw_refs, &raw, true)?;
            if members != group.members {
                return Err(err("形态raw_refs不是同cut组成员"));
            }
            let deps = field(s, &part, "dependency_refs")?;
            let dependency = references(s, &deps, &raw, true)?;
            let member_set = members.iter().copied().collect::<BTreeSet<_>>();
            let expected = group
                .sources
                .iter()
                .filter(|c| !member_set.contains(c))
                .copied()
                .collect::<Vec<_>>();
            if dependency != expected {
                return Err(err("形态dependency_refs不是组事实来源差集"));
            }
            source_union.extend(members);
            source_union.extend(dependency);
            windows[slot] = number(
                s,
                &object.record,
                ["window_start", "window_mid", "window_end"][slot],
            )?
            .to_string();
            if windows[slot] != group.anchor.to_string() {
                return Err(err("形态窗口不等于同cut组锚"));
            }
            highs[slot] = text(s, &group.payload, "high")?;
            lows[slot] = text(s, &group.payload, "low")?;
        }
        if source_union.into_iter().collect::<Vec<_>>() != object.sources {
            return Err(err("形态source_coords不是完整来源并集"));
        }
        let branch = text(s, &object.record, "branch")?;
        let dir_ab = text(s, &object.record, "dir_ab")?;
        let dir_bc = text(s, &object.record, "dir_bc")?;
        let comparison_text = text(s, &object.record, "comparisons_json")?;
        let comparisons = newchan_rust::session_protocol::strict_json(comparison_text.as_bytes())?;
        if canonical_json(&comparisons) != comparison_text
            || comparisons.as_array().is_none_or(|v| v.len() != 4)
        {
            return Err(err("形态比较不是规范四项"));
        }
        let comparisons = Built {
            digest: super::history::hash_json_until(&comparisons, deadline)?,
            kind: Kind::Array,
            extent: Some(4),
        };
        let input_refs = built(object.refs.clone())?;
        let identity = producer
            .shape_identity(ShapeIdentity {
                branch: &branch,
                dir_ab: &dir_ab,
                dir_bc: &dir_bc,
                window: windows,
                highs,
                lows,
                comparisons: &comparisons,
                input_refs: &input_refs,
            })
            .map_err(err)?;
        if content_producer::object_id_of(&identity) != object.id {
            return Err(err("CC006 obj3与同cut组事实身份不符"));
        }
        shapes.insert(object.id.clone(), Shape { groups: indices });
    }
    let witness_array = field(s, state, "witnesses")?;
    let mut seen = BTreeSet::new();
    for witness in elements(s, &witness_array)? {
        super::history::check_deadline(deadline)?;
        exact(&witness, Kind::Object, 9)?;
        let id = text(s, &witness, "witness_id")?;
        let object_id = text(s, &witness, "object_id")?;
        let slot = number(s, &witness, "slot")?;
        if !(0..3).contains(&slot) || !seen.insert((object_id.clone(), slot)) {
            return Err(err("见证槽位重复/越界"));
        }
        let shape = shapes
            .get(&object_id)
            .ok_or_else(|| err("见证没有同cut形态对象"))?;
        let mi = shape.groups[slot as usize];
        let group = &groups[&mi];
        if number(s, &witness, "merged_source_index")? != group.anchor {
            return Err(err("见证锚与同cut组不同"));
        }
        for (merged, field_name) in [
            ("merged_high", "high"),
            ("merged_low", "low"),
            ("merged_open", "open"),
            ("merged_close", "close"),
        ] {
            if field(s, &witness, merged)? != field(s, &group.payload, field_name)? {
                return Err(err("见证合并值不是同cut组事实"));
            }
        }
        let bars = field(s, &witness, "raw_bars")?;
        let items = elements(s, &bars)?;
        if items.len() != group.members.len() {
            return Err(err("见证raw_bars成员数不同"));
        }
        for (bar, coord) in items.iter().zip(&group.members) {
            super::history::check_deadline(deadline)?;
            if digest(bar)? != raw[coord].raw {
                return Err(err("见证raw_bars不是同cut真实原始成员"));
            }
        }
        let want = super::generation::witness_identity(
            s,
            &object_id,
            slot as usize,
            mi as usize,
            &built(bars)?,
        )?;
        if id != want {
            return Err(err("见证ID不符"));
        }
    }
    if seen.len() != shapes.len() * 3 {
        return Err(err("形态缺完整三槽见证"));
    }
    let observations = field(s, state, "observations")?;
    let mut insufficient = 0;
    let mut outside_domain = 0;
    for item in elements(s, &observations)? {
        super::history::check_deadline(deadline)?;
        match text(s, &item, "kind")?.as_str() {
            "insufficient_knowledge" => insufficient += 1,
            "domain_not_satisfied" => outside_domain += 1,
            _ => {}
        }
    }
    Ok(Counts {
        objects: objects.len(),
        groups: groups.len(),
        windows: groups.len().saturating_sub(2),
        effective: effective.len(),
        insufficient,
        outside_domain,
    })
}

#[cfg(test)]
mod tests {
    use super::super::generation::{GenerationInput, GenerationRuntime};
    use super::*;
    use crate::{
        content_bi::PriorState,
        meta_get_opt, open_db, read_raw_events,
        tests::{accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles},
    };
    use sc::MemoryNodes;
    const STATE: &[&str] = &[
        "generation",
        "structure_cut",
        "input_frontier",
        "catalog_revision",
        "profile_id",
        "profile_hash",
        "rule_revision",
        "raw_events",
        "effective_events",
        "objects",
        "witnesses",
        "relations",
        "observations",
        "scope",
    ];
    const TYPED: &[&str] = &[
        "object_id",
        "object_revision",
        "kind",
        "fact_key",
        "payload_json",
        "input_refs_json",
        "source_coords_json",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
    ];
    const SHAPE_FIELDS: &[&str] = &[
        "object_id",
        "object_revision",
        "kind",
        "branch",
        "window_start",
        "window_mid",
        "window_end",
        "dir_ab",
        "dir_bc",
        "comparisons_json",
        "input_refs_json",
        "source_coords_json",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
    ];
    const WITNESS: &[&str] = &[
        "witness_id",
        "object_id",
        "slot",
        "merged_source_index",
        "merged_high",
        "merged_low",
        "merged_open",
        "merged_close",
        "raw_bars",
    ];
    fn fixture() -> (MemoryNodes, ContentRef, Vec<Value>, String) {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        let c = open_db(&files.db()).unwrap();
        let raw = read_raw_events(&c).unwrap();
        let effective = read_effective_events(&raw).unwrap();
        let get = |k| meta_get_opt(&c, k).unwrap().unwrap();
        let profile_hash = get("profile_hash");
        let rule = get("rule_revision");
        let catalog = get("catalog_revision");
        let scope = serde_json::from_str(&get("scope")).unwrap();
        let mut store = MemoryNodes::default();
        let mut runtime = GenerationRuntime::new(
            format!("source-audit/{}", files.db().display()),
            tb02a::PROFILE,
            &rule,
        );
        let input = GenerationInput {
            all_events: &raw,
            effective: &effective,
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: raw.len() as i64 - 1,
            profile_id: tb02a::PROFILE,
            profile_hash: &profile_hash,
            rule_revision: &rule,
            catalog_revision: &catalog,
            scope: &scope,
            semantic_commit_ns: None,
            prior: PriorState {
                bi_records: vec![],
                version_record: None,
            },
        };
        let generated = runtime.generate(&mut store, &input).unwrap();
        let root = generated.generated_state.clone();
        (store, root, raw, rule)
    }
    fn replace_field(
        s: &mut dyn NodeStore,
        r: &ContentRef,
        names: &[&str],
        key: &str,
        replacement: V,
    ) -> ContentRef {
        let fields = names
            .iter()
            .map(|name| {
                (
                    (*name).into(),
                    if *name == key {
                        replacement.clone()
                    } else {
                        V::Edge(field(s, r, name).unwrap().digest().unwrap())
                    },
                )
            })
            .collect();
        sc::write_value(s, &V::Object(fields))
            .unwrap()
            .content_ref()
    }
    fn replace_member(
        s: &mut dyn NodeStore,
        state: &ContentRef,
        collection: &str,
        index: usize,
        record: ContentRef,
    ) -> ContentRef {
        let values = field(s, state, collection).unwrap();
        let mut elements = elements(s, &values).unwrap();
        elements[index] = record;
        replace_field(
            s,
            state,
            STATE,
            collection,
            V::Array(
                elements
                    .into_iter()
                    .map(|r| V::Edge(r.digest().unwrap()))
                    .collect(),
            ),
        )
    }
    #[test]
    fn real_ohlc_objects_and_witnesses_bind_to_sources_without_reclassification() {
        let (mut s, state, raw, rule) = fixture();
        let result = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap();
        assert!(result.objects > result.groups);
        assert!(result.groups > 3);
        assert_eq!(result.effective, 14);
        assert_eq!(result.windows, result.groups - 2);
        let error =
            audit_generated(&mut s, &state, &raw, tb02a::PROFILE, "other-rule").unwrap_err();
        assert!(error.contains("typed obj3身份"), "{error}");
    }
    #[test]
    fn valid_merkle_graph_with_changed_typed_id_or_shape_recipe_is_rejected() {
        let (mut s, state, raw, rule) = fixture();
        let root = field(&mut s, &state, "objects").unwrap();
        let all = elements(&mut s, &root).unwrap();
        let typed = all
            .iter()
            .position(|r| text(&mut s, r, "kind").unwrap() != CC006_SHAPE_KIND)
            .unwrap();
        let changed = replace_field(
            &mut s,
            &all[typed],
            TYPED,
            "object_id",
            V::String(format!("obj3-{}", "a".repeat(64))),
        );
        let forged = replace_member(&mut s, &state, "objects", typed, changed);
        sc::verify(&mut s, &forged).unwrap();
        assert!(
            audit_generated(&mut s, &forged, &raw, tb02a::PROFILE, &rule)
                .unwrap_err()
                .contains("typed obj3身份")
        );
        let shape = all
            .iter()
            .position(|r| text(&mut s, r, "kind").unwrap() == CC006_SHAPE_KIND)
            .unwrap();
        let branch = text(&mut s, &all[shape], "branch").unwrap();
        let replacement = if branch == "TOP" { "BOTTOM" } else { "TOP" };
        let changed = replace_field(
            &mut s,
            &all[shape],
            SHAPE_FIELDS,
            "branch",
            V::String(replacement.into()),
        );
        let forged = replace_member(&mut s, &state, "objects", shape, changed);
        sc::verify(&mut s, &forged).unwrap();
        assert!(
            audit_generated(&mut s, &forged, &raw, tb02a::PROFILE, &rule)
                .unwrap_err()
                .contains("CC006 obj3")
        );
    }
    #[test]
    fn witness_merged_claim_is_checked_against_group_fact_not_itself() {
        let (mut s, state, raw, rule) = fixture();
        let root = field(&mut s, &state, "witnesses").unwrap();
        let first = elements(&mut s, &root).unwrap().remove(0);
        // 原 witness ID 配方不含 merged_high；只改此值仍有合法 witness ID 和 Merkle 闭包。
        let changed = replace_field(
            &mut s,
            &first,
            WITNESS,
            "merged_high",
            V::String("999999".into()),
        );
        let forged = replace_member(&mut s, &state, "witnesses", 0, changed);
        sc::verify(&mut s, &forged).unwrap();
        let error = audit_generated(&mut s, &forged, &raw, tb02a::PROFILE, &rule).unwrap_err();
        assert!(error.contains("合并值不是同cut组事实"), "{error}");
    }
    #[test]
    fn witness_original_bytes_are_bound_even_if_reference_metadata_is_unchanged() {
        let (mut s, state, mut raw, rule) = fixture();
        raw[0]["high"] = json!("999999");
        let error = audit_generated(&mut s, &state, &raw, tb02a::PROFILE, &rule).unwrap_err();
        assert!(error.contains("raw_bars不是同cut真实原始成员"), "{error}");
    }
}
