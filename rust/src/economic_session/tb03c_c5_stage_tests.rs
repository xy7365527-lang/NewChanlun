//! #1406 C5 v3：真实 peer → flush → SQLite Stage 的目标复读锁。
//! 普通 /2 SQLite 仅验已存在节点的 Noop 和读后拒绝；不替代 HFS 物理门。
//! 最后一条 ignored 入口才执行 Stage::new、实际新写、checkpoint 和失败清理。
use super::*;
use crate::economic_session::{store, tb03c_c5_read_tests::expire_after_sql_returns};
use crate::s_content::job::{DEADLINE, LIMIT_EXCEEDED};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

thread_local! { static EXPIRE_FLUSH: Cell<bool> = const { Cell::new(false) }; }
struct ExpireFlush;
impl ExpireFlush {
    fn arm() -> Self {
        EXPIRE_FLUSH.with(|v| assert!(!v.replace(true)));
        Self
    }
}
impl Drop for ExpireFlush {
    fn drop(&mut self) {
        EXPIRE_FLUSH.with(|v| v.set(false));
    }
}
/// 只在 cfg(test) 最终/满块 flush 边界令外部期限到达，不替换 SQL 或节点。
pub(super) fn before_peer_flush(deadline: Option<Instant>) -> Option<Instant> {
    if EXPIRE_FLUSH.with(|v| v.replace(false)) {
        let at = Instant::now();
        assert!(deadline.is_some_and(|old| old > at));
        Some(at)
    } else {
        deadline
    }
}

struct Files(PathBuf);
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Db {
    // 字段按此顺序析构：先关 SQLite，再回收本测试独占目录。
    c: Connection,
    files: Files,
}
impl Db {
    fn fresh(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "c5-stage-{}-{tag}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let c = Connection::open(dir.join("eb.sqlite")).unwrap();
        c.execute_batch("CREATE TABLE meta(k TEXT PRIMARY KEY,v TEXT NOT NULL) STRICT;")
            .unwrap();
        c.execute_batch(store::CONTENT_SQL).unwrap();
        c.execute("INSERT INTO meta VALUES('schema_revision',?1)", [SCHEMA_V2])
            .unwrap();
        c.execute(
            "INSERT INTO content_meta VALUES(1,?1)",
            [s_content::CONTENT_FORMAT],
        )
        .unwrap();
        Self {
            c,
            files: Files(dir),
        }
    }
    fn path(&self) -> PathBuf {
        self.files.0.join("eb.sqlite")
    }
    /// 已存在/拒绝分支不会进入物理写核；产品载体前件另由 ignored 锁覆盖。
    fn read_stage(&mut self) -> Stage<'_> {
        let db = self.path();
        Stage {
            c: &mut self.c,
            db,
            token: "c5-target".into(),
            ceiling: 16_777_216,
        }
    }
}
struct Node {
    root: ContentRef,
    digest: NodeDigest,
    tag: u8,
    body: Vec<u8>,
}
impl Node {
    fn scalar(value: s_content::Value) -> Self {
        let mut memory = s_content::MemoryNodes::new();
        let root = s_content::write_value(&mut memory, &value)
            .unwrap()
            .content_ref();
        assert_eq!(memory.iter().count(), 1, "必须是完整合法单节点原件");
        let digest = root.digest().unwrap();
        let (tag, body) = memory.get(&digest).unwrap();
        Self {
            root,
            digest,
            tag,
            body: body.to_vec(),
        }
    }
    fn payload(&self) -> u128 {
        8 + self.body.len() as u128
    }
    fn tuple(&self) -> (NodeDigest, u8, Vec<u8>) {
        (self.digest, self.tag, self.body.clone())
    }
}
fn insert(c: &Connection, table: &str, node: &Node, tag: i64, body: &[u8]) {
    let sql = match table {
        "content_nodes" => "INSERT INTO content_nodes VALUES(?1,?2,?3)",
        "content_staging" => "INSERT INTO content_staging VALUES(?1,?2,?3,'c5-target')",
        _ => panic!("未知测试表"),
    };
    c.execute(sql, params![&node.digest[..], tag, body])
        .unwrap();
}
fn seed(c: &Connection, table: &str, node: &Node) {
    insert(c, table, node, i64::from(node.tag), &node.body);
}
type RowImage = (String, Vec<u8>, i64, Vec<u8>, String);
fn image(c: &Connection) -> Vec<RowImage> {
    let mut q = c.prepare("SELECT 'published',digest,tag,body,'' FROM content_nodes UNION ALL SELECT 'staged',digest,tag,body,stage FROM content_staging ORDER BY 1,2").unwrap();
    let rows = q
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    rows
}
fn limits() -> JobLimits {
    JobLimits {
        max_total_wire_bytes: 16_777_216,
        max_total_node_bytes: 16_777_216,
        max_total_nodes: 100_000,
        max_total_records: 1,
        max_total_requests: 100,
        max_elapsed_ms: 1000,
        max_staged_bytes: 16_777_216,
        max_store_bytes: 16_777_216,
    }
}
fn job(c: &Connection, db: &Path, caps: JobLimits) -> (Job, Rc<Cell<Instant>>) {
    let clock = Rc::new(Cell::new(Instant::now()));
    let for_job = clock.clone();
    let job = Job::new(
        caps,
        existing(c, db).unwrap(),
        Box::new(move || for_job.get()),
    )
    .unwrap();
    (job, clock)
}
fn no_reservations(job: &Job) {
    let s = job.snapshot();
    assert_eq!(s.open_reservations, 0);
    assert_eq!(s.staged.pending, 0);
    assert_eq!(s.store.pending, 0);
    assert_eq!(s.records, 0, "SQL 行不是六族/Delta 顶层记录");
}

