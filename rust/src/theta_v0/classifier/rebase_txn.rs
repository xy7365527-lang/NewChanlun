//! #543 D1a：`RebaseTransformTxnV1` 构造证书 seam（**纯观测**，行为零变化）。
//!
//! ## 为什么需要这条 seam（调研 `.chanlun/review-results/rebase-identity-d1-research-20260728.md` §5.1）
//!
//! 现行三流（`trades` / `tower_events` / `center_lifecycle` / `rebase_observability`）只能看见
//! 「旧 `CenterId` 三元组消失、新三元组出现」。`tower_events` 按向量下标对齐旧、新 `LeveledMove`，
//! 同下标只要 `end_index` 变就记 `extend`——它会把**整窗替换**伪装成延伸（seq=66 的 `L2#121` 实锤：
//! `ordinal` 是 pop 后按 `prefix_count + i` 重发的确定性位置号，不是跨重切的稳定谱系）。
//! 因此无法复核：同 seed 修订 / seed 起点右移 / 九段升级 1→N / 多旧窗到多新窗 / ordinal 复用。
//!
//! 本模块在塔构造层产出**可离线复核**的事务证书：每次重基事务记录旧、新节点的完整四边框、
//! **有序**直接源见证（`lineage_id` + 修订指纹 + 外缘 + 中枢三元组）、递归链接（下一级同 bar 事务
//! 的连续边）与变换边（`relation` + `functional/injective/unique/bijective`）——这四个布尔正是
//! D0 §6.1 过继规则的直接机检输入。
//!
//! ## 边界（票 #543 特批口径）
//!
//! - **只产构造事实**：不接触挂起、campaign、清算、订单路径；不回馈任何决策；
//! - **env 门**：`OPSEM_DUMP_DIR` 未设 ⟹ [`enabled`] 恒 false，全部产出点 no-op（生产路径逐字节
//!   不变，同 `OpsemDump::from_env` 先例）；已设 ⟹ 只在 dump 目录**新增**
//!   `rebase_transform_txn.jsonl`，既有四个产物一个字节不动；
//! - 长期 `LineageBook` / 双层身份 / 挂起原子迁移属 D1b（#466 线自理），不在本模块。
//!
//! ## 数字指纹（离线 verifier 可重算）
//!
//! 全部 digest 用 FNV-1a 64（[`fnv1a64`]）算在一条**规范化 ASCII 串**上（见各构造函数文档给出的
//! 逐字段拼接式）。选 FNV 而非 sha2 的理由：`sha2` 在本 crate 是 optional feature（仅
//! `backtest_bin`），而本 seam 必须在默认 `--lib` 构建下可用；FNV-1a 是 20 行纯算术，Python
//! verifier 可逐字节复现（`scripts` 侧实现见 #543 报告附的 verifier）。

use super::recursive_tower::{ElementId, LeveledMove, WinMeta};
use super::center_lifecycle::CenterId;
use super::super::types::{Center, Tick};

/// 证书 schema 名（写进每行 JSON；verifier 据此拒绝未知版本）。
pub const SCHEMA: &str = "rebase_transform_txn_v1";
/// 变换边判定算法版本（改判定逻辑必须同步 bump——verifier 据此发现算法漂移）。
pub const ALGORITHM_VERSION: &str = "d1a-edge-classify-v1";
/// 产物文件名（落在 `OPSEM_DUMP_DIR` 目录，与既有四产物并列，不覆盖任何既有文件）。
pub const DUMP_FILE: &str = "rebase_transform_txn.jsonl";

/// FNV-1a 64 位（离线 verifier 逐字节可复现的确定性指纹）。
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn digest_hex(s: &str) -> String {
    format!("{:016x}", fnv1a64(s.as_bytes()))
}

/// `usize::MAX` 是「+∞ 哨兵」（`read_end_src` 窗口开放 / `dirty_e` 无脏源）——JSON 记 null，
/// 不写 18446744073709551615 让 verifier 去猜。
fn opt_sentinel(v: usize) -> Option<usize> {
    (v != usize::MAX).then_some(v)
}

// ─────────────────────────────────────────────────────────────────────────────
//  证书数据结构（调研 §5.3 最小字段七组）
// ─────────────────────────────────────────────────────────────────────────────

/// 单个直接源的见证（有序——**无序集合不可复核**，调研 §5.3 明列）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    /// 源对象的构造身份（次级别 `ElementId`）。**注意**：`ordinal` 可被 pop 复用，
    /// 单靠它不能证明同 lineage——须配合 `lower_txn_id`/`lower_edge_refs` 的连续边。
    pub lineage_id: ElementId,
    /// 该源当次修订的内容指纹（见 [`SourceRef::revision_ref`] 构造式）。
    pub revision_ref: String,
    pub start: usize,
    pub end: usize,
    /// 源走势外缘 `[lo, hi]`（`RMove::lo()/hi()`，递归子树聚合）。
    pub lo: Tick,
    pub hi: Tick,
    /// 源自身若是 compose（携中枢）⟹ 其末中枢三元组 `(start_index, zd, zg)`；线段源 ⟹ None。
    pub center_id: Option<(usize, Tick, Tick)>,
    /// `seed`（窗口前三段）或 `absorbed`（延伸吸收尾段）——调研 §5.3 要求二者单列。
    pub role: &'static str,
}

impl SourceRef {
    /// 修订指纹规范串：`src|{level}#{ordinal}|{start}|{end}|{lo}|{hi}|{center}`，
    /// 其中 `{center}` = `-` 或 `{start_index}:{zd}:{zg}`。
    fn revision_canonical(id: ElementId, start: usize, end: usize, lo: Tick, hi: Tick,
                          center_id: Option<(usize, Tick, Tick)>) -> String {
        let c = match center_id {
            None => "-".to_string(),
            Some((s, zd, zg)) => format!("{s}:{zd}:{zg}"),
        };
        format!("src|{}#{}|{start}|{end}|{lo}|{hi}|{c}", id.level, id.ordinal)
    }

    fn from_move(m: &LeveledMove, role: &'static str) -> SourceRef {
        let lo = m.rmove.lo();
        let hi = m.rmove.hi();
        let center_id = last_center_of(m).map(|c| (c.start_index, c.zd, c.zg));
        SourceRef {
            lineage_id: m.id,
            revision_ref: digest_hex(&Self::revision_canonical(
                m.id, m.start_index, m.end_index, lo, hi, center_id,
            )),
            start: m.start_index,
            end: m.end_index,
            lo,
            hi,
            center_id,
            role,
        }
    }

    fn json(&self) -> String {
        let c = match self.center_id {
            None => "null".to_string(),
            Some((s, zd, zg)) => format!("{{\"start\":{s},\"zd\":{zd},\"zg\":{zg}}}"),
        };
        format!(
            "{{\"lineage_id\":{{\"level\":{},\"ordinal\":{}}},\"revision_ref\":\"{}\",\
             \"start\":{},\"end\":{},\"envelope\":{{\"lo\":{},\"hi\":{}}},\"center_id\":{c},\"role\":\"{}\"}}",
            self.lineage_id.level, self.lineage_id.ordinal, self.revision_ref,
            self.start, self.end, self.lo, self.hi, self.role,
        )
    }
}

