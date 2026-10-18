//! #1488 G-only signed fixed-key compressed binary radix. No variable/text keys.
//! Algorithm references: fixed-index XERJ receipt and HANDOFF; no third-party source copied.
use super::audit_store::CodecRead;
use newchan_rust::s_content::{self as sc, CodecError};
use sc::capture_allocation::scratch::{NativeReservation, ScratchFailure, WorkScope};
use std::alloc::{alloc, dealloc, Layout};
use std::mem::ManuallyDrop;
use std::ops::{Bound, Deref, DerefMut, RangeBounds};
use std::ptr::NonNull;

mod sealed {
    pub trait Key {}
    impl Key for i64 {}
    impl Key for (i64, i64) {}
}
pub(crate) trait FixedKey: sealed::Key + Copy + Ord {
    const WIDTH: usize;
    fn encoded(self) -> u128;
}
impl FixedKey for i64 {
    const WIDTH: usize = 64;
    fn encoded(self) -> u128 {
        (((self as u64) ^ (1u64 << 63)) as u128) << 64
    }
}
impl FixedKey for (i64, i64) {
    const WIDTH: usize = 128;
    fn encoded(self) -> u128 {
        (((self.0 as u64) ^ (1u64 << 63)) as u128) << 64
            | (((self.1 as u64) ^ (1u64 << 63)) as u128)
    }
}
fn side(code: u128, bit: usize) -> usize {
    ((code >> (127 - bit)) & 1) as usize
}
fn suffix(bit: usize) -> u128 {
    if bit == 0 {
        u128::MAX
    } else {
        u128::MAX >> bit
    }
}
fn allocation_error(work: Option<&WorkScope>) -> CodecError {
    work.map_or_else(
        || {
            CodecError::new(
                sc::capture_allocation::LIMIT,
                "fixed radix allocation failed",
            )
        },
        |work| work.reject(ScratchFailure::Allocation),
    )
}

// Each block has a single owning holder. Guard exists before payload drop, and
// releases backing memory before its permit even while unwinding one payload panic.
struct Block<T> {
    ptr: NonNull<T>,
    permit: Option<NativeReservation>,
    _owned: std::marker::PhantomData<T>,
}
struct Deallocation<T> {
    ptr: NonNull<T>,
    permit: Option<NativeReservation>,
}
impl<T> Drop for Deallocation<T> {
    fn drop(&mut self) {
        // SAFETY: this guard uniquely owns an allocated T block whose payload was
        // dropped/read already. All tree payload layouts here are non-zero-sized.
        unsafe { dealloc(self.ptr.as_ptr().cast(), Layout::new::<T>()) };
        record_deallocation();
        drop(self.permit.take());
    }
}
impl<T> Block<T> {
    fn new(value: T, work: Option<&WorkScope>) -> Result<Self, CodecError> {
        let layout = Layout::new::<T>();
        assert!(
            layout.size() != 0,
            "only nonzero leaf/branch blocks are allocated"
        );
        let permit = work
            .map(|work| work.reserve_native(layout.size()))
            .transpose()?;
        // SAFETY: exact nonzero T layout; allocation is charged before this call.
        let raw = unsafe { alloc(layout) }.cast::<T>();
        let Some(ptr) = NonNull::new(raw) else {
            return Err(allocation_error(work));
        };
        record_allocation();
        // SAFETY: fresh aligned unique block, initialized exactly once.
        unsafe { ptr.as_ptr().write(value) };
        Ok(Self {
            ptr,
            permit,
            _owned: std::marker::PhantomData,
        })
    }
    fn into_inner(self) -> T {
        let mut owned = ManuallyDrop::new(self);
        let guard = Deallocation {
            ptr: owned.ptr,
            permit: owned.permit.take(),
        };
        // SAFETY: self uniquely owns initialized T; ManuallyDrop prevents a second drop.
        let value = unsafe { owned.ptr.as_ptr().read() };
        drop(guard);
        value
    }
}
impl<T> Deref for Block<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}
impl<T> DerefMut for Block<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}
impl<T> Drop for Block<T> {
    fn drop(&mut self) {
        let _guard = Deallocation {
            ptr: self.ptr,
            permit: self.permit.take(),
        };
        // SAFETY: unique live initialized payload. Guard handles allocation and
        // permit after normal drop or one panic; Rust drop glue visits other children.
        unsafe { std::ptr::drop_in_place(self.ptr.as_ptr()) };
    }
}
#[cfg(test)]
thread_local! { static BLOCK_EVENTS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) }; }
fn record_allocation() {
    #[cfg(test)]
    BLOCK_EVENTS.with(|e| {
        let (a, d) = e.get();
        e.set((a + 1, d));
    });
}
fn record_deallocation() {
    #[cfg(test)]
    BLOCK_EVENTS.with(|e| {
        let (a, d) = e.get();
        e.set((a, d + 1));
    });
}

