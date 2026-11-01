// #1481 S1：在正式 owner 准入入口对拍；旧审计只在断言预期值时作为测试 oracle。
// 本文件由 construction_tests 在 test+s_incremental+unix 下 include，生产无钩子。

struct H1AdmissionFixture {
    _anchor: prepared::tests::Fixture,
    db: PathBuf,
    conn: Connection,
    profile: PathBuf,
    clock: v2::ClockPlan,
}
impl H1AdmissionFixture {
    fn new() -> Self {
        let anchor = prepared::tests::Fixture::new();
        let db = anchor.db_path().with_file_name("admission.sqlite");
        let clock_path = db.with_extension("clock.json");
        let mut events = serde_json::Map::new();
        for n in 1..=6 {
            events.insert(
                format!("input-{n}"),
                json!({"accept_ns":(n*10).to_string(),
                "attempts":[{"begin_ns":(n*10+1).to_string(),"commit_ns":(n*10+2).to_string()},
                    {"begin_ns":(n*10+4).to_string(),"commit_ns":(n*10+5).to_string()}]}),
            );
        }
        events.insert("recover-1".into(), json!({"recover_ns":"63"}));
        let value = json!({"schema_revision":"s-clock-plan/1","clock_plan_id":"h1-admission",
            "origin_utc":"2026-09-18T00:00:00Z","unit":"ns","events":events});
        std::fs::write(&clock_path, canonical_json(&value)).unwrap();
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        let profile = base.join("profiles/ohlc_integer_tb02a_v1.json");
        init_v3(
            &db,
            "s-session-testonly-001",
            &base.join("catalog/signed-catalog.json"),
            &profile,
            "1",
            &clock_path,
            6,
        )
        .unwrap();
        let conn = open_db(&db).unwrap();
        Self {
            _anchor: anchor,
            db,
            conn,
            profile,
            clock: v2::ClockPlan::parse(value).unwrap(),
        }
    }
    fn runtime(&self) -> prepared::Runtime {
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        let rule = meta_get_opt(&self.conn, "rule_revision").unwrap().unwrap();
        runtime
            .install_audit_ledger(audit_ledger::tests::ledger(
                &self.db,
                "admission-ledger.sqlite",
                &rule,
            ))
            .unwrap();
        runtime
    }
    fn open(
        &self,
        runtime: &prepared::Runtime,
        context: &mut request_audit::RequestAuditContext,
    ) -> Result<AuditConnection, String> {
        AuditConnection::open_admitted_history(
            &self.db,
            1 << 20,
            None,
            context,
            h1_qualified_source(runtime),
        )
        .map(|(owner, _cold_elapsed)| owner)
    }
    fn full_canonical(&self) -> String {
        let tx = self.conn.unchecked_transaction().unwrap();
        // Native 完整 oracle 保留 ImmutableHistoricalAudit 的全部可选 Native 事实。
        // Ordinary audit 的语义事实没有这些可选事实，不可拿 fingerprint 冒充全规范输出。
        let before = state::full_audit_calls_for_test();
        let expected = state::audit_native_current(&tx).unwrap().h1_canonical();
        assert!(
            state::full_audit_calls_for_test() > before,
            "计数钩子必须能观察旧 Native oracle 正控"
        );
        tx.rollback().unwrap();
        expected
    }
}

fn h1_admission_canonical(owner: &AuditConnection) -> String {
    owner
        .complete
        .borrow()
        .as_ref()
        .expect("准入必须持认证历史")
        .audited
        .h1_canonical()
}

