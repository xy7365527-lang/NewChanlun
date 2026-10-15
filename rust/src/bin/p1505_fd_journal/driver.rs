//! #1505：只在具名远端窗口运行的夹具/独立oracle/子进程监督。suite父进程不初始化SQLite。
#![deny(unsafe_op_in_unsafe_fn)]
use super::{
    allocator,
    overlay::{Overlay, CHUNK, JOURNAL, MAIN},
    vfs::{self, Fault, Registration, Result},
    Limits,
};
use rusqlite::{ffi, params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;
const CURRENT: &str = "p1505-current-rule"; // 原型规则身份，不伪装正式RULE_REVISION。
const MAGIC: [u8; 8] = [0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Row {
    id: i64,
    label: String,
    payload: String,
    payload_type: String,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Data {
    rows: Vec<Row>,
    rule: String,
    schema: Vec<(String, String, String)>,
    quick_check: String,
    page_count: i64,
    page_size: i64,
}
fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(msg.to_string().into())
    }
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn load(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn hash(bytes: &[u8]) -> String {
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
fn private(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn journal(main: &Path) -> PathBuf {
    let mut x = main.as_os_str().to_os_string();
    x.push("-journal");
    PathBuf::from(x)
}
fn snapshot(path: &Path) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for e in fs::read_dir(path)? {
        let p = e?.path();
        let m = fs::symlink_metadata(&p)?;
        ensure(
            m.is_file() && !m.file_type().is_symlink(),
            "source contains non-regular object",
        )?;
        out.insert(p.file_name().ok_or("filename")?.to_string_lossy().into_owned(),json!({"len":m.len(),"dev":m.dev(),"ino":m.ino(),"nlink":m.nlink(),"sha256":file_hash(&p)?}));
    }
    Ok(out)
}
fn copy_source(from: &Path, to: &Path) -> Result<Value> {
    private(to)?;
    let mut copies = Vec::new();
    for name in ["main.sqlite", "main.sqlite-journal"] {
        let src = from.join(name);
        if src.exists() {
            let dst = to.join(name);
            fs::copy(&src, &dst)?;
            let a = fs::metadata(&src)?;
            let b = fs::metadata(&dst)?;
            let h = file_hash(&src)?;
            ensure(
                h == file_hash(&dst)?
                    && a.len() == b.len()
                    && (a.dev() != b.dev() || a.ino() != b.ino()),
                "copy not independent exact bytes",
            )?;
            copies.push(json!({"name":name,"length":a.len(),"sha256":h,"source_dev":a.dev(),"source_ino":a.ino(),"copy_dev":b.dev(),"copy_ino":b.ino()}));
        }
    }
    Ok(json!({"files":copies}))
}
fn query(c: &Connection) -> rusqlite::Result<Data> {
    let rows = c
        .prepare("SELECT id,label,hex(payload),typeof(payload) FROM records ORDER BY id")?
        .query_map([], |r| {
            Ok(Row {
                id: r.get(0)?,
                label: r.get(1)?,
                payload: r.get(2)?,
                payload_type: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let rule = c.query_row(
        "SELECT value FROM meta WHERE key='rule_revision'",
        [],
        |r| r.get(0),
    )?;
    let schema = c
        .prepare(
            "SELECT type,name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY type,name",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Data {
        rows,
        rule,
        schema,
        quick_check: c.query_row("PRAGMA quick_check", [], |r| r.get(0))?,
        page_count: c.query_row("PRAGMA page_count", [], |r| r.get(0))?,
        page_size: c.query_row("PRAGMA page_size", [], |r| r.get(0))?,
    })
}
fn source_id() -> Result<String> {
    let id = unsafe { CStr::from_ptr(ffi::sqlite3_sourceid()) }
        .to_str()?
        .to_owned();
    ensure(id == vfs::EXPECTED_SOURCE_ID, "SQLite source-id drift")?;
    Ok(id)
}
fn rss() -> Result<u64> {
    let mut r: libc::rusage = unsafe { std::mem::zeroed() };
    ensure(
        unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut r) } == 0,
        "getrusage",
    )?;
    let n = u64::try_from(r.ru_maxrss)?;
    Ok(if cfg!(target_os = "macos") {
        n
    } else {
        n.checked_mul(1024).ok_or("RSS overflow")?
    })
}
fn now_ns() -> Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    ensure(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut t) } == 0,
        "clock",
    )?;
    Ok(u64::try_from(t.tv_sec)?
        .checked_mul(1_000_000_000)
        .and_then(|x| x.checked_add(t.tv_nsec as u64))
        .ok_or("clock overflow")?)
}
fn common_deadline() -> Result<u64> {
    Ok(std::env::var("P1505_DEADLINE_NS")?.parse()?)
}
fn deadline() -> Result<u64> {
    let suite: u64 = std::env::var("P1505_SUITE_DEADLINE_NS")?.parse()?;
    ensure(
        suite <= common_deadline()?,
        "suite deadline exceeds shared deadline",
    )?;
    Ok(suite)
}
fn check_deadline(end: u64) -> Result<()> {
    ensure(now_ns()? < end, "shared absolute deadline exceeded")
}
fn child_root(path: &Path) -> Result<()> {
    let root = PathBuf::from(std::env::var_os("P1505_ROOT").ok_or("child only: missing root")?);
    let canonical = path.canonicalize()?;
    ensure(
        canonical.starts_with(root.canonicalize()?) && canonical != root,
        "child path outside private root",
    )?;
    check_deadline(deadline()?)
}
struct OwnedChild {
    child: Child,
    reaped: bool,
    end: u64,
}
impl OwnedChild {
    fn wait(&mut self, end: u64) -> Result<ExitStatus> {
        loop {
            if let Some(s) = self.child.try_wait()? {
                self.reaped = true;
                return Ok(s);
            }
            check_deadline(end)?;
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn kill_and_reap(&mut self) -> Result<ExitStatus> {
        self.child.kill()?;
        self.wait(self.end)
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        let _ = self.child.kill();
        while now_ns().is_ok_and(|n| n < self.end) {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    self.reaped = true;
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => break,
            }
        }
        if !self.reaped {
            eprintln!(
                "D2 UNREAPED child pid={} remains in outer supervised process group",
                self.child.id()
            );
        }
    }
}

fn command(root: &Path, end: u64, mode: &str) -> Result<Command> {
    let mut c = Command::new(std::env::current_exe()?);
    c.arg(mode)
        .env("P1505_ROOT", root)
        .env("P1505_DEADLINE_NS", common_deadline()?.to_string())
        .env("P1505_SUITE_DEADLINE_NS", end.to_string());
    Ok(c)
}
fn run(root: &Path, end: u64, mode: &str, args: &[&std::ffi::OsStr], log: &Path) -> Result<Value> {
    check_deadline(end)?;
    let mut c = command(root, end, mode)?;
    c.args(args)
        .stdin(Stdio::null())
        .stdout(File::create(log.with_extension("stdout"))?)
        .stderr(File::create(log.with_extension("stderr"))?);
    let start = now_ns()?;
    let mut child = OwnedChild {
        child: c.spawn()?,
        reaped: false,
        end,
    };
    let pid = child.child.id();
    let pgid = unsafe { libc::getpgid(pid as i32) };
    let status = match child.wait(end) {
        Ok(status) => status,
        Err(e) => {
            // 失败也保留监督回执；Drop 仍仅在原截止内回收，不另起等待窗口。
            save(
                &log.with_extension("process.json"),
                &json!({"mode":mode,"pid":pid,"pgid":pgid,
                "success":false,"reaped":child.reaped,"started_ns":start,"ended_ns":now_ns()?,"wait_error":e.to_string()}),
            )?;
            return Err(e);
        }
    };
    let receipt = json!({"mode":mode,"pid":pid,"pgid":pgid,"exit_code":status.code(),"signal":status.signal(),"success":status.success(),"reaped":child.reaped,"started_ns":start,"ended_ns":now_ns()?});
    save(&log.with_extension("process.json"), &receipt)?;
    ensure(status.success(), &format!("child {mode} failed"))?;
    Ok(receipt)
}
fn wait_ready(child: &mut OwnedChild, end: u64, raw_output: &Path) -> Result<Value> {
    let mut raw_log = File::create(raw_output)?;
    let stdout = child.child.stdout.as_mut().ok_or("ready stdout")?;
    let fd = stdout.as_raw_fd();
    let old = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    ensure(
        old >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, old | libc::O_NONBLOCK) } == 0,
        "nonblocking ready",
    )?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(65536)?;
    loop {
        check_deadline(end)?;
        let mut buf = [0_u8; 1024];
        match stdout.read(&mut buf) {
            Ok(0) => return Err("child exited before ready".into()),
            Ok(n) => {
                raw_log.write_all(&buf[..n])?;
                ensure(bytes.len() + n <= 65536, "ready overflow")?;
                bytes.extend_from_slice(&buf[..n]);
                if bytes.contains(&b'\n') {
                    return Ok(serde_json::from_slice(&bytes)?);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e.into()),
        }
    }
}
fn kill_ready(
    root: &Path,
    end: u64,
    mode: &str,
    args: &[&std::ffi::OsStr],
    log: &Path,
) -> Result<Value> {
    check_deadline(end)?;
    let mut c = command(root, end, mode)?;
    c.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(File::create(log.with_extension("stderr"))?);
    let mut child = OwnedChild {
        child: c.spawn()?,
        reaped: false,
        end,
    };
    let pid = child.child.id();
    let pgid = unsafe { libc::getpgid(pid as i32) };
    let ready = match wait_ready(&mut child, end, &log.with_extension("stdout")) {
        Ok(ready) => ready,
        Err(e) => {
            save(
                &log.with_extension("kill.json"),
                &json!({"mode":mode,"pid":pid,"pgid":pgid,
                "ready":false,"success":false,"reaped":child.reaped,"ready_error":e.to_string(),"ended_ns":now_ns()?}),
            )?;
            return Err(e);
        }
    };
    ensure(ready["ready"] == true, "invalid READY handshake")?;
    let prekill_observation = if mode == "writer" {
        let work = Path::new(args.first().ok_or("writer work argument")?);
        let main = work.join("main.sqlite");
        let observed_hash = file_hash(&main)?;
        let observed_len = fs::metadata(&main)?.len();
        let observed_journal_len = fs::metadata(journal(&main))?.len();
        ensure(
            ready["uncommitted"] == true
                && ready["main_uncommitted_hash"] == observed_hash
                && ready["main_before_hash"] != observed_hash
                && ready["main_uncommitted_len"] == observed_len
                && ready["journal_len"] == observed_journal_len
                && observed_journal_len > 512,
            "parent did not confirm live spilled source",
        )?;
        json!({"main_sha256":observed_hash,"main_len":observed_len,"journal_len":observed_journal_len})
    } else {
        ensure(
            ready["query_ok"] == true && ready["overlay_write_calls"].as_u64().unwrap_or(0) > 0,
            "reader not successfully recovered before kill",
        )?;
        json!({"reader_recovery_observed":true})
    };
    ensure(
        child.child.try_wait()?.is_none(),
        "ready child already exited",
    )?;
    let sent_at = now_ns()?;
    check_deadline(end)?;
    let status = match child.kill_and_reap() {
        Ok(status) => status,
        Err(e) => {
            save(
                &log.with_extension("kill.json"),
                &json!({"mode":mode,"pid":pid,"pgid":pgid,
                "ready":ready,"success":false,"reaped":child.reaped,"sent_ns":sent_at,"kill_or_wait_error":e.to_string(),"ended_ns":now_ns()?}),
            )?;
            return Err(e);
        }
    };
    ensure(
        status.signal() == Some(libc::SIGKILL) && status.code().is_none(),
        "not actual SIGKILL",
    )?;
    let receipt = json!({"mode":mode,"pid":pid,"pgid":pgid,"ready":ready,"prekill_observation":prekill_observation,"signal_sent":libc::SIGKILL,"wait_signal":status.signal(),"wait_exit_code":status.code(),"reaped":true,"sent_ns":sent_at,"reaped_ns":now_ns()?});
    save(&log.with_extension("kill.json"), &receipt)?;
    Ok(receipt)
}
fn announce_and_hold(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string(value)?);
    std::io::stdout().flush()?;
    let mut x = [0_u8; 1];
    let _ = std::io::stdin().read(&mut x)?;
    Err("parent did not SIGKILL held child".into())
}

pub fn writer(work: &Path, rule: &str, mode: &str) -> Result<()> {
    child_root(work)?;
    ensure(
        [CURRENT, "old-rule", "unknown-rule"].contains(&rule),
        "test rule",
    )?;
    ensure(["hot", "clean", "cold"].contains(&mode), "writer mode")?;
    let main = work.join("main.sqlite");
    ensure(!main.exists(), "writer refuses existing DB")?;
    let mut c = Connection::open(&main)?;
    let id = source_id()?;
    c.execute_batch("PRAGMA page_size=1024; PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA cache_size=5; PRAGMA cache_spill=ON;
        CREATE TABLE records(id INTEGER PRIMARY KEY,label TEXT NOT NULL,payload BLOB NOT NULL);
        CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);")?;
    if mode == "cold" {
        c.execute_batch("PRAGMA journal_mode=PERSIST;")?;
    }
    let tx = c.transaction()?;
    tx.execute("INSERT INTO meta VALUES('rule_revision',?1)", [rule])?;
    for id in 0..48_i64 {
        let payload: Vec<u8> = (0..8192)
            .map(|n| ((n as i64 + id * 17) % 251) as u8)
            .collect();
        tx.execute(
            "INSERT INTO records VALUES(?1,?2,?3)",
            params![id, format!("committed-{id:03}"), payload],
        )?;
    }
    tx.commit()?;
    let data = query(&c)?;
    ensure(
        data.rows.len() == 48 && data.quick_check == "ok",
        "baseline",
    )?;
    save(&work.join("baseline.json"), &data)?;
    let before = file_hash(&main)?;
    let before_len = fs::metadata(&main)?.len();
    if mode != "hot" {
        c.close().map_err(|(_, e)| e)?;
        if mode == "cold" {
            let bytes = fs::read(journal(&main))?;
            ensure(
                !bytes.is_empty() && bytes[0] == 0,
                "PERSIST cold journal not produced",
            )?;
        }
        save(
            &work.join("writer.json"),
            &json!({"mode":mode,"source_id":id,"main_before_hash":before,"original_main_len":before_len}),
        )?;
        return Ok(());
    }
    c.execute_batch("BEGIN IMMEDIATE;")?;
    for id in 0..48_i64 {
        c.execute(
            "UPDATE records SET payload=?1 WHERE id=?2",
            params![vec![0xB7_u8; 8192], id],
        )?;
    }
    for id in 100..108_i64 {
        c.execute(
            "INSERT INTO records VALUES(?1,'uncommitted',?2)",
            params![id, vec![0xCC_u8; 8192]],
        )?;
    }
    let after = file_hash(&main)?;
    let journal_len = fs::metadata(journal(&main))?.len();
    let current_len = fs::metadata(&main)?.len();
    ensure(
        before != after && journal_len > 512 && current_len > before_len,
        "transaction did not really spill",
    )?;
    // 只数已知真 writer 的对齐 magic 标记作多段生成证据；不解释页、不生成恢复答案。
    let raw_journal = fs::read(journal(&main))?;
    let header_offsets: Vec<_> = raw_journal
        .windows(MAGIC.len())
        .enumerate()
        .filter_map(|(off, bytes)| (off % 512 == 0 && bytes == MAGIC).then_some(off))
        .collect();
    ensure(
        header_offsets.len() > 1,
        "multiple journal headers were not produced",
    )?;
    let ready = json!({"ready":true,"source_id":id,"original_main_len":before_len,"main_before_hash":before,"main_uncommitted_hash":after,"main_uncommitted_len":current_len,"journal_len":journal_len,"uncommitted":true,
        "aligned_header_magic_offsets":header_offsets,"header_observation":"raw markers only; SQLite remains the journal interpreter"});
    save(&work.join("writer.json"), &ready)?;
    announce_and_hold(&ready)
}
pub fn oracle(source: &Path, output: &Path) -> Result<()> {
    child_root(source)?;
    let c = Connection::open_with_flags(
        source.join("main.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    c.busy_timeout(Duration::ZERO)?;
    let id = source_id()?;
    let got = query(&c);
    let mut result = match got {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(e) => {
            json!({"ok":false,"error":e.to_string(),"extended_code":e.sqlite_error().map(|x|x.extended_code)})
        }
    };
    c.close().map_err(|(_, e)| e)?;
    result["source_id"] = json!(id);
    result["main_sha256"] = json!(file_hash(&source.join("main.sqlite"))?);
    result["main_len"] = json!(fs::metadata(source.join("main.sqlite"))?.len());
    result["default_vfs"] = json!(true);
    save(output, &result)
}
fn reader_inner(source: &Path, limits: &Limits, mode: &str) -> Result<Value> {
    let source_before = snapshot(source)?;
    let main = source.join("main.sqlite");
    let journal = journal(&main);
    let jp = journal.exists().then_some(journal.as_path());
    let default_before = unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) };
    let mut reg = Registration::new(&main, jp, limits)?;
    if mode == "callbacks" {
        let checks = reg.callback_contract(&source.join("outside-sentinel"))?;
        let life = reg.finish()?;
        ensure(
            snapshot(source)? == source_before,
            "callback source changed",
        )?;
        return Ok(json!({"ok":true,"callbacks":checks,"lifecycle":life}));
    }
    let swap = if mode == "path-swap" {
        let moved = source.with_extension("pinned");
        fs::rename(source, &moved)?;
        private(source)?;
        fs::write(source.join("main.sqlite"), b"external-sentinel")?;
        Some(moved)
    } else {
        None
    };
    let session = reg.connect(mode != "late-exclusive");
    let mut result = match session {
        Ok(mut session) => {
            match mode {
                "read-io" => reg.inject(Fault::ReadIo),
                "read-short" => reg.inject(Fault::ReadShort),
                "read-panic" => reg.inject(Fault::ReadPanic),
                _ => (),
            }
            if mode == "sql-denials" {
                for sql in [
                    "CREATE TABLE forbidden(x)",
                    "INSERT INTO meta VALUES('x','y')",
                    "ATTACH ':memory:' AS escaped",
                    "VACUUM",
                    "PRAGMA journal_mode=WAL",
                    "PRAGMA locking_mode=NORMAL",
                    "SELECT load_extension('none')",
                ] {
                    let before = reg.sql_denials();
                    ensure(
                        session.connection().execute_batch(sql).is_err()
                            && reg.sql_denials() > before,
                        "SQL denial not reached",
                    )?;
                }
            }
            let got = query(session.connection());
            let mut data = match got {
                Ok(data) => json!({"ok":true,"accepted_rule":data.rule==CURRENT,"data":data}),
                Err(e) => {
                    json!({"ok":false,"error":e.to_string(),"extended_code":e.sqlite_error().map(|e|e.extended_code)})
                }
            };
            if mode == "sql-denials" {
                data["statement_lifecycle"] = session.statement_lifecycle_contract()?;
            }
            if mode == "hold" {
                let report = reg.report()?;
                ensure(
                    report["overlay"]["write_calls"].as_u64().unwrap_or(0) > 0,
                    "no real overlay write before hold",
                )?;
                announce_and_hold(
                    &json!({"ready":true,"overlay_write_calls":report["overlay"]["write_calls"],"query_ok":data["ok"]}),
                )?;
            }
            if mode == "close-error" {
                reg.inject(Fault::CloseIo);
            }
            match session.close() {
                Ok(()) => data,
                Err(e) => json!({"ok":false,"error":e.to_string(),"close_error":true}),
            }
        }
        Err(e) => json!({"ok":false,"error":e.to_string(),"connect_error":true}),
    };
    if result["ok"] == true {
        let bytes = reg.bytes(MAIN)?;
        result["virtual_main_sha256"] = json!(hash(&bytes));
        result["virtual_main_len"] = json!(bytes.len());
    }
    let life = reg.finish()?;
    ensure(
        unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) } == default_before,
        "default VFS changed",
    )?;
    let after = snapshot(swap.as_deref().unwrap_or(source))?;
    ensure(after == source_before, "source bytes changed")?;
    if swap.is_some() {
        ensure(
            fs::read(source.join("main.sqlite"))? == b"external-sentinel",
            "swap sentinel changed",
        )?;
        result["binding_changed"] = json!(true);
        result["accepted_rule"] = json!(false);
    }
    result["lifecycle"] = life;
    result["before_source"] = json!(source_before);
    result["after_source"] = json!(after);
    Ok(result)
}
pub fn reader(source: &Path, output: &Path, config: &Path, mode: &str) -> Result<()> {
    child_root(source)?;
    let original_source = snapshot(source)?;
    let limits = Limits::load(config)?;
    allocator::install(limits.sqlite_heap_limit_bytes).map_err(|e| format!("allocator: {e}"))?;
    let id = source_id()?;
    let allocation_probe = if mode == "allocator-failure" {
        Some(allocator::allocation_contract()?)
    } else {
        None
    };
    if mode == "arena-oom" {
        super::overlay::fail_next_allocation();
    }
    let got = reader_inner(source, &limits, mode);
    let mut result = match got {
        Ok(x) => x,
        Err(e) => json!({"ok":false,"error":e.to_string(),"registration_or_harness_error":true}),
    };
    let final_source_path = if mode == "path-swap" {
        source.with_extension("pinned")
    } else {
        source.to_owned()
    };
    let final_source = snapshot(&final_source_path)?;
    ensure(
        final_source == original_source,
        "reader source changed on success/error path",
    )?;
    result["whole_reader_source_before"] = json!(original_source);
    result["whole_reader_source_after"] = json!(final_source);
    result["vfs_counters"] = vfs::counters();
    result["owned_allocation_ledger"] = super::overlay::allocation_snapshot();
    ensure(
        result["owned_allocation_ledger"]["actual_owned_layout_current_bytes"] == 0,
        "owned allocation leak",
    )?;
    result["allocation_probe"] = json!(allocation_probe);
    result["source_id"] = json!(id);
    result["limits"] = json!(limits);
    result["rss_bytes"] = json!(rss()?);
    result["allocator"] = allocator::finish().map_err(|e| format!("allocator finish: {e}"))?;
    save(output, &result)
}

