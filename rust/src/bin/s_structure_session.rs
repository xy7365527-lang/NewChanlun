//! #1370 TB-01-A：正式结构会话 S（唯一结构核，独立 SQLite/WAL 单写者 + ReadCatalog/Snapshot）。
//!
//! 本二进制是 #1323 R2 SPEC 的 TB-01-A 叶片交付：真实 S 会话（结构域），从正式 launcher 启动，
//! 输入具名原始逐笔档案，用现役严格 parser（`theta_v0::parser::ParseLayerIncr`）同次真实计算
//! CC-006 `local_shape` 窗口，并在 S 自有 SQLite/WAL（synchronous=FULL）独立持久域发布；
//! `catalog`/`snapshot` 是只读查询（S.ReadCatalog / S.Snapshot）。
//!
//! ## 生产零调用纪律（AGENTS.md）
//!
//! - 本入口**真正接通** `theta_v0::parser::ParseLayerIncr::append` 与
//!   `theta_v0::classifier::local_shape::classify_local_shape`——不写第二份生产判定器。
//! - 不调用 `ThetaPiStream` 资金推进冒充 S；不启动 E/B/X；不加载经济政策。
//! - 持久域是 S 自己的数据库文件（`--db`），不复用 `trading_system/persistence/database.py` 共库。
//!
//! ## 持久边界（C09 / INTERFACE-CONTRACTS.md）
//!
//! - **Begin 在判定前持久**：`advance` 先用短写事务核 `writer_epoch`、取得唯一推进权，持久
//!   Begin/门与输入前沿；**Commit 在同一事务**核 Begin/epoch/前沿仍有效后发布对象/关系/修订/
//!   索引可达根与目录状态。读端（`catalog`/`snapshot`/只读查询外壳）只在单个读事务内读同一 cut。
//! - 内容寻址批次（`batches.canonical_bytes`）封存**完整**正式结果：原始事件（含身份/修订/收据/
//!   源坐标）、对象（含四次严格比较与 input_refs）、见证、关系、观察（知识不足/域不满足）与
//!   profile/规则绑定——不是只封存 merged OHLC 摘要。
//!
//! ## 精确整数（AC5）
//!
//! 价格/时间戳/成交量在 wire 上以**规范十进制字符串**承载（TEXT 存储 + JSON 输出），Rust 侧
//! 仅在计算边界解析为 `i64`（精确整数域），不经过 `f64`/JS Number。含 >2^53 往返反例。
//! 规范字符串要求唯一十进制表示（拒绝 `+0010000`、`" 10000 "`、前导零等非规范形式）。

use std::collections::BTreeMap;
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

/// S 唯一结构写者被授予的 writer_epoch（SPEC.md:328/332：S/E/B 写事务核持久 writer_epoch）。
const WRITER_EPOCH: &str = "1";

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
        // 只有 `0` 本身规范；`00`/`01`/`-0` 均非规范。
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

