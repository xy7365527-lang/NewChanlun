//! #1394 C §5.6：当前一致事务中，将已封存Delta anchor绑定真实历史与范围证明。
//! 这是内部Delta读取接线，尚未注册Q3/CLI/服务，不把本函数视为完整Q capture。
//! Projection、token、validated_capture_digest由Q完整投影/捕获层继续接线；不填伪值。
//! Job计本次证明遍历的唯一节点及从开门起的期限；独立的Q capture/verify预算尚待上层贯通。

use super::*;
use newchan_rust::s_content::access::Access;
#[cfg(test)]
use newchan_rust::s_content::job::Job;
use newchan_rust::s_content::{self as sc, TransportRef};
use newchan_rust::s_content::{
    fragment::{self, Fragment, Unit},
    path::PathStep,
    ContentRef,
};
use newchan_rust::session_protocol::required_view;
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::Transaction;
use sc::capture_allocation::audit_json::{self as aj, JsonView};

/// 调用方在请求内持有这个事务；返回后不得跨请求继续持锁。
pub(super) fn delta_fragment(
    tx: &Transaction<'_>,
    anchor: &Value,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    resources: &q_resources::QResources,
    job: &mut dyn Access,
) -> Result<Fragment, String> {
    job.begin_first_read().map_err(err)?;
    let audited = state::audit(tx)?;
    job.pre_io_check().map_err(err)?;
    let cut = validated_delta(tx, &audited, anchor)?;
    let mut source = content_store::ReadStore::new(tx).map_err(err)?;
    q_delta::measure(&mut source, &cut.delta, resources, job)?;
    fragment::read_refview(
        &mut source,
        cut.delta.view(),
        path,
        unit,
        offset,
        limit,
        resources.limit(q_resources::Limit::ReplyBytes),
        job,
    )
    .map_err(err)
}
/// 已核副本与直接事务读者共用同一发布/会话/保留窗口判据。
pub(super) fn validated_delta<'a>(
    tx: &Transaction<'_>,
    audited: &'a state::AuditedState,
    anchor: &Value,
) -> Result<&'a history::PublishedCut, String> {
    validated_delta_view(tx, audited, anchor.into())
}
fn anchor_fields(anchor: JsonView<'_>, fields: &[&str], kind: &str) -> bool {
    anchor
        .as_object()
        .is_some_and(|o| o.len() == fields.len() && o.keys().all(|k| fields.contains(&k)))
        && anchor.get("kind").and_then(JsonView::as_str) == Some(kind)
}
fn first_available(tx: &Transaction<'_>) -> Result<i64, String> {
    let error = |e| {
        err(crate::evidence_store::sql_columns(
            e,
            &["first_available_generation"],
        ))
    };
    let mut stmt = CheckedStatement::prepare(
        tx,
        c"SELECT first_available_generation FROM s_delivery_policy WHERE singleton=1",
    )
    .map_err(error)?;
    let value = stmt
        .next()
        .map_err(error)?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
        .map_err(error)?
        .integer(0)
        .map_err(error)?;
    stmt.finalize().map_err(error)?;
    Ok(value)
}
/// 借用transport声明只核形状；format/root摘要仍留在原来的实际内容读取门。
pub(super) fn validated_delta_view<'a>(
    tx: &Transaction<'_>,
    audited: &'a state::AuditedState,
    anchor: JsonView<'_>,
) -> Result<&'a history::PublishedCut, String> {
    const FIELDS: [&str; 8] = [
        "kind",
        "session_id",
        "session_generation",
        "generation",
        "base_cut",
        "next_cut",
        "index_frontier",
        "delta_ref",
    ];
    if !anchor_fields(anchor, &FIELDS, "delta") {
        return Err("SchemaUnsupported：Delta anchor字段必须exact且不能混入projection".into());
    }
    let generation = positive(required_view(anchor, "generation")?, "delta generation")?;
    let claimed = TransportRef::from_json_view(anchor.get("delta_ref").unwrap_or(JsonView::Null))
        .map_err(err)?;
    let first = first_available(tx)?;
    let cut = audited
        .cuts
        .iter()
        .find(|c| c.metadata.generation == generation)
        .ok_or_else(|| "Incomplete/ContentAnchorUnavailable：指定Delta未发布".to_string())?;
    if generation < first {
        return Err(
            "Incomplete/ContentAnchorUnavailable：Delta已退出交付保留窗口，须沿Watch Gap重建"
                .into(),
        );
    }
    let reference = cut.delta.view();
    let expected = delta_anchor_view(cut);
    if claimed.format() != reference.format()
        || claimed.root() != reference.root()
        || claimed.kind() != reference.kind()
        || claimed.extent() != reference.extent()
        || anchor != expected.view()
    {
        return Err("IdentityConflict：anchor与当前完整核验所得的同会话已发布Delta不同".into());
    }
    Ok(cut)
}
pub(super) struct DeltaAnchor<'a> {
    cut: &'a history::PublishedCut,
    generation: [u8; 20],
    start: usize,
    reference: projection::TransportView<'a>,
}
pub(super) fn delta_anchor_view(cut: &history::PublishedCut) -> DeltaAnchor<'_> {
    let mut generation = [0u8; 20];
    let mut n = cut.metadata.generation.unsigned_abs();
    let mut start = 20;
    loop {
        start -= 1;
        generation[start] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if cut.metadata.generation < 0 {
        start -= 1;
        generation[start] = b'-';
    }
    DeltaAnchor {
        cut,
        generation,
        start,
        reference: projection::TransportView::new(cut.delta.view()),
    }
}
impl DeltaAnchor<'_> {
    pub(super) fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("Delta anchor八字段唯一")
    }
}
impl aj::JsonObject for DeltaAnchor<'_> {
    fn len(&self) -> usize {
        8
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        let h = &self.cut.metadata;
        Some(match index {
            0 => ("kind", JsonView::String("delta")),
            1 => ("session_id", JsonView::String(h.session_id.as_str())),
            2 => (
                "session_generation",
                JsonView::String(h.session_generation.as_str()),
            ),
            3 => (
                "generation",
                JsonView::String(
                    std::str::from_utf8(&self.generation[self.start..]).expect("ASCII整数"),
                ),
            ),
            4 => ("base_cut", JsonView::String(h.base_cut.as_str())),
            5 => ("next_cut", JsonView::String(h.structure_cut.as_str())),
            6 => (
                "index_frontier",
                JsonView::String(self.cut.batch_id.as_str()),
            ),
            7 => ("delta_ref", self.reference.view()),
            _ => return None,
        })
    }
}
/// 普通旧Value入口仍由同一借用表示核构造；Owned调用方直接持view或显式预算输出。
pub(super) fn delta_anchor(cut: &history::PublishedCut) -> Value {
    assert!(
        matches!(&cut.delta, audit_store::RefValue::Plain(_))
            && [
                &cut.metadata.session_id,
                &cut.metadata.session_generation,
                &cut.metadata.base_cut,
                &cut.metadata.structure_cut,
                &cut.batch_id
            ]
            .iter()
            .all(|v| matches!(v, super::business_json::TextValue::Plain(_))),
        "旧Value anchor仅属于普通cut；Owned请借delta_anchor_view"
    );
    serde_json::to_value(delta_anchor_view(cut).view()).expect("已核Delta anchor仅含JSON值")
}

