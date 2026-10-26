//! #1527：旧域投影拥有者；键为具名字段，源值只借用，载荷不开放裸修改。
use super::{MixedJsonError, NativeJsonLayout, PlainJson, Value, WorkScope};
use newchan_rust::s_content::{self as sc, capture_allocation::native_owned::NativeValue};

#[derive(Clone, Copy)]
pub(crate) enum ProjectionKind {
    ObjectWire,
    WitnessWire,
    ObservationWire,
    RawEventWire,
    LegacyPayload,
    AtPreviousLifecycle,
    IndexBirth,
    Withdrawal,
    Relation,
}
#[derive(Clone, Copy)]
pub(crate) enum Field {
    OldObjectId,
    NewObjectId,
    SchemaRevision,
    Open,
    High,
    Low,
    Close,
    WitnessId,
    ObservationId,
    Slot,
    MergedSourceIndex,
    MergedHigh,
    MergedLow,
    MergedOpen,
    MergedClose,
    RawBars,
    Reason,
    Detail,
    Subject,
    RelationType,
    Object,
    FactKey,
    PayloadJson,
    InputRefsJson,
    SourceCoordsJson,
    Payload,
    MergedIndex,
    RawRefs,
    DependencyRefs,
    IdentityKey,
    Revision,
    InputRevision,
    PayloadHash,
    ReceiptId,
    Seq,
    SourceNamespace,
    SourceEpoch,
    Instrument,
    EventId,
    ReceivedAt,
    RawText,
    Price,
    Ts,
    Volume,
    SourceCoord,
    SupersedesRevision,
    ObjectId,
    ObjectRevision,
    Kind,
    BatchId,
    Branch,
    DirAb,
    DirBc,
    WindowStart,
    WindowMid,
    WindowEnd,
    Comparisons,
    InputRefs,
    SourceCoords,
    FirstKnownGeneration,
    FirstKnownCut,
    PublishedGeneration,
    WithdrawnGeneration,
    WithdrawalReason,
    SupersededBy,
    Lifecycle,
}
impl Field {
    fn name(self) -> &'static str {
        match self {
            Self::OldObjectId => "old_object_id",
            Self::NewObjectId => "new_object_id",
            Self::SchemaRevision => "schema_revision",
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::WitnessId => "witness_id",
            Self::ObservationId => "observation_id",
            Self::Slot => "slot",
            Self::MergedSourceIndex => "merged_source_index",
            Self::MergedHigh => "merged_high",
            Self::MergedLow => "merged_low",
            Self::MergedOpen => "merged_open",
            Self::MergedClose => "merged_close",
            Self::RawBars => "raw_bars",
            Self::Reason => "reason",
            Self::Detail => "detail",
            Self::Subject => "subject",
            Self::RelationType => "relation_type",
            Self::Object => "object",
            Self::FactKey => "fact_key",
            Self::PayloadJson => "payload_json",
            Self::InputRefsJson => "input_refs_json",
            Self::SourceCoordsJson => "source_coords_json",
            Self::Payload => "payload",
            Self::MergedIndex => "merged_index",
            Self::RawRefs => "raw_refs",
            Self::DependencyRefs => "dependency_refs",
            Self::IdentityKey => "identity_key",
            Self::Revision => "revision",
            Self::InputRevision => "input_revision",
            Self::PayloadHash => "payload_hash",
            Self::ReceiptId => "receipt_id",
            Self::Seq => "seq",
            Self::SourceNamespace => "source_namespace",
            Self::SourceEpoch => "source_epoch",
            Self::Instrument => "instrument",
            Self::EventId => "event_id",
            Self::ReceivedAt => "received_at",
            Self::RawText => "raw_text",
            Self::Price => "price",
            Self::Ts => "ts",
            Self::Volume => "volume",
            Self::SourceCoord => "source_coord",
            Self::SupersedesRevision => "supersedes_revision",
            Self::ObjectId => "object_id",
            Self::ObjectRevision => "object_revision",
            Self::Kind => "kind",
            Self::BatchId => "batch_id",
            Self::Branch => "branch",
            Self::DirAb => "dir_ab",
            Self::DirBc => "dir_bc",
            Self::WindowStart => "window_start",
            Self::WindowMid => "window_mid",
            Self::WindowEnd => "window_end",
            Self::Comparisons => "comparisons",
            Self::InputRefs => "input_refs",
            Self::SourceCoords => "source_coords",
            Self::FirstKnownGeneration => "first_known_generation",
            Self::FirstKnownCut => "first_known_cut",
            Self::PublishedGeneration => "published_generation",
            Self::WithdrawnGeneration => "withdrawn_generation",
            Self::WithdrawalReason => "withdrawal_reason",
            Self::SupersededBy => "superseded_by",
            Self::Lifecycle => "lifecycle",
        }
    }
}
pub(crate) struct ScopedLegacyRecord<'a> {
    owner: PlainJson,
    work: &'a WorkScope,
    kind: ProjectionKind,
}
impl<'a> ScopedLegacyRecord<'a> {
    pub(crate) fn object(
        kind: ProjectionKind,
        work: &'a WorkScope,
    ) -> Result<Self, MixedJsonError> {
        let permit = work.reserve_native(0).map_err(MixedJsonError::Resource)?;
        let mut owner = PlainJson::from_reserved(Value::Object(Default::default()), Some(permit));
        owner.layout = NativeJsonLayout::ItemizedObject;
        Ok(Self { owner, work, kind })
    }
    pub(crate) fn from_owned(
        kind: ProjectionKind,
        owner: PlainJson,
        work: &'a WorkScope,
    ) -> Result<Self, MixedJsonError> {
        work.check_current().map_err(MixedJsonError::Resource)?;
        if !owner
            .permit
            .as_ref()
            .is_some_and(|p| p.belongs_to_parent(work))
        {
            return Err(MixedJsonError::Resource(
                work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger),
            ));
        }
        Ok(Self { owner, work, kind })
    }
    pub(crate) fn clone_projection(
        kind: ProjectionKind,
        value: &Value,
        work: &'a WorkScope,
    ) -> Result<Self, MixedJsonError> {
        let owner =
            PlainJson::try_clone_from(value, Some(work)).map_err(MixedJsonError::Resource)?;
        Ok(Self { owner, work, kind })
    }
    pub(crate) fn text(&mut self, field: Field, text: &str) -> Result<(), MixedJsonError> {
        let permit = self
            .work
            .reserve_native(text.len())
            .map_err(MixedJsonError::Resource)?;
        self.insert(
            field,
            PlainJson::from_reserved(Value::String(text.to_owned()), Some(permit)),
        )
    }
    pub(crate) fn integer(&mut self, field: Field, n: i64) -> Result<(), MixedJsonError> {
        let text = NativeValue::try_format(format_args!("{n}"), Some(self.work))
            .map_err(MixedJsonError::Resource)?;
        self.text(field, text.as_ref())
    }
    pub(crate) fn numeric(&mut self, field: Field, n: i64) -> Result<(), MixedJsonError> {
        let permit = self
            .work
            .reserve_native(0)
            .map_err(MixedJsonError::Resource)?;
        self.insert(
            field,
            PlainJson::from_reserved(Value::from(n), Some(permit)),
        )
    }
    pub(crate) fn borrowed(&mut self, field: Field, value: &Value) -> Result<(), MixedJsonError> {
        let child =
            PlainJson::try_clone_from(value, Some(self.work)).map_err(MixedJsonError::Resource)?;
        self.insert(field, child)
    }
    pub(crate) fn null(&mut self, field: Field) -> Result<(), MixedJsonError> {
        let permit = self
            .work
            .reserve_native(0)
            .map_err(MixedJsonError::Resource)?;
        self.insert(field, PlainJson::from_reserved(Value::Null, Some(permit)))
    }
    pub(crate) fn insert(&mut self, field: Field, child: PlainJson) -> Result<(), MixedJsonError> {
        self.insert_legacy_key(field.name(), child)
    }
    // 任意键仅用于LegacyPayload递归的原键；业务顶层入口仍是Field。
    fn insert_legacy_key(&mut self, key: &str, mut child: PlainJson) -> Result<(), MixedJsonError> {
        let work = self.work;
        work.check_current().map_err(MixedJsonError::Resource)?;
        if !child
            .permit
            .as_ref()
            .is_some_and(|p| p.belongs_to_parent(work))
        {
            return Err(MixedJsonError::Resource(
                work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger),
            ));
        }
        let object = self
            .owner
            .value
            .as_object()
            .ok_or_else(|| MixedJsonError::Business("旧域投影必须是对象".into()))?;
        if object.contains_key(key) {
            return Err(MixedJsonError::Business("已有字段须用受控替换".into()));
        }
        let slots = super::json_layout::tree_slots::<String, Value>(1)
            .checked_add(key.len())
            .ok_or_else(|| {
                MixedJsonError::Resource(
                    work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
                )
            })?;
        let slot = work
            .reserve_native(slots)
            .map_err(MixedJsonError::Resource)?;
        let count = child.child_permits.len().checked_add(2).ok_or_else(|| {
            MixedJsonError::Resource(
                work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
            )
        })?;
        self.owner
            .child_permits
            .try_reserve_native(work, count)
            .map_err(MixedJsonError::Resource)?;
        // 全部可返回的许可失败在此之前；以下只移动现成载荷及已预留许可。
        self.owner.value.as_object_mut().unwrap().insert(
            key.to_owned(),
            std::mem::replace(&mut child.value, Value::Null),
        );
        self.owner
            .child_permits
            .try_push(slot)
            .expect("已预留字段许可");
        self.owner
            .child_permits
            .try_push(child.permit.take().unwrap())
            .expect("已预留载荷许可");
        for permit in std::mem::take(&mut child.child_permits) {
            self.owner
                .child_permits
                .try_push(permit)
                .expect("已预留子许可");
        }
        Ok(())
    }
    pub(crate) fn replace_known_field(
        &mut self,
        field: Field,
        borrowed: &Value,
    ) -> Result<(), MixedJsonError> {
        let child = PlainJson::try_clone_from(borrowed, Some(self.work))
            .map_err(MixedJsonError::Resource)?;
        self.replace_owned(field, child)
    }
    pub(crate) fn replace_text(&mut self, field: Field, text: &str) -> Result<(), MixedJsonError> {
        let permit = self
            .work
            .reserve_native(text.len())
            .map_err(MixedJsonError::Resource)?;
        let child = PlainJson::from_reserved(Value::String(text.to_owned()), Some(permit));
        self.replace_owned(field, child)
    }
    pub(crate) fn replace_integer(&mut self, field: Field, n: i64) -> Result<(), MixedJsonError> {
        let text = NativeValue::try_format(format_args!("{n}"), Some(self.work))
            .map_err(MixedJsonError::Resource)?;
        let permit = self
            .work
            .reserve_native(text.len())
            .map_err(MixedJsonError::Resource)?;
        let child = PlainJson::from_reserved(Value::String(text.as_ref().to_owned()), Some(permit));
        self.replace_owned(field, child)
    }
    pub(crate) fn replace_owned(
        &mut self,
        field: Field,
        child: PlainJson,
    ) -> Result<(), MixedJsonError> {
        // 原聚合许可不猜单个子树退款；新旧完整投影在替换成功前同时持费。
        let mut next = Self::clone_projection(self.kind, &self.owner.value, self.work)?;
        let object = next
            .owner
            .value
            .as_object_mut()
            .ok_or_else(|| MixedJsonError::Business("旧域投影必须是对象".into()))?;
        object.remove(field.name());
        next.insert(field, child)?;
        self.owner = next.owner;
        Ok(())
    }
    /// 删除字段只释放实际Value；聚合许可保留到owner析构，不猜测子树退款。
    fn remove(&mut self, field: Field) -> Result<(), MixedJsonError> {
        self.work
            .check_current()
            .map_err(MixedJsonError::Resource)?;
        self.owner
            .value
            .as_object_mut()
            .expect("projection object")
            .remove(field.name());
        Ok(())
    }
    pub(crate) fn finish(self) -> PlainJson {
        self.owner
    }
}

