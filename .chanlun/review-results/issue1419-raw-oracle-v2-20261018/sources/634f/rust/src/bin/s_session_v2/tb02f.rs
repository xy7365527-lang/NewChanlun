//! #1424 TB-02-F：相邻中枢五叶关系（CC-020）与延伸测试单元位置（CC-018）→ S 事实对象
//! （schema `s-center-pair/1`，`.chanlun/review-results/issue1424-tb02f-20261014/tb1424-schema.md`）。
//!
//! 本模块不重新扫描、不另写判据：吃 `tb02e::build` 同一次扫描（[`tb02e::CenterBuild`]，#804 L2），对与位置全走
//! 库内 `center_scan::{pair_facts, tested_positions}` → `center::{center_pair_facts, unit_core_position}`（塔的
//! `classify_relation` 与延伸循环读同一判定）。本模块只做身份、修订、获知时点与 wire 投影。
//!
//! 根裁定（#1424 issuecomment-6160553615）：一端为 ≥9 窗口的对照发对象、`status = PENDING_UPGRADE_TB02H`、叶/方向/
//! 矩阵为 null（第 2 问）；扩展/新生只是关系，`realization` 固定写「未构造/未断言」（票面验收二）；离开不是破坏，
//! `destruction` 固定写待 #1434；两段式结算 R5 归 #1496（第 6 问）。
use super::*;
use newchan_rust::theta_v0::classifier::center::{PairEndpoint, PairLeaf};
use newchan_rust::theta_v0::classifier::center_scan::PairOutcome;

pub(super) const KINDS: &[&str] = &[
    "CC-018.unit_core_position",
    "CC-020.center_pair",
    "CC-055.pair_relation",
    "CC-055.pair_change",
    "CC-056.pair_version",
];
pub(super) const SCHEMA_REVISION: &str = "s-center-pair/1";
const SEMANTIC_VERSION: &str = "center-pair/1";
const POLICY_VERSION: &str = "zn_outer_five_leaf_pending_ge9";
const DELIVERED_SCOPE: &str = "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position";
/// 版本对象与 CC-018/CC-020 目录轴的延后范围（带实票号）。
pub(super) const DEFERRED_SCOPE: [&str; 6] = [
    "upper_level_pairs_1429",
    "ge9_pairs_pending_1429",
    "upper_center_realization_1429",
    "trend_realization_1426",
    "first_retest_destruction_1434",
    "two_step_settlement_r5_1496",
];
pub(super) const DELIVERED_SUBOBLIGATIONS: [&str; 3] = [
    "l0_unit_core_position_five_leaves",
    "l0_adjacent_pair_five_leaves_zn_outer_with_full_matrix",
    "expansion_relation_separate_from_realization",
];
/// 本规则下扩展/新生关系的实现状态（固定字面量：本片不构造上级对象、不断言趋势已实现）。
pub(super) const REALIZATION_UPPER: &str = "NOT_CONSTRUCTED_DEFERRED_1429";
pub(super) const REALIZATION_TREND: &str = "NOT_ASSERTED_DEFERRED_1426";
/// 离开不是破坏：首回试与破坏归 #1434。
pub(super) const DESTRUCTION: &str = "NOT_EVALUATED_DEFERRED_1434";
pub(super) const PENDING: &str = "PENDING_UPGRADE_TB02H";

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

fn payload_data(o: &Value) -> Value {
    serde_json::from_str::<Value>(o["payload_json"].as_str().expect("对象携 payload_json"))
        .expect("payload_json 是 JSON")["data"]
        .clone()
}

fn ord_str(o: std::cmp::Ordering) -> &'static str {
    match o {
        std::cmp::Ordering::Less => "<",
        std::cmp::Ordering::Equal => "=",
        std::cmp::Ordering::Greater => ">",
    }
}

fn center_anchor(center_id: &str) -> &str {
    center_id.strip_prefix("center:").unwrap_or(center_id)
}

/// 去掉修订簿记字段后比较（这些字段不改变事实本身）。
fn same_fact(old: &Value, new: &Value, bookkeeping: &[&str]) -> bool {
    let strip = |v: &Value| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            for k in bookkeeping {
                o.remove(*k);
            }
        }
        v
    };
    strip(old) == strip(new)
}

