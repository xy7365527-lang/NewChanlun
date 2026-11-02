// #1419 M3：生产 W2 的 W 未定 prior 与 bounded-layout/2 窗口冷往返。
// 与 H1 admission 同处 test+s_incremental+unix include；共享正式 Driver 的完整对拍。

fn w2_h1_window_field(object: &Value, field: &str) -> Value {
    serde_json::from_str(object[field].as_str().expect("原件必须保留 JSON 文本字段")).unwrap()
}

fn w2_h1_window_active(object: &Value) -> bool {
    object["withdrawn_generation"].is_null()
}

fn w2_h1_window_objects<'a>(snapshot: &'a Value, kind: &str, active: bool) -> Vec<&'a Value> {
    snapshot["objects"]
        .as_array()
        .expect("完整快照必须携全部历史对象原件")
        .iter()
        .filter(|o| o["kind"] == kind && (!active || w2_h1_window_active(o)))
        .collect()
}

fn w2_h1_window_pending(snapshot: &Value) -> std::collections::BTreeMap<String, Value> {
    w2_h1_window_objects(snapshot, content_producer::CC004_UNDETERMINED_KIND, true)
        .into_iter()
        .map(|object| {
            let slot: Value = serde_json::from_str(object["fact_key"].as_str().unwrap()).unwrap();
            (slot[1].as_str().unwrap().to_owned(), object.clone())
        })
        .collect()
}

fn w2_h1_window_requests(snapshot: &Value) -> Vec<Value> {
    let knowledge = w2_h1_window_objects(snapshot, content_producer::CC054_KNOWLEDGE_KIND, true);
    assert_eq!(knowledge.len(), 1, "夹具须有唯一 TB-02-A 知识状态");
    w2_h1_window_field(knowledge[0], "payload_json")["requests"]
        .as_array()
        .unwrap()
        .clone()
}

fn w2_h1_window_waiting(snapshot: &Value, expected: &[(i64, usize)]) {
    let init = w2_h1_window_objects(snapshot, content_producer::CC004_INITIALIZATION_KIND, true);
    assert_eq!(init.len(), 1);
    let payload = w2_h1_window_field(init[0], "payload_json");
    assert_eq!(payload["direction_state"], "Undetermined");
    assert!(payload["first_raw_pair"].is_null());
    assert!(w2_h1_window_objects(snapshot, content_producer::CC005_GROUP_KIND, true).is_empty());
    let pending = w2_h1_window_pending(snapshot);
    assert_eq!(pending.len(), expected.len(), "未定原件必须真实非空");
    for (coord, since) in expected {
        let object = pending.get(&coord.to_string()).expect("应有该坐标未定原件");
        let payload = w2_h1_window_field(object, "payload_json");
        assert_eq!(payload["pending_since_cut"], format!("cut-{since}"));
        let refs = w2_h1_window_field(object, "input_refs_json");
        assert_eq!(
            refs.as_array().unwrap().len(),
            1,
            "未定原件绑定同坐标源版本"
        );
    }
    let group_pending = canonical_json(&json!([content_producer::CC005_GROUP_KIND, "pending"]));
    let requests = w2_h1_window_requests(snapshot);
    assert!(
        requests.iter().any(|request| {
            request["request_id"] == group_pending && request["subject_id"].is_null()
        }),
        "W 等待期必须真实发出待定组请求"
    );
    for object in pending.values() {
        assert!(requests
            .iter()
            .any(|r| r["subject_id"] == object["object_id"]));
    }
}

fn w2_h1_window_established(snapshot: &Value) {
    let init = w2_h1_window_objects(snapshot, content_producer::CC004_INITIALIZATION_KIND, true);
    assert_eq!(init.len(), 1);
    let payload = w2_h1_window_field(init[0], "payload_json");
    assert_eq!(payload["direction_state"], "Established");
    assert!(!payload["first_raw_pair"].is_null());
    assert!(w2_h1_window_pending(snapshot).is_empty());
    assert!(!w2_h1_window_objects(snapshot, content_producer::CC005_GROUP_KIND, true).is_empty());
    let group_pending = canonical_json(&json!([content_producer::CC005_GROUP_KIND, "pending"]));
    assert!(w2_h1_window_requests(snapshot)
        .iter()
        .all(|request| request["request_id"] != group_pending));
    assert!(
        w2_h1_window_objects(snapshot, content_producer::CC004_UNDETERMINED_KIND, false)
            .iter()
            .all(|o| !w2_h1_window_active(o)),
        "方向成立须撤回全部等待原件，历史仍保留"
    );
}

