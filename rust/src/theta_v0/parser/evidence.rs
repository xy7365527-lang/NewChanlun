//! #1394：完整包含证据的**内部共享表示**（不变换判据，只换表示）。
//!
//! 本模块只服务 parser 内部：把「逐 bar 追加的完整证据」表示成**不可变、结构共享**的
//! 节点，使增量状态每追加 1 根只产生 O(1) 个新节点，而展开（materialize）仍与
//! [`super::inclusion::process_inclusion_with_facts`] 的全部字段逐位相同。
//!
//! 三条约束（#1394 已授权工程命题）：
//!
//! 1. **不丢来源**：坐标集合保留**排序 + 去重**，非连续 source 坐标原样保留；不得把
//!    坐标区间当稠密索引（区间化会丢掉「哪些原始 K 真的在里面」）。
//! 2. **不逐追加全复制**：每次追加只新建 O(1) 个节点；旧节点被后续步骤共享，不重写。
//! 3. **不深递归**：9011 层量级的右倾链只能**迭代**遍历（显式栈），不得用递归下降。
//!
//! ## 两类表示
//!
//! - [`ChunkedSeq`]：分块追加序列（原始 K 按到达序）。`push` 只碰末块，块内 `push` 摊销
//!   O(1)；遍历是块的顺序迭代。
//! - [`CoordsRef`]：升序去重坐标集合的共享句柄（`None` = 空集）。
//!   [`CoordsRef::concat`] 要求左段最大坐标 < 右段最小坐标——这是包含证据的实际形状
//!   （方向证据集的最大值 = 当前组锚，组成员都 ≥ 组锚），故拼接为 O(1) 且天然升序去重。

use std::rc::Rc;

/// 分块追加序列：追加 O(1) 摊销，遍历为块的顺序迭代（无递归）。
///
/// 块内 `push` 走 `Rc::make_mut`：本类型**不向外分发块句柄**，正常单持有者路径
/// （`strong_count == 1`）是 O(1)；一旦序列本身被 clone（块共享），下一次 `push` 只复制
/// 末块（≤ [`CHUNK`] 个元素），不是整个前缀。
#[derive(Debug, Clone)]
pub struct ChunkedSeq<T> {
    chunks: Vec<Rc<Vec<T>>>,
    len: usize,
}

impl<T> Default for ChunkedSeq<T> {
    fn default() -> Self {
        ChunkedSeq {
            chunks: Vec::new(),
            len: 0,
        }
    }
}

impl<T> ChunkedSeq<T> {
    /// 单块容量。取 128：块内追加的复制代价有界，块数（外层 Vec）增长足够慢。
    pub const CHUNK: usize = 128;

    #[cfg(test)]
    pub fn new() -> Self {
        ChunkedSeq {
            chunks: Vec::new(),
            len: 0,
        }
    }

    #[cfg(test)]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let mut offset = index;
        for chunk in &self.chunks {
            if offset < chunk.len() {
                return chunk.get(offset);
            }
            offset -= chunk.len();
        }
        None
    }

    /// 顺序迭代（外层块序 × 块内序）。返回的是 `&T`，调用方按需 clone。
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.chunks.iter().flat_map(|chunk| chunk.iter())
    }
}

impl<T: Clone> ChunkedSeq<T> {
    pub fn push(&mut self, value: T) {
        let need_new_chunk = self
            .chunks
            .last()
            .is_none_or(|chunk| chunk.len() >= Self::CHUNK);
        if need_new_chunk {
            let mut chunk = Vec::with_capacity(Self::CHUNK);
            chunk.push(value);
            self.chunks.push(Rc::new(chunk));
        } else {
            Rc::make_mut(self.chunks.last_mut().expect("刚判过非空"))
                .push(value);
        }
        self.len += 1;
    }

    pub fn to_vec(&self) -> Vec<T> {
        let mut out = Vec::with_capacity(self.len);
        out.extend(self.iter().cloned());
        out
    }
}

/// 升序去重坐标集合的不可变共享节点。
///
/// 不变量：每个节点自成一个**升序、无重复**的坐标序列；[`CoordNode::Concat`] 额外保证
/// 左段全部坐标 < 右段全部坐标。构造口只有 [`CoordsRef::concat`] 与
/// [`CoordsRef::append_greater`]（均要求该前提），保证不变量不被破坏。
#[derive(Debug)]
pub enum CoordNode {
    Leaf(usize),
    Concat(Rc<CoordNode>, Rc<CoordNode>),
}

/// 坐标集合句柄（`None` = 空集），O(1) clone。
#[derive(Debug, Clone, Default)]
pub struct CoordsRef(Option<Rc<CoordNode>>);

