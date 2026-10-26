//! #1527：控制表模式与换代历史的持费读取。
use super::*;
use crate::v3::{
    audit_budget::LegacyObserver, audit_store::NativeTree, business_json::MixedJsonError,
};
type Text = NativeValue<String>;
pub(crate) type EpochFacts = CodecVec<(TextValue, i64)>;
pub(crate) struct Schema {
    tables: NativeTree<Text, Text>,
}
struct Normalized<'a>(&'a str);
impl fmt::Display for Normalized<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        normalize_schema_chunks(self.0, |chunk| f.write_str(chunk))
    }
}
fn normalized(value: &str, work: &WorkScope) -> Result<Text, MixedJsonError> {
    Text::try_format(format_args!("{}", Normalized(value)), Some(work))
        .map_err(MixedJsonError::Resource)
}
fn sql_text(
    row: &mut CheckedRow<'_>,
    column: usize,
    work: &WorkScope,
) -> Result<Text, MixedJsonError> {
    Text::try_from_str(row.text(column).map_err(err)?, Some(work)).map_err(MixedJsonError::Resource)
}
impl Schema {
    fn new() -> Self {
        Self {
            tables: Default::default(),
        }
    }
    pub(crate) fn add(
        &mut self,
        label: &str,
        ddl: &str,
        work: &WorkScope,
    ) -> Result<(), MixedJsonError> {
        ddl_table_entries(label, ddl, |name, sql| {
            let key = Text::try_from_str(name, Some(work)).map_err(MixedJsonError::Resource)?;
            let value = normalized(sql, work)?;
            self.tables
                .insert_admitted(key, value, &NativeAdmission(Some(work)))
                .map_err(MixedJsonError::Resource)?;
            Ok(())
        })
    }
    pub(crate) fn verify(
        &self,
        conn: &Connection,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<(), MixedJsonError> {
        observer
            .check_work(work)
            .map_err(MixedJsonError::Resource)?;
        let mut stmt = CheckedStatement::prepare(
            conn,
            c"SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(err)?;
        let mut seen = crate::v3::audit_store::NativeSet::default();
        while let Some(mut row) = stmt.next().map_err(err)? {
            let kind = sql_text(&mut row, 0, work)?;
            let name = sql_text(&mut row, 1, work)?;
            let sql = sql_text(&mut row, 2, work)?;
            if kind.as_str() != "table" {
                return Err(err(format!(
                    "模式/约束不匹配 {}/{}",
                    kind.as_ref(),
                    name.as_ref()
                ))
                .into());
            }
            let value = normalized(&sql, work)?;
            if self.tables.get(name.as_str()).map(|v| v.as_str()) != Some(value.as_str()) {
                return Err(err(format!(
                    "模式/约束不匹配 {}/{}",
                    kind.as_ref(),
                    name.as_ref()
                ))
                .into());
            }
            seen.insert_admitted(name, &NativeAdmission(Some(work)))
                .map_err(MixedJsonError::Resource)?;
        }
        stmt.finalize().map_err(err)?;
        if seen.len() != self.tables.len() {
            return Err(err("缺必需表/约束").into());
        }
        for table in self.tables.keys() {
            let mut columns = CheckedStatement::prepare(
                conn,
                c"SELECT type,\"notnull\",pk FROM pragma_table_info(?1) ORDER BY cid",
            )
            .map_err(err)?;
            columns.bind_text(1, table).map_err(err)?;
            let mut declarations = CodecVec::default();
            while let Some(mut row) = columns.next().map_err(err)? {
                let item = (
                    sql_text(&mut row, 0, work)?,
                    row.integer(1).map_err(err)?,
                    row.integer(2).map_err(err)?,
                );
                declarations
                    .try_reserve_native(work, 1)
                    .map_err(MixedJsonError::Resource)?;
                declarations
                    .try_push(item)
                    .map_err(MixedJsonError::Resource)?;
            }
            columns.finalize().map_err(err)?;
            let sql = Text::try_format(
                format_args!("SELECT * FROM {}\0", table.as_ref()),
                Some(work),
            )
            .map_err(MixedJsonError::Resource)?;
            let sql = CStr::from_bytes_with_nul(sql.as_bytes()).map_err(err)?;
            let mut rows = CheckedStatement::prepare(conn, sql).map_err(err)?;
            while let Some(mut row) = rows.next().map_err(err)? {
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
                    if !valid_storage_cell(kind, *notnull, *pk, storage, text) {
                        return Err(err(format!(
                            "{} 第{i}列实际类型/非空约束损坏",
                            table.as_ref()
                        ))
                        .into());
                    }
                }
            }
            rows.finalize().map_err(err)?;
        }
        Ok(())
    }
}
pub(crate) fn verify_schema(
    conn: &Connection,
    enabled: bool,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<Schema, MixedJsonError> {
    let ohlc = tb02a::enabled_admitted(conn, Some(work))?;
    let mut expected = Schema::new();
    expected.add("schema常量", SCHEMA, work)?;
    if enabled {
        expected.add("schema常量", SCHEMA_V2, work)?;
    }
    if ohlc {
        expected.add("schema常量", tb02a::SCHEMA, work)?;
    }
    let delta: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
            params![delta_storage::TABLE],
            |r| r.get(0),
        )
        .map_err(err)?;
    if delta == 1 {
        expected.add("Delta DDL", delta_storage::DDL, work)?;
    }
    let graph: i64 = conn
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
    if graph != 0 {
        if graph != 3 {
            return Err(err("私有证据图表不齐全（三表必须同生）").into());
        }
        expected.add("证据图 DDL", evidence_store::DDL, work)?;
    }
    expected.verify(conn, work, observer)?;
    delta_mode(conn, work)?;
    evidence_store::read_format_typed(conn, None, Some(work))?;
    Ok(expected)
}
pub(crate) fn active(conn: &Connection, work: &WorkScope) -> Result<bool, MixedJsonError> {
    let marker = crate::audit::mixed_profile::meta_text(conn, "protocol_revision", work)?;
    let tables:i64=conn.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN ('s_protocol_meta','s_input_messages','s_clock_events','s_delivery_policy','s_delivery_refs')",[],|r|r.get(0)).map_err(err)?;
    match (marker.as_ref().map(|v| v.as_str()), tables) {
        (None, 0) => Ok(false),
        (Some("s-session/2"), 5) => Ok(true),
        _ => Err(err("协议标记/控制表缺失或不支持").into()),
    }
}
pub(crate) fn epoch_history(
    conn: &Connection,
    enabled: bool,
    work: &WorkScope,
) -> Result<EpochFacts, MixedJsonError> {
    let generation = crate::audit::mixed_profile::meta_text(conn, "generation", work)?
        .ok_or_else(|| "StorageUnavailable：meta.generation 缺失".to_string())?;
    let generation = parse_canonical_i64(&generation, "meta.generation")
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let current = crate::audit::mixed_profile::meta_text(conn, "writer_epoch", work)?
        .ok_or_else(|| err("缺writer_epoch"))?;
    let current = parse_writer_epoch(&current, "meta.writer_epoch").map_err(err)?;
    let mut query=CheckedStatement::prepare(conn,c"SELECT ordinal,from_epoch,to_epoch,generation_at_transition,advance_state_at_transition,transitioned_at FROM writer_epoch_history ORDER BY ordinal").map_err(err)?;
    let (mut previous_epoch, mut previous_generation, mut ordinal) = (None, 0, 0i64);
    let mut facts = CodecVec::default();
    while let Some(mut row) = query.next().map_err(err)? {
        ordinal += 1;
        let actual = row.integer(0).map_err(err)?;
        let from = parse_writer_epoch(&sql_text(&mut row, 1, work)?, "epoch.from").map_err(err)?;
        let to = parse_writer_epoch(&sql_text(&mut row, 2, work)?, "epoch.to").map_err(err)?;
        let at = nonnegative(&sql_text(&mut row, 3, work)?, "epoch.generation").map_err(err)?;
        let state = sql_text(&mut row, 4, work)?;
        let ns = sql_text(&mut row, 5, work)?;
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
            enabled,
        )?;
        previous_epoch = Some(to);
        previous_generation = at;
        facts
            .try_reserve_native(work, 1)
            .map_err(MixedJsonError::Resource)?;
        facts.try_push((TextValue::Plain(ns), at)).map_err(MixedJsonError::Resource)?;
    }
    query.finalize().map_err(err)?;
    if previous_epoch.is_some_and(|last| last != current) {
        return Err(err("writer_epoch与最后换代结果不一致").into());
    }
    Ok(facts)
}

