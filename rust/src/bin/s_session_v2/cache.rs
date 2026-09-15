//! #1372：仅缓存精确原字节解码与已验证结构前缀。每次仍在当前事务读全域。
//! 不使用 generation/mtime/data_version 代替依赖校验；默认禁用，serve 显式配置容量。
//! #1394 第二段（2D）：单一可配置预算把三类持久复用合并记账——图原件（本次捕获读入的不可变
//! 节点负载）、解码值（已核根句柄的展开字节与其解码 JSON）、root walk（Delta 行前缀 + 审计
//! 捕获）。临时读集（`capture`，每次捕获重建）与持久句柄（`handles` / `root`）分列：前者只在
//! 同一次捕获里省重复读，后者跨请求复用。容量不足时**只失去复用**，不改变任何合法性判据：
//! 读路径必须能无缓存地严格重核。
//!
//! 该上限是规范源字节记账（原字节 + 等额解码规范源字节 + 节点负载 + id + root rows/walk/identity），
//! 不是 allocator/RSS 上限，也不声称 RSS 受其限制。
use super::evidence_store::Node;
use super::*;
use std::collections::HashMap;
use std::{cell::RefCell, sync::Arc};

pub(super) type DeltaRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    // 末项：该代 Delta 原件的私有根闭包指纹（图域）；整块域为空串（原件就在上一列）。
    // 它使「已核前缀」对 Delta 原件的**当前内容**敏感：节点/根/槽任一变化都改指纹、都作废前缀。
    String,
);
/// （session_id、catalog_revision、（profile_id、profile_hash）、Delta 模式标签、证据图格式标签）。
/// 末两项是 #1394 的私有域：Delta 模式与证据图格式变化必须使旧 root/证书失效，不跨域复用。
type Identity = (String, String, (String, String), String, String);
struct Root {
    rows: Vec<DeltaRow>,
    identity: Identity,
    walk: audit::AuditWalk,
    bytes: usize,
}
/// 已核不可变根句柄：闭包指纹 + 展开字节 +（按需解码的）规范 JSON。
struct Handle {
    closure: [u8; 32],
    bytes: Arc<[u8]>,
    value: Option<Arc<Value>>,
    cost: usize,
}
/// 本次捕获的临时读集：本次读入的节点原行（容量不足则不保留，只失去复用）。
#[derive(Default)]
struct Capture {
    /// 本次捕获读入的节点原行：按**连接身份**分列——跨库复用同一 id 的节点不得互相顶替。
    nodes: HashMap<(usize, [u8; 32]), Arc<Node>>,
    /// 本次捕获按新鲜行算出的节点派生量（不含槽长度、槽出现次数、子树摘要）。
    derived: HashMap<(usize, [u8; 32]), NodeDerived>,
    /// 本次捕获从 SQLite 读入过的可达节点（跨读取器合并；供不可达残留核对）。
    reachable: std::collections::BTreeSet<(usize, [u8; 32])>,
    bytes: usize,
    /// 本次捕获临时读集的保留上限（沿用既有单次读取常量；超过即停止保留，只失去同次复用）。
    node_budget: usize,
    /// 本捕获内已解析的存储格式（schema 级事实；按连接身份缓存，捕获结束即作废）。
    format: Option<(usize, evidence_store::Format)>,
}
/// 一个节点的**与根无关**派生量：由当前新鲜读入的 payload 与子树自下而上算出。
///
/// - `length`：不含槽字节的展开长度（与 `Stager::seq` 的同一公式）；
/// - `slot_counts[i]`：子树（按展开树，含共享子树的重复引用）里第 i 号槽出现的次数；
/// - `digest`：子树摘要（id 已把 payload 钉死，故摘要只需 id + 长度 + 子节点摘要）。
#[derive(Clone, Default)]
pub(super) struct NodeDerived {
    pub(super) length: u64,
    pub(super) slot_counts: Vec<u64>,
    pub(super) digest: [u8; 32],
}
struct Memo {
    limit: usize,
    handles: BTreeMap<String, Handle>,
    handles_bytes: usize,
    capture: Option<Capture>,
    root: Option<Root>,
}
impl Default for Memo {
    fn default() -> Self {
        #[cfg(test)]
        let limit = std::env::var("S_TEST_AUDIT_CACHE_SOURCE_BYTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        #[cfg(not(test))]
        let limit = 0;
        Self {
            limit,
            handles: BTreeMap::new(),
            handles_bytes: 0,
            capture: None,
            root: None,
        }
    }
}
impl Memo {
    /// limit=0：完全禁用，root / 句柄 / 节点都不留。
    fn disabled() -> Self {
        Self {
            limit: 0,
            handles: BTreeMap::new(),
            handles_bytes: 0,
            capture: None,
            root: None,
        }
    }
    fn capture_bytes(&self) -> usize {
        self.capture.as_ref().map_or(0, |c| c.bytes)
    }
    fn root_bytes(&self) -> usize {
        self.root.as_ref().map_or(0, |r| r.bytes)
    }
    #[cfg(test)]
    fn total_bytes(&self) -> usize {
        self.root_bytes()
            .saturating_add(self.handles_bytes)
            .saturating_add(self.capture_bytes())
    }
    /// 按开销从大到小驱逐句柄，直到句柄总量不超过给定预算（不整块清空）。
    fn trim_handles(&mut self, budget: usize) {
        if self.handles_bytes <= budget {
            return;
        }
        let mut order = self
            .handles
            .iter()
            .map(|(k, h)| (h.cost, k.clone()))
            .collect::<Vec<_>>();
        order.sort_by(|a, b| b.cmp(a));
        let mut over = self.handles_bytes - budget;
        for (cost, key) in order {
            if over == 0 {
                break;
            }
            self.handles.remove(&key);
            self.handles_bytes = self.handles_bytes.saturating_sub(cost);
            over = over.saturating_sub(cost);
        }
    }
}
#[cfg(test)]
thread_local! {static LAST_RESUMED: std::cell::Cell<usize> = const {std::cell::Cell::new(0)};}
#[cfg(test)]
pub(super) fn last_resumed() -> usize {
    LAST_RESUMED.with(|v| v.get())
}
thread_local! { static MEMO: RefCell<Memo> = RefCell::new(Memo::default()); }

/// #1394 第二段阶段计数（只在测试构建里累加，供成本证据：唯一节点读/解析数、历史展开数、
/// 句柄命中/失效）。生产构建不含这些计数，也不改变任何判据。
#[cfg(test)]
#[derive(Default, Clone, Copy)]
pub(super) struct EvidenceCounters {
    /// 从 SQLite **新鲜读入**的唯一节点行数（捕获内去重后）。
    pub node_rows_read: usize,
    /// 读入的节点负载字节。
    pub node_payload_bytes_read: usize,
    /// 同次捕获内命中临时读集、未再读 SQLite 的次数。
    pub node_reused_in_capture: usize,
    /// 根读取次数 / 命中已核句柄次数 / 完整展开次数 / 解码为 JSON 次数。
    pub root_reads: usize,
    pub root_handle_hits: usize,
    pub root_expansions: usize,
    pub root_parses: usize,
    /// 批次：命中已核句柄 vs 本次完整展开（历史 batch 展开数）。
    pub batch_handle_hits: usize,
    pub batch_expansions: usize,
    /// 阶段耗时（ns）：根审计（含闭包新鲜读 + 句柄复用/展开）、批次 stage、Commit。
    pub audit_ns: u64,
    pub stage_ns: u64,
    pub commit_ns: u64,
}
#[cfg(test)]
thread_local! { static COUNTERS: RefCell<EvidenceCounters> = RefCell::new(EvidenceCounters::default()); }
#[cfg(test)]
pub(super) fn counter(mut f: impl FnMut(&mut EvidenceCounters)) {
    COUNTERS.with(|c| f(&mut c.borrow_mut()));
}
#[cfg(test)]
pub(super) fn counters_reset() {
    COUNTERS.with(|c| *c.borrow_mut() = EvidenceCounters::default());
}
#[cfg(test)]
pub(super) fn counters_snapshot() -> EvidenceCounters {
    COUNTERS.with(|c| *c.borrow())
}

/// #1394 第二段：丢掉**跨请求的持久句柄**（保留配置与临时读集），用于测量真正的冷展开成本。
#[cfg(test)]
pub(super) fn clear_handles() {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        m.handles.clear();
        m.handles_bytes = 0;
        m.root = None;
    });
}

