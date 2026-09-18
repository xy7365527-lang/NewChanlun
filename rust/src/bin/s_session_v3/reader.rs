//! #1394 C §5.6：当前一致事务中，将已封存Delta anchor绑定真实历史与范围证明。
//! 这是内部Delta读取接线，尚未注册Q3/CLI/服务，不把本函数视为完整Q capture。
//! Projection、token、validated_capture_digest由Q完整投影/捕获层继续接线；不填伪值。
//! Job计本次证明遍历的唯一节点及从开门起的期限；独立的Q capture/verify预算尚待上层贯通。

use super::*;
use newchan_rust::s_content::{
    fragment::{self, Fragment, Unit},
    job::Job,
    path::PathStep,
    ContentRef,
};
use newchan_rust::session_protocol::required;
use rusqlite::Transaction;

/// 调用方在请求内持有这个事务；返回后不得跨请求继续持锁。
pub(super) fn delta_fragment(
    tx: &Transaction<'_>,
    anchor: &Value,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    job: &mut Job,
) -> Result<Fragment, String> {
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
    if !anchor
        .as_object()
        .is_some_and(|o| o.len() == FIELDS.len() && o.keys().all(|k| FIELDS.contains(&k.as_str())))
        || anchor["kind"] != "delta"
    {
        return Err("SchemaUnsupported：Delta anchor字段必须exact且不能混入projection".into());
    }
    let generation = positive(required(anchor, "generation")?, "delta generation")?;
    let claimed = ContentRef::from_transport_json(&anchor["delta_ref"]).map_err(err)?;
    job.begin_first_read().map_err(err)?;
    // 同一判据：包含来源、manifest配方、独立索引、模板解析及当前目录，不能只验Hash。
    let audited = state::audit(tx)?;
    job.pre_io_check().map_err(err)?;
    let first: i64 = tx
        .query_row(
            "SELECT first_available_generation FROM s_delivery_policy WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
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
    let expected = delta_anchor(cut);
    if claimed != cut.delta || *anchor != expected {
        return Err("IdentityConflict：anchor与当前完整核验所得的同会话已发布Delta不同".into());
    }
    let mut source = content_store::ReadStore::new(tx).map_err(err)?;
    fragment::read(
        &mut source,
        &cut.delta,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        job,
    )
    .map_err(err)
}
pub(super) fn delta_anchor(cut: &history::PublishedCut) -> Value {
    json!({"kind":"delta","session_id":cut.metadata.session_id,"session_generation":cut.metadata.session_generation,
        "generation":cut.metadata.generation.to_string(),"base_cut":cut.metadata.base_cut,"next_cut":cut.metadata.structure_cut,
        "index_frontier":cut.batch_id,"delta_ref":cut.delta.to_transport_json()})
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
    job: &mut Job,
) -> Result<Fragment, String> {
    const FIELDS: [&str; 4] = [
        "kind",
        "fixed_cut",
        "cut_projection_digest",
        "projection_ref",
    ];
    if !anchor
        .as_object()
        .is_some_and(|o| o.len() == FIELDS.len() && o.keys().all(|k| FIELDS.contains(&k.as_str())))
        || anchor["kind"] != "projection"
    {
        return Err("SchemaUnsupported：projection anchor必须是exact四字段".into());
    }
    let generation = newchan_rust::session_protocol::nonnegative(
        required(&anchor["fixed_cut"], "cut_generation")?,
        "cut_generation",
    )?;
    let mode = match required(&anchor["fixed_cut"], "history_mode")? {
        "AsKnown" => projection::HistoryMode::AsKnown,
        "RecomputedWithRevision" => projection::HistoryMode::RecomputedWithRevision,
        _ => return Err("SchemaUnsupported：history_mode不符".into()),
    };
    let claimed = ContentRef::from_transport_json(&anchor["projection_ref"]).map_err(err)?;
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
    if claimed != actual.root || *anchor != actual.anchor() {
        return Err("IdentityConflict：projection anchor与同一读集的完整投影不同".into());
    }
    fragment::read(
        &mut graph,
        &actual.root,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        job,
    )
    .map_err(err)
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
    fn job() -> Job {
        Job::new(JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1048576","max_total_node_bytes":"2097152","max_total_nodes":"10000","max_total_records":"10000","max_total_requests":"20","max_elapsed_ms":"10000","max_staged_bytes":"1048576","max_store_bytes":"4194304"})).unwrap(),Existing{store_bytes:0,staged_bytes:0},Box::new(std::time::Instant::now)).unwrap()
    }
    fn publish(f: &mut Fixture, n: i64) -> Value {
        let id = format!("raw-{n}");
        let (e, t) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&e, &t).unwrap();
        let tx = f.conn.transaction().unwrap();
        let state = state::audit(&tx).unwrap();
        let c = state.cuts.last().unwrap();
        json!({"kind":"delta","session_id":c.metadata.session_id,"session_generation":c.metadata.session_generation,
            "generation":n.to_string(),"base_cut":c.metadata.base_cut,"next_cut":c.metadata.structure_cut,"index_frontier":c.batch_id,"delta_ref":c.delta.to_transport_json()})
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
            REPLY,
            &mut job(),
        )
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
            REPLY,
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
