//! #1419 M8D 3A：owner 生涯封印签发的受管 SQL reader。
//!
//! 只在测试驱动（M8D §6：3A/3B 不接非测试调用点，生产来源准备归 3C），生产增量与 native delta
//! 两构建同一份签发。签发方是持私有
//! `Connection` 的 AuditConnection：reader 独占借本事务 `&mut Transaction`，同时在 reader
//! 生涯内给该私有连接装只读授权器，同连接任何别名（`unchecked_transaction`、
//! `execute_batch("COMMIT")`、写入、DDL、带值 PRAGMA、checkpoint）都在 SQLite 预编译时被拒。
//! 事务身份因此由所有权与连接级授权封闭，不靠统计观测。每次节点 SELECT 前，生涯门
//! 用 owner 的真实夹持（FD/WAL/stat/data_version/schema/域/控制原件全等于本切面）、
//! 事务编号/写序、父账身份与材料能力核验；library 只执行这个门，不能自行构造它。
use super::owner_sql::{ChargedOwner, LegacyOwner, OwnerBackend};
use super::*;
use newchan_rust::s_content::m8d_owner_bridge::{
    ChargedReadTx, GatedSqlReader, LiveGate, ReaderTx,
};
use newchan_rust::s_content::{code, CodecError};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use std::ffi::{c_char, c_int, c_void, CStr};

/// reader 生涯内唯一允许的语句：读表、计数、本门与原夹持实际使用的只读 PRAGMA。
const READ_PRAGMAS: [&str; 9] = [
    "read_uncommitted",
    "schema_version",
    "data_version",
    "foreign_keys",
    "recursive_triggers",
    "trusted_schema",
    "query_only",
    "encoding",
    "database_list",
];

fn read_only(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Select | AuthAction::Read { .. } => Authorization::Allow,
        AuthAction::Function {
            function_name: "count",
        } => Authorization::Allow,
        AuthAction::Pragma {
            pragma_name,
            pragma_value: None,
        } if READ_PRAGMAS.contains(&pragma_name) => Authorization::Allow,
        _ => Authorization::Deny,
    }
}

/// [`read_only`] 的同一判据，收费组件连接用的 C 授权回调形式（参数位置同 `sqlite3_set_authorizer`：
/// FUNCTION 的函数名在第 4 参数，PRAGMA 的名与值在第 3、4 参数）。
unsafe extern "C" fn read_only_c(
    _: *mut c_void,
    action: c_int,
    a: *const c_char,
    b: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    let text = |p: *const c_char| {
        // SAFETY: SQLite passes NUL-terminated strings or null.
        (!p.is_null())
            .then(|| unsafe { CStr::from_ptr(p) }.to_str().ok())
            .flatten()
    };
    let allowed = match action {
        rusqlite::ffi::SQLITE_SELECT | rusqlite::ffi::SQLITE_READ => true,
        rusqlite::ffi::SQLITE_FUNCTION => text(b) == Some("count"),
        rusqlite::ffi::SQLITE_PRAGMA => {
            b.is_null() && text(a).is_some_and(|name| READ_PRAGMAS.contains(&name))
        }
        _ => false,
    };
    if allowed {
        rusqlite::ffi::SQLITE_OK
    } else {
        rusqlite::ffi::SQLITE_DENY
    }
}

