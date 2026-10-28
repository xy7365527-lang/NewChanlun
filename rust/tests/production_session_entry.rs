//! #1769（ADR 0039 迁移步骤 1，C-L 验收第 3 项）：唯一生产 launcher 与 X 骨架的入口测试。
//!
//! 只在 Linux（nc-remote、CI ubuntu）运行：读 `/proc/<pid>/{environ,stat}`。凭据只用非敏感
//! 哨兵；不连接任何真实场所（端点指向测试内本地监听器）。S 的 admission 锚用与
//! `s_session/tests/issue1527_recovery/test_support.rs` 同口径的 TestOnly 造锚（生产签发归 #1657）。
#![cfg(target_os = "linux")]

use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const SESSION: &str = "s-session-testonly-001";
const DECLARED_NS: &str = "testonly.ohlc";
/// 能力 manifest 中的 HL 场所 namespace（执行行情；不得进 S）。
const HL_NS: &str = "hl.perp.BTC";
const CREDENTIAL_KEY: &str = "NC_X_VENUE_CREDENTIAL_SENTINEL";
const CREDENTIAL_VALUE: &str = "sentinel-not-a-secret";
/// v3 serve 只接受此完整固定 OHLC profile（`writer_cli::checked_profile_bytes`）。
const TB02A_PROFILE: &str = "profiles/ohlc_integer_tb02a_v1.json";

fn s_bin() -> &'static str {
    env!("CARGO_BIN_EXE_s_structure_session")
}
fn x_bin() -> &'static str {
    env!("CARGO_BIN_EXE_x_execution_session")
}
fn launcher_bin() -> &'static str {
    env!("CARGO_BIN_EXE_production_session")
}
fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("s_session")
        .join(rel)
}
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn write_json(path: &Path, value: &Value) -> PathBuf {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    path.to_path_buf()
}

struct Space(tempfile::TempDir);
impl Space {
    fn new() -> Self {
        Self(
            tempfile::Builder::new()
                .prefix("pse")
                .tempdir_in("/tmp")
                .unwrap(),
        )
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.path().canonicalize().unwrap().join(name)
    }
}

fn declaration(category: &str, ns: &str) -> Value {
    json!({"category": category, "namespaces": [ns]})
}

fn clock_plan(space: &Space) -> PathBuf {
    write_json(
        &space.path("clock.json"),
        &json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"pse-01","origin_utc":"2000-01-01T00:00:00Z","unit":"ns",
            "events":{"input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"}]},
                "input-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"22"}]}}}),
    )
}

fn run(cmd: &mut Command) -> Output {
    cmd.stdin(Stdio::null()).output().unwrap()
}

