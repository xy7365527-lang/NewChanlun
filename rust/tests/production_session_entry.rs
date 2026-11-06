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
    let (mut c, db) = s_init_v3_command(space, declared);
    let out = run(&mut c);
    assert!(
        out.status.success(),
        "S v3 init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    db
}

fn s_init_v3_command(space: &Space, declared: Option<&Value>) -> (Command, PathBuf) {
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
    (c, db)
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

// #1823 A 观察族：SIGKILL S → 原 manifest 重启。只观察不清理；收据经 `std::io::stderr().write_all`
// 直写（libtest 不截获），所有出口共用 `finalize`。只对超时、遗留进程与 harness 错误失败，不对错误文本断言。
// 每个用例在本测试二进制的独立 helper 进程中执行：subreaper 是进程级属性，只能随 helper 生灭，
// helper 的全部子进程才可视为本例所有物。

use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::{Arc, Mutex};

/// 父测试（三个公开用例）的绝对上限。
const OBS_CASE: Duration = Duration::from_secs(150);
/// helper 内部用例预算；父测试在其外再留启动、超时回收与收据的时间。
const OBS_HELPER_CASE: Duration = Duration::from_secs(135);
const OBS_HELPER_WAIT: Duration = Duration::from_secs(142);
const OBS_HELPER_REAP_DONE: Duration = Duration::from_secs(3);
const OBS_HELPER_SETTLE_DONE: Duration = Duration::from_secs(1);
const OBS_HELPER_ENV: &str = "PSE_OBS_HELPER_SCENARIO";
const OBS_HELPER_TEST: &str = "socket_residue_observe_helper";
/// 父测试进入接管回收的两种触发原因；真实观察用例遇到任一都判失败。
const OBS_TRIGGER_HANDOFF: &str = "helper 请求交接本例后代，父测试接管回收";
const OBS_TRIGGER_TIMEOUT: &str = "helper 未在限时内退出，父测试接管回收";
/// 接管回收之后仍有后代存在或 helper 未被 wait。
const OBS_TREE_OPEN: &str = "helper 进程树未闭合";
/// 父测试接管时请 helper 收割已终结孩子的信号。
const OBS_DRAIN_SIGNAL: libc::c_int = libc::SIGUSR1;
const OBS_SCENARIOS: [(&str, Option<&str>); 3] = [
    ("OBS-OUT", None),
    ("OBS-IN-A", Some("s/s-state/diagnostic.sock")),
    ("OBS-IN-BASENAME", Some("s/s.db.sock")),
];
/// 从 helper 预算中预留给冻结、回收、捕获终态、收尾投影与收据的时间。
const OBS_REAP_RESERVE: Duration = Duration::from_secs(15);
const OBS_REAP_DONE: Duration = Duration::from_secs(8);
const OBS_SETTLE_DONE: Duration = Duration::from_secs(5);
const OBS_READ_DONE: Duration = Duration::from_secs(2);
const OBS_INIT: Duration = Duration::from_secs(20);
const OBS_READY: Duration = Duration::from_secs(30);
const OBS_L1_REAP: Duration = Duration::from_secs(30);
const OBS_L2: Duration = Duration::from_secs(60);
const OBS_STOP: Duration = Duration::from_secs(10);
const OBS_FREEZE: Duration = Duration::from_secs(5);
const OBS_PROBE: Duration = Duration::from_secs(5);
const OBS_PROBE_LOG: usize = 40;

const OBS_DATA: [(&str, &str, &str); 4] = [
    (
        "s.raw_events",
        "s",
        "SELECT CAST(COUNT(*) AS TEXT) FROM raw_events",
    ),
    (
        "s.generation",
        "s",
        "SELECT value FROM meta WHERE key='generation'",
    ),
    (
        "s.structure_cut",
        "s",
        "SELECT value FROM meta WHERE key='structure_cut'",
    ),
    (
        "s.index_frontier",
        "s",
        "SELECT value FROM meta WHERE key='index_frontier'",
    ),
];
const OBS_CONTROL: [(&str, &str, &str); 4] = [
    (
        "s.writer_epoch",
        "s",
        "SELECT value FROM meta WHERE key='writer_epoch'",
    ),
    (
        "e.writer_epoch",
        "e",
        "SELECT v FROM meta WHERE k='writer_epoch'",
    ),
    (
        "b.writer_epoch",
        "b",
        "SELECT v FROM meta WHERE k='writer_epoch'",
    ),
    (
        "x.writer_epoch",
        "x",
        "SELECT value FROM x_meta WHERE key='writer_epoch'",
    ),
];
/// 每行至多归一个标记，按此顺序取第一个命中（S 专属文本先于泛化的 TransportUnavailable）。
const OBS_MARKERS: [(&str, &str, Option<&str>); 6] = [
    (
        "TransportUnavailable：socket路径已占用",
        "s_args_socket_gate",
        Some("S"),
    ),
    ("Address already in use", "bind_eaddrinuse", None),
    ("os error 98", "bind_eaddrinuse", None),
    ("SourceIdentityChanged", "source_identity_changed", None),
    ("StaleWriter", "stale_writer", None),
    ("TransportUnavailable", "transport_unavailable", None),
];

struct Budget {
    start: Instant,
    ops_end: Instant,
    end: Instant,
}

impl Budget {
    fn new() -> Self {
        let start = Instant::now();
        Self {
            start,
            ops_end: start + OBS_HELPER_CASE - OBS_REAP_RESERVE,
            end: start + OBS_HELPER_CASE,
        }
    }
    fn phase(&self, limit: Duration) -> Instant {
        (Instant::now() + limit).min(self.ops_end)
    }
    fn before_end(&self, margin: Duration) -> Instant {
        self.end - margin
    }
    fn ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

fn left(until: Instant) -> Duration {
    until.saturating_duration_since(Instant::now())
}

fn sleep_until_at_most(until: Instant, step: Duration) {
    std::thread::sleep(left(until).min(step));
}

fn wait_fd(fd: libc::c_int, events: libc::c_short, until: Instant) -> Result<(), String> {
    loop {
        let ms = left(until).as_millis().min(i32::MAX as u128) as libc::c_int;
        if ms == 0 {
            return Err("TIMEOUT：绝对期限已到".into());
        }
        let mut pfd = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
        if rc > 0 {
            return Ok(());
        }
        if rc == 0 {
            return Err("TIMEOUT：绝对期限已到".into());
        }
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::Interrupted {
            return Err(format!("poll: {e}"));
        }
    }
}

/// 非阻塞 connect：受同一绝对期限约束，失败或超时时 fd 在返回前随 `UnixStream` 关闭。
fn unix_connect(socket: &Path, until: Instant) -> Result<UnixStream, String> {
    use std::os::unix::ffi::OsStrExt;
    let path = socket.as_os_str().as_bytes();
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if path.len() >= addr.sun_path.len() {
        return Err("socket 路径过长".into());
    }
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (dst, src) in addr.sun_path.iter_mut().zip(path) {
        *dst = *src as libc::c_char;
    }
    let len = (std::mem::size_of::<libc::sa_family_t>() + path.len() + 1) as libc::socklen_t;
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(format!("socket: {}", std::io::Error::last_os_error()));
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    loop {
        if left(until).is_zero() {
            return Err("TIMEOUT：connect 未在期限内完成".into());
        }
        let rc = unsafe {
            libc::connect(
                fd,
                (&addr as *const libc::sockaddr_un).cast::<libc::sockaddr>(),
                len,
            )
        };
        if rc == 0 {
            return Ok(stream);
        }
        let e = std::io::Error::last_os_error();
        match e.raw_os_error() {
            Some(libc::EINTR) => {}
            Some(libc::EAGAIN) => sleep_until_at_most(until, Duration::from_millis(20)),
            Some(libc::EINPROGRESS) => {
                wait_fd(fd, libc::POLLOUT, until)?;
                return match stream.take_error() {
                    Ok(None) => Ok(stream),
                    Ok(Some(e)) | Err(e) => Err(format!("connect: {e}")),
                };
            }
            _ => return Err(format!("connect: {e}")),
        }
    }
}

/// 一帧请求、写端 EOF、读到对端 EOF；全部在 `until` 前完成，返回前 fd 已关闭。
fn unix_exchange(socket: &Path, request: &[u8], until: Instant) -> Result<Vec<u8>, String> {
    let mut stream = unix_connect(socket, until)?;
    let fd = stream.as_raw_fd();
    let mut sent = 0;
    while sent < request.len() {
        if left(until).is_zero() {
            return Err("TIMEOUT：write 未在期限内完成".into());
        }
        match stream.write(&request[sent..]) {
            Ok(0) => return Err("write: 写入 0 字节".into()),
            Ok(n) => sent += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                wait_fd(fd, libc::POLLOUT, until)?
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(format!("write: {e}")),
        }
    }
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| format!("shutdown: {e}"))?;
    let mut reply = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if left(until).is_zero() {
            return Err("TIMEOUT：read 未在期限内读到 EOF".into());
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(reply),
            Ok(n) => reply.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                wait_fd(fd, libc::POLLIN, until)?
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(format!("read: {e}")),
        }
    }
}