/// 生涯门在 owner 后端上的落点：连接封印、写前刷页、签出 reader、reader 身份与材料能力。
pub(super) trait GateBackend: OwnerBackend {
    type Reader<'c>: ReaderTx;
    fn seal(conn: &Self::Conn) -> Result<(), String>;
    fn unseal(conn: &Self::Conn);
    fn cache_flush(txn: &Self::Txn<'_>) -> Result<(), String>;
    fn with_reader<'c, R>(txn: &mut Self::Txn<'c>, f: impl FnOnce(&mut Self::Reader<'c>) -> R)
        -> R;
    fn reader_on(reader: &Self::Reader<'_>, conn: &Self::Conn) -> bool;
    fn reader_autocommit(reader: &Self::Reader<'_>) -> bool;
    fn conn_key(conn: &Self::Conn) -> super::owner_sql::JournalKey;
    fn material_writes(
        material: &effects::MaterialLive,
        txn: &Self::Txn<'_>,
    ) -> Result<effects::MaterialWrites, String>;
    fn material_check(material: &effects::MaterialLive, txn: &Self::Txn<'_>) -> Result<(), String>;
    fn material_check_reader(
        material: &effects::MaterialLive,
        reader: &Self::Reader<'_>,
    ) -> Result<(), String>;
}

impl GateBackend for LegacyOwner {
    type Reader<'c> = Transaction<'c>;
    fn seal(conn: &rusqlite::Connection) -> Result<(), String> {
        conn.authorizer(Some(read_only))
            .map_err(history_clamp::stale)
    }
    fn unseal(conn: &rusqlite::Connection) {
        let _ = conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    }
    fn cache_flush(txn: &Transaction<'_>) -> Result<(), String> {
        txn.cache_flush().map_err(history_clamp::stale)
    }
    fn with_reader<'c, R>(
        txn: &mut Transaction<'c>,
        f: impl FnOnce(&mut Transaction<'c>) -> R,
    ) -> R {
        f(txn)
    }
    fn reader_on(reader: &Transaction<'_>, conn: &rusqlite::Connection) -> bool {
        // 比较读者实际借入的事务所在连接与 owner 私有连接；不是读者与自身比较。
        std::ptr::eq::<rusqlite::Connection>(&**reader, conn)
    }
    fn reader_autocommit(reader: &Transaction<'_>) -> bool {
        reader.is_autocommit()
    }
    fn conn_key(conn: &rusqlite::Connection) -> super::owner_sql::JournalKey {
        // SAFETY: the handle is only compared as an address.
        super::owner_sql::JournalKey::Handle(unsafe { conn.handle() } as usize)
    }
    fn material_writes(
        material: &effects::MaterialLive,
        txn: &Transaction<'_>,
    ) -> Result<effects::MaterialWrites, String> {
        material.writes(txn)
    }
    fn material_check(
        material: &effects::MaterialLive,
        txn: &Transaction<'_>,
    ) -> Result<(), String> {
        material.check(txn)
    }
    fn material_check_reader(
        material: &effects::MaterialLive,
        reader: &Transaction<'_>,
    ) -> Result<(), String> {
        material.check(reader)
    }
}

/// 材料期（node 自身写之后、effects 未闭合）的材料能力来自 effects 的 journal 与 hook；effects 的
/// 受管迁移与 PublishPhase/TxStore 同属状态函数单元（主控已裁排序）。在那之前收费 owner 上的
/// 材料期签发一律拒绝，不以空材料或恒真判据代替。
fn charged_material_unavailable() -> String {
    history_clamp::stale("受管 owner 的材料期 reader 归 effects 状态函数单元，尚未接线")
}

impl GateBackend for ChargedOwner {
    type Reader<'c> = ChargedReadTx<'c>;
    fn seal(conn: &super::owner_sql::ChargedOwnerConn) -> Result<(), String> {
        let hook = newchan_rust::s_content::capture_allocation::sqlite_parent::Callback {
            callback: read_only_c
                as newchan_rust::s_content::capture_allocation::sqlite_parent::AuthorizerFn,
            context: std::ptr::null_mut(),
        };
        // SAFETY: the authorizer uses no context.
        conn.binding
            .run(&conn.connection, |q| unsafe {
                q.set_authorizer(Some(hook))
            })
            .map_err(|e| history_clamp::stale(format!("受管SQLite拒绝：{e:?}")))
    }
    fn unseal(conn: &super::owner_sql::ChargedOwnerConn) {
        // 经组件窄收尾撤销本封印：首拒之后也能撤，收尾回滚因而不被只读授权器拒绝；普通 SQL
        // 仍在组件准入处被首拒挡住。撤销失败（绑定已非当前）时授权器留在连接上，只会继续拒绝。
        let _ = conn.binding.clear_authorizer(&conn.connection);
    }
    fn cache_flush(txn: &super::owner_sql::ChargedOwnerTxn<'_>) -> Result<(), String> {
        txn.conn
            .binding
            .run(&txn.conn.connection, |q| q.cache_flush())
            .map_err(|e| history_clamp::stale(format!("受管SQLite拒绝：{e:?}")))
    }
    fn with_reader<'c, R>(
        txn: &mut super::owner_sql::ChargedOwnerTxn<'c>,
        f: impl FnOnce(&mut ChargedReadTx<'c>) -> R,
    ) -> R {
        let conn: &'c super::owner_sql::ChargedOwnerConn = txn.conn;
        let mut reader = ChargedReadTx::new(&conn.binding, &conn.connection);
        f(&mut reader)
    }
    fn reader_on(reader: &ChargedReadTx<'_>, conn: &super::owner_sql::ChargedOwnerConn) -> bool {
        reader.connection_id() == conn.connection.id()
    }
    fn reader_autocommit(reader: &ChargedReadTx<'_>) -> bool {
        reader.in_autocommit()
    }
    fn conn_key(conn: &super::owner_sql::ChargedOwnerConn) -> super::owner_sql::JournalKey {
        super::owner_sql::JournalKey::Charged(conn.connection.id())
    }
    fn material_writes(
        _: &effects::MaterialLive,
        _: &super::owner_sql::ChargedOwnerTxn<'_>,
    ) -> Result<effects::MaterialWrites, String> {
        Err(charged_material_unavailable())
    }
    fn material_check(
        _: &effects::MaterialLive,
        _: &super::owner_sql::ChargedOwnerTxn<'_>,
    ) -> Result<(), String> {
        Err(charged_material_unavailable())
    }
    fn material_check_reader(
        _: &effects::MaterialLive,
        _: &ChargedReadTx<'_>,
    ) -> Result<(), String> {
        Err(charged_material_unavailable())
    }
}

/// 连接级封印。析构撤去授权器；被遗忘时授权器留在连接上，只会继续拒写，不会放宽。
struct ConnectionSeal<'a, B: GateBackend> {
    connection: &'a B::Conn,
}
impl<'a, B: GateBackend> ConnectionSeal<'a, B> {
    fn install(connection: &'a B::Conn) -> Result<Self, String> {
        B::seal(connection)?;
        Ok(Self { connection })
    }
}
impl<B: GateBackend> Drop for ConnectionSeal<'_, B> {
    fn drop(&mut self) {
        B::unseal(self.connection);
    }
}

/// 生涯门。字段全部私有，只由 `AuditTxn::with_step_reader` 构造；没有 Clone、序列化或导出口。
pub(super) struct OwnerLiveGate<'a, 'c, B: GateBackend> {
    owner: &'a AuditConnection<B>,
    transaction_id: u64,
    write_sequence: u64,
    phase: history_clamp::Phase,
    parent: native::Budget,
    material: Option<effects::MaterialLive>,
    deadline: Option<std::time::Instant>,
    _reader: std::marker::PhantomData<&'c ()>,
}

fn source_error(error: String) -> CodecError {
    CodecError::new(code::SOURCE, error)
}

impl<'c, B: GateBackend> OwnerLiveGate<'_, 'c, B> {
    fn core(&self, tx: &B::Reader<'c>) -> Result<(), String> {
        if self.owner.retired.get() {
            return Err(history_clamp::stale("owner 已退役"));
        }
        if !B::reader_on(tx, &self.owner.connection) {
            return Err(history_clamp::stale("reader 事务不在 owner 私有连接上"));
        }
        if B::reader_autocommit(tx)
            || self.owner.next_transaction.get() != self.transaction_id
            || self.owner.write_sequence.get() != self.write_sequence
        {
            return Err(history_clamp::stale("reader 生涯内事务或写序已变化"));
        }
        {
            let clamp = self.owner.history_clamp.borrow();
            let clamp = clamp
                .as_ref()
                .ok_or_else(|| history_clamp::stale("reader 生涯内夹持已撤去"))?;
            if !clamp
                .parent()
                .is_some_and(|parent| parent.same_parent(&self.parent))
                || self.parent.check_current().is_err()
            {
                return Err(history_clamp::stale("reader 父账身份或冻结域已变化"));
            }
            clamp.check_phase(B::conn_sql(&self.owner.connection), &self.phase)?;
        }
        match &self.material {
            Some(material) => B::material_check_reader(material, tx),
            None if effects::has_journal_key(B::conn_key(&self.owner.connection))? => Err(
                history_clamp::stale("材料阶段出现 owner 日志而 reader 未绑定材料能力"),
            ),
            None => Ok(()),
        }
    }
}

impl<'c, B: GateBackend> LiveGate<B::Reader<'c>> for OwnerLiveGate<'_, 'c, B> {
    fn check_live(&self, tx: &B::Reader<'c>) -> Result<(), CodecError> {
        self.sample()?;
        // 入核后的失败沿原夹持语义：粘滞退役并清候选，不换源、不重审恢复。
        let result = self.core(tx);
        self.owner.history_result(result).map_err(source_error)
    }
    fn sample(&self) -> Result<(), CodecError> {
        history::check_deadline(self.deadline).map_err(source_error)
    }
}

/// 签发收据：本次 reader 生涯内节点 SELECT 与固定域元数据读数，供 3B 读数门对账。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct StepReaderReceipt {
    pub(super) node_selects: u64,
    pub(super) metadata_reads: u64,
}

impl<'a, B: GateBackend> AuditTxn<'a, B> {
    /// 签发前置：先用原夹持解释本事务全部自身写（无写走 `check`；已闭合写走 `check_candidate`；
    /// node 自身写后、effects 未闭合的材料期走与 `candidate` 同一判据的材料切面），再封印连接、
    /// 取本切面观测。`parent` 由调用方提供，签发前与 owner 夹持父账比对；异父只拒本次签发，不进 owner 核。
    fn issue_gate<'c>(
        &self,
        parent: &native::Budget,
        material: Option<effects::MaterialLive>,
        deadline: Option<std::time::Instant>,
    ) -> Result<(OwnerLiveGate<'a, 'c, B>, ConnectionSeal<'a, B>), String> {
        history::check_deadline(deadline)?;
        let owner = self.owner;
        let txn = self.transaction.as_ref().unwrap();
        if owner.retired.get() || B::conn_autocommit(&owner.connection) {
            return Err(history_clamp::stale("owner 已退役或事务已终结"));
        }
        {
            let clamp = owner.history_clamp.borrow();
            let clamp = clamp
                .as_ref()
                .ok_or_else(|| history_clamp::stale("受管 reader 需要 owner 夹持"))?;
            if !clamp
                .parent()
                .is_some_and(|current| current.same_parent(parent))
            {
                return Err(history_clamp::stale("reader 父账与 owner 夹持父账不同"));
            }
        }
        parent
            .check_current()
            .map_err(|_| history_clamp::stale("reader 父账冻结域已失效"))?;
        // 先封印连接，再取唯一一次观测：它必须同时通过原判据（无写对生涯基线、已闭合写对
        // effects 候选），并原样作为本切面；签发后的每次 check_phase 只能与这份已核观测全等。
        let open_write = self.dirty.get() && self.history_write.borrow().is_none();
        if open_write && material.is_none() {
            // 未闭合且未绑定材料能力的自身写是签发时点错误，不是外部变化：只拒本次签发，不进 owner 核。
            return Err(history_clamp::stale("自身写尚未闭合为已核 effects"));
        }
        if open_write {
            // 与 finish 前同一物理顺序门：本连接脏页先写入 WAL，否则读期 pager spill 会晚于本切面观测。
            B::cache_flush(txn)?;
        }
        let seal = ConnectionSeal::<B>::install(&owner.connection)?;
        let phase = match (&material, open_write) {
            (Some(material), true) => (|| {
                let writes = B::material_writes(material, txn)?;
                let pending = self.pending.borrow();
                let originals = &pending
                    .as_ref()
                    .ok_or_else(|| history_clamp::stale("材料期缺写前原件"))?
                    .originals;
                let clamp = owner.history_clamp.borrow();
                clamp
                    .as_ref()
                    .unwrap()
                    .phase_from_material(self, &writes, material, originals)
            })(),
            _ if self.dirty.get() => {
                let clamp = owner.history_clamp.borrow();
                let candidate = self.history_write.borrow();
                clamp
                    .as_ref()
                    .unwrap()
                    .phase_from_candidate(self, candidate.as_ref().unwrap())
            }
            _ => {
                let mut clamp = owner.history_clamp.borrow_mut();
                clamp.as_mut().unwrap().phase_from_baseline(self)
            }
        };
        let phase = owner.history_result(phase)?;
        match &material {
            Some(material) => owner.history_result(B::material_check(material, txn))?,
            None if effects::has_journal_key(B::journal_key(txn))? => {
                return Err(history_clamp::stale(
                    "材料阶段签发 reader 必须绑定真实材料能力",
                ))
            }
            None => {}
        }
        Ok((
            OwnerLiveGate {
                owner,
                transaction_id: owner.next_transaction.get(),
                write_sequence: owner.write_sequence.get(),
                phase,
                parent: parent.clone(),
                material,
                deadline,
                _reader: std::marker::PhantomData,
            },
            seal,
        ))
    }

