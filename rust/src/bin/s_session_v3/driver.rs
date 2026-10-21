//! #1394：S3 消息到真实接纳、Begin 和发布的同步驱动。
//! 传输层须先执行资源/帧约束；CLI 由 writer_cli 接线。
//! 同一实例持有原进程锁；每次读取/写入都在当前事务调用同一完整状态审计。

use super::*;
use newchan_rust::session_protocol::{required, validate_envelope};
use std::fs::File;

/// CLI 预检与运行中 handler 共用的唯一 typed 请求判据。
pub(super) fn validate_request(envelope: Value) -> Result<Value, String> {
    let envelope = validate_envelope(envelope, PROTOCOL_REVISION)?;
    let payload = &envelope["payload"];
    let keys: &[&str] = match required(payload, "op")? {
        "ready" => &["op"],
        "receipt" => &[
            "op",
            "original_source_namespace",
            "original_source_epoch",
            "original_message_id",
            "original_payload_hash",
        ],
        "ingest" => {
            v2::validate_ingest(&envelope)?;
            return Ok(envelope);
        }
        _ => return Err("SchemaUnsupported：未知S3 op；recover只属于监督器".into()),
    };
    let object = payload
        .as_object()
        .ok_or("SchemaUnsupported：payload必须对象")?;
    if object.len() != keys.len() || object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err("SchemaUnsupported：S3 typed payload 字段不完整或多余".into());
    }
    for key in keys {
        required(payload, key)?;
    }
    Ok(envelope)
}

/// 对当前持久原件作只读准入；预检与持锁后复核共用此判据。
/// 这里故意不推进 pending，冲突请求不得借 Driver::open 的恢复产生写入。
pub(super) fn check_request_before_resume(
    conn: &Connection,
    control: &control::S3Control<'_>,
    request: &Value,
    profile_id: &str,
) -> Result<(), String> {
    let request = validate_request(request.clone())?;
    let (session, incarnation, _) = control.identity();
    if required(&request, "session_id")? != session
        || required(&request, "session_generation")? != incarnation
    {
        return Err("IdentityConflict：请求不属于本会话/化身".into());
    }
    let payload = &request["payload"];
    match required(payload, "op")? {
        "ingest" => {
            let input = tb02a::parse_input(&payload["raw_input"])?;
            raw_input::validate_profile_binding(&input, profile_id)?;
            control.inspect_ingest(request)?;
        }
        "receipt" => {
            v2::lookup(
                conn,
                required(payload, "original_source_namespace")?,
                required(payload, "original_source_epoch")?,
                required(payload, "original_message_id")?,
                required(payload, "original_payload_hash")?,
            )?;
        }
        "ready" => {}
        _ => unreachable!("validate_request already rejected unknown S3 op"),
    }
    Ok(())
}

pub(super) struct Driver {
    conn: super::audit_owner::AuditConnection,
    /// 启动前冻结的互斥私有模式；Owned 与 Runtime 同账，不保留 capture/审计结果。
    mode: s_allocation::Mode,
    runtime: prepared::Runtime,
    clock: v2::ClockPlan,
    epoch: String,
    profile_id: String,
    volume_unit: Option<String>,
    /// 启动时冻结的唯一审计源保留预算；完整候选须由owner在每次handle绑定当前快照。
    audit_cache_source_bytes: Option<usize>,
    /// #1394：Native私有诊断开关在首次事务前冻结。
    #[cfg(feature = "s_capture_native_testonly")]
    sqlite_pager_probe: bool,
    _lock: File,
    #[cfg(test)]
    stop_after: Option<&'static str>,
}

impl Driver {
    pub(super) fn open_for_serve(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        audit_cache_source_bytes: usize,
    ) -> Result<Self, String> {
        Self::open_for_serve_with_allocation(
            db,
            profile,
            epoch,
            clock,
            audit_cache_source_bytes,
            None,
        )
    }

    pub(super) fn open_for_serve_with_allocation(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        audit_cache_source_bytes: usize,
        allocation: Option<q_allocation::Profile>,
    ) -> Result<Self, String> {
        Self::open_for_serve_with_mode(
            db,
            profile,
            epoch,
            clock,
            audit_cache_source_bytes,
            allocation.into(),
        )
    }

