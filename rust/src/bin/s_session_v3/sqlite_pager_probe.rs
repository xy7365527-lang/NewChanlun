//! #1394 → #1640：`s_incremental` 生产增量构建只做只读 SQLite pager 诊断（ADR 0042 D8 诊断行）；
//! 私有缓存实验（读 `S_NATIVE_SQLITE_CACHE_KIB` 并设置 cache_size）只编入 `s_capture_native_testonly`。
//! 非审计证明；cache_size 是连接内建议页缓存量，不是 heap/RSS 峰值硬上限。

use rusqlite::{ffi, Connection};
use serde_json::{json, Map, Value};
use std::io::Write as _;

#[cfg(feature = "s_capture_native_testonly")]
const CACHE_ENV: &str = "S_NATIVE_SQLITE_CACHE_KIB";
const METRICS: [(&str, i32); 6] = [
    (
        "cache_used_bytes_current_approximate",
        ffi::SQLITE_DBSTATUS_CACHE_USED,
    ),
    ("cache_hit_cumulative_i32", ffi::SQLITE_DBSTATUS_CACHE_HIT),
    ("cache_miss_cumulative_i32", ffi::SQLITE_DBSTATUS_CACHE_MISS),
    (
        "cache_write_pages_cumulative_i32",
        ffi::SQLITE_DBSTATUS_CACHE_WRITE,
    ),
    (
        "cache_spill_pages_mid_transaction_cumulative_i32",
        ffi::SQLITE_DBSTATUS_CACHE_SPILL,
    ),
    (
        "stmt_used_bytes_current_approximate",
        ffi::SQLITE_DBSTATUS_STMT_USED,
    ),
];

pub(super) fn enabled() -> bool {
    std::env::var_os("S_MANIFEST_WORKSPACE_DIAGNOSTICS").as_deref()
        == Some(std::ffi::OsStr::new("1"))
}

/// 只有显式实验输入的校验、设置、回读失败会拒绝启动；纯诊断失败仅记 unavailable。
pub(super) fn configure_and_log(conn: &Connection) -> Result<(), String> {
    log(configure(conn)?);
    Ok(())
}

fn configure(conn: &Connection) -> Result<Value, String> {
    let requested = apply_requested_cache(conn)?;
    Ok(startup_line(conn, requested))
}

/// #1640 D-A（ADR 0042 D1）：生产增量构建不读缓存实验输入、不碰 cache_size，请求值恒为 None。
#[cfg(not(feature = "s_capture_native_testonly"))]
fn apply_requested_cache(_conn: &Connection) -> Result<Option<i32>, String> {
    Ok(None)
}

/// testonly 私有缓存实验：读输入、校验，合法时设置 cache_size 并回读。
#[cfg(feature = "s_capture_native_testonly")]
fn apply_requested_cache(conn: &Connection) -> Result<Option<i32>, String> {
    let requested = parse_cache_kib(std::env::var_os(CACHE_ENV).as_deref())?;
    if let Some(kib) = requested {
        set_cache_kib(conn, kib)?;
    }
    Ok(requested)
}

#[cfg(feature = "s_capture_native_testonly")]
fn parse_cache_kib(raw: Option<&std::ffi::OsStr>) -> Result<Option<i32>, String> {
    raw.map(|raw| {
        let raw = raw.to_str().ok_or_else(invalid_cache)?;
        if raw.is_empty() || raw.starts_with('0') || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid_cache());
        }
        let kib: i32 = raw.parse().map_err(|_| invalid_cache())?;
        if !(1..=262_144).contains(&kib) {
            return Err(invalid_cache());
        }
        Ok(kib)
    })
    .transpose()
}

#[cfg(feature = "s_capture_native_testonly")]
fn set_cache_kib(conn: &Connection, kib: i32) -> Result<(), String> {
    let target = -kib;
    conn.pragma_update(None, "cache_size", target)
        .map_err(|e| format!("WriterSqliteCacheInvalid：设置 cache_size 失败：{e}"))?;
    let actual: i32 = conn
        .pragma_query_value(None, "cache_size", |r| r.get(0))
        .map_err(|e| format!("WriterSqliteCacheInvalid：回读 cache_size 失败：{e}"))?;
    if actual != target {
        return Err(format!(
            "WriterSqliteCacheInvalid：cache_size 回读 {actual}，预期 {target}"
        ));
    }
    Ok(())
}

