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

static NEXT_LIFE: AtomicU64 = AtomicU64::new(1);

pub(super) struct AuditConnection {
    connection: Connection,
    life: u64,
    retired: Cell<bool>,
    next_transaction: Cell<u64>,
    write_sequence: Cell<u64>,
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
}
impl PublishPhase<'_, '_> {
    pub(super) fn publish(
        self,
        runtime: &mut prepared::Runtime,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<commit::CommitReceipt, String> {
        runtime.publish_impl(self.transaction, token, envelope, Some(context))
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
    #[cfg(feature = "s_capture_native_testonly")]
    complete: std::cell::RefCell<Option<CompleteCandidate>>,
}

pub(super) enum Admission {
    Reply(Value),
    Advance { original: Value, accepted: bool },
}

/// 保留原 Advance→Prepare 的实际工作集重叠；Owned 峰值不能因拆函数被静默改口径。
pub(super) struct Begun {
    pub(super) token: String,
    _audited: state::AuditedState,
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
            #[cfg(feature = "s_capture_native_testonly")]
            complete: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            source_keeper: Default::default(),
        })
    }

    fn clear_candidates(&self) {
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
            #[cfg(feature = "s_capture_native_testonly")]
            complete: Default::default(),
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
                        active.before_write()?;
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
        self.clear_complete();
        let active = self.begin(TransactionBehavior::Immediate)?;
        let tx = active.tx();
        let audited =
            s_allocation::audit_in_request(tx, mode, s_allocation::Phase::Advance, context)?;
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
        let frontier = i64::try_from(audited.raw.len())
            .map_err(invalid)?
            .checked_sub(1)
            .ok_or_else(|| invalid("没有原始输入"))?;
        let base = meta_get_opt(tx, "structure_cut")?.ok_or_else(|| invalid("缺当前cut"))?;
        active.before_write()?;
        let token = control.begin(&view, generation, frontier, &base, epoch)?;
        drop(control);
        active.commit()?;
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
            active.before_write()?;
            let phase = PublishPhase {
                transaction: active.transaction.as_mut().unwrap(),
            };
            let receipt = runtime.publish_in_owner(phase, token, envelope, context)?;
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
        // 完整候选仅由本 owner 的本次无写事务签发；旧来源池仍独立移交。
        let cap = context.complete_retention_cap();
        let mut complete = ended.complete;
        // 只有对应事务 read_end 封存的实际无写结果可接回。slot/fixture 没有签发能力。
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
        self.clear_candidates();
        let next = parse_writer_epoch(new_epoch, "new writer_epoch")?;
        let active = self.begin(TransactionBehavior::Immediate)?;
        let tx = active.tx();
        state::audit(tx)?;
        let current = meta_get_opt(tx, "writer_epoch")?.ok_or_else(|| invalid("缺writer_epoch"))?;
        if next <= parse_writer_epoch(&current, "current writer_epoch")? {
            return Err("StaleWriter：recover换代必须严格上升".into());
        }
        let control = control::S3Control::attach(tx, clock)?;
        let generation = meta_get_opt(tx, "generation")?.ok_or_else(|| invalid("缺generation"))?;
        let prior = meta_get_opt(tx, "advance_state")?.ok_or_else(|| invalid("缺advance_state"))?;
        active.before_write()?;
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
        active.commit()?;
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
        Ok(Ended {
            life: self.owner.life,
            #[cfg(feature = "s_capture_native_testonly")]
            complete: None,
        })
    }
}

