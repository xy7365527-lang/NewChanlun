//! #1419：结构投影的临时值与返回容器。每次扩容先在当前 WorkScope 准入，
//! 文本和 buffer 各自携带原许可；不跨租约保存 reader，不创建 serde 全对象树。
use newchan_rust::s_content::{self as sc, Built, CodecVec, NodeSource, NodeStore, ValueView};
use sc::capture_allocation::native_owned::NativeValue;
use std::{
    borrow::Borrow,
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Text(NativeValue<String>);
impl Text {
    pub fn new(s: &str, nodes: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        NativeValue::<String>::try_from_str(s, nodes.codec_work())
            .map(Self)
            .map_err(sc::ContextError::from)
    }
    pub fn format(
        args: fmt::Arguments<'_>,
        nodes: &dyn NodeSource,
    ) -> Result<Self, sc::ContextError> {
        NativeValue::<String>::try_format(args, nodes.codec_work())
            .map(Self)
            .map_err(sc::ContextError::from)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
impl PartialEq<str> for Text {
    fn eq(&self, rhs: &str) -> bool {
        self.as_str() == rhs
    }
}
impl PartialEq<&str> for Text {
    fn eq(&self, rhs: &&str) -> bool {
        self.as_str() == *rhs
    }
}
impl PartialEq<String> for Text {
    fn eq(&self, rhs: &String) -> bool {
        self.as_str() == rhs
    }
}
impl PartialEq<Text> for String {
    fn eq(&self, rhs: &Text) -> bool {
        self == rhs.as_str()
    }
}
impl serde::Serialize for Text {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

pub(crate) trait CloneIn: Sized {
    fn clone_in(&self, nodes: &dyn NodeSource) -> Result<Self, sc::ContextError>;
}
impl CloneIn for Text {
    fn clone_in(&self, nodes: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Self::new(self.as_str(), nodes)
    }
}
impl CloneIn for Built {
    fn clone_in(&self, _: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Ok(self.clone())
    }
}
impl CloneIn for usize {
    fn clone_in(&self, _: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Ok(*self)
    }
}
impl<T: ?Sized> CloneIn for &T {
    fn clone_in(&self, _: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Ok(*self)
    }
}
impl<A: CloneIn, B: CloneIn> CloneIn for (A, B) {
    fn clone_in(&self, nodes: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Ok((self.0.clone_in(nodes)?, self.1.clone_in(nodes)?))
    }
}

#[derive(Debug)]
pub(crate) struct Vec<T>(CodecVec<T>);
impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self(CodecVec::default())
    }
}
impl<T> Vec<T> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, value: T, nodes: &dyn NodeSource) -> Result<(), sc::ContextError> {
        if let Some(work) = nodes.codec_work() {
            self.0
                .try_reserve_native(work, 1)
                .map_err(sc::ContextError::from)?;
        }
        self.0.try_push(value).map_err(sc::ContextError::from)
    }
    pub fn insert(
        &mut self,
        at: usize,
        value: T,
        nodes: &dyn NodeSource,
    ) -> Result<(), sc::ContextError> {
        if let Some(work) = nodes.codec_work() {
            self.0.try_insert_native(at, value, work)
        } else {
            self.0.try_insert(at, value)
        }
        .map_err(sc::ContextError::from)
    }
    pub fn extend(
        &mut self,
        iter: impl IntoIterator<Item = T>,
        nodes: &dyn NodeSource,
    ) -> Result<(), sc::ContextError> {
        for value in iter {
            self.push(value, nodes)?;
        }
        Ok(())
    }
    pub fn from_iter(
        iter: impl IntoIterator<Item = T>,
        nodes: &dyn NodeSource,
    ) -> Result<Self, sc::ContextError> {
        let mut out = Self::new();
        out.extend(iter, nodes)?;
        Ok(out)
    }
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = Result<T, sc::ContextError>>,
        nodes: &dyn NodeSource,
    ) -> Result<Self, sc::ContextError> {
        let mut out = Self::new();
        for value in iter {
            out.push(value?, nodes)?;
        }
        Ok(out)
    }
    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        let mut write = 0;
        for read in 0..self.len() {
            if write == 0 || self[read] != self[write - 1] {
                self.0.swap(write, read);
                write += 1;
            }
        }
        self.0.truncate(write);
    }
}
impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}
impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}
impl<T: CloneIn> CloneIn for Vec<T> {
    fn clone_in(&self, nodes: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Self::try_from_iter(self.iter().map(|v| v.clone_in(nodes)), nodes)
    }
}
impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = <CodecVec<T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<T: serde::Serialize> serde::Serialize for Vec<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0[..].serialize(s)
    }
}

pub(crate) struct Map<K, V>(Vec<(K, V)>);
impl<K: Ord, V> Map<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.0
            .binary_search_by(|(k, _)| k.borrow().cmp(key))
            .ok()
            .map(|i| &self.0[i].1)
    }
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get(key).is_some()
    }
    pub fn insert(
        &mut self,
        key: K,
        value: V,
        nodes: &dyn NodeSource,
    ) -> Result<Option<V>, sc::ContextError> {
        match self.0.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(i) => Ok(Some(std::mem::replace(&mut self.0[i].1, value))),
            Err(i) => {
                self.0.insert(i, (key, value), nodes)?;
                Ok(None)
            }
        }
    }
    pub fn iter(&self) -> std::slice::Iter<'_, (K, V)> {
        self.0.iter()
    }
}
impl<'a, K, V> IntoIterator for &'a Map<K, V> {
    type Item = &'a (K, V);
    type IntoIter = std::slice::Iter<'a, (K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
pub(crate) struct Set<K>(Map<K, ()>);
impl<K: Ord> Set<K> {
    pub fn new() -> Self {
        Self(Map::new())
    }
    pub fn insert(&mut self, k: K, nodes: &dyn NodeSource) -> Result<bool, sc::ContextError> {
        Ok(self.0.insert(k, (), nodes)?.is_none())
    }
    pub fn contains<Q: Ord + ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.0.contains_key(k)
    }
}

