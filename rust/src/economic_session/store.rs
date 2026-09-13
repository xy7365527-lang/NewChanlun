use super::book::{apply, decode, empty_image, fact_key, hash, target};
use crate::session_protocol::{
    canonical_json, nonnegative, positive, required, sha256_hex, strict_json,
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "macos"))]
use std::process::Command;
use std::time::Duration;

pub(crate) fn err(e: impl std::fmt::Display) -> String {
    format!("StorageUnavailable：{e}")
}
pub(crate) const SCHEMA: &str = "economic-session/1";
const TABLES: [&str; 10] = [
    "meta",
    "epochs",
    "commits",
    "facts",
    "pending",
    "applications",
    "unmatched",
    "book",
    "messages",
    "receipts",
];
const SQL: &str = r#"
CREATE TABLE meta(k TEXT PRIMARY KEY,v TEXT NOT NULL) STRICT;
CREATE TABLE epochs(epoch INTEGER PRIMARY KEY,phase TEXT NOT NULL,at_ns TEXT NOT NULL) STRICT;
CREATE TABLE commits(seq INTEGER PRIMARY KEY,prev_root TEXT NOT NULL,root TEXT NOT NULL UNIQUE,commit_ns TEXT NOT NULL,image TEXT NOT NULL) STRICT;
CREATE TABLE facts(k TEXT PRIMARY KEY,json TEXT NOT NULL) STRICT;
CREATE TABLE pending(k TEXT PRIMARY KEY,json TEXT NOT NULL) STRICT;
CREATE TABLE applications(k TEXT PRIMARY KEY,fact_key TEXT NOT NULL UNIQUE,json TEXT NOT NULL) STRICT;
CREATE TABLE unmatched(k TEXT PRIMARY KEY,json TEXT NOT NULL) STRICT;
CREATE TABLE book(k TEXT PRIMARY KEY CHECK(k='book'),json TEXT NOT NULL) STRICT;
CREATE TABLE messages(k TEXT PRIMARY KEY,payload_hash TEXT NOT NULL,receipt_key TEXT NOT NULL,request_json TEXT NOT NULL) STRICT;
CREATE TABLE receipts(k TEXT PRIMARY KEY,json TEXT NOT NULL) STRICT;
"#;

