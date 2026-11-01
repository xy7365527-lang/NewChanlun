//! #1769（ADR 0039 迁移步骤 1）：唯一生产 launcher 可运行骨架。
//!
//! 职责只有编排：校验 manifest（含结构源声明）→ 必要时 init 各域库 → 以独立进程拉起
//! S、E、每个 Bᵢ、X → 监督 → 收到 SIGTERM/SIGINT 时回收全部子进程。各域进程自持库、
//! `<db>.writer.lock` 与 `writer_epoch`；launcher 不打开任何域库，不放宽其锁/epoch/隔离。
//!
//! - 结构源违规（类别不在封闭集合、namespace 与执行能力 manifest 相交）在拉起 S 之前以
//!   `InvalidDomain` 退出，S 库文件不创建；既有 S 库由 S 持锁自核声明，失败按 critical exit 回收会话。
//! - 凭据按 X 执行 profile 的 `credential_scope` 读文件，只注入 X 子进程环境；S/E/B 子进程
//!   以清空的环境启动，launcher 自身环境不写入凭据。
//! - X 退出只记录，不停 S/E/B；S/E/B 任一退出视为会话失败，回收其余进程后非 0 退出。
//! - 日志经 `redact`（移植 `381fda3f` `trading_system/execution/hl_paper_launcher.py:46-53`
//!   `_redact`）过滤后写 stderr。
//!
//! 用法：production_session --manifest FILE
use newchan_rust::structure_source::Declaration;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const MANIFEST_REVISION: &str = "production-session-manifest/1";
const PROFILES: [&str; 4] = ["simulation", "dry-run", "experiment", "production"];
const STOP_GRACE: Duration = Duration::from_secs(10);
/// S 子进程从 launcher 环境放回的 admission 声明变量；凭据不在其中。
#[cfg(not(feature = "production_session_testonly"))]
const S_PASS: [&str; 2] = ["S_CONTROL_ADMISSION_SHA256", "S_CONTROL_ADMISSION_IDENTITY"];
/// #1769 TestOnly 构建：额外转交 S 的诊断 theta 开关；生产构建不转交（S 侧亦拒绝）。
#[cfg(feature = "production_session_testonly")]
const S_PASS: [&str; 3] = [
    "S_CONTROL_ADMISSION_SHA256",
    "S_CONTROL_ADMISSION_IDENTITY",
    "S_THETA_DIAGNOSTIC_TESTONLY",
];

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_stop(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

/// 与 `_redact` 同口径：敏感键名的值整体遮蔽；24 位以上纯 token 字符的字符串遮蔽。
fn redact(value: &Value) -> Value {
    fn secret_key(key: &str) -> bool {
        let k: String = key
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        let raw = key.to_ascii_lowercase();
        [
            "privatekey",
            "secret",
            "password",
            "token",
            "credential",
            "apikey",
        ]
        .iter()
        .any(|w| k.contains(w) || raw.contains(w))
            && !key.ends_with("_env")
    }
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let v = if secret_key(k) {
                        json!("[REDACTED]")
                    } else {
                        redact(v)
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        Value::String(s)
            if s.len() >= 24
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
        {
            json!("[REDACTED]")
        }
        other => other.clone(),
    }
}

fn log(event: &str, fields: Value) {
    let mut line = json!({"event": event});
    if let (Value::Object(target), Value::Object(extra)) = (&mut line, fields) {
        target.extend(extra);
    }
    eprintln!("production_session {}", redact(&line));
}

fn text<'a>(v: &'a Value, path: &str) -> Result<&'a str, String> {
    let mut node = v;
    for part in path.split('.') {
        node = &node[part];
    }
    node.as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("InvalidDomain：manifest 缺 {path}"))
}

fn read_json(path: &Path, what: &str) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("InvalidDomain：读{what} {} 失败：{e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("InvalidDomain：{what} JSON 解析失败：{e}"))
}

struct Manifest {
    raw: Value,
    profile: String,
    state_dir: PathBuf,
    source: Declaration,
    x_profile: PathBuf,
    credential: Option<(String, String)>,
}

