//! #1394 Native test-only：仅本事务新节点暂存；没有旧节点删除或跨事务 keeper。
use super::*;
use newchan_rust::s_content::{self as sc, NodeDigest, NodeSink, NodeSource};
use newchan_rust::session_protocol::strict_json;
use rusqlite::Transaction;
use sc::capture_allocation::scratch::{NativeReservation, ScratchVec, WorkScope};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

// 字段顺序保证正文先释放，再退还对应许可。
struct ChargedNode {
    tag: u8,
    body: Vec<u8>,
    _body_permit: Option<NativeReservation>,
}

pub(crate) enum FlushCut<'a> {
    Source(sc::RefView<'a>),
    Row(&'a [NodeDigest]),
    Final(&'a [NodeDigest]),
}

// 工作队列和已访问集合分别持有真实容器的许可，不能只给正文记账。
enum CutQueue {
    Plain(Vec<NodeDigest>),
    Admitted(ScratchVec<NodeDigest>),
}
impl CutQueue {
    fn new(work: Option<&WorkScope>) -> Self {
        match work {
            Some(w) => Self::Admitted(w.vector()),
            None => Self::Plain(Vec::new()),
        }
    }
    fn push(&mut self, digest: NodeDigest) -> Result<(), sc::CodecError> {
        match self {
            Self::Plain(v) => {
                v.push(digest);
                Ok(())
            }
            Self::Admitted(v) => v.try_push(digest),
        }
    }
    fn get(&self, index: usize) -> Option<NodeDigest> {
        match self {
            Self::Plain(v) => v.get(index).copied(),
            Self::Admitted(v) => v.get(index).copied(),
        }
    }
}
struct CutSeen {
    nodes: BTreeSet<NodeDigest>,
    _permit: Option<NativeReservation>,
}

pub(super) struct StageOwner<'w> {
    connection: usize,
    nodes: RefCell<BTreeMap<NodeDigest, ChargedNode>>,
    tree_permit: RefCell<Option<NativeReservation>>,
    tree_high_water: std::cell::Cell<usize>,
    work: Option<&'w WorkScope>,
    identity: Option<crate::v3::audit_owner::PublicationStageIdentity>,
    poisoned: std::cell::Cell<bool>,
    live_body_capacity: std::cell::Cell<usize>,
    accepted: std::cell::Cell<usize>,
    peak_body_capacity: std::cell::Cell<usize>,
    puts: std::cell::Cell<u64>,
    kept: std::cell::Cell<usize>,
    flushed: std::cell::Cell<usize>,
    heap_sample_max: std::cell::Cell<usize>,
    heap_samples: std::cell::Cell<usize>,
    heap_start: usize,
}
impl<'w> StageOwner<'w> {
    pub(super) fn new(tx: &Transaction<'_>) -> Self {
        // SQLite handle在Transaction按值传递时不变；owner由Runtime::commit单次栈上持有。
        let heap_start = heap_sample();
        Self {
            connection: unsafe { tx.handle() } as usize,
            nodes: Default::default(),
            tree_permit: Default::default(),
            tree_high_water: Default::default(),
            work: None,
            identity: None,
            poisoned: Default::default(),
            live_body_capacity: Default::default(),
            accepted: Default::default(),
            peak_body_capacity: Default::default(),
            puts: Default::default(),
            kept: Default::default(),
            flushed: Default::default(),
            heap_sample_max: std::cell::Cell::new(heap_start),
            heap_samples: std::cell::Cell::new(1),
            heap_start,
        }
    }
    pub(super) fn new_admitted(
        tx: &Transaction<'_>,
        parent: &crate::v3::audit_owner::PublicationScope<'w>,
    ) -> Result<Self, String> {
        parent.check_transaction(tx)?;
        let mut owner = Self::new(tx);
        owner.identity = Some(parent.stage_identity(tx)?);
        owner.work = Some(parent.work());
        Ok(owner)
    }
    pub(super) fn work(&self) -> Option<&'w WorkScope> {
        self.work
    }
    fn check_live(&self) -> Result<(), sc::CodecError> {
        if self.poisoned.get() {
            return Err(sc::CodecError::new(
                sc::code::SOURCE,
                "暂存owner已失败，不可重试",
            ));
        }
        Ok(())
    }
    pub(super) fn check_transaction(&self, tx: &Transaction<'_>) -> Result<(), sc::CodecError> {
        self.check_live()?;
        if let (Some(identity), Some(work)) = (&self.identity, self.work) {
            identity
                .check(tx, work)
                .map_err(|e| sc::CodecError::new(sc::code::SOURCE, e))?;
        }
        if self.connection != unsafe { tx.handle() } as usize {
            return Err(sc::CodecError::new(
                sc::code::SOURCE,
                "暂存owner不属于当前事务连接",
            ));
        }
        if tx.is_autocommit() {
            return Err(sc::CodecError::new(
                sc::code::SOURCE,
                "暂存必须属于尚未完成的事务",
            ));
        }
        Ok(())
    }
    pub(super) fn reject_after_error(&self) {
        if self.work.is_some() {
            self.poisoned.set(true);
        }
    }
    pub(super) fn read_scoped(
        &self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        let result = self.read_scoped_inner(d);
        if result.is_err() {
            self.reject_after_error();
        }
        result
    }
    fn read_scoped_inner(
        &self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        self.check_live()?;
        let nodes = self.nodes.borrow();
        let Some(node) = nodes.get(d) else {
            return Ok(None);
        };
        let (body, permit) = content_store::native_body_reserved(self.work, &node.body)?;
        let frame = sc::job::CheckedFrame::checked(d, node.tag, body)?;
        Ok(Some(sc::job::ScopedFrame::new(frame, permit)))
    }
    pub(super) fn check_original(
        &self,
        d: &NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<(), sc::CodecError> {
        self.check_live()?;
        if let Some(node) = self.nodes.borrow().get(d) {
            if node.tag != tag || node.body != body {
                self.poisoned.set(true);
                return Err(sc::CodecError::new(
                    sc::code::NODE_DIGEST,
                    "SQL与暂存同摘要异原件",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn put_new(&self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
        let result = self.put_new_inner(d, t, b);
        if result.is_err() {
            self.reject_after_error();
        }
        result
    }
    fn put_new_inner(&self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
        self.check_live()?;
        self.puts.set(self.puts.get() + 1);
        let mut nodes = self.nodes.borrow_mut();
        if let Some(node) = nodes.get(&d) {
            if node.tag != t || node.body.as_slice() != b {
                return Err(sc::CodecError::new(
                    sc::code::NODE_DIGEST,
                    "同暂存摘要异原件，拒绝",
                ));
            }
        } else {
            let next = nodes
                .len()
                .checked_add(1)
                .ok_or_else(|| sc::CodecError::new(sc::code::SOURCE, "暂存节点数溢出"))?;
            let high_water = next.max(self.tree_high_water.get());
            let tree = content_store::native_tree_storage::<NodeDigest, ChargedNode>(
                self.work, high_water,
            )?;
            let (body, permit) = content_store::native_body_reserved(self.work, b)?;
            let capacity = body.capacity();
            nodes.insert(
                d,
                ChargedNode {
                    tag: t,
                    body,
                    _body_permit: permit,
                },
            );
            *self.tree_permit.borrow_mut() = tree;
            self.tree_high_water.set(high_water);
            self.live_body_capacity
                .set(self.live_body_capacity.get() + capacity);
            self.peak_body_capacity.set(
                self.peak_body_capacity
                    .get()
                    .max(self.live_body_capacity.get()),
            );
            self.accepted.set(self.accepted.get() + 1);
        }
        Ok(())
    }
    fn sample_heap_phase(&self) {
        self.heap_sample_max
            .set(self.heap_sample_max.get().max(heap_sample()));
        self.heap_samples.set(self.heap_samples.get() + 1);
    }
    pub(super) fn report(&self) -> Value {
        self.sample_heap_phase();
        let nodes = self.nodes.borrow();
        json!({"new_unique_accepted":self.accepted.get(),"new_put_visits":self.puts.get(),"actual_body_capacity_peak_bytes":self.peak_body_capacity.get(),"body_length_bytes":nodes.values().map(|n|n.body.len()).sum::<usize>(),"digest_original_bytes":nodes.len()*32,"tuple_inline_size_bytes":nodes.len()*std::mem::size_of::<(NodeDigest,ChargedNode)>(),"BTree_allocations_and_strict_heap_peak":"UNMEASURED; body capacity is not total heap","process_malloc_zones_start_bytes":self.heap_start,"process_malloc_zones_sampled_max_bytes":self.heap_sample_max.get(),"process_malloc_zones_sample_count":self.heap_samples.get(),"heap_scope":"actual process malloc zones sampled at owner creation, flush entry and report; not staging-exclusive or strict heap peak","final_union_new_kept":self.kept.get(),"new_physical_inserts":self.flushed.get(),"physical_INSERT_count_basis":"node table row delta across final flush, checked on success and error; successful put alone does not imply INSERT","new_accepted_not_flushed":self.accepted.get().saturating_sub(self.flushed.get()),"Job_node_acceptance_cumulative":"existing interfaces not fully Job-wired; staging visits are diagnostic, never decrement upper-layer access charges","whole_Job_or_max_store_qualified":false,"physical_INSERT_error_timing":"delayed until final flush; not equivalent to eager injection timing"})
    }
    /// #1488：只遍历本次cut显式根；不扫描历史控制表，也不把overlay当材料证明。
    pub(crate) fn flush_cut(&self, tx: &Transaction<'_>, cut: FlushCut<'_>) -> Result<(), String> {
        let result = self.flush_cut_inner(tx, cut);
        if result.is_err() {
            self.poisoned.set(true);
        }
        result
    }
    fn flush_cut_inner(&self, tx: &Transaction<'_>, cut: FlushCut<'_>) -> Result<(), String> {
        self.check_transaction(tx).map_err(|e| e.to_string())?;
        self.sample_heap_phase();
        let uo1488_kind = match &cut {
            FlushCut::Source(_) => "Source",
            FlushCut::Row(_) => "Row",
            FlushCut::Final(_) => "Final",
        };
        let mut queue = CutQueue::new(self.work);
        match cut {
            FlushCut::Source(root) => queue
                .push(root.digest().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?,
            FlushCut::Row(roots) | FlushCut::Final(roots) => {
                for root in roots {
                    queue.push(*root).map_err(|e| e.to_string())?;
                }
            }
        }
        let mut seen = CutSeen {
            nodes: BTreeSet::new(),
            _permit: None,
        };
        let mut store = content_store::TxStore::new_with_stage(tx, Some(self))
            .map_err(|e| e.to_string())?
            .with_work(self.work);
        let mut cursor = 0;
        while let Some(d) = queue.get(cursor) {
            cursor += 1;
            if seen.nodes.contains(&d) {
                continue;
            }
            let next = seen.nodes.len().checked_add(1).ok_or("cut节点数溢出")?;
            let permit = content_store::native_tree_storage::<NodeDigest, ()>(self.work, next)
                .map_err(|e| e.to_string())?;
            seen.nodes.insert(d);
            seen._permit = permit;
            let frame = store
                .checked_read_scoped(&d)
                .map_err(|e| e.to_string())?
                .ok_or("cut闭包缺节点")?;
            let b = frame.body();
            let offsets: &[usize] = match frame.tag() {
                0 | 1 | 2 | 8 | 17 if b.is_empty() => &[],
                3 | 4 | 5 | 6 if b.len() == 40 => &[8],
                7 | 18 if b.len() == 32 => &[0],
                19 if b.len() == 72 => &[8, 40],
                20 if b.len() == 64 => &[0, 32],
                16 if !b.is_empty() && b.len() <= 4096 => &[],
                _ => return Err("cut节点tag或body长度错误".into()),
            };
            for offset in offsets {
                queue
                    .push(
                        b[*offset..*offset + 32]
                            .try_into()
                            .map_err(|_| "cut子边长度")?,
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        drop(store);
        let mut eager = content_store::TxStore::new(tx)
            .map_err(|e| e.to_string())?
            .with_work(self.work);
        for digest in &seen.nodes {
            // 先保留原件及许可直到真实SQL put成功；不提前释放尚在暂存的正文。
            let present = {
                let nodes = self.nodes.borrow();
                if let Some(node) = nodes.get(digest) {
                    let old = eager
                        .checked_read_scoped(digest)
                        .map_err(|e| e.to_string())?;
                    if let Some(old) = old {
                        if old.tag() != node.tag || old.body() != node.body {
                            return Err("cut SQL与暂存原件冲突".into());
                        }
                    } else {
                        let before = tx.total_changes();
                        eager
                            .put(*digest, node.tag, &node.body)
                            .map_err(|e| e.to_string())?;
                        match tx.total_changes().checked_sub(before) {
                            Some(0) => {}
                            Some(1) if tx.changes() == 1 => {
                                eprintln!(
                                    "UO1488_FLUSH kind={uo1488_kind} d={}",
                                    digest.iter().take(16).map(|b| format!("{b:02x}")).collect::<String>()
                                );
                                self.flushed.set(self.flushed.get() + 1)
                            }
                            _ => return Err("cut节点INSERT出现非目标实际写入".into()),
                        }
                    }
                    true
                } else {
                    false
                }
            };
            if present {
                let node = self
                    .nodes
                    .borrow_mut()
                    .remove(digest)
                    .ok_or("cut暂存原件消失")?;
                self.live_body_capacity
                    .set(self.live_body_capacity.get() - node.body.capacity());
                drop(node); // body字段先于_body_permit；树保留费直到整树销毁。
                self.kept.set(self.kept.get() + 1);
            }
        }
        Ok(())
    }
    pub(super) fn flush(&self, tx: &Transaction<'_>, runtime: &[NodeDigest]) -> Result<(), String> {
        self.check_transaction(tx).map_err(|e| e.to_string())?;
        self.sample_heap_phase();
        let mut roots = runtime.to_vec();
        for (table, cols) in [
            (
                "s_content_publications",
                vec!["manifest_root", "snapshot_root", "delta_root"],
            ),
            (
                "s_content_objects",
                vec![
                    "generated_root",
                    "internal_root",
                    "immutable_root",
                    "birth_wire_root",
                ],
            ),
            ("s_content_witnesses", vec!["generated_root", "wire_root"]),
            ("s_content_relations", vec!["generated_root"]),
            (
                "s_content_observations",
                vec!["generated_root", "birth_wire_root"],
            ),
        ] {
            for col in cols {
                let mut s = tx
                    .prepare(&format!("SELECT {col} FROM {table}"))
                    .map_err(|e| e.to_string())?;
                for row in s
                    .query_map([], |r| r.get::<_, Vec<u8>>(0))
                    .map_err(|e| e.to_string())?
                {
                    roots.push(
                        row.map_err(|e| e.to_string())?
                            .try_into()
                            .map_err(|_| "持久root非32B")?,
                    );
                }
            }
        }
        for (table, col) in [
            ("structure_deltas", "delta_json"),
            ("structure_deltas", "catalog_evidence_json"),
            ("catalog", "evidence_json"),
        ] {
            let mut s = tx
                .prepare(&format!("SELECT {col} FROM {table}"))
                .map_err(|e| e.to_string())?;
            for row in s
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?
            {
                let v = strict_json(row.map_err(|e| e.to_string())?.as_bytes())?;
                if table == "catalog" && v == json!({}) {
                    continue;
                }
                roots.push(
                    sc::TransportRef::from_json_view((&v).into())
                        .map_err(|e| e.to_string())?
                        .digest()
                        .map_err(|e| e.to_string())?,
                );
            }
        }
        // 当前meta仍仅普通scope/profile/catalog定义与控制身份，没有声明ContentRef的新增列。
        let mut store =
            content_store::TxStore::new_with_stage(tx, Some(self)).map_err(|e| e.to_string())?;
        let mut seen = BTreeSet::new();
        let mut todo = roots;
        while let Some(d) = todo.pop() {
            if !seen.insert(d) {
                continue;
            }
            let frame = store
                .checked_read(&d)
                .map_err(|e| e.to_string())?
                .ok_or("联合闭包缺节点")?;
            let t = frame.tag();
            let b = frame.body();
            let offsets: &[usize] = match t {
                0 | 1 | 2 | 8 | 17 => {
                    if !b.is_empty() {
                        return Err("empty节点body错误".into());
                    }
                    &[]
                }
                3 | 4 | 5 | 6 => {
                    if b.len() != 40 {
                        return Err("值节点header错误".into());
                    }
                    &[8]
                }
                7 | 18 => {
                    if b.len() != 32 {
                        return Err("单边body错误".into());
                    }
                    &[0]
                }
                19 => {
                    if b.len() != 72 {
                        return Err("seq分支body错误".into());
                    }
                    &[8, 40]
                }
                20 => {
                    if b.len() != 64 {
                        return Err("field body错误".into());
                    }
                    &[0, 32]
                }
                16 => {
                    if b.is_empty() || b.len() > 4096 {
                        return Err("chunk长度错误".into());
                    }
                    &[]
                }
                _ => return Err("未知tag，拒绝flush".into()),
            };
            for &offset in offsets {
                todo.push(b[offset..offset + 32].try_into().unwrap());
            }
        }
        let nodes = self.nodes.borrow();
        let needed = nodes.keys().filter(|d| seen.contains(*d)).count();
        self.kept.set(needed);
        let initial: i64 = tx
            .query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let mut eager = content_store::TxStore::new(tx).map_err(|e| e.to_string())?;
        let result = nodes
            .iter()
            .filter(|(d, _)| seen.contains(*d))
            .try_for_each(|(d, node)| {
                eager
                    .put(*d, node.tag, &node.body)
                    .map_err(|e| e.to_string())
            });
        // #1394：只在批次边界核对；INSERT中途失败也保留回滚前真实新增数。
        // TxStore::put可能只验证已有原件并返回Ok，不能按Ok次数计INSERT。
        let count: i64 = tx
            .query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
            .map_err(|e| match &result {
                Err(original) => format!("{original}; flush计数核对失败：{e}"),
                Ok(()) => e.to_string(),
            })?;
        self.flushed
            .set(usize::try_from(count - initial).map_err(|e| match &result {
                Err(original) => format!("{original}; flush新增行数无效：{e}"),
                Ok(()) => format!("flush新增行数无效：{e}"),
            })?);
        result
    }
}

#[cfg(target_os = "macos")]
fn heap_sample() -> usize {
    #[repr(C)]
    #[derive(Default)]
    struct Stats {
        blocks: u32,
        size: usize,
        max: usize,
        allocated: usize,
    }
    unsafe extern "C" {
        fn malloc_zone_statistics(zone: *const std::ffi::c_void, stats: *mut Stats);
    }
    let mut s = Stats::default();
    unsafe {
        malloc_zone_statistics(std::ptr::null(), &mut s);
    }
    s.size
}
#[cfg(not(target_os = "macos"))]
fn heap_sample() -> usize {
    0
}

#[test]
fn staged_nodes_capacity_counts_unique_owned_vecs_without_per_put_heap_sampling() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let tx = conn.transaction().unwrap();
    let owner = StageOwner::new(&tx);
    for len in [0, 1, 17, 4096] {
        let body = vec![len as u8; len];
        let digest = sc::frame(sc::tag::BYTES_CHUNK, &body);
        owner.put_new(digest, sc::tag::BYTES_CHUNK, &body).unwrap();
        let actual: usize = owner
            .nodes
            .borrow()
            .values()
            .map(|node| node.body.capacity())
            .sum();
        assert_eq!(owner.peak_body_capacity.get(), actual);
        for _ in 0..128 {
            owner.put_new(digest, sc::tag::BYTES_CHUNK, &body).unwrap();
        }
        assert_eq!(owner.peak_body_capacity.get(), actual);
        assert!(owner.put_new(digest, sc::tag::NULL, b"conflict").is_err());
        assert_eq!(owner.peak_body_capacity.get(), actual);
    }
    assert_eq!(owner.nodes.borrow().len(), 4);
    assert_eq!(owner.heap_samples.get(), 1);
    let report = owner.report();
    assert_eq!(report["process_malloc_zones_sample_count"], json!(2));
    assert!(report["heap_scope"]
        .as_str()
        .unwrap()
        .contains("not staging-exclusive or strict heap peak"));
    assert_eq!(report["whole_Job_or_max_store_qualified"], json!(false));
}

#[test]
fn staged_nodes_flush_counts_rows_and_traverses_old_parent_to_new_child() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL);
         INSERT INTO s_protocol_meta VALUES(1,'s-session/3');
         CREATE TABLE s_content_publications(manifest_root BLOB,snapshot_root BLOB,delta_root BLOB);
         CREATE TABLE s_content_objects(generated_root BLOB,internal_root BLOB,immutable_root BLOB,birth_wire_root BLOB);
         CREATE TABLE s_content_witnesses(generated_root BLOB,wire_root BLOB);
         CREATE TABLE s_content_relations(generated_root BLOB);
         CREATE TABLE s_content_observations(generated_root BLOB,birth_wire_root BLOB);
         CREATE TABLE structure_deltas(delta_json TEXT,catalog_evidence_json TEXT);
         CREATE TABLE catalog(evidence_json TEXT);",
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    content_store::init_schema(&tx).unwrap();
    let owner = StageOwner::new(&tx);
    let child = sc::frame(sc::tag::BYTES_CHUNK, b"new-child");
    let parent = sc::frame(sc::tag::SEQ_LEAF, &child);
    let already = sc::frame(sc::tag::BYTES_CHUNK, b"already-inserted");
    let orphan = sc::frame(sc::tag::BYTES_CHUNK, b"unreferenced");
    owner
        .put_new(child, sc::tag::BYTES_CHUNK, b"new-child")
        .unwrap();
    owner
        .put_new(already, sc::tag::BYTES_CHUNK, b"already-inserted")
        .unwrap();
    owner
        .put_new(orphan, sc::tag::BYTES_CHUNK, b"unreferenced")
        .unwrap();
    let mut eager = content_store::TxStore::new(&tx).unwrap();
    // #1394：旧孤立parent可以引用本事务new child，mark不能停在旧节点。
    eager.put(parent, sc::tag::SEQ_LEAF, &child).unwrap();
    eager
        .put(already, sc::tag::BYTES_CHUNK, b"already-inserted")
        .unwrap();
    owner.flush(&tx, &[parent, already]).unwrap();
    let report = owner.report();
    assert_eq!(report["new_unique_accepted"], json!(3));
    assert_eq!(report["final_union_new_kept"], json!(2));
    assert_eq!(report["new_physical_inserts"], json!(1));
    assert_eq!(report["new_accepted_not_flushed"], json!(2));
    assert_eq!(report["process_malloc_zones_sample_count"], json!(3));
    assert!(eager.checked_read(&child).unwrap().is_some());
    assert!(eager.checked_read(&parent).unwrap().is_some());
    assert!(eager.checked_read(&already).unwrap().is_some());
    assert!(eager.checked_read(&orphan).unwrap().is_none());
    owner.flush(&tx, &[parent, already]).unwrap();
    assert_eq!(owner.report()["new_physical_inserts"], json!(0));
}

#[test]
fn staged_nodes_preserve_frame_and_current_SQL_error_gates() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL);INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
    let tx = conn.transaction().unwrap();
    content_store::init_schema(&tx).unwrap();
    let owner = StageOwner::new(&tx);
    let mut store = content_store::TxStore::new_with_stage(&tx, Some(&owner)).unwrap();
    let digest = sc::frame(sc::tag::BYTES_CHUNK, b"native-probe");
    assert!(store
        .put([9; 32], sc::tag::BYTES_CHUNK, b"native-probe")
        .is_err());
    assert_eq!(owner.report()["new_unique_accepted"], json!(0));
    store
        .put(digest, sc::tag::BYTES_CHUNK, b"native-probe")
        .unwrap();
    store
        .put(digest, sc::tag::BYTES_CHUNK, b"native-probe")
        .unwrap();
    assert_eq!(owner.report()["new_unique_accepted"], json!(1));
    assert_eq!(
        tx.query_row("SELECT count(*) FROM s_content_nodes", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        store.checked_read(&digest).unwrap().unwrap().body(),
        b"native-probe"
    );
    tx.execute_batch("ALTER TABLE s_content_nodes RENAME TO test_hidden_nodes")
        .unwrap();
    assert!(store.checked_read(&digest).is_err()); // 不能用已暂存原件吞旧SQL读取失败。
}

// 只锁S存储层费用，不冒充PublicationScope/owner资格测试。
#[test]
fn staged_scoped_copies_hold_distinct_permits_until_payload_drop() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let tx = conn.transaction().unwrap();
    let budget = crate::v3::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
    budget
        .codec_work()
        .unwrap()
        .run(|work| -> Result<(), String> {
            let baseline = work.snapshot().live;
            let mut stage = StageOwner::new(&tx);
            stage.work = Some(work); // 测试模块内部装配；不签发publication资格。
            let bytes = b"independent-copy";
            let digest = sc::frame(sc::tag::BYTES_CHUNK, bytes);
            stage.put_new(digest, sc::tag::BYTES_CHUNK, bytes).unwrap();
            let stored = work.snapshot().live;
            assert!(stored > baseline + bytes.len()); // 含暂存树上界。
            let first = stage.read_scoped(&digest).unwrap().unwrap();
            assert_eq!(work.snapshot().live, stored + bytes.len());
            let second = stage.read_scoped(&digest).unwrap().unwrap();
            assert_eq!(work.snapshot().live, stored + 2 * bytes.len());
            drop(first);
            assert_eq!(work.snapshot().live, stored + bytes.len());
            assert_eq!(second.body(), bytes);
            drop(stage); // 返回帧的许可不能依附StageOwner存储许可。
            assert_eq!(work.snapshot().live, baseline + bytes.len());
            assert_eq!(second.body(), bytes);
            drop(second);
            assert_eq!(work.snapshot().live, baseline);
            Ok(())
        })
        .unwrap();
}

#[test]
fn staged_selective_cut_follows_sql_parent_and_keeps_unselected_payload_charged() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL);INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
    let tx = conn.transaction().unwrap();
    content_store::init_schema(&tx).unwrap();
    let budget = crate::v3::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
    budget
        .codec_work()
        .unwrap()
        .run(|work| -> Result<(), String> {
            let mut stage = StageOwner::new(&tx);
            stage.work = Some(work);
            let child = sc::frame(sc::tag::BYTES_CHUNK, b"selected");
            let orphan = sc::frame(sc::tag::BYTES_CHUNK, b"later-cut");
            let parent = sc::frame(sc::tag::SEQ_LEAF, &child);
            stage
                .put_new(child, sc::tag::BYTES_CHUNK, b"selected")
                .unwrap();
            stage
                .put_new(orphan, sc::tag::BYTES_CHUNK, b"later-cut")
                .unwrap();
            let mut sql = content_store::TxStore::new(&tx)
                .unwrap()
                .with_work(Some(work));
            sql.put(parent, sc::tag::SEQ_LEAF, &child).unwrap();
            let before = work.snapshot().live;
            stage.flush_cut(&tx, FlushCut::Row(&[parent])).unwrap();
            assert_eq!(work.snapshot().live, before - b"selected".len());
            assert!(sql.checked_read_scoped(&child).unwrap().is_some());
            assert!(sql.checked_read_scoped(&orphan).unwrap().is_none());
            assert!(stage.nodes.borrow().contains_key(&orphan));
            assert!(!stage.nodes.borrow().contains_key(&child));
            assert_eq!(stage.flushed.get(), 1);
            let retained = work.snapshot().live;
            stage.flush_cut(&tx, FlushCut::Final(&[parent])).unwrap();
            assert_eq!(stage.flushed.get(), 1); // 同根重复cut无新的真实INSERT。
            assert_eq!(work.snapshot().live, retained);
            stage.flush_cut(&tx, FlushCut::Final(&[orphan])).unwrap();
            assert_eq!(stage.flushed.get(), 2);
            assert!(stage.nodes.borrow().is_empty());
            assert!(work.snapshot().live > 0); // 树许可不能按len=0提前归零。
            drop(stage);
            assert_eq!(work.snapshot().live, 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn staged_native_lease_uses_overlay_without_sql_material_cache_and_poison_cannot_retry() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL);INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
    let mut tx = conn.transaction().unwrap();
    content_store::init_schema(&tx).unwrap();
    let budget = crate::v3::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
    budget
        .codec_work()
        .unwrap()
        .run(|work| -> Result<(), String> {
            let mut stage = StageOwner::new(&tx);
            stage.work = Some(work);
            let bytes = b"lease-payload";
            let digest = sc::frame(sc::tag::BYTES_CHUNK, bytes);
            {
                let mut lease =
                    content_store::NativeReadLease::new_admitted_staged(&mut tx, work, &stage)
                        .unwrap();
                lease.put(digest, sc::tag::BYTES_CHUNK, bytes).unwrap();
                let retained = work.snapshot().live;
                let frame = lease.checked_read_scoped(&digest).unwrap().unwrap();
                assert_eq!(frame.body(), bytes);
                assert_eq!(work.snapshot().live, retained + bytes.len());
                drop(frame);
                assert_eq!(work.snapshot().live, retained); // overlay未进入SQL SameMaterial缓存。
            }
            assert_eq!(
                tx.query_row("SELECT count(*) FROM s_content_nodes", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            tx.execute(
                "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
                rusqlite::params![
                    &digest[..],
                    sc::tag::BYTES_CHUNK as i64,
                    b"corrupted".as_slice()
                ],
            )
            .unwrap();
            let mut store = content_store::TxStore::new_with_stage(&tx, Some(&stage))
                .unwrap()
                .with_work(Some(work));
            assert!(store.checked_read_scoped(&digest).is_err());
            assert!(stage.read_scoped(&digest).is_err());
            assert!(stage.flush_cut(&tx, FlushCut::Final(&[digest])).is_err());
            Ok(())
        })
        .unwrap();
}
