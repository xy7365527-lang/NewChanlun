//! #1488 Native/Plain 载荷所有权。原始许可从准入点移动，不跨域深复制或裸移出。
use super::scratch::{NativeReservation, ScratchFailure, WorkScope};
use crate::s_content::{CodecError, ContentRef, RefView};
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
    ops::Deref,
};

pub struct NativeValue<T> {
    value: T,
    // 声明顺序保证 value（含其 allocation）先析构。
    permit: Option<NativeReservation>,
}
impl<T> NativeValue<T> {
    pub(crate) fn from_reserved(value: T, permit: Option<NativeReservation>) -> Self {
        Self { value, permit }
    }
    pub(crate) fn attach_reserved(mut self, permit: Option<NativeReservation>) -> Self {
        debug_assert!(self.permit.is_none());
        self.permit = permit;
        self
    }
    pub fn as_ref(&self) -> &T {
        &self.value
    }
    /// 只说明本值是否持有原 Native 许可，不授予新作用域的扩容或复制资格。
    pub fn is_native_admitted(&self) -> bool {
        self.permit.is_some()
    }
}
impl<T> AsRef<T> for NativeValue<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}
impl<T> Deref for NativeValue<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for NativeValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
impl<T: PartialEq> PartialEq for NativeValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T: Eq> Eq for NativeValue<T> {}
impl<T: PartialOrd> PartialOrd for NativeValue<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}
impl<T: Ord> Ord for NativeValue<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}
impl<T: Hash> Hash for NativeValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}
impl<T: serde::Serialize> serde::Serialize for NativeValue<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
impl Borrow<str> for NativeValue<String> {
    fn borrow(&self) -> &str {
        &self.value
    }
}
impl AsRef<str> for NativeValue<String> {
    fn as_ref(&self) -> &str {
        &self.value
    }
}
fn failure(work: Option<&WorkScope>, kind: ScratchFailure) -> CodecError {
    work.map_or_else(super::work::allocation_error, |w| w.reject(kind))
}
fn copy_string(text: &str, work: Option<&WorkScope>) -> Result<String, CodecError> {
    let mut value = String::new();
    value
        .try_reserve_exact(text.len())
        .map_err(|_| failure(work, ScratchFailure::Allocation))?;
    value.push_str(text);
    Ok(value)
}
impl NativeValue<String> {
    /// 接收旧无作用域 API 已拥有的字符串；不分配，也不声称已准入。
    pub fn from_unscoped(value: String) -> Self {
        Self::from_reserved(value, None)
    }
    pub fn try_from_str(text: &str, work: Option<&WorkScope>) -> Result<Self, CodecError> {
        let permit = work.map(|w| w.reserve_native(text.len())).transpose()?;
        let value = copy_string(text, work)?;
        Ok(Self::from_reserved(value, permit))
    }
    pub fn try_clone_admitted(&self, work: &WorkScope) -> Result<Self, CodecError> {
        Self::try_from_str(&self.value, Some(work))
    }
}
impl NativeValue<ContentRef> {
    /// 接收旧无作用域 transport 值，不核验其 format/root，不新增预算资格。
    pub fn from_unscoped(value: ContentRef) -> Self {
        Self::from_reserved(value, None)
    }
    /// Built 已有 digest/kind/extent；根文本先在栈上格式化，再准入两段实际文本。
    /// 不依赖 testonly feature 的 Built::owned_ref，也不先构造未计费 ContentRef。
    pub fn try_from_built(
        built: &crate::s_content::Built,
        work: Option<&WorkScope>,
    ) -> Result<Self, CodecError> {
        let root = crate::s_content::FixedRoot::new(&built.digest);
        let bytes = crate::s_content::CONTENT_FORMAT
            .len()
            .checked_add(root.as_str().len())
            .ok_or_else(|| failure(work, ScratchFailure::Budget))?;
        let permit = work.map(|w| w.reserve_native(bytes)).transpose()?;
        let value = ContentRef {
            format: copy_string(crate::s_content::CONTENT_FORMAT, work)?,
            root: copy_string(root.as_str(), work)?,
            kind: built.kind,
            extent: built.extent,
        };
        Ok(Self::from_reserved(value, permit))
    }
    /// 复制 transport 字段，不调用 digest；坏 format/root 仍由原核决定错误顺序。
    pub fn try_from_view(view: RefView<'_>, work: Option<&WorkScope>) -> Result<Self, CodecError> {
        let bytes = view
            .format()
            .len()
            .checked_add(view.root().len())
            .ok_or_else(|| failure(work, ScratchFailure::Budget))?;
        let permit = work.map(|w| w.reserve_native(bytes)).transpose()?;
        let value = ContentRef {
            format: copy_string(view.format(), work)?,
            root: copy_string(view.root(), work)?,
            kind: view.kind(),
            extent: view.extent(),
        };
        Ok(Self::from_reserved(value, permit))
    }
    pub fn try_clone_admitted(&self, work: &WorkScope) -> Result<Self, CodecError> {
        Self::try_from_view((&self.value).into(), Some(work))
    }
}