/// v3 init（可带结构源声明）。
fn s_init_v3(space: &Space, declared: Option<&Value>) -> PathBuf {
    let db = space.path("s.db");
    let mut c = Command::new(s_bin());
    c.args(["init", "--protocol-revision", "s-session/3", "--db"])
        .arg(&db)
        .args(["--session", SESSION, "--catalog"])
        .arg(fixture("catalog/signed-catalog.json"))
        .arg("--profile")
        .arg(fixture(TB02A_PROFILE))
        .args(["--session-generation", "1", "--clock-plan"])
        .arg(clock_plan(space))
        .args(["--delivery-retain-generations", "4"]);
    if let Some(d) = declared {
        c.arg("--structure-source")
            .arg(write_json(&space.path("source.json"), d));
    }
    let out = run(&mut c);
    assert!(
        out.status.success(),
        "S v3 init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    db
}

/// v1 init（既有 TestOnly 方式；可带结构源声明）。
fn s_init_v1(space: &Space, declared: Option<&Value>) -> PathBuf {
    let db = space.path("s.db");
    let mut c = Command::new(s_bin());
    c.args(["init", "--db"])
        .arg(&db)
        .args(["--session", SESSION, "--catalog"])
        .arg(fixture("catalog/signed-catalog.json"));
    if let Some(d) = declared {
        c.arg("--structure-source")
            .arg(write_json(&space.path("source.json"), d));
    }
    let out = run(&mut c);
    assert!(
        out.status.success(),
        "S v1 init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    db
}

/// TestOnly 造锚：owner 绑定 + state/s-admission/{limits,binding}.json；返回 limits 路径。
fn anchor(db: &Path) -> PathBuf {
    let db = db.canonicalize().unwrap();
    let state = db.parent().unwrap().join("s-state");
    let published = state.join("s-admission");
    std::fs::create_dir_all(&published).unwrap();
    std::fs::set_permissions(&published, std::fs::Permissions::from_mode(0o700)).unwrap();
    let limits = published.join("limits.json");
    let binding_path = published.join("binding.json");
    let raw = serde_json::to_vec(&json!({
        "max_total_wire_bytes":"1073741824", "max_total_node_bytes":"1073741824", "max_total_nodes":"10000000",
        "max_total_records":"10000000", "max_total_requests":"1000000", "max_elapsed_ms":"600000",
        "max_staged_bytes":"67108864", "max_store_bytes":"1073741824"
    }))
    .unwrap();
    for (p, bytes) in [(&limits, raw.as_slice()), (&binding_path, &b""[..])] {
        std::fs::write(p, bytes).unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let identity = |p: &Path| {
        let m = std::fs::metadata(p).unwrap();
        json!({"device":m.dev(),"inode":m.ino(),"uid":m.uid(),"mode":m.mode()&0o7777})
    };
    let binding = json!({"db":db,"path":limits,"sha256":sha256_hex(&raw),"file_identity":identity(&limits),
        "binding_identity":identity(&binding_path),"directory_identity":identity(&published)});
    std::fs::write(&binding_path, serde_json::to_vec(&binding).unwrap()).unwrap();
    let owner = PathBuf::from(format!("{}.control-owner.json", db.display()));
    std::fs::write(
        &owner,
        serde_json::to_vec(&json!({"db":db,"state_dir":state,"admission":binding})).unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(&owner, std::fs::Permissions::from_mode(0o600)).unwrap();
    limits
}

fn raw_rows(db: &Path) -> i64 {
    let c = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    c.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .unwrap()
}

/// S 未运行时读行数：`immutable=1` 不建 `-shm`（普通只读连接留下的 `-shm` 会让 S 写者
/// admission 拒绝启动）。S 运行中读最新提交用 [`raw_rows`]。
fn raw_rows_offline(db: &Path) -> i64 {
    let c = Connection::open_with_flags(
        format!("file:{}?mode=ro&immutable=1", db.display()),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    c.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .unwrap()
}

fn meta(db: &Path, sql: &str) -> Option<String> {
    let c = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    c.query_row(sql, [], |r| r.get(0)).ok()
}

fn x_profile(
    space: &Space,
    mode: &str,
    endpoint: Option<&str>,
    fingerprint: Option<&str>,
    credential: bool,
) -> PathBuf {
    let file = space.path("credential.txt");
    if credential {
        std::fs::write(&file, CREDENTIAL_VALUE).unwrap();
    }
    write_json(
        &space.path("x-profile.json"),
        &json!({"profile_revision":"x-execution-profile/1","mode":mode,"endpoint":endpoint,
            "tb09_fingerprint":fingerprint,"credential_scope":{"env":CREDENTIAL_KEY,"file":file}}),
    )
}

fn economic_manifest(space: &Space) -> PathBuf {
    write_json(
        &space.path("economic.json"),
        &json!({"schema_revision":"economic-manifest/1","session_id":"pse","session_generation":"1","book_revision":"tb03c-book/1",
            "e":{"db":space.path("e.db"),"socket":space.path("e.sock")},
            "chongs":[{"chong_id":"one","symbol":"TEST","operation_level":"1","fixed_q":"10","db":space.path("b.db"),"socket":space.path("b.sock")}],
            "authorized_sources":[{"source_namespace":"pse-test","source_epoch":"1","producer_id":"reader","producer_epoch":"1","operations":["ReadView","Watch"]}],
            "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"30000","queue_capacity":"2","max_connections":"2"},
            "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000"],"messages":{}}}}),
    )
}

fn s_resources(space: &Space) -> PathBuf {
    write_json(
        &space.path("s-resources.json"),
        &json!({"schema_revision":"s-structure-resources/1","s_resources":{
            "max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"30000",
            "audit_cache_source_bytes":"1048576","queue_capacity":"2","max_connections":"2"}}),
    )
}

/// S 写者以诊断夹具 theta 配置开门的 TestOnly 开关（仅 `production_session_testonly` 构建认）。
/// P0 场所/成本记录未签（#1774）期间正式配置必拒；本文件除 `unsigned_p0_refuses_s_serve_*` 外
/// 测的是 launcher 接线、进程隔离与来源门，显式走此 TestOnly 臂，不签、不伪造 P0。
const THETA_DIAGNOSTIC: (&str, &str) = ("S_THETA_DIAGNOSTIC_TESTONLY", "1");

/// S 正式写者（`serve --protocol-revision s-session/3`）命令；不含 TestOnly 开关。
fn s_serve_command(space: &Space, db: &Path, limits: &Path) -> Command {
    let mut command = Command::new(s_bin());
    command
        .args([
            "serve",
            "--protocol-revision",
            "s-session/3",
            "--admission-job-limits",
        ])
        .arg(limits)
        .arg("--db")
        .arg(db)
        .arg("--profile")
        .arg(fixture(TB02A_PROFILE))
        .args(["--writer-epoch", "1", "--clock-plan"])
        .arg(space.path("clock.json"))
        .arg("--socket")
        .arg(space.path("s.sock"))
        .arg("--resource-config")
        .arg(s_resources(space))
        .env_remove(THETA_DIAGNOSTIC.0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

/// 以 TestOnly 诊断 theta 臂拉起 S 写者，等 socket 就绪。
fn s_serve(space: &Space, db: &Path, limits: &Path) -> Child {
    let socket = space.path("s.sock");
    let mut child = s_serve_command(space, db, limits)
        .env(THETA_DIAGNOSTIC.0, THETA_DIAGNOSTIC.1)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !socket.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("S serve 提前退出 {status}：{}", launcher_stderr(&mut child));
        }
        assert!(Instant::now() < deadline, "S socket 未就绪");
        std::thread::sleep(Duration::from_millis(50));
    }
    child
}

fn launcher_manifest(
    space: &Space,
    profile: &str,
    source: &Value,
    s_db: &Path,
    limits: &Path,
    x_prof: &Path,
) -> PathBuf {
    let resources = s_resources(space);
    write_json(
        &space.path("production.json"),
        &json!({"manifest_revision":"production-session-manifest/1","profile":profile,
            "state_dir":space.path("launcher-state"),
            "execution_capabilities":{"namespaces":[HL_NS]},
            "structure_source":source,
            "s":{"db":s_db,"session":SESSION,"catalog":fixture("catalog/signed-catalog.json"),
                "profile":fixture(TB02A_PROFILE),"clock_plan":space.path("clock.json"),
                "session_generation":"1","delivery_retain_generations":"4","admission_job_limits":limits,
                "writer_epoch":"1","socket":space.path("s.sock"),"resource_config":resources},
            "economic":{"manifest":economic_manifest(space),"e":{"db":space.path("e.db"),"epoch":"1"},
                "b":[{"chong":"one","db":space.path("b.db"),"epoch":"1"}]},
            "x":{"db":space.path("x.db"),"profile":x_prof,"writer_epoch":"1"}}),
    )
}

fn spawn_launcher(manifest: &Path) -> Child {
    assert!(
        std::env::var_os(CREDENTIAL_KEY).is_none(),
        "测试进程环境不得含哨兵键"
    );
    Command::new(launcher_bin())
        .arg("--manifest")
        .arg(manifest)
        .env_remove(CREDENTIAL_KEY)
        .env(THETA_DIAGNOSTIC.0, THETA_DIAGNOSTIC.1)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn ppid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
fn exe_name(pid: u32) -> String {
    std::fs::read(format!("/proc/{pid}/cmdline"))
        .ok()
        .and_then(|b| {
            b.split(|c| *c == 0)
                .next()
                .map(|s| String::from_utf8_lossy(s).into_owned())
        })
        .map(|s| {
            Path::new(&s)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_default()
}
fn environ(pid: u32) -> String {
    String::from_utf8_lossy(&std::fs::read(format!("/proc/{pid}/environ")).unwrap()).into_owned()
}
/// launcher 直接子进程：域名 → pid。
fn domain_children(launcher: u32) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if ppid(pid) == Some(launcher) {
            out.push((exe_name(pid), pid));
        }
    }
    out.sort();
    out
}
const DOMAINS: [&str; 4] = [
    "b_chong_session",
    "e_economic_session",
    "s_structure_session",
    "x_execution_session",
];

fn launcher_stderr(child: &mut Child) -> String {
    let mut s = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut s);
    }
    s
}

/// 等四类域进程都在 launcher 名下存活；launcher 先退出则带其 stderr 失败。
fn wait_domains(launcher: &mut Child) -> Vec<(String, u32)> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = launcher.try_wait().unwrap() {
            panic!("launcher 提前退出 {status}：{}", launcher_stderr(launcher));
        }
        let kids = domain_children(launcher.id());
        let names: Vec<&str> = kids.iter().map(|(n, _)| n.as_str()).collect();
        if DOMAINS.iter().all(|d| names.contains(d)) {
            return kids;
        }
        assert!(Instant::now() < deadline, "域进程未全部就绪：{kids:?}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn stop(launcher: &mut Child) -> std::process::ExitStatus {
    unsafe { libc::kill(launcher.id() as libc::pid_t, libc::SIGTERM) };
    launcher.wait().unwrap()
}

/// 域进程出现在 launcher 名下不等于已持锁/建库/开 socket；就绪类断言一律限时轮询。
fn eventually(what: &str, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready() {
        assert!(Instant::now() < deadline, "{what} 未就绪");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn assert_lock_held(db: &Path) {
    let path = format!("{}.writer.lock", db.display());
    eventually(&format!("{path} 被持有"), || {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .is_ok_and(|lock| lock.try_lock().is_err())
    });
}

/// 完整 simulation 夹具：S 带声明 init + 锚；返回 (manifest, s_db)。
fn simulation(space: &Space, profile: &str, credential: bool) -> (PathBuf, PathBuf) {
    let source = declaration("TestOnlyStructure", DECLARED_NS);
    let s_db = s_init_v3(space, Some(&source));
    let limits = anchor(&s_db);
    let xp = x_profile(space, profile, None, Some("tb09-sentinel"), credential);
    (
        launcher_manifest(space, profile, &source, &s_db, &limits, &xp),
        s_db,
    )
}

#[test]
fn simulation_profile_spawns_isolated_domain_processes() {
    let space = Space::new();
    let (manifest, s_db) = simulation(&space, "simulation", false);
    let mut launcher = spawn_launcher(&manifest);
    let kids = wait_domains(&mut launcher);
    let mut pids: Vec<u32> = kids.iter().map(|(_, p)| *p).collect();
    pids.sort_unstable();
    pids.dedup();
    assert_eq!(pids.len(), kids.len(), "PID 必须互异");
    for db in [
        s_db.clone(),
        space.path("e.db"),
        space.path("b.db"),
        space.path("x.db"),
    ] {
        assert_lock_held(&db);
    }
    assert!(meta(&s_db, "SELECT value FROM meta WHERE key='writer_epoch'").is_some());
    for db in ["e.db", "b.db"] {
        eventually(&format!("{db} 的 writer_epoch"), || {
            meta(&space.path(db), "SELECT v FROM meta WHERE k='writer_epoch'").is_some()
        });
    }
    eventually("x.db 的 writer_epoch=1", || {
        meta(
            &space.path("x.db"),
            "SELECT value FROM x_meta WHERE key='writer_epoch'",
        )
        .as_deref()
            == Some("1")
    });
    let status = stop(&mut launcher);
    assert_eq!(status.code(), Some(0), "{}", launcher_stderr(&mut launcher));
    for (_, pid) in kids {
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "子进程 {pid} 未回收"
        );
    }
}

#[test]
fn credential_sentinel_only_in_x_process_environment() {
    let space = Space::new();
    // 凭据文件须存在，launcher 才按 credential_scope 读取并只注入 X。
    let (manifest, _) = simulation(&space, "simulation", true);
    let mut launcher = spawn_launcher(&manifest);
    let kids = wait_domains(&mut launcher);
    assert!(
        !environ(launcher.id()).contains(CREDENTIAL_KEY),
        "launcher 环境含哨兵键"
    );
    for (name, pid) in &kids {
        let has = environ(*pid).contains(CREDENTIAL_KEY);
        assert_eq!(
            has,
            name == "x_execution_session",
            "{name} 环境哨兵键归属错误"
        );
    }
    assert_eq!(stop(&mut launcher).code(), Some(0));
}

#[test]
fn missing_credential_refuses_x_send_surface() {
    let space = Space::new();
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let xp = x_profile(
        &space,
        "experiment",
        Some(&endpoint),
        Some("tb09-sentinel"),
        false,
    );
    let out = run(Command::new(x_bin())
        .args(["serve", "--db"])
        .arg(space.path("x.db"))
        .arg("--profile")
        .arg(&xp)
        .args(["--writer-epoch", "1"])
        .env_remove(CREDENTIAL_KEY));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("MissingDependency"));
    std::thread::sleep(Duration::from_millis(200));
    let mut connections = 0;
    while listener.accept().is_ok() {
        connections += 1;
    }
    assert_eq!(connections, 0, "X 不得连接端点");
    // S 不受 X 缺凭据影响：声明源输入经正式写入口照常接纳。
    let s_db = s_init_v3(&space, Some(&declaration("TestOnlyStructure", DECLARED_NS)));
    let limits = anchor(&s_db);
    let mut s = s_serve(&space, &s_db, &limits);
    let reply = s_ingest(&space.path("s.sock"), DECLARED_NS, 1);
    assert!(!reply.to_string().contains("InvalidDomain"), "{reply}");
    assert_eq!(raw_rows(&s_db), 1, "{reply}");
    s.kill().unwrap();
    s.wait().unwrap();
}

#[test]
fn production_profile_without_tb09_fingerprint_refuses_send() {
    let space = Space::new();
    let xp = x_profile(&space, "production", None, None, true);
    let db = space.path("x.db");
    let mut x = Command::new(x_bin())
        .args(["serve", "--db"])
        .arg(&db)
        .arg("--profile")
        .arg(&xp)
        .args(["--writer-epoch", "1"])
        .env(CREDENTIAL_KEY, CREDENTIAL_VALUE)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let state = loop {
        let out = run(Command::new(x_bin()).args(["query", "--db"]).arg(&db));
        if out.status.success() {
            break serde_json::from_slice::<Value>(&out.stdout).unwrap();
        }
        assert!(Instant::now() < deadline, "X 查询不可用");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(state["send_state"], "refused");
    assert_eq!(state["send_refusal_reason"], "MissingTb09Fingerprint");
    assert_eq!(state["tb09_fingerprint"], "");
    let submit = run(Command::new(x_bin()).args(["submit", "--db"]).arg(&db));
    assert!(String::from_utf8_lossy(&submit.stderr).contains("UnsupportedCapability"));
    x.kill().unwrap();
    x.wait().unwrap();
}

#[test]
fn manifest_rejects_execution_feed_as_structure_source() {
    let space = Space::new();
    let s_db = space.path("s.db");
    let xp = x_profile(&space, "simulation", None, Some("tb09-sentinel"), false);
    // HL 场所 namespace，载荷与美股 bar 同形，类别仍标个股线。
    let source = declaration("UsEquityStockLineTick", HL_NS);
    let manifest = launcher_manifest(
        &space,
        "simulation",
        &source,
        &s_db,
        &space.path("limits.json"),
        &xp,
    );
    let out = run(Command::new(launcher_bin())
        .arg("--manifest")
        .arg(&manifest));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("InvalidDomain"));
    assert!(!s_db.exists(), "S 库不得被创建");
}

#[test]
fn s_rejects_same_shape_bars_from_undeclared_source() {
    let space = Space::new();
    let s_db = s_init_v3(&space, Some(&declaration("TestOnlyStructure", DECLARED_NS)));
    let limits = anchor(&s_db);
    let before = raw_rows_offline(&s_db);
    let mut s = s_serve(&space, &s_db, &limits);
    let rejected = s_ingest(&space.path("s.sock"), HL_NS, 1);
    assert!(rejected.to_string().contains("InvalidDomain"), "{rejected}");
    assert_eq!(raw_rows(&s_db), before);
    let accepted = s_ingest(&space.path("s.sock"), DECLARED_NS, 2);
    assert!(
        !accepted.to_string().contains("InvalidDomain"),
        "{accepted}"
    );
    assert_eq!(raw_rows(&s_db), before + 1, "{accepted}");
    s.kill().unwrap();
    s.wait().unwrap();
}

#[test]
fn production_session_refuses_s_db_without_source_declaration() {
    let space = Space::new();
    let s_db = s_init_v1(&space, None);
    let before = raw_rows(&s_db);
    let xp = x_profile(&space, "simulation", None, Some("tb09-sentinel"), false);
    let source = declaration("TestOnlyStructure", DECLARED_NS);
    let manifest = launcher_manifest(
        &space,
        "simulation",
        &source,
        &s_db,
        &space.path("limits.json"),
        &xp,
    );
    let out = run(Command::new(launcher_bin())
        .arg("--manifest")
        .arg(&manifest));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("InvalidDomain"));
    assert_eq!(raw_rows(&s_db), before);
}