    /// owner 工厂：签发生涯门后，在 `parent` 的真实工作域里签发独占本事务的 reader。
    pub(super) fn with_step_reader<T>(
        &mut self,
        parent: &native::Budget,
        material: Option<effects::MaterialLive>,
        deadline: Option<std::time::Instant>,
        consume: impl FnOnce(&mut GatedSqlReader<'_, 'a, '_, B::Reader<'a>>) -> T,
    ) -> Result<(T, StepReaderReceipt), String> {
        let (gate, seal) = self.issue_gate(parent, material, deadline)?;
        let tx = self.transaction.as_mut().unwrap();
        let failure = |error: native::Failure| match error {
            native::Failure::BudgetExceeded => history_clamp::stale("reader 工作域超过父账额度"),
            native::Failure::Invalid(error) => error,
        };
        let result = parent
            .codec_work()
            .map_err(failure)?
            .run(|work| {
                B::with_reader(tx, |reader| {
                    let mut reader = GatedSqlReader::new(reader, work, &gate)
                        .map_err(|error| error.to_string())?;
                    let value = consume(&mut reader);
                    // 步出口：consume 内超期、取消或外部变化而未再读时，也不得交付成功。
                    reader.exit().map_err(|error| error.to_string())?;
                    let receipt = StepReaderReceipt {
                        node_selects: reader.node_selects(),
                        metadata_reads: reader.metadata_reads(),
                    };
                    Ok::<_, String>((value, receipt))
                })
            })
            .map_err(|error| failure(native::Failure::from(error)));
        drop(seal);
        result
    }
}