/// 同核字段结果；键、引用、结果 buffer 各自持有原许可，可跨调用只读。
#[derive(Debug)]
pub struct NativeRefFields {
    entries: super::work::CodecVec<(NativeValue<String>, NativeValue<ContentRef>)>,
}
impl Default for NativeRefFields {
    fn default() -> Self {
        Self {
            entries: super::work::CodecVec::default(),
        }
    }
}
impl NativeRefFields {
    /// 借用来源不转移旧许可：新键、引用、buffer 都在当前域先准入再复制。
    /// 输入可任意排列，输出按键规范序；重复名称不以 map 覆盖掩盖。
    pub fn try_from_views<'a>(
        views: impl IntoIterator<Item = (&'a str, RefView<'a>)>,
        work: Option<&WorkScope>,
    ) -> Result<Self, CodecError> {
        let mut result = Self::default();
        for (name, reference) in views {
            if let Some(work) = work {
                result.entries.try_reserve_native(work, 1)?;
            }
            let key = NativeValue::<String>::try_from_str(name, work)?;
            let reference = NativeValue::<ContentRef>::try_from_view(reference, work)?;
            result.entries.try_push((key, reference))?;
        }
        // slice::sort_unstable_by 原地，不为排序再分配未准入工作块。
        result
            .entries
            .sort_unstable_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));
        if result
            .entries
            .windows(2)
            .any(|pair| pair[0].0.as_str() == pair[1].0.as_str())
        {
            return Err(CodecError::static_detail(
                crate::s_content::code::DUPLICATE_KEY,
                "字段结果含重复键",
            ));
        }
        Ok(result)
    }
    pub fn try_clone_admitted(&self, work: &WorkScope) -> Result<Self, CodecError> {
        Self::try_from_views(
            self.iter()
                .map(|(name, reference)| (name, reference.into())),
            Some(work),
        )
    }
    pub fn get(&self, key: &str) -> Option<&NativeValue<ContentRef>> {
        self.entries
            .binary_search_by(|(name, _)| name.as_str().cmp(key))
            .ok()
            .map(|at| &self.entries[at].1)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &NativeValue<ContentRef>)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value))
    }
    pub(crate) fn append_reserved(
        &mut self,
        key: &str,
        reference: NativeValue<ContentRef>,
        permit: Option<NativeReservation>,
        work: Option<&super::work::WorkBudget>,
    ) -> Result<(), CodecError> {
        if self
            .entries
            .last()
            .is_some_and(|(previous, _)| previous.as_str() >= key)
        {
            return Err(CodecError::static_detail(
                crate::s_content::code::OBJECT_KEY_ORDER,
                "codec字段结果追加必须严格既有canonical序",
            ));
        }
        let mut name = String::new();
        name.try_reserve_exact(key.len()).map_err(|_| {
            work.map_or_else(super::work::allocation_error, |w| {
                w.native_failure(ScratchFailure::Allocation)
            })
        })?;
        name.push_str(key);
        let name = NativeValue::from_reserved(name, permit);
        if let Some(work @ super::work::WorkBudget::Scratch(_)) = work {
            self.entries.reserve_native_work(work, 1)?;
        }
        self.entries.try_push((name, reference))
    }
}

