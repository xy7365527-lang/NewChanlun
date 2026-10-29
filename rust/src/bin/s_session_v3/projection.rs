//! #1394 C §5.5：完整七字段Q投影；普通/Owned共享排序、键、内容根及元数据门。
//! 只接本事务AuditedState与实际HashReader，派生节点不写SQLite。
use super::audit_store::{CodecRead, CodecStore, RefValue};
use super::business_json::{self as bj, JsonOwner, TextValue};
use super::*;
use newchan_rust::s_content::access::Access;
#[cfg(test)]
use newchan_rust::s_content::job::Job;
#[cfg(test)]
use newchan_rust::s_content::ContentRef;
use newchan_rust::s_content::{self as sc, Kind, NodeDigest, NodeRead, NodeSink, NodeSource};
use newchan_rust::session_protocol::parse_canonical_i64;
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::Transaction;
use sc::capture_allocation::{
    audit_json::{self as aj, JsonView},
    OwnedBudget,
};

const FAMILIES: [&str; 6] = [
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
];
#[derive(Clone, Copy, Debug)]
pub(super) enum HistoryMode {
    AsKnown,
    RecomputedWithRevision,
}
impl HistoryMode {
    fn name(self) -> &'static str {
        match self {
            Self::AsKnown => "AsKnown",
            Self::RecomputedWithRevision => "RecomputedWithRevision",
        }
    }
}
pub(super) struct Projection {
    pub root: RefValue,
    pub digest: TextValue,
    pub wire: JsonOwner,
}
impl Projection {
    pub(super) fn anchor_view(&self) -> ProjectionAnchor<'_> {
        ProjectionAnchor {
            projection: self,
            reference: TransportView::new(self.root.view()),
        }
    }
    #[cfg(test)]
    pub(super) fn anchor_with_budget(
        &self,
        allocation: Option<&OwnedBudget>,
    ) -> Result<JsonOwner, String> {
        match (&self.wire, allocation) {
            (JsonOwner::Plain(v), None) if !v.is_native_admitted() => {}
            #[cfg(feature = "s_allocation_json_testonly")]
            (JsonOwner::Owned(value), Some(b)) if value.budget().same_ledger(b) => {}
            _ => return Err(err("projection实际owner与anchor预算不一致")),
        }
        bj::json_from_view(self.anchor_view().view(), allocation)
    }
    /// 旧Value接口只属于实际None构造；Owned调用必须使用anchor_with_budget或借用anchor_view。
    #[cfg(test)]
    pub(super) fn anchor(&self) -> Value {
        match self
            .anchor_with_budget(None)
            .expect("旧anchor仅支持普通Projection")
        {
            JsonOwner::Plain(value) => value.into_unscoped().expect("兼容anchor仅接无scope值"),
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(_) => unreachable!("None兼容接口不可擦除Owned"),
        }
    }
}
pub(super) struct ProjectionAnchor<'a> {
    projection: &'a Projection,
    reference: TransportView<'a>,
}
impl ProjectionAnchor<'_> {
    pub(super) fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("anchor四字段唯一")
    }
}
impl aj::JsonObject for ProjectionAnchor<'_> {
    fn len(&self) -> usize {
        4
    }
    fn field(&self, i: usize) -> Option<(&str, JsonView<'_>)> {
        Some(match i {
            0 => ("kind", JsonView::String("projection")),
            1 => (
                "fixed_cut",
                bj::field(self.projection.wire.view(), "fixed_cut"),
            ),
            2 => (
                "cut_projection_digest",
                JsonView::String(&self.projection.digest),
            ),
            3 => ("projection_ref", self.reference.view()),
            _ => return None,
        })
    }
}
#[derive(Clone, Copy)]
struct UDecimal {
    bytes: [u8; 20],
    start: usize,
}
impl UDecimal {
    fn new(mut n: u64) -> Self {
        let mut bytes = [0; 20];
        let mut start = 20;
        loop {
            start -= 1;
            bytes[start] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        Self { bytes, start }
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[self.start..]).expect("规范u64十进制")
    }
}
pub(crate) struct TransportView<'a> {
    reference: sc::RefView<'a>,
    extent: Option<UDecimal>,
}
impl<'a> TransportView<'a> {
    pub(crate) fn new(reference: sc::RefView<'a>) -> Self {
        Self {
            reference,
            extent: reference.extent().map(UDecimal::new),
        }
    }
    pub(crate) fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("transport四字段唯一")
    }
}
impl aj::JsonObject for TransportView<'_> {
    fn len(&self) -> usize {
        4
    }
    fn field(&self, i: usize) -> Option<(&str, JsonView<'_>)> {
        Some(match i {
            0 => ("format", JsonView::String(self.reference.format())),
            1 => ("root", JsonView::String(self.reference.root())),
            2 => ("kind", JsonView::String(self.reference.kind().name())),
            3 => (
                "extent",
                self.extent
                    .as_ref()
                    .map(|v| JsonView::String(v.text()))
                    .unwrap_or(JsonView::Null),
            ),
            _ => return None,
        })
    }
}
fn allocation(s: &(impl CodecRead + ?Sized)) -> Result<Option<OwnedBudget>, String> {
    let budget = s.allocation_budget();
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if budget.is_some() {
        return Err(err("owned Q投影入口未启用"));
    }
    Ok(budget)
}
fn decode(bytes: &[u8], budget: Option<&OwnedBudget>) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = budget {
        return aj::OwnedJson::parse(bytes, b)
            .map(JsonOwner::Owned)
            .map_err(|e| e.to_string());
    }
    if budget.is_some() {
        return Err(err("owned Q JSON入口未启用"));
    }
    strict_json(bytes).map(|v| JsonOwner::Plain(v.into()))
}
fn push_array(array: &mut JsonOwner, value: JsonView<'_>) -> Result<(), String> {
    array.push(value)
}