impl CoordsRef {
    pub fn empty() -> Self {
        CoordsRef(None)
    }

    pub fn single(coord: usize) -> Self {
        CoordsRef(Some(Rc::new(CoordNode::Leaf(coord))))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// 拼接：要求 `left` 的最大坐标 < `right` 的最小坐标（空集视作可拼接任意一侧）。
    ///
    /// 该前提在包含证据里天然成立——方向证据集的最大值 = 当前组锚，组成员坐标都 ≥ 组锚；
    /// 调用方 [`super::inclusion::IncrFactsState`] 只在此前提下调用，故输出必然升序去重。
    pub fn concat(left: &CoordsRef, right: &CoordsRef) -> Self {
        match (&left.0, &right.0) {
            (None, _) => right.clone(),
            (_, None) => left.clone(),
            (Some(l), Some(r)) => CoordsRef(Some(Rc::new(CoordNode::Concat(l.clone(), r.clone())))),
        }
    }

    /// 追加一个**严格大于**当前最大坐标的坐标（空集时等价于 [`CoordsRef::single`]）。
    pub fn append_greater(&self, coord: usize) -> Self {
        CoordsRef::concat(self, &CoordsRef::single(coord))
    }

    /// 展开为升序去重 `Vec<usize>`——显式栈迭代中序，无递归。
    pub fn materialize(&self) -> Vec<usize> {
        let Some(root) = &self.0 else {
            return Vec::new();
        };
        let mut out: Vec<usize> = Vec::new();
        let mut stack: Vec<&Rc<CoordNode>> = vec![root];
        while let Some(node) = stack.pop() {
            match node.as_ref() {
                CoordNode::Leaf(coord) => out.push(*coord),
                // 先压右后压左：左子树整体先于右子树展开（LIFO）。
                CoordNode::Concat(left, right) => {
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
        out
    }

    /// 最大坐标（不展开整棵树，沿最右路径迭代下行）。
    pub fn max(&self) -> Option<usize> {
        let mut node = self.0.as_ref()?;
        loop {
            match node.as_ref() {
                CoordNode::Leaf(coord) => return Some(*coord),
                CoordNode::Concat(_, right) => node = right,
            }
        }
    }

    /// 集合大小（不展开；每个叶计 1，每个内部节点只下潜）。
    pub fn count(&self) -> usize {
        let Some(root) = &self.0 else {
            return 0;
        };
        let mut total = 0usize;
        let mut stack: Vec<&Rc<CoordNode>> = vec![root];
        while let Some(node) = stack.pop() {
            match node.as_ref() {
                CoordNode::Leaf(_) => total += 1,
                CoordNode::Concat(left, right) => {
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_seq_appends_and_reads_in_order() {
        let mut seq = ChunkedSeq::new();
        for i in 0..(ChunkedSeq::<usize>::CHUNK * 3 + 7) {
            seq.push(i);
        }
        let flat = seq.to_vec();
        assert_eq!(flat.len(), ChunkedSeq::<usize>::CHUNK * 3 + 7);
        assert_eq!(flat, (0..flat.len()).collect::<Vec<_>>());
        assert_eq!(seq.get(0), Some(&0));
        assert_eq!(seq.get(ChunkedSeq::<usize>::CHUNK), Some(&ChunkedSeq::<usize>::CHUNK));
        assert_eq!(seq.get(ChunkedSeq::<usize>::CHUNK * 3 + 7), None);
    }

    #[test]
    fn coords_ref_materializes_sorted_unique_without_recursion() {
        // 模拟包含证据的形状：方向证据集（共享前缀）+ 组成员尾链，深 5000 层右倾。
        let mut deep = CoordsRef::empty();
        for i in 0..5000usize {
            deep = deep.append_greater(i);
        }
        let tail = CoordsRef::single(5000).append_greater(5001);
        let merged = CoordsRef::concat(&deep, &tail);
        assert_eq!(merged.count(), 5002);
        assert_eq!(merged.max(), Some(5001));
        let flat = merged.materialize();
        assert_eq!(flat.len(), 5002);
        assert_eq!(flat[0], 0);
        assert_eq!(flat[5001], 5001);
        assert!(flat.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn coords_ref_empty_identity() {
        let empty = CoordsRef::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.materialize(), Vec::<usize>::new());
        assert_eq!(empty.max(), None);
        let one = CoordsRef::single(7);
        assert_eq!(CoordsRef::concat(&empty, &one).materialize(), vec![7]);
        assert_eq!(CoordsRef::concat(&one, &empty).materialize(), vec![7]);
    }
}