// 不调用 b2_accept_begin/h1_publish_round：它们内部有完整 oracle，不能用于无旧审计调用锁。
fn h1_admission_accept_begin(
    owner: &AuditConnection,
    envelope: &Value,
    c: &mut request_audit::RequestAuditContext,
    epoch: &str,
) -> String {
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let (admission, ended) = owner
        .admit(
            envelope.clone(),
            epoch,
            &clock,
            tb02a::PROFILE,
            Some("1"),
            &s_allocation::Mode::CapturedNative,
            c,
        )
        .unwrap();
    assert!(matches!(
        admission,
        Admission::Advance { accepted: true, .. }
    ));
    owner.continue_with(ended).unwrap();
    owner
        .begin_advance(
            envelope,
            epoch,
            &clock,
            &s_allocation::Mode::CapturedNative,
            c,
        )
        .unwrap()
        .token
}
fn h1_admission_publish(
    owner: &AuditConnection,
    runtime: &mut prepared::Runtime,
    c: &mut request_audit::RequestAuditContext,
    n: i64,
) {
    let envelope = b2_envelope(n);
    let token = h1_admission_accept_begin(owner, &envelope, c, "1");
    let (receipt, ended) = owner.publish(runtime, &token, &envelope, c).unwrap();
    assert_eq!(receipt.generation, n);
    owner.continue_with(ended).unwrap();
}

#[test]
fn h1_admission_cold_prefixes_match_full_canonical() {
    let f = H1AdmissionFixture::new();
    let mut runtime = f.runtime();
    let mut c = context();
    let mut owner = f.open(&runtime, &mut c).unwrap();
    for n in 0..=3 {
        if n > 0 {
            h1_admission_publish(&owner, &mut runtime, &mut c, n);
        }
        let expected = f.full_canonical();
        assert_eq!(
            h1_admission_canonical(&owner).as_bytes(),
            expected.as_bytes(),
            "在线前缀 {n}"
        );
        drop(owner);
        c = context();
        let before = state::full_audit_calls_for_test();
        let (cold, events, _) = state::traced(false, || f.open(&runtime, &mut c));
        owner = cold.unwrap();
        assert_eq!(
            state::full_audit_calls_for_test(),
            before,
            "冷准入不可回落旧审计"
        );
        assert_eq!(
            h1_qualified_cuts(&events),
            (1..=n).collect::<Vec<_>>(),
            "冷从 g1 起逐代用同一 Δ 步"
        );
        assert_eq!(
            h1_admission_canonical(&owner).as_bytes(),
            expected.as_bytes(),
            "冷前缀 {n}"
        );
    }
}