#[derive(Debug)]
pub(crate) enum MixedSqlError {
    Sql(rusqlite::Error),
    Projection(MixedJsonError),
}
impl From<MixedJsonError> for MixedSqlError {
    fn from(error: MixedJsonError) -> Self {
        Self::Projection(error)
    }
}
impl From<rusqlite::Error> for MixedSqlError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

/// 与Row::get<String>一致的类型/UTF8错误，不提前生成owned文本。
pub(crate) fn sql_text<'row>(
    row: &'row rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<&'row str> {
    use rusqlite::{types::ValueRef, Error};
    let value = row.get_ref(index)?;
    match value {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes).map_err(|e| Error::Utf8Error(index, e)),
        other => Err(Error::InvalidColumnType(
            index,
            row.as_ref().column_name(index)?.to_owned(),
            other.data_type(),
        )),
    }
}
pub(crate) fn project_raw_history_row(row:&rusqlite::Row<'_>,work:&WorkScope)->Result<PlainJson,MixedSqlError>{
    raw_history_row(row,work,false)
}
pub(crate) fn decode_raw_history_row(row:&rusqlite::Row<'_>,work:&WorkScope)->Result<PlainJson,MixedSqlError>{
    raw_history_row(row,work,true)
}
fn raw_history_row(
    row: &rusqlite::Row<'_>,
    work: &WorkScope,
    numeric:bool,
) -> Result<PlainJson, MixedSqlError> {
    let mut out = ScopedLegacyRecord::object(ProjectionKind::RawEventWire, work)?;
    out.text(Field::IdentityKey, sql_text(row, 0)?)?;
    if numeric {out.numeric(Field::Revision,row.get(1)?)?;} else {out.integer(Field::Revision,row.get(1)?)?;}
    out.text(Field::InputRevision, sql_text(row, 2)?)?;
    out.text(Field::PayloadHash, sql_text(row, 3)?)?;
    out.text(Field::ReceiptId, sql_text(row, 4)?)?;
    if numeric {out.numeric(Field::Seq,row.get(5)?)?;} else {out.integer(Field::Seq,row.get(5)?)?;}
    out.text(Field::SourceNamespace, sql_text(row, 6)?)?;
    out.text(Field::SourceEpoch, sql_text(row, 7)?)?;
    out.text(Field::Instrument, sql_text(row, 8)?)?;
    out.text(Field::EventId, sql_text(row, 9)?)?;
    out.text(Field::ReceivedAt, sql_text(row, 10)?)?;
    out.text(Field::RawText, sql_text(row, 11)?)?;
    out.text(Field::Price, sql_text(row, 12)?)?;
    out.text(Field::Ts, sql_text(row, 13)?)?;
    out.text(Field::Volume, sql_text(row, 14)?)?;
    out.text(Field::SourceCoord, sql_text(row, 15)?)?;
    match row.get::<_, Option<i64>>(16)? {
        Some(n) => if numeric {out.numeric(Field::SupersedesRevision,n)?;} else {out.integer(Field::SupersedesRevision,n)?;},
        None => out.null(Field::SupersedesRevision)?,
    }
    Ok(out.finish())
}

