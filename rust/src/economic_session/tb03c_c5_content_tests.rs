//! /2 内容读口的独立小型 SQLite 锁；不声称固定 HFS 载体/正式服务已验收。
use super::*;
use crate::economic_session::{content_store, store::ReadBudget};
use crate::s_content::job::{Existing, JobLimits, DEADLINE, LIMIT_EXCEEDED};
use rusqlite::{params, Connection};
use std::{cell::Cell, rc::Rc, time::Duration};

fn limits(records: u64) -> JobLimits {
    JobLimits {
        max_total_wire_bytes: 16_777_216,
        max_total_node_bytes: 16_777_216,
        max_total_nodes: 100_000,
        max_total_records: records,
        max_total_requests: 100,
        max_elapsed_ms: 1000,
        max_staged_bytes: 16_777_216,
        max_store_bytes: 16_777_216,
    }
}
fn new_job(records: u64) -> Job {
    let now = Instant::now();
    // 仅内存 SQLite，磁盘既存/暂存实测为空。
    Job::new(
        limits(records),
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(move || now),
    )
    .unwrap()
}
fn db() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE content_nodes(digest BLOB PRIMARY KEY,tag INTEGER,body BLOB); CREATE TABLE meta(k TEXT PRIMARY KEY,v TEXT); INSERT INTO meta VALUES('schema_revision','economic-session/2'); CREATE TABLE content_meta(singleton INTEGER, content_format TEXT); INSERT INTO content_meta VALUES(1,'s-content/1');").unwrap();
    c
}
fn read(
    c: &Connection,
    digest: &NodeDigest,
    job: &mut Job,
    deadline: Option<Instant>,
) -> Result<NodeRead, CodecError> {
    content_store::read_table_metered(
        c,
        "content_nodes",
        digest,
        &mut ReadBudget::new(Some(job), deadline),
    )
}
#[test]
fn c5_h2_content_bad_tag_and_digest_keep_sql_bytes_without_business_records() {
    let c = db();
    let digest = sc::frame(0, &[]);
    for (tag, body) in [(256i64, vec![]), (0, vec![1u8])] {
        c.execute(
            "INSERT OR REPLACE INTO content_nodes VALUES(?1,?2,?3)",
            params![&digest[..], tag, &body],
        )
        .unwrap();
        let mut job = new_job(1);
        let failed = read(&c, &digest, &mut job, None);
        assert!(failed.is_err());
        let first = job.snapshot();
        assert_eq!(first.wire_bytes, 8 + body.len() as u128);
        assert_eq!(first.records, 0);
        assert_eq!(first.nodes, 0);
        assert!(read(&c, &digest, &mut job, None).is_err());
        assert_eq!(job.snapshot().wire_bytes, first.wire_bytes * 2);
        assert_eq!(job.snapshot().records, 0);
        assert!(matches!(
            read(&c, &[7; 32], &mut job, None).unwrap(),
            NodeRead::Missing
        ));
        assert_eq!(job.snapshot().wire_bytes, first.wire_bytes * 2);
    }
}
#[test]
fn c5_h2_content_repeated_node_counts_wire_once_per_sql_return_and_stops_at_deadline() {
    let c = db();
    let mut nodes = sc::MemoryNodes::new();
    let root = sc::write_value(&mut nodes, &sc::Value::string("same-node"))
        .unwrap()
        .content_ref();
    let digest = root.digest().unwrap();
    let (tag, body) = nodes.get(&digest).unwrap();
    c.execute(
        "INSERT INTO content_nodes VALUES(?1,?2,?3)",
        params![&digest[..], i64::from(tag), body],
    )
    .unwrap();
    let mut job = new_job(1);
    for _ in 0..2 {
        let mut callback = |digest: &NodeDigest, job: &mut Job| read(&c, digest, job, None);
        let mut source = MeteredSource {
            source: ReadSource::WithJob(&mut callback),
            job: &mut job,
        };
        source.read(&digest).unwrap();
    }
    let twice = job.snapshot();
    assert_eq!(twice.wire_bytes, 2 * (8 + body.len() as u128));
    assert_eq!(twice.nodes, 1);
    assert_eq!(twice.records, 0);
    let expired = Instant::now();
    assert!(read(&c, &digest, &mut job, Some(expired)).is_err());
    assert_eq!(job.snapshot().wire_bytes, twice.wire_bytes);
    // 同一 Job 自身期限到达，也不得读取下一个节点。
    let clock = Rc::new(Cell::new(Instant::now()));
    let for_job = clock.clone();
    let mut timed = Job::new(
        limits(1),
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(move || for_job.get()),
    )
    .unwrap();
    read(&c, &digest, &mut timed, None).unwrap();
    let before = timed.snapshot().wire_bytes;
    clock.set(clock.get() + Duration::from_millis(1000));
    assert!(read(&c, &digest, &mut timed, None).is_err());
    assert_eq!(timed.snapshot().stopped, Some(DEADLINE));
    assert_eq!(timed.snapshot().wire_bytes, before);
}

