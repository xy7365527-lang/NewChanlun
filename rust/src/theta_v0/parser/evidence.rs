//! #1394：完整包含证据的**内部共享表示**（不变换判据，只换表示）。
//!
//! 本模块只服务 parser 内部：把「逐 bar 追加的完整证据」表示成**不可变、结构共享**的
//! 节点，使坐标前缀每次追加只产生 O(log n) 个平衡树节点，而展开（materialize）仍与
//! [`super::inclusion::process_inclusion_with_facts`] 的全部字段逐位相同。
//!
//! 三条约束（#1394 已授权工程命题）：
//!
//! 1. **不丢来源**：坐标集合保留**排序 + 去重**，非连续 source 坐标原样保留；不得把
//!    坐标区间当稠密索引（区间化会丢掉「哪些原始 K 真的在里面」）。
//! 2. **不逐追加全复制**：追加沿 O(log n) 路径创建节点；未变子树共享，不重写。
//! 3. **不深递归**：坐标树保持 AVL 平衡，构造和析构深度为 O(log n)；展开用显式栈。
//!
//! ## 两类表示
//!
//! - [`ChunkedSeq`]：分块追加序列（原始 K 按到达序）。`push` 只碰末块，块内 `push` 摊销
//!   O(1)；遍历是块的顺序迭代。
//! - [`CoordsRef`]：升序去重坐标集合的共享句柄（`None` = 空集）。
//!   [`CoordsRef::concat`] 要求左段最大坐标 < 右段最小坐标——这是包含证据的实际形状
//!   （方向证据集的最大值 = 当前组锚，组成员都 ≥ 组锚），拼接保留原顺序并维持平衡。

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
            Rc::make_mut(self.chunks.last_mut().expect("刚判过非空")).push(value);
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
    Concat(Rc<CoordNode>, Rc<CoordNode>, u32),
}

fn height(node: &Rc<CoordNode>) -> u32 {
    match node.as_ref() {
        CoordNode::Leaf(_) => 1,
        CoordNode::Concat(_, _, height) => *height,
    }
}

fn branch(left: Rc<CoordNode>, right: Rc<CoordNode>) -> Rc<CoordNode> {
    let level = 1 + height(&left).max(height(&right));
    Rc::new(CoordNode::Concat(left, right, level))
}

/// 旋转只改变共享树形状，叶的顺序和旧版本原件保持。
fn balance(left: Rc<CoordNode>, right: Rc<CoordNode>) -> Rc<CoordNode> {
    if height(&left) > height(&right) + 1 {
        let CoordNode::Concat(a, b, _) = left.as_ref() else {
            unreachable!()
        };
        if height(a) >= height(b) {
            return branch(a.clone(), branch(b.clone(), right));
        }
        let CoordNode::Concat(c, d, _) = b.as_ref() else {
            unreachable!()
        };
        return branch(branch(a.clone(), c.clone()), branch(d.clone(), right));
    }
    if height(&right) > height(&left) + 1 {
        let CoordNode::Concat(a, b, _) = right.as_ref() else {
            unreachable!()
        };
        if height(b) >= height(a) {
            return branch(branch(left, a.clone()), b.clone());
        }
        let CoordNode::Concat(c, d, _) = a.as_ref() else {
            unreachable!()
        };
        return branch(branch(left, c.clone()), branch(d.clone(), b.clone()));
    }
    branch(left, right)
}

