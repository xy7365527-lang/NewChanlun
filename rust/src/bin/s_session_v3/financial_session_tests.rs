//! #1394 DESIGN-R6 c2 会话测试锁：失效条件、anchor 不符只拒本次（A6 r2 §2.2）、资源帽、首末托管、与冷路径逐字节相同、
//! 截止分型（F4）、整体截止（F5）、换 inode（F6）、READY/停机/SIGTERM（F3）、错误前缀（F8）。
//! 全部在真实 S3 夹具（init_v3 + 真实接纳/Begin/提交，文件库 WAL）上进行；socket 级用 socketpair。
//! 两个 `#[ignore]` CLI 测试需要先构建 release 二进制并在允许 bind Unix socket 的环境运行。
use super::super::prepared::tests::Fixture;
use super::*;
use rusqlite::OpenFlags;
use std::time::SystemTime;

fn publish(f: &mut Fixture, n: i64) {
    let id = format!("raw-{n}");
    let (envelope, token) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
    f.publish(&envelope, &token).unwrap();
}

fn resources() -> SessionResources {
    SessionResources {
        startup_timeout_ms: 600_000,
        retained_bytes: 1 << 34,
        max_requests: 4096,
        max_request_bytes: 193_120,
        max_reply_bytes: 827_022,
        request_timeout_ms: 45_000,
        max_capture_bytes: 1 << 32,
        max_closure_nodes: 1 << 20,
    }
}

const TEST_BINARY: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn start_with(f: &Fixture, resources: SessionResources) -> Result<Session, String> {
    Session::start(StartConfig {
        db: f.db_path(),
        resources,
        custody: Custody::observe(&f.db_path(), None).unwrap(),
        binary_sha256: TEST_BINARY.into(),
    })
}
fn start(f: &Fixture) -> Session {
    start_with(f, resources()).unwrap()
}

/// 冷路径参照：与 `financial_reader::run` 相同的只读连接、双标记核与 `audited=None` 分支。
fn cold(f: &Fixture, command: &str, request: &Value) -> Result<Vec<u8>, String> {
    let mut conn =
        Connection::open_with_flags(f.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.execute_batch("PRAGMA query_only=ON").unwrap();
    let tx = conn.transaction().unwrap();
    super::super::financial_reader::check_markers(&tx)?;
    let out = match command {
        "financial-basis-ref" => {
            super::super::financial_reader::basis_ref_with(&tx, request, None)?
        }
        "financial-closure" => super::super::financial_reader::closure_with(
            &tx,
            request,
            None,
            &mut None,
            super::super::financial_reader::ClosureBounds::UNBOUNDED,
            &mut Default::default(),
        )?,
        _ => super::super::financial_reader::content_with(&tx, request, sc::MAX_FRAME_LEN, None)?,
    };
    let mut bytes = canonical_json(&out).into_bytes();
    bytes.push(b'\n');
    Ok(bytes)
}
fn body(v: &Value) -> Vec<u8> {
    canonical_json(v).into_bytes()
}
/// 当前活动快照里的对象（对象索引另含已撤回的出生行，冷路径对它们回 MissingDependency）。
fn object_ids(f: &mut Fixture) -> Vec<String> {
    let all: Vec<String> = {
        let tx = f.conn.transaction().unwrap();
        let state = state::audit(&tx).unwrap();
        state.objects.iter().map(|o| o.id.to_string()).collect()
    };
    all.into_iter()
        .filter(|id| cold(f, "financial-basis-ref", &query(std::slice::from_ref(id))).is_ok())
        .collect()
}
fn query(ids: &[String]) -> Value {
    json!({"schema_revision":"s-financial-query/1","session_id":"s-session-testonly-001",
        "session_generation":"1","writer_epoch":"1","object_ids":ids})
}
fn ask(anchor: &Value, path: Value, unit: &str, offset: u64, limit: u64) -> Value {
    json!({"schema_revision":"s-financial-content/1","anchor":anchor,"path":path,"unit":unit,
        "offset":offset.to_string(),"limit":limit.to_string()})
}
fn objects_path() -> Value {
    json!([{"field":"snapshot"},{"field":"objects"}])
}
fn assert_refused(reply: &Reply, needle: &str) {
    let e = reply.as_ref().expect_err("expected refusal");
    assert!(e.contains(needle), "refusal {e:?} lacks {needle:?}");
}
fn wal(f: &Fixture) -> PathBuf {
    sibling(&f.db_path(), "-wal")
}
fn touch(path: &Path) {
    let file = std::fs::File::options().write(true).open(path).unwrap();
    let later = SystemTime::now() + Duration::from_secs(3);
    file.set_modified(later).unwrap();
}
/// 模拟“他连接提交却不改 stat/data_version”的退化文件系统，只为单独锁住更后面的一层判据。
fn rebaseline_stat(s: &mut Session) {
    s.main_stat = FileStat::of(&s.db).unwrap();
    s.wal_stat = FileStat::optional(&s.wal).unwrap();
}
fn rebaseline_version(s: &mut Session) {
    s.v_star = capture::version(&s.source).unwrap();
}
fn finish_json(s: &mut Session, shutdown: Shutdown, log: Option<&Path>) -> Value {
    strict_json(&s.finish(shutdown, log)).unwrap()
}
fn far() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

#[test]
fn session_replies_are_byte_identical_to_cold_path() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    publish(&mut f, 3);
    let ids = object_ids(&mut f);
    assert!(ids.len() >= 2);
    {
        let mut s = start(&f);
        let anchor = s.receipt()["anchor"].clone();
        let mut requests = vec![
            ("financial-basis-ref", query(&[])),
            ("financial-basis-ref", query(&ids[..1])),
            (
                "financial-basis-ref",
                query(&[ids[1].clone(), ids[0].clone()]),
            ),
        ];
        let total = {
            let first = cold(
                &f,
                "financial-content",
                &ask(&anchor, objects_path(), "items", 0, 1),
            )
            .unwrap();
            strict_json(&first).unwrap()["total"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };
        for offset in 0..total {
            requests.push((
                "financial-content",
                ask(&anchor, objects_path(), "items", offset, 1),
            ));
        }
        requests.push((
            "financial-content",
            ask(&anchor, objects_path(), "items", 0, total),
        ));
        requests.push((
            "financial-content",
            ask(
                &anchor,
                json!([{"field":"snapshot"},{"field":"raw_history"}]),
                "items",
                0,
                2,
            ),
        ));
        requests.push((
            "financial-content",
            ask(
                &anchor,
                json!([{"field":"snapshot"},{"field":"session_id"}]),
                "bytes",
                0,
                5,
            ),
        ));
        requests.push((
            "financial-content",
            ask(
                &anchor,
                json!([{"field":"snapshot"},{"field":"objects"},{"index":"0"}]),
                "items",
                0,
                3,
            ),
        ));
        requests.push((
            "financial-content",
            ask(
                &anchor,
                json!([{"field":"pending_revision"}]),
                "value",
                0,
                1,
            ),
        ));
        for (command, request) in &requests {
            let warm = s.handle_for_test(command, &body(request));
            let reference = cold(&f, command, request);
            assert_eq!(warm, reference, "{command} {request}");
            assert!(warm.is_ok(), "{command} {request}: {warm:?}");
        }
        assert!(s.stale_reason().is_none());
        let fin = finish_json(&mut s, Shutdown::Graceful, None);
        assert_eq!(fin["status"], "PASS", "{fin}");
        assert_eq!(fin["custody_equal"], true);
    }
}

#[test]
fn r7_binary_sha_or_db_identity_mismatch_invalidates() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    for wrong in ["binary", "dev", "ino"] {
        let mut s = start(&f);
        let mut header = json!({"schema_revision":REQUEST_SCHEMA,"command":"financial-basis-ref",
            "binary_sha256":TEST_BINARY,"db_dev":s.main_stat.dev.to_string(),
            "db_ino":s.main_stat.ino.to_string()});
        match wrong {
            "binary" => header["binary_sha256"] = json!("2".repeat(64)),
            "dev" => header["db_dev"] = json!((s.main_stat.dev + 1).to_string()),
            _ => header["db_ino"] = json!((s.main_stat.ino + 1).to_string()),
        }
        let reply = s.handle(&header, &body(&query(&[])), far(), &mut Timing::default());
        assert_refused(&reply, "MissingDependency：R7");
        assert!(s.stale_reason().is_some());
        assert_refused(
            &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
            "会话已失效",
        );
        assert_eq!(
            finish_json(&mut s, Shutdown::Graceful, None)["status"],
            "FAIL"
        );
    }
}

#[test]
fn r4_wal_mtime_change_without_content_change_invalidates() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut s = start(&f);
    assert!(s
        .handle_for_test("financial-basis-ref", &body(&query(&[])))
        .is_ok());
    let before = std::fs::read(wal(&f)).unwrap();
    touch(&wal(&f));
    assert_eq!(
        std::fs::read(wal(&f)).unwrap(),
        before,
        "touch must not change WAL bytes"
    );
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "R4：WAL身份或stat改变",
    );
    // F8：失效后的拒绝文本只有一层类别前缀。
    assert_eq!(
        s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        Err("MissingDependency：会话已失效：R4：WAL身份或stat改变".to_string())
    );
    let fin = finish_json(&mut s, Shutdown::Graceful, None);
    assert_eq!(fin["status"], "FAIL");
    assert_eq!(
        fin["custody_equal"], false,
        "custody includes WAL stat (mtime)"
    );
    assert_eq!(
        fin["custody_last"]["wal"]["sha256"],
        fin["custody_first"]["wal"]["sha256"]
    );
}

#[test]
fn r4_main_db_mtime_change_invalidates() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut s = start(&f);
    touch(&f.db_path());
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "R4：主库身份或stat改变",
    );
    assert!(s.stale_reason().is_some());
}

