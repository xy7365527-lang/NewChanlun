//! 规范序列（§5.2 的 M 树）的持久操作：concat / split / append。
//!
//! 正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md` §5.2
//! （唯一内容编码；序列分解形状按 `k = 严格小于 n 的最大 2 的幂` 唯一确定）。
//!
//! 目的：同核证据直接生成共享内容之后，增量 append / concat / split 不再物化旧历史。
//! 驱动只用「逻辑叶数 n」与 `split_index(n)`；不引入任何私有树形身份，结果与
//! `write_sequence` 对同一叶序列产出的规范根与节点逐字相同。
//!
//! **校验边界（照实，别当已核）**：
//! - `SequenceRef` 的两个字段是输入，不是已核事实；构造名一律带 `unchecked`。
//! - 每个算子只核**它触碰到的边界**：读到的节点必须 ① 摘要与 (tag, body) 重算相符
//!   （缺节点／换字节即拒绝），② tag 与 body 长度与该位置的期望叶数相符
//!   （0→0x11、1→0x12、n≥2→0x13），③ 分支节点的声明 n 等于期望 count，且直接子节点的
//!   声明计数等于 `split(n)` 给出的 k 与 n-k。
//! - 未被触碰的更深子树**不重核**；整棵闭包的规范形状核验仍归 `verify`（全树遍历）。
//!   本层只保证「写下/返回的树在每个被触碰节点上与规范分解一致」。
//! - 叶摘要的业务语义（例如叶本身不能是序列节点）由写值面与 `verify` 负责，
//!   本层与 `write_sequence` 同口径：叶摘要是 32 字节不透明值。
//! - 资源上限由调用方按 §5.6.1 计费；本模块不设默认值、不加缓存、不展开全叶。
//!
//! **成本（照实，按节点读写计）**：
//! - append：靶向实测与树高同阶（测试断言 ≤ 8·(叶数二进制位数+2)，深度 10/20/30/50 全过）；
//! - concat / split：**最坏复杂度未在本文件证明**。下面只是**已测构型**的读数，不外推到
//!   其它形状：2^51 个逻辑叶的重复压缩子树（两条 2^50 共享链合并后再在非规范边界切一刀）
//!   实测整段 append×2 + concat + split = 读 318 次、写 106 次，硬上限测试固定读 2^12、
//!   写 2^10。不同叶、不同共享结构的树没有覆盖，不能据此声称普遍界。
//!   互递归本身可能产生高次多项式；单次调用内的 `Memo` 只做子问题去重（不是跨调用缓存，
//!   也不是结果证明缓存），调用返回即丢。任何未证明的高次项都按「实测边界」记，不按定理记。
//! - 批量后缀重建 `sequence_rebuild_suffix`：内部节点写入为 popcount(keep)+m−1 ≤ m·(bits+1)，另写 m 个叶节点；m=0 且 keep<count 时等价于 split，不适用此上界。
//!
//! **写放大（照实）**：concat/split 会写入若干**中间规范节点**（例如某段前缀/后缀的
//! 规范根）。它们本身都是合法内容节点，但不一定被本次结果引用。已测构型：11 叶 concat
//! 多出 1 个中间节点；2^51 共享构型见上「成本」段。其它形状未测，测试给的仪表上界是
//! 8·bits²（不是最坏界证明）。清理归调用方的内容存储策略（§5.6.1 的 `max_store_bytes`）。
//! 批量后缀重建只写结果树右脊上的节点，不产生逐元素 append 那样的作废中间根。

#[cfg(feature = "s_allocation_json_testonly")]
use super::capture_allocation::work::WorkMap;
use super::capture_allocation::OwnedBudget;
use super::closed::CodecContext;
use super::{
    be_u64, checked_frame_len, digest_at, frame, length_overflow_error, parse_root, root_string,
    sequence_shape_error, split_index, tag, CodecError, NodeDigest, NodeSink, NodeSource,
    MAX_FRAME_LEN,
};
use std::collections::HashMap;
use std::hash::Hash;