/// 业务身份键（同身份重放/冲突判据）：`(namespace, epoch, instrument, event_id)`。
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
  identity_key TEXT PRIMARY KEY,
  input_revision TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  receipt_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  source_namespace TEXT NOT NULL,
  source_epoch TEXT NOT NULL,
  instrument TEXT NOT NULL,
  event_id TEXT NOT NULL,
  revision TEXT NOT NULL,
  received_at TEXT NOT NULL,
  raw_text TEXT NOT NULL,
  price TEXT NOT NULL,
  ts TEXT NOT NULL,
  volume TEXT NOT NULL,
  source_coord TEXT NOT NULL
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
  input_refs_json TEXT NOT NULL
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
  raw_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS relations (
  relation_id TEXT PRIMARY KEY,
  subject TEXT NOT NULL,
  relation_type TEXT NOT NULL,
  object TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS observations (
  observation_id TEXT PRIMARY KEY,
  batch_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  window_start INTEGER,
  window_mid INTEGER,
  window_end INTEGER,
  reason TEXT NOT NULL,
  detail_json TEXT NOT NULL
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
    // C09.2：WAL + synchronous=FULL。macOS fullfsync 仅在 macOS 平台开启（Linux 容器不声称已测）。
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

/// 核持久 writer_epoch（DUR-H02）：所有 S 写入（AcceptInput / Advance 的 Begin+Commit）都必须在
/// 各自写事务内先过此 fence；epoch 不匹配即 StaleWriter、零写入。
fn verify_writer_epoch(conn: &Connection) -> Result<(), String> {
    let epoch = meta_get_opt(conn, "writer_epoch")?.unwrap_or_default();
    if epoch != WRITER_EPOCH {
        return Err(format!(
            "StaleWriter：writer_epoch=`{epoch}`，非当前单写者（期望 `{WRITER_EPOCH}`）"
        ));
    }
    Ok(())
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

fn cmd_init(db: &Path, session: &str, catalog_path: &Path) -> Result<(), String> {
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
    meta_set(&conn, "sqlite_version", &sqlite_version)?;
    meta_set(&conn, "journal_mode", &journal_mode)?;
    meta_set(&conn, "synchronous", &synchronous.to_string())?;
    meta_set(&conn, "platform", std::env::consts::OS)?;
    meta_set(&conn, "writer_epoch", WRITER_EPOCH)?;
    meta_set(&conn, "advance_state", "idle")?;
    meta_set(&conn, "rule_revision", RULE_REVISION)?;
    meta_set(
        &conn,
        "scope",
        &json!({"structure": "CompleteCut", "economic": "not_started"}).to_string(),
    )?;

    // 从已签目录载入 catalog（未实现条目不隐藏——全部写入 catalog 表，status 初始 not_implemented）。
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
    println!(
        "{}",
        json!({
            "ok": true,
            "session_id": session,
            "sqlite_version": sqlite_version,
            "journal_mode": journal_mode,
            "synchronous": synchronous,
            "platform": std::env::consts::OS,
            "catalog_items": items.len()
        })
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// S.AcceptInput：持久原始事件 + 同身份重放/冲突合同 + profile 绑定
// ────────────────────────────────────────────────────────────────────────────

fn cmd_accept(db: &Path, input_path: &Path, profile_path: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(input_path).map_err(|e| format!("读输入失败：{e}"))?;
    let input: RawInputFile =
        serde_json::from_str(&text).map_err(|e| format!("输入 JSON 解析失败：{e}"))?;
    if input.schema_revision != "1" {
        return Err(format!(
            "SchemaUnsupported：schema_revision={}",
            input.schema_revision
        ));
    }
    // M4：正式入口要求显式加载并绑定具名 profile；输入必须与 profile 一致。
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
        // DUR-H02：AcceptInput 的写事务受同一 writer_epoch fence 约束——任何 raw/收据/profile/meta
        // 写入前先核；epoch 不匹配即 StaleWriter、零接纳、零收据、零 profile 更新。
        verify_writer_epoch(&tx)?;

        // DUR-M01：session 的 profile 绑定在首次接纳时固定；后续同名异字节明确拒绝，不覆盖已发布
        // cut 的来源（同一 cut/index_frontier 的 Snapshot 不得因重放而被改写 profile_hash）。
        let bound_profile_id = meta_get_opt(&tx, "profile_id")?.unwrap_or_default();
        if bound_profile_id.is_empty() {
            meta_set(&tx, "profile_id", &profile_id)?;
            meta_set(&tx, "profile_hash", &profile_hash)?;
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

        let mut stmt = tx
            .prepare("SELECT receipt_id, payload_hash, seq FROM raw_events WHERE identity_key = ?1")
            .map_err(|e| format!("prepare 查询失败：{e}"))?;
        let mut insert = tx
            .prepare(
                "INSERT INTO raw_events(identity_key, input_revision, payload_hash, receipt_id, seq,
                   source_namespace, source_epoch, instrument, event_id, revision, received_at,
                   raw_text, price, ts, volume, source_coord)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            )
            .map_err(|e| format!("prepare insert 失败：{e}"))?;

        let mut next_seq: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读 max seq 失败：{e}"))?;

        for e in &input.events {
            // 精确整数校验（AC2：规范十进制字符串 + 精确整数并存，不静默截断）。
            parse_canonical_i64(&e.price, "price")?;
            parse_canonical_i64(&e.timestamp, "timestamp")?;
            parse_canonical_i64(&e.volume, "volume")?;
            // M1：profile 声明单位成交量 ⟹ 精确拒绝非 1（域缺项，不静默替换为 1.0）。
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
            let content = canonical_event_content(e);
            let payload_hash = sha256_hex(content.as_bytes());

            let existing = stmt
                .query_row(params![ikey], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })
                .optional()
                .map_err(|e| format!("查询 identity 失败：{e}"))?;

            if let Some((rec, ph, _seq)) = existing {
                if ph == payload_hash {
                    results.push(json!({
                        "status": "replay",
                        "identity_key": ikey,
                        "event_id": e.event_id,
                        "receipt_id": rec,
                        "payload_hash": payload_hash,
                    }));
                } else {
                    results.push(json!({
                        "status": "IdentityConflict",
                        "identity_key": ikey,
                        "event_id": e.event_id,
                        "existing_receipt_id": rec,
                        "existing_payload_hash": ph,
                        "new_payload_hash": payload_hash,
                    }));
                }
                continue;
            }

            next_seq += 1;
            // 收据绑定身份 + 内容（不同身份同内容不撞收据）。
            let receipt_id = format!(
                "rcpt-{}",
                &sha256_hex(format!("{ikey}|{payload_hash}").as_bytes())[..16]
            );
            insert
                .execute(params![
                    ikey,
                    e.revision,
                    payload_hash,
                    receipt_id,
                    next_seq,
                    input.source_namespace,
                    input.source_epoch,
                    input.instrument,
                    e.event_id,
                    e.revision,
                    e.received_at,
                    e.raw_text,
                    e.price,
                    e.timestamp,
                    e.volume,
                    e.seq,
                ])
                .map_err(|e| format!("insert raw_events 失败：{e}"))?;
            results.push(json!({
                "status": "accepted",
                "identity_key": ikey,
                "event_id": e.event_id,
                "receipt_id": receipt_id,
                "payload_hash": payload_hash,
                // WIRE：接纳序是无 JS safe 上限的精确整数，外发为规范十进制字符串。
                "seq": next_seq.to_string(),
            }));
        }

        // 本次输入档案记录（不覆盖 profile 绑定；profile_id/profile_hash 已在首次接纳时固定）。
        meta_set(&tx, "input_profile", &input.profile)?;
        meta_set(&tx, "input_file_hash", &sha256_hex(text.as_bytes()))?;
    }
    tx.commit()
        .map_err(|e| format!("accept commit 失败：{e}"))?;
    println!(
        "{}",
        json!({
            "ok": true,
            "session_id": input.session_id,
            "profile": profile_id,
            "profile_hash": profile_hash,
            "volume_unit": volume_unit,
            "results": results,
        })
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// S.Advance：Begin（判定前持久）→ 同次真实 Rust parser → CC-006 → Commit（CAS 发布）
// ────────────────────────────────────────────────────────────────────────────

fn read_raw_events(conn: &Connection, frontier: i64) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT identity_key, input_revision, payload_hash, receipt_id, seq, source_namespace,
                    source_epoch, instrument, event_id, revision, received_at, raw_text, price,
                    ts, volume, source_coord
             FROM raw_events WHERE seq <= ?1 ORDER BY seq ASC",
        )
        .map_err(|e| format!("prepare 读取失败：{e}"))?;
    let rows = stmt
        .query_map(params![frontier], |r| {
            Ok(json!({
                "identity_key": r.get::<_, String>(0)?,
                "input_revision": r.get::<_, String>(1)?,
                "payload_hash": r.get::<_, String>(2)?,
                "receipt_id": r.get::<_, String>(3)?,
                "seq": r.get::<_, i64>(4)?,
                "source_namespace": r.get::<_, String>(5)?,
                "source_epoch": r.get::<_, String>(6)?,
                "instrument": r.get::<_, String>(7)?,
                "event_id": r.get::<_, String>(8)?,
                "revision": r.get::<_, String>(9)?,
                "received_at": r.get::<_, String>(10)?,
                "raw_text": r.get::<_, String>(11)?,
                "price": r.get::<_, String>(12)?,
                "ts": r.get::<_, String>(13)?,
                "volume": r.get::<_, String>(14)?,
                "source_coord": r.get::<_, String>(15)?,
            }))
        })
        .map_err(|e| format!("query 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect 失败：{e}"))?;
    Ok(rows)
}

fn dir_str(d: Direction) -> &'static str {
    match d {
        Direction::Up => "UP",
        Direction::Down => "DOWN",
    }
}

// 参数数 10>7：把对象内容寻址所需字段（merged/raw 映射/输入引用/profile/规则）一次性传入，
// 拆结构体只换形状不降复杂度；本函数只组装、不判结构（判断仍唯一来自 classify_local_shape）。
#[allow(clippy::too_many_arguments)]
fn build_object_and_witnesses(
    merged: &[Bar],
    raw_by_seq: &BTreeMap<i64, Value>,
    merged_raws: &[Vec<i64>],
    start: usize,
    branch: &str,
    dir_ab: &str,
    dir_bc: &str,
    comparisons_json: &str,
    input_refs: &[Value],
    profile_id: &str,
) -> (Value, Vec<Value>, Vec<Value>) {
    let mid = start + 1;
    let end = start + 2;
    let object_canonical = json!({
        "branch": branch,
        "dir_ab": dir_ab,
        "dir_bc": dir_bc,
        "window_start": start,
        "window_mid": mid,
        "window_end": end,
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
            .filter_map(|seq| raw_by_seq.get(seq).cloned())
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
    relations.push(json!({
        "subject": format!("window-{start}-{mid}-{end}"),
        "relation_type": "classified_as",
        "object": object_id,
    }));

    let object = json!({
        "object_id": object_id,
        "object_revision": 1,
        "kind": "CC-006.local_shape",
        "branch": branch,
        "window_start": start,
        "window_mid": mid,
        "window_end": end,
        "dir_ab": dir_ab,
        "dir_bc": dir_bc,
        "comparisons_json": comparisons_json,
        "input_refs_json": canonical_json(&json!(input_refs)),
    });
    (object, witnesses, relations)
}

/// DUR-H01：已知失败的正常收尾——仅当仍持有本 attempt 的 Begin（token/gen/frontier 精确匹配、
/// writer_epoch 未变、published generation 仍为 gen-1）时，把本 attempt 的推进占用条件清理为 idle。
/// 验证与变更同事务 CAS；不清除别人的 Begin、不把未知中断自动放行（崩溃/悬挂修订仍保留 Begin/闭门，
/// 属 TB-05 恢复矩阵）。
fn cancel_own_begin(
    conn: &mut Connection,
    token: &str,
    gen: i64,
    frontier: i64,
) -> Result<bool, String> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("cancel Begin 事务失败：{e}"))?;
    let epoch = meta_get_opt(&tx, "writer_epoch")?.unwrap_or_default();
    if epoch != WRITER_EPOCH {
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
) -> Result<(), String> {
    verify_writer_epoch(conn)?;
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

/// C09.4 / #1370 AC4：完整对象先独立持久；此事务不发布任何对象表或结构根。
/// 后续发布失败时保留不可达批次与 Begin，不能用删批次冒充恢复。
fn persist_batch_before_publish(
    conn: &mut Connection,
    token: &str,
    gen: i64,
    frontier: i64,
    batch_id: &str,
    bytes: &[u8],
) -> Result<(), String> {
    if batch_id != format!("batch-{}", sha256_hex(bytes)) {
        return Err("IdentityConflict：batch_id 与规范字节哈希不符".to_string());
    }
    let byte_len = i64::try_from(bytes.len()).map_err(|_| "batch 长度超出持久 i64 域")?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开批次持久事务失败：{e}"))?;
    verify_begin_owner(&tx, token, gen, frontier)?;
    tx.execute(
        "INSERT OR IGNORE INTO batches(batch_id, canonical_bytes, byte_len) VALUES(?1, ?2, ?3)",
        params![batch_id, bytes, byte_len],
    )
    .map_err(|e| format!("持久 batch 失败：{e}"))?;
    verify_batch_bytes(&tx, batch_id, bytes)?;
    tx.commit()
        .map_err(|e| format!("批次持久 commit 失败：{e}"))
}

fn cmd_advance(db: &Path) -> Result<(), String> {
    let mut conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let profile_id = meta_get_opt(&conn, "profile_id")?.unwrap_or_default();
    let profile_hash = meta_get_opt(&conn, "profile_hash")?.unwrap_or_default();
    let catalog_revision = meta_get_opt(&conn, "catalog_revision")?.unwrap_or_default();

    // ── Begin：判定前先由短写事务核 writer_epoch、取得唯一推进权、持久 Begin/门/输入前沿 ──
    let (token, gen, frontier) = {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| format!("Begin 事务失败：{e}"))?;
        verify_writer_epoch(&tx)?;
        let state = meta_get_opt(&tx, "advance_state")?.unwrap_or_else(|| "idle".to_string());
        if state != "idle" {
            return Err(format!(
                "StaleWriter：已有进行中的 advance（advance_state=`{state}`）；恢复闭门属 TB-05，可用 reset 一次性重开"
            ));
        }
        let cur_gen: i64 = meta_get_opt(&tx, "generation")?
            .and_then(|s| s.parse().ok())
            .ok_or("读 generation 失败")?;
        let gen = cur_gen + 1;
        let frontier: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读输入前沿失败：{e}"))?;
        let token = make_token();
        meta_set(
            &tx,
            "advance_state",
            &format!("begun:{token}:{gen}:{frontier}"),
        )?;
        meta_set(&tx, "begin_generation", &gen.to_string())?;
        meta_set(&tx, "begin_input_frontier", &frontier.to_string())?;
        meta_set(&tx, "begin_token", &token)?;
        tx.commit().map_err(|e| format!("Begin commit 失败：{e}"))?;
        (token, gen, frontier)
    };

    // ── 同次真实 Rust parser（本地屏障已持久，Begin 之后只读 ≤ 前沿的已接纳事件）──
    let events = read_raw_events(&conn, frontier)?;
    let mut raw_by_seq: BTreeMap<i64, Value> = BTreeMap::new();
    for ev in &events {
        raw_by_seq.insert(ev["seq"].as_i64().unwrap_or(-1), ev.clone());
    }

    let config = ThetaConfig::default();
    let mut incr = ParseLayerIncr::new(&config);
    let mut layer = None;
    let mut raw_bars: Vec<Bar> = Vec::new();
    for ev in &events {
        let px = parse_canonical_i64(ev["price"].as_str().unwrap_or(""), "price")?;
        let t = parse_canonical_i64(ev["ts"].as_str().unwrap_or(""), "timestamp")?;
        // 受测域 profile volume_unit="1"（accept 已精确拒绝非 1）⟹ Bar.volume=1.0 是忠实投影。
        let bar = Bar {
            source_index: ev["seq"].as_i64().unwrap_or(0) as usize,
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

    // raw → merged 组号（inclusion 同源映射）。
    let mut merged_raws: Vec<Vec<i64>> = vec![Vec::new(); merged.len()];
    for seq in raw_by_seq.keys().copied() {
        if let Some(g) = parser::inclusion::merged_group_index(merged, seq as usize) {
            merged_raws[g].push(seq);
        }
    }

    // 逐窗口构造对象/见证/关系，并持久化「知识不足 / 域不满足」观察（不造第五 Other 分型）。
    let mut objects: Vec<Value> = Vec::new();
    let mut witnesses: Vec<Value> = Vec::new();
    let mut relations: Vec<Value> = Vec::new();
    let mut observations: Vec<Value> = Vec::new();

    // 域前件（原始三K，喂入 parser 之前）：相邻包含（含等值）⟹ 域不满足——parser 会把包含
    // 合并成更少标准 K，若不在此先查，会误报「知识不足」。复用 inclusion::contains 同一判断。
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
            "detail": {"raw_events": events.len()},
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
                            .filter_map(|seq| raw_by_seq.get(seq))
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
                    &raw_by_seq,
                    &merged_raws,
                    start,
                    branch.as_str(),
                    dir_ab_s,
                    dir_bc_s,
                    &cmp_json,
                    &input_refs,
                    &profile_id,
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
                // 滑窗仅对 merged.len() >= 3 调用，此分支为防御性兜底。
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

    // 去重（DUR）：raw 级域前件（喂入 parser 前）与 merged 级 DomainNotSatisfied 可能报告同一窗口同一
    // 违反（例：全等三K无方向时 parser 不合并，merged==raw，两处都报 window[0,1,2] adjacent_inclusion），
    // observation 身份 = (kind, window_start, window_mid, window_end, reason)，同一事实只发布一次，
    // 不造 UNIQUE 约束冲突，也不删合法事实（不同窗口/原因保留）。
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
    // ── 内容寻址批次：封存完整正式结果（原始事件 + 对象 + 见证 + 关系 + 观察 + 绑定）──
    let batch_json = json!({
        "generation": gen,
        "structure_cut": structure_cut,
        "input_frontier": frontier,
        "catalog_revision": catalog_revision,
        "profile_id": profile_id,
        "profile_hash": profile_hash,
        "rule_revision": RULE_REVISION,
        "raw_events": events,
        "objects": objects,
        "witnesses": witnesses,
        "relations": relations,
        "observations": observations,
    });
    let batch_bytes = canonical_bytes_of_value(&batch_json);
    let batch_id = format!("batch-{}", sha256_hex(&batch_bytes));

    persist_batch_before_publish(&mut conn, &token, gen, frontier, &batch_id, &batch_bytes)?;

    // ── Commit：核 Begin/epoch/前沿仍有效后，同一事务发布对象/关系/修订/索引根/目录状态 ──
    let classified_count = objects.len();
    let mut frontier_moved = false;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开 commit 事务失败：{e}"))?;
    {
        verify_begin_owner(&tx, &token, gen, frontier)?;
        verify_batch_bytes(&tx, &batch_id, &batch_bytes)?;
        let cur_frontier: i64 = tx
            .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| format!("读输入前沿失败：{e}"))?;
        if cur_frontier != frontier {
            // DUR-H01：已知前沿变化、本 attempt 尚未公布结构 Commit —— 本事务不做任何写入，
            // 事务结束后由 cancel_own_begin 条件清理自己的推进占用（不清理别人的 Begin）。
            frontier_moved = true;
        } else {
            tx.execute("DELETE FROM objects", [])
                .map_err(|e| format!("clear objects 失败：{e}"))?;
            tx.execute("DELETE FROM witnesses", [])
                .map_err(|e| format!("clear witnesses 失败：{e}"))?;
            tx.execute("DELETE FROM relations", [])
                .map_err(|e| format!("clear relations 失败：{e}"))?;
            tx.execute("DELETE FROM observations", [])
                .map_err(|e| format!("clear observations 失败：{e}"))?;

            {
                let mut stmt = tx
                .prepare(
                    "INSERT INTO objects(object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc,
                       window_start, window_mid, window_end, comparisons_json, input_refs_json)
                     VALUES(?1, 1, 'CC-006.local_shape', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(|e| format!("prepare objects 失败：{e}"))?;
                for o in &objects {
                    stmt.execute(params![
                        o["object_id"].as_str().unwrap_or(""),
                        o["batch_id"].as_str().unwrap_or(batch_id.as_str()),
                        o["branch"].as_str().unwrap_or(""),
                        o["dir_ab"].as_str().unwrap_or(""),
                        o["dir_bc"].as_str().unwrap_or(""),
                        o["window_start"].as_i64().unwrap_or(0),
                        o["window_mid"].as_i64().unwrap_or(0),
                        o["window_end"].as_i64().unwrap_or(0),
                        o["comparisons_json"].as_str().unwrap_or("[]"),
                        o["input_refs_json"].as_str().unwrap_or("[]"),
                    ])
                    .map_err(|e| format!("insert objects 失败：{e}"))?;
                }
            }
            {
                let mut stmt = tx
                .prepare(
                    "INSERT INTO witnesses(witness_id, object_id, slot, merged_source_index, merged_high,
                       merged_low, merged_open, merged_close, raw_json)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
                    ])
                    .map_err(|e| format!("insert witnesses 失败：{e}"))?;
                }
            }
            {
                let mut stmt = tx
                .prepare(
                    "INSERT INTO relations(relation_id, subject, relation_type, object) VALUES(?1, ?2, ?3, ?4)",
                )
                .map_err(|e| format!("prepare relations 失败：{e}"))?;
                for r in &relations {
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
                    ])
                    .map_err(|e| format!("insert relations 失败：{e}"))?;
                }
            }
            {
                let mut stmt = tx
                .prepare(
                    "INSERT INTO observations(observation_id, batch_id, kind, window_start, window_mid,
                       window_end, reason, detail_json)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .map_err(|e| format!("prepare observations 失败：{e}"))?;
                for ob in &observations {
                    let oid = format!(
                        "obs-{}",
                        &sha256_hex(&canonical_bytes_of_value(&json!({
                            "kind": ob["kind"],
                            "window_start": ob["window_start"],
                            "window_mid": ob["window_mid"],
                            "window_end": ob["window_end"],
                            "reason": ob["reason"],
                        })))[..16]
                    );
                    stmt.execute(params![
                        oid,
                        batch_id,
                        ob["kind"].as_str().unwrap_or(""),
                        ob["window_start"].as_i64(),
                        ob["window_mid"].as_i64(),
                        ob["window_end"].as_i64(),
                        ob["reason"].as_str().unwrap_or(""),
                        serde_json::to_string(&ob["detail"]).unwrap_or_else(|_| "{}".to_string()),
                    ])
                    .map_err(|e| format!("insert observations 失败：{e}"))?;
                }
            }

            // 索引可达根：structure_cut / generation / index_frontier（与对象同事务生效）。
            meta_set(&tx, "generation", &gen.to_string())?;
            meta_set(&tx, "structure_cut", &structure_cut)?;
            meta_set(&tx, "index_frontier", &batch_id)?;

            // 目录状态（H3）：实现/证明/运行分列，绑定具体证据；无实例不标 run、不伪造已证。
            let run_status = if classified_count >= 1 {
                "run"
            } else {
                "not_run"
            };
            let proof_status = "not_proved";
            let evidence = json!({
                "tested_domain": "TestOnly tick 1:1 OHLC（O=H=L=C），相邻严格无包含，初始化方向由前两点确定，无同价端点竞争",
                "profile_id": profile_id,
                "profile_hash": profile_hash,
                "rule_revision": RULE_REVISION,
                "batch_id": batch_id,
                "structure_cut": structure_cut,
                "classified_objects": classified_count,
                "windows_total": wins.len(),
                "merged_bars": merged.len(),
                "insufficient_knowledge": observations.iter().filter(|o| o["kind"] == "insufficient_knowledge").count(),
                "domain_not_satisfied": observations.iter().filter(|o| o["kind"] == "domain_not_satisfied").count(),
                "oracle_test": "local_shape::tests::cc006_four_branch_oracle（独立硬编码期望值；执行记录见验证报告，本会话不据此自证）",
                "scope": {"structure": "CompleteCut", "economic": "not_started"},
            });
            tx.execute(
            "UPDATE catalog SET impl_status='implemented', proof_status=?1, run_status=?2, evidence_json=?3 WHERE catalog_id='CC-006'",
            params![proof_status, run_status, evidence.to_string()],
        )
        .map_err(|e| format!("update CC-006 catalog 失败：{e}"))?;
            tx.execute(
                "UPDATE catalog SET run_status='not_run' WHERE catalog_id <> 'CC-006'",
                [],
            )
            .map_err(|e| format!("update 其余 catalog 失败：{e}"))?;

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
        // DUR-H01：仅清理本 attempt 自己的 Begin（token/gen/frontier 精确匹配 + epoch/generation 未变），
        // 让下一次合法推进可取得新 token；崩溃/悬挂修订仍保留 Begin/闭门。
        let released = cancel_own_begin(&mut conn, &token, gen, frontier)?;
        let disposition = if released {
            "本 attempt 已正常取消并释放推进权"
        } else {
            "Begin 所有权已变化，未清理推进占用"
        };
        return Err(format!(
            "StaleWriter：输入前沿已由 {frontier} 变化（新输入已接纳，请重新 advance）；{disposition}"
        ));
    }

    println!(
        "{}",
        json!({
            "ok": true,
            // WIRE：generation 是无 JS safe 上限的精确整数，外发为规范十进制字符串。
            "generation": gen.to_string(),
            "structure_cut": structure_cut,
            "batch_id": batch_id,
            "merged_bars": merged.len(),
            "raw_events": events.len(),
            "windows_total": wins.len(),
            "objects_published": objects.len(),
            "observations": {
                "insufficient_knowledge": observations.iter().filter(|o| o["kind"] == "insufficient_knowledge").count(),
                "domain_not_satisfied": observations.iter().filter(|o| o["kind"] == "domain_not_satisfied").count(),
            },
            "batch_byte_len": batch_bytes.len(),
            "scope": {"structure": "CompleteCut", "economic": "not_started"},
        })
    );
    Ok(())
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

// ────────────────────────────────────────────────────────────────────────────
// WIRE 精确整数投影（AC5 / INTERFACE-CONTRACTS.md:11）：只转换「明确外发」的精确坐标/版本/
// 来源游标字段为规范十进制字符串；不改内部封存（batch canonical JSON / 对象身份）的数值，
// 不改已签目录的有限分类码。内部计算仍在 i64/usize 精确域。
// ────────────────────────────────────────────────────────────────────────────

fn num_to_str(v: &Value) -> Result<Value, String> {
    v.as_i64()
        .map(|n| Value::String(n.to_string()))
        .ok_or_else(|| "持久整数必须是 i64，不能是浮点、布尔、空值或文本".to_string())
}

/// `objects[].input_refs`：把 `merged_index`（结构坐标）与 `raw_refs[].seq`（接纳序）投影为字符串。
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
        }
        out.push(Value::Object(obj));
    }
    Ok(Value::Array(out))
}