/// S v3 只读 `ready`：与 [`s_ingest`] 同一 `s-session/3` 信封，只换 payload。
fn s_ready(socket: &Path, until: Instant) -> Result<Value, String> {
    let payload = json!({"op":"ready"});
    let envelope = json!({"schema_revision":"s-session/3","session_id":SESSION,"session_generation":"1",
        "source_namespace":DECLARED_NS,"source_epoch":"1","message_id":"observe-ready","producer_id":"pse-test",
        "producer_epoch":"1","payload_hash":sha256_hex(newchan_rust::session_protocol::canonical_json(&payload).as_bytes()),
        "causal_refs":[],"payload":payload});
    let line = format!(
        "{}\n",
        newchan_rust::session_protocol::canonical_json(&envelope)
    );
    let reply = unix_exchange(socket, line.as_bytes(), until)?;
    let text = String::from_utf8_lossy(&reply);
    serde_json::from_str(text.trim_end()).map_err(|e| format!("reply {text:?}: {e}"))
}

/// S 回复是 `reply_envelope` 形状的完整信封，Ready 状态在 `payload` 内。信封头与 payload 都须属于本会话
/// 化身（`session_generation`，不是 payload 的数据 `generation`），且 `payload_hash` 须与 payload 相符。
fn s_ready_matches(reply: &Value) -> bool {
    let payload = &reply["payload"];
    let identity = |v: &Value| {
        v["schema_revision"] == "s-session/3"
            && v["session_id"] == SESSION
            && v["session_generation"] == "1"
    };
    payload.is_object()
        && identity(reply)
        && reply["payload_hash"]
            == sha256_hex(newchan_rust::session_protocol::canonical_json(payload).as_bytes())
        && identity(payload)
        && payload["kind"] == "Ready"
        && payload["ok"] == true
}

#[test]
fn s_ready_matcher_requires_identity_in_envelope_and_payload() {
    let request = json!({"source_namespace":DECLARED_NS,"source_epoch":"1","message_id":"observe-ready","payload_hash":"h"});
    let ready = json!({"ok":true,"kind":"Ready","schema_revision":"s-session/3","session_id":SESSION,
        "session_generation":"1","writer_epoch":"1","generation":"0","structure_cut":"cut-0","index_frontier":""});
    let envelope = |payload: Value, session_id: &str, session_generation: &str| {
        newchan_rust::session_protocol::reply_envelope(
            &request,
            payload,
            "s-session/3",
            session_id,
            session_generation,
            "s-session/replies",
            &format!("S:{session_id}"),
            "1",
        )
    };
    let with = |key: &str, value: Value| {
        let mut p = ready.clone();
        p[key] = value;
        p
    };
    assert!(s_ready_matches(&envelope(ready.clone(), SESSION, "1")));

    let mut tampered = envelope(ready.clone(), SESSION, "1");
    tampered["payload"]["structure_cut"] = json!("cut-9");
    let mut wrong_schema = envelope(ready.clone(), SESSION, "1");
    wrong_schema["schema_revision"] = json!("s-session/2");
    let rejected = [
        ("裸 payload", ready.clone()),
        (
            "信封缺 payload",
            json!({"schema_revision":"s-session/3","session_id":SESSION,"session_generation":"1"}),
        ),
        ("非对象", json!("Ready")),
        (
            "信封 session_id 错",
            envelope(ready.clone(), "other-session", "1"),
        ),
        (
            "信封 session_generation 错",
            envelope(ready.clone(), SESSION, "2"),
        ),
        ("信封 schema 错", wrong_schema),
        ("payload_hash 不符", tampered),
        (
            "payload session_id 错",
            envelope(with("session_id", json!("other-session")), SESSION, "1"),
        ),
        (
            "payload session_generation 错",
            envelope(with("session_generation", json!("0")), SESSION, "1"),
        ),
        ("payload 缺 session_generation 只有 generation", {
            let mut p = ready.clone();
            p.as_object_mut().unwrap().remove("session_generation");
            envelope(p, SESSION, "1")
        }),
        (
            "payload schema 错",
            envelope(with("schema_revision", json!("s-session/2")), SESSION, "1"),
        ),
        (
            "非 Ready",
            envelope(with("kind", json!("Receipt")), SESSION, "1"),
        ),
        ("ok=false", envelope(with("ok", json!(false)), SESSION, "1")),
    ];
    for (case, reply) in rejected {
        assert!(!s_ready_matches(&reply), "{case} 不得被当作 Ready：{reply}");
    }
}

/// 只读观察域写锁：查 `/proc/locks` 里该锁文件 inode 上的 FLOCK WRITE 记录，不打开锁文件、不尝试加锁。
/// 域进程以一次性 `try_lock` 取锁（`session_transport::process_lock`），探针若先拿到锁会让它以 StaleWriter 拒启。
/// 设备号与 inode 必须同时相符才算同一锁文件；对不上即未就绪，不以持锁 PID 推断对象。
fn lock_observed(db: &Path) -> Result<bool, String> {
    let path = PathBuf::from(format!("{}.writer.lock", db.display()));
    let m = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("stat {}: {e}", path.display())),
    };
    let dev = format!("{:02x}:{:02x}", libc::major(m.dev()), libc::minor(m.dev()));
    let ino = m.ino().to_string();
    let table = std::fs::read_to_string("/proc/locks")
        .map_err(|e| format!("/proc/locks 不可读，锁状态未观测：{e}"))?;
    Ok(lock_table_holds_write(&table, &dev, &ino))
}

/// `/proc/locks` 中是否有生效（非阻塞等待）的 FLOCK WRITE 记录落在 `dev`（`MAJ:MIN`，十六进制）上的 `ino`。
fn lock_table_holds_write(table: &str, dev: &str, ino: &str) -> bool {
    table.lines().any(|line| {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() < 6 || c[1] != "FLOCK" || c[3] != "WRITE" {
            return false;
        }
        c[5].rsplit_once(':') == Some((dev, ino))
    })
}

#[test]
fn lock_table_requires_same_device_and_inode() {
    let held = "1: FLOCK  ADVISORY  WRITE 4242 08:01:123 0 EOF\n";
    assert!(lock_table_holds_write(held, "08:01", "123"));
    for (case, table) in [
        (
            "设备不同 inode 相同",
            "1: FLOCK  ADVISORY  WRITE 4242 08:02:123 0 EOF\n",
        ),
        (
            "inode 不同",
            "1: FLOCK  ADVISORY  WRITE 4242 08:01:124 0 EOF\n",
        ),
        ("读锁", "1: FLOCK  ADVISORY  READ 4242 08:01:123 0 EOF\n"),
        (
            "POSIX 锁",
            "1: POSIX  ADVISORY  WRITE 4242 08:01:123 0 EOF\n",
        ),
        (
            "阻塞等待项",
            "1: -> FLOCK  ADVISORY  WRITE 4243 08:01:123 0 EOF\n",
        ),
        ("空表", ""),
    ] {
        assert!(
            !lock_table_holds_write(table, "08:01", "123"),
            "{case} 不得判为持有"
        );
    }
}

/// SQLite 只读读取：busy 等待不超过剩余绝对预算（替换库默认的 5s busy handler）。
fn read_field(db: &Path, sql: &str, until: Instant) -> Value {
    if left(until).is_zero() {
        return json!({"not_run":"预算已耗尽"});
    }
    match Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Err(e) => json!({"error": e.to_string()}),
        Ok(c) => {
            let budget = left(until);
            if budget.is_zero() {
                return json!({"not_run":"开库后预算已耗尽"});
            }
            if let Err(e) = c.busy_timeout(budget) {
                return json!({"error": format!("busy_timeout: {e}")});
            }
            match c.query_row(sql, [], |r| r.get::<_, Option<String>>(0)) {
                Ok(v) => json!({ "value": v }),
                Err(rusqlite::Error::QueryReturnedNoRows) => json!({"absent": true}),
                Err(e) => json!({"error": e.to_string()}),
            }
        }
    }
}

fn x_epoch_ready(space: &Space, until: Instant) -> bool {
    read_field(
        &space.path("x.db"),
        "SELECT value FROM x_meta WHERE key='writer_epoch'",
        until,
    )["value"]
        == "1"
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn status_record(status: std::process::ExitStatus) -> Value {
    use std::os::unix::process::ExitStatusExt;
    json!({"display":status.to_string(),"code":status.code(),"signal":status.signal()})
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
            json!({"path":path,"kind":kind,"uid":m.uid(),"dev":m.dev(),"inode":m.ino(),"mode":m.mode()&0o7777,
                "mtime":m.mtime(),"mtime_nsec":m.mtime_nsec()})
        }
    }
}

/// `/proc/net/unix` 中绑定到 `path` 的 socket inode（收据增强，取不到写原因）。
fn proc_unix_inode(path: &Path) -> Value {
    match std::fs::read_to_string("/proc/net/unix") {
        Err(e) => json!({"unavailable": e.to_string()}),
        Ok(table) => {
            let want = path.to_string_lossy();
            let inodes: Vec<String> = table
                .lines()
                .skip(1)
                .filter_map(|l| {
                    let cols: Vec<&str> = l.split_whitespace().collect();
                    (cols.len() >= 8 && cols[7] == want).then(|| cols[6].to_string())
                })
                .collect();
            json!({"inodes": inodes})
        }
    }
}

fn stat_fields(pid: u32) -> Option<Vec<String>> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    Some(
        stat.rsplit_once(')')?
            .1
            .split_whitespace()
            .map(str::to_string)
            .collect(),
    )
}

/// 进程身份 = PID + /proc starttime；PID 复用后身份不再匹配。`role` 取自 cmdline，进程成僵尸后变空，
/// 只作展示，不参与相等比较。
#[derive(Clone, Debug)]
struct ProcId {
    role: String,
    pid: u32,
    start: String,
}

impl PartialEq for ProcId {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid && self.start == other.start
    }
}