/// 只观察真实连接上准备的内容 SELECT，不替换返回值或授权结果。
/// 与 wire 的实际返回字节联合断言，区分源读取和目标复读；仅凭 wire 相等会
/// 把“在第二次源读取就停止”误当作目标边界已覆盖。每轮调用前清掉 image 查询。
struct SqlReads(Arc<Mutex<Vec<String>>>);
impl SqlReads {
    fn watch(c: &Connection) -> Self {
        let reads = Arc::new(Mutex::new(Vec::new()));
        let observed = reads.clone();
        c.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
            if let rusqlite::hooks::AuthAction::Read {
                table_name,
                column_name: "tag",
            } = ctx.action
            {
                if matches!(table_name, "content_nodes" | "content_staging") {
                    observed.lock().unwrap().push(table_name.to_owned());
                }
            }
            rusqlite::hooks::Authorization::Allow
        }))
        .unwrap();
        Self(reads)
    }
    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
    fn assert_tables(&self, expected: &[&str]) {
        assert_eq!(self.0.lock().unwrap().as_slice(), expected);
    }
}

#[test]
fn c5_h2_peer_flush_counts_published_staged_and_duplicate_target_returns() {
    let node = Node::scalar(s_content::Value::Null);
    for tables in [
        vec!["content_nodes"],
        vec!["content_staging"],
        vec!["content_nodes", "content_staging"],
    ] {
        let peer = Db::fresh("peer");
        let mut target = Db::fresh("target");
        seed(&peer.c, "content_nodes", &node);
        for table in &tables {
            seed(&target.c, table, &node);
        }
        let before = image(&target.c);
        let (mut job, _) = job(&target.c, &target.path(), limits());
        let source_reads = SqlReads::watch(&peer.c);
        let target_reads = SqlReads::watch(&target.c);
        let mut stage = target.read_stage();
        for attempt in 1..=2 {
            source_reads.clear();
            target_reads.clear();
            import_peer_refs_with_deadline(
                &peer.c,
                std::slice::from_ref(&node.root),
                &mut stage,
                &mut job,
                None,
            )
            .unwrap();
            let s = job.snapshot();
            source_reads.assert_tables(&["content_nodes", "content_nodes"]);
            target_reads.assert_tables(&[
                "content_nodes",
                "content_staging",
                "content_nodes",
                "content_staging",
            ]);
            // 根 header 与闭包各读源一次；两项 pending 各复读目标两表。
            // 不存在的行没有返回载荷，已存在的同一行复读仍全额计费。
            assert_eq!(
                s.wire_bytes,
                attempt * 2 * (1 + tables.len() as u128) * node.payload()
            );
            assert_eq!(s.nodes, 1, "相同节点只计一次，复读 wire 全额再计");
            assert_eq!(s.stopped, None);
            no_reservations(&job);
            assert_eq!(image(stage.c), before, "Noop 不写、不删任一目标行");
        }
    }
}

