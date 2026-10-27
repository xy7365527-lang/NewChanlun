//! #1394 C §5.4：把已准备的内容发布、独立索引和控制前沿提交到同一个事务。
//!
//! 调用者须先在当前一致性读集中审计完整旧状态，再在本事务中生成节点、暂写真实出生/
//! 撤回索引并准备三根。本函数接管事务：任何错误都回滚这些暂写；成功才发布实际根、
//! Delta、clock、delivery 和 meta。它不代替对象来源、全部历史或 Delta 方程的业务审计。
//! 本模块不提供 CLI；完整 writer 尚未接入，不能把本模块测试当完整 S3 验收。

use super::audit_store::{CodecRead, NativeAdmission};
use super::business_json::TextValue;
use super::{control::S3Control, *};
#[cfg(test)]
use content_manifest::ManifestRoots;
use content_manifest::{ManifestMetadata, ManifestRootSource, UnpublishedManifest};
use newchan_rust::s_content::{self as sc, path::PathStep, ContentRef, Kind};
use rusqlite::Transaction;
use sc::capture_allocation::native_owned::NativeValue;
use sc::capture_allocation::scratch::WorkScope;

/// S3 私有发布索引。原 `structure_deltas` 保留共同控制列和 delivery 外键；其 `delta_json`
/// 在明确的 s-session/3 域中存四字段 ContentRef，真实 Delta 在这里同根绑定。
/// 旧 v1/v2 入口先拒绝新域，不把这个引用误读为旧 Delta 全文。
pub(super) const SCHEMA: &str = "CREATE TABLE s_content_publications (
          generation INTEGER PRIMARY KEY CHECK(generation>=1),
          batch_id TEXT NOT NULL UNIQUE,
          manifest_root BLOB NOT NULL CHECK(typeof(manifest_root)='blob' AND length(manifest_root)=32),
          snapshot_root BLOB NOT NULL CHECK(typeof(snapshot_root)='blob' AND length(snapshot_root)=32),
          delta_root BLOB NOT NULL CHECK(typeof(delta_root)='blob' AND length(delta_root)=32),
          FOREIGN KEY(generation) REFERENCES structure_deltas(generation)
        );";

pub(super) fn init_schema(tx: &Transaction<'_>) -> Result<(), String> {
    content_store::TxStore::new(tx).map_err(err)?;
    if meta_get_opt(tx, "protocol_revision")?.as_deref() != Some(PROTOCOL_REVISION) {
        return Err(err("根协议标记不是 S3"));
    }
    tx.execute_batch(SCHEMA).map_err(err)
}

#[derive(Debug)]
pub(super) struct CommitReceipt {
    #[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
    pub generation: i64,
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub structure_cut: TextValue,
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub batch_id: TextValue,
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub snapshot: NativeValue<ContentRef>,
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub delta: NativeValue<ContentRef>,
    pub message_receipt: Value,
}

/// `tx` 按值接管。包括调用者已经暂写的节点/独立索引在内，失败全部回滚。
#[cfg(test)]
pub(super) fn publish_prepared(
    mut tx: Transaction<'_>,
    begin_token: &str,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
) -> Result<CommitReceipt, String> {
    let receipt = publish_borrowed(
        &mut tx,
        begin_token,
        metadata,
        roots,
        manifest,
        &ThetaConfig::default().record_labels(),
    )?;
    tx.commit().map_err(err)?;
    Ok(receipt)
}

