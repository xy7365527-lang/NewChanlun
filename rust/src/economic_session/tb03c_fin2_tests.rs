//! #1672：FIN/2 在四个独立 HFS+ 载体上的 WE7 正例、恢复与载体负控。
use super::tb03c_allocation_tests::{
    apply_request, capture_request, manifest, resolve_request, Session,
};
use super::tb03c_book_tests::Space;
use super::{book, store};
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};
use std::io::Write;
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
use std::path::{Path, PathBuf};

const RAW: &str = include_str!("../../../s_session/tests/fixtures/tb03c_allocation/RAW.json");
const HAND: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/HAND-ORACLE.json");

fn carrier_root() -> PathBuf {
    PathBuf::from(
        std::env::var("TB03C_FIN2_CARRIER_ROOT")
            .expect("TB03C_FIN2_CARRIER_ROOT must point to four mounted HFS+ roots"),
    )
}

fn owner_db(root: &Path, owner: usize) -> PathBuf {
    root.join(["e", "b0", "b1", "b2"][owner]).join("owner.db")
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
    let applicants = json!(["fin2-unused-decision"]);
    let complete_request_hash = book::hash(&applicants);
    let object_id = "fin2-unused-S-object";
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
        "rights":[{
            "right_id":"fin2-unused-right",
            "owner":"B:chong-N1",
            "version":"1",
            "amount":"1000",
            "currency":"TEST-CASH",
            "source_ref":"TestOnly FIN/2 unused closed-plan right"
        }],
        "plans":[{
            "decision_id":"fin2-unused-decision",
            "attempt_id":"fin2-unused-attempt",
            "plan_id":"fin2-unused-plan",
            "owner":"B:chong-N1",
            "voice_binding_id":"fin2-unused-voice",
            "right_id":"fin2-unused-right",
            "expected_book_version":"1",
            "expected_rights_version":"1",
            "applicant_epoch":"1",
            "applicant_generation":"1",
            "slot_id":"fin2-unused-slot",
            "quantity":"1",
            "limit_price":"1",
            "fee_tail":"0",
            "quantity_source":"TestOnly FIN/2 unused quantity",
            "price_source":"TestOnly FIN/2 unused price",
            "fee_source":"TestOnly FIN/2 unused fee",
            "s_ref":{
                "schema_revision":"s-object-ref/2",
                "object_id":object_id,
                "object_hash":book::hash(&json!(object_id)),
                "profile_id":"fin2-unused-profile"
            },
            "o_ref":{
                "operation_id":"fin2-unused-operation",
                "kind":"TestOnlyCashBuy",
                "source_ref":"TestOnly FIN/2 unused operation",
                "s_object_id":object_id,
                "quantity":"1"
            }
        }],
        "slots":[{
            "slot_id":"fin2-unused-slot",
            "applicants":applicants,
            "input_end":{
                "kind":"ControlledInputEnd",
                "source_id":"fin2-we7-test",
                "source_epoch":"1",
                "watermark":"1",
                "complete_request_hash":complete_request_hash
            }
        }]
    });
    let raw = canonical_json(&input);
    let path = root.join("fin-input.json");
    std::fs::write(&path, raw.as_bytes()).unwrap();
    (path, sha256_hex(raw.as_bytes()))
}

