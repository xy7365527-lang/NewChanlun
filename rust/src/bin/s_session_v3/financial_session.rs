//! #1394 DESIGN-R6 方案 c2：长驻只读财务会话。
//!
//! 依据：APPROVED-SOURCE §5.7 修订记录 A6 r2（`0e620363…`；用户第一问 A，根裁 A6-2）。会话改变的是
//! §5.7:336 多请求一致性中审计执行的次数与位置：启动时一致捕获并完整审计一次，之后每请求以 R1–R8
//! 证明仍是同一已审状态。一致性只有 a 一档，anchor／会话身份不符只拒本次（reject-request），两者都
//! 没有可选档（#799）。公共协议 `s-financial-basis/3`、`s-financial-content/1` 的请求/回复字节不变。
//!
//! 启动（一次，在任何经济 Job 之外）：严格校验 runner 交来的写者收尾托管文档
//! （`s-financial-session-custody/2`）并与首次托管逐项相等 → 活源连接 S（READ_ONLY|NO_MUTEX、
//! query_only、read_uncommitted=OFF）→ 三点 data_version 夹持的一致捕获，同一读事务取控制原件 K0
//! → 导入私有副本 P → P 上与冷路径逐字相同的 `state::audit(&tx)` → 同一 `current_basis` 取 anchor
//! → K0 与 P 逐字节相同 → R9 保留帽（只计 P 与审计状态；进程足迹只观测）。
//! socket 在启动前绑定；READY 之前到达的连接一律拒绝。启动完成后才写 STARTUP.json 并输出唯一 READY 行。
//!
//! 每请求（同进程、同活源连接、单请求在飞，整体截止含读请求与写回复）：R7 客户端二进制 SHA 与
//! `--db` (dev, ino)；R4 主库/WAL 身份与 stat；R2/R3 autocommit 与只读设置；R5 同连接 data_version
//! （BEGIN 前、首读后、ROLLBACK 后、回复前）；R6 控制原件逐字节等于 K0，并在活源短事务上跑冷路径同一
//! `current_control` 判据；R8 从 P 的新短事务经冷路径同一 `basis_ref_with`/`content_with`（`HashReader::new` 逐节点核验）供片。
//! 显式比较得出的不一致（不论是否已过截止）→ 本请求 MissingDependency，会话 stale 并拒绝之后所有请求；
//! 只有本请求的 SQLite 读取确被期限中断或 busy 等待耗尽，才算“未完成”只拒本次。请求内从不重审、
//! 从不回落冷路径。停止：`financial-serve-stop` 优雅停机写 FINAL；SIGTERM 尽力写 FINAL 并标非正常停机。
//!
//! #1394 第三问选项 A（financial-closure，A6 r2 §3，用户已批）：第三条财务命令
//! 照走同一 R7/R2–R6 夹持与 R8 已审副本短事务，经冷路径同一 `closure_with` 供块；会话至多保留一张
//! 闭包表（摘要升序），表帽为资源 `max_closure_nodes`，表字节计入 R9 保留门（`retained_bytes`）。
use super::*;
use newchan_rust::s_content as sc;
use newchan_rust::session_protocol::{canonical_json, positive, required, strict_json};
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 会话一致性的唯一档（A6 r2 §2.2，#1394）：没有 `--consistency` 参数，也没有原件字节门。启动回执
/// 仍写 `consistency` 键，是 r7 接口 `receipt_keys_exact` 的形状；值恒为本常量。
const SESSION_CONSISTENCY: &str = "a";
/// anchor（financial-content）或会话身份（financial-basis-ref）与已审状态不符：只拒本次，会话保持有效，
/// 回复与冷路径相同——不符说明请求绑定的是别的 basis，不说明源变化（A6 r2 §2.2，用户裁定
/// `anchor_mismatch=reject-request`）。回执键 `anchor_mismatch_policy` 同上，值恒为本常量。
const ANCHOR_MISMATCH_POLICY: &str = "reject-request";

const RESOURCES_SCHEMA: &str = "s-financial-session-resources/3";
const RESOURCE_FIELDS: [&str; 9] = [
    "schema_revision",
    "startup_timeout_ms",
    "retained_bytes",
    "max_requests",
    "max_request_bytes",
    "max_reply_bytes",
    "request_timeout_ms",
    "max_capture_bytes",
    "max_closure_nodes",
];
const CUSTODY_SCHEMA: &str = "s-financial-session-custody/2";
const CUSTODY_FIELDS: [&str; 5] = ["schema_revision", "db", "main", "wal", "shm"];
const HASHED_FILE_FIELDS: [&str; 6] = [
    "sha256",
    "st_dev",
    "st_ino",
    "st_size",
    "st_mtime_ns",
    "st_ctime_ns",
];
const STAT_FIELDS: [&str; 5] = ["st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns"];
const REQUEST_SCHEMA: &str = "s-financial-session-request/1";
const REQUEST_FIELDS: [&str; 5] = [
    "schema_revision",
    "command",
    "binary_sha256",
    "db_dev",
    "db_ino",
];
const STOP_FIELDS: [&str; 3] = ["schema_revision", "command", "binary_sha256"];
const STARTUP_SCHEMA: &str = "s-financial-session-startup/2";
const READY_SCHEMA: &str = "s-financial-session-ready/1";
const FINAL_SCHEMA: &str = "s-financial-session-final/2";
/// 内部帧头（命令与绑定）上限；与冻结的请求/回复帽无关。
const MAX_HEADER_FRAME: usize = 4096;
/// 客户端接受的回复帧硬上限；会话自身按冻结 `max_reply_bytes` 更早拒绝。
const CLIENT_MAX_REPLY: usize = 1 << 30;
/// macOS `sockaddr_un.sun_path` 含结尾 NUL 共 104 字节。
const MAX_SOCKET_PATH: usize = 103;
/// READY 之前拒绝一个连接时读写的整体截止；短于请求期限，避免慢客户端拖住就绪转换与 SIGTERM。
const PRE_READY_IO_MS: u64 = 2000;
/// 客户端与停机命令等待回复的宽上限（单次期限由经济侧 45 s 子进程门执行）。
const CLIENT_WAIT_MS: u64 = 120_000;
/// SQLite progress handler 的 VM 步数粒度（同 Q3 捕获）。
const PROGRESS_OPS: i32 = 100;
const NOT_READY: &str = "会话尚未就绪：启动未完成，READY 之前一律拒绝";
const KIND_PREFIXES: [&str; 3] = [
    "MissingDependency：",
    "FinancialReadInvalid：",
    "Q3CaptureInvalid：",
];

fn missing(m: impl std::fmt::Display) -> String {
    format!("MissingDependency：{m}")
}
fn invalid(m: impl std::fmt::Display) -> String {
    format!("FinancialReadInvalid：{m}")
}
fn incomplete(kind: &str, m: impl std::fmt::Display) -> String {
    format!("Incomplete/{kind}：{m}")
}
/// F8：去掉内层错误的类别前缀，避免“MissingDependency：R6：MissingDependency：…”式嵌套。
fn bare(e: &str) -> &str {
    let mut s = e;
    while let Some(p) = KIND_PREFIXES.iter().find(|p| s.starts_with(**p)) {
        s = &s[p.len()..];
    }
    s
}
fn exact(value: &Value, fields: &[&str], what: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("{what}必须是JSON对象")))?;
    if object.len() != fields.len() || object.keys().any(|k| !fields.contains(&k.as_str())) {
        return Err(invalid(format!(
            "{what}字段必须exact为 {}",
            fields.join(",")
        )));
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hex64(value: &str, what: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(format!("{what}必须是64位小写十六进制")));
    }
    Ok(())
}
/// 规范非负十进制文本（无符号、无前导零、"0" 合法）→ u64。
fn decimal(value: &Value, key: &str, what: &str) -> Result<u64, String> {
    let text = value[key]
        .as_str()
        .ok_or_else(|| invalid(format!("{what}.{key}须为十进制文本")))?;
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(invalid(format!("{what}.{key}不是规范十进制")));
    }
    text.parse::<u64>()
        .map_err(|_| invalid(format!("{what}.{key}超出u64")))
}
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
fn us(t: Instant) -> u64 {
    u64::try_from(t.elapsed().as_micros()).unwrap_or(u64::MAX)
}
fn check_deadline(deadline: Instant, kind: &str) -> Result<(), String> {
    if Instant::now() >= deadline {
        Err(incomplete(kind, "超过绝对期限"))
    } else {
        Ok(())
    }
}
fn sha_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// R9：会话资源（全部显式正整数，规范十进制文本；无默认、无无限量）。
/// `retained_bytes` 只管私有副本 P 与审计状态（B 另加保留原件）及闭包表的保留字节，不管进程足迹。
/// `max_closure_nodes` 是 financial-closure 闭包表的摘要数上限（会话至多一张表）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SessionResources {
    pub startup_timeout_ms: u64,
    pub retained_bytes: u64,
    pub max_requests: u64,
    pub max_request_bytes: u64,
    pub max_reply_bytes: u64,
    pub request_timeout_ms: u64,
    pub max_capture_bytes: u64,
    pub max_closure_nodes: u64,
}
impl SessionResources {
    pub(super) fn from_json(value: &Value) -> Result<Self, String> {
        exact(value, &RESOURCE_FIELDS, "会话资源")?;
        if value["schema_revision"] != RESOURCES_SCHEMA {
            return Err(invalid("会话资源版本不符"));
        }
        let n = |key: &str| -> Result<u64, String> {
            let text = required(value, key).map_err(invalid)?;
            Ok(positive(text, key).map_err(invalid)? as u64)
        };
        let resources = Self {
            startup_timeout_ms: n("startup_timeout_ms")?,
            retained_bytes: n("retained_bytes")?,
            max_requests: n("max_requests")?,
            max_request_bytes: n("max_request_bytes")?,
            max_reply_bytes: n("max_reply_bytes")?,
            request_timeout_ms: n("request_timeout_ms")?,
            max_capture_bytes: n("max_capture_bytes")?,
            max_closure_nodes: n("max_closure_nodes")?,
        };
        if resources.max_request_bytes > u64::from(u32::MAX)
            || resources.max_reply_bytes > u64::from(u32::MAX) - 1
        {
            return Err(invalid("会话帧帽超过帧长度域"));
        }
        Ok(resources)
    }
    pub(super) fn to_json(&self) -> Value {
        json!({"schema_revision":RESOURCES_SCHEMA,
            "startup_timeout_ms":self.startup_timeout_ms.to_string(),
            "retained_bytes":self.retained_bytes.to_string(),
            "max_requests":self.max_requests.to_string(),
            "max_request_bytes":self.max_request_bytes.to_string(),
            "max_reply_bytes":self.max_reply_bytes.to_string(),
            "request_timeout_ms":self.request_timeout_ms.to_string(),
            "max_capture_bytes":self.max_capture_bytes.to_string(),
            "max_closure_nodes":self.max_closure_nodes.to_string()})
    }
}

