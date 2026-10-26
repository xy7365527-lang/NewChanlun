//! #1527：旧域投影拥有者；键为具名字段，源值只借用，载荷不开放裸修改。
use super::{MixedJsonError, NativeJsonLayout, PlainJson, WorkScope, Value};
use newchan_rust::s_content::{self as sc, capture_allocation::native_owned::NativeValue};

#[derive(Clone, Copy)]
pub(crate) enum ProjectionKind {
    ObjectWire, WitnessWire, ObservationWire, RawEventWire, LegacyPayload,
    AtPreviousLifecycle, IndexBirth, Withdrawal, Relation,
}
#[derive(Clone, Copy)]
pub(crate) enum Field {
    FactKey, PayloadJson, InputRefsJson, SourceCoordsJson, Payload,
    MergedIndex, RawRefs, DependencyRefs,
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
    pub(crate) fn object(kind: ProjectionKind, work: &'a WorkScope) -> Result<Self, MixedJsonError> {
        let permit = work.reserve_native(0).map_err(MixedJsonError::Resource)?;
        let mut owner = PlainJson::from_reserved(Value::Object(Default::default()), Some(permit));
        owner.layout = NativeJsonLayout::ItemizedObject;
        Ok(Self {owner, work, kind})
    }
    pub(crate) fn clone_projection(kind: ProjectionKind, value: &Value, work: &'a WorkScope) -> Result<Self, MixedJsonError> {
        let owner = PlainJson::try_clone_from(value, Some(work)).map_err(MixedJsonError::Resource)?;
        Ok(Self { owner, work, kind })
    }
    pub(crate) fn text(&mut self, field: Field, text: &str) -> Result<(), MixedJsonError> {
        let permit = self.work.reserve_native(text.len()).map_err(MixedJsonError::Resource)?;
        self.insert(field, PlainJson::from_reserved(Value::String(text.to_owned()), Some(permit)))
    }
    pub(crate) fn integer(&mut self, field: Field, n: i64) -> Result<(), MixedJsonError> {
        let text = NativeValue::try_format(format_args!("{n}"), Some(self.work)).map_err(MixedJsonError::Resource)?;
        self.text(field, text.as_ref())
    }
    pub(crate) fn numeric(&mut self, field: Field, n: i64) -> Result<(), MixedJsonError> {
        let permit = self.work.reserve_native(0).map_err(MixedJsonError::Resource)?;
        self.insert(field, PlainJson::from_reserved(Value::from(n), Some(permit)))
    }
    pub(crate) fn borrowed(&mut self, field: Field, value: &Value) -> Result<(), MixedJsonError> {
        let child = PlainJson::try_clone_from(value, Some(self.work)).map_err(MixedJsonError::Resource)?;
        self.insert(field, child)
    }
    pub(crate) fn null(&mut self, field: Field) -> Result<(), MixedJsonError> {
        let permit = self.work.reserve_native(0).map_err(MixedJsonError::Resource)?;
        self.insert(field, PlainJson::from_reserved(Value::Null, Some(permit)))
    }
    pub(crate) fn insert(&mut self, field: Field, mut child: PlainJson) -> Result<(), MixedJsonError> {
        let work = self.work;
        work.check_current().map_err(MixedJsonError::Resource)?;
        if !child.permit.as_ref().is_some_and(|p| p.belongs_to_parent(work)) {
            return Err(MixedJsonError::Resource(work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)));
        }
        let key = field.name();
        let object = self.owner.value.as_object().ok_or_else(|| MixedJsonError::Business("旧域投影必须是对象".into()))?;
        if object.contains_key(key) {
            return Err(MixedJsonError::Business("已有字段须用受控替换".into()));
        }
        let slots = super::json_layout::tree_slots::<String,Value>(1).checked_add(key.len())
            .ok_or_else(|| MixedJsonError::Resource(work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget)))?;
        let slot = work.reserve_native(slots).map_err(MixedJsonError::Resource)?;
        let count = child.child_permits.len().checked_add(2)
            .ok_or_else(|| MixedJsonError::Resource(work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget)))?;
        self.owner.child_permits.try_reserve_native(work, count).map_err(MixedJsonError::Resource)?;
        // 全部可返回的许可失败在此之前；以下只移动现成载荷及已预留许可。
        self.owner.value.as_object_mut().unwrap().insert(key.to_owned(), std::mem::replace(&mut child.value, Value::Null));
        self.owner.child_permits.try_push(slot).expect("已预留字段许可");
        self.owner.child_permits.try_push(child.permit.take().unwrap()).expect("已预留载荷许可");
        for permit in std::mem::take(&mut child.child_permits) {
            self.owner.child_permits.try_push(permit).expect("已预留子许可");
        }
        Ok(())
    }
    pub(crate) fn replace_known_field(&mut self, field: Field, borrowed: &Value) -> Result<(), MixedJsonError> {
        let child = PlainJson::try_clone_from(borrowed, Some(self.work)).map_err(MixedJsonError::Resource)?;
        self.replace_owned(field, child)
    }
    pub(crate) fn replace_text(&mut self, field: Field, text: &str) -> Result<(), MixedJsonError> {
        let permit = self.work.reserve_native(text.len()).map_err(MixedJsonError::Resource)?;
        let child = PlainJson::from_reserved(Value::String(text.to_owned()), Some(permit));
        self.replace_owned(field, child)
    }
    pub(crate) fn replace_integer(&mut self, field: Field, n: i64) -> Result<(), MixedJsonError> {
        let text = NativeValue::try_format(format_args!("{n}"), Some(self.work)).map_err(MixedJsonError::Resource)?;
        let permit = self.work.reserve_native(text.len()).map_err(MixedJsonError::Resource)?;
        let child = PlainJson::from_reserved(Value::String(text.as_ref().to_owned()), Some(permit));
        self.replace_owned(field, child)
    }
    pub(crate) fn replace_owned(&mut self, field: Field, child: PlainJson) -> Result<(), MixedJsonError> {
        // 原聚合许可不猜单个子树退款；新旧完整投影在替换成功前同时持费。
        let mut next = Self::clone_projection(self.kind, &self.owner.value, self.work)?;
        let object = next.owner.value.as_object_mut().ok_or_else(|| MixedJsonError::Business("旧域投影必须是对象".into()))?;
        object.remove(field.name());
        next.insert(field, child)?;
        self.owner = next.owner;
        Ok(())
    }
    pub(crate) fn finish(self) -> PlainJson { self.owner }
}