#[cfg(test)]
pub(crate) fn original_schema(conn: &Connection, enabled: bool) -> Result<(), String> {
    super::verify_schema(conn, enabled)
}
#[cfg(test)]
pub(crate) fn original_active(conn: &Connection) -> Result<bool, String> {
    super::active(conn)
}

pub(crate) fn delta_mode(
    conn: &Connection,
    work: &WorkScope,
) -> Result<delta_storage::Mode, MixedJsonError> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name=?1",
            params![delta_storage::TABLE],
            |r| r.get(0),
        )
        .map_err(|e| format!("StorageUnavailable：读私有格式登记失败：{e}"))?;
    if exists == 0 {
        return Ok(delta_storage::Mode::LegacyFull);
    }
    let mut stmt = conn
        .prepare("SELECT singleton,storage_format FROM s_storage_format")
        .map_err(|e| format!("StorageUnavailable：私有格式登记结构不符：{e}"))?;
    let mut query = stmt
        .query([])
        .map_err(|e| format!("StorageUnavailable：读私有格式登记失败：{e}"))?;
    let mut rows = CodecVec::default();
    while let Some(row) = query
        .next()
        .map_err(|e| format!("StorageUnavailable：私有格式登记行损坏：{e}"))?
    {
        let n: i64 = row
            .get(0)
            .map_err(|e| format!("StorageUnavailable：私有格式登记行损坏：{e}"))?;
        let value = row
            .get_ref(1)
            .and_then(|v| {
                v.as_str().map_err(|e| match e {
                    rusqlite::types::FromSqlError::InvalidType => {
                        rusqlite::Error::InvalidColumnType(
                            1,
                            "storage_format".into(),
                            v.data_type(),
                        )
                    }
                    other => {
                        rusqlite::Error::FromSqlConversionFailure(1, v.data_type(), Box::new(other))
                    }
                })
            })
            .map_err(|e| format!("StorageUnavailable：私有格式登记行损坏：{e}"))?;
        let text = Text::try_from_str(value, Some(work)).map_err(MixedJsonError::Resource)?;
        rows.try_reserve_native(work, 1)
            .map_err(MixedJsonError::Resource)?;
        rows.try_push((n, text)).map_err(MixedJsonError::Resource)?;
    }
    delta_storage::validate_mode_rows(rows.len(), rows.first().map(|(n, s)| (*n, s.as_str())))
        .map_err(Into::into)
}