/// 编码容器，不额外计业务成员；每个child已持同父许可。
pub(crate) struct ScopedLegacyArray<'a> {
    items: sc::CodecVec<PlainJson>,
    work: &'a WorkScope,
}
impl<'a> ScopedLegacyArray<'a> {
    pub(crate) fn new(work: &'a WorkScope) -> Self {
        Self {
            items: Default::default(),
            work,
        }
    }
    pub(crate) fn push(&mut self, child: PlainJson) -> Result<(), MixedJsonError> {
        if !child
            .permit
            .as_ref()
            .is_some_and(|p| p.belongs_to_parent(self.work))
        {
            return Err(MixedJsonError::Resource(
                self.work
                    .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger),
            ));
        }
        self.items
            .try_reserve_native(self.work, 1)
            .map_err(MixedJsonError::Resource)?;
        self.items.try_push(child).map_err(MixedJsonError::Resource)
    }
    pub(crate) fn finish(self) -> Result<PlainJson, MixedJsonError> {
        let fail = || {
            MixedJsonError::Resource(
                self.work
                    .reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
            )
        };
        let bytes = self
            .items
            .len()
            .checked_mul(std::mem::size_of::<Value>())
            .ok_or_else(fail)?;
        let permit = self
            .work
            .reserve_native(bytes)
            .map_err(MixedJsonError::Resource)?;
        let permits = self
            .items
            .iter()
            .try_fold(0usize, |n, item| {
                n.checked_add(item.child_permits.len())?.checked_add(1)
            })
            .ok_or_else(fail)?;
        let mut retained = sc::CodecVec::default();
        retained
            .try_reserve_native(self.work, permits)
            .map_err(MixedJsonError::Resource)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(self.items.len()).map_err(|_| {
            MixedJsonError::Resource(
                self.work
                    .reject(sc::capture_allocation::scratch::ScratchFailure::Allocation),
            )
        })?;
        for mut child in self.items {
            rows.push(std::mem::replace(&mut child.value, Value::Null));
            retained
                .try_push(child.permit.take().unwrap())
                .expect("已预留载荷许可");
            for permit in std::mem::take(&mut child.child_permits) {
                retained.try_push(permit).expect("已预留子许可");
            }
        }
        let mut owner = PlainJson::from_reserved(Value::Array(rows), Some(permit));
        owner.layout = NativeJsonLayout::ItemizedArray;
        owner.child_permits = retained;
        Ok(owner)
    }
}
fn integer_value(value: &Value) -> Result<i64, MixedJsonError> {
    value.as_i64().ok_or_else(|| {
        MixedJsonError::Business("持久整数必须是 i64，不能是浮点、布尔、空值或文本".into())
    })
}
pub(crate) fn project_input_refs(
    refs: &Value,
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    let groups = refs
        .as_array()
        .ok_or_else(|| MixedJsonError::Business("input_refs 必须是数组".into()))?;
    let mut out = ScopedLegacyArray::new(work);
    for group in groups {
        group
            .as_object()
            .ok_or_else(|| MixedJsonError::Business("input_refs 成员必须是对象".into()))?;
        let mut obj =
            ScopedLegacyRecord::clone_projection(ProjectionKind::LegacyPayload, group, work)?;
        obj.replace_integer(
            Field::MergedIndex,
            integer_value(group.get("merged_index").unwrap_or(&Value::Null))?,
        )?;
        for (key, field) in [
            ("raw_refs", Field::RawRefs),
            ("dependency_refs", Field::DependencyRefs),
        ] {
            if key == "dependency_refs" && group.get(key).is_none() {
                continue;
            }
            let raw_refs = group
                .get(key)
                .and_then(Value::as_array)
                .ok_or_else(|| MixedJsonError::Business("input_refs成员来源必须为数组".into()))?;
            let mut projected = ScopedLegacyArray::new(work);
            for raw in raw_refs {
                raw.as_object()
                    .ok_or_else(|| MixedJsonError::Business("input_refs来源成员必须对象".into()))?;
                let mut item =
                    ScopedLegacyRecord::clone_projection(ProjectionKind::LegacyPayload, raw, work)?;
                item.replace_integer(
                    Field::Seq,
                    integer_value(raw.get("seq").unwrap_or(&Value::Null))?,
                )?;
                if let Some(revision) = raw.get("revision") {
                    item.replace_integer(Field::Revision, integer_value(revision)?)?;
                }
                projected.push(item.finish())?;
            }
            obj.replace_owned(field, projected.finish()?)?;
        }
        out.push(obj.finish())?;
    }
    out.finish()
}
fn sql_json(
    row: &rusqlite::Row<'_>,
    index: usize,
    shape: crate::JsonShape,
    work: &WorkScope,
) -> Result<PlainJson, MixedSqlError> {
    let text = sql_text(row, index)?;
    match crate::json_shape_mixed(text, shape, work) {
        Ok(value) => Ok(value),
        Err(MixedJsonError::Business(error)) => {
            Err(MixedSqlError::Sql(crate::json_column_error(index, error)))
        }
        Err(error) => Err(MixedSqlError::Projection(error)),
    }
}
pub(crate) fn project_object_row(
    row: &rusqlite::Row<'_>,
    as_of: Option<i64>,
    work: &WorkScope,
) -> Result<PlainJson, MixedSqlError> {
    let wg: Option<i64> = row.get(16)?;
    let fkg: i64 = row.get(13)?;
    if let Some(w) = wg {
        if w <= fkg {
            return Err(crate::json_column_error(
                16,
                format!(
                    "对象生命周期非法：withdrawn_generation={w} <= first_known_generation={fkg}"
                ),
            )
            .into());
        }
    }
    let effective = match (as_of, wg) {
        (Some(n), Some(w)) if w > n => None,
        _ => wg,
    };
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObjectWire, work)?;
    out.text(Field::ObjectId, sql_text(row, 0)?)?;
    out.integer(Field::ObjectRevision, row.get(1)?)?;
    out.text(Field::Kind, sql_text(row, 2)?)?;
    out.text(Field::BatchId, sql_text(row, 3)?)?;
    out.text(Field::Branch, sql_text(row, 4)?)?;
    out.text(Field::DirAb, sql_text(row, 5)?)?;
    out.text(Field::DirBc, sql_text(row, 6)?)?;
    out.integer(Field::WindowStart, row.get(7)?)?;
    out.integer(Field::WindowMid, row.get(8)?)?;
    out.integer(Field::WindowEnd, row.get(9)?)?;
    out.insert(
        Field::Comparisons,
        sql_json(row, 10, crate::JsonShape::Array, work)?,
    )?;
    let refs = sql_json(row, 11, crate::JsonShape::Array, work)?;
    let refs = project_input_refs(&refs, work).map_err(|error| match error {
        MixedJsonError::Business(error) => MixedSqlError::Sql(crate::json_column_error(11, error)),
        error => MixedSqlError::Projection(error),
    })?;
    out.insert(Field::InputRefs, refs)?;
    out.insert(
        Field::SourceCoords,
        sql_json(row, 12, crate::JsonShape::Array, work)?,
    )?;
    out.integer(Field::FirstKnownGeneration, row.get(13)?)?;
    out.text(Field::FirstKnownCut, sql_text(row, 14)?)?;
    out.integer(Field::PublishedGeneration, row.get(15)?)?;
    match effective {
        Some(n) => out.integer(Field::WithdrawnGeneration, n)?,
        None => out.null(Field::WithdrawnGeneration)?,
    }
    for (index, field) in [(17, Field::WithdrawalReason), (18, Field::SupersededBy)] {
        if effective.is_some() && !matches!(row.get_ref(index)?, rusqlite::types::ValueRef::Null) {
            out.text(field, sql_text(row, index)?)?;
        } else {
            out.null(field)?;
        }
    }
    out.text(
        Field::Lifecycle,
        if effective.is_some() {
            "withdrawn"
        } else {
            "active"
        },
    )?;
    Ok(out.finish())
}