#[derive(Debug, PartialEq)]
enum ReapOutcome {
    Reaped(libc::c_int),
    Running,
    /// ECHILD 且身份已不存在：进程已终结并被收割，但不是本次调用收割的。
    EchildGone,
    /// ECHILD 但身份仍存在：它已不是本进程的孩子，归属丢失。
    Lost(&'static str),
    Error(String),
}

/// 按身份对本进程的一个孩子做一次非阻塞 waitpid。
fn try_reap(p: &ProcId) -> ReapOutcome {
    let mut status: libc::c_int = 0;
    let rc = unsafe { libc::waitpid(p.pid as libc::pid_t, &mut status, libc::WNOHANG) };
    if rc == p.pid as libc::pid_t {
        return ReapOutcome::Reaped(status);
    }
    if rc == 0 {
        return ReapOutcome::Running;
    }
    let e = std::io::Error::last_os_error();
    match (e.raw_os_error(), p.state()) {
        (Some(libc::EINTR), _) => ReapOutcome::Running,
        (Some(libc::ECHILD), "gone" | "pid_reused") => ReapOutcome::EchildGone,
        (Some(libc::ECHILD), state) => ReapOutcome::Lost(state),
        _ => ReapOutcome::Error(e.to_string()),
    }
}

impl ProcId {
    fn capture(role: &str, pid: u32) -> Option<Self> {
        Some(Self {
            role: role.into(),
            pid,
            start: stat_fields(pid)?.get(19)?.clone(),
        })
    }
    fn state(&self) -> &'static str {
        match stat_fields(self.pid) {
            None => "gone",
            Some(f) if f.get(19) != Some(&self.start) => "pid_reused",
            Some(f) => match f.first().map(String::as_str) {
                Some("Z") => "zombie",
                Some("T") | Some("t") => "stopped",
                _ => "alive",
            },
        }
    }
    fn exists(&self) -> bool {
        !matches!(self.state(), "gone" | "pid_reused")
    }
    fn json(&self) -> Value {
        json!({"role":self.role,"pid":self.pid,"start_time_ticks":self.start,"state":self.state()})
    }
}

/// 经 pidfd 发信号：先固定进程引用再核身份，PID 被复用或已成僵尸时不发。
fn signal_identity(p: &ProcId, signal: libc::c_int) -> Value {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, p.pid as libc::pid_t, 0) };
    if fd < 0 {
        return json!({"pid":p.pid,"start_time_ticks":p.start,"signal":signal,"delivered":false,
            "pidfd_open":fd,"errno":std::io::Error::last_os_error().to_string()});
    }
    let fd = fd as libc::c_int;
    let state = p.state();
    let (rc, errno) = if matches!(state, "alive" | "stopped") {
        let rc = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                fd,
                signal,
                std::ptr::null::<libc::siginfo_t>(),
                0u32,
            )
        };
        (
            Some(rc),
            (rc != 0).then(|| std::io::Error::last_os_error().to_string()),
        )
    } else {
        (None, None)
    };
    unsafe { libc::close(fd) };
    json!({"pid":p.pid,"start_time_ticks":p.start,"signal":signal,"state_at_signal":state,
        "pidfd_send_signal":rc,"errno":errno,"delivered":rc == Some(0)})
}

fn children_of(parent: u32) -> Vec<ProcId> {
    domain_children(parent)
        .into_iter()
        .filter_map(|(name, pid)| ProcId::capture(&name, pid))
        .collect()
}

/// 两次连续扫描结果一致才视为稳定；/proc 遍历不是原子快照。
fn stable_children(parent: u32, until: Instant) -> (Vec<ProcId>, bool) {
    let mut last = children_of(parent);
    loop {
        let next = children_of(parent);
        if next == last {
            return (next, true);
        }
        if left(until).is_zero() {
            let extra: Vec<ProcId> = next.into_iter().filter(|p| !last.contains(p)).collect();
            last.extend(extra);
            return (last, false);
        }
        last = next;
        sleep_until_at_most(until, Duration::from_millis(20));
    }
}

/// helper 进程是 child subreaper：launcher 死后其后代改挂到 helper，可按 PID waitpid。
/// helper 只承载一个用例，故其全部子进程（除仍持有 `Child` 句柄的 launcher）均属本例。
fn reap_adopted(exclude: &[u32], until: Instant) -> Value {
    let me = std::process::id();
    let mut signals = Vec::new();
    let mut handled: Vec<ProcId> = Vec::new();
    let (mut reaped, mut echild_gone, mut lost, mut errors) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut pending: Vec<ProcId> = Vec::new();
    loop {
        let (now, stable) = stable_children(me, until);
        for p in now.into_iter().filter(|p| !exclude.contains(&p.pid)) {
            if !pending.contains(&p) && !handled.contains(&p) {
                signals.push(signal_identity(&p, libc::SIGKILL));
                pending.push(p);
            }
        }
        let mut still = Vec::new();
        for p in pending.drain(..) {
            let entry = |extra: Value| json!({"role":p.role,"pid":p.pid,"start_time_ticks":p.start,"detail":extra});
            match try_reap(&p) {
                ReapOutcome::Running => {
                    still.push(p);
                    continue;
                }
                ReapOutcome::Reaped(status) => reaped.push(entry(json!({"wait_status":status}))),
                ReapOutcome::EchildGone => echild_gone.push(entry(Value::Null)),
                ReapOutcome::Lost(state) => lost.push(entry(json!({"state":state}))),
                ReapOutcome::Error(e) => {
                    if errors.len() < OBS_PROBE_LOG {
                        errors.push(entry(json!(e)));
                    }
                    still.push(p);
                    continue;
                }
            }
            handled.push(p);
        }
        pending = still;
        if (pending.is_empty() && stable) || left(until).is_zero() {
            let pending: Vec<Value> = pending.iter().map(ProcId::json).collect();
            return json!({"subreaper_scan_stable":stable,"signals":signals,"reaped":reaped,
                "echild_gone":echild_gone,"lost":lost,"wait_errors":errors,"unreaped":pending});
        }
        sleep_until_at_most(until, Duration::from_millis(20));
    }
}

fn adoption_open(adopted: &Value) -> bool {
    ["unreaped", "lost"]
        .iter()
        .any(|k| adopted[*k].as_array().is_some_and(|a| !a.is_empty()))
}

struct CaptureState {
    bytes: Vec<u8>,
    end: Option<Result<(), String>>,
}

/// 持续读入原始字节；`settle` 在预算内等到 EOF/读错误，否则如实报告未完成。
struct Capture {
    state: Arc<Mutex<CaptureState>>,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl Capture {
    fn start(mut source: impl Read + Send + 'static) -> Self {
        let state = Arc::new(Mutex::new(CaptureState {
            bytes: Vec::new(),
            end: None,
        }));
        let shared = state.clone();
        let reader = std::thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            loop {
                match source.read(&mut chunk) {
                    Ok(0) => {
                        shared.lock().unwrap().end = Some(Ok(()));
                        return;
                    }
                    Ok(n) => shared.lock().unwrap().bytes.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => {
                        shared.lock().unwrap().end = Some(Err(e.to_string()));
                        return;
                    }
                }
            }
        });
        Self {
            state,
            reader: Some(reader),
        }
    }
    fn len(&self) -> usize {
        self.state.lock().unwrap().bytes.len()
    }
    fn settle(&mut self, until: Instant) {
        while self.reader.as_ref().is_some_and(|r| !r.is_finished()) && !left(until).is_zero() {
            sleep_until_at_most(until, Duration::from_millis(10));
        }
        if self.reader.as_ref().is_some_and(|r| r.is_finished()) {
            let _ = self.reader.take().unwrap().join();
        }
    }
    /// 一次加锁取出字节与终态；同一份快照同时供原始字节与首错解析使用。
    fn frozen(&self) -> (Vec<u8>, Value) {
        let state = self.state.lock().unwrap();
        let end = match (&state.end, self.reader.is_none()) {
            (Some(Ok(())), true) => json!("eof"),
            (Some(Ok(())), false) => json!("eof_reader_not_joined"),
            (Some(Err(e)), _) => json!({"read_error": e}),
            (None, _) => json!("timeout_incomplete"),
        };
        (state.bytes.clone(), end)
    }
    fn complete(&self) -> bool {
        self.reader.is_none() && matches!(self.state.lock().unwrap().end, Some(Ok(())))
    }
    fn contains(&self, needle: &[u8]) -> bool {
        self.state
            .lock()
            .unwrap()
            .bytes
            .windows(needle.len())
            .any(|w| w == needle)
    }
}

fn capture_json(bytes: &[u8], end: Value) -> Value {
    json!({"bytes":bytes.len(),"end":end,"sha256":sha256_hex(bytes),
        "hex":hex_bytes(bytes),"utf8_lossy":String::from_utf8_lossy(bytes)})
}