/// 经 S socket 提交第 `n` 条 ingest（`s-session/3` 封包、`s-ohlc/1` 原件；一帧 LF + 写端 EOF）。
fn s_ingest(socket: &Path, ns: &str, n: u32) -> Value {
    let raw = json!({"schema_revision":"s-ohlc/1","session_id":SESSION,"source_namespace":ns,"source_epoch":"1",
        "instrument":"TEST.OHLC","profile":"ohlc_integer_tb02a_v1",
        "events":[{"event_id":format!("bar-{n}"),"revision":"1","seq":(n * 10).to_string(),"received_at":"2026-09-18T00:00:00Z",
            "raw_text":"3/12","open":"6","high":"12","low":"3","close":"7","timestamp":(n * 10).to_string(),"volume":"1"}]});
    let payload = json!({"op":"ingest","target_session_id":SESSION,"target_session_generation":"1","writer_epoch":"1",
        "clock_event_id":format!("input-{n}"),"raw_input":raw});
    let envelope = json!({"schema_revision":"s-session/3","session_id":SESSION,"session_generation":"1",
        "source_namespace":ns,"source_epoch":"1","message_id":format!("message-{n}"),"producer_id":"pse-test",
        "producer_epoch":"1","payload_hash":sha256_hex(newchan_rust::session_protocol::canonical_json(&payload).as_bytes()),
        "causal_refs":[],"payload":payload});
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .write_all(
            format!(
                "{}\n",
                newchan_rust::session_protocol::canonical_json(&envelope)
            )
            .as_bytes(),
        )
        .unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).unwrap();
    serde_json::from_str(reply.trim_end()).unwrap()
}