/// 走势若为 compose ⟹ 其携带的末中枢（`RMove::Compose.centers` 末位）；线段 ⟹ None。
fn last_center_of(m: &LeveledMove) -> Option<Center> {
    match &m.rmove {
        super::descend::RMove::Segment { .. } => None,
        super::descend::RMove::Compose { centers, .. } => centers.last().copied(),
    }
}

/// 事务里的一个构造节点（旧侧 = pop/cascade 丢弃的，新侧 = 本次重扫产出的）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnNode {
    /// `old` / `new`。
    pub side: &'static str,
    /// 该节点的输出身份（`ElementId`；**仅诊断**——ordinal 会被复用，见模块头）。
    pub node_ref: ElementId,
    /// 该节点在本级输出序列中的下标（= `ElementId.ordinal`，单列以便 verifier 不必解 id）。
    pub output_ordinal: usize,
    /// 完整四边框 + source 首尾（ADR 补充十三：中枢 = 四边框）。
    pub center: Center,
    /// 节点内容指纹（见 [`TxnNode::revision_canonical`]）。
    pub revision_digest: String,
    /// 产出该节点的窗口（`win_start/win_exit/read_end_src/emitted`）。
    pub win: WinMeta,
    /// **有序**直接源见证。
    pub sources: Vec<SourceRef>,
}

impl TxnNode {
    /// 节点指纹规范串：
    /// `node|{level}#{ordinal}|{start}|{end}|{zd}|{zg}|{dd}|{gg}|{win_start}|{win_exit}|{read_end_src|-}|{emitted}|` +
    /// 各源 `revision_canonical` 以 `,` 连接。
    fn revision_canonical(node_ref: ElementId, c: &Center, win: &WinMeta, sources: &[SourceRef]) -> String {
        let read_end = match opt_sentinel(win.read_end_src) {
            None => "-".to_string(),
            Some(v) => v.to_string(),
        };
        let mut s = format!(
            "node|{}#{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|",
            node_ref.level, node_ref.ordinal, c.start_index, c.end_index, c.zd, c.zg, c.dd, c.gg,
            win.win_start, win.win_exit, read_end, win.emitted,
        );
        for (i, src) in sources.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&SourceRef::revision_canonical(
                src.lineage_id, src.start, src.end, src.lo, src.hi, src.center_id,
            ));
        }
        s
    }

    /// 从一条 `LeveledMove` + 其中枢 + 其 `WinMeta` 造节点。
    ///
    /// 源 = `sub_moves`（compose 的真窗口切片：seed 三段 + 延伸吸收段，`recursive_tower::compose_level_resume`）。
    /// 前三段标 `seed`，其余标 `absorbed`（调研 §5.3「前三 seed 与 absorbed tail 单列」）。
    pub fn from_output(side: &'static str, m: &LeveledMove, center: &Center, win: WinMeta) -> TxnNode {
        let sources: Vec<SourceRef> = m
            .sub_moves
            .iter()
            .enumerate()
            .map(|(i, s)| SourceRef::from_move(s, if i < 3 { "seed" } else { "absorbed" }))
            .collect();
        let revision_digest = digest_hex(&Self::revision_canonical(m.id, center, &win, &sources));
        TxnNode {
            side,
            node_ref: m.id,
            output_ordinal: m.id.ordinal as usize,
            center: *center,
            revision_digest,
            win,
            sources,
        }
    }

    /// 前三个源的 lineage（`(level, ordinal)` 三元组）——变换边判定的**种子键**。
    /// 不足三源（理论不达：中枢窗口 ≥3 段）⟹ 取实有源。
    fn seed_key(&self) -> Vec<(u32, u64)> {
        self.sources.iter().take(3).map(|s| (s.lineage_id.level, s.lineage_id.ordinal)).collect()
    }

    fn json(&self) -> String {
        let read_end = match opt_sentinel(self.win.read_end_src) {
            None => "null".to_string(),
            Some(v) => v.to_string(),
        };
        let srcs: Vec<String> = self.sources.iter().map(|s| s.json()).collect();
        let seed: Vec<String> = self
            .sources
            .iter()
            .filter(|s| s.role == "seed")
            .map(|s| format!("{{\"level\":{},\"ordinal\":{}}}", s.lineage_id.level, s.lineage_id.ordinal))
            .collect();
        let absorbed: Vec<String> = self
            .sources
            .iter()
            .filter(|s| s.role == "absorbed")
            .map(|s| format!("{{\"level\":{},\"ordinal\":{}}}", s.lineage_id.level, s.lineage_id.ordinal))
            .collect();
        format!(
            "{{\"side\":\"{}\",\"node_ref\":{{\"level\":{},\"ordinal\":{}}},\"output_ordinal\":{},\
             \"full_center\":{{\"start\":{},\"end\":{},\"zd\":{},\"zg\":{},\"dd\":{},\"gg\":{}}},\
             \"revision_digest\":\"{}\",\
             \"window\":{{\"win_start\":{},\"win_exit\":{},\"read_end_src\":{read_end},\"emitted\":{}}},\
             \"direct_source_refs\":[{}],\"seed_source_refs\":[{}],\"absorbed_tail_refs\":[{}]}}",
            self.side, self.node_ref.level, self.node_ref.ordinal, self.output_ordinal,
            self.center.start_index, self.center.end_index, self.center.zd, self.center.zg,
            self.center.dd, self.center.gg,
            self.revision_digest,
            self.win.win_start, self.win.win_exit, self.win.emitted,
            srcs.join(","), seed.join(","), absorbed.join(","),
        )
    }
}

/// 变换边关系（D0 §6 过继规则的机检输入；`continued_1to1` 且四布尔全 true 才允许过继）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// 旧、新一一对应且种子 lineage 逐位相等——唯一可过继形态。
    Continued1To1,
    /// 一个旧窗口产出被重切为多个新对象（#148 九段升级）。
    Split,
    /// 多个旧对象被并回一个新对象（升级窗口回退）。
    Merge,
    /// 旧对象没有种子 lineage 相等的新对象（构造窗撤出，如 seq=66 的旧窗）。
    Removed,
    /// 新对象没有种子 lineage 相等的旧对象（新构造窗建立）。
    Created,
    /// 种子键碰撞 / 多对多 / 缺源——歧义，D0 明令拒绝自动过继。
    Unknown,
}

impl Relation {
    pub fn as_str(self) -> &'static str {
        match self {
            Relation::Continued1To1 => "continued_1to1",
            Relation::Split => "split",
            Relation::Merge => "merge",
            Relation::Removed => "removed",
            Relation::Created => "created",
            Relation::Unknown => "unknown",
        }
    }
}

/// 一条变换边（旧节点 → 新节点；`removed` 无新侧、`created` 无旧侧）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformEdge {
    pub old_node_ref: Option<ElementId>,
    pub new_node_ref: Option<ElementId>,
    pub relation: Relation,
    /// 判定依据（`ordered_seed_lineage` / `window_group_split` / … ）——照实登记，不含推测。
    pub basis: &'static str,
    /// 该旧侧实体映射到的新节点数（`removed` = 0）。
    pub left_degree: usize,
    /// 该新侧实体被映射的旧节点数（`created` = 0）。
    pub right_degree: usize,
    /// 旧 → 恰好一个新。
    pub functional: bool,
    /// 新 ← 恰好一个旧。
    pub injective: bool,
    /// 种子键在本事务内无碰撞（旧侧、新侧各只有一个实体持该键）。
    pub unique: bool,
    /// `functional ∧ injective ∧ unique ∧ relation == continued_1to1`——D0 §6.1 全 true 才可过继。
    pub bijective: bool,
}