/// 第一阶段仅取得SQL文本，三列JSON在最终typed project之前不解析。
pub(crate) fn collect_fact_row(
    row: &rusqlite::Row<'_>,
    work: &WorkScope,
) -> Result<PlainJson, MixedSqlError> {
    let mut out = ScopedLegacyRecord::object(ProjectionKind::LegacyPayload, work)?;
    out.text(Field::ObjectId, sql_text(row, 0)?)?;
    out.numeric(Field::ObjectRevision, row.get(1)?)?;
    out.text(Field::Kind, sql_text(row, 2)?)?;
    out.text(Field::BatchId, sql_text(row, 3)?)?;
    out.text(Field::FactKey, sql_text(row, 4)?)?;
    out.text(Field::PayloadJson, sql_text(row, 5)?)?;
    out.text(Field::InputRefsJson, sql_text(row, 6)?)?;
    out.text(Field::SourceCoordsJson, sql_text(row, 7)?)?;
    out.numeric(Field::FirstKnownGeneration, row.get(8)?)?;
    out.text(Field::FirstKnownCut, sql_text(row, 9)?)?;
    out.numeric(Field::PublishedGeneration, row.get(10)?)?;
    match row.get::<_, Option<i64>>(11)? {
        Some(n) => out.numeric(Field::WithdrawnGeneration, n)?,
        None => out.null(Field::WithdrawnGeneration)?,
    }
    for (i, field) in [(12, Field::WithdrawalReason), (13, Field::SupersededBy)] {
        if matches!(row.get_ref(i)?, rusqlite::types::ValueRef::Null) {
            out.null(field)?;
        } else {
            out.text(field, sql_text(row, i)?)?;
        }
    }
    Ok(out.finish())
}

