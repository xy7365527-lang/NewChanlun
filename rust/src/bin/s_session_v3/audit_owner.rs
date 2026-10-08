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
    #[cfg(feature = "s_capture_native_testonly")]
    source_keeper: std::cell::RefCell<Option<cross_request_source_cache::Keeper>>,
}

/// 此标记只证明 owner 已确认事务终结，不证明任何缓存依赖保持。
pub(super) struct Ended {
    life: u64,
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
    pub(super) fn new(connection: Connection) -> Result<Self, String> {
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
            #[cfg(feature = "s_capture_native_testonly")]
            source_keeper: Default::default(),
        })
    }

    fn clear_candidates(&self) {
        #[cfg(feature = "s_capture_native_testonly")]
        drop(self.source_keeper.borrow_mut().take());
    }

    fn begin(&self, behavior: TransactionBehavior) -> Result<AuditTxn<'_>, String> {
        if self.retired.get() || !self.connection.is_autocommit() {
            self.retired.set(true);
            self.clear_candidates();
            return Err(invalid("连接已退役或存在未终结事务"));
        }
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
        s_allocation::audit_in_request(tx, mode, s_allocation::Phase::Handle, context)?;
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
                        let accepted = raw_input::accept_events(tx, &input, volume_unit)?;
                        control.accept(&view, &accepted)?;
                        wrote = true;
                        Admission::Advance {
                            original: envelope,
                            accepted: true,
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
        // 此处只移交既有来源候选；下轮仍完整重绑定原件，Ended 不授权跳过 Q/C_g。
        *self.source_keeper.get_mut() = context.finish_source_keeper(ended);
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
        let _sealed = (self.owner.life, self.data_version, self.schema_version);
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
    context.finish_source_keeper(Ended { life: 0 })
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
