//! #1394 C §5.1/5.8：S3 新域（`s-session/3`）的隔离初始化与版本分派。
//!
//! 本模块负责版本分派与**在全新库上建立 `s-session/3` 域**，子模块提供内部运行接线：
//!
//! - **显式选择**：只有 `--protocol-revision s-session/3` 进本域；显式 `s-session/2` 强制走既有
//!   v2 初始化（v2 合同要求的参数必须齐全，缺参拒绝，不回落旧域）；未知/重复/变形选择器一律
//!   拒绝；无选择器时返回 `false`，调用方完全沿原入口行为。
//! - **先验证后创建**：clock/profile/catalog/参数全部验过才落盘；目标库已存在一律拒绝
//!   （无热迁移，不做原地升级）。
//! - **同一 SQLite 事务建域**：共同 S authority 表（root `SCHEMA`）+ `v2::SCHEMA_V2` 控制表 +
//!   profile 需要时的 `tb02a::SCHEMA` + `content_store::init_schema`（登记 `s-content/1`）。
//! - **不调用 `init_core`**：新域不登记旧 private 格式（`s_storage_format` 的 changes-only/1、
//!   证据图三表），也不写旧批次；旧域与新域各自独立，不迁移、不混代。
//! - **clock 共用一份规则**：`s-clock-plan/1` 的解析/校验/hash 全部复用 `v2::ClockPlan`，
//!   本模块不复制第二份 clock 解析（同一判断不得有宽严两档）。
//!
//! 公开命令当前只有 init；内部 `driver` 已贯通接纳/Begin/Commit、恢复与完整状态审计，
//! 但资源实测和传输接线尚未完成，serve/advance/reader 仍未注册。旧命令打到本域会被既有控制行核验
//! （`v2::prepare_control` → 协议标记/控制表不支持）在任何写入之前拒绝，不冒充可用。
//!
//! ⚠️ 边界如实（独评 F2）：输入预验证保证坏输入不落盘，事务保证建表/登记同成同败；但**「创建 db
//! 文件」与「事务提交」不是同一个原子动作**——`open` 之后、`commit` 之前中断会留下不完整库文件。
//! 此时本模块不自动清理、不覆盖：拒绝已存在路径本身就是 fail-closed，重试要换新路径。
//!
//! ⚠️ 旧 `reset` 路由（独评 F1，含二次收紧）：无 selector 的旧 `reset` 是「先删后建 v1」，只懂
//! v1/v2 合同。`refuse_reset_of_new_domain` 在任何删除之前**只读**判定域：读不出/锁冲突/非 SQLite/
//! 标记冲突或不完整一律拒绝（不做写打开回退、不让缺一个标记降级）。正常 v1（无协议标记、无控制表、
//! 无内容表）与两处标记同为 `s-session/2` 的 v2 仍走旧 reset；本切片不提供新的迁移/重置功能。

use super::*;
use newchan_rust::s_content;
use newchan_rust::session_protocol::{positive, strict_json};
use newchan_rust::session_transport;
use rusqlite::OpenFlags;
use std::collections::BTreeSet;

mod control;
mod object_records;
mod commit;
mod generation;
mod control_audit;
mod history;
mod catalog_evidence;
mod prepared;
mod state;
mod driver;
mod reader;
mod projection;
mod q_resources;
mod q_delta;
mod q_snapshot;
mod q_wire;
mod q_content;
mod q_watch;
mod q_lease;
#[cfg(unix)]
mod q_executor;
#[cfg(unix)]
mod capture;
pub(super) mod aux_index;
pub(super) mod object_index;
mod source_audit;

/// 本模块唯一负责的协议版本。
pub(super) const PROTOCOL_REVISION: &str = "s-session/3";
/// 旧 v2 合同版本号；显式给出时仍由既有 v2 初始化入口执行。
pub(super) const PREVIOUS_REVISION: &str = "s-session/2";
const SELECTOR: &str = "--protocol-revision";
const DELIVERY_POLICY_REVISION: &str = "generation-window/1";
/// v3 init 允许出现的全部 flag（含 selector）；其余一律拒绝，不假装可用。
const V3_INIT_FLAGS: [&str; 8] = [
    SELECTOR,
    "--db",
    "--session",
    "--catalog",
    "--profile",
    "--session-generation",
    "--clock-plan",
    "--delivery-retain-generations",
];
/// 显式 `s-session/2` 必须齐全的初始化参数（与 `v2::dispatch` init 分支同一份合同）。
const V2_INIT_FLAGS: [&str; 7] = [
    "--db",
    "--session",
    "--catalog",
    "--profile",
    "--session-generation",
    "--clock-plan",
    "--delivery-retain-generations",
];

/// 版本选择器分派：`Ok(true)` 表示本次调用已处理，调用方直接结束；无选择器时 `Ok(false)`。
pub(super) fn dispatch(args: &[String]) -> Result<bool, String> {
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    let mut positions = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        if arg.as_str() == SELECTOR {
            positions.push(index);
        } else if arg.starts_with(SELECTOR) {
            // `--protocol-revision=...` 一类变形选择器：明确拒绝，不当作「没给」。
            return Err(format!(
                "ProtocolSelectorUnsupported：只支持 `{SELECTOR} <版本>` 形式，收到 `{arg}`"
            ));
        }
    }
    if positions.is_empty() {
        return Ok(false);
    }
    if positions.len() > 1 {
        return Err(format!("ProtocolSelectorConflict：{SELECTOR} 只能出现一次"));
    }
    let value = args
        .get(positions[0] + 1)
        .filter(|v| !v.is_empty() && !v.starts_with("--"))
        .ok_or_else(|| format!("VersionUnsupported：{SELECTOR} 缺版本值"))?;
    if cmd != "init" {
        return Err(format!(
            "VersionUnsupported：{SELECTOR} 目前只适用于 init；`{cmd}` 尚未接线，拒绝执行"
        ));
    }
    if value == PROTOCOL_REVISION {
        let out = init_from_args(args)?;
        println!("{out}");
        return Ok(true);
    }
    if value == PREVIOUS_REVISION {
        // 显式 s-session/2：强制走既有 v2 初始化；缺参不得回落旧 v1 入口（那会静默建错域）。
        for key in V2_INIT_FLAGS {
            if !args.iter().any(|a| a == key) {
                return Err(format!(
                    "VersionUnsupported：显式 {PREVIOUS_REVISION} 缺 {key}（v2 初始化合同要求全参）"
                ));
            }
        }
        if !v2::dispatch(args)? {
            return Err(format!(
                "VersionUnsupported：显式 {PREVIOUS_REVISION} 未进入 v2 初始化分派；拒绝回落旧域"
            ));
        }
        return Ok(true);
    }
    Err(format!(
        "VersionUnsupported：未知协议版本选择 `{value}`；支持 {PREVIOUS_REVISION}（旧合同）与 {PROTOCOL_REVISION}（新域）"
    ))
}

