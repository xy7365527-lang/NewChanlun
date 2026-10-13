//! #1419 TB-02-E：同次段对象 → 中枢事实对象（CC-016/017/022，schema `s-center/1`，
//! `tb1419-schema.md` v1）。
//!
//! 本模块不重新求值中枢判据。完成单元先经输入合同（schema v1 §1）核成与生产塔 L0 同一列表，
//! 然后扫描与派生全走库内 `theta_v0::classifier::center_scan`：seed 判定 `center::seed_facts`、
//! Zn 聚合 `center::zn_extrema`、单元归属（根裁定 D9/D-G/D-I 与 r2 R2/R3/R6）。本模块只做身份、
//! 修订、获知时点与 wire 投影。
use super::*;
use newchan_rust::theta_v0::classifier::center::{
    center_from_segments, CenterRoles, RoleVar, SeedExclusion, SeedInput, SeedRule, UnitRange,
};
use newchan_rust::theta_v0::classifier::center_scan::{
    l0_units, scan_center_facts_full, CenterScanFacts, CenterWindowFacts, ScanState, SeedFromSelf,
    UnitAssignment,
};
use newchan_rust::theta_v0::types::Segment;

pub(super) const KINDS: &[&str] = &[
    "CC-016.center_seed_attempt",
    "CC-016.center",
    "CC-017.center_roles",
    "CC-022.unit_assignment",
    "CC-055.center_relation",
    "CC-055.center_change",
    "CC-056.center_version",
];
pub(super) const SCHEMA_REVISION: &str = "s-center/1";
const SEMANTIC_VERSION: &str = "center-seed/1";
const POLICY_VERSION: &str = "strict_seed_zn_outer_c_resume";
const DELIVERED_SCOPE: &str = "l0_strict_seed_zn_roles_c_resume_assignment";
/// 版本对象与 CC-016/017/022 目录轴的延后范围（根裁定 D-J 第 4 点：带实票号）。
pub(super) const DEFERRED_SCOPE: [&str; 5] = [
    "global_G009_upper_levels_1429",
    "ge9_upgrade_1429",
    "third_class_and_break_1434",
    "expansion_newborn_1424",
    "connector_upper_construction_1429",
];
pub(super) const DELIVERED_SUBOBLIGATIONS: [&str; 3] = [
    "l0_strict_seed_vector_and_core_order",
    "zn_outer_inner_roles_with_contributors",
    "c_resume_assignment_and_connectors",
];

#[cfg(test)]
thread_local! {
    /// 测试注入：把第 `.0` 个塔侧 L0 单元的 `hi` 加 `.1`，制造输入合同不符（D-J 第 5 点注入测试）。
    pub(super) static INJECT_LAYER_HI_DELTA: std::cell::Cell<Option<(usize, i64)>> =
        const { std::cell::Cell::new(None) };
}

/// 同 cut 一个 `CC-013.segment` 的单元投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SegUnit {
    pub(super) unit_id: String,
    pub(super) object_id: String,
    /// 完成单元的区间与方向（塔侧同一份 `UnitRange`）；进行中单元为 `None`。
    pub(super) range: Option<UnitRange>,
    pub(super) sources: Vec<usize>,
}

/// 输入合同核过的单元列表：完成单元（与塔 L0 逐位相同）+ 至多一个进行中单元。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct UnitInputs {
    pub(super) completed: Vec<SegUnit>,
    pub(super) in_progress: Option<SegUnit>,
}

fn payload_of(o: &Value) -> Value {
    serde_json::from_str(o["payload_json"].as_str().expect("对象携 payload_json"))
        .expect("payload_json 是 JSON")
}

fn wire_int(v: &Value, name: &str) -> Result<i64, String> {
    v.as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| format!("TB02E_INPUT_CONTRACT：CC-013 字段 {name} 不是十进制文本"))
}

/// 完成段成员笔的方向极值：`stroke_refs` 中下标属于 `construction.stroke_indices` 的笔，取全部笔端价的
/// `[min, max]`（schema v1.3 §1）。成员笔缺 `stroke_refs` 条目 ⟹ 输入合同失败。
fn member_stroke_extremes(d: &Value) -> Result<(i64, i64), String> {
    let members = d["construction"]["stroke_indices"]
        .as_array()
        .ok_or("TB02E_INPUT_CONTRACT：CC-013 缺 construction.stroke_indices")?;
    let mut extremes: Option<(i64, i64)> = None;
    for m in members {
        let r = d["stroke_refs"]
            .as_array()
            .and_then(|refs| refs.iter().find(|r| r["stroke_index"] == *m))
            .ok_or_else(|| {
                format!("TB02E_INPUT_CONTRACT：成员笔 {m} 不在 CC-013 stroke_refs 中")
            })?;
        let (s, e) = (
            wire_int(&r["start_price"], "stroke_refs.start_price")?,
            wire_int(&r["end_price"], "stroke_refs.end_price")?,
        );
        let (lo, hi) = extremes.unwrap_or((s.min(e), s.max(e)));
        extremes = Some((lo.min(s).min(e), hi.max(s).max(e)));
    }
    extremes.ok_or_else(|| "TB02E_INPUT_CONTRACT：完成段没有成员笔".to_string())
}