/// 最终投影才解析三列；复用原kind/数字/JSON核，保持每个读取和首错的先后。
pub(crate) fn project_fact(o: &Value, work: &WorkScope) -> Result<PlainJson, MixedJsonError> {
    let kind = crate::tb02a::checked_fact_kind(o).map_err(MixedJsonError::Business)?;
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObjectWire, work)?;
    out.borrowed(Field::ObjectId, &o["object_id"])?;
    let n = crate::exact_wire_integer(&o["object_revision"]).map_err(MixedJsonError::Business)?;
    out.integer(Field::ObjectRevision, n)?;
    out.text(Field::Kind, kind)?;
    out.borrowed(Field::BatchId, o.get("batch_id").unwrap_or(&Value::Null))?;
    out.borrowed(Field::FactKey, &o["fact_key"])?;
    for (field, key, message, shape) in [
        (
            Field::Payload,
            "payload_json",
            "StorageUnavailable：事实payload缺失",
            crate::JsonShape::Object,
        ),
        (
            Field::InputRefs,
            "input_refs_json",
            "StorageUnavailable：事实来源缺失",
            crate::JsonShape::Array,
        ),
        (
            Field::SourceCoords,
            "source_coords_json",
            "StorageUnavailable：事实坐标缺失",
            crate::JsonShape::Array,
        ),
    ] {
        let text = o[key]
            .as_str()
            .ok_or_else(|| MixedJsonError::Business(message.into()))?;
        out.insert(field, crate::json_shape_mixed(text, shape, work)?)?;
    }
    let n = crate::exact_wire_integer(&o["first_known_generation"])
        .map_err(MixedJsonError::Business)?;
    out.integer(Field::FirstKnownGeneration, n)?;
    out.borrowed(Field::FirstKnownCut, &o["first_known_cut"])?;
    let n =
        crate::exact_wire_integer(&o["published_generation"]).map_err(MixedJsonError::Business)?;
    out.integer(Field::PublishedGeneration, n)?;
    out.null(Field::WithdrawnGeneration)?;
    out.borrowed(
        Field::WithdrawalReason,
        o.get("withdrawal_reason").unwrap_or(&Value::Null),
    )?;
    out.borrowed(
        Field::SupersededBy,
        o.get("superseded_by").unwrap_or(&Value::Null),
    )?;
    out.text(Field::Lifecycle, "active")?;
    if !o["withdrawn_generation"].is_null() {
        let n = crate::exact_wire_integer(&o["withdrawn_generation"])
            .map_err(MixedJsonError::Business)?;
        out.replace_integer(Field::WithdrawnGeneration, n)?;
        out.replace_text(Field::Lifecycle, "withdrawn")?;
    }
    Ok(out.finish())
}

/// 第三阶段：沿原生命周期/as_of核过滤；被过滤的坏JSON不解析。
pub(crate) fn filter_fact_lifecycle(
    owner: PlainJson,
    as_of: Option<i64>,
    work: &WorkScope,
) -> Result<Option<PlainJson>, MixedJsonError> {
    match crate::tb02a::fact_lifecycle(&owner, as_of).map_err(MixedJsonError::Business)? {
        crate::tb02a::FactLifecycle::Skip => Ok(None),
        crate::tb02a::FactLifecycle::Keep => Ok(Some(owner)),
        crate::tb02a::FactLifecycle::ClearDeath => {
            let mut out =
                ScopedLegacyRecord::from_owned(ProjectionKind::AtPreviousLifecycle, owner, work)?;
            for field in [
                Field::WithdrawnGeneration,
                Field::WithdrawalReason,
                Field::SupersededBy,
            ] {
                out.replace_known_field(field, &Value::Null)?;
            }
            Ok(Some(out.finish()))
        }
    }
}

