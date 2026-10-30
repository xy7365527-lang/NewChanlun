//! #1372：本地单写者传输、语义时钟和独立普通投递保留；结构判断仍由父模块唯一核心完成。
use super::*;
use crate::v3::audit_store::{CodecRead, NativeAdmission};
use crate::v3::business_json::{self as control_json, BusinessMap, JsonOwner, TextValue};
#[cfg_attr(not(feature = "s_allocation_json_testonly"), allow(unused_imports))]
use newchan_rust::s_content::capture_allocation::audit_json::{self as control_aj, JsonView};
use newchan_rust::s_content::capture_allocation::native_owned::NativeValue;
use newchan_rust::s_content::capture_allocation::scratch::WorkScope;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecVec};
pub use newchan_rust::session_protocol::strict_json;
use newchan_rust::session_protocol::{nonnegative, positive, required, validate_envelope};
use newchan_rust::session_transport::{self, serve_unix, TransportBounds};
use newchan_rust::sqlite_checked::{CheckedRow, CheckedStatement};
use std::ffi::CStr;
use std::fmt;
use std::fs::File;
use std::time::Duration;
#[cfg(test)]
use std::time::Instant;

pub const SCHEMA_V2: &str = r#"
CREATE TABLE s_protocol_meta (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 protocol_revision TEXT NOT NULL,
 session_generation TEXT NOT NULL,
 clock_plan_hash TEXT NOT NULL,
 clock_plan_json TEXT NOT NULL,
 next_attempt_ordinal INTEGER NOT NULL CHECK(next_attempt_ordinal>=1),
 logical_phase_frontier TEXT NOT NULL
);
CREATE TABLE s_input_messages (
 source_namespace TEXT NOT NULL,
 source_epoch TEXT NOT NULL,
 message_id TEXT NOT NULL,
 payload_hash TEXT NOT NULL,
 canonical_envelope TEXT NOT NULL,
 receipt_id TEXT NOT NULL,
 accepted_seq INTEGER NOT NULL CHECK(accepted_seq>=0),
 status TEXT NOT NULL CHECK(status IN ('AcceptedPending','Committed')),
 first_published_generation INTEGER,
 clock_event_id TEXT NOT NULL UNIQUE,
 attempt_count INTEGER NOT NULL CHECK(attempt_count>=0),
 PRIMARY KEY(source_namespace,source_epoch,message_id)
);
CREATE TABLE s_clock_events (
 clock_event_id TEXT NOT NULL,
 phase TEXT NOT NULL CHECK(phase IN ('accept','begin','commit','recover')),
 attempt_index INTEGER NOT NULL,
 semantic_ns TEXT NOT NULL,
 message_key TEXT NOT NULL,
 attempt_ordinal INTEGER,
 generation INTEGER NOT NULL CHECK(generation>=0),
 PRIMARY KEY(clock_event_id,phase,attempt_index)
);
CREATE TABLE s_delivery_policy (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 session_id TEXT NOT NULL,
 session_generation TEXT NOT NULL,
 policy_revision TEXT NOT NULL,
 retain_generations INTEGER NOT NULL CHECK(retain_generations>=1),
 first_available_generation INTEGER NOT NULL CHECK(first_available_generation>=1),
 head_generation INTEGER NOT NULL CHECK(head_generation>=0)
);
CREATE TABLE s_delivery_refs (
 generation INTEGER PRIMARY KEY CHECK(generation>=1),
 FOREIGN KEY(generation) REFERENCES structure_deltas(generation)
);
"#;

fn err(e: impl fmt::Display) -> String {
    crate::v3::business_json::mixed_diagnostic(format_args!("StorageUnavailable：{e}"))
}

/// `s-clock-plan/1` 是全仓唯一的 clock 规则（严格 JSON + 非负/不倒退 attempt 校验 + 规范 JSON hash）。
/// #1394 C：`s-session/3` 新域复用本类型，不再写第二份 clock 解析（同一判断不得有宽严两档）；
/// 因此 `value/hash/parse/load/phase` 对同 harness 的兄弟模块开放 `pub(super)`，语义不变。
#[derive(Clone)]
pub struct ClockPlan {
    pub(super) value: Value,
    pub(super) hash: String,
}
impl ClockPlan {
    pub(super) fn parse(value: Value) -> Result<Self, String> {
        let parsed = newchan_rust::session_protocol::ClockView::parse((&value).into())?;
        let hash = parsed.hash_fixed()?.as_str().to_owned();
        Ok(Self { value, hash })
    }
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        Self::parse(strict_json(&std::fs::read(path).map_err(err)?)?)
    }
    pub(super) fn phase(&self, id: &str, phase: &str, attempt: i64) -> Result<String, String> {
        newchan_rust::session_protocol::clock_phase_view((&self.value).into(), id, phase, attempt)
            .map(str::to_owned)
    }
}

