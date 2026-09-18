//! #1394：真实 OHLC 生成、独立变化索引与原子发布接线。
//! 接纳和 Begin 须已持久；本模块接管 Commit 事务，任何失败回滚并清空全部运行缓存。
//! 目前是内部接线，尚未接 CLI/服务、资源 job 或 Q/E/B，不代表完整 S3 验收。

use super::*;
use content_manifest::{ManifestMetadata, ManifestRoots, SourceCommit, UnpublishedManifest};
use generation::{GenerationInput, GenerationRuntime};
use newchan_rust::s_content::{self as sc, ContentRef, NodeStore, Value as V};
use newchan_rust::session_protocol::required;
use rusqlite::Transaction;

pub(super) struct Runtime {
    binding: Option<Vec<String>>,
    generator: Option<GenerationRuntime>,
    auxiliary: aux_index::Runtime,
    #[cfg(test)]
    after_prepare: Option<fn(&Transaction<'_>) -> Result<(), String>>,
}

impl Runtime {
    pub(super) fn new() -> Self {
        Self {
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
        // 所有审计均发生在节点、索引、目录等第一次写入之前。
        let control_history = control_audit::prepare_s3_control_history(tx)?;
        let raw = read_raw_events(tx)?;
        let cuts = history::read_history(tx, &raw, control_history)?;
        verify_unrun_catalog(tx, cuts.is_empty())?;
        let clock = v2::stored_clock(tx)?;
        let control = control::S3Control::attach(tx, &clock)?;
        let view = control.inspect_ingest(envelope.clone())?;
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
        let frontier = i64::try_from(raw.len())
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

        let (old_objects, old_aux) = {
            let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
            let mut store = history::HashReader::new(&mut reader);
            let mut cut_previous = -1;
            let indexed = object_index::load(tx, &mut store, generation - 1)?;
            for cut in &cuts {
                let prefix = &raw[..=cut.metadata.input_frontier as usize];
                let counts = source_audit::audit_generated(
                    &mut store,
                    &cut.roots.generated_state,
                    prefix,
                    &cut.metadata.profile_id,
                    &cut.metadata.rule_revision,
                )?;
                let (withdrawals, replaces) =
                    object_index::changes_at(&indexed, cut.metadata.generation)?;
                catalog_evidence::verify_cut(
                    &mut store,
                    &cut.metadata,
                    &cut.roots.generated_state,
                    &cut.delta,
                    prefix,
                    cut_previous,
                    &counts,
                    &withdrawals,
                    &replaces,
                )?;
                cut_previous = cut.metadata.input_frontier;
            }
            if let Some(last) = cuts.last() {
                let axes = field(&mut store, &last.roots.generated_state, "catalog_axes")?;
                catalog_evidence::verify_current(tx, &mut store, &axes)?;
            }
            let aux = aux_index::load(tx, &mut store, generation - 1)?;
            (indexed, aux)
        };
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
        let effective = read_effective_events(&raw)?;
        let (prior_bi, prior_version) = object_index::prior_records(&old_objects)?;
        let input = GenerationInput {
            all_events: &raw,
            effective: &effective,
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
        let mut store = content_store::TxStore::new(tx).map_err(err)?;
        sc::write_value(&mut store, &scope_value).map_err(err)?;
        let mut generated = self
            .generator
            .as_mut()
            .ok_or_else(|| err("生成器未绑定"))?
            .generate(&mut store, &input)?;
        let objects = object_index::plan(&mut store, &generated.objects, &old_objects, generation)?;
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
            &raw,
            &metadata.profile_id,
            &metadata.rule_revision,
        )?;
        let auxiliary = self.auxiliary.plan(
            &mut store,
            &old_aux,
            &generated,
            &objects.replaces,
            &raw,
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
/// 静态列/冻结原件由共享控制审计核验；本层补核从未运行的动态列。
fn verify_unrun_catalog(tx: &Transaction<'_>, initial: bool) -> Result<(), String> {
    let mut stmt = tx
        .prepare("SELECT catalog_id,impl_status,proof_status,run_status,evidence_json FROM catalog")
        .map_err(err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(err)?;
    for row in rows {
        let (id, implemented, proof, run, evidence) = row.map_err(err)?;
        if (initial || !catalog_evidence::AXIS_IDS.contains(&id.as_str()))
            && (implemented != "not_implemented"
                || proof != "not_proved"
                || run != "not_run"
                || evidence != "{}")
        {
            return Err(err(format!("未运行目录{id}带无来源运行状态")));
        }
    }
    Ok(())
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
fn field(s: &mut dyn NodeStore, r: &ContentRef, key: &str) -> Result<ContentRef, String> {
    sc::path::lookup_path(s, r, &[sc::path::PathStep::Field(key.into())]).map_err(err)
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
mod tests {
    use super::*;
    use crate::tests::located_ohlc_input;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        path: PathBuf,
        conn: Connection,
        runtime: Runtime,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    impl Fixture {
        fn new() -> Self {
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
                4,
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
        fn accept_begin(
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
        fn publish(
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
            let tx = f.conn.transaction().unwrap();
            let control = control_audit::prepare_s3_control_history(&tx).unwrap();
            let raw = read_raw_events(&tx).unwrap();
            let cuts = history::read_history(&tx, &raw, control).unwrap();
            assert_eq!(cuts.len(), n as usize);
            if let Some(root) = &first_snapshot {
                assert_eq!(&cuts[0].snapshot, root);
            } else {
                first_snapshot = Some(cuts[0].snapshot.clone());
            }
            let mut reader = content_store::ReadStore::new(&tx).unwrap();
            let mut store = history::HashReader::new(&mut reader);
            let objects = object_index::load(&tx, &mut store, n).unwrap();
            let mut previous = -1;
            for cut in &cuts {
                let prefix = &raw[..=cut.metadata.input_frontier as usize];
                let counts = source_audit::audit_generated(
                    &mut store,
                    &cut.roots.generated_state,
                    prefix,
                    &cut.metadata.profile_id,
                    &cut.metadata.rule_revision,
                )
                .unwrap();
                let (withdrawals, replaces) =
                    object_index::changes_at(&objects, cut.metadata.generation).unwrap();
                catalog_evidence::verify_cut(
                    &mut store,
                    &cut.metadata,
                    &cut.roots.generated_state,
                    &cut.delta,
                    prefix,
                    previous,
                    &counts,
                    &withdrawals,
                    &replaces,
                )
                .unwrap();
                previous = cut.metadata.input_frontier;
            }
            let axes = field(
                &mut store,
                &cuts.last().unwrap().roots.generated_state,
                "catalog_axes",
            )
            .unwrap();
            catalog_evidence::verify_current(&tx, &mut store, &axes).unwrap();
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
        let mut f = Fixture::new();
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
