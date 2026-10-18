//! #1371 TB-01-B：S 正式结构会话的修订、撤回、持久 Delta 与真实进程恢复（SPEC #1340 / 图 #1323）。
//!
//! 本二进制在 #1370 TB-01-A 已合的正式结构会话之上，为同一严格 Rust 核（`ParseLayerIncr` +
//! `classify_local_shape`）补齐 B 片必需行为：
//!
//! - **合法修订链**：源事件 `(namespace, epoch, instrument, event_id)` 业务身份不含 revision；
//!   同 revision 同内容重放返回原收据、同 revision 异内容 `IdentityConflict`；更高 revision 以
//!   `supersedes/replaces` 关联旧记录，旧版重放不复活；更低 revision 晚到（已见更高）拒绝。
//! - **有效源位置语义**：源 `seq` 是源坐标，持久接纳序（raw_events.seq）另有含义；结构计算按
//!   「每个源坐标的最新 revision」在源坐标序上取有效值，晚到 e2 新版不当作第五 tick。
//! - **持久 StructureDelta + 对象撤回**：每次 Advance 计算新完整对象集，与既有活动对象对拍，持久
//!   撤回（withdraw）/替代（replaces）/upsert，对象保留原身份、first_known、发生区间与撤回理由；
//!   当前视图移除不删历史。对象/边/端点在同一事务原子应用。
//! - **真实 writer 换代**：`writer_epoch` 为持久值；`recover` 是正式换代命令（清未决 Begin + 提升
//!   epoch，与 generation 同序），旧 epoch 进程重放被 `StaleWriter` 拒绝，不能读新 epoch 自授权，
//!   也不能手改 SQL 当换代。
//! - **真实 SIGKILL 恢复**：具名 TestOnly 暂停点（`S_SESSION_PAUSE`）暴露真实阶段（Begin 后 / 完整
//!   批次后 / Commit 后），供外部对精确 S PID 发 SIGKILL；重启经正式入口读原接纳序/身份、未决
//!   Begin、可达根、writer 代际；未完修订保持相关门关闭，合法恢复后才能继续；不可达 batch 不冒充
//!   已发布 cut。
//! - **同源 Watch / 指定 cut Snapshot / AsKnown / RecomputedWithRevision**：持久 delta 可经
//!   `watch`（同源读取 S 自己的库）续接，指定 cut 快照逐字段可对拍；AsKnown（`--as-of`）按当时
//!   可知，RecomputedWithRevision（默认）绑定最新 input/rule 版本。
//!
//! ## 生产零调用纪律（不变）
//!
//! - 真正接通 `theta_v0::parser::ParseLayerIncr::append` 与 `theta_v0::classifier::local_shape`，
//!   不写第二份生产判定器；不调用 `ThetaPiStream` 资金推进冒充 S；不启动 E/B/X。
//! - S 持久域是 S 自己的数据库文件（`--db`），不复用 `trading_system/persistence/database.py` 共库。

#[path = "s_session_v2/mod.rs"]
mod v2;
// #1394 C §5.1/5.8：S3 新域（s-session/3）的隔离初始化与版本分派；本文件只挂模块与入口选择。
#[path = "s_session_v3/raw_input.rs"]
mod raw_input;
#[path = "s_session_v3/mod.rs"]
mod v3;
// #1394 C：对象集对拍判据的共用纯函数（旧 S2 adapter 与 S3 新域共用一份，见 `compute_diff`）。
#[path = "s_session_v3/object_diff.rs"]
mod object_diff;

use newchan_rust::s_content::capture_allocation::native_owned::NativeValue;
use std::collections::BTreeMap;
use v3::business_json::{PlainJson, TextValue};

#[path = "s_session_v2/audit.rs"]
mod audit;
#[path = "s_session_v2/cache.rs"]
mod cache;
// #1394：新库的显式私有存储格式登记（changes-only/1）；旧库无该表即旧 full 域。
#[path = "s_session_v2/delta_storage.rs"]
mod delta_storage;
// #1394 第二段：完整证据的私有内容寻址共享表示（evidence-graph/2）；旧库无该表即旧整块域。
#[path = "s_session_v2/content_store.rs"]
mod content_store;
#[path = "s_session_v2/evidence_store.rs"]
mod evidence_store;
#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[path = "s_session_v2/staged_nodes.rs"]
mod staged_nodes;
// #1394 采纳 C §5.2/5.3：S 生产侧从同核共享证据直接构造 CC-004 内容对象（生产构造器，
// 供正式 S3 入口调用；本文件不接 S3Commit）。
#[path = "s_session_v2/content_bi.rs"]
mod content_bi;
#[path = "s_session_v2/content_manifest.rs"]
mod content_manifest;
#[path = "s_session_v2/content_producer.rs"]
mod content_producer;
#[path = "s_session_v2/content_publication.rs"]
mod content_publication;
// #1394 2A-S：正式 S 的同代增量状态与稳定对象复用（进程内缓存；性能结构，不充任准入权威）。
#[path = "s_session_v2/s_fact_cache.rs"]
mod s_fact_cache;
#[path = "s_session_v2/tb02a.rs"]
mod tb02a;
#[path = "s_session_v2/tb02a_facts.rs"]
mod tb02a_facts;
#[path = "s_session_v2/tb02b.rs"]
mod tb02b;
#[path = "s_session_v2/tb02c.rs"]
mod tb02c;
use std::path::{Path, PathBuf};

#[cfg(test)]
use newchan_rust::session_protocol::is_canonical_integer;
use newchan_rust::session_protocol::{canonical_json, parse_canonical_i64, sha256_hex};
use newchan_rust::theta_v0::classifier::local_shape::{
    classify_local_shape_sliding, window_domain_violation, Cc006DomainViolation, Cc006LocalShape,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{self, ParseLayerIncr};
use newchan_rust::theta_v0::types::{Bar, Direction};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};

/// CC-006 验收合同修订（SPEC-COVERAGE-INPUT.json `/classification_axes/5`）。
///
/// v2 结构写者（本文件 `init_core`/`accept_core`/`advance_core`）的规则修订：它发布 TB-02-A…D。
const RULE_REVISION: &str = "s2-axis-quantifiers+new-bi/1+segment-second/1";

/// #1394 整合 W-2：`s-session/3`（v3）写者的规则修订。v3 只发布 TB-02-A/B 对象（包含、分型、
/// 新笔），不声明 `segment-second`；它冻结在 FIN 线与 S86 r6 库所用的值，新二进制因此仍能
/// 接收这些库，v3 对象身份配方里的规则修订也不变。
///
/// 两个常量对应两个规则修订、各配自己发布的对象集与目录轴表
/// （[`tb02a_facts::CatalogAxisTable`]），由各写者按自己的修订选定，不是同一判断的宽严两档
/// （#799）。退场条件：v3 承载 TB-02-C/D 的新票落地即与 [`RULE_REVISION`] 合并为一个常量。
const V3_RULE_REVISION: &str = "s2-axis-quantifiers+new-bi/1";

/// S 唯一结构写者的默认 writer_epoch（可经 `--writer-epoch` / `recover` 换代）。
const DEFAULT_WRITER_EPOCH: &str = "1";

fn jstr(s: &str) -> String {
    Value::String(s.to_string()).to_string()
}

/// Begin 唯一推进 token（单写者 CAS 提交条件用）。
fn make_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}:{nanos}", std::process::id())
}

// ────────────────────────────────────────────────────────────────────────────
// 规范十进制整数（AC5：唯一表示，不静默截断/不接浮点）
// ────────────────────────────────────────────────────────────────────────────

/// #1371：源位置是规范非负精确整数；不要求从零起或连续，允许超过 JS 安全整数。
fn parse_source_coord(s: &str) -> Result<i64, String> {
    let coord = parse_canonical_i64(s, "source_coord")?;
    usize::try_from(coord)
        .map_err(|_| format!("source_coord `{s}` 不是本平台可无损表示的非负源位置"))?;
    Ok(coord)
}

fn stored_source_coord(event: &Value) -> Result<i64, String> {
    let raw = event["source_coord"]
        .as_str()
        .ok_or("StorageUnavailable：raw.source_coord 不是字符串")?;
    parse_source_coord(raw).map_err(|e| format!("StorageUnavailable：{e}"))
}

// ────────────────────────────────────────────────────────────────────────────
// 原始输入 schema（AC2）
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
struct RawEvent {
    event_id: String,
    revision: String,
    seq: String,
    received_at: String,
    raw_text: String,
    price: String,
    timestamp: String,
    volume: String,
    #[serde(skip)]
    ohlc: Option<tb02a::Ohlc>,
}

#[derive(Debug, serde::Deserialize)]
struct RawInputFile {
    schema_revision: String,
    #[serde(default)]
    session_id: String,
    source_namespace: String,
    source_epoch: String,
    instrument: String,
    #[serde(default)]
    profile: String,
    events: Vec<RawEvent>,
}

/// 规范事件内容字节（payload_hash 绑定规范字节；固定键序，含事件自身字段）。
fn canonical_event_content(e: &RawEvent) -> String {
    if let Some(prices) = &e.ohlc {
        return tb02a::canonical(e, prices);
    }
    format!(
        r#"{{"event_id":{},"price":{},"raw_text":{},"received_at":{},"revision":{},"seq":{},"timestamp":{},"volume":{}}}"#,
        jstr(&e.event_id),
        jstr(&e.price),
        jstr(&e.raw_text),
        jstr(&e.received_at),
        jstr(&e.revision),
        jstr(&e.seq),
        jstr(&e.timestamp),
        jstr(&e.volume),
    )
}

/// 业务身份键（同身份重放/冲突判据）：`(namespace, epoch, instrument, event_id)`（不含 revision）。
fn identity_key(ns: &str, epoch: &str, instr: &str, event_id: &str) -> String {
    format!("{ns}|{epoch}|{instr}|{event_id}")
}

// ────────────────────────────────────────────────────────────────────────────
// 数据库初始化（C09.2：WAL + synchronous=FULL + 平台前件核）
// ────────────────────────────────────────────────────────────────────────────

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS raw_events (
  identity_key TEXT NOT NULL,
  revision INTEGER NOT NULL,
  input_revision TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  receipt_id TEXT NOT NULL,
  seq INTEGER NOT NULL UNIQUE,
  source_namespace TEXT NOT NULL,
  source_epoch TEXT NOT NULL,
  instrument TEXT NOT NULL,
  event_id TEXT NOT NULL,
  received_at TEXT NOT NULL,
  raw_text TEXT NOT NULL,
  price TEXT NOT NULL,
  ts TEXT NOT NULL,
  volume TEXT NOT NULL,
  source_coord TEXT NOT NULL,
  supersedes_revision INTEGER,
  PRIMARY KEY (identity_key, revision)
);
CREATE TABLE IF NOT EXISTS batches (
  batch_id TEXT PRIMARY KEY,
  canonical_bytes BLOB NOT NULL,
  byte_len INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS objects (
  object_id TEXT PRIMARY KEY,
  object_revision INTEGER NOT NULL,
  kind TEXT NOT NULL,
  batch_id TEXT NOT NULL,
  branch TEXT NOT NULL,
  dir_ab TEXT NOT NULL,
  dir_bc TEXT NOT NULL,
  window_start INTEGER NOT NULL,
  window_mid INTEGER NOT NULL,
  window_end INTEGER NOT NULL,
  comparisons_json TEXT NOT NULL,
  input_refs_json TEXT NOT NULL,
  source_coords_json TEXT NOT NULL,
  first_known_generation INTEGER NOT NULL,
  first_known_cut TEXT NOT NULL,
  published_generation INTEGER NOT NULL,
  withdrawn_generation INTEGER,
  withdrawal_reason TEXT,
  superseded_by TEXT
);
CREATE TABLE IF NOT EXISTS witnesses (
  witness_id TEXT PRIMARY KEY,
  object_id TEXT NOT NULL,
  slot INTEGER NOT NULL,
  merged_source_index INTEGER NOT NULL,
  merged_high TEXT NOT NULL,
  merged_low TEXT NOT NULL,
  merged_open TEXT NOT NULL,
  merged_close TEXT NOT NULL,
  raw_json TEXT NOT NULL,
  published_generation INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS relations (
  relation_id TEXT PRIMARY KEY,
  subject TEXT NOT NULL,
  relation_type TEXT NOT NULL,
  object TEXT NOT NULL,
  published_generation INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS observations (
  observation_id TEXT PRIMARY KEY,
  batch_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  window_start INTEGER,
  window_mid INTEGER,
  window_end INTEGER,
  reason TEXT NOT NULL,
  detail_json TEXT NOT NULL,
  published_generation INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS structure_deltas (
  generation INTEGER PRIMARY KEY,
  session_id TEXT NOT NULL,
  catalog_revision TEXT NOT NULL,
  base_cut TEXT NOT NULL,
  next_cut TEXT NOT NULL,
  seq_range_json TEXT NOT NULL,
  index_frontier TEXT NOT NULL,
  input_frontier INTEGER NOT NULL,
  catalog_run_status TEXT NOT NULL,
  catalog_evidence_json TEXT NOT NULL,
  delta_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS writer_epoch_history (
  ordinal INTEGER PRIMARY KEY,
  from_epoch TEXT NOT NULL,
  to_epoch TEXT NOT NULL,
  generation_at_transition TEXT NOT NULL,
  advance_state_at_transition TEXT NOT NULL,
  transitioned_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS catalog (
  catalog_id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  domain TEXT NOT NULL,
  branches_json TEXT NOT NULL,
  impl_status TEXT NOT NULL,
  proof_status TEXT NOT NULL,
  run_status TEXT NOT NULL,
  evidence_json TEXT NOT NULL
);
"#;

fn open_db(db: &Path) -> Result<Connection, String> {
    v3::admission::require_current(db)?;
    let conn = Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::default() | rusqlite::OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
    )
    .map_err(|e| format!("打开 SQLite 失败：{e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
        .map_err(|e| format!("设置 WAL/FULL 失败：{e}"))?;
    #[cfg(target_os = "macos")]
    conn.execute_batch("PRAGMA fullfsync=ON;")
        .map_err(|e| format!("设置 fullfsync 失败：{e}"))?;
    Ok(conn)
}

fn db_meta(conn: &Connection) -> Result<BTreeMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM meta")
        .map_err(|e| format!("读取 meta 失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| format!("查询 meta 失败：{e}"))?;
    rows.collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|e| format!("meta 字段损坏：{e}"))
}

fn meta_get_opt(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .map_err(|e| format!("读 meta.{key} 失败：{e}"))
}

fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("写 meta.{key} 失败：{e}"))?;
    Ok(())
}

/// writer epoch 的唯一整数域（#1371）：持久值、授予值和恢复目标共用，不接受负数或非规范表示。
fn parse_writer_epoch(raw: &str, what: &str) -> Result<i64, String> {
    let epoch = parse_canonical_i64(raw, what)?;
    if epoch < 0 {
        return Err(format!("{what} 必须为非负整数"));
    }
    Ok(epoch)
}

/// 核持久 writer_epoch（DUR-H02）：configured 是本进程被授予的 epoch，不能读库内值自授权。
fn verify_writer_epoch(conn: &Connection, configured: &str) -> Result<(), String> {
    let epoch = meta_get_opt(conn, "writer_epoch")?.unwrap_or_default();
    let stored = parse_writer_epoch(&epoch, "meta.writer_epoch")
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let granted = parse_writer_epoch(configured, "--writer-epoch")
        .map_err(|e| format!("InvalidDomain：{e}"))?;
    if stored != granted {
        return Err(format!(
            "StaleWriter：writer_epoch=`{epoch}`，非本进程被授予的 `{configured}`（换代后旧 writer 拒绝，零写入）"
        ));
    }
    Ok(())
}

/// 核已发布可达根（index_frontier 指向的不可变 batch 必须仍存在）。所有正式写入入口（accept/advance/
/// recover）在同一序边界核此前件：自身存储坏（悬空根）不得被当作成功或用新 cut 掩盖。
/// 历史入参：预算由 `cache::configure` 的**单一预算**唯一决定，容量满只失去复用，不改判据。
const EVIDENCE_READ_BUDGET: usize = 268_435_456;

/// 整块域的句柄指纹：只由当前读到的行内原字节决定（与图域闭包指纹域分离，两者不得互相命中）。
fn blob_digest(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"batch-blob/1");
    h.update(bytes);
    h.finalize().into()
}

/// #1394：批次规范字节的**唯一**读取入口。
///
/// 物理域由私有登记表唯一决定（旧整块 / 新节点图），两者都按同一内容寻址判据核验；
/// 不提供「一种失败再试另一种」的降级通道：图域见到整块字节即拒绝，整块域见不到字节即拒绝。
fn load_batch_bytes(conn: &Connection, batch_id: &str) -> Result<std::sync::Arc<[u8]>, String> {
    let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
    load_batch_with(conn, &mut reader, batch_id, false).map(|(bytes, _, _)| bytes)
}

fn load_batch_bytes_with(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    batch_id: &str,
) -> Result<std::sync::Arc<[u8]>, String> {
    load_batch_with(conn, reader, batch_id, false).map(|(bytes, _, _)| bytes)
}

/// Arc内联owner和控制块各自随最后共享者析构；不借scope保守记账代替所有权。
struct SharedHeaderText {
    text: TextValue,
    _permit: Option<newchan_rust::s_content::capture_allocation::scratch::NativeReservation>,
}
fn share_header_text_admitted(
    text: &str,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<std::sync::Arc<SharedHeaderText>, String> {
    use v3::audit_store::{CodecRead, NativeAdmission};
    let permit = work
        .map(|work| {
            work.reserve_native(
                std::mem::size_of::<SharedHeaderText>() + 2 * std::mem::size_of::<usize>(),
            )
        })
        .transpose()
        .map_err(|e| e.to_string())?;
    let text = NativeAdmission(work)
        .native_text(text)
        .map_err(|e| e.to_string())?;
    Ok(std::sync::Arc::new(SharedHeaderText {
        text,
        _permit: permit,
    }))
}
impl SharedHeaderText {
    fn as_str(&self) -> &str {
        self.text.as_str()
    }
}
impl std::ops::Deref for SharedHeaderText {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Debug for SharedHeaderText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl PartialEq for SharedHeaderText {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for SharedHeaderText {}
struct SharedPlainJson {
    value: PlainJson,
    _permit: Option<newchan_rust::s_content::capture_allocation::scratch::NativeReservation>,
}
impl SharedPlainJson {
    fn as_ref(&self) -> &Value {
        self.value.as_ref()
    }
}
impl std::ops::Deref for SharedPlainJson {
    type Target = Value;
    fn deref(&self) -> &Value {
        self.as_ref()
    }
}
impl std::fmt::Debug for SharedPlainJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}
fn share_plain_json_admitted(
    value: PlainJson,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<std::sync::Arc<SharedPlainJson>, String> {
    let permit = work
        .map(|work| {
            work.reserve_native(
                std::mem::size_of::<SharedPlainJson>() + 2 * std::mem::size_of::<usize>(),
            )
        })
        .transpose()
        .map_err(|e| e.to_string())?;
    Ok(std::sync::Arc::new(SharedPlainJson {
        value,
        _permit: permit,
    }))
}

/// 只从已完整严格解析且公开内容身份核验通过的 batch 投影；不补字段、不从控制表反推。
/// 旧 v1 忽略这四字段；v2 对每个 Some 字符串继续逐字比较，缺失/非字符串均不能通过。
#[derive(Clone, Debug, PartialEq, Eq)]
struct BatchControlHeader {
    protocol_revision: Option<std::sync::Arc<SharedHeaderText>>,
    session_generation: Option<std::sync::Arc<SharedHeaderText>>,
    clock_plan_hash: Option<std::sync::Arc<SharedHeaderText>>,
    semantic_commit_ns: Option<std::sync::Arc<SharedHeaderText>>,
}
impl BatchControlHeader {
    fn from_verified_batch(batch: &Value) -> Self {
        Self::from_verified_batch_admitted(batch, None).expect("None admission cannot fail")
    }
    fn from_verified_batch_admitted(
        batch: &Value,
        work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
    ) -> Result<Self, String> {
        let string = |key| {
            batch
                .get(key)
                .and_then(Value::as_str)
                .map(|text| share_header_text_admitted(text, work))
                .transpose()
        };
        Ok(Self {
            protocol_revision: string("protocol_revision")?,
            session_generation: string("session_generation")?,
            clock_plan_hash: string("clock_plan_hash")?,
            semantic_commit_ns: string("semantic_commit_ns")?,
        })
    }
    fn source_bytes(&self) -> usize {
        [
            &self.protocol_revision,
            &self.session_generation,
            &self.clock_plan_hash,
            &self.semantic_commit_ns,
        ]
        .iter()
        .fold(4usize, |n, value| {
            n.saturating_add(value.as_ref().map_or(0, |v| v.len()))
        })
    }
}

fn batch_content_hash(batch_id: &str) -> Result<&str, String> {
    match batch_id.strip_prefix("batch-") {
        Some(h) if h.len() == 64 => Ok(h),
        _ => Err(format!(
            "StorageUnavailable：batch_id `{batch_id}` 非规范内容寻址形态"
        )),
    }
}

/// 全文/控制头共用同一条当前批次行与公开身份读取门。
fn read_batch_row(conn: &Connection, batch_id: &str) -> Result<(Vec<u8>, usize), String> {
    let (blob, len) = read_batch_row_with_owner(conn, batch_id, None)?;
    Ok((blob.try_into_plain().map_err(|e| e.to_string())?, len))
}
fn read_batch_row_with_owner(
    conn: &Connection,
    batch_id: &str,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
) -> Result<(newchan_rust::s_content::CodecVec<u8>, usize), String> {
    read_batch_row_admitted(conn, batch_id, owner, None)
}
fn read_batch_row_admitted(
    conn: &Connection,
    batch_id: &str,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<(newchan_rust::s_content::CodecVec<u8>, usize), String> {
    use v3::audit_store::{CodecRead, NativeAdmission};
    let err = |e| {
        format!(
            "读 batch {batch_id} 失败：{}",
            evidence_store::sql_columns(e, &["canonical_bytes", "byte_len"])
        )
    };
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        conn,
        c"SELECT canonical_bytes, byte_len FROM batches WHERE batch_id=?1",
    )
    .map_err(err)?;
    stmt.bind_text(1, batch_id).map_err(err)?;
    let mut row = stmt.next().map_err(err)?.ok_or_else(|| {
        format!("StorageUnavailable：可达 batch `{batch_id}` 不存在（引用不完整，拒绝写入）")
    })?;
    let mut blob = v3::business_json::new_work(owner);
    let source = row.blob(0).map_err(err)?;
    NativeAdmission(work)
        .native_codec_room(&mut blob, source.len())
        .map_err(|e| e.to_string())?;
    blob.try_extend_bytes(source).map_err(|e| e.to_string())?;
    let byte_len = row.integer(1).map_err(err)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    batch_content_hash(batch_id)?;
    let byte_len = usize::try_from(byte_len)
        .map_err(|_| format!("StorageUnavailable：batch `{batch_id}` byte_len 非负越界"))?;
    Ok((blob, byte_len))
}
/// 行、公开身份与物理域的共用判定门；Owned不从失败切回普通读取。
fn read_batch_storage(
    conn: &Connection,
    batch_id: &str,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
) -> Result<(newchan_rust::s_content::CodecVec<u8>, usize, bool), String> {
    read_batch_storage_admitted(conn, batch_id, owner, None)
}
fn read_batch_storage_admitted(
    conn: &Connection,
    batch_id: &str,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<(newchan_rust::s_content::CodecVec<u8>, usize, bool), String> {
    let (blob, byte_len) = read_batch_row_admitted(conn, batch_id, owner, work)?;
    let expect_hash = batch_content_hash(batch_id)?;
    let graph = match owner {
        Some(_) => evidence_store::read_format_admitted(conn, owner, work)?,
        None => evidence_store::format_admitted(conn, work)?,
    }
    .graph();
    if !graph {
        if blob.is_empty()
            || blob.len() != byte_len
            || !evidence_store::hash_matches(&blob, expect_hash)
        {
            return Err(format!("StorageUnavailable：batch `{batch_id}` 规范字节/长度与内容寻址不符（坏 bytes/byte_len），拒绝写入"));
        }
    } else if !blob.is_empty() {
        return Err(format!(
            "StorageUnavailable：图域 batch `{batch_id}` 仍带整块字节（与私有根混写），拒绝写入"
        ));
    }
    Ok((blob, byte_len, graph))
}
enum ProfileBatchBytes {
    Plain(std::sync::Arc<[u8]>),
    Owned(newchan_rust::s_content::CodecVec<u8>),
}
impl std::ops::Deref for ProfileBatchBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Plain(b) => b,
            Self::Owned(b) => b,
        }
    }
}
fn load_profile_batch_bytes(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    batch_id: &str,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
) -> Result<ProfileBatchBytes, String> {
    let Some(owner) = owner else {
        return load_batch_bytes_with(conn, reader, batch_id).map(ProfileBatchBytes::Plain);
    };
    #[cfg(test)]
    cache::counter(|c| c.batch_full_reads += 1);
    let (blob, byte_len, graph) = read_batch_storage(conn, batch_id, Some(owner))?;
    if !graph {
        return Ok(ProfileBatchBytes::Owned(blob));
    }
    let root_key = evidence_store::batch_root_key_with_owner(batch_id, Some(owner))?;
    let root = reader.read_root_owned(root_key.as_str())?;
    #[cfg(test)]
    cache::counter(|c| c.batch_expansions += 1);
    verify_batch_root_binding(
        batch_id,
        byte_len,
        root.declared_length,
        root.content_sha256.as_str(),
    )?;
    Ok(ProfileBatchBytes::Owned(root.bytes))
}

fn verify_batch_root_binding(
    batch_id: &str,
    byte_len: usize,
    declared_length: u64,
    hash: &str,
) -> Result<(), String> {
    if declared_length as usize != byte_len {
        return Err(format!(
            "StorageUnavailable：图域 batch `{batch_id}` 根展开长度与 byte_len 不符"
        ));
    }
    if hash != batch_content_hash(batch_id)? {
        return Err(format!(
            "StorageUnavailable：batch `{batch_id}` 私有根展开内容与公开身份不符，拒绝写入"
        ));
    }
    Ok(())
}

/// 捕获状态不冒充已验控制头；PendingFull 必须在本次完整根审计中解决。
#[derive(Clone)]
enum BatchRef {
    CertifiedHeader(BatchControlHeader),
    PendingFull,
}

fn capture_batch_ref_with(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    batch_id: &str,
) -> Result<(BatchRef, Option<std::sync::Arc<cache::RootCert>>), String> {
    if evidence_store::format(conn)?.graph() {
        let (blob, byte_len) = read_batch_row(conn, batch_id)?;
        if !blob.is_empty() {
            return Err(format!("StorageUnavailable：图域 batch `{batch_id}` 仍带整块字节（与私有根混写），拒绝写入"));
        }
        let key = evidence_store::batch_root_key(batch_id);
        if let Some(proof) = reader.prove_root(&key)? {
            verify_batch_root_binding(
                batch_id,
                byte_len,
                proof.declared_length,
                &proof.content_sha256,
            )?;
            if let Some(header) =
                cache::batch_header_get(&key, batch_id, byte_len, &proof.root_cert, &proof.closure)
            {
                return Ok((BatchRef::CertifiedHeader(header), Some(proof.root_cert)));
            }
            return Ok((BatchRef::PendingFull, Some(proof.root_cert)));
        }
    }
    // 缺缓存不等于核验成功；只延后全文到逐代控制/结构共用的同一份读取。
    Ok((BatchRef::PendingFull, None))
}

/// 批次规范字节的**唯一**读取内核：整块域取行内字节，图域由私有根无损展开（可带解码值）。
///
/// 图域的公开身份绑定是**廉价且完整**的：根行 `content_sha256` 在完整展开时与展开字节逐字节
/// 对拍过，而闭包指纹又把当前 `content_sha256` 钉死，故 `content_sha256 == batch-<hex>` 与
/// `sha256(展开字节) == batch-<hex>` 等价；命中已核句柄时不必重新哈希全部展开字节。
fn load_batch_with(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    batch_id: &str,
    with_value: bool,
) -> Result<
    (
        std::sync::Arc<[u8]>,
        Option<std::sync::Arc<SharedPlainJson>>,
        Option<std::sync::Arc<cache::RootCert>>,
    ),
    String,
> {
    let work = reader.work();
    #[cfg(test)]
    cache::counter(|c| c.batch_full_reads += 1);
    let (blob, byte_len, graph) = read_batch_storage_admitted(conn, batch_id, None, work)?;
    let blob = blob.try_into_plain().map_err(|e| e.to_string())?;
    if !graph {
        // 旧整块域：完整规范字节就在这一行。
        evidence_store::admit_byte_arc(blob.len(), work)?;
        let bytes: std::sync::Arc<[u8]> = std::sync::Arc::from(blob.as_slice());
        let digest = blob_digest(&blob);
        // 整块域也走同一句柄缓存：命中要求当前行内字节与此前核过的完全一致。
        let key = evidence_store::batch_root_key_admitted(batch_id, None, work)?;
        let value = if with_value {
            match cache::handle_get_blob(&key, &blob, &digest).and_then(|(_, v)| v) {
                Some(value) => Some(value),
                None => {
                    let parsed = parse_batch_value_admitted(&bytes, batch_id, work)?;
                    cache::handle_put_blob_admitted(
                        &key,
                        digest,
                        bytes.clone(),
                        Some(parsed.clone()),
                        work,
                    )?;
                    Some(parsed)
                }
            }
        } else {
            cache::handle_put_blob_admitted(&key, digest, bytes.clone(), None, work)?;
            None
        };
        return Ok((bytes, value, None));
    }
    // 图域：共用物理域门已拒绝整块混写。
    let root_key = evidence_store::batch_root_key_admitted(batch_id, None, work)?;
    let root = reader.read_root(&root_key, with_value)?;
    #[cfg(test)]
    cache::counter(|c| {
        if root.reused {
            c.batch_handle_hits += 1;
        } else {
            c.batch_expansions += 1;
        }
    });
    verify_batch_root_binding(
        batch_id,
        byte_len,
        root.declared_length,
        &root.content_sha256,
    )?;
    if let (Some(value), Some(cert)) = (&root.value, &root.root_cert) {
        cache::batch_header_put_admitted(
            &root_key,
            batch_id,
            byte_len,
            cert,
            root.closure,
            BatchControlHeader::from_verified_batch_admitted(value, work)?,
            work,
        )?;
    }
    Ok((root.bytes, root.value, root.root_cert))
}

fn parse_batch_value(
    bytes: &[u8],
    batch_id: &str,
) -> Result<std::sync::Arc<SharedPlainJson>, String> {
    parse_batch_value_admitted(bytes, batch_id, None)
}
fn parse_batch_value_admitted(
    bytes: &[u8],
    batch_id: &str,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<std::sync::Arc<SharedPlainJson>, String> {
    use v3::audit_store::{CodecRead, NativeAdmission};
    let parsed = v3::business_json::strict_json_owner_admitted(bytes, None, work)
        .map(|parsed| match parsed {
            v3::business_json::JsonOwner::Plain(value) => value,
            #[cfg(feature = "s_allocation_json_testonly")]
            v3::business_json::JsonOwner::Owned(_) => {
                unreachable!("ordinary batch keeps plain JSON")
            }
        })
        .map_err(|e| match e {
            v3::business_json::StrictOwnerError::Syntax(cause) => {
                format!("StorageUnavailable：batch {batch_id} 规范字节非严格 JSON：{cause}")
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            v3::business_json::StrictOwnerError::OwnedSyntax(cause) => {
                format!("StorageUnavailable：batch {batch_id} 规范字节非严格 JSON：{cause}")
            }
            v3::business_json::StrictOwnerError::Allocation(cause) => cause.to_string(),
        })?;
    if !parsed.is_object() {
        return Err(format!(
            "StorageUnavailable：batch {batch_id} 规范字节非 JSON 对象"
        ));
    }
    share_plain_json_admitted(parsed, work)
}

/// #1394 第二段：逻辑记录大原件的**唯一**读取入口（Delta 原件与结构事实三列原件共用）。
///
/// - 无登记表的旧整块域与 `evidence-graph/2`：原件就是行内列的原文，逐字节不动；
/// - `evidence-graph/3`：行内列必须是空占位，完整原件由该逻辑记录的私有根无损展开；
///   不提供「一种失败再试另一种」的降级通道（列非空即拒绝，根缺失即拒绝）。
fn read_record_original(
    _conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    format: evidence_store::Format,
    root_key: &str,
    column: &str,
    label: &str,
) -> Result<std::sync::Arc<[u8]>, String> {
    if !format.records_in_graph() {
        return Ok(std::sync::Arc::from(column.as_bytes()));
    }
    if !column.is_empty() {
        return Err(format!(
            "StorageUnavailable：图域 {label} 行内仍带整块原文（与私有根混写），拒绝"
        ));
    }
    Ok(reader.read_fixed_cut(root_key)?)
}

/// 同上的文本形态（Delta 原件与结构事实三列都是 JSON 文本列）。
fn read_record_original_text(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    format: evidence_store::Format,
    root_key: &str,
    column: &str,
    label: &str,
) -> Result<String, String> {
    let bytes = read_record_original(conn, reader, format, root_key, column, label)?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| format!("StorageUnavailable：{label} 非 UTF-8：{e}"))
}

fn load_batch_value_with(
    conn: &Connection,
    reader: &mut evidence_store::Reader<'_>,
    batch_id: &str,
) -> Result<
    (
        std::sync::Arc<SharedPlainJson>,
        Option<std::sync::Arc<cache::RootCert>>,
    ),
    String,
> {
    let (_, value, root_cert) = load_batch_with(conn, reader, batch_id, true)?;
    let value = value.ok_or("StorageUnavailable：batch 解码值缺失（实现缺陷）")?;
    Ok((value, root_cert))
}

/// 核已发布可达根与全部历史可达引用（ROOT-B-R2-01）：
/// - generation==0（真正初态）⟹ index_frontier 必须空且无 delta 行；
/// - generation>=1 ⟹ meta.index_frontier 必须存在且非空、指向内容完整的 batch，并与
///   structure_deltas[generation].index_frontier 一致；每条已发布 delta 的 index_frontier 也
///   必须指向内容完整的 batch。孤儿 batch（after_batch 未提交）不在此列，不算坏根。
fn meta_i64(conn: &Connection, key: &str) -> Result<i64, String> {
    let v =
        meta_get_opt(conn, key)?.ok_or_else(|| format!("StorageUnavailable：meta.{key} 缺失"))?;
    parse_canonical_i64(&v, &format!("meta.{key}")).map_err(|e| format!("StorageUnavailable：{e}"))
}

/// 必需 meta 的存在/规范整数/域与同 cut 对应关系校验（所有读/写入口共享；坏缺值不得
/// unwrap_or(-1)/空串成功兜底）。返回 generation。
fn verify_required_meta(conn: &Connection) -> Result<i64, String> {
    let session_id =
        meta_get_opt(conn, "session_id")?.ok_or("StorageUnavailable：meta.session_id 缺失")?;
    if session_id.is_empty() {
        return Err("StorageUnavailable：meta.session_id 为空".to_string());
    }
    let epoch = meta_get_opt(conn, "writer_epoch")?.ok_or("StorageUnavailable：缺 writer_epoch")?;
    parse_writer_epoch(&epoch, "持久 writer_epoch")
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let generation = meta_i64(conn, "generation")?;
    if generation < 0 {
        return Err("StorageUnavailable：meta.generation 为负".to_string());
    }
    let cut = meta_get_opt(conn, "structure_cut")?.unwrap_or_default();
    if cut.is_empty() {
        return Err("StorageUnavailable：meta.structure_cut 缺失或为空".to_string());
    }
    let expect_cut = format!("cut-{generation}");
    if cut != expect_cut {
        return Err(format!(
            "StorageUnavailable：meta.structure_cut=`{cut}` 与 generation={generation} 不一致（应 `{expect_cut}`）"
        ));
    }
    let cat = meta_get_opt(conn, "catalog_revision")?.unwrap_or_default();
    if cat.is_empty() {
        return Err("StorageUnavailable：meta.catalog_revision 缺失或为空".to_string());
    }
    let idx = meta_get_opt(conn, "index_frontier")?.unwrap_or_default();
    let published_frontier = meta_i64(conn, "last_advance_frontier")?;
    if published_frontier < -1 {
        return Err(format!(
            "StorageUnavailable：meta.last_advance_frontier=`{published_frontier}` 越域（只允许 -1 或非负）"
        ));
    }
    // last_advance_frontier 必须与当前已发布 Delta 的 input_frontier 一致（不得由新 advance 掩盖坏值）。
    if generation > 0 {
        if idx.is_empty() {
            return Err(format!(
                "StorageUnavailable：generation={generation} 已发布却 index_frontier 为空（缺根 meta）"
            ));
        }
        let delta_frontier: Option<i64> = conn
            .query_row(
                "SELECT input_frontier FROM structure_deltas WHERE generation=?1",
                params![generation],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("读 delta[{generation}].input_frontier 失败：{e}"))?;
        match delta_frontier {
            None => {
                return Err(format!(
                    "StorageUnavailable：generation={generation} 无对应持久 Delta（根/历史引用断裂）"
                ))
            }
            Some(df) if df != published_frontier => {
                return Err(format!(
                    "StorageUnavailable：meta.last_advance_frontier=`{published_frontier}` 与 delta[{generation}].input_frontier=`{df}` 不一致"
                ))
            }
            Some(_) => {}
        }
    } else {
        if !idx.is_empty() {
            return Err(format!(
                "StorageUnavailable：初态 generation=0 却存在 index_frontier=`{idx}`（不一致）"
            ));
        }
        if published_frontier != -1 {
            return Err(format!(
                "StorageUnavailable：初态 generation=0 却 last_advance_frontier=`{published_frontier}`（应 -1）"
            ));
        }
    }
    read_scope(conn)?;
    Ok(generation)
}

/// 核被引用 batch 内封存的坐标与该代 Delta 一致（batch 存 generation/structure_cut/input_frontier/
/// catalog_revision；不要求 batch 内不存在字段如 session_id）。
fn verify_batch_coordinates(
    v: &Value,
    batch_id: &str,
    gen: i64,
    cut: &str,
    frontier: i64,
    cat: &str,
) -> Result<(), String> {
    if v["generation"].as_i64() != Some(gen) {
        return Err(format!(
            "StorageUnavailable：batch `{batch_id}` 封存 generation 与该代 Delta 不一致"
        ));
    }
    if v["structure_cut"].as_str() != Some(cut) {
        return Err(format!(
            "StorageUnavailable：batch `{batch_id}` 封存 structure_cut 与该代 Delta 不一致"
        ));
    }
    if v["input_frontier"].as_i64() != Some(frontier) {
        return Err(format!(
            "StorageUnavailable：batch `{batch_id}` 封存 input_frontier 与该代 Delta 不一致"
        ));
    }
    if v["catalog_revision"].as_str() != Some(cat) {
        return Err(format!(
            "StorageUnavailable：batch `{batch_id}` 封存 catalog_revision 与该代 Delta 不一致"
        ));
    }
    Ok(())
}

// #1371 R9：scope 是本片已初始化 S 的必需身份，不是缺省配置。
fn validate_scope(scope: &Value) -> Result<(), String> {
    if scope != &json!({"structure": "CompleteCut", "economic": "not_started"}) {
        return Err("StorageUnavailable：scope 与本片结构/经济未启动声明不符".to_string());
    }
    Ok(())
}

fn required_array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("StorageUnavailable：必需集合 {key} 缺失或非数组"))
}

fn legacy_payload_projection(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let projected = if k == "detail" {
                        project_wire_integers(v)
                    } else if k == "raw_bars" {
                        match v.as_array() {
                            Some(bars) => Value::Array(
                                bars.iter()
                                    .map(|bar| {
                                        let mut bar = bar.clone();
                                        if let Some(sup) = bar.get_mut("supersedes_revision") {
                                            *sup = project_wire_integers(sup);
                                        }
                                        bar
                                    })
                                    .collect(),
                            ),
                            None => v.clone(),
                        }
                    } else {
                        legacy_payload_projection(v)
                    };
                    (k.clone(), projected)
                })
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(legacy_payload_projection).collect()),
        _ => value.clone(),
    }
}

/// 借用记录，只省去调用方的临时 Value 数组；投影、完整规范字符串、排序与查重逐项保留。
fn normalize_record_iter<'a>(
    records: impl IntoIterator<Item = &'a Value>,
    field: &str,
) -> Result<Vec<String>, String> {
    let mut records = records
        .into_iter()
        .map(|r| {
            if !r.is_object() {
                return Err("StorageUnavailable：集合成员非对象".to_string());
            }
            Ok(canonical_json(&legacy_payload_projection(r)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    records.sort();
    if records.windows(2).any(|w| w[0] == w[1]) {
        return Err(format!("StorageUnavailable：{field} 重复成员"));
    }
    Ok(records)
}

fn normalize_record_array(records: &Value, field: &str) -> Result<Vec<String>, String> {
    normalize_record_iter(
        records.as_array().ok_or("StorageUnavailable：集合非数组")?,
        field,
    )
}

/// 两侧必须已完整规范化并查重；比较全部字符串，不使用 Value 相等或摘要替代。
fn compare_normalized_records(
    actual: &[String],
    expected: &[String],
    field: &str,
) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "StorageUnavailable：{field} 与封存批次/同 cut 持久快照不一致"
        ));
    }
    Ok(())
}

fn same_record_iters<'a, 'b>(
    actual: impl IntoIterator<Item = &'a Value>,
    expected: impl IntoIterator<Item = &'b Value>,
    field: &str,
) -> Result<(), String> {
    let actual = normalize_record_iter(actual, field)?;
    let expected = normalize_record_iter(expected, field)?;
    compare_normalized_records(&actual, &expected, field)
}

fn same_records_against_iter<'a>(
    actual: &Value,
    expected: impl IntoIterator<Item = &'a Value>,
    field: &str,
) -> Result<(), String> {
    let actual = normalize_record_array(actual, field)?;
    let expected = normalize_record_iter(expected, field)?;
    compare_normalized_records(&actual, &expected, field)
}

fn same_records(actual: &Value, expected: &Value, field: &str) -> Result<(), String> {
    // 必须先完成 actual 的全部规范化/查重，再检查 expected 的数组形状和内容。
    let actual = normalize_record_array(actual, field)?;
    let expected = normalize_record_array(expected, field)?;
    compare_normalized_records(&actual, &expected, field)
}

// #1371 R9：七集合均为必需。以不可变 batch 的本次结果和独立历史索引对拍；
// 只读投影，不重算分类，不修改旧 BLOB/hash/获知史。旧整数 wire 仅作无损语义投影。
fn verify_delta_evidence(delta: &Value, batch: &Value) -> Result<(), String> {
    for key in [
        "upserts",
        "withdrawals",
        "replaces",
        "witnesses",
        "relations",
        "observations",
        "raw_history_added",
    ] {
        required_array(delta, key)?;
    }
    validate_scope(&batch["scope"])?;
    validate_scope(&delta["catalog_evidence"]["scope"])?;
    let evidence = &delta["catalog_evidence"];
    for key in ["profile_id", "profile_hash", "rule_revision"] {
        if evidence.get(key).is_none() || evidence[key] != batch[key] {
            return Err(format!(
                "StorageUnavailable：目录证据 {key} 与封存批次不一致"
            ));
        }
    }
    if evidence["batch_id"] != delta["index_frontier"]
        || evidence["structure_cut"] != delta["next_cut"]
    {
        return Err("StorageUnavailable：目录证据 batch/cut 不一致".to_string());
    }
    if batch["profile_id"] == tb02a::PROFILE
        && (batch["catalog_axes"].as_object().is_none()
            || evidence["axes"] != batch["catalog_axes"])
    {
        return Err("StorageUnavailable：逐轴目录证据与封存批次不一致".into());
    }
    if batch["profile_id"] == tb02a::PROFILE {
        let changes = &evidence["axes"]["CC-056"]["evidence"];
        same_records(
            &changes["withdrawals"],
            &delta["withdrawals"],
            "CC056撤回证据",
        )?;
        same_records(&changes["replaces"], &delta["replaces"], "CC056替代证据")?;
        let start = parse_canonical_i64(
            delta["seq_range"]["from"]
                .as_str()
                .ok_or("StorageUnavailable：Delta起始前沿缺失")?,
            "from",
        )?;
        let revisions: Vec<Value> = required_array(batch, "raw_events")?
            .iter()
            .filter(|r| {
                r["seq"].as_i64().is_some_and(|n| n >= start)
                    && r["revision"].as_i64().is_some_and(|n| n > 1)
            })
            .map(tb02a_facts::raw_ref)
            .collect();
        same_records(
            &changes["raw_revisions"],
            &json!(revisions),
            "CC056原始修订证据",
        )?;
    }
    let expected_run = if required_array(batch, "objects")?.is_empty() {
        "not_run"
    } else {
        "run"
    };
    if delta["catalog_run_status"] != expected_run {
        return Err("StorageUnavailable：目录运行状态与封存结果不一致".to_string());
    }
    for key in [
        "classified_objects",
        "windows_total",
        "merged_bars",
        "effective_source_positions",
        "withdrawals",
        "replaces",
        "insufficient_knowledge",
        "domain_not_satisfied",
    ] {
        let count = match evidence.get(key) {
            Some(Value::String(s)) => parse_canonical_i64(s, key).ok(),
            Some(Value::Number(n)) => n.as_i64(),
            _ => None,
        };
        if !matches!(count,Some(n) if n>=0) {
            return Err(format!("StorageUnavailable：目录计数 {key} 非非负精确整数"));
        }
    }
    Ok(())
}

fn observation_id(ob: &Value) -> String {
    if ob["detail"]["tb02a"] == true {
        return format!("obs-{}", sha256_hex(canonical_json(ob).as_bytes()));
    }
    format!(
        "obs-{}",
        &sha256_hex(&canonical_bytes_of_value(&json!({
            "kind": ob["kind"], "window_start": ob["window_start"], "window_mid": ob["window_mid"],
            "window_end": ob["window_end"], "reason": ob["reason"],
        })))[..16]
    )
}

// #1371 R10：输入profile是固定身份。未发布的已接纳输入也必须有独立内容绑定。
fn verify_profile_binding(conn: &Connection) -> Result<(String, String), String> {
    let (id, hash) = verify_profile_binding_with_owner(conn, None)?;
    match (id, hash) {
        (v3::business_json::TextValue::Plain(id), v3::business_json::TextValue::Plain(hash)) => {
            Ok((
                id.into_unscoped().map_err(|e| e.to_string())?,
                hash.into_unscoped().map_err(|e| e.to_string())?,
            ))
        }
        _ => Err("prototype_audit_allocation_incomplete：normal profile不可擦除Owned".into()),
    }
}
fn verify_profile_binding_with_owner(
    conn: &Connection,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
) -> Result<(v3::business_json::TextValue, v3::business_json::TextValue), String> {
    verify_profile_binding_admitted(conn, owner, None)
}
fn verify_profile_binding_admitted(
    conn: &Connection,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<(v3::business_json::TextValue, v3::business_json::TextValue), String> {
    use v3::audit_store::{CodecRead, NativeAdmission};
    use v3::business_json::TextValue;
    let admission = NativeAdmission(work);
    if work.is_some() && owner.is_some() {
        return Err("Native profile父域不得混入Owned存储".into());
    }

    let id = v2::control_meta_text_admitted(conn, "profile_id", owner, work)?;
    let hash = v2::control_meta_text_admitted(conn, "profile_hash", owner, work)?;
    let input = v2::control_meta_text_admitted(conn, "input_profile", owner, work)?;
    let definition = v2::control_meta_text_admitted(conn, "profile_definition", owner, work)?;
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if id.is_none() && hash.is_none() && input.is_none() && definition.is_none() && raw_count == 0 {
        return Ok((
            TextValue::from_str_admitted("", owner, work)?,
            TextValue::from_str_admitted("", owner, work)?,
        ));
    }
    let id = id
        .filter(|v| !v.is_empty())
        .ok_or("StorageUnavailable：已绑定profile_id缺失")?;
    let hash = hash
        .filter(|v| {
            v.len() == 64
                && v.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or("StorageUnavailable：已绑定profile_hash缺失或不规范")?;
    if input.as_deref() != Some(id.as_str()) {
        return Err("StorageUnavailable：input_profile与固定profile_id不一致".to_string());
    }
    if let Some(definition) = definition {
        let profile =
            json_shape_with_owner_admitted(definition.as_str(), JsonShape::Object, owner, work)?;
        let view = profile.view();
        if view.get("profile_id").and_then(|v| v.as_str()) != Some(id.as_str())
            || newchan_rust::session_protocol::canonical_hash_view_with_owner(view, owner)?.as_str()
                != hash.as_str()
        {
            return Err("StorageUnavailable：profile定义与固定id/hash不一致".to_string());
        }
    } else {
        // 旧版本没有profile_definition：只接受独立已封存来源；没有批次的旧TestOnly接纳
        // 以同一声明档案核规范hash。这是旧格式取证，不在失败后换宽松判据。
        // #1394：物理域可能是旧整块也可能是新节点图，故按已发布代序逐代展开选取首个带
        // profile 的可达批次；不再用 SQL 直接读整块列（图域该列为长度占位）。
        let mut stmt=newchan_rust::sqlite_checked::CheckedStatement::prepare(conn,c"SELECT b.batch_id FROM batches b JOIN structure_deltas d ON d.index_frontier=b.batch_id ORDER BY d.generation ASC").map_err(|e|e.to_string())?;
        let mut sealed_ids = v3::business_json::new_work(owner);
        while let Some(mut row) = stmt.next().map_err(|e| e.to_string())? {
            admission
                .native_codec_room(&mut sealed_ids, 1)
                .map_err(|e| e.to_string())?;
            sealed_ids
                .try_push(TextValue::from_sql_admitted(&mut row, 0, owner, work)?)
                .map_err(|e| e.to_string())?;
        }
        stmt.finalize().map_err(|e| e.to_string())?;
        let mut sealed_found = false;
        let mut reader = match owner {
            Some(b) => evidence_store::Reader::with_owner(conn, b),
            None => evidence_store::Reader::new_admitted(conn, EVIDENCE_READ_BUDGET, work)?,
        };
        for batch_id in &sealed_ids {
            let bytes = load_profile_batch_bytes(conn, &mut reader, batch_id, owner)?;
            // #1394：同一 Reader 显式携带 Native 工作域；旧 blob / 图源核仍走各自原判据。
            let batch = v3::business_json::legacy_json_owner_admitted(&bytes, owner, work)?;
            let batch = batch.view();
            if batch
                .get("profile_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                continue;
            }
            if batch.get("profile_id").and_then(|v| v.as_str()) != Some(id.as_str())
                || batch.get("profile_hash").and_then(|v| v.as_str()) != Some(hash.as_str())
            {
                return Err("StorageUnavailable：旧profile绑定与可达封存批次不一致".to_string());
            }
            sealed_found = true;
            break;
        }
        if !sealed_found {
            let declared = json_shape_with_owner_admitted(
                include_str!("../../../s_session/profiles/testonly_tick_1_1_ohlc.json"),
                JsonShape::Object,
                owner,
                work,
            )?;
            let declared = declared.view();
            if declared.get("profile_id").and_then(|v| v.as_str()) != Some(id.as_str())
                || newchan_rust::session_protocol::canonical_hash_view_with_owner(declared, owner)?
                    .as_str()
                    != hash.as_str()
            {
                return Err("StorageUnavailable：旧未发布profile缺可核独立来源".to_string());
            }
        }
    }
    Ok((id, hash))
}

fn verify_batch_profile(batch: &Value, binding: &(String, String)) -> Result<(), String> {
    let id = batch["profile_id"]
        .as_str()
        .ok_or("StorageUnavailable：batch.profile_id缺失")?;
    let hash = batch["profile_hash"]
        .as_str()
        .ok_or("StorageUnavailable：batch.profile_hash缺失")?;
    // 首次接纳以前的空发布可合法未绑定；不得回填其历史profile。
    if id.is_empty() && hash.is_empty() && required_array(batch, "raw_events")?.is_empty() {
        return Ok(());
    }
    if id != binding.0 || hash != binding.1 {
        return Err("StorageUnavailable：历史批次profile与固定输入绑定不一致".to_string());
    }
    Ok(())
}

fn profile_at_cut(conn: &Connection, generation: i64) -> Result<(String, String), String> {
    if generation == 0 {
        return Ok((String::new(), String::new()));
    }
    let batch_id:String=conn.query_row("SELECT b.batch_id FROM batches b JOIN structure_deltas d ON d.index_frontier=b.batch_id WHERE d.generation=?1",params![generation],|r|r.get(0)).map_err(|e|format!("StorageUnavailable：cut来源不存在：{e}"))?;
    let bytes = load_batch_bytes(conn, &batch_id)?;
    let batch: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok((
        batch["profile_id"]
            .as_str()
            .ok_or("StorageUnavailable：cut.profile_id缺失")?
            .to_string(),
        batch["profile_hash"]
            .as_str()
            .ok_or("StorageUnavailable：cut.profile_hash缺失")?
            .to_string(),
    ))
}

/// 一次新鲜捕获：小批次控制头/引用 + 每个私有根的完整原件证明；不持全部历史解码值。
///
/// 捕获作用域（`_capture`）**活到调用方释放为止**：同一次根审计里批次、Delta 原件与结构事实
/// 共用同一份节点读集，任一唯一节点原行只读一次。
struct Reachable {
    batches: BTreeMap<String, BatchRef>,
    /// 私有根 key → 本次新鲜行算出的闭包指纹（节点/根/槽任一变化都改它）。
    roots: BTreeMap<String, [u8; 32]>,
    /// 私有根 key → 该根**当前保留的原件证书**（本次新鲜行已逐字节比较过）。
    /// 覆盖 batch、Delta、fact；无法保留时显式记 `None`，不得漏键放行旧前缀。
    originals: cache::Originals,
    _capture: cache::CaptureGuard,
}

/// 本次捕获读到的全部根与独立索引（供根审计与失败前拒绝共用）。
fn capture_reachable_batches(conn: &Connection) -> Result<Reachable, String> {
    // #1394 第二段（2D）：每次仍在当前一致性读集中取得**全部可达根**（批次、Delta 原件、结构
    // 事实三列原件）、全部独立索引行与每个**唯一节点的当前原行**；同一捕获内共享依赖只读一次。
    // 历史重复内容以「闭包指纹 + 已核句柄」复用（指纹由本次新鲜行算出），故不再逐代全量展开、
    // 不再逐代重新解析。容量超限只失去复用，判据不变。
    let ids = {
        let mut q = conn
            .prepare("SELECT d.index_frontier FROM structure_deltas d")
            .map_err(|e| e.to_string())?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };
    let format = evidence_store::format(conn)?;
    let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
    let mut batches = BTreeMap::new();
    let mut originals = cache::Originals::new();
    for id in ids {
        let (reference, root_cert) = capture_batch_ref_with(conn, &mut reader, &id)?;
        if format.graph() {
            originals.insert(evidence_store::batch_root_key(&id), root_cert);
        }
        batches.insert(id, reference);
    }
    let mut roots: BTreeMap<String, [u8; 32]> = BTreeMap::new();
    // 不可达残留不得发布：每个私有根都必须有**唯一对应逻辑记录**（批次行 / Delta 行 / 结构事实行）、
    // 闭包完整（缺节点/坏字节/坏长度、未 Commit 的 after_batch 批次都在此核验），且库内每个节点
    // 都落在某个根的闭包内。命名空间与物理域必须一致：graph/2 只声明 batch 域。
    if format.graph() {
        for key in evidence_store::root_keys(conn)? {
            let (kind, record) = evidence_store::root_kind(&key)?;
            match kind {
                evidence_store::RootKind::Batch => {
                    let present: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM batches WHERE batch_id=?1",
                            params![record],
                            |r| r.get(0),
                        )
                        .map_err(|e| format!("StorageUnavailable：核私有根批次行失败：{e}"))?;
                    if present != 1 {
                        return Err(format!(
                            "StorageUnavailable：私有根 `{key}` 没有对应批次行（不可达残留），拒绝写入"
                        ));
                    }
                    if !batches.contains_key(record) {
                        // 尚未 Commit 的根也须核公开 batch_id/长度，不能只核私有根的自述摘要。
                        let (_, _, root_cert) = load_batch_with(conn, &mut reader, record, false)?;
                        originals.insert(key.clone(), root_cert);
                    }
                }
                evidence_store::RootKind::Delta => {
                    if !format.records_in_graph() {
                        return Err(format!(
                            "StorageUnavailable：格式 `{}` 未声明 Delta 根 `{key}`（不得混代）",
                            format.tag()
                        ));
                    }
                    let generation = record.parse::<i64>().map_err(|_| {
                        format!("StorageUnavailable：Delta 根 `{key}` 代数非规范十进制")
                    })?;
                    let present: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM structure_deltas WHERE generation=?1",
                            params![generation],
                            |r| r.get(0),
                        )
                        .map_err(|e| format!("StorageUnavailable：核 Delta 根行失败：{e}"))?;
                    if present != 1 {
                        return Err(format!(
                            "StorageUnavailable：私有根 `{key}` 没有对应 Delta 行（不可达残留），拒绝写入"
                        ));
                    }
                    let verified = reader.read_root(&key, false)?;
                    roots.insert(key.clone(), verified.closure);
                    originals.insert(key.clone(), verified.root_cert);
                }
                evidence_store::RootKind::Fact => {
                    if !format.records_in_graph() {
                        return Err(format!(
                            "StorageUnavailable：格式 `{}` 未声明结构事实根 `{key}`（不得混代）",
                            format.tag()
                        ));
                    }
                    let present: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM structure_facts WHERE object_id=?1",
                            params![record],
                            |r| r.get(0),
                        )
                        .map_err(|e| format!("StorageUnavailable：核结构事实根行失败：{e}"))?;
                    if present != 1 {
                        return Err(format!(
                            "StorageUnavailable：私有根 `{key}` 没有对应结构事实行（不可达残留），拒绝写入"
                        ));
                    }
                    let verified = reader.read_root(&key, false)?;
                    roots.insert(key.clone(), verified.closure);
                    originals.insert(key.clone(), verified.root_cert);
                }
            }
        }
        // 不可达残留：表内行数必须等于**本次捕获**（覆盖全部根闭包，跨读取器合并）读到的可达节点数。
        evidence_store::ensure_all_nodes_reachable(
            conn,
            cache::capture_reachable_len(reader.conn_id()),
        )?;
    } else if evidence_store::graph_tables_present(conn)? {
        // 整块域不得出现任何私有根/节点（半降级一律拒绝，不提供降级通道）。
        let roots = evidence_store::root_keys(conn)?;
        let (nodes, _) = evidence_store::stats(conn)?;
        if !roots.is_empty() || nodes != 0 {
            return Err(format!(
                "StorageUnavailable：整块域出现私有根/节点 {} 根 / {nodes} 节点，拒绝写入",
                roots.len()
            ));
        }
    }
    Ok(Reachable {
        batches,
        roots,
        originals,
        _capture: reader.into_capture(),
    })
}

fn verify_reachable_root(conn: &Connection) -> Result<(), String> {
    verified_root_raw_events(conn).map(|_| ())
}

fn verified_root_raw_events(conn: &Connection) -> Result<Vec<Value>, String> {
    #[cfg(test)]
    let started = std::time::Instant::now();
    let out = verified_root_raw_events_inner(conn);
    #[cfg(test)]
    cache::counter(|c| c.audit_ns += started.elapsed().as_nanos() as u64);
    out
}

fn verified_root_raw_events_inner(conn: &Connection) -> Result<Vec<Value>, String> {
    // #1394 第二段（2D）：整次根审计共享**一个**捕获作用域——根清单、Delta 原件与结构事实
    // 共用同一份节点读集，同一唯一节点原行只读一次（写入路径会显式作废该读集）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    if conn.is_autocommit() {
        // 所有节点、根和独立索引必须来自同一个读集，包括直接调用本入口的读者。
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("开根审计读事务失败：{e}"))?;
        let raw = verified_root_raw_events_inner(&tx)?;
        tx.commit()
            .map_err(|e| format!("结束根审计读事务失败：{e}"))?;
        return Ok(raw);
    }
    let reachable = capture_reachable_batches(conn)?;
    let control = v2::prepare_control(conn)?;
    verify_reachable_root_in_tx(conn, &reachable, control)
        .map_err(|e| format!("StorageUnavailable：{e}"))
}

fn verify_published_index_generations(conn: &Connection, generation: i64) -> Result<(), String> {
    // #1371：索引行须来自一次实际发布；cut-0 没有发布行。逐 cut 重建会过滤未来行，故须另核全表。
    // raw_events 可已接纳尚未发布，batches 可有未 Commit 的不可达批次，不纳入此索引门。
    for (table, predicate) in [
        ("objects", "typeof(first_known_generation)<>'integer' OR first_known_generation<1 OR first_known_generation>?1 OR typeof(published_generation)<>'integer' OR published_generation<1 OR published_generation>?1 OR (withdrawn_generation IS NOT NULL AND (typeof(withdrawn_generation)<>'integer' OR withdrawn_generation<1 OR withdrawn_generation>?1))"),
        ("witnesses", "typeof(published_generation)<>'integer' OR published_generation<1 OR published_generation>?1"),
        ("relations", "typeof(published_generation)<>'integer' OR published_generation<1 OR published_generation>?1"),
        ("observations", "typeof(published_generation)<>'integer' OR published_generation<1 OR published_generation>?1"),
    ] {
        let invalid: bool = conn.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {predicate})"),
            params![generation], |r| r.get(0),
        ).map_err(|e| format!("检查 {table} 发布代失败：{e}"))?;
        if invalid {
            return Err(format!("{table} 含不属于 1..={generation} 已发布代的索引行"));
        }
    }
    Ok(())
}

fn verify_reachable_root_in_tx(
    conn: &Connection,
    reachable: &Reachable,
    mut control: v2::ExpectedControl,
) -> Result<Vec<Value>, String> {
    let batches = &reachable.batches;
    // #1394：库的 Delta 域（旧 full / changes-only）由私有登记表唯一决定；模式损坏在读取与
    // 任何写入之前拒绝，不提供「旧全量核验失败再试稀疏」的降级通道。
    let mode = delta_storage::read_mode(conn)?;
    let format = evidence_store::format(conn)?;
    let generation = verify_required_meta(conn)?;
    verify_published_index_generations(conn, generation)?;
    // #1371 AC7：已接纳但未发布的原始事实也是持久前件；所有结构读写共用坐标完整性门。
    let raw_events = read_raw_events(conn)?;
    let profile_binding = verify_profile_binding(conn)?;
    let meta_session = meta_get_opt(conn, "session_id")?.unwrap_or_default();
    let meta_cat = meta_get_opt(conn, "catalog_revision")?.unwrap_or_default();
    let meta_idx = meta_get_opt(conn, "index_frontier")?.unwrap_or_default();
    let meta_cut = meta_get_opt(conn, "structure_cut")?.unwrap_or_default();
    // 全量 Delta 行（含未来行检测：不得存在 generation > meta.generation 的已发布行）。
    let mut stmt = conn
        .prepare(
            "SELECT generation, session_id, catalog_revision, base_cut, next_cut, seq_range_json,
                    index_frontier, input_frontier, catalog_run_status, catalog_evidence_json, delta_json
             FROM structure_deltas ORDER BY generation ASC",
        )
        .map_err(|e| format!("prepare 可达根核失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
            ))
        })
        .map_err(|e| format!("query 可达根失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect 可达根失败：{e}"))?;
    // 末列：该代 Delta 原件的**当前内容指纹**。整块域就是行内原文；图域是由本次新鲜读入的
    // 节点/根算出的闭包指纹——节点/长度/根/槽任一变化都改它，从而作废已核前缀并严格重核。
    let rows: Vec<cache::DeltaRow> = rows
        .into_iter()
        .map(|row| {
            let digest = if format.records_in_graph() {
                let key = evidence_store::delta_root_key(row.0);
                match reachable.roots.get(&key) {
                    Some(d) => evidence_store::hex_bytes(d),
                    None => "missing".to_string(),
                }
            } else {
                row.10.clone()
            };
            // 行内列**原样**参与前缀比较：图域它必须是空占位，非空（被塞回整块原文）即改前缀、
            // 即作废复用并进入严格重核。
            let delta_json = row.10;
            (
                row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, delta_json,
                digest,
            )
        })
        .collect();

    if generation == 0 {
        if !rows.is_empty() {
            return Err(
                "StorageUnavailable：初态 generation=0 却存在持久 Delta（不一致）".to_string(),
            );
        }
        control.finish()?;
        return Ok(raw_events);
    }

    if rows.len() != generation as usize {
        return Err(format!(
            "StorageUnavailable：已发布代链断裂（generation={generation}，仅 {} 行 Delta）",
            rows.len()
        ));
    }

    // 审计的原始历史与同 cut 快照逐字段同形：只取**已发布前沿**的前缀并做同一 wire 投影。
    let published_frontier: i64 = meta_get_opt(conn, "last_advance_frontier")?
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1);
    let audit_raw: Vec<Value> = raw_events
        .iter()
        .filter(|e| e["seq"].as_i64().is_some_and(|n| n <= published_frontier))
        .map(project_raw_event_wire)
        .collect::<Result<_, _>>()?;
    let mut audit = audit::AuditWalk::capture(conn, &audit_raw)?;
    let identity = (
        meta_session.clone(),
        meta_cat.clone(),
        profile_binding.clone(),
        mode.tag().to_string(),
        format.tag().to_string(),
    );
    // 同一次捕获的全部私有根原件共同绑定前缀；行内原件仍由 rows/AuditWalk 逐字节核对。
    let mut originals = reachable.originals.clone();
    let resume = cache::resume(&rows, &identity, &mut audit, &originals);
    let mut prev_frontier: i64 = if resume == 0 { -1 } else { rows[resume - 1].7 };
    // 后缀核验共用一个读取器：与批次/事实共用本次捕获的节点读集与已核句柄。
    let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
    for (
        i,
        (
            g,
            sid,
            cat,
            base,
            next,
            seq_range_json,
            idx,
            frontier,
            run_status,
            evidence_json,
            delta_json,
            _delta_root_digest,
        ),
    ) in rows.iter().enumerate()
    {
        let reference = batches.get(idx).ok_or("StorageUnavailable：缺可达批次")?;
        if i < resume {
            if let BatchRef::CertifiedHeader(header) = reference {
                control.verify_header(*g, idx, header)?;
                continue;
            }
        }
        // 每代至多一份完整 Value：先核控制头，再将同一 Value 交给原结构判据。
        // 前缀 PendingFull 也不能省略控制核验；原定 resume 在本轮内不得重新提升。
        #[cfg(test)]
        cache::counter(|c| {
            if matches!(reference, BatchRef::PendingFull) {
                c.batch_header_full_fallbacks += 1;
            }
            if i >= resume {
                c.batch_suffix_full_reads += 1;
            }
        });
        let (batch, root_cert) = load_batch_value_with(conn, &mut reader, idx)?;
        if format.graph() {
            originals.insert(evidence_store::batch_root_key(idx), root_cert);
        }
        control.verify_header(*g, idx, &BatchControlHeader::from_verified_batch(&batch))?;
        if i < resume {
            continue;
        }
        // Delta 原件：图域由 `delta/<generation>` 私有根无损展开（列必须是空占位），整块域取列。
        let delta_json = read_record_original_text(
            conn,
            &mut reader,
            format,
            &evidence_store::delta_root_key(*g),
            delta_json,
            &format!("delta[{g}]"),
        )?;
        let expect = (i + 1) as i64;
        if *g != expect {
            return Err(format!(
                "StorageUnavailable：Delta 代际断链（第 {} 行 generation={g}，应 {expect}）",
                i + 1
            ));
        }
        // 未来行（> meta.generation）不可能出现在本循环（行数==generation 且 1..g 连续），防御保留。
        if sid != &meta_session {
            return Err(format!(
                "StorageUnavailable：delta[{g}].session_id 与 meta.session_id 不一致"
            ));
        }
        if cat != &meta_cat {
            return Err(format!(
                "StorageUnavailable：delta[{g}].catalog_revision 与 meta.catalog_revision 不一致"
            ));
        }
        let expect_base = if *g == 1 {
            "cut-0".to_string()
        } else {
            format!("cut-{}", *g - 1)
        };
        if base != &expect_base {
            return Err(format!(
                "StorageUnavailable：delta[{g}].base_cut=`{base}` 与前置 cut `{expect_base}` 不一致"
            ));
        }
        let expect_next = format!("cut-{g}");
        if next != &expect_next {
            return Err(format!(
                "StorageUnavailable：delta[{g}].next_cut=`{next}` 与 `{expect_next}` 不一致"
            ));
        }
        if idx.is_empty() {
            return Err(format!(
                "StorageUnavailable：delta[{g}].index_frontier 为空（已发布 cut 缺批次引用）"
            ));
        }
        verify_batch_coordinates(&batch, idx, *g, next, *frontier, cat)?;
        // seq_range 连续：from = 上一已发布 frontier + 1，to = 本代 frontier。
        let sr: Value = json_shape(seq_range_json, JsonShape::Object)?;
        let sr_from = sr["from"]
            .as_str()
            .and_then(|s| parse_canonical_i64(s, "seq_range.from").ok())
            .ok_or_else(|| format!("StorageUnavailable：delta[{g}].seq_range.from 非规范十进制"))?;
        let sr_to = sr["to"]
            .as_str()
            .and_then(|s| parse_canonical_i64(s, "seq_range.to").ok())
            .ok_or_else(|| format!("StorageUnavailable：delta[{g}].seq_range.to 非规范十进制"))?;
        if sr_from != prev_frontier + 1 || sr_to != *frontier {
            return Err(format!(
                "StorageUnavailable：delta[{g}].seq_range=[{sr_from},{sr_to}] 与前沿链（期望 [{},{}]）不一致",
                prev_frontier + 1,
                frontier
            ));
        }
        prev_frontier = *frontier;
        // 内层 payload 头与外层列同一身份（11 列逐一对应）。
        let dj: Value = json_shape(&delta_json, JsonShape::Object)?;
        if dj["session_id"].as_str() != Some(sid.as_str())
            || dj["generation"].as_str() != Some(g.to_string().as_str())
            || dj["catalog_revision"].as_str() != Some(cat.as_str())
            || dj["base_cut"].as_str() != Some(base.as_str())
            || dj["next_cut"].as_str() != Some(next.as_str())
            || dj["index_frontier"].as_str() != Some(idx.as_str())
            || dj["input_frontier"].as_str() != Some(frontier.to_string().as_str())
            || dj["catalog_run_status"].as_str() != Some(run_status.as_str())
        {
            return Err(format!(
                "StorageUnavailable：delta[{g}] 内层 payload 头与外层列不一致"
            ));
        }
        // seq_range 与 catalog_evidence：深等（键序无关），不比较原始 JSON 字节。
        if canonical_json(&dj["seq_range"]) != canonical_json(&sr) {
            return Err(format!(
                "StorageUnavailable：delta[{g}] 内层 seq_range 与外层 seq_range_json 不一致"
            ));
        }
        let evidence: Value = json_shape(evidence_json, JsonShape::Object)?;
        if canonical_json(&dj["catalog_evidence"]) != canonical_json(&evidence) {
            return Err(format!(
                "StorageUnavailable：delta[{g}] 内层 catalog_evidence 与外层 catalog_evidence_json 不一致"
            ));
        }
        verify_batch_profile(&batch, &profile_binding)?;
        audit.verify(&dj, *g, &batch, mode)?;
    }
    control.finish()?;
    audit.finish()?;
    // meta 与最末已发布 Delta 的根元组对应关系。
    let last = rows.last().unwrap();
    if tb02a::enabled(conn)? {
        let last_evidence = json_shape(&last.9, JsonShape::Object)?;
        let axes = last_evidence["axes"]
            .as_object()
            .ok_or("StorageUnavailable：末代逐轴证据缺失")?;
        for (axis, expected) in axes {
            let actual:Value=conn.query_row("SELECT impl_status,proof_status,run_status,evidence_json FROM catalog WHERE catalog_id=?1",params![axis],|r|Ok(json!({"impl_status":r.get::<_,String>(0)?,"proof_status":r.get::<_,String>(1)?,"run_status":r.get::<_,String>(2)?,"evidence":json_column(r,3,JsonShape::Object)?}))).map_err(|e|e.to_string())?;
            if actual != *expected {
                return Err(format!(
                    "StorageUnavailable：当前目录轴{axis}与末代Delta证据不一致"
                ));
            }
        }
    } else {
        let (run, evidence): (String, String) = conn
            .query_row(
                "SELECT run_status,evidence_json FROM catalog WHERE catalog_id='CC-006'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| format!("StorageUnavailable：读当前目录证据失败：{e}"))?;
        if run != last.8
            || json_shape(&evidence, JsonShape::Object)? != json_shape(&last.9, JsonShape::Object)?
        {
            return Err("StorageUnavailable：当前目录与末代 Delta 证据不一致".to_string());
        }
    }
    if meta_idx != last.6 {
        return Err(format!(
            "StorageUnavailable：meta.index_frontier=`{meta_idx}` 与 delta[{generation}].index_frontier=`{}` 不一致",
            last.6
        ));
    }
    if meta_cut != last.4 {
        return Err(format!(
            "StorageUnavailable：meta.structure_cut=`{meta_cut}` 与 delta[{generation}].next_cut=`{}` 不一致",
            last.4
        ));
    }
    cache::store_root(rows, identity, audit, &originals);
    Ok(raw_events)
}

fn ensure_initialized(conn: &Connection) -> Result<(), String> {
    if db_meta(conn)?.contains_key("session_id") {
        Ok(())
    } else {
        Err(
            "StorageUnavailable：S 库未初始化（请先 init；init 只允许不存在的目标，不覆盖既有库）"
                .to_string(),
        )
    }
}

/// #1394 第二段：写入域门。只有旧整块域与 `evidence-graph/3` 可写；`evidence-graph/2` 只读兼容——
/// 本 writer 不在同一个格式名下换存储含义，也不原地迁移。
fn ensure_writable(conn: &Connection) -> Result<(), String> {
    evidence_store::format(conn)?.writable()
}

fn load_profile(path: &Path) -> Result<(Value, String, String), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 profile 失败：{e}"))?;
    let profile: Value =
        serde_json::from_str(&text).map_err(|e| format!("profile JSON 解析失败：{e}"))?;
    let profile_id = profile
        .get("profile_id")
        .and_then(Value::as_str)
        .ok_or("profile 缺 profile_id")?
        .to_string();
    if profile_id == tb02a::PROFILE {
        let declared: Value = serde_json::from_str(include_str!(
            "../../../s_session/profiles/ohlc_integer_tb02a_v1.json"
        ))
        .map_err(|e| e.to_string())?;
        if profile != declared {
            return Err("InvalidDomain：OHLC profile必须为已声明的完整固定配置".into());
        }
    }
    let profile_hash = sha256_hex(&canonical_bytes_of_value(&profile));
    Ok((profile, profile_id, profile_hash))
}

/// 具名 TestOnly 受控暂停点：暴露真实阶段供外部对精确 S PID 发 SIGKILL。只由
/// `S_SESSION_PAUSE`（阶段名）+ `S_SESSION_PAUSE_MARKER`（标记文件路径）激活；生产时序不变，
/// 不改提交路径。marker 写入自身 PID/阶段/DB/可执行文件，供外部核身份后 kill。
fn testonly_pause(stage: &str) {
    if std::env::var("S_SESSION_PAUSE").ok().as_deref() != Some(stage) {
        return;
    }
    let db = std::env::var("S_SESSION_PAUSE_DB").unwrap_or_default();
    let marker = std::env::var("S_SESSION_PAUSE_MARKER")
        .unwrap_or_else(|_| format!("/tmp/s_session_pause_{stage}.pid"));
    let release = std::env::var("S_SESSION_PAUSE_RELEASE").unwrap_or_default();
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let _ = std::fs::write(
        &marker,
        format!(
            "pid={}\nstage={stage}\ndb={db}\nexe={exe}\n",
            std::process::id()
        ),
    );
    // 具名 TestOnly 受控暂停：可被 SIGKILL（真实恢复试验），也可由 release 文件显式解除（用于
    // 活旧 writer 跨 epoch 交错的受控试验）；生产路径不设 S_SESSION_PAUSE，不受影响。
    loop {
        if !release.is_empty() && std::path::Path::new(&release).exists() {
            let _ = std::fs::remove_file(&release);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// TestOnly 诊断 sidecar：从当前传入的**真实 writer 连接**（非另开连接）回读平台前件 PRAGMA，
/// 记录 PID/命令/DB/阶段/platform 与三项实际值。仅由 `S_SESSION_PRAGMA_SIDECAR=1` 显式启用；
/// 不改变 stdout 回执/结构化 stderr/fence 合同。
fn record_connection_pragmas(conn: &Connection, stage: &str) {
    if std::env::var("S_SESSION_PRAGMA_SIDECAR").ok().as_deref() != Some("1") {
        return;
    }
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap_or_default();
    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .unwrap_or(-999);
    let fullfsync: Option<i64> = {
        #[cfg(target_os = "macos")]
        {
            Some(
                conn.query_row("PRAGMA fullfsync", [], |r| r.get(0))
                    .unwrap_or(-999),
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    };
    let db = std::env::var("S_SESSION_PRAGMA_SIDECAR_DB").unwrap_or_default();
    let cmd = std::env::args().collect::<Vec<_>>().join(" ");
    let path = std::env::var("S_SESSION_PRAGMA_SIDECAR_PATH")
        .unwrap_or_else(|_| format!("/tmp/s_session_pragma_{stage}.txt"));
    let out = format!(
        "pid={}\nstage={stage}\nplatform={}\ncmd={cmd}\ndb={db}\njournal_mode={journal_mode}\nsynchronous={synchronous}\nfullfsync={}\n",
        std::process::id(),
        std::env::consts::OS,
        fullfsync.map(|v| v.to_string()).unwrap_or_else(|| "n/a".to_string()),
    );
    let _ = std::fs::write(&path, out);
}

fn cmd_init(db: &Path, session: &str, catalog_path: &Path) -> Result<(), String> {
    println!("{}", init_core(db, session, catalog_path)?);
    Ok(())
}

/// 旧 reset（先删后建 v1）语义不变；#1394 独评 F1 只加一道门：已有 `s-session/3` 域在任何删除
/// 之前 fail closed（旧 reset 只懂 v1/v2 合同，本切片不提供新迁移/重置功能）。
fn cmd_reset(db: &Path, session: &str, catalog_path: &Path) -> Result<(), String> {
    // 与新域 init / 正式 writer 同一把锁覆盖「判域 → 删除 → 重建」，避免检查后换域。
    let _writer = newchan_rust::session_transport::process_lock(db, "S")?;
    v3::refuse_reset_of_new_domain(db)?;
    remove_db_files(db);
    cmd_init(db, session, catalog_path)
}

fn init_core(db: &Path, session: &str, catalog_path: &Path) -> Result<Value, String> {
    // H4：init 只允许不存在的目标——拒绝覆盖既有库（既有持久事实可恢复）。
    if db.exists() {
        return Err(format!(
            "库已存在（{}）：init 拒绝覆盖；请用新路径，或显式 reset（一次性试验）",
            db.display()
        ));
    }
    let conn = open_db(db)?;
    conn.execute_batch(SCHEMA)
        .map_err(|e| format!("建 schema 失败：{e}"))?;
    // #1394：新库固定私有 changes-only/1 登记（显式单行）。旧库无此表，继续旧 full 域，不迁移。
    delta_storage::declare_new(&conn)?;
    // #1394 第二段：新库固定私有 evidence-graph/2 登记（显式单行）。旧库无此表，继续旧整块域，不迁移。
    evidence_store::declare_new(&conn)?;
    let sqlite_version: String = conn
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .map_err(|e| format!("读 sqlite_version 失败：{e}"))?;
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(|e| format!("读 journal_mode 失败：{e}"))?;
    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(|e| format!("读 synchronous 失败：{e}"))?;
    meta_set(&conn, "session_id", session)?;
    meta_set(&conn, "generation", "0")?;
    meta_set(&conn, "structure_cut", "cut-0")?;
    meta_set(&conn, "index_frontier", "")?;
    meta_set(&conn, "last_advance_frontier", "-1")?;
    meta_set(&conn, "sqlite_version", &sqlite_version)?;
    meta_set(&conn, "journal_mode", &journal_mode)?;
    meta_set(&conn, "synchronous", &synchronous.to_string())?;
    meta_set(&conn, "platform", std::env::consts::OS)?;
    meta_set(&conn, "writer_epoch", DEFAULT_WRITER_EPOCH)?;
    meta_set(&conn, "advance_state", "idle")?;
    meta_set(&conn, "rule_revision", RULE_REVISION)?;
    meta_set(
        &conn,
        "scope",
        &json!({"structure": "CompleteCut", "economic": "not_started"}).to_string(),
    )?;

    let catalog_text =
        std::fs::read_to_string(catalog_path).map_err(|e| format!("读 catalog 失败：{e}"))?;
    let catalog: Value =
        serde_json::from_str(&catalog_text).map_err(|e| format!("catalog JSON 解析失败：{e}"))?;
    let items = catalog
        .get("items")
        .and_then(Value::as_array)
        .ok_or("catalog 缺 items 数组")?;
    let catalog_revision = catalog
        .get("catalog_revision")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    meta_set(&conn, "catalog_revision", &catalog_revision)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开事务失败：{e}"))?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR REPLACE INTO catalog(catalog_id, kind, title, domain, branches_json, impl_status, proof_status, run_status, evidence_json)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .map_err(|e| format!("prepare catalog 失败：{e}"))?;
        for item in items {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let kind = item
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let domain = item
                .get("domain")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let branches = item
                .get("branches")
                .cloned()
                .unwrap_or(json!([]))
                .to_string();
            stmt.execute(params![
                id,
                kind,
                title,
                domain,
                branches,
                "not_implemented",
                "not_proved",
                "not_run",
                "{}"
            ])
            .map_err(|e| format!("insert catalog 失败：{e}"))?;
        }
    }
    tx.commit()
        .map_err(|e| format!("catalog commit 失败：{e}"))?;
    Ok(json!({
        "ok": true,
        "session_id": session,
        "sqlite_version": sqlite_version,
        "journal_mode": journal_mode,
        "synchronous": synchronous,
        "platform": std::env::consts::OS,
        "writer_epoch": DEFAULT_WRITER_EPOCH,
        "catalog_items": items.len()
    }))
}

// ────────────────────────────────────────────────────────────────────────────
// S.AcceptInput：持久原始事件 + 合法修订链（同身份同 revision 重放 / 异内容冲突 / 新 revision
// supersedes）+ profile 绑定。
// ────────────────────────────────────────────────────────────────────────────

fn cmd_accept(
    db: &Path,
    input_path: &Path,
    profile_path: &Path,
    configured_epoch: &str,
) -> Result<(), String> {
    let text = std::fs::read_to_string(input_path).map_err(|e| format!("读输入失败：{e}"))?;
    println!(
        "{}",
        accept_core(db, &text, profile_path, configured_epoch, None)?
    );
    Ok(())
}

fn accept_core(
    db: &Path,
    text: &str,
    profile_path: &Path,
    configured_epoch: &str,
    context: Option<&v2::InputContext>,
) -> Result<Value, String> {
    // #1394 第二段（2D）：一次 accept 共享一个捕获作用域（写入前会显式作废）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    let input = tb02a::parse_input(&v2::strict_json(text.as_bytes())?)?;
    let (profile, profile_id, profile_hash) = load_profile(profile_path)?;
    raw_input::validate_profile_binding(&input, &profile_id)?;
    let volume_unit = profile
        .get("volume_unit")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
    ensure_writable(&conn)?;
    if meta_get_opt(&conn, "rule_revision")?.as_deref() != Some(RULE_REVISION) {
        return Err("VersionMismatch：历史规则会话仅可读，禁止接纳新规则输入".into());
    }
    record_connection_pragmas(&conn, "accept");
    let meta_session = meta_get_opt(&conn, "session_id")?.unwrap_or_default();
    if meta_session != input.session_id {
        return Err(format!(
            "IdentityConflict：输入 session_id=`{}` 与库 session_id=`{meta_session}` 不一致",
            input.session_id
        ));
    }

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开 accept 事务失败：{e}"))?;
    let results;
    {
        verify_writer_epoch(&tx, configured_epoch)?;
        verify_reachable_root(&tx)?;

        let bound_profile_id = meta_get_opt(&tx, "profile_id")?.unwrap_or_default();
        if bound_profile_id.is_empty() {
            if profile_id == tb02a::PROFILE {
                tx.execute_batch(tb02a::SCHEMA).map_err(|e| e.to_string())?;
            }
            meta_set(&tx, "profile_id", &profile_id)?;
            meta_set(&tx, "profile_hash", &profile_hash)?;
            meta_set(&tx, "profile_definition", &canonical_json(&profile))?;
        } else if bound_profile_id != profile_id {
            return Err(format!(
                "IdentityConflict：session 已绑定 profile=`{bound_profile_id}`，本次 `{profile_id}` 不一致（不覆盖既有 cut 来源）"
            ));
        } else {
            let bound_hash = meta_get_opt(&tx, "profile_hash")?.unwrap_or_default();
            if bound_hash != profile_hash {
                return Err(format!(
                    "IdentityConflict：profile_id=`{profile_id}` 已绑定规范哈希 `{bound_hash}`，本次字节哈希 `{profile_hash}` 不同（同名异字节，拒绝接纳元数据偷换已提交快照来源）"
                ));
            }
        }

        results = raw_input::accept_events(&tx, &input, volume_unit.as_deref())?;

        v2::record_accept(&tx, context, &results)?;
        meta_set(&tx, "input_profile", &input.profile)?;
        meta_set(&tx, "input_file_hash", &sha256_hex(text.as_bytes()))?;
    }
    tx.commit()
        .map_err(|e| format!("accept commit 失败：{e}"))?;
    Ok(json!({
        "ok": true,
        "session_id": input.session_id,
        "profile": profile_id,
        "profile_hash": profile_hash,
        "volume_unit": volume_unit,
        "results": results,
    }))
}

// ────────────────────────────────────────────────────────────────────────────
// S.Advance：Begin（判定前持久）→ 有效源位置 → 同次真实 Rust parser → CC-006 → Commit（CAS 发布）
// ────────────────────────────────────────────────────────────────────────────

fn read_raw_events(conn: &Connection) -> Result<Vec<Value>, String> {
    read_raw_events_until(conn, None)
}

fn check_read_deadline(deadline: Option<std::time::Instant>) -> Result<(), String> {
    if deadline.is_some_and(|d| std::time::Instant::now() >= d) {
        Err("Incomplete/VerifyDeadline：完整核验超过绝对期限".into())
    } else {
        Ok(())
    }
}

fn read_raw_events_until(
    conn: &Connection,
    deadline: Option<std::time::Instant>,
) -> Result<Vec<Value>, String> {
    check_read_deadline(deadline)?;
    let mut stmt = conn
        .prepare(
            "SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace,
                    source_epoch, instrument, event_id, received_at, raw_text, price,
                    ts, volume, source_coord, supersedes_revision
             FROM raw_events ORDER BY seq ASC",
        )
        .map_err(|e| format!("prepare 读取失败：{e}"))?;
    let mut rows = stmt
        .query_map([], decode_raw_row)
        .map_err(|e| format!("query 失败：{e}"))?
        .map(|row| {
            check_read_deadline(deadline)?;
            row.map_err(|e| format!("collect 失败：{e}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    tb02a::enrich_raw_until(conn, &mut rows, deadline)?;
    v3::business_json::validate_raw(v3::business_json::RawEventsView::plain(&rows), deadline)?;
    check_read_deadline(deadline)?;
    Ok(rows)
}

/// 当前scope拥有完整raw数组；借SQL字段与OHLC视图，不拆出裸Vec<Value>。
fn read_raw_events_until_admitted(
    conn: &Connection,
    deadline: Option<std::time::Instant>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<v3::business_json::JsonOwner, String> {
    check_read_deadline(deadline)?;
    if work.is_none() {
        return Ok(v3::business_json::JsonOwner::Plain(
            Value::Array(read_raw_events_until(conn, deadline)?).into(),
        ));
    }
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        conn, c"SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace, source_epoch, instrument, event_id, received_at, raw_text, price, ts, volume, source_coord, supersedes_revision FROM raw_events ORDER BY seq ASC",
    ).map_err(|e| format!("prepare 读取失败：{e}"))?;
    let mut rows = v3::business_json::ScopedJsonArray::new(None, work)?;
    while let Some(mut row) = stmt.next().map_err(|e| format!("collect 失败：{e}"))? {
        check_read_deadline(deadline)?;
        let owner = decode_raw_row_admitted(&mut row, work)?;
        rows.push(owner.view())?;
    }
    stmt.finalize()
        .map_err(|e| format!("finalize 读取失败：{e}"))?;
    // 全部raw列类型与SQL完成后才进入OHLC阶段，保留旧首错顺序。
    let raw = rows.finish();
    check_read_deadline(deadline)?;
    let ohlc = tb02a::prepare_raw_enrichment_admitted(conn, work)?;
    let owner = if ohlc {
        let mut enriched = v3::business_json::ScopedJsonArray::new(None, work)?;
        for row in raw.view().as_array().expect("raw构造器只产数组").iter() {
            check_read_deadline(deadline)?;
            let row = tb02a::enrich_raw_view_admitted(conn, row, true, work)?;
            enriched.push(row.view())?;
        }
        enriched.finish()
    } else {
        raw
    };
    // 同一验证核；RawEvents由拥有者保留原许可，不使借视图逃出。
    // 验证逐行时不能丢掉修订链；统一入口接受借用JsonOwner数组。
    v3::business_json::validate_raw_admitted((&owner).into(), deadline, work)?;
    check_read_deadline(deadline)?;
    Ok(owner)
}

fn decode_raw_row_admitted(
    row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<v3::business_json::JsonOwner, String> {
    use newchan_rust::s_content::capture_allocation::audit_json::JsonView;
    use v3::audit_store::{CodecRead, NativeAdmission};
    let admission = NativeAdmission(work);
    let columns = [
        (0, "identity_key"),
        (2, "input_revision"),
        (3, "payload_hash"),
        (4, "receipt_id"),
        (6, "source_namespace"),
        (7, "source_epoch"),
        (8, "instrument"),
        (9, "event_id"),
        (10, "received_at"),
        (11, "raw_text"),
        (12, "price"),
        (13, "ts"),
        (14, "volume"),
        (15, "source_coord"),
    ];
    let mut texts: [Option<NativeValue<String>>; 14] = std::array::from_fn(|_| None);
    let mut revision = None;
    let mut seq = None;
    for ((column, _), text) in columns.iter().zip(texts.iter_mut()) {
        // 与旧decode_raw_row的列读取顺序相同：文本0、整数1、文本2..4、整数5。
        if *column == 2 {
            revision = Some(Value::from(
                row.integer(1).map_err(|e| format!("collect 失败：{e}"))?,
            ));
        }
        if *column == 6 {
            seq = Some(Value::from(
                row.integer(5).map_err(|e| format!("collect 失败：{e}"))?,
            ));
        }
        *text = Some(
            admission
                .native_string(
                    row.text(*column)
                        .map_err(|e| format!("collect 失败：{e}"))?,
                )
                .map_err(|e| e.to_string())?,
        );
    }
    let revision = revision.expect("固定列序包含revision");
    let seq = seq.expect("固定列序包含seq");
    let supersedes = row
        .optional_integer(16)
        .map_err(|e| format!("collect 失败：{e}"))?
        .map(Value::from)
        .unwrap_or(Value::Null);
    let mut fields: [(&str, JsonView<'_>); 17] = [("", JsonView::Null); 17];
    for (index, ((_, key), text)) in columns.iter().zip(texts.iter()).enumerate() {
        fields[index] = (key, JsonView::String(text.as_ref().unwrap().as_str()));
    }
    fields[14] = ("revision", (&revision).into());
    fields[15] = ("seq", (&seq).into());
    fields[16] = ("supersedes_revision", (&supersedes).into());
    v3::business_json::json_from_view_admitted(
        &admission,
        JsonView::object_fields(&fields).map_err(|e| e.to_string())?,
    )
}

/// Q固定追加与cold路径复用完全相同的列解码/OHLC借视图核。
#[cfg(feature = "s_capture_native_testonly")]
fn read_raw_event_at_admitted(
    conn: &Connection,
    seq: i64,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<v3::business_json::JsonOwner, String> {
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(conn,
        c"SELECT identity_key,revision,input_revision,payload_hash,receipt_id,seq,source_namespace,source_epoch,instrument,event_id,received_at,raw_text,price,ts,volume,source_coord,supersedes_revision FROM raw_events WHERE seq=?1").map_err(|e|format!("StorageUnavailable：{e}"))?;
    stmt.bind_integer(1, seq)
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let mut row = stmt
        .next()
        .map_err(|e| format!("StorageUnavailable：{e}"))?
        .ok_or("StorageUnavailable：raw seq缺失")?;
    let owner = decode_raw_row_admitted(&mut row, work)?;
    drop(row);
    stmt.finalize()
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    // 固定seq入口仍只读该行及对应OHLC，不新增全表计数。
    let ohlc = tb02a::enabled_admitted(conn, work)?;
    if ohlc {
        tb02a::enrich_raw_view_admitted(conn, owner.view(), true, work)
    } else {
        Ok(owner)
    }
}

fn decode_raw_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "identity_key": r.get::<_, String>(0)?,
        "revision": r.get::<_, i64>(1)?,
        "input_revision": r.get::<_, String>(2)?,
        "payload_hash": r.get::<_, String>(3)?,
        "receipt_id": r.get::<_, String>(4)?,
        "seq": r.get::<_, i64>(5)?,
        "source_namespace": r.get::<_, String>(6)?,
        "source_epoch": r.get::<_, String>(7)?,
        "instrument": r.get::<_, String>(8)?,
        "event_id": r.get::<_, String>(9)?,
        "received_at": r.get::<_, String>(10)?,
        "raw_text": r.get::<_, String>(11)?,
        "price": r.get::<_, String>(12)?,
        "ts": r.get::<_, String>(13)?,
        "volume": r.get::<_, String>(14)?,
        "source_coord": r.get::<_, String>(15)?,
        "supersedes_revision": r.get::<_, Option<i64>>(16)?,
    }))
}
/// 固定 Accept 的实际 seq 回读；沿完整读取相同解码，不构造旧 raw 副本。
#[cfg(feature = "s_capture_native_testonly")]
fn read_raw_event_at(conn: &Connection, seq: i64) -> Result<Value, String> {
    let mut value = conn.query_row(
        "SELECT identity_key,revision,input_revision,payload_hash,receipt_id,seq,source_namespace,source_epoch,instrument,event_id,received_at,raw_text,price,ts,volume,source_coord,supersedes_revision FROM raw_events WHERE seq=?1", [seq], decode_raw_row).map_err(|e| format!("StorageUnavailable：{e}"))?;
    tb02a::enrich_raw_row(conn, &mut value)?;
    Ok(value)
}

/// 有效源位置：每个业务身份取最新 revision，按源坐标升序（晚到新版不当作追加 tick）。
fn read_effective_events(all: &[Value]) -> Result<Vec<Value>, String> {
    read_effective_events_until(all, None)
}

fn read_effective_events_until(
    all: &[Value],
    deadline: Option<std::time::Instant>,
) -> Result<Vec<Value>, String> {
    check_read_deadline(deadline)?;
    let mut latest: BTreeMap<String, Value> = BTreeMap::new();
    for ev in all {
        check_read_deadline(deadline)?;
        let ikey = ev["identity_key"].as_str().unwrap_or("").to_string();
        let rev = ev["revision"].as_i64().unwrap_or(0);
        match latest.get(&ikey) {
            None => {
                latest.insert(ikey, ev.clone());
            }
            Some(prev) => {
                if rev > prev["revision"].as_i64().unwrap_or(0) {
                    latest.insert(ikey, ev.clone());
                }
            }
        }
    }
    let mut ordered = BTreeMap::new();
    for event in latest.into_values() {
        check_read_deadline(deadline)?;
        let coord = stored_source_coord(&event)?;
        if ordered.insert(coord, event).is_some() {
            return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖业务身份".to_string());
        }
    }
    Ok(ordered.into_values().collect())
}

fn dir_str(d: Direction) -> &'static str {
    match d {
        Direction::Up => "UP",
        Direction::Down => "DOWN",
    }
}

// 参数数 >7：把对象内容寻址所需字段一次性传入；本函数只组装、不判结构。
#[allow(clippy::too_many_arguments)]
fn build_object_and_witnesses(
    merged: &[Bar],
    raw_by_coord: &BTreeMap<i64, Value>,
    merged_raws: &[Vec<i64>],
    start: usize,
    branch: &str,
    dir_ab: &str,
    dir_bc: &str,
    comparisons_json: &str,
    input_refs: &[Value],
    profile_id: &str,
    gen: i64,
    structure_cut: &str,
) -> (Value, Vec<Value>, Vec<Value>) {
    let mid = start + 1;
    let end = start + 2;
    let source_coords: Vec<String> = [start, mid, end]
        .iter()
        .map(|i| merged[*i].source_index.to_string())
        .collect();
    let object_canonical = json!({
        "branch": branch,
        "dir_ab": dir_ab,
        "dir_bc": dir_bc,
        "window_start": source_coords[0],
        "window_mid": source_coords[1],
        "window_end": source_coords[2],
        "merged_highs": [merged[start].high.to_string(), merged[mid].high.to_string(), merged[end].high.to_string()],
        "merged_lows": [merged[start].low.to_string(), merged[mid].low.to_string(), merged[end].low.to_string()],
        "comparisons": serde_json::from_str::<Value>(comparisons_json).unwrap_or(json!([])),
        "input_refs": input_refs,
        "rule_revision": RULE_REVISION,
        "profile_id": profile_id,
    });
    let object_id = format!(
        "obj-{}",
        sha256_hex(&canonical_bytes_of_value(&object_canonical))
    );

    let mut witnesses = Vec::new();
    let mut relations = Vec::new();
    for (slot, mi) in [start, mid, end].iter().enumerate() {
        let m = &merged[*mi];
        let raw_bars: Vec<Value> = merged_raws[*mi]
            .iter()
            .filter_map(|coord| raw_by_coord.get(coord).cloned())
            .collect();
        let witness_id = format!(
            "wit-{}",
            &sha256_hex(&canonical_bytes_of_value(&json!({
                "object_id": object_id,
                "slot": slot,
                "mi": mi,
                "raw_bars": raw_bars,
            })))[..16]
        );
        witnesses.push(json!({
            "witness_id": witness_id,
            "object_id": object_id,
            "slot": slot,
            "merged_source_index": m.source_index,
            "merged_high": m.high.to_string(),
            "merged_low": m.low.to_string(),
            "merged_open": m.open.to_string(),
            "merged_close": m.close.to_string(),
            "raw_bars": raw_bars,
        }));
        relations.push(json!({
            "subject": object_id,
            "relation_type": "has_witness",
            "object": witness_id,
        }));
    }
    let c0 = &source_coords[0];
    let c1 = &source_coords[1];
    let c2 = &source_coords[2];
    relations.push(json!({
        "subject": format!("window-{c0}-{c1}-{c2}"),
        "relation_type": "classified_as",
        "object": object_id,
    }));

    let all_source_coords = if profile_id == tb02a::PROFILE {
        let mut coords: Vec<i64> = input_refs
            .iter()
            .flat_map(|group| {
                ["raw_refs", "dependency_refs"]
                    .into_iter()
                    .flat_map(move |key| group[key].as_array().unwrap().iter())
            })
            .map(|raw| {
                raw["source_coord"]
                    .as_str()
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
            })
            .collect();
        coords.sort_unstable();
        coords.dedup();
        coords.iter().map(ToString::to_string).collect::<Vec<_>>()
    } else {
        source_coords.clone()
    };
    let object = json!({
        "object_id": object_id,
        "object_revision": 1,
        "kind": "CC-006.local_shape",
        "branch": branch,
        "window_start": source_coords[0].parse::<i64>().unwrap_or(0),
        "window_mid": source_coords[1].parse::<i64>().unwrap_or(0),
        "window_end": source_coords[2].parse::<i64>().unwrap_or(0),
        "dir_ab": dir_ab,
        "dir_bc": dir_bc,
        "comparisons_json": comparisons_json,
        "input_refs_json": canonical_json(&json!(input_refs)),
        "source_coords_json": canonical_json(&json!(all_source_coords)),
        "first_known_generation": gen,
        "first_known_cut": structure_cut,
        "published_generation": gen,
    });
    (object, witnesses, relations)
}

/// 已存对象的完整不可变内容读集（**含已撤回历史身份**），供 diff 逐字段对拍；提交事务内调用。
struct StoredObjects {
    /// 当前活动对象（`withdrawn_generation IS NULL`），保持既有排序（objects → facts）。
    active: Vec<Value>,
    /// 全部已存身份（活动 + 已撤回）的不可变内容，按 object_id。
    identities: BTreeMap<String, Value>,
}

fn stored_object_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "object_id": r.get::<_, String>(0)?,
        "object_revision": r.get::<_, i64>(1)?,
        "kind": r.get::<_, String>(2)?,
        "branch": r.get::<_, String>(3)?,
        "dir_ab": r.get::<_, String>(4)?,
        "dir_bc": r.get::<_, String>(5)?,
        "window_start": r.get::<_, i64>(6)?,
        "window_mid": r.get::<_, i64>(7)?,
        "window_end": r.get::<_, i64>(8)?,
        "comparisons_json": r.get::<_, String>(9)?,
        "input_refs_json": r.get::<_, String>(10)?,
        "source_coords_json": r.get::<_, String>(11)?,
        "first_known_generation": r.get::<_, i64>(12)?,
        "first_known_cut": r.get::<_, String>(13)?,
        "batch_id": r.get::<_, String>(14)?,
        "published_generation": r.get::<_, i64>(15)?,
        "withdrawn_generation": r.get::<_, Option<i64>>(16)?,
    }))
}

/// 读全部已存对象身份：不可变内容 + 四个可沿用元数据 + 撤回状态。
/// 只读 ID 不足以宣称「同 ID 相等」——同 ID 异内容必须在此暴露，不得被后面的 DO NOTHING 吞掉。
fn read_stored_objects(conn: &Connection) -> Result<StoredObjects, String> {
    // 一次读对象集只读一次全域图原件（调用方若已有外层捕获则复用）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    let mut stmt = conn
        .prepare(
            "SELECT object_id, object_revision, kind, branch, dir_ab, dir_bc,
                    window_start, window_mid, window_end,
                    comparisons_json, input_refs_json, source_coords_json,
                    first_known_generation, first_known_cut, batch_id, published_generation,
                    withdrawn_generation
             FROM objects ORDER BY window_start",
        )
        .map_err(|e| format!("prepare 读已存对象失败：{e}"))?;
    let rows = stmt
        .query_map([], stored_object_row)
        .map_err(|e| format!("query 已存对象失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect 已存对象失败：{e}"))?;
    let (facts_active, facts_withdrawn) = tb02a::read_facts(conn, None)?;
    let mut active: Vec<Value> = Vec::new();
    let mut identities: BTreeMap<String, Value> = BTreeMap::new();
    let mut insert = |o: &Value| -> Result<(), String> {
        let id = o["object_id"]
            .as_str()
            .ok_or("StorageUnavailable：已存身份缺 object_id")?
            .to_string();
        if identities.insert(id, o.clone()).is_some() {
            return Err("StorageUnavailable：两份事实表公开对象身份重复".into());
        }
        Ok(())
    };
    for o in rows
        .iter()
        .chain(facts_active.iter())
        .chain(facts_withdrawn.iter())
    {
        insert(o)?;
    }
    for o in rows.iter().chain(facts_active.iter()) {
        if o["withdrawn_generation"].is_null() {
            active.push(o.clone());
        }
    }
    Ok(StoredObjects { active, identities })
}

/// 活动对象读集（供既有调用方沿用）+ 完整身份读集。
fn read_active_objects(conn: &Connection) -> Result<Vec<Value>, String> {
    Ok(read_stored_objects(conn)?.active)
}

/// 同 ID 必须逐字段等价的不可变内容；四个可沿用元数据（first_known/batch/published）不在其中。
fn immutable_content(o: &Value, what: &str) -> Result<String, String> {
    let keys: &[&str] = if tb02a::typed(o) {
        &[
            "object_revision",
            "kind",
            "fact_key",
            "payload_json",
            "input_refs_json",
            "source_coords_json",
        ]
    } else {
        &[
            "object_revision",
            "kind",
            "branch",
            "dir_ab",
            "dir_bc",
            "window_start",
            "window_mid",
            "window_end",
            "comparisons_json",
            "input_refs_json",
            "source_coords_json",
        ]
    };
    let map = o
        .as_object()
        .ok_or_else(|| format!("StorageUnavailable：{what} 非对象"))?;
    let mut out = serde_json::Map::new();
    for key in keys {
        let value = map
            .get(*key)
            .filter(|v| !v.is_null())
            .ok_or_else(|| format!("StorageUnavailable：{what} 缺不可变字段 {key}"))?;
        out.insert((*key).to_string(), value.clone());
    }
    Ok(canonical_json(&Value::Object(out)))
}

/// 旧域取值适配：对象 → 共用键。只用 `tb02a::slot` 定槽位，不可变内容留到真正的对拍点才读
/// （`object_diff::plan` 按同 ID 命中顺序调用 `Content::read`，与旧实现同一求值点位）。
struct JsonContent<'a> {
    object: &'a Value,
    what: &'static str,
}

impl object_diff::Content for JsonContent<'_> {
    type Value = String;
    fn read(&self) -> Result<String, String> {
        immutable_content(self.object, self.what)
    }
}

fn diff_key<'a>(
    object: &'a Value,
    object_id: &str,
    what: &'static str,
) -> object_diff::Key<JsonContent<'a>> {
    object_diff::Key {
        object_id: object_id.to_string(),
        slot: tb02a::slot(object),
        immutable: JsonContent { object, what },
    }
}

/// 新对象集与既有持久身份对拍，产出 (upserts, withdrawals, replaces)。
///
/// 判据主体（同 ID 不可变内容一致 / 已撤回不得复活 / 同槽位替换 / 移除 / 真实出生）已提取到
/// `object_diff::plan`，S2 旧域与 S3 新域共用一份，不再各写一套同义判据；本函数只做旧域的取值
/// 与落字适配：
///
/// - 同一发生区间同一 ID：先核不可变内容等价 → 等价即「未变化」，不重发，元数据仍沿用既有首次值；
///   异内容 → `IdentityConflict`（不吞掉、不静默改内容、不借 `DO NOTHING` 掩盖）。
/// - 同一发生区间异 ID：撤旧 + 替代 + 新生；新区间 = 新生；旧区间消失 = 撤（防御性，append-only 域不触发）。
/// - 已撤回历史身份不得借本次发布复活。
/// - `LegacyFull`（无登记表的旧库）保持旧语义：upserts 仍是完整活动集，既有对象沿用原首次元数据。
fn compute_diff(
    new_objects: &[Value],
    stored: &StoredObjects,
    mode: delta_storage::Mode,
) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>), String> {
    // 既有身份：活动集按原主序在前（撤回/替代扫描主序），撤回身份在后；两者都进同 ID 对拍。
    // 活动表同时留作旧 full 域的首次元数据沿用（不是第二套判据，只做取值）。
    let mut active_by_id: BTreeMap<&str, &Value> = BTreeMap::new();
    let mut old_values: Vec<&Value> = Vec::with_capacity(stored.identities.len());
    let mut old_keys: Vec<object_diff::Existing<JsonContent<'_>>> =
        Vec::with_capacity(stored.identities.len());
    for old in &stored.active {
        let id = old["object_id"]
            .as_str()
            .ok_or("StorageUnavailable：活动对象缺 object_id")?;
        active_by_id.insert(id, old);
        old_values.push(old);
        old_keys.push(object_diff::Existing {
            key: diff_key(old, id, "已存身份"),
            active: true,
        });
    }
    for (id, old) in &stored.identities {
        if active_by_id.contains_key(id.as_str()) {
            continue;
        }
        old_values.push(old);
        old_keys.push(object_diff::Existing {
            key: diff_key(old, id, "已存身份"),
            active: false,
        });
    }
    let mut new_keys: Vec<object_diff::Key<JsonContent<'_>>> =
        Vec::with_capacity(new_objects.len());
    for candidate in new_objects {
        let id = candidate["object_id"]
            .as_str()
            .ok_or("StorageUnavailable：本次对象缺 object_id")?;
        new_keys.push(diff_key(candidate, id, "本次对象"));
    }
    let changes = object_diff::plan(&new_keys, &old_keys)?;

    // 落字：撤回（理由文本 + 替代指针）与替换对，顺序即 `plan` 给的索引顺序。
    let mut withdrawals: Vec<Value> = Vec::with_capacity(changes.withdrawals.len());
    let mut replaces: Vec<Value> = Vec::with_capacity(changes.replaces.len());
    for w in &changes.withdrawals {
        let old = old_values[w.old_index];
        let (reason, superseded_by) = match w.removal {
            object_diff::Removal::Removed => (
                if tb02a::typed(old) {
                    "fact_removed"
                } else {
                    "window_removed"
                },
                Value::Null,
            ),
            object_diff::Removal::Superseded => match w.new_index {
                Some(new_index) => (
                    "superseded_by_revision",
                    new_objects[new_index]["object_id"].clone(),
                ),
                // `plan` 的替代项必带新索引；本条为不可达防御，不 panic。
                None => {
                    return Err(
                        "StorageUnavailable：替代撤回缺候选索引（共用对拍不可达）".to_string()
                    )
                }
            },
        };
        withdrawals.push(tb02a::withdrawal(old, reason, superseded_by));
    }
    for r in &changes.replaces {
        replaces.push(json!({
            "new_object_id": new_objects[r.new_index]["object_id"],
            "old_object_id": old_values[r.old_index]["object_id"],
        }));
    }

    if mode.changes_only() {
        // 真实变化：只有真实出生进 upserts（未变化对象不重发）。
        let births: Vec<Value> = changes
            .births
            .iter()
            .map(|index| new_objects[*index].clone())
            .collect();
        return Ok((births, withdrawals, replaces));
    }
    // 旧 full 域：Delta 仍是完整活动集；既有对象沿用既有首次元数据/批次（逐字节同旧实现）。
    let mut upserts: Vec<Value> = Vec::with_capacity(new_objects.len());
    for candidate in new_objects {
        let oid = candidate["object_id"].as_str().unwrap_or("");
        let mut obj = candidate.clone();
        if let Some(ex) = active_by_id.get(oid) {
            for key in [
                "first_known_generation",
                "first_known_cut",
                "batch_id",
                "published_generation",
            ] {
                obj[key] = ex[key].clone();
            }
        }
        upserts.push(obj);
    }
    Ok((upserts, withdrawals, replaces))
}

/// DUR-H01：已知失败的正常收尾——仅当仍持有本 attempt 的 Begin（token/gen/frontier 精确匹配、
/// writer_epoch 未变、published generation 仍为 gen-1）时，把本 attempt 的推进占用条件清理为 idle。
fn cancel_own_begin(
    conn: &mut Connection,
    token: &str,
    gen: i64,
    frontier: i64,
    configured_epoch: &str,
) -> Result<bool, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("cancel Begin 事务失败：{e}"))?;
    let epoch = meta_get_opt(&tx, "writer_epoch")?.unwrap_or_default();
    if epoch != configured_epoch {
        drop(tx);
        return Ok(false);
    }
    let state = meta_get_opt(&tx, "advance_state")?.unwrap_or_default();
    let expect = format!("begun:{token}:{gen}:{frontier}");
    if state != expect {
        drop(tx);
        return Ok(false);
    }
    let begin_token = meta_get_opt(&tx, "begin_token")?.unwrap_or_default();
    if begin_token != token {
        drop(tx);
        return Ok(false);
    }
    let cur_gen: i64 = meta_get_opt(&tx, "generation")?
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1);
    if cur_gen != gen - 1 {
        drop(tx);
        return Ok(false);
    }
    meta_set(
        &tx,
        "last_cancelled_begin",
        &format!("{token}:{gen}:{frontier}:frontier_moved"),
    )?;
    meta_set(&tx, "advance_state", "idle")?;
    meta_set(&tx, "begin_token", "")?;
    tx.commit()
        .map_err(|e| format!("cancel Begin commit 失败：{e}"))?;
    Ok(true)
}

/// 同一 attempt 的阶段写入共用同一所有权判据，不现读新 epoch 自授予权限。
/// 两代协议共用 Begin 占用的持久元数据；调用方持有写事务并已核写者/前沿。
fn persist_begin_state(
    conn: &Connection,
    token: &str,
    gen: i64,
    frontier: i64,
) -> Result<(), String> {
    meta_set(
        conn,
        "advance_state",
        &format!("begun:{token}:{gen}:{frontier}"),
    )?;
    meta_set(conn, "begin_generation", &gen.to_string())?;
    meta_set(conn, "begin_input_frontier", &frontier.to_string())?;
    meta_set(conn, "begin_token", token)
}

fn verify_begin_owner(
    conn: &Connection,
    token: &str,
    gen: i64,
    frontier: i64,
    configured_epoch: &str,
) -> Result<(), String> {
    verify_writer_epoch(conn, configured_epoch)?;
    verify_reachable_root(conn)?;
    let expected = format!("begun:{token}:{gen}:{frontier}");
    if meta_get_opt(conn, "advance_state")?.as_deref() != Some(expected.as_str())
        || meta_get_opt(conn, "begin_token")?.as_deref() != Some(token)
        || meta_get_opt(conn, "generation")?.as_deref() != Some((gen - 1).to_string().as_str())
    {
        return Err("StaleWriter：Begin 所有权或已发布 generation 已变化".to_string());
    }
    Ok(())
}

fn verify_batch_bytes(conn: &Connection, batch_id: &str, bytes: &[u8]) -> Result<(), String> {
    let stored =
        load_batch_bytes(conn, batch_id).map_err(|e| format!("完整不可变批次不可读：{e}"))?;
    if stored.as_ref() != bytes {
        return Err(
            "IdentityConflict：不可变 batch 的规范字节或长度不符，拒绝覆写和发布".to_string(),
        );
    }
    Ok(())
}

/// C09.4 / AC4：完整对象先独立持久；此事务不发布任何对象表或结构根。
#[allow(clippy::too_many_arguments)]
fn persist_batch_before_publish(
    conn: &mut Connection,
    token: &str,
    gen: i64,
    frontier: i64,
    batch_id: &str,
    bytes: &[u8],
    batch_value: &Value,
    slots: &[evidence_store::SlotSpec],
    configured_epoch: &str,
) -> Result<(), String> {
    #[cfg(test)]
    let started = std::time::Instant::now();
    let out = persist_batch_before_publish_inner(
        conn,
        token,
        gen,
        frontier,
        batch_id,
        bytes,
        batch_value,
        slots,
        configured_epoch,
    );
    #[cfg(test)]
    cache::counter(|c| c.stage_ns += started.elapsed().as_nanos() as u64);
    out
}

#[allow(clippy::too_many_arguments)]
fn persist_batch_before_publish_inner(
    conn: &mut Connection,
    token: &str,
    gen: i64,
    frontier: i64,
    batch_id: &str,
    bytes: &[u8],
    batch_value: &Value,
    slots: &[evidence_store::SlotSpec],
    configured_epoch: &str,
) -> Result<(), String> {
    if batch_id != format!("batch-{}", sha256_hex(bytes)) {
        return Err("IdentityConflict：batch_id 与规范字节哈希不符".to_string());
    }
    let byte_len = i64::try_from(bytes.len()).map_err(|_| "batch 长度超出持久 i64 域")?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开批次持久事务失败：{e}"))?;
    verify_begin_owner(&tx, token, gen, frontier, configured_epoch)?;
    // #1394：物理域由登记表唯一决定；图域在**同一事务**内先落不可变节点与根，再落批次行。
    let graph = evidence_store::format(&tx)?.graph();
    let stored_blob: Vec<u8> = if graph {
        let staged = evidence_store::stage_complete(batch_value, slots)?;
        evidence_store::publish_refs(&tx, &evidence_store::batch_root_key(batch_id), &staged)?;
        Vec::new()
    } else {
        bytes.to_vec()
    };
    tx.execute(
        "INSERT OR IGNORE INTO batches(batch_id, canonical_bytes, byte_len) VALUES(?1, ?2, ?3)",
        params![batch_id, stored_blob, byte_len],
    )
    .map_err(|e| format!("持久 batch 失败：{e}"))?;
    verify_batch_bytes(&tx, batch_id, bytes)?;
    tx.commit()
        .map_err(|e| format!("批次持久 commit 失败：{e}"))
}

fn cmd_advance(db: &Path, configured_epoch: &str) -> Result<(), String> {
    println!("{}", advance_core(db, configured_epoch, None)?);
    Ok(())
}

fn advance_core(
    db: &Path,
    configured_epoch: &str,
    context: Option<&v2::InputContext>,
) -> Result<Value, String> {
    // #1394 第二段（2D）：一次 advance 共享一个捕获作用域（写入前会显式作废）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
    ensure_writable(&conn)?;
    if meta_get_opt(&conn, "rule_revision")?.as_deref() != Some(RULE_REVISION) {
        return Err(
            "VersionMismatch：历史规则会话只读；新笔规则须新建代际，不复用旧 Open 缓存".into(),
        );
    }
    record_connection_pragmas(&conn, "advance");
    // ── Begin：判定前先由短写事务核 writer_epoch、取得唯一推进权、持久 Begin/门/输入前沿 ──
    let (
        token,
        gen,
        frontier,
        base_cut,
        profile_id,
        profile_hash,
        catalog_revision,
        all_events,
        mode,
    ) = {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Begin 事务失败：{e}"))?;
        verify_writer_epoch(&tx, configured_epoch)?;
        let all_events = verified_root_raw_events(&tx)?;
        // 模式由登记表唯一决定：损坏（缺行/多行/未声明值）在本次任何写入之前即拒绝。
        let mode = delta_storage::read_mode(&tx)?;
        let (profile_id, profile_hash) = verify_profile_binding(&tx)?;
        let catalog_revision = meta_get_opt(&tx, "catalog_revision")?.unwrap_or_default();
        let state = meta_get_opt(&tx, "advance_state")?.unwrap_or_else(|| "idle".to_string());
        if state != "idle" {
            return Err(format!(
                "StaleWriter：已有进行中的 advance（advance_state=`{state}`）；恢复闭门，请先经 recover 合法恢复"
            ));
        }
        let cur_gen: i64 = meta_get_opt(&tx, "generation")?
            .and_then(|s| s.parse().ok())
            .ok_or("读 generation 失败")?;
        let frontier: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读输入前沿失败：{e}"))?;
        // 复用同一事务根门已校验的原始向量；结构解释仍在持久 Begin 后。
        // 幂等重试：无新输入（frontier == 已提交前沿）且已有已发布代 → 不产新 cut，返回现有已提交结果。
        // （after_commit 丢回执后同 argv 重试不再新增 generation；DeliveryUnknown 按原身份查权威结果。）
        let last_frontier: i64 = meta_i64(&tx, "last_advance_frontier")?;
        if frontier == last_frontier && cur_gen >= 1 {
            let structure_cut = meta_get_opt(&tx, "structure_cut")?.unwrap_or_default();
            let index_frontier = meta_get_opt(&tx, "index_frontier")?.unwrap_or_default();
            drop(tx);
            return Ok(json!({
                "ok": true,
                "idempotent": true,
                "generation": cur_gen.to_string(),
                "structure_cut": structure_cut,
                "index_frontier": index_frontier,
                "note": "无新输入（frontier 已提交），不产新 cut；DeliveryUnknown 请按原业务身份查询权威结果",
            }));
        }
        let gen = cur_gen + 1;
        let base_cut = meta_get_opt(&tx, "structure_cut")?.unwrap_or_else(|| "cut-0".to_string());
        let token = v2::record_begin(&tx, context, gen, frontier, &base_cut, configured_epoch)?;
        persist_begin_state(&tx, &token, gen, frontier)?;
        tx.commit().map_err(|e| format!("Begin commit 失败：{e}"))?;
        (
            token,
            gen,
            frontier,
            base_cut,
            profile_id,
            profile_hash,
            catalog_revision,
            all_events,
            mode,
        )
    };

    // ★测试暂停点 1：Begin 已持久、解释尚未开始（供 SIGKILL）。
    testonly_pause("after_begin");

    // ── 同次真实 Rust parser：按「有效源位置」在源坐标序上取每个身份的最新 revision ──
    let effective = read_effective_events(&all_events)?;
    let mut raw_by_coord: BTreeMap<i64, Value> = BTreeMap::new();
    for ev in &effective {
        let coord = stored_source_coord(ev)?;
        if raw_by_coord.insert(coord, ev.clone()).is_some() {
            return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖见证".to_string());
        }
    }

    let config = ThetaConfig::default();
    let is_ohlc = profile_id == tb02a::PROFILE;
    // #1394 2A-S：同代增量车道（库/会话/规则/档案/目录身份键；身份不符即丢弃旧车道）。
    let session_id = meta_get_opt(&conn, "session_id")?.unwrap_or_default();
    let lane_key = s_fact_cache::lane_key(
        db,
        &session_id,
        &profile_hash,
        RULE_REVISION,
        &profile_id,
        &catalog_revision,
    );
    let mut lane_guard = s_fact_cache::LaneGuard::take(&lane_key);
    let mut incr = ParseLayerIncr::new(&config);
    let mut layer = None;
    let mut raw_bars: Vec<Bar> = Vec::new();
    for ev in &effective {
        let px = parse_canonical_i64(ev["price"].as_str().unwrap_or(""), "price")?;
        let price = |key: &str| {
            parse_canonical_i64(
                ev[key].as_str().ok_or("StorageUnavailable：OHLC价缺失")?,
                key,
            )
        };
        let bar = Bar {
            source_index: usize::try_from(stored_source_coord(ev)?)
                .map_err(|_| "StorageUnavailable：源坐标不能无损转换为 source_index")?,
            timestamp: parse_canonical_i64(ev["ts"].as_str().unwrap_or(""), "timestamp")?,
            open: if is_ohlc { price("open")? } else { px },
            high: if is_ohlc { price("high")? } else { px },
            low: if is_ohlc { price("low")? } else { px },
            close: if is_ohlc { price("close")? } else { px },
            volume: 1.0,
            untradable: false,
        };
        raw_bars.push(bar);
        if !is_ohlc {
            layer = Some(incr.append(bar));
        }
    }
    let mut oracle_facts = None;
    let (layer, group_evidence) = if is_ohlc {
        if s_fact_cache::oracle_enabled() {
            // 测试开关：全量 oracle（与增量路径逐代对拍用）。
            let started = s_fact_cache::phase::begin();
            let (layer, facts) = parser::parse_layer_with_inclusion_facts(&raw_bars, &config);
            s_fact_cache::phase::record("parse_oracle", started.elapsed().as_nanos());
            let groups = parser::GroupParseEvidence {
                groups: facts.groups.clone(),
                stroke_facts: facts
                    .stroke_facts
                    .as_ref()
                    .expect("完整解析附带新笔事实")
                    .clone(),
                segment_facts: facts
                    .segment_facts
                    .as_ref()
                    .expect("完整解析附带段构造事实")
                    .clone(),
            };
            oracle_facts = Some(facts);
            (layer, Some(groups))
        } else {
            // 从**真实生效输入**的已证稳定前缀追加；前缀内变化即从最早受影响坐标失效并保守重放。
            let started = s_fact_cache::phase::begin();
            let outcome = lane_guard.lane(&lane_key).advance(&raw_bars);
            let (layer, facts) = parser::parse_layer_with_group_evidence(
                lane_guard.lane(&lane_key).state(),
                &config,
            );
            s_fact_cache::phase::record("parse", started.elapsed().as_nanos());
            let _ = outcome;
            (layer, Some(facts))
        }
    } else {
        (
            layer.unwrap_or_else(|| parser::parse_layer(&[], &config)),
            None,
        )
    };
    let merged_rc = layer.merged_bars.clone();
    let merged: &[Bar] = merged_rc.as_ref();

    let wins = classify_local_shape_sliding(merged);

    // 同次包含 trace 提供完整成员；旧 tick 保留原映射。
    let mut merged_raws: Vec<Vec<i64>> = vec![Vec::new(); merged.len()];
    if let Some(facts) = &group_evidence {
        for (i, group) in facts.groups.iter().enumerate() {
            merged_raws[i] = group.members.iter().map(|x| *x as i64).collect();
        }
    } else {
        for coord in raw_by_coord.keys().copied() {
            let source_index =
                usize::try_from(coord).map_err(|_| "StorageUnavailable：源坐标超界")?;
            if let Some(g) = parser::inclusion::merged_group_index(merged, source_index) {
                merged_raws[g].push(coord);
            }
        }
    }

    let mut objects: Vec<Value> = Vec::new();
    let mut witnesses: Vec<Value> = Vec::new();
    let mut relations: Vec<Value> = Vec::new();
    let mut observations: Vec<Value> = Vec::new();

    let mut raw_violations: Vec<(usize, Cc006DomainViolation)> = Vec::new();
    for i in 0..if is_ohlc {
        0
    } else {
        raw_bars.len().saturating_sub(2)
    } {
        if let Some(reason) =
            window_domain_violation(&raw_bars[i], &raw_bars[i + 1], &raw_bars[i + 2])
        {
            raw_violations.push((i, reason));
        }
    }

    if !is_ohlc && raw_bars.len() < 3 {
        observations.push(json!({
            "kind": "insufficient_knowledge",
            "reason": "fewer_than_three_bars",
            "window_start": null,
            "window_mid": null,
            "window_end": null,
            "detail": {"raw_events": effective.len()},
        }));
    }
    for (i, reason) in &raw_violations {
        observations.push(json!({
            "kind": "domain_not_satisfied",
            "reason": reason.as_str(),
            "window_start": *i,
            "window_mid": *i + 1,
            "window_end": *i + 2,
            "detail": {
                "raw_prices": [
                    raw_bars[*i].high.to_string(),
                    raw_bars[*i + 1].high.to_string(),
                    raw_bars[*i + 2].high.to_string(),
                ],
            },
        }));
    }

    for (start, shape) in &wins {
        let start = *start;
        // #1373 同角色极值身份仍有竞争时，不给依赖该组的分型选根。
        if group_evidence.as_ref().is_some_and(|f| {
            f.groups[start..start + 3]
                .iter()
                .any(|g| g.high_sources.len() != 1 || g.low_sources.len() != 1)
        }) {
            continue;
        }
        match shape {
            Cc006LocalShape::Classified {
                branch,
                dir_ab,
                dir_bc,
                comparisons,
            } => {
                let dir_ab_s = dir_str(*dir_ab);
                let dir_bc_s = dir_str(*dir_bc);
                let cmp_json = json!(comparisons
                    .iter()
                    .map(|c| json!({
                        "axis": c.axis.as_str(),
                        "pair": c.pair.as_str(),
                        "prev": c.prev.to_string(),
                        "cur": c.cur.to_string(),
                        "strict_up": c.strict_up,
                        "held": c.held,
                    }))
                    .collect::<Vec<_>>())
                .to_string();

                let input_refs: Vec<Value> = [start, start + 1, start + 2]
                    .iter()
                    .map(|mi| {
                        let refs: Vec<Value> = merged_raws[*mi]
                            .iter()
                            .filter_map(|coord| raw_by_coord.get(coord))
                            .map(|ev| {
                                json!({
                                    "identity_key": ev["identity_key"],
                                    "payload_hash": ev["payload_hash"],
                                    "receipt_id": ev["receipt_id"],
                                    "event_id": ev["event_id"],
                                    "input_revision": ev["input_revision"],
                                    "revision": ev["revision"],
                                    "seq": ev["seq"],
                                    "source_coord": ev["source_coord"],
                                })
                            })
                            .collect();
                        let mut group_refs=json!({ "merged_index": mi, "raw_refs": refs });
                        if let Some(facts)=&group_evidence {
                            let group=&facts.groups[*mi];
                            let mut dependency_coords=group.direction_evidence.as_ref().map(|e|e.source_coords.clone()).unwrap_or_default();
                            // #1373：三组当前划分还依赖关闭边界的真实后继，不能只绑定价格算式。
                            if let Some(boundary)=&group.confirmation_evidence {dependency_coords.extend(&boundary.source_coords);}
                            dependency_coords.retain(|coord|!group.members.contains(coord));
                            dependency_coords.sort_unstable();dependency_coords.dedup();
                            let dependencies:Vec<Value>=dependency_coords.iter().map(|coord|{
                                let ev=&raw_by_coord[&(*coord as i64)];
                                json!({"identity_key":ev["identity_key"],"payload_hash":ev["payload_hash"],"receipt_id":ev["receipt_id"],
                                    "event_id":ev["event_id"],"input_revision":ev["input_revision"],"revision":ev["revision"],
                                    "seq":ev["seq"],"source_coord":ev["source_coord"]})
                            }).collect();
                            group_refs["dependency_refs"]=json!(dependencies);
                        }
                        group_refs
                    })
                    .collect();

                let (object, wits, rels) = build_object_and_witnesses(
                    merged,
                    &raw_by_coord,
                    &merged_raws,
                    start,
                    branch.as_str(),
                    dir_ab_s,
                    dir_bc_s,
                    &cmp_json,
                    &input_refs,
                    &profile_id,
                    gen,
                    &format!("cut-{gen}"),
                );
                objects.push(object);
                witnesses.extend(wits);
                relations.extend(rels);
            }
            Cc006LocalShape::DomainNotSatisfied { reason } => {
                observations.push(json!({
                    "kind": "domain_not_satisfied",
                    "reason": reason.as_str(),
                    "window_start": start,
                    "window_mid": start + 1,
                    "window_end": start + 2,
                    "detail": {},
                }));
            }
            Cc006LocalShape::InsufficientKnowledge => {
                observations.push(json!({
                    "kind": "insufficient_knowledge",
                    "reason": "fewer_than_three_bars",
                    "window_start": start,
                    "window_mid": null,
                    "window_end": null,
                    "detail": {},
                }));
            }
        }
    }

    let structure_cut = format!("cut-{gen}");
    if let Some(facts) = &group_evidence {
        // #1394 2A-S/2C：同代增量状态 + 稳定对象原件复用 + 比较单元缓存（与全量路径逐字段相同）。
        let started = s_fact_cache::phase::begin();
        let (typed, waiting) = if let Some(full) = &oracle_facts {
            tb02a_facts::build(full, &raw_by_coord, &objects, gen, &structure_cut, frontier)
        } else {
            lane_guard.lane(&lane_key).build_typed(
                &facts.groups,
                &raw_by_coord,
                &objects,
                gen,
                &structure_cut,
                frontier,
            )
        };
        s_fact_cache::phase::record("facts", started.elapsed().as_nanos());
        objects.extend(typed);
        observations.extend(waiting);
        let (new_bi, edges) = tb02b::build(
            &facts.stroke_facts,
            &raw_by_coord,
            &all_events,
            &read_active_objects(&conn)?,
            gen,
            &structure_cut,
            frontier,
            v2::commit_time(&conn, context)?,
        );
        objects.extend(new_bi);
        relations.extend(edges);
        let (segments, edges) = tb02c::build(
            &facts.segment_facts,
            &facts.stroke_facts,
            &raw_by_coord,
            &all_events,
            &read_active_objects(&conn)?,
            gen,
            &structure_cut,
            frontier,
            v2::commit_time(&conn, context)?,
        );
        objects.extend(segments);
        relations.extend(edges);
    }

    // 去重：同一 (kind, window, reason) 只发布一次。
    {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        observations.retain(|ob| {
            let key = if ob["detail"]["tb02a"] == true {
                canonical_json(ob)
            } else {
                format!(
                    "{}|{:?}|{:?}|{:?}|{}",
                    ob["kind"],
                    ob["window_start"],
                    ob["window_mid"],
                    ob["window_end"],
                    ob["reason"]
                )
            };
            seen.insert(key)
        });
    }

    let structure_cut = format!("cut-{gen}");
    // ── 内容寻址批次：封存完整正式结果（全部修订 + 有效源位置 + 对象 + 见证 + 关系 + 观察 + 绑定）──
    let mut batch_json = json!({
        "generation": gen,
        "structure_cut": structure_cut,
        "input_frontier": frontier,
        "catalog_revision": catalog_revision,
        "profile_id": profile_id,
        "profile_hash": profile_hash,
        "rule_revision": RULE_REVISION,
        "raw_events": all_events,
        "effective_events": effective,
        "objects": objects,
        "witnesses": witnesses,
        "relations": relations,
        "observations": observations,
        "scope": {"structure": "CompleteCut", "economic": "not_started"},
    });
    if is_ohlc {
        let previous_frontier = meta_get_opt(&conn, "last_advance_frontier")?
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(-1);
        let started = s_fact_cache::phase::begin();
        let (_, withdrawals, replaces) =
            compute_diff(&objects, &read_stored_objects(&conn)?, mode)?;
        batch_json["catalog_axes"] = tb02a_facts::catalog_axes(
            tb02a_facts::CatalogAxisTable::for_rule_revision(RULE_REVISION)
                .expect("v2 写者的规则修订必有目录轴表"),
            &objects,
            &all_events,
            previous_frontier,
            &withdrawals,
            &replaces,
        );
        s_fact_cache::phase::record("catalog_axes", started.elapsed().as_nanos());
    }
    let started = s_fact_cache::phase::begin();
    v2::seal_batch(&conn, context, &mut batch_json)?;
    let batch_bytes = canonical_bytes_of_value(&batch_json);
    let batch_id = format!("batch-{}", sha256_hex(&batch_bytes));
    s_fact_cache::phase::record("batch_materialize", started.elapsed().as_nanos());
    for o in &mut objects {
        o["batch_id"] = json!(batch_id);
    }

    // #1394 第二段：代际参数槽只含**本代生成态**标量（数值按规范十进制、字符串按同一
    // serializer 的带引号字节双匹配）。同代全部对象共用同一批槽值，故同一模板跨代共享。
    let mut evidence_slots = vec![
        evidence_store::SlotSpec::number(&gen.to_string()),
        evidence_store::SlotSpec::number(&frontier.to_string()),
        evidence_store::SlotSpec::string(&structure_cut),
    ];
    if let Some(ns) = batch_json["semantic_commit_ns"].as_str() {
        evidence_slots.push(evidence_store::SlotSpec::string(ns));
    }

    let started = s_fact_cache::phase::begin();
    persist_batch_before_publish(
        &mut conn,
        &token,
        gen,
        frontier,
        &batch_id,
        &batch_bytes,
        &batch_json,
        &evidence_slots,
        configured_epoch,
    )?;
    s_fact_cache::phase::record("persist_batch", started.elapsed().as_nanos());
    let started_commit = s_fact_cache::phase::begin();

    // ★测试暂停点 2：完整不可变批次已独立持久、Commit 尚未发生（供 SIGKILL）。
    testonly_pause("after_batch");

    // ── Commit：核 Begin/epoch/前沿仍有效后，同一事务原子应用对象/边/端点 + 撤回/替代 + delta ──
    let classified_count = objects.len();
    let mut frontier_moved = false;
    #[cfg(test)]
    let commit_started = std::time::Instant::now();
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开 commit 事务失败：{e}"))?;
    {
        verify_begin_owner(&tx, &token, gen, frontier, configured_epoch)?;
        verify_batch_bytes(&tx, &batch_id, &batch_bytes)?;
        let cur_frontier: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读输入前沿失败：{e}"))?;
        if cur_frontier != frontier {
            frontier_moved = true;
        } else {
            // 与既有持久身份对拍（提交事务内，原子）：同 ID 先核不可变内容，异内容即 IdentityConflict。
            // 未变化对象沿用既有 first_known（不把重发布当新获知）；新对象 first_known = 本 gen。
            let (delta_upserts, withdrawals, replaces) =
                compute_diff(&objects, &read_stored_objects(&tx)?, mode)?;

            // 1) 撤回：当前视图移除，历史不删（UPDATE 生命周期，保留原身份/first_known/发生区间）。
            {
                let mut stmt = tx
                    .prepare(
                        "UPDATE objects SET withdrawn_generation=?1, withdrawal_reason=?2, superseded_by=?3 WHERE object_id=?4 AND withdrawn_generation IS NULL",
                    )
                    .map_err(|e| format!("prepare 撤回失败：{e}"))?;
                for w in &withdrawals {
                    let superseded = w["superseded_by"].as_str().map(|s| s.to_string());
                    if tb02a::typed(w) {
                        tx.execute("UPDATE structure_facts SET withdrawn_generation=?1,withdrawal_reason=?2,superseded_by=?3 WHERE object_id=?4 AND withdrawn_generation IS NULL",params![gen,w["reason"].as_str(),superseded,w["object_id"].as_str()]).map_err(|e|e.to_string())?;
                        continue;
                    }
                    stmt.execute(params![
                        gen,
                        w["reason"].as_str().unwrap_or(""),
                        superseded,
                        w["object_id"].as_str().unwrap_or(""),
                    ])
                    .map_err(|e| format!("撤回对象失败：{e}"))?;
                }
            }

            // 2) upsert：新对象插入（首次获知固定）；既有对象仅推进 published_generation/batch。
            {
                let mut stmt = tx
                .prepare(
                    "INSERT INTO objects(object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc,
                       window_start, window_mid, window_end, comparisons_json, input_refs_json,
                       source_coords_json, first_known_generation, first_known_cut, published_generation)
                     VALUES(?1, 1, 'CC-006.local_shape', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                     ON CONFLICT(object_id) DO NOTHING",
                )
                .map_err(|e| format!("prepare objects 失败：{e}"))?;
                // 只写真正新生/替代的对象（旧 full 域下 delta_upserts 仍是完整活动集，逐字节同旧实现）。
                for o in &delta_upserts {
                    if tb02a::typed(o) {
                        tb02a::save_fact(&tx, o, &batch_id)?;
                        continue;
                    }
                    stmt.execute(params![
                        o["object_id"].as_str().unwrap_or(""),
                        batch_id.as_str(),
                        o["branch"].as_str().unwrap_or(""),
                        o["dir_ab"].as_str().unwrap_or(""),
                        o["dir_bc"].as_str().unwrap_or(""),
                        o["window_start"].as_i64().unwrap_or(0),
                        o["window_mid"].as_i64().unwrap_or(0),
                        o["window_end"].as_i64().unwrap_or(0),
                        o["comparisons_json"].as_str().unwrap_or("[]"),
                        o["input_refs_json"].as_str().unwrap_or("[]"),
                        o["source_coords_json"].as_str().unwrap_or("[]"),
                        o["first_known_generation"].as_i64().unwrap_or(0),
                        o["first_known_cut"].as_str().unwrap_or(""),
                        o["published_generation"].as_i64().unwrap_or(0),
                    ])
                    .map_err(|e| format!("insert objects 失败：{e}"))?;
                }
            }

            // 3) 见证 / 关系 / 观察（内容寻址，幂等插入）。
            // changes-only 域：各族 Delta 只含本代首次发布成员；旧 full 域保持完整集（与旧实现逐字节相同）。
            let mut first_witnesses: Vec<Value> = Vec::new();
            {
                let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO witnesses(witness_id, object_id, slot, merged_source_index, merged_high,
                       merged_low, merged_open, merged_close, raw_json, published_generation)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(|e| format!("prepare witnesses 失败：{e}"))?;
                for w in &witnesses {
                    let inserted = stmt
                        .execute(params![
                            w["witness_id"].as_str().unwrap_or(""),
                            w["object_id"].as_str().unwrap_or(""),
                            w["slot"].as_i64().unwrap_or(0),
                            w["merged_source_index"].as_i64().unwrap_or(0),
                            w["merged_high"].as_str().unwrap_or(""),
                            w["merged_low"].as_str().unwrap_or(""),
                            w["merged_open"].as_str().unwrap_or(""),
                            w["merged_close"].as_str().unwrap_or(""),
                            serde_json::to_string(&w["raw_bars"])
                                .unwrap_or_else(|_| "[]".to_string()),
                            gen,
                        ])
                        .map_err(|e| format!("insert witnesses 失败：{e}"))?;
                    if inserted == 1 {
                        first_witnesses.push(w.clone());
                    }
                }
            }
            // 关系集合：has_witness / classified_as + replaces（替代边）+ raw supersedes（源修订链）。
            let mut rels: Vec<Value> = relations.clone();
            for rp in &replaces {
                rels.push(json!({
                    "subject": rp["new_object_id"],
                    "relation_type": "replaces",
                    "object": rp["old_object_id"],
                }));
            }
            for ev in &all_events {
                if let Some(sup) = ev["supersedes_revision"].as_i64() {
                    rels.push(json!({
                        "subject": format!("{}@{}", ev["identity_key"].as_str().unwrap_or(""), ev["revision"].as_i64().unwrap_or(0)),
                        "relation_type": "supersedes",
                        "object": format!("{}@{sup}", ev["identity_key"].as_str().unwrap_or("")),
                    }));
                }
            }
            {
                // 关系：has_witness / classified_as + replaces（替代边）+ raw supersedes（源修订链）。
                let mut stmt = tx
                    .prepare(
                        "INSERT OR IGNORE INTO relations(relation_id, subject, relation_type, object, published_generation) VALUES(?1, ?2, ?3, ?4, ?5)",
                    )
                    .map_err(|e| format!("prepare relations 失败：{e}"))?;
                let mut first_relations: Vec<Value> = Vec::new();
                for r in &rels {
                    let rid = format!(
                        "rel-{}",
                        &sha256_hex(&canonical_bytes_of_value(&json!({
                            "s": r["subject"],
                            "t": r["relation_type"],
                            "o": r["object"],
                        })))[..16]
                    );
                    let inserted = stmt
                        .execute(params![
                            rid,
                            r["subject"].as_str().unwrap_or(""),
                            r["relation_type"].as_str().unwrap_or(""),
                            r["object"].as_str().unwrap_or(""),
                            gen,
                        ])
                        .map_err(|e| format!("insert relations 失败：{e}"))?;
                    if inserted == 1 {
                        first_relations.push(r.clone());
                    }
                }
                rels = if mode.changes_only() {
                    first_relations
                } else {
                    rels
                };
            }
            // 观察：补 identity/batch，供 delta 与同 cut Snapshot 逐字段对齐（INSERT OR IGNORE：首获知固定）。
            let mut obs_with_id: Vec<Value> = Vec::with_capacity(observations.len());
            let mut first_observations: Vec<Value> = Vec::new();
            for ob in &observations {
                let oid = observation_id(ob);
                let stored: Option<Value> = tx.query_row(
                    "SELECT observation_id,batch_id,kind,window_start,window_mid,window_end,reason,detail_json FROM observations WHERE observation_id=?1",
                    params![oid], |r| Ok(json!({
                        "observation_id":r.get::<_,String>(0)?, "batch_id":r.get::<_,String>(1)?,
                        "kind":r.get::<_,String>(2)?, "window_start":r.get::<_,Option<i64>>(3)?,
                        "window_mid":r.get::<_,Option<i64>>(4)?, "window_end":r.get::<_,Option<i64>>(5)?,
                        "reason":r.get::<_,String>(6)?, "detail":json_column(r,7,JsonShape::Object)?,
                    }))).optional().map_err(|e| format!("读观察首版失败：{e}"))?;
                let first = stored.is_none();
                let o = if let Some(stored) = stored {
                    stored
                } else {
                    let mut fresh = ob.clone();
                    fresh["observation_id"] = json!(oid);
                    fresh["batch_id"] = json!(batch_id);
                    fresh
                };
                if first {
                    first_observations.push(o.clone());
                }
                obs_with_id.push(o);
            }
            {
                let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO observations(observation_id, batch_id, kind, window_start, window_mid,
                       window_end, reason, detail_json, published_generation)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .map_err(|e| format!("prepare observations 失败：{e}"))?;
                for ob in &obs_with_id {
                    stmt.execute(params![
                        ob["observation_id"].as_str().unwrap_or(""),
                        ob["batch_id"].as_str().unwrap_or(""),
                        ob["kind"].as_str().unwrap_or(""),
                        ob["window_start"].as_i64(),
                        ob["window_mid"].as_i64(),
                        ob["window_end"].as_i64(),
                        ob["reason"].as_str().unwrap_or(""),
                        serde_json::to_string(&ob["detail"]).unwrap_or_else(|_| "{}".to_string()),
                        gen,
                    ])
                    .map_err(|e| format!("insert observations 失败：{e}"))?;
                }
            }

            // 4) 持久 StructureDelta（session/generation、catalog_revision、base_cut/next_cut、
            //    seq_range、完整 upsert/撤回/替代、关系/见证、index_frontier）。
            let prev_frontier: i64 = meta_get_opt(&tx, "last_advance_frontier")?
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            let seq_range = json!({
                "from": (prev_frontier + 1).to_string(),
                "to": frontier.to_string(),
            });
            // 本代目录运行证据（随代持久，供 AsKnown 历史 cut 复现；不引用未来 cut）。
            let run_status = if classified_count >= 1 {
                "run"
            } else {
                "not_run"
            };
            let mut evidence = json!({
                "tested_domain": "TestOnly tick 1:1 OHLC（O=H=L=C），相邻严格无包含，初始化方向由前两点确定，无同价端点竞争",
                "profile_id": profile_id,
                "profile_hash": profile_hash,
                "rule_revision": RULE_REVISION,
                "batch_id": batch_id,
                "structure_cut": structure_cut,
                "classified_objects": classified_count.to_string(),
                "windows_total": wins.len().to_string(),
                "merged_bars": merged.len().to_string(),
                "effective_source_positions": effective.len().to_string(),
                "withdrawals": withdrawals.len().to_string(),
                "replaces": replaces.len().to_string(),
                "insufficient_knowledge": observations.iter().filter(|o| o["kind"] == "insufficient_knowledge").count().to_string(),
                "domain_not_satisfied": observations.iter().filter(|o| o["kind"] == "domain_not_satisfied").count().to_string(),
                "scope": {"structure": "CompleteCut", "economic": "not_started"},
            });
            if is_ohlc {
                evidence["tested_domain"]=json!("TB-02-A：显式规范整数OHLC；已建方向与无同角色极值身份竞争域；初始包含/tie具名等待");
                evidence["axes"] = batch_json["catalog_axes"].clone();
            }
            // wire 形态：整数坐标/版本 → 规范十进制字符串，与同 cut Snapshot 逐字段对齐。
            let wire_upserts: Vec<Value> = delta_upserts
                .iter()
                .map(project_object_wire)
                .collect::<Result<_, _>>()?;
            let wire_withdrawals: Vec<Value> = withdrawals
                .iter()
                .map(project_withdrawal_wire)
                .collect::<Result<_, _>>()?;
            // changes-only 域：三族 Delta 只含本代首次发布（与独立索引 born[gen] 强相等）；
            // 旧 full 域仍是完整集。封存 full batch 自身保持完整（本段不改 batch 形态）。
            let (wire_witnesses, wire_observations) = if mode.changes_only() {
                (
                    first_witnesses
                        .iter()
                        .map(project_witness_wire)
                        .collect::<Result<Vec<_>, _>>()?,
                    first_observations
                        .iter()
                        .map(project_observation_wire)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                (
                    witnesses
                        .iter()
                        .map(project_witness_wire)
                        .collect::<Result<Vec<_>, _>>()?,
                    obs_with_id
                        .iter()
                        .map(project_observation_wire)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            };
            // 本代新接纳的源事件（seq ∈ seq_range），供 Watch 消费者重建 raw_history（不另轮询）。
            let wire_raw_history_added: Vec<Value> = all_events
                .iter()
                .filter(|ev| ev["seq"].as_i64().unwrap_or(-1) > prev_frontier)
                .map(project_raw_event_wire)
                .collect::<Result<_, _>>()?;
            let delta_json = json!({
                "session_id": meta_get_opt(&tx, "session_id")?.unwrap_or_default(),
                "generation": gen.to_string(),
                "catalog_revision": catalog_revision,
                "base_cut": base_cut,
                "next_cut": structure_cut,
                "seq_range": seq_range,
                "input_frontier": frontier.to_string(),
                "index_frontier": batch_id,
                "catalog_run_status": run_status,
                "catalog_evidence": evidence,
                "upserts": wire_upserts,
                "withdrawals": wire_withdrawals,
                "replaces": replaces,
                "witnesses": wire_witnesses,
                "relations": rels,
                "observations": wire_observations,
                "raw_history_added": wire_raw_history_added,
            });
            // #1394 第二段：Delta 原件与批次同图。图域先落不可变节点与 `delta/<generation>` 根
            // （同一事务），行内 delta_json 只留空占位；整块域保持原逐字节行为。
            let delta_text = canonical_json(&delta_json);
            let stored_delta = if evidence_store::format(&tx)?.records_in_graph() {
                let delta_slots = vec![
                    evidence_store::SlotSpec::string(&gen.to_string()),
                    evidence_store::SlotSpec::string(&frontier.to_string()),
                    evidence_store::SlotSpec::string(&base_cut),
                    evidence_store::SlotSpec::string(&structure_cut),
                    evidence_store::SlotSpec::string(&batch_id),
                ];
                let staged = evidence_store::stage_complete(&delta_json, &delta_slots)?;
                evidence_store::publish_refs(&tx, &evidence_store::delta_root_key(gen), &staged)?;
                String::new()
            } else {
                delta_text.clone()
            };
            tx.execute(
                "INSERT OR REPLACE INTO structure_deltas(generation, session_id, catalog_revision, base_cut, next_cut, seq_range_json, index_frontier, input_frontier, catalog_run_status, catalog_evidence_json, delta_json)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    gen,
                    meta_get_opt(&tx, "session_id")?.unwrap_or_default(),
                    catalog_revision,
                    base_cut,
                    structure_cut,
                    seq_range.to_string(),
                    batch_id,
                    frontier,
                    run_status,
                    evidence.to_string(),
                    stored_delta,
                ],
            )
            .map_err(|e| format!("持久 delta 失败：{e}"))?;
            if evidence_store::format(&tx)?.records_in_graph() {
                // 写后立刻重读核：行内必须是空占位，私有根必须无损展开成同一份原件。
                let row: String = tx
                    .query_row(
                        "SELECT delta_json FROM structure_deltas WHERE generation=?1",
                        params![gen],
                        |r| r.get(0),
                    )
                    .map_err(|e| format!("读回 delta 失败：{e}"))?;
                if !row.is_empty() {
                    return Err("StorageUnavailable：图域 delta 行仍带整块原文，拒绝发布".into());
                }
                let mut reader = evidence_store::Reader::new(&tx, EVIDENCE_READ_BUDGET);
                let bytes = reader.read_fixed_cut(&evidence_store::delta_root_key(gen))?;
                if bytes.as_ref() != delta_text.as_bytes() {
                    return Err(format!(
                        "IdentityConflict：delta[{gen}] 私有根展开与本次原件不符，拒绝发布"
                    ));
                }
            }

            // 5) 索引可达根：generation / structure_cut / index_frontier（与对象同事务生效）。
            meta_set(&tx, "generation", &gen.to_string())?;
            meta_set(&tx, "structure_cut", &structure_cut)?;
            meta_set(&tx, "index_frontier", &batch_id)?;
            meta_set(&tx, "last_advance_frontier", &frontier.to_string())?;

            if is_ohlc {
                for (axis, value) in batch_json["catalog_axes"].as_object().ok_or("目录轴缺失")?
                {
                    tx.execute("UPDATE catalog SET impl_status=?1,proof_status=?2,run_status=?3,evidence_json=?4 WHERE catalog_id=?5",params![value["impl_status"].as_str(),value["proof_status"].as_str(),value["run_status"].as_str(),canonical_json(&value["evidence"]),axis]).map_err(|e|e.to_string())?;
                }
            } else {
                // 6) 目录状态：实现/证明/运行分列（与 delta 同代证据一致）。
                tx.execute(
            "UPDATE catalog SET impl_status='implemented', proof_status='not_proved', run_status=?1, evidence_json=?2 WHERE catalog_id='CC-006'",
            params![run_status, evidence.to_string()],
        )
        .map_err(|e| format!("update CC-006 catalog 失败：{e}"))?;
                tx.execute(
                    "UPDATE catalog SET run_status='not_run' WHERE catalog_id <> 'CC-006'",
                    [],
                )
                .map_err(|e| format!("update 其余 catalog 失败：{e}"))?;
            }

            v2::record_commit(&tx, context, gen, frontier)?;
            meta_set(&tx, "advance_state", "idle")?;
        }
    }
    if frontier_moved {
        drop(tx);
    } else {
        testonly_pause("before_commit");
        tx.commit()
            .map_err(|e| format!("advance commit 失败：{e}"))?;
    }
    s_fact_cache::phase::record("commit", started_commit.elapsed().as_nanos());
    #[cfg(test)]
    cache::counter(|c| c.commit_ns += commit_started.elapsed().as_nanos() as u64);

    if frontier_moved {
        let released = cancel_own_begin(&mut conn, &token, gen, frontier, configured_epoch)?;
        let disposition = if released {
            "本 attempt 已正常取消并释放推进权"
        } else {
            "Begin 所有权已变化，未清理推进占用"
        };
        return Err(format!(
            "StaleWriter：输入前沿已由 {frontier} 变化（新输入已接纳，请重新 advance）；{disposition}"
        ));
    }

    // ★测试暂停点 3：Commit 已持久、回执尚未送达（供 SIGKILL）。
    testonly_pause("after_commit");

    Ok(json!({
        "ok": true,
        "generation": gen.to_string(),
        "structure_cut": structure_cut,
        "batch_id": batch_id,
        "merged_bars": merged.len(),
        "raw_events": all_events.len(),
        "effective_source_positions": effective.len(),
        "windows_total": wins.len(),
        "objects_published": objects.len(),
        "observations": {
            "insufficient_knowledge": observations.iter().filter(|o| o["kind"] == "insufficient_knowledge").count(),
            "domain_not_satisfied": observations.iter().filter(|o| o["kind"] == "domain_not_satisfied").count(),
        },
        "batch_byte_len": batch_bytes.len(),
        "scope": {"structure": "CompleteCut", "economic": "not_started"},
    }))
}

// ────────────────────────────────────────────────────────────────────────────
// 规范字节：对 serde_json::Value 递归排序对象键，输出紧凑 JSON（无空白）。
// ────────────────────────────────────────────────────────────────────────────

fn canonical_bytes_of_value(v: &Value) -> Vec<u8> {
    canonical_json(v).into_bytes()
}

/// 最终 wire 边界的安全递归精确整数投影：JSON 整数（i64/u64）→ 规范十进制字符串；
/// bool 仍 bool、null 仍 null、浮点/非整数原值保持。用于已发布历史/Delta/Catalog 的读取输出，
/// 不改变内部存储/已发布 DB/BLOB/hash/cut/first_known。
fn project_wire_integers(v: &Value) -> Value {
    match v3::business_json::project_wire_integers_with_owner(v.into(), None)
        .expect("普通 wire 整数投影")
    {
        v3::business_json::JsonOwner::Plain(v) => {
            v.into_unscoped().expect("None wire入口不持Native许可")
        }
        #[cfg(feature = "s_allocation_json_testonly")]
        v3::business_json::JsonOwner::Owned(_) => unreachable!("None入口不返回owned"),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// WIRE 精确整数投影：把「明确外发」的精确坐标/版本/来源游标字段投影为规范十进制字符串。
// ────────────────────────────────────────────────────────────────────────────

fn num_to_str(v: &Value) -> Result<Value, String> {
    v.as_i64()
        .map(|n| Value::String(n.to_string()))
        .ok_or_else(|| "持久整数必须是 i64，不能是浮点、布尔、空值或文本".to_string())
}

/// `objects[].input_refs`：merged_index、raw_refs[].seq、raw_refs[].revision → 字符串。
fn project_input_refs(refs: &Value) -> Result<Value, String> {
    let groups = refs.as_array().ok_or("input_refs 必须是数组")?;
    let mut out = Vec::with_capacity(groups.len());
    for group in groups {
        let mut obj = group
            .as_object()
            .ok_or("input_refs 成员必须是对象")?
            .clone();
        let index = num_to_str(obj.get("merged_index").unwrap_or(&Value::Null))?;
        obj.insert("merged_index".to_string(), index);
        for key in ["raw_refs", "dependency_refs"] {
            if key == "dependency_refs" && !obj.contains_key(key) {
                continue;
            }
            let raw_refs = obj
                .get_mut(key)
                .and_then(Value::as_array_mut)
                .ok_or("input_refs成员来源必须为数组")?;
            for raw in raw_refs {
                let raw = raw.as_object_mut().ok_or("input_refs来源成员必须对象")?;
                let seq = num_to_str(raw.get("seq").unwrap_or(&Value::Null))?;
                raw.insert("seq".to_string(), seq);
                if raw.contains_key("revision") {
                    let rev = num_to_str(raw.get("revision").unwrap_or(&Value::Null))?;
                    raw.insert("revision".to_string(), rev);
                }
            }
        }
        out.push(Value::Object(obj));
    }
    Ok(Value::Array(out))
}

/// `witnesses[].raw_bars`：seq、revision → 字符串。
fn project_raw_bars(bars: &Value) -> Result<Value, String> {
    let bars = bars.as_array().ok_or("raw_bars 必须是数组")?;
    let mut out = Vec::with_capacity(bars.len());
    for bar in bars {
        let mut obj = bar.as_object().ok_or("raw_bars 成员必须是对象")?.clone();
        let seq = num_to_str(obj.get("seq").unwrap_or(&Value::Null))?;
        obj.insert("seq".to_string(), seq);
        if obj.contains_key("revision") {
            let rev = num_to_str(obj.get("revision").unwrap_or(&Value::Null))?;
            obj.insert("revision".to_string(), rev);
        }
        if obj.contains_key("supersedes_revision") {
            let sup = obj.get("supersedes_revision").unwrap();
            if sup.is_i64() {
                obj.insert("supersedes_revision".to_string(), num_to_str(sup)?);
            } else if !sup.is_null() {
                return Err("raw_bars.supersedes_revision 必须是 i64 或 null".to_string());
            }
        }
        let mut wire = Value::Object(obj);
        tb02a::raw_wire(&mut wire, bar);
        out.push(wire);
    }
    Ok(Value::Array(out))
}

/// i64 形态对象 → wire 形态（与 read_snapshot 投影一致）。
fn project_object_wire(o: &Value) -> Result<Value, String> {
    if tb02a::typed(o) {
        return tb02a::project(o);
    }
    let comparisons = json_shape(
        o["comparisons_json"].as_str().unwrap_or("[]"),
        JsonShape::Array,
    )?;
    let input_refs = json_shape(
        o["input_refs_json"].as_str().unwrap_or("[]"),
        JsonShape::Array,
    )?;
    let source_coords = json_shape(
        o["source_coords_json"].as_str().unwrap_or("[]"),
        JsonShape::Array,
    )?;
    Ok(json!({
        "object_id": o["object_id"].clone(),
        "object_revision": num_to_str(&o["object_revision"])?,
        "kind": o["kind"].clone(),
        "batch_id": o.get("batch_id").cloned().unwrap_or(Value::Null),
        "branch": o["branch"].clone(),
        "dir_ab": o["dir_ab"].clone(),
        "dir_bc": o["dir_bc"].clone(),
        "window_start": num_to_str(&o["window_start"])?,
        "window_mid": num_to_str(&o["window_mid"])?,
        "window_end": num_to_str(&o["window_end"])?,
        "comparisons": comparisons,
        "input_refs": project_input_refs(&input_refs)?,
        "source_coords": source_coords,
        "first_known_generation": num_to_str(&o["first_known_generation"])?,
        "first_known_cut": o["first_known_cut"].clone(),
        "published_generation": num_to_str(&o["published_generation"])?,
        "withdrawn_generation": o
            .get("withdrawn_generation")
            .and_then(|v| v.as_i64())
            .map(|v| Value::String(v.to_string()))
            .unwrap_or(Value::Null),
        "withdrawal_reason": o.get("withdrawal_reason").cloned().unwrap_or(Value::Null),
        "superseded_by": o.get("superseded_by").cloned().unwrap_or(Value::Null),
        "lifecycle": if o
            .get("withdrawn_generation")
            .and_then(|v| v.as_i64())
            .is_some()
        {
            "withdrawn"
        } else {
            "active"
        },
    }))
}

fn project_withdrawal_wire(w: &Value) -> Result<Value, String> {
    let wire = v3::business_json::WithdrawalWire::new(w.into())?;
    serde_json::to_value(wire.view()).map_err(|cause| cause.to_string())
}

fn project_witness_wire(w: &Value) -> Result<Value, String> {
    let raw_bars = w["raw_bars"].clone();
    Ok(json!({
        "witness_id": w["witness_id"].clone(),
        "object_id": w["object_id"].clone(),
        "slot": num_to_str(&w["slot"])?,
        "merged_source_index": num_to_str(&w["merged_source_index"])?,
        "merged_high": w["merged_high"].clone(),
        "merged_low": w["merged_low"].clone(),
        "merged_open": w["merged_open"].clone(),
        "merged_close": w["merged_close"].clone(),
        "raw_bars": project_raw_bars(&raw_bars)?,
    }))
}

fn project_observation_wire(ob: &Value) -> Result<Value, String> {
    let project = |v: &Value| -> Result<Value, String> {
        if v.is_null() {
            Ok(Value::Null)
        } else {
            num_to_str(v)
        }
    };
    Ok(json!({
        "observation_id": ob["observation_id"].clone(),
        "batch_id": ob["batch_id"].clone(),
        "kind": ob["kind"].clone(),
        "window_start": project(&ob["window_start"])?,
        "window_mid": project(&ob["window_mid"])?,
        "window_end": project(&ob["window_end"])?,
        "reason": ob["reason"].clone(),
        "detail": ob["detail"].clone(),
    }))
}

/// i64 形态源事件 → wire 形态（revision/seq/supersedes → 规范十进制字符串）。
fn project_raw_event_wire(ev: &Value) -> Result<Value, String> {
    let mut wire = json!({
        "identity_key": ev["identity_key"].clone(),
        "revision": num_to_str(&ev["revision"])?,
        "input_revision": ev["input_revision"].clone(),
        "payload_hash": ev["payload_hash"].clone(),
        "receipt_id": ev["receipt_id"].clone(),
        "seq": num_to_str(&ev["seq"])?,
        "source_namespace": ev["source_namespace"].clone(),
        "source_epoch": ev["source_epoch"].clone(),
        "instrument": ev["instrument"].clone(),
        "event_id": ev["event_id"].clone(),
        "received_at": ev["received_at"].clone(),
        "raw_text": ev["raw_text"].clone(),
        "price": ev["price"].clone(),
        "ts": ev["ts"].clone(),
        "volume": ev["volume"].clone(),
        "source_coord": ev["source_coord"].clone(),
        "supersedes_revision": ev
            .get("supersedes_revision")
            .and_then(|v| v.as_i64())
            .map(|v| Value::String(v.to_string()))
            .unwrap_or(Value::Null),
    });
    tb02a::raw_wire(&mut wire, ev);
    Ok(wire)
}

enum JsonShape {
    Array,
    Object,
    CatalogBranches,
}

fn json_shape(text: &str, shape: JsonShape) -> Result<Value, String> {
    match json_shape_with_owner(text, shape, None)? {
        v3::business_json::JsonOwner::Plain(v) => v.into_unscoped(),
        #[cfg(feature = "s_allocation_json_testonly")]
        v3::business_json::JsonOwner::Owned(_) => {
            Err("prototype_audit_allocation_incomplete：normal JSON shape不可擦除Owned".into())
        }
    }
}
fn json_shape_with_owner_admitted(
    text: &str,
    shape: JsonShape,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<v3::business_json::JsonOwner, String> {
    #[cfg(feature = "s_capture_native_testonly")]
    if let Some(work) = work {
        if owner.is_some() {
            return Err(work
                .reject(
                    newchan_rust::s_content::capture_allocation::scratch::ScratchFailure::Ledger,
                )
                .to_string());
        }
        work.reserve_native(v3::audit_owner::native::JsonShape::scan(text.as_bytes()).parse_peak())
            .map_err(|e| e.to_string())?
            .retain();
    }
    #[cfg(not(feature = "s_capture_native_testonly"))]
    if let Some(work) = work {
        return Err(work
            .reject(newchan_rust::s_content::capture_allocation::scratch::ScratchFailure::Ledger)
            .to_string());
    }
    json_shape_with_owner(text, shape, owner)
}

fn json_shape_with_owner(
    text: &str,
    shape: JsonShape,
    owner: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
) -> Result<v3::business_json::JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    let value = if let Some(b) = owner {
        newchan_rust::s_content::capture_allocation::audit_json::OwnedJson::parse_legacy_shape(
            text.as_bytes(),
            b,
        )
        .map(v3::business_json::JsonOwner::Owned)
        .map_err(|e| format!("持久 JSON 解析失败：{e}"))?
    } else {
        serde_json::from_str::<Value>(text)
            .map(|v| v3::business_json::JsonOwner::Plain(v.into()))
            .map_err(|e| format!("持久 JSON 解析失败：{e}"))?
    };
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    let value = {
        if owner.is_some() {
            return Err("prototype_audit_allocation_incomplete：owned legacy JSON未启用".into());
        }
        serde_json::from_str::<Value>(text)
            .map(|v| v3::business_json::JsonOwner::Plain(v.into()))
            .map_err(|e| format!("持久 JSON 解析失败：{e}"))?
    };
    let view = value.view();
    let valid = match shape {
        JsonShape::Array => view.is_array(),
        JsonShape::Object => view.is_object(),
        JsonShape::CatalogBranches => view.is_array() || view.is_object(),
    };
    if valid {
        Ok(value)
    } else {
        Err("持久 JSON 形状不符".into())
    }
}

fn json_column_error(index: usize, reason: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, reason)),
    )
}

fn json_column(row: &rusqlite::Row<'_>, index: usize, shape: JsonShape) -> rusqlite::Result<Value> {
    json_shape(&row.get::<_, String>(index)?, shape).map_err(|e| json_column_error(index, e))
}

// #1371 R9：不同查询封装使用同一 publication tuple；gen0 合法且无已发布 batch。
fn publication_fields(conn: &Connection, gen: i64) -> Result<Value, String> {
    if gen == 0 {
        return Ok(
            json!({"input_frontier":"-1", "seq_range":{"from":"0","to":"-1"},
            "catalog_run_status":"not_run", "catalog_evidence":{}}),
        );
    }
    conn.query_row("SELECT input_frontier,seq_range_json,catalog_run_status,catalog_evidence_json FROM structure_deltas WHERE generation=?1", params![gen], |r| Ok(json!({
        "input_frontier":r.get::<_,i64>(0)?.to_string(), "seq_range":json_column(r,1,JsonShape::Object)?,
        "catalog_run_status":r.get::<_,String>(2)?, "catalog_evidence":json_column(r,3,JsonShape::Object)?,
    }))).map_err(|e| format!("StorageUnavailable：读 publication tuple 失败：{e}"))
}

fn read_scope(conn: &Connection) -> Result<Value, String> {
    match meta_get_opt(conn, "scope")? {
        Some(text) => {
            let scope = json_shape(&text, JsonShape::Object)?;
            validate_scope(&scope)?;
            Ok(scope)
        }
        None => Err("StorageUnavailable：meta.scope 缺失".to_string()),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// S.ReadCatalog / S.Snapshot / S.Watch（只读查询，单个读事务内读同一已提交 cut）
// ────────────────────────────────────────────────────────────────────────────

fn read_catalog_in_tx(conn: &Connection, as_of: Option<i64>) -> Result<Value, String> {
    let meta = db_meta(conn)?;
    // 历史 cut 头：AsKnown 取该代持久头；CC-006 的 run_status/evidence 也来自该代（不引用未来）。
    let (header_gen, header_cut, header_cat, cc006_run, cc006_evidence) = match as_of {
        None => (
            meta.get("generation").cloned().unwrap_or_default(),
            meta.get("structure_cut").cloned().unwrap_or_default(),
            meta.get("catalog_revision").cloned().unwrap_or_default(),
            None,
            None,
        ),
        Some(n) => {
            if n < 0 {
                return Err("InvalidDomain：catalog as_of 必须 >= 0".to_string());
            }
            let (idx, cat, run, ev) = if n == 0 {
                (
                    "".to_string(),
                    meta.get("catalog_revision").cloned().unwrap_or_default(),
                    "not_run".to_string(),
                    json!({}).to_string(),
                )
            } else {
                let row: (String, String, String, String) = conn
                    .query_row(
                        "SELECT index_frontier, catalog_revision, catalog_run_status, catalog_evidence_json FROM structure_deltas WHERE generation=?1",
                        params![n],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                    )
                    .optional()
                    .map_err(|e| format!("读 generation {n} 目录头失败：{e}"))?
                    .ok_or_else(|| {
                        format!("Unavailable：请求的 cut generation={n} 尚未发布（无对应持久 Delta）")
                    })?;
                (row.0, row.1, row.2, row.3)
            };
            let _ = idx;
            (n.to_string(), format!("cut-{n}"), cat, Some(run), Some(ev))
        }
    };

    let sealed_axes = if tb02a::enabled(conn)? {
        let generation = header_gen
            .parse::<i64>()
            .map_err(|_| "StorageUnavailable：目录代号非法")?;
        if generation == 0 {
            None
        } else {
            let evidence: String = conn
                .query_row(
                    "SELECT catalog_evidence_json FROM structure_deltas WHERE generation=?1",
                    params![generation],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let evidence = json_shape(&evidence, JsonShape::Object)?;
            Some(
                evidence["axes"]
                    .as_object()
                    .ok_or("StorageUnavailable：逐轴目录缺失")?
                    .clone(),
            )
        }
    } else {
        None
    };
    let mut stmt = conn
        .prepare("SELECT catalog_id, kind, title, domain, branches_json, impl_status, proof_status, run_status, evidence_json FROM catalog ORDER BY catalog_id")
        .map_err(|e| format!("prepare catalog 失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| {
            let cid = r.get::<_, String>(0)?;
            let mut run_status = r.get::<_, String>(7)?;
            let mut evidence = json_column(r, 8, JsonShape::Object)?;
            if cid == "CC-006" {
                if let Some(rs) = &cc006_run {
                    run_status = rs.clone();
                }
                if let Some(ev) = &cc006_evidence {
                    evidence =
                        json_shape(ev, JsonShape::Object).map_err(|e| json_column_error(8, e))?;
                }
            }
            let (mut implementation_status, mut proof_status) = if let Some(n) = as_of {
                if n == 0 {
                    run_status = "not_run".to_string();
                    evidence = json!({});
                }
                (
                    if cid == "CC-006" && n > 0 {
                        "implemented"
                    } else {
                        "not_implemented"
                    }
                    .to_string(),
                    "not_proved".to_string(),
                )
            } else {
                (r.get::<_, String>(5)?, r.get::<_, String>(6)?)
            };
            if let Some(axis) = sealed_axes.as_ref().and_then(|axes| axes.get(&cid)) {
                implementation_status = axis["impl_status"]
                    .as_str()
                    .ok_or_else(|| json_column_error(8, "目录实现状态缺失".into()))?
                    .to_string();
                proof_status = axis["proof_status"]
                    .as_str()
                    .ok_or_else(|| json_column_error(8, "目录证明状态缺失".into()))?
                    .to_string();
                run_status = axis["run_status"]
                    .as_str()
                    .ok_or_else(|| json_column_error(8, "目录运行状态缺失".into()))?
                    .to_string();
                evidence = axis["evidence"].clone();
            }
            Ok(json!({
                "id": cid,
                "kind": r.get::<_, String>(1)?,
                "title": r.get::<_, String>(2)?,
                "domain": r.get::<_, String>(3)?,
                "branches": json_column(r, 4, JsonShape::CatalogBranches)?,
                "implementation_status": implementation_status,
                "proof_status": proof_status,
                "run_status": run_status,
                "evidence": evidence,
            }))
        })
        .map_err(|e| format!("query catalog 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect catalog 失败：{e}"))?;
    let mut result = json!({
        "session_id": meta.get("session_id").cloned().unwrap_or_default(),
        "generation": header_gen,
        "structure_cut": header_cut,
        "catalog_revision": header_cat,
        "scope": read_scope(conn)?,
        "items": rows,
        "counts": {
            "implemented": rows.iter().filter(|r| r["implementation_status"] == "implemented").count(),
            "not_implemented": rows.iter().filter(|r| r["implementation_status"] == "not_implemented").count(),
            "run": rows.iter().filter(|r| r["run_status"] == "run").count(),
            "not_run": rows.iter().filter(|r| r["run_status"] == "not_run").count(),
        }
    });
    let gen = header_gen.parse::<i64>().map_err(|e| e.to_string())?;
    let fields = publication_fields(conn, gen)?;
    result
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    result["index_frontier"] = if gen == 0 {
        json!("")
    } else {
        json!(conn
            .query_row(
                "SELECT index_frontier FROM structure_deltas WHERE generation=?1",
                params![gen],
                |r| r.get::<_, String>(0)
            )
            .map_err(|e| e.to_string())?)
    };
    Ok(result)
}

/// 读对象（active 或 as_of 视图）与撤回对象，按生命周期的 cut 过滤。
fn read_objects_view(
    conn: &Connection,
    as_of: Option<i64>,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    // 合法生命周期域：撤回代必须晚于首获知代；wg<=fkg 为持久损坏，不得被 as_of 过滤隐藏后成功。
    let invalid_lifecycle: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM objects WHERE withdrawn_generation IS NOT NULL AND withdrawn_generation <= first_known_generation",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("核对象生命周期失败：{e}"))?;
    if invalid_lifecycle != 0 {
        return Err("StorageUnavailable：对象生命周期非法（withdrawn_generation <= first_known_generation）".to_string());
    }
    let active_sql = match as_of {
        None => {
            "SELECT object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, window_start, window_mid, window_end, comparisons_json, input_refs_json, source_coords_json, first_known_generation, first_known_cut, published_generation, withdrawn_generation, withdrawal_reason, superseded_by FROM objects WHERE withdrawn_generation IS NULL ORDER BY window_start"
                .to_string()
        }
        Some(_) => {
            "SELECT object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, window_start, window_mid, window_end, comparisons_json, input_refs_json, source_coords_json, first_known_generation, first_known_cut, published_generation, withdrawn_generation, withdrawal_reason, superseded_by FROM objects WHERE first_known_generation <= ?1 AND (withdrawn_generation IS NULL OR withdrawn_generation > ?1) ORDER BY window_start"
                .to_string()
        }
    };
    let withdrawn_sql = match as_of {
        None => {
            "SELECT object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, window_start, window_mid, window_end, comparisons_json, input_refs_json, source_coords_json, first_known_generation, first_known_cut, published_generation, withdrawn_generation, withdrawal_reason, superseded_by FROM objects WHERE withdrawn_generation IS NOT NULL ORDER BY first_known_generation"
                .to_string()
        }
        Some(_) => {
            "SELECT object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, window_start, window_mid, window_end, comparisons_json, input_refs_json, source_coords_json, first_known_generation, first_known_cut, published_generation, withdrawn_generation, withdrawal_reason, superseded_by FROM objects WHERE withdrawn_generation IS NOT NULL AND withdrawn_generation <= ?1 AND first_known_generation <= ?1 ORDER BY first_known_generation"
                .to_string()
        }
    };

    let mut stmt = conn
        .prepare(&active_sql)
        .map_err(|e| format!("prepare objects 失败：{e}"))?;
    let mut objects = match as_of {
        None => stmt
            .query_map([], |r| project_object_row(r, None))
            .map_err(|e| format!("query objects 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect objects 失败：{e}"))?,
        Some(n) => stmt
            .query_map(params![n], |r| project_object_row(r, Some(n)))
            .map_err(|e| format!("query objects 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect objects 失败：{e}"))?,
    };

    let mut wstmt = conn
        .prepare(&withdrawn_sql)
        .map_err(|e| format!("prepare withdrawn 失败：{e}"))?;
    let mut withdrawn = match as_of {
        None => wstmt
            .query_map([], |r| project_object_row(r, None))
            .map_err(|e| format!("query withdrawn 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect withdrawn 失败：{e}"))?,
        Some(n) => wstmt
            .query_map(params![n], |r| project_object_row(r, Some(n)))
            .map_err(|e| format!("query withdrawn 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect withdrawn 失败：{e}"))?,
    };

    let (typed_active, typed_dead) = tb02a::read_facts(conn, as_of)?;
    objects.extend(
        typed_active
            .iter()
            .map(tb02a::project)
            .collect::<Result<Vec<_>, _>>()?,
    );
    withdrawn.extend(
        typed_dead
            .iter()
            .map(tb02a::project)
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok((objects, withdrawn))
}

/// 对象行 → wire 形态；as_of=Some(N) 时，若对象在 N 之后才撤回（withdrawn_generation > N），
/// 按「当时仍活动」投影（不把后来的撤回元数据倒填到旧 cut）。
fn project_object_row(row: &rusqlite::Row<'_>, as_of: Option<i64>) -> rusqlite::Result<Value> {
    let wg: Option<i64> = row.get(16)?;
    // 合法生命周期：撤回代必须晚于首获知代（wg > fkg）；wg<=fkg 为持久损坏，不得投影后成功。
    let fkg_raw: i64 = row.get(13)?;
    if let Some(w) = wg {
        if w <= fkg_raw {
            return Err(json_column_error(
                16,
                format!("对象生命周期非法：withdrawn_generation={w} <= first_known_generation={fkg_raw}"),
            ));
        }
    }
    let effective_wg = match (as_of, wg) {
        (Some(n), Some(w)) if w > n => None,
        _ => wg,
    };
    Ok(json!({
        "object_id": row.get::<_, String>(0)?,
        "object_revision": row.get::<_, i64>(1)?.to_string(),
        "kind": row.get::<_, String>(2)?,
        "batch_id": row.get::<_, String>(3)?,
        "branch": row.get::<_, String>(4)?,
        "dir_ab": row.get::<_, String>(5)?,
        "dir_bc": row.get::<_, String>(6)?,
        "window_start": row.get::<_, i64>(7)?.to_string(),
        "window_mid": row.get::<_, i64>(8)?.to_string(),
        "window_end": row.get::<_, i64>(9)?.to_string(),
        "comparisons": json_column(row, 10, JsonShape::Array)?,
        "input_refs": project_input_refs(&json_column(row, 11, JsonShape::Array)?)
            .map_err(|e| json_column_error(11, e))?,
        "source_coords": json_column(row, 12, JsonShape::Array)?,
        "first_known_generation": row.get::<_, i64>(13)?.to_string(),
        "first_known_cut": row.get::<_, String>(14)?,
        "published_generation": row.get::<_, i64>(15)?.to_string(),
        "withdrawn_generation": effective_wg.map(|v| v.to_string()),
        "withdrawal_reason": if effective_wg.is_some() {
            row.get::<_, Option<String>>(17)?
        } else {
            None
        },
        "superseded_by": if effective_wg.is_some() {
            row.get::<_, Option<String>>(18)?
        } else {
            None
        },
        "lifecycle": if effective_wg.is_some() { "withdrawn" } else { "active" },
    }))
}

/// 指定 generation 的输入前沿（该代提交后的接纳序上限）。
/// gen == 0 ⟹ Some(-1)（初始空 cut）；gen > 0 且 delta 行存在 ⟹ Some(frontier)；
/// delta 行不存在（该 cut 尚未发布）⟹ None（显式不可用，不回退 -1 泄漏空/未来）。
fn frontier_at_generation(conn: &Connection, gen: i64) -> Result<Option<i64>, String> {
    if gen == 0 {
        return Ok(Some(-1));
    }
    if gen < 0 {
        return Ok(None);
    }
    conn.query_row(
        "SELECT input_frontier FROM structure_deltas WHERE generation=?1",
        params![gen],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| format!("读 generation {gen} 输入前沿失败：{e}"))
}

/// 源修订链（含 supersedes），可选按接纳序上限过滤（AsKnown 当时可知）。
fn read_raw_history(conn: &Connection, max_seq: Option<i64>) -> Result<Vec<Value>, String> {
    let sql = match max_seq {
        None => {
            "SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace, source_epoch, instrument, event_id, received_at, raw_text, price, ts, volume, source_coord, supersedes_revision FROM raw_events ORDER BY seq ASC"
                .to_string()
        }
        Some(_) => {
            "SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace, source_epoch, instrument, event_id, received_at, raw_text, price, ts, volume, source_coord, supersedes_revision FROM raw_events WHERE seq <= ?1 ORDER BY seq ASC"
                .to_string()
        }
    };
    let project = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Value> {
        Ok(json!({
            "identity_key": r.get::<_, String>(0)?,
            "revision": r.get::<_, i64>(1)?.to_string(),
            "input_revision": r.get::<_, String>(2)?,
            "payload_hash": r.get::<_, String>(3)?,
            "receipt_id": r.get::<_, String>(4)?,
            "seq": r.get::<_, i64>(5)?.to_string(),
            "source_namespace": r.get::<_, String>(6)?,
            "source_epoch": r.get::<_, String>(7)?,
            "instrument": r.get::<_, String>(8)?,
            "event_id": r.get::<_, String>(9)?,
            "received_at": r.get::<_, String>(10)?,
            "raw_text": r.get::<_, String>(11)?,
            "price": r.get::<_, String>(12)?,
            "ts": r.get::<_, String>(13)?,
            "volume": r.get::<_, String>(14)?,
            "source_coord": r.get::<_, String>(15)?,
            "supersedes_revision": r.get::<_, Option<i64>>(16)?.map(|v| v.to_string()),
        }))
    };
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("prepare raw_history 失败：{e}"))?;
    let mut rows = match max_seq {
        None => stmt
            .query_map([], project)
            .map_err(|e| format!("query raw_history 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect raw_history 失败：{e}"))?,
        Some(m) => stmt
            .query_map(params![m], project)
            .map_err(|e| format!("query raw_history 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect raw_history 失败：{e}"))?,
    };
    tb02a::enrich_raw(conn, &mut rows)?;
    for row in &mut rows {
        let raw = row.clone();
        tb02a::raw_wire(row, &raw);
    }
    Ok(rows)
}

fn read_snapshot_in_tx(conn: &Connection, as_of: Option<i64>) -> Result<Value, String> {
    // #1394 第二段（2D）：一次快照读共享一个捕获作用域（对象、事实、批次 profile 共用读集）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    let meta = db_meta(conn)?;
    let (objects, withdrawn) = read_objects_view(conn, as_of)?;
    // 历史/当前切面头：AsKnown 取该代持久头（不可变）；current 用当前已提交头。
    // raw_history：AsKnown 按该代输入前沿；current 只到「已发布前沿」（未 Advance 的接纳日志不得混入）。
    let (header_gen, header_cut, header_index, header_cat, max_seq) = match as_of {
        Some(n) => {
            let fr = frontier_at_generation(conn, n)?;
            let fr = fr.ok_or_else(|| {
                format!("Unavailable：请求的 cut generation={n} 尚未发布（无对应持久 Delta），不返回混合/空历史")
            })?;
            let (idx, cat) = if n == 0 {
                (
                    "".to_string(),
                    meta.get("catalog_revision").cloned().unwrap_or_default(),
                )
            } else {
                let row: (String, String) = conn
                    .query_row(
                        "SELECT index_frontier, catalog_revision FROM structure_deltas WHERE generation=?1",
                        params![n],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .map_err(|e| format!("读 generation {n} 历史头失败：{e}"))?;
                (row.0, row.1)
            };
            (n.to_string(), format!("cut-{n}"), idx, cat, Some(fr))
        }
        None => {
            let published_frontier: i64 = meta
                .get("last_advance_frontier")
                .and_then(|s| s.parse().ok())
                .unwrap_or(-1);
            (
                meta.get("generation").cloned().unwrap_or_default(),
                meta.get("structure_cut").cloned().unwrap_or_default(),
                meta.get("index_frontier").cloned().unwrap_or_default(),
                meta.get("catalog_revision").cloned().unwrap_or_default(),
                Some(published_frontier),
            )
        }
    };

    let (cut_profile_id, cut_profile_hash) =
        profile_at_cut(conn, header_gen.parse::<i64>().map_err(|e| e.to_string())?)?;
    let wit_sql = match as_of {
        None => {
            "SELECT witness_id, object_id, slot, merged_source_index, merged_high, merged_low, merged_open, merged_close, raw_json FROM witnesses ORDER BY object_id, slot"
                .to_string()
        }
        Some(_) => {
            "SELECT witness_id, object_id, slot, merged_source_index, merged_high, merged_low, merged_open, merged_close, raw_json FROM witnesses WHERE published_generation <= ?1 ORDER BY object_id, slot"
                .to_string()
        }
    };
    let mut wstmt = conn
        .prepare(&wit_sql)
        .map_err(|e| format!("prepare witnesses 失败：{e}"))?;
    let witness_project = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Value> {
        Ok(json!({
            "witness_id": r.get::<_, String>(0)?,
            "object_id": r.get::<_, String>(1)?,
            "slot": r.get::<_, i64>(2)?.to_string(),
            "merged_source_index": r.get::<_, i64>(3)?.to_string(),
            "merged_high": r.get::<_, String>(4)?,
            "merged_low": r.get::<_, String>(5)?,
            "merged_open": r.get::<_, String>(6)?,
            "merged_close": r.get::<_, String>(7)?,
            "raw_bars": project_raw_bars(&json_column(r, 8, JsonShape::Array)?)
                .map_err(|e| json_column_error(8, e))?,
        }))
    };
    let witnesses = match as_of {
        None => wstmt
            .query_map([], witness_project)
            .map_err(|e| format!("query witnesses 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect witnesses 失败：{e}"))?,
        Some(n) => wstmt
            .query_map(params![n], witness_project)
            .map_err(|e| format!("query witnesses 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect witnesses 失败：{e}"))?,
    };

    let rel_sql = match as_of {
        None => {
            "SELECT subject, relation_type, object FROM relations ORDER BY subject, relation_type, object"
                .to_string()
        }
        Some(_) => {
            "SELECT subject, relation_type, object FROM relations WHERE published_generation <= ?1 ORDER BY subject, relation_type, object"
                .to_string()
        }
    };
    let mut rstmt = conn
        .prepare(&rel_sql)
        .map_err(|e| format!("prepare relations 失败：{e}"))?;
    let rel_project = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Value> {
        Ok(json!({
            "subject": r.get::<_, String>(0)?,
            "relation_type": r.get::<_, String>(1)?,
            "object": r.get::<_, String>(2)?,
        }))
    };
    let relations = match as_of {
        None => rstmt
            .query_map([], rel_project)
            .map_err(|e| format!("query relations 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect relations 失败：{e}"))?,
        Some(n) => rstmt
            .query_map(params![n], rel_project)
            .map_err(|e| format!("query relations 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect relations 失败：{e}"))?,
    };

    let obs_sql = match as_of {
        None => {
            "SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, reason, detail_json FROM observations ORDER BY kind, window_start"
                .to_string()
        }
        Some(_) => {
            "SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, reason, detail_json FROM observations WHERE published_generation <= ?1 ORDER BY kind, window_start"
                .to_string()
        }
    };
    let mut obs_stmt = conn
        .prepare(&obs_sql)
        .map_err(|e| format!("prepare observations 失败：{e}"))?;
    let obs_project = |r: &rusqlite::Row<'_>| -> rusqlite::Result<Value> {
        Ok(json!({
            "observation_id": r.get::<_, String>(0)?,
            "batch_id": r.get::<_, String>(1)?,
            "kind": r.get::<_, String>(2)?,
            "window_start": r.get::<_, Option<i64>>(3)?.map(|v| v.to_string()),
            "window_mid": r.get::<_, Option<i64>>(4)?.map(|v| v.to_string()),
            "window_end": r.get::<_, Option<i64>>(5)?.map(|v| v.to_string()),
            "reason": r.get::<_, String>(6)?,
            "detail": json_column(r, 7, JsonShape::Object)?,
        }))
    };
    let observations = match as_of {
        None => obs_stmt
            .query_map([], obs_project)
            .map_err(|e| format!("query observations 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect observations 失败：{e}"))?,
        Some(n) => obs_stmt
            .query_map(params![n], obs_project)
            .map_err(|e| format!("query observations 失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect observations 失败：{e}"))?,
    };

    let mut result = json!({
        "session_id": meta.get("session_id").cloned().unwrap_or_default(),
        "generation": header_gen,
        "structure_cut": header_cut,
        "catalog_revision": header_cat,
        "scope": read_scope(conn)?,
        "index_frontier": header_index,
        "profile_id": cut_profile_id,
        "profile_hash": cut_profile_hash,
        "history_mode": if as_of.is_some() { "AsKnown" } else { "RecomputedWithRevision" },
        "as_of_generation": as_of.map(|v| v.to_string()),
        "objects": objects,
        "withdrawn_objects": withdrawn,
        "witnesses": witnesses,
        "relations": relations,
        "observations": observations,
        "raw_history": read_raw_history(conn, max_seq)?,
    });
    let gen = header_gen.parse::<i64>().map_err(|e| e.to_string())?;
    let fields = publication_fields(conn, gen)?;
    result
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    Ok(result)
}

fn cmd_catalog(db: &Path, as_of: Option<i64>) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开读事务失败：{e}"))?;
    verify_reachable_root(&tx)?;
    let out = read_catalog_in_tx(&tx, as_of)?;
    drop(tx);
    println!("{}", project_wire_integers(&out));
    Ok(())
}

fn cmd_snapshot(db: &Path, as_of: Option<i64>) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开读事务失败：{e}"))?;
    verify_reachable_root(&tx)?;
    let out = read_snapshot_in_tx(&tx, as_of)?;
    drop(tx);
    println!("{}", project_wire_integers(&out));
    Ok(())
}

/// DeliveryUnknown 权威查询：按原输入业务身份（identity_key = namespace|epoch|instrument|event_id）
/// 只读返回该身份的全部接纳修订与收据，并注明是否已进入已发布前沿。不产生写入、不新建动作。
fn cmd_query(db: &Path, identity_key: &str) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开读事务失败：{e}"))?;
    verify_reachable_root(&tx)?;
    let published_frontier: i64 = meta_i64(&tx, "last_advance_frontier")?;
    let mut rows: Vec<Value> = {
        let mut stmt = tx
            .prepare(
                "SELECT revision, input_revision, payload_hash, receipt_id, seq, source_coord, price, supersedes_revision FROM raw_events WHERE identity_key=?1 ORDER BY revision ASC",
            )
            .map_err(|e| format!("prepare query 失败：{e}"))?;
        let mut q = stmt
            .query(params![identity_key])
            .map_err(|e| format!("query identity 失败：{e}"))?;
        let mut out: Vec<Value> = Vec::new();
        while let Some(row) = q.next().map_err(|e| format!("query identity 失败：{e}"))? {
            let rec: Value = (|| -> rusqlite::Result<Value> {
                Ok(json!({
                    "revision": row.get::<_, i64>(0)?.to_string(),
                    "input_revision": row.get::<_, String>(1)?,
                    "payload_hash": row.get::<_, String>(2)?,
                    "receipt_id": row.get::<_, String>(3)?,
                    "seq": row.get::<_, i64>(4)?.to_string(),
                    "source_coord": row.get::<_, String>(5)?,
                    "price": row.get::<_, String>(6)?,
                    "supersedes_revision": row.get::<_, Option<i64>>(7)?.map(|v| v.to_string()),
                }))
            })()
            .map_err(|e| format!("query identity 失败：{e}"))?;
            out.push(rec);
        }
        out
    };
    if tb02a::enabled(&tx)? {
        for row in &mut rows {
            row["identity_key"] = json!(identity_key);
        }
        tb02a::enrich_raw(&tx, &mut rows)?;
        for row in &mut rows {
            let raw = row.clone();
            tb02a::raw_wire(row, &raw);
            row.as_object_mut().unwrap().remove("identity_key");
        }
    }
    let latest = rows
        .iter()
        .max_by_key(|r| {
            r["revision"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
        })
        .cloned();
    let published = latest
        .as_ref()
        .map(|r| {
            r["seq"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(-1)
                <= published_frontier
        })
        .unwrap_or(false);
    drop(tx);
    let out = json!({
        "ok": true,
        "identity_key": identity_key,
        "published_frontier": published_frontier.to_string(),
        "records": rows,
        "latest": latest,
        "latest_published": published,
        "note": if rows.is_empty() {
            "NoRecord：该业务身份无接纳记录（不据此自动新建动作）"
        } else if published {
            "已接纳且已进入已发布前沿（权威结果）"
        } else {
            "已接纳但尚未进入已发布前沿（未 Advance）"
        },
    });
    println!("{}", project_wire_integers(&out));
    Ok(())
}

/// #1394：普通财务准备读取 S 原结构证据，S 不参加经济事务或经营轮。
fn financial_basis_pending(conn: &Connection) -> Result<bool, String> {
    // begin_token 在 Commit 后保留为历史证据；只有当前 Begin 门或未发布输入才未完成。
    let state = meta_get_opt(conn, "advance_state")?.ok_or("MissingDependency：S 缺推进状态")?;
    let published = meta_i64(conn, "last_advance_frontier")?;
    let accepted: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
            r.get(0)
        })
        .map_err(|e| format!("读 S 已接纳前沿失败：{e}"))?;
    Ok(state != "idle" || accepted != published)
}

fn cmd_financial_basis(db: &Path) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    verify_reachable_root(&tx)?;
    let meta = db_meta(&tx)?;
    let (sid, generation, _) = v2::protocol_identity(&tx)?;
    let pending = financial_basis_pending(&tx)?;
    let snapshot = read_snapshot_in_tx(&tx, None)?;
    let commit_ns: Option<String> = tx.query_row(
        "SELECT semantic_ns FROM s_clock_events WHERE phase='commit' AND generation=? ORDER BY attempt_index DESC LIMIT 1",
        [snapshot["generation"].as_str().ok_or("MissingDependency：S 发布代缺失")?], |r|r.get(0)
    ).optional().map_err(|e|e.to_string())?;
    println!(
        "{}",
        project_wire_integers(&json!({"kind":"FinancialBasis", "session_id":sid,
        "session_generation":generation, "writer_epoch":meta.get("writer_epoch"),
        "pending_revision":pending, "commit_ns":commit_ns, "snapshot":snapshot}))
    );
    Ok(())
}

fn cmd_meta(db: &Path) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let meta = db_meta(&conn)?;
    println!("{}", json!(meta));
    Ok(())
}

/// TestOnly 诊断：用与 accept/advance/recover 相同的 open_db 连接，读回实际 PRAGMA
/// （journal_mode / synchronous / fullfsync）。供 macOS 根验从真实 writer 连接取证 fullfsync；
/// 不是持久性自证，仅记录该连接的实际平台前件读数。
fn cmd_pragma(db: &Path) -> Result<(), String> {
    let conn = open_db(db)?;
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(|e| format!("读 journal_mode 失败：{e}"))?;
    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(|e| format!("读 synchronous 失败：{e}"))?;
    let fullfsync: Option<i64> = {
        #[cfg(target_os = "macos")]
        {
            Some(
                conn.query_row("PRAGMA fullfsync", [], |r| r.get(0))
                    .map_err(|e| format!("读 fullfsync 失败：{e}"))?,
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    };
    println!(
        "{}",
        json!({
            "ok": true,
            "platform": std::env::consts::OS,
            "journal_mode": journal_mode,
            "synchronous": synchronous,
            "fullfsync": fullfsync,
        })
    );
    Ok(())
}

/// 读一批 StructureDelta 行并投影为 wire 形式（整数坐标 → 规范十进制字符串）。
///
/// 行内 `delta_json` 只是**物理列**：图域里它必须是空占位，完整原件由 `delta/<generation>`
/// 私有根无损展开（同一捕获共用节点读集与已核句柄）；整块域取行内原文。返回顺序与 SQL 一致。
fn delta_wire_rows(
    conn: &Connection,
    sql: &str,
    after_generation: i64,
) -> Result<Vec<Value>, String> {
    let raw: Vec<(
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        String,
    )> = {
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| format!("prepare 读 Delta 行失败：{e}"))?;
        let rows = stmt
            .query_map(params![after_generation], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                ))
            })
            .map_err(|e| format!("query 读 Delta 行失败：{e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect 读 Delta 行失败：{e}"))?;
        rows
    };
    let format = evidence_store::format(conn)?;
    let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
    let mut out = Vec::with_capacity(raw.len());
    for (
        generation,
        session_id,
        catalog_revision,
        base_cut,
        next_cut,
        seq_range,
        index_frontier,
        input_frontier,
        column,
    ) in raw
    {
        let text = read_record_original_text(
            conn,
            &mut reader,
            format,
            &evidence_store::delta_root_key(generation),
            &column,
            &format!("delta[{generation}]"),
        )?;
        out.push(json!({
            "generation": generation.to_string(),
            "session_id": session_id,
            "catalog_revision": catalog_revision,
            "base_cut": base_cut,
            "next_cut": next_cut,
            "seq_range": json_shape(&seq_range, JsonShape::Object)
                .map_err(|e| format!("读取 delta[{generation}] seq_range 失败：{e}"))?,
            "index_frontier": index_frontier,
            "input_frontier": input_frontier.to_string(),
            "delta": json_shape(&text, JsonShape::Object)
                .map_err(|e| format!("读取 delta[{generation}] 失败：{e}"))?,
        }));
    }
    Ok(out)
}

fn now_nanos() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_default()
}

/// S.Watch：同源读取 S 自己持久化的 StructureDelta（base_cut/next_cut/seq_range/upsert/撤回/替代/
/// 关系/见证/index_frontier）。重复（generation ≤ cursor）不回放；缺口显式 Gap 并给重建 cut。
fn cmd_watch(db: &Path, after_generation: i64) -> Result<(), String> {
    // #1394 第二段（2D）：一次 watch 共享一个捕获作用域（根审计 + Delta 原件展开）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开读事务失败：{e}"))?;
    verify_reachable_root(&tx)?;
    if let Some(value) = v2::retained_watch(&tx, after_generation)? {
        drop(tx);
        println!("{}", project_wire_integers(&value));
        return Ok(());
    }
    let meta = db_meta(&tx)?;
    let current_gen: i64 = meta_i64(&tx, "generation")?;
    let current_cut = meta.get("structure_cut").cloned().unwrap_or_default();

    let rows: Vec<Value> = delta_wire_rows(
        &tx,
        "SELECT generation, session_id, catalog_revision, base_cut, next_cut, seq_range_json, index_frontier, input_frontier, delta_json FROM structure_deltas WHERE generation > ?1 ORDER BY generation ASC",
        after_generation,
    )
    .map_err(|e| format!("读取 delta 失败：{e}"))?;

    // 缺口检测：cursor 落后时核完整连续序列与末端抵达 current；游标超前（重建会话）显式 Gap。
    let mut gap: Value = Value::Null;
    if after_generation > current_gen {
        gap = json!({
            "reason": "cursor_ahead_or_session_rebuilt",
            "rebuild_cut": current_cut,
        });
    } else if after_generation < current_gen {
        let gens: Vec<i64> = rows
            .iter()
            .map(|d| {
                d["generation"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(-1)
            })
            .collect();
        let expected: Vec<i64> = (after_generation + 1..=current_gen).collect();
        if gens != expected {
            gap = json!({
                "reason": "cursor_stale_or_retained_delta_missing",
                "rebuild_cut": current_cut,
            });
        }
    }

    drop(tx);
    let out = json!({
        "ok": true,
        "session_id": meta.get("session_id").cloned().unwrap_or_default(),
        "generation": current_gen.to_string(),
        "structure_cut": current_cut,
        "after_generation": after_generation.to_string(),
        "gap": gap,
        "deltas": rows,
    });
    println!("{}", project_wire_integers(&out));
    Ok(())
}

/// 正式 writer 换代 / 恢复：核当前持久状态，清未决 Begin（上一 writer 已退出，门保持关闭直到本
/// 命令合法恢复），并按 `--new-epoch` 提升 writer_epoch（与 generation 同序持久历史）。旧 epoch 进程
/// 此后重放被 StaleWriter 拒绝。这不是「手改 SQL」——是正式入口，历史写入 writer_epoch_history。
fn cmd_recover(db: &Path, new_epoch: &str) -> Result<(), String> {
    println!("{}", recover_core(db, new_epoch, None)?);
    Ok(())
}

fn recover_core(
    db: &Path,
    new_epoch: &str,
    clock: Option<(&v2::ClockPlan, &str)>,
) -> Result<Value, String> {
    // #1394 第二段（2D）：一次 recover 共享一个捕获作用域（写入前会显式作废）。
    let _capture = cache::begin_capture(EVIDENCE_READ_BUDGET);
    let new_epoch_i =
        parse_writer_epoch(new_epoch, "--new-epoch").map_err(|e| format!("InvalidDomain：{e}"))?;
    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
    ensure_writable(&conn)?;
    record_connection_pragmas(&conn, "recover");
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开 recover 事务失败：{e}"))?;
    let cur_epoch_raw = meta_get_opt(&tx, "writer_epoch")?.unwrap_or_default();
    let cur_epoch = parse_writer_epoch(&cur_epoch_raw, "meta.writer_epoch")
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    // writer 代际只准单调上升：new_epoch 必须严格大于当前；等于/回退（含重写旧 epoch）拒绝，
    // 避免旧 owner 借同/低 epoch 重新取得写权或清不属于该恢复的未决 Begin。
    if new_epoch_i <= cur_epoch {
        return Err(format!(
            "StaleWriter：recover 换代必须严格上升（当前 writer_epoch=`{cur_epoch_raw}`，请求 `{new_epoch}`）——禁止回退/重写旧 epoch"
        ));
    }
    let state = meta_get_opt(&tx, "advance_state")?.unwrap_or_else(|| "idle".to_string());
    let generation = meta_get_opt(&tx, "generation")?.unwrap_or_default();

    // 恢复引用完整性：与 accept/advance 同一严格可达根核（坏 bytes/缺历史引用/缺根 meta 均拒绝，
    // 不清理门、不换代）。
    verify_reachable_root(&tx)?;

    let mut recovered_begin: Value = Value::Null;
    let mut state_after = state.clone();
    if state != "idle" {
        // 未决 Begin：清门仅在真实 epoch 换代（严格上升）下发生；不删除可达根、不伪造该 Begin 发布。
        recovered_begin = json!({
            "advance_state": state,
            "resolved_to": "idle",
            "generation_at_recovery": generation,
        });
        meta_set(&tx, "advance_state", "idle")?;
        meta_set(&tx, "begin_token", "")?;
        meta_set(
            &tx,
            "last_recovered_begin",
            &format!("{state}:recovered_at_generation_{generation}"),
        )?;
        state_after = "idle".to_string();
    }
    let transitioned_at = v2::record_recover(&tx, clock, &generation)?;
    meta_set(&tx, "writer_epoch", new_epoch)?;
    tx.execute(
        "INSERT INTO writer_epoch_history(from_epoch, to_epoch, generation_at_transition, advance_state_at_transition, transitioned_at) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![cur_epoch_raw, new_epoch, generation, state_after, transitioned_at],
    )
    .map_err(|e| format!("写 writer_epoch_history 失败：{e}"))?;
    tx.commit()
        .map_err(|e| format!("recover commit 失败：{e}"))?;

    Ok(json!({
        "ok": true,
        "previous_epoch": cur_epoch_raw,
        "new_epoch": new_epoch,
        "epoch_transitioned": true,
        "recovered_begin": recovered_begin,
        "reachable_root": {
            "generation": generation,
            "structure_cut": meta_get_opt(&conn, "structure_cut")?.unwrap_or_default(),
            "index_frontier": meta_get_opt(&conn, "index_frontier")?.unwrap_or_default(),
        },
    }))
}

// ────────────────────────────────────────────────────────────────────────────
// CLI 分发
// ────────────────────────────────────────────────────────────────────────────

fn usage() -> String {
    "用法：s_structure_session <init|accept|advance|catalog|snapshot|meta|watch|recover|reset|serve|query-serve> [参数]\n\
     \x20 init      --db <路径> --session <id> --catalog <catalog.json>      （只允许不存在的目标）\n\
     \x20 accept    --db <路径> --input <输入.json> --profile <profile.json> [--writer-epoch <n>]\n\
     \x20 advance   --db <路径> [--writer-epoch <n>]\n\
     \x20 catalog   --db <路径> [--as-of <generation>]\n\
     \x20 snapshot  --db <路径> [--as-of <generation>]\n\
     \x20 meta      --db <路径>\n\
     \x20 pragma    --db <路径>\n\
     \x20 query     --db <路径> --identity-key <k>\n\
     \x20 query-serve --protocol-revision s-session/3 --db <已有库> --resource-config <Q21.json> --producer-epoch <n> [--host <IPv4主机>] [--port <n>]（Unix；启用前须具名实测资源）\n\
     \x20 serve --protocol-revision s-session/3 --db <已有库> --profile <profile.json> --writer-epoch <n> --clock-plan <clock.json> --socket <路径> --resource-config <S资源.json>（资源及C09条件队列资格未满足时拒绝监听）\n\
     \x20 advance --protocol-revision s-session/3 --db <已有库> --profile <profile.json> --writer-epoch <n> --clock-plan <clock.json> [--input <单份envelope.json> --max-input-bytes <显式上限>]（无input仅恢复已持久接纳的原消息）\n\
     \x20 recover --protocol-revision s-session/3 --db <已有库> --new-epoch <n> --clock-plan <clock.json> --clock-event-id <id>（监督器换代）\n\
     \x20 watch     --db <路径> --after-generation <n>\n\
     \x20 recover   --db <路径> --new-epoch <n>\n\
     \x20 reset     --db <路径> --session <id> --catalog <catalog.json>      （一次性试验：删除并重建）\n"
        .to_string()
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn remove_db_files(db: &Path) {
    let _ = std::fs::remove_file(db);
    let _ = std::fs::remove_file(PathBuf::from(format!("{}-wal", db.display())));
    let _ = std::fs::remove_file(PathBuf::from(format!("{}-shm", db.display())));
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(usage());
    }
    if v3::dispatch(&args)? {
        return Ok(());
    }
    if v2::dispatch(&args)? {
        return Ok(());
    }
    let _writer_lock = v2::legacy_write_lock(&args)?;
    if let Some(guard) = &_writer_lock {
        guard.require_admitted()?;
    }
    let _admission_open_guard = _writer_lock
        .as_ref()
        .map(|g| g.enter())
        .transpose()?
        .flatten();
    let cmd = args[1].as_str();
    let db = arg_value(&args, "--db").map(PathBuf::from);
    let session = arg_value(&args, "--session").unwrap_or_else(|| "s-session-default".to_string());
    let catalog = arg_value(&args, "--catalog").map(PathBuf::from);
    let input = arg_value(&args, "--input").map(PathBuf::from);
    let profile = arg_value(&args, "--profile").map(PathBuf::from);
    let configured_epoch =
        arg_value(&args, "--writer-epoch").unwrap_or_else(|| DEFAULT_WRITER_EPOCH.to_string());
    let as_of = arg_value(&args, "--as-of")
        .map(|s| parse_canonical_i64(&s, "--as-of"))
        .transpose()?
        .map(|v| {
            if v < 0 {
                Err("InvalidDomain：--as-of 必须 >= 0".to_string())
            } else {
                Ok(v)
            }
        })
        .transpose()?;
    let new_epoch = arg_value(&args, "--new-epoch");
    let identity_key = arg_value(&args, "--identity-key");

    match cmd {
        "init" => {
            let d = db.ok_or("缺 --db".to_string())?;
            let c = catalog.ok_or("缺 --catalog".to_string())?;
            cmd_init(&d, &session, &c)
        }
        "reset" => {
            let d = db.ok_or("缺 --db".to_string())?;
            let c = catalog.ok_or("缺 --catalog".to_string())?;
            cmd_reset(&d, &session, &c)
        }
        "accept" => {
            let d = db.ok_or("缺 --db".to_string())?;
            let i = input.ok_or("缺 --input".to_string())?;
            let p = profile.ok_or("缺 --profile".to_string())?;
            cmd_accept(&d, &i, &p, &configured_epoch)
        }
        "advance" => cmd_advance(&db.ok_or("缺 --db".to_string())?, &configured_epoch),
        "catalog" => cmd_catalog(&db.ok_or("缺 --db".to_string())?, as_of),
        "snapshot" => cmd_snapshot(&db.ok_or("缺 --db".to_string())?, as_of),
        "financial-basis" => cmd_financial_basis(&db.ok_or("缺 --db".to_string())?),
        "meta" => cmd_meta(&db.ok_or("缺 --db".to_string())?),
        "pragma" => cmd_pragma(&db.ok_or("缺 --db".to_string())?),
        "query" => {
            let k = identity_key.ok_or("query 缺 --identity-key".to_string())?;
            cmd_query(&db.ok_or("缺 --db".to_string())?, &k)
        }
        "watch" => {
            let after = arg_value(&args, "--after-generation")
                .map(|s| parse_canonical_i64(&s, "--after-generation"))
                .transpose()?
                .unwrap_or(0);
            if after < 0 {
                return Err("InvalidDomain：--after-generation 必须 >= 0".to_string());
            }
            cmd_watch(&db.ok_or("缺 --db".to_string())?, after)
        }
        "recover" => {
            let n = new_epoch.ok_or("recover 缺 --new-epoch".to_string())?;
            cmd_recover(&db.ok_or("缺 --db".to_string())?, &n)
        }
        other => Err(format!("未知子命令 {other}\n{}", usage())),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", json!({"ok": false, "error": e}));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    pub(super) struct TestFiles(std::path::PathBuf);

    impl TestFiles {
        pub(super) fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let number = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("s1371-{}-{stamp}-{number}", std::process::id()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        pub(super) fn db(&self) -> std::path::PathBuf {
            self.0.join("结构 ?#%.sqlite")
        }
    }

    impl Drop for TestFiles {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(super) fn fixture(path: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session")
            .join(path)
    }

    pub(super) fn init_test_db(files: &TestFiles) {
        cmd_init(
            &files.db(),
            "s-session-testonly-001",
            &fixture("catalog/signed-catalog.json"),
        )
        .unwrap();
    }

    fn set_test_begin(conn: &Connection) {
        meta_set(conn, "writer_epoch", DEFAULT_WRITER_EPOCH).unwrap();
        meta_set(conn, "generation", "0").unwrap();
        meta_set(conn, "advance_state", "begun:owned:1:6").unwrap();
        meta_set(conn, "begin_token", "owned").unwrap();
    }

    fn ev(id: &str, rev: &str, seq: &str, price: &str) -> Value {
        json!({
            "event_id": id,
            "revision": rev,
            "seq": seq,
            "received_at": "2026-09-09T00:00:00.000Z",
            "raw_text": price,
            "price": price,
            "timestamp": seq,
            "volume": "1",
        })
    }

    fn test_input(events: &[Value]) -> Value {
        json!({
            "schema_revision": "1",
            "session_id": "s-session-testonly-001",
            "source_namespace": "testonly.tick.ohlc",
            "source_epoch": "1",
            "instrument": "TEST.TICK",
            "profile": "testonly_tick_1_1_ohlc",
            "events": events,
        })
    }

    pub(super) fn write_input(files: &TestFiles, name: &str, value: &Value) -> std::path::PathBuf {
        let p = files.0.join(name);
        std::fs::write(&p, value.to_string()).unwrap();
        p
    }

    fn accept(files: &TestFiles, value: &Value, name: &str, epoch: &str) {
        let p = write_input(files, name, value);
        cmd_accept(
            &files.db(),
            &p,
            &fixture("profiles/testonly_tick_1_1_ohlc.json"),
            epoch,
        )
        .unwrap();
    }

    fn snapshot_of(files: &TestFiles, as_of: Option<i64>) -> Value {
        let conn = open_db(&files.db()).unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        let out = read_snapshot_in_tx(&tx, as_of).unwrap();
        drop(tx);
        out
    }

    fn ohlc_input(bars: &[(i64, i64)], revision: bool) -> Value {
        let events:Vec<Value>=bars.iter().enumerate().map(|(i,(low,high))|json!({
            "event_id":format!("o{i}"),"revision":if revision && i==2{"2"}else{"1"},"seq":i.to_string(),
            "received_at":"2026-09-13T00:00:00Z","raw_text":format!("{low}/{high}"),
            "open":low.to_string(),"high":high.to_string(),"low":low.to_string(),"close":high.to_string(),
            "timestamp":i.to_string(),"volume":"1"})).collect();
        json!({"schema_revision":"s-ohlc/1","session_id":"s-session-testonly-001","source_namespace":"testonly.ohlc",
            "source_epoch":"1","instrument":"TEST.OHLC","profile":tb02a::PROFILE,"events":events})
    }
    pub(super) fn accept_ohlc(files: &TestFiles, input: &Value) {
        accept_core(
            &files.db(),
            &canonical_json(input),
            &fixture("profiles/ohlc_integer_tb02a_v1.json"),
            DEFAULT_WRITER_EPOCH,
            None,
        )
        .unwrap();
    }

    /// #1392 独立 oracle 的具名 OHLC（A_STRICT_HIGHER_TOP + OPPOSITE_FORMATION_COMPONENT +
    /// RIGHT_GROUP_SEALED_COMPONENT，逐根 H/L 抄自 ORACLE.json），`source_coord` ×10 以留出
    /// 70 与 80 之间的历史插入位。O=C=floor((H+L)/2)。
    pub(super) fn tb02b_oracle_rows() -> Vec<(&'static str, i64, i64, i64, i64, i64)> {
        vec![
            ("b0", 0, 8, 10, 7, 8),
            ("b1", 10, 6, 8, 5, 6),
            ("b2", 20, 10, 12, 8, 10),
            ("b3", 30, 10, 11, 9, 10),
            ("b4", 40, 12, 14, 11, 12),
            ("b5", 50, 15, 17, 14, 15),
            ("b6", 60, 13, 15, 12, 13),
            ("b7", 70, 17, 19, 16, 17),
            ("b8", 80, 14, 16, 13, 14),
            ("b9", 90, 12, 14, 10, 12),
            ("b10", 100, 10, 12, 8, 10),
            ("b11", 110, 8, 10, 6, 8),
            ("b12", 120, 11, 13, 9, 11),
            ("b13", 130, 13, 15, 11, 13),
        ]
    }

    #[test]
    fn tb02c_persistent_first_kind_and_normal_recovery() {
        let ledger: Value = serde_json::from_slice(
            &std::fs::read(fixture("tests/fixtures/tb02c/raw-ledger.json")).unwrap(),
        )
        .unwrap();
        for case in ["first_up", "first_down"] {
            let files = TestFiles::new();
            init_test_db(&files);
            let rows = ledger["cases"][case].as_array().unwrap();
            for count in 1..=rows.len() {
                let prices: Vec<_> = rows[..count]
                    .iter()
                    .map(|r| (r["low"].as_i64().unwrap(), r["high"].as_i64().unwrap()))
                    .collect();
                let mut input = ohlc_input(&prices, false);
                for (i, row) in rows[..count].iter().enumerate() {
                    for key in ["open", "close"] {
                        input["events"][i][key] = json!(row[key].as_i64().unwrap().to_string());
                    }
                }
                accept_ohlc(&files, &input);
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
            let current = snapshot_of(&files, None);
            let old = snapshot_of(&files, Some(16));
            let data = |snapshot: &Value| {
                snapshot["objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| {
                        o["kind"] == "CC-013.segment"
                            && o["payload"]["data"]["entity_id"] == "segment:1:1"
                    })
                    .unwrap()["payload"]["data"]
                    .clone()
            };
            assert_eq!(data(&old)["result"], "NO_FIRST");
            assert_eq!(data(&current)["result"], "CASE_ONE");
            assert_eq!(data(&current)["geometric_end"]["group_anchor"], "13");
            cmd_recover(&files.db(), "2").unwrap();
            assert_eq!(snapshot_of(&files, Some(16)), old);
            assert_eq!(snapshot_of(&files, None), current);
            let out = std::process::Command::new("python3")
                .arg(fixture("tests/tb02c_verify.py"))
                .arg("--database")
                .arg(files.db())
                .arg("--case")
                .arg(case)
                .arg("--node")
                .arg("node")
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{case}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    /// 显式 (event_id, source_coord, open, high, low, close) 的 OHLC 输入；业务身份由 event_id
    /// 固定，历史插入不改旧身份，故不得按数组下标命名。
    pub(super) fn located_ohlc_input(rows: &[(&str, i64, i64, i64, i64, i64)]) -> Value {
        let events: Vec<Value> = rows
            .iter()
            .map(|(id, coord, open, high, low, close)| {
                json!({"event_id":id,"revision":"1","seq":coord.to_string(),
                    "received_at":"2026-09-13T00:00:00Z","raw_text":format!("{low}/{high}"),
                    "open":open.to_string(),"high":high.to_string(),
                    "low":low.to_string(),"close":close.to_string(),
                    "timestamp":coord.to_string(),"volume":"1"})
            })
            .collect();
        json!({"schema_revision":"s-ohlc/1","session_id":"s-session-testonly-001",
            "source_namespace":"testonly.ohlc","source_epoch":"1","instrument":"TEST.OHLC",
            "profile":tb02a::PROFILE,"events":events})
    }

    fn bi_object(snap: &Value, entity: &str) -> Option<Value> {
        snap["objects"].as_array().unwrap().iter().find_map(|o| {
            (o["kind"] == "CC-010.bi" && o["payload"]["data"]["entity_id"] == entity)
                .then(|| o["payload"]["data"].clone())
        })
    }

    #[test]
    fn tb02b_persisted_identity_lifecycle_revision_and_old_cuts() {
        let files = TestFiles::new();
        init_test_db(&files);
        let prices = [10, 5, 8, 11, 14, 17, 13, 10, 7, 3, 6, 9];
        let bars: Vec<_> = prices.iter().map(|h| (h - 2, *h)).collect();
        let mut snapshots = vec![];
        for n in 1..=bars.len() {
            accept_ohlc(&files, &ohlc_input(&bars[..n], false));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            let snap = snapshot_of(&files, None);
            if n >= 7 {
                let bi = snap["objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| {
                        o["kind"] == "CC-010.bi" && o["payload"]["data"]["entity_id"] == "bi:1:1"
                    })
                    .unwrap();
                assert_eq!(bi["payload"]["data"]["formed_known_at"]["generation"], "7");
                assert_eq!(
                    bi["payload"]["data"]["state"],
                    if n == 12 {
                        "CONFIRMED"
                    } else {
                        "FORMED_UNCONFIRMED"
                    }
                );
                if n == 12 {
                    assert_eq!(
                        bi["payload"]["data"]["confirmed_known_at"]["generation"],
                        "12"
                    );
                }
            }
            snapshots.push(snapshot_of(&files, Some(n as i64)));
        }
        let mut revised = ohlc_input(&bars, false);
        revised["events"][5]["revision"] = json!("2");
        revised["events"][5]["high"] = json!("18");
        revised["events"][5]["close"] = json!("18");
        revised["events"][5]["raw_text"] = json!("15/18 revision");
        accept_ohlc(&files, &revised);
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        for (index, snap) in snapshots.iter().enumerate() {
            assert_eq!(&snapshot_of(&files, Some(index as i64 + 1)), snap);
        }
        let final_snapshot = snapshot_of(&files, None);
        assert!(final_snapshot["objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "CC-010.bi" && o["payload"]["data"]["entity_id"] == "bi:2:1"));
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let verifier = fixture("tests/tb02b_verify.py");
        let result = std::process::Command::new("python3")
            .arg(verifier)
            .arg("--database")
            .arg(files.db())
            .arg("--node")
            .arg("node")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "Q/前端合同真实库验证失败：{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn tb02b_same_input_extension_emits_all_changes() {
        let files = TestFiles::new();
        init_test_db(&files);
        let prices = [10, 5, 8, 11, 14, 17, 13, 18, 15];
        let bars: Vec<_> = prices.iter().map(|h| (h - 2, *h)).collect();
        for n in 1..=bars.len() {
            accept_ohlc(&files, &ohlc_input(&bars[..n], false));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let s = snapshot_of(&files, None);
        let changes: Vec<_> = s["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["kind"] == "CC-055.change_event")
            .map(|o| o["payload"]["data"]["change"].as_str().unwrap())
            .collect();
        assert!(changes.contains(&"extended"));
        assert!(changes.contains(&"endpoint_replaced"));
        let bi = s["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["kind"] == "CC-010.bi")
            .unwrap();
        assert_eq!(bi["payload"]["data"]["entity_id"], "bi:1:1");
        assert_eq!(bi["payload"]["data"]["end"]["group_anchor"], "7");
    }

    #[test]
    fn tb02b_historical_identity_insertion_opens_new_generation_and_keeps_old_cut() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let before = snapshot_of(&files, None);
        let old = bi_object(&before, "bi:1:10").expect("独立 oracle 前缀下 UP 笔必须存在");
        assert_eq!(old["end"]["group_anchor"], "70");
        assert_eq!(old["end"]["price"], "19");
        let old_confirmation = old["confirmation"].clone();
        let old_confirmed_known_at = old["confirmed_known_at"].clone();
        assert!(!old_confirmation.is_null(), "该端点须已确认");
        assert_eq!(
            old_confirmation["successor_start"],
            old["end"]["group_anchor"]
        );
        // 历史插入：新业务身份 rev1 落在已接纳过的坐标区间内（75 ∈ (70, 80)），旧 raw 不改。
        accept_ohlc(&files, &located_ohlc_input(&[("x75", 75, 22, 25, 20, 22)]));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let after = snapshot_of(&files, None);
        // 当前视图：被改写端点不再是原代际对象；新代际对象承载重建后的确认事实。
        assert!(
            bi_object(&after, "bi:1:10").is_none(),
            "历史插入必须退出原代际身份，不得就地改写"
        );
        let new = bi_object(&after, "bi:2:10").expect("历史插入须开新代际对象");
        assert_eq!(new["end"]["group_anchor"], "75");
        assert_eq!(new["end"]["price"], "25");
        assert_ne!(new["confirmed_known_at"], old_confirmed_known_at);
        if !new["confirmation"].is_null() {
            // 证书必须与当前端点同源：不得把 70 端点的旧证书贴到 75。
            assert_eq!(
                new["confirmation"]["successor_start"],
                new["end"]["group_anchor"]
            );
            assert_eq!(new["confirmed_known_at"]["generation"], "2");
        }
        let withdrawal = after["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| {
                o["kind"] == "CC-055.change_event"
                    && o["payload"]["data"]["entity_id"] == "bi:1:10"
                    && o["payload"]["data"]["change"] == "withdrawn"
            })
            .expect("原代际对象须有撤回事件");
        assert!(withdrawal["payload"]["data"]["causes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "raw_fact_revision"));
        // 旧 AsKnown cut 保留 19 及当时证书。
        let as_known = snapshot_of(&files, Some(1));
        let old_cut = bi_object(&as_known, "bi:1:10").expect("旧 cut 必须保留原对象");
        assert_eq!(old_cut["end"]["group_anchor"], "70");
        assert_eq!(old_cut["end"]["price"], "19");
        assert_eq!(old_cut["confirmation"], old_confirmation);
        assert_eq!(old_cut["confirmed_known_at"], old_confirmed_known_at);
    }

    #[test]
    fn tb02b_same_generation_tail_append_keeps_first_confirmation() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let before = bi_object(&snapshot_of(&files, None), "bi:1:10").unwrap();
        // 普通尾部追加：新身份落在既有最大坐标之后（140 > 130），落在尾组内不封新组。
        accept_ohlc(
            &files,
            &located_ohlc_input(&[("x140", 140, 13, 15, 12, 13)]),
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let after =
            bi_object(&snapshot_of(&files, None), "bi:1:10").expect("同代际正常追加不得换身份");
        assert_eq!(after["confirmed_known_at"], before["confirmed_known_at"]);
        assert_eq!(after["confirmation"], before["confirmation"]);
        assert_eq!(after["formed_known_at"], before["formed_known_at"]);
    }

    #[test]
    fn tb02b_successor_extension_preserves_confirmation_and_relation() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let original = snapshot_of(&files, None);
        let historical = snapshot_of(&files, Some(1));
        let before = bi_object(&original, "bi:1:10").unwrap();
        assert_eq!(before["state"], "CONFIRMED");
        // #1392：raw13 顶的反向间距不足，被跳过；raw15 底更低，延伸后继 DOWN。
        // 它的右邻 raw16 尚未封口，但此前 UP 的 raw11/13 证书仍有效。
        accept_ohlc(
            &files,
            &located_ohlc_input(&[
                ("b14", 140, 9, 11, 7, 9),
                ("b15", 150, 5, 7, 3, 5),
                ("b16", 160, 7, 9, 5, 7),
            ]),
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let current = snapshot_of(&files, None);
        let after = bi_object(&current, "bi:1:10").unwrap();
        assert_eq!(after["end"]["group_anchor"], "70");
        assert_eq!(after["confirmation"], before["confirmation"]);
        assert_eq!(after["confirmed_known_at"], before["confirmed_known_at"]);
        let successor = bi_object(&current, "bi:1:70").unwrap();
        assert_eq!(successor["end"]["group_anchor"], "150");
        assert!(successor["end"]["sealed_at"].is_null());
        assert!(
            current["objects"].as_array().unwrap().iter().any(|o| {
                let d = &o["payload"]["data"];
                o["kind"] == "CC-055.relation"
                    && d["source_id"] == "bi:1:70"
                    && d["relation_kind"] == "confirms"
                    && d["target_id"] == "bi:1:10"
            }),
            "已保留的确认证书必须仍有同源 confirms 关系"
        );
        assert!(
            snapshot_of(&files, Some(1)) == historical,
            "旧 AsKnown 快照不得被后续追加改写"
        );
    }

    #[test]
    fn tb02b_revision_of_inserted_identity_opens_another_generation() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept_ohlc(&files, &located_ohlc_input(&[("x75", 75, 22, 25, 20, 22)]));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let inserted =
            bi_object(&snapshot_of(&files, None), "bi:2:10").expect("历史插入须开二代际");
        // 对已被历史插入占用的坐标再做同身份修订：有效事实集只剩最新 revision，看不出
        // 「先按序接纳再修订」与「历史插入后再修订」的差别——只看 Σ(revision−1) 会漏换代，
        // 同一「贴旧确认」路径复发；须由完整接纳历史计数。
        let mut revision = located_ohlc_input(&[("x75", 75, 23, 26, 21, 23)]);
        revision["events"][0]["revision"] = json!("2");
        accept_ohlc(&files, &revision);
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let after = snapshot_of(&files, None);
        assert!(
            bi_object(&after, "bi:2:10").is_none(),
            "再修订必须开新代际，不得就地改写二代际对象"
        );
        let third = bi_object(&after, "bi:3:10").expect("再修订须开三代际");
        assert_eq!(third["end"]["group_anchor"], "75");
        assert_eq!(third["end"]["price"], "26");
        let kept =
            bi_object(&snapshot_of(&files, Some(2)), "bi:2:10").expect("二代际 cut 保留当时对象");
        assert_eq!(kept["end"]["price"], inserted["end"]["price"]);
        assert_eq!(kept["confirmation"], inserted["confirmation"]);
    }

    #[test]
    fn tb02b_batched_history_revision_and_tail_keep_distinct_fact_generation() {
        for publish_revision_separately in [false, true] {
            let files = TestFiles::new();
            init_test_db(&files);
            accept_ohlc(&files, &located_ohlc_input(&tb02b_oracle_rows()));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            accept_ohlc(&files, &located_ohlc_input(&[("x75", 75, 22, 25, 20, 22)]));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            let old_cut = snapshot_of(&files, Some(2));
            let old = bi_object(&old_cut, "bi:2:10").unwrap();

            let mut revision = located_ohlc_input(&[("x75", 75, 23, 26, 21, 23)]);
            revision["events"][0]["revision"] = json!("2");
            accept_ohlc(&files, &revision);
            if publish_revision_separately {
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
            accept_ohlc(
                &files,
                &located_ohlc_input(&[("x140", 140, 13, 15, 12, 13)]),
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            let current = snapshot_of(&files, None);
            assert!(
                bi_object(&current, "bi:2:10").is_none(),
                "同批尾部追加不得遮蔽历史修订"
            );
            let new = bi_object(&current, "bi:3:10").expect("分批方式不得改变事实代际");
            assert_eq!(new["end"]["price"], "26");
            assert_ne!(new["formed_known_at"], old["formed_known_at"]);
            assert_eq!(snapshot_of(&files, Some(2)), old_cut);
            let version = current["objects"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["kind"] == "CC-056.version" && o["payload"]["slot"] == "TB-02-B")
                .unwrap();
            assert_eq!(
                version["payload"]["data"]["version_causes"],
                if publish_revision_separately {
                    json!(["append"])
                } else {
                    json!(["raw_fact_revision"])
                }
            );

            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            assert_eq!(snapshot_of(&files, None), current);
            cmd_recover(&files.db(), "2").unwrap();
            cmd_advance(&files.db(), "2").unwrap();
            assert_eq!(
                snapshot_of(&files, None)["generation"],
                current["generation"]
            );
            assert_eq!(bi_object(&snapshot_of(&files, None), "bi:3:10"), Some(new));
            assert_eq!(
                bi_object(&snapshot_of(&files, Some(2)), "bi:2:10"),
                Some(old)
            );
        }
    }

    #[test]
    fn tb02a_strict_schema_discriminator_and_numeric_domain() {
        let valid = ohlc_input(&[(5, 10), (6, 12), (7, 11)], false);
        assert!(tb02a::parse_input(&valid).is_ok());
        for (key, value) in [
            ("schema_revision", json!("1")),
            ("profile", json!("testonly_tick_1_1_ohlc")),
        ] {
            let mut bad = valid.clone();
            bad[key] = value;
            assert!(tb02a::parse_input(&bad).is_err());
        }
        for (key, value) in [
            ("price", json!("10")),
            ("high", json!(10)),
            ("open", json!("05")),
            ("low", json!("13")),
        ] {
            let mut bad = valid.clone();
            bad["events"][0][key] = value;
            assert!(tb02a::parse_input(&bad).is_err());
        }
    }

    #[test]
    fn tb02a_sealed_revision_history_and_independent_fact_index() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &ohlc_input(&[(5, 10), (6, 12), (7, 11)], false));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let old = snapshot_of(&files, Some(1));
        assert!(!old["objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "CC-006.local_shape"));
        assert!(old["objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "CC-005.inclusion_group"
                && o["payload"]["members"] == json!(["1", "2"])));
        assert!(old["raw_history"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.get("price").is_none() && r["schema_revision"] == tb02a::RAW_SCHEMA));
        accept_ohlc(&files, &ohlc_input(&[(5, 10), (6, 12), (4, 9)], true));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(snapshot_of(&files, Some(1)), old);
        let now = snapshot_of(&files, None);
        assert!(now["objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "CC-006.local_shape" && o["branch"] == "TOP"));
        assert!(now["withdrawn_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["kind"] == "CC-005.inclusion_group"));
        let delta: Value = delta_of(&conn, 2);
        assert_eq!(
            delta["catalog_evidence"]["axes"]["CC-056"]["run_status"],
            "run"
        );
        assert_eq!(
            delta["catalog_evidence"]["axes"]["CC-056"]["evidence"]["raw_revisions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        conn.execute("UPDATE structure_facts SET payload_json='{}' WHERE kind='CC-005.inclusion_group' AND withdrawn_generation IS NULL",[]).unwrap();
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
    }

    #[test]
    fn tb02a_three_groups_with_equal_roots_wait_for_exact_shape_request() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(
            &files,
            &ohlc_input(&[(0, 1), (3, 10), (5, 10), (20, 30)], false),
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        verify_reachable_root(&open_db(&files.db()).unwrap()).unwrap();
        let snapshot = snapshot_of(&files, None);
        let objects = snapshot["objects"].as_array().unwrap();
        assert_eq!(
            objects
                .iter()
                .filter(|o| o["kind"] == "CC-005.inclusion_group")
                .count(),
            3
        );
        assert!(objects.iter().any(|o| o["kind"] == "CC-005.inclusion_group"
            && o["payload"]["high_sources"] == json!(["1", "2"])));
        assert!(!objects.iter().any(|o| o["kind"] == "CC-006.local_shape"));
        let knowledge = objects
            .iter()
            .find(|o| o["kind"] == "CC-054.knowledge_state")
            .unwrap();
        let slot = canonical_json(&json!(["CC-006.local_shape", ["0", "1", "3"]]));
        let request = knowledge["payload"]["requests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["request_id"] == slot)
            .unwrap();
        assert!(request["subject_id"].is_null());
        assert_eq!(request["reasons"], json!(["equal_extreme_identity"]));
        assert_eq!(request["source_coords"], json!(["0", "1", "2", "3"]));
        assert_eq!(request["axes"]["computation"], "INCOMPLETE");
        let conn = open_db(&files.db()).unwrap();
        let evidence: String = conn
            .query_row(
                "SELECT catalog_evidence_json FROM structure_deltas WHERE generation=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let evidence: Value = serde_json::from_str(&evidence).unwrap();
        assert_eq!(evidence["axes"]["CC-006"]["run_status"], "waiting");
    }

    #[test]
    fn tb02a_window_external_direction_revision_disappearance_and_return() {
        let files = TestFiles::new();
        init_test_db(&files);
        let base = [(0, 1), (4, 8), (5, 7), (2, 4), (6, 9)];
        let mut histories = Vec::new();
        let mut shape_ids = Vec::new();
        let mut description_ids = Vec::new();
        for (generation, first) in [(1, (0, 1)), (2, (0, 2)), (3, (0, 10)), (4, (0, 1))] {
            let mut bars = base;
            bars[0] = first;
            let mut input = ohlc_input(&bars, false);
            input["events"][0]["revision"] = json!(generation.to_string());
            accept_ohlc(&files, &input);
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            verify_reachable_root(&open_db(&files.db()).unwrap()).unwrap();
            let snapshot = snapshot_of(&files, None);
            for (i, old) in histories.iter().enumerate() {
                assert_eq!(&snapshot_of(&files, Some((i + 1) as i64)), old);
            }
            let objects = snapshot["objects"].as_array().unwrap();
            let shape = objects.iter().find(|o| {
                o["kind"] == "CC-006.local_shape"
                    && o["window_start"] == "1"
                    && o["window_mid"] == "3"
                    && o["window_end"] == "4"
            });
            if generation == 3 {
                assert!(shape.is_none());
                assert!(snapshot["withdrawn_objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|o| shape_ids.contains(&o["object_id"])));
            } else {
                let shape = shape.unwrap();
                assert_eq!(shape["branch"], "BOTTOM");
                assert_eq!(shape["source_coords"], json!(["0", "1", "2", "3", "4"]));
                let first_group = &shape["input_refs"][0];
                assert_eq!(
                    first_group["raw_refs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r["source_coord"].clone())
                        .collect::<Vec<_>>(),
                    vec![json!("1"), json!("2")]
                );
                assert!(first_group["dependency_refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["source_coord"] == "0" && r["revision"] == generation.to_string()));
                assert!(!shape_ids.contains(&shape["object_id"]));
                shape_ids.push(shape["object_id"].clone());
                let description = objects
                    .iter()
                    .find(|o| {
                        o["kind"] == "CC-007.fractal_description"
                            && o["payload"]["shape_object_id"] == shape["object_id"]
                    })
                    .unwrap();
                assert!(description["input_refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["source_coord"] == "0" && r["revision"] == generation.to_string()));
                assert!(!description_ids.contains(&description["object_id"]));
                description_ids.push(description["object_id"].clone());
                let knowledge = objects
                    .iter()
                    .find(|o| o["kind"] == "CC-054.knowledge_state")
                    .unwrap();
                for request in knowledge["payload"]["requests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| {
                        r["subject_id"] == shape["object_id"]
                            || r["subject_id"] == description["object_id"]
                    })
                {
                    let request_key: Value =
                        serde_json::from_str(request["request_id"].as_str().unwrap()).unwrap();
                    if request_key[1] == "descriptive_labels" {
                        assert_eq!(request["source_coords"], json!(["1", "2", "3", "4"]));
                    } else {
                        assert!(request["source_coords"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("0")));
                    }
                }
                // 仅重放对象声明的计算输入，仍恢复同一真实三锚 BOTTOM。
                let replay: Vec<Bar> = shape["source_coords"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|coord| {
                        let i = coord.as_str().unwrap().parse::<usize>().unwrap();
                        let (low, high) = bars[i];
                        Bar {
                            source_index: i,
                            timestamp: i as i64,
                            open: low,
                            high,
                            low,
                            close: high,
                            volume: 1.0,
                            untradable: false,
                        }
                    })
                    .collect();
                let (layer, facts) =
                    parser::parse_layer_with_inclusion_facts(&replay, &ThetaConfig::default());
                assert!(!facts.initial_direction_unsettled);
                assert!(classify_local_shape_sliding(&layer.merged_bars).iter().any(|(i,shape)|layer.merged_bars[*i].source_index==1 && matches!(shape,Cc006LocalShape::Classified{branch,..} if branch.as_str()=="BOTTOM")));
            }
            histories.push(snapshot_of(&files, Some(generation)));
        }
    }

    #[test]
    fn tb02a_restored_numeric_shape_has_new_boundary_identity_without_reviving_history() {
        let files = TestFiles::new();
        init_test_db(&files);
        let stages: Vec<Vec<(i64, i64)>> = vec![
            vec![(0, 1), (2, 3), (4, 6)],
            vec![(0, 1), (2, 3), (4, 6), (5, 5)],
            vec![(0, 1), (2, 3), (4, 6), (7, 8)],
        ];
        let mut histories = Vec::new();
        let mut shapes = Vec::new();
        for (index, bars) in stages.iter().enumerate() {
            let generation = (index + 1) as i64;
            let mut input = ohlc_input(bars, false);
            if generation == 3 {
                input["events"][3]["revision"] = json!("2");
            }
            accept_ohlc(&files, &input);
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            verify_reachable_root(&open_db(&files.db()).unwrap()).unwrap();
            for (old_index, old) in histories.iter().enumerate() {
                assert_eq!(&snapshot_of(&files, Some((old_index + 1) as i64)), old);
            }
            let snapshot = snapshot_of(&files, Some(generation));
            let shape = snapshot["objects"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| {
                    o["kind"] == "CC-006.local_shape"
                        && o["window_start"] == "0"
                        && o["window_mid"] == "1"
                        && o["window_end"] == "2"
                })
                .unwrap();
            assert_eq!(shape["branch"], "RISING");
            assert!(shapes
                .iter()
                .all(|old: &Value| old["object_id"] != shape["object_id"]));
            shapes.push(shape.clone());
            histories.push(snapshot);
        }
        assert_eq!(shapes[0]["comparisons"], shapes[2]["comparisons"]);
        assert_eq!(shapes[0]["source_coords"], json!(["0", "1", "2"]));
        assert_eq!(shapes[2]["source_coords"], json!(["0", "1", "2", "3"]));
        assert_eq!(
            shapes[2]["input_refs"][2]["raw_refs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["source_coord"].clone())
                .collect::<Vec<_>>(),
            vec![json!("2")]
        );
        assert!(shapes[2]["input_refs"][2]["dependency_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["source_coord"] == "3" && r["revision"] == "2"));
        assert!(histories[2]["withdrawn_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["object_id"] == shapes[0]["object_id"] && o["withdrawn_generation"] == "2"));
        assert_eq!(shapes[2]["first_known_generation"], "3");
    }

    #[test]
    fn tb02a_raw_sidecar_tampering_is_not_tick_fallback() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &ohlc_input(&[(5, 10), (6, 12)], false));
        let conn = open_db(&files.db()).unwrap();
        conn.execute("UPDATE raw_ohlc SET high='99' WHERE revision=1", [])
            .unwrap();
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
    }

    #[test]
    fn r9_prefix_observation_retains_sealed_first_version() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[ev("e0", "1", "0", "10000")]),
            "one.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let before = snapshot_of(&files, Some(1));
        accept(
            &files,
            &test_input(&[ev("e0", "1", "0", "10000"), ev("e1", "1", "1", "11000")]),
            "two.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let delta: Value = delta_of(&conn, 2);
        // #1394 changes-only 域：Delta 只含本代首次封存的观察。首次版本仍完整保留在持久索引与
        // 同 cut 快照里（含首次 batch），只是不再逐代重发——这正是「未变对象不重发」的收益面。
        let before_ids: std::collections::BTreeSet<String> = before["observations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["observation_id"].as_str().unwrap().to_string())
            .collect();
        for ob in delta["observations"].as_array().unwrap() {
            assert!(
                !before_ids.contains(ob["observation_id"].as_str().unwrap()),
                "旧观察不得重发到本代 Delta：{ob}"
            );
        }
        let after = snapshot_of(&files, None);
        let find = |snap: &Value, oid: &str| {
            snap["observations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["observation_id"].as_str() == Some(oid))
                .cloned()
        };
        for oid in &before_ids {
            assert_eq!(
                find(&after, oid),
                find(&before, oid),
                "旧观察必须逐字段保留首次封存版本（含首次 batch_id）"
            );
        }
        assert_eq!(snapshot_of(&files, Some(1)), before);
    }

    #[test]
    fn r9_all_required_delta_collections_and_scope_gate_writes() {
        for key in [
            "upserts",
            "withdrawals",
            "replaces",
            "witnesses",
            "relations",
            "observations",
            "raw_history_added",
            "scope",
        ] {
            let files = TestFiles::new();
            init_test_db(&files);
            accept(
                &files,
                &test_input(&[
                    ev("e0", "1", "0", "10000"),
                    ev("e1", "1", "1", "11000"),
                    ev("e2", "1", "2", "10500"),
                ]),
                "input.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            let conn = open_db(&files.db()).unwrap();
            if key == "scope" {
                conn.execute("DELETE FROM meta WHERE key='scope'", [])
                    .unwrap();
            } else {
                // #1394 第二段：缺必需集合的 Delta 原件必须整体替换该代私有根（图域）/
                // 行内原文（整块域），两种物理表示下都构成同一处语义缺陷。
                let mut broken = delta_of(&conn, 1);
                broken
                    .as_object_mut()
                    .unwrap()
                    .remove(&key.to_string())
                    .unwrap();
                set_delta(&conn, 1, &broken);
            }
            assert!(cmd_snapshot(&files.db(), None).is_err(), "{key}");
            assert!(cmd_catalog(&files.db(), Some(0)).is_err(), "{key}");
            assert!(cmd_watch(&files.db(), 0).is_err(), "{key}");
            assert!(
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).is_err(),
                "{key}"
            );
            assert!(cmd_recover(&files.db(), "2").is_err(), "{key}");
        }
    }

    #[test]
    fn r10_as_known_zero_is_stable_through_accept_publish_and_revision() {
        let files = TestFiles::new();
        init_test_db(&files);
        let zero = snapshot_of(&files, Some(0));
        let conn = open_db(&files.db()).unwrap();
        let catalog = read_catalog_in_tx(&conn, Some(0)).unwrap();
        drop(conn);
        for (i, events) in [
            vec![ev("e0", "1", "0", "10000")],
            vec![ev("e1", "1", "1", "11000")],
            vec![ev("e2", "1", "2", "10500")],
            vec![ev("e2", "2", "2", "11500")],
            vec![ev("e3", "1", "3", "10800")],
        ]
        .iter()
        .enumerate()
        {
            accept(
                &files,
                &test_input(events),
                &format!("{i}.json"),
                DEFAULT_WRITER_EPOCH,
            );
            assert_eq!(snapshot_of(&files, Some(0)), zero);
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            assert_eq!(snapshot_of(&files, Some(0)), zero);
            let conn = open_db(&files.db()).unwrap();
            assert_eq!(read_catalog_in_tx(&conn, Some(0)).unwrap(), catalog);
        }
    }

    #[test]
    fn r10_pending_profile_corruption_fails_before_any_write() {
        for published in [false, true] {
            for key in ["profile_id", "profile_hash", "input_profile"] {
                let files = TestFiles::new();
                init_test_db(&files);
                accept(
                    &files,
                    &test_input(&[ev("e0", "1", "0", "10000")]),
                    "one.json",
                    DEFAULT_WRITER_EPOCH,
                );
                if published {
                    cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
                    accept(
                        &files,
                        &test_input(&[ev("e1", "1", "1", "11000")]),
                        "two.json",
                        DEFAULT_WRITER_EPOCH,
                    );
                }
                let conn = open_db(&files.db()).unwrap();
                meta_set(&conn, key, &"f".repeat(64)).unwrap();
                let before = db_meta(&conn).unwrap();
                assert!(cmd_snapshot(&files.db(), None)
                    .unwrap_err()
                    .contains("StorageUnavailable"));
                assert!(cmd_catalog(&files.db(), Some(0))
                    .unwrap_err()
                    .contains("StorageUnavailable"));
                assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
                    .unwrap_err()
                    .contains("StorageUnavailable"));
                assert!(cmd_recover(&files.db(), "2")
                    .unwrap_err()
                    .contains("StorageUnavailable"));
                assert_eq!(db_meta(&conn).unwrap(), before);
            }
        }
    }

    #[test]
    fn full_signed_catalog_preserves_both_declared_branch_shapes() {
        let files = TestFiles::new();
        init_test_db(&files);
        let conn = open_db(&files.db()).unwrap();
        let signed: Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("catalog/signed-catalog.json")).unwrap(),
        )
        .unwrap();
        let actual = read_catalog_in_tx(&conn, None).unwrap();
        let expected = signed["items"].as_array().unwrap();
        let rows = actual["items"].as_array().unwrap();
        assert_eq!(expected.len(), 116);
        assert_eq!(rows.len(), expected.len());
        assert_eq!(
            expected.iter().filter(|r| r["branches"].is_array()).count(),
            82
        );
        assert_eq!(
            expected
                .iter()
                .filter(|r| r["branches"].is_object())
                .count(),
            34
        );
        for item in expected {
            let row = rows.iter().find(|r| r["id"] == item["id"]).unwrap();
            assert_eq!(row["branches"], item["branches"]);
            assert_eq!(row["title"], item["title"]);
            assert_eq!(row["domain"], item["domain"]);
            assert_eq!(row["implementation_status"], json!("not_implemented"));
        }
        for bad in ["null", "true", "1", "\"text\"", "{"] {
            assert!(json_shape(bad, JsonShape::CatalogBranches).is_err());
        }
    }

    #[test]
    fn complete_batch_is_visible_before_reachable_root_and_never_overwritten() {
        let files = TestFiles::new();
        init_test_db(&files);
        let mut conn = open_db(&files.db()).unwrap();
        set_test_begin(&conn);
        let bytes = br#"{"objects":[],"raw_events":[],"scope":"test-phase-boundary"}"#;
        let id = format!("batch-{}", sha256_hex(bytes));
        let value: Value = serde_json::from_slice(bytes).expect("测试批次是可解析 JSON");
        assert_eq!(canonical_bytes_of_value(&value), bytes.as_slice());
        persist_batch_before_publish(
            &mut conn,
            "owned",
            1,
            6,
            &id,
            bytes,
            &value,
            &[],
            DEFAULT_WRITER_EPOCH,
        )
        .unwrap();
        let reader =
            Connection::open_with_flags(&files.db(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        verify_batch_bytes(&reader, &id, bytes).unwrap();
        assert_eq!(
            meta_get_opt(&reader, "structure_cut").unwrap().as_deref(),
            Some("cut-0")
        );
        assert_eq!(
            meta_get_opt(&reader, "index_frontier").unwrap().as_deref(),
            Some("")
        );
        assert_eq!(
            meta_get_opt(&reader, "advance_state").unwrap().as_deref(),
            Some("begun:owned:1:6")
        );
        assert!(persist_batch_before_publish(
            &mut conn,
            "owned",
            1,
            6,
            &id,
            b"changed",
            &value,
            &[],
            DEFAULT_WRITER_EPOCH
        )
        .is_err());
        verify_batch_bytes(&reader, &id, bytes).unwrap();
        conn.execute(
            "UPDATE batches SET canonical_bytes=?1 WHERE batch_id=?2",
            params![b"corrupt".as_slice(), id],
        )
        .unwrap();
        assert!(persist_batch_before_publish(
            &mut conn,
            "owned",
            1,
            6,
            &id,
            bytes,
            &value,
            &[],
            DEFAULT_WRITER_EPOCH
        )
        .is_err());
        assert_eq!(
            meta_get_opt(&reader, "structure_cut").unwrap().as_deref(),
            Some("cut-0")
        );
    }

    #[test]
    fn cancellation_preserves_foreign_begin_and_reports_actual_release() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        for (key, value) in [
            ("writer_epoch", "2"),
            ("begin_token", "foreign"),
            ("generation", "1"),
            ("advance_state", "begun:foreign:1:6"),
        ] {
            set_test_begin(&conn);
            meta_set(&conn, key, value).unwrap();
            let before = db_meta(&conn);
            assert!(!cancel_own_begin(&mut conn, "owned", 1, 6, DEFAULT_WRITER_EPOCH).unwrap());
            assert_eq!(db_meta(&conn), before);
        }
        set_test_begin(&conn);
        assert!(cancel_own_begin(&mut conn, "owned", 1, 6, DEFAULT_WRITER_EPOCH).unwrap());
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert!(!cancel_own_begin(&mut conn, "owned", 1, 6, DEFAULT_WRITER_EPOCH).unwrap());
    }

    #[test]
    fn accept_rejects_stale_epoch_without_writing_receipts_or_metadata() {
        let files = TestFiles::new();
        init_test_db(&files);
        let conn = open_db(&files.db()).unwrap();
        meta_set(&conn, "writer_epoch", "2").unwrap();
        let before = db_meta(&conn);
        let error = cmd_accept(
            &files.db(),
            &fixture("inputs/cc006_four_branch.json"),
            &fixture("profiles/testonly_tick_1_1_ohlc.json"),
            DEFAULT_WRITER_EPOCH,
        )
        .unwrap_err();
        assert!(error.contains("StaleWriter"));
        assert_eq!(db_meta(&conn), before);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn replay_does_not_rebind_profile_of_an_existing_cut() {
        let files = TestFiles::new();
        init_test_db(&files);
        let input = fixture("inputs/cc006_four_branch.json");
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        cmd_accept(&files.db(), &input, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let before = db_meta(&conn);
        let mut altered: Value =
            serde_json::from_str(&std::fs::read_to_string(&profile).unwrap()).unwrap();
        altered["note"] = json!("same profile_id, different canonical content");
        let other = files.0.join("changed-profile.json");
        std::fs::write(&other, altered.to_string()).unwrap();
        assert!(
            cmd_accept(&files.db(), &input, &other, DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("IdentityConflict")
        );
        assert_eq!(db_meta(&conn), before);
        cmd_accept(&files.db(), &input, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        assert_eq!(db_meta(&conn), before);
    }

    #[test]
    fn equal_prices_publish_domain_observation_and_corrupt_json_fails_read() {
        let files = TestFiles::new();
        init_test_db(&files);
        let mut input: Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("inputs/cc006_four_branch.json")).unwrap(),
        )
        .unwrap();
        let events = input["events"].as_array_mut().unwrap();
        events.truncate(3);
        for event in events {
            event["price"] = json!("10000");
            event["raw_text"] = json!("10000");
        }
        let source = files.0.join("equal.json");
        std::fs::write(&source, input.to_string()).unwrap();
        cmd_accept(
            &files.db(),
            &source,
            &fixture("profiles/testonly_tick_1_1_ohlc.json"),
            DEFAULT_WRITER_EPOCH,
        )
        .unwrap();
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let snapshot = read_snapshot_in_tx(&conn, None).unwrap();
        assert!(snapshot["objects"].as_array().unwrap().is_empty());
        assert_eq!(snapshot["observations"].as_array().unwrap().len(), 1);
        assert_eq!(
            snapshot["observations"][0]["kind"],
            json!("domain_not_satisfied")
        );
        conn.execute("UPDATE observations SET detail_json='{'", [])
            .unwrap();
        assert!(read_snapshot_in_tx(&conn, None).is_err());
        conn.execute(
            "UPDATE catalog SET branches_json='null' WHERE catalog_id='CC-006'",
            [],
        )
        .unwrap();
        assert!(read_catalog_in_tx(&conn, None).is_err());
    }

    #[test]
    fn wire_projection_rejects_corrupt_integer_types_without_mutating_input() {
        for value in [
            json!(1.5),
            json!(true),
            Value::Null,
            json!("1"),
            json!(9223372036854775808_u64),
        ] {
            assert!(num_to_str(&value).is_err());
            let refs = json!([{"merged_index": 0, "raw_refs": [{"seq": value}]}]);
            let unchanged = refs.clone();
            assert!(project_input_refs(&refs).is_err());
            assert_eq!(refs, unchanged);
            assert!(project_raw_bars(&json!([{"seq": value}])).is_err());
        }
        assert!(project_input_refs(&json!([{"raw_refs": []}])).is_err());
        assert_eq!(
            num_to_str(&json!(i64::MAX)).unwrap(),
            json!(i64::MAX.to_string())
        );
    }

    #[test]
    fn canonical_integer_rejects_non_canonical() {
        assert!(is_canonical_integer("0"));
        assert!(is_canonical_integer("10000"));
        assert!(is_canonical_integer("-10000"));
        assert!(!is_canonical_integer("+0010000"));
        assert!(!is_canonical_integer(" 10000 "));
        assert!(!is_canonical_integer("01000"));
        assert!(!is_canonical_integer("-0"));
        assert!(!is_canonical_integer(""));
        assert!(!is_canonical_integer("1.5"));
        assert!(parse_canonical_i64("9007199254740993", "x").is_ok());
        assert!(parse_canonical_i64("+0010000", "x").is_err());
    }

    #[test]
    fn wire_projection_stringifies_coordinates_and_seq() {
        let refs = json!([
            {"merged_index": 0, "raw_refs": [
                {"identity_key": "k|1|i|e0", "seq": 9007199254740993_i64, "source_coord": "0"}
            ]}
        ]);
        let projected = project_input_refs(&refs).unwrap();
        assert_eq!(projected[0]["merged_index"], json!("0"));
        assert_eq!(
            projected[0]["raw_refs"][0]["seq"],
            json!("9007199254740993")
        );
        assert_eq!(projected[0]["raw_refs"][0]["source_coord"], json!("0"));

        let bars = json!([{"seq": 9007199254740993_i64, "price": "10000"}]);
        let pbars = project_raw_bars(&bars).unwrap();
        assert_eq!(pbars[0]["seq"], json!("9007199254740993"));
        assert_eq!(pbars[0]["price"], json!("10000"));

        assert_eq!(projected[0]["raw_refs"][0]["source_coord"], json!("0"));
    }

    // ── #1371 TB-01-B 新测试 ──

    #[test]
    fn source_coord_invalid_input_rolls_back_whole_accept_batch() {
        for bad in [
            "",
            " ",
            "+1",
            "01",
            "-0",
            "1.5",
            "bad",
            "-1",
            "9223372036854775808",
        ] {
            let files = TestFiles::new();
            init_test_db(&files);
            let conn = open_db(&files.db()).unwrap();
            let before = db_meta(&conn).unwrap();
            let mut invalid = ev("bad", "1", bad, "11000");
            invalid["timestamp"] = json!("1");
            let input = write_input(
                &files,
                "bad-coordinate.json",
                &test_input(&[ev("ok", "1", "0", "10000"), invalid]),
            );
            let error = cmd_accept(
                &files.db(),
                &input,
                &fixture("profiles/testonly_tick_1_1_ohlc.json"),
                DEFAULT_WRITER_EPOCH,
            )
            .unwrap_err();
            assert!(error.contains("InvalidDomain"), "{bad}: {error}");
            assert!(read_raw_events(&conn).unwrap().is_empty());
            assert_eq!(db_meta(&conn).unwrap(), before, "{bad}");
        }
    }

    #[test]
    fn source_coord_collision_rejects_same_batch_and_other_business_identities() {
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let files = TestFiles::new();
        init_test_db(&files);
        let conn = open_db(&files.db()).unwrap();
        let before = db_meta(&conn).unwrap();
        let input = write_input(
            &files,
            "same-batch.json",
            &test_input(&[ev("e0", "1", "0", "10000"), ev("e1", "1", "0", "11000")]),
        );
        assert!(
            cmd_accept(&files.db(), &input, &profile, DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("IdentityConflict")
        );
        assert!(read_raw_events(&conn).unwrap().is_empty());
        assert_eq!(db_meta(&conn).unwrap(), before);

        accept(
            &files,
            &test_input(&[ev("e0", "1", "0", "10000")]),
            "seed.json",
            DEFAULT_WRITER_EPOCH,
        );
        let before_raw = read_raw_events(&conn).unwrap();
        let before = db_meta(&conn).unwrap();
        for changed in ["event_id", "source_namespace", "source_epoch", "instrument"] {
            let mut input_value = test_input(&[
                ev("new-free-position", "1", "2", "12000"),
                ev("e0", "1", "0", "11000"),
            ]);
            if changed == "event_id" {
                input_value["events"][1]["event_id"] = json!("another-event");
            } else {
                input_value[changed] = json!("other");
            }
            let input = write_input(&files, "collision.json", &input_value);
            assert!(
                cmd_accept(&files.db(), &input, &profile, DEFAULT_WRITER_EPOCH)
                    .unwrap_err()
                    .contains("IdentityConflict"),
                "{changed}"
            );
            assert_eq!(read_raw_events(&conn).unwrap(), before_raw, "{changed}");
            assert_eq!(db_meta(&conn).unwrap(), before, "{changed}");
        }
    }

    #[test]
    fn source_coord_revision_and_replay_keep_one_owner() {
        let files = TestFiles::new();
        init_test_db(&files);
        let original = test_input(&[
            ev("e0", "1", "0", "10000"),
            ev("e1", "1", "1", "11000"),
            ev("e2", "1", "2", "10500"),
        ]);
        accept(&files, &original, "seed.json", DEFAULT_WRITER_EPOCH);
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let correction = test_input(&[ev("e2", "2", "2", "11500")]);
        accept(&files, &correction, "correction.json", DEFAULT_WRITER_EPOCH);
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let before = read_raw_events(&conn).unwrap();
        assert_eq!(before.len(), 4);
        accept(&files, &correction, "replay.json", DEFAULT_WRITER_EPOCH);
        accept(&files, &original, "old-replay.json", DEFAULT_WRITER_EPOCH);
        assert_eq!(read_raw_events(&conn).unwrap(), before);
        let snapshot = snapshot_of(&files, None);
        assert_eq!(snapshot["objects"][0]["branch"], json!("RISING"));
        assert_eq!(
            snapshot["objects"][0]["source_coords"],
            json!(["0", "1", "2"])
        );
        for input_ref in snapshot["objects"][0]["input_refs"].as_array().unwrap() {
            assert_eq!(input_ref["raw_refs"].as_array().unwrap().len(), 1);
        }
        let moved = write_input(
            &files,
            "moved.json",
            &test_input(&[ev("e2", "3", "3", "11800")]),
        );
        assert!(cmd_accept(
            &files.db(),
            &moved,
            &fixture("profiles/testonly_tick_1_1_ohlc.json"),
            DEFAULT_WRITER_EPOCH
        )
        .unwrap_err()
        .contains("IdentityConflict"));
        assert_eq!(read_raw_events(&conn).unwrap(), before);
    }

    #[test]
    fn source_coord_exact_large_and_sparse_positions_remain_valid() {
        for valid in ["0", "7", "9007199254740993", "9223372036854775807"] {
            let expected = valid.parse::<i64>().unwrap();
            if usize::try_from(expected).is_ok() {
                assert_eq!(parse_source_coord(valid).unwrap(), expected);
            } else {
                assert!(parse_source_coord(valid).is_err());
            }
        }
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "7", "10000"),
                ev("e1", "1", "15", "11000"),
                ev("e2", "1", "99", "10500"),
            ]),
            "sparse.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        assert_eq!(
            snapshot_of(&files, None)["objects"][0]["source_coords"],
            json!(["7", "15", "99"])
        );
    }

    #[test]
    fn source_coord_pending_corruption_blocks_all_root_operations() {
        for published in [false, true] {
            for bad in ["bad", "0"] {
                let files = TestFiles::new();
                init_test_db(&files);
                accept(
                    &files,
                    &test_input(&[ev("e0", "1", "0", "10000"), ev("e1", "1", "1", "11000")]),
                    "seed.json",
                    DEFAULT_WRITER_EPOCH,
                );
                if published {
                    cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
                }
                accept(
                    &files,
                    &test_input(&[ev("e2", "1", "2", "10500")]),
                    "pending.json",
                    DEFAULT_WRITER_EPOCH,
                );
                let conn = open_db(&files.db()).unwrap();
                conn.execute(
                    "UPDATE raw_events SET source_coord=?1 WHERE event_id='e2'",
                    params![bad],
                )
                .unwrap();
                let before = db_meta(&conn).unwrap();
                let extra = write_input(
                    &files,
                    "extra.json",
                    &test_input(&[ev("e3", "1", "3", "10800")]),
                );
                for result in [
                    cmd_snapshot(&files.db(), None),
                    cmd_snapshot(&files.db(), Some(0)),
                    cmd_accept(
                        &files.db(),
                        &extra,
                        &fixture("profiles/testonly_tick_1_1_ohlc.json"),
                        DEFAULT_WRITER_EPOCH,
                    ),
                    cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH),
                    cmd_recover(&files.db(), "2"),
                ] {
                    assert!(
                        result.unwrap_err().contains("StorageUnavailable"),
                        "{published}/{bad}"
                    );
                    assert_eq!(db_meta(&conn).unwrap(), before);
                    assert_eq!(
                        conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r
                            .get::<_, i64>(0))
                            .unwrap(),
                        3
                    );
                    assert_eq!(
                        conn.query_row("SELECT COUNT(*) FROM writer_epoch_history", [], |r| r
                            .get::<_, i64>(0))
                            .unwrap(),
                        0
                    );
                }
            }
        }
    }

    #[test]
    fn source_coord_stored_corruption_rejects_before_new_begin() {
        for published in [false, true] {
            for bad in ["bad", "-1", "01", "9223372036854775808", "1"] {
                let files = TestFiles::new();
                init_test_db(&files);
                accept(
                    &files,
                    &test_input(&[
                        ev("e0", "1", "0", "10000"),
                        ev("e1", "1", "1", "11000"),
                        ev("e2", "1", "2", "10500"),
                    ]),
                    "seed.json",
                    DEFAULT_WRITER_EPOCH,
                );
                if published {
                    cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
                }
                let conn = open_db(&files.db()).unwrap();
                conn.execute(
                    "UPDATE raw_events SET source_coord=?1 WHERE event_id='e0'",
                    params![bad],
                )
                .unwrap();
                let before = db_meta(&conn).unwrap();
                let before_batches: i64 = conn
                    .query_row("SELECT COUNT(*) FROM batches", [], |r| r.get(0))
                    .unwrap();
                let error = cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap_err();
                assert!(
                    error.contains("StorageUnavailable"),
                    "{published}/{bad}: {error}"
                );
                assert_eq!(db_meta(&conn).unwrap(), before);
                let after_batches: i64 = conn
                    .query_row("SELECT COUNT(*) FROM batches", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(before_batches, after_batches);
            }
        }
    }

    /// 核心轨迹：e0/e1/e2=10000/11000/10500 先 TOP；e2 新 revision=11500 撤 TOP 成 RISING；追加 e3=10800
    /// 成新 TOP。旧 TOP 保留原身份/first_known/发生区间/撤回理由；as_of 回看 AsKnown 不提前出现。
    #[test]
    fn correction_revision_chain_withdraws_top_then_rising_then_new_top() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let s1 = snapshot_of(&files, None);
        assert_eq!(s1["generation"], json!("1"));
        assert_eq!(s1["history_mode"], json!("RecomputedWithRevision"));
        let objs1 = s1["objects"].as_array().unwrap();
        assert_eq!(objs1.len(), 1);
        assert_eq!(objs1[0]["branch"], json!("TOP"));
        assert_eq!(objs1[0]["source_coords"], json!(["0", "1", "2"]));
        assert_eq!(objs1[0]["first_known_generation"], json!("1"));
        assert_eq!(objs1[0]["lifecycle"], json!("active"));
        let old_top_id = objs1[0]["object_id"].as_str().unwrap().to_string();

        // 更正：e2 新 revision=2 → 11500（合法修订链，supersedes revision 1）。
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "in2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let s2 = snapshot_of(&files, None);
        assert_eq!(s2["generation"], json!("2"));
        let active2 = s2["objects"].as_array().unwrap();
        assert_eq!(active2.len(), 1);
        assert_eq!(active2[0]["branch"], json!("RISING"));
        let withdrawn2 = s2["withdrawn_objects"].as_array().unwrap();
        assert_eq!(withdrawn2.len(), 1);
        assert_eq!(withdrawn2[0]["object_id"].as_str().unwrap(), old_top_id);
        assert_eq!(withdrawn2[0]["branch"], json!("TOP"));
        assert_eq!(withdrawn2[0]["lifecycle"], json!("withdrawn"));
        assert_eq!(
            withdrawn2[0]["withdrawal_reason"],
            json!("superseded_by_revision")
        );
        assert_eq!(
            withdrawn2[0]["superseded_by"].as_str().unwrap(),
            active2[0]["object_id"].as_str().unwrap()
        );
        // 旧 TOP 原身份/first_known/发生区间不丢。
        assert_eq!(withdrawn2[0]["first_known_generation"], json!("1"));
        assert_eq!(withdrawn2[0]["source_coords"], json!(["0", "1", "2"]));

        // 追加 e3=10800 成新 TOP（[1,2,3]）。
        accept(
            &files,
            &test_input(&[ev("e3", "1", "3", "10800")]),
            "in3.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let s3 = snapshot_of(&files, None);
        assert_eq!(s3["generation"], json!("3"));
        let active3 = s3["objects"].as_array().unwrap();
        assert_eq!(active3.len(), 2);
        let branches3: Vec<&str> = active3
            .iter()
            .map(|o| o["branch"].as_str().unwrap())
            .collect();
        assert_eq!(branches3, vec!["RISING", "TOP"]);
        let top3 = active3.iter().find(|o| o["branch"] == "TOP").unwrap();
        assert_eq!(top3["source_coords"], json!(["1", "2", "3"]));
        assert_eq!(top3["first_known_generation"], json!("3"));
        let w3 = s3["withdrawn_objects"].as_array().unwrap();
        assert_eq!(w3.len(), 1);

        // AsKnown 回看 generation 1：只有旧 TOP，不提前出现 RISING / 新 TOP。
        let a1 = snapshot_of(&files, Some(1));
        assert_eq!(a1["history_mode"], json!("AsKnown"));
        assert_eq!(a1["as_of_generation"], json!("1"));
        let a1o = a1["objects"].as_array().unwrap();
        assert_eq!(a1o.len(), 1);
        assert_eq!(a1o[0]["branch"], json!("TOP"));
        assert_eq!(a1o[0]["object_id"].as_str().unwrap(), old_top_id);

        // 源修订链：raw_history 含 e2 两个 revision 与 supersedes 关系。
        let rh = s3["raw_history"].as_array().unwrap();
        let e2_rows: Vec<&Value> = rh.iter().filter(|r| r["event_id"] == "e2").collect();
        assert_eq!(e2_rows.len(), 2);
        let rev2 = e2_rows.iter().find(|r| r["revision"] == "2").unwrap();
        assert_eq!(rev2["supersedes_revision"], json!("1"));
        // 有效源位置仍 4 个（不把晚到新版当第五 tick）——对象窗口源坐标覆盖 0..3。
        let rels = s3["relations"].as_array().unwrap();
        assert!(rels.iter().any(|r| r["relation_type"] == "replaces"));
        assert!(rels.iter().any(|r| r["relation_type"] == "supersedes"));
    }

    #[test]
    #[cfg(feature = "s_capture_native_testonly")]
    fn native_batch_header_shares_original_text_permits_until_last_drop() {
        let budget = v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let header = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                BatchControlHeader::from_verified_batch_admitted(
                    &json!({"protocol_revision":"s-session/2", "session_generation":"1",
                    "clock_plan_hash":"clock", "semantic_commit_ns":"20"}),
                    Some(work),
                )
            })
            .unwrap();
        assert!(budget.live() > baseline);
        let shared = header.clone();
        drop(header);
        assert_eq!(
            shared.semantic_commit_ns.as_deref().map(|v| v.as_str()),
            Some("20")
        );
        assert!(budget.live() > baseline);
        drop(shared);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    fn assert_native_raw_matches_plain(conn: &Connection, expected_len: usize, ohlc: bool) {
        let expected = read_raw_events_until(conn, None).unwrap();
        assert_eq!(expected.len(), expected_len);
        assert!(expected
            .iter()
            .any(|row| row["supersedes_revision"].as_i64() == Some(1)));
        let budget = v3::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let retained = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let actual = read_raw_events_until_admitted(conn, None, Some(work))?;
                let parsed = match &actual {
                    v3::business_json::JsonOwner::Plain(value) => value.as_ref(),
                    #[cfg(feature = "s_allocation_json_testonly")]
                    _ => unreachable!("Native raw保留PlainJson"),
                };
                assert_eq!(
                    parsed,
                    &Value::Array(expected.clone()),
                    "完整SQL JSON逐字段对照"
                );
                for (seq, row) in parsed.as_array().unwrap().iter().enumerate() {
                    v3::business_json::validate_raw_row_admitted(
                        row.into(),
                        seq,
                        row["supersedes_revision"].as_i64(),
                        Some(work),
                    )?;
                    assert_eq!(row["payload_hash"].as_str().unwrap().len(), 64);
                    assert_eq!(row["receipt_id"].as_str().unwrap().len(), 21);
                    assert_eq!(row.get("schema_revision").is_some(), ohlc);
                    if ohlc {
                        assert_eq!(row["schema_revision"], tb02a::RAW_SCHEMA);
                        assert_eq!(row["price"], row["close"]);
                        for field in ["open", "high", "low", "close"] {
                            assert!(row[field].is_string());
                        }
                    }
                }
                Ok::<_, String>(actual)
            })
            .unwrap();
        assert!(budget.live() > 0);
        let held = budget.live();
        drop(retained);
        assert!(budget.live() < held, "raw父拥有者析构归还原载荷许可");
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[test]
    #[cfg(feature = "s_capture_native_testonly")]
    fn native_cold_raw_sql_json_hash_receipt_revision_matches_plain() {
        let files = TestFiles::new();
        init_test_db(&files);
        let mut first = test_input(&[ev("e0", "1", "0", "10000"), ev("e1", "1", "1", "10500")]);
        first["events"][0]["raw_text"] = json!("汉😀\n\"原始账\"");
        accept(
            &files,
            &first,
            "native-raw-first.json",
            DEFAULT_WRITER_EPOCH,
        );
        accept(
            &files,
            &test_input(&[ev("e1", "2", "1", "11500")]),
            "native-raw-revision.json",
            DEFAULT_WRITER_EPOCH,
        );
        let conn = open_db(&files.db()).unwrap();
        assert_native_raw_matches_plain(&conn, 3, false);
    }

    #[test]
    #[cfg(feature = "s_capture_native_testonly")]
    fn native_raw_column_errors_precede_ohlc_cardinality_and_earlier_row_faults() {
        // 两个真正双故障；用BLOB并核typeof，避免TEXT affinity把INTEGER自动转成文本。
        for cardinality_fault in [true, false] {
            let files = TestFiles::new();
            init_test_db(&files);
            accept_ohlc(&files, &ohlc_input(&[(0, 4), (2, 5), (1, 6)], false));
            let conn = open_db(&files.db()).unwrap();
            let last: i64 = conn
                .query_row("SELECT MAX(seq) FROM raw_events", [], |r| r.get(0))
                .unwrap();
            let original: String = conn
                .query_row(
                    "SELECT raw_text FROM raw_events WHERE seq=?1",
                    [last],
                    |r| r.get(0),
                )
                .unwrap();
            conn.execute(
                "UPDATE raw_events SET raw_text=x'626164' WHERE seq=?1",
                [last],
            )
            .unwrap();
            let stored_type: String = conn
                .query_row(
                    "SELECT typeof(raw_text) FROM raw_events WHERE seq=?1",
                    [last],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(stored_type, "blob", "损坏fixture必须真实违反TEXT列类型");
            if cardinality_fault {
                conn.execute(
                    "DELETE FROM raw_ohlc WHERE rowid=(SELECT MIN(rowid) FROM raw_ohlc)",
                    [],
                )
                .unwrap();
            } else {
                conn.execute("UPDATE raw_ohlc SET close='999999' WHERE (identity_key,revision)=(SELECT identity_key,revision FROM raw_events ORDER BY seq LIMIT 1)", []).unwrap();
            }
            let native_error = || {
                let budget = v3::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
                match budget
                    .codec_work()
                    .unwrap()
                    .run(|work| read_raw_events_until_admitted(&conn, None, Some(work)))
                    .err()
                    .expect("双故障必须拒绝")
                {
                    newchan_rust::s_content::capture_allocation::scratch::WorkError::Operation(
                        message,
                    ) => message,
                    newchan_rust::s_content::capture_allocation::scratch::WorkError::Resource(
                        error,
                    ) => {
                        panic!("首错顺序测试意外遭遇资源失败：{error:?}")
                    }
                }
            };
            let plain = read_raw_events_until(&conn, None).unwrap_err();
            let native = native_error();
            assert!(
                plain.starts_with("collect 失败："),
                "旧入口先拒绝全部raw列类型：{plain}"
            );
            assert!(
                native.starts_with("collect 失败："),
                "Native不得提前返回OHLC故障：{native}"
            );
            assert!(!native.contains("OHLC"));
            // 修复较晚raw列后，原本被遮住的OHLC错误才成为首错。
            conn.execute(
                "UPDATE raw_events SET raw_text=?1 WHERE seq=?2",
                params![original, last],
            )
            .unwrap();
            let expected = if cardinality_fault {
                "StorageUnavailable：raw_ohlc与原始账不一一对应"
            } else {
                "StorageUnavailable：OHLC schema/close索引矛盾"
            };
            assert_eq!(read_raw_events_until(&conn, None).unwrap_err(), expected);
            assert_eq!(native_error(), expected);
        }
    }

    #[test]
    #[cfg(feature = "s_capture_native_testonly")]
    fn native_cold_raw_ohlc_and_revision_matches_plain() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept_ohlc(&files, &ohlc_input(&[(0, 4), (2, 5), (1, 6)], false));
        accept_ohlc(&files, &ohlc_input(&[(0, 4), (2, 5), (1, 7)], true));
        let conn = open_db(&files.db()).unwrap();
        assert_native_raw_matches_plain(&conn, 4, true);
    }

    #[test]
    fn revision_replay_conflict_and_old_version_no_revive() {
        let files = TestFiles::new();
        init_test_db(&files);
        let p = write_input(
            &files,
            "a.json",
            &test_input(&[ev("e2", "1", "2", "10500")]),
        );
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let out1 = cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        // 同 revision 同内容 → replay（原收据）。
        let out2 = cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        // 同 revision 异内容 → IdentityConflict。
        let c = write_input(&files, "c.json", &test_input(&[ev("e2", "1", "2", "9999")]));
        let out3 = cmd_accept(&files.db(), &c, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        // 新 revision → accepted + supersedes。
        let r2 = write_input(
            &files,
            "r2.json",
            &test_input(&[ev("e2", "2", "2", "11500")]),
        );
        let out4 = cmd_accept(&files.db(), &r2, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        // 旧版重放 → replay（不复活、不改有效值）。
        let out5 = cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH).unwrap();

        let _ = (out1, out2, out3, out4, out5);

        let conn = open_db(&files.db()).unwrap();
        let all = read_raw_events(&conn).unwrap();
        assert_eq!(all.len(), 2);
        let effective = read_effective_events(&all).unwrap();
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0]["price"], json!("11500"));
    }

    #[test]
    fn out_of_order_late_revision_rejected() {
        let files = TestFiles::new();
        init_test_db(&files);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let r2 = write_input(
            &files,
            "r2.json",
            &test_input(&[ev("e2", "2", "2", "11500")]),
        );
        cmd_accept(&files.db(), &r2, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        // 更低 revision 晚到（已见 rev 2）→ InvalidDomain，不新接纳。
        let r1 = write_input(
            &files,
            "r1.json",
            &test_input(&[ev("e2", "1", "2", "10500")]),
        );
        cmd_accept(&files.db(), &r1, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let all = read_raw_events(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0]["revision"], json!(2));
    }

    #[test]
    fn writer_epoch_invalid_persistence_rejects_all_writes() {
        for epoch in ["foo", "01", "-1", "", "9223372036854775808"] {
            let files = TestFiles::new();
            init_test_db(&files);
            let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
            let input = write_input(
                &files,
                "epoch.json",
                &test_input(&[ev("e0", "1", "0", "10000")]),
            );
            let conn = open_db(&files.db()).unwrap();
            meta_set(&conn, "writer_epoch", epoch).unwrap();
            let before = db_meta(&conn).unwrap();
            for result in [
                cmd_accept(&files.db(), &input, &profile, epoch),
                cmd_advance(&files.db(), epoch),
                cmd_recover(&files.db(), "2"),
            ] {
                assert!(
                    result.unwrap_err().contains("StorageUnavailable"),
                    "epoch={epoch:?}"
                );
                assert_eq!(db_meta(&conn).unwrap(), before);
                assert!(read_raw_events(&conn).unwrap().is_empty());
                let history: i64 = conn
                    .query_row("SELECT COUNT(*) FROM writer_epoch_history", [], |r| {
                        r.get(0)
                    })
                    .unwrap();
                assert_eq!(history, 0);
            }
        }
    }

    #[test]
    fn writer_epoch_invalid_configuration_does_not_change_state() {
        let files = TestFiles::new();
        init_test_db(&files);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let input = write_input(
            &files,
            "epoch.json",
            &test_input(&[ev("e0", "1", "0", "10000")]),
        );
        let conn = open_db(&files.db()).unwrap();
        let before = db_meta(&conn).unwrap();
        for epoch in ["foo", "01", "-1", "", "9223372036854775808"] {
            for result in [
                cmd_accept(&files.db(), &input, &profile, epoch),
                cmd_advance(&files.db(), epoch),
                cmd_recover(&files.db(), epoch),
            ] {
                assert!(result.unwrap_err().contains("InvalidDomain"));
                assert_eq!(db_meta(&conn).unwrap(), before);
                assert!(read_raw_events(&conn).unwrap().is_empty());
            }
        }
    }

    #[test]
    fn writer_epoch_zero_and_i64_max_are_exact_valid_epochs() {
        let files = TestFiles::new();
        init_test_db(&files);
        let conn = open_db(&files.db()).unwrap();
        meta_set(&conn, "writer_epoch", "0").unwrap();
        verify_writer_epoch(&conn, "0").unwrap();
        cmd_recover(&files.db(), "9223372036854775807").unwrap();
        verify_writer_epoch(&conn, "9223372036854775807").unwrap();
        assert!(verify_writer_epoch(&conn, "0")
            .unwrap_err()
            .contains("StaleWriter"));
        assert!(cmd_recover(&files.db(), "9223372036854775807")
            .unwrap_err()
            .contains("StaleWriter"));
    }

    #[test]
    fn writer_epoch_transition_rejects_old_writer_and_recovers_pending_begin() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();

        // 正式换代到 epoch 2（recover：advance_state 为 idle 时仅提升 epoch）。
        cmd_recover(&files.db(), "2").unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "writer_epoch").unwrap().as_deref(),
            Some("2")
        );

        // 旧 epoch 进程重放 accept / advance → StaleWriter（零写入）。
        let extra = test_input(&[ev("e3", "1", "3", "10800")]);
        let ep = write_input(&files, "e3.json", &extra);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        assert!(cmd_accept(&files.db(), &ep, &profile, "1")
            .unwrap_err()
            .contains("StaleWriter"));
        assert!(cmd_advance(&files.db(), "1")
            .unwrap_err()
            .contains("StaleWriter"));
        // 新 epoch 进程成功。
        cmd_accept(&files.db(), &ep, &profile, "2").unwrap();
        cmd_advance(&files.db(), "2").unwrap();
    }

    #[test]
    fn watch_returns_ordered_delta_and_gap_detection() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "in2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();

        let conn = open_db(&files.db()).unwrap();
        // 从 0 起读全部 delta。
        let generations: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT generation FROM structure_deltas WHERE generation > 0 ORDER BY generation ASC")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let rows: Vec<String> = generations
            .iter()
            .map(|g| delta_text_of(&conn, *g))
            .collect();
        assert_eq!(rows.len(), 2);
        let d2: Value = serde_json::from_str(&rows[1]).unwrap();
        assert_eq!(d2["generation"], json!("2"));
        assert_eq!(d2["base_cut"], json!("cut-1"));
        assert_eq!(d2["next_cut"], json!("cut-2"));
        assert_eq!(d2["withdrawals"].as_array().unwrap().len(), 1);
        assert_eq!(d2["replaces"].as_array().unwrap().len(), 1);
        assert_eq!(d2["upserts"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn financial_basis_pending_distinguishes_commit_history_and_unpublished_revision() {
        let files = TestFiles::new();
        init_test_db(&files);
        let conn = open_db(&files.db()).unwrap();
        assert!(!financial_basis_pending(&conn).unwrap());
        drop(conn);
        accept(
            &files,
            &test_input(&[ev("e0", "1", "0", "10000")]),
            "first.json",
            DEFAULT_WRITER_EPOCH,
        );
        assert!(financial_basis_pending(&open_db(&files.db()).unwrap()).unwrap());
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert!(meta_get_opt(&conn, "begin_token")
            .unwrap()
            .is_some_and(|t| !t.is_empty()));
        assert!(
            !financial_basis_pending(&conn).unwrap(),
            "历史 Begin 标记不是未完成修订"
        );
        drop(conn);
        accept(
            &files,
            &test_input(&[ev("e0", "2", "0", "10001")]),
            "revision.json",
            DEFAULT_WRITER_EPOCH,
        );
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert!(
            financial_basis_pending(&conn).unwrap(),
            "修订已接纳但未 Begin/发布也必须等待"
        );
        drop(conn);
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        assert!(!financial_basis_pending(&open_db(&files.db()).unwrap()).unwrap());
    }

    #[test]
    fn financial_basis_pending_detects_active_begin_without_new_raw_input() {
        let files = TestFiles::new();
        init_test_db(&files);
        let mut conn = open_db(&files.db()).unwrap();
        let tx = conn.transaction().unwrap();
        let token = v2::record_begin(&tx, None, 1, -1, "cut-0", DEFAULT_WRITER_EPOCH).unwrap();
        meta_set(&tx, "advance_state", &format!("begun:{token}:1:-1")).unwrap();
        meta_set(&tx, "begin_token", &token).unwrap();
        meta_set(&tx, "begin_generation", "1").unwrap();
        meta_set(&tx, "begin_input_frontier", "-1").unwrap();
        tx.commit().unwrap();
        assert!(financial_basis_pending(&conn).unwrap());
        drop(conn);
        cmd_recover(&files.db(), "2").unwrap();
        assert!(!financial_basis_pending(&open_db(&files.db()).unwrap()).unwrap());
    }

    #[test]
    fn recover_clears_pending_begin_and_keeps_reachable_root() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        // 模拟真实 SIGKILL 后的未决 Begin（与 cmd_advance Begin 提交后状态一致）。
        let conn = open_db(&files.db()).unwrap();
        meta_set(&conn, "advance_state", "begun:dead:1:2").unwrap();
        meta_set(&conn, "begin_token", "dead").unwrap();
        assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StaleWriter"));

        cmd_recover(&files.db(), "2").unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert_eq!(
            meta_get_opt(&conn, "generation").unwrap().as_deref(),
            Some("0")
        );
        assert_eq!(
            meta_get_opt(&conn, "structure_cut").unwrap().as_deref(),
            Some("cut-0")
        );
        // 恢复后新 writer 合法推进。
        cmd_advance(&files.db(), "2").unwrap();
        let s = snapshot_of(&files, None);
        assert_eq!(s["generation"], json!("1"));
        assert_eq!(s["objects"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn as_known_cut_filters_all_fields_and_no_premature_top() {
        let files = TestFiles::new();
        init_test_db(&files);
        // 缺右邻：仅两点 → 无 TOP，只有知识不足观察。
        accept(
            &files,
            &test_input(&[ev("e0", "1", "0", "10000"), ev("e1", "1", "1", "11000")]),
            "p2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let s1 = snapshot_of(&files, None);
        assert_eq!(s1["generation"], json!("1"));
        assert!(s1["objects"].as_array().unwrap().is_empty());
        assert_eq!(
            s1["observations"][0]["reason"],
            json!("fewer_than_three_bars")
        );
        assert_eq!(s1["raw_history"].as_array().unwrap().len(), 2);
        assert!(s1["witnesses"].as_array().unwrap().is_empty());
        assert!(s1["relations"].as_array().unwrap().is_empty());

        // 形成 TOP，再更正撤回，再追加新 TOP。
        accept(
            &files,
            &test_input(&[ev("e2", "1", "2", "10500")]),
            "e2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "e2c.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e3", "1", "3", "10800")]),
            "e3.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();

        // AsKnown gen2（TOP 刚形成）：raw_history 只 3 条，无 replaces/supersedes，对象 active 且无撤回元数据。
        let a2 = snapshot_of(&files, Some(2));
        assert_eq!(a2["history_mode"], json!("AsKnown"));
        assert_eq!(a2["raw_history"].as_array().unwrap().len(), 3);
        let a2o = a2["objects"].as_array().unwrap();
        assert_eq!(a2o.len(), 1);
        assert_eq!(a2o[0]["branch"], json!("TOP"));
        assert_eq!(a2o[0]["lifecycle"], json!("active"));
        assert_eq!(a2o[0]["withdrawn_generation"], Value::Null);
        assert!(a2["relations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["relation_type"] != "replaces" && r["relation_type"] != "supersedes"));
        assert!(a2["withdrawn_objects"].as_array().unwrap().is_empty());

        // AsKnown gen3（更正后）：撤旧 + RISING；raw_history 4 条，含 replaces + supersedes。
        let a3 = snapshot_of(&files, Some(3));
        assert_eq!(a3["raw_history"].as_array().unwrap().len(), 4);
        assert_eq!(a3["objects"][0]["branch"], json!("RISING"));
        assert_eq!(a3["withdrawn_objects"][0]["branch"], json!("TOP"));
        let rel_types: Vec<&str> = a3["relations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["relation_type"].as_str().unwrap())
            .collect();
        assert!(rel_types.contains(&"replaces"));
        assert!(rel_types.contains(&"supersedes"));

        // 当前 cut（gen4）：两点缺右邻时的 insufficient_knowledge 观察仍在历史。
        let cur = snapshot_of(&files, None);
        assert_eq!(cur["generation"], json!("4"));
        assert_eq!(cur["raw_history"].as_array().unwrap().len(), 5);
        assert!(cur["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["reason"] == "fewer_than_three_bars"));
    }

    #[test]
    fn recover_requires_strictly_increasing_epoch() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        cmd_recover(&files.db(), "2").unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "writer_epoch").unwrap().as_deref(),
            Some("2")
        );
        // 同 epoch / 回退 epoch 均拒绝，不重写旧 epoch。
        assert!(cmd_recover(&files.db(), "2")
            .unwrap_err()
            .contains("StaleWriter"));
        assert!(cmd_recover(&files.db(), "1")
            .unwrap_err()
            .contains("StaleWriter"));
        assert_eq!(
            meta_get_opt(&conn, "writer_epoch").unwrap().as_deref(),
            Some("2")
        );
        // 旧 epoch 进程写入拒绝，新 epoch 成功。
        let e3 = test_input(&[ev("e3", "1", "3", "10800")]);
        let p = write_input(&files, "e3.json", &e3);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        assert!(cmd_accept(&files.db(), &p, &profile, "1")
            .unwrap_err()
            .contains("StaleWriter"));
        cmd_accept(&files.db(), &p, &profile, "2").unwrap();
        cmd_advance(&files.db(), "2").unwrap();
        let s = snapshot_of(&files, None);
        assert_eq!(s["generation"], json!("2"));
    }

    #[test]
    fn advance_without_new_input_is_idempotent() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "generation").unwrap().as_deref(),
            Some("1")
        );
        // 无新输入重试（after_commit 丢回执同 argv 重试）→ 不产新 cut。
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "generation").unwrap().as_deref(),
            Some("1")
        );
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
    }

    #[test]
    fn as_of_unpublished_cut_is_rejected() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        let err = read_snapshot_in_tx(&tx, Some(5)).unwrap_err();
        assert!(err.contains("尚未发布"), "{}", err);
        drop(tx);
    }

    #[test]
    fn current_snapshot_excludes_unpublished_accepted_revision() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        // 接纳 e2 rev2 但不 advance：当前 cut 的 raw_history 不得混入未发布修订。
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "in2.json",
            DEFAULT_WRITER_EPOCH,
        );
        let s = snapshot_of(&files, None);
        assert_eq!(s["generation"], json!("1"));
        assert_eq!(s["raw_history"].as_array().unwrap().len(), 3);
        assert!(s["raw_history"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["revision"] == "1"));
    }

    #[test]
    fn corrupt_reachable_root_blocks_writes() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        {
            let conn = open_db(&files.db()).unwrap();
            let idx = meta_get_opt(&conn, "index_frontier").unwrap().unwrap();
            conn.execute("DELETE FROM batches WHERE batch_id=?1", params![idx])
                .unwrap();
        }
        let extra = test_input(&[ev("e3", "1", "3", "10800")]);
        let p = write_input(&files, "e3.json", &extra);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        assert!(cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StorageUnavailable"));
        assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StorageUnavailable"));
    }

    #[test]
    fn corrupt_root_variants_block_all_writes() {
        // gen1 TOP + gen2 更正（RISING 撤旧 TOP）→ 三个已发布 cut 的库。
        let build = |files: &TestFiles| {
            init_test_db(files);
            accept(
                files,
                &test_input(&[
                    ev("e0", "1", "0", "10000"),
                    ev("e1", "1", "1", "11000"),
                    ev("e2", "1", "2", "10500"),
                ]),
                "in1.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            accept(
                files,
                &test_input(&[ev("e2", "2", "2", "11500")]),
                "in2.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        };
        let extra = test_input(&[ev("e3", "1", "3", "10800")]);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let assert_reject = |files: &TestFiles| {
            let p = write_input(files, "e3.json", &extra);
            assert!(cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_recover(&files.db(), "2")
                .unwrap_err()
                .contains("StorageUnavailable"));
        };

        // 1) 当前根 batch 坏 bytes。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            let idx = meta_get_opt(&conn, "index_frontier").unwrap().unwrap();
            conn.execute(
                "UPDATE batches SET canonical_bytes=x'7b7d' WHERE batch_id=?1",
                params![idx],
            )
            .unwrap();
            drop(conn);
            assert_reject(&files);
        }
        // 2) 历史 delta[1] 的 batch 被删（缺历史引用）。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            let idx: String = conn
                .query_row(
                    "SELECT index_frontier FROM structure_deltas WHERE generation=1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            conn.execute("DELETE FROM batches WHERE batch_id=?1", params![idx])
                .unwrap();
            drop(conn);
            assert_reject(&files);
        }
        // 3) 缺 index_frontier meta（缺根 meta）。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            conn.execute("DELETE FROM meta WHERE key='index_frontier'", [])
                .unwrap();
            drop(conn);
            assert_reject(&files);
        }
    }

    #[test]
    fn interior_delta_missing_blocks_writes_and_reads() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "in2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e3", "1", "3", "10800")]),
            "in3.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        {
            let conn = open_db(&files.db()).unwrap();
            conn.execute("DELETE FROM structure_deltas WHERE generation=2", [])
                .unwrap();
        }
        let extra = test_input(&[ev("e4", "1", "4", "10900")]);
        let p = write_input(&files, "e4.json", &extra);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        assert!(cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StorageUnavailable"));
        assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StorageUnavailable"));
        assert!(cmd_recover(&files.db(), "2")
            .unwrap_err()
            .contains("StorageUnavailable"));
        // 读入口也拒绝（正式 cmd_snapshot 路径，含可达根核）。
        assert!(cmd_snapshot(&files.db(), None)
            .unwrap_err()
            .contains("StorageUnavailable"));
    }

    #[test]
    fn bad_last_advance_frontier_blocks_reads_and_writes() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        {
            let conn = open_db(&files.db()).unwrap();
            meta_set(&conn, "last_advance_frontier", "broken").unwrap();
        }
        let extra = test_input(&[ev("e3", "1", "3", "10800")]);
        let p = write_input(&files, "e3.json", &extra);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        assert!(cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StorageUnavailable"));
        assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
            .unwrap_err()
            .contains("StorageUnavailable"));
        assert!(cmd_recover(&files.db(), "2")
            .unwrap_err()
            .contains("StorageUnavailable"));
        // 读入口也拒绝（正式 cmd_snapshot 路径）。
        assert!(cmd_snapshot(&files.db(), None)
            .unwrap_err()
            .contains("StorageUnavailable"));
    }

    #[test]
    fn future_withdrawn_object_is_rejected_as_corrupt() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "in2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        // 撤回对象 first_known=1，withdrawn=2；改为 withdrawn=0 → 非法生命周期。
        {
            let conn = open_db(&files.db()).unwrap();
            conn.execute(
                "UPDATE objects SET withdrawn_generation=0 WHERE withdrawn_generation IS NOT NULL",
                [],
            )
            .unwrap();
        }
        assert!(cmd_snapshot(&files.db(), None)
            .unwrap_err()
            .contains("StorageUnavailable"));
        assert!(cmd_snapshot(&files.db(), Some(0))
            .unwrap_err()
            .contains("StorageUnavailable"));
    }

    #[test]
    fn future_index_rows_block_current_history_and_writes() {
        for (table, id_column, withdrawn) in [
            ("objects", "object_id", false),
            ("objects", "object_id", true),
            ("witnesses", "witness_id", false),
            ("relations", "relation_id", false),
            ("observations", "observation_id", false),
        ] {
            let files = TestFiles::new();
            init_test_db(&files);
            for (n, price) in ["10000", "11000", "10500"].iter().enumerate() {
                let input = test_input(&[ev(&format!("e{n}"), "1", &n.to_string(), price)]);
                accept(&files, &input, &format!("e{n}.json"), DEFAULT_WRITER_EPOCH);
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
            accept(
                &files,
                &test_input(&[ev("e2", "2", "2", "11500")]),
                "correction.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            let conn = open_db(&files.db()).unwrap();
            // 克隆一条真实发布行，隐藏到所有已发布 cut 之外；不能只篡改已有行而由旧对拍顺带抓到。
            let columns: Vec<String> = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap()
                .query_map([], |r| r.get(1))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            let expressions: Vec<String> = columns
                .iter()
                .map(|column| {
                    if column == id_column {
                        format!("'future-' || {column}")
                    } else if column == "published_generation" || column == "first_known_generation"
                    {
                        "5".to_string()
                    } else if column == "withdrawn_generation" {
                        if withdrawn { "6" } else { "NULL" }.to_string()
                    } else {
                        column.clone()
                    }
                })
                .collect();
            assert_eq!(
                conn.execute(
                    &format!(
                        "INSERT INTO {table} SELECT {} FROM {table} LIMIT 1",
                        expressions.join(",")
                    ),
                    []
                )
                .unwrap(),
                1
            );
            let before = db_meta(&conn).unwrap();
            let before_raw = read_raw_events(&conn).unwrap();
            let extra = write_input(
                &files,
                "extra.json",
                &test_input(&[ev("e3", "1", "3", "10800")]),
            );
            for result in [
                cmd_snapshot(&files.db(), None),
                cmd_snapshot(&files.db(), Some(4)),
                cmd_accept(
                    &files.db(),
                    &extra,
                    &fixture("profiles/testonly_tick_1_1_ohlc.json"),
                    DEFAULT_WRITER_EPOCH,
                ),
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH),
                cmd_recover(&files.db(), "2"),
            ] {
                assert!(
                    result.unwrap_err().contains("StorageUnavailable"),
                    "{table}, withdrawn={withdrawn}"
                );
                assert_eq!(db_meta(&conn).unwrap(), before);
                assert_eq!(read_raw_events(&conn).unwrap(), before_raw);
            }
        }
    }

    #[test]
    fn zero_generation_rejects_published_index_but_allows_pending_input() {
        let files = TestFiles::new();
        init_test_db(&files);
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let input = write_input(
            &files,
            "pending.json",
            &test_input(&[ev("e0", "1", "0", "10000")]),
        );
        cmd_accept(&files.db(), &input, &profile, DEFAULT_WRITER_EPOCH).unwrap();
        // 接纳与发布是两步：合法待发布原始事实不被发布索引门排除。
        cmd_snapshot(&files.db(), None).unwrap();
        let conn = open_db(&files.db()).unwrap();
        conn.execute("INSERT INTO observations VALUES('zero-index','unpublished','InsufficientKnowledge',NULL,NULL,NULL,'bogus','{}',0)", []).unwrap();
        let before = db_meta(&conn).unwrap();
        for result in [
            cmd_snapshot(&files.db(), None),
            cmd_snapshot(&files.db(), Some(0)),
            cmd_accept(&files.db(), &input, &profile, DEFAULT_WRITER_EPOCH),
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH),
            cmd_recover(&files.db(), "2"),
        ] {
            assert!(result.unwrap_err().contains("StorageUnavailable"));
            assert_eq!(db_meta(&conn).unwrap(), before);
            assert_eq!(read_raw_events(&conn).unwrap().len(), 1);
        }
    }

    #[test]
    fn empty_advance_is_valid_frontier_minus_one() {
        let files = TestFiles::new();
        init_test_db(&files);
        // 无输入 advance：合法空输入域，gen1/cut-1/frontier=-1，insufficient_knowledge。
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let s = snapshot_of(&files, None);
        assert_eq!(s["generation"], json!("1"));
        assert_eq!(s["structure_cut"], json!("cut-1"));
        assert_eq!(s["objects"].as_array().unwrap().len(), 0);
        assert_eq!(
            s["observations"][0]["reason"],
            json!("fewer_than_three_bars")
        );
        assert_eq!(s["raw_history"].as_array().unwrap().len(), 0);
        // 再空 advance 幂等（frontier 未变）。
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "generation").unwrap().as_deref(),
            Some("1")
        );
    }

    #[test]
    fn wrong_root_tuple_blocks_writes_and_reads() {
        let build = |files: &TestFiles| {
            init_test_db(files);
            accept(
                files,
                &test_input(&[
                    ev("e0", "1", "0", "10000"),
                    ev("e1", "1", "1", "11000"),
                    ev("e2", "1", "2", "10500"),
                ]),
                "in1.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            accept(
                files,
                &test_input(&[ev("e2", "2", "2", "11500")]),
                "in2.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            accept(
                files,
                &test_input(&[ev("e3", "1", "3", "10800")]),
                "in3.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        };
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let extra = test_input(&[ev("e4", "1", "4", "10900")]);
        let assert_reject = |files: &TestFiles| {
            let p = write_input(files, "e4.json", &extra);
            assert!(cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_recover(&files.db(), "2")
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_snapshot(&files.db(), None)
                .unwrap_err()
                .contains("StorageUnavailable"));
        };
        // 1) meta.index_frontier 指向另一合法 batch（gen1 的）。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            let wrong: String = conn
                .query_row(
                    "SELECT index_frontier FROM structure_deltas WHERE generation=1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            meta_set(&conn, "index_frontier", &wrong).unwrap();
            drop(conn);
            assert_reject(&files);
        }
        // 2) meta.structure_cut 改 cut-99。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            meta_set(&conn, "structure_cut", "cut-99").unwrap();
            drop(conn);
            assert_reject(&files);
        }
    }

    #[test]
    fn inner_delta_payload_mismatch_blocks_writes_and_reads() {
        let build = |files: &TestFiles| {
            init_test_db(files);
            accept(
                files,
                &test_input(&[
                    ev("e0", "1", "0", "10000"),
                    ev("e1", "1", "1", "11000"),
                    ev("e2", "1", "2", "10500"),
                ]),
                "in1.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            accept(
                files,
                &test_input(&[ev("e2", "2", "2", "11500")]),
                "in2.json",
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        };
        let profile = fixture("profiles/testonly_tick_1_1_ohlc.json");
        let extra = test_input(&[ev("e3", "1", "3", "10800")]);
        let assert_reject = |files: &TestFiles| {
            let p = write_input(files, "e3.json", &extra);
            assert!(cmd_accept(&files.db(), &p, &profile, DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH)
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_recover(&files.db(), "2")
                .unwrap_err()
                .contains("StorageUnavailable"));
            assert!(cmd_snapshot(&files.db(), None)
                .unwrap_err()
                .contains("StorageUnavailable"));
        };
        // 1) 内层 seq_range.from 改 99（外层仍 2）。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            let mut broken = delta_of(&conn, 2);
            broken["seq_range"]["from"] = json!("99");
            set_delta(&conn, 2, &broken);
            drop(conn);
            assert_reject(&files);
        }
        // 2) 内层 catalog_run_status 改 not_run（外层仍 run）。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            let mut broken = delta_of(&conn, 2);
            broken["catalog_run_status"] = json!("not_run");
            set_delta(&conn, 2, &broken);
            drop(conn);
            assert_reject(&files);
        }
        // 3) 内层 catalog_evidence 改 bogus。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            let mut broken = delta_of(&conn, 2);
            broken["catalog_evidence"] = json!({"bogus": true});
            set_delta(&conn, 2, &broken);
            drop(conn);
            assert_reject(&files);
        }
    }

    #[test]
    fn large_predecessor_supersedes_and_evidence_wire_are_strings() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        // 连续两次 >2^53 修订：supersedes 引用精确往返为规范十进制字符串。
        accept(
            &files,
            &test_input(&[ev("e2", "9007199254740993", "2", "11500")]),
            "c1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "9007199254740994", "2", "12000")]),
            "c2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();

        let s = snapshot_of(&files, None);
        // 见证 raw_bars.supersedes_revision 必须是规范十进制字符串（不 Number 中转）。
        let mut saw_supersedes: Option<String> = None;
        for w in s["witnesses"].as_array().unwrap() {
            for rb in w["raw_bars"].as_array().unwrap() {
                if let Some(sup) = rb.get("supersedes_revision") {
                    if !sup.is_null() {
                        assert!(sup.is_string(), "supersedes_revision 应为字符串：{sup}");
                        saw_supersedes = Some(sup.as_str().unwrap().to_string());
                    }
                }
            }
        }
        assert_eq!(saw_supersedes.as_deref(), Some("9007199254740993"));
        // raw_history 的 supersedes_revision 同样为字符串。
        for r in s["raw_history"].as_array().unwrap() {
            if r["supersedes_revision"] != Value::Null {
                assert!(r["supersedes_revision"].is_string());
            }
        }
        // catalog evidence 计数为十进制字符串。
        let conn = open_db(&files.db()).unwrap();
        let cat = read_catalog_in_tx(&conn, None).unwrap();
        let cc = cat["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "CC-006")
            .unwrap();
        for k in [
            "classified_objects",
            "windows_total",
            "merged_bars",
            "effective_source_positions",
            "withdrawals",
            "replaces",
            "insufficient_knowledge",
            "domain_not_satisfied",
        ] {
            assert!(
                cc["evidence"][k].is_string(),
                "evidence.{k} 应为字符串：{}",
                cc["evidence"][k]
            );
        }
    }

    #[test]
    fn wire_integer_projection_preserves_bool_null_float() {
        let v = json!({
            "i64": 9007199254740993_i64,
            "small": 2,
            "negative": -1,
            "bool_true": true,
            "bool_false": false,
            "null": null,
            "float": 1.5,
            "str": "9007199254740993",
            "nested": {"a": 3, "b": [4, 5]},
        });
        let p = project_wire_integers(&v);
        assert_eq!(p["i64"], json!("9007199254740993"));
        assert_eq!(p["small"], json!("2"));
        assert_eq!(p["negative"], json!("-1"));
        assert_eq!(p["bool_true"], json!(true));
        assert_eq!(p["bool_false"], json!(false));
        assert_eq!(p["null"], Value::Null);
        assert_eq!(p["float"], json!(1.5));
        assert_eq!(p["str"], json!("9007199254740993"));
        assert_eq!(p["nested"]["a"], json!("3"));
        assert_eq!(p["nested"]["b"], json!(["4", "5"]));
    }

    #[test]
    fn bigint_source_coords_and_revisions_roundtrip_wire() {
        let files = TestFiles::new();
        init_test_db(&files);
        let big_seqs = [
            "9007199254741000",
            "9007199254741001",
            "9007199254741002",
            "9007199254741003",
        ];
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", big_seqs[0], "10000"),
                ev("e1", "1", big_seqs[1], "11000"),
                ev("e2", "9007199254740993", big_seqs[2], "10500"),
            ]),
            "big1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "9007199254740994", big_seqs[2], "11500")]),
            "big2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e3", "1", big_seqs[3], "10800")]),
            "big3.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();

        let s = snapshot_of(&files, None);
        let rh = s["raw_history"].as_array().unwrap();
        assert_eq!(rh.len(), 5);
        let e2rev2 = rh
            .iter()
            .find(|r| r["revision"] == "9007199254740994")
            .unwrap();
        assert_eq!(e2rev2["supersedes_revision"], json!("9007199254740993"));
        assert_eq!(e2rev2["source_coord"], json!("9007199254741002"));
        // wire：整数坐标/版本都是规范十进制字符串，不静默舍入。
        for r in rh {
            assert!(r["revision"].is_string());
            assert!(r["seq"].is_string());
            assert!(r["source_coord"].is_string());
        }
        let active = s["objects"].as_array().unwrap();
        assert_eq!(active.len(), 2);
        assert_eq!(
            active[0]["source_coords"],
            json!(["9007199254741000", "9007199254741001", "9007199254741002"])
        );
        assert_eq!(
            active[1]["source_coords"],
            json!(["9007199254741001", "9007199254741002", "9007199254741003"])
        );
        for o in active {
            for g in o["input_refs"].as_array().unwrap() {
                assert!(g["merged_index"].is_string());
                for rr in g["raw_refs"].as_array().unwrap() {
                    assert!(rr["seq"].is_string());
                    assert!(rr["revision"].is_string());
                }
            }
        }
    }

    // ── #1394：私有 changes-only/1 域的真实变化 Delta、撤回历史与三方对拍 ──

    fn delta_of(conn: &Connection, generation: i64) -> Value {
        let text = delta_text_of(conn, generation);
        serde_json::from_str(&text).unwrap()
    }

    /// 逻辑 Delta 原件的当前文本：整块域取列，图域取 `delta/<generation>` 私有根的无损展开。
    fn delta_text_of(conn: &Connection, generation: i64) -> String {
        if evidence_store::format(conn).unwrap().records_in_graph() {
            let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
            let bytes = reader
                .read_fixed_cut(&evidence_store::delta_root_key(generation))
                .unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        } else {
            conn.query_row(
                "SELECT delta_json FROM structure_deltas WHERE generation=?1",
                params![generation],
                |r| r.get(0),
            )
            .unwrap()
        }
    }

    /// 注入「错的 Delta 原件」：整块域改列；图域**整体替换**该代私有根（模拟发布方写错原件）。
    fn set_delta(conn: &Connection, generation: i64, delta: &Value) {
        if evidence_store::format(conn).unwrap().records_in_graph() {
            evidence_store::force_replace_root(
                conn,
                &evidence_store::delta_root_key(generation),
                delta,
            )
            .unwrap();
        } else {
            conn.execute(
                "UPDATE structure_deltas SET delta_json=?1 WHERE generation=?2",
                params![canonical_json(delta), generation],
            )
            .unwrap();
        }
    }

    fn sorted_records(v: &Value) -> Vec<String> {
        let mut out: Vec<String> = v.as_array().unwrap().iter().map(canonical_json).collect();
        out.sort();
        out
    }

    /// 消费端重放：base 快照 + 本代 Delta。upsert 按身份覆盖、撤回移入 withdrawn_objects，
    /// 发布头取自 Delta 自身（与 read_snapshot 的公开字段一一对应）。
    fn apply_delta(base: &Value, delta: &Value) -> Value {
        let mut out = base.clone();
        let mut objects: Vec<Value> = base["objects"].as_array().unwrap().clone();
        let mut withdrawn: Vec<Value> = base["withdrawn_objects"].as_array().unwrap().clone();
        for u in delta["upserts"].as_array().unwrap() {
            let id = u["object_id"].as_str().unwrap().to_string();
            objects.retain(|o| o["object_id"].as_str() != Some(id.as_str()));
            withdrawn.retain(|o| o["object_id"].as_str() != Some(id.as_str()));
            objects.push(u.clone());
        }
        for w in delta["withdrawals"].as_array().unwrap() {
            let id = w["object_id"].as_str().unwrap().to_string();
            if let Some(pos) = objects
                .iter()
                .position(|o| o["object_id"].as_str() == Some(id.as_str()))
            {
                let mut moved = objects.remove(pos);
                moved["lifecycle"] = json!("withdrawn");
                moved["withdrawn_generation"] = delta["generation"].clone();
                moved["withdrawal_reason"] = w["reason"].clone();
                moved["superseded_by"] = w["superseded_by"].clone();
                withdrawn.push(moved);
            }
        }
        for (field, key) in [
            ("witnesses", "witness_id"),
            ("relations", ""),
            ("observations", "observation_id"),
        ] {
            let mut list: Vec<Value> = out[field].as_array().unwrap().clone();
            for rec in delta[field].as_array().unwrap() {
                let fresh = canonical_json(rec);
                if list.iter().any(|old| {
                    if key.is_empty() {
                        canonical_json(old) == fresh
                    } else {
                        old[key] == rec[key]
                    }
                }) {
                    continue;
                }
                list.push(rec.clone());
            }
            out[field] = json!(list);
        }
        out["objects"] = json!(objects);
        out["withdrawn_objects"] = json!(withdrawn);
        for rec in delta["raw_history_added"].as_array().unwrap() {
            out["raw_history"].as_array_mut().unwrap().push(rec.clone());
        }
        out["generation"] = delta["generation"].clone();
        out["structure_cut"] = delta["next_cut"].clone();
        out["index_frontier"] = delta["index_frontier"].clone();
        for key in [
            "input_frontier",
            "seq_range",
            "catalog_run_status",
            "catalog_evidence",
        ] {
            out[key] = delta[key].clone();
        }
        out
    }

    /// #1394：新域下真正未变的对象不重发；四个首次元数据与批次列在跨代保持首发布值。
    #[test]
    fn changes_only_delta_omits_unchanged_objects_and_keeps_first_metadata() {
        let files = TestFiles::new();
        init_test_db(&files);
        let rows = tb02b_oracle_rows();
        for n in 1..=rows.len() {
            accept_ohlc(&files, &located_ohlc_input(&rows[..n]));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let gen: i64 = meta_get_opt(&conn, "generation")
            .unwrap()
            .unwrap()
            .parse()
            .unwrap();
        assert!(gen >= 2);
        let delta = delta_of(&conn, gen);
        let prev = snapshot_of(&files, Some(gen - 1));
        let next = snapshot_of(&files, None);
        let by_id = |snap: &Value| -> BTreeMap<String, Value> {
            snap["objects"]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| (o["object_id"].as_str().unwrap().to_string(), o.clone()))
                .collect()
        };
        let (old, new) = (by_id(&prev), by_id(&next));
        let upsert_ids: std::collections::BTreeSet<String> = delta["upserts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["object_id"].as_str().unwrap().to_string())
            .collect();
        let mut unchanged = 0;
        for (id, before) in &old {
            let Some(after) = new.get(id) else { continue };
            unchanged += 1;
            assert_eq!(
                canonical_json(before),
                canonical_json(after),
                "未变对象必须逐字段保持"
            );
            for key in [
                "first_known_generation",
                "first_known_cut",
                "batch_id",
                "published_generation",
            ] {
                assert_eq!(before[key], after[key], "首次元数据不得漂移：{id}.{key}");
            }
            assert!(
                !upsert_ids.contains(id),
                "未变对象不得出现在本代 upserts：{id}"
            );
        }
        assert!(unchanged >= 1, "该场景必须存在跨代未变对象");
        assert!(
            delta["upserts"].as_array().unwrap().len() < next["objects"].as_array().unwrap().len(),
            "本代 upsert 必须是完整活动集的真子集"
        );
        for u in delta["upserts"].as_array().unwrap() {
            assert_eq!(
                u["first_known_generation"],
                json!(gen.to_string()),
                "本代 upsert 只能是本代新生/替代目标：{u}"
            );
            assert_eq!(u["batch_id"], delta["index_frontier"]);
        }
    }

    /// #1394：替换/撤回历史完整；三族 Delta 精确等于本代首次发布（首次 batch 不外移）。
    #[test]
    fn changes_only_replacement_keeps_withdrawn_history_and_first_published_families() {
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        accept(
            &files,
            &test_input(&[ev("e2", "2", "2", "11500")]),
            "in2.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let delta = delta_of(&conn, 2);
        assert_eq!(delta["withdrawals"].as_array().unwrap().len(), 1);
        assert_eq!(delta["replaces"].as_array().unwrap().len(), 1);
        let old_id = delta["withdrawals"][0]["object_id"]
            .as_str()
            .unwrap()
            .to_string();
        let new_id = delta["replaces"][0]["new_object_id"]
            .as_str()
            .unwrap()
            .to_string();
        // 撤回历史完整：旧身份仍以原发生区间在库，撤回代/理由/替代指针闭合。
        let (wg, reason, sup): (Option<i64>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT withdrawn_generation,withdrawal_reason,superseded_by FROM objects WHERE object_id=?1",
                params![old_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(wg, Some(2));
        assert_eq!(reason.as_deref(), Some("superseded_by_revision"));
        assert_eq!(sup.as_deref(), Some(new_id.as_str()));
        // 替代目标：first_known 本代、批次本代，且被撤对象不得复活。
        let (fk, batch): (i64, String) = conn
            .query_row(
                "SELECT first_known_generation,batch_id FROM objects WHERE object_id=?1",
                params![new_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (fk, batch.as_str()),
            (2, delta["index_frontier"].as_str().unwrap())
        );
        assert!(!delta["upserts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["object_id"].as_str() == Some(old_id.as_str())));
        let snapshot = snapshot_of(&files, None);
        let withdrawn = snapshot["withdrawn_objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["object_id"].as_str() == Some(old_id.as_str()))
            .unwrap();
        assert_eq!(withdrawn["lifecycle"], json!("withdrawn"));
        assert_eq!(withdrawn["withdrawn_generation"], json!("2"));
        // 三族 Delta 精确等于本代首次发布（published_generation=2），不是完整累计集。
        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
        assert_eq!(
            delta["witnesses"].as_array().unwrap().len() as i64,
            count("SELECT COUNT(*) FROM witnesses WHERE published_generation=2")
        );
        assert_eq!(
            delta["relations"].as_array().unwrap().len() as i64,
            count("SELECT COUNT(*) FROM relations WHERE published_generation=2")
        );
        assert_eq!(
            delta["observations"].as_array().unwrap().len() as i64,
            count("SELECT COUNT(*) FROM observations WHERE published_generation=2")
        );
        assert!(
            (delta["witnesses"].as_array().unwrap().len() as i64)
                < count("SELECT COUNT(*) FROM witnesses"),
            "本代见证必须是完整见证集的真子集"
        );
        // 旧观察（若有）保留首次 batch：不重发，也不改持久行。
        let gen1_batch: String = conn
            .query_row(
                "SELECT index_frontier FROM structure_deltas WHERE generation=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let mut stmt = conn
            .prepare("SELECT observation_id,batch_id FROM observations ORDER BY observation_id")
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for (oid, batch) in rows {
            let resent = delta["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["observation_id"].as_str() == Some(oid.as_str()));
            if resent {
                assert_eq!(batch, delta["index_frontier"].as_str().unwrap());
            } else {
                assert_eq!(batch, gen1_batch, "旧观察必须保留首次 batch");
            }
        }
    }

    /// #1394：base Snapshot + Delta = next Snapshot（含发布头）；重复应用同一 Delta 不再生效。
    #[test]
    fn changes_only_base_snapshot_plus_delta_replays_next_snapshot() {
        let files = TestFiles::new();
        init_test_db(&files);
        let prices = [10, 5, 8, 11, 14, 17, 13, 18, 15];
        let bars: Vec<_> = prices.iter().map(|h| (h - 2, *h)).collect();
        for n in 1..=bars.len() {
            accept_ohlc(&files, &ohlc_input(&bars[..n], false));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let mut revised = ohlc_input(&bars, false);
        revised["events"][5]["revision"] = json!("2");
        revised["events"][5]["high"] = json!("18");
        revised["events"][5]["close"] = json!("18");
        revised["events"][5]["raw_text"] = json!("15/18 revision");
        accept_ohlc(&files, &revised);
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let gen: i64 = meta_get_opt(&conn, "generation")
            .unwrap()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(gen, bars.len() as i64 + 1);
        let mut replayed = snapshot_of(&files, Some(1));
        for g in 2..=gen {
            let delta = delta_of(&conn, g);
            let next = snapshot_of(&files, Some(g));
            replayed = apply_delta(&replayed, &delta);
            let twice = apply_delta(&replayed, &delta);
            for field in [
                "objects",
                "withdrawn_objects",
                "witnesses",
                "relations",
                "observations",
            ] {
                assert_eq!(
                    sorted_records(&replayed[field]),
                    sorted_records(&next[field]),
                    "base+Delta 必须逐字段等于 next Snapshot：{field} @ cut-{g}"
                );
                assert_eq!(
                    sorted_records(&twice[field]),
                    sorted_records(&replayed[field]),
                    "重复应用同一 Delta 不再生效：{field} @ cut-{g}"
                );
            }
            assert_eq!(
                sorted_records(&replayed["raw_history"]),
                sorted_records(&next["raw_history"]),
                "raw_history 必须完整续接 @ cut-{g}"
            );
            // 发布头（generation/cut/frontier/seq_range/catalog_evidence）也须等价，不能只比六族。
            for key in [
                "generation",
                "structure_cut",
                "index_frontier",
                "input_frontier",
                "seq_range",
                "catalog_run_status",
                "catalog_evidence",
            ] {
                assert_eq!(
                    canonical_json(&replayed[key]),
                    canonical_json(&next[key]),
                    "发布头 {key} 必须与 next Snapshot 一致 @ cut-{g}"
                );
            }
        }
    }

    /// #1394：新域漏 birth / 伪旧 member 都必须拒绝（不做降级重判）。
    #[test]
    fn changes_only_audit_rejects_missing_birth_and_fake_old_member() {
        let files = TestFiles::new();
        init_test_db(&files);
        let prices = [10, 5, 8, 11, 14, 17, 13, 18];
        let bars: Vec<_> = prices.iter().map(|h| (h - 2, *h)).collect();
        for n in 1..=bars.len() {
            accept_ohlc(&files, &ohlc_input(&bars[..n], false));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let gen: i64 = meta_get_opt(&conn, "generation")
            .unwrap()
            .unwrap()
            .parse()
            .unwrap();
        let delta = delta_of(&conn, gen);
        assert!(!delta["upserts"].as_array().unwrap().is_empty());
        // ① 漏 birth：抹掉本代新生 → 独立索引出生不再出现在 Delta。
        let mut missing = delta.clone();
        missing["upserts"] = json!([]);
        set_delta(&conn, gen, &missing);
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
        // ② 伪旧 member：把一个未变对象当本代 upsert 重新发布 → 它没有本代出生。
        set_delta(&conn, gen, &delta);
        verify_reachable_root(&conn).unwrap();
        let prev = snapshot_of(&files, Some(gen - 1));
        let fresh: std::collections::BTreeSet<String> = delta["upserts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["object_id"].as_str().unwrap().to_string())
            .collect();
        let old = prev["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| !fresh.contains(o["object_id"].as_str().unwrap()))
            .unwrap()
            .clone();
        let mut fake = delta.clone();
        fake["upserts"].as_array_mut().unwrap().push(old);
        set_delta(&conn, gen, &fake);
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
        set_delta(&conn, gen, &delta);
        verify_reachable_root(&conn).unwrap();
    }

    /// #1394：索引和 Delta 一起被改时，仍须由完整封存原件揭露漂移。
    #[test]
    fn changes_only_rejects_witness_index_and_delta_drift_from_sealed_source() {
        let files = TestFiles::new();
        init_test_db(&files);
        for (i, price) in ["10000", "11000", "10500"].iter().enumerate() {
            accept(
                &files,
                &test_input(&[ev(&format!("e{i}"), "1", &i.to_string(), price)]),
                &format!("input-{i}.json"),
                DEFAULT_WRITER_EPOCH,
            );
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let (id, old_high, born): (String, String, i64) = conn
            .query_row(
                "SELECT witness_id,merged_high,published_generation FROM witnesses ORDER BY witness_id LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        let changed_high = (old_high.parse::<i64>().unwrap() + 1).to_string();
        let mut delta = delta_of(&conn, born);
        let witness = delta["witnesses"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|w| w["witness_id"] == id)
            .unwrap();
        witness["merged_high"] = json!(changed_high);
        conn.execute(
            "UPDATE witnesses SET merged_high=?1 WHERE witness_id=?2",
            params![changed_high, id],
        )
        .unwrap();
        set_delta(&conn, born, &delta);
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("封存批次内容/发布代与独立索引不一致"));
        let output = std::process::Command::new("python3")
            .arg(fixture("tests/tb02b_verify.py"))
            .arg("--database")
            .arg(files.db())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("封存原件内容/发布代与独立索引不一致")
        );
    }

    /// #1394：旧 full 域（无登记表）保持旧判据——删掉冗余的未变对象 upsert 仍当损坏拒绝。
    #[test]
    fn legacy_full_domain_rejects_dropped_redundant_upsert() {
        let files = TestFiles::new();
        init_test_db(&files);
        // 模拟旧库：无私有登记表 → 旧 full 域（本测试的全部代都由新 writer 按旧域语义写入）。
        {
            let conn = open_db(&files.db()).unwrap();
            conn.execute("DROP TABLE s_storage_format", []).unwrap();
            verify_reachable_root(&conn).unwrap();
        }
        let prices = [10, 5, 8, 11, 14, 17, 13, 18];
        let bars: Vec<_> = prices.iter().map(|h| (h - 2, *h)).collect();
        for n in 1..=bars.len() {
            accept_ohlc(&files, &ohlc_input(&bars[..n], false));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let gen: i64 = meta_get_opt(&conn, "generation")
            .unwrap()
            .unwrap()
            .parse()
            .unwrap();
        let delta = delta_of(&conn, gen);
        assert_eq!(
            delta["upserts"].as_array().unwrap().len(),
            snapshot_of(&files, None)["objects"]
                .as_array()
                .unwrap()
                .len(),
            "旧 full 域的 Delta upserts 仍是完整活动集"
        );
        let mut broken = delta.clone();
        broken["upserts"].as_array_mut().unwrap().pop();
        set_delta(&conn, gen, &broken);
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
        set_delta(&conn, gen, &delta);
        verify_reachable_root(&conn).unwrap();
    }

    /// #1394：私有格式登记的未知值/缺行/多行/结构不符与暖 cache 后改模式，都在写入前拒绝。
    #[test]
    fn storage_format_registry_corruption_is_rejected_before_any_write() {
        // 先放对照：同一输入链在新旧两域必须得到同一 batch 字节/hash（见下一个测试）。
        let files = TestFiles::new();
        init_test_db(&files);
        accept(
            &files,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        let conn = open_db(&files.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        let before = db_meta(&conn).unwrap();
        // ① 未知模式值。
        conn.execute(
            "UPDATE s_storage_format SET storage_format='changes-only/9'",
            [],
        )
        .unwrap();
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("SchemaUnsupported"));
        for refused in [
            cmd_snapshot(&files.db(), None),
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH),
            cmd_recover(&files.db(), "2"),
        ] {
            assert!(refused.is_err());
        }
        assert_eq!(db_meta(&conn).unwrap(), before);
        // ② 缺行（表在而登记行没了）。
        conn.execute(
            "UPDATE s_storage_format SET storage_format='changes-only/1'",
            [],
        )
        .unwrap();
        verify_reachable_root(&conn).unwrap();
        conn.execute("DELETE FROM s_storage_format", []).unwrap();
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
        // ③ 多行（换掉 CHECK 的 DDL）：结构不符即拒绝，不按「最后一行」猜。
        conn.execute_batch(
            "DROP TABLE s_storage_format;
             CREATE TABLE s_storage_format(singleton INTEGER, storage_format TEXT);
             INSERT INTO s_storage_format VALUES(1,'changes-only/1'),(2,'changes-only/1');",
        )
        .unwrap();
        assert!(verify_reachable_root(&conn)
            .unwrap_err()
            .contains("StorageUnavailable"));
        drop(conn);
        // ④ 暖 cache 后改模式：已核前缀/证书身份含模式标签，改模式即失效且写前拒绝，库不变。
        let warm = TestFiles::new();
        init_test_db(&warm);
        accept(
            &warm,
            &test_input(&[
                ev("e0", "1", "0", "10000"),
                ev("e1", "1", "1", "11000"),
                ev("e2", "1", "2", "10500"),
            ]),
            "in1.json",
            DEFAULT_WRITER_EPOCH,
        );
        cmd_advance(&warm.db(), DEFAULT_WRITER_EPOCH).unwrap();
        cache::configure(268435456);
        let healthy = open_db(&warm.db()).unwrap();
        verify_reachable_root(&healthy).unwrap(); // 首次装入，尚无已核前缀可复用
        verify_reachable_root(&healthy).unwrap();
        assert_eq!(cache::last_resumed(), 1);
        let other = open_db(&warm.db()).unwrap();
        other
            .execute(
                "UPDATE s_storage_format SET storage_format='legacy-full/0'",
                [],
            )
            .unwrap();
        drop(other);
        let before = db_meta(&healthy).unwrap();
        assert!(verify_reachable_root(&healthy).is_err());
        for refused in [
            cmd_snapshot(&warm.db(), None),
            cmd_advance(&warm.db(), DEFAULT_WRITER_EPOCH),
            cmd_recover(&warm.db(), "2"),
        ] {
            assert!(refused.is_err());
        }
        assert_eq!(db_meta(&healthy).unwrap(), before);
        // ⑤ 暖 cache 后整表消失（退化为旧 full 域）：模式标签进缓存身份，已核前缀不得复用；
        //    稀疏 Delta 也不满足旧 full 判据，故仍在写入前拒绝（不是「旧全量失败了再试稀疏」）。
        let dropped = TestFiles::new();
        init_test_db(&dropped);
        let rows = tb02b_oracle_rows();
        for n in 1..=rows.len() {
            accept_ohlc(&dropped, &located_ohlc_input(&rows[..n]));
            cmd_advance(&dropped.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        cache::configure(268435456);
        let healthy = open_db(&dropped.db()).unwrap();
        verify_reachable_root(&healthy).unwrap(); // 首次装入
        verify_reachable_root(&healthy).unwrap();
        assert_eq!(cache::last_resumed(), rows.len());
        let other = open_db(&dropped.db()).unwrap();
        other.execute("DROP TABLE s_storage_format", []).unwrap();
        drop(other);
        let before = db_meta(&healthy).unwrap();
        assert!(verify_reachable_root(&healthy).is_err());
        assert_eq!(cache::last_resumed(), 0, "模式变化不得复用旧前缀/证书");
        for refused in [
            cmd_snapshot(&dropped.db(), None),
            cmd_advance(&dropped.db(), DEFAULT_WRITER_EPOCH),
            cmd_recover(&dropped.db(), "2"),
        ] {
            assert!(refused.is_err());
        }
        assert_eq!(db_meta(&healthy).unwrap(), before);
        cache::configure(0);
    }

    /// #1394：封存 batch 的字节/内容寻址 hash 不得随 Delta 域变化。
    /// #1394 第二段：图域每一代仍按**完整展开字节**核公开身份，且跨代共享必须真实发生。
    #[test]
    fn evidence_graph_is_lossless_public_identical_and_shared() {
        let files = TestFiles::new();
        init_test_db(&files);
        let rows = tb02b_oracle_rows();
        for n in 1..=rows.len() {
            accept_ohlc(&files, &located_ohlc_input(&rows[..n]));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
        let conn = open_db(&files.db()).unwrap();
        let format: String = conn
            .query_row("SELECT graph_format FROM s_evidence_format", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(format, evidence_store::GRAPH_FORMAT);
        let mut total = 0i64;
        let mut count = 0i64;
        {
            let mut stmt = conn
                .prepare("SELECT batch_id,canonical_bytes,byte_len FROM batches ORDER BY batch_id")
                .unwrap();
            let mut q = stmt.query([]).unwrap();
            while let Some(r) = q.next().unwrap() {
                let id: String = r.get(0).unwrap();
                let blob: Vec<u8> = r.get(1).unwrap();
                let len: i64 = r.get(2).unwrap();
                // 图域批次行只留长度占位；完整字节必须由私有根无损展开。
                assert!(blob.is_empty(), "图域不得再存整块字节：{id}");
                let bytes = load_batch_bytes(&conn, &id).unwrap();
                assert_eq!(bytes.len() as i64, len, "展开长度必须等于 byte_len");
                assert_eq!(format!("batch-{}", sha256_hex(&bytes)), id, "公开身份不变");
                total += len;
                count += 1;
            }
        }
        assert_eq!(count, rows.len() as i64);
        let (_, payload) = evidence_store::stats(&conn).unwrap();
        // #1394 第二段：三处大原件（批次 / Delta / 结构事实）同图，故共享率按**逻辑原件总量**
        // 计：节点负载必须显著小于「批次累计 + Delta 累计 + 事实三列累计」。
        let mut logical = total;
        let generations: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT generation FROM structure_deltas ORDER BY generation")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        for g in &generations {
            logical += delta_text_of(&conn, *g).len() as i64;
        }
        let fact_ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT object_id FROM structure_facts ORDER BY object_id")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let mut reader = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET);
        for id in &fact_ids {
            let bytes = reader
                .read_fixed_cut(&evidence_store::fact_root_key(id))
                .unwrap();
            logical += bytes.len() as i64;
        }
        assert!(
            payload * 2 < logical,
            "跨代共享必须真实发生：节点负载 {payload} 字节 vs 逻辑原件累计 {logical} 字节"
        );
        // 公开读取路径（固定 cut 快照/根审计）在图上必须照常成立。
        verify_reachable_root(&conn).unwrap();
        assert_eq!(
            snapshot_of(&files, None)["generation"],
            json!(rows.len().to_string())
        );
    }

    /// #1394 第二段：当前读集下改历史叶/长度/引用/根/登记，任何写入之前一律拒绝且不留痕。
    #[test]
    fn evidence_graph_damage_blocks_every_write_before_it_happens() {
        for sql in [
            // 改叶字节但保 node ID（内容寻址不符）。
            "UPDATE s_evidence_nodes SET payload=x'00' WHERE node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_kind=0 AND length(payload)>8 LIMIT 1)",
            // 改拼接节点声明长度。
            "UPDATE s_evidence_nodes SET expanded_length=0 WHERE node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_kind=1 LIMIT 1)",
            // 改叶声明长度。
            "UPDATE s_evidence_nodes SET expanded_length=expanded_length+3 WHERE node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_kind=0 LIMIT 1)",
            // 删中间节点（闭包缺项）。
            "DELETE FROM s_evidence_nodes WHERE node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_kind=1 LIMIT 1)",
            // 换根指针（指向另一个存在的叶）。
            "UPDATE s_evidence_roots SET node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_kind=0 LIMIT 1)",
            // 改根长度 / 摘要 / 参数槽。
            "UPDATE s_evidence_roots SET expanded_length=expanded_length+1",
            "UPDATE s_evidence_roots SET content_sha256='0'||substr(content_sha256,2)",
            "UPDATE s_evidence_roots SET slot_ids=x''",
            // 删根 / 删批次行（不可达根与悬空根）。
            "DELETE FROM s_evidence_roots WHERE root_key=(SELECT root_key FROM s_evidence_roots ORDER BY root_key LIMIT 1)",
            "DELETE FROM batches WHERE batch_id=(SELECT batch_id FROM batches ORDER BY batch_id LIMIT 1)",
            // 伪造不可达残留节点 / 未声明种类 / 未声明格式。
            "INSERT INTO s_evidence_nodes(node_id,node_kind,payload,expanded_length) VALUES(zeroblob(32),0,x'00',1)",
            "UPDATE s_evidence_nodes SET node_kind=7 WHERE node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_kind=0 LIMIT 1)",
            "UPDATE s_evidence_format SET graph_format='evidence-graph/9'",
            // 图域批次行仍带整块字节（与私有根混写）。
            "UPDATE batches SET canonical_bytes=x'7b7d' WHERE batch_id=(SELECT batch_id FROM batches ORDER BY batch_id LIMIT 1)",
        ] {
            cache::configure(268435456);
            let files = TestFiles::new();
            init_test_db(&files);
            // 4 代足以覆盖「同一节点跨代被复用后受损」的冷/暖两条读集，避免逐例跑满全链。
            let oracle = tb02b_oracle_rows();
            let rows = &oracle[..4];
            for n in 1..=rows.len() {
                accept_ohlc(&files, &located_ohlc_input(&rows[..n]));
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
            // 先暖一次当前读集，确保损坏不被旧证书掩盖。
            let healthy = open_db(&files.db()).unwrap();
            verify_reachable_root(&healthy).unwrap();
            verify_reachable_root(&healthy).unwrap();
            assert_eq!(cache::last_resumed(), rows.len());
            let other = open_db(&files.db()).unwrap();
            other.execute_batch(sql).unwrap();
            drop(other);
            let before = graph_digest(&healthy);
            assert!(verify_reachable_root(&healthy).is_err(), "{sql}");
            for refused in [
                cmd_snapshot(&files.db(), None),
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH),
                cmd_recover(&files.db(), "2"),
            ] {
                assert!(refused.is_err(), "{sql}");
            }
            assert_eq!(graph_digest(&healthy), before, "拒绝前不得留下任何写入：{sql}");
            cache::configure(0);
        }
    }

    /// #1394 第二段：同 ID 异内容必须在**发布入口**拒绝，不能吞掉等将来读审计发现。
    #[test]
    fn evidence_graph_publish_rejects_same_id_different_content() {
        let files = TestFiles::new();
        init_test_db(&files);
        let conn = open_db(&files.db()).unwrap();
        let value = json!({"a":[1,2,3,4],"b":"x","c":{"d":"y"}});
        assert_eq!(
            canonical_bytes_of_value(&value),
            br#"{"a":[1,2,3,4],"b":"x","c":{"d":"y"}}"#
        );
        let staged = evidence_store::stage_complete(&value, &[]).unwrap();
        let key = evidence_store::batch_root_key("batch-unit-test");
        evidence_store::publish_refs(&conn, &key, &staged).unwrap();
        // 节点同 ID 异内容：保留 node_id 改负载，再发布同一份 staging。
        conn.execute(
            "UPDATE s_evidence_nodes SET payload=x'00' WHERE node_id=(SELECT node_id FROM s_evidence_nodes WHERE length(payload)>1 LIMIT 1)",
            [],
        )
        .unwrap();
        let err = evidence_store::publish_refs(&conn, &key, &staged).unwrap_err();
        assert!(err.contains("IdentityConflict"), "{err}");
        // 根同 key 异内容：换根指针后重新发布必须拒绝覆写。
        conn.execute(
            "UPDATE s_evidence_roots SET node_id=(SELECT node_id FROM s_evidence_nodes WHERE node_id<>s_evidence_roots.node_id LIMIT 1)",
            [],
        )
        .unwrap();
        let err = evidence_store::publish_refs(&conn, &key, &staged).unwrap_err();
        assert!(err.contains("IdentityConflict"), "{err}");
    }

    /// #1394 第二段：旧库整块域继续严格可读；半降级/混写一律拒绝，不提供降级通道。
    #[test]
    fn evidence_graph_downgrade_is_strict() {
        let build = |files: &TestFiles| {
            init_test_db(files);
            let rows = tb02b_oracle_rows();
            for n in 1..=rows.len() {
                accept_ohlc(files, &located_ohlc_input(&rows[..n]));
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
        };
        // ① 完整降级为旧库：整块字节写回批次行并撤掉三表 → 旧整块判据仍必须严格可读。
        let legacy = TestFiles::new();
        build(&legacy);
        {
            let conn = open_db(&legacy.db()).unwrap();
            let ids: Vec<String> = {
                let mut stmt = conn
                    .prepare("SELECT batch_id FROM batches ORDER BY batch_id")
                    .unwrap();
                stmt.query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
            };
            for id in ids {
                let bytes = load_batch_bytes(&conn, &id).unwrap();
                conn.execute(
                    "UPDATE batches SET canonical_bytes=?1 WHERE batch_id=?2",
                    params![bytes, id],
                )
                .unwrap();
            }
            // #1394 第二段：三处大原件都要写回行内整块列，降级后的旧库才与旧 writer 的产物同形。
            let generations: Vec<i64> = {
                let mut stmt = conn
                    .prepare("SELECT generation FROM structure_deltas ORDER BY generation")
                    .unwrap();
                stmt.query_map([], |r| r.get::<_, i64>(0))
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap()
            };
            for g in generations {
                let text = delta_text_of(&conn, g);
                conn.execute(
                    "UPDATE structure_deltas SET delta_json=?1 WHERE generation=?2",
                    params![text, g],
                )
                .unwrap();
            }
            let fact_ids: Vec<String> = {
                let mut stmt = conn
                    .prepare("SELECT object_id FROM structure_facts ORDER BY object_id")
                    .unwrap();
                stmt.query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap()
            };
            let mut reader = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET);
            for id in fact_ids {
                let bytes = reader
                    .read_fixed_cut(&evidence_store::fact_root_key(&id))
                    .unwrap();
                let (payload, refs, coords) =
                    evidence_store::split_fact_root(&bytes, &format!("事实 `{id}`")).unwrap();
                conn.execute(
                    "UPDATE structure_facts SET payload_json=?1,input_refs_json=?2,source_coords_json=?3 WHERE object_id=?4",
                    params![payload, refs, coords, id],
                )
                .unwrap();
            }
            conn.execute_batch("DELETE FROM s_evidence_roots; DELETE FROM s_evidence_nodes; DROP TABLE s_evidence_roots; DROP TABLE s_evidence_nodes; DROP TABLE s_evidence_format;")
                .unwrap();
        }
        let conn = open_db(&legacy.db()).unwrap();
        verify_reachable_root(&conn).unwrap();
        cmd_snapshot(&legacy.db(), None).unwrap();
        drop(conn);
        // ② 半降级（缺一张表）仍按已知协议模式拒绝。
        for drop_sql in [
            "DROP TABLE s_evidence_roots",
            "DROP TABLE s_evidence_nodes",
            "DROP TABLE s_evidence_format",
        ] {
            let half = TestFiles::new();
            build(&half);
            let conn = open_db(&half.db()).unwrap();
            conn.execute(drop_sql, []).unwrap();
            drop(conn);
            assert!(cmd_snapshot(&half.db(), None).is_err(), "{drop_sql}");
            assert!(
                cmd_advance(&half.db(), DEFAULT_WRITER_EPOCH).is_err(),
                "{drop_sql}"
            );
        }
        // ③ 三表齐全但未声明/坏声明：与模式损坏同门。
        for sql in [
            "DELETE FROM s_evidence_format",
            "UPDATE s_evidence_format SET graph_format='evidence-graph/9'",
            "UPDATE s_evidence_format SET graph_format='evidence-graph/1'",
        ] {
            let bad = TestFiles::new();
            build(&bad);
            let conn = open_db(&bad.db()).unwrap();
            conn.execute(sql, []).unwrap();
            drop(conn);
            assert!(cmd_snapshot(&bad.db(), None).is_err(), "{sql}");
        }
    }

    /// 库的持久摘要：拒绝路径必须逐字节原地不动（含 meta 与图三表）。
    #[cfg(test)]
    pub(super) fn graph_digest(conn: &Connection) -> String {
        let mut parts: Vec<String> = Vec::new();
        for sql in [
            "SELECT key||'='||value FROM meta ORDER BY key",
            "SELECT batch_id||':'||byte_len||':'||hex(canonical_bytes) FROM batches ORDER BY batch_id",
            "SELECT hex(node_id)||':'||node_kind||':'||expanded_length||':'||hex(payload) FROM s_evidence_nodes ORDER BY node_id",
            "SELECT root_key||':'||hex(node_id)||':'||hex(slot_ids)||':'||expanded_length||':'||content_sha256 FROM s_evidence_roots ORDER BY root_key",
            "SELECT graph_format FROM s_evidence_format ORDER BY singleton",
        ] {
            let mut stmt = conn.prepare(sql).unwrap();
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            parts.extend(rows);
        }
        sha256_hex(parts.join("|").as_bytes())
    }

    /// 同一条输入链分别只改「有没有私有登记表」，逐代 batch_id 与 byte_len 必须完全相同；
    /// 变化只能落在 Delta 的 upserts 上（新域少发未变对象）。
    #[test]
    fn batch_bytes_and_hash_do_not_depend_on_delta_domain() {
        let build = |legacy: bool| -> (TestFiles, Vec<(i64, String, i64)>, Vec<usize>) {
            let files = TestFiles::new();
            init_test_db(&files);
            if legacy {
                let conn = open_db(&files.db()).unwrap();
                conn.execute("DROP TABLE s_storage_format", []).unwrap();
            }
            let prices = [10, 5, 8, 11, 14, 17, 13, 18];
            let bars: Vec<_> = prices.iter().map(|h| (h - 2, *h)).collect();
            for n in 1..=bars.len() {
                accept_ohlc(&files, &ohlc_input(&bars[..n], false));
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
            let conn = open_db(&files.db()).unwrap();
            let batches: Vec<(i64, String, i64)> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT d.generation,b.batch_id,b.byte_len FROM structure_deltas d
                         JOIN batches b ON b.batch_id=d.index_frontier ORDER BY d.generation",
                    )
                    .unwrap();
                stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                    .unwrap()
                    .collect::<Result<_, _>>()
                    .unwrap()
            };
            let upserts: Vec<usize> = (1..=batches.len() as i64)
                .map(|g| delta_of(&conn, g)["upserts"].as_array().unwrap().len())
                .collect();
            (files, batches, upserts)
        };
        let (_new_files, new_batches, new_upserts) = build(false);
        let (_old_files, old_batches, old_upserts) = build(true);
        assert_eq!(
            new_batches, old_batches,
            "batch 字节长度与内容寻址 hash 必须与 Delta 域无关"
        );
        assert_eq!(new_upserts.len(), old_upserts.len());
        assert!(
            new_upserts
                .iter()
                .zip(&old_upserts)
                .all(|(new, old)| new <= old),
            "新域的 upserts 不得多于旧 full 域"
        );
        assert!(
            new_upserts
                .iter()
                .zip(&old_upserts)
                .any(|(new, old)| new < old),
            "新域必须在至少一代真正少发未变对象"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // #1394 2A-S/2C：增量路径 vs 全量 oracle 的逐代对拍与测量入口（显式 ignored）。
    //
    // 对拍面：同一输入逐代 Advance 的 batch_id（= 每代完整快照的内容寻址身份）必须逐代相同。
    // 测量面：parse / facts 阶段 wall time、稳定对象命中/未命中、CC-007 比较单元实算/复用。
    // ────────────────────────────────────────────────────────────────────────

    /// 同源 OHLC 输入：`bars[i] = (low, high)`，逐根一代。
    fn incremental_bars(n: usize) -> Vec<(i64, i64)> {
        (0..n)
            .map(|i| {
                let base = 1000 + (i as i64) * 3 + ((i as i64 * 37) % 47) - 23;
                let span = 1 + (i as i64 % 5);
                (base, base + span)
            })
            .collect()
    }

    /// 逐根 Advance，返回每代的 (batch_id, batch 字节数)。`lane_cold` 为真时每代先清空车道。
    fn drive_prefix(bars: &[(i64, i64)], lane_cold: bool) -> Vec<(String, usize)> {
        let files = TestFiles::new();
        init_test_db(&files);
        let mut out = Vec::new();
        for n in 1..=bars.len() {
            if lane_cold {
                s_fact_cache::cold_lane();
            }
            accept_ohlc(&files, &ohlc_input(&bars[..n], false));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            let conn = open_db(&files.db()).unwrap();
            let (batch_id, byte_len): (String, i64) = conn
                .query_row(
                    "SELECT d.index_frontier, b.byte_len FROM structure_deltas d
                     JOIN batches b ON b.batch_id=d.index_frontier ORDER BY d.generation DESC LIMIT 1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            out.push((batch_id, usize::try_from(byte_len).unwrap()));
        }
        out
    }

    #[test]
    #[ignore = "#1394 显式测量入口（逐代增量状态 + 稳定对象复用 + 比较单元缓存）"]
    fn issue1394_incremental_prefix_measurement() {
        let prefix: usize = std::env::var("ISSUE1394_MEASURE_PREFIX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(64);
        let bars = incremental_bars(prefix);
        // 1) 增量路径（暖车道）
        s_fact_cache::phase::reset();
        s_fact_cache::phase::enable(true);
        s_fact_cache::warm_lane_reset();
        let started = std::time::Instant::now();
        let warm = drive_prefix(&bars, false);
        let warm_total = started.elapsed();
        let phases = s_fact_cache::phase::summary();
        let counts = s_fact_cache::lane_counts();
        s_fact_cache::phase::enable(false);
        // 2) 全量 oracle（同输入，另一个库）
        s_fact_cache::set_oracle(true);
        let started = std::time::Instant::now();
        let oracle = drive_prefix(&bars, false);
        let oracle_total = started.elapsed();
        s_fact_cache::set_oracle(false);
        assert_eq!(
            warm, oracle,
            "增量路径与全量 oracle 的逐代 batch_id/字节数必须逐项相同"
        );
        // 3) 冷车道（每代清空）必须同样一致
        let cold = drive_prefix(&bars, true);
        assert_eq!(cold, warm, "冷车道与暖车道输出必须一致");
        let report = serde_json::json!({
            "schema": "issue1394-incremental-prefix/1",
            "prefix": prefix.to_string(),
            "generations": warm.len().to_string(),
            "warm_wall_seconds": format!("{:.6}", warm_total.as_secs_f64()),
            "oracle_wall_seconds": format!("{:.6}", oracle_total.as_secs_f64()),
            "warm_vs_oracle_ratio": format!("{:.4}", warm_total.as_secs_f64() / oracle_total.as_secs_f64()),
            "batch_bytes_last": warm.last().map(|(_, b)| b.to_string()).unwrap_or_default(),
            "batch_bytes_total": warm.iter().map(|(_, b)| *b as u64).sum::<u64>().to_string(),
            "phases": phases.iter().map(|(name, nanos, count)| serde_json::json!({
                "phase": name, "total_ms": format!("{:.3}", *nanos as f64 / 1e6), "calls": count.to_string()})).collect::<Vec<_>>(),
            "fact_cache": {
                "hits": counts.0.to_string(), "misses": counts.1.to_string(),
                "units_computed": counts.2.to_string(), "units_reused": counts.3.to_string(),
            },
            "batch_hashes": warm.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        });
        println!("{report}");
    }

    #[test]
    fn issue1394_same_price_revision_preserves_exact_current_source() {
        let bars = [(0, 1), (4, 8), (5, 7), (2, 4), (6, 9)];
        let run = |oracle: bool| {
            s_fact_cache::warm_lane_reset();
            s_fact_cache::set_oracle(oracle);
            let files = TestFiles::new();
            init_test_db(&files);
            let mut published = Vec::new();
            for revision in 1..=3 {
                let mut input = ohlc_input(&bars, false);
                // 价格与坐标均不变，只有真实事件版本变更；来源原件仍须更换。
                input["events"][0]["revision"] = json!(revision.to_string());
                accept_ohlc(&files, &input);
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
                let conn = open_db(&files.db()).unwrap();
                verify_reachable_root(&conn).unwrap();
                let batch_id: String = conn
                    .query_row(
                        "SELECT index_frontier FROM structure_deltas WHERE generation=?1",
                        params![revision],
                        |row| row.get(0),
                    )
                    .unwrap();
                let bytes = load_batch_bytes(&conn, &batch_id).unwrap();
                for (old, (_, snapshot)) in published.iter().enumerate() {
                    assert!(
                        &snapshot_of(&files, Some(old as i64 + 1)) == snapshot,
                        "真实 revision 后旧 cut {} 改变",
                        old + 1
                    );
                }
                published.push((bytes, snapshot_of(&files, Some(revision))));
            }
            s_fact_cache::set_oracle(false);
            published
        };
        let warm = run(false);
        let oracle = run(true);
        assert_ne!(oracle[0].0, oracle[1].0, "真实 revision 必须改变原件身份");
        for (index, (actual, expected)) in warm.iter().zip(&oracle).enumerate() {
            assert!(
                actual == expected,
                "同价 revision 第 {} 代的完整 batch 字节/快照不一致：warm={} oracle={}",
                index + 1,
                sha256_hex(&actual.0),
                sha256_hex(&expected.0)
            );
        }
    }

    #[test]
    fn issue1394_incremental_matches_oracle_on_revision_return() {
        // 真实修订（消失/返回）+ 追加：增量路径与 oracle 逐代 batch_id 必须相同。
        let bars = incremental_bars(12);
        // 与 ohlc_input 同形，但把 events[index] 的 revision 设为给定值（真实 late revision）。
        let revision_input = |bars: &[(i64, i64)], index: usize, revision: &str| -> Value {
            let events: Vec<Value> = bars
                .iter()
                .enumerate()
                .map(|(i, (low, high))| {
                    json!({
                        "event_id":format!("o{i}"),
                        "revision":if i == index { revision } else { "1" },
                        "seq":i.to_string(),
                        "received_at":"2026-09-13T00:00:00Z",
                        "raw_text":format!("{low}/{high}"),
                        "open":low.to_string(),"high":high.to_string(),"low":low.to_string(),"close":high.to_string(),
                        "timestamp":i.to_string(),"volume":"1"
                    })
                })
                .collect();
            json!({"schema_revision":"s-ohlc/1","session_id":"s-session-testonly-001","source_namespace":"testonly.ohlc",
                "source_epoch":"1","instrument":"TEST.OHLC","profile":tb02a::PROFILE,"events":events})
        };
        let mut revised = bars.clone();
        revised[2] = (bars[2].0 + 7, bars[2].1 + 7);
        let steps: Vec<Vec<(i64, i64)>> = vec![
            bars.clone(),
            revised.clone(),
            bars.clone(), // 返回原价（revision 3）
        ];
        let run = |oracle: bool| -> Vec<(i64, String)> {
            s_fact_cache::warm_lane_reset();
            s_fact_cache::set_oracle(oracle);
            let files = TestFiles::new();
            init_test_db(&files);
            for n in 1..=bars.len() {
                accept_ohlc(&files, &ohlc_input(&bars[..n], false));
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            }
            accept_ohlc(&files, &revision_input(&steps[1], 2, "2"));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            accept_ohlc(&files, &revision_input(&steps[2], 2, "3"));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
            s_fact_cache::set_oracle(false);
            let conn = open_db(&files.db()).unwrap();
            let mut stmt = conn
                .prepare("SELECT d.generation, d.index_frontier FROM structure_deltas d ORDER BY d.generation")
                .unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let warm_ids = run(false);
        let oracle_ids = run(true);
        assert_eq!(
            warm_ids, oracle_ids,
            "真实修订（消失/返回）下增量与 oracle 的逐代 batch_id 必须相同"
        );
        assert_eq!(warm_ids.len(), bars.len() + 2);
    }
}

#[cfg(test)]
mod staged_graph_audit_regressions {
    use super::*;
    use crate::tests::{
        accept_ohlc, fixture, graph_digest, init_test_db, located_ohlc_input, tb02b_oracle_rows,
        write_input, TestFiles,
    };

    /// 把库从 `evidence-graph/3` 退成**真正的** `evidence-graph/2`：Delta 与结构事实三列大原件
    /// 写回行内整块列，撤掉 delta/fact 根（节点按不可达清理），再把登记行改成 v2。
    fn downgrade_to_graph2(conn: &Connection) {
        let generations: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT generation FROM structure_deltas ORDER BY generation")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        for g in generations {
            let text = delta_text_of_test(conn, g);
            conn.execute(
                "UPDATE structure_deltas SET delta_json=?1 WHERE generation=?2",
                params![text, g],
            )
            .unwrap();
        }
        let fact_ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT object_id FROM structure_facts ORDER BY object_id")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
        for id in fact_ids {
            let bytes = reader
                .read_fixed_cut(&evidence_store::fact_root_key(&id))
                .unwrap();
            let (payload, refs, coords) =
                evidence_store::split_fact_root(&bytes, &format!("事实 `{id}`")).unwrap();
            conn.execute(
                "UPDATE structure_facts SET payload_json=?1,input_refs_json=?2,source_coords_json=?3 WHERE object_id=?4",
                params![payload, refs, coords, id],
            )
            .unwrap();
        }
        drop(reader);
        conn.execute(
            "DELETE FROM s_evidence_roots WHERE root_key LIKE 'delta/%' OR root_key LIKE 'fact/%'",
            [],
        )
        .unwrap();
        evidence_store::prune_unreachable_nodes(conn).unwrap();
        conn.execute(
            "UPDATE s_evidence_format SET graph_format=?1",
            params![evidence_store::GRAPH_FORMAT_V2],
        )
        .unwrap();
    }

    fn delta_text_of_test(conn: &Connection, generation: i64) -> String {
        if evidence_store::format(conn).unwrap().records_in_graph() {
            let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
            let bytes = reader
                .read_fixed_cut(&evidence_store::delta_root_key(generation))
                .unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        } else {
            conn.query_row(
                "SELECT delta_json FROM structure_deltas WHERE generation=?1",
                params![generation],
                |r| r.get(0),
            )
            .unwrap()
        }
    }

    fn build_oracle(files: &TestFiles, generations: usize) {
        init_test_db(files);
        let rows = tb02b_oracle_rows();
        for n in 1..=generations.min(rows.len()) {
            accept_ohlc(files, &located_ohlc_input(&rows[..n]));
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        }
    }

    fn all_batch_ids(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT batch_id FROM batches ORDER BY batch_id")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn read_all_batches(conn: &Connection) -> Vec<(String, Vec<u8>)> {
        all_batch_ids(conn)
            .into_iter()
            .map(|id| {
                let bytes = load_batch_bytes(conn, &id).unwrap();
                (id, bytes.to_vec())
            })
            .collect()
    }

    /// 同 cut 快照（用于「缓存不得改变查询结果」的逐字段对拍）。
    fn snapshot_here(files: &TestFiles) -> Value {
        let conn = open_db(&files.db()).unwrap();
        let tx = conn.unchecked_transaction().unwrap();
        read_snapshot_in_tx(&tx, None).unwrap()
    }

    /// 图域读路径的「查询结果」：逐代 Delta 原件 + 每条结构事实三列原件。
    /// （批次原件由 `read_all_batches` 单独对拍。）
    fn graph_record_texts(conn: &Connection) -> Vec<(String, Vec<u8>)> {
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        let generations: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT generation FROM structure_deltas ORDER BY generation")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        for g in generations {
            out.push((
                format!("delta/{g}"),
                delta_text_of_test(conn, g).into_bytes(),
            ));
        }
        let fact_ids: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT object_id FROM structure_facts ORDER BY object_id")
                .unwrap();
            stmt.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let mut reader = evidence_store::Reader::new(conn, EVIDENCE_READ_BUDGET);
        for id in fact_ids {
            let bytes = reader
                .read_fixed_cut(&evidence_store::fact_root_key(&id))
                .unwrap();
            out.push((format!("fact/{id}"), bytes.to_vec()));
        }
        out
    }

    /// #1394 第三段（exact-originals）：同一数据下**冷 / 暖 / 零预算 / 不足预算 / 足预算**读出的
    /// 完整原始 bytes 必须逐字节一致；单一 source-byte 预算（保留量 + 本次捕获瞬时读集）在每一处
    /// 都实际满足。暖读必须真的命中已核句柄——原件逐字节比较通过才复用，不是摘要相等。
    #[test]
    fn graph3_cold_warm_and_budgeted_reads_are_byte_identical_and_budget_bounded() {
        let files = TestFiles::new();
        build_oracle(&files, 3);
        let conn = open_db(&files.db()).unwrap();
        let ample = 1usize << 30;

        // ① 冷（丢掉跨请求持有）：真的完整展开，且不留任何命中。
        cache::configure(ample);
        cache::clear_handles();
        cache::counters_reset();
        let cold = read_all_batches(&conn);
        let counters = cache::counters_snapshot();
        assert!(counters.batch_expansions > 0, "冷读必须真的展开");
        assert_eq!(counters.batch_handle_hits, 0, "冷读不得命中句柄");
        let cold_records = graph_record_texts(&conn);
        let cold_snapshot = snapshot_here(&files);
        assert!(cache::retained_bytes() <= ample);
        assert_eq!(cache::capture_bytes_now(), 0, "捕获结束必须释放瞬时读集");

        // ② 暖（同一配置）：逐字节一致，且真的复用（命中 > 0、重新展开 == 0）。
        cache::counters_reset();
        let warm = read_all_batches(&conn);
        let counters = cache::counters_snapshot();
        assert_eq!(cold, warm, "冷/暖完整原始 bytes 必须逐字节一致");
        assert_eq!(
            cold_records,
            graph_record_texts(&conn),
            "冷/暖查询结果必须一致"
        );
        assert_eq!(cold_snapshot, snapshot_here(&files), "冷/暖快照必须一致");
        assert!(counters.batch_handle_hits > 0, "暖读必须复用已核句柄");
        assert_eq!(counters.batch_expansions, 0, "暖读不得重新展开");
        assert!(counters.cert_compared > 0, "暖读必须逐字节比较原件证书");
        assert!(cache::retained_bytes() <= ample);

        // ③ 零预算：不留任何跨请求持有，结果仍必须逐字节一致。
        cache::configure(0);
        let zero = read_all_batches(&conn);
        assert_eq!(cold, zero, "零预算读出的 bytes 必须一致");
        assert_eq!(
            cold_records,
            graph_record_texts(&conn),
            "零预算查询结果必须一致"
        );
        assert_eq!(cache::retained_bytes(), 0, "零预算不得保留任何跨请求持有");

        // ④ 不足预算：装不下的只失去复用，bytes 与自有预算仍必须成立。
        let tight = 64 * 1024;
        cache::configure(tight);
        cache::clear_handles();
        let limited = read_all_batches(&conn);
        assert_eq!(cold, limited, "不足预算读出的 bytes 必须一致");
        assert_eq!(
            cold_records,
            graph_record_texts(&conn),
            "不足预算查询结果必须一致"
        );
        assert!(
            cache::retained_bytes() <= tight,
            "不足预算下保留量必须在预算内：{}",
            cache::retained_bytes()
        );

        // ⑤ 预算重回足额：仍必须逐字节一致，并重新出现复用。
        cache::configure(ample);
        cache::clear_handles();
        let _ = read_all_batches(&conn);
        cache::counters_reset();
        let amply = read_all_batches(&conn);
        assert_eq!(cold, amply, "足预算读出的 bytes 必须一致");
        assert_eq!(
            cold_records,
            graph_record_texts(&conn),
            "足预算查询结果必须一致"
        );
        assert_eq!(cold_snapshot, snapshot_here(&files), "足预算快照必须一致");
        assert!(cache::counters_snapshot().batch_handle_hits > 0);
        cache::configure(268_435_456);
    }

    /// #1394 第三段：单一 source-byte 预算在**捕获进行中**（节点加入 / 句柄加入）、捕获结束、
    /// 以及 root 替换（前缀变长）之后都实际满足；超限只失去复用。
    #[test]
    fn graph3_source_byte_budget_holds_inside_and_after_captures() {
        let files = TestFiles::new();
        build_oracle(&files, 2);
        let conn = open_db(&files.db()).unwrap();
        for limit in [1usize << 30, 1 << 20, 4096] {
            cache::configure(limit);
            {
                // 捕获进行中：每读一个根，保留量 + 瞬时读集都必须在同一上限内。
                let mut reader = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET);
                for id in all_batch_ids(&conn) {
                    let _ = reader
                        .read_fixed_cut(&evidence_store::batch_root_key(&id))
                        .unwrap();
                    assert!(
                        cache::retained_bytes().saturating_add(cache::capture_bytes_now()) <= limit,
                        "捕获中预算必须满足（limit={limit}）"
                    );
                }
            }
            // 捕获结束：瞬时读集释放；保留量仍在预算内。
            assert_eq!(cache::capture_bytes_now(), 0, "捕获结束必须释放瞬时读集");
            assert!(
                cache::retained_bytes() <= limit,
                "捕获结束后保留量必须在预算内（limit={limit}）"
            );
            // 暖读与完整根审计在不足预算下同样必须通过（超限只失去复用，不改判据）。
            verify_reachable_root(&conn).unwrap();
            assert!(
                cache::retained_bytes() <= limit,
                "根审计后保留量必须在预算内（limit={limit}）"
            );
        }
        drop(conn);
        // root 替换（新增一代 → 前缀变长）：新 root 装入并驱逐后仍必须在同一预算内。
        let tight = 64 * 1024;
        cache::configure(tight);
        let rows = tb02b_oracle_rows();
        accept_ohlc(&files, &located_ohlc_input(&rows[..3]));
        cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap();
        assert!(
            cache::retained_bytes() <= tight,
            "root 替换后保留量必须在预算内：{} 分解(root,handles,certs)={:?}",
            cache::retained_bytes(),
            cache::retained_breakdown()
        );
        cache::configure(268_435_456);
    }

    /// #1394 第三段：审计前缀复用必须是「同一份已核 Delta 原件」——末列摘要逐字节相同也不够。
    /// 丢掉原件证书后（同一个摘要、另一份证书对象）不得复用前缀，而查询结果必须完全一致。
    #[test]
    fn graph3_audit_prefix_reuse_requires_same_originals_and_keeps_results_identical() {
        let files = TestFiles::new();
        build_oracle(&files, 3);
        cache::configure(1 << 30);
        cache::clear_handles();
        let conn = open_db(&files.db()).unwrap();
        let cold_snapshot = snapshot_here(&files);
        // 冷：完整核验一次，保留已核原件证书与审计前缀。
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 0, "冷捕获不得复用前缀");
        let digest = graph_digest(&conn);
        // 暖：前缀行逐字节相同 **且**每代 Delta 原件是同一份已核证书 ⟹ 前缀复用。
        cache::counters_reset();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 3, "暖捕获必须复用已核前缀（3 代）");
        assert!(
            cache::counters_snapshot().cert_compared > 0,
            "复用前必须逐字节比较原件证书"
        );
        assert_eq!(graph_digest(&conn), digest, "缓存不得改变任何持久事实");
        assert_eq!(snapshot_here(&files), cold_snapshot, "查询结果必须一致");

        // 丢掉跨请求持有：末列摘要逐字节不变，但原件证书是**另一份**对象（指针不同）
        // ⟹ 不得冒充同一原件、不得复用前缀。
        cache::clear_handles();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 0, "另一份证书对象不得冒充同一原件");
        assert_eq!(graph_digest(&conn), digest);
        assert_eq!(snapshot_here(&files), cold_snapshot, "查询结果必须一致");
        cache::configure(268_435_456);
    }

    /// #1394：合法包装只改变原件根指针，保留公开 bytes、槽和所有旧节点的可达性。
    fn wrap_root_with_single_ref(conn: &Connection, key: &str) {
        use sha2::{Digest, Sha256};
        let (node, _, _, _) = evidence_store::Reader::root_row(conn, key)
            .unwrap()
            .unwrap();
        let length: i64 = conn
            .query_row(
                "SELECT expanded_length FROM s_evidence_nodes WHERE node_id=?1",
                params![node.as_slice()],
                |r| r.get(0),
            )
            .unwrap();
        let mut payload = vec![1u8]; // TAG_REF
        payload.extend_from_slice(&node);
        let wrapped: [u8; 32] = Sha256::digest([&[1u8][..], &payload].concat()).into();
        conn.execute(
            "INSERT INTO s_evidence_nodes(node_id,node_kind,payload,expanded_length) VALUES(?1,1,?2,?3)",
            params![wrapped.as_slice(), payload, length],
        )
        .unwrap();
        conn.execute(
            "UPDATE s_evidence_roots SET node_id=?1 WHERE root_key=?2",
            params![wrapped.as_slice(), key],
        )
        .unwrap();
    }

    fn assert_wrapped_root_rechecks_prefix(kind: &str, graph2: bool) {
        let files = TestFiles::new();
        build_oracle(&files, 3);
        let conn = open_db(&files.db()).unwrap();
        if graph2 {
            downgrade_to_graph2(&conn);
        }
        cache::configure(1 << 30);
        cache::clear_handles();
        let key: String = conn
            .query_row(
                "SELECT root_key FROM s_evidence_roots WHERE root_key LIKE ?1 ORDER BY root_key LIMIT 1",
                params![format!("{kind}/%")],
                |r| r.get(0),
            )
            .unwrap();
        let original = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET)
            .read_root(&key, false)
            .unwrap();
        let snapshot = snapshot_here(&files);
        verify_reachable_root(&conn).unwrap();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 3, "变更前须已复用暖前缀");

        wrap_root_with_single_ref(&conn, &key);
        // 先验证包装是合法的内容等价原件，且确实旋转了根证明。
        let wrapped = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET)
            .read_root(&key, false)
            .unwrap();
        assert_eq!(wrapped.bytes, original.bytes);
        assert!(!wrapped.originals_verified && !wrapped.reused);
        assert!(!std::sync::Arc::ptr_eq(
            original.root_cert.as_ref().unwrap(),
            wrapped.root_cert.as_ref().unwrap()
        ));
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 0, "{key} 原件变化必须重核旧前缀");
        assert_eq!(snapshot_here(&files), snapshot);
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 3, "下一次未改原件须恢复暖前缀复用");
        cache::configure(268_435_456);
    }

    #[test]
    fn graph3_batch_original_wrapper_rechecks_prefix() {
        assert_wrapped_root_rechecks_prefix("batch", false);
    }

    #[test]
    fn graph3_fact_original_wrapper_rechecks_prefix() {
        assert_wrapped_root_rechecks_prefix("fact", false);
    }

    #[test]
    fn graph2_batch_original_wrapper_rechecks_prefix() {
        assert_wrapped_root_rechecks_prefix("batch", true);
    }

    /// #1394 第二段：`evidence-graph/2`（只有 batch 走私有图）继续**只读**兼容；本 writer 明确
    /// 拒绝在 v2 域写入（不原地迁移、不降级猜测），且混代根（v2 名下出现 delta/fact 根）一律拒绝。
    #[test]
    fn graph2_domain_stays_read_only_and_mixed_roots_are_rejected() {
        // ① 真正的 v2 库：读全部通过，写一律 SchemaUnsupported。
        let files = TestFiles::new();
        build_oracle(&files, 3);
        {
            let conn = open_db(&files.db()).unwrap();
            verify_reachable_root(&conn).unwrap();
            downgrade_to_graph2(&conn);
            assert_eq!(
                evidence_store::format(&conn).unwrap(),
                evidence_store::Format::GraphV2
            );
            verify_reachable_root(&conn).unwrap();
        }
        cmd_snapshot(&files.db(), None).unwrap();
        cmd_catalog(&files.db(), None).unwrap();
        cmd_watch(&files.db(), 1).unwrap();
        // 三个正式写入入口（accept / advance / recover）都在**第一笔写入之前**按写域门拒绝。
        for refusal in [
            cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH).unwrap_err(),
            cmd_recover(&files.db(), "2").unwrap_err(),
        ] {
            assert!(
                refusal.contains("SchemaUnsupported")
                    && refusal.contains(evidence_store::GRAPH_FORMAT_V2),
                "{refusal}"
            );
        }
        let accept_refusal = {
            let input = located_ohlc_input(&tb02b_oracle_rows()[..1]);
            let path = write_input(&files, "v2-accept.json", &input);
            cmd_accept(
                &files.db(),
                &path,
                &fixture("profiles/ohlc_integer_tb02a_v1.json"),
                DEFAULT_WRITER_EPOCH,
            )
            .unwrap_err()
        };
        assert!(
            accept_refusal.contains("SchemaUnsupported"),
            "{accept_refusal}"
        );
        // ② 混代：v3 产物把登记行改成 v2，delta/fact 根仍在 → 读写都必须拒绝。
        let mixed = TestFiles::new();
        build_oracle(&mixed, 3);
        {
            let conn = open_db(&mixed.db()).unwrap();
            conn.execute(
                "UPDATE s_evidence_format SET graph_format=?1",
                params![evidence_store::GRAPH_FORMAT_V2],
            )
            .unwrap();
            let err = verify_reachable_root(&conn).unwrap_err();
            assert!(err.contains("未声明"), "{err}");
        }
        assert!(cmd_snapshot(&mixed.db(), None).is_err());
        assert!(cmd_advance(&mixed.db(), DEFAULT_WRITER_EPOCH).is_err());
    }

    /// #1394 第二段：Delta 原件与结构事实三列原件的根/节点/行任一变化，都在**任何写入之前**拒绝，
    /// 且拒绝前后库摘要逐字节不变（热缓存与冷缓存都执行）。
    #[test]
    fn graph3_delta_and_fact_damage_blocks_every_write_before_it_happens() {
        for (sql, warm) in [
            (
                "INSERT INTO s_evidence_roots SELECT 'delta/01',node_id,slot_ids,expanded_length,content_sha256 FROM s_evidence_roots WHERE root_key='delta/1'",
                true,
            ),
            (
                "INSERT INTO s_evidence_roots SELECT 'delta/+1',node_id,slot_ids,expanded_length,content_sha256 FROM s_evidence_roots WHERE root_key='delta/1'",
                false,
            ),
            ("UPDATE structure_deltas SET delta_json='{}' WHERE generation=1", true),
            ("DELETE FROM structure_deltas WHERE generation=1", true),
            ("UPDATE structure_deltas SET catalog_evidence_json='{}' WHERE generation=1", true),
            ("UPDATE structure_deltas SET input_frontier=99 WHERE generation=1", true),
            (
                "UPDATE structure_facts SET payload_json='{}' WHERE object_id=(SELECT object_id FROM structure_facts ORDER BY object_id LIMIT 1)",
                true,
            ),
            (
                "UPDATE structure_facts SET input_refs_json='[]' WHERE object_id=(SELECT object_id FROM structure_facts ORDER BY object_id LIMIT 1)",
                true,
            ),
            (
                "UPDATE structure_facts SET first_known_generation=first_known_generation+1 WHERE object_id=(SELECT object_id FROM structure_facts ORDER BY object_id LIMIT 1)",
                true,
            ),
            (
                "DELETE FROM structure_facts WHERE object_id=(SELECT object_id FROM structure_facts ORDER BY object_id LIMIT 1)",
                true,
            ),
            (
                "DELETE FROM s_evidence_nodes WHERE node_id=(SELECT node_id FROM s_evidence_roots WHERE root_key LIKE 'delta/%' ORDER BY root_key LIMIT 1)",
                true,
            ),
            (
                "DELETE FROM s_evidence_nodes WHERE node_id=(SELECT node_id FROM s_evidence_roots WHERE root_key LIKE 'fact/%' ORDER BY root_key LIMIT 1)",
                false,
            ),
            (
                "DELETE FROM s_evidence_roots WHERE root_key=(SELECT root_key FROM s_evidence_roots WHERE root_key LIKE 'delta/%' ORDER BY root_key LIMIT 1)",
                true,
            ),
            (
                "DELETE FROM s_evidence_roots WHERE root_key=(SELECT root_key FROM s_evidence_roots WHERE root_key LIKE 'fact/%' ORDER BY root_key LIMIT 1)",
                false,
            ),
            (
                "UPDATE s_evidence_roots SET expanded_length=expanded_length+1 WHERE root_key=(SELECT root_key FROM s_evidence_roots WHERE root_key LIKE 'fact/%' ORDER BY root_key LIMIT 1)",
                true,
            ),
            (
                "UPDATE s_evidence_roots SET slot_ids=x'' WHERE root_key=(SELECT root_key FROM s_evidence_roots WHERE root_key LIKE 'delta/%' ORDER BY root_key LIMIT 1)",
                true,
            ),
            (
                "UPDATE s_evidence_roots SET content_sha256='0'||substr(content_sha256,2) WHERE root_key=(SELECT root_key FROM s_evidence_roots WHERE root_key LIKE 'delta/%' ORDER BY root_key LIMIT 1)",
                true,
            ),
            (
                "UPDATE s_evidence_nodes SET node_kind=7 WHERE node_id=(SELECT node_id FROM s_evidence_roots WHERE root_key LIKE 'fact/%' ORDER BY root_key LIMIT 1)",
                true,
            ),
        ] {
            cache::configure(268435456);
            let files = TestFiles::new();
            build_oracle(&files, 3);
            let healthy = open_db(&files.db()).unwrap();
            if warm {
                verify_reachable_root(&healthy).unwrap();
                verify_reachable_root(&healthy).unwrap();
            }
            let other = open_db(&files.db()).unwrap();
            other.execute_batch(sql).unwrap();
            drop(other);
            let before = graph_digest(&healthy);
            let error = verify_reachable_root(&healthy).unwrap_err();
            assert!(error.contains("StorageUnavailable"), "{sql} -> {error}");
            for refused in [
                cmd_snapshot(&files.db(), None),
                cmd_advance(&files.db(), DEFAULT_WRITER_EPOCH),
                cmd_recover(&files.db(), "2"),
            ] {
                assert!(refused.is_err(), "{sql}");
            }
            assert_eq!(graph_digest(&healthy), before, "拒绝前不得留下任何写入：{sql}");
            cache::configure(0);
        }
    }

    /// #1394 第二段：结构事实的同 ID 重复写只有**不可变内容逐字节一致**才幂等；异内容直接
    /// `IdentityConflict`（不得 `ON CONFLICT DO NOTHING` 吞掉、留待将来的读审计发现）。
    #[test]
    fn graph3_fact_same_id_different_content_is_refused_at_publish() {
        let files = TestFiles::new();
        build_oracle(&files, 2);
        let conn = open_db(&files.db()).unwrap();
        let (active, _) = tb02a::read_facts(&conn, None).unwrap();
        assert!(!active.is_empty());
        let fact = active[0].clone();
        let object_id = fact["object_id"].as_str().unwrap().to_string();
        let batch = fact["batch_id"].as_str().unwrap().to_string();
        // ① 同内容重复写：幂等。
        tb02a::save_fact(&conn, &fact, &batch).unwrap();
        assert_eq!(
            tb02a::read_facts(&conn, None).unwrap().0.len(),
            active.len()
        );
        // ② 同 ID 异内容：发布入口直接拒绝。
        let payload = fact["payload_json"].as_str().unwrap().to_string();
        let mut parsed: Value = serde_json::from_str(&payload).unwrap();
        parsed["payload_injected"] = json!("x");
        let mut tampered = fact.clone();
        tampered["payload_json"] = json!(canonical_json(&parsed));
        let error = tb02a::save_fact(&conn, &tampered, &batch).unwrap_err();
        assert!(error.contains("IdentityConflict"), "{error}");
        // ③ 既有根的指针必须与本次原件不同（同 ID 异内容不可能同根）。
        let staged = evidence_store::stage_complete(
            &evidence_store::fact_root_value(
                tampered["payload_json"].as_str().unwrap(),
                tampered["input_refs_json"].as_str().unwrap(),
                tampered["source_coords_json"].as_str().unwrap(),
            ),
            &[],
        )
        .unwrap();
        let row =
            evidence_store::Reader::root_row(&conn, &evidence_store::fact_root_key(&object_id))
                .unwrap()
                .unwrap();
        assert_ne!(row.0, staged.root, "异内容不可能得到同一根指针");
        // 事实行仍逐字段不变，且读取路径不受影响。
        let (after, _) = tb02a::read_facts(&conn, None).unwrap();
        assert_eq!(after[0], fact);
        verify_reachable_root(&conn).unwrap();
    }

    #[test]
    fn graph_cold_audit_reads_each_published_batch_once() {
        let files = TestFiles::new();
        build_oracle(&files, 3);
        let conn = open_db(&files.db()).unwrap();
        for limit in [0, 268435456] {
            cache::configure(limit);
            cache::counters_reset();
            verify_reachable_root(&conn).unwrap();
            assert_eq!(
                cache::counters_snapshot().batch_full_reads,
                3,
                "cold limit={limit}: 每已发布代只处理一份完整批次"
            );
            assert_eq!(cache::counters_snapshot().batch_suffix_full_reads, 3);
            assert_eq!(cache::last_resumed(), 0);
            if limit != 0 {
                cache::evict_full_handles_for_test();
                cache::counters_reset();
                verify_reachable_root(&conn).unwrap();
                assert_eq!(
                    cache::last_resumed(),
                    3,
                    "冷核得到的新证明须进入最终审计前缀"
                );
                assert_eq!(cache::counters_snapshot().batch_full_reads, 0);
                // 小头单独丢失时，原前缀证明仍可成立，但不能跳过这些 PendingFull 的控制核。
                cache::evict_batch_headers_for_test();
                cache::counters_reset();
                verify_reachable_root(&conn).unwrap();
                assert_eq!(cache::last_resumed(), 3);
                assert_eq!(cache::counters_snapshot().batch_full_reads, 3);
                assert_eq!(cache::counters_snapshot().batch_suffix_full_reads, 0);
                assert_eq!(cache::counters_snapshot().batch_header_full_fallbacks, 3);
            }
        }
        cache::configure(0);
    }

    #[test]
    fn batch_header_preserves_legacy_fields_and_strict_full_parse() {
        cache::configure(0);
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        for field in [
            Value::Null,
            json!(3),
            json!([]),
            json!({}),
            json!("unchanged"),
        ] {
            let value = json!({"protocol_revision":field,"session_generation":field,
                "clock_plan_hash":field,"semantic_commit_ns":field});
            let bytes = canonical_json(&value).into_bytes();
            let id = format!("batch-{}", sha256_hex(&bytes));
            conn.execute(
                "INSERT INTO batches(batch_id,canonical_bytes,byte_len) VALUES(?1,?2,?3)",
                params![id, bytes, bytes.len() as i64],
            )
            .unwrap();
            let (batch, _) = load_batch_value_with(
                &conn,
                &mut evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET),
                &id,
            )
            .unwrap();
            let header = BatchControlHeader::from_verified_batch(&batch);
            assert_eq!(
                header.protocol_revision.as_deref().map(|v| v.as_str()),
                field.as_str()
            );
            assert_eq!(load_batch_bytes(&conn, &id).unwrap().as_ref(), bytes);
        }
        let duplicate = br#"{"protocol_revision":"s-session/2","nested":{"x":1,"x":2}}"#;
        let id = format!("batch-{}", sha256_hex(duplicate));
        conn.execute(
            "INSERT INTO batches(batch_id,canonical_bytes,byte_len) VALUES(?1,?2,?3)",
            params![id, duplicate.as_slice(), duplicate.len() as i64],
        )
        .unwrap();
        assert!(load_batch_value_with(
            &conn,
            &mut evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET),
            &id
        )
        .is_err());
        assert_eq!(cache::retained_bytes(), 0);
    }

    #[test]
    fn batch_header_never_promotes_before_public_identity_validation() {
        cache::configure(1 << 20);
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        evidence_store::declare_new(&conn).unwrap();
        let value = json!({"protocol_revision":"s-session/2","session_generation":"1",
            "clock_plan_hash":"hash","semantic_commit_ns":"20"});
        let staged = evidence_store::stage_complete(&value, &[]).unwrap();
        let wrong_id = format!("batch-{}", "0".repeat(64));
        evidence_store::publish_refs(&conn, &evidence_store::batch_root_key(&wrong_id), &staged)
            .unwrap();
        conn.execute(
            "INSERT INTO batches(batch_id,canonical_bytes,byte_len) VALUES(?1,x'',?2)",
            params![wrong_id, canonical_json(&value).len() as i64],
        )
        .unwrap();
        for _ in 0..2 {
            assert!(load_batch_value_with(
                &conn,
                &mut evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET),
                &wrong_id
            )
            .is_err());
            assert_eq!(cache::batch_header_bytes(), 0);
        }
        cache::configure(0);
    }

    #[test]
    fn graph_unpublished_batch_still_requires_its_full_public_hash() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        evidence_store::declare_new(&conn).unwrap();
        let value = json!({"phase": "staged"});
        let staged = evidence_store::stage_complete(&value, &[]).unwrap();
        let id = format!("batch-{}", sha256_hex(&canonical_bytes_of_value(&value)));
        conn.execute(
            "INSERT INTO batches VALUES(?1,x'',?2)",
            params![id, staged.expanded_length as i64],
        )
        .unwrap();
        evidence_store::publish_refs(&conn, &evidence_store::batch_root_key(&id), &staged).unwrap();
        assert!(capture_reachable_batches(&conn).unwrap().batches.is_empty());
        // 尚未发布的合法暂存根允许存在，但批次身份仍须绑定完整字节。
        let bad = format!("batch-{}", "0".repeat(64));
        conn.execute("UPDATE batches SET batch_id=?1", params![bad])
            .unwrap();
        conn.execute(
            "UPDATE s_evidence_roots SET root_key=?1",
            params![evidence_store::batch_root_key(&bad)],
        )
        .unwrap();
        assert!(
            capture_reachable_batches(&conn).is_err(),
            "私有根摘要正确不能代替公开批次身份核验"
        );
    }
}

#[cfg(test)]
mod record_borrow_regressions {
    use super::*;

    // #1394：从 b8d423be 原样保留旧判据，仅更名；不调用新的规范化/比较帮助函数。
    fn old_same_records(actual: &Value, expected: &Value, field: &str) -> Result<(), String> {
        let normalize = |v: &Value| -> Result<Vec<String>, String> {
            let mut records = v
                .as_array()
                .ok_or("StorageUnavailable：集合非数组")?
                .iter()
                .map(|r| {
                    if !r.is_object() {
                        return Err("StorageUnavailable：集合成员非对象".to_string());
                    }
                    Ok(canonical_json(&legacy_payload_projection(r)))
                })
                .collect::<Result<Vec<_>, String>>()?;
            records.sort();
            if records.windows(2).any(|w| w[0] == w[1]) {
                return Err(format!("StorageUnavailable：{field} 重复成员"));
            }
            Ok(records)
        };
        if normalize(actual)? != normalize(expected)? {
            return Err(format!(
                "StorageUnavailable：{field} 与封存批次/同 cut 持久快照不一致"
            ));
        }
        Ok(())
    }

    fn rejected(message: &str) -> Result<(), String> {
        Err(message.to_string())
    }

    fn check_case(
        name: &str,
        actual_json: &str,
        expected_json: &str,
        field: &str,
        wanted: Result<(), String>,
    ) {
        // from_str 保留 -0.0、0.0、1.0 的浮点表示。
        let actual: Value = serde_json::from_str(actual_json).unwrap();
        let expected: Value = serde_json::from_str(expected_json).unwrap();
        let oracle = old_same_records(&actual, &expected, field);
        assert_eq!(oracle, wanted, "旧判据明确预期：{name}");
        assert_eq!(
            same_records(&actual, &expected, field),
            oracle,
            "数组入口：{name}"
        );
        if let Some(expected) = expected.as_array() {
            // 旧 Audit 经 Vec<&Value> + json! 物化；必须保留浮点规范字节。
            let materialized_expected = json!(expected.iter().collect::<Vec<&Value>>());
            let materialized_oracle = old_same_records(&actual, &materialized_expected, field);
            assert_eq!(materialized_oracle, oracle, "旧 expected 物化路径：{name}");
            assert_eq!(
                same_records_against_iter(&actual, expected.iter(), field),
                materialized_oracle,
                "借用 expected：{name}"
            );
            if let Some(actual) = actual.as_array() {
                let materialized_actual = json!(actual.iter().collect::<Vec<&Value>>());
                let materialized_oracle =
                    old_same_records(&materialized_actual, &materialized_expected, field);
                assert_eq!(materialized_oracle, oracle, "旧双侧物化路径：{name}");
                assert_eq!(
                    same_record_iters(actual.iter(), expected.iter(), field),
                    materialized_oracle,
                    "双侧借用：{name}"
                );
            }
        }
    }

    #[test]
    fn borrowed_records_preserve_complete_canonical_set_semantics() {
        let mismatch = || rejected("StorageUnavailable：records 与封存批次/同 cut 持久快照不一致");
        let duplicate = || rejected("StorageUnavailable：records 重复成员");
        for (name, actual, expected, wanted) in [
            ("空集合", "[]", "[]", Ok(())),
            (
                "集合排列与对象键序",
                r#"[{"id":"b","v":2},{"id":"a","v":1}]"#,
                r#"[{"v":1,"id":"a"},{"v":2,"id":"b"}]"#,
                Ok(()),
            ),
            (
                "actual 完整重复",
                r#"[{"id":"a"},{"id":"a"}]"#,
                r#"[{"id":"a"}]"#,
                duplicate(),
            ),
            (
                "expected 完整重复",
                r#"[{"id":"a"}]"#,
                r#"[{"id":"a"},{"id":"a"}]"#,
                duplicate(),
            ),
            (
                "detail 递归整数投影",
                r#"[{"id":"a","detail":{"n":[1,{"v":2}]}}]"#,
                r#"[{"id":"a","detail":{"n":["1",{"v":"2"}]}}]"#,
                Ok(()),
            ),
            (
                "detail 投影后 actual 重复",
                r#"[{"detail":{"n":1}},{"detail":{"n":"1"}}]"#,
                r#"[{"detail":{"n":"1"}}]"#,
                duplicate(),
            ),
            (
                "detail 投影后 expected 重复",
                r#"[{"detail":{"n":"1"}}]"#,
                r#"[{"detail":{"n":1}},{"detail":{"n":"1"}}]"#,
                duplicate(),
            ),
            (
                "raw_bars 修订投影",
                r#"[{"raw_bars":[{"supersedes_revision":2,"seq":7}]}]"#,
                r#"[{"raw_bars":[{"supersedes_revision":"2","seq":7}]}]"#,
                Ok(()),
            ),
            (
                "raw_bars 投影后重复",
                r#"[{"raw_bars":[{"supersedes_revision":2}]},{"raw_bars":[{"supersedes_revision":"2"}]}]"#,
                r#"[{"raw_bars":[{"supersedes_revision":"2"}]}]"#,
                duplicate(),
            ),
            (
                "raw_bars 其它整数不投影",
                r#"[{"raw_bars":[{"supersedes_revision":2,"seq":7}]}]"#,
                r#"[{"raw_bars":[{"supersedes_revision":"2","seq":"7"}]}]"#,
                mismatch(),
            ),
            (
                "嵌套数组顺序",
                r#"[{"id":"a","nested":[{"v":1},{"v":2}]}]"#,
                r#"[{"id":"a","nested":[{"v":2},{"v":1}]}]"#,
                mismatch(),
            ),
            (
                "正负浮点零有别",
                r#"[{"v":-0.0}]"#,
                r#"[{"v":0.0}]"#,
                mismatch(),
            ),
            (
                "正负浮点零不去重",
                r#"[{"v":-0.0},{"v":0.0}]"#,
                r#"[{"v":0.0},{"v":-0.0}]"#,
                Ok(()),
            ),
            (
                "整数与浮点有别",
                r#"[{"v":1}]"#,
                r#"[{"v":1.0}]"#,
                mismatch(),
            ),
            (
                "整数与浮点不去重",
                r#"[{"v":1},{"v":1.0}]"#,
                r#"[{"v":1.0},{"v":1}]"#,
                Ok(()),
            ),
            (
                "detail 浮点不转整数 wire",
                r#"[{"detail":{"v":1.0}}]"#,
                r#"[{"detail":{"v":"1"}}]"#,
                mismatch(),
            ),
            (
                "同 identity 异内容不一致",
                r#"[{"id":"a","v":1}]"#,
                r#"[{"id":"a","v":2}]"#,
                mismatch(),
            ),
            (
                "同 identity 异内容不算重复",
                r#"[{"id":"a","v":1},{"id":"a","v":2}]"#,
                r#"[{"id":"a","v":2},{"id":"a","v":1}]"#,
                Ok(()),
            ),
        ] {
            check_case(name, actual, expected, "records", wanted);
        }
    }

    #[test]
    fn borrowed_records_preserve_actual_first_error_priority() {
        let array = || rejected("StorageUnavailable：集合非数组");
        let member = || rejected("StorageUnavailable：集合成员非对象");
        let duplicate = || rejected("StorageUnavailable：priority 重复成员");
        for (name, actual, expected, wanted) in [
            ("双侧非数组", "null", "false", array()),
            (
                "actual 非数组先于 expected 坏成员",
                "null",
                "[false]",
                array(),
            ),
            (
                "actual 非数组先于 expected 重复",
                "null",
                "[{},{}]",
                array(),
            ),
            (
                "actual 坏成员先于 expected 非数组",
                "[false]",
                "null",
                member(),
            ),
            ("双侧坏成员", "[false]", "[null]", member()),
            (
                "actual 坏成员先于 expected 重复",
                "[false]",
                "[{},{}]",
                member(),
            ),
            (
                "actual 重复先于 expected 非数组",
                "[{},{}]",
                "null",
                duplicate(),
            ),
            (
                "actual 重复先于 expected 坏成员",
                "[{},{}]",
                "[false]",
                duplicate(),
            ),
            ("双侧重复", "[{},{}]", "[{},{}]", duplicate()),
            ("actual 扫完成员才查重复", "[{},{},false]", "null", member()),
            ("expected 非数组先于集合差异", "[]", "null", array()),
            ("expected 坏成员先于集合差异", "[]", "[false]", member()),
            ("expected 重复先于集合差异", "[]", "[{},{}]", duplicate()),
            ("expected 扫完成员才查重复", "[]", "[{},{},false]", member()),
        ] {
            check_case(name, actual, expected, "priority", wanted);
        }
    }

    #[test]
    fn normalized_batch_reuse_matches_oracle_after_successful_index_comparison() {
        let index: Value = serde_json::from_str(r#"[
            {"id":"a","detail":{"count":"1"},"raw_bars":[{"supersedes_revision":"2","seq":7}],"nested":[0.0,-0.0]},
            {"id":"b","detail":{"count":"2"},"raw_bars":[],"nested":[1,1.0]}
        ]"#).unwrap();
        let batch: Value = serde_json::from_str(r#"[
            {"id":"b","detail":{"count":2},"raw_bars":[],"nested":[1,1.0]},
            {"id":"a","detail":{"count":1},"raw_bars":[{"supersedes_revision":2,"seq":7}],"nested":[0.0,-0.0]}
        ]"#).unwrap();
        for key in ["objects", "witnesses", "relations", "observations"] {
            let initial_field = format!("index.{key}");
            assert_eq!(old_same_records(&index, &batch, &initial_field), Ok(()));
            let actual = normalize_record_array(&index, &initial_field).unwrap();
            let expected =
                normalize_record_iter(batch.as_array().unwrap().iter(), &initial_field).unwrap();
            assert_eq!(
                compare_normalized_records(&actual, &expected, &initial_field),
                Ok(())
            );
            // 初次 index/batch 完整比较成功后，才把已排序、查重的全文向量用于后续比较。
            assert_eq!(expected, vec![
                r#"{"detail":{"count":"1"},"id":"a","nested":[0.0,-0.0],"raw_bars":[{"seq":7,"supersedes_revision":"2"}]}"#.to_string(),
                r#"{"detail":{"count":"2"},"id":"b","nested":[1,1.0],"raw_bars":[]}"#.to_string(),
            ]);
            let mut changed = index.clone();
            changed[0]["detail"]["count"] = json!("99");
            for (name, actual, error) in [
                ("delta.upserts", index.clone(), None),
                ("full.upserts", batch.clone(), None),
                ("delta.changed", changed, Some("mismatch")),
                ("full.missing", json!([index[0]]), Some("mismatch")),
                (
                    "delta.duplicate",
                    json!([index[0], index[0]]),
                    Some("duplicate"),
                ),
                ("full.member", json!([false]), Some("member")),
                ("delta.array", Value::Null, Some("array")),
            ] {
                let field = format!("{name}.{key}");
                let wanted = match error {
                    None => Ok(()),
                    Some("mismatch") => Err(format!(
                        "StorageUnavailable：{field} 与封存批次/同 cut 持久快照不一致"
                    )),
                    Some("duplicate") => Err(format!("StorageUnavailable：{field} 重复成员")),
                    Some("member") => rejected("StorageUnavailable：集合成员非对象"),
                    Some("array") => rejected("StorageUnavailable：集合非数组"),
                    _ => unreachable!(),
                };
                let oracle = old_same_records(&actual, &batch, &field);
                assert_eq!(oracle, wanted, "旧判据明确预期：{field}");
                let reused = normalize_record_array(&actual, &field)
                    .and_then(|actual| compare_normalized_records(&actual, &expected, &field));
                assert_eq!(reused, oracle, "复用 batch 全文：{field}");
            }
        }
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod legacy_profile_owner_regressions {
    use super::*;
    use newchan_rust::s_content::capture_allocation::{audit_json as aj, OwnedBudget, LIMIT};

    fn old_profile_fixture() -> (Connection, String) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE batches(batch_id TEXT PRIMARY KEY,canonical_bytes BLOB NOT NULL,byte_len INTEGER NOT NULL); CREATE TABLE structure_deltas(generation INTEGER,index_frontier TEXT); CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE raw_events(seq INTEGER)").unwrap();
        let hash = "a".repeat(64);
        for (key, value) in [
            ("profile_id", "fixed"),
            ("profile_hash", hash.as_str()),
            ("input_profile", "fixed"),
        ] {
            conn.execute("INSERT INTO meta VALUES(?1,?2)", params![key, value])
                .unwrap();
        }
        (conn, hash)
    }
    fn seal_blob(conn: &Connection, bytes: &[u8]) -> String {
        let id = format!("batch-{}", sha256_hex(bytes));
        conn.execute(
            "INSERT INTO batches VALUES(?1,?2,?3)",
            params![id, bytes, bytes.len() as i64],
        )
        .unwrap();
        conn.execute("INSERT INTO structure_deltas VALUES(1,?1)", params![id])
            .unwrap();
        id
    }
    fn seal_graph(conn: &Connection, generation: i64, value: &Value) {
        let bytes = canonical_bytes_of_value(value);
        let id = format!("batch-{}", sha256_hex(&bytes));
        let staged =
            evidence_store::stage_complete(value, &[evidence_store::SlotSpec::number("9")])
                .unwrap();
        evidence_store::publish_refs(conn, &evidence_store::batch_root_key(&id), &staged).unwrap();
        conn.execute(
            "INSERT INTO batches VALUES(?1,?2,?3)",
            params![id, Vec::<u8>::new(), bytes.len() as i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO structure_deltas VALUES(?1,?2)",
            params![generation, id],
        )
        .unwrap();
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r13_native_legacy_profile_blob_matches_none_on_parent_scope() {
        use v3::audit_store::{CodecRead, NativeAdmission};
        cache::configure(0);
        // 既有旧版本存档夹具从建库起就没有 profile_definition；没有删改元数据造分支。
        let (conn, hash) = old_profile_fixture();
        let value = json!({"profile_id":"fixed","profile_hash":hash,"detail":{"text":"完整旧批次","values":[1,2,3]}});
        let expected = canonical_bytes_of_value(&value);
        let id = seal_blob(&conn, &expected);
        let plain_binding = verify_profile_binding(&conn).unwrap();
        let mut plain_reader = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET);
        let plain = load_profile_batch_bytes(&conn, &mut plain_reader, &id, None).unwrap();
        assert_eq!(&plain[..], &expected);
        drop(plain_reader);
        let budget = v3::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                NativeAdmission(Some(scope))
                    .native_admit(137)
                    .map_err(|e| e.to_string())?;
                let before = budget.live();
                let mut reader =
                    evidence_store::Reader::new_admitted(&conn, EVIDENCE_READ_BUDGET, Some(scope))?;
                let bytes = load_profile_batch_bytes(&conn, &mut reader, &id, None)?;
                assert!(matches!(&bytes, ProfileBatchBytes::Plain(_)));
                assert_eq!(&bytes[..], &plain[..]);
                assert!(evidence_store::hash_matches(
                    &bytes,
                    batch_content_hash(&id)?
                ));
                assert_eq!(bytes.len(), expected.len());
                drop(reader);
                let binding = verify_profile_binding_admitted(&conn, None, Some(scope))?;
                assert!(matches!(&binding.0, v3::business_json::TextValue::Plain(_)));
                assert_eq!(binding.0.as_str(), plain_binding.0);
                assert_eq!(binding.1.as_str(), plain_binding.1);
                assert!(budget.live() > before);
                Ok::<_, String>(())
            })
            .unwrap();
        assert!(budget.live() > 137, "Native许可留在调用方原父域");
        assert_eq!(budget.scratch_observation().2, 0);
        assert_eq!(cache::retained_bytes(), 0);
        cache::configure(0);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r13_native_legacy_profile_graph_roots_match_none_and_keep_cache_gates() {
        use v3::audit_store::{CodecRead, NativeAdmission};
        for graph_format in [
            evidence_store::GRAPH_FORMAT,
            evidence_store::GRAPH_FORMAT_V2,
        ] {
            for cache_limit in [0, 1024 * 1024] {
                cache::configure(cache_limit);
                let (conn, hash) = old_profile_fixture();
                evidence_store::declare_new(&conn).unwrap();
                seal_graph(&conn, 1, &json!({"profile_id":"","v":9}));
                let repeated = json!({"v":9,"text":"正常封存证据".repeat(40)});
                let value = json!({"profile_id":"fixed","profile_hash":hash,"a":repeated,"b":repeated,
                    "protocol_revision":"s-session/2","session_generation":"2","clock_plan_hash":"clock","semantic_commit_ns":"9"});
                seal_graph(&conn, 2, &value);
                // 这份存档只含 batch 根，符合 graph/2 的已发布布局；声明变体不改变任何原件或元数据。
                if graph_format == evidence_store::GRAPH_FORMAT_V2 {
                    conn.execute(
                        "UPDATE s_evidence_format SET graph_format=?1",
                        params![graph_format],
                    )
                    .unwrap();
                }
                let expected = canonical_bytes_of_value(&value);
                let id = format!("batch-{}", sha256_hex(&expected));
                let key = evidence_store::batch_root_key(&id);
                let plain_binding = verify_profile_binding(&conn).unwrap();
                let mut plain_reader = evidence_store::Reader::new(&conn, EVIDENCE_READ_BUDGET);
                let plain = plain_reader.read_root(&key, true).unwrap();
                assert_eq!(plain.bytes.as_ref(), &expected);
                assert_eq!(plain.declared_length as usize, expected.len());
                assert_eq!(plain.content_sha256, batch_content_hash(&id).unwrap());
                assert_eq!(plain.value.as_deref().map(|v| v.as_ref()), Some(&value));
                drop(plain_reader);
                cache::configure(cache_limit);
                let budget = v3::audit_owner::native::Budget::new(16 * 1024 * 1024, 0).unwrap();
                let tx = conn.unchecked_transaction().unwrap();
                budget
                    .codec_work()
                    .unwrap()
                    .run(|scope| {
                        NativeAdmission(Some(scope))
                            .native_admit(137)
                            .map_err(|e| e.to_string())?;
                        let before = budget.live();
                        let mut reader = evidence_store::Reader::new_admitted(
                            &tx,
                            EVIDENCE_READ_BUDGET,
                            Some(scope),
                        )?;
                        for pass in 0..2 {
                            let root = reader.read_root(&key, true)?;
                            assert_eq!(root.bytes.as_ref(), plain.bytes.as_ref());
                            assert_eq!(root.declared_length, plain.declared_length);
                            assert_eq!(root.content_sha256, plain.content_sha256);
                            assert_eq!(root.closure, plain.closure);
                            assert_eq!(root.value.as_deref().map(|v| v.as_ref()), Some(&value));
                            assert_eq!(root.reused, cache_limit > 0 && pass == 1);
                        }
                        // 第二次命中含已解析Value，仍会真实投影控制头并走其缓存准入。
                        let bytes = load_profile_batch_bytes(&tx, &mut reader, &id, None)?;
                        assert!(matches!(&bytes, ProfileBatchBytes::Plain(_)));
                        assert_eq!(&bytes[..], &expected);
                        drop(reader);
                        let binding = verify_profile_binding_admitted(&tx, None, Some(scope))?;
                        assert_eq!(binding.0.as_str(), plain_binding.0);
                        assert_eq!(binding.1.as_str(), plain_binding.1);
                        assert!(budget.live() > before);
                        Ok::<_, String>(())
                    })
                    .unwrap();
                assert!(budget.live() > 137);
                assert_eq!(budget.scratch_observation().2, 0);
                if cache_limit == 0 {
                    assert_eq!(cache::retained_bytes(), 0);
                    assert_eq!(cache::capture_bytes_now(), 0);
                } else {
                    assert!(cache::retained_bytes() > 0);
                    assert!(cache::retained_bytes() <= cache_limit);
                }
                drop(tx);
                cache::configure(0);
            }
        }
    }

    #[test]
    fn owned_legacy_profile_fallback_blob_keeps_duplicate_policy_and_bytes_decode_peak() {
        let (conn, hash) = old_profile_fixture();
        let blob =
            format!(r#"{{"profile_id":"old","profile_id":"fixed","profile_hash":"{hash}"}}"#);
        let id = seal_blob(&conn, blob.as_bytes());
        let budget = OwnedBudget::new(1 << 20);
        let binding = verify_profile_binding_with_owner(&conn, Some(&budget)).unwrap();
        assert_eq!(binding.0.as_str(), "fixed");
        assert_eq!(binding.1.as_str(), hash);
        assert!(matches!(binding.0, v3::business_json::TextValue::Owned(_)));
        drop(binding);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let mut reader = evidence_store::Reader::with_owner(&conn, &budget);
        let bytes = load_profile_batch_bytes(&conn, &mut reader, &id, Some(&budget)).unwrap();
        assert_eq!(&bytes[..], blob.as_bytes());
        let byte_live = budget.snapshot().live_requested_layout_bytes;
        let json = aj::OwnedJson::parse_legacy_shape(&bytes, &budget).unwrap();
        assert_eq!(
            json.view().get("profile_id").and_then(|v| v.as_str()),
            Some("fixed")
        );
        assert!(budget.snapshot().live_requested_layout_bytes > byte_live);
        drop(json);
        drop(bytes);
        drop(reader);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_legacy_profile_fallback_graph_checks_slots_skips_empty_and_refuses_budget() {
        let (conn, hash) = old_profile_fixture();
        evidence_store::declare_new(&conn).unwrap();
        seal_graph(&conn, 1, &json!({"profile_id":"","v":9}));
        seal_graph(
            &conn,
            2,
            &json!({"profile_id":"fixed","profile_hash":hash,"a":{"v":9},"b":{"v":9}}),
        );
        let budget = OwnedBudget::new(1 << 20);
        let binding = verify_profile_binding_with_owner(&conn, Some(&budget)).unwrap();
        assert_eq!(binding.0.as_str(), "fixed");
        assert_eq!(binding.1.as_str(), hash);
        drop(binding);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let low = OwnedBudget::new(budget.snapshot().peak_requested_layout_bytes - 1);
        let error = verify_profile_binding_with_owner(&conn, Some(&low))
            .err()
            .unwrap();
        assert!(error.starts_with(LIMIT), "{error}");
        assert!(low.snapshot().rejected_layouts > 0);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_legacy_profile_fallback_keeps_bad_source_first_error_and_definition_gate() {
        let (conn, hash) = old_profile_fixture();
        let bytes = canonical_bytes_of_value(&json!({"profile_id":"fixed","profile_hash":hash}));
        seal_blob(&conn, &bytes);
        conn.execute("UPDATE batches SET byte_len=-1", []).unwrap();
        let plain = verify_profile_binding_with_owner(&conn, None)
            .err()
            .unwrap();
        let budget = OwnedBudget::new(1 << 20);
        let owned = verify_profile_binding_with_owner(&conn, Some(&budget))
            .err()
            .unwrap();
        assert_eq!(plain, owned);
        assert!(owned.contains("byte_len 非负越界"));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        conn.execute(
            "UPDATE batches SET byte_len=?1",
            params![bytes.len() as i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO meta VALUES('profile_definition','{broken')",
            [],
        )
        .unwrap();
        assert!(verify_profile_binding_with_owner(&conn, Some(&budget)).is_err());
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
