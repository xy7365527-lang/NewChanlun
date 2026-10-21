//! #1394 封闭审计读取边界。动态来源没有捕获能力；唯有具体 HashReader 自己选择
//! 与实际读取绑定的不可变 owner。两种路径都用同一业务函数，不传成功 token。
pub(crate) use super::history::HashReader;
use newchan_rust::s_content::{
    self as sc, CodecError, ContentRef, NodeDigest, NodeRead, NodeSink, NodeSource, NodeStore,
};

use sc::capture_allocation::native_owned::{NativeRefFields, NativeValue};
use sc::path::PathStep;
use std::io::Write;

/// 仅传递当前工作域的分配能力；不持 SQL、原件或核验资格。
pub(crate) struct NativeAdmission<'a>(
    pub(crate) Option<&'a sc::capture_allocation::scratch::WorkScope>,
);
impl NodeSource for NativeAdmission<'_> {
    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.0
    }
    fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, CodecError> {
        Err(CodecError::new(sc::code::SOURCE, "分配能力不能读取原件"))
    }
}
impl CodecRead for NativeAdmission<'_> {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}

/// #1488：节点上界许可随树拥有者移动；remove 不猜物理节点释放，clear 真释放。
/// 声明顺序保证树先析构，原 reserve token 最后析构；没有裸容器消费出口。
pub(crate) struct NativeTree<K, V> {
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
    pub(crate) fn insert_admitted(
        &mut self,
        key: K,
        value: V,
        admission: &(impl CodecRead + ?Sized),
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
                    .ok_or_else(|| admission.native_overflow())?;
                self.permits.try_reserve_native(work, 1)?;
                let permit = work.reserve_native(bytes)?;
                self.permits.try_push(permit)?;
            }
            self.high_water = self.tree.len() + 1;
        }
        Ok(self.tree.insert(key, value))
    }
    pub(crate) fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.tree.remove(key)
    }
    /// 仅借出既有值；值的动态增长仍必须由其持费接口准入，不能改变树键或容量。
    pub(crate) fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.tree.get_mut(key)
    }
    /// 移出键值及其各自许可；树节点的保守高水位费仍留到 clear/drop。
    pub(crate) fn remove_entry<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.tree.remove_entry(key)
    }
    pub(crate) fn clear(&mut self) {
        self.tree.clear();
        self.high_water = 0;
        self.permits.clear();
    }
    pub(crate) fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.tree.retain(f);
    }
    #[allow(dead_code, reason = "#1488 再合 #1608：各构建组合均无使用方（可能只在其他 testonly feature 下用），先量再动、保留不删")]
    pub(crate) fn permit_capacity(&self) -> usize {
        self.permits.capacity()
    }
    /// 仅供保留结构统计：含尚未 clear 的节点上界与许可缓冲；不用于父账退款。
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(crate) fn allocation_bytes(&self) -> usize {
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
/// std HashMap的表容量原许可；字段顺序保证旧块先释放再返费。
pub(crate) struct NativeMap<K, V> {
    map: std::collections::HashMap<K, V>,
    permit: Option<sc::capture_allocation::scratch::NativeReservation>,
}
impl<K, V> Default for NativeMap<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            permit: None,
        }
    }
}
impl<K: Eq + std::hash::Hash, V> NativeMap<K, V> {
    pub(crate) fn reserve_admitted(
        &mut self,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<(), CodecError> {
        let Some(work) = admission.codec_work() else {
            if self.permit.is_some() {
                return Err(CodecError::new(
                    sc::capture_allocation::INCOMPLETE,
                    "Native map缺当前工作域",
                ));
            }
            return self
                .map
                .try_reserve(1)
                .map_err(|_| CodecError::new(sc::capture_allocation::LIMIT, "map扩容失败"));
        };
        work.check_current()?;
        if self
            .permit
            .as_ref()
            .is_some_and(|p| !p.belongs_to_parent(work))
            || (self.permit.is_none() && self.map.capacity() != 0)
        {
            return Err(work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger));
        }
        let bytes = admission.native_hash_bytes::<K, V>(self.map.len(), self.map.capacity())?;
        if bytes == 0 {
            return Ok(());
        }
        let permit = work.reserve_native(bytes)?;
        let mut next = std::collections::HashMap::new();
        next.try_reserve(self.map.len() + 1).map_err(|_| {
            work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
        })?;
        next.extend(self.map.drain());
        drop(std::mem::replace(&mut self.map, next));
        self.permit = Some(permit);
        Ok(())
    }
    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        assert!(
            self.permit.is_none()
                || self.map.contains_key(&key)
                || self.map.len() < self.map.capacity()
        );
        self.map.insert(key, value)
    }
    pub(crate) fn remove<Q: std::hash::Hash + Eq + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.map.remove(key)
    }
    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }
    pub(crate) fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.map.retain(f);
    }
}
impl<K, V> std::ops::Deref for NativeMap<K, V> {
    type Target = std::collections::HashMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

/// 树集合与节点许可同属一个拥有者；集合先析构，许可随后返还。
pub(crate) struct NativeSet<K> {
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
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(crate) fn first(&self) -> Option<&K> {
        self.values.first_key_value().map(|(k, _)| k)
    }
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(crate) fn last(&self) -> Option<&K> {
        self.values.last_key_value().map(|(k, _)| k)
    }
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(crate) fn range<Q: Ord + ?Sized, R: std::ops::RangeBounds<Q>>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &K>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.values.range(range).map(|(key, _)| key)
    }
    pub(crate) fn insert_admitted(
        &mut self,
        key: K,
        admission: &(impl CodecRead + ?Sized),
    ) -> Result<bool, CodecError> {
        self.values
            .insert_admitted(key, (), admission)
            .map(|old| old.is_none())
    }
    pub(crate) fn contains<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
    {
        self.values.contains_key(key)
    }
    pub(crate) fn take<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<K>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.values.remove_entry(key).map(|(key, ())| key)
    }
    pub(crate) fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
    {
        self.take(key).is_some()
    }
    pub(crate) fn clear(&mut self) {
        self.values.clear();
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn iter(&self) -> std::collections::btree_map::Keys<'_, K, ()> {
        self.values.keys()
    }
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"))]
    pub(crate) fn allocation_bytes(&self) -> usize {
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

/// #1394：内部字段定位借用原键，Owned路径不先构造未持账的String。
pub(crate) struct BorrowedFieldPath<'a>(pub(crate) &'a str);
impl sc::path::PathView for BorrowedFieldPath<'_> {
    fn len(&self) -> usize {
        1
    }
    fn step(&self, index: usize) -> Option<sc::path::PathStepView<'_>> {
        (index == 0).then_some(sc::path::PathStepView::Field(self.0))
    }
}