/// 原raw_bars投影：先对象、seq、可选revision、可选supersedes，再typed raw_wire。
pub(crate) fn project_raw_bars(
    bars: &Value,
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    let bars = bars
        .as_array()
        .ok_or_else(|| MixedJsonError::Business("raw_bars 必须是数组".into()))?;
    let mut out = ScopedLegacyArray::new(work);
    for bar in bars {
        bar.as_object()
            .ok_or_else(|| MixedJsonError::Business("raw_bars 成员必须是对象".into()))?;
        let mut item =
            ScopedLegacyRecord::clone_projection(ProjectionKind::RawEventWire, bar, work)?;
        item.replace_integer(Field::Seq, integer_value(&bar["seq"])?)?;
        if bar.get("revision").is_some() {
            item.replace_integer(Field::Revision, integer_value(&bar["revision"])?)?;
        }
        if let Some(sup) = bar.get("supersedes_revision") {
            if let Some(n) = sup.as_i64() {
                item.replace_integer(Field::SupersedesRevision, n)?;
            } else if !sup.is_null() {
                return Err(MixedJsonError::Business(
                    "raw_bars.supersedes_revision 必须是 i64 或 null".into(),
                ));
            }
        }
        // 原raw_wire对其它字段赋回原值；clone已保留完全相同的字段，只有删除price改变内容。
        if bar["schema_revision"] == crate::tb02a::RAW_SCHEMA {
            item.remove(Field::Price)?;
            for (key, field) in [
                ("schema_revision", Field::SchemaRevision),
                ("open", Field::Open),
                ("high", Field::High),
                ("low", Field::Low),
                ("close", Field::Close),
            ] {
                if bar.get(key).is_none() {
                    item.null(field)?;
                }
            }
        }
        out.push(item.finish())?;
    }
    out.finish()
}
pub(crate) fn project_family_row(
    row: &rusqlite::Row<'_>,
    name: &str,
    work: &WorkScope,
) -> Result<(NativeValue<String>, i64, PlainJson), MixedSqlError> {
    let (id, generation, record) = match name {
        "witnesses" => {
            let id = NativeValue::try_from_str(sql_text(row, 0)?, Some(work))
                .map_err(MixedJsonError::Resource)?;
            let generation = row.get(1)?;
            let mut out = ScopedLegacyRecord::object(ProjectionKind::WitnessWire, work)?;
            out.text(Field::WitnessId, sql_text(row, 0)?)?;
            out.text(Field::ObjectId, sql_text(row, 2)?)?;
            out.integer(Field::Slot, row.get(3)?)?;
            out.integer(Field::MergedSourceIndex, row.get(4)?)?;
            out.text(Field::MergedHigh, sql_text(row, 5)?)?;
            out.text(Field::MergedLow, sql_text(row, 6)?)?;
            out.text(Field::MergedOpen, sql_text(row, 7)?)?;
            out.text(Field::MergedClose, sql_text(row, 8)?)?;
            let bars = sql_json(row, 9, crate::JsonShape::Array, work)?;
            let bars = project_raw_bars(&bars, work).map_err(|error| match error {
                MixedJsonError::Business(error) => {
                    MixedSqlError::Sql(crate::json_column_error(9, error))
                }
                error => MixedSqlError::Projection(error),
            })?;
            out.insert(Field::RawBars, bars)?;
            (id, generation, out.finish())
        }
        "observations" => {
            let id = NativeValue::try_from_str(sql_text(row, 0)?, Some(work))
                .map_err(MixedJsonError::Resource)?;
            let generation = row.get(1)?;
            let mut out = ScopedLegacyRecord::object(ProjectionKind::ObservationWire, work)?;
            out.text(Field::ObservationId, sql_text(row, 0)?)?;
            out.text(Field::BatchId, sql_text(row, 2)?)?;
            out.text(Field::Kind, sql_text(row, 3)?)?;
            for (index, field) in [
                (4, Field::WindowStart),
                (5, Field::WindowMid),
                (6, Field::WindowEnd),
            ] {
                match row.get::<_, Option<i64>>(index)? {
                    Some(n) => out.numeric(field, n)?,
                    None => out.null(field)?,
                }
            }
            out.text(Field::Reason, sql_text(row, 7)?)?;
            out.insert(
                Field::Detail,
                sql_json(row, 8, crate::JsonShape::Object, work)?,
            )?;
            (id, generation, out.finish())
        }
        "relations" => {
            // 原入口先读完三个文本，再规范化身份，随后读取generation。
            let subject = NativeValue::try_from_str(sql_text(row, 0)?, Some(work))
                .map_err(MixedJsonError::Resource)?;
            let relation = NativeValue::try_from_str(sql_text(row, 1)?, Some(work))
                .map_err(MixedJsonError::Resource)?;
            let object = NativeValue::try_from_str(sql_text(row, 2)?, Some(work))
                .map_err(MixedJsonError::Resource)?;
            let mut out = ScopedLegacyRecord::object(ProjectionKind::Relation, work)?;
            out.text(Field::Subject, subject.as_ref())?;
            out.text(Field::RelationType, relation.as_ref())?;
            out.text(Field::Object, object.as_ref())?;
            let record = out.finish();
            let id = NativeValue::try_format(format_args!("{}", record.as_ref()), Some(work))
                .map_err(MixedJsonError::Resource)?;
            let generation = row.get(3)?;
            (id, generation, record)
        }
        _ => return Err(MixedJsonError::Business("StorageUnavailable：未知索引族".into()).into()),
    };
    Ok((id, generation, record))
}