impl NativeValue<String> {
    /// 栈上计长度，再先准入、后格式化；格式化长度漂移也不允许暗扩容。
    pub fn try_format(
        args: std::fmt::Arguments<'_>,
        work: Option<&WorkScope>,
    ) -> Result<Self, CodecError> {
        struct Count(usize);
        impl std::fmt::Write for Count {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.0 = self.0.checked_add(s.len()).ok_or(std::fmt::Error)?;
                Ok(())
            }
        }
        let mut count = Count(0);
        std::fmt::write(&mut count, args).map_err(|_| failure(work, ScratchFailure::Budget))?;
        let permit = work.map(|w| w.reserve_native(count.0)).transpose()?;
        let mut value = String::new();
        value
            .try_reserve_exact(count.0)
            .map_err(|_| failure(work, ScratchFailure::Allocation))?;
        struct Bounded<'a>(&'a mut String, usize);
        impl std::fmt::Write for Bounded<'_> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                if s.len() > self.1.saturating_sub(self.0.len()) {
                    return Err(std::fmt::Error);
                }
                self.0.push_str(s);
                Ok(())
            }
        }
        std::fmt::write(&mut Bounded(&mut value, count.0), args)
            .map_err(|_| failure(work, ScratchFailure::Budget))?;
        Ok(Self::from_reserved(value, permit))
    }
}
impl NativeValue<serde_json::Value> {
    /// 不暴露可拆子树的 &mut Value；需独立子树时重新在当前域准入复制。
    pub fn try_clone_from(
        value: &serde_json::Value,
        work: Option<&WorkScope>,
    ) -> Result<Self, CodecError> {
        fn bytes(value: &serde_json::Value) -> Option<usize> {
            use serde_json::Value;
            match value {
                Value::String(s) => Some(s.len()),
                Value::Array(a) => a.iter().try_fold(
                    a.len().checked_mul(std::mem::size_of::<Value>())?,
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
        let bytes = bytes(value).ok_or_else(|| failure(work, ScratchFailure::Budget))?;
        let permit = work.map(|w| w.reserve_native(bytes)).transpose()?;
        Ok(Self::from_reserved(value.clone(), permit))
    }
    pub fn try_clone_admitted(&self, work: &WorkScope) -> Result<Self, CodecError> {
        Self::try_clone_from(&self.value, Some(work))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s_content::capture_allocation::{
        audit_json::BusinessMap,
        scratch::{CodecWork, ScratchCharge, ScratchParent},
    };
    use crate::s_content::{CodecVec, Kind};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Counts {
        live: usize,
        native: usize,
        peak: usize,
        broken: bool,
    }
    struct Parent {
        limit: usize,
        counts: Mutex<Counts>,
    }
    impl Parent {
        fn new(limit: usize) -> Arc<Self> {
            Arc::new(Self {
                limit,
                counts: Mutex::new(Counts::default()),
            })
        }
        fn live(&self) -> usize {
            self.counts.lock().unwrap().live
        }
        fn native(&self) -> usize {
            self.counts.lock().unwrap().native
        }
        fn peak(&self) -> usize {
            self.counts.lock().unwrap().peak
        }
    }
    impl ScratchParent for Parent {
        fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<(), ScratchFailure> {
            let mut counts = self.counts.lock().unwrap();
            let next = counts
                .live
                .checked_add(bytes)
                .ok_or(ScratchFailure::Budget)?;
            if next > self.limit {
                return Err(ScratchFailure::Budget);
            }
            counts.live = next;
            if matches!(kind, ScratchCharge::Native) {
                counts.native += bytes;
            }
            counts.peak = counts.peak.max(next);
            Ok(())
        }
        fn release(&self, bytes: usize, kind: ScratchCharge) -> Result<(), ScratchFailure> {
            let mut counts = self.counts.lock().unwrap();
            counts.live = counts
                .live
                .checked_sub(bytes)
                .ok_or(ScratchFailure::Ledger)?;
            if matches!(kind, ScratchCharge::Native) {
                counts.native = counts
                    .native
                    .checked_sub(bytes)
                    .ok_or(ScratchFailure::Ledger)?;
            }
            Ok(())
        }
        fn health(&self) -> Result<(), ScratchFailure> {
            if self.counts.lock().unwrap().broken {
                Err(ScratchFailure::Ledger)
            } else {
                Ok(())
            }
        }
        fn invalidate(&self) {
            self.counts.lock().unwrap().broken = true;
        }
    }
    fn run<T>(parent: &Arc<Parent>, f: impl FnOnce(&WorkScope) -> Result<T, CodecError>) -> T {
        CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(f)
            .unwrap()
    }

    #[test]
    fn native_payloads_outlive_scope_and_repeated_drop_returns_parent_to_zero() {
        let parent = Parent::new(1 << 20);
        for _ in 0..4 {
            let text = run(&parent, |work| {
                let text = NativeValue::<String>::try_from_str("owned native text", Some(work))?;
                assert_eq!(work.snapshot().live, 0);
                Ok(text)
            });
            assert_eq!(parent.native(), text.len());
            let copied = run(&parent, |work| text.try_clone_admitted(work));
            assert_eq!(parent.native(), 2 * text.len());
            drop(text);
            assert_eq!(copied.as_str(), "owned native text");
            drop(copied);
            assert_eq!(parent.live(), 0);
            parent.health().unwrap();
        }
    }

    #[test]
    fn native_vec_into_iter_keeps_buffer_and_moved_element_permits_independent() {
        let parent = Parent::new(1 << 20);
        let values = run(&parent, |work| {
            let mut values = CodecVec::default();
            values.try_reserve_native(work, 2)?;
            values.try_push(NativeValue::<String>::try_from_str("first", Some(work))?)?;
            values.try_push(NativeValue::<String>::try_from_str("second", Some(work))?)?;
            Ok(values)
        });
        let live = parent.live();
        let mut iter = values.into_iter();
        let first = iter.next().unwrap();
        assert_eq!(parent.live(), live);
        drop(iter); // second + original buffer are actually gone; first remains live.
        assert_eq!(parent.native(), first.len());
        drop(first);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn native_vec_growth_releases_old_block_only_after_new_block_exists() {
        let parent = Parent::new(1 << 20);
        let mut values = run(&parent, |work| {
            let mut values = CodecVec::default();
            values.try_reserve_native(work, 4)?;
            for n in 0..4u64 {
                values.try_push(n)?;
            }
            Ok(values)
        });
        let old = parent.native();
        run(&parent, |work| values.try_reserve_native(work, 5));
        assert_eq!(&*values, &[0, 1, 2, 3]);
        let current = values.capacity() * std::mem::size_of::<u64>();
        assert_eq!(parent.native(), current);
        assert!(parent.peak() >= old + current + CodecWork::metadata_bytes());
        values.clear();
        assert_eq!(parent.native(), current); // clear does not free the buffer.
        drop(values);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn native_vec_failed_growth_preserves_prior_contents_and_permit() {
        let parent = Parent::new(CodecWork::metadata_bytes() + 64);
        let mut values = run(&parent, |work| {
            let mut values = CodecVec::default();
            values.try_reserve_native(work, 4)?;
            for n in 0..4u64 {
                values.try_push(n)?;
            }
            Ok(values)
        });
        let prior = parent.native();
        let result = CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| values.try_reserve_native(work, 5));
        assert!(result.is_err());
        assert_eq!(&*values, &[0, 1, 2, 3]);
        assert_eq!(parent.native(), prior);
        assert!(values.try_push(4).is_err()); // capacity growth requires a new active scope.
        drop(values);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn native_map_growth_remove_and_drop_follow_real_owners() {
        let parent = Parent::new(1 << 20);
        let mut map = run(&parent, |work| {
            let mut map = BusinessMap::plain();
            for n in 0..20u64 {
                map.try_reserve_native(work, 1)?;
                map.insert(
                    NativeValue::<String>::try_format(format_args!("key-{n}"), Some(work))?,
                    n,
                )?;
            }
            Ok(map)
        });
        assert_eq!(map.get("key-12"), Some(&12));
        let before = parent.native();
        assert_eq!(map.remove("key-12"), Some(12));
        assert_eq!(parent.native(), before - "key-12".len());
        drop(map);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn native_map_failed_growth_does_not_drain_existing_map() {
        // First 4-bucket bound fits; old + second 8-bucket bound does not.
        let parent = Parent::new(CodecWork::metadata_bytes() + 500);
        let mut map = run(&parent, |work| {
            let mut map = BusinessMap::plain();
            map.try_reserve_native(work, 3)?;
            for n in 0..3u64 {
                map.insert(n, n + 10)?;
            }
            Ok(map)
        });
        let prior = parent.native();
        assert!(CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| map.try_reserve_native(work, 1))
            .is_err());
        assert_eq!(map.get(&1), Some(&11));
        assert_eq!(map.len(), 3);
        assert_eq!(parent.native(), prior);
        drop(map);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn native_reference_copy_does_not_validate_transport_early() {
        let parent = Parent::new(1 << 20);
        let bad = ContentRef {
            format: "bad-format".into(),
            root: "not-a-root".into(),
            kind: Kind::Array,
            extent: Some(0),
        };
        let copy = run(&parent, |work| {
            NativeValue::<ContentRef>::try_from_view((&bad).into(), Some(work))
        });
        assert_eq!(copy.as_ref(), &bad);
        assert_eq!(copy.digest(), bad.digest());
        assert_eq!(parent.native(), bad.format.len() + bad.root.len());
        drop(copy);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn native_payload_destructor_runs_before_its_permit_is_released() {
        struct Observe(Arc<Parent>);
        impl Drop for Observe {
            fn drop(&mut self) {
                assert_eq!(self.0.native(), 17);
            }
        }
        let parent = Parent::new(1 << 20);
        let value = run(&parent, |work| {
            Ok(NativeValue::from_reserved(
                Observe(parent.clone()),
                Some(work.reserve_native(17)?),
            ))
        });
        drop(value);
        assert_eq!(parent.live(), 0);
    }
    #[test]
    fn typed_path_array_and_fields_match_legacy_kernel_and_release_all_results() {
        use crate::s_content::{
            job::{CheckedFrame, ScopedFrame},
            path, write_value, MemoryNodes, NodeDigest, NodeRead, NodeSource, Value,
        };
        struct Source<'a> {
            nodes: &'a mut MemoryNodes,
            work: &'a WorkScope,
        }
        impl NodeSource for Source<'_> {
            fn codec_work(&self) -> Option<&WorkScope> {
                Some(self.work)
            }
            fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.nodes.read(digest)
            }
            fn checked_read_scoped(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<Option<ScopedFrame>, CodecError> {
                let Some((tag, body)) = self.nodes.get(digest) else {
                    return Ok(None);
                };
                let permit = self.work.reserve_native(body.len())?;
                Ok(Some(ScopedFrame::new(
                    CheckedFrame::checked(digest, tag, body.to_vec())?,
                    Some(permit),
                )))
            }
        }
        let parent = Parent::new(1 << 20);
        let mut nodes = MemoryNodes::new();
        let root = write_value(
            &mut nodes,
            &Value::Object(vec![
                (
                    "a".into(),
                    Value::Array(vec![Value::Null, Value::Bool(true)]),
                ),
                ("b".into(), Value::Null),
            ]),
        )
        .unwrap()
        .content_ref();
        let expected_fields =
            path::lookup_fields_refview(&mut nodes, (&root).into(), &["a", "b"], false).unwrap();
        let expected_array =
            path::lookup_array_items_refview(&mut nodes, expected_fields.get("a").unwrap().into())
                .unwrap();
        let (fields, array) = run(&parent, |work| {
            let mut source = Source {
                nodes: &mut nodes,
                work,
            };
            let fields =
                path::lookup_fields_native(&mut source, (&root).into(), &["a", "b"], false)?;
            for (name, reference) in fields.iter() {
                assert_eq!(reference.as_ref(), expected_fields.get(name).unwrap());
            }
            let array =
                path::lookup_array_items_native(&mut source, fields.get("a").unwrap().into())?;
            assert_eq!(array.len(), expected_array.len());
            for (actual, expected) in array.iter().zip(&expected_array) {
                assert_eq!(actual.as_ref(), expected);
            }
            Ok((fields, array))
        });
        assert!(parent.native() > 0);
        drop(fields);
        assert_eq!(array[1].kind, Kind::Bool);
        drop(array);
        assert_eq!(parent.live(), 0);
        for bad in [
            ContentRef {
                format: "bad".into(),
                ..root.clone()
            },
            ContentRef {
                root: "bad".into(),
                ..root.clone()
            },
        ] {
            let old = path::lookup_path_refview(&mut nodes, (&bad).into(), &[]).unwrap_err();
            let new = CodecWork::new(parent.clone(), parent.limit)
                .unwrap()
                .run(|work| {
                    path::lookup_path_native(
                        &mut Source {
                            nodes: &mut nodes,
                            work,
                        },
                        (&bad).into(),
                        &[],
                    )
                });
            match new {
                Err(crate::s_content::capture_allocation::scratch::WorkError::Operation(error)) => {
                    assert_eq!(error, old)
                }
                _ => panic!("transport error classification changed"),
            }
            assert_eq!(parent.live(), 0);
        }
        let missing = [path::PathStep::Field("missing".into())];
        let old = path::lookup_path_refview(&mut nodes, (&root).into(), &missing).unwrap_err();
        let new = CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| {
                path::lookup_path_native(
                    &mut Source {
                        nodes: &mut nodes,
                        work,
                    },
                    (&root).into(),
                    &missing,
                )
            });
        match new {
            Err(crate::s_content::capture_allocation::scratch::WorkError::Operation(error)) => {
                assert_eq!(error, old)
            }
            _ => panic!("missing field changed"),
        }
        assert_eq!(parent.live(), 0);
    }
    #[test]
    fn native_built_reference_matches_public_four_fields_without_testonly_feature() {
        use crate::s_content::{Built, CONTENT_FORMAT};
        let parent = Parent::new(1 << 20);
        for kind in Kind::ALL {
            let built = Built {
                digest: [0xa5; 32],
                kind,
                extent: kind.extent_is_byte_or_count().then_some(7),
            };
            let expected = built.content_ref();
            let plain = NativeValue::<ContentRef>::try_from_built(&built, None).unwrap();
            assert_eq!(plain.as_ref(), &expected);
            assert!(!plain.is_native_admitted());
            let actual = run(&parent, |work| {
                NativeValue::<ContentRef>::try_from_built(&built, Some(work))
            });
            assert_eq!(actual.as_ref(), &expected);
            assert!(actual.is_native_admitted());
            assert_eq!(parent.native(), CONTENT_FORMAT.len() + expected.root.len());
            drop(actual);
            assert_eq!(parent.live(), 0);
        }
    }

    #[test]
    fn native_content_ref_of_preserves_root_source_header_order_and_original_permit() {
        use crate::s_content::{
            self as sc,
            job::{CheckedFrame, ScopedFrame},
            MemoryNodes, NodeDigest, NodeRead, NodeSource,
        };
        struct Source<'a> {
            nodes: &'a mut MemoryNodes,
            work: Option<&'a WorkScope>,
            reads: usize,
            fail: bool,
        }
        impl NodeSource for Source<'_> {
            fn codec_work(&self) -> Option<&WorkScope> {
                self.work
            }
            fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.reads += 1;
                if self.fail {
                    return Err(CodecError::static_detail(
                        sc::code::SOURCE,
                        "native-reference source sentinel",
                    ));
                }
                self.nodes.read(digest)
            }
            fn checked_read_scoped(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<Option<ScopedFrame>, CodecError> {
                self.reads += 1;
                if self.fail {
                    return Err(CodecError::static_detail(
                        sc::code::SOURCE,
                        "native-reference source sentinel",
                    ));
                }
                let Some((tag, body)) = self.nodes.get(digest) else {
                    return Ok(None);
                };
                let permit = self
                    .work
                    .map(|w| w.reserve_native(body.len()))
                    .transpose()?;
                Ok(Some(ScopedFrame::new(
                    CheckedFrame::checked(digest, tag, body.to_vec())?,
                    permit,
                )))
            }
        }
        let parent = Parent::new(1 << 20);
        let mut nodes = MemoryNodes::new();
        let root = sc::write_value(&mut nodes, &sc::Value::Bool(true))
            .unwrap()
            .root();
        let expected = sc::content_ref_of(&mut nodes, &root).unwrap();
        let actual = run(&parent, |work| {
            let mut source = Source {
                nodes: &mut nodes,
                work: Some(work),
                reads: 0,
                fail: false,
            };
            let result = sc::path::content_ref_of_native(&mut source, &root)?;
            assert_eq!(source.reads, 1);
            Ok(result)
        });
        assert_eq!(actual.as_ref(), &expected);
        assert_eq!(parent.native(), expected.format.len() + expected.root.len());
        drop(actual);
        assert_eq!(parent.live(), 0);
        // 坏根必须先于原件读取/来源错误；原核的错误文本也不变化。
        for bad in [
            "missing-prefix",
            "sc1-00",
            "sc1-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            let mut old_source = Source {
                nodes: &mut nodes,
                work: None,
                reads: 0,
                fail: true,
            };
            let old = sc::content_ref_of(&mut old_source, bad).unwrap_err();
            assert_eq!(old_source.reads, 0);
            let actual = CodecWork::new(parent.clone(), parent.limit)
                .unwrap()
                .run(|work| {
                    let mut source = Source {
                        nodes: &mut nodes,
                        work: Some(work),
                        reads: 0,
                        fail: true,
                    };
                    let result = sc::path::content_ref_of_native(&mut source, bad);
                    assert_eq!(source.reads, 0);
                    result
                });
            match actual {
                Err(sc::capture_allocation::scratch::WorkError::Operation(error)) => {
                    assert_eq!(error, old)
                }
                _ => panic!("bad-root precedence changed"),
            }
            assert_eq!(parent.live(), 0);
        }
        let mut old_source = Source {
            nodes: &mut nodes,
            work: None,
            reads: 0,
            fail: true,
        };
        let old = sc::content_ref_of(&mut old_source, &root).unwrap_err();
        assert_eq!(old_source.reads, 1);
        let actual = CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| {
                let mut source = Source {
                    nodes: &mut nodes,
                    work: Some(work),
                    reads: 0,
                    fail: true,
                };
                let result = sc::path::content_ref_of_native(&mut source, &root);
                assert_eq!(source.reads, 1);
                result
            });
        match actual {
            Err(sc::capture_allocation::scratch::WorkError::Operation(error)) => {
                assert_eq!(error, old)
            }
            _ => panic!("source error precedence changed"),
        }
        assert_eq!(parent.live(), 0);
        // 真 F 相符但不是 Value role 的头，仍由同 reference_header 拒绝。
        let digest = sc::frame(0xff, &[]);
        nodes.insert_unchecked(digest, 0xff, Vec::new());
        let bad_header = sc::FixedRoot::new(&digest);
        let old = sc::content_ref_of(&mut nodes, bad_header.as_str()).unwrap_err();
        let actual = CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| {
                sc::path::content_ref_of_native(
                    &mut Source {
                        nodes: &mut nodes,
                        work: Some(work),
                        reads: 0,
                        fail: false,
                    },
                    bad_header.as_str(),
                )
            });
        match actual {
            Err(sc::capture_allocation::scratch::WorkError::Operation(error)) => {
                assert_eq!(error, old)
            }
            _ => panic!("header error precedence changed"),
        }
        assert_eq!(parent.live(), 0);
    }
    #[test]
    fn native_text_consumes_original_buffer_and_token_without_copy_or_double_charge() {
        let parent = Parent::new(1 << 20);
        let bytes = run(&parent, |work| {
            let mut bytes = CodecVec::default();
            bytes.try_reserve_native(work, 32)?;
            bytes.try_extend_bytes("原buffer".as_bytes())?;
            Ok(bytes)
        });
        let pointer = bytes.as_ptr();
        let capacity = bytes.capacity();
        let charged = parent.native();
        let text = bytes.try_into_native_text().unwrap();
        assert_eq!(text.as_ptr(), pointer);
        assert_eq!(text.capacity(), capacity);
        assert_eq!(text.as_str(), "原buffer");
        assert_eq!(parent.native(), charged);
        assert!(text.is_native_admitted());
        drop(text);
        assert_eq!(parent.live(), 0);
        let invalid = run(&parent, |work| {
            let mut bytes = CodecVec::default();
            bytes.try_reserve_native(work, 4)?;
            bytes.try_extend_bytes(&[0xff, 0xfe])?;
            Ok(bytes)
        });
        assert!(invalid.try_into_native_text().is_err());
        assert_eq!(parent.live(), 0);
        let unscoped = CodecVec::from_plain(b"plain".to_vec())
            .try_into_native_text()
            .unwrap();
        assert_eq!(unscoped.as_str(), "plain");
        assert!(!unscoped.is_native_admitted());
    }

