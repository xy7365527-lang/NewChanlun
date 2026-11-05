//! #1419 M8D 3B：SQLite 同父分配前收费合同，对真实 Native 父账（`native::Budget`）验收。
//! allocator 只能在本进程首次 SQLite 初始化前安装，所以每条测试须在全新进程中以
//! `--ignored --exact` 单独运行（CI 经 `.github/1488-uo-release-locks.txt` 逐条执行）。
use super::native::Budget;
use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
use newchan_rust::s_content::capture_allocation::sqlite_parent::{
    charged_bytes, SqliteBinding, SqliteParentAllocator,
};
use rusqlite::{ffi, Connection, ErrorCode};
use std::ptr;

const LIMIT: usize = 64 << 20;
const ROOM: usize = 16 << 20;

struct Domains<'a> {
    allocator: &'a SqliteParentAllocator,
    current: Budget,
    request: u64,
}

impl Domains<'_> {
    /// 换到下一个冻结域并把 SQLite 收费绑定过去。
    fn next(&mut self, allowance: usize) -> SqliteBinding {
        self.advance(allowance);
        let (parent, origin) = self.current.sqlite_parent();
        self.allocator.bind(parent, origin).expect("bind")
    }
    /// 只换域、不换绑：旧绑定随即过期。
    fn advance(&mut self, allowance: usize) {
        self.request += 1;
        self.current = self
            .current
            .handoff(self.request, allowance)
            .expect("handoff to the next frozen domain");
    }
}

fn nomem(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(ErrorCode::OutOfMemory)
}

/// 小语句的 VM 落在连接打开时已收费的 lookaside 里，不新增堆块；
/// 长 IN 列表使 VM 操作数组超出 lookaside 槽，prepare 必须申请新的堆块。
fn heap_query(head: &str) -> String {
    let list: Vec<String> = (0..400).map(|i| i.to_string()).collect();
    format!("{head} AND 0 NOT IN ({})", list.join(","))
}