fn err(e: impl std::fmt::Display) -> String {
    format!("S3ContentReadInvalid：{e}")
}

/// 每次在当前一致事务重建指定完整Q根；旧cut不受Delta交付窗口淘汰影响。
pub(super) fn projection_fragment(
    tx: &Transaction<'_>,
    anchor: &Value,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    max_metadata_bytes: u64,
    job: &mut dyn Access,
) -> Result<Fragment, String> {
    let anchor_view: JsonView<'_> = anchor.into();
    const FIELDS: [&str; 4] = [
        "kind",
        "fixed_cut",
        "cut_projection_digest",
        "projection_ref",
    ];
    if !anchor_fields(anchor_view, &FIELDS, "projection") {
        return Err("SchemaUnsupported：projection anchor必须是exact四字段".into());
    }
    let fixed = anchor_view.get("fixed_cut").unwrap_or(JsonView::Null);
    let generation = newchan_rust::session_protocol::nonnegative(
        required_view(fixed, "cut_generation")?,
        "cut_generation",
    )?;
    let mode = match required_view(fixed, "history_mode")? {
        "AsKnown" => projection::HistoryMode::AsKnown,
        "RecomputedWithRevision" => projection::HistoryMode::RecomputedWithRevision,
        _ => return Err("SchemaUnsupported：history_mode不符".into()),
    };
    TransportRef::from_json_view(anchor_view.get("projection_ref").unwrap_or(JsonView::Null))
        .map_err(err)?;
    job.begin_first_read().map_err(err)?;
    let audited = state::audit(tx)?;
    job.pre_io_check().map_err(err)?;
    let mut source = content_store::ReadStore::new(tx).map_err(err)?;
    let mut graph = history::HashReader::new(&mut source);
    let actual = projection::build(
        tx,
        &audited,
        &mut graph,
        generation,
        mode,
        max_metadata_bytes,
        job,
    )?;
    check_projection_anchor(&actual, anchor)?;
    fragment::read_refview(
        &mut graph,
        actual.root.view(),
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        job,
    )
    .map_err(err)
}
pub(super) fn check_projection_anchor(
    actual: &projection::Projection,
    anchor: &Value,
) -> Result<(), String> {
    check_projection_anchor_view(actual, anchor.into())
}
pub(super) fn check_projection_anchor_view(
    actual: &projection::Projection,
    anchor: JsonView<'_>,
) -> Result<(), String> {
    if anchor != actual.anchor_view().view() {
        Err("IdentityConflict：projection anchor与同一读集的完整投影不同".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::{
        self as sc,
        job::{Existing, JobLimits},
    };
    use prepared::tests::Fixture;
    const REPLY: u64 = 262144;
    fn resources() -> q_resources::QResources {
        let mut config = q_resources::tests::functional_input();
        config["limits"]["max_reply_bytes"] = json!(REPLY.to_string());
        q_resources::QResources::parse(&serde_json::to_vec(&config).unwrap()).unwrap()
    }
    // TestOnly，不是产品期限（#1394 B1）：功能测试 job 的 max_elapsed_ms 放长到 600000，不随 runner 速度超期；测 job 期限的用例自带受控时钟。
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"2097152","max_total_nodes":"10000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"600000","max_staged_bytes":"1048576","max_store_bytes":"4194304"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(std::time::Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) -> Value {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
        let tx = f.conn.transaction().unwrap();
        let state = state::audit(&tx).unwrap();
        let c = state.cuts.last().unwrap();
        json!({"kind":"delta","session_id":c.metadata.session_id,"session_generation":c.metadata.session_generation,
            "generation":n.to_string(),"base_cut":c.metadata.base_cut,"next_cut":c.metadata.structure_cut,"index_frontier":c.batch_id,"delta_ref":c.delta})
    }
    fn read(f: &mut Fixture, anchor: &Value) -> Result<Fragment, String> {
        let tx = f.conn.transaction().unwrap();
        delta_fragment(
            &tx,
            anchor,
            &[PathStep::Field("raw_history_added".into())],
            Unit::Items,
            0,
            1,
            &resources(),
            &mut job(),
        )
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_delta_anchor_views_keep_legacy_order_without_early_digest_or_owner_copy() {
        use sc::capture_allocation::{audit_json::OwnedJson, OwnedBudget};
        let mut f = Fixture::with_retain(1);
        let old = publish(&mut f, 1);
        let current = publish(&mut f, 2);
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        let budget = OwnedBudget::new(1_048_576);
        let owned = OwnedJson::from_view((&current).into(), &budget).unwrap();
        let before = budget.snapshot();
        let plain = validated_delta(&tx, &audited, &current).unwrap();
        let borrowed = validated_delta_view(&tx, &audited, owned.view()).unwrap();
        assert!(std::ptr::eq(plain, borrowed));
        assert_eq!(delta_anchor_view(plain).view(), (&current).into());
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let mut unpublished = current.clone();
        unpublished["generation"] = json!("3");
        unpublished["delta_ref"]["format"] = json!("unknown");
        let mut expired = old.clone();
        expired["delta_ref"]["format"] = json!("unknown");
        let mut bad_identity = current.clone();
        bad_identity["delta_ref"]["format"] = json!("unknown");
        for (anchor, first) in [
            (unpublished, "指定Delta未发布"),
            (expired, "保留窗口"),
            (bad_identity, "IdentityConflict"),
        ] {
            let plain = validated_delta(&tx, &audited, &anchor).err().unwrap();
            let owned = OwnedJson::from_view((&anchor).into(), &budget).unwrap();
            let borrowed = validated_delta_view(&tx, &audited, owned.view())
                .err()
                .unwrap();
            assert_eq!(plain, borrowed);
            assert!(borrowed.contains(first), "{borrowed}");
            assert!(!borrowed.contains("ContentRef.format"), "{borrowed}");
            drop(owned);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn persisted_delta_fragments_survive_next_commit_and_verify_without_database() {
        let mut f = Fixture::new();
        let a = publish(&mut f, 1);
        let first = read(&mut f, &a).unwrap();
        assert_eq!(first.total, 1);
        assert_eq!(first.items[0]["kind"], "object");
        publish(&mut f, 2);
        assert_eq!(read(&mut f, &a).unwrap(), first);
        let root = ContentRef::from_transport_json(&a["delta_ref"]).unwrap();
        fragment::verify(
            &root,
            &[PathStep::Field("raw_history_added".into())],
            Unit::Items,
            0,
            1,
            &first,
            REPLY,
            &mut job(),
        )
        .unwrap();
        let mut c = Connection::open(f.db_path()).unwrap();
        c.pragma_update(None, "query_only", true).unwrap();
        let before = c.total_changes();
        let tx = c.transaction().unwrap();
        let part = delta_fragment(
            &tx,
            &a,
            &[PathStep::Field("raw_history_added".into())],
            Unit::Items,
            0,
            1,
            &resources(),
            &mut job(),
        )
        .unwrap();
        assert_eq!(part, first);
        drop(tx);
        assert_eq!(c.total_changes(), before);
    }

    #[test]
    fn root_headers_cross_session_and_unpublished_nodes_never_authorize_reads() {
        let mut f = Fixture::new();
        let a = publish(&mut f, 1);
        for key in [
            "session_id",
            "session_generation",
            "base_cut",
            "next_cut",
            "index_frontier",
            "kind",
        ] {
            let mut changed = a.clone();
            changed[key] = json!("other");
            assert!(read(&mut f, &changed).is_err(), "{key}");
        }
        let mut changed = a.clone();
        changed["extra"] = json!(true);
        assert!(read(&mut f, &changed).is_err());
        let mut changed = a.clone();
        changed["generation"] = json!("2");
        assert!(read(&mut f, &changed)
            .unwrap_err()
            .contains("ContentAnchorUnavailable"));
        let mut changed = a.clone();
        changed["delta_ref"]["extent"] = json!("18");
        assert!(read(&mut f, &changed).is_err());
        let unpublished = {
            let tx = f.conn.transaction().unwrap();
            let root = sc::write_value(
                &mut content_store::TxStore::new(&tx).unwrap(),
                &sc::Value::string("未发布值"),
            )
            .unwrap()
            .content_ref();
            tx.commit().unwrap();
            root
        };
        let mut changed = a.clone();
        changed["delta_ref"] = unpublished.to_transport_json();
        assert!(read(&mut f, &changed)
            .unwrap_err()
            .contains("IdentityConflict"));
        assert!(read(&mut f, &unpublished.to_transport_json()).is_err());
        assert!(read(&mut f, &a).is_ok());
    }

    #[test]
    fn retention_and_full_current_state_validation_gate_old_and_new_fragments() {
        let mut f = Fixture::with_retain(1);
        let old = publish(&mut f, 1);
        let current = publish(&mut f, 2);
        assert!(read(&mut f, &old)
            .unwrap_err()
            .contains("ContentAnchorUnavailable"));
        assert!(read(&mut f, &current).is_ok());
        f.conn
            .execute(
                "UPDATE catalog SET run_status='not_run' WHERE catalog_id='CC-005'",
                [],
            )
            .unwrap();
        assert!(
            read(&mut f, &current).is_err(),
            "根仍存在也不能略过完整当前状态核验"
        );
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 2);
    }
}
