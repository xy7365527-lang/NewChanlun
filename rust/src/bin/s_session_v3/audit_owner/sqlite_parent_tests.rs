//! #1419 M8D 3B：SQLite 同父分配前收费与操作域合同，对真实 Native 父账（`native::Budget`）验收。
//! allocator 只能在本进程首次 SQLite 初始化前安装，所以每条测试须在全新进程中以
//! `--ignored --exact` 单独运行（CI 经 `.github/1488-uo-release-locks.txt` 逐条执行）。
use super::native::Budget;
use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
use newchan_rust::s_content::capture_allocation::sqlite_parent::{
    charged_bytes, SqlGate, SqlOps, SqliteBinding, SqliteParentAllocator,
};
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef};
use rusqlite::{ffi, Connection, ErrorCode};
use std::cell::Cell;
use std::ffi::{c_int, c_uint, c_void, CStr};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 主库连接上每条语句开始执行（`SQLITE_TRACE_STMT`）的次数：进入 SQLite 执行的探针。
static ENTERED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn count_entry(
    _: c_uint,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
) -> c_int {
    ENTERED.fetch_add(1, Ordering::SeqCst);
    0
}

fn entered() -> usize {
    ENTERED.load(Ordering::SeqCst)
}

const LIMIT: usize = 64 << 20;
const ROOM: usize = 16 << 20;

struct Domains<'a> {
    allocator: &'a SqliteParentAllocator,
    current: Budget,
    request: u64,
}

impl Domains<'_> {
    /// 换到下一个冻结域并把 SQLite 操作域绑定过去。
    fn next(&mut self, allowance: usize) -> SqliteBinding {
        self.advance(allowance);
        self.rebind()
    }
    /// 对当前冻结域（同父同 origin）重新绑定。
    fn rebind(&self) -> SqliteBinding {
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

/// 绑定参数时尝试让父账换域的 `ToSql`；记录 handoff 是否成功。
struct Handoff<'a> {
    budget: &'a Budget,
    request: u64,
    tried: &'a Cell<Option<bool>>,
}

impl ToSql for Handoff<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let moved = self.budget.handoff(self.request, ROOM).is_ok();
        self.tried.set(Some(moved));
        Ok(ToSqlOutput::from(7i64))
    }
}

/// 借出调用方大 BLOB 的参数，记 `to_sql` 被调用的次数。
struct Counted<'a> {
    blob: &'a [u8],
    calls: &'a Cell<usize>,
}

impl ToSql for Counted<'_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.calls.set(self.calls.get() + 1);
        Ok(ToSqlOutput::Borrowed(ValueRef::Blob(self.blob)))
    }
}

/// 绑定参数时在同一操作域里触发收费拒绝并吞掉错误的 `ToSql`。
struct Refuse<'q, 'a> {
    q: &'q SqlOps<'a>,
    hit: &'q Cell<Option<bool>>,
}

impl ToSql for Refuse<'_, '_> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let refused = self
            .q
            .prepare(&heap_query("SELECT body FROM t WHERE id = 2"))
            .map(drop);
        self.hit.set(Some(matches!(
            refused,
            Err(SqlGate::Refused(ScratchFailure::Budget))
        )));
        Ok(ToSqlOutput::from(1i64))
    }
}

fn nomem(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(ErrorCode::OutOfMemory)
}

fn refused<T: std::fmt::Debug>(result: Result<T, SqlGate>) -> ScratchFailure {
    match result {
        Err(SqlGate::Refused(first)) => first,
        other => panic!("expected a charging refusal, got {other:?}"),
    }
}

/// 长 IN 列表使 VM 操作数组超出 lookaside 槽，prepare 必须申请新的堆块。
fn heap_query(head: &str) -> String {
    let list: Vec<String> = (0..400).map(|i| i.to_string()).collect();
    format!("{head} AND 0 NOT IN ({})", list.join(","))
}

/// 在连接上准备一条裸语句，交给后续域里单独 step / column / finalize。
fn raw_prepare(q: &SqlOps<'_>, sql: &CStr) -> Result<usize, SqlGate> {
    let mut raw = ptr::null_mut();
    let rc =
        unsafe { ffi::sqlite3_prepare_v2(q.handle(), sql.as_ptr(), -1, &mut raw, ptr::null_mut()) };
    if rc == ffi::SQLITE_OK {
        Ok(raw as usize)
    } else {
        Err(SqlGate::Sqlite(unsafe {
            ffi::sqlite3_extended_errcode(q.handle())
        }))
    }
}