/// 文件身份与 stat（与 runner 合同 `st_dev,st_ino,st_size,st_mtime_ns,st_ctime_ns` 同名同义）。
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileStat {
    dev: u64,
    ino: u64,
    size: u64,
    mtime_ns: u64,
    ctime_ns: u64,
}
impl FileStat {
    fn of(path: &Path) -> Result<Self, String> {
        let m = std::fs::symlink_metadata(path)
            .map_err(|e| missing(format!("stat {}：{e}", path.display())))?;
        Self::from_metadata(&m)
    }
    fn optional(path: &Path) -> Result<Option<Self>, String> {
        match std::fs::symlink_metadata(path) {
            Ok(m) => Self::from_metadata(&m).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(missing(format!("stat {}：{e}", path.display()))),
        }
    }
    fn from_metadata(m: &std::fs::Metadata) -> Result<Self, String> {
        if !m.file_type().is_file() {
            return Err(missing("DB、WAL或SHM不是普通文件"));
        }
        let ns = |s: i64, n: i64| {
            u64::try_from(i128::from(s) * 1_000_000_000 + i128::from(n))
                .map_err(|_| missing("文件时间超出u64纳秒域"))
        };
        Ok(Self {
            dev: m.dev(),
            ino: m.ino(),
            size: m.size(),
            mtime_ns: ns(m.mtime(), m.mtime_nsec())?,
            ctime_ns: ns(m.ctime(), m.ctime_nsec())?,
        })
    }
    fn from_json(value: &Value, what: &str) -> Result<Self, String> {
        Ok(Self {
            dev: decimal(value, "st_dev", what)?,
            ino: decimal(value, "st_ino", what)?,
            size: decimal(value, "st_size", what)?,
            mtime_ns: decimal(value, "st_mtime_ns", what)?,
            ctime_ns: decimal(value, "st_ctime_ns", what)?,
        })
    }
    fn json(&self) -> Value {
        json!({"st_dev":self.dev.to_string(),"st_ino":self.ino.to_string(),
            "st_size":self.size.to_string(),"st_mtime_ns":self.mtime_ns.to_string(),
            "st_ctime_ns":self.ctime_ns.to_string()})
    }
}
fn optional_stat_json(stat: &Option<FileStat>) -> Value {
    stat.as_ref().map_or(Value::Null, FileStat::json)
}

fn sibling(db: &Path, suffix: &str) -> PathBuf {
    let mut s = db.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn file_sha256(path: &Path, deadline: Option<Instant>) -> Result<String, String> {
    let mut f =
        std::fs::File::open(path).map_err(|e| missing(format!("读 {}：{e}", path.display())))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if let Some(deadline) = deadline {
            check_deadline(deadline, "SessionStartupDeadline")?;
        }
        let n = f
            .read(&mut buf)
            .map_err(|e| missing(format!("读 {}：{e}", path.display())))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}
fn self_sha256() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| missing(format!("current_exe：{e}")))?;
    file_sha256(&exe, None)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HashedFile {
    sha256: String,
    stat: FileStat,
}
impl HashedFile {
    fn from_json(value: &Value, what: &str) -> Result<Self, String> {
        exact(value, &HASHED_FILE_FIELDS, what)?;
        let sha256 = required(value, "sha256").map_err(invalid)?.to_owned();
        hex64(&sha256, &format!("{what}.sha256"))?;
        Ok(Self {
            sha256,
            stat: FileStat::from_json(value, what)?,
        })
    }
    fn json(&self) -> Value {
        let mut v = self.stat.json();
        v["sha256"] = json!(self.sha256);
        v
    }
}

/// 写者收尾托管与会话首末托管的同一 exact 形状（`s-financial-session-custody/2`，runner 产出）：
/// `db` 为与 `--db` 逐字相同的绝对路径；`main`/`wal` 为 SHA-256 加五项 stat；`shm` 只记五项 stat
/// （读者合法改写 -shm，不作相等门）；`wal`/`shm` 为 null 表示该文件在托管时不存在。全部整数为规范十进制文本。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Custody {
    db: String,
    main: HashedFile,
    wal: Option<HashedFile>,
    shm: Option<FileStat>,
}
impl Custody {
    pub(super) fn from_json(value: &Value) -> Result<Self, String> {
        exact(value, &CUSTODY_FIELDS, "托管")?;
        if value["schema_revision"] != CUSTODY_SCHEMA {
            return Err(invalid("托管版本不符"));
        }
        let db = required(value, "db").map_err(invalid)?.to_owned();
        if !Path::new(&db).is_absolute() {
            return Err(invalid("托管.db必须为绝对路径"));
        }
        let wal = match &value["wal"] {
            Value::Null => None,
            v => Some(HashedFile::from_json(v, "托管.wal")?),
        };
        let shm = match &value["shm"] {
            Value::Null => None,
            v => {
                exact(v, &STAT_FIELDS, "托管.shm")?;
                Some(FileStat::from_json(v, "托管.shm")?)
            }
        };
        Ok(Self {
            db,
            main: HashedFile::from_json(&value["main"], "托管.main")?,
            wal,
            shm,
        })
    }
    pub(super) fn to_json(&self) -> Value {
        json!({"schema_revision":CUSTODY_SCHEMA,"db":self.db,"main":self.main.json(),
            "wal":self.wal.as_ref().map_or(Value::Null, HashedFile::json),
            "shm":optional_stat_json(&self.shm)})
    }
    /// 现场托管：先取三文件 stat，哈希主库与 WAL，再核主库与 WAL 的 stat 未变（-shm 不核）。
    pub(super) fn observe(db: &Path, deadline: Option<Instant>) -> Result<Self, String> {
        let wal_path = sibling(db, "-wal");
        let main_stat = FileStat::of(db)?;
        let wal_stat = FileStat::optional(&wal_path)?;
        let shm = FileStat::optional(&sibling(db, "-shm"))?;
        let main_sha = file_sha256(db, deadline)?;
        let wal = match wal_stat {
            Some(stat) => Some(HashedFile {
                sha256: file_sha256(&wal_path, deadline)?,
                stat,
            }),
            None => None,
        };
        if FileStat::of(db)? != main_stat
            || FileStat::optional(&wal_path)? != wal.as_ref().map(|w| w.stat.clone())
        {
            return Err(missing("托管哈希期间DB或WAL改变"));
        }
        Ok(Self {
            db: db
                .to_str()
                .ok_or_else(|| invalid("--db不是UTF-8"))?
                .to_owned(),
            main: HashedFile {
                sha256: main_sha,
                stat: main_stat,
            },
            wal,
            shm,
        })
    }
    /// 托管相等：同一路径，主库与 WAL 的 SHA 与 stat 全同；-shm 不比。
    fn same_files(&self, other: &Self) -> bool {
        self.db == other.db && self.main == other.main && self.wal == other.wal
    }
}

/// K0：meta 全表（含 generation/structure_cut/index_frontier/last_advance_frontier/advance_state/
/// writer_epoch/protocol_revision 等控制行）、s_protocol_meta 全行（含列名与 clock_plan_hash/
/// clock_plan_json）、MAX(raw_events.seq)。按 SQLite 原类型与原字节编码，供逐字节比较。
fn control_bytes(c: &Connection) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for (label, sql) in [
        ("meta", "SELECT key,value FROM meta ORDER BY key"),
        (
            "s_protocol_meta",
            "SELECT * FROM s_protocol_meta ORDER BY singleton",
        ),
        ("raw_events", "SELECT COALESCE(MAX(seq),-1) FROM raw_events"),
    ] {
        out.extend_from_slice(label.as_bytes());
        out.push(0);
        let mut stmt = c.prepare(sql).map_err(|e| e.to_string())?;
        let names = stmt
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for name in &names {
            out.extend_from_slice(&(name.len() as u64).to_be_bytes());
            out.extend_from_slice(name.as_bytes());
        }
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            out.push(b'[');
            for i in 0..names.len() {
                match row.get_ref(i).map_err(|e| e.to_string())? {
                    ValueRef::Null => out.push(0),
                    ValueRef::Integer(v) => {
                        out.push(1);
                        out.extend_from_slice(&v.to_be_bytes());
                    }
                    ValueRef::Real(v) => {
                        out.push(2);
                        out.extend_from_slice(&v.to_bits().to_be_bytes());
                    }
                    ValueRef::Text(v) => {
                        out.push(3);
                        out.extend_from_slice(&(v.len() as u64).to_be_bytes());
                        out.extend_from_slice(v);
                    }
                    ValueRef::Blob(v) => {
                        out.push(4);
                        out.extend_from_slice(&(v.len() as u64).to_be_bytes());
                        out.extend_from_slice(v);
                    }
                }
            }
            out.push(b']');
        }
    }
    Ok(out)
}