#[derive(Debug)]
pub(crate) enum MixedSqlError {
    Sql(rusqlite::Error),
    Projection(MixedJsonError),
}
impl From<MixedJsonError> for MixedSqlError {
    fn from(error: MixedJsonError) -> Self { Self::Projection(error) }
}
impl From<rusqlite::Error> for MixedSqlError {
    fn from(error: rusqlite::Error) -> Self { Self::Sql(error) }
}

/// 与Row::get<String>一致的类型/UTF8错误，不提前生成owned文本。
pub(crate) fn sql_text<'row>(row: &'row rusqlite::Row<'_>, index: usize) -> rusqlite::Result<&'row str> {
    use rusqlite::{types::ValueRef, Error};
    let value = row.get_ref(index)?;
    match value {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes)
            .map_err(|e| Error::Utf8Error(index, e)),
        other => Err(Error::InvalidColumnType(index,
            row.as_ref().column_name(index)?.to_owned(), other.data_type())),
    }
}
pub(crate) fn project_raw_history_row(row: &rusqlite::Row<'_>, work: &WorkScope) -> Result<PlainJson, MixedSqlError> {
    let mut out = ScopedLegacyRecord::object(ProjectionKind::RawEventWire, work)?;
    out.text(Field::IdentityKey, sql_text(row,0)?)?;
    out.integer(Field::Revision, row.get(1)?)?;
    out.text(Field::InputRevision, sql_text(row,2)?)?;
    out.text(Field::PayloadHash, sql_text(row,3)?)?;
    out.text(Field::ReceiptId, sql_text(row,4)?)?;
    out.integer(Field::Seq, row.get(5)?)?;
    out.text(Field::SourceNamespace, sql_text(row,6)?)?;
    out.text(Field::SourceEpoch, sql_text(row,7)?)?;
    out.text(Field::Instrument, sql_text(row,8)?)?;
    out.text(Field::EventId, sql_text(row,9)?)?;
    out.text(Field::ReceivedAt, sql_text(row,10)?)?;
    out.text(Field::RawText, sql_text(row,11)?)?;
    out.text(Field::Price, sql_text(row,12)?)?;
    out.text(Field::Ts, sql_text(row,13)?)?;
    out.text(Field::Volume, sql_text(row,14)?)?;
    out.text(Field::SourceCoord, sql_text(row,15)?)?;
    match row.get::<_,Option<i64>>(16)? {
        Some(n) => out.integer(Field::SupersedesRevision,n)?,
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
    pub(crate) fn new(work: &'a WorkScope) -> Self { Self { items: Default::default(), work } }
    pub(crate) fn push(&mut self, child: PlainJson) -> Result<(), MixedJsonError> {
        if !child.permit.as_ref().is_some_and(|p| p.belongs_to_parent(self.work)) {
            return Err(MixedJsonError::Resource(self.work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)));
        }
        self.items.try_reserve_native(self.work,1).map_err(MixedJsonError::Resource)?;
        self.items.try_push(child).map_err(MixedJsonError::Resource)
    }
    pub(crate) fn finish(self) -> Result<PlainJson, MixedJsonError> {
        let fail = || MixedJsonError::Resource(self.work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget));
        let bytes = self.items.len().checked_mul(std::mem::size_of::<Value>()).ok_or_else(fail)?;
        let permit = self.work.reserve_native(bytes).map_err(MixedJsonError::Resource)?;
        let permits = self.items.iter().try_fold(0usize, |n,item| n.checked_add(item.child_permits.len())?.checked_add(1)).ok_or_else(fail)?;
        let mut retained = sc::CodecVec::default();
        retained.try_reserve_native(self.work, permits).map_err(MixedJsonError::Resource)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(self.items.len()).map_err(|_| MixedJsonError::Resource(self.work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)))?;
        for mut child in self.items {
            rows.push(std::mem::replace(&mut child.value, Value::Null));
            retained.try_push(child.permit.take().unwrap()).expect("已预留载荷许可");
            for permit in std::mem::take(&mut child.child_permits) {
                retained.try_push(permit).expect("已预留子许可");
            }
        }
        let mut owner = PlainJson::from_reserved(Value::Array(rows),Some(permit));
        owner.layout = NativeJsonLayout::ItemizedArray;
        owner.child_permits = retained;
        Ok(owner)
    }
}
fn integer_value(value: &Value) -> Result<i64,MixedJsonError> {
    value.as_i64().ok_or_else(|| MixedJsonError::Business("持久整数必须是 i64，不能是浮点、布尔、空值或文本".into()))
}
pub(crate) fn project_input_refs(refs: &Value, work: &WorkScope) -> Result<PlainJson,MixedJsonError> {
    let groups = refs.as_array().ok_or_else(|| MixedJsonError::Business("input_refs 必须是数组".into()))?;
    let mut out = ScopedLegacyArray::new(work);
    for group in groups {
        group.as_object().ok_or_else(|| MixedJsonError::Business("input_refs 成员必须是对象".into()))?;
        let mut obj = ScopedLegacyRecord::clone_projection(ProjectionKind::LegacyPayload, group, work)?;
        obj.replace_integer(Field::MergedIndex, integer_value(group.get("merged_index").unwrap_or(&Value::Null))?)?;
        for (key,field) in [("raw_refs",Field::RawRefs),("dependency_refs",Field::DependencyRefs)] {
            if key == "dependency_refs" && group.get(key).is_none() { continue; }
            let raw_refs = group.get(key).and_then(Value::as_array).ok_or_else(|| MixedJsonError::Business("input_refs成员来源必须为数组".into()))?;
            let mut projected = ScopedLegacyArray::new(work);
            for raw in raw_refs {
                raw.as_object().ok_or_else(|| MixedJsonError::Business("input_refs来源成员必须对象".into()))?;
                let mut item = ScopedLegacyRecord::clone_projection(ProjectionKind::LegacyPayload,raw,work)?;
                item.replace_integer(Field::Seq, integer_value(raw.get("seq").unwrap_or(&Value::Null))?)?;
                if let Some(revision) = raw.get("revision") {
                    item.replace_integer(Field::Revision,integer_value(revision)?)?;
                }
                projected.push(item.finish())?;
            }
            obj.replace_owned(field,projected.finish()?)?;
        }
        out.push(obj.finish())?;
    }
    out.finish()
}
fn sql_json(row: &rusqlite::Row<'_>, index: usize, shape: crate::JsonShape, work: &WorkScope) -> Result<PlainJson,MixedSqlError> {
    let text = sql_text(row,index)?;
    match crate::json_shape_mixed(text,shape,work) {
        Ok(value) => Ok(value),
        Err(MixedJsonError::Business(error)) => Err(MixedSqlError::Sql(crate::json_column_error(index,error))),
        Err(error) => Err(MixedSqlError::Projection(error)),
    }
}
pub(crate) fn project_object_row(row: &rusqlite::Row<'_>, as_of: Option<i64>, work: &WorkScope) -> Result<PlainJson,MixedSqlError> {
    let wg: Option<i64> = row.get(16)?;
    let fkg: i64 = row.get(13)?;
    if let Some(w) = wg {
        if w <= fkg {
            return Err(crate::json_column_error(16,format!("对象生命周期非法：withdrawn_generation={w} <= first_known_generation={fkg}")).into());
        }
    }
    let effective = match (as_of,wg) { (Some(n),Some(w)) if w > n => None, _ => wg };
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObjectWire,work)?;
    out.text(Field::ObjectId,sql_text(row,0)?)?;
    out.integer(Field::ObjectRevision,row.get(1)?)?;
    out.text(Field::Kind,sql_text(row,2)?)?;
    out.text(Field::BatchId,sql_text(row,3)?)?;
    out.text(Field::Branch,sql_text(row,4)?)?;
    out.text(Field::DirAb,sql_text(row,5)?)?;
    out.text(Field::DirBc,sql_text(row,6)?)?;
    out.integer(Field::WindowStart,row.get(7)?)?;
    out.integer(Field::WindowMid,row.get(8)?)?;
    out.integer(Field::WindowEnd,row.get(9)?)?;
    out.insert(Field::Comparisons,sql_json(row,10,crate::JsonShape::Array,work)?)?;
    let refs = sql_json(row,11,crate::JsonShape::Array,work)?;
    let refs = project_input_refs(&refs,work).map_err(|error| match error {
        MixedJsonError::Business(error) => MixedSqlError::Sql(crate::json_column_error(11,error)),
        error => MixedSqlError::Projection(error),
    })?;
    out.insert(Field::InputRefs,refs)?;
    out.insert(Field::SourceCoords,sql_json(row,12,crate::JsonShape::Array,work)?)?;
    out.integer(Field::FirstKnownGeneration,row.get(13)?)?;
    out.text(Field::FirstKnownCut,sql_text(row,14)?)?;
    out.integer(Field::PublishedGeneration,row.get(15)?)?;
    match effective {
        Some(n) => out.integer(Field::WithdrawnGeneration,n)?,
        None => out.null(Field::WithdrawnGeneration)?,
    }
    for (index,field) in [(17,Field::WithdrawalReason),(18,Field::SupersededBy)] {
        if effective.is_some() && !matches!(row.get_ref(index)?,rusqlite::types::ValueRef::Null) {
            out.text(field,sql_text(row,index)?)?;
        } else { out.null(field)?; }
    }
    out.text(Field::Lifecycle,if effective.is_some() {"withdrawn"} else {"active"})?;
    Ok(out.finish())
}

