#![cfg(feature = "s_session")]
//! #1394 C §5.6.1「共用 job 资源累计核心」的边界测试。
//!
//! 验收 seam 是已批准 §5.9 的资源门面（job/reader 限额的记账侧），逐项覆盖：
//! 八字段与数域、既有基线（含纯读取 job）、累计重试耗尽、唯一（含本地复用）节点首次计费、
//! 记录重访、单一绝对期限（假时钟，全程不 sleep）、暂存／存储预留-对账-回收、
//! 超限原子性、reconcile 超限按真实 actual 入账并保留清理通道、句柄跨 job 不消费、
//! 读包装的摘要污染拒绝。
//!
//! 边界（照实）：本文件不覆盖真实 I/O 接线、不覆盖 S/Q/E/B 接入，也不构成 9011 验收；
//! 其中的八字段数值只是测试输入，不是 §5.6.1 要求由冻结内容规模与实测形成的首批数值。

use newchan_rust::s_content::job::{
    Clock, Existing, Job, JobLimits, JobNodeSource, NodeAcceptance, SpaceKind, DEADLINE,
    LIMIT_DOMAIN, LIMIT_EXCEEDED, LIMIT_FIELD, LIMIT_FIELDS, RECLAIM, RECONCILE, RESERVATION,
};
use newchan_rust::s_content::{
    access::MeteredSource, code, frame, frame_calls_on_thread, tag, CodecError, MemoryNodes,
    NodeRead, NodeSink, NodeSource, DOMAIN_SEPARATOR,
};
use serde_json::{json, Value as J};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 显式八字段配置（规范十进制字符串）：exact 八项，无缺省、无无限量。
fn limits_json() -> J {
    json!({
        "max_total_wire_bytes": "4096",
        "max_total_node_bytes": "4096",
        "max_total_nodes": "64",
        "max_total_records": "64",
        "max_total_requests": "16",
        "max_elapsed_ms": "7000",
        "max_staged_bytes": "1024",
        "max_store_bytes": "8192",
    })
}

fn limits() -> JobLimits {
    JobLimits::from_canonical_json(&limits_json()).expect("八字段合法配置")
}

/// 复制基准后只改关心的一项。**不在此处校验域**：越域输入要留给 `Job::new` 拒绝，
/// 只有「八字段与数域」一例故意越域，其余调用都改在 1..=2^63-1 内。
fn limits_with(edit: impl FnOnce(&mut JobLimits)) -> JobLimits {
    let mut limits = limits();
    edit(&mut limits);
    limits
}

fn existing(store_bytes: u64, staged_bytes: u64) -> Existing {
    Existing {
        store_bytes,
        staged_bytes,
    }
}

fn json_code(node: &J) -> &'static str {
    JobLimits::from_canonical_json(node)
        .expect_err("应当拒绝")
        .code()
}

/// 假单调时钟：`store` 毫秒即推进；测试全程不 sleep。
fn mock_clock() -> (Arc<AtomicU64>, Box<dyn Clock>) {
    let ms = Arc::new(AtomicU64::new(0));
    let handle = ms.clone();
    let base = Instant::now();
    let clock: Box<dyn Clock> =
        Box::new(move || base + Duration::from_millis(handle.load(Ordering::SeqCst)));
    (ms, clock)
}

fn try_job(limits: JobLimits, existing: Existing) -> Result<Job, CodecError> {
    let (_ms, clock) = mock_clock();
    Job::new(limits, existing, clock)
}

fn job_with(limits: JobLimits) -> (Arc<AtomicU64>, Job) {
    job_with_existing(limits, existing(200, 0))
}

fn job_with_existing(limits: JobLimits, existing: Existing) -> (Arc<AtomicU64>, Job) {
    let (ms, clock) = mock_clock();
    (ms, Job::new(limits, existing, clock).expect("job 建立"))
}

/// §5.6.1 的节点分帧字节：`len(D) + 1 + 8 + len(body)`。
fn framed(body_len: usize) -> u64 {
    DOMAIN_SEPARATOR.len() as u64 + 1 + 8 + body_len as u64
}