pub(super) fn configure(limit: usize) {
    MEMO.with(|m| {
        *m.borrow_mut() = Memo {
            limit,
            ..Memo::default()
        }
    });
}

// ───────────────────────── 临时读集（本次捕获） ─────────────────────────

/// 捕获作用域守卫：最外层守卫析构时清空临时读集。
///
/// 读路径每次捕获都从**新鲜行**重建临时读集；同一捕获内同一节点只读一次，跨捕获不复用节点行
/// （任何节点内容/长度变化都必须在下一次捕获被重读到，不得被旧读集掩盖）。
pub(super) struct CaptureGuard(bool);
impl Drop for CaptureGuard {
    fn drop(&mut self) {
        if self.0 {
            end_capture();
        }
    }
}
/// 开启（或复用外层）捕获作用域。
///
/// `node_budget` 是本次捕获临时读集的保留上限（既有单次读取常量），**不是**持久复用预算：
/// 临时读集保证「同次捕获每个唯一节点原行只读一次」，其字节计入同一份总账；持久预算不足时
/// 先失去跨请求复用（句柄），临时读集不为它让路。
pub(super) fn begin_capture(node_budget: usize) -> CaptureGuard {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.capture.is_some() {
            return CaptureGuard(false);
        }
        m.capture = Some(Capture {
            node_budget,
            ..Capture::default()
        });
        CaptureGuard(true)
    })
}
/// 作废临时读集：临时读集不得跨写事务存活（写后重读必须重新读新鲜行）。
pub(super) fn end_capture() {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(c) = m.capture.as_mut() {
            c.nodes.clear();
            c.bytes = 0;
            c.format = None;
        }
        m.capture = None;
    });
}
/// 本次捕获是否已读入该节点原行（只有同一捕获内的重复引用才命中）。
pub(super) fn node_get(conn_id: usize, id: &[u8; 32]) -> Option<Arc<Node>> {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .and_then(|c| c.nodes.get(&(conn_id, *id)).cloned())
    })
}
/// 记入本次捕获的节点原行；超出**本次读集保留上限**只失去同次复用（当次读仍照常返回该节点）。
/// 临时读集的字节同时计入同一份总账（`total_bytes`），持久句柄按剩余预算装入。
pub(super) fn node_put(conn_id: usize, id: [u8; 32], node: Arc<Node>, cost: usize) {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(c) = m.capture.as_mut() {
            if c.nodes.contains_key(&(conn_id, id)) {
                return;
            }
            if c.bytes.saturating_add(cost) > c.node_budget {
                return;
            }
            c.bytes = c.bytes.saturating_add(cost);
            c.nodes.insert((conn_id, id), node);
        }
    });
}
/// 本次捕获已读入的节点数（供「同次捕获只读一次」的行为证据）。
#[cfg(test)]
pub(super) fn capture_len() -> usize {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .map_or(0, |c| c.nodes.len())
    })
}