#[test]
#[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
fn sqlite_parent_charges_same_parent_before_allocation() {
    // SAFETY: this test runs alone in a fresh process before any SQLite use.
    let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
    assert!(unsafe { SqliteParentAllocator::install() }.is_err());

    // 未绑定：SQLite 初始化与打开都没有归属，整体拒绝且不旁路到系统 malloc。
    let unattributed = allocator.unattributed_refusals();
    let error = Connection::open_in_memory().expect_err("unbound open refused");
    assert!(nomem(&error), "{error}");
    assert!(allocator.unattributed_refusals() > unattributed);

    let root = Budget::fresh(LIMIT).unwrap();
    let base = root.live();
    let mut d = Domains {
        allocator: &allocator,
        current: root.bind_frozen(1, LIMIT, 0, 0).unwrap(),
        request: 1,
    };
    let (parent, origin) = d.current.sqlite_parent();
    let wide = allocator.bind(parent, origin).unwrap();

    let dir = std::env::temp_dir().join(format!("sqlite-parent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let main = Connection::open(dir.join("main.sqlite3")).unwrap();
    main.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE t(id INTEGER PRIMARY KEY, body BLOB NOT NULL);
         INSERT INTO t VALUES(1, zeroblob(4096));
         INSERT INTO t VALUES(2, zeroblob(200000));",
    )
    .unwrap();
    let ledger = Connection::open(dir.join("ledger.sqlite3")).unwrap();
    ledger
        .execute_batch(
            "CREATE TABLE k(key BLOB PRIMARY KEY) WITHOUT ROWID;
             INSERT INTO k VALUES(x'01');",
        )
        .unwrap();
    // 两条连接（主库与台账）及 SQLite 全局状态的全部存活字节都在同一父账上，且只在那里。
    assert!(wide.live_bytes() > 0);
    assert_eq!(d.current.live() - base, wide.live_bytes());
    println!(
        "SQLITE_PARENT_OPEN live_bytes={} blocks={}",
        wide.live_bytes(),
        wide.live_blocks()
    );

    // 块头与对齐一并收费：一次 80 B 申请恰收 charged_bytes(80)。
    let before = d.current.live();
    let block = unsafe { ffi::sqlite3_malloc(80) };
    assert!(!block.is_null());
    assert_eq!(d.current.live() - before, charged_bytes(80).unwrap());
    assert_eq!(unsafe { ffi::sqlite3_msize(block) }, 80);
    unsafe { ffi::sqlite3_free(block) };
    assert_eq!(d.current.live(), before);

    // lookaside：打开连接时已整体收费；零额度域内的小 prepare 只用它，不新增费用。
    let prepaid = d.next(0);
    let live = d.current.live();
    drop(main.prepare("SELECT body FROM t WHERE id = 2").unwrap());
    assert_eq!(prepaid.refusal(), None);
    assert_eq!(d.current.live(), live);

    // prepare：需要新堆块时在零额度域内被拒，首个拒绝是 Budget，账不残留。
    let prepare = d.next(0);
    let live = d.current.live();
    let error = main
        .prepare(&heap_query("SELECT body FROM t WHERE id = 2"))
        .expect_err("prepare refused");
    assert!(nomem(&error), "{error}");
    assert_eq!(prepare.refusal(), Some(ScratchFailure::Budget));
    assert_eq!(prepare.refusal_of(&error), Some(ScratchFailure::Budget));
    assert_eq!(d.current.live(), live);

    // 首个拒绝后本绑定不再放行任何分配，包括本可容纳的小块与后续 SELECT。
    let latched = d.next(charged_bytes(4096).unwrap());
    let live = d.current.live();
    assert!(unsafe { ffi::sqlite3_malloc(8192) }.is_null());
    assert_eq!(latched.refusal(), Some(ScratchFailure::Budget));
    assert!(d.current.remaining() >= charged_bytes(16).unwrap());
    assert!(unsafe { ffi::sqlite3_malloc(16) }.is_null());
    let error = ledger
        .prepare(&heap_query("SELECT EXISTS(SELECT 1 FROM k WHERE key = ?1)"))
        .expect_err("no SELECT after the first refusal");
    assert!(nomem(&error), "{error}");
    assert_eq!(d.current.live(), live);

    // step：语句在宽域准备，换到零额度域后取跨页大行被拒。
    let _roomy = d.next(ROOM);
    let mut statement = main.prepare("SELECT body FROM t WHERE id = 2").unwrap();
    let step = d.next(0);
    let live = d.current.live();
    let error = statement
        .query_row([], |row| row.get::<_, Vec<u8>>(0))
        .expect_err("step refused");
    assert!(nomem(&error), "{error}");
    assert_eq!(step.refusal(), Some(ScratchFailure::Budget));
    assert_eq!(d.current.live(), live);
    drop(statement);

    // 列取值：已在 step 时物化（并收费）的 BLOB，零额度域内 column_blob 不再新增费用；
    // 需要另建缓冲的取值（BLOB 按 TEXT 取需追加终止符）在零额度域内返回空指针并报 NOMEM。
    let _roomy = d.next(ROOM);
    let mut raw = ptr::null_mut();
    assert_eq!(
        unsafe {
            ffi::sqlite3_prepare_v2(
                main.handle(),
                c"SELECT zeroblob(65536), randomblob(50000)".as_ptr(),
                -1,
                &mut raw,
                ptr::null_mut(),
            )
        },
        ffi::SQLITE_OK
    );
    assert_eq!(unsafe { ffi::sqlite3_step(raw) }, ffi::SQLITE_ROW);
    let column = d.next(0);
    let live = d.current.live();
    assert!(!unsafe { ffi::sqlite3_column_blob(raw, 0) }.is_null());
    assert_eq!(unsafe { ffi::sqlite3_column_bytes(raw, 0) }, 65536);
    assert_eq!(column.refusal(), None);
    assert_eq!(d.current.live(), live);
    assert!(unsafe { ffi::sqlite3_column_text(raw, 1) }.is_null());
    assert_eq!(
        unsafe { ffi::sqlite3_errcode(main.handle()) },
        ffi::SQLITE_NOMEM
    );
    assert_eq!(column.refusal(), Some(ScratchFailure::Budget));
    assert_eq!(d.current.live(), live);
    let _roomy = d.next(ROOM);
    unsafe { ffi::sqlite3_finalize(raw) };

    // 台账点查与新开连接同样在原父账内被拒。
    let point = d.next(0);
    let error = ledger
        .prepare(&heap_query("SELECT EXISTS(SELECT 1 FROM k WHERE key = ?1)"))
        .expect_err("ledger lookup refused");
    assert!(nomem(&error), "{error}");
    assert_eq!(point.refusal(), Some(ScratchFailure::Budget));
    let open = d.next(0);
    let error = Connection::open(dir.join("third.sqlite3")).expect_err("open refused");
    assert!(nomem(&error), "{error}");
    assert_eq!(open.refusal(), Some(ScratchFailure::Budget));

    // 换域而不换绑：旧绑定过期，按未知归属拒绝（Ledger），不扣新域。
    let stale = d.next(ROOM);
    d.advance(ROOM);
    let live = d.current.live();
    assert!(unsafe { ffi::sqlite3_malloc(16) }.is_null());
    assert_eq!(stale.refusal(), Some(ScratchFailure::Ledger));
    assert_eq!(d.current.live(), live);

    // realloc：新块在旧块存活时完整预留；差 1 B 即拒且旧块不变，
    // 恰好够时成功；旧块按旧 origin 返还，不回补当前域额度。
    let _roomy = d.next(ROOM);
    let old = unsafe { ffi::sqlite3_malloc(80) };
    assert!(!old.is_null());
    unsafe { ptr::write_bytes(old.cast::<u8>(), 0x5a, 80) };
    let need = charged_bytes(160).unwrap();
    let short = d.next(need - 1);
    let live = d.current.live();
    assert!(unsafe { ffi::sqlite3_realloc(old, 160) }.is_null());
    assert_eq!(short.refusal(), Some(ScratchFailure::Budget));
    assert_eq!(unsafe { ffi::sqlite3_msize(old) }, 80);
    assert_eq!(
        unsafe { std::slice::from_raw_parts(old.cast::<u8>(), 80) },
        &[0x5a; 80]
    );
    assert_eq!(d.current.live(), live);
    let _exact = d.next(need);
    let grown = unsafe { ffi::sqlite3_realloc(old, 160) };
    assert!(!grown.is_null());
    assert_eq!(
        unsafe { std::slice::from_raw_parts(grown.cast::<u8>(), 80) },
        &[0x5a; 80]
    );
    assert_eq!(d.current.remaining(), 0);
    assert_eq!(d.current.live(), live - charged_bytes(80).unwrap() + need);
    unsafe { ffi::sqlite3_free(grown) };
    assert_eq!(d.current.remaining(), need);

    // 错父：另一父账名下开出的连接，换绑回主父账后关闭，块只回到原父账。
    let other_root = Budget::fresh(LIMIT).unwrap();
    let other_base = other_root.live();
    let other = other_root.bind_frozen(1, LIMIT, 0, 0).unwrap();
    let (other_parent, other_origin) = other.sqlite_parent();
    let foreign_binding = allocator.bind(other_parent, other_origin).unwrap();
    let foreign = Connection::open_in_memory().unwrap();
    foreign
        .execute_batch("CREATE TABLE f(x BLOB); INSERT INTO f VALUES(zeroblob(50000));")
        .unwrap();
    assert!(foreign_binding.live_bytes() > 0);
    assert_eq!(other.live() - other_base, foreign_binding.live_bytes());
    let home = d.next(ROOM);
    let main_live = d.current.live();
    let foreign_live = foreign_binding.live_bytes();
    drop(foreign);
    assert!(foreign_binding.live_bytes() < foreign_live);
    assert_eq!(other.live() - other_base, foreign_binding.live_bytes());
    assert_eq!(d.current.live(), main_live);
    drop(foreign_binding);

    // 关闭全部连接并 shutdown 后，主父账回到出生读数，没有残留或破损。
    drop(main);
    drop(ledger);
    assert_eq!(unsafe { ffi::sqlite3_shutdown() }, ffi::SQLITE_OK);
    assert_eq!(home.live_bytes(), 0);
    assert_eq!(home.live_blocks(), 0);
    assert_eq!(d.current.live(), base);
    assert!(!allocator.is_broken());
    println!(
        "SQLITE_PARENT_FINAL base={base} live={} remaining={} requests={}",
        d.current.live(),
        d.current.remaining(),
        d.request
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "fresh process only: initializes SQLite before install; run alone with --ignored --exact"]
fn sqlite_parent_install_refused_after_sqlite_initialized() {
    assert_eq!(unsafe { ffi::sqlite3_initialize() }, ffi::SQLITE_OK);
    // SAFETY: no concurrent SQLite use in this fresh process.
    let error = unsafe { SqliteParentAllocator::install() }
        .err()
        .expect("install after initialization refused");
    assert_eq!(error.sqlite_error_code(), Some(ErrorCode::ApiMisuse));
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE x(y)").unwrap();
}