/// 按原始到达顺序列出错误标记与 launcher exit 事件。多进程共写一条管道，到达顺序不证明因果先后。
fn error_timeline(stderr: &[u8], stop_offset: Option<usize>) -> Value {
    let mut offset = 0;
    let mut markers = Vec::new();
    let mut exits = Vec::new();
    for (line_no, line) in stderr.split(|b| *b == b'\n').enumerate() {
        let phase = if stop_offset.is_some_and(|s| offset >= s) {
            "after_active_stop"
        } else {
            "observation"
        };
        let text = String::from_utf8_lossy(line);
        let event = text
            .strip_prefix("production_session ")
            .and_then(|j| serde_json::from_str::<Value>(j).ok());
        if let Some(ev) = event {
            if ev["event"] == "refused" {
                markers.push(json!({"line":line_no,"offset":offset,"phase":phase,
                    "marker":"launcher_refused","domain":"launcher","text":text}));
            } else if ev["event"] == "exit" {
                exits.push(json!({"line":line_no,"offset":offset,"phase":phase,"event":ev}));
            }
        } else if let Some((_, kind, domain)) =
            OBS_MARKERS.iter().find(|(m, _, _)| text.contains(m))
        {
            markers.push(json!({"line":line_no,"offset":offset,"phase":phase,
                "marker":kind,"domain":domain,"text":text}));
        }
        offset += line.len() + 1;
    }
    let observed: Vec<&Value> = markers
        .iter()
        .filter(|m| m["phase"] == "observation")
        .collect();
    let s_gate = observed.iter().any(|m| m["marker"] == "s_args_socket_gate");
    let s_exit = exits
        .iter()
        .any(|e| e["phase"] == "observation" && e["event"]["domain"] == "S");
    let s_local = if s_gate && s_exit {
        json!({"stage":"s_args_socket_gate","not_reached":["s_admission","s_epoch","s_db","s_bind"],
            "basis":"观察窗内同时出现 S 专属 parse 门文本与 launcher 记录的 S exit；writer_cli 中 ServeOptions::parse 先于 admission/epoch/Driver/serve_unix。只陈述 S 局部路径，不陈述 E/B 相对先后"})
    } else {
        json!({"stage":"UNKNOWN","basis":"观察窗内未同时出现 S parse 门文本与 S exit"})
    };
    json!({"markers":markers,"exits":exits,"active_stop_offset":stop_offset,
        "earliest_observed_marker":observed.first(),
        "causal_first_error":{"value":"UNKNOWN","reason":"launcher 与 S/E/B/X 共写同一 stderr 管道，到达顺序不证明跨进程因果先后"},
        "s_local":s_local})
}

#[derive(Default)]
struct Probes {
    total: usize,
    log: Vec<Value>,
}

impl Probes {
    fn note(&mut self, run: &Launched, budget: &Budget, kind: &str, result: Result<(), String>) {
        self.total += 1;
        if self.log.len() < OBS_PROBE_LOG {
            self.log.push(json!({"launch":run.label,"t_ms":budget.ms(),"kind":kind,
                "stderr_offset":run.stderr.len(),"fd_closed_before_return":true,"error":result.err()}));
        }
    }
}

struct Launched {
    label: &'static str,
    child: Child,
    launcher: Option<ProcId>,
    stdout: Capture,
    stderr: Capture,
    seen: Vec<ProcId>,
    status: Option<std::process::ExitStatus>,
    active_stop: Option<Value>,
    reap: Option<Value>,
}

impl Launched {
    fn spawn(label: &'static str, manifest: &Path) -> Self {
        assert!(
            std::env::var_os(CREDENTIAL_KEY).is_none(),
            "测试进程环境不得含哨兵键"
        );
        let mut child = Command::new(launcher_bin())
            .arg("--manifest")
            .arg(manifest)
            .env_remove(CREDENTIAL_KEY)
            .env_remove(THETA_DIAGNOSTIC.0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = Capture::start(child.stdout.take().unwrap());
        let stderr = Capture::start(child.stderr.take().unwrap());
        let launcher = ProcId::capture("production_session", child.id());
        Self {
            label,
            child,
            launcher,
            stdout,
            stderr,
            seen: Vec::new(),
            status: None,
            active_stop: None,
            reap: None,
        }
    }

    fn merge(&mut self, found: Vec<ProcId>) {
        for id in found {
            if !self.seen.contains(&id) {
                self.seen.push(id);
            }
        }
    }

    /// launcher 未被 wait 前其 PID 不会复用，此时按 PPID 记录的子进程身份可信。
    fn poll(&mut self) -> Option<std::process::ExitStatus> {
        if self.status.is_none() {
            let found = children_of(self.child.id());
            self.merge(found);
            self.status = self.child.try_wait().ok().flatten();
        }
        self.status
    }

    fn wait_exit(&mut self, until: Instant) -> Option<std::process::ExitStatus> {
        loop {
            if let Some(status) = self.poll() {
                return Some(status);
            }
            if left(until).is_zero() {
                return None;
            }
            sleep_until_at_most(until, Duration::from_millis(50));
        }
    }

    fn lingering(&self) -> Vec<Value> {
        let mut out: Vec<Value> = self
            .seen
            .iter()
            .filter(|p| p.exists())
            .map(ProcId::json)
            .collect();
        if self.status.is_none() {
            out.extend(
                self.launcher
                    .iter()
                    .filter(|l| l.exists())
                    .map(ProcId::json),
            );
        }
        out
    }

    /// 确认 launcher 已停或已退出后，再取稳定的直接子进程集合，与此前身份合并后按身份回收。
    fn reap(&mut self, until: Instant) {
        let mut freeze = json!({"needed":false});
        let mut scan = json!({"skipped":"launcher 已被 wait；后代由 subreaper 扫描核验"});
        if self.poll().is_none() {
            let rc = unsafe { libc::kill(self.child.id() as libc::pid_t, libc::SIGSTOP) };
            let freeze_until = (Instant::now() + OBS_FREEZE).min(until);
            let confirmed = loop {
                if self.poll().is_some() {
                    break "exited";
                }
                if self
                    .launcher
                    .as_ref()
                    .is_some_and(|l| l.state() == "stopped")
                {
                    break "stopped";
                }
                if left(freeze_until).is_zero() {
                    break "unconfirmed";
                }
                sleep_until_at_most(freeze_until, Duration::from_millis(10));
            };
            freeze = json!({"needed":true,"sigstop_rc":rc,"confirmed":confirmed});
            if self.status.is_none() {
                let (found, stable) = stable_children(self.child.id(), until);
                scan = json!({"stable":stable,"found":found.iter().map(ProcId::json).collect::<Vec<_>>()});
                self.merge(found);
            }
        }
        let signals: Vec<Value> = self
            .seen
            .iter()
            .filter(|p| matches!(p.state(), "alive" | "stopped"))
            .map(|p| signal_identity(p, libc::SIGKILL))
            .collect();
        let launcher_kill = self
            .status
            .is_none()
            .then(|| format!("{:?}", self.child.kill()));
        let status = self.wait_exit(until);
        self.reap = Some(json!({"freeze":freeze,"child_scan":scan,"signals":signals,
            "launcher_kill":launcher_kill,"launcher_status":status.map(status_record)}));
    }

    fn record(&self) -> Value {
        let stop = self
            .active_stop
            .as_ref()
            .and_then(|v| v["stderr_offset"].as_u64())
            .map(|n| n as usize);
        let (out, out_end) = self.stdout.frozen();
        let (err, err_end) = self.stderr.frozen();
        json!({"label":self.label,"launcher":self.launcher.as_ref().map(ProcId::json),
            "processes":self.seen.iter().map(ProcId::json).collect::<Vec<_>>(),
            "status":self.status.map(status_record),"active_stop":self.active_stop,"reap":self.reap,
            "stdout":capture_json(&out, out_end),"stderr":capture_json(&err, err_end),
            "error_timeline":error_timeline(&err, stop)})
    }
}

enum ReadyOutcome {
    Ready(Value),
    Exited(std::process::ExitStatus),
    Timeout,
}

fn probe_until(until: Instant) -> Instant {
    (Instant::now() + OBS_PROBE).min(until)
}

fn eb_ready(
    run: &Launched,
    space: &Space,
    until: Instant,
    probes: &mut Probes,
    budget: &Budget,
) -> bool {
    for (kind, sock) in [("connect_e", "e.sock"), ("connect_b", "b.sock")] {
        let r = unix_connect(&space.path(sock), probe_until(until)).map(drop);
        let ok = r.is_ok();
        probes.note(run, budget, kind, r);
        if !ok {
            return false;
        }
    }
    true
}

/// 四域就绪：S 经 `s_ready` 核身份；四域写锁经 `/proc/locks` 只读观察为已持有；X `x_meta.writer_epoch=1`；
/// E/B socket 可连。
/// 不以路径存在判 ready（残留路径会让它立即为真）。
fn wait_ready(
    run: &mut Launched,
    space: &Space,
    s_db: &Path,
    s_socket: &Path,
    until: Instant,
    probes: &mut Probes,
    budget: &Budget,
) -> ReadyOutcome {
    let mut s_reply: Option<Value> = None;
    loop {
        if let Some(status) = run.poll() {
            return ReadyOutcome::Exited(status);
        }
        let alive: Vec<String> = run
            .seen
            .iter()
            .filter(|p| p.state() == "alive")
            .map(|p| p.role.clone())
            .collect();
        if DOMAINS.iter().all(|d| alive.iter().any(|a| a == d)) {
            if s_reply.is_none() {
                match s_ready(s_socket, probe_until(until)) {
                    Ok(reply) if s_ready_matches(&reply) => {
                        probes.note(run, budget, "s_ready", Ok(()));
                        s_reply = Some(reply);
                    }
                    Ok(reply) => {
                        probes.note(run, budget, "s_ready", Err(format!("身份不匹配：{reply}")))
                    }
                    Err(e) => probes.note(run, budget, "s_ready", Err(e)),
                }
            }
            if let Some(reply) = &s_reply {
                let mut locks_held = true;
                for (domain, db) in [
                    ("s", s_db.to_path_buf()),
                    ("x", space.path("x.db")),
                    ("e", space.path("e.db")),
                    ("b", space.path("b.db")),
                ] {
                    match lock_observed(&db) {
                        Ok(true) => {}
                        Ok(false) => locks_held = false,
                        Err(e) => {
                            probes.note(
                                run,
                                budget,
                                "lock_observation",
                                Err(format!("{domain}: {e}")),
                            );
                            locks_held = false;
                        }
                    }
                }
                if locks_held
                    && x_epoch_ready(space, probe_until(until))
                    && eb_ready(run, space, until, probes, budget)
                {
                    return ReadyOutcome::Ready(reply.clone());
                }
            }
        }
        if left(until).is_zero() {
            return ReadyOutcome::Timeout;
        }
        sleep_until_at_most(until, Duration::from_millis(100));
    }
}

fn projection_section(
    fields: &[(&str, &str, &str)],
    db: &dyn Fn(&str) -> PathBuf,
    until: Instant,
) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|(name, domain, sql)| ((*name).to_string(), read_field(&db(domain), sql, until)))
            .collect(),
    )
}

