//! #1394 C §5.1/5.8：`s-session/3` 新域的**控制行机械规则**（接纳 / Begin / 提交时刻 / Commit / recover）。
//!
//! 本模块是 v2 控制机械在 S3 域上的**同一份规则**，不是第二套：
//!
//! - clock 计划解析、校验与相位读取复用 `v2::ClockPlan`；持久 clock 绑定核验复用 `v2::check_clock`。
//! - `ingest` 业务形状复用 `v2::validate_ingest`；公共头严格走 `session_protocol::validate_envelope`。
//! - 接纳 / Begin / Commit / recover 的落库动作复用 `v2::record_*_context` 系列：真实持久
//!   attempt/ordinal、同一 token 配方、delivery refs 与保留窗口、相位前沿不倒退都只有那一份实现。
//! - 持久 message 行与回执读取复用 `v2::message_row` / `v2::lookup`；S3 只额外核 clock 绑定。
//!
//! 两条边界写清楚，免得被读成「已经接好」：
//!
//! 1. **只管控制行**：本模块不核根状态（可达根、批次、内容闭包、epoch 换代历史、profile 绑定等）。
//!    调用方必须在任何写入之前完成自己的 S3 完整 state 审计；本模块返回 `Ok` 只表示控制行已按
//!    共用机械规则写入**调用方的同一个事务**。写入被拒时调用方必须回滚该事务——本模块不持有
//!    事务，也不提供「半个控制行也算成功」的路径。
//! 2. **没有绕过域核验的写入口**：所有写方法都是 [`S3Control`] 的方法，而 [`S3Control`] 只能由
//!    [`S3Control::attach`] 在已登记的 `s-session/3` 域上构造。非本域一律 `Err`，不存在 v2
//!    `active()` 那种「旧库静默 no-op」的分支——S3 不把未核验的库当成功。
//!
//! ⚠️ 完成后仍不算 S3 生产提交：完整 S3 writer/发布接线由 root 另行完成，本模块只交付控制行底座。

use super::*;
use newchan_rust::session_protocol::{required, validate_envelope};

/// 已核验的 `s-session/3` 控制域句柄：只可能由 [`S3Control::attach`] 构造。
pub(super) struct S3Control<'a> {
    conn: &'a Connection,
    clock: v2::ClockPlan,
    session_id: String,
    session_generation: String,
    writer_epoch: String,
}

/// 单条 ingest 在持久控制行上的真实状态（无第四态：`s_input_messages.status` 只有两个合法值）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StoredState {
    /// 从未接纳（无 `s_input_messages` 行）。
    Fresh,
    /// 已接纳、未提交：可继续 Begin/Commit，或按原 attempt 配方重试。
    AcceptedPending,
    /// 已提交：同 key 同 hash 同 clock 的重投按原回执幂等返回，不再写任何控制行。
    Committed,
}

impl StoredState {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Fresh => "Fresh",
            Self::AcceptedPending => "AcceptedPending",
            Self::Committed => "Committed",
        }
    }
}

/// 已核验的 ingest 视图：严格校验后的 envelope + 域绑定 + 持久状态 + 原回执。
///
/// 视图不持有事务借用，可在同一事务内继续交给 [`S3Control`] 的写方法使用。
pub(super) struct IngestView {
    envelope: Value,
    context: v2::InputContext,
    state: StoredState,
    receipt: Value,
}

impl IngestView {
    #[cfg(test)]
    pub(super) fn state(&self) -> StoredState {
        self.state
    }
    pub(super) fn is_fresh(&self) -> bool {
        self.state == StoredState::Fresh
    }
    pub(super) fn is_committed(&self) -> bool {
        self.state == StoredState::Committed
    }
    /// 持久回执视图（与 v2 `lookup` 同一形状）：未接纳时 `kind=Unknown`，否则为真实状态。
    pub(super) fn receipt(&self) -> &Value {
        &self.receipt
    }
    pub(super) fn envelope(&self) -> &Value {
        &self.envelope
    }
}

