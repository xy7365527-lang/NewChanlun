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
use sc::capture_allocation::scratch::WorkScope;

/// 私有构造字段：只有本模块成功获得 NativeReadLease 的分支可发此代生成资格。
/// 不授予节点存在、checked frame、旧账或发布闭包证明。
pub(super) struct NativeGeneration {
    _lease_route: (),
}

/// #1394：事务局部业务 codec view。预算来自本次 AuditedState，同账覆盖辅助
/// 业务工作集与跨请求业务缓存；不持 capture，不授予捕获能力或整堆分配资格。
/// 原 TxStore 的 SQL 读写、检查帧和 generic codec 路径全部直接委托，None 保持原行为。
struct BusinessTxStore<T: NodeStore> {
    inner: T,
    budget: Option<sc::capture_allocation::OwnedBudget>,
}
impl<T: NodeStore> sc::NodeSource for BusinessTxStore<T> {
    fn checked_read_scoped(
        &mut self,
        d: &sc::NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        self.inner.checked_read_scoped(d)
    }
    fn codec_work(&self) -> Option<&WorkScope> {
        self.inner.codec_work()
    }
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
impl<T: NodeStore> sc::NodeSink for BusinessTxStore<T> {
    fn codec_sink_work(&self) -> Option<&WorkScope> {
        self.inner.codec_sink_work()
    }
    fn put(&mut self, digest: sc::NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        sc::NodeSink::put(&mut self.inner, digest, tag, body)
    }
}
impl<T: NodeStore> audit_store::CodecRead for BusinessTxStore<T> {
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        self.budget.clone()
    }
    fn generic_source(&mut self) -> &mut dyn sc::NodeSource {
        &mut self.inner
    }
}
impl<T: NodeStore> audit_store::CodecStore for BusinessTxStore<T> {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        &mut self.inner
    }
}

pub(super) struct Runtime {
    /// 与 Driver 同一启动模式；不持 capture/审计结果，Owned 业务缓存按真实生命周期计费。
    mode: s_allocation::Mode,
    binding: Option<Vec<String>>,
    generator: Option<GenerationRuntime>,
    auxiliary: aux_index::Runtime,
    #[cfg(test)]
    after_prepare: Option<fn(&Transaction<'_>) -> Result<(), String>>,
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    stage_new_nodes: bool,
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    native_read_lease: bool,
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    manifest_workspace: bool,
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    last_stage_report: Option<Value>,
}

impl Runtime {
    pub(super) fn new() -> Self {
        Self::with_allocation(None)
    }

    pub(super) fn with_allocation(allocation: Option<q_allocation::Profile>) -> Self {
        Self::with_mode(allocation.into())
    }

    pub(super) fn with_mode(mode: s_allocation::Mode) -> Self {
        Self {
            mode,
            binding: None,
            generator: None,
            auxiliary: Default::default(),
            #[cfg(test)]
            after_prepare: None,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            stage_new_nodes: false,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            native_read_lease: true,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            manifest_workspace: true,
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            last_stage_report: None,
        }
    }