/// LegacyPayload保持原递归转换边界：detail全整数投影，raw_bars只转换supersedes_revision。
pub(crate) fn legacy_payload(value: &Value, work: &WorkScope) -> Result<PlainJson, MixedJsonError> {
    match value {
        Value::Object(fields) => {
            let mut out = ScopedLegacyRecord::object(ProjectionKind::LegacyPayload, work)?;
            for (key, value) in fields {
                let projected = if key == "detail" {
                    wire_integers(value, work)?
                } else if key == "raw_bars" {
                    if let Some(bars) = value.as_array() {
                        let mut out = ScopedLegacyArray::new(work);
                        for bar in bars {
                            let mut projected = ScopedLegacyRecord::clone_projection(
                                ProjectionKind::LegacyPayload,
                                bar,
                                work,
                            )?;
                            if let Some(sup) = bar.get("supersedes_revision") {
                                projected.replace_owned(
                                    Field::SupersedesRevision,
                                    wire_integers(sup, work)?,
                                )?;
                            }
                            out.push(projected.finish())?;
                        }
                        out.finish()?
                    } else {
                        PlainJson::try_clone_from(value, Some(work))
                            .map_err(MixedJsonError::Resource)?
                    }
                } else {
                    legacy_payload(value, work)?
                };
                out.insert_legacy_key(key, projected)?;
            }
            Ok(out.finish())
        }
        Value::Array(values) => {
            let mut out = ScopedLegacyArray::new(work);
            for value in values {
                out.push(legacy_payload(value, work)?)?;
            }
            out.finish()
        }
        _ => PlainJson::try_clone_from(value, Some(work)).map_err(MixedJsonError::Resource),
    }
}
fn wire_integers(value: &Value, work: &WorkScope) -> Result<PlainJson, MixedJsonError> {
    if let Value::Number(n) = value {
        let text = if let Some(i) = n.as_i64() {
            Some(NativeValue::try_format(format_args!("{i}"), Some(work)))
        } else {
            n.as_u64()
                .map(|u| NativeValue::try_format(format_args!("{u}"), Some(work)))
        };
        if let Some(text) = text {
            let text = text.map_err(MixedJsonError::Resource)?;
            let permit = work
                .reserve_native(text.as_ref().len())
                .map_err(MixedJsonError::Resource)?;
            return Ok(PlainJson::from_reserved(
                Value::String(text.as_ref().clone()),
                Some(permit),
            ));
        }
    }
    match value {
        Value::Object(fields) => {
            let mut out = ScopedLegacyRecord::object(ProjectionKind::LegacyPayload, work)?;
            for (key, value) in fields {
                out.insert_legacy_key(key, wire_integers(value, work)?)?;
            }
            Ok(out.finish())
        }
        Value::Array(values) => {
            let mut out = ScopedLegacyArray::new(work);
            for value in values {
                out.push(wire_integers(value, work)?)?;
            }
            out.finish()
        }
        _ => PlainJson::try_clone_from(value, Some(work)).map_err(MixedJsonError::Resource),
    }
}

pub(crate) fn normalize_records<'a>(
    records: impl IntoIterator<Item = &'a Value>,
    field: &str,
    family: &'static str,
    work: &WorkScope,
    observer: &dyn crate::v3::audit_budget::LegacyObserver,
) -> Result<sc::CodecVec<NativeValue<String>>, MixedJsonError> {
    use crate::v3::audit_budget::LegacyRecordRef;
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    let mut out = sc::CodecVec::default();
    for record in records {
        observer
            .visit_record(family, "normalize", LegacyRecordRef::Json(record))
            .map_err(MixedJsonError::Resource)?;
        if !record.is_object() {
            return Err(MixedJsonError::Business(
                "StorageUnavailable：集合成员非对象".into(),
            ));
        }
        let projected = legacy_payload(record, work)?;
        let text = NativeValue::try_format(format_args!("{}", projected.as_ref()), Some(work))
            .map_err(MixedJsonError::Resource)?;
        out.try_reserve_native(work, 1)
            .map_err(MixedJsonError::Resource)?;
        out.try_push(text).map_err(MixedJsonError::Resource)?;
    }
    let sort_bytes = out
        .len()
        .checked_mul(std::mem::size_of::<NativeValue<String>>())
        .ok_or_else(|| {
            MixedJsonError::Resource(
                work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
            )
        })?;
    let sort_permit = work
        .reserve_native(sort_bytes)
        .map_err(MixedJsonError::Resource)?;
    out.sort();
    drop(sort_permit);
    if out.windows(2).any(|w| w[0] == w[1]) {
        return Err(MixedJsonError::Business(format!(
            "StorageUnavailable：{field} 重复成员"
        )));
    }
    Ok(out)
}
pub(crate) fn compare_normalized(
    actual: &[NativeValue<String>],
    expected: &[NativeValue<String>],
    field: &str,
    family: &'static str,
    observer: &dyn crate::v3::audit_budget::LegacyObserver,
) -> Result<(), MixedJsonError> {
    use crate::v3::audit_budget::LegacyRecordRef;
    let mismatch = || {
        MixedJsonError::Business(format!(
            "StorageUnavailable：{field} 与封存批次/同 cut 持久快照不一致"
        ))
    };
    if actual.len() != expected.len() {
        return Err(mismatch());
    }
    for (actual, expected) in actual.iter().zip(expected) {
        observer
            .visit_record(
                family,
                "compare",
                LegacyRecordRef::Canonical(actual.as_ref()),
            )
            .map_err(MixedJsonError::Resource)?;
        observer
            .visit_record(
                family,
                "compare",
                LegacyRecordRef::Canonical(expected.as_ref()),
            )
            .map_err(MixedJsonError::Resource)?;
        if actual != expected {
            return Err(mismatch());
        }
    }
    Ok(())
}