fn fin2_manifest(space: &Space, root: &Path) -> Value {
    let mut m = manifest(space, true);
    let (input_path, input_sha256) = write_fin_input(&space.0);
    m["e"]["db"] = json!(owner_db(root, 0));
    for index in 0..3 {
        m["chongs"][index]["db"] = json!(owner_db(root, index + 1));
    }
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
            "binary":space.0.join("unit-S-binary"),
            "binary_sha256":book::hash(&json!("unit")),
            "db":space.0.join("s.db"),
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
    let space = Space::new("fin2-hfs");
    let manifest_value = fin2_manifest(&space, root);
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(&manifest_value)).unwrap();
    let mut configs = vec![store::Config::load(&path, &owner_db(root, 0), None).unwrap()];
    for index in 0..3 {
        configs.push(
            store::Config::load(
                &path,
                &owner_db(root, index + 1),
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

struct Original {
    owner: usize,
    request: Value,
    request_json: String,
    response: Value,
}

fn stored_request_json(session: &Session, owner: usize, message_id: &str) -> String {
    session.dbs[owner]
        .query_row(
            "SELECT request_json FROM messages WHERE json_extract(request_json,'$.message_id')=?1",
            [message_id],
            |row| row.get(0),
        )
        .unwrap()
}

fn storage_stats(session: &Session, owner: usize) -> Value {
    let db = &session.dbs[owner];
    let read = |name: &str| -> i64 {
        db.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .unwrap()
    };
    let path = &session.configs[owner].db;
    let mut files = Vec::new();
    for suffix in ["", "-wal", "-shm"] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        let metadata = match std::fs::metadata(PathBuf::from(name)) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("storage metadata: {error}"),
        };
        files.push(json!({
            "suffix":suffix,
            "bytes":metadata.as_ref().map_or(0, |metadata| metadata.len()),
            "allocated_bytes":metadata.as_ref().map_or(0, |metadata| metadata.blocks() * 512),
        }));
    }
    let parent = std::ffi::CString::new(path.parent().unwrap().as_os_str().as_bytes()).unwrap();
    let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    assert_eq!(
        unsafe { libc::statvfs(parent.as_ptr(), vfs.as_mut_ptr()) },
        0
    );
    let vfs = unsafe { vfs.assume_init() };
    let free = u64::from(vfs.f_bavail) * vfs.f_frsize;
    let page_size = read("page_size");
    let max_pages = read("max_page_count");
    let capacity = super::content_store::carrier_capacity(path).unwrap();
    let carrier_page_limit = capacity / (8 * (u64::try_from(page_size).unwrap() + 24));
    let one = i64::try_from(carrier_page_limit).unwrap() * (page_size + 24) + 65536;
    let image_bytes: i64 = db
        .query_row(
            "SELECT length(CAST(image AS BLOB)) FROM commits ORDER BY seq DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    json!({
        "page_size":page_size,
        "page_count":read("page_count"),
        "connection_max_page_count":max_pages,
        "freelist_count":read("freelist_count"),
        "current_image_bytes":image_bytes,
        "allocated":super::content_store::allocated(path).unwrap(),
        "files":files,
        "capacity_bytes":capacity,
        "carrier_page_limit":carrier_page_limit,
        "cleanup_budget_basis":"carrier_page_limit",
        "free_bytes":free,
        "cleanup_needed_bytes":2 * one,
        "cleanup_margin_bytes":i64::try_from(free).unwrap() - 2 * one,
        "next_write_needed_bytes":4 * one,
        "next_write_margin_bytes":i64::try_from(free).unwrap() - 4 * one,
    })
}

fn archive_evidence(name: &str, bytes: &[u8], append: bool) {
    if let Some(directory) = std::env::var_os("TB03C_FIN2_EVIDENCE_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(directory.join(name))
            .unwrap();
        file.write_all(bytes).unwrap();
    }
}

fn observe_storage(session: &Session, owner: usize, phase: &str, message_id: &str) {
    let observation = json!({
        "phase":phase,
        "message_id":message_id,
        "owner":session.configs[owner].domain,
        "writer_epoch":session.epochs[owner],
        "seq":latest_seq(session, owner),
        "storage":storage_stats(session, owner),
    });
    let line = format!("{}\n", canonical_json(&observation));
    print!("FIN2_STORAGE {line}");
    archive_evidence("storage.jsonl", line.as_bytes(), true);
}