#[test]
fn s_keeps_committing_while_x_is_stopped() {
    let space = Space::new();
    let (manifest, s_db) = simulation(&space, "simulation", false);
    let mut launcher = spawn_launcher(&manifest);
    let kids = wait_domains(&mut launcher);
    let x = kids
        .iter()
        .find(|(n, _)| n == "x_execution_session")
        .unwrap()
        .1;
    let generation = |db: &Path| meta(db, "SELECT value FROM meta WHERE key='generation'").unwrap();
    let before = generation(&s_db);
    unsafe { libc::kill(x as libc::pid_t, libc::SIGSTOP) };
    eventually("S socket", || space.path("s.sock").exists());
    let reply = s_ingest(&space.path("s.sock"), DECLARED_NS, 1);
    let deadline = Instant::now() + Duration::from_secs(20);
    while generation(&s_db) == before {
        assert!(
            Instant::now() < deadline,
            "X 停止期间 S generation 未增加：{reply}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        meta(&s_db, "SELECT value FROM meta WHERE key='structure_cut'").is_some(),
        "快照不可读"
    );
    unsafe { libc::kill(x as libc::pid_t, libc::SIGCONT) };
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        launcher.try_wait().unwrap().is_none(),
        "launcher 应仍在监督"
    );
    let now: Vec<String> = domain_children(launcher.id())
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    for d in DOMAINS {
        assert!(now.iter().any(|n| n == d), "{d} 不在监督中：{now:?}");
    }
    assert_eq!(stop(&mut launcher).code(), Some(0));
}

