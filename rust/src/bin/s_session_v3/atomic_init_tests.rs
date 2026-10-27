//! #1568：init 在磁盘满、fd 耗尽、写入中途被杀后，目标要么不存在、要么是完整库，且原路径可直接重试。
//!
//! 资源限制只能按进程设，故失败现场放在子进程里：子进程重新执行本测试二进制、只跑
//! `s1568_child_init_under_rlimit`，先 `setrlimit` 再调 `init_v3`；父进程只读检查现场后在原路径重试。

use super::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const CHILD_TEST: &str = "s1568_child_init_under_rlimit";
const ENV_DB: &str = "S1568_CHILD_DB";
const ENV_CLOCK: &str = "S1568_CHILD_CLOCK";
const ENV_MODE: &str = "S1568_CHILD_MODE";

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "s1568-{tag}-{}-{}-{n}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("s_session")
        .join(path)
}

fn write_clock(dir: &Path) -> PathBuf {
    let path = dir.join("clock.json");
    let value = json!({
        "schema_revision": "s-clock-plan/1",
        "clock_plan_id": "s1568-clock",
        "origin_utc": "2026-09-18T00:00:00Z",
        "unit": "ns",
        "events": {"op-1": {"accept_ns": "10",
            "attempts": [{"begin_ns": "11", "commit_ns": "12"}]}},
    });
    std::fs::write(&path, canonical_json(&value)).unwrap();
    path
}

fn run_init(db: &Path, clock: &Path) -> Result<Value, String> {
    init_v3(
        db,
        "s1568-testonly",
        &fixture("catalog/signed-catalog.json"),
        &fixture("profiles/ohlc_integer_tb02a_v1.json"),
        "1",
        clock,
        4,
    )
}

fn staging_leftovers(db: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(db.parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| atomic_init::is_staging_entry(db, &e.file_name()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// 目标只许两态：不存在（返回 false），或可只读打开的完整 `s-session/3` 库（返回 true）。
fn target_absent_or_complete(db: &Path) -> bool {
    if !db.exists() {
        return false;
    }
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap_or_else(|e| panic!("目标存在却打不开：{e}"));
    let protocol: String = conn
        .query_row(
            "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|e| panic!("目标存在却不完整：{e}"));
    assert_eq!(protocol, PROTOCOL_REVISION);
    let items: i64 = conn
        .query_row("SELECT COUNT(*) FROM catalog", [], |r| r.get(0))
        .unwrap();
    assert!(items > 0, "目标存在却缺 catalog");
    true
}

/// 失败现场之后：目标两态之一；不存在时原路径直接重试必须成功，且暂存残留被清扫。
fn assert_retry_from_scene(db: &Path, clock: &Path, label: &str) {
    if target_absent_or_complete(db) {
        // 子进程在限制前已完成发布：目标已是完整库，init 必须拒绝且不改动。
        let before = std::fs::read(db).unwrap();
        assert!(run_init(db, clock).is_err(), "{label}: 既有库必须拒绝");
        assert_eq!(std::fs::read(db).unwrap(), before, "{label}: 既有库被改动");
        return;
    }
    let out = run_init(db, clock).unwrap_or_else(|e| panic!("{label}: 原路径重试失败：{e}"));
    assert_eq!(out["ok"], json!(true));
    assert!(target_absent_or_complete(db), "{label}: 重试后目标不完整");
    assert!(
        staging_leftovers(db).is_empty(),
        "{label}: 重试后仍有暂存残留 {:?}",
        staging_leftovers(db)
    );
}

struct ChildOutcome {
    status: std::process::ExitStatus,
    stdout: String,
}

fn spawn_child(db: &Path, clock: &Path, mode: &str) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            CHILD_TEST,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
            "-q",
        ])
        .env(ENV_DB, db)
        .env(ENV_CLOCK, clock)
        .env(ENV_MODE, mode)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn run_child(db: &Path, clock: &Path, mode: &str) -> ChildOutcome {
    let out = spawn_child(db, clock, mode).wait_with_output().unwrap();
    ChildOutcome {
        status: out.status,
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

fn child_result(outcome: &ChildOutcome) -> Option<&str> {
    outcome
        .stdout
        .lines()
        .find_map(|l| l.strip_prefix("S1568_CHILD "))
}

/// 子进程入口：父进程未设环境变量时什么都不做。
#[test]
#[ignore = "#1568 子进程入口，只由同文件父测试带环境变量调用"]
fn s1568_child_init_under_rlimit() {
    let (Ok(db), Ok(clock), Ok(mode)) = (
        std::env::var(ENV_DB),
        std::env::var(ENV_CLOCK),
        std::env::var(ENV_MODE),
    ) else {
        return;
    };
    let (kind, value) = mode.split_once(':').unwrap_or((mode.as_str(), "0"));
    let value: u64 = value.parse().unwrap();
    let set = |resource, limit: u64| {
        let rl = libc::rlimit {
            rlim_cur: limit as libc::rlim_t,
            rlim_max: limit as libc::rlim_t,
        };
        assert_eq!(unsafe { libc::setrlimit(resource, &rl) }, 0);
    };
    match kind {
        // 磁盘满：写超限返回 EFBIG（SIGXFSZ 忽略），进程内看到 I/O 错误。
        "fsize" => {
            unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) };
            set(libc::RLIMIT_FSIZE, value);
        }
        // 写入中途被杀：SIGXFSZ 保持默认动作，超限那一次 write 直接终止进程。
        "fsize-kill" => {
            unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_DFL) };
            set(libc::RLIMIT_FSIZE, value);
        }
        "nofile" => set(libc::RLIMIT_NOFILE, value),
        // 不设限，交给父进程按时 SIGKILL。
        "plain" => {}
        other => panic!("未知模式 {other}"),
    }
    match run_init(Path::new(&db), Path::new(&clock)) {
        Ok(_) => println!("S1568_CHILD ok"),
        Err(e) => println!("S1568_CHILD err {e}"),
    }
}