#[test]
fn checked_read_matches_old_acceptance_at_f_deadline_boundary() {
    let digest = frame(tag::NULL, &[]);
    let mut source = MemoryNodes::new();
    source.insert_unchecked(digest, tag::NULL, vec![]);
    for checked in [false, true] {
        for metered in [false, true] {
            let before = frame_calls_on_thread();
            let now = Instant::now();
            let mut job = Job::new(
                limits_with(|limits| limits.max_elapsed_ms = 1),
                existing(200, 0),
                Box::new(move || {
                    now + Duration::from_millis(u64::from(frame_calls_on_thread() > before))
                }),
            ).unwrap();
            let outcome = if metered {
                let mut reader = MeteredSource::new(&mut job, &mut source);
                if checked { reader.checked_read(&digest).map(|_| ()) }
                else { reader.read(&digest).map(|_| ()) }
            } else {
                let mut reader = JobNodeSource::new(&mut job, &mut source);
                if checked { reader.checked_read(&digest).map(|_| ()) }
                else { reader.read(&digest).map(|_| ()) }
            };
            assert_eq!(frame_calls_on_thread() - before, 1, "每次读取只核一次 F");
            let snapshot = job.snapshot();
            assert_eq!(snapshot.nodes, 1, "旧读与已核读都须在 F 后保留计账");
            assert_eq!(snapshot.node_bytes, framed(0));
            if metered {
                assert_eq!(outcome.unwrap_err().code(), DEADLINE, "post precheck 旧行为");
                assert_eq!(snapshot.stopped, Some(DEADLINE));
            } else {
                outcome.unwrap();
                assert_eq!(snapshot.stopped, None, "F 后不可多加一次期限门");
            }
        }
    }
}

#[test]
fn limits_are_exact_eight_canonical_decimal_strings() {
    let limits = limits();
    assert_eq!(limits.entries().len(), 8);
    assert_eq!(limits.max_elapsed_ms, 7000);
    assert_eq!(limits.max_store_bytes, 8192);

    // 缺任何一项都必须拒绝：没有缺省路径。
    for field in LIMIT_FIELDS {
        let mut broken = limits_json();
        broken.as_object_mut().expect("对象").remove(field);
        assert_eq!(json_code(&broken), LIMIT_FIELD, "缺 `{field}` 必须拒绝");
    }
    // 多字段、JSON 数字、非对象同样不 exact。
    let mut extra = limits_json();
    extra
        .as_object_mut()
        .expect("对象")
        .insert("max_atomic_batch_bytes".to_string(), json!("1"));
    assert_eq!(json_code(&extra), LIMIT_FIELD);
    let mut numeric = limits_json();
    numeric
        .as_object_mut()
        .expect("对象")
        .insert("max_total_nodes".to_string(), json!(64));
    assert_eq!(json_code(&numeric), LIMIT_FIELD);
    assert_eq!(json_code(&json!(["max_total_nodes"])), LIMIT_FIELD);

    // 数域 1..=2^63-1：0／前导零／符号／空白／小数／越界全部拒绝。
    for bad in [
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "",
        "1.0",
        "9223372036854775808",
        "99999999999999999999999999",
    ] {
        let mut node = limits_json();
        node.as_object_mut()
            .expect("对象")
            .insert("max_total_wire_bytes".to_string(), json!(bad));
        assert_eq!(json_code(&node), LIMIT_DOMAIN, "`{bad}` 必须拒绝");
    }
    let mut top = limits_json();
    top.as_object_mut().expect("对象").insert(
        "max_total_wire_bytes".to_string(),
        json!("9223372036854775807"),
    );
    assert_eq!(
        JobLimits::from_canonical_json(&top)
            .expect("2^63-1 合法")
            .max_total_wire_bytes,
        9_223_372_036_854_775_807
    );

    // 内存形态也要过同一关：Job::new 拒绝零值与越界值。
    assert_eq!(
        try_job(limits_with(|l| l.max_total_nodes = 0), existing(0, 0))
            .expect_err("0 不在数域")
            .code(),
        LIMIT_DOMAIN
    );
    assert_eq!(
        try_job(limits_with(|l| l.max_elapsed_ms = u64::MAX), existing(0, 0))
            .expect_err("越过 2^63-1")
            .code(),
        LIMIT_DOMAIN
    );
}