#[test]
fn c5_h2_peer_flush_wire_limit_keeps_target_return_then_stops_before_next_read() {
    let node = Node::scalar(s_content::Value::Null);
    let peer = Db::fresh("wire-peer");
    let mut target = Db::fresh("wire-target");
    seed(&peer.c, "content_nodes", &node);
    seed(&target.c, "content_nodes", &node);
    seed(&target.c, "content_staging", &node);
    let before = image(&target.c);
    let mut caps = limits();
    // 两次源返回恰好用完帽，第一次目标返回实记后超帽，下一目标表不得读取。
    caps.max_total_wire_bytes = 2 * node.payload() as u64;
    let (mut job, _) = job(&target.c, &target.path(), caps);
    let source_reads = SqlReads::watch(&peer.c);
    let target_reads = SqlReads::watch(&target.c);
    let mut stage = target.read_stage();
    for attempt in 0..2 {
        source_reads.clear();
        target_reads.clear();
        let failure = import_peer_refs_with_deadline(
            &peer.c,
            std::slice::from_ref(&node.root),
            &mut stage,
            &mut job,
            None,
        )
        .unwrap_err();
        assert!(failure.to_string().contains(LIMIT_EXCEEDED), "{failure}");
        source_reads.assert_tables(if attempt == 0 {
            &["content_nodes", "content_nodes"]
        } else {
            &[]
        });
        target_reads.assert_tables(if attempt == 0 {
            &["content_nodes"]
        } else {
            &[]
        });
        assert_eq!(job.snapshot().wire_bytes, 3 * node.payload());
        assert_eq!(job.snapshot().stopped, Some(LIMIT_EXCEEDED));
        assert_eq!(job.snapshot().nodes, 1);
        assert_eq!(image(stage.c), before);
        no_reservations(&job);
    }
}

#[test]
fn c5_h2_peer_flush_bad_target_tag_or_digest_keeps_failed_bytes_on_same_job_retry() {
    let node = Node::scalar(s_content::Value::Null);
    for table in ["content_nodes", "content_staging"] {
        for (tag, body) in [(256, vec![]), (i64::from(node.tag), vec![1])] {
            let peer = Db::fresh("bad-peer");
            let mut target = Db::fresh("bad-target");
            seed(&peer.c, "content_nodes", &node);
            // 保留产品 DDL；仅此反例注入越界 tag，随即恢复 CHECK。
            target
                .c
                .execute_batch("PRAGMA ignore_check_constraints=ON")
                .unwrap();
            insert(&target.c, table, &node, tag, &body);
            target
                .c
                .execute_batch("PRAGMA ignore_check_constraints=OFF")
                .unwrap();
            let before = image(&target.c);
            let (mut job, _) = job(&target.c, &target.path(), limits());
            let source_reads = SqlReads::watch(&peer.c);
            let target_reads = SqlReads::watch(&target.c);
            let mut stage = target.read_stage();
            for attempt in 1..=2 {
                source_reads.clear();
                target_reads.clear();
                assert!(import_peer_refs_with_deadline(
                    &peer.c,
                    std::slice::from_ref(&node.root),
                    &mut stage,
                    &mut job,
                    None
                )
                .is_err());
                source_reads.assert_tables(&["content_nodes", "content_nodes"]);
                target_reads.assert_tables(if table == "content_nodes" {
                    &["content_nodes"]
                } else {
                    &["content_nodes", "content_staging"]
                });
                assert_eq!(
                    job.snapshot().wire_bytes,
                    attempt * (2 * node.payload() + 8 + body.len() as u128)
                );
                assert_eq!(job.snapshot().nodes, 1);
                assert_eq!(job.snapshot().stopped, None);
                assert_eq!(image(stage.c), before);
                no_reservations(&job);
            }
        }
    }
}