/// 输入合同（schema v1.3 §1）：同 cut `CC-013.segment` 按起笔排序，除末个外都须完成且首尾相连；完成单元的
/// 区间 = 成员笔（`stroke_refs` 中下标属于 `construction.stroke_indices` 的笔）的方向极值
/// `[min(笔低), max(笔高)]`，方向 = `construction.direction`，须与同次 parser `layer.segments` 经塔同一投影的结果
/// 逐位相等。不取几何端点（`stroke_refs[0].start_price`／`geometric_end.price`）：真实数据上段端点可以不是段内极值，
/// 塔经 `make_segment` 取的是成员笔极值（`078:58`／`:60` 标准化；根裁定 `ROOT-RULING-1419-L3`，#804 同一判定只有一个
/// 输入值）。也不取全部 `stroke_refs`：它延伸到 observed_through，会越过段端。任一不符 ⟹
/// `Err(TB02E_INPUT_CONTRACT 收据)`，收据逐条列 `{check, position, expected, actual}`（根裁定 D-J 第 5 点）。
pub(super) fn units_from_segments(
    segment_objects: &[Value],
    layer_segments: &[Segment],
) -> Result<UnitInputs, String> {
    struct Row {
        unit_id: String,
        object_id: String,
        start_stroke: i64,
        end_stroke: Option<i64>,
        completed: bool,
        lo: i64,
        hi: i64,
        direction: String,
        price_direction: &'static str,
        sources: Vec<usize>,
    }
    let mut rows = vec![];
    for o in segment_objects
        .iter()
        .filter(|o| o["kind"] == "CC-013.segment")
    {
        let d = payload_of(o)["data"].clone();
        let completed = matches!(d["result"].as_str(), Some("CASE_ONE" | "CASE_TWO"));
        let (lo, hi, end_stroke, price_direction) = if completed {
            let a = wire_int(
                &d["stroke_refs"][0]["start_price"],
                "stroke_refs[0].start_price",
            )?;
            let b = wire_int(&d["geometric_end"]["price"], "geometric_end.price")?;
            let end = wire_int(
                &d["geometric_end"]["stroke_index"],
                "geometric_end.stroke_index",
            )?;
            let (lo, hi) = member_stroke_extremes(&d)?;
            (lo, hi, Some(end), if b > a { "UP" } else { "DOWN" })
        } else {
            (0, 0, None, "")
        };
        let sources: Vec<String> =
            serde_json::from_str(o["source_coords_json"].as_str().unwrap_or("[]"))
                .map_err(|e| format!("TB02E_INPUT_CONTRACT：CC-013 来源坐标不可读：{e}"))?;
        rows.push(Row {
            unit_id: d["entity_id"].as_str().unwrap_or_default().to_string(),
            object_id: o["object_id"].as_str().unwrap_or_default().to_string(),
            start_stroke: wire_int(
                &d["construction"]["start_stroke"],
                "construction.start_stroke",
            )?,
            end_stroke,
            completed,
            lo,
            hi,
            direction: d["construction"]["direction"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            price_direction,
            sources: sources.iter().filter_map(|s| s.parse().ok()).collect(),
        });
    }
    rows.sort_by_key(|r| r.start_stroke);
    #[allow(unused_mut)]
    let mut tower: Vec<UnitRange> = l0_units(layer_segments);
    #[cfg(test)]
    if let Some((k, delta)) = INJECT_LAYER_HI_DELTA.with(|c| c.get()) {
        if let Some(u) = tower.get_mut(k) {
            u.hi += delta;
        }
    }
    let mut mismatches: Vec<Value> = vec![];
    let mut miss = |check: &str, position: Value, expected: Value, actual: Value| {
        mismatches
            .push(json!({"check":check,"position":position,"expected":expected,"actual":actual}));
    };
    let n = rows.len();
    let mut completed: Vec<&Row> = vec![];
    let mut in_progress = None;
    for (i, r) in rows.iter().enumerate() {
        let position = json!({"candidate_index":i.to_string(),"unit_id":r.unit_id});
        if r.completed {
            if let Some(prev) = completed.last() {
                if prev.end_stroke.map(|e| e + 1) != Some(r.start_stroke) {
                    miss(
                        "contiguity",
                        position.clone(),
                        json!((prev.end_stroke.unwrap_or(-1) + 1).to_string()),
                        json!(r.start_stroke.to_string()),
                    );
                }
            }
            if r.direction != r.price_direction {
                miss(
                    "direction",
                    position,
                    json!(r.price_direction),
                    json!(r.direction),
                );
            }
            completed.push(r);
        } else if i + 1 == n {
            if let Some(prev) = completed.last() {
                if prev.end_stroke.map(|e| e + 1) != Some(r.start_stroke) {
                    miss(
                        "contiguity",
                        position,
                        json!((prev.end_stroke.unwrap_or(-1) + 1).to_string()),
                        json!(r.start_stroke.to_string()),
                    );
                }
            }
            in_progress = Some(r);
        } else {
            miss(
                "interior_incomplete",
                position,
                json!("CASE_ONE|CASE_TWO"),
                json!("incomplete"),
            );
        }
    }
    if completed.len() != tower.len() {
        miss(
            "count",
            json!({"completed_units":completed.len().to_string()}),
            json!(tower.len().to_string()),
            json!(completed.len().to_string()),
        );
    }
    for (k, (r, u)) in completed.iter().zip(&tower).enumerate() {
        let position = json!({"completed_index":k.to_string(),"unit_id":r.unit_id});
        if r.lo != u.lo {
            miss(
                "lo",
                position.clone(),
                json!(u.lo.to_string()),
                json!(r.lo.to_string()),
            );
        }
        if r.hi != u.hi {
            miss(
                "hi",
                position.clone(),
                json!(u.hi.to_string()),
                json!(r.hi.to_string()),
            );
        }
        if r.direction != dir_str(u.direction) {
            miss(
                "direction",
                position,
                json!(dir_str(u.direction)),
                json!(r.direction),
            );
        }
    }
    if !mismatches.is_empty() {
        return Err(format!(
            "TB02E_INPUT_CONTRACT：段对象与塔 L0 单元不符，本 cut 拒绝提交：{}",
            canonical_json(&json!(mismatches))
        ));
    }
    let unit = |r: &Row, range: Option<UnitRange>| SegUnit {
        unit_id: r.unit_id.clone(),
        object_id: r.object_id.clone(),
        range,
        sources: r.sources.clone(),
    };
    Ok(UnitInputs {
        completed: completed
            .iter()
            .zip(&tower)
            .map(|(r, u)| unit(r, Some(*u)))
            .collect(),
        in_progress: in_progress.map(|r| unit(r, None)),
    })
}

fn add(
    b: &mut tb02a_facts::Builder<'_>,
    kind: &str,
    slot: Value,
    generation: i64,
    data: Value,
    sources: Vec<usize>,
) -> String {
    b.add(
        kind,
        json!([kind, slot]),
        json!({"schema_revision":SCHEMA_REVISION,"semantic_version":SEMANTIC_VERSION,"policy_version":POLICY_VERSION,
            "view_role":"main","object_generation":generation.to_string(),"slot":slot,"data":tb02c::wire(data)}),
        sources,
    )
}

/// `segment:<g>:<anchor>` → `<g>:<anchor>`。
fn anchor_of(unit_id: &str) -> &str {
    unit_id.strip_prefix("segment:").unwrap_or(unit_id)
}

fn unit_json(u: &SegUnit) -> Value {
    match &u.range {
        Some(r) => {
            json!({"unit_id":u.unit_id,"unit_object_id":u.object_id,"direction":dir_str(r.direction),
            "lo":r.lo.to_string(),"hi":r.hi.to_string()})
        }
        None => json!({"unit_id":u.unit_id,"unit_object_id":u.object_id,"completed":false}),
    }
}

fn union_sources<'a>(units: impl IntoIterator<Item = &'a SegUnit>) -> Vec<usize> {
    let mut s: Vec<usize> = units
        .into_iter()
        .flat_map(|u| u.sources.iter().copied())
        .collect();
    s.sort_unstable();
    s.dedup();
    s
}

/// tb02c 同款修订：与上一修订（修订号归一后）逐值相同 ⟹ 沿用修订号，否则 +1。
fn revise(prior: Option<&Value>, mut data: Value) -> (Value, i64, bool) {
    data["entity_revision"] = json!("1");
    let changed = prior
        .map(|p| {
            let mut normalized = p.clone();
            normalized["entity_revision"] = json!("1");
            normalized != data
        })
        .unwrap_or(true);
    let revision = prior
        .and_then(|p| p["entity_revision"].as_str()?.parse::<i64>().ok())
        .unwrap_or(0)
        + i64::from(changed);
    data["entity_revision"] = json!(revision.to_string());
    (data, revision, changed)
}

/// 无修订号的对象：除 `known_at` 外与上一份相同 ⟹ 沿用上一份（获知时点不前移）。
fn keep_known(prior: Option<&Value>, data: Value) -> Value {
    if let Some(p) = prior {
        let mut candidate = data.clone();
        candidate["known_at"] = p["known_at"].clone();
        if candidate == *p {
            return p.clone();
        }
    }
    data
}