/// 启动诊断行（D8）：各构建字段集相同；生产增量构建 requested_cache_kib 恒为 null。
fn startup_line(conn: &Connection, requested: Option<i32>) -> Value {
    let mut pragmas = Map::new();
    for name in [
        "cache_size",
        "page_size",
        "journal_mode",
        "synchronous",
        "fullfsync",
        "wal_autocheckpoint",
        "cache_spill",
        "mmap_size",
    ] {
        let result = if name == "journal_mode" {
            conn.pragma_query_value(None, name, |r| r.get::<_, String>(0))
                .map(Value::from)
        } else {
            conn.pragma_query_value(None, name, |r| r.get::<_, i64>(0))
                .map(Value::from)
        };
        let value = match result {
            Ok(value) => json!({"availability":"available", "value":value}),
            Err(e) => json!({"availability":"unavailable", "rc":sqlite_rc(&e)}),
        };
        pragmas.insert(name.into(), value);
    }
    json!({"diagnostic":"WriterSqlitePager", "schema_revision":1,
    "stage":"startup-before-first-transaction", "requested_cache_kib":requested,
    "cache_size_semantics":"connection-session suggested pager cache; negative value is KiB; not heap/RSS peak bound",
    "pragmas":pragmas, "snapshot":snapshot(conn).value()})
}

#[cfg(feature = "s_capture_native_testonly")]
fn invalid_cache() -> String {
    format!("WriterSqliteCacheInvalid：{CACHE_ENV} 必须是规范十进制正整数 1..262144 KiB")
}