impl TransformEdge {
    fn json(&self) -> String {
        let idj = |id: &Option<ElementId>| match id {
            None => "null".to_string(),
            Some(i) => format!("{{\"level\":{},\"ordinal\":{}}}", i.level, i.ordinal),
        };
        format!(
            "{{\"old_node_ref\":{},\"new_node_ref\":{},\"relation\":\"{}\",\"basis\":\"{}\",\
             \"left_degree\":{},\"right_degree\":{},\"functional\":{},\"injective\":{},\
             \"unique\":{},\"bijective\":{}}}",
            idj(&self.old_node_ref), idj(&self.new_node_ref), self.relation.as_str(), self.basis,
            self.left_degree, self.right_degree, self.functional, self.injective,
            self.unique, self.bijective,
        )
    }
}

/// 下一级同 bar 事务的连续边引用（证明「源 X 是同 lineage 的修订」而非 ordinal 复用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowerEdgeRef {
    pub old_ordinal: u64,
    pub new_ordinal: Option<u64>,
    pub relation: Relation,
}

/// 一次完整的重基事务证书。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseTransformTxn {
    pub txn_id: u64,
    /// 原始 K 序 bar 号（= 本 bar 末 merged bar 的 `source_index`，与三流 `bar` 同坐标系）。
    pub bar: usize,
    /// 塔层下标（`level_idx`）；产出对象的 `ElementId.level` = `level + 1`。
    pub level: usize,
    /// 事务成因：`append` / `frontier_pop` / `cascade_p0` / `cascade_prefix`。
    pub cause: &'static str,
    /// cascade 的本级局部触发原因：`len_shrink` / `frontier_mutated` / `inherited`（下级传播）/ `-`。
    pub cascade_reason: &'static str,
    /// 本 bar 累积脏源下界（`usize::MAX` ⟹ null）。
    pub dirty_e: usize,
    pub resume_start: usize,
    pub prefix_count: usize,
    pub old_nodes: Vec<TxnNode>,
    pub new_nodes: Vec<TxnNode>,
    pub edges: Vec<TransformEdge>,
    pub lower_txn_id: Option<u64>,
    pub lower_edge_refs: Vec<LowerEdgeRef>,
    pub source_order_digest: String,
    pub txn_digest: String,
}