/// 中枢事实全集（输入合同之后的纯投影）。返回 (对象, 关系)。
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    inputs: &UnitInputs,
    raw: &BTreeMap<i64, Value>,
    all_events: &[Value],
    previous: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
    semantic_commit_ns: Option<String>,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let generation = tb02b::fact_basis(all_events);
    let units: Vec<UnitRange> = inputs
        .completed
        .iter()
        .map(|u| u.range.expect("完成单元"))
        .collect();
    let facts = scan_center_facts_full(&units, center_from_segments);
    if let Some(bad) = facts
        .attempts
        .iter()
        .find(|a| a.facts.exclusion(SeedRule::Complete) == Some(SeedExclusion::NotAlternating))
    {
        let at = bad.at;
        return Err(format!(
            "TB02E_INPUT_CONTRACT：段对象与塔 L0 单元不符，本 cut 拒绝提交：{}",
            canonical_json(&json!([{"check":"not_alternating",
                "position":{"completed_index":at.to_string(),"unit_id":inputs.completed[at].unit_id},
                "expected":"alternating","actual":"NOT_ALTERNATING"}]))
        ));
    }
    let latest = raw
        .values()
        .max_by_key(|r| r["seq"].as_i64())
        .cloned()
        .unwrap_or(Value::Null);
    let known = json!({"generation":gen.to_string(),"input_frontier":frontier.to_string(),"receipt_id":latest["receipt_id"],
        "received_at":latest["received_at"],"semantic_commit_ns":semantic_commit_ns});
    let mut b = tb02a_facts::Builder {
        raw,
        generation: gen,
        cut,
        objects: vec![],
    };
    let prior: BTreeMap<(String, String), Value> = previous
        .iter()
        .filter(|o| o["kind"].as_str().is_some_and(|k| KINDS[..4].contains(&k)))
        .map(|o| {
            let d = payload_of(o)["data"].clone();
            (
                (
                    o["kind"].as_str().unwrap().to_string(),
                    d["entity_id"].as_str().unwrap_or_default().to_string(),
                ),
                d,
            )
        })
        .collect();
    let prior_of = |kind: &str, entity: &str| prior.get(&(kind.to_string(), entity.to_string()));
    let ids: Vec<&str> = inputs
        .completed
        .iter()
        .map(|u| u.unit_id.as_str())
        .collect();
    let windows: Vec<CenterWindowFacts> = facts.window_facts(&units);
    let center_ids: Vec<String> = windows
        .iter()
        .map(|w| format!("center:{}", anchor_of(ids[w.first])))
        .collect();
    let mut relations = vec![];
    let mut edges: Vec<(String, &str, String, String, String, Vec<usize>)> = vec![];

    // ── CC-016.center_seed_attempt ──
    let mut attempt_ids = BTreeMap::new();
    for a in &facts.attempts {
        let seed = &inputs.completed[a.at..a.at + 3];
        let seed_units = &units[a.at..a.at + 3];
        let roles = CenterRoles::of_members(seed_units);
        let extremum = |var: RoleVar| {
            let contributors: Vec<&str> = roles
                .contributors(seed_units, var)
                .into_iter()
                .map(|k| ids[a.at + k])
                .collect();
            let witness: Vec<&str> = if contributors.len() >= 2 {
                contributors.clone()
            } else {
                vec![]
            };
            json!({"value":roles.value(var).to_string(),"contributors":contributors,"same_price_witness":witness})
        };
        let v = a.facts.condition_vector(SeedInput::TOWER);
        let entity = format!("seed_attempt:{}", anchor_of(ids[a.at]));
        let center = facts
            .windows
            .iter()
            .position(|w| w.seed_at == a.at)
            .map(|w| center_ids[w].clone());
        let data = json!({"entity_id":entity,"units":seed.iter().map(unit_json).collect::<Vec<_>>(),
            "vector":{"completed":v.completed,"consecutive":v.consecutive,"same_level":v.same_level,"alternating":v.alternating,"strict":v.strict},
            "ZD":extremum(RoleVar::Zd),"ZG":extremum(RoleVar::Zg),"core_order":a.facts.core_order.as_str(),
            "result":if a.formed {"FORMED"} else {"EXCLUDED"},
            "exclusion_reason":a.facts.exclusion(SeedRule::Complete).map(SeedExclusion::as_str),
            "center_id":center,"known_at":known});
        let data = keep_known(prior_of(KINDS[0], &entity), tb02c::wire(data));
        let sources = union_sources(seed);
        let oid = add(
            &mut b,
            KINDS[0],
            json!(entity),
            generation,
            data,
            sources.clone(),
        );
        edges.push((
            entity.clone(),
            "attempted_from",
            ids[a.at].to_string(),
            "1".into(),
            oid.clone(),
            sources,
        ));
        attempt_ids.insert(a.at, entity);
    }

    // ── CC-016.center + CC-017.center_roles ──
    let mut center_revisions = vec![];
    let mut center_objects = vec![];
    for (w, wf) in windows.iter().enumerate() {
        let entity = &center_ids[w];
        let members = &inputs.completed[wf.first..=wf.last];
        let member_ids: Vec<&str> = (wf.first..=wf.last).map(|k| ids[k]).collect();
        let roles_entity = format!("roles:{entity}");
        let status = if wf.upgrade_pending {
            "UPGRADE_PENDING_TB02H"
        } else if wf.settled {
            "SETTLED"
        } else {
            "EXTENDING"
        };
        let next_tested = wf
            .next_tested_unit
            .filter(|&k| k == units.len())
            .and(inputs.in_progress.as_ref())
            .map(|u| u.unit_id.clone());
        let break_unit = wf.break_unit.map(|k| ids[k].to_string());
        let old = prior_of(KINDS[1], entity);
        let settled_known = if wf.settled {
            old.filter(|p| p["settled"] == true && p["break_unit"] == json!(break_unit))
                .map(|p| p["settled_known_at"].clone())
                .unwrap_or(known.clone())
        } else {
            Value::Null
        };
        let data = tb02c::wire(
            json!({"entity_id":entity,"entity_revision":"1","seed_attempt_id":attempt_ids[&wf.first],
            "seed_units":&member_ids[..3],"zn_direction":dir_str(units[wf.first].direction),"members":member_ids,
            "zn_members":wf.zn_members.iter().map(|&k| ids[k]).collect::<Vec<_>>(),"status":status,"settled":wf.settled,
            "break_unit":break_unit,"break_up":wf.break_up.unwrap_or(false),"break_up_defined":wf.break_up.is_some(),
            "resume_anchor":wf.settled.then(|| json!({"after_unit_id":ids[wf.last],"offset":"2"})),
            "next_tested_unit":next_tested,"roles_id":roles_entity,
            "formed_known_at":old.map(|p| p["formed_known_at"].clone()).unwrap_or(known.clone()),
            "settled_known_at":settled_known}),
        );
        let (data, revision, changed) = revise(old, data);
        let mut sources = union_sources(members);
        if let Some(k) = wf.break_unit {
            sources.extend(&inputs.completed[k].sources);
        }
        let oid = add(
            &mut b,
            KINDS[1],
            json!(entity),
            generation,
            data.clone(),
            sources.clone(),
        );
        // 角色：≥9 成员只发核心，动态四变量置 null 并给原因（R3）。
        let role = |var: RoleVar| {
            let r = wf.role(var);
            let hidden = wf.upgrade_pending && !var.frozen();
            let contributors: Vec<&str> = if hidden {
                vec![]
            } else {
                r.contributors.iter().map(|&k| ids[k]).collect()
            };
            let witness: Vec<&str> = if contributors.len() >= 2 {
                contributors.clone()
            } else {
                vec![]
            };
            json!({"value":if hidden {Value::Null} else {json!(r.value.to_string())},"contributors":contributors,
                "same_price_witness":witness,"frozen":var.frozen(),"null_reason":if hidden {json!("UPGRADE_PENDING_TB02H")} else {Value::Null}})
        };
        let roles = json!({"CORE":{"ZD":role(RoleVar::Zd),"ZG":role(RoleVar::Zg)},"OUTER":{"DD":role(RoleVar::Dd),"GG":role(RoleVar::Gg)},
            "INNER_EXTREMA":{"D":role(RoleVar::D),"G":role(RoleVar::G)}});
        let old_roles = prior_of(KINDS[2], &roles_entity);
        let roles_data = match old_roles {
            Some(p) if p["roles"] == roles && p["center_id"] == json!(entity) => p.clone(),
            _ => {
                let rev = old_roles
                    .and_then(|p| p["entity_revision"].as_str()?.parse::<i64>().ok())
                    .unwrap_or(0)
                    + 1;
                tb02c::wire(
                    json!({"entity_id":roles_entity,"entity_revision":rev.to_string(),"center_id":entity,"roles":roles,
                    "previous":old_roles.map(|p| json!({"entity_revision":p["entity_revision"],"roles":p["roles"]})),"known_at":known}),
                )
            }
        };
        let roles_changed = old_roles.is_some_and(|p| *p != roles_data);
        let roles_oid = add(
            &mut b,
            KINDS[2],
            json!(roles_entity),
            generation,
            roles_data.clone(),
            sources.clone(),
        );
        if wf.upgrade_pending {
            // ≥9：成员仍列在 center.members，但单元归属记 UPGRADE_PENDING_TB02H，不发 member_of 边。
        } else {
            for k in wf.first..=wf.last {
                edges.push((
                    ids[k].to_string(),
                    "member_of",
                    entity.clone(),
                    revision.to_string(),
                    oid.clone(),
                    inputs.completed[k].sources.clone(),
                ));
            }
        }
        let mut changes: Vec<&str> = vec![];
        match old {
            None => changes.push("formed"),
            Some(p) if changed => {
                if p["members"].as_array().map_or(0, Vec::len) < member_ids.len() {
                    changes.push("member_added");
                }
                if p["settled"] != true && wf.settled {
                    changes.push("settled");
                }
                if p["status"] != "UPGRADE_PENDING_TB02H" && wf.upgrade_pending {
                    changes.push("upgrade_pending");
                }
            }
            _ => {}
        }
        for change in changes {
            add(
                &mut b,
                KINDS[5],
                json!([gen.to_string(), entity, change]),
                generation,
                json!({"entity_id":entity,"change":change,"before":old,"after":data,"known_at":known,"object_id":oid}),
                sources.clone(),
            );
        }
        if roles_changed {
            add(
                &mut b,
                KINDS[5],
                json!([gen.to_string(), roles_entity, "roles_updated"]),
                generation,
                json!({"entity_id":roles_entity,"change":"roles_updated","before":old_roles,"after":roles_data,"known_at":known,"object_id":roles_oid}),
                sources.clone(),
            );
        }
        center_revisions.push(revision);
        center_objects.push(oid);
    }
    for ((kind, entity), p) in &prior {
        if kind == KINDS[1] && !center_ids.contains(entity) {
            add(
                &mut b,
                KINDS[5],
                json!([gen.to_string(), entity, "withdrawn"]),
                generation,
                json!({"entity_id":entity,"change":"withdrawn","before":p,"after":null,"known_at":known,"object_id":null}),
                raw.keys().map(|k| *k as usize).collect(),
            );
        }
    }

    // ── CC-022.unit_assignment ──
    let mut waiting: Vec<&str> = vec![];
    let unit_facts = facts.unit_facts();
    let assignments = unit_facts
        .iter()
        .enumerate()
        .map(|(k, f)| (&inputs.completed[k], Some(*f)))
        .chain(inputs.in_progress.iter().map(|u| (u, None)));
    for (unit, f) in assignments {
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
        if let Some(r) = reason {
            waiting.push(r);
        }
        let data = tb02c::wire(
            json!({"entity_id":entity,"unit":unit_json(unit),"status":status,
            "center_id":center.map(|w| &center_ids[w]),"left_center_id":left.map(|w| &center_ids[w]),
            "right_center_id":right.map(|w| &center_ids[w]),"right_pending":pending,"reason":reason,
            "seed_from_self":seed.as_str(),"known_at":known}),
        );
        let data = keep_known(prior_of(KINDS[3], &entity), data);
        let oid = add(
            &mut b,
            KINDS[3],
            json!(entity),
            generation,
            data,
            unit.sources.clone(),
        );
        for (kind, w) in [("connects_left", left), ("connects_right", right)] {
            if let Some(w) = w {
                edges.push((
                    unit.unit_id.clone(),
                    kind,
                    center_ids[w].clone(),
                    center_revisions[w].to_string(),
                    oid.clone(),
                    unit.sources.clone(),
                ));
            }
        }
    }

    // ── CC-055.center_relation ──
    for (source, kind, target, version, witness, sources) in edges {
        let edge = json!({"source_id":source,"relation_kind":kind,"target_id":target,"version":version,"witness_object_id":witness});
        add(
            &mut b,
            KINDS[4],
            json!([source, kind, target]),
            generation,
            edge,
            sources,
        );
        relations.push(json!({"subject":source,"relation_type":kind,"object":target}));
    }

    // ── CC-056.center_version ──
    waiting.sort_unstable();
    waiting.dedup();
    add(
        &mut b,
        KINDS[6],
        json!("TB-02-E"),
        generation,
        json!({"input_frontier":frontier.to_string(),"known_at":known,"scan_cursor":scan_cursor(&facts, inputs, &center_ids, &windows),
            "waiting_reasons":waiting,"delivered_scope":DELIVERED_SCOPE,"deferred_scope":DEFERRED_SCOPE}),
        raw.keys().map(|k| *k as usize).collect(),
    );
    Ok((b.objects, relations))
}