#[test]
fn disk_full_failures_leave_nothing_and_retry_succeeds() {
    // 库约 300KB；这些上限都落在建库/提交/检查点途中。
    for limit in [4096u64, 16_384, 65_536, 131_072, 262_144] {
        let dir = Dir::new("fsize");
        let db = dir.0.join("s3.sqlite");
        let clock = write_clock(&dir.0);
        let outcome = run_child(&db, &clock, &format!("fsize:{limit}"));
        let result = child_result(&outcome)
            .unwrap_or_else(|| panic!("fsize:{limit} 子进程无结果：{:?}", outcome.status));
        assert!(result.starts_with("err "), "fsize:{limit} 应失败：{result}");
        assert!(!db.exists(), "fsize:{limit} 失败后目标不得存在");
        assert!(
            staging_leftovers(&db).is_empty(),
            "fsize:{limit} 进程内失败应自行删除暂存：{:?}",
            staging_leftovers(&db)
        );
        assert_retry_from_scene(&db, &clock, &format!("fsize:{limit}"));
    }
}

#[test]
fn killed_mid_write_leaves_only_identifiable_staging_and_retry_succeeds() {
    for limit in [4096u64, 65_536, 262_144] {
        let dir = Dir::new("xfsz");
        let db = dir.0.join("s3.sqlite");
        let clock = write_clock(&dir.0);
        let outcome = run_child(&db, &clock, &format!("fsize-kill:{limit}"));
        assert_eq!(
            outcome.status.signal(),
            Some(libc::SIGXFSZ),
            "fsize-kill:{limit} 子进程应在写入中被 SIGXFSZ 杀死：{:?} {}",
            outcome.status,
            outcome.stdout
        );
        assert!(!db.exists(), "fsize-kill:{limit} 被杀后目标不得存在");
        assert!(
            !staging_leftovers(&db).is_empty(),
            "fsize-kill:{limit} 被杀应留下可识别暂存文件"
        );
        assert_retry_from_scene(&db, &clock, &format!("fsize-kill:{limit}"));
    }
    // SIGKILL：先轮询到暂存文件出现（子进程已进入 init 建库），再延迟不同时长杀死，让落点散布在
    // 建库/提交/发布各处；每个落点都须满足两态并可重试。
    // #1666 L1：暂存文件只在建库开始到发布 rename 之间存在，故「被 SIGKILL 杀死 + 目标不存在 +
    // 有暂存残留」即确证命中 init 中途；至少命中一次，否则此循环没有测到东西。
    let mut mid_init_hits = Vec::new();
    for delay_ms in [0u64, 0, 1, 2, 5, 10, 20, 40] {
        let dir = Dir::new("kill");
        let db = dir.0.join("s3.sqlite");
        let clock = write_clock(&dir.0);
        let mut child = spawn_child(&db, &clock, "plain");
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while staging_leftovers(&db).is_empty()
            && child.try_wait().unwrap().is_none()
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_micros(200));
        }
        std::thread::sleep(Duration::from_millis(delay_ms));
        let _ = child.kill();
        let status = child.wait().unwrap();
        let label = format!("kill@staging+{delay_ms}ms");
        if status.signal() == Some(libc::SIGKILL)
            && !db.exists()
            && !staging_leftovers(&db).is_empty()
        {
            mid_init_hits.push(label.clone());
        }
        assert_retry_from_scene(&db, &clock, &label);
    }
    println!("S1666_KILL_MID_INIT_HITS {mid_init_hits:?}");
    assert!(
        !mid_init_hits.is_empty(),
        "SIGKILL 循环一次也没命中 init 中途（暂存存在、目标未发布）"
    );
}