fn rusage_json() -> Value {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage 只写入调用方提供的结构体。
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    if rc != 0 {
        return Value::Null;
    }
    let tv = |t: libc::timeval| format!("{}.{:06}", t.tv_sec, t.tv_usec);
    json!({"ru_maxrss":ru.ru_maxrss.to_string(),"user_s":tv(ru.ru_utime),"sys_s":tv(ru.ru_stime)})
}
fn sqlite_memory_used() -> i64 {
    // SAFETY: 无参只读统计。
    unsafe { rusqlite::ffi::sqlite3_memory_used() }
}

/// 堆读数：全部 malloc zone 的 `size_in_use`（Rust System 分配器与 SQLite 都经 malloc）与 SQLite
/// 自记的 `sqlite3_memory_used`。会话启动单线程进行，两次读数之差可归属到其间的调用。
#[cfg(target_os = "macos")]
pub(super) fn heap_probe() -> Result<(u64, i64), String> {
    let mut stats: libc::malloc_statistics_t = unsafe { std::mem::zeroed() };
    // SAFETY: zone 为 NULL 时统计全部 zone；只写入调用方结构体。
    unsafe { libc::malloc_zone_statistics(std::ptr::null_mut(), &mut stats) };
    Ok((stats.size_in_use as u64, sqlite_memory_used()))
}
#[cfg(not(target_os = "macos"))]
pub(super) fn heap_probe() -> Result<(u64, i64), String> {
    Err(missing("会话保留量读数当前只在macOS实现"))
}
/// Δ(全部 zone 在用) − Δ(SQLite 自记)：期间新保留的非 SQLite 堆字节。
fn rust_heap_delta(before: (u64, i64), after: (u64, i64)) -> i64 {
    let zone = i128::from(after.0) - i128::from(before.0);
    let sqlite = i128::from(after.1) - i128::from(before.1);
    i64::try_from(zone - sqlite).unwrap_or(i64::MAX)
}

/// 观测值（不当门）：进程物理足迹 macOS `ri_phys_footprint`。
#[cfg(target_os = "macos")]
fn process_footprint() -> Option<u64> {
    let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    // SAFETY: flavor 与结构体版本一致；内核只写入该结构体。
    let rc = unsafe {
        libc::proc_pid_rusage(
            std::process::id() as libc::c_int,
            libc::RUSAGE_INFO_V2,
            &mut info as *mut libc::rusage_info_v2 as *mut libc::rusage_info_t,
        )
    };
    (rc == 0).then_some(info.ri_phys_footprint)
}
#[cfg(not(target_os = "macos"))]
fn process_footprint() -> Option<u64> {
    None
}

/// P 连接（私有内存副本）自身的 SQLite 堆：页缓存、模式、预编译语句（`sqlite3_db_status`）。
fn connection_sqlite_bytes(conn: &Connection) -> (i64, i64, i64) {
    let status = |op: i32| -> i64 {
        let (mut cur, mut hi) = (0, 0);
        // SAFETY: 句柄属于存活连接；只读统计，不重置。
        let rc =
            unsafe { rusqlite::ffi::sqlite3_db_status(conn.handle(), op, &mut cur, &mut hi, 0) };
        if rc == 0 {
            i64::from(cur)
        } else {
            -1
        }
    };
    (
        status(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_USED),
        status(rusqlite::ffi::SQLITE_DBSTATUS_SCHEMA_USED),
        status(rusqlite::ffi::SQLITE_DBSTATUS_STMT_USED),
    )
}

/// R9 保留量（F1）：只计私有副本 P 的 SQLite 堆与审计状态的保留堆。会话不保留原件镜像（A6-2），
/// `retained_original_image_heap_bytes` 恒为 0，留在回执里是 r7 接口 `retained_keys` 的形状。
#[derive(Clone, Debug)]
struct Retained {
    copy_cache: i64,
    copy_schema: i64,
    copy_stmt: i64,
    audited_state: i64,
    /// financial-closure 闭包表（摘要数组容量 + 键 + 表头）；启动时为 0。
    closure_table: i64,
}
impl Retained {
    fn total(&self) -> u64 {
        [
            self.copy_cache,
            self.copy_schema,
            self.copy_stmt,
            self.audited_state,
            self.closure_table,
        ]
        .iter()
        .map(|n| u64::try_from(*n).unwrap_or(0))
        .sum()
    }
    fn valid(&self) -> bool {
        self.copy_cache >= 0 && self.copy_schema >= 0 && self.copy_stmt >= 0
    }
    fn json(&self) -> Value {
        json!({"audited_copy_sqlite_cache_bytes":self.copy_cache.to_string(),
            "audited_copy_sqlite_schema_bytes":self.copy_schema.to_string(),
            "audited_copy_sqlite_stmt_bytes":self.copy_stmt.to_string(),
            "audited_state_heap_bytes":self.audited_state.to_string(),
            "retained_original_image_heap_bytes":"0",
            "closure_table_bytes":self.closure_table.to_string(),
            "total_bytes":self.total().to_string()})
    }
}
const RETAINED_METHOD: &str = "total = P 的 sqlite3_db_status(CACHE_USED+SCHEMA_USED+STMT_USED) + 审计状态堆 + financial-closure 闭包表（摘要数组容量+键+表头，建表时核门）；审计状态堆 = state::audit(&tx) 前后 Δmalloc_zone_statistics(NULL).size_in_use − Δsqlite3_memory_used；不保留原件镜像（retained_original_image_heap_bytes 恒为 0）。启动单线程，读数可复现；ri_phys_footprint 只作观测，不当门";

fn observed_json() -> Value {
    let heap = heap_probe().ok();
    json!({"process_phys_footprint_bytes":process_footprint().map(|n| n.to_string()),
        "malloc_zone_size_in_use_bytes":heap.map(|h| h.0.to_string()),
        "sqlite3_memory_used_bytes":heap.map(|h| h.1.to_string()),
        "rusage":rusage_json()})
}

// ---- F4：本请求是否确被期限中断（类型化判据，不按墙钟） ----
thread_local! {
    static BUSY_DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
    static BUSY_GAVE_UP: Cell<bool> = const { Cell::new(false) };
}
/// 活源连接的 busy 处理：截止前每 1 ms 重试；到期放弃并记下“busy 耗尽”。
fn busy_wait(_count: i32) -> bool {
    match BUSY_DEADLINE.with(Cell::get) {
        Some(deadline) if Instant::now() < deadline => {
            std::thread::sleep(Duration::from_millis(1));
            true
        }
        _ => {
            BUSY_GAVE_UP.with(|g| g.set(true));
            false
        }
    }
}

/// 一致性核验的失败分型。`Inconsistent` 使会话失效；`Incomplete` 只拒本次。
#[derive(Debug)]
enum Fault {
    Inconsistent(String),
    Incomplete(String),
}
impl Fault {
    fn text(self) -> String {
        match self {
            Self::Inconsistent(e) | Self::Incomplete(e) => e,
        }
    }
}

/// 一次请求在活源连接上的期限装置：progress handler 到期中断并置位；busy handler 到期放弃并置位。
struct SourceBound {
    interrupted: Arc<AtomicBool>,
}
impl SourceBound {
    fn install(source: &Connection, deadline: Instant) -> Result<Self, Fault> {
        let interrupted = Arc::new(AtomicBool::new(false));
        BUSY_DEADLINE.with(|d| d.set(Some(deadline)));
        BUSY_GAVE_UP.with(|g| g.set(false));
        let flag = interrupted.clone();
        source
            .busy_handler(Some(busy_wait))
            .and_then(|()| {
                source.progress_handler(
                    PROGRESS_OPS,
                    Some(move || {
                        let late = Instant::now() >= deadline;
                        if late {
                            flag.store(true, Ordering::SeqCst);
                        }
                        late
                    }),
                )
            })
            .map_err(|e| Fault::Inconsistent(missing(format!("活源期限装置失败：{e}"))))?;
        Ok(Self { interrupted })
    }
    /// SQLite 操作失败的分型：本请求确被中断或 busy 耗尽才算未完成，其他一律视为不一致。
    fn sql(&self, stage: &str, e: impl std::fmt::Display) -> Fault {
        let e = e.to_string();
        let text = missing(format!("{stage}：{}", bare(&e)));
        if self.interrupted.load(Ordering::SeqCst) || BUSY_GAVE_UP.with(Cell::get) {
            Fault::Incomplete(text)
        } else {
            Fault::Inconsistent(text)
        }
    }
    fn remove(source: &Connection) {
        let _ = source.progress_handler(0, None::<fn() -> bool>);
        BUSY_DEADLINE.with(|d| d.set(None));
    }
}

