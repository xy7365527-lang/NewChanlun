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

use std::collections::BTreeMap;

#[path = "s_session_v2/audit.rs"]
mod audit;
#[path = "s_session_v2/cache.rs"]
mod cache;
use std::path::{Path, PathBuf};

use newchan_rust::theta_v0::classifier::local_shape::{
    classify_local_shape_sliding, window_domain_violation, Cc006DomainViolation, Cc006LocalShape,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{self, ParseLayerIncr};
use newchan_rust::theta_v0::types::{Bar, Direction};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// CC-006 验收合同修订（SPEC-COVERAGE-INPUT.json `/classification_axes/5`）。
const RULE_REVISION: &str = "s2-axis-quantifiers";

/// S 唯一结构写者的默认 writer_epoch（可经 `--writer-epoch` / `recover` 换代）。
const DEFAULT_WRITER_EPOCH: &str = "1";

fn jstr(s: &str) -> String {
    Value::String(s.to_string()).to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
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

/// 规范十进制整数：`0` 或 `-?[1-9][0-9]*`（无 `+`、无空白、无前导零、`-0` 非规范）。
fn is_canonical_integer(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    let (neg, rest) = if b[0] == b'-' {
        (true, &b[1..])
    } else {
        (false, b)
    };
    if rest.is_empty() {
        return false;
    }
    if rest[0] == b'0' {
        return rest.len() == 1 && !neg;
    }
    rest.iter().all(|c| c.is_ascii_digit())
}

/// 把规范十进制字符串解析为精确 i64（非规范或超范围报错，不静默截断）。
fn parse_canonical_i64(s: &str, what: &str) -> Result<i64, String> {
    if !is_canonical_integer(s) {
        return Err(format!(
            "{what} `{s}` 不是规范十进制整数（要求唯一表示：无 +、无空白、无前导零）"
        ));
    }
    s.parse::<i64>()
        .map_err(|e| format!("{what} `{s}` 超出 i64 精确整数域：{e}"))
}

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
    let conn = Connection::open(db).map_err(|e| format!("打开 SQLite 失败：{e}"))?;
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
/// 核单个不可变 batch 的规范内容寻址完整性：batch_id = "batch-" + sha256(canonical_bytes)，
/// byte_len == canonical_bytes.len()。缺失/坏字节/坏长度均 StorageUnavailable。
fn verify_stored_batch(conn: &Connection, batch_id: &str) -> Result<(), String> {
    let row: Option<(Vec<u8>, i64)> = conn
        .query_row(
            "SELECT canonical_bytes, byte_len FROM batches WHERE batch_id=?1",
            params![batch_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("读 batch {batch_id} 失败：{e}"))?;
    let (bytes, byte_len) = row.ok_or_else(|| {
        format!("StorageUnavailable：可达 batch `{batch_id}` 不存在（引用不完整，拒绝写入）")
    })?;
    let expect_hash = match batch_id.strip_prefix("batch-") {
        Some(h) if h.len() == 64 => h,
        _ => {
            return Err(format!(
                "StorageUnavailable：batch_id `{batch_id}` 非规范内容寻址形态"
            ))
        }
    };
    let actual_hash = sha256_hex(&bytes);
    if actual_hash != expect_hash || usize::try_from(byte_len).ok() != Some(bytes.len()) {
        return Err(format!(
            "StorageUnavailable：batch `{batch_id}` 规范字节/长度与内容寻址不符（坏 bytes/byte_len），拒绝写入"
        ));
    }
    Ok(())
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

fn same_records(actual: &Value, expected: &Value, field: &str) -> Result<(), String> {
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
    let id = meta_get_opt(conn, "profile_id")?;
    let hash = meta_get_opt(conn, "profile_hash")?;
    let input = meta_get_opt(conn, "input_profile")?;
    let definition = meta_get_opt(conn, "profile_definition")?;
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if id.is_none() && hash.is_none() && input.is_none() && definition.is_none() && raw_count == 0 {
        return Ok((String::new(), String::new()));
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
        let profile = json_shape(&definition, JsonShape::Object)?;
        if profile["profile_id"].as_str() != Some(id.as_str())
            || sha256_hex(&canonical_bytes_of_value(&profile)) != hash
        {
            return Err("StorageUnavailable：profile定义与固定id/hash不一致".to_string());
        }
    } else {
        // 旧版本没有profile_definition：只接受独立已封存来源；没有批次的旧TestOnly接纳
        // 以同一声明档案核规范hash。这是旧格式取证，不在失败后换宽松判据。
        let sealed: Option<(String,Vec<u8>)> = conn.query_row(
            "SELECT b.batch_id,b.canonical_bytes FROM batches b JOIN structure_deltas d ON d.index_frontier=b.batch_id WHERE json_extract(b.canonical_bytes,'$.profile_id')<>'' ORDER BY d.generation LIMIT 1",
            [],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        if let Some((batch_id, bytes)) = sealed {
            verify_stored_batch(conn, &batch_id)?;
            let batch: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if batch["profile_id"].as_str() != Some(id.as_str())
                || batch["profile_hash"].as_str() != Some(hash.as_str())
            {
                return Err("StorageUnavailable：旧profile绑定与可达封存批次不一致".to_string());
            }
        } else {
            let declared: Value = serde_json::from_str(include_str!(
                "../../../s_session/profiles/testonly_tick_1_1_ohlc.json"
            ))
            .map_err(|e| e.to_string())?;
            if declared["profile_id"].as_str() != Some(id.as_str())
                || sha256_hex(&canonical_bytes_of_value(&declared)) != hash
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
    let bytes: Vec<u8>=conn.query_row("SELECT b.canonical_bytes FROM batches b JOIN structure_deltas d ON d.index_frontier=b.batch_id WHERE d.generation=?1",params![generation],|r|r.get(0)).map_err(|e|format!("StorageUnavailable：cut来源不存在：{e}"))?;
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

// 每次读取同一事务的全部可达原字节；只复用逐字节相同原件的严格解码。
fn capture_reachable_batches(
    conn: &Connection,
) -> Result<BTreeMap<String, std::sync::Arc<Value>>, String> {
    let mut q=conn.prepare("SELECT d.index_frontier,b.canonical_bytes,b.byte_len FROM structure_deltas d LEFT JOIN batches b ON b.batch_id=d.index_frontier").map_err(|e|e.to_string())?;
    let mut rows = q.query([]).map_err(|e| e.to_string())?;
    let mut batches = BTreeMap::new();
    let mut captured = BTreeMap::new();
    while let Some(r) = rows.next().map_err(|e| e.to_string())? {
        let id: String = r.get(0).map_err(|e| e.to_string())?;
        let bytes = r
            .get_ref(1)
            .map_err(|e| e.to_string())?
            .as_blob()
            .map_err(|e| format!("StorageUnavailable：可达批次缺失/非BLOB：{e}"))?;
        let len: i64 = r.get(2).map_err(|e| e.to_string())?;
        if usize::try_from(len).ok() != Some(bytes.len()) {
            return Err("StorageUnavailable：可达 batch 长度不符".into());
        }
        let (original, value) = match cache::decoded(&id, bytes) {
            Some(value) => value,
            None => {
                if id != format!("batch-{}", sha256_hex(&bytes)) {
                    return Err("StorageUnavailable：可达 batch 内容寻址不符".into());
                }
                let value = v2::strict_json(&bytes)
                    .map_err(|e| format!("StorageUnavailable：batch JSON：{e}"))?;
                if !value.is_object() {
                    return Err("StorageUnavailable：batch 非对象".into());
                }
                (
                    std::sync::Arc::<[u8]>::from(bytes),
                    std::sync::Arc::new(value),
                )
            }
        };
        captured.insert(id.clone(), (original, value.clone()));
        batches.insert(id, value);
    }
    cache::store_batches(captured);
    Ok(batches)
}

fn verify_reachable_root(conn: &Connection) -> Result<(), String> {
    verified_root_raw_events(conn).map(|_| ())
}

fn verified_root_raw_events(conn: &Connection) -> Result<Vec<Value>, String> {
    let batches = capture_reachable_batches(conn)?;
    v2::verify_control(conn, &batches)?;
    verify_reachable_root_in_tx(conn, &batches).map_err(|e| format!("StorageUnavailable：{e}"))
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
    batches: &BTreeMap<String, std::sync::Arc<Value>>,
) -> Result<Vec<Value>, String> {
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

    if generation == 0 {
        if !rows.is_empty() {
            return Err(
                "StorageUnavailable：初态 generation=0 却存在持久 Delta（不一致）".to_string(),
            );
        }
        return Ok(raw_events);
    }

    if rows.len() != generation as usize {
        return Err(format!(
            "StorageUnavailable：已发布代链断裂（generation={generation}，仅 {} 行 Delta）",
            rows.len()
        ));
    }

    let mut audit = audit::AuditWalk::capture(conn)?;
    let identity = (
        meta_session.clone(),
        meta_cat.clone(),
        profile_binding.clone(),
    );
    let resume = cache::resume(&rows, &identity, &mut audit);
    let mut prev_frontier: i64 = if resume == 0 { -1 } else { rows[resume - 1].7 };
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
        ),
    ) in rows.iter().enumerate().skip(resume)
    {
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
        let batch = batches.get(idx).ok_or("StorageUnavailable：缺可达批次")?;
        verify_batch_coordinates(batch, idx, *g, next, *frontier, cat)?;
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
        let dj: Value = json_shape(delta_json, JsonShape::Object)?;
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
        verify_batch_profile(batch, &profile_binding)?;
        audit.verify(&dj, *g, batch)?;
    }
    audit.finish()?;
    // meta 与最末已发布 Delta 的根元组对应关系。
    let last = rows.last().unwrap();
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
    cache::store_root(rows, identity, audit);
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

fn load_profile(path: &Path) -> Result<(Value, String, String), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 profile 失败：{e}"))?;
    let profile: Value =
        serde_json::from_str(&text).map_err(|e| format!("profile JSON 解析失败：{e}"))?;
    let profile_id = profile
        .get("profile_id")
        .and_then(Value::as_str)
        .ok_or("profile 缺 profile_id")?
        .to_string();
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
    let input: RawInputFile =
        serde_json::from_str(&text).map_err(|e| format!("输入 JSON 解析失败：{e}"))?;
    if input.schema_revision != "1" {
        return Err(format!(
            "SchemaUnsupported：schema_revision={}",
            input.schema_revision
        ));
    }
    let (profile, profile_id, profile_hash) = load_profile(profile_path)?;
    if input.profile != profile_id {
        return Err(format!(
            "InvalidDomain：输入 profile=`{}` 与 --profile 的 profile_id=`{profile_id}` 不一致",
            input.profile
        ));
    }
    let volume_unit = profile
        .get("volume_unit")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
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
    let mut results = Vec::new();
    {
        verify_writer_epoch(&tx, configured_epoch)?;
        verify_reachable_root(&tx)?;

        let bound_profile_id = meta_get_opt(&tx, "profile_id")?.unwrap_or_default();
        if bound_profile_id.is_empty() {
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

        let mut qrev = tx
            .prepare("SELECT receipt_id, payload_hash, source_coord, seq FROM raw_events WHERE identity_key=?1 AND revision=?2")
            .map_err(|e| format!("prepare 查询失败：{e}"))?;
        let mut qmax = tx
            .prepare("SELECT COALESCE(MAX(revision), 0) FROM raw_events WHERE identity_key=?1")
            .map_err(|e| format!("prepare 查询失败：{e}"))?;
        let mut insert = tx
            .prepare(
                "INSERT INTO raw_events(identity_key, revision, input_revision, payload_hash, receipt_id, seq,
                   source_namespace, source_epoch, instrument, event_id, received_at,
                   raw_text, price, ts, volume, source_coord, supersedes_revision)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            )
            .map_err(|e| format!("prepare insert 失败：{e}"))?;

        let mut next_seq: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读 max seq 失败：{e}"))?;

        for e in &input.events {
            parse_source_coord(&e.seq).map_err(|e| format!("InvalidDomain：{e}"))?;
            parse_canonical_i64(&e.price, "price")?;
            parse_canonical_i64(&e.timestamp, "timestamp")?;
            parse_canonical_i64(&e.volume, "volume")?;
            let rev = parse_canonical_i64(&e.revision, "revision")?;
            if let Some(vu) = &volume_unit {
                if e.volume != *vu {
                    return Err(format!(
                        "InvalidDomain：profile volume_unit=`{vu}`，事件 `{}` volume=`{}` 不在单位成交量域（本片不擅自改成交量类型）",
                        e.event_id, e.volume
                    ));
                }
            }
            let ikey = identity_key(
                &input.source_namespace,
                &input.source_epoch,
                &input.instrument,
                &e.event_id,
            );
            // 同一计算序的坐标只能归一个业务身份；该身份的历史 revision 可共享坐标。
            let occupied: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM raw_events WHERE source_coord=?1 AND identity_key<>?2)",
                    params![e.seq, ikey],
                    |row| row.get(0),
                )
                .map_err(|e| format!("查询源坐标归属失败：{e}"))?;
            if occupied {
                return Err(format!(
                    "IdentityConflict：source_coord `{}` 已属于另一业务身份（不覆盖原始事件或见证）",
                    e.seq
                ));
            }
            let content = canonical_event_content(e);
            let payload_hash = sha256_hex(content.as_bytes());

            let existing = qrev
                .query_row(params![ikey, rev], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                    ))
                })
                .optional()
                .map_err(|e| format!("查询 identity 失败：{e}"))?;

            if let Some((rec, ph, coord, seq)) = existing {
                if ph == payload_hash {
                    results.push(json!({
                        "status": "replay",
                        "identity_key": ikey,
                        "event_id": e.event_id,
                        "revision": rev.to_string(),
                        "receipt_id": rec,
                        "payload_hash": payload_hash,
                        "seq": seq.to_string(),
                        "source_coord": coord,
                    }));
                } else {
                    results.push(json!({
                        "status": "IdentityConflict",
                        "identity_key": ikey,
                        "event_id": e.event_id,
                        "revision": rev.to_string(),
                        "existing_receipt_id": rec,
                        "existing_payload_hash": ph,
                        "new_payload_hash": payload_hash,
                    }));
                }
                continue;
            }

            // 该 (identity, revision) 未见过。
            let max_rev: i64 = qmax
                .query_row(params![ikey], |r| r.get(0))
                .map_err(|e| format!("读 max revision 失败：{e}"))?;
            if rev <= max_rev {
                // 旧版（已有更高 revision）且该 revision 无记录：晚到/跳号旧版不复活、不新接纳。
                results.push(json!({
                    "status": "InvalidDomain",
                    "identity_key": ikey,
                    "event_id": e.event_id,
                    "revision": rev.to_string(),
                    "max_revision": max_rev.to_string(),
                    "reason": "out_of_order_revision_after_later_accepted",
                }));
                continue;
            }
            // 新 revision：supersedes = 上一 revision（如有）。
            let supersedes = if max_rev == 0 { None } else { Some(max_rev) };
            // 源坐标一致性：同一业务身份的新 revision 必须落在同一源坐标。
            if let Some(prev_rev) = supersedes {
                let prev_coord: String = tx
                    .query_row(
                        "SELECT source_coord FROM raw_events WHERE identity_key=?1 AND revision=?2",
                        params![ikey, prev_rev],
                        |r| r.get(0),
                    )
                    .map_err(|e| format!("读前一 revision 源坐标失败：{e}"))?;
                if prev_coord != e.seq {
                    return Err(format!(
                        "IdentityConflict：事件 `{}` 新 revision=`{rev}` 源坐标 `{}` 与已接纳 revision=`{prev_rev}` 源坐标 `{prev_coord}` 不一致（同一业务身份不得换源位置）",
                        e.event_id, e.seq
                    ));
                }
            }

            next_seq += 1;
            let receipt_id = format!(
                "rcpt-{}",
                &sha256_hex(format!("{ikey}|{payload_hash}").as_bytes())[..16]
            );
            insert
                .execute(params![
                    ikey,
                    rev,
                    e.revision,
                    payload_hash,
                    receipt_id,
                    next_seq,
                    input.source_namespace,
                    input.source_epoch,
                    input.instrument,
                    e.event_id,
                    e.received_at,
                    e.raw_text,
                    e.price,
                    e.timestamp,
                    e.volume,
                    e.seq,
                    supersedes,
                ])
                .map_err(|e| format!("insert raw_events 失败：{e}"))?;
            results.push(json!({
                "status": "accepted",
                "identity_key": ikey,
                "event_id": e.event_id,
                "revision": rev.to_string(),
                "receipt_id": receipt_id,
                "payload_hash": payload_hash,
                "seq": next_seq.to_string(),
                "source_coord": e.seq,
                "supersedes_revision": supersedes.map(|v| v.to_string()),
            }));
        }

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
    let mut stmt = conn
        .prepare(
            "SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace,
                    source_epoch, instrument, event_id, received_at, raw_text, price,
                    ts, volume, source_coord, supersedes_revision
             FROM raw_events ORDER BY seq ASC",
        )
        .map_err(|e| format!("prepare 读取失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| {
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
        })
        .map_err(|e| format!("query 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect 失败：{e}"))?;
    let mut owners = BTreeMap::new();
    let mut positions = BTreeMap::new();
    let mut prior_revisions = BTreeMap::new();
    for (expected_seq, event) in rows.iter().enumerate() {
        let coord = stored_source_coord(event)?;
        let identity = event["identity_key"]
            .as_str()
            .ok_or("StorageUnavailable：raw.identity_key 不是字符串")?;
        if owners
            .insert(coord, identity)
            .is_some_and(|old| old != identity)
            || positions
                .insert(identity, coord)
                .is_some_and(|old| old != coord)
        {
            return Err("StorageUnavailable：原始源坐标与业务身份的唯一归属被破坏".to_string());
        }
        let raw = RawEvent {
            event_id: event["event_id"]
                .as_str()
                .ok_or("StorageUnavailable：event_id非文本")?
                .to_owned(),
            revision: event["input_revision"]
                .as_str()
                .ok_or("StorageUnavailable：input_revision非文本")?
                .to_owned(),
            seq: event["source_coord"].as_str().unwrap().to_owned(),
            received_at: event["received_at"]
                .as_str()
                .ok_or("StorageUnavailable：received_at非文本")?
                .to_owned(),
            raw_text: event["raw_text"]
                .as_str()
                .ok_or("StorageUnavailable：raw_text非文本")?
                .to_owned(),
            price: event["price"]
                .as_str()
                .ok_or("StorageUnavailable：price非文本")?
                .to_owned(),
            timestamp: event["ts"]
                .as_str()
                .ok_or("StorageUnavailable：ts非文本")?
                .to_owned(),
            volume: event["volume"]
                .as_str()
                .ok_or("StorageUnavailable：volume非文本")?
                .to_owned(),
        };
        let revision = parse_canonical_i64(&raw.revision, "input_revision")
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let previous = prior_revisions.insert(identity, revision);
        let expected_identity = identity_key(
            event["source_namespace"]
                .as_str()
                .ok_or("StorageUnavailable：namespace非文本")?,
            event["source_epoch"]
                .as_str()
                .ok_or("StorageUnavailable：source_epoch非文本")?,
            event["instrument"]
                .as_str()
                .ok_or("StorageUnavailable：instrument非文本")?,
            &raw.event_id,
        );
        let payload_hash = sha256_hex(canonical_event_content(&raw).as_bytes());
        let receipt = format!(
            "rcpt-{}",
            &sha256_hex(format!("{identity}|{payload_hash}").as_bytes())[..16]
        );
        if event["seq"].as_i64() != i64::try_from(expected_seq).ok()
            || revision < 1
            || event["revision"].as_i64() != Some(revision)
            || previous.is_some_and(|p| p >= revision)
            || event["supersedes_revision"] != json!(previous)
            || identity != expected_identity
            || event["payload_hash"] != payload_hash
            || event["receipt_id"] != receipt
        {
            return Err(
                "StorageUnavailable：原始身份/接纳序/修订链/内容hash/receipt矛盾".to_owned(),
            );
        }
        for (name, value) in [
            ("price", &raw.price),
            ("timestamp", &raw.timestamp),
            ("volume", &raw.volume),
        ] {
            parse_canonical_i64(value, name).map_err(|e| format!("StorageUnavailable：{e}"))?;
        }
    }
    Ok(rows)
}

/// 有效源位置：每个业务身份取最新 revision，按源坐标升序（晚到新版不当作追加 tick）。
fn read_effective_events(all: &[Value]) -> Result<Vec<Value>, String> {
    let mut latest: BTreeMap<String, Value> = BTreeMap::new();
    for ev in all {
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
        "source_coords_json": canonical_json(&json!(source_coords)),
        "first_known_generation": gen,
        "first_known_cut": structure_cut,
        "published_generation": gen,
    });
    (object, witnesses, relations)
}

/// 读当前活动对象（withdrawn_generation IS NULL），供 diff 使用（在提交事务内调用）。
fn read_active_objects(conn: &Connection) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT object_id, branch, window_start, window_mid, window_end, first_known_generation, first_known_cut, batch_id, published_generation FROM objects WHERE withdrawn_generation IS NULL ORDER BY window_start",
        )
        .map_err(|e| format!("prepare 读活动对象失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(json!({
                "object_id": r.get::<_, String>(0)?,
                "branch": r.get::<_, String>(1)?,
                "window_start": r.get::<_, i64>(2)?,
                "window_mid": r.get::<_, i64>(3)?,
                "window_end": r.get::<_, i64>(4)?,
                "first_known_generation": r.get::<_, i64>(5)?,
                "first_known_cut": r.get::<_, String>(6)?,
                "batch_id": r.get::<_, String>(7)?,
                "published_generation": r.get::<_, i64>(8)?,
            }))
        })
        .map_err(|e| format!("query 活动对象失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect 活动对象失败：{e}"))?;
    Ok(rows)
}

