// #1419 M3：正式生产 W2 写者的 H1 冷往返；由 construction_tests 的 unix 门引入。
// 原件、全部 SQL 格值/typeof、四根与对象历史逐代对拍；不改生产规则。
type W2H1Row = (&'static str, i64, i64, i64, i64, i64, i64);

fn w2_h1_envelope(n: i64, rows: &[W2H1Row]) -> Value {
    assert_eq!(rows.len(), 1, "正式 ingest 一条消息恰一事件");
    let (id, coord, open, high, low, close, revision) = rows[0];
    h1_admission_revision_envelope(n, (id, coord, open, high, low, close), revision, "1")
}

struct W2H1Fixture {
    dir: PathBuf,
    db: PathBuf,
    profile: PathBuf,
    clock: v2::ClockPlan,
}
impl W2H1Fixture {
    fn new(messages: usize) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "newchan-w2-h1-{}-{}",
            std::process::id(),
            NEXT_LIFE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("w2-h1.sqlite");
        let clock_path = db.with_extension("clock.json");
        let mut events = serde_json::Map::new();
        for n in 1..=messages {
            events.insert(
                format!("input-{n}"),
                json!({"accept_ns":(n*10).to_string(),
                "attempts":[{"begin_ns":(n*10+1).to_string(),"commit_ns":(n*10+2).to_string()}]}),
            );
        }
        let clock = json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"w2-h1-m3",
            "origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":events});
        std::fs::write(&clock_path, canonical_json(&clock)).unwrap();
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let profile = base.join("profiles/ohlc_integer_tb02a_v1.json");
        init_v3(
            &db,
            "s-session-testonly-001",
            &base.join("catalog/signed-catalog.json"),
            &profile,
            "1",
            &clock_path,
            messages as i64,
        )
        .unwrap();
        let conn = Connection::open(&db).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "rule_revision").unwrap().as_deref(),
            Some(crate::W_STRUCTURE_RULE_REVISION)
        );
        assert_eq!(crate::W_STRUCTURE_RULE_REVISION,
            "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/2+bounded-layout/2");
        drop(conn);
        Self {
            dir,
            db,
            profile,
            clock: v2::ClockPlan::parse(clock).unwrap(),
        }
    }
    #[cfg(test)]
    fn open(&self, db: &Path) -> driver::Driver {
        h1_admission_assert_clean_close(db);
        driver::Driver::open_with_request_and_mode_retention(
            db,
            &self.profile,
            "1",
            self.clock.clone(),
            None,
            s_allocation::Mode::CapturedNative,
            536_870_912,
            true,
            super::super::admission_anchor::test_prepare(db).into(),
        )
        .unwrap()
    }
}

impl Drop for W2H1Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// SQL 所有表均逐列保留实际 typeof 与完整值。排序只消除 SQL 未声明行序；不哈希载荷。
fn w2_h1_business(conn: &Connection) -> Value {
    let tables = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
        rows.collect::<Result<Vec<_>, _>>().unwrap()
    };
    let mut out = serde_json::Map::new();
    for table in tables.into_iter().filter(|t| t != "s_content_nodes") {
        let quoted = table.replace('"', "\"\"");
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM \"{quoted}\""))
            .unwrap();
        let columns = stmt
            .column_names()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let rows = stmt
            .query_map([], |r| {
                columns
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        use rusqlite::types::Value as Sql;
                        Ok(match r.get::<_, Sql>(i)? {
                            Sql::Null => json!({"typeof":"null","value":null}),
                            Sql::Integer(v) => json!({"typeof":"integer","value":v}),
                            Sql::Real(v) => json!({"typeof":"real","bits":v.to_bits().to_string()}),
                            Sql::Text(v) => json!({"typeof":"text","value":v}),
                            Sql::Blob(v) => json!({"typeof":"blob","value":v}),
                        })
                    })
                    .collect::<rusqlite::Result<Vec<Value>>>()
            })
            .unwrap();
        let mut rows = rows.collect::<Result<Vec<_>, _>>().unwrap();
        rows.sort_by_cached_key(|r| canonical_json(&json!(r)));
        out.insert(table, json!({"columns":columns,"rows":rows}));
    }
    Value::Object(out)
}