/// v3 init 的完整 key/value 解析（独评 F4）：每个 token 必须是被允许的 flag，且紧跟一个非空、
/// 非 flag 形态的值。缺值、重复 key、孤立 token、未知/变形 flag 一律在**创建任何文件之前**拒绝；
/// 不用「找 key 再取下一 token」的旧口径（那会把下一个 flag 当值）。无 selector 与显式 v2 路径
/// 不经过本解析，既有参数语义不变。
fn parse_v3_args(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    let mut index = 2;
    while index < args.len() {
        let key = args[index].as_str();
        if !V3_INIT_FLAGS.contains(&key) {
            return Err(format!(
                "VersionUnsupported：{PROTOCOL_REVISION} init 不认识 token `{key}`（未知/变形 flag 或孤立 token）"
            ));
        }
        if values.contains_key(key) {
            return Err(format!(
                "VersionUnsupported：{PROTOCOL_REVISION} init 的 {key} 重复"
            ));
        }
        let value = args
            .get(index + 1)
            .filter(|value| !value.is_empty() && !value.starts_with("--"))
            .ok_or_else(|| format!("VersionUnsupported：{PROTOCOL_REVISION} init 的 {key} 缺值"))?;
        values.insert(key.to_string(), value.clone());
        index += 2;
    }
    Ok(values)
}

fn init_from_args(args: &[String]) -> Result<Value, String> {
    let values = parse_v3_args(args)?;
    if values.get(SELECTOR).map(String::as_str) != Some(PROTOCOL_REVISION) {
        return Err(format!(
            "VersionUnsupported：{SELECTOR} 值必须是 {PROTOCOL_REVISION}"
        ));
    }
    let required = |key: &str| -> Result<String, String> {
        values
            .get(key)
            .cloned()
            .ok_or_else(|| format!("VersionUnsupported：{PROTOCOL_REVISION} init 缺 {key}"))
    };
    let db = PathBuf::from(required("--db")?);
    let session = required("--session")?;
    let catalog = PathBuf::from(required("--catalog")?);
    let profile = PathBuf::from(required("--profile")?);
    let session_generation = required("--session-generation")?;
    let clock = PathBuf::from(required("--clock-plan")?);
    let delivery_retain = positive(
        &required("--delivery-retain-generations")?,
        "--delivery-retain-generations",
    )?;
    init_v3(
        &db,
        &session,
        &catalog,
        &profile,
        &session_generation,
        &clock,
        delivery_retain,
    )
}

/// 目录严格校验：复用 v2 审计（`s_session/s_query_integrity.py` `_verify_control` 段）对已签目录
/// 定义的不变量——严格 JSON 解码、`catalog_revision` 非空、`items` 完整且身份唯一、每条具备
/// `kind/title/domain/branches`；定义与规范 hash 落库后可被同一审计逐字重算。不引入第二套签署
/// 格式，也不接受缺版本的默认值（旧 `init_core` 的 `.unwrap_or("unknown")` 宽松口径不复用）。
fn load_catalog_strict(path: &Path) -> Result<(Value, String, usize), String> {
    let definition =
        strict_json(&std::fs::read(path).map_err(|e| format!("读 catalog 失败：{e}"))?)?;
    let (revision, item_count) = {
        let object = definition
            .as_object()
            .ok_or("SchemaUnsupported：catalog 顶层必须是对象")?;
        let revision = object
            .get("catalog_revision")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("SchemaUnsupported：catalog 缺非空 catalog_revision（新域不默认 unknown）")?
            .to_string();
        let items = object
            .get("items")
            .and_then(Value::as_array)
            .ok_or("SchemaUnsupported：catalog 缺 items 数组")?;
        if items.is_empty() {
            return Err("InvalidDomain：catalog items 为空".into());
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (index, item) in items.iter().enumerate() {
            let item = item
                .as_object()
                .ok_or_else(|| format!("SchemaUnsupported：catalog items[{index}] 不是对象"))?;
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("SchemaUnsupported：catalog items[{index}] 缺非空 id"))?;
            if !seen.insert(id) {
                return Err(format!("InvalidDomain：catalog 身份重复 `{id}`"));
            }
            for key in ["kind", "title", "domain"] {
                item.get(key)
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("SchemaUnsupported：catalog `{id}` 缺文本 {key}"))?;
            }
            match item.get("branches") {
                Some(Value::Object(_)) | Some(Value::Array(_)) => {}
                _ => {
                    return Err(format!(
                        "SchemaUnsupported：catalog `{id}` 的 branches 必须是对象或数组"
                    ))
                }
            }
        }
        if let Some(count) = object.get("item_count") {
            let count = count
                .as_i64()
                .ok_or("SchemaUnsupported：catalog item_count 必须是整数")?;
            if usize::try_from(count).ok() != Some(items.len()) {
                return Err(format!(
                    "SchemaUnsupported：catalog item_count={count} 与 items 实际 {} 项不符",
                    items.len()
                ));
            }
        }
        (revision, items.len())
    };
    Ok((definition, revision, item_count))
}

/// 拒绝覆盖已有路径（独评 F2）：不提供重置建议，也不自动清理；换新路径是唯一去处。
fn existing_db_error(db: &Path) -> String {
    format!(
        "库已存在（{}）：{PROTOCOL_REVISION} init 拒绝覆盖（无热迁移，也不提供重置；请换新路径）",
        db.display()
    )
}

/// 回读一个 meta 值；缺失即报错，不回落到参数。
fn read_meta(meta: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    meta.get(key)
        .cloned()
        .ok_or_else(|| format!("StorageUnavailable：初始化回读缺 meta.{key}"))
}

