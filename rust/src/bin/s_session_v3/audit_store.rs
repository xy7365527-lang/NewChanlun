//! #1394 封闭审计读取边界。动态来源没有捕获能力；唯有具体 HashReader 自己选择
//! 与实际读取绑定的不可变 owner。两种路径都用同一业务函数，不传成功 token。
pub(crate) use super::history::HashReader;
use newchan_rust::s_content::{
    self as sc, CodecError, ContentRef, NodeDigest, NodeRead, NodeSink, NodeSource, NodeStore,
};

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

/// 仅bin内的静态codec门；具体捕获owner不可擦除成外来来源。
pub(crate) trait CodecRead: NodeSource {
    fn native_admit(&self, bytes: usize) -> Result<(), CodecError> {
        if let Some(work) = self.codec_work() {
            work.reserve_native(bytes)?.retain();
        }
        Ok(())
    }
    fn native_overflow(&self) -> CodecError {
        self.codec_work().map_or_else(
            || CodecError::new(sc::capture_allocation::LIMIT, "Native容量溢出"),
            |w| w.reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
        )
    }
    fn native_vec_room<T>(&self, values: &mut Vec<T>, additional: usize) -> Result<(), CodecError> {
        if self.codec_work().is_none() {
            return Ok(());
        }
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or_else(|| self.native_overflow())?;
        if needed <= values.capacity() {
            return Ok(());
        }
        let next = needed
            .max(
                values
                    .capacity()
                    .checked_mul(2)
                    .ok_or_else(|| self.native_overflow())?,
            )
            .max(4);
        let bytes = next
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(|| self.native_overflow())?;
        let permit = self.codec_work().unwrap().reserve_native(bytes)?;
        values.try_reserve_exact(next - values.len()).map_err(|_| {
            self.codec_work()
                .unwrap()
                .reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
        })?;
        permit.retain();
        Ok(())
    }
    fn native_codec_room<T>(
        &self,
        values: &mut sc::CodecVec<T>,
        additional: usize,
    ) -> Result<(), CodecError> {
        if self.codec_work().is_none() {
            return Ok(());
        }
        if values.allocation_budget().is_some() {
            return Err(self
                .codec_work()
                .unwrap()
                .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger));
        }
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or_else(|| self.native_overflow())?;
        if needed <= values.capacity() {
            return Ok(());
        }
        let next = needed
            .max(
                values
                    .capacity()
                    .checked_mul(2)
                    .ok_or_else(|| self.native_overflow())?,
            )
            .max(4);
        let bytes = next
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(|| self.native_overflow())?;
        let permit = self.codec_work().unwrap().reserve_native(bytes)?;
        values.try_reserve_exact(next - values.len()).map_err(|_| {
            self.codec_work()
                .unwrap()
                .reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
        })?;
        permit.retain();
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
        let bytes = self.native_hash_bytes::<K, V>(map.len(), map.capacity())?;
        if bytes > 0 {
            let work = self.codec_work().unwrap();
            let permit = work.reserve_native(bytes)?;
            map.try_reserve_plain(1).map_err(|_| {
                work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
            })?;
            permit.retain();
        }
        Ok(())
    }
    fn native_std_map_room<K: Eq + std::hash::Hash, V>(
        &self,
        map: &mut std::collections::HashMap<K, V>,
    ) -> Result<(), CodecError> {
        let bytes = self.native_hash_bytes::<K, V>(map.len(), map.capacity())?;
        if bytes > 0 {
            let work = self.codec_work().unwrap();
            let permit = work.reserve_native(bytes)?;
            map.try_reserve(1).map_err(|_| {
                work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
            })?;
            permit.retain();
        }
        Ok(())
    }
    fn native_tree_room<K, V>(&self, len: usize) -> Result<(), CodecError> {
        if self.codec_work().is_none() {
            return Ok(());
        }
        let depth = usize::BITS as usize - len.leading_zeros() as usize + 2;
        let bytes = std::mem::size_of::<(K, V)>()
            .checked_mul(11)
            .and_then(|n| n.checked_add(16 * std::mem::size_of::<usize>()))
            .and_then(|n| n.checked_mul(depth))
            .ok_or_else(|| self.native_overflow())?;
        self.native_admit(bytes)
    }
    fn native_ref_clone(&self, value: &RefValue) -> Result<RefValue, CodecError> {
        match value {
            RefValue::Plain(r) => self.native_plain_ref(r.into()).map(RefValue::Plain),
            RefValue::Owned(r) => Ok(RefValue::Owned(*r)),
        }
    }
    /// 借用声明只复制文本，不提前核 digest，也不把 Plain 改成 Owned。
    fn native_plain_ref(&self, r: sc::RefView<'_>) -> Result<ContentRef, CodecError> {
        self.native_admit(
            r.format()
                .len()
                .checked_add(r.root().len())
                .ok_or_else(|| self.native_overflow())?,
        )?;
        Ok(ContentRef {
            format: r.format().to_owned(),
            root: r.root().to_owned(),
            kind: r.kind(),
            extent: r.extent(),
        })
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
        // FixedRoot 是栈上文本；两个 String 均在同父账准入之后才构造。
        let root = sc::FixedRoot::new(&b.digest);
        self.native_admit(sc::CONTENT_FORMAT.len() + root.as_str().len())?;
        Ok(RefValue::Plain(ContentRef {
            format: sc::CONTENT_FORMAT.to_owned(),
            root: root.as_str().to_owned(),
            kind: b.kind,
            extent: b.extent,
        }))
    }
    fn native_string(&self, text: &str) -> Result<String, CodecError> {
        let permit = self
            .codec_work()
            .map(|w| w.reserve_native(text.len()))
            .transpose()?;
        let mut value = String::new();
        value.try_reserve_exact(text.len()).map_err(|_| {
            self.codec_work().map_or_else(
                || CodecError::new(sc::capture_allocation::LIMIT, "Native文本分配失败"),
                |w| w.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation),
            )
        })?;
        value.push_str(text);
        if let Some(permit) = permit {
            permit.retain();
        }
        Ok(value)
    }
    fn native_field(&self, key: &str) -> Result<PathStep, CodecError> {
        self.native_string(key).map(PathStep::Field)
    }
    /// 先向栈上计数器格式化，再一次性准入实际文本长度；不构造临时 String。
    fn native_format(&self, args: std::fmt::Arguments<'_>) -> Result<String, CodecError> {
        struct Count(usize);
        impl std::fmt::Write for Count {
            fn write_str(&mut self, value: &str) -> std::fmt::Result {
                self.0 = self.0.checked_add(value.len()).ok_or(std::fmt::Error)?;
                Ok(())
            }
        }
        let mut count = Count(0);
        std::fmt::write(&mut count, args).map_err(|_| self.native_overflow())?;
        let permit = self
            .codec_work()
            .map(|w| w.reserve_native(count.0))
            .transpose()?;
        let mut value = String::new();
        value.try_reserve_exact(count.0).map_err(|_| {
            self.codec_work().map_or_else(
                || CodecError::new(sc::capture_allocation::LIMIT, "Native文本分配失败"),
                |w| w.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation),
            )
        })?;
        struct BoundedText<'a>(&'a mut String, usize);
        impl std::fmt::Write for BoundedText<'_> {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                if text.len() > self.1.saturating_sub(self.0.len()) {
                    return Err(std::fmt::Error);
                }
                self.0.push_str(text);
                Ok(())
            }
        }
        std::fmt::write(&mut BoundedText(&mut value, count.0), args)
            .map_err(|_| self.native_overflow())?;
        if let Some(permit) = permit {
            permit.retain();
        }
        Ok(value)
    }
    fn native_ref_fields_bytes<'a>(
        &self,
        fields: impl IntoIterator<Item = (&'a str, &'a ContentRef)>,
    ) -> Result<(), CodecError> {
        let node =
            11 * std::mem::size_of::<(String, ContentRef)>() + 16 * std::mem::size_of::<usize>();
        for (name, reference) in fields {
            let bytes = node
                .checked_add(name.len())
                .and_then(|n| n.checked_add(reference.format.len()))
                .and_then(|n| n.checked_add(reference.root.len()))
                .ok_or_else(|| self.native_overflow())?;
            self.native_admit(bytes)?;
        }
        Ok(())
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
                let permit = self
                    .scope
                    .reserve_native(self.capture.original(digest).map_or(0, |(_, b)| b.len()))?;
                let value = self.capture.reader().read(digest)?;
                permit.retain();
                Ok(value)
            }
            fn checked_read(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<Option<sc::job::CheckedFrame>, CodecError> {
                self.checked_read_scoped(digest)
                    .map(|f| f.map(sc::job::ScopedFrame::into_frame))
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
    ) -> Result<serde_json::Value, CodecError> {
        fn bytes(value: &serde_json::Value) -> Option<usize> {
            use serde_json::Value;
            match value {
                Value::String(s) => Some(s.len()),
                Value::Array(v) => v.iter().try_fold(
                    v.len().checked_mul(std::mem::size_of::<Value>())?,
                    |n, v| n.checked_add(bytes(v)?),
                ),
                Value::Object(m) => m.iter().try_fold(0usize, |n, (k, v)| {
                    n.checked_add(
                        11 * std::mem::size_of::<(String, Value)>()
                            + 16 * std::mem::size_of::<usize>(),
                    )?
                    .checked_add(k.len())?
                    .checked_add(bytes(v)?)
                }),
                _ => Some(0),
            }
        }
        if self.codec_work().is_some() {
            self.native_admit(bytes(value).ok_or_else(|| self.native_overflow())?)?;
        }
        Ok(value.clone())
    }
    fn parse_rendered_plain(&self, bytes: &[u8]) -> Result<serde_json::Value, String> {
        newchan_rust::session_protocol::strict_json(bytes)
    }

    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<RefValue>, CodecError> {
        let refs = sc::path::lookup_array_items_refview(self.generic_source(), r)?;
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
        self.content_ref_of(root).map(RefValue::Plain)
    }
    fn lookup_ref(&mut self, r: sc::RefView<'_>, p: &[PathStep]) -> Result<RefValue, CodecError> {
        sc::path::lookup_path_refview(self.generic_source(), r, p).map(RefValue::Plain)
    }
    fn lookup_field_ref(&mut self, r: sc::RefView<'_>, key: &str) -> Result<RefValue, CodecError> {
        sc::path::lookup_path_view_refview(self.generic_source(), r, &BorrowedFieldPath(key))
            .map(RefValue::Plain)
    }
    fn lookup_optional_field_ref(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
    ) -> Result<Option<RefValue>, CodecError> {
        sc::path::lookup_optional_field_refview(self.generic_source(), r, key)
            .map(|value| value.map(RefValue::Plain))
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        sc::path::lookup_fields_refview(self.generic_source(), r, names, false)
            .map(RefFields::Plain)
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        names: &[&'a str],
    ) -> Result<RefFields<'a>, CodecError> {
        sc::path::lookup_fields_refview(self.generic_source(), r, names, true).map(RefFields::Plain)
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
    #[cfg_attr(not(all(test, feature = "s_allocation_json_testonly")), allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用"))]
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
    #[cfg_attr(not(feature = "s_capture_native_testonly"), allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 下使用"))]
    fn verify_audit_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), CodecError> {
        sc::audit_verify_many(self.generic_source(), r, mode)
    }
    #[cfg_attr(not(all(test, feature = "s_capture_native_testonly")), allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用"))]
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
    #[cfg_attr(not(all(test, feature = "s_allocation_json_testonly")), allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用"))]
    fn verify_cached(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), CodecError> {
        self.verify_fresh(r, mode)
    }
    #[cfg_attr(not(all(test, feature = "s_allocation_json_testonly")), allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用"))]
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
        sc::path::replace_batch_placeholder_refview(self.generic_store(), r, p, batch)
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
    fn write_value(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        sc::write_value(self.generic_store(), v)
    }
    #[cfg_attr(not(all(test, feature = "s_allocation_json_testonly")), allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用"))]
    fn write_template(&mut self, v: &sc::Value) -> Result<sc::Built, CodecError> {
        sc::write_template(self.generic_store(), v)
    }
    #[cfg_attr(not(all(test, feature = "s_allocation_json_testonly")), allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用"))]
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
    #[allow(dead_code, reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删")]
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
    fn parse_rendered_plain(&self, bytes: &[u8]) -> Result<serde_json::Value, String> {
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
#[derive(Clone)]
pub(crate) enum RefValue {
    Plain(ContentRef),
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
        Self::Plain(r)
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
    fn r9a_ref_outputs_charge_plain_text_and_preserve_values() {
        let built = sc::Built {
            digest: [0xad; 32],
            kind: sc::Kind::Array,
            extent: Some(7),
        };
        let reference = built.content_ref();
        let bytes = reference.format.len() + reference.root.len();
        let fields = RefFields::Plain(sc::CodecFields::from_plain(
            std::collections::BTreeMap::from([("items".into(), reference.clone())]),
        ));
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
                assert_eq!(formatted, "Some(18446744073709551615)");
                assert_eq!(budget.live() - before, 3 * bytes + formatted.len());
                drop((value, copied, returned, formatted));
                assert!(budget.live() > before, "Plain许可保守留到父域结束");
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
                store.native_string("scope remains usable")?,
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
                    serde_json::to_value(RefValue::Plain(public_ref)).unwrap(),
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
    Plain(sc::CodecFields),
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
                .map(|r| s.native_plain_ref(r.into()).map(RefValue::Plain))
                .transpose(),
            Self::Owned(f) => Ok(f.get(key).copied().map(RefValue::Owned)),
        }
    }
    pub fn get(&self, key: &str) -> Option<RefValue> {
        match self {
            Self::Plain(f) => f.get(key).cloned().map(RefValue::Plain),
            Self::Owned(f) => f.get(key).copied().map(RefValue::Owned),
        }
    }
    #[cfg_attr(not(all(test, feature = "s_capture_native_testonly")), allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用"))]
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