/// `witnesses[].raw_bars`：把 `seq`（接纳序）投影为字符串。
fn project_raw_bars(bars: &Value) -> Result<Value, String> {
    let bars = bars.as_array().ok_or("raw_bars 必须是数组")?;
    let mut out = Vec::with_capacity(bars.len());
    for bar in bars {
        let mut obj = bar.as_object().ok_or("raw_bars 成员必须是对象")?.clone();
        let seq = num_to_str(obj.get("seq").unwrap_or(&Value::Null))?;
        obj.insert("seq".to_string(), seq);
        out.push(Value::Object(obj));
    }
    Ok(Value::Array(out))
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
        // 已签目录允许枚举数组或生成式对象，不改写二者。
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

fn read_scope(conn: &Connection) -> Result<Value, String> {
    match meta_get_opt(conn, "scope")? {
        Some(text) => json_shape(&text, JsonShape::Object),
        None => Ok(json!({})),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// S.ReadCatalog / S.Snapshot（只读查询，单个读事务内读同一已提交 cut）
// ────────────────────────────────────────────────────────────────────────────

fn read_catalog_in_tx(conn: &Connection) -> Result<Value, String> {
    let meta = db_meta(conn)?;
    let mut stmt = conn
        .prepare("SELECT catalog_id, kind, title, domain, branches_json, impl_status, proof_status, run_status, evidence_json FROM catalog ORDER BY catalog_id")
        .map_err(|e| format!("prepare catalog 失败：{e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "kind": r.get::<_, String>(1)?,
                "title": r.get::<_, String>(2)?,
                "domain": r.get::<_, String>(3)?,
                "branches": json_column(r, 4, JsonShape::CatalogBranches)?,
                "implementation_status": r.get::<_, String>(5)?,
                "proof_status": r.get::<_, String>(6)?,
                "run_status": r.get::<_, String>(7)?,
                "evidence": json_column(r, 8, JsonShape::Object)?,
            }))
        })
        .map_err(|e| format!("query catalog 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect catalog 失败：{e}"))?;
    Ok(json!({
        "session_id": meta.get("session_id").cloned().unwrap_or_default(),
        "generation": meta.get("generation").cloned().unwrap_or_default(),
        "structure_cut": meta.get("structure_cut").cloned().unwrap_or_default(),
        "catalog_revision": meta.get("catalog_revision").cloned().unwrap_or_default(),
        "scope": read_scope(conn)?,
        "items": rows,
        "counts": {
            "implemented": rows.iter().filter(|r| r["implementation_status"] == "implemented").count(),
            "not_implemented": rows.iter().filter(|r| r["implementation_status"] == "not_implemented").count(),
            "run": rows.iter().filter(|r| r["run_status"] == "run").count(),
            "not_run": rows.iter().filter(|r| r["run_status"] == "not_run").count(),
        }
    }))
}