// ---- F5：整体截止的套接字读写 ----
#[derive(Debug, PartialEq, Eq)]
enum IoFault {
    TimedOut,
    Closed,
    Other(String),
}
impl IoFault {
    fn text(&self, what: &str) -> String {
        match self {
            Self::TimedOut => {
                incomplete("SessionClientTimeout", format!("{what}超过本请求整体截止"))
            }
            Self::Closed => invalid(format!("{what}：对端提前关闭")),
            Self::Other(e) => invalid(format!("{what}：{e}")),
        }
    }
}
fn remaining(deadline: Instant) -> Result<Duration, IoFault> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        Err(IoFault::TimedOut)
    } else {
        Ok(left)
    }
}
/// 设本次读/写的超时为距整体截止的剩余时间。macOS 在对端已关闭的套接字上 setsockopt 返回 EINVAL；
/// 此时缓冲区里已有的字节或 EOF 会立即返回，不会阻塞，所以照常继续读写。
fn arm_timeout(s: &UnixStream, deadline: Instant, read: bool) -> Result<(), IoFault> {
    let left = Some(remaining(deadline)?);
    let set = if read {
        s.set_read_timeout(left)
    } else {
        s.set_write_timeout(left)
    };
    match set {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(libc::EINVAL) => Ok(()),
        Err(e) => Err(IoFault::Other(e.to_string())),
    }
}
fn io_kind(e: std::io::Error) -> Option<IoFault> {
    match e.kind() {
        std::io::ErrorKind::Interrupted => None,
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => Some(IoFault::TimedOut),
        _ => Some(IoFault::Other(e.to_string())),
    }
}
/// 每次 read 前把超时设为距整体截止的剩余时间；滴流客户端不能借逐次重置超时拖过截止。
fn read_full(s: &mut UnixStream, buf: &mut [u8], deadline: Instant) -> Result<(), IoFault> {
    let mut off = 0;
    while off < buf.len() {
        arm_timeout(s, deadline, true)?;
        match s.read(&mut buf[off..]) {
            Ok(0) => return Err(IoFault::Closed),
            Ok(n) => off += n,
            Err(e) => {
                if let Some(fault) = io_kind(e) {
                    return Err(fault);
                }
            }
        }
    }
    Ok(())
}
fn write_full(s: &mut UnixStream, bytes: &[u8], deadline: Instant) -> Result<(), IoFault> {
    let mut off = 0;
    while off < bytes.len() {
        arm_timeout(s, deadline, false)?;
        match s.write(&bytes[off..]) {
            Ok(0) => return Err(IoFault::Closed),
            Ok(n) => off += n,
            Err(e) => {
                if let Some(fault) = io_kind(e) {
                    return Err(fault);
                }
            }
        }
    }
    Ok(())
}
#[derive(Debug)]
enum FrameFault {
    Io(IoFault),
    TooLarge(usize),
}
fn read_frame(s: &mut UnixStream, max: usize, deadline: Instant) -> Result<Vec<u8>, FrameFault> {
    let mut len = [0u8; 4];
    read_full(s, &mut len, deadline).map_err(FrameFault::Io)?;
    let n = u32::from_be_bytes(len) as usize;
    if n > max {
        return Err(FrameFault::TooLarge(n));
    }
    let mut b = vec![0u8; n];
    read_full(s, &mut b, deadline).map_err(FrameFault::Io)?;
    Ok(b)
}
fn write_frame(s: &mut UnixStream, bytes: &[u8], deadline: Instant) -> Result<(), IoFault> {
    let len = u32::try_from(bytes.len()).map_err(|_| IoFault::Other("帧超过u32".into()))?;
    let mut frame = Vec::with_capacity(bytes.len() + 4);
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(bytes);
    write_full(s, &frame, deadline)
}

#[cfg(target_os = "macos")]
fn peer_uid_matches(stream: &UnixStream) -> bool {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: fd 属于存活的 stream；只写两个输出参数。
    let rc = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    rc == 0 && uid == unsafe { libc::geteuid() }
}
#[cfg(not(target_os = "macos"))]
fn peer_uid_matches(_stream: &UnixStream) -> bool {
    false
}

#[derive(Default)]
struct Timing {
    pre_stat_us: u64,
    source_us: u64,
    serve_us: u64,
    post_us: u64,
    closure: super::financial_reader::ClosureStats,
}

/// 启动参数（CLI 与测试共用）。`binary_sha256` 由 CLI 取自身自哈希。
pub(super) struct StartConfig {
    pub db: PathBuf,
    pub resources: SessionResources,
    pub custody: Custody,
    pub binary_sha256: String,
}

/// 停机方式。只有 `Graceful` 可能得到 PASS。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shutdown {
    Graceful,
    Sigterm,
}

/// 计数（FINAL 与日志共用）。
#[derive(Default, Clone, Debug)]
struct Counts {
    connections: u64,
    requests: u64,
    served: u64,
    refused: u64,
    refused_before_ready: u64,
    client_timeouts: u64,
}

pub(super) struct Session {
    db: PathBuf,
    wal: PathBuf,
    source: Connection,
    audited_copy: Connection,
    audited: state::AuditedState,
    v_star: i64,
    main_stat: FileStat,
    wal_stat: Option<FileStat>,
    control: Vec<u8>,
    binary_sha256: String,
    resources: SessionResources,
    custody_first: Custody,
    retained: Retained,
    stale: Option<String>,
    counts: Counts,
    started: Instant,
    receipt: Value,
    receipt_sha256: String,
    /// financial-closure 的唯一一张闭包表（换键即丢旧表）。
    closure: Option<super::financial_reader::ClosureTable>,
}

/// 一次请求的结果：回复字节，或拒绝原因（会话是否因此失效记在 `Session::stale`）。
pub(super) type Reply = Result<Vec<u8>, String>;

/// 一个连接处理完后的去向。
pub(super) enum Conn {
    Done,
    /// 停机请求：调用方先 `finish` 并落 FINAL，再在同一截止内把 FINAL 回给该连接。
    Stop(UnixStream, Instant),
}

impl Session {
    /// 启动步骤（DESIGN-R6 §2.1）。任一步失败即不提供服务，不降级。
    pub(super) fn start(config: StartConfig) -> Result<Self, String> {
        let started = Instant::now();
        let r = &config.resources;
        let deadline = started
            .checked_add(Duration::from_millis(r.startup_timeout_ms))
            .ok_or_else(|| invalid("启动窗溢出"))?;
        let db = config.db.clone();
        let wal = sibling(&db, "-wal");
        if Some(config.custody.db.as_str()) != db.to_str() {
            return Err(missing("写者收尾托管的db路径与--db不同"));
        }
        // 首次托管：主库与 WAL 的 SHA 与 stat 必须等于写者收尾托管，且托管期间 stat 不变。
        let t = Instant::now();
        let custody_first = Custody::observe(&db, Some(deadline))?;
        let custody_ms = ms(t);
        if !custody_first.same_files(&config.custody) {
            return Err(missing("会话首次托管与写者收尾托管不同（SHA或stat）"));
        }
        let main_stat = custody_first.main.stat.clone();
        let wal_stat = custody_first.wal.as_ref().map(|w| w.stat.clone());
        let t = Instant::now();
        let mut source =
            capture::session_open_source(&db, r.max_capture_bytes, r.startup_timeout_ms, deadline)?;
        let open_ms = ms(t);
        let t = Instant::now();
        let cap = capture::session_capture(
            &mut source,
            &db,
            r.max_capture_bytes,
            deadline,
            control_bytes,
            heap_probe,
        )?;
        let capture_ms = ms(t);
        if FileStat::of(&db)? != main_stat || FileStat::optional(&wal)? != wal_stat {
            return Err(missing("捕获期间DB或WAL的stat改变"));
        }
        let mut audited_copy = cap.db;
        let page_count: i64 = audited_copy
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .map_err(invalid)?;
        let page_size: i64 = audited_copy
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .map_err(invalid)?;
        let t = Instant::now();
        let (audited, anchor, audit_ms, audited_state) = {
            let tx = audited_copy.transaction().map_err(invalid)?;
            super::financial_reader::check_markers(&tx)?;
            // 与 financial_reader 冷路径同一调用；P 上 import_image 留下的 progress handler 以启动窗
            // 中断 SQLite 读取，审计返回后再核一次期限。审计前后的堆读数只用于 R9 记账。
            let before = heap_probe()?;
            let audited = state::audit(&tx)?;
            let after = heap_probe()?;
            let audit_ms = ms(t);
            check_deadline(deadline, "SessionStartupDeadline")?;
            let anchor = super::financial_reader::session_anchor(&tx, &audited)?;
            if control_bytes(&tx).map_err(missing)? != cap.control {
                return Err(missing("控制原件K0与已审副本不同"));
            }
            (audited, anchor, audit_ms, rust_heap_delta(before, after))
        };
        audited_copy
            .progress_handler(0, None::<fn() -> bool>)
            .map_err(invalid)?;
        check_deadline(deadline, "SessionStartupDeadline")?;
        let (copy_cache, copy_schema, copy_stmt) = connection_sqlite_bytes(&audited_copy);
        let retained = Retained {
            copy_cache,
            copy_schema,
            copy_stmt,
            audited_state,
            closure_table: 0,
        };
        if !retained.valid() {
            return Err(missing("sqlite3_db_status 读数失败"));
        }
        if retained.total() > r.retained_bytes {
            return Err(incomplete(
                "SessionRetained",
                format!(
                    "P 与审计状态的保留字节 {} 超过 retained_bytes {}",
                    retained.total(),
                    r.retained_bytes
                ),
            ));
        }
        let mut session = Self {
            db: db.clone(),
            wal,
            source,
            audited_copy,
            audited,
            v_star: cap.versions[0],
            main_stat,
            wal_stat,
            control: cap.control,
            binary_sha256: config.binary_sha256.clone(),
            resources: config.resources.clone(),
            custody_first,
            retained,
            stale: None,
            counts: Counts::default(),
            started,
            receipt: Value::Null,
            receipt_sha256: String::new(),
            closure: None,
        };
        session.post_check().map_err(Fault::text)?;
        session.receipt = json!({
            "schema":STARTUP_SCHEMA,
            "status":"READY",
            "candidate":"#1394 DESIGN-R6 c2 + 第三问选项 A financial-closure；APPROVED-SOURCE §5.7 修订记录 A6 r2（0e620363…）",
            "consistency":SESSION_CONSISTENCY,
            "anchor_mismatch_policy":ANCHOR_MISMATCH_POLICY,
            "binary_sha256":session.binary_sha256,
            "pid":std::process::id().to_string(),
            "db":db.display().to_string(),
            "db_stat":session.main_stat.json(),
            "wal_stat":optional_stat_json(&session.wal_stat),
            "custody_first":session.custody_first.to_json(),
            "resources":session.resources.to_json(),
            "resources_sha256":sha_hex(canonical_json(&session.resources.to_json()).as_bytes()),
            "data_version":cap.versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            "control_bytes":session.control.len().to_string(),
            "control_sha256":sha_hex(&session.control),
            "anchor":anchor,
            "capture_digest":cap.digest.as_str(),
            "captured_bytes":cap.captured_bytes.to_string(),
            "captured_image_heap_bytes":cap.image_heap_bytes.to_string(),
            "retained_original_accounted_bytes":"0",
            "audited_copy":{"page_count":page_count.to_string(),"page_size":page_size.to_string()},
            "retained":session.retained.json(),
            "retained_method":RETAINED_METHOD,
            "observed":observed_json(),
            "ms":{"custody":format!("{custody_ms:.3}"),"open_source":format!("{open_ms:.3}"),
                  "capture_and_import":format!("{capture_ms:.3}"),"audit":format!("{audit_ms:.3}"),
                  "startup_total":format!("{:.3}", ms(started))},
        });
        session.receipt_sha256 = sha_hex(canonical_json(&session.receipt).as_bytes());
        Ok(session)
    }