/// F6：主库被同字节的新 inode 替换。会话自己的 R4 与客户端上报的身份（R7）都能发现。
#[test]
fn db_replaced_by_new_inode_invalidates() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    for via_client_identity in [false, true] {
        let mut s = start(&f);
        assert!(s
            .handle_for_test("financial-basis-ref", &body(&query(&[])))
            .is_ok());
        let db = f.db_path();
        let copy = sibling(&db, ".replacement");
        std::fs::copy(&db, &copy).unwrap();
        std::fs::rename(&copy, &db).unwrap();
        assert_eq!(std::fs::read(&db).unwrap().len() as u64, s.main_stat.size);
        let now = FileStat::of(&db).unwrap();
        assert_ne!(now.ino, s.main_stat.ino);
        if via_client_identity {
            let header = json!({"schema_revision":REQUEST_SCHEMA,"command":"financial-basis-ref",
                "binary_sha256":TEST_BINARY,"db_dev":now.dev.to_string(),"db_ino":now.ino.to_string()});
            let reply = s.handle(&header, &body(&query(&[])), far(), &mut Timing::default());
            assert_refused(&reply, "R7：客户端--db身份与会话不同");
        } else {
            assert_refused(
                &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
                "R4：主库身份或stat改变",
            );
        }
        assert!(s.stale_reason().is_some());
        let fin = finish_json(&mut s, Shutdown::Graceful, None);
        assert_eq!(fin["custody_equal"], false);
        assert_eq!(
            fin["custody_last"]["main"]["sha256"],
            fin["custody_first"]["main"]["sha256"]
        );
        assert_ne!(
            fin["custody_last"]["main"]["st_ino"],
            fin["custody_first"]["main"]["st_ino"]
        );
    }
}

#[test]
fn external_commit_invalidates_and_each_later_layer_also_detects_it() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut s = start(&f);
    publish(&mut f, 2);
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "MissingDependency：R4",
    );
    assert!(s.stale_reason().is_some());
    let fin = finish_json(&mut s, Shutdown::Graceful, None);
    assert_eq!(fin["custody_equal"], false, "末次托管看见提交");
    let mut s = start(&f);
    publish(&mut f, 3);
    rebaseline_stat(&mut s);
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "R5：",
    );
    assert!(s.stale_reason().is_some());
    let mut s = start(&f);
    publish(&mut f, 4);
    rebaseline_stat(&mut s);
    rebaseline_version(&mut s);
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "R6：控制原件",
    );
    assert!(s.stale_reason().is_some());
}

/// F4：截止之后发现的显式不一致仍使会话失效；没有不一致而只是到期，只拒本次。
#[test]
fn deadline_never_downgrades_a_real_inconsistency() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    // 只到期：拒本次、不失效，随后同会话的合法请求照常。
    let mut s = start(&f);
    let expired = Instant::now() - Duration::from_millis(1);
    let reply = s.handle_at("financial-basis-ref", &body(&query(&[])), expired);
    assert_refused(&reply, "Incomplete/SessionRequestDeadline");
    assert!(s.stale_reason().is_none());
    assert_eq!(
        s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        cold(&f, "financial-basis-ref", &query(&[]))
    );
    // 到期且源已变化（R4 显式比较）：失效。
    touch(&wal(&f));
    let reply = s.handle_at("financial-basis-ref", &body(&query(&[])), expired);
    assert_refused(&reply, "R4：WAL身份或stat改变");
    assert!(s.stale_reason().is_some());
    // 到期且有他连接提交（屏蔽 R4）：R5 在第一次可被中断的读取之前做显式比较，仍失效。
    let mut s = start(&f);
    publish(&mut f, 2);
    rebaseline_stat(&mut s);
    let reply = s.handle_at("financial-basis-ref", &body(&query(&[])), expired);
    assert_refused(&reply, "R5：BEGIN前data_version改变");
    assert!(s.stale_reason().is_some());
    // 到期且只有控制原件变化（屏蔽 R4/R5）：K0 的读取被期限中断、差异尚未被比较出来，只拒本次；
    // 下一个按时的请求比较出差异并失效。
    let mut s = start(&f);
    publish(&mut f, 3);
    rebaseline_stat(&mut s);
    rebaseline_version(&mut s);
    let reply = s.handle_at("financial-basis-ref", &body(&query(&[])), expired);
    assert_refused(&reply, "Incomplete/SessionRequestDeadline");
    assert!(s.stale_reason().is_none());
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "R6：控制原件与启动K0逐字节不同",
    );
    assert!(s.stale_reason().is_some());
}

/// F4：SQLite 失败的分型只看本请求的中断/busy 记号，不看墙钟。
#[test]
fn source_fault_classification_is_typed() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let s = start(&f);
    // 已过截止：重查询必被 progress handler 中断，分型为“未完成”。
    let bound = SourceBound::install(&s.source, Instant::now() - Duration::from_millis(1)).unwrap();
    let heavy = "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c WHERE x<200000) \
                 SELECT sum(x) FROM c";
    let e = s
        .source
        .query_row(heavy, [], |r| r.get::<_, i64>(0))
        .unwrap_err();
    assert!(matches!(bound.sql("R6", e), Fault::Incomplete(_)));
    SourceBound::remove(&s.source);
    // 未到期的 SQLite 失败（坏 SQL）：分型为“不一致”。
    let bound = SourceBound::install(&s.source, far()).unwrap();
    let e = s
        .source
        .prepare("SELECT no_such_column FROM meta")
        .err()
        .unwrap();
    match bound.sql("R6", e) {
        Fault::Inconsistent(text) => assert!(text.starts_with("MissingDependency：R6："), "{text}"),
        other => panic!("{other:?}"),
    }
    SourceBound::remove(&s.source);
    // 到期之后才发生、但与中断无关的失败：同样是“不一致”（不按墙钟改判）。
    let bound = SourceBound::install(&s.source, Instant::now() - Duration::from_millis(1)).unwrap();
    let e = s
        .source
        .prepare("SELECT no_such_column FROM meta")
        .err()
        .unwrap();
    assert!(matches!(bound.sql("R6", e), Fault::Inconsistent(_)));
    SourceBound::remove(&s.source);
}

#[test]
fn empty_writes_do_not_invalidate() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    {
        let mut s = start(&f);
        let first = s
            .handle_for_test("financial-basis-ref", &body(&query(&[])))
            .unwrap();
        let stats = (
            FileStat::of(&f.db_path()).unwrap(),
            FileStat::optional(&wal(&f)).unwrap(),
        );
        f.conn.execute_batch("BEGIN IMMEDIATE; COMMIT;").unwrap();
        f.conn
            .execute("UPDATE meta SET value=value WHERE key='generation'", [])
            .unwrap();
        assert_eq!(
            (
                FileStat::of(&f.db_path()).unwrap(),
                FileStat::optional(&wal(&f)).unwrap()
            ),
            stats,
            "SQLite wrote no page for the no-op write"
        );
        assert_eq!(
            s.handle_for_test("financial-basis-ref", &body(&query(&[])))
                .unwrap(),
            first
        );
        assert!(s.stale_reason().is_none());
        assert_eq!(
            finish_json(&mut s, Shutdown::Graceful, None)["status"],
            "PASS"
        );
    }
}

#[test]
fn anchor_mismatch_default_rejects_only_the_request() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut s = start(&f);
    assert_eq!(s.receipt()["anchor_mismatch_policy"], "reject-request");
    let anchor = s.receipt()["anchor"].clone();
    let mut wrong = anchor.clone();
    wrong["writer_epoch"] = json!("2");
    let bad = ask(&wrong, objects_path(), "items", 0, 1);
    let reply = s.handle_for_test("financial-content", &body(&bad));
    assert_eq!(
        reply,
        cold(&f, "financial-content", &bad),
        "same refusal as cold path"
    );
    assert_refused(
        &reply,
        "MissingDependency：财务anchor与本片当前控制元组不同",
    );
    let mut q = query(&[]);
    q["writer_epoch"] = json!("2");
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&q)),
        "writer epoch不是当前值",
    );
    assert!(s.stale_reason().is_none());
    let good = ask(&anchor, objects_path(), "items", 0, 1);
    assert_eq!(
        s.handle_for_test("financial-content", &body(&good)),
        cold(&f, "financial-content", &good)
    );
}

/// A6 r2 §2.2：anchor／会话身份不符与坏请求、缺对象、重复 id 一样只拒本次；没有使会话失效的档。
#[test]
fn request_level_refusals_and_binding_mismatch_never_invalidate() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut s = start(&f);
    assert_refused(
        &s.handle_for_test("financial-basis-ref", b"{"),
        "FinancialReadInvalid",
    );
    assert_refused(
        &s.handle_for_test(
            "financial-basis-ref",
            &body(&query(&["absent-object".into()])),
        ),
        "请求对象不在当前活动快照",
    );
    assert_refused(
        &s.handle_for_test(
            "financial-basis-ref",
            &body(&query(&["a".into(), "a".into()])),
        ),
        "object_ids重复",
    );
    let anchor = s.receipt()["anchor"].clone();
    let mut wrong = anchor.clone();
    wrong["structure_cut"] = json!("cut-0");
    assert_refused(
        &s.handle_for_test(
            "financial-content",
            &body(&ask(&wrong, objects_path(), "items", 0, 1)),
        ),
        "财务anchor与本片当前控制元组不同",
    );
    let mut q = query(&[]);
    q["session_id"] = json!("other");
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&q)),
        "writer epoch不是当前值",
    );
    assert!(s.stale_reason().is_none());
    let good = ask(&anchor, objects_path(), "items", 0, 1);
    assert_eq!(
        s.handle_for_test("financial-content", &body(&good)),
        cold(&f, "financial-content", &good)
    );
}