impl RebaseTransformTxn {
    fn json(&self) -> String {
        let old: Vec<String> = self.old_nodes.iter().map(|n| n.json()).collect();
        let new: Vec<String> = self.new_nodes.iter().map(|n| n.json()).collect();
        let edges: Vec<String> = self.edges.iter().map(|e| e.json()).collect();
        let lower: Vec<String> = self
            .lower_edge_refs
            .iter()
            .map(|r| {
                let n = match r.new_ordinal {
                    None => "null".to_string(),
                    Some(v) => v.to_string(),
                };
                format!(
                    "{{\"old_ordinal\":{},\"new_ordinal\":{n},\"relation\":\"{}\"}}",
                    r.old_ordinal, r.relation.as_str()
                )
            })
            .collect();
        let dirty = match opt_sentinel(self.dirty_e) {
            None => "null".to_string(),
            Some(v) => v.to_string(),
        };
        let lower_txn = match self.lower_txn_id {
            None => "null".to_string(),
            Some(v) => format!("\"txn-{v:08}\""),
        };
        format!(
            "{{\"schema\":\"{SCHEMA}\",\"txn_id\":\"txn-{:08}\",\"bar\":{},\"level\":{},\
             \"output_level\":{},\"cause\":\"{}\",\"cascade_reason\":\"{}\",\"dirty_e\":{dirty},\
             \"resume_start\":{},\"prefix_count\":{},\"old_nodes\":[{}],\"new_nodes\":[{}],\
             \"transform_edges\":[{}],\"lower_txn_id\":{lower_txn},\"lower_edge_refs\":[{}],\
             \"algorithm_version\":\"{ALGORITHM_VERSION}\",\"source_order_digest\":\"{}\",\
             \"txn_digest\":\"{}\"}}",
            self.txn_id, self.bar, self.level, self.level + 1, self.cause, self.cascade_reason,
            self.resume_start, self.prefix_count, old.join(","), new.join(","), edges.join(","),
            lower.join(","), self.source_order_digest, self.txn_digest,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  变换边判定（D0 §6.1 机检输入）
// ─────────────────────────────────────────────────────────────────────────────

/// 窗口实体：同一父窗口产出的 k 个节点（#148 九段升级共享 `win_start/win_exit/emitted`；
/// 常态窗口 k=1）。判定在**实体**层做——否则升级重切的 t≥1 子中枢会被误判成凭空 `created`。
#[derive(Debug, Clone)]
struct Entity {
    members: Vec<usize>,
    key: Vec<(u32, u64)>,
}

fn group_entities(nodes: &[TxnNode], remap: Option<&LowerMap>) -> Vec<Entity> {
    let mut out: Vec<Entity> = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        let same_window = out.last().is_some_and(|e| {
            let head = &nodes[e.members[0]];
            head.win.emitted > 1
                && head.win.win_start == n.win.win_start
                && head.win.win_exit == n.win.win_exit
                && head.win.emitted == n.win.emitted
                && e.members.len() < head.win.emitted
        });
        if same_window {
            out.last_mut().expect("same_window 蕴含非空").members.push(i);
        } else {
            let key = match remap {
                None => n.seed_key(),
                Some(map) => map.apply(&n.seed_key()),
            };
            out.push(Entity { members: vec![i], key });
        }
    }
    out
}

/// 下一级同 bar 事务给出的 lineage 重映射：旧 ordinal → 新 ordinal（`continued_1to1` 边）；
/// `removed` 的旧 ordinal 映射为「已死」（键位打洞，永不与新侧相等）。
#[derive(Debug, Clone, Default)]
pub struct LowerMap {
    /// 下级输出对象的级别（= 本级源的 `ElementId.level`）。
    pub level: u32,
    /// `continued_1to1` 的 old→new ordinal。
    pub continued: Vec<(u64, u64)>,
    /// 非连续的 old ordinal 及其**真实**下级关系（removed/split/merge/unknown）——照实登记，
    /// 让离线 verifier 能区分「真删除」与「九段分裂」，并算宽读法反事实。
    pub broken: Vec<(u64, Relation)>,
}

impl LowerMap {
    fn apply(&self, key: &[(u32, u64)]) -> Vec<(u32, u64)> {
        key.iter()
            .map(|&(lv, ord)| {
                if lv != self.level {
                    return (lv, ord);
                }
                if let Some(&(_, n)) = self.continued.iter().find(|&&(o, _)| o == ord) {
                    return (lv, n);
                }
                if self.broken.iter().any(|&(o, _)| o == ord) {
                    // 已死 lineage：抬到不可能与新侧相等的键位（u64::MAX 减去 ordinal，
                    // 保证不同 broken 源互不碰撞，也不与任何真 ordinal 相等）。
                    return (lv, u64::MAX - ord);
                }
                (lv, ord)
            })
            .collect()
    }
}

/// 按**有序种子 lineage**（经下级连续边重映射后）判定旧、新节点间的变换边。
///
/// 判定规则（照实，不外推）：
/// 1. 先按父窗口把节点折成实体（升级重切的 k 个子中枢 = 一个实体，键取其头节点的种子三元组）；
/// 2. 实体按种子键配对。键在同侧出现多次 ⟹ 该键涉及的全部边判 `unknown`（碰撞，D0 拒绝）；
/// 3. 配对成功：`1↔1` ⟹ `continued_1to1`；`1↔k` ⟹ `split`；`k↔1` ⟹ `merge`；
///    `a↔b`（a,b>1）⟹ 逐位比较全序源 lineage，全等则逐位 `continued_1to1`，否则 `unknown`；
/// 4. 旧侧未配对 ⟹ `removed`（新侧 null）；新侧未配对 ⟹ `created`（旧侧 null）。
pub fn classify_edges(
    old_nodes: &[TxnNode],
    new_nodes: &[TxnNode],
    lower: Option<&LowerMap>,
) -> Vec<TransformEdge> {
    let olds = group_entities(old_nodes, lower);
    let news = group_entities(new_nodes, None);
    let mut edges: Vec<TransformEdge> = Vec::new();

    let key_dup = |es: &[Entity], k: &Vec<(u32, u64)>| es.iter().filter(|e| &e.key == k).count() > 1;

    let mut new_matched: Vec<bool> = vec![false; news.len()];
    for oe in &olds {
        let cands: Vec<usize> = news
            .iter()
            .enumerate()
            .filter(|(_, ne)| ne.key == oe.key)
            .map(|(i, _)| i)
            .collect();
        if cands.is_empty() {
            for &m in &oe.members {
                edges.push(TransformEdge {
                    old_node_ref: Some(old_nodes[m].node_ref),
                    new_node_ref: None,
                    relation: Relation::Removed,
                    basis: "no_new_entity_with_same_seed_lineage",
                    left_degree: 0,
                    right_degree: 0,
                    functional: false,
                    injective: false,
                    unique: !key_dup(&olds, &oe.key),
                    bijective: false,
                });
            }
            continue;
        }
        let collision = cands.len() > 1 || key_dup(&olds, &oe.key);
        for &ci in &cands {
            new_matched[ci] = true;
        }
        if collision {
            // 键碰撞（同一种子键有多个实体）——歧义，逐对记 unknown，四布尔照实为假。
            for &ci in &cands {
                for &om in &oe.members {
                    for &nm in &news[ci].members {
                        edges.push(TransformEdge {
                            old_node_ref: Some(old_nodes[om].node_ref),
                            new_node_ref: Some(new_nodes[nm].node_ref),
                            relation: Relation::Unknown,
                            basis: "seed_lineage_key_collision",
                            left_degree: cands.iter().map(|&c| news[c].members.len()).sum(),
                            right_degree: olds.iter().filter(|e| e.key == oe.key)
                                .map(|e| e.members.len()).sum(),
                            functional: false,
                            injective: false,
                            unique: false,
                            bijective: false,
                        });
                    }
                }
            }
            continue;
        }
        let ne = &news[cands[0]];
        let (a, b) = (oe.members.len(), ne.members.len());
        match (a, b) {
            (1, 1) => edges.push(TransformEdge {
                old_node_ref: Some(old_nodes[oe.members[0]].node_ref),
                new_node_ref: Some(new_nodes[ne.members[0]].node_ref),
                relation: Relation::Continued1To1,
                basis: "ordered_seed_lineage",
                left_degree: 1,
                right_degree: 1,
                functional: true,
                injective: true,
                unique: true,
                bijective: true,
            }),
            (1, _) => {
                for &nm in &ne.members {
                    edges.push(TransformEdge {
                        old_node_ref: Some(old_nodes[oe.members[0]].node_ref),
                        new_node_ref: Some(new_nodes[nm].node_ref),
                        relation: Relation::Split,
                        basis: "window_group_split",
                        left_degree: b,
                        right_degree: 1,
                        functional: false,
                        injective: true,
                        unique: true,
                        bijective: false,
                    });
                }
            }
            (_, 1) => {
                for &om in &oe.members {
                    edges.push(TransformEdge {
                        old_node_ref: Some(old_nodes[om].node_ref),
                        new_node_ref: Some(new_nodes[ne.members[0]].node_ref),
                        relation: Relation::Merge,
                        basis: "window_group_merge",
                        left_degree: 1,
                        right_degree: a,
                        functional: true,
                        injective: false,
                        unique: true,
                        bijective: false,
                    });
                }
            }
            _ => {
                // a,b > 1（旧、新都是同父窗口的升级重切组）：**逐位**配对，每对各自按种子三元组
                // （与单体同一判据，不因同组邻居的延伸吸收而连坐）判连续；配不上的按 removed/created 记。
                let m = a.min(b);
                for i in 0..m {
                    let (om, nm) = (oe.members[i], ne.members[i]);
                    let ok = match lower {
                        None => old_nodes[om].seed_key(),
                        Some(map) => map.apply(&old_nodes[om].seed_key()),
                    };
                    let same = ok == new_nodes[nm].seed_key();
                    edges.push(TransformEdge {
                        old_node_ref: Some(old_nodes[om].node_ref),
                        new_node_ref: Some(new_nodes[nm].node_ref),
                        relation: if same { Relation::Continued1To1 } else { Relation::Unknown },
                        basis: if same { "positional_seed_lineage" } else { "positional_seed_mismatch" },
                        left_degree: 1,
                        right_degree: 1,
                        functional: same,
                        injective: same,
                        unique: same,
                        bijective: same,
                    });
                }
                for &om in &oe.members[m..] {
                    edges.push(TransformEdge {
                        old_node_ref: Some(old_nodes[om].node_ref),
                        new_node_ref: None,
                        relation: Relation::Removed,
                        basis: "window_group_shrunk",
                        left_degree: 0,
                        right_degree: 0,
                        functional: false,
                        injective: false,
                        unique: false,
                        bijective: false,
                    });
                }
                for &nm in &ne.members[m..] {
                    edges.push(TransformEdge {
                        old_node_ref: None,
                        new_node_ref: Some(new_nodes[nm].node_ref),
                        relation: Relation::Created,
                        basis: "window_group_grew",
                        left_degree: 0,
                        right_degree: 0,
                        functional: false,
                        injective: false,
                        unique: false,
                        bijective: false,
                    });
                }
            }
        }
    }
    for (i, ne) in news.iter().enumerate() {
        if new_matched[i] {
            continue;
        }
        for &m in &ne.members {
            edges.push(TransformEdge {
                old_node_ref: None,
                new_node_ref: Some(new_nodes[m].node_ref),
                relation: Relation::Created,
                basis: "no_old_entity_with_same_seed_lineage",
                left_degree: 0,
                right_degree: 0,
                functional: false,
                injective: false,
                unique: !key_dup(&news, &ne.key),
                bijective: false,
            });
        }
    }
    edges
}

/// ★#679 D1b：从变换边抽出**可过继**的 `旧中枢身份 → 新中枢身份` 对。
///
/// 门槛与 D0 §6.1 逐字一致：`relation == continued_1to1` **且**
/// `functional ∧ injective ∧ unique ∧ bijective` 全真。任何一项不满足即不产对
/// （split / merge / removed / created / unknown 全部在此被挡下）。
///
/// 身份取节点的完整四边框投影 `CenterId::of`（`(start_index, zd, zg)`）——这正是挂起簿的键
/// 与生命周期链的对账口径，不另立第二套身份。
pub fn continued_center_pairs(
    old_nodes: &[TxnNode],
    new_nodes: &[TxnNode],
    edges: &[TransformEdge],
) -> Vec<(CenterId, CenterId)> {
    let mut out = Vec::new();
    for e in edges {
        if e.relation != Relation::Continued1To1
            || !(e.functional && e.injective && e.unique && e.bijective)
        {
            continue;
        }
        let (Some(o), Some(n)) = (e.old_node_ref, e.new_node_ref) else { continue };
        let (Some(on), Some(nn)) = (
            old_nodes.iter().find(|x| x.node_ref == o),
            new_nodes.iter().find(|x| x.node_ref == n),
        ) else {
            continue;
        };
        out.push((CenterId::of(&on.center), CenterId::of(&nn.center)));
    }
    out
}

/// 从本事务的连续边导出供**上一级**使用的 lineage 重映射。
pub fn lower_map_of(level_of_outputs: u32, edges: &[TransformEdge]) -> LowerMap {
    let mut map = LowerMap { level: level_of_outputs, continued: Vec::new(), broken: Vec::new() };
    for e in edges {
        match (e.old_node_ref, e.new_node_ref, e.relation) {
            (Some(o), Some(n), Relation::Continued1To1) => map.continued.push((o.ordinal, n.ordinal)),
            (Some(o), _, rel) => map.broken.push((o.ordinal, rel)),
            _ => {}
        }
    }
    map
}

// ─────────────────────────────────────────────────────────────────────────────
//  事务装配与落盘
// ─────────────────────────────────────────────────────────────────────────────

/// 事务上下文（放置点传入的事务头字段）。
#[derive(Debug, Clone, Copy)]
pub struct TxnContext {
    pub bar: usize,
    pub level: usize,
    pub cause: &'static str,
    pub cascade_reason: &'static str,
    pub dirty_e: usize,
    pub resume_start: usize,
    pub prefix_count: usize,
}

/// 装配一条事务证书（纯函数——单测直接调它，与生产同一条路径）。
pub fn build_txn(
    txn_id: u64,
    ctx: TxnContext,
    old_nodes: Vec<TxnNode>,
    new_nodes: Vec<TxnNode>,
    lower: Option<(u64, &LowerMap)>,
) -> RebaseTransformTxn {
    let lower_map = lower.map(|(_, m)| m);
    let edges = classify_edges(&old_nodes, &new_nodes, lower_map);
    // 源序指纹：旧、新全部节点的**有序**源 lineage 串接（源顺序变化即指纹变化）。
    let mut order = String::from("order|");
    for n in old_nodes.iter().chain(new_nodes.iter()) {
        order.push_str(n.side);
        order.push('|');
        for s in &n.sources {
            order.push_str(&format!("{}#{};", s.lineage_id.level, s.lineage_id.ordinal));
        }
        order.push('|');
    }
    let source_order_digest = digest_hex(&order);

    // 递归链接：只登记与本事务源实际相关的下级边（源 ordinal 命中即收）。
    let mut lower_edge_refs: Vec<LowerEdgeRef> = Vec::new();
    if let Some(map) = lower_map {
        let mut want: Vec<u64> = old_nodes
            .iter()
            .flat_map(|n| n.sources.iter())
            .filter(|s| s.lineage_id.level == map.level)
            .map(|s| s.lineage_id.ordinal)
            .collect();
        want.sort_unstable();
        want.dedup();
        for o in want {
            if let Some(&(_, n)) = map.continued.iter().find(|&&(oo, _)| oo == o) {
                lower_edge_refs.push(LowerEdgeRef {
                    old_ordinal: o,
                    new_ordinal: Some(n),
                    relation: Relation::Continued1To1,
                });
            } else if let Some(&(_, rel)) = map.broken.iter().find(|&&(oo, _)| oo == o) {
                lower_edge_refs.push(LowerEdgeRef { old_ordinal: o, new_ordinal: None, relation: rel });
            }
        }
    }

    let mut txn = RebaseTransformTxn {
        txn_id,
        bar: ctx.bar,
        level: ctx.level,
        cause: ctx.cause,
        cascade_reason: ctx.cascade_reason,
        dirty_e: ctx.dirty_e,
        resume_start: ctx.resume_start,
        prefix_count: ctx.prefix_count,
        old_nodes,
        new_nodes,
        edges,
        lower_txn_id: lower.map(|(id, _)| id),
        lower_edge_refs,
        source_order_digest,
        txn_digest: String::new(),
    };
    // 事务指纹：头字段 + 各节点 revision_digest + 各边 + 源序指纹（不含 txn_digest 自身）。
    let mut canon = format!(
        "txn|{}|{}|{}|{}|{}|{}|{}|{}|",
        txn.txn_id, txn.bar, txn.level, txn.cause, txn.cascade_reason,
        match opt_sentinel(txn.dirty_e) { None => "-".to_string(), Some(v) => v.to_string() },
        txn.resume_start, txn.prefix_count,
    );
    for n in txn.old_nodes.iter().chain(txn.new_nodes.iter()) {
        canon.push_str(&format!("{}:{};", n.side, n.revision_digest));
    }
    canon.push('|');
    for e in &txn.edges {
        canon.push_str(&format!(
            "{}->{}:{}:{}{}{}{};",
            e.old_node_ref.map_or("-".to_string(), |i| format!("{}#{}", i.level, i.ordinal)),
            e.new_node_ref.map_or("-".to_string(), |i| format!("{}#{}", i.level, i.ordinal)),
            e.relation.as_str(),
            e.functional as u8, e.injective as u8, e.unique as u8, e.bijective as u8,
        ));
    }
    canon.push('|');
    canon.push_str(&txn.source_order_digest);
    txn.txn_digest = digest_hex(&canon);
    txn
}

// ── 落盘 sink（env 门；未启用 ⟹ 全链 no-op） ────────────────────────────────

use std::sync::{Mutex, OnceLock};

fn sink() -> Option<&'static Mutex<std::fs::File>> {
    static SINK: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    SINK.get_or_init(|| {
        let dir = std::env::var("OPSEM_DUMP_DIR").ok().filter(|s| !s.is_empty())?;
        let path = std::path::Path::new(&dir);
        std::fs::create_dir_all(path).ok()?;
        // 截断创建（每次回测重写，同 OpsemDump 落盘语义）。**只新增本文件**，
        // 既有 trades/tower_events/center_lifecycle/rebase_observability 不触碰。
        let file = std::fs::File::create(path.join(DUMP_FILE)).ok()?;
        Some(Mutex::new(file))
    })
    .as_ref()
}

#[cfg(test)]
thread_local! {
    /// 单测捕获缓冲：`Some(_)` ⟹ [`enabled`] 为真且事务写入本缓冲（不落盘、不受并行测试干扰）。
    static CAPTURE: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// 单测：开启捕获（清空既有缓冲）。
#[cfg(test)]
pub fn test_capture_start() {
    CAPTURE.with(|c| *c.borrow_mut() = Some(Vec::new()));
}

/// 单测：取回并关闭捕获。
#[cfg(test)]
pub fn test_capture_take() -> Vec<String> {
    CAPTURE.with(|c| c.borrow_mut().take()).unwrap_or_default()
}

/// 证书产出是否启用。**未启用 ⟹ 放置点连快照都不做**（零开销，行为逐字节不变）。
///
/// ★#679 D1b：判据从「只看 `OPSEM_DUMP_DIR`」扩为「谱系簿要建 **或** 要落盘」——票面范围第 1 条
/// 「簿的读写与 `OPSEM_DUMP_DIR` 环境门解耦，生产判径可用」。默认
/// [`consumer_enabled`](super::super::lineage_book::consumer_enabled) 为真 ⟹ 生产路径上 seam 常开；
/// `THETA_REBASE_MIGRATE_SKIP=1` 且未设 dump 目录 ⟹ 回到 D1a 前的全链 no-op。
pub fn enabled() -> bool {
    #[cfg(test)]
    {
        if CAPTURE.with(|c| c.borrow().is_some()) {
            return true;
        }
    }
    super::super::lineage_book::consumer_enabled() || sink().is_some()
}

/// 全局单调事务号（跨 bar/level，首条 = 1）。
fn next_txn_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}

/// 装配 + 落盘一条事务证书，返回（事务号，供上一级用的 lineage 重映射）。
///
/// 调用方须先判 [`enabled`]。返回值只用于**同 bar 上一级**的递归链接，不回馈任何生产状态。
pub fn emit(
    ctx: TxnContext,
    old_nodes: Vec<TxnNode>,
    new_nodes: Vec<TxnNode>,
    lower: Option<(u64, &LowerMap)>,
) -> (u64, LowerMap) {
    let id = next_txn_id();
    let txn = build_txn(id, ctx, old_nodes, new_nodes, lower);
    let map = lower_map_of(ctx.level as u32 + 1, &txn.edges);

    // ★#679 D1b：谱系簿消费**同一产出点**（与 `OPSEM_DUMP_DIR` 解耦——见 [`enabled`]）。
    // 只登记 `continued_1to1 ∧ 四布尔全真` 的边；宽读法边只在开关打开时另算一遍
    // （`classify_edges(.., None)` = 不追下级 lineage，即 D0 探针口径，见 lineage_book 模块头）。
    if super::super::lineage_book::consumer_enabled() {
        // ★#679 用户 2026-07-29 裁定：宽读法转生产默认（`wide_reading()` 恒真，除非
        // `THETA_REBASE_MIGRATE_STRICT=1`）。`strict` 边**全量入簿但生产默认永不被
        // `lookup` 取用**——`lineage_book::lookup` 只读 `wide_reading()` 选中的那一半簿
        // （见 `LineageBook::lookup`）。留档对照语义：严格读法作反事实基线常驻计算，
        // 需要复核 D1a §5.4 两读法差异或切回 `_STRICT=1` 时无需另跑一遍即有数可查。
        // 未加 lookup 门控（不按 wide_reading() 跳过 strict 计算）是有意选择——这段计算
        // 相对 wide_edges 的 `classify_edges` 重算是轻量增量（同一 `txn.edges` 上过滤），
        // 常开换取「随时可回读严格口径」不值得为省这点算力另开一条门控分支。
        let strict = continued_center_pairs(&txn.old_nodes, &txn.new_nodes, &txn.edges);
        let wide = if super::super::lineage_book::wide_reading() {
            let wide_edges = classify_edges(&txn.old_nodes, &txn.new_nodes, None);
            continued_center_pairs(&txn.old_nodes, &txn.new_nodes, &wide_edges)
        } else {
            Vec::new()
        };
        super::super::lineage_book::record_edges(
            txn.bar,
            txn.level,
            txn.txn_id,
            &txn.txn_digest,
            &strict,
            &wide,
        );
    }

    // JSON 序列化只在真要落盘/捕获时做——它是本 seam 的开销大头（wf8 实测 38.5 MB / 4172 行，
    // D1a §3.4）。D1b 让 seam 在生产判径常开，若无条件序列化就等于给生产路径挂上 38 MB 的
    // 字符串构造，故此处按需。
    let need_line = {
        #[cfg(test)]
        {
            CAPTURE.with(|c| c.borrow().is_some()) || sink().is_some()
        }
        #[cfg(not(test))]
        {
            sink().is_some()
        }
    };
    if !need_line {
        return (id, map);
    }
    let mut line = txn.json();
    line.push('\n');
    #[cfg(test)]
    {
        let captured = CAPTURE.with(|c| {
            let mut b = c.borrow_mut();
            match b.as_mut() {
                None => false,
                Some(v) => {
                    v.push(line.clone());
                    true
                }
            }
        });
        if captured {
            return (id, map);
        }
    }
    if let Some(m) = sink() {
        if let Ok(mut f) = m.lock() {
            use std::io::Write;
            // 观测旁路：写失败静默跳过（诊断产物不得影响交易决策，同 D0 fail-open 口径）。
            let _ = f.write_all(line.as_bytes());
        }
    }
    (id, map)
}

/// 从「输出对象 + 中枢 + 窗口」三元组批量造节点（放置点共用）。
pub fn snapshot_nodes(
    side: &'static str,
    moves: &[LeveledMove],
    centers: &[Center],
    metas: &[WinMeta],
) -> Vec<TxnNode> {
    debug_assert!(
        moves.len() == centers.len() && moves.len() == metas.len(),
        "构造证书快照：upper_moves/centers/win_meta 须 1:1 对齐"
    );
    moves
        .iter()
        .zip(centers.iter())
        .zip(metas.iter())
        .map(|((m, c), w)| TxnNode::from_output(side, m, c, *w))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::classifier::center::UnitRange;
    use crate::theta_v0::types::Direction;
    use std::rc::Rc;

    fn unit(start: usize, end: usize, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: start, end_index: end, lo, hi, direction: Direction::Up }
    }