    #[cfg(test)]
    pub(super) fn set_after_prepare_for_request_test(
        &mut self,
        hook: Option<fn(&Transaction<'_>) -> Result<(), String>>,
    ) {
        self.after_prepare = hook;
    }

    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn disable_manifest_workspace_for_test(&mut self) {
        self.manifest_workspace = false;
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn visit_runtime_roots(
        &self,
    ) -> Result<Vec<(&'static str, sc::NodeDigest)>, String> {
        let mut roots = Vec::new();
        let mut f = |name, root| roots.push((name, root));
        if let Some(generator) = &self.generator {
            generator.visit_runtime_roots(&mut f);
        }
        self.auxiliary.visit_runtime_roots(&mut f)?;
        Ok(roots)
    }

    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn enable_staging_for_native_test(&mut self) -> Result<(), String> {
        if !matches!(self.mode, s_allocation::Mode::CapturedNative) {
            return Err("TestOnly staging只允许Native".into());
        }
        self.stage_new_nodes = true;
        Ok(())
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn staging_report_for_native_test(&self) -> Option<&Value> {
        self.last_stage_report.as_ref()
    }

    pub(super) fn discard(&mut self) {
        if let Some(generator) = &mut self.generator {
            generator.discard_runtime_state();
        }
        self.auxiliary.discard();
    }

    /// 按值接管事务；包括准备阶段节点、独立索引和目录在内，同成同败。
    #[cfg(test)]
    pub(super) fn commit(
        &mut self,
        mut tx: Transaction<'_>,
        token: &str,
        envelope: &Value,
    ) -> Result<commit::CommitReceipt, String> {
        let result = (|| {
            let receipt = self.publish_impl(
                &mut tx,
                token,
                envelope,
                None,
                #[cfg(feature = "s_capture_native_testonly")]
                None,
            )?;
            tx.commit()
                .map_err(|e| format!("PublicationInvalid：{e}"))?;
            Ok(receipt)
        })();
        if result.is_err() {
            self.discard();
        }
        result
    }
    #[cfg(test)]
    pub(super) fn commit_in_request(
        &mut self,
        mut tx: Transaction<'_>,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<commit::CommitReceipt, String> {
        let result = (|| {
            let receipt = self.publish_impl(
                &mut tx,
                token,
                envelope,
                Some(context),
                #[cfg(feature = "s_capture_native_testonly")]
                None,
            )?;
            tx.commit()
                .map_err(|e| format!("PublicationInvalid：{e}"))?;
            Ok(receipt)
        })();
        if result.is_err() {
            self.discard();
        }
        result
    }
    pub(super) fn publish_in_owner(
        &mut self,
        phase: super::audit_owner::PublishPhase<'_, '_>,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<commit::CommitReceipt, String> {
        phase.publish(self, token, envelope, context)
    }
    pub(super) fn publish_impl(
        &mut self,
        tx: &mut Transaction<'_>,
        token: &str,
        envelope: &Value,
        context: Option<&mut request_audit::RequestAuditContext>,
        #[cfg(feature = "s_capture_native_testonly")] bound_audit: Option<&state::AuditedState>,
    ) -> Result<commit::CommitReceipt, String> {
        self.publish_impl_admitted(
            tx,
            token,
            envelope,
            context,
            #[cfg(feature = "s_capture_native_testonly")]
            bound_audit,
            None,
        )
    }
    /// 调用者在 prepare 之前开启唯一父域，并保持到 Commit 返回；此入口本身不签发整堆资格。
    pub(super) fn publish_impl_admitted(
        &mut self,
        tx: &mut Transaction<'_>,
        token: &str,
        envelope: &Value,
        context: Option<&mut request_audit::RequestAuditContext>,
        #[cfg(feature = "s_capture_native_testonly")] bound_audit: Option<&state::AuditedState>,
        work: Option<&WorkScope>,
    ) -> Result<commit::CommitReceipt, String> {
        if work.is_some() {
            #[cfg(feature = "s_capture_native_testonly")]
            if !matches!(self.mode, s_allocation::Mode::CapturedNative) {
                return Err("PublicationInvalid：Native工作域不能覆盖Owned或普通模式".into());
            }
            #[cfg(not(feature = "s_capture_native_testonly"))]
            return Err("PublicationInvalid：Native工作域需要相应feature".into());
        }
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if work.is_some() && self.stage_new_nodes {
            return Err("PublicationInvalid：staged发布工作域尚未接入，不能丢弃父scope".into());
        }
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        let stage = if self.stage_new_nodes {
            if !matches!(self.mode, s_allocation::Mode::CapturedNative) {
                return Err("TestOnly staging只允许Native".into());
            }
            Some(staged_nodes::StageOwner::new(&tx))
        } else {
            None
        };
        let result = (|| {
            let prepared = self.prepare(
                tx,
                token,
                envelope,
                context,
                #[cfg(feature = "s_capture_native_testonly")]
                bound_audit,
                #[cfg(all(test, feature = "s_capture_native_testonly"))]
                stage.as_ref(),
                work,
            )?;
            #[cfg(test)]
            if let Some(inject) = self.after_prepare {
                inject(&tx)?;
            }
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            if let Some(stage) = &stage {
                let roots = self
                    .visit_runtime_roots()?
                    .into_iter()
                    .map(|(_, d)| d)
                    .collect::<Vec<_>>();
                return commit::publish_borrowed_staged(
                    tx,
                    token,
                    &prepared.metadata,
                    &prepared.roots,
                    &prepared.manifest,
                    stage,
                    &roots,
                );
            }
            #[cfg(feature = "s_capture_native_testonly")]
            return commit::publish_borrowed_with_workspace_admitted(
                tx,
                token,
                &prepared.metadata,
                &prepared.roots,
                &prepared.manifest,
                prepared.workspace,
                work,
            );
            #[cfg(not(feature = "s_capture_native_testonly"))]
            commit::publish_borrowed(
                tx,
                token,
                &prepared.metadata,
                &prepared.roots,
                &prepared.manifest,
            )
        })();
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        {
            self.last_stage_report = stage.as_ref().map(|owner| owner.report());
        }
        if result.is_err() {
            self.discard();
        }
        result
    }

    fn prepare(
        &mut self,
        tx: &mut Transaction<'_>,
        token: &str,
        envelope: &Value,
        context: Option<&mut request_audit::RequestAuditContext>,
        #[cfg(feature = "s_capture_native_testonly")] bound_audit: Option<&state::AuditedState>,
        #[cfg(all(test, feature = "s_capture_native_testonly"))] stage: Option<
            &staged_nodes::StageOwner,
        >,
        work: Option<&WorkScope>,
    ) -> Result<Prepared, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        let workspace_budget = context
            .as_deref()
            .and_then(request_audit::RequestAuditContext::manifest_workspace_budget);
        // 完整旧状态审计（读写共用同一份 state::audit）先于节点、索引、目录等第一次写入完成。
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let bound_audit: Option<&state::AuditedState> = None;
        let fresh;
        let audited = if let Some(bound) = bound_audit {
            bound
        } else {
            fresh = match context {
                Some(context) => s_allocation::audit_in_request(
                    tx,
                    &self.mode,
                    s_allocation::Phase::Prepare,
                    context,
                )?,
                None => s_allocation::audit(tx, &self.mode, s_allocation::Phase::Prepare)?,
            };
            &fresh
        };
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
        content_publication::verify_current_begin_admitted(tx, token, metadata.view(), work)?;
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
        #[cfg(feature = "s_capture_native_testonly")]
        let use_lease = matches!(self.mode, s_allocation::Mode::CapturedNative) && {
            #[cfg(test)]
            {
                stage.is_none() && self.native_read_lease
            }
            #[cfg(not(test))]
            {
                true
            }
        };
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let use_lease = false;
        #[cfg(feature = "s_capture_native_testonly")]
        if use_lease {
            if audited.budget.is_some() {
                return Err(err("Native读取租约不允许Owned业务预算"));
            }
            // End the only safe same-connection alias before the exclusive read phase.
            drop(control);
            let products = {
                let mut store = BusinessTxStore {
                    inner: content_store::NativeReadLease::new_admitted(tx, work).map_err(err)?,
                    budget: None,
                };
                self.generate_node_products(
                    &mut store,
                    Some(&NativeGeneration { _lease_route: () }),
                    workspace_budget,
                    &input,
                    &audited,
                    &metadata,
                    &scope_value,
                    previous,
                    generation,
                    frontier,
                )?
            }; // The lease ends before raw SQL; the frozen workspace is inert until row rebinding.
            let NodeProducts {
                generated: _generated,
                objects,
                axes,
                counts: _counts,
                auxiliary,
                run: _run,
                evidence: _evidence,
                roots,
                manifest,
                workspace,
            } = products;
            let mut store = BusinessTxStore {
                inner: content_store::TxStore::new(tx)
                    .map_err(err)?
                    .with_work(work),
                budget: None,
            };
            // C2仅在generate_node_products的旧完整来源判据成功后记录材料cut。
            super::audit_owner::note_qualified_source_cut(
                tx,
                &mut store,
                (&roots.generated_state).into(),
            )?;
            object_index::stage(tx, &mut store, &objects, &manifest.batch_id)?;
            aux_index::stage(tx, &mut store, &auxiliary, &manifest.batch_id)?;
            catalog_evidence::stage_current(tx, &mut store, &axes)?;
            return Ok(Prepared {
                metadata,
                roots,
                manifest,
                workspace,
            });
        }
        let _ = use_lease;
        let mut store = BusinessTxStore {
            #[cfg(all(test, feature = "s_capture_native_testonly"))]
            inner: content_store::TxStore::new_with_stage(tx, stage)
                .map_err(err)?
                .with_work(work),
            #[cfg(not(all(test, feature = "s_capture_native_testonly")))]
            inner: content_store::TxStore::new(tx)
                .map_err(err)?
                .with_work(work),
            budget: audited.budget.clone(),
        };
        let NodeProducts {
            generated: _generated,
            objects,
            axes,
            counts: _counts,
            auxiliary,
            run: _run,
            evidence: _evidence,
            roots,
            manifest,
            #[cfg(feature = "s_capture_native_testonly")]
            workspace,
        } = self.generate_node_products(
            &mut store,
            None,
            #[cfg(feature = "s_capture_native_testonly")]
            workspace_budget,
            &input,
            &audited,
            &metadata,
            &scope_value,
            previous,
            generation,
            frontier,
        )?;
        #[cfg(feature = "s_capture_native_testonly")]
        super::audit_owner::note_qualified_source_cut(
            tx,
            &mut store,
            (&roots.generated_state).into(),
        )?;
        object_index::stage(tx, &mut store, &objects, &manifest.batch_id)?;
        aux_index::stage(tx, &mut store, &auxiliary, &manifest.batch_id)?;
        catalog_evidence::stage_current(tx, &mut store, &axes)?;
        Ok(Prepared {
            metadata,
            roots,
            manifest,
            #[cfg(feature = "s_capture_native_testonly")]
            workspace,
        })
    }
    fn generate_node_products<'raw, S: NodeStore>(
        &mut self,
        store: &mut BusinessTxStore<S>,
        native: Option<&NativeGeneration>,
        #[cfg(feature = "s_capture_native_testonly")] workspace_budget: Option<
            request_audit::ManifestWorkspaceBudget,
        >,
        input: &GenerationInput<'raw>,
        audited: &state::AuditedState,
        metadata: &ManifestMetadata,
        scope_value: &V,
        previous: i64,
        generation: i64,
        frontier: i64,
    ) -> Result<NodeProducts<'raw>, String> {
        sc::write_value(store, scope_value).map_err(err)?;
        let generator = self.generator.as_mut().ok_or_else(|| err("生成器未绑定"))?;
        let mut generated = match native {
            Some(native) => generator.generate_native_lease(store, input, native)?,
            None => generator.generate_with_allocation(store, input, audited.budget.as_ref())?,
        };
        #[cfg(feature = "s_capture_native_testonly")]
        let objects = if matches!(self.mode, s_allocation::Mode::CapturedNative) {
            object_index::plan_native_union_for_testonly(
                store,
                &generated.objects,
                &audited.objects,
                generation,
            )?
        } else {
            object_index::plan(store, &generated.objects, &audited.objects, generation)?
        };
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let objects = object_index::plan(store, &generated.objects, &audited.objects, generation)?;
        generated.finish_axes(store, previous, &objects.withdrawals, &objects.replaces)?;
        let axes = generated
            .catalog_axes
            .clone()
            .ok_or_else(|| err("目录轴完成后仍缺真实根"))?;
        let counts = source_audit::audit_generated(
            store,
            &generated.generated_state,
            &audited.raw,
            &metadata.profile_id,
            &metadata.rule_revision,
        )?;
        let auxiliary = self.auxiliary.plan(
            store,
            &audited.auxiliary,
            &generated,
            &objects.replaces,
            &audited.raw,
            previous,
            frontier,
            generation,
        )?;
        let (run, evidence) = catalog_evidence::build_template(
            store,
            metadata,
            &generated.generated_state,
            &counts,
            &objects.withdrawals,
            &objects.replaces,
        )?;
        let (snapshot, delta) = templates(
            store, metadata, previous, &objects, &auxiliary, &run, &evidence,
        )?;
        let roots = ManifestRoots {
            generated_state: generated.generated_state.clone(),
            publication_template: snapshot,
            delta_template: delta,
        };
        #[cfg(feature = "s_capture_native_testonly")]
        let (manifest, workspace) = if native.is_some() && workspace_budget.is_some() && {
            #[cfg(test)]
            {
                self.manifest_workspace
            }
            #[cfg(not(test))]
            {
                true
            }
        } {
            content_manifest::assemble_unpublished_with_workspace(
                store,
                metadata,
                &roots,
                workspace_budget.expect("qualified entry budget"),
            )
            .map_err(err)?
        } else {
            (
                content_manifest::assemble_unpublished(store, metadata, &roots).map_err(err)?,
                None,
            )
        };
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let manifest =
            content_manifest::assemble_unpublished(store, metadata, &roots).map_err(err)?;
        Ok(NodeProducts {
            generated,
            objects,
            axes,
            counts,
            auxiliary,
            run,
            evidence,
            roots,
            manifest,
            #[cfg(feature = "s_capture_native_testonly")]
            workspace,
        })
    }
}

// Raw lifetime is audited input, never the lease. These owners existed through legacy stage
// already; retaining them avoids changing Owned peak/drop timing while separating Native SQL.
struct NodeProducts<'raw> {
    generated: generation::Generated<'raw>,
    objects: object_index::Plan,
    axes: sc::ContentRef,
    counts: source_audit::Counts,
    auxiliary: aux_index::Plan,
    run: String,
    evidence: sc::ContentRef,
    roots: ManifestRoots,
    manifest: UnpublishedManifest,
    #[cfg(feature = "s_capture_native_testonly")]
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
}

struct Prepared {
    metadata: ManifestMetadata,
    roots: ManifestRoots,
    manifest: UnpublishedManifest,
    #[cfg(feature = "s_capture_native_testonly")]
    workspace: Option<content_manifest::PreparedManifestWorkspace>,
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