/// 生产 P0 场所/成本记录未签（#1774）时，不带 TestOnly 开关的 S 正式写者拒绝启动：
/// 以 AwaitingPolicy 退出、不开 socket、不改库；同一构建里 TestOnly 臂不影响正式路径。
#[test]
fn unsigned_p0_refuses_s_serve_without_testonly_opt_in() {
    let space = Space::new();
    let s_db = s_init_v3(&space, Some(&declaration("TestOnlyStructure", DECLARED_NS)));
    let limits = anchor(&s_db);
    let before = raw_rows_offline(&s_db);
    let out = s_serve_command(&space, &s_db, &limits).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("AwaitingPolicy"), "{stderr}");
    assert!(stderr.contains("venue_profile"), "{stderr}");
    assert!(stderr.contains("cost_inputs"), "{stderr}");
    assert!(!space.path("s.sock").exists(), "未签 P0 不得开 socket");
    assert_eq!(raw_rows_offline(&s_db), before);
}

/// launcher 本身不替 S 打开 TestOnly 臂：不设开关时经 launcher 拉起的 S 同样拒绝，launcher 失败退出。
#[test]
fn unsigned_p0_refuses_s_serve_via_launcher_without_testonly_opt_in() {
    let space = Space::new();
    let (manifest, _) = simulation(&space, "simulation", false);
    let out = run(Command::new(launcher_bin())
        .arg("--manifest")
        .arg(&manifest)
        .env_remove(CREDENTIAL_KEY)
        .env_remove(THETA_DIAGNOSTIC.0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("AwaitingPolicy"), "{stderr}");
}
