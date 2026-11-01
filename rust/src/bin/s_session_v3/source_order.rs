//! #1488 B2：确定平衡的 occurrence AVL；只保存顺序，不保存材料/业务资格。
//! 每个节点独立预付分配；没有整体 arena Vec 扩容、后缀改号或搬迁历史。
use super::*;
use crate::v3::audit_store::NativeTree;
use crate::v3::native_shared::{NativeRc as Rc, NativeWeak as Weak};
use std::cell::{Cell, RefCell};
pub(super) type Handle = usize;
type Link = Option<Rc<RefCell<Node>>>;
struct Node {
    handle: Handle,
    value: NodeDigest,
    left: Link,
    right: Link,
    parent: Weak<RefCell<Node>>,
    size: usize,
    height: usize,
    live: bool,
}
#[derive(Default)]
pub(super) struct Order {
    nodes: NativeTree<Handle, Rc<RefCell<Node>>>,
    node_bytes: usize,
    root: Link,
    next: usize,
    pub(super) visits: Cell<usize>,
}
impl Order {
    pub(super) fn len(&self) -> usize {
        Self::size(&self.root)
    }
    pub(super) fn retained_bytes(&self) -> usize {
        // 每个索引节点只计一次共享块；浅链接不重复计。仅统计，不据此退父账。
        self.nodes
            .allocation_bytes()
            .saturating_add(self.node_bytes)
    }
    fn size(at: &Link) -> usize {
        at.as_ref().map_or(0, |n| n.borrow().size)
    }
    fn height(at: &Link) -> usize {
        at.as_ref().map_or(0, |n| n.borrow().height)
    }
    fn parent(at: &Link, parent: Option<&Rc<RefCell<Node>>>) {
        if let Some(node) = at {
            node.borrow_mut().parent = parent.map_or_else(Weak::new, Rc::downgrade);
        }
    }
    fn update(&mut self, at: &Rc<RefCell<Node>>) {
        self.visits.set(self.visits.get() + 1);
        let (left, right) = {
            let node = at.borrow();
            (node.left.clone(), node.right.clone())
        };
        {
            let mut node = at.borrow_mut();
            node.size = 1 + Self::size(&left) + Self::size(&right);
            node.height = 1 + Self::height(&left).max(Self::height(&right));
        }
        Self::parent(&left, Some(at));
        Self::parent(&right, Some(at));
    }
    fn rotate_left(&mut self, at: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        let root = at.borrow().right.clone().expect("AVL right child");
        let middle = root.borrow().left.clone();
        at.borrow_mut().right = middle;
        root.borrow_mut().left = Some(at.clone());
        self.update(&at);
        self.update(&root);
        Self::parent(&Some(root.clone()), None);
        root
    }
    fn rotate_right(&mut self, at: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        let root = at.borrow().left.clone().expect("AVL left child");
        let middle = root.borrow().right.clone();
        at.borrow_mut().left = middle;
        root.borrow_mut().right = Some(at.clone());
        self.update(&at);
        self.update(&root);
        Self::parent(&Some(root.clone()), None);
        root
    }
    fn balance(&mut self, at: Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        self.update(&at);
        let (left, right) = {
            let n = at.borrow();
            (n.left.clone(), n.right.clone())
        };
        if Self::height(&left) > Self::height(&right) + 1 {
            let child = left.expect("AVL left-heavy");
            let reverse = {
                let n = child.borrow();
                Self::height(&n.right) > Self::height(&n.left)
            };
            if reverse {
                at.borrow_mut().left = Some(self.rotate_left(child));
            }
            self.rotate_right(at)
        } else if Self::height(&right) > Self::height(&left) + 1 {
            let child = right.expect("AVL right-heavy");
            let reverse = {
                let n = child.borrow();
                Self::height(&n.left) > Self::height(&n.right)
            };
            if reverse {
                at.borrow_mut().right = Some(self.rotate_right(child));
            }
            self.rotate_left(at)
        } else {
            Self::parent(&Some(at.clone()), None);
            at
        }
    }
    fn join(&mut self, left: Link, pivot: Rc<RefCell<Node>>, right: Link) -> Rc<RefCell<Node>> {
        if Self::height(&left) > Self::height(&right) + 1 {
            let root = left.expect("AVL join left");
            let child = root.borrow().right.clone();
            let next = self.join(child, pivot, right);
            root.borrow_mut().right = Some(next);
            self.balance(root)
        } else if Self::height(&right) > Self::height(&left) + 1 {
            let root = right.expect("AVL join right");
            let child = root.borrow().left.clone();
            let next = self.join(left, pivot, child);
            root.borrow_mut().left = Some(next);
            self.balance(root)
        } else {
            {
                let mut p = pivot.borrow_mut();
                p.left = left;
                p.right = right;
            }
            self.update(&pivot);
            Self::parent(&Some(pivot.clone()), None);
            pivot
        }
    }
    fn remove_last(&mut self, at: Rc<RefCell<Node>>) -> (Link, Rc<RefCell<Node>>) {
        let right = at.borrow().right.clone();
        match right {
            None => {
                let left = at.borrow_mut().left.take();
                Self::parent(&left, None);
                self.update(&at);
                (left, at)
            }
            Some(right) => {
                let (rest, last) = self.remove_last(right);
                at.borrow_mut().right = rest;
                (Some(self.balance(at)), last)
            }
        }
    }
    fn merge(&mut self, left: Link, right: Link) -> Link {
        let root = match (left, right) {
            (None, other) | (other, None) => other,
            (Some(left), right) => {
                let (left, pivot) = self.remove_last(left);
                Some(self.join(left, pivot, right))
            }
        };
        Self::parent(&root, None);
        root
    }
    fn split(&mut self, root: Link, count: usize) -> (Link, Link) {
        let Some(at) = root else { return (None, None) };
        self.visits.set(self.visits.get() + 1);
        let (left, right) = {
            let n = at.borrow();
            (n.left.clone(), n.right.clone())
        };
        let left_count = Self::size(&left);
        let result = if count <= left_count {
            let (a, b) = self.split(left, count);
            (a, Some(self.join(b, at, right)))
        } else {
            let (a, b) = self.split(right, count - left_count - 1);
            (Some(self.join(left, at, a)), b)
        };
        Self::parent(&result.0, None);
        Self::parent(&result.1, None);
        result
    }
    pub(super) fn height_bound(&self) -> usize {
        Self::height(&self.root)
    }
    pub(super) fn at(&self, mut rank: usize) -> Option<Handle> {
        let mut next = self.root.clone();
        while let Some(at) = next {
            self.visits.set(self.visits.get() + 1);
            let n = at.borrow();
            let left = Self::size(&n.left);
            if rank == left {
                return Some(n.handle);
            }
            if rank < left {
                next = n.left.clone();
            } else {
                rank -= left + 1;
                next = n.right.clone();
            }
        }
        None
    }
    pub(super) fn rank(&self, handle: Handle) -> Result<usize, String> {
        let mut at = self
            .nodes
            .get(&handle)
            .filter(|n| n.borrow().live)
            .cloned()
            .ok_or_else(|| err("来源顺序句柄已撤销"))?;
        let mut rank = Self::size(&at.borrow().left);
        loop {
            self.visits.set(self.visits.get() + 1);
            let parent = at.borrow().parent.upgrade();
            let Some(parent) = parent else { break };
            {
                let n = parent.borrow();
                if n.right.as_ref().is_some_and(|right| Rc::ptr_eq(right, &at)) {
                    rank += Self::size(&n.left) + 1;
                }
            }
            at = parent;
        }
        if !self.root.as_ref().is_some_and(|root| Rc::ptr_eq(root, &at)) {
            return Err(err("来源顺序句柄不属于当前根"));
        }
        Ok(rank)
    }
    pub(super) fn value(&self, handle: Handle) -> Result<NodeDigest, String> {
        self.nodes
            .get(&handle)
            .filter(|n| n.borrow().live)
            .map(|n| n.borrow().value)
            .ok_or_else(|| err("来源顺序句柄已撤销"))
    }
    pub(super) fn insert(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        rank: usize,
        value: NodeDigest,
    ) -> Result<Handle, String> {
        if rank > self.len() {
            return Err(err("来源有序新增位置越界"));
        }
        let handle = self.next;
        let next = self
            .next
            .checked_add(1)
            .ok_or_else(|| err("来源顺序序号溢出"))?;
        // Node 的动态成员只含已准入共享句柄；新块先准入，失败不消耗 handle。
        let node = Self::new_node(s, handle, value)?;
        let node_bytes = self
            .node_bytes
            .checked_add(node.allocation_bytes())
            .ok_or_else(|| err("来源顺序节点保留费溢出"))?;
        // 树先完成全部可能失败的准入，再提交索引、序号和 AVL 变更。
        self.nodes
            .insert_admitted(handle, node.clone(), s)
            .map_err(err)?;
        self.next = next;
        self.node_bytes = node_bytes;
        let root = self.root.take();
        let (a, b) = self.split(root, rank);
        let a = self.merge(a, Some(node));
        self.root = self.merge(a, b);
        Ok(handle)
    }
    fn new_node(
        s: &(impl CodecRead + ?Sized),
        handle: Handle,
        value: NodeDigest,
    ) -> Result<Rc<RefCell<Node>>, String> {
        Rc::try_new_shallow(
            RefCell::new(Node {
                handle,
                value,
                left: None,
                right: None,
                parent: Weak::new(),
                size: 1,
                height: 1,
                live: true,
            }),
            s.codec_work(),
        )
        .map_err(err)
    }
    pub(super) fn remove(
        &mut self,
        _s: &(impl CodecRead + ?Sized),
        rank: usize,
        expected: NodeDigest,
    ) -> Result<Handle, String> {
        let handle = self.at(rank).ok_or_else(|| err("来源有序删除位置越界"))?;
        if self.value(handle)? != expected {
            return Err(err("来源有序删除原件不同"));
        }
        let root = self.root.take();
        let (a, rest) = self.split(root, rank);
        let (removed, b) = self.split(rest, 1);
        let removed = removed.ok_or_else(|| err("来源有序删除句柄不同"))?;
        if removed.borrow().handle != handle {
            return Err(err("来源有序删除句柄不同"));
        }
        self.root = self.merge(a, b);
        {
            let mut n = removed.borrow_mut();
            n.live = false;
            n.parent = Weak::new();
        }
        Ok(handle)
    }
    pub(super) fn reclaim(
        &mut self,
        _s: &(impl CodecRead + ?Sized),
        handle: Handle,
    ) -> Result<(), String> {
        if self.nodes.get(&handle).is_none_or(|n| n.borrow().live) {
            return Err(err("来源顺序活句柄或已回收句柄不能回收"));
        }
        let node = self.nodes.remove(&handle).expect("已核来源顺序回收句柄");
        // 仅更新 O(1) 结构统计；node/外部 Link/Weak 各自决定真实许可何时释放。
        self.node_bytes = self
            .node_bytes
            .checked_sub(node.allocation_bytes())
            .expect("来源顺序节点保留费不下溢");
        if self.nodes.is_empty() {
            self.nodes.clear();
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn check(tree: &Order, expected: &[(Handle, NodeDigest)]) {
        assert_eq!(
            tree.node_bytes,
            tree.nodes
                .values()
                .map(|node| node.allocation_bytes())
                .sum::<usize>()
        );
        fn walk(
            at: &Link,
            parent: Option<&Rc<RefCell<Node>>>,
            out: &mut Vec<Handle>,
        ) -> (usize, usize) {
            let Some(at) = at else { return (0, 0) };
            let n = at.borrow();
            assert!(n.live);
            match (n.parent.upgrade(), parent) {
                (None, None) => {}
                (Some(a), Some(b)) => assert!(Rc::ptr_eq(&a, b)),
                _ => panic!("parent mismatch"),
            }
            let (ls, lh) = walk(&n.left, Some(at), out);
            out.push(n.handle);
            let (rs, rh) = walk(&n.right, Some(at), out);
            assert!(lh.abs_diff(rh) <= 1);
            assert_eq!(n.height, 1 + lh.max(rh));
            assert_eq!(n.size, 1 + ls + rs);
            (n.size, n.height)
        }
        let mut got = Vec::new();
        let (n, h) = walk(&tree.root, None, &mut got);
        assert_eq!(n, expected.len());
        let (mut a, mut b) = (0usize, 1usize);
        for _ in 0..h {
            let next = 1 + a + b;
            a = b;
            b = next;
        }
        assert!(n >= a);
        assert_eq!(got, expected.iter().map(|(h, _)| *h).collect::<Vec<_>>());
        for (rank, (handle, value)) in expected.iter().enumerate() {
            assert_eq!(tree.at(rank), Some(*handle));
            assert_eq!(tree.rank(*handle).unwrap(), rank);
            assert_eq!(tree.value(*handle).unwrap(), *value);
        }
    }
    #[test]
    fn b2_order_avl_stable_handles_survive_prefix_middle_and_reverse_edits() {
        let s = sc::MemoryNodes::default();
        for mode in 0..4 {
            let mut tree = Order::default();
            let mut expected = Vec::new();
            for i in 0..256usize {
                let mut value = [0; 32];
                value[..8].copy_from_slice(&(i as u64).to_be_bytes());
                let rank = match mode {
                    0 => expected.len(),
                    1 => 0,
                    2 => expected.len() / 2,
                    _ => (i * 37) % (expected.len() + 1),
                };
                let handle = tree.insert(&s, rank, value).unwrap();
                expected.insert(rank, (handle, value));
                check(&tree, &expected);
            }
            for i in 0..256usize {
                let rank = (i * 71) % expected.len();
                let (handle, value) = expected.remove(rank);
                assert_eq!(tree.remove(&s, rank, value).unwrap(), handle);
                tree.reclaim(&s, handle).unwrap();
                let rank = expected.len() - rank.min(expected.len());
                let handle = tree.insert(&s, rank, value).unwrap();
                expected.insert(rank, (handle, value));
                check(&tree, &expected);
            }
        }
    }
    #[test]
    fn b2_order_rejects_wrong_original_and_stale_handle_before_reuse() {
        let s = sc::MemoryNodes::default();
        let mut tree = Order::default();
        let handle = tree.insert(&s, 0, [1; 32]).unwrap();
        assert!(tree.remove(&s, 0, [2; 32]).is_err());
        assert_eq!(tree.rank(handle).unwrap(), 0);
        tree.remove(&s, 0, [1; 32]).unwrap();
        assert!(tree.rank(handle).is_err());
        assert!(tree.value(handle).is_err());
        tree.reclaim(&s, handle).unwrap();
        assert!(tree.reclaim(&s, handle).is_err());
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_order_reclaimed_node_and_weak_tail_keep_original_parent_charge() {
        let budget = crate::v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let (weak, node_bytes) = budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let scope_baseline = budget.live();
                let s = crate::v3::audit_store::NativeAdmission(Some(scope));
                let mut tree = Order::default();
                let handle = tree.insert(&s, 0, [1; 32]).unwrap();
                let node = tree.nodes.get(&handle).unwrap().clone();
                assert!(node.is_native_admitted());
                let node_bytes = node.allocation_bytes();
                let before_clone = budget.live();
                let weak = Rc::downgrade(&node);
                let other = weak.upgrade().unwrap();
                assert!(Rc::ptr_eq(&other, &node));
                assert_eq!(budget.live(), before_clone, "浅共享不重复准入");
                drop(other);
                tree.remove(&s, 0, [1; 32]).unwrap();
                tree.reclaim(&s, handle).unwrap();
                assert!(!node.borrow().live);
                assert!(tree.nodes.is_empty());
                assert_eq!(tree.retained_bytes(), tree.nodes.allocation_bytes());
                drop(tree);
                assert_eq!(budget.live(), scope_baseline + node_bytes);
                drop(node);
                assert!(weak.upgrade().is_none());
                assert_eq!(budget.live(), scope_baseline + node_bytes, "弱尾保整块");
                Ok::<_, String>((weak, node_bytes))
            })
            .unwrap();
        assert_eq!(budget.live(), baseline + node_bytes, "原许可跨工作域保留");
        drop(weak);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_order_admission_failure_preserves_avl_and_next_handle() {
        const LIMIT: usize = 1024 * 1024;
        for fail_at_tree in [false, true] {
            let budget = crate::v3::audit_owner::native::Budget::new(LIMIT, 0).unwrap();
            let baseline = budget.live();
            let (mut tree, mut expected) = budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let s = crate::v3::audit_store::NativeAdmission(Some(scope));
                    let mut tree = Order::default();
                    let mut expected = Vec::new();
                    for i in 0..3u8 {
                        let value = [i; 32];
                        let handle = tree.insert(&s, expected.len(), value).unwrap();
                        expected.push((handle, value));
                    }
                    Ok::<_, String>((tree, expected))
                })
                .unwrap();
            let next = tree.next;
            let root = tree.root.clone().unwrap();
            let failed = budget.codec_work().unwrap().run(|scope| {
                let s = crate::v3::audit_store::NativeAdmission(Some(scope));
                let probe = Order::new_node(&s, next, [8; 32]).unwrap();
                let node_bytes = probe.allocation_bytes();
                drop(probe);
                // 第一轮在共享块准入失败；第二轮仅够共享块，树新增槽准入失败。
                let available = if fail_at_tree { node_bytes } else { 0 };
                let guard = scope
                    .reserve_native(LIMIT - budget.live() - available)
                    .unwrap();
                let failure = tree.insert(&s, 1, [8; 32]).unwrap_err();
                assert_eq!(tree.next, next);
                assert_eq!(tree.nodes.len(), expected.len());
                assert!(Rc::ptr_eq(tree.root.as_ref().unwrap(), &root));
                assert!(!tree.nodes.contains_key(&next));
                check(&tree, &expected);
                drop(guard);
                // 资源失败粘在当前工作域；观测状态后立即返回，不在失败域重试。
                Err::<(), _>(failure)
            });
            assert!(failed.is_err());
            budget
                .codec_work()
                .unwrap()
                .run(|scope| {
                    let s = crate::v3::audit_store::NativeAdmission(Some(scope));
                    let handle = tree.insert(&s, 1, [8; 32]).unwrap();
                    assert_eq!(handle, next, "失败未消耗序号");
                    expected.insert(1, (handle, [8; 32]));
                    check(&tree, &expected);
                    Ok::<_, String>(())
                })
                .unwrap();
            drop(root);
            drop(tree);
            assert_eq!(budget.live(), baseline);
        }
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn native_order_rotations_remove_reinsert_and_drop_release_all_nodes() {
        let budget = crate::v3::audit_owner::native::Budget::new(4 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let scope_baseline = budget.live();
                let s = crate::v3::audit_store::NativeAdmission(Some(scope));
                let mut tree = Order::default();
                let mut expected = Vec::new();
                for i in 0..64u8 {
                    let rank = (usize::from(i) * 37) % (expected.len() + 1);
                    let value = [i; 32];
                    let handle = tree.insert(&s, rank, value).unwrap();
                    expected.insert(rank, (handle, value));
                    check(&tree, &expected);
                }
                for i in 0..64usize {
                    let rank = (i * 71) % expected.len();
                    let (old_handle, value) = expected.remove(rank);
                    assert_eq!(tree.remove(&s, rank, value).unwrap(), old_handle);
                    tree.reclaim(&s, old_handle).unwrap();
                    let handle = tree.insert(&s, 0, value).unwrap();
                    assert!(handle > old_handle);
                    expected.insert(0, (handle, value));
                    check(&tree, &expected);
                }
                for (handle, value) in expected.drain(..) {
                    assert_eq!(tree.remove(&s, 0, value).unwrap(), handle);
                    tree.reclaim(&s, handle).unwrap();
                }
                assert_eq!(tree.len(), 0);
                assert_eq!(tree.retained_bytes(), tree.nodes.allocation_bytes());
                drop(tree);
                assert_eq!(budget.live(), scope_baseline);
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(budget.live(), baseline);
    }
}
