//! #1527 31Q/31V TestOnly：N1 受限候选的业务事务对拍、原 Job 审计与 syscall 反例锁。临时文件、SQLite 与独立
//! `NativeHooks` 实例；不安装进全局 aSyscall，不触目标卷、S0 或生产入口。
use super::*;
use newchan_rust::s_content::job::{self as job, Existing, JobLimits};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" fn open3(p: *const c_char, f: c_int, m: c_int) -> c_int {
    libc::open(p, f, m as libc::c_uint)
}

fn libc_orig() -> Orig {
    Orig {
        open: open3,
        close: libc::close,
        fstat: libc::fstat,
        stat: libc::stat,
        read: libc::read,
        pread: Some(libc::pread),
        pread64: None,
        write: libc::write,
        pwrite: Some(libc::pwrite),
        pwrite64: None,
        ftruncate: libc::ftruncate,
        mmap: Some(libc::mmap),
        unlink: libc::unlink,
        fchmod: Some(libc::fchmod),
        fchown: Some(libc::fchown),
    }
}

fn hooks() -> &'static NativeHooks {
    Box::leak(Box::new(NativeHooks::new(libc_orig())))
}

struct Temp(PathBuf);
impl Temp {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "issue1527-31q-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, b"issue1527 31Q TestOnly").unwrap();
        Temp(path)
    }
    fn c(&self) -> CString {
        CString::new(self.0.as_os_str().as_bytes()).unwrap()
    }
    fn id(&self) -> DevIno {
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::stat(self.c().as_ptr(), &mut st) }, 0);
        ident(&st)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_lock(fd: c_int) {
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_type = libc::F_WRLCK as _;
    lock.l_whence = libc::SEEK_SET as _;
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETLK, &lock) }, 0, "owner 取得写锁");
}

/// 另一进程尝试同一 POSIX 记录锁：true = 取得（即本进程的锁已丢）。
fn other_process_can_lock(path: &Path) -> bool {
    let status = std::process::Command::new("python3")
        .args([
            "-c",
            "import fcntl,sys\nf=open(sys.argv[1],'r+')\ntry:\n fcntl.lockf(f,fcntl.LOCK_EX|fcntl.LOCK_NB)\nexcept OSError:\n sys.exit(3)\n",
        ])
        .arg(path)
        .status()
        .expect("python3 可用（ubuntu runner 自带）");
    match status.code() {
        Some(0) => true,
        Some(3) => false,
        other => panic!("锁探针异常退出 {other:?}"),
    }
}

fn fd_open(fd: c_int) -> bool {
    unsafe { libc::fcntl(fd, libc::F_GETFD) != -1 }
}

fn limits(elapsed_ms: u64, store: u64) -> JobLimits {
    JobLimits {
        max_total_wire_bytes: 1 << 20,
        max_total_node_bytes: 1 << 20,
        max_total_nodes: 1000,
        max_total_records: 1000,
        max_total_requests: 1000,
        max_elapsed_ms: elapsed_ms,
        max_staged_bytes: 1 << 20,
        max_store_bytes: store,
    }
}

struct Fake(Rc<Cell<Instant>>);
impl job::Clock for Fake {
    fn now(&self) -> Instant {
        self.0.get()
    }
}

fn gate_with(elapsed_ms: u64, store: u64) -> (NativeIoGate, Rc<RefCell<job::Job>>, Rc<Cell<Instant>>) {
    let now = Rc::new(Cell::new(Instant::now()));
    let j = job::Job::new(
        limits(elapsed_ms, store),
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(Fake(Rc::clone(&now))),
    )
    .unwrap();
    let j = Rc::new(RefCell::new(j));
    (NativeIoGate::for_test(Rc::clone(&j)), j, now)
}

fn identity_bound() -> Option<AllocationBound> {
    Some(AllocationBound::new(|len| len))
}

// ------------------------------------------------------------ G1 非 owner FD 与登记

#[test]
fn native_n1_l1a_deferred_close_keeps_owner_posix_lock_while_plain_close_drops_it() {
    let file = Temp::new("lock");
    let owner_fd = unsafe { libc::open(file.c().as_ptr(), libc::O_RDWR) };
    assert!(owner_fd >= 0);
    write_lock(owner_fd);
    assert!(!other_process_can_lock(&file.0), "基线：owner 锁生效");

    let h = hooks();
    let token = h.begin_for_test([file.id(), (0, 1), (0, 2)]).unwrap();
    let peer = unsafe { libc::open(file.c().as_ptr(), libc::O_RDWR) };
    assert!(peer >= 0);
    assert_eq!(unsafe { h.close(peer) }, 0, "被拒 FD 的 close 不报错给调用者");
    assert!(fd_open(peer), "钩子未真实 close 指向 pin inode 的 FD");
    assert!(h.deferred_for_test().contains(&peer));
    assert!(!other_process_can_lock(&file.0), "31M 反例关闭：owner 锁在被拒 close 后仍在");

    let deferred = h.lock().end(token);
    assert_eq!(deferred, vec![peer]);
    assert_eq!(unsafe { libc::close(peer) }, 0);
    assert!(other_process_can_lock(&file.0), "对照：真实 close 同 inode 任一 FD 即释放 owner 锁");
    unsafe { libc::close(owner_fd) };
}

#[test]
fn native_n1_l1a_peer_thread_open_of_pin_is_refused_before_any_fd_exists() {
    let file = Temp::new("prestat");
    let h = hooks();
    let token = h.begin_for_test([file.id(), (0, 1), (0, 2)]).unwrap();
    let path = file.c();
    let (rc, err) = std::thread::spawn(move || {
        let rc = unsafe { h.open(path.as_ptr(), libc::O_RDWR, 0) };
        (rc, errno())
    })
    .join()
    .unwrap();
    assert_eq!((rc, err), (-1, libc::EACCES));
    let reg = h.lock();
    let active = reg.active_for(token).unwrap();
    assert_eq!(active.peer_error, Some(NativeCode::InProcessPeer));
    assert!(active.deferred.is_empty(), "打开前拒绝：未产生 FD");
    assert_eq!(reg.inflight_open, 0);
}

#[test]
fn native_n1_l1a_owner_thread_open_outside_main_scope_is_not_a_receipt() {
    let file = Temp::new("scope");
    let h = hooks();
    let token = h.begin_for_test([file.id(), (0, 1), (0, 2)]).unwrap();
    let rc = unsafe { h.open(file.c().as_ptr(), libc::O_RDWR, 0) };
    assert_eq!((rc, errno()), (-1, libc::EACCES));
    assert!(h.lock().active_for(token).unwrap().receipts.is_empty());
}

#[test]
fn native_n1_l1b_begin_covers_inflight_and_closing_not_only_fds() {
    let h = hooks();
    assert_eq!(h.note_open_for_test(), 1);
    assert_eq!(h.begin_for_test([(9, 9), (0, 1), (0, 2)]), Err(NativeCode::InProcessPeer));
    h.finish_open_for_test(77, Some((5, 5)));
    assert_eq!(h.lock().inflight_open, 0);

    h.lock().closing.insert(78, FdEntry::Known((5, 6)));
    assert_eq!(
        h.begin_for_test([(9, 9), (0, 1), (0, 2)]),
        Err(NativeCode::InProcessPeer),
        "正在 close 的 FD 可能仍持 pin inode 锁"
    );
    h.lock().closing.clear();

    h.lock().fds.insert(79, FdEntry::Known((9, 9)));
    assert_eq!(h.begin_for_test([(9, 9), (0, 1), (0, 2)]), Err(NativeCode::InProcessPeer));
    h.lock().fds.insert(79, FdEntry::Unknown);
    assert_eq!(h.begin_for_test([(9, 9), (0, 1), (0, 2)]), Err(NativeCode::TrackingLost));
    h.lock().fds.remove(&79);
    assert!(h.begin_for_test([(9, 9), (0, 1), (0, 2)]).is_ok());
}

#[test]
fn native_n1_l1b_recheck_requires_own_precheck_receipts_closed() {
    let mut reg = FdRegistry::default();
    let token = reg.begin(std::thread::current().id(), [(9, 9), (0, 1), (0, 2)]).unwrap();
    reg.active.as_mut().unwrap().receipts.insert(40, Class::Main);
    reg.fds.insert(40, FdEntry::Known((9, 9)));
    assert_eq!(reg.recheck(token), Err(NativeCode::InProcessPeer));
    reg.fds.remove(&40);
    reg.active.as_mut().unwrap().receipts.clear();
    assert_eq!(reg.recheck(token), Ok(()));
}

#[test]
fn native_n1_l1c_no_owner_open_close_keep_original_result_and_errno() {
    let h = hooks();
    let missing = CString::new("/nonexistent/issue1527-31q/x").unwrap();
    let direct = unsafe { libc::open(missing.as_ptr(), libc::O_RDONLY) };
    let direct_errno = errno();
    let hooked = unsafe { h.open(missing.as_ptr(), libc::O_RDONLY, 0) };
    assert_eq!((hooked, errno()), (direct, direct_errno));

    let file = Temp::new("plain");
    let fd = unsafe { h.open(file.c().as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o600) };
    assert!(fd >= 0, "无 owner 时 O_CREAT 等 flags 原样透传");
    assert_eq!(h.lock().fds.get(&fd), Some(&FdEntry::Known(file.id())));
    assert_eq!(unsafe { h.close(fd) }, 0);
    assert!(!fd_open(fd), "无 owner：真实 close");
    {
        let reg = h.lock();
        assert!(reg.fds.is_empty() && reg.closing.is_empty());
    }
    set_errno(0);
    assert_eq!(unsafe { h.close(-1) }, -1);
    assert_eq!(errno(), libc::EBADF, "失败 close 的 errno 原样");
    assert!(h.lock().tracking_lost, "close 结果失败：后续 N1 不可 begin");
}