/// 规范序列引用：公开身份只有「规范 M 根摘要 + 逻辑叶数」。
///
/// **两个字段都是输入，不是已核事实。** 构造不做任何读取或校验；校验发生在算子
/// 触碰到的边界（见模块文档）。因此构造名一律带 `unchecked`：拿到 `SequenceRef`
/// 不等于该根已被核过。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SequenceRef {
    root: NodeDigest,
    count: u64,
}

impl SequenceRef {
    /// 未核构造。`count` 超出 `2^63-1` 时由算子按 `length_overflow` 拒绝。
    pub const fn from_parts_unchecked(root: NodeDigest, count: u64) -> Self {
        Self { root, count }
    }

    /// 空序列 `M([]) = F(0x11, empty)`：此根完全由规范确定，不依赖 store 里有没有这个节点。
    pub fn empty() -> Self {
        Self {
            root: frame(tag::SEQ_EMPTY, &[]),
            count: 0,
        }
    }

    /// 未核构造：只严格解析 `sc1-` 根文本（前缀、长度、小写十六进制），不核节点。
    /// `count > 2^63-1` 当场拒绝（不是规范里能表达的叶数）。
    pub fn from_root_text_unchecked(root: &str, count: u64) -> Result<Self, CodecError> {
        if count > MAX_FRAME_LEN {
            return Err(length_overflow_error(format!(
                "序列叶数 {count} 超过 2^63-1"
            )));
        }
        Ok(Self {
            root: parse_root(root)?,
            count,
        })
    }

    pub const fn root(&self) -> NodeDigest {
        self.root
    }

    pub fn root_text(&self) -> String {
        root_string(&self.root)
    }

    pub const fn count(&self) -> u64 {
        self.count
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// 同一事务内既可读又可写的节点面。
///
/// 算子只取**一个** `&mut dyn NodeStore`：读自己刚写的节点不需要第二个冲突可变引用，
/// 结果创建后即可继续 append/concat。对端只读源与本地 sink 分离时，由调用方按自己的
/// 事务边界提供 overlay；本模块不偷偷加缓存，也不把一个 sink 冒充成可读来源。
pub trait NodeStore: NodeSource + NodeSink {}

impl<T: NodeSource + NodeSink + ?Sized> NodeStore for T {}

/// 已按期望叶数核过 header 的序列节点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeqNode {
    Empty,
    Leaf(NodeDigest),
    Branch {
        k: u64,
        left: NodeDigest,
        right: NodeDigest,
    },
}

/// 单节点核验：摘要↔(tag, body) 重算、tag/长度与期望 `count`、分支声明 n 与 k 规则。
/// 只核这一个节点，不下钻子节点（直接子节点计数由 `read_seq_node` 补）。
fn read_header(
    store: &mut CodecContext<'_, '_>,
    digest: &NodeDigest,
    count: u64,
) -> Result<SeqNode, CodecError> {
    if count > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "序列期望叶数 {count} 超过 2^63-1"
        )));
    }
    if count == 0 {
        // 空节点由规范唯一确定：其摘要即身份，无需读取即可判「这个根是不是空序列」。
        let empty = frame(tag::SEQ_EMPTY, &[]);
        if *digest != empty {
            return Err(sequence_shape_error(format!(
                "叶数为 0 的序列根必须是空序列节点 {}，实为 {}",
                root_string(&empty),
                root_string(digest)
            )));
        }
        return Ok(SeqNode::Empty);
    }
    // 活动工作域下读取签发的原正文许可随 body 持有，核完形状随 body 释放。
    let (t, body) = store.read(digest)?;
    if count == 1 {
        if t != tag::SEQ_LEAF {
            return Err(sequence_shape_error(format!(
                "叶数为 1 的序列根必须是单叶序列节点 0x12，实得 tag=0x{t:02x}（{}）",
                root_string(digest)
            )));
        }
        if body.len() != 32 {
            return Err(sequence_shape_error(format!(
                "单叶序列 body 必须是 32 字节叶摘要，实得 {} 字节（{}）",
                body.len(),
                root_string(digest)
            )));
        }
        return Ok(SeqNode::Leaf(digest_at(&body, 0)));
    }
    if t != tag::SEQ_BRANCH {
        return Err(sequence_shape_error(format!(
            "叶数 {count} ≥ 2 的序列根必须是分支序列节点 0x13，实得 tag=0x{t:02x}（{}）",
            root_string(digest)
        )));
    }
    if body.len() != 72 {
        return Err(sequence_shape_error(format!(
            "分支序列 body 必须是 U64(n) + 左右子摘要（72 字节），实得 {} 字节（{}）",
            body.len(),
            root_string(digest)
        )));
    }
    let declared = be_u64(&body);
    if declared > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "分支序列声明 n={declared} 超过 2^63-1（{}）",
            root_string(digest)
        )));
    }
    if declared != count {
        return Err(sequence_shape_error(format!(
            "分支序列声明 n={declared} 与期望叶数 {count} 不符（{}）",
            root_string(digest)
        )));
    }
    let k = split_index(count).expect("count ≥ 2 时 k 必然存在");
    Ok(SeqNode::Branch {
        k,
        left: digest_at(&body, 8),
        right: digest_at(&body, 40),
    })
}

