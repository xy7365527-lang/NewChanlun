//! #1394：真实 OHLC 生成、独立变化索引与原子发布接线。
//! 接纳和 Begin 须已持久；本模块接管 Commit 事务，任何失败回滚并清空全部运行缓存。
//! 写前那段完整旧状态核验不在此处：它在 `state::audit`（只读入口与提交口共用同一份判据）。
//! 目前是内部接线，尚未接 CLI/服务、资源 job 或 Q/E/B，不代表完整 S3 验收。

use super::*;
use content_manifest::{ManifestMetadata, ManifestRoots, SourceCommit, UnpublishedManifest};
use generation::{GenerationInput, GenerationRuntime};
use newchan_rust::s_content::{self as sc, ContentRef, NodeStore, Value as V};
use newchan_rust::session_protocol::required;
use rusqlite::Transaction;

/// #1394：事务局部业务 codec view。预算来自本次 AuditedState，同账覆盖辅助
/// 业务工作集与跨请求业务缓存；不持 capture，不授予捕获能力或整堆分配资格。
/// 原 TxStore 的 SQL 读写、检查帧和 generic codec 路径全部直接委托，None 保持原行为。
struct BusinessTxStore<'a, 'conn> {
    inner: content_store::TxStore<'a, 'conn>,
    budget: Option<sc::capture_allocation::OwnedBudget>,
}
impl sc::NodeSource for BusinessTxStore<'_, '_> {
    fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        sc::NodeSource::read(&mut self.inner, digest)
    }
    fn checked_read(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        sc::NodeSource::checked_read(&mut self.inner, digest)
    }
}
impl sc::NodeSink for BusinessTxStore<'_, '_> {
    fn put(&mut self, digest: sc::NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        sc::NodeSink::put(&mut self.inner, digest, tag, body)
    }
}
impl audit_store::CodecRead for BusinessTxStore<'_, '_> {
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        self.budget.clone()
    }
    fn generic_source(&mut self) -> &mut dyn sc::NodeSource {
        &mut self.inner
    }
}
impl audit_store::CodecStore for BusinessTxStore<'_, '_> {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        &mut self.inner
    }
}

pub(super) struct Runtime {
    /// 私有 TESTONLY 分配档；只持预算与数值，不持捕获或审计结果。
    allocation: Option<q_allocation::Profile>,
    binding: Option<Vec<String>>,
    generator: Option<GenerationRuntime>,
    auxiliary: aux_index::Runtime,
    #[cfg(test)]
    after_prepare: Option<fn(&Transaction<'_>) -> Result<(), String>>,
}

impl Runtime {
    pub(super) fn new() -> Self {
        Self::with_allocation(None)
    }

    pub(super) fn with_allocation(allocation: Option<q_allocation::Profile>) -> Self {
        Self {
            allocation,
            binding: None,
            generator: None,
            auxiliary: Default::default(),
            #[cfg(test)]
            after_prepare: None,
        }
    }

    fn discard(&mut self) {
        if let Some(generator) = &mut self.generator {
            generator.discard_runtime_state();
        }
        self.auxiliary.discard();
    }

    /// 按值接管事务；包括准备阶段节点、独立索引和目录在内，同成同败。
    pub(super) fn commit(
        &mut self,
        tx: Transaction<'_>,
        token: &str,
        envelope: &Value,
    ) -> Result<commit::CommitReceipt, String> {
        let result = (|| {
            let prepared = self.prepare(&tx, token, envelope)?;
            #[cfg(test)]
            if let Some(inject) = self.after_prepare {
                inject(&tx)?;
            }
            commit::publish_prepared(
                tx,
                token,
                &prepared.metadata,
                &prepared.roots,
                &prepared.manifest,
            )
        })();
        if result.is_err() {
            self.discard();
        }
        result
    }