impl AuditTxn<'_> {
    /// 只供负控：以本 owner 签发的门核另一个事务，验证门比较的是 owner 私有连接而非自比。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn gate_check_on_for_test(
        &self,
        parent: &native::Budget,
        other: &Transaction<'_>,
    ) -> Result<(), String> {
        let (gate, seal) = self.issue_gate(parent, None, None)?;
        let result = gate.check_live(other).map_err(|error| error.to_string());
        drop(seal);
        result
    }
}

type MaterialHook = Box<dyn for<'a, 'b> FnOnce(&'b mut AuditTxn<'a>)>;
thread_local! {
    static MATERIAL_HOOK: std::cell::RefCell<Option<MaterialHook>> = const { std::cell::RefCell::new(None) };
}
/// 测试驱动：在真实 publish 的 effects 闭合之前（node 写后、Recording 仍活）运行一次。
pub(super) fn arm_material_hook(hook: MaterialHook) {
    MATERIAL_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
}
pub(super) fn run_material_hook(active: &mut AuditTxn<'_>) {
    if let Some(hook) = MATERIAL_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook(active);
    }
}

/// #1419 3C U5：owner 连接、事务守卫与生涯门在收费组件连接上的生命周期子合同（真实父账、
/// 全新进程）。材料期签发依赖 effects（归状态函数单元），此处只断言它被明确拒绝。
#[cfg(test)]
mod charged_tests {
    use super::super::owner_sql::ChargedOwnerConn;
    use super::*;
    use newchan_rust::s_content as sc;
    use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{
        SqlGate, SqliteParentAllocator,
    };
    use std::rc::Rc;

