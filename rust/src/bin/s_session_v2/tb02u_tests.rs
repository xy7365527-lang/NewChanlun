// #1393：复用 S owner 的 toy 合同测试。#1419 裁定②后写者是测试种子（v2 生产写入口已拒），
// W 对象的生产承载待 S3（#1565）。
fn tb02u_kind(snapshot: &Value, kind: &str) -> Vec<Value> {
    snapshot["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["kind"] == kind)
        .cloned()
        .collect()
}

#[test]
fn tb02u_old_cut_explicit_replaces_revision_and_recovery() {
    let files = TestFiles::new();
    init_test_db(&files);
    let cuts = [
        (vec![(0, 10), (2, 8)], "1"),
        (vec![(0, 10), (2, 8), (3, 11)], "1"),
        (vec![(0, 10), (2, 8), (1, 9)], "2"),
        (vec![(0, 10), (2, 8), (3, 11)], "3"),
    ];
    let mut histories = Vec::new();
    let mut old_pending: Vec<Value> = vec![];
    let mut epoch = DEFAULT_WRITER_EPOCH.to_owned();
    for (index, (raw, revision)) in cuts.iter().enumerate() {
        let mut input = ohlc_input(raw, false);
        if raw.len() == 3 {
            input["events"][2]["revision"] = json!(revision);
        }
        accept_core(
            &files.db(),
            &canonical_json(&input),
            &fixture("profiles/ohlc_integer_tb02a_v1.json"),
            &epoch,
            None,
        )
        .unwrap();
        advance_core_fixture(&files.db(), &epoch, None).unwrap();
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let now = snapshot_of(&files, None);
        let pending = tb02u_kind(&now, "CC-004.undetermined_raw");
        let groups = tb02u_kind(&now, "CC-005.inclusion_group");
        let initialization = tb02u_kind(&now, "CC-004.initialization");
        assert_eq!(initialization.len(), 1);
        assert_eq!(
            initialization[0]["payload"]["initialization_policy_revision"],
            "raw-initialization-w/1"
        );
        // #1419 裁定②：v2 写者已退役，本测试的写者是测试种子（库规则串仍是旧 RULE_REVISION，
        // 不带 W 后缀）。W 判据（未定 raw 首次发生、显式 replaces、修订、历史 cut 不变）在此由共享
        // 事实构造层承载；生产承载（S3 发 W 对象）阻塞于 #1565，届时迁到 S3。
        assert!(!RULE_REVISION.contains("raw-initialization-w"));
        assert_eq!(
            meta_get_opt(&conn, "rule_revision").unwrap().as_deref(),
            Some(RULE_REVISION)
        );
        if index % 2 == 0 {
            assert_eq!(pending.len(), raw.len());
            assert!(groups.is_empty());
            assert!(tb02u_kind(&now, "CC-006.local_shape").is_empty());
            for object in &pending {
                assert_eq!(object["payload"]["direction_state"], "Undetermined");
                assert_eq!(object["input_refs"].as_array().unwrap().len(), 1);
                assert_eq!(object["first_known_generation"], (index + 1).to_string());
            }
            if index == 2 {
                let first_pending = tb02u_kind(&histories[0], "CC-004.undetermined_raw");
                assert!(pending.iter().all(|o| !first_pending
                    .iter()
                    .any(|p| p["object_id"] == o["object_id"])));
                // raw0/raw1 的原始身份未变；改变的是未定结构记录的发生身份。
                for old in &first_pending {
                    let current = pending
                        .iter()
                        .find(|p| {
                            p["payload"]["raw"]["source_coord"]
                                == old["payload"]["raw"]["source_coord"]
                        })
                        .unwrap();
                    assert_eq!(current["input_refs"], old["input_refs"]);
                }
            }
            old_pending = pending;
        } else {
            assert!(pending.is_empty());
            assert_eq!(groups.len(), 2);
            let delta: String = conn
                .query_row(
                    "SELECT delta_json FROM structure_deltas WHERE generation=?1",
                    params![(index + 1) as i64],
                    |r| r.get(0),
                )
                .unwrap();
            let delta: Value = serde_json::from_str(&delta).unwrap();
            for old in &old_pending {
                let raw_coord = &old["payload"]["raw"]["source_coord"];
                let group = groups
                    .iter()
                    .find(|g| {
                        g["payload"]["members"]
                            .as_array()
                            .unwrap()
                            .contains(raw_coord)
                    })
                    .unwrap();
                assert!(delta["replaces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["old_object_id"] == old["object_id"]
                        && r["new_object_id"] == group["object_id"]));
                assert!(now["relations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["relation_type"] == "replaces"
                        && r["subject"] == group["object_id"]
                        && r["object"] == old["object_id"]));
            }
        }
        for (i, old) in histories.iter().enumerate() {
            assert_eq!(&snapshot_of(&files, Some((i + 1) as i64)), old);
        }
        histories.push(snapshot_of(&files, Some((index + 1) as i64)));
        drop(conn);
        // 重开持久边界并轮换 writer epoch；无新 raw 的 Advance 不造对象。
        // （recover 段同为测试种子写者；S3 承接阻塞于 #1565。）
        epoch = (index + 2).to_string();
        cmd_recover(&files.db(), &epoch).unwrap();
        advance_core_fixture(&files.db(), &epoch, None).unwrap();
        assert_eq!(snapshot_of(&files, None), now);
    }
    assert_eq!(histories[0]["raw_history"].as_array().unwrap().len(), 2);
    assert_eq!(histories[3]["raw_history"].as_array().unwrap().len(), 5);
}

#[test]
fn tb02u_pending_append_keeps_first_known_and_policy_gate() {
    let files = TestFiles::new();
    init_test_db(&files);
    let mut first = Value::Null;
    for n in 1..=3 {
        accept_ohlc(&files, &ohlc_input(&[(0, 10), (2, 8), (1, 11)][..n], false));
        advance_core_fixture(&files.db(), DEFAULT_WRITER_EPOCH, None).unwrap();
        let snapshot = snapshot_of(&files, None);
        let pending = tb02u_kind(&snapshot, "CC-004.undetermined_raw");
        assert_eq!(pending.len(), n);
        let raw0 = pending
            .iter()
            .find(|o| o["payload"]["raw"]["source_coord"] == "0")
            .unwrap();
        if n == 1 {
            first = raw0.clone();
        } else {
            assert_eq!(raw0, &first);
        }
        assert!(tb02u_kind(&snapshot, "CC-005.inclusion_group").is_empty());
        verify_reachable_root(&open_db(&files.db()).unwrap()).unwrap();
    }
    let conn = open_db(&files.db()).unwrap();
    meta_set(
        &conn,
        "rule_revision",
        "s2-axis-quantifiers+new-bi/1+segment-second/1",
    )
    .unwrap();
    let error = advance_core_fixture(&files.db(), DEFAULT_WRITER_EPOCH, None).unwrap_err();
    assert!(error.contains("VersionMismatch"));
}
