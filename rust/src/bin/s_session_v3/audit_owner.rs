//! #1394：Driver 的连接生涯与有限事务入口。
//!
//! 连接和事务不向调用者解引用；固定操作返回独立结果，游标在操作内结束。
//! data_version 只在真实 main 读快照建立后、同连接第一次写入前取得。

use super::*;
use newchan_rust::session_protocol::required;
use rusqlite::Transaction;
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "s_capture_native_testonly")]
#[path = "sqlite_pager_probe.rs"]
mod sqlite_pager_probe;
#[cfg(feature = "s_capture_native_testonly")]
pub(super) struct PagerSnapshot(sqlite_pager_probe::Snapshot);
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn pager_enabled() -> bool {
    sqlite_pager_probe::enabled()
}
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn log_pager_handle(
    before: PagerSnapshot,
    after: PagerSnapshot,
    id: Option<Value>,
    ok: bool,
) {
    sqlite_pager_probe::log_handle(before.0, after.0, id, ok);
}

#[cfg(feature = "s_capture_native_testonly")]
pub(super) mod effects;
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) mod native;
#[cfg(feature = "s_capture_native_testonly")]
mod q;

/// 只有 owner 从同快照完整候选以及固定 effects 闭合后创建；不对外开放构造。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) mod allocation_probe;

#[cfg(feature = "s_capture_native_testonly")]
#[path = "construction_set.rs"]
mod construction_set;
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) use effects::StageKind;
#[path = "construction_state.rs"]
mod construction_state;
pub(crate) use super::business_json::{RawWinner, RawWinnerEdit};
#[cfg(test)]
pub(crate) use construction_state::RawAdvanceFixture;
pub(crate) use construction_state::{BoundRawAdvance, RawAdvanceBinding, RawDomain};

pub(crate) struct BoundOriginals {
    _private: (),
}

static NEXT_LIFE: AtomicU64 = AtomicU64::new(1);

pub(super) struct AuditConnection {
    connection: Connection,
    life: u64,
    retired: Cell<bool>,
    next_transaction: Cell<u64>,
    write_sequence: Cell<u64>,
    construction_lease: construction_state::Lease,
    construction: std::cell::RefCell<Option<construction_state::CommittedRawState>>,
    #[cfg(feature = "s_capture_native_testonly")]
    complete: std::cell::RefCell<Option<CompleteCandidate>>,
    #[cfg(feature = "s_capture_native_testonly")]
    source_keeper: std::cell::RefCell<Option<cross_request_source_cache::Keeper>>,
}

/// 此标记只证明 owner 已确认事务终结，不证明任何缓存依赖保持。
pub(super) struct Ended {
    life: u64,
    #[cfg(feature = "s_capture_native_testonly")]
    complete: Option<CompleteCandidate>,
}

/// 下层只接受固定发布能力；没有 SQL、Connection 或 commit 的访问器。
pub(super) struct PublishPhase<'a, 'db> {
    transaction: &'a mut Transaction<'db>,
    #[cfg(feature = "s_capture_native_testonly")]
    audited: Option<&'a state::AuditedState>,
}
impl PublishPhase<'_, '_> {
    pub(super) fn publish(
        self,
        runtime: &mut prepared::Runtime,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<commit::CommitReceipt, String> {
        runtime.publish_impl(
            self.transaction,
            token,
            envelope,
            Some(context),
            #[cfg(feature = "s_capture_native_testonly")]
            self.audited,
        )
    }
}

struct AuditTxn<'a> {
    owner: &'a AuditConnection,
    transaction: Option<Transaction<'a>>,
    // 只能随本事务封存；结束后不重新采样。
    data_version: i64,
    schema_version: i64,
    transaction_id: u64,
    write_sequence: u64,
    dirty: Cell<bool>,
    construction: std::cell::RefCell<Option<construction_state::PendingRawState>>,
    #[cfg(feature = "s_capture_native_testonly")]
    complete: std::cell::RefCell<Option<CompleteCandidate>>,
    #[cfg(feature = "s_capture_native_testonly")]
    pending: std::cell::RefCell<Option<CompleteCandidate>>,
    #[cfg(feature = "s_capture_native_testonly")]
    recording: std::cell::RefCell<Option<effects::Recording>>,
    #[cfg(feature = "s_capture_native_testonly")]
    effect_limit: Cell<usize>,
}

pub(super) enum Admission {
    Reply(Value),
    Advance { original: Value, accepted: bool },
}

/// 保留原 Advance→Prepare 的实际工作集重叠；Owned 峰值不能因拆函数被静默改口径。
pub(super) struct Begun {
    pub(super) token: String,
    _audited: Option<state::AuditedState>,
}

impl AuditConnection {
    // 生产调用只接受路径，经固定无扩展/无自定义函数的 private-cache 开门过程创建。
    // 从此没有 register_function/ATTACH/TEMP/schema/配置写能力外借。
    pub(super) fn open(db: &Path) -> Result<Self, String> {
        Self::new(open_db(db)?)
    }

