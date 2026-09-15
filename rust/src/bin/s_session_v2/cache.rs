//! #1372：仅缓存精确原字节解码与已验证结构前缀。每次仍在当前事务读全域。
//! 不使用 generation/mtime/data_version 代替依赖校验；默认禁用，serve 显式配置容量。
//!
//! #1394 第二段（2B/2D）的单预算纪律：**一个**可配置上限（`configure`）约束全部**保留量**——
//! root walk（Delta 行前缀 + 审计捕获）、持久句柄（展开字节 + 解码 JSON + 根原件证书）、持久
//! 原件证书（每个唯一节点一份原始行）。本次捕获的临时读集与派生量是**瞬时工作集**：每次捕获
//! 重建、捕获结束即释放，不是第二个可配置预算、也不是跨请求缓存。超限先驱逐**可丢弃**的持久
//! 句柄（最贵、最可弃），仍装不下就跳过该项保留——只失去复用，不改变任何合法性判据。
//!
//! #1394 第三段（exact-originals）：跨请求复用的判据是**精确原件逐字节相等**，不是摘要相等。
//! 每个唯一节点保留一份原件证书（原始 kind/payload/声明长度 + 有序子证书键），每个根保留完整
//! 原始根行与有序槽。每次新捕获从 SQLite 读当前实际行，逐字节与保留证书比较（同一次捕获内按
//! 唯一节点去重），全部相同才允许复用展开字节/解码值/审计前缀。摘要（闭包指纹）与公开内容
//! 寻址只作**附加**一致校验，不作替代：任一原件不同都进不了复用门。
//!
//! 该上限是规范源字节记账（原字节 + 等额解码规范源字节 + 唯一原件证书 + id + root rows/walk/identity），
//! 不是 allocator/RSS 上限，也不声称 RSS 受其限制。
use super::*;
use std::collections::{BTreeSet, HashMap};
use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};

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
    // 它使「已核前缀」对 Delta 原件的**当前内容**敏感，但它只是附加校验——
    // 图域真正的前缀门是 `Root::originals` 里的全部已核原件证书（见 `resume`）。
    String,
);
/// （session_id、catalog_revision、（profile_id、profile_hash）、Delta 模式标签、证据图格式标签）。
/// 末两项是 #1394 的私有域：Delta 模式与证据图格式变化必须使旧 root/证书失效，不跨域复用。
type Identity = (String, String, (String, String), String, String);
/// 本次捕获的全部私有根证明。无法保留的根仍须带键，以 `None` 禁止前缀复用。
pub(super) type Originals = BTreeMap<String, Option<Arc<RootCert>>>;
struct Root {
    rows: Vec<DeltaRow>,
    identity: Identity,
    /// batch、Delta、fact 的全部私有根证明；只持弱引用，不让被驱逐原件逃出预算。
    /// 旧整块域为空，行内原件由 `rows` / `AuditWalk` 逐字节比较。
    originals: BTreeMap<String, Option<Weak<RootCert>>>,
    walk: audit::AuditWalk,
    bytes: usize,
}

/// 一个唯一节点的**精确原件证书**：原始行（种类 + payload 原字节 + 声明长度）与有序直接子节点
/// 引用。子引用就是子证书的键（内容寻址 id），故证书图不需要递归深链，也不需要第二层摘要。
pub(super) struct Cert {
    pub(super) id: [u8; 32],
    pub(super) kind: i64,
    pub(super) payload: Arc<[u8]>,
    pub(super) expanded_length: u64,
    /// 有序直接子节点引用（= 子证书键）。叶子为空。
    pub(super) children: Vec<[u8; 32]>,
}
impl Cert {
    /// 逐字节比较**实际原件**：不用摘要、mtime、generation、data_version 代替。
    pub(super) fn same_originals(&self, kind: i64, payload: &[u8], expanded_length: u64) -> bool {
        self.kind == kind
            && self.expanded_length == expanded_length
            && self.payload.as_ref() == payload
    }
    fn payload_cost(&self) -> usize {
        self.payload
            .len()
            .saturating_add(64)
            .saturating_add(self.children.len().saturating_mul(32))
    }
}

/// 一个私有根的**精确原件证书**：完整原始根行（根指针、声明长度、内容摘要）与有序参数槽。
pub(super) struct RootCert {
    pub(super) node: [u8; 32],
    pub(super) slots: Vec<[u8; 32]>,
    pub(super) expanded_length: u64,
    pub(super) content_sha256: String,
    // 原件池一旦丢失或替换节点，这份证明失效；此令牌仅负责失效，不能替代新鲜原件比较。
    lineage: Weak<()>,
}
impl RootCert {
    pub(super) fn new(
        node: [u8; 32],
        slots: Vec<[u8; 32]>,
        expanded_length: u64,
        content_sha256: String,
    ) -> Self {
        Self {
            node,
            slots,
            expanded_length,
            content_sha256,
            lineage: Weak::new(),
        }
    }
    /// 逐字节比较**实际原件**：根指针、有序槽列、声明长度、内容摘要。
    pub(super) fn same_originals(
        &self,
        node: &[u8; 32],
        slots: &[[u8; 32]],
        expanded_length: u64,
        content_sha256: &str,
    ) -> bool {
        self.node == *node
            && self.slots == slots
            && self.expanded_length == expanded_length
            && self.content_sha256 == content_sha256
    }
    fn cost(&self) -> usize {
        self.slots
            .len()
            .saturating_mul(32)
            .saturating_add(32 + 8 + 64)
            .saturating_add(self.content_sha256.len())
    }
}