/// 本捕获已算出的节点派生量（同一连接、同一次捕获内复用；跨捕获必然重算）。
pub(super) fn derived_get(conn_id: usize, id: &[u8; 32]) -> Option<NodeDerived> {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .and_then(|c| c.derived.get(&(conn_id, *id)).cloned())
    })
}
pub(super) fn derived_put(conn_id: usize, id: [u8; 32], derived: NodeDerived) {
    MEMO.with(|m| {
        if let Some(c) = m.borrow_mut().capture.as_mut() {
            c.derived.insert((conn_id, id), derived);
        }
    });
}

/// 记下「本捕获从 SQLite 读入过该节点」——跨同一次捕获里的全部读取器合并。
pub(super) fn mark_reachable(conn_id: usize, id: [u8; 32]) {
    MEMO.with(|m| {
        if let Some(c) = m.borrow_mut().capture.as_mut() {
            c.reachable.insert((conn_id, id));
        }
    });
}
/// 本捕获在该连接上读入过的可达节点数（与节点表行数比对即等价于「无不可达残留」）。
pub(super) fn capture_reachable_len(conn_id: usize) -> usize {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .map_or(0, |c| c.reachable.iter().filter(|(id, _)| *id == conn_id).count())
    })
}
/// 本捕获内已解析的存储格式（只在同一次捕获、同一个连接上复用；捕获结束即作废）。
pub(super) fn capture_format(conn_id: usize) -> Option<evidence_store::Format> {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .and_then(|c| c.format.filter(|(id, _)| *id == conn_id).map(|(_, f)| f))
    })
}
pub(super) fn capture_format_set(conn_id: usize, format: evidence_store::Format) {
    MEMO.with(|m| {
        if let Some(c) = m.borrow_mut().capture.as_mut() {
            c.format = Some((conn_id, format));
        }
    });
}