/// 字段路径的文本与原许可共同存活；使用 PathView 而非拆出裸 String。
#[allow(dead_code, reason = "#1488 再合 #1608：各构建组合均无使用方（可能只在其他 testonly feature 下用），先量再动、保留不删")]
pub(crate) struct NativeFieldPath(NativeValue<String>);
impl sc::path::PathView for NativeFieldPath {
    fn len(&self) -> usize {
        1
    }
    fn step(&self, index: usize) -> Option<sc::path::PathStepView<'_>> {
        (index == 0).then_some(sc::path::PathStepView::Field(self.0.as_str()))
    }
}

/// 仅bin内的静态codec门；具体捕获owner不可擦除成外来来源。
pub(crate) trait CodecRead: NodeSource {
    #[must_use = "原许可必须由实际载荷持有至析构"]
    fn native_reservation(
        &self,
        bytes: usize,
    ) -> Result<Option<sc::capture_allocation::scratch::NativeReservation>, CodecError> {
        self.codec_work()
            .map(|work| work.reserve_native(bytes))
            .transpose()
    }
    fn native_overflow(&self) -> CodecError {
        self.codec_work().map_or_else(
            || CodecError::new(sc::capture_allocation::LIMIT, "Native容量溢出"),
            |w| w.reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
        )
    }
    fn native_vec_room<T>(
        &self,
        values: &mut sc::CodecVec<T>,
        additional: usize,
    ) -> Result<(), CodecError> {
        match self.codec_work() {
            Some(work) => values.try_reserve_native(work, additional),
            None => values.try_reserve(additional),
        }
    }
    fn native_codec_room<T>(
        &self,
        values: &mut sc::CodecVec<T>,
        additional: usize,
    ) -> Result<(), CodecError> {
        if let Some(work) = self.codec_work() {
            values.try_reserve_native(work, additional)?;
        }
        Ok(())
    }
    fn native_hash_bytes<K, V>(&self, len: usize, capacity: usize) -> Result<usize, CodecError> {
        if self.codec_work().is_none() || len < capacity {
            return Ok(0);
        }
        let needed = len.checked_add(1).ok_or_else(|| self.native_overflow())?;
        // 固定 SwissTable 的容量档：4/8 桶各空一槽，其后 7/8 装载率。
        let buckets = if needed <= 3 {
            4
        } else if needed <= 7 {
            8
        } else {
            needed
                .checked_mul(8)
                .and_then(|n| n.checked_add(6))
                .map(|n| n / 7)
                .and_then(usize::checked_next_power_of_two)
                .ok_or_else(|| self.native_overflow())?
        };
        // 每桶 payload、控制字节及一组哨兵；额外 payload 对齐最多一个 bucket。
        buckets
            .checked_add(16)
            .and_then(|n| n.checked_mul(std::mem::size_of::<(K, V)>() + 1))
            .ok_or_else(|| self.native_overflow())
    }
    fn native_map_room<K: Eq + std::hash::Hash, V>(
        &self,
        map: &mut sc::capture_allocation::audit_json::BusinessMap<K, V>,
    ) -> Result<(), CodecError> {
        if let Some(work) = self.codec_work() {
            map.try_reserve_native(work, 1)?;
        }
        Ok(())
    }
    fn native_std_map_room<K: Eq + std::hash::Hash, V>(
        &self,
        map: &mut NativeMap<K, V>,
    ) -> Result<(), CodecError> {
        map.reserve_admitted(self)
    }
    /// #1488 新消费面必须持有返回 owner；不会经过旧 retain 接口后二次收费。
    fn native_owned_text(
        &self,
        text: &str,
    ) -> Result<sc::capture_allocation::native_owned::NativeValue<String>, CodecError> {
        sc::capture_allocation::native_owned::NativeValue::<String>::try_from_str(
            text,
            self.codec_work(),
        )
    }
    fn native_owned_ref(
        &self,
        reference: sc::RefView<'_>,
    ) -> Result<sc::capture_allocation::native_owned::NativeValue<ContentRef>, CodecError> {
        sc::capture_allocation::native_owned::NativeValue::<ContentRef>::try_from_view(
            reference,
            self.codec_work(),
        )
    }
    fn native_ref_clone(&self, value: &RefValue) -> Result<RefValue, CodecError> {
        match value {
            RefValue::Plain(r) => {
                if r.is_native_admitted() && self.codec_work().is_none() {
                    return Err(CodecError::new(
                        sc::capture_allocation::INCOMPLETE,
                        "Native引用复制需要当前WorkScope",
                    ));
                }
                self.native_plain_ref(r.into()).map(RefValue::Plain)
            }
            RefValue::Owned(r) => Ok(RefValue::Owned(*r)),
        }
    }
    /// 借用声明只复制文本，不提前核 digest，也不把 Plain 改成 Owned。
    fn native_plain_ref(&self, r: sc::RefView<'_>) -> Result<NativeValue<ContentRef>, CodecError> {
        self.native_owned_ref(r)
    }
    fn native_built_ref(&self, b: &sc::Built) -> Result<RefValue, CodecError> {
        if self.allocation_budget().is_some() {
            #[cfg(feature = "s_allocation_json_testonly")]
            return Ok(RefValue::Owned(b.owned_ref()));
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            return Err(CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "owned refs未启用TestOnly feature",
            ));
        }
        NativeValue::<ContentRef>::try_from_built(b, self.codec_work()).map(RefValue::Plain)
    }
    fn native_string(&self, text: &str) -> Result<NativeValue<String>, CodecError> {
        self.native_owned_text(text)
    }
    #[allow(dead_code, reason = "#1488 再合 #1608：各构建组合均无使用方（可能只在其他 testonly feature 下用），先量再动、保留不删")]
    fn native_field(&self, key: &str) -> Result<NativeFieldPath, CodecError> {
        self.native_string(key).map(NativeFieldPath)
    }
    fn native_format(
        &self,
        args: std::fmt::Arguments<'_>,
    ) -> Result<NativeValue<String>, CodecError> {
        NativeValue::<String>::try_format(args, self.codec_work())
    }
    fn native_verify_original(
        &self,
        capture: &sc::AuditCapture,
        reference: sc::RefView<'_>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        let Some(scope) = self.codec_work() else {
            return capture.verify_refview(reference, mode);
        };
        struct Current<'a> {
            capture: &'a sc::AuditCapture,
            scope: &'a sc::capture_allocation::scratch::WorkScope,
        }
        impl NodeSource for Current<'_> {
            fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
                Some(self.scope)
            }
            fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.read_scoped(digest)?.into_plain()
            }
            fn read_scoped(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<sc::job::ScopedNodeRead, CodecError> {
                let permit = self
                    .scope
                    .reserve_native(self.capture.original(digest).map_or(0, |(_, b)| b.len()))?;
                let value = self.capture.reader().read(digest)?;
                Ok(sc::job::ScopedNodeRead::from_plain(value, Some(permit)))
            }
            fn checked_read(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<Option<sc::job::CheckedFrame>, CodecError> {
                self.checked_read_scoped(digest)
                    .and_then(|f| f.map(sc::job::ScopedFrame::into_frame).transpose())
            }
            fn checked_read_scoped(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<Option<sc::job::ScopedFrame>, CodecError> {
                let permit = self
                    .scope
                    .reserve_native(self.capture.original(digest).map_or(0, |(_, b)| b.len()))?;
                let value = self.capture.reader().checked_read(digest)?;
                Ok(value.map(|f| sc::job::ScopedFrame::new(f, Some(permit))))
            }
        }
        sc::verify_refview(&mut Current { capture, scope }, reference, mode)
    }
    fn native_text(&self, text: &str) -> Result<super::business_json::TextValue, String> {
        if self.codec_work().is_some() && self.allocation_budget().is_none() {
            return self
                .native_string(text)
                .map(super::business_json::TextValue::Plain)
                .map_err(|e| e.to_string());
        }
        super::business_json::TextValue::from_str(text, self.allocation_budget().as_ref())
    }
    fn native_json_clone(
        &self,
        value: &serde_json::Value,
    ) -> Result<super::business_json::PlainJson, CodecError> {
        super::business_json::PlainJson::try_clone_from(value, self.codec_work())
    }
    fn parse_rendered_plain(
        &self,
        bytes: &[u8],
    ) -> Result<super::business_json::PlainJson, String> {
        super::business_json::strict_plain_json_admitted(bytes, self.codec_work())
    }

    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<RefValue>, CodecError> {
        let refs = sc::path::lookup_array_items_native(self.generic_source(), r)?;
        let mut out = sc::CodecVec::from_plain(Vec::new());
        self.native_codec_room(&mut out, refs.len())?;
        for r in refs {
            out.try_push(RefValue::Plain(r))?;
        }
        Ok(out)
    }
    fn verify_many_ref<'a>(
        &mut self,
        refs: impl IntoIterator<Item = sc::RefView<'a>>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        sc::audit_verify_many_refview(self.generic_source(), refs, mode)
    }
    fn verify_many_ref_fresh<'a>(
        &mut self,
        refs: impl IntoIterator<Item = sc::RefView<'a>>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        self.verify_many_ref(refs, mode)
    }
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        None
    }
    fn content_ref_view(&mut self, root: &str) -> Result<RefValue, CodecError> {
        sc::path::content_ref_of_native(self.generic_source(), root).map(RefValue::Plain)
    }
    fn lookup_ref(&mut self, r: sc::RefView<'_>, p: &[PathStep]) -> Result<RefValue, CodecError> {
        sc::path::lookup_path_native(self.generic_source(), r, p).map(RefValue::Plain)
    }
    fn lookup_field_ref(&mut self, r: sc::RefView<'_>, key: &str) -> Result<RefValue, CodecError> {
        sc::path::lookup_path_view_native(self.generic_source(), r, &BorrowedFieldPath(key))
            .map(RefValue::Plain)
    }
    fn lookup_optional_field_ref(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
    ) -> Result<Option<RefValue>, CodecError> {
        sc::path::lookup_optional_field_native(self.generic_source(), r, key)
            .map(|value| value.map(RefValue::Plain))
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        sc::path::lookup_fields_native(self.generic_source(), r, names, false).map(RefFields::Plain)
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        sc::path::lookup_fields_native(self.generic_source(), r, names, true).map(RefFields::Plain)
    }
    fn verify_ref(&mut self, r: sc::RefView<'_>, mode: sc::Mode) -> Result<(), CodecError> {
        sc::verify_refview(self.generic_source(), r, mode)
    }
    fn render_ref(&mut self, r: sc::RefView<'_>, out: &mut dyn Write) -> Result<(), CodecError> {
        sc::render_refview(self.generic_source(), r, out)
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource;
    #[allow(dead_code, reason = "#1608：特质默认方法，当前无调用方；保留不删")]
    fn new_ref_result(&mut self) -> sc::CodecVec<ContentRef> {
        sc::CodecVec::from_plain(Vec::new())
    }
    fn content_ref_of(&mut self, root: &str) -> Result<ContentRef, CodecError> {
        sc::content_ref_of(self.generic_source(), root)
    }
    fn lookup_path(&mut self, r: &ContentRef, p: &[PathStep]) -> Result<ContentRef, CodecError> {
        sc::path::lookup_path(self.generic_source(), r, p)
    }
    #[cfg_attr(not(test), allow(dead_code, reason = "#1608：仅测试使用"))]
    fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<sc::CodecVec<ContentRef>, CodecError> {
        sc::path::lookup_array_items(self.generic_source(), r).map(sc::CodecVec::from_plain)
    }
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        sc::path::lookup_object_fields(self.generic_source(), r, n).map(sc::CodecFields::from_plain)
    }
    fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        sc::path::lookup_object_fields_subset(self.generic_source(), r, n)
            .map(sc::CodecFields::from_plain)
    }
    fn render_canonical(&mut self, r: &ContentRef, w: &mut dyn Write) -> Result<(), CodecError> {
        sc::render_canonical(self.generic_source(), r, w)
    }
    #[cfg_attr(
        not(all(test, feature = "s_allocation_json_testonly")),
        allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
    )]
    fn verify_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match mode {
            sc::Mode::Public => sc::verify(self.generic_source(), r).map(|_| ()),
            sc::Mode::Template => sc::verify_template(self.generic_source(), r).map(|_| ()),
        }
    }
    fn verify_audit_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match mode {
            sc::Mode::Public => sc::audit_verify_public(self.generic_source(), r),
            sc::Mode::Template => sc::audit_verify(self.generic_source(), r, mode),
        }
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 下使用")
    )]
    fn verify_audit_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        sc::audit_verify_many(self.generic_source(), r, mode)
    }
    #[cfg_attr(
        not(all(test, feature = "s_capture_native_testonly")),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
    )]
    fn verify_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match mode {
            sc::Mode::Public => sc::verify_many(self.generic_source(), r).map(|_| ()),
            sc::Mode::Template => sc::verify_template_many(self.generic_source(), r).map(|_| ()),
        }
    }
    #[cfg_attr(
        not(all(test, feature = "s_allocation_json_testonly")),
        allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
    )]
    fn verify_cached(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        self.verify_fresh(r, mode)
    }
    #[cfg_attr(
        not(all(test, feature = "s_allocation_json_testonly")),
        allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
    )]
    fn verify_many_cached(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        self.verify_many_fresh(r, mode)
    }
}
pub(crate) trait CodecStore: CodecRead + NodeSink {
    fn replace_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
        batch: &str,
    ) -> Result<RefValue, CodecError> {
        sc::path::replace_batch_placeholder_native(self.generic_store(), r, p, batch)
            .map(RefValue::Plain)
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        sc::write_value_view(self.generic_store(), v)
    }
    fn write_template_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        sc::write_template_view(self.generic_store(), v)
    }
    fn write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<sc::Built, CodecError> {
        sc::write_json_view(self.generic_store(), v)
    }
    fn generic_store(&mut self) -> &mut dyn NodeStore;
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    fn write_value(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        sc::write_value(self.generic_store(), v)
    }
    #[cfg_attr(
        not(all(test, feature = "s_allocation_json_testonly")),
        allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
    )]
    fn write_template(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        sc::write_template(self.generic_store(), v)
    }
    #[cfg_attr(
        not(all(test, feature = "s_allocation_json_testonly")),
        allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
    )]
    fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
        batch: &str,
    ) -> Result<ContentRef, CodecError> {
        sc::path::replace_batch_placeholder(self.generic_store(), r, p, batch)
    }
}
impl CodecRead for dyn NodeSource + '_ {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecRead for dyn NodeStore + '_ {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecStore for dyn NodeStore + '_ {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
}
impl CodecRead for sc::MemoryNodes {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecStore for sc::MemoryNodes {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
}
impl CodecRead for crate::content_store::TxStore<'_, '_> {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}
impl CodecStore for crate::content_store::TxStore<'_, '_> {
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
}
impl CodecRead for crate::content_store::ReadStore<'_> {
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
}