    fn prepare(
        &mut self,
        tx: &Transaction<'_>,
        token: &str,
        envelope: &Value,
    ) -> Result<Prepared, String> {
        // 完整旧状态审计（读写共用同一份 state::audit）先于节点、索引、目录等第一次写入完成。
        let audited =
            s_allocation::audit(tx, self.allocation.as_ref(), s_allocation::Phase::Prepare)?;
        let clock = v2::stored_clock(tx)?;
        let control = control::S3Control::attach(tx, &clock)?;
        let view = control.load_pending(
            required(envelope, "source_namespace")?,
            required(envelope, "source_epoch")?,
            required(envelope, "message_id")?,
        )?;
        if view.envelope() != envelope {
            return Err(err("Commit原件与已接纳消息不同"));
        }
        if view.is_fresh() || view.is_committed() {
            return Err(err("Commit只接已接纳未提交的消息"));
        }
        let semantic_commit_ns = control
            .commit_time(&view)?
            .ok_or_else(|| err("缺实际Begin提交时刻"))?;
        let get = |key| meta_get_opt(tx, key)?.ok_or_else(|| err(format!("缺冻结字段{key}")));
        let scope = strict_json(get("scope")?.as_bytes())?;
        let scope_value = cv(&scope)?;
        let scope_root = sc::write_value(&mut sc::MemoryNodes::default(), &scope_value)
            .map_err(err)?
            .content_ref();
        let generation = meta_i64(tx, "generation")?
            .checked_add(1)
            .ok_or_else(|| err("代号溢出"))?;
        let frontier = i64::try_from(audited.raw.len())
            .map_err(err)?
            .checked_sub(1)
            .ok_or_else(|| err("原始前沿为空"))?;
        let previous = meta_i64(tx, "last_advance_frontier")?;
        let e = view.envelope();
        let attempts: i64 = tx.query_row(
            "SELECT attempt_count FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",
            params![required(e,"source_namespace")?, required(e,"source_epoch")?, required(e,"message_id")?],
            |row| row.get(0),
        ).map_err(err)?;
        let metadata = ManifestMetadata {
            session_id: get("session_id")?,
            session_generation: required(e, "session_generation")?.into(),
            writer_epoch: get("writer_epoch")?,
            generation,
            base_cut: get("structure_cut")?,
            structure_cut: format!("cut-{generation}"),
            input_frontier: frontier,
            source_commit: SourceCommit {
                source_namespace: required(e, "source_namespace")?.into(),
                source_epoch: required(e, "source_epoch")?.into(),
                message_id: required(e, "message_id")?.into(),
                payload_hash: required(e, "payload_hash")?.into(),
                producer_id: required(e, "producer_id")?.into(),
                producer_epoch: required(e, "producer_epoch")?.into(),
                clock_event_id: required(&e["payload"], "clock_event_id")?.into(),
                attempt_index: attempts
                    .checked_sub(1)
                    .ok_or_else(|| err("缺实际Begin attempt"))?,
            },
            clock_plan_hash: clock.hash,
            semantic_commit_ns,
            profile_id: get("profile_id")?,
            profile_hash: get("profile_hash")?,
            rule_revision: get("rule_revision")?,
            catalog_revision: get("catalog_revision")?,
            scope: scope_root,
        };
        content_publication::verify_current_begin(tx, token, &metadata)?;
        if metadata.profile_id != tb02a::PROFILE {
            return Err(err("非OHLC档生成/来源接线仍未实现"));
        }
        if frontier <= previous {
            return Err(err("本次没有新增原始前沿"));
        }

        let database: String = tx
            .query_row(
                "SELECT file FROM pragma_database_list WHERE name='main'",
                [],
                |r| r.get(0),
            )
            .map_err(err)?;
        let lane_key = s_fact_cache::lane_key(
            Path::new(&database),
            &metadata.session_id,
            &metadata.session_generation,
            &metadata.rule_revision,
            &metadata.profile_id,
            &metadata.catalog_revision,
        );
        let binding = vec![
            database,
            metadata.session_id.clone(),
            metadata.session_generation.clone(),
            metadata.profile_id.clone(),
            metadata.profile_hash.clone(),
            metadata.rule_revision.clone(),
            metadata.catalog_revision.clone(),
        ];
        if let Some(prior) = &self.binding {
            if prior != &binding {
                return Err(err("运行缓存绑定了另一库/会话/冻结域；必须创建新运行实例"));
            }
        } else {
            self.generator = Some(GenerationRuntime::new(
                lane_key,
                &metadata.profile_id,
                &metadata.rule_revision,
            ));
            self.binding = Some(binding);
        }
        let effective = super::business_json::effective(audited.raw.view(), None)?;
        let (prior_bi, prior_version) = object_index::prior_records(&audited.objects)?;
        let input = GenerationInput {
            all_events: audited.raw.view(),
            effective: effective.raw_view(),
            generation,
            structure_cut: &metadata.structure_cut,
            input_frontier: frontier,
            profile_id: &metadata.profile_id,
            profile_hash: &metadata.profile_hash,
            rule_revision: &metadata.rule_revision,
            catalog_revision: &metadata.catalog_revision,
            scope: &scope,
            semantic_commit_ns: Some(&metadata.semantic_commit_ns),
            prior: content_bi::PriorState {
                bi_records: prior_bi.iter().collect(),
                version_record: prior_version.as_ref(),
            },
        };
        let mut store = BusinessTxStore {
            inner: content_store::TxStore::new(tx).map_err(err)?,
            budget: audited.budget.clone(),
        };
        sc::write_value(&mut store, &scope_value).map_err(err)?;
        let mut generated = self
            .generator
            .as_mut()
            .ok_or_else(|| err("生成器未绑定"))?
            .generate(&mut store, &input)?;
        let objects =
            object_index::plan(&mut store, &generated.objects, &audited.objects, generation)?;
        generated.finish_axes(
            &mut store,
            previous,
            &objects.withdrawals,
            &objects.replaces,
        )?;
        let axes = generated
            .catalog_axes
            .clone()
            .ok_or_else(|| err("目录轴完成后仍缺真实根"))?;
        let counts = source_audit::audit_generated(
            &mut store,
            &generated.generated_state,
            &audited.raw,
            &metadata.profile_id,
            &metadata.rule_revision,
        )?;
        let auxiliary = self.auxiliary.plan(
            &mut store,
            &audited.auxiliary,
            &generated,
            &objects.replaces,
            &audited.raw,
            previous,
            frontier,
            generation,
        )?;
        let (run, evidence) = catalog_evidence::build_template(
            &mut store,
            &metadata,
            &generated.generated_state,
            &counts,
            &objects.withdrawals,
            &objects.replaces,
        )?;
        let (snapshot, delta) = templates(
            &mut store, &metadata, previous, &objects, &auxiliary, &run, &evidence,
        )?;
        let roots = ManifestRoots {
            generated_state: generated.generated_state.clone(),
            publication_template: snapshot,
            delta_template: delta,
        };
        let manifest =
            content_manifest::assemble_unpublished(&mut store, &metadata, &roots).map_err(err)?;
        object_index::stage(tx, &mut store, &objects, &manifest.batch_id)?;
        aux_index::stage(tx, &mut store, &auxiliary, &manifest.batch_id)?;
        catalog_evidence::stage_current(tx, &mut store, &axes)?;
        Ok(Prepared {
            metadata,
            roots,
            manifest,
        })
    }
}

struct Prepared {
    metadata: ManifestMetadata,
    roots: ManifestRoots,
    manifest: UnpublishedManifest,
}
fn err(e: impl std::fmt::Display) -> String {
    format!("S3PrepareInvalid：{e}")
}
fn cv(v: &Value) -> Result<V, String> {
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(x) => V::Bool(*x),
        Value::String(x) => V::String(x.clone()),
        Value::Number(x) => V::Number(sc::NumberToken::parse(&x.to_string()).map_err(err)?),
        Value::Array(a) => V::Array(a.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(o) => V::Object(
            o.iter()
                .map(|(k, v)| Ok((k.clone(), cv(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
fn edge(r: &ContentRef) -> Result<V, String> {
    Ok(V::Edge(r.digest().map_err(err)?))
}
fn templates(
    s: &mut dyn NodeStore,
    m: &ManifestMetadata,
    previous: i64,
    o: &object_index::Plan,
    a: &aux_index::Plan,
    run: &str,
    evidence: &ContentRef,
) -> Result<(ContentRef, ContentRef), String> {
    let text = |v: &str| V::String(v.into());
    let range = cv(&json!({"from":(previous+1).to_string(),"to":m.input_frontier.to_string()}))?;
    let common = vec![
        ("session_id".into(), text(&m.session_id)),
        ("generation".into(), text(&m.generation.to_string())),
        ("catalog_revision".into(), text(&m.catalog_revision)),
        ("index_frontier".into(), V::BatchSelf),
        ("input_frontier".into(), text(&m.input_frontier.to_string())),
        ("seq_range".into(), range),
        ("catalog_run_status".into(), text(run)),
        ("catalog_evidence".into(), edge(evidence)?),
    ];
    let mut snapshot = common.clone();
    snapshot.extend([
        ("structure_cut".into(), text(&m.structure_cut)),
        ("scope".into(), edge(&m.scope)?),
        ("profile_id".into(), text(&m.profile_id)),
        ("profile_hash".into(), text(&m.profile_hash)),
        ("history_mode".into(), text("RecomputedWithRevision")),
        ("as_of_generation".into(), V::Null),
        ("objects".into(), V::Edge(o.active.digest)),
        ("withdrawn_objects".into(), V::Edge(o.withdrawn.digest)),
    ]);
    let mut delta = common;
    let withdrawals = o
        .withdrawals
        .iter()
        .map(project_withdrawal_wire)
        .collect::<Result<Vec<_>, _>>()?;
    delta.extend([
        ("base_cut".into(), text(&m.base_cut)),
        ("next_cut".into(), text(&m.structure_cut)),
        ("upserts".into(), V::Edge(o.upserts.digest)),
        ("withdrawals".into(), cv(&json!(withdrawals))?),
        ("replaces".into(), cv(&json!(o.replaces))?),
    ]);
    for (fields, values, raw_key) in [
        (&mut snapshot, &a.snapshot_values, "raw_history"),
        (&mut delta, &a.delta_values, "raw_history_added"),
    ] {
        fields.extend([
            ("witnesses".into(), V::Edge(values.witnesses.digest)),
            ("relations".into(), V::Edge(values.relations.digest)),
            ("observations".into(), V::Edge(values.observations.digest)),
            (raw_key.into(), V::Edge(values.raw_history.digest)),
        ]);
    }
    Ok((
        sc::write_template(s, &V::Object(snapshot))
            .map_err(err)?
            .content_ref(),
        sc::write_template(s, &V::Object(delta))
            .map_err(err)?
            .content_ref(),
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::tests::located_ohlc_input;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// 真实 OHLC 域夹具：init_v3 + 真实接纳/Begin + 真实提交；同域兄弟模块的测试可复用。
    pub(crate) struct Fixture {
        path: PathBuf,
        pub(crate) conn: Connection,
        runtime: Runtime,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    impl Fixture {
        pub(crate) fn new() -> Self {
            Self::with_retain(4)
        }
        pub(crate) fn with_retain(retain: i64) -> Self {
            let path = std::env::temp_dir().join(format!(
                "s1394-real-{}-{}-{}",
                std::process::id(),
                now_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            let mut events = serde_json::Map::new();
            for n in 1..=4 {
                events.insert(format!("input-{n}"),json!({"accept_ns":(n*10).to_string(),"attempts":[{"begin_ns":(n*10+1).to_string(),"commit_ns":(n*10+2).to_string()}]}));
            }
            let clock = json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"real-content-commit","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":events});
            let clock_path = path.join("clock.json");
            std::fs::write(&clock_path, canonical_json(&clock)).unwrap();
            let base = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("s_session");
            let db = path.join("s.sqlite");
            init_v3(
                &db,
                "s-session-testonly-001",
                &base.join("catalog/signed-catalog.json"),
                &base.join("profiles/ohlc_integer_tb02a_v1.json"),
                "1",
                &clock_path,
                retain,
            )
            .unwrap();
            let conn = open_db(&db).unwrap();
            let runtime = Runtime::new();
            Self {
                path,
                conn,
                runtime,
            }
        }
        /// 库文件路径：只读入口测试用来开第二条 `query_only=ON` 连接。
        pub(crate) fn db_path(&self) -> PathBuf {
            self.path.join("s.sqlite")
        }
        pub(crate) fn accept_begin(
            &mut self,
            n: i64,
            row: (&str, i64, i64, i64, i64, i64),
            revision: i64,
        ) -> (Value, String) {
            let mut raw = located_ohlc_input(&[row]);
            raw["events"][0]["revision"] = json!(revision.to_string());
            let payload = json!({"op":"ingest","target_session_id":"s-session-testonly-001","target_session_generation":"1","writer_epoch":"1","clock_event_id":format!("input-{n}"),"raw_input":raw});
            let envelope = json!({"schema_revision":"s-session/3","session_id":"s-session-testonly-001","session_generation":"1","source_namespace":"testonly.ohlc","source_epoch":"1","message_id":format!("message-{n}"),"producer_id":"real-content-test","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
            let clock = v2::stored_clock(&self.conn).unwrap();
            let tx = self.conn.transaction().unwrap();
            let control = control::S3Control::attach(&tx, &clock).unwrap();
            let view = control.inspect_ingest(envelope.clone()).unwrap();
            let accepted =
                raw_input::accept_events(&tx, &tb02a::parse_input(&raw).unwrap(), Some("1"))
                    .unwrap();
            control.accept(&view, &accepted).unwrap();
            tx.commit().unwrap();
            let tx = self.conn.transaction().unwrap();
            let control = control::S3Control::attach(&tx, &clock).unwrap();
            let view = control.inspect_ingest(envelope.clone()).unwrap();
            let generation = meta_i64(&tx, "generation").unwrap() + 1;
            let frontier = read_raw_events(&tx).unwrap().len() as i64 - 1;
            let base = meta_get_opt(&tx, "structure_cut").unwrap().unwrap();
            let token = control
                .begin(&view, generation, frontier, &base, "1")
                .unwrap();
            tx.commit().unwrap();
            (envelope, token)
        }
        pub(crate) fn publish(
            &mut self,
            envelope: &Value,
            token: &str,
        ) -> Result<commit::CommitReceipt, String> {
            let tx = self.conn.transaction().map_err(err)?;
            self.runtime.commit(tx, token, envelope)
        }
    }
    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn catalog(conn: &Connection) -> Vec<(String, String, String, String, String)> {
        conn.prepare("SELECT catalog_id,impl_status,proof_status,run_status,evidence_json FROM catalog ORDER BY catalog_id").unwrap()
            .query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap().collect::<Result<_,_>>().unwrap()
    }

    #[test]
    fn real_nonempty_commits_bind_history_and_revision_to_true_control_and_sources() {
        let mut f = Fixture::new();
        let mut first_snapshot = None;
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (e, token) = f.accept_begin(n, row, revision);
            let receipt = f.publish(&e, &token).unwrap();
            assert_eq!(receipt.message_receipt["kind"], "Committed");
            assert_eq!(receipt.generation, n);
            assert!(count(&f.conn, "s_content_objects") > 0);
            // 完整状态审计只有一份判据（state::audit）；此处核它随每代真实提交后的读数。
            let objects_rows = count(&f.conn, "s_content_objects");
            let witness_rows = count(&f.conn, "s_content_witnesses");
            {
                let tx = f.conn.transaction().unwrap();
                let audited = state::audit(&tx).unwrap();
                assert_eq!(audited.raw.len(), n as usize);
                assert_eq!(audited.cuts.len(), n as usize);
                if let Some(root) = &first_snapshot {
                    assert_eq!(&audited.cuts[0].snapshot, root);
                } else {
                    first_snapshot = Some(audited.cuts[0].snapshot.clone());
                }
                assert!(!audited.objects.is_empty(), "真实提交后对象索引必须非空");
                assert_eq!(
                    audited.objects.len() as i64,
                    objects_rows,
                    "审计返回的对象索引必须与持久行一一对应"
                );
                assert_eq!(
                    audited.auxiliary.witnesses.len() as i64,
                    witness_rows,
                    "审计返回的辅助索引必须与持久行一一对应"
                );
            }
        }
        assert!(count(&f.conn, "s_content_witnesses") > 0);
        let withdrawn: i64 = f
            .conn
            .query_row(
                "SELECT COUNT(*) FROM s_content_objects WHERE withdrawn_generation=4",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(withdrawn > 0, "真实修订必须发生撤回");
        assert_eq!(count(&f.conn, "s_clock_events"), 12);
    }

    #[test]
    fn late_failure_rolls_back_real_nodes_indices_and_catalog_then_same_runtime_retries() {
        let mut f = Fixture::new();
        let (e, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        let before = catalog(&f.conn);
        f.runtime.after_prepare = Some(|tx| {
            tx.execute_batch("CREATE TRIGGER fail_real_commit BEFORE UPDATE OF value ON meta WHEN NEW.key='index_frontier' BEGIN SELECT RAISE(ABORT,'real-commit-fault'); END;").map_err(err)
        });
        let error = f.publish(&e, &token).unwrap_err();
        assert!(error.contains("real-commit-fault"), "{error}");
        assert_eq!(catalog(&f.conn), before);
        for table in [
            "s_content_nodes",
            "s_content_objects",
            "s_content_witnesses",
            "s_content_relations",
            "s_content_observations",
            "s_content_publications",
            "structure_deltas",
        ] {
            assert_eq!(count(&f.conn, table), 0, "{table}");
        }
        assert_eq!(count(&f.conn, "s_clock_events"), 2);
        f.runtime.after_prepare = None;
        assert_eq!(f.publish(&e, &token).unwrap().generation, 1);
    }

    #[test]
    fn damaged_current_catalog_is_rejected_before_generating_or_advancing() {
        let mut f = Fixture::new();
        let (e, t) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        f.publish(&e, &t).unwrap();
        let (e, t) = f.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
        f.conn
            .execute(
                "UPDATE catalog SET run_status='not_run' WHERE catalog_id='CC-005'",
                [],
            )
            .unwrap();
        let before = count(&f.conn, "s_content_nodes");
        assert!(f.publish(&e, &t).is_err());
        assert_eq!(count(&f.conn, "s_content_nodes"), before);
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 1);
        assert_eq!(count(&f.conn, "s_clock_events"), 5);
    }

    #[test]
    fn sqlite_commit_failure_cannot_return_a_receipt_or_keep_real_prepared_state() {
        check_sqlite_commit_failure(None);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_sqlite_commit_failure_cannot_return_receipt_or_keep_prepared_state() {
        let profile = q_allocation::Profile {
            budget: sc::capture_allocation::OwnedBudget::new(50 * 1024 * 1024),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        };
        let budget = profile.budget.clone();
        s_allocation::probe::start(None);
        check_sqlite_commit_failure(Some(profile));
        let (attempts, captures) = s_allocation::probe::finish();
        // 两次真实 prepare：失败 COMMIT 后保持原 Runtime 重试，两次都 fresh capture。
        assert_eq!(attempts, [0, 0, 2]);
        assert_eq!(captures[2].unwrap().capture_return_ordinal, 2);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(!budget.ledger_health().invariant_broken);
    }

    fn check_sqlite_commit_failure(allocation: Option<q_allocation::Profile>) {
        let mut f = Fixture::new();
        f.runtime = Runtime::with_allocation(allocation);
        let (e, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        let before = catalog(&f.conn);
        // 注入发生在写前审计之后；所有语句先成功，SQLite COMMIT才因外键失败。
        f.runtime.after_prepare = Some(|tx| {
            tx.execute_batch("PRAGMA defer_foreign_keys=ON; INSERT INTO s_delivery_refs(generation) VALUES(999);").map_err(err)
        });
        let tx = f.conn.transaction().unwrap();
        let error = f.runtime.commit(tx, &token, &e).unwrap_err();
        assert!(error.contains("FOREIGN KEY"), "{error}");
        assert_eq!(catalog(&f.conn), before);
        assert_eq!(count(&f.conn, "s_content_nodes"), 0);
        assert_eq!(count(&f.conn, "s_content_objects"), 0);
        assert_eq!(count(&f.conn, "s_content_publications"), 0);
        assert_eq!(count(&f.conn, "s_delivery_refs"), 0);
        assert_eq!(count(&f.conn, "s_clock_events"), 2);
        f.runtime.after_prepare = None;
        assert_eq!(f.publish(&e, &token).unwrap().generation, 1);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_second_commit_fault_discards_real_cache_and_reloads_current_ledger() {
        for foreign_ledger in [false, true] {
            let mut f = Fixture::new();
            let mut ordinary = Fixture::new();
            let profile = q_allocation::Profile {
                budget: sc::capture_allocation::OwnedBudget::new(50 * 1024 * 1024),
                overlay_slots: 32768,
                overlay_bytes: 1_000_000,
            };
            let budget = profile.budget.clone();
            f.runtime = Runtime::with_allocation(Some(profile.clone()));
            let (e1, t1) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
            let (plain1, pt1) = ordinary.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
            assert_eq!(
                f.publish(&e1, &t1).unwrap().message_receipt,
                ordinary.publish(&plain1, &pt1).unwrap().message_receipt
            );
            assert!(budget.snapshot().live_requested_layout_bytes > 0);
            let delivery_refs = |conn: &Connection| {
                conn.prepare("SELECT generation FROM s_delivery_refs ORDER BY generation")
                    .unwrap()
                    .query_map([], |r| r.get::<_, i64>(0))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
            };
            let first_delivery_refs = delivery_refs(&f.conn);
            assert_eq!(
                first_delivery_refs,
                vec![1],
                "第一代真实 delivery ref 必须已持久"
            );
            let (e2, t2) = f.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
            let (plain2, pt2) = ordinary.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
            let expected = ordinary.publish(&plain2, &pt2).unwrap().message_receipt;
            let other = sc::capture_allocation::OwnedBudget::new(50 * 1024 * 1024);
            assert!(!budget.same_ledger(&other));
            if foreign_ledger {
                // 只在测试私有域换账，证明活业务缓存不会把另一份预算当同账。
                f.runtime.allocation = Some(q_allocation::Profile {
                    budget: other.clone(),
                    overlay_slots: 32768,
                    overlay_bytes: 1_000_000,
                });
            } else {
                f.runtime.after_prepare = Some(|tx| {
                    tx.execute_batch("PRAGMA defer_foreign_keys=ON; INSERT INTO s_delivery_refs(generation) VALUES(999);").map_err(err)
                });
            }
            s_allocation::probe::start(None);
            let error = f.publish(&e2, &t2).unwrap_err();
            let (attempts, captures) = s_allocation::probe::finish();
            assert_eq!(attempts, [0, 0, 1]);
            assert_eq!(captures[2].unwrap().raw, 2);
            assert_eq!(captures[2].unwrap().clocks, 5);
            if foreign_ledger {
                assert!(
                    error
                        .contains("prototype_audit_allocation_incomplete：缓存账本改变，须discard"),
                    "{error}"
                );
            } else {
                assert!(error.contains("FOREIGN KEY"), "{error}");
            }
            assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 1);
            assert_eq!(count(&f.conn, "s_content_publications"), 1);
            assert_eq!(count(&f.conn, "structure_deltas"), 1);
            let retained_delivery_refs = delivery_refs(&f.conn);
            assert_eq!(
                retained_delivery_refs, first_delivery_refs,
                "失败必须逐行保留第一代 refs"
            );
            assert!(
                !retained_delivery_refs.contains(&2),
                "未提交的第二代 ref 不得混入"
            );
            assert!(
                !retained_delivery_refs.contains(&999),
                "FK 故障 ref 不得混入"
            );
            assert_eq!(count(&f.conn, "s_clock_events"), 5);
            assert_ne!(
                meta_get_opt(&f.conn, "advance_state").unwrap().as_deref(),
                Some("idle")
            );
            // commit 的真实失败路径调用 discard；不靠测试主动清缓存求归零。
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert_eq!(other.snapshot().live_requested_layout_bytes, 0);
            f.runtime.after_prepare = None;
            f.runtime.allocation = Some(profile);
            s_allocation::probe::start(None);
            let retried = f.publish(&e2, &t2).unwrap().message_receipt;
            let (attempts, captures) = s_allocation::probe::finish();
            assert_eq!(retried, expected);
            assert_eq!(attempts, [0, 0, 1]);
            assert_eq!(captures[2].unwrap().live_before, 0);
            assert_eq!(count(&f.conn, "s_clock_events"), 6);
            assert_eq!(count(&f.conn, "s_content_publications"), 2);
            assert_eq!(count(&f.conn, "structure_deltas"), 2);
            assert!(budget.snapshot().live_requested_layout_bytes > 0);
            assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
            drop(f);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert_eq!(
                budget.snapshot().allocated_layouts,
                budget.snapshot().released_layouts
            );
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[test]
    fn initial_catalog_corruption_and_wrong_begin_are_rejected_without_content_writes() {
        let mut f = Fixture::new();
        let (e, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        assert!(f.publish(&e, "wrong-begin").is_err());
        assert_eq!(count(&f.conn, "s_content_nodes"), 0);
        f.conn
            .execute(
                "UPDATE catalog SET title='changed' WHERE catalog_id='CC-005'",
                [],
            )
            .unwrap();
        let error = f.publish(&e, &token).unwrap_err();
        assert!(error.contains("目录静态内容与来源不同"), "{error}");
        assert_eq!(count(&f.conn, "s_content_nodes"), 0);
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 0);
        assert_eq!(count(&f.conn, "s_clock_events"), 2);
    }
}