#[test]
fn native_n1_l1c_poisoned_registry_while_active_still_defers_pin_close() {
    let file = Temp::new("poison");
    let h = hooks();
    let token = h.begin_for_test([file.id(), (0, 1), (0, 2)]).unwrap();
    h.active_any.store(true, Ordering::SeqCst);
    let peer = unsafe { libc::open(file.c().as_ptr(), libc::O_RDWR) };
    h.poison_for_test();
    assert_eq!(unsafe { h.close(peer) }, 0);
    assert!(fd_open(peer), "中毒不改道为直调原 close");
    assert!(h.unavailable.load(Ordering::SeqCst));
    {
        let reg = h.lock();
        let active = reg.active_for(token).unwrap();
        assert_eq!(active.peer_error, Some(NativeCode::Unavailable));
        assert!(active.deferred.contains(&peer));
    }
    let _ = h.lock().end(token);
    assert_eq!(h.begin_for_test([(1, 1), (0, 1), (0, 2)]), Err(NativeCode::Unavailable));
    unsafe { libc::close(peer) };
}

#[test]
fn native_n1_l1e_non_pin_close_during_active_passes_through() {
    let file = Temp::new("nonpin");
    let h = hooks();
    let fd = unsafe { h.open(file.c().as_ptr(), libc::O_RDWR, 0) };
    assert!(fd >= 0);
    let _token = h.begin_for_test([(1, 1), (0, 1), (0, 2)]).unwrap();
    assert_eq!(unsafe { h.close(fd) }, 0);
    assert!(!fd_open(fd), "可证明非 pin 的 FD 照常关闭");
    assert!(h.deferred_for_test().is_empty());
}

// ------------------------------------------------------------ G2 SHM 预算与写门

#[test]
fn native_n1_l2a_cover_reserves_before_any_writable_effect_regardless_of_bextend() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let mut budget = ShmBudget::new(0, identity_bound()).unwrap();
    budget.cover(&gate, 32_768).unwrap();
    assert_eq!(budget.grant_count(), 1);
    assert_eq!(j.borrow().snapshot().store.pending, 32_768);
    budget.cover(&gate, 32_768).unwrap();
    budget.cover(&gate, 16_384).unwrap();
    assert_eq!(budget.grant_count(), 1, "L2-g 重复/更小映射零新预留");
    budget.mark_delegated();
    budget.settle(&gate, Ok(0)).unwrap();
    let snap = j.borrow().snapshot();
    assert_eq!(snap.store.committed, 32_768, "稀疏/延迟分配：按保守量入账，不按 fstat 实测 0 退还");
    assert_eq!(snap.open_reservations, 0);
}

#[test]
fn native_n1_l2b_existing_b0_only_charges_growth_beyond_floor() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let mut budget = ShmBudget::new(8_192, identity_bound()).unwrap();
    budget.cover(&gate, 32_768).unwrap();
    assert_eq!(j.borrow().snapshot().store.pending, 32_768 - 8_192);
}

#[test]
fn native_n1_l2_missing_allocation_bound_refuses_before_registration() {
    assert_eq!(
        ShmBudget::new(0, None).err(),
        Some(NativeError::Code(NativeCode::ShmAllocationBoundUnknown))
    );
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let h = hooks();
    let refused = NativeOwner::begin(h, gate, [(1, 1), (0, 1), (0, 2)], 0, None);
    assert!(matches!(refused, Err(NativeError::Code(NativeCode::ShmAllocationBoundUnknown))));
    assert!(h.lock().active.is_none(), "缺 A(len) 先于登记拒绝");
}

#[test]
fn native_n1_l2c_second_page_write_after_deadline_is_refused_first_page_charged() {
    let (gate, j, now) = gate_with(1_000, 1 << 20);
    j.borrow_mut().begin_first_read().unwrap();
    let mut tls = OwnerTls {
        token: 1,
        gate,
        budget: ShmBudget::new(0, identity_bound()).unwrap(),
        first: None,
        cleanup: false,
    };
    assert!(matches!(
        owner_step(&mut tls, Class::Shm, None, Effect::Write { end: 4_096 }),
        Step::Count
    ));
    tls.gate.charge_wire_bytes(1).unwrap();
    now.set(now.get() + Duration::from_millis(2_000));
    assert!(matches!(
        owner_step(&mut tls, Class::Shm, None, Effect::Write { end: 8_192 }),
        Step::Deny(libc::EIO)
    ));
    match &tls.first {
        Some(NativeError::Job(e)) => assert_eq!(e.code(), job::DEADLINE),
        other => panic!("首错应为期限：{other:?}"),
    }
    assert_eq!(j.borrow().snapshot().wire_bytes, 1, "L2-d 第一页实际 n 已入账");
    assert_eq!(tls.budget.covered(), 4_096, "期限门先于新预留");
    assert!(matches!(
        owner_step(&mut tls, Class::Shm, None, Effect::Truncate { end: 3 }),
        Step::Deny(libc::EIO)
    ));
}

#[test]
fn native_n1_l2_main_and_wal_writes_before_seal_are_refused_reads_counted() {
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let mut tls = OwnerTls {
        token: 1,
        gate,
        budget: ShmBudget::new(0, identity_bound()).unwrap(),
        first: None,
        cleanup: false,
    };
    assert!(matches!(owner_step(&mut tls, Class::Wal, None, Effect::Read), Step::Count));
    assert!(matches!(
        owner_step(&mut tls, Class::Main, None, Effect::Write { end: 10 }),
        Step::Deny(libc::EACCES)
    ));
    assert_eq!(tls.first, Some(NativeError::Code(NativeCode::WriteBeforeSeal)));
}

#[test]
fn native_n1_l2_writer_phase_uncounted_but_cannot_widen_shm_mapping() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let mut tls = OwnerTls {
        token: 1,
        gate,
        budget: ShmBudget::new(0, identity_bound()).unwrap(),
        first: None,
        cleanup: false,
    };
    tls.budget.cover(&tls.gate, 32_768).unwrap();
    tls.gate.set_phase(Phase::Writer);
    assert!(matches!(
        owner_step(&mut tls, Class::Shm, None, Effect::Map { end: 32_768 }),
        Step::Pass
    ));
    assert!(matches!(
        owner_step(&mut tls, Class::Main, None, Effect::Write { end: 4_096 }),
        Step::Pass
    ));
    assert!(matches!(
        owner_step(&mut tls, Class::Shm, None, Effect::Map { end: 65_536 }),
        Step::Deny(libc::ENOMEM)
    ));
    assert_eq!(j.borrow().snapshot().store.pending, 32_768, "Writer 不新预留");
}

#[test]
fn native_n1_l2e_measure_failure_is_first_error_and_still_settles_conservatively() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let mut budget = ShmBudget::new(0, identity_bound()).unwrap();
    budget.cover(&gate, 32_768).unwrap();
    budget.mark_delegated();
    assert_eq!(
        budget.settle(&gate, Err(())),
        Err(NativeError::Code(NativeCode::ShmMeasureFailed))
    );
    let snap = j.borrow().snapshot();
    assert_eq!((snap.store.committed, snap.open_reservations), (32_768, 0));
}

#[test]
fn native_n1_l2f_undelegated_grant_is_released_delegated_is_charged() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let mut budget = ShmBudget::new(0, identity_bound()).unwrap();
    budget.cover(&gate, 16_384).unwrap();
    budget.mark_delegated();
    budget.cover(&gate, 32_768).unwrap();
    budget.settle(&gate, Ok(0)).unwrap();
    let snap = j.borrow().snapshot();
    assert_eq!(snap.store.committed, 16_384, "未委托的第二笔 release，不入账");
    assert_eq!(snap.open_reservations, 0);
}

#[test]
fn native_n1_l2_bound_violation_charges_actual_then_stops_job() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let mut budget = ShmBudget::new(0, identity_bound()).unwrap();
    budget.cover(&gate, 16_384).unwrap();
    budget.mark_delegated();
    assert_eq!(
        budget.settle(&gate, Ok(20_480)),
        Err(NativeError::Code(NativeCode::ShmBoundViolated))
    );
    let snap = j.borrow().snapshot();
    assert_eq!(snap.store.committed, 20_480, "越界实测先照实入账");
    assert_eq!(snap.stopped, Some(job::RECONCILE));
}

#[test]
fn native_n1_l2_reserve_over_store_limit_refuses_before_delegate() {
    let (gate, _, _) = gate_with(60_000, 4_096);
    let mut tls = OwnerTls {
        token: 1,
        gate,
        budget: ShmBudget::new(0, identity_bound()).unwrap(),
        first: None,
        cleanup: false,
    };
    assert!(matches!(
        owner_step(&mut tls, Class::Shm, None, Effect::Truncate { end: 32_768 }),
        Step::Deny(libc::ENOMEM)
    ));
    assert_eq!(tls.budget.grant_count(), 0);
    assert!(matches!(tls.first, Some(NativeError::Job(_))));
}

// ------------------------------------------------------------ G3/同 Job 门面与收尾

#[test]
fn native_n1_gate_short_borrow_conflict_is_named_error_not_panic() {
    let (gate, j, _) = gate_with(60_000, 1 << 20);
    let held = j.borrow_mut();
    assert_eq!(gate.pre_io_check(), Err(NativeError::Code(NativeCode::GateBorrowed)));
    drop(held);
    assert_eq!(gate.pre_io_check(), Ok(()));
    gate.charge_wire_bytes(7).unwrap();
    assert_eq!(j.borrow().snapshot().wire_bytes, 7, "门面记在同一本 Job");
}

#[test]
fn native_n1_l3b_first_error_survives_seal_and_cleanup_errors() {
    let mut errors = NativeErrors::default();
    errors.record_cleanup(NativeCode::CloseBusy.into());
    assert!(matches!(errors.reported(), Some(Reported::Cleanup(_))));
    errors.record_first(NativeCode::InProcessPeer.into());
    errors.record_first(NativeCode::TrackingLost.into());
    match errors.reported() {
        Some(Reported::First(NativeError::Code(code))) => assert_eq!(*code, NativeCode::InProcessPeer),
        other => panic!("{other:?}"),
    }
}