fn h1_admission_error_class(error: &str) -> &str {
    // owner 的固定诊断外壳不是拒绝类别；保留原审计层给出的类别，不比较首报位置/详情。
    let mut payload = error;
    while let Some(rest) = payload.strip_prefix("S3DriverInvalid：") {
        payload = rest;
    }
    payload.split('：').next().unwrap()
}
fn h1_admission_damage(conn: &Connection, case: &str) {
    let changed = match case {
        "node-bad" => conn.execute("UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications WHERE generation=1)", []).unwrap(),
        "node-missing" => conn.execute("DELETE FROM s_content_nodes WHERE digest=(SELECT snapshot_root FROM s_content_publications WHERE generation=1)", []).unwrap(),
        "same-id-new-content" => {
            let (target, replacement): (Vec<u8>, Vec<u8>) = conn.query_row(
                "SELECT a.digest,b.body FROM s_content_nodes a JOIN s_content_publications p ON a.digest=p.snapshot_root JOIN s_content_nodes b ON b.tag=a.tag AND b.digest<>a.digest AND b.body<>a.body WHERE p.generation=1 LIMIT 1",
                [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            conn.execute("UPDATE s_content_nodes SET body=?1 WHERE digest=?2", params![replacement,target]).unwrap()
        }
        "index" => conn.execute("UPDATE structure_deltas SET input_frontier=999 WHERE generation=1", []).unwrap(),
        "clock" => conn.execute("UPDATE s_clock_events SET semantic_ns='999' WHERE phase='accept'", []).unwrap(),
        "manifest" => conn.execute("UPDATE s_content_publications SET batch_id='h1-other-batch' WHERE generation=1", []).unwrap(),
        _ => panic!("未知负例 {case}"),
    };
    assert!(changed > 0, "负例必须真实改库：{case}");
}
#[test]
fn h1_admission_cold_negative_classes_match_full_oracle() {
    for case in [
        "node-bad",
        "node-missing",
        "same-id-new-content",
        "index",
        "clock",
        "manifest",
    ] {
        let f = H1AdmissionFixture::new();
        let mut runtime = f.runtime();
        let mut c = context();
        let owner = f.open(&runtime, &mut c).unwrap();
        h1_admission_publish(&owner, &mut runtime, &mut c, 1);
        assert_eq!(h1_admission_canonical(&owner), f.full_canonical());
        drop(owner);
        h1_admission_damage(&f.conn, case);
        let tx = f.conn.unchecked_transaction().unwrap();
        let ordinary = state::audit(&tx).err().expect("旧完整审计必须拒绝负例");
        let native = state::audit_native_current(&tx)
            .err()
            .expect("Native 完整 oracle 必须拒绝负例");
        tx.rollback().unwrap();
        c = context();
        let before = state::full_audit_calls_for_test();
        let cold = f
            .open(&runtime, &mut c)
            .err()
            .expect("正式冷准入必须拒绝负例");
        assert_eq!(
            state::full_audit_calls_for_test(),
            before,
            "{case} 失败不得重判"
        );
        assert_eq!(
            h1_admission_error_class(&cold),
            h1_admission_error_class(&native),
            "{case}: cold={cold}; native={native}"
        );
        assert_eq!(
            h1_admission_error_class(&cold),
            h1_admission_error_class(&ordinary),
            "{case}: cold={cold}; ordinary={ordinary}"
        );
        assert!(!c.has_slot(), "拒绝不留下准入 slot：{case}");
    }
}

#[test]
fn h1_admission_online_own_commits_match_full_canonical() {
    let f = H1AdmissionFixture::new();
    let mut runtime = f.runtime();
    let mut c = context();
    let owner = f.open(&runtime, &mut c).unwrap();
    for n in 1..=3 {
        let before = state::full_audit_calls_for_test();
        h1_admission_publish(&owner, &mut runtime, &mut c, n);
        assert_eq!(
            state::full_audit_calls_for_test(),
            before,
            "第 {n} 代正式接纳/Begin/Prepare 不得旧审计"
        );
        assert_eq!(
            h1_admission_canonical(&owner).as_bytes(),
            f.full_canonical().as_bytes()
        );
        assert_eq!(
            h1_qualified_frontier(&owner),
            Some(runtime.audit_qualification().unwrap())
        );
    }
}

#[test]
fn h1_admission_foreign_sqlite_changes_retire_writer() {
    for case in [
        "node-bad",
        "node-missing",
        "same-id-new-content",
        "index",
        "clock",
        "manifest",
    ] {
        let f = H1AdmissionFixture::new();
        let mut runtime = f.runtime();
        let mut c = context();
        let owner = f.open(&runtime, &mut c).unwrap();
        h1_admission_publish(&owner, &mut runtime, &mut c, 1);
        // conn 与 owner.connection 是不同 SQLite 连接，真实 COMMIT 必须打断 U-A 生涯。
        h1_admission_damage(&f.conn, case);
        let before = state::full_audit_calls_for_test();
        let error = owner
            .identity("1", &f.clock, Some(&mut c))
            .err()
            .expect("外连接写入后的正式 identity 必须拒绝");
        assert!(!error.is_empty());
        assert!(owner.retired.get(), "stale 必须粘住：{case}");
        assert!(owner.complete.borrow().is_none());
        assert!(owner.summary("meta", "1", &f.clock).is_err());
        assert_eq!(
            state::full_audit_calls_for_test(),
            before,
            "{case} stale 后不得旧审计"
        );
    }
}

#[test]
fn h1_admission_cold_handoff_foreign_sqlite_commit_rejects() {
    use std::cell::RefCell;

    thread_local! {
        static HANDOFF_WRITES: Cell<usize> = const { Cell::new(0) };
        static AFTER_FOREIGN_COMMIT: RefCell<Option<(Vec<u8>, Option<Vec<u8>>)>> = const { RefCell::new(None) };
    }
    fn foreign_commit(db: &Path) -> Result<(), String> {
        // 真实独立连接的 COMMIT 落在私有认证完成、正式 owner 出生之间。
        let mut foreign = Connection::open(db).map_err(|error| error.to_string())?;
        let tx = foreign.transaction().map_err(|error| error.to_string())?;
        let changed = tx
            .execute(
                "UPDATE structure_deltas SET input_frontier=999 WHERE generation=1",
                [],
            )
            .map_err(|error| error.to_string())?;
        assert_eq!(changed, 1);
        tx.commit().map_err(|error| error.to_string())?;
        HANDOFF_WRITES.with(|count| count.set(count.get() + 1));
        AFTER_FOREIGN_COMMIT.with(|bytes| {
            *bytes.borrow_mut() = Some(h1_admission_source_bytes(db));
        });
        Ok(())
    }

    let f = H1AdmissionFixture::new();
    h1_admission_seed_three(&f);
    HANDOFF_WRITES.with(|count| count.set(0));
    AFTER_FOREIGN_COMMIT.with(|bytes| *bytes.borrow_mut() = None);
    AuditConnection::inject_cold_after_certification_for_test(foreign_commit);
    let before = state::full_audit_calls_for_test();
    let (opened, events, _) = state::traced(false, || h1_admission_driver(&f, "1"));
    let error = opened
        .err()
        .expect("冷认证后的外连接 COMMIT 必须拒绝，不能给旧 capture 盖新 owner seal");
    assert!(!error.is_empty());
    HANDOFF_WRITES.with(|count| assert_eq!(count.get(), 1, "必须实际命中交接间隙"));
    assert_eq!(
        h1_qualified_cuts(&events),
        vec![1, 2, 3],
        "外写发生在完整认证之后，拒绝时不可重认证"
    );
    assert_eq!(state::full_audit_calls_for_test(), before);
    AFTER_FOREIGN_COMMIT.with(|bytes| {
        assert_eq!(
            h1_admission_source_bytes(&f.db),
            *bytes.borrow().as_ref().expect("必须保存真实外写后的字节"),
            "外写之后的拒绝不得再写源库/WAL"
        );
    });
}

#[test]
fn h1_admission_formal_accept_begin_prepare_recover_never_full_audit() {
    let f = H1AdmissionFixture::new();
    let mut runtime = f.runtime();
    let mut c = context();
    let before = state::full_audit_calls_for_test();
    let owner = f.open(&runtime, &mut c).unwrap();
    owner
        .startup(
            &f.profile,
            "1",
            &f.clock,
            None,
            &s_allocation::Mode::CapturedNative,
            Some(&mut c),
        )
        .unwrap();
    for n in 1..=3 {
        // identity 与 summary 的只读事务交接后，下一代必须仍持 H1 资格。
        owner.identity("1", &f.clock, Some(&mut c)).unwrap();
        owner.summary("meta", "1", &f.clock).unwrap();
        h1_admission_publish(&owner, &mut runtime, &mut c, n);
    }
    drop(owner);
    c = context();
    // 监督恢复先创建新生涯并从 g1 冷认证，再进入已认证 owner 的 recover 固定写口。
    // 完整 Driver 冷重建另有独立锁，不把此处复用 Runtime 当作跨进程重建证据。
    let (recovered, events, _) = state::traced(false, || {
        let owner = f.open(&runtime, &mut c)?;
        let recovered = owner.recover("2", &f.clock, "recover-1")?;
        Ok::<_, String>((owner, recovered))
    });
    let (owner, recovered) = recovered.unwrap();
    assert_eq!(recovered["new_epoch"], "2");
    assert_eq!(
        h1_qualified_cuts(&events),
        vec![1, 2, 3],
        "recover 必须重新从 g1 认证"
    );
    owner.identity("2", &f.clock, Some(&mut c)).unwrap();
    owner.summary("meta", "2", &f.clock).unwrap();
    assert_eq!(
        state::full_audit_calls_for_test(),
        before,
        "真实 startup/identity/summary/Accept/Begin/Prepare/recover 不得调用旧审计"
    );
    // oracle 只放在整个计数区间之后，防止测试 helper 自己污染无调用断言。
    assert_eq!(
        h1_admission_canonical(&owner).as_bytes(),
        f.full_canonical().as_bytes()
    );
}

fn h1_admission_driver(f: &H1AdmissionFixture, epoch: &str) -> Result<driver::Driver, String> {
    driver::Driver::open_with_request_and_mode_retention(
        &f.db,
        &f.profile,
        epoch,
        f.clock.clone(),
        None,
        s_allocation::Mode::CapturedNative,
        536_870_912,
        true,
        super::super::admission_anchor::test_prepare(&f.db).into(),
    )
}
fn h1_admission_ledger_path(db: &Path) -> PathBuf {
    let mut name = db.as_os_str().to_os_string();
    name.push(".audit-ledger.sqlite");
    PathBuf::from(name)
}
fn h1_admission_revision_envelope(
    n: i64,
    row: (&str, i64, i64, i64, i64, i64),
    revision: i64,
    epoch: &str,
) -> Value {
    let mut envelope = b2_envelope(n);
    let mut raw = crate::tests::located_ohlc_input(&[row]);
    raw["events"][0]["revision"] = json!(revision.to_string());
    envelope["payload"]["raw_input"] = raw;
    envelope["payload"]["writer_epoch"] = json!(epoch);
    envelope["payload_hash"] = json!(sha256_hex(canonical_json(&envelope["payload"]).as_bytes()));
    envelope
}
fn h1_admission_driver_commit(
    driver: &mut driver::Driver,
    n: i64,
    row: (&str, i64, i64, i64, i64, i64),
    revision: i64,
    epoch: &str,
) {
    let before = state::full_audit_calls_for_test();
    let receipt = driver
        .handle(h1_admission_revision_envelope(n, row, revision, epoch))
        .unwrap();
    assert_eq!(receipt["kind"], "Committed");
    assert_eq!(
        state::full_audit_calls_for_test(),
        before,
        "Driver 第 {n} 代正式提交不得旧审计"
    );
}
fn h1_admission_driver_cold(
    f: &H1AdmissionFixture,
    epoch: &str,
    through: i64,
    expected_ledger: &[String],
    expected_historical: &str,
) -> driver::Driver {
    let before = state::full_audit_calls_for_test();
    let (cold, events, _) = state::traced(false, || h1_admission_driver(f, epoch));
    let cold = cold.unwrap();
    assert_eq!(
        state::full_audit_calls_for_test(),
        before,
        "新 Driver 冷起步不得旧审计"
    );
    assert_eq!(
        h1_qualified_cuts(&events),
        (1..=through).collect::<Vec<_>>()
    );
    assert_eq!(
        audit_ledger::tests::dump(&h1_admission_ledger_path(&f.db)),
        expected_ledger,
        "新 Runtime 必须从原件重建所有分区，逐行逐字节等于在线台账"
    );
    assert_eq!(
        h1_admission_canonical(cold.h1_owner_for_test()).as_bytes(),
        expected_historical.as_bytes()
    );
    cold
}

#[test]
fn h1_admission_driver_cold_recover_rebuilds_ledger_bitwise() {
    let f = H1AdmissionFixture::new();
    let before = state::full_audit_calls_for_test();
    let mut driver = h1_admission_driver(&f, "1").unwrap();
    assert_eq!(
        state::full_audit_calls_for_test(),
        before,
        "正式 Driver 空库接纳也不得旧审计"
    );
    for (n, row) in [
        (1, ("a", 0, 6, 12, 3, 7)),
        (2, ("b", 1, 14, 20, 10, 15)),
        (3, ("c", 2, 6, 12, 3, 7)),
    ] {
        h1_admission_driver_commit(&mut driver, n, row, 1, "1");
        assert_eq!(
            h1_admission_canonical(driver.h1_owner_for_test()),
            f.full_canonical()
        );
    }
    let path = h1_admission_ledger_path(&f.db);
    let online3 = audit_ledger::tests::dump(&path);
    let historical3 = f.full_canonical();
    drop(driver);
    // 不只重新开 H1 前沿：旧台账已不是 SQLite，启动仍必须只读内容原件确定重建。
    std::fs::write(&path, b"#1481 discarded stale ledger").unwrap();
    let mut driver = h1_admission_driver_cold(&f, "1", 3, &online3, &historical3);
    // 冷起步之后继续真实修订/撤回，锁住来源状态必须回交 Runtime；不能下一代退回全量 cover。
    h1_admission_driver_commit(&mut driver, 4, ("b", 1, 14, 21, 10, 15), 2, "1");
    h1_admission_driver_commit(&mut driver, 5, ("c", 2, 26, 30, 22, 27), 2, "1");
    assert_eq!(
        h1_admission_canonical(driver.h1_owner_for_test()),
        f.full_canonical()
    );
    let online5 = audit_ledger::tests::dump(&path);
    let historical5 = f.full_canonical();
    drop(driver);
    // 第二轮冷重建必须包含上一轮冷起步之后产生的 Δ，覆盖 raw/cover/node/cert 的折叠摘要。
    let driver = h1_admission_driver_cold(&f, "1", 5, &online5, &historical5);
    // R>F：真实接纳并 Begin 第六条，但尚未发布；恢复不得把这个尾部错算成已发布资格。
    let pending = h1_admission_revision_envelope(6, ("d", 3, 14, 20, 10, 15), 1, "1");
    let before = state::full_audit_calls_for_test();
    h1_admission_accept_begin(driver.h1_owner_for_test(), &pending, &mut context(), "1");
    assert_eq!(state::full_audit_calls_for_test(), before);
    assert_eq!(
        f.conn
            .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 5);
    drop(driver);
    let before = state::full_audit_calls_for_test();
    let (recovered, events, _) = state::traced(false, || {
        driver::recover_scoped_with_retention(
            &f.db,
            "2",
            &f.clock,
            "recover-1",
            536_870_912,
            super::super::admission_anchor::test_prepare(&f.db).into(),
        )
    });
    let (recovered, scope) = recovered.unwrap();
    assert_eq!(recovered["new_epoch"], "2");
    scope.finish_startup().unwrap();
    drop(scope);
    assert_eq!(
        state::full_audit_calls_for_test(),
        before,
        "正式监督 recover 不得旧审计"
    );
    assert_eq!(
        h1_qualified_cuts(&events),
        vec![1, 2, 3, 4, 5],
        "recover 的新 Runtime 必须从 g1 冷认证"
    );
    assert_eq!(
        audit_ledger::tests::dump(&path),
        online5,
        "换 epoch 不改变已发布台账事实"
    );
    // 新 Driver 的冷认证先看到 g1..g5；随后自动重接原 pending，用第二次 Begin 的
    // 64/65 时刻提交第六代。恢复事件 63 不得被早于它的第一次 Begin 61 混用。
    let before = state::full_audit_calls_for_test();
    let (driver, events, _) = state::traced(false, || h1_admission_driver(&f, "2"));
    let driver = driver.unwrap();
    assert_eq!(
        state::full_audit_calls_for_test(),
        before,
        "恢复后 pending 续写不得旧审计"
    );
    assert_eq!(h1_qualified_cuts(&events), vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 6);
    assert_eq!(
        meta_get_opt(&f.conn, "advance_state").unwrap().as_deref(),
        Some("idle")
    );
    assert_eq!(
        h1_admission_canonical(driver.h1_owner_for_test()),
        f.full_canonical()
    );
    let online6 = audit_ledger::tests::dump(&path);
    let historical6 = f.full_canonical();
    drop(driver);
    drop(h1_admission_driver_cold(&f, "2", 6, &online6, &historical6));
}

#[test]
fn h1_admission_driver_cold_preserves_original_node_record_gates() {
    for field in ["max_total_nodes", "max_total_records"] {
        let f = H1AdmissionFixture::new();
        let mut runtime = f.runtime();
        let mut c = context();
        let owner = f.open(&runtime, &mut c).unwrap();
        for n in 1..=3 {
            h1_admission_publish(&owner, &mut runtime, &mut c, n);
        }
        drop(owner);
        drop(runtime);
        // 先用正式 owner 写入历史，再首次签发短门原件；已有锚会令测试 helper 复用旧门。
        let mut owner_path = f.db.as_os_str().to_os_string();
        owner_path.push(".control-owner.json");
        assert!(!Path::new(&owner_path).exists(), "短门必须是本库首次签锚");
        let admission = super::super::admission_anchor::test_prepare_caps(&f.db, &[(field, 1)]);
        let before = state::full_audit_calls_for_test();
        let error = driver::Driver::open_with_request_and_mode_retention(
            &f.db,
            &f.profile,
            "1",
            f.clock.clone(),
            None,
            s_allocation::Mode::CapturedNative,
            536_870_912,
            true,
            admission.into(),
        )
        .err()
        .expect("正式 Driver 冷折叠不可绕过原准入账的短节点/记录门");
        assert!(error.contains("job_limit_exceeded"), "{field}：{error}");
        assert!(error.contains(field), "必须由指定原账门拒绝：{error}");
        assert_eq!(
            state::full_audit_calls_for_test(),
            before,
            "{field} 超限不得回落旧审计"
        );
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 3);
        for table in ["raw_events", "s_input_messages"] {
            let count: i64 = f
                .conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 3);
        }
        assert_eq!(
            meta_get_opt(&f.conn, "advance_state").unwrap().as_deref(),
            Some("idle"),
            "拒绝冷准入后不接受请求或 Begin"
        );
    }
}