/// 触碰边界核验：本节点 + （分支时）两个直接子节点按各自应有计数核验。
/// 这是 §5.2 的 k 规则在本地边界的实例；未下钻的更深子树不在此核。
fn read_seq_node(
    store: &mut CodecContext<'_, '_>,
    digest: &NodeDigest,
    count: u64,
) -> Result<SeqNode, CodecError> {
    let node = read_header(store, digest, count)?;
    if let SeqNode::Branch { k, left, right } = node {
        read_header(store, &left, k)?;
        read_header(store, &right, count - k)?;
    }
    Ok(node)
}

/// 入口校验：`SequenceRef` 的根与叶数必须在 store 里自洽（触碰边界的一层）。
fn checked_input(store: &mut CodecContext<'_, '_>, seq: &SequenceRef) -> Result<(), CodecError> {
    read_seq_node(store, &seq.root, seq.count)?;
    Ok(())
}

fn branch_node(
    store: &mut CodecContext<'_, '_>,
    left: &NodeDigest,
    right: &NodeDigest,
    n: u64,
) -> Result<NodeDigest, CodecError> {
    let mut body = [0u8; 72];
    body[..8].copy_from_slice(&checked_frame_len(n)?);
    body[8..40].copy_from_slice(left);
    body[40..72].copy_from_slice(right);
    let digest = store.emit(tag::SEQ_BRANCH, &body)?;
    Ok(digest)
}

fn empty_node(store: &mut CodecContext<'_, '_>) -> Result<SequenceRef, CodecError> {
    let digest = store.emit(tag::SEQ_EMPTY, &[])?;
    Ok(SequenceRef::from_parts_unchecked(digest, 0))
}

/// 规范 concat：`M(A ++ B)`。两个输入的根与叶数先按触碰边界核过；结果节点已写入 store。
pub fn sequence_concat(
    store: &mut dyn NodeStore,
    left: &SequenceRef,
    right: &SequenceRef,
) -> Result<SequenceRef, CodecError> {
    sequence_concat_with_allocation(store, left, right, None)
}