fn union_sources<'a>(units: impl IntoIterator<Item = &'a tb02e::SegUnit>) -> Vec<usize> {
    let mut s: Vec<usize> = units
        .into_iter()
        .flat_map(|u| u.sources.iter().copied())
        .collect();
    s.sort_unstable();
    s.dedup();
    s
}

/// CC-018/CC-020/CC-055/CC-056 对象（输入合同与 `tb02e::build` 之后的纯投影）。返回 (对象, 关系)。
pub(super) fn build(
    inputs: &tb02e::UnitInputs,
    cb: &tb02e::CenterBuild,
    raw: &BTreeMap<i64, Value>,
    previous: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
) -> (Vec<Value>, Vec<Value>) {
    let known = &cb.known;
    let mut b = tb02a_facts::Builder {
        raw,
        generation: gen,
        cut,
        objects: vec![],
    };
    let prior: BTreeMap<(String, String), Value> = previous
        .iter()
        .filter(|o| o["kind"].as_str().is_some_and(|k| KINDS[..2].contains(&k)))
        .map(|o| {
            let d = payload_data(o);
            (
                (
                    o["kind"].as_str().unwrap().to_string(),
                    d["entity_id"].as_str().unwrap_or_default().to_string(),
                ),
                d,
            )
        })
        .collect();
    let ids: Vec<&str> = inputs
        .completed
        .iter()
        .map(|u| u.unit_id.as_str())
        .collect();
    let mut edges: Vec<(String, &str, String, String, String, Vec<usize>)> = vec![];
    let mut relations = vec![];

    // ── CC-018.unit_core_position ──
    for (w, tested) in cb.facts.tested_positions(&cb.units).iter().enumerate() {
        let center_id = &cb.center_ids[w];
        let win = &cb.windows[w];
        let core = cb.facts.window_center(&cb.units, w);
        let seed = &inputs.completed[win.first..win.first + 3];
        for t in tested {
            let unit = &inputs.completed[t.unit];
            let entity = format!(
                "ucp:{}:{}",
                center_anchor(center_id),
                tb02e::anchor_of(&unit.unit_id)
            );
            let fact = tb02c::wire(
                json!({"entity_id":entity,"center_id":center_id,"unit":tb02e::unit_json(unit),
                "core":{"ZD":core.zd.to_string(),"ZG":core.zg.to_string()},"position":t.position.as_str(),
                "closed_overlap":t.position.closed_overlap(),"strict_leave":t.position.leave_side().is_some(),
                "leave_side":t.position.leave_side().map(dir_str),"construction_role":t.role.as_str(),
                "destruction":DESTRUCTION}),
            );
            let data = match prior.get(&(KINDS[0].to_string(), entity.clone())) {
                Some(p) if same_fact(p, &fact, &["center_revision", "known_at"]) => p.clone(),
                _ => {
                    let mut d = fact;
                    d["center_revision"] = json!(cb.center_revisions[w].to_string());
                    d["known_at"] = known.clone();
                    tb02c::wire(d)
                }
            };
            let version = data["center_revision"].as_str().unwrap_or("1").to_string();
            let sources = union_sources(seed.iter().chain(std::iter::once(unit)));
            let oid = add(
                &mut b,
                KINDS[0],
                json!(entity),
                cb.fact_generation,
                data,
                sources.clone(),
            );
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
                center_id.clone(),
                version,
                oid,
                sources,
            ));
        }
    }

    // ── CC-020.center_pair + CC-055.pair_change ──
    let mut pair_ids = vec![];
    let mut pending = 0usize;
    for p in cb.facts.pair_facts(&cb.units) {
        let (a_id, b_id) = (&cb.center_ids[p.left], &cb.center_ids[p.right]);
        let entity = format!("pair:{}:{}", center_anchor(a_id), center_anchor(b_id));
        let side = |w: usize| {
            let c = cb.facts.window_center(&cb.units, w);
            let outer = !cb.windows[w].upgrade_pending;
            json!({"ZD":c.zd.to_string(),"ZG":c.zg.to_string(),
                "DD":outer.then(|| c.dd.to_string()),"GG":outer.then(|| c.gg.to_string())})
        };
        let (status, leaf, matrix, witnesses, null_reason) = match p.outcome {
            PairOutcome::Evaluated(f) => {
                let mut m = serde_json::Map::new();
                for x in PairEndpoint::ALL {
                    for y in PairEndpoint::ALL {
                        m.insert(
                            format!("B.{}?A.{}", x.as_str(), y.as_str()),
                            json!(ord_str(f.cmp(x, y))),
                        );
                    }
                }
                (
                    "EVALUATED",
                    Some(f.leaf),
                    Value::Object(m),
                    json!({"core_tangent":f.core_tangent(),"outer_touch":f.outer_touch(),
                        "outer_containment":f.outer_containment().as_str(),"core_containment":f.core_containment().as_str()}),
                    Value::Null,
                )
            }
            PairOutcome::PendingUpgrade => {
                pending += 1;
                (PENDING, None, Value::Null, Value::Null, json!(PENDING))
            }
        };
        let fact = tb02c::wire(
            json!({"entity_id":entity,"a_center_id":a_id,"a_revision":cb.center_revisions[p.left].to_string(),
            "b_center_id":b_id,"b_revision":cb.center_revisions[p.right].to_string(),
            "connector_units":p.connectors.clone().map(|k| ids[k]).collect::<Vec<_>>(),
            "status":status,"leaf":leaf.map(PairLeaf::as_str),"direction":leaf.and_then(PairLeaf::direction).map(dir_str),
            "provisional":!cb.windows[p.right].settled,"inputs":{"A":side(p.left),"B":side(p.right)},
            "matrix":matrix,"witnesses":witnesses,
            "realization":{"upper_center":REALIZATION_UPPER,"trend":REALIZATION_TREND},"null_reason":null_reason}),
        );
        let old = prior.get(&(KINDS[1].to_string(), entity.clone()));
        let bookkeeping = ["entity_revision", "previous", "formed_known_at", "known_at"];
        let (data, change) = match old {
            Some(o) if same_fact(o, &fact, &bookkeeping) => (o.clone(), None),
            _ => {
                let revision = old
                    .and_then(|o| o["entity_revision"].as_str()?.parse::<i64>().ok())
                    .unwrap_or(0)
                    + 1;
                let mut d = fact;
                d["entity_revision"] = json!(revision.to_string());
                d["previous"] = old.map_or(Value::Null, |o| {
                    json!({"entity_revision":o["entity_revision"],"status":o["status"],"leaf":o["leaf"]})
                });
                d["formed_known_at"] = old.map_or(known.clone(), |o| o["formed_known_at"].clone());
                d["known_at"] = known.clone();
                let change = match old {
                    None => "formed",
                    Some(o) if o["status"] != d["status"] && d["status"] == PENDING => {
                        "pending_upgrade"
                    }
                    Some(o) if o["leaf"] != d["leaf"] => "leaf_revised",
                    Some(_) => "inputs_revised",
                };
                (tb02c::wire(d), Some(change))
            }
        };
        let (wa, wb) = (&cb.windows[p.left], &cb.windows[p.right]);
        let sources = union_sources(&inputs.completed[wa.first..=wb.last]);
        let oid = add(
            &mut b,
            KINDS[1],
            json!(entity),
            cb.fact_generation,
            data.clone(),
            sources.clone(),
        );
        let version = data["entity_revision"].as_str().unwrap().to_string();
        for (kind, target) in [("pair_from", a_id), ("pair_to", b_id)] {
            edges.push((
                entity.clone(),
                kind,
                target.clone(),
                version.clone(),
                oid.clone(),
                sources.clone(),
            ));
        }
        if let Some(change) = change {
            add(
                &mut b,
                KINDS[3],
                json!([gen.to_string(), entity, change]),
                cb.fact_generation,
                json!({"entity_id":entity,"change":change,"before":old,"after":data,"known_at":known,"object_id":oid}),
                sources,
            );
        }
        pair_ids.push(entity);
    }
    for ((kind, entity), p) in &prior {
        if kind == KINDS[1] && !pair_ids.contains(entity) {
            add(
                &mut b,
                KINDS[3],
                json!([gen.to_string(), entity, "withdrawn"]),
                cb.fact_generation,
                json!({"entity_id":entity,"change":"withdrawn","before":p,"after":null,"known_at":known,"object_id":null}),
                raw.keys().map(|k| *k as usize).collect(),
            );
        }
    }

    // ── CC-055.pair_relation ──
    for (source, kind, target, version, witness, sources) in edges {
        let edge = json!({"source_id":source,"relation_kind":kind,"target_id":target,"version":version,"witness_object_id":witness});
        add(
            &mut b,
            KINDS[2],
            json!([source, kind, target]),
            cb.fact_generation,
            edge,
            sources,
        );
        relations.push(json!({"subject":source,"relation_type":kind,"object":target}));
    }

    // ── CC-056.pair_version ──
    let waiting: Vec<&str> = if pending > 0 { vec![PENDING] } else { vec![] };
    add(
        &mut b,
        KINDS[4],
        json!("TB-02-F"),
        cb.fact_generation,
        json!({"input_frontier":frontier.to_string(),"known_at":known,"pair_count":pair_ids.len().to_string(),
            "pending_pairs":pending.to_string(),"waiting_reasons":waiting,
            "delivered_scope":DELIVERED_SCOPE,"deferred_scope":DEFERRED_SCOPE}),
        raw.keys().map(|k| *k as usize).collect(),
    );
    (b.objects, relations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::theta_v0::classifier::center::UnitRange;

    /// 段层合成输入（同 `tb02e` 测试：端点 `P_k→P_{k+1}` 为完成单元，末端挂一个进行中单元，来源坐标 = {k}）。
    fn synthetic(endpoints: &[i64]) -> (tb02e::UnitInputs, BTreeMap<i64, Value>) {
        let completed: Vec<tb02e::SegUnit> = endpoints
            .windows(2)
            .enumerate()
            .map(|(k, w)| tb02e::SegUnit {
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
        let in_progress = Some(tb02e::SegUnit {
            unit_id: format!("segment:1:{}", 10 + 4 * n),
            object_id: format!("obj-seg-{n}"),
            range: None,
            sources: vec![n],
        });
        (
            tb02e::UnitInputs {
                completed,
                in_progress,
            },
            raw,
        )
    }

    /// 一个 cut：先 `tb02e::build`（与会话同参），再 `tb02f::build` 吃同一次扫描；返回 (中枢对象, 对对象)。
    fn run(endpoints: &[i64], previous: &[Value], gen: i64) -> (Vec<Value>, Vec<Value>) {
        let (inputs, raw) = synthetic(endpoints);
        let cut = format!("cut-{gen}");
        let (centers, _, cb) =
            tb02e::build(&inputs, &raw, &[], previous, gen, &cut, gen - 1, None).unwrap();
        let (pairs, _) = build(&inputs, &cb, &raw, previous, gen, &cut, gen - 1);
        (centers, pairs)
    }

    fn data_of(objects: &[Value], kind: &str, entity: &str) -> Value {
        objects
            .iter()
            .find(|o| o["kind"] == kind && payload_data(o)["entity_id"] == entity)
            .map(payload_data)
            .unwrap_or_else(|| panic!("缺 {kind} {entity}"))
    }

    fn of_kind(objects: &[Value], kind: &str) -> Vec<Value> {
        objects
            .iter()
            .filter(|o| o["kind"] == kind)
            .map(payload_data)
            .collect()
    }

    fn seg(k: usize) -> String {
        format!("segment:1:{}", 10 + 4 * k)
    }

    const BASE: [i64; 9] = [10, 40, 20, 35, 30, 38, 21, 25, 5];
    fn stream(tail: &[i64]) -> Vec<i64> {
        BASE.iter().chain(tail).copied().collect()
    }

    /// W1（#1419 UP 流 K12）：A→B 向下扩展；16 格矩阵、证据、连接段、输入取 CC-017 Zn 外缘；`realization`
    /// 固定为「未构造/未断言」；B 未结算 ⟹ provisional。CC-018：A 的 u3..u7 是延伸成员，u8 是突破段、向下离开、
    /// 破坏不评估；B 的 u12 是延伸成员。期待值取自 oracle 夹具 W1。
    #[test]
    fn tb02f_w1_pair_down_expansion_and_unit_positions() {
        let (centers, pairs) = run(&stream(&[15, 2, 12, 4, 9]), &[], 13);
        let p = data_of(&pairs, KINDS[1], "pair:1:10:1:46");
        assert_eq!(
            (
                p["status"].clone(),
                p["leaf"].clone(),
                p["direction"].clone(),
                p["provisional"].clone()
            ),
            (
                json!("EVALUATED"),
                json!("DOWN_EXPANSION"),
                json!("DOWN"),
                json!(true)
            )
        );
        assert_eq!(p["a_center_id"], "center:1:10");
        assert_eq!(p["b_center_id"], "center:1:46");
        assert_eq!(
            p["b_revision"],
            data_of(&centers, tb02e::KINDS[1], "center:1:46")["entity_revision"]
        );
        assert_eq!(p["connector_units"], json!([seg(8)]));
        assert_eq!(
            p["inputs"],
            json!({"A":{"ZD":"20","ZG":"35","DD":"10","GG":"40"},"B":{"ZD":"4","ZG":"12","DD":"2","GG":"15"}})
        );
        let m = p["matrix"].as_object().unwrap();
        assert_eq!(m.len(), 16);
        assert_eq!(
            (
                m["B.ZD?A.ZG"].clone(),
                m["B.ZG?A.ZD"].clone(),
                m["B.DD?A.GG"].clone(),
                m["B.GG?A.DD"].clone()
            ),
            (json!("<"), json!("<"), json!("<"), json!(">"))
        );
        assert_eq!(
            p["witnesses"],
            json!({"core_tangent":false,"outer_touch":false,"outer_containment":"NONE","core_containment":"NONE"})
        );
        assert_eq!(
            p["realization"],
            json!({"upper_center":REALIZATION_UPPER,"trend":REALIZATION_TREND})
        );
        assert!(p.get("level_lift").is_none());
        assert_eq!(p["entity_revision"], "1");
        assert_eq!(p["previous"], Value::Null);
        let ucp = of_kind(&pairs, KINDS[0]);
        let a_pos: Vec<(Value, Value, Value)> = ucp
            .iter()
            .filter(|u| u["center_id"] == "center:1:10")
            .map(|u| {
                (
                    u["unit"]["unit_id"].clone(),
                    u["position"].clone(),
                    u["construction_role"].clone(),
                )
            })
            .collect();
        let mut want: Vec<(Value, Value, Value)> = (3..=7)
            .map(|k| {
                (
                    json!(seg(k)),
                    json!("INTERIOR_CONTACT"),
                    json!("EXTENSION_MEMBER"),
                )
            })
            .collect();
        want.push((json!(seg(8)), json!("STRICT_BELOW"), json!("BREAK_UNIT")));
        assert_eq!(a_pos, want);
        let brk = data_of(&pairs, KINDS[0], "ucp:1:10:1:42");
        assert_eq!(
            (
                brk["leave_side"].clone(),
                brk["strict_leave"].clone(),
                brk["closed_overlap"].clone(),
                brk["destruction"].clone(),
                brk["core"].clone()
            ),
            (
                json!("DOWN"),
                json!(true),
                json!(false),
                json!(DESTRUCTION),
                json!({"ZD":"20","ZG":"35"})
            )
        );
        assert_eq!(
            data_of(&pairs, KINDS[0], "ucp:1:46:1:58")["position"],
            "INTERIOR_CONTACT"
        );
        let v = of_kind(&pairs, KINDS[4]);
        assert_eq!(v.len(), 1);
        assert_eq!(
            (
                v[0]["pair_count"].clone(),
                v[0]["pending_pairs"].clone(),
                v[0]["waiting_reasons"].clone(),
                v[0]["deferred_scope"].clone()
            ),
            (json!("1"), json!("0"), json!([]), json!(DEFERRED_SCOPE))
        );
        let rel: Vec<(Value, Value)> = of_kind(&pairs, KINDS[2])
            .iter()
            .filter(|r| r["source_id"] == "pair:1:10:1:46")
            .map(|r| (r["relation_kind"].clone(), r["target_id"].clone()))
            .collect();
        assert_eq!(
            rel,
            vec![
                (json!("pair_from"), json!("center:1:10")),
                (json!("pair_to"), json!("center:1:46"))
            ]
        );
    }

    /// W12（三切面）：K11 formed DOWN_NEWBIRTH；K12 B 加成员 ⟹ 修订 2（inputs_revised，叶不变）；K13 B.GG 9→12
    /// ⟹ 修订 3、leaf_revised 为 DOWN_EXPANSION，previous 指回修订 2，formed_known_at 保持 K11（A-ST-047-05）。
    #[test]
    fn tb02f_w12_revision_chain_newbirth_to_expansion() {
        let s = stream(&[9, 2, 8, 3, 12, 4]);
        let (c11, p11) = run(&s[..13], &[], 12);
        let prev11: Vec<Value> = c11.iter().chain(&p11).cloned().collect();
        let (c12, p12) = run(&s[..14], &prev11, 13);
        let prev12: Vec<Value> = c12.iter().chain(&p12).cloned().collect();
        let (_, p13) = run(&s[..15], &prev12, 14);
        let e = "pair:1:10:1:46";
        let (a, b, c) = (
            data_of(&p11, KINDS[1], e),
            data_of(&p12, KINDS[1], e),
            data_of(&p13, KINDS[1], e),
        );
        assert_eq!(
            (a["entity_revision"].clone(), a["leaf"].clone()),
            (json!("1"), json!("DOWN_NEWBIRTH"))
        );
        assert_eq!(
            (b["entity_revision"].clone(), b["leaf"].clone()),
            (json!("2"), json!("DOWN_NEWBIRTH"))
        );
        assert_eq!(
            (c["entity_revision"].clone(), c["leaf"].clone()),
            (json!("3"), json!("DOWN_EXPANSION"))
        );
        assert_eq!(
            c["previous"],
            json!({"entity_revision":"2","status":"EVALUATED","leaf":"DOWN_NEWBIRTH"})
        );
        assert_eq!(c["formed_known_at"], a["formed_known_at"]);
        assert_eq!(c["known_at"]["generation"], "14");
        let changes = |objs: &[Value]| -> Vec<Value> {
            of_kind(objs, KINDS[3])
                .iter()
                .map(|x| x["change"].clone())
                .collect()
        };
        assert_eq!(changes(&p11), vec![json!("formed")]);
        assert_eq!(changes(&p12), vec![json!("inputs_revised")]);
        assert_eq!(changes(&p13), vec![json!("leaf_revised")]);
    }

    /// 同一输入、前一 cut 已发布且事实未变 ⟹ 对象逐值沿用（修订号与获知时点不前移）。
    #[test]
    fn tb02f_unchanged_facts_keep_revision_and_known_at() {
        let s = stream(&[15, 2, 12, 4, 9]);
        let (c1, p1) = run(&s, &[], 13);
        let prev: Vec<Value> = c1.iter().chain(&p1).cloned().collect();
        let (_, p2) = run(&s, &prev, 14);
        assert_eq!(
            data_of(&p2, KINDS[1], "pair:1:10:1:46"),
            data_of(&p1, KINDS[1], "pair:1:10:1:46")
        );
        assert_eq!(
            data_of(&p2, KINDS[0], "ucp:1:10:1:42"),
            data_of(&p1, KINDS[0], "ucp:1:10:1:42")
        );
        assert!(of_kind(&p2, KINDS[3]).is_empty(), "事实未变不发变化事件");
    }

    /// W9（根裁定第 2 问）：A 12 成员 UPGRADE_PENDING_TB02H ⟹ 对 A→B 照发、PENDING_UPGRADE_TB02H、叶/方向/矩阵/证据为
    /// null、A 的 DD/GG 为 null 带 null_reason；A 的延伸成员角色为 UPGRADE_PENDING_MEMBER；版本对象记等待原因。
    #[test]
    fn tb02f_w9_pair_pending_upgrade_not_copied_from_tower() {
        let (_, pairs) = run(
            &[
                10, 40, 20, 35, 30, 38, 22, 36, 25, 33, 21, 26, 5, 9, 2, 8, 3,
            ],
            &[],
            17,
        );
        let p = data_of(&pairs, KINDS[1], "pair:1:10:1:62");
        assert_eq!(p["status"], PENDING);
        assert_eq!(p["null_reason"], PENDING);
        for key in ["leaf", "direction", "matrix", "witnesses"] {
            assert_eq!(p[key], Value::Null, "{key}");
        }
        assert_eq!(
            p["inputs"]["A"],
            json!({"ZD":"20","ZG":"35","DD":null,"GG":null})
        );
        assert_eq!(p["connector_units"], json!([seg(12)]));
        let roles: Vec<Value> = of_kind(&pairs, KINDS[0])
            .iter()
            .filter(|u| u["center_id"] == "center:1:10")
            .map(|u| u["construction_role"].clone())
            .collect();
        assert_eq!(roles.len(), 10);
        assert!(roles[..9].iter().all(|r| r == "UPGRADE_PENDING_MEMBER"));
        assert_eq!(roles[9], "BREAK_UNIT");
        let v = &of_kind(&pairs, KINDS[4])[0];
        assert_eq!(v["waiting_reasons"], json!([PENDING]));
        assert_eq!(v["pending_pairs"], "1");
    }

    /// B 在对已发布后长到 9 成员：新修订记 pending_upgrade，前值留在 previous，不发 withdrawn（LC-03「九段优先级
    /// 不抹别的关系事实」，AsKnown）。
    #[test]
    fn tb02f_pair_turns_pending_when_b_reaches_nine_members() {
        let s = stream(&[9, 2, 8, 3, 7, 4, 7, 4, 7, 4, 7]);
        // B = u9.. 起：K16 成员 8，K17 成员 9。
        let (c16, p16) = run(&s[..18], &[], 17);
        assert_eq!(
            data_of(&p16, KINDS[1], "pair:1:10:1:46")["status"],
            "EVALUATED"
        );
        let prev: Vec<Value> = c16.iter().chain(&p16).cloned().collect();
        let (_, p17) = run(&s[..19], &prev, 18);
        let p = data_of(&p17, KINDS[1], "pair:1:10:1:46");
        assert_eq!(p["status"], PENDING);
        assert_eq!(p["previous"]["status"], "EVALUATED");
        let changes: Vec<Value> = of_kind(&p17, KINDS[3])
            .iter()
            .map(|x| x["change"].clone())
            .collect();
        assert_eq!(changes, vec![json!("pending_upgrade")]);
    }

    /// 同一输入两次 build 逐字节相同（A-CC-018-03、A-CC-020-03 的单进程半边）。
    #[test]
    fn tb02f_same_input_rebuilds_byte_identical() {
        let s = stream(&[9, 2, 8, 3, 12, 4]);
        assert_eq!(run(&s, &[], 15), run(&s, &[], 15));
    }

    /// #804 L3 的 S→Q 半边：写者在全部工作例流每个前缀切面上的输出（逐 cut 以前一 cut 为 previous，覆盖修订链、
    /// 撤回与待定翻转）都过 Python Q 合同 `validate_pair_cut`（内含中枢整 cut 门；对族：单对象 + 规则 + 跨对象 +
    /// 目录轴绑定）。流与 `s_session/tests/fixtures/tb02f-contract/generate-corpus.py` 同源。
    #[test]
    fn tb02f_writer_output_passes_python_pair_contract_on_all_prefix_cuts() {
        use std::io::Write;
        let up: Vec<i64> = stream(&[15, 2, 12, 4, 9, 6]);
        let w3: Vec<i64> = stream(&[9, 2, 8, 3, 5]);
        let streams: Vec<(&str, Vec<i64>)> = vec![
            ("UP", up.clone()),
            ("DOWN", up.iter().map(|p| 45 - p).collect()),
            ("W3", w3.clone()),
            ("W3_MIRROR", w3.iter().map(|p| 45 - p).collect()),
            ("W5", stream(&[10, 2, 8, 3, 5])),
            (
                "W6",
                vec![10, 40, 20, 35, 30, 50, 40, 60, 30, 40, 35, 38, 36],
            ),
            (
                "W7",
                vec![10, 40, 20, 35, 30, 50, 40, 55, 42, 48, 5, 60, 45, 50],
            ),
            (
                "W9",
                vec![
                    10, 40, 20, 35, 30, 38, 22, 36, 25, 33, 21, 26, 5, 9, 2, 8, 3, 5,
                ],
            ),
            (
                "W10",
                vec![10, 40, 20, 45, 40, 50, 12, 20, 8, 15, 3, 12, 5, 9],
            ),
            ("W12", stream(&[9, 2, 8, 3, 12, 4])),
            (
                "E1419_W9",
                vec![10, 40, 20, 35, 25, 38, 22, 36, 24, 37, 26, 28],
            ),
        ];
        let payload = |o: &Value| -> Value {
            serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap()
        };
        let mut cases = vec![];
        let mut evaluated = std::collections::BTreeSet::new();
        for (name, endpoints) in &streams {
            let mut previous: Vec<Value> = vec![];
            for k in 0..endpoints.len() - 1 {
                let cut = &endpoints[..k + 2];
                let (centers, pairs) = run(cut, &previous, k as i64 + 2);
                let objects: Vec<Value> = centers.iter().chain(&pairs).cloned().collect();
                let n = cut.len() - 1;
                let mut records: Vec<Value> = (0..n)
                    .map(|j| json!({"kind":"CC-013.segment","object_id":format!("obj-seg-{j}"),"payload":{"schema_revision":"s-segment/2","data":{
                        "entity_id":seg(j),"result":"CASE_ONE","construction":{"start_stroke":(3 * j).to_string(),"direction":if cut[j + 1] > cut[j] {"UP"} else {"DOWN"},
                            "stroke_indices":[(3 * j).to_string()]},
                        "stroke_refs":[{"stroke_index":(3 * j).to_string(),"start_price":cut[j].to_string(),"end_price":cut[j + 1].to_string()}],
                        "geometric_end":{"price":cut[j + 1].to_string()}}}}))
                    .collect();
                records.push(json!({"kind":"CC-013.segment","object_id":format!("obj-seg-{n}"),"payload":{"schema_revision":"s-segment/2","data":{
                    "entity_id":seg(n),"result":"NO_FIRST","construction":{"start_stroke":(3 * n).to_string(),"direction":null},
                    "stroke_refs":[{"start_price":cut[n].to_string()}],"geometric_end":null}}}));
                records.extend(objects.iter().map(
                    |o| json!({"kind":o["kind"],"object_id":o["object_id"],"payload":payload(o)}),
                ));
                let ids = |kinds: &[&str]| -> Vec<Value> {
                    objects
                        .iter()
                        .filter(|o| kinds.contains(&o["kind"].as_str().unwrap()))
                        .map(|o| o["object_id"].clone())
                        .collect()
                };
                let c = tb02e::KINDS;
                let axes = json!({"CC-016":{"evidence":{"object_ids":ids(&c[..2])}},"CC-017":{"evidence":{"object_ids":ids(&c[2..3])}},
                    "CC-022":{"evidence":{"object_ids":ids(&c[3..4])}},"CC-018":{"evidence":{"object_ids":ids(&KINDS[..1])}},
                    "CC-020":{"evidence":{"object_ids":ids(&KINDS[1..2])}}});
                for d in of_kind(&pairs, KINDS[1]) {
                    evaluated.insert(if d["status"] == "EVALUATED" {
                        d["leaf"].as_str().unwrap().to_string()
                    } else {
                        d["status"].as_str().unwrap().to_string()
                    });
                }
                cases.push(json!({"name":format!("{name} K{k}"),"records":records,"axes":axes}));
                previous = objects;
            }
        }
        assert_eq!(evaluated.len(), 6, "五叶与待定全覆盖：{evaluated:?}");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../s_session");
        let program = "import json,sys;sys.path.insert(0,sys.argv[1]);import s_tb02_contract as c\nbad=[]\ncases=json.load(sys.stdin)\nfor case in cases:\n try: c.validate_pair_cut(case['records'],c.PAIR_RULE,case['axes'])\n except ValueError as e: bad.append((case['name'],str(e)))\nprint(len(cases),len(bad));[print(b) for b in bad];sys.exit(1 if bad else 0)\n";
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
    }

    /// 尚无中枢 / 只有一个中枢：无对象（CC-018 只在有延伸测试单元时发），版本对象照发、计数为 0。
    #[test]
    fn tb02f_no_pair_until_second_center() {
        let (_, pairs) = run(&BASE[..4], &[], 4);
        assert!(of_kind(&pairs, KINDS[1]).is_empty());
        assert_eq!(of_kind(&pairs, KINDS[4])[0]["pair_count"], "0");
        let (_, pairs) = run(&stream(&[15, 2]), &[], 11);
        assert!(of_kind(&pairs, KINDS[1]).is_empty(), "B 尚未成立");
        assert_eq!(
            data_of(&pairs, KINDS[0], "ucp:1:10:1:42")["construction_role"],
            "BREAK_UNIT"
        );
    }
}