fn compare_projection(baseline: &Value, after: &Value) -> Value {
    let mut out = serde_json::Map::new();
    for (section, fields) in [("data", &OBS_DATA), ("control", &OBS_CONTROL)] {
        let mut cmp = serde_json::Map::new();
        for (name, _, _) in fields.iter() {
            let (b, a) = (&baseline[section][*name], &after[section][*name]);
            let entry = if b.get("value").is_some() && a.get("value").is_some() {
                json!({"baseline":b["value"],"after":a["value"],"equal":b["value"] == a["value"]})
            } else {
                json!({"status":"not_observed","baseline":b,"after":a})
            };
            cmp.insert((*name).to_string(), entry);
        }
        out.insert(section.into(), Value::Object(cmp));
    }
    Value::Object(out)
}

/// 有界轮询等待直接子进程；到期返回 TIMEOUT，不发任何信号，回收由调用方负责。
fn wait_child(child: &mut Child, until: Instant) -> Result<std::process::ExitStatus, String> {
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Ok(status);
        }
        if left(until).is_zero() {
            return Err("TIMEOUT".into());
        }
        sleep_until_at_most(until, Duration::from_millis(50));
    }
}

struct Case {
    scenario: &'static str,
    budget: Budget,
    space: Space,
    s_db: PathBuf,
    socket: PathBuf,
    socket_class: &'static str,
    receipt: serde_json::Map<String, Value>,
    steps: Vec<Value>,
    snapshots: Vec<Value>,
    probes: Probes,
    l1: Option<Launched>,
    l2: Option<Launched>,
    baseline: Option<Value>,
    init_stderr: Option<Capture>,
    init_status: Option<Value>,
    precondition: Result<(), String>,
}

/// 设置并读回 subreaper；只在 helper 进程内调用，随 helper 退出而消失，不触及调用者进程。
fn enable_subreaper() -> (Value, Result<(), String>) {
    let set = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1 as libc::c_ulong) };
    let set_errno = (set != 0).then(|| std::io::Error::last_os_error().to_string());
    let mut flag: libc::c_int = 0;
    let get = unsafe { libc::prctl(libc::PR_GET_CHILD_SUBREAPER, &mut flag as *mut libc::c_int) };
    let get_errno = (get != 0).then(|| std::io::Error::last_os_error().to_string());
    let record =
        json!({"set_rc":set,"set_errno":set_errno,"get_rc":get,"get_errno":get_errno,"flag":flag});
    let ok = if set == 0 && get == 0 && flag == 1 {
        Ok(())
    } else {
        Err(format!("subreaper 未生效：{record}"))
    };
    (record, ok)
}

/// 启动任何服务前，在本进程自建的牺牲进程上实测回收所需的 pidfd SIGSTOP/SIGKILL 能力。
fn pidfd_precheck() -> (Value, Result<(), String>) {
    use std::os::unix::process::ExitStatusExt;
    let mut child = match Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let record = json!({"spawn_error": e.to_string()});
            return (record, Err("pidfd 前检无法拉起牺牲进程".into()));
        }
    };
    let until = Instant::now() + Duration::from_secs(5);
    let probe = ProcId::capture("pidfd_probe", child.id());
    let (stop, stopped, kill) = match &probe {
        None => (Value::Null, false, Value::Null),
        Some(p) => {
            let stop = signal_identity(p, libc::SIGSTOP);
            while p.state() != "stopped" && !left(until).is_zero() {
                sleep_until_at_most(until, Duration::from_millis(10));
            }
            let stopped = p.state() == "stopped";
            (stop, stopped, signal_identity(p, libc::SIGKILL))
        }
    };
    let fallback = (kill["delivered"] != true).then(|| format!("{:?}", child.kill()));
    let status = wait_child(&mut child, until + Duration::from_secs(2));
    let killed = status
        .as_ref()
        .is_ok_and(|s| s.signal() == Some(libc::SIGKILL));
    let ok = stop["delivered"] == true && stopped && kill["delivered"] == true && killed;
    let record = json!({"probe":probe.as_ref().map(ProcId::json),"sigstop":stop,"stopped_confirmed":stopped,
        "sigkill":kill,"fallback_kill":fallback,"status":format!("{status:?}"),"ok":ok});
    let result = if ok {
        Ok(())
    } else {
        Err(format!("pidfd 回收能力前检失败：{record}"))
    };
    (record, result)
}

impl Case {
    fn new(scenario: &'static str, s_socket: Option<&str>) -> Self {
        let budget = Budget::new();
        let space = Space::new();
        let s_db = space.path("s").join("s.db");
        let socket = space.path(s_socket.unwrap_or("s.sock"));
        let socket_class = match s_socket {
            None => "outside_M",
            Some(rel) if rel.contains("s-state") => "A_subtree",
            Some(_) => "basename_family",
        };
        let env = |k: &str| std::env::var(k).ok();
        let mut receipt = serde_json::Map::new();
        receipt.insert("scenario".into(), json!(scenario));
        receipt.insert(
            "build".into(),
            json!({"production_session_testonly": cfg!(feature = "production_session_testonly"),
                "github_sha":env("GITHUB_SHA"),"github_head_ref":env("GITHUB_HEAD_REF"),
                "run_url":env("GITHUB_RUN_ID").map(|id| format!("{}/{}/actions/runs/{id}",
                    env("GITHUB_SERVER_URL").unwrap_or_default(),env("GITHUB_REPOSITORY").unwrap_or_default())),
                "run_attempt":env("GITHUB_RUN_ATTEMPT"),"job":env("GITHUB_JOB"),
                "boot_id":std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map(|s| s.trim().to_string()).map_err(|e| e.to_string())}),
        );
        let (subreaper, subreaper_ok) = enable_subreaper();
        let (pidfd, pidfd_ok) = pidfd_precheck();
        let precondition = subreaper_ok.and(pidfd_ok);
        receipt.insert(
            "helper".into(),
            json!({"pid":std::process::id(),"subreaper":subreaper,"pidfd_precheck":pidfd}),
        );
        Self {
            scenario,
            budget,
            space,
            s_db,
            socket,
            socket_class,
            receipt,
            steps: Vec::new(),
            snapshots: Vec::new(),
            probes: Probes::default(),
            l1: None,
            l2: None,
            baseline: None,
            init_stderr: None,
            init_status: None,
            precondition,
        }
    }

    fn set(&mut self, key: &str, value: Value) {
        self.receipt.insert(key.into(), value);
    }

    fn step(&mut self, name: &str, detail: Value) {
        let t = self.budget.ms();
        self.steps
            .push(json!({"t_ms":t,"step":name,"detail":detail}));
    }

    fn snapshot(&mut self, when: &str) {
        let paths = [
            self.socket.clone(),
            self.space.path("e.sock"),
            self.space.path("b.sock"),
            self.space.path("s"),
            self.space.path("s/s-state"),
        ];
        let entries: Vec<Value> = paths
            .iter()
            .map(|p| json!({"lstat":lstat_record(p),"proc_net_unix":proc_unix_inode(p)}))
            .collect();
        let t = self.budget.ms();
        self.snapshots
            .push(json!({"when":when,"t_ms":t,"paths":entries}));
    }

    fn projection(&self, until: Instant) -> Value {
        let db = |domain: &str| {
            if domain == "s" {
                self.s_db.clone()
            } else {
                self.space.path(&format!("{domain}.db"))
            }
        };
        json!({"t_ms":self.budget.ms(),"data":projection_section(&OBS_DATA, &db, until),
            "control":projection_section(&OBS_CONTROL, &db, until)})
    }

    /// 与 `simulation` 同一夹具；S init 改为限时拉起，超时只杀本例 init 进程并在同一期限内等待。
    fn fixture(&mut self) -> Result<PathBuf, String> {
        let source = declaration("TestOnlyStructure", DECLARED_NS);
        let (mut init, db) = s_init_v3_command(&self.space, Some(&source));
        let mut child = init
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("S init 启动失败：{e}"))?;
        self.init_stderr = Some(Capture::start(child.stderr.take().unwrap()));
        let status = match wait_child(&mut child, self.budget.phase(OBS_INIT)) {
            Ok(status) => status,
            Err(_) => {
                let _ = child.kill();
                let reaped = wait_child(&mut child, self.budget.before_end(OBS_REAP_DONE));
                self.init_status = Some(json!({"timeout":true,"after_kill":format!("{reaped:?}")}));
                return Err("S init 超时".into());
            }
        };
        self.init_status = Some(status_record(status));
        if !status.success() {
            return Err(format!("S init 失败 {status}"));
        }
        let limits = anchor(&db);
        let xp = x_profile(
            &self.space,
            "simulation",
            None,
            Some("tb09-sentinel"),
            false,
        );
        let manifest = launcher_manifest(&self.space, "simulation", &source, &db, &limits, &xp);
        if self.socket_class != "outside_M" {
            let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
            m["s"]["socket"] = json!(self.socket);
            write_json(&manifest, &m);
        }
        let fixture = json!({"root":self.space.path(""),"manifest":std::fs::read_to_string(&manifest).unwrap(),
            "s_db":db,"s_socket":self.socket,"P":self.space.path("s"),"A":self.space.path("s/s-state"),
            "s_socket_class":self.socket_class,"profile":TB02A_PROFILE,"anchor":"TestOnly","credential":false});
        self.set("fixture", fixture);
        Ok(manifest)
    }
}