/// 显式计量调用内两张 memo 的实际哈希表 Layout；沿用调用方原账本。
/// 节点 body、写栈和 store 自身分配不由本入口计量。
pub fn sequence_concat_with_allocation(
    store: &mut dyn NodeStore,
    left: &SequenceRef,
    right: &SequenceRef,
    budget: Option<&OwnedBudget>,
) -> Result<SequenceRef, CodecError> {
    let store = &mut CodecContext::Store(store);
    let mut memo = Memo::new(budget)?;
    checked_input(store, left)?;
    checked_input(store, right)?;
    if left.count.checked_add(right.count).is_none() || left.count + right.count > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "合并后叶数 {} + {} 超过 2^63-1",
            left.count, right.count
        )));
    }
    concat_core(store, left, right, &mut memo)
}

/// 规范 split：按逻辑叶位置切一段，返回 `(M(h[0:i]), M(h[i:n]))`。
/// `i > count` 拒绝。切在规范边界（i 等于某个 k）时不写新节点。
pub fn sequence_split(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    index: u64,
) -> Result<(SequenceRef, SequenceRef), CodecError> {
    sequence_split_with_allocation(store, seq, index, None)
}

/// 显式计量调用内 memo，互递归共用同一工作集；返回或错误时释放。
pub fn sequence_split_with_allocation(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    index: u64,
    budget: Option<&OwnedBudget>,
) -> Result<(SequenceRef, SequenceRef), CodecError> {
    let store = &mut CodecContext::Store(store);
    let mut memo = Memo::new(budget)?;
    if index > seq.count {
        return Err(sequence_shape_error(format!(
            "拆分位置 {index} 超过序列长度 {}",
            seq.count
        )));
    }
    checked_input(store, seq)?;
    split_core(store, seq, index, &mut memo)
}

/// 规范 append：在尾部追加一个叶摘要（`M(h ++ [leaf])`）。
/// 与 `write_sequence` 同口径，叶摘要是不透明的 32 字节值：本函数只写单叶序列节点本身，
/// 不读叶节点；叶的业务语义归写值面与 `verify`。
pub fn sequence_append(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    leaf: &NodeDigest,
) -> Result<SequenceRef, CodecError> {
    sequence_append_with_allocation(store, seq, leaf, None)
}

/// 显式计量 append 使用的调用内 memo；失败可能已有中间节点写入，事务由调用方回滚。
pub fn sequence_append_with_allocation(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    leaf: &NodeDigest,
    budget: Option<&OwnedBudget>,
) -> Result<SequenceRef, CodecError> {
    let store = &mut CodecContext::Store(store);
    let mut memo = Memo::new(budget)?;
    checked_input(store, seq)?;
    let leaf_digest = store.emit(tag::SEQ_LEAF, leaf)?;
    let tail = SequenceRef::from_parts_unchecked(leaf_digest, 1);
    concat_core(store, seq, &tail, &mut memo)
}

/// 批量后缀重建：保留 `seq` 前 `keep` 个叶，其后接 `leaves`，一次构造结果树。
/// 结果与「split 出前缀后逐个 sequence_append」逐位相同，但不写逐元素 append 的作废中间根。
pub fn sequence_rebuild_suffix(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    keep: u64,
    leaves: &[NodeDigest],
) -> Result<SequenceRef, CodecError> {
    sequence_rebuild_suffix_with_allocation(store, seq, keep, leaves, None)
}

/// 显式计量右脊栈的实际 allocator Layout，沿用原账本，返回或错误时释放。
/// 节点 body 与 store 自身分配不由本入口计量；失败可能已有节点写入，事务由调用方回滚。
pub fn sequence_rebuild_suffix_with_allocation(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    keep: u64,
    leaves: &[NodeDigest],
    budget: Option<&OwnedBudget>,
) -> Result<SequenceRef, CodecError> {
    sequence_rebuild_suffix_ctx(&mut CodecContext::Store(store), seq, keep, leaves, budget)
}