    fn new(connection: Connection) -> Result<Self, String> {
        let life = NEXT_LIFE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| invalid("连接生涯编号耗尽"))?;
        if !connection.is_autocommit() {
            return Err(invalid("不能接管已有活动事务的连接"));
        }
        Ok(Self {
            connection,
            life,
            retired: Cell::new(false),
            next_transaction: Cell::new(0),
            write_sequence: Cell::new(0),
            construction_lease: Default::default(),
            construction: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            complete: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            source_keeper: Default::default(),
        })
    }

    fn invalidate_construction(&self) {
        self.construction_lease.invalidate();
        drop(self.construction.borrow_mut().take());
    }

    fn clear_candidates(&self) {
        self.invalidate_construction();
        self.clear_optional_candidates();
    }

    fn clear_optional_candidates(&self) {
        #[cfg(feature = "s_capture_native_testonly")]
        {
            drop(self.source_keeper.borrow_mut().take());
            drop(self.complete.borrow_mut().take());
        }
    }

    fn begin(&self, behavior: TransactionBehavior) -> Result<AuditTxn<'_>, String> {
        if self.retired.get() || !self.connection.is_autocommit() {
            self.retired.set(true);
            self.clear_candidates();
            return Err(invalid("连接已退役或存在未终结事务"));
        }
        let transaction_id = self
            .next_transaction
            .get()
            .checked_add(1)
            .ok_or_else(|| invalid("事务编号耗尽"))?;
        self.next_transaction.set(transaction_id);
        let transaction =
            Transaction::new_unchecked(&self.connection, behavior).map_err(|error| {
                self.clear_candidates();
                invalid(error)
            })?;
        let mut active = AuditTxn {
            owner: self,
            transaction: Some(transaction),
            data_version: 0,
            schema_version: 0,
            transaction_id,
            write_sequence: self.write_sequence.get(),
            dirty: Cell::new(false),
            construction: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            complete: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            pending: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            recording: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            effect_limit: Cell::new(0),
        };
        // COUNT(*) 确实访问 main.meta Btree；SELECT 1/只读 PRAGMA 不能替代此步。
        let _: i64 = active
            .tx()
            .query_row("SELECT COUNT(*) FROM main.meta", [], |r| r.get(0))
            .map_err(invalid)?;
        active.data_version = active
            .tx()
            .query_row("PRAGMA main.data_version", [], |r| r.get(0))
            .map_err(invalid)?;
        active.schema_version = active
            .tx()
            .query_row("PRAGMA main.schema_version", [], |r| r.get(0))
            .map_err(invalid)?;
        Ok(active)
    }

    pub(super) fn startup(
        &self,
        profile: &Path,
        epoch: &str,
        clock: &v2::ClockPlan,
        request: Option<&Value>,
        mode: &s_allocation::Mode,
        context: Option<&mut request_audit::RequestAuditContext>,
    ) -> Result<(String, Option<String>), String> {
        let active = self.begin(TransactionBehavior::Deferred)?;
        let tx = active.tx();
        if let Some(context) = context {
            if !matches!(mode, s_allocation::Mode::CapturedNative) {
                return Err(invalid("Native bootstrap只接受显式CapturedNative模式"));
            }
            state::audit_native_in_request(tx, context)?;
        } else {
            s_allocation::audit_startup(tx, mode)?;
        }
        verify_writer_epoch(tx, epoch)?;
        let control = control::S3Control::attach(tx, clock)?;
        let (profile_value, profile_id, profile_hash) = load_profile(profile)?;
        if profile_id != tb02a::PROFILE {
            return Err("VersionUnsupported：S3驱动尚未实现非OHLC生成，拒绝接纳".into());
        }
        if meta_get_opt(tx, "profile_hash")?.as_deref() != Some(profile_hash.as_str()) {
            return Err("IdentityConflict：S3驱动profile与冻结档案不符".into());
        }
        let volume_unit = profile_value
            .get("volume_unit")
            .and_then(Value::as_str)
            .map(str::to_string);
        if meta_get_opt(tx, "advance_state")?.as_deref() != Some("idle") {
            return Err("StaleWriter：未完成Begin须先由监督器recover".into());
        }
        if let Some(request) = request {
            driver::check_request_before_resume(tx, &control, request, &profile_id)?;
        }
        drop(control);
        active.read_end()?;
        Ok((profile_id, volume_unit))
    }

    pub(super) fn identity(
        &self,
        epoch: &str,
        clock: &v2::ClockPlan,
        context: Option<&mut request_audit::RequestAuditContext>,
    ) -> Result<(String, String, String), String> {
        let active = self.begin(TransactionBehavior::Deferred)?;
        let tx = active.tx();
        match context {
            Some(context) => {
                state::audit_native_in_request(tx, context)?;
            }
            None => {
                state::audit(tx)?;
            }
        }
        verify_writer_epoch(tx, epoch)?;
        let control = control::S3Control::attach(tx, clock)?;
        let (session, generation, epoch) = control.identity();
        let result = (session.into(), generation.into(), epoch.into());
        drop(control);
        active.read_end()?;
        Ok(result)
    }

    pub(super) fn summary(
        &self,
        command: &str,
        epoch: &str,
        clock: &v2::ClockPlan,
    ) -> Result<Value, String> {
        let active = self.begin(TransactionBehavior::Deferred)?;
        let tx = active.tx();
        state::audit(tx)?;
        verify_writer_epoch(tx, epoch)?;
        let control = control::S3Control::attach(tx, clock)?;
        let (session, incarnation, epoch) = control.identity();
        let pending: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM s_input_messages WHERE status='AcceptedPending'",
                [],
                |r| r.get(0),
            )
            .map_err(invalid)?;
        let result = json!({"ok":true,"command":command,"session_id":session,
            "session_generation":incarnation,"writer_epoch":epoch,
            "generation":meta_get_opt(tx,"generation")?,"structure_cut":meta_get_opt(tx,"structure_cut")?,
            "index_frontier":meta_get_opt(tx,"index_frontier")?,"accepted_pending":pending.to_string()});
        drop(control);
        active.read_end()?;
        Ok(result)
    }

    pub(super) fn admit(
        &self,
        envelope: Value,
        epoch: &str,
        clock: &v2::ClockPlan,
        profile_id: &str,
        volume_unit: Option<&str>,
        mode: &s_allocation::Mode,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<(Admission, Ended), String> {
        let active = self.begin(TransactionBehavior::Deferred)?;
        let tx = active.tx();
        active.audit_admission(&envelope, mode, context)?;
        verify_writer_epoch(tx, epoch)?;
        let control = control::S3Control::attach(tx, clock)?;
        let (sid, inc, _) = control.identity();
        if required(&envelope, "session_id")? != sid
            || required(&envelope, "session_generation")? != inc
        {
            return Err("IdentityConflict：请求不属于本会话/化身".into());
        }
        let payload = &envelope["payload"];
        let mut wrote = false;
        let result = match required(payload, "op")? {
            "ready" => Admission::Reply(v2::ready_for_protocol(tx, PROTOCOL_REVISION)?),
            "receipt" => Admission::Reply(v2::lookup(
                tx,
                required(payload, "original_source_namespace")?,
                required(payload, "original_source_epoch")?,
                required(payload, "original_message_id")?,
                required(payload, "original_payload_hash")?,
            )?),
            "ingest" => {
                let input = tb02a::parse_input(&payload["raw_input"])?;
                raw_input::validate_profile_binding(&input, profile_id)?;
                let view = control.inspect_ingest(envelope.clone())?;
                if view.is_committed() {
                    Admission::Reply(view.receipt().clone())
                } else {
                    if meta_get_opt(tx, "advance_state")?.as_deref() != Some("idle") {
                        return Err("StaleWriter：已有未完成Begin，禁止新接纳或再次Begin".into());
                    }
                    if view.is_fresh() {
                        let pending: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM s_input_messages WHERE status='AcceptedPending')",[],|r|r.get(0)).map_err(invalid)?;
                        if pending {
                            return Err(
                                "MissingDependency：须先完成已接纳消息，禁止新消息越过未发布前沿"
                                    .into(),
                            );
                        }
                        let event = required(payload, "clock_event_id")?;
                        clock.phase(event, "accept", -1)?;
                        clock.phase(event, "begin", 0)?;
                        clock.phase(event, "commit", 0)?;
                        // #1488：仅测试选择已审 owner 原型；所有正式准入与实际 Accept/COMMIT 原样执行。
                        #[cfg(all(test, feature = "s_capture_native_testonly"))]
                        construction_tests::select_for_formal_accept(&active, context)?;
                        active.before_fixed_write(FixedOperation::Accept, Some(context))?;
                        let accepted = raw_input::accept_events(tx, &input, volume_unit)?;
                        control.accept(&view, &accepted)?;
                        wrote = true;
                        // #1394：新消息可以引用已发布 raw；接纳控制账此时已将其
                        // 记为 Committed。回读实际状态并返回原发布回执，不再 Begin。
                        let accepted_view = control.inspect_ingest(envelope.clone())?;
                        if accepted_view.is_committed() {
                            Admission::Reply(accepted_view.receipt().clone())
                        } else {
                            Admission::Advance {
                                original: envelope,
                                accepted: true,
                            }
                        }
                    } else {
                        let original = control
                            .load_pending(
                                required(&envelope, "source_namespace")?,
                                required(&envelope, "source_epoch")?,
                                required(&envelope, "message_id")?,
                            )?
                            .envelope()
                            .clone();
                        Admission::Advance {
                            original,
                            accepted: false,
                        }
                    }
                }
            }
            _ => return Err("SchemaUnsupported：未知S3 op；recover只属于监督器".into()),
        };
        drop(control);
        let ended = if wrote {
            active.finish_fixed_write()?;
            active.commit()?
        } else {
            active.read_end()?
        };
        Ok((result, ended))
    }

    pub(super) fn begin_advance(
        &self,
        envelope: &Value,
        epoch: &str,
        clock: &v2::ClockPlan,
        mode: &s_allocation::Mode,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<Begun, String> {
        let active = self.begin(TransactionBehavior::Immediate)?;
        let tx = active.tx();
        let reused = active.bind_existing(Some(context))?;
        let audited = if reused {
            None
        } else {
            Some(s_allocation::audit_in_request(
                tx,
                mode,
                s_allocation::Phase::Advance,
                context,
            )?)
        };
        verify_writer_epoch(tx, epoch)?;
        if meta_get_opt(tx, "advance_state")?.as_deref() != Some("idle") {
            return Err("StaleWriter：已有未完成Begin".into());
        }
        let control = control::S3Control::attach(tx, clock)?;
        let view = control.load_pending(
            required(envelope, "source_namespace")?,
            required(envelope, "source_epoch")?,
            required(envelope, "message_id")?,
        )?;
        if view.envelope() != envelope {
            return Err("IdentityConflict：推进原件与已接纳消息不同".into());
        }
        let generation = meta_i64(tx, "generation")?
            .checked_add(1)
            .ok_or_else(|| invalid("代号溢出"))?;
        let raw_len = match &audited {
            Some(a) => a.raw.len(),
            None => active.bound_raw_len()?,
        };
        let frontier = i64::try_from(raw_len)
            .map_err(invalid)?
            .checked_sub(1)
            .ok_or_else(|| invalid("没有原始输入"))?;
        let base = meta_get_opt(tx, "structure_cut")?.ok_or_else(|| invalid("缺当前cut"))?;
        active.before_fixed_write(FixedOperation::Begin, Some(context))?;
        let token = control.begin(&view, generation, frontier, &base, epoch)?;
        drop(control);
        active.finish_fixed_write()?;
        let ended = active.commit()?;
        self.continue_with(ended)?;
        Ok(Begun {
            token,
            _audited: audited,
        })
    }

    pub(super) fn publish(
        &self,
        runtime: &mut prepared::Runtime,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<(commit::CommitReceipt, Ended), String> {
        let result = (|| {
            let mut active = self.begin(TransactionBehavior::Immediate)?;
            // 固定 phase 覆盖 Runtime 内全部 node/stage/非节点/发布写；无裸 SQL 能力外借。
            active.bind_existing(Some(context))?;
            active.before_fixed_write(FixedOperation::Publish, Some(context))?;
            #[cfg(feature = "s_capture_native_testonly")]
            let pending = active.pending.borrow();
            let phase = PublishPhase {
                transaction: active.transaction.as_mut().unwrap(),
                #[cfg(feature = "s_capture_native_testonly")]
                audited: pending.as_ref().map(|c| &c.audited),
            };
            let receipt = runtime.publish_in_owner(phase, token, envelope, context)?;
            #[cfg(feature = "s_capture_native_testonly")]
            drop(pending);
            active.finish_fixed_write()?;
            let ended = active.commit_publication()?;
            Ok((receipt, ended))
        })();
        if result.is_err() {
            runtime.discard();
            drop(context.take());
        }
        result
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn take_source_keeper(&mut self) -> Option<cross_request_source_cache::Keeper> {
        self.source_keeper.borrow_mut().take()
    }

    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn source_keeper_for_test(
        &self,
    ) -> std::cell::Ref<'_, Option<cross_request_source_cache::Keeper>> {
        self.source_keeper.borrow()
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn finish_source_keeper(
        &mut self,
        context: request_audit::RequestAuditContext,
        ended: Ended,
    ) {
        *self.source_keeper.get_mut() = None;
        if ended.life != self.life || self.retired.get() || !self.connection.is_autocommit() {
            return;
        }
        // 完整候选由本 owner 的无写审计或固定 effects 核验签发；来源池独立移交。
        let cap = context.complete_retention_cap();
        let raw_retained = self
            .construction
            .get_mut()
            .as_ref()
            .map_or(0, |c| c.retained_bytes());
        let cap = match cap.checked_sub(raw_retained) {
            Some(cap) => cap,
            None => {
                self.invalidate_construction();
                0
            }
        };
        let mut complete = ended.complete;
        // 只有对应事务成功终结后封存的结果可接回。slot/fixture 没有签发能力。
        if let Some(candidate) = &complete {
            if candidate.seal.life != self.life
                || candidate.seal.write_sequence != self.write_sequence.get()
                || candidate.seal.transaction_id != self.next_transaction.get()
                || candidate.charge.total > cap
            {
                complete = None;
            }
        }
        // 完整候选已经按值接收 slot；这里的旧 keeper 仍沿原收费/释放规则收尾。
        *self.source_keeper.get_mut() = context.finish_source_keeper(Ended {
            life: self.life,
            complete: None,
        });
        let sparse = self
            .source_keeper
            .get_mut()
            .as_ref()
            .map_or(0, |k| k.retained_source_bytes());
        if complete
            .as_ref()
            .is_some_and(|c| c.charge.total.saturating_add(sparse) > cap)
        {
            complete = None;
        }
        *self.complete.get_mut() = complete;
    }

    pub(super) fn pending_originals(&self) -> Result<Vec<String>, String> {
        let active = self.begin(TransactionBehavior::Deferred)?;
        let originals = {
            let mut stmt = active.tx().prepare("SELECT canonical_envelope FROM s_input_messages WHERE status='AcceptedPending' ORDER BY accepted_seq,message_id").map_err(invalid)?;
            let mut rows = stmt.query([]).map_err(invalid)?;
            let mut originals = Vec::new();
            while let Some(row) = rows.next().map_err(invalid)? {
                originals.push(row.get(0).map_err(invalid)?);
            }
            drop(rows);
            stmt.finalize().map_err(invalid)?;
            originals
        };
        active.read_end()?;
        Ok(originals)
    }

    pub(super) fn recover(
        &self,
        new_epoch: &str,
        clock: &v2::ClockPlan,
        clock_event: &str,
    ) -> Result<Value, String> {
        let next = parse_writer_epoch(new_epoch, "new writer_epoch")?;
        let active = self.begin(TransactionBehavior::Immediate)?;
        let tx = active.tx();
        if !active.bind_existing(None)? {
            state::audit(tx)?;
        }
        let current = meta_get_opt(tx, "writer_epoch")?.ok_or_else(|| invalid("缺writer_epoch"))?;
        if next <= parse_writer_epoch(&current, "current writer_epoch")? {
            return Err("StaleWriter：recover换代必须严格上升".into());
        }
        let control = control::S3Control::attach(tx, clock)?;
        let generation = meta_get_opt(tx, "generation")?.ok_or_else(|| invalid("缺generation"))?;
        let prior = meta_get_opt(tx, "advance_state")?.ok_or_else(|| invalid("缺advance_state"))?;
        active.before_fixed_write(FixedOperation::Recover, None)?;
        let transitioned_at = control.recover(clock_event, &generation)?;
        let recovered = if prior != "idle" {
            meta_set(tx, "advance_state", "idle")?;
            meta_set(tx, "begin_token", "")?;
            meta_set(
                tx,
                "last_recovered_begin",
                &format!("{prior}:recovered_at_generation_{generation}"),
            )?;
            json!({"advance_state":prior,"resolved_to":"idle","generation_at_recovery":generation})
        } else {
            Value::Null
        };
        meta_set(tx, "writer_epoch", new_epoch)?;
        tx.execute("INSERT INTO writer_epoch_history(from_epoch,to_epoch,generation_at_transition,advance_state_at_transition,transitioned_at) VALUES(?1,?2,?3,'idle',?4)",params![current,new_epoch,generation,transitioned_at]).map_err(invalid)?;
        let result = json!({"ok":true,"previous_epoch":current,"new_epoch":new_epoch,"epoch_transitioned":true,"recovered_begin":recovered,
            "reachable_root":{"generation":generation,"structure_cut":meta_get_opt(tx,"structure_cut")?,"index_frontier":meta_get_opt(tx,"index_frontier")?}});
        drop(control);
        active.finish_fixed_write()?;
        let ended = active.commit()?;
        self.continue_with(ended)?;
        Ok(result)
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn configure_pager_probe(&self) -> Result<(), String> {
        self.clear_candidates();
        sqlite_pager_probe::configure_and_log(&self.connection)
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn pager_snapshot(&self) -> PagerSnapshot {
        PagerSnapshot(sqlite_pager_probe::snapshot(&self.connection))
    }
}

impl AuditTxn<'_> {
    fn tx(&self) -> &Transaction<'_> {
        self.transaction.as_ref().unwrap()
    }

    fn read_end(mut self) -> Result<Ended, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(c) = self.complete.borrow().as_ref() {
            if c.seal.transaction_id != self.transaction_id
                || c.seal.life != self.owner.life
                || c.seal.write_sequence != self.owner.write_sequence.get()
                || self.dirty.get()
            {
                return Err(invalid("完整候选不属于本次实际无写审计"));
            }
        }
        if self.construction.borrow().is_some() {
            self.owner.invalidate_construction();
            return Err(construction_state::stale(
                "构造 pending 不可通过只读回滚晋升",
            ));
        }
        self.owner.connection.flush_prepared_statement_cache();
        let result = self.transaction.take().unwrap().rollback().map_err(invalid);
        if result.is_err() || !self.owner.connection.is_autocommit() {
            self.owner.clear_candidates();
            self.owner.retired.set(true);
        }
        result?;
        if self.owner.retired.get() {
            return Err(invalid("读事务未确认终结"));
        }
        Ok(Ended {
            life: self.owner.life,
            #[cfg(feature = "s_capture_native_testonly")]
            complete: if !self.dirty.get() {
                self.complete.borrow_mut().take()
            } else {
                None
            },
        })
    }

    fn commit(mut self) -> Result<Ended, String> {
        self.commit_as(false)
    }

    fn commit_publication(mut self) -> Result<Ended, String> {
        self.commit_as(true)
    }

    fn commit_as(&mut self, publication: bool) -> Result<Ended, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(c) = self.complete.borrow().as_ref() {
            if c.seal.life != self.owner.life
                || c.seal.transaction_id != self.transaction_id
                || c.seal.write_sequence != self.owner.write_sequence.get()
            {
                return Err(invalid("写后候选没有绑定本次已闭合事务"));
            }
        }
        if self.construction.borrow().is_some() {
            let checked = (|| {
                if self.owner.construction_lease.stale.get() {
                    return Err(construction_state::stale("提交前租约已撤销"));
                }
                self.construction
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .check_commit(self.construction_seal()?, self.tx().total_changes())
            })();
            if let Err(error) = checked {
                self.owner.invalidate_construction();
                return Err(error);
            }
        }
        // 此处封存锁内读取的版本；不得在 commit 后刷新 data_version。
        let _sealed = (
            self.owner.life,
            self.transaction_id,
            self.write_sequence,
            self.data_version,
            self.schema_version,
        );
        self.owner.connection.flush_prepared_statement_cache();
        let mut transaction = self.transaction.take().unwrap();
        // 提交失败后的终结由 owner 核实，不依赖 Transaction 析构器的隐式回滚。
        transaction.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        let result = transaction.commit().map_err(|e| {
            if publication {
                format!("PublicationInvalid：{e}")
            } else {
                invalid(e)
            }
        });
        if result.is_err() {
            self.owner.clear_candidates();
            if !self.owner.connection.is_autocommit() {
                let rollback = self.owner.connection.execute_batch("ROLLBACK");
                if rollback.is_err() || !self.owner.connection.is_autocommit() {
                    self.owner.retired.set(true);
                }
            } else {
                // 错误返回但已无事务，不视为经 owner 确认的正常回滚。
                self.owner.retired.set(true);
            }
        } else if !self.owner.connection.is_autocommit() {
            self.owner.clear_candidates();
            self.owner.retired.set(true);
            return Err(invalid("提交返回后事务仍活动"));
        }
        result?;
        if let Some(pending) = self.construction.borrow_mut().take() {
            // SQLite 已确认 COMMIT，才 move 到下一事务候选；失败绝不复原旧 facts。
            if let Err(error) = self.owner.construction_lease.close() {
                self.owner.invalidate_construction();
                return Err(error);
            }
            *self.owner.construction.borrow_mut() = Some(pending.committed());
        }
        Ok(Ended {
            life: self.owner.life,
            #[cfg(feature = "s_capture_native_testonly")]
            complete: self.complete.borrow_mut().take(),
        })
    }
}