/// F1：保留帽只计 P 与审计状态（无原件镜像，A6-2）；门只比 total；进程足迹只观测。审计状态堆是进程级堆差，
/// 在并行测试进程里会混入其他线程的分配，所以“为正且可复现”只在单线程下断言（见
/// `retained_audited_state_is_positive_single_threaded`，以 `--test-threads 1` 运行），读数以复制品收据为准。
#[test]
fn retained_cap_counts_copy_and_audited_state_only() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    let s = start(&f);
    let r = &s.receipt()["retained"];
    let n = |k: &str| r[k].as_str().unwrap().parse::<i64>().unwrap();
    assert!(n("audited_copy_sqlite_cache_bytes") > 0);
    assert_eq!(n("retained_original_image_heap_bytes"), 0);
    let total = [
        "audited_copy_sqlite_cache_bytes",
        "audited_copy_sqlite_schema_bytes",
        "audited_copy_sqlite_stmt_bytes",
        "audited_state_heap_bytes",
    ]
    .iter()
    .map(|k| n(k).max(0))
    .sum::<i64>();
    assert_eq!(n("total_bytes"), total);
    assert!(s.receipt()["observed"]["process_phys_footprint_bytes"].is_string());
    // 帽高于读数即可启动（进程足迹不参与），1 字节即拒。
    let mut roomy = resources();
    roomy.retained_bytes = total as u64 + (512 << 20);
    assert!(start_with(&f, roomy).is_ok());
    let mut tiny = resources();
    tiny.retained_bytes = 1;
    let e = start_with(&f, tiny).err().unwrap();
    assert!(e.starts_with("Incomplete/SessionRetained"), "{e}");
}

#[test]
#[ignore = "进程级堆差，须以 --test-threads 1 单独运行"]
fn retained_audited_state_is_positive_single_threaded() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    let reading = |s: &Session| {
        s.receipt()["retained"]["audited_state_heap_bytes"]
            .as_str()
            .unwrap()
            .parse::<i64>()
            .unwrap()
    };
    let first = reading(&start(&f));
    let second = reading(&start(&f));
    assert!(first > 0 && second > 0, "{first} {second}");
    // 进程内第一次审计另含只初始化一次的进程级静态（之后的审计复用），所以第二次不大于第一次，
    // 差额是一个小常量。生产会话每次都是新进程的第一次审计，复制品七次启动读数相差 < 0.5%。
    assert!(
        second <= first && first - second <= 32 * 1024,
        "{first} {second}"
    );
}

#[test]
fn startup_resource_caps_and_custody_refuse_to_start() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut r = resources();
    r.max_capture_bytes = 1024;
    let e = start_with(&f, r).err().unwrap();
    assert!(e.contains("Incomplete/CaptureBytes"), "{e}");
    let mut r = resources();
    r.startup_timeout_ms = 1;
    std::thread::sleep(Duration::from_millis(2));
    let e = start_with(&f, r).err().unwrap();
    assert!(e.contains("Deadline"), "{e}");
    let custody = Custody::observe(&f.db_path(), None).unwrap();
    let start_custody = |c: Custody| {
        Session::start(StartConfig {
            db: f.db_path(),
            resources: resources(),
            custody: c,
            binary_sha256: TEST_BINARY.into(),
        })
        .err()
        .unwrap()
    };
    let mut bad_sha = custody.clone();
    bad_sha.main.sha256 = "0".repeat(64);
    assert!(start_custody(bad_sha).contains("首次托管与写者收尾托管不同"));
    let mut bad_stat = custody.clone();
    bad_stat.wal.as_mut().unwrap().stat.mtime_ns += 1;
    assert!(start_custody(bad_stat).contains("首次托管与写者收尾托管不同"));
    let mut bad_db = custody.clone();
    bad_db.db = "/elsewhere/s.sqlite".into();
    assert!(start_custody(bad_db).contains("db路径与--db不同"));
    // -shm 只记不比。
    let mut shm_differs = custody;
    shm_differs.shm.as_mut().unwrap().size += 1;
    assert!(Session::start(StartConfig {
        db: f.db_path(),
        resources: resources(),
        custody: shm_differs,
        binary_sha256: TEST_BINARY.into(),
    })
    .is_ok());
}

#[test]
fn request_caps_reject_without_invalidating_except_request_count() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut r = resources();
    r.max_reply_bytes = 64;
    let mut s = start_with(&f, r).unwrap();
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "Incomplete/SessionReplyBytes",
    );
    assert!(s.stale_reason().is_none());
    let mut r = resources();
    r.max_requests = 2;
    let mut s = start_with(&f, r).unwrap();
    assert!(s
        .handle_for_test("financial-basis-ref", &body(&query(&[])))
        .is_ok());
    assert!(s
        .handle_for_test("financial-basis-ref", &body(&query(&[])))
        .is_ok());
    assert_refused(
        &s.handle_for_test("financial-basis-ref", &body(&query(&[]))),
        "Incomplete/SessionRequests",
    );
    assert!(s.stale_reason().is_some());
}

fn over_socket(
    s: &mut Session,
    command: &'static str,
    binary: &'static str,
    payload: Vec<u8>,
) -> (Reply, Value) {
    let (server, client) = UnixStream::pair().unwrap();
    let db = (s.main_stat.dev, s.main_stat.ino);
    let peer = std::thread::spawn(move || exchange(client, command, binary, db, &payload, far()));
    let mut log = Vec::new();
    assert!(matches!(s.serve_connection(server, &mut log), Conn::Done));
    let line = strict_json(&log).unwrap();
    (peer.join().unwrap(), line)
}

#[test]
fn socket_frames_stop_and_final_report_counts_custody_and_log() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    let mut r = resources();
    r.max_request_bytes = 4096;
    let mut s = start_with(&f, r).unwrap();
    let anchor = s.receipt()["anchor"].clone();
    let q = ask(&anchor, objects_path(), "items", 0, 2);
    let (reply, line) = over_socket(&mut s, "financial-content", TEST_BINARY, body(&q));
    assert_eq!(reply, cold(&f, "financial-content", &q));
    assert_eq!(line["ok"], true);
    assert_eq!(line["delivered"], true);
    let (reply, _) = over_socket(&mut s, "financial-content", TEST_BINARY, vec![b' '; 5000]);
    assert_refused(&reply, "Incomplete/SessionRequestBytes");
    assert!(s.stale_reason().is_none());
    // 停机：serve_connection 交回 Stop；FINAL 记计数、首末托管与请求日志的 SHA/字节数。
    let dir = std::env::temp_dir().join(format!("c2-final-{}-{}", std::process::id(), line["n"]));
    std::fs::create_dir_all(&dir).unwrap();
    let log_path = dir.join("requests.jsonl");
    {
        let mut log = std::fs::File::create(&log_path).unwrap();
        let (server, client) = UnixStream::pair().unwrap();
        let db = (s.main_stat.dev, s.main_stat.ino);
        let payload = body(&q);
        let peer = std::thread::spawn(move || {
            exchange(
                client,
                "financial-content",
                TEST_BINARY,
                db,
                &payload,
                far(),
            )
        });
        assert!(matches!(s.serve_connection(server, &mut log), Conn::Done));
        assert!(peer.join().unwrap().is_ok());
    }
    let (server, mut client) = UnixStream::pair().unwrap();
    let peer = std::thread::spawn(move || {
        let header =
            json!({"schema_revision":REQUEST_SCHEMA,"command":"stop","binary_sha256":TEST_BINARY});
        write_frame(&mut client, canonical_json(&header).as_bytes(), far()).unwrap();
        read_reply(&mut client, far())
    });
    let mut sink = Vec::new();
    let Conn::Stop(mut stream, deadline) = s.serve_connection(server, &mut sink) else {
        panic!("stop not returned")
    };
    assert!(sink.is_empty(), "stop is not logged as a request");
    let bytes = s.finish(Shutdown::Graceful, Some(&log_path));
    let mut frame = vec![0u8];
    frame.extend_from_slice(&bytes);
    write_frame(&mut stream, &frame, deadline).unwrap();
    assert_eq!(peer.join().unwrap().unwrap(), bytes);
    let fin = strict_json(&bytes).unwrap();
    assert_eq!(fin["status"], "PASS", "{fin}");
    assert_eq!(fin["shutdown"], "graceful-stop");
    assert_eq!(fin["counts"]["requests"], "3");
    assert_eq!(fin["counts"]["served"], "2");
    assert_eq!(fin["counts"]["refused"], "1");
    let log_bytes = std::fs::read(&log_path).unwrap();
    assert_eq!(fin["requests_log"]["sha256"], sha_hex(&log_bytes));
    assert_eq!(fin["requests_log"]["bytes"], log_bytes.len().to_string());
    assert_eq!(fin["custody_first"]["main"], fin["custody_last"]["main"]);
    assert_eq!(fin["custody_equal"], true);
    // 同一会话若以 SIGTERM 结束：即便其余全部正常也只能是 FAIL，并标非正常停机。
    let sig = finish_json(&mut s, Shutdown::Sigterm, Some(&log_path));
    assert_eq!(sig["status"], "FAIL");
    assert_eq!(sig["abnormal_shutdown"], true);
    assert_eq!(sig["shutdown"], "SIGTERM-abnormal");
    // 坏帧头（非本客户端）：拒绝并失效。
    let (server, mut client) = UnixStream::pair().unwrap();
    let peer = std::thread::spawn(move || {
        write_frame(&mut client, b"not json", far()).unwrap();
        read_reply(&mut client, far())
    });
    let mut log = Vec::new();
    assert!(matches!(s.serve_connection(server, &mut log), Conn::Done));
    assert_refused(&peer.join().unwrap(), "帧头非严格JSON");
    assert!(s.stale_reason().is_some());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// F5：整体截止覆盖读请求体；逐字节滴流的客户端在截止处被断开并计数，会话不失效。