    fn seg_move(ordinal: u64, start: usize, end: usize, lo: Tick, hi: Tick) -> LeveledMove {
        LeveledMove::from_unit(&unit(start, end, lo, hi), ElementId { level: 0, ordinal })
    }

    /// 造一条 compose 输出：给定源走势序列 + 中枢 + 输出 ordinal。
    fn composed(ordinal: u64, subs: &[LeveledMove], center: Center) -> LeveledMove {
        LeveledMove::compose(subs, center, 1, ElementId { level: 1, ordinal })
    }

    fn center(start: usize, end: usize, zd: Tick, zg: Tick, dd: Tick, gg: Tick) -> Center {
        Center { start_index: start, end_index: end, zd, zg, dd, gg }
    }

    fn win(win_start: usize, win_exit: usize, read_end: usize, emitted: usize) -> WinMeta {
        WinMeta { win_start, win_exit, read_end_src: read_end, emitted }
    }

    fn ctx() -> TxnContext {
        TxnContext {
            bar: 240310,
            level: 0,
            cause: "frontier_pop",
            cascade_reason: "-",
            dirty_e: usize::MAX,
            resume_start: 3,
            prefix_count: 5,
        }
    }

    /// 放置点①②③（常态 frontier pop → 重扫 tail）：同 seed、第三源外缘修订 ⟹ `continued_1to1`
    /// 且四布尔全 true（D0 §6.1 可过继形态）。对应 D0 八条中的 seq 3/20/47/53/56。
    #[test]
    fn frontier_pop_same_seed_third_source_revision_yields_continued_1to1() {
        let s0 = seg_move(12, 100, 110, 1000, 1100);
        let s1 = seg_move(13, 110, 120, 1050, 1150);
        let old_s2 = seg_move(14, 120, 130, 1080, 1180);
        // 重扫后第三源外缘/终点修订（同 lineage id，内容变）。
        let new_s2 = seg_move(14, 120, 136, 1060, 1200);

        let old = composed(5, &[s0.clone(), s1.clone(), old_s2], center(100, 130, 1080, 1100, 1000, 1180));
        let new = composed(5, &[s0, s1, new_s2], center(100, 136, 1060, 1100, 1000, 1200));

        let old_nodes = snapshot_nodes("old", &[old], &[center(100, 130, 1080, 1100, 1000, 1180)], &[win(3, 6, 140, 1)]);
        let new_nodes = snapshot_nodes("new", &[new], &[center(100, 136, 1060, 1100, 1000, 1200)], &[win(3, 6, 146, 1)]);

        let txn = build_txn(1, ctx(), old_nodes, new_nodes, None);
        assert_eq!(txn.edges.len(), 1);
        let e = &txn.edges[0];
        assert_eq!(e.relation, Relation::Continued1To1);
        assert!(e.functional && e.injective && e.unique && e.bijective);
        assert_eq!((e.left_degree, e.right_degree), (1, 1));
        // 内容确实变了 ⟹ 两侧 revision_digest 必不同（否则证书没有区分力）。
        assert_ne!(txn.old_nodes[0].revision_digest, txn.new_nodes[0].revision_digest);
        // 源序指纹相同（有序 lineage 未变），事务指纹非空。
        assert!(!txn.txn_digest.is_empty());
    }