impl Drop for AuditTxn<'_> {
    fn drop(&mut self) {
        if let Some(tx) = self.transaction.take() {
            #[cfg(feature = "s_capture_native_testonly")]
            {
                if tx
                    .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
                    .is_err()
                {
                    self.owner.retired.set(true);
                }
                effects::clear_node_journal(&tx);
            }
            self.owner.clear_candidates();
            self.owner.connection.flush_prepared_statement_cache();
            // 不依赖 rusqlite 的隐式 Drop 回滚；必须核实终结结果。
            if tx.rollback().is_err() || !self.owner.connection.is_autocommit() {
                self.owner.retired.set(true);
            }
        }
    }
}

fn invalid(error: impl std::fmt::Display) -> String {
    format!("S3DriverInvalid：{error}")
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) fn finish_fixture_source_keeper(
    connection: &Connection,
    context: request_audit::RequestAuditContext,
) -> Option<cross_request_source_cache::Keeper> {
    assert!(connection.is_autocommit(), "夹具事务必须已确认终结");
    context.finish_source_keeper(Ended {
        life: 0,
        complete: None,
    })
}

// 既有测试的原库断言继续使用原 SQL；生产构建不存在裸连接借用能力。
#[cfg(test)]
impl std::ops::Deref for AuditConnection {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}
#[cfg(test)]
impl std::ops::DerefMut for AuditConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::super::prepared::tests::Fixture;
    use super::*;

    #[test]
    fn deferred_snapshot_precedes_version_and_next_transaction_observes_normal_commit() {
        let mut fixture = Fixture::new();
        let (envelope, token) = fixture.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        fixture.publish(&envelope, &token).unwrap();
        let owner = AuditConnection::new(open_db(&fixture.db_path()).unwrap()).unwrap();
        let first = owner.begin(TransactionBehavior::Deferred).unwrap();
        let version = first.data_version;
        let generation: i64 = first
            .tx()
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM main.meta WHERE key='generation'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(generation, 1);

        // 第二连接只运行现成 Accept/Begin/Publish 正常 API，未修改或损坏库原件。
        let (envelope, token) = fixture.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
        fixture.publish(&envelope, &token).unwrap();
        let still_snapshot_version: i64 = first
            .tx()
            .query_row("PRAGMA main.data_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(still_snapshot_version, version);
        assert_eq!(meta_i64(first.tx(), "generation").unwrap(), 1);
        first.read_end().unwrap();
        assert!(owner.connection.is_autocommit());

        let second = owner.begin(TransactionBehavior::Deferred).unwrap();
        assert_ne!(second.data_version, version);
        assert_eq!(meta_i64(second.tx(), "generation").unwrap(), 2);
        second.read_end().unwrap();
        let reopened = AuditConnection::new(open_db(&fixture.db_path()).unwrap()).unwrap();
        assert_ne!(owner.life, reopened.life);
        assert!(reopened.connection.is_autocommit());
    }
}

impl AuditConnection {
    fn clear_complete(&self) {
        #[cfg(feature = "s_capture_native_testonly")]
        drop(self.complete.borrow_mut().take());
    }
    pub(super) fn discard_after_error(&self) {
        self.clear_candidates();
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn complete_retained_bytes(&self) -> usize {
        self.complete
            .borrow()
            .as_ref()
            .map_or(0, |c| c.charge.total)
            .saturating_add(
                self.construction
                    .borrow()
                    .as_ref()
                    .map_or(0, |c| c.retained_bytes()),
            )
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_report(&self) -> Value {
        self.complete.borrow().as_ref().map_or(Value::Null, |c| json!({
            "charge":c.charge,"life":c.seal.life,"transaction":c.seal.transaction_id,
            "write_sequence":c.seal.write_sequence,"tables":c.originals.len(),
            "nodes":c.node_count,"raw_rows":c.audited.raw.len(),"cuts":c.audited.cuts.len(),
            "raw_checked":c.audited.raw_prefixes.as_ref().map(|p|p.checked_len()),
            "published_prefix":c.audited.raw_prefixes.as_ref().map(|p|p.published_len()),
            "source_hash_rows":c.audited.raw_sources.as_ref().map(|p|p.hashed_rows()),
            "source_counts_preserved":c.slot.as_ref().is_some_and(|s|s.source_pending.is_some()),
            "capacity_method":"实际 Vec/String capacity 加树/散列表保守槽位上界；非RSS",
        }))
    }
}
impl AuditTxn<'_> {
    fn construction_seal(&self) -> Result<SnapshotSeal, String> {
        Ok(SnapshotSeal {
            life: self.owner.life,
            transaction_id: self.transaction_id,
            write_sequence: self.owner.write_sequence.get(),
            data_version: self
                .tx()
                .query_row("PRAGMA main.data_version", [], |r| r.get(0))
                .map_err(invalid)?,
            schema_version: self
                .tx()
                .query_row("PRAGMA main.schema_version", [], |r| r.get(0))
                .map_err(invalid)?,
            domain: self.read_domain()?,
        })
    }

    /// #1488：唯一生产构造点仍封闭在 owner；调用者不能传 raw、prefix 或成功布尔量。
    /// 此原型只授 raw 变化资格；集合完整性仍须独立 seal，根整合前不接正式 Runtime。
    #[cfg(feature = "s_capture_native_testonly")]
    fn with_bound_raw_advance<R>(
        &self,
        context: &request_audit::RequestAuditContext,
        consume: impl for<'raw> FnOnce(BoundRawAdvance<'raw>) -> Result<R, String>,
    ) -> Result<R, String> {
        let result = (|| {
            use super::audit_retention::sum;
            if self.dirty.get() || self.construction.borrow().is_some() {
                return Err(construction_state::stale("仅允许本事务首次写前签发一次"));
            }
            let seal = self.construction_seal()?;
            if seal.data_version != self.data_version || seal.schema_version != self.schema_version
            {
                return Err(construction_state::stale("原始读快照已变"));
            }
            let complete = self.complete.borrow();
            let candidate = complete
                .as_ref()
                .ok_or_else(|| construction_state::stale("缺当前同快照已核原件"))?;
            if candidate.seal != seal {
                return Err(construction_state::stale("完整原件不属于当前连接/快照"));
            }
            let previous = self.owner.construction.borrow_mut().take();
            let retained = sum([
                candidate.charge.total,
                previous.as_ref().map_or(0, |s| s.retained_bytes()),
                std::mem::size_of::<construction_state::PendingRawState>(),
                std::mem::size_of::<BoundRawAdvance<'_>>(),
            ]);
            let budget = native::Budget::new(context.complete_workspace_limit(), retained)
                .map_err(|_| construction_state::stale("raw增量工作额度不足"))?;
            let work = budget
                .codec_work()
                .map_err(|_| construction_state::stale("raw增量分配许可不足"))?;
            work.run(|scope| {
                let bound = BoundOriginals { _private: () };
                let (pending, token) = construction_state::PendingRawState::advance(
                    previous,
                    candidate.audited.raw.view(),
                    seal,
                    &bound,
                    &self.owner.construction_lease,
                    &super::audit_store::NativeAdmission(Some(scope)),
                )?;
                *self.construction.borrow_mut() = Some(pending);
                let value = consume(token)?;
                if self.owner.construction_lease.stale.get() {
                    return Err(construction_state::stale("消费者已撤销raw租约"));
                }
                Ok(value)
            })
            .map_err(|error| match native::Failure::from(error) {
                native::Failure::Invalid(e) => e,
                native::Failure::BudgetExceeded => construction_state::stale("raw增量分配额度不足"),
            })
        })();
        if result.is_err() {
            self.owner.invalidate_construction();
            drop(self.construction.borrow_mut().take());
        }
        result
    }

    fn before_write(&self) -> Result<(), String> {
        self.before_write_inner(false)
    }

    fn before_write_inner(&self, preserve_construction: bool) -> Result<(), String> {
        // 必须发生在任何固定 operation（含 Noop）的第一条写 SQL 之前。
        if !preserve_construction {
            self.owner.invalidate_construction();
            if self.construction.borrow_mut().take().is_some() {
                return Err(construction_state::stale("增量期间出现未声明写口"));
            }
        }
        self.owner.clear_optional_candidates();
        #[cfg(feature = "s_capture_native_testonly")]
        drop(self.complete.borrow_mut().take());
        let next = self
            .owner
            .write_sequence
            .get()
            .checked_add(1)
            .ok_or_else(|| invalid("写序耗尽"))?;
        self.owner.write_sequence.set(next);
        self.dirty.set(true);
        Ok(())
    }