#[test]
fn slow_client_is_cut_at_the_overall_deadline() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut r = resources();
    r.request_timeout_ms = 800;
    let mut s = start_with(&f, r).unwrap();
    let (server, mut client) = UnixStream::pair().unwrap();
    let (dev, ino) = (s.main_stat.dev, s.main_stat.ino);
    let peer = std::thread::spawn(move || {
        let header = json!({"schema_revision":REQUEST_SCHEMA,"command":"financial-basis-ref",
            "binary_sha256":TEST_BINARY,"db_dev":dev.to_string(),"db_ino":ino.to_string()});
        let mut frame = (canonical_json(&header).len() as u32)
            .to_be_bytes()
            .to_vec();
        frame.extend_from_slice(canonical_json(&header).as_bytes());
        client.write_all(&frame).unwrap();
        let payload = body(&query(&[]));
        let _ = client.write_all(&(payload.len() as u32).to_be_bytes());
        // 每 150 ms 一个字节：单次读超时永远不会触发，只有整体截止能断开它。
        for b in payload {
            if client.write_all(&[b]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    });
    let started = Instant::now();
    let mut log = Vec::new();
    assert!(matches!(s.serve_connection(server, &mut log), Conn::Done));
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(700) && took < Duration::from_millis(3000),
        "{took:?}"
    );
    let line = strict_json(&log).unwrap();
    assert_eq!(line["client_timeout"], true);
    assert_eq!(line["delivered"], false);
    assert_eq!(s.counts.client_timeouts, 1);
    assert!(s.stale_reason().is_none());
    peer.join().unwrap();
    // 完全沉默的客户端同样在截止处放弃。
    let (server, client) = UnixStream::pair().unwrap();
    let mut log = Vec::new();
    assert!(matches!(s.serve_connection(server, &mut log), Conn::Done));
    drop(client);
    assert_eq!(s.counts.client_timeouts, 2);
    assert!(s.stale_reason().is_none());
    assert!(s
        .handle_for_test("financial-basis-ref", &body(&query(&[])))
        .is_ok());
}

/// F3：READY 之前的连接一律拒绝（读完对端帧再回 NotReady）。
#[test]
fn connections_before_ready_are_refused() {
    let (server, client) = UnixStream::pair().unwrap();
    let peer = std::thread::spawn(move || {
        exchange(
            client,
            "financial-basis-ref",
            TEST_BINARY,
            (1, 2),
            b"{}",
            far(),
        )
    });
    let (command, timed_out) = refuse_before_ready(server, 193_120);
    assert_eq!(command, "financial-basis-ref");
    assert!(!timed_out);
    assert_eq!(peer.join().unwrap(), Err(missing(NOT_READY)));
}

#[test]
fn resource_and_custody_documents_are_exact() {
    let good = resources().to_json();
    assert_eq!(SessionResources::from_json(&good).unwrap(), resources());
    let mut old = good.clone();
    old["schema_revision"] = json!("s-financial-session-resources/1");
    assert!(SessionResources::from_json(&old).is_err());
    let mut extra = good.clone();
    extra["max_pages"] = json!("1");
    assert!(SessionResources::from_json(&extra).is_err());
    let mut zero = good.clone();
    zero["max_requests"] = json!("0");
    assert!(SessionResources::from_json(&zero).is_err());
    let mut number = good.clone();
    number["max_requests"] = json!(4096);
    assert!(SessionResources::from_json(&number).is_err());
    let mut padded = good;
    padded["max_requests"] = json!("04096");
    assert!(SessionResources::from_json(&padded).is_err());

    let file = |sha: bool| {
        let mut v = json!({"st_dev":"16777233","st_ino":"42","st_size":"0","st_mtime_ns":"1760000000123456789",
            "st_ctime_ns":"1760000000123456789"});
        if sha {
            v["sha256"] = json!("a".repeat(64));
        }
        v
    };
    let custody = json!({"schema_revision":CUSTODY_SCHEMA,"db":"/abs/s.sqlite","main":file(true),
        "wal":file(true),"shm":file(false)});
    let parsed = Custody::from_json(&custody).unwrap();
    assert_eq!(parsed.to_json(), custody, "round trip is exact");
    let nulls = json!({"schema_revision":CUSTODY_SCHEMA,"db":"/abs/s.sqlite","main":file(true),"wal":null,"shm":null});
    assert!(Custody::from_json(&nulls).is_ok());
    let mutate = |f: &dyn Fn(&mut Value)| {
        let mut v = custody.clone();
        f(&mut v);
        Custody::from_json(&v).is_err()
    };
    assert!(mutate(
        &|v| v["schema_revision"] = json!("s-financial-session-custody/1")
    ));
    assert!(mutate(&|v| v["extra"] = json!("x")));
    assert!(mutate(&|v| v["db"] = json!("relative/s.sqlite")));
    assert!(mutate(&|v| v["main"]["sha256"] = json!("A".repeat(64))));
    assert!(mutate(&|v| v["main"]["st_ino"] = json!("042")));
    assert!(mutate(&|v| v["main"]["st_ino"] = json!(42)));
    assert!(mutate(&|v| v["main"]["st_size"] = json!("-1")));
    assert!(mutate(
        &|v| v["main"]["st_size"] = json!("18446744073709551616")
    ));
    assert!(mutate(&|v| v
        .as_object_mut()
        .unwrap()
        .remove("shm")
        .map(|_| ())
        .unwrap()));
    assert!(mutate(&|v| v["shm"]["sha256"] = json!("a".repeat(64))));
    assert!(mutate(&|v| v["main"] = Value::Null));
    assert!(mutate(&|v| v["wal"]
        .as_object_mut()
        .unwrap()
        .remove("st_ctime_ns")
        .map(|_| ())
        .unwrap()));
}

#[test]
fn error_prefixes_are_not_nested() {
    assert_eq!(bare("MissingDependency：Q3CaptureInvalid：x"), "x");
    assert_eq!(bare("FinancialReadInvalid：y"), "y");
    assert_eq!(bare("Incomplete/X：z"), "Incomplete/X：z");
}

#[test]
fn cli_grammars_are_exact() {
    let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let ok = v(&[
        "s",
        "financial-content",
        "--protocol-revision",
        "s-session/3",
        "--db",
        "/d",
        "--session",
        "/s",
    ]);
    assert!(parse_flags(&ok, &["--protocol-revision", "--db", "--session"], "c").is_ok());
    for bad in [
        v(&[
            "s",
            "financial-content",
            "--protocol-revision",
            "s-session/3",
            "--db",
            "/d",
            "--db",
            "/s",
        ]),
        v(&[
            "s",
            "financial-content",
            "--protocol-revision",
            "s-session/2",
            "--db",
            "/d",
            "--session",
            "/s",
        ]),
        v(&[
            "s",
            "financial-content",
            "--protocol-revision",
            "s-session/3",
            "--db",
            "--session",
            "/s",
            "x",
        ]),
        v(&[
            "s",
            "financial-content",
            "--protocol-revision",
            "s-session/3",
            "--db",
            "/d",
        ]),
    ] {
        assert!(
            parse_flags(&bad, &["--protocol-revision", "--db", "--session"], "c").is_err(),
            "{bad:?}"
        );
    }
}

/// #1394 A6 r2 §2.2（A6-2 已裁）：会话一致性只有 a 一档，serve 语法里没有 `--consistency`，带上它即拒；
/// 启动回执的 `consistency`、`anchor_mismatch_policy` 两键（r7 接口形状）恒为唯一档，不保留原件镜像。
#[test]
fn serve_grammar_has_no_consistency_choice() {
    let serve = |extra: &[&str]| -> Vec<String> {
        let mut a = vec![
            "s",
            "financial-serve",
            "--protocol-revision",
            "s-session/3",
            "--db",
            "/d",
            "--socket",
            "/s",
            "--resources",
            "/r",
            "--custody",
            "/c",
        ];
        a.extend_from_slice(extra);
        a.extend_from_slice(&["--receipts", "/o"]);
        a.into_iter().map(String::from).collect()
    };
    assert!(!SERVE_FLAGS.contains(&"--consistency"));
    assert!(parse_flags(&serve(&[]), &SERVE_FLAGS, "financial-serve").is_ok());
    for choice in ["a", "b"] {
        assert!(
            parse_flags(
                &serve(&["--consistency", choice]),
                &SERVE_FLAGS,
                "financial-serve"
            )
            .is_err(),
            "--consistency {choice}"
        );
    }
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let s = start(&f);
    let r = s.receipt();
    assert_eq!(r["consistency"], "a");
    assert_eq!(r["anchor_mismatch_policy"], "reject-request");
    assert_eq!(r["retained_original_accounted_bytes"], "0");
    assert_eq!(r["retained"]["retained_original_image_heap_bytes"], "0");
}

// ---------------- CLI 层（真实二进制、真实 Unix socket；需 bind 权限） ----------------