    pub(super) fn receipt(&self) -> &Value {
        &self.receipt
    }
    #[cfg(test)]
    pub(super) fn stale_reason(&self) -> Option<&str> {
        self.stale.as_deref()
    }

    fn invalidate(&mut self, reason: String) -> String {
        if self.stale.is_none() {
            self.stale = Some(reason.clone());
        }
        reason
    }
    fn stale_refusal(&self) -> Option<String> {
        self.stale
            .as_deref()
            .map(|reason| missing(format!("会话已失效：{}", bare(reason))))
    }

    /// R4：主库与 WAL 的身份与 stat 等于启动值（显式比较）。
    fn check_stat(&self) -> Result<(), String> {
        if FileStat::of(&self.db).map_err(|e| missing(format!("R4：{}", bare(&e))))?
            != self.main_stat
        {
            return Err(missing("R4：主库身份或stat改变"));
        }
        if FileStat::optional(&self.wal).map_err(|e| missing(format!("R4：{}", bare(&e))))?
            != self.wal_stat
        {
            return Err(missing("R4：WAL身份或stat改变"));
        }
        Ok(())
    }

    /// 回复写出前：R5 v3 与 R4 再核；失败都是不一致。
    fn post_check(&self) -> Result<(), Fault> {
        let v3 = capture::version(&self.source)
            .map_err(|e| Fault::Inconsistent(missing(format!("R5：{}", bare(&e)))))?;
        if v3 != self.v_star {
            return Err(Fault::Inconsistent(missing("R5：回复前data_version改变")));
        }
        self.check_stat().map_err(Fault::Inconsistent)
    }

    /// R4 → R2/R3 → R5 → R6，全在活源连接 S 的一个新短读事务里；不持有跨请求事务。
    fn check_source(&mut self, deadline: Instant, t: &mut Timing) -> Result<(), Fault> {
        let a = Instant::now();
        self.check_stat().map_err(Fault::Inconsistent)?;
        t.pre_stat_us = us(a);
        let b = Instant::now();
        let bound = SourceBound::install(&self.source, deadline)?;
        let checked = self.check_source_bounded(&bound);
        SourceBound::remove(&self.source);
        checked?;
        t.source_us = us(b);
        Ok(())
    }
    fn check_source_bounded(&mut self, bound: &SourceBound) -> Result<(), Fault> {
        let v_star = self.v_star;
        capture::validate_source(&self.source).map_err(|e| bound.sql("R2/R3", e))?;
        let v0 = capture::version(&self.source).map_err(|e| bound.sql("R5", e))?;
        if v0 != v_star {
            return Err(Fault::Inconsistent(missing("R5：BEGIN前data_version改变")));
        }
        let tx = self.source.transaction().map_err(|e| bound.sql("R5", e))?;
        let control = control_bytes(&tx).map_err(|e| bound.sql("R6", e))?;
        let v1 = capture::version(&tx).map_err(|e| bound.sql("R5", e))?;
        if v1 != v_star {
            return Err(Fault::Inconsistent(missing("R5：首读后data_version改变")));
        }
        if control != self.control {
            return Err(Fault::Inconsistent(missing(
                "R6：控制原件与启动K0逐字节不同",
            )));
        }
        super::financial_reader::check_current_control(&tx, &self.audited)
            .map_err(|e| bound.sql("R6", e))?;
        tx.rollback().map_err(|e| bound.sql("R5", e))?;
        let v2 = capture::version(&self.source).map_err(|e| bound.sql("R5", e))?;
        if v2 != v_star {
            return Err(Fault::Inconsistent(missing(
                "R5：ROLLBACK后data_version改变",
            )));
        }
        Ok(())
    }

    /// R8：从已审副本 P 的新短事务供片；判据与冷路径同一函数，审计结果来自启动那一次。
    fn serve_audited(
        &mut self,
        command: &str,
        request: &Value,
        deadline: Instant,
        stats: &mut super::financial_reader::ClosureStats,
    ) -> Result<Value, String> {
        let query_only: i64 = self
            .audited_copy
            .query_row("PRAGMA query_only", [], |row| row.get(0))
            .map_err(|e| missing(format!("已审副本：{e}")))?;
        if query_only != 1 {
            return Err(missing("已审副本只读设置改变"));
        }
        let interrupted = Arc::new(AtomicBool::new(false));
        let flag = interrupted.clone();
        self.audited_copy
            .progress_handler(
                PROGRESS_OPS,
                Some(move || {
                    let late = Instant::now() >= deadline;
                    if late {
                        flag.store(true, Ordering::SeqCst);
                    }
                    late
                }),
            )
            .map_err(invalid)?;
        let audited = &self.audited;
        // 闭包表计入 R9：表可用的保留额度 = retained_bytes − 启动保留量（不含旧表；换键先丢旧表）。
        let bounds = super::financial_reader::ClosureBounds {
            max_nodes: self.resources.max_closure_nodes,
            max_table_bytes: self.resources.retained_bytes.saturating_sub(
                Retained {
                    closure_table: 0,
                    ..self.retained.clone()
                }
                .total(),
            ),
        };
        let table = &mut self.closure;
        let out = (|| {
            let tx = self.audited_copy.transaction().map_err(invalid)?;
            super::financial_reader::check_markers(&tx)?;
            match command {
                "financial-basis-ref" => {
                    super::financial_reader::basis_ref_with(&tx, request, Some(audited))
                }
                "financial-content" => super::financial_reader::content_with(
                    &tx,
                    request,
                    sc::MAX_FRAME_LEN,
                    Some(audited),
                ),
                "financial-closure" => super::financial_reader::closure_with(
                    &tx,
                    request,
                    Some(audited),
                    table,
                    bounds,
                    stats,
                ),
                _ => Err(invalid("未知财务命令")),
            }
        })();
        self.retained.closure_table = self
            .closure
            .as_ref()
            .map_or(0, |t| i64::try_from(t.retained_bytes()).unwrap_or(i64::MAX));
        self.audited_copy
            .progress_handler(0, None::<fn() -> bool>)
            .map_err(invalid)?;
        match out {
            Err(e) if interrupted.load(Ordering::SeqCst) => Err(incomplete(
                "SessionRequestDeadline",
                format!("供片未在请求期限内完成：{}", bare(&e)),
            )),
            other => other,
        }
    }

    /// 处理一个已读入的财务请求（帧头已核 exact）。返回回复字节或拒绝原因。
    fn handle(&mut self, header: &Value, body: &[u8], deadline: Instant, t: &mut Timing) -> Reply {
        if let Some(refusal) = self.stale_refusal() {
            return Err(refusal);
        }
        let command = header["command"].as_str().unwrap_or_default().to_owned();
        // R7：客户端二进制与 --db 身份。
        let dev = header["db_dev"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok());
        let ino = header["db_ino"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok());
        if header["binary_sha256"].as_str() != Some(self.binary_sha256.as_str()) {
            return Err(self.invalidate(missing("R7：客户端二进制SHA不等于会话启动回执")));
        }
        if dev != Some(self.main_stat.dev) || ino != Some(self.main_stat.ino) {
            return Err(self.invalidate(missing("R7：客户端--db身份与会话不同")));
        }
        match self.check_source(deadline, t) {
            Ok(()) => {}
            // F4：只有本请求的读取确被中断或 busy 耗尽才只拒本次；显式比较得出的不一致一律失效。
            Err(Fault::Incomplete(e)) => {
                return Err(incomplete(
                    "SessionRequestDeadline",
                    format!("一致性核验未在请求期限内完成：{}", bare(&e)),
                ))
            }
            Err(Fault::Inconsistent(e)) => return Err(self.invalidate(e)),
        }
        let request = strict_json(body).map_err(invalid)?;
        let s = Instant::now();
        // R7(b)：anchor／会话身份不符与其他请求级拒绝一样只拒本次，会话保持有效（ANCHOR_MISMATCH_POLICY）。
        let out = self.serve_audited(&command, &request, deadline, &mut t.closure)?;
        let mut bytes = canonical_json(&out).into_bytes();
        bytes.push(b'\n');
        t.serve_us = us(s);
        if bytes.len() as u64 > self.resources.max_reply_bytes {
            return Err(incomplete(
                "SessionReplyBytes",
                "回复超过会话 max_reply_bytes",
            ));
        }
        let p = Instant::now();
        if let Err(f) = self.post_check() {
            return Err(self.invalidate(f.text()));
        }
        t.post_us = us(p);
        check_deadline(deadline, "SessionRequestDeadline")?;
        Ok(bytes)
    }