pub(crate) fn project_object_wire(
    value: &Value,
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    if crate::tb02a::typed(value) {
        return project_fact(value, work);
    }
    let comparisons = crate::json_shape_mixed(
        value["comparisons_json"].as_str().unwrap_or("[]"),
        crate::JsonShape::Array,
        work,
    )?;
    let refs = crate::json_shape_mixed(
        value["input_refs_json"].as_str().unwrap_or("[]"),
        crate::JsonShape::Array,
        work,
    )?;
    let coords = crate::json_shape_mixed(
        value["source_coords_json"].as_str().unwrap_or("[]"),
        crate::JsonShape::Array,
        work,
    )?;
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObjectWire, work)?;
    out.borrowed(Field::ObjectId, &value["object_id"])?;
    out.integer(
        Field::ObjectRevision,
        integer_value(&value["object_revision"])?,
    )?;
    out.borrowed(Field::Kind, &value["kind"])?;
    out.borrowed(Field::BatchId, &value["batch_id"])?;
    out.borrowed(Field::Branch, &value["branch"])?;
    out.borrowed(Field::DirAb, &value["dir_ab"])?;
    out.borrowed(Field::DirBc, &value["dir_bc"])?;
    for (key, field) in [
        ("window_start", Field::WindowStart),
        ("window_mid", Field::WindowMid),
        ("window_end", Field::WindowEnd),
    ] {
        out.integer(field, integer_value(&value[key])?)?;
    }
    out.insert(Field::Comparisons, comparisons)?;
    out.insert(Field::InputRefs, project_input_refs(&refs, work)?)?;
    out.insert(Field::SourceCoords, coords)?;
    out.integer(
        Field::FirstKnownGeneration,
        integer_value(&value["first_known_generation"])?,
    )?;
    out.borrowed(Field::FirstKnownCut, &value["first_known_cut"])?;
    out.integer(
        Field::PublishedGeneration,
        integer_value(&value["published_generation"])?,
    )?;
    match value["withdrawn_generation"].as_i64() {
        Some(n) => out.integer(Field::WithdrawnGeneration, n)?,
        None => out.null(Field::WithdrawnGeneration)?,
    }
    out.borrowed(Field::WithdrawalReason, &value["withdrawal_reason"])?;
    out.borrowed(Field::SupersededBy, &value["superseded_by"])?;
    out.text(
        Field::Lifecycle,
        if value["withdrawn_generation"].as_i64().is_some() {
            "withdrawn"
        } else {
            "active"
        },
    )?;
    Ok(out.finish())
}
pub(crate) fn project_witness_wire(
    value: &Value,
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    let bars = PlainJson::try_clone_from(&value["raw_bars"], Some(work))
        .map_err(MixedJsonError::Resource)?;
    let mut out = ScopedLegacyRecord::object(ProjectionKind::WitnessWire, work)?;
    out.borrowed(Field::WitnessId, &value["witness_id"])?;
    out.borrowed(Field::ObjectId, &value["object_id"])?;
    out.integer(Field::Slot, integer_value(&value["slot"])?)?;
    out.integer(
        Field::MergedSourceIndex,
        integer_value(&value["merged_source_index"])?,
    )?;
    for (key, field) in [
        ("merged_high", Field::MergedHigh),
        ("merged_low", Field::MergedLow),
        ("merged_open", Field::MergedOpen),
        ("merged_close", Field::MergedClose),
    ] {
        out.borrowed(field, &value[key])?;
    }
    out.insert(Field::RawBars, project_raw_bars(&bars, work)?)?;
    Ok(out.finish())
}
pub(crate) fn project_observation_wire(
    value: &Value,
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObservationWire, work)?;
    out.borrowed(Field::ObservationId, &value["observation_id"])?;
    out.borrowed(Field::BatchId, &value["batch_id"])?;
    out.borrowed(Field::Kind, &value["kind"])?;
    for (key, field) in [
        ("window_start", Field::WindowStart),
        ("window_mid", Field::WindowMid),
        ("window_end", Field::WindowEnd),
    ] {
        if value[key].is_null() {
            out.null(field)?;
        } else {
            out.integer(field, integer_value(&value[key])?)?;
        }
    }
    out.borrowed(Field::Reason, &value["reason"])?;
    out.borrowed(Field::Detail, &value["detail"])?;
    Ok(out.finish())
}
pub(crate) fn project_raw_event_wire(
    value: &Value,
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    let mut out = ScopedLegacyRecord::object(ProjectionKind::RawEventWire, work)?;
    out.borrowed(Field::IdentityKey, &value["identity_key"])?;
    out.integer(Field::Revision, integer_value(&value["revision"])?)?;
    for (key, field) in [
        ("input_revision", Field::InputRevision),
        ("payload_hash", Field::PayloadHash),
        ("receipt_id", Field::ReceiptId),
    ] {
        out.borrowed(field, &value[key])?;
    }
    out.integer(Field::Seq, integer_value(&value["seq"])?)?;
    for (key, field) in [
        ("source_namespace", Field::SourceNamespace),
        ("source_epoch", Field::SourceEpoch),
        ("instrument", Field::Instrument),
        ("event_id", Field::EventId),
        ("received_at", Field::ReceivedAt),
        ("raw_text", Field::RawText),
        ("price", Field::Price),
        ("ts", Field::Ts),
        ("volume", Field::Volume),
        ("source_coord", Field::SourceCoord),
    ] {
        out.borrowed(field, &value[key])?;
    }
    match value["supersedes_revision"].as_i64() {
        Some(n) => out.integer(Field::SupersedesRevision, n)?,
        None => out.null(Field::SupersedesRevision)?,
    }
    if value["schema_revision"] == crate::tb02a::RAW_SCHEMA {
        out.remove(Field::Price)?;
        for (key, field) in [
            ("schema_revision", Field::SchemaRevision),
            ("open", Field::Open),
            ("high", Field::High),
            ("low", Field::Low),
            ("close", Field::Close),
        ] {
            out.borrowed(field, &value[key])?;
        }
    }
    Ok(out.finish())
}