#[test]
fn existing_baselines_are_mandatory_and_bounded_at_construction() {
    // 暂存是总存储的子集：暂存基线大于存储基线直接拒绝。
    assert_eq!(
        try_job(limits(), existing(100, 101))
            .expect_err("子集关系不成立")
            .code(),
        LIMIT_DOMAIN
    );
    // 任一基线超自己的上限即拒绝：不能先建 job 再补测。
    assert_eq!(
        try_job(
            limits_with(|l| l.max_staged_bytes = 100),
            existing(1000, 101)
        )
        .expect_err("暂存基线超上限")
        .code(),
        LIMIT_EXCEEDED
    );
    assert_eq!(
        try_job(limits_with(|l| l.max_store_bytes = 100), existing(101, 0))
            .expect_err("存储基线超上限")
            .code(),
        LIMIT_EXCEEDED
    );

    // 纯读取 job：一次 reserve 都不发，既存存储也已经进账。
    let (_ms, job) = job_with_existing(limits(), existing(3000, 120));
    let snap = job.snapshot();
    assert_eq!(snap.store.baseline, 3000);
    assert_eq!(snap.store.exposure, 3000, "纯读取也受 max_store_bytes 约束");
    assert_eq!(snap.staged.exposure, 120);
    assert_eq!(snap.open_reservations, 0);
    assert_eq!(snap.stopped, None);
    assert_eq!(snap.wire_bytes, 0);
}

#[test]
fn wire_and_requests_count_every_retry_and_exhaust_the_aggregate() {
    let (_ms, mut job) = job_with(limits_with(|l| {
        l.max_total_wire_bytes = 1000;
        l.max_total_requests = 2;
    }));
    job.begin_first_read().expect("首次读取");
    job.charge_requests(1).expect("首次请求");
    job.charge_requests(1).expect("重试也是一次尝试");
    job.charge_wire_bytes(600).expect("首片实际读写");
    job.charge_wire_bytes(400).expect("同片重传照实再计");
    assert_eq!(job.snapshot().wire_bytes, 1000);

    let err = job.charge_wire_bytes(1).unwrap_err();
    assert_eq!(err.code(), LIMIT_EXCEEDED);
    let snap = job.snapshot();
    assert_eq!(
        snap.wire_bytes, 1001,
        "§5.6.1 实读后计费：越限那一次已经发生的字节照实入账，再停止"
    );
    assert_eq!(snap.requests, 2);
    assert_eq!(snap.stopped, Some(LIMIT_EXCEEDED));
    assert_eq!(
        job.charge_requests(1).unwrap_err().code(),
        LIMIT_EXCEEDED,
        "停止后不能继续借超限"
    );
    assert_eq!(job.snapshot().requests, 2);
    assert_eq!(job.snapshot().wire_bytes, 1001, "停止后不再接纳新字节");
}

#[test]
fn unique_local_reuse_counts_first_node_bytes_once() {
    let body = b"local-cached-value".to_vec();
    let digest = frame(tag::STRING, &body);
    let mut nodes = MemoryNodes::new();
    nodes.put(digest, tag::STRING, &body).expect("本地缓存写入");
    let (_ms, mut job) = job_with(limits());
    {
        let mut source = JobNodeSource::new(&mut job, &mut nodes);
        // 本地复用：wire 上一个字节都没有，但首次接纳仍占 node_bytes。
        assert!(matches!(
            source.read(&digest).expect("本地读"),
            NodeRead::Found { .. }
        ));
        assert!(matches!(
            source.read(&digest).expect("重复本地读"),
            NodeRead::Found { .. }
        ));
    }
    assert_eq!(
        job.accept_node(&digest, tag::STRING, &body)
            .expect("同一根再接纳"),
        NodeAcceptance::AlreadyAccepted
    );
    let snap = job.snapshot();
    assert_eq!(snap.nodes, 1, "同一根只计一次");
    assert_eq!(snap.node_bytes, framed(body.len()));
    assert_eq!(snap.wire_bytes, 0, "本地复用不进 wire 账");
}

#[test]
fn records_count_every_revisit_including_revalidation() {
    let (_ms, mut job) = job_with(limits_with(|l| l.max_total_records = 5));
    job.charge_records(3).expect("首次访问");
    job.charge_records(2).expect("同批记录重访照实再计");
    assert_eq!(job.snapshot().records, 5);
    assert_eq!(job.charge_records(1).unwrap_err().code(), LIMIT_EXCEEDED);
    assert_eq!(job.snapshot().records, 5, "失败那次不进账");
}