/// 新对象集与既有活动对象对拍，产出 (upserts, withdrawals, replaces)。同一发生区间同一内容 = 无变化；
/// 同一发生区间内容变 = 撤旧 + 替代；新区间 = upsert；旧区间消失 = 撤（防御性，append-only 域不触发）。
fn compute_diff(
    new_objects: &[Value],
    existing_active: &[Value],
) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut upserts: Vec<Value> = Vec::new();
    let mut withdrawals: Vec<Value> = Vec::new();
    let mut replaces: Vec<Value> = Vec::new();

    // 区间 → 新对象（同一发生区间在合法域内唯一）。
    let mut new_by_window: BTreeMap<(i64, i64, i64), &Value> = BTreeMap::new();
    for o in new_objects {
        let w = (
            o["window_start"].as_i64().unwrap_or(0),
            o["window_mid"].as_i64().unwrap_or(0),
            o["window_end"].as_i64().unwrap_or(0),
        );
        new_by_window.insert(w, o);
    }

    for old in existing_active {
        let w = (
            old["window_start"].as_i64().unwrap_or(0),
            old["window_mid"].as_i64().unwrap_or(0),
            old["window_end"].as_i64().unwrap_or(0),
        );
        match new_by_window.get(&w) {
            None => {
                withdrawals.push(json!({
                    "object_id": old["object_id"],
                    "window_start": w.0,
                    "window_mid": w.1,
                    "window_end": w.2,
                    "reason": "window_removed",
                    "superseded_by": Value::Null,
                }));
            }
            Some(new) => {
                if new["object_id"].as_str() != old["object_id"].as_str() {
                    withdrawals.push(json!({
                        "object_id": old["object_id"],
                        "window_start": w.0,
                        "window_mid": w.1,
                        "window_end": w.2,
                        "reason": "superseded_by_revision",
                        "superseded_by": new["object_id"].clone(),
                    }));
                    replaces.push(json!({
                        "new_object_id": new["object_id"],
                        "old_object_id": old["object_id"],
                    }));
                }
            }
        }
    }

    // upsert = 全部新对象（含无变化者，供同 cut 对拍与幂等重投）。
    for o in new_objects {
        upserts.push(o.clone());
    }
    (upserts, withdrawals, replaces)
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
    let (stored, byte_len): (Vec<u8>, i64) = conn
        .query_row(
            "SELECT canonical_bytes, byte_len FROM batches WHERE batch_id=?1",
            params![batch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("完整不可变批次不可读：{e}"))?;
    if stored != bytes || usize::try_from(byte_len).ok() != Some(bytes.len()) {
        return Err(
            "IdentityConflict：不可变 batch 的规范字节或长度不符，拒绝覆写和发布".to_string(),
        );
    }
    Ok(())
}