fn trace_check(value: &Value, require_write: bool) -> Result<()> {
    let life = &value["lifecycle"];
    let report = &life["report"];
    ensure(
        life["unregistered"] == true
            && life["source_fds_closed"] == true
            && report["active_files"] == 0
            && report["source_unchanged"] == true
            && report["trace_overflow"] == false,
        "VFS lifecycle failed",
    )?;
    let events = report["events"].as_array().ok_or("no events")?;
    let mut armed = false;
    let mut writes = 0;
    for e in events {
        let op = e["op"].as_str().ok_or("event op")?;
        if op == "exclusive-armed" {
            armed = true;
        }
        if op == "read" && !armed {
            ensure(
                e["kind"] == MAIN && e["a"] == 0 && e["b"] == 100,
                "schema read before exclusive",
            )?;
        }
        if [
            "write-overlay",
            "truncate-overlay",
            "delete-virtual-journal",
        ]
        .contains(&op)
        {
            ensure(armed, "recovery before exclusive")?;
            if op == "write-overlay" && e["rc"] == 0 {
                writes += 1;
            }
        }
    }
    if require_write {
        ensure(writes > 0, "no actual hot rollback writes")?;
    }
    Ok(())
}
fn compare_case(
    root: &Path,
    end: u64,
    source: &Path,
    base: &Value,
    config: &Path,
    name: &str,
    hot: bool,
) -> Result<Value> {
    let oracle_dir = root.join(format!("{name}-oracle"));
    let copied = copy_source(source, &oracle_dir)?;
    let oracle_out = root.join(format!("{name}-oracle.json"));
    let reader_out = root.join(format!("{name}-reader.json"));
    let before = snapshot(source)?;
    let oracle_process = run(
        root,
        end,
        "oracle",
        &[oracle_dir.as_os_str(), oracle_out.as_os_str()],
        &root.join(format!("{name}-oracle-process")),
    )?;
    let reader_process = run(
        root,
        end,
        "reader",
        &[
            source.as_os_str(),
            reader_out.as_os_str(),
            config.as_os_str(),
            std::ffi::OsStr::new("normal"),
        ],
        &root.join(format!("{name}-reader-process")),
    )?;
    let oracle = load(&oracle_out)?;
    let reader = load(&reader_out)?;
    ensure(
        oracle["ok"] == true && reader["ok"] == true,
        "positive case must succeed both ways",
    )?;
    ensure(
        oracle["data"] == *base && reader["data"] == oracle["data"],
        "full SQL oracle mismatch",
    )?;
    ensure(
        reader["virtual_main_sha256"] == oracle["main_sha256"]
            && reader["virtual_main_len"] == oracle["main_len"],
        "virtual main bytes differ from recovered oracle",
    )?;
    ensure(
        reader["accepted_rule"] == json!(base["rule"] == CURRENT),
        "old/unknown rule gate",
    )?;
    ensure(snapshot(source)? == before, "source changed")?;
    trace_check(&reader, hot)?;
    let result = json!({"name":name,"copy":copied,"oracle_process":oracle_process,"reader_process":reader_process,
        "rows":reader["data"]["rows"].as_array().map(Vec::len),"data_sha256":hash(&serde_json::to_vec(&reader["data"])?),"source_before":before,"source_after":snapshot(source)?,"all_expected":true});
    save(&root.join(format!("{name}-case.json")), &result)?;
    Ok(result)
}
fn byte_contract(root: &Path) -> Result<Value> {
    let base_path = root.join("byte-base");
    let oracle_path = root.join("byte-oracle");
    let initial: Vec<u8> = (0..(3 * CHUNK + 37)).map(|i| (i % 251) as u8).collect();
    fs::write(&base_path, &initial)?;
    fs::write(&oracle_path, &initial)?;
    let base = File::open(&base_path)?;
    let oracle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&oracle_path)?;
    let mut o = Overlay::new(8 * CHUNK, 65536, initial.len() as u64, Some(8))?;
    let mut operations = Vec::new();
    for (op, off, bytes) in [
        ("truncate", 3, Vec::new()),
        ("write", 5, b"XY".to_vec()),
        ("truncate", 8, Vec::new()),
        ("write", 4094, vec![0xA1; 19]),
        ("write", 4097, vec![0xB2; 7]),
        ("truncate", 4100, Vec::new()),
        ("truncate", 9000, Vec::new()),
        ("write", 8190, vec![0xC3; 21]),
        ("truncate", 0, Vec::new()),
        ("truncate", 1024, Vec::new()),
    ] {
        if op == "write" {
            o.write(MAIN, &base, off, &bytes)?;
            oracle.write_all_at(&bytes, off)?;
        } else {
            o.truncate(MAIN, off)?;
            oracle.set_len(off)?;
        }
        let len = o.state(MAIN)?.logical_len as usize;
        let mut actual = vec![0_u8; len];
        let mut expected = vec![0_u8; len];
        ensure(
            !o.read(MAIN, &base, 0, &mut actual)?,
            "model unexpected EOF",
        )?;
        oracle.read_exact_at(&mut expected, 0)?;
        ensure(actual == expected, "overlay/file byte semantics differ")?;
        operations.push(json!({"op":op,"offset_or_size":off,"write_len":bytes.len(),"len":len,"sha256":hash(&actual)}));
    }
    let mut eof = [0xAA_u8; 16];
    ensure(
        o.read(MAIN, &base, 1024, &mut eof)? && eof == [0; 16],
        "EOF zero fill",
    )?;
    ensure(!o.read(MAIN, &base, 0, &mut [])?, "zero length read")?;
    let same = o.view_fingerprint();
    o.write(MAIN, &base, 0, &[])?;
    ensure(
        same == o.view_fingerprint(),
        "zero length write changed view",
    )?;
    let mut one = Overlay::new(CHUNK, 1024, initial.len() as u64, None)?;
    one.write(MAIN, &base, 0, b"A")?;
    let before = one.view_fingerprint();
    ensure(
        one.write(MAIN, &base, 4096, b"B").is_err() && one.view_fingerprint() == before,
        "capacity failure atomicity",
    )?;
    ensure(
        Overlay::new(CHUNK - 1, 1024, 8, None).is_err(),
        "capacity minus one accepted",
    )?;
    let short_path = root.join("byte-short-base");
    fs::write(&short_path, vec![0_u8; CHUNK])?;
    let short = File::open(&short_path)?;
    let mut io = Overlay::new(4 * CHUNK, 2048, (3 * CHUNK) as u64, None)?;
    let before = io.view_fingerprint();
    ensure(
        io.write(MAIN, &short, 0, &vec![1_u8; 2 * CHUNK]).is_err()
            && io.view_fingerprint() == before,
        "pre-read failure changed published view",
    )?;
    let mut virtual_journal = Overlay::new(CHUNK, 1024, 8, Some(8))?;
    virtual_journal.delete_journal()?;
    ensure(!virtual_journal.state(JOURNAL)?.exists, "virtual delete")?;
    ensure(
        file_hash(&base_path)? == hash(&initial),
        "byte base changed",
    )?;
    let result = json!({"ordinary_file_oracle":true,"operations":operations,"capacity_exact_and_minus_one":true,"atomic_capacity_failure":true,
        "atomic_preread_failure":true,"zero_fill":true,"virtual_delete":true,"overlay":o.report(),"failure_overlay":one.report(),"io_failure_overlay":io.report()});
    save(&root.join("byte-contract.json"), &result)?;
    Ok(result)
}
fn limits_contract(root: &Path, limits: &Limits) -> Result<Value> {
    let good = serde_json::to_value(limits)?;
    let fields: Vec<String> = good
        .as_object()
        .ok_or("limits object")?
        .keys()
        .cloned()
        .collect();
    let mut rejected = Vec::new();
    for field in fields {
        for mutation in ["missing", "zero", "overflow", "negative"] {
            let mut bad = good.clone();
            let object = bad.as_object_mut().ok_or("limits object")?;
            match mutation {
                "missing" => {
                    object.remove(&field);
                }
                "zero" => {
                    object.insert(field.clone(), json!(0));
                }
                "overflow" => {
                    object.insert(field.clone(), json!(u64::MAX));
                }
                "negative" => {
                    object.insert(field.clone(), json!(-1));
                }
                _ => return Err("invalid limits mutation".into()),
            }
            let accepted =
                serde_json::from_value::<Limits>(bad).is_ok_and(|x| x.validate().is_ok());
            ensure(!accepted, "invalid explicit limit accepted")?;
            rejected.push(json!({"field":field,"mutation":mutation}));
        }
    }
    let mut extra = good;
    extra["unapproved_default"] = json!(1);
    ensure(
        serde_json::from_value::<Limits>(extra).is_err(),
        "unknown limit field accepted",
    )?;
    let result = json!({"profile":"D2-SMALL-01","rejected":rejected,"unknown_field_rejected":true,"defaults":false});
    save(&root.join("limits-contract.json"), &result)?;
    Ok(result)
}
fn super_journal_cases(root: &Path, end: u64, hot: &Path, config: &Path) -> Result<Value> {
    let mut cases = Vec::new();
    let sentinel = root.join("external-super-sentinel");
    fs::write(&sentinel, b"D2 external sentinel: must remain untouched")?;
    let sentinel_hash = file_hash(&sentinel)?;
    for (name, bytes, valid) in [
        (
            "absolute",
            sentinel.as_os_str().as_encoded_bytes().to_vec(),
            true,
        ),
        ("relative", b"../external-super-sentinel".to_vec(), true),
        ("embedded-nul", b"../outside\0tail".to_vec(), true),
        ("length-512", vec![b'x'; 512], true),
        ("length-513", vec![b'x'; 513], false),
        ("empty", vec![], false),
        ("bad-checksum", b"../outside-bad".to_vec(), false),
        ("zero-first-name", vec![0, b'x'], false),
        ("length-beyond-file", b"../outside-beyond".to_vec(), false),
        ("bad-magic", b"../outside-magic".to_vec(), false),
    ] {
        let dir = root.join(format!("super-{name}"));
        copy_source(hot, &dir)?;
        let jp = dir.join("main.sqlite-journal");
        let mut f = OpenOptions::new().append(true).open(&jp)?;
        f.write_all(&bytes)?;
        let footer_length = if name == "length-beyond-file" {
            u32::MAX
        } else {
            bytes.len() as u32
        };
        f.write_all(&footer_length.to_be_bytes())?;
        let mut sum = bytes.iter().fold(0_u32, |s, &b| {
            s.wrapping_add(b as std::ffi::c_char as i32 as u32)
        });
        if name == "bad-checksum" {
            sum = sum.wrapping_add(1);
        }
        f.write_all(&sum.to_be_bytes())?;
        let mut magic = MAGIC;
        if name == "bad-magic" {
            magic[7] ^= 1;
        }
        f.write_all(&magic)?;
        f.sync_all()?;
        drop(f);
        let detect = vfs::external_super_journal(&File::open(&jp)?)?;
        ensure(detect == valid, "footer detector boundary")?;
        let sqlite_boundary = if valid {
            let out = root.join(format!("super-{name}.json"));
            let before = snapshot(&dir)?;
            run(
                root,
                end,
                "reader",
                &[
                    dir.as_os_str(),
                    out.as_os_str(),
                    config.as_os_str(),
                    std::ffi::OsStr::new("normal"),
                ],
                &root.join(format!("super-{name}-reader")),
            )?;
            let result = load(&out)?;
            ensure(
                result["ok"] == false
                    && result["vfs_counters"]["registrations"] == 0
                    && result["vfs_counters"]["open_calls"] == 0,
                "external super reached SQLite",
            )?;
            ensure(snapshot(&dir)? == before, "super journal source changed")?;
            json!({"preconnection_denial":true})
        } else {
            compare_boundary(root, end, &dir, config, &format!("super-{name}"))?
        };
        ensure(
            file_hash(&sentinel)? == sentinel_hash,
            "external super sentinel changed",
        )?;
        cases.push(json!({"name":name,"detector_expected":valid,"detector":detect,"sqlite_boundary":sqlite_boundary,"sentinel_sha256":sentinel_hash}));
    }
    save(&root.join("super-journal-cases.json"), &cases)?;
    Ok(json!(cases))
}
fn compare_boundary(
    root: &Path,
    end: u64,
    source: &Path,
    config: &Path,
    name: &str,
) -> Result<Value> {
    let before = snapshot(source)?;
    let oracle_dir = root.join(format!("{name}-boundary-oracle"));
    let copied = copy_source(source, &oracle_dir)?;
    let oo = root.join(format!("{name}-boundary-oracle.json"));
    let ro = root.join(format!("{name}-boundary-reader.json"));
    run(
        root,
        end,
        "oracle",
        &[oracle_dir.as_os_str(), oo.as_os_str()],
        &root.join(format!("{name}-boundary-oracle-process")),
    )?;
    run(
        root,
        end,
        "reader",
        &[
            source.as_os_str(),
            ro.as_os_str(),
            config.as_os_str(),
            std::ffi::OsStr::new("normal"),
        ],
        &root.join(format!("{name}-boundary-reader-process")),
    )?;
    let oracle = load(&oo)?;
    let reader = load(&ro)?;
    ensure(
        reader["registration_or_harness_error"] != true,
        "boundary harness failed",
    )?;
    ensure(reader["ok"] == oracle["ok"], "boundary status differs")?;
    if oracle["ok"] == true {
        ensure(
            reader["data"] == oracle["data"]
                && reader["virtual_main_sha256"] == oracle["main_sha256"]
                && reader["virtual_main_len"] == oracle["main_len"],
            "boundary full data/bytes differ",
        )?;
        trace_check(&reader, false)?;
    } else {
        ensure(
            reader["extended_code"].is_i64() && reader["extended_code"] == oracle["extended_code"],
            "boundary error code absent or different",
        )?;
    }
    ensure(snapshot(source)? == before, "boundary source changed")?;
    let result = json!({"name":name,"copy":copied,"sqlite_succeeded":oracle["ok"],"positive_hot_proof":false,
        "status_and_full_data_or_error_equal":true,"before":before,"after":snapshot(source)?});
    save(&root.join(format!("{name}-boundary-case.json")), &result)?;
    Ok(result)
}
fn tail_boundaries(root: &Path, end: u64, hot: &Path, config: &Path) -> Result<Value> {
    let mut results = Vec::new();
    for mode in ["truncate-seven", "bad-last-byte"] {
        let dir = root.join(format!("tail-{mode}-source"));
        copy_source(hot, &dir)?;
        let jp = dir.join("main.sqlite-journal");
        let mut f = OpenOptions::new().read(true).write(true).open(&jp)?;
        let len = f.metadata()?.len();
        ensure(len > 16, "tail source short")?;
        if mode == "truncate-seven" {
            f.set_len(len - 7)?;
        } else {
            let mut byte = [0_u8; 1];
            f.read_exact_at(&mut byte, len - 1)?;
            byte[0] ^= 1;
            f.write_all_at(&byte, len - 1)?;
        }
        f.flush()?;
        drop(f);
        let before = snapshot(&dir)?;
        let oracle_dir = root.join(format!("tail-{mode}-oracle"));
        copy_source(&dir, &oracle_dir)?;
        let oo = root.join(format!("tail-{mode}-oracle.json"));
        let ro = root.join(format!("tail-{mode}-reader.json"));
        run(
            root,
            end,
            "oracle",
            &[oracle_dir.as_os_str(), oo.as_os_str()],
            &root.join(format!("tail-{mode}-oracle-process")),
        )?;
        run(
            root,
            end,
            "reader",
            &[
                dir.as_os_str(),
                ro.as_os_str(),
                config.as_os_str(),
                std::ffi::OsStr::new("normal"),
            ],
            &root.join(format!("tail-{mode}-reader-process")),
        )?;
        let oracle = load(&oo)?;
        let reader = load(&ro)?;
        ensure(
            reader["registration_or_harness_error"] != true,
            "tail harness failed",
        )?;
        ensure(reader["ok"] == oracle["ok"], "tail status differs")?;
        if oracle["ok"] == true {
            ensure(reader["data"] == oracle["data"], "tail full data differs")?;
            trace_check(&reader, true)?;
        } else {
            ensure(
                reader["extended_code"].is_i64()
                    && reader["extended_code"] == oracle["extended_code"],
                "tail error code differs",
            )?;
        }
        ensure(snapshot(&dir)? == before, "tail source changed")?;
        results.push(json!({"name":mode,"kind":"opaque-byte-mutation","sqlite_succeeded":oracle["ok"],"positive_hot_proof":false,"status_and_full_data_or_error_equal":true,"before":before,"after":snapshot(&dir)?}));
    }
    save(&root.join("tail-boundaries.json"), &results)?;
    Ok(json!(results))
}
pub fn suite(output: &Path, config: &Path) -> Result<()> {
    ensure(
        output.is_absolute() && config.is_absolute(),
        "absolute suite paths required",
    )?;
    let limits = Limits::load(config)?;
    ensure(
        limits.exact_small_profile(),
        "suite requires explicit approved D2-SMALL-01",
    )?;
    let end = deadline()?;
    check_deadline(end)?; // 执行器须在 build 前生成；suite没有新的1170秒起钟。
    let parent = output.parent().ok_or("output parent")?;
    ensure(
        parent.canonicalize()? == parent && !output.exists(),
        "canonical new evidence directory",
    )?;
    private(output)?;
    let config = output.join("limits.json");
    save(&config, &limits)?;
    let parameter_checks = limits_contract(output, &limits)?;
    let started = now_ns()?;
    let mut cases = Vec::new();
    let mut current_hot = None;
    let mut current_base = None;
    for (label, rule) in [
        ("current", CURRENT),
        ("old", "old-rule"),
        ("unknown", "unknown-rule"),
    ] {
        for mode in ["clean", "cold", "hot"] {
            check_deadline(end)?;
            let name = format!("{label}-{mode}");
            let work = output.join(format!("{name}-writer"));
            private(&work)?;
            let args = [
                work.as_os_str(),
                std::ffi::OsStr::new(rule),
                std::ffi::OsStr::new(mode),
            ];
            if mode == "hot" {
                kill_ready(
                    output,
                    end,
                    "writer",
                    &args,
                    &output.join(format!("{name}-writer")),
                )?;
            } else {
                run(
                    output,
                    end,
                    "writer",
                    &args,
                    &output.join(format!("{name}-writer")),
                )?;
            }
            let source = output.join(format!("{name}-source"));
            copy_source(&work, &source)?;
            let base = load(&work.join("baseline.json"))?;
            cases.push(compare_case(
                output,
                end,
                &source,
                &base,
                &config,
                &name,
                mode == "hot",
            )?);
            if label == "current" && mode == "hot" {
                current_hot = Some(source);
                current_base = Some(base);
            }
        }
    }
    let hot = current_hot.ok_or("hot fixture")?;
    let base = current_base.ok_or("baseline")?;
    let small = output.join("original-longer-source");
    copy_source(&hot, &small)?;
    let original_len = base["page_count"]
        .as_u64()
        .ok_or("page count")?
        .checked_mul(base["page_size"].as_u64().ok_or("page size")?)
        .ok_or("original length")?;
    let before = snapshot(&small)?;
    let truncated = (original_len / 2 / 1024) * 1024;
    OpenOptions::new()
        .write(true)
        .open(small.join("main.sqlite"))?
        .set_len(truncated)?;
    save(
        &output.join("original-longer-mutation.json"),
        &json!({"kind":"crash-image-mutation","journal_from_real_sigkill":true,"before":before,"after":snapshot(&small)?,"original_len":original_len,"mutated_main_len":truncated,"not_natural_crash_length_shape":true}),
    )?;
    cases.push(compare_case(
        output,
        end,
        &small,
        &base,
        &config,
        "original-longer",
        true,
    )?);
    let clean = output.join("current-clean-source");
    let empty = output.join("empty-journal-source");
    copy_source(&clean, &empty)?;
    fs::write(empty.join("main.sqlite-journal"), [])?;
    cases.push(compare_case(
        output,
        end,
        &empty,
        &base,
        &config,
        "empty-journal",
        false,
    )?);
    let tails = tail_boundaries(output, end, &hot, &config)?;
    let bytes = byte_contract(output)?;
    let supers = super_journal_cases(output, end, &hot, &config)?;
    let mut negatives = Vec::new();
    for mode in [
        "callbacks",
        "sql-denials",
        "read-io",
        "read-short",
        "read-panic",
        "late-exclusive",
        "close-error",
        "allocator-failure",
        "arena-oom",
        "path-swap",
    ] {
        let dir = output.join(format!("fault-{mode}-source"));
        copy_source(&hot, &dir)?;
        let out = output.join(format!("fault-{mode}.json"));
        run(
            output,
            end,
            "reader",
            &[
                dir.as_os_str(),
                out.as_os_str(),
                config.as_os_str(),
                std::ffi::OsStr::new(mode),
            ],
            &output.join(format!("fault-{mode}-reader")),
        )?;
        let r = load(&out)?;
        let expected_success =
            ["callbacks", "sql-denials", "allocator-failure", "path-swap"].contains(&mode);
        ensure(
            r["source_id"] == vfs::EXPECTED_SOURCE_ID
                && r["allocator"]["after_shutdown_xsize_bytes"] == 0,
            "negative reader infrastructure",
        )?;
        ensure(
            r["ok"] == json!(expected_success),
            &format!("fault result {mode}"),
        )?;
        if mode != "arena-oom" {
            ensure(
                r["registration_or_harness_error"] != true,
                "negative failed in harness rather than intended seam",
            )?;
        }
        if expected_success && mode != "callbacks" {
            ensure(
                r["data"] == base,
                "successful negative control lost baseline data",
            )?;
        }
        if ["read-io", "read-short", "read-panic"].contains(&mode) {
            let expected_rc = match mode {
                "read-io" => ffi::SQLITE_IOERR_READ,
                "read-short" => ffi::SQLITE_IOERR_SHORT_READ,
                _ => ffi::SQLITE_IOERR,
            };
            let events = r["lifecycle"]["report"]["events"]
                .as_array()
                .ok_or("negative trace missing")?;
            ensure(
                events
                    .iter()
                    .any(|e| e["op"] == "read" && e["rc"] == expected_rc),
                "read fault seam not reached",
            )?;
        }
        if mode == "read-panic" {
            ensure(
                r["lifecycle"]["report"]["caught_panics"]
                    .as_u64()
                    .unwrap_or(0)
                    > 0,
                "controlled panic not reached",
            )?;
        }
        if mode == "close-error" {
            ensure(
                r["close_error"] == true
                    && r["lifecycle"]["report"]["close_errors"]
                        .as_u64()
                        .unwrap_or(0)
                        > 0,
                "close fault not reached",
            )?;
        }
        if mode == "late-exclusive" {
            let events = r["lifecycle"]["report"]["events"]
                .as_array()
                .ok_or("late-exclusive trace missing")?;
            ensure(
                !events.iter().any(|e| e["op"] == "exclusive-armed")
                    && events
                        .iter()
                        .any(|e| e["op"] == "read" && e["rc"] == ffi::SQLITE_IOERR_READ),
                "late-exclusive seam not reached",
            )?;
        }
        if mode == "arena-oom" {
            ensure(
                r["owned_allocation_ledger"]["failures"]
                    .as_u64()
                    .unwrap_or(0)
                    > 0
                    && r["vfs_counters"]["registrations"] == 0,
                "arena OOM not reached",
            )?;
        }
        if mode == "path-swap" {
            ensure(
                r["binding_changed"] == true && r["accepted_rule"] == false && r["data"] == base,
                "swap binding",
            )?;
        }
        if mode == "sql-denials" {
            ensure(
                r["lifecycle"]["report"]["sql_denials"]
                    .as_u64()
                    .unwrap_or(0)
                    >= 7,
                "SQL denial count",
            )?;
            let proof = &r["statement_lifecycle"];
            ensure(
                proof["prepare_rc"] == ffi::SQLITE_OK
                    && proof["step_rc"] == ffi::SQLITE_ROW
                    && proof["close_rc"] == ffi::SQLITE_BUSY
                    && proof["close_refused"] == true
                    && proof["original_connection_retained"] == true
                    && proof["owner_registration_retained"] == true
                    && proof["active_files_after_refusal"].as_u64().unwrap_or(0) > 0
                    && proof["authorizer_restored_rc"] == ffi::SQLITE_OK
                    && proof["finalize_rc"] == ffi::SQLITE_OK,
                "active statement close-refusal lifecycle missing",
            )?;
            trace_check(&r, true)?;
        }
        negatives.push(json!({"mode":mode,"ok":r["ok"],"all_expected":true}));
    }
    let kill_source = output.join("reader-kill-source");
    copy_source(&hot, &kill_source)?;
    let before = snapshot(&kill_source)?;
    let kill_out = output.join("killed-reader.json");
    let killed = kill_ready(
        output,
        end,
        "reader",
        &[
            kill_source.as_os_str(),
            kill_out.as_os_str(),
            config.as_os_str(),
            std::ffi::OsStr::new("hold"),
        ],
        &output.join("reader-kill"),
    )?;
    ensure(
        snapshot(&kill_source)? == before,
        "killed reader modified source",
    )?;
    cases.push(compare_case(
        output,
        end,
        &kill_source,
        &base,
        &config,
        "post-reader-kill",
        true,
    )?);
    let result = json!({"verdict":"PASS_D2_COMPONENT_ONLY","cases":cases,"negative_cases":negatives,"super_journal":supers,"byte_contract":bytes,
        "reader_sigkill":killed,"tail_boundaries":tails,"limits":limits,"parameter_checks":parameter_checks,"absolute_deadline_ns":common_deadline()?,"suite_deadline_ns":end,"started_ns":started,"finished_ns":now_ns()?,"parent_process_never_initialized_sqlite":true,
        "supervisor_sigterm_sigint":"NOT_RUN","production_v4":"BLOCK_NOT_MERGEABLE","source_rw_opened":false,
        "outstanding":["formal producer/R4/CLI","complete S store baseline","source RW final-open continuity","synchronous IO hard deadline","49GB history","64GiB/store","148GiB dual-run prerequisite","whole graph acceptance"]});
    check_deadline(end)?;
    save(&output.join("RESULT.json"), &result)?;
    println!("PASS_D2_COMPONENT_ONLY {}", output.display());
    Ok(())
}