/// 已核不可变根句柄：闭包指纹 +（图域的）根原件证书 + 展开字节 +（按需解码的）规范 JSON。
///
/// `root_cert` 指向**保留**的根原件证书；复用要求它与当前的保留证书是**同一个原件**
/// （指针同一），故摘要相等既不是充分条件也不是唯一条件。
struct Handle {
    closure: [u8; 32],
    root_cert: Option<Weak<RootCert>>,
    bytes: Arc<[u8]>,
    value: Option<Arc<Value>>,
    cost: usize,
}
/// 本次捕获的临时读集（瞬时工作集，捕获结束即释放）：本次读入并比较过的节点原件。
#[derive(Default)]
struct Capture {
    /// 本次捕获读入的节点原件：按**连接身份**分列——跨库复用同一 id 的节点不得互相顶替。
    nodes: HashMap<(usize, [u8; 32]), Arc<Cert>>,
    /// 读集里 payload 字节由本次捕获**独占**（未与持久证书共享）的节点：提交时把这份字节转记为
    /// 持久证书，从而「共享原件按唯一保留量记账」，不重复计费。
    owned: BTreeSet<(usize, [u8; 32])>,
    /// 本次捕获已与保留证书**逐字节比较且相同**的唯一节点（复用门的充要条件）。
    matched: BTreeSet<(usize, [u8; 32])>,
    /// 本次捕获已由**保留证书侧**自下而上认证完毕的子树根（同次认证去重：
    /// 整次捕获的认证成本是 O(唯一节点)，不是 O(根数 × 闭包））。
    certified: BTreeSet<(usize, [u8; 32])>,
    /// 本次捕获按新鲜行算出的节点派生量（不含槽长度、槽出现次数、子树摘要）。
    /// 这是遍历的即时状态：不跨捕获保留，也不假装成跨请求缓存。
    derived: HashMap<(usize, [u8; 32]), NodeDerived>,
    /// 本次捕获从 SQLite 读入过的可达节点（跨读取器合并；供不可达残留核对）。
    reachable: BTreeSet<(usize, [u8; 32])>,
    bytes: usize,
    /// 本次捕获临时读集的**瞬时**保留上限（非持久预算、不是第二个可配置缓存预算；
    /// 它与单一预算同时约束，两者取更紧的那个）。
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
    /// 精确原件证书：按内容寻址 id 唯一保留一份（共享原件按唯一保留量记账）。
    certs: HashMap<[u8; 32], Arc<Cert>>,
    /// 私有根的精确原件证书：按根键唯一保留一份（保持指针稳定供「同一原件」判定）。
    root_certs: HashMap<String, Arc<RootCert>>,
    certs_bytes: usize,
    cert_lineage: Arc<()>,
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
            certs: HashMap::new(),
            root_certs: HashMap::new(),
            certs_bytes: 0,
            cert_lineage: Arc::new(()),
            capture: None,
            root: None,
        }
    }
}
impl Memo {
    /// limit=0：完全禁用，root / 句柄 / 证书 / 读集都不留。
    fn disabled() -> Self {
        Self {
            limit: 0,
            ..Self::default()
        }
    }
    fn capture_bytes(&self) -> usize {
        self.capture.as_ref().map_or(0, |c| c.bytes)
    }
    fn root_bytes(&self) -> usize {
        self.root.as_ref().map_or(0, |r| r.bytes)
    }
    /// 跨请求**保留量**：root walk + 持久句柄 + 持久原件证书。由单一 `limit` 约束。
    fn retained_bytes(&self) -> usize {
        self.root_bytes()
            .saturating_add(self.handles_bytes)
            .saturating_add(self.certs_bytes)
    }
    #[cfg(test)]
    fn total_bytes(&self) -> usize {
        self.retained_bytes().saturating_add(self.capture_bytes())
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
    /// 单一预算下为 `cost` 字节腾出空间：先驱逐**可丢弃**的持久句柄，返回腾出后是否装得下。
    fn make_room(&mut self, cost: usize) -> bool {
        // 先判这一项在**没有任何句柄**时能否装下：装不下就不该为它驱逐其他仍可复用的句柄
        // （单条过大只跳过自己，不牵连别人）。
        let floor = self
            .root_bytes()
            .saturating_add(self.certs_bytes)
            .saturating_add(self.capture_bytes());
        if floor.saturating_add(cost) > self.limit {
            return false;
        }
        let used = self.retained_bytes().saturating_add(self.capture_bytes());
        if used.saturating_add(cost) <= self.limit {
            return true;
        }
        let need = used.saturating_add(cost).saturating_sub(self.limit);
        self.trim_handles(self.handles_bytes.saturating_sub(need));
        self.retained_bytes()
            .saturating_add(self.capture_bytes())
            .saturating_add(cost)
            <= self.limit
    }
    /// 移除索引后，捕获若仍持有原件，把唯一 payload 费用转给其中一个读集条目。
    fn remove_cert(&mut self, key: &[u8; 32]) {
        let Some(cert) = self.certs.remove(key) else {
            return;
        };
        self.certs_bytes = self.certs_bytes.saturating_sub(cert.payload_cost());
        self.cert_lineage = Arc::new(());
        if let Some(c) = self.capture.as_mut() {
            if let Some(owner) = c
                .nodes
                .iter()
                .find(|(_, held)| Arc::ptr_eq(held, &cert))
                .map(|(key, _)| *key)
            {
                c.owned.insert(owner);
                c.bytes = c.bytes.saturating_add(cert.payload_cost());
            }
        }
    }
    /// 驱逐独有节点证书（单条最大者优先）与根证书，直到证书总量不超过给定预算。
    /// 证书只影响复用、不影响任何判据；根证书被驱逐时其句柄一并作废（不留永远命不中的死句柄）。
    fn trim_certs(&mut self, budget: usize) {
        if self.certs_bytes <= budget {
            return;
        }
        let mut over = self.certs_bytes - budget;
        let mut order = self
            .certs
            .iter()
            .map(|(k, c)| (c.payload_cost(), *k))
            .collect::<Vec<_>>();
        order.sort_by(|a, b| b.cmp(a));
        for (cost, key) in order {
            if over == 0 {
                break;
            }
            self.remove_cert(&key);
            over = over.saturating_sub(cost);
        }
        if over > 0 {
            let mut order = self
                .root_certs
                .iter()
                .map(|(k, c)| (c.cost().saturating_add(k.len()), k.clone()))
                .collect::<Vec<_>>();
            order.sort_by(|a, b| b.cmp(a));
            for (cost, key) in order {
                if over == 0 {
                    break;
                }
                if self.root_certs.remove(&key).is_some() {
                    self.certs_bytes = self.certs_bytes.saturating_sub(cost);
                    over = over.saturating_sub(cost);
                    if let Some(old) = self.handles.remove(&key) {
                        self.handles_bytes = self.handles_bytes.saturating_sub(old.cost);
                    }
                }
            }
        }
    }
    /// 单一预算的**最终把关**：保留量（root walk + 句柄 + 原件证书）必须落在同一上限内
    /// （再扣掉本次捕获的瞬时读集）。句柄最可弃，其次才是原件证书。
    fn fit_retention(&mut self) {
        let after_root = self
            .limit
            .saturating_sub(self.root_bytes())
            .saturating_sub(self.capture_bytes());
        self.trim_handles(after_root.saturating_sub(self.certs_bytes));
        self.trim_certs(after_root.saturating_sub(self.handles_bytes));
        // 证书驱逐会把仍活着的共享原件转入捕获；若其暂存挤占新 root，只放弃节点暂存。
        // 必要的遍历派生量/可达集合仍属当前算法工作集，根与下一次请求不能持有它们。
        if self.retained_bytes().saturating_add(self.capture_bytes()) > self.limit {
            if let Some(c) = self.capture.as_mut() {
                c.nodes.clear();
                c.owned.clear();
                c.bytes = 0;
            }
        }
        let after_root = self
            .limit
            .saturating_sub(self.root_bytes())
            .saturating_sub(self.capture_bytes());
        self.trim_handles(after_root.saturating_sub(self.certs_bytes));
        self.trim_certs(after_root.saturating_sub(self.handles_bytes));
    }
}
/// 保留一份根原件证书（指针稳定：内容相同则复用既有 Arc，不换原件）。
fn put_root_cert(memo: &mut Memo, root_key: &str, mut cert: RootCert) -> Option<Arc<RootCert>> {
    if memo.limit == 0 {
        return None;
    }
    if let Some(old) = memo.root_certs.get(root_key) {
        if Weak::ptr_eq(&old.lineage, &Arc::downgrade(&memo.cert_lineage))
            && old.same_originals(
                &cert.node,
                &cert.slots,
                cert.expanded_length,
                &cert.content_sha256,
            )
        {
            return Some(old.clone());
        }
    }
    let cost = cert.cost().saturating_add(root_key.len());
    if let Some(old) = memo.root_certs.remove(root_key) {
        memo.certs_bytes = memo
            .certs_bytes
            .saturating_sub(old.cost().saturating_add(root_key.len()));
    }
    if !memo.make_room(cost) {
        return None;
    }
    cert.lineage = Arc::downgrade(&memo.cert_lineage);
    let old = memo.root_certs.insert(root_key.to_string(), Arc::new(cert));
    if let Some(old) = old {
        memo.certs_bytes = memo
            .certs_bytes
            .saturating_sub(old.cost().saturating_add(root_key.len()));
    }
    memo.certs_bytes = memo.certs_bytes.saturating_add(cost);
    memo.root_certs.get(root_key).cloned()
}
#[cfg(test)]
thread_local! {static LAST_RESUMED: std::cell::Cell<usize> = const {std::cell::Cell::new(0)};}
#[cfg(test)]
pub(super) fn last_resumed() -> usize {
    LAST_RESUMED.with(|v| v.get())
}
thread_local! { static MEMO: RefCell<Memo> = RefCell::new(Memo::default()); }

/// #1394 阶段计数（只在测试构建里累加，供成本/复用证据；生产构建不含这些计数）。
#[cfg(test)]
#[derive(Default, Clone, Copy)]
pub(super) struct EvidenceCounters {
    /// 从 SQLite **新鲜读入**的唯一节点行数（捕获内去重后）。
    pub node_rows_read: usize,
    /// 读入的节点负载字节。
    pub node_payload_bytes_read: usize,
    /// 同次捕获内命中临时读集、未再读 SQLite 的次数。
    pub node_reused_in_capture: usize,
    /// 本次捕获与保留原件证书逐字节比较过的唯一节点数 / 比较不符数 / 已保留证书数。
    pub cert_compared: usize,
    pub cert_mismatched: usize,
    pub cert_committed: usize,
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

/// #1394：丢掉**跨请求的持久持有**（句柄 + 原件证书 + root walk），保留配置与临时读集。
/// 用于测量真正的冷展开成本，也用于「冷/暖结果必须一致」的对照。
pub(super) fn clear_handles() {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        m.handles.clear();
        m.handles_bytes = 0;
        let ids: Vec<_> = m.certs.keys().copied().collect();
        for id in ids {
            m.remove_cert(&id);
        }
        m.root_certs.clear();
        m.certs_bytes = 0;
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
pub(super) fn retention_enabled() -> bool {
    MEMO.with(|m| m.borrow().limit != 0)
}

/// 跨请求**保留量**的当前值（测试用；供「预算在每一处都实际满足」的行为证据）。
#[cfg(test)]
pub(super) fn retained_bytes() -> usize {
    MEMO.with(|m| m.borrow().retained_bytes())
}
/// 本次捕获**瞬时读集**的当前字节（测试用；捕获结束必须回到 0）。
#[cfg(test)]
pub(super) fn capture_bytes_now() -> usize {
    MEMO.with(|m| m.borrow().capture_bytes())
}
/// 保留量分解（root walk、句柄、原件证书）——测试用账目证据。
#[cfg(test)]
pub(super) fn retained_breakdown() -> (usize, usize, usize) {
    MEMO.with(|m| {
        let m = m.borrow();
        (m.root_bytes(), m.handles_bytes, m.certs_bytes)
    })
}

// ───────────────────────── 临时读集（本次捕获） ─────────────────────────

/// 捕获作用域守卫：最外层守卫析构时清空临时读集。
///
/// 读路径每次捕获都从**新鲜行**重建临时读集；同一捕获内同一节点只读一次、只与保留证书比较
/// 一次（同次比较去重），跨捕获不复用节点行（任何节点内容/长度变化都必须在下一次捕获被重读到，
/// 不得被旧读集掩盖）。
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
/// `node_budget` 是本次捕获读集的**瞬时**安全帽（既有单次读取常量）：它不是第二个可配置的
/// 持久预算，也不是保留量上限——保留量由 `configure` 的单一预算唯一决定。两条约束同时生效。
/// 装不下只失去同次复用（当次读仍照常返回该节点）。
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
        m.borrow_mut().capture = None;
    });
}
/// 本次捕获是否已读入该节点原件（只有同一捕获内的重复引用才命中）。
pub(super) fn node_get(conn_id: usize, id: &[u8; 32]) -> Option<Arc<Cert>> {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .and_then(|c| c.nodes.get(&(conn_id, *id)).cloned())
    })
}
/// 记入本次捕获的节点原件。`owned=true` 表示这份 payload 由本次读集独占（未与持久证书共享），
/// 其字节按唯一保留量记账并在提交时转记为持久证书。
///
/// 单一预算下超限先驱逐可丢弃句柄；仍装不下只失去这次复用（当次读照常返回该节点）。
pub(super) fn node_put(conn_id: usize, cert: Arc<Cert>, owned: bool) {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.capture.is_none() {
            return;
        }
        if m.capture
            .as_ref()
            .is_some_and(|c| c.nodes.contains_key(&(conn_id, cert.id)))
        {
            return;
        }
        // 独占则连 payload 字节一起记账；共享（已由持久证书计费）只记读集条目本身。
        let cost = if owned {
            cert.payload_cost().saturating_add(32 + 64)
        } else {
            32 + 64
        };
        // 瞬时读集上限先判：装不下就直接不保留，**不**去动任何持久持有（无副作用）。
        if m.capture
            .as_ref()
            .is_some_and(|c| c.bytes.saturating_add(cost) > c.node_budget)
        {
            return;
        }
        if !m.make_room(cost) {
            return;
        }
        let id = cert.id;
        let Some(c) = m.capture.as_mut() else { return };
        c.bytes = c.bytes.saturating_add(cost);
        if owned {
            c.owned.insert((conn_id, id));
        }
        c.nodes.insert((conn_id, id), cert);
    });
}
/// 本次捕获已读入的节点数（供「同次捕获只读一次」的行为证据）。
#[cfg(test)]
pub(super) fn capture_len() -> usize {
    MEMO.with(|m| m.borrow().capture.as_ref().map_or(0, |c| c.nodes.len()))
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
/// 本次捕获是否已结算该节点的派生量（**不克隆**——热路径只问存在性，克隆会为每个节点多分配一次）。
pub(super) fn derived_has(conn_id: usize, id: &[u8; 32]) -> bool {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .is_some_and(|c| c.derived.contains_key(&(conn_id, *id)))
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
        m.borrow().capture.as_ref().map_or(0, |c| {
            c.reachable.iter().filter(|(id, _)| *id == conn_id).count()
        })
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

// ───────────────────────── 精确原件证书（跨请求保留） ─────────────────────────

/// 取该唯一节点的保留原件证书（每个 id 一份；`None` 表示没有可比较的原件）。
pub(super) fn cert_get(id: &[u8; 32]) -> Option<Arc<Cert>> {
    MEMO.with(|m| m.borrow().certs.get(id).cloned())
}
/// 记下「本次捕获已把该节点的当前原行与保留证书逐字节比较且相同」。
pub(super) fn cert_matched_mark(conn_id: usize, id: [u8; 32]) {
    MEMO.with(|m| {
        if let Some(c) = m.borrow_mut().capture.as_mut() {
            c.matched.insert((conn_id, id));
        }
    });
}
/// 该根闭包内每个唯一节点都在**本次捕获**逐字节命中原件证书（复用门的闭包那一半）。
pub(super) fn certs_all_matched(conn_id: usize, ids: &BTreeSet<[u8; 32]>) -> bool {
    MEMO.with(|m| {
        let m = m.borrow();
        let Some(c) = m.capture.as_ref() else {
            return false;
        };
        ids.iter().all(|id| c.matched.contains(&(conn_id, *id)))
    })
}
/// **保留证书侧的闭包认证**：根行原件已逐字节命中后，沿保留证书记录的有序子引用自下而上确认
/// 「闭包内每个唯一节点都在本次捕获逐字节命中原件」。
///
/// 推理是完整的：根行逐字节相同 ⟹ 根指针与有序槽相同；每个节点的 payload 逐字节相同 ⟹ 它引用的
/// 子节点 id 与顺序相同。故保留闭包与当前闭包同形，不需要再遍历当前图。已认证子树在本捕获内
/// 不再重复遍历（同次认证去重），故整次捕获的认证成本是 O(唯一节点)。
/// 任一节点缺保留证书、或本次未逐字节命中 ⟹ 不认证（只失去复用，不改变任何判据）。
/// 该节点本次是否已逐字节命中原件证书（认证自下而上的前提）。
pub(super) fn cert_marked(conn_id: usize, id: [u8; 32]) -> bool {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .is_some_and(|c| c.matched.contains(&(conn_id, id)))
    })
}
/// 记下「该节点的整棵子树本次都逐字节命中原件证书」。认证**搭在既有后序遍历上**，
/// 不另开一次遍历：调用方在子节点全部结算完之后才调用（故成本为零）。
pub(super) fn cert_certify(conn_id: usize, id: [u8; 32]) {
    MEMO.with(|m| {
        if let Some(c) = m.borrow_mut().capture.as_mut() {
            c.certified.insert((conn_id, id));
        }
    });
}
/// 该节点的整棵子树本次是否已认证（复用门的闭包那一半）。
pub(super) fn cert_is_certified(conn_id: usize, id: [u8; 32]) -> bool {
    MEMO.with(|m| {
        m.borrow()
            .capture
            .as_ref()
            .is_some_and(|c| c.certified.contains(&(conn_id, id)))
    })
}
/// 保留本次已核根闭包里的节点原件证书（共享同一份原节点，按唯一保留量记账）。
///
/// 只允许在该根**已完整展开并核过公开长度/摘要**（或本次已逐字节命中原证书）之后调用：
/// 保留证书的内容必须来自一次成功核验的原件，绝不能来自未核验的读。
pub(super) fn cert_commit(conn_id: usize, ids: &BTreeSet<[u8; 32]>) {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.limit == 0 {
            return;
        }
        for id in ids {
            let key = (conn_id, *id);
            let Some(cert) = m.capture.as_ref().and_then(|c| c.nodes.get(&key).cloned()) else {
                continue;
            };
            if m.certs.get(id).is_some_and(|old| Arc::ptr_eq(old, &cert)) {
                continue;
            }
            m.remove_cert(id);
            let owner = m.capture.as_ref().and_then(|c| {
                if c.owned.contains(&key) {
                    return Some(key);
                }
                c.owned
                    .iter()
                    .find(|key| {
                        c.nodes
                            .get(key)
                            .is_some_and(|held| Arc::ptr_eq(held, &cert))
                    })
                    .copied()
            });
            let owned = owner.is_some();
            if !owned && !m.make_room(cert.payload_cost()) {
                continue;
            }
            if owned {
                // 独占：字节已由读集持有，只把它从瞬时读集**转记**为持久证书（总量不变）。
                if let Some(c) = m.capture.as_mut() {
                    c.bytes = c.bytes.saturating_sub(cert.payload_cost());
                    c.owned.remove(&owner.unwrap());
                }
            }
            m.certs_bytes = m.certs_bytes.saturating_add(cert.payload_cost());
            m.certs.insert(*id, cert);
            #[cfg(test)]
            counter(|c| c.cert_committed += 1);
        }
    });
}
/// 冷展开后，只有全部当前原件仍在池里且与本次读集为同一不可变原件，才能建立新根证明。
pub(super) fn certify_retained_originals(conn_id: usize, ids: &BTreeSet<[u8; 32]>) -> bool {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        let Some(c) = &m.capture else { return false };
        let complete = ids
            .iter()
            .all(|id| match (m.certs.get(id), c.nodes.get(&(conn_id, *id))) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            });
        // 已完整展开核验、且整个闭包仍持有同一份原件：本次捕获后续读取可复用刚建立的证明。
        if complete {
            let c = m.capture.as_mut().unwrap();
            for id in ids {
                c.matched.insert((conn_id, *id));
                c.certified.insert((conn_id, *id));
            }
        }
        complete
    })
}
/// 旧依赖未被逐字节确认时撤销该根的旧证明，即便根行/摘要本身未变。
pub(super) fn invalidate_root(root_key: &str) {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if let Some(old) = m.root_certs.remove(root_key) {
            m.certs_bytes = m
                .certs_bytes
                .saturating_sub(old.cost().saturating_add(root_key.len()));
        }
        if let Some(old) = m.handles.remove(root_key) {
            m.handles_bytes = m.handles_bytes.saturating_sub(old.cost);
        }
    });
}
/// 当前保留的根原件证书（**指针稳定**：同一原件跨捕获返回同一个 Arc）。
pub(super) fn root_cert_get(root_key: &str) -> Option<Arc<RootCert>> {
    MEMO.with(|m| m.borrow().root_certs.get(root_key).cloned())
}
/// 把本次新鲜读到的根行原件与保留证书**逐字节比较**：相同返回该保留证书，不同/缺失返回 `None`。
pub(super) fn root_cert_match(
    root_key: &str,
    node: &[u8; 32],
    slots: &[[u8; 32]],
    expanded_length: u64,
    content_sha256: &str,
) -> Option<Arc<RootCert>> {
    MEMO.with(|m| {
        let m = m.borrow();
        m.root_certs
            .get(root_key)
            .filter(|c| {
                Weak::ptr_eq(&c.lineage, &Arc::downgrade(&m.cert_lineage))
                    && c.same_originals(node, slots, expanded_length, content_sha256)
            })
            .cloned()
    })
}