fn read_snapshot_in_tx(conn: &Connection) -> Result<Value, String> {
    let meta = db_meta(conn)?;
    let mut ostmt = conn
        .prepare("SELECT object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, window_start, window_mid, window_end, comparisons_json, input_refs_json FROM objects ORDER BY window_start")
        .map_err(|e| format!("prepare objects 失败：{e}"))?;
    let objects = ostmt
        .query_map([], |r| {
            Ok(json!({
                "object_id": r.get::<_, String>(0)?,
                "object_revision": r.get::<_, i64>(1)?.to_string(),
                "kind": r.get::<_, String>(2)?,
                "batch_id": r.get::<_, String>(3)?,
                "branch": r.get::<_, String>(4)?,
                "dir_ab": r.get::<_, String>(5)?,
                "dir_bc": r.get::<_, String>(6)?,
                "window_start": r.get::<_, i64>(7)?.to_string(),
                "window_mid": r.get::<_, i64>(8)?.to_string(),
                "window_end": r.get::<_, i64>(9)?.to_string(),
                "comparisons": json_column(r, 10, JsonShape::Array)?,
                "input_refs": project_input_refs(
                    &json_column(r, 11, JsonShape::Array)?
                ).map_err(|e| json_column_error(11, e))?,
            }))
        })
        .map_err(|e| format!("query objects 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect objects 失败：{e}"))?;

    let mut wstmt = conn
        .prepare("SELECT witness_id, object_id, slot, merged_source_index, merged_high, merged_low, merged_open, merged_close, raw_json FROM witnesses ORDER BY object_id, slot")
        .map_err(|e| format!("prepare witnesses 失败：{e}"))?;
    let witnesses = wstmt
        .query_map([], |r| {
            Ok(json!({
                "witness_id": r.get::<_, String>(0)?,
                "object_id": r.get::<_, String>(1)?,
                "slot": r.get::<_, i64>(2)?.to_string(),
                "merged_source_index": r.get::<_, i64>(3)?.to_string(),
                "merged_high": r.get::<_, String>(4)?,
                "merged_low": r.get::<_, String>(5)?,
                "merged_open": r.get::<_, String>(6)?,
                "merged_close": r.get::<_, String>(7)?,
                "raw_bars": project_raw_bars(
                    &json_column(r, 8, JsonShape::Array)?
                ).map_err(|e| json_column_error(8, e))?,
            }))
        })
        .map_err(|e| format!("query witnesses 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect witnesses 失败：{e}"))?;

    let mut rstmt = conn
        .prepare("SELECT subject, relation_type, object FROM relations ORDER BY subject, relation_type, object")
        .map_err(|e| format!("prepare relations 失败：{e}"))?;
    let relations = rstmt
        .query_map([], |r| {
            Ok(json!({
                "subject": r.get::<_, String>(0)?,
                "relation_type": r.get::<_, String>(1)?,
                "object": r.get::<_, String>(2)?,
            }))
        })
        .map_err(|e| format!("query relations 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect relations 失败：{e}"))?;

    let mut obs_stmt = conn
        .prepare("SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, reason, detail_json FROM observations ORDER BY kind, window_start")
        .map_err(|e| format!("prepare observations 失败：{e}"))?;
    let observations = obs_stmt
        .query_map([], |r| {
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
        })
        .map_err(|e| format!("query observations 失败：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect observations 失败：{e}"))?;

    Ok(json!({
        "session_id": meta.get("session_id").cloned().unwrap_or_default(),
        "generation": meta.get("generation").cloned().unwrap_or_default(),
        "structure_cut": meta.get("structure_cut").cloned().unwrap_or_default(),
        "catalog_revision": meta.get("catalog_revision").cloned().unwrap_or_default(),
        "scope": read_scope(conn)?,
        "index_frontier": meta.get("index_frontier").cloned().unwrap_or_default(),
        "profile_id": meta.get("profile_id").cloned().unwrap_or_default(),
        "profile_hash": meta.get("profile_hash").cloned().unwrap_or_default(),
        "objects": objects,
        "witnesses": witnesses,
        "relations": relations,
        "observations": observations,
    }))
}

