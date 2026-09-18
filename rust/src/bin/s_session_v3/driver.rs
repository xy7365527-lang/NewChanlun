//! #1394：S3 消息到真实接纳、Begin 和发布的同步驱动。
//! 传输层须先执行资源/帧约束；本模块暂不注册 CLI 或监听服务。
//! 同一实例持有原进程锁；每次读取/写入都在当前事务调用同一完整状态审计。

use super::*;
use newchan_rust::session_protocol::{required, validate_envelope};
use std::fs::File;

pub(super) struct Driver {
    conn: Connection,
    runtime: prepared::Runtime,
    clock: v2::ClockPlan,
    epoch: String,
    _lock: File,
    #[cfg(test)]
    stop_after: Option<&'static str>,
}

impl Driver {
    pub(super) fn open(
        db: &Path,
        profile: &Path,
        epoch: &str,
        clock: v2::ClockPlan,
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
        let (_, profile_id, profile_hash) = load_profile(profile)?;
        if profile_id != tb02a::PROFILE {
            return Err("VersionUnsupported：S3驱动尚未实现非OHLC生成，拒绝接纳".into());
        }
        if meta_get_opt(&tx, "profile_hash")?.as_deref() != Some(profile_hash.as_str()) {
            return Err("IdentityConflict：S3驱动profile与冻结档案不符".into());
        }
        if meta_get_opt(&tx, "advance_state")?.as_deref() != Some("idle") {
            return Err("StaleWriter：未完成Begin须先由监督器recover".into());
        }
        // 借用只用于核持久身份；不以配置中的身份覆盖已核事实。
        let _ = control.identity();
        drop(tx);
        let mut driver = Self {
            conn,
            runtime: prepared::Runtime::new(),
            clock,
            epoch: epoch.into(),
            _lock: lock,
            #[cfg(test)]
            stop_after: None,
        };
        driver.resume_pending()?;
        Ok(driver)
    }

    pub(super) fn handle(&mut self, envelope: Value) -> Result<Value, String> {
        let envelope = validate_envelope(envelope, PROTOCOL_REVISION)?;
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
                    let input = tb02a::parse_input(&payload["raw_input"])?;
                    let accepted = raw_input::accept_events(&tx, &input, Some(&self.epoch))?;
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
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "s1394-driver-{}-{}-{}",
                std::process::id(),
                now_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            let clock_value = json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"s3-driver","origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":{
                "input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"},{"begin_ns":"14","commit_ns":"15"}]},
                "input-2":{"accept_ns":"20","attempts":[{"begin_ns":"21","commit_ns":"22"},{"begin_ns":"24","commit_ns":"25"}]},
                "recover-1":{"recover_ns":"13"},"recover-2":{"recover_ns":"23"}}});
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
