// #1394 r3：r2 原样真实输入的永久回归；不改原 Fixture 或生产 facts。
use super::*;
type Row = (&'static str, i64, i64, i64, i64, i64, i64);

fn audit_summary(audited: &state::AuditedState) -> Value {
    json!({
        "raw":audited.raw,
        "cuts":audited.cuts.iter().map(|cut| json!({
            "metadata":format!("{:?}", cut.metadata),
            "roots":format!("{:?}", cut.roots),
            "snapshot":cut.snapshot,"delta":cut.delta
        })).collect::<Vec<_>>(),
        "objects":format!("{:?}", audited.objects),
        "witnesses":format!("{:?}", audited.auxiliary.witnesses),
        "relations":format!("{:?}", audited.auxiliary.relations),
        "observations":format!("{:?}", audited.auxiliary.observations)
    })
}

fn run_normal(scenario: &'static str, rows: [Row; 4]) {
    let mut fixture = Fixture::new();
    let mut prior_expected = Vec::new();
    for (offset, (id, coord, open, high, low, close, revision)) in rows.into_iter().enumerate() {
        let step = offset + 1;
        let (envelope, token) =
            fixture.accept_begin(step as i64, (id, coord, open, high, low, close), revision);
        fixture
            .publish(&envelope, &token)
            .unwrap_or_else(|error| panic!("{scenario} step={step} publish: {error}"));
        let tx = fixture.conn.transaction().unwrap();
        let audited = state::audit(&tx)
            .unwrap_or_else(|error| panic!("{scenario} step={step} ordinary: {error}"));
        assert_eq!(audited.cuts.len(), step);
        let live_groups = audited
            .objects
            .iter()
            .filter(|object| {
                object.kind == content_producer::CC005_GROUP_KIND
                    && object.life.withdrawn_generation.is_none()
            })
            .count();
        // #1393 W（raw-initialization-w/1，S3 写者唯一规则）：等待期零正式组；首个相邻严格非包含对
        // 出现时从 raw0 按其方向重折（逐例手算，(high,low) 按 source_coord 序）：
        // - 同 OHLC 三根互含，第 4 根 (17,6) 对 (12,3) 严格向上 ⟹ 第 4 步重折为 2 组；
        // - 较早修订：(12,3)→(16,6) 严格向上 2 组；a 改 (13,4) 仍严格向上 2 组；a 改 (17,6) 包含 b ⟹ 回到等待期；
        // - 首来源修订：a=b 互含；c (17,6) 严格向上 ⟹ 2 组；a 改 (9,1) 后 a→b 严格向上 ⟹ 3 组；
        // - 后根包含初始根：b (14,2) 包含 a；c (16,4) 对 b 严格向上 ⟹ UP 重折 {a,b},{c} 2 组；d 再一组 3 组。
        let expected_groups = match (scenario, step) {
            ("乱序新身份且前三根同OHLC" | "顺序新身份且前三根同OHLC", 4) => 2,
            ("较早坐标修订", 2 | 3) => 2,
            ("正常首来源修订使未定后缀重新完成映射", 3) => 2,
            ("正常首来源修订使未定后缀重新完成映射", 4) => 3,
            ("正常后根包含初始根且后继保持未定", 3) => 2,
            ("正常后根包含初始根且后继保持未定", 4) => 3,
            _ => 0,
        };
        assert_eq!(
            live_groups, expected_groups,
            "{scenario} step={step}真实组数"
        );
        if matches!(scenario, "乱序新身份且前三根同OHLC" | "顺序新身份且前三根同OHLC" | "较早坐标修订") {
            prior_expected = assert_pending_w2_oracle(&tx, &audited, &prior_expected, step as i64);
        }
        eprintln!(
            "{scenario} step={step} publish+ordinary OK root={}",
            audited.cuts.last().unwrap().roots.generated_state.root()
        );
    }
    let tx = fixture.conn.transaction().unwrap();
    let ordinary = state::audit(&tx).unwrap();
    #[cfg(feature = "s_capture_native_testonly")]
    {
        let native = state::audit_native_current(&tx).unwrap();
        assert_eq!(audit_summary(&ordinary), audit_summary(&native));
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        let profile = super::super::q_allocation::Profile {
            budget: sc::capture_allocation::OwnedBudget::new(50 * 1024 * 1024),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        };
        let (owned, source) =
            state::audit_bounded_with_allocation(&tx, None, None, &profile).unwrap();
        assert_eq!(audit_summary(&ordinary), audit_summary(&owned));
        assert!(profile.budget.snapshot().live_requested_layout_bytes > 0);
        drop(owned);
        drop(source);
        let meter = profile.budget.snapshot();
        assert_eq!(meter.live_requested_layout_bytes, 0);
        assert_eq!(meter.allocated_layouts, meter.released_layouts);
    }
    // 同一已发布原件再次冷读，验证 replay 结果不依赖上次审计或临时索引。
    let replay = state::audit(&tx).unwrap();
    assert_eq!(audit_summary(&ordinary), audit_summary(&replay));
}

#[test]
fn pending_source_unordered_same_ohlc() {
    run_normal(
        "乱序新身份且前三根同OHLC",
        [
            ("a", 20, 6, 12, 3, 7, 1),
            ("b", 0, 6, 12, 3, 7, 1),
            ("c", 10, 6, 12, 3, 7, 1),
            ("d", 30, 8, 17, 6, 12, 1),
        ],
    );
}

#[test]
fn pending_source_ordered_same_ohlc() {
    run_normal(
        "顺序新身份且前三根同OHLC",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 6, 12, 3, 7, 1),
            ("c", 20, 6, 12, 3, 7, 1),
            ("d", 30, 8, 17, 6, 12, 1),
        ],
    );
}