#[test]
fn native_n1_owner_rejects_second_owner_on_same_thread() {
    let h = hooks();
    let (g1, _, _) = gate_with(60_000, 1 << 20);
    let first = NativeOwner::begin(h, g1, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let (g2, _, _) = gate_with(60_000, 1 << 20);
    assert!(matches!(
        NativeOwner::begin(hooks(), g2, [(2, 2), (0, 1), (0, 2)], 0, identity_bound()),
        Err(NativeError::Code(NativeCode::AlreadyActive))
    ));
    let mut errors = NativeErrors::default();
    let _ = first.destroy(&mut errors);
}


// ------------------------------------------------------------ 31V 原业务核的实际事务消费者

fn native_input() -> String {
    serde_json::json!({
        "schema_revision":"1", "session_id":"s-session-testonly-001",
        "source_namespace":"testonly.tick.ohlc", "source_epoch":"1",
        "instrument":"TEST.TICK", "profile":"testonly_tick_1_1_ohlc",
        "events":[{"event_id":"e1","revision":"1","seq":"1",
            "received_at":"2026-09-09T00:00:00.000Z","raw_text":"10",
            "price":"10","timestamp":"1","volume":"1"}]
    }).to_string()
}

/// 比较所有业务表的完整 SQL 值，不以成功返回或选几列替代持久效果。
fn logical_rows(conn: &rusqlite::Connection) -> Vec<(String, Vec<String>)> {
    let names: Vec<String> = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).unwrap().query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
    names.into_iter().map(|name| {
        let mut statement = conn.prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""))).unwrap();
        let columns = statement.column_count();
        let mut rows: Vec<String> = statement.query_map([], |row| {
            let values = (0..columns).map(|i| row.get::<_, rusqlite::types::Value>(i)).collect::<Result<Vec<_>, _>>()?;
            Ok(format!("{values:?}"))
        }).unwrap().collect::<Result<_, _>>().unwrap();
        rows.sort();
        (name, rows)
    }).collect()
}

#[test]
fn native_n1_accept_in_matches_original_output_and_all_persisted_rows() {
    let old = crate::tests::TestFiles::new();
    let owned = crate::tests::TestFiles::new();
    crate::tests::init_test_db(&old);
    crate::tests::init_test_db(&owned);
    let input = native_input();
    let profile = crate::tests::fixture("profiles/testonly_tick_1_1_ohlc.json");
    let expected = crate::accept_core(&old.db(), &input, &profile, crate::DEFAULT_WRITER_EPOCH, None).unwrap();
    let mut connection = crate::open_db(&owned.db()).unwrap();
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
    let actual = crate::accept_in(&tx, &input, &profile, crate::DEFAULT_WRITER_EPOCH, None).unwrap();
    assert_eq!(actual, expected);
    tx.commit().unwrap();
    assert_eq!(logical_rows(&connection), logical_rows(&crate::open_db(&old.db()).unwrap()));
}

#[test]
fn native_n1_accept_in_caller_rollback_discards_all_business_effects() {
    let files = crate::tests::TestFiles::new();
    crate::tests::init_test_db(&files);
    let mut connection = crate::open_db(&files.db()).unwrap();
    let before = logical_rows(&connection);
    let tx = connection.transaction().unwrap();
    crate::accept_in(&tx, &native_input(), &crate::tests::fixture("profiles/testonly_tick_1_1_ohlc.json"), crate::DEFAULT_WRITER_EPOCH, None).unwrap();
    assert_ne!(logical_rows(&tx), before);
    tx.rollback().unwrap();
    assert_eq!(logical_rows(&connection), before);
}

#[test]
fn native_n1_accept_in_preserves_parse_profile_and_uninitialized_error_order() {
    let files = crate::tests::TestFiles::new();
    let profile = crate::tests::fixture("profiles/testonly_tick_1_1_ohlc.json");
    let missing = files.db().with_extension("missing-profile");
    for (input, selected) in [("not-json".to_owned(), &missing), (native_input(), &missing), (native_input(), &profile)] {
        let expected = crate::accept_core(&files.db(), &input, selected, crate::DEFAULT_WRITER_EPOCH, None).unwrap_err();
        let mut connection = rusqlite::Connection::open(&files.db()).unwrap();
        let tx = connection.transaction().unwrap();
        let actual = crate::accept_in(&tx, &input, selected, crate::DEFAULT_WRITER_EPOCH, None).unwrap_err();
        assert_eq!(actual, expected);
        tx.rollback().unwrap();
    }
}

#[test]
fn native_n1_migrate_in_uses_caller_transaction_and_is_idempotent() {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    connection.execute_batch(crate::PRE_POLICY_RECORDS_DELTAS).unwrap();
    assert!(crate::policy_records_missing(&connection).unwrap());
    let tx = connection.transaction().unwrap();
    crate::migrate_in(&tx).unwrap();
    crate::migrate_in(&tx).unwrap();
    assert!(!crate::policy_records_missing(&tx).unwrap());
    tx.rollback().unwrap();
    assert!(crate::policy_records_missing(&connection).unwrap());
    let tx = connection.transaction().unwrap();
    crate::migrate_in(&tx).unwrap();
    tx.commit().unwrap();
    assert!(!crate::policy_records_missing(&connection).unwrap());
}

#[test]
fn native_n1_s2_audit_keeps_original_missing_budget_adapter_rejection() {
    let files = crate::tests::TestFiles::new();
    crate::tests::init_test_db(&files);
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(hooks(), gate, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let mut session = owner.open(&files.db()).unwrap();
    let tx = session.begin().unwrap();
    let error = tx.audit().err().expect("旧证据图没有非 mixed 同源费用适配，不发收据");
    assert!(error.starts_with("LegacyAdmissionBudgetAdapterMissing"), "{error}");
    tx.rollback().unwrap();
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
}

fn native_s3_fixture() -> crate::tests::TestFiles {
    let files = crate::tests::TestFiles::new();
    let mut producer = crate::v3::prepared::tests::Fixture::new();
    let (envelope, token) = producer.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
    producer.publish(&envelope, &token).unwrap();
    // 用真实 S3 写者生成一代，再以 SQLite 快照复制到待准入源；不是手填计费节点。
    let db = files.db();
    producer.conn.execute("VACUUM INTO ?1", [db.to_str().unwrap()]).unwrap();
    files
}

#[test]
fn native_n1_s3_real_audit_charges_original_job_and_keeps_s2_accept_version_gate() {
    let files = native_s3_fixture();
    let consumed = super::super::test_prepare(&files.db()).consume().unwrap();
    let source = crate::v3::admission_provider::MeasuredDomain::measure(&consumed).unwrap();
    let started = consumed.into_job(source.existing).unwrap();
    let gate = started.native_io_gate().unwrap();
    let account = Rc::clone(&gate.job);
    let before = account.borrow().snapshot();
    let owner = NativeOwner::begin(hooks(), gate, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let mut session = owner.open(&files.db()).unwrap();
    let tx = session.begin().unwrap();
    let receipt = tx.audit().unwrap();
    assert_eq!(receipt.delta, crate::DeltaShape::Current);
    let after = account.borrow().snapshot();
    assert!(after.nodes > before.nodes && after.node_bytes > before.node_bytes);
    assert!(after.records > before.records);
    let mut errors = NativeErrors::default();
    tx.settle_shm(Ok(0)).unwrap();
    let mut finished = tx.seal(started, &receipt, &mut errors).expect("同一本 Job 完成 seal");
    assert!(Rc::ptr_eq(&account, &finished.job));
    assert_eq!(with_owner(|tls| tls.gate.phase()), Some(Phase::Writer));
    let error = tx.accept(&receipt, &native_input(),
        &crate::tests::fixture("profiles/testonly_tick_1_1_ohlc.json"), crate::DEFAULT_WRITER_EPOCH, None).unwrap_err();
    assert!(error.starts_with("VersionMismatch"), "S3审计成功不能充作S2写资格：{error}");
    tx.rollback().unwrap();
    let destroyed = match session.close(&mut errors) {
        Teardown::Destroyed(receipt) => receipt,
        Teardown::Alive(_) => panic!("事务已关闭"),
    };
    n1_finish(&mut finished, &destroyed).unwrap();
}

#[test]
fn native_n1_active_tracking_loss_never_delegates_real_io() {
    let h = hooks();
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    h.unavailable.store(true, Ordering::SeqCst);
    let called = Cell::new(false);
    assert_eq!(unsafe { h.io(-1, Effect::Read, || { called.set(true); 7 }) }, -1);
    assert!(!called.get());
    assert_eq!(owner.first_error(), Some(NativeError::Code(NativeCode::TrackingLost)));
    let _ = owner.destroy(&mut NativeErrors::default());
}

#[test]
fn native_n1_writer_first_error_prevents_later_write_without_restarting_deadline() {
    let (gate, _, now) = gate_with(1, 1 << 20);
    gate.with_job(|job| job.begin_first_read()).unwrap();
    gate.set_phase(Phase::Writer);
    now.set(now.get() + Duration::from_secs(1));
    let mut tls = OwnerTls { token: 1, gate, budget: ShmBudget::new(0, identity_bound()).unwrap(), first: None, cleanup: false };
    assert!(matches!(owner_step(&mut tls, Class::Main, None, Effect::Write { end: 1 }), Step::Pass));
    tls.fail(NativeCode::TrackingLost.into());
    assert!(matches!(owner_step(&mut tls, Class::Main, None, Effect::Write { end: 1 }), Step::Deny(libc::EIO)));
}


#[test]
fn native_n1_actual_short_read_counts_returned_n_in_same_job() {
    let file = Temp::new("actual-read");
    let h = hooks();
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, [file.id(), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    owner.recheck().unwrap();
    let fd = owner.open_scope(|| unsafe { h.open(file.c().as_ptr(), libc::O_RDONLY, 0) });
    assert!(fd >= 0);
    owner.enter_audit();
    let mut buffer = [0u8; 128];
    let got = unsafe { h.pread(fd, buffer.as_mut_ptr().cast(), buffer.len(), 0) };
    let expected = std::fs::read(&file.0).unwrap();
    assert_eq!(got as usize, expected.len());
    assert_eq!(&buffer[..got as usize], expected.as_slice());
    assert_eq!(account.borrow().snapshot().wire_bytes, got as u128);
    assert_eq!(unsafe { h.close(fd) }, 0);
    let _ = owner.destroy(&mut NativeErrors::default());
}

#[test]
fn native_n1_peer_thread_cannot_use_or_close_owner_receipt_fd() {
    let file = Temp::new("peer-fd");
    let h = hooks();
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, [file.id(), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let fd = owner.open_scope(|| unsafe { h.open(file.c().as_ptr(), libc::O_RDWR, 0) });
    assert!(fd >= 0);
    write_lock(fd);
    let (read_rc, close_rc) = std::thread::spawn(move || {
        let mut b = [0u8; 1];
        (unsafe { h.read(fd, b.as_mut_ptr().cast(), 1) }, unsafe { h.close(fd) })
    }).join().unwrap();
    assert_eq!((read_rc, close_rc), (-1, 0));
    assert_eq!(account.borrow().snapshot().wire_bytes, 0);
    assert!(fd_open(fd));
    assert!(!other_process_can_lock(&file.0));
    assert_eq!(owner.first_error(), Some(NativeError::Code(NativeCode::InProcessPeer)));
    assert!(h.deferred_for_test().is_empty(), "owner 同一 FD 不排入双关队列");
    assert_eq!(unsafe { h.close(fd) }, 0);
    assert!(!fd_open(fd));
    let _ = owner.destroy(&mut NativeErrors::default());
}

#[test]
fn native_n1_open_scope_panic_cannot_leave_main_receipt_window_open() {
    let file = Temp::new("scope-panic");
    let h = hooks();
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, [file.id(), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| owner.open_scope(|| panic!("test-only")))).is_err());
    assert!(!h.lock().active_for(owner.token).unwrap().main_scope);
    assert_eq!(unsafe { h.open(file.c().as_ptr(), libc::O_RDONLY, 0) }, -1);
    let _ = owner.destroy(&mut NativeErrors::default());
}

#[test]
fn native_n1_shm_settle_borrow_failure_retains_unsettled_grants() {
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let mut budget = ShmBudget::new(0, identity_bound()).unwrap();
    budget.cover(&gate, 32768).unwrap();
    budget.mark_delegated();
    let held = account.borrow_mut();
    assert_eq!(budget.settle(&gate, Ok(0)), Err(NativeError::Code(NativeCode::GateBorrowed)));
    assert_eq!(budget.grant_count(), 1);
    assert_eq!(held.snapshot().open_reservations, 1);
    drop(held);
    budget.settle(&gate, Ok(0)).unwrap();
    assert_eq!(budget.grant_count(), 0);
    assert_eq!(account.borrow().snapshot().store.committed, 32768);
    assert_eq!(account.borrow().snapshot().open_reservations, 0);
}


#[test]
fn native_n1_active_poison_never_delegates_open_or_unlink() {
    let file = Temp::new("poison-unlink");
    let h = hooks();
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    h.poison_for_test();
    assert_eq!(unsafe { h.open(file.c().as_ptr(), libc::O_RDWR, 0) }, -1);
    assert_eq!(h.lock().inflight_open, 0);
    assert_eq!(unsafe { h.unlink(file.c().as_ptr()) }, -1);
    assert!(file.0.exists(), "active失去登记资格时不透传删除，即使该路径不是已知pin");
    assert!(owner.first_error().is_some());
    let _ = owner.destroy(&mut NativeErrors::default());
}


#[test]
fn native_n1_seal_qualification_error_does_not_replace_earlier_owner_error() {
    let files = native_s3_fixture();
    let consumed = super::super::test_prepare(&files.db()).consume().unwrap();
    let source = crate::v3::admission_provider::MeasuredDomain::measure(&consumed).unwrap();
    let started = consumed.into_job(source.existing).unwrap();
    let owner = NativeOwner::begin(hooks(), started.native_io_gate().unwrap(),
        [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let _ = with_owner(|tls| tls.fail(NativeCode::TrackingLost.into()));
    // 负控才构造错误收据：不能由此进入Writer，也不能覆盖先发生的失跟踪错误。
    let invalid = NativeAuditReceipt { token: owner.token.wrapping_add(1), delta: crate::DeltaShape::Current };
    let mut errors = NativeErrors::default();
    assert!(owner.seal(started, &invalid, &mut errors).is_none());
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Code(NativeCode::TrackingLost)))));
    assert_eq!(with_owner(|tls| tls.gate.phase()), Some(Phase::Precheck));
    let _ = owner.destroy(&mut errors);
}