/// 第一阶段仅取得SQL文本，三列JSON在最终typed project之前不解析。
pub(crate) fn collect_fact_row(row: &rusqlite::Row<'_>, work: &WorkScope) -> Result<PlainJson, MixedSqlError> {
    let mut out = ScopedLegacyRecord::object(ProjectionKind::LegacyPayload, work)?;
    out.text(Field::ObjectId, sql_text(row,0)?)?;
    out.numeric(Field::ObjectRevision, row.get(1)?)?;
    out.text(Field::Kind, sql_text(row,2)?)?;
    out.text(Field::BatchId, sql_text(row,3)?)?;
    out.text(Field::FactKey, sql_text(row,4)?)?;
    out.text(Field::PayloadJson, sql_text(row,5)?)?;
    out.text(Field::InputRefsJson, sql_text(row,6)?)?;
    out.text(Field::SourceCoordsJson, sql_text(row,7)?)?;
    out.numeric(Field::FirstKnownGeneration, row.get(8)?)?;
    out.text(Field::FirstKnownCut, sql_text(row,9)?)?;
    out.numeric(Field::PublishedGeneration, row.get(10)?)?;
    match row.get::<_,Option<i64>>(11)? {
        Some(n) => out.numeric(Field::WithdrawnGeneration,n)?,
        None => out.null(Field::WithdrawnGeneration)?,
    }
    for (i,field) in [(12,Field::WithdrawalReason),(13,Field::SupersededBy)] {
        if matches!(row.get_ref(i)?,rusqlite::types::ValueRef::Null) { out.null(field)?; }
        else { out.text(field,sql_text(row,i)?)?; }
    }
    Ok(out.finish())
}