enum Backend<'a, 'source> {
    Generic(&'a mut dyn NodeStore),
    Audit(&'a mut HashReader<'source>),
}
/// 字段及Backend私有；不能把任意dyn store和另一份capture拼成一条准入路径。
pub(crate) struct StoreView<'a, 'source> {
    backend: Backend<'a, 'source>,
}
impl<'a, 'source> StoreView<'a, 'source> {
    pub(crate) fn generic(s: &'a mut dyn NodeStore) -> Self {
        Self {
            backend: Backend::Generic(s),
        }
    }
    pub(crate) fn for_audit(s: &'a mut HashReader<'source>) -> Self {
        Self {
            backend: Backend::Audit(s),
        }
    }
    fn store(&mut self) -> &mut dyn NodeStore {
        match &mut self.backend {
            Backend::Generic(s) => &mut **s,
            Backend::Audit(s) => &mut **s,
        }
    }
    fn source(&self) -> &dyn NodeSource {
        match &self.backend {
            Backend::Generic(s) => &**s,
            Backend::Audit(s) => &**s,
        }
    }
    pub(crate) fn owned_codec_active(&self) -> bool {
        match &self.backend {
            Backend::Generic(_) => false,
            Backend::Audit(s) => s.owned_codec_active(),
        }
    }
    pub(crate) fn swap_codec_full_array_role(&mut self, enabled: bool) -> bool {
        match &mut self.backend {
            Backend::Generic(s) => s.audit_full_array_scan(),
            Backend::Audit(s) => s.swap_codec_full_array_role(enabled),
        }
    }
    #[cfg(test)]
    pub(crate) fn verify_public(&mut self, r: &ContentRef) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => sc::verify(&mut **s, r).map(|_| ()),
            Backend::Audit(s) => s.verify_public(r),
        }
    }
    #[allow(
        dead_code,
        reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
    )]
    pub(crate) fn verify_many(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => sc::audit_verify_many(&mut **s, r, mode),
            Backend::Audit(s) => s.verify_many(r, mode),
        }
    }
}
impl CodecRead for StoreView<'_, '_> {
    fn parse_rendered_plain(
        &self,
        bytes: &[u8],
    ) -> Result<super::business_json::PlainJson, String> {
        match &self.backend {
            Backend::Generic(s) => s.parse_rendered_plain(bytes),
            Backend::Audit(s) => s.parse_rendered_plain(bytes),
        }
    }

    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<RefValue>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_array_ref(r),
            Backend::Audit(s) => s.lookup_array_ref(r),
        }
    }
    fn verify_many_ref<'a>(
        &mut self,
        r: impl IntoIterator<Item = sc::RefView<'a>>,
        m: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_ref(r, m),
            Backend::Audit(s) => s.verify_many_ref(r, m),
        }
    }
    fn verify_many_ref_fresh<'a>(
        &mut self,
        r: impl IntoIterator<Item = sc::RefView<'a>>,
        m: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_ref_fresh(r, m),
            Backend::Audit(s) => s.verify_many_ref_fresh(r, m),
        }
    }
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        match &self.backend {
            Backend::Generic(s) => s.allocation_budget(),
            Backend::Audit(s) => s.allocation_budget(),
        }
    }
    fn content_ref_view(&mut self, r: &str) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.content_ref_view(r),
            Backend::Audit(s) => s.content_ref_view(r),
        }
    }
    fn lookup_ref(&mut self, r: sc::RefView<'_>, p: &[PathStep]) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_ref(r, p),
            Backend::Audit(s) => s.lookup_ref(r, p),
        }
    }
    fn lookup_field_ref(&mut self, r: sc::RefView<'_>, key: &str) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_field_ref(r, key),
            Backend::Audit(s) => s.lookup_field_ref(r, key),
        }
    }
    fn lookup_optional_field_ref(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
    ) -> Result<Option<RefValue>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_optional_field_ref(r, key),
            Backend::Audit(s) => s.lookup_optional_field_ref(r, key),
        }
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_fields_ref(r, n),
            Backend::Audit(s) => s.lookup_fields_ref(r, n),
        }
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_subset_ref(r, n),
            Backend::Audit(s) => s.lookup_subset_ref(r, n),
        }
    }
    fn render_ref(&mut self, r: sc::RefView<'_>, o: &mut dyn Write) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.render_ref(r, o),
            Backend::Audit(s) => s.render_ref(r, o),
        }
    }
    fn verify_ref(&mut self, r: sc::RefView<'_>, m: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_ref(r, m),
            Backend::Audit(s) => s.verify_ref(r, m),
        }
    }
    fn new_ref_result(&mut self) -> sc::CodecVec<ContentRef> {
        match &mut self.backend {
            Backend::Generic(s) => s.new_ref_result(),
            Backend::Audit(s) => s.new_ref_result(),
        }
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
    fn content_ref_of(&mut self, root: &str) -> Result<ContentRef, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.content_ref_of(root),
            Backend::Audit(s) => s.content_ref_of(root),
        }
    }
    fn lookup_path(&mut self, r: &ContentRef, p: &[PathStep]) -> Result<ContentRef, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_path(r, p),
            Backend::Audit(s) => s.lookup_path(r, p),
        }
    }
    fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<sc::CodecVec<ContentRef>, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_array_items(r),
            Backend::Audit(s) => s.lookup_array_items(r),
        }
    }
    fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_object_fields(r, n),
            Backend::Audit(s) => s.lookup_object_fields(r, n),
        }
    }
    fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.lookup_object_fields_subset(r, n),
            Backend::Audit(s) => s.lookup_object_fields_subset(r, n),
        }
    }
    fn render_canonical(&mut self, r: &ContentRef, w: &mut dyn Write) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.render_canonical(r, w),
            Backend::Audit(s) => s.render_canonical(r, w),
        }
    }
    fn verify_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_fresh(r, mode),
            Backend::Audit(s) => s.verify_fresh(r, mode),
        }
    }
    fn verify_audit_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_audit_fresh(r, mode),
            Backend::Audit(s) => s.verify_audit_fresh(r, mode),
        }
    }
    fn verify_audit_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_audit_many_fresh(r, mode),
            Backend::Audit(s) => s.verify_audit_many_fresh(r, mode),
        }
    }
    fn verify_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_fresh(r, mode),
            Backend::Audit(s) => s.verify_many_fresh(r, mode),
        }
    }
    fn verify_cached(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_cached(r, mode),
            Backend::Audit(s) => s.verify_cached(r, mode),
        }
    }
    fn verify_many_cached(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.verify_many_cached(r, mode),
            Backend::Audit(s) => s.verify_many_cached(r, mode),
        }
    }
}
impl CodecStore for StoreView<'_, '_> {
    fn replace_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
        batch: &str,
    ) -> Result<RefValue, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.replace_ref(r, p, batch),
            Backend::Audit(s) => s.replace_ref(r, p, batch),
        }
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_value_view(v),
            Backend::Audit(s) => s.write_value_view(v),
        }
    }
    fn write_template_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_template_view(v),
            Backend::Audit(s) => s.write_template_view(v),
        }
    }
    fn generic_store(&mut self) -> &mut dyn NodeStore {
        self
    }
    fn write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_json_view(v),
            Backend::Audit(s) => s.write_json_view(v),
        }
    }
    fn write_value(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_value(v),
            Backend::Audit(s) => s.write_value(v),
        }
    }
    fn write_template(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.write_template(v),
            Backend::Audit(s) => s.write_template(v),
        }
    }
    fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
        batch: &str,
    ) -> Result<ContentRef, CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.replace_batch_placeholder(r, p, batch),
            Backend::Audit(s) => s.replace_batch_placeholder(r, p, batch),
        }
    }
}
impl NodeSource for StoreView<'_, '_> {
    fn checked_read_scoped(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        match &mut self.backend {
            Backend::Generic(s) => s.checked_read_scoped(d),
            Backend::Audit(s) => s.checked_read_scoped(d),
        }
    }

    fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source().codec_work()
    }
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.store().read(d)
    }
    fn checked_read(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, CodecError> {
        self.store().checked_read(d)
    }
    fn before_path_index(&mut self, p: &[sc::path::PathStep], i: u64) -> Result<(), CodecError> {
        self.store().before_path_index(p, i)
    }
    fn audit_full_array_scan(&self) -> bool {
        self.source().audit_full_array_scan()
    }
    fn max_render_bytes(&self) -> Option<u64> {
        self.source().max_render_bytes()
    }
}
impl NodeSink for StoreView<'_, '_> {
    fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.source().codec_work()
    }
    fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), CodecError> {
        self.store().put(d, t, b)
    }
}

// 单独cargo rustc -- --cfg issue1394_closed_owner_refusal实际编译本段，预期拒绝。
// 普通构建不包含本编译反例；它不用stub类型冒充真实边界。
#[cfg(issue1394_closed_owner_refusal)]
mod compile_refusal {
    use super::*;
    fn dynamic_cannot_select_captured_branch(
        source: &mut dyn NodeStore,
        capture: &sc::AuditCapture,
    ) {
        let _ = StoreView::for_audit(source);
        let _ = HashReader::captured(capture, source);
    }
}

/// 内部引用所有者；Owned只有固定inline字段，不补造ContentRef/String。
pub(crate) enum RefValue {
    Plain(NativeValue<ContentRef>),
    Owned(sc::RefOwner),
}
impl RefValue {
    pub fn view(&self) -> sc::RefView<'_> {
        match self {
            Self::Plain(r) => r.into(),
            Self::Owned(r) => r.view(),
        }
    }
    pub fn kind(&self) -> sc::Kind {
        self.view().kind()
    }
    pub fn extent(&self) -> Option<u64> {
        self.view().extent()
    }
    pub fn root(&self) -> &str {
        match self {
            Self::Plain(r) => &r.root,
            Self::Owned(r) => r.root(),
        }
    }
    pub fn digest(&self) -> Result<NodeDigest, CodecError> {
        self.view().digest()
    }
}
impl From<ContentRef> for RefValue {
    fn from(r: ContentRef) -> Self {
        Self::Plain(NativeValue::<ContentRef>::from_unscoped(r))
    }
}
impl From<sc::RefOwner> for RefValue {
    fn from(r: sc::RefOwner) -> Self {
        Self::Owned(r)
    }
}
impl PartialEq for RefValue {
    fn eq(&self, o: &Self) -> bool {
        let a = self.view();
        let b = o.view();
        a.format() == b.format()
            && a.root() == b.root()
            && a.kind() == b.kind()
            && a.extent() == b.extent()
    }
}
impl Eq for RefValue {}
impl std::fmt::Debug for RefValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.view();
        f.debug_struct("ContentRef")
            .field("format", &v.format())
            .field("root", &v.root())
            .field("kind", &v.kind())
            .field("extent", &v.extent())
            .finish()
    }
}
impl serde::Serialize for RefValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let v = self.view();
        let mut st = s.serialize_struct("ContentRef", 4)?;
        st.serialize_field("format", v.format())?;
        st.serialize_field("root", v.root())?;
        st.serialize_field("kind", v.kind().name())?;
        match v.extent() {
            Some(mut n) => {
                let mut digits = [0u8; 20];
                let mut start = digits.len();
                loop {
                    start -= 1;
                    digits[start] = b'0' + (n % 10) as u8;
                    n /= 10;
                    if n == 0 {
                        break;
                    }
                }
                let extent =
                    std::str::from_utf8(&digits[start..]).map_err(serde::ser::Error::custom)?;
                st.serialize_field("extent", extent)?;
            }
            None => st.serialize_field("extent", &None::<&str>)?,
        }
        st.end()
    }
}
#[cfg(test)]
mod ref_value_tests {
    use super::*;

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_tree_removed_entry_keeps_payload_charge_after_tree_and_scope_drop() {
        let budget = crate::v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let entry = budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let scope_baseline = budget.live();
                let admission = NativeAdmission(Some(scope));
                let mut tree = NativeTree::default();
                let key = admission.native_string("entry")?;
                let value = admission.native_string("payload")?;
                tree.insert_admitted(key, (value, 1_u64), &admission)?;
                let before = budget.live();
                tree.get_mut("entry").unwrap().1 = 2;
                assert!(tree.get_mut("missing").is_none());
                assert!(tree.remove_entry("missing").is_none());
                assert_eq!(budget.live(), before);
                let entry = tree.remove_entry("entry").unwrap();
                assert_eq!(entry.1 .1, 2);
                assert!(tree.is_empty());
                assert_eq!(budget.live(), before, "移出键值不退其费用或树高水位费");
                drop(tree);
                assert_eq!(
                    budget.live(),
                    scope_baseline + "entry".len() + "payload".len()
                );
                Ok::<_, CodecError>(entry)
            })
            .unwrap();
        assert_eq!(entry.0.as_str(), "entry");
        assert_eq!(entry.1 .0.as_str(), "payload");
        assert_eq!(budget.live(), baseline + "entry".len() + "payload".len());
        drop(entry);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r9a_ref_outputs_charge_plain_text_and_preserve_values() {
        let built = sc::Built {
            digest: [0xad; 32],
            kind: sc::Kind::Array,
            extent: Some(7),
        };
        let reference = built.content_ref();
        let bytes = reference.format.len() + reference.root.len();
        let mut fixture = sc::MemoryNodes::new();
        let object = sc::write_value(
            &mut fixture,
            &sc::Value::Object(vec![(
                "items".into(),
                sc::Value::Array(vec![sc::Value::Null; 7]),
            )]),
        )
        .unwrap()
        .content_ref();
        let fields = fixture
            .lookup_fields_ref((&object).into(), &["items"])
            .unwrap();
        let reference = fields.get("items").unwrap();
        let reference = NativeValue::<ContentRef>::try_from_view(reference, None).unwrap();
        let built = sc::Built {
            digest: reference.digest().unwrap(),
            kind: reference.kind,
            extent: reference.extent,
        };
        let budget = crate::v3::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let store = NativeAdmission(Some(scope));
                let before = budget.live();
                let value = fields.get_admitted(&store, "items")?.unwrap();
                assert_eq!(value, RefValue::Plain(reference));
                assert_eq!(budget.live() - before, bytes);
                let copied = store.native_ref_clone(&value)?;
                let returned = store.native_built_ref(&built)?;
                assert_eq!(copied, value);
                assert_eq!(returned, value);
                assert!(matches!(returned, RefValue::Plain(_)));
                assert_eq!(budget.live() - before, 3 * bytes);
                assert!(fields.get_admitted(&store, "missing")?.is_none());
                assert_eq!(budget.live() - before, 3 * bytes);
                let formatted = store.native_format(format_args!("{:?}", Some(u64::MAX)))?;
                assert_eq!(formatted.as_str(), "Some(18446744073709551615)");
                assert_eq!(budget.live() - before, 3 * bytes + formatted.len());
                drop((value, copied, returned, formatted));
                assert_eq!(budget.live(), before, "真实Plain载荷析构后归还许可");
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn r9a_optional_fields_preserve_absence_kind_and_scoped_reads() {
        let mut nodes = sc::MemoryNodes::new();
        let mut deep = sc::Value::Null;
        for _ in 0..160 {
            deep = sc::Value::Array(vec![deep]);
        }
        let values = [
            sc::Value::Object(vec![]),
            sc::Value::Object(vec![("deep".into(), deep)]),
            sc::Value::Object(vec![("tb02a".into(), sc::Value::Bool(true))]),
            sc::Value::Object(vec![("tb02a".into(), sc::Value::Bool(false))]),
            sc::Value::Object(vec![(
                "tb02a".into(),
                sc::Value::JsonText(Box::new(sc::JsonValue::Bool(true))),
            )]),
        ];
        let roots: Vec<_> = values
            .iter()
            .map(|v| sc::write_value(&mut nodes, v).unwrap().content_ref())
            .collect();
        let expected = [
            None,
            None,
            Some(sc::Kind::Bool),
            Some(sc::Kind::Bool),
            Some(sc::Kind::JsonText),
        ];
        let capture = sc::AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let ((), stats) = with_native_store_probe(&capture, 8 * 1024 * 1024, |store, _| {
            for (root, kind) in roots.iter().zip(expected) {
                let found = store.lookup_optional_field_ref(root.into(), "tb02a")?;
                assert_eq!(found.as_ref().map(RefValue::kind), kind);
                if let Some(found) = found {
                    assert!(matches!(found, RefValue::Plain(_)));
                    assert_eq!(found, store.lookup_field_ref(root.into(), "tb02a")?);
                }
            }
            assert_eq!(
                store.native_string("scope remains usable")?.as_str(),
                "scope remains usable"
            );
            Ok(())
        });
        assert_eq!((stats.0, stats.2), (0, 0));
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            use sc::capture_allocation::{CaptureSlab, OwnedBudget};
            let budget = OwnedBudget::new(16 * 1024 * 1024);
            let bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
            let mut slab = CaptureSlab::new(nodes.len(), bytes, &budget).unwrap();
            for (d, t, b) in nodes.iter() {
                slab.push(*d, t, b).unwrap();
            }
            let owned =
                sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
            with_owned_store_probe(&owned, &budget, 64, 65536, |store| {
                for (root, kind) in roots.iter().zip(expected) {
                    let found = store
                        .lookup_optional_field_ref(root.into(), "tb02a")
                        .unwrap();
                    assert_eq!(found.as_ref().map(RefValue::kind), kind);
                    if let Some(found) = found {
                        assert!(matches!(found, RefValue::Owned(_)));
                        assert_eq!(found, store.lookup_field_ref(root.into(), "tb02a").unwrap());
                    }
                }
            });
        }
    }

    #[test]
    fn ref_value_transport_matches_public_ref_for_all_kinds_and_extents() {
        for kind in sc::Kind::ALL {
            for extent in [None, Some(0), Some(u64::MAX)] {
                let built = sc::Built {
                    digest: [0xad; 32],
                    kind,
                    extent,
                };
                let public_ref = built.content_ref();
                let expected = public_ref.to_transport_json();
                assert_eq!(expected.as_object().unwrap().len(), 4);
                assert_eq!(
                    serde_json::to_value(RefValue::from(public_ref)).unwrap(),
                    expected,
                    "normal kind={} extent={extent:?}",
                    kind.name(),
                );
                #[cfg(feature = "s_allocation_json_testonly")]
                assert_eq!(
                    serde_json::to_value(RefValue::Owned(built.owned_ref())).unwrap(),
                    expected,
                    "owned kind={} extent={extent:?}",
                    kind.name(),
                );
            }
        }
    }
}
pub(crate) enum RefFields<'a> {
    Plain(NativeRefFields),
    Owned(sc::OwnedRefFields<'a>),
}
impl RefFields<'_> {
    pub fn get_admitted(
        &self,
        s: &(impl CodecRead + ?Sized),
        key: &str,
    ) -> Result<Option<RefValue>, CodecError> {
        match self {
            Self::Plain(f) => f
                .get(key)
                .map(|r| {
                    if r.is_native_admitted() && s.codec_work().is_none() {
                        return Err(CodecError::new(
                            sc::capture_allocation::INCOMPLETE,
                            "Native字段复制需要当前WorkScope",
                        ));
                    }
                    s.native_plain_ref(r.into()).map(RefValue::Plain)
                })
                .transpose(),
            Self::Owned(f) => Ok(f.get(key).copied().map(RefValue::Owned)),
        }
    }
    pub fn get(&self, key: &str) -> Option<sc::RefView<'_>> {
        match self {
            Self::Plain(f) => f.get(key).map(Into::into),
            Self::Owned(f) => f.get(key).map(sc::RefOwner::view),
        }
    }
    #[cfg_attr(
        not(all(test, feature = "s_capture_native_testonly")),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
    )]
    pub fn len(&self) -> usize {
        match self {
            Self::Plain(f) => f.len(),
            Self::Owned(f) => f.len(),
        }
    }
}