fn join(left: Rc<CoordNode>, right: Rc<CoordNode>) -> Rc<CoordNode> {
    if height(&left) > height(&right) + 1 {
        let CoordNode::Concat(a, b, _) = left.as_ref() else {
            unreachable!()
        };
        return balance(a.clone(), join(b.clone(), right));
    }
    if height(&right) > height(&left) + 1 {
        let CoordNode::Concat(a, b, _) = right.as_ref() else {
            unreachable!()
        };
        return balance(join(left, a.clone()), b.clone());
    }
    branch(left, right)
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
            (Some(l), Some(r)) => CoordsRef(Some(join(l.clone(), r.clone()))),
        }
    }

    /// 追加一个**严格大于**当前最大坐标的坐标（空集时等价于 [`CoordsRef::single`]）。
    pub fn append_greater(&self, coord: usize) -> Self {
        CoordsRef::concat(self, &CoordsRef::single(coord))
    }

    /// #1394 5.3：只读借出共享坐标树的根节点（空集 = `None`）。
    ///
    /// 供下游转换按**自身**唯一分解遍历（如 s-content/1 的 `M(...)` 按元素计数的二分）：
    /// 本树是 AVL 平衡后的**物理**形状——旋转只改形状、不改叶序——**不是**规范 M 结构，
    /// 也不决定任何公开内容身份（内容 Hash 只看逻辑元素序列）。
    ///
    /// 访问器提供不可变借用；调用方可以克隆 Rc 保留快照。持有返回句柄不影响本句柄后续
    /// 拼接（拼接新建节点，旧树保持原内容）。
    pub fn root_node(&self) -> Option<&Rc<CoordNode>> {
        self.0.as_ref()
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
                CoordNode::Concat(left, right, _) => {
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
                CoordNode::Concat(_, right, _) => node = right,
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
                CoordNode::Concat(left, right, _) => {
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
        total
    }
}

/// #1597：完整事实里来源坐标集合的**公开**表示（取代展开的 `Vec<usize>`）。
///
/// 两种内部形态，对外只暴露同一个逻辑序列：
///
/// - 共享：[`CoordsRef`] 平衡拼接树（升序去重）。完整事实入口在 source 坐标严格递增时
///   用它——第 i 步的证据坐标与前一步共享前缀，单步只新建 O(log n) 个节点，整份事实的
///   留存从 Θ(n²) 降到 O(n log n)。
/// - 扁平：`Rc<[usize]>`，原样保存元素顺序（含非升序）。非单调输入的回落路径、
///   手写夹具与 `From<Vec<usize>>` 用它。
///
/// **对外合同**（与旧 `Vec<usize>` 字段逐位相同的部分）：`Debug`（含 `{:#?}`）、
/// `serde::Serialize`（带长度的序列）、`PartialEq`/`Eq`（按元素序列比较，与内部形态无关）。
/// 克隆 O(1)（共享句柄）。不实现 `Deref<Target=[usize]>`：展开须显式调用
/// [`SourceCoords::to_vec`] 或按 [`SourceCoords::iter`] 遍历，避免隐式缓存把 Θ(n²) 带回来。
#[derive(Clone)]
pub struct SourceCoords(SourceRepr);

#[derive(Clone)]
enum SourceRepr {
    Shared(CoordsRef),
    /// 带完整性标记的共享树：树内含**全部**已处理 raw 中 ≤ `complete_upto` 的坐标（另可含
    /// 若干 > `complete_upto` 的坐标）。只由 parser 内部按构造保证的地方创建
    /// （[`SourceCoords::complete_prefix`] / [`union_prefixed`]），公开构造口不产出本形态。
    Prefix {
        tree: CoordsRef,
        complete_upto: usize,
    },
    Flat(Rc<[usize]>),
}

impl Default for SourceCoords {
    fn default() -> Self {
        SourceCoords(SourceRepr::Shared(CoordsRef::empty()))
    }
}

impl SourceCoords {
    /// 空集合。
    pub fn new() -> Self {
        Self::default()
    }

    /// 包装共享坐标树（升序去重由 [`CoordsRef`] 的构造口保证）。
    pub fn from_shared(coords: CoordsRef) -> Self {
        SourceCoords(SourceRepr::Shared(coords))
    }

    /// 共享坐标树句柄；扁平形态返回 `None`。
    pub fn as_shared(&self) -> Option<&CoordsRef> {
        match &self.0 {
            SourceRepr::Shared(coords) => Some(coords),
            SourceRepr::Prefix { tree, .. } => Some(tree),
            SourceRepr::Flat(_) => None,
        }
    }

    /// parser 内部：标记一棵**完整 raw 前缀**树（含全部已处理 raw 中 ≤ 其最大值的坐标）。
    /// 空树不标记（返回普通共享形态）。调用方对完整性负责。
    pub(crate) fn complete_prefix(tree: CoordsRef) -> Self {
        match tree.max() {
            Some(complete_upto) => SourceCoords(SourceRepr::Prefix {
                tree,
                complete_upto,
            }),
            None => SourceCoords(SourceRepr::Shared(tree)),
        }
    }

    /// parser 内部：带完整性标记的形态 ⟹ `(树, complete_upto)`。
    pub(crate) fn prefix_parts(&self) -> Option<(&CoordsRef, usize)> {
        match &self.0 {
            SourceRepr::Prefix {
                tree,
                complete_upto,
            } => Some((tree, *complete_upto)),
            _ => None,
        }
    }

    /// 按存储顺序遍历元素（共享形态为升序）。显式栈，无递归，不分配展开副本。
    pub fn iter(&self) -> SourceCoordsIter<'_> {
        match &self.0 {
            SourceRepr::Flat(items) => SourceCoordsIter::Flat(items.iter()),
            SourceRepr::Shared(coords) | SourceRepr::Prefix { tree: coords, .. } => {
                SourceCoordsIter::Tree(match coords.root_node() {
                    Some(root) => vec![root.as_ref()],
                    None => Vec::new(),
                })
            }
        }
    }

    /// 逆序遍历元素（共享形态为降序）。显式栈，惰性：`take_while` 截断时只走右侧路径。
    pub fn iter_rev(&self) -> impl Iterator<Item = usize> + '_ {
        let (flat, mut stack): (Option<std::iter::Rev<std::slice::Iter<'_, usize>>>, Vec<&CoordNode>) =
            match &self.0 {
                SourceRepr::Flat(items) => (Some(items.iter().rev()), Vec::new()),
                SourceRepr::Shared(coords) | SourceRepr::Prefix { tree: coords, .. } => (
                    None,
                    coords.root_node().map(|r| vec![r.as_ref()]).unwrap_or_default(),
                ),
            };
        let mut flat = flat;
        std::iter::from_fn(move || {
            if let Some(items) = flat.as_mut() {
                return items.next().copied();
            }
            while let Some(node) = stack.pop() {
                match node {
                    CoordNode::Leaf(coord) => return Some(*coord),
                    CoordNode::Concat(left, right, _) => {
                        stack.push(left.as_ref());
                        stack.push(right.as_ref());
                    }
                }
            }
            None
        })
    }

    /// 元素个数。共享形态需遍历叶，O(len)。
    pub fn len(&self) -> usize {
        match &self.0 {
            SourceRepr::Flat(items) => items.len(),
            SourceRepr::Shared(coords) | SourceRepr::Prefix { tree: coords, .. } => coords.count(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.0 {
            SourceRepr::Flat(items) => items.is_empty(),
            SourceRepr::Shared(coords) | SourceRepr::Prefix { tree: coords, .. } => {
                coords.is_empty()
            }
        }
    }

    /// 展开为 `Vec<usize>`（与旧字段同值）。
    pub fn to_vec(&self) -> Vec<usize> {
        match &self.0 {
            SourceRepr::Flat(items) => items.to_vec(),
            SourceRepr::Shared(coords) | SourceRepr::Prefix { tree: coords, .. } => {
                coords.materialize()
            }
        }
    }

    /// 是否含 `coord`（线性扫描）。
    pub fn contains(&self, coord: &usize) -> bool {
        self.iter().any(|c| c == coord)
    }

    /// 首元素。
    pub fn first(&self) -> Option<usize> {
        self.iter().next().copied()
    }

    /// 末元素（共享形态沿最右路径下行，O(log n)）。
    pub fn last(&self) -> Option<usize> {
        match &self.0 {
            SourceRepr::Flat(items) => items.last().copied(),
            SourceRepr::Shared(coords) | SourceRepr::Prefix { tree: coords, .. } => coords.max(),
        }
    }

    /// 置空（只改本句柄，不影响共享同一棵树的其他句柄）。
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl From<Vec<usize>> for SourceCoords {
    fn from(items: Vec<usize>) -> Self {
        SourceCoords(SourceRepr::Flat(Rc::from(items)))
    }
}

impl From<&[usize]> for SourceCoords {
    fn from(items: &[usize]) -> Self {
        SourceCoords(SourceRepr::Flat(Rc::from(items)))
    }
}

impl FromIterator<usize> for SourceCoords {
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        SourceCoords::from(iter.into_iter().collect::<Vec<_>>())
    }
}

/// [`SourceCoords::iter`] 的迭代器。
pub enum SourceCoordsIter<'a> {
    Flat(std::slice::Iter<'a, usize>),
    Tree(Vec<&'a CoordNode>),
}

impl<'a> Iterator for SourceCoordsIter<'a> {
    type Item = &'a usize;
    fn next(&mut self) -> Option<&'a usize> {
        match self {
            SourceCoordsIter::Flat(items) => items.next(),
            SourceCoordsIter::Tree(stack) => {
                while let Some(node) = stack.pop() {
                    match node {
                        CoordNode::Leaf(coord) => return Some(coord),
                        // 先压右后压左：左子树整体先于右子树产出（LIFO）。
                        CoordNode::Concat(left, right, _) => {
                            stack.push(right.as_ref());
                            stack.push(left.as_ref());
                        }
                    }
                }
                None
            }
        }
    }
}

impl<'a> IntoIterator for &'a SourceCoords {
    type Item = &'a usize;
    type IntoIter = SourceCoordsIter<'a>;
    fn into_iter(self) -> SourceCoordsIter<'a> {
        self.iter()
    }
}

impl std::fmt::Debug for SourceCoords {
    /// 与 `Vec<usize>` 的 `Debug` 逐字相同（`debug_list`，含 `{:#?}` 换行缩进）。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl serde::Serialize for SourceCoords {
    /// 与 `Vec<usize>` 的序列化同形：带长度的序列。
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for coord in self.iter() {
            seq.serialize_element(coord)?;
        }
        seq.end()
    }
}