#[test]
fn w2_h1_cold_w_undetermined_prior_revision_matches_online() {
    // pending_source_ordered_same_ohlc 的真实 W 配方；等待期同 OHLC 新版本只换源身份。
    let rows: Vec<W2H1Row> = vec![
        ("a", 0, 6, 12, 3, 7, 1),
        ("b", 10, 6, 12, 3, 7, 1),
        ("c", 20, 6, 12, 3, 7, 1),
        ("a", 0, 6, 12, 3, 7, 2),
        ("d", 30, 8, 17, 6, 12, 1),
    ];
    let envelopes: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(n, row)| w2_h1_envelope(n as i64 + 1, &[*row]))
        .collect();
    let mut original_a = None;
    w2_h1_roundtrip(
        &envelopes,
        &[1, 2, 3, 4],
        |generation, snapshot| match generation {
            1 => w2_h1_window_waiting(snapshot, &[(0, 1)]),
            2 => w2_h1_window_waiting(snapshot, &[(0, 1), (10, 2)]),
            3 => {
                w2_h1_window_waiting(snapshot, &[(0, 1), (10, 2), (20, 3)]);
                original_a = Some(w2_h1_window_pending(snapshot)["0"]["object_id"].clone());
            }
            4 => {
                w2_h1_window_waiting(snapshot, &[(0, 4), (10, 2), (20, 3)]);
                let pending = w2_h1_window_pending(snapshot);
                assert_eq!(
                    w2_h1_window_field(&pending["0"], "input_refs_json")[0]["revision"],
                    "2"
                );
                assert_ne!(
                    pending["0"]["object_id"],
                    *original_a.as_ref().unwrap(),
                    "同价源修订必须使旧 W 身份失效且重置该原件 pending_since_cut"
                );
                let historical = w2_h1_window_objects(
                    snapshot,
                    content_producer::CC004_UNDETERMINED_KIND,
                    false,
                );
                assert!(historical
                    .iter()
                    .any(|o| o["object_id"] == *original_a.as_ref().unwrap()
                        && !w2_h1_window_active(o)));
            }
            5 => w2_h1_window_established(snapshot),
            _ => unreachable!(),
        },
    );

    // pending_source_earlier_revision：已成立 → 历史源修订重新未定 → 再成立。
    let rows: Vec<W2H1Row> = vec![
        ("a", 0, 6, 12, 3, 7, 1),
        ("b", 10, 8, 16, 6, 12, 1),
        ("a", 0, 8, 13, 4, 7, 2),
        ("a", 0, 8, 17, 6, 12, 3),
        ("d", 20, 12, 20, 8, 14, 1),
    ];
    let envelopes: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(n, row)| w2_h1_envelope(n as i64 + 1, &[*row]))
        .collect();
    w2_h1_roundtrip(
        &envelopes,
        &[1, 3, 4],
        |generation, snapshot| match generation {
            1 => w2_h1_window_waiting(snapshot, &[(0, 1)]),
            2 | 3 => w2_h1_window_established(snapshot),
            4 => w2_h1_window_waiting(snapshot, &[(0, 4), (10, 4)]),
            5 => w2_h1_window_established(snapshot),
            _ => unreachable!(),
        },
    );
}

fn w2_h1_window_fractals(snapshot: &Value) -> std::collections::BTreeMap<String, Vec<i64>> {
    w2_h1_window_objects(snapshot, content_producer::CC007_FRACTAL_KIND, true)
        .into_iter()
        .map(|object| {
            let payload = w2_h1_window_field(object, "payload_json");
            let window = payload["window"]
                .as_array()
                .unwrap()
                .iter()
                .map(|coord| coord.as_str().unwrap().parse::<i64>().unwrap())
                .collect();
            (object["object_id"].as_str().unwrap().to_owned(), window)
        })
        .collect()
}

fn w2_h1_window_batches(snapshot: &Value) -> std::collections::BTreeMap<String, Value> {
    w2_h1_window_objects(
        snapshot,
        content_producer::CC007_DEVELOPMENT_BATCH_KIND,
        true,
    )
    .into_iter()
    .map(|object| {
        (
            object["object_id"].as_str().unwrap().to_owned(),
            object.clone(),
        )
    })
    .collect()
}