/// 扫描游标的实体锚表示（schema v1 §3，根裁定 D-J 第 2 点）：位置 = `after_unit_id` 在完成单元序中的位置
/// + `offset`；`after_unit_id = null` 只在尚无完成单元时出现，位置从窗口起点前一格算起。
fn scan_cursor(
    facts: &CenterScanFacts,
    inputs: &UnitInputs,
    center_ids: &[String],
    windows: &[CenterWindowFacts],
) -> Value {
    let len = inputs.completed.len();
    let id_at = |k: usize| -> Option<String> {
        if k < len {
            Some(inputs.completed[k].unit_id.clone())
        } else if k == len {
            inputs.in_progress.as_ref().map(|u| u.unit_id.clone())
        } else {
            None
        }
    };
    let awaiting = |k: usize| -> (Value, Value, Value) {
        match id_at(k) {
            Some(id) => (json!(id), Value::Null, Value::Null),
            None => {
                let after = len
                    .checked_sub(1)
                    .map(|i| inputs.completed[i].unit_id.clone());
                let offset = k as i64 - (len as i64 - 1);
                (Value::Null, json!(after), json!(offset.to_string()))
            }
        }
    };
    match facts.scan_state() {
        ScanState::Extending { center, next_unit } => {
            let (unit, after, offset) = awaiting(next_unit);
            debug_assert_eq!(windows[center].last + 1, next_unit);
            json!({"state":"EXTENDING","center_id":center_ids[center],"seed_start_unit_id":null,
                "awaiting_unit_id":unit,"awaiting_after_unit_id":after,"awaiting_offset":offset})
        }
        ScanState::SeekingSeed {
            seed_start,
            awaiting_unit,
        } => {
            let (unit, after, offset) = awaiting(awaiting_unit);
            json!({"state":"SEEKING_SEED","center_id":null,"seed_start_unit_id":id_at(seed_start),
                "awaiting_unit_id":unit,"awaiting_after_unit_id":after,"awaiting_offset":offset})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 段层合成输入（PLAN：S 写者单测先用段层输入；原始 K 线落地等 S0b）：端点 `P_k→P_{k+1}` 为完成单元，
    /// `in_progress` 时末个端点后再挂一个进行中单元。每单元来源坐标 = {k}。
    fn synthetic(endpoints: &[i64], in_progress: bool) -> (UnitInputs, BTreeMap<i64, Value>) {
        let completed: Vec<SegUnit> = endpoints
            .windows(2)
            .enumerate()
            .map(|(k, w)| SegUnit {
                unit_id: format!("segment:1:{}", 10 + 4 * k),
                object_id: format!("obj-seg-{k}"),
                range: Some(UnitRange {
                    start_index: 4 * k,
                    end_index: 4 * k + 4,
                    direction: if w[1] > w[0] {
                        Direction::Up
                    } else {
                        Direction::Down
                    },
                    lo: w[0].min(w[1]),
                    hi: w[0].max(w[1]),
                }),
                sources: vec![k],
            })
            .collect();
        let n = completed.len();
        let raw: BTreeMap<i64, Value> = (0..=n as i64)
            .map(|i| (i, json!({"identity_key":format!("raw-{i}"),"payload_hash":"h","receipt_id":format!("receipt-{i}"),
                "event_id":format!("bar-{i}"),"input_revision":"1","revision":1,"seq":i,"source_coord":i.to_string(),
                "received_at":"2000-01-01T00:00:00Z"})))
            .collect();
        let in_progress = in_progress.then(|| SegUnit {
            unit_id: format!("segment:1:{}", 10 + 4 * n),
            object_id: format!("obj-seg-{n}"),
            range: None,
            sources: vec![n],
        });
        (
            UnitInputs {
                completed,
                in_progress,
            },
            raw,
        )
    }

    fn run(endpoints: &[i64], previous: &[Value], gen: i64) -> Vec<Value> {
        let (inputs, raw) = synthetic(endpoints, true);
        build(
            &inputs,
            &raw,
            &[],
            previous,
            gen,
            &format!("cut-{gen}"),
            gen - 1,
            None,
        )
        .unwrap()
        .0
    }

    fn data_of(objects: &[Value], kind: &str, entity: &str) -> Value {
        objects
            .iter()
            .find(|o| o["kind"] == kind && payload_of(o)["data"]["entity_id"] == entity)
            .map(|o| payload_of(o)["data"].clone())
            .unwrap_or_else(|| panic!("缺 {kind} {entity}"))
    }

    fn entities(objects: &[Value], kind: &str) -> Vec<String> {
        objects
            .iter()
            .filter(|o| o["kind"] == kind)
            .map(|o| {
                payload_of(o)["data"]["entity_id"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }

    /// S0 oracle UP 流（u0..u12 完成、u13 进行中）。
    const UP: [i64; 15] = [10, 40, 20, 35, 30, 38, 21, 25, 5, 15, 2, 12, 4, 9, 6];
    fn seg(k: usize) -> String {
        format!("segment:1:{}", 10 + 4 * k)
    }
    fn up_cut(k: usize) -> &'static [i64] {
        &UP[..k + 2]
    }

    /// K10（D9/A-ST-016-01）：只有 A；u8 CONNECTOR(A→pending)；u9、u10 待评估；u11 进行中；无 candidate 类对象；
    /// A 的续扫锚是 {after_unit_id: u7, offset: "2"}，游标等 seed 自 u9、等待 u11 完成。
    #[test]
    fn tb02e_k10_one_center_connector_pending_and_entity_anchors() {
        let objects = run(up_cut(10), &[], 11);
        assert_eq!(
            entities(&objects, KINDS[1]),
            vec![format!("center:1:{}", 10)]
        );
        assert!(objects
            .iter()
            .all(|o| !o["kind"].as_str().unwrap().contains("candidate")));
        let a = data_of(&objects, KINDS[1], "center:1:10");
        assert_eq!(a["status"], "SETTLED");
        assert_eq!(a["break_unit"], json!(seg(8)));
        assert_eq!(
            (a["break_up"].clone(), a["break_up_defined"].clone()),
            (json!(false), json!(true))
        );
        assert_eq!(
            a["resume_anchor"],
            json!({"after_unit_id":seg(7),"offset":"2"})
        );
        assert_eq!(a["next_tested_unit"], Value::Null);
        let u8 = data_of(&objects, KINDS[3], &format!("assign:{}", seg(8)));
        assert_eq!(
            (
                u8["status"].clone(),
                u8["left_center_id"].clone(),
                u8["right_pending"].clone()
            ),
            (json!("CONNECTOR"), json!("center:1:10"), json!(true))
        );
        for k in [9, 10] {
            let u = data_of(&objects, KINDS[3], &format!("assign:{}", seg(k)));
            assert_eq!(u["reason"], "SEED_NOT_YET_EVALUABLE", "u{k}");
        }
        let u11 = data_of(&objects, KINDS[3], &format!("assign:{}", seg(11)));
        assert_eq!(
            (u11["reason"].clone(), u11["unit"]["completed"].clone()),
            (json!("UNIT_NOT_COMPLETED"), json!(false))
        );
        let version = data_of_version(&objects);
        assert_eq!(
            version["scan_cursor"],
            json!({"state":"SEEKING_SEED","center_id":null,"seed_start_unit_id":seg(9),"awaiting_unit_id":seg(11),
                "awaiting_after_unit_id":null,"awaiting_offset":null})
        );
        assert_eq!(version["deferred_scope"], json!(DEFERRED_SCOPE));
    }

    fn data_of_version(objects: &[Value]) -> Value {
        objects
            .iter()
            .find(|o| o["kind"] == KINDS[6])
            .map(|o| payload_of(o)["data"].clone())
            .unwrap()
    }

    /// K11→K12：B 在 K11 成立并发布（formed），u8 由 CONNECTOR(A→pending) 变为 CONNECTOR(A→B)；K12 B 吸收 u12
    /// （member_added，修订 2）；六变量与贡献成员取 oracle；角色修订链 previous 指回上一版（A-OB-004-01）。
    #[test]
    fn tb02e_k11_k12_revision_chain_and_roles() {
        let k10 = run(up_cut(10), &[], 11);
        let k11 = run(up_cut(11), &k10, 12);
        let b = data_of(&k11, KINDS[1], "center:1:46");
        assert_eq!(
            (b["entity_revision"].clone(), b["status"].clone()),
            (json!("1"), json!("EXTENDING"))
        );
        assert_eq!(b["formed_known_at"]["generation"], "12");
        let u8 = data_of(&k11, KINDS[3], &format!("assign:{}", seg(8)));
        assert_eq!(
            (u8["right_center_id"].clone(), u8["right_pending"].clone()),
            (json!("center:1:46"), json!(false))
        );
        assert_eq!(u8["known_at"]["generation"], "12", "归属变化即新获知时点");
        let a_k10 = data_of(&k10, KINDS[1], "center:1:10");
        assert_eq!(
            data_of(&k11, KINDS[1], "center:1:10"),
            a_k10,
            "A 未变 ⟹ 对象逐值不变"
        );
        let k12 = run(up_cut(12), &k11, 13);
        let b12 = data_of(&k12, KINDS[1], "center:1:46");
        assert_eq!(b12["entity_revision"], "2");
        assert_eq!(b12["formed_known_at"]["generation"], "12");
        assert_eq!(b12["members"], json!([seg(9), seg(10), seg(11), seg(12)]));
        assert_eq!(b12["zn_members"], json!([seg(9), seg(11)]));
        let roles = data_of(&k12, KINDS[2], "roles:center:1:46");
        let r = &roles["roles"];
        assert_eq!(r["CORE"]["ZD"]["value"], "4");
        assert_eq!(r["CORE"]["ZG"]["contributors"], json!([seg(10), seg(11)]));
        assert_eq!(
            r["CORE"]["ZG"]["same_price_witness"],
            json!([seg(10), seg(11)])
        );
        assert_eq!(
            (
                r["OUTER"]["DD"]["value"].clone(),
                r["OUTER"]["GG"]["value"].clone()
            ),
            (json!("2"), json!("15"))
        );
        assert_eq!(
            (
                r["INNER_EXTREMA"]["D"]["value"].clone(),
                r["INNER_EXTREMA"]["G"]["value"].clone()
            ),
            (json!("4"), json!("12"))
        );
        assert_eq!(r["CORE"]["ZD"]["frozen"], true);
        assert_eq!(r["OUTER"]["DD"]["frozen"], false);
        let changes: Vec<Value> = k12
            .iter()
            .filter(|o| o["kind"] == KINDS[5])
            .map(|o| payload_of(o)["data"]["change"].clone())
            .collect();
        assert!(changes.contains(&json!("member_added")), "{changes:?}");
        // u12 [4,9] 不改 B 的 Zn 极值（u12 非 Zn）⟹ 角色不出新修订。
        assert_eq!(roles["entity_revision"], "1");
        assert!(!changes.contains(&json!("roles_updated")));
    }

    /// UP K4/K6：D 20→30、G 35→25（D>G 合法），角色出新修订且 previous 指回上一版；ZD/ZG 冻结不变（A-ST-012-01）。
    #[test]
    fn tb02e_inner_extrema_update_creates_roles_revision() {
        let k3 = run(up_cut(3), &[], 4);
        let k4 = run(up_cut(4), &k3, 5);
        let roles = data_of(&k4, KINDS[2], "roles:center:1:10");
        assert_eq!(roles["entity_revision"], "2");
        assert_eq!(roles["roles"]["INNER_EXTREMA"]["D"]["value"], "30");
        assert_eq!(
            roles["previous"]["roles"]["INNER_EXTREMA"]["D"]["value"],
            "20"
        );
        assert_eq!(roles["roles"]["CORE"], roles["previous"]["roles"]["CORE"]);
        let k5 = run(up_cut(5), &k4, 6);
        let k6 = run(up_cut(6), &k5, 7);
        let r6 = data_of(&k6, KINDS[2], "roles:center:1:10");
        assert_eq!(
            (
                r6["roles"]["INNER_EXTREMA"]["D"]["value"].clone(),
                r6["roles"]["INNER_EXTREMA"]["G"]["value"].clone()
            ),
            (json!("30"), json!("25"))
        );
        assert_eq!(
            r6["roles"]["INNER_EXTREMA"]["G"]["contributors"],
            json!([seg(6)])
        );
    }

    /// E4 K7：续扫锚 u5 的 seed 相切 ⟹ EXCLUDED/TANGENT_ZD_EQ_ZG、Bool⁵ = 11110、center_id=null；u5 CONNECTOR(A→pending)
    /// 且 seed_from_self=FAILED。
    #[test]
    fn tb02e_e4_tangent_attempt_is_published_with_reason() {
        let e4 = [10, 40, 20, 35, 5, 15, 2, 18, 15];
        let objects = run(&e4, &[], 9);
        let t = data_of(
            &objects,
            KINDS[0],
            &format!("seed_attempt:1:{}", 10 + 4 * 5),
        );
        assert_eq!(
            (
                t["result"].clone(),
                t["exclusion_reason"].clone(),
                t["core_order"].clone()
            ),
            (json!("EXCLUDED"), json!("TANGENT_ZD_EQ_ZG"), json!("ZD=ZG"))
        );
        assert_eq!(
            t["vector"],
            json!({"completed":true,"consecutive":true,"same_level":true,"alternating":true,"strict":false})
        );
        assert_eq!(t["center_id"], Value::Null);
        assert_eq!(
            (t["ZD"]["value"].clone(), t["ZG"]["value"].clone()),
            (json!("15"), json!("15"))
        );
        let u5 = data_of(&objects, KINDS[3], &format!("assign:{}", seg(5)));
        assert_eq!(
            (
                u5["status"].clone(),
                u5["right_pending"].clone(),
                u5["seed_from_self"].clone()
            ),
            (json!("CONNECTOR"), json!(true), json!("FAILED"))
        );
    }

    /// W9 K8（R3/D-I）：第 9 个成员加入 ⟹ 中枢对象保留并标 UPGRADE_PENDING_TB02H，ZD/ZG 照常，DD/GG/D/G 置 null 带原因；
    /// 窗口单元归属 UPGRADE_PENDING_TB02H，不发 member_of 边。
    #[test]
    fn tb02e_w9_upgrade_pending_keeps_center_and_nulls_dynamic_roles() {
        let w9 = [10, 40, 20, 35, 25, 38, 22, 36, 24, 37, 26];
        let k7 = run(&w9[..9], &[], 8);
        let k8 = run(&w9[..10], &k7, 9);
        let c = data_of(&k8, KINDS[1], "center:1:10");
        assert_eq!(c["status"], "UPGRADE_PENDING_TB02H");
        assert_eq!(
            c["formed_known_at"],
            data_of(&k7, KINDS[1], "center:1:10")["formed_known_at"]
        );
        let r = data_of(&k8, KINDS[2], "roles:center:1:10")["roles"].clone();
        assert_eq!(
            (
                r["CORE"]["ZD"]["value"].clone(),
                r["CORE"]["ZG"]["value"].clone()
            ),
            (json!("20"), json!("35"))
        );
        for (role, var) in [
            ("OUTER", "DD"),
            ("OUTER", "GG"),
            ("INNER_EXTREMA", "D"),
            ("INNER_EXTREMA", "G"),
        ] {
            assert_eq!(r[role][var]["value"], Value::Null, "{var}");
            assert_eq!(
                r[role][var]["null_reason"], "UPGRADE_PENDING_TB02H",
                "{var}"
            );
        }
        for k in 0..=8 {
            assert_eq!(
                data_of(&k8, KINDS[3], &format!("assign:{}", seg(k)))["reason"],
                "UPGRADE_PENDING_TB02H",
                "u{k}"
            );
        }
        assert!(k8
            .iter()
            .filter(|o| o["kind"] == KINDS[4])
            .all(|o| payload_of(o)["data"]["relation_kind"] != "member_of"));
        let changes: Vec<Value> = k8
            .iter()
            .filter(|o| o["kind"] == KINDS[5])
            .map(|o| payload_of(o)["data"]["change"].clone())
            .collect();
        assert!(changes.contains(&json!("upgrade_pending")), "{changes:?}");
    }

    /// A-CC-016/017/022-03：同一输入两次 build 逐字节相同（对象 id 与 payload）。
    #[test]
    fn tb02e_same_input_rebuilds_byte_identical() {
        let a = run(up_cut(12), &[], 13);
        let b = run(up_cut(12), &[], 13);
        assert_eq!(a, b);
    }

    /// 无完成单元 / 不足三单元：只发归属与版本对象；游标用 after=null 的窗口起点约定。
    #[test]
    fn tb02e_empty_and_short_windows_use_window_start_anchor() {
        let (inputs, raw) = synthetic(&[10], false);
        let (objects, _) = build(&inputs, &raw, &[], &[], 1, "cut-1", 0, None).unwrap();
        assert_eq!(
            data_of_version(&objects)["scan_cursor"],
            json!({"state":"SEEKING_SEED","center_id":null,"seed_start_unit_id":null,"awaiting_unit_id":null,
                "awaiting_after_unit_id":null,"awaiting_offset":"3"})
        );
        let objects = run(&UP[..3], &[], 3);
        assert_eq!(
            data_of_version(&objects)["scan_cursor"],
            json!({"state":"SEEKING_SEED","center_id":null,"seed_start_unit_id":seg(0),"awaiting_unit_id":seg(2),
                "awaiting_after_unit_id":null,"awaiting_offset":null})
        );
        assert_eq!(
            data_of(&objects, KINDS[3], &format!("assign:{}", seg(0)))["reason"],
            "NO_LEFT_CENTER_IN_WINDOW"
        );
    }

    /// 输入合同（D-J 第 5 点）：同次 parser 真实段对象在全部冻结 raw 前缀上都满足合同（非真空：有完成单元的前缀
    /// 计数 > 0）；注入塔侧区间不符 ⟹ Err，收据逐条列 check/position/expected/actual。
    #[test]
    fn tb02e_input_contract_holds_on_frozen_raw_prefixes_and_rejects_injected_mismatch() {
        let mut nonvacuous = 0;
        for fixture in [
            include_str!("../../../../s_session/tests/fixtures/tb02c/raw-ledger.json"),
            include_str!("../../../../s_session/tests/fixtures/tb02d/raw-ledger.json"),
        ] {
            let ledger: Value = serde_json::from_str(fixture).unwrap();
            for (name, rows) in ledger["cases"].as_object().unwrap() {
                let rows = rows.as_array().unwrap();
                for count in 1..=rows.len() {
                    let bars: Vec<Bar> = rows[..count]
                        .iter()
                        .enumerate()
                        .map(|(i, row)| Bar {
                            source_index: i,
                            timestamp: i as i64,
                            open: row["open"].as_i64().unwrap(),
                            high: row["high"].as_i64().unwrap(),
                            low: row["low"].as_i64().unwrap(),
                            close: row["close"].as_i64().unwrap(),
                            volume: 1.0,
                            untradable: false,
                        })
                        .collect();
                    let (layer, facts) =
                        newchan_rust::theta_v0::parser::parse_layer_with_inclusion_facts(
                            &bars,
                            &ThetaConfig::default(),
                        );
                    let raw: BTreeMap<i64, Value> = (0..count as i64).map(|i| (i, json!({
                        "identity_key":format!("raw-{i}"),"payload_hash":"h","receipt_id":format!("receipt-{i}"),"event_id":format!("bar-{i}"),
                        "input_revision":"1","revision":1,"seq":i,"source_coord":i.to_string(),"received_at":"2000-01-01T00:00:00Z"}))).collect();
                    let (segments, _) = tb02c::build(
                        facts.segment_facts.as_ref().unwrap(),
                        facts.stroke_facts.as_ref().unwrap(),
                        &raw,
                        &[],
                        &[],
                        count as i64,
                        &format!("cut-{count}"),
                        count as i64 - 1,
                        None,
                    );
                    let inputs = units_from_segments(&segments, &layer.segments)
                        .unwrap_or_else(|e| panic!("{name} 前缀 {count}：{e}"));
                    if !inputs.completed.is_empty() {
                        nonvacuous += 1;
                        INJECT_LAYER_HI_DELTA.with(|c| c.set(Some((0, 1))));
                        let err = units_from_segments(&segments, &layer.segments).unwrap_err();
                        INJECT_LAYER_HI_DELTA.with(|c| c.set(None));
                        assert!(err.starts_with("TB02E_INPUT_CONTRACT"), "{err}");
                        let receipt: Value = serde_json::from_str(
                            err.split_once("：").unwrap().1.split_once("：").unwrap().1,
                        )
                        .unwrap();
                        assert_eq!(receipt[0]["check"], "hi");
                        assert_eq!(receipt[0]["position"]["completed_index"], "0");
                        assert_eq!(
                            receipt[0]["position"]["unit_id"],
                            json!(inputs.completed[0].unit_id)
                        );
                    }
                }
            }
        }
        assert!(nonvacuous > 0, "冻结 raw 须有完成段（非真空）");
    }

    /// schema v1.3 §1（根裁定 ROOT-RULING-1419-L3）真实数据判别例：BTC 尾 5 万根前 600 根上的 CC-013 输入合同字段与
    /// 生产塔 L0 单元（夹具来源见其 provenance）。有段的几何端点不是段内极值；输入合同取成员笔方向极值，与塔逐位相同
    /// 才接纳——改回几何端点即在这些段上整 cut 拒绝（变异体收据见 #1419 收尾报告）。
    #[test]
    fn tb02e_real_btc_unit_interval_is_member_stroke_extremes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02e-real/btc-tail50k-first600.json"
        ))
        .unwrap();
        let objects: Vec<Value> = fixture["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| json!({"kind":r["kind"],"object_id":r["object_id"],"payload_json":r["payload"].to_string(),"source_coords_json":"[]"}))
            .collect();
        let tower: Vec<Segment> = fixture["tower_units"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| {
                let (lo, hi) = (u["lo"].as_i64().unwrap(), u["hi"].as_i64().unwrap());
                let up = u["dir"] == "UP";
                Segment {
                    direction: if up { Direction::Up } else { Direction::Down },
                    start_index: u["start_index"].as_u64().unwrap() as usize,
                    end_index: u["end_index"].as_u64().unwrap() as usize,
                    start_price: if up { lo } else { hi },
                    end_price: if up { hi } else { lo },
                }
            })
            .collect();
        let inputs = units_from_segments(&objects, &tower).unwrap_or_else(|e| panic!("{e}"));
        let got: Vec<(i64, i64)> = inputs
            .completed
            .iter()
            .map(|u| (u.range.unwrap().lo, u.range.unwrap().hi))
            .collect();
        let want: Vec<(i64, i64)> = tower
            .iter()
            .map(|s| {
                (
                    s.start_price.min(s.end_price),
                    s.start_price.max(s.end_price),
                )
            })
            .collect();
        assert_eq!(got, want);
        assert!(inputs.in_progress.is_some());
        // 判别：这些段的几何端点区间与塔（成员笔极值）不同。
        let disc: Vec<usize> =
            serde_json::from_value(fixture["discriminating_units"].clone()).unwrap();
        assert!(!disc.is_empty());
        for k in disc {
            let d = &fixture["segments"][k]["payload"]["data"];
            let a: i64 = d["stroke_refs"][0]["start_price"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            let b: i64 = d["geometric_end"]["price"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_ne!((a.min(b), a.max(b)), want[k], "单元 {k} 不是判别例");
        }
    }

    /// S0 oracle 六条流（`worked-examples.json` 的 `streams.*.endpoints`）。
    const STREAMS: [(&str, &[i64]); 6] = [
        ("UP", &UP),
        (
            "DOWN",
            &[35, 5, 25, 10, 15, 7, 24, 20, 40, 30, 43, 33, 41, 36, 39],
        ),
        ("E4", &[10, 40, 20, 35, 5, 15, 2, 18, 15, 17, 1, 8, 6]),
        (
            "E4_DOWN",
            &[35, 5, 25, 10, 40, 30, 43, 27, 30, 28, 44, 37, 39],
        ),
        ("LEAD", &[10, 40, 5, 9, 3, 20, 12, 15]),
        ("W9", &[10, 40, 20, 35, 25, 38, 22, 36, 24, 37, 26, 28]),
    ];

    fn stream(name: &str) -> &'static [i64] {
        STREAMS.iter().find(|(n, _)| *n == name).unwrap().1
    }

    /// schema v1.1 §3 七个「seed 起点为进行中单元」切面与 W9 K8/K9 的唯一编码（根裁定 D-J2 第 1、2 点）。
    #[test]
    fn tb02e_cursor_entity_anchors_match_schema_v1_1_table() {
        for (name, k, start, after) in [
            ("UP", 8, 9, 8),
            ("DOWN", 8, 9, 8),
            ("E4", 4, 5, 4),
            ("E4", 10, 11, 10),
            ("E4_DOWN", 4, 5, 4),
            ("E4_DOWN", 10, 11, 10),
            ("LEAD", 5, 6, 5),
        ] {
            let objects = run(&stream(name)[..k + 2], &[], 20);
            assert_eq!(
                data_of_version(&objects)["scan_cursor"],
                json!({"state":"SEEKING_SEED","center_id":null,"seed_start_unit_id":seg(start),"awaiting_unit_id":null,
                    "awaiting_after_unit_id":seg(after),"awaiting_offset":"3"}),
                "{name} K{k}"
            );
        }
        for k in [8, 9] {
            let objects = run(&stream("W9")[..k + 2], &[], 20);
            assert_eq!(
                data_of_version(&objects)["scan_cursor"],
                json!({"state":"EXTENDING","center_id":"center:1:10","seed_start_unit_id":null,"awaiting_unit_id":seg(k + 1),
                    "awaiting_after_unit_id":null,"awaiting_offset":null}),
                "W9 K{k}"
            );
            let c = data_of(&objects, KINDS[1], "center:1:10");
            assert_eq!(
                (c["status"].clone(), c["next_tested_unit"].clone()),
                (json!("UPGRADE_PENDING_TB02H"), json!(seg(k + 1)))
            );
            assert_eq!(
                c["members"].as_array().unwrap().len(),
                k + 1,
                "UPGRADE 修订 members 照列"
            );
            for u in 0..=k {
                let a = data_of(&objects, KINDS[3], &format!("assign:{}", seg(u)));
                assert_eq!(
                    (
                        a["status"].clone(),
                        a["reason"].clone(),
                        a["center_id"].clone()
                    ),
                    (
                        json!("NOT_YET_ASSIGNED"),
                        json!("UPGRADE_PENDING_TB02H"),
                        Value::Null
                    ),
                    "W9 K{k} u{u}"
                );
            }
        }
    }

    /// 段层合成输入对应的同 cut `CC-013.segment` 存根（Q 输入合同只读这些字段）。
    fn segment_stubs(endpoints: &[i64]) -> Vec<Value> {
        let n = endpoints.len() - 1;
        (0..n)
            .map(|k| json!({"kind":"CC-013.segment","object_id":format!("obj-seg-{k}"),"payload":{"schema_revision":"s-segment/2","data":{
                "entity_id":seg(k),"result":"CASE_ONE","construction":{"start_stroke":(3 * k).to_string(),"direction":if endpoints[k + 1] > endpoints[k] {"UP"} else {"DOWN"},
                    "stroke_indices":[(3 * k).to_string()]},
                "stroke_refs":[{"stroke_index":(3 * k).to_string(),"start_price":endpoints[k].to_string(),"end_price":endpoints[k + 1].to_string()}],
                "geometric_end":{"price":endpoints[k + 1].to_string()}}}}))
            .chain(std::iter::once(json!({"kind":"CC-013.segment","object_id":format!("obj-seg-{n}"),"payload":{"schema_revision":"s-segment/2","data":{
                "entity_id":seg(n),"result":"NO_FIRST","construction":{"start_stroke":(3 * n).to_string(),"direction":null},
                "stroke_refs":[{"start_price":endpoints[n].to_string()}],"geometric_end":null}}})))
            .collect()
    }

    /// #804 L3 的 S→Q 半边：写者在六条 oracle 流全部切面上的输出（逐 cut 以前一 cut 为 previous）都过 Python Q 合同
    /// `validate_center_cut`（单对象 + 规则修订 + 跨对象 + 游标 + 目录轴绑定）。
    #[test]
    fn tb02e_writer_output_passes_python_center_contract_on_all_oracle_cuts() {
        use std::io::Write;
        let mut cases = vec![];
        for (name, endpoints) in STREAMS {
            let mut previous: Vec<Value> = vec![];
            for k in 0..endpoints.len() - 1 {
                let cut = &endpoints[..k + 2];
                let objects = run(cut, &previous, k as i64 + 2);
                let mut records: Vec<Value> = segment_stubs(cut);
                records.extend(objects.iter().map(|o| json!({"kind":o["kind"],"object_id":o["object_id"],"payload":payload_of(o)})));
                let ids = |kinds: &[&str]| -> Vec<Value> {
                    objects
                        .iter()
                        .filter(|o| kinds.contains(&o["kind"].as_str().unwrap()))
                        .map(|o| o["object_id"].clone())
                        .collect()
                };
                let axes = json!({"CC-016":{"evidence":{"object_ids":ids(&KINDS[..2])}},"CC-017":{"evidence":{"object_ids":ids(&KINDS[2..3])}},
                    "CC-022":{"evidence":{"object_ids":ids(&KINDS[3..4])}}});
                cases.push(json!({"name":format!("{name} K{k}"),"records":records,"axes":axes}));
                previous = objects;
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../s_session");
        let program = "import json,sys;sys.path.insert(0,sys.argv[1]);import s_tb02_contract as c\nbad=[]\nfor case in json.load(sys.stdin):\n try: c.validate_center_cut(case['records'],c.CENTER_RULE,case['axes'])\n except ValueError as e: bad.append((case['name'],str(e)))\nprint(len(bad));[print(b) for b in bad];sys.exit(1 if bad else 0)\n";
        let mut child = std::process::Command::new("python3")
            .arg("-c")
            .arg(program)
            .arg(&root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(serde_json::to_string(&cases).unwrap().as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(cases.len() > 50, "非真空：{} 个切面", cases.len());
    }
}