fn h1_admission_seed_three(f: &H1AdmissionFixture) {
    let mut runtime = f.runtime();
    let mut c = context();
    let owner = f.open(&runtime, &mut c).unwrap();
    for n in 1..=3 {
        h1_admission_publish(&owner, &mut runtime, &mut c, n);
    }
}

fn h1_admission_source_bytes(db: &Path) -> (Vec<u8>, Option<Vec<u8>>) {
    let mut wal = db.as_os_str().to_os_string();
    wal.push("-wal");
    let wal = match std::fs::read(Path::new(&wal)) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("读取源 WAL 失败：{error}"),
    };
    (std::fs::read(db).unwrap(), wal)
}

#[test]
fn h1_admission_cold_real_wire_short_budget_rejects_before_write() {
    use super::super::admission_private::s4_read_trace;

    // 校准只读前缀的原账用量，不能把 SQL 长度或 quick_check 的读取当成冷折叠读取。
    let probe = H1AdmissionFixture::new();
    h1_admission_seed_three(&probe);
    s4_read_trace::enable();
    let opened = h1_admission_driver(&probe, "1").unwrap();
    let prefix_wire = s4_read_trace::cold_start_wire().expect("正式 Driver 必须进入私有冷阶段");
    let reads = s4_read_trace::measured_bytes();
    s4_read_trace::finish();
    assert!(reads[3] > 512, "必须有冷阶段 FD/VFS 实读正控：{reads:?}");
    drop(opened);

    // 新同形库首次签锚，避免 test_prepare_caps 复用 probe 已签发的长预算。
    // 512 字节余量涵盖控制原件的数字/路径长度差异，仍小于真实冷读取正控。
    let f = H1AdmissionFixture::new();
    h1_admission_seed_three(&f);
    let cap = u64::try_from(prefix_wire)
        .unwrap()
        .checked_add(512)
        .unwrap();
    let admission =
        super::super::admission_anchor::test_prepare_caps(&f.db, &[("max_total_wire_bytes", cap)]);
    let before_bytes = h1_admission_source_bytes(&f.db);
    let before_calls = state::full_audit_calls_for_test();
    s4_read_trace::enable();
    let error = driver::Driver::open_with_request_and_mode_retention(
        &f.db,
        &f.profile,
        "1",
        f.clock.clone(),
        None,
        s_allocation::Mode::CapturedNative,
        536_870_912,
        true,
        admission.into(),
    )
    .err()
    .expect("真实 SQLite 冷读取超原 wire 门必须写前拒绝");
    let cold_start = s4_read_trace::cold_start_wire().expect("短门必须通过前缀后进入冷阶段");
    let reads = s4_read_trace::measured_bytes();
    s4_read_trace::finish();
    assert!(cold_start < u128::from(cap), "不能用前缀超限代替冷读取超限");
    assert!(
        reads[3] > 0,
        "必须由实际 FD/VFS read 消耗短 wire：{reads:?}"
    );
    assert!(error.contains("job_limit_exceeded"), "{error}");
    assert!(error.contains("max_total_wire_bytes"), "{error}");
    assert_eq!(state::full_audit_calls_for_test(), before_calls);
    assert_eq!(h1_admission_source_bytes(&f.db), before_bytes);
    assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 3);
    assert_eq!(
        meta_get_opt(&f.conn, "advance_state").unwrap().as_deref(),
        Some("idle")
    );
}