pub(super) fn sequence_rebuild_suffix_ctx(
    store: &mut CodecContext<'_, '_>,
    seq: &SequenceRef,
    keep: u64,
    leaves: &[NodeDigest],
    budget: Option<&OwnedBudget>,
) -> Result<SequenceRef, CodecError> {
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if budget.is_some() {
        return Err(CodecError::static_detail(
            super::capture_allocation::INCOMPLETE,
            "sequence rebuild 缺 Owned 构建，拒绝 Plain 兜底",
        ));
    }
    if keep > seq.count {
        return Err(sequence_shape_error(format!(
            "rebuild_suffix keep {keep} exceeds sequence count {}",
            seq.count
        )));
    }
    checked_input(store, seq)?;
    if leaves.is_empty() && keep == seq.count {
        return Ok(*seq);
    }
    let appended = u64::try_from(leaves.len())
        .map_err(|_| length_overflow_error("rebuild_suffix leaf count overflow".to_string()))?;
    let total = keep
        .checked_add(appended)
        .filter(|t| *t <= MAX_FRAME_LEN)
        .ok_or_else(|| {
            length_overflow_error(format!("rebuild_suffix total {keep}+{appended} overflow"))
        })?;
    if total == 0 {
        return empty_node(store);
    }
    let mut stack = super::CodecVec::<(NodeDigest, u64)>::new_with_allocation(budget);
    let mut rest = keep;
    let mut node = (seq.root, seq.count);
    while rest > 0 {
        if node.1 == rest && rest.is_power_of_two() {
            stack.try_push(node)?;
            break;
        }
        match read_seq_node(store, &node.0, node.1)? {
            SeqNode::Branch { k, left, right } => {
                if k <= rest {
                    stack.try_push((left, k))?;
                    rest -= k;
                    node = (right, node.1 - k);
                } else {
                    node = (left, k);
                }
            }
            _ => {
                return Err(sequence_shape_error(format!(
                    "rebuild_suffix descent reached non-branch with {rest} leaves remaining"
                )))
            }
        }
    }
    for leaf in leaves {
        let leaf_digest = store.emit(tag::SEQ_LEAF, leaf)?;
        stack.try_push((leaf_digest, 1))?;
        while stack.len() >= 2 && stack[stack.len() - 1].1 == stack[stack.len() - 2].1 {
            let right = stack.pop().expect("len>=2");
            let left = stack.pop().expect("len>=2");
            let size = left.1 + right.1;
            stack.try_push((branch_node(store, &left.0, &right.0, size)?, size))?;
        }
    }
    let mut acc = stack
        .pop()
        .ok_or_else(|| sequence_shape_error("rebuild_suffix empty stack".to_string()))?;
    while let Some(prev) = stack.pop() {
        let size = prev.1 + acc.1;
        acc = (branch_node(store, &prev.0, &acc.0, size)?, size);
    }
    Ok(SequenceRef::from_parts_unchecked(acc.0, total))
}

/// 单次调用内的子问题备忘：同一 `(摘要, 叶数[, 位置])` 只解一次。
///
/// 互递归的 concat/split 天然会重复遇到同一子问题（例如「叶 + T(2^j-1)」）；不去重就是
/// 指数级重算（实测 2^51 树在非规范边界切一刀会重算 >2^20 次读）。这张表是**调用内工作集**，
/// 规模随 O(log n) 级子问题数封顶，函数返回即丢弃——不是跨调用缓存，也不进公开身份。
///
/// 诚实边界：① key 含两个操作数的摘要与叶数（split 还含位置），长度/位置缺一不可；
/// ② 只有**成功**结果才写入（错误经 `?` 直接上抛，不会被记成成功）；③ 它只是去重，
/// 不构成任何复杂度定理——见模块文档的「成本」段。
type ConcatKey = (NodeDigest, u64, NodeDigest, u64);
type SplitKey = (NodeDigest, u64, u64);
type SplitValue = (SequenceRef, SequenceRef);