    /// 放置点④（cascade 后缀失效）：seed 起点右移一位 ⟹ 旧窗 `removed` + 新窗 `created`，
    /// **不得**被判 `continued`。对应 D0 的 seq=66（`#494,#495,#496 → #495,#496,#497,#498`）。
    #[test]
    fn cascade_seed_shift_yields_removed_and_created_not_continued() {
        let s494 = seg_move(494, 238178, 238400, 4182110000000, 4317400000000);
        let s495 = seg_move(495, 238400, 238612, 4291997000000, 4378735000000);
        let s496 = seg_move(496, 238612, 239000, 4311413000000, 4388236000000);
        let s496b = seg_move(496, 238612, 239000, 4321576000000, 4347000000000);
        let s497 = seg_move(497, 239000, 239400, 4311413000000, 4388236000000);
        let s498 = seg_move(498, 239400, 239900, 4320000000000, 4390000000000);

        let old = composed(121, &[s494, s495.clone(), s496],
            center(238178, 239000, 4311413000000, 4317400000000, 4182110000000, 4388236000000));
        let new = composed(121, &[s495, s496b, s497, s498],
            center(238612, 239900, 4321576000000, 4347000000000, 4291997000000, 4390000000000));

        let old_nodes = snapshot_nodes("old", &[old],
            &[center(238178, 239000, 4311413000000, 4317400000000, 4182110000000, 4388236000000)],
            &[win(10, 13, 239000, 1)]);
        let new_nodes = snapshot_nodes("new", &[new],
            &[center(238612, 239900, 4321576000000, 4347000000000, 4291997000000, 4390000000000)],
            &[win(11, 15, usize::MAX, 1)]);

        let c = TxnContext { cause: "cascade_prefix", cascade_reason: "frontier_mutated", dirty_e: 238612, ..ctx() };
        let txn = build_txn(2, c, old_nodes, new_nodes, None);
        assert_eq!(txn.edges.len(), 2);
        assert_eq!(txn.edges[0].relation, Relation::Removed);
        assert!(txn.edges[0].new_node_ref.is_none());
        assert!(!txn.edges[0].bijective);
        assert_eq!(txn.edges[1].relation, Relation::Created);
        assert!(txn.edges[1].old_node_ref.is_none());
        // ordinal 相同（L1#121 位置号复用）却判非连续——正是本 seam 要证的假证排除。
        assert_eq!(txn.old_nodes[0].node_ref.ordinal, txn.new_nodes[0].node_ref.ordinal);
    }