// ───────────────────────── 持久句柄（跨请求复用） ─────────────────────────

/// 取已核根句柄：闭包指纹必须逐字节相同（根行、闭包每个节点的 kind/payload/length/slot 都已在
/// 本次捕获用新鲜行核过，故指纹相同即展开字节相同）。命中不改变任何判据。
pub(super) fn handle_get(
    root_key: &str,
    closure: &[u8; 32],
) -> Option<(Arc<[u8]>, Option<Arc<Value>>)> {
    MEMO.with(|m| {
        m.borrow()
            .handles
            .get(root_key)
            .filter(|h| h.closure == *closure)
            .map(|h| (h.bytes.clone(), h.value.clone()))
    })
}
/// 记入已核根句柄（只有在本次已完整展开并核过公开长度/摘要之后才可调用）。
/// 装不下只失去这次复用，不改任何判据；也不整块清空其他仍可复用的句柄。
pub(super) fn handle_put(
    root_key: &str,
    closure: [u8; 32],
    bytes: Arc<[u8]>,
    value: Option<Arc<Value>>,
) {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.limit == 0 {
            return;
        }
        if let Some(old) = m.handles.remove(root_key) {
            m.handles_bytes = m.handles_bytes.saturating_sub(old.cost);
        }
        let cost = handle_cost(root_key.len(), bytes.len(), value.is_some());
        let budget = m
            .limit
            .saturating_sub(m.root_bytes())
            .saturating_sub(m.capture_bytes());
        if cost > budget {
            return;
        }
        m.trim_handles(budget.saturating_sub(cost));
        if m.handles_bytes.saturating_add(cost) > budget {
            return;
        }
        m.handles_bytes = m.handles_bytes.saturating_add(cost);
        m.handles.insert(
            root_key.to_string(),
            Handle {
                closure,
                bytes,
                value,
                cost,
            },
        );
    });
}
/// 句柄记账：原字节 +（有解码值时）等额解码规范源字节 + 根键与固定开销。
fn handle_cost(key_len: usize, bytes_len: usize, decoded: bool) -> usize {
    key_len
        .saturating_add(if decoded {
            bytes_len.saturating_mul(2)
        } else {
            bytes_len
        })
        .saturating_add(64)
}

pub(super) fn resume(rows: &[DeltaRow], identity: &Identity, walk: &mut audit::AuditWalk) -> usize {
    #[cfg(test)]
    LAST_RESUMED.with(|v| v.set(0));
    MEMO.with(|m| {
        let m = m.borrow();
        let Some(old) = &m.root else { return 0 };
        if &old.identity != identity
            || rows.len() < old.rows.len()
            || rows[..old.rows.len()] != old.rows
        {
            return 0;
        }
        let generation = old.rows.len() as i64;
        if walk.resume_from(&old.walk, generation) {
            #[cfg(test)]
            LAST_RESUMED.with(|v| v.set(old.rows.len()));
            old.rows.len()
        } else {
            0
        }
    })
}
pub(super) fn store_root(rows: Vec<DeltaRow>, identity: Identity, walk: audit::AuditWalk) {
    let bytes = root_cost(&rows, &identity, &walk);
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.limit == 0 {
            *m = Memo::disabled();
            return;
        }
        if bytes > m.limit {
            // 新 root 单独超限：保留此前仍可复用的旧前缀（仍在当前上限内、身份相同、且仍是当前行的
            // 前缀），不清空句柄缓存；判不出仍有用时只丢旧 root。
            let keep = m.root.as_ref().is_some_and(|old| {
                old.bytes <= m.limit
                    && old.identity == identity
                    && rows.len() >= old.rows.len()
                    && rows[..old.rows.len()] == old.rows
            });
            if !keep {
                m.root = None;
            }
            return;
        }
        m.root = Some(Root {
            rows,
            identity,
            walk,
            bytes,
        });
        // root walk 优先装入，再按剩余预算驱逐句柄（不因新 root 变大而整块清空）。
        let budget = m
            .limit
            .saturating_sub(bytes)
            .saturating_sub(m.capture_bytes());
        m.trim_handles(budget);
    });
}