#[test]
fn fd_exhaustion_failures_leave_nothing_and_retry_succeeds() {
    let mut failures = Vec::new();
    for limit in 4u64..=40 {
        let dir = Dir::new("nofile");
        let db = dir.0.join("s3.sqlite");
        let clock = write_clock(&dir.0);
        let outcome = run_child(&db, &clock, &format!("nofile:{limit}"));
        let label = format!("nofile:{limit}");
        match child_result(&outcome) {
            Some(r) if r.starts_with("err ") => {
                assert!(!db.exists(), "{label} 失败后目标不得存在：{r}");
                assert!(
                    staging_leftovers(&db).is_empty(),
                    "{label} 进程内失败应自行删除暂存：{r} {:?}",
                    staging_leftovers(&db)
                );
                failures.push(format!("{label}: {r}"));
            }
            Some(_) => assert!(target_absent_or_complete(&db), "{label} 成功却无完整库"),
            // 限得太低时测试框架自身可能起不来；现场照样须满足两态。
            None => {}
        }
        assert_retry_from_scene(&db, &clock, &label);
    }
    println!("S1568_NOFILE_FAILURES {failures:#?}");
    assert!(
        failures.iter().any(
            |f| ["SQLite", "WAL", "unable to open", "事务", "commit", "暂存"]
                .iter()
                .any(|k| f.contains(k))
        ),
        "fd 上限扫描未触发任何建库阶段失败：{failures:#?}"
    );
}

#[test]
fn existing_target_is_refused_untouched_even_with_stale_staging() {
    let dir = Dir::new("existing");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    // 用户已有的任意文件（含 #1527 那种 4096 字节残留形态）：拒绝，不改字节，不清扫掉它。
    std::fs::write(&db, vec![0u8; 4096]).unwrap();
    let stale = dir.0.join(".s3.sqlite.s3-init.1.2.tmp");
    std::fs::write(&stale, b"stale").unwrap();
    let err = run_init(&db, &clock).unwrap_err();
    assert!(err.contains("库已存在"), "{err}");
    assert_eq!(std::fs::read(&db).unwrap(), vec![0u8; 4096]);
    assert!(stale.exists(), "目标已存在时不进入清扫");
}

#[test]
fn sweep_only_removes_this_databases_staging_names() {
    let dir = Dir::new("sweep");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let ours = [
        ".s3.sqlite.s3-init.11.22.tmp",
        ".s3.sqlite.s3-init.11.22.tmp-wal",
        ".s3.sqlite.s3-init.11.22.tmp-shm",
        ".s3.sqlite.s3-init.3.4.tmp-journal",
    ];
    let decoys = [
        ".s3.sqlite.s3-init.x.22.tmp",
        ".s3.sqlite.s3-init.11.22.tmp.bak",
        ".other.sqlite.s3-init.11.22.tmp",
        "s3.sqlite.s3-init.11.22.tmp",
        ".s3.sqlite.s3-init.5.6.s3-init.1.2.tmp",
    ];
    for name in ours.iter().chain(decoys.iter()) {
        std::fs::write(dir.0.join(name), b"x").unwrap();
    }
    run_init(&db, &clock).unwrap();
    for name in ours {
        assert!(!dir.0.join(name).exists(), "本库暂存残留应被清扫：{name}");
    }
    for name in decoys {
        assert!(dir.0.join(name).exists(), "非本库暂存格式不得删除：{name}");
    }
    assert!(target_absent_or_complete(&db));
}

#[test]
fn foreign_wal_beside_absent_target_is_refused_and_kept() {
    let dir = Dir::new("foreign");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let wal = dir.0.join("s3.sqlite-wal");
    std::fs::write(&wal, b"foreign wal").unwrap();
    let err = run_init(&db, &clock).unwrap_err();
    assert!(err.contains("目标旁已有"), "{err}");
    assert!(!db.exists());
    assert_eq!(std::fs::read(&wal).unwrap(), b"foreign wal");
    std::fs::remove_file(&wal).unwrap();
    run_init(&db, &clock).unwrap();
    assert!(target_absent_or_complete(&db));
}

// ── #1666 L2/L3：发布阶段故障注入（`atomic_init::fault`，仅本线程生效） ──

use atomic_init::fault::{self, Fault};

