//! #1394：S3 消息到真实接纳、Begin 和发布的同步驱动。
//! 传输层须先执行资源/帧约束；CLI 由 writer_cli 接线，监听保持启动资格闭门。
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
    conn: Connection,
    runtime: prepared::Runtime,
    clock: v2::ClockPlan,
    epoch: String,
    profile_id: String,
    volume_unit: Option<String>,
    _lock: File,
    #[cfg(test)]
    stop_after: Option<&'static str>,
}

impl Driver {
    /// 服务回复只绑定经过当前持久审计的身份，不另建一份控制状态。
    pub(super) fn identity(&self) -> Result<(String, String, String), String> {
        let tx = self.conn.unchecked_transaction().map_err(err)?;
        state::audit(&tx)?;
        verify_writer_epoch(&tx, &self.epoch)?;
        let control = control::S3Control::attach(&tx, &self.clock)?;
        let (session, generation, epoch) = control.identity();
        Ok((session.into(), generation.into(), epoch.into()))
    }

    /// one-shot 返回已实际推进后的持久前沿；不把“启动成功”当作发布收据。
    pub(super) fn state_summary(&self, command: &str) -> Result<Value, String> {
        let tx = self.conn.unchecked_transaction().map_err(err)?;
        state::audit(&tx)?;
        verify_writer_epoch(&tx, &self.epoch)?;
        let control = control::S3Control::attach(&tx, &self.clock)?;
        let (session, incarnation, epoch) = control.identity();
        let pending: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM s_input_messages WHERE status='AcceptedPending'",
                [],
                |row| row.get(0),
            )
            .map_err(err)?;
        Ok(json!({
            "ok": true,
            "command": command,
            "session_id": session,
            "session_generation": incarnation,
            "writer_epoch": epoch,
            "generation": meta_get_opt(&tx, "generation")?,
            "structure_cut": meta_get_opt(&tx, "structure_cut")?,
            "index_frontier": meta_get_opt(&tx, "index_frontier")?,
            "accepted_pending": pending.to_string(),
        }))
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
        parse_writer_epoch(epoch, "configured writer_epoch")?;
        if !db.is_file() {
            return Err("StorageUnavailable：S3驱动只打开已初始化库".into());
        }
        let lock = session_transport::process_lock(db, "S")?;
        let mut conn = open_db(db)?;
        let tx = conn.transaction().map_err(err)?;
        state::audit(&tx)?;
        verify_writer_epoch(&tx, epoch)?;
        let control = control::S3Control::attach(&tx, &clock)?;
        let (profile_value, profile_id, profile_hash) = load_profile(profile)?;
        if profile_id != tb02a::PROFILE {
            return Err("VersionUnsupported：S3驱动尚未实现非OHLC生成，拒绝接纳".into());
        }
        if meta_get_opt(&tx, "profile_hash")?.as_deref() != Some(profile_hash.as_str()) {
            return Err("IdentityConflict：S3驱动profile与冻结档案不符".into());
        }
        // 与旧 accept_core 共用冻结 profile 的可选 volume_unit 口径，绝不借 writer epoch 充当成交量。
        let volume_unit = profile_value
            .get("volume_unit")
            .and_then(Value::as_str)
            .map(str::to_string);
        if meta_get_opt(&tx, "advance_state")?.as_deref() != Some("idle") {
            return Err("StaleWriter：未完成Begin须先由监督器recover".into());
        }
        if let Some(request) = request {
            check_request_before_resume(&tx, &control, request, &profile_id)?;
        }
        drop(tx);
        let mut driver = Self {
            conn,
            runtime: prepared::Runtime::new(),
            clock,
            epoch: epoch.into(),
            profile_id,
            volume_unit,
            _lock: lock,
            #[cfg(test)]
            stop_after: None,
        };
        driver.resume_pending()?;
        Ok(driver)
    }

    pub(super) fn handle(&mut self, envelope: Value) -> Result<Value, String> {
        let envelope = validate_request(envelope)?;
        let tx = self.conn.transaction().map_err(err)?;
        state::audit(&tx)?;
        verify_writer_epoch(&tx, &self.epoch)?;
        let control = control::S3Control::attach(&tx, &self.clock)?;
        let (sid, inc, _) = control.identity();
        if required(&envelope, "session_id")? != sid
            || required(&envelope, "session_generation")? != inc
        {
            return Err("IdentityConflict：请求不属于本会话/化身".into());
        }
        let payload = &envelope["payload"];
        match required(payload, "op")? {
            "ready" => v2::ready_for_protocol(&tx, PROTOCOL_REVISION),
            "receipt" => v2::lookup(
                &tx,
                required(payload, "original_source_namespace")?,
                required(payload, "original_source_epoch")?,
                required(payload, "original_message_id")?,
                required(payload, "original_payload_hash")?,
            ),
            "ingest" => {
                let input = tb02a::parse_input(&payload["raw_input"])?;
                raw_input::validate_profile_binding(&input, &self.profile_id)?;
                let view = control.inspect_ingest(envelope.clone())?;
                if view.is_committed() {
                    return Ok(view.receipt().clone());
                }
                if meta_get_opt(&tx, "advance_state")?.as_deref() != Some("idle") {
                    return Err("StaleWriter：已有未完成Begin，禁止新接纳或再次Begin".into());
                }
                let original = if view.is_fresh() {
                    let pending: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM s_input_messages WHERE status='AcceptedPending')",
                        [], |r| r.get(0),
                    ).map_err(err)?;
                    if pending {
                        return Err(
                            "MissingDependency：须先完成已接纳消息，禁止新消息越过未发布前沿"
                                .into(),
                        );
                    }
                    let clock_event = required(payload, "clock_event_id")?;
                    self.clock.phase(clock_event, "accept", -1)?;
                    self.clock.phase(clock_event, "begin", 0)?;
                    self.clock.phase(clock_event, "commit", 0)?;
                    let accepted =
                        raw_input::accept_events(&tx, &input, self.volume_unit.as_deref())?;
                    control.accept(&view, &accepted)?;
                    tx.commit().map_err(err)?;
                    #[cfg(test)]
                    if self.stop_after == Some("accept") {
                        return Err("TestOnly：stop_after_accept".into());
                    }
                    envelope
                } else {
                    // 重投准入按原有 key/hash/clock 判据；推进只用数据库原件。
                    let original = control
                        .load_pending(
                            required(&envelope, "source_namespace")?,
                            required(&envelope, "source_epoch")?,
                            required(&envelope, "message_id")?,
                        )?
                        .envelope()
                        .clone();
                    drop(tx);
                    original
                };
                self.advance(&original)
            }
            _ => Err("SchemaUnsupported：未知S3 op；recover只属于监督器".into()),
        }
    }

    fn advance(&mut self, envelope: &Value) -> Result<Value, String> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(err)?;
        let audited = state::audit(&tx)?;
        verify_writer_epoch(&tx, &self.epoch)?;
        if meta_get_opt(&tx, "advance_state")?.as_deref() != Some("idle") {
            return Err("StaleWriter：已有未完成Begin".into());
        }
        let control = control::S3Control::attach(&tx, &self.clock)?;
        let view = control.load_pending(
            required(envelope, "source_namespace")?,
            required(envelope, "source_epoch")?,
            required(envelope, "message_id")?,
        )?;
        if view.envelope() != envelope {
            return Err("IdentityConflict：推进原件与已接纳消息不同".into());
        }
        let generation = meta_i64(&tx, "generation")?
            .checked_add(1)
            .ok_or_else(|| err("代号溢出"))?;
        let frontier = i64::try_from(audited.raw.len())
            .map_err(err)?
            .checked_sub(1)
            .ok_or_else(|| err("没有原始输入"))?;
        let base = meta_get_opt(&tx, "structure_cut")?.ok_or_else(|| err("缺当前cut"))?;
        let token = control.begin(&view, generation, frontier, &base, &self.epoch)?;
        tx.commit().map_err(err)?;
        #[cfg(test)]
        if self.stop_after == Some("begin") {
            return Err("TestOnly：stop_after_begin".into());
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(err)?;
        Ok(self.runtime.commit(tx, &token, envelope)?.message_receipt)
    }

    fn resume_pending(&mut self) -> Result<(), String> {
        let originals = {
            let mut stmt = self.conn.prepare("SELECT canonical_envelope FROM s_input_messages WHERE status='AcceptedPending' ORDER BY accepted_seq,message_id").map_err(err)?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(err)?
        };
        if originals.len() > 1 {
            return Err(
                "InvalidDomain：存在多条待提交原件，不能用单条Begin合并发布；保留原库等待逐条处置"
                    .into(),
            );
        }
        for raw in originals {
            self.advance(&strict_json(raw.as_bytes())?)?;
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
    let next = parse_writer_epoch(new_epoch, "new writer_epoch")?;
    if !db.is_file() {
        return Err("StorageUnavailable：恢复需要已初始化库".into());
    }
    let _lock = session_transport::process_lock(db, "S")?;
    let mut conn = open_db(db)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(err)?;
    state::audit(&tx)?;
    let current = meta_get_opt(&tx, "writer_epoch")?.ok_or_else(|| err("缺writer_epoch"))?;
    if next <= parse_writer_epoch(&current, "current writer_epoch")? {
        return Err("StaleWriter：recover换代必须严格上升".into());
    }
    let control = control::S3Control::attach(&tx, clock)?;
    let generation = meta_get_opt(&tx, "generation")?.ok_or_else(|| err("缺generation"))?;
    let prior = meta_get_opt(&tx, "advance_state")?.ok_or_else(|| err("缺advance_state"))?;
    let transitioned_at = control.recover(clock_event, &generation)?;
    let recovered = if prior != "idle" {
        meta_set(&tx, "advance_state", "idle")?;
        meta_set(&tx, "begin_token", "")?;
        meta_set(
            &tx,
            "last_recovered_begin",
            &format!("{prior}:recovered_at_generation_{generation}"),
        )?;
        json!({"advance_state":prior,"resolved_to":"idle","generation_at_recovery":generation})
    } else {
        Value::Null
    };
    meta_set(&tx, "writer_epoch", new_epoch)?;
    tx.execute("INSERT INTO writer_epoch_history(from_epoch,to_epoch,generation_at_transition,advance_state_at_transition,transitioned_at) VALUES(?1,?2,?3,'idle',?4)",
        params![current,new_epoch,generation,transitioned_at]).map_err(err)?;
    let output = json!({"ok":true,"previous_epoch":current,"new_epoch":new_epoch,"epoch_transitioned":true,"recovered_begin":recovered,
        "reachable_root":{"generation":generation,"structure_cut":meta_get_opt(&tx,"structure_cut")?,"index_frontier":meta_get_opt(&tx,"index_frontier")?}});
    tx.commit().map_err(err)?;
    Ok(output)
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
            let row = if n == 1 {
                ("a", 0, 6, 12, 3, 7)
            } else {
                ("b", 10, 8, 16, 6, 12)
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

    #[test]
    fn cold_audit_live_driver_requests_re_read_and_reject_current_node_damage() {
        use super::super::state::cold_audit_probe as probe;
        let f = Fixture::new();
        let first = Fixture::ingest(1);
        let second = Fixture::ingest(2);
        let inputs = json!([first, second]);
        let cap = probe::begin(&inputs);
        let mut d = f.open("1").unwrap();
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
        let (result, mut phase) = probe::observe("driver_reopen_current_history", || f.open("1"));
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
    fn explicit_s3_serve_with_complete_test_resources_refuses_before_lock_or_listen() {
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
        assert!(error.contains("AwaitingPolicy"), "{error}");
        assert!(!socket.exists());
        let db = open_db(&f.db).unwrap();
        assert_eq!(meta_i64(&db, "generation").unwrap(), 0);
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
}