#[test]
fn deadline_is_absolute_from_first_read_and_never_renewed() {
    let (ms, mut job) = job_with(limits_with(|l| l.max_elapsed_ms = 7_000));
    job.begin_first_read().expect("首次读取");
    ms.store(6_999, Ordering::SeqCst);
    job.charge_wire_bytes(10).expect("期限内的分片");
    // 逐片段调用不续期：再调一次只是重申同一起点。
    job.begin_first_read().expect("幂等");
    assert_eq!(job.snapshot().elapsed_ms, Some(6_999));

    ms.store(7_000, Ordering::SeqCst);
    // §5.6.1 实读后计费：已经发生的物理字节先照实入账，再按绝对期限停止。
    assert_eq!(job.charge_wire_bytes(10).unwrap_err().code(), DEADLINE);
    let snap = job.snapshot();
    assert_eq!(snap.wire_bytes, 20, "越期那一刻的实收字节已先入账");
    assert_eq!(snap.elapsed_ms, Some(7_000));
    assert_eq!(snap.stopped, Some(DEADLINE));
    assert_eq!(job.begin_first_read().unwrap_err().code(), DEADLINE);
    assert_eq!(job.check_deadline().unwrap_err().code(), DEADLINE);
    assert_eq!(job.charge_wire_bytes(999).unwrap_err().code(), DEADLINE);
    assert_eq!(job.snapshot().wire_bytes, 20, "停止后不再接纳新字节");
    assert_eq!(job.finish_check().unwrap_err().code(), DEADLINE);
}

/// §5.6.1 的绝对期限取用口与真正 I/O 之前的门：起钟一次、不续期、过期就地停止。
#[test]
fn first_read_deadline_is_fixed_and_pre_io_check_gates_before_work() {
    let (ms, mut job) = job_with(limits_with(|l| l.max_elapsed_ms = 1_000));
    let first = job
        .first_read_deadline()
        .expect("首次读取起钟并给出绝对期限");
    assert!(job.snapshot().started, "取绝对期限即起钟");
    ms.store(500, Ordering::SeqCst);
    assert_eq!(
        job.first_read_deadline().expect("重复调用不续期"),
        first,
        "同一 job 的绝对期限不因再次取用而续期"
    );
    job.pre_io_check().expect("期限内尚未停止");

    // 期限走到：真正 I/O 之前的门就地停止并报 job_deadline。
    ms.store(1_000, Ordering::SeqCst);
    assert_eq!(job.pre_io_check().unwrap_err().code(), DEADLINE);
    assert_eq!(job.snapshot().stopped, Some(DEADLINE));
    assert_eq!(job.first_read_deadline().unwrap_err().code(), DEADLINE);
    assert_eq!(
        job.pre_io_check().unwrap_err().code(),
        DEADLINE,
        "已停止的 job 原样报停止原因"
    );

    // 极端 max_elapsed_ms 不 panic：要么给出期限，要么以 job_deadline 拒绝并停止。
    let (_ms2, mut extreme) = job_with(limits_with(|l| {
        l.max_elapsed_ms = 9_223_372_036_854_775_807
    }));
    match extreme.first_read_deadline() {
        Ok(deadline) => assert!(deadline > Instant::now()),
        Err(err) => {
            assert_eq!(err.code(), DEADLINE);
            assert_eq!(extreme.snapshot().stopped, Some(DEADLINE));
        }
    }
}