#[test]
fn h1_admission_prepolicy_bad_history_rejected_before_migration() {
    for case in ["node-bad", "manifest"] {
        let f = H1AdmissionFixture::new();
        h1_admission_seed_three(&f);
        f.conn
            .execute_batch("ALTER TABLE structure_deltas DROP COLUMN policy_records")
            .unwrap();
        let (root, body, batch): (Vec<u8>, Vec<u8>, String) = f
            .conn
            .query_row(
                "SELECT p.snapshot_root,n.body,p.batch_id FROM s_content_publications p JOIN s_content_nodes n ON n.digest=p.snapshot_root WHERE p.generation=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        h1_admission_damage(&f.conn, case);
        assert!(crate::policy_records_missing(&f.conn).unwrap());
        crate::verify_writer_epoch(&f.conn, "1").unwrap();
        let admission = super::super::admission_anchor::test_prepare(&f.db);
        // f.conn 在整个拒绝区间持续打开，防止夹具末连接关闭所致 checkpoint 污染字节锁。
        let before_bytes = h1_admission_source_bytes(&f.db);
        let before_calls = state::full_audit_calls_for_test();
        let (result, events, _) = state::traced(false, || {
            driver::Driver::open_with_request_and_mode_retention(
                &f.db,
                &f.profile,
                "1",
                f.clock.clone(),
                None,
                s_allocation::Mode::CapturedNative,
                536_870_912,
                true,
                admission.into(),
            )
        });
        let error = result.err().expect("正确 epoch 的坏历史必须在迁移之前拒绝");
        assert!(
            !error.contains("StaleWriter"),
            "不能靠错误 epoch 拒绝：{case}：{error}"
        );
        assert!(
            !h1_qualified_cuts(&events).contains(&3),
            "坏 g1 不能完成历史认证"
        );
        assert_eq!(state::full_audit_calls_for_test(), before_calls, "{case}");
        assert_eq!(
            h1_admission_source_bytes(&f.db),
            before_bytes,
            "{case}：源库/WAL 必须逐字节不变"
        );
        assert!(
            crate::policy_records_missing(&f.conn).unwrap(),
            "{case}：不得迁移"
        );
        assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 3);

        // 同库只修复那项坏历史，PrePolicyRecords 形状保持不变；完整认证后才准许迁移。
        // 这项正控防止实现靠一律拒绝旧形状来通过上面的拒绝锁。
        let repaired = match case {
            "node-bad" => f.conn.execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=?2",
                params![body, root],
            ),
            "manifest" => f.conn.execute(
                "UPDATE s_content_publications SET batch_id=?1 WHERE generation=1",
                [batch],
            ),
            _ => unreachable!(),
        }
        .unwrap();
        assert_eq!(repaired, 1);
        assert!(crate::policy_records_missing(&f.conn).unwrap());
        let before_calls = state::full_audit_calls_for_test();
        let (opened, events, _) = state::traced(false, || h1_admission_driver(&f, "1"));
        let opened = opened.unwrap_or_else(|error| panic!("{case}：合法旧形状不得缩域：{error}"));
        assert_eq!(h1_qualified_cuts(&events), vec![1, 2, 3]);
        assert_eq!(state::full_audit_calls_for_test(), before_calls);
        assert!(!crate::policy_records_missing(&f.conn).unwrap());
        assert_eq!(
            h1_admission_canonical(opened.h1_owner_for_test()).as_bytes(),
            f.full_canonical().as_bytes(),
            "{case}：认证后迁移的正式历史仍须与完整 oracle 逐字节相等"
        );
    }
}