fn sqlite_rc(error: &rusqlite::Error) -> Option<i32> {
    match error {
        rusqlite::Error::SqliteFailure(e, _) => Some(e.extended_code),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Reading {
    rc: i32,
    current: i32,
    highwater: i32,
}

/// 只保存整数，不保留 Connection 引用、SQLite 指针或语句。
pub(super) struct Snapshot([Reading; 6], ProcessMaxRss);

pub(super) fn snapshot(conn: &Connection) -> Snapshot {
    Snapshot(
        METRICS.map(|(_, verb)| read_status(conn, verb)),
        process_maxrss(),
    )
}

fn read_status(conn: &Connection, verb: i32) -> Reading {
    let mut current = 0;
    let mut highwater = 0;
    // SAFETY：conn 借用期间 handle 有效；当前线程同步调用，不保存指针。
    // 输出为栈内有效 i32；resetFlag=0，不复位本连接的共享累计计数。
    let rc =
        unsafe { ffi::sqlite3_db_status(conn.handle(), verb, &mut current, &mut highwater, 0) };
    Reading {
        rc,
        current,
        highwater,
    }
}

struct ProcessMaxRss {
    availability: &'static str,
    rc: Option<i32>,
    raw: Option<i64>,
    raw_unit: &'static str,
    bytes: Option<u64>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn process_maxrss() -> ProcessMaxRss {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY：getrusage 同步写入有效的栈内 rusage；不保存指针。
    // 仅 rc=0 后读取已初始化结果；RUSAGE_SELF 为只读进程资源统计。
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    #[cfg(target_os = "macos")]
    let (raw_unit, multiplier) = ("bytes", 1u64);
    #[cfg(target_os = "linux")]
    let (raw_unit, multiplier) = ("KiB", 1024u64);
    let raw = if rc == 0 {
        // SAFETY：上面的 getrusage 成功并初始化 usage。
        Some(unsafe { usage.assume_init() }.ru_maxrss as i64)
    } else {
        None
    };
    let bytes = raw.and_then(|n| u64::try_from(n).ok()?.checked_mul(multiplier));
    ProcessMaxRss {
        availability: if bytes.is_some() {
            "available"
        } else {
            "unavailable"
        },
        rc: Some(rc),
        raw,
        raw_unit,
        bytes,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn process_maxrss() -> ProcessMaxRss {
    ProcessMaxRss {
        availability: "unsupported",
        rc: None,
        raw: None,
        raw_unit: "unsupported",
        bytes: None,
    }
}

impl Snapshot {
    fn value(&self) -> Value {
        let mut values = Map::new();
        for ((name, _), reading) in METRICS.iter().zip(self.0.iter()) {
            values.insert((*name).into(), json!({
                "availability":if reading.rc == ffi::SQLITE_OK { "available" } else { "unavailable" },
                "rc":reading.rc,
                "current_i32":(reading.rc == ffi::SQLITE_OK).then_some(reading.current),
                "highwater_i32":(reading.rc == ffi::SQLITE_OK).then_some(reading.highwater),
                "highwater_semantics":"unsupported; SQLite returns zero, not a peak"
            }));
        }
        values.insert("process_maxrss".into(), json!({
            "availability":self.1.availability, "rc":self.1.rc,
            "raw_maxrss":self.1.raw, "raw_unit":self.1.raw_unit,
            "bytes":self.1.bytes,
            "semantics":"kernel reported process lifetime RSS high-water as of this sample; not Rust heap or this request's independent peak"
        }));
        Value::Object(values)
    }
}

pub(super) fn log_handle(before: Snapshot, after: Snapshot, message_id: Option<Value>, ok: bool) {
    let mut delta = Map::new();
    for index in 1..=4 {
        let (name, _) = METRICS[index];
        let b = before.0[index];
        let a = after.0[index];
        let (value, reason) = if b.rc != ffi::SQLITE_OK || a.rc != ffi::SQLITE_OK {
            (None, "unavailable")
        } else if b.current < 0 || a.current < 0 || a.current < b.current {
            (None, "overflow_or_reset_uncertain")
        } else {
            (Some(a.current - b.current), "nonnegative_monotonic_i32_endpoints; wrap/reset between samples cannot be ruled out")
        };
        delta.insert(name.into(), json!({"value":value, "interpretation":reason}));
    }
    log(
        json!({"diagnostic":"WriterSqlitePager", "schema_revision":1,
        "stage":"handle-before-after", "message_id":message_id, "business_result_ok":ok,
        "before":before.value(), "after":after.value(), "cumulative_delta":delta,
        "cache_write_semantics":"dirty pages written; undefined after SQLite IO/write error",
        "reset_flag":0, "pager_peak_measurement":false}),
    );
}

fn log(line: Value) {
    // 已提交的业务结果不能被诊断输出错误覆写。
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    const STARTUP_FIELDS: [&str; 7] = [
        "cache_size_semantics",
        "diagnostic",
        "pragmas",
        "requested_cache_kib",
        "schema_revision",
        "snapshot",
        "stage",
    ];
    const PRAGMA_FIELDS: [&str; 8] = [
        "cache_size",
        "cache_spill",
        "fullfsync",
        "journal_mode",
        "mmap_size",
        "page_size",
        "synchronous",
        "wal_autocheckpoint",
    ];

    fn keys(value: &Value) -> Vec<&str> {
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        keys
    }

    fn cache_size(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "cache_size", |r| r.get(0))
            .unwrap()
    }

    /// #1640 D-A / ADR 0042 D8：启动诊断行字段集在所有含本模块的构建不变，请求值照出。
    #[test]
    fn pager_probe_startup_line_keeps_field_set() {
        let conn = Connection::open_in_memory().unwrap();
        for requested in [None, Some(4096)] {
            let line = startup_line(&conn, requested);
            assert_eq!(keys(&line), STARTUP_FIELDS);
            assert_eq!(keys(&line["pragmas"]), PRAGMA_FIELDS);
            assert_eq!(line["diagnostic"], json!("WriterSqlitePager"));
            assert_eq!(line["stage"], json!("startup-before-first-transaction"));
            assert_eq!(line["requested_cache_kib"], json!(requested));
        }
    }

    /// #1640 D-A：生产增量构建不读缓存实验输入、不改 cache_size，诊断行照出且请求值为 null。
    #[cfg(not(feature = "s_capture_native_testonly"))]
    #[test]
    fn pager_probe_production_ignores_cache_request() {
        let conn = Connection::open_in_memory().unwrap();
        let default = cache_size(&conn);
        let line = configure(&conn).unwrap();
        assert_eq!(keys(&line), STARTUP_FIELDS);
        assert_eq!(line["requested_cache_kib"], Value::Null);
        assert_eq!(
            line["pragmas"]["cache_size"],
            json!({"availability":"available", "value":default})
        );
        assert_eq!(cache_size(&conn), default);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn pager_probe_testonly_cache_kib_parse() {
        use std::ffi::OsStr;
        assert_eq!(parse_cache_kib(None), Ok(None));
        for (raw, kib) in [("1", 1), ("4096", 4096), ("262144", 262_144)] {
            assert_eq!(parse_cache_kib(Some(OsStr::new(raw))), Ok(Some(kib)));
        }
        for raw in [
            "",
            "0",
            "01",
            "0262144",
            "262145",
            "2147483648",
            "abc",
            "12a",
            "-1",
            "+1",
            " 1",
        ] {
            assert_eq!(
                parse_cache_kib(Some(OsStr::new(raw))),
                Err(invalid_cache()),
                "{raw:?}"
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            assert_eq!(
                parse_cache_kib(Some(OsStr::from_bytes(b"1\xff"))),
                Err(invalid_cache())
            );
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn pager_probe_testonly_cache_request_sets_cache_size() {
        let conn = Connection::open_in_memory().unwrap();
        set_cache_kib(&conn, 262_144).unwrap();
        assert_eq!(cache_size(&conn), -262_144);
        let line = startup_line(&conn, Some(262_144));
        assert_eq!(keys(&line), STARTUP_FIELDS);
        assert_eq!(line["requested_cache_kib"], json!(262_144));
        assert_eq!(line["pragmas"]["cache_size"]["value"], json!(-262_144));
    }
}
