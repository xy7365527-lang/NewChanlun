//! #1527：mixed旧审计的持费索引；观察者仅由本次调用借入，不进入缓存。
use super::*;
use crate::v3::{
    audit_budget::{LegacyObserver, LegacyRecordRef},
    audit_store::{NativeAdmission, NativeSet, NativeTree},
    business_json::{scoped_legacy, MixedJsonError, PlainJson},
    native_shared::NativeArc,
};
use newchan_rust::s_content::capture_allocation::{native_owned::NativeValue, scratch::WorkScope};

type Text = NativeValue<String>;
type Shared = NativeArc<PlainJson>;
pub(crate) struct Family {
    name: &'static str,
    records: NativeTree<Text, (Shared, Text, i64)>,
    born: NativeTree<i64, NativeSet<Text>>,
    seen: NativeSet<Text>,
}
fn sql_error(error: scoped_legacy::MixedSqlError) -> MixedJsonError {
    match error {
        scoped_legacy::MixedSqlError::Sql(error) => MixedJsonError::Business(error.to_string()),
        scoped_legacy::MixedSqlError::Projection(error) => error,
    }
}
fn identity(value: &Value, family: &str, work: &WorkScope) -> Result<Text, MixedJsonError> {
    if family == "relations" {
        return Text::try_format(format_args!("{value}"), Some(work))
            .map_err(MixedJsonError::Resource);
    }
    let field = match family {
        "witnesses" => "witness_id",
        "observations" => "observation_id",
        _ => "object_id",
    };
    Text::try_from_str(
        value[field]
            .as_str()
            .ok_or("StorageUnavailable：索引身份缺失")?,
        Some(work),
    )
    .map_err(MixedJsonError::Resource)
}
impl Family {
    pub(crate) fn capture(
        conn: &Connection,
        name: &'static str,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<Self, MixedJsonError> {
        observer
            .check_work(work)
            .map_err(MixedJsonError::Resource)?;
        let sql = match name {
            "witnesses" => WITNESSES_SQL,
            "relations" => RELATIONS_SQL,
            "observations" => OBSERVATIONS_SQL,
            _ => return Err("StorageUnavailable：未知索引族".into()),
        };
        let mut result = Self {
            name,
            records: Default::default(),
            born: Default::default(),
            seen: Default::default(),
        };
        let mut query = conn
            .prepare(sql)
            .map_err(|e| MixedJsonError::Business(e.to_string()))?;
        let mut rows = query
            .query([])
            .map_err(|e| MixedJsonError::Business(e.to_string()))?;
        let admission = NativeAdmission(Some(work));
        while let Some(row) = rows
            .next()
            .map_err(|e| MixedJsonError::Business(e.to_string()))?
        {
            let (id, g, record) =
                scoped_legacy::project_family_row(row, name, work).map_err(sql_error)?;
            observer
                .visit_record(name, "index-capture", LegacyRecordRef::Json(&record))
                .map_err(MixedJsonError::Resource)?;
            if identity(&record, name, work)? != id {
                return Err(
                    format!("StorageUnavailable：{name} 独立索引身份与其记录不一致").into(),
                );
            }
            if result.records.contains_key(id.as_ref().as_str()) {
                return Err(format!("StorageUnavailable：{name} 重复身份").into());
            }
            if !result.born.contains_key(&g) {
                result
                    .born
                    .insert_admitted(g, NativeSet::default(), &admission)
                    .map_err(MixedJsonError::Resource)?;
            }
            result
                .born
                .get_mut(&g)
                .unwrap()
                .insert_admitted(
                    id.try_clone_admitted(work)
                        .map_err(MixedJsonError::Resource)?,
                    &admission,
                )
                .map_err(MixedJsonError::Resource)?;
            let projected = scoped_legacy::legacy_payload(&record, work)?;
            let canonical = Text::try_format(format_args!("{}", projected.as_ref()), Some(work))
                .map_err(MixedJsonError::Resource)?;
            drop(projected);
            let record =
                NativeArc::try_new_shallow(record, Some(work)).map_err(MixedJsonError::Resource)?;
            result
                .records
                .insert_admitted(id, (record, canonical, g), &admission)
                .map_err(MixedJsonError::Resource)?;
        }
        Ok(result)
    }
    pub(crate) fn verify<'a>(
        &mut self,
        delta: &Value,
        g: i64,
        mode: delta_storage::Mode,
        batch_records: impl IntoIterator<Item = &'a Value>,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<(), MixedJsonError> {
        observer
            .check_work(work)
            .map_err(MixedJsonError::Resource)?;
        let admission = NativeAdmission(Some(work));
        let mut present = NativeSet::default();
        for record in required_array(delta, self.name)? {
            observer
                .visit_record(self.name, "family-delta", LegacyRecordRef::Json(record))
                .map_err(MixedJsonError::Resource)?;
            let id = identity(record, self.name, work)?;
            if !present
                .insert_admitted(
                    id.try_clone_admitted(work)
                        .map_err(MixedJsonError::Resource)?,
                    &admission,
                )
                .map_err(MixedJsonError::Resource)?
            {
                return Err(format!("StorageUnavailable：{} Delta 重复身份", self.name).into());
            }
            let (stored, canonical, born) = self
                .records
                .get(id.as_ref().as_str())
                .ok_or("StorageUnavailable：Delta 没有独立索引端点")?;
            observer
                .visit_record(
                    self.name,
                    "family-delta-index",
                    LegacyRecordRef::Json(stored.as_ref()),
                )
                .map_err(MixedJsonError::Resource)?;
            let projected = scoped_legacy::legacy_payload(record, work)?;
            let actual = Text::try_format(format_args!("{}", projected.as_ref()), Some(work))
                .map_err(MixedJsonError::Resource)?;
            if *canonical != actual
                || *born > g
                || (!self.seen.contains(id.as_ref().as_str()) && *born != g)
            {
                return Err(
                    format!("StorageUnavailable：{} 内容/首次发布代不一致", self.name).into(),
                );
            }
            self.seen
                .insert_admitted(id, &admission)
                .map_err(MixedJsonError::Resource)?;
        }
        let mut batch_ids = NativeSet::default();
        for record in batch_records {
            observer
                .visit_record(self.name, "family-batch", LegacyRecordRef::Json(record))
                .map_err(MixedJsonError::Resource)?;
            let id = identity(record, self.name, work)?;
            if !batch_ids
                .insert_admitted(
                    id.try_clone_admitted(work)
                        .map_err(MixedJsonError::Resource)?,
                    &admission,
                )
                .map_err(MixedJsonError::Resource)?
            {
                return Err(format!("StorageUnavailable：{} 封存批次重复身份", self.name).into());
            }
            let (stored, canonical, born) =
                self.records.get(id.as_ref().as_str()).ok_or_else(|| {
                    MixedJsonError::Business(format!(
                        "StorageUnavailable：{} 封存批次成员没有独立索引端点",
                        self.name
                    ))
                })?;
            observer
                .visit_record(
                    self.name,
                    "family-batch-index",
                    LegacyRecordRef::Json(stored.as_ref()),
                )
                .map_err(MixedJsonError::Resource)?;
            // 原短路：未来出生不执行规范投影。
            let differs = if *born > g {
                true
            } else {
                let projected = scoped_legacy::legacy_payload(record, work)?;
                let actual = Text::try_format(format_args!("{}", projected.as_ref()), Some(work))
                    .map_err(MixedJsonError::Resource)?;
                *canonical != actual
            };
            if differs {
                return Err(format!(
                    "StorageUnavailable：{} 封存批次内容/发布代与独立索引不一致",
                    self.name
                )
                .into());
            }
        }
        let empty = NativeSet::default();
        let born = self.born.get(&g).unwrap_or(&empty);
        if mode.changes_only() {
            if !present.iter().eq(born.iter()) {
                return Err(format!("StorageUnavailable：{} 本代 Delta 不等于本代首次发布集（漏 birth 或伪旧 member）",self.name).into());
            }
            if !born
                .iter()
                .all(|id| batch_ids.contains(id.as_ref().as_str()))
            {
                return Err(
                    format!("StorageUnavailable：{} 封存批次缺本代首次成员", self.name).into(),
                );
            }
        } else if !born.iter().all(|id| present.contains(id.as_ref().as_str())) {
            return Err(format!(
                "StorageUnavailable：{} 本代索引出生未出现在 Delta",
                self.name
            )
            .into());
        }
        Ok(())
    }
    pub(crate) fn finish(&self) -> Result<(), MixedJsonError> {
        if self.seen.len() != self.records.len() {
            return Err(format!("StorageUnavailable：{} 存在不可达幽灵行", self.name).into());
        }
        Ok(())
    }
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }
}