    /// 放置点⑤（九段一窗产 k 子对象，`recursive_tower.rs:756-790`）：一个旧窗口 → k 个共享父窗口的
    /// 新子中枢 ⟹ `split`（`functional=false`、`bijective=false`，D0 明令拒绝自动过继）。
    #[test]
    fn upgrade_window_emitting_k_children_yields_split_not_created() {
        let subs: Vec<LeveledMove> = (0..9)
            .map(|i| seg_move(200 + i as u64, 1000 + i * 10, 1010 + i * 10, 500 + i as Tick, 600 + i as Tick))
            .collect();
        let old_center = center(1000, 1100, 505, 595, 500, 608);
        let old = composed(7, &subs, old_center);
        let old_nodes = snapshot_nodes("old", &[old], &[old_center], &[win(4, 13, 1200, 1)]);

        // 升级重切：同一父窗口 (4,13) 产 k=3 个子中枢，emitted=3 逐值相同。
        let mut new_moves = Vec::new();
        let mut new_centers = Vec::new();
        let mut new_metas = Vec::new();
        for t in 0..3u64 {
            let s = (t * 3) as usize;
            let sub = &subs[s..s + 3];
            let c = center(1000 + s * 10, 1030 + s * 10, 505, 595, 500 + s as Tick, 600 + s as Tick);
            new_moves.push(composed(7 + t, sub, c));
            new_centers.push(c);
            new_metas.push(win(4, 13, 1200, 3));
        }
        let new_nodes = snapshot_nodes("new", &new_moves, &new_centers, &new_metas);

        let txn = build_txn(3, ctx(), old_nodes, new_nodes, None);
        assert_eq!(txn.edges.len(), 3, "1→3 应产 3 条边，不是 1 条连续 + 2 条 created");
        for e in &txn.edges {
            assert_eq!(e.relation, Relation::Split);
            assert_eq!(e.left_degree, 3);
            assert!(!e.functional && !e.bijective);
        }
    }

    /// 递归链接：下级 `continued_1to1` 把源 ordinal 重映射后，本级仍判连续；
    /// 下级判 `removed` 的源 ⟹ 本级不得靠 ordinal 复用伪造连续。
    #[test]
    fn lower_txn_remap_distinguishes_lineage_from_ordinal_reuse() {
        let old_subs = [seg_move(1, 0, 10, 10, 20), seg_move(2, 10, 20, 12, 22), seg_move(3, 20, 30, 14, 24)];
        let new_subs = [seg_move(1, 0, 10, 10, 20), seg_move(2, 10, 20, 12, 22), seg_move(3, 20, 32, 14, 26)];
        let oc = center(0, 30, 14, 20, 10, 24);
        let nc = center(0, 32, 14, 20, 10, 26);
        let old_nodes = snapshot_nodes("old", &[composed(0, &old_subs, oc)], &[oc], &[win(0, 3, 40, 1)]);
        let new_nodes = snapshot_nodes("new", &[composed(0, &new_subs, nc)], &[nc], &[win(0, 3, 42, 1)]);

        // (a) 下级把 1,2,3 判为连续 ⟹ 本级连续。
        let ok = LowerMap { level: 0, continued: vec![(1, 1), (2, 2), (3, 3)], broken: vec![] };
        let t1 = build_txn(4, ctx(), old_nodes.clone(), new_nodes.clone(), Some((9, &ok)));
        assert_eq!(t1.edges[0].relation, Relation::Continued1To1);
        assert_eq!(t1.lower_txn_id, Some(9));
        assert_eq!(t1.lower_edge_refs.len(), 3);

        // (b) 下级把源 #2 判为 removed（ordinal 被新 lineage 复用）⟹ 本级必须判非连续。
        let broken = LowerMap {
            level: 0,
            continued: vec![(1, 1), (3, 3)],
            broken: vec![(2, Relation::Removed)],
        };
        let t2 = build_txn(5, ctx(), old_nodes, new_nodes, Some((9, &broken)));
        assert_eq!(t2.edges[0].relation, Relation::Removed);
        assert_eq!(t2.edges[1].relation, Relation::Created);
    }