#[derive(Clone)]
pub struct InputContext {
    envelope: Value,
    clock: ClockPlan,
}
impl InputContext {
    /// #1394 C：S3（`s-session/3`）在自核域与自身业务形状后复用同一上下文类型与同一批机械规则；
    /// 只最小开放构造，字段语义不变。
    pub(super) fn new(envelope: Value, clock: ClockPlan) -> Self {
        Self { envelope, clock }
    }
    fn id(&self) -> &str {
        self.envelope["payload"]["clock_event_id"].as_str().unwrap()
    }
    fn key(&self) -> String {
        canonical_json(&json!([
            self.envelope["source_namespace"],
            self.envelope["source_epoch"],
            self.envelope["message_id"]
        ]))
    }
}
fn active(conn: &Connection) -> Result<bool, String> {
    let marker = meta_get_opt(conn, "protocol_revision")?;
    let tables:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN ('s_protocol_meta','s_input_messages','s_clock_events','s_delivery_policy','s_delivery_refs')",[],|r|r.get(0)).map_err(err)?;
    match (marker.as_deref(), tables) {
        (None, 0) => Ok(false),
        (Some("s-session/2"), 5) => Ok(true),
        _ => Err(err("协议标记/控制表缺失或不支持")),
    }
}
/// 从 `s_protocol_meta` 回读并重算 clock 计划（唯一解析入口；v2 与 S3 共用）。
pub(super) fn stored_clock(conn: &Connection) -> Result<ClockPlan, String> {
    let clock = stored_control_clock(conn, None)?;
    match (clock.value, clock.hash) {
        (JsonOwner::Plain(value), TextValue::Plain(hash)) => Ok(ClockPlan {
            value: value.into_unscoped()?,
            hash: hash.into_unscoped().map_err(err)?,
        }),
        _ => Err(err("normal ClockPlan不可擦除Owned结果")),
    }
}
struct ControlClock {
    value: JsonOwner,
    hash: TextValue,
}
impl ControlClock {
    fn phase(&self, id: &str, phase: &str, index: i64) -> Result<&str, String> {
        newchan_rust::session_protocol::clock_phase_view(self.value.view(), id, phase, index)
    }
}
fn control_decode(
    raw: &str,
    owner: Option<&OwnedBudget>,
    legacy: bool,
) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = owner {
        return if legacy {
            control_aj::OwnedJson::parse_legacy_shape(raw.as_bytes(), b)
        } else {
            control_aj::OwnedJson::parse(raw.as_bytes(), b)
        }
        .map(JsonOwner::Owned)
        .map_err(|e| e.to_string());
    }
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned控制JSON未启用".into());
    }
    if legacy {
        serde_json::from_str::<Value>(raw)
            .map(|v| JsonOwner::Plain(v.into()))
            .map_err(|e| e.to_string())
    } else {
        strict_json(raw.as_bytes()).map(|v| JsonOwner::Plain(v.into()))
    }
}
fn control_native_owner(
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    if let Some(work) = work {
        if owner.is_some() {
            return Err(work
                .reject(
                    newchan_rust::s_content::capture_allocation::scratch::ScratchFailure::Ledger,
                )
                .to_string());
        }
    }
    Ok(())
}
// 每次实际增长前从同一个候选父域申请；None 保留原容器操作和错误类型。
fn control_push<T>(
    values: &mut CodecVec<T>,
    value: T,
    work: Option<&WorkScope>,
) -> Result<(), newchan_rust::s_content::CodecError> {
    NativeAdmission(work).native_codec_room(values, 1)?;
    values.try_push(value)
}
fn control_extend(
    values: &mut CodecVec<u8>,
    bytes: &[u8],
    work: Option<&WorkScope>,
) -> Result<(), newchan_rust::s_content::CodecError> {
    NativeAdmission(work).native_codec_room(values, bytes.len())?;
    values.try_extend_bytes(bytes)
}
fn control_insert<K: Eq + std::hash::Hash, V>(
    map: &mut BusinessMap<K, V>,
    key: K,
    value: V,
    work: Option<&WorkScope>,
) -> Result<Option<V>, newchan_rust::s_content::CodecError> {
    NativeAdmission(work).native_map_room(map)?;
    map.insert(key, value)
}
fn control_decode_admitted(
    raw: &str,
    owner: Option<&OwnedBudget>,
    legacy: bool,
    work: Option<&WorkScope>,
) -> Result<JsonOwner, String> {
    control_native_owner(owner, work)?;
    #[cfg(feature = "s_incremental")]
    let permit = if work.is_some() {
        NativeAdmission(work)
            .native_reservation(
                crate::v3::audit_owner::native::JsonShape::scan(raw.as_bytes()).parse_peak(),
            )
            .map_err(err)?
    } else {
        None
    };
    #[cfg(not(feature = "s_incremental"))]
    if work.is_some() {
        return Err("prototype_audit_allocation_incomplete：Native控制JSON未启用".into());
    }
    let value = control_decode(raw, owner, legacy)?;
    #[cfg(feature = "s_incremental")]
    let value = match value {
        JsonOwner::Plain(value) => JsonOwner::Plain(value.with_reservation(permit)),
        #[cfg(feature = "s_allocation_json_testonly")]
        owned @ JsonOwner::Owned(_) => owned,
    };
    Ok(value)
}
fn control_canonical_equal(
    value: JsonView<'_>,
    raw: &str,
    owner: Option<&OwnedBudget>,
) -> Result<bool, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = owner {
        let encoded = control_aj::canonical_bytes(value, b).map_err(|e| e.to_string())?;
        return Ok(&*encoded == raw.as_bytes());
    }
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned规范JSON未启用".into());
    }
    // A byte-comparing writer avoids the old canonical String/parts Vec while retaining serde formatter and sorted view keys.
    struct Compare<'a> {
        expected: &'a [u8],
        at: usize,
        equal: bool,
    }
    impl std::io::Write for Compare<'_> {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            let end = self.at.saturating_add(b.len());
            if self.expected.get(self.at..end) != Some(b) {
                self.equal = false;
            }
            self.at = end;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut compare = Compare {
        expected: raw.as_bytes(),
        at: 0,
        equal: true,
    };
    serde_json::to_writer(&mut compare, &value).map_err(|e| e.to_string())?;
    Ok(compare.equal && compare.at == raw.len())
}
fn stored_control_clock(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
) -> Result<ControlClock, String> {
    stored_control_clock_admitted(conn, owner, None)
}
fn stored_control_clock_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<ControlClock, String> {
    control_native_owner(owner, work)?;
    #[cfg(test)]
    let clock_start = std::time::Instant::now();
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned控制clock未启用".into());
    }
    let mut stmt = CheckedStatement::prepare(
        conn,
        c"SELECT clock_plan_json,clock_plan_hash FROM s_protocol_meta WHERE singleton=1",
    )
    .map_err(err)?;
    let mut row = stmt
        .next()
        .map_err(err)?
        .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
    let raw = TextValue::from_sql_admitted(&mut row, 0, owner, work).map_err(err)?;
    let stored_hash = TextValue::from_sql_admitted(&mut row, 1, owner, work).map_err(err)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    let value = control_decode_admitted(raw.as_str(), owner, false, work).map_err(err)?;
    let view = newchan_rust::session_protocol::ClockView::parse(value.view()).map_err(err)?;
    let hash = view.hash_fixed_with_owner(owner).map_err(err)?;
    if hash.as_str() != stored_hash.as_str()
        || !control_canonical_equal(value.view(), raw.as_str(), owner)?
    {
        return Err(err("clock计划/hash不一致"));
    }
    let hash = TextValue::from_str_admitted(hash.as_str(), owner, work)?;
    #[cfg(test)]
    crate::v3::trace_request_clock(raw.as_str().len(), clock_start.elapsed());
    Ok(ControlClock { value, hash })
}
/// 持久 clock 计划绑定核验（唯一判据；v2 与 S3 共用，不另立第二份）。
pub(super) fn check_clock(conn: &Connection, clock: &ClockPlan) -> Result<(), String> {
    if stored_clock(conn)?.hash != clock.hash {
        return Err("IdentityConflict：clock计划与持久绑定不同".into());
    }
    Ok(())
}
fn record_phase(
    conn: &Connection,
    clock: &ClockPlan,
    id: &str,
    phase: &str,
    index: i64,
    key: &str,
    ordinal: Option<i64>,
    generation: i64,
) -> Result<String, String> {
    check_clock(conn, clock)?;
    let ns = clock.phase(id, phase, index)?;
    let old = control_query_row(
        conn,
        c"SELECT logical_phase_frontier FROM s_protocol_meta WHERE singleton=1",
        &[],
        |r| r.text(0).map(str::to_owned).map_err(|e| e.to_string()),
    )?;
    if parse_canonical_i64(&ns, "clock")? < parse_canonical_i64(&old, "clock frontier")? {
        return Err("InvalidDomain：实际语义clock倒退".into());
    }
    conn.execute("INSERT INTO s_clock_events(clock_event_id,phase,attempt_index,semantic_ns,message_key,attempt_ordinal,generation) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,phase,index,ns,key,ordinal,generation]).map_err(err)?;
    conn.execute(
        "UPDATE s_protocol_meta SET logical_phase_frontier=?1 WHERE singleton=1",
        params![ns],
    )
    .map_err(err)?;
    Ok(ns)
}

pub fn record_accept(
    conn: &Connection,
    c: Option<&InputContext>,
    results: &[Value],
) -> Result<(), String> {
    if !active(conn)? {
        return Ok(());
    }
    let c = c.ok_or("MissingDependency：v2接纳必须带持久message身份和clock")?;
    record_accept_context(conn, c, results)
}

/// #1394 C：v2 与 S3 共用的**接纳机械规则**（调用方已核自身域、身份与 clock 上下文）。
/// 只写接纳控制行与 accept clock 事件；不核根状态、不代做完整 state 审计。
pub(super) fn record_accept_context(
    conn: &Connection,
    c: &InputContext,
    results: &[Value],
) -> Result<(), String> {
    if results.len() != 1 {
        return Err("InvalidDomain：v2每次只接纳单事件".into());
    }
    let r = &results[0];
    if r["status"] != "accepted" && r["status"] != "replay" {
        return Err(format!("{}：输入未接纳", r["status"]));
    }
    let seq = parse_canonical_i64(required(r, "seq")?, "accepted_seq")?;
    let g: i64 = conn
        .query_row(
            "SELECT MIN(generation) FROM structure_deltas WHERE input_frontier>=?1",
            params![seq],
            |r| Ok(r.get::<_, Option<i64>>(0)?.unwrap_or(0)),
        )
        .map_err(err)?;
    conn.execute("INSERT INTO s_input_messages(source_namespace,source_epoch,message_id,payload_hash,canonical_envelope,receipt_id,accepted_seq,status,first_published_generation,clock_event_id,attempt_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,0)",params![required(&c.envelope,"source_namespace")?,required(&c.envelope,"source_epoch")?,required(&c.envelope,"message_id")?,required(&c.envelope,"payload_hash")?,canonical_json(&c.envelope),required(r,"receipt_id")?,seq,if g==0{"AcceptedPending"}else{"Committed"},if g==0{None}else{Some(g)},c.id()]).map_err(err)?;
    record_phase(
        conn,
        &c.clock,
        c.id(),
        "accept",
        -1,
        &c.key(),
        None,
        meta_i64(conn, "generation")?,
    )?;
    Ok(())
}
pub fn record_begin(
    conn: &Connection,
    c: Option<&InputContext>,
    gen: i64,
    frontier: i64,
    base: &str,
    epoch: &str,
) -> Result<String, String> {
    if !active(conn)? {
        return Ok(make_token());
    }
    let c = c.ok_or("MissingDependency：v2推进缺message/clock上下文")?;
    record_begin_context(conn, c, gen, frontier, base, epoch)
}

/// #1394 C：v2 与 S3 共用的 **Begin 机械规则**：真实持久 attempt/ordinal、同一 token 配方、
/// begin clock 事件与相位前沿一并入调用方事务；不核根状态。
pub(super) fn record_begin_context(
    conn: &Connection,
    c: &InputContext,
    gen: i64,
    frontier: i64,
    base: &str,
    epoch: &str,
) -> Result<String, String> {
    let index:i64=conn.query_row("SELECT attempt_count FROM s_input_messages WHERE clock_event_id=?1 AND status='AcceptedPending'",params![c.id()],|r|r.get(0)).map_err(err)?;
    let (ordinal, inc) = control_query_row(
        conn,
        c"SELECT next_attempt_ordinal,session_generation FROM s_protocol_meta WHERE singleton=1",
        &[],
        |r| {
            let ordinal = r.integer(0).map_err(|e| e.to_string())?;
            let inc = r.text(1).map_err(|e| e.to_string())?.to_owned();
            Ok((ordinal, inc))
        },
    )?;
    let next = ordinal
        .checked_add(1)
        .ok_or("InvalidDomain：attempt ordinal溢出")?;
    // Commit时刻也先核存在；绝不在已经Begin之后才发现计划根本没有该attempt。
    c.clock.phase(c.id(), "commit", index)?;
    record_phase(
        conn,
        &c.clock,
        c.id(),
        "begin",
        index,
        &c.key(),
        Some(ordinal),
        gen,
    )?;
    conn.execute(
        "UPDATE s_input_messages SET attempt_count=attempt_count+1 WHERE clock_event_id=?1",
        params![c.id()],
    )
    .map_err(err)?;
    conn.execute(
        "UPDATE s_protocol_meta SET next_attempt_ordinal=?1 WHERE singleton=1",
        params![next],
    )
    .map_err(err)?;
    Ok(format!(
        "begin-{}",
        sha256_hex(
            canonical_json(&json!([
                meta_get_opt(conn, "session_id")?,
                inc,
                epoch,
                ordinal.to_string(),
                base,
                gen.to_string(),
                frontier.to_string()
            ]))
            .as_bytes()
        )
    ))
}
/// #1392：本次已持久 Begin 对应的语义提交时刻；不读后续输入的时钟/收据。
pub fn commit_time(conn: &Connection, c: Option<&InputContext>) -> Result<Option<String>, String> {
    if !active(conn)? {
        return Ok(None);
    }
    let c = c.ok_or("MissingDependency：新笔获知时点缺本次上下文")?;
    commit_time_context(conn, c)
}

/// #1394 C：v2 与 S3 共用的提交时刻读取（只读本次已持久 Begin 对应的 attempt）。
pub(super) fn commit_time_context(
    conn: &Connection,
    c: &InputContext,
) -> Result<Option<String>, String> {
    let index: i64 = conn
        .query_row(
            "SELECT attempt_count-1 FROM s_input_messages WHERE clock_event_id=?1",
            params![c.id()],
            |r| r.get(0),
        )
        .map_err(err)?;
    Ok(Some(c.clock.phase(c.id(), "commit", index)?))
}

pub fn seal_batch(
    conn: &Connection,
    c: Option<&InputContext>,
    batch: &mut Value,
) -> Result<(), String> {
    if !active(conn)? {
        return Ok(());
    }
    let c = c.ok_or("MissingDependency：缺v2上下文")?;
    let (inc,index)=control_query_row(conn,c"SELECT p.session_generation,m.attempt_count-1 FROM s_protocol_meta p JOIN s_input_messages m ON m.clock_event_id=?1 WHERE p.singleton=1",&[ControlBind::Text(c.id())],|r|{let inc=r.text(0).map_err(|e|e.to_string())?.to_owned();let index=r.integer(1).map_err(|e|e.to_string())?;Ok((inc,index))})?;
    batch["protocol_revision"] = json!("s-session/2");
    batch["session_generation"] = json!(inc);
    batch["clock_plan_hash"] = json!(c.clock.hash);
    batch["semantic_commit_ns"] = json!(c.clock.phase(c.id(), "commit", index)?);
    Ok(())
}
pub fn record_commit(
    conn: &Connection,
    c: Option<&InputContext>,
    gen: i64,
    frontier: i64,
) -> Result<(), String> {
    if !active(conn)? {
        return Ok(());
    }
    let c = c.ok_or("MissingDependency：缺v2提交上下文")?;
    record_commit_context(conn, c, gen, frontier)
}

/// #1394 C：v2 与 S3 共用的 **Commit 机械规则**：commit clock 事件、status/first_published_generation
/// 推进与 delivery refs/policy（含保留窗口）在调用方同一事务内完成；不核根状态。
pub(super) fn record_commit_context(
    conn: &Connection,
    c: &InputContext,
    gen: i64,
    frontier: i64,
) -> Result<(), String> {
    let (index,ordinal):(i64,i64)=conn.query_row("SELECT m.attempt_count-1,e.attempt_ordinal FROM s_input_messages m JOIN s_clock_events e ON e.clock_event_id=m.clock_event_id AND e.phase='begin' AND e.attempt_index=m.attempt_count-1 WHERE m.clock_event_id=?1",params![c.id()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(err)?;
    record_phase(
        conn,
        &c.clock,
        c.id(),
        "commit",
        index,
        &c.key(),
        Some(ordinal),
        gen,
    )?;
    conn.execute("UPDATE s_input_messages SET status='Committed',first_published_generation=?1 WHERE status='AcceptedPending' AND accepted_seq<=?2",params![gen,frontier]).map_err(err)?;
    let keep: i64 = conn
        .query_row(
            "SELECT retain_generations FROM s_delivery_policy WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let first = 1.max(gen - keep + 1);
    conn.execute(
        "INSERT INTO s_delivery_refs(generation) VALUES(?1)",
        params![gen],
    )
    .map_err(err)?;
    conn.execute(
        "DELETE FROM s_delivery_refs WHERE generation<?1",
        params![first],
    )
    .map_err(err)?;
    conn.execute("UPDATE s_delivery_policy SET first_available_generation=?1,head_generation=?2 WHERE singleton=1",params![first,gen]).map_err(err)?;
    Ok(())
}
pub fn record_recover(
    conn: &Connection,
    clock: Option<(&ClockPlan, &str)>,
    generation: &str,
) -> Result<String, String> {
    if !active(conn)? {
        return Ok(now_nanos());
    }
    let (clock, id) = clock.ok_or("MissingDependency：v2 recover需要clock-plan及clock-event-id")?;
    record_recover_context(conn, clock, id, generation)
}

/// #1394 C：v2 与 S3 共用的 **recover 机械规则**（只写 recover clock 事件与相位前沿；
/// writer 换代/历史等自身状态由调用方在同一事务内完成）。
pub(super) fn record_recover_context(
    conn: &Connection,
    clock: &ClockPlan,
    id: &str,
    generation: &str,
) -> Result<String, String> {
    record_phase(
        conn,
        clock,
        id,
        "recover",
        -1,
        "",
        None,
        parse_canonical_i64(generation, "generation")?,
    )
}

fn envelope(value: Value) -> Result<Value, String> {
    validate_envelope(value, "s-session/2")
}
fn normalize_schema(s: &str) -> String {
    let mut out = String::new();
    normalize_schema_chunks(s, |v| {
        out.push_str(v);
        Ok::<(), String>(())
    })
    .expect("normal String sink");
    out
}
fn normalize_schema_chunks<E>(
    s: &str,
    mut emit: impl FnMut(&str) -> Result<(), E>,
) -> Result<(), E> {
    // Original global replace then ASCII-only whitespace filtering; no SQL reinterpretation.
    for part in s.split("IF NOT EXISTS") {
        for ch in part.chars().filter(|c| !c.is_ascii_whitespace()) {
            let mut bytes = [0; 4];
            emit(ch.encode_utf8(&mut bytes))?;
        }
    }
    Ok(())
}
fn control_normalized_admitted(
    s: &str,
    b: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<TextValue, String> {
    control_native_owner(b, work)?;
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if b.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned控制文本未启用".into());
    }
    let mut bytes = control_json::new_work(b);
    normalize_schema_chunks(s, |v| {
        control_extend(&mut bytes, v.as_bytes(), work).map_err(err)
    })?;
    TextValue::from_bytes(bytes).map_err(err)
}
pub(super) fn control_meta_text(
    conn: &Connection,
    key: &str,
    owner: Option<&OwnedBudget>,
) -> Result<Option<TextValue>, String> {
    control_meta_text_admitted(conn, key, owner, None)
}
pub(super) fn control_meta_text_admitted(
    conn: &Connection,
    key: &str,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<Option<TextValue>, String> {
    control_native_owner(owner, work)?;
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned控制文本未启用".into());
    }
    let mut stmt = CheckedStatement::prepare(conn, c"SELECT value FROM meta WHERE key = ?1")
        .map_err(|e| format!("读 meta.{key} 失败：{e}"))?;
    stmt.bind_text(1, key)
        .map_err(|e| format!("读 meta.{key} 失败：{e}"))?;
    let value = match stmt
        .next()
        .map_err(|e| format!("读 meta.{key} 失败：{e}"))?
    {
        Some(mut row) => Some(
            TextValue::from_sql_admitted(&mut row, 0, owner, work)
                .map_err(|e| format!("读 meta.{key} 失败：{e}"))?,
        ),
        None => None,
    };
    stmt.finalize()
        .map_err(|e| format!("读 meta.{key} 失败：{e}"))?;
    Ok(value)
}
pub(super) fn control_meta_i64(
    conn: &Connection,
    key: &str,
    owner: Option<&OwnedBudget>,
) -> Result<i64, String> {
    control_meta_i64_admitted(conn, key, owner, None)
}
pub(super) fn control_meta_i64_admitted(
    conn: &Connection,
    key: &str,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<i64, String> {
    control_native_owner(owner, work)?;
    let v = control_meta_text_admitted(conn, key, owner, work)?
        .ok_or_else(|| format!("StorageUnavailable：meta.{key} 缺失"))?;
    let mut label = control_json::new_work(owner);
    control_extend(&mut label, b"meta.", work).map_err(err)?;
    control_extend(&mut label, key.as_bytes(), work).map_err(err)?;
    let label = TextValue::from_bytes(label).map_err(err)?;
    parse_canonical_i64(v.as_str(), &label).map_err(|e| format!("StorageUnavailable：{e}"))
}
pub(super) struct ControlSchema {
    tables: BusinessMap<TextValue, TextValue>,
    owner: Option<OwnedBudget>,
}
impl ControlSchema {
    pub(super) fn new(owner: Option<&OwnedBudget>) -> Self {
        Self {
            tables: control_json::new_map(owner),
            owner: owner.cloned(),
        }
    }
    pub(super) fn add_ddl_admitted(
        &mut self,
        label: &str,
        ddl: &str,
        work: Option<&WorkScope>,
    ) -> Result<(), String> {
        control_native_owner(self.owner.as_ref(), work)?;
        ddl_table_entries(label, ddl, |name, stmt| {
            let key = TextValue::from_str_admitted(name, self.owner.as_ref(), work)?;
            let value = control_normalized_admitted(stmt, self.owner.as_ref(), work)?;
            control_insert(&mut self.tables, key, value, work).map_err(err)?;
            Ok(())
        })
    }
}
fn ddl_table_entries<E: From<String>>(
    label: &str,
    ddl: &str,
    mut emit: impl FnMut(&str, &str) -> Result<(), E>,
) -> Result<(), E> {
    for stmt in ddl.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let rest = stmt
            .strip_prefix("CREATE TABLE ")
            .ok_or_else(|| format!("{label}不是建表"))?
            .strip_prefix("IF NOT EXISTS ")
            .unwrap_or_else(|| stmt.strip_prefix("CREATE TABLE ").unwrap());
        let name = rest
            .split(|c: char| c == '(' || c.is_whitespace())
            .next()
            .unwrap();
        emit(name, stmt)?;
    }
    Ok(())
}
pub(super) fn expect_ddl_tables(
    label: &str,
    ddl: &str,
    expected: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    ddl_table_entries(label, ddl, |name, stmt| {
        expected.insert(name.to_string(), normalize_schema(stmt));
        Ok(())
    })
}
enum SchemaRead<'a> {
    Normal(&'a BTreeMap<String, String>),
    Owned(&'a ControlSchema),
}
impl SchemaRead<'_> {
    fn owner(&self) -> Option<&OwnedBudget> {
        match self {
            Self::Normal(_) => None,
            Self::Owned(s) => s.owner.as_ref(),
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Normal(s) => s.len(),
            Self::Owned(s) => s.tables.len(),
        }
    }
    fn get(&self, k: &str) -> Option<&str> {
        match self {
            Self::Normal(s) => s.get(k).map(String::as_str),
            Self::Owned(s) => s.tables.get(k).map(TextValue::as_str),
        }
    }
    fn sorted_names_admitted(&self, work: Option<&WorkScope>) -> Result<CodecVec<&str>, String> {
        let mut names = control_json::new_work(self.owner());
        match self {
            Self::Normal(s) => {
                for k in s.keys() {
                    control_push(&mut names, k.as_str(), work).map_err(err)?;
                }
            }
            Self::Owned(s) => {
                for (k, _) in s.tables.iter() {
                    control_push(&mut names, k.as_str(), work).map_err(err)?;
                }
            }
        }
        names.sort_unstable();
        Ok(names)
    }
}
pub(super) fn verify_expected_schema(
    conn: &Connection,
    expected: &BTreeMap<String, String>,
) -> Result<(), String> {
    verify_schema_read(conn, SchemaRead::Normal(expected))
}
pub(super) fn verify_control_schema_admitted(
    conn: &Connection,
    expected: &ControlSchema,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    verify_schema_read_admitted(conn, SchemaRead::Owned(expected), work)
}
fn verify_schema_read(conn: &Connection, expected: SchemaRead<'_>) -> Result<(), String> {
    verify_schema_read_admitted(conn, expected, None)
}
fn verify_schema_read_admitted(
    conn: &Connection,
    expected: SchemaRead<'_>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    let owner = expected.owner();
    control_native_owner(owner, work)?;
    let mut q = CheckedStatement::prepare(
        conn,
        c"SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .map_err(err)?;
    let mut seen = control_json::new_map::<TextValue, bool>(owner);
    while let Some(mut r) = q.next().map_err(err)? {
        let kind = TextValue::from_sql_admitted(&mut r, 0, owner, work).map_err(err)?;
        let name = TextValue::from_sql_admitted(&mut r, 1, owner, work).map_err(err)?;
        let sql = TextValue::from_sql_admitted(&mut r, 2, owner, work).map_err(err)?;
        if kind != "table" {
            return Err(err(format!("模式/约束不匹配 {kind}/{name}")));
        }
        let normalized = control_normalized_admitted(sql.as_str(), owner, work)?;
        if expected.get(name.as_str()) != Some(normalized.as_str()) {
            return Err(err(format!("模式/约束不匹配 {kind}/{name}")));
        }
        control_insert(&mut seen, name, true, work).map_err(err)?;
    }
    q.finalize().map_err(err)?;
    if seen.len() != expected.len() {
        return Err(err("缺必需表/约束"));
    }
    for table in &expected.sorted_names_admitted(work)? {
        let mut columns = CheckedStatement::prepare(
            conn,
            c"SELECT type,\"notnull\",pk FROM pragma_table_info(?1) ORDER BY cid",
        )
        .map_err(err)?;
        columns.bind_text(1, table).map_err(err)?;
        let mut declarations = control_json::new_work(owner);
        while let Some(mut r) = columns.next().map_err(err)? {
            control_push(
                &mut declarations,
                (
                    TextValue::from_sql_admitted(&mut r, 0, owner, work).map_err(err)?,
                    r.integer(1).map_err(err)?,
                    r.integer(2).map_err(err)?,
                ),
                work,
            )
            .map_err(err)?;
        }
        columns.finalize().map_err(err)?;
        let mut sql = control_json::new_work(owner);
        control_extend(&mut sql, b"SELECT * FROM ", work).map_err(err)?;
        control_extend(&mut sql, table.as_bytes(), work).map_err(err)?;
        // Only the C ABI terminator is added; the SQL text and its bindings are unchanged.
        control_push(&mut sql, 0, work).map_err(err)?;
        let sql = CStr::from_bytes_with_nul(&sql).map_err(err)?;
        let mut query = CheckedStatement::prepare(conn, sql).map_err(err)?;
        while let Some(mut row) = query.next().map_err(err)? {
            for (i, (kind, notnull, pk)) in declarations.iter().enumerate() {
                let storage = row.column_type(i).map_err(err)?;
                if storage == rusqlite::types::Type::Blob {
                    row.blob(i).map_err(err)?;
                }
                let text = if storage == rusqlite::types::Type::Text {
                    Some(row.text_bytes(i).map_err(err)?)
                } else {
                    None
                };
                let valid = valid_storage_cell(kind, *notnull, *pk, storage, text);
                if !valid {
                    return Err(err(format!("{table} 第{i}列实际类型/非空约束损坏")));
                }
            }
        }
        query.finalize().map_err(err)?;
    }
    Ok(())
}

/// 两条审计路径逐列运行相同的实际 storage-class/UTF-8 判据。
pub(super) fn valid_storage_cell(
    kind: &str,
    notnull: i64,
    pk: i64,
    storage: rusqlite::types::Type,
    text: Option<&[u8]>,
) -> bool {
    use rusqlite::types::Type;
    match storage {
        Type::Null => notnull == 0 && pk == 0,
        Type::Integer => kind == "INTEGER",
        Type::Text => kind == "TEXT" && text.is_some_and(|b| std::str::from_utf8(b).is_ok()),
        Type::Blob => kind == "BLOB",
        Type::Real => false,
    }
}

fn verify_schema(
    conn: &Connection,
    v2: bool,
    before_write: Option<&std::cell::Cell<crate::DeltaShape>>,
) -> Result<(), String> {
    let mut expected = BTreeMap::new();
    expect_ddl_tables(
        "schema常量",
        &format!(
            "{}{}{}",
            SCHEMA,
            if v2 { SCHEMA_V2 } else { "" },
            if tb02a::enabled(conn)? {
                tb02a::SCHEMA
            } else {
                ""
            }
        ),
        &mut expected,
    )?;
    // #1394：存在私有格式登记表时严格核其结构与声明；无表则按旧库的完整 Delta 判据验证。
    // 删除新库登记表不在此处硬拒：若各代稀疏 Delta 恰等于完整活动集，字节上无法与旧库区分。
    let has_format_table: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
            params![delta_storage::TABLE],
            |r| r.get(0),
        )
        .map_err(err)?;
    if has_format_table == 1 {
        expected.insert(
            delta_storage::TABLE.to_string(),
            normalize_schema(delta_storage::DDL),
        );
    }
    // #1394 第二段：存在私有证据图表时严格核其三表结构；无表则按旧库整块域验证。
    // 三表同生同灭（由 `evidence_store::DDL` 一次建出），缺一即模式门拒绝。
    let has_evidence_tables: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN (?1,?2,?3)",
            params![
                evidence_store::FORMAT_TABLE,
                evidence_store::NODES_TABLE,
                evidence_store::ROOTS_TABLE
            ],
            |r| r.get(0),
        )
        .map_err(err)?;
    if has_evidence_tables != 0 {
        if has_evidence_tables != 3 {
            return Err(err("私有证据图表不齐全（三表必须同生）"));
        }
        expect_ddl_tables("证据图 DDL", evidence_store::DDL, &mut expected)?;
    }
    // 写前准入：迁移前源换成迁移前的精确 structure_deltas 期望并记下形状，判别与迁移同一谓词。
    if let Some(shape) = before_write {
        if crate::policy_records_missing(conn)? {
            expect_ddl_tables(
                "#1602前Delta",
                crate::PRE_POLICY_RECORDS_DELTAS,
                &mut expected,
            )?;
            shape.set(crate::DeltaShape::PrePolicyRecords);
        }
    }
    verify_expected_schema(conn, &expected)?;
    // 表在而登记行缺/多/未声明值：与模式损坏同门，读取与写入前一律拒绝。
    delta_storage::read_mode(conn)?;
    evidence_store::read_format(conn)?;
    Ok(())
}

pub(super) fn verify_epoch_history(
    conn: &Connection,
    v2: bool,
) -> Result<Vec<(String, i64)>, String> {
    verify_epoch_history_with_owner(conn, v2, None)?
        .into_iter()
        .map(|(s, g)| match s {
            TextValue::Plain(s) => Ok((s.into_unscoped().map_err(err)?, g)),
            TextValue::Owned(_) => Err(err("normal epoch结果不可擦除Owned文本")),
        })
        .collect()
}
pub(super) fn verify_epoch_history_with_owner(
    conn: &Connection,
    v2: bool,
    owner: Option<&OwnedBudget>,
) -> Result<CodecVec<(TextValue, i64)>, String> {
    verify_epoch_history_admitted(conn, v2, owner, None)
}
pub(super) fn verify_epoch_history_admitted(
    conn: &Connection,
    v2: bool,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<CodecVec<(TextValue, i64)>, String> {
    control_native_owner(owner, work)?;
    let generation = control_meta_i64_admitted(conn, "generation", owner, work)?;
    let current = parse_writer_epoch(
        &control_meta_text_admitted(conn, "writer_epoch", owner, work)?
            .ok_or_else(|| err("缺writer_epoch"))?,
        "meta.writer_epoch",
    )
    .map_err(err)?;
    let mut query=CheckedStatement::prepare(conn,c"SELECT ordinal,from_epoch,to_epoch,generation_at_transition,advance_state_at_transition,transitioned_at FROM writer_epoch_history ORDER BY ordinal").map_err(err)?;
    let (mut previous_epoch, mut previous_generation, mut ordinal) = (None, 0, 0i64);
    let mut facts = control_json::new_work(owner);
    while let Some(mut r) = query.next().map_err(err)? {
        ordinal += 1;
        let actual: i64 = r.integer(0).map_err(err)?;
        let from = parse_writer_epoch(
            &TextValue::from_sql_admitted(&mut r, 1, owner, work).map_err(err)?,
            "epoch.from",
        )
        .map_err(err)?;
        let to = parse_writer_epoch(
            &TextValue::from_sql_admitted(&mut r, 2, owner, work).map_err(err)?,
            "epoch.to",
        )
        .map_err(err)?;
        let at = nonnegative(
            &TextValue::from_sql_admitted(&mut r, 3, owner, work).map_err(err)?,
            "epoch.generation",
        )
        .map_err(err)?;
        let state = TextValue::from_sql_admitted(&mut r, 4, owner, work).map_err(err)?;
        let ns = TextValue::from_sql_admitted(&mut r, 5, owner, work).map_err(err)?;
        nonnegative(&ns, "epoch.transitioned_at").map_err(err)?;
        validate_epoch_step(
            actual,
            ordinal,
            from,
            to,
            at,
            &state,
            previous_epoch,
            previous_generation,
            generation,
            v2,
        )?;
        previous_epoch = Some(to);
        previous_generation = at;
        control_push(&mut facts, (ns, at), work).map_err(err)?;
    }
    query.finalize().map_err(err)?;
    if previous_epoch.is_some_and(|last| last != current) {
        return Err(err("writer_epoch与最后换代结果不一致"));
    }
    Ok(facts)
}

pub(super) fn validate_epoch_step(
    actual: i64,
    ordinal: i64,
    from: i64,
    to: i64,
    at: i64,
    state: &str,
    previous_epoch: Option<i64>,
    previous_generation: i64,
    generation: i64,
    v2: bool,
) -> Result<(), String> {
    if actual != ordinal
        || previous_epoch.is_some_and(|p| p != from)
        || to <= from
        || at < previous_generation
        || at > generation
        || (v2 && state != "idle")
    {
        return Err(err("writer换代历史序号/接续/代际/状态不一致"));
    }
    Ok(())
}

/// 全局控制行核验通过后的逐代期望（v2 与 S3 共用**同一份**消费逻辑）。
///
/// 这一层不判协议版本：显式 `s-session/2` / `s-session/3` 由各入口在调用
/// [`ControlHistory::consume_header`] 之前逐字核验（`ExpectedControl::observe_batch` 与
/// S3 控制审计入口），版本不混进共用判据，也不存在「严格入口判不出来就换一档重判」。
// 借用查询与真实 tuple 逐项同 Hash；查询不复制文本或脱离账本。
#[derive(Hash)]
struct GenerationKey<'a>(i64, &'a str);
impl hashbrown::Equivalent<(i64, TextValue)> for GenerationKey<'_> {
    fn equivalent(&self, key: &(i64, TextValue)) -> bool {
        self.0 == key.0 && self.1 == key.1.as_str()
    }
}
#[derive(Hash)]
struct PhaseKey<'a>(&'a str, &'a str, i64);
impl hashbrown::Equivalent<(TextValue, TextValue, i64)> for PhaseKey<'_> {
    fn equivalent(&self, key: &(TextValue, TextValue, i64)) -> bool {
        self.0 == key.0.as_str() && self.1 == key.1.as_str() && self.2 == key.2
    }
}
/// 全控制谓词产生的独立发布头事实；不包含 SQL 连接或可继续查询的能力。
/// 原始控制行的所有权和跨事务当前性必须另由 audit owner 保证。
pub(super) struct ControlFacts {
    session_generation: TextValue,
    clock_plan_hash: TextValue,
    headers: BusinessMap<(i64, TextValue), ControlHeader>,
}
struct ControlHeader {
    commit_ns: TextValue,
    consumed: bool,
}
pub(super) struct ControlHistory {
    facts: ControlFacts,
    pending: usize,
}
impl ControlFacts {
    #[cfg(feature = "s_incremental")]
    pub(super) fn resume_owner(
        mut self,
        generation: i64,
        batch: &str,
        commit_ns: &str,
        _bound: &crate::v3::audit_owner::BoundOriginals,
        retained: &mut crate::v3::audit_retention::RetainedDelta,
        admission: &(impl crate::v3::audit_store::CodecRead + ?Sized),
    ) -> Result<ControlHistory, newchan_rust::s_content::ContextError> {
        if generation != self.headers.len() as i64 + 1 {
            return Err(err("owner 新控制头非连续尾").into());
        }
        let key = (
            generation,
            admission.native_text(batch).map_err(|error| {
                newchan_rust::s_content::ContextError::from(error).context("StorageUnavailable：")
            })?,
        );
        let header = ControlHeader {
            commit_ns: admission.native_text(commit_ns).map_err(|error| {
                newchan_rust::s_content::ContextError::from(error).context("StorageUnavailable：")
            })?,
            consumed: false,
        };
        retained.add(&key);
        retained.add(&header);
        let capacity = self.headers.capacity();
        admission
            .native_map_room(&mut self.headers)
            .map_err(|error| {
                newchan_rust::s_content::ContextError::from(error).context("StorageUnavailable：")
            })?;
        if self
            .headers
            .insert(key, header)
            .map_err(|error| {
                newchan_rust::s_content::ContextError::from(error).context("StorageUnavailable：")
            })?
            .is_some()
        {
            return Err(err("重复发布控制头").into());
        }
        retained
            .hash_capacity::<(i64, TextValue), ControlHeader>(capacity, self.headers.capacity());
        Ok(ControlHistory {
            facts: self,
            pending: 1,
        })
    }
    #[cfg(all(test, feature = "s_incremental"))]
    pub(super) fn len(&self) -> usize {
        self.headers.len()
    }
    pub(super) fn verify_header(
        &self,
        generation: i64,
        batch_id: &str,
        session_generation: Option<&str>,
        clock_plan_hash: Option<&str>,
        commit_ns: Option<&str>,
    ) -> Result<(), String> {
        let expected = self
            .headers
            .get(&GenerationKey(generation, batch_id))
            .ok_or_else(|| err("v2批次控制头缺待验证期望或重复消费"))?;
        if session_generation != Some(self.session_generation.as_str())
            || clock_plan_hash != Some(self.clock_plan_hash.as_str())
            || commit_ns != Some(expected.commit_ns.as_str())
        {
            return Err(err("发布batch与真实语义clock/session不一致"));
        }
        Ok(())
    }
}
impl ControlHistory {
    #[cfg(test)]
    pub(super) fn session_generation(&self) -> &str {
        self.facts.session_generation.as_str()
    }
    #[cfg(test)]
    pub(super) fn clock_plan_hash(&self) -> &str {
        self.facts.clock_plan_hash.as_str()
    }
    #[cfg(test)]
    pub(super) fn pending_count(&self) -> usize {
        self.pending
    }
    pub(super) fn remaining_commit(&self, generation: i64, batch_id: &str) -> Option<&str> {
        self.facts
            .headers
            .get(&GenerationKey(generation, batch_id))
            .filter(|header| !header.consumed)
            .map(|header| header.commit_ns.as_str())
    }
    /// 校验失败不消费；成功只改同一分配内的消费标记，不复制或丢失原头。
    pub(super) fn consume_header(
        &mut self,
        generation: i64,
        batch_id: &str,
        session_generation: Option<&str>,
        clock_plan_hash: Option<&str>,
        commit_ns: Option<&str>,
    ) -> Result<(), String> {
        if self.remaining_commit(generation, batch_id).is_none() {
            return Err(err("v2批次控制头缺待验证期望或重复消费"));
        }
        self.facts.verify_header(
            generation,
            batch_id,
            session_generation,
            clock_plan_hash,
            commit_ns,
        )?;
        self.facts
            .headers
            .get_mut(&GenerationKey(generation, batch_id))
            .unwrap()
            .consumed = true;
        self.pending -= 1;
        Ok(())
    }
    /// 只有全部 pending 头闭合后才可移交纯所有权事实；不签发跨事务资格。
    pub(super) fn into_facts(self) -> Result<ControlFacts, String> {
        if self.pending != 0 {
            return Err(err("v2发布批次控制头未全部验证"));
        }
        Ok(self.facts)
    }
    pub(super) fn finish(self) -> Result<(), String> {
        self.into_facts().map(drop)
    }
}

/// 全局控制行通过后留下的逐代期望；结构核验必须逐项消费并检查闭合。
pub(super) enum ExpectedControl {
    Legacy,
    V2 { history: ControlHistory },
}

impl ExpectedControl {
    /// 旧 v2 发布入口：**先逐字核 `s-session/2` 标记**，再走 v2/S3 共用的消费核心。
    /// 标记不是 `s-session/2`（例如新域 manifest 的 `s-session/3`）一律拒绝，不放宽。
    pub(super) fn observe_batch(
        &mut self,
        generation: i64,
        batch_id: &str,
        header: &BatchControlHeader,
    ) -> Result<(), String> {
        let Self::V2 { history } = self else {
            return Ok(());
        };
        if header.protocol_revision.as_deref().map(|v| v.as_str()) != Some("s-session/2") {
            return Err(err("发布batch协议标记不是 s-session/2"));
        }
        history.consume_header(
            generation,
            batch_id,
            header.session_generation.as_deref().map(|v| v.as_str()),
            header.clock_plan_hash.as_deref().map(|v| v.as_str()),
            header.semantic_commit_ns.as_deref().map(|v| v.as_str()),
        )
    }

    /// 既有调用点的一行别名（`s_structure_session.rs` 仍调 `verify_header`）。
    pub(super) fn verify_header(
        &mut self,
        generation: i64,
        batch_id: &str,
        header: &BatchControlHeader,
    ) -> Result<(), String> {
        self.observe_batch(generation, batch_id, header)
    }

    pub(super) fn finish(self) -> Result<(), String> {
        match self {
            Self::Legacy => Ok(()),
            Self::V2 { history } => history.finish(),
        }
    }
}

/// #1527 私有只读连接复用根的完整 S2/legacy 审计，不另写判据。
/// 原入口包含全部可达批次、Delta、控制史、独立索引和来源对拍。
pub(crate) fn audit_private_startup(
    conn: &Connection,
    before_write: Option<&std::cell::Cell<crate::DeltaShape>>,
) -> Result<(), String> {
    crate::verified_root_raw_events_before_write(conn, before_write).map(|_| ())
}

/// 新鲜检查所有全局控制行，再准备逐代控制头期望；返回不代表批次控制审计通过。
/// 不能按generation过滤未来/同代损坏或信任自报摘要。
pub(super) fn prepare_control(
    conn: &Connection,
    before_write: Option<&std::cell::Cell<crate::DeltaShape>>,
) -> Result<ExpectedControl, String> {
    let enabled = active(conn)?;
    verify_schema(conn, enabled, before_write)?;
    let epoch_facts = verify_epoch_history(conn, enabled)?;
    if !enabled {
        return Ok(ExpectedControl::Legacy);
    }
    Ok(ExpectedControl::V2 {
        history: validate_control_history(conn, "s-session/2", epoch_facts)?,
    })
}

/// v2 与 S3 共用的**唯一一份**控制历史判断（原 `prepare_control` 主体，逐字保留）。
///
/// `protocol_revision` 由入口显式传 `s-session/2` 或 `s-session/3`：它同时用于
/// `s_protocol_meta.protocol_revision` 的逐字比对与每条持久消息 `canonical_envelope` 的
/// 公共头校验；其它取值一律拒绝，不提供默认值、不从库内自称反推。
pub(super) fn validate_control_history(
    conn: &Connection,
    protocol_revision: &str,
    epoch_facts: Vec<(String, i64)>,
) -> Result<ControlHistory, String> {
    let mut facts = control_json::new_work(None);
    for (ns, g) in epoch_facts {
        facts
            .try_push((
                TextValue::Plain(NativeValue::<String>::from_unscoped(ns)),
                g,
            ))
            .map_err(err)?;
    }
    validate_control_history_with_owner(conn, protocol_revision, facts, None)
}

enum ControlBind<'a> {
    Text(&'a str),
    Integer(i64),
}
fn control_query_row<T>(
    conn: &Connection,
    sql: &CStr,
    params: &[ControlBind<'_>],
    read: impl FnOnce(&mut CheckedRow<'_>) -> Result<T, String>,
) -> Result<T, String> {
    let mut stmt = CheckedStatement::prepare(conn, sql).map_err(err)?;
    for (i, p) in params.iter().enumerate() {
        match p {
            ControlBind::Text(v) => stmt.bind_text(i as i32 + 1, v),
            ControlBind::Integer(v) => stmt.bind_integer(i as i32 + 1, *v),
        }
        .map_err(err)?;
    }
    let mut row = stmt
        .next()
        .map_err(err)?
        .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
    let value = read(&mut row).map_err(err)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    Ok(value)
}
fn control_canonical_text_admitted(
    v: JsonView<'_>,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<TextValue, String> {
    control_native_owner(owner, work)?;
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(owner) = owner {
        return TextValue::from_bytes(control_aj::canonical_bytes(v, owner).map_err(err)?)
            .map_err(Into::into);
    }
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned规范文本未启用".into());
    }
    if work.is_none() {
        return serde_json::to_string(&v)
            .map(|s| TextValue::Plain(NativeValue::<String>::from_unscoped(s)))
            .map_err(err);
    }
    struct AdmittedWriter<'a> {
        bytes: CodecVec<u8>,
        work: Option<&'a WorkScope>,
        failure: Option<String>,
    }
    impl std::io::Write for AdmittedWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if let Err(error) = control_extend(&mut self.bytes, bytes, self.work) {
                self.failure = Some(err(error));
                return Err(std::io::Error::other("Native canonical text admission"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = AdmittedWriter {
        bytes: control_json::new_work(None),
        work,
        failure: None,
    };
    let result = serde_json::to_writer(&mut writer, &v);
    if let Some(error) = writer.failure {
        return Err(error);
    }
    result.map_err(err)?;
    TextValue::from_bytes(writer.bytes).map_err(err)
}
fn control_identity_admitted(
    input: &tb02a::RawInputView<'_>,
    ev: tb02a::RawEventView<'_>,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<TextValue, String> {
    control_native_owner(owner, work)?;
    let mut bytes = control_json::new_work(owner);
    for (i, s) in [
        input.source_namespace,
        input.source_epoch,
        input.instrument,
        ev.event_id,
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            control_push(&mut bytes, b'|', work).map_err(err)?;
        }
        control_extend(&mut bytes, s.as_bytes(), work).map_err(err)?;
    }
    TextValue::from_bytes(bytes).map_err(err)
}
// 固定字段直接借用原事件；与 canonical_event_content 的 tick/OHLC 两种字节形状一致。
fn control_event_hash(
    ev: tb02a::RawEventView<'_>,
    owner: Option<&OwnedBudget>,
) -> Result<newchan_rust::session_protocol::FixedHash, String> {
    let base = [
        ("event_id", JsonView::String(ev.event_id)),
        ("revision", JsonView::String(ev.revision)),
        ("seq", JsonView::String(ev.seq)),
        ("received_at", JsonView::String(ev.received_at)),
        ("raw_text", JsonView::String(ev.raw_text)),
        ("timestamp", JsonView::String(ev.timestamp)),
        ("volume", JsonView::String(ev.volume)),
    ];
    if let Some(p) = ev.ohlc {
        let fields = [
            base[0],
            base[1],
            base[2],
            base[3],
            base[4],
            base[5],
            base[6],
            ("open", JsonView::String(p.open)),
            ("high", JsonView::String(p.high)),
            ("low", JsonView::String(p.low)),
            ("close", JsonView::String(p.close)),
        ];
        newchan_rust::session_protocol::canonical_hash_view_with_owner(
            JsonView::object_fields(&fields).map_err(err)?,
            owner,
        )
    } else {
        let fields = [
            base[0],
            base[1],
            base[2],
            base[3],
            base[4],
            base[5],
            base[6],
            ("price", JsonView::String(ev.price)),
        ];
        newchan_rust::session_protocol::canonical_hash_view_with_owner(
            JsonView::object_fields(&fields).map_err(err)?,
            owner,
        )
    }
}
fn control_sorted_keys_admitted<'a, K: Eq + std::hash::Hash + Ord, V>(
    map: &'a BusinessMap<K, V>,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<CodecVec<&'a K>, String> {
    control_native_owner(owner, work)?;
    let mut keys = control_json::new_work(owner);
    for (key, _) in map.iter() {
        control_push(&mut keys, key, work).map_err(err)?;
    }
    keys.sort_unstable();
    Ok(keys)
}
/// 持久 message 的行解码与业务谓词，由 cold 和固定写口回读共用。
pub(super) struct AuditMessage {
    pub(super) ns: TextValue,
    pub(super) se: TextValue,
    pub(super) id: TextValue,
    pub(super) hash: TextValue,
    pub(super) raw: TextValue,
    pub(super) receipt: TextValue,
    pub(super) seq: i64,
    pub(super) status: TextValue,
    pub(super) published: Option<i64>,
    pub(super) event: TextValue,
    pub(super) count: i64,
}
#[cfg(feature = "s_incremental")]
pub(super) fn read_audit_message(
    r: &mut CheckedRow<'_>,
    owner: Option<&OwnedBudget>,
) -> Result<AuditMessage, String> {
    read_audit_message_admitted(r, owner, None)
}
pub(super) fn read_audit_message_admitted(
    r: &mut CheckedRow<'_>,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<AuditMessage, String> {
    control_native_owner(owner, work)?;
    let ns = TextValue::from_sql_admitted(r, 0, owner, work).map_err(err)?;
    let se = TextValue::from_sql_admitted(r, 1, owner, work).map_err(err)?;
    let id = TextValue::from_sql_admitted(r, 2, owner, work).map_err(err)?;
    let hash = TextValue::from_sql_admitted(r, 3, owner, work).map_err(err)?;
    let raw = TextValue::from_sql_admitted(r, 4, owner, work).map_err(err)?;
    let receipt = TextValue::from_sql_admitted(r, 5, owner, work).map_err(err)?;
    let seq = r.integer(6).map_err(err)?;
    let status = TextValue::from_sql_admitted(r, 7, owner, work).map_err(err)?;
    let published = r.optional_integer(8).map_err(err)?;
    let event = TextValue::from_sql_admitted(r, 9, owner, work).map_err(err)?;
    let count = r.integer(10).map_err(err)?;
    Ok(AuditMessage {
        ns,
        se,
        id,
        hash,
        raw,
        receipt,
        seq,
        status,
        published,
        event,
        count,
    })
}
#[cfg(feature = "s_incremental")]
pub(super) fn validate_audit_message(
    conn: &Connection,
    m: &AuditMessage,
    protocol_revision: &str,
    sid: &str,
    inc: &str,
    clock: JsonView<'_>,
    first_pub: Option<i64>,
    owner: Option<&OwnedBudget>,
) -> Result<TextValue, String> {
    validate_audit_message_admitted(
        conn,
        m,
        protocol_revision,
        sid,
        inc,
        clock,
        first_pub,
        owner,
        None,
    )
}
pub(super) fn validate_audit_message_admitted(
    conn: &Connection,
    m: &AuditMessage,
    protocol_revision: &str,
    sid: &str,
    inc: &str,
    clock: JsonView<'_>,
    first_pub: Option<i64>,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<TextValue, String> {
    control_native_owner(owner, work)?;
    use control_json::field;
    use newchan_rust::session_protocol::validate_envelope_view_with_owner;
    let AuditMessage {
        ns,
        se,
        id,
        hash,
        raw,
        receipt,
        seq,
        status,
        published,
        event,
        count,
    } = m;
    let (seq, count, published) = (*seq, *count, *published);
    let envelope = control_decode_admitted(&raw, owner, false, work).map_err(err)?;
    let e = validate_envelope_view_with_owner(envelope.view(), protocol_revision, owner)
        .map_err(err)?;
    if !control_canonical_equal(e, &raw, owner)?
        || field(e, "source_namespace").as_str() != Some(ns.as_str())
        || field(e, "source_epoch").as_str() != Some(se.as_str())
        || field(e, "message_id").as_str() != Some(id.as_str())
        || field(e, "payload_hash").as_str() != Some(hash.as_str())
        || field(e, "session_id").as_str() != Some(sid)
        || field(e, "session_generation").as_str() != Some(inc)
        || field(field(e, "payload"), "clock_event_id").as_str() != Some(event.as_str())
        || count < 0
        || seq < 0
    {
        return Err(err("消息独立列/规范原件不一致"));
    }
    validate_ingest_view(e).map_err(err)?;
    let input = tb02a::parse_input_view(field(field(e, "payload"), "raw_input")).map_err(err)?;
    if input.events_len() != 1 {
        return Err(err("持久输入消息非单事件"));
    }
    let ev = input.event(0).map_err(err)?;
    let ik = control_identity_admitted(&input, ev, owner, work)?;
    let actual = control_query_row(
        conn,
        c"SELECT receipt_id,payload_hash,identity_key,revision FROM raw_events WHERE seq=?1",
        &[ControlBind::Integer(seq)],
        |mut r| {
            Ok((
                TextValue::from_sql_admitted(&mut r, 0, owner, work)?,
                TextValue::from_sql_admitted(&mut r, 1, owner, work)?,
                TextValue::from_sql_admitted(&mut r, 2, owner, work)?,
                r.integer(3).map_err(|e| e.to_string())?,
            ))
        },
    )?;
    if &actual.0 != receipt
        || actual.1.as_str() != control_event_hash(ev, owner).map_err(err)?.as_str()
        || actual.2 != ik
        || actual.3 != parse_canonical_i64(ev.revision, "revision").map_err(err)?
    {
        return Err(err("消息/raw receipt内容或身份不一致"));
    }
    if published != first_pub
        || status.as_str()
            != if published.is_some() {
                "Committed"
            } else {
                "AcceptedPending"
            }
    {
        return Err(err("消息持久结果与最早已发布前沿不符"));
    }
    newchan_rust::session_protocol::clock_phase_view(clock, &event, "accept", -1).map_err(err)?;
    let key_fields = [
        JsonView::String(&ns),
        JsonView::String(&se),
        JsonView::String(&id),
    ];
    return control_canonical_text_admitted(JsonView::ArraySlice(&key_fields), owner, work);
}

/// 单 phase 与实际消息/根/clock 的关系核；唯一性与连续性由各自完整索引闭合。
pub(super) fn validate_audit_phase(
    id: &str,
    phase: &str,
    index: i64,
    ns: &str,
    key: &str,
    ord: Option<i64>,
    g: i64,
    root_g: i64,
    next: i64,
    clock: JsonView<'_>,
    message: Option<(&str, i64, Option<i64>)>,
) -> Result<(), String> {
    let next_g = root_g.checked_add(1).ok_or_else(|| err("generation溢出"))?;
    if newchan_rust::session_protocol::clock_phase_view(clock, id, phase, index).map_err(err)? != ns
        || g < 0
        || g > next_g
    {
        return Err(err("phase时间/代际与计划不一致"));
    }
    if phase == "recover" {
        if index != -1 || ord.is_some() || !key.is_empty() || g > root_g {
            return Err(err("recover phase归属错误"));
        }
    } else {
        let (expected_key, count, published) = message.ok_or_else(|| err("phase没有原消息"))?;
        if key != expected_key {
            return Err(err("phase消息归属错误"));
        }
        if phase == "accept" {
            if index != -1 || ord.is_some() || g > root_g {
                return Err(err("accept phase域错误"));
            }
        } else if (phase != "begin" && phase != "commit")
            || index < 0
            || index >= count
            || ord.is_none_or(|o| o < 1 || o >= next)
        {
            return Err(err("attempt phase域错误"));
        }
        if phase == "begin" && g != published.unwrap_or(next_g) {
            return Err(err("Begin代际与消息推进目标不同"));
        }
        if phase == "commit" && published != Some(g) {
            return Err(err("commit phase与实际首次发布不一致"));
        }
    }
    Ok(())
}

pub(super) fn validate_delivery_policy(
    ps: &str,
    pi: &str,
    pr: &str,
    keep: i64,
    first: i64,
    head: i64,
    sid: &str,
    inc: &str,
    root: i64,
) -> Result<(), String> {
    if ps != sid
        || pi != inc
        || pr != "generation-window/1"
        || keep < 1
        || head != root
        || first != 1.max(head - keep + 1)
    {
        return Err(err("普通投递frontier/policy与根不一致"));
    }
    Ok(())
}
pub(super) fn validate_attempt_summary(
    count: i64,
    published: Option<i64>,
    has_accept: bool,
    indexes: &[i64],
    same_pairs: bool,
    last_commit: bool,
) -> Result<(), String> {
    if !has_accept {
        return Err(err("消息缺接纳phase"));
    }
    if indexes.len() as i128 != count as i128
        || indexes
            .iter()
            .enumerate()
            .any(|(i, n)| i as i128 != *n as i128)
    {
        return Err(err("消息attempt_count与实际Begin集合不闭合"));
    }
    if !same_pairs {
        return Err(err("Begin/Commit ordinal/generation不一致"));
    }
    if count > 0 && published.is_some() && !last_commit {
        return Err(err("已发布消息缺最后Commit phase"));
    }
    Ok(())
}

pub(super) fn validate_control_history_with_owner(
    conn: &Connection,
    protocol_revision: &str,
    epoch_facts: CodecVec<(TextValue, i64)>,
    owner: Option<&OwnedBudget>,
) -> Result<ControlHistory, String> {
    validate_control_history_admitted(conn, protocol_revision, epoch_facts, owner, None)
}
fn verify_control_catalog_admitted(
    conn: &Connection,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    use control_json::field;
    use newchan_rust::session_protocol::{canonical_hash_view_with_owner, required_view};
    let catalog_raw = control_meta_text_admitted(conn, "catalog_definition", owner, work)?
        .ok_or_else(|| err("v2缺catalog来源"))?;
    let catalog_hash = control_meta_text_admitted(conn, "catalog_hash", owner, work)?
        .ok_or_else(|| err("v2缺catalog hash"))?;
    let catalog = control_decode_admitted(&catalog_raw, owner, false, work).map_err(err)?;
    let catalog_view = catalog.view();
    if !control_canonical_equal(catalog_view, &catalog_raw, owner)?
        || canonical_hash_view_with_owner(catalog_view, owner)
            .map_err(err)?
            .as_str()
            != catalog_hash.as_str()
        || field(catalog_view, "catalog_revision").as_str()
            != control_meta_text_admitted(conn, "catalog_revision", owner, work)?.as_deref()
    {
        return Err(err("目录来源/hash/revision不一致"));
    }
    let items = field(catalog_view, "items")
        .as_array()
        .ok_or_else(|| err("目录源items缺失"))?;
    let actual_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM catalog", [], |r| r.get(0))
        .map_err(err)?;
    if actual_count as usize != items.len() {
        return Err(err("目录静态行数不同"));
    }
    for item in items.iter() {
        let actual = control_query_row(
            conn,
            c"SELECT kind,title,domain,branches_json FROM catalog WHERE catalog_id=?1",
            &[ControlBind::Text(required_view(item, "id").map_err(err)?)],
            |r| {
                Ok((
                    TextValue::from_sql_admitted(r, 0, owner, work)?,
                    TextValue::from_sql_admitted(r, 1, owner, work)?,
                    TextValue::from_sql_admitted(r, 2, owner, work)?,
                    TextValue::from_sql_admitted(r, 3, owner, work)?,
                ))
            },
        )?;
        if field(item, "kind").as_str() != Some(actual.0.as_str())
            || field(item, "title").as_str() != Some(actual.1.as_str())
            || field(item, "domain").as_str() != Some(actual.2.as_str())
            || field(item, "branches")
                != control_decode_admitted(&actual.3, owner, false, work)
                    .map_err(err)?
                    .view()
        {
            return Err(err("目录静态内容与来源不同"));
        }
    }
    Ok(())
}

pub(super) fn validate_control_history_admitted(
    conn: &Connection,
    protocol_revision: &str,
    mut epoch_facts: CodecVec<(TextValue, i64)>,
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<ControlHistory, String> {
    control_native_owner(owner, work)?;
    if !matches!(protocol_revision, "s-session/2" | "s-session/3") {
        return Err(err(format!(
            "协议版本参数只允许 s-session/2 或 s-session/3，收到 `{protocol_revision}`"
        )));
    }
    // 拒绝未启用Owned入口，不允许在空集合路径上退回普通存储。
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if owner.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned控制历史未启用".into());
    }
    for (table, count_sql, singleton_sql) in [
        (
            "s_protocol_meta",
            "SELECT COUNT(*) FROM s_protocol_meta",
            "SELECT singleton FROM s_protocol_meta",
        ),
        (
            "s_delivery_policy",
            "SELECT COUNT(*) FROM s_delivery_policy",
            "SELECT singleton FROM s_delivery_policy",
        ),
    ] {
        let count: i64 = conn.query_row(count_sql, [], |r| r.get(0)).map_err(err)?;
        let singleton: i64 = conn
            .query_row(singleton_sql, [], |r| r.get(0))
            .map_err(err)?;
        if count != 1 || singleton != 1 {
            return Err(err(format!("{table}必须全表唯一且singleton=1")));
        }
    }
    for key in [
        "profile_id",
        "profile_hash",
        "profile_definition",
        "input_profile",
    ] {
        if control_meta_text_admitted(conn, key, owner, work)?.is_none_or(|s| s.is_empty()) {
            return Err(err(format!("v2初始化缺完整{key}绑定")));
        }
    }
    positive(
        &control_meta_text_admitted(conn, "writer_epoch", owner, work)?
            .ok_or_else(|| err("缺writer_epoch"))?,
        "v2 writer_epoch",
    )
    .map_err(err)?;
    let clock = stored_control_clock_admitted(conn, owner, work)?;
    let (protocol, inc, next, frontier) = control_query_row(conn,c"SELECT protocol_revision,session_generation,next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1", &[], |r| Ok((TextValue::from_sql_admitted(r,0,owner, work)?,TextValue::from_sql_admitted(r,1,owner, work)?,r.integer(2).map_err(|e|e.to_string())?,TextValue::from_sql_admitted(r,3,owner, work)?)))?;
    if protocol != protocol_revision || next < 1 {
        return Err(err("协议版本/attempt域错误"));
    }
    positive(&inc, "session_generation").map_err(err)?;
    let root_g = control_meta_i64_admitted(conn, "generation", owner, work)?;
    let sid = control_meta_text_admitted(conn, "session_id", owner, work)?
        .ok_or_else(|| err("缺session"))?;
    verify_control_catalog_admitted(conn, owner, work)?;
    let (ps,pi,pr,keep,first,head) = control_query_row(conn,c"SELECT session_id,session_generation,policy_revision,retain_generations,first_available_generation,head_generation FROM s_delivery_policy WHERE singleton=1", &[], |r| Ok((TextValue::from_sql_admitted(r,0,owner, work)?,TextValue::from_sql_admitted(r,1,owner, work)?,TextValue::from_sql_admitted(r,2,owner, work)?,r.integer(3).map_err(|e|e.to_string())?,r.integer(4).map_err(|e|e.to_string())?,r.integer(5).map_err(|e|e.to_string())?)))?;
    validate_delivery_policy(&ps, &pi, &pr, keep, first, head, &sid, &inc, root_g)?;
    let mut stmt = conn
        .prepare("SELECT generation FROM s_delivery_refs ORDER BY generation")
        .map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    let mut refs = control_json::new_work(owner);
    while let Some(r) = rows.next().map_err(err)? {
        control_push(&mut refs, r.get::<_, i64>(0).map_err(err)?, work).map_err(err)?;
    }
    if refs.len() as i128 != (head as i128 - first as i128 + 1).max(0)
        || refs
            .iter()
            .enumerate()
            .any(|(i, g)| *g as i128 != first as i128 + i as i128)
    {
        return Err(err("普通投递引用缺失/未来/越保留界"));
    }
    let mut publication_frontiers = control_json::new_work(owner);
    {
        let mut q = conn
            .prepare("SELECT generation,input_frontier FROM structure_deltas ORDER BY generation")
            .map_err(err)?;
        let mut rows = q.query([]).map_err(err)?;
        while let Some(r) = rows.next().map_err(err)? {
            control_push(
                &mut publication_frontiers,
                (
                    r.get::<_, i64>(0).map_err(err)?,
                    r.get::<_, i64>(1).map_err(err)?,
                ),
                work,
            )
            .map_err(err)?;
        }
    }
    if publication_frontiers.windows(2).any(|w| w[0].1 > w[1].1) {
        return Err(err("发布前沿倒退"));
    }
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT source_namespace,source_epoch,message_id,payload_hash,canonical_envelope,receipt_id,accepted_seq,status,first_published_generation,clock_event_id,attempt_count FROM s_input_messages ORDER BY accepted_seq,message_id").map_err(err)?;
    let mut messages = control_json::new_map(owner);
    let mut message_sequences = control_json::new_map::<i64, ()>(owner);
    while let Some(mut r) = stmt.next().map_err(err)? {
        let m = read_audit_message_admitted(&mut r, owner, work)?;
        let first_pub = publication_frontiers
            .get(publication_frontiers.partition_point(|(_, frontier)| *frontier < m.seq))
            .map(|(g, _)| *g);
        let key = validate_audit_message_admitted(
            conn,
            &m,
            protocol_revision,
            &sid,
            &inc,
            clock.value.view(),
            first_pub,
            owner,
            work,
        )?;
        let AuditMessage {
            seq,
            event,
            count,
            published,
            ..
        } = m;
        control_insert(&mut message_sequences, seq, (), work).map_err(err)?;
        if control_insert(&mut messages, event, (key, count, published), work)
            .map_err(err)?
            .is_some()
        {
            return Err(err("重复clock消息身份"));
        }
    }
    stmt.finalize().map_err(err)?;
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(err)?;
    if message_sequences.len() as i128 != raw_count as i128
        || control_sorted_keys_admitted(&message_sequences, owner, work)?
            .iter()
            .enumerate()
            .any(|(i, seq)| **seq as i128 != i as i128)
    {
        return Err(err("v2原始接纳序缺持久消息承接"));
    }
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT clock_event_id,phase,attempt_index,semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events ORDER BY semantic_ns,clock_event_id,phase,attempt_index").map_err(err)?;
    let mut seen = control_json::new_map(owner);
    let mut ordinals = control_json::new_map(owner);
    let mut commits_by_generation = control_json::new_map(owner);
    let mut begins_by_message: BusinessMap<TextValue, BusinessMap<i64, ()>> =
        control_json::new_map(owner);
    let mut recover_facts = control_json::new_work(owner);
    let mut max_ns = -1;
    while let Some(mut r) = stmt.next().map_err(err)? {
        let id = TextValue::from_sql_admitted(&mut r, 0, owner, work).map_err(err)?;
        let phase = TextValue::from_sql_admitted(&mut r, 1, owner, work).map_err(err)?;
        let index = r.integer(2).map_err(err)?;
        let ns = TextValue::from_sql_admitted(&mut r, 3, owner, work).map_err(err)?;
        let key = TextValue::from_sql_admitted(&mut r, 4, owner, work).map_err(err)?;
        let ord = r.optional_integer(5).map_err(err)?;
        let g = r.integer(6).map_err(err)?;
        validate_audit_phase(
            &id,
            &phase,
            index,
            &ns,
            &key,
            ord,
            g,
            root_g,
            next,
            clock.value.view(),
            messages
                .get(id.as_str())
                .map(|(k, c, p)| (k.as_str(), *c, *p)),
        )?;
        max_ns = max_ns.max(nonnegative(&ns, "semantic_ns").map_err(err)?);
        if phase == "recover" {
            control_push(
                &mut recover_facts,
                (ns.try_clone_admitted(work).map_err(err)?, g),
                work,
            )
            .map_err(err)?;
        } else {
            messages
                .get(id.as_str())
                .ok_or_else(|| err("phase没有原消息"))?;
            if phase == "begin"
                && control_insert(
                    &mut ordinals,
                    ord.unwrap(),
                    (id.try_clone_admitted(work).map_err(err)?, index),
                    work,
                )
                .map_err(err)?
                .is_some()
            {
                return Err(err("重复全局attempt"));
            }
            if phase == "begin" {
                if !begins_by_message.contains_key(id.as_str()) {
                    control_insert(
                        &mut begins_by_message,
                        id.try_clone_admitted(work).map_err(err)?,
                        control_json::new_map(owner),
                        work,
                    )
                    .map_err(err)?;
                }
                control_insert(
                    begins_by_message.get_mut(id.as_str()).unwrap(),
                    index,
                    (),
                    work,
                )
                .map_err(err)?;
            }
        }
        if phase == "commit"
            && control_insert(
                &mut commits_by_generation,
                g,
                (id.try_clone_admitted(work).map_err(err)?, index),
                work,
            )
            .map_err(err)?
            .is_some()
        {
            return Err(err("每代重复Commit phase"));
        }
        if control_insert(&mut seen, (id, phase, index), (ord, g), work)
            .map_err(err)?
            .is_some()
        {
            return Err(err("重复phase"));
        }
    }
    stmt.finalize().map_err(err)?;
    if parse_canonical_i64(&frontier, "logical_phase_frontier").map_err(err)? != max_ns
        || next as i128 != ordinals.len() as i128 + 1
        || control_sorted_keys_admitted(&ordinals, owner, work)?
            .iter()
            .enumerate()
            .any(|(i, o)| **o as i128 != i as i128 + 1)
    {
        return Err(err("phase frontier/attempt序列断裂"));
    }
    for id in &control_sorted_keys_admitted(&messages, owner, work)? {
        let (_, count, published) = messages.get(id.as_str()).unwrap();
        let indexes = begins_by_message.get(id.as_str());
        let mut sorted_indexes = control_json::new_work(owner);
        if let Some(indexes) = indexes {
            for (i, _) in indexes.iter() {
                control_push(&mut sorted_indexes, *i, work).map_err(err)?;
            }
        }
        sorted_indexes.sort_unstable();
        let same_pairs = sorted_indexes.iter().all(|i| {
            let begin = seen.get(&PhaseKey(id.as_str(), "begin", *i));
            begin.is_some()
                && seen
                    .get(&PhaseKey(id.as_str(), "commit", *i))
                    .is_none_or(|commit| Some(commit) == begin)
        });
        validate_attempt_summary(
            *count,
            *published,
            seen.contains_key(&PhaseKey(id.as_str(), "accept", -1)),
            &sorted_indexes,
            same_pairs,
            seen.contains_key(&PhaseKey(id.as_str(), "commit", count - 1)),
        )?;
    }
    epoch_facts.sort_unstable();
    recover_facts.sort_unstable();
    if *epoch_facts != *recover_facts {
        return Err(err("recover phase与writer换代时间/代际不闭合"));
    }
    let mut q = CheckedStatement::prepare(
        conn,
        c"SELECT generation,index_frontier FROM structure_deltas ORDER BY generation",
    )
    .map_err(err)?;
    let mut remaining = control_json::new_map(owner);
    while let Some(mut r) = q.next().map_err(err)? {
        let g = r.integer(0).map_err(err)?;
        let batch_id = TextValue::from_sql_admitted(&mut r, 1, owner, work).map_err(err)?;
        let (id, index) = commits_by_generation
            .get(&g)
            .ok_or_else(|| err("v2发布缺真实Commit phase"))?;
        control_insert(
            &mut remaining,
            (g, batch_id),
            ControlHeader {
                commit_ns: TextValue::from_str_admitted(
                    clock.phase(id, "commit", *index).map_err(err)?,
                    owner,
                    work,
                )
                .map_err(err)?,
                consumed: false,
            },
            work,
        )
        .map_err(err)?;
    }
    q.finalize().map_err(err)?;
    Ok(ControlHistory {
        pending: remaining.len(),
        facts: ControlFacts {
            session_generation: inc,
            clock_plan_hash: clock.hash,
            headers: remaining,
        },
    })
}

fn process_lock(db: &Path) -> Result<File, String> {
    session_transport::process_lock(db, "S")
}
fn init_v2(
    db: &Path,
    session: &str,
    catalog: &Path,
    profile: &Path,
    inc: &str,
    clock_path: &Path,
    keep: i64,
) -> Result<Value, String> {
    positive(inc, "session-generation")?;
    if keep < 1 {
        return Err("InvalidDomain：delivery retain必须大于0".into());
    }
    let clock = ClockPlan::load(clock_path)?;
    let (profile_def, profile_id, profile_hash) = load_profile(profile)?;
    let catalog_def = strict_json(&std::fs::read(catalog).map_err(err)?)?;
    let _lock = process_lock(db)?;
    let mut result = init_core(db, session, catalog)?;
    let mut conn = open_db(db)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(err)?;
    verify_reachable_root(&tx)?;
    tx.execute_batch(SCHEMA_V2).map_err(err)?;
    if profile_id == tb02a::PROFILE {
        tx.execute_batch(tb02a::SCHEMA).map_err(err)?;
    }
    meta_set(&tx, "protocol_revision", "s-session/2")?;
    meta_set(&tx, "profile_id", &profile_id)?;
    meta_set(&tx, "profile_hash", &profile_hash)?;
    meta_set(&tx, "input_profile", &profile_id)?;
    meta_set(&tx, "profile_definition", &canonical_json(&profile_def))?;
    meta_set(&tx, "catalog_definition", &canonical_json(&catalog_def))?;
    meta_set(
        &tx,
        "catalog_hash",
        &sha256_hex(canonical_json(&catalog_def).as_bytes()),
    )?;
    tx.execute(
        "INSERT INTO s_protocol_meta VALUES(1,'s-session/2',?1,?2,?3,1,'-1')",
        params![inc, clock.hash, canonical_json(&clock.value)],
    )
    .map_err(err)?;
    tx.execute(
        "INSERT INTO s_delivery_policy VALUES(1,?1,?2,'generation-window/1',?3,1,0)",
        params![session, inc, keep],
    )
    .map_err(err)?;
    verify_reachable_root(&tx)?;
    tx.commit().map_err(err)?;
    result["protocol_revision"] = json!("s-session/2");
    result["session_generation"] = json!(inc);
    result["clock_plan_hash"] = json!(clock.hash);
    result["profile_hash"] = json!(profile_hash);
    Ok(result)
}
pub(super) fn protocol_identity(conn: &Connection) -> Result<(String, String, String), String> {
    let (session, generation, epoch) = protocol_identity_with_owner(conn, None)?;
    Ok((
        session.to_string(),
        generation.to_string(),
        epoch.to_string(),
    ))
}
pub(crate) fn protocol_identity_with_owner(
    conn: &Connection,
    allocation: Option<&newchan_rust::s_content::capture_allocation::OwnedBudget>,
) -> Result<
    (
        crate::v3::business_json::TextValue,
        crate::v3::business_json::TextValue,
        crate::v3::business_json::TextValue,
    ),
    String,
> {
    Ok((
        control_meta_text(conn, "session_id", allocation)?.ok_or_else(|| err("缺session"))?,
        control_query_row(
            conn,
            c"SELECT session_generation FROM s_protocol_meta WHERE singleton=1",
            &[],
            |row| crate::v3::business_json::TextValue::from_sql(row, 0, allocation),
        )?,
        control_meta_text(conn, "writer_epoch", allocation)?.ok_or_else(|| err("缺epoch"))?,
    ))
}
/// 持久 message 行的机械读取（唯一查询/状态口径；v2 的 `lookup` 与 S3 域核验共用）。
pub(super) struct MessageRow {
    pub(super) payload_hash: String,
    pub(super) receipt_id: String,
    pub(super) accepted_seq: i64,
    pub(super) status: String,
    pub(super) first_published_generation: Option<i64>,
    pub(super) clock_event_id: String,
}
pub(super) fn message_row(
    conn: &Connection,
    ns: &str,
    se: &str,
    id: &str,
) -> Result<Option<MessageRow>, String> {
    conn.query_row("SELECT payload_hash,receipt_id,accepted_seq,status,first_published_generation,clock_event_id FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",params![ns,se,id],|r|Ok(MessageRow{payload_hash:r.get(0)?,receipt_id:r.get(1)?,accepted_seq:r.get(2)?,status:r.get(3)?,first_published_generation:r.get(4)?,clock_event_id:r.get(5)?})).optional().map_err(err)
}
/// 持久回执视图（唯一形状与 hash 判据；v2 `handle` 与 S3 域共用）。
pub(super) fn lookup(
    conn: &Connection,
    ns: &str,
    se: &str,
    id: &str,
    hash: &str,
) -> Result<Value, String> {
    let Some(MessageRow {
        payload_hash: stored,
        receipt_id: receipt,
        accepted_seq: seq,
        status,
        first_published_generation: g,
        ..
    }) = message_row(conn, ns, se, id)?
    else {
        return Ok(json!({"ok":true,"kind":"Unknown","original_message_id":id}));
    };
    if stored != hash {
        return Err("IdentityConflict：原message身份绑定不同内容".into());
    }
    let (publication, policy_records) = if let Some(g) = g {
        conn.query_row("SELECT generation,next_cut,index_frontier,catalog_revision,input_frontier,policy_records FROM structure_deltas WHERE generation=?1",params![g],|r|Ok((json!({"generation":r.get::<_,i64>(0)?.to_string(),"structure_cut":r.get::<_,String>(1)?,"index_frontier":r.get::<_,String>(2)?,"catalog_revision":r.get::<_,String>(3)?,"input_frontier":r.get::<_,i64>(4)?.to_string()}),Value::String(r.get::<_,String>(5)?)))).map_err(err)?
    } else {
        (Value::Null, Value::Null)
    };
    Ok(
        json!({"ok":true,"kind":status,"original_message_id":id,"receipt_id":receipt,"accepted_seq":seq.to_string(),"publication":publication,"policy_records":policy_records}),
    )
}
fn ready(conn: &Connection) -> Result<Value, String> {
    ready_for_protocol(conn, "s-session/2")
}
/// 控制就绪头共用机械字段；调用方已完成相应版本的完整状态核验。
pub(super) fn ready_for_protocol(conn: &Connection, protocol: &str) -> Result<Value, String> {
    if !matches!(protocol, "s-session/2" | "s-session/3")
        || meta_get_opt(conn, "protocol_revision")?.as_deref() != Some(protocol)
    {
        return Err("SchemaUnsupported：Ready协议与持久域不同".into());
    }
    let (sid, inc, epoch) = protocol_identity(conn)?;
    let clock = stored_clock(conn)?;
    let (first,head,keep):(i64,i64,i64)=conn.query_row("SELECT first_available_generation,head_generation,retain_generations FROM s_delivery_policy WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(err)?;
    Ok(
        json!({"ok":true,"kind":"Ready","schema_revision":protocol,"session_id":sid,"session_generation":inc,"writer_epoch":epoch,"profile_id":meta_get_opt(conn,"profile_id")?,"profile_hash":meta_get_opt(conn,"profile_hash")?,"clock_plan_hash":clock.hash,"advance_state":meta_get_opt(conn,"advance_state")?,"generation":meta_get_opt(conn,"generation")?,"structure_cut":meta_get_opt(conn,"structure_cut")?,"index_frontier":meta_get_opt(conn,"index_frontier")?,"delivery_frontier":{"first_available_generation":first.to_string(),"head_generation":head.to_string(),"min_valid_after_generation":(first-1).to_string(),"retain_generations":keep.to_string()}}),
    )
}
/// `ingest` payload 的业务形状（唯一判据；v2 `handle` 与 S3 域共用，不另写第二份宽/严副本）。
pub(super) fn validate_ingest(e: &Value) -> Result<(), String> {
    validate_ingest_view(e.into())
}
pub(super) fn validate_ingest_view(e: JsonView<'_>) -> Result<(), String> {
    use control_json::field;
    use newchan_rust::session_protocol::required_view;
    let p = field(e, "payload");
    let keys = [
        "op",
        "target_session_id",
        "target_session_generation",
        "writer_epoch",
        "clock_event_id",
        "raw_input",
    ];
    if p.as_object().map(|m| m.len()) != Some(keys.len())
        || p.as_object().unwrap().keys().any(|k| !keys.contains(&k))
    {
        return Err("SchemaUnsupported：ingest字段不完整/多余".into());
    }
    if field(p, "target_session_id") != field(e, "session_id")
        || field(p, "target_session_generation") != field(e, "session_generation")
    {
        return Err("IdentityConflict：target与公共头不同".into());
    }
    required_view(p, "clock_event_id")?;
    positive(required_view(p, "writer_epoch")?, "writer_epoch")?;
    let raw = field(p, "raw_input");
    tb02a::parse_input_view(raw)?;
    if field(raw, "session_id") != field(e, "session_id")
        || field(raw, "source_namespace") != field(e, "source_namespace")
        || field(raw, "source_epoch") != field(e, "source_epoch")
    {
        return Err("IdentityConflict：原始输入与公共头不同".into());
    }
    if field(raw, "events").as_array().map(|a| a.len()) != Some(1) {
        return Err("InvalidDomain：ingest必须一事件".into());
    }
    Ok(())
}

fn handle_with_policy(
    db: &Path,
    profile: &Path,
    epoch: &str,
    clock: &ClockPlan,
    e: &Value,
    production: bool,
) -> Result<Value, String> {
    handle_with_policy_and_config(db, profile, epoch, clock, e, production, None)
}

#[allow(clippy::too_many_arguments)]
fn handle_with_policy_and_config(
    db: &Path,
    profile: &Path,
    epoch: &str,
    clock: &ClockPlan,
    e: &Value,
    production: bool,
    _fixture_config: Option<&ThetaConfig>,
) -> Result<Value, String> {
    let conn = open_db(db)?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    verify_reachable_root(&tx)?;
    let (sid, inc, _) = protocol_identity(&tx)?;
    if e["session_id"] != sid || e["session_generation"] != inc {
        return Err("IdentityConflict：旧session/化身请求".into());
    }
    match required(&e["payload"], "op")? {
        "ready" => ready(&tx),
        "receipt" => {
            let p = &e["payload"];
            lookup(
                &tx,
                required(p, "original_source_namespace")?,
                required(p, "original_source_epoch")?,
                required(p, "original_message_id")?,
                required(p, "original_payload_hash")?,
            )
        }
        "ingest" => {
            validate_ingest(e)?;
            verify_writer_epoch(&tx, required(&e["payload"], "writer_epoch")?)?;
            if required(&e["payload"], "writer_epoch")? != epoch {
                return Err("StaleWriter：消息epoch与服务配置不同".into());
            }
            let old = lookup(
                &tx,
                required(e, "source_namespace")?,
                required(e, "source_epoch")?,
                required(e, "message_id")?,
                required(e, "payload_hash")?,
            )?;
            if old["kind"] == "Committed" {
                return Ok(old);
            }
            let c = InputContext {
                envelope: e.clone(),
                clock: clock.clone(),
            };
            // 接纳前核可执行的初始clock，不能持久一个完全无后继计划的消息。
            c.clock.phase(c.id(), "accept", -1)?;
            c.clock.phase(c.id(), "begin", 0)?;
            c.clock.phase(c.id(), "commit", 0)?;
            drop(tx);
            drop(conn);
            if old["kind"] == "Unknown" {
                accept_core(
                    db,
                    &canonical_json(&e["payload"]["raw_input"]),
                    profile,
                    epoch,
                    Some(&c),
                )?;
            }
            let advanced = if production {
                advance_core(db, epoch, Some(&c))?
            } else {
                #[cfg(test)]
                {
                    if let Some(config) = _fixture_config {
                        advance_core_with_config(db, epoch, Some(&c), config)?
                    } else {
                        advance_core_fixture(db, epoch, Some(&c))?
                    }
                }
                #[cfg(not(test))]
                unreachable!("fixture only exists in test builds")
            };
            let started = s_fact_cache::phase::begin();
            let conn = open_db(db)?;
            let tx = conn.unchecked_transaction().map_err(err)?;
            verify_reachable_root(&tx)?;
            let mut out = lookup(
                &tx,
                required(e, "source_namespace")?,
                required(e, "source_epoch")?,
                required(e, "message_id")?,
                required(e, "payload_hash")?,
            )?;
            s_fact_cache::phase::record("post_advance_root_audit", started.elapsed().as_nanos());
            if out["kind"] == "Committed" {
                if out["policy_records"] != advanced["policy_records"] {
                    return Err("IdentityConflict：外部回执与本次实际生成的记录版本戳不同".into());
                }
                out["policy_records"] = advanced["policy_records"].clone();
            }
            Ok(out)
        }
        _ => Err("SchemaUnsupported：未知op，恢复只允许正式监督器CLI".into()),
    }
}
fn response(
    e: &Value,
    result: Result<Value, String>,
    identity: &(String, String, String),
    control_instance_id: Option<&str>,
) -> Value {
    #[cfg_attr(not(feature = "s_incremental"), allow(unused_mut))]
    let mut payload = result.unwrap_or_else(|error| json!({"ok":false,"error":error,"identity_binding":"verified_at_service_start_only"}));
    let ready = payload["kind"] == "Ready";
    // #1640 C4：v2 serve 在增量构建里同样编入，Ready 须在摘要 payload 内自报构建身份（取值同 v3），
    // 供 launcher 拒绝 testonly 构建；默认 s_session 构建不发此字段，旧 v2 Ready 不变。
    #[cfg(feature = "s_incremental")]
    {
        if ready {
            payload["build_profile"] = json!(crate::v3::BUILD_PROFILE);
        }
    }
    let mut reply = newchan_rust::session_protocol::reply_envelope(
        e,
        payload,
        "s-session/2",
        &identity.0,
        &identity.1,
        "s-session/replies",
        &format!("S:{}", identity.0),
        &identity.2,
    );
    if ready {
        if let Some(value) = control_instance_id {
            reply["control_instance_id"] = json!(value);
        }
    }
    reply
}

#[derive(Clone)]
struct Bounds {
    frame: usize,
    read: Duration,
    write: Duration,
    response: Duration,
    cache_bytes: usize,
    queue: usize,
    connections: usize,
}
impl Bounds {
    fn transport(&self) -> TransportBounds {
        TransportBounds {
            frame: self.frame,
            read: self.read,
            write: self.write,
            response: self.response,
            queue: self.queue,
            connections: self.connections,
        }
    }
}

pub(crate) fn verify_serve_startup(
    conn: &Connection,
    db: &Path,
    epoch: &str,
    clock: &ClockPlan,
    before_write: Option<&std::cell::Cell<crate::DeltaShape>>,
    load_profile: impl FnOnce() -> Result<(Value, String, String), String>,
) -> Result<(), String> {
    verify_writer_epoch(conn, epoch)?;
    audit_private_startup(conn, before_write)?;
    if !active(conn)? {
        return Err("SchemaUnsupported：serve需要显式v2初始化".into());
    }
    check_clock(conn, clock)?;
    let (_, _, profile_hash) = load_profile()?;
    if meta_get_opt(conn, "profile_hash")?.as_deref() != Some(&profile_hash) {
        return Err("IdentityConflict：serve profile绑定不同".into());
    }
    if meta_get_opt(conn, "advance_state")?.as_deref() != Some("idle") {
        // #1576：历史规则库的 recover 必拒（VersionMismatch），不能再把运维指向它；闭门本身不放松。
        let rule = meta_get_opt(conn, "rule_revision")?;
        if rule.as_deref() != Some(RULE_REVISION) {
            return Err(stale_begin_on_historical_rule(rule.as_deref(), db));
        }
        return Err("StaleWriter：未完成Begin保持闭门，请监督器先正式recover".into());
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn handle_fixture(
    db: &Path,
    profile: &Path,
    epoch: &str,
    clock: &v2::ClockPlan,
    envelope: &Value,
) -> Result<Value, String> {
    handle_with_policy(db, profile, epoch, clock, envelope, false)
}

#[cfg(test)]
fn handle_fixture_with_config(
    db: &Path,
    profile: &Path,
    epoch: &str,
    clock: &v2::ClockPlan,
    envelope: &Value,
    config: &ThetaConfig,
) -> Result<Value, String> {
    handle_with_policy_and_config(db, profile, epoch, clock, envelope, false, Some(config))
}

fn serve_with_policy(
    db: &Path,
    socket: &Path,
    profile: &Path,
    epoch: &str,
    clock_path: &Path,
    bounds: Bounds,
    mut admission: crate::v3::admission_anchor::PreparedAdmission,
    production: bool,
) -> Result<(), String> {
    positive(epoch, "v2 writer_epoch")?;
    cache::configure(bounds.cache_bytes);
    // 监督器nonce仅用于把Ready绑定真实启动进程；绝不进入业务payload/hash/持久事实。
    let control_instance_id = std::env::var("S_CONTROL_INSTANCE_ID").ok();
    let clock = ClockPlan::parse(strict_json(&admission.read_file(clock_path, None)?)?)?;
    let scope = crate::v3::admission::WriterScope::admit_legacy_serve(
        db, admission, epoch, &clock, profile,
    )?;
    let _open_guard = scope.enter()?;
    let db = scope.db_path();
    scope.require_admitted()?;
    let conn = open_db(db)?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    verify_serve_startup(&tx, db, epoch, &clock, None, || load_profile(profile))?;
    let identity = protocol_identity(&tx)?;
    let pending = {
        let mut q=tx.prepare("SELECT canonical_envelope FROM s_input_messages WHERE status='AcceptedPending' ORDER BY accepted_seq,message_id").map_err(err)?;
        let out = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        out
    };
    drop(tx);
    drop(conn);
    for raw in pending {
        let e = envelope(strict_json(raw.as_bytes())?)?;
        let context = InputContext {
            envelope: e,
            clock: clock.clone(),
        };
        // 此身份已持久接纳：换代只取得新推进权，不能修改原payload中的旧writer_epoch。
        scope.require_admitted()?;
        if production {
            advance_core(db, epoch, Some(&context))?;
        } else {
            #[cfg(test)]
            {
                advance_core_fixture(db, epoch, Some(&context))?;
            }
            #[cfg(not(test))]
            unreachable!("fixture only exists in test builds")
        }
    }
    scope.finish_startup()?;
    serve_unix(socket, bounds.transport(), envelope, |e| {
        response(
            e,
            scope
                .require_admitted()
                .and_then(|()| handle_with_policy(db, profile, epoch, &clock, e, production)),
            &identity,
            control_instance_id.as_deref(),
        )
    })
}

/// #1576：历史规则库留有未完成 Begin 时 serve 的拒绝文案。recover 对历史规则只读库必拒，
/// 故只给实际可走的路径：CLI 只读查看，或人工以当前规则新建代际（本协议无原库迁移入口）。
/// `rule_revision` 键缺失时明示缺失，不显示空值（#1653，#1576 独审 INFO-3）。
fn stale_begin_on_historical_rule(rule: Option<&str>, db: &Path) -> String {
    let rule = match rule {
        Some(r) => format!("rule_revision=`{r}`"),
        None => "meta 缺 rule_revision 键".to_string(),
    };
    format!(
        "VersionMismatch：历史规则会话（{rule}，本二进制为 `{RULE_REVISION}`）留有未完成Begin，永久闭门：历史规则只读，recover 同样拒绝（不换代、不清未决Begin），本库不能再以 serve 提供服务。可走路径：①只读查看用 CLI `catalog`/`snapshot`/`meta --db {}`；②恢复服务须人工以当前规则走 v2 `init` 建新路径的库（`--db <新路径> --session --catalog --profile --session-generation <新代> --clock-plan --delivery-retain-generations` 缺一不可；不带后三者会建成 v1 库，serve 不收）并重新接纳原始输入——当前无旧库迁移入口（TB-10 #1368）",
        db.display()
    )
}

fn arg(args: &[String], key: &str) -> Result<String, String> {
    arg_value(args, key).ok_or_else(|| format!("缺 {key}"))
}
fn positive_arg(args: &[String], key: &str) -> Result<i64, String> {
    positive(&arg(args, key)?, key)
}
#[cfg(test)]
fn serve_fixture(
    db: &Path,
    socket: &Path,
    profile: &Path,
    epoch: &str,
    clock_path: &Path,
    bounds: Bounds,
    admission: crate::v3::admission_anchor::PreparedAdmission,
) -> Result<(), String> {
    serve_with_policy(
        db, socket, profile, epoch, clock_path, bounds, admission, false,
    )
}

pub fn dispatch(args: &[String]) -> Result<bool, String> {
    dispatch_with_policy(args, true)
}

#[cfg(test)]
pub fn dispatch_fixture(args: &[String]) -> Result<bool, String> {
    dispatch_with_policy(args, false)
}

fn dispatch_with_policy(args: &[String], production: bool) -> Result<bool, String> {
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    if cmd == "init"
        && [
            "--session-generation",
            "--clock-plan",
            "--delivery-retain-generations",
        ]
        .iter()
        .any(|k| args.iter().any(|a| a == k))
    {
        let out = init_v2(
            Path::new(&arg(args, "--db")?),
            &arg(args, "--session")?,
            Path::new(&arg(args, "--catalog")?),
            Path::new(&arg(args, "--profile")?),
            &arg(args, "--session-generation")?,
            Path::new(&arg(args, "--clock-plan")?),
            positive_arg(args, "--delivery-retain-generations")?,
        )?;
        println!("{out}");
        return Ok(true);
    }
    if cmd == "serve" {
        let bounds = Bounds {
            frame: usize::try_from(positive_arg(args, "--max-frame-bytes")?).map_err(err)?,
            read: Duration::from_millis(positive_arg(args, "--read-timeout-ms")? as u64),
            write: Duration::from_millis(positive_arg(args, "--write-timeout-ms")? as u64),
            response: Duration::from_millis(positive(
                &arg_value(args, "--response-timeout-ms")
                    .unwrap_or_else(|| arg_value(args, "--read-timeout-ms").unwrap_or_default()),
                "response-timeout-ms",
            )? as u64),
            cache_bytes: arg_value(args, "--audit-cache-source-bytes")
                .map(|s| {
                    parse_canonical_i64(&s, "audit-cache-source-bytes")
                        .and_then(|v| usize::try_from(v).map_err(err))
                })
                .transpose()?
                .unwrap_or(0),
            queue: usize::try_from(positive_arg(args, "--queue-capacity")?).map_err(err)?,
            connections: usize::try_from(positive_arg(args, "--max-connections")?).map_err(err)?,
        };
        let db = PathBuf::from(arg(args, "--db")?);
        let admission = crate::v3::admission_anchor::from_args_for_db(
            &db,
            args,
            crate::v3::admission_anchor::ControlEnv::capture(),
        )
        .map_err(|error| error.to_string())?;
        serve_with_policy(
            Path::new(&arg(args, "--db")?),
            Path::new(&arg(args, "--socket")?),
            Path::new(&arg(args, "--profile")?),
            &arg(args, "--writer-epoch")?,
            Path::new(&arg(args, "--clock-plan")?),
            bounds,
            admission,
            production,
        )?;
        return Ok(true);
    }
    if cmd == "recover"
        && ["--clock-plan", "--clock-event-id"]
            .iter()
            .any(|k| args.iter().any(|a| a == k))
    {
        let db = PathBuf::from(arg(args, "--db")?);
        let mut admission = crate::v3::admission_anchor::from_args_for_db(
            &db,
            args,
            crate::v3::admission_anchor::ControlEnv::capture(),
        )
        .map_err(|error| error.to_string())?;
        let clock = ClockPlan::parse(strict_json(
            &admission.read_file(Path::new(&arg(args, "--clock-plan")?), None)?,
        )?)?;
        let scope = crate::v3::admission::WriterScope::admit(&db, admission)?;
        let _open_guard = scope.enter()?;
        scope.require_admitted()?;
        let out = recover_core(
            scope.db_path(),
            &arg(args, "--new-epoch")?,
            Some((&clock, &arg(args, "--clock-event-id")?)),
        )?;
        scope.write_result(&out)?;
        scope.finish_startup()?;
        return Ok(true);
    }
    Ok(false)
}

// init/reset 保留旧锁分支，本提案不授予新库准备、重置或既存无锚接管资格。
pub enum LegacyWriteGuard {
    Initialization(
        #[allow(
            dead_code,
            reason = "#1527：文件持有本身保持 init/reset 锁直到 guard 析构"
        )]
        File,
    ),
    Existing(crate::v3::admission::WriterScope),
}
impl LegacyWriteGuard {
    pub(crate) fn enter(&self) -> Result<Option<crate::v3::admission::OpenGuard>, String> {
        match self {
            Self::Initialization(_) => Ok(None),
            Self::Existing(scope) => scope.enter().map(Some),
        }
    }
    pub fn require_admitted(&self) -> Result<(), String> {
        match self {
            Self::Initialization(_) => Ok(()),
            Self::Existing(scope) => scope.require_admitted(),
        }
    }
}

pub fn legacy_write_lock(args: &[String]) -> Result<Option<LegacyWriteGuard>, String> {
    let command = args.get(1).map(String::as_str);
    if matches!(command, Some("accept" | "advance" | "recover")) {
        let db = PathBuf::from(arg(args, "--db")?);
        let prepared = crate::v3::admission_anchor::from_args_for_db(
            &db,
            args,
            crate::v3::admission_anchor::ControlEnv::capture(),
        )
        .map_err(|error| error.to_string())?;
        return crate::v3::admission::WriterScope::admit(&db, prepared)
            .map(|scope| Some(LegacyWriteGuard::Existing(scope)));
    }
    if matches!(command, Some("init" | "reset")) {
        let db = PathBuf::from(arg(args, "--db")?);
        return process_lock(&db).map(|file| Some(LegacyWriteGuard::Initialization(file)));
    }
    Ok(None)
}

/// v2普通Watch只能走独立保留引用；旧v1数据仍由原命令实现读取。
pub fn retained_watch(conn: &Connection, after: i64) -> Result<Option<Value>, String> {
    // #1394 第二段（2D）：一次投递读取共享一个捕获作用域（Delta 原件同次只读一次）。
    let _capture = cache::begin_capture(crate::EVIDENCE_READ_BUDGET);
    if !active(conn)? {
        return Ok(None);
    }
    let (first,head):(i64,i64)=conn.query_row("SELECT first_available_generation,head_generation FROM s_delivery_policy WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(err)?;
    let (sid, inc, _) = protocol_identity(conn)?;
    let cut = meta_get_opt(conn, "structure_cut")?.ok_or_else(|| err("缺cut"))?;
    let gap = if after > head {
        json!({"reason":"cursor_ahead","missing_range":null,"rebuild_cut":cut,"rebuild_generation":head.to_string()})
    } else if after < first - 1 {
        json!({"reason":"retention_expired","missing_range":{"from_generation":(after+1).to_string(),"to_generation":(first-1).to_string()},"rebuild_cut":cut,"rebuild_generation":head.to_string()})
    } else {
        Value::Null
    };
    let deltas = if gap.is_null() {
        // #1394 第二段：Delta 原件在图域由 `delta/<generation>` 私有根承载，读取必须走同一入口。
        crate::delta_wire_rows(
            conn,
            "SELECT d.generation,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.delta_json FROM s_delivery_refs r JOIN structure_deltas d ON d.generation=r.generation WHERE d.generation>?1 ORDER BY d.generation",
            after,
        )
        .map_err(err)?
    } else {
        Vec::new()
    };
    Ok(Some(
        json!({"ok":true,"session_id":sid,"session_generation":inc,"generation":head.to_string(),"structure_cut":cut,"after_generation":after.to_string(),"gap":gap,"deltas":deltas,"delivery_frontier":{"first_available_generation":first.to_string(),"min_valid_after_generation":(first-1).to_string(),"head_generation":head.to_string()}}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_control_test_state(publications: &[(i64, &str, &str)]) -> ExpectedControl {
        let mut remaining = control_json::new_map(None);
        for (generation, batch_id, commit_ns) in publications {
            remaining
                .insert(
                    (*generation, TextValue::from_str(batch_id, None).unwrap()),
                    ControlHeader {
                        commit_ns: TextValue::from_str(commit_ns, None).unwrap(),
                        consumed: false,
                    },
                )
                .unwrap();
        }
        ExpectedControl::V2 {
            history: ControlHistory {
                pending: remaining.len(),
                facts: ControlFacts {
                    session_generation: TextValue::from_str("1", None).unwrap(),
                    clock_plan_hash: TextValue::from_str("test-clock-hash", None).unwrap(),
                    headers: remaining,
                },
            },
        }
    }

    fn expected_control_test_header(commit_ns: &str) -> BatchControlHeader {
        BatchControlHeader {
            protocol_revision: Some(share_header_text_admitted("s-session/2", None).unwrap()),
            session_generation: Some(share_header_text_admitted("1", None).unwrap()),
            clock_plan_hash: Some(share_header_text_admitted("test-clock-hash", None).unwrap()),
            semantic_commit_ns: Some(share_header_text_admitted(commit_ns, None).unwrap()),
        }
    }

    #[test]
    fn expected_control_requires_every_publication_before_finish() {
        let mut expected =
            expected_control_test_state(&[(1, "batch-a", "1020"), (2, "batch-b", "2020")]);
        expected
            .verify_header(1, "batch-a", &expected_control_test_header("1020"))
            .unwrap();
        assert!(
            expected.finish().is_err(),
            "仍有未消费发布时不得完成控制审计"
        );
    }

    #[test]
    fn expected_control_rejects_missing_and_repeated_consumption() {
        let mut expected = expected_control_test_state(&[(1, "batch-a", "1020")]);
        let header = expected_control_test_header("1020");
        assert!(expected.verify_header(2, "batch-a", &header).is_err());
        assert!(expected.verify_header(1, "batch-other", &header).is_err());
        expected.verify_header(1, "batch-a", &header).unwrap();
        assert!(expected.verify_header(1, "batch-a", &header).is_err());
        expected.finish().unwrap();
    }

    #[test]
    fn expected_control_consumes_reused_batch_id_by_generation() {
        let mut expected =
            expected_control_test_state(&[(1, "batch-a", "1020"), (2, "batch-a", "2020")]);
        expected
            .verify_header(1, "batch-a", &expected_control_test_header("1020"))
            .unwrap();
        expected
            .verify_header(2, "batch-a", &expected_control_test_header("2020"))
            .unwrap();
        expected.finish().unwrap();
    }

    /// v2 入口必须只认自己的协议标记：新域 manifest（`s-session/3`）不能借共用消费核心
    /// 混入旧域，也不能因为共用而放宽旧入口。
    #[test]
    fn v2_observe_batch_rejects_s3_marker_and_keeps_shared_consumption() {
        let mut expected = expected_control_test_state(&[(1, "batch-a", "1020")]);
        let mut s3 = expected_control_test_header("1020");
        s3.protocol_revision = Some(share_header_text_admitted("s-session/3", None).unwrap());
        assert!(expected.observe_batch(1, "batch-a", &s3).is_err());
        // 拒绝不消费：换成真实 v2 头仍必须逐字通过并闭合。
        expected
            .observe_batch(1, "batch-a", &expected_control_test_header("1020"))
            .unwrap();
        expected.finish().unwrap();
        // Legacy 域没有逐代期望，旧行为保持。
        let mut legacy = ExpectedControl::Legacy;
        assert!(legacy.observe_batch(1, "batch-a", &s3).is_ok());
    }

    #[test]
    fn expected_control_rejects_missing_fields_and_changed_clock_without_consuming() {
        for field in 0..6 {
            let mut expected = expected_control_test_state(&[(1, "batch-a", "1020")]);
            let correct = expected_control_test_header("1020");
            let mut bad = correct.clone();
            match field {
                0 => bad.protocol_revision = None,
                1 => bad.session_generation = None,
                2 => bad.clock_plan_hash = None,
                3 => bad.semantic_commit_ns = None,
                4 => {
                    bad.clock_plan_hash =
                        Some(share_header_text_admitted("other-clock-hash", None).unwrap())
                }
                _ => {
                    bad.semantic_commit_ns =
                        Some(share_header_text_admitted("01020", None).unwrap())
                }
            }
            assert!(expected.verify_header(1, "batch-a", &bad).is_err());
            expected.verify_header(1, "batch-a", &correct).unwrap();
            expected.finish().unwrap();
        }
    }

    /// #1394 第二段成本测量：用**冻结 9011 根来源**的前 N 根、与基线同一时钟计划
    /// （`ns = 100 + 3*i`、`long-s-%05d`、一源一代）跑真实 v2 会话，逐代记时并收集库统计。
    ///
    /// 默认不跑（`S_EVIDENCE_MEASURE=<N>` 显式驱动），因为它按定义是长跑而不是单测；
    /// 但它走的是与生产完全相同的 `handle/advance_core` 入口与真实 context，不另写旁路。
    #[test]
    #[ignore = "#1394 显式同源成本测量；须设置 S_EVIDENCE_MEASURE 并提供冻结原件"]
    fn issue1394_prefix_cost_measurement() {
        let spec =
            std::env::var("S_EVIDENCE_MEASURE").expect("显式成本测量须设置 S_EVIDENCE_MEASURE");
        let prefix: usize = spec.trim().parse().expect("S_EVIDENCE_MEASURE=<N>");
        // 与正式启动器同档：`s_resources.audit_cache_source_bytes` 默认 268435456；
        // 不设这一段就等于关掉审计前缀复用，会污染可比成本。
        cache::configure(268_435_456);
        const BAR_PATH: &str = "/Users/silencehan/Projects/NewChanlun-1394-tb03b/.scratch/issue1394-long-source-search/runs/RW05_u57_l12_40_a400_2000_r25_70_d2.bars.json";
        let text = std::fs::read_to_string(BAR_PATH).expect("冻结来源可读");
        assert_eq!(
            sha256_hex(text.as_bytes()),
            "76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087"
        );
        let bars: Vec<Value> = serde_json::from_str(&text).expect("冻结来源是 JSON");
        assert_eq!(bars.len(), 9011);
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let dir = std::env::temp_dir().join(format!(
            "issue1394-measure-{}-{}-{prefix}",
            std::process::id(),
            now_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("s.sqlite");
        let profile = base.join("profiles/ohlc_integer_tb02a_v1.json");
        let mut events = serde_json::Map::new();
        for i in 0..prefix {
            let ns = 100 + 3 * i as i64;
            events.insert(
                format!("long-s-{i:05}"),
                json!({"accept_ns": ns.to_string(),
                       "attempts": [{"begin_ns": (ns + 1).to_string(), "commit_ns": (ns + 2).to_string()}]}),
            );
        }
        let clock = ClockPlan::parse(json!({
            "schema_revision": "s-clock-plan/1",
            "clock_plan_id": "tb03b-long-s-full-source-v1",
            "origin_utc": "2026-09-14T00:00:00Z",
            "unit": "ns",
            "events": events,
        }))
        .unwrap();
        let clock_path = dir.join("clock.json");
        std::fs::write(&clock_path, canonical_json(&clock.value)).unwrap();
        const SID: &str = "tb03b-long-financial-v1";
        const SOURCE: &str = "testonly.ohlc.tb03b-long-financial-v1";
        init_v2(
            &db,
            SID,
            &base.join("catalog/signed-catalog.json"),
            &profile,
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        let mut per_generation = Vec::with_capacity(prefix);
        // #1394 第二段：逐代记录阶段计数增量（唯一节点读、历史 batch 展开/复用、根审计/stage/commit）。
        let mut per_generation_audit_ms = Vec::with_capacity(prefix);
        let mut per_generation_stage_ms = Vec::with_capacity(prefix);
        let mut per_generation_commit_ms = Vec::with_capacity(prefix);
        let mut per_generation_node_rows = Vec::with_capacity(prefix);
        let mut per_generation_batch_expansions = Vec::with_capacity(prefix);
        let mut per_generation_batch_hits = Vec::with_capacity(prefix);
        let mut total = std::time::Duration::ZERO;
        cache::counters_reset();
        for (i, bar) in bars[..prefix].iter().enumerate() {
            let ns = 100 + 3 * i as i64;
            let op = format!("long-s-{i:05}");
            let event = json!({
                "event_id": format!("long-raw-{i}"),
                "revision": "1",
                "seq": bar["source_index"].to_string(),
                "received_at": "2026-09-14T00:00:00Z",
                "raw_text": canonical_json(bar),
                "open": bar["open"].to_string(),
                "high": bar["high"].to_string(),
                "low": bar["low"].to_string(),
                "close": bar["close"].to_string(),
                "timestamp": bar["timestamp"].to_string(),
                "volume": "1",
            });
            let payload = json!({
                "op": "ingest",
                "target_session_id": SID,
                "target_session_generation": "1",
                "writer_epoch": "1",
                "clock_event_id": op,
                "raw_input": {"schema_revision": "s-ohlc/1", "session_id": SID,
                              "source_namespace": SOURCE, "source_epoch": "1",
                              "instrument": "TEST.LONG.TB03B", "profile": "ohlc_integer_tb02a_v1",
                              "events": [event]},
            });
            let message = json!({
                "schema_revision": "s-session/2", "session_id": SID, "session_generation": "1",
                "source_namespace": SOURCE, "source_epoch": "1",
                "producer_id": "tb03b-long-input", "producer_epoch": "1",
                "message_id": format!("long-s-{i:05}"), "causal_refs": [],
                "payload_hash": sha256_hex(canonical_json(&payload).as_bytes()),
                "payload": payload,
            });
            let started = Instant::now();
            let before = cache::counters_snapshot();
            handle_fixture(&db, &profile, "1", &clock, &envelope(message).unwrap()).unwrap();
            let spent = started.elapsed();
            let after = cache::counters_snapshot();
            per_generation_audit_ms.push(format!(
                "{:.3}",
                (after.audit_ns - before.audit_ns) as f64 / 1e6
            ));
            per_generation_stage_ms.push(format!(
                "{:.3}",
                (after.stage_ns - before.stage_ns) as f64 / 1e6
            ));
            per_generation_commit_ms.push(format!(
                "{:.3}",
                (after.commit_ns - before.commit_ns) as f64 / 1e6
            ));
            per_generation_node_rows
                .push((after.node_rows_read - before.node_rows_read).to_string());
            per_generation_batch_expansions
                .push((after.batch_expansions - before.batch_expansions).to_string());
            per_generation_batch_hits
                .push((after.batch_handle_hits - before.batch_handle_hits).to_string());
            total += spent;
            per_generation.push(spent.as_secs_f64());
            let _ = ns;
        }
        let write_phase = cache::counters_snapshot();
        let conn = open_db(&db).unwrap();
        let page_count: i64 = conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .unwrap();
        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap();
        let (nodes, node_payload) = evidence_store::stats(&conn).unwrap();
        let node_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM s_evidence_nodes", [], |r| r.get(0))
            .unwrap();
        let roots: i64 = conn
            .query_row("SELECT COUNT(*) FROM s_evidence_roots", [], |r| r.get(0))
            .unwrap();
        let batch_sum: i64 = conn
            .query_row("SELECT COALESCE(SUM(byte_len),0) FROM batches", [], |r| {
                r.get(0)
            })
            .unwrap();
        let blob_sum: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(length(canonical_bytes)),0) FROM batches",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let delta_sum: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(length(delta_json)),0) FROM structure_deltas",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 图域里 delta_json 列是空占位，故另按私有根展开计 Delta 逻辑原件字节。
        let delta_origin_total: i64 = {
            let format = evidence_store::format(&conn).unwrap();
            let mut reader = evidence_store::Reader::new(&conn, crate::EVIDENCE_READ_BUDGET);
            let mut stmt = conn
                .prepare("SELECT generation, delta_json FROM structure_deltas ORDER BY generation")
                .unwrap();
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut total = 0i64;
            for (generation, column) in rows {
                let text = crate::read_record_original_text(
                    &conn,
                    &mut reader,
                    format,
                    &evidence_store::delta_root_key(generation),
                    &column,
                    &format!("delta[{generation}]"),
                )
                .unwrap();
                total += text.len() as i64;
            }
            total
        };
        // 公开读路径的完整展开成本：先丢弃跨请求持久句柄（真正的冷读集），再逐代展开，
        // 逐字节核内容寻址与公开批次身份。
        cache::clear_handles();
        cache::counters_reset();
        let read_counters_before = cache::counters_snapshot();
        let read_started = Instant::now();
        let mut expanded = 0u64;
        {
            let mut stmt = conn
                .prepare("SELECT batch_id FROM batches ORDER BY batch_id")
                .unwrap();
            let ids = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            for id in ids {
                expanded += load_batch_bytes(&conn, &id).unwrap().len() as u64;
            }
        }
        let read_seconds = read_started.elapsed().as_secs_f64();
        let read_counters_after = cache::counters_snapshot();
        // 与基线逐代对拍（同一时钟计划 → 逐代 batch_id 必须逐字节相同）。
        let mut comparison = Value::Null;
        if let Ok(baseline) = Connection::open_with_flags(
            "/private/tmp/tb1394-s-prefix64-20260915/s.db",
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            let base_hash: String = baseline
                .query_row("SELECT clock_plan_hash FROM s_protocol_meta", [], |r| {
                    r.get(0)
                })
                .unwrap();
            let mine: String = conn
                .query_row("SELECT clock_plan_hash FROM s_protocol_meta", [], |r| {
                    r.get(0)
                })
                .unwrap();
            let mut same = mine == base_hash;
            let mut checked = 0i64;
            if same {
                let mut stmt = conn
                    .prepare("SELECT d.generation,b.batch_id,b.byte_len FROM structure_deltas d JOIN batches b ON b.batch_id=d.index_frontier ORDER BY d.generation")
                    .unwrap();
                let mine_rows = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, i64>(2)?,
                        ))
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                let mut stmt = baseline
                    .prepare("SELECT d.generation,b.batch_id,b.byte_len FROM structure_deltas d JOIN batches b ON b.batch_id=d.index_frontier ORDER BY d.generation")
                    .unwrap();
                let base_rows = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, i64>(2)?,
                        ))
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                same = mine_rows.len() <= base_rows.len()
                    && mine_rows.iter().zip(base_rows.iter()).all(|(a, b)| a == b);
                checked = mine_rows.len().min(base_rows.len()) as i64;
            }
            comparison = json!({"same_clock_plan": mine == base_hash,
                                "identical_generations": same, "checked": checked.to_string(),
                                "baseline_db": "/private/tmp/tb1394-s-prefix64-20260915/s.db"});
        }
        println!(
            "{}",
            json!({
                "schema": "issue1394-prefix-cost/1",
                "prefix": prefix.to_string(),
                "clock_plan_hash": conn.query_row("SELECT clock_plan_hash FROM s_protocol_meta", [], |r| r.get::<_, String>(0)).unwrap(),
                "wall_seconds_total": format!("{:.3}", total.as_secs_f64()),
                "wall_seconds_per_generation": per_generation.iter().map(|v| format!("{v:.4}")).collect::<Vec<_>>(),
                "db_files": {"sqlite_pages": page_count.to_string(), "page_size": page_size.to_string(),
                              "sqlite_bytes": (page_count * page_size).to_string()},
                "batches": {"rows": roots.to_string(), "expanded_bytes_total": batch_sum.to_string(),
                             "stored_blob_bytes": blob_sum.to_string()},
                "graph": {"nodes": nodes.to_string(), "node_rows": node_rows.to_string(),
                           "node_payload_bytes": node_payload.to_string(),
                           "roots": roots.to_string()},
                "delta_json_bytes_total": delta_sum.to_string(),
                "full_expansion_seconds": format!("{read_seconds:.3}"),
                "full_expansion_bytes": expanded.to_string(),
                "delta_origin_bytes_total": delta_origin_total.to_string(),
                "stages": {
                    "units": "毫秒与计数均为逐代增量；cache 命中/失效为同一捕获内的句柄/节点粒度",
                    "wall_ms_per_generation": per_generation.iter().map(|v| format!("{:.3}", v * 1000.0)).collect::<Vec<_>>(),
                    "audit_ms_per_generation": per_generation_audit_ms,
                    "stage_ms_per_generation": per_generation_stage_ms,
                    "commit_ms_per_generation": per_generation_commit_ms,
                    "unique_node_rows_read_per_generation": per_generation_node_rows,
                    "historical_batch_expansions_per_generation": per_generation_batch_expansions,
                    "historical_batch_handle_hits_per_generation": per_generation_batch_hits,
                    "write_phase_totals": {
                        "node_rows_read": write_phase.node_rows_read.to_string(),
                        "node_payload_bytes_read": write_phase.node_payload_bytes_read.to_string(),
                        "node_reused_in_capture": write_phase.node_reused_in_capture.to_string(),
                        "root_reads": write_phase.root_reads.to_string(),
                        "root_handle_hits": write_phase.root_handle_hits.to_string(),
                        "root_expansions": write_phase.root_expansions.to_string(),
                        "root_parses": write_phase.root_parses.to_string(),
                        "batch_handle_hits": write_phase.batch_handle_hits.to_string(),
                        "batch_expansions": write_phase.batch_expansions.to_string(),
                        "audit_ms_total": format!("{:.3}", write_phase.audit_ns as f64 / 1e6),
                        "stage_ms_total": format!("{:.3}", write_phase.stage_ns as f64 / 1e6),
                        "commit_ms_total": format!("{:.3}", write_phase.commit_ns as f64 / 1e6),
                    },
                    "cold_public_read_phase": {
                        "seconds": format!("{read_seconds:.3}"),
                        "bytes": expanded.to_string(),
                        "node_rows_read": (read_counters_after.node_rows_read - read_counters_before.node_rows_read).to_string(),
                        "root_expansions": (read_counters_after.root_expansions - read_counters_before.root_expansions).to_string(),
                        "root_handle_hits": (read_counters_after.root_handle_hits - read_counters_before.root_handle_hits).to_string(),
                        "cache_budget_bytes": "268435456",
                    },
                },
                "baseline_comparison": comparison,
                "db_dir": dir.display().to_string(),
            })
        );
    }

    use newchan_rust::session_transport::{
        enqueue_before_deadline, io_connection, read_frame, Job,
    };
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    };
    use std::time::Instant;
    static N: AtomicUsize = AtomicUsize::new(0);
    pub(super) fn with_mixed_control_fixture(run: impl FnOnce(&Connection)) {
        let fixture = Fixture::new(4, 1);
        fixture.ingest(0);
        fixture.ingest(1);
        let conn = open_db(&fixture.db).unwrap();
        run(&conn);
    }
    fn dump_db(db: &Path) -> Vec<(String, Vec<String>)> {
        let conn = open_db(db).unwrap();
        let names = [
            "meta",
            "raw_events",
            "batches",
            "objects",
            "witnesses",
            "relations",
            "observations",
            "structure_deltas",
            "s_storage_format",
            "writer_epoch_history",
            "catalog",
            "s_protocol_meta",
            "s_input_messages",
            "s_clock_events",
            "s_delivery_policy",
            "s_delivery_refs",
        ];
        names
            .iter()
            .map(|n| {
                let exists: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
                        params![n],
                        |r| r.get(0),
                    )
                    .unwrap();
                if exists == 0 {
                    return (n.to_string(), vec!["<absent>".to_string()]);
                }
                let mut q = conn
                    .prepare(&format!("SELECT * FROM {n} ORDER BY rowid"))
                    .unwrap();
                let cols = q.column_count();
                let rows = q
                    .query_map([], |r| {
                        Ok((0..cols)
                            .map(|i| format!("{:?}", r.get_ref(i).unwrap()))
                            .collect::<Vec<_>>()
                            .join("|"))
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                (n.to_string(), rows)
            })
            .collect()
    }
    struct Fixture {
        dir: PathBuf,
        db: PathBuf,
        profile: PathBuf,
        clock: ClockPlan,
    }
    impl Fixture {
        fn new(keep: i64, attempts: usize) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tb01c-v2-{}-{}-{}",
                std::process::id(),
                now_nanos(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir(&dir).unwrap();
            let db = dir.join("s.sqlite");
            let base = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("s_session");
            let profile = base.join("profiles/testonly_tick_1_1_ohlc.json");
            let mut events = serde_json::Map::new();
            for i in 0..16i64 {
                let t = 1000 * (i + 1);
                let a=(0..attempts).map(|n|json!({"begin_ns":(t+10+30*n as i64).to_string(),"commit_ns":(t+20+30*n as i64).to_string()})).collect::<Vec<_>>();
                events.insert(
                    format!("op-{i}"),
                    json!({"accept_ns":t.to_string(),"attempts":a}),
                );
            }
            events.insert("recover-1".into(), json!({"recover_ns":"1030"}));
            let clock=ClockPlan::parse(json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"testclock","origin_utc":"2000-01-01T00:00:00Z","unit":"ns","events":events})).unwrap();
            let cp = dir.join("clock.json");
            std::fs::write(&cp, canonical_json(&clock.value)).unwrap();
            init_v2(
                &db,
                "test-c",
                &base.join("catalog/signed-catalog.json"),
                &profile,
                "1",
                &cp,
                keep,
            )
            .unwrap();
            Self {
                dir,
                db,
                profile,
                clock,
            }
        }
        fn message(&self, i: i64) -> Value {
            let prices = [10000, 14000, 19000, 12000, 16000, 11000, 18000, 13000];
            let price = (prices[i as usize % 8] + i).to_string();
            let raw = json!({"schema_revision":"1","session_id":"test-c","source_namespace":"test.feed","source_epoch":"1","instrument":"TEST.TICK","profile":"testonly_tick_1_1_ohlc","events":[{"event_id":format!("event-{i}"),"revision":"1","seq":i.to_string(),"received_at":"2000-01-01T00:00:00Z","raw_text":price,"price":price,"timestamp":i.to_string(),"volume":"1"}]});
            self.wrap(json!({"op":"ingest","target_session_id":"test-c","target_session_generation":"1","writer_epoch":"1","clock_event_id":format!("op-{i}"),"raw_input":raw}),&format!("msg-{i}"))
        }
        fn wrap(&self, p: Value, id: &str) -> Value {
            json!({"schema_revision":"s-session/2","session_id":"test-c","session_generation":"1","source_namespace":"test.feed","source_epoch":"1","message_id":id,"producer_id":"test-driver","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&p).as_bytes()),"causal_refs":[],"payload":p})
        }
        fn ingest(&self, i: i64) -> Value {
            let m = envelope(self.message(i)).unwrap();
            handle_fixture(&self.db, &self.profile, "1", &self.clock, &m).unwrap()
        }
        fn dump(&self) -> Vec<(String, Vec<String>)> {
            dump_db(&self.db)
        }
        fn begin_only(&self) -> InputContext {
            let c = InputContext {
                envelope: self.message(0),
                clock: self.clock.clone(),
            };
            accept_core(
                &self.db,
                &canonical_json(&c.envelope["payload"]["raw_input"]),
                &self.profile,
                "1",
                Some(&c),
            )
            .unwrap();
            let mut conn = open_db(&self.db).unwrap();
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            verify_reachable_root(&tx).unwrap();
            let token = record_begin(&tx, Some(&c), 1, 0, "cut-0", "1").unwrap();
            meta_set(&tx, "advance_state", &format!("begun:{token}:1:0")).unwrap();
            meta_set(&tx, "begin_token", &token).unwrap();
            meta_set(&tx, "begin_generation", "1").unwrap();
            meta_set(&tx, "begin_input_frontier", "0").unwrap();
            tx.commit().unwrap();
            c
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// #1576 负控：历史规则库留有未完成 Begin 时 serve 仍闭门，但文案不再指向必拒的 recover；
    /// 文案给出的路径逐条实走：CLI 只读查看可用、当前规则新建代际可用，recover 确实必拒，库字节不变。
    #[test]
    fn v2_serve_stale_begin_on_historical_rule_points_to_walkable_paths() {
        let f = Fixture::new(8, 2);
        f.begin_only();
        let clock_path = f.dir.join("clock.json");
        let bounds = || Bounds {
            frame: 65536,
            read: Duration::from_millis(1000),
            write: Duration::from_millis(1000),
            response: Duration::from_millis(1000),
            cache_bytes: 0,
            queue: 4,
            connections: 4,
        };
        let socket = f.dir.join("s.sock");
        // 对照：当前规则库的未决 Begin 仍按原口径指向 recover（该路径对当前规则可走）。
        let current = serve_fixture(
            &f.db,
            &socket,
            &f.profile,
            "1",
            &clock_path,
            bounds(),
            crate::v3::admission_anchor::test_prepare(&f.db),
        )
        .unwrap_err();
        assert!(
            current.starts_with("StaleWriter：") && current.contains("recover"),
            "{current}"
        );

        {
            let writer = open_db(&f.db).unwrap();
            meta_set(&writer, "rule_revision", "s2-axis-quantifiers+new-bi/1").unwrap();
        }
        let files = |db: &Path| {
            ["", "-journal", "-wal"]
                .iter()
                // 只读连接可留下空 -wal（见 open_historical_readonly 注释），空与缺失同视。
                .map(|x| {
                    std::fs::read(format!("{}{x}", db.display()))
                        .ok()
                        .filter(|b| !b.is_empty())
                })
                .collect::<Vec<_>>()
        };
        let before = files(&f.db);

        let msg = serve_fixture(
            &f.db,
            &socket,
            &f.profile,
            "1",
            &clock_path,
            bounds(),
            crate::v3::admission_anchor::test_prepare(&f.db),
        )
        .unwrap_err();
        assert!(msg.starts_with("VersionMismatch："), "{msg}");
        assert!(!msg.contains("请监督器先正式recover"), "{msg}");
        for path in [
            "`catalog`",
            "`snapshot`",
            "`meta --db",
            "v2 `init`",
            "--clock-plan --delivery-retain-generations",
            "#1368",
        ] {
            assert!(msg.contains(path), "缺可走路径 {path}：{msg}");
        }
        assert!(!socket.exists(), "拒绝须发生在绑定 socket 之前");

        // 文案说 recover 不适用：实走确认必拒。
        assert!(recover_core(&f.db, "2", Some((&f.clock, "recover-1")))
            .unwrap_err()
            .starts_with("VersionMismatch："));
        // 路径①：CLI 只读查看实走可用。
        cmd_catalog(&f.db, None).unwrap();
        cmd_snapshot(&f.db, None).unwrap();
        cmd_meta(&f.db).unwrap();
        // 闭门未放松：未决 Begin 仍在，主库与 -wal 字节不变。
        let ro = open_db(&f.db).unwrap();
        assert!(meta_get_opt(&ro, "advance_state")
            .unwrap()
            .is_some_and(|s| s.starts_with("begun:")));
        drop(ro);
        assert_eq!(files(&f.db), before, "历史规则库字节须不变");

        // 路径②：人工以当前规则新建代际，新库满足 serve 的规则与闭门前件（未实起 serve，避免绑定阻塞）。
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let fresh = f.dir.join("s-new-generation.sqlite");
        init_v2(
            &fresh,
            "test-c",
            &base.join("catalog/signed-catalog.json"),
            &f.profile,
            "2",
            &clock_path,
            8,
        )
        .unwrap();
        let conn = open_db(&fresh).unwrap();
        assert_eq!(
            meta_get_opt(&conn, "rule_revision").unwrap().as_deref(),
            Some(RULE_REVISION)
        );
        assert_eq!(
            meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
    }

    /// #1653（#1576 独审 INFO-3）：`rule_revision` 键缺失时文案明示缺失，不显示空值；拒绝不放松。
    #[test]
    fn v2_serve_stale_begin_with_missing_rule_revision_says_missing() {
        let f = Fixture::new(8, 2);
        f.begin_only();
        {
            let writer = open_db(&f.db).unwrap();
            writer
                .execute("DELETE FROM meta WHERE key='rule_revision'", [])
                .unwrap();
        }
        let socket = f.dir.join("s.sock");
        let bounds = Bounds {
            frame: 65536,
            read: Duration::from_millis(1000),
            write: Duration::from_millis(1000),
            response: Duration::from_millis(1000),
            cache_bytes: 0,
            queue: 4,
            connections: 4,
        };
        let msg = serve_fixture(
            &f.db,
            &socket,
            &f.profile,
            "1",
            &f.dir.join("clock.json"),
            bounds,
            crate::v3::admission_anchor::test_prepare(&f.db),
        )
        .unwrap_err();
        assert!(msg.starts_with("VersionMismatch："), "{msg}");
        assert!(msg.contains("meta 缺 rule_revision 键"), "{msg}");
        assert!(!msg.contains("rule_revision=``"), "{msg}");
        assert!(!socket.exists(), "拒绝须发生在绑定 socket 之前");
    }

    #[test]
    fn v2_serve_stale_prefix_preserves_profile_order_and_adapter_gate() {
        // #1527×#1602：当前规则旧库只缺新增列时，写前前缀照样给出原首错，且不迁移源库。
        for (bad_profile, pre_policy_records) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let f = Fixture::new(8, 2);
            let profile = if bad_profile {
                f.begin_only();
                let conn = open_db(&f.db).unwrap();
                meta_set(&conn, "rule_revision", "s2-axis-quantifiers+new-bi/1").unwrap();
                drop(conn);
                let mut value: Value =
                    serde_json::from_slice(&std::fs::read(&f.profile).unwrap()).unwrap();
                value["tick_size"] = json!("2");
                let path = f.dir.join("different-profile.json");
                std::fs::write(&path, canonical_json(&value)).unwrap();
                path
            } else {
                f.profile.clone()
            };
            if pre_policy_records {
                Connection::open(&f.db)
                    .unwrap()
                    .execute_batch("ALTER TABLE structure_deltas DROP COLUMN policy_records")
                    .unwrap();
            }
            let before = std::fs::read(&f.db).unwrap();
            let socket = f.dir.join("prefix-must-not-bind.sock");
            let msg = serve_fixture(
                &f.db,
                &socket,
                &profile,
                "1",
                &f.dir.join("clock.json"),
                Bounds {
                    frame: 65536,
                    read: Duration::from_millis(1000),
                    write: Duration::from_millis(1000),
                    response: Duration::from_millis(1000),
                    cache_bytes: 0,
                    queue: 4,
                    connections: 4,
                },
                crate::v3::admission_anchor::test_prepare(&f.db),
            )
            .unwrap_err();
            assert!(
                msg.starts_with(if bad_profile {
                    "IdentityConflict：serve profile绑定不同"
                } else {
                    "LegacyAdmissionBudgetAdapterMissing："
                }),
                "{msg}"
            );
            assert!(!socket.exists());
            assert_eq!(std::fs::read(&f.db).unwrap(), before);
            assert_eq!(
                crate::policy_records_missing(&Connection::open(&f.db).unwrap()).unwrap(),
                pre_policy_records
            );
        }
    }

    #[test]
    fn v2_header_capture_survives_full_handle_eviction() {
        cache::configure(268435456);
        let f = Fixture::new(8, 2);
        for i in 0..4 {
            f.ingest(i);
        }
        let conn = open_db(&f.db).unwrap();
        cache::configure(268435456);
        cache::counters_reset();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::counters_snapshot().batch_header_full_fallbacks, 4);
        assert_eq!(cache::counters_snapshot().batch_full_reads, 4);
        assert_eq!(cache::counters_snapshot().batch_suffix_full_reads, 4);
        cache::evict_full_handles_for_test();
        cache::counters_reset();
        let tx = conn.unchecked_transaction().unwrap();
        let mut reachable = capture_reachable_batches(&tx).unwrap();
        let expected = prepare_control(&tx, None).unwrap();
        assert_eq!(cache::counters_snapshot().batch_header_hits, 4);
        assert_eq!(cache::counters_snapshot().batch_full_reads, 0);
        assert_eq!(
            cache::counters_snapshot().batch_expansions,
            0,
            "完整原件证明尚存时，控制头捕获不得因大句柄驱逐而展开历史batch"
        );
        verify_reachable_root_in_tx(&tx, &reachable, expected).unwrap();
        assert_eq!(cache::last_resumed(), 4);
        assert_eq!(cache::counters_snapshot().batch_suffix_full_reads, 0);
        for field in 0..4 {
            let mut bad = reachable.batches.clone();
            let BatchRef::CertifiedHeader(header) = bad.values_mut().next().unwrap() else {
                panic!("暖缓存应捕获已认证控制头");
            };
            match field {
                0 => header.protocol_revision = None,
                1 => header.session_generation = None,
                2 => header.clock_plan_hash = None,
                _ => {
                    header.semantic_commit_ns =
                        Some(share_header_text_admitted("01020", None).unwrap())
                }
            }
            std::mem::swap(&mut reachable.batches, &mut bad);
            let expected = prepare_control(&tx, None).unwrap();
            assert!(
                verify_reachable_root_in_tx(&tx, &reachable, expected).is_err(),
                "控制头不得暗补或数字归一化"
            );
            std::mem::swap(&mut reachable.batches, &mut bad);
        }
        cache::configure(0);
    }

    #[test]
    fn v2_bounded_cache_reuses_only_fresh_equal_dependencies() {
        cache::configure(268435456);
        let f = Fixture::new(8, 2);
        for i in 0..4 {
            f.ingest(i);
        }
        let conn = open_db(&f.db).unwrap();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 4);
        f.ingest(4);
        assert_eq!(cache::last_resumed(), 4); // 本次只核新增第五代
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 5);
        // 新消息对旧业务原件合法重放：消息覆盖为集合，不能强制一一对应。
        let old = f.message(0);
        let mut payload = old["payload"].clone();
        payload["clock_event_id"] = json!("op-5");
        let replay = f.wrap(payload, "business-replay");
        assert_eq!(
            handle_fixture(&f.db, &f.profile, "1", &f.clock, &replay).unwrap()["kind"],
            "Committed"
        );
        verify_reachable_root(&conn).unwrap();
        // 缓存极小须回完整门，既不拒合法库也不留下前缀健康标签。
        cache::configure(1);
        cache::counters_reset();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 0);
        assert_eq!(cache::counters_snapshot().batch_header_hits, 0);
        assert!(cache::counters_snapshot().batch_header_full_fallbacks >= 5);
        assert!(cache::counters_snapshot().batch_full_reads >= 5);
        assert!(cache::counters_snapshot().batch_suffix_full_reads >= 5);
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 0);
        cache::configure(0);
    }
    #[test]
    fn v2_warm_cache_cross_connection_damage_stops_all_writes() {
        for sql in [
            "UPDATE raw_events SET price='+1' WHERE seq=0",
            "UPDATE raw_events SET source_coord='-1' WHERE seq=0",
            "UPDATE raw_events SET identity_key='foreign' WHERE seq=0",
            "UPDATE objects SET first_known_generation=5",
            "UPDATE objects SET first_known_generation=1",
            "UPDATE objects SET published_generation=5",
            "UPDATE objects SET withdrawn_generation=4,withdrawal_reason='unbacked'",
            "UPDATE witnesses SET published_generation=1",
            "UPDATE relations SET published_generation=1",
            "UPDATE observations SET published_generation=4",
            "UPDATE observations SET detail_json='{}'",
            "DELETE FROM witnesses",
            "DELETE FROM relations",
            "DELETE FROM observations",
            "UPDATE structure_deltas SET session_id='foreign' WHERE generation=1",
            "UPDATE structure_deltas SET delta_json='{}' WHERE generation=1",
            "UPDATE structure_deltas SET catalog_evidence_json='{}' WHERE generation=1",
            "UPDATE batches SET canonical_bytes=x'7b7d',byte_len=2",
            "UPDATE batches SET byte_len=byte_len+1",
            "UPDATE s_evidence_nodes SET payload=CAST(payload||x'00' AS BLOB) WHERE node_id=(SELECT node_id FROM s_evidence_roots WHERE root_key LIKE 'batch/%' LIMIT 1)",
            "UPDATE s_evidence_roots SET slot_ids=zeroblob(32) WHERE root_key LIKE 'batch/%'",
            "UPDATE s_evidence_roots SET content_sha256=lower(hex(zeroblob(32))) WHERE root_key LIKE 'batch/%'",
            "DELETE FROM meta WHERE key='profile_definition'",
            "UPDATE meta SET value='foreign' WHERE key='catalog_hash'",
            "UPDATE meta SET value='-1' WHERE key='writer_epoch'",
            "UPDATE s_input_messages SET canonical_envelope='{}'",
            "DELETE FROM s_clock_events WHERE phase='accept'",
            "UPDATE s_protocol_meta SET logical_phase_frontier='0'",
            "DELETE FROM s_delivery_refs",
            "UPDATE s_delivery_policy SET first_available_generation=3",
            "CREATE TABLE foreign_schema(value TEXT)",
            // #1394：私有存储格式登记损坏（未知值 / 缺行）必须与其它根损坏同门拒绝。
            "UPDATE s_storage_format SET storage_format='changes-only/9'",
            "UPDATE s_storage_format SET storage_format='legacy-full/0'",
            "DELETE FROM s_storage_format",
            "INSERT INTO writer_epoch_history VALUES(1,'bad','2',4,'idle','bad')",
        ] {
            cache::configure(268435456);
            let f = Fixture::new(8, 2);
            for i in 0..4 {
                f.ingest(i);
            }
            let healthy = open_db(&f.db).unwrap();
            verify_reachable_root(&healthy).unwrap();
            assert_eq!(cache::last_resumed(), 4);
            cache::evict_full_handles_for_test();
            let other = open_db(&f.db).unwrap();
            other.execute_batch(sql).unwrap();
            drop(other);
            let before = f.dump();
            assert!(verify_reachable_root(&healthy).is_err(), "{sql}");
            let c = InputContext {
                envelope: f.message(4),
                clock: f.clock.clone(),
            };
            for refused in [
                accept_core(
                    &f.db,
                    &canonical_json(&c.envelope["payload"]["raw_input"]),
                    &f.profile,
                    "1",
                    Some(&c),
                )
                .is_err(),
                advance_core_fixture(&f.db, "1", Some(&c)).is_err(),
                recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).is_err(),
            ] {
                assert!(refused, "{sql}");
            }
            assert_eq!(before, f.dump(), "{sql}");
        }
        cache::configure(0);
    }
    #[test]
    fn v2_two_fresh_directories_have_identical_authoritative_rows() {
        let a = Fixture::new(8, 2);
        let b = Fixture::new(8, 2);
        for i in 0..4 {
            assert_eq!(a.ingest(i), b.ingest(i));
            assert_eq!(a.dump(), b.dump());
        }
    }
    #[test]
    fn v2_retention_removes_only_delivery_refs_and_replays_original_result() {
        let f = Fixture::new(2, 2);
        let first = f.ingest(0);
        for i in 1..5 {
            f.ingest(i);
        }
        let conn = open_db(&f.db).unwrap();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM structure_deltas", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            conn.query_row("SELECT MIN(generation) FROM s_delivery_refs", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            4
        );
        let before = f.dump();
        assert_eq!(f.ingest(0), first);
        assert_eq!(f.dump(), before);
        assert_eq!(
            read_snapshot_in_tx(&conn, Some(1)).unwrap()["generation"],
            "1"
        );
        let expired = retained_watch(&conn, 0).unwrap().unwrap();
        assert_eq!(expired["gap"]["reason"], "retention_expired");
        assert!(expired["deltas"].as_array().unwrap().is_empty());
        let valid = retained_watch(&conn, 3).unwrap().unwrap();
        assert!(valid["gap"].is_null());
        assert_eq!(valid["deltas"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn v2_external_receipts_keep_historical_policy_records_across_config_versions() {
        let f = Fixture::new(8, 2);
        let v1 = ThetaConfig::default();
        let mut v2 = v1.clone();
        v2.records.risk.record.version =
            newchan_rust::theta_v0::policy_records::PolicyVersion::Signed(2);
        assert!(!v1.admissible());

        let first_input = envelope(f.message(0)).unwrap();
        let first = handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_input, &v1)
            .unwrap();
        assert_eq!(first["kind"], "Committed");
        assert!(first["policy_records"]
            .as_str()
            .unwrap()
            .contains("theta-risk/1"));
        assert!(first["policy_records"]
            .as_str()
            .unwrap()
            .contains("venue-profile-diagnostic-fixture/1"));

        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_input, &v2,)
                .unwrap(),
            first,
            "配置改版后的幂等重投必须返回原生成记录戳"
        );
        let first_query = envelope(f.wrap(
            json!({"op":"receipt","original_source_namespace":first_input["source_namespace"],"original_source_epoch":first_input["source_epoch"],"original_message_id":first_input["message_id"],"original_payload_hash":first_input["payload_hash"]}),
            "receipt-query-1",
        ))
        .unwrap();
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_query, &v2,)
                .unwrap(),
            first,
            "receipt 查询不得用当前配置改写历史"
        );

        let context = InputContext {
            envelope: first_input.clone(),
            clock: f.clock.clone(),
        };
        let repeated_advance = advance_core_with_config(&f.db, "1", Some(&context), &v2).unwrap();
        assert_eq!(repeated_advance["idempotent"], true);
        assert_eq!(
            repeated_advance["policy_records"], first["policy_records"],
            "无新输入的 advance 也必须读取原 generation，而不是当前配置"
        );

        let second_input = envelope(f.message(1)).unwrap();
        let second =
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &second_input, &v2)
                .unwrap();
        assert!(second["policy_records"]
            .as_str()
            .unwrap()
            .contains("theta-risk/2"));
        assert_ne!(second["policy_records"], first["policy_records"]);

        let second_query = envelope(f.wrap(
            json!({"op":"receipt","original_source_namespace":second_input["source_namespace"],"original_source_epoch":second_input["source_epoch"],"original_message_id":second_input["message_id"],"original_payload_hash":second_input["payload_hash"]}),
            "receipt-query-2",
        ))
        .unwrap();
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &second_query, &v1,)
                .unwrap(),
            second,
            "重新打开后即使配置回退，也只能读取结果实际使用的 v2 记录戳"
        );
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_query, &v1,)
                .unwrap(),
            first
        );
    }

    #[test]
    fn v2_legacy_policy_records_migration_preserves_history_and_new_versions() {
        let f = Fixture::new(8, 2);
        let v1 = ThetaConfig::default();
        let mut v2 = v1.clone();
        v2.records.risk.record.version =
            newchan_rust::theta_v0::policy_records::PolicyVersion::Signed(2);

        let first_input = envelope(f.message(0)).unwrap();
        let first = handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_input, &v1)
            .unwrap();
        assert_eq!(first["publication"]["generation"], "1");
        let original = |conn: &Connection| {
            conn.query_row(
                "SELECT m.canonical_envelope,m.receipt_id,d.delta_json,d.catalog_evidence_json,d.index_frontier
                 FROM s_input_messages m JOIN structure_deltas d ON d.generation=m.first_published_generation
                 WHERE m.message_id='msg-0'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .unwrap()
        };
        let before = original(&Connection::open(&f.db).unwrap());

        // 真实旧协议表：已提交 generation 留在库内，只缺本票新增的物理列。
        {
            let conn = Connection::open(&f.db).unwrap();
            conn.execute_batch("ALTER TABLE structure_deltas DROP COLUMN policy_records")
                .unwrap();
            assert!(conn
                .prepare("SELECT policy_records FROM structure_deltas")
                .is_err());
            assert_eq!(original(&conn), before);
        }

        let mut legacy_first = first.clone();
        legacy_first["policy_records"] = json!(crate::LEGACY_POLICY_RECORDS_UNKNOWN);
        let first_query = envelope(f.wrap(
            json!({"op":"receipt","original_source_namespace":first_input["source_namespace"],"original_source_epoch":first_input["source_epoch"],"original_message_id":first_input["message_id"],"original_payload_hash":first_input["payload_hash"]}),
            "legacy-receipt-query-1",
        ))
        .unwrap();
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_query, &v2)
                .unwrap(),
            legacy_first,
            "旧 receipt 查询须迁移并返回历史未知，不得冒充当前 v2 配置"
        );
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "1", &f.clock, &first_input, &v2)
                .unwrap(),
            legacy_first,
            "迁移后的幂等重投仍返回原消息与原 generation"
        );

        let context = InputContext {
            envelope: first_input.clone(),
            clock: f.clock.clone(),
        };
        let repeated = advance_core_with_config(&f.db, "1", Some(&context), &v2).unwrap();
        assert_eq!(repeated["idempotent"], true);
        assert_eq!(repeated["generation"], "1");
        assert_eq!(
            repeated["policy_records"],
            json!(crate::LEGACY_POLICY_RECORDS_UNKNOWN)
        );

        recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).unwrap();
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "2", &f.clock, &first_query, &v2)
                .unwrap(),
            legacy_first,
            "恢复并重开后，旧 generation 的记录戳仍是历史未知"
        );

        let mut second_message = f.message(1);
        second_message["payload"]["writer_epoch"] = json!("2");
        second_message["payload_hash"] = json!(sha256_hex(
            canonical_json(&second_message["payload"]).as_bytes()
        ));
        let second_input = envelope(second_message).unwrap();
        let second =
            handle_fixture_with_config(&f.db, &f.profile, "2", &f.clock, &second_input, &v2)
                .unwrap();
        assert_eq!(second["publication"]["generation"], "2");
        assert_eq!(second["policy_records"], json!(v2.record_labels()));
        assert!(second["policy_records"]
            .as_str()
            .unwrap()
            .contains("theta-risk/2"));
        assert_eq!(
            handle_fixture_with_config(&f.db, &f.profile, "2", &f.clock, &first_query, &v1)
                .unwrap(),
            legacy_first
        );

        let conn = open_db(&f.db).unwrap();
        verify_reachable_root(&conn).unwrap();
        assert_eq!(original(&conn), before, "迁移和后续发布不得改写旧原件");
        let mut stmt = conn
            .prepare("SELECT policy_records FROM structure_deltas ORDER BY generation")
            .unwrap();
        let stamps = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            stamps,
            vec![
                crate::LEGACY_POLICY_RECORDS_UNKNOWN.to_string(),
                v2.record_labels(),
            ]
        );
        drop(stmt);
        drop(conn);

        let output = std::process::Command::new("python3")
            .arg(crate::tests::fixture("tests/tb02b_verify.py"))
            .arg("--database")
            .arg(&f.db)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "迁移后的 Python 只读严格审计失败：stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// #1527×#1602 B 臂：当前规则 s-session/2 库只缺新增列时，mixed 写前准入只读审到迁移前精确形状；
    /// 真实 recover 入口在 scope 内迁移后照跑严格审计，结果与原调用（无 scope 时首个开门者迁移）相同。
    /// serve 入口的封存前缀拒绝与形状无关。
    #[test]
    fn v2_mixed_admission_migrates_current_rule_pre_policy_records_store_in_scope() {
        let f = Fixture::new(4, 1);
        f.ingest(0);
        Connection::open(&f.db)
            .unwrap()
            .execute_batch("ALTER TABLE structure_deltas DROP COLUMN policy_records")
            .unwrap();
        assert!(crate::policy_records_missing(&Connection::open(&f.db).unwrap()).unwrap());
        let original = f.dir.join("original-call.sqlite");
        std::fs::copy(&f.db, &original).unwrap();
        let bytes = || {
            let wal = std::fs::read(format!("{}-wal", f.db.display()))
                .ok()
                .filter(|b| !b.is_empty());
            (std::fs::read(&f.db).unwrap(), wal)
        };
        let before = bytes();

        let command = crate::v3::admission_anchor::CommandAccounting::begin();
        let prepared = crate::v3::admission_anchor::test_prepare_mixed(&f.db, 1 << 26, 1 << 26);
        let error = crate::v3::test_with_mixed_entry(|| {
            crate::v3::admission::WriterScope::admit_legacy_serve(
                &f.db, prepared, "1", &f.clock, &f.profile,
            )
        })
        .err()
        .unwrap();
        let mut output = Vec::new();
        command.report(&error, &mut output);
        let output: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(output["code"], "MixedProtocolMismatch");
        assert_eq!(bytes(), before);

        let _command = crate::v3::admission_anchor::CommandAccounting::begin();
        let prepared = crate::v3::admission_anchor::test_prepare_mixed(&f.db, 1 << 26, 1 << 26);
        let scope = crate::v3::test_with_mixed_entry(|| {
            crate::v3::admission::WriterScope::admit(&f.db, prepared)
        })
        .unwrap();
        assert_eq!(bytes(), before, "准入只读，不迁移");
        let out = {
            let _guard = scope.enter().unwrap();
            scope.require_admitted().unwrap();
            recover_core(scope.db_path(), "2", Some((&f.clock, "recover-1"))).unwrap()
        };
        scope.finish_startup().unwrap();
        drop(scope);
        let conn = open_db(&f.db).unwrap();
        assert!(!crate::policy_records_missing(&conn).unwrap());
        verify_reachable_root(&conn).unwrap();
        drop(conn);

        let expected = recover_core(&original, "2", Some((&f.clock, "recover-1"))).unwrap();
        assert_eq!(out, expected);
        assert_eq!(dump_db(&f.db), dump_db(&original));
    }

    #[test]
    fn v2_same_message_cannot_change_clock_or_payload() {
        let f = Fixture::new(8, 2);
        f.ingest(0);
        let before = f.dump();
        let mut e = f.message(0);
        e["payload"]["clock_event_id"] = json!("op-1");
        e["payload_hash"] = json!(sha256_hex(canonical_json(&e["payload"]).as_bytes()));
        assert!(handle_fixture(&f.db, &f.profile, "1", &f.clock, &e)
            .unwrap_err()
            .contains("IdentityConflict"));
        assert_eq!(before, f.dump());
    }
    #[test]
    fn v2_pending_recovery_preserves_identity_and_uses_next_attempt() {
        let f = Fixture::new(8, 2);
        let c = f.begin_only();
        recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).unwrap();
        advance_core_fixture(&f.db, "2", Some(&c)).unwrap();
        let conn = open_db(&f.db).unwrap();
        verify_reachable_root(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM s_clock_events WHERE phase='begin'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        let original: String = conn
            .query_row("SELECT canonical_envelope FROM s_input_messages", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(original, canonical_json(&f.message(0)));
        let result = lookup(
            &conn,
            "test.feed",
            "1",
            "msg-0",
            required(&c.envelope, "payload_hash").unwrap(),
        )
        .unwrap();
        assert_eq!(result["kind"], "Committed");
    }
    #[test]
    fn v2_exhausted_attempt_plan_has_zero_new_begin_or_write() {
        let f = Fixture::new(8, 1);
        let c = f.begin_only();
        recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).unwrap();
        let before = f.dump();
        assert!(advance_core_fixture(&f.db, "2", Some(&c))
            .unwrap_err()
            .contains("MissingDependency"));
        assert_eq!(before, f.dump());
    }
    #[test]
    fn v2_fresh_audit_rejects_birth_drift_in_every_cumulative_family() {
        // 所有行仍在 1..G；错误不能只靠未来行门发现，也不能由后代重复发布掩盖。
        for table in ["witnesses", "relations", "observations"] {
            let f = Fixture::new(8, 2);
            for i in 0..4 {
                f.ingest(i);
            }
            let conn = open_db(&f.db).unwrap();
            let sql=format!("UPDATE {table} SET published_generation=CASE WHEN published_generation=1 THEN 2 ELSE 1 END");
            assert!(conn.execute(&sql, []).unwrap() > 0);
            let before = f.dump();
            assert!(verify_reachable_root(&conn).is_err(), "{table}");
            assert!(
                handle_fixture(&f.db, &f.profile, "1", &f.clock, &f.message(4)).is_err(),
                "{table}"
            );
            assert!(
                recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).is_err(),
                "{table}"
            );
            assert_eq!(before, f.dump(), "{table}");
        }
    }
    #[test]
    fn v2_same_generation_pending_damage_is_not_hidden_by_previous_healthy_audit() {
        let f = Fixture::new(8, 2);
        f.ingest(0);
        let c = InputContext {
            envelope: f.message(1),
            clock: f.clock.clone(),
        };
        accept_core(
            &f.db,
            &canonical_json(&c.envelope["payload"]["raw_input"]),
            &f.profile,
            "1",
            Some(&c),
        )
        .unwrap();
        let conn = open_db(&f.db).unwrap();
        verify_reachable_root(&conn).unwrap();
        conn.execute("UPDATE raw_events SET source_coord='-1' WHERE seq=1", [])
            .unwrap();
        let before = f.dump();
        assert!(verify_reachable_root(&conn).is_err());
        assert!(advance_core_fixture(&f.db, "1", Some(&c)).is_err());
        assert_eq!(before, f.dump());
    }
    #[test]
    fn v2_readonly_root_rejects_corrupt_persisted_epoch() {
        let f = Fixture::new(8, 2);
        let conn = open_db(&f.db).unwrap();
        for invalid in ["0", "-1", "+1", "01", "x", "9223372036854775808"] {
            meta_set(&conn, "writer_epoch", invalid).unwrap();
            assert!(verify_reachable_root(&conn).is_err(), "{invalid}");
        }
        meta_set(&conn, "writer_epoch", "1").unwrap();
        verify_reachable_root(&conn).unwrap();
    }
    #[test]
    fn v2_all_control_rows_and_pending_message_coverage_gate_every_write() {
        let mut failures = Vec::new();
        for (name,sql,pending) in [
            ("extra_protocol_singleton", "PRAGMA ignore_check_constraints=ON; INSERT INTO s_protocol_meta SELECT 2,protocol_revision,'broken','bad','not-json',0,'garbage' FROM s_protocol_meta WHERE singleton=1;",false),
            ("extra_delivery_singleton", "PRAGMA ignore_check_constraints=ON; INSERT INTO s_delivery_policy SELECT 2,'foreign','bad','bad',-1,-1,-1 FROM s_delivery_policy WHERE singleton=1;",false),
            ("malformed_epoch_history", "INSERT INTO writer_epoch_history VALUES(1,'bad','not-an-epoch','future','bad-state','not-ns');",false),
            ("missing_profile_definition", "DELETE FROM meta WHERE key='profile_definition';",false),
            ("pending_without_message", "DELETE FROM s_input_messages; DELETE FROM s_clock_events; UPDATE s_protocol_meta SET logical_phase_frontier='-1';",true),
        ] {
            let f=Fixture::new(8,2);
            let c=InputContext{envelope:f.message(0),clock:f.clock.clone()};
            if pending { accept_core(&f.db,&canonical_json(&c.envelope["payload"]["raw_input"]),&f.profile,"1",Some(&c)).unwrap(); }
            let conn=open_db(&f.db).unwrap();conn.execute_batch(sql).unwrap();
            let before=f.dump();
            let refused=[verify_reachable_root(&conn).is_err(),
                accept_core(&f.db,&canonical_json(&c.envelope["payload"]["raw_input"]),&f.profile,"1",Some(&c)).is_err(),
                advance_core_fixture(&f.db,"1",Some(&c)).is_err(),
                recover_core(&f.db,"2",Some((&f.clock,"recover-1"))).is_err()];
            if refused!=[true;4] || before!=f.dump() { failures.push((name,refused)); }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }
    #[test]
    fn v2_epoch_history_and_recover_phase_are_one_complete_chain() {
        let f = Fixture::new(8, 2);
        let c = f.begin_only();
        recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).unwrap();
        advance_core_fixture(&f.db, "2", Some(&c)).unwrap();
        let mut conn = open_db(&f.db).unwrap();
        verify_reachable_root(&conn).unwrap();
        for sql in [
            "UPDATE writer_epoch_history SET ordinal=2",
            "UPDATE writer_epoch_history SET from_epoch='+1'",
            "UPDATE writer_epoch_history SET to_epoch='1'",
            "UPDATE meta SET value='3' WHERE key='writer_epoch'",
            "UPDATE writer_epoch_history SET generation_at_transition='2'",
            "UPDATE writer_epoch_history SET transitioned_at='1031'",
            "UPDATE writer_epoch_history SET advance_state_at_transition='garbage'",
            "DELETE FROM writer_epoch_history",
            "DELETE FROM s_clock_events WHERE phase='recover'",
            "UPDATE s_clock_events SET generation=1 WHERE phase='recover'",
            "UPDATE s_input_messages SET attempt_count=9223372036854775807",
            "INSERT INTO meta VALUES(NULL,'hidden bad primary key')",
        ] {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            assert!(verify_reachable_root(&tx).is_err(), "{sql}");
            tx.rollback().unwrap();
        }
        verify_reachable_root(&conn).unwrap();
    }
    #[test]
    fn v2_corrupt_control_or_pending_rows_block_all_writes() {
        for sql in [
            "UPDATE s_protocol_meta SET next_attempt_ordinal=99",
            "UPDATE s_clock_events SET semantic_ns='999' WHERE phase='accept'",
            "UPDATE s_delivery_policy SET head_generation=1",
            "UPDATE raw_events SET source_coord='-1'",
            "UPDATE raw_events SET price='99999'",
            "DROP TABLE s_delivery_refs",
        ] {
            let f = Fixture::new(8, 2);
            let c = InputContext {
                envelope: f.message(0),
                clock: f.clock.clone(),
            };
            accept_core(
                &f.db,
                &canonical_json(&c.envelope["payload"]["raw_input"]),
                &f.profile,
                "1",
                Some(&c),
            )
            .unwrap();
            open_db(&f.db).unwrap().execute_batch(sql).unwrap();
            let before = f.dump();
            let conn = open_db(&f.db).unwrap();
            assert!(verify_reachable_root(&conn).is_err(), "{sql}");
            assert!(
                accept_core(
                    &f.db,
                    &canonical_json(&c.envelope["payload"]["raw_input"]),
                    &f.profile,
                    "1",
                    Some(&c)
                )
                .is_err(),
                "{sql}"
            );
            assert!(advance_core_fixture(&f.db, "1", Some(&c)).is_err(), "{sql}");
            assert!(
                recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).is_err(),
                "{sql}"
            );
            assert_eq!(meta_i64(&conn, "generation").unwrap(), 0);
            assert_eq!(before, f.dump(), "{sql}");
            assert_eq!(
                meta_get_opt(&conn, "advance_state").unwrap().as_deref(),
                Some("idle")
            );
        }
    }
    #[test]
    fn v2_strict_json_rejects_nested_duplicate_keys() {
        assert!(strict_json(br#"{"payload":{"a":1,"a":2}}"#).is_err());
    }
    #[test]
    fn v2_expired_response_deadline_cannot_admit_a_job() {
        let make_job = |name| {
            let (result, _) = mpsc::sync_channel(1);
            Job {
                envelope: json!({"name": name}),
                result,
            }
        };
        let (tx, rx) = mpsc::sync_channel(1);
        assert!(!enqueue_before_deadline(
            &tx,
            make_job("expired"),
            Instant::now()
        ));
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(enqueue_before_deadline(
            &tx,
            make_job("first"),
            Instant::now() + Duration::from_secs(1)
        ));
        assert!(!enqueue_before_deadline(
            &tx,
            make_job("full"),
            Instant::now() + Duration::from_millis(2)
        ));
        assert_eq!(rx.try_recv().unwrap().envelope, json!({"name":"first"}));
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        drop(rx);
        assert!(!enqueue_before_deadline(
            &tx,
            make_job("disconnected"),
            Instant::now() + Duration::from_secs(1)
        ));
    }
    #[test]
    fn v2_response_wait_has_its_own_bounded_deadline() {
        let f = Fixture::new(8, 2);
        for (response_ms, expected) in [(2000, true), (20, false)] {
            let (server, mut client) = UnixStream::pair().unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let (tx, rx) = mpsc::sync_channel::<Job>(1);
            let count = Arc::new(AtomicUsize::new(1));
            let remaining = count.clone();
            let io = std::thread::spawn(move || {
                io_connection(
                    server,
                    tx,
                    Bounds {
                        frame: 1048576,
                        read: Duration::from_millis(200),
                        write: Duration::from_millis(100),
                        response: Duration::from_millis(response_ms),
                        cache_bytes: 0,
                        queue: 1,
                        connections: 1,
                    }
                    .transport(),
                    remaining,
                    envelope,
                )
            });
            client
                .write_all(format!("{}\n", canonical_json(&f.message(0))).as_bytes())
                .unwrap();
            client.shutdown(std::net::Shutdown::Write).unwrap();
            let job = rx.recv_timeout(Duration::from_secs(1)).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            let _ = job.result.try_send(json!({"ok":true}));
            let mut body = String::new();
            client.read_to_string(&mut body).unwrap();
            assert_eq!(!body.is_empty(), expected);
            io.join().unwrap();
            assert_eq!(count.load(Ordering::SeqCst), 0);
        }
    }
    #[test]
    fn v2_frame_requires_eof_and_rejects_delayed_second_frame() {
        let f = Fixture::new(8, 2);
        let body = format!("{}\n", canonical_json(&f.message(0)));
        let (mut a, mut b) = UnixStream::pair().unwrap();
        let sender = std::thread::spawn(move || {
            b.write_all(body.as_bytes()).unwrap();
            std::thread::sleep(Duration::from_millis(5));
            b.write_all(b"{}\n").unwrap();
            b.shutdown(std::net::Shutdown::Write).unwrap();
        });
        assert!(read_frame(
            &mut a,
            1024 * 1024,
            Instant::now() + Duration::from_secs(1),
            envelope
        )
        .is_err());
        sender.join().unwrap();
    }
    #[test]
    fn v2_process_lock_prevents_second_writer() {
        let f = Fixture::new(8, 2);
        let _held = process_lock(&f.db).unwrap();
        assert!(process_lock(&f.db).unwrap_err().contains("StaleWriter"));
        let alias = f.dir.join("alias.sqlite");
        std::os::unix::fs::symlink(&f.db, &alias).unwrap();
        assert!(process_lock(&alias).unwrap_err().contains("StaleWriter"));
    }
    #[test]
    fn v2_ready_nonce_is_only_transport_diagnostics() {
        let f = Fixture::new(8, 2);
        assert_eq!(
            ready(&open_db(&f.db).unwrap()).unwrap()["schema_revision"],
            "s-session/2"
        );
        assert!(ready_for_protocol(&open_db(&f.db).unwrap(), "s-session/3").is_err());
        assert!(ready_for_protocol(&open_db(&f.db).unwrap(), "s-session/4").is_err());
        let e = f.wrap(json!({"op":"ready"}), "ready-1");
        let identity = ("test-c".to_string(), "1".to_string(), "1".to_string());
        let a = response(
            &e,
            Ok(json!({"ok":true,"kind":"Ready"})),
            &identity,
            Some("first"),
        );
        let b = response(
            &e,
            Ok(json!({"ok":true,"kind":"Ready"})),
            &identity,
            Some("second"),
        );
        assert_eq!(a["payload"], b["payload"]);
        assert_eq!(a["payload_hash"], b["payload_hash"]);
        assert_eq!(a["message_id"], b["message_id"]);
        assert_ne!(a["control_instance_id"], b["control_instance_id"]);
        assert!(response(
            &e,
            Err("StorageUnavailable".into()),
            &identity,
            Some("first")
        )
        .get("control_instance_id")
        .is_none());
    }

    /// #1640 C4：v2 Ready 与 v3 同规则自报构建身份；在所有构建都编译，按 cfg 分支断言。
    #[test]
    fn v2_ready_reply_reports_build_profile_in_hashed_payload() {
        #[cfg(feature = "s_incremental")]
        let declared = Some(crate::v3::BUILD_PROFILE);
        #[cfg(not(feature = "s_incremental"))]
        let declared: Option<&str> = None;
        let expected = if !cfg!(feature = "s_incremental") {
            None
        } else if cfg!(feature = "s_capture_native_testonly") {
            Some("testonly")
        } else {
            Some("production")
        };
        assert_eq!(declared, expected);
        let request_payload = json!({"op":"ready"});
        let e = json!({"schema_revision":"s-session/2","session_id":"test-c","session_generation":"1","source_namespace":"test.feed","source_epoch":"1","message_id":"ready-1","producer_id":"test-driver","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&request_payload).as_bytes()),"causal_refs":[],"payload":request_payload});
        let identity = ("test-c".to_string(), "1".to_string(), "1".to_string());
        let ready = response(
            &e,
            Ok(json!({"ok":true,"kind":"Ready"})),
            &identity,
            Some("first"),
        );
        let fields = ready["payload"].as_object().unwrap();
        match expected {
            Some(profile) => {
                assert_eq!(fields["build_profile"], profile);
                assert_eq!(fields.len(), 3);
                let mut unprofiled = ready["payload"].clone();
                unprofiled.as_object_mut().unwrap().remove("build_profile");
                assert_ne!(
                    ready["payload_hash"],
                    sha256_hex(canonical_json(&unprofiled).as_bytes())
                );
            }
            None => {
                assert!(fields.get("build_profile").is_none());
                assert_eq!(fields.len(), 2);
            }
        }
        assert_eq!(
            ready["payload_hash"],
            sha256_hex(canonical_json(&ready["payload"]).as_bytes())
        );
        assert_eq!(ready["control_instance_id"], "first");
        assert!(ready.get("build_profile").is_none());
        let receipt = response(
            &e,
            Ok(json!({"ok":true,"kind":"Receipt"})),
            &identity,
            Some("first"),
        );
        assert!(receipt["payload"].get("build_profile").is_none());
        let error = response(&e, Err("Busy".into()), &identity, None);
        assert!(error["payload"].get("build_profile").is_none());
    }
}

#[cfg(test)]
#[test]
#[ignore = "explicit isolated database diagnostic"]
fn testonly_audit_cost_breakdown() {
    fn retained_sample() -> Value {
        let (root, handles, originals) = cache::retained_breakdown();
        json!({
            "retained_breakdown": {"root_walk_bytes":root,"handle_bytes":handles,"original_cert_bytes":originals,"batch_header_bytes":cache::batch_header_bytes()},
            "capture_bytes_now":cache::capture_bytes_now()
        })
    }
    fn counter_delta(after: &Value, before: &Value) -> Value {
        if let Some(fields) = after.as_object() {
            return Value::Object(
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), counter_delta(value, &before[key])))
                    .collect(),
            );
        }
        json!(after
            .as_u64()
            .unwrap()
            .checked_sub(before.as_u64().unwrap())
            .unwrap())
    }
    let path = std::env::var("S_AUDIT_COST_DB").expect("isolated copy path required");
    let expected_text = std::env::var("S_AUDIT_COST_EXPECT_GENERATION")
        .expect("explicit actual persistent generation required; not the requested prefix length");
    let expected_generation =
        parse_canonical_i64(&expected_text, "expected persistent generation").unwrap();
    assert!(expected_generation >= 0);
    assert!(
        Path::new(&path).is_file(),
        "existing stopped database copy required"
    );
    let _lock =
        process_lock(Path::new(&path)).expect("database copy must not have an active S writer");
    let conn = open_db(Path::new(&path)).unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    let generation = meta_i64(&tx, "generation").unwrap();
    assert_eq!(
        generation, expected_generation,
        "persistent generation differs from the explicit declaration"
    );
    cache::configure(268435456);
    for iteration in 0..3 {
        cache::counters_reset();
        let before = retained_sample();
        let t = Instant::now();
        let batches = capture_reachable_batches(&tx).unwrap();
        let b = t.elapsed();
        let captured = retained_sample();
        let capture_counters = serde_json::to_value(cache::counters_snapshot()).unwrap();
        let expected = prepare_control(&tx, None).unwrap();
        let c = t.elapsed();
        let control_counters = serde_json::to_value(cache::counters_snapshot()).unwrap();
        verify_reachable_root_in_tx(&tx, &batches, expected).unwrap();
        let r = t.elapsed();
        let verified = retained_sample();
        let last_resumed = cache::last_resumed();
        let verified_counters = serde_json::to_value(cache::counters_snapshot()).unwrap();
        drop(batches);
        let released = retained_sample();
        assert_eq!(
            cache::capture_bytes_now(),
            0,
            "capture must end after releasing batches"
        );
        assert_eq!(meta_i64(&tx, "generation").unwrap(), expected_generation);
        let total_counters = serde_json::to_value(cache::counters_snapshot()).unwrap();
        eprintln!(
            "AUDIT_COST {}",
            json!({
                "iteration":iteration,"cache_state":if iteration == 0 {"cold"} else {"warm"},
                "generation":generation,"expected_generation":expected_generation,
                "cache_source_byte_limit":268435456,"last_resumed":last_resumed,
                "retention":{"before":before,"captured":captured,"verified":verified,"released":released},
                "counters":{"capture":capture_counters,
                    "control":counter_delta(&control_counters, &capture_counters),
                    "structure":counter_delta(&verified_counters, &control_counters),"total":total_counters},
                "batches_ms":b.as_secs_f64()*1000.0,"control_ms":(c-b).as_secs_f64()*1000.0,
                "structure_ms":(r-c).as_secs_f64()*1000.0,"total_ms":r.as_secs_f64()*1000.0,
                "timing_note":"control_ms 仅含全局控制检查与逐代期望准备，逐批控制头核对计入 structure_ms。沿原累积 Instant 分段；测试采样开销含在墙钟内。内部 audit_ns 与墙钟重叠，不相加。",
                "storage_note":"保留量是源字节账，非 RSS；只验证显式实际代号，不代表目标前缀已全部完成。"
            })
        );
    }
}

#[cfg(feature = "s_incremental")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::retained_struct;

    retained_struct!(ControlFacts {
        session_generation,
        clock_plan_hash,
        headers
    });
    retained_struct!(ControlHeader {
        commit_ns,
        consumed
    });
}

#[cfg(test)]
impl ControlFacts {
    pub(super) fn overlay_ab_control_signature(&self) -> serde_json::Value {
        let mut headers = self
            .headers
            .iter()
            .map(|((generation, batch), h)| {
                (
                    *generation,
                    batch.as_str(),
                    h.commit_ns.as_str(),
                    h.consumed,
                )
            })
            .collect::<Vec<_>>();
        headers.sort();
        serde_json::json!({"session_generation":self.session_generation.as_str(),"clock_plan_hash":self.clock_plan_hash.as_str(),"headers":headers})
    }
}

pub(crate) mod mixed_control;

#[cfg(test)]
pub(crate) fn with_mixed_control_fixture(run: impl FnOnce(&Connection)) {
    tests::with_mixed_control_fixture(run)
}

pub(crate) mod mixed_root;
