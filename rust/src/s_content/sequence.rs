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
//!
//! **写放大（照实）**：concat/split 会写入若干**中间规范节点**（例如某段前缀/后缀的
//! 规范根）。它们本身都是合法内容节点，但不一定被本次结果引用。已测构型：11 叶 concat
//! 多出 1 个中间节点；2^51 共享构型见上「成本」段。其它形状未测，测试给的仪表上界是
//! 8·bits²（不是最坏界证明）。清理归调用方的内容存储策略（§5.6.1 的 `max_store_bytes`）。

use super::{
    be_u64, checked_frame_len, digest_at, frame, length_overflow_error, parse_root, read_checked,
    root_string, sequence_shape_error, split_index, tag, CodecError, NodeDigest, NodeSink,
    NodeSource, MAX_FRAME_LEN,
};
use std::collections::HashMap;

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
    store: &mut dyn NodeStore,
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
    let (t, body) = read_checked(&mut *store, digest)?;
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
    store: &mut dyn NodeStore,
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
fn checked_input(store: &mut dyn NodeStore, seq: &SequenceRef) -> Result<(), CodecError> {
    read_seq_node(store, &seq.root, seq.count)?;
    Ok(())
}

fn branch_node(
    store: &mut dyn NodeStore,
    left: &NodeDigest,
    right: &NodeDigest,
    n: u64,
) -> Result<NodeDigest, CodecError> {
    let mut body = [0u8; 72];
    body[..8].copy_from_slice(&checked_frame_len(n)?);
    body[8..40].copy_from_slice(left);
    body[40..72].copy_from_slice(right);
    let digest = frame(tag::SEQ_BRANCH, &body);
    store.put(digest, tag::SEQ_BRANCH, &body)?;
    Ok(digest)
}

fn empty_node(store: &mut dyn NodeStore) -> Result<SequenceRef, CodecError> {
    let digest = frame(tag::SEQ_EMPTY, &[]);
    store.put(digest, tag::SEQ_EMPTY, &[])?;
    Ok(SequenceRef::from_parts_unchecked(digest, 0))
}

/// 规范 concat：`M(A ++ B)`。两个输入的根与叶数先按触碰边界核过；结果节点已写入 store。
pub fn sequence_concat(
    store: &mut dyn NodeStore,
    left: &SequenceRef,
    right: &SequenceRef,
) -> Result<SequenceRef, CodecError> {
    checked_input(store, left)?;
    checked_input(store, right)?;
    if left.count.checked_add(right.count).is_none() || left.count + right.count > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "合并后叶数 {} + {} 超过 2^63-1",
            left.count, right.count
        )));
    }
    concat_core(store, left, right, &mut Memo::default())
}

/// 规范 split：按逻辑叶位置切一段，返回 `(M(h[0:i]), M(h[i:n]))`。
/// `i > count` 拒绝。切在规范边界（i 等于某个 k）时不写新节点。
pub fn sequence_split(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    index: u64,
) -> Result<(SequenceRef, SequenceRef), CodecError> {
    if index > seq.count {
        return Err(sequence_shape_error(format!(
            "拆分位置 {index} 超过序列长度 {}",
            seq.count
        )));
    }
    checked_input(store, seq)?;
    split_core(store, seq, index, &mut Memo::default())
}

/// 规范 append：在尾部追加一个叶摘要（`M(h ++ [leaf])`）。
/// 与 `write_sequence` 同口径，叶摘要是不透明的 32 字节值：本函数只写单叶序列节点本身，
/// 不读叶节点；叶的业务语义归写值面与 `verify`。
pub fn sequence_append(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
    leaf: &NodeDigest,
) -> Result<SequenceRef, CodecError> {
    checked_input(store, seq)?;
    let leaf_digest = frame(tag::SEQ_LEAF, leaf);
    store.put(leaf_digest, tag::SEQ_LEAF, leaf)?;
    let tail = SequenceRef::from_parts_unchecked(leaf_digest, 1);
    concat_core(store, seq, &tail, &mut Memo::default())
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
#[derive(Default)]
struct Memo {
    concat: HashMap<(NodeDigest, u64, NodeDigest, u64), SequenceRef>,
    split: HashMap<(NodeDigest, u64, u64), (SequenceRef, SequenceRef)>,
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
    store: &mut dyn NodeStore,
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
    memo.concat.insert(key, out);
    Ok(out)
}

/// 规范核心的拆分面：`T(n)` 在叶位置 `i` 切开。
/// `i` 落在规范边界（等于某个 k）时直接返回既有子树，不写新节点。
///
/// 前置：`seq` 已按 `checked_input` 核过，且 `0 ≤ index ≤ seq.count`。
fn split_core(
    store: &mut dyn NodeStore,
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
    memo.split.insert(key, out);
    Ok(out)
}