#[test]
fn c5_h2_peer_final_flush_expiry_prevents_target_read_or_write_and_is_call_scoped() {
    let node = Node::scalar(s_content::Value::Null);
    for present in [false, true] {
        let peer = Db::fresh("flush-peer");
        let mut target = Db::fresh("flush-target");
        seed(&peer.c, "content_nodes", &node);
        if present {
            seed(&target.c, "content_nodes", &node);
        }
        let before = image(&target.c);
        let (mut job, _) = job(&target.c, &target.path(), limits());
        let source_reads = SqlReads::watch(&peer.c);
        let target_reads = SqlReads::watch(&target.c);
        let mut stage = target.read_stage();
        let _hook = ExpireFlush::arm();
        let failure = import_peer_refs_with_deadline(
            &peer.c,
            std::slice::from_ref(&node.root),
            &mut stage,
            &mut job,
            Some(Instant::now() + Duration::from_secs(60)),
        )
        .unwrap_err();
        assert!(failure.to_string().contains("最早绝对期限"), "{failure}");
        EXPIRE_FLUSH.with(|v| assert!(!v.get(), "必须经过最后 flush 的时钟触发点"));
        source_reads.assert_tables(&["content_nodes", "content_nodes"]);
        target_reads.assert_tables(&[]);
        assert_eq!(
            job.snapshot().wire_bytes,
            2 * node.payload(),
            "源读取完成，目标读取尚未开始"
        );
        assert_eq!(job.snapshot().nodes, 1);
        assert_eq!(
            job.snapshot().stopped,
            None,
            "更早的调用期限不重写 Job 自身期限"
        );
        assert_eq!(image(stage.c), before);
        no_reservations(&job);
        if present {
            source_reads.clear();
            target_reads.clear();
            import_peer_refs_with_deadline(
                &peer.c,
                std::slice::from_ref(&node.root),
                &mut stage,
                &mut job,
                None,
            )
            .unwrap();
            source_reads.assert_tables(&["content_nodes", "content_nodes"]);
            target_reads.assert_tables(&[
                "content_nodes",
                "content_staging",
                "content_nodes",
                "content_staging",
            ]);
            assert_eq!(job.snapshot().wire_bytes, 6 * node.payload());
            assert_eq!(image(stage.c), before, "复用 Stage 不保留旧调用期限");
            no_reservations(&job);
        }
    }
}

#[test]
fn c5_h2_peer_target_return_at_job_deadline_keeps_wire_and_blocks_next_read() {
    let node = Node::scalar(s_content::Value::Null);
    let peer = Db::fresh("deadline-peer");
    let mut target = Db::fresh("deadline-target");
    seed(&peer.c, "content_nodes", &node);
    seed(&target.c, "content_nodes", &node);
    seed(&target.c, "content_staging", &node);
    let before = image(&target.c);
    let mut caps = limits();
    caps.max_elapsed_ms = 1;
    let (mut job, clock) = job(&target.c, &target.path(), caps);
    let source_reads = SqlReads::watch(&peer.c);
    let target_reads = SqlReads::watch(&target.c);
    // 第三次返回才是第一次目标返回；前两次属于源 header/闭包。
    let _hook = expire_after_sql_returns(clock, 3);
    let mut stage = target.read_stage();
    for attempt in 0..2 {
        source_reads.clear();
        target_reads.clear();
        let failure = import_peer_refs_with_deadline(
            &peer.c,
            std::slice::from_ref(&node.root),
            &mut stage,
            &mut job,
            None,
        )
        .unwrap_err();
        assert!(failure.to_string().contains(DEADLINE), "{failure}");
        source_reads.assert_tables(if attempt == 0 {
            &["content_nodes", "content_nodes"]
        } else {
            &[]
        });
        target_reads.assert_tables(if attempt == 0 {
            &["content_nodes"]
        } else {
            &[]
        });
        assert_eq!(job.snapshot().wire_bytes, 3 * node.payload());
        assert_eq!(job.snapshot().stopped, Some(DEADLINE));
        assert_eq!(job.snapshot().nodes, 1);
        assert_eq!(image(stage.c), before);
        no_reservations(&job);
    }
}