/// 仅测试：真实capture/overlay/config进入实际HashReader backend，不制作owned成功token。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(crate) fn with_native_store_probe<T>(
    capture: &sc::AuditCapture,
    limit: usize,
    action: impl FnOnce(&mut StoreView<'_, '_>, &dyn Fn() -> usize) -> Result<T, CodecError>,
) -> (T, (usize, usize, usize, usize)) {
    let budget = super::audit_owner::native::Budget::new(limit, 0).unwrap();
    let result = budget
        .codec_work()
        .unwrap()
        .run(|scope| {
            let mut reader =
                HashReader::captured_for_audit(capture, None)?.with_codec_work(scope, &budget);
            action(&mut StoreView::for_audit(&mut reader), &|| budget.live())
        })
        .unwrap();
    (result, budget.scratch_observation())
}

/// 仅测试：真实capture/overlay/config进入实际HashReader backend，不制作owned成功token。
#[cfg(all(test, feature = "s_allocation_json_testonly"))]
pub(crate) fn with_owned_store_probe<T>(
    capture: &sc::AuditCapture,
    budget: &sc::capture_allocation::OwnedBudget,
    slots: usize,
    bytes: usize,
    action: impl FnOnce(&mut StoreView<'_, '_>) -> T,
) -> T {
    super::history::with_overlay_prototype(
        super::history::OverlayPrototype {
            slots,
            bytes,
            budget: budget.clone(),
            peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
        },
        || {
            let mut reader = super::history::HashReader::captured_for_audit(capture, None)
                .expect("test actual closed capture/overlay initialization");
            action(&mut StoreView::for_audit(&mut reader))
        },
    )
}

impl<'a> From<&'a RefValue> for sc::RefView<'a> {
    fn from(r: &'a RefValue) -> Self {
        r.view()
    }
}