#[test]
fn staged_reservation_occupies_both_limits_atomically() {
    let limits = limits_with(|l| {
        l.max_staged_bytes = 1000;
        l.max_store_bytes = 1200;
    });
    let (_ms, mut job) = job_with_existing(limits, existing(500, 0));
    job.begin_first_read().expect("首次读取");

    let staged = job.reserve_staged(600).expect("两本账都还有额度");
    assert_eq!(staged.kind(), SpaceKind::Staged);
    assert_eq!(staged.requested(), 600);
    let snap = job.snapshot();
    assert_eq!((snap.staged.pending, snap.staged.exposure), (600, 600));
    assert_eq!(snap.staged.peak, 600);
    assert_eq!(snap.store.exposure, 1_100, "同一批暂存字节也占总账");
    assert_eq!(snap.store.peak, 1_100);

    // 暂存账还剩 400，总账只剩 100：一次预留核两本 cap，失败原子。
    let before = job.snapshot();
    assert_eq!(job.reserve_staged(200).unwrap_err().code(), LIMIT_EXCEEDED);
    let mut expected = before.clone();
    expected.stopped = Some(LIMIT_EXCEEDED);
    assert_eq!(job.snapshot(), expected, "失败不得留下部分计费状态");
    assert_eq!(job.reserve_staged(1).unwrap_err().code(), LIMIT_EXCEEDED);
    assert_eq!(job.snapshot().store.exposure, 1_100);

    // 非暂存写入只占总账，不占暂存账。
    let (_ms2, mut read_job) = job_with_existing(limits, existing(500, 0));
    let store_only = read_job.reserve_store(700).expect("总账 500+700");
    assert_eq!(store_only.kind(), SpaceKind::Store);
    let snap = read_job.snapshot();
    assert_eq!(snap.staged.exposure, 0);
    assert_eq!(snap.store.exposure, 1_200);
}

#[test]
fn staged_reconcile_reclaim_and_release_never_zero_cumulative() {
    let limits = limits_with(|l| {
        l.max_staged_bytes = 1000;
        l.max_store_bytes = 1200;
    });
    let (_ms, mut job) = job_with_existing(limits, existing(500, 100));
    job.begin_first_read().expect("首次读取");
    job.charge_wire_bytes(64).expect("wire 累计");
    job.charge_records(2).expect("records 累计");

    // 写前预留 400（最坏量占两本账），写后按实际 120 对账。
    let grant = job.reserve_staged(400).expect("两本账都够");
    let snap = job.snapshot();
    assert_eq!((snap.staged.exposure, snap.store.exposure), (500, 900));
    job.reconcile(&grant, 120).expect("按实际对账");
    let snap = job.snapshot();
    assert_eq!((snap.staged.pending, snap.staged.committed), (0, 120));
    assert_eq!(snap.staged.exposure, 220);
    assert_eq!(snap.staged.peak, 500, "峰值按写前最坏量");
    assert_eq!(snap.store.exposure, 620, "总账含已对账暂存");

    // 取消路径一：未决预留 release，只退出预留。
    let grant = job.reserve_staged(500).expect("两本账都够");
    assert_eq!(job.snapshot().store.exposure, 1_120);
    job.release(&grant).expect("释放未决预留");
    let snap = job.snapshot();
    assert_eq!(snap.staged.pending, 0);
    assert_eq!(snap.staged.committed, 120, "释放不动已对账量");
    assert_eq!(snap.store.exposure, 620);

    // 取消路径二：已对账暂存按实际删除量回收。
    job.reclaim_staged(20).expect("实际删了 20 字节");
    let snap = job.snapshot();
    assert_eq!(snap.staged.committed, 100);
    // 峰值是历史最大值（前面那笔 500 字节预留把暂存峰值推到 720），回收不回退它。
    assert_eq!(snap.staged.peak, 720, "峰值不回退");
    assert_eq!(snap.store.exposure, 600);
    assert_eq!(
        job.reclaim_staged(101).unwrap_err().code(),
        RECLAIM,
        "不得越界回收成历史内容删除"
    );
    let snap = job.snapshot();
    assert_eq!(snap.staged.committed, 100);
    assert_eq!(snap.wire_bytes, 64, "回收不得归零累计 wire");
    assert_eq!(snap.records, 2, "回收不得归零累计 records");
    assert_eq!(snap.open_reservations, 0);
    job.finish_check().expect("无未决预留、未停止、未过期");
}

#[test]
fn after_stop_only_deallocation_paths_still_work() {
    let (_ms, mut job) = job_with(limits_with(|l| l.max_total_wire_bytes = 10));
    job.begin_first_read().expect("首次读取");
    let grant = job.reserve_staged(100).expect("预留与 wire 无关");
    assert_eq!(
        job.charge_wire_bytes(11).unwrap_err().code(),
        LIMIT_EXCEEDED
    );
    assert_eq!(job.snapshot().stopped, Some(LIMIT_EXCEEDED));
    assert_eq!(job.charge_records(1).unwrap_err().code(), LIMIT_EXCEEDED);
    assert_eq!(job.reserve_staged(1).unwrap_err().code(), LIMIT_EXCEEDED);
    job.reconcile(&grant, 40).expect("停止后仍可对账已开预留");
    let snap = job.snapshot();
    assert_eq!(snap.staged.committed, 40);
    assert_eq!(
        snap.wire_bytes, 11,
        "越限的尾字节照实入账（停止在记录之后）"
    );
}

