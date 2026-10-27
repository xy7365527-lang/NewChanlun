use super::book::{decode, fact_key, hash};
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
/// #1394 §5.1：E/B 域版本（信封与持久域）的唯一登记点。
///
/// `economic-session/1` 是现役旧域；`economic-session/2` 只给全新测试会话，
/// 旧域不原地升级、不混写，未知值当场拒绝（§5.8）。
pub(crate) const SCHEMA_V1: &str = "economic-session/1";
pub(crate) const SCHEMA_V2: &str = "economic-session/2";
/// #1394：旧 `/1` 调用与单位测试的兼容别名；
/// 按 manifest 运行的生产调用一律走 `Config::revision()`。
#[cfg(test)]
pub(crate) const SCHEMA: &str = SCHEMA_V1;
/// manifest 顶层的版本选择子：缺省 `/1`（旧 manifest 逐字仍可读），显式只认两个已知值。
/// 外层 `economic-manifest/1` 不因此改变。
const REVISION_FIELD: &str = "protocol_revision";
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
/// `/2` 域的本地内容节点表：格式登记、公开节点、暂存节点。
/// 与 `TABLES` 分开登记：`/1` 域的「恰好 10 表」判据不随新域扩容（§5.8 旧域不混写）。
pub(crate) const CONTENT_TABLES: [&str; 3] = ["content_meta", "content_nodes", "content_staging"];
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
/// TestOnly：#1394 第三问选项 A 的载体实测按 `init` 同一 DDL 建库（不写经济历史行）。
#[cfg(test)]
pub(crate) const TEST_DOMAIN_SQL: &str = SQL;
/// `/2` 域新增的内容表。公开节点的身份就是 §5.2 的 F(tag,body) 摘要，不可变；
/// 暂存按 `stage` 分组，只由 content_store 的发布钩子在同一经济事务内搬入公开表。
pub(crate) const CONTENT_SQL: &str = r#"
CREATE TABLE content_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),content_format TEXT NOT NULL) STRICT;
CREATE TABLE content_nodes(digest BLOB PRIMARY KEY CHECK(typeof(digest)='blob' AND length(digest)=32),tag INTEGER NOT NULL CHECK(tag>=0 AND tag<=255),body BLOB NOT NULL) STRICT;
CREATE TABLE content_staging(digest BLOB PRIMARY KEY CHECK(typeof(digest)='blob' AND length(digest)=32),tag INTEGER NOT NULL CHECK(tag>=0 AND tag<=255),body BLOB NOT NULL,stage TEXT NOT NULL) STRICT;
"#;
/// 域版本分派：唯一入口。缺省 `/1`，显式 `/2`，其余值与形状一律拒绝。
pub(crate) fn domain_revision(manifest: &Value) -> Result<&'static str, String> {
    match manifest.get(REVISION_FIELD) {
        None | Some(Value::Null) => Ok(SCHEMA_V1),
        Some(value) => {
            let text = value
                .as_str()
                .ok_or("SchemaUnsupported：E/B 域版本必须是文本")?;
            if text == SCHEMA_V1 {
                Ok(SCHEMA_V1)
            } else if text == SCHEMA_V2 {
                Ok(SCHEMA_V2)
            } else {
                Err(
                    "SchemaUnsupported：E/B 域版本只有 economic-session/1 与 economic-session/2"
                        .into(),
                )
            }
        }
    }
}

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
    /// 本域实际登记的版本（由 manifest 顶层 `protocol_revision` 显式选择）。
    /// 所有持久域判据、信封校验与内部身份都从这一处取版本，不得各写一份。
    pub(crate) fn revision(&self) -> Result<&'static str, String> {
        domain_revision(&self.manifest)
    }
    /// #1406：本域书账子合同版本（manifest 顶层 `book_revision`），与域版本正交。
    pub(crate) fn book_rules(&self) -> Result<super::book::Rules, String> {
        super::book::rules(&self.manifest)
    }
    pub fn load(path: &Path, db: &Path, chong: Option<&str>) -> Result<Self, String> {
        let manifest = strict_json(&std::fs::read(path).map_err(err)?)?;
        if manifest["schema_revision"] != "economic-manifest/1" {
            return Err("SchemaUnsupported：manifest版本".into());
        }
        // 域版本先判：未知值/混写不得进入任何后续配置校验。
        if domain_revision(&manifest)? == SCHEMA_V2
            && !manifest.get("finance").is_some_and(|v| !v.is_null())
        {
            return Err("SchemaUnsupported：economic-session/2 必须冻结 tb03b-finance/2".into());
        }
        // #1717：SR 扩量是并列子合同，与域版本一同先判；未选择它的 manifest 不认识其操作。
        let sr_expansion = super::sr_expansion::validate_config(&manifest)?;
        super::evidence::validate_config(&manifest)?;
        super::finance::validate_config(&manifest)?;
        super::tb03c_allocation::validate_config(&manifest)?;
        super::tb03c_funding::validate_config(&manifest)?;
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
        if domain_revision(&manifest)? == SCHEMA_V2 {
            let resources = super::financial_reader_resources::FinancialReaderResources::from_json(
                &manifest["finance"]["structure"]["reader_resources"],
            )?;
            #[cfg(target_os = "macos")]
            {
                use std::os::unix::fs::MetadataExt;
                let mut devices = std::collections::BTreeSet::new();
                for path in &paths {
                    let parent = path
                        .parent()
                        .ok_or("InvalidDomain：新域数据库缺父目录")?
                        .canonicalize()
                        .map_err(err)?;
                    if !devices.insert(parent.metadata().map_err(err)?.dev()) {
                        return Err("IdentityConflict：E/B 新域必须各占独立固定容量卷".into());
                    }
                    let capacity = super::content_store::carrier_capacity(path)
                        .map_err(|e| format!("Incomplete：新域固定容量载体：{e}"))?;
                    if capacity > resources.job_limits.max_store_bytes
                        || capacity > resources.job_limits.max_staged_bytes
                    {
                        return Err("Incomplete：新域载体总容量超过冻结 staged/store 上限".into());
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = resources;
                return Err("Incomplete：新域固定容量载体仅在 macOS HFS+ 已证明".into());
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
                let known = [
                    "CaptureExternal",
                    "ResolveEvidence",
                    "ApplyAllocation",
                    "ReadView",
                    "Watch",
                    "EconomicReadOpen",
                    "EconomicReadPage",
                    "EconomicReadClose",
                    "QueryFact",
                    "QueryAllocation",
                    "BindVoice",
                    "QueryVoice",
                    "ImportResponsibility",
                    "ChongAdvance",
                    "PrepareDecision",
                    "Reserve",
                    "CommitDecision",
                    "AbortDecision",
                    "CloseApplicants",
                    "ActivateCapPolicy",
                    "QueryDecision",
                    "QueryGrant",
                ]
                .contains(&op)
                    || (sr_expansion
                        && [super::sr_expansion::OP, super::sr_expansion::QUERY].contains(&op));
                if !known || !unique.insert(op) {
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
        let configured_faults = fault_plans(&manifest["test_only_fault"]);
        let mut fault_keys = std::collections::BTreeSet::new();
        for f in configured_faults {
            if manifest["clock"]["kind"] != "TestOnly" {
                return Err("InvalidDomain：故障暂停仅TestOnly".into());
            }
            required(f, "message_id")?;
            absolute(required(f, "marker_path")?)?;
            if !fault_keys.insert((required(f, "message_id")?, required(f, "phase")?)) {
                return Err("IdentityConflict：故障调度重复".into());
            }
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
        if let Some(f) = fault_plans(&self.manifest["test_only_fault"])
            .into_iter()
            .find(|f| f["message_id"] == msg && f["phase"] == phase)
        {
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
fn fault_plans(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => vec![],
        Value::Array(values) => values.iter().collect(),
        _ => vec![value],
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
    open_metered(db, read_only, &mut ReadBudget::new(None, None))
}
pub(super) fn open_metered(
    db: &Path,
    read_only: bool,
    budget: &mut ReadBudget<'_>,
) -> Result<(Connection, Value), String> {
    budget.before()?;
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
    let mode: String = budget.one(&c, "PRAGMA journal_mode", [], |r| r.get(0))?;
    let sync: i64 = budget.one(&c, "PRAGMA synchronous", [], |r| r.get(0))?;
    let full: i64 = budget.one(&c, "PRAGMA fullfsync", [], |r| r.get(0))?;
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
pub(crate) use super::store_read::ReadBudget;
pub(crate) fn metered_row(
    c: &Connection,
    table: &str,
    k: &str,
    budget: &mut ReadBudget<'_>,
) -> Result<Option<Value>, String> {
    if !TABLES.contains(&table) {
        return Err("InvalidDomain：未知表".into());
    }
    let raw: Option<String> = budget.optional(
        c,
        &format!("SELECT json FROM {table} WHERE k=?"),
        [k],
        |r| r.get(0),
    )?;
    raw.map(|s| strict_json(s.as_bytes())).transpose()
}
fn metered_rows(
    c: &Connection,
    table: &str,
    budget: &mut ReadBudget<'_>,
) -> Result<Vec<Value>, String> {
    if !TABLES.contains(&table) {
        return Err("InvalidDomain：未知表".into());
    }
    budget.before()?;
    let mut statement = c
        .prepare(&format!("SELECT json FROM {table} ORDER BY k"))
        .map_err(err)?;
    let mut cursor = statement.query([]).map_err(err)?;
    let mut values = Vec::new();
    while let Some(row) = budget.next(&mut cursor)? {
        let raw: String = row.get(0).map_err(err)?;
        values.push(strict_json(raw.as_bytes())?);
    }
    Ok(values)
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
/// #1394/C06.5：把本事务已核证据覆盖、从完整图像移除的 Pending 持久清除。
/// 只删当前图像已不含的 token；连续 Capture 的新 token 不在其中，不被前一个 Resolve 带走。
pub(super) fn sync_pending(c: &Connection, image: &Value) -> Result<(), String> {
    let kept: std::collections::BTreeSet<String> = image["pending"]
        .as_array()
        .ok_or("StorageUnavailable：域损坏：pending形状")?
        .iter()
        .filter_map(|p| p["fact_key"].as_str().map(str::to_owned))
        .collect();
    let stale: Vec<String> = rows(c, "pending")?
        .iter()
        .filter_map(|p| p["fact_key"].as_str().map(str::to_owned))
        .filter(|k| !kept.contains(k))
        .collect();
    for k in stale {
        c.execute("DELETE FROM pending WHERE k=?", [&k])
            .map_err(err)?;
    }
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
pub(super) fn root(config: &Config, seq: i64, prev: &str, ns: &str, image: &Value) -> String {
    root_with_image_hash(config, seq, prev, ns, &hash(image))
}
/// #1768：同一根式只按图像摘要计算；delta 续接据此逐链核根，不另抄公式。
pub(super) fn root_with_image_hash(
    config: &Config,
    seq: i64,
    prev: &str,
    ns: &str,
    image_hash: &str,
) -> String {
    hash(
        &json!({"domain_id":config.domain,"session_id":config.session,"session_generation":config.generation,"commit_seq":seq.to_string(),"previous_root":prev,"commit_ns":ns,"image_hash":image_hash}),
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
/// #1406 C6a：只读页的持久身份；先读长度，再允许有界原字节读取。
/// 不持有跨请求 SQLite 事务，不新增持久游标或修改经济事实。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StoredImage {
    pub seq: i64,
    pub root: String,
    pub previous_root: String,
    pub commit_ns: String,
    pub bytes: usize,
}

pub(super) fn stored_image(
    c: &Connection,
    seq: i64,
    maximum: usize,
) -> Result<StoredImage, String> {
    // CAST 为 BLOB 很关键：SQLite TEXT 的 length/substr 是字符数，并非 UTF-8 字节数。
    let row: Option<(String, String, String, i64)> = c
        .query_row(
            "SELECT root,prev_root,commit_ns,length(CAST(image AS BLOB)) FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(err)?;
    let (root, previous_root, commit_ns, bytes) =
        row.ok_or("MissingDependency：指定历史cut不存在")?;
    let bytes = usize::try_from(bytes).map_err(err)?;
    if bytes == 0 || bytes > maximum {
        return Err("Incomplete：完整持久图像超出读取逻辑字节帽".into());
    }
    Ok(StoredImage {
        seq,
        root,
        previous_root,
        commit_ns,
        bytes,
    })
}

pub(super) fn stored_image_bytes(
    c: &Connection,
    image: &StoredImage,
    offset: usize,
    count: usize,
) -> Result<Vec<u8>, String> {
    let end = offset
        .checked_add(count)
        .filter(|end| count > 0 && *end <= image.bytes)
        .ok_or("InvalidDomain：原图像页范围越界")?;
    let start =
        i64::try_from(offset.checked_add(1).ok_or("InvalidDomain：页offset溢出")?).map_err(err)?;
    let count_sql = i64::try_from(end - offset).map_err(err)?;
    let size_sql = i64::try_from(image.bytes).map_err(err)?;
    let bytes: Option<Vec<u8>> = c
        .query_row(
            "SELECT substr(CAST(image AS BLOB),?,?) FROM commits WHERE seq=? AND root=? AND prev_root=? AND commit_ns=? AND length(CAST(image AS BLOB))=?",
            params![start, count_sql, image.seq, image.root, image.previous_root, image.commit_ns, size_sql],
            |r| r.get(0),
        )
        .optional()
        .map_err(err)?;
    let bytes = bytes.ok_or("IdentityConflict：固定cut原图像身份已改变")?;
    if bytes.len() != count {
        return Err("StorageUnavailable：原图像页截短".into());
    }
    Ok(bytes)
}
pub(crate) fn append(
    c: &Connection,
    config: &Config,
    seq: i64,
    prev: &str,
    ns: &str,
    image: &Value,
) -> Result<Value, String> {
    let raw = readable_image(image)?;
    let h = root(config, seq, prev, ns, image);
    c.execute(
        "INSERT INTO commits(seq,prev_root,root,commit_ns,image)VALUES(?,?,?,?,?)",
        params![seq, prev, h, ns, raw],
    )
    .map_err(err)?;
    Ok(cut(config, seq, &h))
}
/// #1406 C5 H1：最终完整图像必须满足唯一读合同；不放宽解析深度。
/// 这是有界安全接受域，不承诺任意深度来源都能嵌入新的跨 owner 证明。
pub(super) fn readable_image(image: &Value) -> Result<String, String> {
    let raw = canonical_json(image);
    strict_json(raw.as_bytes())
        .map_err(|e| format!("SchemaUnsupported：完整持久图像超出统一可读安全域：{e}"))?;
    Ok(raw)
}
pub(crate) fn init(config: &Config) -> Result<Value, String> {
    if config.db.exists() {
        return Err("IdentityConflict：init拒绝覆盖已有库".into());
    }
    let revision = config.revision()?;
    let (mut c, runtime) = open(&config.db, false)?;
    let tx = c.transaction().map_err(err)?;
    tx.execute_batch(SQL).map_err(err)?;
    if revision == SCHEMA_V2 {
        tx.execute_batch(CONTENT_SQL).map_err(err)?;
    }
    for (k, v) in [
        ("schema_revision", revision),
        ("domain_id", config.domain.as_str()),
        ("session_id", config.session.as_str()),
        ("session_generation", config.generation.as_str()),
        ("manifest_hash", config.manifest_hash.as_str()),
        ("writer_epoch", "1"),
    ] {
        set_meta(&tx, k, v)?;
    }
    if revision == SCHEMA_V2 {
        // §5.2 内容格式登记只由 init 显式绑定；recover 不暗迁移。
        tx.execute(
            "INSERT INTO content_meta VALUES(1,?)",
            params![crate::s_content::CONTENT_FORMAT],
        )
        .map_err(err)?;
    }
    if super::evidence::enabled(config) {
        set_meta(&tx, "voice_binding_revision", "voice-binding/1")?;
    }
    // #1406 / 根裁定 OQ-7：书账子合同版本只由 init 显式绑定；旧域无此行。
    if let Some(revision) = config.book_rules()?.revision() {
        set_meta(&tx, "book_revision", revision)?;
    }
    if let Some(revision) = super::sr_expansion::bound_revision(config)? {
        set_meta(&tx, super::sr_expansion::META_KEY, revision)?;
    }
    let ns = config.clock(None, "init_ns", 0)?;
    tx.execute("INSERT INTO epochs VALUES(1,'init',?)", [&ns])
        .map_err(err)?;
    let cut = append(&tx, config, 0, "", &ns, &super::finance::empty(config)?)?;
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
    let mut independent = if config.revision()? == SCHEMA_V2 {
        Some(new_content_job(c, config)?)
    } else {
        None
    };
    audit_with_job(c, config, independent.as_mut())
}
pub(crate) fn new_content_job(
    c: &Connection,
    config: &Config,
) -> Result<crate::s_content::job::Job, String> {
    let resources = super::financial_reader_resources::FinancialReaderResources::from_json(
        &config.manifest["finance"]["structure"]["reader_resources"],
    )?;
    let existing = super::content_store::existing(c, &config.db)
        .map_err(|e| format!("StorageUnavailable：内容资源基线：{e}"))?;
    crate::s_content::job::Job::new(
        resources.job_limits,
        existing,
        Box::new(std::time::Instant::now),
    )
    .map_err(|e| format!("Incomplete：内容 job：{e}"))
}
pub(crate) fn operation_deadline(
    config: &Config,
    job: &mut crate::s_content::job::Job,
    ingress_response_deadline: Option<std::time::Instant>,
) -> Result<std::time::Instant, String> {
    let ms = u64::try_from(positive(
        required(&config.manifest["resources"], "response_timeout_ms")?,
        "response_timeout_ms",
    )?)
    .map_err(err)?;
    let response = match ingress_response_deadline {
        Some(trusted) => trusted,
        None => std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(ms))
            .ok_or("InvalidDomain：内部经济响应绝对期限溢出")?,
    };
    Ok(response.min(job.first_read_deadline().map_err(|e| e.to_string())?))
}
pub(crate) fn check_operation_deadline(
    mut job: Option<&mut crate::s_content::job::Job>,
    deadline: Option<std::time::Instant>,
) -> Result<(), String> {
    if let Some(job) = job.as_deref_mut() {
        job.pre_io_check()
            .map_err(|e| format!("Incomplete：经济job已停止：{e}"))?;
    }
    if deadline.is_some_and(|at| std::time::Instant::now() >= at) {
        return Err("Incomplete：经济操作最早绝对期限已过".into());
    }
    Ok(())
}
/// The caller's one operation Job covers local history and peer audits.
/// Only independent recovery/read work creates its own Job via `audit`.
pub(crate) fn audit_with_job(
    c: &Connection,
    config: &Config,
    shared_job: Option<&mut crate::s_content::job::Job>,
) -> Result<(), String> {
    audit_with_deadline(c, config, shared_job, None)
}
pub(crate) fn audit_with_deadline(
    c: &Connection,
    config: &Config,
    shared_job: Option<&mut crate::s_content::job::Job>,
    deadline: Option<std::time::Instant>,
) -> Result<(), String> {
    let mut budget = ReadBudget::new(shared_job, deadline);
    audit_with_budget(c, config, &mut budget)
}
pub(super) fn audit_with_budget(
    c: &Connection,
    config: &Config,
    budget: &mut ReadBudget<'_>,
) -> Result<(), String> {
    let revision = config.revision()?;
    if revision == SCHEMA_V2 {
        let resources = super::financial_reader_resources::FinancialReaderResources::from_json(
            &config.manifest["finance"]["structure"]["reader_resources"],
        )?;
        let job = budget
            .job
            .as_deref_mut()
            .ok_or("MissingDependency：新域审计缺共同 job")?;
        if job.snapshot().limits != resources.job_limits {
            return Err("IdentityConflict：新域审计 job 限额与冻结 manifest 不同".into());
        }
        job.begin_first_read()
            .map_err(|e| format!("Incomplete：审计起钟：{e}"))?;
    }
    budget.before()?;
    let expected = Connection::open_in_memory().map_err(err)?;
    expected.execute_batch(SQL).map_err(err)?;
    if revision == SCHEMA_V2 {
        expected.execute_batch(CONTENT_SQL).map_err(err)?;
    }
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
    let actual_schema: Vec<(String, String, String)> = budget.all(
        c,
        "SELECT type,name,sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type,name",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    if actual_schema != schema_dump(&expected)? {
        return Err("StorageUnavailable：域损坏：完整schema/索引发生变化".into());
    }
    let integrity: String = budget.one(c, "PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(format!("StorageUnavailable：域损坏：SQLite {integrity}"));
    }
    let names: Vec<String> = budget.all(c,"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name", [], |r| r.get(0))?;
    // 表集合按域版本登记：`/1` 恰好 10 表，`/2` 另加内容三表（§5.8 旧域不混写）。
    let mut want: Vec<String> = if revision == SCHEMA_V2 {
        TABLES
            .iter()
            .chain(CONTENT_TABLES.iter())
            .map(|s| s.to_string())
            .collect()
    } else {
        TABLES.iter().map(|s| s.to_string()).collect()
    };
    want.sort();
    if names != want {
        return Err("StorageUnavailable：域损坏：表集合变化".into());
    }
    let extra: i64 = budget.one(
        c,
        "SELECT count(*) FROM sqlite_master WHERE type IN ('trigger','view')",
        [],
        |r| r.get(0),
    )?;
    if extra != 0 {
        return Err("StorageUnavailable：域损坏：非预期触发器/视图".into());
    }
    for (k, v) in [
        ("schema_revision", revision),
        ("domain_id", config.domain.as_str()),
        ("session_id", config.session.as_str()),
        ("session_generation", config.generation.as_str()),
        ("manifest_hash", config.manifest_hash.as_str()),
    ] {
        let actual: String = budget.one(c, "SELECT v FROM meta WHERE k=?", [k], |r| r.get(0))?;
        if actual != v {
            return Err(format!("IdentityConflict：持久{k}未绑定当前配置"));
        }
    }
    let voice_revision: Option<String> = budget.optional(
        c,
        "SELECT v FROM meta WHERE k='voice_binding_revision'",
        [],
        |r| r.get(0),
    )?;
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
    let book_revision: Option<String> =
        budget.optional(c, "SELECT v FROM meta WHERE k='book_revision'", [], |r| {
            r.get(0)
        })?;
    if book_revision.as_deref() != config.book_rules()?.revision() {
        return Err("IdentityConflict：书账版本未由 init 显式绑定；不得在 recover 暗迁移".into());
    }
    let sr_revision: Option<String> = budget.optional(
        c,
        "SELECT v FROM meta WHERE k=?",
        [super::sr_expansion::META_KEY],
        |r| r.get(0),
    )?;
    if sr_revision.as_deref() != super::sr_expansion::bound_revision(config)? {
        return Err(
            "IdentityConflict：SR 扩量合同版本未由 init 显式绑定；不得在 recover 暗迁移".into(),
        );
    }
    // 内容格式登记与 Voice 版本同一条纪律：只认 init 显式绑定，recover 不暗迁移。
    let content_format: Option<String> = if revision == SCHEMA_V2 {
        budget.optional(
            c,
            "SELECT content_format FROM content_meta WHERE singleton=1",
            [],
            |r| r.get(0),
        )?
    } else {
        None
    };
    if content_format.as_deref()
        != if revision == SCHEMA_V2 {
            Some(crate::s_content::CONTENT_FORMAT)
        } else {
            None
        }
    {
        return Err(
            "IdentityConflict：内容存储格式未由 init 显式绑定；不得在 recover 暗迁移".into(),
        );
    }
    let epoch_raw: String =
        budget.one(c, "SELECT v FROM meta WHERE k='writer_epoch'", [], |r| {
            r.get(0)
        })?;
    let epoch = positive(&epoch_raw, "writer_epoch")?;
    let (count, max): (i64, i64) =
        budget.one(c, "SELECT count(*),max(epoch) FROM epochs", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    if count != epoch || max != epoch {
        return Err("StorageUnavailable：域损坏：epoch历史不连续".into());
    }
    budget.before()?;
    let mut q = c
        .prepare("SELECT seq,prev_root,root,commit_ns,image FROM commits ORDER BY seq")
        .map_err(err)?;
    let mut rs = q.query([]).map_err(err)?;
    let mut previous = String::new();
    let mut previous_image = super::finance::empty(config)?;
    let mut last_ns = -1;
    let mut seen = 0i64;
    while let Some(r) = budget.next(&mut rs)? {
        let (seq, prev, h, ns, raw): (i64, String, String, String, String) = (
            r.get(0).map_err(err)?,
            r.get(1).map_err(err)?,
            r.get(2).map_err(err)?,
            r.get(3).map_err(err)?,
            r.get(4).map_err(err)?,
        );
        let image = strict_json(raw.as_bytes())?;
        let n = nonnegative(&ns, "commit_ns")?;
        if seq != seen
            || prev != previous
            || h != root(config, seq, &prev, &ns, &image)
            || n < last_ns
        {
            return Err("StorageUnavailable：域损坏：提交链/时间/根不一致".into());
        }
        if revision == SCHEMA_V2 {
            super::content_store::verify_image_with_budget(
                c,
                &image,
                &config.manifest["finance"]["structure"],
                budget,
            )
            .map_err(|e| format!("StorageUnavailable：本地内容闭包审计失败：{e}"))?;
        }
        if seq == 0 {
            if image != super::finance::empty(config)? {
                return Err("StorageUnavailable：域损坏：创世图像非空".into());
            }
        } else if image["history"]
            .as_array()
            .and_then(|a| a.last())
            .is_some_and(|e| e["kind"] == "ResolveEvidence")
        {
            let event = image["history"].as_array().unwrap().last().unwrap();
            if image != super::tb03c_e::replay(c, config, &previous_image, event, seq, &ns, budget)?
            {
                return Err("StorageUnavailable：Resolve 重放不等于完整提交图像".into());
            }
        } else if image["history"]
            .as_array()
            .and_then(|a| a.last())
            .is_some_and(|e| e["kind"] == super::sr_expansion::OP)
        {
            let event = image["history"].as_array().unwrap().last().unwrap();
            let expected =
                super::sr_expansion::replay(c, config, &previous_image, event, seq, &ns, budget)?;
            if image != expected {
                return Err("StorageUnavailable：SR 评估重放不等于完整提交图像".into());
            }
        } else if image["history"]
            .as_array()
            .and_then(|a| a.last())
            .is_some_and(super::finance::is_event)
        {
            let event = image["history"].as_array().unwrap().last().unwrap();
            if image != super::finance::replay(c, config, &previous_image, event, seq, &ns, budget)?
            {
                return Err("StorageUnavailable：财务重放不等于完整提交图像".into());
            }
        } else if image["history"]
            .as_array()
            .and_then(|a| a.last())
            .map(|e| e["kind"] == "BindVoice")
            .unwrap_or(false)
        {
            let event = image["history"].as_array().unwrap().last().unwrap();
            let expected =
                super::voice::replay(c, config, &previous_image, event, seq, &ns, budget)?;
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
            verify_record(record, config.book_rules()?)?;
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
            verify_original_message(c, record, config, budget)?;
            let expected = if config.domain == "E" {
                let mut x = previous_image.clone();
                x["recorded"].as_array_mut().unwrap().push(record.clone());
                let p = super::tb03c_e::pending_entry(config, &previous_image, record, seq)?;
                x["pending"].as_array_mut().unwrap().push(p);
                x["history"].as_array_mut().unwrap().push(json!({"kind":"CaptureExternal","fact_key":record["fact_key"],"commit_seq":seq.to_string(),"commit_ns":ns}));
                x
            } else {
                super::book::apply_in(&config.manifest, &previous_image, record, seq, &ns)?.0
            };
            if image != expected {
                return Err("StorageUnavailable：域损坏：原件重放不等于提交图像".into());
            }
        }
        previous = h;
        previous_image = image;
        last_ns = n;
        seen += 1;
        super::phase_trace::mark_seq("audit_commit", &config.domain, seq);
    }
    budget.before()?;
    if seen == 0 {
        return Err("StorageUnavailable：域损坏：无创世提交".into());
    }
    for (table, key) in [
        ("facts", "recorded"),
        ("pending", "pending"),
        ("applications", "applied"),
        ("unmatched", "unmatched"),
    ] {
        if sorted(metered_rows(c, table, budget)?)
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
    if metered_row(c, "book", "book", budget)?.unwrap_or(Value::Null) != previous_image["book"] {
        return Err("StorageUnavailable：域损坏：book物化不等于完整图像".into());
    }
    let receipt_rows = metered_rows(c, "receipts", budget)?;
    if receipt_rows.len() != (seen - 1) as usize {
        return Err("StorageUnavailable：域损坏：永久回执数不等于业务提交数".into());
    }
    for r in receipt_rows {
        if r["kind"] == "Resolved" {
            super::tb03c_e::audit_receipt(c, config, &previous_image, &r, budget)?;
            continue;
        }
        if r["kind"] == super::sr_expansion::RECEIPT {
            super::sr_expansion::audit_receipt(c, config, &r, budget)?;
            continue;
        }
        if r.get("receipt_key").is_some() {
            super::finance::audit_receipt(c, config, &r, budget)?;
            continue;
        }
        if r["kind"] == "VoiceBound" {
            if config.domain == "E" {
                return Err("StorageUnavailable：E 非法持有 Voice 回执".into());
            }
            let record = super::voice::find_binding(&previous_image, required(&r, "binding_id")?)?
                .ok_or("StorageUnavailable：Voice 回执缺不可变对象")?;
            let seq = positive(required(&record, "commit_seq")?, "Voice actual commit seq")?;
            let (actual_root, ns, raw): (String, String, String) = budget.one(
                c,
                "SELECT root,commit_ns,image FROM commits WHERE seq=?",
                [seq],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
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
        let f = metered_row(c, "facts", required(&r, "fact_key")?, budget)?
            .ok_or("StorageUnavailable：域损坏：回执缺原件")?;
        let a = if config.domain == "E" {
            None
        } else {
            Some(
                metered_row(
                    c,
                    "applications",
                    super::tb03c_allocation::application_id(&f)?,
                    budget,
                )?
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
        let (actual_root, ns, raw): (String, String, String) = budget.one(
            c,
            "SELECT root,commit_ns,image FROM commits WHERE seq=?",
            [seq],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
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
            json!({"kind":a["status"],"domain_id":config.domain,"fact_key":f["fact_key"],"fact_hash":f["fact_hash"],"allocation_id":super::tb03c_allocation::application_id(&f)?,"target_domain":config.domain,"received_ns":f["b_observation"]["received_ns"],"first_known_ns":f["b_observation"]["first_known_ns"],"commit_ns":ns,"e_first_known_ns":f["first_known_ns"],"cut":cut(config,seq,&actual_root)})
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
    super::phase_trace::mark("audit_receipts", &config.domain);
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
        budget.before()?;
        let mut q = c
            .prepare(&format!("SELECT k,json FROM {table}"))
            .map_err(err)?;
        let mut rs = q.query([]).map_err(err)?;
        while let Some(r) = budget.next(&mut rs)? {
            let (k, s): (String, String) = (r.get(0).map_err(err)?, r.get(1).map_err(err)?);
            let v = strict_json(s.as_bytes())?;
            let expected_key = if table == "receipts" && v["kind"] == "Resolved" {
                super::tb03c_e::receipt_key(required(&v, "evidence_id")?)
            } else if table == "receipts" && v["kind"] == super::sr_expansion::RECEIPT {
                super::sr_expansion::receipt_key(required(&v, "evaluation_id")?)
            } else if table == "receipts" && v.get("receipt_key").is_some() {
                required(&v, "receipt_key")?.to_owned()
            } else if table == "receipts" && v["kind"] == "VoiceBound" {
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
    let wrong: i64 = budget.one(
        c,
        "SELECT count(*) FROM applications WHERE fact_key != json_extract(json,'$.fact_key')",
        [],
        |r| r.get(0),
    )?;
    if wrong != 0 {
        return Err("StorageUnavailable：域损坏：应用事实唯一键与内容不同".into());
    }
    budget.before()?;
    let mut mq = c
        .prepare("SELECT k,payload_hash,receipt_key,request_json FROM messages")
        .map_err(err)?;
    let mut messages = mq.query([]).map_err(err)?;
    let mut covered = std::collections::BTreeSet::new();
    while let Some(m) = budget.next(&mut messages)? {
        let (k, h, rk, s): (String, String, String, String) = (
            m.get(0).map_err(err)?,
            m.get(1).map_err(err)?,
            m.get(2).map_err(err)?,
            m.get(3).map_err(err)?,
        );
        let request =
            crate::session_protocol::validate_envelope(strict_json(s.as_bytes())?, revision)?;
        config.authorized(&request)?;
        if k != hash(&json!([
            request["source_namespace"],
            request["source_epoch"],
            request["message_id"]
        ])) || request["payload_hash"] != h
        {
            return Err("StorageUnavailable：域损坏：原消息身份/摘要不一致".into());
        }
        let receipt = metered_row(c, "receipts", &rk, budget)?
            .ok_or("StorageUnavailable：域损坏：消息缺原回执")?;
        let payload = &request["payload"];
        if receipt["kind"] == "Resolved" {
            let event =
                super::tb03c_e::event_for(&previous_image, required(&receipt, "evidence_id")?)?;
            if config.domain != "E"
                || payload["op"] != "ResolveEvidence"
                || rk != super::tb03c_e::receipt_key(required(payload, "evidence_id")?)
                || super::tb03c_e::evidence_body(payload) != event["evidence"]
                || payload["frontier"]["source_namespace"] != request["source_namespace"]
                || payload["frontier"]["source_epoch"] != request["source_epoch"]
            {
                return Err("StorageUnavailable：Resolve 消息与原证据/来源不一致".into());
            }
        } else if receipt["kind"] == super::sr_expansion::RECEIPT {
            if config.domain != "E"
                || payload["op"] != super::sr_expansion::OP
                || rk != super::sr_expansion::receipt_key(required(payload, "evaluation_id")?)
            {
                return Err("StorageUnavailable：SR 消息与永久回执类型/身份不符".into());
            }
            super::sr_expansion::audit_message(c, config, &request, &receipt, budget)?;
        } else if receipt.get("receipt_key").is_some() {
            super::finance::audit_message(c, config, &request, &receipt, budget)?;
        } else if config.domain == "E" {
            let f = metered_row(c, "facts", &rk, budget)?
                .ok_or("StorageUnavailable：域损坏：E消息缺原件")?;
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
            if super::voice::original_request(c, event, config, budget)?["payload"] != *payload {
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
    let orphan:i64=budget.one(c,"SELECT count(*) FROM messages m LEFT JOIN receipts r ON m.receipt_key=r.k WHERE r.k IS NULL",[],|r|r.get(0))?;
    if orphan != 0 {
        return Err("StorageUnavailable：域损坏：消息缺永久回执".into());
    }
    Ok(())
}
fn e_receipt(record: &Value, cut: Value) -> Value {
    json!({"kind":"Recorded","domain_id":"E","fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"allocation_id":record["event"]["allocation_id"],"target_domain":record["target_domain"],"token":record["token"],"first_known_ns":record["first_known_ns"],"commit_ns":record["commit_ns"],"cut":cut})
}

/// 本域已提交原事实的原消息不可被别名重送替换；双向消息→回执检查仍在 audit 中。
fn verify_original_message(
    c: &Connection,
    record: &Value,
    config: &Config,
    budget: &mut ReadBudget<'_>,
) -> Result<(), String> {
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
    let (payload_hash, receipt_key, raw): (String, String, String) = budget
        .optional(
            c,
            "SELECT payload_hash,receipt_key,request_json FROM messages WHERE k=?",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?
        .ok_or("StorageUnavailable：域损坏：已提交事实的原消息绑定丢失")?;
    // 信封版本与本域登记一致：`/1` 与 `/2` 各读自己的原消息，不互相解释。
    let revision = config.revision()?;
    let request =
        crate::session_protocol::validate_envelope(strict_json(raw.as_bytes())?, revision)?;
    let actual = json!({"source_namespace":request["source_namespace"],"source_epoch":request["source_epoch"],"message_id":request["message_id"],"payload_hash":request["payload_hash"]});
    let expected_key = if is_e {
        required(record, "fact_key")?
    } else {
        super::tb03c_allocation::application_id(record)?
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

/// #1621：B 对一份 E 事实的首次获知不得早于该事实在 E 的持久提交。
/// 纯 E 副本尚未附加 b_observation，此时无可比较的 B 时刻，直接通过。
pub(crate) fn verify_b_observation_after_e_commit(record: &Value) -> Result<(), String> {
    let Some(observation) = record.get("b_observation") else {
        return Ok(());
    };
    let known = nonnegative(required(observation, "first_known_ns")?, "B first_known_ns")?;
    let e_commit = nonnegative(required(record, "commit_ns")?, "E commit_ns")?;
    if known < e_commit {
        return Err("InvalidDomain：B 获知时刻早于 E 提交".into());
    }
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
    verify_b_observation_after_e_commit(record)?;
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
        "e_resolution_proof",
        "resolved_allocation",
        "b_observation",
        "b_origin_message",
        "b_origin_request_hash",
        "b_source_debit_proof",
    ] {
        original
            .as_object_mut()
            .ok_or("StorageUnavailable：域损坏：B证据非对象")?
            .remove(key);
    }
    if image["recorded"].as_array().and_then(|a| a.last()) != Some(&original) {
        return Err("StorageUnavailable：域损坏：E提交证明不含对应完整原件".into());
    }
    super::tb03c_allocation::verify_resolution(record, config)?;
    // E 实读阶段尚未附加源证明；凡本地已提交的 B 原件必须完整自持。
    if config.book_rules()?.is_tb03c()
        && (record.get("b_observation").is_some() || record.get("b_source_debit_proof").is_some())
    {
        super::tb03c_transfer::verify_source_proof(config, record)?;
    }
    Ok(())
}

pub(crate) fn verify_record(r: &Value, rules: super::book::Rules) -> Result<(), String> {
    let raw = required(r, "raw_utf8")?;
    let (doc, event) = decode(raw, required(r, "json_pointer")?, rules)?;
    if r["raw_sha256"] != sha256_hex(raw.as_bytes())
        || r["event"] != event
        || r["fact_key"] != fact_key(&event)?
        || r["fact_hash"]
            != hash(
                &json!({"raw_sha256":r["raw_sha256"],"json_pointer":r["json_pointer"],"event":event,"units":doc["units"]}),
            )
        || r["target_domain"] != super::tb03c_allocation::target_domain(&event)?
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
    if config.revision()? == SCHEMA_V2 {
        let resources = super::financial_reader_resources::FinancialReaderResources::from_json(
            &config.manifest["finance"]["structure"]["reader_resources"],
        )?;
        super::content_store::configure(&c, &config.db, resources.job_limits)
            .map_err(|e| format!("Incomplete：恢复载体配置失败：{e}"))?;
    }
    let tx = c.transaction().map_err(err)?;
    audit(&tx, config)?;
    if config.revision()? == SCHEMA_V2 {
        super::content_store::discard_in_transaction(&tx)
            .map_err(|e| format!("Incomplete：恢复清理未发布节点失败：{e}"))?;
    }
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