    /// 一个连接：帧头 →（财务请求）请求体 → 回复帧，全部在 accept 起算的整体截止内。
    /// 慢客户端超时即断开并计数（不使会话失效）。停机请求交回调用方处理。
    pub(super) fn serve_connection(&mut self, mut stream: UnixStream, log: &mut dyn Write) -> Conn {
        let accepted = Instant::now();
        let deadline = accepted + Duration::from_millis(self.resources.request_timeout_ms);
        self.counts.connections += 1;
        let mut t = Timing::default();
        let mut command = String::new();
        let mut request_bytes = 0usize;
        let mut timed_out = false;
        let reply: Reply = (|| {
            if !peer_uid_matches(&stream) {
                return Err(self.invalidate(missing("对端uid不是会话属主")));
            }
            let raw = match read_frame(&mut stream, MAX_HEADER_FRAME, deadline) {
                Ok(raw) => raw,
                Err(FrameFault::TooLarge(_)) => return Err(self.invalidate(invalid("帧头超界"))),
                Err(FrameFault::Io(e)) => {
                    timed_out = e == IoFault::TimedOut;
                    return Err(e.text("读帧头"));
                }
            };
            let header = match strict_json(&raw) {
                Ok(h) => h,
                Err(e) => return Err(self.invalidate(invalid(format!("帧头非严格JSON：{e}")))),
            };
            command = header["command"].as_str().unwrap_or_default().to_owned();
            if command == "stop" {
                if let Err(e) = exact(&header, &STOP_FIELDS, "停止帧头") {
                    return Err(self.invalidate(e));
                }
                if header["schema_revision"] != REQUEST_SCHEMA {
                    return Err(self.invalidate(invalid("停止帧头版本不符")));
                }
                if header["binary_sha256"].as_str() != Some(self.binary_sha256.as_str()) {
                    self.invalidate(missing("停止请求的二进制SHA不等于会话启动回执"));
                }
                return Ok(Vec::new());
            }
            if let Err(e) = exact(&header, &REQUEST_FIELDS, "请求帧头") {
                return Err(self.invalidate(e));
            }
            if header["schema_revision"] != REQUEST_SCHEMA
                || !matches!(
                    command.as_str(),
                    "financial-basis-ref" | "financial-content" | "financial-closure"
                )
            {
                return Err(self.invalidate(invalid("请求帧头版本或命令不符")));
            }
            self.counts.requests += 1;
            if self.counts.requests > self.resources.max_requests {
                return Err(
                    self.invalidate(incomplete("SessionRequests", "会话请求数超过 max_requests"))
                );
            }
            let max = usize::try_from(self.resources.max_request_bytes).unwrap_or(usize::MAX);
            let body = match read_frame(&mut stream, max, deadline) {
                Ok(body) => body,
                Err(FrameFault::TooLarge(n)) => {
                    request_bytes = n;
                    return Err(incomplete(
                        "SessionRequestBytes",
                        "请求超过会话 max_request_bytes",
                    ));
                }
                Err(FrameFault::Io(e)) => {
                    timed_out = e == IoFault::TimedOut;
                    return Err(e.text("读请求体"));
                }
            };
            request_bytes = body.len();
            self.handle(&header, &body, deadline, &mut t)
        })();
        if command == "stop" && reply.is_ok() {
            return Conn::Stop(stream, deadline);
        }
        let mut frame = Vec::new();
        let (ok, reply_bytes, error) = match &reply {
            Ok(bytes) => {
                frame.reserve(bytes.len() + 1);
                frame.push(0u8);
                frame.extend_from_slice(bytes);
                (true, bytes.len(), None)
            }
            Err(e) => {
                frame.push(1u8);
                frame.extend_from_slice(e.as_bytes());
                (false, 0, Some(e.clone()))
            }
        };
        let w = Instant::now();
        let written = if timed_out {
            Err(IoFault::TimedOut)
        } else {
            write_frame(&mut stream, &frame, deadline)
        };
        if written == Err(IoFault::TimedOut) {
            timed_out = true;
        }
        if timed_out {
            self.counts.client_timeouts += 1;
        }
        let delivered = ok && written.is_ok();
        if delivered {
            self.counts.served += 1;
        } else {
            self.counts.refused += 1;
        }
        drop(stream);
        let line = json!({"n":self.counts.connections.to_string(),"phase":"ready","command":command,
            "ok":ok,"delivered":delivered,"error":error,"client_timeout":timed_out,
            "write_error":written.err().map(|e| format!("{e:?}")),
            "request_bytes":request_bytes.to_string(),"reply_bytes":reply_bytes.to_string(),
            "us":{"pre_stat":t.pre_stat_us.to_string(),"source":t.source_us.to_string(),
                  "gate":"0","serve":t.serve_us.to_string(),
                  "post":t.post_us.to_string(),"write":us(w).to_string(),"total":us(accepted).to_string(),
                  "closure_enumerate":t.closure.enumerate_us.to_string()},
            "closure_enumerated":t.closure.enumerated,"closure_nodes":t.closure.nodes.to_string(),
            "gate_rows":"0","gate_bytes":"0",
            "stale":self.stale.is_some()});
        if log
            .write_all((canonical_json(&line) + "\n").as_bytes())
            .and_then(|()| log.flush())
            .is_err()
        {
            self.invalidate(missing("会话请求日志写入失败"));
        }
        Conn::Done
    }

    /// 会话末：R5/R4 终核与末次 SHA 托管。只有优雅停机、从未失效、终核通过且首末托管相同才 PASS。
    /// `requests_log` 为请求日志路径（已写完、已 flush）；FINAL 记其字节数、行数与 SHA。
    pub(super) fn finish(&mut self, shutdown: Shutdown, requests_log: Option<&Path>) -> Vec<u8> {
        let final_check = self.post_check().err().map(Fault::text);
        let last = Custody::observe(&self.db, None);
        let custody_equal = last
            .as_ref()
            .is_ok_and(|c| c.same_files(&self.custody_first));
        let pass = shutdown == Shutdown::Graceful
            && self.stale.is_none()
            && final_check.is_none()
            && custody_equal;
        let (copy_cache, copy_schema, copy_stmt) = connection_sqlite_bytes(&self.audited_copy);
        let retained_final = Retained {
            copy_cache,
            copy_schema,
            copy_stmt,
            ..self.retained.clone()
        };
        let c = &self.counts;
        let fin = json!({
            "schema":FINAL_SCHEMA,
            "status": if pass {"PASS"} else {"FAIL"},
            "shutdown": match shutdown {Shutdown::Graceful => "graceful-stop", Shutdown::Sigterm => "SIGTERM-abnormal"},
            "abnormal_shutdown": shutdown != Shutdown::Graceful,
            "ready":true,
            "startup_receipt_sha256":self.receipt_sha256,
            "stale":self.stale,
            "final_check_error":final_check,
            "custody_first":self.custody_first.to_json(),
            "custody_last":last.as_ref().map(Custody::to_json).unwrap_or(Value::Null),
            "custody_last_error":last.err(),
            "custody_equal":custody_equal,
            "counts":{"connections":c.connections.to_string(),"requests":c.requests.to_string(),
                "served":c.served.to_string(),"refused":c.refused.to_string(),
                "refused_before_ready":c.refused_before_ready.to_string(),
                "client_timeouts":c.client_timeouts.to_string()},
            "requests_log":requests_log.map(log_digest),
            "retained_startup":Retained {
                closure_table: 0,
                ..self.retained.clone()
            }
            .json(),
            "retained_final":retained_final.json(),
            "closure_table":self.closure.as_ref().map(|t| json!({"nodes":t.nodes().to_string(),
                "retained_bytes":t.retained_bytes().to_string()})),
            "observed":observed_json(),
            "wall_ms":format!("{:.3}", ms(self.started)),
        });
        let mut bytes = canonical_json(&fin).into_bytes();
        bytes.push(b'\n');
        bytes
    }

    #[cfg(test)]
    fn handle_for_test(&mut self, command: &str, body: &[u8]) -> Reply {
        let deadline = Instant::now() + Duration::from_millis(self.resources.request_timeout_ms);
        self.handle_at(command, body, deadline)
    }
    #[cfg(test)]
    fn handle_at(&mut self, command: &str, body: &[u8], deadline: Instant) -> Reply {
        let header = json!({"schema_revision":REQUEST_SCHEMA,"command":command,
            "binary_sha256":self.binary_sha256,"db_dev":self.main_stat.dev.to_string(),
            "db_ino":self.main_stat.ino.to_string()});
        self.counts.requests += 1;
        if self.counts.requests > self.resources.max_requests {
            return Err(
                self.invalidate(incomplete("SessionRequests", "会话请求数超过 max_requests"))
            );
        }
        self.handle(&header, body, deadline, &mut Timing::default())
    }
}

fn log_digest(path: &Path) -> Value {
    match std::fs::read(path) {
        Ok(b) => json!({"path":path.display().to_string(),"bytes":b.len().to_string(),
            "sha256":sha_hex(&b),"lines":b.iter().filter(|c| **c == b'\n').count().to_string()}),
        Err(e) => json!({"path":path.display().to_string(),"error":e.to_string()}),
    }
}