/// C09.4 / AC4：完整对象先独立持久；此事务不发布任何对象表或结构根。
fn persist_batch_before_publish(
    conn: &mut Connection,
    token: &str,
    gen: i64,
    frontier: i64,
    batch_id: &str,
    bytes: &[u8],
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
    tx.execute(
        "INSERT OR IGNORE INTO batches(batch_id, canonical_bytes, byte_len) VALUES(?1, ?2, ?3)",
        params![batch_id, bytes, byte_len],
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
    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
    record_connection_pragmas(&conn, "advance");
    // ── Begin：判定前先由短写事务核 writer_epoch、取得唯一推进权、持久 Begin/门/输入前沿 ──
    let (token, gen, frontier, base_cut, profile_id, profile_hash, catalog_revision, all_events) = {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Begin 事务失败：{e}"))?;
        verify_writer_epoch(&tx, configured_epoch)?;
        let all_events = verified_root_raw_events(&tx)?;
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
        meta_set(
            &tx,
            "advance_state",
            &format!("begun:{token}:{gen}:{frontier}"),
        )?;
        meta_set(&tx, "begin_generation", &gen.to_string())?;
        meta_set(&tx, "begin_input_frontier", &frontier.to_string())?;
        meta_set(&tx, "begin_token", &token)?;
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
    let mut incr = ParseLayerIncr::new(&config);
    let mut layer = None;
    let mut raw_bars: Vec<Bar> = Vec::new();
    for ev in &effective {
        let px = parse_canonical_i64(ev["price"].as_str().unwrap_or(""), "price")?;
        let t = parse_canonical_i64(ev["ts"].as_str().unwrap_or(""), "timestamp")?;
        let coord = stored_source_coord(ev)?;
        // 源坐标即本根在该源流中的位置；1:1 退化 OHLC 逐点即标准 K。
        let bar = Bar {
            source_index: usize::try_from(coord)
                .map_err(|_| "StorageUnavailable：源坐标不能无损转换为 source_index")?,
            timestamp: t,
            open: px,
            high: px,
            low: px,
            close: px,
            volume: 1.0,
            untradable: false,
        };
        raw_bars.push(bar);
        layer = Some(incr.append(bar));
    }
    let layer = layer.unwrap_or_else(|| parser::parse_layer(&[], &config));
    let merged_rc = layer.merged_bars.clone();
    let merged: &[Bar] = merged_rc.as_ref();

    let wins = classify_local_shape_sliding(merged);

    // source_coord → merged 组号（inclusion 同源映射；1:1 域各根一组）。
    let mut merged_raws: Vec<Vec<i64>> = vec![Vec::new(); merged.len()];
    for coord in raw_by_coord.keys().copied() {
        let source_index = usize::try_from(coord)
            .map_err(|_| "StorageUnavailable：源坐标不能无损转换为 source_index")?;
        if let Some(g) = parser::inclusion::merged_group_index(merged, source_index) {
            merged_raws[g].push(coord);
        }
    }

    let mut objects: Vec<Value> = Vec::new();
    let mut witnesses: Vec<Value> = Vec::new();
    let mut relations: Vec<Value> = Vec::new();
    let mut observations: Vec<Value> = Vec::new();

    let mut raw_violations: Vec<(usize, Cc006DomainViolation)> = Vec::new();
    for i in 0..raw_bars.len().saturating_sub(2) {
        if let Some(reason) =
            window_domain_violation(&raw_bars[i], &raw_bars[i + 1], &raw_bars[i + 2])
        {
            raw_violations.push((i, reason));
        }
    }

    if raw_bars.len() < 3 {
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
                        json!({ "merged_index": mi, "raw_refs": refs })
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

    // 去重：同一 (kind, window, reason) 只发布一次。
    {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        observations.retain(|ob| {
            let key = format!(
                "{}|{:?}|{:?}|{:?}|{}",
                ob["kind"], ob["window_start"], ob["window_mid"], ob["window_end"], ob["reason"]
            );
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
    v2::seal_batch(&conn, context, &mut batch_json)?;
    let batch_bytes = canonical_bytes_of_value(&batch_json);
    let batch_id = format!("batch-{}", sha256_hex(&batch_bytes));
    for o in &mut objects {
        o["batch_id"] = json!(batch_id);
    }

    persist_batch_before_publish(
        &mut conn,
        &token,
        gen,
        frontier,
        &batch_id,
        &batch_bytes,
        configured_epoch,
    )?;

    // ★测试暂停点 2：完整不可变批次已独立持久、Commit 尚未发生（供 SIGKILL）。
    testonly_pause("after_batch");

    // ── Commit：核 Begin/epoch/前沿仍有效后，同一事务原子应用对象/边/端点 + 撤回/替代 + delta ──
    let classified_count = objects.len();
    let mut frontier_moved = false;
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
            // 与既有活动对象对拍（提交事务内，原子）。
            let existing_active = read_active_objects(&tx)?;
            let (_upserts, withdrawals, replaces) = compute_diff(&objects, &existing_active);
            // 未变化对象沿用既有 first_known（不把重发布当新获知）；新对象 first_known = 本 gen。
            let existing_by_id: BTreeMap<String, Value> = existing_active
                .iter()
                .map(|o| (o["object_id"].as_str().unwrap_or("").to_string(), o.clone()))
                .collect();
            let mut delta_upserts: Vec<Value> = Vec::new();
            for o in &objects {
                let oid = o["object_id"].as_str().unwrap_or("").to_string();
                let mut obj = o.clone();
                if let Some(ex) = existing_by_id.get(&oid) {
                    // 未变对象沿用既有 first_known / batch / published（首发布即历史稳定值，不因重发布前移）。
                    obj["first_known_generation"] = ex["first_known_generation"].clone();
                    obj["first_known_cut"] = ex["first_known_cut"].clone();
                    obj["batch_id"] = ex["batch_id"].clone();
                    obj["published_generation"] = ex["published_generation"].clone();
                }
                delta_upserts.push(obj);
            }

            // 1) 撤回：当前视图移除，历史不删（UPDATE 生命周期，保留原身份/first_known/发生区间）。
            {
                let mut stmt = tx
                    .prepare(
                        "UPDATE objects SET withdrawn_generation=?1, withdrawal_reason=?2, superseded_by=?3 WHERE object_id=?4 AND withdrawn_generation IS NULL",
                    )
                    .map_err(|e| format!("prepare 撤回失败：{e}"))?;
                for w in &withdrawals {
                    let superseded = w["superseded_by"].as_str().map(|s| s.to_string());
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
                for o in &objects {
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
            {
                let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO witnesses(witness_id, object_id, slot, merged_source_index, merged_high,
                       merged_low, merged_open, merged_close, raw_json, published_generation)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(|e| format!("prepare witnesses 失败：{e}"))?;
                for w in &witnesses {
                    stmt.execute(params![
                        w["witness_id"].as_str().unwrap_or(""),
                        w["object_id"].as_str().unwrap_or(""),
                        w["slot"].as_i64().unwrap_or(0),
                        w["merged_source_index"].as_i64().unwrap_or(0),
                        w["merged_high"].as_str().unwrap_or(""),
                        w["merged_low"].as_str().unwrap_or(""),
                        w["merged_open"].as_str().unwrap_or(""),
                        w["merged_close"].as_str().unwrap_or(""),
                        serde_json::to_string(&w["raw_bars"]).unwrap_or_else(|_| "[]".to_string()),
                        gen,
                    ])
                    .map_err(|e| format!("insert witnesses 失败：{e}"))?;
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
                for r in &rels {
                    let rid = format!(
                        "rel-{}",
                        &sha256_hex(&canonical_bytes_of_value(&json!({
                            "s": r["subject"],
                            "t": r["relation_type"],
                            "o": r["object"],
                        })))[..16]
                    );
                    stmt.execute(params![
                        rid,
                        r["subject"].as_str().unwrap_or(""),
                        r["relation_type"].as_str().unwrap_or(""),
                        r["object"].as_str().unwrap_or(""),
                        gen,
                    ])
                    .map_err(|e| format!("insert relations 失败：{e}"))?;
                }
            }
            // 观察：补 identity/batch，供 delta 与同 cut Snapshot 逐字段对齐（INSERT OR IGNORE：首获知固定）。
            let mut obs_with_id: Vec<Value> = Vec::with_capacity(observations.len());
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
                let o = if let Some(stored) = stored {
                    stored
                } else {
                    let mut fresh = ob.clone();
                    fresh["observation_id"] = json!(oid);
                    fresh["batch_id"] = json!(batch_id);
                    fresh
                };
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
            let evidence = json!({
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
            // wire 形态：整数坐标/版本 → 规范十进制字符串，与同 cut Snapshot 逐字段对齐。
            let wire_upserts: Vec<Value> = delta_upserts
                .iter()
                .map(project_object_wire)
                .collect::<Result<_, _>>()?;
            let wire_withdrawals: Vec<Value> = withdrawals
                .iter()
                .map(project_withdrawal_wire)
                .collect::<Result<_, _>>()?;
            let wire_witnesses: Vec<Value> = witnesses
                .iter()
                .map(project_witness_wire)
                .collect::<Result<_, _>>()?;
            let wire_observations: Vec<Value> = obs_with_id
                .iter()
                .map(project_observation_wire)
                .collect::<Result<_, _>>()?;
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
                    canonical_json(&delta_json),
                ],
            )
            .map_err(|e| format!("持久 delta 失败：{e}"))?;

            // 5) 索引可达根：generation / structure_cut / index_frontier（与对象同事务生效）。
            meta_set(&tx, "generation", &gen.to_string())?;
            meta_set(&tx, "structure_cut", &structure_cut)?;
            meta_set(&tx, "index_frontier", &batch_id)?;
            meta_set(&tx, "last_advance_frontier", &frontier.to_string())?;

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

            v2::record_commit(&tx, context, gen, frontier)?;
            meta_set(&tx, "advance_state", "idle")?;
        }
    }
    if frontier_moved {
        drop(tx);
    } else {
        tx.commit()
            .map_err(|e| format!("advance commit 失败：{e}"))?;
    }

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

fn canonical_json(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical_json(&map[*k])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
    }
}

fn canonical_bytes_of_value(v: &Value) -> Vec<u8> {
    canonical_json(v).into_bytes()
}

/// 最终 wire 边界的安全递归精确整数投影：JSON 整数（i64/u64）→ 规范十进制字符串；
/// bool 仍 bool、null 仍 null、浮点/非整数原值保持。用于已发布历史/Delta/Catalog 的读取输出，
/// 不改变内部存储/已发布 DB/BLOB/hash/cut/first_known。
fn project_wire_integers(v: &Value) -> Value {
    match v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::String(i.to_string())
            } else if let Some(u) = n.as_u64() {
                Value::String(u.to_string())
            } else {
                Value::Number(n.clone())
            }
        }
        Value::Array(arr) => Value::Array(arr.iter().map(project_wire_integers).collect()),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                out.insert(k.clone(), project_wire_integers(val));
            }
            Value::Object(out)
        }
        other => other.clone(),
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
        let raw_refs = obj
            .get_mut("raw_refs")
            .and_then(Value::as_array_mut)
            .ok_or("input_refs 成员必须含 raw_refs 数组")?;
        for raw in raw_refs {
            let raw = raw.as_object_mut().ok_or("raw_refs 成员必须是对象")?;
            let seq = num_to_str(raw.get("seq").unwrap_or(&Value::Null))?;
            raw.insert("seq".to_string(), seq);
            if raw.contains_key("revision") {
                let rev = num_to_str(raw.get("revision").unwrap_or(&Value::Null))?;
                raw.insert("revision".to_string(), rev);
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
        out.push(Value::Object(obj));
    }
    Ok(Value::Array(out))
}

/// i64 形态对象 → wire 形态（与 read_snapshot 投影一致）。
fn project_object_wire(o: &Value) -> Result<Value, String> {
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
    Ok(json!({
        "object_id": w["object_id"].clone(),
        "window_start": num_to_str(&w["window_start"])?,
        "window_mid": num_to_str(&w["window_mid"])?,
        "window_end": num_to_str(&w["window_end"])?,
        "reason": w["reason"].clone(),
        "superseded_by": w["superseded_by"].clone(),
    }))
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
    Ok(json!({
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
    }))
}

enum JsonShape {
    Array,
    Object,
    CatalogBranches,
}

fn json_shape(text: &str, shape: JsonShape) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| format!("持久 JSON 解析失败：{e}"))?;
    let valid = match shape {
        JsonShape::Array => value.is_array(),
        JsonShape::Object => value.is_object(),
        JsonShape::CatalogBranches => value.is_array() || value.is_object(),
    };
    if valid {
        Ok(value)
    } else {
        Err("持久 JSON 形状不符".to_string())
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
            let (implementation_status, proof_status) = if let Some(n) = as_of {
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
    let objects = match as_of {
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
    let withdrawn = match as_of {
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
    let rows = match max_seq {
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
    Ok(rows)
}

fn read_snapshot_in_tx(conn: &Connection, as_of: Option<i64>) -> Result<Value, String> {
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
    let rows: Vec<Value> = {
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

/// 读 structure_deltas 一行并投影为 wire 形式（整数坐标 → 规范十进制字符串）。
fn project_delta_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "generation": row.get::<_, i64>(0)?.to_string(),
        "session_id": row.get::<_, String>(1)?,
        "catalog_revision": row.get::<_, String>(2)?,
        "base_cut": row.get::<_, String>(3)?,
        "next_cut": row.get::<_, String>(4)?,
        "seq_range": json_column(row, 5, JsonShape::Object)?,
        "index_frontier": row.get::<_, String>(6)?,
        "input_frontier": row.get::<_, i64>(7)?.to_string(),
        "delta": json_column(row, 8, JsonShape::Object)?,
    }))
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

    let rows: Vec<Value> = {
        let mut stmt = tx
            .prepare(
                "SELECT generation, session_id, catalog_revision, base_cut, next_cut, seq_range_json, index_frontier, input_frontier, delta_json FROM structure_deltas WHERE generation > ?1 ORDER BY generation ASC",
            )
            .map_err(|e| format!("prepare watch 失败：{e}"))?;
        let mut q = stmt
            .query(params![after_generation])
            .map_err(|e| format!("query watch 失败：{e}"))?;
        let mut out: Vec<Value> = Vec::new();
        while let Some(row) = q.next().map_err(|e| format!("query watch 失败：{e}"))? {
            out.push(project_delta_row(row).map_err(|e| format!("读取 delta 失败：{e}"))?);
        }
        out
    };

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
    let new_epoch_i =
        parse_writer_epoch(new_epoch, "--new-epoch").map_err(|e| format!("InvalidDomain：{e}"))?;
    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
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
    "用法：s_structure_session <init|accept|advance|catalog|snapshot|meta|watch|recover|reset> [参数]\n\
     \x20 init      --db <路径> --session <id> --catalog <catalog.json>      （只允许不存在的目标）\n\
     \x20 accept    --db <路径> --input <输入.json> --profile <profile.json> [--writer-epoch <n>]\n\
     \x20 advance   --db <路径> [--writer-epoch <n>]\n\
     \x20 catalog   --db <路径> [--as-of <generation>]\n\
     \x20 snapshot  --db <路径> [--as-of <generation>]\n\
     \x20 meta      --db <路径>\n\
     \x20 pragma    --db <路径>\n\
     \x20 query     --db <路径> --identity-key <k>\n\
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
    if v2::dispatch(&args)? {
        return Ok(());
    }
    let _writer_lock = v2::legacy_write_lock(&args)?;
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
            remove_db_files(&d);
            cmd_init(&d, &session, &c)
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

    struct TestFiles(std::path::PathBuf);

    impl TestFiles {
        fn new() -> Self {
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

        fn db(&self) -> std::path::PathBuf {
            self.0.join("结构 ?#%.sqlite")
        }
    }

    impl Drop for TestFiles {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fixture(path: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session")
            .join(path)
    }

    fn init_test_db(files: &TestFiles) {
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

    fn write_input(files: &TestFiles, name: &str, value: &Value) -> std::path::PathBuf {
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
        let delta: String = conn
            .query_row(
                "SELECT delta_json FROM structure_deltas WHERE generation=2",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let delta: Value = serde_json::from_str(&delta).unwrap();
        assert_eq!(delta["observations"], before["observations"]);
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
                conn.execute(
                    "UPDATE structure_deltas SET delta_json=json_remove(delta_json,?1)",
                    params![format!("$.{key}")],
                )
                .unwrap();
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
        persist_batch_before_publish(&mut conn, "owned", 1, 6, &id, bytes, DEFAULT_WRITER_EPOCH)
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
        let mut stmt = conn
            .prepare("SELECT delta_json FROM structure_deltas WHERE generation > 0 ORDER BY generation ASC")
            .unwrap();
        let rows: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
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
            conn.execute(
                "UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.seq_range.from','99') WHERE generation=2",
                [],
            )
            .unwrap();
            drop(conn);
            assert_reject(&files);
        }
        // 2) 内层 catalog_run_status 改 not_run（外层仍 run）。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            conn.execute(
                "UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.catalog_run_status','not_run') WHERE generation=2",
                [],
            )
            .unwrap();
            drop(conn);
            assert_reject(&files);
        }
        // 3) 内层 catalog_evidence 改 bogus。
        {
            let files = TestFiles::new();
            build(&files);
            let conn = open_db(&files.db()).unwrap();
            conn.execute(
                "UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.catalog_evidence',json('{\"bogus\":true}')) WHERE generation=2",
                [],
            )
            .unwrap();
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
}