#[cfg_attr(
    all(not(test), feature = "s_incremental"),
    allow(
        dead_code,
        reason = "#1608：s_capture_native_testonly 的非测试构建不触达此项"
    )
)]
pub(super) fn publish_borrowed(
    tx: &mut Transaction<'_>,
    begin_token: &str,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    policy_records: &str,
) -> Result<CommitReceipt, String> {
    publish_prepared_inner(
        tx,
        begin_token,
        metadata,
        roots,
        manifest,
        policy_records,
        #[cfg(all(test, feature = "s_incremental"))]
        None,
        #[cfg(all(test, feature = "s_incremental"))]
        &[],
        #[cfg(feature = "s_incremental")]
        None,
        None,
    )
}
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn publish_borrowed_staged(
    tx: &mut Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    policy_records: &str,
    stage: &staged_nodes::StageOwner,
    runtime: &[sc::NodeDigest],
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
    work: Option<&WorkScope>,
) -> Result<CommitReceipt, String> {
    publish_prepared_inner(
        tx,
        token,
        m,
        roots,
        manifest,
        policy_records,
        Some(stage),
        runtime,
        workspace,
        work,
    )
}
#[cfg(feature = "s_incremental")]
#[allow(
    dead_code,
    reason = "#1608：旧包装入口，全仓无调用方（prepared.rs 直接调 _admitted 版本）；去留见 #1635"
)]
pub(super) fn publish_borrowed_with_workspace(
    tx: &mut Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    policy_records: &str,
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
) -> Result<CommitReceipt, String> {
    publish_borrowed_with_workspace_admitted(
        tx,
        token,
        m,
        roots,
        manifest,
        policy_records,
        workspace,
        None,
    )
}
#[cfg(feature = "s_incremental")]
pub(super) fn publish_borrowed_with_workspace_admitted(
    tx: &mut Transaction<'_>,
    token: &str,
    m: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    policy_records: &str,
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
    work: Option<&WorkScope>,
) -> Result<CommitReceipt, String> {
    publish_prepared_inner(
        tx,
        token,
        m,
        roots,
        manifest,
        policy_records,
        #[cfg(test)]
        None,
        #[cfg(test)]
        &[],
        workspace,
        work,
    )
}
fn publish_prepared_inner(
    tx: &mut Transaction<'_>,
    begin_token: &str,
    metadata: &ManifestMetadata,
    roots: &(impl ManifestRootSource + ?Sized),
    manifest: &UnpublishedManifest,
    policy_records: &str,
    #[cfg(all(test, feature = "s_incremental"))] stage: Option<&staged_nodes::StageOwner>,
    #[cfg(all(test, feature = "s_incremental"))] runtime: &[sc::NodeDigest],
    #[cfg(feature = "s_incremental")] workspace: Option<
        content_manifest::PreparedManifestWorkspace,
    >,
    work: Option<&WorkScope>,
) -> Result<CommitReceipt, String> {
    let m = metadata;
    let (count, head): (i64, i64) = tx
        .query_row(
            "SELECT COUNT(*),COALESCE(MAX(generation),0) FROM s_content_publications",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(err)?;
    if count != head || m.generation.checked_sub(1) != Some(head) {
        return Err(err("发布历史缺代或本次代号不是下一代"));
    }
    #[cfg(all(test, feature = "s_incremental"))]
    let resolved = match stage {
        Some(stage) => content_publication::resolve_current_births_staged(
            tx,
            begin_token,
            m,
            roots,
            manifest,
            stage,
            workspace,
            work,
        )?,
        None => content_publication::resolve_current_births_with_workspace_admitted(
            tx,
            begin_token,
            m,
            roots,
            manifest,
            workspace,
            work,
        )?,
    };
    #[cfg(all(not(test), feature = "s_incremental"))]
    let resolved = content_publication::resolve_current_births_with_workspace_admitted(
        tx,
        begin_token,
        m,
        roots,
        manifest,
        workspace,
        work,
    )?;
    #[cfg(not(feature = "s_incremental"))]
    let resolved =
        content_publication::resolve_current_births(&tx, begin_token, m, roots, manifest)?;
    let clock = v2::stored_clock(&tx)?;
    let control = S3Control::attach(&tx, &clock)?;
    let source = &m.source_commit;
    let view = control.load_pending(
        &source.source_namespace,
        &source.source_epoch,
        &source.message_id,
    )?;
    if view.is_committed() || view.is_fresh() {
        return Err(err("本次发布消息不是已接纳未提交状态"));
    }
    if control.commit_time(&view)?.as_deref() != Some(m.semantic_commit_ns.as_str()) {
        return Err(err("本次提交时刻与持久 Begin 不同"));
    }

    let (seq_range, run, evidence_ref) = {
        #[cfg(all(test, feature = "s_incremental"))]
        let mut store = content_store::TxStore::new_with_stage(&tx, stage)
            .map_err(err)?
            .with_work(work);
        #[cfg(not(all(test, feature = "s_incremental")))]
        let mut store = content_store::TxStore::new(&tx)
            .map_err(err)?
            .with_work(work);
        let seq_range = small_field(&mut store, &resolved.delta, "seq_range")?;
        let run = small_field(&mut store, &resolved.delta, "catalog_run_status")?;
        let run = run
            .as_str()
            .ok_or_else(|| err("目录运行状态不是文本"))?
            .to_owned();
        // 活动工作域下查找签发的原正文许可随引用持有，至函数结束随对象释放。
        let evidence_ref = sc::path::lookup_path_native(
            &mut store,
            (&resolved.delta).into(),
            &[PathStep::Field("catalog_evidence".into())],
        )
        .map_err(err)?;
        if evidence_ref.kind != Kind::Object {
            return Err(err("目录证据不是对象"));
        }
        (canonical_json(&seq_range), run, evidence_ref)
    };

    // 先建真实 Delta 控制行，delivery 的外键随后才能引用本代；大值存显式内容引用。
    tx.execute(
        "INSERT INTO structure_deltas(generation,session_id,catalog_revision,base_cut,next_cut,
         seq_range_json,index_frontier,input_frontier,catalog_run_status,catalog_evidence_json,delta_json,policy_records)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![m.generation,m.session_id,m.catalog_revision,m.base_cut,m.structure_cut,
            seq_range,manifest.batch_id,m.input_frontier,run,
            reference_json(&evidence_ref)?,reference_json(&resolved.delta)?,policy_records],
    ).map_err(err)?;
    tx.execute(
        "INSERT INTO s_content_publications(generation,batch_id,manifest_root,snapshot_root,delta_root)
         VALUES(?1,?2,?3,?4,?5)",
        params![m.generation,manifest.batch_id,&manifest.value.digest[..],
            &resolved.snapshot.digest().map_err(err)?[..],&resolved.delta.digest().map_err(err)?[..]],
    ).map_err(err)?;
    control.commit(&view, m.generation, m.input_frontier)?;
    let admission = NativeAdmission(work);
    let generation_text = admission
        .native_format(format_args!("{}", m.generation))
        .map_err(err)?;
    let frontier_text = admission
        .native_format(format_args!("{}", m.input_frontier))
        .map_err(err)?;
    for (key, value) in [
        ("generation", generation_text.as_str()),
        ("structure_cut", m.structure_cut.as_str()),
        ("index_frontier", manifest.batch_id.as_str()),
        ("last_advance_frontier", frontier_text.as_str()),
        ("advance_state", "idle"),
    ] {
        meta_set(&tx, key, value)?;
    }
    // 同旧生产 Commit：begin_* 保留为最后一次 Begin 的历史证据，idle 表示占用已释放。
    // 不把新内容格式变更扩成 Begin 证据的删除；换代清理仍属于 recover。
    // 收据从刚推进的真实 message 行回读；提交失败时不能向调用者交出成功结果。
    let message_receipt = v2::lookup(
        &tx,
        &source.source_namespace,
        &source.source_epoch,
        &source.message_id,
        &source.payload_hash,
    )?;
    #[cfg(all(test, feature = "s_incremental"))]
    if let Some(stage) = stage {
        if stage.work().is_some() {
            // 本代显式控制/目录/生成根；generated_state包含catalog_axes各轴evidence。
            let current = [
                manifest.value.digest,
                resolved.snapshot.digest().map_err(err)?,
                resolved.delta.digest().map_err(err)?,
                evidence_ref.digest().map_err(err)?,
                roots.view().generated_state.digest().map_err(err)?,
                roots.view().publication_template.digest().map_err(err)?,
                roots.view().delta_template.digest().map_err(err)?,
                metadata.scope.digest().map_err(err)?,
            ];
            stage.flush_cut(tx, staged_nodes::FlushCut::Final(&current))?;
            stage.flush_cut(tx, staged_nodes::FlushCut::Final(runtime))?;
        } else {
            // 仅既有无scope诊断臂保留旧计数口径；strict publication不走此分支。
            stage.flush(tx, runtime)?;
        }
    }
    Ok(CommitReceipt {
        generation: m.generation,
        structure_cut: TextValue::from_str_admitted(&m.structure_cut, None, work)?,
        batch_id: manifest.batch_id.try_clone_admitted(work)?,
        snapshot: resolved.snapshot,
        delta: resolved.delta,
        message_receipt,
    })
}