fn run_case(c: &mut Case) -> Result<(), String> {
    let manifest = c.fixture()?;
    c.snapshot("initial");
    c.l1 = Some(Launched::spawn("L1", &manifest));
    c.step("l1_spawned", Value::Null);
    let until = c.budget.phase(OBS_READY);
    let outcome = wait_ready(
        c.l1.as_mut().unwrap(),
        &c.space,
        &c.s_db,
        &c.socket,
        until,
        &mut c.probes,
        &c.budget,
    );
    let ready = match outcome {
        ReadyOutcome::Ready(reply) => reply,
        ReadyOutcome::Exited(status) => {
            c.step("l1_exited_before_ready", status_record(status));
            c.set("first_start", json!({"ready":false}));
            return Ok(());
        }
        ReadyOutcome::Timeout => return Err("L1 首启 30s 内未就绪".into()),
    };
    c.baseline = Some(c.projection(c.budget.phase(OBS_PROBE)));
    c.set("first_start", json!({"ready":true,"s_ready_reply":ready}));
    c.snapshot("before_kill");

    let l1 = c.l1.as_mut().unwrap();
    let s = l1
        .seen
        .iter()
        .find(|p| p.role == "s_structure_session" && p.state() == "alive")
        .cloned()
        .ok_or("L1 名下找不到存活的 S")?;
    l1.active_stop = Some(json!({"action":"SIGKILL S","stderr_offset":l1.stderr.len()}));
    let kill = signal_identity(&s, libc::SIGKILL);
    let delivered = kill["delivered"] == true;
    c.set("sigkill", kill);
    if !delivered {
        return Err("SIGKILL S 未送达".into());
    }
    c.step("s_sigkilled", Value::Null);
    let until = c.budget.phase(OBS_L1_REAP);
    let l1 = c.l1.as_mut().unwrap();
    let Some(status) = l1.wait_exit(until) else {
        return Err("SIGKILL S 后 L1 30s 内未退出".into());
    };
    let lingering = l1.lingering();
    c.step("l1_exited", status_record(status));
    let adopted = reap_adopted(&[], until);
    let adopted_any = adopted["signals"].as_array().is_some_and(|a| !a.is_empty());
    let adopted_open = adoption_open(&adopted);
    c.set("l1_adopted_after_exit", adopted);
    if !lingering.is_empty() || adopted_any || adopted_open {
        return Err(format!("L1 退出后仍有本例后代存在或被托管：{lingering:?}"));
    }
    c.snapshot("after_l1");

    c.l2 = Some(Launched::spawn("L2", &manifest));
    c.step("l2_spawned", Value::Null);
    let until = c.budget.phase(OBS_L2);
    let outcome = wait_ready(
        c.l2.as_mut().unwrap(),
        &c.space,
        &c.s_db,
        &c.socket,
        until,
        &mut c.probes,
        &c.budget,
    );
    match outcome {
        ReadyOutcome::Ready(reply) => {
            c.set("restarted_ready", json!(true));
            c.set("l2_s_ready_reply", reply);
            let until = c.budget.phase(OBS_STOP);
            let l2 = c.l2.as_mut().unwrap();
            l2.active_stop =
                Some(json!({"action":"SIGTERM launcher","stderr_offset":l2.stderr.len()}));
            unsafe { libc::kill(l2.child.id() as libc::pid_t, libc::SIGTERM) };
            if l2.wait_exit(until).is_none() {
                return Err("L2 就绪后 SIGTERM 未在限时内停止".into());
            }
            c.step("l2_stopped", Value::Null);
        }
        ReadyOutcome::Exited(status) => {
            c.set("restarted_ready", json!(false));
            c.step("l2_exited", status_record(status));
        }
        ReadyOutcome::Timeout => {
            c.set("restarted_ready", json!(false));
            return Err("L2 60s 内既未就绪也未退出".into());
        }
    }
    Ok(())
}

fn finalize(mut c: Case, result: Result<(), String>) {
    let mut failures: Vec<String> = result.err().into_iter().collect();
    let reap_until = c.budget.before_end(OBS_REAP_DONE);
    for run in [c.l2.as_mut(), c.l1.as_mut()].into_iter().flatten() {
        run.reap(reap_until);
    }
    let exclude: Vec<u32> = [c.l1.as_ref(), c.l2.as_ref()]
        .into_iter()
        .flatten()
        .filter(|r| r.status.is_none())
        .map(|r| r.child.id())
        .collect();
    let adopted = reap_adopted(&exclude, reap_until);
    if adoption_open(&adopted) {
        failures.push(format!(
            "subreaper 下仍有未收割或归属丢失的本例后代：unreaped={} lost={}",
            adopted["unreaped"], adopted["lost"]
        ));
    }
    let lingering: Vec<Value> = [c.l1.as_ref(), c.l2.as_ref()]
        .into_iter()
        .flatten()
        .flat_map(Launched::lingering)
        .collect();
    if !lingering.is_empty() {
        failures.push(format!("本例进程未闭合：{lingering:?}"));
    }
    let freeze_unconfirmed = [c.l1.as_ref(), c.l2.as_ref()]
        .into_iter()
        .flatten()
        .any(|r| {
            r.reap
                .as_ref()
                .is_some_and(|v| v["freeze"]["confirmed"] == "unconfirmed")
        });
    if freeze_unconfirmed {
        failures.push("launcher 冻结未确认，子进程集合可能不完整".into());
    }
    let settle_until = c.budget.before_end(OBS_SETTLE_DONE);
    for run in [c.l1.as_mut(), c.l2.as_mut()].into_iter().flatten() {
        run.stdout.settle(settle_until);
        run.stderr.settle(settle_until);
    }
    if let Some(init) = c.init_stderr.as_mut() {
        init.settle(settle_until);
    }
    let captures_complete = [c.l1.as_ref(), c.l2.as_ref()]
        .into_iter()
        .flatten()
        .all(|r| r.stdout.complete() && r.stderr.complete())
        && c.init_stderr.as_ref().is_none_or(Capture::complete);
    if !captures_complete {
        failures.push("已启动阶段的输出捕获出现读错误或未闭合".into());
    }
    let init = match &c.init_stderr {
        Some(capture) => {
            let (bytes, end) = capture.frozen();
            json!({"status":c.init_status,"stderr":capture_json(&bytes, end)})
        }
        None => json!({"status":"NOT_REACHED"}),
    };
    c.set("init", init);
    c.snapshot("final");
    let after = if lingering.is_empty() {
        c.projection(c.budget.before_end(OBS_READ_DONE))
    } else {
        json!({"not_observed":"仍有本例进程存在，未读持久状态"})
    };
    let baseline = c
        .baseline
        .clone()
        .unwrap_or_else(|| json!({"not_observed":"L1 未就绪或未执行到基线读取，未取得基线"}));
    let comparison = compare_projection(&baseline, &after);
    c.set(
        "projection",
        json!({"baseline":baseline,"after":after,"comparison":comparison}),
    );
    let records: Vec<(&str, Value)> = [("l1", &c.l1), ("l2", &c.l2)]
        .into_iter()
        .map(|(key, run)| {
            (
                key,
                run.as_ref().map_or_else(
                    || json!({"status":"NOT_REACHED","reason":"未拉起"}),
                    Launched::record,
                ),
            )
        })
        .collect();
    for (key, value) in records {
        c.set(key, value);
    }
    let reason = failures
        .first()
        .cloned()
        .unwrap_or_else(|| "L1 未就绪即退出".to_string());
    if !c.receipt.contains_key("sigkill") {
        c.set("sigkill", json!({"status":"NOT_REACHED","reason":reason}));
    }
    if c.l2.is_none() {
        c.set("restart", json!({"status":"NOT_REACHED","reason":reason}));
    }
    if !c.receipt.contains_key("restarted_ready") {
        c.set("restarted_ready", json!(false));
    }
    let probes = json!({"total":c.probes.total,"logged":c.probes.log});
    c.set("probes", probes);
    let (steps, snapshots) = (json!(c.steps), json!(c.snapshots));
    c.set("steps", steps);
    c.set("snapshots", snapshots);
    c.set("reap", json!({"adopted":adopted,"lingering":lingering}));
    let remaining: Vec<Value> = descendants(std::process::id())
        .iter()
        .map(ProcId::json)
        .collect();
    if !remaining.is_empty() {
        failures.push("helper 名下仍有本例后代，转交父测试回收".into());
    }
    c.set("helper_descendants_at_end", json!(remaining));
    if c.budget.start.elapsed() > OBS_HELPER_CASE {
        failures.push("helper 用例超过预算".into());
    }
    let complete = failures.is_empty()
        && captures_complete
        && c.receipt
            .get("first_start")
            .is_some_and(|f| f["ready"] == true)
        && c.receipt
            .get("sigkill")
            .is_some_and(|k| k["delivered"] == true)
        && c.l2.as_ref().is_some_and(|r| r.status.is_some());
    c.set(
        "observation_complete",
        json!({"value":complete,"captures_complete":captures_complete}),
    );
    c.set("harness_failures", json!(failures));
    c.set("elapsed_ms", json!(c.budget.ms()));
    let line = format!(
        "socket_residue_observe_receipt {}\n",
        Value::Object(c.receipt.clone())
    );
    std::io::stderr().write_all(line.as_bytes()).unwrap();
    if !remaining.is_empty() {
        // helper 是本例后代唯一的托管者；先退出会让它们改挂到别处而失去归属，故留在原地交给父测试冻结回收。
        let handoff = json!({"scenario":c.scenario,"remaining":remaining});
        std::io::stderr()
            .write_all(format!("socket_residue_observe_handoff {handoff}\n").as_bytes())
            .unwrap();
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    if !failures.is_empty() {
        panic!("{}：{}", c.scenario, failures.join("；"));
    }
}

fn observe_sigkill_restart(scenario: &'static str, s_socket: Option<&str>) {
    assert!(
        std::env::var_os(OBS_HELPER_ENV).is_some(),
        "观察用例只能在 helper 进程内执行"
    );
    let mut c = Case::new(scenario, s_socket);
    let result = match c.precondition.clone() {
        Err(why) => Err(why),
        Ok(()) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_case(&mut c)))
            .unwrap_or_else(|panic| {
                let text = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                Err(format!("harness panic：{text}"))
            }),
    };
    finalize(c, result);
}