/// READY 之前到达的连接：在短截止内读完它的帧（避免对端写入时收到 EPIPE），回 NotReady 后关闭。
fn refuse_before_ready(mut stream: UnixStream, max_request: usize) -> (String, bool) {
    let deadline = Instant::now() + Duration::from_millis(PRE_READY_IO_MS);
    let mut command = String::new();
    let mut timed_out = false;
    match read_frame(&mut stream, MAX_HEADER_FRAME, deadline) {
        Ok(raw) => {
            if let Ok(header) = strict_json(&raw) {
                command = header["command"].as_str().unwrap_or_default().to_owned();
                if command != "stop" {
                    if let Err(FrameFault::Io(IoFault::TimedOut)) =
                        read_frame(&mut stream, max_request, deadline)
                    {
                        timed_out = true;
                    }
                }
            }
        }
        Err(FrameFault::Io(IoFault::TimedOut)) => timed_out = true,
        Err(_) => {}
    }
    let mut frame = vec![1u8];
    frame.extend_from_slice(missing(NOT_READY).as_bytes());
    if write_frame(&mut stream, &frame, deadline) == Err(IoFault::TimedOut) {
        timed_out = true;
    }
    (command, timed_out)
}

struct Flags(std::collections::BTreeMap<String, String>);
fn parse_flags(args: &[String], allowed: &[&str], what: &str) -> Result<Flags, String> {
    if args.len() != 2 + 2 * allowed.len() {
        return Err(invalid(format!("{what}只接受 {}", allowed.join(" "))));
    }
    let mut map = std::collections::BTreeMap::new();
    for pair in args[2..].chunks_exact(2) {
        if !allowed.contains(&pair[0].as_str()) {
            return Err(invalid(format!("{what}含未知flag")));
        }
        if pair[1].is_empty() || pair[1].starts_with("--") {
            return Err(invalid("CLI flag缺少值"));
        }
        if map.insert(pair[0].clone(), pair[1].clone()).is_some() {
            return Err(invalid(format!("{what}含重复flag")));
        }
    }
    if map.get("--protocol-revision").map(String::as_str) != Some(PROTOCOL_REVISION) {
        return Err(invalid("必须显式选择s-session/3"));
    }
    Ok(Flags(map))
}
fn absolute(value: &str, what: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(invalid(format!("{what}必须为绝对路径")));
    }
    Ok(path)
}
fn read_json_file(path: &Path, what: &str) -> Result<(Value, String), String> {
    let bytes = std::fs::read(path).map_err(|e| invalid(format!("读{what}：{e}")))?;
    let value = strict_json(&bytes).map_err(|e| invalid(format!("{what}非严格JSON：{e}")))?;
    Ok((value, sha_hex(&bytes)))
}

/// `gate`、`gate_rows`、`gate_bytes`（requests.jsonl）恒为 "0"：原件字节门已删（A6-2），键留作 r7 接口形状。
const SERVE_FLAGS: [&str; 6] = [
    "--protocol-revision",
    "--db",
    "--socket",
    "--resources",
    "--custody",
    "--receipts",
];

/// 会话 socket：父目录须为本 euid 所有、权限无组/其他位；路径未被占用且短于 sun_path。
fn prepare_socket(path: &Path) -> Result<UnixListener, String> {
    if path.as_os_str().len() > MAX_SOCKET_PATH {
        return Err(invalid("socket路径超过sun_path长度"));
    }
    let parent = path.parent().ok_or_else(|| invalid("socket缺父目录"))?;
    let meta =
        std::fs::symlink_metadata(parent).map_err(|e| invalid(format!("socket目录：{e}")))?;
    // SAFETY: 无参只读。
    let euid = unsafe { libc::geteuid() };
    if !meta.file_type().is_dir() || meta.uid() != euid || meta.permissions().mode() & 0o077 != 0 {
        return Err(invalid("socket目录须为本用户所有且权限0700"));
    }
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(invalid("socket路径已存在"));
    }
    let listener = UnixListener::bind(path).map_err(|e| invalid(format!("bind socket：{e}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| invalid(format!("socket非阻塞：{e}")))?;
    Ok(listener)
}

// ---- F3：SIGTERM 自管道 ----
static SIGNAL_PIPE_WR: AtomicI32 = AtomicI32::new(-1);
extern "C" fn on_sigterm(_: libc::c_int) {
    let fd = SIGNAL_PIPE_WR.load(Ordering::Relaxed);
    if fd >= 0 {
        // SAFETY: write 是 async-signal-safe；写端非阻塞，满了就丢（管道里已有待处理字节）。
        let _ = unsafe { libc::write(fd, b"T".as_ptr().cast(), 1) };
    }
}
fn pipe_pair() -> Result<(RawFd, RawFd), String> {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: pipe 写入两个 fd。
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(invalid("pipe失败"));
    }
    for fd in fds {
        // SAFETY: 对自己刚创建的 fd 设 FD_CLOEXEC 与 O_NONBLOCK。
        unsafe {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
    Ok((fds[0], fds[1]))
}
fn install_sigterm() -> Result<RawFd, String> {
    let (rd, wr) = pipe_pair()?;
    SIGNAL_PIPE_WR.store(wr, Ordering::SeqCst);
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = on_sigterm as *const () as libc::sighandler_t;
    action.sa_flags = libc::SA_RESTART;
    // SAFETY: 安装只写自管道的处理函数。
    unsafe { libc::sigemptyset(&mut action.sa_mask) };
    if unsafe { libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut()) } != 0 {
        return Err(invalid("安装SIGTERM处理失败"));
    }
    Ok(rd)
}
/// FINAL 已尽力落盘后，以默认动作重新投递 SIGTERM，使回收方看到“被信号终止”。
fn die_by_sigterm() -> ! {
    // SAFETY: 恢复默认处理并向自己投递。
    unsafe {
        libc::signal(libc::SIGTERM, libc::SIG_DFL);
        libc::raise(libc::SIGTERM);
    }
    std::process::exit(143)
}
/// poll 等待若干 fd 可读；EINTR 重试。
fn wait_readable(fds: &[RawFd]) -> Result<Vec<bool>, String> {
    let mut polls: Vec<libc::pollfd> = fds
        .iter()
        .map(|fd| libc::pollfd {
            fd: *fd,
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    loop {
        // SAFETY: polls 在本帧内存活。
        let rc = unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as libc::nfds_t, -1) };
        if rc >= 0 {
            return Ok(polls.iter().map(|p| p.revents != 0).collect());
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return Err(invalid("poll失败"));
        }
    }
}
fn drain(fd: RawFd) {
    let mut buf = [0u8; 64];
    // SAFETY: 非阻塞读空自管道。
    while unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) } > 0 {}
}
fn create_new(dir: &Path, name: &str) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name))
        .map_err(|e| invalid(format!("建{name}：{e}")))
}
/// 启动阶段失败或被 SIGTERM 打断时的 FINAL（尚无会话，只能给出这些）。
fn startup_final(
    db: &Path,
    shutdown: &str,
    error: Option<&str>,
    log: &Path,
    refused: u64,
) -> Vec<u8> {
    let last = Custody::observe(db, None);
    let fin = json!({"schema":FINAL_SCHEMA,"status":"FAIL","shutdown":shutdown,"abnormal_shutdown":true,
        "ready":false,"error":error,
        "custody_last":last.as_ref().map(Custody::to_json).unwrap_or(Value::Null),
        "custody_last_error":last.err(),
        "counts":{"refused_before_ready":refused.to_string()},
        "requests_log":log_digest(log),
        "observed":observed_json()});
    let mut bytes = canonical_json(&fin).into_bytes();
    bytes.push(b'\n');
    bytes
}
fn write_final(dir: &Path, bytes: &[u8]) {
    if let Ok(mut f) = create_new(dir, "FINAL.json") {
        let _ = f.write_all(bytes).and_then(|()| f.sync_all());
    }
}