#[test]
fn pending_source_earlier_revision() {
    run_normal(
        "较早坐标修订",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 8, 16, 6, 12, 1),
            ("a", 0, 8, 13, 4, 7, 2),
            ("a", 0, 8, 17, 6, 12, 3),
        ],
    );
}

#[test]
fn pending_source_revision_resolves_initial_suffix() {
    run_normal(
        "正常首来源修订使未定后缀重新完成映射",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 6, 12, 3, 7, 1),
            ("c", 20, 8, 17, 6, 12, 1),
            ("a", 0, 4, 9, 1, 6, 2),
        ],
    );
}

#[test]
fn pending_source_incoming_encloses_seed() {
    run_normal(
        "正常后根包含初始根且后继保持未定",
        [
            ("a", 0, 6, 12, 3, 7, 1),
            ("b", 10, 7, 14, 2, 8, 1),
            ("c", 20, 8, 16, 4, 12, 1),
            ("d", 30, 9, 17, 5, 14, 1),
        ],
    );
}

#[test]
fn bounded2_revision_restore_preserves_archived_cuts() {
    fn run(layout: super::super::super::layout::Layout) -> Result<(), String> {
        super::super::super::layout::with_forced_layout_for_test(Some(layout), || {
            let mut fixture = Fixture::with_clock_events(6, 3);
            let rows = crate::tests::tb02b_oracle_rows();
            assert_eq!(rows.len(), 14);
            let (envelope, token) = fixture.accept_begin_rows(1, &rows, 1);
            fixture.publish(&envelope, &token)?;
            let cuts = |f: &mut Fixture| -> Vec<Value> {
                let tx = f.conn.transaction().unwrap();
                let state = state::audit(&tx).unwrap();
                state.cuts.iter().map(|cut| json!({
                    "metadata": format!("{:?}", cut.metadata),
                    "roots": format!("{:?}", cut.roots),
                    "snapshot": cut.snapshot, "delta": cut.delta
                })).collect()
            };
            let original_cuts = cuts(&mut fixture);
            let original_batches = {
                let tx = fixture.conn.transaction().unwrap();
                let state = state::audit(&tx).unwrap();
                state.objects.iter().filter(|o| o.kind == content_producer::CC007_DEVELOPMENT_BATCH_KIND)
                    .map(|o| o.id.as_str().to_owned()).collect::<Vec<_>>()
            };
            assert!(!original_batches.is_empty(), "夹具必须有真实分型及后续单元");
            let row = rows[10];
            let revised = (row.0, row.1, row.2 + 1, row.3 + 1, row.4 + 1, row.5 + 1);
            let (envelope, token) = fixture.accept_begin(2, revised, 2);
            fixture.publish(&envelope, &token)?;
            let before_restore = cuts(&mut fixture);
            assert_eq!(&before_restore[..1], original_cuts.as_slice());
            let withdrawn = {
                let tx = fixture.conn.transaction().unwrap();
                let state = state::audit(&tx).unwrap();
                state.objects.iter().filter(|o| original_batches.iter().any(|id| id == o.id.as_str()) && o.life.withdrawn_generation.is_some())
                    .map(|o| o.id.as_str().to_owned()).collect::<Vec<_>>()
            };
            assert!(!withdrawn.is_empty(), "修订必须撤回旧批记录");
            let (envelope, token) = fixture.accept_begin(3, row, 3);
            fixture.publish(&envelope, &token)?;
            let after_restore = cuts(&mut fixture);
            assert_eq!(&after_restore[..before_restore.len()], before_restore.as_slice(), "全部归档 cut 的 AsKnown 内容不变");
            let tx = fixture.conn.transaction().unwrap();
            let state = state::audit(&tx).unwrap();
            for id in &withdrawn {
                let object = state.objects.iter().find(|o| o.id.as_str() == id).unwrap();
                assert!(object.life.withdrawn_generation.is_some(), "旧身份不得复活");
            }
            let active_batches: Vec<_> = state.objects.iter().filter(|o| o.kind == content_producer::CC007_DEVELOPMENT_BATCH_KIND && o.life.withdrawn_generation.is_none()).collect();
            assert!(active_batches.iter().any(|o| !original_batches.iter().any(|id| id == o.id.as_str())), "恢复必须生成新 batch 身份");
            Ok(())
        })
    }
    let failure = run(super::super::super::layout::Layout::Bounded1).unwrap_err();
    assert!(failure.contains("IdentityConflict"), "旧单元机制应复现撤回身份冲突：{failure}");
    run(super::super::super::layout::Layout::Bounded2).unwrap();
}


