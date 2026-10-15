//! #1394 固定写口的实际 SQLite 效果。hook 只记录表/rowid；原件必须在锁内回读。
//! 每次扩容先检查旧新容量共存账；预算不足撤销可选候选，不改变业务写入结果。
use super::super::audit_retention::{sum, vec_bytes, Retained};
use super::super::audit_store::CodecRead;
use super::super::request_audit::{ExactRows, SqlCell};
use super::*;
use newchan_rust::s_content::{
    self as sc,
    capture_allocation::scratch::{NativeReservation, WorkError, WorkScope},
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Table {
    Meta,
    Raw,
    Ohlc,
    Message,
    Phase,
    Protocol,
    Epoch,
    Delta,
    Publication,
    Object,
    Witness,
    Relation,
    Observation,
    Catalog,
    Delivery,
    Policy,
    Node,
}
impl Table {
    pub(super) const ALL: [Self; 17] = [
        Self::Meta,
        Self::Raw,
        Self::Ohlc,
        Self::Message,
        Self::Phase,
        Self::Protocol,
        Self::Epoch,
        Self::Delta,
        Self::Publication,
        Self::Object,
        Self::Witness,
        Self::Relation,
        Self::Observation,
        Self::Catalog,
        Self::Delivery,
        Self::Policy,
        Self::Node,
    ];
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Meta => "meta",
            Self::Raw => "raw_events",
            Self::Ohlc => "raw_ohlc",
            Self::Message => "s_input_messages",
            Self::Phase => "s_clock_events",
            Self::Protocol => "s_protocol_meta",
            Self::Epoch => "writer_epoch_history",
            Self::Delta => "structure_deltas",
            Self::Publication => "s_content_publications",
            Self::Object => "s_content_objects",
            Self::Witness => "s_content_witnesses",
            Self::Relation => "s_content_relations",
            Self::Observation => "s_content_observations",
            Self::Catalog => "catalog",
            Self::Delivery => "s_delivery_refs",
            Self::Policy => "s_delivery_policy",
            Self::Node => "s_content_nodes",
        }
    }
    // 固定表集合的SQL借用静态字节；原件复制由ExactRows先行计费。
    fn row_sql(self) -> &'static str {
        match self {
            Self::Meta => "SELECT rowid,* FROM main.\"meta\" WHERE rowid=?1",
            Self::Raw => "SELECT rowid,* FROM main.\"raw_events\" WHERE rowid=?1",
            Self::Ohlc => "SELECT rowid,* FROM main.\"raw_ohlc\" WHERE rowid=?1",
            Self::Message => "SELECT rowid,* FROM main.\"s_input_messages\" WHERE rowid=?1",
            Self::Phase => "SELECT rowid,* FROM main.\"s_clock_events\" WHERE rowid=?1",
            Self::Protocol => "SELECT rowid,* FROM main.\"s_protocol_meta\" WHERE rowid=?1",
            Self::Epoch => "SELECT rowid,* FROM main.\"writer_epoch_history\" WHERE rowid=?1",
            Self::Delta => "SELECT rowid,* FROM main.\"structure_deltas\" WHERE rowid=?1",
            Self::Publication => {
                "SELECT rowid,* FROM main.\"s_content_publications\" WHERE rowid=?1"
            }
            Self::Object => "SELECT rowid,* FROM main.\"s_content_objects\" WHERE rowid=?1",
            Self::Witness => "SELECT rowid,* FROM main.\"s_content_witnesses\" WHERE rowid=?1",
            Self::Relation => "SELECT rowid,* FROM main.\"s_content_relations\" WHERE rowid=?1",
            Self::Observation => {
                "SELECT rowid,* FROM main.\"s_content_observations\" WHERE rowid=?1"
            }
            Self::Catalog => "SELECT rowid,* FROM main.\"catalog\" WHERE rowid=?1",
            Self::Delivery => "SELECT rowid,* FROM main.\"s_delivery_refs\" WHERE rowid=?1",
            Self::Policy => "SELECT rowid,* FROM main.\"s_delivery_policy\" WHERE rowid=?1",
            Self::Node => "SELECT 0,digest,tag,body FROM main.s_content_nodes WHERE digest=?1",
        }
    }
    fn original_sql(self) -> &'static str {
        match self {
            Self::Meta => "SELECT rowid,* FROM main.\"meta\" ORDER BY rowid",
            Self::Raw => "SELECT rowid,* FROM main.\"raw_events\" ORDER BY rowid",
            Self::Ohlc => "SELECT rowid,* FROM main.\"raw_ohlc\" ORDER BY rowid",
            Self::Message => "SELECT rowid,* FROM main.\"s_input_messages\" ORDER BY rowid",
            Self::Phase => "SELECT rowid,* FROM main.\"s_clock_events\" ORDER BY rowid",
            Self::Protocol => "SELECT rowid,* FROM main.\"s_protocol_meta\" ORDER BY rowid",
            Self::Epoch => "SELECT rowid,* FROM main.\"writer_epoch_history\" ORDER BY rowid",
            Self::Delta => "SELECT rowid,* FROM main.\"structure_deltas\" ORDER BY rowid",
            Self::Publication => {
                "SELECT rowid,* FROM main.\"s_content_publications\" ORDER BY rowid"
            }
            Self::Object => "SELECT rowid,* FROM main.\"s_content_objects\" ORDER BY rowid",
            Self::Witness => "SELECT rowid,* FROM main.\"s_content_witnesses\" ORDER BY rowid",
            Self::Relation => "SELECT rowid,* FROM main.\"s_content_relations\" ORDER BY rowid",
            Self::Observation => {
                "SELECT rowid,* FROM main.\"s_content_observations\" ORDER BY rowid"
            }
            Self::Catalog => "SELECT rowid,* FROM main.\"catalog\" ORDER BY rowid",
            Self::Delivery => "SELECT rowid,* FROM main.\"s_delivery_refs\" ORDER BY rowid",
            Self::Policy => "SELECT rowid,* FROM main.\"s_delivery_policy\" ORDER BY rowid",
            Self::Node => "SELECT rowid,* FROM main.\"s_content_nodes\" ORDER BY rowid",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == s)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operation {
    Accept,
    Begin,
    Publish,
    Recover,
}
impl Operation {
    fn permits(self, t: Table, action: rusqlite::hooks::Action) -> bool {
        use rusqlite::hooks::Action::{SQLITE_DELETE as D, SQLITE_INSERT as I, SQLITE_UPDATE as U};
        use Table::*;
        match (self, t, action) {
            (Self::Accept, Raw | Ohlc | Message | Phase, I) => true,
            (Self::Accept, Protocol, U) => true,
            (Self::Begin, Phase, I) => true,
            (Self::Begin, Message | Protocol, U) => true,
            (Self::Begin, Meta, I | U) => true,
            (Self::Recover, Epoch | Phase, I) => true,
            (Self::Recover, Protocol, U) => true,
            (Self::Recover, Meta, I | U) => true,
            (
                Self::Publish,
                Node | Object | Witness | Relation | Observation | Delta | Publication | Phase
                | Delivery,
                I,
            ) => true,
            (Self::Publish, Object | Message | Protocol | Catalog | Policy, U) => true,
            (Self::Publish, Meta, I | U) => true,
            (Self::Publish, Delivery, D) => true,
            _ => false,
        }
    }
}
#[derive(Clone, Copy)]
struct Key {
    table: Table,
    rowid: i64,
    digest: [u8; 32],
    action: Option<rusqlite::hooks::Action>,
    material_epoch: u64,
    node_write: Option<(u64, u64, bool)>,
}
/// #1488：覆盖收据只证明实际写效果均被解释，不证明业务集合变化完整。
struct Coverage {
    start: u64,
    hook_events: u64,
    node_inserts: u64,
    exact: bool,
}
struct Journal {
    keys: Vec<Key>,
    nodes: std::collections::HashSet<[u8; 32]>,
    operation: Operation,
    overflow: bool,
    unexpected: bool,
    cap: usize,
    bytes: usize,
    peak: usize,
    coverage: Option<Coverage>,
    set_required: bool,
    set_seed: Option<construction_set::Seed>,
    set: Option<construction_set::PendingSet>,
    stages: Vec<StageReceipt>,
    active_stage: Option<usize>,
    material_epoch: u64,
    extra_bytes: usize,
    consumed: std::collections::HashSet<(Table, i64)>,
    stage_rows_bytes: usize,
    // 所有载荷字段在resources之前：错误Drop先析构载荷，再释放外层cap许可。
    resources: Option<MaterialResources>,
}
struct MaterialResources {
    budget: native::Budget,
    // Some(work)对本片子域只占一次cap；不是另一次逐容器父账收费。
    outer_cap: Option<NativeReservation>,
}
impl Journal {
    fn layout(keys: usize, node_capacity: usize) -> usize {
        sum([
            vec_bytes::<Key>(keys),
            super::super::audit_retention::hash_slots::<[u8; 32], ()>(node_capacity),
            std::mem::size_of::<Self>(),
            128,
        ])
    }
    fn reserve_one(&mut self, node: bool) -> bool {
        if self.overflow {
            return false;
        }
        let grow_keys = self.keys.len() == self.keys.capacity();
        let grow_nodes = node && self.nodes.len() == self.nodes.capacity();
        if !grow_keys && !grow_nodes {
            return true;
        }
        let keys = if grow_keys {
            self.keys.capacity().saturating_mul(2).max(32)
        } else {
            self.keys.capacity()
        };
        let nodes = if grow_nodes {
            self.nodes.capacity().saturating_mul(2).max(32)
        } else {
            self.nodes.capacity()
        };
        // 只预留实际增长的容器；未增长的另一个容器已经在 self.bytes 中。
        // std SwissTable 的下一桶档按小表/7:8装载率推导，再按实际capacity结算。
        let node_capacity = if grow_nodes {
            match super::native::hash_capacity(nodes) {
                Ok(capacity) => capacity,
                Err(_) => {
                    self.overflow = true;
                    return false;
                }
            }
        } else {
            0
        };
        let new_blocks = sum([
            if grow_keys { vec_bytes::<Key>(keys) } else { 0 },
            if grow_nodes {
                super::super::audit_retention::hash_slots::<[u8; 32], ()>(node_capacity)
            } else {
                0
            },
        ]);
        let coexist = sum([self.bytes, new_blocks]);
        if coexist > self.cap {
            self.overflow = true;
            return false;
        }
        self.peak = self.peak.max(coexist);
        let permit = match self
            .resources
            .as_ref()
            .map(|r| r.budget.reserve(new_blocks))
            .transpose()
        {
            Ok(permit) => permit,
            Err(_) => {
                self.overflow = true;
                return false;
            }
        };
        let old_keys = self.keys.capacity();
        let old_nodes = self.nodes.capacity();
        let succeeded = (!grow_keys || self.keys.try_reserve_exact(keys - self.keys.len()).is_ok())
            && (!grow_nodes || self.nodes.try_reserve(nodes - self.nodes.len()).is_ok());
        if let Some(permit) = permit {
            // 包括第二个容器分配失败的出口；首个实际新块不能提前返还许可。
            let changed_keys = old_keys != self.keys.capacity();
            let changed_nodes = old_nodes != self.nodes.capacity();
            let block = |k, n| {
                sum([
                    if changed_keys { vec_bytes::<Key>(k) } else { 0 },
                    if changed_nodes {
                        super::super::audit_retention::hash_slots::<[u8; 32], ()>(n)
                    } else {
                        0
                    },
                ])
            };
            permit.commit(
                block(old_keys, old_nodes),
                block(self.keys.capacity(), self.nodes.capacity()),
            );
        }
        self.bytes = sum([
            Self::layout(self.keys.capacity(), self.nodes.capacity()),
            self.extra_bytes,
        ]);
        self.overflow |= !succeeded;
        succeeded
    }
}
pub(super) struct Recording {
    journal: Arc<Mutex<Journal>>,
}
impl Recording {
    pub(super) fn bind_set(&self, seed: construction_set::Seed) -> Result<(), String> {
        let mut j = self.journal.lock().map_err(invalid)?;
        if j.operation != Operation::Publish || j.set_required || !j.keys.is_empty() {
            return Err(construction_state::stale("集合seed不是本次publish写前绑定"));
        }
        j.set_required = true;
        j.set_seed = Some(seed);
        Ok(())
    }
    pub(super) fn start(
        tx: &Transaction<'_>,
        operation: Operation,
        cap: usize,
        strict: bool,
    ) -> Result<Option<Self>, String> {
        let inline = std::mem::size_of::<Journal>() + 128;
        if cap < inline {
            return Ok(None);
        }
        // 复用实际 TxStore 的 schema/FK/唯一键门；total_changes 单独不覆盖 REPLACE 隐式删除。
        let coverage = if strict {
            if !crate::content_store::owner_node_insert_gate(tx).map_err(invalid)? {
                return Err(construction_state::stale("节点schema不支持精确effects覆盖"));
            }
            Some(Coverage {
                start: tx.total_changes(),
                hook_events: 0,
                node_inserts: 0,
                exact: true,
            })
        } else {
            None
        };
        let mut journal = Journal {
            keys: Vec::new(),
            nodes: Default::default(),
            operation,
            overflow: false,
            unexpected: false,
            cap,
            bytes: inline,
            peak: inline,
            coverage,
            set_required: false,
            set_seed: None,
            set: None,
            stages: Vec::new(),
            active_stage: None,
            material_epoch: 0,
            extra_bytes: 0,
            consumed: Default::default(),
            stage_rows_bytes: 0,
            resources: None,
        };
        if !journal.reserve_one(false) {
            return Ok(None);
        }
        let journal = Arc::new(Mutex::new(journal));
        let hook = journal.clone();
        tx.update_hook(Some(move |action, db: &str, table: &str, rowid| {
            let mut j = hook.lock().expect("同步 owner 效果记录锁");
            if let Some(c) = j.coverage.as_mut() {
                match c.hook_events.checked_add(1) {
                    Some(n) => c.hook_events = n,
                    None => c.exact = false,
                }
            }
            let Some(table) = Table::parse(table) else {
                j.unexpected = true;
                return;
            };
            if db != "main" || !j.operation.permits(table, action) {
                j.unexpected = true;
                return;
            }
            if !j.reserve_one(false) {
                return;
            }
            let material_epoch = j.material_epoch;
            j.keys.push(Key {
                table,
                rowid,
                digest: [0; 32],
                action: Some(action),
                material_epoch,
                node_write: None,
            });
        }))
        .map_err(invalid)?;
        NODE_JOURNAL.with(|slot| {
            let mut slot = slot.borrow_mut();
            assert!(slot.is_none(), "同线程不能嵌套 owner 写口");
            *slot = Some((unsafe { tx.handle() } as usize, journal.clone()));
        });
        Ok(Some(Self { journal }))
    }
    pub(super) fn finish(
        self,
        tx: &Transaction<'_>,
        originals: &mut [ExactRows],
        cap: usize,
    ) -> Result<Option<Effects>, String> {
        tx.update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
            .map_err(invalid)?;
        clear_node_journal(tx);
        let mut journal = Arc::try_unwrap(self.journal)
            .map_err(|_| invalid("效果日志仍有外借"))?
            .into_inner()
            .map_err(invalid)?;
        if journal.unexpected {
            return Err(invalid("固定 operation 出现未声明实际效果"));
        }
        if journal.overflow {
            return Ok(None);
        }
        let covered_total = if let Some(c) = journal.coverage.as_ref() {
            let after = tx.total_changes();
            let expected = c.hook_events.checked_add(c.node_inserts);
            if !c.exact || after.checked_sub(c.start) != expected {
                return Err(construction_state::stale(
                    "actual writes与hook/node收据不一致",
                ));
            }
            Some(after)
        } else {
            None
        };
        if journal.set_required {
            validate_stage_coverage(&journal)?;
            journal.set.as_mut().unwrap().close_stages(
                journal.material_epoch,
                journal.keys.len(),
                journal.stages.len(),
                covered_total.ok_or_else(|| construction_state::stale("集合缺完整实际写覆盖"))?,
            )?;
        }
        journal
            .keys
            .sort_unstable_by_key(|k| (k.table, k.rowid, k.digest));
        journal.keys.dedup_by_key(|k| (k.table, k.rowid, k.digest));
        let mut changes = Vec::new();
        let initial = sum([journal.bytes, vec_bytes::<Change>(journal.keys.len())]);
        if initial >= cap || changes.try_reserve_exact(journal.keys.len()).is_err() {
            return Ok(None);
        }
        let mut used = initial;
        let mut peak = used.max(journal.peak);
        let mut lower = (Table::Meta, 0usize);
        for key in journal.keys {
            if lower.0 != key.table {
                lower = (key.table, 0);
            }
            let sql = key.table.row_sql();
            let Some(remaining) = cap.checked_sub(used) else {
                return Ok(None);
            };
            let loaded = if key.table == Table::Node {
                ExactRows::read_optional_bounded_params(tx, sql, [&key.digest[..]], remaining)?
            } else {
                ExactRows::read_optional_bounded_params(tx, sql, [key.rowid], remaining)?
            };
            let Some((mut rows, read_peak)) = loaded else {
                return Ok(None);
            };
            peak = peak.max(sum([used, read_peak]));
            if rows.rows.len() > 1 {
                return Err(invalid("rowid 效果非唯一"));
            }
            if let Some(row) = rows.rows.first() {
                verify_storage(tx, key.table, row)?;
            }
            let new = rows.rows.pop();
            let mut slot = None;
            let old = if key.table == Table::Node {
                None
            } else {
                let table = table_mut(originals, key.table)?;
                match table.rows[lower.1..]
                    .binary_search_by_key(&key.rowid, |r| integer(&r[0]).unwrap_or(i64::MIN))
                {
                    Ok(i) => {
                        let i = i + lower.1;
                        lower.1 = i + 1;
                        slot = Some(i);
                        Some(std::mem::take(&mut table.rows[i]))
                    }
                    Err(_) => None,
                }
            };
            used = sum([used, new.heap_bytes()]);
            changes.push(Change {
                table: key.table,
                rowid: key.rowid,
                slot,
                old,
                new,
            });
        }
        if journal.set_required {
            for stage in &journal.stages {
                let actual = changes
                    .iter()
                    .find(|c| c.table == stage.kind.table() && c.rowid == stage.rowid)
                    .and_then(|c| c.new.as_ref());
                if actual != Some(&stage.row) {
                    return Err(construction_state::stale(
                        "stage即时原件与最终effects不一致",
                    ));
                }
            }
        }
        let mut set = journal.set.take();
        if let Some(set) = &mut set {
            let charge = journal.resources.as_mut().and_then(|r| r.outer_cap.take());
            set.hold_parent_charge(charge)?;
        }
        let set_bytes = set.as_ref().map_or(0, |set| set.retained_bytes());
        Ok(Some(Effects {
            changes,
            set,
            operation: journal.operation,
            peak,
            // journal及读行头已真实Drop；旧载荷仍由原件账全额承接，不在这里重复计入。
            bytes: sum([used - journal.bytes, set_bytes]),
            covered_total,
        }))
    }
}
pub(super) struct Change {
    pub(super) table: Table,
    pub(super) rowid: i64,
    slot: Option<usize>,
    pub(super) old: Option<Vec<SqlCell>>,
    pub(super) new: Option<Vec<SqlCell>>,
}
pub(super) struct Effects {
    pub(super) set: Option<construction_set::PendingSet>,
    pub(super) changes: Vec<Change>,
    pub(super) operation: Operation,
    pub(super) peak: usize,
    pub(super) bytes: usize,
    covered_total: Option<u64>,
}
impl Effects {
    pub(super) fn has_exact_coverage(&self, total_changes: u64) -> bool {
        self.covered_total == Some(total_changes)
    }
    pub(super) fn rows(&self, table: Table) -> impl Iterator<Item = &Change> {
        self.changes.iter().filter(move |c| c.table == table)
    }
    pub(super) fn apply(
        mut self,
        originals: &mut [ExactRows],
        budget: &super::native::Budget,
        original_charge: &mut usize,
    ) -> Result<usize, super::native::Failure> {
        let mut peak = self.peak;
        let mut shift = (Table::Meta, 0isize);
        for change in &mut self.changes {
            if shift.0 != change.table {
                shift = (change.table, 0);
            }
            if change.table == Table::Node {
                continue;
            }
            let rows = &mut table_mut(originals, change.table)?.rows;
            // 被取走的旧行槽仍在原位置，由 rowid 已有顺序确定插入点。
            let index = if let Some(slot) = change.slot {
                slot.checked_add_signed(shift.1)
                    .ok_or_else(|| invalid("效果原槽溢出"))?
            } else {
                rows.partition_point(|r| integer(&r[0]).unwrap_or(i64::MIN) < change.rowid)
            };
            let old_payload = change.old.heap_bytes();
            let new_payload = change.new.heap_bytes();
            let old_capacity = vec_bytes::<Vec<SqlCell>>(rows.capacity());
            match change.new.take() {
                Some(row) if change.old.is_some() => {
                    rows[index] = row;
                }
                Some(row) => {
                    if rows.len() == rows.capacity() {
                        let next = rows
                            .capacity()
                            .checked_mul(2)
                            .ok_or(super::native::Failure::BudgetExceeded)?
                            .max(4);
                        let request = vec_bytes::<Vec<SqlCell>>(next)
                            .checked_add(self.bytes)
                            .ok_or(super::native::Failure::BudgetExceeded)?;
                        peak = peak.max(request);
                    }
                    budget.vec_grow(rows, 1)?;
                    rows.insert(index, row);
                    shift.1 += 1;
                }
                None if change.old.is_some() => {
                    rows.remove(index);
                    shift.1 -= 1;
                }
                None => return Err(invalid("固定效果同时缺旧新原件").into()),
            }
            *original_charge = original_charge
                .checked_sub(old_payload)
                .and_then(|n| n.checked_add(new_payload))
                .and_then(|n| {
                    n.checked_add(vec_bytes::<Vec<SqlCell>>(rows.capacity()) - old_capacity)
                })
                .ok_or_else(|| invalid("SQL原件增量归属账溢出/缺旧费用"))?;
        }
        Ok(peak)
    }
}
pub(super) fn table(originals: &[ExactRows], t: Table) -> Result<&ExactRows, String> {
    let sql = t.original_sql();
    originals
        .iter()
        .find(|r| r.sql == sql)
        .ok_or_else(|| invalid(format!("缺完整 {} 原件", t.name())))
}
fn table_mut(originals: &mut [ExactRows], t: Table) -> Result<&mut ExactRows, String> {
    let sql = t.original_sql();
    originals
        .iter_mut()
        .find(|r| r.sql == sql)
        .ok_or_else(|| invalid(format!("缺完整 {} 原件", t.name())))
}
pub(super) fn integer(c: &SqlCell) -> Result<i64, String> {
    if let SqlCell::Integer(n) = c {
        Ok(*n)
    } else {
        Err(invalid("原件不是 INTEGER"))
    }
}
pub(super) fn text(c: &SqlCell) -> Result<&str, String> {
    if let SqlCell::Text(b) = c {
        std::str::from_utf8(b).map_err(invalid)
    } else {
        Err(invalid("原件不是 TEXT"))
    }
}
pub(super) fn optional_integer(c: &SqlCell) -> Result<Option<i64>, String> {
    if matches!(c, SqlCell::Null) {
        Ok(None)
    } else {
        integer(c).map(Some)
    }
}
fn verify_storage(tx: &Transaction<'_>, table: Table, row: &[SqlCell]) -> Result<(), String> {
    use rusqlite::types::Type;
    let mut stmt = tx
        .prepare("SELECT type,\"notnull\",pk FROM pragma_table_info(?1) ORDER BY cid")
        .map_err(invalid)?;
    let mut columns = stmt.query([table.name()]).map_err(invalid)?;
    let mut count = 0;
    while let Some(c) = columns.next().map_err(invalid)? {
        count += 1;
        let cell = row.get(count).ok_or_else(|| invalid("效果原件缺列"))?;
        let (kind, bytes) = match cell {
            SqlCell::Null => (Type::Null, None),
            SqlCell::Integer(_) => (Type::Integer, None),
            SqlCell::Real(_) => (Type::Real, None),
            SqlCell::Text(b) => (Type::Text, Some(b.as_slice())),
            SqlCell::Blob(_) => (Type::Blob, None),
        };
        if !v2::valid_storage_cell(
            &c.get::<_, String>(0).map_err(invalid)?,
            c.get(1).map_err(invalid)?,
            c.get(2).map_err(invalid)?,
            kind,
            bytes,
        ) {
            return Err(invalid(format!(
                "{} 第{}列实际类型/非空约束损坏",
                table.name(),
                count - 1
            )));
        }
    }
    if count + 1 != row.len() {
        return Err(invalid("效果原件列数不符"));
    }
    Ok(())
}