    #[test]
    fn native_field_views_sort_clone_and_subset_own_independent_permits() {
        use crate::s_content::Built;
        let parent = Parent::new(1 << 20);
        let first = Built {
            digest: [1; 32],
            kind: Kind::Null,
            extent: None,
        }
        .content_ref();
        let second = Built {
            digest: [2; 32],
            kind: Kind::Bool,
            extent: None,
        }
        .content_ref();
        let fields = run(&parent, |work| {
            NativeRefFields::try_from_views(
                [("b", (&second).into()), ("a", (&first).into())],
                Some(work),
            )
        });
        assert_eq!(
            fields.iter().map(|(name, _)| name).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(fields.get("a").unwrap().as_ref(), &first);
        let charged = parent.native();
        let cloned = run(&parent, |work| fields.try_clone_admitted(work));
        assert_eq!(parent.native(), 2 * charged);
        drop(cloned);
        assert_eq!(parent.native(), charged);
        let subset = run(&parent, |work| {
            NativeRefFields::try_from_views(
                fields
                    .iter()
                    .filter(|(name, _)| *name == "b")
                    .map(|(name, value)| (name, value.into())),
                Some(work),
            )
        });
        assert_eq!(subset.len(), 1);
        drop(fields);
        assert_eq!(subset.get("b").unwrap().as_ref(), &second);
        drop(subset);
        assert_eq!(parent.live(), 0);
        let duplicate = CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| {
                NativeRefFields::try_from_views(
                    [("a", (&first).into()), ("a", (&second).into())],
                    Some(work),
                )
            });
        assert!(duplicate.is_err());
        assert_eq!(parent.live(), 0);
    }
}