#[test]
fn c5_h2_stage_old_single_and_batch_entries_share_target_metering() {
    let nodes = [
        Node::scalar(s_content::Value::Null),
        Node::scalar(s_content::Value::Bool(true)),
    ];
    let mut target = Db::fresh("trait-target");
    seed(&target.c, "content_nodes", &nodes[0]);
    seed(&target.c, "content_staging", &nodes[1]);
    let before = image(&target.c);
    let (mut job, _) = job(&target.c, &target.path(), limits());
    let mut stage = target.read_stage();
    for node in &nodes {
        job.accept_node(&node.digest, node.tag, &node.body).unwrap();
        stage
            .put_budgeted(&mut job, node.digest, node.tag, &node.body)
            .unwrap();
    }
    let payload: u128 = nodes.iter().map(Node::payload).sum();
    assert_eq!(job.snapshot().wire_bytes, payload);
    let mut inserted = Vec::new();
    stage
        .put_batch_budgeted(
            &mut job,
            &nodes.iter().map(Node::tuple).collect::<Vec<_>>(),
            &mut inserted,
        )
        .unwrap();
    assert!(inserted.is_empty());
    assert_eq!(job.snapshot().wire_bytes, payload * 2);
    let node = &nodes[0];
    assert!(stage
        .put_budgeted_with_deadline(
            &mut job,
            node.digest,
            node.tag,
            &node.body,
            Some(Instant::now())
        )
        .is_err());
    assert_eq!(job.snapshot().wire_bytes, payload * 2);
    assert!(matches!(
        stage.read_budgeted(&node.digest, &mut job, None).unwrap(),
        NodeRead::Found { .. }
    ));
    assert_eq!(job.snapshot().wire_bytes, payload * 2 + node.payload());
    assert_eq!(job.snapshot().nodes, 2);
    assert_eq!(image(stage.c), before);
    no_reservations(&job);
}

#[test]
fn c5_h2_cleanup_pragmas_charge_wire_before_limit_error_without_records() {
    let target = Db::fresh("cleanup-pragmas");
    let mut caps = limits();
    caps.max_total_wire_bytes = 8;
    let (mut job, _) = job(&target.c, &target.path(), caps);
    let failure = cleanup_room_metered(
        &target.c,
        &target.path(),
        true,
        &mut ReadBudget::new(Some(&mut job), None),
    )
    .unwrap_err();
    assert!(failure.to_string().contains(LIMIT_EXCEEDED), "{failure}");
    assert_eq!(
        job.snapshot().wire_bytes,
        16,
        "max_page_count 和 page_size 各返回 i64"
    );
    no_reservations(&job);
}

#[test]
fn c5_h2_charged_checkpoint_read_failure_reconciles_staged_and_store_grants() {
    for staged in [false, true] {
        let target = Db::fresh("checkpoint");
        let mut caps = limits();
        caps.max_total_wire_bytes = 1;
        let (mut job, _) = job(&target.c, &target.path(), caps);
        let before = allocated(&target.path()).unwrap();
        let failure = charged_checkpoint_with_budget(
            &target.c,
            &target.path(),
            &mut ReadBudget::new(Some(&mut job), None),
            caps.max_store_bytes,
            staged,
        )
        .unwrap_err();
        let actual = u128::from(allocated(&target.path()).unwrap().saturating_sub(before));
        assert!(failure.to_string().contains(LIMIT_EXCEEDED), "{failure}");
        assert_eq!(job.snapshot().wire_bytes, 24, "checkpoint 返回三个 i64");
        assert_eq!(job.snapshot().stopped, Some(LIMIT_EXCEEDED));
        assert_eq!(
            job.snapshot().store.committed,
            if staged { 0 } else { actual }
        );
        assert_eq!(
            job.snapshot().staged.committed,
            if staged { actual } else { 0 }
        );
        no_reservations(&job);
    }
}