    const LIMIT: usize = 64 << 20;

    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn charged_owner_lifecycle_and_gate_on_same_parent() {
        // SAFETY: this test runs alone in a fresh process before any SQLite use.
        let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
        let root = native::Budget::fresh(LIMIT).unwrap();
        let current = root.bind_frozen(1, LIMIT, 0, 0).unwrap();
        let (parent, origin) = current.sqlite_parent();
        let binding = Rc::new(allocator.bind(parent, origin).expect("bind"));
        let dir = std::env::temp_dir().join(format!("charged-owner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("owner.sqlite3");
        let path = std::ffi::CString::new(db.to_str().unwrap()).unwrap();

        let mut nodes = sc::MemoryNodes::new();
        let reference = sc::write_value(&mut nodes, &sc::Value::String("owner reader".into()))
            .unwrap()
            .content_ref();
        let digest = reference.digest().unwrap();
        let (tag, body) = nodes.get(&digest).unwrap();

        let base = current.live();
        let open = || ChargedOwnerConn {
            binding: binding.clone(),
            connection: binding.open(&path).unwrap(),
        };
        let close = |conn: ChargedOwnerConn| {
            binding.clear_write_hooks(&conn.connection).unwrap();
            binding.close(conn.connection).map_err(|(_, e)| e).unwrap();
        };
        let seed = open();
        binding
            .run(&seed.connection, |q| {
                q.execute_batch(
                    "PRAGMA journal_mode=WAL;
                     CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                     INSERT INTO meta VALUES('rule_revision', 'r');
                     CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),
                       protocol_revision TEXT NOT NULL, session_generation TEXT NOT NULL,
                       clock_plan_hash TEXT NOT NULL, clock_plan_json TEXT NOT NULL,
                       next_attempt_ordinal INTEGER NOT NULL, logical_phase_frontier TEXT NOT NULL);
                     INSERT INTO s_protocol_meta VALUES(1, 's-session/3', 'g', 'h', '{}', 1, 'f');
                     CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL,
                       body BLOB NOT NULL);",
                )?;
                let tag = i64::from(tag);
                q.execute(
                    "INSERT INTO s_content_nodes VALUES(?1, ?2, ?3)",
                    [&&digest[..] as &dyn rusqlite::ToSql, &tag, &body],
                )
                .map(drop)
            })
            .unwrap();

