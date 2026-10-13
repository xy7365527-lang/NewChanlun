//! #1394 第三问选项 A：固定 HFS+ 载体上的实测入口（`#[ignore]`，由 `tools/drive.py` 以环境变量驱动）。
//! 走产品路径：`content_store::Stage`（configure、8 倍页规则、cleanup_room、物理对账）、
//! `read_current_pins`（会话 pins → financial-closure 分块导入 → 成员核验）、最终经济事务的内容部分
//! （`reserve_final` → 同一事务 `publish` → COMMIT → `reconcile_final`）、`verify_image` 离线审计、
//! 批量 `import_peer_refs`。不写经济历史行；每次调用把读数写到 `CLOSURE_REPLICA_OUT`。
use super::super::{content_store, store};
use super::*;
use crate::s_content::job::{Existing, JobLimits};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("缺环境变量 {name}"))
}
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
fn blocks(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).map_or(0, |m| m.blocks() * 512)
}
fn side(db: &Path, suffix: &str) -> PathBuf {
    let mut s = db.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}
fn facts(c: &Connection, db: &Path) -> Value {
    let p = |name: &str| -> i64 {
        c.query_row(&format!("PRAGMA {name}"), [], |r| r.get(0))
            .unwrap_or(-1)
    };
    let count = |sql: &str| -> i64 { c.query_row(sql, [], |r| r.get(0)).unwrap_or(-1) };
    let mut vfs: libc::statvfs = unsafe { std::mem::zeroed() };
    let parent = std::ffi::CString::new(db.parent().unwrap().to_str().unwrap()).unwrap();
    unsafe { libc::statvfs(parent.as_ptr(), &mut vfs) };
    json!({"page_size":p("page_size"),"page_count":p("page_count"),"freelist_count":p("freelist_count"),
        "max_page_count":p("max_page_count"),
        "content_nodes_rows":count("SELECT count(*) FROM content_nodes"),
        "content_staging_rows":count("SELECT count(*) FROM content_staging"),
        "db_bytes":std::fs::metadata(db).map_or(0, |m| m.len()),
        "db_st_blocks_bytes":blocks(db),"wal_st_blocks_bytes":blocks(&side(db, "-wal")),
        "shm_st_blocks_bytes":blocks(&side(db, "-shm")),
        "allocated_total":content_store::allocated(db).ok(),
        "carrier_capacity":content_store::carrier_capacity(db).ok(),
        "volume_free_bytes":u128::from(vfs.f_bavail) * u128::from(vfs.f_frsize),
        "volume_total_bytes":u128::from(vfs.f_blocks) * u128::from(vfs.f_frsize)})
}
fn rusage() -> Value {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    let tv = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1e6;
    json!({"ru_maxrss_bytes":ru.ru_maxrss,"user_s":tv(ru.ru_utime),"sys_s":tv(ru.ru_stime)})
}
/// 与 `store::init` 同一 DDL（`SQL` + `CONTENT_SQL`）与同一 `store::open` 前件；只写
/// `format_guard` 需要的 meta 与 content_meta 行，不写经济历史。
pub(super) fn create_db(db: &Path) -> Connection {
    assert!(!db.exists(), "拒绝覆盖已有库");
    let (mut c, _) = store::open(db, false).unwrap();
    let tx = c.transaction().unwrap();
    tx.execute_batch(store::TEST_DOMAIN_SQL).unwrap();
    tx.execute_batch(store::CONTENT_SQL).unwrap();
    tx.execute(
        "INSERT INTO meta(k,v) VALUES('schema_revision',?1)",
        [store::SCHEMA_V2],
    )
    .unwrap();
    tx.execute("INSERT INTO content_meta VALUES(1,?1)", [sc::CONTENT_FORMAT])
        .unwrap();
    tx.commit().unwrap();
    c
}
pub(super) fn new_job(c: &Connection, db: &Path, limits: JobLimits) -> Job {
    let existing: Existing = content_store::existing(c, db).unwrap();
    Job::new(limits, existing, Box::new(Instant::now)).unwrap()
}
pub(super) fn snapshot(job: &Job) -> Value {
    let s = job.snapshot();
    json!({"elapsed_ms":s.elapsed_ms,"wire_bytes":s.wire_bytes.to_string(),"node_bytes":s.node_bytes,
        "nodes":s.nodes,"records":s.records,"requests":s.requests,
        "staged_peak":s.staged.peak.to_string(),"store_peak":s.store.peak.to_string(),
        "staged_committed":s.staged.committed.to_string(),"store_committed":s.store.committed.to_string(),
        "open_reservations":s.open_reservations,"stopped":s.stopped})
}
/// 最终经济事务的内容部分（同 `finance::persist_with_job` 的顺序，不写经济历史行）。
pub(super) fn publish(
    c: &mut Connection,
    db: &Path,
    roots: &[ContentRef],
    job: &mut Job,
) -> Result<Value, String> {
    let t = Instant::now();
    let (grant, before, ceiling) =
        content_store::reserve_final(c, db, job).map_err(|e| e.to_string())?;
    let t1 = Instant::now();
    let tx = c.transaction().map_err(|e| e.to_string())?;
    content_store::publish(&tx, roots, job).map_err(|e| e.to_string())?;
    let publish_ms = ms(t1);
    let pages_in_tx: i64 = tx
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let t2 = Instant::now();
    tx.commit().map_err(|e| e.to_string())?;
    let commit_ms = ms(t2);
    let wal_after_commit = blocks(&side(db, "-wal"));
    let allocated_after_commit = content_store::allocated(db).map_err(|e| e.to_string())?;
    let actual = content_store::reconcile_final(c, db, job, &grant, before, ceiling)
        .map_err(|e| e.to_string())?;
    Ok(json!({"publish_in_tx_ms":publish_ms,"commit_ms":commit_ms,"total_ms":ms(t),
        "page_count_in_tx":pages_in_tx,"wal_st_blocks_after_commit":wal_after_commit,
        "allocated_after_commit_before_checkpoint":allocated_after_commit,
        "allocated_before":before,"actual_growth":actual,"ceiling":ceiling}))
}
fn audit(c: &Connection, db: &Path, structure: &Value, evidence: &Value, k: usize, limits: JobLimits) -> Value {
    let mut job = new_job(c, db, limits);
    let history: Vec<Value> = (0..k).map(|_| json!({"witness":{"structure":evidence}})).collect();
    let image = json!({"history":history,"recorded":[]});
    let t = Instant::now();
    let result = content_store::verify_image(c, &image, structure, &mut job);
    json!({"evidence_occurrences":k,"verify_image_ms":ms(t),"error":result.err().map(|e| e.to_string()),
        "job":snapshot(&job)})
}
fn structure() -> Value {
    let resources: Value =
        serde_json::from_slice(&std::fs::read(env("CLOSURE_READER_RESOURCES")).unwrap()).unwrap();
    json!({"binary":env("CLOSURE_S_BINARY"),"binary_sha256":env("CLOSURE_S_SHA256"),
        "db":env("CLOSURE_S_DB"),"session_id":env("CLOSURE_SESSION_ID"),
        "session_generation":"1","writer_epoch":"1","reader_revision":"s-financial-basis/3",
        "content_format":"s-content/1","reader_resources":resources})
}
fn write_report(report: &Value) {
    std::fs::write(
        env("CLOSURE_REPLICA_OUT"),
        serde_json::to_vec_pretty(report).unwrap(),
    )
    .unwrap();
}