// ------------------------------------------------------------ 31Y 单 owner 连接关闭域

fn close_fixture(tag: &str) -> (NativeSession, Temp, &'static NativeHooks, Rc<RefCell<job::Job>>) {
    let file = Temp::new(tag);
    std::fs::write(&file.0, []).unwrap();
    let conn = rusqlite::Connection::open(&file.0).unwrap();
    conn.execute_batch("CREATE TABLE payload(data BLOB); INSERT INTO payload VALUES(zeroblob(8));").unwrap();
    conn.close().unwrap();
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let h = hooks();
    let owner = NativeOwner::begin(h, gate, [file.id(), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    (owner.open(&file.0).unwrap(), file, h, account)
}

struct CallbackDrop(std::sync::Arc<AtomicU64>);
impl Drop for CallbackDrop {
    fn drop(&mut self) { self.0.fetch_add(1, Ordering::Relaxed); }
}

fn close_callbacks(session: &mut NativeSession) -> (std::sync::Arc<AtomicU64>, std::sync::Arc<AtomicU64>, std::sync::Arc<AtomicU64>) {
    let progress = std::sync::Arc::new(AtomicU64::new(0));
    let authorizer = std::sync::Arc::new(AtomicU64::new(0));
    let drops = std::sync::Arc::new(AtomicU64::new(0));
    let p = progress.clone();
    let pd = CallbackDrop(drops.clone());
    session.progress_handler(1, move || { let _keep = &pd; p.fetch_add(1, Ordering::Relaxed); false }).unwrap();
    let a = authorizer.clone();
    let ad = CallbackDrop(drops.clone());
    session.authorizer(move |event| {
        let _keep = &ad;
        if event.action == rusqlite::ffi::SQLITE_SELECT { a.fetch_add(1, Ordering::Relaxed); }
        rusqlite::hooks::Authorization::Allow
    }).unwrap();
    (progress, authorizer, drops)
}

fn close_peer(file: &Temp, h: &NativeHooks) -> c_int {
    let fd = unsafe { libc::open(file.c().as_ptr(), libc::O_RDWR) };
    assert!(fd >= 0);
    assert_eq!(unsafe { h.close(fd) }, 0);
    assert!(fd_open(fd) && h.deferred_for_test().contains(&fd));
    fd
}

fn hold_sqlite_lock_until_close(session: &NativeSession, file: &Temp) {
    session.state.as_ref().unwrap().view.as_ref().unwrap().execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(!other_process_can_lock(&file.0), "成功重试前真实SQLite事务仍持锁");
}

fn assert_live_callbacks(session: &NativeSession, progress: &AtomicU64, authorizer: &AtomicU64) {
    let (p, a) = (progress.load(Ordering::Relaxed), authorizer.load(Ordering::Relaxed));
    session.state.as_ref().unwrap().view.as_ref().unwrap().execute_batch("SELECT 73;").unwrap();
    assert!(progress.load(Ordering::Relaxed) > p, "关闭失败后真实progress仍执行");
    assert!(authorizer.load(Ordering::Relaxed) > a, "关闭失败后真实authorizer仍执行");
}

#[test]
fn native_n1_l3a_close_busy_keeps_callback_context_then_destroys() {
    let (mut session, file, h, account) = close_fixture("statement-close");
    let (p, a, drops) = close_callbacks(&mut session);
    let raw = session.state.as_ref().unwrap().raw;
    // 留一个 cached statement：close 必须先 finalize 缓存，再测试真正悬挂的 statement。
    { let view = session.state.as_ref().unwrap().view.as_ref().unwrap();
      let mut cached = view.prepare_cached("SELECT 7").unwrap();
      assert_eq!(cached.query_row([], |r| r.get::<_, i64>(0)).unwrap(), 7); }
    let mut stmt = std::ptr::null_mut();
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_prepare_v2(raw, c"SELECT data FROM payload".as_ptr(), -1, &mut stmt, std::ptr::null_mut()) }, rusqlite::ffi::SQLITE_OK);
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_step(stmt) }, rusqlite::ffi::SQLITE_ROW);
    assert!(!other_process_can_lock(&file.0), "真实SQLite读锁先存在");
    let peer = close_peer(&file, h);
    with_owner(|tls| tls.budget.cover(&tls.gate, 32768).unwrap());
    assert_eq!(account.borrow().snapshot().open_reservations, 1);
    let mut errors = NativeErrors::default();
    let session = match session.close(&mut errors) { Teardown::Alive(s) => s, _ => panic!("真实statement BUSY") };
    assert_eq!(errors.cleanup(), &[NativeError::Code(NativeCode::CloseBusy)]);
    assert_live_callbacks(&session, &p, &a);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert!(!other_process_can_lock(&file.0));
    assert!(fd_open(peer));
    assert_eq!(account.borrow().snapshot().open_reservations, 1);
    assert!(with_owner(|_| ()).is_some() && h.active_any.load(Ordering::SeqCst));
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_finalize(stmt) }, rusqlite::ffi::SQLITE_OK);
    session.settle_shm(Ok(0)).unwrap();
    hold_sqlite_lock_until_close(&session, &file);
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert_eq!(drops.load(Ordering::Relaxed), 2, "仅成功close后释放两个回调上下文");
    assert!(!fd_open(peer));
    assert!(other_process_can_lock(&file.0));
    assert!(with_owner(|_| ()).is_none() && !h.active_any.load(Ordering::SeqCst));
}

