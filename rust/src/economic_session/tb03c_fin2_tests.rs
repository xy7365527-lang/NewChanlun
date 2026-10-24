//! #1672：FIN/2 在四个独立 HFS+ 载体上的 WE7 正例、恢复与载体负控。
use super::tb03c_allocation_tests::{apply_request, manifest, resolve_request, Session};
use super::tb03c_book_tests::Space;
use super::{book, store};
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const RAW: &str = include_str!("../../../s_session/tests/fixtures/tb03c_allocation/RAW.json");
const HAND: &str = include_str!("../../../s_session/tests/fixtures/tb03c_allocation/HAND-ORACLE.json");

fn carrier_root() -> PathBuf {
    PathBuf::from(
        std::env::var("TB03C_FIN2_CARRIER_ROOT")
            .expect("TB03C_FIN2_CARRIER_ROOT must point to four mounted HFS+ roots"),
    )
}

fn reader_resources() -> Value {
    json!({
        "schema_revision":"s-financial-reader-resources/1",
        "max_request_bytes":"1048576",
        "max_reply_bytes":"1048576",
        "max_stderr_bytes":"1048576",
        "request_timeout_ms":"30000",
        "job_limits":{
            "max_total_wire_bytes":"16777216",
            "max_total_node_bytes":"16777216",
            "max_total_nodes":"10000",
            "max_total_records":"10000",
            "max_total_requests":"10000",
            "max_elapsed_ms":"30000",
            "max_staged_bytes":"33554432",
            "max_store_bytes":"33554432"
        }
    })
}

fn write_fin_input(root: &Path) -> (PathBuf, String) {
    let doc: Value = serde_json::from_str(RAW).unwrap();
    let covered = (3..6)
        .map(|index| {
            let event = &doc["events"][index];
            let fact_key = book::fact_key(event).unwrap();
            let fact_hash = book::hash(&json!({
                "raw_sha256":sha256_hex(RAW.as_bytes()),
                "json_pointer":format!("/events/{index}"),
                "event":event,
                "units":doc["units"]
            }));
            json!({
                "fact_key":fact_key,
                "fact_hash":fact_hash,
                "token":format!("token-{}", event["source"]["external_fact_id"].as_str().unwrap())
            })
        })
        .collect::<Vec<_>>();
    let input = json!({
        "schema_revision":"tb03b-finance/2",
        "kind":"TestOnly",
        "source_id":"fin2-we7-test",
        "source_epoch":"1",
        "policy":{
            "policy_id":"fin2-we7-policy",
            "allocation_policy_id":"#1349/atomic-slot-wait/v1",
            "cap":"1000",
            "currency":"TEST-CASH",
            "money_unit":"TEST-CASH",
            "quantity_unit":"TEST-UNIT",
            "quantum":"1",
            "capability_id":"TestOnly-fin2-we7",
            "model":"TestOnlyUnleveragedCashBuy",
            "valid_through_ns":"1000000000"
        },
        "baseline":{
            "responsibility_id":"fin2-we7-baseline",
            "amount":"1000",
            "source_ref":"TestOnly FIN/2 responsibility baseline",
            "covered_facts":covered
        },
        "rights":[],
        "plans":[],
        "slots":[]
    });
    let raw = canonical_json(&input);
    let path = root.join("fin-input.json");
    std::fs::write(&path, raw.as_bytes()).unwrap();
    (path, sha256_hex(raw.as_bytes()))
}

fn fin2_manifest(root: &Path) -> Value {
    let space = Space(root.to_path_buf());
    let mut m = manifest(&space, true);
    let (input_path, input_sha256) = write_fin_input(root);
    m["protocol_revision"] = json!("economic-session/2");
    m["resources"]["response_timeout_ms"] = json!("30000");
    m["finance"] = json!({
        "schema_revision":"tb03b-finance/2",
        "input_path":input_path,
        "input_sha256":input_sha256,
        "coordinator_source":{
            "source_namespace":"testonly-tb1406",
            "source_epoch":"fixture-epoch-1",
            "producer_id":"tb1406-c4a",
            "producer_epoch":"1"
        },
        "structure":{
            "binary":root.join("unit-S-binary"),
            "binary_sha256":book::hash(&json!("unit")),
            "db":root.join("s.db"),
            "session_id":"tb1406-c4a",
            "session_generation":"1",
            "writer_epoch":"1",
            "reader_revision":"s-financial-basis/3",
            "content_format":"s-content/1",
            "reader_resources":reader_resources(),
            "reader_session":null
        }
    });
    m["authorized_sources"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!("ImportResponsibility"));
    m
}