fn stmt(raw: usize) -> *mut ffi::sqlite3_stmt {
    raw as *mut ffi::sqlite3_stmt
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
    let wide = d.rebind();

    let dir = std::env::temp_dir().join(format!("sqlite-parent-{}", std::process::id()));
    let path = |name: &str| std::ffi::CString::new(dir.join(name).to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let main = wide.open(&path("main.sqlite3")).unwrap();
    wide.run(&main, |q| {
        let rc = unsafe {
            ffi::sqlite3_trace_v2(
                q.handle(),
                ffi::SQLITE_TRACE_STMT,
                Some(count_entry),
                ptr::null_mut(),
            )
        };
        assert_eq!(rc, ffi::SQLITE_OK);
        q.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE t(id INTEGER PRIMARY KEY, body BLOB NOT NULL);
             INSERT INTO t VALUES(1, zeroblob(4096));
             INSERT INTO t VALUES(2, zeroblob(200000));",
        )
    })
    .unwrap();
    let ledger = wide.open(&path("ledger.sqlite3")).unwrap();
    wide.run(&ledger, |q| {
        q.execute_batch(
            "CREATE TABLE k(key BLOB PRIMARY KEY) WITHOUT ROWID;
             INSERT INTO k VALUES(x'01');",
        )
    })
    .unwrap();
    // 两条连接（主库与台账）及 SQLite 全局状态的全部存活字节都在同一父账上，且只在那里。
    assert!(wide.live_bytes() > 0);
    assert_eq!(d.current.live() - base, wide.live_bytes());
    println!(
        "SQLITE_PARENT_OPEN live_bytes={} blocks={}",
        wide.live_bytes(),
        wide.live_blocks()
    );

    // 操作域外：即使已绑定，不在 run 中的分配也按未知归属拒绝，不收费。
    let live = d.current.live();
    let unattributed = allocator.unattributed_refusals();
    assert!(unsafe { ffi::sqlite3_malloc(16) }.is_null());
    assert!(allocator.unattributed_refusals() > unattributed);
    assert_eq!(d.current.live(), live);

    // 块头与对齐一并收费：一次 80 B 申请恰收 charged_bytes(80)。
    let before = d.current.live();
    wide.run(&main, |_| {
        let block = unsafe { ffi::sqlite3_malloc(80) };
        assert!(!block.is_null());
        assert_eq!(d.current.live() - before, charged_bytes(80).unwrap());
        assert_eq!(unsafe { ffi::sqlite3_msize(block) }, 80);
        unsafe { ffi::sqlite3_free(block) };
        Ok(())
    })
    .unwrap();
    assert_eq!(d.current.live(), before);

    // lookaside：打开连接时已整体收费；零额度域内的小 prepare 只用它，不新增费用。
    let prepaid = d.next(0);
    let live = d.current.live();
    prepaid
        .run(&main, |q| {
            q.prepare("SELECT body FROM t WHERE id = 2").map(drop)
        })
        .unwrap();
    assert_eq!(prepaid.refusal(), None);
    assert_eq!(d.current.live(), live);

    // B1 反例的前置：先在宽域准备不再需要新堆块的 SELECT 1 与一条 INSERT。
    let roomy = d.next(ROOM);
    let (select_one, insert) = roomy
        .run(&main, |q| {
            Ok((
                raw_prepare(q, c"SELECT 1")?,
                raw_prepare(q, c"INSERT INTO t VALUES(3, x'00')")?,
            ))
        })
        .unwrap();
    // 证明它确实无需新分配：零额度域内 step 照常出行，零新增费用、零拒绝。
    let free_step = d.next(0);
    let live = d.current.live();
    let rc = free_step
        .run(&main, |_| {
            let rc = unsafe { ffi::sqlite3_step(stmt(select_one)) };
            unsafe { ffi::sqlite3_reset(stmt(select_one)) };
            Ok(rc)
        })
        .unwrap();
    assert_eq!(rc, ffi::SQLITE_ROW);
    assert_eq!(free_step.refusal(), None);
    assert_eq!(d.current.live(), live);

    // prepare：需要新堆块时在零额度域内被拒，首个拒绝是 Budget，账不残留。
    let prepare = d.next(0);
    let live = d.current.live();
    let first = refused(prepare.run(&main, |q| {
        let sql = std::ffi::CString::new(heap_query("SELECT body FROM t WHERE id = 2")).unwrap();
        let Err(SqlGate::Sqlite(code)) = raw_prepare(q, &sql) else {
            panic!("prepare refused with NOMEM");
        };
        assert_eq!(code & 0xff, ffi::SQLITE_NOMEM);
        assert_eq!(prepare.refusal_of(code), Some(ScratchFailure::Budget));
        Ok(())
    }));
    assert_eq!(first, ScratchFailure::Budget);
    assert_eq!(d.current.live(), live);

    // B1：首个拒绝之后零 SQL。同一闭包内，首拒前已准备的 SELECT 1（零新分配）、新的 SELECT 1
    // 与 INSERT 都在操作口被拒，SQLite 语句执行探针不增；返回原 typed 首拒。
    let latched = d.next(charged_bytes(4096).unwrap());
    let live = d.current.live();
    let mut codes = (0, 0);
    let big = vec![0x5au8; 1 << 20];
    let counted_calls = Cell::new(0);
    let counted = Counted {
        blob: &big,
        calls: &counted_calls,
    };
    let first = refused(latched.run(&main, |q| {
        let mut one = q.prepare("SELECT 1")?;
        let before = entered();
        assert_eq!(one.query_row([], |[v]| v.as_i64())?, Ok(1));
        assert_eq!(entered(), before + 1);
        assert!(unsafe { ffi::sqlite3_malloc(8192) }.is_null());
        let before = entered();
        let after = one.query_row([], |[v]| v.as_i64());
        assert!(
            matches!(after, Err(SqlGate::Refused(ScratchFailure::Budget))),
            "{after:?}"
        );
        let after = q.query_row("SELECT 1", [], |[v]| v.as_i64());
        assert!(
            matches!(after, Err(SqlGate::Refused(ScratchFailure::Budget))),
            "{after:?}"
        );
        let after = q.execute("INSERT INTO t VALUES(3, x'00')", []);
        assert!(
            matches!(after, Err(SqlGate::Refused(ScratchFailure::Budget))),
            "{after:?}"
        );
        // 首拒后带大借用 BLOB 参数的调用：参数不被求值（to_sql 零次），组件不建任何副本。
        let after = q.execute("INSERT INTO t VALUES(4, ?1)", [&counted]);
        assert!(
            matches!(after, Err(SqlGate::Refused(ScratchFailure::Budget))),
            "{after:?}"
        );
        assert_eq!(counted_calls.get(), 0);
        let after = q.prepare("SELECT 1").map(drop);
        assert!(
            matches!(after, Err(SqlGate::Refused(ScratchFailure::Budget))),
            "{after:?}"
        );
        assert_eq!(entered(), before);
        // unsafe 逃生口不在合同内；即便绕过操作口裸 step，进度回调仍不出行、不写。
        codes = unsafe {
            (
                ffi::sqlite3_step(stmt(select_one)),
                ffi::sqlite3_step(stmt(insert)),
            )
        };
        unsafe {
            ffi::sqlite3_reset(stmt(select_one));
            ffi::sqlite3_reset(stmt(insert));
        }
        Ok(())
    }));
    assert_eq!(first, ScratchFailure::Budget);
    assert_eq!(codes, (ffi::SQLITE_INTERRUPT, ffi::SQLITE_INTERRUPT));
    assert!(d.current.remaining() >= charged_bytes(16).unwrap());
    let mut calls = 0;
    let first = refused(latched.run(&ledger, |_| {
        calls += 1;
        Ok(())
    }));
    assert_eq!((first, calls), (ScratchFailure::Budget, 0));
    // 同父同 origin 重绑不清除首拒：额度虽够装下小块，仍不放行任何 SQL 或分配。
    let again = d.rebind();
    assert_eq!(again.refusal(), Some(ScratchFailure::Budget));
    let first = refused(again.run(&main, |_| {
        calls += 1;
        Ok(unsafe { ffi::sqlite3_malloc(16) })
    }));
    assert_eq!((first, calls), (ScratchFailure::Budget, 0));
    assert_eq!(d.current.live(), live);
    drop(again);
    drop(latched);

    // step：语句在宽域准备，换到零额度域后取跨页大行被拒。
    let roomy = d.next(ROOM);
    let big = roomy
        .run(&main, |q| {
            raw_prepare(q, c"SELECT body FROM t WHERE id = 2")
        })
        .unwrap();
    let step = d.next(0);
    let live = d.current.live();
    let mut rc = 0;
    let first = refused(step.run(&main, |_| {
        rc = unsafe { ffi::sqlite3_step(stmt(big)) };
        Ok(())
    }));
    assert_eq!(rc, ffi::SQLITE_NOMEM);
    assert_eq!(first, ScratchFailure::Budget);
    assert_eq!(d.current.live(), live);

    // 列取值：已在 step 时物化（并收费）的 BLOB，零额度域内 column_blob 不再新增费用；
    // 需要另建缓冲的取值（BLOB 按 TEXT 取需追加终止符）在零额度域内返回空指针并报 NOMEM。
    let roomy = d.next(ROOM);
    let pair = roomy
        .run(&main, |q| {
            let raw = raw_prepare(q, c"SELECT zeroblob(65536), randomblob(50000)")?;
            assert_eq!(unsafe { ffi::sqlite3_step(stmt(raw)) }, ffi::SQLITE_ROW);
            Ok(raw)
        })
        .unwrap();
    let column = d.next(0);
    let live = d.current.live();
    column
        .run(&main, |_| {
            assert!(!unsafe { ffi::sqlite3_column_blob(stmt(pair), 0) }.is_null());
            assert_eq!(unsafe { ffi::sqlite3_column_bytes(stmt(pair), 0) }, 65536);
            Ok(())
        })
        .unwrap();
    assert_eq!(column.refusal(), None);
    assert_eq!(d.current.live(), live);
    let first = refused(column.run(&main, |q| {
        assert!(unsafe { ffi::sqlite3_column_text(stmt(pair), 1) }.is_null());
        assert_eq!(
            unsafe { ffi::sqlite3_errcode(q.handle()) },
            ffi::SQLITE_NOMEM
        );
        Ok(())
    }));
    assert_eq!(first, ScratchFailure::Budget);
    // 失败路径会释放旧的动态值：旧 origin 返还只降 live，不回补零额度域，也没有新收费。
    assert!(d.current.live() <= live);
    assert_eq!(d.current.remaining(), 0);

    // 台账点查与新开连接同样在原父账内被拒。
    let point = d.next(0);
    let first = refused(point.run(&ledger, |q| {
        q.prepare(&heap_query("SELECT EXISTS(SELECT 1 FROM k WHERE key = ?1)"))
            .map(drop)
    }));
    assert_eq!(first, ScratchFailure::Budget);
    let open = d.next(0);
    assert_eq!(
        refused(open.open(&path("third.sqlite3"))),
        ScratchFailure::Budget
    );

    // 换域而不换绑：旧绑定过期，按未知归属拒绝（Ledger），不扣新域。
    // 父账已换域、绑定未换：run 不进入（调用计数不增）。
    let stale = d.next(ROOM);
    d.advance(ROOM);
    let live = d.current.live();
    let mut calls = 0;
    let before = entered();
    let first = refused(stale.run(&main, |q| {
        calls += 1;
        q.query_row("SELECT 1", [], |[v]| v.as_i64())
    }));
    assert_eq!(
        (first, calls, entered()),
        (ScratchFailure::Ledger, 0, before)
    );
    assert_eq!(d.current.live(), live);
    // 操作域固定冻结域：run 持有父账活动作用域，期间 handoff 被拒——包括参数 ToSql 回调里
    // 的 handoff；该查询仍在原 origin 上执行并成功。run 结束后作用域释放，换域恢复可用。
    let held = d.next(ROOM);
    let tried = Cell::new(None);
    let param = Handoff {
        budget: &d.current,
        request: d.request + 1,
        tried: &tried,
    };
    let before = entered();
    let value = held
        .run(&main, |q| {
            assert!(d.current.handoff(d.request + 1, ROOM).is_err());
            let mut select = q.prepare("SELECT ?1")?;
            select.query_row([&param], |[v]| v.as_i64())
        })
        .unwrap();
    assert_eq!(
        (value, tried.get(), entered()),
        (Ok(7), Some(false), before + 1)
    );
    assert_eq!(held.refusal(), None);
    d.advance(ROOM);
    let mut calls = 0;
    let first = refused(held.run(&main, |_| {
        calls += 1;
        Ok(())
    }));
    assert_eq!((first, calls), (ScratchFailure::Ledger, 0));

    // 大结果行以借值交给映射，不由组件复制；映射期间任何 SQL 操作在准入处拒绝（零执行）。
    // 宽域内带大借用 BLOB 参数的写入照常成功（to_sql 一次），组件不建副本。
    let rows = d.next(ROOM);
    let before = entered();
    let (len, nested) = rows
        .run(&main, |q| {
            q.query_row("SELECT body FROM t WHERE id = 2", [], |[body]| {
                let nested = q.query_row("SELECT 1", [], |[v]| v.as_i64());
                (
                    body.as_blob().map(<[u8]>::len),
                    matches!(nested, Err(SqlGate::Refused(ScratchFailure::Ledger))),
                )
            })
        })
        .unwrap();
    assert_eq!((len, nested, entered()), (Ok(200000), true, before + 1));
    let wrote = rows
        .run(&main, |q| {
            q.execute("INSERT INTO t VALUES(5, ?1)", [&counted])
        })
        .unwrap();
    assert_eq!((wrote, counted_calls.get()), (1, 1));
    assert_eq!(rows.refusal(), None);

    // 参数回调内的首拒：ToSql 在零额度域触发 prepare 收费拒绝并吞掉错误后返回值；
    // 外层语句在准入时即拒，不再进入 SQLite（执行探针不增），返回原 Budget 首错。
    let callback = d.next(0);
    let hit = Cell::new(None);
    let before = entered();
    let first = refused(callback.run(&main, |q| {
        let mut select = q.prepare("SELECT ?1")?;
        let param = Refuse { q, hit: &hit };
        select.query_row([&param], |[v]| v.as_i64())
    }));
    assert_eq!(
        (first, hit.get(), entered()),
        (ScratchFailure::Budget, Some(true), before)
    );

    // realloc：新块在旧块存活时完整预留；差 1 B 即拒且旧块不变，
    // 恰好够时成功；旧块按旧 origin 返还，不回补当前域额度。
    let roomy = d.next(ROOM);
    let old = roomy
        .run(&main, |_| {
            let old = unsafe { ffi::sqlite3_malloc(80) };
            assert!(!old.is_null());
            unsafe { ptr::write_bytes(old.cast::<u8>(), 0x5a, 80) };
            Ok(old as usize)
        })
        .unwrap() as *mut std::ffi::c_void;
    let need = charged_bytes(160).unwrap();
    let short = d.next(need - 1);
    let live = d.current.live();
    let first = refused(short.run(&main, |_| {
        assert!(unsafe { ffi::sqlite3_realloc(old, 160) }.is_null());
        Ok(())
    }));
    assert_eq!(first, ScratchFailure::Budget);
    assert_eq!(unsafe { ffi::sqlite3_msize(old) }, 80);
    assert_eq!(
        unsafe { std::slice::from_raw_parts(old.cast::<u8>(), 80) },
        &[0x5a; 80]
    );
    assert_eq!(d.current.live(), live);
    let exact = d.next(need);
    let grown = exact
        .run(&main, |_| {
            Ok(unsafe { ffi::sqlite3_realloc(old, 160) } as usize)
        })
        .unwrap() as *mut std::ffi::c_void;
    assert!(!grown.is_null());
    assert_eq!(
        unsafe { std::slice::from_raw_parts(grown.cast::<u8>(), 80) },
        &[0x5a; 80]
    );
    assert_eq!(d.current.remaining(), 0);
    assert_eq!(d.current.live(), live - charged_bytes(80).unwrap() + need);
    unsafe { ffi::sqlite3_free(grown) };
    assert_eq!(d.current.remaining(), need);

    // B2 错父：A（另一父账）的连接与块仍存活时绑定主父账 B。
    let other_root = Budget::fresh(LIMIT).unwrap();
    let other_base = other_root.live();
    let other = other_root.bind_frozen(1, LIMIT, 0, 0).unwrap();
    let (other_parent, other_origin) = other.sqlite_parent();
    let foreign_binding = allocator.bind(other_parent.clone(), other_origin).unwrap();
    let foreign = foreign_binding.open(c":memory:").unwrap();
    let foreign_block = foreign_binding
        .run(&foreign, |q| {
            q.execute_batch("CREATE TABLE f(x BLOB); INSERT INTO f VALUES(zeroblob(50000));")?;
            Ok(unsafe { ffi::sqlite3_malloc(64) } as usize)
        })
        .unwrap() as *mut std::ffi::c_void;
    assert!(!foreign_block.is_null());
    assert!(foreign_binding.live_bytes() > 0);
    assert_eq!(other.live() - other_base, foreign_binding.live_bytes());
    let home = d.next(ROOM);
    let main_live = d.current.live();
    let foreign_live = other.live();
    // A 连接在 B 域：错父 prepare 在进入 SQLite 前拒绝，两边零增量。
    let mut calls = 0;
    let wrong = home.run(&foreign, |q| {
        calls += 1;
        q.prepare(&heap_query("SELECT x FROM f WHERE 1")).map(drop)
    });
    assert!(matches!(wrong, Err(SqlGate::WrongParent)), "{wrong:?}");
    assert_eq!(calls, 0);
    // 已失效的 A 绑定不能再执行。
    assert_eq!(
        refused(foreign_binding.run(&foreign, |_| {
            calls += 1;
            Ok(())
        })),
        ScratchFailure::Ledger
    );
    assert_eq!(calls, 0);
    // 跨父 realloc：A 的块在 B 域内 realloc，收费前拒绝，块与两边账都不变。
    let first = refused(home.run(&main, |_| {
        assert!(unsafe { ffi::sqlite3_realloc(foreign_block, 4096) }.is_null());
        Ok(())
    }));
    assert_eq!(first, ScratchFailure::Ledger);
    assert_eq!(unsafe { ffi::sqlite3_msize(foreign_block) }, 64);
    assert_eq!(d.current.live(), main_live);
    assert_eq!(other.live(), foreign_live);

    // 操作域排他：run 中换绑、再入与他线程的 SQLite 分配都被拒，零收费。
    let home = d.next(ROOM);
    let main_live = d.current.live();
    let (parent, origin) = d.current.sqlite_parent();
    home.run(&main, |_| {
        assert!(allocator.bind(parent, origin).is_err());
        assert_eq!(
            refused(home.run(&ledger, |_| Ok(()))),
            ScratchFailure::Ledger
        );
        let raced = std::thread::scope(|s| {
            s.spawn(|| unsafe { ffi::sqlite3_malloc(16) } as usize)
                .join()
                .unwrap()
        });
        assert_eq!(raced, 0);
        Ok(())
    })
    .unwrap();
    assert_eq!(home.refusal(), None);
    assert_eq!(d.current.live(), main_live);
    // 错父关闭被拒并交回连接；回到 A 的域内释放与关闭，字节只回到 A。
    let (foreign, gate) = home
        .close(foreign)
        .err()
        .expect("wrong-parent close refused");
    assert!(matches!(gate, SqlGate::WrongParent), "{gate:?}");
    drop(home);
    let foreign_home = allocator.bind(other_parent, other_origin).unwrap();
    assert_eq!(foreign_home.refusal(), None);
    foreign_home
        .run(&foreign, |_| {
            unsafe { ffi::sqlite3_free(foreign_block) };
            Ok(())
        })
        .unwrap();
    foreign_home.close(foreign).map_err(|(_, e)| e).unwrap();
    assert_eq!(other.live() - other_base, foreign_home.live_bytes());
    assert_eq!(d.current.live(), main_live);
    drop(foreign_home);

    // 零存活块的首拒：父 Z 以零额度绑定，打开连接即拒（Budget），Z 名下零块；解除绑定后槽交还
    // 父句柄，但同父同 origin 重绑仍取回原首拒，开库零执行；Z 换到新冻结域后才开新记录。
    // 首次绑定须取根的完整额度，再换到零额度冻结域（与 `Domains::next(0)` 同法）。
    let zero_root = Budget::fresh(LIMIT).unwrap();
    let mut zero = zero_root
        .bind_frozen(1, LIMIT, 0, 0)
        .unwrap()
        .handoff(2, 0)
        .unwrap();
    let (zero_parent, zero_origin) = zero.sqlite_parent();
    let first_bind = allocator.bind(zero_parent.clone(), zero_origin).unwrap();
    assert_eq!(
        refused(first_bind.open(c":memory:")),
        ScratchFailure::Budget
    );
    assert_eq!(first_bind.live_blocks(), 0);
    drop(first_bind);
    let rebound = allocator.bind(zero_parent.clone(), zero_origin).unwrap();
    assert_eq!(rebound.refusal(), Some(ScratchFailure::Budget));
    assert_eq!(refused(rebound.open(c":memory:")), ScratchFailure::Budget);
    drop(rebound);
    zero = zero.handoff(3, ROOM).unwrap();
    let (zero_parent, zero_origin) = zero.sqlite_parent();
    let fresh = allocator.bind(zero_parent, zero_origin).unwrap();
    assert_eq!(fresh.refusal(), None);
    let zero_conn = fresh.open(c":memory:").unwrap();
    fresh.close(zero_conn).map_err(|(_, e)| e).unwrap();
    assert_eq!(fresh.live_blocks(), 0);
    drop(fresh);

    // 关闭全部连接并 shutdown 后，主父账回到出生读数，没有残留或破损。
    let home = d.next(ROOM);
    home.run(&main, |_| {
        for raw in [select_one, insert, big, pair] {
            unsafe { ffi::sqlite3_finalize(stmt(raw)) };
        }
        Ok(())
    })
    .unwrap();
    // B1 的 INSERT 被中止，表里没有 id=3。
    let inserted = home
        .run(&main, |q| {
            q.query_row("SELECT count(*) FROM t WHERE id = 3", [], |[v]| v.as_i64())
        })
        .unwrap();
    assert_eq!(inserted, Ok(0));
    home.close(main).map_err(|(_, e)| e).unwrap();
    home.close(ledger).map_err(|(_, e)| e).unwrap();
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

/// 调用方进度回调被调用的次数；`STOP` 置位时要求中止。
static USER_PROGRESS: AtomicUsize = AtomicUsize::new(0);
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe extern "C" fn user_progress(_: *mut c_void) -> c_int {
    USER_PROGRESS.fetch_add(1, Ordering::SeqCst);
    c_int::from(STOP.load(Ordering::SeqCst))
}

/// 原私有快照式授权门：拒绝读取表 `secret`。
unsafe extern "C" fn deny_secret(
    _: *mut c_void,
    action: c_int,
    table: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> c_int {
    if action == ffi::SQLITE_READ
        && !table.is_null()
        && unsafe { CStr::from_ptr(table) }.to_bytes() == b"secret"
    {
        ffi::SQLITE_DENY
    } else {
        ffi::SQLITE_OK
    }
}

fn user_calls() -> usize {
    USER_PROGRESS.load(Ordering::SeqCst)
}

fn sqlite_code(result: Result<impl std::fmt::Debug, SqlGate>) -> c_int {
    match result {
        Err(SqlGate::Sqlite(code)) => code,
        other => panic!("expected an SQLite result code, got {other:?}"),
    }
}

#[test]
#[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
fn sqlite_parent_connection_boundary_keeps_original_open_semantics() {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{Callback, DbFlag, OpenSpec};
    // SAFETY: this test runs alone in a fresh process before any SQLite use.
    let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
    let root = Budget::fresh(LIMIT).unwrap();
    let base = root.live();
    let mut d = Domains {
        allocator: &allocator,
        current: root.bind_frozen(1, LIMIT, 0, 0).unwrap(),
        request: 1,
    };
    let dir = std::env::temp_dir().join(format!("sqlite-boundary-{}", std::process::id()));
    let path = |name: &str| std::ffi::CString::new(dir.join(name).to_str().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wide = d.rebind();
    let seed = wide.open(&path("snap.sqlite3")).unwrap();
    wide.run(&seed, |q| {
        q.execute_batch(
            "CREATE TABLE s(x INTEGER); INSERT INTO s VALUES(1);
             CREATE TABLE secret(y INTEGER); INSERT INTO secret VALUES(2);",
        )
    })
    .unwrap();
    wide.close(seed).map_err(|(_, e)| e).unwrap();

    let mut vfs: ffi::sqlite3_vfs = unsafe { *ffi::sqlite3_vfs_find(ptr::null()) };
    vfs.zName = c"bsql-private".as_ptr();
    let vfs_ptr: *mut ffi::sqlite3_vfs = &mut vfs;
    unsafe { wide.register_vfs(vfs_ptr) }.unwrap();
    assert_eq!(
        unsafe { ffi::sqlite3_vfs_find(c"bsql-private".as_ptr()) },
        vfs_ptr
    );

    let configs = [
        (DbFlag::NoCheckpointOnClose, true),
        (DbFlag::Defensive, true),
        (DbFlag::EnableLoadExtension, false),
    ];
    let limits = [(ffi::SQLITE_LIMIT_WORKER_THREADS, 0)];
    let spec = OpenSpec {
        flags: ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_PRIVATECACHE | ffi::SQLITE_OPEN_NOMUTEX,
        vfs: Some(c"bsql-private"),
        db_config: &configs,
        limits: &limits,
        busy_timeout_ms: Some(0),
        authorizer: Some(Callback {
            callback: deny_secret,
            context: ptr::null_mut(),
        }),
        progress: Some(Callback {
            callback: user_progress,
            context: ptr::null_mut(),
        }),
    };
    let live = d.current.live();
    let missing = OpenSpec {
        vfs: Some(c"bsql-missing"),
        ..OpenSpec::default()
    };
    let code = sqlite_code(unsafe { wide.open_with(&path("snap.sqlite3"), &missing) });
    assert_eq!(code & 0xff, ffi::SQLITE_ERROR);
    assert_eq!(d.current.live(), live);

    let snap = unsafe { wide.open_with(&path("snap.sqlite3"), &spec) }.unwrap();
    let before = user_calls();
    wide.run(&snap, |q| {
        let mut used: *mut ffi::sqlite3_vfs = ptr::null_mut();
        let rc = unsafe {
            ffi::sqlite3_file_control(
                q.handle(),
                c"main".as_ptr(),
                ffi::SQLITE_FCNTL_VFS_POINTER,
                (&mut used as *mut *mut ffi::sqlite3_vfs).cast(),
            )
        };
        assert_eq!(rc, ffi::SQLITE_OK);
        assert_eq!(used, vfs_ptr);
        for (flag, want) in configs {
            let mut out: c_int = -1;
            let rc = unsafe {
                ffi::sqlite3_db_config(q.handle(), flag.op(), -1, &mut out as *mut c_int)
            };
            assert_eq!((rc, out), (ffi::SQLITE_OK, c_int::from(want)));
        }
        assert_eq!(
            unsafe { ffi::sqlite3_limit(q.handle(), ffi::SQLITE_LIMIT_WORKER_THREADS, -1) },
            0
        );
        assert_eq!(
            q.query_row("SELECT x FROM s", [], |[v]| v.as_i64()),
            Ok(Ok(1))
        );
        // 只读打开：写被 SQLite 以 READONLY 拒绝。
        let code = sqlite_code(q.execute("INSERT INTO s VALUES(9)", []));
        assert_eq!(code & 0xff, ffi::SQLITE_READONLY);
        // 原授权门仍在：读 secret 被拒（SQLITE_AUTH）。
        let code = sqlite_code(q.query_row("SELECT y FROM secret", [], |[v]| v.as_i64()));
        assert_eq!(code & 0xff, ffi::SQLITE_AUTH);
        Ok(())
    })
    .unwrap();
    // 调用方进度回调串在组件门之后，真实被调用；它要求中止时查询被中止。
    assert!(user_calls() > before);
    STOP.store(true, Ordering::SeqCst);
    let code = wide
        .run(&snap, |q| {
            Ok(sqlite_code(q.query_row(
                "SELECT count(*) FROM s",
                [],
                |[v]| v.as_i64(),
            )))
        })
        .unwrap();
    assert_eq!(code & 0xff, ffi::SQLITE_INTERRUPT);
    STOP.store(false, Ordering::SeqCst);
    // 运行中可更换授权门（原 3A 连接封印式）；撤销后 secret 可读。
    wide.run(&snap, |q| {
        unsafe { q.set_authorizer(None) }?;
        assert_eq!(
            q.query_row("SELECT y FROM secret", [], |[v]| v.as_i64()),
            Ok(Ok(2))
        );
        Ok(())
    })
    .unwrap();
    // 只读连接与 VFS 都记在本父账名下。
    assert_eq!(d.current.live() - base, wide.live_bytes());
    drop(wide);

    // 首拒后的收尾：事务中触发收费拒绝，之后调用方 SQL 一律不进 SQLite，调用方进度回调也不再
    // 被调用；回滚与关闭作为收尾仍完成，首拒保持，且回滚期间调用方要求中止也不中断回滚。
    let home = d.next(ROOM);
    let writer = unsafe {
        home.open_with(
            &path("w.sqlite3"),
            &OpenSpec {
                progress: Some(Callback {
                    callback: user_progress,
                    context: ptr::null_mut(),
                }),
                ..OpenSpec::default()
            },
        )
    }
    .unwrap();
    home.run(&writer, |q| {
        q.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE w(v INTEGER);")
    })
    .unwrap();
    let home = d.next(ROOM);
    let first = refused(home.run(&writer, |q| {
        q.execute_batch("BEGIN; INSERT INTO w VALUES(1);")?;
        assert!(unsafe { ffi::sqlite3_malloc64(ROOM as u64 + 1) }.is_null());
        Ok(())
    }));
    assert_eq!(first, ScratchFailure::Budget);
    let mut calls = 0;
    let quiet = user_calls();
    assert_eq!(
        refused(home.run(&writer, |q| {
            calls += 1;
            q.execute("INSERT INTO w VALUES(2)", [])
        })),
        ScratchFailure::Budget
    );
    assert_eq!(calls, 0);
    assert_eq!(user_calls(), quiet);
    STOP.store(true, Ordering::SeqCst);
    let in_tx = home.run(&writer, |_| Ok(()));
    assert_eq!(refused(in_tx), ScratchFailure::Budget);
    home.rollback(&writer).unwrap();
    STOP.store(false, Ordering::SeqCst);
    assert_eq!(user_calls(), quiet);
    assert_eq!(home.refusal(), Some(ScratchFailure::Budget));
    home.close(writer).map_err(|(_, e)| e).unwrap();
    assert_eq!(home.refusal(), Some(ScratchFailure::Budget));
    drop(home);

    // 新冻结域：回滚确已生效（没有 v=1），收尾后的连接与 VFS 全部归还。
    let home = d.next(ROOM);
    let reader = home.open(&path("w.sqlite3")).unwrap();
    assert_eq!(
        home.run(&reader, |q| q.query_row(
            "SELECT count(*) FROM w",
            [],
            |[v]| v.as_i64()
        )),
        Ok(Ok(0))
    );
    home.close(reader).map_err(|(_, e)| e).unwrap();
    home.close(snap).map_err(|(_, e)| e).unwrap();
    unsafe { home.unregister_vfs(vfs_ptr) }.unwrap();
    assert!(unsafe { ffi::sqlite3_vfs_find(c"bsql-private".as_ptr()) }.is_null());
    assert_eq!(unsafe { ffi::sqlite3_shutdown() }, ffi::SQLITE_OK);
    assert_eq!(home.live_bytes(), 0);
    assert_eq!(d.current.live(), base);
    assert!(!allocator.is_broken());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "release lock: run alone with --ignored --exact"]
fn sqlite_parent_db_config_accepts_only_boolean_int_pointer_ops() {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::DbFlag;
    for flag in DbFlag::ALL {
        let op = match flag {
            DbFlag::NoCheckpointOnClose | DbFlag::Defensive | DbFlag::EnableLoadExtension => {
                flag.op()
            }
        };
        assert!(
            op != ffi::SQLITE_DBCONFIG_MAINDBNAME && op != ffi::SQLITE_DBCONFIG_LOOKASIDE,
            "{flag:?} maps to a non-(int,int*) opcode {op}"
        );
    }
    let mut ops = DbFlag::ALL.map(DbFlag::op);
    ops.sort_unstable();
    assert!(
        ops.windows(2).all(|w| w[0] != w[1]),
        "duplicate opcode in {ops:?}"
    );

    let conn = Connection::open_in_memory().unwrap();
    // SAFETY: the handle belongs to `conn`, which outlives every call below.
    let db = unsafe { conn.handle() };
    for flag in DbFlag::ALL {
        for value in [1, 0, 1] {
            let mut out: c_int = -1;
            let rc =
                unsafe { ffi::sqlite3_db_config(db, flag.op(), value, &mut out as *mut c_int) };
            assert_eq!((rc, out), (ffi::SQLITE_OK, value), "{flag:?} set {value}");
            let mut read: c_int = -1;
            let rc = unsafe { ffi::sqlite3_db_config(db, flag.op(), -1, &mut read as *mut c_int) };
            assert_eq!(
                (rc, read),
                (ffi::SQLITE_OK, value),
                "{flag:?} read back {value}"
            );
        }
    }
}

/// #1419 3C：3A 受管 reader 在收费组件事务上运行原判据（签发门、域核、同事务点查、读前许可、
/// 首错粘滞）。SQLite pager/语句分配向同一父账收费；收费首拒按原工作域类型返回，之后零 SELECT。
#[test]
#[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
fn charged_step_reader_reads_on_same_parent_and_types_refusal() {
    use newchan_rust::s_content as sc;
    use newchan_rust::s_content::capture_allocation::scratch::WorkError;
    use newchan_rust::s_content::m8d_owner_bridge::{ChargedReadTx, GatedSqlReader, LiveGate};
    use newchan_rust::s_content::CodecError;

    struct CountingGate(Cell<usize>);
    impl<'b> LiveGate<ChargedReadTx<'b>> for CountingGate {
        fn check_live(&self, _: &ChargedReadTx<'b>) -> Result<(), CodecError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
        fn sample(&self) -> Result<(), CodecError> {
            Ok(())
        }
    }

    // SAFETY: this test runs alone in a fresh process before any SQLite use.
    let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
    let root = Budget::fresh(LIMIT).unwrap();
    let mut d = Domains {
        allocator: &allocator,
        current: root.bind_frozen(1, LIMIT, 0, 0).unwrap(),
        request: 1,
    };
    let dir = std::env::temp_dir().join(format!("charged-reader-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = std::ffi::CString::new(dir.join("nodes.sqlite3").to_str().unwrap()).unwrap();

    let mut nodes = sc::MemoryNodes::new();
    let reference = sc::write_value(&mut nodes, &sc::Value::String("charged reader".into()))
        .unwrap()
        .content_ref();
    let digest = reference.digest().unwrap();
    let (tag, body) = nodes.get(&digest).unwrap();
    let big_digest = [7u8; 32];

    let wide = d.rebind();
    let db = wide.open(&path).unwrap();
    wide.run(&db, |q| {
        q.execute_batch(
            "CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL, body BLOB NOT NULL)",
        )?;
        let tag = i64::from(tag);
        q.execute(
            "INSERT INTO s_content_nodes VALUES(?1, ?2, ?3)",
            [&&digest[..] as &dyn ToSql, &tag, &body],
        )?;
        // 32 MiB 正文以 zeroblob 写入：SQLite 直接写零页，不在内存中组装整行，准备阶段没有
        // 32 MiB 级分配；读回时才整块装入，由窄域的读来触发收费首拒。
        q.execute(
            "INSERT INTO s_content_nodes VALUES(?1, 0, zeroblob(33554432))",
            [&&big_digest[..] as &dyn ToSql],
        )?;
        q.execute_batch("BEGIN")
    })
    .unwrap();

    let gate = CountingGate(Cell::new(0));
    let mut tx = ChargedReadTx::new(&wide, &db);
    let outcome = d
        .current
        .codec_work()
        .ok()
        .expect("codec work")
        .run(|work| {
            let mut reader = GatedSqlReader::new(&mut tx, work, &gate)?;
            let found = reader.frame(&digest, |t, b| (t, b.to_vec()))?;
            let missing = reader.frame(&[9u8; 32], |t, _| t)?;
            reader.exit()?;
            Ok::<_, CodecError>((
                found,
                missing,
                reader.node_selects(),
                reader.metadata_reads(),
            ))
        });
    let Ok((found, missing, selects, metadata)) = outcome else {
        panic!("charged reader must read in the wide domain");
    };
    assert_eq!(found, Some((tag, body.to_vec())));
    assert_eq!(missing, None);
    assert_eq!(selects, 2);
    assert_eq!(
        metadata, 12,
        "three PRAGMAs at issue, before each SELECT and at exit"
    );
    assert_eq!(
        gate.0.get(),
        4,
        "issuer gate at issue, before each SELECT and at exit"
    );
    assert!(
        wide.live_bytes() > 0,
        "connection and pager bytes live on the parent"
    );
    wide.rollback(&db).unwrap();
    wide.close(db).map_err(|(_, e)| e).unwrap();

    let tight = d.next(8 << 20);
    let base = d.current.live();
    let db = tight.open(&path).unwrap();
    tight.run(&db, |q| q.execute_batch("BEGIN")).unwrap();
    let gate = CountingGate(Cell::new(0));
    let mut tx = ChargedReadTx::new(&tight, &db);
    let observed = std::cell::RefCell::new(None);
    let outcome = d
        .current
        .codec_work()
        .ok()
        .expect("codec work")
        .run(|work| {
            let mut reader = GatedSqlReader::new(&mut tx, work, &gate)?;
            let first = reader.frame(&big_digest, |_, b| b.len());
            let again = reader.frame(&digest, |t, _| t);
            *observed.borrow_mut() = Some((
                first.err().and_then(|e| e.scratch_failure()),
                again.err().and_then(|e| e.scratch_failure()),
                reader.node_selects(),
            ));
            Ok::<_, CodecError>(())
        });
    assert!(
        matches!(outcome, Err(WorkError::Resource(ScratchFailure::Budget))),
        "SQLite refusal is the work domain's typed Budget refusal"
    );
    assert_eq!(
        observed.take(),
        Some((
            Some(ScratchFailure::Budget),
            Some(ScratchFailure::Budget),
            1
        )),
        "first refusal sticks and no further SELECT runs"
    );
    assert_eq!(tight.refusal(), Some(ScratchFailure::Budget));
    tight.rollback(&db).unwrap();
    assert!(
        db.is_autocommit(),
        "teardown rolled the read transaction back"
    );
    tight.close(db).map_err(|(_, e)| e).unwrap();
    assert_eq!(
        d.current.live(),
        base,
        "refused read leaves no charged bytes"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

static HOOK_UPDATES: AtomicUsize = AtomicUsize::new(0);
static HOOK_LAST_ROWID: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static HOOK_COMMITS: AtomicUsize = AtomicUsize::new(0);
static HOOK_FRAMES: AtomicUsize = AtomicUsize::new(0);
static HOOK_CHECKPOINT: std::sync::Mutex<Option<Result<(c_int, c_int), SqlGate>>> =
    std::sync::Mutex::new(None);

unsafe extern "C" fn record_update(
    _: *mut c_void,
    _: c_int,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    rowid: i64,
) {
    HOOK_UPDATES.fetch_add(1, Ordering::SeqCst);
    HOOK_LAST_ROWID.store(rowid, Ordering::SeqCst);
}

unsafe extern "C" fn record_wal_commit(
    _: *mut c_void,
    db: *mut ffi::sqlite3,
    _: *const std::ffi::c_char,
    frames: c_int,
) -> c_int {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::wal_checkpoint;
    HOOK_COMMITS.fetch_add(1, Ordering::SeqCst);
    // SAFETY: db is the handle SQLite passed to this WAL hook.
    let noop = unsafe { wal_checkpoint(db, ffi::SQLITE_CHECKPOINT_NOOP) };
    HOOK_FRAMES.store(usize::try_from(frames).unwrap_or(0), Ordering::SeqCst);
    if let Ok(mut slot) = HOOK_CHECKPOINT.lock() {
        *slot = Some(noop);
    }
    ffi::SQLITE_OK
}

/// #1419 3C：owner 连接家族所需的组件能力（行变更与 WAL 提交回调、回调内 checkpoint、
/// cache_flush、按实际列数借行、收尾撤销回调）在真实父账上的合同。
#[test]
#[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
fn sqlite_parent_owner_hooks_flush_and_cells_on_same_parent() {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{
        wal_checkpoint, Callback, MAX_ROW_COLUMNS,
    };
    use std::ops::ControlFlow;
    // SAFETY: this test runs alone in a fresh process before any SQLite use.
    let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
    let root = Budget::fresh(LIMIT).unwrap();
    let d = Domains {
        allocator: &allocator,
        current: root.bind_frozen(1, LIMIT, 0, 0).unwrap(),
        request: 1,
    };
    let dir = std::env::temp_dir().join(format!("owner-hooks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("owner.sqlite3");
    let mut wal_path = db_path.as_os_str().to_os_string();
    wal_path.push("-wal");
    let path = std::ffi::CString::new(db_path.to_str().unwrap()).unwrap();

    let base = d.current.live();
    let b = d.rebind();
    let conn = b.open(&path).unwrap();
    b.run(&conn, |q| {
        q.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE t(x INTEGER);")
    })
    .unwrap();
    let update = Callback {
        callback: record_update
            as newchan_rust::s_content::capture_allocation::sqlite_parent::UpdateFn,
        context: ptr::null_mut(),
    };
    let wal = Callback {
        callback: record_wal_commit
            as newchan_rust::s_content::capture_allocation::sqlite_parent::WalFn,
        context: ptr::null_mut(),
    };
    // SAFETY: both hooks use static state only.
    b.run(&conn, |q| unsafe {
        q.set_update_hook(Some(update))?;
        q.set_wal_hook(Some(wal))
    })
    .unwrap();
    let commits = HOOK_COMMITS.load(Ordering::SeqCst);
    let wal_len = || std::fs::metadata(&wal_path).map_or(0, |m| m.len());
    let before_flush = wal_len();
    b.run(&conn, |q| {
        q.execute_batch("BEGIN IMMEDIATE; INSERT INTO t VALUES(10); INSERT INTO t VALUES(20);")
    })
    .unwrap();
    assert_eq!(
        wal_len(),
        before_flush,
        "uncommitted pages stay in the page cache"
    );
    b.run(&conn, |q| q.cache_flush()).unwrap();
    assert!(
        wal_len() > before_flush,
        "cache_flush wrote the open transaction's dirty pages to the WAL"
    );
    assert_eq!(HOOK_UPDATES.load(Ordering::SeqCst), 2);
    assert_eq!(HOOK_LAST_ROWID.load(Ordering::SeqCst), 2);
    b.run(&conn, |q| q.execute_batch("COMMIT")).unwrap();
    assert_eq!(
        HOOK_COMMITS.load(Ordering::SeqCst),
        commits + 1,
        "one WAL hook per COMMIT"
    );
    assert!(
        HOOK_FRAMES.load(Ordering::SeqCst) > 0,
        "WAL hook saw the committed frames"
    );
    let checkpoint = HOOK_CHECKPOINT.lock().unwrap().take();
    assert!(
        matches!(checkpoint, Some(Ok((frames, _))) if frames > 0),
        "checkpoint inside the WAL hook runs in the operation domain: {checkpoint:?}"
    );
    assert_eq!(
        unsafe { wal_checkpoint(ptr::null_mut(), ffi::SQLITE_CHECKPOINT_NOOP) },
        Err(SqlGate::Refused(ScratchFailure::Ledger)),
        "checkpoint outside any operation domain is refused before touching SQLite"
    );

    let mut rows = Vec::new();
    b.run(&conn, |q| {
        q.prepare("SELECT rowid, * FROM t ORDER BY rowid")?
            .for_each_cells([], |cells| {
                rows.push((cells.len(), cells[0].as_i64().ok(), cells[1].as_i64().ok()));
                ControlFlow::Continue(())
            })
    })
    .unwrap();
    assert_eq!(rows, [(2, Some(1), Some(10)), (2, Some(2), Some(20))]);
    let wide: String = (0..=MAX_ROW_COLUMNS).map(|i| format!("{i},")).collect();
    let wide = format!("SELECT {}", wide.trim_end_matches(','));
    assert_eq!(
        b.run(&conn, |q| q
            .prepare(&wide)?
            .for_each_cells([], |_| ControlFlow::Continue(()))),
        Err(SqlGate::Sqlite(ffi::SQLITE_RANGE)),
        "rows wider than the stack buffer are refused without allocating"
    );

    b.clear_write_hooks(&conn).unwrap();
    let updates = HOOK_UPDATES.load(Ordering::SeqCst);
    let commits = HOOK_COMMITS.load(Ordering::SeqCst);
    b.run(&conn, |q| q.execute_batch("INSERT INTO t VALUES(30);"))
        .unwrap();
    assert_eq!(
        (
            HOOK_UPDATES.load(Ordering::SeqCst),
            HOOK_COMMITS.load(Ordering::SeqCst)
        ),
        (updates, commits),
        "teardown cleared both hooks"
    );

    // SAFETY: both hooks use static state only.
    b.run(&conn, |q| unsafe {
        q.set_update_hook(Some(update))?;
        q.set_wal_hook(Some(wal))
    })
    .unwrap();
    let refused = b.run(&conn, |q| {
        q.query_row("SELECT length(randomblob(134217728))", [], |[v]| v.as_i64())
    });
    assert_eq!(
        refused.map(drop),
        Err(SqlGate::Refused(ScratchFailure::Budget))
    );
    assert_eq!(
        b.run(&conn, |q| unsafe { q.set_update_hook(None) }),
        Err(SqlGate::Refused(ScratchFailure::Budget)),
        "ordinary hook changes stay behind the first refusal"
    );
    b.clear_write_hooks(&conn).unwrap();
    assert_eq!(
        b.refusal(),
        Some(ScratchFailure::Budget),
        "first refusal kept"
    );
    b.close(conn).map_err(|(_, e)| e).unwrap();
    drop(b);
    assert_eq!(d.current.live(), base, "all connection bytes returned");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 新 WAL 回调的执行计划：1 只做 PASSIVE；2 先用 `sqlite3_malloc64` 触发收费首拒再 PASSIVE；
/// 3 先把本域余额借到不足一个最小块再 PASSIVE（checkpoint 自身的分配被拒）。
static WAL_PLAN: AtomicUsize = AtomicUsize::new(0);
static WAL_MALLOC_REFUSED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static WAL_PAD_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static WAL_RESULT: std::sync::Mutex<Option<Result<(c_int, c_int), SqlGate>>> =
    std::sync::Mutex::new(None);
thread_local! {
    static WAL_BUDGET: std::cell::RefCell<Option<Budget>> = const { std::cell::RefCell::new(None) };
}

unsafe extern "C" fn planned_wal_commit(
    _: *mut c_void,
    db: *mut ffi::sqlite3,
    _: *const std::ffi::c_char,
    _: c_int,
) -> c_int {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::wal_checkpoint;
    let mut pad = ptr::null_mut();
    match WAL_PLAN.load(Ordering::SeqCst) {
        2 => {
            // SAFETY: plain SQLite allocation inside the running operation domain.
            let p = unsafe { ffi::sqlite3_malloc64(128 << 20) };
            WAL_MALLOC_REFUSED.store(p.is_null(), Ordering::SeqCst);
            // SAFETY: p is null or a block from sqlite3_malloc64.
            unsafe { ffi::sqlite3_free(p) };
        }
        3 => {
            let remaining = WAL_BUDGET
                .with(|b| b.borrow().as_ref().map(Budget::remaining))
                .unwrap_or(0);
            // 16 的倍数，SQLite 的取整不再放大；剩余不足一个最小块。
            let payload = remaining.saturating_sub(64) / 16 * 16;
            let left = charged_bytes(payload).and_then(|c| remaining.checked_sub(c));
            // SAFETY: plain SQLite allocation inside the running operation domain.
            pad = unsafe { ffi::sqlite3_malloc64(payload as u64) };
            WAL_PAD_HELD.store(
                !pad.is_null()
                    && left.is_some_and(|left| charged_bytes(1).is_some_and(|c| c > left)),
                Ordering::SeqCst,
            );
        }
        _ => {}
    }
    // SAFETY: db is the handle SQLite passed to this WAL hook.
    let result = unsafe { wal_checkpoint(db, ffi::SQLITE_CHECKPOINT_PASSIVE) };
    // SAFETY: pad is null or a block from sqlite3_malloc64.
    unsafe { ffi::sqlite3_free(pad) };
    if let Ok(mut slot) = WAL_RESULT.lock() {
        *slot = Some(result);
    }
    ffi::SQLITE_OK
}

/// #1419 3C 4da 独审 F1：WAL 回调内 checkpoint 与普通操作同准入、同首拒优先结算。已有首拒时返回
/// 同一带类型首拒且不进入底层 checkpoint（主库文件不被回填）；checkpoint 自身的收费拒绝返回带类型
/// 首拒。正控：同一回调在可用额度内 PASSIVE 回填全部帧。
#[test]
#[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
fn sqlite_parent_wal_checkpoint_admits_and_settles_first_refusal() {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{Callback, WalFn};
    // SAFETY: this test runs alone in a fresh process before any SQLite use.
    let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
    let root = Budget::fresh(LIMIT).unwrap();
    let mut d = Domains {
        allocator: &allocator,
        current: root.bind_frozen(1, LIMIT, 0, 0).unwrap(),
        request: 1,
    };
    let dir = std::env::temp_dir().join(format!("wal-checkpoint-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("wal.sqlite3");
    let path = std::ffi::CString::new(db_path.to_str().unwrap()).unwrap();
    let main_len = || std::fs::metadata(&db_path).map_or(0, |m| m.len());
    let hook = Callback {
        callback: planned_wal_commit as WalFn,
        context: ptr::null_mut(),
    };
    let take = || WAL_RESULT.lock().unwrap().take();
    let commit_blob = |b: &SqliteBinding, conn: &_| {
        b.run(conn, |q| {
            q.execute_batch("BEGIN IMMEDIATE; INSERT INTO t VALUES(randomblob(20000)); COMMIT;")
        })
    };

    let b = d.next(16 << 20);
    let conn = b.open(&path).unwrap();
    b.run(&conn, |q| {
        q.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE t(x BLOB);")?;
        // SAFETY: the hook uses static state only.
        unsafe { q.set_wal_hook(Some(hook)) }
    })
    .unwrap();
    WAL_PLAN.store(1, Ordering::SeqCst);
    let before = main_len();
    commit_blob(&b, &conn).unwrap();
    assert!(
        matches!(take(), Some(Ok((log, done))) if log > 0 && done == log),
        "PASSIVE inside the hook backfills every frame"
    );
    assert!(main_len() > before, "the backfill reached the main file");

    WAL_PLAN.store(2, Ordering::SeqCst);
    let before = main_len();
    assert_eq!(
        commit_blob(&b, &conn),
        Err(SqlGate::Refused(ScratchFailure::Budget))
    );
    assert!(
        WAL_MALLOC_REFUSED.load(Ordering::SeqCst),
        "the hook allocation was refused"
    );
    assert_eq!(
        take(),
        Some(Err(SqlGate::Refused(ScratchFailure::Budget))),
        "an existing first refusal is returned before SQLite is entered"
    );
    assert_eq!(
        main_len(),
        before,
        "no checkpoint ran: the new frames were not backfilled"
    );
    b.clear_write_hooks(&conn).unwrap();
    b.close(conn).map_err(|(_, e)| e).unwrap();

    let b = d.next(16 << 20);
    WAL_BUDGET.with(|slot| *slot.borrow_mut() = Some(d.current.clone()));
    let base = d.current.live();
    let conn = b.open(&path).unwrap();
    // SAFETY: the hook uses static state only.
    b.run(&conn, |q| unsafe { q.set_wal_hook(Some(hook)) })
        .unwrap();
    WAL_PLAN.store(3, Ordering::SeqCst);
    assert_eq!(
        commit_blob(&b, &conn),
        Err(SqlGate::Refused(ScratchFailure::Budget))
    );
    assert!(
        WAL_PAD_HELD.load(Ordering::SeqCst),
        "the pad fit and left less than one minimal block"
    );
    assert_eq!(
        take(),
        Some(Err(SqlGate::Refused(ScratchFailure::Budget))),
        "the checkpoint's own charged refusal is returned as the typed first refusal"
    );
    assert_eq!(b.refusal(), Some(ScratchFailure::Budget));
    b.clear_write_hooks(&conn).unwrap();
    b.close(conn).map_err(|(_, e)| e).unwrap();
    WAL_BUDGET.with(|slot| *slot.borrow_mut() = None);
    assert_eq!(d.current.live(), base, "pad and connection bytes returned");
    let _ = std::fs::remove_dir_all(&dir);
}