fn err(e: impl std::fmt::Display) -> String {
    format!("PublicationInvalid：{e}")
}

fn reference_json(reference: &ContentRef) -> Result<String, String> {
    reference.digest().map_err(err)?;
    Ok(canonical_json(&reference.to_transport_json()))
}

/// 只读 seq_range/运行状态等固定小控制字段；大目录证据和集合保持引用。
fn small_field(
    store: &mut dyn sc::NodeStore,
    root: &ContentRef,
    field: &str,
) -> Result<Value, String> {
    // 活动工作域下查找签发的原正文许可随引用持有，渲染后随对象释放。
    let reference =
        sc::path::lookup_path_native(store, root.into(), &[PathStep::Field(field.into())])
            .map_err(err)?;
    let mut bytes = Vec::new();
    sc::render_canonical(store, &reference, &mut bytes).map_err(err)?;
    strict_json(&bytes)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use content_manifest::SourceCommit;
    use sc::Value as V;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    pub(crate) struct Fixture {
        path: PathBuf,
        pub(crate) conn: Connection,
        pub(crate) token: String,
        pub(crate) envelope: Value,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    fn value(v: &Value) -> V {
        match v {
            Value::Null => V::Null,
            Value::Bool(b) => V::Bool(*b),
            Value::String(s) => V::String(s.clone()),
            Value::Number(n) => V::Number(sc::NumberToken::parse(&n.to_string()).unwrap()),
            Value::Array(a) => V::Array(a.iter().map(value).collect()),
            Value::Object(o) => V::Object(o.iter().map(|(k, v)| (k.clone(), value(v))).collect()),
        }
    }
    fn set(v: &mut V, key: &str, replacement: V) {
        let V::Object(fields) = v else {
            panic!("fixture object")
        };
        fields.iter_mut().find(|(k, _)| k == key).unwrap().1 = replacement;
    }
    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    pub(crate) fn fixture() -> Fixture {
        let path = std::env::temp_dir().join(format!(
            "s1394-commit-{}-{}-{}",
            std::process::id(),
            now_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        let db = path.join("s.sqlite");
        let clock_path = path.join("clock.json");
        let clock_value = json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"commit-fixture","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{"input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"}]}}});
        std::fs::write(&clock_path, canonical_json(&clock_value)).unwrap();
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        super::super::init_v3(
            &db,
            "commit-fixture",
            &base.join("catalog/signed-catalog.json"),
            &base.join("profiles/testonly_tick_1_1_ohlc.json"),
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        let mut conn = open_db(&db).unwrap();
        let clock = v2::ClockPlan::parse(clock_value).unwrap();
        let raw = json!({"schema_revision":"1","session_id":"commit-fixture","source_namespace":"test.feed","source_epoch":"1","instrument":"TEST.TICK","profile":"testonly_tick_1_1_ohlc","events":[{"event_id":"event-1","revision":"1","seq":"0","received_at":"2026-09-18T00:00:00Z","raw_text":"10000","price":"10000","timestamp":"0","volume":"1"}]});
        let payload = json!({"op":"ingest","target_session_id":"commit-fixture","target_session_generation":"1","writer_epoch":"1","clock_event_id":"input-1","raw_input":raw});
        let envelope = json!({"schema_revision":"s-session/3","session_id":"commit-fixture","session_generation":"1","source_namespace":"test.feed","source_epoch":"1","message_id":"message-1","producer_id":"fixture-driver","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
        // 真接纳与真 Begin 分别持久，模拟 Commit 失败时仍须保留的待恢复状态。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control.inspect_ingest(envelope.clone()).unwrap();
        let input = tb02a::parse_input(&raw).unwrap();
        let accepted = raw_input::accept_events(&tx, &input, Some("1")).unwrap();
        control.accept(&view, &accepted).unwrap();
        tx.commit().unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control.inspect_ingest(envelope.clone()).unwrap();
        let token = control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        assert_eq!(
            meta_get_opt(&tx, "begin_token").unwrap().as_deref(),
            Some(token.as_str())
        );
        assert_eq!(
            meta_get_opt(&tx, "advance_state").unwrap().as_deref(),
            Some(format!("begun:{token}:1:0").as_str())
        );
        tx.commit().unwrap();
        Fixture {
            path,
            conn,
            token,
            envelope,
        }
    }

    /// 固定发布事务夹具：真实接纳、Begin、原始行与首次观察索引；对象集为空。
    /// 不声称这是完整结构生成器输出，结构/身份/Delta业务审计由生成接线另验。
    pub(crate) fn prepare(
        tx: &Transaction<'_>,
        envelope: &Value,
    ) -> (ManifestMetadata, ManifestRoots, UnpublishedManifest) {
        let text = |key| meta_get_opt(tx, key).unwrap().unwrap();
        let raw = read_raw_events(tx).unwrap();
        let raw_wire = raw
            .iter()
            .map(project_raw_event_wire)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let scope = strict_json(text("scope").as_bytes()).unwrap();
        let mut store = content_store::TxStore::new(tx).unwrap();
        let m = ManifestMetadata {
            session_id: text("session_id"),
            session_generation: "1".into(),
            writer_epoch: "1".into(),
            generation: 1,
            base_cut: "cut-0".into(),
            structure_cut: "cut-1".into(),
            input_frontier: 0,
            source_commit: SourceCommit {
                source_namespace: "test.feed".into(),
                source_epoch: "1".into(),
                message_id: "message-1".into(),
                payload_hash: envelope["payload_hash"].as_str().unwrap().into(),
                producer_id: "fixture-driver".into(),
                producer_epoch: "1".into(),
                clock_event_id: "input-1".into(),
                attempt_index: 0,
            },
            clock_plan_hash: v2::stored_clock(tx).unwrap().hash,
            semantic_commit_ns: "12".into(),
            profile_id: text("profile_id"),
            profile_hash: text("profile_hash"),
            rule_revision: text("rule_revision"),
            catalog_revision: text("catalog_revision"),
            scope: sc::write_value(&mut store, &value(&scope))
                .unwrap()
                .content_ref(),
        };
        let observation = json!({"kind":"insufficient_knowledge","window_start":null,"window_mid":null,"window_end":null,"reason":"fewer_than_three_bars","detail":{"raw_events":1}});
        let id = observation_id(&observation);
        let observation_root = sc::write_value(&mut store, &value(&observation)).unwrap();
        let mut ob = value(&observation);
        let V::Object(fields) = &mut ob else {
            unreachable!()
        };
        fields.push(("observation_id".into(), V::String(id.clone())));
        fields.push(("batch_id".into(), V::BatchSelf));
        let generated = value(
            &json!({"generation":1,"structure_cut":"cut-1","input_frontier":0,"catalog_revision":m.catalog_revision,"profile_id":m.profile_id,"profile_hash":m.profile_hash,"rule_revision":m.rule_revision,"raw_events":raw,"effective_events":raw,"objects":[],"witnesses":[],"relations":[],"observations":[observation],"scope":scope}),
        );
        let mut snapshot = value(
            &json!({"session_id":m.session_id,"generation":"1","structure_cut":"cut-1","catalog_revision":m.catalog_revision,"scope":scope,"index_frontier":null,"profile_id":m.profile_id,"profile_hash":m.profile_hash,"history_mode":"RecomputedWithRevision","as_of_generation":null,"objects":[],"withdrawn_objects":[],"witnesses":[],"relations":[],"observations":[],"raw_history":raw_wire,"input_frontier":"0","seq_range":{"from":"0","to":"0"},"catalog_run_status":"not_run","catalog_evidence":null}),
        );
        let mut delta = value(
            &json!({"generation":"1","session_id":m.session_id,"catalog_revision":m.catalog_revision,"base_cut":"cut-0","next_cut":"cut-1","seq_range":{"from":"0","to":"0"},"index_frontier":null,"input_frontier":"0","catalog_run_status":"not_run","catalog_evidence":null,"upserts":[],"withdrawals":[],"replaces":[],"witnesses":[],"relations":[],"observations":[],"raw_history_added":raw_wire}),
        );
        for root in [&mut snapshot, &mut delta] {
            set(root, "index_frontier", V::BatchSelf);
            set(
                root,
                "catalog_evidence",
                V::Object(vec![("batch_id".into(), V::BatchSelf)]),
            );
            set(root, "observations", V::Array(vec![ob.clone()]));
        }
        let roots = ManifestRoots {
            generated_state: sc::write_value(&mut store, &generated)
                .unwrap()
                .content_ref(),
            publication_template: sc::write_template(&mut store, &snapshot)
                .unwrap()
                .content_ref(),
            delta_template: sc::write_template(&mut store, &delta)
                .unwrap()
                .content_ref(),
        };
        let manifest = content_manifest::assemble_unpublished(&mut store, &m, &roots).unwrap();
        let template = sc::path::lookup_path(
            &mut store,
            &roots.publication_template,
            &[
                sc::path::PathStep::Field("observations".into()),
                sc::path::PathStep::Index(0),
            ],
        )
        .unwrap();
        let wire = sc::path::replace_batch_placeholder(
            &mut store,
            &template,
            &[sc::path::PathStep::Field("batch_id".into())],
            &manifest.batch_id,
        )
        .unwrap();
        tx.execute("INSERT INTO s_content_observations VALUES(?1,'insufficient_knowledge',NULL,?2,?3,?4,1)",params![id,manifest.batch_id,observation_root.digest.as_slice(),wire.digest().unwrap().as_slice()]).unwrap();
        (m, roots, manifest)
    }

    #[test]
    fn prepared_commit_publishes_content_control_and_receipt_atomically() {
        let mut f = fixture();
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (m, roots, manifest) = prepare(&tx, &f.envelope);
        let result = publish_prepared(tx, &f.token, &m, &roots, &manifest).unwrap();
        assert_eq!(result.message_receipt["kind"], "Committed");
        assert_eq!(result.message_receipt["publication"]["generation"], "1");
        assert_eq!(result.generation, 1);
        assert_eq!(result.structure_cut, "cut-1");
        assert_eq!(result.batch_id, manifest.batch_id);
        assert_eq!(count(&f.conn, "s_content_publications"), 1);
        assert_eq!(count(&f.conn, "structure_deltas"), 1);
        assert_eq!(count(&f.conn, "s_delivery_refs"), 1);
        assert_eq!(count(&f.conn, "s_clock_events"), 3);
        assert_eq!(
            meta_get_opt(&f.conn, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert_eq!(
            meta_get_opt(&f.conn, "begin_token").unwrap().as_deref(),
            Some(f.token.as_str())
        );
        let tx = f.conn.transaction().unwrap();
        let mut reader = content_store::ReadStore::new(&tx).unwrap();
        sc::verify(&mut reader, &result.snapshot).unwrap();
        sc::verify(&mut reader, &result.delta).unwrap();
        let saved: String = tx
            .query_row("SELECT delta_json FROM structure_deltas", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            ContentRef::from_transport_json(&strict_json(saved.as_bytes()).unwrap()).unwrap(),
            *result.delta
        );
    }

    #[test]
    fn prepared_commit_late_failure_rolls_back_nodes_indices_clock_and_frontiers() {
        let mut f = fixture();
        // 失败发生在根、Delta、message 和 clock 都已在事务内暂写之后。
        f.conn.execute_batch("CREATE TRIGGER fail_publication BEFORE UPDATE OF value ON meta WHEN NEW.key='index_frontier' BEGIN SELECT RAISE(ABORT,'test-late-fault'); END;").unwrap();
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (m, roots, manifest) = prepare(&tx, &f.envelope);
        assert!(count(&tx, "s_content_nodes") > 0);
        let error = publish_prepared(tx, &f.token, &m, &roots, &manifest).unwrap_err();
        assert!(error.contains("test-late-fault"), "{error}");
        for table in [
            "s_content_publications",
            "structure_deltas",
            "s_delivery_refs",
            "s_content_nodes",
            "s_content_observations",
        ] {
            assert_eq!(count(&f.conn, table), 0, "{table}");
        }
        assert_eq!(count(&f.conn, "raw_events"), 1);
        assert_eq!(count(&f.conn, "s_clock_events"), 2);
        assert_eq!(
            meta_get_opt(&f.conn, "generation").unwrap().as_deref(),
            Some("0")
        );
        assert_eq!(
            meta_get_opt(&f.conn, "advance_state").unwrap().as_deref(),
            Some(format!("begun:{}:1:0", f.token).as_str())
        );
        let state: String = f
            .conn
            .query_row("SELECT status FROM s_input_messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "AcceptedPending");
    }

    #[test]
    fn prepared_commit_sqlite_commit_failure_returns_no_receipt_and_rolls_back() {
        let mut f = fixture();
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        // 故障延后到 SQLite 的 COMMIT 本身：所有内部语句先成功，最后的外键检查拒绝。
        tx.execute_batch("PRAGMA defer_foreign_keys=ON;").unwrap();
        let (m, roots, manifest) = prepare(&tx, &f.envelope);
        tx.execute("INSERT INTO s_delivery_refs(generation) VALUES(999)", [])
            .unwrap();
        let error = publish_prepared(tx, &f.token, &m, &roots, &manifest).unwrap_err();
        assert!(error.contains("FOREIGN KEY"), "{error}");
        for table in [
            "s_content_publications",
            "structure_deltas",
            "s_delivery_refs",
            "s_content_nodes",
            "s_content_observations",
        ] {
            assert_eq!(count(&f.conn, table), 0, "{table}");
        }
        assert_eq!(count(&f.conn, "s_clock_events"), 2);
        assert_eq!(
            meta_get_opt(&f.conn, "generation").unwrap().as_deref(),
            Some("0")
        );
        let status: String = f
            .conn
            .query_row("SELECT status FROM s_input_messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "AcceptedPending");
    }

    #[test]
    fn prepared_commit_rejects_wrong_begin_and_rolls_back_prepared_indices() {
        let mut f = fixture();
        let tx = f
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (m, roots, manifest) = prepare(&tx, &f.envelope);
        assert!(publish_prepared(tx, "wrong-begin", &m, &roots, &manifest).is_err());
        assert_eq!(count(&f.conn, "s_content_observations"), 0);
        assert_eq!(count(&f.conn, "s_content_nodes"), 0);
        assert_eq!(count(&f.conn, "s_content_publications"), 0);
    }
}
