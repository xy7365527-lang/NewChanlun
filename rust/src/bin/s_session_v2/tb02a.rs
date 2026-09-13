//! #1373：OHLC 输入、同核包含事实的持久投影。所有结构判断来自 parser。
use super::*;

pub(super) const PROFILE: &str = "ohlc_integer_tb02a_v1";
pub(super) const RAW_SCHEMA: &str = "s-ohlc/1";
pub(super) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS raw_ohlc (
  identity_key TEXT NOT NULL,
  revision INTEGER NOT NULL,
  schema_revision TEXT NOT NULL,
  open TEXT NOT NULL,
  high TEXT NOT NULL,
  low TEXT NOT NULL,
  close TEXT NOT NULL,
  PRIMARY KEY(identity_key,revision)
);
CREATE TABLE IF NOT EXISTS structure_facts (
  object_id TEXT PRIMARY KEY,
  object_revision INTEGER NOT NULL,
  kind TEXT NOT NULL,
  batch_id TEXT NOT NULL,
  fact_key TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  input_refs_json TEXT NOT NULL,
  source_coords_json TEXT NOT NULL,
  first_known_generation INTEGER NOT NULL,
  first_known_cut TEXT NOT NULL,
  published_generation INTEGER NOT NULL,
  withdrawn_generation INTEGER,
  withdrawal_reason TEXT,
  superseded_by TEXT
);
"#;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ohlc {
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
}

pub(super) fn enabled(conn: &Connection) -> Result<bool, String> {
    Ok(meta_get_opt(conn, "profile_id")?.as_deref() == Some(PROFILE))
}

pub(super) fn parse_input(value: &Value) -> Result<RawInputFile, String> {
    let top = [
        "schema_revision",
        "session_id",
        "source_namespace",
        "source_epoch",
        "instrument",
        "profile",
        "events",
    ];
    exact_keys(value, &top)?;
    for key in &top[..6] {
        text(value, key)?;
    }
    let schema = text(value, "schema_revision")?;
    let profile = text(value, "profile")?;
    let (keys, ohlc) = match (schema, profile) {
        ("1", "testonly_tick_1_1_ohlc") => (
            vec![
                "event_id",
                "revision",
                "seq",
                "received_at",
                "raw_text",
                "price",
                "timestamp",
                "volume",
            ],
            false,
        ),
        (RAW_SCHEMA, PROFILE) => (
            vec![
                "event_id",
                "revision",
                "seq",
                "received_at",
                "raw_text",
                "open",
                "high",
                "low",
                "close",
                "timestamp",
                "volume",
            ],
            true,
        ),
        _ => return Err("SchemaUnsupported：schema_revision/profile组合未声明".into()),
    };
    let mut events = Vec::new();
    for e in value["events"]
        .as_array()
        .ok_or("SchemaUnsupported：events必须数组")?
    {
        exact_keys(e, &keys)?;
        for key in &keys {
            text(e, key)?;
        }
        let prices = if ohlc {
            Some(Ohlc {
                open: text(e, "open")?.into(),
                high: text(e, "high")?.into(),
                low: text(e, "low")?.into(),
                close: text(e, "close")?.into(),
            })
        } else {
            None
        };
        let event = RawEvent {
            event_id: text(e, "event_id")?.into(),
            revision: text(e, "revision")?.into(),
            seq: text(e, "seq")?.into(),
            received_at: text(e, "received_at")?.into(),
            raw_text: text(e, "raw_text")?.into(),
            price: if let Some(p) = &prices {
                p.close.clone()
            } else {
                text(e, "price")?.into()
            },
            timestamp: text(e, "timestamp")?.into(),
            volume: text(e, "volume")?.into(),
            ohlc: prices,
        };
        validate_prices(&event)?;
        events.push(event);
    }
    Ok(RawInputFile {
        schema_revision: schema.into(),
        session_id: text(value, "session_id")?.into(),
        source_namespace: text(value, "source_namespace")?.into(),
        source_epoch: text(value, "source_epoch")?.into(),
        instrument: text(value, "instrument")?.into(),
        profile: profile.into(),
        events,
    })
}