        // 有夹持的 owner：begin 取生涯基线，签发受管 reader 读真实节点，出口，read_end。
        let owner = AuditConnection::<ChargedOwner>::new(open()).unwrap();
        let refusal = std::cell::Cell::new(false);
        *owner.history_clamp.borrow_mut() = Some(
            history_clamp::Clamp::open_in_parent_with_refusal(
                &owner.connection,
                &db,
                1 << 20,
                Some(current.clone()),
                &refusal,
                None,
            )
            .unwrap(),
        );
        let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let (found, receipt) = active
            .with_step_reader(&current, None, None, |reader| {
                reader.frame(&digest, |t, b| (t, b.to_vec()))
            })
            .unwrap();
        assert_eq!(found.unwrap(), Some((tag, body.to_vec())));
        assert_eq!(receipt.node_selects, 1);
        assert!(
            binding.live_bytes() > 0,
            "owner connection bytes live on the parent"
        );
        let ended = active.read_end().unwrap();
        owner.continue_with(ended).unwrap();
        assert!(!owner.retired.get());

        // 有夹持时提交需要已核自身 effects：effects 未迁移前按原判据拒绝，owner 粘滞退役。
        let mut active = owner.begin(TransactionBehavior::Immediate).unwrap();
        let refused = active
            .commit_as(false)
            .err()
            .expect("commit needs closed effects");
        assert!(refused.contains("提交缺已核自身effects"), "{refused}");
        drop(active);
        assert!(
            owner.retired.get(),
            "clamp owner retires on the refused commit"
        );
        close(owner.connection);