/// `financial-serve --protocol-revision s-session/3 --db <abs> --socket <abs> --resources <json>
///  --custody <json> --receipts <空目录>`
///
/// 输出合同：stdout 只在启动完成后输出唯一一行 `READY <canonical JSON>`（`s-financial-session-ready/1`），
/// 其中 `startup_receipt_sha256` = SHA-256(STARTUP.json 去掉末尾 LF 的字节) = SHA-256(canonical_json(回执))。
/// receipts 目录下另有 `requests.jsonl`（每个连接一行，含 READY 前被拒的连接）与 `FINAL.json`（停机时写）。
/// 退出：优雅停机且 FINAL=PASS 退出 0；FINAL=FAIL 或启动失败退出 1；SIGTERM 落 FINAL 后以 SIGTERM 终止。
pub(super) fn serve(args: &[String]) -> Result<(), String> {
    let flags = parse_flags(args, &SERVE_FLAGS, "financial-serve")?;
    let f = &flags.0;
    let db = absolute(&f["--db"], "--db")?;
    let socket = absolute(&f["--socket"], "--socket")?;
    let receipts = absolute(&f["--receipts"], "--receipts")?;
    let (resources_json, resources_file_sha256) =
        read_json_file(&absolute(&f["--resources"], "--resources")?, "会话资源")?;
    let resources = SessionResources::from_json(&resources_json)?;
    let (custody_json, custody_file_sha256) =
        read_json_file(&absolute(&f["--custody"], "--custody")?, "写者收尾托管")?;
    let custody = Custody::from_json(&custody_json)?;
    let entries = std::fs::read_dir(&receipts).map_err(|e| invalid(format!("--receipts：{e}")))?;
    if entries.count() != 0 {
        return Err(invalid("--receipts 必须是已存在的空目录"));
    }
    let binary_sha256 = self_sha256()?;
    let log_path = receipts.join("requests.jsonl");
    let log = Arc::new(Mutex::new(create_new(&receipts, "requests.jsonl")?));
    let signal_rd = install_sigterm()?;
    // socket 先绑定；READY 之前到达的连接由守门线程一律拒绝，SIGTERM 也由它尽力落 FINAL。
    let listener = prepare_socket(&socket)?;
    let (ready_rd, ready_wr) = pipe_pair()?;
    let gate = {
        let listener = listener
            .try_clone()
            .map_err(|e| invalid(format!("socket复制：{e}")))?;
        let log = log.clone();
        let (db, receipts, log_path, socket) = (
            db.clone(),
            receipts.clone(),
            log_path.clone(),
            socket.clone(),
        );
        let max_request = usize::try_from(resources.max_request_bytes).unwrap_or(usize::MAX);
        std::thread::spawn(move || -> u64 {
            let mut refused = 0u64;
            loop {
                let Ok(ready) = wait_readable(&[signal_rd, ready_rd, listener.as_raw_fd()]) else {
                    return refused;
                };
                if ready[0] {
                    let bytes =
                        startup_final(&db, "SIGTERM-during-startup", None, &log_path, refused);
                    write_final(&receipts, &bytes);
                    let _ = std::fs::remove_file(&socket);
                    die_by_sigterm();
                }
                if ready[1] {
                    return refused;
                }
                if ready[2] {
                    if let Ok((stream, _)) = listener.accept() {
                        let _ = stream.set_nonblocking(false);
                        let (command, timed_out) = refuse_before_ready(stream, max_request);
                        refused += 1;
                        let line = json!({"n":"0","phase":"before_ready","command":command,"ok":false,
                            "delivered":false,"error":missing(NOT_READY),"client_timeout":timed_out});
                        if let Ok(mut file) = log.lock() {
                            let _ = file.write_all((canonical_json(&line) + "\n").as_bytes());
                        }
                    }
                }
            }
        })
    };
    let started = Session::start(StartConfig {
        db: db.clone(),
        resources,
        custody,
        binary_sha256,
    });
    // 就绪转换：先让守门线程退出（此后到达的连接都在 READY 输出之后才被处理），再写回执、输出 READY。
    // SAFETY: 向自己的管道写一个字节。
    let _ = unsafe { libc::write(ready_wr, b"R".as_ptr().cast(), 1) };
    let refused_before_ready = gate.join().unwrap_or(0);
    let mut session = match started {
        Ok(session) => session,
        Err(e) => {
            let bytes = startup_final(
                &db,
                "startup-failed",
                Some(&e),
                &log_path,
                refused_before_ready,
            );
            write_final(&receipts, &bytes);
            let _ = std::fs::remove_file(&socket);
            return Err(e);
        }
    };
    session.counts.refused_before_ready = refused_before_ready;
    let mut receipt = session.receipt().clone();
    receipt["socket"] = json!(socket.display().to_string());
    receipt["resources_file_sha256"] = json!(resources_file_sha256);
    receipt["custody_file_sha256"] = json!(custody_file_sha256);
    receipt["refused_before_ready"] = json!(refused_before_ready.to_string());
    let canonical = canonical_json(&receipt);
    session.receipt = receipt.clone();
    session.receipt_sha256 = sha_hex(canonical.as_bytes());
    {
        let mut file = create_new(&receipts, "STARTUP.json")?;
        file.write_all((canonical.clone() + "\n").as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(invalid)?;
    }
    {
        let ready = json!({"schema":READY_SCHEMA,"startup_receipt_sha256":session.receipt_sha256,
            "startup_receipt_path":receipts.join("STARTUP.json").display().to_string(),
            "startup_receipt":receipt});
        let mut out = std::io::stdout().lock();
        writeln!(out, "READY {}", canonical_json(&ready)).map_err(invalid)?;
        out.flush().map_err(invalid)?;
    }
    loop {
        let ready = wait_readable(&[signal_rd, listener.as_raw_fd()])?;
        if ready[0] {
            drain(signal_rd);
            let bytes = session.finish(Shutdown::Sigterm, Some(&log_path));
            write_final(&receipts, &bytes);
            let _ = std::fs::remove_file(&socket);
            die_by_sigterm();
        }
        if !ready[1] {
            continue;
        }
        let Ok((stream, _)) = listener.accept() else {
            continue;
        };
        let _ = stream.set_nonblocking(false);
        let outcome = {
            let mut guard = log.lock().map_err(|_| invalid("日志锁中毒"))?;
            session.serve_connection(stream, &mut *guard)
        };
        if let Conn::Stop(mut stream, deadline) = outcome {
            let bytes = session.finish(Shutdown::Graceful, Some(&log_path));
            write_final(&receipts, &bytes);
            let mut frame = vec![0u8];
            frame.extend_from_slice(&bytes);
            let _ = write_frame(&mut stream, &frame, deadline);
            drop(stream);
            let _ = std::fs::remove_file(&socket);
            let fin = strict_json(&bytes).map_err(invalid)?;
            if fin["status"] != "PASS" {
                return Err(missing("会话终态 FAIL（见 FINAL.json）"));
            }
            return Ok(());
        }
    }
}

fn connect(socket: &str) -> Result<UnixStream, String> {
    UnixStream::connect(socket).map_err(|e| missing(format!("会话不可达：{e}")))
}
fn read_reply(stream: &mut UnixStream, deadline: Instant) -> Reply {
    let reply = read_frame(stream, CLIENT_MAX_REPLY, deadline).map_err(|f| match f {
        FrameFault::Io(e) => missing(format!("读会话回复：{e:?}")),
        FrameFault::TooLarge(_) => missing("会话回复帧超界"),
    })?;
    match reply.first() {
        Some(0) => Ok(reply[1..].to_vec()),
        Some(1) => Err(String::from_utf8_lossy(&reply[1..]).into_owned()),
        _ => Err(missing("会话回复格式错误")),
    }
}

/// 客户端一次交换（CLI 与测试共用）：帧头 + 请求原字节 → 回复原字节或会话拒绝原因。
pub(super) fn exchange(
    mut stream: UnixStream,
    command: &str,
    binary_sha256: &str,
    db: (u64, u64),
    body: &[u8],
    deadline: Instant,
) -> Reply {
    let header = json!({"schema_revision":REQUEST_SCHEMA,"command":command,
        "binary_sha256":binary_sha256,"db_dev":db.0.to_string(),"db_ino":db.1.to_string()});
    write_frame(&mut stream, canonical_json(&header).as_bytes(), deadline)
        .map_err(|e| missing(format!("写会话请求：{e:?}")))?;
    write_frame(&mut stream, body, deadline).map_err(|e| missing(format!("写会话请求：{e:?}")))?;
    read_reply(&mut stream, deadline)
}

/// 客户端：`<financial-basis-ref|financial-content|financial-closure> --protocol-revision s-session/3 --db <db>
/// --session <socket>`。只转发 stdin 原字节、自身二进制 SHA 与 --db 的 (dev, ino)，把回复原字节
/// 写到 stdout；会话拒绝即以其原因失败；从不本地审计、从不回落冷路径。
pub(super) fn client(args: &[String]) -> Result<(), String> {
    let command = args.get(1).map(String::as_str).unwrap_or_default();
    if !matches!(
        command,
        "financial-basis-ref" | "financial-content" | "financial-closure"
    ) {
        return Err(invalid("会话客户端只接受三条财务命令"));
    }
    let flags = parse_flags(
        args,
        &["--protocol-revision", "--db", "--session"],
        "会话客户端",
    )?;
    let db = Path::new(&flags.0["--db"]);
    let identity = super::financial_reader::client_db_identity(db)?;
    let mut body = Vec::new();
    std::io::stdin().read_to_end(&mut body).map_err(invalid)?;
    let sha = self_sha256()?;
    let stream = connect(&flags.0["--session"])?;
    let deadline = Instant::now() + Duration::from_millis(CLIENT_WAIT_MS);
    let reply = exchange(stream, command, &sha, identity, &body, deadline)?;
    let mut out = std::io::stdout().lock();
    out.write_all(&reply).map_err(invalid)?;
    out.flush().map_err(invalid)
}

/// `financial-serve-stop --protocol-revision s-session/3 --session <socket>`：会话处理完在飞请求后
/// 优雅停机，落 FINAL.json 并把同一字节回给本命令；FINAL 非 PASS 时以失败退出。
pub(super) fn stop(args: &[String]) -> Result<(), String> {
    let flags = parse_flags(
        args,
        &["--protocol-revision", "--session"],
        "financial-serve-stop",
    )?;
    let sha = self_sha256()?;
    let mut stream = connect(&flags.0["--session"])?;
    let deadline = Instant::now() + Duration::from_millis(CLIENT_WAIT_MS);
    let header = json!({"schema_revision":REQUEST_SCHEMA,"command":"stop","binary_sha256":sha});
    write_frame(&mut stream, canonical_json(&header).as_bytes(), deadline)
        .map_err(|e| missing(format!("写停机请求：{e:?}")))?;
    let bytes = read_reply(&mut stream, deadline)?;
    let mut out = std::io::stdout().lock();
    out.write_all(&bytes).map_err(invalid)?;
    out.flush().map_err(invalid)?;
    let fin = strict_json(&bytes).map_err(invalid)?;
    if fin["status"] != "PASS" {
        return Err(missing("会话终态 FAIL"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "financial_session_tests.rs"]
mod tests;