#[test]
fn native_n1_close_blob_busy_keeps_real_callbacks() {
    let (mut session, file, h, _) = close_fixture("blob-close");
    let (p, a, drops) = close_callbacks(&mut session);
    let raw = session.state.as_ref().unwrap().raw;
    let mut blob = std::ptr::null_mut();
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_blob_open(raw, c"main".as_ptr(), c"payload".as_ptr(), c"data".as_ptr(), 1, 0, &mut blob) }, rusqlite::ffi::SQLITE_OK);
    // SQLite 3.53.2 的blob内部statement可见；另用backup覆盖next_stmt看不到的BUSY。
    assert!(!other_process_can_lock(&file.0));
    let peer = close_peer(&file, h);
    let mut errors = NativeErrors::default();
    errors.record_first(NativeCode::TrackingLost.into());
    let session = match session.close(&mut errors) { Teardown::Alive(s) => s, _ => panic!("真实blob BUSY") };
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Code(NativeCode::TrackingLost)))));
    assert_eq!(errors.cleanup(), &[NativeError::Code(NativeCode::CloseBusy)]);
    assert_live_callbacks(&session, &p, &a);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert!(fd_open(peer) && !other_process_can_lock(&file.0));
    let mut data = [1u8; 8];
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_blob_read(blob, data.as_mut_ptr().cast(), 8, 0) }, rusqlite::ffi::SQLITE_OK);
    assert_eq!(data, [0; 8]);
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_blob_close(blob) }, rusqlite::ffi::SQLITE_OK);
    hold_sqlite_lock_until_close(&session, &file);
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert_eq!(drops.load(Ordering::Relaxed), 2);
    assert!(!fd_open(peer) && other_process_can_lock(&file.0));
}

#[test]
fn native_n1_alive_drop_retains_callbacks_lock_grant_and_deferred_fd() {
    const CHILD: &str = "NEWCHAN_31Y_ALIVE_DROP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["native_n1_alive_drop_retains_callbacks_lock_grant_and_deferred_fd", "--nocapture"])
            .env(CHILD, "1").output().unwrap();
        assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("31Y alive resources observed"));
        return;
    }
    let (mut session, file, h, account) = close_fixture("drop-alive");
    let (p, a, drops) = close_callbacks(&mut session);
    let raw = session.state.as_ref().unwrap().raw;
    let mut blob = std::ptr::null_mut();
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_blob_open(raw, c"main".as_ptr(), c"payload".as_ptr(), c"data".as_ptr(), 1, 0, &mut blob) }, rusqlite::ffi::SQLITE_OK);
    let peer = close_peer(&file, h);
    with_owner(|tls| tls.budget.cover(&tls.gate, 32768).unwrap());
    let mut errors = NativeErrors::default();
    let alive = session.close(&mut errors);
    assert!(matches!(&alive, Teardown::Alive(_)));
    drop(alive);
    let (before_p, before_a) = (p.load(Ordering::Relaxed), a.load(Ordering::Relaxed));
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_exec(raw, c"SELECT 93".as_ptr(), None, std::ptr::null_mut(), std::ptr::null_mut()) }, rusqlite::ffi::SQLITE_OK);
    assert!(p.load(Ordering::Relaxed) > before_p && a.load(Ordering::Relaxed) > before_a);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert_eq!(account.borrow().snapshot().open_reservations, 1);
    assert!(fd_open(peer) && !other_process_can_lock(&file.0));
    assert!(with_owner(|_| ()).is_some() && h.active_any.load(Ordering::SeqCst));
    println!("31Y alive resources observed");
    // 这个单独测试进程在此退出；泄漏不污染其他测试的TLS/FD。
}

#[test]
fn native_n1_open_failure_keeps_owner_until_explicit_close() {
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let h = hooks();
    let owner = NativeOwner::begin(h, gate, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let error = owner.open(Path::new("/nonexistent/issue1527-no-create.db")).err().expect("不自动创建源");
    assert!(error.error.starts_with("NativeSqliteOpen:"));
    assert!(with_owner(|_| ()).is_some() && h.active_any.load(Ordering::SeqCst));
    assert!(!error.session.state.as_ref().unwrap().raw.is_null(), "SQLite open失败仍可能返回待关闭句柄");
    let mut errors = NativeErrors::default();
    assert!(matches!(error.session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Business(e))) if e.starts_with("NativeSqliteOpen:")));
    assert!(with_owner(|_| ()).is_none() && !h.active_any.load(Ordering::SeqCst));
}

/// backup登记在源Btree上；源连接无statement仍不能sqlite3_close。
fn pending_backup(source: *mut rusqlite::ffi::sqlite3) -> (rusqlite::Connection, *mut rusqlite::ffi::sqlite3_backup) {
    // 保留显式源读事务，外部进程可观测锁；不能用目的连接做close探针。
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_exec(source,
        c"CREATE TABLE IF NOT EXISTS payload(data BLOB); INSERT INTO payload VALUES(zeroblob(65536)); BEGIN; SELECT data FROM payload;".as_ptr(),
        None, std::ptr::null_mut(), std::ptr::null_mut()) }, rusqlite::ffi::SQLITE_OK);
    let destination = rusqlite::Connection::open_in_memory().unwrap();
    let backup = unsafe { rusqlite::ffi::sqlite3_backup_init(destination.handle(), c"main".as_ptr(), source, c"main".as_ptr()) };
    assert!(!backup.is_null());
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_backup_step(backup, 1) }, rusqlite::ffi::SQLITE_OK, "一页未完成");
    assert!(unsafe { rusqlite::ffi::sqlite3_next_stmt(source, std::ptr::null_mut()) }.is_null());
    (destination, backup)
}

#[test]
fn native_n1_close_backup_busy_preserves_callbacks_where_31v_loses_them() {
    // 31V的实际close序列负控：cache flush、next_stmt为空、rusqlite owned close。
    let legacy = rusqlite::Connection::open_in_memory().unwrap();
    let (_legacy_destination, backup) = pending_backup(unsafe { legacy.handle() });
    let calls = std::sync::Arc::new(AtomicU64::new(0));
    let observed = calls.clone();
    legacy.progress_handler(1, Some(move || { observed.fetch_add(1, Ordering::Relaxed); false })).unwrap();
    legacy.flush_prepared_statement_cache();
    assert!(unsafe { rusqlite::ffi::sqlite3_next_stmt(legacy.handle(), std::ptr::null_mut()) }.is_null());
    let (legacy, error) = legacy.close().err().expect("backup令旧close返回BUSY");
    assert_eq!(error.sqlite_error_code(), Some(rusqlite::ErrorCode::DatabaseBusy));
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_backup_finish(backup) }, rusqlite::ffi::SQLITE_OK);
    legacy.execute_batch("SELECT 19").unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0, "负控实测：31V close路径已卸载progress");
    println!("31Y backup negative control: next_stmt=null, close=BUSY, progress_calls={}", calls.load(Ordering::Relaxed));
    legacy.close().unwrap();

    let (mut session, file, h, _) = close_fixture("backup-close");
    let (p, a, drops) = close_callbacks(&mut session);
    let raw = session.state.as_ref().unwrap().raw;
    let (_destination, backup) = pending_backup(raw);
    assert!(!other_process_can_lock(&file.0), "backup源连接显式读事务持有SQLite锁");
    let peer = close_peer(&file, h);
    let mut errors = NativeErrors::default();
    let session = match session.close(&mut errors) { Teardown::Alive(s) => s, _ => panic!("真实backup BUSY") };
    assert_eq!(errors.cleanup(), &[NativeError::Code(NativeCode::CloseBusy)]);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert!(fd_open(peer) && !other_process_can_lock(&file.0));
    // 结束backup，但尚不重试close；源连接读事务仍然持锁。
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_backup_finish(backup) }, rusqlite::ffi::SQLITE_OK);
    assert_live_callbacks(&session, &p, &a);
    session.state.as_ref().unwrap().view.as_ref().unwrap().execute_batch("ROLLBACK").unwrap();
    println!("31Y backup fixed: progress_calls={}, authorizer_calls={}, callback_drops={}", p.load(Ordering::Relaxed), a.load(Ordering::Relaxed), drops.load(Ordering::Relaxed));
    hold_sqlite_lock_until_close(&session, &file);
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert_eq!(drops.load(Ordering::Relaxed), 2);
    assert!(!fd_open(peer) && other_process_can_lock(&file.0));
}

fn native_child(name: &str) -> bool {
    if std::env::var("NEWCHAN_32D_CHILD").ok().as_deref() == Some(name) { return true; }
    let output = std::process::Command::new(std::env::current_exe().unwrap()).args([name, "--nocapture"])
        .env("NEWCHAN_32D_CHILD", name).output().unwrap();
    assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    println!("{}", String::from_utf8_lossy(&output.stdout));
    false
}

fn wal_fixture(tag: &str) -> (Temp, [DevIno; 3], std::fs::File, u64) {
    use std::os::unix::fs::MetadataExt;
    let file = Temp::new(tag);
    std::fs::write(&file.0, []).unwrap();
    let conn = rusqlite::Connection::open(&file.0).unwrap();
    let mut set = 0;
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_db_config(conn.handle(), rusqlite::ffi::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, 1, &mut set) }, rusqlite::ffi::SQLITE_OK);
    conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE payload(data BLOB); INSERT INTO payload VALUES(zeroblob(65536));").unwrap();
    conn.close().unwrap();
    let wal = PathBuf::from(format!("{}-wal", file.0.display()));
    let shm = PathBuf::from(format!("{}-shm", file.0.display()));
    let pin = std::fs::File::open(&shm).unwrap();
    let metadata = pin.metadata().unwrap();
    let wal_meta = std::fs::metadata(wal).unwrap();
    let pins = [file.id(), (wal_meta.dev(), wal_meta.ino()), (metadata.dev(), metadata.ino())];
    (file, pins, pin, metadata.blocks() * 512)
}