/// 完整校验 manifest；此阶段不创建任何域文件。
fn load_manifest(path: &Path) -> Result<Manifest, String> {
    let raw = read_json(path, "manifest")?;
    if raw["manifest_revision"].as_str() != Some(MANIFEST_REVISION) {
        return Err(format!(
            "VersionUnsupported：manifest_revision 必须是 {MANIFEST_REVISION}"
        ));
    }
    let profile = text(&raw, "profile")?.to_string();
    if !PROFILES.contains(&profile.as_str()) {
        return Err(format!("InvalidDomain：未知运行 profile `{profile}`"));
    }
    let source = Declaration::parse(&raw["structure_source"])?;
    let execution: BTreeSet<String> = raw["execution_capabilities"]["namespaces"]
        .as_array()
        .ok_or("InvalidDomain：manifest 缺 execution_capabilities.namespaces")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or("InvalidDomain：执行能力 namespace 必须是字符串".to_string())
        })
        .collect::<Result<_, _>>()?;
    source.check_disjoint_from_execution(&execution)?;
    for key in [
        "s.db",
        "s.session",
        "s.catalog",
        "s.profile",
        "s.clock_plan",
        "s.session_generation",
        "s.delivery_retain_generations",
        "s.admission_job_limits",
        "s.writer_epoch",
        "s.socket",
        "s.resource_config",
        "economic.manifest",
        "economic.e.db",
        "economic.e.epoch",
        "x.db",
        "x.profile",
        "x.writer_epoch",
    ] {
        text(&raw, key)?;
    }
    let chongs = raw["economic"]["b"]
        .as_array()
        .filter(|b| !b.is_empty())
        .ok_or("InvalidDomain：manifest economic.b 至少一个 B")?;
    for b in chongs {
        for key in ["chong", "db", "epoch"] {
            text(b, key)?;
        }
    }
    let x_profile = PathBuf::from(text(&raw, "x.profile")?);
    let xp = read_json(&x_profile, "X 执行 profile")?;
    if xp["mode"].as_str() != Some(profile.as_str()) {
        return Err(format!(
            "InvalidDomain：X 执行 profile mode `{}` 与运行 profile `{profile}` 不一致",
            xp["mode"]
        ));
    }
    let credential_env = text(&xp, "credential_scope.env")?.to_string();
    // 凭据文件缺失不在此判错：是否必需由 X 按 profile 裁决（MissingDependency）。
    let credential = xp["credential_scope"]["file"]
        .as_str()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|value| (credential_env, value));
    Ok(Manifest {
        state_dir: PathBuf::from(text(&raw, "state_dir")?),
        raw,
        profile,
        source,
        x_profile,
        credential,
    })
}

fn sibling(name: &str) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("MissingDependency：{e}"))?;
    let path = exe
        .parent()
        .ok_or("MissingDependency：launcher 无父目录")?
        .join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("MissingDependency：缺域进程 {}", path.display()))
    }
}