impl PartialEq for SourceCoords {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.as_shared(), other.as_shared()) {
            if let (Some(x), Some(y)) = (a.root_node(), b.root_node()) {
                if Rc::ptr_eq(x, y) {
                    return true;
                }
            }
        }
        self.iter().eq(other.iter())
    }
}
impl Eq for SourceCoords {}

impl PartialEq<[usize]> for SourceCoords {
    fn eq(&self, other: &[usize]) -> bool {
        self.iter().eq(other.iter())
    }
}
impl PartialEq<Vec<usize>> for SourceCoords {
    fn eq(&self, other: &Vec<usize>) -> bool {
        self.iter().eq(other.iter())
    }
}
impl<const N: usize> PartialEq<[usize; N]> for SourceCoords {
    fn eq(&self, other: &[usize; N]) -> bool {
        self.iter().eq(other.iter())
    }
}
impl PartialEq<SourceCoords> for Vec<usize> {
    fn eq(&self, other: &SourceCoords) -> bool {
        other == self
    }
}

/// #1597：若干来源集合的升序去重并集，共享构造（不复制前缀）。
///
/// `prefixed` 是带完整性标记的集合（[`SourceCoords::prefix_parts`]：树含全部已处理 raw 中
/// ≤ `complete_upto` 的坐标），`plain` 是其余集合的元素；全部元素都必须是已处理 raw 坐标
/// （调用方保证：组成员与证据坐标都取自已处理 raw）。
///
/// 取 `complete_upto` 最大的一份 `(T, c)`：并集中 ≤ c 的部分 = 全部已处理 raw ≤ c ⊆ T；
/// > c 的部分 = 各集合 > c 的元素。后者若落在 `(c, max T]` 内必须已在 T 中，> max T 的按升序
/// 接在 T 之后——结果与「全部拼接后排序去重」逐位相同。不满足（有元素落在 T 的缺口里）或
/// 没有带标记的集合 ⟹ `None`，调用方回落扁平构造。只遍历各集合 > c 的尾段（逆序到界即停）。
pub(crate) fn union_prefixed<'a>(
    prefixed: &[&'a SourceCoords],
    plain: impl IntoIterator<Item = &'a usize>,
) -> Option<SourceCoords> {
    let (best, (tree, bound)) = prefixed
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.prefix_parts().map(|p| (i, p)))
        .max_by_key(|(_, (_, c))| *c)?;
    if prefixed.iter().any(|c| c.prefix_parts().is_none()) {
        return None;
    }
    let top = tree.max().expect("带标记的树非空");
    let above = |c: &SourceCoords| -> Vec<usize> {
        c.iter_rev().take_while(|x| *x > bound).collect()
    };
    let own: std::collections::BTreeSet<usize> = above(prefixed[best]).into_iter().collect();
    let mut extra: Vec<usize> = plain.into_iter().copied().filter(|x| *x > bound).collect();
    for (i, c) in prefixed.iter().enumerate() {
        if i != best {
            extra.extend(above(c));
        }
    }
    extra.sort_unstable();
    extra.dedup();
    let mut out = tree.clone();
    for x in extra {
        if x <= top {
            if !own.contains(&x) {
                return None;
            }
        } else {
            out = out.append_greater(x);
        }
    }
    Some(SourceCoords(SourceRepr::Prefix {
        tree: out,
        complete_upto: bound,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coords_ref_balances_persistent_prefixes() {
        let mut all = CoordsRef::empty();
        let mut prefix = CoordsRef::empty();
        for i in 0..4096 {
            all = all.append_greater(i * 3);
            if i == 511 {
                prefix = all.clone();
            }
        }
        assert_eq!(
            prefix.materialize(),
            (0..512).map(|i| i * 3).collect::<Vec<_>>()
        );
        assert_eq!(
            all.materialize(),
            (0..4096).map(|i| i * 3).collect::<Vec<_>>()
        );
        let mut depth = 0;
        let mut pending = vec![(all.0.as_ref().unwrap(), 1)];
        while let Some((node, level)) = pending.pop() {
            depth = depth.max(level);
            if let CoordNode::Concat(left, right, ..) = node.as_ref() {
                pending.push((left, level + 1));
                pending.push((right, level + 1));
            }
        }
        assert!(depth <= 25, "4096 次持续追加仍形成深链：depth={depth}");
    }

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
        assert_eq!(
            seq.get(ChunkedSeq::<usize>::CHUNK),
            Some(&ChunkedSeq::<usize>::CHUNK)
        );
        assert_eq!(seq.get(ChunkedSeq::<usize>::CHUNK * 3 + 7), None);
    }

    #[test]
    fn coords_ref_materializes_sorted_unique_without_recursion() {
        // 方向证据集（共享前缀）+ 组成员尾链，持续追加 5000 次。
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

    /// 手工遍历共享坐标树（借用口暴露的物理形状）：叶序 = 升序去重集合。
    fn collect_leaves(node: &CoordNode) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            match current {
                CoordNode::Leaf(coord) => out.push(*coord),
                CoordNode::Concat(left, right, _) => {
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
        out
    }

    #[test]
    fn coords_ref_root_node_exposes_shared_tree_without_changing_materialize() {
        let mut coords = CoordsRef::empty();
        assert!(coords.root_node().is_none(), "空集无根节点");
        for i in 0..300usize {
            coords = coords.append_greater(i * 3 + 1);
        }
        let want = coords.materialize();
        let root = Rc::clone(coords.root_node().expect("非空集合必有根节点"));
        let leaves = collect_leaves(&root);
        assert_eq!(leaves, want, "共享树中序叶序 = 升序去重内容");
        assert_eq!(leaves.len(), coords.count(), "叶数 = 集合大小");
        assert_eq!(leaves.last().copied(), coords.max(), "最右叶 = 最大坐标");
        // 只读借出不改变原展开。
        assert_eq!(coords.materialize(), want, "借出根节点后展开结果变化");
        // 持有旧根句柄继续拼接：旧根内容保持（持久化共享，不就地改写）。
        let extended = coords.append_greater(100_000);
        assert_eq!(collect_leaves(&root), want, "旧根内容被后续拼接改写");
        assert_eq!(coords.materialize(), want, "原句柄内容被后续拼接改写");
        let extended_flat = extended.materialize();
        assert_eq!(extended_flat.len(), want.len() + 1);
        assert_eq!(&extended_flat[..want.len()], want.as_slice());
    }
}