#[test]
#[ignore = "需要独占固定 HFS+ 载体，CLOSURE_CARRIER_DB 指向根下不存在的新库；与其它载体测试分别执行"]
fn c5_h2_real_hfs_checkpoint_failure_keeps_committed_cleanup_list_and_reconciles() {
    let db = PathBuf::from(std::env::var("CLOSURE_CARRIER_DB").unwrap());
    assert!(!db.exists(), "不得覆盖已有载体库");
    let (mut c, _) = store::open(&db, false).unwrap();
    c.execute_batch(store::TEST_DOMAIN_SQL).unwrap();
    c.execute_batch(store::CONTENT_SQL).unwrap();
    c.execute(
        "INSERT INTO meta(k,v)VALUES('schema_revision',?1)",
        [SCHEMA_V2],
    )
    .unwrap();
    c.execute(
        "INSERT INTO content_meta VALUES(1,?1)",
        [s_content::CONTENT_FORMAT],
    )
    .unwrap();
    let mut stage = Stage::new(&mut c, &db, limits(), "c5-hfs".into()).unwrap();
    let node = Node::scalar(s_content::Value::Null);
    // 两个 cleanup PRAGMA = 16；预写 checkpoint = 24；再次 cleanup = 16；
    // 提交后的 checkpoint = 24。计费报错仍核账，并保留已经提交的摘要。
    for (cap, expected_wire, committed) in [(16, 40, false), (56, 80, true)] {
        let mut caps = limits();
        caps.max_total_wire_bytes = cap;
        let (mut job, _) = job(stage.c, &db, caps);
        job.accept_node(&node.digest, node.tag, &node.body).unwrap();
        let before = allocated(&db).unwrap();
        let mut inserted = Vec::new();
        let failure = stage
            .put_batch_budgeted_with_deadline(&mut job, &[node.tuple()], &mut inserted, None)
            .unwrap_err();
        assert!(failure.to_string().contains(LIMIT_EXCEEDED), "{failure}");
        assert_eq!(job.snapshot().wire_bytes, expected_wire);
        assert_eq!(inserted, if committed { vec![node.digest] } else { vec![] });
        assert_eq!(image(stage.c).len(), usize::from(committed));
        let actual = u128::from(allocated(&db).unwrap().saturating_sub(before));
        assert!(job.snapshot().staged.committed >= actual);
        assert_eq!(
            job.snapshot().store.committed,
            0,
            "暂存已进入总敞口，不能再重记 store.committed"
        );
        assert_eq!(
            job.snapshot().store.exposure,
            job.snapshot().store.baseline + job.snapshot().staged.committed
        );
        no_reservations(&job);
        assert_eq!(
            stage.discard_staged(&inserted).unwrap(),
            u64::from(committed)
        );
        assert!(image(stage.c).is_empty(), "停止 Job 不阻断预付失败清理");
        assert_eq!(
            job.snapshot().wire_bytes,
            expected_wire,
            "清理不假造新 Job 或抹累计"
        );
    }
    let other = Node::scalar(s_content::Value::Bool(true));
    let (mut job, _) = job(stage.c, &db, limits());
    for n in [&node, &other] {
        job.accept_node(&n.digest, n.tag, &n.body).unwrap();
    }
    let mut inserted = Vec::new();
    stage
        .put_batch_budgeted(
            &mut job,
            &[node.tuple(), other.tuple(), node.tuple()],
            &mut inserted,
        )
        .unwrap();
    assert_eq!(inserted.len(), 2, "合法不同节点成功，块内重复不新增行");
    assert_eq!(image(stage.c).len(), 2);
    assert_eq!(
        job.snapshot().wire_bytes,
        96,
        "两次 checkpoint 与三组 cleanup 的真实返回载荷"
    );
    let wire = job.snapshot().wire_bytes;
    let mut repeated = Vec::new();
    stage
        .put_batch_budgeted(&mut job, &[node.tuple(), other.tuple()], &mut repeated)
        .unwrap();
    assert!(repeated.is_empty());
    assert_eq!(
        job.snapshot().wire_bytes,
        wire + node.payload() + other.payload()
    );
    assert_eq!(job.snapshot().nodes, 2);
    no_reservations(&job);
    assert_eq!(stage.discard_staged(&inserted).unwrap(), 2);
    assert!(image(stage.c).is_empty());
}
