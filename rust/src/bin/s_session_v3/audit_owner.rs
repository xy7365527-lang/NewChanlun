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
/// 冷根父账类型：无native特征时不存在可出生的根，参数只能为None。
#[cfg(feature = "s_capture_native_testonly")]
type ColdRoot = native::Budget;
#[cfg(not(feature = "s_capture_native_testonly"))]
type ColdRoot = std::convert::Infallible;
#[cfg(feature = "s_capture_native_testonly")]
mod q;

/// 只有 owner 从同快照完整候选以及固定 effects 闭合后创建；不对外开放构造。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) mod allocation_probe;
/// #1488 P3-1 TestOnly：冷根审计失败分支在根析构前记下（清除前根活许可, 清除后根活许可）。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) mod cold_root_probe {
    use std::cell::RefCell;
    thread_local!(static NOTES: RefCell<Vec<(usize, usize)>> = const { RefCell::new(Vec::new()) });
    thread_local!(static FAIL_NEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    /// 下一次根内审计成功后注入一次失败，驱动真实的失败分支（首跑副作用已全部发生）。
    pub(crate) fn fail_next_cold_root_audit() {
        FAIL_NEXT.with(|f| f.set(true));
    }
    pub(super) fn inject_failure() -> Result<(), String> {
        if FAIL_NEXT.with(|f| f.replace(false)) {
            return Err("cold_root_probe: injected".into());
        }
        Ok(())
    }
    thread_local!(static LEAK_NEXT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) });
    thread_local!(static LEAKED: RefCell<Vec<super::native::Permit>> = const { RefCell::new(Vec::new()) });
    /// #1488 P3-d：下一次冷根失败分支在根上挂一张 legacy cache 之外、clear_handles 清不掉的许可。
    pub(crate) fn leak_next_cold_root_permit(bytes: usize) {
        LEAK_NEXT.with(|l| l.set(bytes));
    }
    pub(super) fn leak_if_armed(root: &super::native::Budget) {
        let bytes = LEAK_NEXT.with(|l| l.replace(0));
        if bytes != 0 {
            let permit = root.reserve(bytes).expect("注入残留许可");
            LEAKED.with(|l| l.borrow_mut().push(permit));
        }
    }
    /// 取回注入的残留许可（析构即还给其根）。
    pub(crate) fn take_leaked() -> Vec<super::native::Permit> {
        LEAKED.with(|l| std::mem::take(&mut *l.borrow_mut()))
    }
    pub(crate) fn note(before: usize, after: usize) {
        NOTES.with(|n| n.borrow_mut().push((before, after)));
    }
    pub(crate) fn take() -> Vec<(usize, usize)> {
        NOTES.with(|n| std::mem::take(&mut *n.borrow_mut()))
    }
    thread_local!(static RERUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) });
    /// #1488 P3-δ：冷根失败分支析构根后无票重跑的次数（残留非零分支必须为0，不得自动重试）。
    pub(super) fn note_rerun() {
        RERUNS.with(|r| r.set(r.get() + 1));
    }
    pub(crate) fn take_reruns() -> usize {
        RERUNS.with(|r| r.replace(0))
    }
}

#[cfg(feature = "s_capture_native_testonly")]
#[path = "construction_set.rs"]
mod construction_set;
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) use construction_set::{Edit, OwnerSourceCheck, SequenceDelta, SourceArray};
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) use effects::{
    MaterialAccess, PublicationStageIdentity, SourceCutBinding, SourceMaterial, StageKind,
};
#[path = "construction_state.rs"]
mod construction_state;
pub(crate) use super::business_json::{RawWinner, RawWinnerEdit};
#[cfg(test)]
pub(crate) use construction_state::RawAdvanceFixture;
pub(crate) use construction_state::{BoundRawAdvance, RawAdvanceBinding, RawDomain};

