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
fn scalar(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<Value, String> {
    let r = field(s, r, k)?;
    scalar_value(s, &r)
}
fn scalar_value(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Value, String> {
    if !matches!(
        r.kind,
        Kind::Number | Kind::String | Kind::Bool | Kind::Null
    ) {
        return Err(err("预期标量"));
    }
    let mut b = Vec::new();
    sc::render_canonical(s, r, &mut b).map_err(err)?;
    newchan_rust::session_protocol::strict_json(&b)
}
/// 仅当前 part/payload 的固定小字段；Some 由 ArrayAuditSource 关闭批读。
fn local_fields(
    s: &mut dyn NodeStore,
    r: &ContentRef,
    names: &[&str],
) -> Result<Option<super::object_index::Fields>, String> {
    if s.audit_full_array_scan() {
        super::object_index::Fields::read_subset(s, r, names)
            .map(Some)
            .map_err(err)
    } else {
        Ok(None)
    }
}
fn text(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<String, String> {
    let r = field(s, r, k)?;
    text_value(s, &r, k)
}
fn text_value(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<String, String> {
    scalar_value(s, r)?
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
pub(super) fn elements(s: &mut dyn NodeSource, r: &ContentRef) -> Result<Vec<ContentRef>, String> {
    if r.kind != Kind::Array {
        return Err(err("预期数组"));
    }
    if s.audit_full_array_scan() {
        return sc::path::lookup_array_items(s, r).map_err(err);
    }
    (0..r.extent.ok_or_else(|| err("数组缺长度"))?)
        .map(|i| lookup_path(s, r, &[PathStep::Index(i)]).map_err(err))
        .collect()
}

/// 活期仅为本次完整来源审计；预算/期限 Some 强制原路径。
struct ArrayAuditSource<'a> {
    source: &'a mut dyn NodeStore,
    full_arrays: bool,
    deadline: Option<std::time::Instant>,
}
impl NodeSource for ArrayAuditSource<'_> {
    fn audit_full_array_scan(&self) -> bool {
        self.full_arrays && self.deadline.is_none()
    }
    fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        self.source.read(d)
    }
    fn checked_read(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        self.source.checked_read(d)
    }
    fn before_path_index(&mut self, p: &[PathStep], i: u64) -> Result<(), sc::CodecError> {
        self.source.before_path_index(p, i)
    }
    fn max_render_bytes(&self) -> Option<u64> {
        self.source.max_render_bytes()
    }
}
impl NodeSink for ArrayAuditSource<'_> {
    fn put(&mut self, d: NodeDigest, t: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.source.put(d, t, body)
    }
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
    fields: super::object_index::Fields,
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
    checked: &mut BTreeMap<(NodeDigest, bool), i64>,
) -> Result<Vec<i64>, String> {
    let mut out = Vec::new();
    for item in elements(s, r)? {
        // elements每次仍核实际成员头；仅本次已全核state、固定raw读集内复用
        // 首次完整八字段原件匹配。未知引用仍走原坐标读取及独立原件对拍。
        let key = (digest(&item)?, numeric);
        let coord = if let Some(coord) = checked.get(&key) {
            *coord
        } else {
            let coord = decimal(&text(s, &item, "source_coord")?)?;
            let expected = raw
                .get(&coord)
                .ok_or_else(|| err("引用不是本cut生效来源"))?;
            if key.0
                != if numeric {
                    expected.numeric
                } else {
                    expected.wire
                }
            {
                return Err(err("源引用不是本cut的真实完整八字段原件"));
            }
            checked.insert(key, coord);
            coord
        };
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
    audit_generated_metered(s, state, raw_prefix, profile, rule, None, None)
}
pub(super) fn audit_generated_until(
    s: &mut dyn NodeStore,
    state: &ContentRef,
    raw_prefix: &[Value],
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
) -> Result<Counts, String> {
    audit_generated_metered(s, state, raw_prefix, profile, rule, deadline, None)
}
pub(super) fn audit_generated_metered(
    s: &mut dyn NodeStore,
    state: &ContentRef,
    raw_prefix: &[Value],
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Counts, String> {
    audit_generated_in_store(
        &mut super::audit_store::StoreView::generic(s),
        state,
        raw_prefix,
        profile,
        rule,
        deadline,
        budget,
    )
}
pub(super) fn audit_generated_for_audit(
    s: &mut super::history::HashReader<'_>,
    state: &ContentRef,
    raw_prefix: &[Value],
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Counts, String> {
    audit_generated_in_store(
        &mut super::audit_store::StoreView::for_audit(s),
        state,
        raw_prefix,
        profile,
        rule,
        deadline,
        budget,
    )
}
fn audit_generated_in_store(
    s: &mut super::audit_store::StoreView<'_, '_>,
    state: &ContentRef,
    raw_prefix: &[Value],
    profile: &str,
    rule: &str,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Counts, String> {
    super::history::check_deadline(deadline)?;
    #[cfg(test)]
    super::state::trace_event("source_closure", None);
    s.verify_public(state).map_err(err)?;
    let mut array_source = ArrayAuditSource {
        source: s,
        full_arrays: budget.is_none(),
        deadline,
    };
    let s: &mut dyn NodeStore = &mut array_source;
    #[cfg(test)]
    super::state::trace_event("source_raw", None);
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
                wire: super::history::hash_json_metered(&wire, deadline, budget)?,
                numeric: super::history::hash_json_metered(&numeric, deadline, budget)?,
                raw: super::history::hash_json_metered(event, deadline, budget)?,
            },
        );
    }
    let object_array = field(s, state, "objects")?;
    #[cfg(test)]
    super::state::trace_event("source_objects", None);
    // 活期只覆盖当前一次audit_generated；不在代际、事务或源修订之间带出。
    let mut checked_refs = BTreeMap::new();
    let mut objects = Vec::new();
    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::new();
    for record in elements(s, &object_array)? {
        super::history::check_deadline(deadline)?;
        let kind = text(s, &record, "kind")?;
        let mut names = vec![
            "object_id",
            "kind",
            "object_revision",
            "input_refs_json",
            "source_coords_json",
        ];
        if kind == CC006_SHAPE_KIND {
            names.extend([
                "branch",
                "dir_ab",
                "dir_bc",
                "comparisons_json",
                "window_start",
                "window_mid",
                "window_end",
            ]);
        } else {
            names.extend(["payload_json", "fact_key"]);
        }
        let fields = super::object_index::Fields::read_subset(s, &record, &names).map_err(err)?;
        let id = fields.text(s, "object_id").map_err(err)?;
        if !ids.insert(id.clone()) || fields.integer(s, "object_revision").map_err(err)? != 1 {
            return Err(err("对象身份重复或修订不符"));
        }
        let refs = lookup_path(
            s,
            fields.get("input_refs_json").map_err(err)?,
            &[PathStep::JsonText],
        )
        .map_err(err)?;
        let source_ref = lookup_path(
            s,
            fields.get("source_coords_json").map_err(err)?,
            &[PathStep::JsonText],
        )
        .map_err(err)?;
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
            let payload = lookup_path(
                s,
                fields.get("payload_json").map_err(err)?,
                &[PathStep::JsonText],
            )
            .map_err(err)?;
            if payload.kind != Kind::Object {
                return Err(err("typed载荷不是对象"));
            }
            if references(s, &refs, &raw, false, &mut checked_refs)? != sources {
                return Err(err("typed引用与source_coords不同"));
            }
            let key = fields.text(s, "fact_key").map_err(err)?;
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
                    != super::history::hash_json_metered(&parsed[1], deadline, budget)?
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
            fields,
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
    #[cfg(test)]
    super::state::trace_event("source_shapes", None);
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
            let part_fields =
                local_fields(s, &part, &["merged_index", "raw_refs", "dependency_refs"])?;
            let index = if let Some(fields) = &part_fields {
                scalar_value(s, fields.get("merged_index").map_err(err)?)?
                    .as_i64()
                    .ok_or_else(|| err("merged_index不是整数"))?
            } else {
                number(s, &part, "merged_index")?
            };
            indices[slot] = index;
            if slot > 0 && index != indices[slot - 1] + 1 {
                return Err(err("形态来源组不是连续三组"));
            }
            let group = groups
                .get(&index)
                .ok_or_else(|| err("形态引用没有同cut组事实"))?;
            let raw_refs = if let Some(fields) = &part_fields {
                fields.get("raw_refs").map_err(err)?.clone()
            } else {
                field(s, &part, "raw_refs")?
            };
            let members = references(s, &raw_refs, &raw, true, &mut checked_refs)?;
            if members != group.members {
                return Err(err("形态raw_refs不是同cut组成员"));
            }
            let deps = if let Some(fields) = &part_fields {
                fields.get("dependency_refs").map_err(err)?.clone()
            } else {
                field(s, &part, "dependency_refs")?
            };
            let dependency = references(s, &deps, &raw, true, &mut checked_refs)?;
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
            windows[slot] = object
                .fields
                .integer(s, ["window_start", "window_mid", "window_end"][slot])
                .map_err(err)?
                .to_string();
            if windows[slot] != group.anchor.to_string() {
                return Err(err("形态窗口不等于同cut组锚"));
            }
            if let Some(fields) = local_fields(s, &group.payload, &["high", "low"])? {
                highs[slot] = text_value(s, fields.get("high").map_err(err)?, "high")?;
                lows[slot] = text_value(s, fields.get("low").map_err(err)?, "low")?;
            } else {
                highs[slot] = text(s, &group.payload, "high")?;
                lows[slot] = text(s, &group.payload, "low")?;
            }
        }
        if source_union.into_iter().collect::<Vec<_>>() != object.sources {
            return Err(err("形态source_coords不是完整来源并集"));
        }
        let branch = object.fields.text(s, "branch").map_err(err)?;
        let dir_ab = object.fields.text(s, "dir_ab").map_err(err)?;
        let dir_bc = object.fields.text(s, "dir_bc").map_err(err)?;
        let comparison_text = object.fields.text(s, "comparisons_json").map_err(err)?;
        let comparisons = newchan_rust::session_protocol::strict_json(comparison_text.as_bytes())?;
        if canonical_json(&comparisons) != comparison_text
            || comparisons.as_array().is_none_or(|v| v.len() != 4)
        {
            return Err(err("形态比较不是规范四项"));
        }
        let comparisons = Built {
            digest: super::history::hash_json_metered(&comparisons, deadline, budget)?,
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
    #[cfg(test)]
    super::state::trace_event("source_witnesses", None);
    let mut seen = BTreeSet::new();
    for witness in elements(s, &witness_array)? {
        super::history::check_deadline(deadline)?;
        exact(&witness, Kind::Object, 9)?;
        let fields = super::object_index::Fields::read_subset(
            s,
            &witness,
            &[
                "witness_id",
                "object_id",
                "slot",
                "merged_source_index",
                "merged_high",
                "merged_low",
                "merged_open",
                "merged_close",
                "raw_bars",
            ],
        )
        .map_err(err)?;
        let id = fields.text(s, "witness_id").map_err(err)?;
        let object_id = fields.text(s, "object_id").map_err(err)?;
        let slot = fields.integer(s, "slot").map_err(err)?;
        if !(0..3).contains(&slot) || !seen.insert((object_id.clone(), slot)) {
            return Err(err("见证槽位重复/越界"));
        }
        let shape = shapes
            .get(&object_id)
            .ok_or_else(|| err("见证没有同cut形态对象"))?;
        let mi = shape.groups[slot as usize];
        let group = &groups[&mi];
        if fields.integer(s, "merged_source_index").map_err(err)? != group.anchor {
            return Err(err("见证锚与同cut组不同"));
        }
        let payload_fields = local_fields(s, &group.payload, &["high", "low", "open", "close"])?;
        for (merged, field_name) in [
            ("merged_high", "high"),
            ("merged_low", "low"),
            ("merged_open", "open"),
            ("merged_close", "close"),
        ] {
            let actual = fields.get(merged).map_err(err)?;
            let expected = if let Some(fields) = &payload_fields {
                fields.get(field_name).map_err(err)?.clone()
            } else {
                field(s, &group.payload, field_name)?
            };
            if actual != &expected {
                return Err(err("见证合并值不是同cut组事实"));
            }
        }
        let bars = fields.get("raw_bars").map_err(err)?.clone();
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
    #[cfg(test)]
    super::state::trace_event("source_observations", None);
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
    struct Reads<'a> {
        source: &'a mut dyn NodeSource,
        digests: Vec<NodeDigest>,
    }
    impl NodeSource for Reads<'_> {
        fn read(&mut self, d: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.digests.push(*d);
            self.source.read(d)
        }
        fn checked_read(
            &mut self,
            d: &NodeDigest,
        ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
            self.digests.push(*d);
            self.source.checked_read(d)
        }
    }
    #[derive(Default)]
    struct Meter {
        events: std::cell::RefCell<Vec<String>>,
        stop_at: Option<usize>,
    }
    impl super::super::audit_budget::Budget for Meter {
        fn precheck(&self) -> Result<(), sc::CodecError> {
            self.events.borrow_mut().push("precheck".into());
            if self.stop_at == Some(self.events.borrow().len()) {
                Err(sc::CodecError::new("field_stop", "当前预算门拒绝"))
            } else {
                Ok(())
            }
        }
        fn prepare_node(&self) -> Result<(), sc::CodecError> {
            self.precheck()?;
            self.events.borrow_mut().push("prepare".into());
            Ok(())
        }
        fn accept_prepared_frame(
            &self,
            frame: &sc::job::CheckedFrame,
        ) -> Result<(), sc::CodecError> {
            self.events
                .borrow_mut()
                .push(sc::root_string(frame.digest()));
            Ok(())
        }
        fn node(&self, d: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
            self.prepare_node()?;
            self.accept_prepared_frame(&sc::job::CheckedFrame::checked(d, tag, body.to_vec())?)
        }
        fn record(&self) -> Result<(), sc::CodecError> {
            self.precheck()?;
            self.events.borrow_mut().push("record".into());
            Ok(())
        }
    }
    // 与生产顺序一致：part 先整数，再两内容边；shape payload 先 high 文本再 low；
    // witness payload 只逐项 Ref 对拍，不额外 render。
    fn read_local(
        s: &mut dyn NodeStore,
        root: &ContentRef,
        names: &[&str],
        legacy: bool,
    ) -> Result<Vec<(ContentRef, Option<Value>)>, String> {
        let fields = if legacy {
            None
        } else {
            local_fields(s, root, names)?
        };
        names
            .iter()
            .map(|name| {
                let r = if let Some(fields) = &fields {
                    fields.get(name).map_err(err)?.clone()
                } else {
                    field(s, root, name)?
                };
                let value = if *name == "merged_index" || names.len() == 2 {
                    let value = scalar_value(s, &r)?;
                    if *name == "merged_index" {
                        value.as_i64().ok_or_else(|| err("merged_index不是整数"))?;
                    } else {
                        value
                            .as_str()
                            .ok_or_else(|| err(format!("{name}不是文本")))?;
                    }
                    Some(value)
                } else {
                    None
                };
                Ok((r, value))
            })
            .collect()
    }
    fn local_roots(s: &mut MemoryNodes, state: &ContentRef) -> (ContentRef, ContentRef) {
        let objects = field(s, state, "objects").unwrap();
        let objects = elements(s, &objects).unwrap();
        let shape = objects
            .iter()
            .find(|r| text(s, r, "kind").unwrap() == CC006_SHAPE_KIND)
            .unwrap();
        let refs = field(s, shape, "input_refs_json").unwrap();
        let refs = lookup_path(s, &refs, &[PathStep::JsonText]).unwrap();
        let part = elements(s, &refs).unwrap().remove(0);
        let index = number(s, &part, "merged_index").unwrap();
        let payload = objects
            .iter()
            .find_map(|r| {
                if text(s, r, "kind").unwrap() == CC005_GROUP_KIND {
                    let payload = field(s, r, "payload_json").unwrap();
                    let payload = lookup_path(s, &payload, &[PathStep::JsonText]).unwrap();
                    if text(s, &payload, "group_index").unwrap() == index.to_string() {
                        return Some(payload);
                    }
                }
                None
            })
            .unwrap();
        (part, payload)
    }
    #[test]
    fn local_none_subsets_keep_ordered_refs_types_values_and_reduce_real_reads() {
        let (mut nodes, state, _, _) = fixture();
        let (part, payload) = local_roots(&mut nodes, &state);
        let mut db = super::super::commit::tests::fixture();
        let tx = db.conn.transaction().unwrap();
        let mut sink = crate::content_store::TxStore::new(&tx).unwrap();
        for (d, tag, body) in nodes.iter() {
            sink.put(*d, tag, body).unwrap();
        }
        for (root, names) in [
            (&part, &["merged_index", "raw_refs", "dependency_refs"][..]),
            (&payload, &["high", "low"][..]),
            (&payload, &["high", "low", "open", "close"][..]),
        ] {
            let run = |legacy| {
                let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
                let mut reads = Reads {
                    source: &mut source,
                    digests: vec![],
                };
                let sql = crate::content_store::sql_reads_on_thread();
                let frames = sc::frame_calls_on_thread();
                let mut reader = super::super::history::HashReader::new(&mut reads);
                let mut s = ArrayAuditSource {
                    source: &mut reader,
                    full_arrays: true,
                    deadline: None,
                };
                let result = read_local(&mut s, root, names, legacy).unwrap();
                (
                    result,
                    crate::content_store::sql_reads_on_thread() - sql,
                    sc::frame_calls_on_thread() - frames,
                    reads.digests,
                )
            };
            let old = run(true);
            let actual = run(false);
            assert_eq!(
                actual.0, old.0,
                "顺序、ContentRef、kind、extent及render值一致"
            );
            let root_digest = digest(root).unwrap();
            let roots = |ds: &[NodeDigest]| ds.iter().filter(|d| **d == root_digest).count();
            assert_eq!(roots(&old.3), names.len(), "旧路径每字段重复根寻路");
            eprintln!(
                "local {names:?}: SQL {} -> {}, F {} -> {}, root {} -> {}",
                old.1,
                actual.1,
                old.2,
                actual.2,
                roots(&old.3),
                roots(&actual.3)
            );
            assert_eq!(roots(&actual.3), 1, "None 同一局部根只批读一次");
            assert!(actual.1 < old.1, "必须减少真实SQL读取");
            assert_eq!(actual.2, actual.1, "每个实际节点读仍核本次F");
        }
    }
    #[test]
    fn local_some_keeps_sql_f_budget_error_order_and_deadline_before_sql() {
        let (mut nodes, state, _, _) = fixture();
        let (part, payload) = local_roots(&mut nodes, &state);
        let wrong_index = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("merged_index".into(), V::String("not-an-integer".into())),
                ("raw_refs".into(), V::Null),
                ("dependency_refs".into(), V::Null),
            ]),
        )
        .unwrap()
        .content_ref();
        let wrong_high_missing_low =
            sc::write_value(&mut nodes, &V::Object(vec![("high".into(), V::Bool(true))]))
                .unwrap()
                .content_ref();
        sc::verify(&mut nodes, &wrong_index).unwrap();
        sc::verify(&mut nodes, &wrong_high_missing_low).unwrap();
        let mut db = super::super::commit::tests::fixture();
        let tx = db.conn.transaction().unwrap();
        let mut sink = crate::content_store::TxStore::new(&tx).unwrap();
        for (d, tag, body) in nodes.iter() {
            sink.put(*d, tag, body).unwrap();
        }
        for (root, names) in [
            (&part, &["merged_index", "raw_refs", "dependency_refs"][..]),
            (&payload, &["high", "low"][..]),
            (&payload, &["high", "low", "open", "close"][..]),
            (
                &wrong_index,
                &["merged_index", "raw_refs", "dependency_refs"][..],
            ),
            (&wrong_high_missing_low, &["high", "low"][..]),
        ] {
            let run = |legacy, stop_at, deadline| {
                let meter = Meter {
                    stop_at,
                    ..Meter::default()
                };
                let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
                source.enable_audit_same_material();
                let sql = crate::content_store::sql_reads_on_thread();
                let frames = sc::frame_calls_on_thread();
                let mut reader = super::super::history::HashReader::metered(
                    &mut source,
                    deadline,
                    None,
                    Some(&meter),
                );
                let mut s = ArrayAuditSource {
                    source: &mut reader,
                    full_arrays: false,
                    deadline,
                };
                let result = read_local(&mut s, root, names, legacy);
                (
                    result,
                    crate::content_store::sql_reads_on_thread() - sql,
                    sc::frame_calls_on_thread() - frames,
                    meter.events.into_inner(),
                )
            };
            let old = run(true, None, None);
            assert_eq!(run(false, None, None), old);
            if root == &wrong_index {
                assert_eq!(old.0, Err(err("merged_index不是整数")));
            } else if root == &wrong_high_missing_low {
                assert_eq!(
                    old.0,
                    Err(err("high不是文本")),
                    "Some须先拒绝high类型，不能提前读取缺失的low"
                );
            }
            assert_eq!(old.2, old.1 * 2, "Some 保持旧SQL源F和HashReader F");
            // 遍历旧成功轨迹的每一道预算前门，包含跨字段/render的即时读取交错。
            for (at, event) in old.3.iter().enumerate() {
                if event == "precheck" {
                    assert_eq!(
                        run(false, Some(at + 1), None),
                        run(true, Some(at + 1), None),
                        "Some stop_at={} 必须保持Ref/错误、SQL、F和预算门顺序",
                        at + 1
                    );
                }
            }
            let deadline = Some(std::time::Instant::now());
            let expired = run(false, None, deadline);
            assert_eq!(expired, run(true, None, deadline));
            assert!(expired.0.unwrap_err().contains("verify_deadline"));
            assert_eq!((expired.1, expired.2), (0, 0));
            assert!(expired.3.is_empty(), "期限门先于预算门与SQL");
        }
    }
    #[test]
    fn local_subset_budget_and_deadline_opt_in_keep_legacy_sql_f_and_gate_order() {
        let mut db = super::super::commit::tests::fixture();
        let tx = db.conn.transaction().unwrap();
        let names = (0..15).map(|i| format!("field-{i:02}")).collect::<Vec<_>>();
        let keys = names[..10].iter().map(String::as_str).collect::<Vec<_>>();
        let root = sc::write_value(
            &mut crate::content_store::TxStore::new(&tx).unwrap(),
            &V::Object(names.iter().map(|k| (k.clone(), V::Null)).collect()),
        )
        .unwrap()
        .content_ref();
        let run = |legacy, budget_some, deadline, stop_at| {
            let meter = Meter {
                stop_at,
                ..Meter::default()
            };
            let mut source = crate::content_store::ReadStore::new(&tx).unwrap();
            source.enable_audit_same_material();
            let mut reads = Reads {
                source: &mut source,
                digests: vec![],
            };
            let sql = crate::content_store::sql_reads_on_thread();
            let frames = sc::frame_calls_on_thread();
            let mut reader = super::super::history::HashReader::metered(
                &mut reads,
                deadline,
                None,
                if budget_some { Some(&meter) } else { None },
            );
            reader.enable_full_arrays_for_test();
            assert!(!reader.audit_full_array_scan());
            let mut s = ArrayAuditSource {
                source: &mut reader,
                full_arrays: !budget_some,
                deadline,
            };
            assert!(!s.audit_full_array_scan());
            let fields = if legacy {
                sc::path::lookup_object_fields_subset(&mut s, &root, &keys).map_err(err)
            } else {
                // 上层保留逐字段业务顺序；同时锁低层 Subset 不受本次优化影响。
                assert!(local_fields(&mut s, &root, &keys).unwrap().is_none());
                sc::path::lookup_object_fields_subset(&mut s, &root, &keys).map_err(err)
            };
            (
                fields,
                crate::content_store::sql_reads_on_thread() - sql,
                sc::frame_calls_on_thread() - frames,
                reads.digests,
                meter.events.into_inner(),
            )
        };
        for (budget, deadline) in [
            (true, None),
            (
                false,
                Some(std::time::Instant::now() + std::time::Duration::from_secs(3600)),
            ),
            (
                true,
                Some(std::time::Instant::now() + std::time::Duration::from_secs(3600)),
            ),
        ] {
            let old = run(true, budget, deadline, None);
            assert_eq!(run(false, budget, deadline, None), old);
            assert!(old.0.is_ok(), "Some 原 Subset 结果：{:?}", old.0);
            eprintln!(
                "SUBSET_GATE budget={budget} deadline={} SQL={} F={} reads={}",
                deadline.is_some(),
                old.1,
                old.2,
                old.3.len()
            );
            assert_eq!(old.1, 199, "Some 的真实 SQL 读取保持原 15取10");
            // 期限 Some/预算 None 的旧源仍逐次 SQL 读，但同素材仅核 91 次纯 F。
            assert_eq!(old.2, if budget { 398 } else { 91 });
            if budget {
                for (at, event) in old.4.iter().enumerate() {
                    if event == "precheck" {
                        assert_eq!(
                            run(false, budget, deadline, Some(at + 1)),
                            run(true, budget, deadline, Some(at + 1))
                        );
                    }
                }
            }
        }
        let expired = Some(std::time::Instant::now());
        for budget in [false, true] {
            let old = run(true, budget, expired, None);
            assert_eq!(run(false, budget, expired, None), old);
            assert!(old.0.unwrap_err().contains("verify_deadline"));
            assert_eq!((old.1, old.2), (0, 0));
            assert!(old.3.is_empty() && old.4.is_empty());
        }
    }
    #[test]
    fn full_some_audit_keeps_frozen_read_and_budget_trace() {
        let (mut nodes, state, raw, rule) = fixture();
        let mut reads = Reads {
            source: &mut nodes,
            digests: vec![],
        };
        let meter = Meter::default();
        let frames = sc::frame_calls_on_thread();
        let mut reader =
            super::super::history::HashReader::metered(&mut reads, None, None, Some(&meter));
        let counts = audit_generated_metered(
            &mut reader,
            &state,
            &raw,
            tb02a::PROFILE,
            &rule,
            None,
            Some(&meter),
        )
        .unwrap();
        let frames = sc::frame_calls_on_thread() - frames;
        let roots = reads
            .digests
            .iter()
            .map(sc::root_string)
            .collect::<Vec<_>>();
        let read_hash = crate::sha256_hex(canonical_json(&json!(roots)).as_bytes());
        let budget_hash =
            crate::sha256_hex(canonical_json(&json!(meter.events.into_inner())).as_bytes());
        eprintln!("Some full: counts={counts:?}, reads={}, F={frames}, read_hash={read_hash}, budget_hash={budget_hash}", roots.len());
        // dc425df7 固定fixture的旧路径实测；不是由候选路径重算期望。
        assert_eq!(
            (
                counts.objects,
                counts.groups,
                counts.windows,
                counts.effective,
                counts.insufficient,
                counts.outside_domain
            ),
            (76, 13, 11, 14, 0, 0)
        );
        assert_eq!(roots.len(), 151351, "固定基线的完整Some读取轨迹");
        assert_eq!(frames, 170797, "固定基线的完整Some F次数");
        assert_eq!(
            read_hash,
            "668e022e318e445f163d23036d33cccefb5e735a82ae97ce6db8b453b936d69a"
        );
        assert_eq!(
            budget_hash,
            "dc5c1b67c63046ecb4edf84c12e84c784793112fff94bc4a449cef1c8befd1b2"
        );
    }
    fn json_text_edge(s: &mut dyn NodeStore, value: &ContentRef) -> ContentRef {
        let body = digest(value).unwrap();
        let d = sc::frame(sc::tag::JSON_TEXT, &body);
        s.put(d, sc::tag::JSON_TEXT, &body).unwrap();
        ContentRef::new(Kind::JsonText, &d, None)
    }
    fn assert_rejected_in_both_modes(
        s: &mut MemoryNodes,
        state: &ContentRef,
        raw: &[Value],
        rule: &str,
        reason: &str,
    ) {
        sc::verify(s, state).unwrap();
        let none = audit_generated(s, state, raw, tb02a::PROFILE, rule).unwrap_err();
        let meter = Meter::default();
        let mut reader = super::super::history::HashReader::metered(s, None, None, Some(&meter));
        let some = audit_generated_metered(
            &mut reader,
            state,
            raw,
            tb02a::PROFILE,
            rule,
            None,
            Some(&meter),
        )
        .unwrap_err();
        assert!(none.contains(reason), "{none}");
        assert_eq!(none, some, "合法Merkle畸形在None/Some均须由原业务判据拒绝");
    }
    #[test]
    fn valid_merkle_shape_dependency_forgery_is_rejected_after_none_subset() {
        let (mut s, state, raw, rule) = fixture();
        let objects = field(&mut s, &state, "objects").unwrap();
        let objects = elements(&mut s, &objects).unwrap();
        let shape = objects
            .iter()
            .position(|r| text(&mut s, r, "kind").unwrap() == CC006_SHAPE_KIND)
            .unwrap();
        let refs = field(&mut s, &objects[shape], "input_refs_json").unwrap();
        let refs = lookup_path(&mut s, &refs, &[PathStep::JsonText]).unwrap();
        let mut parts = elements(&mut s, &refs).unwrap();
        let members = field(&mut s, &parts[0], "raw_refs").unwrap();
        // 真成员不是来源差集；不引入坏节点/缺节点，必须由差集判据拒绝。
        parts[0] = replace_field(
            &mut s,
            &parts[0],
            &["merged_index", "raw_refs", "dependency_refs"],
            "dependency_refs",
            V::Edge(digest(&members).unwrap()),
        );
        let refs = sc::write_value(
            &mut s,
            &V::Array(parts.iter().map(|r| V::Edge(digest(r).unwrap())).collect()),
        )
        .unwrap()
        .content_ref();
        let refs = json_text_edge(&mut s, &refs);
        let changed = replace_field(
            &mut s,
            &objects[shape],
            SHAPE_FIELDS,
            "input_refs_json",
            V::Edge(digest(&refs).unwrap()),
        );
        let forged = replace_member(&mut s, &state, "objects", shape, changed);
        assert_rejected_in_both_modes(
            &mut s,
            &forged,
            &raw,
            &rule,
            "dependency_refs不是组事实来源差集",
        );
    }
    #[test]
    fn valid_merkle_reidentified_group_high_still_breaks_shape_identity() {
        let (mut s, state, raw, rule) = fixture();
        let (_, payload) = local_roots(&mut s, &state);
        let objects = field(&mut s, &state, "objects").unwrap();
        let objects = elements(&mut s, &objects).unwrap();
        let group = objects
            .iter()
            .position(|r| {
                if text(&mut s, r, "kind").unwrap() != CC005_GROUP_KIND {
                    return false;
                }
                let p = field(&mut s, r, "payload_json").unwrap();
                lookup_path(&mut s, &p, &[PathStep::JsonText]).unwrap() == payload
            })
            .unwrap();
        let mut bytes = Vec::new();
        sc::render_canonical(&mut s, &payload, &mut bytes).unwrap();
        let mut value: Value = newchan_rust::session_protocol::strict_json(&bytes).unwrap();
        value["high"] = json!("999999");
        let changed_payload = sc::write_value(&mut s, &cv(&value).unwrap())
            .unwrap()
            .content_ref();
        let refs = field(&mut s, &objects[group], "input_refs_json").unwrap();
        let refs = lookup_path(&mut s, &refs, &[PathStep::JsonText]).unwrap();
        let key = text(&mut s, &objects[group], "fact_key").unwrap();
        let id = Cc004Producer::new(tb02a::PROFILE, &rule)
            .typed_object_id(
                CC005_GROUP_KIND,
                &key,
                &built(changed_payload.clone()).unwrap(),
                &built(refs).unwrap(),
            )
            .unwrap();
        let payload_json = json_text_edge(&mut s, &changed_payload);
        let changed = replace_field(
            &mut s,
            &objects[group],
            TYPED,
            "payload_json",
            V::Edge(digest(&payload_json).unwrap()),
        );
        let changed = replace_field(&mut s, &changed, TYPED, "object_id", V::String(id));
        let forged = replace_member(&mut s, &state, "objects", group, changed);
        // 连typed身份都重算成合法，仍须在shape high/low配方拒绝，而不是提前撞typed ID。
        assert_rejected_in_both_modes(
            &mut s,
            &forged,
            &raw,
            &rule,
            "CC006 obj3与同cut组事实身份不符",
        );
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