#[test]
fn w2_h1_cold_bounded2_shape_development_window_matches_online() {
    // #1392 独立 OHLC oracle + pending_source 的 bounded2_revision_restore 配方。
    let oracle = crate::tests::tb02b_oracle_rows();
    assert_eq!(oracle.len(), 14);
    let mut rows: Vec<W2H1Row> = oracle
        .iter()
        .map(|&(id, c, o, h, l, p)| (id, c, o, h, l, p, 1))
        .collect();
    let (id, c, o, h, l, p) = oracle[10];
    rows.push((id, c, o + 1, h + 1, l + 1, p + 1, 2));
    rows.push((id, c, o, h, l, p, 3));
    // 最早三组的中组改为包住左右原件，强制窗口移走；恢复仍是新源版本。
    rows.push(("b1", 10, 10, 20, 1, 10, 2));
    let (id, c, o, h, l, p) = oracle[1];
    rows.push((id, c, o, h, l, p, 3));
    let envelopes: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(n, row)| w2_h1_envelope(n as i64 + 1, &[*row]))
        .collect();
    let mut birth_fractals = Vec::new();
    let mut before_move_windows = Vec::new();
    let mut original_batches = std::collections::BTreeMap::new();
    let mut previous_count = 0;
    let mut saw_development_growth = false;
    w2_h1_roundtrip(
        &envelopes,
        &[3, 4, 5, 14, 15, 17],
        |generation, snapshot| {
            let fractals = w2_h1_window_fractals(snapshot);
            let shapes = w2_h1_window_objects(snapshot, content_producer::CC006_SHAPE_KIND, true);
            for shape in &shapes {
                let refs = w2_h1_window_field(shape, "input_refs_json");
                let groups = refs.as_array().unwrap();
                assert_eq!(groups.len(), 3, "形态必须绑定完整左中右三组来源");
                for group in groups {
                    assert!(group["raw_refs"].as_array().is_some_and(|r| !r.is_empty()));
                    assert!(group["dependency_refs"].is_array());
                }
            }
            let batches = w2_h1_window_batches(snapshot);
            for object in batches.values() {
                let payload = w2_h1_window_field(object, "payload_json");
                let entries = payload["entries"].as_array().unwrap();
                assert!(!entries.is_empty());
                let windows: Vec<_> = entries
                    .iter()
                    .map(|entry| {
                        let id = entry["fractal_object_id"].as_str().unwrap();
                        assert!(
                            entry["unit"]["source_ref"].is_object(),
                            "bounded2 单元须有真实源版本绑定"
                        );
                        assert!(
                            entry["development"]["root"].is_array(),
                            "发展根承载完整已发布单元数组"
                        );
                        assert!(
                            entry["development"]["count"]
                                .as_str()
                                .unwrap()
                                .parse::<usize>()
                                .unwrap()
                                > 0
                        );
                        fractals.get(id).expect("开发批只引用当前真实分型").clone()
                    })
                    .collect();
                assert!(
                    windows.windows(2).all(|w| w[0] < w[1]),
                    "开发批 entries 必须严格按分型窗口序"
                );
            }
            if generation == 3 {
                assert!(
                    !fractals.is_empty() && !shapes.is_empty(),
                    "出生冷点必须真实产生分型及形态"
                );
                assert!(fractals.values().any(|window| window == &[0, 10, 20]));
                birth_fractals = fractals.keys().cloned().collect();
            }
            if generation == 4 {
                assert!(!birth_fractals.is_empty());
                assert!(
                    birth_fractals.iter().any(|id| !fractals.contains_key(id)),
                    "右组包含续写必须修订出生分型身份"
                );
                let history =
                    w2_h1_window_objects(snapshot, content_producer::CC007_FRACTAL_KIND, false);
                assert!(history
                    .iter()
                    .any(|o| birth_fractals.iter().any(|id| o["object_id"] == *id)
                        && !w2_h1_window_active(o)));
            }
            if (5..=14).contains(&generation) {
                assert!(!batches.is_empty(), "后续发展冷点必须真实发出开发批");
                saw_development_growth |= batches.len() > previous_count;
                previous_count = batches.len();
            }
            if generation == 14 {
                assert!(saw_development_growth, "至少经历一次后续历史续接");
                original_batches = batches.clone();
            }
            if generation == 15 {
                assert!(!original_batches.is_empty());
                let historical = w2_h1_window_objects(
                    snapshot,
                    content_producer::CC007_DEVELOPMENT_BATCH_KIND,
                    false,
                );
                assert!(
                    historical.iter().any(|o| original_batches
                        .contains_key(o["object_id"].as_str().unwrap())
                        && !w2_h1_window_active(o)),
                    "源修订必须撤回真实旧开发批身份"
                );
                assert!(batches.keys().any(|id| !original_batches.contains_key(id)));
            }
            if generation == 16 {
                before_move_windows = fractals.values().cloned().collect::<Vec<_>>();
                before_move_windows.sort();
                assert!(!before_move_windows.is_empty());
            }
            if generation == 17 {
                assert!(
                    before_move_windows
                        .iter()
                        .any(|old| !fractals.values().any(|new| new == old)),
                    "包含修订必须使修订前活窗口失效或移窗"
                );
                let historical =
                    w2_h1_window_objects(snapshot, content_producer::CC007_FRACTAL_KIND, false);
                assert!(
                    historical.iter().any(|object| {
                        let payload = w2_h1_window_field(object, "payload_json");
                        let window: Vec<i64> = payload["window"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_str().unwrap().parse().unwrap())
                            .collect();
                        before_move_windows.contains(&window)
                            && !fractals.values().any(|v| v == &window)
                            && !w2_h1_window_active(object)
                    }),
                    "失效分型须留历史撤回原件"
                );
            }
            if generation == 18 {
                let mut restored_windows: Vec<_> = fractals.values().cloned().collect();
                restored_windows.sort();
                assert_eq!(
                    restored_windows, before_move_windows,
                    "复原后须以新源版本恢复修订前窗口"
                );
            }
        },
    );
}