#[cfg(test)]
thread_local! { static PATH_VISITS: std::cell::Cell<(usize,usize)> = const { std::cell::Cell::new((0,0)) }; }
fn lookup_visit() {
    #[cfg(test)]
    PATH_VISITS.with(|n| {
        let (a, b) = n.get();
        n.set((a + 1, b));
    });
}
fn range_visit() {
    #[cfg(test)]
    PATH_VISITS.with(|n| {
        let (a, b) = n.get();
        n.set((a, b + 1));
    });
}

struct Leaf<K, V> {
    key: K,
    value: V,
}
struct Branch<K, V> {
    bit: usize,
    prefix: u128,
    children: [Option<Link<K, V>>; 2],
}
enum Link<K, V> {
    Leaf(Block<Leaf<K, V>>),
    Branch(Block<Branch<K, V>>),
}
impl<K: FixedKey, V> Link<K, V> {
    fn interval(&self) -> (u128, u128) {
        range_visit();
        match self {
            Self::Leaf(leaf) => {
                let key = leaf.key.encoded();
                (key, key)
            }
            Self::Branch(branch) => (branch.prefix, branch.prefix | suffix(branch.bit)),
        }
    }
}
enum Ownership {
    Unbound,
    Plain,
    Native(NativeReservation),
}
pub(crate) struct FixedRadix<K: FixedKey, V> {
    root: Option<Link<K, V>>,
    len: usize,
    ownership: Ownership,
}
impl<K: FixedKey, V> Default for FixedRadix<K, V> {
    fn default() -> Self {
        Self {
            root: None,
            len: 0,
            ownership: Ownership::Unbound,
        }
    }
}
impl<K: FixedKey, V> FixedRadix<K, V> {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn len(&self) -> usize {
        self.len
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub(crate) fn check_admission(
        &mut self,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<(), CodecError> {
        let work = admission.codec_work();
        // Even replacements, empty clear/remove, and memo hits pass this gate.
        let probe = work.map(|work| work.reserve_native(0)).transpose()?;
        if admission.allocation_budget().is_some() {
            return Err(work.map_or_else(
                || CodecError::new(sc::code::SOURCE, "fixed radix rejects Owned admission"),
                |work| work.reject(ScratchFailure::Ledger),
            ));
        }
        match (&self.ownership, work) {
            (Ownership::Unbound, _) => {
                self.ownership = probe.map_or(Ownership::Plain, Ownership::Native);
                return Ok(());
            }
            (Ownership::Plain, None) => {}
            (Ownership::Native(parent), Some(work)) if parent.belongs_to_parent(work) => {}
            _ => {
                return Err(work.map_or_else(
                    || CodecError::new(sc::code::SOURCE, "fixed radix admission domain mismatch"),
                    |work| work.reject(ScratchFailure::Ledger),
                ))
            }
        }
        drop(probe);
        Ok(())
    }
    fn leaf(&self, key: K) -> Option<&Leaf<K, V>> {
        let code = key.encoded();
        let mut link = self.root.as_ref()?;
        loop {
            lookup_visit();
            match link {
                Link::Leaf(leaf) => return Some(leaf),
                Link::Branch(branch) => {
                    link = branch.children[side(code, branch.bit)]
                        .as_ref()
                        .expect("two children")
                }
            }
        }
    }
    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.leaf(*key)
            .filter(|leaf| leaf.key.encoded() == key.encoded())
            .map(|leaf| &leaf.value)
    }
    pub(crate) fn insert_admitted(
        &mut self,
        key: K,
        value: V,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<Option<V>, CodecError> {
        self.check_admission(admission)?;
        let code = key.encoded();
        let Some(found) = self.leaf(key) else {
            let leaf = Block::new(Leaf { key, value }, admission.codec_work())?;
            self.root = Some(Link::Leaf(leaf));
            self.len = 1;
            return Ok(None);
        };
        let difference = (code ^ found.key.encoded()).leading_zeros() as usize;
        if difference == 128 {
            let mut link = self.root.as_mut().expect("existing root");
            loop {
                lookup_visit();
                match link {
                    Link::Leaf(leaf) => return Ok(Some(std::mem::replace(&mut leaf.value, value))),
                    Link::Branch(branch) => {
                        let index = side(code, branch.bit);
                        link = branch.children[index].as_mut().expect("two children")
                    }
                }
            }
        }
        debug_assert!(difference < K::WIDTH);
        // Both actual blocks are admitted and allocated before a single old link moves.
        let leaf = Block::new(Leaf { key, value }, admission.codec_work())?;
        let mut branch = Block::new(
            Branch {
                bit: difference,
                prefix: code & !suffix(difference),
                children: [None, None],
            },
            admission.codec_work(),
        )?;
        let slot = Self::insertion_slot(&mut self.root, code, difference);
        let index = side(code, difference);
        branch.children[index] = Some(Link::Leaf(leaf));
        branch.children[1 - index] = slot.take();
        *slot = Some(Link::Branch(branch));
        self.len += 1;
        Ok(None)
    }
    fn insertion_slot(
        slot: &mut Option<Link<K, V>>,
        code: u128,
        bit: usize,
    ) -> &mut Option<Link<K, V>> {
        lookup_visit();
        let descend = matches!(slot.as_ref(),Some(Link::Branch(branch)) if branch.bit < bit);
        if descend {
            let Some(Link::Branch(branch)) = slot.as_mut() else {
                unreachable!()
            };
            let index = side(code, branch.bit);
            Self::insertion_slot(&mut branch.children[index], code, bit)
        } else {
            slot
        }
    }
    #[allow(dead_code)]
    pub(crate) fn remove_admitted(
        &mut self,
        key: &K,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<Option<V>, CodecError> {
        self.check_admission(admission)?;
        let old = Self::remove_link(&mut self.root, key.encoded());
        if old.is_some() {
            self.len -= 1;
        }
        Ok(old)
    }
    fn remove_link(slot: &mut Option<Link<K, V>>, code: u128) -> Option<V> {
        lookup_visit();
        match slot.as_mut()? {
            Link::Leaf(leaf) => {
                if leaf.key.encoded() != code {
                    return None;
                }
                let Some(Link::Leaf(leaf)) = slot.take() else {
                    unreachable!()
                };
                Some(leaf.into_inner().value)
            }
            Link::Branch(branch) => {
                let index = side(code, branch.bit);
                let old = Self::remove_link(&mut branch.children[index], code);
                if old.is_some() && branch.children[index].is_none() {
                    let Some(Link::Branch(branch)) = slot.take() else {
                        unreachable!()
                    };
                    let mut branch = branch.into_inner();
                    *slot = branch.children[1 - index].take();
                }
                old
            }
        }
    }
    pub(crate) fn clear_admitted(
        &mut self,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<(), CodecError> {
        self.check_admission(admission)?;
        // Set logical state before dropping payloads; ownership token stays bound even empty.
        self.len = 0;
        let old = self.root.take();
        drop(old);
        Ok(())
    }
    pub(crate) fn iter(&self) -> Iter<'_, K, V> {
        self.range(..)
    }
    pub(crate) fn values(&self) -> impl DoubleEndedIterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }
    pub(crate) fn range<R: RangeBounds<K>>(&self, range: R) -> Iter<'_, K, V> {
        let low = range.start_bound().map(|key| key.encoded());
        let high = range.end_bound().map(|key| key.encoded());
        if let (Bound::Included(a) | Bound::Excluded(a), Bound::Included(b) | Bound::Excluded(b)) =
            (low, high)
        {
            assert!(a <= b, "range start is greater than range end");
            assert!(
                !(a == b
                    && matches!(low, Bound::Excluded(_))
                    && matches!(high, Bound::Excluded(_))),
                "range start and end are equal and excluded"
            );
        }
        Iter {
            front: Cursor::new(self.root.as_ref(), false, low, high),
            back: Cursor::new(self.root.as_ref(), true, low, high),
            last_front: None,
            last_back: None,
            done: false,
        }
    }
}

// Two bounded DFS cursors prune disjoint prefix intervals. Each stack has 129
// link slots (W+1); no heap iterator, and shared crossing keys prevent duplicates.
struct Cursor<'a, K: FixedKey, V> {
    stack: [Option<&'a Link<K, V>>; 129],
    len: usize,
    reverse: bool,
    low: Bound<u128>,
    high: Bound<u128>,
}
impl<'a, K: FixedKey, V> Cursor<'a, K, V> {
    fn new(
        root: Option<&'a Link<K, V>>,
        reverse: bool,
        low: Bound<u128>,
        high: Bound<u128>,
    ) -> Self {
        let mut result = Self {
            stack: [None; 129],
            len: 0,
            reverse,
            low,
            high,
        };
        if let Some(root) = root {
            result.push(root);
        }
        result
    }
    fn intersects(&self, link: &Link<K, V>) -> bool {
        let (min, max) = link.interval();
        let lower = match self.low {
            Bound::Unbounded => true,
            Bound::Included(n) => max >= n,
            Bound::Excluded(n) => max > n,
        };
        let upper = match self.high {
            Bound::Unbounded => true,
            Bound::Included(n) => min <= n,
            Bound::Excluded(n) => min < n,
        };
        lower && upper
    }
    fn push(&mut self, link: &'a Link<K, V>) {
        if self.intersects(link) {
            assert!(self.len < 129, "fixed radix depth invariant");
            self.stack[self.len] = Some(link);
            self.len += 1;
        }
    }
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        while self.len != 0 {
            self.len -= 1;
            let node = self.stack[self.len].take().unwrap();
            match node {
                Link::Leaf(leaf) => return Some((&leaf.key, &leaf.value)),
                Link::Branch(branch) => {
                    let first = usize::from(self.reverse);
                    self.push(branch.children[1 - first].as_ref().expect("two children"));
                    self.push(branch.children[first].as_ref().expect("two children"));
                }
            }
        }
        None
    }
}
pub(crate) struct Iter<'a, K: FixedKey, V> {
    front: Cursor<'a, K, V>,
    back: Cursor<'a, K, V>,
    last_front: Option<u128>,
    last_back: Option<u128>,
    done: bool,
}
impl<'a, K: FixedKey, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let Some(item) = self.front.next() else {
            self.done = true;
            return None;
        };
        let code = item.0.encoded();
        if self.last_back.is_some_and(|last| code >= last) {
            self.done = true;
            return None;
        }
        self.last_front = Some(code);
        Some(item)
    }
}
impl<K: FixedKey, V> DoubleEndedIterator for Iter<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let Some(item) = self.back.next() else {
            self.done = true;
            return None;
        };
        let code = item.0.encoded();
        if self.last_front.is_some_and(|last| code <= last) {
            self.done = true;
            return None;
        }
        self.last_back = Some(code);
        Some(item)
    }
}
impl<K: FixedKey, V> std::iter::FusedIterator for Iter<'_, K, V> {}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::capture_allocation::scratch::{CodecWork, ScratchCharge, ScratchParent, WorkError};
    use sc::{NodeDigest, NodeRead, NodeSource};
    use std::collections::BTreeMap;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct Parent {
        live: AtomicUsize,
        attempts: AtomicUsize,
        peak: AtomicUsize,
        fail_on: AtomicUsize,
    }
    impl Parent {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                live: AtomicUsize::new(0),
                attempts: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                fail_on: AtomicUsize::new(usize::MAX),
            })
        }
    }
    impl ScratchParent for Parent {
        fn reserve(&self, n: usize, kind: ScratchCharge) -> Result<(), ScratchFailure> {
            if n != 0 && matches!(kind, ScratchCharge::Native) {
                let attempt = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt == self.fail_on.load(Ordering::SeqCst) {
                    return Err(ScratchFailure::Budget);
                }
            }
            let live = self.live.fetch_add(n, Ordering::SeqCst) + n;
            self.peak.fetch_max(live, Ordering::SeqCst);
            Ok(())
        }
        fn release(&self, n: usize, _: ScratchCharge) -> Result<(), ScratchFailure> {
            assert!(self.live.fetch_sub(n, Ordering::SeqCst) >= n);
            Ok(())
        }
        fn health(&self) -> Result<(), ScratchFailure> {
            Ok(())
        }
        fn invalidate(&self) {
            panic!("unbalanced radix test parent")
        }
    }
    struct Admission<'a>(Option<&'a WorkScope>);
    impl NodeSource for Admission<'_> {
        fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, CodecError> {
            Ok(NodeRead::Missing)
        }
        fn codec_work(&self) -> Option<&WorkScope> {
            self.0
        }
    }
    impl CodecRead for Admission<'_> {
        fn generic_source(&mut self) -> &mut dyn NodeSource {
            self
        }
    }
    fn run<T>(
        parent: &Arc<Parent>,
        f: impl FnOnce(&Admission<'_>) -> Result<T, CodecError>,
    ) -> Result<T, WorkError<CodecError>> {
        CodecWork::new(parent.clone(), 1 << 24)
            .unwrap()
            .run(|work| f(&Admission(Some(work))))
    }
    fn entries<K: FixedKey, V: Copy>(tree: &FixedRadix<K, V>) -> Vec<(K, V)> {
        tree.iter().map(|(k, v)| (*k, *v)).collect()
    }
    fn counts() -> (usize, usize) {
        BLOCK_EVENTS.with(|events| events.get())
    }

    #[test]
    fn signed_tuple_order_mixed_updates_and_double_ended_ranges_match_btree() {
        let plain = Admission(None);
        let mut tree = FixedRadix::new();
        let mut oracle = BTreeMap::new();
        let extremes = [i64::MIN, -100, -1, 0, 1, 100, i64::MAX];
        for (i, key) in extremes.into_iter().enumerate() {
            assert_eq!(
                tree.insert_admitted(key, i, &plain).unwrap(),
                oracle.insert(key, i)
            );
        }
        for i in 0..512usize {
            let key = ((i * 137) % 103) as i64 - 51;
            if i % 3 == 0 {
                assert_eq!(
                    tree.remove_admitted(&key, &plain).unwrap(),
                    oracle.remove(&key)
                );
            } else {
                assert_eq!(
                    tree.insert_admitted(key, i, &plain).unwrap(),
                    oracle.insert(key, i)
                );
            }
            assert_eq!(
                entries(&tree),
                oracle.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>()
            );
            assert_eq!(tree.len(), oracle.len());
        }
        for low in extremes {
            for high in extremes {
                if low <= high {
                    for (start, end) in [
                        (Bound::Included(low), Bound::Included(high)),
                        (Bound::Excluded(low), Bound::Included(high)),
                        (Bound::Included(low), Bound::Excluded(high)),
                    ] {
                        let expected = oracle
                            .range((start, end))
                            .map(|(k, v)| (*k, *v))
                            .collect::<Vec<_>>();
                        assert_eq!(
                            tree.range((start, end))
                                .map(|(k, v)| (*k, *v))
                                .collect::<Vec<_>>(),
                            expected
                        );
                        assert_eq!(
                            tree.range((start, end))
                                .rev()
                                .map(|(k, v)| (*k, *v))
                                .collect::<Vec<_>>(),
                            expected.iter().copied().rev().collect::<Vec<_>>()
                        );
                        let mut actual = tree.range((start, end));
                        let mut expected = expected.into_iter();
                        let mut front = true;
                        loop {
                            let x = if front {
                                actual.next()
                            } else {
                                actual.next_back()
                            };
                            let y = if front {
                                expected.next()
                            } else {
                                expected.next_back()
                            };
                            assert_eq!(x.map(|(k, v)| (*k, *v)), y);
                            if y.is_none() {
                                break;
                            }
                            front = !front;
                        }
                        assert!(actual.next().is_none());
                        assert!(actual.next_back().is_none());
                    }
                }
            }
        }
        let mut pairs = FixedRadix::new();
        let mut ordered = BTreeMap::new();
        for a in extremes {
            for b in extremes {
                pairs.insert_admitted((a, b), b, &plain).unwrap();
                ordered.insert((a, b), b);
            }
        }
        assert_eq!(entries(&pairs), ordered.into_iter().collect::<Vec<_>>());
        let slot = pairs.get(&(0, 0)).unwrap() as *const i64;
        assert_eq!(pairs.insert_admitted((0, 0), 99, &plain).unwrap(), Some(0));
        assert_eq!(
            pairs.get(&(0, 0)).unwrap() as *const i64,
            slot,
            "replace existing leaf in place"
        );
        tree.clear_admitted(&plain).unwrap();
        assert!(tree.is_empty());
        assert!(tree.iter().next_back().is_none());
    }
    #[test]
    fn excluded_equal_and_reversed_ranges_keep_btree_panic_contract() {
        let plain = Admission(None);
        let mut tree = FixedRadix::<i64, i64>::new();
        tree.insert_admitted(0, 0, &plain).unwrap();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tree.range(1..=0))).is_err()
        );
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || tree.range((Bound::Excluded(0), Bound::Excluded(0)))
        ))
        .is_err());
        assert_eq!(tree.range(..).next(), Some((&0, &0)));
        assert_eq!(tree.range(..0).next(), None);
        assert_eq!(tree.range(0..).next(), Some((&0, &0)));
    }
    #[test]
    fn native_each_new_block_refusal_preserves_contents_and_real_layout_ledger() {
        let parent = Parent::new();
        let mut tree = FixedRadix::<i64, usize>::new();
        parent.fail_on.store(1, Ordering::SeqCst);
        let before = counts();
        assert!(matches!(
            run(&parent, |a| tree.insert_admitted(0, 1, a)),
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert!(tree.is_empty());
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        assert_eq!(counts(), before);
        parent.fail_on.store(usize::MAX, Ordering::SeqCst);
        run(&parent, |a| tree.insert_admitted(0, 1, a)).unwrap();
        let leaf = Layout::new::<Leaf<i64, usize>>().size();
        let branch = Layout::new::<Branch<i64, usize>>().size();
        assert_eq!(parent.live.load(Ordering::SeqCst), leaf);
        for offset in [1, 2] {
            let attempts = parent.attempts.load(Ordering::SeqCst);
            parent.fail_on.store(attempts + offset, Ordering::SeqCst);
            parent.peak.store(leaf, Ordering::SeqCst);
            let old = entries(&tree);
            let blocks = counts();
            assert!(matches!(
                run(&parent, |a| tree.insert_admitted(7, 2, a)),
                Err(WorkError::Resource(ScratchFailure::Budget))
            ));
            assert_eq!(entries(&tree), old);
            assert_eq!(
                parent.peak.load(Ordering::SeqCst),
                if offset == 1 { leaf } else { 2 * leaf },
                "staged leaf stays charged until rejected branch preparation unwinds"
            );
            assert_eq!(parent.live.load(Ordering::SeqCst), leaf);
            let now = counts();
            assert_eq!(
                now.0 - blocks.0,
                now.1 - blocks.1,
                "any staged real blocks freed"
            );
        }
        parent.fail_on.store(usize::MAX, Ordering::SeqCst);
        parent.peak.store(leaf, Ordering::SeqCst);
        run(&parent, |a| tree.insert_admitted(7, 2, a)).unwrap();
        assert_eq!(
            parent.peak.load(Ordering::SeqCst),
            2 * leaf + branch,
            "old leaf plus both prepared blocks are simultaneously charged"
        );
        assert_eq!(parent.live.load(Ordering::SeqCst), 2 * leaf + branch);
        run(&parent, |a| {
            assert_eq!(tree.insert_admitted(7, 3, a)?, Some(2));
            Ok(())
        })
        .unwrap();
        assert_eq!(parent.live.load(Ordering::SeqCst), 2 * leaf + branch);
        run(&parent, |a| {
            assert_eq!(tree.remove_admitted(&0, a)?, Some(1));
            Ok(())
        })
        .unwrap();
        assert_eq!(parent.live.load(Ordering::SeqCst), leaf);
        run(&parent, |a| tree.clear_admitted(a)).unwrap();
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        run(&parent, |a| tree.insert_admitted(1, 4, a)).unwrap();
        drop(tree);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn every_write_including_empty_clear_and_missing_remove_checks_domain() {
        let parent = Parent::new();
        let foreign = Parent::new();
        let mut tree = FixedRadix::<i64, usize>::new();
        run(&parent, |a| tree.insert_admitted(1, 1, a)).unwrap();
        assert!(tree.insert_admitted(1, 2, &Admission(None)).is_err());
        assert_eq!(tree.get(&1), Some(&1));
        assert!(matches!(
            run(&foreign, |a| tree.insert_admitted(1, 2, a)),
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        assert!(matches!(
            run(&foreign, |a| tree.clear_admitted(a)),
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        run(&parent, |a| tree.clear_admitted(a)).unwrap();
        assert!(tree.clear_admitted(&Admission(None)).is_err());
        assert!(tree.remove_admitted(&0, &Admission(None)).is_err());
        assert!(matches!(
            run(&foreign, |a| tree.remove_admitted(&0, a)),
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        assert!(matches!(
            run(&parent, |a| {
                let _ = a.0.unwrap().reject(ScratchFailure::Budget);
                tree.clear_admitted(a)
            }),
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert!(matches!(
            run(&parent, |a| {
                let _ = a.0.unwrap().reject(ScratchFailure::Budget);
                tree.remove_admitted(&0, a)
            }),
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert!(matches!(
            run(&parent, |a| {
                let _ = a.0.unwrap().reject(ScratchFailure::Budget);
                tree.insert_admitted(2, 99, a)
            }),
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        run(&parent, |a| tree.insert_admitted(2, 2, a)).unwrap();
        let mut plain = FixedRadix::<i64, usize>::new();
        plain.insert_admitted(0, 0, &Admission(None)).unwrap();
        assert!(matches!(
            run(&parent, |a| plain.insert_admitted(1, 1, a)),
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        drop(tree);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn one_panicking_payload_still_frees_all_real_blocks_and_permits() {
        struct Bomb(Arc<AtomicUsize>, bool);
        impl Drop for Bomb {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                if self.1 {
                    panic!("single payload panic")
                }
            }
        }
        let parent = Parent::new();
        let drops = Arc::new(AtomicUsize::new(0));
        let mut tree = FixedRadix::<i64, Bomb>::new();
        let before = counts();
        run(&parent, |a| {
            tree.insert_admitted(-1, Bomb(drops.clone(), true), a)?;
            tree.insert_admitted(1, Bomb(drops.clone(), false), a)?;
            Ok(())
        })
        .unwrap();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(tree))).is_err());
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        let after = counts();
        assert_eq!(after.0 - before.0, 3);
        assert_eq!(after.1 - before.1, 3);
    }
    #[test]
    fn zero_sized_payload_drop_is_not_elided() {
        thread_local! {static DROPS:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
        struct Zst;
        impl Drop for Zst {
            fn drop(&mut self) {
                DROPS.with(|n| n.set(n.get() + 1));
            }
        }
        let before = DROPS.with(|n| n.get());
        let mut tree = FixedRadix::<i64, Zst>::new();
        tree.insert_admitted(0, Zst, &Admission(None)).unwrap();
        let old = tree.insert_admitted(0, Zst, &Admission(None)).unwrap();
        drop(old);
        drop(tree);
        assert_eq!(DROPS.with(|n| n.get()), before + 2);
    }
    #[test]
    fn fixed_width_bounds_each_lookup_and_seek_path_as_history_grows() {
        let plain = Admission(None);
        let mut tree = FixedRadix::<i64, i64>::new();
        for size in [1, 16, 256, 4096] {
            for key in tree.len() as i64..size {
                PATH_VISITS.with(|n| n.set((0, 0)));
                tree.insert_admitted(key, key, &plain).unwrap();
                assert!(
                    PATH_VISITS.with(|n| n.get().0) <= 2 * (64 + 1),
                    "two bounded insertion paths"
                );
            }
            for key in [-1, 0, size / 2, size] {
                PATH_VISITS.with(|n| n.set((0, 0)));
                let _ = tree.get(&key);
                assert!(PATH_VISITS.with(|n| n.get().0) <= 64 + 1);
                PATH_VISITS.with(|n| n.set((0, 0)));
                let rows = tree.range(key..=key).count();
                assert!(rows <= 1);
                assert!(
                    PATH_VISITS.with(|n| n.get().1) <= 4 * (64 + 1),
                    "two cursor roots plus bounded prefix pruning"
                );
            }
        }
        let mut pairs = FixedRadix::<(i64, i64), usize>::new();
        for i in 0..1024 {
            pairs
                .insert_admitted((i / 32, i % 32), i as usize, &plain)
                .unwrap();
        }
        PATH_VISITS.with(|n| n.set((0, 0)));
        assert_eq!(pairs.get(&(17, 19)), Some(&(17 * 32 + 19)));
        assert!(PATH_VISITS.with(|n| n.get().0) <= 128 + 1);
    }
    #[test]
    fn owned_admission_is_not_silently_rebound_plain() {
        struct Owned(sc::capture_allocation::OwnedBudget);
        impl NodeSource for Owned {
            fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, CodecError> {
                Ok(NodeRead::Missing)
            }
        }
        impl CodecRead for Owned {
            fn generic_source(&mut self) -> &mut dyn NodeSource {
                self
            }
            fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
                Some(self.0.clone())
            }
        }
        let mut tree = FixedRadix::<i64, i64>::new();
        assert!(tree
            .insert_admitted(0, 0, &Owned(sc::capture_allocation::OwnedBudget::new(4096)))
            .is_err());
        assert!(tree.is_empty());
    }
    fn assert_range_oracle<K: FixedKey + std::fmt::Debug>(
        tree: &FixedRadix<K, usize>,
        oracle: &BTreeMap<K, usize>,
        bounds: (Bound<K>, Bound<K>),
    ) {
        let expected = oracle
            .range(bounds)
            .map(|(k, v)| (*k, *v))
            .collect::<Vec<_>>();
        assert_eq!(
            tree.range(bounds)
                .map(|(k, v)| (*k, *v))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            tree.range(bounds)
                .rev()
                .map(|(k, v)| (*k, *v))
                .collect::<Vec<_>>(),
            expected.iter().copied().rev().collect::<Vec<_>>()
        );
        for front_first in [true, false] {
            let mut actual = tree.range(bounds);
            let mut wanted = expected.clone().into_iter();
            let mut front = front_first;
            loop {
                let a = if front {
                    actual.next()
                } else {
                    actual.next_back()
                };
                let b = if front {
                    wanted.next()
                } else {
                    wanted.next_back()
                };
                assert_eq!(a.map(|(k, v)| (*k, *v)), b);
                if b.is_none() {
                    break;
                }
                front = !front;
            }
            assert!(actual.next().is_none());
            assert!(actual.next_back().is_none());
        }
    }
    fn valid_endpoint_matrix<K: FixedKey + std::fmt::Debug>(keys: &[K]) {
        let plain = Admission(None);
        let mut tree = FixedRadix::<K, usize>::new();
        let mut oracle = BTreeMap::new();
        // Re-run all bounds on fresh empty, single leaf and branching trees.
        for (phase, count) in [
            (0, 0),
            (1, 1),
            (2, keys.len()),
            (3, keys.len()),
            (4, keys.len()),
        ] {
            tree.clear_admitted(&plain).unwrap();
            oracle.clear();
            for (i, key) in keys.iter().copied().take(count).enumerate() {
                tree.insert_admitted(key, i, &plain).unwrap();
                oracle.insert(key, i);
            }
            if phase == 3 {
                for key in keys {
                    tree.remove_admitted(key, &plain).unwrap();
                    oracle.remove(key);
                }
            }
            if phase == 4 {
                tree.clear_admitted(&plain).unwrap();
                oracle.clear();
            }
            let mut bounds = vec![Bound::Unbounded];
            for key in keys.iter().copied() {
                bounds.extend([Bound::Included(key), Bound::Excluded(key)]);
            }
            for start in bounds.iter().copied() {
                for end in bounds.iter().copied() {
                    let valid = match (start, end) {
                        (
                            Bound::Included(a) | Bound::Excluded(a),
                            Bound::Included(b) | Bound::Excluded(b),
                        ) => {
                            a < b
                                || (a == b
                                    && !(matches!(start, Bound::Excluded(_))
                                        && matches!(end, Bound::Excluded(_))))
                        }
                        _ => true,
                    };
                    if valid {
                        assert_range_oracle(&tree, &oracle, (start, end));
                    }
                }
            }
        }
    }
    #[test]
    fn full_valid_endpoint_cartesian_matrix_for_both_fixed_widths() {
        valid_endpoint_matrix(&[i64::MIN, -1, 0, 1, i64::MAX]);
        valid_endpoint_matrix(&[
            (i64::MIN, i64::MIN),
            (-1, i64::MAX),
            (0, i64::MIN),
            (0, 0),
            (0, i64::MAX),
            (1, i64::MIN),
            (i64::MAX, i64::MAX),
        ]);
    }
    #[test]
    fn tuple_mixed_remove_replace_ranges_and_two_cursor_meeting_match_btree() {
        let plain = Admission(None);
        let mut tree = FixedRadix::new();
        let mut oracle = BTreeMap::new();
        for i in 0..768usize {
            let key = (((i * 13) % 9) as i64 - 4, ((i * 37) % 31) as i64 - 15);
            if i % 4 == 0 {
                assert_eq!(
                    tree.remove_admitted(&key, &plain).unwrap(),
                    oracle.remove(&key)
                );
            } else {
                assert_eq!(
                    tree.insert_admitted(key, i, &plain).unwrap(),
                    oracle.insert(key, i)
                );
            }
            assert_eq!(
                entries(&tree),
                oracle.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>()
            );
            assert_eq!(tree.len(), oracle.len());
            for bounds in [
                (Bound::Unbounded, Bound::Unbounded),
                (Bound::Excluded((0, -10)), Bound::Excluded((0, 10))),
                (Bound::Included((-2, 10)), Bound::Excluded((2, -10))),
                (Bound::Unbounded, Bound::Included(key)),
                (Bound::Excluded(key), Bound::Unbounded),
            ] {
                assert_range_oracle(&tree, &oracle, bounds);
            }
        }
        // Remove every surviving key in reverse order: exercise repeated sibling promotion.
        for key in oracle.keys().copied().rev().collect::<Vec<_>>() {
            assert_eq!(
                tree.remove_admitted(&key, &plain).unwrap(),
                oracle.remove(&key)
            );
            assert_range_oracle(&tree, &oracle, (Bound::Unbounded, Bound::Unbounded));
            assert_eq!(tree.len(), oracle.len());
        }
        assert!(tree.root.is_none());
    }

    fn remove_path_bound<K: FixedKey + std::fmt::Debug>(key: impl Fn(usize) -> K) {
        let plain = Admission(None);
        for size in [1usize, 16, 256, 4096] {
            let mut tree = FixedRadix::new();
            let mut oracle = BTreeMap::new();
            for i in 0..size {
                tree.insert_admitted(key(i), i, &plain).unwrap();
                oracle.insert(key(i), i);
            }
            for i in [size, 0, size / 2, size - 1, 0] {
                PATH_VISITS.with(|n| n.set((0, 0)));
                let blocks = counts();
                let got = tree.remove_admitted(&key(i), &plain).unwrap();
                let visits = PATH_VISITS.with(|n| n.get().0);
                let after = counts();
                assert_eq!(got, oracle.remove(&key(i)));
                assert!(
                    visits > 0 && visits <= K::WIDTH + 1,
                    "single fixed-width remove path"
                );
                assert_eq!(after.0, blocks.0, "remove allocates no real node");
                assert_eq!(
                    entries(&tree),
                    oracle.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>()
                );
            }
        }
    }
    #[test]
    fn remove_hit_miss_and_compression_keep_64_and_128_bit_work_bounds() {
        remove_path_bound(|i| i as i64);
        remove_path_bound(|i| ((i / 64) as i64, (i % 64) as i64));
    }
    #[test]
    fn clear_single_panic_leaves_empty_bound_tree_with_refunded_real_blocks() {
        struct Bomb(Arc<AtomicUsize>, bool);
        impl Drop for Bomb {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                if self.1 {
                    panic!("one clear payload panic");
                }
            }
        }
        let parent = Parent::new();
        let foreign = Parent::new();
        let drops = Arc::new(AtomicUsize::new(0));
        let mut tree = FixedRadix::<i64, Bomb>::new();
        let before = counts();
        run(&parent, |a| {
            tree.insert_admitted(-1, Bomb(drops.clone(), true), a)?;
            tree.insert_admitted(1, Bomb(drops.clone(), false), a)?;
            Ok(())
        })
        .unwrap();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = run(&parent, |a| tree.clear_admitted(a));
        }))
        .is_err());
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
        assert!(tree.root.is_none());
        assert!(tree.iter().next().is_none());
        assert!(tree.iter().next_back().is_none());
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        let cleared = counts();
        assert_eq!(cleared.0 - before.0, 3);
        assert_eq!(cleared.1 - before.1, 3);
        assert!(tree.clear_admitted(&Admission(None)).is_err());
        assert!(matches!(
            run(&foreign, |a| tree.clear_admitted(a)),
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        assert!(matches!(
            run(&parent, |a| {
                let _ = a.0.unwrap().reject(ScratchFailure::Budget);
                tree.clear_admitted(a)
            }),
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        run(&parent, |a| {
            tree.insert_admitted(2, Bomb(drops.clone(), false), a)
        })
        .unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(
            parent.live.load(Ordering::SeqCst),
            Layout::new::<Leaf<i64, Bomb>>().size()
        );
        run(&parent, |a| tree.clear_admitted(a)).unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 3);
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
        drop(tree);
        let after = counts();
        assert_eq!(after.0 - before.0, 4);
        assert_eq!(after.1 - before.1, 4);
    }
    fn invalid_bounds_are_rejected_even_without_root<K: FixedKey>(low: K, high: K) {
        let plain = Admission(None);
        let mut tree = FixedRadix::<K, usize>::new();
        for state in 0..4 {
            if state == 1 {
                tree.insert_admitted(low, 1, &plain).unwrap();
            }
            if state == 2 {
                tree.clear_admitted(&plain).unwrap();
            }
            if state == 3 {
                tree.insert_admitted(high, 2, &plain).unwrap();
                tree.remove_admitted(&high, &plain).unwrap();
            }
            for start in [Bound::Included(high), Bound::Excluded(high)] {
                for end in [Bound::Included(low), Bound::Excluded(low)] {
                    assert!(
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tree
                            .range((start, end))
                            .count()))
                        .is_err()
                    );
                }
            }
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tree
                    .range((Bound::Excluded(low), Bound::Excluded(low)))
                    .count()))
                .is_err()
            );
        }
    }
    #[test]
    fn illegal_range_contract_is_history_independent_including_empty_roots() {
        // Deliberate private API contract: std BTreeMap has an empty-root exception.
        invalid_bounds_are_rejected_even_without_root(-1i64, 1);
        invalid_bounds_are_rejected_even_without_root((0, -1), (0, 1));
    }
}