fn apply_image_sizes(session: &Session, owner: usize, request: &Value) -> (usize, usize) {
    let payload = &request["payload"];
    let fact_key = payload["fact_key"].as_str().unwrap();
    let e = &session.dbs[0];
    let e_config = &session.configs[0];
    let mut record = store::row(e, "facts", fact_key).unwrap().unwrap();
    let receipt = store::row(e, "receipts", fact_key).unwrap().unwrap();
    record["cut"] = receipt["cut"].clone();
    record["e_receipt"] = receipt;
    record["e_domain_id"] = json!("E");
    record["e_session_id"] = json!(e_config.session);
    record["e_session_generation"] = json!(e_config.generation);
    let recorded_seq: i64 = record["recorded_seq"].as_str().unwrap().parse().unwrap();
    let (previous_root, commit_ns, image): (String, String, String) = e
        .query_row(
            "SELECT prev_root,commit_ns,image FROM commits WHERE seq=?",
            [recorded_seq],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    record["e_commit_proof"] = json!({
        "previous_root":previous_root,
        "commit_ns":commit_ns,
        "image":serde_json::from_str::<Value>(&image).unwrap(),
    });
    super::tb03c_allocation::attach_resolution(e, &session.configs[owner], &mut record).unwrap();
    let message_id = request["message_id"].as_str().unwrap();
    let recovery_index = usize::try_from(session.epochs[owner] - 1).unwrap();
    let config = &session.configs[owner];
    let received_ns = config
        .clock(Some(message_id), "received_ns", recovery_index)
        .unwrap();
    let first_known_ns = config
        .clock(Some(message_id), "first_known_ns", recovery_index)
        .unwrap();
    let commit_ns = config
        .clock(Some(message_id), "commit_ns", recovery_index)
        .unwrap();
    record["b_observation"] = json!({
        "received_ns":received_ns,
        "first_known_ns":first_known_ns,
        "commit_ns":commit_ns.clone(),
    });
    record["b_origin_message"] = json!({
        "source_namespace":request["source_namespace"],
        "source_epoch":request["source_epoch"],
        "message_id":request["message_id"],
        "payload_hash":request["payload_hash"],
    });
    record["b_origin_request_hash"] = json!(book::hash(request));
    let current = latest_image(session, owner);
    let (next, _) = book::apply_in(
        &session.configs[owner].manifest,
        &current,
        &record,
        latest_seq(session, owner) + 1,
        &commit_ns,
    )
    .unwrap();
    (canonical_json(&current).len(), canonical_json(&next).len())
}

fn send_original(
    session: &mut Session,
    originals: &mut Vec<Original>,
    owner: usize,
    payload: Value,
) -> Value {
    let request = session.new_envelope(owner, payload);
    let response = session.send_envelope(owner, &request);
    let evidence = json!({"owner":owner,"request":request,"response":response});
    archive_evidence(
        "originals.jsonl",
        format!("{}\n", canonical_json(&evidence)).as_bytes(),
        true,
    );
    observe_storage(
        session,
        owner,
        "original",
        request["message_id"].as_str().unwrap(),
    );
    assert_eq!(response["schema_revision"], "economic-session/2");
    assert_eq!(response["source_namespace"], "economic-session/replies");
    if response["payload"]["kind"] == "Error" {
        let mut storage = storage_stats(session, owner);
        if request["payload"]["op"] == "ApplyAllocation" {
            let (current_image_bytes, prospective_image_bytes) =
                apply_image_sizes(session, owner, &request);
            storage["current_image_bytes"] = json!(current_image_bytes);
            storage["prospective_image_bytes"] = json!(prospective_image_bytes);
        }
        panic!(
            "原请求 {} 失败: {response}; storage={storage}",
            request["message_id"]
        );
    }
    let message_id = request["message_id"].as_str().unwrap();
    let request_json = stored_request_json(session, owner, message_id);
    assert_eq!(request_json, canonical_json(&request));
    let payload = response["payload"].clone();
    originals.push(Original {
        owner,
        request,
        request_json,
        response,
    });
    payload
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
    let root = carrier_root();
    let sentinel = root.join("fin2-we7-do-not-delete.sentinel");
    std::fs::write(&sentinel, "preserve the caller-owned carrier root").unwrap();
    let mut session = load(&root);
    for name in ["manifest.json", "fin-input.json"] {
        archive_evidence(
            name,
            &std::fs::read(session._space.0.join(name)).unwrap(),
            false,
        );
    }
    archive_evidence("RAW.json", RAW.as_bytes(), false);
    archive_evidence("HAND-ORACLE.json", HAND.as_bytes(), false);
    for (owner, config) in session.configs.iter().enumerate() {
        let revision: String = session.dbs[owner]
            .query_row("SELECT v FROM meta WHERE k='schema_revision'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(revision, "economic-session/2");
        let format: String = session.dbs[owner]
            .query_row(
                "SELECT content_format FROM content_meta WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(format, "s-content/1");
        assert_eq!(
            config.domain,
            ["E", "B:chong-N1", "B:chong-N2", "B:chong-N3"][owner]
        );
    }

    let imported = session.send(
        0,
        json!({"op":"ImportResponsibility","responsibility_id":"fin2-we7-baseline","writer_epoch":"1"}),
    );
    assert_eq!(
        imported["kind"], "ResponsibilityImportedReceipt",
        "{imported}"
    );

    archive_evidence(
        "import-receipt.json",
        canonical_json(&imported).as_bytes(),
        false,
    );
    for owner in 0..4 {
        observe_storage(&session, owner, "initialized", "");
    }
    let hand: Value = serde_json::from_str(HAND).unwrap();
    let mut originals = Vec::new();
    let mut captures = Vec::new();
    let mut resolves = Vec::new();
    let mut applies = Vec::new();
    for index in 0..3 {
        let capture = send_original(&mut session, &mut originals, 0, capture_request(RAW, index));
        assert_eq!(capture["kind"], "Recorded", "{capture}");
        let applied = send_original(
            &mut session,
            &mut originals,
            index + 1,
            apply_request(&capture, &capture["allocation_id"]),
        );
        assert_eq!(applied["kind"], "Applied", "{applied}");
        store::audit(&session.dbs[index + 1], &session.configs[index + 1]).unwrap();
        assert_eq!(staging_count(&session, index + 1), 0);
        captures.push(capture);
        applies.push(applied);
    }

    for round in 0..3 {
        let capture = send_original(
            &mut session,
            &mut originals,
            0,
            capture_request(RAW, round + 3),
        );
        assert_eq!(capture["kind"], "Recorded", "{capture}");
        let resolved = send_original(
            &mut session,
            &mut originals,
            0,
            resolve_request(&capture, round + 4),
        );
        assert_eq!(resolved["kind"], "Resolved", "{resolved}");
        let allocations = session.allocations(&capture);
        for index in 0..3 {
            let applied = send_original(
                &mut session,
                &mut originals,
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
                hand["reports"][round]["K"][index]
                    .as_i64()
                    .unwrap()
                    .to_string()
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
    assert_eq!(originals.len(), 21);
    let mut allocation_ids = std::collections::BTreeSet::new();
    for receipt in &applies {
        assert_eq!(receipt["kind"], "Applied");
        assert!(allocation_ids.insert(receipt["allocation_id"].as_str().unwrap().to_owned()));
    }
    assert_eq!(allocation_ids.len(), 12);

    let before_images = (0..4)
        .map(|owner| latest_image(&session, owner))
        .collect::<Vec<_>>();
    let before_seq = (0..4)
        .map(|owner| latest_seq(&session, owner))
        .collect::<Vec<_>>();
    for owner in 0..4 {
        observe_storage(&session, owner, "before_recover", "");
        drop(session.dbs.remove(owner));
        let recovered = store::recover(&session.configs[owner]).unwrap();
        archive_evidence(
            "recoveries.jsonl",
            format!("{}\n", canonical_json(&recovered)).as_bytes(),
            true,
        );
        assert_eq!(recovered["kind"], "Recovered");
        let next_epoch = book::integer(&recovered, "writer_epoch").unwrap();
        assert!(next_epoch > session.epochs[owner]);
        session.epochs[owner] = next_epoch;
        session.dbs.insert(
            owner,
            store::open(&session.configs[owner].db, false).unwrap().0,
        );
        store::audit(&session.dbs[owner], &session.configs[owner]).unwrap();
        assert_eq!(
            canonical_json(&latest_image(&session, owner)),
            canonical_json(&before_images[owner])
        );
        assert_eq!(staging_count(&session, owner), 0);
        observe_storage(&session, owner, "after_recover", "");
    }

    let before_messages = (0..4)
        .map(|owner| store::rows(&session.dbs[owner], "messages").unwrap())
        .collect::<Vec<_>>();
    let before_receipts = (0..4)
        .map(|owner| store::rows(&session.dbs[owner], "receipts").unwrap())
        .collect::<Vec<_>>();
    let before_applications = (0..4)
        .map(|owner| store::rows(&session.dbs[owner], "applications").unwrap())
        .collect::<Vec<_>>();
    let before_next = session.next;

    // 新 writer epoch 下逐字重送 6 Capture + 3 Resolve + 12 Apply 的原信封。
    // DB 原 request_json、提交序列与 application 表保持不变；完整 /2 响应仅
    // writer epoch 随 recover 合法前进，其余字段（含原收据与因果引用）逐项相同。
    for original in &originals {
        assert_eq!(
            canonical_json(&original.request),
            original.request_json,
            "重送前原信封字节漂移"
        );
        let message_id = original.request["message_id"].as_str().unwrap();
        observe_storage(&session, original.owner, "before_resend", message_id);
        let response = session.send_envelope(original.owner, &original.request);
        observe_storage(&session, original.owner, "after_resend", message_id);
        archive_evidence(
            "resends.jsonl",
            format!(
                "{}\n",
                canonical_json(&json!({
                    "owner":original.owner,"request":original.request,"response":response
                }))
            )
            .as_bytes(),
            true,
        );
        let mut expected = original.response.clone();
        let writer_epoch = json!(session.epochs[original.owner].to_string());
        expected["source_epoch"] = writer_epoch.clone();
        expected["producer_epoch"] = writer_epoch;
        assert_eq!(response, expected);
        assert_eq!(response["schema_revision"], "economic-session/2");
        assert_eq!(response["source_namespace"], "economic-session/replies");
        assert_eq!(
            stored_request_json(
                &session,
                original.owner,
                original.request["message_id"].as_str().unwrap(),
            ),
            original.request_json
        );
    }
    assert_eq!(session.next, before_next, "重送不得分配新 message_id");
    for owner in 0..4 {
        assert_eq!(latest_seq(&session, owner), before_seq[owner]);
        assert_eq!(
            canonical_json(&latest_image(&session, owner)),
            canonical_json(&before_images[owner])
        );
        store::audit(&session.dbs[owner], &session.configs[owner]).unwrap();
        assert_eq!(staging_count(&session, owner), 0);
        assert_eq!(
            store::rows(&session.dbs[owner], "messages").unwrap(),
            before_messages[owner]
        );
        assert_eq!(
            store::rows(&session.dbs[owner], "receipts").unwrap(),
            before_receipts[owner]
        );
        assert_eq!(
            store::rows(&session.dbs[owner], "applications").unwrap(),
            before_applications[owner]
        );
    }

    let final_books = (1..4)
        .map(|owner| latest_image(&session, owner)["book"].clone())
        .collect::<Vec<_>>();
    archive_evidence(
        "RESULT.json",
        canonical_json(&json!({
            "captures":captures.len(),"resolves":resolves.len(),"applies":applies.len(),
            "resends":originals.len(),"final_books":final_books,"epochs":session.epochs,
            "sequences":before_seq,"next":session.next,"recovery_and_resend_invariants":"passed",
        }))
        .as_bytes(),
        false,
    );
    drop(session);
    assert!(root.is_dir(), "测试不得删除调用方载体根");
    assert_eq!(
        std::fs::read_to_string(&sentinel).unwrap(),
        "preserve the caller-owned carrier root"
    );
    for owner in 0..4 {
        let db = owner_db(&root, owner);
        assert!(db.parent().unwrap().is_dir(), "测试不得卸载/删除挂载根");
        assert!(db.is_file(), "测试 owner DB 必须位于真实挂载根");
    }
    std::fs::remove_file(sentinel).unwrap();
}

#[test]
#[ignore = "需要 TB03C_FIN2_CARRIER_ROOT 指向四个独立 HFS+ 挂载根"]
fn fin2_capacity_shared_carrier_and_non_hfs_are_rejected() {
    let root = carrier_root();
    let sentinel = root.join("fin2-negative-do-not-delete.sentinel");
    std::fs::write(&sentinel, "preserve the caller-owned carrier root").unwrap();
    let space = Space::new("fin2-hfs-negative");
    for field in ["max_store_bytes", "max_staged_bytes"] {
        let mut value = fin2_manifest(&space, &root);
        value["finance"]["structure"]["reader_resources"]["job_limits"][field] = json!("1");
        let path = space.0.join(format!("{field}.json"));
        std::fs::write(&path, canonical_json(&value)).unwrap();
        let error = store::Config::load(&path, &owner_db(&root, 0), None)
            .err()
            .unwrap();
        assert!(
            error.contains("总容量超过冻结 staged/store 上限"),
            "{field}: {error}"
        );
    }

    let mut shared = fin2_manifest(&space, &root);
    shared["chongs"][0]["db"] = json!(root.join("e").join("shared-owner.db"));
    let path = space.0.join("shared-carrier.json");
    std::fs::write(&path, canonical_json(&shared)).unwrap();
    let error = store::Config::load(&path, &owner_db(&root, 0), None)
        .err()
        .unwrap();
    assert!(error.contains("必须各占独立固定容量卷"), "{error}");

    let nested = root.join("e").join("not-a-mount-root");
    std::fs::create_dir(&nested).unwrap();
    let mut non_root = fin2_manifest(&space, &root);
    non_root["e"]["db"] = json!(nested.join("owner.db"));
    let path = space.0.join("non-mount-root.json");
    std::fs::write(&path, canonical_json(&non_root)).unwrap();
    let error = store::Config::load(&path, &nested.join("owner.db"), None)
        .err()
        .unwrap();
    assert!(error.contains("mount root"), "{error}");
    assert!(!nested.join("owner.db").exists());
    std::fs::remove_dir(nested).unwrap();

    let apfs = space.0.join("apfs");
    std::fs::create_dir_all(&apfs).unwrap();
    let mut non_hfs = fin2_manifest(&space, &root);
    non_hfs["e"]["db"] = json!(apfs.join("owner.db"));
    let path = space.0.join("non-hfs.json");
    std::fs::write(&path, canonical_json(&non_hfs)).unwrap();
    let error = store::Config::load(&path, &apfs.join("owner.db"), None)
        .err()
        .unwrap();
    assert!(
        error.contains("HFS") || error.contains("mount root"),
        "{error}"
    );
    assert!(!apfs.join("owner.db").exists());
    drop(space);
    assert!(root.is_dir(), "负控不得删除调用方载体根");
    assert_eq!(
        std::fs::read_to_string(&sentinel).unwrap(),
        "preserve the caller-owned carrier root"
    );
    for owner in 0..4 {
        assert!(owner_db(&root, owner).parent().unwrap().is_dir());
    }
    std::fs::remove_file(sentinel).unwrap();
}