fn publish_warnings(out: &Value) -> Vec<String> {
    out.get("publish_warnings")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|w| w.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn link_fallback_publishes_complete_db_without_leftovers() {
    let dir = Dir::new("link");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let out = fault::with(Fault::RenameErrno(libc::ENOTSUP), || run_init(&db, &clock))
        .unwrap_or_else(|e| panic!("link 回退应发布成功：{e}"));
    assert_eq!(out["ok"], json!(true));
    assert!(publish_warnings(&out).is_empty(), "{out}");
    assert!(target_absent_or_complete(&db));
    assert!(
        staging_leftovers(&db).is_empty(),
        "{:?}",
        staging_leftovers(&db)
    );
}

#[test]
fn link_fallback_never_overwrites_existing_target() {
    let dir = Dir::new("link-exists");
    let db = dir.0.join("s3.sqlite");
    let staging = atomic_init::Staging::new(&db).unwrap();
    std::fs::write(staging.path(), b"staged").unwrap();
    // 发布前目标被他人抢先创建：link 以 EEXIST 失败，既有字节不动，暂存由守卫删除。
    std::fs::write(&db, b"someone else").unwrap();
    let err = fault::with(Fault::RenameErrno(libc::ENOTSUP), || staging.publish()).unwrap_err();
    assert!(err.contains("库已存在"), "{err}");
    assert_eq!(std::fs::read(&db).unwrap(), b"someone else");
    assert!(
        staging_leftovers(&db).is_empty(),
        "{:?}",
        staging_leftovers(&db)
    );
}

#[test]
fn rename_einval_classification_follows_platform_manual() {
    let dir = Dir::new("einval");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let result = fault::with(Fault::RenameErrno(libc::EINVAL), || run_init(&db, &clock));
    if cfg!(target_os = "linux") {
        // renameat2(2)：EINVAL = 文件系统不支持该标志 → 走 link 回退并成功。
        let out = result.unwrap_or_else(|e| panic!("Linux EINVAL 应回退 link：{e}"));
        assert!(publish_warnings(&out).is_empty(), "{out}");
        assert!(target_absent_or_complete(&db));
    } else {
        // renamex_np(2)：EINVAL 不是「不支持」，不回退；目标不存在、暂存已清。
        let err = result.unwrap_err();
        assert!(err.contains("原子发布暂存库失败"), "{err}");
        assert!(!db.exists());
    }
    assert!(
        staging_leftovers(&db).is_empty(),
        "{:?}",
        staging_leftovers(&db)
    );
    // 其他 errno 一律不回退。
    let dir = Dir::new("eacces");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let err = fault::with(Fault::RenameErrno(libc::EACCES), || run_init(&db, &clock)).unwrap_err();
    assert!(err.contains("原子发布暂存库失败"), "{err}");
    assert!(!db.exists());
    assert!(
        staging_leftovers(&db).is_empty(),
        "{:?}",
        staging_leftovers(&db)
    );
}

#[test]
fn dir_fsync_failure_after_publish_is_success_with_warning() {
    let dir = Dir::new("dirsync");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let out = fault::with(Fault::DirSync, || run_init(&db, &clock))
        .unwrap_or_else(|e| panic!("目标已发布，目录 fsync 失败不得报 init 失败：{e}"));
    assert_eq!(out["ok"], json!(true));
    let warnings = publish_warnings(&out);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("PublishedUnsynced"), "{warnings:?}");
    assert!(target_absent_or_complete(&db));
    assert!(
        staging_leftovers(&db).is_empty(),
        "{:?}",
        staging_leftovers(&db)
    );
}

#[test]
fn link_fallback_unlink_failure_is_success_with_leftover_warning() {
    let dir = Dir::new("unlink");
    let db = dir.0.join("s3.sqlite");
    let clock = write_clock(&dir.0);
    let out = fault::with(Fault::RenameUnsupportedUnlinkFails, || {
        run_init(&db, &clock)
    })
    .unwrap_or_else(|e| panic!("目标已 link 发布，删暂存名失败不得报 init 失败：{e}"));
    assert_eq!(out["ok"], json!(true));
    let warnings = publish_warnings(&out);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("PublishedWithLeftover"),
        "{warnings:?}"
    );
    assert!(target_absent_or_complete(&db));
    // 告警点名的残留就是那个与目标同 inode 的暂存名。
    let leftovers = staging_leftovers(&db);
    assert_eq!(leftovers.len(), 1, "{leftovers:?}");
    assert!(
        warnings[0].contains(&leftovers[0]),
        "{warnings:?} {leftovers:?}"
    );
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        std::fs::metadata(dir.0.join(&leftovers[0])).unwrap().ino(),
        std::fs::metadata(&db).unwrap().ino()
    );
    // 同路径再 init：识别为既有库，拒绝且不改动。
    let before = std::fs::read(&db).unwrap();
    let err = run_init(&db, &clock).unwrap_err();
    assert!(err.contains("库已存在"), "{err}");
    assert_eq!(std::fs::read(&db).unwrap(), before);
}
