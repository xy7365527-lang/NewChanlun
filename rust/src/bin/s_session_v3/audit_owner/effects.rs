//! #1394 固定写口的实际 SQLite 效果。hook 只记录表/rowid；原件必须在锁内回读。
//! 每次扩容先检查旧新容量共存账；预算不足撤销可选候选，不改变业务写入结果。
use super::super::audit_retention::{sum, vec_bytes, Retained};
use super::super::audit_store::CodecRead;
use super::super::request_audit::{ExactRows, SqlCell};
use super::*;
use newchan_rust::s_content::ContextError as PublishError;
use newchan_rust::s_content::{
    self as sc,
    capture_allocation::scratch::{NativeReservation, WorkError, WorkScope},
};
use std::sync::{Arc, Mutex, Weak};

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
/// #1488 0035：逐收据事件（8B）。key 表按“同一 publication journal + 节点摘要”合并，
/// 每条实际收据（行写或 node put，含 Noop）仍按发生序各占一个事件：
/// 高 32 位 = key 下标；位 1..=31 = 该次 put 的 after−before（行事件为 0，溢出存哨兵）；位 0 = inserted。
/// node 事件的 material_epoch 即其在 node 事件中的序数（稠密，由 validate 闭合核对）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
struct Receipt(u64);
impl Receipt {
    const DELTA_LIMIT: u64 = (1 << 31) - 1;
    fn row(key: usize) -> Option<Self> {
        Some(Self(u64::from(u32::try_from(key).ok()?) << 32))
    }
    fn node(key: usize, before: u64, after: u64, inserted: bool) -> Option<Self> {
        let delta = after
            .checked_sub(before)
            .filter(|d| *d < Self::DELTA_LIMIT)
            .unwrap_or(Self::DELTA_LIMIT);
        Some(Self(
            u64::from(u32::try_from(key).ok()?) << 32 | delta << 1 | u64::from(inserted),
        ))
    }
    fn key(self) -> usize {
        (self.0 >> 32) as usize
    }
    fn inserted(self) -> bool {
        self.0 & 1 == 1
    }
    /// 该次实际写数；溢出哨兵返回 None。
    fn write_delta(self) -> Option<u64> {
        Some((self.0 >> 1) & Self::DELTA_LIMIT).filter(|d| *d != Self::DELTA_LIMIT)
    }
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
    // 摘要 → 首次 occurrence 的 key 下标（#1488 0035 合并 key）。
    nodes: std::collections::HashMap<[u8; 32], u32>,
    // 仅 set_required 时逐收据登记；stage start/end 与 ordered_events 以它为坐标。
    events: Vec<Receipt>,
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
    source_binding: Option<SourceCutBinding>,
    stages: Vec<StageReceipt>,
    active_stage: Option<usize>,
    material_epoch: u64,
    extra_bytes: usize,
    consumed: std::collections::HashSet<(Table, i64)>,
    stage_rows_bytes: usize,
    // 所有载荷字段在resources之前：错误Drop先析构载荷，再释放外层cap许可。
    resources: Option<MaterialResources>,
    material_issued: bool,
}
struct MaterialResources {
    parent_limit: usize,
    publication: Option<([u64; 3], usize)>,
    journal_owned_bytes: usize,
    shared_parent: bool,
    journal_charge: native::Permit,
    budget: native::Budget,
    // 子账及其元数据先结束，最后返还外父账cap；不复制逐容器许可。
    outer_cap: Option<NativeReservation>,
}
impl Journal {
    /// 只在失败出口观测；不增 Journal 字段、不改尺寸/预算，不清除首错或重试。
    #[cfg(test)]
    fn observe_failure(
        &self,
        stage: &'static str,
        requested: Option<usize>,
        resource: Option<sc::capture_allocation::scratch::ScratchFailure>,
        autocommit: Option<bool>,
        scope: Option<sc::capture_allocation::scratch::WorkSnapshot>,
    ) {
        use std::io::Write as _;
        let line = json!({
            "diagnostic":"NativeOwnerMaterialFailure",
            "stage":stage,
            "resource":resource.map(|e| format!("{e:?}")),
            "requested_bytes":requested,
            "operation":format!("{:?}", self.operation),
            "owner_seal":self.set_seed.as_ref().map(|seed| json!({
                "life":seed.seal.life,"transaction":seed.seal.transaction_id,
                "write_sequence":seed.seal.write_sequence})),
            "cap":self.cap,"bytes":self.bytes,"peak":self.peak,
            "keys":self.keys.len(),"nodes":self.nodes.len(),"events":self.events.len(),
            "material_epoch":self.material_epoch,
            "flags":{"unexpected":self.unexpected,"overflow":self.overflow,
                "resources":self.resources.is_some(),"set":self.set.is_some(),
                "stages":!self.stages.is_empty(),"autocommit":autocommit},
            "parent":self.resources.as_ref().map(|r| json!({
                "live":r.budget.live(),"peak":r.budget.peak(),
                "scratch":r.budget.scratch_observation(),
                "native":r.budget.native_observation(),"outer_cap":r.outer_cap.is_some()})),
            "scope":scope.map(|s| json!({"live":s.live,"peak":s.peak,"attempts":s.attempts})),
        });
        let _ = writeln!(std::io::stderr().lock(), "{line}");
    }