/// 以 `root` 为根、按 PPID 广度遍历的后代身份（不含 root）。
fn descendants(root: u32) -> Vec<ProcId> {
    let mut out: Vec<ProcId> = Vec::new();
    let mut queue = vec![root];
    while let Some(pid) = queue.pop() {
        for child in children_of(pid) {
            if !out.contains(&child) {
                queue.push(child.pid);
                out.push(child);
            }
        }
    }
    out
}

/// helper 内的排空处理器：收割 helper 名下全部已终结孩子。只在父测试接管回收时由父测试触发；
/// 此时本例已判失败，抢走 launcher 句柄的退出状态无碍。
extern "C" fn drain_children(_: libc::c_int) {
    unsafe {
        let saved = *libc::__errno_location();
        while libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) > 0 {}
        *libc::__errno_location() = saved;
    }
}

fn install_drain_handler() -> Result<(), String> {
    let prev = unsafe {
        libc::signal(
            OBS_DRAIN_SIGNAL,
            drain_children as *const () as libc::sighandler_t,
        )
    };
    if prev == libc::SIG_ERR {
        Err(format!(
            "排空处理器安装失败：{}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

/// 按身份 SIGSTOP 并确认进入停止态。
fn freeze_identity(p: &ProcId, until: Instant) -> (Value, bool) {
    let sent = signal_identity(p, libc::SIGSTOP);
    while p.state() != "stopped" && !left(until).is_zero() {
        sleep_until_at_most(until, Duration::from_millis(10));
    }
    let state = p.state();
    (json!({"signal":sent,"state":state}), state == "stopped")
}

/// 冻结 `root` 的全部后代：反复 SIGSTOP 存活成员，直到两次遍历一致且没有仍在运行的成员。
fn freeze_descendants(root: u32, until: Instant) -> (Vec<ProcId>, bool) {
    loop {
        let before = descendants(root);
        for p in before.iter().filter(|p| p.state() == "alive") {
            signal_identity(p, libc::SIGSTOP);
        }
        sleep_until_at_most(until, Duration::from_millis(20));
        let after = descendants(root);
        let settled = after.iter().all(|p| p.state() != "alive");
        if after == before && settled {
            return (after, true);
        }
        if left(until).is_zero() {
            return (after, false);
        }
    }
}

/// 父测试接管 helper 树。helper 是本例后代唯一的托管者，必须最后一个死：
/// 每轮先冻结 helper（它停着就不能再拉起进程），再冻结并遍历它的后代；后代非空则按身份 SIGKILL，
/// 然后 SIGCONT helper 并发排空信号让它收割僵尸。直到 helper 停止态下后代为空，才按身份杀 helper 并 wait。
/// 僵尸不算已回收；期限内收不干净就如实列在 `open`。
fn reap_helper_tree(child: &mut Child, helper: Option<&ProcId>, until: Instant) -> Value {
    let Some(helper) = helper else {
        let kill = format!("{:?}", child.kill());
        let status = wait_child(child, until);
        return json!({"closed":false,"reason":"未取得 helper 身份，无法安全接管后代",
            "helper_kill":kill,"helper_status":format!("{status:?}"),"members":[],"open":[]});
    };
    let mut members: Vec<ProcId> = Vec::new();
    let mut rounds = Vec::new();
    let (closed, reason) = loop {
        let (freeze, stopped) = freeze_identity(helper, until);
        if !stopped {
            break (false, format!("helper 未能确认停止：{freeze}"));
        }
        let (desc, stable) = freeze_descendants(helper.pid, until);
        for p in &desc {
            if !members.contains(p) {
                members.push(p.clone());
            }
        }
        let states: Vec<Value> = desc.iter().map(ProcId::json).collect();
        if desc.is_empty() && stable {
            rounds.push(json!({"helper_freeze":freeze,"descendants":states}));
            break (true, "helper 停止态下后代为空".into());
        }
        let kills: Vec<Value> = desc
            .iter()
            .filter(|p| matches!(p.state(), "alive" | "stopped"))
            .map(|p| signal_identity(p, libc::SIGKILL))
            .collect();
        let cont = signal_identity(helper, libc::SIGCONT);
        let drain = signal_identity(helper, OBS_DRAIN_SIGNAL);
        rounds.push(
            json!({"helper_freeze":freeze,"descendants":states,"stable":stable,
            "kills":kills,"helper_cont":cont,"drain":drain}),
        );
        if left(until).is_zero() {
            break (false, "期限内未能在 helper 停止态下见到空后代集".into());
        }
        sleep_until_at_most(until, Duration::from_millis(50));
    };
    let helper_kill = signal_identity(helper, libc::SIGKILL);
    let helper_status = wait_child(child, until);
    let open: Vec<Value> = members
        .iter()
        .filter(|p| p.exists())
        .map(ProcId::json)
        .collect();
    json!({"closed":closed && open.is_empty() && helper_status.is_ok(),"reason":reason,
        "rounds":rounds,"helper_kill":helper_kill,"helper_status":format!("{helper_status:?}"),
        "members":members.iter().map(ProcId::json).collect::<Vec<_>>(),"open":open})
}

/// 拉起并看管一个 helper；父测试自身不设 subreaper，不回收 helper 树以外的任何进程。返回包装收据与失败项。
fn run_helper(scenario: &str, wait: Duration) -> (Value, Vec<String>) {
    assert!(
        std::env::var_os(OBS_HELPER_ENV).is_none(),
        "helper 内不得再起 helper"
    );
    let start = Instant::now();
    let end = start + OBS_CASE;
    let mut failures: Vec<String> = Vec::new();
    let (precheck, precheck_ok) = pidfd_precheck();
    if let Err(why) = precheck_ok {
        failures.push(format!("父测试 pidfd 前检失败，未拉起 helper：{why}"));
        let wrapper = json!({"scenario":scenario,"parent_pidfd_precheck":precheck,"helper":"NOT_SPAWNED","failures":failures});
        return (wrapper, failures);
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            OBS_HELPER_TEST,
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env(OBS_HELPER_ENV, scenario)
        .env_remove(CREDENTIAL_KEY)
        .env_remove(THETA_DIAGNOSTIC.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let helper = ProcId::capture("helper", child.id());
    let mut out = Capture::start(child.stdout.take().unwrap());
    let mut err = Capture::start(child.stderr.take().unwrap());
    let wait_until = (start + wait).min(start + OBS_HELPER_WAIT);
    let (exited, handoff) = loop {
        if let Some(status) = child.try_wait().ok().flatten() {
            break (Some(status), false);
        }
        if err.contains(b"socket_residue_observe_handoff ") {
            break (None, true);
        }
        if left(wait_until).is_zero() {
            break (None, false);
        }
        sleep_until_at_most(wait_until, Duration::from_millis(50));
    };
    let (status, tree) = match exited {
        Some(status) => (Some(status), Value::Null),
        None => {
            failures.push(if handoff {
                OBS_TRIGGER_HANDOFF.into()
            } else {
                OBS_TRIGGER_TIMEOUT.into()
            });
            let tree = reap_helper_tree(&mut child, helper.as_ref(), end - OBS_HELPER_REAP_DONE);
            if tree["closed"] != true {
                failures.push(OBS_TREE_OPEN.into());
            }
            (child.try_wait().ok().flatten(), tree)
        }
    };
    out.settle(end - OBS_HELPER_SETTLE_DONE);
    err.settle(end - OBS_HELPER_SETTLE_DONE);
    let (out_bytes, out_end) = out.frozen();
    let (err_bytes, err_end) = err.frozen();
    std::io::stderr().write_all(&err_bytes).unwrap();
    let receipts: Vec<Value> = String::from_utf8_lossy(&err_bytes)
        .lines()
        .filter_map(|l| l.strip_prefix("socket_residue_observe_receipt "))
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    match status {
        Some(s) if s.success() => {}
        other => failures.push(format!("helper 退出状态 {other:?}")),
    }
    if receipts.len() != 1 {
        failures.push(format!("helper 收据条数 {}（须为 1）", receipts.len()));
    }
    if receipts.iter().any(|r| {
        r["harness_failures"]
            .as_array()
            .is_none_or(|a| !a.is_empty())
    }) {
        failures.push("helper 收据含 harness 失败".into());
    }
    if receipts
        .iter()
        .any(|r| r["observation_complete"]["captures_complete"] != true)
    {
        failures.push("helper 收据显示已启动阶段的输出捕获未闭合".into());
    }
    if !(out.complete() && err.complete()) {
        failures.push("helper 输出捕获未闭合".into());
    }
    if start.elapsed() > OBS_CASE {
        failures.push("用例超过 150s 上限".into());
    }
    let wrapper = json!({"scenario":scenario,"parent_pidfd_precheck":precheck,"handoff":handoff,
        "helper":helper.as_ref().map(ProcId::json),
        "status":status.map(status_record),"tree_reap":tree,
        "helper_stdout":capture_json(&out_bytes, out_end),
        "helper_stderr":{"bytes":err_bytes.len(),"end":err_end,"sha256":sha256_hex(&err_bytes),"relayed_raw_above":true},
        "receipts":receipts.len(),"failures":failures,"elapsed_ms":start.elapsed().as_millis() as u64});
    std::io::stderr()
        .write_all(format!("socket_residue_observe_helper {wrapper}\n").as_bytes())
        .unwrap();
    (wrapper, failures)
}

fn observe_in_helper(scenario: &'static str) {
    let (_, failures) = run_helper(scenario, OBS_HELPER_WAIT);
    if !failures.is_empty() {
        panic!("{scenario}：{}", failures.join("；"));
    }
}

/// 回收自检用的故障 helper：留下一个直接孩子和一个托管孙进程（sh 退出后改挂 helper），不自行回收。
/// HANDOFF 写交接行后挂起；TIMEOUT 什么也不写直接挂起。
fn fault_helper(scenario: &str) {
    enable_subreaper()
        .1
        .expect("故障 helper 须先确认 subreaper");
    let quiet = |c: &mut Command| {
        c.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    };
    let mut direct = Command::new("sleep");
    direct.arg("60");
    quiet(&mut direct);
    drop(direct.spawn().unwrap());
    let mut orphaner = Command::new("sh");
    orphaner.args(["-c", "sleep 60 & exit 0"]);
    quiet(&mut orphaner);
    assert!(orphaner.status().unwrap().success());
    let deadline = Instant::now() + Duration::from_secs(5);
    while descendants(std::process::id()).len() < 2 && !left(deadline).is_zero() {
        sleep_until_at_most(deadline, Duration::from_millis(20));
    }
    let kids: Vec<Value> = descendants(std::process::id())
        .iter()
        .map(ProcId::json)
        .collect();
    let mut line = format!(
        "socket_residue_harness_fault {}\n",
        json!({"scenario":scenario,"left_behind":kids})
    );
    if scenario == "FAULT-HANDOFF" {
        line.push_str(&format!(
            "socket_residue_observe_handoff {}\n",
            json!({"scenario":scenario,"remaining":kids})
        ));
    }
    std::io::stderr().write_all(line.as_bytes()).unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// 故障自检：只接受本场景预期的触发原因，以及故障 helper 被杀、未写收据带来的两条必然失败；
/// 回收未闭合、捕获未闭合、超时上限或另一种触发原因一律判红。
fn assert_parent_takes_over(scenario: &str, wait: Duration, expect_handoff: bool) {
    let (wrapper, failures) = run_helper(scenario, wait);
    assert_eq!(
        wrapper["handoff"],
        json!(expect_handoff),
        "{scenario} 触发方式不符：{wrapper}"
    );
    let (trigger, other) = if expect_handoff {
        (OBS_TRIGGER_HANDOFF, OBS_TRIGGER_TIMEOUT)
    } else {
        (OBS_TRIGGER_TIMEOUT, OBS_TRIGGER_HANDOFF)
    };
    assert!(
        failures.iter().any(|f| f == trigger),
        "{scenario} 缺预期触发原因：{failures:?}"
    );
    assert!(!failures.iter().any(|f| f == other), "{failures:?}");
    for f in &failures {
        assert!(
            f == trigger || f.starts_with("helper 退出状态") || f == "helper 收据条数 0（须为 1）",
            "{scenario} 出现非预期失败：{f}；全部：{failures:?}"
        );
    }
    let tree = &wrapper["tree_reap"];
    assert_eq!(tree["closed"], true, "{scenario} 树未闭合：{wrapper}");
    let others: Vec<&Value> = tree["members"].as_array().unwrap().iter().collect();
    assert!(
        others.len() >= 2,
        "{scenario} 应接管故障 helper 留下的两个后代：{wrapper}"
    );
    for m in others {
        assert!(
            matches!(m["state"].as_str(), Some("gone" | "pid_reused")),
            "{scenario} 后代须已被收割而非留作僵尸：{m}"
        );
    }
    assert_eq!(tree["open"], json!([]), "{wrapper}");
}

/// 交接分支：helper 声明后代未闭合并挂起，父测试冻结、按身份杀后代、让 helper 收割僵尸，最后才杀 helper。
#[test]
fn socket_residue_harness_handoff_reaps_descendants_before_helper() {
    assert_parent_takes_over("FAULT-HANDOFF", OBS_HELPER_WAIT, true);
}

/// 超时分支：helper 不写交接行也不退出，父测试到期后走同一接管路径。
#[test]
fn socket_residue_harness_timeout_reaps_descendants_before_helper() {
    assert_parent_takes_over("FAULT-TIMEOUT", Duration::from_secs(3), false);
}

/// 同一进程从存活变僵尸后 cmdline 变空、role 随之改变，身份仍须相等；重复观察只入队一次，
/// 第一次 waitpid 收割后第二次得 ECHILD 且身份已不存在，归为 EchildGone 而非未回收或归属丢失。
#[test]
fn proc_identity_survives_zombie_transition_and_reap_is_single() {
    /// 断言失败时也回收本测试的 sleep；本测试自己 waitpid 收割后取走句柄，不再对已收割 PID 发信号。
    struct Sleeper(Option<Child>);
    impl Drop for Sleeper {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let cmdline =
        |pid: u32| std::fs::read(format!("/proc/{pid}/cmdline")).map_err(|e| e.to_string());
    let mut sleeper = Sleeper(Some(
        Command::new("sleep")
            .arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    ));
    let pid = sleeper.0.as_ref().unwrap().id();
    // spawn 返回时 execve 可能尚未把新 argv 装好，cmdline 会短暂为空；有界等到 argv[0] 可见。
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    loop {
        let raw = cmdline(pid);
        if raw
            .as_ref()
            .is_ok_and(|b| b.split(|c| *c == 0).next() == Some(&b"sleep"[..]))
        {
            break;
        }
        if seen.last() != Some(&raw) {
            seen.push(raw);
        }
        assert!(
            !left(deadline).is_zero(),
            "sleep 的 cmdline 5s 内未可见：{seen:?}"
        );
        sleep_until_at_most(deadline, Duration::from_millis(10));
    }
    let live = ProcId::capture(&exe_name(pid), pid).unwrap();
    assert_eq!(live.role, "sleep");
    assert_eq!(try_reap(&live), ReapOutcome::Running);
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    let deadline = Instant::now() + Duration::from_secs(5);
    while live.state() != "zombie" {
        assert!(!left(deadline).is_zero(), "SIGKILL 后未进入僵尸态");
        sleep_until_at_most(deadline, Duration::from_millis(10));
    }
    assert_eq!(cmdline(pid), Ok(Vec::new()), "僵尸的 cmdline 应可读且为空");
    let zombie = ProcId::capture(&exe_name(pid), pid).unwrap();
    assert_eq!(zombie.role, "");
    assert_eq!(live, zombie, "身份只看 pid+starttime");
    let mut pending = vec![live.clone()];
    if !pending.contains(&zombie) {
        pending.push(zombie.clone());
    }
    assert_eq!(pending.len(), 1, "同一身份不得重复入队");
    assert!(matches!(try_reap(&zombie), ReapOutcome::Reaped(_)));
    // 已由本测试收割：丢弃句柄（Child 的 drop 不发信号），守卫不再触碰该 PID。
    drop(sleeper.0.take());
    assert_eq!(try_reap(&live), ReapOutcome::EchildGone);
}

#[test]
#[ignore = "仅由 socket_residue_observe_* / socket_residue_harness_* 以独立 helper 进程调用"]
fn socket_residue_observe_helper() {
    let Ok(scenario) = std::env::var(OBS_HELPER_ENV) else {
        return;
    };
    install_drain_handler().expect("helper 须先装好排空处理器");
    if scenario.starts_with("FAULT-") {
        fault_helper(&scenario);
        return;
    }
    let (name, socket) = OBS_SCENARIOS
        .iter()
        .copied()
        .find(|(name, _)| *name == scenario)
        .expect("未知观察场景");
    observe_sigkill_restart(name, socket);
}

#[test]
fn socket_residue_observe_sigkill_restart_outside_m() {
    observe_in_helper("OBS-OUT");
}

#[test]
fn socket_residue_observe_invalid_layout_a_subtree() {
    observe_in_helper("OBS-IN-A");
}

#[test]
fn socket_residue_observe_invalid_layout_basename_family() {
    observe_in_helper("OBS-IN-BASENAME");
}