struct Served {
    child: std::process::Child,
    ready: Value,
    socket: PathBuf,
    receipts: PathBuf,
}
fn short_socket_dir(tag: &str) -> PathBuf {
    let dir = PathBuf::from(format!("/tmp/c2s-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    dir
}
fn spawn_serve(
    binary: &Path,
    db: &Path,
    work: &Path,
    socket_dir: &Path,
    before_ready: impl FnOnce(&Path),
) -> Served {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let receipts = work.join("receipts");
    std::fs::create_dir_all(&receipts).unwrap();
    let mut r = resources();
    r.retained_bytes = 1 << 32;
    std::fs::write(work.join("resources.json"), canonical_json(&r.to_json())).unwrap();
    let custody = Custody::observe(db, None).unwrap();
    std::fs::write(
        work.join("custody.json"),
        canonical_json(&custody.to_json()),
    )
    .unwrap();
    let socket = socket_dir.join("s.sock");
    let mut child = Command::new(binary)
        .args([
            "financial-serve",
            "--protocol-revision",
            "s-session/3",
            "--db",
        ])
        .arg(db)
        .arg("--socket")
        .arg(&socket)
        .arg("--resources")
        .arg(work.join("resources.json"))
        .arg("--custody")
        .arg(work.join("custody.json"))
        .arg("--receipts")
        .arg(&receipts)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < until, "socket never bound");
        std::thread::sleep(Duration::from_millis(5));
    }
    before_ready(&socket);
    let mut line = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let ready = strict_json(
        line.strip_prefix("READY ")
            .expect("READY line")
            .trim_end()
            .as_bytes(),
    )
    .unwrap();
    let startup = std::fs::read(receipts.join("STARTUP.json")).unwrap();
    assert_eq!(startup.last(), Some(&b'\n'));
    assert_eq!(
        ready["startup_receipt_sha256"],
        sha_hex(&startup[..startup.len() - 1])
    );
    assert_eq!(strict_json(&startup).unwrap(), ready["startup_receipt"]);
    Served {
        child,
        ready,
        socket,
        receipts,
    }
}
fn cli_call(
    binary: &Path,
    db: &Path,
    socket: Option<&Path>,
    command: &str,
    request: &[u8],
) -> (bool, Vec<u8>, Vec<u8>) {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(binary);
    cmd.args([command, "--protocol-revision", "s-session/3", "--db"])
        .arg(db);
    if let Some(socket) = socket {
        cmd.arg("--session").arg(socket);
    }
    let mut child = cmd
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(request).unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.success(), out.stdout, out.stderr)
}
fn pre_ready_probe(socket: &Path) -> Reply {
    let stream = UnixStream::connect(socket).unwrap();
    exchange(
        stream,
        "financial-basis-ref",
        TEST_BINARY,
        (0, 0),
        b"{}",
        far(),
    )
}
fn stop_cli(binary: &Path, served: Served) -> (std::process::ExitStatus, Value, Value) {
    let out = std::process::Command::new(binary)
        .args([
            "financial-serve-stop",
            "--protocol-revision",
            "s-session/3",
            "--session",
        ])
        .arg(&served.socket)
        .env_clear()
        .output()
        .unwrap();
    let mut child = served.child;
    let status = child.wait().unwrap();
    let reply = strict_json(&out.stdout).unwrap();
    let file = strict_json(&std::fs::read(served.receipts.join("FINAL.json")).unwrap()).unwrap();
    assert_eq!(reply, file, "stop reply equals FINAL.json");
    let log = std::fs::read(served.receipts.join("requests.jsonl")).unwrap();
    assert_eq!(file["requests_log"]["sha256"], sha_hex(&log));
    assert_eq!(file["requests_log"]["bytes"], log.len().to_string());
    (status, file, served.ready)
}

/// 真实二进制在夹具库上：READY 唯一行与回执 SHA、会话回复与 6 参冷 CLI 逐字节相同、优雅停机 FINAL、
/// SIGTERM 落 FINAL 并以信号终止。运行：`S3_FINANCIAL_CLI_BIN=<release 二进制> cargo test … -- --ignored`。
#[test]
#[ignore = "需要 S3_FINANCIAL_CLI_BIN 与 bind Unix socket 的权限"]
fn cli_fixture_ready_stop_and_sigterm() {
    use std::os::unix::process::ExitStatusExt;
    let binary = PathBuf::from(std::env::var("S3_FINANCIAL_CLI_BIN").unwrap());
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    let db = f.db_path();
    let work = db.parent().unwrap().join("cli-a");
    std::fs::create_dir(&work).unwrap();
    let served = spawn_serve(&binary, &db, &work, &short_socket_dir("a"), |_| {});
    let request = body(&query(&[]));
    let (ok, warm, err) = cli_call(
        &binary,
        &db,
        Some(&served.socket),
        "financial-basis-ref",
        &request,
    );
    assert!(ok, "{}", String::from_utf8_lossy(&err));
    let (ok, cold_reply, _) = cli_call(&binary, &db, None, "financial-basis-ref", &request);
    assert!(ok);
    assert_eq!(warm, cold_reply);
    // #1394 第三问选项 A：financial-closure 的 8 参会话客户端与 6 参冷 CLI 逐块逐字节相同。
    let anchor = served.ready["startup_receipt"]["anchor"].clone();
    let mut after = String::new();
    let mut chunks = 0;
    loop {
        let ask = body(&closure_ask(&anchor, json!([]), &after, 6000));
        let (ok, warm, err) = cli_call(
            &binary,
            &db,
            Some(&served.socket),
            "financial-closure",
            &ask,
        );
        assert!(ok, "{}", String::from_utf8_lossy(&err));
        let (ok, cold_reply, err) = cli_call(&binary, &db, None, "financial-closure", &ask);
        assert!(ok, "{}", String::from_utf8_lossy(&err));
        assert_eq!(warm, cold_reply, "闭包块会话与冷 CLI 逐字节相同");
        chunks += 1;
        let reply = strict_json(&warm).unwrap();
        if reply["done"] == true {
            break;
        }
        after = reply["next"].as_str().unwrap().to_owned();
    }
    assert!(chunks > 1);
    let (status, fin, _) = stop_cli(&binary, served);
    assert!(status.success());
    assert_eq!(fin["status"], "PASS");
    assert_eq!(fin["counts"]["served"], (1 + chunks).to_string());

    let work = db.parent().unwrap().join("cli-b");
    std::fs::create_dir(&work).unwrap();
    let mut served = spawn_serve(&binary, &db, &work, &short_socket_dir("b"), |_| {});
    let (ok, _, _) = cli_call(
        &binary,
        &db,
        Some(&served.socket),
        "financial-basis-ref",
        &request,
    );
    assert!(ok);
    // SAFETY: 向自己启动的子进程发 SIGTERM。
    unsafe { libc::kill(served.child.id() as libc::pid_t, libc::SIGTERM) };
    let status = served.child.wait().unwrap();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{status:?}");
    let fin = strict_json(&std::fs::read(served.receipts.join("FINAL.json")).unwrap()).unwrap();
    assert_eq!(fin["status"], "FAIL");
    assert_eq!(fin["shutdown"], "SIGTERM-abnormal");
    assert_eq!(fin["abnormal_shutdown"], true);
    assert_eq!(fin["custody_equal"], true);
    assert!(fin["stale"].is_null());
    assert!(!served.socket.exists());
}

/// r5 对拍锁（cargo 载体）：在诊断克隆上起会话，READY 前探针被拒，r5 的 28 个请求经 8 参客户端回放，
/// 有 r5 参照的 27 个逐字节相同，优雅停机 FINAL=PASS。需要环境变量：S3_FINANCIAL_CLI_BIN、
/// S3_SESSION_REPLICA_DB（克隆主库路径）、S3_SESSION_REPLICA_MAIN_SHA256、S3_SESSION_R5_RUN（r5 run-1 目录，只读其请求与回复文件）、
/// S3_SESSION_WORK（空工作目录）。绝不指向 r5 S-carrier 原件。
#[test]
#[ignore = "需要诊断克隆、r5 证据文件与 bind Unix socket 的权限"]
fn cli_replica_replays_r5_byte_identical() {
    let env = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("缺 {k}"));
    let binary = PathBuf::from(env("S3_FINANCIAL_CLI_BIN"));
    let db = PathBuf::from(env("S3_SESSION_REPLICA_DB"));
    assert!(
        !db.to_string_lossy().contains("S-carrier"),
        "绝不打开 r5 原件"
    );
    assert_eq!(
        file_sha256(&db, None).unwrap(),
        env("S3_SESSION_REPLICA_MAIN_SHA256")
    );
    let r5 = PathBuf::from(env("S3_SESSION_R5_RUN"));
    let work = PathBuf::from(env("S3_SESSION_WORK"));
    let mut probe = None;
    let served = spawn_serve(&binary, &db, &work, &short_socket_dir("r5"), |socket| {
        probe = Some(pre_ready_probe(socket));
    });
    assert_eq!(probe.unwrap(), Err(missing(NOT_READY)));
    let mut labels = vec![("financial-basis-ref", "S-discovery-basis".to_string())];
    labels.extend((0..27).map(|i| ("financial-content", format!("S-support-request-{i}"))));
    let (mut compared, mut identical) = (0, 0);
    for (command, label) in labels {
        let request = strict_json(
            &std::fs::read(r5.join("s-query").join(format!("{label}.request.json"))).unwrap(),
        )
        .unwrap();
        let (ok, reply, err) = cli_call(
            &binary,
            &db,
            Some(&served.socket),
            command,
            canonical_json(&request).as_bytes(),
        );
        assert!(ok, "{label}: {}", String::from_utf8_lossy(&err));
        if let Ok(reference) = std::fs::read(r5.join(format!("{label}.stdout"))) {
            compared += 1;
            assert_eq!(reply, reference, "{label}");
            identical += 1;
        }
    }
    assert_eq!((compared, identical), (27, 27));
    let (status, fin, _) = stop_cli(&binary, served);
    assert!(status.success(), "{fin}");
    assert_eq!(fin["status"], "PASS");
    assert_eq!(fin["counts"]["refused_before_ready"], "1");
    assert_eq!(fin["counts"]["served"], "28");
}

// ---------------- #1394 第三问选项 A：financial-closure ----------------

fn closure_ask(anchor: &Value, path: Value, after: &str, budget: u64) -> Value {
    json!({"schema_revision":"s-financial-closure/1","anchor":anchor,"path":path,"after":after,
        "max_reply_bytes":budget.to_string()})
}
fn node_parts(v: &Value) -> (sc::NodeDigest, u8, Vec<u8>) {
    let node = newchan_rust::s_content::fragment::ProofNode::from_json(v).unwrap();
    let digest = sc::parse_root(&node.root).unwrap();
    let tag = u8::from_str_radix(&node.tag, 16).unwrap();
    let body = node
        .body_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    (digest, tag, body)
}
/// 逐块取完一个闭包（经会话），每块与冷路径逐字节相同；返回全部回复。
fn walk_closure(s: &mut Session, f: &Fixture, path: Value, budget: u64) -> Vec<Value> {
    let anchor = s.receipt()["anchor"].clone();
    let mut after = String::new();
    let mut replies = Vec::new();
    loop {
        let request = closure_ask(&anchor, path.clone(), &after, budget);
        let warm = s
            .handle_for_test("financial-closure", &body(&request))
            .unwrap();
        assert_eq!(
            warm,
            cold(f, "financial-closure", &request).unwrap(),
            "会话块与冷路径逐字节相同"
        );
        assert!(
            warm.len() as u64 <= budget,
            "块含末尾 LF 不超过 max_reply_bytes"
        );
        let reply = strict_json(&warm).unwrap();
        after = reply["next"].as_str().unwrap().to_owned();
        let done = reply["done"] == true;
        replies.push(reply);
        if done {
            return replies;
        }
    }
}