fn pending_oracle_identity(object: &Value, rule: &str, nodes: &mut sc::MemoryNodes) -> String {
    let payload: Value = serde_json::from_str(object["payload_json"].as_str().unwrap()).unwrap();
    let refs: Value = serde_json::from_str(object["input_refs_json"].as_str().unwrap()).unwrap();
    let identity = json!({"kind": object["kind"], "fact_key": object["fact_key"],
        "payload": payload, "input_refs": refs, "profile_id": tb02a::PROFILE, "rule_revision": rule});
    let digest = sc::write_value(nodes, &cv(&identity).unwrap()).unwrap().digest;
    format!("obj3-{}", &sc::FixedRoot::new(&sc::frame(0x20, &digest)).as_str()[4..])
}

fn pending_oracle_record(object: &Value, nodes: &mut sc::MemoryNodes) -> sc::Built {
    let fields = object.as_object().unwrap().iter().map(|(key, value)| {
        let content = if matches!(key.as_str(), "payload_json" | "input_refs_json" | "source_coords_json") {
            let inner: Value = serde_json::from_str(value.as_str().unwrap()).unwrap();
            let digest = sc::write_value(nodes, &cv(&inner).unwrap()).unwrap().digest;
            let framed = sc::frame(sc::tag::JSON_TEXT, &digest);
            nodes.put(framed, sc::tag::JSON_TEXT, &digest).unwrap();
            V::Edge(framed)
        } else { cv(value).unwrap() };
        (key.clone(), content)
    }).collect();
    sc::write_value(nodes, &V::Object(fields)).unwrap()
}