impl<'a> S3Control<'a> {
    /// 在读事务/连接上核验本域：根 `meta.protocol_revision` 与 `s_protocol_meta.protocol_revision`
    /// 都必须是 `s-session/3`、五张控制表齐全、持久 clock 计划与本次计划 hash 相同，并快照域身份。
    ///
    /// 只读，不写任何行；任何一项不满足即 `Err`（不回落到 v2 语义、不跳过检查）。
    pub(super) fn attach(conn: &'a Connection, clock: &v2::ClockPlan) -> Result<Self, String> {
        let meta_marker = meta_get_opt(conn, "protocol_revision")?;
        let table_marker: Option<String> = conn
            .query_row(
                "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("StorageUnavailable：读 s_protocol_meta 失败：{e}"))?;
        let tables: i64=conn.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name IN ('s_protocol_meta','s_input_messages','s_clock_events','s_delivery_policy','s_delivery_refs')",[],|r|r.get(0)).map_err(|e| format!("StorageUnavailable：读控制表清单失败：{e}"))?;
        if meta_marker.as_deref() != Some(PROTOCOL_REVISION)
            || table_marker.as_deref() != Some(PROTOCOL_REVISION)
            || tables != 5
        {
            return Err(format!(
                "IdentityConflict：控制行只属于已登记的 {PROTOCOL_REVISION} 域（meta标记={meta_marker:?}, 控制表标记={table_marker:?}, 控制表={tables}/5），拒绝写入"
            ));
        }
        v2::check_clock(conn, clock)?;
        let (session_id, session_generation, writer_epoch) = v2::protocol_identity(conn)?;
        Ok(Self {
            conn,
            clock: clock.clone(),
            session_id,
            session_generation,
            writer_epoch,
        })
    }

    /// 本域当前持久身份：`session_id` / `session_generation` / 持久 `writer_epoch`（回复绑定用）。
    pub(super) fn identity(&self) -> (&str, &str, &str) {
        (
            &self.session_id,
            &self.session_generation,
            &self.writer_epoch,
        )
    }

    fn require_write_transaction(&self) -> Result<(), String> {
        if self.conn.is_autocommit() {
            return Err("InvalidDomain：S3 控制写入必须在事务内".into());
        }
        Ok(())
    }

    /// 严格校验 `s-session/3` ingest envelope、核域身份与 writer epoch 绑定，并读持久 message 状态。
    ///
    /// 只读：不写任何控制行。同 key 同 hash 同 clock 的已持久消息按幂等视图返回（`Committed`
    /// 由调用方直接回原回执）；hash 不同或 clock 不同一律 `IdentityConflict`。
    pub(super) fn inspect_ingest(&self, envelope: Value) -> Result<IngestView, String> {
        let envelope = self.validate_ingest_envelope(envelope)?;
        let epoch = required(&envelope["payload"], "writer_epoch")?;
        if epoch != self.writer_epoch {
            return Err(format!(
                "StaleWriter：消息 writer_epoch=`{epoch}`，非本域当前持久 `{}`（换代后旧 writer 拒绝，零写入）",
                self.writer_epoch
            ));
        }
        self.bind_ingest(envelope)
    }

    /// 内部推进只从本库取已经接纳的原件；不接收调用者可改写的旧 epoch envelope。
    /// 与新外部请求的当前写者准入分开：监督器换代后旧原件必须保持原字节。
    pub(super) fn load_pending(
        &self,
        namespace: &str,
        source_epoch: &str,
        message_id: &str,
    ) -> Result<IngestView, String> {
        let (raw,status): (String,String) = self.conn.query_row(
            "SELECT canonical_envelope,status FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",
            params![namespace,source_epoch,message_id],|r|Ok((r.get(0)?,r.get(1)?)),
        ).map_err(|e|format!("StorageUnavailable：读取已接纳原件失败：{e}"))?;
        if status != "AcceptedPending" {
            return Err("InvalidDomain：内部推进只接已接纳未提交原件".into());
        }
        let envelope = self.validate_ingest_envelope(strict_json(raw.as_bytes())?)?;
        if canonical_json(&envelope) != raw
            || required(&envelope, "source_namespace")? != namespace
            || required(&envelope, "source_epoch")? != source_epoch
            || required(&envelope, "message_id")? != message_id
        {
            return Err("IdentityConflict：已接纳原件与独立message行不同".into());
        }
        let view = self.bind_ingest(envelope)?;
        if view.state != StoredState::AcceptedPending {
            return Err("IdentityConflict：已接纳状态变更".into());
        }
        Ok(view)
    }

    fn validate_ingest_envelope(&self, envelope: Value) -> Result<Value, String> {
        let envelope = validate_envelope(envelope, PROTOCOL_REVISION)?;
        if required(&envelope["payload"], "op")? != "ingest" {
            return Err(format!(
                "SchemaUnsupported：{PROTOCOL_REVISION} 控制入口只受理 ingest"
            ));
        }
        v2::validate_ingest(&envelope)?;
        if required(&envelope, "session_id")? != self.session_id
            || required(&envelope, "session_generation")? != self.session_generation
        {
            return Err("IdentityConflict：旧session/化身请求".into());
        }
        Ok(envelope)
    }

    fn bind_ingest(&self, envelope: Value) -> Result<IngestView, String> {
        let payload = &envelope["payload"];
        let key = (
            required(&envelope, "source_namespace")?.to_string(),
            required(&envelope, "source_epoch")?.to_string(),
            required(&envelope, "message_id")?.to_string(),
        );
        let hash = required(&envelope, "payload_hash")?;
        let clock_event_id = required(payload, "clock_event_id")?;
        let stored = v2::message_row(self.conn, &key.0, &key.1, &key.2)?;
        let state = match &stored {
            None => StoredState::Fresh,
            Some(row) => {
                // 同一 message 身份只认同一 clock 事件；换 clock 即换内容证据，不得当重投。
                if row.clock_event_id != clock_event_id {
                    return Err(format!(
                        "IdentityConflict：原message `{}` 绑定 clock事件 `{}`，与本次 `{clock_event_id}` 不同",
                        key.2, row.clock_event_id
                    ));
                }
                match row.status.as_str() {
                    "Committed" => StoredState::Committed,
                    "AcceptedPending" => StoredState::AcceptedPending,
                    other => {
                        return Err(format!(
                            "StorageUnavailable：s_input_messages.status 非法取值 `{other}`"
                        ))
                    }
                }
            }
        };
        // 回执形状与 hash 判据复用 v2 的唯一实现（Fresh 时即 `kind=Unknown`）。
        let receipt = v2::lookup(self.conn, &key.0, &key.1, &key.2, hash)?;
        Ok(IngestView {
            context: v2::InputContext::new(envelope.clone(), self.clock.clone()),
            envelope,
            state,
            receipt,
        })
    }

    /// 接纳（仅 `Fresh`）：写 `s_input_messages` 行与 accept clock 事件，入调用方同一事务。
    ///
    /// `results` 是调用方实际接纳产出的结果（`status/seq/receipt_id`），本模块不代业务判定。
    pub(super) fn accept(&self, view: &IngestView, results: &[Value]) -> Result<(), String> {
        self.require_write_transaction()?;
        if view.state != StoredState::Fresh {
            return Err(format!(
                "InvalidDomain：消息已持久为 {}，不得重复接纳（控制行不可覆盖）",
                view.state.name()
            ));
        }
        v2::record_accept_context(self.conn, &view.context, results)
    }

    /// Begin（`Fresh` 刚在本事务内接纳，或持久 `AcceptedPending`）：真实 attempt/ordinal 与同一
    /// token 配方，写 begin clock 事件并推进 attempt 序；返回与 v2 逐字同配方的 token。
    pub(super) fn begin(
        &self,
        view: &IngestView,
        gen: i64,
        frontier: i64,
        base_cut: &str,
        configured_epoch: &str,
    ) -> Result<String, String> {
        self.require_write_transaction()?;
        verify_writer_epoch(self.conn, configured_epoch)?;
        if view.state == StoredState::Committed {
            return Err(
                "InvalidDomain：已提交消息不得再 Begin（先经 recover/新 generation 走正式入口）"
                    .into(),
            );
        }
        let token = v2::record_begin_context(
            self.conn,
            &view.context,
            gen,
            frontier,
            base_cut,
            configured_epoch,
        )?;
        persist_begin_state(self.conn, &token, gen, frontier)?;
        Ok(token)
    }

    /// 本次已持久 Begin 对应的 Commit 时刻；未 Begin 即 `Err`。
    ///
    /// Plan 读同一 attempt 的 `commit_ns`。System（#1769）在此取样一次：只由准备发布调用，
    /// 读数封进批次元数据，发布事务以 [`Self::sealed_commit_matches`]／[`Self::commit_sealed`]
    /// 沿用该值，不再取样。
    pub(super) fn commit_time(&self, view: &IngestView) -> Result<Option<String>, String> {
        v2::commit_reading_context(self.conn, &view.context)
    }

    /// 发布事务内核已封批的 Commit 读数（Plan 等于计划值；System 不早于持久相位前沿）。
    pub(super) fn sealed_commit_matches(
        &self,
        view: &IngestView,
        semantic_commit_ns: &str,
    ) -> Result<bool, String> {
        v2::sealed_commit_matches_context(self.conn, &view.context, semantic_commit_ns)
    }

    /// Commit（`AcceptedPending`，或本事务内刚接纳+Begin）：写 commit clock 事件、推进
    /// status/first_published_generation 与 delivery refs/policy（含保留窗口），入调用方同一事务。
    #[cfg(test)]
    pub(super) fn commit(&self, view: &IngestView, gen: i64, frontier: i64) -> Result<(), String> {
        self.require_write_transaction()?;
        if view.state == StoredState::Committed {
            return Err("InvalidDomain：消息已提交，控制行不重复写".into());
        }
        v2::record_commit_context(self.conn, &view.context, gen, frontier)
    }

    /// 同 [`Self::commit`]，commit clock 读数取自本次已封批元数据（System 不在提交时取样）。
    pub(super) fn commit_sealed(
        &self,
        view: &IngestView,
        gen: i64,
        frontier: i64,
        semantic_commit_ns: &str,
    ) -> Result<(), String> {
        self.require_write_transaction()?;
        if view.state == StoredState::Committed {
            return Err("InvalidDomain：消息已提交，控制行不重复写".into());
        }
        v2::record_commit_sealed(self.conn, &view.context, gen, frontier, semantic_commit_ns)
    }

    /// recover：写 recover clock 事件并推进相位前沿（换代自身状态由调用方在同一事务内完成）。
    pub(super) fn recover(&self, clock_event_id: &str, generation: &str) -> Result<String, String> {
        self.require_write_transaction()?;
        v2::record_recover_context(self.conn, &self.clock, clock_event_id, generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    const SESSION: &str = "s3-control-test";
    const CLOCK_ID: &str = "s3-control-test-clock";
    const ACCEPT_RESULT_SEQ: &str = "0";

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "s1394-s3-control-{}-{stamp}-{number}",
                std::process::id()
            ));
            std::fs::create_dir(&dir).unwrap();
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fixture(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session")
            .join(path)
    }

    /// 显式 clock fixture：op-1 的正常 attempt（accept 10 → begin 11 → commit 120），
    /// op-2 的 accept(13) 落在 op-1 的已提交前沿(120)之后（用于回退时钟拒绝），
    /// recover-1 的 130 在 op-1 前沿之后。
    fn clock_value(clock_id: &str) -> Value {
        json!({
            "schema_revision": "s-clock-plan/1",
            "clock_plan_id": clock_id,
            "origin_utc": "2026-09-18T00:00:00Z",
            "unit": "ns",
            "events": {
                "op-1": {"accept_ns": "10",
                    "attempts": [{"begin_ns": "11", "commit_ns": "120"}]},
                "op-2": {"accept_ns": "13",
                    "attempts": [{"begin_ns": "14", "commit_ns": "15"}]},
                "recover-1": {"recover_ns": "130"}
            }
        })
    }

    fn write_clock(dir: &Temp, name: &str, clock_id: &str) -> PathBuf {
        let path = dir.path(name);
        std::fs::write(&path, canonical_json(&clock_value(clock_id))).unwrap();
        path
    }

    /// 真实 S3 域：`init_v3` 建共同 authority 表 + v2 控制表 + `s-content/1` 节点表。
    fn init_domain(dir: &Temp, clock_id: &str) -> (PathBuf, PathBuf, v2::ClockPlan) {
        let db = dir.path("s3.sqlite");
        let clock_path = write_clock(dir, "clock.json", clock_id);
        super::super::init_v3(
            &db,
            SESSION,
            &fixture("catalog/signed-catalog.json"),
            &fixture("profiles/testonly_tick_1_1_ohlc.json"),
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        let clock = v2::ClockPlan::load(&clock_path).unwrap();
        (db, clock_path, clock)
    }

    fn envelope(message_id: &str, clock_event_id: &str, price: &str, epoch: &str) -> Value {
        let raw = json!({
            "schema_revision": "1",
            "session_id": SESSION,
            "source_namespace": "test.feed",
            "source_epoch": "1",
            "instrument": "TEST.TICK",
            "profile": "testonly_tick_1_1_ohlc",
            "events": [{
                "event_id": format!("event-{message_id}"),
                "revision": "1",
                "seq": "0",
                "received_at": "2026-09-18T00:00:00Z",
                "raw_text": price,
                "price": price,
                "timestamp": "0",
                "volume": "1"
            }]
        });
        let payload = json!({
            "op": "ingest",
            "target_session_id": SESSION,
            "target_session_generation": "1",
            "writer_epoch": epoch,
            "clock_event_id": clock_event_id,
            "raw_input": raw
        });
        json!({
            "schema_revision": "s-session/3",
            "session_id": SESSION,
            "session_generation": "1",
            "source_namespace": "test.feed",
            "source_epoch": "1",
            "message_id": message_id,
            "producer_id": "s3-control-test-driver",
            "producer_epoch": "1",
            "payload_hash": sha256_hex(canonical_json(&payload).as_bytes()),
            "causal_refs": [],
            "payload": payload
        })
    }

    fn accept_result(receipt: &str) -> Value {
        json!({"status": "accepted", "seq": ACCEPT_RESULT_SEQ, "receipt_id": receipt})
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    /// 单值读取统一按文本比较（SQLite 里 `||`/CAST 才是 TEXT，整数列直接读 `String` 会报类型错）。
    fn scalar(conn: &Connection, sql: &str) -> String {
        conn.query_row(&format!("SELECT CAST(({sql}) AS TEXT)"), [], |r| r.get(0))
            .unwrap()
    }

    /// 必须失败的调用：`S3Control`/`IngestView` 不实现 `Debug`，不用 `unwrap_err`。
    fn expect_err<T>(result: Result<T, String>, what: &str) -> String {
        match result {
            Ok(_) => panic!("{what}（预期拒绝）"),
            Err(error) => error,
        }
    }

    /// 本代 delta 由 root 在本事务内先落盘；`s_delivery_refs.generation` 的外键因此可满足
    ///（rusqlite bundled 默认 `SQLITE_DEFAULT_FOREIGN_KEYS=1`，与生产同一条连接形态）。
    fn insert_delta(conn: &Connection, generation: i64) {
        conn.execute(
            "INSERT INTO structure_deltas(generation,session_id,catalog_revision,base_cut,next_cut,
                 seq_range_json,index_frontier,input_frontier,catalog_run_status,catalog_evidence_json,delta_json,policy_records)
             VALUES(?1,?2,?3,'cut-0','cut-1','[0,0]','batch-test',0,'not_run','{}','{}','control-fixture/1')",
            params![
                generation,
                SESSION,
                scalar(conn, "SELECT value FROM meta WHERE key='catalog_revision'")
            ],
        )
        .unwrap();
    }

    #[test]
    fn s3_control_requires_transaction_for_every_write() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let conn = open_db(&db).unwrap();
        let control = S3Control::attach(&conn, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        for result in [
            control.accept(&view, &[accept_result("receipt-1")]),
            control.begin(&view, 1, 0, "cut-0", "1").map(|_| ()),
            control.commit(&view, 1, 0),
            control.recover("recover-1", "1").map(|_| ()),
        ] {
            let error = result.expect_err("裸连接只允许只读检查，控制写入须有事务");
            assert!(error.contains("必须在事务内"), "{error}");
        }
        assert_eq!(count(&conn, "s_input_messages"), 0);
        assert_eq!(count(&conn, "s_clock_events"), 0);
        assert_eq!(count(&conn, "s_delivery_refs"), 0);
    }

    #[test]
    fn s3_control_rejects_configured_epoch_before_begin_writes() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        let error = control
            .begin(&view, 1, 0, "cut-0", "2")
            .expect_err("消息 epoch 正确也不能代替实际 writer 配置核验");
        assert!(error.contains("StaleWriter"), "{error}");
        assert_eq!(
            scalar(&tx, "SELECT attempt_count FROM s_input_messages"),
            "0"
        );
        assert_eq!(
            scalar(&tx, "SELECT next_attempt_ordinal FROM s_protocol_meta"),
            "1"
        );
        assert_eq!(count(&tx, "s_clock_events"), 1);
        tx.rollback().unwrap();
    }

    #[test]
    fn s3_control_accept_begin_commit_writes_real_rows_and_idempotent_receipt() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();
        // s_content 域表与 s-session/3 登记同时存在：控制行只写在这个新域里。
        assert_eq!(
            scalar(
                &conn,
                "SELECT content_format FROM s_content_meta WHERE singleton=1"
            ),
            "s-content/1"
        );
        assert_eq!(count(&conn, "s_content_nodes"), 0);

        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        assert_eq!(control.identity(), (SESSION, "1", "1"));
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        assert!(view.is_fresh());
        assert_eq!(view.state().name(), "Fresh");
        assert_eq!(view.receipt()["kind"], json!("Unknown"));

        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        // 接纳行与 accept clock 事件在同一事务内已可见，但尚未提交。
        assert_eq!(
            scalar(
                &tx,
                "SELECT status FROM s_input_messages WHERE message_id='msg-1'"
            ),
            "AcceptedPending"
        );
        assert_eq!(
            scalar(&tx, "SELECT attempt_count FROM s_input_messages"),
            "0"
        );
        assert_eq!(scalar(&tx, "SELECT semantic_ns FROM s_clock_events"), "10");

        let token = control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        assert!(token.starts_with("begin-"), "Begin token 形态异常：{token}");
        assert_eq!(
            control.commit_time(&view).unwrap().as_deref(),
            Some("120"),
            "本次 Begin 的提交时刻必须是 clock 计划里的 commit_ns"
        );
        // 真实流程里 root 在同一事务内先落本代 delta，控制行才引用它（回执 publication 因此可读）。
        insert_delta(&tx, 1);
        control.commit(&view, 1, 0).unwrap();
        let committed = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        assert!(committed.is_committed());
        tx.commit().unwrap();

        // 真实控制行：接纳状态、attempt/ordinal、三相位 clock、前沿、投递 refs/policy。
        assert_eq!(
            scalar(
                &conn,
                "SELECT status FROM s_input_messages WHERE message_id='msg-1'"
            ),
            "Committed"
        );
        assert_eq!(
            scalar(
                &conn,
                "SELECT receipt_id||'|'||accepted_seq||'|'||attempt_count||'|'||first_published_generation||'|'||clock_event_id FROM s_input_messages"
            ),
            "receipt-1|0|1|1|op-1"
        );
        assert_eq!(
            scalar(
                &conn,
                "SELECT protocol_revision||'|'||next_attempt_ordinal||'|'||logical_phase_frontier FROM s_protocol_meta"
            ),
            "s-session/3|2|120"
        );
        let events: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT phase||'|'||attempt_index||'|'||semantic_ns||'|'||IFNULL(attempt_ordinal,-1)||'|'||generation||'|'||message_key FROM s_clock_events ORDER BY phase")
                .unwrap();
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
            rows.collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert_eq!(
            events,
            vec![
                "accept|-1|10|-1|0|[\"test.feed\",\"1\",\"msg-1\"]",
                "begin|0|11|1|1|[\"test.feed\",\"1\",\"msg-1\"]",
                "commit|0|120|1|1|[\"test.feed\",\"1\",\"msg-1\"]",
            ]
        );
        assert_eq!(count(&conn, "s_delivery_refs"), 1);
        assert_eq!(
            scalar(
                &conn,
                "SELECT first_available_generation||'|'||head_generation||'|'||retain_generations FROM s_delivery_policy"
            ),
            "1|1|4"
        );

        // 同 key 同 hash 同 clock 重投：原回执照返回，零新增控制行。
        let tx = conn.transaction().unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let replayed = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        assert!(replayed.is_committed());
        assert_eq!(replayed.receipt()["kind"], json!("Committed"));
        assert_eq!(replayed.receipt()["receipt_id"], json!("receipt-1"));
        assert_eq!(replayed.receipt()["accepted_seq"], json!("0"));
        assert_eq!(replayed.receipt()["publication"]["generation"], json!("1"));
        assert!(control
            .accept(&replayed, &[accept_result("receipt-1")])
            .is_err());
        assert!(control.begin(&replayed, 2, 1, "cut-0", "1").is_err());
        assert!(control.commit(&replayed, 2, 1).is_err());
        tx.rollback().unwrap();
        assert_eq!(count(&conn, "s_clock_events"), 3);
        assert_eq!(
            scalar(&conn, "SELECT attempt_count FROM s_input_messages"),
            "1"
        );
    }