pub(crate) fn read_facts(
    conn: &Connection,
    as_of: Option<i64>,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<
    (
        newchan_rust::s_content::CodecVec<PlainJson>,
        newchan_rust::s_content::CodecVec<PlainJson>,
    ),
    MixedJsonError,
> {
    use newchan_rust::s_content::CodecVec;
    use scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    if !tb02a::enabled_admitted(conn, Some(work))? {
        return Ok((CodecVec::default(), CodecVec::default()));
    }
    let format = evidence_store::read_format_typed(conn, None, Some(work))?;
    let mut reader =
        evidence_store::Reader::new_mixed(conn, crate::EVIDENCE_READ_BUDGET, work, observer)?;
    let mut stmt = conn
        .prepare(tb02a::FACTS_READ_SQL)
        .map_err(|e| MixedJsonError::Business(e.to_string()))?;
    let mut cursor = stmt
        .query([])
        .map_err(|e| MixedJsonError::Business(e.to_string()))?;
    let mut rows = CodecVec::default();
    while let Some(row) = cursor
        .next()
        .map_err(|e| MixedJsonError::Business(e.to_string()))?
    {
        let row = scoped_legacy::collect_fact_row(row, work).map_err(sql_error)?;
        rows.try_reserve_native(work, 1)
            .map_err(MixedJsonError::Resource)?;
        rows.try_push(row).map_err(MixedJsonError::Resource)?;
    }
    drop(cursor);
    let mut enriched = CodecVec::default();
    for row in rows {
        let row = if format.records_in_graph() {
            let object_id_owner =
                Text::try_from_str(row["object_id"].as_str().unwrap_or(""), Some(work))
                    .map_err(MixedJsonError::Resource)?;
            let object_id = object_id_owner.as_ref();
            for (column, label) in [
                ("payload_json", "payload"),
                ("input_refs_json", "input_refs"),
                ("source_coords_json", "source_coords"),
            ] {
                if !row[column].as_str().unwrap_or("").is_empty() {
                    return Err(format!("StorageUnavailable：图域结构事实 `{object_id}` 行内 {label} 仍带整块原文，拒绝").into());
                }
            }
            let key = evidence_store::fact_root_key_mixed(&object_id, work)?;
            let bytes = reader.read_root_typed(&key, false)?.bytes;
            let what = Text::try_format(format_args!("事实 `{object_id}`"), Some(work))
                .map_err(MixedJsonError::Resource)?;
            let (payload, refs, coords) =
                evidence_store::split_fact_root_mixed(&bytes, &what, work)?;
            let mut projected =
                ScopedLegacyRecord::from_owned(ProjectionKind::LegacyPayload, row, work)?;
            projected.replace_text(Field::PayloadJson, &payload)?;
            projected.replace_text(Field::InputRefsJson, &refs)?;
            projected.replace_text(Field::SourceCoordsJson, &coords)?;
            projected.finish()
        } else {
            row
        };
        enriched
            .try_reserve_native(work, 1)
            .map_err(MixedJsonError::Resource)?;
        enriched.try_push(row).map_err(MixedJsonError::Resource)?;
    }
    let mut active = CodecVec::default();
    let mut withdrawn = CodecVec::default();
    for row in enriched {
        observer
            .visit_record("objects", "fact-lifecycle", LegacyRecordRef::Json(&row))
            .map_err(MixedJsonError::Resource)?;
        let Some(row) = scoped_legacy::filter_fact_lifecycle(row, as_of, work)? else {
            continue;
        };
        let target = if row["withdrawn_generation"].is_null() {
            &mut active
        } else {
            &mut withdrawn
        };
        target
            .try_reserve_native(work, 1)
            .map_err(MixedJsonError::Resource)?;
        target.try_push(row).map_err(MixedJsonError::Resource)?;
    }
    Ok((active, withdrawn))
}

pub(crate) fn read_objects(
    conn: &Connection,
    as_of: Option<i64>,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<
    (
        newchan_rust::s_content::CodecVec<PlainJson>,
        newchan_rust::s_content::CodecVec<PlainJson>,
    ),
    MixedJsonError,
> {
    use newchan_rust::s_content::CodecVec;
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    let invalid:i64=conn.query_row(
        "SELECT COUNT(*) FROM objects WHERE withdrawn_generation IS NOT NULL AND withdrawn_generation <= first_known_generation",
        [],|r|r.get(0)).map_err(|e|MixedJsonError::Business(format!("核对象生命周期失败：{e}")))?;
    if invalid != 0 {
        return Err("StorageUnavailable：对象生命周期非法（withdrawn_generation <= first_known_generation）".into());
    }
    let collect = |sql: &str, label: &str| -> Result<CodecVec<PlainJson>, MixedJsonError> {
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| MixedJsonError::Business(format!("prepare {label} 失败：{e}")))?;
        let mut rows = match as_of {
            None => stmt.query([]),
            Some(n) => stmt.query(params![n]),
        }
        .map_err(|e| MixedJsonError::Business(format!("query {label} 失败：{e}")))?;
        let mut out = CodecVec::default();
        while let Some(row) = rows
            .next()
            .map_err(|e| MixedJsonError::Business(format!("collect {label} 失败：{e}")))?
        {
            let value =
                scoped_legacy::project_object_row(row, as_of, work).map_err(
                    |error| match error {
                        scoped_legacy::MixedSqlError::Sql(e) => {
                            MixedJsonError::Business(format!("collect {label} 失败：{e}"))
                        }
                        scoped_legacy::MixedSqlError::Projection(e) => e,
                    },
                )?;
            observer
                .visit_record(
                    "objects",
                    "object-sql-project",
                    LegacyRecordRef::Json(&value),
                )
                .map_err(MixedJsonError::Resource)?;
            out.try_reserve_native(work, 1)
                .map_err(MixedJsonError::Resource)?;
            out.try_push(value).map_err(MixedJsonError::Resource)?;
        }
        Ok(out)
    };
    let mut objects = collect(
        if as_of.is_none() {
            crate::OBJECT_ACTIVE_SQL
        } else {
            crate::OBJECT_ACTIVE_ASOF_SQL
        },
        "objects",
    )?;
    let mut withdrawn = collect(
        if as_of.is_none() {
            crate::OBJECT_DEAD_SQL
        } else {
            crate::OBJECT_DEAD_ASOF_SQL
        },
        "withdrawn",
    )?;
    let (typed_active, typed_dead) = read_facts(conn, as_of, work, observer)?;
    for (source, target) in [(&typed_active, &mut objects), (&typed_dead, &mut withdrawn)] {
        // 原collect完整typed投影成功后才extend目标。
        let mut projected = CodecVec::default();
        for value in source {
            observer
                .visit_record("objects", "fact-project", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            let value = scoped_legacy::project_fact(value, work)?;
            projected
                .try_reserve_native(work, 1)
                .map_err(MixedJsonError::Resource)?;
            projected
                .try_push(value)
                .map_err(MixedJsonError::Resource)?;
        }
        target
            .try_reserve_native(work, projected.len())
            .map_err(MixedJsonError::Resource)?;
        for value in projected {
            target.try_push(value).map_err(MixedJsonError::Resource)?;
        }
    }
    Ok((objects, withdrawn))
}

pub(crate) struct AuditWalk {
    active: NativeTree<Text, Shared>,
    births: NativeTree<i64, newchan_rust::s_content::CodecVec<Shared>>,
    deaths: NativeTree<i64, newchan_rust::s_content::CodecVec<Shared>>,
    witnesses: Family,
    relations: Family,
    observations: Family,
    original_observations: NativeTree<Text, Shared>,
    raw: newchan_rust::s_content::CodecVec<PlainJson>,
    objects: NativeTree<Text, Shared>,
}
fn push_event(
    tree: &mut NativeTree<i64, newchan_rust::s_content::CodecVec<Shared>>,
    generation: i64,
    value: Shared,
    work: &WorkScope,
) -> Result<(), MixedJsonError> {
    let admission = NativeAdmission(Some(work));
    if !tree.contains_key(&generation) {
        tree.insert_admitted(generation, Default::default(), &admission)
            .map_err(MixedJsonError::Resource)?;
    }
    let list = tree.get_mut(&generation).unwrap();
    list.try_reserve_native(work, 1)
        .map_err(MixedJsonError::Resource)?;
    list.try_push(value).map_err(MixedJsonError::Resource)
}
impl AuditWalk {
    pub(crate) fn capture<'a>(
        conn: &Connection,
        raw: impl IntoIterator<Item = &'a Value>,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<Self, MixedJsonError> {
        let (objects, withdrawn) = read_objects(conn, None, work, observer)?;
        let witnesses = Family::capture(conn, "witnesses", work, observer)?;
        let relations = Family::capture(conn, "relations", work, observer)?;
        let observations = Family::capture(conn, "observations", work, observer)?;
        let mut raw_copy = newchan_rust::s_content::CodecVec::default();
        for record in raw {
            let copied =
                PlainJson::try_clone_from(record, Some(work)).map_err(MixedJsonError::Resource)?;
            raw_copy
                .try_reserve_native(work, 1)
                .map_err(MixedJsonError::Resource)?;
            raw_copy
                .try_push(copied)
                .map_err(MixedJsonError::Resource)?;
        }
        let mut result = Self {
            active: Default::default(),
            births: Default::default(),
            deaths: Default::default(),
            witnesses,
            relations,
            observations,
            original_observations: Default::default(),
            raw: raw_copy,
            objects: Default::default(),
        };
        let mut ids = NativeSet::default();
        let admission = NativeAdmission(Some(work));
        for value in objects.iter().chain(withdrawn.iter()) {
            observer
                .visit_record("objects", "capture-lifecycle", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            if !ids
                .insert_admitted(identity(value, "objects", work)?, &admission)
                .map_err(MixedJsonError::Resource)?
            {
                return Err("StorageUnavailable：对象重复身份".into());
            }
            let copied = value
                .try_clone_mixed(work)
                .map_err(MixedJsonError::Resource)?;
            let shared =
                NativeArc::try_new_shallow(copied, Some(work)).map_err(MixedJsonError::Resource)?;
            result
                .objects
                .insert_admitted(
                    identity(value, "objects", work)?,
                    shared.clone(),
                    &admission,
                )
                .map_err(MixedJsonError::Resource)?;
            push_event(
                &mut result.births,
                number(value, "first_known_generation")?,
                shared.clone(),
                work,
            )?;
            if !value["withdrawn_generation"].is_null() {
                push_event(
                    &mut result.deaths,
                    number(value, "withdrawn_generation")?,
                    shared,
                    work,
                )?;
            }
        }
        Ok(result)
    }
    pub(crate) fn source_bytes(
        &self,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<usize, MixedJsonError> {
        observer
            .check_work(work)
            .map_err(MixedJsonError::Resource)?;
        let admission = NativeAdmission(Some(work));
        let mut seen = NativeSet::default();
        let mut total = 0usize;
        let account = |seen: &mut NativeSet<usize>,
                       total: &mut usize,
                       value: &Shared,
                       family: &'static str|
         -> Result<(), MixedJsonError> {
            let pointer = NativeArc::as_ptr(value) as usize;
            if seen
                .insert_admitted(pointer, &admission)
                .map_err(MixedJsonError::Resource)?
            {
                value
                    .check_retained_source()
                    .map_err(MixedJsonError::Resource)?;
                value
                    .as_ref()
                    .check_retained_source()
                    .map_err(MixedJsonError::Resource)?;
                observer
                    .visit_record(
                        family,
                        "source-bytes",
                        LegacyRecordRef::Json(value.as_ref()),
                    )
                    .map_err(MixedJsonError::Resource)?;
                let canonical =
                    Text::try_format(format_args!("{}", value.as_ref().as_ref()), Some(work))
                        .map_err(MixedJsonError::Resource)?;
                *total = total.saturating_add(canonical.len());
            }
            Ok(())
        };
        for (value, family) in self
            .active
            .values()
            .chain(self.objects.values())
            .map(|value| (value, "objects"))
            .chain(
                self.original_observations
                    .values()
                    .map(|value| (value, "observations")),
            )
            .chain(
                self.births
                    .values()
                    .flat_map(|values| values.iter())
                    .map(|value| (value, "objects")),
            )
            .chain(
                self.deaths
                    .values()
                    .flat_map(|values| values.iter())
                    .map(|value| (value, "objects")),
            )
        {
            account(&mut seen, &mut total, value, family)?;
        }
        for value in &self.raw {
            value
                .check_retained_source()
                .map_err(MixedJsonError::Resource)?;
            observer
                .visit_record("raw_history", "source-bytes", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            let canonical = Text::try_format(format_args!("{}", value.as_ref()), Some(work))
                .map_err(MixedJsonError::Resource)?;
            total = total.saturating_add(canonical.len());
        }
        for family in [&self.witnesses, &self.relations, &self.observations] {
            for (id, (value, canonical, _)) in family.records.iter() {
                account(&mut seen, &mut total, value, family.name)?;
                total = total.saturating_add(id.len() + canonical.len() + 8);
            }
            for id in family.seen.iter() {
                total = total.saturating_add(id.len());
            }
            for id in family.born.values().flat_map(|ids| ids.iter()) {
                total = total.saturating_add(id.len() + 8);
            }
        }
        Ok(total)
    }
    pub(crate) fn resume_from(
        &mut self,
        old: &Self,
        generation: i64,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<bool, MixedJsonError> {
        observer
            .check_work(work)
            .map_err(MixedJsonError::Resource)?;
        if self.raw.len() < old.raw.len() {
            return Ok(false);
        }
        old.raw
            .check_retained_source()
            .map_err(MixedJsonError::Resource)?;
        for (current, previous) in self.raw.iter().zip(old.raw.iter()) {
            previous
                .check_retained_source()
                .map_err(MixedJsonError::Resource)?;
            observer
                .visit_record(
                    "raw_history",
                    "resume-raw-current",
                    LegacyRecordRef::Json(current),
                )
                .map_err(MixedJsonError::Resource)?;
            observer
                .visit_record(
                    "raw_history",
                    "resume-raw-previous",
                    LegacyRecordRef::Json(previous),
                )
                .map_err(MixedJsonError::Resource)?;
            if current.as_ref() != previous.as_ref() {
                return Ok(false);
            }
        }
        for (current, previous) in [
            (&self.witnesses, &old.witnesses),
            (&self.relations, &old.relations),
            (&self.observations, &old.observations),
        ] {
            for (id, (record, canonical, g)) in previous.records.iter() {
                record
                    .check_retained_source()
                    .map_err(MixedJsonError::Resource)?;
                record
                    .as_ref()
                    .check_retained_source()
                    .map_err(MixedJsonError::Resource)?;
                observer
                    .visit_record(
                        previous.name,
                        "resume-family-previous",
                        LegacyRecordRef::Json(record.as_ref()),
                    )
                    .map_err(MixedJsonError::Resource)?;
                let Some((next, next_c, next_g)) = current.records.get(id.as_ref().as_str()) else {
                    return Ok(false);
                };
                observer
                    .visit_record(
                        current.name,
                        "resume-family-current",
                        LegacyRecordRef::Json(next.as_ref()),
                    )
                    .map_err(MixedJsonError::Resource)?;
                if record.as_ref().as_ref() != next.as_ref().as_ref()
                    || canonical != next_c
                    || g != next_g
                {
                    return Ok(false);
                }
            }
            for (id, (record, _, g)) in current.records.iter() {
                observer
                    .visit_record(
                        current.name,
                        "resume-family-birth",
                        LegacyRecordRef::Json(record.as_ref()),
                    )
                    .map_err(MixedJsonError::Resource)?;
                if *g <= generation && !previous.records.contains_key(id.as_ref().as_str()) {
                    return Ok(false);
                }
            }
        }
        let mut matched = 0;
        for (id, record) in self.objects.iter() {
            observer
                .visit_record(
                    "objects",
                    "resume-object-current",
                    LegacyRecordRef::Json(record.as_ref()),
                )
                .map_err(MixedJsonError::Resource)?;
            let Ok(born) = number(record.as_ref(), "first_known_generation") else {
                return Ok(false);
            };
            if born > generation {
                continue;
            }
            let mut projected = None;
            if !record["withdrawn_generation"].is_null() {
                let Ok(withdrawn) = number(record.as_ref(), "withdrawn_generation") else {
                    return Ok(false);
                };
                if withdrawn > generation {
                    use scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
                    let mut previous = ScopedLegacyRecord::clone_projection(
                        ProjectionKind::AtPreviousLifecycle,
                        record.as_ref(),
                        work,
                    )?;
                    for field in [
                        Field::WithdrawnGeneration,
                        Field::WithdrawalReason,
                        Field::SupersededBy,
                    ] {
                        previous.replace_known_field(field, &Value::Null)?;
                    }
                    previous.replace_text(Field::Lifecycle, "active")?;
                    projected = Some(previous.finish());
                }
            }
            let Some(previous) = old.objects.get(id.as_ref().as_str()) else {
                return Ok(false);
            };
            previous
                .check_retained_source()
                .map_err(MixedJsonError::Resource)?;
            previous
                .as_ref()
                .check_retained_source()
                .map_err(MixedJsonError::Resource)?;
            observer
                .visit_record(
                    "objects",
                    "resume-object-previous",
                    LegacyRecordRef::Json(previous.as_ref()),
                )
                .map_err(MixedJsonError::Resource)?;
            let at_previous = projected
                .as_ref()
                .map(PlainJson::as_ref)
                .unwrap_or_else(|| record.as_ref().as_ref());
            if previous.as_ref().as_ref() != at_previous {
                return Ok(false);
            }
            matched += 1;
        }
        if matched != old.objects.len() {
            return Ok(false);
        }
        let copy_tree = |tree: &NativeTree<Text, Shared>| {
            tree.try_copy_mixed(work, |key, value| {
                Ok((
                    key.try_clone_admitted(work)?,
                    value.share_or_copy_mixed(work, |value, work| value.try_clone_mixed(work))?,
                ))
            })
            .map_err(MixedJsonError::Resource)
        };
        let copy_set = |set: &NativeSet<Text>| -> Result<NativeSet<Text>, MixedJsonError> {
            let mut next = NativeSet::default();
            let admission = NativeAdmission(Some(work));
            for id in set.iter() {
                id.check_retained_source()
                    .map_err(MixedJsonError::Resource)?;
                next.insert_admitted(
                    id.try_clone_admitted(work)
                        .map_err(MixedJsonError::Resource)?,
                    &admission,
                )
                .map_err(MixedJsonError::Resource)?;
            }
            Ok(next)
        };
        let active = copy_tree(&old.active)?;
        let witnesses = copy_set(&old.witnesses.seen)?;
        let relations = copy_set(&old.relations.seen)?;
        let observations = copy_set(&old.observations.seen)?;
        let originals = copy_tree(&old.original_observations)?;
        self.births.retain(|g, _| *g > generation);
        self.deaths.retain(|g, _| *g > generation);
        self.active = active;
        self.witnesses.seen = witnesses;
        self.relations.seen = relations;
        self.observations.seen = observations;
        self.original_observations = originals;
        Ok(true)
    }
    pub(crate) fn finish(&self) -> Result<(), MixedJsonError> {
        if !self.births.is_empty() || !self.deaths.is_empty() {
            return Err("StorageUnavailable：对象生灭代不可达".into());
        }
        self.witnesses.finish()?;
        self.relations.finish()?;
        self.observations.finish()
    }
}

fn same_records<'a, 'b>(
    actual: impl IntoIterator<Item = &'a Value>,
    expected: impl IntoIterator<Item = &'b Value>,
    field: &str,
    family: &'static str,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<(), MixedJsonError> {
    let actual = scoped_legacy::normalize_records(actual, field, family, work, observer)?;
    let expected = scoped_legacy::normalize_records(expected, field, family, work, observer)?;
    scoped_legacy::compare_normalized(&actual, &expected, field, family, observer)
}
fn value_array(value: &Value) -> Result<&[Value], MixedJsonError> {
    value
        .as_array()
        .map(|v| v.as_slice())
        .ok_or_else(|| MixedJsonError::Business("StorageUnavailable：集合非数组".into()))
}
fn verify_delta_evidence(
    delta: &Value,
    batch: &Value,
    work: &WorkScope,
    observer: &dyn LegacyObserver,
) -> Result<(), MixedJsonError> {
    crate::verify_delta_prefix(delta, batch)?;
    let evidence = &delta["catalog_evidence"];
    if batch["profile_id"] == tb02a::PROFILE {
        let changes = &evidence["axes"]["CC-056"]["evidence"];
        same_records(
            value_array(&changes["withdrawals"])?,
            value_array(&delta["withdrawals"])?,
            "CC056撤回证据",
            "objects",
            work,
            observer,
        )?;
        same_records(
            value_array(&changes["replaces"])?,
            value_array(&delta["replaces"])?,
            "CC056替代证据",
            "relations",
            work,
            observer,
        )?;
        let start = parse_canonical_i64(
            delta["seq_range"]["from"]
                .as_str()
                .ok_or("StorageUnavailable：Delta起始前沿缺失")?,
            "from",
        )?;
        let mut revisions = newchan_rust::s_content::CodecVec::default();
        for raw in required_array(batch, "raw_events")? {
            observer
                .visit_record(
                    "raw_history",
                    "delta-raw-filter",
                    LegacyRecordRef::Json(raw),
                )
                .map_err(MixedJsonError::Resource)?;
            if raw["seq"].as_i64().is_some_and(|n| n >= start)
                && raw["revision"].as_i64().is_some_and(|n| n > 1)
            {
                use scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
                let mut out = ScopedLegacyRecord::object(ProjectionKind::RawEventWire, work)?;
                for (key, field) in [
                    ("identity_key", Field::IdentityKey),
                    ("payload_hash", Field::PayloadHash),
                    ("receipt_id", Field::ReceiptId),
                    ("event_id", Field::EventId),
                    ("input_revision", Field::InputRevision),
                ] {
                    out.borrowed(field, &raw[key])?;
                }
                out.integer(Field::Revision, raw["revision"].as_i64().unwrap())?;
                out.integer(Field::Seq, raw["seq"].as_i64().unwrap())?;
                out.borrowed(Field::SourceCoord, &raw["source_coord"])?;
                revisions
                    .try_reserve_native(work, 1)
                    .map_err(MixedJsonError::Resource)?;
                revisions
                    .try_push(out.finish())
                    .map_err(MixedJsonError::Resource)?;
            }
        }
        same_records(
            value_array(&changes["raw_revisions"])?,
            revisions.iter().map(PlainJson::as_ref),
            "CC056原始修订证据",
            "raw_history",
            work,
            observer,
        )?;
    }
    crate::verify_delta_suffix(delta, batch)?;
    Ok(())
}

fn push<T>(
    items: &mut newchan_rust::s_content::CodecVec<T>,
    value: T,
    work: &WorkScope,
) -> Result<(), MixedJsonError> {
    items
        .try_reserve_native(work, 1)
        .map_err(MixedJsonError::Resource)?;
    items.try_push(value).map_err(MixedJsonError::Resource)
}
fn copy_shared_tree(
    tree: &NativeTree<Text, Shared>,
    work: &WorkScope,
) -> Result<NativeTree<Text, Shared>, MixedJsonError> {
    tree.try_copy_mixed(work, |key, value| {
        Ok((
            key.try_clone_admitted(work)?,
            value.share_or_copy_mixed(work, |value, work| value.try_clone_mixed(work))?,
        ))
    })
    .map_err(MixedJsonError::Resource)
}
fn active_projection(value: &Value, work: &WorkScope) -> Result<PlainJson, MixedJsonError> {
    use scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
    let mut out = ScopedLegacyRecord::clone_projection(ProjectionKind::IndexBirth, value, work)?;
    for field in [
        Field::WithdrawnGeneration,
        Field::WithdrawalReason,
        Field::SupersededBy,
    ] {
        out.replace_known_field(field, &Value::Null)?;
    }
    out.replace_text(Field::Lifecycle, "active")?;
    Ok(out.finish())
}
fn observation_identity(value: &Value, work: &WorkScope) -> Result<Text, MixedJsonError> {
    use scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
    use sha2::Digest;
    if value["detail"]["tb02a"] == true {
        let canonical = Text::try_format(format_args!("{value}"), Some(work))
            .map_err(MixedJsonError::Resource)?;
        return Text::try_format(
            format_args!("obs-{:x}", sha2::Sha256::digest(canonical.as_bytes())),
            Some(work),
        )
        .map_err(MixedJsonError::Resource);
    }
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObservationWire, work)?;
    for (key, field) in [
        ("kind", Field::Kind),
        ("window_start", Field::WindowStart),
        ("window_mid", Field::WindowMid),
        ("window_end", Field::WindowEnd),
        ("reason", Field::Reason),
    ] {
        out.borrowed(field, &value[key])?;
    }
    let out = out.finish();
    let canonical = Text::try_format(format_args!("{}", out.as_ref()), Some(work))
        .map_err(MixedJsonError::Resource)?;
    let digest = Text::try_format(
        format_args!("{:x}", sha2::Sha256::digest(canonical.as_bytes())),
        Some(work),
    )
    .map_err(MixedJsonError::Resource)?;
    Text::try_format(format_args!("obs-{}", &digest[..16]), Some(work))
        .map_err(MixedJsonError::Resource)
}
impl AuditWalk {
    pub(crate) fn verify(
        &mut self,
        delta: &Value,
        g: i64,
        batch: &Value,
        mode: delta_storage::Mode,
        work: &WorkScope,
        observer: &dyn LegacyObserver,
    ) -> Result<(), MixedJsonError> {
        use newchan_rust::s_content::CodecVec;
        use scoped_legacy::{Field, ProjectionKind, ScopedLegacyRecord};
        observer
            .check_work(work)
            .map_err(MixedJsonError::Resource)?;
        let admission = NativeAdmission(Some(work));
        verify_delta_evidence(delta, batch, work, observer)?;
        let mut batch_next = CodecVec::default();
        for record in required_array(batch, "objects")? {
            observer
                .visit_record("objects", "batch-next", LegacyRecordRef::Json(record))
                .map_err(MixedJsonError::Resource)?;
            let mut record =
                ScopedLegacyRecord::clone_projection(ProjectionKind::ObjectWire, record, work)?;
            record.replace_known_field(Field::BatchId, &delta["index_frontier"])?;
            let wire = scoped_legacy::project_object_wire(&record.finish(), work)?;
            let wire = ScopedLegacyRecord::from_owned(ProjectionKind::ObjectWire, wire, work)?;
            // 身份读取发生在原project_object_wire之后。
            let value = wire.finish();
            let id = identity(&value, "objects", work)?;
            let mut wire = ScopedLegacyRecord::from_owned(ProjectionKind::ObjectWire, value, work)?;
            if let Some(old) = self.active.get(id.as_ref().as_str()) {
                observer
                    .visit_record(
                        "objects",
                        "batch-next-active",
                        LegacyRecordRef::Json(old.as_ref()),
                    )
                    .map_err(MixedJsonError::Resource)?;
                for (key, field) in [
                    ("batch_id", Field::BatchId),
                    ("first_known_generation", Field::FirstKnownGeneration),
                    ("first_known_cut", Field::FirstKnownCut),
                    ("published_generation", Field::PublishedGeneration),
                ] {
                    wire.replace_known_field(field, &old[key])?;
                }
            }
            push(&mut batch_next, wire.finish(), work)?;
        }
        let mut index_next = copy_shared_tree(&self.active, work)?;
        let mut index_born = CodecVec::default();
        if let Some(births) = self.births.remove(&g) {
            for value in births {
                observer
                    .visit_record(
                        "objects",
                        "index-birth",
                        LegacyRecordRef::Json(value.as_ref()),
                    )
                    .map_err(MixedJsonError::Resource)?;
                let value = NativeArc::try_new_shallow(
                    active_projection(value.as_ref(), work)?,
                    Some(work),
                )
                .map_err(MixedJsonError::Resource)?;
                push(&mut index_born, value.clone(), work)?;
                if index_next
                    .insert_admitted(
                        identity(value.as_ref(), "objects", work)?,
                        value,
                        &admission,
                    )
                    .map_err(MixedJsonError::Resource)?
                    .is_some()
                {
                    return Err("StorageUnavailable：重复对象出生".into());
                }
            }
        }
        let mut withdrawals = CodecVec::default();
        let mut replaces = CodecVec::default();
        for value in self.deaths.remove(&g).unwrap_or_default() {
            observer
                .visit_record(
                    "objects",
                    "index-death",
                    LegacyRecordRef::Json(value.as_ref()),
                )
                .map_err(MixedJsonError::Resource)?;
            let id = identity(value.as_ref(), "objects", work)?;
            if index_next.remove(id.as_ref().as_str()).is_none() {
                return Err("StorageUnavailable：撤回对象尚未出生".into());
            }
            if number(value.as_ref(), "withdrawn_generation")? != g
                || number(value.as_ref(), "first_known_generation")? > g
            {
                return Err("StorageUnavailable：撤回历史首次获知/撤回代不一致".into());
            }
            let reason = value["withdrawal_reason"]
                .as_str()
                .ok_or("StorageUnavailable：撤回原因缺失")?;
            let mut withdrawal = ScopedLegacyRecord::object(ProjectionKind::Withdrawal, work)?;
            withdrawal.borrowed(Field::ObjectId, &value["object_id"])?;
            if tb02a::typed(value.as_ref()) {
                withdrawal.borrowed(Field::FactKey, &value["fact_key"])?;
            } else {
                for (key, field) in [
                    ("window_start", Field::WindowStart),
                    ("window_mid", Field::WindowMid),
                    ("window_end", Field::WindowEnd),
                ] {
                    withdrawal.borrowed(field, &value[key])?;
                }
            }
            withdrawal.text(Field::Reason, reason)?;
            withdrawal.borrowed(Field::SupersededBy, &value["superseded_by"])?;
            push(&mut withdrawals, withdrawal.finish(), work)?;
            if !value["superseded_by"].is_null() {
                let mut replace = ScopedLegacyRecord::object(ProjectionKind::Relation, work)?;
                replace.borrowed(Field::OldObjectId, &value["object_id"])?;
                replace.borrowed(Field::NewObjectId, &value["superseded_by"])?;
                push(&mut replaces, replace.finish(), work)?;
            }
        }
        let mut delta_next = copy_shared_tree(&self.active, work)?;
        for value in required_array(delta, "upserts")? {
            observer
                .visit_record("objects", "delta-upsert", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            let id = identity(value, "objects", work)?;
            match delta_next.get(id.as_ref().as_str()) {
                None => {
                    let copied = PlainJson::try_clone_from(value, Some(work))
                        .map_err(MixedJsonError::Resource)?;
                    let value = NativeArc::try_new_shallow(copied, Some(work))
                        .map_err(MixedJsonError::Resource)?;
                    delta_next
                        .insert_admitted(id, value, &admission)
                        .map_err(MixedJsonError::Resource)?;
                }
                Some(old) if !mode.changes_only() => {
                    observer
                        .visit_record(
                            "objects",
                            "delta-upsert-active",
                            LegacyRecordRef::Json(old.as_ref()),
                        )
                        .map_err(MixedJsonError::Resource)?;
                    let old = scoped_legacy::legacy_payload(old.as_ref(), work)?;
                    let old = Text::try_format(format_args!("{}", old.as_ref()), Some(work))
                        .map_err(MixedJsonError::Resource)?;
                    let next = scoped_legacy::legacy_payload(value, work)?;
                    let next = Text::try_format(format_args!("{}", next.as_ref()), Some(work))
                        .map_err(MixedJsonError::Resource)?;
                    if old != next {
                        return Err(
                            "StorageUnavailable：Delta upsert 与既有活动身份内容不一致".into()
                        );
                    }
                }
                Some(_) => {
                    return Err(
                        "StorageUnavailable：Delta upsert 覆盖既有活动身份（同 ID 异内容不得吞掉）"
                            .into(),
                    )
                }
            }
        }
        for value in required_array(delta, "withdrawals")? {
            observer
                .visit_record("objects", "delta-withdrawal", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            let id = Text::try_from_str(
                value["object_id"]
                    .as_str()
                    .ok_or("StorageUnavailable：Delta 撤回缺 object_id")?,
                Some(work),
            )
            .map_err(MixedJsonError::Resource)?;
            if delta_next.remove(id.as_ref().as_str()).is_none() {
                return Err("StorageUnavailable：Delta 撤回无活动前件".into());
            }
        }
        let normalized_batch = {
            let actual = scoped_legacy::normalize_records(
                index_next.values().map(|v| v.as_ref().as_ref()),
                "objects/独立索引活动集",
                "objects",
                work,
                observer,
            )?;
            let expected = scoped_legacy::normalize_records(
                batch_next.iter().map(PlainJson::as_ref),
                "objects/独立索引活动集",
                "objects",
                work,
                observer,
            )?;
            scoped_legacy::compare_normalized(
                &actual,
                &expected,
                "objects/独立索引活动集",
                "objects",
                observer,
            )?;
            expected
        };
        drop(batch_next);
        {
            let actual = scoped_legacy::normalize_records(
                delta_next.values().map(|v| v.as_ref().as_ref()),
                "objects/Delta活动集",
                "objects",
                work,
                observer,
            )?;
            scoped_legacy::compare_normalized(
                &actual,
                &normalized_batch,
                "objects/Delta活动集",
                "objects",
                observer,
            )?;
        }
        same_records(
            value_array(&delta["withdrawals"])?,
            withdrawals.iter().map(PlainJson::as_ref),
            "withdrawals",
            "objects",
            work,
            observer,
        )?;
        same_records(
            value_array(&delta["replaces"])?,
            replaces.iter().map(PlainJson::as_ref),
            "replaces",
            "relations",
            work,
            observer,
        )?;
        if mode.changes_only() {
            same_records(
                value_array(&delta["upserts"])?,
                index_born.iter().map(|v| v.as_ref().as_ref()),
                "upserts/本代首次",
                "objects",
                work,
                observer,
            )?;
        } else {
            let actual = scoped_legacy::normalize_records(
                value_array(&delta["upserts"])?,
                "upserts/batch",
                "objects",
                work,
                observer,
            )?;
            scoped_legacy::compare_normalized(
                &actual,
                &normalized_batch,
                "upserts/batch",
                "objects",
                observer,
            )?;
        }
        drop(normalized_batch);
        let mut witnesses = CodecVec::default();
        for value in required_array(batch, "witnesses")? {
            observer
                .visit_record("witnesses", "batch-project", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            push(
                &mut witnesses,
                scoped_legacy::project_witness_wire(value, work)?,
                work,
            )?;
        }
        let mut relations = CodecVec::default();
        for value in required_array(batch, "relations")? {
            observer
                .visit_record("relations", "batch-project", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            push(
                &mut relations,
                PlainJson::try_clone_from(value, Some(work)).map_err(MixedJsonError::Resource)?,
                work,
            )?;
        }
        for replace in &replaces {
            observer
                .visit_record(
                    "relations",
                    "replacement-relation",
                    LegacyRecordRef::Json(replace),
                )
                .map_err(MixedJsonError::Resource)?;
            let mut relation = ScopedLegacyRecord::object(ProjectionKind::Relation, work)?;
            relation.borrowed(Field::Subject, &replace["new_object_id"])?;
            relation.text(Field::RelationType, "replaces")?;
            relation.borrowed(Field::Object, &replace["old_object_id"])?;
            push(&mut relations, relation.finish(), work)?;
        }
        let to = number(delta, "input_frontier")?;
        let end = usize::try_from(to.checked_add(1).ok_or("StorageUnavailable：前沿溢出")?)
            .map_err(|e| e.to_string())?;
        let raw = self
            .raw
            .get(..end)
            .ok_or("StorageUnavailable：批次前沿超过原始事实")?;
        let mut batch_raw = CodecVec::default();
        for value in required_array(batch, "raw_events")? {
            observer
                .visit_record("raw_history", "batch-project", LegacyRecordRef::Json(value))
                .map_err(MixedJsonError::Resource)?;
            push(
                &mut batch_raw,
                scoped_legacy::project_raw_event_wire(value, work)?,
                work,
            )?;
        }
        same_records(
            raw.iter().map(PlainJson::as_ref),
            batch_raw.iter().map(PlainJson::as_ref),
            "raw_history/batch",
            "raw_history",
            work,
            observer,
        )?;
        let from =
            usize::try_from(number(&delta["seq_range"], "from")?).map_err(|e| e.to_string())?;
        same_records(
            value_array(&delta["raw_history_added"])?,
            raw.get(from..)
                .ok_or("StorageUnavailable：raw区间反向")?
                .iter()
                .map(PlainJson::as_ref),
            "raw_history_added",
            "raw_history",
            work,
            observer,
        )?;
        for value in raw {
            observer
                .visit_record(
                    "raw_history",
                    "raw-supersedes",
                    LegacyRecordRef::Json(value),
                )
                .map_err(MixedJsonError::Resource)?;
            if let Some(sup) = value["supersedes_revision"].as_str() {
                let subject = Text::try_format(
                    format_args!(
                        "{}@{}",
                        value["identity_key"].as_str().unwrap_or(""),
                        value["revision"].as_str().unwrap_or("")
                    ),
                    Some(work),
                )
                .map_err(MixedJsonError::Resource)?;
                let object = Text::try_format(
                    format_args!("{}@{sup}", value["identity_key"].as_str().unwrap_or("")),
                    Some(work),
                )
                .map_err(MixedJsonError::Resource)?;
                let mut relation = ScopedLegacyRecord::object(ProjectionKind::Relation, work)?;
                relation.text(Field::Subject, &subject)?;
                relation.text(Field::RelationType, "supersedes")?;
                relation.text(Field::Object, &object)?;
                push(&mut relations, relation.finish(), work)?;
            }
        }
        if !mode.changes_only() {
            same_records(
                value_array(&delta["witnesses"])?,
                witnesses.iter().map(PlainJson::as_ref),
                "witnesses/batch",
                "witnesses",
                work,
                observer,
            )?;
            same_records(
                value_array(&delta["relations"])?,
                relations.iter().map(PlainJson::as_ref),
                "relations/batch",
                "relations",
                work,
                observer,
            )?;
        }
        let mut observations = CodecVec::default();
        let mut first_observations = CodecVec::default();
        for ob in required_array(batch, "observations")? {
            observer
                .visit_record(
                    "observations",
                    "batch-observation",
                    LegacyRecordRef::Json(ob),
                )
                .map_err(MixedJsonError::Resource)?;
            let id = observation_identity(ob, work)?;
            let (stored, _, born) = self
                .observations
                .records
                .get(id.as_ref().as_str())
                .ok_or("StorageUnavailable：观察没有独立端点")?;
            observer
                .visit_record(
                    "observations",
                    "batch-observation-index",
                    LegacyRecordRef::Json(stored.as_ref()),
                )
                .map_err(MixedJsonError::Resource)?;
            if !self
                .original_observations
                .contains_key(id.as_ref().as_str())
            {
                if *born != g || stored["batch_id"] != delta["index_frontier"] {
                    return Err("StorageUnavailable：观察首版批次/出生代不一致".into());
                }
                let mut first = ScopedLegacyRecord::clone_projection(
                    ProjectionKind::ObservationWire,
                    ob,
                    work,
                )?;
                first.replace_text(Field::ObservationId, &id)?;
                first.replace_known_field(Field::BatchId, &delta["index_frontier"])?;
                let original = scoped_legacy::project_observation_wire(&first.finish(), work)?;
                same_records(
                    std::iter::once(stored.as_ref().as_ref()),
                    std::iter::once(original.as_ref()),
                    "observation/原封存首版",
                    "observations",
                    work,
                    observer,
                )?;
                let original = NativeArc::try_new_shallow(original, Some(work))
                    .map_err(MixedJsonError::Resource)?;
                self.original_observations
                    .insert_admitted(
                        id.try_clone_admitted(work)
                            .map_err(MixedJsonError::Resource)?,
                        original,
                        &admission,
                    )
                    .map_err(MixedJsonError::Resource)?;
            }
            let original = self
                .original_observations
                .get(id.as_ref().as_str())
                .unwrap()
                .clone();
            if *born == g {
                push(&mut first_observations, original.clone(), work)?;
            }
            push(&mut observations, original, work)?;
        }
        if mode.changes_only() {
            same_records(
                value_array(&delta["observations"])?,
                first_observations.iter().map(|v| v.as_ref().as_ref()),
                "observations/本代首次",
                "observations",
                work,
                observer,
            )?;
        } else {
            same_records(
                value_array(&delta["observations"])?,
                observations.iter().map(|v| v.as_ref().as_ref()),
                "observations/首版",
                "observations",
                work,
                observer,
            )?;
        }
        self.witnesses.verify(
            delta,
            g,
            mode,
            witnesses.iter().map(PlainJson::as_ref),
            work,
            observer,
        )?;
        self.relations.verify(
            delta,
            g,
            mode,
            relations.iter().map(PlainJson::as_ref),
            work,
            observer,
        )?;
        self.observations.verify(
            delta,
            g,
            mode,
            observations.iter().map(|v| v.as_ref().as_ref()),
            work,
            observer,
        )?;
        self.active = index_next;
        Ok(())
    }
}
