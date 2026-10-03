//! #1394 Native test-only：仅本事务新节点暂存；没有旧节点删除或跨事务 keeper。
use super::*;
use newchan_rust::s_content::{self as sc, NodeDigest, NodeSink, NodeSource};
use newchan_rust::session_protocol::strict_json;
use rusqlite::Transaction;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct StageOwner {
    connection: usize,
    nodes: RefCell<BTreeMap<NodeDigest, (u8, Vec<u8>)>>,
    peak_body_capacity: std::cell::Cell<usize>,
    puts: std::cell::Cell<u64>,
    kept: std::cell::Cell<usize>,
    flushed: std::cell::Cell<usize>,
    heap_sample_max: std::cell::Cell<usize>,
    heap_start: usize,
}
impl StageOwner {
    pub(super) fn new(tx: &Transaction<'_>) -> Self {
        // SQLite handle在Transaction按值传递时不变；owner由Runtime::commit单次栈上持有。
        Self {
            connection: unsafe { tx.handle() } as usize,
            nodes: Default::default(),
            peak_body_capacity: Default::default(),
            puts: Default::default(),
            kept: Default::default(),
            flushed: Default::default(),
            heap_sample_max: Default::default(),
            heap_start: heap_sample(),
        }
    }
    pub(super) fn check_transaction(&self, tx: &Transaction<'_>) -> Result<(), sc::CodecError> {
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
    pub(super) fn read(&self, d: &NodeDigest) -> Option<(u8, Vec<u8>)> {
        self.nodes.borrow().get(d).cloned()
    }
    pub(super) fn put_new(&self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
        self.puts.set(self.puts.get() + 1);
        let mut nodes = self.nodes.borrow_mut();
        if let Some((old_tag, old_body)) = nodes.get(&d) {
            if *old_tag != t || old_body.as_slice() != b {
                return Err(sc::CodecError::new(
                    sc::code::NODE_DIGEST,
                    "同暂存摘要异原件，拒绝",
                ));
            }
        } else {
            nodes.insert(d, (t, b.to_vec()));
        }
        let cap = nodes.values().map(|(_, b)| b.capacity()).sum();
        self.peak_body_capacity
            .set(self.peak_body_capacity.get().max(cap));
        self.heap_sample_max
            .set(self.heap_sample_max.get().max(heap_sample()));
        Ok(())
    }
    pub(super) fn report(&self) -> Value {
        let nodes = self.nodes.borrow();
        json!({"new_unique_accepted":nodes.len(),"new_put_visits":self.puts.get(),"actual_body_capacity_peak_bytes":self.peak_body_capacity.get(),"body_length_bytes":nodes.values().map(|(_,b)|b.len()).sum::<usize>(),"digest_original_bytes":nodes.len()*32,"tuple_inline_size_bytes":nodes.len()*std::mem::size_of::<(NodeDigest,(u8,Vec<u8>))>(),"BTree_allocations_and_strict_heap_peak":"UNMEASURED; body capacity is not total heap","process_malloc_zones_start_bytes":self.heap_start,"process_malloc_zones_sampled_max_bytes":self.heap_sample_max.get(),"heap_scope":"actual process malloc zones sampled after staged put, not staging-exclusive or strict heap peak","final_union_new_kept":self.kept.get(),"new_physical_inserts":self.flushed.get(),"new_accepted_not_flushed":nodes.len().saturating_sub(self.flushed.get()),"Job_node_acceptance_cumulative":"existing interfaces not fully Job-wired; staging visits are diagnostic, never decrement upper-layer access charges","whole_Job_or_max_store_qualified":false,"physical_INSERT_error_timing":"delayed until final flush; not equivalent to eager injection timing"})
    }
    pub(super) fn flush(&self, tx: &Transaction<'_>, runtime: &[NodeDigest]) -> Result<(), String> {
        self.check_transaction(tx).map_err(|e| e.to_string())?;
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
        for (d, (t, b)) in nodes.iter().filter(|(d, _)| seen.contains(*d)) {
            eager.put(*d, *t, b).map_err(|e| e.to_string())?;
            let count: i64 = tx
                .query_row("SELECT count(*) FROM s_content_nodes", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            self.flushed.set((count - initial) as usize);
        }
        Ok(())
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
