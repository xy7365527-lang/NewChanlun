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
use super::*;
use newchan_rust::s_content::m8d_owner_bridge::{GatedSqlReader, LiveGate};
use newchan_rust::s_content::{code, CodecError};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

/// reader 生涯内唯一允许的语句：读表、计数、本门与原夹持实际使用的只读 PRAGMA。
fn read_only(ctx: AuthContext<'_>) -> Authorization {
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

/// 连接级封印。析构撤去授权器；被遗忘时授权器留在连接上，只会继续拒写，不会放宽。
struct ConnectionSeal<'a> {
    connection: &'a rusqlite::Connection,
}
impl<'a> ConnectionSeal<'a> {
    fn install(connection: &'a rusqlite::Connection) -> Result<Self, String> {
        connection
            .authorizer(Some(read_only))
            .map_err(history_clamp::stale)?;
        Ok(Self { connection })
    }
}
impl Drop for ConnectionSeal<'_> {
    fn drop(&mut self) {
        let _ = self
            .connection
            .authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    }
}

/// 生涯门。字段全部私有，只由 `AuditTxn::with_step_reader` 构造；没有 Clone、序列化或导出口。
pub(super) struct OwnerLiveGate<'a> {
    owner: &'a AuditConnection,
    transaction_id: u64,
    write_sequence: u64,
    phase: history_clamp::Phase,
    parent: native::Budget,
    material: Option<effects::MaterialLive>,
    deadline: Option<std::time::Instant>,
}

fn source_error(error: String) -> CodecError {
    CodecError::new(code::SOURCE, error)
}

impl OwnerLiveGate<'_> {
    fn core(&self, tx: &Transaction<'_>) -> Result<(), String> {
        if self.owner.retired.get() {
            return Err(history_clamp::stale("owner 已退役"));
        }
        // 比较读者实际借入的事务所在连接与 owner 私有连接；不是读者与自身比较。
        if !std::ptr::eq::<rusqlite::Connection>(&**tx, &self.owner.connection) {
            return Err(history_clamp::stale("reader 事务不在 owner 私有连接上"));
        }
        if tx.is_autocommit()
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
            clamp.check_phase(tx, &self.phase)?;
        }
        match &self.material {
            Some(material) => material.check(tx),
            None if effects::has_journal(tx)? => Err(history_clamp::stale(
                "材料阶段出现 owner 日志而 reader 未绑定材料能力",
            )),
            None => Ok(()),
        }
    }
}

impl<'c> LiveGate<Transaction<'c>> for OwnerLiveGate<'_> {
    fn check_live(&self, tx: &Transaction<'c>) -> Result<(), CodecError> {
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

impl<'a> AuditTxn<'a> {
    /// 签发前置：先用原夹持解释本事务全部自身写（无写走 `check`；已闭合写走 `check_candidate`；
    /// node 自身写后、effects 未闭合的材料期走与 `candidate` 同一判据的材料切面），再封印连接、
    /// 取本切面观测。`parent` 由调用方提供，签发前与 owner 夹持父账比对；异父只拒本次签发，不进 owner 核。
    fn issue_gate(
        &self,
        parent: &native::Budget,
        material: Option<effects::MaterialLive>,
        deadline: Option<std::time::Instant>,
    ) -> Result<(OwnerLiveGate<'a>, ConnectionSeal<'a>), String> {
        history::check_deadline(deadline)?;
        let owner = self.owner;
        if owner.retired.get() || self.tx().is_autocommit() {
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
            self.tx().cache_flush().map_err(history_clamp::stale)?;
        }
        let seal = ConnectionSeal::install(&owner.connection)?;
        let phase = match (&material, open_write) {
            (Some(material), true) => (|| {
                let writes = material.writes(self.tx())?;
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
            Some(material) => owner.history_result(material.check(self.tx()))?,
            None if effects::has_journal(self.tx())? => {
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
        consume: impl FnOnce(&mut GatedSqlReader<'_, 'a, '_>) -> T,
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
                let mut reader =
                    GatedSqlReader::new(tx, work, &gate).map_err(|error| error.to_string())?;
                let value = consume(&mut reader);
                // 步出口：consume 内超期、取消或外部变化而未再读时，也不得交付成功。
                reader.exit().map_err(|error| error.to_string())?;
                let receipt = StepReaderReceipt {
                    node_selects: reader.node_selects(),
                    metadata_reads: reader.metadata_reads(),
                };
                Ok::<_, String>((value, receipt))
            })
            .map_err(|error| failure(native::Failure::from(error)));
        drop(seal);
        result
    }

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