    pub(super) fn open_for_serve_with_mode(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        audit_cache_source_bytes: usize,
        mode: s_allocation::Mode,
    ) -> Result<Self, String> {
        state::check_audit_source_retention(audit_cache_source_bytes)?;
        let mut driver = Self::open_with_request_and_mode(db, profile, epoch, clock, None, mode)?;
        driver.audit_cache_source_bytes = Some(audit_cache_source_bytes);
        Ok(driver)
    }
    /// #1394：仅服务Native bootstrap栈内复用纯历史，返回前释放；不跨至Ready请求。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn open_for_serve_with_identity(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        audit_cache_source_bytes: usize,
        mode: s_allocation::Mode,
    ) -> Result<(Self, (String, String, String)), String> {
        state::check_audit_source_retention(audit_cache_source_bytes)?;
        if !matches!(mode, s_allocation::Mode::CapturedNative) {
            return Err("TestOnly Native bootstrap只接受显式CapturedNative模式".into());
        }
        let mut context = request_audit::RequestAuditContext::default();
        let mut driver = Self::open_with_request_and_mode_context(
            db,
            profile,
            epoch,
            clock,
            None,
            mode,
            Some(&mut context),
        )?;
        driver.audit_cache_source_bytes = Some(audit_cache_source_bytes);
        let identity = driver.identity_in_bootstrap(&mut context)?;
        drop(context);
        Ok((driver, identity))
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn identity_in_bootstrap(
        &self,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<(String, String, String), String> {
        if !matches!(self.mode, s_allocation::Mode::CapturedNative) {
            return Err("TestOnly Native bootstrap只接受显式CapturedNative模式".into());
        }
        let result = self.conn.identity(&self.epoch, &self.clock, Some(context));
        if result.is_err() {
            drop(context.take());
        }
        result
    }

    pub(super) fn identity(&self) -> Result<(String, String, String), String> {
        self.conn.identity(&self.epoch, &self.clock, None)
    }

    pub(super) fn state_summary(&self, command: &str) -> Result<Value, String> {
        self.conn.summary(command, &self.epoch, &self.clock)
    }

    pub(super) fn open(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
    ) -> Result<Self, String> {
        Self::open_with_request(db, profile, epoch, clock, None)
    }

    /// 输入请求在同一写者进程锁内、任何 pending 自动恢复之前再核一次。
    pub(super) fn open_with_request(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        request: Option<&Value>,
    ) -> Result<Self, String> {
        Self::open_with_request_and_allocation(db, profile, epoch, clock, request, None)
    }

    /// 在恢复 pending 之前冻结私有 TestOnly 档；启动身份审计仍走原入口。
    pub(super) fn open_with_request_and_allocation(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        request: Option<&Value>,
        allocation: Option<q_allocation::Profile>,
    ) -> Result<Self, String> {
        Self::open_with_request_and_mode(db, profile, epoch, clock, request, allocation.into())
    }

    /// startup仅显式CapturedNative做同核全捕获；其它模式保持普通。在pending恢复前审计并冻结模式。
    /// 多错坏库的首报顺序可能随先捕获改变；拒绝义务不变，不保留startup结果。
    pub(super) fn open_with_request_and_mode(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        request: Option<&Value>,
        mode: s_allocation::Mode,
    ) -> Result<Self, String> {
        Self::open_with_request_and_mode_context(db, profile, epoch, clock, request, mode, None)
    }
    // #1394：None保持原open；Some仅专有Native服务bootstrap调用，不保存context。
    fn open_with_request_and_mode_context(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
        request: Option<&Value>,
        mode: s_allocation::Mode,
        mut startup_context: Option<&mut request_audit::RequestAuditContext>,
    ) -> Result<Self, String> {
        mode.validate()?;
        parse_writer_epoch(epoch, "configured writer_epoch")?;
        if !db.is_file() {
            return Err("StorageUnavailable：S3驱动只打开已初始化库".into());
        }
        let lock = session_transport::process_lock(db, "S")?;
        let conn = super::audit_owner::AuditConnection::open(db)?;
        #[cfg(feature = "s_capture_native_testonly")]
        let sqlite_pager_probe = matches!(mode, s_allocation::Mode::CapturedNative)
            && super::audit_owner::pager_enabled();
        #[cfg(feature = "s_capture_native_testonly")]
        if sqlite_pager_probe {
            conn.configure_pager_probe()?;
        }
        let (profile_id, volume_unit) = conn.startup(
            profile,
            epoch,
            &clock,
            request,
            &mode,
            startup_context.as_deref_mut(),
        )?;
        let mut driver = Self {
            conn,
            runtime: prepared::Runtime::with_mode(mode.clone()),
            mode,
            clock,
            epoch: epoch.into(),
            profile_id,
            volume_unit,
            audit_cache_source_bytes: None,
            #[cfg(feature = "s_capture_native_testonly")]
            sqlite_pager_probe,
            _lock: lock,
            #[cfg(test)]
            stop_after: None,
        };
        driver.resume_pending(startup_context)?;
        Ok(driver)
    }

    pub(super) fn handle(&mut self, envelope: Value) -> Result<Value, String> {
        #[cfg(feature = "s_capture_native_testonly")]
        let pager_before = self.sqlite_pager_probe.then(|| {
            (
                self.conn.pager_snapshot(),
                envelope.get("message_id").cloned(),
            )
        });
        let result = self.handle_with_request_audit(envelope);
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some((before, message_id)) = pager_before {
            let after = self.conn.pager_snapshot();
            super::audit_owner::log_pager_handle(before, after, message_id, result.is_ok());
        }
        result
    }

    fn handle_with_request_audit(&mut self, envelope: Value) -> Result<Value, String> {
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let mut request_audit = request_audit::RequestAuditContext::default();
        #[cfg(feature = "s_capture_native_testonly")]
        let mut request_audit = {
            let mut previous = self.conn.take_source_keeper();
            let mut context = request_audit::RequestAuditContext::default();
            if matches!(self.mode, s_allocation::Mode::CapturedNative) {
                if let Some(total) = self.audit_cache_source_bytes {
                    let legacy = crate::cache::retained_bytes();
                    let keeper_cap = total
                        .checked_sub(legacy)
                        .ok_or("S3WriterResourcesInvalid：legacy来源费用超过S7")?;
                    // #1488 P1-1/P1-2/P1-α：逐项分区；不成立即丢候选重映射，再不成立只丢可选池。
                    let partition = self
                        .conn
                        .entry_old_pool_partition(&mut previous, keeper_cap)?;
                    context.freeze_manifest_workspace_partition(total, legacy, partition)?;
                    if super::cross_request_source_cache::Keeper::retention_possible(keeper_cap) {
                        let mut keeper = previous.unwrap_or_else(|| {
                            super::cross_request_source_cache::Keeper::new(keeper_cap)
                        });
                        keeper.set_retention_cap(keeper_cap);
                        context.attach_source_keeper(keeper);
                    }
                    if std::env::var_os("S_MANIFEST_WORKSPACE_DIAGNOSTICS").as_deref()
                        == Some(std::ffi::OsStr::new("1"))
                    {
                        use std::io::Write as _;
                        let line = json!({"diagnostic":"NativeTestOnlyManifestWorkspace",
                            "stage":"request-budget-entry", "message_id":envelope.get("message_id"),
                            "budget":context.manifest_workspace_budget(),
                            "keeper_cap":keeper_cap});
                        let _ = writeln!(std::io::stderr().lock(), "{line}");
                    }
                }
            }
            context
        };
        match self.handle_in_request(envelope, &mut request_audit) {
            Ok((reply, ended)) => {
                #[cfg(feature = "s_capture_native_testonly")]
                {
                    self.conn.finish_source_keeper(request_audit, ended);
                }
                #[cfg(not(feature = "s_capture_native_testonly"))]
                let _ = ended;
                Ok(reply)
            }
            Err(error) => {
                self.conn.discard_after_error();
                Err(error)
            }
        }
    }

    fn handle_in_request(
        &mut self,
        envelope: Value,
        request_audit: &mut request_audit::RequestAuditContext,
    ) -> Result<(Value, super::audit_owner::Ended), String> {
        let envelope = validate_request(envelope)?;
        if let Some(cap) = self.audit_cache_source_bytes {
            state::check_audit_source_retention_bytes(cap, request_audit.retained_source_bytes())?;
        }
        let (admission, ended) = self.conn.admit(
            envelope,
            &self.epoch,
            &self.clock,
            &self.profile_id,
            self.volume_unit.as_deref(),
            &self.mode,
            request_audit,
        )?;
        match admission {
            super::audit_owner::Admission::Reply(reply) => Ok((reply, ended)),
            super::audit_owner::Admission::Advance { original, accepted } => {
                self.conn.continue_with(ended)?;
                #[cfg(test)]
                if accepted && self.stop_after == Some("accept") {
                    return Err("TestOnly：stop_after_accept".into());
                }
                #[cfg(not(test))]
                let _ = accepted;
                self.advance(&original, request_audit)
            }
        }
    }

    fn advance(
        &mut self,
        envelope: &Value,
        request_audit: &mut request_audit::RequestAuditContext,
    ) -> Result<(Value, super::audit_owner::Ended), String> {
        let begun = self.conn.begin_advance(
            envelope,
            &self.epoch,
            &self.clock,
            &self.mode,
            request_audit,
        )?;
        #[cfg(test)]
        if self.stop_after == Some("begin") {
            return Err("TestOnly：stop_after_begin".into());
        }
        let (receipt, ended) =
            self.conn
                .publish(&mut self.runtime, &begun.token, envelope, request_audit)?;
        drop(begun);
        Ok((receipt.message_receipt, ended))
    }

    fn resume_pending(
        &mut self,
        mut bootstrap_context: Option<&mut request_audit::RequestAuditContext>,
    ) -> Result<(), String> {
        let originals = self.conn.pending_originals()?;
        if originals.len() > 1 {
            return Err(
                "InvalidDomain：存在多条待提交原件，不能用单条Begin合并发布；保留原库等待逐条处置"
                    .into(),
            );
        }
        for raw in originals {
            // #1394：bootstrap slot不得跨入恢复请求；idle无pending才可复用到identity。
            if let Some(context) = bootstrap_context.as_deref_mut() {
                drop(context.take());
            }
            let mut request_audit = request_audit::RequestAuditContext::default();
            self.advance(&strict_json(raw.as_bytes())?, &mut request_audit)?;
        }
        Ok(())
    }
}

/// 监督器恢复：完整旧状态有效且epoch严格上升后才清Begin，不重写已接纳消息。
/// 尚未接CLI；换代事务提交后，由新Driver开门继续持久pending原件。
pub(super) fn recover(
    db: &Path,
    new_epoch: &str,
    clock: &v2::ClockPlan,
    clock_event: &str,
) -> Result<Value, String> {
    parse_writer_epoch(new_epoch, "new writer_epoch")?;
    if !db.is_file() {
        return Err("StorageUnavailable：恢复需要已初始化库".into());
    }
    let _lock = session_transport::process_lock(db, "S")?;
    let conn = super::audit_owner::AuditConnection::open(db)?;
    conn.recover(new_epoch, clock, clock_event)
}
fn err(e: impl std::fmt::Display) -> String {
    format!("S3DriverInvalid：{e}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::located_ohlc_input;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        path: PathBuf,
        db: PathBuf,
        profile: PathBuf,
        clock: v2::ClockPlan,
        preserve: bool,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if !self.preserve {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }
    impl Fixture {
        fn new() -> Self {
            Self::with_clock(
                json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"s3-driver","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{
                "input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"},{"begin_ns":"14","commit_ns":"15"}]},
                "input-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"22"},{"begin_ns":"24","commit_ns":"25"}]},
                "input-3":{"accept_ns":"30","attempts":[{"begin_ns":"31","commit_ns":"32"},{"begin_ns":"34","commit_ns":"35"}]},
                "recover-1":{"recover_ns":"13"},"recover-2":{"recover_ns":"23"}}}),
            )
        }
        fn with_clock(clock_value: Value) -> Self {
            let path = std::env::temp_dir().join(format!(
                "s1394-driver-{}-{}-{}",
                std::process::id(),
                now_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            let cp = path.join("clock.json");
            std::fs::write(&cp, canonical_json(&clock_value)).unwrap();
            let base = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("s_session");
            let profile = base.join("profiles/ohlc_integer_tb02a_v1.json");
            let db = path.join("s.sqlite");
            init_v3(
                &db,
                "s-session-testonly-001",
                &base.join("catalog/signed-catalog.json"),
                &profile,
                "1",
                &cp,
                4,
            )
            .unwrap();
            Self {
                path,
                db,
                profile,
                clock: v2::ClockPlan::parse(clock_value).unwrap(),
                preserve: false,
            }
        }
        fn open(&self, epoch: &str) -> Result<Driver, String> {
            Driver::open(&self.db, &self.profile, epoch, self.clock.clone())
        }
        fn ingest(n: i64) -> Value {
            let row = match n {
                1 => ("a", 0, 6, 12, 3, 7),
                2 => ("b", 10, 8, 16, 6, 12),
                3 => ("c", 20, 8, 13, 4, 7),
                _ => panic!("driver Fixture only defines three inputs"),
            };
            let raw = located_ohlc_input(&[row]);
            envelope(
                &format!("message-{n}"),
                json!({"op":"ingest","target_session_id":"s-session-testonly-001","target_session_generation":"1","writer_epoch":"1","clock_event_id":format!("input-{n}"),"raw_input":raw}),
            )
        }
    }
    fn envelope(id: &str, payload: Value) -> Value {
        json!({"schema_revision":"s-session/3","session_id":"s-session-testonly-001","session_generation":"1","source_namespace":"testonly.ohlc","source_epoch":"1","message_id":id,"producer_id":"driver-fixture","producer_epoch":"1","payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload})
    }
    fn receipt_request(e: &Value) -> Value {
        envelope(
            "receipt-query",
            json!({"op":"receipt","original_source_namespace":e["source_namespace"],"original_source_epoch":e["source_epoch"],"original_message_id":e["message_id"],"original_payload_hash":e["payload_hash"]}),
        )
    }
    fn count(c: &Connection, t: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .unwrap()
    }

    /// #1405：只有规则元数据为历史 /1 的空库也不准新写者接纳/恢复；Q 的只读审计仍可读。
    #[test]
    fn historical_bi_rule_is_readable_but_writer_and_recover_leave_it_unchanged() {
        let f = Fixture::new();
        let mut c = open_db(&f.db).unwrap();
        meta_set(&c, "rule_revision", crate::LEGACY_V3_RULE_REVISION).unwrap();
        let before = db_meta(&c).unwrap();
        {
            let tx = c.transaction().unwrap();
            state::audit(&tx).expect("历史规则只读审计");
        }
        assert!(f.open("1").err().unwrap().contains("VersionUnsupported"));
        assert!(recover(&f.db, "2", &f.clock, "recover-1")
            .err()
            .unwrap()
            .contains("VersionUnsupported"));
        assert_eq!(db_meta(&c).unwrap(), before);
        assert_eq!(count(&c, "raw_events"), 0);
        assert_eq!(count(&c, "s_input_messages"), 0);
    }

    #[test]
    fn audit_owner_driver_normal_requests_release_transactions_and_statements() {
        fn ended(driver: &Driver) {
            assert!(driver.conn.is_autocommit());
            // 只检查本测试所有者的 statement 生命周期，不写库、不注入错误。
            let statement = unsafe {
                rusqlite::ffi::sqlite3_next_stmt(driver.conn.handle(), std::ptr::null_mut())
            };
            assert!(statement.is_null(), "请求结束仍有 statement 存活");
        }
        let fixture = Fixture::new();
        let mut driver = fixture.open("1").unwrap();
        ended(&driver);
        for n in 1..=3 {
            let input = Fixture::ingest(n);
            let receipt = driver.handle(input.clone()).unwrap();
            ended(&driver);
            for _ in 0..2 {
                assert_eq!(driver.handle(input.clone()).unwrap(), receipt);
                ended(&driver);
                assert_eq!(driver.handle(receipt_request(&input)).unwrap(), receipt);
                ended(&driver);
                let ready = driver
                    .handle(envelope("ready", json!({"op":"ready"})))
                    .unwrap();
                assert_eq!(ready["generation"], n.to_string());
                ended(&driver);
            }
        }
        let identity = driver.identity().unwrap();
        ended(&driver);
        let summary = driver.state_summary("normal-owner").unwrap();
        ended(&driver);
        drop(driver);
        let reopened = fixture.open("1").unwrap();
        assert_eq!(reopened.identity().unwrap(), identity);
        assert_eq!(reopened.state_summary("normal-owner").unwrap(), summary);
        ended(&reopened);
    }

    #[test]
    fn audit_owner_normal_idle_recover_and_reopen_preserve_old_receipt() {
        let fixture = Fixture::new();
        let input = Fixture::ingest(1);
        let mut before = fixture.open("1").unwrap();
        let receipt = before.handle(input.clone()).unwrap();
        let first_summary = before.state_summary("before-recover").unwrap();
        drop(before);
        let recovered = recover(&fixture.db, "2", &fixture.clock, "recover-1").unwrap();
        assert_eq!(
            recovered["reachable_root"]["structure_cut"],
            first_summary["structure_cut"]
        );
        let mut after = fixture.open("2").unwrap();
        assert_eq!(after.handle(receipt_request(&input)).unwrap(), receipt);
        let mut second = Fixture::ingest(2);
        second["payload"]["writer_epoch"] = json!("2");
        second["payload_hash"] = json!(sha256_hex(canonical_json(&second["payload"]).as_bytes()));
        assert_eq!(after.handle(second).unwrap()["kind"], "Committed");
        assert_eq!(
            after.state_summary("after-recover").unwrap()["generation"],
            "2"
        );
        assert!(after.conn.is_autocommit());
    }

    // 仅调用正常 Driver，覆盖“新消息携带既有 raw”而控制账继续变化的情况。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_new_message_raw_replay_matches_ordinary_complete_tables() {
        let native_fixture = Fixture::new();
        let ordinary_fixture = Fixture::new();
        let mut native = Driver::open_for_serve_with_mode(
            &native_fixture.db,
            &native_fixture.profile,
            "1",
            native_fixture.clock.clone(),
            536_870_912,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        let mut ordinary = ordinary_fixture.open("1").unwrap();
        let first = Fixture::ingest(1);
        let first_reply = native.handle(first.clone()).unwrap();
        assert_eq!(ordinary.handle(first.clone()).unwrap(), first_reply);
        let first_root: Vec<u8> = native
            .conn
            .query_row(
                "SELECT snapshot_root FROM s_content_publications WHERE generation=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        for n in 2..=3 {
            native
                .handle(envelope("establish-before-replay", json!({"op":"ready"})))
                .unwrap();
            assert!(native.conn.complete_retained_bytes() > 0);
            let mut next = Fixture::ingest(n);
            next["payload"]["raw_input"] = first["payload"]["raw_input"].clone();
            next["payload_hash"] = json!(sha256_hex(canonical_json(&next["payload"]).as_bytes()));
            let ordinary_result = ordinary.handle(next.clone());
            let native_result = native.handle(next.clone());
            eprintln!("normal new-message raw replay: ordinary={ordinary_result:?}; native={native_result:?}");
            assert_eq!(native_result, ordinary_result);
            let actual = native_result.unwrap();
            assert!(
                native.conn.complete_retained_bytes() > 0,
                "新 message replay 必须由实际 Accept Q 效果取得写后资格"
            );
            native.conn.assert_complete_matches_cold();
            assert_eq!(actual["kind"], "Committed");
            assert_eq!(count(&native.conn, "raw_events"), 1);
            assert_eq!(count(&native.conn, "raw_ohlc"), 1);
            assert_eq!(count(&native.conn, "s_input_messages"), n);
            assert_eq!(count(&native.conn, "s_clock_events"), n + 2);
            assert_eq!(count(&native.conn, "s_content_publications"), 1);
            assert_eq!(
                prepared::tests::runtime_root_tables(&native.conn, true),
                prepared::tests::runtime_root_tables(&ordinary.conn, true),
            );
            let before_duplicate = prepared::tests::runtime_root_tables(&native.conn, true);
            assert_eq!(native.handle(next.clone()).unwrap(), actual);
            assert_eq!(native.handle(receipt_request(&next)).unwrap(), actual);
            assert_eq!(
                prepared::tests::runtime_root_tables(&native.conn, true),
                before_duplicate,
            );
            let ready = envelope("ready-replay", json!({"op":"ready"}));
            assert_eq!(
                native.handle(ready.clone()).unwrap(),
                ordinary.handle(ready).unwrap()
            );
            assert_eq!(native.handle(receipt_request(&first)).unwrap(), first_reply);
            let tx = native.conn.transaction().unwrap();
            let audited = state::audit(&tx).unwrap();
            assert_eq!(audited.cuts.len(), 1);
            assert_eq!(audited.raw.len(), 1);
            assert_eq!(
                audited.cuts[0].snapshot.digest().unwrap().as_slice(),
                first_root
            );
            for cut in &audited.cuts {
                assert_eq!(cut.metadata.input_frontier, 0);
            }
            drop(audited);
            tx.rollback().unwrap();
        }
    }

    /// #1394 完整 owner 的验收锁：只有实际省去旧 C_g 才能通过。
    /// 不能将此断言改成接受 cold 重扫，也不能把事务入口收拢当作复用已完成。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_native_no_write_must_not_repeat_all_old_cut_semantics() {
        let fixture = Fixture::new();
        let mut driver = Driver::open_for_serve_with_mode(
            &fixture.db,
            &fixture.profile,
            "1",
            fixture.clock.clone(),
            536_870_912,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        for n in 1..=3 {
            let (receipt, events, work) = state::traced(true, || driver.handle(Fixture::ingest(n)));
            assert_eq!(receipt.unwrap()["kind"], "Committed");
            eprintln!(
                "{}",
                json!({"scenario":"owner-normal-append","generation":n,
                "history_cut_checks":events.iter().filter(|e| e["phase"] == "history_template_closure").count(),
                "audit_work":work})
            );
        }
        let request = envelope("ready-owner", json!({"op":"ready"}));
        let expected = driver.handle(request.clone()).unwrap();
        let reads = content_store::sql_reads_on_thread();
        let capture_before = content_store::prototype_capture_work_on_thread();
        let hashes = newchan_rust::s_content::frame_calls_on_thread();
        let started = std::time::Instant::now();
        let (actual, events, work) = state::traced(true, || driver.handle(request));
        assert_eq!(actual.unwrap(), expected);
        let old_cuts = events
            .iter()
            .filter(|e| e["phase"] == "history_template_closure")
            .count();
        let capture_after = content_store::prototype_capture_work_on_thread();
        eprintln!(
            "{}",
            json!({"scenario":"owner-repeated-ready","history_cut_checks":old_cuts,
                "individual_node_sql_reads":content_store::sql_reads_on_thread()-reads,
                "capture_node_scans":capture_after.node_scans-capture_before.node_scans,
                "capture_rows":capture_after.decoded_rows-capture_before.decoded_rows,
                "capture_body_bytes":capture_after.body_bytes-capture_before.body_bytes,
            "frame_hash_calls":newchan_rust::s_content::frame_calls_on_thread()-hashes,
            "elapsed_ns":started.elapsed().as_nanos().to_string(),"audit_work":work,
            "source_retention":driver.conn.source_keeper_for_test().as_ref().unwrap().report(),
            "complete_retention":driver.conn.complete_report(),
            "remaining_clock_checks":events.iter().filter(|e|e["phase"]=="request_clock_full_check").collect::<Vec<_>>()})
        );
        assert_eq!(
            old_cuts, 0,
            "重复无写仍执行旧 C_g：完整事务 owner 与历史保持证明尚未接通"
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    fn complete_driver(f: &Fixture) -> Driver {
        Driver::open_for_serve_with_mode(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            536_870_912,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap()
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_complete_ready_receipt_repost_keep_owned_state_and_match_ordinary() {
        let f = Fixture::new();
        let o = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        for n in 1..=3 {
            let e = Fixture::ingest(n);
            assert_eq!(
                native.handle(e.clone()).unwrap(),
                ordinary.handle(e).unwrap()
            );
        }
        assert!(
            native.conn.complete_report()["charge"]["incremental_steps"]
                .as_u64()
                .unwrap_or(0)
                >= 9,
            "三次正常写应完整消费 owner effects 与新代"
        );
        let ready = envelope("ready-complete", json!({"op":"ready"}));
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready.clone()).unwrap()
        );
        native.conn.assert_complete_matches_cold();
        let pointers = native.conn.complete_pointers();
        let before = prepared::tests::runtime_root_tables(&native.conn, true);
        for request in [
            ready.clone(),
            receipt_request(&Fixture::ingest(2)),
            Fixture::ingest(3),
            ready,
        ] {
            let expected = ordinary.handle(request.clone()).unwrap();
            let capture = content_store::prototype_capture_work_on_thread();
            let hashes = newchan_rust::s_content::frame_calls_on_thread();
            let start = std::time::Instant::now();
            let (actual, events, work) = state::traced(true, || native.handle(request.clone()));
            assert_eq!(actual.unwrap(), expected);
            assert!(events.iter().any(|e| e["phase"] == "owner_complete_hit"));
            assert!(!events
                .iter()
                .any(|e| e["phase"] == "history_template_closure"));
            let after = content_store::prototype_capture_work_on_thread();
            assert_eq!(after.capture_builds, capture.capture_builds);
            assert_eq!(after.node_scans, capture.node_scans);
            assert_eq!(after.decoded_rows, capture.decoded_rows);
            assert_eq!(newchan_rust::s_content::frame_calls_on_thread(), hashes);
            assert_eq!(
                native.conn.complete_pointers(),
                pointers,
                "全raw/历史/SQL原件不得复制"
            );
            assert_eq!(
                prepared::tests::runtime_root_tables(&native.conn, true),
                before
            );
            assert_eq!(
                before,
                prepared::tests::runtime_root_tables(&ordinary.conn, true)
            );
            eprintln!(
                "{}",
                json!({"scenario":"owner-complete-no-write","op":request["payload"]["op"],"elapsed_ns":start.elapsed().as_nanos().to_string(),"events":events,"work":work,"retention":native.conn.complete_report()})
            );
        }
        native.conn.assert_complete_matches_cold();
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification() {
        fn context(d: &mut Driver) -> request_audit::RequestAuditContext {
            let mut context = request_audit::RequestAuditContext::default();
            if matches!(d.mode, s_allocation::Mode::CapturedNative) {
                let total = d.audit_cache_source_bytes.unwrap();
                let legacy = crate::cache::retained_bytes();
                let keeper_cap = total.checked_sub(legacy).unwrap();
                let mut keeper = d.conn.take_source_keeper().expect("成功链保留原来源池");
                // #1488 P1-1：入口旧费走生产同一逐项分区（O+uncovered），断言不变。
                let partition = d.conn.old_pool_partition(Some(&keeper)).unwrap();
                let old_pool = partition.carried();
                assert!(old_pool <= keeper_cap);
                context
                    .freeze_manifest_workspace_partition(total, legacy, partition)
                    .unwrap();
                keeper.set_retention_cap(keeper_cap);
                context.attach_source_keeper(keeper);
            }
            context
        }
        fn same(n: &Driver, o: &Driver, prior: &Value, raw: u64, published: u64) -> Value {
            let current = n.conn.complete_report();
            assert!(n.conn.complete_retained_bytes() > 0);
            assert_eq!(current["life"], prior["life"]);
            assert!(
                current["transaction"].as_u64().unwrap() > prior["transaction"].as_u64().unwrap()
            );
            assert!(
                current["write_sequence"].as_u64().unwrap()
                    > prior["write_sequence"].as_u64().unwrap()
            );
            assert_eq!(current["raw_checked"], raw);
            assert_eq!(current["published_prefix"], published);
            n.conn.assert_complete_matches_cold();
            assert_eq!(
                prepared::tests::runtime_root_tables(&n.conn, true),
                prepared::tests::runtime_root_tables(&o.conn, true)
            );
            current
        }
        let f = Fixture::new();
        let o = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        let first = Fixture::ingest(1);
        assert_eq!(
            native.handle(first.clone()).unwrap(),
            ordinary.handle(first).unwrap()
        );
        let ready = envelope("ready-effects", json!({"op":"ready"}));
        native.handle(ready.clone()).unwrap();
        assert!(native.conn.complete_retained_bytes() > 0);
        let mut prior = native.conn.complete_report();
        let second = Fixture::ingest(2);
        // 首 Commit12 → Accept20 → Begin21 → Recover23 → Begin24 → Commit25。
        for d in [&mut native, &mut ordinary] {
            let mut context = context(d);
            let (admitted, ended) = d
                .conn
                .admit(
                    second.clone(),
                    &d.epoch,
                    &d.clock,
                    &d.profile_id,
                    d.volume_unit.as_deref(),
                    &d.mode,
                    &mut context,
                )
                .unwrap();
            assert!(matches!(
                admitted,
                super::super::audit_owner::Admission::Advance { accepted: true, .. }
            ));
            d.conn.finish_source_keeper(context, ended);
        }
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready.clone()).unwrap()
        );
        prior = same(&native, &ordinary, &prior, 2, 1);
        let mut ready_after_begin = Vec::new();
        for d in [&mut native, &mut ordinary] {
            let mut context = context(d);
            let begun = d
                .conn
                .begin_advance(&second, &d.epoch, &d.clock, &d.mode, &mut context)
                .unwrap();
            drop(begun);
            // Begin 已由 owner 正常提交；复用本请求预算和来源池完成真实 ready，
            // 以该无写事务的 Ended 归还 keeper，不手造结果或丢池后重新授信。
            let (reply, ended) = d
                .conn
                .admit(
                    ready.clone(),
                    &d.epoch,
                    &d.clock,
                    &d.profile_id,
                    d.volume_unit.as_deref(),
                    &d.mode,
                    &mut context,
                )
                .unwrap();
            let super::super::audit_owner::Admission::Reply(reply) = reply else {
                panic!("ready必须返回原回执")
            };
            ready_after_begin.push(reply);
            d.conn.finish_source_keeper(context, ended);
        }
        assert_eq!(ready_after_begin[0], ready_after_begin[1]);
        prior = same(&native, &ordinary, &prior, 2, 1);
        let nr = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap();
        let or = ordinary
            .conn
            .recover("2", &ordinary.clock, "recover-2")
            .unwrap();
        assert_eq!(nr, or);
        assert!(native.conn.complete_retained_bytes() > 0);
        native.epoch = "2".into();
        ordinary.epoch = "2".into();
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready.clone()).unwrap()
        );
        prior = same(&native, &ordinary, &prior, 2, 1);
        let mut committed = Vec::new();
        for d in [&mut native, &mut ordinary] {
            let mut context = context(d);
            let (reply, ended) = d.advance(&second, &mut context).unwrap();
            assert_eq!(reply["kind"], "Committed");
            committed.push(reply);
            d.conn.finish_source_keeper(context, ended);
        }
        assert_eq!(committed[0], committed[1]);
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready.clone()).unwrap()
        );
        prior = same(&native, &ordinary, &prior, 2, 2);
        let mut revision = Fixture::ingest(3);
        let raw = &mut revision["payload"]["raw_input"];
        *raw = second["payload"]["raw_input"].clone();
        raw["events"][0]["revision"] = json!("2");
        raw["events"][0]["high"] = json!("17");
        revision["payload"]["writer_epoch"] = json!("2");
        revision["payload_hash"] =
            json!(sha256_hex(canonical_json(&revision["payload"]).as_bytes()));
        assert_eq!(
            native.handle(revision.clone()).unwrap(),
            ordinary.handle(revision).unwrap()
        );
        assert!(native.conn.complete_retained_bytes() > 0);
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready).unwrap()
        );
        same(&native, &ordinary, &prior, 3, 3);
        eprintln!(
            "{}",
            json!({"scenario":"owner-normal-accept-begin-recover-retry-revision","complete":native.conn.complete_report()})
        );
    }

    // ---- #1488 受控同owner Recover迁移锁（0031）----
    #[cfg(feature = "s_capture_native_testonly")]
    fn recover_lock_context(d: &mut Driver) -> request_audit::RequestAuditContext {
        let mut context = request_audit::RequestAuditContext::default();
        let total = d.audit_cache_source_bytes.unwrap();
        let legacy = crate::cache::retained_bytes();
        let keeper_cap = total.checked_sub(legacy).unwrap();
        let mut keeper = d.conn.take_source_keeper().expect("成功链保留原来源池");
        let partition = d.conn.old_pool_partition(Some(&keeper)).unwrap();
        context
            .freeze_manifest_workspace_partition(total, legacy, partition)
            .unwrap();
        keeper.set_retention_cap(keeper_cap);
        context.attach_source_keeper(keeper);
        context
    }
    /// 同 test 01 前段：Commit12 → Accept20 → ready → Begin21 → ready；停在 Recover 之前。
    /// owner 此时持有已提交构造资格与同快照完整原件，持久 writer_epoch=1、begin_token 非空。
    #[cfg(feature = "s_capture_native_testonly")]
    fn recover_lock_driver(f: &Fixture) -> (Driver, Value, Value) {
        let mut native = complete_driver(f);
        native.handle(Fixture::ingest(1)).unwrap();
        let ready = envelope("ready-recover-lock", json!({"op":"ready"}));
        native.handle(ready.clone()).unwrap();
        let second = Fixture::ingest(2);
        let mut context = recover_lock_context(&mut native);
        let (admitted, ended) = native
            .conn
            .admit(
                second.clone(),
                &native.epoch,
                &native.clock,
                &native.profile_id,
                native.volume_unit.as_deref(),
                &native.mode,
                &mut context,
            )
            .unwrap();
        assert!(matches!(
            admitted,
            super::super::audit_owner::Admission::Advance { accepted: true, .. }
        ));
        native.conn.finish_source_keeper(context, ended);
        native.handle(ready.clone()).unwrap();
        let mut context = recover_lock_context(&mut native);
        drop(
            native
                .conn
                .begin_advance(&second, &native.epoch, &native.clock, &native.mode, &mut context)
                .unwrap(),
        );
        let (_, ended) = native
            .conn
            .admit(
                ready.clone(),
                &native.epoch,
                &native.clock,
                &native.profile_id,
                native.volume_unit.as_deref(),
                &native.mode,
                &mut context,
            )
            .unwrap();
        native.conn.finish_source_keeper(context, ended);
        let probe = native.conn.recover_probe_for_test();
        assert!(!probe.stale && !probe.retired);
        let q = probe.qualification.expect("Recover前owner持有已提交构造资格");
        assert!(q.writer_epoch.is_none());
        assert!(native.conn.complete_retained_bytes() > 0);
        assert_eq!(recover_lock_meta(f, "writer_epoch").as_deref(), Some("1"));
        assert_ne!(recover_lock_meta(f, "begin_token").as_deref(), Some(""));
        (native, second, ready)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn recover_lock_meta(f: &Fixture, key: &str) -> Option<String> {
        use rusqlite::OptionalExtension;
        Connection::open(&f.db)
            .unwrap()
            .query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0))
            .optional()
            .unwrap()
    }
    /// 第二连接两次真实提交、终态内容不变：只有版本能看出外写（同值 UPDATE 不落盘，不算外写）。
    #[cfg(feature = "s_capture_native_testonly")]
    fn recover_lock_external_write(f: &Fixture) {
        let c = Connection::open(&f.db).unwrap();
        c.execute("UPDATE meta SET value=value||'~' WHERE key='generation'", [])
            .unwrap();
        c.execute(
            "UPDATE meta SET value=substr(value,1,length(value)-1) WHERE key='generation'",
            [],
        )
        .unwrap();
    }
    /// 失败的 Recover 不得晋升：租约撤销、资格不在 owner、持久 epoch 不动。
    #[cfg(feature = "s_capture_native_testonly")]
    fn assert_recover_revoked(native: &Driver, f: &Fixture, serial_before: u64, epoch: &str) {
        let probe = native.conn.recover_probe_for_test();
        assert!(probe.stale, "失败后租约必须撤销：{probe:?}");
        assert!(probe.serial > serial_before, "撤销须推进租约序号：{probe:?}");
        assert!(probe.qualification.is_none(), "失败后不得留资格：{probe:?}");
        assert_eq!(recover_lock_meta(f, "writer_epoch").as_deref(), Some(epoch));
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_same_owner_promotes_and_voids_old_epoch_token_and_lease() {
        let f = Fixture::with_clock(
            json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"s3-driver","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{
            "input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"},{"begin_ns":"14","commit_ns":"15"}]},
            "input-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"22"},{"begin_ns":"24","commit_ns":"25"}]},
            "input-3":{"accept_ns":"30","attempts":[{"begin_ns":"31","commit_ns":"32"},{"begin_ns":"34","commit_ns":"35"}]},
            "recover-1":{"recover_ns":"13"},"recover-2":{"recover_ns":"23"},"recover-3":{"recover_ns":"26"}}}),
        );
        let (mut native, second, ready) = recover_lock_driver(&f);
        let before = native.conn.recover_probe_for_test();
        let old = before.qualification.unwrap();
        let reply = native.conn.recover("2", &native.clock, "recover-2").unwrap();
        assert!(!reply["recovered_begin"].is_null());
        let after = native.conn.recover_probe_for_test();
        assert!(!after.stale && !after.retired, "{after:?}");
        // 旧租约：真实 COMMIT 后 close 推进序号，旧 raw token 绑定的 serial 不再匹配。
        assert!(after.serial > before.serial, "{before:?} → {after:?}");
        let promoted = after.qualification.expect("Recover成功后晋升新资格");
        assert_eq!(promoted.writer_epoch, Some(2));
        assert_eq!(promoted.binding, old.binding);
        assert_eq!(promoted.has_fee, old.has_fee);
        // 旧 Begin token 随持久换代清空。
        assert_eq!(recover_lock_meta(&f, "writer_epoch").as_deref(), Some("2"));
        assert_eq!(recover_lock_meta(&f, "begin_token").as_deref(), Some(""));
        assert_eq!(recover_lock_meta(&f, "advance_state").as_deref(), Some("idle"));
        // 资格戳=持久 epoch，下一次 Recover 从该戳出发仍可迁移。
        native.conn.recover("3", &native.clock, "recover-3").unwrap();
        let chained = native.conn.recover_probe_for_test();
        assert!(chained.serial > after.serial);
        assert_eq!(chained.qualification.unwrap().writer_epoch, Some(3));
        // 现 epoch 的无写请求保留资格并归还来源池；随后旧 epoch 请求被拒，且不保留任何资格。
        native.epoch = "3".into();
        native.handle(ready).unwrap();
        let chained = native.conn.recover_probe_for_test();
        assert!(!chained.stale, "{chained:?}");
        assert_eq!(chained.qualification.unwrap().writer_epoch, Some(3));
        native.epoch = "2".into();
        let mut context = recover_lock_context(&mut native);
        let error = native
            .advance(&second, &mut context)
            .err()
            .expect("旧epoch推进必须被拒");
        assert!(error.contains("StaleWriter"), "{error}");
        drop(context);
        assert_recover_revoked(&native, &f, chained.serial, "3");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_non_increasing_epoch() {
        use super::super::audit_owner::AuditConnection;
        assert!(!AuditConnection::recover_handoff_accepts_for_test(2, 2));
        assert!(!AuditConnection::recover_handoff_accepts_for_test(2, 1));
        assert!(AuditConnection::recover_handoff_accepts_for_test(1, 2));
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        let error = native
            .conn
            .recover("1", &native.clock, "recover-2")
            .unwrap_err();
        assert!(error.contains("StaleWriter"), "{error}");
        // 0034 P3-3：生产 owner.recover 门即 RecoverHandoff 构造门，具体原因逐字锁定。
        assert_eq!(error, "StaleWriter：recover换代必须严格上升");
        assert_recover_revoked(&native, &f, serial, "1");
        // 严格下降（晋升到 2 后回退到 1）同样在同一门被拒。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let serial = native.conn.recover_probe_for_test().serial;
        let error = native
            .conn
            .recover("1", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(error, "StaleWriter：recover换代必须严格上升");
        assert_recover_revoked(&native, &f, serial, "2");
        // 资格戳门：严格上升的证明若与资格戳不符，迁移门拒绝（见 recover_lock_rejects_stale_qualification_stamp）。
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_unclosed_recover_effects() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        // Recover 不声明 s_input_messages 写；精确 effects 必须拒绝而非晋升。
        native.conn.set_recover_after_write_for_test(|tx| {
            tx.execute(
                "UPDATE s_input_messages SET status=status WHERE rowid=(SELECT min(rowid) FROM s_input_messages)",
                [],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        });
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        eprintln!("{}", json!({"scenario":"recover-unclosed-effects","error":error}));
        // 0034 P3-4：拒绝原因逐字锁定为精确 effects 门，而非任意失败。
        assert_eq!(error, "S3DriverInvalid：固定 operation 出现未声明实际效果");
        assert_recover_revoked(&native, &f, serial, "1");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_commit_failure_and_rollback_never_promote() {
        // 写后回滚：Recover 写已落事务、effects 闭合前出错。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native
            .conn
            .set_recover_after_write_for_test(|_| Err("测试注入Recover写后回滚".into()));
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert!(error.contains("测试注入Recover写后回滚"), "{error}");
        assert_recover_revoked(&native, &f, serial, "1");
        // 真实 SQLite COMMIT 失败：effects 已闭合、epoch 已确认，仍不得晋升。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.commit_hook(Some(|| true)).unwrap();
        assert!(native
            .conn
            .recover("2", &native.clock, "recover-2")
            .is_err());
        native.conn.commit_hook(None::<fn() -> bool>).unwrap();
        assert_recover_revoked(&native, &f, serial, "1");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_external_write_and_read_domain_change_still_revoke() {
        // Recover 前外写：不得以旧资格迁移。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        let version = |c: &Connection| -> i64 {
            c.query_row("PRAGMA main.data_version", [], |r| r.get(0))
                .unwrap()
        };
        let before = version(&native.conn);
        recover_lock_external_write(&f);
        assert_ne!(version(&native.conn), before, "外写必须真实提交");
        assert!(native
            .conn
            .recover("2", &native.clock, "recover-2")
            .is_err());
        assert_recover_revoked(&native, &f, serial, "1");
        // Recover 前读域变化（连接级 PRAGMA）：同样撤销。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        let recursive: i64 = native
            .conn
            .query_row("PRAGMA recursive_triggers", [], |r| r.get(0))
            .unwrap();
        native
            .conn
            .execute_batch(&format!("PRAGMA recursive_triggers={}", 1 - recursive))
            .unwrap();
        assert!(native
            .conn
            .recover("2", &native.clock, "recover-2")
            .is_err());
        assert_recover_revoked(&native, &f, serial, "1");
        // 迁移成功后的外写：新资格同样被撤销，不因 Recover 晋升而豁免。
        let f = Fixture::new();
        let (mut native, _, ready) = recover_lock_driver(&f);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let promoted = native.conn.recover_probe_for_test();
        assert_eq!(promoted.qualification.unwrap().writer_epoch, Some(2));
        recover_lock_external_write(&f);
        native.epoch = "2".into();
        let _ = native.handle(ready);
        let probe = native.conn.recover_probe_for_test();
        assert!(probe.stale && probe.serial > promoted.serial, "{probe:?}");
        assert!(probe.qualification.is_none(), "{probe:?}");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_handoff_failure_revokes() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.inject_recover_handoff_failure_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert!(error.contains("测试注入Recover交接失败"), "{error}");
        assert_recover_revoked(&native, &f, serial, "1");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_budget_live_and_peak_continue_across_recover() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let old = native.conn.recover_probe_for_test().qualification.unwrap();
        let (mark, live, peak) = native.conn.complete_ledger_for_test().unwrap();
        assert!(live > 0 && peak >= live);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        // 同一父账（不换空账），live 不被抹零、peak 不回落，状态块许可随资格 move。
        assert!(native.conn.complete_on_ledger_for_test(&mark), "Recover后换了父账");
        let (_, live_after, peak_after) = native.conn.complete_ledger_for_test().unwrap();
        assert!(live_after >= live, "live被抹：{live} → {live_after}");
        assert!(peak_after >= peak, "peak被抹：{peak} → {peak_after}");
        assert_eq!(mark.current(), Some(live_after));
        let promoted = native.conn.recover_probe_for_test().qualification.unwrap();
        assert_eq!(promoted.has_fee, old.has_fee);
        assert_eq!(promoted.retained, old.retained);
        eprintln!(
            "{}",
            json!({"scenario":"recover-budget-continuity","live":[live,live_after],"peak":[peak,peak_after],"retained":promoted.retained})
        );
    }

    // ---- #1488 0034 返修锁（按独审 0032）----
    /// P2-1(a)：资格持有 fee=Some 状态块许可时 Recover 迁移，许可原样随迁移、仍挂同父账。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_fee_permit_moves_on_same_parent_ledger() {
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        let mut context = recover_lock_context(&mut native);
        let (_token, ended) = native
            .conn
            .issue_raw_qualification_for_test(&mut context)
            .unwrap();
        native.conn.finish_source_keeper(context, ended);
        let old = native.conn.recover_probe_for_test();
        let old_q = old.qualification.expect("签发后owner持有资格");
        assert!(old_q.has_fee, "须持有状态块许可：{old:?}");
        let (mark, live, peak) = native.conn.complete_ledger_for_test().unwrap();
        let fee = native
            .conn
            .qualification_fee_on_ledger_for_test(&mark)
            .expect("许可挂候选父账");
        assert!(fee > 0);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let after = native.conn.recover_probe_for_test();
        assert!(!after.stale && after.serial > old.serial, "{after:?}");
        let promoted = after.qualification.expect("Recover成功后晋升新资格");
        assert_eq!(promoted.writer_epoch, Some(2));
        assert!(promoted.has_fee, "许可被丢弃：{promoted:?}");
        assert_eq!(promoted.retained, old_q.retained);
        assert_eq!(promoted.binding, old_q.binding);
        assert!(native.conn.complete_on_ledger_for_test(&mark), "Recover后换了父账");
        assert_eq!(
            native.conn.qualification_fee_on_ledger_for_test(&mark),
            Some(fee),
            "许可换账或改字节"
        );
        let (_, live_after, peak_after) = native.conn.complete_ledger_for_test().unwrap();
        assert!(live_after >= live, "live被抹：{live} → {live_after}");
        assert!(peak_after >= peak, "peak被抹：{peak} → {peak_after}");
        eprintln!(
            "{}",
            json!({"scenario":"recover-fee-permit","fee":fee,"live":[live,live_after],"peak":[peak,peak_after]})
        );
    }

    /// P2-1(b)：无候选 / 无当前父账时的 Recover 被拒，原因逐字锁定。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_missing_candidate_or_parent_ledger() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.drop_complete_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：已有增量游标缺连续同域原件；禁止完整核回退"
        );
        assert_recover_revoked(&native, &f, serial, "1");
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.expire_complete_parent_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(error, "StaleWriter：construction资格失效：Recover迁移缺父账交接摘要");
        assert_recover_revoked(&native, &f, serial, "1");
    }

    /// P2-1(c)：Recover 晋升后，旧 serial 的 raw token 与旧 Begin token 实际发起调用均被拒。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_old_raw_token_and_begin_token_rejected_after_promotion() {
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        let mut context = recover_lock_context(&mut native);
        let (token, ended) = native
            .conn
            .issue_raw_qualification_for_test(&mut context)
            .unwrap();
        native.conn.finish_source_keeper(context, ended);
        // 正控：改记为 Recover 前当期序号后可实际绑定。
        let token = native.conn.restamp_raw_token_for_test(token);
        let token = native
            .conn
            .bind_raw_token_for_test(token)
            .expect("Recover前当期token可绑定");
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        assert!(!native.conn.recover_probe_for_test().stale);
        let error = native
            .conn
            .bind_raw_token_for_test(token)
            .err()
            .expect("旧serial raw token必须被拒");
        assert!(error.contains("raw token 已结束、回滚或撤销"), "{error}");
        // 旧 Begin token：生产形态（不经测试选择器签发）晋升后，现 epoch 无写请求归还来源池，
        // 再以真实来源上下文实际发起发布。
        let f = Fixture::new();
        let (mut native, second, _) = recover_lock_driver(&f);
        let old_begin = recover_lock_meta(&f, "begin_token").unwrap();
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        native.epoch = "2".into();
        native
            .handle(envelope("ready-recover-old-begin", json!({"op":"ready"})))
            .unwrap();
        assert!(!native.conn.recover_probe_for_test().stale);
        let mut context = recover_lock_context(&mut native);
        let error = native
            .conn
            .publish(&mut native.runtime, &old_begin, &second, &mut context)
            .err()
            .expect("旧Begin token必须被拒");
        eprintln!("{}", json!({"scenario":"recover-old-begin-token","error":error}));
        assert_eq!(error, "PublicationInvalid：当前 begin_token 不符");
        assert_eq!(recover_lock_meta(&f, "begin_token").as_deref(), Some(""));
        assert_eq!(recover_lock_meta(&f, "writer_epoch").as_deref(), Some("2"));
    }

    /// P2-1(d)：schema 域变化与材料域（raw_events）外写后，Recover 迁移撤销。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_schema_and_material_change_revoke() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        let schema = |c: &Connection| -> i64 {
            c.query_row("PRAGMA main.schema_version", [], |r| r.get(0))
                .unwrap()
        };
        let before = schema(&native.conn);
        native
            .conn
            .execute_batch("CREATE TABLE main.recover_lock_schema_probe(x)")
            .unwrap();
        assert_ne!(schema(&native.conn), before, "schema域必须真实变化");
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        eprintln!("{}", json!({"scenario":"recover-schema-change","error":error}));
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：已有增量游标缺连续同域原件；禁止完整核回退"
        );
        assert_recover_revoked(&native, &f, serial, "1");
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        let c = Connection::open(&f.db).unwrap();
        let rows = c
            .execute(
                "UPDATE raw_events SET raw_text=raw_text||' ' WHERE rowid=(SELECT max(rowid) FROM raw_events)",
                [],
            )
            .unwrap();
        assert_eq!(rows, 1, "材料域外写必须真实命中");
        c.execute(
            "UPDATE raw_events SET raw_text=substr(raw_text,1,length(raw_text)-1) WHERE rowid=(SELECT max(rowid) FROM raw_events)",
            [],
        )
        .unwrap();
        drop(c);
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        eprintln!("{}", json!({"scenario":"recover-material-change","error":error}));
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：已有增量游标缺连续同域原件；禁止完整核回退"
        );
        assert_recover_revoked(&native, &f, serial, "1");
    }

    /// P3-6：资格戳 ≠ 本事务持久 from 时迁移被拒。时钟含 recover-3，去掉资格戳门即迁移成功。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_stale_qualification_stamp() {
        let f = Fixture::with_clock(
            json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"s3-driver","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{
            "input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"},{"begin_ns":"14","commit_ns":"15"}]},
            "input-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"22"},{"begin_ns":"24","commit_ns":"25"}]},
            "recover-1":{"recover_ns":"13"},"recover-2":{"recover_ns":"23"},"recover-3":{"recover_ns":"26"}}}),
        );
        let (native, _, _) = recover_lock_driver(&f);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.set_qualification_epoch_for_test(Some(99));
        let error = native
            .conn
            .recover("3", &native.clock, "recover-3")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移旧资格不属于当前持久epoch"
        );
        assert_recover_revoked(&native, &f, serial, "2");
    }

    /// P3-6：交给 confirm_recover 的持久 epoch 与换代证明不符时不得晋升。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_confirm_epoch_mismatch() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.inject_recover_confirm_skew_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移effects未闭合或持久epoch与换代证明不符"
        );
        assert_recover_revoked(&native, &f, serial, "1");
    }

    /// P3-1（主控裁定）：换代证明绑定父账身份与冻结域摘要；他账或同账他域交接一律拒绝。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_foreign_ledger_budget_handoff() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        let foreign = super::super::audit_owner::native::Budget::fresh(1 << 20).unwrap();
        native.conn.inject_recover_parent_for_test(Some(foreign.clone()));
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移父账或冻结域与交接摘要不符"
        );
        assert_recover_revoked(&native, &f, serial, "1");
        drop(foreign);
        // 同父账、下一冻结域的摘要同样拒绝。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.inject_recover_parent_for_test(None);
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移父账或冻结域与交接摘要不符"
        );
        assert_recover_revoked(&native, &f, serial, "1");
        // 正控：不注入时同一驱动形态迁移成功。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        assert_eq!(
            native.conn.recover_probe_for_test().qualification.unwrap().writer_epoch,
            Some(2)
        );
    }

    // ---- #1488 0037 返修锁（按独审 0036 P2-1）----
    /// P2-1：资格提交时独立记下父账身份与冻结域；比对前候选改挂他账（交接摘要随候选，自指比对恒真）
    /// 或绕过 audit_admission 重绑点经同父账真实 handoff 进入下一冻结域，fee=None 的资格也一律拒绝；
    /// 经合法重绑点（新请求只读 admission）换域则记录随之前进、迁移成功（0041，主控裁定 (a)）。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_qualification_recorded_on_other_parent_or_domain() {
        let expected =
            "StaleWriter：construction资格失效：Recover迁移资格所记父账或冻结域与事务父账不符";
        // 他账：资格与候选分属不同 Ledger。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let probe = native.conn.recover_probe_for_test();
        let q = probe.qualification.expect("Recover前owner持有已提交构造资格");
        assert!(!q.has_fee, "本段须覆盖 fee=None：{q:?}");
        assert!(q.has_parent, "资格提交时须记父账：{q:?}");
        let foreign = super::super::audit_owner::native::Budget::fresh(1 << 30).unwrap();
        let own = native.conn.swap_complete_parent_for_test(foreign.clone());
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(error, expected);
        assert_recover_revoked(&native, &f, probe.serial, "1");
        drop((own, foreign));
        // 0041（主控裁定 (a)）同父账、下一冻结域：经合法重绑点（新请求只读 ready，audit_admission
        // bind_complete_budget）换域 → 资格记录随之前进，迁移成功。
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        let before = native.conn.complete_parent_stamp_for_test();
        native
            .handle(envelope("ready-rebind", json!({"op":"ready"})))
            .unwrap();
        assert!(!native.conn.complete_parent_matches_for_test(&before), "须真实换域");
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        assert_eq!(
            native.conn.recover_probe_for_test().qualification.unwrap().writer_epoch,
            Some(2)
        );
        // 绕过重绑点直接改域（正额度真实 handoff）：资格所记域已不是事务实际域 → 拒绝。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.advance_complete_domain_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(error, expected);
        assert_recover_revoked(&native, &f, serial, "1");
        // 正控：不注入时迁移成功，晋升资格记本事务父账。
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let promoted = native.conn.recover_probe_for_test().qualification.unwrap();
        assert_eq!(promoted.writer_epoch, Some(2));
        assert!(promoted.has_parent, "{promoted:?}");
    }

    // ---- #1488 0041 返修锁（按独审 0040 P2-1 与主控裁定 (a)）----
    /// 正控：Begin（请求 B）后同 owner 新请求 C 只读 ready——候选经 audit_admission 合法重绑点
    /// 换到 C 的冻结域，资格所记冻结域随之前进；随后同 owner Recover 成功晋升。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_promotes_after_new_request_readonly_rebind() {
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        assert!(native.conn.qualification_parent_matches_complete_for_test());
        let before = native.conn.complete_parent_stamp_for_test();
        native
            .handle(envelope("ready-after-begin", json!({"op":"ready"})))
            .unwrap();
        assert!(
            !native.conn.complete_parent_matches_for_test(&before),
            "新请求须经重绑点换到本请求冻结域"
        );
        assert!(
            native.conn.qualification_parent_matches_complete_for_test(),
            "资格所记冻结域须随合法重绑前进"
        );
        let probe = native.conn.recover_probe_for_test();
        assert!(!probe.stale && probe.qualification.is_some(), "{probe:?}");
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let promoted = native.conn.recover_probe_for_test().qualification.unwrap();
        assert_eq!(promoted.writer_epoch, Some(2));
        assert!(promoted.has_parent, "{promoted:?}");
        assert_eq!(recover_lock_meta(&f, "writer_epoch").as_deref(), Some("2"));
    }

    /// P3-3：资格缺父账记录与记录不符分开报；缺记录同样拒绝并撤销。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_qualification_without_parent_record() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.clear_qualification_parent_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移资格缺父账记录"
        );
        assert_recover_revoked(&native, &f, serial, "1");
    }

    // ---- #1488 0044 返修锁（按复审 0043 P3-1/P3-2）----
    /// P3-1：资格记录先改指他账或同账旧域，再经合法 ready 重绑——`BudgetStamp::advance` 的身份链核
    /// 不认 from，记录不前进；Recover 按记录拒绝并撤销。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rebind_does_not_advance_foreign_or_stale_qualification_record() {
        let expected =
            "StaleWriter：construction资格失效：Recover迁移资格所记父账或冻结域与事务父账不符";
        for case in ["foreign", "stale-domain"] {
            let f = Fixture::new();
            let (mut native, _, _) = recover_lock_driver(&f);
            let before = native.conn.complete_parent_stamp_for_test();
            let foreign = super::super::audit_owner::native::Budget::fresh(1 << 30).unwrap();
            let stamp = match case {
                "foreign" => foreign.handoff_stamp().unwrap(),
                _ => before.clone().rewind_domain_for_test(),
            };
            native.conn.set_qualification_parent_for_test(stamp);
            assert!(!native.conn.qualification_parent_matches_complete_for_test(), "{case}");
            native
                .handle(envelope("ready-rebind", json!({"op":"ready"})))
                .unwrap();
            assert!(
                !native.conn.complete_parent_matches_for_test(&before),
                "{case}：须经合法重绑点真实换域"
            );
            assert!(
                !native.conn.qualification_parent_matches_complete_for_test(),
                "{case}：记录不得随重绑前进"
            );
            let serial = native.conn.recover_probe_for_test().serial;
            let error = native
                .conn
                .recover("2", &native.clock, "recover-2")
                .unwrap_err();
            assert_eq!(error, expected, "{case}");
            assert_recover_revoked(&native, &f, serial, "1");
            drop(foreign);
        }
    }

    /// P3-2：receipt 轮询与 ready 同走 audit_admission 合法重绑点；Begin → receipt → Recover 成功晋升。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_promotes_after_new_request_receipt_rebind() {
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        let before = native.conn.complete_parent_stamp_for_test();
        native.handle(receipt_request(&Fixture::ingest(1))).unwrap();
        assert!(
            !native.conn.complete_parent_matches_for_test(&before),
            "receipt 须经重绑点换到本请求冻结域"
        );
        assert!(
            native.conn.qualification_parent_matches_complete_for_test(),
            "资格所记冻结域须随合法重绑前进"
        );
        native.conn.recover("2", &native.clock, "recover-2").unwrap();
        let promoted = native.conn.recover_probe_for_test().qualification.unwrap();
        assert_eq!(promoted.writer_epoch, Some(2));
        assert!(promoted.has_parent, "{promoted:?}");
        assert_eq!(recover_lock_meta(&f, "writer_epoch").as_deref(), Some("2"));
    }

    // ---- #1488 0077 P3 本票修（P3-REGISTER P08/P12）----
    /// P12（0043 P3-2 余项）：Begin 后同 owner 已提交重复 ingest，再连续两个新只读请求 C、D——
    /// 每一步都经 audit_admission 重绑点换到本请求冻结域（stamp 逐步互异），资格所记冻结域逐步
    /// 跟随；链尾 Recover 成功晋升。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_promotes_after_duplicate_ingest_and_chained_rebinds() {
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        let mut seen = vec![native.conn.complete_parent_stamp_for_test()];
        let steps = [
            ("duplicate-ingest", Fixture::ingest(1)),
            ("ready-c", envelope("ready-chain-c", json!({"op":"ready"}))),
            ("ready-d", envelope("ready-chain-d", json!({"op":"ready"}))),
        ];
        for (step, request) in steps {
            native.handle(request).unwrap();
            for earlier in &seen {
                assert!(
                    !native.conn.complete_parent_matches_for_test(earlier),
                    "{step}：须经重绑点换到本请求冻结域，且不回到链上任一旧域"
                );
            }
            assert!(
                native.conn.qualification_parent_matches_complete_for_test(),
                "{step}：资格所记冻结域须随合法重绑前进"
            );
            let probe = native.conn.recover_probe_for_test();
            assert!(
                !probe.stale && probe.qualification.is_some(),
                "{step}：{probe:?}"
            );
            seen.push(native.conn.complete_parent_stamp_for_test());
        }
        native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap();
        let promoted = native.conn.recover_probe_for_test().qualification.unwrap();
        assert_eq!(promoted.writer_epoch, Some(2));
        assert!(promoted.has_parent, "{promoted:?}");
        assert_eq!(recover_lock_meta(&f, "writer_epoch").as_deref(), Some("2"));
    }

    /// P08（0036 P3-3）：迁移时 complete 候选不属本事务快照（seal 不等）——生产不可达分支以注入
    /// 锁定：精确拒绝、不迁移、资格撤销、持久 epoch 不动。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_rejects_missing_same_snapshot_complete() {
        let f = Fixture::new();
        let (native, _, _) = recover_lock_driver(&f);
        let serial = native.conn.recover_probe_for_test().serial;
        native.conn.inject_recover_snapshot_miss_for_test();
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移缺同快照完整原件"
        );
        assert_recover_revoked(&native, &f, serial, "1");
    }

    /// P2-1：holds 门独立负控——资格所记父账与候选一致，唯状态块许可挂他账，迁移前即拒绝。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn recover_lock_holds_gate_rejects_fee_permit_on_foreign_ledger() {
        let f = Fixture::new();
        let (mut native, _, _) = recover_lock_driver(&f);
        let mut context = recover_lock_context(&mut native);
        let (_token, ended) = native
            .conn
            .issue_raw_qualification_for_test(&mut context)
            .unwrap();
        native.conn.finish_source_keeper(context, ended);
        let probe = native.conn.recover_probe_for_test();
        let q = probe.qualification.expect("签发后owner持有资格");
        assert!(q.has_fee && q.has_parent, "{q:?}");
        let foreign = super::super::audit_owner::native::Budget::fresh(1 << 20).unwrap();
        native
            .conn
            .replace_qualification_fee_for_test(foreign.reserve(64).unwrap());
        let error = native
            .conn
            .recover("2", &native.clock, "recover-2")
            .unwrap_err();
        assert_eq!(
            error,
            "StaleWriter：construction资格失效：Recover迁移状态块许可不挂事务父账"
        );
        assert_recover_revoked(&native, &f, probe.serial, "1");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_complete_optional_budget_releases_without_changing_reply() {
        let f = Fixture::new();
        let mut build = f.open("1").unwrap();
        let input = Fixture::ingest(1);
        let committed = build.handle(input.clone()).unwrap();
        let ready = envelope("ready-budget", json!({"op":"ready"}));
        let expected = build.handle(ready.clone()).unwrap();
        drop(build);
        // 已有完整候选后收紧真实 S7，覆盖入口 held > keeper_cap 分支。
        let mut warm = complete_driver(&f);
        assert_eq!(warm.handle(ready.clone()).unwrap(), expected);
        let previously_held = warm.conn.complete_retained_bytes();
        assert!(previously_held > 1024);
        let tables_before = prepared::tests::runtime_root_tables(&warm.conn, true);
        warm.audit_cache_source_bytes = Some(crate::cache::retained_bytes().saturating_add(1024));
        let (tightened, events, _) = state::traced(true, || warm.handle(ready.clone()));
        assert_eq!(tightened.unwrap(), expected);
        assert_eq!(warm.conn.complete_retained_bytes(), 0);
        assert!(!events.iter().any(|e| e["phase"] == "owner_complete_hit"));
        assert_eq!(
            tables_before,
            prepared::tests::runtime_root_tables(&warm.conn, true)
        );
        assert_eq!(warm.handle(input.clone()).unwrap(), committed);
        eprintln!(
            "{}",
            json!({"scenario":"owner-existing-complete-budget-tightened","previously_held":previously_held,"complete_retained":warm.conn.complete_retained_bytes()})
        );
        drop(warm);
        // 收紧现有同一S7，不扩大预算；不足只放弃完整候选，正常严格核仍返回成功。
        let mut bounded = Driver::open_for_serve_with_mode(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            1024,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        for request in [ready.clone(), ready, input] {
            let (reply, events, _) = state::traced(true, || bounded.handle(request.clone()));
            assert_eq!(
                reply.unwrap(),
                if request["payload"]["op"] == "ready" {
                    expected.clone()
                } else {
                    committed.clone()
                }
            );
            assert_eq!(bounded.conn.complete_retained_bytes(), 0);
            assert!(!events.iter().any(|e| e["phase"] == "owner_complete_hit"));
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "history_template_closure")
                    .count(),
                1
            );
        }
        eprintln!(
            "{}",
            json!({"scenario":"owner-optional-retention-budget","cap":1024,"complete_retained":bounded.conn.complete_retained_bytes()})
        );
    }

    /// #1488 P1-1：生产暖路径入口的old_pool逐项分区——O恰为逐项合同额之和，同父L0只计一次
    /// （parent_paid+uncovered=L0，parent_paid+external=O），项键(role,item,origin)不重；回复不变。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn p1_1_driver_warm_old_pool_partition_counts_l0_once() {
        let warm_fixture = Fixture::new();
        let ordinary_fixture = Fixture::new();
        let mut warm = complete_driver(&warm_fixture);
        let mut ordinary = ordinary_fixture.open("1").unwrap();
        let mut seen = Vec::new();
        for n in 1..=3 {
            let partition = {
                let keeper = warm.conn.source_keeper_for_test();
                warm.conn.old_pool_partition(keeper.as_ref()).unwrap()
            };
            let items = partition.items();
            let sum = items.iter().map(|i| i.contract).sum::<usize>();
            assert_eq!(partition.old_pool(), sum, "O=逐项合同额，不含L补差");
            assert_eq!(partition.parent_paid() + partition.uncovered(), partition.l0());
            assert_eq!(partition.parent_paid() + partition.external(), partition.old_pool());
            assert!(partition.carried() >= partition.l0(), "carried覆盖同父L0");
            let mut keys = items
                .iter()
                .map(|i| (i.role, i.item, i.origin))
                .collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), items.len(), "同一合同项不得重复入O");
            if n > 1 {
                assert!(partition.l0() > 0 && partition.old_pool() > 0, "暖入口有同父旧候选");
            }
            seen.push(serde_json::to_value(&partition).unwrap());
            assert_eq!(
                warm.handle(Fixture::ingest(n)).unwrap(),
                ordinary.handle(Fixture::ingest(n)).unwrap()
            );
        }
        eprintln!("{}", json!({"scenario":"p1-1-warm-partition","partitions":seen}));
    }

    /// #1488 P3-1：冷根审计失败（1024 不足）且 legacy cache 保留开启时，根析构前根上不得残留许可；
    /// 回复与基线一致。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn p3_1_cold_root_audit_failure_leaves_no_permit_on_destroyed_root() {
        let f = Fixture::new();
        let mut build = f.open("1").unwrap();
        build.handle(Fixture::ingest(1)).unwrap();
        let ready = envelope("ready-cold-root", json!({"op":"ready"}));
        let expected = build.handle(ready.clone()).unwrap();
        drop(build);
        let probe = |cap: usize, inject: bool| {
            let mut d = Driver::open_for_serve_with_mode(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                cap,
                s_allocation::Mode::CapturedNative,
            )
            .unwrap();
            // v3 driver 不配置 legacy memo；显式开启保留，使首跑真实经 NativeAdmission 保留证书。
            crate::cache::configure(268_435_456);
            assert!(crate::cache::retention_enabled());
            crate::v3::audit_owner::cold_root_probe::take();
            if inject {
                crate::v3::audit_owner::cold_root_probe::fail_next_cold_root_audit();
            }
            let reply = d.handle(ready.clone());
            let legacy = crate::cache::retained_bytes();
            crate::cache::configure(0);
            assert_eq!(reply.unwrap(), expected);
            let notes = crate::v3::audit_owner::cold_root_probe::take();
            assert_eq!(notes.len(), 1, "须恰经一次冷根审计失败分支：{notes:?}");
            assert_eq!(notes[0].1, 0, "根析构前仍有残留许可：{notes:?}");
            (notes[0], legacy)
        };
        // 自然失败：1024 不足，首跑在根上额度耗尽。
        let natural = probe(1024, false);
        // 真实首跑全部副作用发生后注入失败（最坏时点）：根上不得残留任何许可。
        let injected = probe(16 * 1024 * 1024, true);
        assert_eq!(injected.0, (0, 0), "{injected:?}");
        eprintln!(
            "{}",
            json!({"scenario":"p3-1-cold-root-failure","natural":natural,"injected":injected})
        );
    }

    /// #1488 P3-d（复审 0026）：冷根失败分支 clear_handles 后复核根残留；注入一张 legacy cache 之外、
    /// clear_handles 清不掉的根许可，须报错而非析构根后无票重跑；对照组（不注入）回复与基线一致。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn p3_d_cold_root_residue_after_clear_handles_is_rejected() {
        let f = Fixture::new();
        let mut build = f.open("1").unwrap();
        build.handle(Fixture::ingest(1)).unwrap();
        let ready = envelope("ready-cold-root-residue", json!({"op":"ready"}));
        let expected = build.handle(ready.clone()).unwrap();
        drop(build);
        for leak in [0usize, 4096] {
            let mut d = Driver::open_for_serve_with_mode(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                16 * 1024 * 1024,
                s_allocation::Mode::CapturedNative,
            )
            .unwrap();
            crate::cache::configure(268_435_456);
            crate::v3::audit_owner::cold_root_probe::take();
            crate::v3::audit_owner::cold_root_probe::fail_next_cold_root_audit();
            crate::v3::audit_owner::cold_root_probe::leak_next_cold_root_permit(leak);
            let reply = d.handle(ready.clone());
            crate::cache::configure(0);
            let notes = crate::v3::audit_owner::cold_root_probe::take();
            let leaked = crate::v3::audit_owner::cold_root_probe::take_leaked();
            let reruns = crate::v3::audit_owner::cold_root_probe::take_reruns();
            assert_eq!(notes.len(), 1, "须恰经一次冷根审计失败分支：{notes:?}");
            if leak == 0 {
                assert!(leaked.is_empty());
                assert_eq!(notes[0].1, 0, "{notes:?}");
                assert_eq!(reruns, 1, "残留归零才析构根后无票重跑一次");
                assert_eq!(reply.unwrap(), expected);
            } else {
                assert_eq!(leaked.len(), 1);
                assert_eq!(notes[0].1, leak, "clear_handles清不掉注入许可：{notes:?}");
                // #1488 P3-δ（主控裁定）：残留分支不得自动重试，错误须带运维处置说明。
                assert_eq!(reruns, 0, "残留非零分支不得无票重跑");
                let error = reply.expect_err("根残留未归零必须报错");
                assert!(
                    error.contains("冷根审计失败清除保留后根上仍残留4096B许可"),
                    "{error}"
                );
                assert!(error.contains("不自动重试") && error.contains("重启writer进程"), "{error}");
                // #1488 0079（0078 optional_lock）：残留分支只拒触发它的请求，不退役 writer；
                // 同一 driver 再发 ready 必须成功且与基线一致。
                crate::cache::configure(268_435_456);
                let again = d.handle(ready.clone());
                crate::cache::configure(0);
                crate::v3::audit_owner::cold_root_probe::take();
                crate::v3::audit_owner::cold_root_probe::take_reruns();
                assert_eq!(again.expect("残留分支报错后同一 writer 须继续服务"), expected);
            }
        }
    }

    /// #1488 0079（P24-adj，0078 探针 H 形态）：同一 writer 内冷根自然失败（1M）后无票重跑提交，
    /// 之后 allowance 变大（改 16M）使候选重新出生；后续 ingest 必须走 Carry 提交且与 ordinary 一致。
    /// 负控：去掉 `renew_after_cold_root_rerun` 的续租 → ingest-2 报“Carry租约已撤销”。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn p24adj_cold_root_rerun_renews_lease_and_larger_cap_ingest_commits() {
        use crate::v3::audit_owner::cold_root_probe as probe;
        // 返回（首个 ingest 的冷根 notes, 重跑次数, 首个 ingest 后租约, 各 ingest 的
        // (命中完整候选, 提交后持资格, 降级放弃, 租约撤销), 与 ordinary 逐步比对结果, 末态根表是否一致）。
        let run = |first_cap: usize| {
            let f = Fixture::new();
            let o = Fixture::new();
            let mut ordinary = o.open("1").unwrap();
            let mut d = Driver::open_for_serve_with_mode(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                first_cap,
                s_allocation::Mode::CapturedNative,
            )
            .unwrap();
            crate::cache::configure(268_435_456);
            probe::take();
            probe::take_reruns();
            drop(probe::take_leaked());
            let mut mismatched = Vec::new();
            let mut check = |id: &str, actual: Result<Value, String>, oracle: Value| {
                if actual.as_ref() != Ok(&oracle) {
                    mismatched.push(json!({"id":id,"actual":format!("{actual:?}")}));
                }
            };
            let first = d.handle(Fixture::ingest(1));
            let notes = probe::take();
            let reruns = probe::take_reruns();
            let renewed = d.conn.recover_probe_for_test();
            check("ingest-1", first, ordinary.handle(Fixture::ingest(1)).unwrap());
            d.audit_cache_source_bytes = Some(16 << 20);
            let steps = [
                ("ready-a", envelope("p24adj-ready-a", json!({"op":"ready"}))),
                ("ingest-2", Fixture::ingest(2)),
                ("ready-b", envelope("p24adj-ready-b", json!({"op":"ready"}))),
                ("ingest-3", Fixture::ingest(3)),
                ("ready-c", envelope("p24adj-ready-c", json!({"op":"ready"}))),
            ];
            let mut carried = Vec::new();
            for (id, request) in steps {
                let (actual, events, _) = state::traced(true, || d.handle(request.clone()));
                let after = d.conn.recover_probe_for_test();
                if id.starts_with("ingest") {
                    let phase = |p: &str| events.iter().any(|e| e["phase"] == p);
                    carried.push((
                        id,
                        phase("owner_complete_hit"),
                        after.qualification.is_some(),
                        phase("publication_candidate_abandoned"),
                        after.stale,
                    ));
                }
                check(id, actual, ordinary.handle(request).unwrap());
            }
            crate::cache::configure(0);
            let later_notes = probe::take();
            let tables = prepared::tests::runtime_root_tables(&d.conn, true)
                == prepared::tests::runtime_root_tables(&ordinary.conn, true);
            (notes, reruns, renewed, later_notes, carried, mismatched, tables)
        };
        let cold = run(1 << 20);
        let control = run(16 << 20);
        eprintln!(
            "{}",
            json!({"scenario":"p24adj-cold-root-then-larger-cap",
                "cold":{"notes":cold.0,"reruns":cold.1,"carried":format!("{:?}", cold.4)},
                "control":{"notes":control.0,"carried":format!("{:?}", control.4)}})
        );
        assert_eq!(cold.0, [(0, 0)], "1M 下首个 ingest 须恰经一次冷根自然失败且残留归零");
        assert_eq!(cold.1, 1, "残留归零后无票重跑一次");
        assert!(
            !cold.2.stale && !cold.2.retired && cold.2.qualification.is_none(),
            "冷根失败无票重跑提交后须续租：{:?}",
            cold.2
        );
        assert!(cold.3.is_empty() && control.0.is_empty(), "16M 下冷根不应失败");
        assert!(cold.5.is_empty(), "逐步回复须与 ordinary 一致：{:?}", cold.5);
        assert!(control.5.is_empty(), "对照组逐步回复须与 ordinary 一致：{:?}", control.5);
        assert!(cold.6 && control.6, "末态根表须与 ordinary 一致");
        assert_eq!(
            cold.4[0],
            ("ingest-2", true, true, false, false),
            "候选重新出生后 ingest-2 须命中完整候选、经 Carry 提交资格且租约有效"
        );
        assert_eq!(cold.4[1], control.4[1], "ingest-3 须与全程 16M 对照组同路");
        assert!(cold.4.iter().all(|c| !c.4), "租约不得撤销：{:?}", cold.4);
    }

    /// #1488 0079（P24-adj 负控锁）：冷根失败续租不清除真实撤销。
    /// (a) 续租并经 Carry 建立资格后发生真实外写：下一 ingest 仍被拒、租约撤销、资格离开 owner；
    /// (b) 前序请求回滚（IdentityConflict 准入事务析构）已撤销租约：其后的冷根失败无票重跑提交不续租，
    ///     allowance 变大、候选重新出生后 ingest 照旧报“Carry租约已撤销”。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn p24adj_cold_root_renewal_keeps_real_revocations() {
        use crate::v3::audit_owner::cold_root_probe as probe;
        let open = |f: &Fixture| {
            Driver::open_for_serve_with_mode(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                1 << 20,
                s_allocation::Mode::CapturedNative,
            )
            .unwrap()
        };
        let ready = |id: &str| envelope(id, json!({"op":"ready"}));
        crate::cache::configure(268_435_456);
        probe::take();
        probe::take_reruns();
        // (a) 外写。
        let f = Fixture::new();
        let mut d = open(&f);
        let a1 = d.handle(Fixture::ingest(1)).map(|_| ());
        let a_notes = probe::take();
        d.audit_cache_source_bytes = Some(16 << 20);
        let a2 = d.handle(ready("p24adj-neg-ready-a")).map(|_| ());
        let a3 = d.handle(Fixture::ingest(2)).map(|_| ());
        let carried = d.conn.recover_probe_for_test();
        let a4 = d.handle(ready("p24adj-neg-ready-b")).map(|_| ());
        recover_lock_external_write(&f);
        let a5 = d.handle(Fixture::ingest(3));
        let revoked = d.conn.recover_probe_for_test();
        // (b) 前序回滚。
        let g = Fixture::new();
        let mut e = open(&g);
        let mut foreign = ready("p24adj-neg-foreign");
        foreign["session_generation"] = json!("2");
        let b1 = e.handle(foreign);
        let pre_stale = e.conn.recover_probe_for_test();
        probe::take();
        let b2 = e.handle(Fixture::ingest(1)).map(|_| ());
        let b_notes = probe::take();
        let not_renewed = e.conn.recover_probe_for_test();
        e.audit_cache_source_bytes = Some(16 << 20);
        let b3 = e.handle(ready("p24adj-neg-ready-c")).map(|_| ());
        let b4 = e.handle(Fixture::ingest(2));
        crate::cache::configure(0);
        probe::take();
        probe::take_reruns();
        assert_eq!((a1, a2, a3, a4), (Ok(()), Ok(()), Ok(()), Ok(())));
        assert_eq!(a_notes, [(0, 0)], "(a) 须经冷根自然失败");
        assert!(
            !carried.stale && carried.qualification.is_some(),
            "(a) 续租后 Carry 须建立资格：{carried:?}"
        );
        assert!(a5.is_err(), "(a) 外写后 ingest 必须被拒：{a5:?}");
        assert!(
            revoked.stale && revoked.serial > carried.serial && revoked.qualification.is_none(),
            "(a) 外写后租约须撤销、资格离开 owner：{revoked:?}"
        );
        assert!(
            b1.as_ref().is_err_and(|e| e.contains("IdentityConflict")),
            "{b1:?}"
        );
        assert_eq!(b2, Ok(()));
        assert!(pre_stale.stale, "(b) 回滚须先撤销租约：{pre_stale:?}");
        assert_eq!(b_notes, [(0, 0)], "(b) ingest 须经冷根自然失败并无票重跑");
        assert!(not_renewed.stale, "(b) 前序回滚撤销不得被续租清除：{not_renewed:?}");
        assert_eq!(b3, Ok(()));
        assert!(
            b4.as_ref().is_err_and(|e| e.contains("Carry租约已撤销")),
            "(b) 前序真实撤销后照旧 fail-closed：{b4:?}"
        );
    }


    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    // NOT_RUN：本轮只静态检查。
    fn cross_request_ready_first_write_and_two_requests_preserve_parent() {
        let f = Fixture::new();
        let o = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        let ready = envelope("handoff-ready", json!({"op":"ready"}));
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready).unwrap()
        );
        let original = native.conn.complete_budget_for_test();
        let mut peak = original.peak();
        for n in 1..=2 {
            assert_eq!(
                native.handle(Fixture::ingest(n)).unwrap(),
                ordinary.handle(Fixture::ingest(n)).unwrap()
            );
            let current = native.conn.complete_budget_for_test();
            assert!(current.same_parent(&original));
            assert!(current.live() > 0);
            assert!(current.peak() >= peak);
            peak = current.peak();
            native.conn.assert_complete_matches_cold();
        }
        assert!(original.reserve(1).is_err(), "旧请求句柄不能获得新分配资格");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate() {
        let f = Fixture::new();
        let o = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        assert_eq!(
            native.handle(Fixture::ingest(1)).unwrap(),
            ordinary.handle(Fixture::ingest(1)).unwrap()
        );
        let before = native.conn.complete_report();
        let held = native.conn.complete_retained_bytes();
        assert!(held > 0);
        assert_eq!(before["charge"]["slot_result_capacity"], 0);
        let workspace = 16 * 1024 * 1024;
        let old_gate_lower_bound = held.checked_mul(32).unwrap().checked_add(65536).unwrap();
        assert!(
            workspace < old_gate_lower_bound,
            "旧门即使不计effects也会撤候选"
        );
        let cap = crate::cache::retained_bytes() + held + workspace;
        native.audit_cache_source_bytes = Some(cap);
        let (actual, events, _) = state::traced(true, || native.handle(Fixture::ingest(2)));
        assert_eq!(
            actual.unwrap(),
            ordinary.handle(Fixture::ingest(2)).unwrap()
        );
        assert!(native.conn.complete_retained_bytes() > 0);
        let retained = native.conn.complete_report();
        assert!(retained["charge"]["handoff_peak"].as_u64().unwrap() < cap as u64);
        for phase in [
            "history_decode_manifest",
            "generated_sources",
            "catalog_cut",
        ] {
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == phase)
                    .map(|e| e["generation"].as_i64().unwrap())
                    .collect::<Vec<_>>(),
                [2]
            );
        }
        assert!(events.iter().any(|e| e["phase"] == "owner_complete_hit"));
        assert!(!events
            .iter()
            .any(|e| e["phase"] == "owner_retained_full_scan"));
        native.conn.assert_complete_matches_cold();
        assert_eq!(
            prepared::tests::runtime_root_tables(&native.conn, true),
            prepared::tests::runtime_root_tables(&ordinary.conn, true)
        );
        eprintln!(
            "{}",
            json!({"scenario":"r9-normal-below-old-multiplier","workspace":workspace,"cap":cap,"old_gate_lower_bound":old_gate_lower_bound,"retention":retained})
        );
    }

    /// #1488 本票第四处裁改断言（主控裁定 (i)：
    /// https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6281614540）。
    /// 原锁 `audit_owner_write_budget_releases_candidate_after_successful_normal_append` 断言
    /// 第二次正常写成功后 `complete_retained_bytes() == 0`，该断言在现设计不可达，旧绿为假绿
    /// （冷路径降级或写失败归零），故与 0039 正控锁合并为本锁，不再保留 ==0。
    /// 不可达依据（workers/wbfix-0039/REPORT.md §2 读数）：同一 S7 额度余量 M 同时供写期工作额度
    /// （≈M）与保留帽（held+M）；规则释放需 M < charge + raw − held = 3,281,186 + 320,877 − 2,873,735
    /// = 728,328B，而实测写成功下限在 6–7MiB（门槛机理未定位，不是 handoff_peak，见复审 0043 §9），
    /// 两区间不相交；16/9/8/7MiB 写成功且保留 3,602,191B，6MiB 及以下写本身 StaleWriter；
    /// 且即使强制释放候选，construction raw 320,877B 仍计入 retained，==0 仍不成立。
    /// 现语义：夹具不变（legacy + held + 8MiB），effects 路径写成功，完整候选按生产保留裁决点读数
    /// `charge.total ≤ complete_retention_cap − raw` 保留，本请求域峰 handoff_peak − entry ≤ 候选
    /// 工作额度 limit（生产实比之值，0044 按复审 0043 P2-1/P3-3 订正，不在测试里复述公式）。
    /// 开放风险：本锁只证明 8MiB 夹具下的上述关系，并未证明“8MiB 足够”
    /// （RUNTIME-INDEPENDENT-REVIEW【05】），随整票验收的资源读数一并报告。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_write_budget_retains_candidate_within_retention_cap_and_parent_limit() {
        use super::super::audit_owner::allocation_probe::Window;
        let f = Fixture::new();
        let o = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        assert_eq!(
            native.handle(Fixture::ingest(1)).unwrap(),
            ordinary.handle(Fixture::ingest(1)).unwrap()
        );
        let held = native.conn.complete_retained_bytes();
        assert!(held > 0);
        let legacy = crate::cache::retained_bytes();
        let total = legacy + held + 8 * 1024 * 1024;
        // 仅收紧本测试 Driver 的同一 S7 额度，不修改任何默认额度。
        native.audit_cache_source_bytes = Some(total);
        let memory = Window::start();
        let (actual, events, work) = state::traced(true, || native.handle(Fixture::ingest(2)));
        let memory = memory.finish();
        assert_eq!(
            actual.unwrap(),
            ordinary.handle(Fixture::ingest(2)).unwrap()
        );
        // 同一路径：写期走 effects 增量与后缀完整核，不是冷路径降级。
        assert!(events.iter().any(|e| e["phase"] == "owner_complete_hit"));
        assert!(events.iter().any(|e| e["phase"] == "owner_effects"));
        assert!(events.iter().any(|e| e["phase"] == "owner_suffix_complete"));
        let retained = native.conn.complete_retained_bytes();
        assert!(retained > 0);
        // retained 含 construction raw；须核完整候选本身仍在。
        let report = native.conn.complete_report();
        assert!(!report.is_null(), "完整候选未保留，retained={retained} 仅为 raw");
        let charge = report["charge"]["total"].as_u64().unwrap() as usize;
        let entry = report["charge"]["entry_reserved_bytes"].as_u64().unwrap() as usize;
        let peak = report["charge"]["handoff_peak"].as_u64().unwrap() as usize;
        // 0044 P2-1：handoff_peak = entry + 本请求域峰（audit_owner.rs `recording_peak`/effects 收尾），
        // 生产只对后者设门：effects 路径 `peak > self.limit` 即放弃增量，Ledger 预留从同一 allowance
        // checked_sub。故读候选实际 limit，与生产同一口径比较本请求域峰；entry 不再计两次。
        let parent_limit = native.conn.complete_limit_for_test();
        let request_peak = peak.checked_sub(entry).unwrap();
        assert!(request_peak <= parent_limit, "{peak} − {entry} > {parent_limit}");
        // 0044 P3-3：读收尾保留裁决点的生产读数（complete_retention_cap 与 construction 实际 retained）。
        let [decision] = events
            .iter()
            .filter(|e| e["phase"] == "owner_retention")
            .collect::<Vec<_>>()[..]
        else {
            panic!("保留裁决点须恰执行一次：{events:?}")
        };
        let cap = decision["cap"].as_u64().unwrap() as usize;
        let raw = decision["raw"].as_u64().unwrap() as usize;
        assert_eq!(
            retained,
            charge + super::super::audit_owner::native::Budget::metadata_bytes() + raw
        );
        assert!(charge <= cap - raw, "{charge} > {cap} - {raw}");
        native.conn.assert_complete_matches_cold();
        assert_eq!(
            prepared::tests::runtime_root_tables(&native.conn, true),
            prepared::tests::runtime_root_tables(&ordinary.conn, true)
        );
        eprintln!(
            "{}",
            json!({"scenario":"r4-write-budget-retain","old_charge":held,"charge":charge,"cap":cap,"raw":raw,"handoff_peak":peak,"parent_limit":parent_limit,"allocation":memory,"events":events,"work":work,"retained":retained})
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_complete_normal_second_connection_and_new_life_are_cold() {
        let f = Fixture::new();
        let o = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        assert_eq!(
            native.handle(Fixture::ingest(1)).unwrap(),
            ordinary.handle(Fixture::ingest(1)).unwrap()
        );
        let ready = envelope("ready-external", json!({"op":"ready"}));
        native.handle(ready.clone()).unwrap();
        let old_life = native.conn.complete_report()["life"].clone();
        // 第二连接使用实际 owner 的正常接纳/推进/发布；不改坏库、不绕过 idle 前件。
        let external = super::super::audit_owner::AuditConnection::open(&f.db).unwrap();
        let next = Fixture::ingest(2);
        let mut context = request_audit::RequestAuditContext::default();
        let mode = s_allocation::Mode::Ordinary;
        let (admission, _) = external
            .admit(
                next.clone(),
                "1",
                &f.clock,
                &native.profile_id,
                native.volume_unit.as_deref(),
                &mode,
                &mut context,
            )
            .unwrap();
        assert!(matches!(
            admission,
            super::super::audit_owner::Admission::Advance { .. }
        ));
        let begun = external
            .begin_advance(&next, "1", &f.clock, &mode, &mut context)
            .unwrap();
        let mut runtime = prepared::Runtime::with_mode(mode);
        let (receipt, _) = external
            .publish(&mut runtime, &begun.token, &next, &mut context)
            .unwrap();
        assert_eq!(receipt.message_receipt, ordinary.handle(next).unwrap());
        drop(begun);
        drop(external);
        let before = prepared::tests::runtime_root_tables(&native.conn, true);
        let writes_before = native.conn.total_changes();
        let (rejected, events, _) = state::traced(true, || native.handle(ready.clone()));
        assert_eq!(
            rejected.unwrap_err(),
            "StaleWriter：construction资格失效：已有增量游标的材料域改变；禁止冷重签"
        );
        assert_eq!(native.conn.total_changes(), writes_before);
        assert_eq!(
            prepared::tests::runtime_root_tables(&native.conn, true),
            before
        );
        assert_eq!(
            before,
            prepared::tests::runtime_root_tables(&ordinary.conn, true)
        );
        assert_eq!(native.conn.complete_retained_bytes(), 0);
        assert_eq!(native.conn.complete_report(), Value::Null);
        assert!(native.conn.source_keeper_for_test().is_none());
        assert!(!events.iter().any(|e| e["phase"] == "owner_complete_hit"));
        assert!(!events
            .iter()
            .any(|e| e["phase"] == "history_template_closure"));
        // 旧 owner 的来源资格已撤销；连同旧 Runtime 一起丢弃，不能在旧 life 冷重签。
        drop(native);
        let mut reopened = complete_driver(&f);
        assert_eq!(reopened.conn.complete_retained_bytes(), 0);
        assert!(reopened.runtime.visit_runtime_roots().unwrap().is_empty());
        let expected = ordinary.handle(ready.clone()).unwrap();
        let (actual, events, _) = state::traced(true, || reopened.handle(ready));
        assert_eq!(actual.unwrap(), expected);
        assert_eq!(
            events
                .iter()
                .filter(|e| e["phase"] == "history_template_closure")
                .count(),
            2
        );
        assert_ne!(reopened.conn.complete_report()["life"], old_life);
        reopened.conn.assert_complete_matches_cold();
        assert_eq!(
            prepared::tests::runtime_root_tables(&reopened.conn, true),
            prepared::tests::runtime_root_tables(&ordinary.conn, true)
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_bootstrap_idle_identity_hits_then_drops_and_ready_is_new_cold_default_paths_unchanged(
    ) {
        let f = Fixture::new();
        let mut first = f.open("1").unwrap();
        first.handle(Fixture::ingest(1)).unwrap();
        let expected = first.identity().unwrap();
        drop(first);
        let (result, events, _) = state::traced(false, || {
            Driver::open_for_serve_with_identity(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                1,
                s_allocation::Mode::CapturedNative,
            )
        });
        let (mut driver, identity) = result.unwrap();
        assert_eq!(identity, expected);
        let phase_count =
            |events: &[Value], phase: &str| events.iter().filter(|e| e["phase"] == phase).count();
        assert_eq!(phase_count(&events, "request_history_cold"), 1);
        assert_eq!(phase_count(&events, "request_history_hit"), 1);
        assert_eq!(phase_count(&events, "request_history_context_released"), 1);
        assert_eq!(
            events.last().unwrap()["phase"],
            "request_history_context_released",
            "返回Driver前已释放bootstrap原件"
        );
        let (ready, events, _) = state::traced(false, || {
            driver.handle(envelope("bootstrap-ready", json!({"op":"ready"})))
        });
        assert_eq!(ready.unwrap()["kind"], "Ready");
        assert_eq!(phase_count(&events, "request_history_cold"), 1);
        assert_eq!(phase_count(&events, "request_history_hit"), 0);
        assert_eq!(phase_count(&events, "request_history_context_released"), 1);
        assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
        drop(driver);
        let (result, events, _) = state::traced(false, || {
            let driver = Driver::open_for_serve_with_mode(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                1,
                s_allocation::Mode::Ordinary,
            )?;
            let identity = driver.identity()?;
            Ok::<_, String>((driver, identity))
        });
        let (driver, identity) = result.unwrap();
        assert_eq!(identity, expected);
        assert!(!events
            .iter()
            .any(|e| e["phase"].as_str().unwrap().starts_with("request_history_")));
        drop(driver);
        let owned = s_allocation::Mode::Owned(q_allocation::Profile {
            budget: newchan_rust::s_content::capture_allocation::OwnedBudget::new(32),
            overlay_slots: 0,
            overlay_bytes: 0,
        });
        let error = Driver::open_for_serve_with_identity(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            1,
            owned.clone(),
        )
        .err()
        .unwrap();
        assert!(error.contains("只接受显式CapturedNative"));
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let (result, events, _) = state::traced(false, || {
                Driver::open_for_serve_with_mode(&f.db, &f.profile, "1", f.clock.clone(), 1, owned)
            });
            assert!(result.err().unwrap().contains("此构建不支持实际分配原型"));
            assert!(!events
                .iter()
                .any(|e| e["phase"].as_str().unwrap().starts_with("request_history_")));
        }
        println!("Native idle bootstrap cold=1 hit=1 released_before_return=1; Ready cold=1 hit=0 released=1; retained_source=0");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_bootstrap_pending_drops_startup_slot_before_resume_then_identity_cold_matches_current_frontier(
    ) {
        let f = Fixture::new();
        let normal = Fixture::new();
        let mut baseline = normal.open("1").unwrap();
        baseline.handle(Fixture::ingest(1)).unwrap();
        let expected = baseline.handle(Fixture::ingest(2)).unwrap();
        let mut first = f.open("1").unwrap();
        first.handle(Fixture::ingest(1)).unwrap();
        first.stop_after = Some("accept");
        assert!(first
            .handle(Fixture::ingest(2))
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(first);
        let mut context = request_audit::RequestAuditContext::default();
        let mut driver = Driver::open_with_request_and_mode_context(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            None,
            s_allocation::Mode::CapturedNative,
            Some(&mut context),
        )
        .unwrap();
        assert_eq!((context.cold, context.hits), (1, 0));
        assert!(!context.has_slot(), "startup原件不能跨着恢复请求保留");
        assert_eq!(count(&driver.conn, "s_content_publications"), 2);
        assert_eq!(count(&driver.conn, "s_clock_events"), 6);
        assert_eq!(meta_i64(&driver.conn, "generation").unwrap(), 2);
        assert_eq!(meta_i64(&driver.conn, "last_advance_frontier").unwrap(), 1);
        let batch: String = driver
            .conn
            .query_row(
                "SELECT batch_id FROM s_content_publications WHERE generation=2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            meta_get_opt(&driver.conn, "index_frontier")
                .unwrap()
                .as_deref(),
            Some(batch.as_str())
        );
        assert_eq!(
            driver.identity_in_bootstrap(&mut context).unwrap(),
            baseline.identity().unwrap()
        );
        assert_eq!((context.cold, context.hits), (2, 0));
        assert!(context.has_slot());
        drop(context);
        assert_eq!(driver.handle(Fixture::ingest(2)).unwrap(), expected);
        assert_eq!(
            driver
                .handle(envelope("after-resume-ready", json!({"op":"ready"})))
                .unwrap(),
            baseline
                .handle(envelope("after-resume-ready", json!({"op":"ready"})))
                .unwrap()
        );
        println!("Native pending bootstrap cold=2 hit=0 startup_slot_dropped_before_advance=1; generation=2 raw_frontier=1 index_frontier=publication_batch clocks=6; receipt/Ready=ordinary");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_bootstrap_two_transactions_reject_current_damage_or_sql_error_without_old_proof() {
        for sql in [
            "UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "CREATE TABLE unknown_bootstrap_domain(x)",
            "UPDATE meta SET value='99' WHERE key='writer_epoch'",
            "UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "UPDATE s_input_messages SET canonical_envelope='{}' WHERE first_published_generation=1",
            "UPDATE s_clock_events SET semantic_ns='999' WHERE phase='commit'",
            "INSERT INTO raw_ohlc SELECT 'bootstrap-orphan',revision,schema_revision,open,high,low,close FROM raw_ohlc LIMIT 1",
            "ALTER TABLE s_content_nodes RENAME TO missing_bootstrap_nodes",
        ] {
            let f=Fixture::new();
            let mut initial=f.open("1").unwrap();
            initial.handle(Fixture::ingest(1)).unwrap();drop(initial);
            let mut context=request_audit::RequestAuditContext::default();
            let driver=Driver::open_with_request_and_mode_context(&f.db,&f.profile,"1",f.clock.clone(),None,
                s_allocation::Mode::CapturedNative,Some(&mut context)).unwrap();
            assert!(context.has_slot());
            driver.conn.execute_batch(sql).unwrap();
            let before=(count(&driver.conn,"s_input_messages"),count(&driver.conn,"s_clock_events"),count(&driver.conn,"s_content_publications"));
            assert!(driver.identity_in_bootstrap(&mut context).is_err(),"{sql}");
            assert!(!context.has_slot(),"{sql}");assert_eq!(context.hits,0);
            assert_eq!((count(&driver.conn,"s_input_messages"),count(&driver.conn,"s_clock_events"),count(&driver.conn,"s_content_publications")),before);
        }
        // 实际SQL step中断也不能cold重试吞错。
        let f = Fixture::new();
        let mut context = request_audit::RequestAuditContext::default();
        let driver = Driver::open_with_request_and_mode_context(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            None,
            s_allocation::Mode::CapturedNative,
            Some(&mut context),
        )
        .unwrap();
        let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        driver
            .conn
            .progress_handler(
                1,
                Some(move || once.swap(false, std::sync::atomic::Ordering::SeqCst)),
            )
            .unwrap();
        assert!(driver.identity_in_bootstrap(&mut context).is_err());
        driver
            .conn
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        assert!(!context.has_slot());
        assert_eq!((context.cold, context.hits), (1, 0));
        driver.identity_in_bootstrap(&mut context).unwrap();
        assert_eq!((context.cold, context.hits), (2, 0));
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_bootstrap_epoch_profile_idle_and_illegal_request_reject_before_pending_resume_and_release(
    ) {
        for fault in ["epoch", "profile", "idle", "request"] {
            let f = Fixture::new();
            let mut first = f.open("1").unwrap();
            first.stop_after = Some("accept");
            assert!(first
                .handle(Fixture::ingest(1))
                .unwrap_err()
                .contains("stop_after_accept"));
            drop(first);
            let mut profile = f.profile.clone();
            if fault == "profile" {
                let mut value: Value =
                    serde_json::from_slice(&std::fs::read(&profile).unwrap()).unwrap();
                value["volume_unit"] = json!("17");
                profile = f.path.join("foreign-profile.json");
                std::fs::write(&profile, canonical_json(&value)).unwrap();
            }
            if fault == "idle" {
                Connection::open(&f.db)
                    .unwrap()
                    .execute(
                        "UPDATE meta SET value='not-idle' WHERE key='advance_state'",
                        [],
                    )
                    .unwrap();
            }
            let c = Connection::open(&f.db).unwrap();
            let before = (
                count(&c, "s_input_messages"),
                count(&c, "s_clock_events"),
                count(&c, "s_content_publications"),
            );
            drop(c);
            let request = envelope(
                "invalid-bootstrap-request",
                json!({"op":"ready","extra":"bad"}),
            );
            let (result, events, _) = state::traced(false, || {
                if fault == "request" {
                    let mut context = request_audit::RequestAuditContext::default();
                    Driver::open_with_request_and_mode_context(
                        &f.db,
                        &profile,
                        "1",
                        f.clock.clone(),
                        Some(&request),
                        s_allocation::Mode::CapturedNative,
                        Some(&mut context),
                    )
                    .map(|_| ())
                } else {
                    Driver::open_for_serve_with_identity(
                        &f.db,
                        &profile,
                        if fault == "epoch" { "2" } else { "1" },
                        f.clock.clone(),
                        1,
                        s_allocation::Mode::CapturedNative,
                    )
                    .map(|_| ())
                }
            });
            assert!(result.is_err(), "{fault}");
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_context_released")
                    .count(),
                1,
                "{fault}"
            );
            let c = Connection::open(&f.db).unwrap();
            assert_eq!(
                (
                    count(&c, "s_input_messages"),
                    count(&c, "s_clock_events"),
                    count(&c, "s_content_publications")
                ),
                before,
                "{fault}"
            );
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_startup_full_audit_and_pending_resume_before_write_guards() {
        let fixture = Fixture::new();
        let mut normal = fixture.open("1").unwrap();
        normal.handle(Fixture::ingest(1)).unwrap();
        normal.stop_after = Some("accept");
        assert!(normal
            .handle(Fixture::ingest(2))
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(normal);
        let mut db = Connection::open(&fixture.db).unwrap();
        let tx = db.transaction().unwrap();
        let ordinary = s_allocation::audit_startup(&tx, &s_allocation::Mode::Ordinary).unwrap();
        let native = s_allocation::audit_startup(&tx, &s_allocation::Mode::CapturedNative).unwrap();
        let fingerprint = |s: &state::AuditedState| {
            format!(
                "{:?}|{:?}|{:?}|{:?}",
                s.raw,
                s.cuts
                    .iter()
                    .map(|c| format!(
                        "{:?}|{:?}|{:?}|{:?}|{:?}",
                        c.metadata, c.roots, c.batch_id, c.snapshot, c.delta
                    ))
                    .collect::<Vec<_>>(),
                s.objects,
                s.auxiliary
            )
        };
        assert_eq!(fingerprint(&ordinary), fingerprint(&native));
        drop(ordinary);
        drop(native);
        drop(tx);
        drop(db);
        let mut resumed = native_driver(&fixture);
        assert_eq!(count(&resumed.conn, "s_content_publications"), 2);
        assert_eq!(count(&resumed.conn, "s_clock_events"), 6);
        assert_eq!(
            resumed.handle(Fixture::ingest(2)).unwrap()["kind"],
            "Committed"
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_startup_rejects_each_current_damage_before_pending_resume() {
        let faults=["UPDATE meta SET value='foreign' WHERE key='profile_hash'","UPDATE s_clock_events SET semantic_ns='999' WHERE phase='commit'","UPDATE raw_events SET raw_text='corrupt' WHERE seq=0","UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT min(rowid) FROM s_content_objects)","UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)","UPDATE meta SET value='unknown' WHERE key='protocol_revision'","CREATE TABLE unknown_startup_domain(x)"];
        for sql in faults {
            let fixture = Fixture::new();
            let mut old = fixture.open("1").unwrap();
            old.handle(Fixture::ingest(1)).unwrap();
            old.stop_after = Some("accept");
            assert!(old.handle(Fixture::ingest(2)).is_err());
            drop(old);
            let c = Connection::open(&fixture.db).unwrap();
            c.execute_batch(sql).unwrap();
            let before = (
                count(&c, "s_input_messages"),
                c.query_row(
                    "SELECT count(*) FROM s_input_messages WHERE status='AcceptedPending'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap(),
                count(&c, "s_clock_events"),
                count(&c, "s_content_publications"),
                meta_get_opt(&c, "advance_state").unwrap(),
                meta_get_opt(&c, "generation").unwrap(),
            );
            drop(c);
            let result = Driver::open_with_request_and_mode(
                &fixture.db,
                &fixture.profile,
                "1",
                fixture.clock.clone(),
                None,
                s_allocation::Mode::CapturedNative,
            );
            assert!(result.is_err(), "{sql}");
            let c = Connection::open(&fixture.db).unwrap();
            let after = (
                count(&c, "s_input_messages"),
                c.query_row(
                    "SELECT count(*) FROM s_input_messages WHERE status='AcceptedPending'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap(),
                count(&c, "s_clock_events"),
                count(&c, "s_content_publications"),
                meta_get_opt(&c, "advance_state").unwrap(),
                meta_get_opt(&c, "generation").unwrap(),
            );
            assert_eq!(before, after, "{sql}");
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_startup_SQL_errors_propagate_without_ordinary_fallback() {
        let fixture = Fixture::new();
        let mut c = Connection::open(&fixture.db).unwrap();
        let tx = c.transaction().unwrap();
        let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        tx.progress_handler(
            1,
            Some(move || once.swap(false, std::sync::atomic::Ordering::SeqCst)),
        )
        .unwrap();
        assert!(s_allocation::audit_startup(&tx, &s_allocation::Mode::CapturedNative).is_err());
        tx.progress_handler(0, None::<fn() -> bool>).unwrap();
        drop(tx);
        c.execute_batch("ALTER TABLE s_content_nodes RENAME TO hidden_startup_nodes")
            .unwrap();
        let tx = c.transaction().unwrap();
        let error = s_allocation::audit_startup(&tx, &s_allocation::Mode::CapturedNative)
            .err()
            .unwrap();
        assert!(error.contains("no such table"), "{error}");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    #[ignore]
    fn staged_next106_cold_driver_observation() {
        let db = PathBuf::from(std::env::var("NEXT106_DB").unwrap());
        let profile = PathBuf::from(std::env::var("NEXT106_PROFILE").unwrap());
        let request_path = PathBuf::from(std::env::var("NEXT106_REQUEST").unwrap());
        let out = PathBuf::from(std::env::var("NEXT106_OUTPUT").unwrap());
        let arm = std::env::var("NEXT106_ARM").unwrap();
        assert!(["eager", "staged"].contains(&arm.as_str()));
        let request_bytes = std::fs::read(request_path).unwrap();
        let envelope = strict_json(&request_bytes).unwrap();
        assert_eq!(envelope["message_id"], "message-106");
        let resources: Value =
            strict_json(&std::fs::read(std::env::var("NEXT106_RESOURCES").unwrap()).unwrap())
                .unwrap();
        let cap = newchan_rust::session_protocol::positive(
            resources["audit_cache_source_bytes"].as_str().unwrap(),
            "audit_cache_source_bytes",
        )
        .unwrap() as usize;
        let before = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        before.pragma_update(None, "query_only", true).unwrap();
        assert_eq!(meta_i64(&before, "generation").unwrap(), 105);
        assert_eq!(
            meta_get_opt(&before, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert_eq!(
            before
                .query_row(
                    "SELECT count(*) FROM s_input_messages WHERE status='AcceptedPending'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            before
                .query_row(
                    "SELECT next_attempt_ordinal FROM s_protocol_meta WHERE singleton=1",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            106
        );
        assert_eq!(
            before
                .query_row("SELECT count(*) FROM s_input_messages", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            105
        );
        assert_eq!(
            before
                .query_row(
                    "SELECT count(*) FROM s_input_messages WHERE status='Committed'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            105
        );
        assert_eq!(
            before
                .query_row(
                    "SELECT count(*) FROM s_input_messages WHERE message_id='message-106'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        let clock = v2::stored_clock(&before).unwrap();
        clock.phase("input-106", "accept", -1).unwrap();
        clock.phase("input-106", "begin", 0).unwrap();
        clock.phase("input-106", "commit", 0).unwrap();
        let epoch = meta_get_opt(&before, "writer_epoch").unwrap().unwrap();
        drop(before);
        std::fs::write(&out,serde_json::to_vec_pretty(&json!({"arm":arm,"status":"OPEN_RUNNING","cold_restart_runtime":true,"mandatory_open_audit":"explicit CapturedNative startup: fresh full capture + same audit_core; other modes unchanged"})).unwrap()).unwrap();
        let started = std::time::Instant::now();
        let opened = Driver::open_with_request_and_mode(
            &db,
            &profile,
            &epoch,
            clock,
            Some(&envelope),
            s_allocation::Mode::CapturedNative,
        );
        let open_ns = started.elapsed().as_nanos();
        let mut driver = match opened {
            Ok(d) => d,
            Err(error) => {
                std::fs::write(&out,serde_json::to_vec_pretty(&json!({"arm":arm,"open_ns":open_ns.to_string(),"open_error":error,"cold_runtime":true})).unwrap()).unwrap();
                panic!("Driver open失败");
            }
        };
        driver.audit_cache_source_bytes = Some(cap);
        if arm == "staged" {
            driver.runtime.enable_staging_for_native_test().unwrap();
        }
        std::fs::write(&out,serde_json::to_vec_pretty(&json!({"arm":arm,"status":"HANDLE_RUNNING","open_ns":open_ns.to_string(),"cold_restart_runtime":true,"S7_source_cap":cap})).unwrap()).unwrap();
        let started = std::time::Instant::now();
        let reply = driver.handle(envelope);
        let handle_ns = started.elapsed().as_nanos();
        if let Ok(value) = &reply {
            assert_eq!(value["kind"], "Committed");
            assert_eq!(meta_i64(&driver.conn, "generation").unwrap(), 106);
        }

        let live = driver.runtime.visit_runtime_roots().unwrap();
        let roots = live
            .into_iter()
            .map(
                |(owner, d)| json!({"owner":owner,"root":newchan_rust::s_content::root_string(&d)}),
            )
            .collect::<Vec<_>>();
        let result = json!({"arm":arm,"cold_restart_runtime":true,"105_warm_cache_preloaded":false,"full_open_audit":true,"full_handle_advance_prepare_audits":true,"open_ns":open_ns.to_string(),"handle_ns":handle_ns.to_string(),"handle_exceeds_original_30s":handle_ns>30_000_000_000u128,"network30s_qualified":false,"internal_direct_Driver_handle_only":true,"S7_source_cap":cap,"reply":reply.as_ref().ok(),"error":reply.as_ref().err(),"generation_after":meta_i64(&driver.conn,"generation").unwrap(),"runtime_roots":roots,"staging":driver.runtime.staging_report_for_native_test(),"receipt_read_mode":"Mode::CapturedNative; eager default no stage vs explicit Native setter; no fallback"});
        std::fs::write(out, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
        assert!(reply.is_ok(), "{}", reply.unwrap_err());
    }

    #[cfg(feature = "s_capture_native_testonly")]
    fn native_driver(f: &Fixture) -> Driver {
        Driver::open_with_request_and_mode(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            None,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap()
    }

    #[cfg(feature = "s_capture_native_testonly")]
    fn measured_source_cap(driver: &mut Driver) -> usize {
        let mut probe = super::super::cross_request_source_cache::Keeper::new(usize::MAX);
        let tx = driver.conn.transaction().unwrap();
        state::audit_native_with_source_keeper_for_testonly(&tx, &mut probe).unwrap();
        probe.last_required
    }

    /// #1394：S7 下正常写由完整 owner 候选承载。19cae1dc09（「接通正常写入的增量审计与新代核验」）把
    /// 完整候选的准入门由 a96cb02e01 的 no_write 改为 ready/receipt/ingest 全部固定操作，写前审核的
    /// slot（含来源 Counts 候选）随候选在固定写前 detach_slot 真实释放，不再移交稀疏来源池；稀疏池
    /// 只在完整候选未入选时承载（measured cap 的 source_keeper_driver_* 用例锁该路径）。本用例原先
    /// 按旧承载者断言「写前审核封存 n−1 个 cut」，按现设计改为锁住：行与可达原件逐项等于 cold 与
    /// ordinary；legacy＋旧池（来源池＋完整候选，与 Driver 冻结 workspace 预算同口径）＋workspace
    /// 共享 S7；每次正常写由完整候选增量承载且来源池为空；repost 只复用完整 owner 结果、不调来源核；
    /// 重启后首个 repost 全 cold，其无写候选在 slot 内保留来源 Counts 候选。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn counts_subset_driver_normal_rows_append_repost_restart_and_shared_s7() {
        use super::super::prepared::tests::{
            runtime_root_compare_reachable_bytes, runtime_root_tables,
        };
        const S7: usize = 536_870_912;
        let phases =
            |events: &[Value], phase: &str| events.iter().filter(|e| e["phase"] == phase).count();
        let warm_fixture = Fixture::new();
        let cold_fixture = Fixture::new();
        let ordinary_fixture = Fixture::new();
        let mut warm = Driver::open_for_serve_with_mode(
            &warm_fixture.db,
            &warm_fixture.profile,
            "1",
            warm_fixture.clock.clone(),
            S7,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        // 原冷核不启用source Keeper，仍使用相同的Native完整capture。
        let mut cold = native_driver(&cold_fixture);
        let mut ordinary = ordinary_fixture.open("1").unwrap();
        for n in 1..=3 {
            let legacy = crate::cache::retained_bytes();
            let old_pool = warm
                .conn
                .source_keeper_for_test()
                .as_ref()
                .map_or(0, |k| k.retained_source_bytes())
                + warm.conn.complete_retained_bytes();
            assert_eq!(old_pool > 0, n > 1, "首个入口没有旧池，之后旧池为完整候选");
            let (actual, events, _) = state::traced(false, || warm.handle(Fixture::ingest(n)));
            let actual = actual.unwrap();
            assert_eq!(actual, cold.handle(Fixture::ingest(n)).unwrap());
            assert_eq!(actual, ordinary.handle(Fixture::ingest(n)).unwrap());
            let workspace =
                content_manifest::manifest_workspace_stats().last_workspace_source_cost as usize;
            assert!(
                legacy
                    .checked_add(old_pool)
                    .and_then(|v| v.checked_add(workspace))
                    .unwrap()
                    <= S7
            );
            // 只有首代建候选时 cold；之后入口直接命中上一请求留下的完整候选。
            assert_eq!(phases(&events, "request_history_cold"), usize::from(n == 1));
            assert_eq!(
                phases(&events, "owner_complete_cold_candidate"),
                usize::from(n == 1)
            );
            assert!(phases(&events, "owner_complete_hit") > 0);
            let keeper_owner = warm.conn.source_keeper_for_test();
            let keeper = keeper_owner.as_ref().unwrap();
            let report = keeper.report();
            // 写前来源候选随 slot 在固定写前释放；post-COMMIT 也不为新 head 凭空生成资格。
            assert_eq!(report["retained_counts_proofs"], 0);
            assert_eq!(keeper.retained_source_bytes(), 0);
            assert!(report["counts_subset_selection"].is_null());
            assert!(report["retained_capture"].is_null());
            assert_eq!((keeper.hits, keeper.cold), (0, 0));
            let complete = warm.conn.complete_report();
            assert!(warm.conn.complete_retained_bytes() > 0);
            assert_eq!(complete["charge"]["incremental_steps"], 3 * n);
            assert_eq!(complete["charge"]["slot_result_capacity"], 0);
            assert_eq!(complete["charge"]["source_pool_charge"], 0);
            assert_eq!(complete["source_counts_preserved"], false);
            assert!(
                crate::cache::retained_bytes()
                    .checked_add(keeper.retained_source_bytes())
                    .and_then(|v| v.checked_add(warm.conn.complete_retained_bytes()))
                    .unwrap()
                    <= S7
            );
            assert_eq!(
                runtime_root_tables(&warm.conn, true),
                runtime_root_tables(&cold.conn, true)
            );
            assert_eq!(
                runtime_root_tables(&warm.conn, true),
                runtime_root_tables(&ordinary.conn, true)
            );
            runtime_root_compare_reachable_bytes(&warm.conn, &cold.conn);
            runtime_root_compare_reachable_bytes(&warm.conn, &ordinary.conn);
        }
        let held = warm.conn.complete_retained_bytes();
        // 两次repost都只复用完整owner结果，不调用来源核，也不改动完整候选。
        for _ in 0..2 {
            let (actual, events, _) = state::traced(false, || warm.handle(Fixture::ingest(3)));
            let actual = actual.unwrap();
            assert_eq!(actual, cold.handle(Fixture::ingest(3)).unwrap());
            assert_eq!(actual, ordinary.handle(Fixture::ingest(3)).unwrap());
            assert_eq!(phases(&events, "owner_complete_hit"), 1);
            assert_eq!(phases(&events, "request_history_cold"), 0);
            assert_eq!(phases(&events, "request_history_hit"), 0);
            assert_eq!(warm.conn.complete_retained_bytes(), held);
            assert_eq!(
                runtime_root_tables(&warm.conn, true),
                runtime_root_tables(&cold.conn, true)
            );
            assert_eq!(
                runtime_root_tables(&warm.conn, true),
                runtime_root_tables(&ordinary.conn, true)
            );
        }
        {
            let keeper_owner = warm.conn.source_keeper_for_test();
            let keeper = keeper_owner.as_ref().unwrap();
            assert_eq!((keeper.hits, keeper.cold), (0, 0));
            assert_eq!(keeper.retained_source_bytes(), 0);
        }
        drop(warm);
        let mut restarted = Driver::open_for_serve_with_mode(
            &warm_fixture.db,
            &warm_fixture.profile,
            "1",
            warm_fixture.clock.clone(),
            S7,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        assert!(restarted.conn.source_keeper_for_test().is_none());
        assert_eq!(restarted.conn.complete_retained_bytes(), 0);
        assert_eq!(
            restarted.handle(Fixture::ingest(3)).unwrap(),
            cold.handle(Fixture::ingest(3)).unwrap()
        );
        assert_eq!(
            (
                restarted
                    .conn
                    .source_keeper_for_test()
                    .as_ref()
                    .unwrap()
                    .hits,
                restarted
                    .conn
                    .source_keeper_for_test()
                    .as_ref()
                    .unwrap()
                    .cold
            ),
            (0, 3)
        );
        // 无写候选不经固定写，来源 Counts 候选留在其 slot 内并照收费。
        let complete = restarted.conn.complete_report();
        assert_eq!(complete["source_counts_preserved"], true);
        assert!(complete["charge"]["source_pool_charge"].as_u64().unwrap() > 0);
        assert_eq!(
            runtime_root_tables(&restarted.conn, true),
            runtime_root_tables(&cold.conn, true)
        );
        assert_eq!(
            runtime_root_tables(&restarted.conn, true),
            runtime_root_tables(&ordinary.conn, true)
        );
        runtime_root_compare_reachable_bytes(&restarted.conn, &cold.conn);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn source_keeper_driver_append_repost_restart_preserves_results_and_request_reuse() {
        let reference = Fixture::new();
        let mut ordinary = reference.open("1").unwrap();
        let expected = (1..=3)
            .map(|n| ordinary.handle(Fixture::ingest(n)).unwrap())
            .collect::<Vec<_>>();
        let cap = measured_source_cap(&mut ordinary);
        let fixture = Fixture::new();
        let mut driver = Driver::open_for_serve_with_mode(
            &fixture.db,
            &fixture.profile,
            "1",
            fixture.clock.clone(),
            cap,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        assert!(
            driver.conn.source_keeper_for_test().is_none(),
            "startup不得沿用旧请求缓存"
        );
        for n in 1..=3 {
            let (actual, events, _) = state::traced(false, || driver.handle(Fixture::ingest(n)));
            assert_eq!(actual.unwrap(), expected[n as usize - 1]);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_cold")
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_hit")
                    .count(),
                2
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_context_released")
                    .count(),
                1
            );
            let keeper_owner = driver.conn.source_keeper_for_test();
            let keeper = keeper_owner.as_ref().unwrap();
            assert!(keeper.retained_source_bytes() <= cap);
            let report = keeper.report();
            if n == 1 {
                // 初次写前历史没有cut，不保留空证明池。
                assert_eq!(keeper.retained_source_bytes(), 0);
                assert!(report["retained_capture"].is_null());
            } else {
                assert!(keeper.retained_source_bytes() > 0);
                for mode in ["public", "template"] {
                    assert!(report["retained_capture"][mode]
                        .as_object()
                        .unwrap()
                        .values()
                        .all(|v| v.as_u64() == Some(0)));
                }
                assert_eq!(
                    report["retained_capture"]["native_frame_reuse"]["retained_entries"],
                    0
                );
            }
        }
        assert!(
            driver.conn.source_keeper_for_test().as_ref().unwrap().hits > 0,
            "追加请求须复用已核旧cut"
        );
        assert_eq!(driver.handle(Fixture::ingest(3)).unwrap(), expected[2]);
        let hits = driver.conn.source_keeper_for_test().as_ref().unwrap().hits;
        assert_eq!(driver.handle(Fixture::ingest(3)).unwrap(), expected[2]);
        assert_eq!(
            driver.conn.source_keeper_for_test().as_ref().unwrap().hits - hits,
            3
        );
        assert_eq!(
            driver.state_summary("cached-native").unwrap(),
            ordinary.state_summary("cached-native").unwrap()
        );
        drop(driver);
        let mut restarted = Driver::open_for_serve_with_mode(
            &fixture.db,
            &fixture.profile,
            "1",
            fixture.clock.clone(),
            cap,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        assert!(restarted.conn.source_keeper_for_test().is_none());
        assert_eq!(restarted.handle(Fixture::ingest(3)).unwrap(), expected[2]);
        let keeper_owner = restarted.conn.source_keeper_for_test();
        let keeper = keeper_owner.as_ref().unwrap();
        assert_eq!((keeper.cold, keeper.hits), (3, 0));
    }

    /// 三次真实追加；Driver 冻结 workspace 预算时旧池＝来源池＋完整 owner 候选（handle_with_request_audit）。
    /// #1394：原断言「仅第3次入口拥有前次写前审核留下的非空来源池」按 a96cb02e01 前的承载者写成；
    /// 19cae1dc09 起正常写由完整候选承载、写前来源候选随 slot 在固定写前释放，故第2、3次入口的
    /// 非空旧池是完整候选，来源池恒空。All actual rows and reachable originals must match cold Native and ordinary.
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn source_keeper_driver_workspace_shares_s7_with_nonempty_old_pool_and_exact_rows() {
        use super::super::prepared::tests::{
            runtime_root_compare_reachable_bytes, runtime_root_tables,
        };
        let total = 536_870_912;
        let warm_fixture = Fixture::new();
        let cold_fixture = Fixture::new();
        let ordinary_fixture = Fixture::new();
        let mut warm = Driver::open_for_serve_with_mode(
            &warm_fixture.db,
            &warm_fixture.profile,
            "1",
            warm_fixture.clock.clone(),
            total,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        let mut cold = Driver::open_for_serve_with_mode(
            &cold_fixture.db,
            &cold_fixture.profile,
            "1",
            cold_fixture.clock.clone(),
            total,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        cold.runtime.disable_manifest_workspace_for_test();
        let mut ordinary = ordinary_fixture.open("1").unwrap();
        let before = content_manifest::manifest_workspace_stats();
        let mut nonempty_shared = 0;
        for n in 1..=3 {
            let legacy = crate::cache::retained_bytes();
            let source_pool = warm
                .conn
                .source_keeper_for_test()
                .as_ref()
                .map_or(0, |k| k.retained_source_bytes());
            assert_eq!(source_pool, 0);
            let old_pool = source_pool + warm.conn.complete_retained_bytes();
            if old_pool > 0 {
                nonempty_shared += 1
            }
            let actual = warm.handle(Fixture::ingest(n)).unwrap();
            let stats = content_manifest::manifest_workspace_stats();
            let cost = stats.last_workspace_source_cost as usize;
            assert!(cost > 0);
            assert!(
                legacy
                    .checked_add(old_pool)
                    .and_then(|v| v.checked_add(cost))
                    .unwrap()
                    <= total
            );
            assert_eq!(actual, cold.handle(Fixture::ingest(n)).unwrap());
            assert_eq!(actual, ordinary.handle(Fixture::ingest(n)).unwrap());
            let rows = runtime_root_tables(&warm.conn, true);
            assert_eq!(rows, runtime_root_tables(&cold.conn, true));
            assert_eq!(rows, runtime_root_tables(&ordinary.conn, true));
            runtime_root_compare_reachable_bytes(&warm.conn, &cold.conn);
            runtime_root_compare_reachable_bytes(&warm.conn, &ordinary.conn);
            let keeper_owner = warm.conn.source_keeper_for_test();
            let keeper = keeper_owner.as_ref().unwrap();
            assert_eq!(keeper.retained_source_bytes(), 0);
            assert!(warm.conn.complete_retained_bytes() > 0);
            assert_eq!(
                keeper.report()["cap"].as_u64().unwrap() as usize,
                total.checked_sub(legacy).unwrap()
            );
            assert!(
                crate::cache::retained_bytes()
                    .checked_add(keeper.retained_source_bytes())
                    .and_then(|v| v.checked_add(warm.conn.complete_retained_bytes()))
                    .unwrap()
                    <= total
            );
            eprintln!("shared workspace gen{n}: legacy={legacy} old_pool={old_pool} workspace={cost} S7={total}");
        }
        let after = content_manifest::manifest_workspace_stats();
        assert_eq!(nonempty_shared, 2);
        assert_eq!(after.issued - before.issued, 3);
        assert_eq!(after.rebound - before.rebound, 3);
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
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn source_keeper_driver_measured_cap_only_changes_retention() {
        let fixture = Fixture::new();
        let mut seed = fixture.open("1").unwrap();
        let reply = seed.handle(Fixture::ingest(1)).unwrap();
        let cap = measured_source_cap(&mut seed);
        drop(seed);
        for budget in [cap, cap - 1, 1] {
            let mut driver = Driver::open_for_serve_with_mode(
                &fixture.db,
                &fixture.profile,
                "1",
                fixture.clock.clone(),
                budget,
                s_allocation::Mode::CapturedNative,
            )
            .unwrap();
            assert_eq!(
                driver.handle(Fixture::ingest(1)).unwrap(),
                reply,
                "缓存不足不改变已提交回复"
            );
            let retained = driver
                .conn
                .source_keeper_for_test()
                .as_ref()
                .map_or(0, |k| k.retained_source_bytes());
            assert_eq!(retained, if budget == cap { cap } else { 0 });
            if budget == 1 {
                assert!(
                    driver.conn.source_keeper_for_test().is_none(),
                    "原S7=1不创建keeper"
                );
            }
            assert_eq!(driver.handle(Fixture::ingest(1)).unwrap(), reply);
            assert_eq!(meta_i64(&driver.conn, "generation").unwrap(), 1);
        }
        let mut ordinary = fixture.open("1").unwrap();
        ordinary.audit_cache_source_bytes = Some(cap);
        assert_eq!(ordinary.handle(Fixture::ingest(1)).unwrap(), reply);
        assert!(
            ordinary.conn.source_keeper_for_test().is_none(),
            "普通路径没有keeper"
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn request_history_real_driver_three_stages_reuse_and_repost_starts_cold() {
        let fixture = Fixture::new();
        let ordinary = Fixture::new();
        let mut native = native_driver(&fixture);
        let mut baseline = ordinary.open("1").unwrap();
        for n in 1..=3 {
            let (actual, events, _) = state::traced(false, || native.handle(Fixture::ingest(n)));
            assert_eq!(
                actual.unwrap(),
                baseline.handle(Fixture::ingest(n)).unwrap()
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_cold")
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_hit")
                    .count(),
                2
            );
            let (_, events, _) =
                state::traced(false, || native.handle(Fixture::ingest(n)).unwrap());
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_cold")
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_hit")
                    .count(),
                0
            );
        }
        let tx = native.conn.transaction().unwrap();
        let a = state::audit(&tx).unwrap();
        let other = baseline.conn.transaction().unwrap();
        let b = state::audit(&other).unwrap();
        assert_eq!(a.cuts.len(), b.cuts.len());
        for (a, b) in a.cuts.iter().zip(b.cuts.iter()) {
            assert_eq!(a.snapshot.digest().unwrap(), b.snapshot.digest().unwrap());
            assert_eq!(a.delta.digest().unwrap(), b.delta.digest().unwrap());
            assert_eq!(a.batch_id.as_str(), b.batch_id.as_str());
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn request_history_handler_sql_and_commit_failures_release_context() {
        use s_allocation::{probe, Phase};
        for phase in [Phase::Handle, Phase::Advance, Phase::Prepare] {
            let fixture = Fixture::new();
            let mut driver = native_driver(&fixture);
            probe::start(Some((phase, probe::Fault::SqlStep)));
            let (result, events, _) = state::traced(false, || driver.handle(Fixture::ingest(1)));
            assert!(result.is_err());
            driver
                .conn
                .progress_handler(0, None::<fn() -> bool>)
                .unwrap();
            probe::finish();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == "request_history_context_released")
                    .count(),
                1
            );
        }
        // 真SQLITE COMMIT hook拒绝：prepare/写节点完成后final tx.commit失败并回滚。
        let fixture = Fixture::new();
        let mut driver = native_driver(&fixture);
        driver
            .runtime
            .set_after_prepare_for_request_test(Some(|tx| {
                tx.commit_hook(Some(|| true)).map_err(err)?;
                Ok(())
            }));
        let (result, events, _) = state::traced(false, || driver.handle(Fixture::ingest(1)));
        assert!(result.is_err());
        driver.conn.commit_hook(None::<fn() -> bool>).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e["phase"] == "request_history_hit")
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e["phase"] == "request_history_context_released")
                .count(),
            1
        );
        assert_eq!(count(&driver.conn, "s_content_publications"), 0);
        assert_eq!(count(&driver.conn, "raw_events"), 1);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_capture_three_commits_and_restart_match_ordinary_current_state() {
        use s_allocation::{probe, Phase};
        for restart_after_two in [false, true] {
            let f = Fixture::new();
            let ordinary = Fixture::new();
            let mut d = native_driver(&f);
            let mut plain = ordinary.open("1").unwrap();
            for n in 1..=3 {
                probe::start(None);
                let input = Fixture::ingest(n);
                let actual = d.handle(input.clone()).unwrap();
                let expected = plain.handle(input).unwrap();
                let (attempts, captures) = probe::finish();
                assert_eq!(canonical_json(&actual), canonical_json(&expected));
                assert_eq!(attempts, [1, 1, 1]);
                for (i, (phase, raw, clocks, has_begin)) in [
                    (Phase::Handle, (n - 1) as usize, 3 * (n - 1), false),
                    (Phase::Advance, n as usize, 3 * (n - 1) + 1, false),
                    (Phase::Prepare, n as usize, 3 * (n - 1) + 2, true),
                ]
                .into_iter()
                .enumerate()
                {
                    let c = captures[i].unwrap();
                    assert_eq!(
                        (c.phase, c.raw, c.clocks, c.has_begin),
                        (phase, raw, clocks, has_begin)
                    );
                    assert_eq!(c.capture_return_ordinal, i + 1);
                    assert_eq!(c.live_before, 0, "Native不创建OwnedBudget");
                }
                assert_eq!(
                    d.state_summary("native-testonly").unwrap(),
                    plain.state_summary("native-testonly").unwrap()
                );
                let roots = |conn: &Connection| {
                    conn.prepare("SELECT batch_id,hex(manifest_root),hex(snapshot_root),hex(delta_root) FROM s_content_publications ORDER BY generation")
                        .unwrap().query_map([], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?,r.get::<_, String>(3)?)))
                        .unwrap().collect::<Result<Vec<_>, _>>().unwrap()
                };
                assert_eq!(roots(&d.conn), roots(&plain.conn));
                assert_eq!(count(&d.conn, "s_content_publications"), n);
                assert_eq!(count(&d.conn, "structure_deltas"), n);
                assert_eq!(count(&d.conn, "s_clock_events"), 3 * n);
                assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
                if n == 2 && restart_after_two {
                    drop(d);
                    d = native_driver(&f);
                }
            }
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_mode_is_frozen_before_pending_resume_captures_advance_and_prepare() {
        use s_allocation::probe;
        let f = Fixture::new();
        let mut ordinary = f.open("1").unwrap();
        ordinary.stop_after = Some("accept");
        assert!(ordinary
            .handle(Fixture::ingest(1))
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(ordinary);
        probe::start(None);
        let d = native_driver(&f);
        let (attempts, captures) = probe::finish();
        assert_eq!(attempts, [0, 1, 1]);
        assert_eq!(captures[1].unwrap().capture_return_ordinal, 1);
        assert_eq!(captures[2].unwrap().capture_return_ordinal, 2);
        assert_eq!(captures[1].unwrap().clocks, 1);
        assert_eq!(captures[2].unwrap().clocks, 2);
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 1);
        assert_eq!(count(&d.conn, "s_clock_events"), 3);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_capture_real_sql_failures_preserve_each_persistent_phase_boundary() {
        use s_allocation::{probe, Phase};
        for fault in [probe::Fault::SqlPrepare, probe::Fault::SqlStep] {
            for (index, phase) in [Phase::Handle, Phase::Advance, Phase::Prepare]
                .into_iter()
                .enumerate()
            {
                let f = Fixture::new();
                let mut d = native_driver(&f);
                probe::start(Some((phase, fault)));
                let error = d.handle(Fixture::ingest(1)).unwrap_err();
                d.conn.progress_handler(0, None::<fn() -> bool>);
                let (attempts, captures) = probe::finish();
                let expected = match fault {
                    // Native构造的节点SELECT复用rusqlite prepare，实际保留连接errmsg。
                    probe::Fault::SqlPrepare => {
                        "S3StateInvalid：source: S3 内容存储：no such table: s_content_nodes"
                            .to_owned()
                    }
                    probe::Fault::SqlStep => {
                        // ensure_format仍是CheckedStatement，保留原SQLITE_INTERRUPT code。
                        let diagnostic = rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT),
                            None,
                        );
                        format!("S3StateInvalid：source: S3 内容存储：{diagnostic}")
                    }
                    _ => unreachable!(),
                };
                assert_eq!(error, expected, "{phase:?}");
                for i in 0..3 {
                    assert_eq!(attempts[i], usize::from(i <= index));
                    assert_eq!(captures[i].is_some(), i < index);
                }
                assert_allocation_boundary(&d.conn, index);
            }
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_capture_readonly_current_source_rejects_fresh_damage_without_writes() {
        let f = Fixture::new();
        let mut d = native_driver(&f);
        d.handle(Fixture::ingest(1)).unwrap();
        let readonly =
            Connection::open_with_flags(&f.db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        readonly.pragma_update(None, "query_only", true).unwrap();
        {
            let tx = readonly.unchecked_transaction().unwrap();
            let audited = state::audit_native_current(&tx).unwrap();
            assert!(audited.budget.is_none());
            assert!(audited.raw.budget().is_none());
            assert_eq!(audited.raw.len(), 1);
            assert_eq!(audited.cuts.len(), 1);
            let error = tx.execute("UPDATE meta SET value=value", []).unwrap_err();
            assert_eq!(
                error.sqlite_error_code(),
                Some(rusqlite::ErrorCode::ReadOnly)
            );
        }
        let (digest, mut body): (Vec<u8>, Vec<u8>) = d.conn.query_row(
            "SELECT n.digest,n.body FROM s_content_nodes n JOIN s_content_publications p ON n.digest=p.snapshot_root WHERE p.generation=1",
            [], |r| Ok((r.get(0)?,r.get(1)?)),
        ).unwrap();
        body[0] ^= 1;
        d.conn
            .execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=?2",
                params![body, digest],
            )
            .unwrap();
        let tx = readonly.unchecked_transaction().unwrap();
        let error = state::audit_native_current(&tx).err().unwrap();
        assert!(
            error.contains(newchan_rust::s_content::code::NODE_DIGEST),
            "{error}"
        );
        drop(tx);
        s_allocation::probe::start(None);
        let error = d.handle(Fixture::ingest(2)).unwrap_err();
        let (attempts, captures) = s_allocation::probe::finish();
        assert!(
            error.contains(newchan_rust::s_content::code::NODE_DIGEST),
            "{error}"
        );
        assert_eq!(attempts, [1, 0, 0]);
        assert!(captures.iter().all(Option::is_none));
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 1);
        assert_eq!(count(&d.conn, "raw_events"), 1);
        assert_eq!(count(&d.conn, "s_input_messages"), 1);
        assert_eq!(count(&d.conn, "s_content_publications"), 1);
        assert_eq!(count(&d.conn, "structure_deltas"), 1);
        assert_eq!(count(&d.conn, "s_clock_events"), 3);
        assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn allocation_profile() -> q_allocation::Profile {
        // 既有具名 TestOnly 对照域，绝非生产容量或资源默认值。
        q_allocation::Profile {
            budget: newchan_rust::s_content::capture_allocation::OwnedBudget::new(50 * 1024 * 1024),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn allocation_driver(f: &Fixture, profile: q_allocation::Profile) -> Driver {
        Driver::open_with_request_and_allocation(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            None,
            Some(profile),
        )
        .unwrap()
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_real_ingest_captures_current_three_phases_and_preserves_receipt() {
        use s_allocation::{probe, Phase};
        let f = Fixture::new();
        let ordinary = Fixture::new();
        let expected = ordinary
            .open("1")
            .unwrap()
            .handle(Fixture::ingest(1))
            .unwrap();
        let profile = allocation_profile();
        let budget = profile.budget.clone();
        let mut d = allocation_driver(&f, profile);
        probe::start(None);
        let receipt = d.handle(Fixture::ingest(1)).unwrap();
        let (attempts, captures) = probe::finish();
        assert_eq!(receipt, expected, "公开回复与普通路径逐字相同");
        assert_eq!(attempts, [1, 1, 1]);
        for (index, (phase, raw, clocks, has_begin)) in [
            (Phase::Handle, 0, 0, false),
            (Phase::Advance, 1, 1, false),
            (Phase::Prepare, 1, 2, true),
        ]
        .into_iter()
        .enumerate()
        {
            let observed = captures[index].unwrap();
            assert_eq!(observed.phase, phase);
            assert_eq!(observed.raw, raw);
            assert_eq!(observed.clocks, clocks);
            assert_eq!(observed.has_begin, has_begin);
            assert_eq!(observed.capture_return_ordinal, index + 1);
        }
        assert_eq!(captures[0].unwrap().live_before, 0);
        assert_eq!(
            captures[1].unwrap().live_before,
            0,
            "handle 的审计结果与 capture 已释放"
        );
        assert!(
            captures[2].unwrap().live_before > 0,
            "advance 审计结果跨 prepare 存活并持续计费"
        );
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 1);
        assert_eq!(count(&d.conn, "raw_events"), 1);
        assert_eq!(count(&d.conn, "s_content_publications"), 1);
        assert_eq!(count(&d.conn, "structure_deltas"), 1);
        assert_eq!(count(&d.conn, "s_clock_events"), 3);
        let tx = d.conn.transaction().unwrap();
        let published = state::audit(&tx).unwrap();
        assert_eq!(published.cuts.len(), 1);
        assert_eq!(published.raw.len(), 1);
        drop(published);
        drop(tx);
        // 同账业务缓存按真实生命周期留账；不为追求零读数主动清缓存。
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
        drop(d);
        let meter = budget.snapshot();
        assert!(meter.peak_requested_layout_bytes > 0);
        assert_eq!(meter.live_requested_layout_bytes, 0);
        assert_eq!(meter.allocated_layouts, meter.released_layouts);
        assert!(!budget.ledger_health().invariant_broken);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_three_commits_match_ordinary_with_same_ledger_and_real_restart() {
        use s_allocation::probe;
        // 每个重启分支都是独立新夹具，绝不连接实测 A01 库。
        for restart_after_two in [false, true] {
            let f = Fixture::new();
            let ordinary = Fixture::new();
            let mut plain = ordinary.open("1").unwrap();
            let profile = allocation_profile();
            let budget = profile.budget.clone();
            let mut d = allocation_driver(&f, profile);
            for n in 1..=3 {
                let live_before = budget.snapshot().live_requested_layout_bytes;
                probe::start(None);
                let input = Fixture::ingest(n);
                let actual = d.handle(input.clone()).unwrap();
                let expected = plain.handle(input).unwrap();
                let (attempts, captures) = probe::finish();
                assert_eq!(canonical_json(&actual), canonical_json(&expected));
                assert_eq!(attempts, [1, 1, 1]);
                assert_eq!(captures[0].unwrap().raw, (n - 1) as usize);
                assert_eq!(captures[1].unwrap().raw, n as usize);
                assert_eq!(captures[2].unwrap().raw, n as usize);
                assert_eq!(captures[0].unwrap().clocks, 3 * (n - 1));
                assert_eq!(captures[1].unwrap().clocks, 3 * (n - 1) + 1);
                assert_eq!(captures[2].unwrap().clocks, 3 * (n - 1) + 2);
                assert_eq!(captures[0].unwrap().live_before, live_before);
                assert_eq!(captures[1].unwrap().live_before, live_before);
                assert!(captures[2].unwrap().live_before > live_before);
                assert_eq!(meta_i64(&d.conn, "generation").unwrap(), n);
                assert_eq!(count(&d.conn, "s_content_publications"), n);
                assert_eq!(count(&d.conn, "structure_deltas"), n);
                assert_eq!(count(&d.conn, "s_clock_events"), 3 * n);
                // Publication 根和 batch 逐字一致，覆盖生成、辅助索引、Snapshot/Delta。
                let roots = |conn: &Connection| {
                    conn.prepare("SELECT batch_id,hex(manifest_root),hex(snapshot_root),hex(delta_root) FROM s_content_publications ORDER BY generation")
                        .unwrap().query_map([], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?,r.get::<_, String>(3)?)))
                        .unwrap().collect::<Result<Vec<_>, _>>().unwrap()
                };
                assert_eq!(roots(&d.conn), roots(&plain.conn));
                assert!(
                    budget.snapshot().live_requested_layout_bytes > 0,
                    "业务缓存真实跨请求持账"
                );
                assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
                if n == 2 && restart_after_two {
                    drop(d);
                    assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
                    // 相同私有预算重开，缓存从当前已发布原件重载，第三笔仍逐字对拍。
                    d = allocation_driver(
                        &f,
                        q_allocation::Profile {
                            budget: budget.clone(),
                            overlay_slots: 32768,
                            overlay_bytes: 1_000_000,
                        },
                    );
                }
            }
            drop(d);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert_eq!(
                budget.snapshot().allocated_layouts,
                budget.snapshot().released_layouts
            );
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_each_phase_budget_refuses_without_fallback_or_crossing_persistent_boundary(
    ) {
        use s_allocation::{probe, Phase};
        for (index, phase) in [Phase::Handle, Phase::Advance, Phase::Prepare]
            .into_iter()
            .enumerate()
        {
            let f = Fixture::new();
            let profile = allocation_profile();
            let budget = profile.budget.clone();
            let mut d = allocation_driver(&f, profile);
            probe::start(Some((phase, probe::Fault::Budget)));
            let error = d.handle(Fixture::ingest(1)).unwrap_err();
            let (attempts, captures) = probe::finish();
            assert!(
                error.contains(newchan_rust::s_content::capture_allocation::LIMIT),
                "{phase:?}: {error}"
            );
            for i in 0..3 {
                assert_eq!(attempts[i], usize::from(i <= index));
                assert_eq!(captures[i].is_some(), i < index);
            }
            assert_allocation_boundary(&d.conn, index);
            let meter = budget.snapshot();
            assert!(
                meter.rejected_layouts > 0,
                "真实 OwnedBudget 必须拒绝实际申请"
            );
            assert_eq!(meter.live_requested_layout_bytes, 0);
            assert_eq!(meter.allocated_layouts, meter.released_layouts);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[cfg(any(
        feature = "s_allocation_json_testonly",
        feature = "s_capture_native_testonly"
    ))]
    fn assert_allocation_boundary(conn: &Connection, completed_phases: usize) {
        assert_eq!(meta_i64(conn, "generation").unwrap(), 0);
        assert_eq!(count(conn, "raw_events"), i64::from(completed_phases > 0));
        assert_eq!(
            count(conn, "s_input_messages"),
            i64::from(completed_phases > 0)
        );
        assert_eq!(count(conn, "s_clock_events"), completed_phases as i64);
        assert_eq!(
            meta_get_opt(conn, "advance_state").unwrap().as_deref() == Some("idle"),
            completed_phases < 2
        );
        let pending: i64 = conn
            .query_row(
                "SELECT count(*) FROM s_input_messages WHERE status='AcceptedPending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, i64::from(completed_phases > 0));
        for table in [
            "s_content_nodes",
            "s_content_objects",
            "s_content_publications",
            "structure_deltas",
        ] {
            assert_eq!(count(conn, table), 0, "{table}");
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_real_sql_prepare_and_step_errors_propagate_at_each_phase() {
        use s_allocation::{probe, Phase};
        for fault in [probe::Fault::SqlPrepare, probe::Fault::SqlStep] {
            for (index, phase) in [Phase::Handle, Phase::Advance, Phase::Prepare]
                .into_iter()
                .enumerate()
            {
                let f = Fixture::new();
                let profile = allocation_profile();
                let budget = profile.budget.clone();
                let mut d = allocation_driver(&f, profile);
                probe::start(Some((phase, fault)));
                let error = d.handle(Fixture::ingest(1)).unwrap_err();
                d.conn.progress_handler(0, None::<fn() -> bool>);
                let (attempts, captures) = probe::finish();
                // CheckedStatement 保留 sqlite3_prepare_v2 / sqlite3_step 的真实 code，
                // SqliteFailure(_, None) 不携带连接 errmsg。锁 code 和完整既有封装，
                // 不能要求已被该 checked 边界舍弃的 "no such table" 文本。
                let code = match fault {
                    probe::Fault::SqlPrepare => rusqlite::ffi::SQLITE_ERROR,
                    probe::Fault::SqlStep => rusqlite::ffi::SQLITE_INTERRUPT,
                    _ => unreachable!(),
                };
                let diagnostic =
                    rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
                let expected = format!("S3StateInvalid：source: S3 内容存储：{diagnostic}");
                assert_eq!(error, expected, "{phase:?}: actual SQLite code {code}");
                for i in 0..3 {
                    assert_eq!(attempts[i], usize::from(i <= index));
                    assert_eq!(captures[i].is_some(), i < index);
                }
                assert_allocation_boundary(&d.conn, index);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
                assert!(!budget.ledger_health().invariant_broken);
            }
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_testonly_is_bound_before_resume_pending_advances() {
        use s_allocation::{probe, Phase};
        let f = Fixture::new();
        let mut ordinary = f.open("1").unwrap();
        ordinary.stop_after = Some("accept");
        assert!(ordinary
            .handle(Fixture::ingest(1))
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(ordinary);
        let profile = allocation_profile();
        let budget = profile.budget.clone();
        probe::start(Some((Phase::Advance, probe::Fault::Budget)));
        let error = Driver::open_with_request_and_allocation(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            None,
            Some(profile),
        )
        .err()
        .unwrap();
        let (attempts, _) = probe::finish();
        assert!(
            error.contains(newchan_rust::s_content::capture_allocation::LIMIT),
            "{error}"
        );
        assert_eq!(attempts, [0, 1, 0]);
        let conn = open_db(&f.db).unwrap();
        assert_allocation_boundary(&conn, 1);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn cold_audit_live_driver_requests_re_read_and_reject_current_node_damage() {
        use super::super::state::cold_audit_probe as probe;
        let f = Fixture::new();
        let first = Fixture::ingest(1);
        let second = Fixture::ingest(2);
        let inputs = json!([first, second]);
        let cap = probe::begin(&inputs);
        let mut d = Driver::open_for_serve(&f.db, &f.profile, "1", f.clock.clone(), 1).unwrap();
        assert_eq!(d.audit_cache_source_bytes, Some(1));
        assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
        let mut phases = Vec::new();
        for (i, input) in inputs.as_array().unwrap().iter().enumerate() {
            let (result, mut phase) = probe::observe(&format!("actual_ingest_{}", i + 1), || {
                d.handle(input.clone())
            });
            assert_eq!(result.unwrap()["kind"], "Committed");
            let bytes = canonical_json(input);
            phase["input_frame"] = json!({"canonical_body_bytes":bytes.len(),"lf_frame_bytes":bytes.len()+1,
                "body_sha256":sha256_hex(bytes.as_bytes()),"actual_transport_io":false});
            phase["storage_after"] = probe::storage(&d.conn, &f.db);
            phases.push(phase);
        }
        let binding = probe::binding(&d.conn, &inputs, cap);
        let mut original_ready = None;
        for label in [
            "live_driver_ready_first",
            "live_driver_ready_after_prior_result_dropped",
        ] {
            let request = envelope(label, json!({"op":"ready"}));
            let (result, mut phase) = probe::observe(label, || d.handle(request.clone()));
            let ready = result.unwrap();
            assert_eq!(ready["kind"], "Ready");
            if let Some(ref prior) = original_ready {
                assert_eq!(&ready, prior)
            } else {
                original_ready = Some(ready.clone())
            }
            assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
            let bytes = canonical_json(&request);
            phase["input_frame"] = json!({"canonical_body_bytes":bytes.len(),"lf_frame_bytes":bytes.len()+1,
                "body_sha256":sha256_hex(bytes.as_bytes()),"actual_transport_io":false});
            phase["ready_payload_canonical_bytes"] = json!(canonical_json(&ready).len());
            drop(ready);
            phase["storage_after"] = probe::storage(&d.conn, &f.db);
            phases.push(phase);
        }
        let (digest,body):(Vec<u8>,Vec<u8>)=d.conn.query_row(
            "SELECT n.digest,n.body FROM s_content_nodes n JOIN s_content_publications p ON n.digest=p.snapshot_root ORDER BY p.generation LIMIT 1",
            [],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        let mut damaged = body.clone();
        damaged[0] ^= 1;
        d.conn
            .execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=?2",
                params![damaged, &digest],
            )
            .unwrap();
        let (result, mut phase) = probe::observe("live_driver_damaged_published_node", || {
            d.handle(envelope("damaged-ready", json!({"op":"ready"})))
        });
        assert!(result.is_err());
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        phase["storage_after"] = probe::storage(&d.conn, &f.db);
        phases.push(phase);
        d.conn
            .execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=?2",
                params![body, &digest],
            )
            .unwrap();
        let (result, mut phase) = probe::observe("live_driver_restored_original_node", || {
            d.handle(envelope("restored-ready", json!({"op":"ready"})))
        });
        assert_eq!(result.unwrap(), *original_ready.as_ref().unwrap());
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        phase["storage_after"] = probe::storage(&d.conn, &f.db);
        phases.push(phase);
        drop(d);
        let (result, mut phase) = probe::observe("driver_reopen_current_history", || {
            Driver::open_for_serve(&f.db, &f.profile, "1", f.clock.clone(), 1)
        });
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        phase["storage_after"] = probe::storage(&result.as_ref().unwrap().conn, &f.db);
        phases.push(phase);
        let mut reopened = result.unwrap();
        let (result, mut phase) = probe::observe("reopened_driver_ready", || {
            reopened.handle(envelope("reopened-ready", json!({"op":"ready"})))
        });
        assert_eq!(result.unwrap(), original_ready.unwrap());
        assert!(phase["work"]["source_reads"].as_u64().unwrap() > 0);
        phase["storage_after"] = probe::storage(&reopened.conn, &f.db);
        phases.push(phase);
        probe::archive(
            &reopened.conn,
            &f.db,
            "driver-cold-audit",
            &json!({"binding":binding,"phases":phases}),
        );
    }

    fn cli(f: &Fixture, command: &str, epoch_flag: &str, epoch: &str) -> Vec<String> {
        [
            "s_structure_session".into(),
            command.into(),
            "--protocol-revision".into(),
            PROTOCOL_REVISION.into(),
            "--db".into(),
            f.db.to_string_lossy().into_owned(),
            epoch_flag.into(),
            epoch.into(),
            "--clock-plan".into(),
            f.path.join("clock.json").to_string_lossy().into_owned(),
        ]
        .to_vec()
    }

    fn one_shot(f: &Fixture, request: &Value, epoch: &str) -> Result<Option<Value>, String> {
        let path = f.path.join("one-shot.json");
        let bytes = canonical_json(request);
        std::fs::write(&path, &bytes).unwrap();
        let mut args = cli(f, "advance", "--writer-epoch", epoch);
        args.extend([
            "--profile".into(),
            f.profile.to_string_lossy().into_owned(),
            "--input".into(),
            path.to_string_lossy().into_owned(),
            "--max-input-bytes".into(),
            bytes.len().to_string(),
        ]);
        super::super::writer_cli::run(&args)
    }

    fn pending_state(f: &Fixture) -> (i64, i64, i64, String) {
        let conn = open_db(&f.db).unwrap();
        let original: String = conn
            .query_row(
                "SELECT canonical_envelope FROM s_input_messages WHERE message_id='message-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        (
            meta_i64(&conn, "generation").unwrap(),
            count(&conn, "s_clock_events"),
            count(&conn, "s_input_messages"),
            original,
        )
    }

    #[test]
    fn one_shot_conflicting_ingest_does_not_publish_prior_pending() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        let original = Fixture::ingest(1);
        assert!(d
            .handle(original.clone())
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(d);
        let before = pending_state(&f);
        let mut conflict = original;
        conflict["payload"]["raw_input"]["events"][0]["raw_text"] = json!("changed-source");
        conflict["payload_hash"] =
            json!(sha256_hex(canonical_json(&conflict["payload"]).as_bytes()));
        assert!(one_shot(&f, &conflict, "1")
            .unwrap_err()
            .contains("IdentityConflict"));
        assert_eq!(pending_state(&f), before);
    }

    #[test]
    fn one_shot_conflicting_receipt_hash_does_not_publish_prior_pending() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        let original = Fixture::ingest(1);
        assert!(d
            .handle(original.clone())
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(d);
        let before = pending_state(&f);
        let mut receipt = receipt_request(&original);
        receipt["payload"]["original_payload_hash"] = json!("different-original-hash");
        receipt["payload_hash"] = json!(sha256_hex(canonical_json(&receipt["payload"]).as_bytes()));
        assert!(one_shot(&f, &receipt, "1")
            .unwrap_err()
            .contains("IdentityConflict"));
        assert_eq!(pending_state(&f), before);
    }

    #[test]
    fn recovered_epoch_two_uses_frozen_profile_volume_unit_for_new_ingest() {
        let f = Fixture::new();
        let mut first = f.open("1").unwrap();
        assert_eq!(
            first.handle(Fixture::ingest(1)).unwrap()["kind"],
            "Committed"
        );
        drop(first);
        recover(&f.db, "2", &f.clock, "recover-1").unwrap();
        let mut invalid = Fixture::ingest(2);
        invalid["payload"]["writer_epoch"] = json!("2");
        invalid["payload"]["raw_input"]["events"][0]["volume"] = json!("2");
        invalid["payload_hash"] = json!(sha256_hex(canonical_json(&invalid["payload"]).as_bytes()));
        assert!(one_shot(&f, &invalid, "2")
            .unwrap_err()
            .contains("volume_unit"));
        let mut valid = Fixture::ingest(2);
        valid["payload"]["writer_epoch"] = json!("2");
        valid["payload_hash"] = json!(sha256_hex(canonical_json(&valid["payload"]).as_bytes()));
        assert_eq!(
            one_shot(&f, &valid, "2").unwrap().unwrap()["kind"],
            "Committed"
        );
        let conn = open_db(&f.db).unwrap();
        assert_eq!(meta_i64(&conn, "generation").unwrap(), 2);
        assert_eq!(count(&conn, "raw_events"), 2);
    }

    #[test]
    fn old_tick_schema_cannot_enter_frozen_ohlc_s3_domain() {
        let f = Fixture::new();
        let mut request = Fixture::ingest(1);
        request["payload"]["raw_input"] = json!({
            "schema_revision":"1", "session_id":"s-session-testonly-001",
            "source_namespace":"testonly.ohlc", "source_epoch":"1",
            "instrument":"TEST.OHLC", "profile":"testonly_tick_1_1_ohlc",
            "events":[{"event_id":"a","revision":"1","seq":"0",
                "received_at":"2026-09-13T00:00:00Z","raw_text":"7",
                "price":"7","timestamp":"0","volume":"1"}]
        });
        request["payload_hash"] = json!(sha256_hex(canonical_json(&request["payload"]).as_bytes()));
        let mut direct = f.open("1").unwrap();
        assert!(direct
            .handle(request.clone())
            .unwrap_err()
            .contains("profile"));
        drop(direct);
        assert!(one_shot(&f, &request, "1").is_err());
        let conn = open_db(&f.db).unwrap();
        assert_eq!(count(&conn, "raw_events"), 0);
        assert_eq!(count(&conn, "s_input_messages"), 0);
        assert_eq!(count(&conn, "s_clock_events"), 0);
        assert_eq!(meta_i64(&conn, "generation").unwrap(), 0);
    }

    #[test]
    fn explicit_s3_cli_advances_stored_pending_and_preserves_receipt() {
        let f = Fixture::new();
        let original = Fixture::ingest(1);
        let input = f.path.join("ingest.json");
        let bytes = canonical_json(&original);
        std::fs::write(&input, &bytes).unwrap();
        let mut args = cli(&f, "advance", "--writer-epoch", "1");
        args.extend([
            "--profile".into(),
            f.profile.to_string_lossy().into_owned(),
            "--input".into(),
            input.to_string_lossy().into_owned(),
            "--max-input-bytes".into(),
            bytes.len().to_string(),
        ]);
        assert!(super::super::dispatch(&args).unwrap());
        let receipt = receipt_request(&original);
        let receipt_file = f.path.join("receipt.json");
        let receipt_bytes = canonical_json(&receipt);
        std::fs::write(&receipt_file, &receipt_bytes).unwrap();
        let mut receipt_args = cli(&f, "advance", "--writer-epoch", "1");
        receipt_args.extend([
            "--profile".into(),
            f.profile.to_string_lossy().into_owned(),
            "--input".into(),
            receipt_file.to_string_lossy().into_owned(),
            "--max-input-bytes".into(),
            receipt_bytes.len().to_string(),
        ]);
        let received = super::super::writer_cli::run(&receipt_args)
            .unwrap()
            .unwrap();
        assert_eq!(received["kind"], "Committed");
        assert_eq!(received["original_message_id"], original["message_id"]);
        let reopened = f.open("1").unwrap();
        assert_eq!(meta_i64(&reopened.conn, "generation").unwrap(), 1);
        assert_eq!(count(&reopened.conn, "s_input_messages"), 1);
        assert_eq!(count(&reopened.conn, "raw_events"), 1);
        let saved: String = reopened
            .conn
            .query_row(
                "SELECT canonical_envelope FROM s_input_messages WHERE message_id='message-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(saved, bytes);
    }

    #[test]
    fn invalid_one_shot_input_does_not_resume_existing_pending() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        assert!(d
            .handle(Fixture::ingest(1))
            .unwrap_err()
            .contains("stop_after_accept"));
        drop(d);
        let mut bad_hash = Fixture::ingest(2);
        bad_hash["payload_hash"] = json!("bad");
        let bad_ready = envelope("bad-ready", json!({"op":"ready","extra":"x"}));
        let bad_receipt = envelope(
            "bad-receipt",
            json!({"op":"receipt","original_message_id":"message-1"}),
        );
        let mut wrong_session = envelope("wrong-session", json!({"op":"ready"}));
        wrong_session["session_id"] = json!("other-session");
        let mut wrong_clock = Fixture::ingest(2);
        wrong_clock["payload"]["clock_event_id"] = json!("missing-clock-event");
        wrong_clock["payload_hash"] = json!(sha256_hex(
            canonical_json(&wrong_clock["payload"]).as_bytes()
        ));
        let input = f.path.join("invalid.json");
        for invalid in [bad_hash, bad_ready, bad_receipt, wrong_session, wrong_clock] {
            let bytes = canonical_json(&invalid);
            std::fs::write(&input, &bytes).unwrap();
            let mut args = cli(&f, "advance", "--writer-epoch", "1");
            args.extend([
                "--profile".into(),
                f.profile.to_string_lossy().into_owned(),
                "--input".into(),
                input.to_string_lossy().into_owned(),
                "--max-input-bytes".into(),
                bytes.len().to_string(),
            ]);
            assert!(super::super::dispatch(&args).is_err());
        }
        let valid_bytes = canonical_json(&Fixture::ingest(2));
        std::fs::write(&input, &valid_bytes).unwrap();
        let mut too_small = cli(&f, "advance", "--writer-epoch", "1");
        too_small.extend([
            "--profile".into(),
            f.profile.to_string_lossy().into_owned(),
            "--input".into(),
            input.to_string_lossy().into_owned(),
            "--max-input-bytes".into(),
            (valid_bytes.len() - 1).to_string(),
        ]);
        assert!(super::super::dispatch(&too_small)
            .unwrap_err()
            .contains("输入字节超过"));
        let conn = open_db(&f.db).unwrap();
        assert_eq!(meta_i64(&conn, "generation").unwrap(), 0);
        assert_eq!(count(&conn, "s_input_messages"), 1);
        assert_eq!(count(&conn, "s_clock_events"), 1);
    }

    #[test]
    fn explicit_s3_cli_recover_requires_higher_epoch_then_advances_original() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        let original = Fixture::ingest(1);
        d.stop_after = Some("begin");
        assert!(d
            .handle(original.clone())
            .unwrap_err()
            .contains("stop_after_begin"));
        drop(d);
        let mut recover_args = cli(&f, "recover", "--new-epoch", "1");
        recover_args.extend(["--clock-event-id".into(), "recover-1".into()]);
        assert!(super::super::dispatch(&recover_args).is_err());
        recover_args[7] = "2".into();
        assert!(super::super::dispatch(&recover_args).unwrap());
        let mut advance_args = cli(&f, "advance", "--writer-epoch", "2");
        advance_args.extend(["--profile".into(), f.profile.to_string_lossy().into_owned()]);
        assert!(super::super::dispatch(&advance_args).unwrap());
        let mut reopened = f.open("2").unwrap();
        assert_eq!(
            reopened.handle(receipt_request(&original)).unwrap()["kind"],
            "Committed"
        );
        assert_eq!(count(&reopened.conn, "writer_epoch_history"), 1);
        assert_eq!(meta_i64(&reopened.conn, "generation").unwrap(), 1);
    }

    #[test]
    fn malformed_s3_cli_refuses_before_creating_database_or_writer_lock() {
        let dir =
            std::env::temp_dir().join(format!("s1394-cli-{}-{}", std::process::id(), now_nanos()));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("missing.sqlite");
        let args = vec![
            "s_structure_session".into(),
            "advance".into(),
            "--protocol-revision".into(),
            PROTOCOL_REVISION.into(),
            "--db".into(),
            db.to_string_lossy().into_owned(),
            "--writer-epoch".into(),
            "1".into(),
            "--writer-epoch".into(),
            "2".into(),
        ];
        assert!(super::super::dispatch(&args).is_err());
        assert!(!db.exists());
        assert!(!PathBuf::from(format!("{}.writer.lock", db.display())).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn explicit_s3_serve_with_complete_resources_reaches_real_writer_lock_before_listen() {
        let f = Fixture::new();
        let resources = f.path.join("s-resources.json");
        let mut limits = serde_json::Map::new();
        for key in super::super::writer_cli::GATE_FIELDS {
            limits.insert(key.into(), json!("1"));
        }
        std::fs::write(
            &resources,
            canonical_json(&json!({
                "schema_revision": "s-structure-resources/1",
                "s_resources": limits,
            })),
        )
        .unwrap();
        let socket = f.path.join("s.sock");
        let _held = session_transport::process_lock(&f.db, "S").unwrap();
        let mut args = cli(&f, "serve", "--writer-epoch", "1");
        args.extend([
            "--profile".into(),
            f.profile.to_string_lossy().into_owned(),
            "--socket".into(),
            socket.to_string_lossy().into_owned(),
            "--resource-config".into(),
            resources.to_string_lossy().into_owned(),
        ]);
        let error = super::super::dispatch(&args).unwrap_err();
        assert!(
            error.contains("StaleWriter") && error.contains("独占进程锁不可得"),
            "{error}"
        );
        assert!(!socket.exists());
        let db = open_db(&f.db).unwrap();
        assert_eq!(meta_i64(&db, "generation").unwrap(), 0);
    }

    #[test]
    fn invalid_s3_serve_resources_refuse_before_writer_lock_database_write_or_listen() {
        let f = Fixture::new();
        let resources = f.path.join("invalid-s-resources.json");
        let socket = f.path.join("s.sock");
        let lock = PathBuf::from(format!("{}.writer.lock", f.db.display()));
        // init 的自有锁已释放；移除本夹具旧空锁，才能检验非法 serve 不会创建新锁。
        std::fs::remove_file(&lock).unwrap();
        assert!(!lock.exists());
        let original_db = std::fs::read(&f.db).unwrap();
        let mut limits = serde_json::Map::new();
        for key in super::super::writer_cli::GATE_FIELDS {
            limits.insert(key.into(), json!("1"));
        }
        let valid = json!({"schema_revision":"s-structure-resources/1","s_resources":limits});
        let mut missing = valid.clone();
        missing["s_resources"]
            .as_object_mut()
            .unwrap()
            .remove("audit_cache_source_bytes");
        let mut extra = valid.clone();
        extra["s_resources"]["job_limits"] = json!("1");
        let mut zero = valid.clone();
        zero["s_resources"]["audit_cache_source_bytes"] = json!("0");
        let mut numeric = valid.clone();
        numeric["s_resources"]["queue_capacity"] = json!(1);
        let mut wrong_version = valid.clone();
        wrong_version["schema_revision"] = json!("s-structure-resources/2");
        let mut args = cli(&f, "serve", "--writer-epoch", "1");
        args.extend([
            "--profile".into(),
            f.profile.to_string_lossy().into_owned(),
            "--socket".into(),
            socket.to_string_lossy().into_owned(),
            "--resource-config".into(),
            resources.to_string_lossy().into_owned(),
        ]);
        for invalid in [missing, extra, zero, numeric, wrong_version] {
            std::fs::write(&resources, canonical_json(&invalid)).unwrap();
            let error = super::super::dispatch(&args).unwrap_err();
            assert!(error.contains("S3WriterResourcesInvalid"), "{error}");
            assert!(!lock.exists());
            assert!(!socket.exists());
            assert_eq!(std::fs::read(&f.db).unwrap(), original_db);
        }
        let error = Driver::open_for_serve(&f.db, &f.profile, "1", f.clock.clone(), 0)
            .err()
            .unwrap();
        assert!(error.contains("audit_cache_source_bytes"), "{error}");
        assert!(!lock.exists());
        assert_eq!(std::fs::read(&f.db).unwrap(), original_db);
    }

    #[test]
    fn s3_serve_driver_rejects_unimplemented_condition_ops_before_admission() {
        let f = Fixture::new();
        let mut driver =
            Driver::open_for_serve(&f.db, &f.profile, "1", f.clock.clone(), 1).unwrap();
        for op in ["condition", "execute_conditional", "recover"] {
            let error = driver.handle(envelope(op, json!({"op":op}))).unwrap_err();
            assert!(error.contains("未知S3 op"), "{error}");
        }
        assert_eq!(count(&driver.conn, "raw_events"), 0);
        assert_eq!(count(&driver.conn, "s_input_messages"), 0);
        assert_eq!(meta_i64(&driver.conn, "generation").unwrap(), 0);
    }

    #[test]
    fn message_driver_publishes_and_returns_original_receipt_on_duplicate() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        let ready = d.handle(envelope("ready", json!({"op":"ready"}))).unwrap();
        assert_eq!(ready["schema_revision"], "s-session/3");
        assert!(f.open("1").is_err(), "原独占进程锁仍生效");
        let e = Fixture::ingest(1);
        let result = d.handle(e.clone()).unwrap();
        assert_eq!(result["kind"], "Committed");
        assert!(count(&d.conn, "s_content_objects") > 0);
        assert_eq!(d.handle(e.clone()).unwrap(), result);
        assert_eq!(d.handle(receipt_request(&e)).unwrap(), result);
        assert_eq!(count(&d.conn, "s_clock_events"), 3);
        let mut changed = e.clone();
        changed["payload"]["raw_input"]["events"][0]["high"] = json!("13");
        changed["payload_hash"] = json!(sha256_hex(canonical_json(&changed["payload"]).as_bytes()));
        assert!(d.handle(changed).is_err());
        let mut old = e;
        old["schema_revision"] = json!("s-session/2");
        assert!(d.handle(old).is_err());
        assert_eq!(count(&d.conn, "raw_events"), 1);
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 1);
    }

    #[test]
    fn restart_after_accept_resumes_the_stored_original_without_reaccepting() {
        let f = Fixture::new();
        let e = Fixture::ingest(1);
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        assert!(d
            .handle(e.clone())
            .unwrap_err()
            .contains("stop_after_accept"));
        assert_eq!(count(&d.conn, "s_clock_events"), 1);
        drop(d);
        let mut restarted = f.open("1").unwrap();
        assert_eq!(
            restarted.handle(receipt_request(&e)).unwrap()["kind"],
            "Committed"
        );
        assert_eq!(count(&restarted.conn, "raw_events"), 1);
        assert_eq!(count(&restarted.conn, "s_clock_events"), 3);
    }

    #[test]
    fn pending_redelivery_with_same_identity_uses_stored_original_headers() {
        let f = Fixture::new();
        let original = Fixture::ingest(1);
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        assert!(d
            .handle(original.clone())
            .unwrap_err()
            .contains("stop_after_accept"));
        d.stop_after = None;
        let mut retried = original.clone();
        retried["producer_id"] = json!("restarted-producer");
        assert_eq!(d.handle(retried).unwrap()["kind"], "Committed");
        let saved: String = d
            .conn
            .query_row(
                "SELECT canonical_envelope FROM s_input_messages WHERE message_id='message-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(saved, canonical_json(&original));
        assert_eq!(count(&d.conn, "raw_events"), 1);
        assert_eq!(count(&d.conn, "s_clock_events"), 3);
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 1);
    }

    #[test]
    fn fresh_message_cannot_overtake_an_accepted_unpublished_message() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        assert!(d
            .handle(Fixture::ingest(1))
            .unwrap_err()
            .contains("stop_after_accept"));
        d.stop_after = None;
        assert!(d
            .handle(Fixture::ingest(2))
            .unwrap_err()
            .contains("MissingDependency"));
        assert_eq!(count(&d.conn, "raw_events"), 1);
        assert_eq!(count(&d.conn, "s_input_messages"), 1);
        assert_eq!(count(&d.conn, "s_clock_events"), 1);
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 0);
        assert_eq!(d.handle(Fixture::ingest(1)).unwrap()["kind"], "Committed");
        assert_eq!(d.handle(Fixture::ingest(2)).unwrap()["kind"], "Committed");
        assert_eq!(count(&d.conn, "raw_events"), 2);
        assert_eq!(count(&d.conn, "s_clock_events"), 6);
        assert_eq!(meta_i64(&d.conn, "generation").unwrap(), 2);
        let tx = d.conn.transaction().unwrap();
        assert_eq!(state::audit(&tx).unwrap().cuts.len(), 2);
    }

    #[test]
    fn opening_multiple_pending_originals_refuses_without_starting_a_batch() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("accept");
        assert!(d
            .handle(Fixture::ingest(1))
            .unwrap_err()
            .contains("stop_after_accept"));
        // 模拟内部旧调用留下的两条完整接纳原件，不绕过各条接纳/clock的核验。
        let tx = d.conn.transaction().unwrap();
        let e = Fixture::ingest(2);
        let control = control::S3Control::attach(&tx, &f.clock).unwrap();
        let view = control.inspect_ingest(e.clone()).unwrap();
        let input = tb02a::parse_input(&e["payload"]["raw_input"]).unwrap();
        let accepted = raw_input::accept_events(&tx, &input, Some("1")).unwrap();
        control.accept(&view, &accepted).unwrap();
        tx.commit().unwrap();
        drop(d);
        assert!(f.open("1").err().unwrap().contains("多条待提交原件"));
        let c = open_db(&f.db).unwrap();
        assert_eq!(count(&c, "raw_events"), 2);
        assert_eq!(count(&c, "s_clock_events"), 2);
        assert_eq!(count(&c, "s_input_messages"), 2);
        assert_eq!(meta_i64(&c, "generation").unwrap(), 0);
        assert_eq!(
            meta_get_opt(&c, "advance_state").unwrap().as_deref(),
            Some("idle")
        );
        assert_eq!(count(&c, "s_content_publications"), 0);
    }

    /// 显式诊断：冻结9011来源的有界前缀，只测内部驱动，不产生服务资源默认值。
    /// 保留数据库/clock/逐代测量；目录分配是提交后采样，不能冒充过程峰值。
    #[test]
    #[ignore = "#1394 显式S3成本测量，设置S3_CONTENT_MEASURE_PREFIX"]
    fn testonly_s3_prefix_measurement() {
        use std::os::unix::fs::MetadataExt;
        use std::time::Instant;
        let prefix: usize = std::env::var("S3_CONTENT_MEASURE_PREFIX")
            .expect("显式前缀")
            .parse()
            .unwrap();
        const SOURCE: &str = "/Users/silencehan/Projects/NewChanlun-1394-tb03b/.scratch/issue1394-long-source-search/runs/RW05_u57_l12_40_a400_2000_r25_70_d2.bars.json";
        const HASH: &str = "76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087";
        let raw = std::fs::read(SOURCE).unwrap();
        assert_eq!(sha256_hex(&raw), HASH);
        let bars: Vec<Value> = serde_json::from_slice(&raw).unwrap();
        assert_eq!(bars.len(), 9011);
        assert!((1..=bars.len()).contains(&prefix));
        let events: serde_json::Map<String, Value> = (1..=prefix).map(|n| {
            let ns = 100 + 3 * n;
            (format!("input-{n}"), json!({"accept_ns":ns.to_string(),"attempts":[{"begin_ns":(ns+1).to_string(),"commit_ns":(ns+2).to_string()}]}))
        }).collect();
        let mut f = Fixture::with_clock(
            json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"s3-frozen-prefix-cost","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":events}),
        );
        f.preserve = true;
        let mut d = f.open("1").unwrap();
        let mut measurements = Vec::new();
        println!(
            "S3_MEASURE_START {}",
            json!({"directory":f.path,"source_sha256":HASH,"prefix":prefix,"clock_hash":f.clock.hash})
        );
        for (i, bar) in bars[..prefix].iter().enumerate() {
            let n = i + 1;
            let event = json!({"event_id":format!("long-raw-{i}"),"revision":"1","seq":bar["source_index"].to_string(),
                "received_at":"2026-09-14T00:00:00Z","raw_text":canonical_json(bar),
                "open":bar["open"].to_string(),"high":bar["high"].to_string(),"low":bar["low"].to_string(),"close":bar["close"].to_string(),
                "timestamp":bar["timestamp"].to_string(),"volume":"1"});
            let message = envelope(
                &format!("message-{n}"),
                json!({"op":"ingest","target_session_id":"s-session-testonly-001","target_session_generation":"1","writer_epoch":"1","clock_event_id":format!("input-{n}"),
                "raw_input":{"schema_revision":"s-ohlc/1","session_id":"s-session-testonly-001","source_namespace":"testonly.ohlc","source_epoch":"1","instrument":"TEST.LONG.TB03B","profile":tb02a::PROFILE,"events":[event]}}),
            );
            let message_hash = sha256_hex(canonical_json(&message).as_bytes());
            let started = Instant::now();
            let result = d.handle(message).unwrap();
            let elapsed = started.elapsed();
            assert_eq!(result["kind"], "Committed");
            let (nodes, bodies): (i64, i64) = d
                .conn
                .query_row(
                    "SELECT count(*),COALESCE(sum(length(body)),0) FROM s_content_nodes",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            let allocated: u64 = std::fs::read_dir(&f.path)
                .unwrap()
                .map(|p| p.unwrap().metadata().unwrap().blocks() * 512)
                .sum();
            let row = json!({"generation":n,"elapsed_ns":elapsed.as_nanos().to_string(),"stored_nodes":nodes,"stored_framed_bytes":(bodies as u128+nodes as u128*(newchan_rust::s_content::DOMAIN_SEPARATOR.len()+9) as u128).to_string(),"directory_allocated_after_commit":allocated,"input_envelope_sha256":message_hash});
            println!("S3_MEASURE_ROW {row}");
            measurements.push(row);
            std::fs::write(f.path.join("measurements.json"), canonical_json(&json!({"source_sha256":HASH,"prefix":prefix,"rows":measurements,"resource_profile":null,"server_started":false}))).unwrap();
        }
        drop(d);
        let mut c = open_db(&f.db).unwrap();
        c.pragma_update(None, "query_only", true).unwrap();
        let started = Instant::now();
        let tx = c.transaction().unwrap();
        assert_eq!(state::audit(&tx).unwrap().cuts.len(), prefix);
        let result = json!({"directory":f.path,"source_sha256":HASH,"prefix":prefix,"rows":measurements,"cold_audit_ns":started.elapsed().as_nanos().to_string(),"clock_hash":f.clock.hash,
            "resource_profile":null,"server_started":false,"scope":"internal S3 frozen-source prefix; directory allocation sampled after commit includes diagnostic files; not allocation peak, job limits or full9011 acceptance"});
        std::fs::write(f.path.join("measurements.json"), canonical_json(&result)).unwrap();
        println!("S3_MEASURE_DONE {result}");
    }

    #[test]
    fn recovered_writer_finishes_old_pending_bytes_and_preserves_earlier_cut() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.handle(Fixture::ingest(1)).unwrap();
        let first_root: Vec<u8> = d
            .conn
            .query_row(
                "SELECT snapshot_root FROM s_content_publications WHERE generation=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let e = Fixture::ingest(2);
        d.stop_after = Some("begin");
        assert!(d
            .handle(e.clone())
            .unwrap_err()
            .contains("stop_after_begin"));
        let original: String = d
            .conn
            .query_row(
                "SELECT canonical_envelope FROM s_input_messages WHERE message_id='message-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        drop(d);
        assert!(f.open("1").err().unwrap().contains("未完成Begin"));
        assert!(recover(&f.db, "1", &f.clock, "recover-2").is_err());
        let recovered = recover(&f.db, "2", &f.clock, "recover-2").unwrap();
        assert_eq!(recovered["recovered_begin"]["resolved_to"], "idle");
        let mut restarted = f.open("2").unwrap();
        assert_eq!(
            restarted.handle(receipt_request(&e)).unwrap()["kind"],
            "Committed"
        );
        assert!(
            restarted.handle(e).unwrap_err().contains("StaleWriter"),
            "新的外部旧epoch仍拒绝"
        );
        let saved: String = restarted
            .conn
            .query_row(
                "SELECT canonical_envelope FROM s_input_messages WHERE message_id='message-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(saved, original);
        assert_eq!(
            strict_json(saved.as_bytes()).unwrap()["payload"]["writer_epoch"],
            "1"
        );
        assert_eq!(count(&restarted.conn, "raw_events"), 2);
        assert_eq!(count(&restarted.conn, "writer_epoch_history"), 1);
        assert_eq!(count(&restarted.conn, "s_clock_events"), 8);
        let tx = restarted.conn.transaction().unwrap();
        let audited = state::audit(&tx).unwrap();
        assert_eq!(audited.cuts.len(), 2);
        assert_eq!(audited.cuts[0].metadata.writer_epoch, "1");
        assert_eq!(audited.cuts[1].metadata.writer_epoch, "2");
        assert_eq!(audited.cuts[1].metadata.source_commit.attempt_index, 1);
        assert_eq!(
            audited.cuts[0].snapshot.digest().unwrap().as_slice(),
            first_root
        );
        assert_eq!(audited.cuts[1].metadata.semantic_commit_ns, "25");
    }

    #[test]
    fn damaged_state_cannot_be_cleared_by_supervisor_recovery() {
        let f = Fixture::new();
        let mut d = f.open("1").unwrap();
        d.stop_after = Some("begin");
        assert!(d.handle(Fixture::ingest(1)).is_err());
        drop(d);
        let c = open_db(&f.db).unwrap();
        c.execute(
            "UPDATE catalog SET title='corrupt' WHERE catalog_id='CC-005'",
            [],
        )
        .unwrap();
        assert!(recover(&f.db, "2", &f.clock, "recover-1").is_err());
        assert_eq!(meta_get_opt(&c, "writer_epoch").unwrap().unwrap(), "1");
        assert!(meta_get_opt(&c, "advance_state")
            .unwrap()
            .unwrap()
            .starts_with("begun:"));
        assert_eq!(count(&c, "writer_epoch_history"), 0);
        assert_eq!(count(&c, "s_clock_events"), 2);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix() {
        let f = Fixture::new();
        let o = Fixture::new();
        let c = Fixture::new();
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        native.runtime.enable_staging_for_native_test().unwrap();
        let mut control = native_driver(&c);
        control.runtime.enable_staging_for_native_test().unwrap();
        let ready = envelope("stage-establish", json!({"op":"ready"}));
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready).unwrap()
        );
        for n in 1..=3 {
            let input = Fixture::ingest(n);
            let expected = ordinary.handle(input.clone()).unwrap();
            assert_eq!(control.handle(input.clone()).unwrap(), expected);
            let probe = native.conn.sql_read_probe();
            let memory = super::super::audit_owner::allocation_probe::Window::start();
            let (actual, events, work) = state::traced(true, || native.handle(input));
            let memory = memory.finish();
            drop(probe);
            assert_eq!(actual.unwrap(), expected);
            for phase in [
                "history_decode_manifest",
                "generated_sources",
                "catalog_cut",
            ] {
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| e["phase"] == phase)
                        .map(|e| e["generation"].as_i64().unwrap())
                        .collect::<Vec<_>>(),
                    [n]
                );
            }
            assert!(
                !events
                    .iter()
                    .any(|e| e["phase"] == "owner_retained_full_scan"),
                "warm staged Publish 不能全量重算保留费：{events:?}"
            );
            native.conn.assert_complete_matches_cold();
            // staged 原语义只刷可达新节点；以独立逐次 cold 的 staged 臂比较全部节点原字节，
            // ordinary eager 臂比较全部非节点持久原件、公开根与回执。
            assert_eq!(
                prepared::tests::runtime_root_tables(&native.conn, true),
                prepared::tests::runtime_root_tables(&control.conn, true)
            );
            assert_eq!(
                prepared::tests::runtime_root_tables(&native.conn, false),
                prepared::tests::runtime_root_tables(&ordinary.conn, false)
            );
            eprintln!(
                "{}",
                json!({"scenario":"r4-staged-normal-append","generation":n,"allocation":memory,"events":events,"work":work,"staging":native.runtime.staging_report_for_native_test(),"retention":native.conn.complete_report()})
            );
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn audit_owner_incremental_normal_writes_full_oracle_and_suffix() {
        use super::super::audit_owner::allocation_probe::Window;
        fn context(d: &mut Driver) -> request_audit::RequestAuditContext {
            let total = d.audit_cache_source_bytes.unwrap_or(536_870_912);
            let keeper = d
                .conn
                .take_source_keeper()
                .unwrap_or_else(|| super::super::cross_request_source_cache::Keeper::new(total));
            // #1488 P1-1：入口旧费走生产同一逐项分区（O+uncovered）。
            let old = d.conn.old_pool_partition(Some(&keeper)).unwrap();
            let mut c = request_audit::RequestAuditContext::with_source_keeper(keeper);
            c.freeze_manifest_workspace_partition(total, crate::cache::retained_bytes(), old)
                .unwrap();
            c
        }
        fn no_retained_scan(events: &[Value]) {
            assert!(
                !events
                    .iter()
                    .any(|e| e["phase"] == "owner_retained_full_scan"),
                "warm owner 不能全量重算历史保留费：{events:?}"
            );
        }
        fn same(n: &Driver, o: &Driver) {
            n.conn.assert_complete_matches_cold();
            assert_eq!(
                prepared::tests::runtime_root_tables(&n.conn, true),
                prepared::tests::runtime_root_tables(&o.conn, true)
            );
        }
        fn request(n: &mut Driver, o: &mut Driver, e: Value, new: Option<i64>) -> Value {
            let expected = o.handle(e.clone()).unwrap();
            let probe = n.conn.sql_read_probe();
            let frames = newchan_rust::s_content::frame_calls_on_thread();
            let frame_bytes = newchan_rust::s_content::frame_bytes_on_thread();
            let hashes = newchan_rust::session_protocol::hash_work_on_thread();
            let memory = Window::start();
            let (actual, events, work) = state::traced(true, || n.handle(e.clone()));
            let memory = memory.finish();
            let after = newchan_rust::session_protocol::hash_work_on_thread();
            drop(probe);
            assert_eq!(actual.as_ref().unwrap(), &expected);
            for phase in [
                "history_decode_manifest",
                "generated_sources",
                "catalog_cut",
            ] {
                let generations: Vec<_> = events
                    .iter()
                    .filter(|event| event["phase"] == phase)
                    .map(|event| event["generation"].as_i64().unwrap())
                    .collect();
                assert_eq!(
                    generations,
                    new.into_iter().collect::<Vec<_>>(),
                    "{phase}: {events:?}"
                );
            }
            assert!(
                n.conn.complete_retained_bytes() > 0,
                "正常包不得仅因可选保留被丢弃而绕过后缀锁"
            );
            if new.is_some() {
                assert_eq!(work["raw_payload_hashes"], 1);
                assert_eq!(work["raw_source_hashes"], 4);
            } else {
                assert_eq!(work["raw_payload_hashes"], 0);
                assert_eq!(work["raw_source_hashes"], 0);
            }
            eprintln!(
                "{}",
                json!({"scenario":"r4-normal-owner-request","message":e["message_id"],"expected_new_cut":new,"allocation":memory,"events":events,"work":work,
                "sql_classes":["raw_and_ohlc","control","other_nonnode","nodes"],"protocol_sha256_calls":after.0-hashes.0,"protocol_sha256_bytes":after.1-hashes.1,
                "content_frame_hashes":newchan_rust::s_content::frame_calls_on_thread()-frames,
                "content_frame_sha256_bytes":newchan_rust::s_content::frame_bytes_on_thread()-frame_bytes,"retention":n.conn.complete_report()})
            );
            no_retained_scan(&events);
            same(n, o);
            actual.unwrap()
        }
        let base = Fixture::new();
        let mut plan = base.clock.value.clone();
        for n in 4..=5 {
            plan["events"][format!("input-{n}")] = json!({"accept_ns":(n*10).to_string(),"attempts":[{"begin_ns":(n*10+1).to_string(),"commit_ns":(n*10+2).to_string()}]});
        }
        let f = Fixture::with_clock(plan.clone());
        let o = Fixture::with_clock(plan);
        let mut native = complete_driver(&f);
        let mut ordinary = o.open("1").unwrap();
        let ready = envelope("establish-q", json!({"op":"ready"}));
        assert_eq!(
            native.handle(ready.clone()).unwrap(),
            ordinary.handle(ready).unwrap()
        );
        same(&native, &ordinary);
        request(&mut native, &mut ordinary, Fixture::ingest(1), Some(1));
        let second = Fixture::ingest(2);
        // 使用固定 API 正常终结 Accept 与 Begin；没有 stop_after、异常、坏库或故障注入。
        let mut c = context(&mut native);
        let mut oc = request_audit::RequestAuditContext::default();
        let probe = native.conn.sql_read_probe();
        let memory = Window::start();
        let hashes = newchan_rust::session_protocol::hash_work_on_thread();
        let (accepted, events, work) = state::traced(true, || {
            native.conn.admit(
                second.clone(),
                &native.epoch,
                &native.clock,
                &native.profile_id,
                native.volume_unit.as_deref(),
                &native.mode,
                &mut c,
            )
        });
        drop(probe);
        let memory = memory.finish();
        let after = newchan_rust::session_protocol::hash_work_on_thread();
        let (a, end) = accepted.unwrap();
        assert!(matches!(
            a,
            super::super::audit_owner::Admission::Advance { accepted: true, .. }
        ));
        native.conn.finish_source_keeper(c, end);
        let (_, end) = ordinary
            .conn
            .admit(
                second.clone(),
                &ordinary.epoch,
                &ordinary.clock,
                &ordinary.profile_id,
                ordinary.volume_unit.as_deref(),
                &ordinary.mode,
                &mut oc,
            )
            .unwrap();
        ordinary.conn.finish_source_keeper(oc, end);
        assert_eq!(native.conn.complete_report()["raw_checked"], 2);
        assert_eq!(native.conn.complete_report()["published_prefix"], 1);
        assert!(!events
            .iter()
            .any(|e| e["phase"] == "history_decode_manifest"));
        assert_eq!(work["raw_payload_hashes"], 1);
        assert_eq!(work["raw_source_hashes"], 0);
        eprintln!(
            "{}",
            json!({"scenario":"r4-accept-R-greater-than-F","allocation":memory,"protocol_sha256_calls":after.0-hashes.0,"protocol_sha256_bytes":after.1-hashes.1,"events":events,"work":work,"retention":native.conn.complete_report()})
        );
        no_retained_scan(&events);
        same(&native, &ordinary);
        let mut c = context(&mut native);
        let mut oc = request_audit::RequestAuditContext::default();
        let probe = native.conn.sql_read_probe();
        let memory = Window::start();
        let hashes = newchan_rust::session_protocol::hash_work_on_thread();
        let (b, events, work) = state::traced(true, || {
            native
                .conn
                .begin_advance(&second, &native.epoch, &native.clock, &native.mode, &mut c)
        });
        drop(probe);
        let memory = memory.finish();
        let after = newchan_rust::session_protocol::hash_work_on_thread();
        drop(b.unwrap());
        drop(c);
        drop(
            ordinary
                .conn
                .begin_advance(
                    &second,
                    &ordinary.epoch,
                    &ordinary.clock,
                    &ordinary.mode,
                    &mut oc,
                )
                .unwrap(),
        );
        assert!(!events
            .iter()
            .any(|e| e["phase"] == "history_decode_manifest"));
        eprintln!(
            "{}",
            json!({"scenario":"r4-normal-begin","allocation":memory,"protocol_sha256_calls":after.0-hashes.0,"protocol_sha256_bytes":after.1-hashes.1,"events":events,"work":work})
        );
        no_retained_scan(&events);
        same(&native, &ordinary);
        let probe = native.conn.sql_read_probe();
        let memory = Window::start();
        let hashes = newchan_rust::session_protocol::hash_work_on_thread();
        let (recovered, events, work) = state::traced(true, || {
            native.conn.recover("2", &native.clock, "recover-2")
        });
        drop(probe);
        let memory = memory.finish();
        let after = newchan_rust::session_protocol::hash_work_on_thread();
        assert_eq!(
            recovered.unwrap(),
            ordinary
                .conn
                .recover("2", &ordinary.clock, "recover-2")
                .unwrap()
        );
        native.epoch = "2".into();
        ordinary.epoch = "2".into();
        assert!(!events
            .iter()
            .any(|e| e["phase"] == "history_decode_manifest"));
        eprintln!(
            "{}",
            json!({"scenario":"r4-normal-recover","allocation":memory,"protocol_sha256_calls":after.0-hashes.0,"protocol_sha256_bytes":after.1-hashes.1,"events":events,"work":work,"retention":native.conn.complete_report()})
        );
        no_retained_scan(&events);
        same(&native, &ordinary);
        let mut c = context(&mut native);
        let mut oc = request_audit::RequestAuditContext::default();
        let probe = native.conn.sql_read_probe();
        let memory = Window::start();
        let hashes = newchan_rust::session_protocol::hash_work_on_thread();
        let (advanced, events, work) = state::traced(true, || native.advance(&second, &mut c));
        drop(probe);
        let memory = memory.finish();
        let after = newchan_rust::session_protocol::hash_work_on_thread();
        let (reply, end) = advanced.unwrap();
        native.conn.finish_source_keeper(c, end);
        let (expected, end) = ordinary.advance(&second, &mut oc).unwrap();
        ordinary.conn.finish_source_keeper(oc, end);
        assert_eq!(reply, expected);
        for phase in [
            "history_decode_manifest",
            "generated_sources",
            "catalog_cut",
        ] {
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e["phase"] == phase)
                    .map(|e| e["generation"].as_i64().unwrap())
                    .collect::<Vec<_>>(),
                [2]
            );
        }
        assert_eq!(work["raw_payload_hashes"], 0);
        assert_eq!(work["raw_source_hashes"], 4);
        eprintln!(
            "{}",
            json!({"scenario":"r4-retry-original-envelope","allocation":memory,"protocol_sha256_calls":after.0-hashes.0,"protocol_sha256_bytes":after.1-hashes.1,"events":events,"work":work,"retention":native.conn.complete_report()})
        );
        no_retained_scan(&events);
        same(&native, &ordinary);
        let mut revision = Fixture::ingest(3);
        revision["payload"]["raw_input"] = second["payload"]["raw_input"].clone();
        revision["payload"]["raw_input"]["events"][0]["revision"] = json!("2");
        revision["payload"]["raw_input"]["events"][0]["high"] = json!("17");
        revision["payload"]["writer_epoch"] = json!("2");
        revision["payload_hash"] =
            json!(sha256_hex(canonical_json(&revision["payload"]).as_bytes()));
        request(&mut native, &mut ordinary, revision.clone(), Some(3));
        let mut replay = revision.clone();
        replay["message_id"] = json!("message-4");
        replay["payload"]["clock_event_id"] = json!("input-4");
        replay["payload_hash"] = json!(sha256_hex(canonical_json(&replay["payload"]).as_bytes()));
        request(&mut native, &mut ordinary, replay, None);
        assert_eq!(count(&native.conn, "s_content_publications"), 3);
        assert_eq!(count(&native.conn, "raw_events"), 3);
        let mut inserted = Fixture::ingest(3);
        inserted["message_id"] = json!("message-5");
        inserted["payload"]["clock_event_id"] = json!("input-5");
        inserted["payload"]["raw_input"]["events"][0]["seq"] = json!("5");
        inserted["payload"]["writer_epoch"] = json!("2");
        inserted["payload_hash"] =
            json!(sha256_hex(canonical_json(&inserted["payload"]).as_bytes()));
        request(&mut native, &mut ordinary, inserted, Some(4));
        let retained = native.conn.complete_retained_bytes();
        assert!(retained > 0);
        let memory = Window::start();
        drop(native);
        let memory = memory.finish();
        let mut reopened = Driver::open_for_serve_with_mode(
            &f.db,
            &f.profile,
            "2",
            f.clock.clone(),
            536_870_912,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        assert_eq!(reopened.conn.complete_retained_bytes(), 0);
        assert_eq!(
            reopened.handle(receipt_request(&revision)).unwrap(),
            ordinary.handle(receipt_request(&revision)).unwrap()
        );
        reopened.conn.assert_complete_matches_cold();
        eprintln!(
            "{}",
            json!({"scenario":"r4-release-reopen","released_owner_charge":retained,"allocation":memory,"new_life":reopened.conn.complete_report()["life"]})
        );
    }

    /// #1488 0065 P1 锁（v4 L13）：S7 serve 下第 2 代在 publication 尾检位置注入候选父工作域
    /// Allocation 失败（与 j.cap 日志/索引扩容超帽同一 BudgetExceeded 分类）。须：同请求放弃候选、
    /// 无候选发布成功，回复与运行根逐项等于 ordinary oracle，不保留候选；后续代照常接纳并与 oracle
    /// 一致（不得卡在“已有未完成Begin”或“Carry租约已撤销”）。负控：同位置注入 Ledger（真正尾检失败）仍拒绝且不降级。
    /// 0069：`flag` 选注入点；`rejects_with=Some(片段)` 表示须拒绝、不降级且错误含该片段。
    #[cfg(feature = "s_capture_native_testonly")]
    fn publication_candidate_shortfall_case(
        flag: &'static std::thread::LocalKey<std::cell::Cell<bool>>,
        rejects_with: Option<&str>,
    ) {
        use super::super::prepared::tests::runtime_root_tables;
        let phases =
            |events: &[Value], phase: &str| events.iter().filter(|e| e["phase"] == phase).count();
        let warm_fixture = Fixture::new();
        let ordinary_fixture = Fixture::new();
        let mut warm = Driver::open_for_serve_with_mode(
            &warm_fixture.db,
            &warm_fixture.profile,
            "1",
            warm_fixture.clock.clone(),
            536_870_912,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        let mut ordinary = ordinary_fixture.open("1").unwrap();
        assert_eq!(
            warm.handle(Fixture::ingest(1)).unwrap(),
            ordinary.handle(Fixture::ingest(1)).unwrap()
        );
        assert!(
            warm.conn.complete_retained_bytes() > 0,
            "第 2 代入口须带完整候选"
        );
        flag.with(|f| f.set(true));
        let (reply, events, _) = state::traced(false, || warm.handle(Fixture::ingest(2)));
        assert!(!flag.with(|f| f.get()), "故障必须实际到达候选尾检");
        if let Some(expected) = rejects_with {
            let error = reply.expect_err("真正尾检失败必须拒绝");
            assert!(error.contains(expected), "{error}");
            assert_eq!(phases(&events, "publication_candidate_abandoned"), 0);
            assert_eq!(meta_i64(&warm.conn, "generation").unwrap(), 1);
            return;
        }
        assert_eq!(reply.unwrap(), ordinary.handle(Fixture::ingest(2)).unwrap());
        assert_eq!(phases(&events, "publication_candidate_abandoned"), 1);
        assert_eq!(warm.conn.complete_retained_bytes(), 0, "放弃后不保留候选");
        assert_eq!(meta_i64(&warm.conn, "generation").unwrap(), 2);
        assert_eq!(
            runtime_root_tables(&warm.conn, true),
            runtime_root_tables(&ordinary.conn, true)
        );
        // 第 3 代新写 + 同输入 repost（夹具只有三条输入）。
        for n in [3, 3] {
            let (reply, events, _) = state::traced(false, || warm.handle(Fixture::ingest(n)));
            assert_eq!(reply.unwrap(), ordinary.handle(Fixture::ingest(n)).unwrap());
            assert_eq!(phases(&events, "publication_candidate_abandoned"), 0);
            assert_eq!(
                runtime_root_tables(&warm.conn, true),
                runtime_root_tables(&ordinary.conn, true)
            );
        }
        assert!(
            warm.conn.complete_retained_bytes() > 0,
            "后续代重建并保留候选"
        );
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn publication_candidate_budget_shortfall_degrades_and_keeps_admitting() {
        use super::super::audit_owner::PUBLICATION_CANDIDATE_BUDGET_FAILURE;
        publication_candidate_shortfall_case(&PUBLICATION_CANDIDATE_BUDGET_FAILURE, None);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn publication_real_tail_failure_still_rejects_without_degrade() {
        use super::super::audit_owner::PUBLICATION_TAIL_FAILURE;
        publication_candidate_shortfall_case(&PUBLICATION_TAIL_FAILURE, Some("工作账损坏"));
    }
    /// #1488 0069 锁（独审 0068 F1 负控，原探针 refs/r68-red/e）：尾检位置先记 Allocation，
    /// 闭包再返回与预算无关的真实错误。改前被降级成功（reply_ok=true abandoned=1）；须拒绝、
    /// 不降级，且拒绝原因是闭包自身的真实错误。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn publication_real_error_after_budget_failure_rejects_without_degrade() {
        use super::super::audit_owner::PUBLICATION_REAL_ERROR_AFTER_BUDGET;
        publication_candidate_shortfall_case(
            &PUBLICATION_REAL_ERROR_AFTER_BUDGET,
            Some("publication尾检身份不符（0069负控注入）"),
        );
    }
    /// #1488 0069 锁：故障起点改到 audit_native_source 追加原件处（S86 message-20 / MIN12 op-0015
    /// 的真实失败位置，b-0059 diag6），按 s_content 索引扩容超帽同一方式 reject(Allocation) 并经
    /// `invalid` 沿真实调用链上抛。收紧后的判据须仍识别为预算不足并降级（补 0068 I3：原降级锁
    /// 只覆盖尾检注入）。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn publication_source_append_allocation_failure_degrades() {
        use super::super::audit_owner::SOURCE_APPEND_ALLOCATION_FAILURE;
        publication_candidate_shortfall_case(&SOURCE_APPEND_ALLOCATION_FAILURE, None);
    }
    /// #1488 0072 锁（独审 0070 R1）：故障起点改到 prepared 发布读取租约处，真实候选父工作域
    /// Allocation 经 prepared 的 `err` 以 `S3PrepareInvalid：` 包装上抛（S86 message-34 的真实形态）。
    /// 0069 文字白名单下此处被 fail-closed；类型化判据须降级成功，后续代照常。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn publication_prepare_wrapped_allocation_failure_degrades() {
        use super::super::audit_owner::PREPARE_ALLOCATION_FAILURE;
        publication_candidate_shortfall_case(&PREPARE_ALLOCATION_FAILURE, None);
    }
    /// #1488 0072 锁（独审 0070 R2，探针 g refs/r70-red/g 取反）：尾检先记 Allocation，再返回另一
    /// 作用域 Ledger 类 scratch 错误经 `invalid` 的渲染（与预算桥文字逐字相同）。须拒绝、不降级。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn publication_ledger_after_budget_failure_rejects_without_degrade() {
        use super::super::audit_owner::PUBLICATION_LEDGER_AFTER_BUDGET;
        publication_candidate_shortfall_case(
            &PUBLICATION_LEDGER_AFTER_BUDGET,
            Some("S3DriverInvalid：prototype_capture_owned_payload_limit: codec 工作域分配失败"),
        );
    }
    /// 0065：`S_MANIFEST_WORKSPACE_DIAGNOSTICS=1` 时 serve 打开即配置 pager 探针。配置后带候选的
    /// 后续代须照常 Carry 并与 ordinary oracle 一致（改前第 2 代报“Carry租约已撤销”）。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn pager_probe_configure_keeps_carry_lease_open() {
        use super::super::prepared::tests::runtime_root_tables;
        let native_fixture = Fixture::new();
        let ordinary_fixture = Fixture::new();
        let mut native = Driver::open_for_serve_with_mode(
            &native_fixture.db,
            &native_fixture.profile,
            "1",
            native_fixture.clock.clone(),
            536_870_912,
            s_allocation::Mode::CapturedNative,
        )
        .unwrap();
        native.conn.configure_pager_probe().unwrap();
        let mut ordinary = ordinary_fixture.open("1").unwrap();
        for n in 1..=3 {
            assert_eq!(
                native.handle(Fixture::ingest(n)).unwrap(),
                ordinary.handle(Fixture::ingest(n)).unwrap()
            );
            assert!(
                native.conn.complete_retained_bytes() > 0,
                "第 {n} 代后保留候选"
            );
            assert_eq!(
                runtime_root_tables(&native.conn, true),
                runtime_root_tables(&ordinary.conn, true)
            );
        }
    }
}