// None 保留原 std HashMap；Some 必须有实际 allocator，不能降级成普通表。
enum MemoMap<K, V> {
    Plain(HashMap<K, V>),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(WorkMap<K, V>),
}
impl<K: Eq + Hash, V> MemoMap<K, V> {
    fn new(budget: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        match budget {
            None => Ok(Self::Plain(HashMap::new())),
            Some(budget) => {
                #[cfg(feature = "s_allocation_json_testonly")]
                {
                    Ok(Self::Owned(WorkMap::new(Some(budget))))
                }
                #[cfg(not(feature = "s_allocation_json_testonly"))]
                {
                    let _ = budget;
                    Err(CodecError::static_detail(
                        super::capture_allocation::INCOMPLETE,
                        "sequence memo 缺 Owned 构建，拒绝 Plain 兜底",
                    ))
                }
            }
        }
    }
    fn get(&self, key: &K) -> Option<&V> {
        match self {
            Self::Plain(map) => map.get(key),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(map) => map.get(key),
        }
    }
    fn insert(&mut self, key: K, value: V) -> Result<Option<V>, CodecError> {
        match self {
            Self::Plain(map) => Ok(map.insert(key, value)),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(map) => map.insert(key, value),
        }
    }
}
struct Memo {
    concat: MemoMap<ConcatKey, SequenceRef>,
    split: MemoMap<SplitKey, SplitValue>,
}
impl Memo {
    fn new(budget: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        Ok(Self {
            concat: MemoMap::new(budget)?,
            split: MemoMap::new(budget)?,
        })
    }
}

/// 规范核心：只用叶数与 `split_index(n)` 驱动，不展开逻辑叶。
///
/// 递归形态直接来自 M 的定义：n ≥ 2 时 `T(n) = branch(T(k), T(n-k))`，
/// `k = 严格小于 n 的最大 2 的幂`。每一步只走被触碰的那一侧边界，不做全叶展开；
/// 已测构型的读数与最坏界说明见模块文档「成本」段（最坏复杂度未证明，只按实测边界记）。
/// 重复压缩子树（例如 2^50 个同叶）不会被展开成逻辑叶。
///
/// 前置：两个输入的有效性已按 `checked_input` 核过。
fn concat_core(
    store: &mut CodecContext<'_, '_>,
    left: &SequenceRef,
    right: &SequenceRef,
    memo: &mut Memo,
) -> Result<SequenceRef, CodecError> {
    if left.count == 0 {
        // 空 + 空 的结果必须落盘（0x11 节点），否则「结果可立即读回」在新 store 上不成立；
        // 空 + X 直接复用 X（X 的节点已由入口校验读存在）。
        return if right.count == 0 {
            empty_node(store)
        } else {
            Ok(*right)
        };
    }
    if right.count == 0 {
        return Ok(*left);
    }
    let key = (left.root, left.count, right.root, right.count);
    if let Some(hit) = memo.concat.get(&key) {
        return Ok(*hit);
    }
    let total = left.count + right.count; // 入口已核过 2^63-1 上限
    let k = split_index(total).expect("total ≥ 2 时 k 必然存在");
    let out = if left.count == k {
        // 左半正好是 A，右半只能是 B
        let digest = branch_node(store, &left.root, &right.root, total)?;
        SequenceRef::from_parts_unchecked(digest, total)
    } else if left.count > k {
        // 左半是 A 的前缀；右半是 A 的剩余再接 B
        let (prefix, rest) = split_core(store, left, k, memo)?;
        let right_half = concat_core(store, &rest, right, memo)?;
        let digest = branch_node(store, &prefix.root, &right_half.root, total)?;
        SequenceRef::from_parts_unchecked(digest, total)
    } else {
        // A 整体落在左半：左半是 A 再接 B 的前缀，右半是 B 的剩余
        let (head, tail) = split_core(store, right, k - left.count, memo)?;
        let left_half = concat_core(store, left, &head, memo)?;
        let digest = branch_node(store, &left_half.root, &tail.root, total)?;
        SequenceRef::from_parts_unchecked(digest, total)
    };
    memo.concat.insert(key, out)?;
    Ok(out)
}