    fn layout(keys: usize, node_capacity: usize) -> usize {
        sum([
            vec_bytes::<Key>(keys),
            node_slots(node_capacity),
            std::mem::size_of::<Self>(),
            128,
        ])
    }
    /// keys/nodes/inline 之外再计逐收据事件容器。
    fn container_bytes(&self) -> usize {
        sum([
            Self::layout(self.keys.capacity(), self.nodes.capacity()),
            vec_bytes::<Receipt>(self.events.capacity()),
        ])
    }
    fn reserve_one(&mut self, node: bool) -> bool {
        self.reserve_receipt(true, node, false)
    }
    /// #1488 0035：一条收据至多让 keys/nodes/events 各增长一次；只为实际增长的容器预留新块。
    fn reserve_receipt(&mut self, key: bool, node: bool, event: bool) -> bool {
        if self.overflow {
            return false;
        }
        let grow_keys = key && self.keys.len() == self.keys.capacity();
        let grow_nodes = node && self.nodes.len() == self.nodes.capacity();
        let grow_events = event && self.events.len() == self.events.capacity();
        if !grow_keys && !grow_nodes && !grow_events {
            return true;
        }
        let events = if grow_events {
            self.events.capacity().saturating_mul(2).max(32)
        } else {
            self.events.capacity()
        };
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
                    #[cfg(test)]
                    self.observe_failure("journal-hash-capacity", None, None, None, None);
                    return false;
                }
            }
        } else {
            0
        };
        let new_blocks = sum([
            if grow_keys { vec_bytes::<Key>(keys) } else { 0 },
            if grow_nodes {
                node_slots(node_capacity)
            } else {
                0
            },
            if grow_events {
                vec_bytes::<Receipt>(events)
            } else {
                0
            },
        ]);
        let coexist = sum([self.bytes, new_blocks]);
        if coexist > self.cap {
            self.overflow = true;
            #[cfg(test)]
            self.observe_failure("journal-coexist-cap", Some(new_blocks), None, None, None);
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
                #[cfg(test)]
                self.observe_failure("journal-parent-reserve", Some(new_blocks), None, None, None);
                return false;
            }
        };
        let old_keys = self.keys.capacity();
        let old_nodes = self.nodes.capacity();
        let old_events = self.events.capacity();
        let succeeded = (!grow_keys || self.keys.try_reserve_exact(keys - self.keys.len()).is_ok())
            && (!grow_nodes || self.nodes.try_reserve(nodes - self.nodes.len()).is_ok())
            && (!grow_events
                || self
                    .events
                    .try_reserve_exact(events - self.events.len())
                    .is_ok());
        if let Some(permit) = permit {
            // 包括后一个容器分配失败的出口；先前实际新块不能提前返还许可。
            let changed_keys = old_keys != self.keys.capacity();
            let changed_nodes = old_nodes != self.nodes.capacity();
            let changed_events = old_events != self.events.capacity();
            let block = |k, n, e| {
                sum([
                    if changed_keys { vec_bytes::<Key>(k) } else { 0 },
                    if changed_nodes { node_slots(n) } else { 0 },
                    if changed_events {
                        vec_bytes::<Receipt>(e)
                    } else {
                        0
                    },
                ])
            };
            let old = block(old_keys, old_nodes, old_events);
            let actual = block(
                self.keys.capacity(),
                self.nodes.capacity(),
                self.events.capacity(),
            );
            self.resources
                .as_mut()
                .unwrap()
                .journal_charge
                .replace_part(old, permit.finish(actual))
                .expect("journal同域许可");
            if let Some(resources) = self.resources.as_mut().filter(|r| r.shared_parent) {
                resources.journal_owned_bytes = resources
                    .journal_owned_bytes
                    .checked_sub(old)
                    .and_then(|n| n.checked_add(actual))
                    .expect("journal独立保留账");
            }
        }
        self.bytes = sum([self.container_bytes(), self.extra_bytes]);
        self.overflow |= !succeeded;
        #[cfg(test)]
        if !succeeded {
            self.observe_failure(
                "journal-container-allocation",
                Some(new_blocks),
                None,
                None,
                None,
            );
        }
        succeeded
    }
}
fn node_slots(capacity: usize) -> usize {
    super::super::audit_retention::hash_slots::<[u8; 32], u32>(capacity)
}
pub(super) struct Recording {
    journal: Arc<Mutex<Journal>>,
}
impl Recording {
    /// #1488 P2-2：遗留测试 fixture——journal 已分配后才按 j.bytes 从父预留，属事后同额补签；
    /// 生产改走 start_on（许可先于任何 journal 分配），本入口仅测试编译。
    #[cfg(test)]
    pub(super) fn attach_parent(
        &self,
        budget: &native::Budget,
        parent_limit: usize,
    ) -> Result<(), String> {
        let mut j = self.journal.lock().map_err(invalid)?;
        // 无父 start 已带私有子账；fixture 附接替换它（旧私有许可随之返还），共享父仍只附接一次。
        if j.resources
            .as_ref()
            .is_some_and(|r| r.shared_parent || r.outer_cap.is_some())
            || j.material_issued
            || j.unexpected
            || j.overflow
        {
            j.unexpected = true;
            return Err(construction_state::stale("固定写父账重复附接"));
        }
        let permit = budget
            .reserve(j.bytes)
            .map_err(|_| construction_state::stale("journal父账额度不足"))?;
        let journal_charge = permit.finish(j.bytes);
        j.resources = Some(MaterialResources {
            budget: budget.clone(),
            outer_cap: None,
            parent_limit,
            publication: None,
            journal_owned_bytes: j.bytes,
            shared_parent: true,
            journal_charge,
        });
        Ok(())
    }
    pub(super) fn bind_set(&self, seed: construction_set::Seed) -> Result<(), String> {
        let mut j = self.journal.lock().map_err(invalid)?;
        if j.operation != Operation::Publish || j.set_required || !j.keys.is_empty() {
            return Err(construction_state::stale("集合seed不是本次publish写前绑定"));
        }
        j.set_required = true;
        j.set_seed = Some(seed);
        Ok(())
    }
    /// 无共享父：journal 私有子账（上限 cap）先出生，首笔许可先于任何 journal 分配。
    /// #1488 P3-b：生产唯一入口是 start_on；本入口只剩测试调用方，仅测试编译。
    #[cfg(test)]
    pub(super) fn start(
        tx: &Transaction<'_>,
        operation: Operation,
        cap: usize,
        strict: bool,
    ) -> Result<Option<Self>, String> {
        Self::start_inner(tx, operation, cap, strict, None)
    }
    /// #1488 P2-2：生产入口。journal 许可在 Journal 控制块与首个 keys 块分配之前从共享父取得，
    /// 随 journal 持有；之后每个新块仍由 reserve_one 先预留再分配。
    pub(super) fn start_on(
        tx: &Transaction<'_>,
        operation: Operation,
        cap: usize,
        strict: bool,
        budget: &native::Budget,
        parent_limit: usize,
    ) -> Result<Option<Self>, String> {
        Self::start_inner(tx, operation, cap, strict, Some((budget, parent_limit)))
    }
    fn start_inner(
        tx: &Transaction<'_>,
        operation: Operation,
        cap: usize,
        strict: bool,
        parent: Option<(&native::Budget, usize)>,
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
        // #1488 P3-b：结果类别与旧 start+attach_parent 一致——首块超帽仍 Ok(None)（旧 reserve_one
        // 先按帽拒、未及附接），且不先动父账。
        let first_block = vec_bytes::<Key>(32);
        if inline.checked_add(first_block).is_none_or(|n| n > cap) {
            return Ok(None);
        }
        // 先取父与首笔许可：共享父直接预留；无父则私有子账先出生再预留（不经 moved_in）。
        let (budget, parent_limit, shared_parent) = match parent {
            Some((budget, limit)) => (budget.clone(), limit, true),
            None => (
                native::Budget::fresh(cap)
                    .map_err(|_| construction_state::stale("材料共享父账元数据不足"))?,
                cap,
                false,
            ),
        };
        // 父账装不下 inline+首块即旧 attach_parent(j.bytes) 的“journal父账额度不足”，且父账
        // live/峰值都不变。探测许可立即返还；成功时峰值恰为下方 inline+首块正式预留后的值。
        // 之后首块失败只剩真实分配失败，与旧无父 reserve_one 同为 Ok(None)。
        drop(
            budget
                .reserve(inline + first_block)
                .map_err(|_| construction_state::stale("journal父账额度不足"))?,
        );
        let journal_charge = budget
            .reserve(inline)
            .map_err(|_| construction_state::stale("journal父账额度不足"))?
            .finish(inline);
        let mut journal = Journal {
            keys: Vec::new(),
            nodes: Default::default(),
            events: Vec::new(),
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
            source_binding: None,
            stages: Vec::new(),
            active_stage: None,
            material_epoch: 0,
            extra_bytes: 0,
            consumed: Default::default(),
            stage_rows_bytes: 0,
            resources: Some(MaterialResources {
                budget,
                outer_cap: None,
                parent_limit,
                publication: None,
                journal_owned_bytes: if shared_parent { inline } else { 0 },
                shared_parent,
                journal_charge,
            }),
            material_issued: false,
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
            let set = j.set_required;
            let Some(receipt) = Receipt::row(j.keys.len()) else {
                j.overflow = true;
                return;
            };
            if !j.reserve_receipt(true, false, set) {
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
            if set {
                j.events.push(receipt);
            }
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
    ) -> Result<Option<Effects>, PublishError> {
        tx.update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
            .map_err(publish_err)?;
        clear_node_journal(tx);
        let mut journal = Arc::try_unwrap(self.journal)
            .map_err(|_| publish_err("效果日志仍有外借"))?
            .into_inner()
            .map_err(publish_err)?;
        if journal.unexpected {
            return Err(publish_err("固定 operation 出现未声明实际效果"));
        }
        if journal.overflow {
            return Ok(None);
        }
        let covered_total = if let Some(c) = journal.coverage.as_ref() {
            let after = tx.total_changes();
            let expected = c.hook_events.checked_add(c.node_inserts);
            if !c.exact || after.checked_sub(c.start) != expected {
                return Err(publish_stale("actual writes与hook/node收据不一致"));
            }
            Some(after)
        } else {
            None
        };
        if journal.set_required {
            validate_stage_coverage(&journal)?;
            journal.set.as_mut().unwrap().close_stages(
                journal.material_epoch,
                journal.events.len(),
                journal.stages.len(),
                covered_total.ok_or_else(|| publish_stale("集合缺完整实际写覆盖"))?,
            )?;
        }
        journal
            .keys
            .sort_unstable_by_key(|k| (k.table, k.rowid, k.digest));
        journal.keys.dedup_by_key(|k| (k.table, k.rowid, k.digest));
        let parent = journal
            .resources
            .as_ref()
            .filter(|r| r.shared_parent)
            .map(|r| (r.budget.clone(), r.parent_limit));
        let mut changes = native::Tracked::new(Vec::new());
        let changes_bytes = vec_bytes::<Change>(journal.keys.len());
        let initial = sum([journal.bytes, changes_bytes]);
        if initial >= cap {
            return Ok(None);
        }
        let changes_permit = parent
            .as_ref()
            .map(|(budget, _)| budget.reserve(changes_bytes))
            .transpose()
            .map_err(|_| publish_stale("effects行容器父账额度不足"))?;
        if changes.try_reserve_exact(journal.keys.len()).is_err() {
            return Ok(None);
        }
        let actual_changes = vec_bytes::<Change>(changes.capacity());
        changes.attach(changes_permit.map(|permit| permit.finish(actual_changes)));
        let mut used = initial;
        let mut peak = used.max(journal.peak);
        let mut lower = (Table::Meta, 0usize);
        for key in journal.keys {
            if lower.0 != key.table {
                lower = (key.table, 0);
            }
            let sql = key.table.row_sql();
            let Some(mut remaining) = cap.checked_sub(used) else {
                return Ok(None);
            };
            if let Some((budget, _limit)) = &parent {
                remaining = remaining.min(budget.remaining());
            }
            let loaded = if key.table == Table::Node {
                ExactRows::read_optional_bounded_params_in_parent(
                    tx,
                    sql,
                    [&key.digest[..]],
                    remaining,
                    parent.as_ref().map(|(budget, _)| budget),
                )?
            } else {
                ExactRows::read_optional_bounded_params_in_parent(
                    tx,
                    sql,
                    [key.rowid],
                    remaining,
                    parent.as_ref().map(|(budget, _)| budget),
                )?
            };
            let Some((mut rows, read_peak)) = loaded else {
                return Ok(None);
            };
            peak = peak.max(sum([used, read_peak]));
            if rows.rows.len() > 1 {
                return Err(publish_err("rowid 效果非唯一"));
            }
            if let Some(row) = rows.rows.first() {
                verify_storage(tx, key.table, row)?;
            }
            let new = rows.rows.pop();
            let row_header_bytes = rows.manual_heap_bytes();
            drop(rows);
            if parent.is_some() && row_header_bytes != 0 {
                return Err(publish_stale("父账读行头缺持有许可"));
            }
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
                        if let Some((budget, _)) = &parent {
                            require_row_owned(budget, &table.rows[i])
                                .map_err(|e| publish_err(format!("原行许可：{e:?}")))?;
                        }
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
                    return Err(publish_stale("stage即时原件与最终effects不一致"));
                }
            }
        }
        let mut set = journal.set.take();
        if let Some(set) = &mut set {
            let charge = journal.resources.as_mut().and_then(|r| r.outer_cap.take());
            set.hold_parent_charge(charge)?;
            if let Some(resources) = journal.resources.as_ref().filter(|r| r.shared_parent) {
                set.hold_resource_budget(resources.budget.clone())?;
            }
        }
        let set_bytes = set.as_ref().map_or(0, |set| set.retained_bytes());
        if let Some(resources) = journal.resources.as_ref().filter(|r| r.shared_parent) {
            let owned = resources.journal_owned_bytes;
            drop(journal.nodes);
            drop(journal.events);
            drop(journal.stages);
            drop(journal.consumed);
            let _ = (resources, owned); // resources在真实journal载荷之后析构，消费唯一许可。
        }
        Ok(Some(Effects {
            changes,
            set,
            operation: journal.operation,
            peak,
            // journal及读行头已真实Drop；旧载荷仍由原件账全额承接，不在这里重复计入。
            bytes: sum([used - journal.bytes, set_bytes]),
            covered_total,
            shared_parent: parent.is_some(),
        }))
    }
}
pub(super) struct Change {
    pub(super) table: Table,
    pub(super) rowid: i64,
    slot: Option<usize>,
    pub(super) old: Option<sc::CodecVec<SqlCell>>,
    pub(super) new: Option<sc::CodecVec<SqlCell>>,
}
impl Retained for Change {
    fn heap_bytes(&self) -> usize {
        // table/rowid/slot无动态载荷，原有old/new真实容器各计一次。
        sum([self.old.heap_bytes(), self.new.heap_bytes()])
    }
}
pub(super) struct Effects {
    pub(super) set: Option<construction_set::PendingSet>,
    pub(super) changes: native::Tracked<Vec<Change>>,
    pub(super) operation: Operation,
    pub(super) peak: usize,
    pub(super) bytes: usize,
    covered_total: Option<u64>,
    pub(super) shared_parent: bool,
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
            let old_manual = change.old.as_ref().map_or(0, manual_row_bytes);
            let new_payload = change.new.heap_bytes();
            let old_capacity = vec_bytes::<sc::CodecVec<SqlCell>>(rows.capacity());
            // #1488 U2：共享父下 originals 已在父账下读取出生，这里只核本父身份，不追认。
            if self.shared_parent {
                budget.require_codec_vec_owned(rows)?;
            }
            let native_rows = rows.is_native_bound();
            let mut mutate = |work: Option<&WorkScope>| -> Result<(), native::Failure> {
                if let Some(work) = work {
                    rows.try_reserve_native(work, 0)
                        .map_err(|e| native::Failure::Invalid(e.to_string()))?;
                }
                match change.new.take() {
                    Some(row) if change.old.is_some() => {
                        rows[index] = row;
                    }
                    Some(row) => {
                        if rows.len() == rows.capacity() {
                            let next = rows
                                .capacity()
                                .checked_mul(2)
                                .ok_or(native::Failure::BudgetExceeded)?
                                .max(4);
                            peak = peak.max(
                                vec_bytes::<sc::CodecVec<SqlCell>>(next)
                                    .checked_add(self.bytes)
                                    .ok_or(native::Failure::BudgetExceeded)?,
                            );
                        }
                        if let Some(work) = work {
                            rows.try_insert_native(index, row, work)
                                .map_err(|e| native::Failure::Invalid(e.to_string()))?;
                        } else {
                            budget.codec_vec_grow(rows, 1)?;
                            budget
                                .codec_work()?
                                .run(|work| rows.try_insert_native(index, row, work))
                                .map_err(|e| match e {
                                    sc::capture_allocation::scratch::WorkError::Resource(e) => {
                                        native::Failure::from(e)
                                    }
                                    sc::capture_allocation::scratch::WorkError::Operation(e) => {
                                        native::Failure::Invalid(e.to_string())
                                    }
                                })?;
                        }
                        shift.1 += 1;
                    }
                    None if change.old.is_some() => {
                        rows.remove(index);
                        shift.1 -= 1;
                    }
                    None => return Err(invalid("固定效果同时缺旧新原件").into()),
                }
                Ok(())
            };
            if native_rows {
                budget
                    .codec_work()?
                    .run(|scope| mutate(Some(scope)))
                    .map_err(native::Failure::from)?;
            } else {
                mutate(None)?;
            }
            *original_charge = original_charge
                .checked_sub(old_payload)
                .and_then(|n| n.checked_add(new_payload))
                .and_then(|n| {
                    n.checked_add(
                        vec_bytes::<sc::CodecVec<SqlCell>>(rows.capacity()) - old_capacity,
                    )
                })
                .ok_or_else(|| invalid("SQL原件增量归属账溢出/缺旧费用"))?;
            if self.shared_parent {
                drop(change.old.take());
                if old_manual != 0 {
                    return Err(invalid("原行未携带许可").into());
                }
            }
        }
        drop(self.changes);
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
            SqlCell::Text(b) => (Type::Text, Some(b.as_ref())),
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
        let existed = j.nodes.get(&digest).map(|&index| index as usize);
        if existed.is_some() && !j.set_required {
            return;
        }
        if j.set_required {
            let Some(next) = j.material_epoch.checked_add(1) else {
                j.unexpected = true;
                return;
            };
            j.material_epoch = next;
        }
        // #1488 0035：同摘要只留首次 occurrence 的 key；每条收据（含 Noop）另记一个事件。
        let index = existed.unwrap_or(j.keys.len());
        let after = tx.total_changes();
        let (Ok(slot), Some(receipt)) = (
            u32::try_from(index),
            Receipt::node(index, before, after, inserted),
        ) else {
            j.overflow = true;
            return;
        };
        let fresh = existed.is_none();
        let set = j.set_required;
        if !j.reserve_receipt(fresh, fresh, set) {
            return;
        }
        if fresh {
            j.nodes.insert(digest, slot);
            let material_epoch = j.material_epoch;
            j.keys.push(Key {
                table: Table::Node,
                rowid: 0,
                digest,
                action: None,
                material_epoch,
                node_write: Some((before, after, inserted)),
            });
        }
        if set {
            j.events.push(receipt);
        }
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
    row: sc::CodecVec<SqlCell>,
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
    #[cfg(test)]
    stage: Option<&'work crate::staged_nodes::StageOwner<'work>>,
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
impl<'tx, 'work, 'db> MaterialAccess<'tx, 'work, 'db> {
    pub(crate) fn work(&self) -> &'work WorkScope {
        self.work
    }
    fn check(&self, j: &Journal) -> Result<(), String> {
        let actual = current_journal(self.tx)?
            .ok_or_else(|| construction_state::stale("实际事务已无owner日志"))?;
        let seal = j
            .set_seed
            .as_ref()
            .map(|s| s.seal)
            .or_else(|| j.set.as_ref().map(|s| s.seal))
            .or_else(|| j.source_binding.map(|s| s.seal));
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
    fn reader<'material>(&'material self) -> Result<ActualRead<'material, 'db>, PublishError> {
        Ok(ActualRead {
            inner: crate::content_store::TxStore::new(self.tx)
                .map_err(publish_err)?
                .with_work(Some(self.work)),
        })
    }
}