fn exact_keys(value: &Value, keys: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("SchemaUnsupported：预期对象")?;
    if map.len() != keys.len() || map.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err("SchemaUnsupported：字段缺失或多余".into());
    }
    Ok(())
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("SchemaUnsupported：{key}必须文本"))
}
pub(super) fn validate_prices(event: &RawEvent) -> Result<(), String> {
    if let Some(p) = &event.ohlc {
        let o = parse_canonical_i64(&p.open, "open")?;
        let h = parse_canonical_i64(&p.high, "high")?;
        let l = parse_canonical_i64(&p.low, "low")?;
        let c = parse_canonical_i64(&p.close, "close")?;
        if l > o || o > h || l > c || c > h || event.price != p.close {
            return Err("InvalidDomain：OHLC几何或close索引不一致".into());
        }
        if parse_canonical_i64(&event.volume, "volume")? < 0 {
            return Err("InvalidDomain：volume为负".into());
        }
    }
    Ok(())
}
pub(super) fn canonical(event: &RawEvent, p: &Ohlc) -> String {
    canonical_json(
        &json!({"event_id":event.event_id,"revision":event.revision,"seq":event.seq,
        "received_at":event.received_at,"raw_text":event.raw_text,"open":p.open,"high":p.high,
        "low":p.low,"close":p.close,"timestamp":event.timestamp,"volume":event.volume}),
    )
}
pub(super) fn save_raw(
    conn: &Connection,
    identity: &str,
    revision: i64,
    p: &Ohlc,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO raw_ohlc VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![identity, revision, RAW_SCHEMA, p.open, p.high, p.low, p.close],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub(super) fn enrich_raw(conn: &Connection, rows: &mut [Value]) -> Result<(), String> {
    if !enabled(conn)? {
        return Ok(());
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_ohlc", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count != raw_count {
        return Err("StorageUnavailable：raw_ohlc与原始账不一一对应".into());
    }
    for row in rows {
        let revision = match &row["revision"] {
            Value::Number(n) => n.as_i64().ok_or("StorageUnavailable：revision非i64")?,
            Value::String(s) => parse_canonical_i64(s, "revision")?,
            _ => return Err("StorageUnavailable：revision类型非法".into()),
        };
        let p:(String,String,String,String,String)=conn.query_row(
            "SELECT schema_revision,open,high,low,close FROM raw_ohlc WHERE identity_key=?1 AND revision=?2",
            params![row["identity_key"].as_str(),revision],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|e|format!("StorageUnavailable：OHLC原件缺失 {e}"))?;
        if p.0 != RAW_SCHEMA || row["price"] != p.4 {
            return Err("StorageUnavailable：OHLC schema/close索引矛盾".into());
        }
        row["schema_revision"] = json!(p.0);
        row["open"] = json!(p.1);
        row["high"] = json!(p.2);
        row["low"] = json!(p.3);
        row["close"] = json!(p.4);
    }
    Ok(())
}
pub(super) fn stored_prices(row: &Value) -> Result<Option<Ohlc>, String> {
    match row.get("schema_revision") {
        None => Ok(None),
        Some(v) if v == RAW_SCHEMA => Ok(Some(Ohlc {
            open: text(row, "open")?.into(),
            high: text(row, "high")?.into(),
            low: text(row, "low")?.into(),
            close: text(row, "close")?.into(),
        })),
        _ => Err("StorageUnavailable：未知原始schema".into()),
    }
}
pub(super) fn raw_wire(wire: &mut Value, raw: &Value) {
    if raw["schema_revision"] == RAW_SCHEMA {
        wire.as_object_mut().unwrap().remove("price");
        for key in ["schema_revision", "open", "high", "low", "close"] {
            wire[key] = raw[key].clone();
        }
    }
}

pub(super) fn typed(o: &Value) -> bool {
    o.get("fact_key").is_some()
}
pub(super) fn project(o: &Value) -> Result<Value, String> {
    let kind = o["kind"]
        .as_str()
        .ok_or("StorageUnavailable：事实kind缺失")?;
    if !matches!(
        kind,
        "CC-004.inclusion_step"
            | "CC-005.inclusion_group"
            | "CC-007.fractal_description"
            | "CC-054.knowledge_state"
    ) {
        return Err("StorageUnavailable：未知结构事实kind".into());
    }
    let mut result = json!({"object_id":o["object_id"],"object_revision":num_to_str(&o["object_revision"])?,"kind":kind,
        "batch_id":o.get("batch_id").cloned().unwrap_or(Value::Null),"fact_key":o["fact_key"],
        "payload":json_shape(o["payload_json"].as_str().ok_or("StorageUnavailable：事实payload缺失")?,JsonShape::Object)?,
        "input_refs":json_shape(o["input_refs_json"].as_str().ok_or("StorageUnavailable：事实来源缺失")?,JsonShape::Array)?,
        "source_coords":json_shape(o["source_coords_json"].as_str().ok_or("StorageUnavailable：事实坐标缺失")?,JsonShape::Array)?,
        "first_known_generation":num_to_str(&o["first_known_generation"])?,"first_known_cut":o["first_known_cut"],
        "published_generation":num_to_str(&o["published_generation"])?,"withdrawn_generation":null,
        "withdrawal_reason":o.get("withdrawal_reason").cloned().unwrap_or(Value::Null),"superseded_by":o.get("superseded_by").cloned().unwrap_or(Value::Null),"lifecycle":"active"});
    if !o["withdrawn_generation"].is_null() {
        result["withdrawn_generation"] = num_to_str(&o["withdrawn_generation"])?;
        result["lifecycle"] = json!("withdrawn");
    }
    Ok(result)
}
pub(super) fn save_fact(conn: &Connection, o: &Value, batch: &str) -> Result<(), String> {
    conn.execute("INSERT INTO structure_facts(object_id,object_revision,kind,batch_id,fact_key,payload_json,input_refs_json,source_coords_json,first_known_generation,first_known_cut,published_generation) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(object_id) DO NOTHING",
        params![o["object_id"].as_str(),o["object_revision"].as_i64(),o["kind"].as_str(),batch,o["fact_key"].as_str(),o["payload_json"].as_str(),o["input_refs_json"].as_str(),o["source_coords_json"].as_str(),o["first_known_generation"].as_i64(),o["first_known_cut"].as_str(),o["published_generation"].as_i64()]).map_err(|e|format!("持久结构事实失败：{e}"))?;
    Ok(())
}
pub(super) fn read_facts(
    conn: &Connection,
    as_of: Option<i64>,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    if !enabled(conn)? {
        return Ok((vec![], vec![]));
    }
    let mut stmt=conn.prepare("SELECT object_id,object_revision,kind,batch_id,fact_key,payload_json,input_refs_json,source_coords_json,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM structure_facts ORDER BY kind,fact_key,object_id,object_revision").map_err(|e|e.to_string())?;
    let rows=stmt.query_map([],|r|Ok(json!({"object_id":r.get::<_,String>(0)?,"object_revision":r.get::<_,i64>(1)?,"kind":r.get::<_,String>(2)?,"batch_id":r.get::<_,String>(3)?,"fact_key":r.get::<_,String>(4)?,"payload_json":r.get::<_,String>(5)?,"input_refs_json":r.get::<_,String>(6)?,"source_coords_json":r.get::<_,String>(7)?,"first_known_generation":r.get::<_,i64>(8)?,"first_known_cut":r.get::<_,String>(9)?,"published_generation":r.get::<_,i64>(10)?,"withdrawn_generation":r.get::<_,Option<i64>>(11)?,"withdrawal_reason":r.get::<_,Option<String>>(12)?,"superseded_by":r.get::<_,Option<String>>(13)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mut active = vec![];
    let mut withdrawn = vec![];
    for mut o in rows {
        let first = o["first_known_generation"].as_i64().unwrap();
        let death = o["withdrawn_generation"].as_i64();
        if death.is_some_and(|w| w <= first) {
            return Err("StorageUnavailable：结构事实生命周期非法".into());
        }
        if as_of.is_some_and(|n| first > n) {
            continue;
        }
        if matches!((as_of,death),(Some(n),Some(w)) if w>n) {
            for k in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                o[k] = Value::Null;
            }
        }
        if o["withdrawn_generation"].is_null() {
            active.push(o);
        } else {
            withdrawn.push(o);
        }
    }
    Ok((active, withdrawn))
}
pub(super) fn withdrawal(o: &Value, reason: &str, sup: Value) -> Value {
    if typed(o) {
        json!({"object_id":o["object_id"],"fact_key":o["fact_key"],"reason":reason,"superseded_by":sup})
    } else {
        json!({"object_id":o["object_id"],"window_start":o["window_start"],"window_mid":o["window_mid"],"window_end":o["window_end"],"reason":reason,"superseded_by":sup})
    }
}
pub(super) fn slot(o: &Value) -> String {
    if typed(o) {
        canonical_json(&json!([1, o["kind"], o["fact_key"]]))
    } else {
        canonical_json(&json!([
            0,
            o["window_start"],
            o["window_mid"],
            o["window_end"]
        ]))
    }
}