impl Drop for AuditTxn<'_> {
    fn drop(&mut self) {
        if let Some(tx) = self.transaction.take() {
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
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_report(&self) -> Value {
        self.complete.borrow().as_ref().map_or(Value::Null, |c| json!({
            "charge":c.charge,"life":c.seal.life,"transaction":c.seal.transaction_id,
            "write_sequence":c.seal.write_sequence,"tables":c.originals.len(),
            "nodes":c.slot.node_count,"raw_rows":c.audited.raw.len(),"cuts":c.audited.cuts.len(),
            "raw_checked":c.audited.raw_prefixes.as_ref().map(|p|p.checked_len()),
            "published_prefix":c.audited.raw_prefixes.as_ref().map(|p|p.published_len()),
            "source_hash_rows":c.audited.raw_sources.as_ref().map(|p|p.hashed_rows()),
            "source_counts_preserved":c.slot.source_pending.is_some(),
            "capacity_method":"实际 Vec/String capacity 加树/散列表保守槽位上界；非RSS",
        }))
    }
}
impl AuditTxn<'_> {
    fn before_write(&self) -> Result<(), String> {
        // 必须发生在任何固定 operation（含 Noop）的第一条写 SQL 之前。
        self.owner.clear_candidates();
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
            // 请求局部状态读取只决定是否有无写资格。未知操作/新消息/待推进一律 cold。
            // 后续原 inspect_ingest/lookup 仍逐字执行全部请求判据，错误绝不重判。
            let no_write = match envelope["payload"]["op"].as_str() {
                Some("ready" | "receipt") => true,
                Some("ingest") => v2::message_row(
                    self.tx(),
                    required(envelope, "source_namespace")?,
                    required(envelope, "source_epoch")?,
                    required(envelope, "message_id")?,
                )?
                .is_some_and(|m| m.status == "Committed"),
                _ => false,
            };
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
                if no_write && candidate.seal.same_snapshot(&seal) {
                    candidate.seal = seal;
                    #[cfg(test)]
                    state::trace_event("owner_complete_hit", None);
                    *self.complete.borrow_mut() = Some(candidate);
                    return Ok(());
                }
                // 外部提交/写请求在核验前决定冷分派并真释放；不是校验错误后的回退。
                drop(candidate);
            }
            let audited = s_allocation::audit_in_request(
                self.tx(),
                mode,
                s_allocation::Phase::Handle,
                context,
            )?;
            if no_write {
                if let Some(slot) = context.take() {
                    match CompleteCandidate::capture(
                        self.tx(),
                        seal,
                        slot,
                        audited,
                        context.complete_workspace_limit(),
                        context.complete_entry_reserved_bytes(),
                    )? {
                        Ok(candidate) => {
                            context
                                .note_complete_source_work(candidate.slot.source_pending.as_ref());
                            *self.complete.borrow_mut() = Some(candidate);
                        }
                        Err(slot) => context.restore_slot(slot),
                    }
                }
            }
            return Ok(());
        }
        s_allocation::audit_in_request(self.tx(), mode, s_allocation::Phase::Handle, context)?;
        Ok(())
    }

    #[cfg(feature = "s_capture_native_testonly")]
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
                let encoding: String = self
                    .tx()
                    .query_row(sql, [], |r| r.get(0))
                    .map_err(invalid)?;
                out[i] = match encoding.as_str() {
                    "UTF-8" => 1,
                    "UTF-16le" => 2,
                    "UTF-16be" => 3,
                    _ => return Err(invalid("未知编码域")),
                };
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

#[cfg(feature = "s_capture_native_testonly")]
struct SnapshotSeal {
    life: u64,
    transaction_id: u64,
    write_sequence: u64,
    data_version: i64,
    schema_version: i64,
    domain: [i64; 6],
}
#[cfg(feature = "s_capture_native_testonly")]
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
    source_pool_charge: usize,
    current_sql_capacity: usize,
    inline: usize,
    total: usize,
    entry_reserved_bytes: usize,
    handoff_peak: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
struct CompleteCandidate {
    seal: SnapshotSeal,
    slot: request_audit::VerifiedHistoricalSlot,
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
            source_pool_charge: slot
                .source_pending
                .as_ref()
                .map_or(0, |p| p.full_retention_charge()),
            current_sql_capacity: 0,
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
        let mut originals: Vec<request_audit::ExactRows> = Vec::new();
        let mut candidate_peak = charge.total;
        // 表名同样经有界原件读取；不先 collect 一份未登记的 String 列表。
        let Some((tables, tables_peak)) = request_audit::ExactRows::read_optional_bounded(
            tx,
            "SELECT name FROM main.sqlite_schema WHERE type='table' AND name!='s_content_nodes' ORDER BY name",
            limit - charge.total,
        )? else { return Ok(Err(slot)); };
        candidate_peak = candidate_peak.max(sum([charge.total, tables_peak]));
        let table_capacity = tables.rows.len().saturating_add(4);
        let before_reserve = sum([
            charge.total,
            tables.heap_bytes(),
            super::audit_retention::vec_bytes::<request_audit::ExactRows>(table_capacity),
        ]);
        if before_reserve >= limit || originals.try_reserve_exact(table_capacity).is_err() {
            return Ok(Err(slot));
        }
        candidate_peak = candidate_peak.max(before_reserve);
        let mut read_original = |sql: &str, sql_heap: usize| -> Result<bool, String> {
            let extra = sum([originals.heap_bytes(), tables.heap_bytes(), sql_heap]);
            let Some(remaining) = limit.checked_sub(charge.total.saturating_add(extra)) else {
                return Ok(false);
            };
            let Some((rows, row_peak)) =
                request_audit::ExactRows::read_optional_bounded(tx, sql, remaining)?
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
            let extra = sum([originals.heap_bytes(), tables.heap_bytes(), sql.capacity()]);
            let Some(remaining) = limit.checked_sub(charge.total.saturating_add(extra)) else {
                return Ok(Err(slot));
            };
            let Some((rows, row_peak)) =
                request_audit::ExactRows::read_optional_bounded(tx, &sql, remaining)?
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
        #[cfg(test)]
        state::trace_event("owner_complete_cold_candidate", None);
        Ok(Ok(Self {
            seal,
            slot,
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
            std::rc::Rc::as_ptr(&c.slot.result) as usize,
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