/// 数据可拷贝，构造不可外借；同时钉住实际材料与raw，不代表业务已核。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceCutBinding {
    seal: SnapshotSeal,
    epoch: u64,
    root: construction_set::Declaration,
    raw: RawAdvanceBinding,
}
pub(crate) struct SourceMaterial<'material, 'tx, 'work, 'db> {
    access: &'material MaterialAccess<'tx, 'work, 'db>,
    reader: ActualRead<'material, 'db>,
    binding: SourceCutBinding,
    set: construction_set::LazySet,
}
impl SourceMaterial<'_, '_, '_, '_> {
    fn check(&self) -> Result<(), String> {
        let j = self.access.journal.lock().map_err(invalid)?;
        self.access.check(&j)?;
        if j.source_binding != Some(self.binding) || j.material_epoch != self.binding.epoch {
            return Err(construction_state::stale("来源核执行中材料切面改变"));
        }
        Ok(())
    }
    pub(crate) fn root(&self) -> sc::Built {
        self.binding.root.built()
    }
    pub(crate) fn binding(&self) -> SourceCutBinding {
        self.binding
    }
    pub(crate) fn sequence_delta(
        &mut self,
        which: construction_set::SourceArray,
    ) -> Result<&construction_set::SequenceDelta, PublishError> {
        self.check()?;
        self.set.delta(&mut self.reader, self.access.budget, which)
    }
    pub(crate) fn request_delta(
        &mut self,
        ordinal: u64,
    ) -> Result<&construction_set::SequenceDelta, PublishError> {
        self.check()?;
        self.set
            .request_delta(&mut self.reader, self.access.budget, ordinal)
    }
    pub(crate) fn array_member(
        &mut self,
        which: construction_set::SourceArray,
        ordinal: u64,
    ) -> Result<super::super::audit_store::RefValue, PublishError> {
        self.check()?;
        self.set
            .member(&mut self.reader, self.access.budget, which, ordinal)
    }
    pub(crate) fn request_member(
        &mut self,
        knowledge_ordinal: u64,
        ordinal: u64,
    ) -> Result<super::super::audit_store::RefValue, PublishError> {
        self.check()?;
        self.set.request_member(
            &mut self.reader,
            self.access.budget,
            knowledge_ordinal,
            ordinal,
        )
    }
}
impl sc::NodeSource for SourceMaterial<'_, '_, '_, '_> {
    fn codec_work(&self) -> Option<&WorkScope> {
        Some(self.access.work)
    }
    fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        self.check()
            .map_err(|e| sc::CodecError::new(sc::code::SOURCE, e))?;
        sc::NodeSource::read(&mut self.reader, digest)
    }
    fn checked_read_scoped(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        self.check()
            .map_err(|e| sc::CodecError::new(sc::code::SOURCE, e))?;
        sc::NodeSource::checked_read_scoped(&mut self.reader, digest)
    }
}
impl CodecRead for SourceMaterial<'_, '_, '_, '_> {
    fn generic_source(&mut self) -> &mut dyn sc::NodeSource {
        self
    }
}
impl sc::NodeSink for SourceMaterial<'_, '_, '_, '_> {
    fn codec_sink_work(&self) -> Option<&WorkScope> {
        Some(self.access.work)
    }
    fn put(&mut self, _: sc::NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        if let Ok(mut j) = self.access.journal.lock() {
            j.unexpected = true;
        }
        Err(sc::CodecError::new(
            sc::code::SOURCE,
            "来源核材料只读，禁止写入被审原件",
        ))
    }
}
impl super::super::audit_store::CodecStore for SourceMaterial<'_, '_, '_, '_> {
    fn generic_store(&mut self) -> &mut dyn sc::NodeStore {
        self
    }
}

/// 唯一来源桥：真实effects追加原件→Public材料闭包→S语义核→同binding封C2。
pub(super) fn audit_native_source(
    access: &MaterialAccess<'_, '_, '_>,
    capture: &mut sc::AuditCapture,
    previous: Option<super::super::source_audit::NativeSourceState>,
    root: sc::RefView<'_>,
    raw: &BoundRawAdvance<'_>,
    profile: &str,
    rule: &str,
) -> Result<
    (
        super::super::source_audit::Counts,
        super::super::source_audit::NativeSourceState,
    ),
    PublishError,
> {
    let result = (|| {
        raw.ensure_current()?;
        let mut reader = access.reader()?;
        let (seed, binding) = {
            let mut j = access.journal.lock().map_err(publish_err)?;
            access.check(&j)?;
            if j.set.is_some()
                || j.source_binding.is_some()
                || j.active_stage.is_some()
                || !j.stages.is_empty()
            {
                return Err((construction_state::stale("来源材料不是stage前唯一切面")).into());
            }
            // 0069 真实路径锁：与 s_content 原件追加索引扩容超帽同一方式 reject 并经 invalid 上抛。
            #[cfg(test)]
            if super::SOURCE_APPEND_ALLOCATION_FAILURE.with(|flag| flag.replace(false)) {
                return Err((publish_err(
                    access
                        .work
                        .reject(sc::capture_allocation::scratch::ScratchFailure::Allocation),
                ))
                .into());
            }
            // nodes集合只来自当前实际tx的受控写口；每个tuple从真实事务重新读取。
            for digest in j.nodes.keys() {
                // #1488：活动域读取签发的正文许可随正文移入 owner，不再走 Plain 早退。
                let node =
                    sc::NodeSource::read_scoped(&mut reader.inner, digest).map_err(publish_err)?;
                let sc::job::ScopedNodeRead::Found { tag, body } = node else {
                    return Err((construction_state::stale("实际effects新增原件缺失")).into());
                };
                let appended = match body.into_parts() {
                    (body, None) => capture.append_original_for_owner(
                        *digest,
                        tag,
                        body,
                        j.cap,
                        Some(access.work),
                    ),
                    (body, Some(fee)) => {
                        let mut body = sc::CodecVec::from_plain(body);
                        body.attach_native_reservation(fee).and_then(|()| {
                            capture.append_original_codec_for_owner(
                                *digest,
                                tag,
                                body,
                                j.cap,
                                access.work,
                            )
                        })
                    }
                };
                if !appended.map_err(publish_err)? {
                    return Err((construction_state::stale("原件追加预算未消费")).into());
                }
            }
            capture
                .verify_owner_refview(root, sc::Mode::Public, access.work)
                .map_err(publish_err)?;
            let seed = j
                .set_seed
                .take()
                .ok_or_else(|| construction_state::stale("来源核缺owner seed"))?;
            if seed.raw != raw.current_binding() {
                return Err((construction_state::stale("来源材料raw binding不同")).into());
            }
            let binding = SourceCutBinding {
                seal: seed.seal,
                epoch: j.material_epoch,
                root: construction_set::Declaration::read(root)?,
                raw: seed.raw,
            };
            j.source_binding = Some(binding);
            (seed, binding)
        };
        let mut material = SourceMaterial {
            access,
            reader,
            binding,
            set: construction_set::LazySet::new(seed, root, previous.as_ref())?,
        };
        let verdict = super::super::source_audit::audit_native_bound(
            previous,
            &mut material,
            raw,
            profile,
            rule,
        )?;
        material.check()?;
        raw.ensure_current()?;
        let (actual_binding, counts, source) = verdict.into_parts();
        if actual_binding != binding {
            return Err((construction_state::stale("来源核返回不同切面")).into());
        }
        let set = material
            .set
            .finish(binding.epoch, counts.clone(), raw.current_prefix())?;
        let mut j = access.journal.lock().map_err(publish_err)?;
        access.check(&j)?;
        if j.source_binding != Some(binding) || j.material_epoch != binding.epoch {
            return Err((construction_state::stale("来源核封口前材料epoch改变")).into());
        }
        j.peak = j
            .peak
            .max(if j.resources.as_ref().is_some_and(|r| r.shared_parent) {
                j.bytes
            } else {
                access.budget.peak()
            });
        j.extra_bytes = sum([j.extra_bytes, set.retained_bytes()]);
        j.bytes = sum([j.container_bytes(), j.extra_bytes]);
        if j.bytes > j.cap {
            return Err((construction_state::stale("来源/集合保留账超过材料帽")).into());
        }
        j.set = Some(set);
        j.source_binding = None;
        Ok((counts, source))
    })();
    if result.is_err() {
        if let Ok(mut j) = access.journal.lock() {
            j.unexpected = true;
        }
        raw.invalidate_after_consumer_failure();
    }
    result
}