/// root 记账：行内字符串 + 每行 16B + walk 的规范源字节 + identity 字段；全程饱和加，避免溢出。
fn root_cost(rows: &[DeltaRow], identity: &Identity, walk: &audit::AuditWalk) -> usize {
    let row_bytes = rows.iter().fold(0usize, |n, r| {
        n.saturating_add(
            r.1.len()
                .saturating_add(r.2.len())
                .saturating_add(r.3.len())
                .saturating_add(r.4.len())
                .saturating_add(r.5.len())
                .saturating_add(r.6.len())
                .saturating_add(r.8.len())
                .saturating_add(r.9.len())
                .saturating_add(r.10.len())
                .saturating_add(r.11.len())
                .saturating_add(16),
        )
    });
    row_bytes
        .saturating_add(walk.source_bytes())
        .saturating_add(
            identity
                .0
                .len()
                .saturating_add(identity.1.len())
                .saturating_add(identity.2 .0.len())
                .saturating_add(identity.2 .1.len())
                .saturating_add(identity.3.len())
                .saturating_add(identity.4.len()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空索引 walk：缓存不改动审计捕获，故测试只需最小合法 walk 即可组装 Root。
    fn empty_walk() -> audit::AuditWalk {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::SCHEMA).unwrap();
        conn.execute_batch(
            "INSERT INTO meta(key,value) VALUES \
             ('generation','0'),('structure_cut','cut-0'),\
             ('catalog_revision','s2-axis-quantifiers+new-bi/1'),\
             ('scope','{\"structure\":\"CompleteCut\",\"economic\":\"not_started\"}')",
        )
        .unwrap();
        audit::AuditWalk::capture(&conn, &[]).unwrap()
    }
    fn identity(session: &str) -> Identity {
        (
            session.to_string(),
            "catalog-hash".to_string(),
            ("profile-id".to_string(), "profile-hash".to_string()),
            delta_storage::CHANGES_ONLY.to_string(),
            evidence_store::GRAPH_FORMAT.to_string(),
        )
    }
    fn rows(n: i64) -> Vec<DeltaRow> {
        (1..=n)
            .map(|g| {
                (
                    g,
                    "test-c".to_string(),
                    "catalog-hash".to_string(),
                    format!("cut-{}", g - 1),
                    format!("cut-{g}"),
                    "{\"from\":\"0\",\"to\":\"0\"}".to_string(),
                    format!("batch-{g}"),
                    g - 1,
                    "not_run".to_string(),
                    "{}".to_string(),
                    format!("{{\"generation\":\"{g}\"}}"),
                    String::new(),
                )
            })
            .collect()
    }
    fn leaf(payload: &[u8]) -> Arc<Node> {
        Arc::new(Node {
            kind: 0,
            payload: payload.to_vec(),
            expanded_length: payload.len() as u64,
        })
    }
    fn handle(key: &str, len: usize, decoded: bool) -> (Arc<[u8]>, Option<Arc<Value>>) {
        (
            Arc::<[u8]>::from(vec![b'x'; len]),
            if decoded {
                Some(Arc::new(serde_json::json!({"k": key})))
            } else {
                None
            },
        )
    }
    fn memo_root_rows() -> Option<usize> {
        MEMO.with(|m| m.borrow().root.as_ref().map(|r| r.rows.len()))
    }
    fn memo_handle_keys() -> Vec<String> {
        MEMO.with(|m| m.borrow().handles.keys().cloned().collect())
    }
    fn memo_total() -> usize {
        MEMO.with(|m| m.borrow().total_bytes())
    }

    #[test]
    fn root_survives_and_handles_fill_only_the_shared_budget() {
        let base = rows(2);
        let root = root_cost(&base, &identity("test-c"), &empty_walk());
        let one = handle_cost("batch-a".len(), 64, true);
        let limit = root + one * 3;
        configure(limit);
        store_root(base, identity("test-c"), empty_walk());
        for (key, len) in [
            ("batch-a", 64),
            ("batch-bb", 1 << 20),
            ("batch-c", 64),
            ("batch-d", 64),
        ] {
            let (bytes, value) = handle(key, len, true);
            handle_put(key, [0u8; 32], bytes, value);
        }
        // root 优先保留；单条过大句柄跳过而不牵连其他句柄。
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_handle_keys(), vec!["batch-a", "batch-c", "batch-d"]);
        assert!(memo_total() <= limit);
        assert!(handle_get("batch-a", &[0u8; 32]).is_some());
        assert!(handle_get("batch-a", &[1u8; 32]).is_none());
        assert!(handle_get("batch-bb", &[0u8; 32]).is_none());
    }

    #[test]
    fn growing_root_evicts_handles_instead_of_dropping_everything() {
        let small = rows(2);
        let big = rows(6);
        let big_bytes = root_cost(&big, &identity("test-c"), &empty_walk());
        let one = handle_cost("batch-a".len(), 64, true);
        let limit = big_bytes + one;
        configure(limit);
        store_root(small, identity("test-c"), empty_walk());
        for key in ["batch-a", "batch-b"] {
            let (bytes, value) = handle(key, 64, true);
            handle_put(key, [0u8; 32], bytes, value);
        }
        assert_eq!(memo_handle_keys(), vec!["batch-a", "batch-b"]);
        store_root(big, identity("test-c"), empty_walk());
        assert_eq!(memo_root_rows(), Some(6));
        assert_eq!(memo_handle_keys(), vec!["batch-a"]); // 句柄部分驱逐，root 保留
        assert!(memo_total() <= limit);
    }

    #[test]
    fn oversized_new_root_keeps_previous_verified_prefix_usable() {
        let small = rows(2);
        let huge = rows(64);
        let small_bytes = root_cost(&small, &identity("test-c"), &empty_walk());
        let one = handle_cost("batch-a".len(), 64, true);
        let limit = small_bytes + one;
        assert!(root_cost(&huge, &identity("test-c"), &empty_walk()) > limit);
        configure(limit);
        store_root(small, identity("test-c"), empty_walk());
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put("batch-a", [0u8; 32], bytes, value);
        store_root(huge.clone(), identity("test-c"), empty_walk());
        // 装不下新 root：旧前缀与句柄都保留，不清整个 Memo。
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_handle_keys(), vec!["batch-a"]);
        assert!(memo_total() <= limit);
        assert_eq!(resume(&huge, &identity("test-c"), &mut empty_walk()), 2);
        assert_eq!(last_resumed(), 2);
        // 前缀不再匹配时必须作废，不留死前缀、不误用。
        let mut changed = huge.clone();
        changed[0].6 = "batch-9".to_string();
        assert_eq!(resume(&changed, &identity("test-c"), &mut empty_walk()), 0);
        assert_eq!(last_resumed(), 0);
    }

    #[test]
    fn verified_handle_requires_identical_closure_and_identity_are_checked() {
        configure(4096);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put("batch-a", [7u8; 32], bytes, value);
        assert!(handle_get("batch-a", &[7u8; 32]).is_some());
        assert!(handle_get("batch-a", &[8u8; 32]).is_none());
        store_root(rows(3), identity("test-c"), empty_walk());
        assert_eq!(resume(&rows(5), &identity("test-c"), &mut empty_walk()), 3);
        assert_eq!(resume(&rows(5), &identity("other-c"), &mut empty_walk()), 0);
        let mut changed = rows(5);
        changed[1].4 = "cut-99".to_string();
        assert_eq!(resume(&changed, &identity("test-c"), &mut empty_walk()), 0);
        assert_eq!(resume(&rows(2), &identity("test-c"), &mut empty_walk()), 0);
    }

    #[test]
    fn zero_limit_disables_cache_entirely() {
        configure(0);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put("batch-a", [0u8; 32], bytes, value);
        store_root(rows(2), identity("test-c"), empty_walk());
        node_put(7, [9u8; 32], leaf(b"xx"), 2);
        assert_eq!(memo_root_rows(), None);
        assert!(memo_handle_keys().is_empty());
        assert_eq!(memo_total(), 0);
        assert!(handle_get("batch-a", &[0u8; 32]).is_none());
        assert!(node_get(7, &[9u8; 32]).is_none());
        assert_eq!(resume(&rows(2), &identity("test-c"), &mut empty_walk()), 0);
        assert_eq!(last_resumed(), 0);
    }

    /// #1394 第二段：临时读集的节点行只在**同一次捕获**内复用；捕获结束即作废，不得跨捕获。
    #[test]
    fn capture_scope_is_temporary_and_never_spans_captures() {
        configure(1 << 20);
        {
            let _guard = begin_capture(1 << 20);
            node_put(7, [1u8; 32], leaf(b"payload"), 7);
            assert!(node_get(7, &[1u8; 32]).is_some());
        }
        // 捕获结束：新鲜行必须重新读入，旧临时读集不得继续命中。
        assert!(node_get(7, &[1u8; 32]).is_none());
        {
            let _outer = begin_capture(1 << 20);
            node_put(7, [2u8; 32], leaf(b"payload"), 7);
            {
                // 嵌套捕获复用外层读集，内层析构不得提前清空。
                let _inner = begin_capture(1 << 20);
                assert!(node_get(7, &[2u8; 32]).is_some());
            }
            assert!(node_get(7, &[2u8; 32]).is_some());
        }
        assert!(node_get(7, &[2u8; 32]).is_none());
    }

    /// #1394 第二段：写前作废临时读集，写后重读必须重新读新鲜行。
    #[test]
    fn end_capture_drops_the_temporary_read_set() {
        configure(1 << 20);
        let _outer = begin_capture(1 << 20);
        node_put(7, [3u8; 32], leaf(b"payload"), 7);
        assert!(node_get(7, &[3u8; 32]).is_some());
        end_capture();
        assert!(node_get(7, &[3u8; 32]).is_none());
    }

    /// #1394 第二段：临时读集按本次读取上限保留（不被复用预算清空），其字节计入同一份总账，
    /// 从而按剩余预算挤压持久句柄——超限只失去跨请求复用。
    #[test]
    fn capture_nodes_and_handles_share_the_single_budget() {
        let one = handle_cost("batch-a".len(), 64, true);
        configure(one + 16);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put("batch-a", [0u8; 32], bytes, value);
        {
            let _guard = begin_capture(16);
            node_put(7, [4u8; 32], leaf(&[0u8; 10]), 10);
            assert!(node_get(7, &[4u8; 32]).is_some());
            assert_eq!(capture_len(), 1);
            // 第二条节点负载超出本次读集上限：只失去同次复用（当次读仍照常返回该节点）。
            node_put(7, [5u8; 32], leaf(&[0u8; 64]), 64);
            assert!(node_get(7, &[5u8; 32]).is_none());
            assert_eq!(capture_len(), 1);
            // 单一总账 = root walk + 持久句柄 + 本次捕获临时读集。
            assert_eq!(memo_total(), one + 10);
        }
        // 捕获结束：临时读集释放，句柄仍在预算内。
        assert_eq!(memo_total(), one);
    }

    /// #1394 第二段：临时读集**不被** `configure(0)` 清空（同次捕获每个唯一节点只读一次是读路径
    /// 的必需条件），但 `configure(0)` 下不留任何跨请求复用。
    #[test]
    fn disabled_reuse_still_keeps_the_transient_read_set() {
        configure(0);
        {
            let _guard = begin_capture(1 << 20);
            node_put(7, [6u8; 32], leaf(b"payload"), 7);
            assert!(node_get(7, &[6u8; 32]).is_some());
            let (bytes, value) = handle("batch-a", 64, true);
            handle_put("batch-a", [0u8; 32], bytes, value);
            assert!(handle_get("batch-a", &[0u8; 32]).is_none());
        }
        assert!(node_get(7, &[6u8; 32]).is_none());
    }

    /// #1394：证据图格式是缓存身份的一部分——同 session/catalog/profile 但格式不同不得复用已核前缀。
    #[test]
    fn evidence_format_tag_is_part_of_the_verified_root_identity() {
        let with_format = |graph: &str| {
            (
                "test-c".to_string(),
                "catalog-hash".to_string(),
                ("profile-id".to_string(), "profile-hash".to_string()),
                delta_storage::CHANGES_ONLY.to_string(),
                graph.to_string(),
            )
        };
        configure(4096);
        store_root(
            rows(3),
            with_format(evidence_store::GRAPH_FORMAT),
            empty_walk(),
        );
        assert_eq!(
            resume(
                &rows(3),
                &with_format(evidence_store::GRAPH_FORMAT),
                &mut empty_walk()
            ),
            3
        );
        assert_eq!(
            resume(
                &rows(3),
                &with_format(evidence_store::GRAPH_FORMAT_V2),
                &mut empty_walk()
            ),
            0
        );
        assert_eq!(last_resumed(), 0);
        configure(0);
    }
}