#[test]
fn native_n1_vfs32d_real_sqlite_io_and_automatic_cleanup() {
    if !native_child("native_n1_vfs32d_real_sqlite_io_and_automatic_cleanup") { return; }
    let h = install().expect("子进程在首次SQLite调用前安装");
    let (file, pins, pin, b0) = wal_fixture("real-vfs");
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let tx = session.begin().unwrap();
    let len: i64 = tx.tx.as_ref().unwrap().query_row("SELECT sum(length(data)) FROM payload", [], |r| r.get(0)).unwrap();
    assert_eq!(len, 65536);
    let snapshot = account.borrow().snapshot();
    assert!(snapshot.wire_bytes > 0);
    assert_eq!(snapshot.wire_bytes, h.actual_io.load(Ordering::SeqCst) as u128, "同连接只按syscall实际n入账，xRead不重计");
    assert!(snapshot.open_reservations > 0, "真实xShmMap仍持可写映射预留");
    assert!(tx.tx.as_ref().unwrap().execute_batch("ATTACH ':memory:' AS forbidden").is_err(), "原生authorizer实际拒绝旁路连接");
    tx.rollback().unwrap();
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)), "关闭自动清算无需人工settle");
    assert!(errors.reported().is_none(), "{errors:?}");
    assert_eq!(account.borrow().snapshot().open_reservations, 0);
    assert!(file.0.exists() && PathBuf::from(format!("{}-wal", file.0.display())).exists() && PathBuf::from(format!("{}-shm", file.0.display())).exists());
    assert!(other_process_can_lock(&file.0));
    println!("32D real SQLite: wire={}, syscall_n={}, grants_closed=0, sidecars=present", snapshot.wire_bytes, h.actual_io.load(Ordering::SeqCst));
}

#[test]
fn native_n1_vfs32f_failed_commit_rolls_back_with_callbacks_still_installed() {
    if !native_child("native_n1_vfs32f_failed_commit_rolls_back_with_callbacks_still_installed") { return; }
    let h = install().unwrap();
    let (file, pins, pin, b0) = wal_fixture("failed-commit");
    let (gate, account, clock) = gate_with(1000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let raw = session.state.as_ref().unwrap().raw;
    let tx = session.begin().unwrap();
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_get_autocommit(raw) }, 0);
    clock.set(clock.get() + Duration::from_secs(2));
    assert!(tx.commit().is_err(), "COMMIT 期限检查拒绝");
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_get_autocommit(raw) }, 1, "失败 COMMIT 的 Drop 已真实回滚");
    assert!(session.state.as_ref().unwrap().view.as_ref().unwrap().execute_batch("SELECT 1").is_err(), "清理域已退出，首错继续阻止业务");
    assert!(session.state.as_ref().unwrap().progress.is_some() && session.state.as_ref().unwrap().authorizer.is_some());
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Job(_)))));
    assert_eq!(account.borrow().snapshot().open_reservations, 0);
    assert!(other_process_can_lock(&file.0));
}

// 32F 故障注入在子进程首次 install 前替换 aSyscall 原指针；仍执行真实 libc I/O。
#[derive(Default)]
struct Faults32f {
    pin_fail: Option<c_int>,
    short_write: bool,
    expire_write: Option<Rc<Cell<Instant>>>,
    writes: Vec<(usize, isize)>,
    maps: Vec<(u64, usize)>,
    metadata_calls: usize,
}
thread_local! { static FAULTS32F: RefCell<Faults32f> = RefCell::new(Faults32f::default()); }
unsafe extern "C" fn fstat32f(fd: c_int, out: *mut libc::stat) -> c_int {
    if FAULTS32F.with(|f| f.borrow().pin_fail == Some(fd)) { set_errno(libc::EIO); -1 }
    else { libc::fstat(fd, out) }
}
unsafe extern "C" fn pwrite32f(fd: c_int, p: *const c_void, n: usize, off: libc::off_t) -> isize {
    let cap = FAULTS32F.with(|f| if f.borrow().short_write { n.min(3) } else { n });
    let got = libc::pwrite(fd, p, cap, off);
    if with_owner(|_| ()).is_some() {
        FAULTS32F.with(|f| {
            let mut f = f.borrow_mut(); f.writes.push((n, got));
            if let Some(clock) = f.expire_write.take() { clock.set(clock.get() + Duration::from_secs(120)); }
        });
    }
    got
}
unsafe extern "C" fn pwrite64_32f(fd: c_int, p: *const c_void, n: usize, off: i64) -> isize {
    pwrite32f(fd, p, n, off as libc::off_t)
}
unsafe extern "C" fn mmap32f(p: *mut c_void, n: usize, prot: c_int, flags: c_int, fd: c_int, off: libc::off_t) -> *mut c_void {
    if let Some(coverage) = with_owner(|tls| (tls.budget.covered(), tls.budget.grant_count())) {
        FAULTS32F.with(|f| f.borrow_mut().maps.push(coverage));
    }
    libc::mmap(p, n, prot, flags, fd, off)
}
unsafe extern "C" fn fchmod32f(fd: c_int, mode: libc::mode_t) -> c_int {
    FAULTS32F.with(|f| f.borrow_mut().metadata_calls += 1); libc::fchmod(fd, mode)
}
unsafe extern "C" fn fchown32f(fd: c_int, uid: libc::uid_t, gid: libc::gid_t) -> c_int {
    FAULTS32F.with(|f| f.borrow_mut().metadata_calls += 1); libc::fchown(fd, uid, gid)
}
fn fault_hooks32f() -> &'static NativeHooks {
    unsafe {
        let vfs = rusqlite::ffi::sqlite3_vfs_find(std::ptr::null());
        let set = (*vfs).xSetSystemCall.unwrap();
        let get = (*vfs).xGetSystemCall.unwrap();
        for (name, function) in [
            (c"fstat", std::mem::transmute::<FstatFn, SyscallPtr>(fstat32f)),
            (c"pwrite", std::mem::transmute::<PwriteFn, SyscallPtr>(pwrite32f)),
            (c"pwrite64", std::mem::transmute::<Pwrite64Fn, SyscallPtr>(pwrite64_32f)),
            (c"mmap", std::mem::transmute::<MmapFn, SyscallPtr>(mmap32f)),
            (c"fchmod", std::mem::transmute::<FchmodFn, SyscallPtr>(fchmod32f)),
            (c"fchown", std::mem::transmute::<FchownFn, SyscallPtr>(fchown32f)),
        ] { if get(vfs, name.as_ptr()).is_some() { assert_eq!(set(vfs, name.as_ptr(), Some(function)), rusqlite::ffi::SQLITE_OK); } }
    }
    install().unwrap()
}

fn native_file32f(session: &NativeSession) -> *mut rusqlite::ffi::sqlite3_file {
    let mut file = std::ptr::null_mut::<rusqlite::ffi::sqlite3_file>();
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_file_control(session.state.as_ref().unwrap().raw,
        c"main".as_ptr(), rusqlite::ffi::SQLITE_FCNTL_FILE_POINTER, (&mut file as *mut *mut rusqlite::ffi::sqlite3_file).cast()) }, rusqlite::ffi::SQLITE_OK);
    assert!(!file.is_null()); file
}
unsafe fn map32f(file: *mut rusqlite::ffi::sqlite3_file, page: c_int, extend: c_int) -> (c_int, *mut c_void) {
    let mut out = std::ptr::null_mut();
    let rc = (*(*file).pMethods).xShmMap.unwrap()(file, page, 32768, extend, &mut out);
    (rc, out)
}
fn assert_sidecars32f(file: &Path) {
    assert!(file.exists());
    assert!(PathBuf::from(format!("{}-wal", file.display())).exists());
    assert!(PathBuf::from(format!("{}-shm", file.display())).exists());
}

fn shm_fd32f(h: &NativeHooks) -> c_int {
    *h.lock().active.as_ref().unwrap().receipts.iter().find(|(_, class)| **class == Class::Shm).unwrap().0
}
#[test]
fn native_n1_vfs32f_sparse_bextend0_reserves_before_real_mmap_once() {
    if !native_child("native_n1_vfs32f_sparse_bextend0_reserves_before_real_mmap_once") { return; }
    let h = fault_hooks32f();
    let (file, pins, pin, b0) = wal_fixture("sparse-map");
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let handle = native_file32f(&session);
    let tx = session.begin().unwrap();
    // TestOnly 夹具：在已建立DMS锁/region0后制造足长稀疏尾部，避免重开SHM时被SQLite截断。
    assert_eq!(unsafe { libc::ftruncate(shm_fd32f(h), 98304) }, 0);
    FAULTS32F.with(|f| f.borrow_mut().maps.clear());
    let (rc, pointer) = unsafe { map32f(handle, 2, 0) };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK); assert!(!pointer.is_null());
    FAULTS32F.with(|f| { let f = f.borrow(); assert!(!f.maps.is_empty()); assert!(f.maps.iter().all(|(covered, grants)| *covered == 98304 && *grants >= 2)); });
    let before = account.borrow().snapshot();
    assert_eq!(before.store.pending, (98304 - b0) as u128);
    unsafe { pointer.cast::<u8>().write_volatile(7); }
    assert_eq!(unsafe { map32f(handle, 2, 0) }.0, rusqlite::ffi::SQLITE_OK);
    assert_eq!(account.borrow().snapshot().open_reservations, before.open_reservations, "重复map不重预留");
    tx.rollback().unwrap();
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_))); assert!(errors.reported().is_none(), "{errors:?}");
    assert_eq!(account.borrow().snapshot().store.committed, before.store.pending);
    assert_eq!(account.borrow().snapshot().open_reservations, 0); assert_sidecars32f(&file.0);
}