/// 暂存只持 Weak；finish 仍可证明没有外借强引用并 try_unwrap。
pub(crate) struct PublicationStageIdentity {
    journal: Weak<Mutex<Journal>>,
    identity: [u64; 3],
    scope: usize,
    connection: usize,
}
impl PublicationStageIdentity {
    pub(crate) fn check(&self, tx: &Transaction<'_>, work: &WorkScope) -> Result<(), String> {
        let actual = current_journal(tx)?
            .ok_or_else(|| construction_state::stale("publication journal已失效"))?;
        let held = self
            .journal
            .upgrade()
            .ok_or_else(|| construction_state::stale("publication journal已退役"))?;
        if tx.is_autocommit()
            || self.connection != unsafe { tx.handle() } as usize
            || self.scope != work as *const WorkScope as usize
            || !Arc::ptr_eq(&actual, &held)
        {
            return Err(construction_state::stale(
                "publication暂存跨事务/父scope/journal",
            ));
        }
        let j = actual.lock().map_err(invalid)?;
        if j.unexpected
            || j.overflow
            || j.resources.as_ref().and_then(|r| r.publication) != Some((self.identity, self.scope))
        {
            return Err(construction_state::stale("publication暂存资源已失效"));
        }
        Ok(())
    }
}
pub(super) fn attach_recording_resources(
    tx: &Transaction<'_>,
    parent: &PublicationScope<'_>,
) -> Result<(), String> {
    parent.check_transaction(tx)?;
    let journal = current_journal(tx)?
        .ok_or_else(|| construction_state::stale("publication缺真实journal"))?;
    let mut j = journal.lock().map_err(invalid)?;
    let result = (|| {
        if j.resources
            .as_ref()
            .is_none_or(|r| !r.shared_parent || r.publication.is_some())
            || j.material_issued
            || j.unexpected
            || j.overflow
            || j.operation != Operation::Publish
            || !j.set_required
            || j.set_seed.as_ref().map(|s| s.seal) != Some(parent.resources.before)
        {
            return Err(construction_state::stale(
                "publication资源附接重复或身份失效",
            ));
        }
        let resources = j.resources.as_mut().unwrap();
        resources.publication = Some((
            parent.identity(),
            parent.work() as *const WorkScope as usize,
        ));
        Ok(())
    })();
    if result.is_err() {
        j.unexpected = true;
    }
    result
}
pub(super) fn publication_stage_identity(
    tx: &Transaction<'_>,
    parent: &PublicationScope<'_>,
) -> Result<PublicationStageIdentity, String> {
    parent.check_transaction(tx)?;
    let journal =
        current_journal(tx)?.ok_or_else(|| construction_state::stale("publication缺journal"))?;
    let token = PublicationStageIdentity {
        journal: Arc::downgrade(&journal),
        identity: parent.identity(),
        scope: parent.work() as *const WorkScope as usize,
        connection: unsafe { tx.handle() } as usize,
    };
    drop(journal);
    token.check(tx, parent.work())?;
    Ok(token)
}
pub(super) fn with_material_in_publication<T>(
    tx: &Transaction<'_>,
    parent: &PublicationScope<'_>,
    #[cfg(test)] stage: Option<&crate::staged_nodes::StageOwner<'_>>,
    f: impl FnOnce(Option<&MaterialAccess<'_, '_, '_>>) -> Result<T, PublishError>,
) -> Result<T, PublishError> {
    #[cfg(test)]
    parent.trace("material");
    let identity = publication_stage_identity(tx, parent)?;
    let journal = identity
        .journal
        .upgrade()
        .ok_or_else(|| construction_state::stale("材料journal已退役"))?;
    let (seal, epoch) = {
        let mut j = journal.lock().map_err(publish_err)?;
        if j.material_issued
            || j.set.is_some()
            || j.source_binding.is_some()
            || j.active_stage.is_some()
            || !j.stages.is_empty()
        {
            j.unexpected = true;
            return Err((construction_state::stale("材料能力只能在本publish签发一次")).into());
        }
        j.material_issued = true;
        (
            j.set_seed
                .as_ref()
                .ok_or_else(|| construction_state::stale("材料缺seed"))?
                .seal,
            j.material_epoch,
        )
    };
    let access = MaterialAccess {
        tx,
        work: parent.work(),
        budget: parent.budget(),
        journal: journal.clone(),
        seal,
        initial_material_epoch: epoch,
        #[cfg(test)]
        stage,
    };
    let result = f(Some(&access));
    drop(access);
    let result = result.and_then(|value| {
        let j = journal.lock().map_err(publish_err)?;
        if j.unexpected || j.overflow || j.active_stage.is_some() || j.set.is_none() {
            Err((construction_state::stale("材料闭包未完整闭合或吞下失败")).into())
        } else {
            Ok(value)
        }
    });
    if result.is_err() {
        journal.lock().map_err(publish_err)?.unexpected = true;
    }
    result
}

/// 只能由父owner转接prepare原有work。Some逐字借用，不创建替代scope；
/// None只从同一journal cap建立一次共享父账，后续失败不能重建洗账。
pub(super) fn with_material_access<T, E: From<String> + From<PublishError>>(
    tx: &Transaction<'_>,
    original_work: Option<&WorkScope>,
    f: impl FnOnce(Option<&MaterialAccess<'_, '_, '_>>) -> Result<T, E>,
) -> Result<T, E> {
    let Some(journal) = current_journal(tx)? else {
        return f(None);
    };
    let (budget, seal, initial_material_epoch) = {
        let mut j = journal.lock().map_err(publish_err)?;
        if !j.set_required {
            drop(j);
            return f(None);
        }
        let result: Result<_, E> = (|| {
            // #1488 P2-2：私有子账自 Recording 出生即存在；重复签发由 material_issued 拦截。
            if j.unexpected
                || j.overflow
                || j.resources
                    .as_ref()
                    .is_none_or(|r| r.shared_parent && r.publication.is_some())
                || j.material_issued
                || j.set.is_some()
                || !j.stages.is_empty()
                || tx.is_autocommit()
            {
                #[cfg(test)]
                j.observe_failure(
                    "material-entry-flags",
                    None,
                    None,
                    Some(tx.is_autocommit()),
                    None,
                );
                return Err((construction_state::stale("材料能力只能在本publish签发一次")).into());
            }
            let seal = j
                .set_seed
                .as_ref()
                .ok_or_else(|| construction_state::stale("材料能力缺owner seed"))?
                .seal;
            j.material_issued = true;
            let resources = j.resources.as_ref().expect("上方已核");
            if resources.shared_parent {
                if original_work.is_some() {
                    return Err((construction_state::stale(
                        "共享父账必须经publication同scope能力传入",
                    ))
                    .into());
                }
                return Ok((resources.budget.clone(), seal, j.material_epoch));
            }
            // #1488 P2-2：私有子账与 journal 首笔许可已在 Recording 出生、任何 journal 分配之前取得，
            // 许可随 journal 移动；这里只占外父账 cap，不再事后按 j.bytes 同额补签。
            let outer_cap = original_work
                .map(|w| w.reserve_native(j.cap))
                .transpose()
                .map_err(|error| {
                    #[cfg(test)]
                    j.observe_failure(
                        "material-outer-cap",
                        Some(j.cap),
                        error.scratch_failure(),
                        Some(tx.is_autocommit()),
                        original_work.map(|w| w.snapshot()),
                    );
                    publish_err(error)
                })?;
            let resources = j.resources.as_mut().expect("上方已核");
            resources.outer_cap = outer_cap;
            let budget = resources.budget.clone();
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
            #[cfg(test)]
            stage: None,
        };
        let result = f(Some(&access));
        #[cfg(test)]
        if result.is_err() {
            if let Ok(j) = journal.lock() {
                j.observe_failure(
                    "material-operation-return",
                    None,
                    None,
                    Some(tx.is_autocommit()),
                    Some(scope.snapshot()),
                );
            }
        }
        result
    };
    let result = match original_work {
        Some(scope) => run(scope),
        None => match budget.codec_work() {
            Ok(work) => match work.run(run) {
                Ok(value) => Ok(value),
                Err(WorkError::Operation(error)) => Err((error).into()),
                Err(WorkError::Resource(_resource)) => {
                    #[cfg(test)]
                    if let Ok(j) = journal.lock() {
                        j.observe_failure(
                            "material-codec-resource-return",
                            None,
                            Some(_resource),
                            Some(tx.is_autocommit()),
                            None,
                        );
                    }
                    Err((construction_state::stale("材料共享codec额度/工作域失效")).into())
                }
            },
            Err(_) => {
                #[cfg(test)]
                if let Ok(j) = journal.lock() {
                    j.observe_failure(
                        "material-codec-create",
                        None,
                        None,
                        Some(tx.is_autocommit()),
                        None,
                    );
                }
                Err((construction_state::stale("材料共享codec工作域额度不足")).into())
            }
        },
    };
    let result = result.and_then(|value| {
        let j = journal.lock().map_err(publish_err)?;
        if j.unexpected || j.overflow || j.active_stage.is_some() || j.set.is_none() {
            #[cfg(test)]
            j.observe_failure(
                "material-closure-flags",
                None,
                None,
                Some(tx.is_autocommit()),
                None,
            );
            Err((construction_state::stale("材料闭包未完整闭合或吞下失败")).into())
        } else {
            Ok(value)
        }
    });
    if result.is_err() {
        journal.lock().map_err(publish_err)?.unexpected = true;
    }
    result
}

/// 完整旧来源审计成功后，从封闭能力的真实tx读取材料cut。
pub(super) fn note_source_cut(
    material: &MaterialAccess<'_, '_, '_>,
    root: sc::RefView<'_>,
) -> Result<(), PublishError> {
    let mut j = material.journal.lock().map_err(publish_err)?;
    let result = (|| {
        material.check(&j)?;
        if j.set.is_some() || j.active_stage.is_some() || !j.stages.is_empty() {
            return Err(publish_stale("集合cut不是首次stage前唯一切面"));
        }
        let mut source = material.reader()?;
        // 当前root参数本身不携带来源资格；在实际tx上核完整Public闭包后才允许共享跳过。
        source
            .verify_many_ref_fresh([root], sc::Mode::Public)
            .map_err(publish_err)?;
        let seed = j
            .set_seed
            .take()
            .ok_or_else(|| publish_stale("缺owner集合seed"))?;
        let set =
            construction_set::qualify(seed, &mut source, root, j.material_epoch, material.budget)?;
        j.peak = j
            .peak
            .max(if j.resources.as_ref().is_some_and(|r| r.shared_parent) {
                j.bytes
            } else {
                material.budget.peak()
            });
        j.extra_bytes = sum([j.extra_bytes, set.retained_bytes()]);
        j.bytes = sum([j.container_bytes(), j.extra_bytes]);
        if j.bytes > j.cap {
            return Err(publish_stale("集合cut保留账超过额度"));
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
        guard.start = j.events.len();
        guard.material_before = j.material_epoch;
        j.active_stage = Some(guard.start);
    }
    guard.journal = Some(journal);
    Ok(guard)
}
// Native行的许可跟随pop后的行移动，头部析构不退行的费用。
// #1488 U2：父账下原行已由本父许可出生，这里只核身份，不追认；未持票原行拒绝。
fn require_row_owned(
    budget: &native::Budget,
    row: &sc::CodecVec<SqlCell>,
) -> Result<(), native::Failure> {
    for cell in row.iter() {
        if let SqlCell::Text(bytes) | SqlCell::Blob(bytes) = cell {
            budget.require_codec_vec_owned(bytes)?;
        }
    }
    budget.require_codec_vec_owned(row)
}
fn manual_row_bytes(row: &sc::CodecVec<SqlCell>) -> usize {
    super::super::request_audit::manual_cells_bytes(row)
}
struct StageOriginals {
    value: Option<ExactRows>,
}
impl StageOriginals {
    fn row(&self) -> &[SqlCell] {
        &self.value.as_ref().unwrap().rows[0]
    }
    fn into_row(mut self) -> sc::CodecVec<SqlCell> {
        self.value.as_mut().unwrap().rows.pop().unwrap()
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
    let actual = consumed_bytes(j.consumed.capacity());
    j.resources
        .as_mut()
        .ok_or_else(|| construction_state::stale("stage缺父账许可"))?
        .journal_charge
        .replace_part(old, permit.finish(actual))
        .map_err(|_| construction_state::stale("stage许可来源失配"))?;
    if let Some(resources) = j.resources.as_mut().filter(|r| r.shared_parent) {
        resources.journal_owned_bytes = resources
            .journal_owned_bytes
            .checked_sub(old)
            .and_then(|n| n.checked_add(actual))
            .expect("stage身份账");
    }
    Ok(())
}
impl StageGuard {
    pub(crate) fn finish(
        mut self,
        material: Option<&MaterialAccess<'_, '_, '_>>,
    ) -> Result<(), PublishError> {
        let Some(journal) = self.journal.as_ref().cloned() else {
            self.closed = true;
            return Ok(());
        };
        #[cfg(test)]
        if let Some(material) = material {
            if let Some(stage) = material.stage {
                let rowid = {
                    let j = journal.lock().map_err(publish_err)?;
                    material.check(&j)?;
                    let mut rows = j.events[self.start..]
                        .iter()
                        .map(|e| &j.keys[e.key()])
                        .filter(|k| k.table != Table::Node);
                    let row = rows
                        .next()
                        .ok_or_else(|| publish_stale("stage缺实际SQL行"))?;
                    if rows.next().is_some()
                        || row.table != self.kind.table()
                        || row.action != Some(self.kind.action())
                    {
                        return Err(publish_stale("stage SQL并非唯一预期表/动作"));
                    }
                    row.rowid
                };
                // 固定四根栈数组；不在持 journal 锁时调用会回入 note_node 的 SQL put。
                let columns = self.kind.roots();
                let roots = material
                    .tx
                    .query_row(self.kind.table().row_sql(), [rowid], |row| {
                        let mut roots = [[0u8; 32]; 4];
                        for (index, &column) in columns.iter().enumerate() {
                            let bytes = row.get_ref(column)?.as_blob().map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    column,
                                    rusqlite::types::Type::Blob,
                                    Box::new(e),
                                )
                            })?;
                            roots[index] = bytes.try_into().map_err(|_| {
                                rusqlite::Error::InvalidColumnType(
                                    column,
                                    "stage root".into(),
                                    rusqlite::types::Type::Blob,
                                )
                            })?;
                        }
                        Ok(roots)
                    })
                    .map_err(publish_err)?;
                stage.flush_cut(
                    material.tx,
                    crate::staged_nodes::FlushCut::Row(&roots[..columns.len()]),
                )?;
            }
        }
        let mut j = journal.lock().map_err(publish_err)?;
        let result = (|| {
            let material = material.ok_or_else(|| publish_stale("strict stage缺实际材料能力"))?;
            if !Arc::ptr_eq(&journal, &material.journal) {
                return Err(publish_stale("stage与材料能力不属同owner日志"));
            }
            material.check(&j)?;
            let tx = material.tx;
            if self.connection != unsafe { tx.handle() } as usize
                || j.active_stage != Some(self.start)
                || j.material_epoch < self.material_before
            {
                return Err(publish_stale("stage失效或跨连接"));
            }
            let mut rows = j.events[self.start..]
                .iter()
                .map(|event| &j.keys[event.key()])
                .filter(|key| key.table != Table::Node);
            let row = rows
                .next()
                .ok_or_else(|| publish_stale("stage缺实际SQL行"))?;
            if rows.next().is_some()
                || row.table != self.kind.table()
                || row.action != Some(self.kind.action())
            {
                return Err(publish_stale("stage SQL并非唯一预期表/动作"));
            }
            let rowid = row.rowid;
            let identity = (self.kind.table(), rowid);
            if j.consumed.contains(&identity) {
                return Err(publish_stale("业务原件已被前一stage消费"));
            }
            let budget = material.budget;
            let old_stage_capacity = vec_bytes::<StageReceipt>(j.stages.capacity());
            let mut stage_fee = j
                .resources
                .as_mut()
                .ok_or_else(|| publish_stale("stage缺父账"))?
                .journal_charge
                .split(old_stage_capacity)
                .map_err(|_| publish_stale("stage旧许可不足"))?;
            let grow = budget.grow_vec(&mut j.stages, &mut stage_fee, 1);
            j.resources
                .as_mut()
                .unwrap()
                .journal_charge
                .merge(&mut stage_fee)
                .map_err(|_| publish_stale("stage许可来源失配"))?;
            grow.map_err(|_| publish_stale("stage容器额度不足"))?;
            let stage_capacity = vec_bytes::<StageReceipt>(j.stages.capacity());
            if let Some(resources) = j.resources.as_mut().filter(|r| r.shared_parent) {
                resources.journal_owned_bytes += stage_capacity - old_stage_capacity;
            }
            reserve_consumed(&mut j, budget)?;
            let remaining =
                if let Some(_resources) = j.resources.as_ref().filter(|r| r.shared_parent) {
                    j.cap
                        .checked_sub(j.bytes)
                        .ok_or_else(|| publish_stale("stage局部原件额度不足"))?
                        .min(budget.remaining())
                } else {
                    j.cap
                        .checked_sub(budget.live())
                        .ok_or_else(|| publish_stale("stage原件额度不足"))?
                };
            let (originals, _) = ExactRows::read_optional_bounded_params_in_parent(
                tx,
                self.kind.table().row_sql(),
                [rowid],
                remaining,
                Some(budget),
            )?
            .ok_or_else(|| publish_stale("stage原件读取额度不足"))?;
            let originals = StageOriginals {
                value: Some(originals),
            };
            if originals.value.as_ref().unwrap().rows.len() != 1 {
                return Err(publish_stale("stage当前行缺失/重复"));
            }
            verify_storage(tx, self.kind.table(), originals.row())?;
            let mut source = material.reader()?;
            // 只从保存的actual tx回读根闭包；外来memory或未来stage均无补签入口。
            for &column in self.kind.roots() {
                let digest: [u8; 32] = match &originals.row()[column] {
                    SqlCell::Blob(value) => value.as_ref().try_into().map_err(publish_err)?,
                    _ => return Err(publish_stale("stage根不是BLOB")),
                };
                let root = sc::FixedRoot::new(&digest);
                let reference = source
                    .content_ref_view(root.as_str())
                    .map_err(publish_err)?;
                source
                    .verify_many_ref_fresh([reference.view()], sc::Mode::Public)
                    .map_err(publish_err)?;
            }
            j.peak = j
                .peak
                .max(if j.resources.as_ref().is_some_and(|r| r.shared_parent) {
                    j.bytes
                } else {
                    budget.peak()
                });
            let row = originals.into_row();
            j.stage_rows_bytes = sum([j.stage_rows_bytes, row.heap_bytes()]);
            if let Some(resources) = j.resources.as_mut().filter(|r| r.shared_parent) {
                resources.journal_owned_bytes += manual_row_bytes(&row);
            }
            let end = j.events.len();
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
            j.bytes = sum([j.container_bytes(), j.extra_bytes]);
            j.peak = j.peak.max(j.bytes);
            if j.bytes > j.cap {
                return Err(publish_stale("stage实际保留账超过额度"));
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
    // #1488 0035：逐收据核对读 events（发生序）；key 只在首次 occurrence 出现，且出现序即 keys 序。
    let mut introduced = 0usize;
    for (ordinal, receipt) in j.events.iter().enumerate() {
        let index = receipt.key();
        let key = j
            .keys
            .get(index)
            .ok_or_else(|| construction_state::stale("收据事件key序/归属不符"))?;
        let fresh = index == introduced;
        if fresh {
            introduced += 1;
        } else if index > introduced || key.table != Table::Node {
            return Err(construction_state::stale("收据事件key序/归属不符"));
        }
        if key.table == Table::Node {
            let Some((before, after, inserted)) = key.node_write else {
                return Err(construction_state::stale("node缺实际put收据"));
            };
            node_epoch = node_epoch
                .checked_add(1)
                .ok_or_else(|| construction_state::stale("node事件序耗尽"))?;
            if receipt.write_delta() != Some(u64::from(receipt.inserted()))
                || (fresh
                    && (key.material_epoch != node_epoch
                        || receipt.inserted() != inserted
                        || after.checked_sub(before) != Some(u64::from(inserted))))
            {
                return Err(construction_state::stale("node多次事件序/实际写数不符"));
            }
            continue;
        }
        if receipt.inserted() || receipt.write_delta() != Some(0) {
            return Err(construction_state::stale("收据事件key序/归属不符"));
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
    // 闭合：每个 key 至少被一个事件引用；node 事件数恰为 material_epoch（无 occurrence 丢失）。
    if introduced != j.keys.len() || node_epoch != j.material_epoch {
        return Err(construction_state::stale("node多次事件序/实际写数不符"));
    }
    Ok(())
}

#[cfg(test)]
struct ReceiptView {
    digest: [u8; 32],
    inserted: bool,
    material_epoch: u64,
}
#[cfg(test)]
impl Journal {
    /// 把 events 按发生序展开为逐收据视图；node 的 material_epoch 为 node 事件序数。
    fn receipts_for_test(&self) -> Vec<ReceiptView> {
        let mut epoch = 0;
        self.events
            .iter()
            .map(|event| {
                let key = &self.keys[event.key()];
                if key.table == Table::Node {
                    epoch += 1;
                }
                ReceiptView {
                    digest: key.digest,
                    inserted: event.inserted(),
                    material_epoch: if key.table == Table::Node {
                        epoch
                    } else {
                        key.material_epoch
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[test]
fn native_journal_only_reserves_the_container_that_grows() {
    let inline = std::mem::size_of::<Journal>() + 128;
    let mut j = Journal {
        keys: Vec::new(),
        nodes: Default::default(),
        events: Vec::new(),
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
        source_binding: None,
        stages: Vec::new(),
        active_stage: None,
        material_epoch: 0,
        extra_bytes: 0,
        consumed: Default::default(),
        stage_rows_bytes: 0,
        resources: None,
        material_issued: false,
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
    // #1488 P2-2（评审 0024）：journal 许可先于 Journal 控制块/首块分配取得并随 journal 持有。
    // 父余额不足首笔 inline 时在任何 journal 分配与 hook 安装之前拒绝且父账不变；成功时父账
    // 恰增 j.bytes，峰值不含"已分配后再同额补签"的过渡项，journal 析构后全额返还。
    #[test]
    fn p2_2_journal_permit_taken_before_recording_allocation() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let inline = std::mem::size_of::<Journal>() + 128;
        let tight = native::Budget::new(native::Budget::metadata_bytes() + inline - 1, 0).unwrap();
        let before = (tight.live(), tight.peak(), tight.remaining());
        assert!(
            Recording::start_on(&tx, Operation::Publish, 1 << 20, true, &tight, 1 << 20).is_err()
        );
        assert_eq!((tight.live(), tight.peak(), tight.remaining()), before);
        let parent = native::Budget::new(1 << 20, 0).unwrap();
        let base = parent.live();
        let recording =
            Recording::start_on(&tx, Operation::Publish, 1 << 20, true, &parent, 1 << 20)
                .unwrap()
                .unwrap();
        {
            let j = recording.journal.lock().unwrap();
            let r = j.resources.as_ref().unwrap();
            assert!(r.shared_parent && r.outer_cap.is_none());
            assert_eq!(r.journal_owned_bytes, j.bytes);
            assert_eq!(parent.live(), base + j.bytes);
            assert_eq!(parent.peak(), base + j.bytes);
            assert_eq!(parent.moved_in_for_test(), 0);
        }
        // hook/线程槽持 journal 共享引用；finish 卸下后 journal 析构，许可随之全额返还。
        assert!(recording.finish(&tx, &mut [], 0).unwrap().is_none());
        assert_eq!(parent.live(), base);
    }
    // #1488 P3-b（复审 0026）：start_on 结果类别与 P2-2 前生产 start+attach_parent 一致。
    // 首块超帽 → Ok(None)（旧 reserve_one 先按帽拒，即使父账连 inline 都不够，也不报父账错）；
    // 帽内但父账装不下 inline+首块 → Err("journal父账额度不足")（旧 attach_parent(j.bytes)）；
    // 两种拒绝都不改父账 live/峰值/余额；父账恰够 inline+首块即成功且峰值不含过渡项。
    #[test]
    fn p3b_start_on_keeps_pre_p2_2_result_categories() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let inline = std::mem::size_of::<Journal>() + 128;
        let block = vec_bytes::<Key>(32);
        let metadata = native::Budget::metadata_bytes();
        let observe = |b: &native::Budget| (b.live(), b.peak(), b.remaining());
        let starved = native::Budget::new(metadata + inline - 1, 0).unwrap();
        let before = observe(&starved);
        let over_cap = Recording::start_on(
            &tx,
            Operation::Publish,
            inline + block - 1,
            true,
            &starved,
            1 << 20,
        )
        .unwrap();
        assert!(over_cap.is_none());
        assert_eq!(observe(&starved), before);
        let short = native::Budget::new(metadata + inline + block - 1, 0).unwrap();
        let before = observe(&short);
        let error = Recording::start_on(&tx, Operation::Publish, 1 << 20, true, &short, 1 << 20)
            .err()
            .expect("父账装不下inline+首块须报错");
        assert!(error.contains("journal父账额度不足"), "{error}");
        assert_eq!(observe(&short), before);
        let exact = native::Budget::new(metadata + inline + block, 0).unwrap();
        let base = exact.live();
        let recording =
            Recording::start_on(&tx, Operation::Publish, 1 << 20, true, &exact, 1 << 20)
                .unwrap()
                .unwrap();
        assert_eq!(exact.live(), base + inline + block);
        assert_eq!(exact.peak(), base + inline + block);
        assert!(recording.finish(&tx, &mut [], 0).unwrap().is_none());
        assert_eq!(exact.live(), base);
    }
    // 无共享父：私有子账先出生（moved_in 恒 0），首笔许可等于 journal 实占；材料签发不再补签。
    #[test]
    fn p2_2_private_journal_parent_born_before_allocation_and_material_adds_no_fee() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let (child, at_start) = {
            let j = recording.journal.lock().unwrap();
            let r = j.resources.as_ref().unwrap();
            assert!(!r.shared_parent);
            assert_eq!(r.budget.moved_in_for_test(), 0);
            assert_eq!(r.budget.live(), native::Budget::metadata_bytes() + j.bytes);
            (r.budget.clone(), r.budget.live())
        };
        with_material_access::<_, PublishError>(&tx, None, |m| {
            let m = m.unwrap();
            assert!(m.budget.same_parent(&child), "材料子账即出生时子账");
            assert!(m.budget.live() >= at_start);
            note_source_cut(m, (&state).into())
        })
        .unwrap();
        assert_eq!(child.moved_in_for_test(), 0);
        drop(recording);
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
    fn publication_weak_identity_does_not_block_finish_with_large_parent_retention() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let before = recording
            .journal
            .lock()
            .unwrap()
            .set_seed
            .as_ref()
            .unwrap()
            .seal;
        recording.journal.lock().unwrap().cap = 64 << 10;
        let budget = native::Budget::new(8 << 20, 5 << 20).unwrap();
        recording.attach_parent(&budget, 8 << 20).unwrap();
        let resources = PublicationResources {
            budget: budget.clone(),
            parent_limit: 8 << 20,
            before,
            connection: unsafe { tx.handle() } as usize,
        };
        let effects = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let parent = PublicationScope {
                    resources: &resources,
                    work,
                };
                attach_recording_resources(&tx, &parent)?;
                let identity = parent.stage_identity(&tx)?;
                with_material_in_publication(&tx, &parent, None, |material| {
                    let material = material.unwrap();
                    assert!(
                        material.budget.live() > (64 << 10),
                        "父账大于journal子帽仍须允许真实工作"
                    );
                    assert!(std::ptr::eq(material.work(), work));
                    note_source_cut(material, (&state).into()).map_err(Into::into)
                })
                .map_err(newchan_rust::s_content::ContextError::into_message)?;
                let effects = recording
                    .finish(&tx, &mut [], 1 << 20)
                    .map_err(newchan_rust::s_content::ContextError::into_message)?
                    .unwrap();
                assert!(
                    identity.check(&tx, work).is_err(),
                    "旧Weak身份不得绑定已卸journal"
                );
                Ok::<_, String>(effects)
            })
            .unwrap();
        assert!(effects.has_exact_coverage(tx.total_changes()));
        assert!(effects.shared_parent);
        assert!(
            budget.live() > (4 << 20),
            "旧候选超过父帽一半仍可正常finish"
        );
        assert!(budget.peak() <= (8 << 20));
        assert!(effects.peak < (1 << 20), "局部峰不得混入大于5MiB的父峰");
    }
    #[test]
    fn publication_resource_attachment_is_once_and_foreign_scope_rejects() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, _) = set_recording(&tx);
        let before = recording
            .journal
            .lock()
            .unwrap()
            .set_seed
            .as_ref()
            .unwrap()
            .seal;
        let budget = native::Budget::new(16 << 20, 0).unwrap();
        recording.attach_parent(&budget, 16 << 20).unwrap();
        let resources = PublicationResources {
            budget: budget.clone(),
            parent_limit: 16 << 20,
            before,
            connection: unsafe { tx.handle() } as usize,
        };
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let parent = PublicationScope {
                    resources: &resources,
                    work,
                };
                attach_recording_resources(&tx, &parent)?;
                let identity = parent.stage_identity(&tx)?;
                budget
                    .codec_work()
                    .unwrap()
                    .run(|other| {
                        assert!(identity.check(&tx, other).is_err());
                        Ok::<_, String>(())
                    })
                    .unwrap();
                assert!(attach_recording_resources(&tx, &parent).is_err());
                assert!(identity.check(&tx, work).is_err());
                Ok::<_, String>(())
            })
            .unwrap();
        assert!(recording.finish(&tx, &mut [], 1 << 20).is_err());
    }

    #[test]
    fn owner_set_ordered_node_receipts_keep_repeated_noop_and_unreachable_nodes() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        with_material_access::<_, PublishError>(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            // #1488 0035（裁定 issuecomment-6278995984）：逐收据载体由 keys 改为 events，语义不变。
            let (before, keys_before, epoch_before) = {
                let j = recording.journal.lock().unwrap();
                (j.events.len(), j.keys.len(), j.material_epoch)
            };
            put(&tx, 45);
            put(&tx, 45);
            put(&tx, 46);
            let j = recording.journal.lock().unwrap();
            let receipts = j.receipts_for_test();
            let tail = &receipts[before..];
            assert_eq!(tail.len(), 3, "不可因摘要相同丢Noop occurrence");
            assert_eq!(tail[0].digest, tail[1].digest);
            assert!(tail[0].inserted);
            assert!(!tail[1].inserted);
            // #1488 0077 P13（0038 P3-1）：原 `tail[1] > tail[0]` 读 receipts_for_test 现算序数，恒真；
            // 改核 journal 真实计数器——三条收据（含 Noop）各推进一次，首见 key 记其发生时的读数。
            assert_eq!(
                j.material_epoch - epoch_before,
                3,
                "Noop occurrence 也须推进 material_epoch"
            );
            let keys = &j.keys[keys_before..];
            assert_eq!(keys[0].material_epoch, epoch_before + 1);
            assert_eq!(
                keys[1].material_epoch,
                epoch_before + 3,
                "46 首见于第三条收据"
            );
            assert_eq!(
                j.keys.len() - keys_before,
                2,
                "同摘要收据只合并key，不合并事件"
            );
            Ok(())
        })
        .unwrap();
        let effects = recording.finish(&tx, &mut [], 16 << 20).unwrap().unwrap();
        assert!(effects.set.is_some());
        assert!(effects.has_exact_coverage(tx.total_changes()));
    }
    // #1488 0035：合并 key 后逐收据核对仍读 events；丢 Noop 事件、key 下标错位、写数不符都拒绝。
    #[test]
    fn owner_set_merged_receipts_reject_dropped_misplaced_or_miscounted_event() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        with_material_access::<_, PublishError>(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            let before = recording.journal.lock().unwrap().events.len();
            put(&tx, 45);
            put(&tx, 45);
            put(&tx, 46);
            put(&tx, 45);
            let mut j = recording.journal.lock().unwrap();
            assert!(validate_stage_coverage(&j).is_ok());
            let noop = before + 1;
            assert!(!j.events[noop].inserted());
            let receipts = j.receipts_for_test();
            let tail = &receipts[before..];
            assert_eq!(tail.len(), 4, "四条收据逐条可还原");
            assert_eq!(
                tail.iter().map(|r| r.inserted).collect::<Vec<_>>(),
                [true, false, true, false]
            );
            assert_eq!(tail[0].digest, tail[3].digest);
            assert!(tail
                .windows(2)
                .all(|w| w[1].material_epoch == w[0].material_epoch + 1));
            drop(receipts);
            // 丢一条 Noop 事件：node 事件数与 material_epoch 不闭合。
            let saved = j.events.remove(noop);
            let error = validate_stage_coverage(&j).unwrap_err();
            assert!(error.contains("node多次事件序/实际写数不符"), "{error}");
            j.events.insert(noop, saved);
            // 丢尾部 Noop（其后再无首见 key 可错位）：只靠 node 事件数==material_epoch 闭合核抓住。
            let tail = j.events.pop().unwrap();
            assert!(!tail.inserted());
            let error = validate_stage_coverage(&j).unwrap_err();
            assert!(error.contains("node多次事件序/实际写数不符"), "{error}");
            j.events.push(tail);
            // key 下标越过首见序（future index，尚无对应 key）：拒绝。（0077 P15 订正：此处只测越序；
            // 「指向别的既有 key」未在本锁单独构造。）
            let future = Receipt::node(j.keys.len() + 1, 0, 0, false).unwrap();
            j.events[noop] = future;
            let error = validate_stage_coverage(&j).unwrap_err();
            assert!(error.contains("收据事件key序/归属不符"), "{error}");
            // inserted 与实际写数不符：拒绝。
            let forged = Receipt::node(saved.key(), 0, 0, true).unwrap();
            j.events[noop] = forged;
            let error = validate_stage_coverage(&j).unwrap_err();
            assert!(error.contains("node多次事件序/实际写数不符"), "{error}");
            j.events[noop] = saved;
            assert!(validate_stage_coverage(&j).is_ok());
            Ok(())
        })
        .unwrap();
        let effects = recording.finish(&tx, &mut [], 16 << 20).unwrap().unwrap();
        assert!(effects.has_exact_coverage(tx.total_changes()));
    }
    // #1488 0035：events 增长只为 events 付新块；触帽即 journal 溢出（与 keys 溢出共用 reserve_receipt）。
    #[test]
    fn owner_set_receipt_events_grow_alone_and_overflow_like_keys() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let result = with_material_access::<_, PublishError>(&tx, None, |material| {
            let material = material.unwrap();
            note_source_cut(material, (&state).into())?;
            put(&tx, 45);
            let (keys_cap, nodes_cap) = {
                let j = recording.journal.lock().unwrap();
                (j.keys.capacity(), j.nodes.capacity())
            };
            let mut grown = false;
            for _ in 0..64 {
                let (bytes, events_cap) = {
                    let j = recording.journal.lock().unwrap();
                    (j.bytes, j.events.capacity())
                };
                put(&tx, 45);
                let j = recording.journal.lock().unwrap();
                if j.events.capacity() != events_cap {
                    assert_eq!(
                        j.bytes - bytes,
                        vec_bytes::<Receipt>(j.events.capacity())
                            - vec_bytes::<Receipt>(events_cap),
                        "只为增长的events付新块"
                    );
                    assert!(j.peak >= bytes + vec_bytes::<Receipt>(j.events.capacity()));
                    grown = true;
                }
                assert_eq!(
                    (j.keys.capacity(), j.nodes.capacity()),
                    (keys_cap, nodes_cap)
                );
            }
            assert!(grown);
            let mut j = recording.journal.lock().unwrap();
            let fill = j.events.capacity() - j.events.len();
            let keys = j.keys.len();
            j.cap = j.bytes;
            drop(j);
            for _ in 0..=fill {
                put(&tx, 45);
            }
            let j = recording.journal.lock().unwrap();
            assert!(j.overflow, "events触帽须溢出");
            assert_eq!(j.keys.len(), keys, "同摘要收据不新增key");
            assert_eq!(
                (j.keys.capacity(), j.nodes.capacity()),
                (keys_cap, nodes_cap)
            );
            Ok(())
        });
        // 溢出是材料内首错：闭包不得吞下，effects 不得签发。
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("材料闭包未完整闭合或吞下失败"),
            "{error}"
        );
        assert!(!matches!(
            recording.finish(&tx, &mut [], 16 << 20),
            Ok(Some(_))
        ));
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
        let result = with_material_access::<_, PublishError>(&tx, None, |material| {
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
        assert!(with_material_access::<_, PublishError>(&tx, None, |_| Ok(())).is_err());
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
    #[test]
    fn owner_set_stage_drop_foreign_connection_and_omitted_business_row_reject() {
        for scenario in ["drop", "foreign", "omitted"] {
            let mut db = database();
            let tx = db.transaction().unwrap();
            tx.execute_batch(crate::v3::aux_index::SCHEMA).unwrap();
            let (recording, state) = set_recording(&tx);
            let result = with_material_access::<_, PublishError>(&tx, None, |material| {
                let material = material.unwrap();
                note_source_cut(material, (&state).into())?;
                match scenario {
                    "drop" => drop(begin_stage(&tx, StageKind::Witness).unwrap()),
                    "foreign" => {
                        let mut other = database();
                        let other_tx = other.transaction().unwrap();
                        assert!(begin_stage(&other_tx, StageKind::Witness).is_err());
                        assert!(
                            with_material_access::<_, PublishError>(&other_tx, None, |_| Ok(()))
                                .is_err()
                        );
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
        let result = with_material_access::<_, PublishError>(&tx, None, |material| {
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
        with_material_access::<_, PublishError>(&tx, None, |material| {
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
        assert!(
            with_material_access::<_, PublishError>(&tx, None, |m| note_source_cut(
                m.unwrap(),
                (&foreign.content_ref()).into()
            ))
            .is_err()
        );
        for (digest, tag, body) in memory.iter() {
            store.put(*digest, tag, body).unwrap();
        }
        assert!(with_material_access::<_, PublishError>(&tx, None, |_| Ok(())).is_err());
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
    #[test]
    fn owner_set_none_scope_shares_journal_cap_and_failure_cannot_reset() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let result = with_material_access::<_, PublishError>(&tx, None, |m| {
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
        assert!(with_material_access::<_, PublishError>(&tx, None, |_| Ok(())).is_err());
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
                with_material_access::<_, PublishError>(&tx, Some(scope), |m| {
                    let m = m.unwrap();
                    assert!(std::ptr::eq(m.work, scope), "必须是原work同对象");
                    note_source_cut(m, (&state).into())
                })
                .map_err(newchan_rust::s_content::ContextError::into_message)?;
                assert!(
                    parent.native_observation().0 >= cap,
                    "离开材料闭包不能提前返还cap"
                );
                recording
                    .finish(&tx, &mut [], 16 << 20)
                    .map_err(newchan_rust::s_content::ContextError::into_message)?
                    .ok_or_else(|| "缺effects".to_string())
            })
            .unwrap();
        let before = parent.live();
        let native_before = parent.native_observation().0;
        assert!(effects.set.is_some());
        // #1488 0059 主控裁定 B1（第六处裁改断言，
        // https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6287615661）：
        // 原断言（9d03c773ae）：`assert!(native_before > cap, "原codec retain仍在同一个父账中")`；
        //   drop(effects) 后 `assert!(parent.native_observation().0 > 0, "不得手工扣掉原codec保守retain")`。
        // v4 条款（control/1488-unified-budget-ownership-interface-contract-v4.md）：
        //   L9「许可随对象移动，在对象析构以后返还。没有生产unassigned余额、裸字节追认或同额补签。」
        //   L21「生产Budget::adopt(bytes)、ScratchParent::adopt_native(bytes, expected)…必须退出生产路径」
        //   L34「部分构造失败后逐个Drop，仅剩明确仍存活的真实父M」。
        // v4 下 codec 保守 retain 已退役：父账 native 只剩 effects 持有的 cap，返还后只剩根 M。
        // 保留原意：effects 持 cap 期间许可存活，Drop 只返还自己那份（下两行原断言不动）。
        assert_eq!(
            native_before, cap,
            "v4[B]：父账native只剩effects持有的cap许可，无codec retain"
        );
        drop(effects);
        assert_eq!(parent.live(), before - cap);
        assert_eq!(parent.native_observation().0, native_before - cap);
        assert_eq!(
            parent.live(),
            native::Budget::metadata_bytes(),
            "v4[B]：只返还cap本身；返还后父账仅剩根M，无保守retain"
        );
        drop(parent); // 根 M 随父账整个生命周期结束。
    }
    #[test]
    fn owner_set_failed_source_drops_new_cap_without_erasing_existing_codec_retain() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let cap = recording.journal.lock().unwrap().cap;
        let parent = native::Budget::new(64 << 20, 0).unwrap();
        let result = parent.codec_work().unwrap().run(|scope| {
            with_material_access::<_, PublishError>(&tx, Some(scope), |m| {
                note_source_cut(m.unwrap(), (&state).into())?;
                Err::<(), PublishError>("TestOnlyAfterCutRejected".into())
            })
        });
        assert!(result.is_err());
        let before = parent.live();
        let native_before = parent.native_observation().0;
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
        assert_eq!(parent.live(), before - cap, "错误Drop归还新增cap全部余额");
        assert_eq!(parent.native_observation().0, native_before - cap);
        // #1488 0059 主控裁定 B2（第七处裁改断言，
        // https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6287615661）：
        // 原断言（9d03c773ae）：`assert!(parent.native_observation().0 > 0, "原codec retain仍由原父账持有")`。
        // v4 条款（control/1488-unified-budget-ownership-interface-contract-v4.md）：
        //   L9「许可随对象移动，在对象析构以后返还。没有生产unassigned余额、裸字节追认或同额补签。」
        //   L34「部分构造失败后逐个Drop，仅剩明确仍存活的真实父M；禁止unassigned或已分配后adopt追认。」
        // 失败路径的 codec 临时件已析构返还，父账不再有 codec retain 残留，仅剩根 M。
        // 保留原意：上一行“错误Drop归还新增cap全部余额”不动，且不得多还（== M 而非 < M）。
        assert_eq!(
            parent.live(),
            native::Budget::metadata_bytes(),
            "v4[B]：失败Drop只归还新增cap；父账无codec retain残留，仅剩根M"
        );
    }
    #[test]
    fn owner_set_actual_transaction_progress_cancellation_is_preserved() {
        let mut db = database();
        let tx = db.transaction().unwrap();
        let (recording, state) = set_recording(&tx);
        let _ = tx.progress_handler(1, Some(|| true));
        assert!(
            with_material_access::<_, PublishError>(&tx, None, |m| note_source_cut(
                m.unwrap(),
                (&state).into()
            ))
            .is_err()
        );
        let _ = tx.progress_handler(0, None::<fn() -> bool>);
        assert!(
            with_material_access::<_, PublishError>(&tx, None, |_| Ok(())).is_err(),
            "取消后不能重建成功scope"
        );
        assert!(recording.finish(&tx, &mut [], 16 << 20).is_err());
    }
}

#[cfg(all(test, feature = "s_incremental"))]
mod e_owner_move_tests {
    use super::*;
    #[test]
    fn e_stage_into_row_keeps_charge_after_header_drop() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let budget = native::Budget::new(1024 * 1024, 0).unwrap();
        let (rows, _) = ExactRows::read_optional_bounded_in_parent(
            &tx,
            "SELECT 1,X'010203','keep'",
            65536,
            Some(&budget),
        )
        .unwrap()
        .unwrap();
        let row = StageOriginals { value: Some(rows) }.into_row();
        // #1488 0059 主控裁定 B5（第十处裁改断言，
        // https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6287615661）：
        // 原断言（ea41d4dfd1）：`assert_eq!(budget.live(), row.heap_bytes())`；drop(row) 后 `assert_eq!(budget.live(), 0)`。
        // v4 条款（control/1488-unified-budget-ownership-interface-contract-v4.md）：
        //   L10「在分配根Ledger控制块前覆盖实际metadata M并验证A/总帽。」
        //   L34「部分构造失败后逐个Drop，仅剩明确仍存活的真实父M」。
        // v4 根 Ledger 把自身 metadata M 计入 live，两处只多计根 M；行费精确性不降。
        assert_eq!(
            budget.live(),
            row.heap_bytes() + native::Budget::metadata_bytes(),
            "v4[B]"
        );
        assert_eq!(manual_row_bytes(&row), 0);
        drop(row);
        assert_eq!(budget.live(), native::Budget::metadata_bytes(), "v4[B]");
    }
    #[test]
    fn e_apply_native_insert_is_ordered_and_releases_only_manual_outer_charge() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE meta(key TEXT,value TEXT); INSERT INTO meta(rowid,key,value) VALUES(1,'one','a'),(3,'three','c');").unwrap();
        let tx = conn.transaction().unwrap();
        let budget = native::Budget::new(1024 * 1024, 0).unwrap();
        let original = ExactRows::read_optional_bounded_in_parent(
            &tx,
            Table::Meta.original_sql(),
            65536,
            Some(&budget),
        )
        .unwrap()
        .unwrap()
        .0;
        let row = StageOriginals {
            value: Some(
                ExactRows::read_optional_bounded_in_parent(
                    &tx,
                    "SELECT 2,'two','b'",
                    65536,
                    Some(&budget),
                )
                .unwrap()
                .unwrap()
                .0,
            ),
        }
        .into_row();
        let changes = vec![Change {
            table: Table::Meta,
            rowid: 2,
            slot: None,
            old: None,
            new: Some(row),
        }];
        let bytes = vec_bytes::<Change>(changes.capacity());
        let changes_charge = Some(budget.reserve(bytes).unwrap().finish(bytes));
        let mut changes = native::Tracked::new(changes);
        changes.attach(changes_charge);
        let effects = Effects {
            set: None,
            changes,
            operation: Operation::Publish,
            peak: budget.peak(),
            bytes,
            covered_total: None,
            shared_parent: true,
        };
        let mut originals = [original];
        let mut original_charge = originals[0].heap_bytes();
        effects
            .apply(&mut originals, &budget, &mut original_charge)
            .unwrap();
        assert_eq!(
            originals[0]
                .rows
                .iter()
                .map(|r| integer(&r[0]).unwrap())
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        // #1488 0059 主控裁定 B3（第八处裁改断言，
        // https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6287615661）：
        // 原断言（ea41d4dfd1）：`assert_eq!(budget.live(), originals[0].heap_bytes())`；drop(originals) 后 `assert_eq!(budget.live(), 0)`。
        // v4 条款（control/1488-unified-budget-ownership-interface-contract-v4.md）：
        //   L10「在分配根Ledger控制块前覆盖实际metadata M并验证A/总帽。」
        //   L34「部分构造失败后逐个Drop，仅剩明确仍存活的真实父M」。
        // v4 根 Ledger 把自身 metadata M 计入 live，两处只多计根 M；行费精确性不降。
        assert_eq!(
            budget.live(),
            originals[0].heap_bytes() + native::Budget::metadata_bytes(),
            "v4[B]"
        );
        assert_eq!(original_charge, originals[0].heap_bytes());
        drop(originals);
        assert_eq!(budget.live(), native::Budget::metadata_bytes(), "v4[B]");
    }
    #[test]
    fn e_apply_native_replace_and_delete_keep_remaining_rows_owned() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE meta(key TEXT,value TEXT); INSERT INTO meta(rowid,key,value) VALUES(1,'one','a'),(3,'three','c');").unwrap();
        let tx = conn.transaction().unwrap();
        let budget = native::Budget::new(1024 * 1024, 0).unwrap();
        let mut originals = [ExactRows::read_optional_bounded_in_parent(
            &tx,
            Table::Meta.original_sql(),
            65536,
            Some(&budget),
        )
        .unwrap()
        .unwrap()
        .0];
        let mut original_charge = originals[0].heap_bytes();
        for (slot, rowid, replacement) in [(0, 1, true), (1, 3, false)] {
            let old = std::mem::take(&mut originals[0].rows[slot]);
            let new = if replacement {
                Some(
                    StageOriginals {
                        value: Some(
                            ExactRows::read_optional_bounded_in_parent(
                                &tx,
                                "SELECT 1,'updated','b'",
                                65536,
                                Some(&budget),
                            )
                            .unwrap()
                            .unwrap()
                            .0,
                        ),
                    }
                    .into_row(),
                )
            } else {
                None
            };
            let changes = vec![Change {
                table: Table::Meta,
                rowid,
                slot: Some(slot),
                old: Some(old),
                new,
            }];
            let bytes = vec_bytes::<Change>(changes.capacity());
            let changes_charge = Some(budget.reserve(bytes).unwrap().finish(bytes));
            let mut changes = native::Tracked::new(changes);
            changes.attach(changes_charge);
            let effects = Effects {
                set: None,
                changes,
                operation: Operation::Publish,
                peak: budget.peak(),
                bytes,
                covered_total: None,
                shared_parent: true,
            };
            effects
                .apply(&mut originals, &budget, &mut original_charge)
                .unwrap();
            // #1488 0059 主控裁定 B4（第九处裁改断言，
            // https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6287615661）：
            // 原断言（ea41d4dfd1）：`assert_eq!(budget.live(), originals[0].heap_bytes())`；drop(originals) 后 `assert_eq!(budget.live(), 0)`。
            // v4 条款（control/1488-unified-budget-ownership-interface-contract-v4.md）：
            //   L10「在分配根Ledger控制块前覆盖实际metadata M并验证A/总帽。」
            //   L34「部分构造失败后逐个Drop，仅剩明确仍存活的真实父M」。
            // v4 根 Ledger 把自身 metadata M 计入 live，两处只多计根 M；行费精确性不降。
            assert_eq!(
                budget.live(),
                originals[0].heap_bytes() + native::Budget::metadata_bytes(),
                "v4[B]"
            );
            assert_eq!(original_charge, originals[0].heap_bytes());
        }
        assert_eq!(originals[0].rows.len(), 1);
        assert_eq!(text(&originals[0].rows[0][1]).unwrap(), "updated");
        drop(originals);
        assert_eq!(budget.live(), native::Budget::metadata_bytes(), "v4[B]");
    }
}

#[cfg(test)]
// #1488 U2：原 adopt_row（原 1750）改为只核本父出生身份；正/负锁。
mod u2_row_owned_tests {
    use super::*;
    #[test]
    fn u2_effects_row_owned_accepts_cells_born_under_parent() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let budget = native::Budget::new(1024 * 1024, 0).unwrap();
        let (rows, _) = ExactRows::read_optional_bounded_in_parent(
            &tx,
            "SELECT 1,X'010203','keep'",
            65536,
            Some(&budget),
        )
        .unwrap()
        .unwrap();
        let before = (budget.live(), budget.peak(), budget.remaining());
        require_row_owned(&budget, &rows.rows[0]).unwrap();
        assert_eq!((budget.live(), budget.peak(), budget.remaining()), before);
    }
    #[test]
    fn u2_effects_row_owned_rejects_plain_cell_or_foreign_parent_without_adopt() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let owner = native::Budget::new(1024 * 1024, 0).unwrap();
        let (rows, _) =
            ExactRows::read_optional_bounded_in_parent(&tx, "SELECT 1,'keep'", 65536, Some(&owner))
                .unwrap()
                .unwrap();
        let mut row = sc::CodecVec::from_plain(vec![SqlCell::Text(sc::CodecVec::from_plain(
            b"plain".to_vec(),
        ))]);
        // 遗留 moved_in 足以追认整行；新口径必须拒绝且不改账、不绑定。
        let legacy = native::Budget::new(1024 * 1024, 4096).unwrap();
        let before = (legacy.live(), legacy.peak(), legacy.remaining());
        assert!(require_row_owned(&legacy, &row).is_err());
        assert_eq!((legacy.live(), legacy.peak(), legacy.remaining()), before);
        assert!(!row.is_native_bound());
        if let Some(SqlCell::Text(bytes)) = row.iter_mut().next() {
            assert!(!bytes.is_native_bound());
        }
        let other = native::Budget::new(1024 * 1024, 0).unwrap();
        assert!(require_row_owned(&other, &rows.rows[0]).is_err());
        require_row_owned(&owner, &rows.rows[0]).unwrap();
    }
}

fn publish_err(error: impl Into<PublishError>) -> PublishError {
    error.into().context("S3DriverInvalid：")
}

fn publish_stale(detail: &str) -> PublishError {
    construction_state::stale(detail).into()
}
