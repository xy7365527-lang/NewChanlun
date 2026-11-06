//! #1769（ADR 0039 迁移步骤 1，C-L 验收第 3 项）：唯一生产 launcher 与 X 骨架的入口测试。
//!
//! 只在 Linux（nc-remote、CI ubuntu）运行：读 `/proc/<pid>/{environ,stat}`。凭据只用非敏感
//! 哨兵；不连接任何真实场所（端点指向测试内本地监听器）。S 的 admission 锚用与
//! `s_session/tests/issue1527_recovery/test_support.rs` 同口径的 TestOnly 造锚（生产签发归 #1657）。
//!
//! 两种构建都跑本文件，原八项在两种构建下都只走 S 正式配置路径（已签 synthetic profile，#1774）；
//! 纯 `production_session`（生产构建）另真实执行生产 launcher 与生产 S 遇 TestOnly 开关即拒的接线。
#![cfg(target_os = "linux")]
#![cfg_attr(
    not(feature = "production_session_testonly"),
    allow(dead_code, unused_imports)
)]

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
    /// S 库独占子目录：S admission 固定并核对库所在目录本身的身份（含 mtime），与 E/B/X 同目录时
    /// 它们并发建文件会使 S 以 SourceIdentityChanged 拒启。
    fn s_db(&self) -> PathBuf {
        let dir = self.path("s");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("s.db")
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

/// 期望「拒绝启动」的命令：限时等待退出。门失效时 S/launcher 会进入服务循环不退出，
/// 此处超时即杀掉 launcher 及其域子进程并判红，不吃满 job 超时。
#[cfg_attr(feature = "production_session_testonly", allow(dead_code))]
fn run_refusal(cmd: &mut Command) -> Output {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            for (_, pid) in domain_children(child.id()) {
                unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
            }
            let _ = child.kill();
            let _ = child.wait();
            panic!("拒启门失效：进程 60s 内未退出（应以拒绝退出），已杀掉");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    Output {
        status,
        stdout: out_reader.join().unwrap(),
        stderr: err_reader.join().unwrap(),
    }
}

/// v3 init（可带结构源声明）。
fn s_init_v3(space: &Space, declared: Option<&Value>) -> PathBuf {
    let db = space.s_db();
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

/// TestOnly 造锚：owner 绑定 + state/s-admission/{limits,binding}.json；返回 limits 路径。
fn anchor(db: &Path) -> PathBuf {
    anchor_with_wire_limit(db, 1_073_741_824)
}

fn anchor_with_wire_limit(db: &Path, max_total_wire_bytes: u64) -> PathBuf {
    let db = db.canonicalize().unwrap();
    let state = db.parent().unwrap().join("s-state");
    let published = state.join("s-admission");
    std::fs::create_dir_all(&published).unwrap();
    std::fs::set_permissions(&published, std::fs::Permissions::from_mode(0o700)).unwrap();
    let limits = published.join("limits.json");
    let binding_path = published.join("binding.json");
    let raw = serde_json::to_vec(&json!({
        "max_total_wire_bytes":max_total_wire_bytes.to_string(), "max_total_node_bytes":"1073741824", "max_total_nodes":"10000000",
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
/// 本文件只在拒启负控里设置它；成功助手一律 `env_remove`，走已签 synthetic 正式配置（#1769）。
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

/// 以正式配置（已签 synthetic profile）拉起 S 写者，等 socket 就绪。
fn s_serve(space: &Space, db: &Path, limits: &Path) -> Child {
    let socket = space.path("s.sock");
    let mut child = s_serve_command(space, db, limits).spawn().unwrap();
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
        .env_remove(THETA_DIAGNOSTIC.0)
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
    // S 持写锁后仍在做 admission（枚举并固定库文件家族）；开 socket 前读库会新建/改动 `-shm`，
    // 使 admission 以 SourceIdentityChanged 拒启。先等 socket，再读。
    eventually("S socket", || space.path("s.sock").exists());
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
    let s_db = space.s_db();
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

// #1845：所有声明负控使用同一 v3/profile/clock/resource/admission 夹具，避免旧库版本先拒启。
fn structure_source_fixture(space: &Space, fault: &str) -> (PathBuf, PathBuf, PathBuf) {
    let source = declaration("TestOnlyStructure", DECLARED_NS);
    let db = s_init_v3(
        space,
        if fault == "missing" {
            None
        } else {
            Some(&source)
        },
    );
    if fault == "stored_corrupt" {
        let conn = Connection::open(&db).unwrap();
        conn.execute("UPDATE meta SET value='{' WHERE key='structure_source'", [])
            .unwrap();
    }
    let limits = anchor(&db);
    let expected = if fault == "mismatch" {
        declaration("TestOnlyStructure", "testonly.other")
    } else {
        source
    };
    let file = write_json(&space.path("expected-source.json"), &expected);
    if fault == "file_corrupt" {
        std::fs::write(&file, "{").unwrap();
    } else if fault == "file_missing" {
        std::fs::remove_file(&file).unwrap();
    }
    (db, limits, file)
}

fn assert_source_refusal(fault: &str, expected_error: &str) {
    let space = Space::new();
    let (db, limits, file) = structure_source_fixture(&space, fault);
    let before = raw_rows_offline(&db);
    let out = run_refusal(
        s_serve_command(&space, &db, &limits)
            .arg("--structure-source")
            .arg(&file),
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(expected_error), "{stderr}");
    if fault == "mismatch" {
        for ns in [DECLARED_NS, "testonly.other"] {
            let canonical = newchan_rust::structure_source::Declaration::parse(&declaration(
                "TestOnlyStructure",
                ns,
            ))
            .unwrap()
            .canonical();
            // S 的错误字段是 JSON 字符串；去掉外层引号后核对完整声明的转义内容。
            let encoded = serde_json::to_string(&canonical).unwrap();
            assert!(stderr.contains(&encoded[1..encoded.len() - 1]), "{stderr}");
        }
    }
    assert!(!space.path("s.sock").exists());
    assert_eq!(raw_rows_offline(&db), before);
}

#[test]
fn s_serve_explicit_source_rejects_missing_stored_declaration() {
    assert_source_refusal("missing", "不带结构源声明");
}

#[test]
fn s_serve_explicit_source_rejects_mismatch_with_both_canonical_declarations() {
    assert_source_refusal("mismatch", "不一致");
}

#[test]
fn s_serve_explicit_source_rejects_corrupt_file() {
    assert_source_refusal("file_corrupt", "结构源声明 JSON 解析失败");
}

#[test]
fn s_serve_explicit_source_rejects_missing_file() {
    assert_source_refusal("file_missing", "读结构源声明");
}

#[test]
fn s_serve_explicit_source_charges_declaration_bytes_to_startup_limit() {
    let space = Space::new();
    let (db, _, file) = structure_source_fixture(&space, "match");
    let wire_limit = 8 * 1024 * 1024;
    let limits = anchor_with_wire_limit(&db, wire_limit);
    // 声明语义仍匹配，仅在合法 JSON 后追加超过整本启动账字节帽的空白。
    let mut bytes = std::fs::read(&file).unwrap();
    bytes.resize(bytes.len() + wire_limit as usize + 1, b' ');
    std::fs::write(&file, bytes).unwrap();
    let out = run_refusal(
        s_serve_command(&space, &db, &limits)
            .arg("--structure-source")
            .arg(&file),
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("读结构源声明"), "{stderr}");
    assert!(stderr.contains("max_total_wire_bytes"), "{stderr}");
    assert!(!space.path("s.sock").exists());
}

#[test]
fn s_serve_explicit_source_rejects_corrupt_stored_declaration() {
    assert_source_refusal("stored_corrupt", "meta.structure_source 损坏");
}

fn assert_source_listens(fault: &str, explicit: bool) {
    let space = Space::new();
    let (db, limits, file) = structure_source_fixture(&space, fault);
    let mut command = s_serve_command(&space, &db, &limits);
    if explicit {
        command.arg("--structure-source").arg(file);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !space.path("s.sock").exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("S serve 提前退出 {status}：{}", launcher_stderr(&mut child));
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("S socket 未就绪");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_lock_held(&db);
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn s_serve_explicit_source_match_listens() {
    assert_source_listens("match", true);
}

#[test]
fn s_serve_standalone_without_source_keeps_missing_declaration_contract() {
    assert_source_listens("missing", false);
}

#[test]
fn s_serve_standalone_without_source_does_not_check_corrupt_declaration() {
    assert_source_listens("stored_corrupt", false);
}

fn assert_launcher_source_failure(fault: &str) {
    let space = Space::new();
    let (db, limits, _) = structure_source_fixture(&space, fault);
    let before = raw_rows_offline(&db);
    let source = declaration(
        "TestOnlyStructure",
        if fault == "mismatch" {
            "testonly.other"
        } else {
            DECLARED_NS
        },
    );
    let xp = x_profile(&space, "simulation", None, Some("tb09-sentinel"), false);
    let manifest = launcher_manifest(&space, "simulation", &source, &db, &limits, &xp);
    let out = run_refusal(Command::new(launcher_bin()).arg("--manifest").arg(manifest));
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let events: Vec<Value> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("production_session "))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert!(
        events
            .iter()
            .any(|e| e["event"] == "exit" && e["domain"] == "S" && e["critical"] == true),
        "{stderr}"
    );
    assert!(
        events
            .iter()
            .any(|e| e["event"] == "stopped" && e["failed"] == true),
        "{stderr}"
    );
    assert!(!space.path("s.sock").exists());
    assert_eq!(raw_rows_offline(&db), before);
}

#[test]
fn production_session_refuses_s_db_without_source_declaration() {
    assert_launcher_source_failure("missing");
}

#[test]
fn production_session_refuses_s_db_with_mismatched_source_declaration() {
    assert_launcher_source_failure("mismatch");
}

#[test]
fn production_session_refuses_s_db_with_corrupt_source_declaration() {
    assert_launcher_source_failure("stored_corrupt");
}

#[test]
fn production_session_new_s_init_and_serve_share_source_declaration() {
    let space = Space::new();
    let (manifest, db) = simulation(&space, "simulation", false);
    // 锚绑定规范路径，init 重建同一路径；不改 admission 合同。
    std::fs::remove_file(&db).unwrap();
    let mut launcher = spawn_launcher(&manifest);
    let deadline = Instant::now() + Duration::from_secs(30);
    while !space.path("s.sock").exists() {
        if let Some(status) = launcher.try_wait().unwrap() {
            panic!(
                "launcher 提前退出 {status}：{}",
                launcher_stderr(&mut launcher)
            );
        }
        if Instant::now() >= deadline {
            stop(&mut launcher);
            panic!("新 S 库未监听");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(stop(&mut launcher).code(), Some(0));
    let stderr = launcher_stderr(&mut launcher);
    assert!(stderr.contains("\"event\":\"init\""), "{stderr}");
    let conn = Connection::open(&db).unwrap();
    let stored = newchan_rust::structure_source::stored(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(space.path("launcher-state/structure-source.json")).unwrap(),
        stored.canonical()
    );
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
    // 开 socket 前读 S 库会干扰其 admission（见 simulation_profile_spawns_isolated_domain_processes）。
    eventually("S socket", || space.path("s.sock").exists());
    let before = generation(&s_db);
    unsafe { libc::kill(x as libc::pid_t, libc::SIGSTOP) };
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

/// S 子进程的环境不得含 TestOnly 开关键（无论 launcher 自身环境有没有）。
fn assert_s_environ_lacks_theta_diagnostic(kids: &[(String, u32)]) {
    let (_, s_pid) = kids
        .iter()
        .find(|(name, _)| name == "s_structure_session")
        .expect("S 在 launcher 名下");
    let environ = environ(*s_pid);
    assert!(
        !environ.contains(&format!("{}=", THETA_DIAGNOSTIC.0)),
        "launcher 不得向 S 转交 {}",
        THETA_DIAGNOSTIC.0
    );
}

/// #1774 用户批后（synthetic 三项成本已签）正式路径的替换锁：不带 TestOnly 开关的 S 正式写者以
/// 已签 synthetic profile 开门（开 socket），不再以 AwaitingPolicy 拒启；未签拒启负控迁到
/// `driver.rs` 的 synthetic v1 夹具测试（`production_open_rejects_unsigned_profile_before_pending_begin`）。
#[test]
fn signed_synthetic_s_serve_starts_without_testonly_opt_in() {
    let space = Space::new();
    let s_db = s_init_v3(&space, Some(&declaration("TestOnlyStructure", DECLARED_NS)));
    let limits = anchor(&s_db);
    let socket = space.path("s.sock");
    let mut s = s_serve_command(&space, &s_db, &limits).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !socket.exists() {
        if let Some(status) = s.try_wait().unwrap() {
            panic!(
                "正式路径 S serve 提前退出 {status}：{}",
                launcher_stderr(&mut s)
            );
        }
        assert!(Instant::now() < deadline, "S socket 未就绪");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !environ(s.id()).contains(&format!("{}=", THETA_DIAGNOSTIC.0)),
        "正式路径不带 TestOnly 开关"
    );
    s.kill().unwrap();
    s.wait().unwrap();
}

/// launcher 本身不替 S 打开 TestOnly 臂：不设开关时经 launcher 拉起的 S 走正式路径（已签 synthetic）
/// 开门，其环境不含开关键。
#[test]
fn signed_synthetic_launcher_starts_s_without_testonly_opt_in() {
    let space = Space::new();
    let (manifest, _) = simulation(&space, "simulation", false);
    let mut launcher = Command::new(launcher_bin())
        .arg("--manifest")
        .arg(&manifest)
        .env_remove(CREDENTIAL_KEY)
        .env_remove(THETA_DIAGNOSTIC.0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let kids = wait_domains(&mut launcher);
    eventually("S socket", || space.path("s.sock").exists());
    assert_s_environ_lacks_theta_diagnostic(&kids);
    assert_eq!(stop(&mut launcher).code(), Some(0));
}

/// 生产构建（不含 production_session_testonly）的 S 正式写者遇 TestOnly 开关即拒启：
/// 真实执行 `writer_cli` 里 `cfg!(feature = "production_session_testonly")` 的接线，不开 socket、不改库。
#[test]
#[cfg(not(feature = "production_session_testonly"))]
fn production_build_s_serve_refuses_theta_diagnostic_switch() {
    let space = Space::new();
    let s_db = s_init_v3(&space, Some(&declaration("TestOnlyStructure", DECLARED_NS)));
    let limits = anchor(&s_db);
    let before = raw_rows_offline(&s_db);
    let out = run_refusal(
        s_serve_command(&space, &s_db, &limits).env(THETA_DIAGNOSTIC.0, THETA_DIAGNOSTIC.1),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("ThetaDiagnosticTestOnlyInvalid"),
        "{stderr}"
    );
    assert!(stderr.contains("此构建不支持"), "{stderr}");
    assert!(!space.path("s.sock").exists(), "生产构建不得开 socket");
    assert_eq!(raw_rows_offline(&s_db), before);
}

/// 生产 launcher 的 `S_PASS` 不转交 TestOnly 开关：即使 launcher 自身环境带开关，S 也只按正式路径
/// 以已签 synthetic 开门，环境不含开关键（若被转交，生产构建的 S 会以 ThetaDiagnosticTestOnlyInvalid
/// 拒启、launcher 提前退出，`wait_domains` 据此带 stderr 失败）。
#[test]
#[cfg(not(feature = "production_session_testonly"))]
fn production_launcher_does_not_forward_theta_diagnostic_switch() {
    let space = Space::new();
    let (manifest, _) = simulation(&space, "simulation", false);
    let mut launcher = Command::new(launcher_bin())
        .arg("--manifest")
        .arg(&manifest)
        .env_remove(CREDENTIAL_KEY)
        .env(THETA_DIAGNOSTIC.0, THETA_DIAGNOSTIC.1)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let kids = wait_domains(&mut launcher);
    eventually("S socket", || space.path("s.sock").exists());
    assert_s_environ_lacks_theta_diagnostic(&kids);
    assert_eq!(stop(&mut launcher).code(), Some(0));
}

// ── #1823 A 观察族：SIGKILL S → 原 manifest 重启的真实读数（只观察，不清理）──
// 收据经 `std::io::stderr().write_all` 直写（libtest 不截获），log 中缺收据即判「未观测」。
// 用例只对超时、遗留进程与自身 harness 错误失败；不对任何错误文本或阶段断言。

const OBS_READY: Duration = Duration::from_secs(30);
const OBS_L1_REAP: Duration = Duration::from_secs(30);
const OBS_L2: Duration = Duration::from_secs(60);
const OBS_STOP: Duration = Duration::from_secs(20);
const OBS_CASE: Duration = Duration::from_secs(150);

/// S v3 只读 `ready`：与 [`s_ingest`] 同一 `s-session/3` 信封，只换 payload。
fn s_ready(socket: &Path) -> Result<Value, String> {
    let payload = json!({"op":"ready"});
    let envelope = json!({"schema_revision":"s-session/3","session_id":SESSION,"session_generation":"1",
        "source_namespace":DECLARED_NS,"source_epoch":"1","message_id":"observe-ready","producer_id":"pse-test",
        "producer_epoch":"1","payload_hash":sha256_hex(newchan_rust::session_protocol::canonical_json(&payload).as_bytes()),
        "causal_refs":[],"payload":payload});
    let mut stream = UnixStream::connect(socket).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(
            format!(
                "{}\n",
                newchan_rust::session_protocol::canonical_json(&envelope)
            )
            .as_bytes(),
        )
        .map_err(|e| format!("write: {e}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    let mut reply = String::new();
    stream
        .read_to_string(&mut reply)
        .map_err(|e| format!("read: {e}"))?;
    serde_json::from_str(reply.trim_end()).map_err(|e| format!("reply {reply:?}: {e}"))
}

fn s_ready_matches(reply: &Value) -> bool {
    reply["kind"] == "Ready" && reply["session_id"] == SESSION && reply["session_generation"] == "1"
}

fn lock_held(db: &Path) -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("{}.writer.lock", db.display()))
        .is_ok_and(|lock| lock.try_lock().is_err())
}

fn lstat_record(path: &Path) -> Value {
    use std::os::unix::fs::FileTypeExt;
    match path.symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            json!({"path":path,"kind":"absent"})
        }
        Err(e) => json!({"path":path,"kind":"error","error":e.to_string()}),
        Ok(m) => {
            let t = m.file_type();
            let kind = if t.is_socket() {
                "socket"
            } else if t.is_symlink() {
                "symlink"
            } else if t.is_dir() {
                "dir"
            } else if t.is_file() {
                "file"
            } else {
                "other"
            };
            json!({"path":path,"kind":kind,"uid":m.uid(),"dev":m.dev(),"inode":m.ino(),"mode":m.mode()&0o7777})
        }
    }
}

/// `/proc/net/unix` 中绑定到 `path` 的 socket inode（收据增强，取不到写原因）。
fn proc_unix_inode(path: &Path) -> Value {
    match std::fs::read_to_string("/proc/net/unix") {
        Err(e) => json!({"unavailable": e.to_string()}),
        Ok(table) => {
            let want = path.to_string_lossy();
            let inodes: Vec<&str> = table
                .lines()
                .skip(1)
                .filter_map(|l| {
                    let cols: Vec<&str> = l.split_whitespace().collect();
                    (cols.len() >= 8 && cols[7] == want).then(|| cols[6])
                })
                .collect();
            json!({"inodes": inodes})
        }
    }
}

fn boot_and_start(pid: u32) -> Value {
    let boot = match std::fs::read_to_string("/proc/sys/kernel/random/boot_id") {
        Ok(s) => json!(s.trim()),
        Err(e) => json!({"unavailable": e.to_string()}),
    };
    let start = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| {
            s.rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().nth(19).map(str::to_string))
        });
    json!({"boot_id":boot,"start_time_ticks":start})
}

fn status_record(status: std::process::ExitStatus) -> Value {
    use std::os::unix::process::ExitStatusExt;
    json!({"display":status.to_string(),"code":status.code(),"signal":status.signal()})
}

fn stderr_record(bytes: &[u8]) -> Value {
    json!({"bytes":bytes.len(),"sha256":sha256_hex(bytes),"text":String::from_utf8_lossy(bytes)})
}

fn launcher_events(stderr: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter_map(|line| line.strip_prefix("production_session "))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// 首错阶段按 stderr 标记归类；原始 stderr 同在收据中，可由审查者复核。
/// `NOT_REACHED` 只在 S 参数解析门已退出时按源码顺序（`writer_cli` 先 `ServeOptions::parse`
/// 再 admission/epoch/Driver/serve_unix）标出，并写明依据。
fn first_error_stage(events: &[Value], stderr: &[u8], restarted_ready: bool) -> Value {
    let text = String::from_utf8_lossy(stderr);
    let first_exit = events
        .iter()
        .find(|e| e["event"] == "exit")
        .map(|e| e["domain"].clone());
    let (stage, not_reached) = if events.iter().any(|e| e["event"] == "refused") {
        ("launcher_refused", vec![])
    } else if text.contains("TransportUnavailable：socket路径已占用") {
        (
            "s_args_socket_gate",
            vec!["s_admission", "s_epoch", "s_db", "s_bind"],
        )
    } else if text.contains("Address already in use") || text.contains("os error 98") {
        ("bind_eaddrinuse", vec![])
    } else if text.contains("SourceIdentityChanged") {
        ("s_admission", vec![])
    } else if text.contains("StaleWriter") {
        ("epoch", vec![])
    } else if restarted_ready && first_exit.is_none() {
        ("none", vec![])
    } else {
        ("other", vec![])
    };
    json!({"stage":stage,"classified_by":"stderr markers","first_exit_domain":first_exit,
        "not_reached":not_reached.iter().map(|s| json!({"stage":s,
            "reason":"S 已在 ServeOptions::parse 的 socket 门退出，源码顺序上 admission/epoch/Driver/serve_unix 均在其后"})).collect::<Vec<_>>()})
}

struct Launched {
    child: Child,
    stderr: Option<std::thread::JoinHandle<Vec<u8>>>,
    kids_seen: Vec<(String, u32)>,
}

impl Launched {
    fn spawn(manifest: &Path) -> Self {
        let mut child = spawn_launcher(manifest);
        let mut pipe = child.stderr.take().unwrap();
        let stderr = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        });
        Self {
            child,
            stderr: Some(stderr),
            kids_seen: Vec::new(),
        }
    }

    fn note_kids(&mut self) -> Vec<(String, u32)> {
        let kids = domain_children(self.child.id());
        for k in &kids {
            if !self.kids_seen.contains(k) {
                self.kids_seen.push(k.clone());
            }
        }
        kids
    }

    fn wait_until(&mut self, deadline: Instant) -> Option<std::process::ExitStatus> {
        loop {
            self.note_kids();
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// 只回收本用例 launcher 及其直接域子进程。
    fn reap(&mut self) {
        for (_, pid) in domain_children(self.child.id()) {
            unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// launcher 已退出后取其 stderr；管道仍被遗留后代持有时不挂住，返回 None。
    fn take_stderr(&mut self) -> Option<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let handle = self.stderr.take()?;
        while !handle.is_finished() {
            if Instant::now() >= deadline {
                self.stderr = Some(handle);
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Some(handle.join().unwrap())
    }

    /// 本 launcher 拉起过的子进程是否都已消失（PID 不在 /proc 或已非其子）。
    fn leftover_kids(&self) -> Vec<(String, u32)> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let left: Vec<(String, u32)> = self
                .kids_seen
                .iter()
                .filter(|(_, pid)| {
                    Path::new(&format!("/proc/{pid}")).exists()
                        && std::fs::read_to_string(format!("/proc/{pid}/stat"))
                            .map(|s| !s.contains(") Z "))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
            if left.is_empty() || Instant::now() >= deadline {
                return left;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

struct Ready {
    s_reply: Value,
    kids: Vec<(String, u32)>,
}

/// 四域就绪：S 经 `s_ready` 核身份；E/B 持锁且 socket 可连；X 持锁且 `x_meta.writer_epoch=1`。
fn poll_ready(
    space: &Space,
    s_db: &Path,
    s_socket: &Path,
    run: &mut Launched,
    deadline: Instant,
) -> Result<Ready, Option<std::process::ExitStatus>> {
    loop {
        let kids = run.note_kids();
        if let Some(status) = run.child.try_wait().unwrap() {
            return Err(Some(status));
        }
        let names: Vec<&str> = kids.iter().map(|(n, _)| n.as_str()).collect();
        if DOMAINS.iter().all(|d| names.contains(d)) {
            let s = s_ready(s_socket).ok().filter(s_ready_matches);
            let eb_ok = || {
                [("e.db", "e.sock"), ("b.db", "b.sock")]
                    .iter()
                    .all(|(db, sock)| {
                        lock_held(&space.path(db)) && UnixStream::connect(space.path(sock)).is_ok()
                    })
            };
            let x_ok = || {
                lock_held(&space.path("x.db"))
                    && meta(
                        &space.path("x.db"),
                        "SELECT value FROM x_meta WHERE key='writer_epoch'",
                    )
                    .as_deref()
                        == Some("1")
            };
            if let Some(s_reply) = s {
                if lock_held(s_db) && eb_ok() && x_ok() {
                    return Ok(Ready { s_reply, kids });
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

struct Observation {
    scenario: &'static str,
    started: Instant,
    receipt: serde_json::Map<String, Value>,
}

impl Observation {
    fn new(scenario: &'static str, space: &Space) -> Self {
        let env = |k: &str| std::env::var(k).ok();
        let mut receipt = serde_json::Map::new();
        receipt.insert("scenario".into(), json!(scenario));
        receipt.insert(
            "build".into(),
            json!({"production_session_testonly": cfg!(feature = "production_session_testonly"),
                "github_sha":env("GITHUB_SHA"),"github_head_ref":env("GITHUB_HEAD_REF"),
                "run_url":env("GITHUB_RUN_ID").map(|id| format!("{}/{}/actions/runs/{id}",
                    env("GITHUB_SERVER_URL").unwrap_or_default(),env("GITHUB_REPOSITORY").unwrap_or_default())),
                "run_attempt":env("GITHUB_RUN_ATTEMPT"),"job":env("GITHUB_JOB")}),
        );
        receipt.insert("fixture_root".into(), json!(space.path("")));
        Self {
            scenario,
            started: Instant::now(),
            receipt,
        }
    }

    fn set(&mut self, key: &str, value: Value) {
        self.receipt.insert(key.into(), value);
    }

    fn emit(&mut self) {
        self.set(
            "elapsed_ms",
            json!(self.started.elapsed().as_millis() as u64),
        );
        let line = format!(
            "socket_residue_observe_receipt {}\n",
            Value::Object(self.receipt.clone())
        );
        std::io::stderr().write_all(line.as_bytes()).unwrap();
    }

    fn fail(&mut self, run: Option<&mut Launched>, why: &str) -> ! {
        if let Some(run) = run {
            run.reap();
        }
        self.set("harness_failure", json!(why));
        self.emit();
        panic!("{}：{why}", self.scenario);
    }
}

/// 一次 SIGKILL S → 原 manifest 重启的观察。`s_socket=None` 用现有 fixture（`R/s.sock`，M 外）。
fn observe_sigkill_restart(scenario: &'static str, s_socket: Option<&str>) {
    let space = Space::new();
    let mut obs = Observation::new(scenario, &space);
    let (manifest, s_db) = simulation(&space, "simulation", false);
    let socket = match s_socket {
        None => space.path("s.sock"),
        Some(rel) => {
            let path = space.path(rel);
            let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
            m["s"]["socket"] = json!(path);
            write_json(&manifest, &m);
            path
        }
    };
    let layout_class = match s_socket {
        None => "outside_M",
        Some(rel) if rel.contains("s-state") => "A_subtree",
        Some(_) => "basename_family",
    };
    let watched = [socket.clone(), space.path("e.sock"), space.path("b.sock")];
    let snapshot = |when: &str| -> Value {
        json!({"when":when,"sockets":watched.iter().map(|p| json!({"lstat":lstat_record(p),"proc_net_unix":proc_unix_inode(p)})).collect::<Vec<_>>()})
    };
    obs.set(
        "fixture",
        json!({"manifest":std::fs::read_to_string(&manifest).unwrap(),"s_db":s_db,"s_socket":socket,
            "layout":{"P":space.path("s"),"A":space.path("s/s-state"),
                "s_socket_class":layout_class},
            "profile":TB02A_PROFILE,"anchor":"TestOnly","credential":false}),
    );

    // L1：首启并等四域就绪。
    let mut l1 = Launched::spawn(&manifest);
    let ready = match poll_ready(&space, &s_db, &socket, &mut l1, Instant::now() + OBS_READY) {
        Ok(r) => r,
        Err(Some(status)) => {
            let Some(stderr) = l1.take_stderr() else {
                obs.fail(Some(&mut l1), "首启拒绝后 L1 stderr 管道仍被遗留进程持有")
            };
            obs.set(
                "first_start",
                json!({"ready":false,"l1_status":status_record(status),"l1_stderr":stderr_record(&stderr),
                    "stage":first_error_stage(&launcher_events(&stderr), &stderr, false)}),
            );
            obs.set("sigkill", json!("NOT_REACHED：L1 未就绪即退出"));
            obs.set("restart", json!("NOT_REACHED：L1 未就绪即退出"));
            obs.set("restarted_ready", json!(false));
            let left = l1.leftover_kids();
            if !left.is_empty() {
                obs.fail(Some(&mut l1), &format!("首启拒绝后遗留子进程 {left:?}"));
            }
            obs.emit();
            return;
        }
        Err(None) => obs.fail(Some(&mut l1), "L1 首启 30s 内未就绪"),
    };
    let pids: Vec<Value> = ready
        .kids
        .iter()
        .map(|(n, p)| json!({"domain":n,"pid":p,"identity":boot_and_start(*p)}))
        .collect();
    let epoch = |db: &str, sql: &str| meta(&space.path(db), sql);
    obs.set(
        "first_start",
        json!({"ready":true,"launcher_pid":l1.child.id(),"domains":pids,"s_ready_reply":ready.s_reply,
            "writer_epoch":{"e":epoch("e.db","SELECT v FROM meta WHERE k='writer_epoch'"),
                "b":epoch("b.db","SELECT v FROM meta WHERE k='writer_epoch'"),
                "x":epoch("x.db","SELECT value FROM x_meta WHERE key='writer_epoch'")},
            "before_kill":snapshot("before_kill")}),
    );

    // 仅向 L1 的 S 子进程发 SIGKILL；L1 自行回收其余域。
    let s_pid = ready
        .kids
        .iter()
        .find(|(n, _)| n == "s_structure_session")
        .unwrap()
        .1;
    let rc = unsafe { libc::kill(s_pid as libc::pid_t, libc::SIGKILL) };
    let errno = (rc != 0).then(|| std::io::Error::last_os_error().to_string());
    obs.set(
        "sigkill",
        json!({"target_pid":s_pid,"syscall_return":rc,"errno":errno}),
    );
    let Some(l1_status) = l1.wait_until(Instant::now() + OBS_L1_REAP) else {
        obs.fail(Some(&mut l1), "SIGKILL S 后 L1 30s 内未退出")
    };
    let left = l1.leftover_kids();
    let Some(l1_stderr) = l1.take_stderr() else {
        obs.fail(Some(&mut l1), "L1 stderr 管道仍被遗留进程持有")
    };
    let events = launcher_events(&l1_stderr);
    obs.set(
        "l1",
        json!({"status":status_record(l1_status),"stderr":stderr_record(&l1_stderr),
            "s_exit":events.iter().find(|e| e["event"]=="exit" && e["domain"]=="S"),
            "children_seen":l1.kids_seen,"children_left":left,"after_l1":snapshot("after_l1")}),
    );
    if !left.is_empty() {
        obs.fail(None, &format!("L1 子进程未回收 {left:?}"));
    }

    // L2：不删、不改任何 socket/manifest/DB/epoch/admission，原 manifest 重启。
    let mut l2 = Launched::spawn(&manifest);
    let l2_ready = poll_ready(&space, &s_db, &socket, &mut l2, Instant::now() + OBS_L2);
    let restarted_ready = l2_ready.is_ok();
    let (l2_status, l2_reply) = match l2_ready {
        Ok(r) => {
            unsafe { libc::kill(l2.child.id() as libc::pid_t, libc::SIGTERM) };
            match l2.wait_until(Instant::now() + OBS_STOP) {
                Some(status) => (status, r.s_reply),
                None => obs.fail(Some(&mut l2), "L2 就绪后 SIGTERM 20s 内未停止"),
            }
        }
        Err(Some(status)) => (status, Value::Null),
        Err(None) => obs.fail(Some(&mut l2), "L2 60s 内既未就绪也未退出"),
    };
    let left = l2.leftover_kids();
    let Some(l2_stderr) = l2.take_stderr() else {
        obs.fail(Some(&mut l2), "L2 stderr 管道仍被遗留进程持有")
    };
    let events = launcher_events(&l2_stderr);
    let projection = |v: &Value| {
        json!([
            v["writer_epoch"].clone(),
            v["generation"].clone(),
            v["structure_cut"].clone(),
            v["index_frontier"].clone()
        ])
    };
    let projection_equal = (!l2_reply.is_null())
        .then(|| projection(&obs.receipt["first_start"]["s_ready_reply"]) == projection(&l2_reply));
    obs.set("restarted_ready", json!(restarted_ready));
    obs.set(
        "l2",
        json!({"status":status_record(l2_status),"stderr":stderr_record(&l2_stderr),
            "domains_spawned":events.iter().filter(|e| e["event"]=="spawn").collect::<Vec<_>>(),
            "domains_exited":events.iter().filter(|e| e["event"]=="exit").collect::<Vec<_>>(),
            "s_ready_reply":l2_reply,"projection_equal":projection_equal,
            "first_error":first_error_stage(&events, &l2_stderr, restarted_ready),
            "children_seen":l2.kids_seen,"children_left":left,"after_l2":snapshot("after_l2")}),
    );
    if !left.is_empty() {
        obs.fail(None, &format!("L2 子进程未回收 {left:?}"));
    }
    if obs.started.elapsed() > OBS_CASE {
        obs.fail(None, "用例超过 150s 上限");
    }
    obs.emit();
}

#[test]
fn socket_residue_observe_sigkill_restart_outside_m() {
    observe_sigkill_restart("OBS-OUT", None);
}

#[test]
fn socket_residue_observe_invalid_layout_a_subtree() {
    observe_sigkill_restart("OBS-IN-A", Some("s/s-state/diagnostic.sock"));
}

#[test]
fn socket_residue_observe_invalid_layout_basename_family() {
    observe_sigkill_restart("OBS-IN-BASENAME", Some("s/s.db.sock"));
}
