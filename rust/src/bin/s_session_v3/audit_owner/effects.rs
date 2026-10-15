//! #1394 固定写口的实际 SQLite 效果。hook 只记录表/rowid；原件必须在锁内回读。
//! 每次扩容先检查旧新容量共存账；预算不足撤销可选候选，不改变业务写入结果。
use super::super::audit_retention::{sum, vec_bytes, Retained};
use super::super::request_audit::{ExactRows, SqlCell};
use super::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
        if (grow_keys && self.keys.try_reserve_exact(keys - self.keys.len()).is_err())
            || (grow_nodes && self.nodes.try_reserve(nodes - self.nodes.len()).is_err())
        {
            self.overflow = true;
            return false;
        }
        self.bytes = Self::layout(self.keys.capacity(), self.nodes.capacity());
        true
    }
}
pub(super) struct Recording {
    journal: Arc<Mutex<Journal>>,
}
impl Recording {
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
            j.keys.push(Key {
                table,
                rowid,
                digest: [0; 32],
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
        Ok(Some(Effects {
            changes,
            operation: journal.operation,
            peak,
            // journal及读行头已真实Drop；旧载荷仍由原件账全额承接，不在这里重复计入。
            bytes: used - journal.bytes,
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
        if j.nodes.contains(&digest) {
            return;
        }
        if !j.reserve_one(true) {
            return;
        }
        j.nodes.insert(digest);
        j.keys.push(Key {
            table: Table::Node,
            rowid: 0,
            digest,
        });
    });
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
    };
    assert!(j.reserve_one(true));
    let node_cap = j.nodes.capacity();
    let key_cap = j.keys.capacity();
    for rowid in 0..key_cap {
        j.keys.push(Key {
            table: Table::Meta,
            rowid: rowid as i64,
            digest: [0; 32],
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
}