// 与 prepared::tests::runtime_root_compare_reachable_bytes 相同的根与tag配方。
// s_content_nodes 中不可达的编码memo可随在线/冷缓存不同；已发布闭包每格仍须完全相同。
fn w2_h1_reachable_nodes(conn: &Connection) -> Value {
    let nodes = conn.prepare("SELECT digest,tag,body,typeof(digest),typeof(tag),typeof(body) FROM s_content_nodes ORDER BY digest").unwrap()
        .query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, (
            r.get::<_, i64>(1)?, r.get::<_, Vec<u8>>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?))))
        .unwrap().collect::<Result<std::collections::BTreeMap<_,_>,_>>().unwrap();
    let mut todo = Vec::new();
    for (table, columns) in [
        (
            "s_content_publications",
            vec!["manifest_root", "snapshot_root", "delta_root"],
        ),
        (
            "s_content_objects",
            vec![
                "generated_root",
                "internal_root",
                "immutable_root",
                "birth_wire_root",
            ],
        ),
        ("s_content_witnesses", vec!["generated_root", "wire_root"]),
        ("s_content_relations", vec!["generated_root"]),
        (
            "s_content_observations",
            vec!["generated_root", "birth_wire_root"],
        ),
    ] {
        for column in columns {
            todo.extend(
                conn.prepare(&format!("SELECT {column} FROM {table}"))
                    .unwrap()
                    .query_map([], |r| r.get::<_, Vec<u8>>(0))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
            );
        }
    }
    for (table, column) in [
        ("structure_deltas", "delta_json"),
        ("structure_deltas", "catalog_evidence_json"),
        ("catalog", "evidence_json"),
    ] {
        for raw in conn
            .prepare(&format!("SELECT {column} FROM {table}"))
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        {
            let value = strict_json(raw.as_bytes()).unwrap();
            if value != json!({}) {
                todo.push(
                    newchan_rust::s_content::TransportRef::from_json_view((&value).into())
                        .unwrap()
                        .digest()
                        .unwrap()
                        .to_vec(),
                );
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    while let Some(root) = todo.pop() {
        if !seen.insert(root.clone()) {
            continue;
        }
        let (tag, body, _, _, _) = nodes.get(&root).expect("已发布根的每个可达节点必须存在");
        let offsets: &[usize] = match tag {
            3 | 4 | 5 | 6 => &[8],
            7 | 18 => &[0],
            19 => &[8, 40],
            20 => &[0, 32],
            _ => &[],
        };
        for &offset in offsets {
            todo.push(body[offset..offset + 32].to_vec());
        }
    }
    json!(seen.into_iter().map(|digest| {
        let (tag,body,digest_type,tag_type,body_type) = &nodes[&digest];
        json!({"digest":{"typeof":digest_type,"value":digest},"tag":{"typeof":tag_type,"value":tag},"body":{"typeof":body_type,"value":body}})
    }).collect::<Vec<_>>())
}

fn w2_h1_snapshot(driver: &driver::Driver) -> Value {
    let owner = driver.h1_owner_for_test();
    let historical = h1_admission_canonical(owner);
    let complete = owner.complete.borrow();
    let historical_owner = &complete.as_ref().unwrap().audited;
    let tx = owner.connection.unchecked_transaction().unwrap();
    let mut source = content_store::ReadStore::new(&tx).unwrap();
    let mut objects = Vec::new();
    for entry in &historical_owner.objects {
        let mut bytes = Vec::new();
        newchan_rust::s_content::render_refview(&mut source, entry.birth_wire.view(), &mut bytes)
            .unwrap();
        let mut object: Value = serde_json::from_slice(&bytes).unwrap();
        for (wire, encoded) in [
            ("payload", "payload_json"),
            ("input_refs", "input_refs_json"),
            ("source_coords", "source_coords_json"),
            ("comparisons", "comparisons_json"),
        ] {
            if let Some(value) = object.get(wire) {
                let text = canonical_json(value);
                object[encoded] = json!(text);
            }
        }
        // 保留出生wire的原字段类型，同时给断言补当前生命周期标记。
        object["birth_withdrawn_generation"] = object["withdrawn_generation"].clone();
        object["withdrawn_generation"] = json!(entry.life.withdrawn_generation);
        objects.push(object);
    }
    drop(source);
    let business = w2_h1_business(&tx);
    let reachable_nodes = w2_h1_reachable_nodes(&tx);
    tx.rollback().unwrap();
    json!({"objects":objects,"historical":historical,"business":business,"reachable_nodes":reachable_nodes})
}

// 参照始终保持同一 Driver。每个冷点通过 SQLite 的只读源 VACUUM INTO 保全独立文件库；
// 不让 cold 继承参照的 owner、Runtime 或生成器。大快照落夹具临时目录，不常驻内存。
fn w2_h1_roundtrip(
    envelopes: &[Value],
    cold_points: &[usize],
    mut check: impl FnMut(usize, &Value),
) {
    assert!(cold_points.iter().all(|&n| n > 0 && n < envelopes.len()));
    assert_eq!(
        cold_points
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        cold_points.len()
    );
    w2_h1_roundtrip_selected(envelopes, |n, snapshot| {
        check(n, snapshot);
        cold_points.contains(&n)
    });
}

fn w2_h1_roundtrip_selected(envelopes: &[Value], mut check: impl FnMut(usize, &Value) -> bool) {
    let f = W2H1Fixture::new(envelopes.len());
    let mut online = f.open(&f.db);
    let dir = f.db.parent().unwrap();
    let mut prefixes = Vec::new();
    for (offset, envelope) in envelopes.iter().enumerate() {
        let n = offset + 1;
        let before = state::full_audit_calls_for_test();
        assert_eq!(
            online.handle(envelope.clone()).unwrap()["kind"],
            "Committed",
            "在线第 {n} 代"
        );
        assert_eq!(state::full_audit_calls_for_test(), before);
        let snapshot = w2_h1_snapshot(&online);
        let cold_point = check(n, &snapshot);
        std::fs::write(
            dir.join(format!("online-{n}.json")),
            canonical_json(&snapshot),
        )
        .unwrap();
        if cold_point {
            assert!(n < envelopes.len(), "每个冷点必须有后续代");
            let db = dir.join(format!("cold-{n}.sqlite"));
            let source =
                Connection::open_with_flags(&f.db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            source
                .execute("VACUUM main INTO ?1", [db.to_str().unwrap()])
                .unwrap();
            drop(source);
            prefixes.push((n, db));
        }
    }
    assert!(!prefixes.is_empty(), "至少一个真实冷点");
    drop(online);
    for (k, db) in prefixes {
        let before = state::full_audit_calls_for_test();
        let (cold, events, _) = state::traced(false, || f.open(&db));
        let mut cold = cold;
        assert_eq!(
            state::full_audit_calls_for_test(),
            before,
            "冷点 {k} 不得回落旧全审计"
        );
        assert_eq!(
            h1_qualified_cuts(&events),
            (1..=k as i64).collect::<Vec<_>>(),
            "冷点 {k} 必须逐cut取得H1资格"
        );
        for n in k..=envelopes.len() {
            if n > k {
                let before = state::full_audit_calls_for_test();
                assert_eq!(
                    cold.handle(envelopes[n - 1].clone()).unwrap()["kind"],
                    "Committed",
                    "冷点 {k} 续写第 {n} 代"
                );
                assert_eq!(
                    state::full_audit_calls_for_test(),
                    before,
                    "冷续写不得回落旧全审计"
                );
            }
            let actual = canonical_json(&w2_h1_snapshot(&cold));
            let expected = std::fs::read_to_string(dir.join(format!("online-{n}.json"))).unwrap();
            assert_eq!(
                actual.as_bytes(),
                expected.as_bytes(),
                "冷点 {k} 第 {n} 代完整canonical"
            );
        }
        drop(cold);
        h1_admission_assert_clean_close(&db);
    }
}

fn w2_h1_payload(object: &Value) -> Value {
    serde_json::from_str(
        object["payload_json"]
            .as_str()
            .expect("出生wire必须含完整payload_json"),
    )
    .unwrap()
}
fn w2_h1_active<'a>(snapshot: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    snapshot["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(move |o| o["kind"] == kind && o["withdrawn_generation"].is_null())
}

#[test]
fn w2_h1_cold_cd_second_cursor_continuation_matches_online() {
    // #1414 独立手算端点生成的真实 OHLC；不复制生产生成结果。
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("s_session/tests/fixtures/tb02d/raw-ledger.json");
    let ledger: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut rows: Vec<W2H1Row> = ledger["cases"]["second_open_up"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let id: &'static str = Box::leak(format!("cd-{}", r["raw_index"]).into_boxed_str());
            (
                id,
                r["raw_index"].as_i64().unwrap(),
                r["open"].as_i64().unwrap(),
                r["high"].as_i64().unwrap(),
                r["low"].as_i64().unwrap(),
                r["close"].as_i64().unwrap(),
                1,
            )
        })
        .collect();
    let base = rows.len();
    assert!(base > 33);
    let original = rows[base - 3]; // 生成器末端两根为确认尾；修订末个已知笔端（raw37），处于末段。
    rows.push((
        original.0,
        original.1,
        original.2 + 1,
        original.3 + 1,
        original.4 + 1,
        original.5 + 1,
        2,
    ));
    rows.push((
        original.0, original.1, original.2, original.3, original.4, original.5, 3,
    ));
    let last = rows[base - 1];
    rows.push((
        "cd-tail",
        last.1 + 1,
        last.2 + 100,
        last.3 + 100,
        last.4 + 100,
        last.5 + 100,
        1,
    ));
    let envelopes: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| w2_h1_envelope(i as i64 + 1, &[*r]))
        .collect();
    let mut second = false;
    let mut revised_segment = false;
    let mut last_segment_before: Option<(String, i64, i64)> = None;
    w2_h1_roundtrip(
        &envelopes,
        &[27, 33, base, base + 1, base + 2],
        |n, snapshot| {
            let version = w2_h1_active(snapshot, "CC-056.segment_version")
                .map(w2_h1_payload)
                .next()
                .unwrap();
            assert_eq!(
                version["data"]["scan_budget"]["policy"],
                "second-seq-scan-budget/2"
            );
            assert_eq!(version["data"]["scan_budget"]["round_strokes"], "0");
            // 生产签记录为0=无限；不得为造非空检查点偷偷注入测试预算。
            assert_eq!(version["data"]["scan_cursors"], json!([]));
            second |= w2_h1_active(snapshot, "CC-012.feature_sequence")
                .map(w2_h1_payload)
                .any(|p| {
                    p["data"]["entity_id"]
                        .as_str()
                        .is_some_and(|s| s.starts_with("second:"))
                });
            for object in w2_h1_active(snapshot, "CC-013.segment") {
                let payload = w2_h1_payload(object);
                let data = &payload["data"];
                let id = data["entity_id"].as_str().unwrap().to_string();
                let revision: i64 = data["entity_revision"].as_str().unwrap().parse().unwrap();
                if n == base {
                    let start = data["construction"]["start_stroke"]
                        .as_str()
                        .unwrap()
                        .parse::<i64>()
                        .unwrap();
                    if last_segment_before
                        .as_ref()
                        .is_none_or(|(_, _, old_start)| start > *old_start)
                    {
                        last_segment_before = Some((id.clone(), revision, start));
                    }
                }
                if n == base + 1
                    && last_segment_before
                        .as_ref()
                        .is_some_and(|(old_id, old_revision, _)| {
                            *old_id == id && revision > *old_revision
                        })
                {
                    revised_segment = true;
                }
            }
            if n == base {
                assert!(second, "手算场景必须实际发布SECOND序列");
            }
            if n == base + 1 {
                assert!(revised_segment, "冷点须在末段同实体revision递增之后");
            }
        },
    );
    assert!(second && revised_segment);
}

