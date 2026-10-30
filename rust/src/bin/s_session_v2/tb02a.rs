//! #1373：OHLC 输入、同核包含事实的持久投影。所有结构判断来自 parser。
pub(crate) const FACTS_READ_SQL: &str = "SELECT object_id,object_revision,kind,batch_id,fact_key,payload_json,input_refs_json,source_coords_json,first_known_generation,first_known_cut,published_generation,withdrawn_generation,withdrawal_reason,superseded_by FROM structure_facts ORDER BY kind,fact_key,object_id,object_revision";

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

use newchan_rust::s_content::capture_allocation::audit_json::{ArrayIter, ArrayView, JsonView};
const INPUT_FIELDS: &[&str] = &[
    "schema_revision",
    "session_id",
    "source_namespace",
    "source_epoch",
    "instrument",
    "profile",
    "events",
];
const TICK_FIELDS: &[&str] = &[
    "event_id",
    "revision",
    "seq",
    "received_at",
    "raw_text",
    "price",
    "timestamp",
    "volume",
];
const OHLC_FIELDS: &[&str] = &[
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
];

/// 借用原输入完整事件序列；字段私有，构造不授予接受/单事件资格。
pub(super) struct RawInputView<'a> {
    pub schema_revision: &'a str,
    pub session_id: &'a str,
    pub source_namespace: &'a str,
    pub source_epoch: &'a str,
    pub instrument: &'a str,
    pub profile: &'a str,
    events: ArrayView<'a>,
    ohlc: bool,
}
#[derive(Clone, Copy)]
pub(super) struct RawEventView<'a> {
    pub event_id: &'a str,
    pub revision: &'a str,
    pub seq: &'a str,
    pub received_at: &'a str,
    pub raw_text: &'a str,
    pub price: &'a str,
    pub timestamp: &'a str,
    pub volume: &'a str,
    pub ohlc: Option<OhlcView<'a>>,
}
impl RawInputView<'_> {
    pub fn events_len(&self) -> usize {
        self.events.len()
    }
    pub fn events(&self) -> RawEventIter<'_> {
        RawEventIter {
            iter: self.events.iter(),
            ohlc: self.ohlc,
        }
    }
    pub fn event(&self, index: usize) -> Result<RawEventView<'_>, String> {
        event_view(
            self.events
                .get(index)
                .ok_or("SchemaUnsupported：events索引越界")?,
            self.ohlc,
        )
    }
}
pub(super) struct RawEventIter<'a> {
    iter: ArrayIter<'a>,
    ohlc: bool,
}
impl<'a> Iterator for RawEventIter<'a> {
    type Item = Result<RawEventView<'a>, String>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|v| event_view(v, self.ohlc))
    }
}
/// 与普通parse_input共核；先逐条校完全部事件，再返借用view，不能提前取第一条放行。
pub(super) fn parse_input_view(value: JsonView<'_>) -> Result<RawInputView<'_>, String> {
    exact_keys_view(value, INPUT_FIELDS)?;
    for key in &INPUT_FIELDS[..6] {
        input_text(value, key)?;
    }
    let schema = input_text(value, "schema_revision")?;
    let profile = input_text(value, "profile")?;
    let ohlc = match (schema, profile) {
        ("1", "testonly_tick_1_1_ohlc") => false,
        (RAW_SCHEMA, PROFILE) => true,
        _ => return Err("SchemaUnsupported：schema_revision/profile组合未声明".into()),
    };
    let events = value
        .get("events")
        .and_then(JsonView::as_array)
        .ok_or("SchemaUnsupported：events必须数组")?;
    for event in events.iter() {
        event_view(event, ohlc)?;
    }
    Ok(RawInputView {
        schema_revision: schema,
        session_id: input_text(value, "session_id")?,
        source_namespace: input_text(value, "source_namespace")?,
        source_epoch: input_text(value, "source_epoch")?,
        instrument: input_text(value, "instrument")?,
        profile,
        events,
        ohlc,
    })
}
fn event_view(value: JsonView<'_>, ohlc: bool) -> Result<RawEventView<'_>, String> {
    let keys = if ohlc { OHLC_FIELDS } else { TICK_FIELDS };
    exact_keys_view(value, keys)?;
    for key in keys {
        input_text(value, key)?;
    }
    let prices = if ohlc {
        Some(OhlcView {
            open: input_text(value, "open")?,
            high: input_text(value, "high")?,
            low: input_text(value, "low")?,
            close: input_text(value, "close")?,
        })
    } else {
        None
    };
    let event = RawEventView {
        event_id: input_text(value, "event_id")?,
        revision: input_text(value, "revision")?,
        seq: input_text(value, "seq")?,
        received_at: input_text(value, "received_at")?,
        raw_text: input_text(value, "raw_text")?,
        price: if let Some(p) = prices {
            p.close
        } else {
            input_text(value, "price")?
        },
        timestamp: input_text(value, "timestamp")?,
        volume: input_text(value, "volume")?,
        ohlc: prices,
    };
    validate_prices_view(event.price, event.volume, event.ohlc)?;
    Ok(event)
}
fn exact_keys_view(value: JsonView<'_>, keys: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("SchemaUnsupported：预期对象")?;
    if map.len() != keys.len() || map.keys().any(|k| !keys.contains(&k)) {
        return Err("SchemaUnsupported：字段缺失或多余".into());
    }
    Ok(())
}
fn input_text<'a>(value: JsonView<'a>, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(JsonView::as_str)
        .ok_or_else(|| format!("SchemaUnsupported：{key}必须文本"))
}
pub(super) fn parse_input(value: &Value) -> Result<RawInputFile, String> {
    let input = parse_input_view(value.into())?;
    let mut events = Vec::new();
    // 显式normal DTO适配；Owned控制消费者直接持上述view，不调用本String/Vec边界。
    for event in input.events() {
        let e = event?;
        events.push(RawEvent {
            event_id: e.event_id.into(),
            revision: e.revision.into(),
            seq: e.seq.into(),
            received_at: e.received_at.into(),
            raw_text: e.raw_text.into(),
            price: e.price.into(),
            timestamp: e.timestamp.into(),
            volume: e.volume.into(),
            ohlc: e.ohlc.map(|p| Ohlc {
                open: p.open.into(),
                high: p.high.into(),
                low: p.low.into(),
                close: p.close.into(),
            }),
        });
    }
    Ok(RawInputFile {
        schema_revision: input.schema_revision.into(),
        session_id: input.session_id.into(),
        source_namespace: input.source_namespace.into(),
        source_epoch: input.source_epoch.into(),
        instrument: input.instrument.into(),
        profile: input.profile.into(),
        events,
    })
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
    .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    Ok(())
}
pub(super) fn enrich_raw(conn: &Connection, rows: &mut [Value]) -> Result<(), String> {
    enrich_raw_until(conn, rows, None)
}

pub(super) fn enrich_raw_until(
    conn: &Connection,
    rows: &mut [Value],
    deadline: Option<std::time::Instant>,
) -> Result<(), String> {
    enrich_raw_until_measured(conn, rows, deadline, None)
}

pub(super) fn enrich_raw_until_measured(
    conn: &Connection,
    rows: &mut [Value],
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn crate::v3::audit_budget::Budget>,
) -> Result<(), String> {
    crate::check_read_deadline(deadline)?;
    if !prepare_raw_enrichment_admitted(conn, None)? {
        return Ok(());
    }
    for row in rows {
        crate::check_read_deadline(deadline)?;
        if let Some(budget) = budget {
            budget
                .record()
                .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
        }
        enrich_raw_row(conn, row)?;
    }
    Ok(())
}
/// cold读取先核OHLC总数；固定追加回读继续复用相同字段校验。
pub(super) fn prepare_raw_enrichment_admitted(
    conn: &Connection,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<bool, String> {
    let enabled = enabled_admitted(conn, work)?;
    if enabled {
        verify_raw_ohlc_cardinality(conn)?;
    }
    Ok(enabled)
}
/// profile比较借用原SQL文本，不创建未准入的String。
pub(super) fn enabled_admitted(
    conn: &Connection,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<bool, String> {
    if work.is_none() {
        return enabled(conn);
    }
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        conn,
        c"SELECT value FROM meta WHERE key = ?1",
    )
    .map_err(|e| {
        crate::v3::business_json::mixed_diagnostic(format_args!("读 meta.profile_id 失败：{e}"))
    })?;
    stmt.bind_text(1, "profile_id").map_err(|e| {
        crate::v3::business_json::mixed_diagnostic(format_args!("读 meta.profile_id 失败：{e}"))
    })?;
    let enabled = match stmt.next().map_err(|e| {
        crate::v3::business_json::mixed_diagnostic(format_args!("读 meta.profile_id 失败：{e}"))
    })? {
        Some(mut row) => {
            row.text(0).map_err(|e| {
                crate::v3::business_json::mixed_diagnostic(format_args!(
                    "读 meta.profile_id 失败：{e}"
                ))
            })? == PROFILE
        }
        None => false,
    };
    stmt.finalize().map_err(|e| {
        crate::v3::business_json::mixed_diagnostic(format_args!("读 meta.profile_id 失败：{e}"))
    })?;
    Ok(enabled)
}
/// 旧无scope变更入口：唯一借视图核固定None，再显式拒Native裸化。
pub(super) fn enrich_raw_row(conn: &Connection, row: &mut Value) -> Result<(), String> {
    let owner = enrich_raw_view_admitted(conn, (&*row).into(), true, None)?;
    match owner {
        v3::business_json::JsonOwner::Plain(value) => {
            *row = value.into_unscoped()?;
            Ok(())
        }
        #[cfg(feature = "s_allocation_json_testonly")]
        v3::business_json::JsonOwner::Owned(_) => Err("普通OHLC入口不可拆Owned".into()),
    }
}
/// 所有暂存文本持原Native许可；只把借视图交给当前scope拥有者构造核。
pub(super) fn enrich_raw_view_admitted(
    conn: &Connection,
    row: newchan_rust::s_content::capture_allocation::audit_json::JsonView<'_>,
    ohlc: bool,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
) -> Result<v3::business_json::JsonOwner, String> {
    use newchan_rust::s_content::capture_allocation::audit_json::JsonView;
    use newchan_rust::s_content::capture_allocation::native_owned::NativeValue;
    use v3::audit_store::{CodecRead, NativeAdmission};
    let admission = NativeAdmission(work);
    if !ohlc {
        return v3::business_json::json_from_view_admitted(&admission, row).map_err(newchan_rust::s_content::ContextError::into_message);
    }
    let revision = row
        .get("revision")
        .ok_or("StorageUnavailable：revision类型非法")?;
    let revision = if revision.kind()
        == newchan_rust::s_content::capture_allocation::audit_json::JsonKind::Number
    {
        revision
            .as_i64()
            .ok_or("StorageUnavailable：revision非i64")?
    } else if let Some(s) = revision.as_str() {
        parse_canonical_i64(s, "revision")?
    } else {
        return Err("StorageUnavailable：revision类型非法".into());
    };
    let missing = |e| {
        crate::v3::business_json::mixed_diagnostic(format_args!(
            "StorageUnavailable：OHLC原件缺失 {e}"
        ))
    };
    let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
        conn, c"SELECT schema_revision,open,high,low,close FROM raw_ohlc WHERE identity_key=?1 AND revision=?2",
    ).map_err(missing)?;
    match row.get("identity_key").and_then(|v| v.as_str()) {
        Some(identity) => stmt.bind_text(1, identity),
        None => stmt.bind_null(1),
    }
    .map_err(missing)?;
    stmt.bind_integer(2, revision).map_err(missing)?;
    let mut sql = stmt
        .next()
        .map_err(missing)?
        .ok_or("StorageUnavailable：OHLC原件缺失")?;
    if sql.text(0).map_err(missing)? != RAW_SCHEMA
        || row.get("price").and_then(|v| v.as_str()) != Some(sql.text(4).map_err(missing)?)
    {
        return Err("StorageUnavailable：OHLC schema/close索引矛盾".into());
    }
    let names = ["schema_revision", "open", "high", "low", "close"];
    let mut texts: [Option<NativeValue<String>>; 5] = std::array::from_fn(|_| None);
    for (column, text) in texts.iter_mut().enumerate() {
        *text = Some(
            admission
                .native_string(sql.text(column).map_err(missing)?)
                .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?,
        );
    }
    let mut fields = newchan_rust::s_content::CodecVec::default();
    let object = row.as_object().ok_or("StorageUnavailable：raw非对象")?;
    admission
        .native_codec_room(&mut fields, object.len().saturating_add(5))
        .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    for (key, value) in object.iter() {
        if !names.contains(&key) {
            fields
                .try_push((key, value))
                .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
        }
    }
    for (key, text) in names.iter().zip(texts.iter()) {
        fields
            .try_push((*key, JsonView::String(text.as_ref().unwrap().as_str())))
            .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    }
    let owner = v3::business_json::json_from_view_admitted(
        &admission,
        JsonView::object_fields(&fields)
            .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?,
    ).map_err(newchan_rust::s_content::ContextError::into_message)?;
    drop(fields);
    drop(sql);
    stmt.finalize().map_err(missing)?;
    Ok(owner)
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
pub(crate) fn checked_fact_kind(o: &Value) -> Result<&str, String> {
    let kind = o["kind"]
        .as_str()
        .ok_or("StorageUnavailable：事实kind缺失")?;
    if !matches!(
        kind,
        "CC-004.inclusion_step"
            | "CC-005.inclusion_group"
            | "CC-007.fractal_description"
            | "CC-054.knowledge_state"
    ) && !tb02b::KINDS.contains(&kind)
        && !tb02c::KINDS.contains(&kind)
    {
        return Err("StorageUnavailable：未知结构事实kind".into());
    }
    Ok(kind)
}
pub(super) fn project(o: &Value) -> Result<Value, String> {
    let kind = checked_fact_kind(o)?;
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
/// 结构事实的**唯一**写入入口。
///
/// - 同 ID 重复写只有**不可变内容逐字节一致**才算幂等；否则 `IdentityConflict`——不得用
///   `ON CONFLICT DO NOTHING` 把同 ID 异内容吞掉、留待将来的读审计发现。
/// - `evidence-graph/3`：三列大原件先进同一私有节点图（同一事务），行内三列只留空占位；
///   旧整块域与 `graph/2` 域照旧写行内原文。
pub(super) fn save_fact(conn: &Connection, o: &Value, batch: &str) -> Result<(), String> {
    let object_id = o["object_id"]
        .as_str()
        .ok_or("StorageUnavailable：事实身份缺失")?;
    let revision = o["object_revision"]
        .as_i64()
        .ok_or("StorageUnavailable：事实版本非精确整数")?;
    let kind = o["kind"]
        .as_str()
        .ok_or("StorageUnavailable：事实kind缺失")?;
    let fact_key = o["fact_key"]
        .as_str()
        .ok_or("StorageUnavailable：事实fact_key缺失")?;
    let payload = o["payload_json"]
        .as_str()
        .ok_or("StorageUnavailable：事实payload缺失")?;
    let refs = o["input_refs_json"]
        .as_str()
        .ok_or("StorageUnavailable：事实来源缺失")?;
    let coords = o["source_coords_json"]
        .as_str()
        .ok_or("StorageUnavailable：事实坐标缺失")?;
    let format = evidence_store::format(conn)?;
    format.writable()?;
    let root_key = evidence_store::fact_root_key(object_id);
    let staged = if format.records_in_graph() {
        Some(evidence_store::stage_complete(
            &evidence_store::fact_root_value(payload, refs, coords),
            &[],
        )?)
    } else {
        None
    };
    // 同 ID 已存在：只有不可变内容完全一致才幂等。
    let existing: Option<(i64, String, String, String, String, String)> = conn
        .query_row(
            "SELECT object_revision,kind,fact_key,payload_json,input_refs_json,source_coords_json
               FROM structure_facts WHERE object_id=?1",
            params![object_id],
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
        .optional()
        .map_err(|e| {
            crate::v3::business_json::mixed_diagnostic(format_args!("读既有结构事实失败：{e}"))
        })?;
    if let Some((old_revision, old_kind, old_key, old_payload, old_refs, old_coords)) = existing {
        if old_revision != revision || old_kind != kind || old_key != fact_key {
            return Err(format!(
                "IdentityConflict：结构事实 `{object_id}` 同 ID 异身份（既有 {old_revision}/{old_kind}/{old_key}，本次 {revision}/{kind}/{fact_key}），拒绝写入"
            ));
        }
        let identical = match &staged {
            // 图域：三列原件由同名私有根承载，比较根的内容寻址身份（节点 id/槽/长度/摘要）。
            Some(staged) => match evidence_store::Reader::root_row(conn, &root_key)? {
                Some((node, slots, len, hash)) => {
                    node == staged.root
                        && slots == staged.slot_ids
                        && len == staged.expanded_length
                        && hash == staged.content_sha256
                }
                None => false,
            },
            // 整块域：行内原文逐字节比较。
            None => old_payload == payload && old_refs == refs && old_coords == coords,
        };
        if !identical {
            return Err(format!(
                "IdentityConflict：结构事实 `{object_id}` 同 ID 异内容，拒绝写入（不吞差异）"
            ));
        }
        return Ok(());
    }
    // 新事实：图域先落不可变节点与根（同一事务），再落行；行内三列留空占位。
    let (stored_payload, stored_refs, stored_coords) = match &staged {
        Some(staged) => {
            evidence_store::publish_refs(conn, &root_key, staged)?;
            (String::new(), String::new(), String::new())
        }
        None => (payload.to_string(), refs.to_string(), coords.to_string()),
    };
    conn.execute("INSERT INTO structure_facts(object_id,object_revision,kind,batch_id,fact_key,payload_json,input_refs_json,source_coords_json,first_known_generation,first_known_cut,published_generation) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![object_id,revision,kind,batch,fact_key,stored_payload,stored_refs,stored_coords,o["first_known_generation"].as_i64(),o["first_known_cut"].as_str(),o["published_generation"].as_i64()]).map_err(|e|crate::v3::business_json::mixed_diagnostic(format_args!("持久结构事实失败：{e}")))?;
    if staged.is_some() {
        // 写后立刻重读核：行内必须是空占位，且私有根可无损展开成同一份原件。
        let mut reader = evidence_store::Reader::new(conn, crate::EVIDENCE_READ_BUDGET);
        let bytes = reader.read_fixed_cut(&root_key)?;
        let (p, r, c) = evidence_store::split_fact_root(&bytes, &format!("事实 `{object_id}`"))?;
        if p != payload || r != refs || c != coords {
            return Err(format!(
                "IdentityConflict：结构事实 `{object_id}` 私有根展开与本次原件不符，拒绝发布"
            ));
        }
    }
    Ok(())
}
pub(crate) enum FactLifecycle {
    Skip,
    Keep,
    ClearDeath,
}
/// 只在全部SQL行与图原件替换完成后调用；不解析三列JSON。
pub(crate) fn fact_lifecycle(o: &Value, as_of: Option<i64>) -> Result<FactLifecycle, String> {
    let first = o["first_known_generation"].as_i64().unwrap();
    let death = o["withdrawn_generation"].as_i64();
    if death.is_some_and(|w| w <= first) {
        return Err("StorageUnavailable：结构事实生命周期非法".into());
    }
    if as_of.is_some_and(|n| first > n) {
        return Ok(FactLifecycle::Skip);
    }
    if matches!((as_of,death),(Some(n),Some(w)) if w>n) {
        return Ok(FactLifecycle::ClearDeath);
    }
    Ok(FactLifecycle::Keep)
}
pub(crate) fn collect_fact_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(
        json!({"object_id":r.get::<_,String>(0)?,"object_revision":r.get::<_,i64>(1)?,"kind":r.get::<_,String>(2)?,"batch_id":r.get::<_,String>(3)?,"fact_key":r.get::<_,String>(4)?,"payload_json":r.get::<_,String>(5)?,"input_refs_json":r.get::<_,String>(6)?,"source_coords_json":r.get::<_,String>(7)?,"first_known_generation":r.get::<_,i64>(8)?,"first_known_cut":r.get::<_,String>(9)?,"published_generation":r.get::<_,i64>(10)?,"withdrawn_generation":r.get::<_,Option<i64>>(11)?,"withdrawal_reason":r.get::<_,Option<String>>(12)?,"superseded_by":r.get::<_,Option<String>>(13)?}),
    )
}
pub(super) fn read_facts(
    conn: &Connection,
    as_of: Option<i64>,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    if !enabled(conn)? {
        return Ok((vec![], vec![]));
    }
    // #1394 第二段：三列大原件由该事实的私有根承载（graph/3）；整块域取行内原文。
    // 同一次读取共用一个捕获作用域与读取器，唯一节点原行只读一次。
    let format = evidence_store::format(conn)?;
    let mut reader = evidence_store::Reader::new(conn, crate::EVIDENCE_READ_BUDGET);
    let mut stmt = conn
        .prepare(FACTS_READ_SQL)
        .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    let rows = stmt
        .query_map([], collect_fact_row)
        .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    let mut rows = rows;
    for o in &mut rows {
        let object_id = o["object_id"].as_str().unwrap_or("").to_string();
        if !format.records_in_graph() {
            continue;
        }
        for (column, label) in [
            ("payload_json", "payload"),
            ("input_refs_json", "input_refs"),
            ("source_coords_json", "source_coords"),
        ] {
            let text = o[column].as_str().unwrap_or("").to_string();
            if !text.is_empty() {
                return Err(format!(
                    "StorageUnavailable：图域结构事实 `{object_id}` 行内 {label} 仍带整块原文，拒绝"
                ));
            }
        }
        let bytes = reader.read_fixed_cut(&evidence_store::fact_root_key(&object_id))?;
        let (payload, refs, coords) =
            evidence_store::split_fact_root(&bytes, &format!("事实 `{object_id}`"))?;
        o["payload_json"] = json!(payload);
        o["input_refs_json"] = json!(refs);
        o["source_coords_json"] = json!(coords);
    }
    let mut active = vec![];
    let mut withdrawn = vec![];
    for mut o in rows {
        match fact_lifecycle(&o, as_of)? {
            FactLifecycle::Skip => continue,
            FactLifecycle::Keep => {}
            FactLifecycle::ClearDeath => {
                for k in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                    o[k] = Value::Null;
                }
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

/// 普通与拥有态共用借用价格判据，不复制价格文本。
#[derive(Clone, Copy)]
pub(super) struct OhlcView<'a> {
    pub open: &'a str,
    pub high: &'a str,
    pub low: &'a str,
    pub close: &'a str,
}
pub(super) fn stored_prices_view(
    row: newchan_rust::s_content::capture_allocation::audit_json::JsonView<'_>,
) -> Result<Option<OhlcView<'_>>, String> {
    match row.get("schema_revision") {
        None => Ok(None),
        Some(v) if v.as_str() == Some(RAW_SCHEMA) => {
            let text = |k| {
                row.get(k)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("SchemaUnsupported：{k}必须文本"))
            };
            Ok(Some(OhlcView {
                open: text("open")?,
                high: text("high")?,
                low: text("low")?,
                close: text("close")?,
            }))
        }
        _ => Err("StorageUnavailable：未知原始schema".into()),
    }
}
pub(super) fn validate_prices_view(
    price: &str,
    volume: &str,
    p: Option<OhlcView<'_>>,
) -> Result<(), String> {
    if let Some(p) = p {
        let o = parse_canonical_i64(p.open, "open")?;
        let h = parse_canonical_i64(p.high, "high")?;
        let l = parse_canonical_i64(p.low, "low")?;
        let c = parse_canonical_i64(p.close, "close")?;
        if l > o || o > h || l > c || c > h || price != p.close {
            return Err("InvalidDomain：OHLC几何或close索引不一致".into());
        }
        if parse_canonical_i64(volume, "volume")? < 0 {
            return Err("InvalidDomain：volume为负".into());
        }
    }
    Ok(())
}
#[cfg(feature = "s_allocation_json_testonly")]
pub(super) fn enrich_raw_owned(
    conn: &Connection,
    tree: &mut newchan_rust::s_content::capture_allocation::audit_json::OwnedJson,
    deadline: Option<std::time::Instant>,
) -> Result<(), String> {
    crate::check_read_deadline(deadline)?;
    if !enabled(conn)? {
        return Ok(());
    }
    verify_raw_ohlc_cardinality(conn)?;
    let mut stmt=conn.prepare("SELECT schema_revision,open,high,low,close FROM raw_ohlc WHERE identity_key=?1 AND revision=?2").map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    for i in 0..tree.view().as_array().unwrap().len() {
        crate::check_read_deadline(deadline)?;
        let event = tree.view().index(i).unwrap();
        let revision = event
            .get("revision")
            .and_then(|v| v.as_i64())
            .ok_or("StorageUnavailable：revision非i64")?;
        let mut rows = stmt
            .query(params![
                event.get("identity_key").and_then(|v| v.as_str()),
                revision
            ])
            .map_err(|e| {
                crate::v3::business_json::mixed_diagnostic(format_args!(
                    "StorageUnavailable：OHLC原件缺失 {e}"
                ))
            })?;
        let row = rows
            .next()
            .map_err(|e| {
                crate::v3::business_json::mixed_diagnostic(format_args!(
                    "StorageUnavailable：OHLC原件缺失 {e}"
                ))
            })?
            .ok_or("StorageUnavailable：OHLC原件缺失")?;
        let get = |i| -> Result<&str, String> {
            row.get_ref(i)
                .map_err(|e| {
                    crate::v3::business_json::mixed_diagnostic(format_args!(
                        "StorageUnavailable：OHLC原件缺失 {e}"
                    ))
                })?
                .as_str()
                .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))
        };
        if get(0)? != RAW_SCHEMA || event.get("price").and_then(|v| v.as_str()) != Some(get(4)?) {
            return Err("StorageUnavailable：OHLC schema/close索引矛盾".into());
        }
        let mut object = tree
            .object_at_mut(i)
            .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
        for (i, k) in [
            (0, "schema_revision"),
            (1, "open"),
            (2, "high"),
            (3, "low"),
            (4, "close"),
        ] {
            object
                .insert_str(k, get(i)?)
                .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
        }
    }
    Ok(())
}
fn verify_raw_ohlc_cardinality(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_ohlc", [], |r| r.get(0))
        .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    let raw_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .map_err(|e| crate::v3::business_json::mixed_diagnostic(format_args!("{e}")))?;
    if count != raw_count {
        return Err("StorageUnavailable：raw_ohlc与原始账不一一对应".into());
    }
    Ok(())
}

#[cfg(test)]
mod input_view_tests {
    use super::*;
    fn input(ohlc: bool) -> Value {
        let event = if ohlc {
            json!({"event_id":"e1","revision":"1","seq":"0","received_at":"r","raw_text":"汉\"😀\n","open":"-2","high":"4","low":"-3","close":"3","timestamp":"t","volume":"0"})
        } else {
            json!({"event_id":"e1","revision":"not-parsed-here","seq":"not-parsed-here","received_at":"r","raw_text":"汉\"😀\n","price":"legacy-not-parsed-here","timestamp":"t","volume":"-1"})
        };
        json!({"schema_revision":if ohlc {RAW_SCHEMA}else{"1"},"session_id":"s","source_namespace":"n","source_epoch":"e","instrument":"i","profile":if ohlc {PROFILE}else{"testonly_tick_1_1_ohlc"},"events":[event]})
    }
    #[test]
    fn borrowed_raw_input_matches_normal_dto_all_fields_without_tightening_tick() {
        for ohlc in [false, true] {
            let v = input(ohlc);
            let normal = parse_input(&v).unwrap();
            let view = parse_input_view((&v).into()).unwrap();
            assert_eq!(view.schema_revision, normal.schema_revision);
            assert_eq!(view.session_id, normal.session_id);
            assert_eq!(view.source_namespace, normal.source_namespace);
            assert_eq!(view.source_epoch, normal.source_epoch);
            assert_eq!(view.instrument, normal.instrument);
            assert_eq!(view.profile, normal.profile);
            assert_eq!(view.events_len(), normal.events.len());
            for (borrowed, plain) in view.events().zip(&normal.events) {
                let b = borrowed.unwrap();
                assert_eq!(
                    (
                        b.event_id,
                        b.revision,
                        b.seq,
                        b.received_at,
                        b.raw_text,
                        b.price,
                        b.timestamp,
                        b.volume
                    ),
                    (
                        plain.event_id.as_str(),
                        plain.revision.as_str(),
                        plain.seq.as_str(),
                        plain.received_at.as_str(),
                        plain.raw_text.as_str(),
                        plain.price.as_str(),
                        plain.timestamp.as_str(),
                        plain.volume.as_str()
                    )
                );
                match (b.ohlc, &plain.ohlc) {
                    (Some(b), Some(p)) => assert_eq!(
                        (b.open, b.high, b.low, b.close),
                        (
                            p.open.as_str(),
                            p.high.as_str(),
                            p.low.as_str(),
                            p.close.as_str()
                        )
                    ),
                    (None, None) => {}
                    _ => panic!("OHLC shape mismatch"),
                }
            }
        }
    }
    #[test]
    fn borrowed_raw_input_checks_bad_later_event_before_single_event_consumer_gate() {
        for (field, value, error) in [
            ("open", json!("01"), "不是规范十进制整数"),
            ("volume", json!("-1"), "volume为负"),
            ("raw_text", json!(true), "raw_text必须文本"),
        ] {
            let mut v = input(true);
            let mut second = v["events"][0].clone();
            second[field] = value;
            v["events"].as_array_mut().unwrap().push(second);
            let expected = parse_input(&v).unwrap_err();
            let actual = parse_input_view((&v).into()).err().unwrap();
            assert_eq!(actual, expected);
            assert!(actual.contains(error));
        }
        let mut v = input(true);
        v["events"][0]["open"] = json!("01");
        v["events"][0]["volume"] = json!("-1");
        assert!(parse_input_view((&v).into())
            .err()
            .unwrap()
            .contains("open `01`"));
        let mut v = input(true);
        v["events"][0]["high"] = json!("-4");
        assert_eq!(
            parse_input_view((&v).into()).err().unwrap(),
            "InvalidDomain：OHLC几何或close索引不一致"
        );
        let mut v = input(true);
        v["events"].as_array_mut().unwrap().clear();
        assert_eq!(parse_input_view((&v).into()).unwrap().events_len(), 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_raw_input_borrows_arena_budget_then_drops_and_low_quota_refuses_parse() {
        use newchan_rust::s_content::capture_allocation::{audit_json::OwnedJson, OwnedBudget};
        let v = input(true);
        let bytes = canonical_json(&v);
        let b = OwnedBudget::new(100000);
        let owner = OwnedJson::parse(bytes.as_bytes(), &b).unwrap();
        let before = b.snapshot();
        {
            let input = parse_input_view(owner.view()).unwrap();
            assert_eq!(input.events_len(), 1);
            assert_eq!(input.event(0).unwrap().price, "3");
            assert_eq!(b.snapshot().allocation_attempts, before.allocation_attempts);
        }
        drop(owner);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        let low = OwnedBudget::new(0);
        assert_eq!(
            OwnedJson::parse(bytes.as_bytes(), &low)
                .err()
                .unwrap()
                .code(),
            newchan_rust::s_content::capture_allocation::LIMIT
        );
        assert_eq!(low.snapshot().allocated_layouts, 0);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
        let mut v = input(true);
        let mut bad = v["events"][0].clone();
        bad["close"] = json!("9007199254740993");
        v["events"].as_array_mut().unwrap().push(bad);
        let owner = OwnedJson::parse(canonical_json(&v).as_bytes(), &b).unwrap();
        assert_eq!(
            parse_input_view(owner.view()).err().unwrap(),
            parse_input(&v).unwrap_err()
        );
        drop(owner);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
}