thread_local! {
    static NODE_JOURNAL: std::cell::RefCell<Option<(usize,Arc<Mutex<Journal>>)>> = const { std::cell::RefCell::new(None) };
}
pub(super) fn clear_node_journal(tx: &Transaction<'_>) {
    NODE_JOURNAL.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|(db, _)| *db == unsafe { tx.handle() } as usize)
        {
            slot.take();
        }
    });
}
/// 仅固定 TxStore 在实际 INSERT 成功或逐 tuple Noop 校验成功后调用。
pub(crate) fn note_node(tx: &Transaction<'_>, digest: [u8; 32], before: u64, inserted: bool) {
    NODE_JOURNAL.with(|slot| {
        let slot = slot.borrow();
        let Some((db, journal)) = slot.as_ref() else {
            return;
        };
        if *db != unsafe { tx.handle() } as usize {
            return;
        }
        let mut j = journal.lock().expect("同步 owner node 效果锁");
        if let Some(c) = j.coverage.as_mut() {
            let expected = u64::from(inserted);
            if tx.total_changes().checked_sub(before) != Some(expected)
                || (inserted && tx.changes() != 1)
            {
                c.exact = false;
            }
            // 按实际 INSERT 次数登记，不能按去重后的 digest 数计写入。
            match c.node_inserts.checked_add(expected) {
                Some(n) => c.node_inserts = n,
                None => c.exact = false,
            }
        }
        if j.operation != Operation::Publish {
            j.unexpected = true;
            return;
        }
        let existed = j.nodes.contains(&digest);
        if existed && !j.set_required {
            return;
        }
        if j.set_required {
            let Some(next) = j.material_epoch.checked_add(1) else {
                j.unexpected = true;
                return;
            };
            j.material_epoch = next;
        }
        if !j.reserve_one(!existed) {
            return;
        }
        j.nodes.insert(digest);
        let material_epoch = j.material_epoch;
        j.keys.push(Key {
            table: Table::Node,
            rowid: 0,
            digest,
            action: None,
            material_epoch,
            node_write: Some((before, tx.total_changes(), inserted)),
        });
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StageKind {
    ObjectBirth,
    ObjectWithdrawal,
    Witness,
    Relation,
    Observation,
}
impl StageKind {
    fn table(self) -> Table {
        match self {
            Self::ObjectBirth | Self::ObjectWithdrawal => Table::Object,
            Self::Witness => Table::Witness,
            Self::Relation => Table::Relation,
            Self::Observation => Table::Observation,
        }
    }
    fn action(self) -> rusqlite::hooks::Action {
        if self == Self::ObjectWithdrawal {
            rusqlite::hooks::Action::SQLITE_UPDATE
        } else {
            rusqlite::hooks::Action::SQLITE_INSERT
        }
    }
    fn roots(self) -> &'static [usize] {
        match self {
            Self::ObjectBirth | Self::ObjectWithdrawal => &[8, 9, 10, 11],
            Self::Witness | Self::Observation => &[5, 6],
            Self::Relation => &[5],
        }
    }
}
struct StageReceipt {
    kind: StageKind,
    start: usize,
    end: usize,
    material_before: u64,
    material_after: u64,
    rowid: i64,
    row: Vec<SqlCell>,
}
fn current_journal(tx: &Transaction<'_>) -> Result<Option<Arc<Mutex<Journal>>>, String> {
    NODE_JOURNAL.with(|slot| {
        let slot = slot.borrow();
        match slot.as_ref() {
            None => Ok(None),
            Some((db, journal)) if *db == unsafe { tx.handle() } as usize => {
                Ok(Some(journal.clone()))
            }
            Some(_) => Err(construction_state::stale("集合stage跨事务连接")),
        }
    })
}

/// 唯一实际事务材料读能力；既无通用来源参数，也没有自报来源身份接口。
/// 构造只发生在父owner的有界闭包中；不能Clone，不能带scope离开闭包。
pub(crate) struct MaterialAccess<'tx, 'work, 'db> {
    tx: &'tx Transaction<'db>,
    work: &'work WorkScope,
    budget: &'work native::Budget,
    journal: Arc<Mutex<Journal>>,
    seal: SnapshotSeal,
    initial_material_epoch: u64,
}
struct ActualRead<'a, 'db> {
    inner: crate::content_store::TxStore<'a, 'db>,
}
impl sc::NodeSource for ActualRead<'_, '_> {
    fn codec_work(&self) -> Option<&WorkScope> {
        sc::NodeSource::codec_work(&self.inner)
    }
    fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        sc::NodeSource::read(&mut self.inner, digest)
    }
    fn checked_read(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        sc::NodeSource::checked_read(&mut self.inner, digest)
    }
    fn checked_read_scoped(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        sc::NodeSource::checked_read_scoped(&mut self.inner, digest)
    }
}
impl CodecRead for ActualRead<'_, '_> {
    fn generic_source(&mut self) -> &mut dyn sc::NodeSource {
        self
    }
}
impl MaterialAccess<'_, '_, '_> {
    fn check(&self, j: &Journal) -> Result<(), String> {
        let actual = current_journal(self.tx)?
            .ok_or_else(|| construction_state::stale("实际事务已无owner日志"))?;
        let seal = j
            .set_seed
            .as_ref()
            .map(|s| s.seal)
            .or_else(|| j.set.as_ref().map(|s| s.seal));
        if !Arc::ptr_eq(&actual, &self.journal)
            || self.tx.is_autocommit()
            || j.unexpected
            || j.overflow
            || seal != Some(self.seal)
            || j.material_epoch < self.initial_material_epoch
        {
            return Err(construction_state::stale(
                "实际材料能力的owner/事务/epoch已失效",
            ));
        }
        Ok(())
    }
    fn reader(&self) -> Result<ActualRead<'_, '_>, String> {
        Ok(ActualRead {
            inner: crate::content_store::TxStore::new(self.tx)
                .map_err(invalid)?
                .with_work(Some(self.work)),
        })
    }
}