#[test]
fn finish_check_and_foreign_grant_are_rejected() {
    let (_ms, mut job) = job_with(limits());
    job.begin_first_read().expect("首次读取");
    job.finish_check().expect("无预留即可收尾");
    let grant = job.reserve_staged(10).expect("预留");
    assert_eq!(job.finish_check().unwrap_err().code(), RESERVATION);
    job.release(&grant).expect("释放");
    job.finish_check().expect("释放后可以收尾");

    let grant = job.reserve_staged(10).expect("再一次预留");
    let (_ms2, mut other) = job_with(limits());
    assert_eq!(other.reconcile(&grant, 1).unwrap_err().code(), RESERVATION);
    assert_eq!(other.snapshot().stopped, None, "跨 job 句柄不停止对方");
}

#[test]
fn read_wrapper_rejects_unverified_frames_and_bad_repeats() {
    let good = frame(tag::STRING, b"good");
    let bad = frame(tag::STRING, b"bad");
    let mut nodes = MemoryNodes::new();
    nodes.put(good, tag::STRING, b"good").expect("正常节点");
    // 污染：按 `bad` 的摘要放入别的 body（未核 sha 的节点）。
    nodes.insert_unchecked(bad, tag::STRING, b"evil".to_vec());
    let (_ms, mut job) = job_with(limits());
    {
        let mut source = JobNodeSource::new(&mut job, &mut nodes);
        assert_eq!(source.read(&bad).unwrap_err().code(), code::NODE_DIGEST);
        // 坏重复也不能靠「已计过」绕账：第二次仍拒绝。
        assert_eq!(source.read(&bad).unwrap_err().code(), code::NODE_DIGEST);
        assert!(matches!(
            source.read(&good).expect("正常节点"),
            NodeRead::Found { .. }
        ));
        assert!(matches!(
            source.read(&[0u8; 32]).expect("缺失"),
            NodeRead::Missing
        ));
    }
    let snap = job.snapshot();
    assert_eq!(snap.nodes, 1, "只有核过的根进入已计费集合");
    assert_eq!(snap.node_bytes, framed(b"good".len()));
}

#[test]
fn first_charge_and_first_accept_start_the_clock_once() {
    // 只用公共 charge 口（不调 begin_first_read）也必须起钟，且重试不续期。
    let (ms, mut job) = job_with(limits_with(|l| l.max_elapsed_ms = 7_000));
    ms.store(1_000, Ordering::SeqCst);
    job.charge_records(1).expect("首次 charge 即起钟");
    let snap = job.snapshot();
    assert!(snap.started);
    assert_eq!(snap.elapsed_ms, Some(0), "起点是首次 charge 那一刻");
    ms.store(5_000, Ordering::SeqCst);
    job.charge_wire_bytes(1).expect("重试");
    assert_eq!(job.snapshot().elapsed_ms, Some(4_000), "重试不续期");
    ms.store(8_000, Ordering::SeqCst);
    assert_eq!(job.charge_records(1).unwrap_err().code(), DEADLINE);

    // accept_node 同样起钟；同一根重复接纳仍照走期限门，且不重复计费。
    let (ms2, mut other) = job_with(limits_with(|l| l.max_elapsed_ms = 100));
    ms2.store(50, Ordering::SeqCst);
    let body = b"late".to_vec();
    let digest = frame(tag::STRING, &body);
    other
        .accept_node(&digest, tag::STRING, &body)
        .expect("首次 accept 即起钟");
    assert!(other.snapshot().started);
    ms2.store(150, Ordering::SeqCst);
    assert_eq!(
        other
            .accept_node(&digest, tag::STRING, &body)
            .unwrap_err()
            .code(),
        DEADLINE,
        "重复接纳不续期也不免期限"
    );
    assert_eq!(other.snapshot().nodes, 1, "重复接纳只计一次");
}