fn load(root: &Path) -> Session {
    let space = Space(root.to_path_buf());
    let manifest_value = fin2_manifest(root);
    let path = root.join("manifest.json");
    std::fs::write(&path, canonical_json(&manifest_value)).unwrap();
    let mut configs = vec![store::Config::load(&path, &root.join("e.db"), None).unwrap()];
    for index in 0..3 {
        configs.push(
            store::Config::load(
                &path,
                &root.join(format!("b{index}.db")),
                Some(&format!("chong-N{}", index + 1)),
            )
            .unwrap(),
        );
    }
    let mut epochs = Vec::new();
    let dbs = configs
        .iter()
        .map(|config| {
            let receipt = store::init(config).unwrap();
            epochs.push(book::integer(&receipt, "writer_epoch").unwrap());
            store::open(&config.db, false).unwrap().0
        })
        .collect();
    Session {
        _space: space,
        configs,
        dbs,
        next: 0,
        epochs,
    }
}

fn staging_count(session: &Session, owner: usize) -> i64 {
    session.dbs[owner]
        .query_row("SELECT count(*) FROM content_staging", [], |row| row.get(0))
        .unwrap()
}

fn latest_image(session: &Session, owner: usize) -> Value {
    store::latest(&session.dbs[owner]).unwrap().3
}

fn latest_seq(session: &Session, owner: usize) -> i64 {
    store::latest(&session.dbs[owner]).unwrap().0
}