/// 回读并核对：返回**库内实际值**，与预期不符即报错（独评 F3：回执不得拿参数冒充回读）。
fn expect_meta(
    meta: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<String, String> {
    let actual = read_meta(meta, key)?;
    if actual != expected {
        return Err(format!(
            "StorageUnavailable：初始化回读 meta.{key}=`{actual}` 与预期 `{expected}` 不一致"
        ));
    }
    Ok(actual)
}

/// 在全新库上建 `s-session/3` 域；返回从已提交库回读的真实初始化元数据。
pub(super) fn init_v3(
    db: &Path,
    session: &str,
    catalog_path: &Path,
    profile_path: &Path,
    session_generation: &str,
    clock_path: &Path,
    delivery_retain: i64,
) -> Result<Value, String> {
    // ── 第一步：全部输入先验证；此阶段一个字节都不落盘 ──
    positive(session_generation, "session-generation")?;
    if delivery_retain < 1 {
        return Err("InvalidDomain：delivery retain 必须大于0".into());
    }
    let clock = v2::ClockPlan::load(clock_path)?;
    let (profile_def, profile_id, profile_hash) = load_profile(profile_path)?;
    let (catalog_def, catalog_revision, catalog_items) = load_catalog_strict(catalog_path)?;
    let catalog_hash = sha256_hex(canonical_json(&catalog_def).as_bytes());

    // ── 第二步：既有库一律拒绝；与既有初始化共用同一把域进程锁，锁后复查防并发双建 ──
    if db.exists() {
        return Err(existing_db_error(db));
    }
    let _lock = session_transport::process_lock(db, "S")?;
    if db.exists() {
        return Err(existing_db_error(db));
    }
    let mut conn = open_db(db)?;
    let sqlite_version: String = conn
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .map_err(|e| format!("读 sqlite_version 失败：{e}"))?;
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(|e| format!("读 journal_mode 失败：{e}"))?;
    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(|e| format!("读 synchronous 失败：{e}"))?;

    // ── 第三步：同一事务建共同 authority 表、v2 控制表、内容节点表并登记本域 ──
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| format!("开 init 事务失败：{e}"))?;
    tx.execute_batch(SCHEMA)
        .map_err(|e| format!("建共同 S authority 表失败：{e}"))?;
    tx.execute_batch(v2::SCHEMA_V2)
        .map_err(|e| format!("建 v2 控制表失败：{e}"))?;
    if profile_id == tb02a::PROFILE {
        tx.execute_batch(tb02a::SCHEMA)
            .map_err(|e| format!("建 tb02a 表失败：{e}"))?;
    }
    meta_set(&tx, "session_id", session)?;
    meta_set(&tx, "generation", "0")?;
    meta_set(&tx, "structure_cut", "cut-0")?;
    meta_set(&tx, "index_frontier", "")?;
    meta_set(&tx, "last_advance_frontier", "-1")?;
    meta_set(&tx, "sqlite_version", &sqlite_version)?;
    meta_set(&tx, "journal_mode", &journal_mode)?;
    meta_set(&tx, "synchronous", &synchronous.to_string())?;
    meta_set(&tx, "platform", std::env::consts::OS)?;
    meta_set(&tx, "writer_epoch", DEFAULT_WRITER_EPOCH)?;
    meta_set(&tx, "advance_state", "idle")?;
    meta_set(&tx, "rule_revision", RULE_REVISION)?;
    meta_set(
        &tx,
        "scope",
        &json!({"structure": "CompleteCut", "economic": "not_started"}).to_string(),
    )?;
    meta_set(&tx, "protocol_revision", PROTOCOL_REVISION)?;
    meta_set(&tx, "content_format", s_content::CONTENT_FORMAT)?;
    meta_set(&tx, "profile_id", &profile_id)?;
    meta_set(&tx, "profile_hash", &profile_hash)?;
    meta_set(&tx, "input_profile", &profile_id)?;
    meta_set(&tx, "profile_definition", &canonical_json(&profile_def))?;
    meta_set(&tx, "catalog_revision", &catalog_revision)?;
    meta_set(&tx, "catalog_definition", &canonical_json(&catalog_def))?;
    meta_set(&tx, "catalog_hash", &catalog_hash)?;
    tx.execute(
        "INSERT INTO s_protocol_meta(singleton, protocol_revision, session_generation, clock_plan_hash,
             clock_plan_json, next_attempt_ordinal, logical_phase_frontier)
         VALUES(1, ?1, ?2, ?3, ?4, 1, '-1')",
        params![
            PROTOCOL_REVISION,
            session_generation,
            clock.hash,
            canonical_json(&clock.value)
        ],
    )
    .map_err(|e| format!("写 s_protocol_meta 失败：{e}"))?;
    tx.execute(
        "INSERT INTO s_delivery_policy(singleton, session_id, session_generation, policy_revision,
             retain_generations, first_available_generation, head_generation)
         VALUES(1, ?1, ?2, ?3, ?4, 1, 0)",
        params![
            session,
            session_generation,
            DELIVERY_POLICY_REVISION,
            delivery_retain
        ],
    )
    .map_err(|e| format!("写 s_delivery_policy 失败：{e}"))?;
    content_store::init_schema(&tx).map_err(|e| format!("建 s-content/1 节点存储失败：{e}"))?;
    commit::init_schema(&tx)?;
    aux_index::init_schema(&tx)?;
    object_index::init_schema(&tx)?;
    {
        let items = catalog_def
            .get("items")
            .and_then(Value::as_array)
            .ok_or("SchemaUnsupported：catalog 缺 items 数组")?;
        let mut stmt = tx
            .prepare(
                "INSERT OR REPLACE INTO catalog(catalog_id, kind, title, domain, branches_json,
                     impl_status, proof_status, run_status, evidence_json)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .map_err(|e| format!("prepare catalog 失败：{e}"))?;
        for item in items {
            let id = item
                .get("id")
                .and_then(Value::as_str)
                .ok_or("SchemaUnsupported：catalog 项缺 id")?;
            let kind = item
                .get("kind")
                .and_then(Value::as_str)
                .ok_or("SchemaUnsupported：catalog 项缺 kind")?;
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .ok_or("SchemaUnsupported：catalog 项缺 title")?;
            let domain = item
                .get("domain")
                .and_then(Value::as_str)
                .ok_or("SchemaUnsupported：catalog 项缺 domain")?;
            let branches = item
                .get("branches")
                .ok_or("SchemaUnsupported：catalog 项缺 branches")?
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
    tx.commit().map_err(|e| format!("init commit 失败：{e}"))?;

    // ── 第四步：回读已提交状态，逐项核对后返回（独评 F3：返回字段全部来自本次回读） ──
    let meta = db_meta(&conn)?;
    let (
        stored_protocol,
        stored_generation,
        stored_clock_hash,
        stored_clock_json,
        next_ordinal,
        phase_frontier,
    ): (String, String, String, String, i64, String) = conn
        .query_row(
            "SELECT protocol_revision, session_generation, clock_plan_hash, clock_plan_json,
                    next_attempt_ordinal, logical_phase_frontier
             FROM s_protocol_meta WHERE singleton=1",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .map_err(|e| format!("回读 s_protocol_meta 失败：{e}"))?;
    let (policy_revision, retained, first_available, head): (String, i64, i64, i64) = conn
        .query_row(
            "SELECT policy_revision, retain_generations, first_available_generation, head_generation
             FROM s_delivery_policy WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| format!("回读 s_delivery_policy 失败：{e}"))?;
    let content_format: String = conn
        .query_row(
            "SELECT content_format FROM s_content_meta WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("回读 s_content_meta 失败：{e}"))?;
    let stored_catalog_items: i64 = conn
        .query_row("SELECT COUNT(*) FROM catalog", [], |r| r.get(0))
        .map_err(|e| format!("回读 catalog 行数失败：{e}"))?;
    let session_id = expect_meta(&meta, "session_id", session)?;
    expect_meta(&meta, "protocol_revision", PROTOCOL_REVISION)?;
    expect_meta(&meta, "content_format", &content_format)?;
    let generation = expect_meta(&meta, "generation", "0")?;
    let structure_cut = expect_meta(&meta, "structure_cut", "cut-0")?;
    let index_frontier = expect_meta(&meta, "index_frontier", "")?;
    let last_advance_frontier = expect_meta(&meta, "last_advance_frontier", "-1")?;
    let writer_epoch = expect_meta(&meta, "writer_epoch", DEFAULT_WRITER_EPOCH)?;
    let platform = expect_meta(&meta, "platform", std::env::consts::OS)?;
    let profile_id = expect_meta(&meta, "profile_id", &profile_id)?;
    let profile_hash = expect_meta(&meta, "profile_hash", &profile_hash)?;
    let catalog_revision = expect_meta(&meta, "catalog_revision", &catalog_revision)?;
    let catalog_hash = expect_meta(&meta, "catalog_hash", &catalog_hash)?;
    let sqlite_version = expect_meta(&meta, "sqlite_version", &sqlite_version)?;
    let journal_mode = expect_meta(&meta, "journal_mode", &journal_mode)?;
    let synchronous = expect_meta(&meta, "synchronous", &synchronous.to_string())?
        .parse::<i64>()
        .map_err(|e| format!("StorageUnavailable：回读 meta.synchronous 非整数：{e}"))?;
    if stored_protocol != PROTOCOL_REVISION {
        return Err(format!(
            "StorageUnavailable：回读 s_protocol_meta.protocol_revision=`{stored_protocol}` 不是 {PROTOCOL_REVISION}"
        ));
    }
    if stored_generation != session_generation {
        return Err(format!(
            "StorageUnavailable：回读 session_generation=`{stored_generation}` 与 `{session_generation}` 不一致"
        ));
    }
    if stored_clock_hash != clock.hash {
        return Err("StorageUnavailable：回读 clock_plan_hash 与本次 clock 计划不一致".into());
    }
    let stored_clock_def = strict_json(stored_clock_json.as_bytes())?;
    if canonical_json(&stored_clock_def) != stored_clock_json || stored_clock_def != clock.value {
        return Err("StorageUnavailable：回读 clock_plan_json 与本次 clock 计划不一致".into());
    }
    if next_ordinal != 1 || phase_frontier != "-1" {
        return Err(format!(
            "StorageUnavailable：回读 s_protocol_meta 初态异常（next_attempt_ordinal={next_ordinal}, logical_phase_frontier=`{phase_frontier}`）"
        ));
    }
    if policy_revision != DELIVERY_POLICY_REVISION
        || retained != delivery_retain
        || first_available != 1
        || head != 0
    {
        return Err(format!(
            "StorageUnavailable：回读 s_delivery_policy 初态异常（revision=`{policy_revision}`, retain={retained}, first={first_available}, head={head}）"
        ));
    }
    if content_format != s_content::CONTENT_FORMAT {
        return Err(format!(
            "StorageUnavailable：s_content_meta=`{content_format}` 不是 {}",
            s_content::CONTENT_FORMAT
        ));
    }
    if stored_catalog_items != catalog_items as i64 {
        return Err(format!(
            "StorageUnavailable：回读 catalog 行数={stored_catalog_items} 与 items 定义={catalog_items} 不符"
        ));
    }
    Ok(json!({
        "ok": true,
        "session_id": session_id,
        "protocol_revision": stored_protocol,
        "content_format": content_format,
        "session_generation": stored_generation,
        "clock_plan_hash": stored_clock_hash,
        "profile_id": profile_id,
        "profile_hash": profile_hash,
        "catalog_revision": catalog_revision,
        "catalog_hash": catalog_hash,
        "catalog_items": stored_catalog_items,
        "generation": generation,
        "structure_cut": structure_cut,
        "index_frontier": index_frontier,
        "last_advance_frontier": last_advance_frontier,
        "delivery_retain_generations": retained,
        "writer_epoch": writer_epoch,
        "sqlite_version": sqlite_version,
        "journal_mode": journal_mode,
        "synchronous": synchronous,
        "platform": platform,
    }))
}

fn reset_refusal(db: &Path, detail: &str) -> String {
    format!(
        "S3DomainResetRefused：reset 拒绝删除 {}（{detail}）；旧 reset 只懂 v1/v2 合同，域不可确定即 fail closed，本切片不提供新迁移/重置",
        db.display()
    )
}

/// 独评 F1 二次修复：旧 `reset`（先删后建 v1）对 `s-session/3` 域是破坏性迁移，判定必须在**任何
/// 删除之前**完成：只读打开、**不做写打开回退**；读不出/锁冲突/非 SQLite/缺表等无法确定域的情形
/// 一律拒绝。证据三路：`meta.protocol_revision`、`s_protocol_meta.protocol_revision`、
/// `s_content_meta`/内容表线索。任一指向 `s-session/3`/`s-content/1`、两处标记冲突或不完整、出现
/// 不认识的标记值，都拒绝；只有「meta 表在、无协议标记、无控制表、无内容表」的 v1 与「两处标记都
/// 恰为 `s-session/2`」的 v2 放行。缺 `meta` 表（空文件/半建库/外来库）不可判定，同样拒绝——
/// 不保留「损坏库可直接删」的旧偶发行为。本切片不实现新迁移/重置功能。
pub(super) fn refuse_reset_of_new_domain(db: &Path) -> Result<(), String> {
    if !db.exists() {
        return Ok(());
    }
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| reset_refusal(db, &format!("只读打开失败：{e}")))?;
    let tables: BTreeSet<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type='table'")
            .map_err(|e| reset_refusal(db, &format!("读 sqlite_schema 失败：{e}")))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| reset_refusal(db, &format!("读 sqlite_schema 失败：{e}")))?;
        rows.collect::<Result<BTreeSet<_>, _>>()
            .map_err(|e| reset_refusal(db, &format!("读 sqlite_schema 失败：{e}")))?
    };
    if !tables.contains("meta") {
        return Err(reset_refusal(
            db,
            "缺 meta 表（空文件/半建库/外来库，域不可判定）",
        ));
    }
    let content_tables: Vec<&str> = [
        "s_content_meta",
        "s_content_nodes",
        "s_content_publications",
        "s_content_witnesses",
        "s_content_relations",
        "s_content_observations",
        "s_content_objects",
    ]
        .into_iter()
        .filter(|name| tables.contains(*name))
        .collect();
    let content_format = if tables.contains("s_content_meta") {
        conn.query_row(
            "SELECT content_format FROM s_content_meta WHERE singleton=1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| reset_refusal(db, &format!("读 s_content_meta 失败：{e}")))?
    } else {
        None
    };
    let meta_marker = conn
        .query_row(
            "SELECT value FROM meta WHERE key='protocol_revision'",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| reset_refusal(db, &format!("读 meta 协议标记失败：{e}")))?;
    // 控制表存在就必须有唯一协议行：存在而缺行是「不完整」，不按「没有标记」放行。
    let table_marker = if tables.contains("s_protocol_meta") {
        Some(
            conn.query_row(
                "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| reset_refusal(db, &format!("读 s_protocol_meta 协议标记失败：{e}")))?
            .ok_or_else(|| {
                reset_refusal(db, "s_protocol_meta 存在但缺 singleton 协议行，域不可判定")
            })?,
        )
    } else {
        None
    };
    if !content_tables.is_empty() || content_format.as_deref() == Some(s_content::CONTENT_FORMAT) {
        return Err(reset_refusal(
            db,
            &format!(
                "存在 s-content/1 内容表线索（tables={content_tables:?}, format={content_format:?}）"
            ),
        ));
    }
    match (meta_marker.as_deref(), table_marker.as_deref()) {
        (None, None) => Ok(()),
        (Some(PREVIOUS_REVISION), Some(PREVIOUS_REVISION)) => Ok(()),
        (Some(meta), Some(table)) => Err(reset_refusal(
            db,
            &format!(
                "协议标记组合不可判定为旧 v1/v2 域（meta=`{meta}`, s_protocol_meta=`{table}`）"
            ),
        )),
        (meta, table) => Err(reset_refusal(
            db,
            &format!("协议标记不完整（meta={meta:?}, s_protocol_meta={table:?}）"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::{NodeRead, NodeSource};
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    const OHLC_PROFILE: &str = "profiles/ohlc_integer_tb02a_v1.json";
    const CATALOG: &str = "catalog/signed-catalog.json";

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("s1394-v3-{}-{stamp}-{number}", std::process::id()));
            std::fs::create_dir(&dir).unwrap();
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Temp {
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

    fn clock_value(unit: &str) -> Value {
        json!({
            "schema_revision": "s-clock-plan/1",
            "clock_plan_id": "s3-domain-test-clock",
            "origin_utc": "2026-09-18T00:00:00Z",
            "unit": unit,
            "events": {"op-1": {"accept_ns": "10",
                "attempts": [{"begin_ns": "11", "commit_ns": "12"}]}},
        })
    }

    fn write_clock(dir: &Temp, unit: &str) -> PathBuf {
        let path = dir.path("clock.json");
        std::fs::write(&path, canonical_json(&clock_value(unit))).unwrap();
        path
    }

    fn init_args(db: &Path, clock: &Path) -> Vec<String> {
        vec![
            "s_structure_session".to_string(),
            "init".to_string(),
            "--db".to_string(),
            db.display().to_string(),
            "--session".to_string(),
            "s3-testonly-001".to_string(),
            "--catalog".to_string(),
            fixture(CATALOG).display().to_string(),
            "--profile".to_string(),
            fixture(OHLC_PROFILE).display().to_string(),
            "--session-generation".to_string(),
            "1".to_string(),
            "--clock-plan".to_string(),
            clock.display().to_string(),
            "--delivery-retain-generations".to_string(),
            "4".to_string(),
        ]
    }

    fn with_selector(args: &[String], selector: &str) -> Vec<String> {
        let mut out = args.to_vec();
        out.push(SELECTOR.to_string());
        out.push(selector.to_string());
        out
    }

    fn without(args: &[String], key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut index = 0;
        while index < args.len() {
            if args[index] == key {
                index += 2;
                continue;
            }
            out.push(args[index].clone());
            index += 1;
        }
        out
    }

    /// 只删掉 key 的**值** token，保留 key 本身，复现「下一个 flag 被当成值」的入口。
    fn without_value(args: &[String], key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut index = 0;
        while index < args.len() {
            if args[index] == key {
                out.push(args[index].clone());
                index += 2;
                continue;
            }
            out.push(args[index].clone());
            index += 1;
        }
        out
    }

    fn init_domain(dir: &Temp) -> PathBuf {
        let db = dir.path("s3.sqlite");
        let clock = write_clock(dir, "ns");
        init_v3(
            &db,
            "s3-testonly-001",
            &fixture(CATALOG),
            &fixture(OHLC_PROFILE),
            "1",
            &clock,
            4,
        )
        .unwrap();
        db
    }

    fn table_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
            .unwrap();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
        rows.collect::<Result<Vec<_>, _>>().unwrap()
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn v3_init_builds_the_isolated_domain_and_reopens_with_content_nodes() {
        let temp = Temp::new();
        let db = temp.path("s3.sqlite");
        let clock_path = write_clock(&temp, "ns");
        let out = init_v3(
            &db,
            "s3-testonly-001",
            &fixture(CATALOG),
            &fixture(OHLC_PROFILE),
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        assert_eq!(out["ok"], json!(true));
        assert_eq!(out["protocol_revision"], json!(PROTOCOL_REVISION));
        assert_eq!(out["content_format"], json!("s-content/1"));
        assert_eq!(out["session_generation"], json!("1"));
        assert_eq!(out["generation"], json!("0"));
        assert_eq!(out["structure_cut"], json!("cut-0"));
        assert_eq!(out["index_frontier"], json!(""));
        assert_eq!(out["last_advance_frontier"], json!("-1"));
        assert_eq!(out["writer_epoch"], json!(DEFAULT_WRITER_EPOCH));
        let clock_def = strict_json(&std::fs::read(&clock_path).unwrap()).unwrap();
        let clock_hash = sha256_hex(canonical_json(&clock_def).as_bytes());
        assert_eq!(out["clock_plan_hash"], json!(clock_hash));
        let catalog_def = strict_json(&std::fs::read(fixture(CATALOG)).unwrap()).unwrap();
        let items = catalog_def["items"].as_array().unwrap().len();
        assert_eq!(out["catalog_items"], json!(items));
        assert_eq!(
            count(&open_db(&db).unwrap(), "catalog"),
            i64::try_from(items).unwrap()
        );

        let conn = open_db(&db).unwrap();
        let tables = table_names(&conn);
        for expected in [
            "meta",
            "structure_deltas",
            "s_protocol_meta",
            "s_input_messages",
            "s_clock_events",
            "s_delivery_policy",
            "s_delivery_refs",
            "s_content_meta",
            "s_content_nodes",
            "s_content_publications",
            "s_content_witnesses",
            "s_content_relations",
            "s_content_observations",
            "raw_ohlc",
            "structure_facts",
        ] {
            assert!(
                tables.iter().any(|name| name == expected),
                "缺表 {expected}：{tables:?}"
            );
        }
        // 新域不登记旧 private 格式，也不写旧批次/旧控制面。
        for absent in [
            delta_storage::TABLE,
            evidence_store::FORMAT_TABLE,
            evidence_store::NODES_TABLE,
            evidence_store::ROOTS_TABLE,
        ] {
            assert!(
                !tables.iter().any(|name| name == absent),
                "新域不得出现旧格式表 {absent}：{tables:?}"
            );
        }
        for empty in [
            "s_content_publications",
            "s_content_witnesses",
            "s_content_relations",
            "s_content_observations",
            "raw_events",
            "batches",
            "objects",
            "witnesses",
            "relations",
            "observations",
            "structure_deltas",
            "s_input_messages",
            "s_clock_events",
            "s_delivery_refs",
        ] {
            assert_eq!(count(&conn, empty), 0, "{empty} 必须为空");
        }
        assert_eq!(
            meta_get_opt(&conn, "protocol_revision").unwrap().as_deref(),
            Some(PROTOCOL_REVISION)
        );
        assert_eq!(
            meta_get_opt(&conn, "profile_id").unwrap().as_deref(),
            Some(tb02a::PROFILE)
        );
        let (protocol, generation, clock_hash, clock_json, next, frontier): (
            String,
            String,
            String,
            String,
            i64,
            String,
        ) = conn
            .query_row(
                "SELECT protocol_revision, session_generation, clock_plan_hash, clock_plan_json,
                        next_attempt_ordinal, logical_phase_frontier
                 FROM s_protocol_meta WHERE singleton=1",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(protocol, PROTOCOL_REVISION);
        assert_eq!(generation, "1");
        assert_eq!(clock_hash, out["clock_plan_hash"].as_str().unwrap());
        assert_eq!(clock_json, canonical_json(&clock_def));
        assert_eq!(next, 1);
        assert_eq!(frontier, "-1");
        let (policy, retain, first, head): (String, i64, i64, i64) = conn
            .query_row(
                "SELECT policy_revision, retain_generations, first_available_generation, head_generation
                 FROM s_delivery_policy WHERE singleton=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(policy, DELIVERY_POLICY_REVISION);
        assert_eq!((retain, first, head), (4, 1, 0));
        // F3：回执里的初态值必须与库内实际行逐项相等。
        assert_eq!(out["delivery_retain_generations"], json!(retain));
        assert_eq!(
            out["index_frontier"],
            json!(meta_get_opt(&conn, "index_frontier").unwrap().unwrap())
        );
        assert_eq!(
            out["writer_epoch"],
            json!(meta_get_opt(&conn, "writer_epoch").unwrap().unwrap())
        );
        assert_eq!(
            out["platform"],
            json!(meta_get_opt(&conn, "platform").unwrap().unwrap())
        );
        drop(conn);

        // 重开：内容节点在同一事务写入/读出，提交后再次重开仍可读出。
        let mut conn = open_db(&db).unwrap();
        let built = {
            let tx = conn.transaction().unwrap();
            let mut store = content_store::TxStore::new(&tx).unwrap();
            let built = s_content::write_value(
                &mut store,
                &s_content::Value::Array(vec![s_content::Value::String("S3-domain".into())]),
            )
            .unwrap();
            s_content::verify(&mut store, &built.content_ref()).unwrap();
            assert!(matches!(
                store.read(&built.digest).unwrap(),
                NodeRead::Found { .. }
            ));
            tx.commit().unwrap();
            built
        };
        drop(conn);
        let mut conn = open_db(&db).unwrap();
        let tx = conn.transaction().unwrap();
        let mut reader = content_store::ReadStore::new(&tx).unwrap();
        let mut rendered = Vec::new();
        s_content::render_canonical(&mut reader, &built.content_ref(), &mut rendered).unwrap();
        assert_eq!(rendered, b"[\"S3-domain\"]");
    }

    #[test]
    fn v3_init_refuses_existing_domains_without_touching_bytes() {
        let temp = Temp::new();
        let db = init_domain(&temp);
        let before = std::fs::read(&db).unwrap();
        let clock = write_clock(&temp, "ns");
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &fixture(CATALOG),
            &fixture(OHLC_PROFILE),
            "1",
            &clock,
            4,
        )
        .is_err());
        assert_eq!(std::fs::read(&db).unwrap(), before);

        // 既有 v2 旧库同样拒绝（无热迁移）。
        let old = temp.path("v2.sqlite");
        assert!(v2::dispatch(&init_args(&old, &clock)).unwrap());
        let old_before = std::fs::read(&old).unwrap();
        assert!(init_v3(
            &old,
            "s3-testonly-001",
            &fixture(CATALOG),
            &fixture(OHLC_PROFILE),
            "1",
            &clock,
            4,
        )
        .is_err());
        assert_eq!(std::fs::read(&old).unwrap(), old_before);
        let conn = open_db(&old).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "protocol_revision").unwrap().as_deref(),
            Some(PREVIOUS_REVISION)
        );
    }

    #[test]
    fn selector_dispatch_rejects_unknown_duplicate_and_malformed_forms() {
        let temp = Temp::new();
        let clock = write_clock(&temp, "ns");
        let db = temp.path("selector.sqlite");
        let plain = init_args(&db, &clock);
        assert!(!dispatch(&plain).unwrap());

        let unknown = with_selector(&plain, "s-session/9");
        assert!(dispatch(&unknown).is_err());
        assert!(!db.exists(), "未知版本不得创建库");

        let mut duplicate = with_selector(&plain, PROTOCOL_REVISION);
        duplicate.extend([SELECTOR.to_string(), PREVIOUS_REVISION.to_string()]);
        assert!(dispatch(&duplicate).is_err());
        assert!(!db.exists(), "重复选择器不得创建库");

        let mut malformed = plain.clone();
        malformed.push(format!("{SELECTOR}={PROTOCOL_REVISION}"));
        assert!(dispatch(&malformed).is_err());
        assert!(!db.exists(), "变形选择器不得创建库");

        let mut missing_value = plain.clone();
        missing_value.push(SELECTOR.to_string());
        assert!(dispatch(&missing_value).is_err());
        assert!(!db.exists(), "缺选择器值不得创建库");

        let mut non_init = with_selector(&plain, PROTOCOL_REVISION);
        non_init[1] = "advance".to_string();
        assert!(dispatch(&non_init).is_err());
        assert!(!db.exists(), "未接线命令带选择器不得创建库");

        let incomplete = with_selector(&without(&plain, "--clock-plan"), PREVIOUS_REVISION);
        assert!(dispatch(&incomplete).is_err());
        assert!(!db.exists(), "显式 s-session/2 缺参不得回落旧域");
    }

    #[test]
    fn v3_init_requires_complete_key_value_pairs_before_creating_anything() {
        let temp = Temp::new();
        let clock = write_clock(&temp, "ns");

        // `--session --catalog <path>`：flag 后紧跟另一个 flag，必须按缺值拒绝。
        let db = temp.path("missing-value.sqlite");
        let args = with_selector(
            &without_value(&init_args(&db, &clock), "--session"),
            PROTOCOL_REVISION,
        );
        assert!(
            dispatch(&args).is_err(),
            "flag 后接另一 flag 必须按缺值拒绝"
        );
        assert!(!db.exists(), "缺值不得创建库");

        let db = temp.path("duplicate-key.sqlite");
        let mut args = with_selector(&init_args(&db, &clock), PROTOCOL_REVISION);
        args.extend(["--session".to_string(), "s3-second".to_string()]);
        assert!(dispatch(&args).is_err(), "重复 key 必须拒绝");
        assert!(!db.exists(), "重复 key 不得创建库");

        let db = temp.path("orphan.sqlite");
        let mut args = with_selector(&init_args(&db, &clock), PROTOCOL_REVISION);
        args.push("bare-token".to_string());
        assert!(dispatch(&args).is_err(), "孤立 token 必须拒绝");
        assert!(!db.exists(), "孤立 token 不得创建库");

        let db = temp.path("malformed.sqlite");
        let mut args = with_selector(&init_args(&db, &clock), PROTOCOL_REVISION);
        args.push("--db=other.sqlite".to_string());
        assert!(dispatch(&args).is_err(), "变形 flag 必须拒绝");
        assert!(!db.exists(), "变形 flag 不得创建库");

        let db = temp.path("empty-value.sqlite");
        let mut args = with_selector(&init_args(&db, &clock), PROTOCOL_REVISION);
        let index = args.iter().position(|a| a == "--session").unwrap();
        args[index + 1] = String::new();
        assert!(dispatch(&args).is_err(), "空值必须拒绝");
        assert!(!db.exists(), "空值不得创建库");
    }

    #[test]
    fn reset_route_refuses_new_domain_and_keeps_legacy_paths() {
        let temp = Temp::new();
        let catalog = fixture(CATALOG);
        let db = init_domain(&temp);
        let before = std::fs::read(&db).unwrap();
        let refused = cmd_reset(&db, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        assert_eq!(
            std::fs::read(&db).unwrap(),
            before,
            "被拒的 reset 不得改动 S3 库"
        );
        let conn = open_db(&db).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "protocol_revision").unwrap().as_deref(),
            Some(PROTOCOL_REVISION)
        );
        assert_eq!(
            meta_get_opt(&conn, "session_id").unwrap().as_deref(),
            Some("s3-testonly-001")
        );
        drop(conn);

        // v1 旧域：reset 仍是「先删后建」，行为不变。
        let v1 = temp.path("v1.sqlite");
        cmd_init(&v1, "v1-before", &catalog).unwrap();
        cmd_reset(&v1, "v1-after", &catalog).unwrap();
        let conn = open_db(&v1).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "session_id").unwrap().as_deref(),
            Some("v1-after")
        );
        assert!(meta_get_opt(&conn, "protocol_revision").unwrap().is_none());
        drop(conn);

        // v2 旧域：reset 仍沿旧行为（先删后建 v1），不因本次加固改变。
        let v2_db = temp.path("v2-old.sqlite");
        let clock = write_clock(&temp, "ns");
        assert!(v2::dispatch(&init_args(&v2_db, &clock)).unwrap());
        cmd_reset(&v2_db, "v2-reset", &catalog).unwrap();
        let conn = open_db(&v2_db).unwrap();
        assert!(meta_get_opt(&conn, "protocol_revision").unwrap().is_none());
        assert_eq!(
            meta_get_opt(&conn, "session_id").unwrap().as_deref(),
            Some("v2-reset")
        );
    }

    #[test]
    fn reset_refuses_tampered_or_partial_new_domain_markers() {
        let temp = Temp::new();
        let catalog = fixture(CATALOG);
        let clock = write_clock(&temp, "ns");

        // (a) 删掉 meta.protocol_revision，s_protocol_meta 仍登记 3：不得降级放行。
        let db = init_domain(&temp);
        {
            let conn = open_db(&db).unwrap();
            conn.execute("DELETE FROM meta WHERE key='protocol_revision'", [])
                .unwrap();
        }
        let before = std::fs::read(&db).unwrap();
        let refused = cmd_reset(&db, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        assert_eq!(std::fs::read(&db).unwrap(), before);
        {
            let conn = open_db(&db).unwrap();
            assert!(meta_get_opt(&conn, "protocol_revision").unwrap().is_none());
            let table_marker: String = conn
                .query_row(
                    "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(table_marker, PROTOCOL_REVISION);
        }

        // (b) 主标记改成 2，但 s_protocol_meta 与内容表仍指向 3：冲突即拒绝。
        let db = temp.path("tampered-marker.sqlite");
        init_v3(
            &db,
            "s3-tampered",
            &catalog,
            &fixture(OHLC_PROFILE),
            "1",
            &clock,
            4,
        )
        .unwrap();
        {
            let conn = open_db(&db).unwrap();
            conn.execute(
                "UPDATE meta SET value='s-session/2' WHERE key='protocol_revision'",
                [],
            )
            .unwrap();
        }
        let before = std::fs::read(&db).unwrap();
        let refused = cmd_reset(&db, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        assert_eq!(std::fs::read(&db).unwrap(), before);

        // (c) 两处协议标记都删掉，只剩内容表：仍拒绝。
        {
            let conn = open_db(&db).unwrap();
            conn.execute("DELETE FROM meta WHERE key='protocol_revision'", [])
                .unwrap();
            conn.execute("DELETE FROM s_protocol_meta", []).unwrap();
        }
        let before = std::fs::read(&db).unwrap();
        let refused = cmd_reset(&db, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        assert_eq!(std::fs::read(&db).unwrap(), before);
    }

    #[test]
    fn reset_refuses_undecidable_files_and_keeps_bytes() {
        let temp = Temp::new();
        let catalog = fixture(CATALOG);

        // 非 SQLite 文件：读不出域 → 拒绝且字节不变。
        let foreign = temp.path("foreign.sqlite");
        std::fs::write(&foreign, b"not a sqlite database").unwrap();
        let before = std::fs::read(&foreign).unwrap();
        let refused = cmd_reset(&foreign, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        assert_eq!(std::fs::read(&foreign).unwrap(), before);

        // 空文件（半建库形态，无 meta 表）：不可判定 → 拒绝且不删。
        let empty = temp.path("empty.sqlite");
        std::fs::write(&empty, b"").unwrap();
        let refused = cmd_reset(&empty, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        assert!(empty.exists(), "被拒的 reset 不得删除原文件");
        assert!(std::fs::read(&empty).unwrap().is_empty());

        // 权限位 0：只读打开失败 → 拒绝且字节不变（比对前恢复权限）。
        let unreadable = temp.path("unreadable.sqlite");
        std::fs::write(&unreadable, b"no permission to read").unwrap();
        let before = std::fs::read(&unreadable).unwrap();
        let mut perms = std::fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&unreadable, perms).unwrap();
        let refused = cmd_reset(&unreadable, "reset-probe", &catalog).unwrap_err();
        assert!(refused.contains("S3DomainResetRefused"), "{refused}");
        let mut perms = std::fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&unreadable, perms).unwrap();
        assert_eq!(std::fs::read(&unreadable).unwrap(), before);
    }

    #[test]
    fn selectors_route_to_the_matching_contract() {
        let temp = Temp::new();
        let clock = write_clock(&temp, "ns");

        // 无选择器：分派保持原入口行为（v2 参数齐全时进 v2）。
        let plain_db = temp.path("plain.sqlite");
        let plain = init_args(&plain_db, &clock);
        assert!(!dispatch(&plain).unwrap());
        assert!(v2::dispatch(&plain).unwrap());
        let conn = open_db(&plain_db).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "protocol_revision").unwrap().as_deref(),
            Some(PREVIOUS_REVISION)
        );
        drop(conn);

        // 显式 s-session/3：新域。
        let v3_db = temp.path("v3.sqlite");
        let v3_args = with_selector(&init_args(&v3_db, &clock), PROTOCOL_REVISION);
        assert!(dispatch(&v3_args).unwrap());
        let conn = open_db(&v3_db).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "protocol_revision").unwrap().as_deref(),
            Some(PROTOCOL_REVISION)
        );
        assert_eq!(count(&conn, "s_content_nodes"), 0, "init 不应写内容节点");
        drop(conn);

        // 显式 s-session/2：仍走旧 v2 初始化。
        let v2_db = temp.path("explicit-v2.sqlite");
        let v2_args = with_selector(&init_args(&v2_db, &clock), PREVIOUS_REVISION);
        assert!(dispatch(&v2_args).unwrap());
        let conn = open_db(&v2_db).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "protocol_revision").unwrap().as_deref(),
            Some(PREVIOUS_REVISION)
        );
        let old_tables = table_names(&conn);
        assert!(old_tables.iter().any(|name| name == delta_storage::TABLE));
    }

    #[test]
    fn reset_does_not_delete_a_database_owned_by_another_writer() {
        let temp = Temp::new();
        let db = temp.path("locked-reset.sqlite");
        init_core(&db, "original-session", &fixture(CATALOG)).unwrap();
        let before = std::fs::read(&db).unwrap();
        let _writer = session_transport::process_lock(&db, "S").unwrap();
        assert!(cmd_reset(&db, "replacement-session", &fixture(CATALOG)).is_err());
        assert_eq!(std::fs::read(&db).unwrap(), before);
    }

    #[test]
    fn bad_inputs_are_rejected_before_any_database_is_created() {
        let temp = Temp::new();
        let good_clock = write_clock(&temp, "ns");

        let bad_clock = write_clock(&temp, "s");
        let db = temp.path("bad-clock.sqlite");
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &fixture(CATALOG),
            &fixture(OHLC_PROFILE),
            "1",
            &bad_clock,
            4,
        )
        .is_err());
        assert!(!db.exists(), "坏 clock 必须在建库前拒绝");

        let bad_profile = temp.path("bad-profile.json");
        std::fs::write(
            &bad_profile,
            json!({"profile_id": tb02a::PROFILE}).to_string(),
        )
        .unwrap();
        let db = temp.path("bad-profile.sqlite");
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &fixture(CATALOG),
            &bad_profile,
            "1",
            &good_clock,
            4,
        )
        .is_err());
        assert!(!db.exists(), "坏 profile 必须在建库前拒绝");

        let no_revision = temp.path("catalog-no-revision.json");
        std::fs::write(
            &no_revision,
            json!({"items": [{"id": "CC-001", "kind": "k", "title": "t", "domain": "d",
                "branches": []}]})
            .to_string(),
        )
        .unwrap();
        let db = temp.path("bad-catalog-revision.sqlite");
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &no_revision,
            &fixture(OHLC_PROFILE),
            "1",
            &good_clock,
            4,
        )
        .is_err());
        assert!(!db.exists(), "缺 catalog_revision 不得默认 unknown 建库");

        let duplicate = temp.path("catalog-duplicate.json");
        std::fs::write(
            &duplicate,
            json!({"catalog_revision": "dup/1", "items": [
                {"id": "CC-001", "kind": "k", "title": "t", "domain": "d", "branches": []},
                {"id": "CC-001", "kind": "k", "title": "t", "domain": "d", "branches": []}]})
            .to_string(),
        )
        .unwrap();
        let db = temp.path("bad-catalog-duplicate.sqlite");
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &duplicate,
            &fixture(OHLC_PROFILE),
            "1",
            &good_clock,
            4,
        )
        .is_err());
        assert!(!db.exists(), "重复目录身份必须在建库前拒绝");

        let broken = temp.path("catalog-broken.json");
        std::fs::write(&broken, b"{\"items\": [").unwrap();
        let db = temp.path("bad-catalog-json.sqlite");
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &broken,
            &fixture(OHLC_PROFILE),
            "1",
            &good_clock,
            4,
        )
        .is_err());
        assert!(!db.exists(), "坏 JSON 必须在建库前拒绝");
    }

    #[test]
    fn old_write_and_read_entries_refuse_the_new_domain() {
        let temp = Temp::new();
        let db = init_domain(&temp);
        let clock = write_clock(&temp, "ns");
        assert!(cmd_advance(&db, DEFAULT_WRITER_EPOCH).is_err());
        assert!(cmd_snapshot(&db, None).is_err());
        assert!(cmd_catalog(&db, None).is_err());
        assert!(cmd_query(&db, "testonly.ohlc|1|TEST.OHLC|event-1").is_err());
        assert!(v2::dispatch(&init_args(&db, &clock)).is_err());
        let before = std::fs::read(&db).unwrap();
        assert!(init_v3(
            &db,
            "s3-testonly-001",
            &fixture(CATALOG),
            &fixture(OHLC_PROFILE),
            "1",
            &clock,
            4,
        )
        .is_err());
        assert_eq!(std::fs::read(&db).unwrap(), before);
    }
}
