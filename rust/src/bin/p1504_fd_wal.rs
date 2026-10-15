//! #1504 FD/WAL 组件问答原型。只有显式 suite 子命令；没有生产 admission 接线。
//! 具名夹具的创建、复制、换位和删除都属于驱动器；私有 VFS 只读固定 main/WAL。
//! 本机禁止运行；只在独评后的精确远端候选执行。
#![deny(unsafe_op_in_unsafe_fn)]

#[path = "p1504_d1/vfs.rs"]
mod vfs;

use rusqlite::{config::DbConfig, ffi, params, Connection, OpenFlags};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Instant;
use vfs::{Fault, Registration, Result};

const FIXTURE_ROWS: i64 = 48;
const PAYLOAD_BYTES: usize = 2048;
const SQL_ROWS: &str = "SELECT id,label,hex(payload) FROM records ORDER BY id";
const SQL_META: &str = "SELECT marker FROM meta ORDER BY marker";

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Row {
    id: i64,
    label: String,
    payload_hex: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
struct Data {
    records: Vec<Row>,
    markers: Vec<String>,
    quick_check: String,
}

fn ensure(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.to_string().into())
    }
}
fn sidecar(main: &Path, suffix: &str) -> PathBuf {
    let mut name = main.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
fn query(c: &Connection) -> rusqlite::Result<Data> {
    let records = c
        .prepare(SQL_ROWS)?
        .query_map([], |r| {
            Ok(Row {
                id: r.get(0)?,
                label: r.get(1)?,
                payload_hex: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let markers = c
        .prepare(SQL_META)?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    let quick_check = c.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    Ok(Data {
        records,
        markers,
        quick_check,
    })
}
fn outcome(data: &rusqlite::Result<Data>) -> Result<Value> {
    match data {
        Ok(d) => Ok(json!({"ok":true,"rows":d.records.len(),"markers":d.markers,
            "quick_check":d.quick_check,"sha256":hex_hash(&serde_json::to_vec(d)?)})),
        Err(e) => Ok(json!({"ok":false,"error":e.to_string(),
            "extended_code":e.sqlite_error().map(|v|v.extended_code)})),
    }
}
fn hex_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn file_hash(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0_u8; 16384];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
// 文件存在性+全部内容+长度+inode；不要求 atime 不变（真实只读可能改变 atime）。
fn tree_snapshot(root: &Path) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Value>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let m = fs::symlink_metadata(&path)?;
            let relative = path.strip_prefix(root)?.to_string_lossy().into_owned();
            if m.file_type().is_symlink() {
                return Err("unexpected fixture symlink".into());
            }
            if m.is_dir() {
                walk(root, &path, out)?;
            } else {
                ensure(m.is_file(), "unexpected fixture special file")?;
                out.insert(
                    relative,
                    json!({"len":m.len(),"dev":m.dev(),"ino":m.ino(),
                    "sha256":file_hash(&path)?}),
                );
            }
        }
        Ok(())
    }
    walk(root, root, &mut out)?;
    Ok(out)
}
fn make_private_dir(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn clone_pair(from: &Path, to: &Path) -> Result<()> {
    make_private_dir(to)?;
    for filename in ["main.sqlite", "main.sqlite-wal"] {
        let source = from.join(filename);
        if source.exists() {
            fs::copy(source, to.join(filename))?;
        }
    }
    Ok(())
}

/// 由 bundled SQLite 生成页、事务、校验和。捕获时同线程 writer 已返回，无并发变更。
fn generate_fixture(work: &Path, seed: &Path, uncommitted_tail: bool) -> Result<Value> {
    make_private_dir(work)?;
    make_private_dir(seed)?;
    let main = work.join("main.sqlite");
    let mut c = Connection::open(&main)?;
    ensure(
        c.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?,
        "no checkpoint config",
    )?;
    c.execute_batch(
        "PRAGMA page_size=1024; PRAGMA journal_mode=WAL;
        PRAGMA synchronous=FULL; PRAGMA wal_autocheckpoint=0;
        CREATE TABLE records(id INTEGER PRIMARY KEY,label TEXT NOT NULL,payload BLOB NOT NULL);
        CREATE TABLE meta(marker TEXT NOT NULL); INSERT INTO meta VALUES('main-baseline');
        PRAGMA wal_checkpoint(TRUNCATE);",
    )?;
    let main_before_commits = file_hash(&main)?;
    for batch in 0..3_i64 {
        let tx = c.transaction()?;
        for j in 0..16_i64 {
            let id = batch * 16 + j;
            let payload: Vec<u8> = (0..PAYLOAD_BYTES)
                .map(|p| ((p as i64 + 17 * id) % 251) as u8)
                .collect();
            tx.execute(
                "INSERT INTO records(id,label,payload) VALUES(?1,?2,?3)",
                params![id, format!("committed-{id:03}"), payload],
            )?;
        }
        tx.execute("UPDATE meta SET marker=?1", [format!("wal-commit-{batch}")])?;
        tx.commit()?;
    }
    let committed = query(&c)?;
    ensure(
        committed.records.len() == FIXTURE_ROWS as usize && committed.quick_check == "ok",
        "fixture committed rows",
    )?;
    ensure(
        file_hash(&main)? == main_before_commits,
        "fixture main unexpectedly checkpointed",
    )?;
    let wal = sidecar(&main, "-wal");
    let committed_wal_bytes = fs::metadata(&wal)?.len();
    ensure(
        committed_wal_bytes > 32 + 2 * 1024,
        "fixture WAL is not nonempty multipage",
    )?;
    if uncommitted_tail {
        c.execute_batch("PRAGMA cache_size=2; PRAGMA cache_spill=ON; BEGIN IMMEDIATE;")?;
        for id in 100..196_i64 {
            c.execute(
                "INSERT INTO records(id,label,payload) VALUES(?1,'uncommitted',?2)",
                params![id, vec![0x7B_u8; 4096]],
            )?;
        }
        ensure(
            fs::metadata(&wal)?.len() > committed_wal_bytes,
            "uncommitted tail did not spill",
        )?;
    }
    // 捕获独立 bytes，SHM 不复制。它们之后是只读源；默认 VFS oracle 使用另一份。
    fs::copy(&main, seed.join("main.sqlite"))?;
    fs::copy(&wal, seed.join("main.sqlite-wal"))?;
    let captured_wal_bytes = fs::metadata(seed.join("main.sqlite-wal"))?.len();
    if uncommitted_tail {
        c.execute_batch("ROLLBACK;")?;
    }
    c.close().map_err(|(_, e)| e)?;
    Ok(
        json!({"rows":FIXTURE_ROWS,"page_size":1024,"payload_bytes":PAYLOAD_BYTES,
        "committed_wal_bytes":committed_wal_bytes,"captured_wal_bytes":captured_wal_bytes,
        "has_uncommitted_tail":uncommitted_tail,"main_not_checkpointed":true,
        "committed_data_sha256":hex_hash(&serde_json::to_vec(&committed)?),
        "missing_shm":!seed.join("main.sqlite-shm").exists()}),
    )
}

fn resource_observation() -> Result<Value> {
    // 独立单进程原型全局读数，SQLite total heap 包含 WAL index/cache，不冒称逐项归因。
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    ensure(
        unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } == 0,
        "getrusage",
    )?;
    let rss_bytes = if cfg!(target_os = "macos") {
        usage.ru_maxrss as u64
    } else {
        (usage.ru_maxrss as u64).saturating_mul(1024)
    };
    let mut current: i64 = 0;
    let mut highwater: i64 = 0;
    let rc = unsafe {
        ffi::sqlite3_status64(
            ffi::SQLITE_STATUS_MEMORY_USED,
            &mut current,
            &mut highwater,
            0,
        )
    };
    ensure(rc == ffi::SQLITE_OK, "sqlite status")?;
    Ok(
        json!({"process_peak_rss_bytes":rss_bytes,"sqlite_heap_current_bytes":current,
        "sqlite_heap_highwater_bytes":highwater,"wal_index_bytes":"included in SQLite heap, not separately attributed",
        "not_a_production_heap_cap":true}),
    )
}

fn run_case(root: &Path, seed: &Path, name: &str, mutation: &str) -> Result<Value> {
    let case = root.join(name);
    make_private_dir(&case)?;
    let source = case.join("source");
    clone_pair(seed, &source)?;
    let source_wal = source.join("main.sqlite-wal");
    match mutation {
        "flip-last-byte" => {
            let mut f = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&source_wal)?;
            f.seek(SeekFrom::End(-1))?;
            let mut b = [0];
            f.read_exact(&mut b)?;
            f.seek(SeekFrom::End(-1))?;
            f.write_all(&[b[0] ^ 0x80])?;
        }
        "truncate-last-7" => {
            let f = OpenOptions::new().write(true).open(&source_wal)?;
            f.set_len(
                f.metadata()?
                    .len()
                    .checked_sub(7)
                    .ok_or("short mutation input")?,
            )?;
        }
        "none" => (),
        _ => return Err("unknown fixture mutation".into()),
    }
    let oracle_dir = case.join("oracle");
    clone_pair(&source, &oracle_dir)?;
    let oracle_preimage = tree_snapshot(&oracle_dir)?;
    let source_preimage = tree_snapshot(&source)?;
    for filename in ["main.sqlite", "main.sqlite-wal"] {
        let a = &source_preimage[filename];
        let b = &oracle_preimage[filename];
        ensure(
            a["sha256"] == b["sha256"]
                && a["len"] == b["len"]
                && (a["ino"] != b["ino"] || a["dev"] != b["dev"]),
            "oracle not an independent exact copy",
        )?;
    }
    let oracle = Connection::open_with_flags(
        oracle_dir.join("main.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    oracle.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
    let expected = query(&oracle);
    oracle.close().map_err(|(_, e)| e)?;
    // 默认 oracle 单独副本允许 SQLite 正常 SHM；不与 source 共享 inode。
    let source_before = tree_snapshot(&source)?;
    let v = Registration::new(&source.join("main.sqlite"), Some(&source_wal))?;
    let before_resources = resource_observation()?;
    let session = v.connect(true)?;
    let actual = query(session.connection());
    let during_resources = resource_observation()?;
    let actual_summary = outcome(&actual)?;
    let expected_summary = outcome(&expected)?;
    let exact_match = match (&actual, &expected) {
        (Ok(a), Ok(e)) => a == e,
        (Err(a), Err(e)) => {
            a.sqlite_error().map(|v| v.extended_code) == e.sqlite_error().map(|v| v.extended_code)
        }
        _ => false,
    };
    ensure(
        exact_match,
        &format!("{name}: private VFS/oracle mismatch {actual_summary} / {expected_summary}"),
    )?;
    // 真实非空正控不允许双失败冒充通过；所有三类 SQLite 生成尾输入亦必须可查询。
    ensure(
        actual.is_ok(),
        &format!("{name}: required successful query"),
    )?;
    if name == "committed" || name == "uncommitted-tail" {
        let d = actual.as_ref().map_err(|_| "positive query")?;
        ensure(
            d.records.len() == FIXTURE_ROWS as usize
                && d.markers == ["wal-commit-2"]
                && d.quick_check == "ok",
            "required WAL-only content missing",
        )?;
    }
    session.close()?;
    let callback_report = v.finish()?;
    ensure(
        source_before == tree_snapshot(&source)?,
        "source changed during private read",
    )?;
    assert_read_trace(&callback_report)?;
    Ok(json!({"name":name,"status":"PASS","mutation":mutation,
        "private":actual_summary,"oracle":expected_summary,"full_rows_equal":exact_match,
        "source_before_after_equal":true,"source_snapshot":source_before,
        "oracle_preimage":oracle_preimage,"independent_byte_equal_oracle_input":true,
        "resources_before":before_resources,"resources_during":during_resources,
        "resources_after":resource_observation()?,"vfs":callback_report}))
}

fn assert_read_trace(report: &Value) -> Result<()> {
    ensure(
        report["unregistered"] == true
            && report["source_fds_closed"] == true
            && report["report"]["active_files"] == 0
            && report["report"]["source_unchanged"] == true
            && report["report"]["trace_overflow"] == false
            && report["report"]["close_errors"] == 0
            && report["report"]["caught_panics"] == 0,
        "VFS lifecycle/identity failure",
    )?;
    let events = report["report"]["events"].as_array().ok_or("no trace")?;
    let arm = events
        .iter()
        .position(|e| e["op"] == "exclusive-armed")
        .ok_or("not armed")?;
    let wal_open = events
        .iter()
        .position(|e| e["op"] == "open" && e["kind"] == 2)
        .ok_or("no real WAL open")?;
    ensure(arm < wal_open, "WAL accessed before exclusive")?;
    ensure(
        events
            .iter()
            .any(|e| e["op"] == "read" && e["kind"] == 2 && e["rc"] == 0),
        "no WAL reads",
    )?;
    for e in events {
        ensure(
            !matches!(
                e["op"].as_str(),
                Some("write-denied" | "truncate-denied" | "delete-denied")
            ),
            "positive query attempted write/delete",
        )?;
        if e["op"] == "read" && e["exclusive_armed"] == false {
            ensure(
                e["kind"] == 1 && e["a"] == 0 && e["b"] == 100,
                "non-header read before exclusive mode",
            )?;
        }
    }
    Ok(())
}

fn path_swap_case(root: &Path, seed: &Path) -> Result<Value> {
    let case = root.join("pathname-swap");
    make_private_dir(&case)?;
    let source = case.join("source");
    clone_pair(seed, &source)?;
    let main = source.join("main.sqlite");
    let wal = sidecar(&main, "-wal");
    let v = Registration::new(&main, Some(&wal))?;
    // 固定 main/WAL 后替换祖先名；原件仍有一个 link，持有的 File 不经过路径再打开。
    let pinned = case.join("pinned-old-source");
    fs::rename(&source, &pinned)?;
    make_private_dir(&source)?;
    let sentinel = Connection::open(&main)?;
    sentinel.execute_batch(
        "CREATE TABLE records(id INTEGER PRIMARY KEY,label TEXT,payload BLOB);
        CREATE TABLE meta(marker TEXT); INSERT INTO meta VALUES('sentinel-do-not-read');
        INSERT INTO records VALUES(999,'sentinel',X'00');",
    )?;
    sentinel.close().map_err(|(_, e)| e)?;
    let before = tree_snapshot(&case)?;
    let s = v.connect(true)?;
    let actual = query(s.connection())?;
    s.close()?;
    ensure(
        actual.records.len() == FIXTURE_ROWS as usize && actual.markers == ["wal-commit-2"],
        "read sentinel after pin",
    )?;
    let report = v.finish()?;
    assert_read_trace(&report)?;
    ensure(before == tree_snapshot(&case)?, "original/sentinel changed")?;
    Ok(
        json!({"name":"pathname-swap","status":"PASS","source_and_sentinel_unchanged":true,
        "read_old_pinned_rows":actual.records.len(),"snapshot":before,"vfs":report,
        "does_not_close_production_rw_reopen_window":true}),
    )
}

fn denied_case(root: &Path, seed: &Path) -> Result<Value> {
    let case = root.join("denials");
    make_private_dir(&case)?;
    let source = case.join("source");
    clone_pair(seed, &source)?;
    let main = source.join("main.sqlite");
    let wal = sidecar(&main, "-wal");
    let outside = case.join("outside-sentinel");
    fs::write(&outside, b"must remain unchanged")?;
    let before = tree_snapshot(&case)?;
    let mut v = Registration::new(&main, Some(&wal))?;
    let callback_checks = v.callback_contract(&outside)?;
    let callback_report = v.finish()?;
    ensure(
        callback_report["report"]["caught_panics"] == 1
            && callback_report["report"]["close_errors"] == 1
            && callback_report["report"]["active_files"] == 0,
        "injected callback accounting",
    )?;

    let v = Registration::new(&main, Some(&wal))?;
    let s = v.connect(true)?;
    query(s.connection())?;
    let mut sql_denials = Vec::new();
    for sql in [
        "ATTACH DATABASE '/outside/p1504-never-create' AS escaped",
        "ATTACH DATABASE ':memory:' AS escaped",
        "CREATE TEMP TABLE escaped(x)",
        "INSERT INTO records VALUES(500,'denied',X'00')",
        "PRAGMA query_only=OFF",
        "PRAGMA locking_mode=NORMAL",
        "SELECT load_extension('/outside/p1504-extension')",
    ] {
        let denial_count = v.sql_denials();
        let error = s
            .connection()
            .execute_batch(sql)
            .err()
            .ok_or("forbidden SQL accepted")?;
        ensure(
            v.sql_denials() > denial_count,
            "SQL failed without reaching explicit authorizer denial",
        )?;
        sql_denials.push(json!({"sql":sql,"error":error.to_string()}));
    }
    ensure(v.connect(true).is_err(), "second connection accepted")?;
    s.close()?;
    let sql_report = v.finish()?;

    let v = Registration::new(&main, Some(&wal))?;
    let s = v.connect(false)?;
    let late = query(s.connection());
    ensure(late.is_err(), "late exclusive unexpectedly read WAL")?;
    let late_summary = outcome(&late)?;
    s.close()?;
    let late_report = v.finish()?;

    let mut read_faults = Vec::new();
    for fault in [Fault::ReadIo, Fault::ReadShort, Fault::ReadPanic] {
        let v = Registration::new(&main, Some(&wal))?;
        let s = v.connect(true)?;
        v.inject(fault);
        let bad = query(s.connection());
        ensure(bad.is_err(), "read fault accepted")?;
        let summary = outcome(&bad)?;
        s.close()?;
        read_faults.push(json!({"fault":fault,"query":summary,"vfs":v.finish()?}));
    }
    let v = Registration::new(&main, Some(&wal))?;
    v.inject(Fault::OpenNoMem);
    ensure(
        v.connect(true).is_err(),
        "SQLite connection accepted xOpen NOMEM",
    )?;
    let sqlite_open_error_report = v.finish()?;
    ensure(
        sqlite_open_error_report["report"]["active_files"] == 0,
        "NOMEM left active files",
    )?;
    let v = Registration::new(&main, Some(&wal))?;
    let s = v.connect(true)?;
    query(s.connection())?;
    v.inject(Fault::CloseIo);
    ensure(
        s.close().is_err(),
        "close callback error accepted as success",
    )?;
    let sqlite_close_error_report = v.finish()?;
    ensure(
        sqlite_close_error_report["report"]["close_errors"] == 1
            && sqlite_close_error_report["report"]["active_files"] == 0,
        "close error lifecycle",
    )?;
    ensure(
        before == tree_snapshot(&case)?,
        "negative controls changed source/outside files",
    )?;
    // journal 存在必须具名拒绝，不让 VFS 隐藏。此处只证明输入域门，不制造 hot journal 结论。
    let journal = sidecar(&main, "-journal");
    fs::write(&journal, b"unsupported journal object")?;
    let journal_rejected = Registration::new(&main, Some(&wal)).is_err();
    fs::remove_file(&journal)?;
    ensure(journal_rejected, "existing journal hidden")?;
    Ok(
        json!({"name":"denials","status":"PASS","callback_checks":callback_checks,
        "callback_report":callback_report,"sql_denials":sql_denials,"sql_report":sql_report,
        "late_exclusive":late_summary,"late_exclusive_report":late_report,
        "read_faults":read_faults,"journal_presence_rejected":journal_rejected,
        "sqlite_open_error_report":sqlite_open_error_report,"sqlite_close_error_report":sqlite_close_error_report,
        "source_outside_unchanged":true,"snapshot":before}),
    )
}

fn run_suite(output: &Path) -> Result<Value> {
    // 只接受调用者提供的绝对新目录，拒绝覆盖既有目录或带 symlink 的父目录。
    ensure(output.is_absolute(), "output must be absolute")?;
    let parent = output.parent().ok_or("output parent")?;
    ensure(
        parent.canonicalize()? == parent,
        "output parent must already be canonical without symlinks",
    )?;
    ensure(!output.exists(), "output already exists")?;
    make_private_dir(output)?;
    let started = Instant::now();
    let source_id = unsafe { CStr::from_ptr(ffi::sqlite3_sourceid()) }.to_str()?;
    ensure(
        source_id == vfs::EXPECTED_SOURCE_ID,
        "unexpected bundled SQLite source id",
    )?;
    ensure(
        unsafe { ffi::sqlite3_libversion_number() } == 3_053_002,
        "unexpected SQLite version",
    )?;
    let default_vfs = unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) };
    ensure(!default_vfs.is_null(), "default VFS missing")?;
    let default_name = unsafe { CStr::from_ptr((*default_vfs).zName) }
        .to_str()?
        .to_string();
    let work = output.join("fixture-work");
    make_private_dir(&work)?;
    let seed = work.join("committed-seed");
    let fixture = generate_fixture(&work.join("generator"), &seed, false)?;
    let uncommitted = work.join("uncommitted-seed");
    let uncommitted_fixture =
        generate_fixture(&work.join("uncommitted-generator"), &uncommitted, true)?;
    let cases_dir = output.join("cases");
    make_private_dir(&cases_dir)?;
    let mut cases = Vec::new();
    for (name, input, mutation) in [
        ("committed", seed.as_path(), "none"),
        ("uncommitted-tail", uncommitted.as_path(), "none"),
        ("bad-tail", seed.as_path(), "flip-last-byte"),
        ("truncated-tail", seed.as_path(), "truncate-last-7"),
    ] {
        let result = run_case(&cases_dir, input, name, mutation)?;
        fs::write(
            output.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&result)?,
        )?;
        cases.push(json!({"name":name,"status":"PASS"}));
    }
    for name in ["pathname-swap", "denials"] {
        let result = if name == "pathname-swap" {
            path_swap_case(&cases_dir, &seed)?
        } else {
            denied_case(&cases_dir, &seed)?
        };
        fs::write(
            output.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&result)?,
        )?;
        cases.push(json!({"name":name,"status":"PASS"}));
    }
    ensure(
        unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) } == default_vfs,
        "default VFS changed",
    )?;
    let resources = resource_observation()?;
    // 所有连接已显式关闭、注册已注销后，驱动只删除自己 create_dir 成功的私有工作目录。
    let final_fixture_snapshot = tree_snapshot(&cases_dir)?;
    fs::remove_dir_all(&cases_dir)?;
    fs::remove_dir_all(&work)?;
    ensure(
        !cases_dir.exists() && !work.exists(),
        "fixture cleanup incomplete",
    )?;
    Ok(
        json!({"verdict":"PASS_D1_COMPONENT_ONLY","issue":1504,"sqlite_source_id":source_id,
        "sqlite_version":rusqlite::version(),"default_vfs":default_name,"default_unchanged":true,
        "fixture":fixture,"uncommitted_fixture":uncommitted_fixture,"cases":cases,
        "resources":resources,"elapsed_seconds":started.elapsed().as_secs_f64(),
        "cleanup":{"fixtures_removed":true,"registrations_removed":true},
        "final_fixture_snapshot":final_fixture_snapshot,
        "outstanding":["D2 hot journal","formal producer/R4/admission","full S store baseline",
            "synchronous IO hard deadline","production RW pathname continuity","49GB history",
            "64GiB/store","148GiB dual-run start","full #1323/#1340 acceptance"],
        "production_v4":"BLOCK_NOT_MERGEABLE"}),
    )
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 || args[1] != "suite" {
        eprintln!("usage: p1504_fd_wal suite /absolute/new/evidence-directory");
        std::process::exit(2);
    }
    let output = PathBuf::from(&args[2]);
    match run_suite(&output) {
        Ok(result) => {
            let written = serde_json::to_vec_pretty(&result)
                .map_err(|e| e.to_string())
                .and_then(|b| fs::write(output.join("RESULT.json"), b).map_err(|e| e.to_string()));
            if let Err(e) = written {
                eprintln!("report write failed: {e}");
                std::process::exit(1);
            }
            println!("PASS_D1_COMPONENT_ONLY {}", output.display());
        }
        Err(e) => {
            // 失败不删除夹具，保留可查现场；外层 CI 必须具名回收且上传，不能用 PASS 代替 exit0。
            eprintln!("D1 FAILED (fixtures retained if created): {e}");
            std::process::exit(1);
        }
    }
}