    #[test]
    fn s3_control_rejects_same_key_other_hash_or_other_clock() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();

        // 同 key 异内容：payload_hash 与持久绑定不同，拒绝。
        let other_content = expect_err(
            control.inspect_ingest(envelope("msg-1", "op-1", "20000", "1")),
            "同 key 异内容必须拒绝",
        );
        assert!(
            other_content.contains("IdentityConflict"),
            "{other_content}"
        );
        // 同 key 同 hash 但持久 clock 事件被换成另一个（内容证据不一致）：拒绝。
        tx.execute(
            "UPDATE s_input_messages SET clock_event_id='op-2' WHERE message_id='msg-1'",
            [],
        )
        .unwrap();
        let conflict = expect_err(
            control.inspect_ingest(envelope("msg-1", "op-1", "10000", "1")),
            "同 key 异 clock 必须拒绝",
        );
        assert!(conflict.contains("IdentityConflict"), "{conflict}");
        assert!(conflict.contains("clock事件"), "{conflict}");
        // 两次拒绝都没有新增控制行：只有本事务内那次接纳的 accept 事件。
        assert_eq!(count(&tx, "s_clock_events"), 1);
        tx.rollback().unwrap();
        assert_eq!(count(&conn, "s_clock_events"), 0);
    }

    #[test]
    fn s3_control_rejects_stale_epoch_changed_clock_plan_and_backward_clock() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();

        // 旧 epoch：消息 writer_epoch 与域当前持久值不同，零写入拒绝。
        {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let control = S3Control::attach(&tx, &clock).unwrap();
            let stale = expect_err(
                control.inspect_ingest(envelope("msg-1", "op-1", "10000", "2")),
                "旧 epoch 必须拒绝",
            );
            assert!(stale.contains("StaleWriter"), "{stale}");
            tx.rollback().unwrap();
        }
        assert_eq!(count(&conn, "s_input_messages"), 0);

        // clock 计划变更（不同 clock_plan_id ⇒ 不同 hash）：域绑定不上，attach 即拒绝。
        let other_path = write_clock(&temp, "other-clock.json", "s3-control-test-clock-v2");
        let other = v2::ClockPlan::load(&other_path).unwrap();
        let tx = conn.transaction().unwrap();
        let err = expect_err(
            S3Control::attach(&tx, &other),
            "变更后的 clock 计划必须拒绝",
        );
        assert!(err.contains("clock计划与持久绑定不同"), "{err}");
        tx.rollback().unwrap();

        // 正常提交 op-1 后，op-2 的 accept_ns(13) 早于已提交前沿(120)：回退时钟拒绝。
        {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let control = S3Control::attach(&tx, &clock).unwrap();
            let view = control
                .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
                .unwrap();
            control
                .accept(&view, &[accept_result("receipt-1")])
                .unwrap();
            control.begin(&view, 1, 0, "cut-0", "1").unwrap();
            insert_delta(&tx, 1);
            control.commit(&view, 1, 0).unwrap();
            tx.commit().unwrap();
        }
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let newer = control
            .inspect_ingest(envelope("msg-2", "op-2", "10001", "1"))
            .unwrap();
        assert!(newer.is_fresh());
        let backward = expect_err(
            control.accept(&newer, &[accept_result("receipt-2")]),
            "回退时钟必须拒绝",
        );
        assert!(backward.contains("实际语义clock倒退"), "{backward}");
        tx.rollback().unwrap();
        assert_eq!(
            count(&conn, "s_input_messages"),
            1,
            "回退时钟拒绝后不得留下 msg-2 的接纳行"
        );
        assert_eq!(
            scalar(
                &conn,
                "SELECT next_attempt_ordinal||'|'||logical_phase_frontier FROM s_protocol_meta"
            ),
            "2|120"
        );
    }

    #[test]
    fn s3_control_refuses_non_s3_domains_without_writes() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        tx.execute(
            "UPDATE meta SET value='s-session/2' WHERE key='protocol_revision'",
            [],
        )
        .unwrap();
        tx.execute(
            "UPDATE s_protocol_meta SET protocol_revision='s-session/2'",
            [],
        )
        .unwrap();
        let err = expect_err(S3Control::attach(&tx, &clock), "非 s-session/3 域必须拒绝");
        assert!(err.contains("只属于已登记的 s-session/3 域"), "{err}");
        tx.rollback().unwrap();
        assert_eq!(count(&conn, "s_input_messages"), 0);
        assert_eq!(count(&conn, "s_clock_events"), 0);

        // 完全没有控制表的库（旧私有/空库）：读取即失败，不存在「静默 no-op」分支。
        let empty = Connection::open_in_memory().unwrap();
        assert!(S3Control::attach(&empty, &clock).is_err());
    }

    #[test]
    fn s3_control_rollback_leaves_no_partial_control_and_recover_is_phase_bound() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();

        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        tx.rollback().unwrap();
        for table in ["s_input_messages", "s_clock_events", "s_delivery_refs"] {
            assert_eq!(count(&conn, table), 0, "{table} 回滚后必须为空");
        }
        assert_eq!(
            scalar(
                &conn,
                "SELECT next_attempt_ordinal||'|'||logical_phase_frontier FROM s_protocol_meta"
            ),
            "1|-1"
        );
        assert_eq!(
            scalar(
                &conn,
                "SELECT head_generation FROM s_delivery_policy WHERE singleton=1"
            ),
            "0"
        );

        // recover 走同一相位前沿判据：合法事件写行，未知事件与回退事件都拒绝。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        assert_eq!(control.recover("recover-1", "0").unwrap(), "130");
        let unknown = expect_err(
            control.recover("recover-unknown", "0"),
            "未声明的 recover 事件必须拒绝",
        );
        assert!(unknown.contains("MissingDependency"), "{unknown}");
        tx.rollback().unwrap();
        assert_eq!(count(&conn, "s_clock_events"), 0);

        // 回滚没有污染库：同一消息能重新走完整控制链并提交。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        assert!(view.is_fresh());
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        insert_delta(&tx, 1);
        control.commit(&view, 1, 0).unwrap();
        tx.commit().unwrap();
        assert_eq!(
            scalar(&conn, "SELECT status FROM s_input_messages"),
            "Committed"
        );
        assert_eq!(count(&conn, "s_delivery_refs"), 1);
    }

    /// #1769：真实 System 时钟描述的 S3 域（固定三键，hash 不随运行变化）。
    fn init_system_domain(dir: &Temp) -> (PathBuf, v2::ClockPlan) {
        let db = dir.path("s3-system.sqlite");
        let clock_path = dir.path("system-clock.json");
        std::fs::write(
            &clock_path,
            canonical_json(&json!({"schema_revision":"s-clock-system/1","clock_id":"s3-control-system","unit":"ns"})),
        )
        .unwrap();
        super::super::init_v3(
            &db,
            SESSION,
            &fixture("catalog/signed-catalog.json"),
            &fixture("profiles/testonly_tick_1_1_ohlc.json"),
            "1",
            &clock_path,
            4,
        )
        .unwrap();
        let clock = v2::ClockPlan::load(&clock_path).unwrap();
        assert!(clock.is_system());
        (db, clock)
    }

    fn clock_rows(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT clock_event_id||'|'||phase||'|'||attempt_index||'|'||semantic_ns FROM s_clock_events ORDER BY CAST(semantic_ns AS INTEGER)")
            .unwrap();
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
        rows.collect::<Result<Vec<_>, _>>().unwrap()
    }

    #[test]
    fn s_system_clock_samples_once_per_phase() {
        use v2::system_clock_test as fake;
        fake::install(Some(&[100, 200, 300, 400, 500, 600]));
        let temp = Temp::new();
        let (db, clock) = init_system_domain(&temp);
        let mut conn = open_db(&db).unwrap();
        assert_eq!(fake::samples(), 0, "init 不取样");

        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        assert_eq!(fake::samples(), 1);
        control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        assert_eq!(fake::samples(), 2);
        let sealed = control.commit_time(&view).unwrap().unwrap();
        assert_eq!(sealed, "300");
        assert_eq!(fake::samples(), 3);
        assert!(control.sealed_commit_matches(&view, &sealed).unwrap());
        insert_delta(&tx, 1);
        control.commit_sealed(&view, 1, 0, &sealed).unwrap();
        assert_eq!(fake::samples(), 3, "封批读数原样落账，提交时不再取样");
        tx.commit().unwrap();
        assert_eq!(
            clock_rows(&conn),
            vec![
                "op-1|accept|-1|100",
                "op-1|begin|0|200",
                "op-1|commit|0|300"
            ]
        );
        assert_eq!(
            scalar(&conn, "SELECT logical_phase_frontier FROM s_protocol_meta"),
            "300"
        );
        assert_eq!(
            scalar(&conn, "SELECT clock_plan_json FROM s_protocol_meta"),
            canonical_json(&clock.value),
            "描述绑定只含固定三键，不随读数变化"
        );

        // 已提交消息重投与回执查询：读持久回执，零取样、零新行。
        let tx = conn.transaction().unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let replayed = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        assert!(replayed.is_committed());
        assert_eq!(replayed.receipt()["kind"], json!("Committed"));
        assert_eq!(
            v2::lookup(
                &tx,
                "test.feed",
                "1",
                "msg-1",
                replayed.envelope()["payload_hash"].as_str().unwrap()
            )
            .unwrap()["kind"],
            json!("Committed")
        );
        tx.rollback().unwrap();
        assert_eq!(fake::samples(), 3);
        assert_eq!(count(&conn, "s_clock_events"), 3);

        // 第二条消息：未带封批读数的提交在 System 下显式拒绝，不取样。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let second = control
            .inspect_ingest(envelope("msg-2", "op-2", "10001", "1"))
            .unwrap();
        control
            .accept(
                &second,
                &[json!({"status": "accepted", "seq": "1", "receipt_id": "receipt-2"})],
            )
            .unwrap();
        control.begin(&second, 2, 1, "cut-1", "1").unwrap();
        assert_eq!(fake::samples(), 5);
        let unsealed = expect_err(control.commit(&second, 2, 1), "System 提交必须带封批读数");
        assert!(unsealed.contains("封批读数"), "{unsealed}");
        assert_eq!(fake::samples(), 5);
        tx.rollback().unwrap();

        // recover 在其相位取样一次。
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        assert_eq!(control.recover("recover-1", "1").unwrap(), "600");
        assert_eq!(fake::samples(), 6);
        tx.commit().unwrap();
        assert_eq!(
            clock_rows(&conn).last().map(String::as_str),
            Some("recover-1|recover|-1|600")
        );
    }

    #[test]
    fn s_system_clock_regression_rejected() {
        use v2::system_clock_test as fake;
        let temp = Temp::new();
        let (db, clock) = init_system_domain(&temp);
        let mut conn = open_db(&db).unwrap();

        // Begin 读数早于已持久 accept：拒绝，Begin 不落行、attempt 不推进。
        fake::install(Some(&[100, 90]));
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        let backward = expect_err(
            control.begin(&view, 1, 0, "cut-0", "1"),
            "Begin 倒退必须拒绝",
        );
        assert!(backward.contains("实际语义clock倒退"), "{backward}");
        assert_eq!(count(&tx, "s_clock_events"), 1);
        assert_eq!(
            scalar(&tx, "SELECT attempt_count FROM s_input_messages"),
            "0"
        );
        assert_eq!(
            scalar(&tx, "SELECT next_attempt_ordinal FROM s_protocol_meta"),
            "1"
        );
        tx.rollback().unwrap();

        // Commit 读数早于 Begin：准备取样即拒绝；伪造更早的封批值落账也拒绝，不换钟、不钳位。
        fake::install(Some(&[100, 200, 150]));
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        let backward = expect_err(control.commit_time(&view), "Commit 取样倒退必须拒绝");
        assert!(backward.contains("实际语义clock倒退"), "{backward}");
        let sealed = expect_err(
            control.sealed_commit_matches(&view, "150"),
            "早于 Begin 的封批值必须拒绝",
        );
        assert!(sealed.contains("实际语义clock倒退"), "{sealed}");
        insert_delta(&tx, 1);
        let commit = expect_err(
            control.commit_sealed(&view, 1, 0, "150"),
            "早于 Begin 的封批值不得落账",
        );
        assert!(commit.contains("实际语义clock倒退"), "{commit}");
        assert_eq!(count(&tx, "s_clock_events"), 2);
        assert_eq!(fake::samples(), 3);
        tx.rollback().unwrap();
        assert_eq!(count(&conn, "s_clock_events"), 0);
    }

    /// 旧 `s-clock-plan/1` 夹具在封批路径上仍以计划值为准：封批值不等于计划即拒绝。
    #[test]
    fn s_plan_clock_sealed_commit_must_equal_plan() {
        let temp = Temp::new();
        let (db, _, clock) = init_domain(&temp, CLOCK_ID);
        let mut conn = open_db(&db).unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let control = S3Control::attach(&tx, &clock).unwrap();
        let view = control
            .inspect_ingest(envelope("msg-1", "op-1", "10000", "1"))
            .unwrap();
        control
            .accept(&view, &[accept_result("receipt-1")])
            .unwrap();
        control.begin(&view, 1, 0, "cut-0", "1").unwrap();
        assert_eq!(control.commit_time(&view).unwrap().as_deref(), Some("120"));
        assert!(control.sealed_commit_matches(&view, "120").unwrap());
        assert!(!control.sealed_commit_matches(&view, "121").unwrap());
        insert_delta(&tx, 1);
        let mismatch = expect_err(
            control.commit_sealed(&view, 1, 0, "121"),
            "封批值与计划不同必须拒绝",
        );
        assert!(mismatch.contains("封批clock读数与计划不同"), "{mismatch}");
        control.commit_sealed(&view, 1, 0, "120").unwrap();
        tx.commit().unwrap();
        assert_eq!(
            scalar(
                &conn,
                "SELECT semantic_ns FROM s_clock_events WHERE phase='commit'"
            ),
            "120"
        );
    }
}