#[test]
#[ignore = "需要 TB03C_FIN2_CARRIER_ROOT 指向四个独立 HFS+ 挂载根"]
fn fin2_we7_capture_resolve_apply_recover_and_resend() {
    let mut session = load(&carrier_root());
    for (owner, config) in session.configs.iter().enumerate() {
        let revision: String = session.dbs[owner]
            .query_row("SELECT v FROM meta WHERE k='schema_revision'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(revision, "economic-session/2");
        let format: String = session.dbs[owner]
            .query_row("SELECT content_format FROM content_meta WHERE singleton=1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(format, "s-content/1");
        assert_eq!(config.domain, ["E", "B:chong-N1", "B:chong-N2", "B:chong-N3"][owner]);
    }

    let imported = session.send(
        0,
        json!({"op":"ImportResponsibility","responsibility_id":"fin2-we7-baseline","writer_epoch":"1"}),
    );
    assert_eq!(imported["kind"], "ResponsibilityImportedReceipt", "{imported}");

    let hand: Value = serde_json::from_str(HAND).unwrap();
    let mut captures = Vec::new();
    let mut resolves = Vec::new();
    let mut applies = Vec::new();
    for index in 0..3 {
        let capture = session.capture(RAW, index);
        assert_eq!(capture["kind"], "Recorded", "{capture}");
        let applied = session.send(index + 1, apply_request(&capture, &capture["allocation_id"]));
        assert_eq!(applied["kind"], "Applied", "{applied}");
        store::audit(&session.dbs[index + 1], &session.configs[index + 1]).unwrap();
        assert_eq!(staging_count(&session, index + 1), 0);
        captures.push(capture);
        applies.push(applied);
    }

    for round in 0..3 {
        let capture = session.capture(RAW, round + 3);
        assert_eq!(capture["kind"], "Recorded", "{capture}");
        let resolved = session.resolve(&capture, round + 4);
        assert_eq!(resolved["kind"], "Resolved", "{resolved}");
        let allocations = session.allocations(&capture);
        for index in 0..3 {
            let applied = session.send(
                index + 1,
                apply_request(&capture, &allocations[index]["allocation_id"]),
            );
            assert_eq!(applied["kind"], "Applied", "{applied}");
            let book = &session.image(index + 1)["book"];
            assert_eq!(
                book["actual_quantity"],
                hand["reports"][round]["book_quantity"][index]
                    .as_i64()
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                book["cost_basis"]["K"],
                hand["reports"][round]["K"][index].as_i64().unwrap().to_string()
            );
            assert_eq!(
                allocations[index]["commission"],
                hand["reports"][round]["fees"][index]
                    .as_i64()
                    .unwrap()
                    .to_string()
            );
            store::audit(&session.dbs[index + 1], &session.configs[index + 1]).unwrap();
            assert_eq!(staging_count(&session, index + 1), 0);
            applies.push(applied);
        }
        store::audit(&session.dbs[0], &session.configs[0]).unwrap();
        assert_eq!(staging_count(&session, 0), 0);
        captures.push(capture);
        resolves.push(resolved);
    }

    assert_eq!(captures.len(), 6);
    assert_eq!(resolves.len(), 3);
    assert_eq!(applies.len(), 12);
    let mut allocation_ids = std::collections::BTreeSet::new();
    for receipt in &applies {
        assert_eq!(receipt["kind"], "Applied");
        assert!(allocation_ids.insert(receipt["allocation_id"].as_str().unwrap().to_owned()));
    }
    assert_eq!(allocation_ids.len(), 12);

    let before_images = (0..4).map(|owner| latest_image(&session, owner)).collect::<Vec<_>>();
    let before_seq = (0..4).map(|owner| latest_seq(&session, owner)).collect::<Vec<_>>();
    for owner in 0..4 {
        let recovered = store::recover(&session.configs[owner]).unwrap();
        assert_eq!(recovered["kind"], "Recovered");
        let next_epoch = book::integer(&recovered, "writer_epoch").unwrap();
        assert!(next_epoch > session.epochs[owner]);
        session.epochs[owner] = next_epoch;
        store::audit(&session.dbs[owner], &session.configs[owner]).unwrap();
        assert_eq!(canonical_json(&latest_image(&session, owner)), canonical_json(&before_images[owner]));
        assert_eq!(staging_count(&session, owner), 0);
    }

    // 新 writer epoch 下重送全部 6 Capture + 3 Resolve + 12 Apply；返回原收据且不新增 commit。
    for index in 0..3 {
        assert_eq!(session.capture(RAW, index), captures[index]);
        assert_eq!(session.send(index + 1, apply_request(&captures[index], &captures[index]["allocation_id"])), applies[index]);
    }
    for round in 0..3 {
        assert_eq!(session.capture(RAW, round + 3), captures[round + 3]);
        assert_eq!(session.resolve(&captures[round + 3], round + 4), resolves[round]);
        let allocations = session.allocations(&captures[round + 3]);
        for index in 0..3 {
            assert_eq!(
                session.send(
                    index + 1,
                    apply_request(&captures[round + 3], &allocations[index]["allocation_id"]),
                ),
                applies[3 + round * 3 + index]
            );
        }
    }
    for owner in 0..4 {
        assert_eq!(latest_seq(&session, owner), before_seq[owner]);
        assert_eq!(canonical_json(&latest_image(&session, owner)), canonical_json(&before_images[owner]));
        store::audit(&session.dbs[owner], &session.configs[owner]).unwrap();
        assert_eq!(staging_count(&session, owner), 0);
    }
}

#[test]
#[ignore = "需要 TB03C_FIN2_CARRIER_ROOT 指向四个独立 HFS+ 挂载根"]
fn fin2_capacity_shared_carrier_and_non_hfs_are_rejected() {
    let root = carrier_root();
    for field in ["max_store_bytes", "max_staged_bytes"] {
        let mut value = fin2_manifest(&root);
        value["finance"]["structure"]["reader_resources"]["job_limits"][field] = json!("1");
        let path = root.join(format!("{field}.json"));
        std::fs::write(&path, canonical_json(&value)).unwrap();
        let error = store::Config::load(&path, &root.join("e.db"), None)
            .err()
            .unwrap();
        assert!(error.contains("总容量超过冻结 staged/store 上限"), "{field}: {error}");
    }

    let mut shared = fin2_manifest(&root);
    shared["chongs"][0]["db"] = json!(root.join("e-shared.db"));
    let path = root.join("shared-carrier.json");
    std::fs::write(&path, canonical_json(&shared)).unwrap();
    let error = store::Config::load(&path, &root.join("e.db"), None)
        .err()
        .unwrap();
    assert!(error.contains("必须各占独立固定容量卷"), "{error}");

    let apfs = root.join("apfs");
    std::fs::create_dir_all(&apfs).unwrap();
    let mut non_hfs = fin2_manifest(&root);
    non_hfs["e"]["db"] = json!(apfs.join("e.db"));
    let path = apfs.join("non-hfs.json");
    std::fs::write(&path, canonical_json(&non_hfs)).unwrap();
    let error = store::Config::load(&path, &apfs.join("e.db"), None)
        .err()
        .unwrap();
    assert!(error.contains("HFS") || error.contains("mount root"), "{error}");
    assert!(!apfs.join("e.db").exists());
}