/// 规范核心的拆分面：`T(n)` 在叶位置 `i` 切开。
/// `i` 落在规范边界（等于某个 k）时直接返回既有子树，不写新节点。
///
/// 前置：`seq` 已按 `checked_input` 核过，且 `0 ≤ index ≤ seq.count`。
fn split_core(
    store: &mut CodecContext<'_, '_>,
    seq: &SequenceRef,
    index: u64,
    memo: &mut Memo,
) -> Result<(SequenceRef, SequenceRef), CodecError> {
    if index == 0 {
        return Ok((empty_node(store)?, *seq));
    }
    if index == seq.count {
        return Ok((*seq, empty_node(store)?));
    }
    let key = (seq.root, seq.count, index);
    if let Some(hit) = memo.split.get(&key) {
        return Ok(*hit);
    }
    // 0 < index < count ⟹ count ≥ 2，根必须是已被核过的分支节点
    let node = read_seq_node(store, &seq.root, seq.count)?;
    let SeqNode::Branch { k, left, right } = node else {
        return Err(sequence_shape_error(format!(
            "叶数 {} ≥ 2 的序列根必须是分支节点",
            seq.count
        )));
    };
    let left_ref = SequenceRef::from_parts_unchecked(left, k);
    let right_ref = SequenceRef::from_parts_unchecked(right, seq.count - k);
    let out = if index == k {
        (left_ref, right_ref)
    } else if index < k {
        let (head, mid) = split_core(store, &left_ref, index, memo)?;
        let tail = concat_core(store, &mid, &right_ref, memo)?;
        (head, tail)
    } else {
        let (mid, tail) = split_core(store, &right_ref, index - k, memo)?;
        let head = concat_core(store, &left_ref, &mid, memo)?;
        (head, tail)
    };
    memo.split.insert(key, out)?;
    Ok(out)
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod allocation_tests {
    use super::super::{capture_allocation::LIMIT, CodecVec, MemoryNodes};
    use super::*;

    // 用两张真实表的完整 key/value 类型，而非缩小后的代表类型。
    fn check_map_growth<K: Eq + Hash + Copy, V: Copy + Eq + std::fmt::Debug>(
        entry: impl Fn(u64) -> (K, V),
    ) {
        let budget = OwnedBudget::new(1 << 20);
        let mut baseline = CodecVec::<u8>::new_with_allocation(Some(&budget));
        baseline.try_reserve_exact(17).unwrap();
        let base = budget.snapshot().live_requested_layout_bytes;
        let mut map = WorkMap::<K, V>::new(Some(&budget));
        let (first_key, first_value) = entry(0);
        map.insert(first_key, first_value).unwrap();
        let capacity = map.capacity();
        for i in 1..capacity as u64 {
            let (key, value) = entry(i);
            map.insert(key, value).unwrap();
        }
        assert_eq!(map.len(), capacity);
        let old_bytes = map.retained_layout_bytes();
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            base + old_bytes
        );
        let (next_key, next_value) = entry(capacity as u64);
        map.insert(next_key, next_value).unwrap();
        let new_bytes = map.retained_layout_bytes();
        assert!(map.capacity() > capacity && new_bytes > old_bytes);
        assert_eq!(
            budget.snapshot().peak_requested_layout_bytes,
            base + old_bytes + new_bytes
        );
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            base + new_bytes
        );
        drop(map);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);

        let mut map = WorkMap::<K, V>::new(Some(&budget));
        for i in 0..capacity as u64 {
            let (key, value) = entry(i);
            map.insert(key, value).unwrap();
        }
        // 在同一个显式预算内占据真实剩余额度，留 new_bytes-1，必须拒绝整块grow。
        let mut blocker = CodecVec::<u8>::new_with_allocation(Some(&budget));
        let remaining =
            budget.snapshot().limit_bytes - budget.snapshot().live_requested_layout_bytes;
        blocker
            .try_reserve_exact(remaining - (new_bytes - 1))
            .unwrap();
        let blocked_base = budget.snapshot().live_requested_layout_bytes - old_bytes;
        let before = budget.snapshot();
        let error = map.insert(next_key, next_value).unwrap_err();
        assert_eq!(error.code(), LIMIT);
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, new_bytes);
        assert_eq!(
            probe.live_before_refusal,
            before.live_requested_layout_bytes
        );
        assert_eq!(probe.refused_kind, std::any::type_name::<(K, V)>());
        assert_eq!(map.capacity(), capacity);
        assert_eq!(map.retained_layout_bytes(), old_bytes);
        for i in 0..capacity as u64 {
            let (key, value) = entry(i);
            assert_eq!(map.get(&key), Some(&value));
        }
        assert!(map.get(&next_key).is_none());
        // 把最后余量也实际占满，重复key仍可替换且不会尝试分配。
        let mut rest = CodecVec::<u8>::new_with_allocation(Some(&budget));
        rest.try_reserve_exact(new_bytes - 1).unwrap();
        let attempts = budget.snapshot().allocation_attempts;
        assert_eq!(
            map.insert(first_key, next_value).unwrap(),
            Some(first_value)
        );
        assert_eq!(map.get(&first_key), Some(&next_value));
        assert_eq!(budget.snapshot().allocation_attempts, attempts);
        drop(rest);
        drop(map);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, blocked_base);
        drop(blocker);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(baseline);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(!budget.ledger_health().invariant_broken);
        println!(
            "{} map old={old_bytes} new={new_bytes} coexist={}",
            std::any::type_name::<(K, V)>(),
            old_bytes + new_bytes
        );
    }

    #[test]
    fn sequence_local_memo_real_concat_and_split_tables_grow_refuse_and_release() {
        check_map_growth::<ConcatKey, SequenceRef>(|i| {
            let mut digest = [0; 32];
            digest[..8].copy_from_slice(&i.to_be_bytes());
            (
                (digest, i, [2; 32], i + 1),
                SequenceRef::from_parts_unchecked(digest, i),
            )
        });
        check_map_growth::<SplitKey, SplitValue>(|i| {
            let mut digest = [0; 32];
            digest[..8].copy_from_slice(&i.to_be_bytes());
            let value = SequenceRef::from_parts_unchecked(digest, i);
            ((digest, i + 1, i), (value, value))
        });
    }

    #[test]
    fn sequence_noncanonical_split_populates_and_grows_both_local_tables_on_same_ledger() {
        let budget = OwnedBudget::new(1 << 20);
        let mut store = MemoryNodes::new();
        let mut root = frame(tag::SEQ_LEAF, &[7; 32]);
        store.put(root, tag::SEQ_LEAF, &[7; 32]).unwrap();
        for depth in 1..=10 {
            root = branch_node(&mut CodecContext::Store(&mut store), &root, &root, 1 << depth).unwrap();
        }
        let seq = SequenceRef::from_parts_unchecked(root, 1 << 10);
        let mut memo = Memo::new(Some(&budget)).unwrap();
        let out = split_core(&mut CodecContext::Store(&mut store), &seq, seq.count - 1, &mut memo).unwrap();
        assert_eq!(out.0.count + out.1.count, seq.count);
        let MemoMap::Owned(concat) = &memo.concat else {
            panic!("concat必须Owned")
        };
        let MemoMap::Owned(split) = &memo.split else {
            panic!("split必须Owned")
        };
        assert!(concat.len() > 3 && split.len() > 3);
        assert!(concat.capacity() > 3 && split.capacity() > 3);
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            concat.retained_layout_bytes() + split.retained_layout_bytes()
        );
        assert!(
            budget.snapshot().peak_requested_layout_bytes
                > budget.snapshot().live_requested_layout_bytes
        );
        drop(memo);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