#[test]
fn closure_chunks_are_exactly_the_basis_closure_and_match_cold_path() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    publish(&mut f, 3);
    let mut s = start(&f);
    let budget = 3000;
    let replies = walk_closure(&mut s, &f, json!([]), budget);
    assert!(replies.len() > 3, "小预算须分成多块：{}", replies.len());
    let root = sc::ContentRef::from_transport_json(&s.receipt()["anchor"]["basis_ref"]).unwrap();
    let total: usize = replies[0]["total"].as_str().unwrap().parse().unwrap();
    let mut store = sc::MemoryNodes::new();
    let mut previous: Option<sc::NodeDigest> = None;
    for (i, reply) in replies.iter().enumerate() {
        assert_eq!(reply["target_ref"], root.to_transport_json());
        assert_eq!(reply["total"], total.to_string());
        assert_eq!(reply["done"], i + 1 == replies.len());
        let nodes = reply["nodes"].as_array().unwrap();
        assert!(!nodes.is_empty());
        for v in nodes {
            let (digest, tag, body) = node_parts(v);
            assert!(previous.is_none_or(|p| p < digest), "全程按摘要严格升序");
            previous = Some(digest);
            sc::NodeSink::put(&mut store, digest, tag, &body).unwrap();
        }
        assert_eq!(reply["next"], sc::root_string(&previous.unwrap()));
        // 贪心装块：非末块再加下一块首节点（含逗号）即超预算。
        if let Some(following) = replies.get(i + 1) {
            let used = canonical_json(reply).len() as u64 + 1;
            let first = canonical_json(&following["nodes"][0]).len() as u64 + 1;
            assert!(used + first > budget, "块未装满：{used}+{first}<={budget}");
        }
    }
    assert_eq!(store.len(), total);
    let verified = sc::verify(&mut store, &root).expect("收到的节点足以完整核验 basis 根");
    assert_eq!(verified.nodes, total, "不多不少：闭包流恰为根闭包");
    let fin = finish_json(&mut s, Shutdown::Graceful, None);
    assert_eq!(fin["status"], "PASS");
    assert_eq!(fin["closure_table"]["nodes"], total.to_string());
    let table_bytes: u64 = fin["retained_final"]["closure_table_bytes"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(table_bytes >= 32 * total as u64);
    assert_eq!(fin["retained_startup"]["closure_table_bytes"], "0");
}

#[test]
fn closure_of_a_path_carries_the_proof_from_the_basis_root() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    let mut s = start(&f);
    let replies = walk_closure(&mut s, &f, objects_path(), 4000);
    let root = sc::ContentRef::from_transport_json(&s.receipt()["anchor"]["basis_ref"]).unwrap();
    let target = sc::ContentRef::from_transport_json(&replies[0]["target_ref"]).unwrap();
    let mut store = sc::MemoryNodes::new();
    for reply in &replies {
        for v in reply["nodes"].as_array().unwrap() {
            let (d, t, b) = node_parts(v);
            sc::NodeSink::put(&mut store, d, t, &b).unwrap();
        }
    }
    let path = newchan_rust::s_content::fragment::parse_path(&objects_path()).unwrap();
    assert_eq!(
        newchan_rust::s_content::path::lookup_path(&mut store, &root, &path).unwrap(),
        target,
        "块内含从 basis 根到目标的路径证明"
    );
    sc::verify(&mut store, &target).expect("目标闭包完整");
    let whole = walk_closure(&mut s, &f, json!([]), 4000);
    assert!(
        whole[0]["total"]
            .as_str()
            .unwrap()
            .parse::<usize>()
            .unwrap()
            > store.len(),
        "子路径闭包小于整根闭包"
    );
    // 会话只留一张表：最后一次请求的键。
    assert_eq!(
        s.closure.as_ref().unwrap().nodes().to_string(),
        whole[0]["total"].as_str().unwrap()
    );
}