pub(crate) struct Clock {
    pub(crate) value: crate::v3::business_json::PlainJson,
    pub(crate) hash: Text,
}
impl Clock {
    pub(crate) fn phase(&self, id: &str, phase: &str, index: i64) -> Result<&str, String> {
        newchan_rust::session_protocol::clock_phase_view(
            self.value.as_ref().into(),
            id,
            phase,
            index,
        )
    }
}
fn decode(
    raw: &str,
    work: &WorkScope,
) -> Result<crate::v3::business_json::PlainJson, MixedJsonError> {
    crate::v3::business_json::mixed_strict_json_owner(raw.as_bytes(), work).map_err(|e| match e {
        MixedJsonError::Business(e) => MixedJsonError::Business(err(e)),
        e => e,
    })
}
pub(crate) fn clock(conn: &Connection, work: &WorkScope) -> Result<Clock, MixedJsonError> {
    let mut stmt = CheckedStatement::prepare(
        conn,
        c"SELECT clock_plan_json,clock_plan_hash FROM s_protocol_meta WHERE singleton=1",
    )
    .map_err(err)?;
    let mut row = stmt
        .next()
        .map_err(err)?
        .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
    let raw = sql_text(&mut row, 0, work)?;
    let stored_hash = sql_text(&mut row, 1, work)?;
    drop(row);
    stmt.finalize().map_err(err)?;
    let value = decode(&raw, work)?;
    let view =
        newchan_rust::session_protocol::ClockView::parse(value.as_ref().into()).map_err(err)?;
    let hash = view.hash_fixed_with_owner(None).map_err(err)?;
    if hash.as_str() != stored_hash.as_str()
        || !control_canonical_equal(value.as_ref().into(), &raw, None)?
    {
        return Err(err("clock计划/hash不一致").into());
    }
    let hash = Text::try_from_str(hash.as_str(), Some(work)).map_err(MixedJsonError::Resource)?;
    Ok(Clock { value, hash })
}
pub(crate) fn catalog(conn: &Connection, work: &WorkScope) -> Result<(), MixedJsonError> {
    use crate::audit::mixed_profile::meta_text;
    use control_json::field;
    use newchan_rust::session_protocol::{canonical_hash_view_with_owner, required_view};
    let raw = meta_text(conn, "catalog_definition", work)?.ok_or_else(|| err("v2缺catalog来源"))?;
    let hash = meta_text(conn, "catalog_hash", work)?.ok_or_else(|| err("v2缺catalog hash"))?;
    let value = decode(&raw, work)?;
    let view: JsonView<'_> = value.as_ref().into();
    if !control_canonical_equal(view, &raw, None)?
        || canonical_hash_view_with_owner(view, None)
            .map_err(err)?
            .as_str()
            != hash.as_str()
        || field(view, "catalog_revision").as_str()
            != meta_text(conn, "catalog_revision", work)?
                .as_ref()
                .map(|s| s.as_str())
    {
        return Err(err("目录来源/hash/revision不一致").into());
    }
    let items = field(view, "items")
        .as_array()
        .ok_or_else(|| err("目录源items缺失"))?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM catalog", [], |r| r.get(0))
        .map_err(err)?;
    if count as usize != items.len() {
        return Err(err("目录静态行数不同").into());
    }
    for item in items.iter() {
        let mut stmt = CheckedStatement::prepare(
            conn,
            c"SELECT kind,title,domain,branches_json FROM catalog WHERE catalog_id=?1",
        )
        .map_err(err)?;
        stmt.bind_text(1, required_view(item, "id").map_err(err)?)
            .map_err(err)?;
        let mut row = stmt
            .next()
            .map_err(err)?
            .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
        let actual = [
            sql_text(&mut row, 0, work)?,
            sql_text(&mut row, 1, work)?,
            sql_text(&mut row, 2, work)?,
            sql_text(&mut row, 3, work)?,
        ];
        drop(row);
        stmt.finalize().map_err(err)?;
        if field(item, "kind").as_str() != Some(actual[0].as_str())
            || field(item, "title").as_str() != Some(actual[1].as_str())
            || field(item, "domain").as_str() != Some(actual[2].as_str())
            || field(item, "branches") != JsonView::from(decode(&actual[3], work)?.as_ref())
        {
            return Err(err("目录静态内容与来源不同").into());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn original_clock(conn: &Connection) -> Result<String, String> {
    super::stored_clock(conn).map(|c| c.hash)
}

#[cfg(test)]
pub(crate) fn original_catalog(conn: &Connection) -> Result<(), String> {
    super::verify_control_catalog_admitted(conn, None, None)
}

pub(crate) mod history;

pub(crate) fn prepare(conn:&Connection,work:&WorkScope,observer:&dyn LegacyObserver)->Result<ExpectedControl,MixedJsonError>{
    let enabled=active(conn,work)?;
    verify_schema(conn,enabled,work,observer)?;
    let facts=epoch_history(conn,enabled,work)?;
    if !enabled{return Ok(ExpectedControl::Legacy);}
    Ok(ExpectedControl::V2{history:history::validate(conn,"s-session/2",facts,work)?})
}