#[test]
fn c5_h2_content_read_returned_at_deadline_keeps_bytes_and_rejects_next_read() {
    let c = db();
    let digest = sc::frame(0, &[]);
    c.execute(
        "INSERT INTO content_nodes VALUES(?1,0,?2)",
        params![&digest[..], &[] as &[u8]],
    )
    .unwrap();
    let clock = Rc::new(Cell::new(Instant::now()));
    let for_job = clock.clone();
    let mut cap = limits(1);
    cap.max_elapsed_ms = 1;
    let mut job = Job::new(
        cap,
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(move || for_job.get()),
    )
    .unwrap();
    let _hook = crate::economic_session::tb03c_c5_read_tests::expire_on_next_sql_return(clock);
    let failure = read(&c, &digest, &mut job, None).unwrap_err();
    assert!(failure.to_string().contains(DEADLINE), "{failure}");
    assert_eq!(job.snapshot().wire_bytes, 8);
    assert_eq!(job.snapshot().records, 0);
    assert_eq!(job.snapshot().nodes, 0);
    assert!(read(&c, &digest, &mut job, None).is_err());
    assert_eq!(job.snapshot().wire_bytes, 8);
}
fn fixture(records: u64) -> (Connection, Value, Value, Value) {
    let c = db();
    let mut snapshot: Vec<_> = SNAPSHOT
        .iter()
        .map(|key| (key.to_string(), sc::Value::Null))
        .collect();
    for (name, value) in &mut snapshot {
        *value = match name.as_str() {
            "session_id" => sc::Value::string("c5-content"),
            "history_mode" => sc::Value::string("RecomputedWithRevision"),
            "profile_id" => sc::Value::string("profile"),
            "objects" => sc::Value::Array(vec![
                sc::Value::Object(vec![("object_id".into(), sc::Value::string("a"))]),
                sc::Value::Object(vec![("object_id".into(), sc::Value::string("b"))]),
            ]),
            key if FAMILIES.contains(&key) => sc::Value::Array(vec![]),
            _ => sc::Value::Null,
        };
    }
    let mut nodes = sc::MemoryNodes::new();
    let root = sc::write_value(
        &mut nodes,
        &sc::Value::Object(vec![
            ("kind".into(), sc::Value::string("FinancialBasis")),
            ("session_id".into(), sc::Value::string("c5-content")),
            ("session_generation".into(), sc::Value::string("1")),
            ("writer_epoch".into(), sc::Value::string("1")),
            ("pending_revision".into(), sc::Value::Bool(false)),
            ("commit_ns".into(), sc::Value::string("1")),
            ("snapshot".into(), sc::Value::Object(snapshot)),
        ]),
    )
    .unwrap()
    .content_ref();
    let object = path::lookup_path(
        &mut nodes,
        &root,
        &[field("snapshot"), field("objects"), PathStep::Index(0)],
    )
    .unwrap();
    let hash = sc::frame(0x23, &object.digest().unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    for (digest, tag, body) in nodes.iter() {
        c.execute(
            "INSERT INTO content_nodes VALUES(?1,?2,?3)",
            params![&digest[..], i64::from(tag), body],
        )
        .unwrap();
    }
    let caps: serde_json::Map<String, Value> = limits(records)
        .entries()
        .into_iter()
        .map(|(k, v)| (k.to_owned(), json!(v.to_string())))
        .collect();
    let structure = json!({"binary":"/frozen/c5-s","binary_sha256":"0".repeat(64),"db":"/frozen/c5-db","session_id":"c5-content","session_generation":"1","writer_epoch":"1","reader_revision":"s-financial-basis/3","content_format":"s-content/1","reader_session":null,
        "reader_resources":{"schema_revision":"s-financial-reader-resources/1","max_request_bytes":"1048576","max_reply_bytes":"1048576","max_stderr_bytes":"1048576","request_timeout_ms":"1000","job_limits":caps}});
    let evidence = json!({"schema_revision":"s-financial-evidence/1","content_format":"s-content/1","kind":"FinancialBasisEvidence","session_id":"c5-content","session_generation":"1","writer_epoch":"1","pending_revision":false,"commit_ns":"1","basis_ref":root.to_transport_json()});
    let selected = json!({"schema_revision":"s-object-ref/2","object_id":"a","object_hash":hash,"profile_id":"profile"});
    (c, structure, evidence, selected)
}
#[test]
fn c5_h2_content_s_ref_rechecks_use_same_job_and_preserve_business_record_count() {
    let (c, structure, evidence, selected) = fixture(100);
    let mut job = new_job(100);
    for visit in 1..=2 {
        let mut callback = |digest: &NodeDigest, job: &mut Job| read(&c, digest, job, None);
        let result = verify_evidence_with_read(
            &structure,
            &evidence,
            &mut callback,
            &mut job,
            Some(&selected),
        )
        .unwrap();
        assert_eq!(result.selected_objects.len(), 1);
        // 2 个六族行 + s_ref 扫描 2 行 + selected 访问 1 行；SQL 行不增加该账。
        assert_eq!(job.snapshot().records, 5 * visit);
    }
    let first_two = job.snapshot();
    let mut once = new_job(100);
    let mut callback = |digest: &NodeDigest, job: &mut Job| read(&c, digest, job, None);
    verify_evidence_with_read(
        &structure,
        &evidence,
        &mut callback,
        &mut once,
        Some(&selected),
    )
    .unwrap();
    assert_eq!(first_two.wire_bytes, once.snapshot().wire_bytes * 2);
    assert_eq!(first_two.nodes, once.snapshot().nodes);
}
#[test]
fn c5_h2_content_low_record_budget_stops_real_snapshot_rows_before_more_sql() {
    let (c, structure, evidence, _) = fixture(1);
    let mut job = new_job(1);
    let mut callback = |digest: &NodeDigest, job: &mut Job| read(&c, digest, job, None);
    let failed = verify_evidence_with_read(&structure, &evidence, &mut callback, &mut job, None)
        .unwrap_err();
    assert!(failed.contains(LIMIT_EXCEEDED), "{failed}");
    let before = job.snapshot();
    assert_eq!(before.records, 1);
    assert_eq!(before.stopped, Some(LIMIT_EXCEEDED));
    assert!(before.wire_bytes > 0);
    let mut callback = |digest: &NodeDigest, job: &mut Job| read(&c, digest, job, None);
    assert!(
        verify_evidence_with_read(&structure, &evidence, &mut callback, &mut job, None).is_err()
    );
    assert_eq!(job.snapshot().wire_bytes, before.wire_bytes);
    assert_eq!(job.snapshot().records, before.records);
}