#[test]
fn closure_rejects_bad_cursor_budget_fields_and_anchor_without_invalidating() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut s = start(&f);
    let anchor = s.receipt()["anchor"].clone();
    let first = strict_json(
        &s.handle_for_test(
            "financial-closure",
            &body(&closure_ask(&anchor, json!([]), "", 1 << 20)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(first["done"], true, "大预算一块取完");
    let last = first["next"].as_str().unwrap().to_owned();
    let stranger = sc::root_string(&[7u8; 32]);
    for (request, needle) in [
        (
            closure_ask(&anchor, json!([]), &stranger, 4000),
            "after 游标不在该闭包内",
        ),
        (
            closure_ask(&anchor, json!([]), &last, 4000),
            "after 之后已无闭包节点",
        ),
        (
            closure_ask(&anchor, json!([]), "sc1-XYZ", 4000),
            "FinancialReadInvalid",
        ),
        (
            closure_ask(&anchor, json!([]), "", 120),
            "Incomplete/ClosureReplyBytes",
        ),
        (
            closure_ask(&anchor, json!([{"field":"no-such-field"}]), "", 4000),
            "FinancialReadInvalid",
        ),
    ] {
        assert_refused(
            &s.handle_for_test("financial-closure", &body(&request)),
            needle,
        );
        assert_refused(
            &cold(&f, "financial-closure", &request).map_err(|e| e),
            needle,
        );
    }
    let mut zero = closure_ask(&anchor, json!([]), "", 4000);
    zero["max_reply_bytes"] = json!("0");
    assert_refused(
        &s.handle_for_test("financial-closure", &body(&zero)),
        "FinancialReadInvalid",
    );
    let mut extra = closure_ask(&anchor, json!([]), "", 4000);
    extra["limit"] = json!("10");
    assert_refused(
        &s.handle_for_test("financial-closure", &body(&extra)),
        "字段必须exact",
    );
    let mut old = closure_ask(&anchor, json!([]), "", 4000);
    old["schema_revision"] = json!("s-financial-closure/0");
    assert_refused(
        &s.handle_for_test("financial-closure", &body(&old)),
        "财务闭包版本不符",
    );
    let mut drifted = anchor.clone();
    drifted["commit_ns"] = json!("1");
    assert_refused(
        &s.handle_for_test(
            "financial-closure",
            &body(&closure_ask(&drifted, json!([]), "", 4000)),
        ),
        "财务anchor与本片当前控制元组不同",
    );
    assert!(s.stale_reason().is_none(), "请求级拒绝不使会话失效");
    let again = s
        .handle_for_test(
            "financial-closure",
            &body(&closure_ask(&anchor, json!([]), "", 1 << 20)),
        )
        .unwrap();
    assert_eq!(strict_json(&again).unwrap(), first, "同一请求同一回复");
    assert_eq!(
        finish_json(&mut s, Shutdown::Graceful, None)["status"],
        "PASS"
    );
}

#[test]
fn closure_table_cap_and_retained_gate_refuse_only_the_request() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let mut small = resources();
    small.max_closure_nodes = 10;
    let mut s = start_with(&f, small).unwrap();
    let anchor = s.receipt()["anchor"].clone();
    let ask_all = body(&closure_ask(&anchor, json!([]), "", 1 << 20));
    assert_refused(
        &s.handle_for_test("financial-closure", &ask_all),
        "Incomplete/SessionClosureNodes",
    );
    assert!(
        s.closure.is_none() && s.retained.closure_table == 0,
        "超帽不留半张表"
    );
    assert!(s.stale_reason().is_none());
    assert!(s
        .handle_for_test("financial-basis-ref", &body(&query(&object_ids(&mut f))))
        .is_ok());

    let mut s = start(&f);
    s.resources.retained_bytes = s.retained.total() + 64;
    assert_refused(
        &s.handle_for_test("financial-closure", &ask_all),
        "Incomplete/SessionRetained",
    );
    assert!(s.closure.is_none() && s.retained.closure_table == 0);
    s.resources.retained_bytes = 1 << 34;
    let ok = s.handle_for_test("financial-closure", &ask_all).unwrap();
    let table = s.retained.closure_table;
    assert!(table > 0 && s.retained.total() > table as u64);
    // 换键（子路径）先丢旧表：保留量是新表的字节，不是两表之和。
    s.handle_for_test(
        "financial-closure",
        &body(&closure_ask(&anchor, objects_path(), "", 1 << 20)),
    )
    .unwrap();
    assert!(s.retained.closure_table < table);
    assert!(!ok.is_empty() && s.stale_reason().is_none());
}

#[test]
fn closure_stream_stops_when_the_source_changes() {
    let mut f = Fixture::new();
    publish(&mut f, 1);
    publish(&mut f, 2);
    let mut s = start(&f);
    let anchor = s.receipt()["anchor"].clone();
    let first = strict_json(
        &s.handle_for_test(
            "financial-closure",
            &body(&closure_ask(&anchor, json!([]), "", 3000)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(first["done"], false);
    touch(&wal(&f));
    let next = closure_ask(&anchor, json!([]), first["next"].as_str().unwrap(), 3000);
    assert_refused(
        &s.handle_for_test("financial-closure", &body(&next)),
        "R4：WAL身份或stat改变",
    );
    assert_refused(
        &s.handle_for_test("financial-closure", &body(&next)),
        "会话已失效",
    );
    assert_eq!(
        finish_json(&mut s, Shutdown::Graceful, None)["status"],
        "FAIL"
    );
}

/// #1717：单 bar 真实 S3 夹具（init_v3 + 真实接纳/Begin/提交，文件库 WAL）留在原处，路径与可读对象登记到
/// `TB1717_S_EXPORT`，同目录写 `financial-serve` 启动所需的写者收尾托管与会话资源，供经济侧内容域正控以
/// 冻结的 `SessionClosure` 读法读同一库。单 bar 对应 #1394 具名单 bar 小源的规模；会话资源取 A6 r2 §2.3
/// 的 S86 r6 具名值，`max_closure_nodes` 取同节建议值。夹具目录不随本测试删除，写者连接不关闭以免收尾
/// checkpoint 改动托管后的主库与 WAL。
#[test]
#[ignore = "仅供 ci-macos 的 #1717 内容域正控导出真实 S3 库"]
fn export_real_s3_fixture_for_economic_content_domain() {
    let out = PathBuf::from(std::env::var("TB1717_S_EXPORT").expect("TB1717_S_EXPORT 未设置"));
    let dir = out.parent().unwrap();
    let mut f = Fixture::new();
    publish(&mut f, 1);
    let ids = object_ids(&mut f);
    assert!(!ids.is_empty(), "真实 S3 夹具没有可读的当前对象");
    let resources = SessionResources {
        startup_timeout_ms: 300_000,
        retained_bytes: 268_435_456,
        max_requests: 4096,
        max_request_bytes: 193_120,
        max_reply_bytes: 827_022,
        request_timeout_ms: 45_000,
        max_capture_bytes: 805_306_368,
        max_closure_nodes: 262_144,
    };
    let resources_path = dir.join("s-session-resources.json");
    std::fs::write(&resources_path, canonical_json(&resources.to_json())).unwrap();
    let custody_path = dir.join("s-writer-custody.json");
    let custody = Custody::observe(&f.db_path(), None).unwrap();
    std::fs::write(&custody_path, canonical_json(&custody.to_json())).unwrap();
    let export = json!({"schema_revision":"tb1717-s3-fixture-export/2","db":f.db_path(),
        "session_id":"s-session-testonly-001","session_generation":"1","writer_epoch":"1",
        "profile_id":"ohlc_integer_tb02a_v1","bars":"1","object_ids":ids,
        "session_resources":resources_path,"writer_custody":custody_path});
    std::fs::write(&out, canonical_json(&export)).unwrap();
    std::mem::forget(f);
}

type Q3Bar = (String, i64, i64, i64, i64, i64);

/// #1419 Q3 真实导出的数据窗（28Q）：(全部 bar, 修订目标根, 修订插入点)。
///
/// 旧窗（458fc1a3ed，9 端点「每段三笔」链）实跑 101/101 代只成 1 个中枢、无中枢对：解析出的
/// 段端点被相邻段吞并（段 4 个完成、第 5 个在途），第二组重叠段从未成段。本窗按同一构造
/// （每笔 4 根、中点线性插值、上下各留 20；首两根 W 相互包含）直接给出笔端点，经
/// `parse_layer` + `scan_center_facts_full` 逐前缀搜得：第 56 根首中枢成立、第 104 根第二中枢
/// 成立（中枢对），窗口截在第 104 根，不多跑一代。
fn q3_w_bars() -> (Vec<Q3Bar>, usize, usize) {
    const PIVOTS: &[i64] = &[
        1620, 1020, 1350, 1185, 1570, 1390, 1480, 1270, 1780, 1525, 2120, 1820, 1970, 1620, 1800,
        1710, 1920, 1350, 1635, 970, 1360, 1165, 1620, 1470, 1545, 1370, 1970, 1670, 2370,
    ];
    const LEN: usize = 104;
    let bar = |i: usize, mid: i64, half: i64| -> Q3Bar {
        (format!("w-{i}"), i as i64, mid, mid + half, mid - half, mid)
    };
    let mut bars = vec![bar(0, PIVOTS[0], 40), bar(1, PIVOTS[0], 30)];
    for w in PIVOTS.windows(2) {
        for j in 0..4 {
            bars.push(bar(bars.len(), w[0] + (w[1] - w[0]) * j / 4, 20));
        }
    }
    bars.truncate(LEN);
    // 修订目标：首中枢内的笔顶 2120；修订插入点在首中枢（第 56 根）成立之后。
    let top = bars.iter().position(|b| b.2 == 2120).unwrap();
    (bars, top, 60)
}

/// 前缀上的 (完成中枢数)；与 S3 E/F 同一共享核：`parse_layer` 段 → L0 单元 → `scan_center_facts_full`。
fn q3_w_centers(bars: &[Q3Bar]) -> usize {
    use newchan_rust::theta_v0::classifier::{center::center_from_segments, center_scan};
    use newchan_rust::theta_v0::{config::ThetaConfig, parser::parse_layer, types::Bar};
    let raw: std::vec::Vec<Bar> = bars
        .iter()
        .enumerate()
        .map(|(i, b)| Bar {
            source_index: i,
            timestamp: b.1,
            open: b.2,
            high: b.3,
            low: b.4,
            close: b.5,
            volume: 1.0,
            untradable: false,
        })
        .collect();
    let layer = parse_layer(&raw, &ThetaConfig::default());
    let units = center_scan::l0_units(layer.segments_valid_prefix());
    center_scan::scan_center_facts_full(&units, center_from_segments)
        .windows
        .len()
}

/// 秒级锁：导出窗在 2.5 h 实跑前就须能成中枢对（28Q：旧窗实跑到第 101 代才暴露无中枢对）。
#[test]
fn q3_w_export_window_forms_center_pair() {
    let (bars, top, revise_after) = q3_w_bars();
    assert!(top < revise_after && revise_after < bars.len());
    assert!(q3_w_centers(&bars[..revise_after]) >= 1, "修订前无首中枢");
    assert_eq!(
        q3_w_centers(&bars[..bars.len() - 1]),
        1,
        "窗口未截在第二中枢成立处"
    );
    assert_eq!(q3_w_centers(&bars), 2, "导出窗不成中枢对");
}

/// 同一窗经 S3 生成核（`GenerationRuntime::generate`，W 写者规则）一代全量生成：第 104 根才有
/// `CC-020.center_pair`。这是导出逐代发布所调的同一生成函数，只省去逐代提交与来源审计。
#[test]
fn q3_w_export_window_s3_generation_publishes_center_pair() {
    use super::super::business_json::{self as bj, RawEventsView};
    use super::super::generation::{GenerationInput, GenerationRuntime, StructurePriors};
    let (bars, _, _) = q3_w_bars();
    let rows: std::vec::Vec<Value> = bars
        .iter()
        .enumerate()
        .map(|(seq, (id, c, o, h, l, cl))| {
            json!({"identity_key":id,"source_coord":c.to_string(),"revision":1,"seq":seq+1,
                "ts":c.to_string(),"open":o.to_string(),"high":h.to_string(),"low":l.to_string(),
                "close":cl.to_string(),"input_revision":"1","payload_hash":format!("payload-{seq}"),
                "receipt_id":format!("receipt-{seq}"),"event_id":format!("event-{id}")})
        })
        .collect();
    let count = |n: usize| -> (usize, usize) {
        let raw = RawEventsView::plain(&rows[..n]);
        let effective = bj::effective(raw, None).unwrap();
        let scope = json!({});
        let input = GenerationInput {
            all_events: raw,
            effective: effective.raw_view(),
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: n as i64,
            profile_id: crate::tb02a::PROFILE,
            profile_hash: "q3-window-profile",
            rule_revision: crate::W_STRUCTURE_RULE_REVISION,
            catalog_revision: "q3-window-catalog",
            scope: &scope,
            semantic_commit_ns: Some("123456789"),
            prior: crate::content_bi::PriorState {
                bi_records: vec![],
                version_record: None,
            },
            structure_prior: StructurePriors::default(),
        };
        let mut nodes = sc::MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            "q3-window".into(),
            crate::tb02a::PROFILE,
            crate::W_STRUCTURE_RULE_REVISION,
        );
        let generated = runtime.generate(&mut nodes, &input).unwrap();
        let of = |kind: &str| {
            generated
                .objects
                .iter()
                .filter(|o| o.kind() == kind)
                .count()
        };
        (of("CC-016.center"), of("CC-020.center_pair"))
    };
    let (centers, pairs) = count(bars.len());
    assert!(
        centers >= 2 && pairs >= 1,
        "全窗：中枢 {centers}、中枢对 {pairs}"
    );
    assert_eq!(count(bars.len() - 1).1, 0, "窗口未截在中枢对首现处");
}

/// #1419 Q3 真实消费验收：W 规则真实 S3 多代导出（27AZ P1）。走 init_v3 + 真实接纳/Begin/提交，逐代发布：
/// 1 根与 2 根相互包含（W 等待期）→ 真实写者恢复（纪元 1→2）→ 方向建立的前段 → 首中枢成立
/// → 首中枢内笔顶改价修订（撤回）→ 改回原价修订（恢复）→ 尾段至第二中枢成立（中枢对）。数据窗见 [`q3_w_bars`]。任一代被门拒绝即原样记录并停止。导出格式对齐
/// `s_session/tests/fixtures/q3-complete-s16-cuts345.json`，节点取全表。
#[test]
#[ignore = "仅供 #1419 Q3 真实 S3 W 夹具导出"]
fn export_real_s3_w_fixture_for_q3() {
    let (bars, top, revise_after) = q3_w_bars();
    let mut revised = bars[top].clone();
    revised.2 -= 300;
    revised.3 -= 300;
    revised.4 -= 300;
    revised.5 -= 300;
    let direction = 2 + 20;
    // 真实恢复放在 W 等待期（第 2 代后、未定原件仍挂起）：恢复审计对全历史逐 cut 计 wire，
    // fc51265054 在 89 代后、b3d2ca09d0 在 22 代后恢复均超 max_total_wire_bytes 1 GiB 被拒
    // （读数交 #1640 与 #1419 验收，不放宽上限）。方向确立、中枢、撤回与恢复修订均在纪元 2 发生。
    // 修订在首中枢成立（第 56 根）后、尾段之前做，使撤回/恢复两代审计的库较小；随后追加其余
    // bar 至第二中枢成立（第 104 根）。
    assert!(top < revise_after && revise_after < bars.len());
    assert_eq!(
        q3_w_centers(&bars),
        2,
        "导出窗不成中枢对（先跑 q3_w_export_window_forms_center_pair）"
    );
    // S3 接纳协议：一条 ingest 恰一事件，每条消息各自 Begin/提交一代；里程碑统计取该段最后一代。
    let steps: Vec<(&str, Vec<Q3Bar>, i64, bool)> = vec![
        ("w_first_bar", bars[0..1].to_vec(), 1, false),
        ("w_contained_waiting", bars[1..2].to_vec(), 1, false),
        (
            "recovered_epoch2_direction",
            bars[2..direction].to_vec(),
            1,
            true,
        ),
        (
            "chain_first_center",
            bars[direction..revise_after].to_vec(),
            1,
            false,
        ),
        ("revision_withdraw", vec![revised], 2, false),
        ("revision_restore", vec![bars[top].clone()], 3, false),
        ("chain_tail_pair", bars[revise_after..].to_vec(), 1, false),
    ];
    let messages: i64 = steps.iter().map(|(_, rows, _, _)| rows.len() as i64).sum();
    // 真实恢复：恢复前最后一代提交（ns=k*10+2）之后、下一次接纳（ns=(k+1)*10）之前，纪元 1→2。
    let before_recover: i64 = steps
        .iter()
        .take_while(|(_, _, _, recover)| !recover)
        .map(|(_, rows, _, _)| rows.len() as i64)
        .sum();
    let mut f = Fixture::with_clock_plan(64, messages, &[("recover-1", before_recover * 10 + 5)]);
    let mut n: i64 = 0;
    let mut stats = Vec::new();
    let count = |f: &Fixture, sql: &str, g: i64| -> Vec<(String, i64)> {
        let mut q = f.conn.prepare(sql).unwrap();
        q.query_map([g], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    for (label, rows, revision, reopen) in steps.iter() {
        if *reopen {
            f.recover_writer("2", "recover-1");
        }
        let mut published = None;
        let mut refused = None;
        let mut withdrawn_total: std::collections::BTreeMap<String, i64> = Default::default();
        for (id, c, o, h, l, cl) in rows {
            n += 1;
            let (envelope, token) =
                f.accept_begin(n, (id.as_str(), *c, *o, *h, *l, *cl), *revision);
            let started = std::time::Instant::now();
            match f.publish(&envelope, &token) {
                Ok(receipt) => {
                    let live = count(
                        &f,
                        "SELECT kind,count(*) FROM s_content_objects WHERE first_known_generation<=?1 AND (withdrawn_generation IS NULL OR withdrawn_generation>?1) AND kind IN ('CC-010.bi','CC-013.segment','CC-016.center','CC-020.center_pair') GROUP BY kind",
                        receipt.generation,
                    );
                    eprintln!(
                        "Q3-PROGRESS gen={} step={label} ms={} {:?}",
                        receipt.generation,
                        started.elapsed().as_millis(),
                        live
                    );
                    for (kind, count) in count(
                        &f,
                        "SELECT kind,count(*) FROM s_content_objects WHERE withdrawn_generation=?1 GROUP BY kind",
                        receipt.generation,
                    ) {
                        *withdrawn_total.entry(kind).or_default() += count;
                    }
                    published = Some(receipt);
                }
                Err(error) => {
                    refused = Some(format!("message-{n}：{error}"));
                    break;
                }
            }
        }
        match (published, refused) {
            (Some(receipt), None) => {
                let g = receipt.generation;
                let current = count(&f, "SELECT kind,count(*) FROM s_content_objects WHERE first_known_generation<=?1 AND (withdrawn_generation IS NULL OR withdrawn_generation>?1) GROUP BY kind ORDER BY kind", g);
                let present = |list: &[&str]| -> Vec<String> {
                    list.iter()
                        .filter(|k| current.iter().any(|(x, _)| x == *k))
                        .map(|k| k.to_string())
                        .collect()
                };
                if *label == "chain_first_center" {
                    assert!(
                        current.iter().any(|(k, c)| k == "CC-016.center" && *c > 0),
                        "首中枢里程碑无中枢：{current:?}"
                    );
                }
                let w_present = present(&["CC-004.initialization", "CC-004.undetermined_raw"]);
                let ef: Vec<&str> = crate::tb02e::KINDS
                    .iter()
                    .chain(crate::tb02f::KINDS)
                    .copied()
                    .collect();
                let ef_present = present(&ef);
                let withdrawn: serde_json::Map<String, Value> = withdrawn_total
                    .iter()
                    .map(|(k, c)| (k.clone(), json!(c)))
                    .collect();
                let kinds: serde_json::Map<String, Value> =
                    current.iter().map(|(k, c)| (k.clone(), json!(c))).collect();
                stats.push(json!({"step":label,"generation":g.to_string(),"batch_id":receipt.batch_id.as_str(),
                    "bars":rows.len(),"revision":revision,"recovered_writer":reopen,
                    "current_by_kind":kinds,
                    "withdrawn_by_kind":withdrawn,"w_kinds_present":w_present,"ef_kinds_present":ef_present}));
            }
            (_, error) => {
                let error = error.unwrap_or_else(|| "空里程碑".to_string());
                stats.push(
                    json!({"step":label,"bars":rows.len(),"revision":revision,"refused":error}),
                );
                println!("Q3-STATS {}", canonical_json(&json!(stats)));
                panic!("真实 S3 W 导出（{label}）被拒：{error}");
            }
        }
    }
    let mut q = f
        .conn
        .prepare("SELECT generation,batch_id,manifest_root,snapshot_root,delta_root FROM s_content_publications ORDER BY generation")
        .unwrap();
    let root = |v: Vec<u8>| sc::root_string(&<[u8; 32]>::try_from(v.as_slice()).unwrap());
    let publications: Vec<Value> = q
        .query_map([], |r| {
            Ok(json!({"generation":r.get::<_,i64>(0)?.to_string(),"batch_id":r.get::<_,String>(1)?,
                "manifest_root":root(r.get(2)?),"snapshot_root":root(r.get(3)?),"delta_root":root(r.get(4)?)}))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(q);
    let mut q = f
        .conn
        .prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
        .unwrap();
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let nodes: Vec<Value> = q
        .query_map([], |r| {
            Ok(
                json!({"root":root(r.get(0)?),"tag":r.get::<_,i64>(1)?.to_string(),
                "body_hex":hex(&r.get::<_,Vec<u8>>(2)?)}),
            )
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(q);
    let mut q = f
        .conn
        .prepare("SELECT key,value FROM meta ORDER BY key")
        .unwrap();
    let meta: serde_json::Map<String, Value> = q
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, json!(r.get::<_, String>(1)?)))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(q);
    let catalog: Value =
        serde_json::from_str(meta["catalog_definition"].as_str().unwrap()).unwrap();
    let catalog_rows: Vec<Value> = catalog["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            let mut row = item.clone();
            row.as_object_mut().unwrap().remove("acceptance_mode");
            row
        })
        .collect();
    let writer_epoch = meta["writer_epoch"].clone();
    let export = json!({
        "provenance":{"kind":"TestOnly real S3 W-rule multi-cut export (init_v3 + real accept/Begin/commit, file WAL)",
            "test":"v3::financial_session::tests::export_real_s3_w_fixture_for_q3",
            "head":std::env::var("Q3_EXPORT_HEAD").unwrap_or_default(),
            "rule_revision":meta["rule_revision"],"steps":stats},
        "publications":publications,"nodes":nodes,"meta":meta,
        "session_generation":"1","catalog_rows":catalog_rows});
    let text = canonical_json(&export);
    if let Ok(path) = std::env::var("Q3_REAL_EXPORT") {
        std::fs::write(path, &text).unwrap();
    }
    println!("=====Q3-REAL-S3-EXPORT-BEGIN=====");
    println!("{text}");
    println!("=====Q3-REAL-S3-EXPORT-END=====");
    println!("Q3-STATS {}", canonical_json(&json!(stats)));
    // 验收覆盖（导出已打印后再断言，失败时数据仍可取回）：各代均已发布；W 等待期两 kind 出现；
    // 中枢与中枢对非空；修订代确有撤回；恢复后纪元 2 的一代已发布。
    assert_eq!(stats.len(), steps.len(), "每一代都必须真实发布");
    let has = |i: usize, key: &str, kind: &str| {
        stats[i][key]
            .as_object()
            .is_some_and(|m| m.get(kind).and_then(Value::as_i64).unwrap_or(0) > 0)
    };
    assert!(
        has(1, "current_by_kind", "CC-004.undetermined_raw"),
        "等待期缺未定原件"
    );
    assert!(
        has(1, "current_by_kind", "CC-004.initialization"),
        "等待期缺初始化事实"
    );
    assert!(
        (0..stats.len()).any(|i| has(i, "current_by_kind", "CC-016.center")),
        "全程无非空中枢"
    );
    assert!(
        (0..stats.len()).any(|i| has(i, "current_by_kind", "CC-020.center_pair")),
        "全程无中枢对"
    );
    assert!(
        stats
            .iter()
            .find(|s| s["step"] == "revision_withdraw")
            .unwrap()["withdrawn_by_kind"]
            .as_object()
            .is_some_and(|m| !m.is_empty()),
        "修订代无撤回"
    );
    assert_eq!(writer_epoch, json!("2"), "恢复必须真实递增写者纪元");
}