// 复用 financial_session_tests::q3_w_bars 的104根 W2真实输入配方。
// 原锁已证明第56根首中枢、第104根第二中枢。这里只复制输入，不用生产输出作期望。
fn w2_h1_pair_rows() -> Vec<W2H1Row> {
    const PIVOTS: &[i64] = &[
        1620, 1020, 1350, 1185, 1570, 1390, 1480, 1270, 1780, 1525, 2120, 1820, 1970, 1620, 1800,
        1710, 1920, 1350, 1635, 970, 1360, 1165, 1620, 1470, 1545, 1370, 1970, 1670, 2370,
    ];
    let mut bars = vec![(PIVOTS[0], 40), (PIVOTS[0], 30)];
    for w in PIVOTS.windows(2) {
        for j in 0..4 {
            bars.push((w[0] + (w[1] - w[0]) * j / 4, 20));
        }
    }
    bars.truncate(104);
    bars.into_iter()
        .enumerate()
        .map(|(i, (mid, half))| {
            let id: &'static str = Box::leak(format!("w-{i}").into_boxed_str());
            (id, i as i64, mid, mid + half, mid - half, mid, 1)
        })
        .collect()
}

#[test]
fn w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online() {
    let mut rows = w2_h1_pair_rows();
    let top = rows.iter().position(|r| r.2 == 2120).unwrap();
    let original = rows[top];
    // 小幅极值修订保持中枢与pair的entity_id，迫使已建立pair携新的价格输入修订。
    rows.push((
        original.0,
        original.1,
        original.2 + 1,
        original.3 + 1,
        original.4 + 1,
        original.5 + 1,
        2,
    ));
    rows.push((
        original.0, original.1, original.2, original.3, original.4, original.5, 3,
    ));
    let last = rows[103];
    rows.push((
        "w-tail",
        104,
        last.2 + 100,
        last.3 + 100,
        last.4 + 100,
        last.5 + 100,
        1,
    ));
    let envelopes: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| w2_h1_envelope(i as i64 + 1, &[*r]))
        .collect();
    let mut center_previous = std::collections::BTreeMap::<String, Value>::new();
    let mut center_progress = false;
    let mut pair_before = std::collections::BTreeMap::<String, Value>::new();
    let mut pair_revised = false;
    w2_h1_roundtrip_selected(&envelopes, |n, snapshot| {
        let progressed_before = center_progress;
        let centers: Vec<_> = w2_h1_active(snapshot, "CC-016.center")
            .map(w2_h1_payload)
            .collect();
        for payload in &centers {
            let data = &payload["data"];
            let id = data["entity_id"].as_str().unwrap().to_string();
            if let Some(old) = center_previous.get(&id) {
                center_progress |= old["members"] != data["members"]
                    || old["resume_anchor"] != data["resume_anchor"];
            }
            center_previous.insert(id, data.clone());
        }
        if n == 56 {
            assert!(!centers.is_empty(), "第56根必须实际成立center seed");
            assert!(
                w2_h1_active(snapshot, "CC-016.center_seed_attempt")
                    .map(w2_h1_payload)
                    .any(|p| !p["data"]["center_id"].is_null()),
                "必须有成立seed指向center"
            );
        }
        if n == 104 {
            assert!(center_progress, "必须发生center成员延伸或resume_anchor移动");
            assert!(centers.len() >= 2, "第二中心必须实际成立");
            for kind in [
                "CC-017.center_roles",
                "CC-022.unit_assignment",
                "CC-018.unit_core_position",
                "CC-056.center_version",
                "CC-056.pair_version",
            ] {
                assert!(
                    w2_h1_active(snapshot, kind).next().is_some(),
                    "E/F伴随对象不得空真：{kind}"
                );
            }
            for p in w2_h1_active(snapshot, "CC-020.center_pair").map(w2_h1_payload) {
                let data = &p["data"];
                pair_before.insert(
                    data["entity_id"].as_str().unwrap().to_string(),
                    data.clone(),
                );
            }
            assert!(!pair_before.is_empty(), "第104根必须建立pair");
        }
        if n == 105 {
            for p in w2_h1_active(snapshot, "CC-020.center_pair").map(w2_h1_payload) {
                let d = &p["data"];
                if let Some(old) = pair_before.get(d["entity_id"].as_str().unwrap()) {
                    let revision = |v: &Value| {
                        v["entity_revision"]
                            .as_str()
                            .unwrap()
                            .parse::<i64>()
                            .unwrap()
                    };
                    pair_revised |= revision(d) > revision(old)
                        && !d["previous"].is_null()
                        && old["inputs"] != d["inputs"];
                }
            }
            assert!(
                pair_revised,
                "同entity pair必须发生有previous与输入变化依据的修订"
            );
        }
        // 实际成员延伸/resume_anchor变化的那代取冷点，不靠预猜代号。
        n == 56 || (!progressed_before && center_progress) || matches!(n, 104 | 105 | 106)
    });
    assert!(center_progress && pair_revised);
}