    /// 指纹可复算性：同输入 ⟹ 同 digest；任一源顺序变化 ⟹ `source_order_digest` 变。
    #[test]
    fn digests_are_deterministic_and_order_sensitive() {
        let a = seg_move(1, 0, 10, 10, 20);
        let b = seg_move(2, 10, 20, 12, 22);
        let c3 = seg_move(3, 20, 30, 14, 24);
        let cc = center(0, 30, 14, 20, 10, 24);
        let n1 = snapshot_nodes("new", &[composed(0, &[a.clone(), b.clone(), c3.clone()], cc)], &[cc], &[win(0, 3, 40, 1)]);
        let n2 = snapshot_nodes("new", &[composed(0, &[b, a, c3], cc)], &[cc], &[win(0, 3, 40, 1)]);
        let t1 = build_txn(6, ctx(), Vec::new(), n1.clone(), None);
        let t1b = build_txn(6, ctx(), Vec::new(), n1, None);
        let t2 = build_txn(6, ctx(), Vec::new(), n2, None);
        assert_eq!(t1.txn_digest, t1b.txn_digest, "同输入须同指纹");
        assert_ne!(t1.source_order_digest, t2.source_order_digest, "源序变化须改指纹");
        assert_eq!(fnv1a64(b"abc"), 0xe71fa2190541574b, "FNV-1a 64 已知向量（verifier 侧同式）");
    }

    /// JSON 行自洽：七组字段全在，且 `usize::MAX` 哨兵记 null 而非 1.8e19。
    #[test]
    fn json_line_carries_all_seven_field_groups() {
        let subs = [seg_move(1, 0, 10, 10, 20), seg_move(2, 10, 20, 12, 22), seg_move(3, 20, 30, 14, 24)];
        let cc = center(0, 30, 14, 20, 10, 24);
        let nodes = snapshot_nodes("new", &[composed(0, &subs, cc)], &[cc], &[win(0, 3, usize::MAX, 1)]);
        let txn = build_txn(7, ctx(), Vec::new(), nodes, None);
        let line = txn.json();
        for k in [
            "\"schema\":\"rebase_transform_txn_v1\"", "\"txn_id\"", "\"bar\"", "\"level\"", "\"cause\"",
            "\"dirty_e\":null", "\"resume_start\"", "\"prefix_count\"", "\"old_nodes\"", "\"new_nodes\"",
            "\"full_center\"", "\"revision_digest\"", "\"window\"", "\"read_end_src\":null",
            "\"direct_source_refs\"", "\"seed_source_refs\"", "\"absorbed_tail_refs\"",
            "\"transform_edges\"", "\"lower_txn_id\":null", "\"lower_edge_refs\"",
            "\"algorithm_version\"", "\"source_order_digest\"", "\"txn_digest\"",
        ] {
            assert!(line.contains(k), "缺字段 {k}：{line}");
        }
        assert!(!line.contains("18446744073709551615"), "哨兵必须记 null");
        // 单行 JSONL：不得含裸换行。
        assert!(!line.contains('\n'));
    }

    /// ★#679 D1b：[`continued_center_pairs`] 只收 `continued_1to1 ∧ 四布尔全真` 的边——
    /// `split` / `removed` / `created` / `unknown` 一律不产谱系对（D0 §6.1 门槛的实现层锁）。
    #[test]
    fn continued_center_pairs_only_admits_fully_bijective_continued_edges() {
        // ① 同 seed、第三源修订 ⟹ 一条可过继的连续边。
        let s0 = seg_move(12, 100, 110, 1000, 1100);
        let s1 = seg_move(13, 110, 120, 1050, 1150);
        let old_c = center(100, 130, 1080, 1100, 1000, 1180);
        let new_c = center(100, 136, 1060, 1100, 1000, 1200);
        let old = composed(5, &[s0.clone(), s1.clone(), seg_move(14, 120, 130, 1080, 1180)], old_c);
        let new = composed(5, &[s0, s1, seg_move(14, 120, 136, 1060, 1200)], new_c);
        let old_nodes = snapshot_nodes("old", &[old], &[old_c], &[win(3, 6, 140, 1)]);
        let new_nodes = snapshot_nodes("new", &[new], &[new_c], &[win(3, 6, 146, 1)]);
        let txn = build_txn(1, ctx(), old_nodes, new_nodes, None);
        assert_eq!(
            continued_center_pairs(&txn.old_nodes, &txn.new_nodes, &txn.edges),
            vec![(CenterId::of(&old_c), CenterId::of(&new_c))]
        );

        // ② 同一批节点，人为把边降级为非连续/非全真 ⟹ 一对都不产。
        for degraded in [
            TransformEdge { relation: Relation::Split, ..txn.edges[0].clone() },
            TransformEdge { relation: Relation::Unknown, ..txn.edges[0].clone() },
            TransformEdge { bijective: false, ..txn.edges[0].clone() },
            TransformEdge { unique: false, ..txn.edges[0].clone() },
            TransformEdge { injective: false, ..txn.edges[0].clone() },
            TransformEdge { functional: false, ..txn.edges[0].clone() },
            TransformEdge { new_node_ref: None, relation: Relation::Removed, ..txn.edges[0].clone() },
        ] {
            assert!(
                continued_center_pairs(&txn.old_nodes, &txn.new_nodes, &[degraded.clone()]).is_empty(),
                "非 `continued_1to1 ∧ 四布尔全真` 的边不得产谱系对：{degraded:?}"
            );
        }
    }

    /// 反证臂负控：consumer 显式关（`THETA_REBASE_MIGRATE_SKIP=1` 语义）且未开落盘捕获
    /// ⟹ `enabled()` 假——D1a 前旧行为的可复现锚点，防「默认开」把这条路堵死。
    #[test]
    fn disabled_when_consumer_off_without_env_or_capture() {
        assert!(std::env::var("OPSEM_DUMP_DIR").is_err(), "本测试要求进程未设 OPSEM_DUMP_DIR");
        crate::theta_v0::lineage_book::test_set_consumer(Some(false));
        assert!(!super::enabled());
        crate::theta_v0::lineage_book::test_set_consumer(None);
    }

    /// ★#679 D1b 新默认：判据从「只看 `OPSEM_DUMP_DIR`」改为「谱系簿要建 或 要落盘」，
    /// 未置任何反证开关、未开落盘捕获 ⟹ `enabled()` 真（生产判径常开，票面范围第 1 条）。
    #[test]
    fn enabled_by_default_without_env_or_capture() {
        assert!(std::env::var("OPSEM_DUMP_DIR").is_err(), "本测试要求进程未设 OPSEM_DUMP_DIR");
        crate::theta_v0::lineage_book::test_set_consumer(None);
        assert!(super::enabled());
    }

    // Rc 未直接使用时避免 unused 警告（compose 内部持有 Rc<Vec<LeveledMove>>）。
    #[allow(dead_code)]
    fn _rc_marker(x: Rc<Vec<LeveledMove>>) -> usize { x.len() }
}