/// 只能由父owner转接prepare原有work。Some逐字借用，不创建替代scope；
/// None只从同一journal cap建立一次共享父账，后续失败不能重建洗账。
pub(super) fn with_material_access<T>(
    tx: &Transaction<'_>,
    original_work: Option<&WorkScope>,
    f: impl FnOnce(Option<&MaterialAccess<'_, '_, '_>>) -> Result<T, String>,
) -> Result<T, String> {
    let Some(journal) = current_journal(tx)? else {
        return f(None);
    };
    let (budget, seal, initial_material_epoch) = {
        let mut j = journal.lock().map_err(invalid)?;
        if !j.set_required {
            drop(j);
            return f(None);
        }
        let result = (|| {
            if j.unexpected
                || j.overflow
                || j.resources.is_some()
                || j.set.is_some()
                || !j.stages.is_empty()
                || tx.is_autocommit()
            {
                return Err(construction_state::stale("材料能力只能在本publish签发一次"));
            }
            let seal = j
                .set_seed
                .as_ref()
                .ok_or_else(|| construction_state::stale("材料能力缺owner seed"))?
                .seal;
            // 先占外父账，再分配本片子账元数据；外cap不是提前释放的临时许可。
            let outer_cap = original_work
                .map(|w| w.reserve_native(j.cap))
                .transpose()
                .map_err(invalid)?;
            let budget = native::Budget::new(j.cap, j.bytes)
                .map_err(|_| construction_state::stale("材料共享父账额度不足"))?;
            j.resources = Some(MaterialResources {
                budget: budget.clone(),
                outer_cap,
            });
            Ok((budget, seal, j.material_epoch))
        })();
        if result.is_err() {
            j.unexpected = true;
        }
        result?
    };
    let run = |scope: &WorkScope| {
        let access = MaterialAccess {
            tx,
            work: scope,
            budget: &budget,
            journal: journal.clone(),
            seal,
            initial_material_epoch,
        };
        f(Some(&access))
    };
    let result = match original_work {
        Some(scope) => run(scope),
        None => match budget.codec_work() {
            Ok(work) => match work.run(run) {
                Ok(value) => Ok(value),
                Err(WorkError::Operation(error)) => Err(error),
                Err(WorkError::Resource(_)) => {
                    Err(construction_state::stale("材料共享codec额度/工作域失效"))
                }
            },
            Err(_) => Err(construction_state::stale("材料共享codec工作域额度不足")),
        },
    };
    let result = result.and_then(|value| {
        let j = journal.lock().map_err(invalid)?;
        if j.unexpected || j.overflow || j.active_stage.is_some() || j.set.is_none() {
            Err(construction_state::stale("材料闭包未完整闭合或吞下失败"))
        } else {
            Ok(value)
        }
    });
    if result.is_err() {
        journal.lock().map_err(invalid)?.unexpected = true;
    }
    result
}

/// 完整旧来源审计成功后，从封闭能力的真实tx读取材料cut。
pub(super) fn note_source_cut(
    material: &MaterialAccess<'_, '_, '_>,
    root: sc::RefView<'_>,
) -> Result<(), String> {
    let mut j = material.journal.lock().map_err(invalid)?;
    let result = (|| {
        material.check(&j)?;
        if j.set.is_some() || j.active_stage.is_some() || !j.stages.is_empty() {
            return Err(construction_state::stale("集合cut不是首次stage前唯一切面"));
        }
        let mut source = material.reader()?;
        // 当前root参数本身不携带来源资格；在实际tx上核完整Public闭包后才允许共享跳过。
        source
            .verify_many_ref_fresh([root], sc::Mode::Public)
            .map_err(invalid)?;
        let seed = j
            .set_seed
            .take()
            .ok_or_else(|| construction_state::stale("缺owner集合seed"))?;
        let set =
            construction_set::qualify(seed, &mut source, root, j.material_epoch, material.budget)?;
        j.peak = j.peak.max(material.budget.peak());
        j.extra_bytes = sum([j.extra_bytes, set.retained_bytes()]);
        j.bytes = sum([
            Journal::layout(j.keys.capacity(), j.nodes.capacity()),
            j.extra_bytes,
        ]);
        if j.bytes > j.cap {
            return Err(construction_state::stale("集合cut保留账超过额度"));
        }
        j.set = Some(set);
        Ok(())
    })();
    if result.is_err() {
        j.unexpected = true;
    }
    result
}

/// Err/Drop 均毒化当前 actual-effects 流；不能忽略某个stage失败再继续发成功收据。
pub(crate) struct StageGuard {
    journal: Option<Arc<Mutex<Journal>>>,
    connection: usize,
    kind: StageKind,
    start: usize,
    material_before: u64,
    closed: bool,
}
impl Drop for StageGuard {
    fn drop(&mut self) {
        if !self.closed {
            if let Some(journal) = &self.journal {
                if let Ok(mut j) = journal.lock() {
                    j.unexpected = true;
                }
            }
        }
    }
}
pub(super) fn begin_stage(tx: &Transaction<'_>, kind: StageKind) -> Result<StageGuard, String> {
    let mut guard = StageGuard {
        journal: None,
        connection: unsafe { tx.handle() } as usize,
        kind,
        start: 0,
        material_before: 0,
        closed: false,
    };
    let Some(journal) = current_journal(tx)? else {
        return Ok(guard);
    };
    {
        let mut j = journal.lock().map_err(invalid)?;
        if !j.set_required {
            return Ok(guard);
        }
        if j.overflow
            || j.unexpected
            || j.set.is_none()
            || j.active_stage.is_some()
            || j.stages.last().is_some_and(|stage| stage.kind > kind)
        {
            j.unexpected = true;
            return Err(construction_state::stale("stage缺材料cut、重入或顺序倒退"));
        }
        guard.start = j.keys.len();
        guard.material_before = j.material_epoch;
        j.active_stage = Some(guard.start);
    }
    guard.journal = Some(journal);
    Ok(guard)
}
// 实际ExactRows先析构再归还预算；成功只将被move的行费用留给Journal。
struct StageOriginals<'a> {
    value: Option<ExactRows>,
    budget: &'a native::Budget,
    bytes: usize,
}
impl Drop for StageOriginals<'_> {
    fn drop(&mut self) {
        drop(self.value.take());
        self.budget.release(self.bytes);
    }
}
impl StageOriginals<'_> {
    fn row(&self) -> &[SqlCell] {
        &self.value.as_ref().unwrap().rows[0]
    }
    fn into_row(mut self) -> Vec<SqlCell> {
        let row = self.value.as_mut().unwrap().rows.pop().unwrap();
        self.bytes -= row.heap_bytes();
        row
    }
}
fn consumed_bytes(capacity: usize) -> usize {
    super::super::audit_retention::hash_slots::<(Table, i64), ()>(capacity)
}
fn reserve_consumed(j: &mut Journal, budget: &native::Budget) -> Result<(), String> {
    if j.consumed.len() < j.consumed.capacity() {
        return Ok(());
    }
    let next = j
        .consumed
        .capacity()
        .checked_mul(2)
        .ok_or_else(|| construction_state::stale("stage身份索引容量溢出"))?
        .max(4);
    let cap = native::hash_capacity(next)
        .map_err(|_| construction_state::stale("stage身份索引容量溢出"))?;
    let permit = budget
        .reserve(consumed_bytes(cap))
        .map_err(|_| construction_state::stale("stage身份索引额度不足"))?;
    let old = consumed_bytes(j.consumed.capacity());
    j.consumed
        .try_reserve(next - j.consumed.len())
        .map_err(|_| construction_state::stale("stage身份索引分配失败"))?;
    permit.commit(old, consumed_bytes(j.consumed.capacity()));
    Ok(())
}
impl StageGuard {
    pub(crate) fn finish(
        mut self,
        material: Option<&MaterialAccess<'_, '_, '_>>,
    ) -> Result<(), String> {
        let Some(journal) = self.journal.as_ref().cloned() else {
            self.closed = true;
            return Ok(());
        };
        let mut j = journal.lock().map_err(invalid)?;
        let result = (|| {
            let material =
                material.ok_or_else(|| construction_state::stale("strict stage缺实际材料能力"))?;
            if !Arc::ptr_eq(&journal, &material.journal) {
                return Err(construction_state::stale("stage与材料能力不属同owner日志"));
            }
            material.check(&j)?;
            let tx = material.tx;
            if self.connection != unsafe { tx.handle() } as usize
                || j.active_stage != Some(self.start)
                || j.material_epoch < self.material_before
            {
                return Err(construction_state::stale("stage失效或跨连接"));
            }
            let mut rows = j.keys[self.start..]
                .iter()
                .filter(|key| key.table != Table::Node);
            let row = rows
                .next()
                .ok_or_else(|| construction_state::stale("stage缺实际SQL行"))?;
            if rows.next().is_some()
                || row.table != self.kind.table()
                || row.action != Some(self.kind.action())
            {
                return Err(construction_state::stale("stage SQL并非唯一预期表/动作"));
            }
            let rowid = row.rowid;
            let identity = (self.kind.table(), rowid);
            if j.consumed.contains(&identity) {
                return Err(construction_state::stale("业务原件已被前一stage消费"));
            }
            let budget = material.budget;
            budget
                .vec_grow(&mut j.stages, 1)
                .map_err(|_| construction_state::stale("stage容器额度不足"))?;
            reserve_consumed(&mut j, budget)?;
            let remaining = j
                .cap
                .checked_sub(budget.live())
                .ok_or_else(|| construction_state::stale("stage原件额度不足"))?;
            // 先占完整可用上限；ExactRows内部按同一remaining限制每次新旧块共存。
            let read_permit = budget
                .reserve(remaining)
                .map_err(|_| construction_state::stale("stage原件预留失败"))?;
            let (originals, _) = ExactRows::read_optional_bounded_params(
                tx,
                self.kind.table().row_sql(),
                [rowid],
                remaining,
            )?
            .ok_or_else(|| construction_state::stale("stage原件读取额度不足"))?;
            let bytes = originals.heap_bytes();
            read_permit.commit(0, bytes);
            let originals = StageOriginals {
                value: Some(originals),
                budget,
                bytes,
            };
            if originals.value.as_ref().unwrap().rows.len() != 1 {
                return Err(construction_state::stale("stage当前行缺失/重复"));
            }
            verify_storage(tx, self.kind.table(), originals.row())?;
            let mut source = material.reader()?;
            // 只从保存的actual tx回读根闭包；外来memory或未来stage均无补签入口。
            for &column in self.kind.roots() {
                let digest: [u8; 32] = match &originals.row()[column] {
                    SqlCell::Blob(value) => value.as_slice().try_into().map_err(invalid)?,
                    _ => return Err(construction_state::stale("stage根不是BLOB")),
                };
                let root = sc::FixedRoot::new(&digest);
                let reference = source.content_ref_view(root.as_str()).map_err(invalid)?;
                source
                    .verify_many_ref_fresh([reference.view()], sc::Mode::Public)
                    .map_err(invalid)?;
            }
            j.peak = j.peak.max(budget.peak());
            let row = originals.into_row();
            j.stage_rows_bytes = sum([j.stage_rows_bytes, row.heap_bytes()]);
            let end = j.keys.len();
            let material_after = j.material_epoch;
            j.consumed.insert(identity);
            j.stages.push(StageReceipt {
                kind: self.kind,
                start: self.start,
                end,
                material_before: self.material_before,
                material_after,
                rowid,
                row,
            });
            j.extra_bytes = sum([
                j.set.as_ref().map_or(0, |set| set.retained_bytes()),
                vec_bytes::<StageReceipt>(j.stages.capacity()),
                j.stage_rows_bytes,
                consumed_bytes(j.consumed.capacity()),
            ]);
            j.bytes = sum([
                Journal::layout(j.keys.capacity(), j.nodes.capacity()),
                j.extra_bytes,
            ]);
            if j.bytes > j.cap {
                return Err(construction_state::stale("stage实际保留账超过额度"));
            }
            j.active_stage = None;
            Ok(())
        })();
        if result.is_err() {
            j.unexpected = true;
        }
        drop(j);
        if result.is_ok() {
            self.closed = true;
        }
        result
    }
}
fn validate_stage_coverage(j: &Journal) -> Result<(), String> {
    let set = j
        .set
        .as_ref()
        .ok_or_else(|| construction_state::stale("publish缺完整材料cut"))?;
    if j.active_stage.is_some() || j.consumed.len() != j.stages.len() {
        return Err(construction_state::stale("存在未闭合或重复消费stage"));
    }
    let mut cursor = 0usize;
    let mut previous_material = set.material_epoch;
    let mut node_epoch = 0u64;
    for (ordinal, key) in j.keys.iter().enumerate() {
        if key.table == Table::Node {
            let Some((before, after, inserted)) = key.node_write else {
                return Err(construction_state::stale("node缺实际put收据"));
            };
            node_epoch = node_epoch
                .checked_add(1)
                .ok_or_else(|| construction_state::stale("node事件序耗尽"))?;
            if key.material_epoch != node_epoch
                || after.checked_sub(before) != Some(u64::from(inserted))
            {
                return Err(construction_state::stale("node多次事件序/实际写数不符"));
            }
            continue;
        }
        if matches!(
            key.table,
            Table::Object | Table::Witness | Table::Relation | Table::Observation
        ) {
            while cursor < j.stages.len() && ordinal >= j.stages[cursor].end {
                cursor += 1;
            }
            let stage = j
                .stages
                .get(cursor)
                .ok_or_else(|| construction_state::stale("业务SQL不属于stage"))?;
            if ordinal < stage.start || key.table != stage.kind.table() || key.rowid != stage.rowid
            {
                return Err(construction_state::stale("stage遗漏或重复业务SQL"));
            }
        }
    }
    for stage in &j.stages {
        if stage.material_before < previous_material || stage.material_after < stage.material_before
        {
            return Err(construction_state::stale("stage材料epoch倒退"));
        }
        previous_material = stage.material_after;
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn native_journal_only_reserves_the_container_that_grows() {
    let inline = std::mem::size_of::<Journal>() + 128;
    let mut j = Journal {
        keys: Vec::new(),
        nodes: Default::default(),
        operation: Operation::Publish,
        overflow: false,
        unexpected: false,
        cap: usize::MAX,
        bytes: inline,
        peak: inline,
        coverage: None,
        set_required: false,
        set_seed: None,
        set: None,
        stages: Vec::new(),
        active_stage: None,
        material_epoch: 0,
        extra_bytes: 0,
        consumed: Default::default(),
        stage_rows_bytes: 0,
        resources: None,
    };
    assert!(j.reserve_one(true));
    let node_cap = j.nodes.capacity();
    let key_cap = j.keys.capacity();
    for rowid in 0..key_cap {
        j.keys.push(Key {
            table: Table::Meta,
            rowid: rowid as i64,
            digest: [0; 32],
            action: None,
            material_epoch: 0,
            node_write: None,
        });
    }
    let before = j.bytes;
    let request = vec_bytes::<Key>(key_cap * 2);
    j.cap = before + request;
    assert!(j.reserve_one(false), "未增长nodes不得再次占新块许可");
    assert_eq!(j.nodes.capacity(), node_cap);
    assert_eq!(j.peak, before + request);
    assert_eq!(
        j.bytes,
        Journal::layout(j.keys.capacity(), j.nodes.capacity())
    );
    assert!(j.bytes <= j.cap);
}

#[cfg(test)]
mod construction_coverage_tests {
    use super::*;
    use newchan_rust::s_content::{self as sc, NodeSink};

    fn database() -> Connection {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL); INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
        let tx = db.transaction().unwrap();
        crate::content_store::init_schema(&tx).unwrap();
        tx.commit().unwrap();
        db
    }
    fn put(tx: &Transaction<'_>, byte: u8) {
        let body = [byte];
        crate::content_store::TxStore::new(tx)
            .unwrap()
            .put(
                sc::frame(sc::tag::BYTES_CHUNK, &body),
                sc::tag::BYTES_CHUNK,
                &body,
            )
            .unwrap();
    }

    #[test]
    fn owner_effects_node_insert_and_repeated_noop_use_actual_write_counts() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let start = tx.total_changes();
        let recording = Recording::start(&tx, Operation::Publish, 1 << 20, true)
            .unwrap()
            .unwrap();
        put(&tx, 1);
        put(&tx, 1);
        put(&tx, 2);
        assert_eq!(tx.total_changes() - start, 2);
        let effects = recording.finish(&tx, &mut [], 1 << 20).unwrap().unwrap();
        assert!(effects.has_exact_coverage(tx.total_changes()));
        assert_eq!(effects.rows(Table::Node).count(), 2);
        assert!(!effects.has_exact_coverage(tx.total_changes() + 1));
    }

    #[test]
    fn owner_effects_unhooked_without_rowid_write_and_trigger_side_effect_reject() {
        for trigger in [false, true] {
            let mut db = database();
            let tx = db.transaction().unwrap();
            tx.execute_batch("CREATE TABLE side(k INTEGER PRIMARY KEY,v BLOB) WITHOUT ROWID")
                .unwrap();
            if trigger {
                tx.execute_batch("CREATE TRIGGER side_effect AFTER INSERT ON s_content_nodes BEGIN INSERT INTO side VALUES(1,x'01'); END").unwrap();
            }
            let recording = Recording::start(&tx, Operation::Publish, 1 << 20, true)
                .unwrap()
                .unwrap();
            put(&tx, 1);
            if !trigger {
                tx.execute("INSERT INTO side VALUES(1,x'01')", []).unwrap();
            }
            assert!(recording.finish(&tx, &mut [], 1 << 20).is_err());
        }
    }

    #[test]
    fn owner_effects_strict_schema_rejects_rowid_node_substitute() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        tx.execute_batch("DROP TABLE s_content_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER NOT NULL,body BLOB NOT NULL)").unwrap();
        assert!(Recording::start(&tx, Operation::Publish, 1 << 20, true).is_err());
        // 未选择strict路径时仍保持既有可选记录语义。
        let old = Recording::start(&tx, Operation::Publish, 1 << 20, false)
            .unwrap()
            .unwrap();
        assert!(old.finish(&tx, &mut [], 1 << 20).unwrap().is_some());
    }

    #[test]
    fn owner_effects_absent_budget_never_yields_coverage_receipt() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        assert!(Recording::start(&tx, Operation::Publish, 0, true)
            .unwrap()
            .is_none());
        let recording = Recording::start(&tx, Operation::Publish, 1 << 20, true)
            .unwrap()
            .unwrap();
        put(&tx, 1);
        assert!(recording.finish(&tx, &mut [], 0).unwrap().is_none());
    }
    fn set_recording(tx: &Transaction<'_>) -> (Recording, sc::ContentRef) {
        let fixture = super::super::construction_state::RawAdvanceFixture::new();
        let events = [serde_json::json!({"identity_key":"raw","revision":1,"source_coord":"0"})];
        let token = fixture.bind((&events).into(), 0, &[]).unwrap();
        let raw = token.current_binding();
        let seed = construction_set::Seed::new(
            raw,
            SnapshotSeal {
                life: raw.owner_life(),
                transaction_id: 1,
                write_sequence: 0,
                data_version: 1,
                schema_version: 1,
                domain: [0, 1, 1, 0, 0, 1],
            },
            None,
            None,
        )
        .unwrap();
        let recording = Recording::start(tx, Operation::Publish, 16 << 20, true)
            .unwrap()
            .unwrap();
        recording.bind_set(seed).unwrap();
        let mut store = crate::content_store::TxStore::new(tx).unwrap();
        let state = sc::write_value(
            &mut store,
            &sc::Value::Object(vec![
                ("objects".into(), sc::Value::Array(vec![])),
                ("witnesses".into(), sc::Value::Array(vec![])),
                ("observations".into(), sc::Value::Array(vec![])),
            ]),
        )
        .unwrap()
        .content_ref();
        (recording, state)
    }
    #[test]
    fn owner_set_ordered_node_receipts_keep_repeated_noop_and_unreachable_nodes() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        with_material_access(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            let before = recording.journal.lock().unwrap().keys.len();
            put(&tx, 45);
            put(&tx, 45);
            put(&tx, 46);
            let j = recording.journal.lock().unwrap();
            let tail = &j.keys[before..];
            assert_eq!(tail.len(), 3, "不可因摘要相同丢Noop occurrence");
            assert_eq!(tail[0].digest, tail[1].digest);
            assert!(tail[0].node_write.unwrap().2);
            assert!(!tail[1].node_write.unwrap().2);
            assert!(tail[1].material_epoch > tail[0].material_epoch);
            Ok(())
        })
        .unwrap();
        let effects = recording.finish(&tx, &mut [], 16 << 20).unwrap().unwrap();
        assert!(effects.set.is_some());
        assert!(effects.has_exact_coverage(tx.total_changes()));
    }
    #[test]
    fn owner_set_future_stage_cannot_repair_missing_current_original() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        tx.execute_batch(crate::v3::aux_index::SCHEMA).unwrap();
        let (recording, state) = set_recording(&tx);
        let mut memory = sc::MemoryNodes::default();
        let missing =
            sc::write_value(&mut memory, &sc::Value::String("only-in-next-stage".into())).unwrap();
        sc::verify(&mut memory, &missing.content_ref()).unwrap();
        let (tag, body) = memory.get(&missing.digest).unwrap();
        let mut store = crate::content_store::TxStore::new(&tx).unwrap();
        store.put(missing.digest, tag, body).unwrap();
        let result = with_material_access(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            let stage = begin_stage(&tx, StageKind::Observation)?;
            tx.execute(
                "INSERT INTO s_content_observations VALUES('ob','kind',NULL,'batch',?1,?1,1)",
                [&missing.digest[..]],
            )
            .unwrap();
            // memory在同一时点完整，但正式finish没有可接memory/source的参数。
            assert!(stage.finish(Some(material)).is_err());
            for (digest, tag, body) in memory.iter() {
                store.put(*digest, tag, body).unwrap();
            }
            assert!(begin_stage(&tx, StageKind::Observation).is_err());
            Ok(()) // 外层仍须识别poison，不能吞掉失败后成功。
        });
        assert!(result.is_err());
        assert!(with_material_access(&tx, None, |_| Ok(())).is_err());
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
    #[test]
    fn owner_set_stage_drop_foreign_connection_and_omitted_business_row_reject() {
        for scenario in ["drop", "foreign", "omitted"] {
            let mut db = database();
            let tx = db.transaction().unwrap();
            tx.execute_batch(crate::v3::aux_index::SCHEMA).unwrap();
            let (recording, state) = set_recording(&tx);
            let result = with_material_access(&tx, None, |material| {
                let material = material.unwrap();
                note_source_cut(material, (&state).into())?;
                match scenario {
                    "drop" => drop(begin_stage(&tx, StageKind::Witness).unwrap()),
                    "foreign" => {
                        let mut other = database();
                        let other_tx = other.transaction().unwrap();
                        assert!(begin_stage(&other_tx, StageKind::Witness).is_err());
                        assert!(with_material_access(&other_tx, None, |_| Ok(())).is_err());
                        drop(begin_stage(&tx, StageKind::Witness).unwrap());
                    }
                    "omitted" => {
                        tx.execute("INSERT INTO s_content_relations VALUES('r','s','t','o',zeroblob(32),1)", []).unwrap();
                    }
                    _ => unreachable!(),
                }
                Ok(())
            });
            if scenario != "omitted" {
                assert!(result.is_err());
            } else {
                result.unwrap();
            }
            assert!(
                recording.finish(&tx, &mut [], 16 << 20).is_err(),
                "{scenario}"
            );
        }
    }
    // 只验seal实际行单次消费；这些最小行不冒称已经通过对象业务全审。
    fn withdrawal_fixture(tx: &Transaction<'_>) -> Vec<ExactRows> {
        tx.execute_batch(crate::v3::object_index::SCHEMA).unwrap();
        let mut store = crate::content_store::TxStore::new(tx).unwrap();
        let root = sc::write_value(&mut store, &sc::Value::String("existing".into())).unwrap();
        for id in ["first", "second"] {
            tx.execute("INSERT INTO s_content_objects VALUES(?1,1,'kind','fact',NULL,NULL,NULL,?2,?2,?2,?2,'batch',1,'cut-1',1,NULL,NULL,NULL)", rusqlite::params![id, root.digest.as_slice()]).unwrap();
        }
        vec![
            ExactRows::read_optional_bounded(tx, Table::Object.original_sql(), 1 << 20)
                .unwrap()
                .unwrap()
                .0,
        ]
    }
    fn withdraw(tx: &Transaction<'_>, id: &str) {
        // 故意绕开正常stage的withdrawn IS NULL防线，直接测seal。
        assert_eq!(tx.execute("UPDATE s_content_objects SET withdrawn_generation=2,withdrawal_reason='replaced',superseded_by='new-id' WHERE object_id=?1", [id]).unwrap(), 1);
    }
    #[test]
    fn owner_set_duplicate_withdrawal_same_final_row_poisons_ordered_seal() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let mut originals = withdrawal_fixture(&tx);
        let (recording, state) = set_recording(&tx);
        let result = with_material_access(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            let first = begin_stage(&tx, StageKind::ObjectWithdrawal)?;
            withdraw(&tx, "first");
            first.finish(Some(material))?;
            let duplicate = begin_stage(&tx, StageKind::ObjectWithdrawal)?;
            withdraw(&tx, "first");
            assert!(duplicate.finish(Some(material)).is_err());
            assert!(begin_stage(&tx, StageKind::ObjectWithdrawal).is_err());
            Ok(())
        });
        assert!(result.is_err());
        assert!(recording.finish(&tx, &mut originals, 16 << 20).is_err());
    }
    #[test]
    fn owner_set_distinct_withdrawals_each_consume_one_actual_row() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let mut originals = withdrawal_fixture(&tx);
        let (recording, state) = set_recording(&tx);
        with_material_access(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            for id in ["first", "second"] {
                let stage = begin_stage(&tx, StageKind::ObjectWithdrawal)?;
                withdraw(&tx, id);
                stage.finish(Some(material))?;
            }
            let j = recording.journal.lock().unwrap();
            assert_eq!(j.stages.len(), 2);
            assert_eq!(j.consumed.len(), 2);
            Ok(())
        })
        .unwrap();
        let effects = recording
            .finish(&tx, &mut originals, 16 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(effects.rows(Table::Object).count(), 2);
        assert!(effects.has_exact_coverage(tx.total_changes()));
    }
    #[test]
    fn owner_set_source_cut_rejects_foreign_only_closure_and_cannot_reissue() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, _) = set_recording(&tx);
        let mut memory = sc::MemoryNodes::default();
        let foreign = sc::write_value(
            &mut memory,
            &sc::Value::Object(vec![
                ("objects".into(), sc::Value::Array(vec![])),
                (
                    "witnesses".into(),
                    sc::Value::Array(vec![sc::Value::String("foreign-only".into())]),
                ),
                ("observations".into(), sc::Value::Array(vec![])),
            ]),
        )
        .unwrap();
        sc::verify(&mut memory, &foreign.content_ref()).unwrap();
        let (tag, body) = memory.get(&foreign.digest).unwrap();
        let mut store = crate::content_store::TxStore::new(&tx).unwrap();
        store.put(foreign.digest, tag, body).unwrap();
        assert!(with_material_access(&tx, None, |m| note_source_cut(
            m.unwrap(),
            (&foreign.content_ref()).into()
        ))
        .is_err());
        for (digest, tag, body) in memory.iter() {
            store.put(*digest, tag, body).unwrap();
        }
        assert!(with_material_access(&tx, None, |_| Ok(())).is_err());
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
    #[test]
    fn owner_set_none_scope_shares_journal_cap_and_failure_cannot_reset() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let result = with_material_access(&tx, None, |m| {
            let m = m.unwrap();
            note_source_cut(m, (&state).into())?;
            let cap = recording.journal.lock().unwrap().cap;
            assert!(m.budget.live() >= recording.journal.lock().unwrap().bytes);
            let occupied = m.budget.reserve(cap - m.budget.live()).unwrap();
            let error = m.reader()?.content_ref_view(&state.root);
            assert!(
                error.is_err(),
                "已有journal/容器占满同cap时codec不得再借另一份cap"
            );
            drop(occupied);
            Ok(())
        });
        assert!(result.is_err());
        assert!(with_material_access(&tx, None, |_| Ok(())).is_err());
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
    #[test]
    fn owner_set_exact_work_cap_lives_through_effects_then_releases_only_its_charge() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let cap = recording.journal.lock().unwrap().cap;
        let parent = native::Budget::new(64 << 20, 0).unwrap();
        let work = parent.codec_work().unwrap();
        let effects = work
            .run(|scope| {
                with_material_access(&tx, Some(scope), |m| {
                    let m = m.unwrap();
                    assert!(std::ptr::eq(m.work, scope), "必须是原work同对象");
                    note_source_cut(m, (&state).into())
                })?;
                assert!(
                    parent.native_observation().0 >= cap,
                    "离开材料闭包不能提前返还cap"
                );
                recording
                    .finish(&tx, &mut [], 16 << 20)?
                    .ok_or_else(|| "缺effects".to_string())
            })
            .unwrap();
        let before = parent.live();
        let native_before = parent.native_observation().0;
        assert!(effects.set.is_some());
        assert!(native_before > cap, "原codec retain仍在同一个父账中");
        drop(effects);
        assert_eq!(parent.live(), before - cap);
        assert_eq!(parent.native_observation().0, native_before - cap);
        assert!(
            parent.native_observation().0 > 0,
            "不得手工扣掉原codec保守retain"
        );
        drop(parent); // 剩余既有保守账随父账整个生命周期结束。
    }
    #[test]
    fn owner_set_failed_source_drops_new_cap_without_erasing_existing_codec_retain() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let cap = recording.journal.lock().unwrap().cap;
        let parent = native::Budget::new(64 << 20, 0).unwrap();
        let result = parent.codec_work().unwrap().run(|scope| {
            with_material_access(&tx, Some(scope), |m| {
                note_source_cut(m.unwrap(), (&state).into())?;
                Err::<(), _>("TestOnlyAfterCutRejected".to_string())
            })
        });
        assert!(result.is_err());
        let before = parent.live();
        let native_before = parent.native_observation().0;
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
        assert_eq!(parent.live(), before - cap, "错误Drop归还新增cap全部余额");
        assert_eq!(parent.native_observation().0, native_before - cap);
        assert!(
            parent.native_observation().0 > 0,
            "原codec retain仍由原父账持有"
        );
    }
    #[test]
    fn owner_set_actual_transaction_progress_cancellation_is_preserved() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        tx.progress_handler(1, Some(|| true));
        assert!(
            with_material_access(&tx, None, |m| note_source_cut(m.unwrap(), (&state).into()))
                .is_err()
        );
        tx.progress_handler(0, None::<fn() -> bool>);
        assert!(
            with_material_access(&tx, None, |_| Ok(())).is_err(),
            "取消后不能重建成功scope"
        );
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
}