#[test]
fn nonzero_baseline_pure_read_peak_equals_baseline() {
    // 纯读取 job：一次 charge／accept／reserve 都不发，峰值也必须是实测基线本身，
    // 不能把既存占用记成 0。
    let (_ms, job) = job_with_existing(limits(), existing(3000, 120));
    let snap = job.snapshot();
    assert_eq!(snap.store.exposure, 3000);
    assert_eq!(snap.staged.exposure, 120);
    assert_eq!(snap.store.peak, 3000, "存储峰值即实测基线");
    assert_eq!(snap.staged.peak, 120, "暂存峰值即实测基线");
}

#[test]
fn overshoot_records_real_actual_stops_and_allows_cleanup() {
    let limits = limits_with(|l| {
        l.max_staged_bytes = 1000;
        l.max_store_bytes = 1200;
    });
    let (_ms, mut job) = job_with_existing(limits, existing(500, 100));
    job.begin_first_read().expect("首次读取");
    job.charge_wire_bytes(64).expect("wire 累计");
    let grant = job.reserve_staged(100).expect("写前预留 100");

    // 写后实际 400 超过预留 100：不足即停止，但账面必须反映真实已分配量。
    assert_eq!(job.reconcile(&grant, 400).unwrap_err().code(), RECONCILE);
    let snap = job.snapshot();
    assert_eq!((snap.staged.pending, snap.staged.committed), (0, 400));
    assert_eq!(snap.staged.exposure, 500);
    assert_eq!(snap.store.exposure, 900);
    assert_eq!(snap.open_reservations, 0);
    assert_eq!(snap.stopped, Some(RECONCILE));
    assert_eq!(
        job.reconcile(&grant, 400).unwrap_err().code(),
        RESERVATION,
        "该 grant 已闭合，不能重复对账"
    );
    assert_eq!(
        job.reserve_staged(1).unwrap_err().code(),
        RECONCILE,
        "停止后不再借出"
    );

    // 清理走已对账暂存的实际删除口。
    job.reclaim_staged(400).expect("确已删除 400 字节");
    let snap = job.snapshot();
    assert_eq!(snap.staged.committed, 0);
    assert_eq!(snap.staged.peak, 500, "峰值不回退");
    assert_eq!(snap.store.exposure, 500);
    assert_eq!(snap.wire_bytes, 64, "回收不动累计 wire");
}

#[test]
fn overshoot_actual_at_u64_max_is_recorded_without_panic_or_understating() {
    // 暂存基线取 150（≠0）：真实 actual 与基线相加会越过 u64，正是内部账用 u128 的原因。
    let (_ms, mut job) = job_with_existing(limits(), existing(200, 150));
    let grant = job.reserve_staged(1).expect("写前预留 1");
    // 实际量取 u64 上限：真实值必须入账，既不能溢出 panic，也不能低估成 0。
    assert_eq!(
        job.reconcile(&grant, u64::MAX).unwrap_err().code(),
        RECONCILE
    );
    let snap = job.snapshot();
    assert_eq!(snap.staged.committed, u128::from(u64::MAX));
    assert_eq!(
        snap.staged.exposure,
        u128::from(150u64) + u128::from(u64::MAX)
    );
    assert!(snap.store.exposure >= snap.staged.exposure);
    assert_eq!(snap.stopped, Some(RECONCILE));

    job.reclaim_staged(u64::MAX).expect("按实际删除字节回收");
    assert_eq!(job.snapshot().staged.committed, 0);
}

#[test]
fn foreign_job_call_does_not_consume_the_token() {
    let (_ms, mut job) = job_with(limits());
    let (_ms2, mut other) = job_with(limits());
    let grant = job.reserve_staged(10).expect("本 job 的预留");

    // 错传别的 job：只报错，不消费令牌，也不停止对方。
    assert_eq!(other.reconcile(&grant, 1).unwrap_err().code(), RESERVATION);
    assert_eq!(other.release(&grant).unwrap_err().code(), RESERVATION);
    assert_eq!(other.snapshot().stopped, None, "错传不停止对方 job");
    assert_eq!(other.snapshot().open_reservations, 0);

    // owner 手里仍握着同一个句柄，可以正常收尾。
    job.reconcile(&grant, 10).expect("原 owner 收尾");
    let snap = job.snapshot();
    assert_eq!(snap.open_reservations, 0);
    assert_eq!(snap.staged.committed, 10);
    assert_eq!(
        job.release(&grant).unwrap_err().code(),
        RESERVATION,
        "每个 grant 最多关闭一次"
    );
}