#[cfg(all(test, feature = "s_capture_native_testonly"))]
thread_local! {
    static PUBLICATION_SCOPE_TRACE: std::cell::RefCell<Vec<(&'static str, usize)>> = const { std::cell::RefCell::new(Vec::new()) };
    static PUBLICATION_TAIL_FAILURE: Cell<bool> = const { Cell::new(false) };
    // #1488 受控 Recover 迁移负控：交接失败注入点与 Recover 写后/effects 闭合前的事务回调。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    static RECOVER_HANDOFF_FAILURE: Cell<bool> = const { Cell::new(false) };
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    static RECOVER_AFTER_WRITE: Cell<Option<fn(&Transaction<'_>) -> Result<(), String>>> = const { Cell::new(None) };
    // #1488 0034 P3-6：确认门负控注入——交给 confirm_recover 的持久 epoch 读数偏移一位。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    static RECOVER_CONFIRM_SKEW: Cell<bool> = const { Cell::new(false) };
    // #1488 0034 P3-1：交接摘要负控注入——Some(他账)改指他账，None改指同账下一冻结域。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    static RECOVER_PARENT_SKEW: std::cell::RefCell<Option<Option<native::Budget>>> = const { std::cell::RefCell::new(None) };
}
/// #1488：唯一 publication 父资源。绝不持有 journal 强引用。
#[cfg(feature = "s_capture_native_testonly")]
struct PublicationResources {
    budget: native::Budget,
    parent_limit: usize,
    before: SnapshotSeal,
    connection: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) struct PublicationScope<'w> {
    resources: &'w PublicationResources,
    work: &'w newchan_rust::s_content::capture_allocation::scratch::WorkScope,
}
#[cfg(feature = "s_capture_native_testonly")]
impl<'w> PublicationScope<'w> {
    pub(crate) fn work(
        &self,
    ) -> &'w newchan_rust::s_content::capture_allocation::scratch::WorkScope {
        self.work
    }
    pub(crate) fn identity(&self) -> [u64; 3] {
        let s = self.resources.before;
        [s.life, s.transaction_id, s.write_sequence]
    }
    pub(crate) fn check_transaction(&self, tx: &Transaction<'_>) -> Result<(), String> {
        if tx.is_autocommit() || self.resources.connection != unsafe { tx.handle() } as usize {
            return Err(construction_state::stale("publication资源跨事务或已终结"));
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn stage_identity(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<PublicationStageIdentity, String> {
        effects::publication_stage_identity(tx, self)
    }
    #[cfg(test)]
    fn trace(&self, phase: &'static str) {
        PUBLICATION_SCOPE_TRACE.with(|trace| {
            trace
                .borrow_mut()
                .push((phase, self.work as *const _ as usize))
        });
    }
    fn budget(&self) -> &native::Budget {
        &self.resources.budget
    }
    fn remaining(&self) -> usize {
        self.resources
            .parent_limit
            .min(self.resources.budget.remaining())
    }
}
#[cfg(feature = "s_capture_native_testonly")]
struct ConstructionRun<T> {
    value: T,
    before: SnapshotSeal,
    after: SnapshotSeal,
}

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
    // #1488 P1-1/P1-2：请求收尾静止点各保留父账的live读数；下次冻结逐账对拍，不等即来源不明。
    #[cfg(feature = "s_capture_native_testonly")]
    quiescent: std::cell::RefCell<Vec<native::LedgerMark>>,
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
    #[cfg(feature = "s_capture_native_testonly")]
    raw: Option<BoundRawAdvance<'a>>,
    #[cfg(feature = "s_capture_native_testonly")]
    capture: Option<&'a mut newchan_rust::s_content::AuditCapture>,
    #[cfg(feature = "s_capture_native_testonly")]
    source: Option<&'a mut Option<source_audit::NativeSourceState>>,
    #[cfg(feature = "s_capture_native_testonly")]
    publication: Option<&'a PublicationScope<'a>>,
}
impl PublishPhase<'_, '_> {
    pub(super) fn publish(
        self,
        runtime: &mut prepared::Runtime,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<commit::CommitReceipt, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        return runtime.publish_impl_bound(
            self.transaction,
            token,
            envelope,
            Some(context),
            self.audited,
            self.raw.as_ref(),
            self.capture,
            self.source,
            self.publication,
        );
        #[cfg(not(feature = "s_capture_native_testonly"))]
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
    construction: std::cell::RefCell<Option<construction_state::PendingRawState>>,
    #[cfg(feature = "s_capture_native_testonly")]
    complete: std::cell::RefCell<Option<CompleteCandidate>>,
    #[cfg(feature = "s_capture_native_testonly")]
    pending: std::cell::RefCell<Option<CompleteCandidate>>,
    #[cfg(feature = "s_capture_native_testonly")]
    recording: std::cell::RefCell<Option<effects::Recording>>,
    #[cfg(feature = "s_capture_native_testonly")]
    effect_limit: Cell<usize>,
    #[cfg(feature = "s_capture_native_testonly")]
    publication_issued: Cell<bool>,
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
            #[cfg(feature = "s_capture_native_testonly")]
            quiescent: Default::default(),
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
            #[cfg(feature = "s_capture_native_testonly")]
            publication_issued: Cell::new(false),
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
        super::require_current_writer_rule(
            &meta_get_opt(tx, "rule_revision")?.ok_or_else(|| invalid("缺rule_revision"))?,
        )?;
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
            #[cfg(feature = "s_capture_native_testonly")]
            let resources = if active.complete.borrow().is_some() && context.source_cache_enabled()
            {
                Some(active.issue_publication_resources(context)?)
            } else {
                None
            };
            #[cfg(feature = "s_capture_native_testonly")]
            let (receipt, checked) = if let Some(resources) = resources {
                let before = resources.before;
                let work = resources
                    .budget
                    .codec_work()
                    .map_err(|_| construction_state::stale("publication父工作域额度不足"))?;
                let (receipt, after) = work
                    .run(|work| {
                        let parent = PublicationScope {
                            resources: &resources,
                            work,
                        };
                        let receipt = self.publish_body(
                            &mut active,
                            runtime,
                            token,
                            envelope,
                            context,
                            Some(&parent),
                        )?;
                        let after = active.construction_seal()?;
                        active.check_publication_transition(before, after)?;
                        #[cfg(test)]
                        {
                            parent.trace("tail");
                            if PUBLICATION_TAIL_FAILURE.with(|flag| flag.replace(false)) {
                                let _ = parent.work().reject(newchan_rust::s_content::capture_allocation::scratch::ScratchFailure::Ledger);
                            }
                        }
                        Ok::<_, String>((receipt, after))
                    })
                    .map_err(|e| match native::Failure::from(e) {
                        native::Failure::Invalid(e) => e,
                        native::Failure::BudgetExceeded => {
                            construction_state::stale("publication父工作域尾检失败")
                        }
                    })?;
                let checked = ConstructionRun {
                    value: (),
                    before,
                    after,
                };
                (receipt, Some(checked))
            } else {
                (
                    self.publish_body(&mut active, runtime, token, envelope, context, None)?,
                    None,
                )
            };
            #[cfg(not(feature = "s_capture_native_testonly"))]
            let receipt = self.publish_body(&mut active, runtime, token, envelope, context)?;
            #[cfg(feature = "s_capture_native_testonly")]
            if let Some(checked) = checked {
                active.check_publication_run(checked)?;
            }
            let ended = active.commit_publication()?;
            Ok((receipt, ended))
        })();
        if result.is_err() {
            runtime.discard();
            drop(context.take());
        }
        result
    }

    fn publish_body(
        &self,
        active: &mut AuditTxn<'_>,
        runtime: &mut prepared::Runtime,
        token: &str,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
        #[cfg(feature = "s_capture_native_testonly")] parent: Option<&PublicationScope<'_>>,
    ) -> Result<commit::CommitReceipt, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        let issued = if active.complete.borrow().is_some() && context.source_cache_enabled() {
            Some(
                active.with_bound_raw_advance_in_publication(context, parent, |token| {
                    Ok(token.into_issued())
                })?,
            )
        } else {
            None
        };
        active.before_fixed_write(FixedOperation::Publish, Some(context))?;
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(parent) = parent {
            effects::attach_recording_resources(active.tx(), parent)?;
        }
        #[cfg(feature = "s_capture_native_testonly")]
        let has_raw = issued.is_some();
        #[cfg(feature = "s_capture_native_testonly")]
        let mut source = active
            .construction
            .borrow_mut()
            .as_mut()
            .and_then(|s| s.take_source());
        #[cfg(feature = "s_capture_native_testonly")]
        let mut pending = active.pending.borrow_mut();
        #[cfg(feature = "s_capture_native_testonly")]
        let (audited, capture) = match pending.as_mut() {
            Some(candidate) => (Some(&candidate.audited), candidate.capture.as_mut()),
            None => (None, None),
        };
        let phase = PublishPhase {
            #[cfg(feature = "s_capture_native_testonly")]
            publication: parent,
            transaction: active.transaction.as_mut().unwrap(),
            #[cfg(feature = "s_capture_native_testonly")]
            audited,
            #[cfg(feature = "s_capture_native_testonly")]
            capture,
            #[cfg(feature = "s_capture_native_testonly")]
            source: if has_raw { Some(&mut source) } else { None },
            #[cfg(feature = "s_capture_native_testonly")]
            raw: issued
                .map(|issued| {
                    let audited = audited.ok_or_else(|| {
                        construction_state::stale("写前raw签发丢失同一pending原件")
                    })?;
                    issued.bind(audited.raw.view(), &self.construction_lease)
                })
                .transpose()?,
        };
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if let Some(parent) = parent {
            parent.trace("runtime");
        }
        let receipt = runtime.publish_in_owner(phase, token, envelope, context)?;
        #[cfg(feature = "s_capture_native_testonly")]
        if has_raw {
            let source =
                source.ok_or_else(|| construction_state::stale("正式来源核未回交pending状态"))?;
            let growth = active
                .construction
                .borrow_mut()
                .as_mut()
                .ok_or_else(|| construction_state::stale("来源回交丢失raw owner"))?
                .install_source(source)?;
            active.effect_limit.set(
                active
                    .effect_limit
                    .get()
                    .checked_sub(growth)
                    .ok_or_else(|| construction_state::stale("来源新增保留费超过工作帽"))?,
            );
        }
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(candidate) = pending.as_mut() {
            candidate.refresh_capture_charge()?;
            if candidate
                .charge
                .total
                .checked_add(native::Budget::metadata_bytes())
                .is_none_or(|n| n > candidate.resources.total_limit().unwrap_or(0))
            {
                return Err(construction_state::stale("来源原件/证明保留账超过效果帽"));
            }
        }
        #[cfg(feature = "s_capture_native_testonly")]
        drop(pending);
        #[cfg(feature = "s_capture_native_testonly")]
        active.finish_fixed_write_in_publication(parent)?;
        #[cfg(not(feature = "s_capture_native_testonly"))]
        active.finish_fixed_write()?;
        Ok(receipt)
    }

    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn take_source_keeper(&mut self) -> Option<cross_request_source_cache::Keeper> {
        self.source_keeper.borrow_mut().take()
    }

    /// #1488 P1-2锁：把入口取走的keeper放回（模拟生产入口不丢池时的attach路径）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn put_source_keeper_for_test(&self, keeper: cross_request_source_cache::Keeper) {
        *self.source_keeper.borrow_mut() = Some(keeper);
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
            drop(context);
            drop(ended);
            self.mark_quiescent();
            return;
        }
        // 完整候选由本 owner 的无写审计或固定 effects 核验签发；来源池独立移交。
        let cap = context.complete_retention_cap();
        let raw_retained = self
            .construction
            .get_mut()
            .as_ref()
            .map_or(0, |c| c.retained_bytes());
        #[cfg(test)]
        state::trace_owner_retention(cap, raw_retained);
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
        // #1488 P1-1：池与候选同父时拒绝保留池——同一L0不得由两个owner分别认领。
        if let (Some(c), Some(k)) = (complete.as_ref(), self.source_keeper.get_mut().as_mut()) {
            if k.source_home().is_some_and(|home| home.same_parent(&c.resources)) {
                k.reject_completed();
            }
        }
        *self.complete.get_mut() = complete;
        self.mark_quiescent();
    }

    /// #1488 P1-1/P1-2：请求局部持有者（context/slot/ended）均已析构后，对保留下来的
    /// 候选父账与keeper池父账各读一次live，作为下次冻结的来源证明；其后任何未经请求的增减都使映射失效。
    #[cfg(feature = "s_capture_native_testonly")]
    fn mark_quiescent(&self) {
        let mut marks = Vec::new();
        if let Some(c) = self.complete.borrow().as_ref() {
            marks.push(c.resources.quiescent_mark());
        }
        if let Some(home) = self
            .source_keeper
            .borrow()
            .as_ref()
            .and_then(|k| k.source_home())
        {
            marks.push(home.quiescent_mark());
        }
        *self.quiescent.borrow_mut() = marks;
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
        super::require_current_writer_rule(
            &meta_get_opt(tx, "rule_revision")?.ok_or_else(|| invalid("缺rule_revision"))?,
        )?;
        let current = meta_get_opt(tx, "writer_epoch")?.ok_or_else(|| invalid("缺writer_epoch"))?;
        let from = parse_writer_epoch(&current, "current writer_epoch")?;
        // 持久 epoch 严格上升门即换代证明构造门（原 next<=from 门并入，文案与顺序不变）。
        let handoff = construction_state::RecoverHandoff::new(from, next)?;
        // P3-1：证明另绑本事务候选父账身份与冻结域摘要；读不到则不绑，迁移时按缺摘要拒绝。
        #[cfg(feature = "s_capture_native_testonly")]
        #[allow(unused_mut)]
        let mut handoff = match active
            .complete
            .borrow()
            .as_ref()
            .and_then(|c| c.resources.handoff_stamp().ok())
        {
            Some(stamp) => handoff.bind_parent(stamp),
            None => handoff,
        };
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if let Some(skew) = RECOVER_PARENT_SKEW.with(|s| s.borrow_mut().take()) {
            let parent = handoff.parent_for_test();
            *parent = match skew {
                Some(foreign) => foreign.handoff_stamp().ok(),
                None => parent.take().map(|stamp| stamp.skew_domain_for_test()),
            };
        }
        let control = control::S3Control::attach(tx, clock)?;
        let generation = meta_get_opt(tx, "generation")?.ok_or_else(|| invalid("缺generation"))?;
        let prior = meta_get_opt(tx, "advance_state")?.ok_or_else(|| invalid("缺advance_state"))?;
        active.before_fixed_write(FixedOperation::Recover(handoff), None)?;
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
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if let Some(hook) = RECOVER_AFTER_WRITE.with(Cell::take) {
            hook(tx)?;
        }
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
        // #1488 P1-α：丢弃只析构，不新增分配；静止点读数只减去本次析构实际归还的字节，
        // 不重读live——否则静止点之后出现的来源不明许可会被洗进读数、在重试分区时吸收进U。
        #[cfg(feature = "s_capture_native_testonly")]
        let before: Vec<Option<usize>> =
            self.quiescent.borrow().iter().map(|mark| mark.current()).collect();
        self.clear_candidates();
        #[cfg(feature = "s_capture_native_testonly")]
        {
            let mut before = before.into_iter();
            self.quiescent
                .borrow_mut()
                .retain_mut(|mark| mark.settle_release(before.next().flatten()));
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn complete_retained_bytes(&self) -> usize {
        self.complete
            .borrow()
            .as_ref()
            .map_or(0, |c| {
                c.charge
                    .total
                    .saturating_add(native::Budget::metadata_bytes())
            })
            .saturating_add(
                self.construction
                    .borrow()
                    .as_ref()
                    .map_or(0, |c| c.retained_bytes()),
            )
    }
    /// #1488 P1-1（v4「冻结old_pool费用分区」）：O逐项照合同额收，不以max(L)补差；
    /// 同父物理L0由分区只读一次，超出同父合同额的真实分配为uncovered并消耗A。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn old_pool_partition(
        &self,
        keeper: Option<&super::cross_request_source_cache::Keeper>,
    ) -> Result<request_audit::OldPoolPartition, String> {
        use request_audit::OldPoolPartition as P;
        let complete = self.complete.borrow();
        let parent = complete.as_ref().map(|c| &c.resources);
        let mut partition = P::new(parent);
        if let Some(c) = complete.as_ref() {
            partition.item("complete_candidate.charge.total", P::ON_PARENT, c.charge.total)?;
            partition.item(
                "complete_parent.metadata_M",
                P::ON_PARENT,
                native::Budget::metadata_bytes(),
            )?;
        }
        if let Some(raw) = self.construction.borrow().as_ref() {
            // 状态许可从候选同一父账拆出；缺候选即无从证明驻留，由分区拒绝。
            partition.item("committed_raw_state", P::ON_PARENT, raw.retained_bytes())?;
        }
        if let Some(k) = keeper.filter(|k| k.has_pool()) {
            match k.source_home() {
                // #1488 P1-2：池按其所在父账的物理live单列一组（与候选同父则由分区拒绝），
                // 控制块M与池合同额同为该组合同项；不在任何账上的池只能按合同额计。
                Some(home) => {
                    partition.keeper_home(home)?;
                    partition.item("source_keeper_pool", P::KEEPER_PARENT, k.retained_source_bytes())?;
                    partition.item(
                        "keeper_parent.metadata_M",
                        P::KEEPER_PARENT,
                        native::Budget::metadata_bytes(),
                    )?;
                }
                None => partition.item("source_keeper_pool", P::ELSEWHERE, k.retained_source_bytes())?,
            }
        }
        drop(complete);
        for mark in self.quiescent.borrow().iter() {
            partition.attest(mark)?;
        }
        partition.seal()
    }
    /// #1488 P1-1/P1-α：请求入口的old_pool冻结序列（driver生产路径原样搬入，供锁直接驱动）：
    /// 旧费按逐项分区O+uncovered计；映射不成立即丢候选重映射，不补差；keeper池按其父账物理live
    /// 仍装不下或无从证明时只丢可选池，不把可丢弃缓存的失败变成业务失败。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn entry_old_pool_partition(
        &self,
        previous: &mut Option<cross_request_source_cache::Keeper>,
        keeper_cap: usize,
    ) -> Result<request_audit::OldPoolPartition, String> {
        let fits = |p: &Result<request_audit::OldPoolPartition, String>| {
            p.as_ref().is_ok_and(|p| p.carried() <= keeper_cap)
        };
        let mut partition = self.old_pool_partition(previous.as_ref());
        if !fits(&partition) {
            self.discard_after_error();
            if let Some(k) = previous.as_mut() {
                k.set_retention_cap(keeper_cap);
            }
            partition = self.old_pool_partition(previous.as_ref());
        }
        if !fits(&partition) {
            if let Some(k) = previous.as_mut() {
                k.clear();
            }
            partition = self.old_pool_partition(previous.as_ref());
        }
        partition
    }
    /// #1488 受控 Recover 迁移锁：owner 租约与已提交资格的只读观测。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn recover_probe_for_test(&self) -> RecoverProbe {
        RecoverProbe {
            stale: self.construction_lease.stale.get(),
            serial: self.construction_lease.serial.get(),
            retired: self.retired.get(),
            qualification: self.construction.borrow().as_ref().map(|c| RecoverQualification {
                writer_epoch: c.writer_epoch(),
                has_fee: c.has_fee(),
                binding: c.binding(),
                retained: c.retained_bytes(),
                has_parent: c.has_parent(),
            }),
        }
    }
    /// 完整候选所在父账的弱读数（不延长父账寿命）及其 live/peak。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_ledger_for_test(&self) -> Option<(native::LedgerMark, usize, usize)> {
        self.complete.borrow().as_ref().map(|c| {
            (
                c.resources.quiescent_mark(),
                c.resources.live(),
                c.resources.peak(),
            )
        })
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_on_ledger_for_test(&self, mark: &native::LedgerMark) -> bool {
        self.complete
            .borrow()
            .as_ref()
            .is_some_and(|c| mark.live_for(&c.resources).is_some())
    }
    /// 换代证明构造门（只认持久 epoch 严格上升）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn recover_handoff_accepts_for_test(from: i64, to: i64) -> bool {
        construction_state::RecoverHandoff::new(from, to).is_ok()
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn inject_recover_handoff_failure_for_test(&self) {
        RECOVER_HANDOFF_FAILURE.with(|flag| flag.set(true));
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn inject_recover_confirm_skew_for_test(&self) {
        RECOVER_CONFIRM_SKEW.with(|flag| flag.set(true));
    }
    /// P3-1 负控：下一次 recover 的交接摘要改指 foreign（Some）或同账下一冻结域（None）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn inject_recover_parent_for_test(&self, foreign: Option<native::Budget>) {
        RECOVER_PARENT_SKEW.with(|s| *s.borrow_mut() = Some(foreign));
    }
    /// 状态块许可所挂父账：与 mark 同一 Ledger 时返回许可字节。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn qualification_fee_on_ledger_for_test(
        &self,
        mark: &native::LedgerMark,
    ) -> Option<usize> {
        self.construction
            .borrow()
            .as_ref()
            .and_then(|c| c.fee_on_ledger_for_test(mark))
    }
    /// 改写已提交资格的 epoch 戳（只为咬住“资格戳≠from”门）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn set_qualification_epoch_for_test(&self, epoch: Option<i64>) {
        self.construction
            .borrow_mut()
            .as_mut()
            .expect("须持有已提交资格")
            .set_writer_epoch_for_test(epoch);
    }
    /// 0037 P2-1 负控注入（比对之前）：完整候选改挂他账，资格所记父账不动；返回原父账句柄。
    /// 此后 recover() 取的交接摘要与候选同为他账（自指比对恒真），只有资格记录值能拒。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn swap_complete_parent_for_test(&self, foreign: native::Budget) -> native::Budget {
        let mut complete = self.complete.borrow_mut();
        let candidate = complete.as_mut().expect("须持有完整候选");
        std::mem::replace(&mut candidate.resources, foreign)
    }
    /// 0037 P2-1 负控注入（比对之前）：资格状态块许可换成他账许可，资格所记父账与候选不动。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn replace_qualification_fee_for_test(&self, fee: native::Permit) {
        let old = self
            .construction
            .borrow_mut()
            .as_mut()
            .expect("须持有已提交资格")
            .replace_fee_for_test(fee);
        assert!(old.is_some(), "须原持有状态块许可");
    }
    /// 0037 P2-1 负控注入（比对之前）：完整候选绕过 audit_admission 重绑点，经同父账真实 handoff
    /// 进入下一冻结域（0041 P3-1：沿用原额度的正额度换域），资格所记域不动。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn advance_complete_domain_for_test(&self) {
        let mut complete = self.complete.borrow_mut();
        let candidate = complete.as_mut().expect("须持有完整候选");
        let allowance = candidate.resources.allowance_for_test();
        assert!(allowance > 0, "须正额度换域");
        let next = candidate
            .resources
            .handoff(u64::MAX, allowance)
            .expect("同父账换域");
        assert!(next.same_parent(&candidate.resources));
        candidate.resources = next;
    }
    /// 0041 P3-3 负控注入（比对之前）：抹掉已提交资格所记父账。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn clear_qualification_parent_for_test(&self) {
        self.construction
            .borrow_mut()
            .as_mut()
            .expect("须持有已提交资格")
            .clear_parent_for_test();
    }
    /// 0041：已提交资格所记父账/冻结域是否恰为 owner 完整候选当前父账。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn qualification_parent_matches_complete_for_test(&self) -> bool {
        let complete = self.complete.borrow();
        let candidate = complete.as_ref().expect("须持有完整候选");
        self.construction
            .borrow()
            .as_ref()
            .expect("须持有已提交资格")
            .parent_matches_for_test(&candidate.resources)
    }
    /// 0041：owner 完整候选当前父账的交接摘要（弱引用，不延长父账寿命）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_parent_stamp_for_test(&self) -> native::BudgetStamp {
        self.complete
            .borrow()
            .as_ref()
            .expect("须持有完整候选")
            .resources
            .handoff_stamp()
            .expect("当前域")
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_parent_matches_for_test(&self, stamp: &native::BudgetStamp) -> bool {
        self.complete
            .borrow()
            .as_ref()
            .is_some_and(|c| stamp.matches(&c.resources))
    }
    /// 0044 P2-1：完整候选的工作额度 `limit`——effects 路径请求峰实比之值（`recording_peak`
    /// 处 `peak > self.limit` 即放弃增量），与冻结域 allowance 同源（重绑时取 complete_workspace_limit）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn complete_limit_for_test(&self) -> usize {
        self.complete.borrow().as_ref().expect("须持有完整候选").limit
    }
    /// 0044 P3-1 负控注入（重绑之前）：资格所记父账/冻结域改指 stamp，候选不动。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn set_qualification_parent_for_test(&self, stamp: native::BudgetStamp) {
        self.construction
            .borrow_mut()
            .as_mut()
            .expect("须持有已提交资格")
            .set_parent_for_test(stamp);
    }
    /// 拿走 owner 的完整候选（无候选）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn drop_complete_for_test(&self) {
        assert!(self.complete.borrow_mut().take().is_some(), "须持有完整候选");
    }
    /// 同父账开启新请求域，候选所持父账句柄随之不再是当前域（无当前父账）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn expire_complete_parent_for_test(&self) {
        let complete = self.complete.borrow();
        let candidate = complete.as_ref().expect("须持有完整候选");
        assert!(candidate.resources.total_limit().is_ok());
        assert!(candidate.resources.handoff(u64::MAX, 0).is_ok(), "同父账换域");
        assert!(candidate.resources.total_limit().is_err());
    }
    /// 测试选择器（同 construction_tests::mint/close_fixed）：在本 owner 上以 raw 增量签发并提交
    /// 一份持有状态块许可（fee=Some）的资格；返回同事务签发的 raw token。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn issue_raw_qualification_for_test(
        &self,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<(OldRawToken, Ended), String> {
        let active = self.begin(TransactionBehavior::Immediate)?;
        active.audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            context,
        )?;
        let issued = active.with_bound_raw_advance(context, |token| {
            token.ensure_current()?;
            Ok(token.into_issued())
        })?;
        active.before_fixed_write(FixedOperation::Accept, Some(&*context))?;
        active
            .tx()
            .execute(
                "UPDATE s_protocol_meta SET next_attempt_ordinal=next_attempt_ordinal WHERE singleton=1",
                [],
            )
            .map_err(invalid)?;
        active.finish_fixed_write()?;
        Ok((OldRawToken(issued), active.commit()?))
    }
    /// 旧 token 改记为当前租约序号（Recover 前当期）。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn restamp_raw_token_for_test(&self, token: OldRawToken) -> OldRawToken {
        OldRawToken(token.0.restamp_for_test(&self.construction_lease))
    }
    /// 以 owner 当前候选原件与租约实际绑定旧 token；成功则交回同一 token 供再次调用。
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn bind_raw_token_for_test(&self, token: OldRawToken) -> Result<OldRawToken, String> {
        let complete = self.complete.borrow();
        let candidate = complete
            .as_ref()
            .ok_or_else(|| invalid("缺当前候选原件"))?;
        let bound = token
            .0
            .bind(candidate.audited.raw.view(), &self.construction_lease)?;
        Ok(OldRawToken(bound.into_issued()))
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn set_recover_after_write_for_test(
        &self,
        hook: fn(&Transaction<'_>) -> Result<(), String>,
    ) {
        RECOVER_AFTER_WRITE.with(|slot| slot.set(Some(hook)));
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

    #[cfg(feature = "s_capture_native_testonly")]
    fn issue_publication_resources(
        &self,
        context: &request_audit::RequestAuditContext,
    ) -> Result<PublicationResources, String> {
        if self.publication_issued.replace(true) || self.dirty.get() {
            return Err(construction_state::stale("publication资源只能写前签发一次"));
        }
        let before = self.construction_seal()?;
        let complete = self.complete.borrow();
        let candidate = complete
            .as_ref()
            .ok_or_else(|| construction_state::stale("publication缺完整父账"))?;
        if candidate.seal != before
            || candidate.limit != context.complete_workspace_limit()
            || !context.matches_complete_budget(&candidate.resources)
        {
            return Err(construction_state::stale("publication父账或冻结额度域改变"));
        }
        Ok(PublicationResources {
            budget: candidate.resources.clone(),
            parent_limit: candidate.limit,
            before,
            connection: unsafe { self.tx().handle() } as usize,
        })
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn check_publication_transition(
        &self,
        before: SnapshotSeal,
        after: SnapshotSeal,
    ) -> Result<(), String> {
        if !self.publication_issued.get()
            || before.life != after.life
            || before.transaction_id != after.transaction_id
            || before.data_version != after.data_version
            || before.schema_version != after.schema_version
            || before.domain != after.domain
            || before.write_sequence.checked_add(1) != Some(after.write_sequence)
            || after != self.construction_seal()?
        {
            return Err(construction_state::stale(
                "publication尾检前后身份或写序不一致",
            ));
        }
        Ok(())
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn check_publication_run(&self, checked: ConstructionRun<()>) -> Result<(), String> {
        let ConstructionRun {
            value: (),
            before,
            after,
        } = checked;
        self.check_publication_transition(before, after)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn with_bound_raw_advance_in_publication<R>(
        &self,
        context: &request_audit::RequestAuditContext,
        parent: Option<&PublicationScope<'_>>,
        consume: impl for<'raw> FnOnce(BoundRawAdvance<'raw>) -> Result<R, String>,
    ) -> Result<R, String> {
        let Some(parent) = parent else {
            return self.with_bound_raw_advance(context, consume);
        };
        let result = (|| {
            parent.check_transaction(self.tx())?;
            if self.dirty.get() || self.construction.borrow().is_some() {
                return Err(construction_state::stale("仅允许本事务首次写前签发一次"));
            }
            let seal = self.construction_seal()?;
            let complete = self.complete.borrow();
            let candidate = complete
                .as_ref()
                .ok_or_else(|| construction_state::stale("缺当前同快照已核原件"))?;
            if seal != parent.resources.before || candidate.seal != seal {
                return Err(construction_state::stale("raw与publication父身份不一致"));
            }
            let previous = self.owner.construction.borrow_mut().take();
            #[cfg(test)]
            parent.trace("raw");
            let bound = BoundOriginals { _private: () };
            let (mut pending, token) = construction_state::PendingRawState::advance(
                previous,
                candidate.audited.raw.view(),
                seal,
                &bound,
                &self.owner.construction_lease,
                &super::audit_store::NativeAdmission(Some(parent.work())),
            )?;
            pending.stamp_parent(&candidate.resources);
            *self.construction.borrow_mut() = Some(pending);
            let value = consume(token)?;
            if self.owner.construction_lease.stale.get() {
                return Err(construction_state::stale("消费者已撤销raw租约"));
            }
            Ok(value)
        })();
        if result.is_err() {
            self.owner.invalidate_construction();
            drop(self.construction.borrow_mut().take());
        }
        result
    }

    /// #1488：唯一生产构造点仍封闭在 owner；调用者不能传 raw、prefix 或成功布尔量。
    /// 这里只授 raw 变化资格；正式 Runtime 的集合/来源完整性仍须独立 seal。
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
            if retained > context.complete_retention_cap()
                || !context.matches_complete_budget(&candidate.resources)
            {
                return Err(construction_state::stale("raw增量工作额度或父域不符"));
            }
            // 独立入口也沿候选真实父账；旧candidate/raw费用已持有，不能重新seed。
            let budget = candidate.resources.clone();
            let mut _inline = budget
                .reserve(sum([
                    std::mem::size_of::<construction_state::PendingRawState>(),
                    std::mem::size_of::<BoundRawAdvance<'_>>(),
                ]))
                .map_err(|_| construction_state::stale("raw增量工作额度不足"))?;
            // #1488 v4（反例D）：PendingRawState部分从同一预留拆出，随状态持有；token部分在出口归还。
            let state_fee = _inline
                .split(std::mem::size_of::<construction_state::PendingRawState>())
                .map_err(|_| construction_state::stale("raw状态许可拆分失配"))?;
            let work = budget
                .codec_work()
                .map_err(|_| construction_state::stale("raw增量分配许可不足"))?;
            work.run(|scope| {
                let bound = BoundOriginals { _private: () };
                let (mut pending, token) = construction_state::PendingRawState::advance(
                    previous,
                    candidate.audited.raw.view(),
                    seal,
                    &bound,
                    &self.owner.construction_lease,
                    &super::audit_store::NativeAdmission(Some(scope)),
                )?;
                pending.attach_fee(state_fee);
                pending.stamp_parent(&budget);
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
        if self.owner.construction.borrow().is_some()
            && (!matches!(mode, s_allocation::Mode::CapturedNative)
                || !context.source_cache_enabled())
        {
            self.owner.invalidate_construction();
            return Err(construction_state::stale(
                "已有增量游标不能切回独立完整审计模式",
            ));
        }
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
                    let resources = match context.bind_complete_budget(&candidate.resources) {
                        Ok(resources) => resources,
                        Err(error) => {
                            *self.owner.complete.borrow_mut() = Some(candidate);
                            return Err(error);
                        }
                    };
                    // #1488 0041（主控裁定 (a)）：唯一合法重绑点——候选已经同父校验交接到本请求冻结域，
                    // owner 已提交资格所记冻结域随之前进（父账 id 不变）；其他任何路径不改记录。
                    if let Some(state) = self.owner.construction.borrow_mut().as_mut() {
                        state.advance_parent(&candidate.resources, &resources);
                    }
                    candidate.resources = resources;
                    candidate.limit = context.complete_workspace_limit();
                    candidate.seal = seal;
                    candidate.charge.entry_reserved_bytes = context.complete_entry_reserved_bytes();
                    #[cfg(test)]
                    state::trace_event("owner_complete_hit", None);
                    *self.complete.borrow_mut() = Some(candidate);
                    return Ok(());
                }
                // 已持有发布游标的owner不能在输入身份失效后冷重签；必须显式重建owner/runtime。
                if self.owner.construction.borrow().is_some() {
                    drop(candidate);
                    self.owner.invalidate_construction();
                    return Err(construction_state::stale(
                        "已有增量游标的材料域改变；禁止冷重签",
                    ));
                }
                // 尚未建立增量游标的独立冷入口保持既有完整审计。
                drop(candidate);
            }
            if self.owner.construction.borrow().is_some() {
                self.owner.invalidate_construction();
                return Err(construction_state::stale(
                    "已有增量游标丢失原件候选；禁止完整核回退",
                ));
            }
            // #1488 v4 ColdBuildGuard前移：无请求父时，在冷候选第一分配（审计/capture/slot）之前
            // 由本次冻结唯一BootstrapGrant出生根账；此后与请求父同路，在根账活工作域内逐项持票出生。
            // capture不再接收已分配裸载荷后事后MoveIn。根内审计失败（含额度不足）时根账整体析构，
            // 按旧口径无票审计且本次不留候选（与旧capture“超限不留候选”同结果）；真实校验错由重跑原样返回。
            let cold_root = if known && context.request_parent_budget().is_none() {
                context
                    .take_bootstrap_grant()
                    .and_then(|grant| native::Budget::bootstrap(grant).ok())
            } else {
                None
            };
            let (audited, cold_root) = match cold_root {
                Some(root) => {
                    let base = root.live();
                    let first = self.audit_required_in_parent(mode, context, Some(&root));
                    #[cfg(test)]
                    let first = first.and_then(|a| cold_root_probe::inject_failure().map(|()| a));
                    match first {
                        Ok(audited) => (audited, Some(root)),
                        Err(_) => {
                            drop(context.take());
                            // #1488 P3-1：失败首跑在根工作域内经 legacy cache 保留的 NativeArc 仍持根许可；
                            // 根析构前整体清除跨请求保留（缓存只影响复用，不改判据），不留挂在已析构根上的许可。
                            #[cfg(test)]
                            cold_root_probe::leak_if_armed(&root);
                            let residue = root.live().saturating_sub(base);
                            if residue != 0 {
                                crate::cache::clear_handles();
                            }
                            // #1488 P3-d：清除后复核根残留归零；仍有许可即账不平，不析构根后带着
                            // 来源不明的根许可无票重跑，直接报错。
                            // #1488 P3-δ（主控裁定接受）：本分支fail-closed，不在此处或上层自动重试；本请求在
                            // 准入阶段被拒、未提交。运维处置：重启writer进程（重建根账、清空跨请求缓存）后重发；
                            // 账不平属实现缺陷，重启后同请求复现即保留该错误与请求转人工检查账目。
                            let left = root.live().saturating_sub(base);
                            #[cfg(test)]
                            cold_root_probe::note(residue, left);
                            if left != 0 {
                                return Err(format!(
                                    "S3WriterResourcesInvalid：冷根审计失败清除保留后根上仍残留{left}B许可\
                                     （账不平，fail-closed，不自动重试；本请求未提交。处置：重启writer进程后重发，\
                                     重启后复现请保留本错误与请求转人工检查账目）"
                                ));
                            }
                            drop(root);
                            #[cfg(test)]
                            cold_root_probe::note_rerun();
                            (self.audit_required_in_parent(mode, context, None)?, None)
                        }
                    }
                }
                None => (self.audit_required_in_parent(mode, context, None)?, None),
            };
            let parent = context.request_parent_budget().or(cold_root);
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
                        Ok(mut candidate) => {
                            candidate.resources =
                                context.bind_complete_budget(&candidate.resources)?;
                            context.note_complete_source_work(
                                candidate
                                    .slot
                                    .as_ref()
                                    .and_then(|s| s.source_pending.as_ref()),
                            );
                            *self.complete.borrow_mut() = Some(candidate);
                        }
                        Err(slot) => {
                            context.restore_slot(slot);
                            context.note_slot_home(parent.as_ref());
                        }
                    }
                }
            }
            return Ok(());
        }
        self.audit_required_in_parent(mode, context, None)?;
        Ok(())
    }

    fn audit_required_in_parent(
        &self,
        mode: &s_allocation::Mode,
        context: &mut request_audit::RequestAuditContext,
        cold_root: Option<&ColdRoot>,
    ) -> Result<state::AuditedState, String> {
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let _ = cold_root;
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(parent) = context
            .request_parent_budget()
            .or_else(|| cold_root.cloned())
        {
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

/// #1488 Recover 锁：同事务签发后离开事务的旧 raw token（不透明）。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) struct OldRawToken(construction_state::IssuedRawAdvance);
#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RecoverProbe {
    pub(super) stale: bool,
    pub(super) serial: u64,
    pub(super) retired: bool,
    pub(super) qualification: Option<RecoverQualification>,
}
#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RecoverQualification {
    pub(super) writer_epoch: Option<i64>,
    pub(super) has_fee: bool,
    pub(super) binding: RawAdvanceBinding,
    pub(super) retained: usize,
    pub(super) has_parent: bool,
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
    limit: usize,
    // raw载荷必须先于Q持有的Plain raw许可析构。
    audited: state::AuditedState,
    q: q::Facts,
    // Q 全集：所有非节点表、schema/完整配置/clock/raw尾/控制关系，不是只有 H 的绑定。
    originals: native::Tracked<Vec<request_audit::ExactRows>>,
    charge: CompleteCharge,
    _inline_permit: native::Permit,
    // 最后析构：冷审计时的同一父账随所有真实载荷跨事务保留。
    resources: native::Budget,
}
#[cfg(feature = "s_capture_native_testonly")]
impl CompleteCandidate {
    fn refresh_capture_charge(&mut self) -> Result<(), String> {
        use super::audit_retention::sum;
        let capture = self
            .capture
            .as_ref()
            .ok_or_else(|| invalid("缺Native owner原件"))?;
        let r = capture
            .native_retention_numbers_for_testonly()
            .ok_or_else(|| invalid("缺Native原件费用"))?;
        self.node_count = r.nodes;
        self.charge.node_source_and_certificates = sum([
            r.body_len,
            r.nodes.saturating_mul(97),
            8 + newchan_rust::s_content::CONTENT_FORMAT.len() + 4 + 2 * (1 + 1 + 8),
            sum([r.public_roots, r.template_roots]).saturating_mul(32),
        ]);
        self.charge.node_capacity = sum([
            r.body_capacity,
            r.index_payload.saturating_mul(2),
            r.index_capacity.saturating_mul(2).saturating_add(16),
            sum([r.public_capacity, r.template_capacity])
                .saturating_mul(66)
                .saturating_add(32),
            capture.owner_codec_retained_bytes_for_testonly(),
        ]);
        self.charge.total = sum([
            self.charge.node_source_and_certificates,
            self.charge.node_capacity,
            self.charge.historical_and_raw_capacity,
            self.charge.historical_binding_capacity,
            self.charge.slot_result_capacity,
            self.charge.source_pool_charge,
            self.charge.current_sql_capacity,
            self.charge.q_capacity,
            self.charge.inline,
        ]);
        Ok(())
    }
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
        // #1488 v4：根账（请求父或ColdBuildGuard前移出生的冷根）必须先于候选第一分配存在，
        // 全部既有载荷已在其活工作域内持票出生；无父不建无票父，也不事后MoveIn追认。
        let Some(root) = parent.cloned() else {
            return Ok(Err(slot));
        };
        let parent = Some(&root);
        let inline_permit = match parent
            .map(|b| b.reserve(std::mem::size_of::<Self>()))
            .transpose()
        {
            Ok(permit) => permit,
            Err(_) => return Ok(Err(slot)),
        };
        let mut originals = native::Tracked::new(Vec::<request_audit::ExactRows>::new());
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
        let actual_outer =
            super::audit_retention::vec_bytes::<request_audit::ExactRows>(originals.capacity());
        originals.attach(outer_permit.map(|permit| permit.finish(actual_outer)));
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
            let _sql_permit = match parent.map(|b| b.reserve(requested)).transpose() {
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
            // sql_permit在sql之前声明，错误及正常出口均先销毁sql。
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
        let budget = root.clone();
        let inline_permit = inline_permit.ok_or_else(|| invalid("候选初始许可缺失"))?;
        if !originals.has_permit() {
            return Err(invalid("原件容器缺当前根许可"));
        }
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
            resources: budget,
            _inline_permit: inline_permit,
        }))
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
impl AuditConnection {
    pub(super) fn complete_pointers(&self) -> (usize, usize, usize) {
        let candidate = self.complete.borrow();
        let c = candidate.as_ref().unwrap();
        (
            match &c.audited.raw {
                super::business_json::RawEvents::Plain(rows) => rows.as_ptr() as usize,
                super::business_json::RawEvents::Native(rows) => {
                    rows.as_array().unwrap().as_ptr() as usize
                }
                #[cfg(feature = "s_allocation_json_testonly")]
                super::business_json::RawEvents::Owned(_) => {
                    panic!("complete_pointers不接受Owned原始账")
                }
            },
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
        source_check: Option<&OwnerSourceCheck>,
        parent: Option<&PublicationScope<'_>>,
    ) -> Result<Option<Self>, String> {
        use super::audit_retention::sum;
        use effects::Table;
        self.detach_slot()?;
        let retained_at_entry = self.charge.total;
        // 共享父账的峰已含 candidate；journal 局部帽仍由 Recording 单独检查。
        // Budget 允许准入恰等于 limit 的预留，不能把一次满额预留再判作越界。
        let recording_peak = if effects.shared_parent {
            let peak = self.resources.request_peak();
            // limit 已扣除外层 raw/source/set；全父账峰必须和完整父帽比较，
            // 否则会对已经计入同账的载荷再次扣费。
            if peak > self.limit {
                return Ok(None);
            }
            peak
        } else {
            let Some(peak) = retained_at_entry.checked_add(effects.peak) else {
                return Ok(None);
            };
            if peak >= limit {
                return Ok(None);
            }
            peak
        };
        let budget = match parent {
            Some(parent) => {
                parent.check_transaction(tx)?;
                #[cfg(test)]
                parent.trace("validation");
                parent.budget().clone()
            }
            None => self.resources.clone(),
        };
        let capture = self.capture.as_mut().unwrap();
        let mut append_nodes = |scope: &newchan_rust::s_content::capture_allocation::scratch::WorkScope| -> Result<(), String> {
            for change in effects
                .changes
                .iter_mut()
                .filter(|c| c.table == Table::Node)
            {
                let row = change.new.as_mut().ok_or_else(|| invalid("缺新增节点"))?;
                let digest = match &row[1] {
                    request_audit::SqlCell::Blob(b) => b.as_ref().try_into().map_err(invalid)?,
                    _ => return Err(invalid("digest 非 BLOB")),
                };
                let tag = u8::try_from(effects::integer(&row[2])?).map_err(invalid)?;
                let body = match std::mem::replace(&mut row[3], request_audit::SqlCell::Null) {
                    request_audit::SqlCell::Blob(b) => b,
                    _ => return Err(invalid("body 非 BLOB")),
                };
                let existed = capture.contains(&digest);
                if !capture
                    .append_original_codec_for_owner_testonly(digest, tag, body, usize::MAX, scope)
                    .map_err(invalid)?
                {
                    return Err(invalid("Native 节点许可未被消费"));
                }
                if !existed {
                    self.node_count += 1;
                }
            }
            Ok(())
        };
        let node_result = match parent {
            Some(parent) => append_nodes(parent.work()).map_err(native::Failure::Invalid),
            None => match budget.codec_work() {
                Ok(work) => work.run(append_nodes).map_err(native::Failure::from),
                Err(error) => Err(error),
            },
        };
        match node_result {
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
            let row_ids = |table| -> Result<_, native::BudgetExceeded> {
                let mut rows = budget.empty::<Vec<i64>>()?;
                budget.vec_grow(&mut rows, effects.rows(table).count())?;
                rows.extend(effects.rows(table).map(|change| change.rowid));
                Ok(rows.into_parts())
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
            struct RowIds {
                objects: Vec<i64>,
                auxiliary: [Vec<i64>; 3],
                _fees: [native::Permit; 4],
            }
            let row_ids = match row_ids {
                Ok(((objects, objects_fee), [(a0, f0), (a1, f1), (a2, f2)])) => RowIds {
                    objects,
                    auxiliary: [a0, a1, a2],
                    _fees: [objects_fee, f0, f1, f2],
                },
                Err(_) => return Ok(None),
            };
            #[cfg(test)]
            eprintln!(
                "owner actual suffix g={g} nodes={} object_effects={} aux_effects={:?}",
                self.node_count,
                row_ids.objects.len(),
                row_ids.auxiliary.each_ref().map(Vec::len)
            );
            // publication 借同一父scope；其它既有固定写路径保留独立调用合同。
            let mut append_cut =
                |scope: &newchan_rust::s_content::capture_allocation::scratch::WorkScope| {
                    self.audited.append_owner_cut(
                        tx,
                        capture,
                        g,
                        batch,
                        ns,
                        &row_ids.objects,
                        &row_ids.auxiliary,
                        &bound,
                        scope,
                        &budget,
                        source_check,
                    )
                };
            let updated = match parent {
                Some(parent) => append_cut(parent.work()).map_err(native::Failure::from),
                None => match budget.codec_work() {
                    Ok(work) => work.run(append_cut).map_err(native::Failure::from),
                    Err(error) => Err(error),
                },
            };
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
            // 这些行号只在本次 append_cut 借用；真实销毁后归还各 Vec 实际容量。
            drop(row_ids);
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
        self.refresh_capture_charge()?;
        self.charge.effects_peak = self.charge.effects_peak.max(peak);
        self.charge.handoff_peak = self.charge.handoff_peak.max(sum([
            self.charge.entry_reserved_bytes,
            budget.request_peak().max(recording_peak),
        ]));
        if self
            .charge
            .total
            .checked_add(native::Budget::metadata_bytes())
            .is_none_or(|n| n > budget.total_limit().unwrap_or(0))
        {
            return Ok(None);
        }
        self.charge.incremental_steps += 1;
        self.resources = budget;
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

#[derive(Clone)]
enum FixedOperation {
    Accept,
    Begin,
    Publish,
    // #1488：Recover 携带本事务读到的持久换代证明；只有它能触发受控同owner迁移。
    // 非 Native 构建无构造资格、不迁移，证明只经构造门（RecoverHandoff::new）生效、不被读出。
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code))]
    Recover(construction_state::RecoverHandoff),
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
            // 交接点：候选移入owner；请求收尾的finish_source_keeper会在静止点重读覆盖。
            self.mark_quiescent();
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
            let reject_warm = || {
                if self.owner.construction.borrow().is_some() {
                    self.owner.invalidate_construction();
                    Err(construction_state::stale(
                        "已有增量游标缺连续同域原件；禁止完整核回退",
                    ))
                } else {
                    Ok(false)
                }
            };
            let Some(mut candidate) = self.owner.complete.borrow_mut().take() else {
                return reject_warm();
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
                return reject_warm();
            }
            if let Some(context) = context {
                if !context.source_cache_enabled() {
                    return reject_warm();
                }
                let resources = match context.bind_complete_budget(&candidate.resources) {
                    Ok(resources) => resources,
                    Err(error) => {
                        *self.owner.complete.borrow_mut() = Some(candidate);
                        return Err(error);
                    }
                };
                candidate.resources = resources;
                candidate.limit = context.complete_workspace_limit();
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
        if result.is_err() {
            self.owner.invalidate_construction();
        }
        result
    }
    fn before_fixed_write_inner(
        &self,
        operation: FixedOperation,
        context: Option<&request_audit::RequestAuditContext>,
    ) -> Result<(), String> {
        #[cfg(feature = "s_capture_native_testonly")]
        if self.construction.borrow().is_none()
            && self.owner.construction.borrow().is_some()
            && self.complete.borrow().is_none()
        {
            return Err(construction_state::stale(
                "已发布增量游标缺同快照原件；禁止完整路径回退",
            ));
        }
        // #1488 受控同owner Recover迁移：只在本事务尚无构造、owner 持有已提交资格时进入；
        // 预算沿用候选已冻结的父账（无 context 不重绑、不换账），保留费由下方总容量门核验。
        #[cfg(feature = "s_capture_native_testonly")]
        if let FixedOperation::Recover(handoff) = &operation {
            if self.construction.borrow().is_none() && self.owner.construction.borrow().is_some()
            {
                let complete = self.complete.borrow();
                let seal = self.construction_seal()?;
                let Some(candidate) = complete.as_ref().filter(|c| c.seal == seal) else {
                    return Err(construction_state::stale("Recover迁移缺同快照完整原件"));
                };
                let previous = self.owner.construction.borrow_mut().take();
                // 负控注入：原资格已离开 owner、尚未成为事务构造时交接失败。
                #[cfg(test)]
                if RECOVER_HANDOFF_FAILURE.with(|flag| flag.replace(false)) {
                    drop(previous);
                    return Err(construction_state::stale("测试注入Recover交接失败"));
                }
                let pending = construction_state::PendingRawState::recover(
                    previous,
                    seal,
                    candidate.audited.raw.len(),
                    &self.owner.construction_lease,
                    handoff.clone(),
                    &candidate.resources,
                )?;
                *self.construction.borrow_mut() = Some(pending);
            }
        }
        #[cfg(feature = "s_capture_native_testonly")]
        if matches!(operation, FixedOperation::Accept | FixedOperation::Begin)
            && self.construction.borrow().is_none()
            && context.is_some_and(|c| c.source_cache_enabled())
        {
            let complete = self.complete.borrow();
            if let Some(candidate) = complete.as_ref() {
                let seal = self.construction_seal()?;
                if candidate.seal != seal {
                    return Err(construction_state::stale("Carry缺同快照完整原件"));
                }
                let previous = self.owner.construction.borrow_mut().take();
                let mut pending = construction_state::PendingRawState::carry(
                    previous,
                    seal,
                    candidate.audited.raw.len(),
                    &self.owner.construction_lease,
                )?;
                pending.stamp_parent(&candidate.resources);
                let retained = candidate
                    .charge
                    .total
                    .checked_add(pending.retained_bytes())
                    .ok_or_else(|| construction_state::stale("Carry保留账溢出"))?;
                if retained > context.unwrap().complete_retention_cap() {
                    return Err(construction_state::stale("Carry保留账超过完整工作帽"));
                }
                *self.construction.borrow_mut() = Some(pending);
            }
        }
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
                    .unwrap_or(candidate.limit)
                    .min(candidate.resources.remaining());
                let raw_bytes = self
                    .construction
                    .borrow()
                    .as_ref()
                    .map_or(0, |s| s.retained_bytes());
                // raw旧费属于冻结old_pool；总保留门与本域增量门分别核验。
                if candidate
                    .charge
                    .total
                    .checked_add(raw_bytes)
                    .and_then(|n| n.checked_add(native::Budget::metadata_bytes()))
                    .is_none_or(|n| {
                        n > context.map_or_else(
                            || candidate.resources.total_limit().unwrap_or(0),
                            |c| c.complete_retention_cap(),
                        )
                    })
                {
                    return Err(construction_state::stale("完整保留费超过总容量帽"));
                }
                let op = match operation {
                    FixedOperation::Accept => effects::Operation::Accept,
                    FixedOperation::Begin => effects::Operation::Begin,
                    FixedOperation::Publish => effects::Operation::Publish,
                    FixedOperation::Recover(_) => effects::Operation::Recover,
                };
                candidate.detach_slot()?;
                {
                    let recording_cap = limit;
                    // #1488 P2-2：journal 许可先于其任何分配从候选父账取得并随 journal 持有。
                    if let Some(recording) = effects::Recording::start_on(
                        self.tx(),
                        op,
                        (limit / 8).min(recording_cap),
                        strict,
                        &candidate.resources,
                        candidate.limit,
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
        let result = self.finish_fixed_write_inner(
            #[cfg(feature = "s_capture_native_testonly")]
            None,
        );
        if result.is_err() && self.construction.borrow().is_some() {
            self.owner.invalidate_construction();
        }
        result
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn finish_fixed_write_in_publication(
        &self,
        parent: Option<&PublicationScope<'_>>,
    ) -> Result<(), String> {
        let result = self.finish_fixed_write_inner(parent);
        if result.is_err() && self.construction.borrow().is_some() {
            self.owner.invalidate_construction();
        }
        result
    }
    fn finish_fixed_write_inner(
        &self,
        #[cfg(feature = "s_capture_native_testonly")] parent: Option<&PublicationScope<'_>>,
    ) -> Result<(), String> {
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
            let recording_cap = limit;
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
            let Some(mut candidate) = candidate.validate_effects(
                self.tx(),
                effects,
                validation_limit,
                set.as_ref()
                    .map(|s| s.source_check())
                    .transpose()?
                    .flatten(),
                parent,
            )?
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
                let mut construction = self.construction.borrow_mut();
                let construction = construction.as_mut().unwrap();
                construction.close_effects(self.construction_seal()?, closed_changes)?;
                // 受控 Recover 迁移：精确 effects 已闭合，再以已核 Q 换代结果确认持久 epoch。
                let persisted = candidate.q.last_epoch_to();
                #[cfg(test)]
                let persisted = if RECOVER_CONFIRM_SKEW.with(|flag| flag.replace(false)) {
                    persisted.map(|epoch| epoch + 1)
                } else {
                    persisted
                };
                construction.confirm_recover(persisted)?;
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
    material: &MaterialAccess<'_, '_, '_>,
    root: newchan_rust::s_content::RefView<'_>,
) -> Result<(), String> {
    effects::note_source_cut(material, root)
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn audit_native_source_cut(
    material: &MaterialAccess<'_, '_, '_>,
    capture: &mut newchan_rust::s_content::AuditCapture,
    previous: Option<source_audit::NativeSourceState>,
    root: newchan_rust::s_content::RefView<'_>,
    raw: &BoundRawAdvance<'_>,
    profile: &str,
    rule: &str,
) -> Result<(source_audit::Counts, source_audit::NativeSourceState), String> {
    effects::audit_native_source(material, capture, previous, root, raw, profile, rule)
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn with_construction_material<T>(
    tx: &Transaction<'_>,
    work: Option<&newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
    f: impl FnOnce(Option<&MaterialAccess<'_, '_, '_>>) -> Result<T, String>,
) -> Result<T, String> {
    effects::with_material_access(tx, work, f)
}
#[cfg(feature = "s_capture_native_testonly")]
pub(crate) fn with_construction_material_in_publication<T>(
    tx: &Transaction<'_>,
    parent: &PublicationScope<'_>,
    #[cfg(test)] stage: Option<&crate::staged_nodes::StageOwner<'_>>,
    f: impl FnOnce(Option<&MaterialAccess<'_, '_, '_>>) -> Result<T, String>,
) -> Result<T, String> {
    effects::with_material_in_publication(
        tx,
        parent,
        #[cfg(test)]
        stage,
        f,
    )
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
    let partition = owner.old_pool_partition(None).unwrap();
    let old_pool = partition.old_pool();
    let mut next = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(total),
    );
    next.freeze_manifest_workspace_partition(total, 0, partition)
        .unwrap();
    // #1488 v4 暖路径：选唯一真实候选父；next_parent为同父sponsor付费的只读观察句柄。
    let preparation = next
        .prepare_parent(Some(&owner.complete_budget_for_test()))
        .unwrap();
    let next_parent = preparation.observer().unwrap().clone();
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
    assert_eq!(actual.0, expected.0);
    assert!(parent.live() > before);
    assert!(parent.peak() < 1024 * 1024);
    drop(actual);
    assert_eq!(parent.live(), before);
}

/// #1488 P2-4：封存池逐许可普查（独立于live差额）。`seal`=审计开始至封存后本父账全部
/// 签发/归还事件；`released`=封存池析构期间事件，`inventory`=析构前按持有者逐张列出的池许可
/// （`Pool::native_permit_inventory_for_testonly`，不含不属任何持有者的许可）。
/// C1 残余+逐张存活许可之和=封存后live；C2 无未匹配归还；C3 存活许可均为Native；
/// C4 池析构恰好归还封存存活多重集且不签发；C5（#1488 P2-A主控裁定后）来源合同项每节点一项、
/// 和=正文+digest/tag/证书，且都指到已物理计费的正文块；析构逐张归还多重集=持有者清单多重集
/// （#1488 P3-c：按许可对拍，不再依赖allocator released_bytes交错，结果与分配器时序无关）。
/// 原先C5期望每节点一张同额物理来源许可（r14：803张/111,334B=node_body+97·nodes），那是同一
/// allocation在父账的第二个物理角色；现降为合同角色，只在O的source_cost里计一次。父账封存live
/// 共少127,974B（527,048→399,074）=111,334B（上述来源许可）+16,640B（NativeNode去掉
/// Option<NativeReservation>后索引槽变窄的布局连带，=1040槽×(241−225)B；#1488 P2-β，r14逐项锁定）。
/// 以上为#1405前合同串下的数值；#1488 0053起r14按tb02b合同串长推导①与527,048基准（#1405后+27/+54）。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
fn seal_census_check(
    seal: &[native::CensusEvent],
    residual: usize,
    after_seal: usize,
    inventory: Vec<usize>,
    released: &[native::CensusEvent],
    breakdown: &Value,
    nodes: usize,
    claims: (usize, usize),
    claim_amounts: Vec<usize>,
) -> Result<Value, String> {
    let mut live: Vec<(&'static str, usize)> = Vec::new();
    let mut unmatched = Vec::new();
    for e in seal {
        if e.reserve {
            live.push((e.kind, e.bytes));
        } else if let Some(i) = live.iter().rposition(|x| *x == (e.kind, e.bytes)) {
            live.remove(i);
        } else if let Some(i) = live.iter().rposition(|x| x.0 == e.kind && x.1 > e.bytes) {
            live[i].1 -= e.bytes;
        } else {
            unmatched.push((e.kind, e.bytes));
        }
    }
    let sum: usize = live.iter().map(|x| x.1).sum();
    if residual.checked_add(sum) != Some(after_seal) {
        return Err(format!(
            "C1 普查恒等式失败：{after_seal} != {residual} + {sum}"
        ));
    }
    if !unmatched.is_empty() {
        return Err(format!("C2 未匹配归还 {unmatched:?}"));
    }
    if let Some(x) = live.iter().find(|x| x.0 != "native") {
        return Err(format!("C3 封存存活许可非Native {x:?}"));
    }
    if released.iter().any(|e| e.reserve) {
        return Err("C4 池析构期间签发新许可".into());
    }
    let mut dropped = released
        .iter()
        .map(|e| (e.kind, e.bytes))
        .collect::<Vec<_>>();
    live.sort();
    dropped.sort();
    if dropped != live {
        return Err(format!(
            "C4 池析构归还({}张/{}B)不等于封存存活({}张/{sum}B)",
            dropped.len(),
            dropped.iter().map(|x| x.1).sum::<usize>(),
            live.len()
        ));
    }
    let field = |name: &str| {
        breakdown[name]
            .as_u64()
            .map(|v| v as usize)
            .ok_or(format!("来源成本缺{name}"))
    };
    let expected = field("node_body_bytes")? + field("node_digest_tag_certificate_bytes")?;
    if claims != (nodes, expected) {
        return Err(format!(
            "C5 来源合同项{claims:?}不等于每节点一项({nodes}, {expected})"
        ));
    }
    // 析构归还中不属任何持有者的许可（多出的张）即重复计费；清单多出的张即漏列/错账。
    // 0字节许可（空容器的reserve_native(0)）不动父账，普查不登记，清单同样不计。
    let mut held = inventory;
    held.retain(|bytes| *bytes != 0);
    held.sort_unstable();
    let mut returned = dropped.iter().map(|x| x.1).collect::<Vec<_>>();
    returned.sort_unstable();
    let mut extra = returned.clone();
    for bytes in &held {
        if let Some(i) = extra.iter().position(|x| x == bytes) {
            extra.swap_remove(i);
        }
    }
    // #1488 P3-α：实测节点来源物理许可张数——无持有者的归还中，与某节点来源合同额同额者
    // （多重集逐张匹配）。P2-A后来源只是合同角色，物理上应为0；P2-A前形态即每节点一张。
    let mut amounts = claim_amounts;
    let node_source_physical = extra
        .iter()
        .filter(|bytes| {
            amounts
                .iter()
                .position(|a| a == *bytes)
                .map(|i| amounts.swap_remove(i))
                .is_some()
        })
        .count();
    if returned != held {
        return Err(format!(
            "C5 析构归还({}张/{}B)不等于持有者清单({}张/{}B)：无持有者的归还{}张即重复计费，其中node_source_physical_permits={node_source_physical}",
            returned.len(),
            returned.iter().sum::<usize>(),
            held.len(),
            held.iter().sum::<usize>(),
            extra.len()
        ));
    }
    Ok(
        json!({"permits":live.len(),"sealed_bytes":sum,"node_source_claims":claims.0,"node_source_claim_bytes":claims.1,"node_source_physical_permits":node_source_physical}),
    )
}

/// #1488 P2-4负控：在真实封存路径（finish_source_keeper→retain_completed_admitted→
/// build_bounded_counts_admitted 的同一工作域）注入重复许可；live差额恒等式对重复许可
/// 自洽而照样通过，逐许可普查必须识别。无注入的对照组普查通过。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn p2_4_seal_census_flags_duplicate_permit_on_real_seal_path() {
    use cross_request_source_cache::{SealFault, SEAL_FAULT};
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("p24-source", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let total = 64 * 1024 * 1024;
    let mut baseline = None;
    for fault in [
        None,
        Some(SealFault::SourceCostResign),
        Some(SealFault::ProofsBlockDuplicate),
        Some(SealFault::NodeSourcePermits),
    ] {
        let mut owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut keeper = cross_request_source_cache::Keeper::new(total);
        for request in 0..2 {
            let old_pool = keeper.retained_source_bytes();
            let mut context = request_audit::RequestAuditContext::with_source_keeper(keeper);
            context
                .freeze_manifest_workspace_budget(total, 0, old_pool)
                .unwrap();
            context.enable_cold_admission_for_testonly().unwrap();
            let parent = context.cold_parent_budget().expect("准入冷核持真实父账");
            let residual = parent.live();
            parent.census_arm();
            let active = owner.begin(TransactionBehavior::Deferred).unwrap();
            let audited = parent
                .codec_work()
                .unwrap()
                .run(|scope| {
                    state::audit_native_in_request_admitted(active.tx(), &mut context, Some(scope))
                })
                .unwrap();
            drop(audited);
            let ended = active.read_end().unwrap();
            if request == 1 {
                SEAL_FAULT.with(|armed| armed.set(fault));
            }
            owner.finish_source_keeper(context, ended);
            assert_eq!(
                SEAL_FAULT.with(|armed| armed.get()),
                None,
                "注入须由真实封存消费"
            );
            keeper = owner.take_source_keeper().unwrap();
            let seal_census = parent.census_take();
            let report = keeper.report();
            assert_eq!(report["rejected"], 0);
            assert_eq!(report["retained_counts_proofs"], 1);
            assert_eq!(report["hits"], request);
            if request == 0 {
                continue;
            }
            let after_seal = parent.live();
            let pool = keeper.take().expect("封存池存活");
            let nodes = pool.nodes;
            let claims = pool.originals.native_source_claims_for_testonly().unwrap();
            let claim_amounts = pool.originals.native_source_claim_amounts_for_testonly();
            parent.census_arm();
            let inventory = pool.native_permit_inventory_for_testonly().unwrap();
            drop(pool);
            let released = parent.census_take();
            let sealed = after_seal
                .checked_sub(parent.live())
                .expect("封存池析构不得让父账增长");
            // 旧判据（r14 seal_identity 同式）：残余+封存差额=封存后live，析构后回到残余。
            assert_eq!(residual + sealed, after_seal, "差额恒等式");
            assert_eq!(parent.live(), residual, "析构后回到残余");
            let census = seal_census_check(
                &seal_census,
                residual,
                after_seal,
                inventory,
                &released,
                &report["source_cost_breakdown"],
                nodes,
                claims,
                claim_amounts,
            );
            eprintln!(
                "P24_SEAL_CENSUS {}",
                json!({"fault":format!("{fault:?}"),"sealed":sealed,"census":census.as_ref().map_err(|e|e.as_str())})
            );
            match fault {
                None => {
                    let census = census.unwrap();
                    assert_eq!(census["node_source_physical_permits"], 0, "实测无同额来源物理许可");
                    baseline = Some(sealed);
                }
                Some(fault) => {
                    let error = census.expect_err("重复许可必须被普查识别");
                    assert!(error.starts_with("C5"), "{fault:?}: {error}");
                    let extra = sealed - baseline.unwrap();
                    assert!(extra > 0, "注入确实多签");
                    if fault == SealFault::SourceCostResign {
                        assert_eq!(
                            extra,
                            report["retained_source_cost"].as_u64().unwrap() as usize
                        );
                    }
                    // #1488 P3-α负控：P2-A前形态（每节点一张同额物理来源许可）被实测为nodes张，
                    // 多签恰为来源合同额之和；无注入时实测0（r14与对照组）。
                    if fault == SealFault::NodeSourcePermits {
                        assert!(
                            error.contains(&format!("node_source_physical_permits={nodes}")),
                            "{error}"
                        );
                        assert_eq!(extra, claims.1);
                    }
                }
            }
        }
    }
}

/// #1488 P1-2（复审0026反例）：请求n在真实工作域内把keeper池封存在其请求父账K上（K的物理live
/// 大于池合同额）；请求n+1入口discard候选（生产 held>keeper_cap 分支）后重新分区——K须按其物理
/// live只计一次（含M_K），不能只按合同额。冻结后T的物理占用 E+K物理live+A 不超过T；旧口径
/// （O只含合同额）多放出的A=物理−合同，在此定量复现并锁住。随后请求n+1沿生产路径继续，keeper照常命中。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn p1_2_keeper_pool_on_other_ledger_counts_physical_live_after_discard() {
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("p12-source", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let total = 64 * 1024 * 1024;
    let mut owner = AuditConnection::open(&f.db_path()).unwrap();
    let request = |owner: &mut AuditConnection,
                   keeper: cross_request_source_cache::Keeper,
                   partition: Option<request_audit::OldPoolPartition>| {
        let old_pool = keeper.retained_source_bytes();
        let mut context = request_audit::RequestAuditContext::with_source_keeper(keeper);
        match partition {
            Some(p) => context.freeze_manifest_workspace_partition(total, 0, p).unwrap(),
            None => context.freeze_manifest_workspace_budget(total, 0, old_pool).unwrap(),
        }
        let frozen = context.manifest_workspace_budget().unwrap();
        context.enable_cold_admission_for_testonly().unwrap();
        let parent = context.cold_parent_budget().unwrap();
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let audited = parent
            .codec_work()
            .unwrap()
            .run(|scope| {
                state::audit_native_in_request_admitted(active.tx(), &mut context, Some(scope))
            })
            .unwrap();
        drop(audited);
        let ended = active.read_end().unwrap();
        owner.finish_source_keeper(context, ended);
        frozen
    };
    // 请求n-1、n：第二次请求在其冷父账K上封存出keeper池。
    let mut keeper = cross_request_source_cache::Keeper::new(total);
    for _ in 0..2 {
        request(&mut owner, keeper, None);
        keeper = owner.take_source_keeper().unwrap();
    }
    let contract = keeper.retained_source_bytes();
    assert!(contract > 0);
    let home = keeper.source_home().expect("池封存在请求父账K").clone();
    let physical = home.live();
    let m = native::Budget::metadata_bytes();
    assert!(
        physical > contract + m,
        "反例前提：K物理live {physical} 大于池合同额 {contract} + M"
    );
    // 请求n+1入口：discard候选后重新分区。
    owner.discard_after_error();
    let partition = owner.old_pool_partition(Some(&keeper)).unwrap();
    let group = partition
        .homes()
        .iter()
        .find(|h| h.origin == request_audit::OldPoolPartition::KEEPER_PARENT)
        .expect("keeper池按所在父账单列一组");
    assert_eq!(group.l0, physical, "按池父账物理live计");
    assert_eq!(partition.l0(), physical);
    assert_eq!(partition.old_pool(), contract + m, "O=池合同额+M_K");
    assert_eq!(partition.uncovered(), physical - contract - m, "物理超合同部分列为U");
    let carried = partition.carried();
    assert_eq!(carried, physical, "K物理live恰好计一次");
    // 旧口径：keeper只按合同额进O（无M_K、无U）。
    let old_allowance = total - contract;
    // 冻结后T的物理占用：E(0)+K物理live+A ≤ T。
    let mut probe = request_audit::RequestAuditContext::default();
    probe
        .freeze_manifest_workspace_partition(total, 0, partition.clone())
        .unwrap();
    let allowance = probe.manifest_workspace_budget().unwrap().allowance();
    assert_eq!(carried + allowance, total);
    assert!(physical + allowance <= total, "T物理占用不超E+O+A");
    assert_eq!(old_allowance - allowance, physical - contract, "旧口径多放出物理−合同");
    assert!(old_allowance + physical > total, "旧口径下物理占用逃出T");
    drop(probe);
    // 来源不明：静止点后在K上新挂许可，分区拒绝。
    let stray = home.reserve(3_000).unwrap();
    assert!(owner.old_pool_partition(Some(&keeper)).is_err());
    drop(stray);
    // 请求n+1沿生产路径继续：冻结用该分区，keeper照常命中。
    let frozen = request(&mut owner, keeper, Some(partition));
    assert_eq!(frozen.allowance(), allowance);
    let keeper = owner.take_source_keeper().unwrap();
    assert_eq!(keeper.report()["hits"], 2);
    eprintln!(
        "P12_KEEPER_PHYSICAL {}",
        json!({"contract":contract,"physical":physical,"m":m,"carried":carried,"allowance":allowance,"old_allowance":old_allowance})
    );
}

/// #1488 P1-α（复审0028反例）：请求n在K上封存池（静止点读数x）→ K上出现来源不明许可s →
/// 请求n+1首次分区失败 → discard → 二次分区必须仍拒绝：discard不改K的live，静止点读数只能由
/// 析构实际归还量下调，不得把x+s重读为新基准；s不得吸收进U、不得以该池冻结成功。
/// 生产入口序列（`entry_old_pool_partition`，driver原样调用）最终只能丢可选池。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn p1_alpha_stray_on_keeper_ledger_survives_discard_retry_and_is_rejected() {
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("p1a-source", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let total = 64 * 1024 * 1024;
    let mut owner = AuditConnection::open(&f.db_path()).unwrap();
    let mut keeper = cross_request_source_cache::Keeper::new(total);
    for _ in 0..2 {
        let old_pool = keeper.retained_source_bytes();
        let mut context = request_audit::RequestAuditContext::with_source_keeper(keeper);
        context.freeze_manifest_workspace_budget(total, 0, old_pool).unwrap();
        context.enable_cold_admission_for_testonly().unwrap();
        let parent = context.cold_parent_budget().unwrap();
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let audited = parent
            .codec_work()
            .unwrap()
            .run(|scope| {
                state::audit_native_in_request_admitted(active.tx(), &mut context, Some(scope))
            })
            .unwrap();
        drop(audited);
        let ended = active.read_end().unwrap();
        owner.finish_source_keeper(context, ended);
        keeper = owner.take_source_keeper().unwrap();
    }
    let home = keeper.source_home().expect("池封存在请求父账K").clone();
    let x = home.live();
    // 无stray时对照：同一序列可映射，K组按静止点x计。
    let control = owner.old_pool_partition(Some(&keeper)).unwrap();
    let k_group = |p: &request_audit::OldPoolPartition| {
        p.homes()
            .iter()
            .find(|h| h.origin == request_audit::OldPoolPartition::KEEPER_PARENT)
            .map(|h| (h.attested, h.l0, h.uncovered))
    };
    let (attested, l0, control_uncovered) = k_group(&control).expect("K单列一组");
    assert_eq!((attested, l0), (Some(x), x));
    drop(control);
    // 来源不明许可s挂到K。
    let stray = home.reserve(3_000).unwrap();
    assert_eq!(home.live(), x + 3_000);
    let first = owner.old_pool_partition(Some(&keeper));
    assert!(first.is_err(), "首次分区：K的L0≠静止点读数，必须拒绝");
    owner.discard_after_error();
    assert_eq!(home.live(), x + 3_000, "discard不改K的live");
    let second = owner.old_pool_partition(Some(&keeper));
    let error = second.err().expect("二次分区必须仍拒绝，不得把s洗进静止点");
    assert!(error.contains("来源不明"), "{error}");
    // 生产入口序列：映射不成立只能丢可选池；s不入任何组的U，冻结不得带着该池成功。
    let mut previous = Some(keeper);
    let partition = owner.entry_old_pool_partition(&mut previous, total).unwrap();
    assert!(k_group(&partition).is_none(), "不得以K组冻结");
    assert!(!previous.as_ref().unwrap().has_pool(), "只丢可选池");
    assert_eq!(partition.uncovered(), 0, "s不得吸收进U");
    drop(stray);
    eprintln!(
        "P1A_STRAY_REJECTED {}",
        json!({"x":x,"stray":3_000,"control_uncovered":control_uncovered,"error":error})
    );
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
    // v4合同「不变量」：同一生产路径只有一个真实分配父和一个累计live账；许可随对象移动，
    // 在对象析构以后返还；没有裸字节追认或同额补签。封存后父账live因此必须恰为
    // 残余live加上封存池所持许可之和，且每张封存许可都挂在这一真实父账。
    fn seal_identity(
        parent: &native::Budget,
        residual: usize,
        after_seal: usize,
        permits: &[(&native::Budget, usize)],
    ) -> Result<(), String> {
        if let Some((_, bytes)) = permits.iter().find(|(p, _)| !p.same_parent(parent)) {
            return Err(format!("封存许可{bytes}挂错父账"));
        }
        let sum = permits
            .iter()
            .try_fold(0usize, |sum, (_, bytes)| sum.checked_add(*bytes))
            .ok_or("封存许可之和溢出")?;
        if residual.checked_add(sum) != Some(after_seal) {
            return Err(format!(
                "父账恒等式失败：{after_seal} != {residual} + {sum}"
            ));
        }
        Ok(())
    }
    let mut f = prepared::tests::Fixture::new();
    let (envelope, token) = f.accept_begin(1, ("r14-source", 0, 6, 12, 3, 7), 1);
    f.publish(&envelope, &token).unwrap();
    let total = 64 * 1024 * 1024;
    let mut ordinary_source_cost = 0;
    for admitted in [false, true] {
        let mut owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut keeper = cross_request_source_cache::Keeper::new(total);
        // 上一请求的(父账, 残余, 封存后live)：旧池在本请求收尾时析构，许可须返还到它自己的父账。
        let mut sealed_prior: Option<(native::Budget, usize, usize)> = None;
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
            // 残余=审计前父账已有的真实许可（控制块M）；审计/slot许可都应在封存前后析构返还。
            let residual = parent.as_ref().map(|p| p.live());
            if let Some(parent) = &parent {
                parent.census_arm();
            }
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
            let after_drop = parent.as_ref().map(|p| p.live());
            let ended = active.read_end().unwrap();
            owner.finish_source_keeper(context, ended);
            keeper = owner.take_source_keeper().unwrap();
            if let Some((prior, prior_residual, prior_after_seal)) = sealed_prior.take() {
                // 旧池已被本请求的finish清掉：它的封存许可之和只能从旧父账返还。
                let prior_sealed = prior_after_seal
                    .checked_sub(prior.live())
                    .expect("旧池析构不得让旧父账增长");
                assert!(prior_sealed > 0, "封存新增许可必须留在冷核同父账");
                seal_identity(
                    &prior,
                    prior_residual,
                    prior_after_seal,
                    &[(&prior, prior_sealed)],
                )
                .unwrap();
                if let Some(parent) = &parent {
                    assert!(!prior.same_parent(parent), "两请求各自持有真实父账");
                }
            }
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
                let residual = residual.unwrap();
                let after_seal = parent.live();
                let seal_census = parent.census_take();
                assert!(
                    after_seal <= after_drop.unwrap(),
                    "封存不得对已释放字节事后补签"
                );
                if request == 0 {
                    // 池须留给下一请求命中；其许可在下一请求收尾析构时核账。
                    sealed_prior = Some((parent.clone(), residual, after_seal));
                    continue;
                }
                let pool = keeper.take().expect("封存池存活");
                let nodes = pool.nodes;
                let claims = pool.originals.native_source_claims_for_testonly().unwrap();
                let claim_amounts = pool.originals.native_source_claim_amounts_for_testonly();
                assert_eq!(claim_amounts.len(), nodes);
                let (index_bytes, slot_now, slot_before_p2a) =
                    pool.originals.native_index_layout_for_testonly().unwrap();
                parent.census_arm();
                let inventory = pool.native_permit_inventory_for_testonly().unwrap();
                drop(pool);
                let released = parent.census_take();
                // P2-4：逐许可普查独立于下方live差额恒等式，核封存池无重复计费。
                let census = seal_census_check(
                    &seal_census,
                    residual,
                    after_seal,
                    inventory,
                    &released,
                    &report["source_cost_breakdown"],
                    nodes,
                    claims,
                    claim_amounts,
                )
                .unwrap();
                eprintln!("R14_SEAL_CENSUS {census}");
                let sealed = after_seal
                    .checked_sub(parent.live())
                    .expect("封存池析构不得让父账增长");
                assert!(sealed > 0, "封存新增许可必须留在冷核同父账");
                seal_identity(parent, residual, after_seal, &[(parent, sealed)]).unwrap();
                // #1488 P2-A：父账物理只计正文块一次——封存差额=普查逐张物理许可之和，其中不含
                // 来源合同项（原803张/111,334B同额物理来源许可已降为O内合同角色）。
                assert_eq!(census["sealed_bytes"].as_u64().unwrap() as usize, sealed);
                assert_eq!(census["node_source_physical_permits"], 0);
                assert_eq!(census["node_source_claims"].as_u64().unwrap() as usize, nodes);
                // #1488 P2-β：P2-A前（b6969af4cc，R14/P24_SEAL_CENSUS实测1950张/527,048B）到现在的
                // 封存降幅由两部分组成，逐项可复算：
                // ①来源许可降为合同角色：803张同额物理许可消失，=来源合同额之和
                //   （node_body + 97·803；#1405前111,334B=33,443B+97·803）；
                // ②布局连带：NativeNode的`Option<NativeReservation>`字段换成`Option<SourceClaim>`，
                //   索引许可按槽数×槽宽计，槽宽随之变窄，降幅=索引槽数×(旧槽宽−现槽宽)=16,640B
                //   （实测1040槽×(241−225)B，每槽窄16B；旧槽宽取同布局镜像的size_of，非手填）。
                // 两项之和（#1405前）=127,974B；这是预期变化（索引许可仍按真实元素布局计），不是另有许可消失。
                //
                // #1488 0053 主控裁定(b)（第五处裁改断言，
                // https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6283672661）：
                // ①与P2-A前封存基准随tb02b合同字符串长度推导，不再写死#1405前字面值。字符串节点内容寻址
                // 去重，每个不同串只落一个bytes叶（原字节）+一个8B定宽帧长的STRING节点，故正文只随串长变、
                // 节点数不变。记L=len(schema)+len(policy)+len(knowledge_domain)：
                //   ① = (33,357 + L) + 97·803，33,357=#1405前node_body 33,443 − 旧L 86（10+18+58）；
                //   P2-A前封存 = 526,876 + 2L（正文物理一次 + 同额来源许可一次；旧L→527,048）。
                // #1405：policy `standard_raw_gap_3`→`…;same-price/keep-earlier+extreme-region/1`（+41），
                // knowledge_domain `…without_extreme_identity_competition`→`…with_extreme_root_sets`（−14），
                // schema s-new-bi/1→/2 等长：ΔL=+41−14=+27，①111,334→111,361，P2-A前封存527,048→527,102。
                // L由合同字段len()显式相加；不读产生合同额的SourceClaim/正文计费自我对照。
                const R14_NODES: usize = 803;
                let contract_len = |schema: &str, policy: &str, domain: &str| {
                    schema.len() + policy.len() + domain.len()
                };
                let expected_claims = |l: usize| (33_357 + l) + 97 * R14_NODES;
                let sealed_before_p2a_for = |l: usize| 526_876 + 2 * l;
                // 负控（独立手算基准）：两串还原为#1405前旧值，推导须回到原字面值。
                let legacy_len = contract_len(
                    "s-new-bi/1",
                    "standard_raw_gap_3",
                    "established_direction_without_extreme_identity_competition",
                );
                assert_eq!(legacy_len, 10 + 18 + 58, "#1405前合同串长");
                assert_eq!(
                    expected_claims(legacy_len),
                    111_334,
                    "旧串推导须回到111,334"
                );
                assert_eq!(
                    sealed_before_p2a_for(legacy_len),
                    527_048,
                    "旧串推导须回到527,048"
                );
                let contract = crate::tb02b::CURRENT_CONTRACT;
                let current_len =
                    contract_len(contract.schema, contract.policy, contract.knowledge_domain);
                let sealed_before_p2a = sealed_before_p2a_for(current_len);
                assert_eq!(nodes, R14_NODES, "独立交叉核：来源节点数");
                assert_eq!(index_bytes % slot_now, 0, "索引许可=槽数×现槽宽");
                let index_slots = index_bytes / slot_now;
                let layout_drop = index_slots * (slot_before_p2a - slot_now);
                eprintln!(
                    "R14_P2B_DROP {}",
                    json!({"sealed_before_p2a":sealed_before_p2a,"sealed":sealed,
                        "contract_len":current_len,"claims_bytes":claims.1,"index_bytes":index_bytes,"index_slots":index_slots,
                        "slot_now":slot_now,"slot_before_p2a":slot_before_p2a,"layout_drop":layout_drop})
                );
                assert_eq!(claims.1, expected_claims(current_len), "①来源合同额之和");
                assert_eq!(layout_drop, 16_640, "②索引槽宽连带降幅");
                assert_eq!(
                    sealed_before_p2a - sealed,
                    claims.1 + layout_drop,
                    "封存降幅恰为①+②"
                );

                // 负控一｜重复计费：同父对已封存对象再收一次同额费用，恒等式必须失败。
                let held = parent.reserve(sealed).unwrap();
                let duplicate = parent.reserve(sealed).unwrap();
                assert!(
                    seal_identity(parent, residual, parent.live(), &[(parent, sealed)]).is_err(),
                    "重复计费必须使恒等式失败"
                );
                drop(duplicate);
                seal_identity(parent, residual, parent.live(), &[(parent, sealed)]).unwrap();

                // 负控二｜事后追认：对已分配对象补签许可必须被拒且父账不变；
                // 若把未进账的追认许可算进封存许可，恒等式必须失败。
                let before_adopt = parent.live();
                assert!(parent.adopt(sealed).is_err(), "事后追认必须被拒");
                assert_eq!(parent.live(), before_adopt, "被拒追认不改父账");
                assert!(
                    seal_identity(
                        parent,
                        residual,
                        parent.live(),
                        &[(parent, sealed), (parent, sealed)]
                    )
                    .is_err(),
                    "未进账的追认许可必须使恒等式失败"
                );

                // 负控三｜挂错父账：同额许可落在另一个Budget上，必须被识别为错父。
                let other = native::Budget::new(total, 0).unwrap();
                let foreign = other.reserve(sealed).unwrap();
                assert_eq!(parent.live(), before_adopt, "错父许可不进本父账");
                assert!(
                    seal_identity(parent, residual, parent.live(), &[(&other, sealed)]).is_err(),
                    "挂错父账的封存许可必须被拒"
                );
                drop(foreign);
                drop(held);
                assert_eq!(parent.live(), residual, "负控许可析构后父账回到残余");
            }
        }
        assert!(sealed_prior.is_none(), "旧池须在下一请求核账");
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(super) mod construction_tests {
    use super::*;

    include!("audit_owner/b2_source_fault_tests.rs");

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

    fn b2_envelope(n: i64) -> Value {
        let identity = format!("b2-formal-{n}");
        let row = if n % 2 == 0 {
            (identity.as_str(), n * 10, 8, 16, 6, 12)
        } else {
            (identity.as_str(), n * 10, 8, 14, 4, 7)
        };
        let raw = crate::tests::located_ohlc_input(&[row]);
        let payload = json!({"op":"ingest","target_session_id":"s-session-testonly-001",
            "target_session_generation":"1","writer_epoch":"1","clock_event_id":format!("input-{n}"),"raw_input":raw});
        json!({"schema_revision":"s-session/3","session_id":"s-session-testonly-001",
            "session_generation":"1","source_namespace":"testonly.ohlc","source_epoch":"1",
            "message_id":format!("message-{n}"),"producer_id":"real-content-test","producer_epoch":"1",
            "payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})
    }
    fn b2_accept_begin(
        owner: &AuditConnection,
        envelope: &Value,
        context: &mut request_audit::RequestAuditContext,
    ) -> String {
        let clock = v2::stored_clock(&owner.connection).unwrap();
        let before = owner
            .construction
            .borrow()
            .as_ref()
            .map(|s| (s.binding(), s.facts.prefix_len()));
        let (admission, ended) = owner
            .admit(
                envelope.clone(),
                "1",
                &clock,
                tb02a::PROFILE,
                Some("1"),
                &s_allocation::Mode::CapturedNative,
                context,
            )
            .unwrap();
        assert!(matches!(
            admission,
            Admission::Advance { accepted: true, .. }
        ));
        assert!(owner.connection.is_autocommit());
        owner.continue_with(ended).unwrap();
        owner.assert_complete_matches_cold();
        let after_accept = {
            let state = owner.construction.borrow();
            let state = state.as_ref().expect("正式Accept实际effects已Carry");
            let cursor = (state.binding(), state.facts.prefix_len());
            if let Some(before) = before {
                assert_eq!(cursor, before, "Accept不能推进发布游标");
            } else {
                assert!(cursor.0.is_initial());
                assert_eq!(cursor.1, 0);
            }
            cursor
        };
        // 已AcceptedPending重试只返回既有原件，不重新Accept或推进Carry。
        let (retry, ended) = owner
            .admit(
                envelope.clone(),
                "1",
                &clock,
                tb02a::PROFILE,
                Some("1"),
                &s_allocation::Mode::CapturedNative,
                context,
            )
            .unwrap();
        assert!(matches!(
            retry,
            Admission::Advance {
                accepted: false,
                ..
            }
        ));
        owner.continue_with(ended).unwrap();
        {
            let state = owner.construction.borrow();
            let state = state.as_ref().unwrap();
            assert_eq!((state.binding(), state.facts.prefix_len()), after_accept);
        }
        let begun = owner
            .begin_advance(
                envelope,
                "1",
                &clock,
                &s_allocation::Mode::CapturedNative,
                context,
            )
            .unwrap();
        assert!(owner.connection.is_autocommit());
        owner.assert_complete_matches_cold();
        let state = owner.construction.borrow();
        let state = state.as_ref().unwrap();
        assert_eq!(
            (state.binding(), state.facts.prefix_len()),
            after_accept,
            "Begin不能使A2或S漏过本次raw变化"
        );
        begun.token
    }

    pub(in crate::v3) fn b2_assert_counts_match_independent_full_source(owner: &AuditConnection) {
        let actual = owner
            .construction
            .borrow()
            .as_ref()
            .unwrap()
            .source_counts()
            .unwrap();
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let audited = state::audit(active.tx()).unwrap();
        let cut = audited.cuts.last().unwrap();
        let mut source = crate::content_store::TxStore::new(active.tx()).unwrap();
        let mut reader = super::super::history::HashReader::new(&mut source);
        let expected = source_audit::audit_generated_for_audit(
            &mut reader,
            cut.roots.generated_state.view(),
            audited
                .raw
                .view()
                .prefix(cut.metadata.input_frontier as usize + 1)
                .unwrap(),
            &cut.metadata.profile_id,
            &cut.metadata.rule_revision,
            None,
            None,
        )
        .unwrap();
        assert_eq!(actual, expected, "来源Counts六字段逐项对拍");
        drop(reader);
        drop(source);
        drop(audited);
        active.read_end().unwrap();
    }

    #[test]
    fn b2_native_warm_cold_and_ordinary_match_all_rows_nodes_roots_receipts_and_counts() {
        use super::super::prepared::tests::{
            runtime_root_compare_reachable_bytes, runtime_root_tables, Fixture,
        };
        let scenarios = [
            (
                "shape-and-revision",
                [
                    ("a", 0, 6, 12, 3, 7, 1),
                    ("b", 10, 8, 16, 6, 12, 1),
                    ("c", 20, 8, 13, 4, 7, 1),
                    ("b", 10, 8, 17, 6, 12, 2),
                ],
            ),
            (
                "waiting-direction",
                [
                    ("a", 0, 6, 12, 3, 7, 1),
                    ("b", 10, 6, 12, 3, 7, 1),
                    ("c", 20, 6, 12, 3, 7, 1),
                    ("d", 30, 8, 17, 6, 12, 1),
                ],
            ),
            (
                "low-coordinate",
                [
                    ("a", 20, 6, 12, 3, 7, 1),
                    ("b", 0, 6, 12, 3, 7, 1),
                    ("c", 10, 6, 12, 3, 7, 1),
                    ("d", 30, 8, 17, 6, 12, 1),
                ],
            ),
            (
                "same-price-revision",
                [
                    ("a", 0, 6, 12, 3, 7, 1),
                    ("b", 10, 8, 16, 6, 12, 1),
                    ("c", 20, 8, 13, 4, 7, 1),
                    ("b", 10, 8, 16, 6, 12, 2),
                ],
            ),
            (
                "revision-resolves-waiting",
                [
                    ("a", 0, 6, 12, 3, 7, 1),
                    ("b", 10, 6, 12, 3, 7, 1),
                    ("c", 20, 8, 17, 6, 12, 1),
                    ("a", 0, 4, 9, 1, 6, 2),
                ],
            ),
        ];
        let mut saw_shapes = false;
        let mut saw_witnesses = false;
        let mut saw_requests = false;
        for (scenario, rows) in scenarios {
            let warm = Fixture::new();
            let cold = Fixture::new();
            let mut ordinary = Fixture::new();
            let owner = AuditConnection::open(&warm.db_path()).unwrap();
            let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
            let mut c = context();
            for (offset, (id, coord, open, high, low, close, revision)) in
                rows.into_iter().enumerate()
            {
                let n = offset as i64 + 1;
                let (envelope, plain_token) =
                    ordinary.accept_begin(n, (id, coord, open, high, low, close), revision);
                let token = b2_accept_begin(&owner, &envelope, &mut c);
                source_audit::reset_source_call_stats();
                construction_set::take_owner_source_reuses();
                let (actual, ended) = owner
                    .publish(&mut runtime, &token, &envelope, &mut c)
                    .unwrap();
                assert_eq!(
                    source_audit::source_call_stats(),
                    source_audit::SourceCallStats::default(),
                    "{scenario} gen{n}真实warm入口不能完整重判"
                );
                assert_eq!(construction_set::take_owner_source_reuses(), 1);
                owner.continue_with(ended).unwrap();
                let work = owner
                    .construction
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .source_work()
                    .unwrap();
                saw_shapes |= work.shape_local > 0;
                saw_witnesses |= work.witness_local > 0;
                saw_requests |= work.request_local > 0;
                // 每一轮使用全新cold owner和runtime，既有SQLite原件只走独立冷建。
                let cold_owner = AuditConnection::open(&cold.db_path()).unwrap();
                let mut cold_runtime =
                    prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
                let mut cold_context = context();
                let cold_token = b2_accept_begin(&cold_owner, &envelope, &mut cold_context);
                let (cold_receipt, ended) = cold_owner
                    .publish(&mut cold_runtime, &cold_token, &envelope, &mut cold_context)
                    .unwrap();
                cold_owner.continue_with(ended).unwrap();
                let expected = ordinary.publish(&envelope, &plain_token).unwrap();
                for receipt in [&cold_receipt, &expected] {
                    assert_eq!(
                        canonical_json(&actual.message_receipt),
                        canonical_json(&receipt.message_receipt),
                        "{scenario} gen{n}"
                    );
                    assert_eq!(actual.batch_id, receipt.batch_id);
                    assert_eq!(actual.snapshot, receipt.snapshot);
                    assert_eq!(actual.delta, receipt.delta);
                    assert_eq!(actual.generation, receipt.generation);
                }
                let persisted = runtime_root_tables(&warm.conn, true);
                assert_eq!(
                    persisted,
                    runtime_root_tables(&cold.conn, true),
                    "{scenario} gen{n}warm/cold全表含原件逐字"
                );
                assert_eq!(
                    persisted,
                    runtime_root_tables(&ordinary.conn, true),
                    "{scenario} gen{n}warm/ordinary全表含原件逐字"
                );
                runtime_root_compare_reachable_bytes(&warm.conn, &ordinary.conn);
                owner.assert_complete_matches_cold();
                cold_owner.assert_complete_matches_cold();
                b2_assert_counts_match_independent_full_source(&owner);
                b2_assert_counts_match_independent_full_source(&cold_owner);
                // ready/receipt和已发布消息重试均只读，不能推进raw/source游标。
                let before = {
                    let state = owner.construction.borrow();
                    let state = state.as_ref().unwrap();
                    (
                        state.binding(),
                        state.facts.prefix_len(),
                        state.source_counts(),
                    )
                };
                let clock = v2::stored_clock(&owner.connection).unwrap();
                let ready = json!({"session_id":"s-session-testonly-001","session_generation":"1","payload":{"op":"ready"}});
                let lookup = json!({"session_id":"s-session-testonly-001","session_generation":"1","payload":{"op":"receipt","original_source_namespace":envelope["source_namespace"],"original_source_epoch":envelope["source_epoch"],"original_message_id":envelope["message_id"],"original_payload_hash":envelope["payload_hash"]}});
                for request in [ready, lookup, envelope.clone()] {
                    let (reply, ended) = owner
                        .admit(
                            request,
                            "1",
                            &clock,
                            tb02a::PROFILE,
                            Some("1"),
                            &s_allocation::Mode::CapturedNative,
                            &mut c,
                        )
                        .unwrap();
                    assert!(matches!(reply, Admission::Reply(_)));
                    owner.continue_with(ended).unwrap();
                }
                let after = {
                    let state = owner.construction.borrow();
                    let state = state.as_ref().unwrap();
                    (
                        state.binding(),
                        state.facts.prefix_len(),
                        state.source_counts(),
                    )
                };
                assert_eq!(before, after);
                assert_eq!(persisted, runtime_root_tables(&warm.conn, true));
                eprintln!("B2_NATIVE_TRIPLE scenario={scenario} generation={n} work={work:?}");
            }
            if scenario == "shape-and-revision" {
                let count = |sql| {
                    warm.conn
                        .query_row(sql, [], |r| r.get::<_, i64>(0))
                        .unwrap()
                };
                assert!(
                    count("SELECT count(*) FROM s_content_objects WHERE kind='CC-006.local_shape'")
                        > 0
                );
                assert!(
                    count(
                        "SELECT count(*) FROM s_content_objects WHERE kind!='CC-006.local_shape'"
                    ) > 0
                );
                assert!(
                    count("SELECT count(*) FROM s_content_witnesses") >= 3,
                    "三槽不能靠空集合通过"
                );
                assert!(
                    count("SELECT count(*) FROM s_content_objects WHERE withdrawn_generation=4")
                        > 0
                );
                assert!(
                    count("SELECT count(*) FROM s_content_objects WHERE superseded_by IS NOT NULL")
                        > 0
                );
            }
        }
        assert!(
            saw_shapes && saw_witnesses && saw_requests,
            "必须实际进入非空shape/三槽与CC054请求核"
        );
    }

    #[test]
    fn b2_native_two_nonempty_formal_rounds_match_full_oracle_at_every_commit() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut c = context();
        for n in 2..=3 {
            let envelope = b2_envelope(n);
            let token = b2_accept_begin(&owner, &envelope, &mut c);
            source_audit::reset_source_call_stats();
            construction_set::take_owner_source_reuses();
            let (receipt, ended) = owner
                .publish(&mut runtime, &token, &envelope, &mut c)
                .unwrap();
            assert_eq!(receipt.generation, n);
            let calls = source_audit::source_call_stats();
            assert_eq!(calls.full_source_calls, 0, "正式Publish不能重跑完整来源核");
            assert_eq!(calls.full_pending_calls, 0);
            assert_eq!(calls.fixture_bind_calls, 0);
            assert_eq!(calls.full_members_enumerations, 0);
            assert_eq!(
                construction_set::take_owner_source_reuses(),
                1,
                "原完整投影核必须恰好消费一次真实来源结果"
            );
            owner.continue_with(ended).unwrap();
            owner.assert_complete_matches_cold();
            let state = owner.construction.borrow();
            let state = state.as_ref().expect("只在实际COMMIT成功后晋升");
            assert_eq!(state.facts.prefix_len(), n as usize);
            let work = state.source_work().expect("真实Native来源核必须有跨轮状态");
            assert_eq!(work.full_source_calls, 0);
            assert_eq!(work.full_pending_calls, 0);
            assert_eq!(work.full_members_enumerations, 0);
            assert_eq!(work.raw_hashes, if n == 2 { 8 } else { 4 });
            eprintln!("B2_NATIVE_FORMAL generation={n} source_work={work:?}");
            let persisted: i64 = f
                .conn
                .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
                .unwrap();
            assert_eq!(persisted, n, "另一连接可见实际提交raw");
        }
        drop(runtime);
        drop(owner);
        // 重启不复活缓存token；新owner从完整冷审计重建，再消费0→current。
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        assert!(owner.construction.borrow().is_none());
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut c = context();
        let envelope = b2_envelope(4);
        let token = b2_accept_begin(&owner, &envelope, &mut c);
        let (receipt, ended) = owner
            .publish(&mut runtime, &token, &envelope, &mut c)
            .unwrap();
        assert_eq!(receipt.generation, 4);
        owner.continue_with(ended).unwrap();
        owner.assert_complete_matches_cold();
    }

    #[test]
    fn b2_native_formal_failure_rollback_and_commit_failure_revoke_all_state() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        thread_local! {
            static AFTER_PREPARE_HITS: Cell<usize> = const { Cell::new(0) };
        }
        let mut completed = Vec::new();
        for failure in ["consumer", "unknown-write", "commit"] {
            let f = fixture();
            let owner = AuditConnection::open(&f.db_path()).unwrap();
            let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
            let mut c = context();
            let envelope = b2_envelope(2);
            let token = b2_accept_begin(&owner, &envelope, &mut c);
            let (_, ended) = owner
                .publish(&mut runtime, &token, &envelope, &mut c)
                .unwrap();
            owner.continue_with(ended).unwrap();
            assert!(owner
                .construction
                .borrow()
                .as_ref()
                .unwrap()
                .source_work()
                .is_some());
            let envelope = b2_envelope(3);
            let token = b2_accept_begin(&owner, &envelope, &mut c);
            let persisted_before =
                super::super::prepared::tests::runtime_root_tables(&f.conn, true);
            AFTER_PREPARE_HITS.with(|hits| hits.set(0));
            let commit_hits = Arc::new(AtomicUsize::new(0));
            if failure == "consumer" {
                runtime.set_after_prepare_for_request_test(Some(|_| {
                    AFTER_PREPARE_HITS.with(|hits| hits.set(hits.get() + 1));
                    Err("B2TestConsumerRejected".into())
                }));
            } else if failure == "unknown-write" {
                runtime.set_after_prepare_for_request_test(Some(|tx| {
                    AFTER_PREPARE_HITS.with(|hits| hits.set(hits.get() + 1));
                    let before = tx.total_changes();
                    let affected = tx.execute("UPDATE s_content_objects SET object_revision=object_revision WHERE rowid=(SELECT min(rowid) FROM s_content_objects)", []).map_err(invalid)?;
                    assert_eq!(affected, 1, "必须实际改写一行，不能空表假通过");
                    assert_eq!(tx.total_changes() - before, 1);
                    Ok(())
                }));
            } else {
                runtime.set_after_prepare_for_request_test(Some(|_| {
                    AFTER_PREPARE_HITS.with(|hits| hits.set(hits.get() + 1));
                    Ok(())
                }));
                let seen = commit_hits.clone();
                owner
                    .connection
                    .commit_hook(Some(move || {
                        seen.fetch_add(1, Ordering::SeqCst);
                        true
                    }))
                    .unwrap();
            }
            let error = owner
                .publish(&mut runtime, &token, &envelope, &mut c)
                .err()
                .expect("注入的故障必须拒绝Publish");
            AFTER_PREPARE_HITS.with(|hits| {
                assert_eq!(hits.get(), 1, "{failure}必须在真实来源核成功后到达注入点")
            });
            match failure {
                "consumer" => assert_eq!(error, "B2TestConsumerRejected"),
                "unknown-write" => assert_eq!(
                    error,
                    "StaleWriter：construction资格失效：业务SQL不属于stage"
                ),
                "commit" => {
                    assert_eq!(commit_hits.load(Ordering::SeqCst), 1);
                    assert!(error.starts_with("PublicationInvalid："), "{error}");
                }
                _ => unreachable!(),
            }
            owner.connection.commit_hook(None::<fn() -> bool>).unwrap();
            assert!(owner.connection.is_autocommit());
            assert!(owner.construction.borrow().is_none());
            assert!(owner.complete.borrow().is_none());
            assert!(owner.construction_lease.stale.get());
            assert!(runtime.visit_runtime_roots().unwrap().is_empty());
            assert_eq!(
                persisted_before,
                super::super::prepared::tests::runtime_root_tables(&f.conn, true),
                "{failure}全部表和原件逐字回滚到本次Publish之前"
            );
            assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 2);
            assert_eq!(
                f.conn
                    .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                3,
                "Publish回滚保留已提交Accept原件"
            );
            if failure == "commit" {
                assert!(owner.retired.get(), "COMMIT失败后owner必须退役");
                assert_eq!(
                    owner
                        .begin(TransactionBehavior::Immediate)
                        .err()
                        .expect("退役owner必须拒绝新事务"),
                    "S3DriverInvalid：连接已退役或存在未终结事务"
                );
            } else {
                assert!(!owner.retired.get(), "{failure}回滚后owner不得退役");
                let active = owner.begin(TransactionBehavior::Immediate).unwrap();
                assert!(
                    active.with_bound_raw_advance(&c, |_| Ok(())).is_err(),
                    "失败token不可重签"
                );
            }
            completed.push(failure);
        }
        assert_eq!(completed, ["consumer", "unknown-write", "commit"]);
    }

    #[test]
    fn b2_native_warm_missing_or_stale_originals_reject_before_full_source() {
        for failure in [
            "admit-missing",
            "admit-stale",
            "bind-missing",
            "bind-stale",
            "mode-change",
        ] {
            let f = fixture();
            let owner = AuditConnection::open(&f.db_path()).unwrap();
            let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
            let mut c = context();
            let envelope = b2_envelope(2);
            let token = b2_accept_begin(&owner, &envelope, &mut c);
            let (_, ended) = owner
                .publish(&mut runtime, &token, &envelope, &mut c)
                .unwrap();
            owner.continue_with(ended).unwrap();
            if failure.ends_with("missing") {
                drop(owner.complete.borrow_mut().take());
            } else if failure.ends_with("stale") {
                owner
                    .complete
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .seal
                    .write_sequence += 1;
            } else {
                c = request_audit::RequestAuditContext::default();
            }
            source_audit::reset_source_call_stats();
            let active = owner.begin(TransactionBehavior::Immediate).unwrap();
            let error = if failure.starts_with("bind") {
                active.bind_existing(Some(&c)).unwrap_err()
            } else {
                active
                    .audit_admission(&b2_envelope(3), &s_allocation::Mode::CapturedNative, &mut c)
                    .unwrap_err()
            };
            assert!(error.starts_with("StaleWriter："), "{failure}: {error}");
            let calls = source_audit::source_call_stats();
            assert_eq!(calls.full_source_calls, 0, "{failure}");
            assert_eq!(calls.full_pending_calls, 0, "{failure}");
            assert_eq!(calls.fixture_bind_calls, 0, "{failure}");
            assert_eq!(calls.full_members_enumerations, 0, "{failure}");
            assert!(owner.construction_lease.stale.get());
            drop(active);
            assert!(owner.construction.borrow().is_none());
            assert!(owner.complete.borrow().is_none());
            assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 2);
        }
    }

    #[test]
    fn b2_native_carry_budget_rejects_without_losing_committed_frontier() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let mut c = context();
        let envelope = b2_envelope(2);
        let token = b2_accept_begin(&owner, &envelope, &mut c);
        let (_, ended) = owner
            .publish(&mut runtime, &token, &envelope, &mut c)
            .unwrap();
        owner.continue_with(ended).unwrap();
        let mut small = request_audit::RequestAuditContext::with_source_keeper(
            cross_request_source_cache::Keeper::new(1024),
        );
        small.freeze_manifest_workspace_budget(1024, 0, 0).unwrap();
        let clock = v2::stored_clock(&owner.connection).unwrap();
        assert!(owner
            .admit(
                b2_envelope(3),
                "1",
                &clock,
                tb02a::PROFILE,
                Some("1"),
                &s_allocation::Mode::CapturedNative,
                &mut small
            )
            .is_err());
        assert!(owner.connection.is_autocommit());
        assert!(owner.construction_lease.stale.get());
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 2);
        assert_eq!(
            f.conn
                .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
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
            publication: None,
            transaction: active.transaction.as_mut().unwrap(),
            audited: pending.as_ref().map(|candidate| &candidate.audited),
            raw: None,
            capture: None,
            source: None,
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
    // NOT_RUN：新增回归本轮只静态检查。
    // #1488：独立context重新冻结，失败必须确实到达尾检或真实COMMIT。
    fn cross_request_publication_case(fail_tail: bool, fail_commit: bool) {
        let mut f = fixture();
        let (envelope, token) = f.accept_begin(2, ("scope-next", 10, 8, 16, 6, 12), 1);
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut first = context();
        let cold = owner.begin(TransactionBehavior::Deferred).unwrap();
        cold.audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut first,
        )
        .unwrap();
        owner.continue_with(cold.read_end().unwrap()).unwrap();
        let before = owner.complete.borrow().as_ref().unwrap().resources.clone();
        let peak = before.peak();
        let mut next = request_audit::RequestAuditContext::with_source_keeper(
            cross_request_source_cache::Keeper::new(536_870_912),
        );
        next.freeze_manifest_workspace_partition(
            536_870_912,
            0,
            owner.old_pool_partition(None).unwrap(),
        )
        .unwrap();
        let commits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = commits.clone();
        owner
            .connection
            .commit_hook(Some(move || {
                seen.fetch_add(1, Ordering::SeqCst);
                fail_commit
            }))
            .unwrap();
        PUBLICATION_TAIL_FAILURE.with(|flag| flag.set(fail_tail));
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let result = owner.publish(&mut runtime, &token, &envelope, &mut next);
        assert!(
            !PUBLICATION_TAIL_FAILURE.with(Cell::get),
            "必须实际消费尾检故障"
        );
        if fail_tail || fail_commit {
            assert!(result.is_err());
            assert_eq!(commits.load(Ordering::SeqCst), usize::from(fail_commit));
            assert_eq!(meta_i64(&owner.connection, "generation").unwrap(), 1);
            assert!(owner.construction_lease.stale.get());
        } else {
            let (receipt, ended) = result.unwrap();
            assert_eq!(receipt.generation, 2);
            owner.continue_with(ended).unwrap();
            let complete = owner.complete.borrow();
            let after = &complete.as_ref().unwrap().resources;
            assert!(after.same_parent(&before));
            assert!(next.matches_complete_budget(after));
            assert!(after.peak() >= peak);
            assert!(after.live() > 0);
        }
        owner.connection.commit_hook(None::<fn() -> bool>).unwrap();
    }
    #[test]
    fn cross_request_ready_first_publication_preserves_real_parent() {
        cross_request_publication_case(false, false);
    }
    #[test]
    fn cross_request_publication_tail_failure_cannot_commit() {
        cross_request_publication_case(true, false);
    }
    #[test]
    fn cross_request_actual_commit_failure_cannot_promote() {
        cross_request_publication_case(false, true);
    }

    #[test]
    fn publication_single_parent_scope_reaches_raw_runtime_material_and_validation() {
        publication_scope_case(false);
    }
    #[test]
    fn publication_resource_tail_failure_prevents_actual_sql_commit() {
        publication_scope_case(true);
    }
    fn publication_scope_case(fail_tail: bool) {
        let mut f = fixture();
        let (envelope, token) = f.accept_begin(2, ("scope-next", 10, 8, 16, 6, 12), 1);
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut context = context();
        let cold = owner.begin(TransactionBehavior::Deferred).unwrap();
        cold.audit_admission(
            &json!({"payload":{"op":"ready"}}),
            &s_allocation::Mode::CapturedNative,
            &mut context,
        )
        .unwrap();
        let ended = cold.read_end().unwrap();
        owner.continue_with(ended).unwrap();
        let commits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = commits.clone();
        owner
            .connection
            .commit_hook(Some(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                false
            }))
            .unwrap();
        PUBLICATION_SCOPE_TRACE.with(|trace| trace.borrow_mut().clear());
        PUBLICATION_TAIL_FAILURE.with(|flag| flag.set(fail_tail));
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let result = owner.publish(&mut runtime, &token, &envelope, &mut context);
        assert!(
            !PUBLICATION_TAIL_FAILURE.with(Cell::get),
            "故障必须实际到达尾检，不能提前失败冒充负控"
        );
        let trace = PUBLICATION_SCOPE_TRACE.with(|trace| std::mem::take(&mut *trace.borrow_mut()));
        for phase in ["raw", "runtime", "material", "validation", "tail"] {
            assert_eq!(
                trace.iter().filter(|(name, _)| *name == phase).count(),
                1,
                "{phase}: {trace:?}"
            );
        }
        assert!(
            trace.iter().all(|(_, scope)| *scope == trace[0].1),
            "{trace:?}"
        );
        if fail_tail {
            let error = result.err().expect("尾检必须失败");
            assert!(error.contains("工作账损坏"), "{error}");
            assert_eq!(commits.load(Ordering::SeqCst), 0);
            assert_eq!(meta_i64(&owner.connection, "generation").unwrap(), 1);
            assert!(owner.construction_lease.stale.get());
        } else {
            let (receipt, ended) = result.unwrap();
            assert_eq!(receipt.generation, 2);
            assert_eq!(commits.load(Ordering::SeqCst), 1);
            owner.continue_with(ended).unwrap();
        }
        owner.connection.commit_hook(None::<fn() -> bool>).unwrap();
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

    /// #1488 v4 反例A：无enable的生产冷入口由唯一BootstrapGrant在候选第一分配前出生根（ColdBuildGuard前移），
    /// 既有载荷在根账活工作域内持票出生；不经MoveIn事后预留，根账无未拆移入余额。
    /// （0022：原名 owner_cold_candidate_born_from_grant_with_move_in，MoveIn 退役后判据随之改为“无事后移入”。）
    #[test]
    fn owner_cold_candidate_born_from_grant_before_first_allocation() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut c = context();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        mint(&active, &mut c);
        assert!(c.take_bootstrap_grant().is_none(), "出生许可只可消费一次");
        let complete = active.complete.borrow();
        let candidate = complete.as_ref().unwrap();
        assert_eq!(
            candidate.resources.moved_in_for_test(),
            0,
            "冷出生不得留事后移入余额"
        );
        assert!(c.matches_complete_budget(&candidate.resources));
        assert!(candidate.resources.live() <= c.complete_workspace_limit());
    }

    /// #1488 v4 反例D：PendingRawState自身许可随状态持有，不在签发闭包出口提前归还。
    #[test]
    fn owner_raw_pending_state_holds_its_fee_until_drop() {
        let f = fixture();
        let owner = AuditConnection::open(&f.db_path()).unwrap();
        let mut c = context();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        mint(&active, &mut c);
        let parent = active.complete.borrow().as_ref().unwrap().resources.clone();
        assert!(active.construction.borrow().as_ref().unwrap().has_fee());
        let held = parent.live();
        drop(active.construction.borrow_mut().take());
        assert!(
            held - parent.live() >= std::mem::size_of::<construction_state::PendingRawState>(),
            "状态块许可须持有到PendingRawState析构"
        );
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
        source_audit::reset_source_call_stats();
        assert!(
            next.audit_admission(
                &json!({"payload":{"op":"ready"}}),
                &s_allocation::Mode::CapturedNative,
                &mut c,
            )
            .is_err(),
            "外部COMMIT失效必须在完整冷核前拒绝"
        );
        assert_eq!(
            source_audit::source_call_stats(),
            source_audit::SourceCallStats::default()
        );
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

// #1488 仅测试私有构造点；NOT_RUN，本机禁止 cargo/rustc。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod raw_native_move_ownership_tests {
    use super::*;

    #[test]
    fn pending_and_bound_raw_moves_keep_original_permits_until_payload_drop() {
        let raw = vec![
            serde_json::json!({"identity_key":"a", "revision":1, "source_coord":"0"}),
            serde_json::json!({"identity_key":"b", "revision":1, "source_coord":"1"}),
        ];
        let lease = construction_state::Lease::default();
        let bound = BoundOriginals { _private: () };
        let seal = SnapshotSeal {
            life: 1,
            transaction_id: 1,
            write_sequence: 0,
            data_version: 1,
            schema_version: 1,
            domain: [0; 6],
        };
        let budget = native::Budget::new(2 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let (pending, token) = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    construction_state::PendingRawState::advance(
                        None,
                        (&raw).into(),
                        seal,
                        &bound,
                        &lease,
                        &super::super::audit_store::NativeAdmission(Some(scope)),
                    )
                })
                .unwrap();
            token.ensure_current().unwrap();
            assert_eq!(pending.facts.prefix_len(), 2);
            assert_eq!(token.winner_edits().len(), 2);
            let both = budget.live();
            assert!(both > baseline);
            // 禁区中的第二次 move：BoundRawAdvance -> IssuedRawAdvance -> bind。
            let issued = token.into_issued();
            assert_eq!(budget.live(), both);
            let token = issued.bind((&raw).into(), &lease).unwrap();
            assert_eq!(budget.live(), both);
            drop(pending);
            assert!(budget.live() < both);
            assert!(
                budget.live() > baseline,
                "移动后的真实 winner_edits buffer 仍持费"
            );
            drop(token);
            assert_eq!(
                budget.live(),
                baseline,
                "pending 与 token 的真实 owner 全部析构才归零"
            );
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
impl AuditConnection {
    pub(super) fn complete_budget_for_test(&self) -> native::Budget {
        self.complete
            .borrow()
            .as_ref()
            .expect("完整候选")
            .resources
            .clone()
    }
}