#[test]
fn native_n1_vfs32f_pin_measure_failure_after_real_growth_settles_conservatively() {
    use std::os::fd::AsRawFd;
    if !native_child("native_n1_vfs32f_pin_measure_failure_after_real_growth_settles_conservatively") { return; }
    let h = fault_hooks32f();
    let (file, pins, pin, b0) = wal_fixture("failed-pin");
    let pin_fd = pin.as_raw_fd();
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let handle = native_file32f(&session);
    let tx = session.begin().unwrap();
    FAULTS32F.with(|f| { let mut f = f.borrow_mut(); f.pin_fail = Some(pin_fd); f.writes.clear(); });
    assert_eq!(unsafe { map32f(handle, 1, 1) }.0, rusqlite::ffi::SQLITE_IOERR_SHMSIZE);
    assert!(FAULTS32F.with(|f| f.borrow().writes.iter().any(|(_, n)| *n > 0)), "委托已真实分配，不是调用前拒绝");
    let pending = account.borrow().snapshot().store.pending;
    assert_eq!(pending, (65536 - b0) as u128);
    drop(tx); // early return 的实际 RAII 路径，不人工 settle 或 rollback
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Code(NativeCode::ShmMeasureFailed)))));
    assert_eq!(account.borrow().snapshot().store.committed, pending);
    assert_eq!(account.borrow().snapshot().open_reservations, 0);
    assert_sidecars32f(&file.0); assert!(other_process_can_lock(&file.0));
}

#[test]
fn native_n1_vfs32f_internal_page_write_expiry_stops_later_writes_and_cleans_up() {
    if !native_child("native_n1_vfs32f_internal_page_write_expiry_stops_later_writes_and_cleans_up") { return; }
    let h = fault_hooks32f();
    let (file, pins, pin, b0) = wal_fixture("partial-growth");
    let (gate, account, clock) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let handle = native_file32f(&session);
    let tx = session.begin().unwrap();
    let before = account.borrow().snapshot().wire_bytes;
    FAULTS32F.with(|f| { let mut f = f.borrow_mut(); f.writes.clear(); f.expire_write = Some(clock); });
    assert_ne!(unsafe { map32f(handle, 1, 1) }.0, rusqlite::ffi::SQLITE_OK);
    assert_eq!(FAULTS32F.with(|f| f.borrow().writes.clone()), vec![(1, 1)], "SQLite内部第一页面写完成后超期，后续页不得委托");
    assert_eq!(account.borrow().snapshot().wire_bytes, before + 1, "失败仍计已写实际n");
    assert!(std::fs::metadata(format!("{}-shm", file.0.display())).unwrap().len() > 32768);
    drop(tx);
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Job(_)))));
    assert_eq!(account.borrow().snapshot().store.committed, (65536 - b0) as u128);
    assert_eq!(account.borrow().snapshot().open_reservations, 0); assert_sidecars32f(&file.0);
}

#[test]
fn native_n1_vfs32f_owned_syscall_short_write_counts_returned_n() {
    if !native_child("native_n1_vfs32f_owned_syscall_short_write_counts_returned_n") { return; }
    let h = fault_hooks32f();
    let (file, pins, pin, b0) = wal_fixture("real-short-write");
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let tx = session.begin().unwrap();
    let fd = shm_fd32f(h);
    let before = account.borrow().snapshot().wire_bytes;
    FAULTS32F.with(|f| f.borrow_mut().short_write = true);
    // SQLite实际打开/归属的SHM FD，调用已安装的syscall门；真实libc只写3字节。
    assert_eq!(unsafe { t_pwrite(fd, b"12345678".as_ptr().cast(), 8, 32760) }, 3);
    let mut actual = [0u8; 3];
    assert_eq!(unsafe { libc::pread(fd, actual.as_mut_ptr().cast(), 3, 32760) }, 3);
    assert_eq!(&actual, b"123");
    assert_eq!(account.borrow().snapshot().wire_bytes, before + 3);
    tx.rollback().unwrap();
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(errors.reported().is_none(), "{errors:?}"); assert_sidecars32f(&file.0);
}

fn metadata_deadline32f(name: &str, chmod: bool) {
    if !native_child(name) { return; }
    let h = fault_hooks32f();
    let (file, pins, pin, b0) = wal_fixture(name);
    let (gate, _, clock) = gate_with(1000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let tx = session.begin().unwrap();
    let before = FAULTS32F.with(|f| f.borrow().metadata_calls);
    clock.set(clock.get() + Duration::from_secs(2));
    let fd = shm_fd32f(h);
    let rc = unsafe { if chmod { t_fchmod(fd, 0o600) } else { t_fchown(fd, !0, !0) } };
    assert_eq!(rc, -1); assert_eq!(errno(), libc::EIO);
    assert_eq!(FAULTS32F.with(|f| f.borrow().metadata_calls), before, "超期不得委托原metadata syscall");
    drop(tx);
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Job(_))))); assert_sidecars32f(&file.0);
}
#[test]
fn native_n1_vfs32f_fchmod_deadline_before_delegate() { metadata_deadline32f("native_n1_vfs32f_fchmod_deadline_before_delegate", true); }
#[test]
fn native_n1_vfs32f_fchown_deadline_before_delegate() { metadata_deadline32f("native_n1_vfs32f_fchown_deadline_before_delegate", false); }

#[test]
fn native_n1_vfs32f_sleep_expiry_next_lock_is_not_busy_but_unlock_works() {
    if !native_child("native_n1_vfs32f_sleep_expiry_next_lock_is_not_busy_but_unlock_works") { return; }
    let h = install().unwrap();
    let (file, pins, pin, b0) = wal_fixture("sleep-lock");
    let (gate, account, clock) = gate_with(1000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let handle = native_file32f(&session);
    let vfs = unsafe { rusqlite::ffi::sqlite3_vfs_find(session.state.as_ref().unwrap().vfs.as_ref().unwrap().name()) };
    let tx = session.begin().unwrap();
    let method = unsafe { (*(*handle).pMethods).xShmLock.unwrap() };
    let exclusive = rusqlite::ffi::SQLITE_SHM_EXCLUSIVE;
    assert_eq!(unsafe { method(handle, 7, 1, rusqlite::ffi::SQLITE_SHM_LOCK | exclusive) }, rusqlite::ffi::SQLITE_OK);
    clock.set(clock.get() + Duration::from_secs(2));
    assert_eq!(unsafe { (*vfs).xSleep.unwrap()(vfs, 1000) }, 0);
    assert_eq!(unsafe { method(handle, 7, 1, rusqlite::ffi::SQLITE_SHM_LOCK | exclusive) }, rusqlite::ffi::SQLITE_IOERR_LOCK);
    assert_eq!(unsafe { method(handle, 7, 1, rusqlite::ffi::SQLITE_SHM_UNLOCK | exclusive) }, rusqlite::ffi::SQLITE_OK);
    drop(tx);
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Job(_)))));
    assert_eq!(account.borrow().snapshot().open_reservations, 0); assert_sidecars32f(&file.0);
}

// TestOnly 全域物理基线：用于被现役E6拒绝的WAL夹具，不替换生产MeasuredDomain。
fn wal_existing32f(db: &Path, state: &Path) -> Existing {
    use std::collections::BTreeSet;
    use std::os::unix::fs::MetadataExt;
    let parent = db.parent().unwrap();
    let base = db.file_name().unwrap().as_encoded_bytes();
    let mut queue = vec![state.to_path_buf()];
    let mut seen = BTreeSet::new();
    seen.insert(parent.to_path_buf());
    let mut bytes = std::fs::metadata(parent).unwrap().blocks() * 512;
    for item in std::fs::read_dir(parent).unwrap() {
        let item = item.unwrap(); let name = item.file_name(); let raw = name.as_encoded_bytes();
        if raw == base || (raw.starts_with(base) && matches!(raw.get(base.len()), Some(b'.' | b'-'))) { queue.push(item.path()); }
    }
    while let Some(path) = queue.pop() {
        if !seen.insert(path.clone()) { continue; }
        let meta = std::fs::metadata(&path).unwrap(); bytes += meta.blocks() * 512;
        if meta.is_dir() { queue.extend(std::fs::read_dir(path).unwrap().map(|e| e.unwrap().path())); }
    }
    Existing { store_bytes: bytes, staged_bytes: 0 }
}

fn s3_wal32f() -> (crate::tests::TestFiles, [DevIno; 3], std::fs::File, u64) {
    use std::os::unix::fs::MetadataExt;
    let files = native_s3_fixture();
    let conn = rusqlite::Connection::open(files.db()).unwrap();
    let mut enabled = 0;
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_db_config(conn.handle(), rusqlite::ffi::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, 1, &mut enabled) }, rusqlite::ffi::SQLITE_OK);
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA user_version=0;").unwrap(); conn.close().unwrap();
    let paths = [files.db(), PathBuf::from(format!("{}-wal", files.db().display())), PathBuf::from(format!("{}-shm", files.db().display()))];
    let pins = paths.each_ref().map(|path| { let m = std::fs::metadata(path).unwrap(); (m.dev(), m.ino()) });
    let pin = std::fs::File::open(&paths[2]).unwrap(); let b0 = pin.metadata().unwrap().blocks() * 512;
    (files, pins, pin, b0)
}
#[test]
fn native_n1_vfs32f_real_s3_audit_seal_destroy_finish_stays_same_job() {
    if !native_child("native_n1_vfs32f_real_s3_audit_seal_destroy_finish_stays_same_job") { return; }
    let h = install().unwrap();
    let (files, pins, pin, b0) = s3_wal32f();
    let consumed = super::super::test_prepare(&files.db()).consume().unwrap();
    let refusal = crate::v3::admission_provider::MeasuredDomain::measure(&consumed).err().expect("现役E6仍拒SHM");
    assert!(refusal.contains("UnsupportedWalShmJournalCombination"), "{refusal}");
    let existing = wal_existing32f(&files.db(), consumed.lease().state_path());
    let started = consumed.into_job(existing).unwrap();
    let gate = started.native_io_gate().unwrap(); let account = Rc::clone(&gate.job);
    let before = account.borrow().snapshot();
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&files.db(), pin).unwrap();
    let tx = session.begin().unwrap(); let receipt = tx.audit().unwrap();
    assert_eq!(receipt.delta, crate::DeltaShape::Current);
    let audited = account.borrow().snapshot();
    assert!(audited.nodes > before.nodes && audited.records > before.records);
    assert_eq!(audited.wire_bytes - before.wire_bytes, h.actual_io.load(Ordering::SeqCst) as u128, "S3同一连接物理IO全部入原Job");
    assert!(audited.open_reservations > 0);
    let mut errors = NativeErrors::default();
    let mut finished = tx.seal(started, &receipt, &mut errors).expect("E8自动清算，无人工settle");
    assert!(Rc::ptr_eq(&account, &finished.job));
    assert_eq!(account.borrow().snapshot().open_reservations, 0);
    assert_eq!(with_owner(|tls| tls.gate.phase()), Some(Phase::Writer));
    let error = tx.accept(&receipt, &native_input(), &crate::tests::fixture("profiles/testonly_tick_1_1_ohlc.json"), crate::DEFAULT_WRITER_EPOCH, None).unwrap_err();
    assert!(error.starts_with("VersionMismatch"), "S3审计不能冒充S2写资格：{error}");
    tx.rollback().unwrap();
    let destroyed = match session.close(&mut errors) { Teardown::Destroyed(r) => r, _ => panic!("活连接不能签销毁收据") };
    assert!(Rc::ptr_eq(&destroyed.job, &account));
    n1_finish(&mut finished, &destroyed).unwrap();
    assert!(errors.reported().is_none(), "{errors:?}"); assert_sidecars32f(&files.db());
}