        // 外连接提交：生涯门按夹持判 stale 并退役，不换源。
        let owner = AuditConnection::<ChargedOwner>::new(open()).unwrap();
        *owner.history_clamp.borrow_mut() = Some(
            history_clamp::Clamp::open_in_parent_with_refusal(
                &owner.connection,
                &db,
                1 << 20,
                Some(current.clone()),
                &refusal,
                None,
            )
            .unwrap(),
        );
        let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
        binding
            .run(&seed.connection, |q| {
                q.execute_batch("INSERT INTO meta VALUES('foreign', 'write');")
            })
            .unwrap();
        let stale = active.with_step_reader(&current, None, None, |_| ());
        assert!(
            stale.is_err(),
            "a foreign commit is not explained by the clamp"
        );
        drop(active);
        assert!(owner.retired.get());
        close(owner.connection);

        // 无夹持的 owner：守卫 COMMIT 成功；读中收费首拒后提交失败、收尾回滚、首拒保留。
        let owner = AuditConnection::<ChargedOwner>::new(open()).unwrap();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        binding
            .run(&owner.connection.connection, |q| {
                q.execute_batch("INSERT INTO meta VALUES('own', 'write');")
            })
            .unwrap();
        let ended = active.commit().unwrap();
        owner.continue_with(ended).unwrap();
        let active = owner.begin(TransactionBehavior::Immediate).unwrap();
        let first = binding.run(&owner.connection.connection, |_| {
            // SAFETY: plain SQLite allocation inside the operation domain.
            assert!(unsafe { rusqlite::ffi::sqlite3_malloc64(LIMIT as u64 + 1) }.is_null());
            Ok(())
        });
        assert_eq!(first, Err(SqlGate::Refused(ScratchFailure::Budget)));
        let failed = active
            .commit()
            .err()
            .expect("COMMIT refused after the first refusal");
        assert!(failed.contains("Refused(Budget)"), "{failed}");
        assert!(
            owner.connection.connection.is_autocommit(),
            "teardown rolled the transaction back"
        );
        assert_eq!(
            binding.refusal(),
            Some(ScratchFailure::Budget),
            "first refusal kept"
        );
        assert!(
            !owner.retired.get(),
            "a confirmed rollback after the failed COMMIT keeps the owner"
        );
        close(owner.connection);
        close(seed);
        assert_eq!(
            current.live(),
            base,
            "all owner and connection bytes returned"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// U5 独审 F2/F3：封印中的受管 reader 在下一次生涯检查的夹持 SQL 内首次被收费拒绝。读者按值
    /// 得到工作域的 Budget 拒绝（不是 SOURCE），工作域据此报资源失败；owner 退役；封印经窄收尾撤销，
    /// 事务因而能回滚；首拒保留，字节返还。
    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn charged_sealed_reader_first_refusal_in_clamp_sql_is_typed_and_rolls_back() {
        // SAFETY: this test runs alone in a fresh process before any SQLite use.
        let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
        let root = native::Budget::fresh(LIMIT).unwrap();
        let current = root.bind_frozen(1, LIMIT, 0, 0).unwrap();
        let (parent, origin) = current.sqlite_parent();
        let binding = Rc::new(allocator.bind(parent, origin).expect("bind"));
        let dir = std::env::temp_dir().join(format!("charged-seal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("owner.sqlite3");
        let path = std::ffi::CString::new(db.to_str().unwrap()).unwrap();
        let mut nodes = sc::MemoryNodes::new();
        let reference = sc::write_value(&mut nodes, &sc::Value::String("sealed reader".into()))
            .unwrap()
            .content_ref();
        let digest = reference.digest().unwrap();
        let (tag, body) = nodes.get(&digest).unwrap();

        let base = current.live();
        let seed = ChargedOwnerConn {
            binding: binding.clone(),
            connection: binding.open(&path).unwrap(),
        };
        binding
            .run(&seed.connection, |q| {
                q.execute_batch(
                    "PRAGMA journal_mode=WAL;
                     CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                     INSERT INTO meta VALUES('rule_revision', 'r');
                     CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),
                       protocol_revision TEXT NOT NULL, session_generation TEXT NOT NULL,
                       clock_plan_hash TEXT NOT NULL, clock_plan_json TEXT NOT NULL,
                       next_attempt_ordinal INTEGER NOT NULL, logical_phase_frontier TEXT NOT NULL);
                     INSERT INTO s_protocol_meta VALUES(1, 's-session/3', 'g', 'h', '{}', 1, 'f');
                     CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL,
                       body BLOB NOT NULL);",
                )?;
                let tag = i64::from(tag);
                q.execute(
                    "INSERT INTO s_content_nodes VALUES(?1, ?2, ?3)",
                    [&&digest[..] as &dyn rusqlite::ToSql, &tag, &body],
                )
                .map(drop)
            })
            .unwrap();

        let owner = AuditConnection::<ChargedOwner>::new(ChargedOwnerConn {
            binding: binding.clone(),
            connection: binding.open(&path).unwrap(),
        })
        .unwrap();
        let refusal = std::cell::Cell::new(false);
        *owner.history_clamp.borrow_mut() = Some(
            history_clamp::Clamp::open_in_parent_with_refusal(
                &owner.connection,
                &db,
                1 << 20,
                Some(current.clone()),
                &refusal,
                None,
            )
            .unwrap(),
        );
        let mut active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let mut pad: *mut c_void = std::ptr::null_mut();
        let mut observed = None;
        let issued = active.with_step_reader(&current, None, None, |reader| {
            // 封印已装、reader 已签发：把本冻结域余额借到不足一个最小块，下一次 SQLite 分配
            // 就落在生涯门的夹持观测 SQL 里。
            let payload = current.remaining().saturating_sub(64) / 16 * 16;
            binding
                .run(&owner.connection.connection, |_| {
                    // SAFETY: plain SQLite allocation inside the operation domain.
                    pad = unsafe { rusqlite::ffi::sqlite3_malloc64(payload as u64) };
                    Ok(())
                })
                .unwrap();
            observed = Some(
                reader
                    .frame(&digest, |t, b| (t, b.to_vec()))
                    .err()
                    .map(|e| (e.scratch_failure(), e.code())),
            );
        });
        assert!(!pad.is_null(), "the pad fit inside the domain");
        let (failure, code) = observed.flatten().expect("the gate check refused the read");
        assert_eq!(
            failure,
            Some(ScratchFailure::Budget),
            "typed Budget, by value"
        );
        assert_ne!(code, code::SOURCE, "not reported as a SOURCE error");
        let issued = issued
            .err()
            .expect("the work domain reports the resource failure");
        assert!(issued.contains("reader 工作域超过父账额度"), "{issued}");
        assert_eq!(
            binding.refusal(),
            Some(ScratchFailure::Budget),
            "first refusal kept"
        );
        assert!(
            owner.retired.get(),
            "the failed lifetime check retires the owner"
        );

        drop(active);
        assert!(
            owner.connection.connection.is_autocommit(),
            "seal removed in teardown, so the teardown rollback ran"
        );
        // SAFETY: pad is a block from sqlite3_malloc64; freeing needs no operation domain.
        unsafe { rusqlite::ffi::sqlite3_free(pad) };
        let close = |conn: ChargedOwnerConn| {
            binding.clear_write_hooks(&conn.connection).unwrap();
            binding.close(conn.connection).map_err(|(_, e)| e).unwrap();
        };
        close(owner.connection);
        close(seed);
        assert_eq!(
            current.live(),
            base,
            "pad, owner and connection bytes returned"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
