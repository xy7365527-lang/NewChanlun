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
        // 原聚合许可不猜单个子树退款；新旧完整投影在替换成功前同时持费。
        let mut next = Self::clone_projection(self.kind, &self.owner.value, self.work)?;
        let child = PlainJson::try_clone_from(borrowed, Some(self.work)).map_err(MixedJsonError::Resource)?;
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
    use rusqlite::{types::{Type, ValueRef}, Error};
    let value = row.get_ref(index)?;
    match value {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes)
            .map_err(|e| Error::FromSqlConversionFailure(index, Type::Text, Box::new(e))),
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
