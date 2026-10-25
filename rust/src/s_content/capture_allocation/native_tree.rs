//! #1527：原审计NativeTree/NativeSet唯一实现下沉；库和bin共用。
use crate::s_content::{self as sc, CodecError};

/// #1488：节点上界许可随树拥有者移动；remove 不猜物理节点释放，clear 真释放。
/// 声明顺序保证树先析构，原 reserve token 最后析构；没有裸容器消费出口。
pub struct NativeTree<K, V> {
    tree: std::collections::BTreeMap<K, V>,
    high_water: usize,
    permits: sc::CodecVec<sc::capture_allocation::scratch::NativeReservation>,
}
impl<K, V> Default for NativeTree<K, V> {
    fn default() -> Self {
        Self {
            tree: Default::default(),
            high_water: 0,
            permits: Default::default(),
        }
    }
}
impl<K: Ord, V> NativeTree<K, V> {
    pub fn insert_admitted(
        &mut self,
        key: K,
        value: V,
        admission: &(impl sc::NodeSource + ?Sized),
    ) -> Result<Option<V>, CodecError> {
        if let Some(work) = admission.codec_work() {
            self.permits.try_reserve_native(work, 0)?;
        } else if self.permits.is_native_bound() {
            return Err(CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Native树操作缺当前工作域",
            ));
        }
        if !self.tree.contains_key(&key) && self.tree.len() >= self.high_water {
            if let Some(work) = admission.codec_work() {
                let depth = usize::BITS as usize - self.tree.len().leading_zeros() as usize + 2;
                let bytes = std::mem::size_of::<(K, V)>()
                    .checked_mul(11)
                    .and_then(|n| n.checked_add(16 * std::mem::size_of::<usize>()))
                    .and_then(|n| n.checked_mul(depth))
                    .ok_or_else(|| {
                        work.reject(sc::capture_allocation::scratch::ScratchFailure::Budget)
                    })?;
                self.permits.try_reserve_native(work, 1)?;
                let permit = work.reserve_native(bytes)?;
                self.permits.try_push(permit)?;
            }
            self.high_water = self.tree.len() + 1;
        }
        Ok(self.tree.insert(key, value))
    }
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.tree.remove(key)
    }
    /// 仅借出既有值；值的动态增长仍必须由其持费接口准入，不能改变树键或容量。
    pub fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.tree.get_mut(key)
    }
    /// 移出键值及其各自许可；树节点的保守高水位费仍留到 clear/drop。
    pub fn remove_entry<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.tree.remove_entry(key)
    }
    pub fn clear(&mut self) {
        self.tree.clear();
        self.high_water = 0;
        self.permits.clear();
    }
    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.tree.retain(f);
    }
    #[allow(dead_code, reason = "#1488 再合 #1608：8组合实测无调用方，保留不删")]
    pub fn permit_capacity(&self) -> usize {
        self.permits.capacity()
    }
    /// 仅供保留结构统计：含尚未 clear 的节点上界与许可缓冲；不用于父账退款。
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub fn allocation_bytes(&self) -> usize {
        let node_bound = std::mem::size_of::<(K, V)>()
            .saturating_mul(11)
            .saturating_add(16 * std::mem::size_of::<usize>());
        let nodes = if self.permits.is_native_bound() {
            self.permits
                .iter()
                .fold(0usize, |n, p| n.saturating_add(p.bytes()))
        } else {
            self.tree.len().saturating_mul(node_bound)
        };
        nodes.saturating_add(self.permits.capacity().saturating_mul(std::mem::size_of::<
            sc::capture_allocation::scratch::NativeReservation,
        >()))
    }
}
impl<K, V> std::ops::Deref for NativeTree<K, V> {
    type Target = std::collections::BTreeMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl<'a, K, V> IntoIterator for &'a NativeTree<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::btree_map::Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter()
    }
}
/// 树集合与节点许可同属一个拥有者；集合先析构，许可随后返还。
pub struct NativeSet<K> {
    values: NativeTree<K, ()>,
}
impl<K> Default for NativeSet<K> {
    fn default() -> Self {
        Self {
            values: NativeTree::default(),
        }
    }
}
impl<K: Ord> NativeSet<K> {
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub fn first(&self) -> Option<&K> {
        self.values.first_key_value().map(|(k, _)| k)
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub fn last(&self) -> Option<&K> {
        self.values.last_key_value().map(|(k, _)| k)
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub fn range<Q: Ord + ?Sized, R: std::ops::RangeBounds<Q>>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &K>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.values.range(range).map(|(key, _)| key)
    }
    pub fn insert_admitted(
        &mut self,
        key: K,
        admission: &(impl sc::NodeSource + ?Sized),
    ) -> Result<bool, CodecError> {
        self.values
            .insert_admitted(key, (), admission)
            .map(|old| old.is_none())
    }
    pub fn contains<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
    {
        self.values.contains_key(key)
    }
    pub fn take<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<K>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.values.remove_entry(key).map(|(key, ())| key)
    }
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
    {
        self.take(key).is_some()
    }
    pub fn clear(&mut self) {
        self.values.clear();
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn iter(&self) -> std::collections::btree_map::Keys<'_, K, ()> {
        self.values.keys()
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub fn allocation_bytes(&self) -> usize {
        self.values.allocation_bytes()
    }
}
impl<'a, K> IntoIterator for &'a NativeSet<K> {
    type Item = &'a K;
    type IntoIter = std::collections::btree_map::Keys<'a, K, ()>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.keys()
    }
}