fn session_generation(
    tx: &Transaction<'_>,
    budget: Option<&OwnedBudget>,
) -> Result<TextValue, String> {
    // 原protocol_identity三次读取及失败顺序全部保留；只改变实际文本owner。
    v2::control_meta_text(tx, "session_id", budget)?
        .ok_or_else(|| "StorageUnavailable：缺session".to_owned())?;
    let mut stmt = CheckedStatement::prepare(
        tx,
        c"SELECT session_generation FROM s_protocol_meta WHERE singleton=1",
    )
    .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let value = {
        let mut row = stmt
            .next()
            .map_err(|e| format!("StorageUnavailable：{e}"))?
            .ok_or_else(|| {
                format!(
                    "StorageUnavailable：{}",
                    rusqlite::Error::QueryReturnedNoRows
                )
            })?;
        TextValue::from_sql(&mut row, 0, budget).map_err(|e| format!("StorageUnavailable：{e}"))?
    };
    stmt.finalize()
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    v2::control_meta_text(tx, "writer_epoch", budget)?
        .ok_or_else(|| "StorageUnavailable：缺epoch".to_owned())?;
    Ok(value)
}
pub(super) fn build(
    tx: &Transaction<'_>,
    audited: &state::AuditedState,
    graph: &mut history::HashReader<'_>,
    generation: i64,
    mode: HistoryMode,
    max_metadata_bytes: u64,
    job: &mut dyn Access,
) -> Result<Projection, String> {
    if max_metadata_bytes == 0 || max_metadata_bytes > sc::MAX_FRAME_LEN {
        return Err(err("元数据物化上界必须显式给出且在规范数域内"));
    }
    if generation < 0 {
        return Err(err("generation须为非负整数"));
    }
    let cut = if generation == 0 {
        None
    } else {
        Some(
            audited
                .cuts
                .iter()
                .find(|c| c.metadata.generation == generation)
                .ok_or_else(|| err("Unavailable：请求cut尚未发布"))?,
        )
    };
    let budget = allocation(graph)?;
    let get = |key| {
        v2::control_meta_text(tx, key, budget.as_ref())?
            .ok_or_else(|| err(format!("缺冻结字段{key}")))
    };
    if get("profile_id")? != tb02a::PROFILE {
        return Err(err("非OHLC投影仍待接线，不套用s-record-order/2"));
    }
    let session_generation = session_generation(tx, budget.as_ref())?;
    let mut s = Metered {
        graph,
        job,
        metadata_remaining: max_metadata_bytes,
    };
    s.job.pre_io_check().map_err(err)?;
    let empty_fields: [(&str, JsonView<'_>); 0] = [];
    let empty = write(&mut s, JsonView::object_fields(&empty_fields).map_err(err)?)?;
    let scope = decode(get("scope")?.as_bytes(), budget.as_ref())?;
    let zero_range_fields = [
        ("from", JsonView::String("0")),
        ("to", JsonView::String("-1")),
    ];
    let (evidence, run, frontier, range, index) = if let Some(cut) = cut {
        (
            field(&mut s, cut.snapshot.view(), "catalog_evidence")?,
            text(&mut s, cut.snapshot.view(), "catalog_run_status")?,
            TextValue::from_str(
                bj::Decimal::new(cut.metadata.input_frontier).text(),
                budget.as_ref(),
            )?,
            scalar_json(&mut s, cut.snapshot.view(), "seq_range")?,
            TextValue::from_str(cut.batch_id.as_str(), budget.as_ref())?,
        )
    } else {
        (
            s.native_ref_clone(&empty).map_err(err)?,
            TextValue::from_str("not_run", budget.as_ref())?,
            TextValue::from_str("-1", budget.as_ref())?,
            bj::json_from_view(
                JsonView::object_fields(&zero_range_fields).map_err(err)?,
                budget.as_ref(),
            )?,
            TextValue::from_str("", budget.as_ref())?,
        )
    };
    let session_id = get("session_id")?;
    let generation_decimal = bj::Decimal::new(generation);
    let mut cut_bytes = [0u8; 24];
    cut_bytes[..4].copy_from_slice(b"cut-");
    let cut_len = 4 + generation_decimal.text().len();
    cut_bytes[4..cut_len].copy_from_slice(generation_decimal.text().as_bytes());
    let cut_text = std::str::from_utf8(&cut_bytes[..cut_len]).unwrap();
    let catalog_revision = get("catalog_revision")?;
    let profile_id = get("profile_id")?;
    let profile_hash = get("profile_hash")?;
    let rule_revision = get("rule_revision")?;
    let evidence_transport = TransportView::new(evidence.view());
    let fixed_fields = [
        ("session_id", JsonView::String(&session_id)),
        ("session_generation", JsonView::String(&session_generation)),
        ("structure_cut", JsonView::String(cut_text)),
        (
            "cut_generation",
            JsonView::String(generation_decimal.text()),
        ),
        ("catalog_revision", JsonView::String(&catalog_revision)),
        ("index_frontier", JsonView::String(&index)),
        ("profile_id", JsonView::String(&profile_id)),
        ("profile_hash", JsonView::String(&profile_hash)),
        ("input_frontier", JsonView::String(&frontier)),
        ("seq_range", range.view()),
        ("catalog_run_status", JsonView::String(&run)),
        ("catalog_evidence", evidence_transport.view()),
        ("scope", scope.view()),
        ("rule_revision", JsonView::String(&rule_revision)),
        ("history_mode", JsonView::String(mode.name())),
        (
            "as_of_generation",
            if matches!(mode, HistoryMode::AsKnown) {
                JsonView::String(generation_decimal.text())
            } else {
                JsonView::Null
            },
        ),
    ];
    let fixed = bj::json_from_view(
        JsonView::object_fields(&fixed_fields).map_err(err)?,
        budget.as_ref(),
    )?;
    let fixed_root = edge_object(&mut s, fixed.view(), &[("catalog_evidence", &evidence)])?;
    let mut rows = bj::json_from_view(JsonView::ArraySlice(&[]), budget.as_ref())?;
    let mut row_roots = bj::new_work(budget.as_ref());
    let mut counts = [UDecimal::new(0); 7];
    let mut object_ids = bj::new_map(budget.as_ref());
    for (family_index, family) in FAMILIES.into_iter().enumerate() {
        let mut ordered = bj::new_work(budget.as_ref());
        if let Some(cut) = cut {
            let collection = field(&mut s, cut.snapshot.view(), family)?;
            if collection.kind() != Kind::Array {
                return Err(err("快照记录族不是数组"));
            }
            for i in 0..collection.extent().ok_or_else(|| err("数组缺extent"))? {
                s.job.charge_records(1).map_err(err)?;
                let record = s
                    .lookup_ref(collection.view(), &[sc::path::PathStep::Index(i)])
                    .map_err(err)?;
                let (order, key) = record_order_key(&mut s, family, record.view())?;
                ordered.try_push((order, key, record, i)).map_err(err)?;
            }
        }
        // ordinal保留原stable排序的平局顺序，unstable不分配普通sort scratch。
        ordered.sort_unstable_by(|a, b| a.0[..].cmp(&b.0[..]).then(a.3.cmp(&b.3)));
        let mut keys = bj::new_map(budget.as_ref());
        counts[family_index] = UDecimal::new(ordered.len() as u64);
        for (_, key, record, _) in ordered {
            if keys
                .insert(key.try_clone(budget.as_ref())?, ())
                .map_err(err)?
                .is_some()
            {
                return Err(err(format!("{family}含重复公开键")));
            }
            if matches!(family, "objects" | "withdrawn_objects")
                && object_ids
                    .insert(key.try_clone(budget.as_ref())?, ())
                    .map_err(err)?
                    .is_some()
            {
                return Err(err("active/withdrawn对象身份冲突"));
            }
            let record_transport = TransportView::new(record.view());
            let row_fields = [
                ("family", JsonView::String(family)),
                ("key", key.view()),
                ("record", record_transport.view()),
            ];
            let row = JsonView::object_fields(&row_fields).map_err(err)?;
            row_roots
                .try_push(sc::ValueView::Edge(
                    edge_object(&mut s, row, &[("record", &record)])?
                        .digest()
                        .map_err(err)?,
                ))
                .map_err(err)?;
            push_array(&mut rows, row)?;
        }
    }
    counts[6] = UDecimal::new(rows.view().as_array().unwrap().len() as u64);
    let rows_root = write_value(&mut s, sc::ValueView::Array(&row_roots))?;
    let catalog = build_catalog(tx, &mut s, fixed.view(), &evidence, &empty, generation)?;
    let catalog_transport = TransportView::new(catalog.view());
    let count_fields = [
        (FAMILIES[0], JsonView::String(counts[0].text())),
        (FAMILIES[1], JsonView::String(counts[1].text())),
        (FAMILIES[2], JsonView::String(counts[2].text())),
        (FAMILIES[3], JsonView::String(counts[3].text())),
        (FAMILIES[4], JsonView::String(counts[4].text())),
        (FAMILIES[5], JsonView::String(counts[5].text())),
        ("total", JsonView::String(counts[6].text())),
    ];
    let wire_fields = [
        ("fixed_cut", fixed.view()),
        ("catalog", catalog_transport.view()),
        ("rows", rows.view()),
        (
            "counts",
            JsonView::object_fields(&count_fields).map_err(err)?,
        ),
        ("scope", scope.view()),
        ("omissions", JsonView::ArraySlice(&[])),
        ("order_version", JsonView::String("s-record-order/2")),
    ];
    let wire = bj::json_from_view(
        JsonView::object_fields(&wire_fields).map_err(err)?,
        budget.as_ref(),
    )?;
    let root = edge_object(
        &mut s,
        wire.view(),
        &[
            ("fixed_cut", &fixed_root),
            ("catalog", &catalog),
            ("rows", &rows_root),
        ],
    )?;
    let identity = sc::frame(0x22, &root.digest().map_err(err)?);
    let mut digest_bytes = [0u8; 70];
    digest_bytes[..6].copy_from_slice(b"qcut3-");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, b) in identity.iter().enumerate() {
        digest_bytes[6 + i * 2] = HEX[(b >> 4) as usize];
        digest_bytes[7 + i * 2] = HEX[(b & 15) as usize];
    }
    let digest = TextValue::from_str(std::str::from_utf8(&digest_bytes).unwrap(), budget.as_ref())?;
    s.job.pre_io_check().map_err(err)?;
    Ok(Projection { root, digest, wire })
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum OrderPart {
    Number(i64),
    Text(TextValue),
}
#[derive(PartialEq, Eq)]
struct PublicKey(sc::CodecVec<TextValue>);
impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for value in &self.0 {
            value.hash(state);
        }
    }
}
impl PublicKey {
    fn try_clone(&self, budget: Option<&OwnedBudget>) -> Result<Self, String> {
        let mut out = bj::new_work(budget);
        for v in &self.0 {
            out.try_push(v.try_clone()?).map_err(err)?;
        }
        Ok(Self(out))
    }
    fn view(&self) -> JsonView<'_> {
        JsonView::BorrowedArray(self)
    }
}
impl aj::JsonArray for PublicKey {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, i: usize) -> Option<JsonView<'_>> {
        self.0.get(i).map(|v| JsonView::String(v.as_str()))
    }
}
impl serde::Serialize for PublicKey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&self.view(), s)
    }
}
fn record_order_key(
    s: &mut Metered<'_, '_>,
    family: &str,
    r: sc::RefView<'_>,
) -> Result<(sc::CodecVec<OrderPart>, PublicKey), String> {
    use OrderPart::{Number as N, Text as T};
    if r.kind() != Kind::Object {
        return Err(err("族记录不是对象"));
    }
    let budget = allocation(s)?;
    let mut order = bj::new_work(budget.as_ref());
    let mut key = bj::new_work(budget.as_ref());
    match family {
        "objects" => {
            let id = text(s, r, "object_id")?;
            let revision = number(s, r, "object_revision")?;
            match s.lookup_field_ref(r, "fact_key") {
                Ok(fact) => {
                    order.try_push(N(1)).map_err(err)?;
                    order.try_push(T(text(s, r, "kind")?)).map_err(err)?;
                    let rendered = render(s, fact.view())?;
                    order
                        .try_push(T(TextValue::from_str(
                            rendered
                                .view()
                                .as_str()
                                .ok_or_else(|| err("fact_key非文本"))?,
                            budget.as_ref(),
                        )?))
                        .map_err(err)?;
                    order.try_push(T(id.try_clone()?)).map_err(err)?;
                    order.try_push(N(revision)).map_err(err)?;
                }
                Err(e) if e.code() == sc::path::path_code::PATH_FIELD_MISSING => {
                    for n in [
                        0,
                        number(s, r, "window_start")?,
                        number(s, r, "window_mid")?,
                        number(s, r, "window_end")?,
                    ] {
                        order.try_push(N(n)).map_err(err)?;
                    }
                    order.try_push(T(id.try_clone()?)).map_err(err)?;
                    order.try_push(N(revision)).map_err(err)?;
                }
                Err(e) => return Err(err(e)),
            }
            key.try_push(id).map_err(err)?;
        }
        "withdrawn_objects" => {
            let id = text(s, r, "object_id")?;
            order
                .try_push(N(number(s, r, "withdrawn_generation")?))
                .map_err(err)?;
            order.try_push(T(id.try_clone()?)).map_err(err)?;
            order
                .try_push(N(number(s, r, "object_revision")?))
                .map_err(err)?;
            key.try_push(id).map_err(err)?;
        }
        "witnesses" => {
            let id = text(s, r, "witness_id")?;
            order.try_push(T(text(s, r, "object_id")?)).map_err(err)?;
            order.try_push(N(number(s, r, "slot")?)).map_err(err)?;
            order.try_push(T(id.try_clone()?)).map_err(err)?;
            key.try_push(id).map_err(err)?;
        }
        "relations" => {
            for name in ["subject", "relation_type", "object"] {
                key.try_push(text(s, r, name)?).map_err(err)?;
            }
            for v in &key {
                order.try_push(T(v.try_clone()?)).map_err(err)?;
            }
        }
        "observations" => {
            let id = text(s, r, "observation_id")?;
            let w = field(s, r, "window_start")?;
            let n = if w.kind() == Kind::Null {
                -1
            } else {
                number(s, r, "window_start")?
            };
            order.try_push(T(text(s, r, "kind")?)).map_err(err)?;
            order.try_push(N(n)).map_err(err)?;
            order.try_push(T(id.try_clone()?)).map_err(err)?;
            key.try_push(id).map_err(err)?;
        }
        "raw_history" => {
            let id = text(s, r, "identity_key")?;
            let rev = text(s, r, "revision")?;
            order.try_push(N(number(s, r, "seq")?)).map_err(err)?;
            order.try_push(T(id.try_clone()?)).map_err(err)?;
            order
                .try_push(N(parse_canonical_i64(&rev, "revision")?))
                .map_err(err)?;
            key.try_push(id).map_err(err)?;
            key.try_push(rev).map_err(err)?;
        }
        _ => return Err(err("未知记录族")),
    }
    Ok((order, PublicKey(key)))
}
fn build_catalog(
    tx: &Transaction<'_>,
    s: &mut Metered<'_, '_>,
    fixed: JsonView<'_>,
    evidence: &RefValue,
    empty: &RefValue,
    generation: i64,
) -> Result<RefValue, String> {
    let axes = if generation == 0 {
        None
    } else {
        Some(field(s, evidence.view(), "axes")?)
    };
    // #1419：运行轴集合 = 本 cut 规则的目录轴表（与写者/目录证据同一张表）。
    let live_axes: &[&str] = if axes.is_some() {
        let rule = text(s, evidence.view(), "rule_revision")?;
        catalog_evidence::rule_axes(rule.as_str())?
    } else {
        &[]
    };
    let mut stmt = CheckedStatement::prepare(
        tx,
        c"SELECT catalog_id,kind,title,domain,branches_json FROM catalog ORDER BY catalog_id",
    )
    .map_err(err)?;
    let budget = allocation(s)?;
    let mut items = bj::new_work(budget.as_ref());
    let mut counts = [0u64; 4];
    let names = ["implemented", "not_implemented", "run", "not_run"];
    while let Some(mut row) = stmt.next().map_err(err)? {
        s.job.pre_io_check().map_err(err)?;
        let id = TextValue::from_sql(&mut row, 0, budget.as_ref()).map_err(err)?;
        let (implemented, proof, run, ev) = if live_axes.contains(&id.as_str()) && axes.is_some() {
            let axis = field(s, axes.as_ref().unwrap().view(), &id)?;
            (
                text(s, axis.view(), "impl_status")?,
                text(s, axis.view(), "proof_status")?,
                text(s, axis.view(), "run_status")?,
                field(s, axis.view(), "evidence")?,
            )
        } else {
            (
                TextValue::from_str("not_implemented", budget.as_ref())?,
                TextValue::from_str("not_proved", budget.as_ref())?,
                TextValue::from_str("not_run", budget.as_ref())?,
                s.native_ref_clone(empty).map_err(err)?,
            )
        };
        if let Some(i) = names.iter().position(|name| *name == implemented.as_str()) {
            counts[i] += 1;
        }
        if let Some(i) = names.iter().position(|name| *name == run.as_str()) {
            counts[i] += 1;
        }
        let branches = TextValue::from_sql(&mut row, 4, budget.as_ref()).map_err(err)?;
        let kind = TextValue::from_sql(&mut row, 1, budget.as_ref()).map_err(err)?;
        let title = TextValue::from_sql(&mut row, 2, budget.as_ref()).map_err(err)?;
        let domain = TextValue::from_sql(&mut row, 3, budget.as_ref()).map_err(err)?;
        let parsed = decode(branches.as_bytes(), budget.as_ref())?;
        let projected = bj::project_wire_integers_with_owner(parsed.view(), budget.as_ref())?;
        let ev_transport = TransportView::new(ev.view());
        let item_fields = [
            ("id", JsonView::String(&id)),
            ("kind", JsonView::String(&kind)),
            ("title", JsonView::String(&title)),
            ("domain", JsonView::String(&domain)),
            ("branches", projected.view()),
            ("implementation_status", JsonView::String(&implemented)),
            ("proof_status", JsonView::String(&proof)),
            ("run_status", JsonView::String(&run)),
            ("evidence", ev_transport.view()),
        ];
        let item = JsonView::object_fields(&item_fields).map_err(err)?;
        items
            .try_push(sc::ValueView::Edge(
                edge_object(s, item, &[("evidence", &ev)])?
                    .digest()
                    .map_err(err)?,
            ))
            .map_err(err)?;
    }
    stmt.finalize().map_err(err)?;
    let items = write_value(s, sc::ValueView::Array(&items))?;
    let item_transport = TransportView::new(items.view());
    let decimals = counts.map(UDecimal::new);
    let count_fields = [
        (names[0], JsonView::String(decimals[0].text())),
        (names[1], JsonView::String(decimals[1].text())),
        (names[2], JsonView::String(decimals[2].text())),
        (names[3], JsonView::String(decimals[3].text())),
    ];
    let fields = [
        ("session_id", bj::field(fixed, "session_id")),
        ("structure_cut", bj::field(fixed, "structure_cut")),
        ("catalog_revision", bj::field(fixed, "catalog_revision")),
        ("index_frontier", bj::field(fixed, "index_frontier")),
        ("input_frontier", bj::field(fixed, "input_frontier")),
        ("seq_range", bj::field(fixed, "seq_range")),
        ("catalog_run_status", bj::field(fixed, "catalog_run_status")),
        ("catalog_evidence", bj::field(fixed, "catalog_evidence")),
        ("scope", bj::field(fixed, "scope")),
        ("generation", bj::field(fixed, "cut_generation")),
        ("items", item_transport.view()),
        (
            "counts",
            JsonView::object_fields(&count_fields).map_err(err)?,
        ),
    ];
    edge_object(
        s,
        JsonView::object_fields(&fields).map_err(err)?,
        &[("catalog_evidence", evidence), ("items", &items)],
    )
}
struct Metered<'a, 'b> {
    graph: &'a mut history::HashReader<'b>,
    job: &'a mut dyn Access,
    metadata_remaining: u64,
}
impl Metered<'_, '_> {
    fn lookup_field_ref(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
    ) -> Result<RefValue, sc::CodecError> {
        if self.graph.allocation_budget().is_some() {
            self.graph.q_lookup_field(r, key, self.job)
        } else {
            self.lookup_ref(r, &[sc::path::PathStep::Field(key.into())])
        }
    }
}
impl NodeSource for Metered<'_, '_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
        self.job.pre_io_check()?;
        let value = self.graph.read(d)?;
        if let NodeRead::Found { tag, body } = &value {
            self.job.accept_node(d, *tag, body)?;
        }
        self.job.pre_io_check()?;
        Ok(value)
    }
}
impl NodeSink for Metered<'_, '_> {
    fn put(&mut self, d: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.job.accept_node(&d, tag, body)?;
        self.graph.put(d, tag, body)?;
        self.job.pre_io_check()
    }
}
impl CodecRead for Metered<'_, '_> {
    fn allocation_budget(&self) -> Option<OwnedBudget> {
        self.graph.allocation_budget()
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
    fn lookup_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[sc::path::PathStep],
    ) -> Result<RefValue, sc::CodecError> {
        if self.graph.allocation_budget().is_some() {
            self.graph.q_lookup_ref(r, p, self.job)
        } else {
            sc::path::lookup_path_refview(self, r, p).map(RefValue::from)
        }
    }
    fn render_ref(
        &mut self,
        r: sc::RefView<'_>,
        out: &mut dyn std::io::Write,
    ) -> Result<(), sc::CodecError> {
        if self.graph.allocation_budget().is_some() {
            self.graph.q_render_ref(r, out, self.job)
        } else {
            sc::render_refview(self, r, out)
        }
    }
}
impl CodecStore for Metered<'_, '_> {
    fn generic_store(&mut self) -> &mut dyn sc::NodeStore {
        self
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, sc::CodecError> {
        if self.graph.allocation_budget().is_some() {
            self.graph.q_write_value_view(v, self.job)
        } else {
            sc::write_value_view(self, v)
        }
    }
    fn write_json_view(&mut self, v: JsonView<'_>) -> Result<sc::Built, sc::CodecError> {
        if self.graph.allocation_budget().is_some() {
            self.graph.q_write_json_view(v, self.job)
        } else {
            sc::write_json_view(self, v)
        }
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("Q3ProjectionInvalid：{e}")
}
fn field<'r>(
    s: &mut Metered<'_, '_>,
    r: impl Into<sc::RefView<'r>>,
    k: &str,
) -> Result<RefValue, String> {
    s.lookup_field_ref(r.into(), k).map_err(err)
}
fn render<'r>(s: &mut Metered<'_, '_>, r: impl Into<sc::RefView<'r>>) -> Result<JsonOwner, String> {
    let r = r.into();
    let limit = s.metadata_remaining;
    if matches!(r.kind(), Kind::String | Kind::Number) && r.extent().is_some_and(|n| n > limit) {
        return Err(err(
            "Incomplete/ProjectionMetadataLimit：元数据长度超过剩余额度",
        ));
    }
    struct Buffer {
        bytes: sc::CodecVec<u8>,
        limit: u64,
        exceeded: bool,
        allocation_error: Option<sc::CodecError>,
    }
    impl std::io::Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if (self.bytes.len() as u64).saturating_add(bytes.len() as u64) > self.limit {
                self.exceeded = true;
                return Err(std::io::Error::other("元数据输出超限"));
            }
            match self.bytes.try_extend_bytes(bytes) {
                Ok(()) => Ok(bytes.len()),
                Err(e) => {
                    self.allocation_error = Some(e);
                    Err(std::io::ErrorKind::Other.into())
                }
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let budget = allocation(s)?;
    let mut out = Buffer {
        bytes: bj::new_work(budget.as_ref()),
        limit,
        exceeded: false,
        allocation_error: None,
    };
    let result = s.render_ref(r, &mut out);
    s.metadata_remaining -= out.bytes.len() as u64;
    if out.exceeded {
        return Err(err(
            "Incomplete/ProjectionMetadataLimit：元数据展开或转义超过剩余额度",
        ));
    }
    if let Some(e) = out.allocation_error {
        return Err(err(e));
    }
    result.map_err(err)?;
    s.job.pre_io_check().map_err(err)?;
    decode(&out.bytes, budget.as_ref())
}
fn scalar_json<'r>(
    s: &mut Metered<'_, '_>,
    r: impl Into<sc::RefView<'r>>,
    k: &str,
) -> Result<JsonOwner, String> {
    let child = field(s, r, k)?;
    render(s, child.view())
}
fn text<'r>(
    s: &mut Metered<'_, '_>,
    r: impl Into<sc::RefView<'r>>,
    k: &str,
) -> Result<TextValue, String> {
    let value = scalar_json(s, r, k)?;
    let budget = allocation(s)?;
    TextValue::from_str(
        value
            .view()
            .as_str()
            .ok_or_else(|| err(format!("{k}不是规范文本")))?,
        budget.as_ref(),
    )
}
fn number(s: &mut Metered<'_, '_>, r: sc::RefView<'_>, k: &str) -> Result<i64, String> {
    parse_canonical_i64(&text(s, r, k)?, k)
}
fn built_ref(s: &(impl CodecRead + ?Sized), built: sc::Built) -> Result<RefValue, String> {
    s.native_built_ref(&built).map_err(err)
}

