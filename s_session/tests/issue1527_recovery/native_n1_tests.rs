//! #1527 31Q TestOnly：N1 受限候选的合成单元/反例锁。只用临时文件、内存 SQLite 与独立
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
fn native_n1_l3a_close_busy_keeps_callback_context_then_destroys() {
    let (gate, _, _) = gate_with(60_000, 1 << 20);
    let h = hooks();
    let owner = NativeOwner::begin(h, gate, [(1, 1), (0, 1), (0, 2)], 0, identity_bound()).unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let mut stmt: *mut rusqlite::ffi::sqlite3_stmt = std::ptr::null_mut();
    let rc = unsafe {
        rusqlite::ffi::sqlite3_prepare_v2(
            conn.handle(),
            c"select 1".as_ptr(),
            -1,
            &mut stmt,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    let mut errors = NativeErrors::default();
    let bundle = match owner.teardown(conn, &mut errors) {
        Teardown::Alive(bundle) => bundle,
        Teardown::Destroyed(_) => panic!("未 finalize 语句时 sqlite3_close 必 BUSY"),
    };
    assert_eq!(errors.cleanup(), &[NativeError::Code(NativeCode::CloseBusy)]);
    assert!(with_owner(|_| ()).is_some(), "BUSY：gate/TLS 保留");
    assert!(h.lock().active.is_some(), "BUSY：登记与延迟 FD 保留");
    assert_eq!(unsafe { rusqlite::ffi::sqlite3_finalize(stmt) }, rusqlite::ffi::SQLITE_OK);
    let AliveBundle { conn, owner } = bundle;
    assert!(matches!(owner.teardown(conn, &mut errors), Teardown::Destroyed(_)));
    assert!(with_owner(|_| ()).is_none());
    assert!(h.lock().active.is_none() && !h.active_any.load(Ordering::SeqCst));
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
    let mut connection = crate::open_db(&files.db()).unwrap();
    let tx = connection.transaction().unwrap();
    let error = owner.audit_in(&tx).err().expect("旧证据图没有非 mixed 同源费用适配，不发收据");
    assert!(error.starts_with("LegacyAdmissionBudgetAdapterMissing"), "{error}");
    tx.rollback().unwrap();
    let mut errors = NativeErrors::default();
    assert!(matches!(owner.teardown(connection, &mut errors), Teardown::Destroyed(_)));
}

fn native_s3_fixture() -> crate::tests::TestFiles {
    let files = crate::tests::TestFiles::new();
    let clock = files.db().with_extension("clock.json");
    std::fs::write(&clock, serde_json::json!({
        "schema_revision":"s-clock-plan/1", "clock_plan_id":"native-n1-31v",
        "origin_utc":"2026-09-18T00:00:00Z", "unit":"ns",
        "events":{"input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"}]}}
    }).to_string()).unwrap();
    crate::v3::init_v3(&files.db(), "s-session-testonly-001",
        &crate::tests::fixture("catalog/signed-catalog.json"),
        &crate::tests::fixture("profiles/ohlc_integer_tb02a_v1.json"), "1", &clock, 4).unwrap();
    let connection = rusqlite::Connection::open(files.db()).unwrap();
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;").unwrap();
    connection.close().unwrap();
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
    let mut connection = rusqlite::Connection::open(files.db()).unwrap();
    let tx = connection.transaction().unwrap();
    let receipt = owner.audit_in(&tx).unwrap();
    assert_eq!(receipt.delta, crate::DeltaShape::Current);
    let after = account.borrow().snapshot();
    assert!(after.nodes > before.nodes && after.node_bytes > before.node_bytes);
    assert!(after.records > before.records);
    let mut errors = NativeErrors::default();
    owner.settle_shm(Ok(0)).unwrap();
    let mut finished = owner.seal(started, &mut errors).expect("同一本 Job 完成 seal");
    assert!(Rc::ptr_eq(&account, &finished.job));
    owner.enter_writer();
    let error = owner.accept_in(&tx, &receipt, &native_input(),
        &crate::tests::fixture("profiles/testonly_tick_1_1_ohlc.json"), crate::DEFAULT_WRITER_EPOCH, None).unwrap_err();
    assert!(error.starts_with("VersionMismatch"), "S3审计成功不能充作S2写资格：{error}");
    tx.rollback().unwrap();
    let destroyed = match owner.teardown(connection, &mut errors) {
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
    let mut tls = OwnerTls { token: 1, gate, budget: ShmBudget::new(0, identity_bound()).unwrap(), first: None };
    assert!(matches!(owner_step(&mut tls, Class::Main, None, Effect::Write { end: 1 }), Step::Pass));
    tls.fail(NativeCode::TrackingLost.into());
    assert!(matches!(owner_step(&mut tls, Class::Main, None, Effect::Write { end: 1 }), Step::Deny(libc::EIO)));
}