/// 最终投影才解析三列；复用原kind/数字/JSON核，保持每个读取和首错的先后。
pub(crate) fn project_fact(o: &Value, work: &WorkScope) -> Result<PlainJson, MixedJsonError> {
    let kind = crate::tb02a::checked_fact_kind(o).map_err(MixedJsonError::Business)?;
    let mut out = ScopedLegacyRecord::object(ProjectionKind::ObjectWire,work)?;
    out.borrowed(Field::ObjectId,&o["object_id"])?;
    let n = crate::exact_wire_integer(&o["object_revision"]).map_err(MixedJsonError::Business)?;
    out.integer(Field::ObjectRevision,n)?;
    out.text(Field::Kind,kind)?;
    out.borrowed(Field::BatchId,o.get("batch_id").unwrap_or(&Value::Null))?;
    out.borrowed(Field::FactKey,&o["fact_key"])?;
    for (field,key,message,shape) in [
        (Field::Payload,"payload_json","StorageUnavailable：事实payload缺失",crate::JsonShape::Object),
        (Field::InputRefs,"input_refs_json","StorageUnavailable：事实来源缺失",crate::JsonShape::Array),
        (Field::SourceCoords,"source_coords_json","StorageUnavailable：事实坐标缺失",crate::JsonShape::Array),
    ] {
        let text = o[key].as_str().ok_or_else(|| MixedJsonError::Business(message.into()))?;
        out.insert(field,crate::json_shape_mixed(text,shape,work)?)?;
    }
    let n = crate::exact_wire_integer(&o["first_known_generation"]).map_err(MixedJsonError::Business)?;
    out.integer(Field::FirstKnownGeneration,n)?;
    out.borrowed(Field::FirstKnownCut,&o["first_known_cut"])?;
    let n = crate::exact_wire_integer(&o["published_generation"]).map_err(MixedJsonError::Business)?;
    out.integer(Field::PublishedGeneration,n)?;
    out.null(Field::WithdrawnGeneration)?;
    out.borrowed(Field::WithdrawalReason,o.get("withdrawal_reason").unwrap_or(&Value::Null))?;
    out.borrowed(Field::SupersededBy,o.get("superseded_by").unwrap_or(&Value::Null))?;
    out.text(Field::Lifecycle,"active")?;
    if !o["withdrawn_generation"].is_null() {
        let n = crate::exact_wire_integer(&o["withdrawn_generation"]).map_err(MixedJsonError::Business)?;
        out.replace_integer(Field::WithdrawnGeneration,n)?;
        out.replace_text(Field::Lifecycle,"withdrawn")?;
    }
    Ok(out.finish())
}