fn write(s: &mut Metered<'_, '_>, v: JsonView<'_>) -> Result<RefValue, String> {
    let built = s.write_json_view(v).map_err(err)?;
    built_ref(s, built)
}
fn write_value(s: &mut Metered<'_, '_>, v: sc::ValueView<'_>) -> Result<RefValue, String> {
    let built = s.write_value_view(v).map_err(err)?;
    built_ref(s, built)
}
fn edge_object(
    s: &mut Metered<'_, '_>,
    value: JsonView<'_>,
    edges: &[(&str, &RefValue)],
) -> Result<RefValue, String> {
    let object = value.as_object().ok_or_else(|| err("投影值非对象"))?;
    let budget = allocation(s)?;
    let mut fields = bj::new_work(budget.as_ref());
    for (k, v) in object.iter() {
        let value = match edges.iter().find(|(name, _)| *name == k) {
            Some((_, r)) => sc::ValueView::Edge(r.digest().map_err(err)?),
            None => sc::ValueView::Json(v),
        };
        fields.try_push((k, value)).map_err(err)?;
    }
    write_value(s, sc::ValueView::Object(&fields))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::path::PathStep;
    const METADATA: u64 = 1_048_576;
    fn plain_wire(p: &Projection) -> &Value {
        match &p.wire {
            JsonOwner::Plain(v) => v,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(_) => panic!("测试oracle只接普通投影"),
        }
    }
    fn normal_write(s: &mut dyn NodeSink, value: &Value) -> Result<RefValue, String> {
        sc::write_json_view(s, value.into())
            .map(|v| RefValue::from(v.content_ref()))
            .map_err(err)
    }

    fn render_fixture<'r>(
        s: &mut dyn NodeSource,
        r: impl Into<sc::RefView<'r>>,
    ) -> Result<Value, String> {
        let mut bytes = Vec::new();
        sc::render_refview(s, r.into(), &mut bytes).map_err(err)?;
        strict_json(&bytes)
    }
    use prepared::tests::Fixture;
    use sc::job::{Existing, JobLimits};
    use std::io::Write;
    use std::process::{Command, Stdio};
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"16777216","max_total_nodes":"100000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"20000","max_staged_bytes":"1048576","max_store_bytes":"33554432"})).unwrap(), Existing{store_bytes:0,staged_bytes:0},Box::new(std::time::Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
    }
    fn projected(f: &mut Fixture, g: i64, mode: HistoryMode) -> (Projection, Value, Value) {
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        let p = build(&tx, &audited, &mut graph, g, mode, METADATA, &mut job()).unwrap();
        let catalog = render_fixture(
            &mut graph,
            &ContentRef::from_transport_json(&plain_wire(&p)["catalog"]).unwrap(),
        )
        .unwrap();
        let logical = render_fixture(&mut graph, &p.root).unwrap();
        (p, catalog, logical)
    }
    /// 已发布 cut 的原件，只读审计后展开；不得用生成草案代替正式发布。
    fn published_w_cut(f: &mut Fixture, g: i64) -> (Value, Value, Value) {
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let cut = &audited.cuts[(g - 1) as usize];
        assert_eq!(cut.metadata.generation, g);
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        (
            render_fixture(&mut graph, &cut.snapshot).unwrap(),
            render_fixture(&mut graph, &cut.delta).unwrap(),
            render_fixture(&mut graph, cut.roots.generated_state.view()).unwrap(),
        )
    }

    fn w_kind(snapshot: &Value, kind: &str) -> Vec<Value> {
        snapshot["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["kind"] == kind)
            .cloned()
            .collect()
    }

    // bounded-layout/1 排序的是 CC-004..007 请求段；后接的结构分类请求另有具名槽。
    // 保持整数组原下标，验证该段连续，不过滤后重新编号。
    fn is_window_request(request: &Value) -> bool {
        let id: Value = serde_json::from_str(request["request_id"].as_str().unwrap()).unwrap();
        let kind = id[0].as_str().or_else(|| id[0][0].as_str()).unwrap();
        matches!(
            kind,
            "CC-004.initialization"
                | "CC-004.undetermined_raw"
                | "CC-004.inclusion_step"
                | "CC-005.inclusion_group"
                | "CC-006.local_shape"
                | "CC-007.fractal_description"
        )
    }

    fn w_request_order(request: &Value) -> (u8, i64, usize) {
        let id: Value = serde_json::from_str(request["request_id"].as_str().unwrap()).unwrap();
        let (kind, tail, label) = if let Some(slot) = id[0].as_array() {
            let labels = [
                "shape_description",
                "descriptive_labels",
                "subsequent_development",
            ];
            (
                slot[0].as_str().unwrap(),
                &slot[1],
                4 + labels.iter().position(|l| id[1] == *l).unwrap(),
            )
        } else {
            (id[0].as_str().unwrap(), &id[1], 0)
        };
        if kind == "CC-004.initialization" {
            return (0, 0, 0);
        }
        if tail == "pending" {
            return (2, 0, usize::from(kind == "CC-006.local_shape"));
        }
        let start = if tail.is_array() { &tail[0] } else { tail }
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let label = match kind {
            "CC-004.undetermined_raw" => 0,
            "CC-004.inclusion_step" => 1,
            "CC-005.inclusion_group" => 2,
            "CC-006.local_shape" => 3,
            _ => label,
        };
        (1, start, label)
    }

    /// 按 request_id 配对，摘要用完整请求值比较，不排序发布数组。
    /// 无窗口的 Initialization / Pending 不具有变化坐标，不参与 m。
    fn w_change_boundary(old: &[Value], new: &[Value]) -> Option<(i64, usize)> {
        let mut earliest = None;
        for (side, other) in [(old, new), (new, old)] {
            for r in side {
                let same = other.iter().find(|x| x["request_id"] == r["request_id"]);
                if same != Some(r) {
                    let (phase, coord, label) = w_request_order(r);
                    if phase == 1 {
                        let key = (coord, label);
                        earliest = Some(earliest.map_or(key, |m: (i64, usize)| m.min(key)));
                    }
                }
            }
        }
        earliest
    }

    #[test]
    fn w_requests_published_prior_prefix_stability() {
        let mut f = Fixture::new();
        let mut previous = Vec::new();
        let (mut finite_prefix, mut saw_delete, mut saw_insert, mut saw_revision) =
            (false, false, false, false);
        for (n, row, revision) in [
            (1, ("r0", 0, 5, 10, 0, 5), 1),
            (2, ("r1", 10, 5, 8, 2, 5), 1),
            (3, ("r2", 20, 7, 11, 3, 7), 1),
            (4, ("r3", 30, 9, 13, 5, 9), 1),
            (5, ("insert", 15, 6, 10, 2, 6), 1),
            (6, ("r3", 30, 4, 8, 0, 4), 2),
        ] {
            // Fixture::publish 从上一代独立索引读取 structure_prior，真实提交后才读 requests。
            let (e, t) = f.accept_begin(n, row, revision);
            f.publish(&e, &t).unwrap();
            let (snapshot, _, _) = published_w_cut(&mut f, n);
            let knowledge = w_kind(&snapshot, "CC-054.knowledge_state");
            assert_eq!(knowledge.len(), 1);
            let all_requests = knowledge[0]["payload"]["requests"].as_array().unwrap();
            let end = all_requests
                .iter()
                .position(|r| !is_window_request(r))
                .unwrap_or(all_requests.len());
            assert!(all_requests[end..].iter().all(|r| !is_window_request(r)));
            let requests = &all_requests[..end];
            assert_eq!(w_request_order(&requests[0]).0, 0);
            assert!(
                requests
                    .windows(2)
                    .all(|pair| w_request_order(&pair[0]) < w_request_order(&pair[1])),
                "g{n} 原发布数组不按阶段/坐标/label 全序"
            );
            if n > 1 {
                let m = w_change_boundary(&previous, requests);
                for (i, old) in previous.iter().enumerate() {
                    let (phase, coord, label) = w_request_order(old);
                    if phase == 1 && m.is_none_or(|m| (coord, label) < m) {
                        assert_eq!(
                            requests.get(i),
                            Some(old),
                            "g{n} 稳定前缀在原数组下标 {i} 漂移"
                        );
                        finite_prefix |= m.is_some();
                    }
                    if !requests
                        .iter()
                        .any(|r| r["request_id"] == old["request_id"])
                        && phase == 1
                    {
                        saw_delete = true;
                        // 纯删除探针取真实旧侧原件；新侧为空，必须仍贡献旧坐标。
                        assert_eq!(
                            w_change_boundary(std::slice::from_ref(old), &[]),
                            Some((coord, label))
                        );
                    }
                }
                saw_insert |= requests.iter().any(|r| {
                    w_request_order(r).0 == 1
                        && !previous.iter().any(|p| p["request_id"] == r["request_id"])
                });
                saw_revision |= requests.iter().any(|r| {
                    w_request_order(r).0 == 1
                        && previous
                            .iter()
                            .any(|p| p["request_id"] == r["request_id"] && p != r)
                });
            }
            previous = requests.to_vec();
        }
        assert!(finite_prefix, "须有有限 m 且非空前缀的真实相邻发布");
        assert!(saw_delete && saw_insert && saw_revision);
    }

    #[test]
    fn w2_replacement_scope_revision_and_no_fallback() {
        for rule in [crate::W1_STRUCTURE_RULE_REVISION, crate::W_STRUCTURE_RULE_REVISION] {
            let mut f = Fixture::with_rule(rule);
            for (n, row) in [
                (1, ("scope0", 0, 5, 10, 0, 5)),
                (2, ("scope1", 10, 5, 8, 2, 5)),
                (3, ("scope2", 20, 7, 11, 3, 7)),
            ] {
                let (e, t) = f.accept_begin(n, row, 1);
                f.publish(&e, &t).unwrap();
            }
            let (snapshot, delta, _) = published_w_cut(&mut f, 3);
            let raws = snapshot["withdrawn_objects"].as_array().unwrap().iter()
                .filter(|o| o["kind"] == "CC-004.undetermined_raw").collect::<Vec<_>>();
            assert_eq!(raws.len(), 2);
            let groups = w_kind(&snapshot, "CC-005.inclusion_group");
            let w2 = rule == crate::W_STRUCTURE_RULE_REVISION;
            for raw in raws {
                let edges = delta["replaces"].as_array().unwrap().iter()
                    .filter(|edge| edge["old_object_id"] == raw["object_id"]).collect::<Vec<_>>();
                assert_eq!(edges.len(), usize::from(w2));
                if w2 {
                    let target = groups.iter().find(|o| o["object_id"] == edges[0]["new_object_id"]).unwrap();
                    assert!(target["payload"]["members"].as_array().unwrap().contains(&raw["payload"]["raw"]["source_coord"]));
                }
            }
            {
                let tx = f.conn.transaction().unwrap();
                state::audit(&tx).unwrap();
            }
            if w2 {
                let wrong = groups.iter().find(|g| !g["payload"]["members"].as_array().unwrap().contains(&json!("0"))).unwrap();
                f.conn.execute("UPDATE s_content_objects SET superseded_by=?1 WHERE kind='CC-004.undetermined_raw'", [wrong["object_id"].as_str().unwrap()]).unwrap();
                let tx = f.conn.transaction().unwrap();
                assert!(state::audit(&tx).is_err());
            }
        }
    }

    /// #1419：原 tb02u 生命周期判据经真实 S3 接纳、Begin、发布与 recover 承接。
    #[test]
    fn w_published_lifecycle_first_known_revision_replaces_and_recovery() {
        let mut f = Fixture::with_clock_plan(8, 4, &[("recover-w", 35)]);
        let mut waiting: Vec<Value> = Vec::new();
        let mut historical = Vec::new();
        for (n, row, revision) in [
            (1, ("w0", 0, 5, 10, 0, 5), 1),
            (2, ("w1", 10, 5, 8, 2, 5), 1),
            (3, ("w1", 10, 5, 9, 1, 5), 2),
        ] {
            let (e, t) = f.accept_begin(n, row, revision);
            f.publish(&e, &t).unwrap();
            let (snapshot, _, _) = published_w_cut(&mut f, n);
            let pending = w_kind(&snapshot, "CC-004.undetermined_raw");
            assert_eq!(pending.len(), if n == 1 { 1 } else { 2 });
            assert!(w_kind(&snapshot, "CC-005.inclusion_group").is_empty());
            assert!(w_kind(&snapshot, "CC-006.local_shape").is_empty());
            for o in &pending {
                assert_eq!(o["payload"]["direction_state"], "Undetermined");
                assert_eq!(o["input_refs"].as_array().unwrap().len(), 1);
                let coord = o["payload"]["raw"]["source_coord"].as_str().unwrap();
                let birth = if coord == "0" {
                    "1"
                } else if n == 2 {
                    "2"
                } else {
                    "3"
                };
                assert_eq!(o["first_known_generation"], birth);
                assert_eq!(o["payload"]["pending_since_cut"], format!("cut-{birth}"));
            }
            if n > 1 {
                assert_eq!(
                    pending
                        .iter()
                        .find(|o| o["payload"]["raw"]["source_coord"] == "0")
                        .unwrap(),
                    &waiting[0]
                );
            }
            if n == 3 {
                let old = waiting
                    .iter()
                    .find(|o| o["payload"]["raw"]["source_coord"] == "10")
                    .unwrap();
                let new = pending
                    .iter()
                    .find(|o| o["payload"]["raw"]["source_coord"] == "10")
                    .unwrap();
                assert_ne!(
                    old["object_id"], new["object_id"],
                    "raw 修订必须换未定对象身份"
                );
            }
            waiting = pending;
            historical.push(projected(&mut f, n, HistoryMode::AsKnown));
        }
        f.recover_writer("2", "recover-w");
        for (i, (old, catalog, logical)) in historical.iter().enumerate() {
            let (after, after_catalog, after_logical) =
                projected(&mut f, (i + 1) as i64, HistoryMode::AsKnown);
            assert_eq!(after.root, old.root);
            assert_eq!(after.digest, old.digest);
            assert_eq!(plain_wire(&after), plain_wire(old));
            assert_eq!(&after_catalog, catalog);
            assert_eq!(&after_logical, logical, "recover 后完整 AsKnown 不变");
        }
        let (e, t) = f.accept_begin(4, ("w2", 20, 7, 11, 3, 7), 1);
        f.publish(&e, &t).unwrap();
        let (snapshot, delta, _) = published_w_cut(&mut f, 4);
        assert!(w_kind(&snapshot, "CC-004.undetermined_raw").is_empty());
        let groups = w_kind(&snapshot, "CC-005.inclusion_group");
        assert_eq!(groups.len(), 2);
        for old in &waiting {
            let coord = &old["payload"]["raw"]["source_coord"];
            let group = groups
                .iter()
                .find(|g| g["payload"]["members"].as_array().unwrap().contains(coord))
                .unwrap();
            assert!(
                delta["replaces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["old_object_id"] == old["object_id"]
                        && r["new_object_id"] == group["object_id"]),
                "定向后组须逐原件替代未定对象：{coord}"
            );
            assert!(snapshot["relations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["relation_type"] == "replaces"
                    && r["subject"] == group["object_id"]
                    && r["object"] == old["object_id"]));
        }
        for (i, (old, catalog, logical)) in historical.iter().enumerate() {
            let (after, after_catalog, after_logical) =
                projected(&mut f, (i + 1) as i64, HistoryMode::AsKnown);
            assert_eq!(after.root, old.root);
            assert_eq!(after.digest, old.digest);
            assert_eq!(plain_wire(&after), plain_wire(old));
            assert_eq!(&after_catalog, catalog);
            assert_eq!(&after_logical, logical);
        }
    }

    /// #1419 P4 G7 前半：原 `tb02u_pending_append_keeps_first_known_and_policy_gate`
    /// （`028796ba08:rust/src/bin/s_session_v2/tb02u_tests.rs:140–171`）同输入 (0,10)/(2,8)/(1,11)
    /// 逐代真实接纳、Begin、发布：每代未定原件数 = n，raw0 对象逐字等于首获知那代，零正式组，
    /// 发布 cut 经只读审计展开。后半旧规则拒写见 `driver::tests::s3_writer_and_recover_reject_frozen_nonempty_historical_session_unchanged`。
    #[test]
    fn w_pending_append_keeps_first_known_and_zero_groups() {
        let mut f = Fixture::new();
        let mut first = Value::Null;
        for (n, row) in [
            (1, ("o0", 0, 0, 10, 0, 10)),
            (2, ("o1", 1, 2, 8, 2, 8)),
            (3, ("o2", 2, 1, 11, 1, 11)),
        ] {
            let (e, t) = f.accept_begin(n, row, 1);
            f.publish(&e, &t).unwrap();
            let (snapshot, _, _) = published_w_cut(&mut f, n);
            let pending = w_kind(&snapshot, "CC-004.undetermined_raw");
            assert_eq!(pending.len(), n as usize, "g{n}");
            let raw0 = pending
                .iter()
                .find(|o| o["payload"]["raw"]["source_coord"] == "0")
                .unwrap();
            if n == 1 {
                first = raw0.clone();
            } else {
                assert_eq!(raw0, &first, "g{n} 首获知原件不变");
            }
            assert!(
                w_kind(&snapshot, "CC-005.inclusion_group").is_empty(),
                "g{n}"
            );
        }
    }

    /// #1419 P4 G3：原 `tb02a_three_groups_with_equal_roots_publish_shape_with_root_set` 的 CC-054 域断言
    /// （`028796ba08:rust/src/bin/s_structure_session.rs:7017–7020`）按签署合同承接：同输入
    /// (0,1)/(3,10)/(5,10)/(20,30) 经正式 S3 发布，CC-054 域等于该 cut 规则串的笔合同知识域
    /// （`tb02b::contract_for_rule`，W 串为 W_CONTRACT）；与原同价根集组判据一并核。
    #[test]
    fn w_three_groups_with_equal_roots_publish_contract_knowledge_domain() {
        // S3 每条 ingest 一根 raw：四根逐代接纳发布，取末代 cut 核原同输入判据。
        let mut f = Fixture::new();
        for (n, row) in [
            (1, ("o0", 0, 0, 1, 0, 1)),
            (2, ("o1", 1, 3, 10, 3, 10)),
            (3, ("o2", 2, 5, 10, 5, 10)),
            (4, ("o3", 3, 20, 30, 20, 30)),
        ] {
            let (e, t) = f.accept_begin(n, row, 1);
            f.publish(&e, &t).unwrap();
        }
        let rule = meta_get_opt(&f.conn, "rule_revision").unwrap().unwrap();
        assert_eq!(rule, crate::W_STRUCTURE_RULE_REVISION);
        let (snapshot, _, _) = published_w_cut(&mut f, 4);
        let knowledge = w_kind(&snapshot, "CC-054.knowledge_state");
        assert_eq!(knowledge.len(), 1);
        let contract = crate::tb02b::contract_for_rule(&rule).unwrap();
        assert_eq!(contract, crate::tb02b::W_CONTRACT);
        assert_eq!(knowledge[0]["payload"]["domain"], "raw_adjacent_W_initialization");
        assert_ne!(
            knowledge[0]["payload"]["domain"],
            crate::tb02b::CURRENT_CONTRACT.knowledge_domain,
            "W cut 不得发旧串域"
        );
        let groups = w_kind(&snapshot, "CC-005.inclusion_group");
        assert!(
            groups
                .iter()
                .any(|g| g["payload"]["high_sources"] == json!(["1", "2"])),
            "同价高点组以根集发布"
        );
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn owned_capture(tx: &Transaction<'_>, budget: &OwnedBudget) -> sc::AuditCapture {
        sc::AuditCapture::from_slab_for_allocation_prototype(
            content_store::capture_slab_for_prototype(
                tx,
                budget,
                &content_store::CaptureCancellation::never(),
            )
            .unwrap(),
        )
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn with_projection_capture<'c, T>(
        capture: &'c sc::AuditCapture,
        budget: &OwnedBudget,
        run: impl FnOnce(&mut history::HashReader<'c>) -> T,
    ) -> T {
        history::with_overlay_prototype(
            history::OverlayPrototype {
                slots: 8192,
                bytes: 1_048_576,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || run(&mut history::HashReader::captured_for_audit(capture, None).unwrap()),
        )
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_projection_two_input_complete_wire_roots_digest_anchor_and_job_match_plain() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let budget = OwnedBudget::new(67_108_864);
        let capture = owned_capture(&tx, &budget);
        for mode in [HistoryMode::AsKnown, HistoryMode::RecomputedWithRevision] {
            let mut source = content_store::ReadStore::new(&tx).unwrap();
            let mut plain_graph = history::HashReader::new(&mut source);
            let mut plain_job = job();
            let plain = build(
                &tx,
                &audited,
                &mut plain_graph,
                2,
                mode,
                METADATA,
                &mut plain_job,
            )
            .unwrap();
            let plain_logical = render_fixture(&mut plain_graph, plain.root.view()).unwrap();
            let plain_anchor = plain.anchor();
            with_projection_capture(&capture, &budget, |graph| {
                let mut owned_job = job();
                let owned = build(&tx, &audited, graph, 2, mode, METADATA, &mut owned_job).unwrap();
                assert_eq!(owned.root, plain.root);
                assert_eq!(owned.digest, plain.digest);
                assert_eq!(owned.wire.view(), plain.wire.view());
                let (a, b) = (plain_job.snapshot(), owned_job.snapshot());
                assert_eq!(
                    (a.nodes, a.node_bytes, a.records),
                    (b.nodes, b.node_bytes, b.records)
                );
                let anchor = owned.anchor_with_budget(Some(&budget)).unwrap();
                assert_eq!(anchor.view(), JsonView::Plain(&plain_anchor));
                assert!(owned.anchor_with_budget(None).is_err());
                let mut reading_job = job();
                let mut metered = Metered {
                    graph,
                    job: &mut reading_job,
                    metadata_remaining: METADATA,
                };
                let logical = render(&mut metered, owned.root.view()).unwrap();
                assert_eq!(logical.view(), JsonView::Plain(&plain_logical));
                assert_eq!(
                    bj::field(owned.wire.view(), "rows")
                        .as_array()
                        .unwrap()
                        .len(),
                    a.records as usize
                );
            });
        }
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_projection_real_source_metadata_limit_and_missing_root_reject_without_writes() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let budget = OwnedBudget::new(67_108_864);
        let capture = owned_capture(&tx, &budget);
        let before = tx.total_changes();
        with_projection_capture(&capture, &budget, |graph| {
            let error = build(&tx, &audited, graph, 2, HistoryMode::AsKnown, 1, &mut job())
                .err()
                .unwrap();
            assert!(error.contains("ProjectionMetadataLimit"), "{error}");
        });
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(tx.total_changes(), before);
        let digest = audited.cuts[1].snapshot.digest().unwrap();
        tx.execute(
            "DELETE FROM s_content_nodes WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        let before = tx.total_changes();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut plain = history::HashReader::new(&mut source);
        let normal = build(
            &tx,
            &audited,
            &mut plain,
            2,
            HistoryMode::AsKnown,
            METADATA,
            &mut job(),
        )
        .err()
        .unwrap();
        assert!(normal.contains(sc::code::NODE_MISSING), "{normal}");
        let capture = owned_capture(&tx, &budget);
        with_projection_capture(&capture, &budget, |graph| {
            let error = build(
                &tx,
                &audited,
                graph,
                2,
                HistoryMode::AsKnown,
                METADATA,
                &mut job(),
            )
            .err()
            .unwrap();
            assert!(error.contains(sc::code::NODE_MISSING), "{error}");
        });
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(tx.total_changes(), before);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_projection_real_source_zero_and_short_budget_keep_prior_projection_owner_and_drop() {
        use sc::capture_allocation::CaptureSlab;
        let mut f = Fixture::new();
        publish(&mut f, 1);
        publish(&mut f, 2);
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let budget = OwnedBudget::new(67_108_864);
        let capture = owned_capture(&tx, &budget);
        with_projection_capture(&capture, &budget, |graph| {
            let prior = build(
                &tx,
                &audited,
                graph,
                2,
                HistoryMode::AsKnown,
                METADATA,
                &mut job(),
            )
            .unwrap();
            let prior_digest = prior.root.digest().unwrap();
            let prior_rows = bj::field(prior.wire.view(), "rows")
                .as_array()
                .unwrap()
                .len();
            let profile = v2::control_meta_text(&tx, "profile_id", Some(&budget))
                .unwrap()
                .unwrap();
            let request = budget.allocation_request_probe().last_request_bytes;
            drop(profile);
            for remaining in [0, request - 1] {
                let live = budget.snapshot().live_requested_layout_bytes;
                let fill =
                    CaptureSlab::new(0, budget.snapshot().limit_bytes - live - remaining, &budget)
                        .unwrap();
                let error = build(
                    &tx,
                    &audited,
                    graph,
                    2,
                    HistoryMode::AsKnown,
                    METADATA,
                    &mut job(),
                )
                .err()
                .unwrap();
                assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
                assert_eq!(prior.root.digest().unwrap(), prior_digest);
                assert_eq!(
                    bj::field(prior.wire.view(), "rows")
                        .as_array()
                        .unwrap()
                        .len(),
                    prior_rows
                );
                assert!(prior_rows > 0);
                drop(fill);
            }
            let again = build(
                &tx,
                &audited,
                graph,
                2,
                HistoryMode::AsKnown,
                METADATA,
                &mut job(),
            )
            .unwrap();
            assert_eq!(again.wire.view(), prior.wire.view());
            assert_eq!(again.root, prior.root);
        });
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    /// 使用仓内原Q的排序/键函数作为独立oracle；不复写其六族判据。
    fn python_rows(snapshot: &Value) -> Value {
        let script="import sys,json;sys.path.insert(0,sys.argv[1]);import s_query as q;s=json.load(sys.stdin);json.dump([dict(family=f,key=q._key(f,r),record=r) for f in q.FAMILIES for r in sorted(s[f],key=lambda r:q._order(f,r))],sys.stdout,ensure_ascii=False,separators=(',',':'))";
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let mut child = Command::new("python3")
            .args(["-c", script])
            .arg(base)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let data = serde_json::to_vec(snapshot).unwrap();
        child.stdin.take().unwrap().write_all(&data).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        serde_json::from_slice(&output.stdout).unwrap()
    }
    #[test]
    fn real_projection_matches_original_q_order_and_hashes_complete_logical_values() {
        let mut f = Fixture::new();
        for n in 1..=3 {
            publish(&mut f, n);
        }
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        let snapshot = render_fixture(&mut graph, &audited.cuts[2].snapshot).unwrap();
        let mut budget = job();
        let p = build(
            &tx,
            &audited,
            &mut graph,
            3,
            HistoryMode::AsKnown,
            METADATA,
            &mut budget,
        )
        .unwrap();
        assert_eq!(plain_wire(&p).as_object().unwrap().len(), 7);
        assert_eq!(plain_wire(&p)["fixed_cut"].as_object().unwrap().len(), 16);
        assert_eq!(plain_wire(&p)["counts"].as_object().unwrap().len(), 7);
        assert_eq!(plain_wire(&p)["order_version"], "s-record-order/2");
        assert_eq!(plain_wire(&p)["omissions"], json!([]));
        let logical = render_fixture(&mut graph, &p.root).unwrap();
        let expected = python_rows(&snapshot);
        assert_eq!(logical["rows"], expected);
        assert!(!expected.as_array().unwrap().is_empty());
        assert_eq!(
            budget.snapshot().records,
            expected.as_array().unwrap().len() as u64
        );
        assert_eq!(
            logical["fixed_cut"]["catalog_evidence"],
            snapshot["catalog_evidence"]
        );
        let mut independently_composed = plain_wire(&p).clone();
        independently_composed["rows"] = expected;
        independently_composed["catalog"] = logical["catalog"].clone();
        independently_composed["fixed_cut"]["catalog_evidence"] =
            snapshot["catalog_evidence"].clone();
        let expected_root =
            normal_write(&mut sc::MemoryNodes::new(), &independently_composed).unwrap();
        assert_eq!(expected_root, p.root);
        assert_ne!(
            normal_write(&mut sc::MemoryNodes::new(), &plain_wire(&p)).unwrap(),
            p.root,
            "引用carrier不能作为普通JSON进入逻辑根"
        );
        let mut bytes = b"NewChanlun/SContent/1\0".to_vec();
        bytes.push(0x22);
        bytes.extend_from_slice(&32u64.to_be_bytes());
        bytes.extend_from_slice(&p.root.digest().unwrap());
        assert_eq!(p.digest, format!("qcut3-{}", sha256_hex(&bytes)));
        independently_composed["rows"][0]["record"]["proof_body_changed"] = json!(true);
        assert_ne!(
            normal_write(&mut sc::MemoryNodes::new(), &independently_composed).unwrap(),
            p.root
        );
    }
    #[test]
    fn zero_and_historical_catalog_use_their_own_axes_after_later_commit() {
        let mut f = Fixture::with_retain(1);
        let (zero, cat, _) = projected(&mut f, 0, HistoryMode::AsKnown);
        assert_eq!(plain_wire(&zero)["counts"]["total"], "0");
        assert_eq!(plain_wire(&zero)["fixed_cut"]["input_frontier"], "-1");
        assert_eq!(
            plain_wire(&zero)["fixed_cut"]["seq_range"],
            json!({"from":"0","to":"-1"})
        );
        assert_eq!(plain_wire(&zero)["fixed_cut"]["index_frontier"], "");
        assert_eq!(plain_wire(&zero)["fixed_cut"]["profile_id"], tb02a::PROFILE);
        for item in cat["items"].as_array().unwrap() {
            assert_eq!(item["implementation_status"], "not_implemented");
            assert_eq!(item["proof_status"], "not_proved");
            assert_eq!(item["run_status"], "not_run");
            assert_eq!(item["evidence"], json!({}));
        }
        publish(&mut f, 1);
        let (old, old_catalog, logical) = projected(&mut f, 1, HistoryMode::AsKnown);
        let axes = &logical["fixed_cut"]["catalog_evidence"]["axes"];
        let items = old_catalog["items"].as_array().unwrap();
        assert!(items
            .windows(2)
            .all(|w| w[0]["id"].as_str() < w[1]["id"].as_str()));
        for item in items {
            if let Some(axis) = axes.get(item["id"].as_str().unwrap()) {
                for (public, private) in [
                    ("implementation_status", "impl_status"),
                    ("proof_status", "proof_status"),
                    ("run_status", "run_status"),
                    ("evidence", "evidence"),
                ] {
                    assert_eq!(item[public], axis[private]);
                }
            }
        }
        for key in ["implemented", "not_implemented", "run", "not_run"] {
            let field = if key.contains("implemented") {
                "implementation_status"
            } else {
                "run_status"
            };
            assert_eq!(
                old_catalog["counts"][key],
                items.iter().filter(|i| i[field] == key).count().to_string()
            );
        }
        publish(&mut f, 2);
        let (again, again_catalog, _) = projected(&mut f, 1, HistoryMode::AsKnown);
        assert_eq!(again.root, old.root);
        assert_eq!(again.digest, old.digest);
        assert_eq!(plain_wire(&again), plain_wire(&old));
        assert_eq!(again_catalog, old_catalog);
        assert_eq!(projected(&mut f, 0, HistoryMode::AsKnown).0.root, zero.root);
        let current = projected(&mut f, 2, HistoryMode::RecomputedWithRevision).0;
        assert_eq!(
            plain_wire(&current)["fixed_cut"]["as_of_generation"],
            Value::Null
        );
        assert_ne!(current.digest, old.digest);
    }
    #[test]
    fn projection_anchor_is_exact_and_query_only_and_old_cut_survives_delta_retention() {
        let mut f = Fixture::with_retain(1);
        publish(&mut f, 1);
        let anchor = projected(&mut f, 1, HistoryMode::AsKnown).0.anchor();
        publish(&mut f, 2);
        let mut conn = Connection::open(f.db_path()).unwrap();
        conn.pragma_update(None, "query_only", true).unwrap();
        let before = conn.total_changes();
        let tx = conn.transaction().unwrap();
        let path = [PathStep::Field("rows".into())];
        let p = reader::projection_fragment(
            &tx,
            &anchor,
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            262144,
            METADATA,
            &mut job(),
        )
        .unwrap();
        sc::fragment::verify(
            &ContentRef::from_transport_json(&anchor["projection_ref"]).unwrap(),
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            &p,
            262144,
            &mut job(),
        )
        .unwrap();
        for changed in [
            "session_id",
            "session_generation",
            "profile_hash",
            "rule_revision",
            "index_frontier",
            "structure_cut",
        ] {
            let mut bad = anchor.clone();
            bad["fixed_cut"][changed] = json!("other");
            assert!(
                reader::projection_fragment(
                    &tx,
                    &bad,
                    &path,
                    sc::fragment::Unit::Items,
                    0,
                    1,
                    262144,
                    METADATA,
                    &mut job()
                )
                .is_err(),
                "{changed}"
            );
        }
        let mut bad = anchor.clone();
        bad["cut_projection_digest"] = json!("qcut3-bad");
        assert!(reader::projection_fragment(
            &tx,
            &bad,
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            262144,
            METADATA,
            &mut job()
        )
        .is_err());
        let mut bad = anchor.clone();
        bad["projection_ref"]["extent"] = json!("8");
        assert!(reader::projection_fragment(
            &tx,
            &bad,
            &path,
            sc::fragment::Unit::Items,
            0,
            1,
            262144,
            METADATA,
            &mut job()
        )
        .is_err());
        drop(tx);
        assert_eq!(conn.total_changes(), before);
    }
    #[test]
    fn record_limit_stops_projection_without_mutating_storage() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let before = f.conn.total_changes();
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut graph = history::HashReader::new(&mut source);
        let limits = JobLimits {
            max_total_records: 1,
            ..job().snapshot().limits
        };
        let mut budget = Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(std::time::Instant::now),
        )
        .unwrap();
        assert!(build(
            &tx,
            &audited,
            &mut graph,
            1,
            HistoryMode::AsKnown,
            METADATA,
            &mut budget
        )
        .is_err());
        assert!(budget.snapshot().stopped.is_some());
        drop(graph);
        drop(tx);
        assert_eq!(f.conn.total_changes(), before);
    }

    #[test]
    fn metadata_limits_bound_expansion_escaping_and_cumulative_reads() {
        let mut nodes = sc::MemoryNodes::new();
        let small = normal_write(&mut nodes, &json!("abcd")).unwrap();
        let escaped = normal_write(&mut nodes, &json!("\0".repeat(30))).unwrap();
        let large = sc::ContentRef::new(Kind::String, &[7; 32], Some(1u64 << 43));
        let mut graph = history::HashReader::new(&mut nodes);
        let mut budget = job();
        let mut source = Metered {
            graph: &mut graph,
            job: &mut budget,
            metadata_remaining: 11,
        };
        assert_eq!(
            render(&mut source, &small).unwrap().view(),
            JsonView::Plain(&json!("abcd"))
        );
        assert_eq!(source.metadata_remaining, 5);
        assert!(render(&mut source, &small)
            .err()
            .unwrap()
            .contains("ProjectionMetadataLimit"));
        source.metadata_remaining = 100;
        assert!(render(&mut source, &escaped)
            .err()
            .unwrap()
            .contains("ProjectionMetadataLimit"));
        assert!(source.metadata_remaining < 100, "转义失败仍扣已输出前缀");
        // 声明长度已越界时先停，不追逐假根、更不尝试8TiB展开。
        assert!(render(&mut source, &large)
            .err()
            .unwrap()
            .contains("ProjectionMetadataLimit"));
    }

    #[test]
    fn all_six_sort_tuples_match_python_with_numeric_utf8_and_withdrawal_edges() {
        let fixtures = json!({
            "objects":[
                {"object_id":"o-z","object_revision":"2","kind":"typed","fact_key":"中"},
                {"object_id":"o-a","object_revision":"10","kind":"typed","fact_key":"é"},
                {"object_id":"shape-b","object_revision":"1","window_start":"2","window_mid":"10","window_end":"12"},
                {"object_id":"shape-a","object_revision":"1","window_start":"2","window_mid":"3","window_end":"12"}],
            "withdrawn_objects":[
                {"object_id":"a","object_revision":"1","first_known_generation":"1","withdrawn_generation":"10"},
                {"object_id":"z","object_revision":"1","first_known_generation":"2","withdrawn_generation":"3"}],
            "witnesses":[{"witness_id":"z","object_id":"a","slot":"10"},{"witness_id":"a","object_id":"a","slot":"2"}],
            "relations":[{"subject":"a","relation_type":"t","object":"中"},{"subject":"a","relation_type":"t","object":"é"}],
            "observations":[{"observation_id":"z","kind":"k","window_start":"0"},{"observation_id":"a","kind":"k","window_start":null}],
            "raw_history":[{"seq":"10","identity_key":"z","revision":"1"},{"seq":"2","identity_key":"a","revision":"10"}]
        });
        let oracle = python_rows(&fixtures);
        let mut nodes = sc::MemoryNodes::new();
        let mut actual = Vec::new();
        for family in FAMILIES {
            let mut ordered = Vec::new();
            for record in fixtures[family].as_array().unwrap() {
                let r = normal_write(&mut nodes, record).unwrap();
                let mut graph = history::HashReader::new(&mut nodes);
                let mut budget = job();
                let mut source = Metered {
                    graph: &mut graph,
                    job: &mut budget,
                    metadata_remaining: METADATA,
                };
                let (order, key) = record_order_key(&mut source, family, r.view()).unwrap();
                ordered.push((order, json!({"family":family,"key":key,"record":record})));
            }
            ordered.sort_by(|a, b| a.0[..].cmp(&b.0[..]));
            actual.extend(ordered.into_iter().map(|(_, record)| record));
        }
        assert_eq!(json!(actual), oracle);
    }
}