#[derive(Debug)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    String(Text),
    Array(Vec<Value>),
    Object(Vec<(Text, Value)>),
    Edge(sc::NodeDigest),
}
impl CloneIn for Value {
    fn clone_in(&self, nodes: &dyn NodeSource) -> Result<Self, sc::ContextError> {
        Ok(match self {
            Self::Null => Self::Null,
            Self::Bool(v) => Self::Bool(*v),
            Self::Edge(v) => Self::Edge(*v),
            Self::String(v) => Self::String(v.clone_in(nodes)?),
            Self::Array(v) => Self::Array(v.clone_in(nodes)?),
            Self::Object(v) => Self::Object(v.clone_in(nodes)?),
        })
    }
}
pub(crate) fn string(
    v: impl fmt::Display,
    nodes: &dyn NodeSource,
) -> Result<Value, sc::ContextError> {
    Ok(Value::String(Text::format(format_args!("{v}"), nodes)?))
}
pub(crate) fn object<const N: usize>(
    fields: [(&str, Value); N],
    nodes: &dyn NodeSource,
) -> Result<Value, sc::ContextError> {
    let mut out = Vec::new();
    for (k, v) in fields {
        out.push((Text::new(k, nodes)?, v), nodes)?;
    }
    Ok(Value::Object(out))
}
/// 子节点写成内容边，临时视图只借用；与 codec 的规范对象/数组编码共用一核。
pub(crate) fn put(value: &Value, nodes: &mut dyn NodeStore) -> Result<Built, sc::ContextError> {
    let result = match value {
        Value::Null => sc::write_value(nodes, &sc::Value::Null),
        Value::Bool(v) => sc::write_value(nodes, &sc::Value::Bool(*v)),
        Value::String(v) => sc::write_value_view(nodes, ValueView::String(v)),
        Value::Edge(v) => sc::write_value(nodes, &sc::Value::Edge(*v)),
        Value::Array(values) => {
            let mut refs = Vec::new();
            for v in values {
                let b = match v {
                    Value::Edge(d) => *d,
                    _ => put(v, nodes)?.digest,
                };
                refs.push(ValueView::Edge(b), nodes)?;
            }
            sc::write_value_view(nodes, ValueView::Array(&refs))
        }
        Value::Object(values) => {
            let mut refs = Vec::new();
            for (k, v) in values {
                let b = match v {
                    Value::Edge(d) => *d,
                    _ => put(v, nodes)?.digest,
                };
                refs.push((k.as_str(), ValueView::Edge(b)), nodes)?;
            }
            sc::write_value_view(nodes, ValueView::Object(&refs))
        }
    };
    result.map_err(sc::ContextError::from)
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod tests {
    use super::*;
    use crate::v3::audit_owner::native::Budget;
    use sc::capture_allocation::scratch::WorkScope;
    struct Source<'a>(&'a WorkScope);
    impl NodeSource for Source<'_> {
        fn codec_work(&self) -> Option<&WorkScope> {
            Some(self.0)
        }
        fn read(&mut self, _: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            unreachable!("此测试只检查容器和文本 owner")
        }
    }
    #[test]
    fn returned_owner_and_partial_failure_release_parent_ledger() {
        let budget = Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        let values = budget
            .codec_work()
            .unwrap()
            .run(|work| -> Result<_, sc::ContextError> {
                let source = Source(work);
                let mut values = Vec::new();
                values.push(Text::new("segment:1:42", &source)?, &source)?;
                values.insert(0, Text::new("center:1:7", &source)?, &source)?;
                Ok(values)
            })
            .unwrap();
        assert_eq!(values[0].as_str(), "center:1:7");
        assert!(budget.live() > baseline, "返回容器和内嵌文本必须继续持费");
        drop(values);
        assert_eq!(budget.live(), baseline);
        // #1488 v4 L10/L34（同 request_audit 先例）：父账出生先计自身元数据 M，工作域再计其元数据；
        // 48 B 余量放在两者之上，原意“文本准入后扩容必拒”不变。
        let tiny = Budget::new(
            Budget::metadata_bytes()
                + sc::capture_allocation::scratch::CodecWork::metadata_bytes()
                + 48,
            0,
        )
        .unwrap();
        let before = tiny.live();
        let failed = tiny
            .codec_work()
            .unwrap()
            .run(|work| -> Result<(), sc::ContextError> {
                let source = Source(work);
                let mut values = Vec::new();
                values.push(Text::new("admitted-before-buffer", &source)?, &source)?;
                Ok(())
            });
        assert!(failed.is_err());
        assert_eq!(tiny.live(), before, "扩容拒绝必须释放先已准入文本");
    }
    #[test]
    fn foreign_scope_insert_refuses_before_mutation() {
        let parent = Budget::new(1 << 20, 0).unwrap();
        let mut values = parent
            .codec_work()
            .unwrap()
            .run(|work| -> Result<_, sc::ContextError> { Vec::from_iter([7usize], &Source(work)) })
            .unwrap();
        let before = parent.live();
        let foreign = Budget::new(1 << 20, 0).unwrap();
        let foreign_before = foreign.live();
        let result = foreign
            .codec_work()
            .unwrap()
            .run(|work| values.insert(0, 9, &Source(work)));
        assert!(result.is_err());
        assert_eq!(&*values, &[7]);
        assert_eq!(parent.live(), before);
        assert_eq!(foreign.live(), foreign_before);
        drop(values);
        assert_eq!(parent.live(), Budget::metadata_bytes());
    }
}
