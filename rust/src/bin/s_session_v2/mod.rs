//! #1372：本地单写者传输、语义时钟和独立普通投递保留；结构判断仍由父模块唯一核心完成。
use super::*;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

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
    format!("StorageUnavailable：{e}")
}
fn required<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v.get(k)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("SchemaUnsupported：缺非空字符串 {k}"))
}
fn nonnegative(s: &str, field: &str) -> Result<i64, String> {
    let n = parse_canonical_i64(s, field)?;
    if n < 0 {
        return Err(format!("InvalidDomain：{field} 必须非负"));
    }
    Ok(n)
}
fn positive(s: &str, field: &str) -> Result<i64, String> {
    let n = nonnegative(s, field)?;
    if n == 0 {
        return Err(format!("InvalidDomain：{field} 必须大于0"));
    }
    Ok(n)
}

/// serde_json::Value 的普通解析会覆盖重复键；所有新协议/clock输入先经过此严格解码器。
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Strict>()? {
                    v.push(x.0)
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if v.contains_key(&k) {
                        return Err(serde::de::Error::custom(format!("duplicate key {k}")));
                    }
                    v.insert(k, a.next_value::<Strict>()?.0);
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_json(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice::<Strict>(bytes)
        .map(|s| s.0)
        .map_err(|e| format!("SchemaUnsupported：{e}"))
}

#[derive(Clone)]
pub struct ClockPlan {
    value: Value,
    hash: String,
}
impl ClockPlan {
    fn parse(value: Value) -> Result<Self, String> {
        let keys = [
            "schema_revision",
            "clock_plan_id",
            "origin_utc",
            "unit",
            "events",
        ];
        if value.as_object().map(|m| m.len()) != Some(keys.len())
            || value
                .as_object()
                .unwrap()
                .keys()
                .any(|k| !keys.contains(&k.as_str()))
        {
            return Err("SchemaUnsupported：clock计划字段不完整/多余".into());
        }
        if required(&value, "schema_revision")? != "s-clock-plan/1"
            || required(&value, "unit")? != "ns"
        {
            return Err("SchemaUnsupported：需要 s-clock-plan/1 与 ns 单位".into());
        }
        required(&value, "clock_plan_id")?;
        required(&value, "origin_utc")?;
        let events = value["events"]
            .as_object()
            .ok_or("SchemaUnsupported：clock events必须对象")?;
        for (id, e) in events {
            if id.is_empty() {
                return Err("InvalidDomain：空clock_event_id".into());
            }
            if let Some(t) = e.get("recover_ns") {
                nonnegative(
                    t.as_str().ok_or("InvalidDomain：recover_ns非文本")?,
                    "recover_ns",
                )?;
                if e.as_object().map(|m| m.len()) != Some(1) {
                    return Err("InvalidDomain：recover事件字段不唯一".into());
                }
            } else {
                let accept = nonnegative(required(e, "accept_ns")?, "accept_ns")?;
                let a = e["attempts"]
                    .as_array()
                    .filter(|a| !a.is_empty())
                    .ok_or("MissingDependency：clock缺非空attempts")?;
                if e.as_object().map(|m| m.len()) != Some(2) {
                    return Err("InvalidDomain：输入clock事件字段不完整/多余".into());
                }
                let mut prev = accept;
                for phase in a {
                    let b = nonnegative(required(phase, "begin_ns")?, "begin_ns")?;
                    let c = nonnegative(required(phase, "commit_ns")?, "commit_ns")?;
                    if b < prev || c < b || phase.as_object().map(|m| m.len()) != Some(2) {
                        return Err("InvalidDomain：clock attempt时间倒退或字段错误".into());
                    }
                    prev = c;
                }
            }
        }
        let hash = sha256_hex(canonical_json(&value).as_bytes());
        Ok(Self { value, hash })
    }
    fn load(path: &Path) -> Result<Self, String> {
        Self::parse(strict_json(&std::fs::read(path).map_err(err)?)?)
    }
    fn phase(&self, id: &str, phase: &str, attempt: i64) -> Result<String, String> {
        let event = self.value["events"]
            .get(id)
            .ok_or_else(|| format!("MissingDependency：clock事件 {id}"))?;
        let k = format!("{phase}_ns");
        let v = if attempt < 0 {
            event.get(&k)
        } else {
            event["attempts"]
                .as_array()
                .and_then(|a| a.get(attempt as usize))
                .and_then(|a| a.get(&k))
        };
        let t = v
            .and_then(Value::as_str)
            .ok_or_else(|| format!("MissingDependency：clock {id}/{phase}/{attempt}"))?;
        nonnegative(t, &k)?;
        Ok(t.to_string())
    }
}

#[derive(Clone)]
pub struct InputContext {
    envelope: Value,
    clock: ClockPlan,
}
impl InputContext {
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
fn stored_clock(conn: &Connection) -> Result<ClockPlan, String> {
    let (raw, hash): (String, String) = conn
        .query_row(
            "SELECT clock_plan_json,clock_plan_hash FROM s_protocol_meta WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(err)?;
    let clock = ClockPlan::parse(strict_json(raw.as_bytes()).map_err(err)?).map_err(err)?;
    if clock.hash != hash || canonical_json(&clock.value) != raw {
        return Err(err("clock计划/hash不一致"));
    }
    Ok(clock)
}
fn check_clock(conn: &Connection, clock: &ClockPlan) -> Result<(), String> {
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
    let old: String = conn
        .query_row(
            "SELECT logical_phase_frontier FROM s_protocol_meta WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
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
    let index:i64=conn.query_row("SELECT attempt_count FROM s_input_messages WHERE clock_event_id=?1 AND status='AcceptedPending'",params![c.id()],|r|r.get(0)).map_err(err)?;
    let (ordinal, inc): (i64, String) = conn
        .query_row(
            "SELECT next_attempt_ordinal,session_generation FROM s_protocol_meta WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(err)?;
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
pub fn seal_batch(
    conn: &Connection,
    c: Option<&InputContext>,
    batch: &mut Value,
) -> Result<(), String> {
    if !active(conn)? {
        return Ok(());
    }
    let c = c.ok_or("MissingDependency：缺v2上下文")?;
    let (inc,index):(String,i64)=conn.query_row("SELECT p.session_generation,m.attempt_count-1 FROM s_protocol_meta p JOIN s_input_messages m ON m.clock_event_id=?1 WHERE p.singleton=1",params![c.id()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(err)?;
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
    let allowed = [
        "schema_revision",
        "session_id",
        "session_generation",
        "source_namespace",
        "source_epoch",
        "message_id",
        "producer_id",
        "producer_epoch",
        "payload_hash",
        "causal_refs",
        "payload",
    ];
    let map = value.as_object().ok_or("SchemaUnsupported：消息必须对象")?;
    if map.len() != allowed.len() || map.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("SchemaUnsupported：公共头字段不完整/多余".into());
    }
    if required(&value, "schema_revision")? != "s-session/2" {
        return Err("SchemaUnsupported：仅s-session/2".into());
    }
    for k in [
        "session_id",
        "source_namespace",
        "source_epoch",
        "message_id",
        "producer_id",
    ] {
        required(&value, k)?;
    }
    positive(
        required(&value, "session_generation")?,
        "session_generation",
    )?;
    positive(required(&value, "producer_epoch")?, "producer_epoch")?;
    let refs = value["causal_refs"]
        .as_array()
        .ok_or("SchemaUnsupported：causal_refs必须数组")?;
    for r in refs {
        for k in [
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
        ] {
            required(r, k)?;
        }
    }
    if !value["payload"].is_object() {
        return Err("SchemaUnsupported：payload必须对象".into());
    }
    if required(&value, "payload_hash")? != sha256_hex(canonical_json(&value["payload"]).as_bytes())
    {
        return Err("IdentityConflict：payload_hash与规范业务内容不同".into());
    }
    required(&value["payload"], "op")?;
    Ok(value)
}
fn normalize_schema(s: &str) -> String {
    s.replace("IF NOT EXISTS", "")
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect()
}
fn verify_schema(conn: &Connection, v2: bool) -> Result<(), String> {
    let all = format!("{}{}", SCHEMA, if v2 { SCHEMA_V2 } else { "" });
    let mut expected = BTreeMap::new();
    for stmt in all.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let rest = stmt
            .strip_prefix("CREATE TABLE ")
            .ok_or("schema常量不是建表")?
            .strip_prefix("IF NOT EXISTS ")
            .unwrap_or_else(|| stmt.strip_prefix("CREATE TABLE ").unwrap());
        let name = rest
            .split(|c: char| c == '(' || c.is_whitespace())
            .next()
            .unwrap();
        expected.insert(name.to_string(), normalize_schema(stmt));
    }
    let mut q = conn
        .prepare(
            "SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(err)?;
    let mut rows = q.query([]).map_err(err)?;
    let mut seen = BTreeMap::new();
    while let Some(r) = rows.next().map_err(err)? {
        let kind: String = r.get(0).map_err(err)?;
        let name: String = r.get(1).map_err(err)?;
        let sql: String = r.get(2).map_err(err)?;
        if kind != "table" || expected.get(&name) != Some(&normalize_schema(&sql)) {
            return Err(err(format!("模式/约束不匹配 {kind}/{name}")));
        }
        seen.insert(name, true);
    }
    if seen.len() != expected.len() {
        return Err(err("缺必需表/约束"));
    }
    // SQLite 的模式声明不证明存量行满足声明：CHECK 可被另一连接关闭，TEXT PK 可为 NULL。
    // 对当前所有表逐列核实际类型；表名来自上方固定 DDL 的精确白名单。
    for table in expected.keys() {
        let mut columns = conn
            .prepare("SELECT type,\"notnull\",pk FROM pragma_table_info(?1) ORDER BY cid")
            .map_err(err)?;
        let declarations = columns
            .query_map(params![table], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        let mut query = conn
            .prepare(&format!("SELECT * FROM {table}"))
            .map_err(err)?;
        let mut values = query.query([]).map_err(err)?;
        while let Some(row) = values.next().map_err(err)? {
            for (i, (kind, notnull, pk)) in declarations.iter().enumerate() {
                use rusqlite::types::ValueRef;
                let value = row.get_ref(i).map_err(err)?;
                let valid = match value {
                    ValueRef::Null => *notnull == 0 && *pk == 0,
                    ValueRef::Integer(_) => kind == "INTEGER",
                    ValueRef::Text(bytes) => kind == "TEXT" && std::str::from_utf8(bytes).is_ok(),
                    ValueRef::Blob(_) => kind == "BLOB",
                    ValueRef::Real(_) => false,
                };
                if !valid {
                    return Err(err(format!("{table} 第{i}列实际类型/非空约束损坏")));
                }
            }
        }
    }
    Ok(())
}

fn verify_epoch_history(conn: &Connection, v2: bool) -> Result<Vec<(String, i64)>, String> {
    let generation = meta_i64(conn, "generation")?;
    let current = parse_writer_epoch(
        &meta_get_opt(conn, "writer_epoch")?.ok_or_else(|| err("缺writer_epoch"))?,
        "meta.writer_epoch",
    )
    .map_err(err)?;
    let mut query=conn.prepare("SELECT ordinal,from_epoch,to_epoch,generation_at_transition,advance_state_at_transition,transitioned_at FROM writer_epoch_history ORDER BY ordinal").map_err(err)?;
    let mut rows = query.query([]).map_err(err)?;
    let (mut previous_epoch, mut previous_generation, mut ordinal) = (None, 0, 0i64);
    let mut facts = Vec::new();
    while let Some(r) = rows.next().map_err(err)? {
        ordinal += 1;
        let actual: i64 = r.get(0).map_err(err)?;
        let from =
            parse_writer_epoch(&r.get::<_, String>(1).map_err(err)?, "epoch.from").map_err(err)?;
        let to =
            parse_writer_epoch(&r.get::<_, String>(2).map_err(err)?, "epoch.to").map_err(err)?;
        let at =
            nonnegative(&r.get::<_, String>(3).map_err(err)?, "epoch.generation").map_err(err)?;
        let state: String = r.get(4).map_err(err)?;
        let ns: String = r.get(5).map_err(err)?;
        nonnegative(&ns, "epoch.transitioned_at").map_err(err)?;
        if actual != ordinal
            || previous_epoch.is_some_and(|p| p != from)
            || to <= from
            || at < previous_generation
            || at > generation
            || (v2 && state != "idle")
        {
            return Err(err("writer换代历史序号/接续/代际/状态不一致"));
        }
        previous_epoch = Some(to);
        previous_generation = at;
        facts.push((ns, at));
    }
    if previous_epoch.is_some_and(|last| last != current) {
        return Err(err("writer_epoch与最后换代结果不一致"));
    }
    Ok(facts)
}

/// 所有当前控制行都新鲜检查，不能按generation过滤未来/同代损坏或信任自报摘要。
pub fn verify_control(
    conn: &Connection,
    batches: &BTreeMap<String, std::sync::Arc<Value>>,
) -> Result<(), String> {
    let enabled = active(conn)?;
    verify_schema(conn, enabled)?;
    let mut epoch_facts = verify_epoch_history(conn, enabled)?;
    if !enabled {
        return Ok(());
    }
    for table in ["s_protocol_meta", "s_delivery_policy"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .map_err(err)?;
        let singleton: i64 = conn
            .query_row(&format!("SELECT singleton FROM {table}"), [], |r| r.get(0))
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
        if meta_get_opt(conn, key)?.is_none_or(|s| s.is_empty()) {
            return Err(err(format!("v2初始化缺完整{key}绑定")));
        }
    }
    // v1 持久 epoch 保留既有非负域；v2 的 S 公共回复 producer_epoch 是正数。
    // 该协议没有 v1→v2 原库迁移，正式 v2 init 从 1 开始且 recover 只增大。
    positive(
        &meta_get_opt(conn, "writer_epoch")?.ok_or_else(|| err("缺writer_epoch"))?,
        "v2 writer_epoch",
    )
    .map_err(err)?;
    let clock = stored_clock(conn)?;
    let (protocol,inc,next,frontier):(String,String,i64,String)=conn.query_row("SELECT protocol_revision,session_generation,next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(err)?;
    if protocol != "s-session/2" || next < 1 {
        return Err(err("协议版本/attempt域错误"));
    }
    positive(&inc, "session_generation").map_err(err)?;
    let root_g = meta_i64(conn, "generation")?;
    let next_g = root_g.checked_add(1).ok_or_else(|| err("generation溢出"))?;
    let sid = meta_get_opt(conn, "session_id")?.ok_or_else(|| err("缺session"))?;
    let catalog_raw =
        meta_get_opt(conn, "catalog_definition")?.ok_or_else(|| err("v2缺catalog来源"))?;
    let catalog_hash =
        meta_get_opt(conn, "catalog_hash")?.ok_or_else(|| err("v2缺catalog hash"))?;
    let catalog = strict_json(catalog_raw.as_bytes()).map_err(err)?;
    if canonical_json(&catalog) != catalog_raw
        || sha256_hex(catalog_raw.as_bytes()) != catalog_hash
        || catalog["catalog_revision"].as_str()
            != meta_get_opt(conn, "catalog_revision")?.as_deref()
    {
        return Err(err("目录来源/hash/revision不一致"));
    }
    let items = catalog["items"]
        .as_array()
        .ok_or_else(|| err("目录源items缺失"))?;
    let actual_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM catalog", [], |r| r.get(0))
        .map_err(err)?;
    if actual_count as usize != items.len() {
        return Err(err("目录静态行数不同"));
    }
    for item in items {
        let actual: (String, String, String, String) = conn
            .query_row(
                "SELECT kind,title,domain,branches_json FROM catalog WHERE catalog_id=?1",
                params![required(item, "id").map_err(err)?],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(err)?;
        if item["kind"] != actual.0
            || item["title"] != actual.1
            || item["domain"] != actual.2
            || item["branches"] != strict_json(actual.3.as_bytes()).map_err(err)?
        {
            return Err(err("目录静态内容与来源不同"));
        }
    }
    let (ps,pi,pr,keep,first,head):(String,String,String,i64,i64,i64)=conn.query_row("SELECT session_id,session_generation,policy_revision,retain_generations,first_available_generation,head_generation FROM s_delivery_policy WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(err)?;
    if ps != sid
        || pi != inc
        || pr != "generation-window/1"
        || keep < 1
        || head != root_g
        || first != 1.max(head - keep + 1)
    {
        return Err(err("普通投递frontier/policy与根不一致"));
    }
    let mut stmt = conn
        .prepare("SELECT generation FROM s_delivery_refs ORDER BY generation")
        .map_err(err)?;
    let refs = stmt
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    if refs.len() as i128 != (head as i128 - first as i128 + 1).max(0)
        || refs
            .iter()
            .enumerate()
            .any(|(i, g)| *g as i128 != first as i128 + i as i128)
    {
        return Err(err("普通投递引用缺失/未来/越保留界"));
    }
    let publication_frontiers: Vec<(i64, i64)> = {
        let mut q = conn
            .prepare("SELECT generation,input_frontier FROM structure_deltas ORDER BY generation")
            .map_err(err)?;
        let out = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        out
    };
    if publication_frontiers.windows(2).any(|w| w[0].1 > w[1].1) {
        return Err(err("发布前沿倒退"));
    }
    let mut stmt=conn.prepare("SELECT source_namespace,source_epoch,message_id,payload_hash,canonical_envelope,receipt_id,accepted_seq,status,first_published_generation,clock_event_id,attempt_count FROM s_input_messages ORDER BY accepted_seq,message_id").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    let mut messages = BTreeMap::new();
    let mut message_sequences = std::collections::BTreeSet::new();
    while let Some(r) = rows.next().map_err(err)? {
        let (ns, se, id, hash, raw, receipt, seq, status, published, event, count): (
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            String,
            Option<i64>,
            String,
            i64,
        ) = (
            r.get(0).map_err(err)?,
            r.get(1).map_err(err)?,
            r.get(2).map_err(err)?,
            r.get(3).map_err(err)?,
            r.get(4).map_err(err)?,
            r.get(5).map_err(err)?,
            r.get(6).map_err(err)?,
            r.get(7).map_err(err)?,
            r.get(8).map_err(err)?,
            r.get(9).map_err(err)?,
            r.get(10).map_err(err)?,
        );
        let e = envelope(strict_json(raw.as_bytes()).map_err(err)?).map_err(err)?;
        if canonical_json(&e) != raw
            || e["source_namespace"] != ns
            || e["source_epoch"] != se
            || e["message_id"] != id
            || e["payload_hash"] != hash
            || e["session_id"] != sid
            || e["session_generation"] != inc
            || e["payload"]["clock_event_id"] != event
            || count < 0
            || seq < 0
        {
            return Err(err("消息独立列/规范原件不一致"));
        }
        validate_ingest(&e).map_err(err)?;
        let input: RawInputFile =
            serde_json::from_value(e["payload"]["raw_input"].clone()).map_err(err)?;
        if input.events.len() != 1 {
            return Err(err("持久输入消息非单事件"));
        }
        let ev = &input.events[0];
        let ik = identity_key(
            &input.source_namespace,
            &input.source_epoch,
            &input.instrument,
            &ev.event_id,
        );
        let actual: (String, String, String, i64) = conn
            .query_row(
                "SELECT receipt_id,payload_hash,identity_key,revision FROM raw_events WHERE seq=?1",
                params![seq],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(err)?;
        if actual.0 != receipt
            || actual.1 != sha256_hex(canonical_event_content(ev).as_bytes())
            || actual.2 != ik
            || actual.3 != parse_canonical_i64(&ev.revision, "revision").map_err(err)?
        {
            return Err(err("消息/raw receipt内容或身份不一致"));
        }
        let first_pub = publication_frontiers
            .get(publication_frontiers.partition_point(|(_, frontier)| *frontier < seq))
            .map(|(g, _)| *g);
        if published != first_pub
            || status
                != if published.is_some() {
                    "Committed"
                } else {
                    "AcceptedPending"
                }
        {
            return Err(err("消息持久结果与最早已发布前沿不符"));
        }
        clock.phase(&event, "accept", -1).map_err(err)?;
        message_sequences.insert(seq);
        if messages
            .insert(
                event,
                (canonical_json(&json!([ns, se, id])), count, published),
            )
            .is_some()
        {
            return Err(err("重复clock消息身份"));
        }
    }
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(err)?;
    if message_sequences.len() as i128 != raw_count as i128
        || message_sequences
            .iter()
            .enumerate()
            .any(|(i, seq)| *seq as i128 != i as i128)
    {
        return Err(err("v2原始接纳序缺持久消息承接"));
    }
    let mut stmt=conn.prepare("SELECT clock_event_id,phase,attempt_index,semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events ORDER BY semantic_ns,clock_event_id,phase,attempt_index").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    let mut seen = BTreeMap::new();
    let mut ordinals = BTreeMap::new();
    let mut commits_by_generation = BTreeMap::new();
    let mut begins_by_message: BTreeMap<String, std::collections::BTreeSet<i64>> = BTreeMap::new();
    let mut recover_facts = Vec::new();
    let mut max_ns = -1;
    while let Some(r) = rows.next().map_err(err)? {
        let (id, phase, index, ns, key, ord, g): (
            String,
            String,
            i64,
            String,
            String,
            Option<i64>,
            i64,
        ) = (
            r.get(0).map_err(err)?,
            r.get(1).map_err(err)?,
            r.get(2).map_err(err)?,
            r.get(3).map_err(err)?,
            r.get(4).map_err(err)?,
            r.get(5).map_err(err)?,
            r.get(6).map_err(err)?,
        );
        if clock.phase(&id, &phase, index).map_err(err)? != ns || g < 0 || g > next_g {
            return Err(err("phase时间/代际与计划不一致"));
        }
        max_ns = max_ns.max(nonnegative(&ns, "semantic_ns").map_err(err)?);
        if phase == "recover" {
            if index != -1 || ord.is_some() || !key.is_empty() || g > root_g {
                return Err(err("recover phase归属错误"));
            }
            recover_facts.push((ns.clone(), g));
        } else {
            let (expected_key, count, published) =
                messages.get(&id).ok_or_else(|| err("phase没有原消息"))?;
            if &key != expected_key {
                return Err(err("phase消息归属错误"));
            }
            if phase == "accept" {
                if index != -1 || ord.is_some() || g > root_g {
                    return Err(err("accept phase域错误"));
                }
            } else if (phase != "begin" && phase != "commit")
                || index < 0
                || index >= *count
                || ord.is_none_or(|o| o < 1 || o >= next)
            {
                return Err(err("attempt phase域错误"));
            }
            if phase == "begin" && ordinals.insert(ord.unwrap(), (id.clone(), index)).is_some() {
                return Err(err("重复全局attempt"));
            }
            if phase == "begin" {
                begins_by_message
                    .entry(id.clone())
                    .or_default()
                    .insert(index);
            }
            if phase == "begin" && g != published.unwrap_or(next_g) {
                return Err(err("Begin代际与消息推进目标不同"));
            }
            if phase == "commit" && *published != Some(g) {
                return Err(err("commit phase与实际首次发布不一致"));
            }
        }
        if phase == "commit"
            && commits_by_generation
                .insert(g, (id.clone(), index))
                .is_some()
        {
            return Err(err("每代重复Commit phase"));
        }
        if seen.insert((id, phase, index), (ord, g)).is_some() {
            return Err(err("重复phase"));
        }
    }
    if parse_canonical_i64(&frontier, "logical_phase_frontier").map_err(err)? != max_ns
        || next as i128 != ordinals.len() as i128 + 1
        || ordinals
            .keys()
            .enumerate()
            .any(|(i, o)| *o as i128 != i as i128 + 1)
    {
        return Err(err("phase frontier/attempt序列断裂"));
    }
    for (id, (_, count, published)) in &messages {
        if !seen.contains_key(&(id.clone(), "accept".into(), -1)) {
            return Err(err("消息缺接纳phase"));
        }
        let indexes = begins_by_message.get(id);
        if indexes.map_or(0, |v| v.len()) as i128 != *count as i128
            || indexes.is_some_and(|v| {
                v.iter()
                    .enumerate()
                    .any(|(i, actual)| i as i128 != *actual as i128)
            })
        {
            return Err(err("消息attempt_count与实际Begin集合不闭合"));
        }
        for &i in indexes.into_iter().flatten() {
            let begin = seen
                .get(&(id.clone(), "begin".into(), i))
                .ok_or_else(|| err("attempt缺Begin"))?;
            if let Some(commit) = seen.get(&(id.clone(), "commit".into(), i)) {
                if begin != commit {
                    return Err(err("Begin/Commit ordinal/generation不一致"));
                }
            }
        }
        if *count > 0
            && published.is_some()
            && !seen.contains_key(&(id.clone(), "commit".into(), count - 1))
        {
            return Err(err("已发布消息缺最后Commit phase"));
        }
    }
    epoch_facts.sort();
    recover_facts.sort();
    if epoch_facts != recover_facts {
        return Err(err("recover phase与writer换代时间/代际不闭合"));
    }
    let mut q = conn
        .prepare("SELECT generation,index_frontier FROM structure_deltas ORDER BY generation")
        .map_err(err)?;
    let mut rows = q.query([]).map_err(err)?;
    while let Some(r) = rows.next().map_err(err)? {
        let g: i64 = r.get(0).map_err(err)?;
        let id: String = r.get(1).map_err(err)?;
        let batch = batches.get(&id).ok_or_else(|| err("v2缺可达批次"))?;
        let (id, index) = commits_by_generation
            .get(&g)
            .ok_or_else(|| err("v2发布缺真实Commit phase"))?;
        if batch["protocol_revision"] != "s-session/2"
            || batch["session_generation"] != inc
            || batch["clock_plan_hash"] != clock.hash
            || batch["semantic_commit_ns"] != clock.phase(id, "commit", *index).map_err(err)?
        {
            return Err(err("发布batch与真实语义clock/session不一致"));
        }
    }
    Ok(())
}

fn process_lock(db: &Path) -> Result<File, String> {
    let canonical = if db.exists() {
        db.canonicalize().map_err(err)?
    } else {
        let parent = db
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        parent
            .canonicalize()
            .map_err(err)?
            .join(db.file_name().ok_or("InvalidDomain：数据库路径缺文件名")?)
    };
    let p = PathBuf::from(format!("{}.writer.lock", canonical.display()));
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(p)
        .map_err(err)?;
    f.try_lock()
        .map_err(|e| format!("StaleWriter：S独占进程锁不可得：{e}"))?;
    Ok(f)
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
fn protocol_identity(conn: &Connection) -> Result<(String, String, String), String> {
    Ok((
        meta_get_opt(conn, "session_id")?.ok_or_else(|| err("缺session"))?,
        conn.query_row(
            "SELECT session_generation FROM s_protocol_meta WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?,
        meta_get_opt(conn, "writer_epoch")?.ok_or_else(|| err("缺epoch"))?,
    ))
}
fn lookup(conn: &Connection, ns: &str, se: &str, id: &str, hash: &str) -> Result<Value, String> {
    let row:Option<(String,String,i64,String,Option<i64>)>=conn.query_row("SELECT payload_hash,receipt_id,accepted_seq,status,first_published_generation FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",params![ns,se,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(err)?;
    let Some((stored, receipt, seq, status, g)) = row else {
        return Ok(json!({"ok":true,"kind":"Unknown","original_message_id":id}));
    };
    if stored != hash {
        return Err("IdentityConflict：原message身份绑定不同内容".into());
    }
    let publication = if let Some(g) = g {
        conn.query_row("SELECT generation,next_cut,index_frontier,catalog_revision,input_frontier FROM structure_deltas WHERE generation=?1",params![g],|r|Ok(json!({"generation":r.get::<_,i64>(0)?.to_string(),"structure_cut":r.get::<_,String>(1)?,"index_frontier":r.get::<_,String>(2)?,"catalog_revision":r.get::<_,String>(3)?,"input_frontier":r.get::<_,i64>(4)?.to_string()}))).map_err(err)?
    } else {
        Value::Null
    };
    Ok(
        json!({"ok":true,"kind":status,"original_message_id":id,"receipt_id":receipt,"accepted_seq":seq.to_string(),"publication":publication}),
    )
}
fn ready(conn: &Connection) -> Result<Value, String> {
    let (sid, inc, epoch) = protocol_identity(conn)?;
    let clock = stored_clock(conn)?;
    let (first,head,keep):(i64,i64,i64)=conn.query_row("SELECT first_available_generation,head_generation,retain_generations FROM s_delivery_policy WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(err)?;
    Ok(
        json!({"ok":true,"kind":"Ready","schema_revision":"s-session/2","session_id":sid,"session_generation":inc,"writer_epoch":epoch,"profile_id":meta_get_opt(conn,"profile_id")?,"profile_hash":meta_get_opt(conn,"profile_hash")?,"clock_plan_hash":clock.hash,"advance_state":meta_get_opt(conn,"advance_state")?,"generation":meta_get_opt(conn,"generation")?,"structure_cut":meta_get_opt(conn,"structure_cut")?,"index_frontier":meta_get_opt(conn,"index_frontier")?,"delivery_frontier":{"first_available_generation":first.to_string(),"head_generation":head.to_string(),"min_valid_after_generation":(first-1).to_string(),"retain_generations":keep.to_string()}}),
    )
}
fn validate_ingest(e: &Value) -> Result<(), String> {
    let p = &e["payload"];
    let keys = [
        "op",
        "target_session_id",
        "target_session_generation",
        "writer_epoch",
        "clock_event_id",
        "raw_input",
    ];
    if p.as_object().map(|m| m.len()) != Some(keys.len())
        || p.as_object()
            .unwrap()
            .keys()
            .any(|k| !keys.contains(&k.as_str()))
    {
        return Err("SchemaUnsupported：ingest字段不完整/多余".into());
    }
    if p["target_session_id"] != e["session_id"]
        || p["target_session_generation"] != e["session_generation"]
    {
        return Err("IdentityConflict：target与公共头不同".into());
    }
    required(p, "clock_event_id")?;
    positive(required(p, "writer_epoch")?, "writer_epoch")?;
    let raw = &p["raw_input"];
    let rk = [
        "schema_revision",
        "session_id",
        "source_namespace",
        "source_epoch",
        "instrument",
        "profile",
        "events",
    ];
    if raw.as_object().map(|m| m.len()) != Some(rk.len())
        || raw
            .as_object()
            .unwrap()
            .keys()
            .any(|k| !rk.contains(&k.as_str()))
    {
        return Err("SchemaUnsupported：原始输入字段不完整/多余".into());
    }
    if raw["schema_revision"] != "1"
        || raw["session_id"] != e["session_id"]
        || raw["source_namespace"] != e["source_namespace"]
        || raw["source_epoch"] != e["source_epoch"]
    {
        return Err("IdentityConflict：原始输入与公共头不同".into());
    }
    let events = raw["events"]
        .as_array()
        .filter(|a| a.len() == 1)
        .ok_or("InvalidDomain：ingest必须一事件")?;
    let ek = [
        "event_id",
        "revision",
        "seq",
        "received_at",
        "raw_text",
        "price",
        "timestamp",
        "volume",
    ];
    if events[0].as_object().map(|m| m.len()) != Some(ek.len())
        || events[0]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| !ek.contains(&k.as_str()))
    {
        return Err("SchemaUnsupported：event字段不完整/多余".into());
    }
    for k in ek {
        events[0]
            .get(k)
            .and_then(Value::as_str)
            .ok_or("SchemaUnsupported：raw event字段必须文本")?;
    }
    Ok(())
}
fn handle(
    db: &Path,
    profile: &Path,
    epoch: &str,
    clock: &ClockPlan,
    e: &Value,
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
            advance_core(db, epoch, Some(&c))?;
            let conn = open_db(db)?;
            let tx = conn.unchecked_transaction().map_err(err)?;
            verify_reachable_root(&tx)?;
            lookup(
                &tx,
                required(e, "source_namespace")?,
                required(e, "source_epoch")?,
                required(e, "message_id")?,
                required(e, "payload_hash")?,
            )
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
    let payload = result.unwrap_or_else(|error| json!({"ok":false,"error":error,"identity_binding":"verified_at_service_start_only"}));
    let hash = sha256_hex(canonical_json(&payload).as_bytes());
    let id = format!(
        "reply-{}",
        sha256_hex(
            canonical_json(&json!([
                e["source_namespace"],
                e["source_epoch"],
                e["message_id"],
                hash
            ]))
            .as_bytes()
        )
    );
    let ready = payload["kind"] == "Ready";
    let mut reply = json!({"schema_revision":"s-session/2","session_id":identity.0,"session_generation":identity.1,"source_namespace":"s-session/replies","source_epoch":identity.2,"message_id":id,"producer_id":format!("S:{}",identity.0),"producer_epoch":identity.2,"payload_hash":hash,"causal_refs":[{"source_namespace":e["source_namespace"],"source_epoch":e["source_epoch"],"message_id":e["message_id"],"payload_hash":e["payload_hash"]}],"payload":payload});
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
struct Job {
    envelope: Value,
    result: mpsc::SyncSender<Value>,
}
fn read_frame(stream: &mut UnixStream, max: usize, deadline: Instant) -> Result<Value, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("TransportTimeout：请求读期限")?;
        stream.set_read_timeout(Some(remaining)).map_err(err)?;
        let n = stream
            .read(&mut chunk)
            .map_err(|e| format!("TransportUnavailable：{e}"))?;
        if n == 0 {
            break;
        }
        if bytes.len() + n > max {
            return Err("SchemaUnsupported：frame字节超过已声明界限".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    if bytes.last() != Some(&b'\n') || bytes[..bytes.len().saturating_sub(1)].contains(&b'\n') {
        return Err("SchemaUnsupported：只接收一个LF终止帧与请求EOF".into());
    }
    let value = envelope(strict_json(&bytes[..bytes.len() - 1])?)?;
    if Instant::now() > deadline {
        return Err("TransportTimeout：JSON解码超期".into());
    }
    Ok(value)
}
fn io_connection(
    mut stream: UnixStream,
    tx: mpsc::SyncSender<Job>,
    bounds: Bounds,
    count: Arc<AtomicUsize>,
) {
    struct Release(Arc<AtomicUsize>);
    impl Drop for Release {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let _release = Release(count);
    let deadline = Instant::now() + bounds.read;
    let output = match read_frame(&mut stream, bounds.frame, deadline) {
        Err(error) => json!({"ok":false,"error":error}),
        Ok(e) => {
            let response_deadline = Instant::now() + bounds.response;
            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
            let mut job = Job {
                envelope: e,
                result: reply_tx,
            };
            loop {
                match tx.try_send(job) {
                    Ok(()) => break,
                    Err(mpsc::TrySendError::Full(j)) => {
                        job = j;
                        if Instant::now() >= response_deadline {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => return,
                }
            }
            // 已入actor后，回包超期只结束此连接；不能撤销/重复权威工作。
            match reply_rx.recv_timeout(response_deadline.saturating_duration_since(Instant::now()))
            {
                Ok(v) => v,
                Err(_) => return,
            }
        }
    };
    let body = format!("{}\n", canonical_json(&output));
    if body.len() > bounds.frame {
        return;
    }
    let deadline = Instant::now() + bounds.write;
    let mut sent = 0;
    while sent < body.len() {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if stream.set_write_timeout(Some(left)).is_err() {
            break;
        }
        match stream.write(&body.as_bytes()[sent..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => sent += n,
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
fn serve(
    db: &Path,
    socket: &Path,
    profile: &Path,
    epoch: &str,
    clock_path: &Path,
    bounds: Bounds,
) -> Result<(), String> {
    positive(epoch, "v2 writer_epoch")?;
    cache::configure(bounds.cache_bytes);
    // 监督器nonce仅用于把Ready绑定真实启动进程；绝不进入业务payload/hash/持久事实。
    let control_instance_id = std::env::var("S_CONTROL_INSTANCE_ID").ok();
    let _lock = process_lock(db)?;
    let clock = ClockPlan::load(clock_path)?;
    let conn = open_db(db)?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    verify_writer_epoch(&tx, epoch)?;
    verify_reachable_root(&tx)?;
    if !active(&tx)? {
        return Err("SchemaUnsupported：serve需要显式v2初始化".into());
    }
    check_clock(&tx, &clock)?;
    let (_, _, profile_hash) = load_profile(profile)?;
    if meta_get_opt(&tx, "profile_hash")?.as_deref() != Some(&profile_hash) {
        return Err("IdentityConflict：serve profile绑定不同".into());
    }
    if meta_get_opt(&tx, "advance_state")?.as_deref() != Some("idle") {
        return Err("StaleWriter：未完成Begin保持闭门，请监督器先正式recover".into());
    }
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
        advance_core(db, epoch, Some(&context))?;
    }
    // 不擅自unlink既有socket：SIGKILL残留由监督器核路径/PID后清理。
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("TransportUnavailable：socket绑定失败 {e}"))?;
    let (tx, rx) = mpsc::sync_channel::<Job>(bounds.queue);
    let count = Arc::new(AtomicUsize::new(0));
    let accept_bounds = bounds.clone();
    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { break };
            if count
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                    if n < accept_bounds.connections {
                        Some(n + 1)
                    } else {
                        None
                    }
                })
                .is_err()
            {
                drop(stream);
                continue;
            }
            let t = tx.clone();
            let b = accept_bounds.clone();
            let c = count.clone();
            std::thread::spawn(move || io_connection(stream, t, b, c));
        }
    });
    for job in rx {
        let out = response(
            &job.envelope,
            handle(db, profile, epoch, &clock, &job.envelope),
            &identity,
            control_instance_id.as_deref(),
        );
        // try_send保证慢客户端不能挡住下一输入；权威结果已持久可查。
        let _ = job.result.try_send(out);
    }
    Ok(())
}
fn arg(args: &[String], key: &str) -> Result<String, String> {
    arg_value(args, key).ok_or_else(|| format!("缺 {key}"))
}
fn positive_arg(args: &[String], key: &str) -> Result<i64, String> {
    positive(&arg(args, key)?, key)
}
pub fn dispatch(args: &[String]) -> Result<bool, String> {
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
        serve(
            Path::new(&arg(args, "--db")?),
            Path::new(&arg(args, "--socket")?),
            Path::new(&arg(args, "--profile")?),
            &arg(args, "--writer-epoch")?,
            Path::new(&arg(args, "--clock-plan")?),
            bounds,
        )?;
        return Ok(true);
    }
    if cmd == "recover"
        && ["--clock-plan", "--clock-event-id"]
            .iter()
            .any(|k| args.iter().any(|a| a == k))
    {
        let db = PathBuf::from(arg(args, "--db")?);
        let _lock = process_lock(&db)?;
        let clock = ClockPlan::load(Path::new(&arg(args, "--clock-plan")?))?;
        let out = recover_core(
            &db,
            &arg(args, "--new-epoch")?,
            Some((&clock, &arg(args, "--clock-event-id")?)),
        )?;
        println!("{out}");
        return Ok(true);
    }
    Ok(false)
}

pub fn legacy_write_lock(args: &[String]) -> Result<Option<File>, String> {
    if matches!(
        args.get(1).map(String::as_str),
        Some("init" | "reset" | "accept" | "advance" | "recover")
    ) {
        let db = PathBuf::from(arg(args, "--db")?);
        return process_lock(&db).map(Some);
    }
    Ok(None)
}

/// v2普通Watch只能走独立保留引用；旧v1数据仍由原命令实现读取。
pub fn retained_watch(conn: &Connection, after: i64) -> Result<Option<Value>, String> {
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
        let mut q=conn.prepare("SELECT d.generation,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.delta_json FROM s_delivery_refs r JOIN structure_deltas d ON d.generation=r.generation WHERE d.generation>?1 ORDER BY d.generation").map_err(err)?;
        let values = q
            .query_map(params![after], project_delta_row)
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        values
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
    static N: AtomicUsize = AtomicUsize::new(0);
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
            handle(&self.db, &self.profile, "1", &self.clock, &m).unwrap()
        }
        fn dump(&self) -> Vec<(String, Vec<String>)> {
            let conn = open_db(&self.db).unwrap();
            let names = [
                "meta",
                "raw_events",
                "batches",
                "objects",
                "witnesses",
                "relations",
                "observations",
                "structure_deltas",
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
            handle(&f.db, &f.profile, "1", &f.clock, &replay).unwrap()["kind"],
            "Committed"
        );
        verify_reachable_root(&conn).unwrap();
        // 缓存极小须回完整门，既不拒合法库也不留下前缀健康标签。
        cache::configure(1);
        verify_reachable_root(&conn).unwrap();
        assert_eq!(cache::last_resumed(), 0);
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
            "DELETE FROM meta WHERE key='profile_definition'",
            "UPDATE meta SET value='foreign' WHERE key='catalog_hash'",
            "UPDATE meta SET value='-1' WHERE key='writer_epoch'",
            "UPDATE s_input_messages SET canonical_envelope='{}'",
            "DELETE FROM s_clock_events WHERE phase='accept'",
            "UPDATE s_protocol_meta SET logical_phase_frontier='0'",
            "DELETE FROM s_delivery_refs",
            "UPDATE s_delivery_policy SET first_available_generation=3",
            "CREATE TABLE foreign_schema(value TEXT)",
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
                advance_core(&f.db, "1", Some(&c)).is_err(),
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
    fn v2_same_message_cannot_change_clock_or_payload() {
        let f = Fixture::new(8, 2);
        f.ingest(0);
        let before = f.dump();
        let mut e = f.message(0);
        e["payload"]["clock_event_id"] = json!("op-1");
        e["payload_hash"] = json!(sha256_hex(canonical_json(&e["payload"]).as_bytes()));
        assert!(handle(&f.db, &f.profile, "1", &f.clock, &e)
            .unwrap_err()
            .contains("IdentityConflict"));
        assert_eq!(before, f.dump());
    }
    #[test]
    fn v2_pending_recovery_preserves_identity_and_uses_next_attempt() {
        let f = Fixture::new(8, 2);
        let c = f.begin_only();
        recover_core(&f.db, "2", Some((&f.clock, "recover-1"))).unwrap();
        advance_core(&f.db, "2", Some(&c)).unwrap();
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
        assert!(advance_core(&f.db, "2", Some(&c))
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
                handle(&f.db, &f.profile, "1", &f.clock, &f.message(4)).is_err(),
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
        assert!(advance_core(&f.db, "1", Some(&c)).is_err());
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
                advance_core(&f.db,"1",Some(&c)).is_err(),
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
        advance_core(&f.db, "2", Some(&c)).unwrap();
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
            assert!(advance_core(&f.db, "1", Some(&c)).is_err(), "{sql}");
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
                    },
                    remaining,
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
        assert!(read_frame(&mut a, 1024 * 1024, Instant::now() + Duration::from_secs(1)).is_err());
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
}

#[cfg(test)]
#[test]
#[ignore = "explicit isolated database diagnostic"]
fn testonly_audit_cost_breakdown() {
    let path = std::env::var("S_AUDIT_COST_DB").expect("isolated copy path required");
    let conn = open_db(Path::new(&path)).unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    cache::configure(268435456);
    for iteration in 0..3 {
        let t = Instant::now();
        let batches = capture_reachable_batches(&tx).unwrap();
        let b = t.elapsed();
        verify_control(&tx, &batches).unwrap();
        let c = t.elapsed();
        verify_reachable_root_in_tx(&tx, &batches).unwrap();
        let r = t.elapsed();
        eprintln!(
            "AUDIT_COST {}",
            json!({"iteration":iteration,"generation":meta_i64(&tx,"generation").unwrap(),"batches_ms":b.as_secs_f64()*1000.0,"control_ms":(c-b).as_secs_f64()*1000.0,"structure_ms":(r-c).as_secs_f64()*1000.0,"total_ms":r.as_secs_f64()*1000.0})
        );
    }
}