#[test]
fn native_n1_vfs32f_close_borrowed_job_keeps_mapping_grant_and_callbacks() {
    if !native_child("native_n1_vfs32f_close_borrowed_job_keeps_mapping_grant_and_callbacks") { return; }
    let h = install().unwrap();
    let (file, pins, pin, b0) = wal_fixture("borrowed-close");
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let tx = session.begin().unwrap(); tx.rollback().unwrap();
    let fd = shm_fd32f(h);
    let held = account.borrow_mut(); let grants = held.snapshot().open_reservations;
    assert!(grants > 0);
    let mut errors = NativeErrors::default();
    let session = match session.close(&mut errors) { Teardown::Alive(s) => s, _ => panic!("未结Grant必须保活") };
    assert_eq!(held.snapshot().open_reservations, grants); assert!(fd_open(fd));
    let state = session.state.as_ref().unwrap();
    assert!(state.vfs.is_some() && state.progress.is_some() && state.authorizer.is_some());
    assert!(with_owner(|tls| tls.budget.grant_count()).unwrap() > 0);
    drop(held);
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Code(NativeCode::GateBorrowed)))));
    assert_eq!(account.borrow().snapshot().open_reservations, 0); assert!(!fd_open(fd));
    assert_sidecars32f(&file.0);
}

#[test]
fn native_n1_vfs32f_failed_first_seal_cleanup_keep_priority_on_real_connection() {
    use std::os::fd::AsRawFd;
    if !native_child("native_n1_vfs32f_failed_first_seal_cleanup_keep_priority_on_real_connection") { return; }
    let h = fault_hooks32f();
    let (files, pins, pin, b0) = s3_wal32f(); let pin_fd = pin.as_raw_fd();
    let consumed = super::super::test_prepare(&files.db()).consume().unwrap();
    let existing = wal_existing32f(&files.db(), consumed.lease().state_path());
    let started = consumed.into_job(existing).unwrap();
    let gate = started.native_io_gate().unwrap(); let account = Rc::clone(&gate.job);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&files.db(), pin).unwrap();
    let raw = session.state.as_ref().unwrap().raw;
    let tx = session.begin().unwrap(); let receipt = tx.audit().unwrap();
    let mut statement = std::ptr::null_mut();
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_prepare_v2(raw, c"SELECT 1".as_ptr(), -1, &mut statement, std::ptr::null_mut()) }, rusqlite::ffi::SQLITE_OK);
    let foreign_grant = account.borrow_mut().reserve_store(1).unwrap();
    FAULTS32F.with(|f| f.borrow_mut().pin_fail = Some(pin_fd));
    let mut errors = NativeErrors::default();
    assert!(tx.seal(started, &receipt, &mut errors).is_none());
    assert!(errors.seal.is_some(), "未结非SHM Grant产生独立seal错误");
    drop(tx);
    let session = match session.close(&mut errors) { Teardown::Alive(s) => s, _ => panic!("真实statement BUSY") };
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Code(NativeCode::ShmMeasureFailed)))));
    assert!(errors.cleanup().contains(&NativeError::Code(NativeCode::CloseBusy)));
    assert!(session.state.as_ref().unwrap().progress.is_some() && session.state.as_ref().unwrap().authorizer.is_some());
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_finalize(statement) }, rusqlite::ffi::SQLITE_OK);
    account.borrow_mut().release(&foreign_grant).unwrap();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert_eq!(account.borrow().snapshot().open_reservations, 0);
    assert!(matches!(errors.reported(), Some(Reported::First(NativeError::Code(NativeCode::ShmMeasureFailed)))));
    assert_sidecars32f(&files.db());
}

struct DmsHolder32f(std::process::Child);
impl DmsHolder32f {
    fn new(path: &Path) -> Self {
        use std::io::BufRead;
        let mut child = std::process::Command::new("python3").args(["-c",
            "import fcntl,sys\nf=open(sys.argv[1],'r+b')\nfcntl.lockf(f,fcntl.LOCK_SH,1,128)\nprint('ready',flush=True)\nsys.stdin.read()\n"])
            .arg(path).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
        let mut line = String::new(); std::io::BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "ready"); Self(child)
    }
}
impl Drop for DmsHolder32f { fn drop(&mut self) { drop(self.0.stdin.take()); assert!(self.0.wait().unwrap().success()); } }
#[test]
fn native_n1_vfs32f_first_bextend0_map_holds_budget_before_writable_mapping() {
    if !native_child("native_n1_vfs32f_first_bextend0_map_holds_budget_before_writable_mapping") { return; }
    let h = fault_hooks32f();
    let (file, pins, pin, b0) = wal_fixture("first-map");
    // 同外部读者持DMS共享锁，真实unixShmMap不能先truncate已存在的足长SHM。
    let _reader = DmsHolder32f::new(&PathBuf::from(format!("{}-shm", file.0.display())));
    let (gate, account, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let session = owner.open_native(&file.0, pin).unwrap();
    assert_eq!(with_owner(|tls| tls.budget.grant_count()), Some(0));
    let handle = native_file32f(&session);
    let (rc, out) = unsafe { map32f(handle, 0, 0) };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK); assert!(!out.is_null());
    assert_eq!(FAULTS32F.with(|f| f.borrow().maps.clone()), vec![(32768, 1)]);
    assert_eq!(account.borrow().snapshot().open_reservations, 1, "零量grant仍是失败清算载体");
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(errors.reported().is_none(), "{errors:?}"); assert_eq!(account.borrow().snapshot().open_reservations, 0);
    assert_sidecars32f(&file.0);
}
fn wal_writer_locked32f(path: &Path) -> bool {
    let status = std::process::Command::new("python3").args(["-c",
        "import fcntl,sys\nf=open(sys.argv[1],'r+b')\ntry:\n fcntl.lockf(f,fcntl.LOCK_EX|fcntl.LOCK_NB,1,120)\nexcept OSError:\n sys.exit(3)\n"])
        .arg(format!("{}-shm", path.display())).status().unwrap();
    match status.code() { Some(3) => true, Some(0) => false, other => panic!("WAL锁探针 {other:?}") }
}
#[test]
fn native_n1_vfs32f_rejected_peer_and_deferred_close_keep_real_wal_writer_lock() {
    if !native_child("native_n1_vfs32f_rejected_peer_and_deferred_close_keep_real_wal_writer_lock") { return; }
    let h = install().unwrap();
    let (file, pins, pin, b0) = wal_fixture("real-peer-lock");
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap();
    let tx = session.begin().unwrap(); assert!(wal_writer_locked32f(&file.0));
    let path = file.c();
    assert_eq!(std::thread::spawn(move || unsafe { h.open(path.as_ptr(), libc::O_RDWR, 0) }).join().unwrap(), -1);
    assert!(wal_writer_locked32f(&file.0), "打开前拒绝未破坏实际BEGIN IMMEDIATE锁");
    let shm = CString::new(format!("{}-shm", file.0.display())).unwrap();
    let peer = unsafe { libc::open(shm.as_ptr(), libc::O_RDWR) }; assert!(peer >= 0);
    assert_eq!(unsafe { h.close(peer) }, 0);
    assert!(fd_open(peer) && h.deferred_for_test().contains(&peer));
    assert!(wal_writer_locked32f(&file.0), "未知同inode FD的close延迟，真实WAL写锁仍在");
    drop(tx);
    let mut errors = NativeErrors::default();
    assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(!fd_open(peer)); assert!(!wal_writer_locked32f(&file.0)); assert_sidecars32f(&file.0);
}
#[test]
fn native_n1_vfs32f_active_poison_close_cannot_drop_real_owner_lock() {
    if !native_child("native_n1_vfs32f_active_poison_close_cannot_drop_real_owner_lock") { return; }
    let h = install().unwrap();
    let (file, pins, pin, b0) = wal_fixture("poison-lock");
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let owner = NativeOwner::begin(h, gate, pins, b0, identity_bound()).unwrap();
    let mut session = owner.open_native(&file.0, pin).unwrap(); let tx = session.begin().unwrap();
    let owner_fd = shm_fd32f(h); assert!(wal_writer_locked32f(&file.0));
    h.poison_for_test();
    assert_eq!(std::thread::spawn(move || unsafe { h.close(owner_fd) }).join().unwrap(), 0);
    assert!(fd_open(owner_fd) && wal_writer_locked32f(&file.0), "中毒不可透传跨线程owner FD close");
    drop(tx);
    let mut errors = NativeErrors::default(); assert!(matches!(session.close(&mut errors), Teardown::Destroyed(_)));
    assert!(errors.reported().is_some()); assert_sidecars32f(&file.0);
}