// ───────────────────────── 持久句柄（跨请求复用） ─────────────────────────

/// 取**图域**已核根句柄：要求 (a) 该句柄绑定的根原件证书与当前保留证书是同一个原件
/// （指针同一），(b) 闭包指纹一致（附加校验）。闭包内每个唯一节点的原件已由调用方在本次捕获
/// 逐字节比较过（`certs_all_matched`），故复用不改变任何判据。
pub(super) fn handle_get_certified(
    root_key: &str,
    root_cert: &Arc<RootCert>,
    closure: &[u8; 32],
) -> Option<(Arc<[u8]>, Option<Arc<Value>>)> {
    MEMO.with(|m| {
        let m = m.borrow();
        m.handles
            .get(root_key)
            .filter(|h| {
                Weak::ptr_eq(&root_cert.lineage, &Arc::downgrade(&m.cert_lineage))
                    && h.closure == *closure
                    && h.root_cert
                        .as_ref()
                        .is_some_and(|c| Weak::ptr_eq(c, &Arc::downgrade(root_cert)))
            })
            .map(|h| (h.bytes.clone(), h.value.clone()))
    })
}
/// 取**整块域**已核句柄：原件就是行内字节，故复用要求保留字节与本次新鲜字节**逐字节相同**
/// （不是摘要相等）；闭包指纹一致只是附加校验。
pub(super) fn handle_get_blob(
    root_key: &str,
    fresh: &[u8],
    digest: &[u8; 32],
) -> Option<(Arc<[u8]>, Option<Arc<Value>>)> {
    MEMO.with(|m| {
        m.borrow()
            .handles
            .get(root_key)
            .filter(|h| h.root_cert.is_none() && h.closure == *digest && h.bytes.as_ref() == fresh)
            .map(|h| (h.bytes.clone(), h.value.clone()))
    })
}
/// 记入图域已核根句柄，并保留该根的**原件证书**（指针稳定）。
///
/// 只有在本次已完整展开并核过公开长度/摘要、或本次已逐字节命中原证书之后才可调用。
/// 装不下只失去这次复用，不改任何判据；也不整块清空其他仍可复用的句柄。
/// 返回保留下来的根原件证书（审计前缀的原件证据）；`None` 表示本次无法保留。
pub(super) fn handle_put_certified(
    root_key: &str,
    root_cert: &RootCert,
    closure: [u8; 32],
    bytes: Arc<[u8]>,
    value: Option<Arc<Value>>,
) -> Option<Arc<RootCert>> {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.limit == 0 {
            return None;
        }
        // 根证书先保留（小，且是审计前缀与逐字节比较的原件证据）。
        let retained = put_root_cert(
            &mut m,
            root_key,
            RootCert::new(
                root_cert.node,
                root_cert.slots.clone(),
                root_cert.expanded_length,
                root_cert.content_sha256.clone(),
            ),
        )?;
        let cost = handle_cost(root_key.len(), bytes.len(), value.is_some());
        if let Some(old) = m.handles.remove(root_key) {
            m.handles_bytes = m.handles_bytes.saturating_sub(old.cost);
        }
        if !m.make_room(cost) {
            return Some(retained);
        }
        m.handles_bytes = m.handles_bytes.saturating_add(cost);
        m.handles.insert(
            root_key.to_string(),
            Handle {
                closure,
                root_cert: Some(Arc::downgrade(&retained)),
                bytes,
                value,
                cost,
            },
        );
        Some(retained)
    })
}
/// 记入整块域已核句柄（原件就是行内字节，无需根证书）。
pub(super) fn handle_put_blob(
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
        let cost = handle_cost(root_key.len(), bytes.len(), value.is_some());
        if let Some(old) = m.handles.remove(root_key) {
            m.handles_bytes = m.handles_bytes.saturating_sub(old.cost);
        }
        if !m.make_room(cost) {
            return;
        }
        m.handles_bytes = m.handles_bytes.saturating_add(cost);
        m.handles.insert(
            root_key.to_string(),
            Handle {
                closure,
                root_cert: None,
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

/// 审计前缀复用：前缀行逐字节相同，且旧捕获的每个私有根仍是同一份有效原件证明。
/// 新根可加入；旧根缺失、未保留、旋转或原件池失效均须重核。末列摘要只作附加校验。
pub(super) fn resume(
    rows: &[DeltaRow],
    identity: &Identity,
    walk: &mut audit::AuditWalk,
    originals: &Originals,
) -> usize {
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
        let graph3 = identity.4 == evidence_store::GRAPH_FORMAT;
        let graph = graph3 || identity.4 == evidence_store::GRAPH_FORMAT_V2;
        // 图域不得因两边漏收同一根而空集放行。graph/2 的 Delta 原件仍在行内。
        if graph
            && old.rows.iter().any(|row| {
                !old.originals
                    .contains_key(&evidence_store::batch_root_key(&row.6))
                    || (graph3
                        && !old
                            .originals
                            .contains_key(&evidence_store::delta_root_key(row.0)))
            })
        {
            return 0;
        }
        for (key, old_cert) in &old.originals {
            match (old_cert, originals.get(key)) {
                (Some(a), Some(Some(b)))
                    if Weak::ptr_eq(a, &Arc::downgrade(b))
                        && Weak::ptr_eq(&b.lineage, &Arc::downgrade(&m.cert_lineage)) => {}
                _ => return 0,
            }
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
pub(super) fn store_root(
    rows: Vec<DeltaRow>,
    identity: Identity,
    walk: audit::AuditWalk,
    originals: &Originals,
) {
    let bytes = root_cost(&rows, &identity, &walk, originals);
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
            // 即使新 root 装不下，保留量仍必须在同一上限内（旧 root 可能仍占着大头）。
            m.fit_retention();
            return;
        }
        m.root = Some(Root {
            rows,
            identity,
            originals: originals
                .iter()
                .map(|(key, cert)| (key.clone(), cert.as_ref().map(Arc::downgrade)))
                .collect(),
            walk,
            bytes,
        });
        // root walk 优先装入，再按剩余预算驱逐句柄与原件证书（不因新 root 变大而整块清空）。
        m.fit_retention();
    });
}

/// root 记账：行内字符串 + 每行 16B + walk + identity + 每个证明键及条目 16B。
/// 证明原件仍只在原件池记一份；此处不复制闭包或解码值。全程饱和加，避免溢出。
fn root_cost(
    rows: &[DeltaRow],
    identity: &Identity,
    walk: &audit::AuditWalk,
    originals: &Originals,
) -> usize {
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
        .saturating_add(originals.keys().fold(0usize, |n, key| {
            n.saturating_add(key.len()).saturating_add(16)
        }))
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
    fn inline_identity(session: &str) -> Identity {
        (
            session.to_string(),
            "catalog-hash".to_string(),
            ("profile-id".to_string(), "profile-hash".to_string()),
            delta_storage::CHANGES_ONLY.to_string(),
            evidence_store::Format::LegacyBlob.tag().to_string(),
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
    fn cert(id: u8, kind: i64, payload: &[u8], length: u64) -> Arc<Cert> {
        Arc::new(Cert {
            id: [id; 32],
            kind,
            payload: Arc::from(payload),
            expanded_length: length,
            children: Vec::new(),
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
    fn root_cert(node: u8, slots: &[u8], len: u64, hash: &str) -> Arc<RootCert> {
        Arc::new(RootCert::new(
            [node; 32],
            slots.iter().map(|s| [*s; 32]).collect(),
            len,
            hash.to_string(),
        ))
    }
    fn inline_originals() -> Originals {
        Originals::new()
    }
    fn retained_originals(rows: &[DeltaRow], graph3: bool) -> Originals {
        let mut keys = rows
            .iter()
            .map(|row| evidence_store::batch_root_key(&row.6))
            .collect::<Vec<_>>();
        if graph3 {
            keys.extend(rows.iter().map(|row| evidence_store::delta_root_key(row.0)));
            keys.push(evidence_store::fact_root_key("fact-1"));
        }
        keys.into_iter()
            .map(|key| {
                let retained = handle_put_certified(
                    &key,
                    &RootCert::new([1; 32], Vec::new(), 2, "hash".to_string()),
                    [0; 32],
                    Arc::from(&b"{}"[..]),
                    None,
                );
                assert!(retained.is_some());
                (key, retained)
            })
            .collect()
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
    fn memo_retained() -> usize {
        MEMO.with(|m| m.borrow().retained_bytes())
    }

    #[test]
    fn graph3_missing_both_delta_certificates_never_resumes() {
        configure(1 << 20);
        let mut graph = inline_identity("test-c");
        graph.4 = evidence_store::GRAPH_FORMAT.to_string();
        let complete = retained_originals(&rows(2), true);
        for missing in [None, Some(None)] {
            let mut originals = complete.clone();
            match missing {
                None => {
                    originals.remove("delta/1");
                }
                Some(value) => {
                    originals.insert("delta/1".to_string(), value);
                }
            }
            store_root(rows(2), graph.clone(), empty_walk(), &originals);
            assert_eq!(resume(&rows(2), &graph, &mut empty_walk(), &originals), 0);
        }
        configure(0);
    }

    #[test]
    fn graph_domains_require_every_historical_batch_proof_and_nonempty_maps() {
        configure(1 << 20);
        for graph3 in [false, true] {
            let mut graph = inline_identity("test-c");
            graph.4 = if graph3 {
                evidence_store::GRAPH_FORMAT
            } else {
                evidence_store::GRAPH_FORMAT_V2
            }
            .to_string();
            let mut originals = retained_originals(&rows(2), graph3);
            originals.remove("batch/batch-1");
            store_root(rows(2), graph.clone(), empty_walk(), &originals);
            assert_eq!(resume(&rows(2), &graph, &mut empty_walk(), &originals), 0);
            store_root(rows(2), graph.clone(), empty_walk(), &Originals::new());
            assert_eq!(
                resume(&rows(2), &graph, &mut empty_walk(), &Originals::new()),
                0
            );
        }
        configure(0);
    }

    #[test]
    fn graph3_parameter_originals_and_cold_proof_are_bound_to_the_root() {
        configure(1 << 20);
        let conn = Connection::open_in_memory().unwrap();
        evidence_store::declare_new(&conn).unwrap();
        let staged = evidence_store::stage_complete(
            &json!({"v": 9}),
            &[evidence_store::SlotSpec::number("9")],
        )
        .unwrap();
        evidence_store::publish_refs(&conn, "batch/proof", &staged).unwrap();
        let first = evidence_store::Reader::new(&conn, 1 << 20)
            .read_root("batch/proof", false)
            .unwrap();
        let (_, slots, _, _) = evidence_store::Reader::root_row(&conn, "batch/proof")
            .unwrap()
            .unwrap();
        assert!(!slots.is_empty());
        assert!(
            slots.iter().all(|id| cert_get(id).is_some()),
            "参数槽原件必须随根保留"
        );
        let warm = evidence_store::Reader::new(&conn, 1 << 20)
            .read_root("batch/proof", false)
            .unwrap();
        assert!(warm.originals_verified && warm.reused);
        // 人为移除一份原件，模拟证书不可保留；数据库和所有摘要都保持不变。
        MEMO.with(|m| {
            let mut m = m.borrow_mut();
            let old = m.certs.remove(&slots[0]).unwrap();
            m.certs_bytes -= old.payload_cost();
        });
        let cold = evidence_store::Reader::new(&conn, 1 << 20)
            .read_root("batch/proof", false)
            .unwrap();
        assert_eq!(first.bytes, cold.bytes);
        assert!(!cold.originals_verified && !cold.reused);
        assert!(
            !Arc::ptr_eq(
                first.root_cert.as_ref().unwrap(),
                cold.root_cert.as_ref().unwrap()
            ),
            "未比较齐旧依赖时不得返回旧根证明供前缀复用"
        );
        configure(0);
    }

    #[test]
    fn certificate_eviction_accounts_for_still_owned_original_bytes() {
        configure(1 << 20);
        let _capture = begin_capture(1 << 20);
        node_put(7, cert(4, 0, &[0u8; 4096], 4096), true);
        cert_commit(7, &BTreeSet::from([[4u8; 32]]));
        MEMO.with(|m| {
            let mut m = m.borrow_mut();
            m.trim_certs(0);
            // 独立从仍持有的 Arc 读取真实 payload；不以被测计数器自证。
            let payload = m
                .capture
                .as_ref()
                .unwrap()
                .nodes
                .values()
                .map(|c| c.payload.len())
                .sum::<usize>();
            assert!(
                m.total_bytes() >= payload,
                "被驱逐索引中的原件仍被 Capture 持有，不能漏记"
            );
        });
        end_capture();
        assert_eq!(memo_total(), 0);
        configure(0);
    }

    #[test]
    fn evicted_certificates_cannot_be_retained_or_reused_by_old_root_handles() {
        configure(1 << 20);
        let original = root_cert(1, &[], 2, "hash");
        let retained =
            handle_put_certified("delta/1", &original, [0; 32], Arc::from(&b"{}"[..]), None)
                .unwrap();
        let weak = Arc::downgrade(&retained);
        store_root(
            rows(1),
            inline_identity("test-c"),
            empty_walk(),
            &BTreeMap::from([("delta/1".to_string(), Some(retained.clone()))]),
        );
        drop(retained);
        MEMO.with(|m| m.borrow_mut().trim_certs(0));
        assert!(
            weak.upgrade().is_none(),
            "旧 root/walk 不得持有已从预算索引移除的根证书"
        );

        let kept = handle_put_certified("delta/1", &original, [0; 32], Arc::from(&b"{}"[..]), None)
            .unwrap();
        {
            let _capture = begin_capture(1 << 20);
            node_put(7, cert(4, 0, b"x", 1), true);
            cert_commit(7, &BTreeSet::from([[4; 32]]));
        }
        assert!(handle_get_certified("delta/1", &kept, &[0; 32]).is_some());
        MEMO.with(|m| m.borrow_mut().remove_cert(&[4; 32]));
        assert!(
            handle_get_certified("delta/1", &kept, &[0; 32]).is_none(),
            "原件池丢失依赖后，旧根证明不能依赖同一摘要重新生效"
        );
        configure(0);
    }

    #[test]
    fn root_survives_and_handles_fill_only_the_shared_budget() {
        let base = rows(2);
        let root = root_cost(
            &base,
            &inline_identity("test-c"),
            &empty_walk(),
            &inline_originals(),
        );
        let one = handle_cost("batch-a".len(), 64, true);
        let limit = root + one * 3;
        configure(limit);
        store_root(
            base,
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        for (key, len) in [
            ("batch-a", 64),
            ("batch-bb", 1 << 20),
            ("batch-c", 64),
            ("batch-d", 64),
        ] {
            let (bytes, value) = handle(key, len, true);
            handle_put_blob(key, [0u8; 32], bytes, value);
        }
        // root 优先保留；单条过大句柄跳过而不牵连其他句柄。
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_handle_keys(), vec!["batch-a", "batch-c", "batch-d"]);
        assert!(memo_total() <= limit);
        assert!(handle_get_blob("batch-a", &[b'x'; 64], &[0u8; 32]).is_some());
        assert!(handle_get_blob("batch-a", &[b'x'; 64], &[1u8; 32]).is_none());
        assert!(handle_get_blob("batch-bb", &[b'x'; 64], &[0u8; 32]).is_none());
    }

    #[test]
    fn root_proof_keys_and_entries_count_toward_the_same_budget() {
        configure(0);
        let rows = rows(2);
        let identity = inline_identity("test-c");
        let base = root_cost(&rows, &identity, &empty_walk(), &inline_originals());
        let originals = BTreeMap::from([(format!("fact/{}", "x".repeat(4096)), None)]);
        let cost = root_cost(&rows, &identity, &empty_walk(), &originals);
        assert_eq!(cost - base, 4101 + 16);
        configure(cost - 1);
        store_root(rows.clone(), identity.clone(), empty_walk(), &originals);
        assert_eq!(memo_root_rows(), None, "证明键不能绕过 root 保留预算");
        configure(cost);
        store_root(rows, identity, empty_walk(), &originals);
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_retained(), cost);
        configure(0);
    }

    #[test]
    fn growing_root_evicts_handles_instead_of_dropping_everything() {
        let small = rows(2);
        let big = rows(6);
        let big_bytes = root_cost(
            &big,
            &inline_identity("test-c"),
            &empty_walk(),
            &inline_originals(),
        );
        let one = handle_cost("batch-a".len(), 64, true);
        let limit = big_bytes + one;
        configure(limit);
        store_root(
            small,
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        for key in ["batch-a", "batch-b"] {
            let (bytes, value) = handle(key, 64, true);
            handle_put_blob(key, [0u8; 32], bytes, value);
        }
        assert_eq!(memo_handle_keys(), vec!["batch-a", "batch-b"]);
        store_root(
            big,
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        assert_eq!(memo_root_rows(), Some(6));
        assert_eq!(memo_handle_keys(), vec!["batch-a"]); // 句柄部分驱逐，root 保留
        assert!(memo_total() <= limit);
    }

    #[test]
    fn oversized_new_root_keeps_previous_verified_prefix_usable() {
        let small = rows(2);
        let huge = rows(64);
        let small_bytes = root_cost(
            &small,
            &inline_identity("test-c"),
            &empty_walk(),
            &inline_originals(),
        );
        let one = handle_cost("batch-a".len(), 64, true);
        let limit = small_bytes + one;
        assert!(
            root_cost(
                &huge,
                &inline_identity("test-c"),
                &empty_walk(),
                &inline_originals()
            ) > limit
        );
        configure(limit);
        store_root(
            small,
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put_blob("batch-a", [0u8; 32], bytes, value);
        store_root(
            huge.clone(),
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        // 装不下新 root：旧前缀与句柄都保留，不清整个 Memo。
        assert_eq!(memo_root_rows(), Some(2));
        assert_eq!(memo_handle_keys(), vec!["batch-a"]);
        assert!(memo_total() <= limit);
        assert_eq!(
            resume(
                &huge,
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            2
        );
        assert_eq!(last_resumed(), 2);
        // 前缀不再匹配时必须作废，不留死前缀、不误用。
        let mut changed = huge.clone();
        changed[0].6 = "batch-9".to_string();
        assert_eq!(
            resume(
                &changed,
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            0
        );
        assert_eq!(last_resumed(), 0);
    }

    /// #1394：原件证明逐键绑定，不能由相同摘要、遗漏键或失效谱系替代。
    #[test]
    fn audit_prefix_requires_the_same_verified_originals_not_only_the_digest() {
        configure(1 << 20);
        let rows3 = rows(3);
        let mut graph = inline_identity("test-c");
        graph.4 = evidence_store::GRAPH_FORMAT.to_string();
        let originals = retained_originals(&rows3, true);
        store_root(rows3.clone(), graph.clone(), empty_walk(), &originals);
        assert_eq!(resume(&rows3, &graph, &mut empty_walk(), &originals), 3);

        for key in originals.keys() {
            // 每类根都覆盖缺键、显式 None 和同原文却不同对象的证明。
            let mut changed = originals.clone();
            changed.remove(key);
            assert_eq!(
                resume(&rows3, &graph, &mut empty_walk(), &changed),
                0,
                "{key}"
            );
            changed.insert(key.clone(), None);
            assert_eq!(
                resume(&rows3, &graph, &mut empty_walk(), &changed),
                0,
                "{key}"
            );
            let current = originals[key].as_ref().unwrap();
            let mut rotated = RootCert::new(
                current.node,
                current.slots.clone(),
                current.expanded_length,
                current.content_sha256.clone(),
            );
            rotated.lineage = current.lineage.clone();
            changed.insert(key.clone(), Some(Arc::new(rotated)));
            assert_eq!(
                resume(&rows3, &graph, &mut empty_walk(), &changed),
                0,
                "{key}"
            );
        }
        let expanded = retained_originals(&rows(4), true);
        assert_eq!(
            resume(&rows(4), &graph, &mut empty_walk(), &expanded),
            3,
            "新根加入不应废掉完整旧前缀"
        );
        // 原件池失效即使指针相同也不得复用。
        MEMO.with(|m| m.borrow_mut().cert_lineage = Arc::new(()));
        assert_eq!(resume(&rows3, &graph, &mut empty_walk(), &originals), 0);

        // 真实整块域按行内列与审计原规则复用，不假借 graph/2 缺证明的宽松分支。
        store_root(
            rows3.clone(),
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        assert_eq!(
            resume(
                &rows3,
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            3
        );
        configure(0);
    }

    /// #1394 第三段缓存合同：**摘要值相同但原件不同**时，图域句柄一律不复用。
    /// 覆盖根行、节点 payload、节点声明长度、槽顺序与子依赖原件五处；本测试只断言缓存层
    /// 的复用门，不声称构造生产 SHA256 碰撞。
    #[test]
    fn identical_digest_with_different_originals_never_reuses() {
        configure(1 << 20);
        let digest = [9u8; 32];
        let slots = vec![[5u8; 32], [6u8; 32]];
        let wanted = root_cert(7, &[5, 6], 64, "hash-a");
        let (bytes, value) = handle("delta/1", 64, true);
        let retained = handle_put_certified(
            "delta/1",
            &RootCert::new(
                wanted.node,
                wanted.slots.clone(),
                wanted.expanded_length,
                wanted.content_sha256.clone(),
            ),
            digest,
            bytes,
            value,
        )
        .expect("根证书应当被保留");
        let node = [7u8; 32];
        // 正路：根行原件逐字节相同 ⟹ 可复用。
        assert!(root_cert_match("delta/1", &node, &slots, 64, "hash-a").is_some());
        assert!(handle_get_certified("delta/1", &retained, &digest).is_some());
        // 摘要相同、内容也相同的**另一份**证书对象仍不是同一个原件：不得命中句柄。
        let other_object = Arc::new(RootCert::new(node, slots.clone(), 64, "hash-a".to_string()));
        assert!(
            handle_get_certified("delta/1", &other_object, &digest).is_none(),
            "另一份证书对象不得冒充同一原件"
        );
        // ① 根行原件不同（槽顺序调换）⟹ 不复用。
        let swapped = vec![[6u8; 32], [5u8; 32]];
        assert!(root_cert_match("delta/1", &node, &swapped, 64, "hash-a").is_none());
        // ② 根行原件不同（声明长度）⟹ 不复用。
        assert!(root_cert_match("delta/1", &node, &slots, 65, "hash-a").is_none());
        // ③ 根行原件不同（内容摘要）⟹ 不复用。
        assert!(root_cert_match("delta/1", &node, &slots, 64, "hash-b").is_none());
        // ④ 根行原件不同（根指针）⟹ 不复用。
        assert!(root_cert_match("delta/1", &[8u8; 32], &slots, 64, "hash-a").is_none());
        // ⑤ 节点原件：payload / 声明长度 / 种类任一不同都判为不同（逐字节，不是摘要）。
        assert!(cert_get(&[1u8; 32]).is_none()); // 尚未保留任何节点证书
        let _g = begin_capture(1 << 20);
        node_put(0, cert(1, 1, b"same-payload", 12), true);
        let ids = BTreeSet::from([[1u8; 32]]);
        cert_commit(0, &ids);
        let kept = cert_get(&[1u8; 32]).expect("节点证书应被保留");
        assert!(kept.same_originals(1, b"same-payload", 12));
        assert!(!kept.same_originals(1, b"same-payloaD", 12)); // payload 差一个字节
        assert!(!kept.same_originals(1, b"same-payload", 13)); // 声明长度不同
        assert!(!kept.same_originals(0, b"same-payload", 12)); // 种类不同
                                                               // ⑥ 子依赖原件变化：父节点 payload 逐字节不变（子引用相同），但子节点原件不同 ⟹
                                                               // 闭包门必须为假（全闭包逐字节命中才允许复用）。
        let mut parent = cert(2, 1, b"parent-refs-child", 40);
        Arc::get_mut(&mut parent).unwrap().children = vec![[1u8; 32]];
        node_put(0, parent, true);
        let closure = BTreeSet::from([[2u8; 32], [1u8; 32]]);
        cert_commit(0, &closure);
        cert_matched_mark(0, [1u8; 32]);
        assert!(!certs_all_matched(0, &closure)); // 父节点尚未在本次捕获逐字节命中
        cert_matched_mark(0, [2u8; 32]);
        assert!(certs_all_matched(0, &closure));
        // ⑦ 保留证书侧认证：父节点 payload 逐字节不变（子引用相同），但子节点原件不同（未命中）
        // ⟹ 认证失败；两个节点都逐字节命中才认证通过。
        assert!(cert_get(&[2u8; 32]).is_some(), "父节点证书应已保留");
        // 上一段捕获的命中标记不得泄漏到这一段（每次捕获都要重新逐字节比较）。
        end_capture();
        let _g2 = begin_capture(1 << 20);
        assert!(!cert_is_certified(0, [2u8; 32]), "本捕获尚未认证任何节点");
        cert_matched_mark(0, [2u8; 32]); // 父节点自己逐字节命中
        assert!(
            !cert_is_certified(0, [2u8; 32]),
            "子依赖原件不同（子未命中 ⟹ 未认证）不得认证父节点"
        );
        cert_matched_mark(0, [1u8; 32]); // 子节点逐字节命中
        cert_certify(0, [1u8; 32]); // 子节点结算时认证
        cert_certify(0, [2u8; 32]); // 父节点结算：直接子节点已全部认证
        assert!(cert_is_certified(0, [2u8; 32]));
        // 认证与命中标记都只在本次捕获有效。
        end_capture();
        let _g3 = begin_capture(1 << 20);
        assert!(!cert_is_certified(0, [2u8; 32]));
        assert!(!certs_all_matched(0, &BTreeSet::from([[1u8; 32]])));
        configure(0);
    }

    /// #1394 第三段：单一预算同时约束读集与保留量；超限先驱逐可丢弃句柄，仍不够只失去复用，
    /// 绝不把限额不足判成存储损坏。
    #[test]
    fn single_budget_bounds_read_set_and_retained_certificates() {
        configure(1 << 20);
        let _g = begin_capture(1 << 20);
        node_put(7, cert(4, 1, &[0u8; 10], 10), true);
        assert!(node_get(7, &[4u8; 32]).is_some());
        assert_eq!(capture_len(), 1);
        let ids = BTreeSet::from([[4u8; 32]]);
        cert_commit(7, &ids);
        // 独占字节从瞬时读集**转记**为持久证书：总量不变（共享原件按唯一保留量记账）。
        assert!(cert_get(&[4u8; 32]).is_some());
        assert_eq!(memo_retained(), 10 + 64);
        end_capture();
        assert!(cert_get(&[4u8; 32]).is_some(), "持久证书跨捕获保留");
        assert_eq!(memo_total(), 10 + 64);

        // 单一预算收紧到装不下：先驱逐句柄，再放弃读集/证书。
        configure(20);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put_blob("batch-a", [0u8; 32], bytes, value);
        assert!(memo_retained() <= 20);
        let _g2 = begin_capture(1 << 20);
        node_put(7, cert(5, 1, &[0u8; 64], 64), true);
        assert!(node_get(7, &[5u8; 32]).is_none(), "装不下只失去读集保留");
        assert!(memo_total() <= 20);
        end_capture();
        configure(0);
        assert_eq!(memo_total(), 0);
    }

    #[test]
    fn verified_handle_requires_identical_closure_and_identity_are_checked() {
        configure(4096);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put_blob("batch-a", [7u8; 32], bytes, value);
        assert!(handle_get_blob("batch-a", &[b'x'; 64], &[7u8; 32]).is_some());
        assert!(handle_get_blob("batch-a", &[b'x'; 64], &[8u8; 32]).is_none());
        // 整块域的原件就是行内字节：字节不同即不复用，即使摘要相同。
        assert!(handle_get_blob("batch-a", &[b'y'; 64], &[7u8; 32]).is_none());
        store_root(
            rows(3),
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        assert_eq!(
            resume(
                &rows(5),
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            3
        );
        assert_eq!(
            resume(
                &rows(5),
                &inline_identity("other-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            0
        );
        let mut changed = rows(5);
        changed[1].4 = "cut-99".to_string();
        assert_eq!(
            resume(
                &changed,
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            0
        );
        assert_eq!(
            resume(
                &rows(2),
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            0
        );
    }

    #[test]
    fn zero_limit_disables_cache_entirely() {
        configure(0);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put_blob("batch-a", [0u8; 32], bytes, value);
        store_root(
            rows(2),
            inline_identity("test-c"),
            empty_walk(),
            &inline_originals(),
        );
        node_put(7, cert(9, 0, b"xx", 2), true);
        assert_eq!(memo_root_rows(), None);
        assert!(memo_handle_keys().is_empty());
        assert_eq!(memo_total(), 0);
        assert!(handle_get_blob("batch-a", &[b'x'; 64], &[0u8; 32]).is_none());
        assert!(node_get(7, &[9u8; 32]).is_none());
        assert!(cert_get(&[9u8; 32]).is_none());
        assert_eq!(
            resume(
                &rows(2),
                &inline_identity("test-c"),
                &mut empty_walk(),
                &inline_originals()
            ),
            0
        );
        assert_eq!(last_resumed(), 0);
    }

    /// #1394：临时读集的节点原件只在**同一次捕获**内复用；捕获结束即作废，不得跨捕获。
    #[test]
    fn capture_scope_is_temporary_and_never_spans_captures() {
        configure(1 << 20);
        {
            let _guard = begin_capture(1 << 20);
            node_put(7, cert(1, 0, b"payload", 7), true);
            assert!(node_get(7, &[1u8; 32]).is_some());
        }
        // 捕获结束：新鲜行必须重新读入，旧临时读集不得继续命中。
        assert!(node_get(7, &[1u8; 32]).is_none());
        {
            let _outer = begin_capture(1 << 20);
            node_put(7, cert(2, 0, b"payload", 7), true);
            {
                // 嵌套捕获复用外层读集，内层析构不得提前清空。
                let _inner = begin_capture(1 << 20);
                assert!(node_get(7, &[2u8; 32]).is_some());
            }
            assert!(node_get(7, &[2u8; 32]).is_some());
        }
        assert!(node_get(7, &[2u8; 32]).is_none());
    }

    /// #1394：写前作废临时读集，写后重读必须重新读新鲜行。
    #[test]
    fn end_capture_drops_the_temporary_read_set() {
        configure(1 << 20);
        let _outer = begin_capture(1 << 20);
        node_put(7, cert(3, 0, b"payload", 7), true);
        assert!(node_get(7, &[3u8; 32]).is_some());
        end_capture();
        assert!(node_get(7, &[3u8; 32]).is_none());
    }

    /// #1394 第三段：读集的**瞬时**安全帽是本次捕获的读集上限；它与单一预算同时约束，
    /// 且单一预算是保留量的唯一上限——两个上限不得被当成一个「total」混算。
    #[test]
    fn transient_read_set_cap_and_single_budget_both_bind() {
        // 读集条目记账 = payload + 64（独占字节）+ 32 + 64（条目本身）。
        let node_cost = 10 + 64 + 32 + 64;
        let one = handle_cost("batch-a".len(), 64, true);
        configure(one + 16);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put_blob("batch-a", [0u8; 32], bytes, value);
        {
            // 瞬时读集上限比任何一条读集条目都小：只失去同次复用，不牵连已在预算内的句柄。
            let _guard = begin_capture(16);
            node_put(7, cert(4, 0, &[0u8; 10], 10), true);
            assert!(node_get(7, &[4u8; 32]).is_none());
            assert_eq!(capture_len(), 0);
            assert_eq!(memo_total(), one);
        }
        {
            // 装得下瞬时上限、但单一预算不够：先驱逐**可丢弃**的持久句柄给读集让位。
            let _guard = begin_capture(1 << 20);
            node_put(7, cert(5, 0, &[0u8; 10], 10), true);
            assert!(node_get(7, &[5u8; 32]).is_some());
            assert!(memo_handle_keys().is_empty(), "读集加入即驱逐旧持久句柄");
            assert_eq!(memo_total(), node_cost);
            assert!(memo_total() <= one + 16);
        }
        // 捕获结束：临时读集释放。
        assert_eq!(memo_total(), 0);
    }

    /// #1394 第三段：单条过大只跳过自己（不因装不下而驱逐其他仍可复用的句柄）。
    #[test]
    fn oversized_item_never_evicts_reusable_handles() {
        let one = handle_cost("batch-a".len(), 64, true);
        configure(one + 16);
        let (bytes, value) = handle("batch-a", 64, true);
        handle_put_blob("batch-a", [0u8; 32], bytes, value);
        // 单条 1MiB 句柄在任何没有句柄的状态下都装不下：其他句柄必须原样留着。
        let (big, big_value) = handle("batch-big", 1 << 20, true);
        handle_put_blob("batch-big", [0u8; 32], big, big_value);
        assert_eq!(memo_handle_keys(), vec!["batch-a"]);
        assert_eq!(memo_total(), one);
        // 单条超限的读集条目同理：不动句柄。
        let _guard = begin_capture(1 << 20);
        node_put(7, cert(6, 0, &vec![0u8; one + 16], (one + 16) as u64), true);
        assert!(node_get(7, &[6u8; 32]).is_none());
        assert_eq!(memo_handle_keys(), vec!["batch-a"]);
        assert_eq!(memo_total(), one);
    }

    /// #1394 第三段：`configure(0)` 下不留任何跨请求保留（句柄、证书、读集都不留），
    /// 读路径仍必须能无缓存地严格重核。
    #[test]
    fn zero_limit_keeps_no_retention_at_all() {
        configure(0);
        {
            let _guard = begin_capture(1 << 20);
            node_put(7, cert(6, 0, b"payload", 7), true);
            assert!(node_get(7, &[6u8; 32]).is_none());
            let (bytes, value) = handle("batch-a", 64, true);
            handle_put_blob("batch-a", [0u8; 32], bytes, value);
            assert!(handle_get_blob("batch-a", &[b'x'; 64], &[0u8; 32]).is_none());
        }
        assert_eq!(memo_total(), 0);
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
        let originals = retained_originals(&rows(3), false);
        store_root(
            rows(3),
            with_format(evidence_store::GRAPH_FORMAT_V2),
            empty_walk(),
            &originals,
        );
        assert_eq!(
            resume(
                &rows(3),
                &with_format(evidence_store::GRAPH_FORMAT_V2),
                &mut empty_walk(),
                &originals
            ),
            3
        );
        assert_eq!(
            resume(
                &rows(3),
                &with_format(evidence_store::GRAPH_FORMAT),
                &mut empty_walk(),
                &originals
            ),
            0
        );
        assert_eq!(last_resumed(), 0);
        configure(0);
    }
}