    mod pending_source_tests {
        include!("pending_source_tests.rs");
    }

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

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    #[ignore]
    fn staged_new_nodes_complete_commit_observation() {
        let out = PathBuf::from(std::env::var("STAGED_NODES_OUTPUT").unwrap());
        assert!(!out.exists());
        std::fs::create_dir_all(&out).unwrap();
        let mut staged = Fixture::new();
        staged.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        staged.runtime.stage_new_nodes = true;
        let mut eager = Fixture::new();
        eager.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut stages = Vec::new();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (envelope, token) = staged.accept_begin(n, row, revision);
            let (plain, plain_token) = eager.accept_begin(n, row, revision);
            let old = staged_node_material(&staged.conn);
            if n == 4 {
                for fault in ["after_prepare", "flush_INSERT"] {
                    let before = runtime_root_tables(&staged.conn, true);
                    staged.runtime.after_prepare = Some(if fault == "after_prepare" {
                        |_| Err("TestOnlyStageAfterPrepare".into())
                    } else {
                        |tx| {
                            tx.execute_batch(&format!("CREATE TEMP TRIGGER stage_flush_fault BEFORE INSERT ON main.s_content_nodes WHEN (SELECT count(*) FROM s_content_nodes)>{} BEGIN SELECT RAISE(ABORT,'TestOnlyStageFlushINSERT'); END;",tx.query_row("SELECT count(*)+1 FROM s_content_nodes",[],|r|r.get::<_,i64>(0)).map_err(err)?)).map_err(err)?;
                            Ok(())
                        }
                    });
                    let failure = staged.publish(&envelope, &token).unwrap_err();
                    assert!(
                        failure.contains(if fault == "after_prepare" {
                            "TestOnlyStageAfterPrepare"
                        } else {
                            "TestOnlyStageFlushINSERT"
                        }),
                        "{failure}"
                    );
                    if fault == "flush_INSERT" {
                        assert_eq!(
                            staged.runtime.last_stage_report.as_ref().unwrap()
                                ["new_physical_inserts"],
                            json!(2)
                        );
                    }
                    // 两臂重试从相同正常discard状态出发；eager故障时点明确不冒充flush等价。
                    eager.runtime.after_prepare =
                        Some(|_| Err("TestOnlyEagerAlignedRollback".into()));
                    let eager_failure = eager.publish(&plain, &plain_token).unwrap_err();
                    assert!(
                        eager_failure.contains("TestOnlyEagerAlignedRollback"),
                        "{eager_failure}"
                    );
                    eager.runtime.after_prepare = None;
                    assert_eq!(before, runtime_root_tables(&staged.conn, true));
                    assert!(staged.runtime.visit_runtime_roots().unwrap().is_empty());
                    stages.push(json!({"fault":fault,"returned_success_receipt":false,"all_persistent_rows_and_nodes_rolled_back_exact":true,"normal_Runtime_discard_empty_roots":true,"staging":staged.runtime.last_stage_report}));
                    staged.runtime.after_prepare = None;
                }
            }
            content_store::begin_runtime_reads();
            let started = std::time::Instant::now();
            let actual = staged.publish(&envelope, &token).unwrap();
            let elapsed = started.elapsed().as_nanos();
            let reads = content_store::end_runtime_reads();
            let expected = eager.publish(&plain, &plain_token).unwrap();
            assert_eq!(actual.batch_id, expected.batch_id);
            assert_eq!(actual.snapshot, expected.snapshot);
            assert_eq!(actual.delta, expected.delta);
            assert_eq!(
                runtime_root_tables(&staged.conn, false),
                runtime_root_tables(&eager.conn, false)
            );
            runtime_root_compare_reachable_bytes(&staged.conn, &eager.conn);
            let current = staged_node_material(&staged.conn);
            let eager_nodes = staged_node_material(&eager.conn);
            for (digest, original) in &old {
                assert_eq!(current.get(digest), Some(original));
            }
            for (digest, original) in &current {
                assert_eq!(eager_nodes.get(digest), Some(original));
            }
            let label = format!("staged-{n}");
            let mut snap = runtime_roots_snapshot(&staged, &out, &label, &reads, elapsed);
            snap["staging"] = staged.runtime.last_stage_report.clone().unwrap();
            snap["old_nodes_all_preserved"] = json!(true);
            snap["complete_non_node_rows_and_declared_root_bytes_equal"] = json!(true);
            snap["current_node_rows"] = json!(current.len());
            snap["eager_node_rows"] = json!(eager_nodes.len());
            stages.push(snap);
            let _ = runtime_roots_snapshot(&eager, &out, &format!("eager-{n}"), &[], 0);
        }
        std::fs::write(out.join("STAGED-OBSERVATION.json"),serde_json::to_vec_pretty(&json!({"stages":stages,"Native_testonly":true,"shared_owner_all_three_stores":true,"old_material_audit_unchanged":true,"read_modes":"both Runtime Mode::CapturedNative, eager default stage=false vs explicit staged=true; no silent fallback","physical_INSERT_failure_timing_equivalent":false,"105_or_9011_qualification":false,"Job_max_store_or_strict_heap_qualification":false})).unwrap()).unwrap();
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn staged_node_material(c: &Connection) -> std::collections::BTreeMap<Vec<u8>, (u8, Vec<u8>)> {
        c.prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
            .unwrap()
            .query_map([], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    (r.get::<_, u8>(1)?, r.get::<_, Vec<u8>>(2)?),
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    #[ignore]
    fn runtime_roots_append_revision_error_observation() {
        let out = PathBuf::from(std::env::var("RUNTIME_ROOTS_OUTPUT").unwrap());
        assert!(!out.exists());
        std::fs::create_dir_all(&out).unwrap();
        let mut native = Fixture::new();
        native.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut ordinary = Fixture::new();
        let mut reports = Vec::new();
        let rows = [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ];
        for (n, row, revision) in rows {
            let (envelope, token) = native.accept_begin(n, row, revision);
            let (plain, plain_token) = ordinary.accept_begin(n, row, revision);
            if n == 4 {
                let persisted_before = runtime_root_tables(&native.conn, true);
                native.runtime.after_prepare =
                    Some(|_| Err("TestOnlyRuntimeRootFailureAfterPrepare".into()));
                content_store::begin_runtime_reads();
                let started = std::time::Instant::now();
                assert!(native
                    .publish(&envelope, &token)
                    .unwrap_err()
                    .contains("TestOnlyRuntimeRootFailureAfterPrepare"));
                let reads = content_store::end_runtime_reads();
                assert_eq!(persisted_before, runtime_root_tables(&native.conn, true));
                reports.push(runtime_roots_snapshot(
                    &native,
                    &out,
                    "error-cleanup",
                    &reads,
                    started.elapsed().as_nanos(),
                ));
                native.runtime.after_prepare = None;
                assert!(native.runtime.visit_runtime_roots().unwrap().is_empty());
            }
            content_store::begin_runtime_reads();
            let started = std::time::Instant::now();
            let receipt = native.publish(&envelope, &token).unwrap();
            let reads = content_store::end_runtime_reads();
            let expected = ordinary.publish(&plain, &plain_token).unwrap();
            assert_eq!(receipt.batch_id, expected.batch_id);
            assert_eq!(receipt.snapshot, expected.snapshot);
            assert_eq!(receipt.delta, expected.delta);
            assert_eq!(
                runtime_root_tables(&native.conn, false),
                runtime_root_tables(&ordinary.conn, false)
            );
            runtime_root_compare_reachable_bytes(&native.conn, &ordinary.conn);
            let stage = format!("committed-{n}");
            let mut report =
                runtime_roots_snapshot(&native, &out, &stage, &reads, started.elapsed().as_nanos());
            report["ordinary_full_persistent_typed_rows_and_reachable_content_bytes_exact_equal"] =
                json!(true);
            report["normal_append"] = json!(n < 4);
            report["true_revision"] = json!(n == 4);
            reports.push(report);
        }
        let withdrawn: i64 = native
            .conn
            .query_row(
                "SELECT count(*) FROM s_content_objects WHERE withdrawn_generation=4",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(withdrawn > 0);
        std::fs::write(out.join("RUNTIME-ROOTS.json"),serde_json::to_vec_pretty(&json!({"testonly":true,"formal_driver_network_path":false,"large_frozen_105_runtime_reconstructed":false,"scope":"existing real S3 fixture accept/Begin/Runtime.commit; full audit and real generation/publication unchanged; ordinary exact comparison","stages":reports,"revision_withdrawals":withdrawn,"manual_cache_discard_or_GC":false,"root_exclusions":{"aux.raw_wires.key":"canonical JSON SHA memo fingerprint, not a content node root","bi.RawCoords.empty":"uninitialized empty sequence sentinel is omitted; after normal error cleanup visitor is empty","producer.fractal_histories.union_tail":"integer coordinate tree, no content digest","ArrayCache.tree":"Rc coordinate tree and address key, no content digest","producer.raw_tokens/prepared_raw_keys":"identity tokens and raw logical key strings, no content roots","aux.observation_ids.value":"observation identifier string, not a content pointer","generation.lane_key":"facts lane holds ordinary typed facts, no s-content roots"}})).unwrap()).unwrap();
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum RuntimeRootCell {
        Null,
        Integer(i64),
        RealBits(u64),
        Text(Vec<u8>),
        Blob(Vec<u8>),
    }
    pub(crate) fn runtime_root_tables(
        conn: &Connection,
        include_nodes: bool,
    ) -> Vec<(String, Vec<Vec<RuntimeRootCell>>)> {
        use rusqlite::types::ValueRef;
        let tables = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        tables
            .into_iter()
            .filter(|t| include_nodes || t != "s_content_nodes")
            .map(|table| {
                let order = if table == "s_content_nodes" {
                    "digest"
                } else {
                    "rowid"
                };
                let mut stmt = conn
                    .prepare(&format!("SELECT * FROM {table} ORDER BY {order}"))
                    .unwrap();
                let count = stmt.column_count();
                let rows = stmt
                    .query_map([], |row| {
                        (0..count)
                            .map(|i| {
                                Ok(match row.get_ref(i)? {
                                    ValueRef::Null => RuntimeRootCell::Null,
                                    ValueRef::Integer(v) => RuntimeRootCell::Integer(v),
                                    ValueRef::Real(v) => RuntimeRootCell::RealBits(v.to_bits()),
                                    ValueRef::Text(v) => RuntimeRootCell::Text(v.to_vec()),
                                    ValueRef::Blob(v) => RuntimeRootCell::Blob(v.to_vec()),
                                })
                            })
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                (table, rows)
            })
            .collect()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(crate) fn runtime_root_compare_reachable_bytes(left: &Connection, right: &Connection) {
        use std::collections::{BTreeMap, BTreeSet};
        let nodes = |c: &Connection| {
            c.prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
                .unwrap()
                .query_map([], |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        (r.get::<_, u8>(1)?, r.get::<_, Vec<u8>>(2)?),
                    ))
                })
                .unwrap()
                .collect::<Result<BTreeMap<_, _>, _>>()
                .unwrap()
        };
        let roots = |c: &Connection| {
            let mut roots = Vec::new();
            for (table, columns) in [
                (
                    "s_content_publications",
                    vec!["manifest_root", "snapshot_root", "delta_root"],
                ),
                (
                    "s_content_objects",
                    vec![
                        "generated_root",
                        "internal_root",
                        "immutable_root",
                        "birth_wire_root",
                    ],
                ),
                ("s_content_witnesses", vec!["generated_root", "wire_root"]),
                ("s_content_relations", vec!["generated_root"]),
                (
                    "s_content_observations",
                    vec!["generated_root", "birth_wire_root"],
                ),
            ] {
                for column in columns {
                    roots.extend(
                        c.prepare(&format!("SELECT {column} FROM {table}"))
                            .unwrap()
                            .query_map([], |r| r.get::<_, Vec<u8>>(0))
                            .unwrap()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap(),
                    );
                }
            }
            for (table, column) in [
                ("structure_deltas", "delta_json"),
                ("structure_deltas", "catalog_evidence_json"),
                ("catalog", "evidence_json"),
            ] {
                for raw in c
                    .prepare(&format!("SELECT {column} FROM {table}"))
                    .unwrap()
                    .query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
                {
                    let value = strict_json(raw.as_bytes()).unwrap();
                    if value == json!({}) {
                        continue;
                    }
                    let r = sc::TransportRef::from_json_view((&value).into()).unwrap();
                    roots.push(r.digest().unwrap().to_vec());
                }
            }
            roots
        };
        let a = nodes(left);
        let b = nodes(right);
        let mut todo = roots(left);
        assert_eq!(todo, roots(right));
        let mut seen = BTreeSet::new();
        while let Some(root) = todo.pop() {
            if !seen.insert(root.clone()) {
                continue;
            }
            let (tag, body) = a.get(&root).unwrap();
            assert_eq!(a.get(&root), b.get(&root));
            let offsets: &[usize] = match tag {
                3 | 4 | 5 | 6 => &[8],
                7 | 18 => &[0],
                19 => &[8, 40],
                20 => &[0, 32],
                _ => &[],
            };
            for &offset in offsets {
                todo.push(body[offset..offset + 32].to_vec());
            }
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn runtime_roots_snapshot(
        f: &Fixture,
        out: &Path,
        stage: &str,
        reads: &[sc::NodeDigest],
        elapsed: u128,
    ) -> Value {
        let roots = f.runtime.visit_runtime_roots().unwrap();
        let folder = out.join(stage);
        std::fs::create_dir(&folder).unwrap();
        for name in ["s.sqlite", "s.sqlite-wal", "s.sqlite-shm"] {
            let source = f.path.join(name);
            assert!(source.is_file());
            std::fs::copy(source, folder.join(name)).unwrap();
        }
        let list = roots
            .into_iter()
            .map(|(owner, digest)| json!({"owner":owner,"root":sc::root_string(&digest)}))
            .collect::<Vec<_>>();
        json!({"stage":stage,"generation":meta_i64(&f.conn,"generation").unwrap(),"owned_snapshot":folder,"runtime_roots":list,"SQL_node_reads":reads.iter().map(sc::root_string).collect::<Vec<_>>(),"elapsed_ns":elapsed.to_string(),"scope":"postcommit or after existing error rollback/discard; no inflight locals or external process caches","error_persistent_rows_and_nodes_exact_unchanged":stage=="error-cleanup"})
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

    fn fixed_fields_four_real_commits(mode: s_allocation::Mode) {
        let mut old = Fixture::new();
        let mut new = Fixture::new();
        old.runtime = Runtime::with_mode(mode.clone());
        new.runtime = Runtime::with_mode(mode);
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (oe, ot) = old.accept_begin(n, row, revision);
            let (ne, nt) = new.accept_begin(n, row, revision);
            let expected =
                object_index::with_legacy_candidate_fields_for_test(|| old.publish(&oe, &ot))
                    .unwrap();
            let before = object_index::candidate_field_reads_for_test();
            let actual = new.publish(&ne, &nt).unwrap();
            let after = object_index::candidate_field_reads_for_test();
            assert_eq!(
                canonical_json(&actual.message_receipt),
                canonical_json(&expected.message_receipt)
            );
            assert_eq!(actual.snapshot, expected.snapshot);
            assert_eq!(actual.delta, expected.delta);
            assert_eq!(actual.batch_id, expected.batch_id);
            assert_eq!(
                runtime_root_tables(&new.conn, true),
                runtime_root_tables(&old.conn, true),
                "gen{n}全表/节点/四根逐字相同"
            );
            assert!(
                after.0 < before.0,
                "gen{n} actual candidate SQL {} vs {}",
                after.0,
                before.0
            );
            assert!(
                after.1 < before.1,
                "gen{n} actual candidate F {} vs {}",
                after.1,
                before.1
            );
            eprintln!(
                "fixed_fields real gen={n} old_SQL={} new_SQL={} old_F={} new_F={}",
                before.0, after.0, before.1, after.1
            );
        }
        assert!(count(&new.conn, "s_content_objects") > 0);
        assert!(
            new.conn
                .query_row(
                    "SELECT count(*) FROM s_content_objects WHERE kind='CC-006.local_shape'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap()
                > 0
        );
        assert!(
            new.conn
                .query_row(
                    "SELECT count(*) FROM s_content_objects WHERE kind!='CC-006.local_shape'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap()
                > 0
        );
        assert!(
            new.conn
                .query_row(
                    "SELECT count(*) FROM s_content_objects WHERE withdrawn_generation=4",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap()
                > 0
        );
    }
    #[test]
    fn fixed_fields_plain_four_real_commits_match_legacy_all_rows_nodes_receipts_and_roots() {
        fixed_fields_four_real_commits(s_allocation::Mode::Ordinary);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_fields_owned_four_real_commits_match_legacy_all_rows_nodes_receipts_and_roots() {
        let budget = sc::capture_allocation::OwnedBudget::new(100 * 1024 * 1024);
        fixed_fields_four_real_commits(s_allocation::Mode::Owned(q_allocation::Profile {
            budget: budget.clone(),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        }));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(budget.snapshot().allocation_attempts > 0);
        assert!(!budget.ledger_health().invariant_broken);
        eprintln!("fixed_fields real Owned ledger={:?}", budget.snapshot());
    }

    fn fixed_fields_commit_rollback(mode: s_allocation::Mode) {
        let mut f = Fixture::new();
        f.runtime = Runtime::with_mode(mode);
        let (e, t) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        f.publish(&e, &t).unwrap();
        let (e, t) = f.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
        let before = runtime_root_tables(&f.conn, true);
        f.runtime.after_prepare = Some(|tx| {
            tx.execute_batch("PRAGMA defer_foreign_keys=ON; INSERT INTO s_delivery_refs(generation) VALUES(999);").map_err(err)
        });
        assert!(f.publish(&e, &t).unwrap_err().contains("FOREIGN KEY"));
        assert_eq!(
            before,
            runtime_root_tables(&f.conn, true),
            "失败后全表含接受/Begin/clock和全部节点精确保留"
        );
        f.runtime.after_prepare = None;
        f.publish(&e, &t).unwrap();
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 2);
    }
    #[test]
    fn fixed_fields_plain_real_second_commit_fk_failure_rolls_back_every_persistent_row() {
        fixed_fields_commit_rollback(s_allocation::Mode::Ordinary);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn fixed_fields_owned_real_second_commit_fk_failure_rolls_back_every_persistent_row() {
        let budget = sc::capture_allocation::OwnedBudget::new(50 * 1024 * 1024);
        fixed_fields_commit_rollback(s_allocation::Mode::Owned(q_allocation::Profile {
            budget: budget.clone(),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        }));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(!budget.ledger_health().invariant_broken);
    }
    #[test]
    fn generic_business_store_keeps_legacy_owned_layout() {
        struct Before<'a, 'c> {
            inner: content_store::TxStore<'a, 'c>,
            budget: Option<sc::capture_allocation::OwnedBudget>,
        }
        assert_eq!(
            std::mem::size_of::<BusinessTxStore<content_store::TxStore<'static, 'static>>>(),
            std::mem::size_of::<Before<'static, 'static>>()
        );
        assert_eq!(
            std::mem::align_of::<BusinessTxStore<content_store::TxStore<'static, 'static>>>(),
            std::mem::align_of::<Before<'static, 'static>>()
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_candidate_union_four_real_commits_match_ordinary_roots_rows_and_revision() {
        let mut native = Fixture::new();
        native.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut no_lease = Fixture::new();
        no_lease.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        no_lease.runtime.native_read_lease = false;
        let mut ordinary = Fixture::new();
        assert_eq!(
            native
                .conn
                .query_row("PRAGMA read_uncommitted", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0,
            "actual S config lease qualification"
        );
        let mut record_seeds = 0u64;
        let mut source_hits = 0u64;
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (e, t) = native.accept_begin(n, row, revision);
            let (plain, pt) = ordinary.accept_begin(n, row, revision);
            let (same_native, nt) = no_lease.accept_begin(n, row, revision);
            let native_sql = content_store::sql_reads_on_thread();
            let native_f = sc::frame_calls_on_thread();
            super::super::history::reset_native_record_field_stats();
            let actual = native.publish(&e, &t).unwrap();
            let records = super::super::history::native_record_field_stats();
            record_seeds += records.full_record_seeds;
            source_hits += records.source_subset_hits;
            eprintln!(
                "Native real commit gen{n}: exact_record_seeds={} source_subset_hits={}",
                records.full_record_seeds, records.source_subset_hits
            );
            let native_sql = content_store::sql_reads_on_thread() - native_sql;
            let native_f = sc::frame_calls_on_thread() - native_f;
            let no_sql = content_store::sql_reads_on_thread();
            let no_f = sc::frame_calls_on_thread();
            let original_native = no_lease.publish(&same_native, &nt).unwrap();
            let no_sql = content_store::sql_reads_on_thread() - no_sql;
            let no_f = sc::frame_calls_on_thread() - no_f;
            assert_eq!(actual.message_receipt, original_native.message_receipt);
            assert!(
                native_sql < no_sql,
                "gen{n} same-Native actual node SQL lease {native_sql} off {no_sql}"
            );
            assert!(
                native_f < no_f,
                "gen{n} same-Native actual F lease {native_f} off {no_f}"
            );
            assert_eq!(
                actual.message_receipt,
                ordinary.publish(&plain, &pt).unwrap().message_receipt
            );
            assert_eq!(
                runtime_root_tables(&native.conn, true),
                runtime_root_tables(&no_lease.conn, true),
                "same-Native lease only effect must be physical reads"
            );
            assert_eq!(
                runtime_root_tables(&native.conn, true),
                runtime_root_tables(&ordinary.conn, true),
                "gen{n}真实全部持久行与节点须逐字相同"
            );
        }
        assert!(
            record_seeds > 0 && source_hits > 0,
            "真实冷审计必须命中history→source交接"
        );
        assert_eq!(
            source_hits,
            2 * record_seeds,
            "四代每个完整record恰好承接kind与core两个source字段定位"
        );
        eprintln!("Native real commits record handoff: full_record_seeds={record_seeds} source_subset_hits={source_hits}");
        assert!(count(&native.conn, "s_content_objects") > 0);
        assert_eq!(meta_i64(&native.conn, "generation").unwrap(), 4);
    }

    /// Normal append/revision only. Does not call the older observation/fault entries.
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_manifest_workspace_four_normal_commits_match_cold_and_ordinary_full_rows() {
        manifest_workspace_normal_commits(false);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r10_native_manifest_workspace_shared_scope_normal_commits_match_full_rows() {
        manifest_workspace_normal_commits(true);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn manifest_workspace_normal_commits(admitted: bool) {
        let mut warm = Fixture::new();
        warm.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut cold = Fixture::new();
        cold.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
        cold.runtime.manifest_workspace = false;
        let mut ordinary = Fixture::new();
        // Explicit existing S7, never an unlimited or test-default workspace allowance.
        let total = 536_870_912;
        let mut keeper = Some(super::super::cross_request_source_cache::Keeper::new(total));
        let mut shared_old_pool_seen = false;
        let before = content_manifest::manifest_workspace_stats();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (we, wt) = warm.accept_begin(n, row, revision);
            let (ce, ct) = cold.accept_begin(n, row, revision);
            let (oe, ot) = ordinary.accept_begin(n, row, revision);
            let old_pool = keeper.as_ref().unwrap().retained_source_bytes();
            shared_old_pool_seen |= old_pool > 0;
            let legacy = crate::cache::retained_bytes();
            let mut context =
                request_audit::RequestAuditContext::with_source_keeper(keeper.take().unwrap());
            context
                .freeze_manifest_workspace_budget(total, legacy, old_pool)
                .unwrap();
            let allowance = context.manifest_workspace_budget().unwrap().allowance();
            assert_eq!(
                legacy
                    .checked_add(old_pool)
                    .and_then(|v| v.checked_add(allowance)),
                Some(total)
            );
            let start = std::time::Instant::now();
            let mut tx = warm.conn.transaction().unwrap();
            let wr = if admitted {
                // Explicit finite prototype heap cap, independent of the S7 source allowance.
                let parent =
                    super::super::audit_owner::native::Budget::new(64 * 1024 * 1024, 0).unwrap();
                let baseline = parent.live();
                let receipt = parent
                    .codec_work()
                    .unwrap()
                    .run(|work| {
                        warm.runtime.publish_impl_admitted(
                            &mut tx,
                            &wt,
                            &we,
                            Some(&mut context),
                            None,
                            Some(work),
                        )
                    })
                    .unwrap();
                assert!(parent.peak() > baseline);
                assert_eq!(parent.scratch_observation().0, 0);
                assert_eq!(parent.scratch_observation().2, 0);
                eprintln!(
                    "R10_PUBLICATION generation={n} parent_peak={} native_live={}",
                    parent.peak(),
                    parent.live()
                );
                tx.commit().unwrap();
                receipt
            } else {
                warm.runtime
                    .commit_in_request(tx, &wt, &we, &mut context)
                    .unwrap()
            };
            let workspace_cost =
                content_manifest::manifest_workspace_stats().last_workspace_source_cost as usize;
            assert!(
                legacy
                    .checked_add(old_pool)
                    .and_then(|v| v.checked_add(workspace_cost))
                    .unwrap()
                    <= total
            );
            keeper = super::super::audit_owner::finish_fixture_source_keeper(&warm.conn, context);
            eprintln!(
                "manifest workspace owner handoff gen{n}: {}",
                keeper.as_ref().unwrap().report()
            );
            let retained = keeper.as_ref().unwrap().retained_source_bytes();
            if n == 1 {
                // 第一次 prepare 审计的是 generation0；没有旧 cut 的来源证书可保留。
                assert_eq!(retained, 0, "首代不得凭新生成产品签发旧历史资格");
            } else {
                assert!(retained > 0, "已有旧 cut 的后续代须真实保留来源池");
            }
            assert!(
                crate::cache::retained_bytes()
                    .checked_add(keeper.as_ref().unwrap().retained_source_bytes())
                    .unwrap()
                    <= total
            );
            let elapsed = start.elapsed();
            let cr = cold.publish(&ce, &ct).unwrap();
            let or = ordinary.publish(&oe, &ot).unwrap();
            for other in [&cr, &or] {
                assert_eq!(wr.batch_id, other.batch_id);
                assert_eq!(wr.snapshot, other.snapshot);
                assert_eq!(wr.delta, other.delta);
                assert_eq!(wr.message_receipt, other.message_receipt);
            }
            let persistent = runtime_root_tables(&warm.conn, true);
            assert_eq!(persistent, runtime_root_tables(&cold.conn, true));
            assert_eq!(persistent, runtime_root_tables(&ordinary.conn, true));
            runtime_root_compare_reachable_bytes(&warm.conn, &cold.conn);
            runtime_root_compare_reachable_bytes(&warm.conn, &ordinary.conn);
            for fixture in [&mut warm, &mut cold, &mut ordinary] {
                let tx = fixture.conn.transaction().unwrap();
                let audited = state::audit(&tx).unwrap();
                assert_eq!(audited.cuts.len(), n as usize);
                assert_eq!(audited.raw.len(), n as usize);
                assert!(!audited.objects.is_empty());
            }
            eprintln!("manifest workspace normal gen{n}: warm_elapsed={elapsed:?}");
        }
        assert!(
            shared_old_pool_seen,
            "normal workspace must share S7 with an actual nonempty old pool"
        );
        let after = content_manifest::manifest_workspace_stats();
        assert_eq!(after.issued - before.issued, 4);
        assert_eq!(after.rebound - before.rebound, 4);
        assert_eq!(after.overflow - before.overflow, 0);
        assert_eq!(
            after.publication_owner_scopes - before.publication_owner_scopes,
            after.rebound - before.rebound
        );
        assert_eq!(
            after.publication_owner_drops - before.publication_owner_drops,
            after.publication_owner_scopes - before.publication_owner_scopes
        );
        assert!(after.publication_reused_reads > before.publication_reused_reads);
        assert!(after.publication_fallback_sql_reads > before.publication_fallback_sql_reads);
        assert!(after.publication_put_sql_checks > before.publication_put_sql_checks);
        assert_eq!(
            after.publication_unknown_write_invalidations
                - before.publication_unknown_write_invalidations,
            0
        );
        assert!(after.rebind_rows > before.rebind_rows);
        assert!(after.replay_puts > before.replay_puts);
        assert!(
            warm.conn
                .query_row(
                    "SELECT COUNT(*) FROM s_content_objects WHERE withdrawn_generation=4",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap()
                > 0
        );
        eprintln!("manifest workspace stats: before={before:?} after={after:?}");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_read_lease_boundary_cold_publish_fault_matrix_discards_runtime() {
        for fault in 0..3 {
            let mut f = Fixture::new();
            f.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
            let (e, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
            f.runtime.after_prepare = Some(match fault {
                0 => |tx| {
                    tx.execute("UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT birth_wire_root FROM s_content_objects LIMIT 1)",[]).map_err(err)?;
                    Ok(())
                },
                1 => |tx| {
                    tx.execute("DELETE FROM s_content_nodes WHERE digest=(SELECT generated_root FROM s_content_objects LIMIT 1)",[]).map_err(err)?;
                    Ok(())
                },
                _ => |tx| {
                    tx.execute_batch("PRAGMA defer_foreign_keys=ON;INSERT INTO s_delivery_refs(generation) VALUES(999);").map_err(err)
                },
            });
            assert!(
                f.publish(&e, &token).is_err(),
                "after-prepare fault {fault} must be checked through cold publication"
            );
            assert!(
                f.runtime.binding.is_some() && f.runtime.generator.is_some(),
                "discard preserves the original binding/generator shell"
            );
            assert!(f.runtime.visit_runtime_roots().unwrap().is_empty());
            assert_eq!(count(&f.conn, "s_content_nodes"), 0);
            assert_eq!(count(&f.conn, "s_content_publications"), 0);
            assert_eq!(count(&f.conn, "s_content_objects"), 0);
            f.runtime.after_prepare = None;
            assert_eq!(f.publish(&e, &token).unwrap().generation, 1);
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_local_material_memo_cannot_publish_trigger_damaged_deep_key() {
        for delete in [true, false] {
            let mut f = Fixture::new();
            f.runtime = Runtime::with_mode(s_allocation::Mode::CapturedNative);
            for (n, row) in [
                (1, ("a", 0, 6, 12, 3, 7)),
                (2, ("b", 10, 8, 16, 6, 12)),
                (3, ("c", 20, 8, 13, 4, 7)),
            ] {
                let (envelope, token) = f.accept_begin(n, row, 1);
                f.publish(&envelope, &token).unwrap();
            }
            // 首个有后续bar的真实分型单元才会写此键；父reference_prices与unit本身不用它。
            let body = b"reference_value";
            let target = sc::frame(sc::tag::BYTES_CHUNK, body);
            let present: i64 = f
                .conn
                .query_row(
                    "SELECT count(*) FROM s_content_nodes WHERE digest=?1",
                    [&target[..]],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(present, 0, "夹具必须实际进入首次24项关系编码");
            let (envelope, token) = f.accept_begin(4, ("d", 30, 12, 18, 8, 15), 1);
            f.conn
                .execute_batch("CREATE TEMP TABLE native_local_fault_seen(n INTEGER NOT NULL)")
                .unwrap();
            // 节点表WITHOUT ROWID不提供update hook；触发器里的普通表INSERT证明实际触发，
            // Arc计数不受随后SQL回滚影响，表本身则必须一起回滚。
            let fired = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = fired.clone();
            f.conn.update_hook(Some(
                move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                    if table == "native_local_fault_seen" {
                        observed.fetch_add(1, Ordering::SeqCst);
                    }
                },
            ));
            let hex = sc::root_string(&target);
            let hex = hex.strip_prefix("sc1-").unwrap();
            let damage = if delete {
                format!("DELETE FROM s_content_nodes WHERE digest=x'{hex}'")
            } else {
                format!("UPDATE s_content_nodes SET body=x'00' WHERE digest=x'{hex}'")
            };
            f.conn
                .execute_batch(&format!(
                    "CREATE TEMP TRIGGER damage_native_local_key AFTER INSERT ON main.s_content_nodes \
                     WHEN NEW.tag=20 AND EXISTS(SELECT 1 FROM s_content_nodes WHERE digest=x'{hex}') \
                     BEGIN INSERT INTO native_local_fault_seen VALUES(1); {damage}; END;"
                ))
                .unwrap();
            let before = runtime_root_tables(&f.conn, true);
            let error = f.publish(&envelope, &token).unwrap_err();
            assert!(
                fired.load(Ordering::SeqCst) > 0,
                "必须击中真实INSERT触发器，实际错误：{error}"
            );
            let expected = if delete {
                sc::code::NODE_MISSING
            } else {
                sc::code::NODE_DIGEST
            };
            assert!(error.contains(expected), "{expected}: {error}");
            assert_eq!(runtime_root_tables(&f.conn, true), before);
            assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 3);
            assert_eq!(count(&f.conn, "s_content_publications"), 3);
            assert_eq!(count(&f.conn, "native_local_fault_seen"), 0);
            assert!(f.runtime.visit_runtime_roots().unwrap().is_empty());
            f.conn
                .execute_batch("DROP TRIGGER damage_native_local_key")
                .unwrap();
            assert_eq!(f.publish(&envelope, &token).unwrap().generation, 4);
            let actual: Vec<u8> = f
                .conn
                .query_row(
                    "SELECT body FROM s_content_nodes WHERE digest=?1",
                    [&target[..]],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(actual, body);
        }
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

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_capture_sqlite_commit_failure_retries_with_new_current_capture() {
        s_allocation::probe::start(None);
        check_sqlite_commit_failure_with_mode(s_allocation::Mode::CapturedNative);
        let (attempts, captures) = s_allocation::probe::finish();
        assert_eq!(attempts, [0, 0, 2]);
        assert_eq!(captures[2].unwrap().capture_return_ordinal, 2);
        assert_eq!(captures[2].unwrap().live_before, 0);
    }

    fn check_sqlite_commit_failure(allocation: Option<q_allocation::Profile>) {
        check_sqlite_commit_failure_with_mode(allocation.into());
    }

    fn check_sqlite_commit_failure_with_mode(mode: s_allocation::Mode) {
        let mut f = Fixture::new();
        f.runtime = Runtime::with_mode(mode);
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
            assert!(
                f.runtime
                    .generator
                    .as_ref()
                    .unwrap()
                    .sequence_keys_bound_to_for_test(&budget),
                "真实首代六份持久keys须全部绑定AuditedState原账，包括空keys"
            );
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
                f.runtime.mode = s_allocation::Mode::Owned(q_allocation::Profile {
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
            f.runtime.mode = s_allocation::Mode::Owned(profile);
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