fn cmd_catalog(db: &Path) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    // H1：单个读事务内读同一已提交 cut（WAL 读快照）。
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开读事务失败：{e}"))?;
    let out = read_catalog_in_tx(&tx)?;
    drop(tx);
    println!("{}", out);
    Ok(())
}

fn cmd_snapshot(db: &Path) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    // H1：单个读事务内读同一已提交 cut（WAL 读快照）。
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("开读事务失败：{e}"))?;
    let out = read_snapshot_in_tx(&tx)?;
    drop(tx);
    println!("{}", out);
    Ok(())
}

fn cmd_meta(db: &Path) -> Result<(), String> {
    let conn = open_db(db)?;
    ensure_initialized(&conn)?;
    let meta = db_meta(&conn)?;
    println!("{}", json!(meta));
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// CLI 分发
// ────────────────────────────────────────────────────────────────────────────

fn usage() -> String {
    "用法：s_structure_session <init|accept|advance|catalog|snapshot|meta|reset> [参数]\n\
     \x20 init      --db <路径> --session <id> --catalog <catalog.json>      （只允许不存在的目标）\n\
     \x20 accept    --db <路径> --input <输入.json> --profile <profile.json>\n\
     \x20 advance   --db <路径>\n\
     \x20 catalog   --db <路径>\n\
     \x20 snapshot  --db <路径>\n\
     \x20 meta      --db <路径>\n\
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
    let cmd = args[1].as_str();
    let db = arg_value(&args, "--db").map(PathBuf::from);
    let session = arg_value(&args, "--session").unwrap_or_else(|| "s-session-default".to_string());
    let catalog = arg_value(&args, "--catalog").map(PathBuf::from);
    let input = arg_value(&args, "--input").map(PathBuf::from);
    let profile = arg_value(&args, "--profile").map(PathBuf::from);

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
            cmd_accept(&d, &i, &p)
        }
        "advance" => cmd_advance(&db.ok_or("缺 --db".to_string())?),
        "catalog" => cmd_catalog(&db.ok_or("缺 --db".to_string())?),
        "snapshot" => cmd_snapshot(&db.ok_or("缺 --db".to_string())?),
        "meta" => cmd_meta(&db.ok_or("缺 --db".to_string())?),
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
                std::env::temp_dir().join(format!("s1370-{}-{stamp}-{number}", std::process::id()));
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
        meta_set(conn, "writer_epoch", WRITER_EPOCH).unwrap();
        meta_set(conn, "generation", "0").unwrap();
        meta_set(conn, "advance_state", "begun:owned:1:6").unwrap();
        meta_set(conn, "begin_token", "owned").unwrap();
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
        let actual = read_catalog_in_tx(&conn).unwrap();
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
        persist_batch_before_publish(&mut conn, "owned", 1, 6, &id, bytes).unwrap();
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
        assert!(persist_batch_before_publish(&mut conn, "owned", 1, 6, &id, b"changed").is_err());
        verify_batch_bytes(&reader, &id, bytes).unwrap();
        conn.execute(
            "UPDATE batches SET canonical_bytes=?1 WHERE batch_id=?2",
            params![b"corrupt".as_slice(), id],
        )
        .unwrap();
        assert!(persist_batch_before_publish(&mut conn, "owned", 1, 6, &id, bytes).is_err());
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
            assert!(!cancel_own_begin(&mut conn, "owned", 1, 6).unwrap());
            assert_eq!(db_meta(&conn), before);
        }
        set_test_begin(&conn);
        assert!(cancel_own_begin(&mut conn, "owned", 1, 6).unwrap());
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert!(!cancel_own_begin(&mut conn, "owned", 1, 6).unwrap());
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
        cmd_accept(&files.db(), &input, &profile).unwrap();
        cmd_advance(&files.db()).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let before = db_meta(&conn);
        let mut altered: Value =
            serde_json::from_str(&std::fs::read_to_string(&profile).unwrap()).unwrap();
        altered["note"] = json!("same profile_id, different canonical content");
        let other = files.0.join("changed-profile.json");
        std::fs::write(&other, altered.to_string()).unwrap();
        assert!(cmd_accept(&files.db(), &input, &other)
            .unwrap_err()
            .contains("IdentityConflict"));
        assert_eq!(db_meta(&conn), before);
        cmd_accept(&files.db(), &input, &profile).unwrap();
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
        )
        .unwrap();
        cmd_advance(&files.db()).unwrap();
        let conn = open_db(&files.db()).unwrap();
        let snapshot = read_snapshot_in_tx(&conn).unwrap();
        assert!(snapshot["objects"].as_array().unwrap().is_empty());
        assert_eq!(snapshot["observations"].as_array().unwrap().len(), 1);
        assert_eq!(
            snapshot["observations"][0]["kind"],
            json!("domain_not_satisfied")
        );
        conn.execute("UPDATE observations SET detail_json='{'", [])
            .unwrap();
        assert!(read_snapshot_in_tx(&conn).is_err());
        conn.execute(
            "UPDATE catalog SET branches_json='null' WHERE catalog_id='CC-006'",
            [],
        )
        .unwrap();
        assert!(read_catalog_in_tx(&conn).is_err());
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

    /// 规范十进制整数：拒绝 `+`、空白、前导零、`-0`；接受 `0` 与常规整数。
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

    /// WIRE：input_refs 的 merged_index 与 raw_refs[].seq、raw_bars[].seq 投影为规范十进制字符串；
    /// 其它字段（身份/价格/字符串）原样，不全局改写。
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

        // number 与字符串区分：未投影的字段仍是字符串/原值，不把价格等误转。
        assert_eq!(projected[0]["raw_refs"][0]["source_coord"], json!("0"));
    }
}