fn assert_pending_w2_oracle(
    tx: &Transaction<'_>, audited: &state::AuditedState, previous: &[Value], generation: i64,
) -> Vec<Value> {
    use newchan_rust::theta_v0::{parser, types::Bar};
    let raw: Vec<Value> = serde_json::from_value(serde_json::to_value(&audited.raw).unwrap()).unwrap();
    let mut winners = std::collections::BTreeMap::<String, Value>::new();
    for event in &raw {
        let key = event["identity_key"].as_str().unwrap().to_owned();
        let rank = |e: &Value| (e["revision"].as_i64().unwrap(), e["seq"].as_i64().unwrap());
        if winners.get(&key).is_none_or(|old| rank(event) > rank(old)) { winners.insert(key, event.clone()); }
    }
    let by_coord: std::collections::BTreeMap<i64, Value> = winners.into_values().map(|e| {
        (e["source_coord"].as_str().unwrap().parse().unwrap(), e)
    }).collect();
    let effective: Vec<Value> = by_coord.values().cloned().collect();
    let bars: Vec<Bar> = by_coord.iter().map(|(coord, e)| {
        let n = |k: &str| e[k].as_str().unwrap().parse::<i64>().unwrap();
        Bar { source_index: *coord as usize, timestamp: n("ts"), open:n("open"), high:n("high"),
            low:n("low"), close:n("close"), volume:1.0, untradable:false }
    }).collect();
    let config = ThetaConfig::default();
    let (layer, trace, fold) = parser::parse_layer_with_inclusion_facts_store(&bars, &config, tb02c::second_store(&config.parse));
    assert!(layer.merged_bars.len() < 3, "原三场景无三组形态窗");
    let stroke = trace.stroke_facts.as_ref().unwrap();
    let segments = trace.segment_facts.as_ref().unwrap();
    assert!(stroke.endpoints.is_empty() && stroke.strokes.is_empty() && segments.candidates.is_empty());
    let metadata = &audited.cuts.last().unwrap().metadata;
    let cut = metadata.structure_cut.as_str();
    let frontier = raw.len() as i64;
    assert_eq!(metadata.generation, generation);
    assert_eq!(metadata.input_frontier, frontier);
    assert_eq!(metadata.rule_revision, crate::W_STRUCTURE_RULE_REVISION);
    let rule = crate::W_STRUCTURE_RULE_REVISION;
    let mut prior = previous.to_vec();
    for object in &mut prior {
        if object["kind"] == "CC-004.undetermined_raw" {
            let payload: Value = serde_json::from_str(object["payload_json"].as_str().unwrap()).unwrap();
            object["first_known_cut"] = payload["pending_since_cut"].clone();
        }
    }
    let (mut objects, observations) = tb02a_facts::build(&trace, &by_coord, &[], &prior, generation, cut, frontier);
    let commit_ns = Some((generation * 10 + 2).to_string());
    let (bi, relations) = tb02b::build(stroke, &by_coord, &raw, &prior, generation, cut, frontier, commit_ns.clone());
    assert_eq!(bi.len(), 1);
    assert!(relations.is_empty());
    objects.extend(bi);
    let (segment, relations) = tb02c::build(segments, stroke, &by_coord, &raw, &prior, generation, cut, frontier, commit_ns.clone(), &fold).unwrap();
    assert_eq!(segment.len(), 1);
    assert!(relations.is_empty());
    objects.extend(segment);
    let latest = by_coord.values().max_by_key(|e| e["seq"].as_i64()).unwrap();
    let known = json!({"generation":generation.to_string(),"input_frontier":frontier.to_string(),
        "receipt_id":latest["receipt_id"],"received_at":latest["received_at"],"semantic_commit_ns":commit_ns});
    let fact_generation = tb02b::fact_basis(&raw).to_string();
    let source_coords: Vec<usize> = by_coord.keys().map(|c| *c as usize).collect();
    let mut builder = tb02a_facts::Builder { raw:&by_coord, generation, cut, objects:vec![] };
    for (kind, slot, schema, semantic, policy, data) in [
        ("CC-056.center_version", "TB-02-E", "s-center/1", "center-seed/1", "strict_seed_zn_outer_c_resume", json!({
            "input_frontier":frontier.to_string(), "known_at":known, "waiting_reasons":[],
            "scan_cursor":{"state":"SEEKING_SEED","center_id":null,"seed_start_unit_id":null,
                "awaiting_unit_id":null,"awaiting_after_unit_id":null,"awaiting_offset":"3"},
            "delivered_scope":"l0_strict_seed_zn_roles_c_resume_assignment", "deferred_scope":tb02e::DEFERRED_SCOPE})),
        ("CC-056.pair_version", "TB-02-F", "s-center-pair/1", "center-pair/1", "zn_outer_five_leaf_pending_ge9", json!({
            "input_frontier":frontier.to_string(), "known_at":known, "waiting_reasons":[], "pair_count":"0", "pending_pairs":"0",
            "delivered_scope":"l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position", "deferred_scope":tb02f::DEFERRED_SCOPE})),
    ] {
        builder.add(kind, json!([kind,slot]), json!({"schema_revision":schema,"semantic_version":semantic,
            "policy_version":policy,"view_role":"main","object_generation":fact_generation,"slot":slot,"data":data}),source_coords.clone());
    }
    objects.extend(builder.objects);
    let mut nodes = sc::MemoryNodes::default();
    let mut ids = std::collections::BTreeMap::new();
    for object in &mut objects {
        object["first_known_generation"] = json!(generation);
        object["first_known_cut"] = json!(cut);
        object["published_generation"] = json!(generation);
        if object["kind"] != "CC-054.knowledge_state" {
            let old = object["object_id"].as_str().unwrap().to_owned();
            let id = pending_oracle_identity(object, rule, &mut nodes);
            object["object_id"] = json!(id);
            ids.insert(old, id);
        }
    }
    let knowledge = objects.iter_mut().find(|o| o["kind"] == "CC-054.knowledge_state").unwrap();
    let mut payload: Value = serde_json::from_str(knowledge["payload_json"].as_str().unwrap()).unwrap();
    let requests = payload["requests"].as_array_mut().unwrap();
    for request in requests.iter_mut() {
        if let Some(id) = request["subject_id"].as_str() { request["subject_id"] = json!(ids[id]); }
    }
    requests.sort_by_key(|request| {
        let key: Value = serde_json::from_str(request["request_id"].as_str().unwrap()).unwrap();
        if key[0] == "CC-004.initialization" { (0, 0, 0) }
        else if key[1] == "pending" { (2, 0, 0) }
        else { (1, key[1].as_str().unwrap().parse::<i64>().unwrap(), match key[0].as_str().unwrap() {
            "CC-004.undetermined_raw"=>0, "CC-004.inclusion_step"=>1, "CC-005.inclusion_group"=>2, other=>panic!("unexpected {other}") }) }
    });
    knowledge["payload_json"] = json!(canonical_json(&payload));
    knowledge["object_id"] = json!(pending_oracle_identity(knowledge, rule, &mut nodes));
    let mut dead: Vec<&Value> = previous.iter().filter(|old| !objects.iter().any(|new| new["object_id"] == old["object_id"])).collect();
    dead.sort_by_key(|o| (o["kind"].as_str().unwrap(), o["fact_key"].as_str().unwrap(), o["object_id"].as_str().unwrap()));
    let mut withdrawals = vec![];
    let mut replaces = vec![];
    for old in dead {
        let next = objects.iter().find(|new| old["kind"] == new["kind"] && old["fact_key"] == new["fact_key"])
            .or_else(|| tb02a_facts::replacing_group(old, &objects));
        let successor = next.map(|o| o["object_id"].clone()).unwrap_or(Value::Null);
        withdrawals.push(json!({"object_id":old["object_id"],"fact_key":old["fact_key"],
            "reason":if next.is_some(){"superseded_by_revision"}else{"fact_removed"},"superseded_by":successor}));
        if next.is_some() { replaces.push(json!({"old_object_id":old["object_id"],"new_object_id":successor})); }
    }
    let axes = tb02a_facts::catalog_axes_oracle(tb02a_facts::CatalogAxisTable::for_rule_revision(rule).unwrap(),
        objects.as_slice(), raw.as_slice(), frontier-1, withdrawals.as_slice(), replaces.as_slice());
    let scope: Value = serde_json::from_str(&meta_get_opt(tx,"scope").unwrap().unwrap()).unwrap();
    let expected = json!({"generation":generation,"structure_cut":cut,"input_frontier":frontier,
        "catalog_revision":metadata.catalog_revision,"profile_id":metadata.profile_id,"profile_hash":metadata.profile_hash,
        "rule_revision":rule,"scope":scope,"raw_events":raw,"effective_events":effective,"objects":objects,
        "witnesses":[],"relations":[],"observations":observations,"catalog_axes":axes});
    let mut source = content_store::ReadStore::new(tx).unwrap();
    let actual_root = &audited.cuts.last().unwrap().roots.generated_state;
    let mut actual_bytes = Vec::new();
    sc::render_canonical(&mut source, actual_root, &mut actual_bytes).unwrap();
    let actual: Value = serde_json::from_slice(&actual_bytes).unwrap();
    for (field,want) in expected.as_object().unwrap() { assert_eq!(&actual[field], want, "generation {generation} complete W2 field {field}"); }
    assert_eq!(actual.as_object().unwrap().len(), expected.as_object().unwrap().len());
    let record_edges = objects.iter().map(|object| V::Edge(pending_oracle_record(object,&mut nodes).digest)).collect();
    let object_root = sc::write_value(&mut nodes,&V::Array(record_edges)).unwrap();
    let mut state = cv(&expected).unwrap();
    if let V::Object(fields) = &mut state { fields.iter_mut().find(|(key,_)| key=="objects").unwrap().1=V::Edge(object_root.digest); }
    let expected_root = sc::write_value(&mut nodes,&state).unwrap().content_ref();
    assert_eq!(actual_root, &expected_root, "generation {generation} independent complete W2 root");
    objects
}