#[derive(Clone)]
pub(crate) struct Config {
    pub manifest: Value,
    pub manifest_hash: String,
    pub db: PathBuf,
    pub domain: String,
    pub session: String,
    pub generation: String,
    pub producer: String,
    pub socket: PathBuf,
}
impl Config {
    pub fn load(path: &Path, db: &Path, chong: Option<&str>) -> Result<Self, String> {
        let manifest = strict_json(&std::fs::read(path).map_err(err)?)?;
        if manifest["schema_revision"] != "economic-manifest/1" {
            return Err("SchemaUnsupported：manifest版本".into());
        }
        super::evidence::validate_config(&manifest)?;
        let session = required(&manifest, "session_id")?.to_owned();
        let generation = required(&manifest, "session_generation")?.to_owned();
        positive(&generation, "session_generation")?;
        let es = &manifest["e"];
        let epath = absolute(required(es, "db")?)?;
        absolute(required(es, "socket")?)?;
        let mut ids = std::collections::BTreeSet::new();
        let mut semantics = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::from([epath]);
        let mut sockets = std::collections::BTreeSet::from([absolute(required(es, "socket")?)?]);
        let routes = manifest["chongs"]
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("InvalidDomain：重路由为空")?;
        for c in routes {
            let id = required(c, "chong_id")?;
            let symbol = required(c, "symbol")?;
            let level = required(c, "operation_level")?;
            nonnegative(level, "operation_level")?;
            positive(required(c, "fixed_q")?, "fixed_q")?;
            if !ids.insert(id)
                || !semantics.insert((symbol, level))
                || !paths.insert(absolute(required(c, "db")?)?)
                || !sockets.insert(absolute(required(c, "socket")?)?)
            {
                return Err("IdentityConflict：重身份/语义/数据库/socket重复".into());
            }
        }
        let auth = manifest["authorized_sources"]
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("InvalidDomain：未配置获准来源")?;
        let mut principals = std::collections::BTreeSet::new();
        for a in auth {
            for k in [
                "source_namespace",
                "source_epoch",
                "producer_id",
                "producer_epoch",
            ] {
                required(a, k)?;
            }
            positive(required(a, "producer_epoch")?, "producer_epoch")?;
            let ops = a["operations"]
                .as_array()
                .filter(|v| !v.is_empty())
                .ok_or("InvalidDomain：来源缺显式操作权限")?;
            let mut unique = std::collections::BTreeSet::new();
            for op in ops {
                let op = op.as_str().ok_or("InvalidDomain：操作权限须文本")?;
                if ![
                    "CaptureExternal",
                    "ApplyAllocation",
                    "ReadView",
                    "Watch",
                    "QueryFact",
                    "QueryAllocation",
                    "BindVoice",
                    "QueryVoice",
                ]
                .contains(&op)
                    || !unique.insert(op)
                {
                    return Err("InvalidDomain：操作权限未知或重复".into());
                }
            }

            if !principals.insert(hash(a)) {
                return Err("IdentityConflict：重复来源配置".into());
            }
        }
        for k in [
            "max_frame_bytes",
            "read_timeout_ms",
            "write_timeout_ms",
            "response_timeout_ms",
            "queue_capacity",
            "max_connections",
        ] {
            positive(required(&manifest["resources"], k)?, k)?;
        }
        match required(&manifest["clock"], "kind")? {
            "System" => (),
            "TestOnly" => {
                nonnegative(required(&manifest["clock"]["plan"], "init_ns")?, "init_ns")?;
            }
            _ => return Err("InvalidDomain：必须显式指定时钟adapter".into()),
        }
        if let Some(f) = manifest.get("test_only_fault").filter(|v| !v.is_null()) {
            if manifest["clock"]["kind"] != "TestOnly" {
                return Err("InvalidDomain：故障暂停仅TestOnly".into());
            }
            required(f, "message_id")?;
            required(f, "marker_path")?;
            match required(f, "phase")? {
                "before_commit" | "after_commit" => (),
                _ => return Err("InvalidDomain：未支持故障点".into()),
            }
        }
        let (domain, route) = if let Some(id) = chong {
            (
                format!("B:{id}"),
                routes
                    .iter()
                    .find(|c| c["chong_id"] == id)
                    .ok_or("InvalidDomain：未绑定目标重")?,
            )
        } else {
            ("E".to_owned(), es)
        };
        let expected = absolute(required(route, "db")?)?;
        if canonical_target(db)? != canonical_target(&expected)? {
            return Err("IdentityConflict：命令数据库不是manifest域路径".into());
        }
        let socket = absolute(required(route, "socket")?)?;
        Ok(Self {
            manifest_hash: hash(&manifest),
            manifest,
            db: expected,
            producer: format!("{domain}:{session}"),
            domain,
            session,
            generation,
            socket,
        })
    }
    pub fn route(&self, id: &str) -> Result<&Value, String> {
        self.manifest["chongs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["chong_id"] == id)
            .ok_or_else(|| "InvalidDomain：事实目标未绑定".into())
    }
    pub fn clock(
        &self,
        msg: Option<&str>,
        phase: &str,
        recovery_index: usize,
    ) -> Result<String, String> {
        if self.manifest["clock"]["kind"] == "System" {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(err)?
                .as_nanos();
            if n > i64::MAX as u128 {
                return Err("InvalidDomain：系统时间超精确域".into());
            }
            return Ok(n.to_string());
        }
        let p = &self.manifest["clock"]["plan"];
        let v = if let Some(m) = msg {
            {
                let v = &p["messages"][m][phase];
                if v.is_array() {
                    &v[recovery_index]
                } else {
                    v
                }
            }
        } else if phase == "recover_ns" {
            &p[phase][recovery_index]
        } else {
            &p[phase]
        };
        let s = v.as_str().ok_or_else(|| {
            format!("MissingDependency：TestOnly时钟缺{phase}/{msg:?}/{recovery_index}")
        })?;
        nonnegative(s, phase)?;
        Ok(s.to_owned())
    }
    pub fn authorized(&self, e: &Value) -> Result<(), String> {
        if e["session_id"] != self.session || e["session_generation"] != self.generation {
            return Err("IdentityConflict：跨session/化身消息".into());
        }
        if !self.manifest["authorized_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| {
                [
                    "source_namespace",
                    "source_epoch",
                    "producer_id",
                    "producer_epoch",
                ]
                .iter()
                .all(|k| a[k] == e[k])
            })
        {
            return Err("InvalidDomain：来源未获准：未绑定来源/producer epoch".into());
        }
        let op = required(&e["payload"], "op")?;
        if !self.manifest["authorized_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| {
                [
                    "source_namespace",
                    "source_epoch",
                    "producer_id",
                    "producer_epoch",
                ]
                .iter()
                .all(|k| a[k] == e[k])
                    && a["operations"].as_array().unwrap().iter().any(|v| v == op)
            })
        {
            return Err("InvalidDomain：来源未获准执行该操作".into());
        }
        Ok(())
    }
    pub fn pause(&self, msg: &str, phase: &str) -> Result<(), String> {
        let f = &self.manifest["test_only_fault"];
        if f.is_object() && f["message_id"] == msg && f["phase"] == phase {
            let p = absolute(required(f, "marker_path")?)?;
            let text = canonical_json(
                &json!({"domain_id":self.domain,"message_id":msg,"phase":phase,"pid":std::process::id(),"test_only":true}),
            );
            let mut file = match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&p)
            {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let existing = strict_json(&std::fs::read(&p).map_err(err)?)?;
                    if existing["domain_id"] != self.domain
                        || existing["message_id"] != msg
                        || existing["phase"] != phase
                        || existing["test_only"] != true
                    {
                        return Err("IdentityConflict：TestOnly故障marker不是本次计划".into());
                    }
                    return Ok(());
                }
                Err(e) => return Err(err(e)),
            };
            use std::io::Write;
            file.write_all(text.as_bytes()).map_err(err)?;
            file.sync_all().map_err(err)?;
            // #1374：真实进程故障探针，由控制者 SIGKILL；默认路径从不暂停。
            loop {
                std::thread::park_timeout(Duration::from_millis(50));
            }
        }
        Ok(())
    }
}
fn absolute(s: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(s);
    if !p.is_absolute() {
        return Err("InvalidDomain：路径必须绝对".into());
    }
    Ok(p)
}
fn canonical_target(p: &Path) -> Result<PathBuf, String> {
    if p.exists() {
        p.canonicalize().map_err(err)
    } else {
        Ok(p.parent()
            .ok_or("InvalidDomain：路径无父目录")?
            .canonicalize()
            .map_err(err)?
            .join(p.file_name().ok_or("InvalidDomain：路径无文件名")?))
    }
}
fn filesystem(path: &Path) -> Result<String, String> {
    let target = if path.exists() {
        path.canonicalize().map_err(err)?
    } else {
        path.parent()
            .ok_or("InvalidDomain：数据库无父目录")?
            .canonicalize()
            .map_err(err)?
    };
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let cpath = std::ffi::CString::new(target.as_os_str().as_bytes()).map_err(err)?;
        let mut status = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // 成功的statfs初始化整份系统结构；CString和输出空间在调用期间均有效。
        if unsafe { libc::statfs(cpath.as_ptr(), status.as_mut_ptr()) } != 0 {
            return Err(err(std::io::Error::last_os_error()));
        }
        let status = unsafe { status.assume_init() };
        let field = |bytes: &[libc::c_char]| -> Result<String, String> {
            String::from_utf8(
                bytes
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as u8)
                    .collect(),
            )
            .map_err(err)
        };
        let kind = field(&status.f_fstypename)?;
        let local = status.f_flags & libc::MNT_LOCAL as u32 != 0;
        let read_only = status.f_flags & libc::MNT_RDONLY as u32 != 0;
        let detail = format!(
            "{kind}, local={local}, read_only={read_only}, mount={}, source={}",
            field(&status.f_mntonname)?,
            field(&status.f_mntfromname)?
        );
        if !matches!(kind.as_str(), "apfs" | "hfs") || !local {
            return Err(format!("StorageUnavailable：未证明本地apfs/hfs：{detail}"));
        }
        Ok(detail)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let o = Command::new("stat")
            .args(["-f", "-c", "%T"])
            .arg(target)
            .output()
            .map_err(err)?;
        let k = String::from_utf8(o.stdout).map_err(err)?.trim().to_owned();
        if !o.status.success()
            || !["ext2/ext3", "ext4", "xfs", "btrfs", "tmpfs", "overlayfs"].contains(&k.as_str())
        {
            return Err(format!("StorageUnavailable：未证明本地文件系统：{k}"));
        }
        Ok(k)
    }
}
pub(crate) fn open(db: &Path, read_only: bool) -> Result<(Connection, Value), String> {
    let fs = filesystem(db)?;
    let flags = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    };
    let c = Connection::open_with_flags(db, flags).map_err(err)?;
    c.busy_timeout(Duration::from_secs(2)).map_err(err)?;
    let version = rusqlite::version_number();
    if version < 3_051_003 {
        return Err("StorageUnavailable：运行SQLite缺WAL-reset修复".into());
    }
    if !read_only {
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA fullfsync=ON; PRAGMA foreign_keys=ON;").map_err(err)?;
    } else {
        c.execute_batch("PRAGMA query_only=ON; PRAGMA synchronous=FULL; PRAGMA fullfsync=ON;")
            .map_err(err)?;
    }
    let mode: String = c
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(err)?;
    let sync: i64 = c
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(err)?;
    let full: i64 = c
        .query_row("PRAGMA fullfsync", [], |r| r.get(0))
        .map_err(err)?;
    if mode != "wal" || sync != 2 || cfg!(target_os = "macos") && full != 1 {
        return Err("StorageUnavailable：WAL/FULL/fullfsync前件失败".into());
    }
    Ok((
        c,
        json!({"sqlite_version":rusqlite::version(),"sqlite_version_number":version.to_string(),"journal_mode":mode,"synchronous":sync.to_string(),"fullfsync":full.to_string(),"filesystem":fs,"read_only":read_only,"pid":std::process::id().to_string(),"control_instance_id":std::env::var("SESSION_CONTROL_INSTANCE_ID").ok()}),
    ))
}
pub(crate) fn meta(c: &Connection, k: &str) -> Result<String, String> {
    c.query_row("SELECT v FROM meta WHERE k=?", [k], |r| r.get(0))
        .map_err(err)
}
pub(crate) fn set_meta(c: &Connection, k: &str, v: &str) -> Result<(), String> {
    c.execute(
        "INSERT INTO meta(k,v)VALUES(?,?)ON CONFLICT(k)DO UPDATE SET v=excluded.v",
        params![k, v],
    )
    .map_err(err)?;
    Ok(())
}
pub(crate) fn row(c: &Connection, table: &str, k: &str) -> Result<Option<Value>, String> {
    if !TABLES.contains(&table) {
        return Err("InvalidDomain：未知表".into());
    }
    let sql = format!("SELECT json FROM {table} WHERE k=?");
    let s: Option<String> = c
        .query_row(&sql, [k], |r| r.get(0))
        .optional()
        .map_err(err)?;
    s.map(|s| strict_json(s.as_bytes())).transpose()
}
pub(crate) fn rows(c: &Connection, table: &str) -> Result<Vec<Value>, String> {
    if !TABLES.contains(&table) {
        return Err("InvalidDomain：未知表".into());
    }
    let mut s = c
        .prepare(&format!("SELECT json FROM {table} ORDER BY k"))
        .map_err(err)?;
    let r = s.query_map([], |r| r.get::<_, String>(0)).map_err(err)?;
    r.map(|r| strict_json(r.map_err(err)?.as_bytes())).collect()
}
pub(crate) fn put(c: &Connection, table: &str, k: &str, v: &Value) -> Result<(), String> {
    if !TABLES.contains(&table) {
        return Err("InvalidDomain：未知表".into());
    }
    c.execute(
        &format!("INSERT INTO {table}(k,json)VALUES(?,?)"),
        params![k, canonical_json(v)],
    )
    .map_err(err)?;
    Ok(())
}
pub(crate) fn check_epoch(c: &Connection, expected: i64) -> Result<(), String> {
    if positive(&meta(c, "writer_epoch")?, "writer_epoch")? != expected {
        return Err("StaleWriter：持久epoch不等于获准写者".into());
    }
    Ok(())
}
pub(crate) fn cut(config: &Config, seq: i64, root: &str) -> Value {
    json!({"domain_id":config.domain,"commit_seq":seq.to_string(),"root_hash":root})
}
fn root(config: &Config, seq: i64, prev: &str, ns: &str, image: &Value) -> String {
    hash(
        &json!({"domain_id":config.domain,"session_id":config.session,"session_generation":config.generation,"commit_seq":seq.to_string(),"previous_root":prev,"commit_ns":ns,"image_hash":hash(image)}),
    )
}
pub(crate) fn latest(c: &Connection) -> Result<(i64, String, String, Value), String> {
    let (seq, root, ns, image): (i64, String, String, String) = c
        .query_row(
            "SELECT seq,root,commit_ns,image FROM commits ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(err)?;
    Ok((seq, root, ns, strict_json(image.as_bytes())?))
}
pub(crate) fn append(
    c: &Connection,
    config: &Config,
    seq: i64,
    prev: &str,
    ns: &str,
    image: &Value,
) -> Result<Value, String> {
    let h = root(config, seq, prev, ns, image);
    c.execute(
        "INSERT INTO commits(seq,prev_root,root,commit_ns,image)VALUES(?,?,?,?,?)",
        params![seq, prev, h, ns, canonical_json(image)],
    )
    .map_err(err)?;
    Ok(cut(config, seq, &h))
}
pub(crate) fn init(config: &Config) -> Result<Value, String> {
    if config.db.exists() {
        return Err("IdentityConflict：init拒绝覆盖已有库".into());
    }
    let (mut c, runtime) = open(&config.db, false)?;
    let tx = c.transaction().map_err(err)?;
    tx.execute_batch(SQL).map_err(err)?;
    for (k, v) in [
        ("schema_revision", SCHEMA),
        ("domain_id", config.domain.as_str()),
        ("session_id", config.session.as_str()),
        ("session_generation", config.generation.as_str()),
        ("manifest_hash", config.manifest_hash.as_str()),
        ("writer_epoch", "1"),
    ] {
        set_meta(&tx, k, v)?;
    }
    if super::evidence::enabled(config) {
        set_meta(&tx, "voice_binding_revision", "voice-binding/1")?;
    }
    let ns = config.clock(None, "init_ns", 0)?;
    tx.execute("INSERT INTO epochs VALUES(1,'init',?)", [&ns])
        .map_err(err)?;
    let cut = append(&tx, config, 0, "", &ns, &empty_image())?;
    tx.commit().map_err(err)?;
    Ok(
        json!({"kind":"Initialized","domain_id":config.domain,"writer_epoch":"1","cut":cut,"runtime":runtime}),
    )
}
fn sorted(mut a: Vec<Value>) -> Vec<String> {
    a.sort_by_key(canonical_json);
    a.iter().map(canonical_json).collect()
}
/// 全域审计：表集合、meta、全历史根、来源原件、每步重放、当前物化表与永久回执关系。
pub(crate) fn audit(c: &Connection, config: &Config) -> Result<(), String> {
    let expected = Connection::open_in_memory().map_err(err)?;
    expected.execute_batch(SQL).map_err(err)?;
    let schema_dump = |db: &Connection| -> Result<Vec<(String, String, String)>, String> {
        let mut q = db
            .prepare(
                "SELECT type,name,sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type,name",
            )
            .map_err(err)?;
        let r = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(err)?;
        r.collect::<Result<_, _>>().map_err(err)
    };
    if schema_dump(c)? != schema_dump(&expected)? {
        return Err("StorageUnavailable：域损坏：完整schema/索引发生变化".into());
    }
    let integrity: String = c
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(err)?;
    if integrity != "ok" {
        return Err(format!("StorageUnavailable：域损坏：SQLite {integrity}"));
    }
    let names: Vec<String> = {
        let mut q=c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(err)?;
        let r = q.query_map([], |r| r.get(0)).map_err(err)?;
        r.collect::<Result<_, _>>().map_err(err)?
    };
    let mut want = TABLES.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    want.sort();
    if names != want {
        return Err("StorageUnavailable：域损坏：表集合变化".into());
    }
    let extra: i64 = c
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('trigger','view')",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    if extra != 0 {
        return Err("StorageUnavailable：域损坏：非预期触发器/视图".into());
    }
    for (k, v) in [
        ("schema_revision", SCHEMA),
        ("domain_id", config.domain.as_str()),
        ("session_id", config.session.as_str()),
        ("session_generation", config.generation.as_str()),
        ("manifest_hash", config.manifest_hash.as_str()),
    ] {
        if meta(c, k)? != v {
            return Err(format!("IdentityConflict：持久{k}未绑定当前配置"));
        }
    }
    let voice_revision: Option<String> = c
        .query_row(
            "SELECT v FROM meta WHERE k='voice_binding_revision'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(err)?;
    if voice_revision.as_deref()
        != if super::evidence::enabled(config) {
            Some("voice-binding/1")
        } else {
            None
        }
    {
        return Err(
            "IdentityConflict：Voice 存储版本未由 init 显式绑定；不得在 recover 暗迁移".into(),
        );
    }
    let epoch = positive(&meta(c, "writer_epoch")?, "writer_epoch")?;
    let (count, max): (i64, i64) = c
        .query_row("SELECT count(*),max(epoch) FROM epochs", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(err)?;
    if count != epoch || max != epoch {
        return Err("StorageUnavailable：域损坏：epoch历史不连续".into());
    }
    let mut q = c
        .prepare("SELECT seq,prev_root,root,commit_ns,image FROM commits ORDER BY seq")
        .map_err(err)?;
    let rs = q
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(err)?;
    let mut previous = String::new();
    let mut previous_image = empty_image();
    let mut last_ns = -1;
    let mut seen = 0i64;
    for r in rs {
        let (seq, prev, h, ns, raw) = r.map_err(err)?;
        let image = strict_json(raw.as_bytes())?;
        let n = nonnegative(&ns, "commit_ns")?;
        if seq != seen
            || prev != previous
            || h != root(config, seq, &prev, &ns, &image)
            || n < last_ns
        {
            return Err("StorageUnavailable：域损坏：提交链/时间/根不一致".into());
        }
        if seq == 0 {
            if image != empty_image() {
                return Err("StorageUnavailable：域损坏：创世图像非空".into());
            }
        } else if image["history"]
            .as_array()
            .and_then(|a| a.last())
            .map(|e| e["kind"] == "BindVoice")
            .unwrap_or(false)
        {
            let event = image["history"].as_array().unwrap().last().unwrap();
            let expected = super::voice::replay(c, config, &previous_image, event, seq, &ns)?;
            if image != expected {
                return Err("StorageUnavailable：Voice 重放不等于完整提交图像".into());
            }
        } else {
            let records = image["recorded"]
                .as_array()
                .ok_or("StorageUnavailable：域损坏：recorded形状")?;
            let record = records
                .last()
                .ok_or("StorageUnavailable：域损坏：提交缺原件")?;
            verify_record(record)?;
            if config.domain == "E" {
                if record["recorded_seq"] != seq.to_string() || record["commit_ns"] != ns {
                    return Err(
                        "StorageUnavailable：域损坏：原事实序号/时间不属于该实际提交".into(),
                    );
                }
            } else {
                if record["b_observation"]["commit_ns"] != ns {
                    return Err("StorageUnavailable：域损坏：B获知时间不属于该应用提交".into());
                }
                verify_e_copy(record, config)?;
            }
            verify_original_message(c, record, config)?;
            let expected = if config.domain == "E" {
                let mut x = previous_image.clone();
                x["recorded"].as_array_mut().unwrap().push(record.clone());
                let p = json!({"fact_key":record["fact_key"],"token":record["token"],"target_domain":record["target_domain"],"status":"Pending","reason":"No Resolve/Reduce evidence in TB-03-A","first_known_ns":record["first_known_ns"],"recorded_seq":seq.to_string()});
                x["pending"].as_array_mut().unwrap().push(p);
                x["history"].as_array_mut().unwrap().push(json!({"kind":"CaptureExternal","fact_key":record["fact_key"],"commit_seq":seq.to_string(),"commit_ns":ns}));
                x
            } else {
                apply(&previous_image, record, seq, &ns)?.0
            };
            if image != expected {
                return Err("StorageUnavailable：域损坏：原件重放不等于提交图像".into());
            }
        }
        previous = h;
        previous_image = image;
        last_ns = n;
        seen += 1;
    }
    if seen == 0 {
        return Err("StorageUnavailable：域损坏：无创世提交".into());
    }
    for (table, key) in [
        ("facts", "recorded"),
        ("pending", "pending"),
        ("applications", "applied"),
        ("unmatched", "unmatched"),
    ] {
        if sorted(rows(c, table)?)
            != sorted(
                previous_image[key]
                    .as_array()
                    .ok_or("StorageUnavailable：域损坏：图像数组缺失")?
                    .clone(),
            )
        {
            return Err(format!("StorageUnavailable：域损坏：{table}不等于完整图像"));
        }
    }
    if row(c, "book", "book")?.unwrap_or(Value::Null) != previous_image["book"] {
        return Err("StorageUnavailable：域损坏：book物化不等于完整图像".into());
    }
    let receipt_rows = rows(c, "receipts")?;
    if receipt_rows.len() != (seen - 1) as usize {
        return Err("StorageUnavailable：域损坏：永久回执数不等于业务提交数".into());
    }
    for r in receipt_rows {
        if r["kind"] == "VoiceBound" {
            if config.domain == "E" {
                return Err("StorageUnavailable：E 非法持有 Voice 回执".into());
            }
            let record = super::voice::find_binding(&previous_image, required(&r, "binding_id")?)?
                .ok_or("StorageUnavailable：Voice 回执缺不可变对象")?;
            let seq = positive(required(&record, "commit_seq")?, "Voice actual commit seq")?;
            let (actual_root, ns, raw): (String, String, String) = c
                .query_row(
                    "SELECT root,commit_ns,image FROM commits WHERE seq=?",
                    [seq],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .map_err(err)?;
            let image = strict_json(raw.as_bytes())?;
            let event = image["history"]
                .as_array()
                .and_then(|a| a.last())
                .ok_or("StorageUnavailable：Voice 实际提交缺事件")?;
            if event["kind"] != "BindVoice"
                || event["record"] != record
                || record["commit_ns"] != ns
                || r != super::voice::receipt(config, &record, cut(config, seq, &actual_root))
            {
                return Err("StorageUnavailable：Voice 原回执不等于实际提交".into());
            }
            continue;
        }
        let f = row(c, "facts", required(&r, "fact_key")?)?
            .ok_or("StorageUnavailable：域损坏：回执缺原件")?;
        let a = if config.domain == "E" {
            None
        } else {
            Some(
                row(c, "applications", required(&f["event"], "allocation_id")?)?
                    .ok_or("StorageUnavailable：域损坏：原件缺应用")?,
            )
        };
        // #1374 owner-review：序号从事实/应用取得，不能信回执自选一个存在的历史cut。
        let seq = positive(
            required(
                a.as_ref().unwrap_or(&f),
                if a.is_some() {
                    "commit_seq"
                } else {
                    "recorded_seq"
                },
            )?,
            "actual_commit_seq",
        )?;
        let (actual_root, ns, raw): (String, String, String) = c
            .query_row(
                "SELECT root,commit_ns,image FROM commits WHERE seq=?",
                [seq],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(err)?;
        let image = strict_json(raw.as_bytes())?;
        if image["recorded"].as_array().and_then(|a| a.last()) != Some(&f) {
            return Err("StorageUnavailable：域损坏：回执指向的实际提交不含对应原事实".into());
        }
        let expected = if let Some(a) = a {
            if image["applied"].as_array().and_then(|v| v.last()) != Some(&a)
                || a["commit_ns"] != ns
                || f["b_observation"]["commit_ns"] != ns
            {
                return Err("StorageUnavailable：域损坏：B回执/应用/提交时间不一致".into());
            }
            json!({"kind":a["status"],"domain_id":config.domain,"fact_key":f["fact_key"],"fact_hash":f["fact_hash"],"allocation_id":f["event"]["allocation_id"],"target_domain":config.domain,"received_ns":f["b_observation"]["received_ns"],"first_known_ns":f["b_observation"]["first_known_ns"],"commit_ns":ns,"e_first_known_ns":f["first_known_ns"],"cut":cut(config,seq,&actual_root)})
        } else {
            if f["commit_ns"] != ns {
                return Err("StorageUnavailable：域损坏：E回执/原件/提交时间不一致".into());
            }
            e_receipt(&f, cut(config, seq, &actual_root))
        };
        if r != expected {
            return Err("StorageUnavailable：域损坏：永久回执完整内容与实际提交不同".into());
        }
    }
    for (table, field) in [
        ("facts", "fact_key"),
        ("pending", "fact_key"),
        ("applications", "allocation_id"),
        ("unmatched", "allocation_id"),
        (
            "receipts",
            if config.domain == "E" {
                "fact_key"
            } else {
                "allocation_id"
            },
        ),
    ] {
        let mut q = c
            .prepare(&format!("SELECT k,json FROM {table}"))
            .map_err(err)?;
        let rs = q
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(err)?;
        for r in rs {
            let (k, s) = r.map_err(err)?;
            let v = strict_json(s.as_bytes())?;
            let expected_key = if table == "receipts" && v["kind"] == "VoiceBound" {
                super::voice::receipt_key(required(&v, "binding_id")?)
            } else {
                required(&v, field)?.to_owned()
            };
            if expected_key != k {
                return Err(format!(
                    "StorageUnavailable：域损坏：{table}主键与内容不一致"
                ));
            }
        }
    }
    let wrong: i64 = c
        .query_row(
            "SELECT count(*) FROM applications WHERE fact_key != json_extract(json,'$.fact_key')",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    if wrong != 0 {
        return Err("StorageUnavailable：域损坏：应用事实唯一键与内容不同".into());
    }
    let mut mq = c
        .prepare("SELECT k,payload_hash,receipt_key,request_json FROM messages")
        .map_err(err)?;
    let messages = mq
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(err)?;
    let mut covered = std::collections::BTreeSet::new();
    for m in messages {
        let (k, h, rk, s) = m.map_err(err)?;
        let request =
            crate::session_protocol::validate_envelope(strict_json(s.as_bytes())?, SCHEMA)?;
        config.authorized(&request)?;
        if k != hash(&json!([
            request["source_namespace"],
            request["source_epoch"],
            request["message_id"]
        ])) || request["payload_hash"] != h
        {
            return Err("StorageUnavailable：域损坏：原消息身份/摘要不一致".into());
        }
        let receipt = row(c, "receipts", &rk)?.ok_or("StorageUnavailable：域损坏：消息缺原回执")?;
        let payload = &request["payload"];
        if config.domain == "E" {
            let f = row(c, "facts", &rk)?.ok_or("StorageUnavailable：域损坏：E消息缺原件")?;
            if payload["op"] != "CaptureExternal"
                || payload["raw_utf8"] != f["raw_utf8"]
                || payload["json_pointer"] != f["json_pointer"]
                || payload["token"] != f["token"]
                || payload["source_event_time"] != f["source_event_time"]
                || request["source_namespace"] != f["event"]["source"]["source_namespace"]
                || request["source_epoch"] != f["event"]["source"]["source_epoch"]
                || request["producer_id"] != f["source_producer"]["producer_id"]
                || request["producer_epoch"] != f["source_producer"]["producer_epoch"]
            {
                return Err("StorageUnavailable：域损坏：E消息与原件不一致".into());
            }
        } else if payload["op"] == "BindVoice" {
            super::voice::validate_request(payload)?;
            if receipt["kind"] != "VoiceBound"
                || rk != super::voice::receipt_key(required(payload, "binding_id")?)
            {
                return Err("StorageUnavailable：Voice 消息与永久结果类型/身份不符".into());
            }
            let event = super::voice::event_for(&previous_image, required(payload, "binding_id")?)?;
            if super::voice::original_request(c, event, config)?["payload"] != *payload {
                return Err("StorageUnavailable：Voice 重送消息与原绑定内容不同".into());
            }
        } else if payload["op"] != "ApplyAllocation"
            || payload["allocation_id"] != receipt["allocation_id"]
            || payload["fact_key"] != receipt["fact_key"]
            || payload["expected_fact_hash"] != receipt["fact_hash"]
        {
            return Err("StorageUnavailable：域损坏：B原消息与应用不一致".into());
        }
        covered.insert(rk);
    }
    if covered.len() != (seen - 1) as usize {
        return Err("StorageUnavailable：域损坏：回执缺原消息绑定".into());
    }
    let orphan:i64=c.query_row("SELECT count(*) FROM messages m LEFT JOIN receipts r ON m.receipt_key=r.k WHERE r.k IS NULL",[],|r|r.get(0)).map_err(err)?;
    if orphan != 0 {
        return Err("StorageUnavailable：域损坏：消息缺永久回执".into());
    }
    Ok(())
}
fn e_receipt(record: &Value, cut: Value) -> Value {
    json!({"kind":"Recorded","domain_id":"E","fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"allocation_id":record["event"]["allocation_id"],"target_domain":record["target_domain"],"token":record["token"],"first_known_ns":record["first_known_ns"],"commit_ns":record["commit_ns"],"cut":cut})
}

/// 本域已提交原事实的原消息不可被别名重送替换；双向消息→回执检查仍在 audit 中。
fn verify_original_message(c: &Connection, record: &Value, config: &Config) -> Result<(), String> {
    let is_e = config.domain == "E";
    let origin = &record[if is_e {
        "origin_message"
    } else {
        "b_origin_message"
    }];
    let key = hash(&json!([
        required(origin, "source_namespace")?,
        required(origin, "source_epoch")?,
        required(origin, "message_id")?
    ]));
    let (payload_hash, receipt_key, raw): (String, String, String) = c
        .query_row(
            "SELECT payload_hash,receipt_key,request_json FROM messages WHERE k=?",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "StorageUnavailable：域损坏：已提交事实的原消息绑定丢失".to_owned())?;
    let request = crate::session_protocol::validate_envelope(strict_json(raw.as_bytes())?, SCHEMA)?;
    let actual = json!({"source_namespace":request["source_namespace"],"source_epoch":request["source_epoch"],"message_id":request["message_id"],"payload_hash":request["payload_hash"]});
    let expected_key = if is_e {
        required(record, "fact_key")?
    } else {
        required(&record["event"], "allocation_id")?
    };
    if origin != &actual
        || origin["payload_hash"] != payload_hash
        || receipt_key != expected_key
        || record[if is_e {
            "origin_request_hash"
        } else {
            "b_origin_request_hash"
        }] != hash(&request)
    {
        return Err("StorageUnavailable：域损坏：原消息身份/完整内容/永久回执不再一致".into());
    }
    config.authorized(&request)?;
    Ok(())
}

/// 核已从固定E实读、随B事务保存的原件与提交证明；恢复时不依赖当前E可用。
pub(crate) fn verify_e_copy(record: &Value, config: &Config) -> Result<(), String> {
    if record["e_domain_id"] != "E"
        || record["e_session_id"] != config.session
        || record["e_session_generation"] != config.generation
    {
        return Err("StorageUnavailable：域损坏：B证据副本不属于配置的E会话".into());
    }
    let proof = &record["e_commit_proof"];
    let seq = positive(required(record, "recorded_seq")?, "recorded_seq")?;
    let ns = required(proof, "commit_ns")?;
    let previous = required(proof, "previous_root")?;
    let image = &proof["image"];
    let mut ec = config.clone();
    ec.domain = "E".into();
    let actual_cut = cut(&ec, seq, &root(&ec, seq, previous, ns, image));
    if record["cut"] != actual_cut
        || record["e_receipt"] != e_receipt(record, actual_cut)
        || record["commit_ns"] != ns
    {
        return Err("StorageUnavailable：域损坏：E回执/原件/提交证明不一致".into());
    }
    let mut original = record.clone();
    for key in [
        "cut",
        "e_receipt",
        "e_domain_id",
        "e_session_id",
        "e_session_generation",
        "e_commit_proof",
        "b_observation",
        "b_origin_message",
        "b_origin_request_hash",
    ] {
        original
            .as_object_mut()
            .ok_or("StorageUnavailable：域损坏：B证据非对象")?
            .remove(key);
    }
    if image["recorded"]
        .as_array()
        .filter(|a| a.len() == seq as usize)
        .and_then(|a| a.last())
        != Some(&original)
    {
        return Err("StorageUnavailable：域损坏：E提交证明不含对应完整原件".into());
    }
    Ok(())
}

pub(crate) fn verify_record(r: &Value) -> Result<(), String> {
    let raw = required(r, "raw_utf8")?;
    let (doc, event) = decode(raw, required(r, "json_pointer")?)?;
    if r["raw_sha256"] != sha256_hex(raw.as_bytes())
        || r["event"] != event
        || r["fact_key"] != fact_key(&event)?
        || r["fact_hash"]
            != hash(
                &json!({"raw_sha256":r["raw_sha256"],"json_pointer":r["json_pointer"],"event":event,"units":doc["units"]}),
            )
        || r["target_domain"] != format!("B:{}", target(&event)?)
    {
        return Err("StorageUnavailable：域损坏：原始事实字节/身份/目标错误".into());
    }
    for phase in [Some(r), r.get("b_observation")] {
        if let Some(v) = phase {
            let received = nonnegative(required(v, "received_ns")?, "received_ns")?;
            let known = nonnegative(required(v, "first_known_ns")?, "first_known_ns")?;
            let commit = nonnegative(required(v, "commit_ns")?, "commit_ns")?;
            if received > known || known > commit {
                return Err("StorageUnavailable：域损坏：实际获知阶段倒退".into());
            }
        }
    }
    Ok(())
}
pub(crate) fn recover(config: &Config) -> Result<Value, String> {
    let (mut c, runtime) = open(&config.db, false)?;
    let tx = c.transaction().map_err(err)?;
    audit(&tx, config)?;
    let old = positive(&meta(&tx, "writer_epoch")?, "writer_epoch")?;
    let ns = config.clock(None, "recover_ns", (old - 1) as usize)?;
    let (_, _, last_ns, _) = latest(&tx)?;
    if nonnegative(&ns, "recover_ns")? < nonnegative(&last_ns, "last_commit_ns")? {
        return Err("InvalidDomain：恢复时刻早于已提交事实".into());
    }
    let next = old.checked_add(1).ok_or("InvalidDomain：epoch溢出")?;
    set_meta(&tx, "writer_epoch", &next.to_string())?;
    tx.execute(
        "INSERT INTO epochs VALUES(?,'recover',?)",
        params![next, ns],
    )
    .map_err(err)?;
    let (seq, h, _, _) = latest(&tx)?;
    tx.commit().map_err(err)?;
    Ok(
        json!({"kind":"Recovered","domain_id":config.domain,"writer_epoch":next.to_string(),"cut":cut(config,seq,&h),"recover_ns":ns,"runtime":runtime}),
    )
}
pub(crate) fn immediate(c: &mut Connection) -> Result<Transaction<'_>, String> {
    c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(err)
}