    fn audit_admission(
        &self,
        envelope: &Value,
        mode: &s_allocation::Mode,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<(), String> {
        #[cfg(feature = "s_capture_native_testonly")]
        if matches!(mode, s_allocation::Mode::CapturedNative) && context.source_cache_enabled() {
            // 只对固定操作绑定完整候选；未知操作在执行前撤资格。
            // 后续原 inspect_ingest/lookup 仍逐字执行全部请求判据，错误绝不重判。
            let known = matches!(
                envelope["payload"]["op"].as_str(),
                Some("ready" | "receipt" | "ingest")
            );
            let domain = self.read_domain()?;
            let seal = SnapshotSeal {
                life: self.owner.life,
                transaction_id: self.transaction_id,
                write_sequence: self.write_sequence,
                data_version: self.data_version,
                schema_version: self.schema_version,
                domain,
            };
            let prior = self.owner.complete.borrow_mut().take();
            if let Some(mut candidate) = prior {
                if known && candidate.seal.same_snapshot(&seal) {
                    candidate.seal = seal;
                    candidate.charge.entry_reserved_bytes = context.complete_entry_reserved_bytes();
                    #[cfg(test)]
                    state::trace_event("owner_complete_hit", None);
                    *self.complete.borrow_mut() = Some(candidate);
                    return Ok(());
                }
                // 外部提交或未知操作在核验前决定冷分派并真释放；不是校验错误后的回退。
                drop(candidate);
            }
            let audited = self.audit_required_in_parent(mode, context)?;
            let parent = context.cold_parent_budget();
            if known {
                if let Some(slot) = context.take() {
                    match CompleteCandidate::capture(
                        self.tx(),
                        seal,
                        slot,
                        audited,
                        context.complete_workspace_limit(),
                        context.complete_entry_reserved_bytes(),
                        parent.as_ref(),
                    )? {
                        Ok(candidate) => {
                            context.note_complete_source_work(
                                candidate
                                    .slot
                                    .as_ref()
                                    .and_then(|s| s.source_pending.as_ref()),
                            );
                            *self.complete.borrow_mut() = Some(candidate);
                        }
                        Err(slot) => context.restore_slot(slot),
                    }
                }
            }
            return Ok(());
        }
        self.audit_required_in_parent(mode, context)?;
        Ok(())
    }

    fn audit_required_in_parent(
        &self,
        mode: &s_allocation::Mode,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<state::AuditedState, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(parent) = context.cold_parent_budget() {
            let failure = |e: native::Failure| match e {
                native::Failure::BudgetExceeded => {
                    "S3WriterResourcesInvalid：冷审计父预算不足".to_owned()
                }
                native::Failure::Invalid(e) => e,
            };
            return parent
                .codec_work()
                .map_err(failure)?
                .run(|work| {
                    s_allocation::audit_in_request_admitted(
                        self.tx(),
                        mode,
                        s_allocation::Phase::Handle,
                        context,
                        Some(work),
                    )
                })
                .map_err(|e| failure(native::Failure::from(e)));
        }
        s_allocation::audit_in_request(self.tx(), mode, s_allocation::Phase::Handle, context)
    }

    fn read_domain(&self) -> Result<[i64; 6], String> {
        // main 快照/version 已在 begin 中取得；TEMP 和连接设置不由 main version 覆盖。
        let temp: i64 = self
            .tx()
            .query_row("SELECT count(*) FROM temp.sqlite_schema", [], |r| r.get(0))
            .map_err(invalid)?;
        let databases: i64 = self
            .tx()
            .query_row(
                "SELECT count(*) FROM pragma_database_list WHERE name NOT IN ('main','temp')",
                [],
                |r| r.get(0),
            )
            .map_err(invalid)?;
        if temp != 0 || databases != 0 {
            return Err(invalid("完整候选不允许 TEMP 对象或附加数据库"));
        }
        let mut out = [0; 6];
        for (i, sql) in [
            "PRAGMA read_uncommitted",
            "PRAGMA foreign_keys",
            "PRAGMA recursive_triggers",
            "PRAGMA trusted_schema",
            "PRAGMA query_only",
            "PRAGMA main.encoding",
        ]
        .into_iter()
        .enumerate()
        {
            if i == 5 {
                let mut stmt = newchan_rust::sqlite_checked::CheckedStatement::prepare(
                    self.tx(),
                    c"PRAGMA main.encoding",
                )
                .map_err(invalid)?;
                let mut row = stmt
                    .next()
                    .map_err(invalid)?
                    .ok_or_else(|| invalid("编码域缺行"))?;
                out[i] = match row.text(0).map_err(invalid)? {
                    "UTF-8" => 1,
                    "UTF-16le" => 2,
                    "UTF-16be" => 3,
                    _ => return Err(invalid("未知编码域")),
                };
                drop(row);
                stmt.finalize().map_err(invalid)?;
            } else {
                out[i] = self
                    .tx()
                    .query_row(sql, [], |r| r.get(0))
                    .map_err(invalid)?;
            }
        }
        if out[0] != 0 {
            return Err(invalid("完整候选不允许 read_uncommitted"));
        }
        Ok(out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SnapshotSeal {
    life: u64,
    transaction_id: u64,
    write_sequence: u64,
    data_version: i64,
    schema_version: i64,
    domain: [i64; 6],
}
impl SnapshotSeal {
    fn same_snapshot(&self, next: &Self) -> bool {
        self.life != 0
            && self.life == next.life
            && self.transaction_id < next.transaction_id
            && self.write_sequence == next.write_sequence
            && self.data_version == next.data_version
            && self.schema_version == next.schema_version
            && self.domain == next.domain
    }
}
#[cfg(feature = "s_capture_native_testonly")]
#[derive(serde::Serialize)]
struct CompleteCharge {
    node_source_and_certificates: usize,
    node_capacity: usize,
    historical_and_raw_capacity: usize,
    historical_binding_capacity: usize,
    slot_result_capacity: usize,
    source_pool_charge: usize,
    current_sql_capacity: usize,
    q_capacity: usize,
    effects_peak: usize,
    incremental_steps: usize,
    inline: usize,
    total: usize,
    entry_reserved_bytes: usize,
    handoff_peak: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
struct CompleteCandidate {
    seal: SnapshotSeal,
    slot: Option<request_audit::VerifiedHistoricalSlot>,
    capture: Option<newchan_rust::s_content::AuditCapture>,
    node_count: usize,
    q: q::Facts,
    limit: usize,
    audited: state::AuditedState,
    // Q 全集：所有非节点表、schema/完整配置/clock/raw尾/控制关系，不是只有 H 的绑定。
    originals: Vec<request_audit::ExactRows>,
    charge: CompleteCharge,
}
#[cfg(feature = "s_capture_native_testonly")]
impl CompleteCandidate {
    fn capture(
        tx: &Transaction<'_>,
        seal: SnapshotSeal,
        mut slot: request_audit::VerifiedHistoricalSlot,
        audited: state::AuditedState,
        limit: usize,
        entry_reserved_bytes: usize,
        parent: Option<&native::Budget>,
    ) -> Result<Result<Self, request_audit::VerifiedHistoricalSlot>, String> {
        use super::audit_retention::{sum, Retained};
        // 丢掉事务 memo，仅原件和纯 codec 成功证书移动到候选；不复制节点正文。
        slot.capture = slot
            .capture
            .into_originals_and_codec_roots_for_testonly()
            .map_err(invalid)?;
        let retained = slot
            .capture
            .native_retention_numbers_for_testonly()
            .ok_or_else(|| invalid("完整候选缺少 Native 原件"))?;
        let mut charge = CompleteCharge {
            node_source_and_certificates: sum([
                retained.body_len,
                slot.node_count.saturating_mul(97),
                8 + newchan_rust::s_content::CONTENT_FORMAT.len() + 4 + 2 * (1 + 1 + 8),
                sum([retained.public_roots, retained.template_roots]).saturating_mul(32),
            ]),
            node_capacity: sum([
                retained.body_capacity,
                retained.index_payload.saturating_mul(2),
                retained.index_capacity.saturating_mul(2).saturating_add(16),
                // 包含散列表控制区保守余量，不从原97字节证书扣费。
                sum([retained.public_capacity, retained.template_capacity])
                    .saturating_mul(2 * 33)
                    .saturating_add(32),
            ]),
            // 两份归属都照收，Rc 实际共享不是减费依据。
            historical_and_raw_capacity: sum([audited.heap_bytes(), slot.result.heap_bytes()]),
            historical_binding_capacity: slot.binding.heap_bytes(),
            slot_result_capacity: slot.result.heap_bytes(),
            source_pool_charge: slot
                .source_pending
                .as_ref()
                .map_or(0, |p| p.full_retention_charge()),
            current_sql_capacity: 0,
            q_capacity: 0,
            effects_peak: 0,
            incremental_steps: 0,
            inline: std::mem::size_of::<Self>(),
            total: 0,
            entry_reserved_bytes,
            handoff_peak: 0,
        };
        charge.total = sum([
            charge.node_source_and_certificates,
            charge.node_capacity,
            charge.historical_and_raw_capacity,
            charge.historical_binding_capacity,
            charge.source_pool_charge,
            charge.inline,
        ]);
        if charge.total >= limit {
            return Ok(Err(slot));
        }
        if let Some(parent) = parent {
            let Ok(permit) = parent.reserve(std::mem::size_of::<Self>()) else {
                return Ok(Err(slot));
            };
            permit.commit(0, std::mem::size_of::<Self>());
        }
        let mut originals: Vec<request_audit::ExactRows> = Vec::new();
        let mut candidate_peak = charge.total;
        // 表名同样经有界原件读取；不先 collect 一份未登记的 String 列表。
        let Some((tables, tables_peak)) = request_audit::ExactRows::read_optional_bounded_in_parent(
            tx,
            "SELECT name FROM main.sqlite_schema WHERE type='table' AND name!='s_content_nodes' ORDER BY name",
            limit - charge.total,
            parent,
        )? else { return Ok(Err(slot)); };
        candidate_peak = candidate_peak.max(sum([charge.total, tables_peak]));
        let table_capacity = tables.rows.len().saturating_add(4);
        let before_reserve = sum([
            charge.total,
            tables.heap_bytes(),
            super::audit_retention::vec_bytes::<request_audit::ExactRows>(table_capacity),
        ]);
        if before_reserve >= limit {
            return Ok(Err(slot));
        }
        let outer_permit = match parent
            .map(|b| {
                b.reserve(
                    super::audit_retention::vec_bytes::<request_audit::ExactRows>(table_capacity),
                )
            })
            .transpose()
        {
            Ok(permit) => permit,
            Err(_) => return Ok(Err(slot)),
        };
        if originals.try_reserve_exact(table_capacity).is_err() {
            return Ok(Err(slot));
        }
        if let Some(permit) = outer_permit {
            permit.commit(
                0,
                super::audit_retention::vec_bytes::<request_audit::ExactRows>(originals.capacity()),
            );
        }
        candidate_peak = candidate_peak.max(before_reserve);
        let mut read_original = |sql: &str, sql_heap: usize| -> Result<bool, String> {
            let extra = sum([originals.heap_bytes(), tables.heap_bytes(), sql_heap]);
            let Some(remaining) = limit.checked_sub(charge.total.saturating_add(extra)) else {
                return Ok(false);
            };
            let Some((rows, row_peak)) = request_audit::ExactRows::read_optional_bounded_in_parent(
                tx, sql, remaining, parent,
            )?
            else {
                return Ok(false);
            };
            candidate_peak = candidate_peak.max(sum([charge.total, extra, row_peak]));
            originals.push(rows);
            Ok(true)
        };
        // 静态 SQL 是借用，不同时生成四个 String。
        for sql in [
            "SELECT rowid,* FROM main.sqlite_schema ORDER BY rowid",
            "PRAGMA function_list",
            "PRAGMA collation_list",
            "PRAGMA database_list",
        ] {
            if !read_original(sql, 0)? {
                return Ok(Err(slot));
            }
        }
        drop(read_original);
        for table in &tables.rows {
            let Some(request_audit::SqlCell::Text(bytes)) = table.first() else {
                return Err(invalid("表名原件类型不符"));
            };
            let name = std::str::from_utf8(bytes).map_err(invalid)?;
            const PREFIX: &str = "SELECT rowid,* FROM main.\"";
            const SUFFIX: &str = "\" ORDER BY rowid";
            let requested = sum([
                PREFIX.len(),
                name.len(),
                name.bytes().filter(|b| *b == b'"').count(),
                SUFFIX.len(),
            ]);
            let before_sql = sum([
                charge.total,
                originals.heap_bytes(),
                tables.heap_bytes(),
                requested,
            ]);
            if before_sql >= limit {
                return Ok(Err(slot));
            }
            let sql_permit = match parent.map(|b| b.reserve(requested)).transpose() {
                Ok(permit) => permit,
                Err(_) => return Ok(Err(slot)),
            };
            let mut sql = String::new();
            if sql.try_reserve_exact(requested).is_err() {
                return Ok(Err(slot));
            }
            sql.push_str(PREFIX);
            for c in name.chars() {
                if c == '"' {
                    sql.push('"');
                }
                sql.push(c);
            }
            sql.push_str(SUFFIX);
            if let Some(permit) = sql_permit {
                permit.commit(0, sql.capacity());
            }
            let extra = sum([originals.heap_bytes(), tables.heap_bytes(), sql.capacity()]);
            let Some(remaining) = limit.checked_sub(charge.total.saturating_add(extra)) else {
                return Ok(Err(slot));
            };
            let Some((rows, row_peak)) = request_audit::ExactRows::read_optional_bounded_in_parent(
                tx, &sql, remaining, parent,
            )?
            else {
                return Ok(Err(slot));
            };
            candidate_peak =
                candidate_peak
                    .max(before_sql)
                    .max(sum([charge.total, extra, row_peak]));
            originals.push(rows);
        }
        candidate_peak = candidate_peak.max(sum([
            charge.total,
            originals.heap_bytes(),
            tables.heap_bytes(),
        ]));
        if candidate_peak >= limit {
            return Ok(Err(slot));
        }
        drop(tables);
        charge.handoff_peak = entry_reserved_bytes.saturating_add(candidate_peak);
        charge.current_sql_capacity = originals.heap_bytes();
        charge.total = charge.total.saturating_add(charge.current_sql_capacity);
        if charge.total >= limit {
            return Ok(Err(slot));
        }
        let budget = match parent {
            Some(parent) => parent.clone(),
            None => match native::Budget::new(limit, charge.total) {
                Ok(budget) => budget,
                Err(_) => return Ok(Err(slot)),
            },
        };
        let q = match q::Facts::cold(tx, &originals, &audited, &budget) {
            Ok(q) => q,
            Err(native::Failure::BudgetExceeded) => return Ok(Err(slot)),
            Err(native::Failure::Invalid(e)) => return Err(e),
        };
        charge.q_capacity = q.heap_bytes();
        charge.total = charge.total.saturating_add(charge.q_capacity);
        charge.handoff_peak = charge
            .handoff_peak
            .max(entry_reserved_bytes.saturating_add(budget.peak()));
        #[cfg(test)]
        state::trace_event("owner_complete_cold_candidate", None);
        Ok(Ok(Self {
            seal,
            node_count: slot.node_count,
            slot: Some(slot),
            capture: None,
            q,
            limit,
            audited,
            originals,
            charge,
        }))
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
impl AuditConnection {
    pub(super) fn complete_pointers(&self) -> (usize, usize, usize) {
        let candidate = self.complete.borrow();
        let c = candidate.as_ref().unwrap();
        (
            c.audited.raw.plain().unwrap().as_ptr() as usize,
            (&*c.audited as *const state::ImmutableHistoricalAudit) as usize,
            c.originals.as_ptr() as usize,
        )
    }
    pub(super) fn assert_complete_matches_cold(&self) {
        fn view(a: &state::AuditedState) -> Value {
            json!({"raw":a.raw,"cuts":a.cuts.iter().map(|c|format!("{:?}|{:?}|{:?}|{:?}|{:?}",c.metadata,c.roots,c.batch_id,c.snapshot,c.delta)).collect::<Vec<_>>(),
                "objects":format!("{:?}",a.objects),"auxiliary":format!("{:?}",a.auxiliary)})
        }
        let active = self.begin(TransactionBehavior::Deferred).unwrap();
        let candidate = self.complete.borrow();
        let c = candidate.as_ref().unwrap();
        use super::audit_retention::Retained;
        assert_eq!(
            c.charge.current_sql_capacity,
            c.originals.heap_bytes(),
            "SQL增量账与全量oracle"
        );
        assert_eq!(c.charge.q_capacity, c.q.heap_bytes(), "Q增量账与全量oracle");
        assert_eq!(
            c.q.retained_bytes(),
            c.q.heap_bytes(),
            "Q缓存分项与全量oracle"
        );
        assert_eq!(
            c.charge.historical_and_raw_capacity,
            c.audited.heap_bytes() + c.charge.slot_result_capacity,
            "历史/raw增量账与全量oracle"
        );

        if let Some(sweep) = &c.audited.object_facts {
            sweep.assert_owner_projection_oracle(c.audited.cuts.len() as i64);
        }
        let cold = state::audit(active.tx()).unwrap();
        assert_eq!(view(&c.audited), view(&cold));
        for original in &c.originals {
            assert_eq!(
                *original,
                request_audit::ExactRows::read(active.tx(), &original.sql, [], vec![]).unwrap()
            );
        }
        drop(cold);
        drop(candidate);
        active.read_end().unwrap();
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn audit_owner_ended_cannot_promote_earlier_context_or_fixture_life() {
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let mut owner = AuditConnection::open(&f.db_path()).unwrap();
    let mut context = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(536_870_912),
    );
    context
        .freeze_manifest_workspace_budget(536_870_912, 0, 0)
        .unwrap();
    let early = owner.begin(TransactionBehavior::Deferred).unwrap();
    state::audit_native_in_request(early.tx(), &mut context).unwrap();
    early.read_end().unwrap();
    assert!(context.has_slot());
    let later = owner.begin(TransactionBehavior::Deferred).unwrap();
    let ended = later.read_end().unwrap();
    owner.finish_source_keeper(context, ended);
    assert_eq!(
        owner.complete_retained_bytes(),
        0,
        "后来的Ended不得签早先context"
    );
    assert!(
        owner
            .source_keeper
            .borrow()
            .as_ref()
            .unwrap()
            .retained_source_bytes()
            > 0
    );
    // fixture只授旧来源候选（下一次仍须完整逐tuple绑定），无完整owner生涯许可。
    let fixture_end = Ended {
        life: 0,
        complete: None,
    };
    let empty = request_audit::RequestAuditContext::default();
    owner.finish_source_keeper(empty, fixture_end);
    assert_eq!(owner.complete_retained_bytes(), 0);
}

#[cfg(feature = "s_capture_native_testonly")]
impl CompleteCandidate {
    fn detach_slot(&mut self) -> Result<(), String> {
        if let Some(slot) = self.slot.take() {
            self.capture = Some(slot.capture);
            // 真实释放历史绑定和旧来源候选；唯一 audited 继续拥有全 C 与事实。
            drop(slot.result);
            drop(slot.binding);
            drop(slot.source_pending);
            let released = self
                .charge
                .slot_result_capacity
                .checked_add(self.charge.historical_binding_capacity)
                .and_then(|n| n.checked_add(self.charge.source_pool_charge))
                .ok_or_else(|| invalid("slot 释放账不符"))?;
            self.charge.historical_and_raw_capacity = self
                .charge
                .historical_and_raw_capacity
                .checked_sub(self.charge.slot_result_capacity)
                .ok_or_else(|| invalid("slot 历史账下溢"))?;
            self.charge.total = self
                .charge
                .total
                .checked_sub(released)
                .ok_or_else(|| invalid("slot 总账下溢"))?;
            self.charge.slot_result_capacity = 0;
            self.charge.historical_binding_capacity = 0;
            self.charge.source_pool_charge = 0;
        }
        Ok(())
    }
    fn validate_effects(
        mut self,
        tx: &Transaction<'_>,
        mut effects: effects::Effects,
        limit: usize,
    ) -> Result<Option<Self>, String> {
        use super::audit_retention::sum;
        use effects::Table;
        self.detach_slot()?;
        let retained_at_entry = self.charge.total;
        // Recording 已结束；它的峰值与候选保留物共存，不与后续审计峰值相加。
        let Some(recording_peak) = retained_at_entry.checked_add(effects.peak) else {
            return Ok(None);
        };
        if recording_peak >= limit {
            return Ok(None);
        }
        let budget = match native::Budget::new(limit, sum([self.charge.total, effects.bytes])) {
            Ok(budget) => budget,
            Err(_) => return Ok(None),
        };
        let node_work = match budget.codec_work() {
            Ok(work) => work,
            Err(native::Failure::BudgetExceeded) => return Ok(None),
            Err(native::Failure::Invalid(e)) => return Err(e),
        };
        let capture = self.capture.as_mut().unwrap();
        let node_result = node_work.run(|scope| -> Result<(), String> {
            for change in effects
                .changes
                .iter_mut()
                .filter(|c| c.table == Table::Node)
            {
                let row = change.new.as_mut().ok_or_else(|| invalid("缺新增节点"))?;
                let digest = match &row[1] {
                    request_audit::SqlCell::Blob(b) => b.as_slice().try_into().map_err(invalid)?,
                    _ => return Err(invalid("digest 非 BLOB")),
                };
                let tag = u8::try_from(effects::integer(&row[2])?).map_err(invalid)?;
                let body = match std::mem::replace(&mut row[3], request_audit::SqlCell::Null) {
                    request_audit::SqlCell::Blob(b) => b,
                    _ => return Err(invalid("body 非 BLOB")),
                };
                let existed = capture.contains(&digest);
                if !capture
                    .append_original_for_owner_testonly(digest, tag, body, usize::MAX, Some(scope))
                    .map_err(invalid)?
                {
                    return Err(invalid("Native 节点许可未被消费"));
                }
                if !existed {
                    self.node_count += 1;
                }
            }
            Ok(())
        });
        match node_result.map_err(native::Failure::from) {
            Ok(()) => {}
            Err(native::Failure::BudgetExceeded) => return Ok(None),
            Err(native::Failure::Invalid(e)) => return Err(e),
        }
        match self.q.advance(tx, &effects, &mut self.audited, &budget) {
            Ok(raw_added) => {
                let Some(total) = self
                    .charge
                    .historical_and_raw_capacity
                    .checked_add(raw_added)
                else {
                    return Ok(None);
                };
                self.charge.historical_and_raw_capacity = total;
            }
            Err(native::Failure::BudgetExceeded) => return Ok(None),
            Err(native::Failure::Invalid(e)) => return Err(e),
        }
        let bound = BoundOriginals { _private: () };
        match self.audited.check_owner_raw(capture, &bound, &budget) {
            Ok(added) => {
                let Some(total) = self.charge.historical_and_raw_capacity.checked_add(added) else {
                    return Ok(None);
                };
                self.charge.historical_and_raw_capacity = total;
            }
            Err(native::Failure::BudgetExceeded) => return Ok(None),
            Err(native::Failure::Invalid(e)) => return Err(e),
        }
        if effects.operation == effects::Operation::Publish {
            // #1394：成功路径借用 SQL 文本，避免复制 meta 值及动态标签。
            let g = {
                let mut query = newchan_rust::sqlite_checked::CheckedStatement::prepare(
                    tx,
                    c"SELECT value FROM meta WHERE key=?1",
                )
                .map_err(invalid)?;
                query.bind_text(1, "generation").map_err(invalid)?;
                let mut row = query
                    .next()
                    .map_err(invalid)?
                    .ok_or_else(|| invalid("meta.generation 缺失"))?;
                let value = newchan_rust::session_protocol::parse_canonical_i64(
                    row.text(0).map_err(invalid)?,
                    "meta.generation",
                )
                .map_err(invalid)?;
                drop(row);
                query.finalize().map_err(invalid)?;
                value
            };
            let d = effects
                .rows(Table::Delta)
                .next()
                .unwrap()
                .new
                .as_ref()
                .unwrap();
            let batch = effects::text(&d[7])?;
            let commit = effects
                .rows(Table::Phase)
                .find(|c| {
                    c.new
                        .as_ref()
                        .is_some_and(|r| effects::text(&r[2]).ok() == Some("commit"))
                })
                .ok_or_else(|| invalid("缺 commit 实际效果"))?;
            let ns = effects::text(&commit.new.as_ref().unwrap()[4])?;
            let row_ids = |table| -> Result<Vec<i64>, native::BudgetExceeded> {
                let mut rows = Vec::new();
                budget.vec_grow(&mut rows, effects.rows(table).count())?;
                rows.extend(effects.rows(table).map(|change| change.rowid));
                Ok(rows)
            };
            let row_ids = (|| -> Result<_, native::BudgetExceeded> {
                Ok((
                    row_ids(Table::Object)?,
                    [
                        row_ids(Table::Witness)?,
                        row_ids(Table::Relation)?,
                        row_ids(Table::Observation)?,
                    ],
                ))
            })();
            let (objects, auxiliary) = match row_ids {
                Ok(rows) => rows,
                Err(_) => return Ok(None),
            };
            #[cfg(test)]
            eprintln!(
                "owner actual suffix g={g} nodes={} object_effects={} aux_effects={:?}",
                self.node_count,
                objects.len(),
                auxiliary.each_ref().map(Vec::len)
            );
            let work = match budget.codec_work() {
                Ok(work) => work,
                Err(native::Failure::BudgetExceeded) => return Ok(None),
                Err(native::Failure::Invalid(e)) => return Err(e),
            };
            // 整个新 cut 是一次独占操作域；所有 String 桥首次 Err 立即短路，不重判。
            let updated = work
                .run(|scope| {
                    self.audited.append_owner_cut(
                        tx, capture, g, batch, ns, &objects, &auxiliary, &bound, scope, &budget,
                    )
                })
                .map_err(native::Failure::from);
            #[cfg(test)]
            eprintln!(
                "NATIVE_CODEC_WORK live_peak_meta={:?} native_upper_live_peak={:?} total_peak={}",
                budget.scratch_observation(),
                budget.native_observation(),
                budget.peak()
            );
            let (audited, retained) = match updated {
                Ok(updated) => updated,
                Err(native::Failure::BudgetExceeded) => return Ok(None),
                Err(native::Failure::Invalid(e)) => return Err(e),
            };
            self.audited = audited;
            self.charge.historical_and_raw_capacity =
                match retained.apply(self.charge.historical_and_raw_capacity) {
                    Ok(total) => total,
                    Err(native::Failure::BudgetExceeded) => return Ok(None),
                    Err(native::Failure::Invalid(error)) => return Err(error),
                };
        }
        let effect_rows = effects.changes.len();
        let peak = match effects.apply(
            &mut self.originals,
            &budget,
            &mut self.charge.current_sql_capacity,
        ) {
            Ok(peak) => peak,
            Err(native::Failure::BudgetExceeded) => return Ok(None),
            Err(native::Failure::Invalid(e)) => return Err(e),
        };
        self.capture = Some(
            self.capture
                .take()
                .unwrap()
                .finish_owner_step_for_testonly()
                .map_err(invalid)?,
        );
        let retained = self
            .capture
            .as_ref()
            .unwrap()
            .native_retention_numbers_for_testonly()
            .ok_or_else(|| invalid("缺 Native 原件费用"))?;
        self.charge.node_source_and_certificates = sum([
            retained.body_len,
            self.node_count.saturating_mul(97),
            8 + newchan_rust::s_content::CONTENT_FORMAT.len() + 4 + 2 * (1 + 1 + 8),
            sum([retained.public_roots, retained.template_roots]).saturating_mul(32),
        ]);
        self.charge.node_capacity = sum([
            retained.body_capacity,
            retained.index_payload.saturating_mul(2),
            retained.index_capacity.saturating_mul(2).saturating_add(16),
            sum([retained.public_capacity, retained.template_capacity])
                .saturating_mul(66)
                .saturating_add(32),
        ]);
        self.charge.historical_binding_capacity = 0;
        self.charge.source_pool_charge = 0;
        self.charge.q_capacity = self.q.retained_bytes();
        self.charge.total = sum([
            self.charge.node_source_and_certificates,
            self.charge.node_capacity,
            self.charge.historical_and_raw_capacity,
            self.charge.current_sql_capacity,
            self.charge.q_capacity,
            self.charge.inline,
        ]);
        self.charge.effects_peak = self.charge.effects_peak.max(peak);
        self.charge.handoff_peak = self.charge.handoff_peak.max(sum([
            self.charge.entry_reserved_bytes,
            budget.peak().max(recording_peak),
        ]));
        if self.charge.total >= limit {
            return Ok(None);
        }
        self.charge.incremental_steps += 1;
        #[cfg(test)]
        state::trace_owner_effects(
            effect_rows,
            peak,
            self.audited.raw.len(),
            self.audited.cuts.len(),
        );
        Ok(Some(self))
    }
}

#[derive(Clone, Copy)]
enum FixedOperation {
    Accept,
    Begin,
    Publish,
    Recover,
}
impl AuditConnection {
    /// 同一请求的 Accept/Begin 已提交结果在下一事务重新绑定，不刷新版本号。
    pub(super) fn continue_with(&self, ended: Ended) -> Result<(), String> {
        if ended.life != self.life || self.retired.get() || !self.connection.is_autocommit() {
            return Err(invalid("事务结果不属于当前连接生涯"));
        }
        #[cfg(feature = "s_capture_native_testonly")]
        {
            *self.complete.borrow_mut() = ended.complete;
        }
        Ok(())
    }
}
impl AuditTxn<'_> {
    fn bind_existing(
        &self,
        context: Option<&request_audit::RequestAuditContext>,
    ) -> Result<bool, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        {
            let Some(mut candidate) = self.owner.complete.borrow_mut().take() else {
                return Ok(false);
            };
            let seal = SnapshotSeal {
                life: self.owner.life,
                transaction_id: self.transaction_id,
                write_sequence: self.write_sequence,
                data_version: self.data_version,
                schema_version: self.schema_version,
                domain: self.read_domain()?,
            };
            if !candidate.seal.same_snapshot(&seal) {
                return Ok(false);
            }
            if let Some(context) = context {
                if !context.source_cache_enabled() {
                    return Ok(false);
                }
                candidate.charge.entry_reserved_bytes = context.complete_entry_reserved_bytes();
            }
            candidate.seal = seal;
            #[cfg(test)]
            state::trace_event("owner_complete_hit", None);
            *self.complete.borrow_mut() = Some(candidate);
            return Ok(true);
        }
        #[cfg(not(feature = "s_capture_native_testonly"))]
        {
            let _ = context;
            Ok(false)
        }
    }
    fn bound_raw_len(&self) -> Result<usize, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        {
            return self
                .complete
                .borrow()
                .as_ref()
                .map(|c| c.audited.raw.len())
                .ok_or_else(|| invalid("缺当前绑定原始账"));
        }
        #[cfg(not(feature = "s_capture_native_testonly"))]
        Err(invalid("非 Native 无 owner 原始账"))
    }
    fn before_fixed_write(
        &self,
        operation: FixedOperation,
        context: Option<&request_audit::RequestAuditContext>,
    ) -> Result<(), String> {
        let result = self.before_fixed_write_inner(operation, context);
        if result.is_err() && self.construction.borrow().is_some() {
            self.owner.invalidate_construction();
        }
        result
    }
    fn before_fixed_write_inner(
        &self,
        operation: FixedOperation,
        context: Option<&request_audit::RequestAuditContext>,
    ) -> Result<(), String> {
        let strict = self.construction.borrow().is_some();
        if strict && self.owner.construction_lease.stale.get() {
            return Err(construction_state::stale("固定写前租约已撤销"));
        }
        #[cfg(feature = "s_capture_native_testonly")]
        {
            let candidate = self.complete.borrow_mut().take();
            if let Some(mut candidate) = candidate {
                let limit = context
                    .map(|c| c.complete_workspace_limit())
                    .unwrap_or(candidate.limit.saturating_sub(candidate.charge.total));
                let raw_bytes = self
                    .construction
                    .borrow()
                    .as_ref()
                    .map_or(0, |s| s.retained_bytes());
                let limit = limit.checked_sub(raw_bytes).unwrap_or(0);
                let op = match operation {
                    FixedOperation::Accept => effects::Operation::Accept,
                    FixedOperation::Begin => effects::Operation::Begin,
                    FixedOperation::Publish => effects::Operation::Publish,
                    FixedOperation::Recover => effects::Operation::Recover,
                };
                candidate.detach_slot()?;
                if let Some(recording_cap) = limit.checked_sub(candidate.charge.total) {
                    if let Some(recording) = effects::Recording::start(
                        self.tx(),
                        op,
                        (limit / 8).min(recording_cap),
                        strict,
                    )? {
                        if strict && matches!(operation, FixedOperation::Publish) {
                            let previous = candidate
                                .audited
                                .cuts
                                .last()
                                .map(|cut| {
                                    construction_set::Declaration::read(
                                        cut.roots.generated_state.view(),
                                    )
                                })
                                .transpose()?;
                            let seed = self
                                .construction
                                .borrow_mut()
                                .as_mut()
                                .unwrap()
                                .set_seed(previous)?;
                            recording.bind_set(seed)?;
                        }
                        self.effect_limit.set(limit);
                        *self.pending.borrow_mut() = Some(candidate);
                        *self.recording.borrow_mut() = Some(recording);
                    }
                }
            }
            if strict && self.recording.borrow().is_none() {
                self.owner.invalidate_construction();
                return Err(construction_state::stale(
                    "固定写缺完整effects记录额度/原件",
                ));
            }
        }
        #[cfg(not(feature = "s_capture_native_testonly"))]
        {
            let _ = (operation, context);
            if strict {
                return Err(construction_state::stale("未启用真实Native effects能力"));
            }
        }
        self.before_write_inner(strict)
    }
    fn finish_fixed_write(&self) -> Result<(), String> {
        let result = self.finish_fixed_write_inner();
        if result.is_err() && self.construction.borrow().is_some() {
            self.owner.invalidate_construction();
        }
        result
    }
    fn finish_fixed_write_inner(&self) -> Result<(), String> {
        #[cfg(feature = "s_capture_native_testonly")]
        {
            let strict = self.construction.borrow().is_some();
            let missing = || {
                if strict {
                    Err(construction_state::stale(
                        "effects未完整闭合；不得回落旧审计",
                    ))
                } else {
                    Ok(())
                }
            };
            let Some(recording) = self.recording.borrow_mut().take() else {
                return missing();
            };
            let mut candidate = self
                .pending
                .borrow_mut()
                .take()
                .ok_or_else(|| invalid("效果日志缺候选"))?;
            let limit = self.effect_limit.get();
            let recording_cap = limit
                .checked_sub(candidate.charge.total)
                .ok_or_else(|| invalid("效果日志与候选保留费超过工作额度"))?;
            let Some(mut effects) =
                recording.finish(self.tx(), &mut candidate.originals, recording_cap)?
            else {
                return missing();
            };
            let closed_changes = self.tx().total_changes();
            if strict && !effects.has_exact_coverage(closed_changes) {
                return Err(construction_state::stale("actual effects覆盖收据缺失"));
            }
            let set = effects.set.take();
            // 真实旧投影核仍负责 birth/revision/withdrawal/辅助合并的双向语义；
            // C2 不按新叶数制造一条新的SQL判据。
            let set_bytes = set.as_ref().map_or(0, |set| set.retained_bytes());
            let Some(validation_limit) = limit.checked_sub(set_bytes) else {
                return missing();
            };
            effects.bytes = effects
                .bytes
                .checked_sub(set_bytes)
                .ok_or_else(|| invalid("集合保留账不符"))?;
            let Some(mut candidate) =
                candidate.validate_effects(self.tx(), effects, validation_limit)?
            else {
                return missing();
            };
            candidate.seal.write_sequence = self.owner.write_sequence.get();
            if let Some(set) = set {
                let current = candidate
                    .audited
                    .cuts
                    .last()
                    .ok_or_else(|| construction_state::stale("集合闭合缺实际已核发布根"))?;
                self.construction
                    .borrow_mut()
                    .as_mut()
                    .ok_or_else(|| construction_state::stale("集合effects丢失raw owner"))?
                    .close_set(set, current.roots.generated_state.view())?;
            }
            if strict {
                self.construction
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .close_effects(self.construction_seal()?, closed_changes)?;
            }
            *self.complete.borrow_mut() = Some(candidate);
        }
        Ok(())
    }
}

#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn note_persisted_node(
    tx: &Transaction<'_>,
    digest: [u8; 32],
    before: u64,
    inserted: bool,
) {
    effects::note_node(tx, digest, before, inserted);
}

/// C2材料切面：只在原完整来源审计成功后调用，生成器不能传旧根或变化集。
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn note_qualified_source_cut(
    tx: &Transaction<'_>,
    source: &mut (impl super::audit_store::CodecRead + ?Sized),
    root: newchan_rust::s_content::RefView<'_>,
) -> Result<(), String> {
    effects::note_source_cut(tx, source, root)
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn begin_construction_stage(
    tx: &Transaction<'_>,
    kind: StageKind,
) -> Result<effects::StageGuard, String> {
    effects::begin_stage(tx, kind)
}

/// 正常行为锁的实际 SQL 返回行/字节观察器；不计估算的底层 Btree/page I/O。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) struct SqlReadProbe(*mut rusqlite::ffi::sqlite3);
#[cfg(all(test, feature = "s_capture_native_testonly"))]
impl Drop for SqlReadProbe {
    fn drop(&mut self) {
        unsafe {
            rusqlite::ffi::sqlite3_trace_v2(self.0, 0, None, std::ptr::null_mut());
        }
    }
}
#[cfg(all(test, feature = "s_capture_native_testonly"))]
impl AuditConnection {
    pub(super) fn sql_read_probe(&self) -> SqlReadProbe {
        unsafe extern "C" fn observe(
            _: u32,
            _: *mut std::ffi::c_void,
            p: *mut std::ffi::c_void,
            _: *mut std::ffi::c_void,
        ) -> std::ffi::c_int {
            use rusqlite::ffi;
            // SQLite 在 SQLITE_TRACE_ROW 回调期间持有活 stmt；只借用当前结果列。
            let stmt = p.cast::<ffi::sqlite3_stmt>();
            let sql = unsafe { ffi::sqlite3_sql(stmt) };
            if sql.is_null() {
                return 0;
            }
            let sql = unsafe { std::ffi::CStr::from_ptr(sql) }.to_string_lossy();
            let class = if sql.contains("raw_events") || sql.contains("raw_ohlc") {
                0
            } else if [
                "s_input_messages",
                "s_clock_events",
                "s_protocol_meta",
                "writer_epoch_history",
                "s_delivery",
            ]
            .iter()
            .any(|t| sql.contains(t))
            {
                1
            } else if sql.contains("s_content_nodes") {
                3
            } else {
                2
            };
            let mut bytes = 0usize;
            for col in 0..unsafe { ffi::sqlite3_column_count(stmt) } {
                bytes =
                    bytes.saturating_add(match unsafe { ffi::sqlite3_column_type(stmt, col) } {
                        ffi::SQLITE_TEXT | ffi::SQLITE_BLOB => {
                            unsafe { ffi::sqlite3_column_bytes(stmt, col) }.max(0) as usize
                        }
                        ffi::SQLITE_INTEGER | ffi::SQLITE_FLOAT => 8,
                        _ => 0,
                    });
            }
            state::trace_sql_row(class, bytes);
            0
        }
        let db = unsafe { self.connection.handle() };
        let rc = unsafe {
            rusqlite::ffi::sqlite3_trace_v2(
                db,
                rusqlite::ffi::SQLITE_TRACE_ROW as u32,
                Some(observe),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
        SqlReadProbe(db)
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn audit_owner_q_itemized_normal_cold_growth_and_typed_limit() {
    use super::audit_retention::Retained;
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("q-budget", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let owner = AuditConnection::open(&f.db_path()).unwrap();
    let mut context = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(536_870_912),
    );
    context
        .freeze_manifest_workspace_budget(536_870_912, 0, 0)
        .unwrap();
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    active
        .audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut context,
        )
        .unwrap();
    let mut c = active.complete.borrow_mut().take().unwrap();
    let old_gate = c
        .originals
        .heap_bytes()
        .saturating_mul(128)
        .saturating_add(4096);
    let probe = native::Budget::new(536_870_912, 0).unwrap();
    let memory = allocation_probe::Window::start();
    let facts = q::Facts::cold(active.tx(), &c.originals, &c.audited, &probe).unwrap();
    let layout = memory.finish();
    let peak = probe.peak();
    // 本账读数（allocation_probe 按线程记账）；live 可为负，按 i64 求窗口内增长。
    let layout_peak =
        layout["coexistence_peak"].as_i64().unwrap() - layout["live_before"].as_i64().unwrap();
    assert!(
        layout_peak <= i64::try_from(peak).unwrap(),
        "正常小样本实测不能超过其推导预留；实测本身不授准入"
    );
    assert!(peak < old_gate, "正常Q逐容器上界须低于旧整表128倍门");
    let bounded = native::Budget::new(peak, 0).unwrap();
    let actual = q::Facts::cold(active.tx(), &c.originals, &c.audited, &bounded).unwrap();
    assert_eq!(actual.retained_bytes(), facts.heap_bytes());
    assert_eq!(actual.heap_bytes(), facts.heap_bytes());
    let tight = native::Budget::new(128, 0).unwrap();
    let before = tight.live();
    assert!(matches!(
        q::Facts::cold(active.tx(), &c.originals, &c.audited, &tight),
        Err(native::Failure::BudgetExceeded)
    ));
    assert_eq!(
        tight.live(),
        before,
        "拒绝clock构造前未产生未归还的临时许可"
    );
    let candidate_cap = c.charge.handoff_peak.max(c.charge.total).saturating_add(1);
    let old_candidate_gate = c.charge.total - c.charge.q_capacity + old_gate;
    assert!(candidate_cap < old_candidate_gate);
    let held = c.charge.total;
    let actual_release = c.charge.slot_result_capacity
        + c.charge.historical_binding_capacity
        + c.charge.source_pool_charge;
    assert!(actual_release > 0);
    c.detach_slot().unwrap();
    assert_eq!(c.charge.total, held - actual_release);
    assert!(c.slot.is_none() && c.capture.is_some());
    assert_eq!(c.charge.source_pool_charge, 0, "真正drop后释放source归属");
    eprintln!(
        "{}",
        json!({"scenario":"r5-q-itemized-normal","old_128_gate":old_gate,"reserved_peak":peak,"q_retained":actual.retained_bytes(),"slot_released":actual_release,"layout_observation":layout,"rust_layout":{"value":std::mem::size_of::<Value>(),"string":std::mem::size_of::<String>(),"hash_pair":std::mem::size_of::<(String,(i64,i64))>()}})
    );
    drop(c);
    active.read_end().unwrap();
    drop(context);
    let mut context = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(candidate_cap),
    );
    context
        .freeze_manifest_workspace_budget(candidate_cap, 0, 0)
        .unwrap();
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    active
        .audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut context,
        )
        .unwrap();
    assert!(
        active.complete.borrow().is_some(),
        "合法收紧额度保留完整候选；旧128倍门会提前丢弃"
    );
    let ended = active.read_end().unwrap();
    owner.continue_with(ended).unwrap();
    owner.assert_complete_matches_cold();
    eprintln!(
        "{}",
        json!({"scenario":"r5-complete-cold-itemized-budget","candidate_cap":candidate_cap,"old_gate_required":old_candidate_gate,"retention":owner.complete_report()})
    );
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn r13_cold_candidate_and_q_share_parent_and_preserve_complete_output() {
    use super::audit_retention::Retained;
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("r13-parent", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let mut owner = AuditConnection::open(&f.db_path()).unwrap();
    let total = 64 * 1024 * 1024;
    let mut context = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(total),
    );
    context
        .freeze_manifest_workspace_budget(total, 0, 0)
        .unwrap();
    assert!(
        context.cold_parent_budget().is_none(),
        "冻结来源额度本身不启用强制cold准入"
    );
    context.enable_cold_admission_for_testonly().unwrap();
    let parent = context.cold_parent_budget().unwrap();
    let start = parent.live();
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    active
        .audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut context,
        )
        .unwrap();
    let complete = active.complete.borrow();
    let candidate = complete.as_ref().expect("正常额度应持有完整候选");
    assert!(candidate.q.heap_bytes() > 0);
    assert!(
        parent.native_observation().0 > 0,
        "最早cold实际消费同父scope"
    );
    assert!(
        parent.live() > start + parent.native_observation().0,
        "cold后SQL及Q直接许可仍留同账"
    );
    assert_eq!(candidate.charge.handoff_peak, parent.peak());
    assert!(parent.peak() < total);
    for rows in &candidate.originals {
        assert_eq!(
            *rows,
            request_audit::ExactRows::read(active.tx(), &rows.sql, [], vec![]).unwrap()
        );
    }
    let q_bytes = candidate.q.heap_bytes();
    let source_counts = candidate
        .slot
        .as_ref()
        .unwrap()
        .source_pending
        .as_ref()
        .unwrap()
        .work_counts();
    assert_eq!(source_counts, (0, 1));
    drop(complete);
    let before_end = parent.live();
    let ended = active.read_end().unwrap();
    owner.finish_source_keeper(context, ended);
    assert!(owner.complete_retained_bytes() > 0);
    assert_eq!(
        parent.live(),
        before_end,
        "测试观察句柄仍持父账；context退场不归还Native保守费"
    );
    owner.assert_complete_matches_cold();
    let old_pool = owner.complete_retained_bytes();
    let mut next = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(total),
    );
    next.freeze_manifest_workspace_budget(total, 0, old_pool)
        .unwrap();
    next.enable_cold_admission_for_testonly().unwrap();
    let next_parent = next.cold_parent_budget().unwrap();
    let next_start = next_parent.live();
    assert_eq!(
        next_start, start,
        "已从allowance扣除旧池，不在新父账再次seed"
    );
    let hit = owner.begin(TransactionBehavior::Deferred).unwrap();
    hit.audit_admission(
        &json!({"payload":{"op":"ready"}}),
        &s_allocation::Mode::CapturedNative,
        &mut next,
    )
    .unwrap();
    assert!(hit.complete.borrow().is_some());
    assert_eq!(
        next_parent.native_observation().0,
        0,
        "同owner快照命中无需重建cold"
    );
    assert_eq!(next_parent.live(), next_start);
    let ended = hit.read_end().unwrap();
    owner.finish_source_keeper(next, ended);
    owner.assert_complete_matches_cold();
    eprintln!(
        "R13_COLD_PARENT {}",
        json!({"limit_bytes":total,"peak_bytes":parent.peak(),"cold_native_retained_bytes":parent.native_observation().0,"q_bytes":q_bytes,"complete_output_equal":true,"same_snapshot_hit":true,"old_pool_bytes":old_pool,"whole_heap_qualified":false})
    );
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn r13_optional_sql_rows_share_parent_without_changing_cells() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE exact(a,b); INSERT INTO exact VALUES(NULL,1),(1.5,'abc'),(x'00ff','中'),('01',NULL),(7,x'')").unwrap();
    let tx = conn.transaction().unwrap();
    let sql = "SELECT rowid,a,b FROM exact ORDER BY rowid";
    let expected = request_audit::ExactRows::read_optional_bounded(&tx, sql, 1024 * 1024)
        .unwrap()
        .unwrap();
    let parent = native::Budget::new(1024 * 1024, 0).unwrap();
    let before = parent.live();
    let actual = request_audit::ExactRows::read_optional_bounded_in_parent(
        &tx,
        sql,
        1024 * 1024,
        Some(&parent),
    )
    .unwrap()
    .unwrap();
    assert_eq!(actual, expected);
    assert!(parent.live() > before);
    assert!(parent.peak() < 1024 * 1024);
    let retained = parent.live();
    drop(actual);
    assert_eq!(parent.live(), retained);
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn r14_source_keeper_seal_and_next_request_share_explicit_parent() {
    fn complete(a: &state::AuditedState) -> Value {
        let cuts = a
            .cuts
            .iter()
            .map(|cut| {
                format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    cut.metadata, cut.roots, cut.batch_id, cut.snapshot, cut.delta
                )
            })
            .collect::<Vec<_>>();
        json!({"raw":a.raw,"cuts":cuts,"objects":format!("{:?}",a.objects),"auxiliary":format!("{:?}",a.auxiliary)})
    }
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("r14-source", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let total = 64 * 1024 * 1024;
    let mut ordinary_source_cost = 0;
    for admitted in [false, true] {
        let mut owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut keeper = cross_request_source_cache::Keeper::new(total);
        for request in 0..2 {
            let old_pool = keeper.retained_source_bytes();
            let mut context = request_audit::RequestAuditContext::with_source_keeper(keeper);
            context
                .freeze_manifest_workspace_budget(total, 0, old_pool)
                .unwrap();
            if admitted {
                context.enable_cold_admission_for_testonly().unwrap();
            }
            let parent = context.cold_parent_budget();
            let active = owner.begin(TransactionBehavior::Deferred).unwrap();
            let audited = if let Some(parent) = &parent {
                parent
                    .codec_work()
                    .unwrap()
                    .run(|scope| {
                        state::audit_native_in_request_admitted(
                            active.tx(),
                            &mut context,
                            Some(scope),
                        )
                    })
                    .unwrap()
            } else {
                state::audit_native_in_request(active.tx(), &mut context).unwrap()
            };
            let cold = state::audit_native_current(active.tx()).unwrap();
            assert_eq!(complete(&audited), complete(&cold));
            let before_seal = parent.as_ref().map(|p| p.live());
            drop(audited);
            drop(cold);
            let ended = active.read_end().unwrap();
            owner.finish_source_keeper(context, ended);
            keeper = owner.take_source_keeper().unwrap();
            let report = keeper.report();
            assert!(keeper.retained_source_bytes() > 0);
            assert_eq!(report["retained_counts_proofs"], 1);
            assert_eq!(report["rejected"], 0);
            assert_eq!(report["cold"], 1);
            assert_eq!(report["hits"], request);
            assert_eq!(
                owner.complete_retained_bytes(),
                0,
                "来源证书不签发完整候选资格"
            );
            if admitted {
                assert_eq!(keeper.retained_source_bytes(), ordinary_source_cost);
            } else {
                ordinary_source_cost = keeper.retained_source_bytes();
            }
            if let Some(parent) = &parent {
                assert!(
                    parent.live() > before_seal.unwrap(),
                    "封存新增许可必须留在冷核同父账"
                );
                assert!(parent.peak() < total - old_pool);
                assert_eq!(parent.scratch_observation().0, 0);
                assert_eq!(parent.scratch_observation().2, 0);
                eprintln!(
                    "R14_SOURCE_KEEPER {}",
                    json!({
                        "request":request,"limit_bytes":total-old_pool,"old_pool_bytes":old_pool,
                        "before_seal_bytes":before_seal,"after_seal_bytes":parent.live(),
                        "peak_bytes":parent.peak(),"source_cost_bytes":keeper.retained_source_bytes(),
                        "full_output_equal":true,"whole_heap_qualified":false
                    })
                );
            }
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod construction_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct RawObservation {
        previous: RawAdvanceBinding,
        current: RawAdvanceBinding,
        previous_prefix: usize,
        current_prefix: usize,
        edits: Vec<RawWinnerEdit>,
    }
    impl RawObservation {
        fn read(token: &BoundRawAdvance<'_>) -> Result<Self, String> {
            token.ensure_current()?;
            Ok(Self {
                previous: token.previous_binding(),
                current: token.current_binding(),
                previous_prefix: token.previous_prefix(),
                current_prefix: token.current_prefix(),
                edits: token.winner_edits().to_vec(),
            })
        }
    }
    struct AcceptProbe {
        life: u64,
        observations: Vec<RawObservation>,
    }
    thread_local! {
        static ACCEPT_PROBE: std::cell::RefCell<Option<AcceptProbe>> = const { std::cell::RefCell::new(None) };
    }
    struct AcceptProbeGuard;
    impl AcceptProbeGuard {
        fn start(owner: &AuditConnection) -> Self {
            ACCEPT_PROBE.with(|slot| {
                let mut slot = slot.borrow_mut();
                assert!(slot.is_none(), "测试观察器不能嵌套");
                *slot = Some(AcceptProbe {
                    life: owner.life,
                    observations: Vec::new(),
                });
            });
            Self
        }
        fn only_observation(&self) -> RawObservation {
            ACCEPT_PROBE.with(|slot| {
                let slot = slot.borrow();
                let observations = &slot.as_ref().unwrap().observations;
                assert_eq!(observations.len(), 1, "正式Accept只签一次写前资格");
                observations[0].clone()
            })
        }
    }
    impl Drop for AcceptProbeGuard {
        fn drop(&mut self) {
            ACCEPT_PROBE.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }
    /// 只为本线程、指定owner启用；通过原有封闭入口签发，不允许测试提供raw或成功token。
    pub(super) fn select_for_formal_accept(
        active: &AuditTxn<'_>,
        context: &request_audit::RequestAuditContext,
    ) -> Result<(), String> {
        let enabled = ACCEPT_PROBE.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|probe| probe.life == active.owner.life)
        });
        if !enabled {
            return Ok(());
        }
        let observation =
            active.with_bound_raw_advance(context, |token| RawObservation::read(&token))?;
        ACCEPT_PROBE.with(|slot| {
            slot.borrow_mut()
                .as_mut()
                .unwrap()
                .observations
                .push(observation);
        });
        Ok(())
    }

    fn formal_input(revision: bool) -> Value {
        use crate::tests::located_ohlc_input;
        let mut raw = if revision {
            located_ohlc_input(&[("owner-raw", 0, 6, 13, 3, 7)])
        } else {
            located_ohlc_input(&[("owner-next", 10, 8, 16, 6, 12)])
        };
        if revision {
            raw["events"][0]["revision"] = json!("2");
        }
        let payload = json!({"op":"ingest","target_session_id":"s-session-testonly-001",
            "target_session_generation":"1","writer_epoch":"1","clock_event_id":"input-2","raw_input":raw});
        json!({"schema_revision":"s-session/3","session_id":"s-session-testonly-001",
            "session_generation":"1","source_namespace":"testonly.ohlc","source_epoch":"1",
            "message_id":"message-2","producer_id":"real-content-test","producer_epoch":"1",
            "payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})
    }

    fn formal_accept_case(revision: bool, reject_consumer: bool) {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let probe = AcceptProbeGuard::start(&owner);
        let mut c = context();
        let clock = v2::stored_clock(&f.conn).unwrap();
        let (admission, ended) = owner
            .admit(
                formal_input(revision),
                "1",
                &clock,
                tb02a::PROFILE,
                Some("1"),
                &s_allocation::Mode::CapturedNative,
                &mut c,
            )
            .unwrap();
        assert!(
            matches!(admission, Admission::Advance { accepted: true, .. }),
            "必须实际接纳，不能用replay/已发布回执冒充新增raw"
        );
        assert!(owner.connection.is_autocommit(), "admit已完成真实COMMIT");
        // 在另一连接读到新行，不能把本事务内可见当成已提交。
        let rows: Vec<(String, i64, i64, String, Option<i64>)> = f.conn
            .prepare("SELECT identity_key,revision,seq,source_coord,supersedes_revision FROM raw_events ORDER BY seq").unwrap()
            .query_map([], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap()
            .collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].1, rows[0].2, rows[0].3.as_str()), (1, 0, "0"));
        assert_eq!(rows[1].2, 1, "真实新raw尾ordinal");
        if revision {
            assert_eq!(rows[1].0, rows[0].0, "同一业务身份");
            assert_eq!(
                (rows[1].1, rows[1].3.as_str(), rows[1].4),
                (2, "0", Some(1))
            );
        } else {
            assert_ne!(rows[1].0, rows[0].0);
            assert_eq!((rows[1].1, rows[1].3.as_str(), rows[1].4), (1, "10", None));
        }
        let status: String = f
            .conn
            .query_row(
                "SELECT status FROM s_input_messages WHERE message_id='message-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "AcceptedPending", "正式消息控制账一同提交");
        let ohlc_rows: i64 = f
            .conn
            .query_row("SELECT COUNT(*) FROM raw_ohlc", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ohlc_rows, 2, "正式OHLC原件一同提交");
        let before_accept = probe.only_observation();
        assert!(before_accept.previous.is_initial());
        assert_eq!(
            (before_accept.previous_prefix, before_accept.current_prefix),
            (0, 1)
        );
        assert_eq!(before_accept.edits.len(), 1);
        assert!(
            owner.construction.borrow().is_some(),
            "真实COMMIT后才持有旧前缀facts"
        );
        owner.continue_with(ended).unwrap();
        let next = owner.begin(TransactionBehavior::Immediate).unwrap();
        assert!(
            next.bind_existing(Some(&c)).unwrap(),
            "必须消费Accept后真实effects产生的CompleteCandidate，禁止重新手造原件"
        );
        let expected = RawWinnerEdit {
            old: revision.then_some(RawWinner {
                coord: 0,
                ordinal: 0,
            }),
            new: Some(RawWinner {
                coord: if revision { 0 } else { 10 },
                ordinal: 1,
            }),
        };
        let mut observed = None;
        let result = next.with_bound_raw_advance(&c, |token| {
            assert_eq!(token.previous_binding(), before_accept.current);
            assert_ne!(token.current_binding(), token.previous_binding());
            assert_eq!(
                token.current_binding().domain(),
                before_accept.current.domain()
            );
            assert!(!token.previous_binding().is_initial());
            assert_eq!((token.previous_prefix(), token.current_prefix()), (1, 2));
            assert_eq!(token.append_ordinals(), 1..2);
            assert_eq!(
                token.winner_edits(),
                &[expected],
                "下一事务必须得到非空真实赢家变化"
            );
            let raw = token.raw();
            assert_eq!(
                super::super::business_json::field(raw.get(1).unwrap(), "seq").as_i64(),
                Some(1)
            );
            observed = Some(RawObservation::read(&token)?);
            if reject_consumer {
                Err("TestOnlyNonemptyConsumerRejected".into())
            } else {
                Ok(())
            }
        });
        let observed = observed.expect("必须进入真实token消费者");
        assert_eq!(observed.edits, vec![expected]);
        if reject_consumer {
            assert_eq!(result.unwrap_err(), "TestOnlyNonemptyConsumerRejected");
            assert!(owner.construction_lease.stale.get());
            assert!(
                next.construction.borrow().is_none(),
                "消费失败丢弃pending facts"
            );
            let called = Cell::new(false);
            assert!(next
                .with_bound_raw_advance(&c, |_| {
                    called.set(true);
                    Ok(())
                })
                .is_err());
            assert!(!called.get(), "旧owner失效后不能重签相同非空delta");
        } else {
            result.unwrap();
            assert!(!owner.construction_lease.stale.get());
            assert!(next.construction.borrow().is_some());
        }
        drop(next);
        assert!(owner.connection.is_autocommit());
        assert!(
            owner.construction_lease.stale.get(),
            "未提交消费状态回滚后撤销"
        );
        assert!(owner.construction.borrow().is_none());
        let persisted: i64 = f
            .conn
            .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(persisted, 2, "消费者失败/回滚不能抹掉上次真实Accept提交");
    }

    #[test]
    fn owner_raw_formal_accept_probe_is_scoped_to_owner_and_thread() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let foreign = AuditConnection::open(&f.db_path()).unwrap();
        let probe = AcceptProbeGuard::start(&owner);
        let c = context();
        let foreign_tx = foreign.begin(TransactionBehavior::Deferred).unwrap();
        // 无CompleteCandidate；若错误命中本线程另一owner，将进入issuer并立即报错。
        select_for_formal_accept(&foreign_tx, &c).unwrap();
        assert!(foreign_tx.construction.borrow().is_none());
        assert!(!foreign.construction_lease.stale.get());
        foreign_tx.read_end().unwrap();

        let db = f.db_path();
        let other = std::thread::spawn(move || {
            ACCEPT_PROBE.with(|slot| assert!(slot.borrow().is_none(), "父线程注册不能泄漏"));
            let other = AuditConnection::open(&db).unwrap();
            let mut c = context();
            let active = other.begin(TransactionBehavior::Deferred).unwrap();
            select_for_formal_accept(&active, &c).unwrap();
            assert!(
                active.construction.borrow().is_none(),
                "未启用的线程不能签发"
            );
            let own_probe = AcceptProbeGuard::start(&other);
            active
                .audit_admission(
                    &json!({"payload":{"op":"ready"}}),
                    &s_allocation::Mode::CapturedNative,
                    &mut c,
                )
                .unwrap();
            select_for_formal_accept(&active, &c).unwrap();
            own_probe.only_observation()
        })
        .join()
        .unwrap();
        ACCEPT_PROBE.with(|slot| {
            let slot = slot.borrow();
            let parent = slot.as_ref().unwrap();
            assert_eq!(parent.life, owner.life);
            assert!(
                parent.observations.is_empty(),
                "并行线程不得写入父线程观察结果"
            );
        });
        let mut c = context();
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        active
            .audit_admission(
                &json!({"payload":{"op":"ready"}}),
                &s_allocation::Mode::CapturedNative,
                &mut c,
            )
            .unwrap();
        select_for_formal_accept(&active, &c).unwrap();
        let parent = probe.only_observation();
        assert_ne!(parent.current.domain(), other.current.domain());
        assert!(
            !owner.construction_lease.stale.get(),
            "另一线程回滚不得撤销本owner"
        );
    }

    #[test]
    fn owner_raw_formal_accept_append_commits_before_nonempty_delta() {
        formal_accept_case(false, false);
    }
    #[test]
    fn owner_raw_formal_accept_revision_commits_before_nonempty_delta() {
        formal_accept_case(true, false);
    }
    #[test]
    fn owner_raw_nonempty_consumer_failure_stales_after_formal_accept_commit() {
        formal_accept_case(true, true);
    }

    fn context() -> request_audit::RequestAuditContext {
        let mut context = request_audit::RequestAuditContext::with_source_keeper(
            cross_request_source_cache::Keeper::new(536_870_912),
        );
        context
            .freeze_manifest_workspace_budget(536_870_912, 0, 0)
            .unwrap();
        context
    }
    fn fixture() -> prepared::tests::Fixture {
        let mut f = prepared::tests::Fixture::new();
        let (e, t) = f.accept_begin(1, ("owner-raw", 0, 6, 12, 3, 7), 1);
        f.publish(&e, &t).unwrap();
        f
    }
    fn mint(
        active: &AuditTxn<'_>,
        context: &mut request_audit::RequestAuditContext,
    ) -> RawAdvanceBinding {
        active
            .audit_admission(
                &json!({"payload":{"op":"ready"}}),
                &s_allocation::Mode::CapturedNative,
                context,
            )
            .unwrap();
        active
            .with_bound_raw_advance(context, |token| {
                token.ensure_current()?;
                assert!(token.current_prefix() > 0, "正控必须消费非空真实原件");
                Ok(token.current_binding())
            })
            .unwrap()
    }
    fn close_fixed(active: &AuditTxn<'_>, context: &request_audit::RequestAuditContext) {
        active
            .before_fixed_write(FixedOperation::Accept, Some(context))
            .unwrap();
        active.tx().execute("UPDATE s_protocol_meta SET next_attempt_ordinal=next_attempt_ordinal WHERE singleton=1", []).unwrap();
        active.finish_fixed_write().unwrap();
    }

    /// 实际Runtime生成/逐stage/owner全投影核验；只用测试选择器签发raw资格。
    fn set_publish_case(fail_commit: bool, unscoped_update: bool) {
        let mut f = fixture();
        let (envelope, token) = f.accept_begin(2, ("set-next", 10, 8, 16, 6, 12), 1);
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut context = context();
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut active = owner.begin(TransactionBehavior::Immediate).unwrap();
        mint(&active, &mut context);
        active
            .before_fixed_write(FixedOperation::Publish, Some(&context))
            .unwrap();
        let pending = active.pending.borrow();
        let phase = PublishPhase {
            transaction: active.transaction.as_mut().unwrap(),
            audited: pending.as_ref().map(|candidate| &candidate.audited),
        };
        let receipt = runtime
            .publish_in_owner(phase, &token, &envelope, &mut context)
            .unwrap();
        assert_eq!(receipt.generation, 2);
        drop(pending);
        if unscoped_update {
            active.tx().execute("UPDATE s_content_objects SET object_revision=object_revision WHERE rowid=(SELECT min(rowid) FROM s_content_objects)", []).unwrap();
            assert!(
                active.finish_fixed_write().is_err(),
                "stage之外的合法形状SQL也不能借最终行相等蒙混"
            );
            drop(active);
            assert!(owner.construction.borrow().is_none());
            assert!(owner.construction_lease.stale.get());
            return;
        }
        active.finish_fixed_write().unwrap();
        assert!(active
            .construction
            .borrow()
            .as_ref()
            .unwrap()
            .has_pending_set());
        assert!(owner.construction.borrow().is_none(), "效果完成尚未提交");
        if fail_commit {
            active.tx().commit_hook(Some(|| true)).unwrap();
        }
        let committed = active.commit_publication();
        if fail_commit {
            assert!(committed.is_err());
            owner.connection.commit_hook(None::<fn() -> bool>).unwrap();
            assert!(owner.construction.borrow().is_none());
            assert!(owner.construction_lease.stale.get());
            assert_eq!(meta_i64(&owner.connection, "generation").unwrap(), 1);
        } else {
            let ended = committed.unwrap();
            assert!(owner.construction.borrow().as_ref().unwrap().has_set());
            owner.continue_with(ended).unwrap();
            assert_eq!(meta_i64(&owner.connection, "generation").unwrap(), 2);
            let next = owner.begin(TransactionBehavior::Immediate).unwrap();
            assert!(next.bind_existing(Some(&context)).unwrap());
            next.with_bound_raw_advance(&context, |raw| {
                raw.ensure_current()?;
                assert!(raw.append_ordinals().is_empty());
                Ok(())
            })
            .unwrap();
            // 丢弃未提交的下一事务须连上代集合资格一并撤销。
            drop(next);
            assert!(owner.construction.borrow().is_none());
            assert!(owner.construction_lease.stale.get());
        }
    }
    #[test]
    fn owner_set_real_publish_stage_effects_commit_promotes() {
        set_publish_case(false, false);
    }
    #[test]
    fn owner_set_real_publish_commit_failure_revokes_everything() {
        set_publish_case(true, false);
    }
    #[test]
    fn owner_set_real_publish_unscoped_same_value_update_rejects() {
        set_publish_case(false, true);
    }
    #[test]
    fn owner_raw_real_effects_only_promote_after_real_commit() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut c = context();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        let first = mint(&active, &mut c);
        assert!(owner.construction.borrow().is_none());
        close_fixed(&active, &c);
        assert!(
            owner.construction.borrow().is_none(),
            "effects闭合仍非COMMIT"
        );
        let ended = active.commit().unwrap();
        assert!(owner.construction.borrow().is_some());
        owner.continue_with(ended).unwrap();
        let next = owner.begin(TransactionBehavior::Immediate).unwrap();
        next.audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut c,
        )
        .unwrap();
        next.with_bound_raw_advance(&c, |token| {
            assert_eq!(token.previous_binding(), first);
            assert!(!token.previous_binding().is_initial());
            assert!(token.append_ordinals().is_empty());
            assert!(token.winner_edits().is_empty());
            Ok(())
        })
        .unwrap();
        drop(next);
        assert!(owner.construction.borrow().is_none());
        assert!(owner.construction_lease.stale.get());
    }

    #[test]
    fn owner_raw_rollback_unknown_write_and_unclosed_commit_revoke() {
        for failure in ["rollback", "unknown", "unclosed", "after-close"] {
            let f = fixture();
            let owner = AuditConnection::open(&f.db_path()).unwrap();
            let mut c = context();
            let active = owner.begin(TransactionBehavior::Immediate).unwrap();
            mint(&active, &mut c);
            match failure {
                "rollback" => drop(active),
                "unknown" => {
                    assert!(active.before_write().is_err());
                    drop(active);
                }
                "unclosed" => {
                    assert!(active.commit().is_err());
                }
                "after-close" => {
                    close_fixed(&active, &c);
                    active
                        .tx()
                        .execute(
                            "UPDATE s_protocol_meta SET next_attempt_ordinal=next_attempt_ordinal",
                            [],
                        )
                        .unwrap();
                    assert!(active.commit().is_err());
                }
                _ => unreachable!(),
            }
            assert!(owner.connection.is_autocommit(), "{failure}");
            assert!(owner.construction.borrow().is_none(), "{failure}");
            assert!(owner.construction_lease.stale.get(), "{failure}");
            let reopened = AuditConnection::open(&f.db_path()).unwrap();
            let active = reopened.begin(TransactionBehavior::Immediate).unwrap();
            let mut c = context();
            active
                .audit_admission(
                    &json!({"payload":{"op":"ready"}}),
                    &s_allocation::Mode::CapturedNative,
                    &mut c,
                )
                .unwrap();
            active
                .with_bound_raw_advance(&c, |token| {
                    assert!(token.previous_binding().is_initial());
                    Ok(())
                })
                .unwrap();
        }
    }

    #[test]
    fn owner_raw_commit_hook_failure_never_promotes() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut c = context();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        mint(&active, &mut c);
        close_fixed(&active, &c);
        active.tx().commit_hook(Some(|| true)).unwrap();
        assert!(active.commit().is_err());
        owner.connection.commit_hook(None::<fn() -> bool>).unwrap();
        assert!(owner.connection.is_autocommit());
        assert!(owner.construction.borrow().is_none());
        assert!(owner.construction_lease.stale.get());
    }

    #[test]
    fn owner_raw_external_commit_rejects_equal_length_material() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut c = context();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        mint(&active, &mut c);
        close_fixed(&active, &c);
        let ended = active.commit().unwrap();
        owner.continue_with(ended).unwrap();
        let outside = open_db(&f.db_path()).unwrap();
        outside.execute_batch("BEGIN IMMEDIATE; UPDATE s_protocol_meta SET next_attempt_ordinal=next_attempt_ordinal+1; UPDATE s_protocol_meta SET next_attempt_ordinal=next_attempt_ordinal-1; COMMIT;").unwrap();
        let next = owner.begin(TransactionBehavior::Immediate).unwrap();
        next.audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut c,
        )
        .unwrap();
        assert!(next.with_bound_raw_advance(&c, |_| Ok(())).is_err());
        assert!(owner.construction_lease.stale.get());
    }

    #[test]
    fn owner_raw_schema_read_domain_and_consumer_failure_revoke() {
        for failure in ["schema", "domain", "consumer"] {
            let f = fixture();
            let owner = AuditConnection::open(&f.db_path()).unwrap();
            let mut c = context();
            let active = owner.begin(TransactionBehavior::Immediate).unwrap();
            if failure == "consumer" {
                active
                    .audit_admission(
                        &json!({"payload":{"op":"ready"}}),
                        &s_allocation::Mode::CapturedNative,
                        &mut c,
                    )
                    .unwrap();
                assert!(active
                    .with_bound_raw_advance(&c, |_| Err::<(), _>("consumer拒绝".into()))
                    .is_err());
            } else {
                mint(&active, &mut c);
                close_fixed(&active, &c);
                if failure == "schema" {
                    active
                        .tx()
                        .execute_batch("CREATE TABLE unexpected(x)")
                        .unwrap();
                } else {
                    active
                        .tx()
                        .execute_batch("PRAGMA recursive_triggers=OFF")
                        .unwrap();
                    let actual: i64 = active
                        .tx()
                        .query_row("PRAGMA recursive_triggers", [], |r| r.get(0))
                        .unwrap();
                    if actual == active.construction.borrow().as_ref().unwrap().seal.domain[2] {
                        active
                            .tx()
                            .execute_batch("PRAGMA recursive_triggers=ON")
                            .unwrap();
                    }
                }
                assert!(active.commit().is_err());
            }
            assert!(owner.construction.borrow().is_none());
            assert!(owner.construction_lease.stale.get());
        }
    }
}
