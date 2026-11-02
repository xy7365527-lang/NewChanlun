//! #1769（ADR 0039 迁移步骤 1）：X 执行域可运行骨架。
//!
//! X 与 S/E/B 同型：自有 SQLite 库、`<db>.writer.lock` 独占进程锁、持久化 `writer_epoch`。
//! 本骨架只持久化执行 profile 版本与 TB-09 指纹字段、发送面状态，并提供只读查询；
//! **不含任何场所写代码**：一切写调用（`submit`/`cancel`）返回 `UnsupportedCapability`，
//! 不返回假成功。experiment/production profile 缺凭据时，在构造任何传输之前以
//! `MissingDependency` 非 0 退出；production profile 缺 TB-09 指纹时持久化拒绝发送状态。
//!
//! 用法：
//!   x_execution_session serve --db FILE --profile FILE --writer-epoch N
//!   x_execution_session query --db FILE
//!   x_execution_session submit|cancel --db FILE      （恒 UnsupportedCapability）
use newchan_rust::session_protocol::positive;
use newchan_rust::session_transport::process_lock;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

const PROFILE_REVISION: &str = "x-execution-profile/1";
const MODES: [&str; 4] = ["simulation", "dry-run", "experiment", "production"];
const SCHEMA: &str =
    "CREATE TABLE IF NOT EXISTS x_meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);";

/// 执行 profile（`x-execution-profile/1`）。凭据只经 `credential_scope.env` 指名的环境变量
/// 进入本进程；值永不落库、不打印。
struct Profile {
    revision: String,
    mode: String,
    endpoint: Option<String>,
    credential_env: String,
    tb09_fingerprint: Option<String>,
}

impl Profile {
    fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("InvalidDomain：读执行 profile 失败：{e}"))?;
        let v: Value = serde_json::from_str(&text)
            .map_err(|e| format!("InvalidDomain：执行 profile JSON 解析失败：{e}"))?;
        let revision = v["profile_revision"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if revision != PROFILE_REVISION {
            return Err(format!(
                "VersionUnsupported：执行 profile 版本 `{revision}` 不是 {PROFILE_REVISION}"
            ));
        }
        let mode = v["mode"]
            .as_str()
            .filter(|m| MODES.contains(m))
            .ok_or(
                "InvalidDomain：执行 profile mode 必须是 simulation|dry-run|experiment|production",
            )?
            .to_string();
        let credential_env = v["credential_scope"]["env"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("InvalidDomain：执行 profile 缺 credential_scope.env")?
            .to_string();
        let opt = |key: &str| -> Result<Option<String>, String> {
            match &v[key] {
                Value::Null => Ok(None),
                Value::String(s) if !s.is_empty() => Ok(Some(s.clone())),
                _ => Err(format!(
                    "InvalidDomain：执行 profile {key} 必须是非空字符串或 null"
                )),
            }
        };
        Ok(Self {
            revision,
            mode,
            endpoint: opt("endpoint")?,
            credential_env,
            tb09_fingerprint: opt("tb09_fingerprint")?,
        })
    }

    fn needs_credential(&self) -> bool {
        matches!(self.mode.as_str(), "experiment" | "production")
    }
}

fn err(e: impl std::fmt::Display) -> String {
    format!("StorageUnavailable：{e}")
}

fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM x_meta WHERE key=?1", [key], |r| r.get(0))
        .optional()
        .map_err(err)
}

/// 持锁后建库/续库：epoch 只增不减（低 epoch 为陈旧写者），profile 版本、模式、指纹与
/// 发送面状态在同一事务落盘。
fn persist(conn: &mut Connection, profile: &Profile, epoch: i64) -> Result<Value, String> {
    let tx = conn.transaction().map_err(err)?;
    tx.execute_batch(SCHEMA).map_err(err)?;
    if let Some(stored) = meta_get(&tx, "writer_epoch")? {
        let stored: i64 = stored.parse().map_err(err)?;
        if epoch < stored {
            return Err(format!(
                "StaleWriter：X writer_epoch={epoch} 低于已持久化的 {stored}"
            ));
        }
    }
    let (send_state, reason) = if profile.mode == "production" && profile.tb09_fingerprint.is_none()
    {
        ("refused", "MissingTb09Fingerprint")
    } else {
        ("unsupported", "UnsupportedCapability")
    };
    let rows = [
        ("writer_epoch", epoch.to_string()),
        ("profile_revision", profile.revision.clone()),
        ("profile_mode", profile.mode.clone()),
        (
            "tb09_fingerprint",
            profile.tb09_fingerprint.clone().unwrap_or_default(),
        ),
        ("send_state", send_state.to_string()),
        ("send_refusal_reason", reason.to_string()),
    ];
    for (key, value) in rows {
        tx.execute(
            "INSERT INTO x_meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [key, value.as_str()],
        )
        .map_err(err)?;
    }
    tx.commit().map_err(err)?;
    query_conn(conn)
}

fn query_conn(conn: &Connection) -> Result<Value, String> {
    let mut stmt = conn
        .prepare("SELECT key,value FROM x_meta ORDER BY key")
        .map_err(err)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(err)?
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(err)?;
    Ok(json!(rows))
}

fn args() -> Result<(String, BTreeMap<String, String>), String> {
    let mut it = std::env::args().skip(1);
    let command = it
        .next()
        .ok_or("usage: serve|query|submit|cancel --db FILE ...")?;
    let mut opts = BTreeMap::new();
    while let Some(k) = it.next() {
        if !["--db", "--profile", "--writer-epoch"].contains(&k.as_str()) || opts.contains_key(&k) {
            return Err(format!("InvalidDomain：未知/重复参数 {k}"));
        }
        opts.insert(k, it.next().ok_or("InvalidDomain：参数缺值")?);
    }
    Ok((command, opts))
}

fn run() -> Result<(), String> {
    let (command, opts) = args()?;
    let get = |k: &str| {
        opts.get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("InvalidDomain：缺{k}"))
    };
    let db = Path::new(get("--db")?);
    match command.as_str() {
        "serve" => {
            let profile = Profile::load(Path::new(get("--profile")?))?;
            let epoch = positive(get("--writer-epoch")?, "writer_epoch")?;
            let _lock = process_lock(db, "X")?;
            let mut conn = Connection::open(db).map_err(err)?;
            let state = persist(&mut conn, &profile, epoch)?;
            drop(conn);
            // 凭据门在任何传输构造之前：缺即 MissingDependency 非 0 退出，端点从未被连接。
            if profile.needs_credential()
                && std::env::var_os(&profile.credential_env).is_none_or(|v| v.is_empty())
            {
                return Err(format!(
                    "MissingDependency：{} profile 缺凭据（credential_scope.env={}）；发送面未启动",
                    profile.mode, profile.credential_env
                ));
            }
            // 骨架：场所传输待后续叶落地（ADR 0039 迁移步骤 2+）；此处不连接 endpoint。
            let _ = &profile.endpoint;
            eprintln!(
                "x_execution_session serving（发送面 {}）",
                state["send_state"].as_str().unwrap_or_default()
            );
            println!("{state}");
            loop {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        "query" => {
            let conn =
                Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
            println!("{}", query_conn(&conn)?);
            Ok(())
        }
        "submit" | "cancel" => Err(format!(
            "UnsupportedCapability：X `{command}` 写调用在场所适配叶落地前不可用"
        )),
        other => Err(format!("InvalidDomain：未知子命令 {other}")),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