/// 清空环境后只放回 PATH 与显式白名单；凭据不在任何白名单里。
fn clean(mut command: Command, pass: &[&str]) -> Command {
    command.env_clear();
    for key in std::iter::once("PATH").chain(pass.iter().copied()) {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.stdin(Stdio::null());
    command
}

fn run_once(mut command: Command, what: &str) -> Result<(), String> {
    let out = command
        .output()
        .map_err(|e| format!("MissingDependency：{what} 启动失败：{e}"))?;
    if out.status.success() {
        log("init", json!({"domain": what}));
        Ok(())
    } else {
        Err(format!(
            "{}（{what} init 失败，{}）",
            String::from_utf8_lossy(&out.stderr).trim(),
            out.status
        ))
    }
}

struct Supervised {
    domain: String,
    child: Child,
    status: Option<ExitStatus>,
    critical: bool,
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let manifest_path = match args.as_slice() {
        [flag, path] if flag == "--manifest" => PathBuf::from(path),
        _ => return Err("InvalidDomain：usage: production_session --manifest FILE".into()),
    };
    let m = load_manifest(&manifest_path)?;
    let raw = &m.raw;
    let s_db = PathBuf::from(text(raw, "s.db")?);
    let (s_bin, e_bin, b_bin, x_bin) = (
        sibling("s_structure_session")?,
        sibling("e_economic_session")?,
        sibling("b_chong_session")?,
        sibling("x_execution_session")?,
    );
    log(
        "start",
        json!({"profile": m.profile, "structure_source": serde_json::from_str::<Value>(&m.source.canonical()).unwrap_or_default()}),
    );
    // SAFETY：只登记写原子标志的 async-signal-safe 处理器。
    unsafe {
        libc::signal(libc::SIGTERM, on_stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_stop as *const () as libc::sighandler_t);
    }

    // ── init：只对尚不存在的库；S 一律带声明 init ──
    std::fs::create_dir_all(&m.state_dir).map_err(|e| format!("StorageUnavailable：{e}"))?;
    let decl = m.state_dir.join("structure-source.json");
    std::fs::write(&decl, m.source.canonical())
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    if !s_db.exists() {
        let mut c = clean(Command::new(&s_bin), &S_PASS);
        c.args(["init", "--protocol-revision", "s-session/3", "--db"])
            .arg(&s_db)
            .args(["--session", text(raw, "s.session")?])
            .args(["--catalog", text(raw, "s.catalog")?])
            .args(["--profile", text(raw, "s.profile")?])
            .args(["--session-generation", text(raw, "s.session_generation")?])
            .args(["--clock-plan", text(raw, "s.clock_plan")?])
            .args([
                "--delivery-retain-generations",
                text(raw, "s.delivery_retain_generations")?,
            ])
            .arg("--structure-source")
            .arg(&decl);
        run_once(c, "S")?;
    }
    let econ = text(raw, "economic.manifest")?;
    let e_db = text(raw, "economic.e.db")?;
    if !Path::new(e_db).exists() {
        let mut c = clean(Command::new(&e_bin), &[]);
        c.args(["init", "--manifest", econ, "--db", e_db]);
        run_once(c, "E")?;
    }
    let chongs = raw["economic"]["b"].as_array().cloned().unwrap_or_default();
    for b in &chongs {
        let db = text(b, "db")?;
        if !Path::new(db).exists() {
            let mut c = clean(Command::new(&b_bin), &[]);
            c.args([
                "init",
                "--manifest",
                econ,
                "--db",
                db,
                "--chong",
                text(b, "chong")?,
            ]);
            run_once(c, "B")?;
        }
    }

    // ── 拉起：每域独立进程；中途失败也先回收已拉起的子进程再报错，不留孤儿 ──
    let mut children: Vec<Supervised> = Vec::new();
    let spawned = spawn_domains(&m, &s_bin, &e_bin, &b_bin, &x_bin, &mut children);
    let mut failed = spawned.is_err();

    // ── 监督 ──
    while !STOP.load(Ordering::SeqCst) && !failed {
        for sup in children.iter_mut().filter(|c| c.status.is_none()) {
            if let Ok(Some(status)) = sup.child.try_wait() {
                sup.status = Some(status);
                log(
                    "exit",
                    json!({"domain": sup.domain, "status": status.to_string(), "critical": sup.critical}),
                );
                failed |= sup.critical;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // ── 回收：先 SIGTERM，宽限期后 SIGKILL；全部 wait ──
    for sup in children.iter().filter(|c| c.status.is_none()) {
        // SAFETY：向本进程自己拉起、尚未回收的子进程 PID 发信号。
        unsafe {
            libc::kill(sup.child.id() as libc::pid_t, libc::SIGTERM);
        }
    }
    let deadline = Instant::now() + STOP_GRACE;
    for sup in children.iter_mut() {
        while sup.status.is_none() {
            match sup.child.try_wait() {
                Ok(Some(status)) => sup.status = Some(status),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                _ => {
                    let _ = sup.child.kill();
                    sup.status = sup.child.wait().ok();
                    break;
                }
            }
        }
    }
    spawned?;
    log("stopped", json!({"failed": failed}));
    Ok(if failed { 1 } else { 0 })
}

/// 按 S、E、各 Bᵢ、X 顺序拉起；已拉起的子进程即刻登记进 `children`，供调用方统一回收。
fn spawn_domains(
    m: &Manifest,
    s_bin: &Path,
    e_bin: &Path,
    b_bin: &Path,
    x_bin: &Path,
    children: &mut Vec<Supervised>,
) -> Result<(), String> {
    let raw = &m.raw;
    let s_db = text(raw, "s.db")?;
    let econ = text(raw, "economic.manifest")?;
    let e_db = text(raw, "economic.e.db")?;
    let chongs = raw["economic"]["b"].as_array().cloned().unwrap_or_default();
    let mut spawn = |domain: String, mut c: Command, critical: bool| -> Result<(), String> {
        c.stdout(Stdio::null());
        let child = c
            .spawn()
            .map_err(|e| format!("MissingDependency：{domain} 启动失败：{e}"))?;
        log("spawn", json!({"domain": domain, "pid": child.id()}));
        children.push(Supervised {
            domain,
            child,
            status: None,
            critical,
        });
        Ok(())
    };
    let mut s = clean(Command::new(s_bin), &S_PASS);
    s.args(["serve", "--protocol-revision", "s-session/3"])
        .args([
            "--admission-job-limits",
            text(raw, "s.admission_job_limits")?,
        ])
        .arg("--db")
        .arg(s_db)
        .args(["--profile", text(raw, "s.profile")?])
        .args(["--writer-epoch", text(raw, "s.writer_epoch")?])
        .args(["--clock-plan", text(raw, "s.clock_plan")?])
        .args(["--socket", text(raw, "s.socket")?])
        .args(["--resource-config", text(raw, "s.resource_config")?])
        .arg("--structure-source")
        .arg(m.state_dir.join("structure-source.json"));
    spawn("S".into(), s, true)?;
    let e_epoch = text(raw, "economic.e.epoch")?;
    let mut e = clean(Command::new(e_bin), &[]);
    e.args([
        "serve",
        "--manifest",
        econ,
        "--db",
        e_db,
        "--epoch",
        e_epoch,
    ]);
    spawn("E".into(), e, true)?;
    for b in &chongs {
        let chong = text(b, "chong")?;
        let mut c = clean(Command::new(b_bin), &[]);
        c.args(["serve", "--manifest", econ, "--db", text(b, "db")?])
            .args([
                "--chong",
                chong,
                "--epoch",
                text(b, "epoch")?,
                "--e-epoch",
                e_epoch,
            ]);
        spawn(format!("B:{chong}"), c, true)?;
    }
    let mut x = clean(Command::new(x_bin), &[]);
    x.args(["serve", "--db", text(raw, "x.db")?])
        .arg("--profile")
        .arg(&m.x_profile)
        .args(["--writer-epoch", text(raw, "x.writer_epoch")?]);
    if let Some((key, value)) = &m.credential {
        x.env(key, value);
    }
    spawn("X".into(), x, false)
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            log("refused", json!({"error": e}));
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}