/// 模式（`CLOSURE_REPLICA_MODE`）：
/// - `b`：空 owner 经会话取完整闭包 → 发布 → 审计（1 次与 `CLOSURE_AUDIT_K` 次证据出现）。
/// - `small`：同 `b`，预期在小载体上失败；记失败后暂存行（闭包尝试应已回滚为 0）。
/// - `e`：从已发布的 B 库（`CLOSURE_PEER_DB`）批量 peer 导入 → 经会话读当前 basis（本地已全，
///   不应再下载）→ 发布 → 审计 1 次。
#[test]
#[ignore = "需要 HFS 载体、c2+closure 会话与诊断复制品；由 tools/drive.py 驱动"]
fn replica_closure_on_fixed_carrier() {
    let mode = env("CLOSURE_REPLICA_MODE");
    let db = PathBuf::from(env("CLOSURE_EB_DB"));
    let structure = structure();
    let pins = SourcePins::from_structure(&structure)
        .unwrap()
        .with_session(&env("CLOSURE_SOCKET"))
        .unwrap();
    let limits = pins.resources.job_limits;
    let started = Instant::now();
    let mut report = json!({"schema":"issue1394-closure-candidate-replica/1","mode":mode,
        "db":db.display().to_string(),"structure":structure});
    let mut c = create_db(&db);
    report["carrier_initial"] = facts(&c, &db);
    let mut job = new_job(&c, &db, limits);
    let object_id = env("CLOSURE_OBJECT_ID");
    let deadline = Instant::now() + Duration::from_secs(3600);
    if mode == "e" {
        let peer_db = PathBuf::from(env("CLOSURE_PEER_DB"));
        let root_json: Value =
            serde_json::from_slice(&std::fs::read(env("CLOSURE_PEER_ROOT_JSON")).unwrap()).unwrap();
        let root = ContentRef::from_transport_json(&root_json).unwrap();
        let (peer, _) = store::open(&peer_db, true).unwrap();
        let t = Instant::now();
        let imported = {
            let mut stage =
                content_store::Stage::new(&mut c, &db, limits, format!("stg-replica-{mode}")).unwrap();
            content_store::import_peer_refs(&peer, std::slice::from_ref(&root), &mut stage, &mut job)
        };
        report["peer_import"] = json!({"wall_ms":ms(t),"error":imported.err().map(|e| e.to_string()),
            "job":snapshot(&job)});
        report["carrier_after_peer_import"] = facts(&c, &db);
    }
    let t = Instant::now();
    let read = {
        let mut stage =
            content_store::Stage::new(&mut c, &db, limits, format!("stg-replica-{mode}")).unwrap();
        read_current_pins(&pins, &[object_id], &mut job, deadline, &mut stage)
    };
    report["read_current_ms"] = json!(ms(t));
    report["job_after_read"] = snapshot(&job);
    report["carrier_after_read"] = facts(&c, &db);
    report["rusage_after_read"] = rusage();
    match read {
        Err(error) => {
            report["read_error"] = json!(error);
        }
        Ok(verified) => {
            let evidence = verified.evidence.clone();
            report["evidence"] = evidence.clone();
            report["selected_objects"] = json!(verified.selected_objects.len());
            let root = ContentRef::from_transport_json(&evidence["basis_ref"]).unwrap();
            report["publish"] = json!(publish(&mut c, &db, std::slice::from_ref(&root), &mut job));
            report["job_after_publish"] = snapshot(&job);
            report["carrier_after_publish"] = facts(&c, &db);
            report["audit_1"] = audit(&c, &db, &structure, &evidence, 1, limits);
            let k: usize = std::env::var("CLOSURE_AUDIT_K")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            if k > 1 {
                report["audit_k"] = audit(&c, &db, &structure, &evidence, k, limits);
            }
            report["carrier_after_audit"] = facts(&c, &db);
        }
    }
    report["wall_ms"] = json!(ms(started));
    report["rusage_end"] = rusage();
    write_report(&report);
}
