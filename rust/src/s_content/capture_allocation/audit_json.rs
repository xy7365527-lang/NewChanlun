//! #1394 非 default 实验业务 JSON。节点、边、键、字符串、解析/编码栈与输出共用实际 Layout 账本。
//! arena 内只存偏移；move 不退账，clone 复制全部 arena 且旧新并存。无 std Value 深复制兜底。
use super::work::{CodecVec, LedgerAllocator};
use super::OwnedBudget;
use crate::s_content::CodecError;
use serde_json::{Number, Value};
use std::collections::hash_map::RandomState;
use std::io::Write;

#[cfg(feature = "s_allocation_json_testonly")]
mod number;
const SCHEMA: &str = "SchemaUnsupported";
fn invalid(detail: &'static str) -> CodecError {
    CodecError::static_detail(SCHEMA, detail)
}
#[derive(Clone, Copy, Debug, Default)]
struct Span {
    start: usize,
    len: usize,
}
#[derive(Clone, Debug)]
enum Node {
    Null,
    Bool(bool),
    Number(Number),
    String(Span),
    Array {
        first: Option<usize>,
        last: Option<usize>,
        len: usize,
    },
    Object {
        first: Option<usize>,
        last: Option<usize>,
        len: usize,
    },
}
#[derive(Clone, Copy, Debug)]
struct Edge {
    key: Span,
    value: usize,
    next: Option<usize>,
}
type ArrayIndex = hashbrown::HashMap<(usize, usize), usize, RandomState, LedgerAllocator>;
pub struct OwnedJson {
    nodes: CodecVec<Node>,
    edges: CodecVec<Edge>,
    text: CodecVec<u8>,
    root: usize,
    array_index: ArrayIndex,
    budget: OwnedBudget,
}
impl OwnedJson {
    fn empty(budget: &OwnedBudget) -> Self {
        Self {
            nodes: CodecVec::new(Some(budget)),
            edges: CodecVec::new(Some(budget)),
            text: CodecVec::new(Some(budget)),
            root: 0,
            array_index: ArrayIndex::with_hasher_in(
                RandomState::new(),
                LedgerAllocator::new_tag(budget, "JSON array indexed edges"),
            ),
            budget: budget.clone(),
        }
    }
    fn add_node(&mut self, node: Node) -> Result<usize, CodecError> {
        let id = self.nodes.len();
        self.nodes.try_push(node)?;
        Ok(id)
    }
    fn text(&self, s: Span) -> &str {
        std::str::from_utf8(&self.text[s.start..s.start + s.len]).expect("validated arena UTF8")
    }
    fn copy_text(&mut self, s: &str) -> Result<Span, CodecError> {
        let span = Span {
            start: self.text.len(),
            len: s.len(),
        };
        self.text.extend_from_slice(s.as_bytes())?;
        Ok(span)
    }
    fn attach(&mut self, parent: usize, key: Span, value: usize) -> Result<(), CodecError> {
        let id = self.edges.len();
        let array_len = match self.nodes[parent] {
            Node::Array { len, .. } => Some(len),
            _ => None,
        };
        if array_len.is_some() {
            self.array_index
                .try_reserve(1)
                .map_err(|_| super::work::allocation_error())?;
        }
        self.edges.try_push(Edge {
            key,
            value,
            next: None,
        })?;
        if let Some(len) = array_len {
            self.array_index.insert((parent, len), id);
        }
        match &mut self.nodes[parent] {
            Node::Array { first, last, len } | Node::Object { first, last, len } => {
                if let Some(last) = *last {
                    self.edges[last].next = Some(id);
                } else {
                    *first = Some(id);
                }
                *last = Some(id);
                *len += 1;
            }
            _ => return Err(invalid("JSON parent must container")),
        }
        Ok(())
    }
    pub fn view(&self) -> JsonView<'_> {
        JsonView::Owned(OwnedView {
            tree: self,
            id: self.root,
        })
    }
    pub fn budget(&self) -> &OwnedBudget {
        &self.budget
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn try_clone(&self) -> Result<Self, CodecError> {
        // Node::Number is serde_json's finite inline number under the locked std-only feature set.
        let mut array_index = ArrayIndex::with_hasher_in(
            RandomState::new(),
            LedgerAllocator::new_tag(&self.budget, "JSON array indexed edges"),
        );
        array_index
            .try_reserve(self.array_index.len())
            .map_err(|_| super::work::allocation_error())?;
        for (k, v) in &self.array_index {
            array_index.insert(*k, *v);
        }
        Ok(Self {
            nodes: self.nodes.try_clone()?,
            array_index,
            edges: self.edges.try_clone()?,
            text: self.text.try_clone()?,
            root: self.root,
            budget: self.budget.clone(),
        })
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn parse(bytes: &[u8], budget: &OwnedBudget) -> Result<Self, CodecError> {
        Parser::new(bytes, budget, DuplicateKeys::Reject).parse()
    }
    /// #1394 TestOnly：普通 serde_json Value 的 JSON-shape 入口，重复键取最后一个值。
    /// 与严格 wire 入口分别固定策略；严格解析失败不会重试此入口。
    /// 被替换的节点、键和子边仍留在 arena 并持账，直到整个 owner drop。
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn parse_legacy_shape(bytes: &[u8], budget: &OwnedBudget) -> Result<Self, CodecError> {
        Parser::new(bytes, budget, DuplicateKeys::LastWins).parse()
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn from_view(view: JsonView<'_>, budget: &OwnedBudget) -> Result<Self, CodecError> {
        let mut tree = Self::empty(budget);
        tree.root = tree.copy_view(view)?;
        Ok(tree)
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn object(budget: &OwnedBudget) -> Result<Self, CodecError> {
        let mut t = Self::empty(budget);
        t.root = t.add_node(Node::Object {
            first: None,
            last: None,
            len: 0,
        })?;
        Ok(t)
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn array(budget: &OwnedBudget) -> Result<Self, CodecError> {
        let mut t = Self::empty(budget);
        t.root = t.add_node(Node::Array {
            first: None,
            last: None,
            len: 0,
        })?;
        Ok(t)
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn string(s: &str, budget: &OwnedBudget) -> Result<Self, CodecError> {
        let mut t = Self::empty(budget);
        let s = t.copy_text(s)?;
        t.root = t.add_node(Node::String(s))?;
        Ok(t)
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn insert_field(&mut self, key: &str, value: JsonView<'_>) -> Result<(), CodecError> {
        let object = self
            .view()
            .as_object()
            .ok_or_else(|| invalid("JSON insert requires object"))?;
        let existing = object.owned_edge(key);
        let value = self.copy_view(value)?;
        if let Some(e) = existing {
            self.edges[e].value = value;
            return Ok(());
        }
        let key = self.copy_text(key)?;
        self.attach(self.root, key, value)?;
        self.sort_fields(self.root);
        Ok(())
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn push(&mut self, value: JsonView<'_>) -> Result<(), CodecError> {
        if self.view().as_array().is_none() {
            return Err(invalid("JSON push requires array"));
        }
        let value = self.copy_view(value)?;
        self.attach(self.root, Span::default(), value)
    }
    fn sort_fields(&mut self, id: usize) {
        // Stable insertion into linked edges: no std sort scratch, no heap growth.
        let (mut unsorted, len) = match self.nodes[id] {
            Node::Object { first, len, .. } => (first, len),
            _ => return,
        };
        let mut sorted: Option<usize> = None;
        while let Some(e) = unsorted {
            unsorted = self.edges[e].next;
            let mut prev: Option<usize> = None;
            let mut cursor = sorted;
            while let Some(c) = cursor {
                if self.text(self.edges[e].key) < self.text(self.edges[c].key) {
                    break;
                }
                prev = Some(c);
                cursor = self.edges[c].next;
            }
            self.edges[e].next = cursor;
            if let Some(p) = prev {
                self.edges[p].next = Some(e);
            } else {
                sorted = Some(e);
            }
        }
        let mut last = sorted;
        while let Some(e) = last {
            if self.edges[e].next.is_none() {
                break;
            }
            last = self.edges[e].next;
        }
        self.nodes[id] = Node::Object {
            first: sorted,
            last,
            len,
        };
    }
    fn shallow(&mut self, v: JsonView<'_>) -> Result<usize, CodecError> {
        let n = match v.kind() {
            JsonKind::Null => Node::Null,
            JsonKind::Bool => Node::Bool(v.as_bool().unwrap()),
            JsonKind::Number => Node::Number(v.number().unwrap().clone()),
            JsonKind::String => Node::String(self.copy_text(v.as_str().unwrap())?),
            JsonKind::Array => Node::Array {
                first: None,
                last: None,
                len: 0,
            },
            JsonKind::Object => Node::Object {
                first: None,
                last: None,
                len: 0,
            },
        };
        self.add_node(n)
    }
    fn copy_view(&mut self, v: JsonView<'_>) -> Result<usize, CodecError> {
        struct CopyFrame<'a> {
            dest: usize,
            children: Children<'a>,
        }
        let root = self.shallow(v)?;
        let mut stack = CodecVec::new(Some(&self.budget));
        if let Some(children) = v.children() {
            stack.try_push(CopyFrame {
                dest: root,
                children,
            })?;
        }
        while let Some(frame) = stack.last_mut() {
            let Some((key, v)) = frame.children.next() else {
                let done = stack.pop().unwrap();
                self.sort_fields(done.dest);
                continue;
            };
            let parent = frame.dest;
            let key = self.copy_text(key)?;
            let dest = self.shallow(v)?;
            self.attach(parent, key, dest)?;
            if let Some(children) = v.children() {
                if stack.len() >= 127 {
                    return Err(invalid("recursion limit exceeded"));
                }
                stack.try_push(CopyFrame { dest, children })?;
            }
        }
        Ok(root)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonKind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}
#[derive(Clone, Copy)]
pub enum JsonView<'a> {
    Plain(&'a Value),
    Owned(OwnedView<'a>),
    Null,
    Bool(bool),
    String(&'a str),
    Number(&'a Number),
    ArraySlice(&'a [JsonView<'a>]),
    BorrowedArray(&'a dyn JsonArray),
    BorrowedObject(BorrowedObject<'a>),
    ObjectSlice(BorrowedFields<'a>),
}
/// 只由 arena 的有效节点产生。索引及所有者不可由 safe 外部调用者构造或改写。
/// ```compile_fail
/// use newchan_rust::s_content::capture_allocation::audit_json::{JsonView, OwnedJson, OwnedView};
/// fn forge(tree: &OwnedJson) {
///     let _ = JsonView::Owned(OwnedView { tree, id: usize::MAX });
/// }
/// ```
#[derive(Clone, Copy)]
pub struct OwnedView<'a> {
    tree: &'a OwnedJson,
    id: usize,
}
/// 校过唯一键的借用字段组合；不复制key或子树，迭代沿UTF8键排序，无sort scratch。
#[derive(Clone, Copy)]
pub struct BorrowedFields<'a> {
    fields: &'a [(&'a str, JsonView<'a>)],
}
impl<'a> JsonView<'a> {
    pub fn object_fields(fields: &'a [(&'a str, JsonView<'a>)]) -> Result<Self, CodecError> {
        for i in 0..fields.len() {
            for j in 0..i {
                if fields[i].0 == fields[j].0 {
                    return Err(invalid("duplicate key"));
                }
            }
        }
        Ok(Self::ObjectSlice(BorrowedFields { fields }))
    }
}
impl<'a> From<&'a Value> for JsonView<'a> {
    fn from(v: &'a Value) -> Self {
        Self::Plain(v)
    }
}
impl<'a> JsonView<'a> {
    pub fn kind(self) -> JsonKind {
        match self {
            Self::Plain(v) => match v {
                Value::Null => JsonKind::Null,
                Value::Bool(_) => JsonKind::Bool,
                Value::Number(_) => JsonKind::Number,
                Value::String(_) => JsonKind::String,
                Value::Array(_) => JsonKind::Array,
                Value::Object(_) => JsonKind::Object,
            },
            Self::Null => JsonKind::Null,
            Self::Bool(_) => JsonKind::Bool,
            Self::String(_) => JsonKind::String,
            Self::Number(_) => JsonKind::Number,
            Self::ArraySlice(_) | Self::BorrowedArray(_) => JsonKind::Array,
            Self::BorrowedObject(_) => JsonKind::Object,
            Self::ObjectSlice(_) => JsonKind::Object,
            Self::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::Null => JsonKind::Null,
                Node::Bool(_) => JsonKind::Bool,
                Node::Number(_) => JsonKind::Number,
                Node::String(_) => JsonKind::String,
                Node::Array { .. } => JsonKind::Array,
                Node::Object { .. } => JsonKind::Object,
            },
        }
    }
    pub fn is_null(self) -> bool {
        self.kind() == JsonKind::Null
    }
    pub fn is_object(self) -> bool {
        self.kind() == JsonKind::Object
    }
    pub fn is_array(self) -> bool {
        self.kind() == JsonKind::Array
    }
    pub fn is_string(self) -> bool {
        self.kind() == JsonKind::String
    }
    pub fn is_number(self) -> bool {
        self.kind() == JsonKind::Number
    }
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::Plain(v) => v.as_bool(),
            Self::Bool(v) => Some(v),
            Self::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::Bool(b) => Some(b),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            Self::Plain(v) => v.as_str(),
            Self::String(v) => Some(v),
            Self::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::String(s) => Some(tree.text(s)),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn number(self) -> Option<&'a Number> {
        match self {
            Self::Plain(Value::Number(n)) => Some(n),
            Self::Number(n) => Some(n),
            Self::Owned(OwnedView { tree, id }) => match &tree.nodes[id] {
                Node::Number(n) => Some(n),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn as_i64(self) -> Option<i64> {
        self.number()?.as_i64()
    }
    pub fn as_u64(self) -> Option<u64> {
        self.number()?.as_u64()
    }
    pub fn as_f64(self) -> Option<f64> {
        self.number()?.as_f64()
    }
    pub fn as_array(self) -> Option<ArrayView<'a>> {
        (self.kind() == JsonKind::Array).then_some(ArrayView(self))
    }
    pub fn as_object(self) -> Option<ObjectView<'a>> {
        (self.kind() == JsonKind::Object).then_some(ObjectView(self))
    }
    pub fn get(self, key: &str) -> Option<Self> {
        self.as_object()?.get(key)
    }
    pub fn index(self, i: usize) -> Option<Self> {
        self.as_array()?.get(i)
    }
    fn children(self) -> Option<Children<'a>> {
        match self.kind() {
            JsonKind::Array => Some(Children::Array(self.as_array().unwrap().iter())),
            JsonKind::Object => Some(Children::Object(self.as_object().unwrap().iter())),
            _ => None,
        }
    }
}
impl std::fmt::Debug for JsonView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind() {
            JsonKind::Null => f.write_str("null"),
            JsonKind::Bool => self.as_bool().fmt(f),
            JsonKind::Number => self.number().fmt(f),
            JsonKind::String => self.as_str().fmt(f),
            JsonKind::Array => f
                .debug_list()
                .entries(self.as_array().unwrap().iter())
                .finish(),
            JsonKind::Object => f
                .debug_map()
                .entries(self.as_object().unwrap().iter())
                .finish(),
        }
    }
}
impl PartialEq for JsonView<'_> {
    fn eq(&self, o: &Self) -> bool {
        if self.kind() != o.kind() {
            return false;
        }
        match self.kind() {
            JsonKind::Null => true,
            JsonKind::Bool => self.as_bool() == o.as_bool(),
            JsonKind::Number => self.number() == o.number(),
            JsonKind::String => self.as_str() == o.as_str(),
            JsonKind::Array => {
                let a = self.as_array().unwrap();
                let b = o.as_array().unwrap();
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a == b)
            }
            JsonKind::Object => {
                let a = self.as_object().unwrap();
                let b = o.as_object().unwrap();
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k) == Some(v))
            }
        }
    }
}
#[derive(Clone, Copy)]
pub struct ArrayView<'a>(JsonView<'a>);
impl<'a> ArrayView<'a> {
    pub fn len(self) -> usize {
        match self.0 {
            JsonView::Plain(Value::Array(a)) => a.len(),
            JsonView::ArraySlice(a) => a.len(),
            JsonView::BorrowedArray(a) => a.len(),
            JsonView::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::Array { len, .. } => len,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn get(self, i: usize) -> Option<JsonView<'a>> {
        if i >= self.len() {
            return None;
        }
        match self.0 {
            JsonView::Plain(Value::Array(a)) => a.get(i).map(JsonView::Plain),
            JsonView::ArraySlice(a) => a.get(i).copied(),
            JsonView::BorrowedArray(a) => a.get(i),
            JsonView::Owned(OwnedView { tree, id }) => tree.array_index.get(&(id, i)).map(|e| {
                JsonView::Owned(OwnedView {
                    tree,
                    id: tree.edges[*e].value,
                })
            }),
            _ => unreachable!(),
        }
    }
    pub fn iter(self) -> ArrayIter<'a> {
        match self.0 {
            JsonView::Plain(Value::Array(a)) => ArrayIter::Plain(a.iter()),
            JsonView::ArraySlice(a) => ArrayIter::Slice(a.iter()),
            JsonView::BorrowedArray(a) => {
                ArrayIter::Borrowed(BorrowedArrayIter { owner: a, index: 0 })
            }
            JsonView::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::Array { first, .. } => ArrayIter::Owned(OwnedArrayIter { tree, next: first }),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
pub enum ArrayIter<'a> {
    Plain(std::slice::Iter<'a, Value>),
    Slice(std::slice::Iter<'a, JsonView<'a>>),
    Borrowed(BorrowedArrayIter<'a>),
    Owned(OwnedArrayIter<'a>),
}
/// 只由 ArrayView::iter 产生，私有游标沿真实 arena 边移动。
/// ```compile_fail
/// use newchan_rust::s_content::capture_allocation::audit_json::{ArrayIter, OwnedArrayIter, OwnedJson};
/// fn forge(tree: &OwnedJson) {
///     let _ = ArrayIter::Owned(OwnedArrayIter { tree, next: Some(usize::MAX) });
/// }
/// ```
pub struct OwnedArrayIter<'a> {
    tree: &'a OwnedJson,
    next: Option<usize>,
}
impl<'a> Iterator for ArrayIter<'a> {
    type Item = JsonView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Plain(i) => i.next().map(JsonView::Plain),
            Self::Slice(i) => i.next().copied(),
            Self::Borrowed(i) => {
                if i.index >= i.owner.len() {
                    return None;
                }
                let v = i.owner.get(i.index);
                i.index += 1;
                v
            }
            Self::Owned(OwnedArrayIter { tree, next }) => {
                let e = tree.edges[(*next)?];
                *next = e.next;
                Some(JsonView::Owned(OwnedView { tree, id: e.value }))
            }
        }
    }
}
impl<'a> IntoIterator for ArrayView<'a> {
    type Item = JsonView<'a>;
    type IntoIter = ArrayIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[derive(Clone, Copy)]
pub struct ObjectView<'a>(JsonView<'a>);
impl<'a> ObjectView<'a> {
    pub fn len(self) -> usize {
        match self.0 {
            JsonView::Plain(Value::Object(a)) => a.len(),
            JsonView::ObjectSlice(a) => a.fields.len(),
            JsonView::BorrowedObject(a) => a.owner.len(),
            JsonView::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::Object { len, .. } => len,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    fn owned_edge(self, key: &str) -> Option<usize> {
        let JsonView::Owned(OwnedView { tree, id }) = self.0 else {
            return None;
        };
        let Node::Object { first, .. } = tree.nodes[id] else {
            return None;
        };
        let mut cursor = first;
        while let Some(e) = cursor {
            if tree.text(tree.edges[e].key) == key {
                return Some(e);
            }
            cursor = tree.edges[e].next;
        }
        None
    }
    pub fn get(self, key: &str) -> Option<JsonView<'a>> {
        match self.0 {
            JsonView::Plain(v) => v.get(key).map(JsonView::Plain),
            JsonView::BorrowedObject(a) => (0..a.owner.len())
                .filter_map(|i| a.owner.field(i))
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v),
            JsonView::ObjectSlice(a) => a.fields.iter().find(|(k, _)| *k == key).map(|(_, v)| *v),
            JsonView::Owned(OwnedView { tree, .. }) => self.owned_edge(key).map(|e| {
                JsonView::Owned(OwnedView {
                    tree,
                    id: tree.edges[e].value,
                })
            }),
            _ => unreachable!(),
        }
    }
    pub fn keys(self) -> impl Iterator<Item = &'a str> {
        self.iter().map(|(k, _)| k)
    }
    pub fn iter(self) -> ObjectIter<'a> {
        match self.0 {
            JsonView::Plain(Value::Object(a)) => ObjectIter::Plain(a.iter()),
            JsonView::BorrowedObject(a) => ObjectIter::Borrowed(BorrowedObjectIter {
                owner: a.owner,
                last: None,
            }),
            JsonView::ObjectSlice(a) => ObjectIter::Slice(BorrowedFieldsIter {
                fields: a.fields,
                last: None,
            }),
            JsonView::Owned(OwnedView { tree, id }) => match tree.nodes[id] {
                Node::Object { first, .. } => {
                    ObjectIter::Owned(OwnedObjectIter { tree, next: first })
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
pub enum ObjectIter<'a> {
    Plain(serde_json::map::Iter<'a>),
    Slice(BorrowedFieldsIter<'a>),
    Borrowed(BorrowedObjectIter<'a>),
    Owned(OwnedObjectIter<'a>),
}
pub struct BorrowedFieldsIter<'a> {
    fields: &'a [(&'a str, JsonView<'a>)],
    last: Option<&'a str>,
}
/// 只由 ObjectView::iter 产生，私有游标沿真实 arena 边移动。
/// ```compile_fail
/// use newchan_rust::s_content::capture_allocation::audit_json::{ObjectIter, OwnedJson, OwnedObjectIter};
/// fn forge(tree: &OwnedJson) {
///     let _ = ObjectIter::Owned(OwnedObjectIter { tree, next: Some(usize::MAX) });
/// }
/// ```
pub struct OwnedObjectIter<'a> {
    tree: &'a OwnedJson,
    next: Option<usize>,
}
impl<'a> Iterator for ObjectIter<'a> {
    type Item = (&'a str, JsonView<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Plain(i) => i.next().map(|(k, v)| (k.as_str(), JsonView::Plain(v))),
            Self::Borrowed(i) => {
                let next = (0..i.owner.len())
                    .filter_map(|n| i.owner.field(n))
                    .filter(|(k, _)| i.last.is_none_or(|last| *k > last))
                    .min_by_key(|(k, _)| *k)?;
                i.last = Some(next.0);
                Some(next)
            }
            Self::Slice(i) => {
                let next = i
                    .fields
                    .iter()
                    .filter(|(k, _)| i.last.is_none_or(|last| *k > last))
                    .min_by_key(|(k, _)| *k)?;
                i.last = Some(next.0);
                Some(*next)
            }
            Self::Owned(OwnedObjectIter { tree, next }) => {
                let e = tree.edges[(*next)?];
                *next = e.next;
                Some((
                    tree.text(e.key),
                    JsonView::Owned(OwnedView { tree, id: e.value }),
                ))
            }
        }
    }
}
enum Children<'a> {
    Array(ArrayIter<'a>),
    Object(ObjectIter<'a>),
}
impl<'a> Iterator for Children<'a> {
    type Item = (&'a str, JsonView<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Array(i) => i.next().map(|v| ("", v)),
            Self::Object(i) => i.next(),
        }
    }
}

#[derive(Clone, Copy)]
#[cfg(feature = "s_allocation_json_testonly")]
struct ParseFrame {
    node: usize,
    object: bool,
    state: u8,
}
#[derive(Clone, Copy)]
#[cfg(feature = "s_allocation_json_testonly")]
enum DuplicateKeys {
    Reject,
    LastWins,
}
#[cfg(feature = "s_allocation_json_testonly")]
struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    tree: OwnedJson,
    stack: CodecVec<ParseFrame>,
    duplicate_keys: DuplicateKeys,
}
#[cfg(feature = "s_allocation_json_testonly")]
impl<'a> Parser<'a> {
    fn new(input: &'a [u8], b: &OwnedBudget, duplicate_keys: DuplicateKeys) -> Self {
        Self {
            input,
            pos: 0,
            tree: OwnedJson::empty(b),
            stack: CodecVec::new(Some(b)),
            duplicate_keys,
        }
    }
    fn ws(&mut self) {
        while matches!(self.input.get(self.pos), Some(b' ' | b'\n' | b'\t' | b'\r')) {
            self.pos += 1;
        }
    }
    fn take(&mut self, c: u8) -> Result<(), CodecError> {
        if self.input.get(self.pos) == Some(&c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(invalid("invalid JSON syntax"))
        }
    }
    fn parse(mut self) -> Result<OwnedJson, CodecError> {
        self.ws();
        let (root, container) = self.atom()?;
        self.tree.root = root;
        if let Some(object) = container {
            self.stack.try_push(ParseFrame {
                node: root,
                object,
                state: 0,
            })?;
        }
        while let Some(frame) = self.stack.last().copied() {
            self.ws();
            let close = if frame.object { b'}' } else { b']' };
            if frame.state == 2 {
                match self.input.get(self.pos) {
                    Some(c) if *c == close => {
                        self.pos += 1;
                        self.tree.sort_fields(frame.node);
                        self.stack.pop();
                        continue;
                    }
                    Some(b',') => {
                        self.pos += 1;
                        self.stack.last_mut().unwrap().state = 3;
                        continue;
                    }
                    _ => return Err(invalid("expected comma or container end")),
                }
            }
            if frame.state == 0 && self.input.get(self.pos) == Some(&close) {
                self.pos += 1;
                self.stack.pop();
                continue;
            }
            let (key, existing) = if frame.object {
                self.take(b'"')?;
                let k = self.string()?;
                let existing = (ObjectView(JsonView::Owned(OwnedView {
                    tree: &self.tree,
                    id: frame.node,
                })))
                .owned_edge(self.tree.text(k));
                if existing.is_some() && matches!(self.duplicate_keys, DuplicateKeys::Reject) {
                    return Err(CodecError::static_detail(SCHEMA, "duplicate key"));
                }
                self.ws();
                self.take(b':')?;
                self.ws();
                (k, existing)
            } else {
                (Span::default(), None)
            };
            let (child, container) = self.atom()?;
            if let Some(edge) = existing {
                // Keep the original linked edge and the other fields intact. The old value
                // and this occurrence's key remain genuinely allocated and ledger-owned.
                self.tree.edges[edge].value = child;
            } else {
                self.tree.attach(frame.node, key, child)?;
            }
            self.stack.last_mut().unwrap().state = 2;
            if let Some(object) = container {
                if self.stack.len() >= 127 {
                    return Err(invalid("recursion limit exceeded"));
                }
                self.stack.try_push(ParseFrame {
                    node: child,
                    object,
                    state: 0,
                })?;
            }
        }
        self.ws();
        if self.pos != self.input.len() {
            return Err(invalid("trailing characters"));
        }
        Ok(self.tree)
    }
    fn atom(&mut self) -> Result<(usize, Option<bool>), CodecError> {
        let c = *self
            .input
            .get(self.pos)
            .ok_or_else(|| invalid("EOF while parsing value"))?;
        let (mut container, node) = (None, Node::Null);
        let node = match c {
            b'n' => {
                self.literal(b"null")?;
                node
            }
            b't' => {
                self.literal(b"true")?;
                Node::Bool(true)
            }
            b'f' => {
                self.literal(b"false")?;
                Node::Bool(false)
            }
            b'"' => {
                self.pos += 1;
                Node::String(self.string()?)
            }
            b'[' => {
                self.pos += 1;
                container = Some(false);
                Node::Array {
                    first: None,
                    last: None,
                    len: 0,
                }
            }
            b'{' => {
                self.pos += 1;
                container = Some(true);
                Node::Object {
                    first: None,
                    last: None,
                    len: 0,
                }
            }
            b'-' | b'0'..=b'9' => {
                let (n, used) = number::parse(&self.input[self.pos..])?;
                self.pos += used;
                Node::Number(n)
            }
            _ => return Err(invalid("expected value")),
        };
        Ok((self.tree.add_node(node)?, container))
    }
    fn literal(&mut self, v: &[u8]) -> Result<(), CodecError> {
        if self.input[self.pos..].starts_with(v) {
            self.pos += v.len();
            Ok(())
        } else {
            Err(invalid("expected identifier"))
        }
    }
    fn hex(&mut self) -> Result<u16, CodecError> {
        let mut v = 0;
        for _ in 0..4 {
            let b = *self
                .input
                .get(self.pos)
                .ok_or_else(|| invalid("EOF while parsing string"))?;
            self.pos += 1;
            let n = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(invalid("invalid escape")),
            };
            v = (v << 4) | u16::from(n);
        }
        Ok(v)
    }
    fn string(&mut self) -> Result<Span, CodecError> {
        let start = self.tree.text.len();
        loop {
            let run = self.pos;
            while let Some(c) = self.input.get(self.pos) {
                if *c == b'"' || *c == b'\\' || *c < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            let raw = &self.input[run..self.pos];
            std::str::from_utf8(raw).map_err(|_| invalid("invalid unicode code point"))?;
            self.tree.text.extend_from_slice(raw)?;
            let c = *self
                .input
                .get(self.pos)
                .ok_or_else(|| invalid("EOF while parsing string"))?;
            self.pos += 1;
            if c == b'"' {
                return Ok(Span {
                    start,
                    len: self.tree.text.len() - start,
                });
            }
            if c != b'\\' {
                return Err(invalid("control character while parsing string"));
            }
            let e = *self
                .input
                .get(self.pos)
                .ok_or_else(|| invalid("EOF while parsing string"))?;
            self.pos += 1;
            match e {
                b'"' | b'\\' | b'/' => self.tree.text.try_push(e)?,
                b'b' => self.tree.text.try_push(8)?,
                b'f' => self.tree.text.try_push(12)?,
                b'n' => self.tree.text.try_push(10)?,
                b'r' => self.tree.text.try_push(13)?,
                b't' => self.tree.text.try_push(9)?,
                b'u' => {
                    let h = self.hex()?;
                    let cp = if (0xD800..=0xDBFF).contains(&h) {
                        self.take(b'\\')?;
                        self.take(b'u')?;
                        let l = self.hex()?;
                        if !(0xDC00..=0xDFFF).contains(&l) {
                            return Err(invalid("lone leading surrogate in hex escape"));
                        }
                        0x10000 + ((u32::from(h) - 0xD800) << 10) + (u32::from(l) - 0xDC00)
                    } else if (0xDC00..=0xDFFF).contains(&h) {
                        return Err(invalid("lone trailing surrogate in hex escape"));
                    } else {
                        u32::from(h)
                    };
                    let c =
                        char::from_u32(cp).ok_or_else(|| invalid("invalid unicode code point"))?;
                    let mut buf = [0; 4];
                    self.tree
                        .text
                        .extend_from_slice(c.encode_utf8(&mut buf).as_bytes())?;
                }
                _ => return Err(invalid("invalid escape")),
            }
        }
    }
}
trait JsonBytes {
    fn try_push(&mut self, b: u8) -> Result<(), CodecError>;
    fn extend_from_slice(&mut self, b: &[u8]) -> Result<(), CodecError>;
}
impl JsonBytes for CodecVec<u8> {
    fn try_push(&mut self, b: u8) -> Result<(), CodecError> {
        CodecVec::try_push(self, b)
    }
    fn extend_from_slice(&mut self, b: &[u8]) -> Result<(), CodecError> {
        CodecVec::extend_from_slice(self, b)
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
struct BoundedJsonOutput<F> {
    bytes: CodecVec<u8>,
    max: u64,
    precheck: F,
}
#[cfg(feature = "s_allocation_json_testonly")]
impl<F: FnMut() -> Result<(), CodecError>> JsonBytes for BoundedJsonOutput<F> {
    fn try_push(&mut self, b: u8) -> Result<(), CodecError> {
        self.extend_from_slice(&[b])
    }
    fn extend_from_slice(&mut self, b: &[u8]) -> Result<(), CodecError> {
        (self.precheck)()?;
        if (self.bytes.len() as u128) + (b.len() as u128) > self.max as u128 {
            return Err(CodecError::static_detail(
                "Incomplete/CaptureBytes",
                "内容编码超过显式物化上限",
            ));
        }
        // Retained容量门约束实际新Layout，不能logical len合格后先amortized分配8字节。
        // allocator grow仍在旧Layout活着时预留完整新Layout，同账保old+new峰值。
        let end = self
            .bytes
            .len()
            .checked_add(b.len())
            .ok_or_else(super::work::allocation_error)?;
        if end > self.bytes.capacity() {
            let max = usize::try_from(self.max).unwrap_or(usize::MAX);
            let geometric = self.bytes.capacity().max(4).saturating_mul(2);
            let target = end.max(geometric).min(max);
            self.bytes.reserve_exact(target - self.bytes.len())?;
        }
        self.bytes.extend_from_slice(b)
    }
}
fn quoted(out: &mut impl JsonBytes, s: &str) -> Result<(), CodecError> {
    out.try_push(b'"')?;
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        let escaped = match b {
            b'"' => Some(&b"\\\""[..]),
            b'\\' => Some(&b"\\\\"[..]),
            8 => Some(&b"\\b"[..]),
            9 => Some(&b"\\t"[..]),
            10 => Some(&b"\\n"[..]),
            12 => Some(&b"\\f"[..]),
            13 => Some(&b"\\r"[..]),
            _ => None,
        };
        if escaped.is_some() || b < 0x20 {
            out.extend_from_slice(&s.as_bytes()[start..i])?;
            if let Some(e) = escaped {
                out.extend_from_slice(e)?;
            } else {
                const H: &[u8; 16] = b"0123456789abcdef";
                out.extend_from_slice(&[
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    H[(b >> 4) as usize],
                    H[(b & 15) as usize],
                ])?;
            }
            start = i + 1;
        }
    }
    out.extend_from_slice(&s.as_bytes()[start..])?;
    out.try_push(b'"')
}
struct NumberBuffer {
    bytes: [u8; 64],
    used: usize,
}
impl Write for NumberBuffer {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        let end = self.used + b.len();
        assert!(
            end <= self.bytes.len(),
            "finite serde_json Number exceeds 64 bytes"
        );
        self.bytes[self.used..end].copy_from_slice(b);
        self.used = end;
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// serde_json 锁定的有限 Number formatter；返回固定栈所有者，不分配 String。
pub struct FormattedNumber(NumberBuffer);
impl FormattedNumber {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0.bytes[..self.0.used]
    }
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_bytes()).expect("number formatter ASCII")
    }
}
pub fn formatted_number(n: &Number) -> FormattedNumber {
    let mut b = NumberBuffer {
        bytes: [0; 64],
        used: 0,
    };
    serde_json::to_writer(&mut b, n).expect("finite inline Number to infallible stack writer");
    FormattedNumber(b)
}
/// 与 ordinary canonical_json 共用 serde_json 的有限数字 formatter；额度错误只在 fallible 输出处产生。
#[cfg(feature = "s_allocation_json_testonly")]
pub fn canonical_bytes(
    view: JsonView<'_>,
    budget: &OwnedBudget,
) -> Result<CodecVec<u8>, CodecError> {
    canonical_bytes_bounded(view, budget, u64::MAX, || Ok(()))
}
/// 每个编码chunk在max/deadline/真实Layout增长前检查；不通过serde的boxed IO Error桥。
#[cfg(feature = "s_allocation_json_testonly")]
pub fn canonical_bytes_bounded(
    view: JsonView<'_>,
    budget: &OwnedBudget,
    max: u64,
    precheck: impl FnMut() -> Result<(), CodecError>,
) -> Result<CodecVec<u8>, CodecError> {
    struct Frame<'a> {
        children: Children<'a>,
        object: bool,
        first: bool,
    }
    let mut out = BoundedJsonOutput {
        bytes: CodecVec::new(Some(budget)),
        max,
        precheck,
    };
    let mut stack = CodecVec::new(Some(budget));
    let mut current = Some(view);
    loop {
        (out.precheck)()?;
        if let Some(v) = current.take() {
            match v.kind() {
                JsonKind::Null => out.extend_from_slice(b"null")?,
                JsonKind::Bool => out.extend_from_slice(if v.as_bool().unwrap() {
                    b"true"
                } else {
                    b"false"
                })?,
                JsonKind::String => quoted(&mut out, v.as_str().unwrap())?,
                JsonKind::Number => {
                    let mut b = NumberBuffer {
                        bytes: [0; 64],
                        used: 0,
                    };
                    serde_json::to_writer(&mut b, v.number().unwrap())
                        .expect("finite inline Number to infallible stack writer");
                    out.extend_from_slice(&b.bytes[..b.used])?;
                }
                JsonKind::Array | JsonKind::Object => {
                    let object = v.is_object();
                    out.try_push(if object { b'{' } else { b'[' })?;
                    stack.try_push(Frame {
                        children: v.children().unwrap(),
                        object,
                        first: true,
                    })?;
                }
            }
        }
        let Some(frame) = stack.last_mut() else {
            break;
        };
        if let Some((key, child)) = frame.children.next() {
            if !frame.first {
                out.try_push(b',')?;
            }
            frame.first = false;
            if frame.object {
                quoted(&mut out, key)?;
                out.try_push(b':')?;
            }
            current = Some(child);
        } else {
            out.try_push(if frame.object { b'}' } else { b']' })?;
            stack.pop();
        }
    }
    Ok(out.bytes)
}

impl std::fmt::Debug for OwnedJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.view().fmt(f)
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
pub(crate) mod tests {
    use super::*;
    use crate::session_protocol::{canonical_json, strict_json};
    // Observation only: every requested allocation is forwarded unchanged to System.
    // Per-thread activation avoids counting the test harness and concurrent test threads.
    thread_local! { static GLOBAL_CALLS: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) }; }
    struct ObserveAllocator;
    #[global_allocator]
    static OBSERVE: ObserveAllocator = ObserveAllocator;
    unsafe impl std::alloc::GlobalAlloc for ObserveAllocator {
        unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
            let _ = GLOBAL_CALLS.try_with(|n| {
                if let Some(v) = n.get() {
                    n.set(Some(v + 1));
                }
            });
            unsafe { std::alloc::System.alloc(layout) }
        }
        unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
            let _ = GLOBAL_CALLS.try_with(|n| {
                if let Some(v) = n.get() {
                    n.set(Some(v + 1));
                }
            });
            unsafe { std::alloc::System.alloc_zeroed(layout) }
        }
        unsafe fn realloc(&self, p: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
            let _ = GLOBAL_CALLS.try_with(|n| {
                if let Some(v) = n.get() {
                    n.set(Some(v + 1));
                }
            });
            unsafe { std::alloc::System.realloc(p, layout, size) }
        }
        unsafe fn dealloc(&self, p: *mut u8, layout: std::alloc::Layout) {
            unsafe { std::alloc::System.dealloc(p, layout) }
        }
    }
    pub(crate) fn observed<T>(f: impl FnOnce() -> T) -> (T, u64) {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                GLOBAL_CALLS.with(|n| n.set(None));
            }
        }
        GLOBAL_CALLS.with(|n| n.set(Some(0)));
        let reset = Reset;
        let value = f();
        let calls = GLOBAL_CALLS.with(|n| n.get().unwrap());
        drop(reset);
        (value, calls)
    }
    #[test]
    fn owned_json_readonly_global_observer_finds_no_unledgered_scratch_or_errors() {
        let budget = OwnedBudget::new(100000);
        let before = budget.snapshot().allocation_attempts;
        let (tree, calls) = observed(|| {
            OwnedJson::parse(br#"{"\u0061":"x\uD834\uDD1Ex","nested":[{"float":1.234567890123456789e-308},true,null]}"#,&budget)
        });
        let tree = tree.unwrap();
        assert_eq!(calls, budget.snapshot().allocation_attempts - before);
        let before = budget.snapshot().allocation_attempts;
        let (copy, calls) = observed(|| tree.try_clone());
        let copy = copy.unwrap();
        assert_eq!(calls, budget.snapshot().allocation_attempts - before);
        let before = budget.snapshot().allocation_attempts;
        let (bytes, calls) = observed(|| canonical_bytes(tree.view(), &budget));
        let bytes = bytes.unwrap();
        assert_eq!(calls, budget.snapshot().allocation_attempts - before);
        for input in [
            &b"{\"a\":0,\"\\u0061\":[[[["[..],
            &b"1e9999"[..],
            &b"\"\\uD800\""[..],
            &b"null trailing"[..],
        ] {
            let before = budget.snapshot().allocation_attempts;
            let (error, calls) = observed(|| OwnedJson::parse(input, &budget));
            assert!(error.is_err());
            assert_eq!(calls, budget.snapshot().allocation_attempts - before);
        }
        let zero = OwnedBudget::new(0);
        // macOS std Mutex may initialize a fixed native lock on first use.
        // Fixed meter/control initialization is outside the payload scope; warm it before observation.
        let _ = zero.snapshot();
        let (error, calls) = observed(|| OwnedJson::parse(br#"{"large":"payload"}"#, &zero));
        assert_eq!(error.unwrap_err().code(), super::super::LIMIT);
        assert_eq!(calls, 0);
        drop((bytes, copy, tree));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_bounded_number_one_byte_capacity_gate() {
        let budget = OwnedBudget::new(1000);
        let zero = Value::from(0);
        let bytes = canonical_bytes_bounded((&zero).into(), &budget, 1, || Ok(())).unwrap();
        assert_eq!(&*bytes, b"0");
        assert_eq!(bytes.capacity(), 1);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 1);
        drop(bytes);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let no_room = OwnedBudget::new(1000);
        let before = no_room.snapshot().allocation_attempts;
        let e = canonical_bytes_bounded((&zero).into(), &no_room, 0, || Ok(())).unwrap_err();
        assert_eq!(e.code(), "Incomplete/CaptureBytes");
        assert_eq!(no_room.snapshot().allocation_attempts, before);
        assert_eq!(no_room.snapshot().live_requested_layout_bytes, 0);
        let blocked = OwnedBudget::new(1000);
        let _ = blocked.snapshot();
        let e = canonical_bytes_bounded((&zero).into(), &blocked, 1, || {
            Err(CodecError::static_detail(
                "Incomplete/VerifyTimeout",
                "original absolute deadline",
            ))
        })
        .unwrap_err();
        assert_eq!(e.code(), "Incomplete/VerifyTimeout");
        assert_eq!(blocked.snapshot().allocation_attempts, 0);
    }
    #[test]
    fn bounded_escaping_geometric_layouts_stay_within_retained_cap() {
        let v = Value::String("\\".repeat(20000));
        let b = OwnedBudget::new(1_000_000);
        let out = canonical_bytes_bounded((&v).into(), &b, 40002, || Ok(())).unwrap();
        assert_eq!(out.len(), 40002);
        assert!(out.capacity() <= 40002);
        assert!(
            b.snapshot().allocation_attempts < 64,
            "geometric growth must not copy once per escape"
        );
        drop(out);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
    fn parity(input: &[u8]) {
        let ordinary = strict_json(input);
        let b = OwnedBudget::new(1_000_000);
        let owned = OwnedJson::parse(input, &b);
        assert_eq!(
            ordinary.is_ok(),
            owned.is_ok(),
            "input {:?}",
            String::from_utf8_lossy(input)
        );
        if let (Ok(v), Ok(t)) = (ordinary, owned) {
            assert_eq!(t.view(), JsonView::Plain(&v));
            let output = canonical_bytes(t.view(), &b).unwrap();
            assert_eq!(&*output, canonical_json(&v).as_bytes());
            let copy = OwnedJson::from_view(JsonView::Plain(&v), &b).unwrap();
            assert_eq!(t.view(), copy.view());
            let clone = t.try_clone().unwrap();
            assert_eq!(clone.view(), t.view());
            drop((output, copy, clone, t));
        }
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
    fn legacy_shape_parity(input: &[u8]) {
        let ordinary = std::str::from_utf8(input)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(text).ok());
        let budget = OwnedBudget::new(1_000_000);
        let owned = OwnedJson::parse_legacy_shape(input, &budget);
        assert_eq!(ordinary.is_some(), owned.is_ok(), "input {input:?}");
        if let (Some(value), Ok(tree)) = (ordinary, owned) {
            assert_eq!(tree.view(), JsonView::Plain(&value));
            let bytes = canonical_bytes(tree.view(), &budget).unwrap();
            assert_eq!(&*bytes, canonical_json(&value).as_bytes());
            drop((bytes, tree));
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_legacy_shape_last_wins_fields_numbers_and_unicode_match_serde() {
        for input in [
            &b"null"[..],
            &b"[]"[..],
            &b"{}"[..],
            &br#"{"z":true,"a":{"old":[1,2]},"other":"kept","\u0061":[{"n":-0,"n":18446744073709551616},"\uD834\uDD1E"],"last":null}"#[..],
            &br#"[{"a":1,"a":2,"b":3},{"a":{},"b":[],"a":{"a":1,"a":2},"c":false}]"#[..],
            &br#"{"number":1,"number":0.123456789012345678901234567890,"min":2.2250738585072014e-308,"max":1.7976931348623157e308,"underflow":-1e-999999999999999999999,"zero":0e999999999999999999999,"negative_zero":-0}"#[..],
            "{\"𝄞\":0,\"\\uD834\\uDD1E\":1,\"中文\":\"😀é\"}".as_bytes(),
        ] {
            legacy_shape_parity(input);
        }
        let input = br#"{"before":0,"a":1,"a":[{"x":true}],"after":2}"#;
        let budget = OwnedBudget::new(100000);
        let strict = OwnedJson::parse(input, &budget).unwrap_err();
        assert_eq!(strict.detail, "duplicate key");
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let legacy = OwnedJson::parse_legacy_shape(input, &budget).unwrap();
        assert_eq!(legacy.view().as_object().unwrap().len(), 3);
        assert_eq!(legacy.view().get("before").unwrap().as_u64(), Some(0));
        assert_eq!(legacy.view().get("after").unwrap().as_u64(), Some(2));
        drop(legacy);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_legacy_shape_validates_replaced_values_separators_depth_and_full_input() {
        for input in [
            &br#"{"a":0,"a":[1,]}"#[..],
            &br#"{"a":0,"a":1 "later":2}"#[..],
            &br#"{"a":0,"a":1,}"#[..],
            &br#"{"a":0,"a":{}]"#[..],
            &br#"{"a":0,"a":[]} trailing"#[..],
            &br#"{"a":0,"a":1e309}"#[..],
            &br#"{"a":"\uD800","a":"valid"}"#[..],
            &b"{\"a\":\"\xff\",\"a\":0}"[..],
        ] {
            legacy_shape_parity(input);
        }
        for depth in [126, 127, 128] {
            let input = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
            legacy_shape_parity(input.as_bytes());
        }
    }
    #[test]
    fn owned_legacy_shape_duplicate_growth_is_ledgered_refuses_low_budget_and_drops_zero() {
        let input = format!(
            "{{\"kept\":true,{}}}",
            vec![format!("\"a\":\"{}\"", "s".repeat(256)); 128].join(",")
        );
        let full = OwnedBudget::new(1_000_000);
        let before = full.snapshot().allocation_attempts;
        let (tree, calls) = observed(|| OwnedJson::parse_legacy_shape(input.as_bytes(), &full));
        let tree = tree.unwrap();
        assert_eq!(calls, full.snapshot().allocation_attempts - before);
        assert_eq!(tree.view().as_object().unwrap().len(), 2);
        assert_eq!(tree.view().get("kept").unwrap().as_bool(), Some(true));
        let peak = full.snapshot().peak_requested_layout_bytes;
        let retained = full.snapshot().live_requested_layout_bytes;
        // Superseded strings and nodes are not released while the arena is retained.
        assert!(retained > 256 * 128);
        drop(tree);
        assert_eq!(full.snapshot().live_requested_layout_bytes, 0);
        let low = OwnedBudget::new(peak - 1);
        let before = low.snapshot().allocation_attempts;
        let (error, calls) = observed(|| OwnedJson::parse_legacy_shape(input.as_bytes(), &low));
        assert_eq!(error.unwrap_err().code(), super::super::LIMIT);
        assert_eq!(calls, low.snapshot().allocation_attempts - before);
        let request = low.allocation_request_probe();
        assert!(request.live_before_refusal + request.refused_bytes > low.snapshot().limit_bytes);
        assert!(low.snapshot().rejected_layouts > 0);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_json_strings_duplicates_tail_and_depth_parity() {
        for input in [
            &b"{\"z\":[true,null,{},[],\"\\b\\f\\n\\r\\t\\/\\\\\\\"\"],\"a\":\"\\u0000\\uD834\\uDD1E\"}"[..],
            "{\"中文\":\"😀é\",\"é\":\"line\u{0001}\"}".as_bytes(),
            &b"{\"a\":0,\"\\u0061\":[[[["[..],
            &b"{\"\\uD834\\uDD1E\":0,\"\xf0\x9d\x84\x9e\":1}"[..],
            &b"\"\\uD800\""[..], &b"\"\\uDC00\""[..], &b"\"\\uD800\\u0041\""[..],
            &b"\"\xff\""[..], &b"\"\xed\xa0\x80\""[..], &b"\"\x00\""[..],
            &b"[1,]"[..], &b"{\"a\":1,}"[..], &b"null garbage"[..], &b"truefalse"[..],
            &b" \n\t\r [0,\"a\"] \r\n"[..], &b"[false]x"[..], &b""[..],
        ] {parity(input);}
        // Escaped controls are valid; raw controls are rejected.
        for depth in [1, 126, 127, 128, 129] {
            let input = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
            parity(input.as_bytes());
        }
        parity(r#"{"é":"line\u0001","😀":["\uD834\uDD1E"]}"#.as_bytes());
    }
    #[test]
    fn owned_json_locked_number_algorithm_parity() {
        for input in [
            "0",
            "-0",
            "1",
            "-1",
            "18446744073709551615",
            "18446744073709551616",
            "-9223372036854775808",
            "-9223372036854775809",
            "9999999999999999999999999999999999999999.1234",
            "0.123456789012345678901234567890",
            "2.2250738585072014e-308",
            "1.7976931348623157e308",
            "1e309",
            "0e999999999999999999999",
            "-0e999999999999999999999",
            "1e-999999999999999999999",
            "1e999999999999999999999",
            "1e",
            "1e+",
            "1.",
            "01",
            "-01",
            "+1",
            "NaN",
            "1E-324",
            "-1E-324",
        ] {
            parity(input.as_bytes());
        }
        for mantissa in [
            "0",
            "1",
            "-1",
            "12345678901234567890",
            "123456789012345678901",
            "0.0000000000000000000000000000001",
            "-1.23456789012345678901234567890",
        ] {
            for exponent in [-400, -324, -308, -20, 0, 20, 308, 309, 400] {
                parity(format!("{mantissa}e{exponent}").as_bytes());
            }
        }
        // Finite typed numbers preserve their exact Number value during deep copy/clone.
        for bits in [
            1u64,
            0x8000000000000000,
            0x0010000000000000,
            0x3ff0000000000001,
            0x7fefffffffffffff,
        ] {
            let value = Value::from(f64::from_bits(bits));
            let b = OwnedBudget::new(10_000);
            let t = OwnedJson::from_view((&value).into(), &b).unwrap();
            assert_eq!(
                t.view().as_f64().unwrap().to_bits(),
                value.as_f64().unwrap().to_bits()
            );
            let bytes = canonical_bytes(t.view(), &b).unwrap();
            assert_eq!(&*bytes, canonical_json(&value).as_bytes());
            drop((bytes, t));
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn owned_json_quota_clone_and_move_hold_real_layouts() {
        let b = OwnedBudget::new(8000);
        let t = OwnedJson::parse(
            br#"{"text":"a long retained string","array":[1,2,3,4,5]}"#,
            &b,
        )
        .unwrap();
        let before = b.snapshot().live_requested_layout_bytes;
        assert!(before > 0);
        let moved = t;
        assert_eq!(b.snapshot().live_requested_layout_bytes, before);
        let clone = moved.try_clone().unwrap();
        assert_eq!(clone.view(), moved.view());
        assert!(b.snapshot().live_requested_layout_bytes > before);
        drop(clone);
        assert_eq!(b.snapshot().live_requested_layout_bytes, before);
        drop(moved);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        let measurement = OwnedBudget::new(10000);
        let probe = OwnedJson::parse(
            br#"{"long key":"a long string","array":[1,2,3,4]}"#,
            &measurement,
        )
        .unwrap();
        let cap = measurement.snapshot().peak_requested_layout_bytes;
        drop(probe);
        let b = OwnedBudget::new(cap);
        let old =
            OwnedJson::parse(br#"{"long key":"a long string","array":[1,2,3,4]}"#, &b).unwrap();
        let before = b.snapshot();
        let err = old.try_clone().unwrap_err();
        assert_eq!(err.code(), super::super::LIMIT);
        assert!(matches!(err.detail, std::borrow::Cow::Borrowed(_)));
        assert_eq!(
            old.view().get("long key").unwrap().as_str(),
            Some("a long string")
        );
        assert_eq!(
            b.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        drop(old);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_json_key_string_array_growth_rejected_before_alloc_and_drop_zero() {
        for input in [
            format!("{{\"{}\":0}}", "k".repeat(256)),
            format!("\"{}\"", "s".repeat(256)),
            format!("[{}]", vec!["0"; 128].join(",")),
        ] {
            let full = OwnedBudget::new(100000);
            let t = OwnedJson::parse(input.as_bytes(), &full).unwrap();
            let peak = full.snapshot().peak_requested_layout_bytes;
            drop(t);
            let b = OwnedBudget::new(peak - 1);
            let error = OwnedJson::parse(input.as_bytes(), &b).unwrap_err();
            assert_eq!(error.code(), super::super::LIMIT);
            let p = b.allocation_request_probe();
            assert!(p.live_before_refusal + p.refused_bytes > b.snapshot().limit_bytes);
            assert!(b.snapshot().rejected_layouts > 0);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
        // All failure prefixes preserve valid Utf8 and drop partially initialized owners safely.
        for limit in 0..512 {
            let b = OwnedBudget::new(limit);
            let _ = OwnedJson::parse(
                br#"{"escaped\u006bey":"x\uD834\uDD1Ex","a":[true,false,null,{}]}"#,
                &b,
            );
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn owned_json_builder_and_encoding_failure_do_not_erase_owners() {
        let b = OwnedBudget::new(10000);
        let mut tree = OwnedJson::object(&b).unwrap();
        let mut arr = OwnedJson::array(&b).unwrap();
        let s = OwnedJson::string("retained", &b).unwrap();
        arr.push(s.view()).unwrap();
        tree.insert_field("z", arr.view()).unwrap();
        tree.insert_field("a", s.view()).unwrap();
        tree.insert_field("a", JsonView::Plain(&Value::from(42)))
            .unwrap();
        let expected = serde_json::json!({"a":42,"z":["retained"]});
        assert_eq!(tree.view(), JsonView::Plain(&expected));
        let bytes = canonical_bytes(tree.view(), &b).unwrap();
        assert_eq!(&*bytes, canonical_json(&expected).as_bytes());
        drop(bytes);
        let low = OwnedBudget::new(0);
        let error = canonical_bytes(tree.view(), &low).unwrap_err();
        assert_eq!(error.code(), super::super::LIMIT);
        assert_eq!(tree.view(), JsonView::Plain(&expected));
        assert_eq!(low.snapshot().allocation_attempts, 0);
        drop((s, arr, tree));
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
}

/// 窄实验构造器；无Plain选择或额度失败后fallback。
#[cfg(feature = "s_allocation_json_testonly")]
pub fn owned_vec<T>(b: &OwnedBudget) -> CodecVec<T> {
    CodecVec::new(Some(b))
}
#[cfg(feature = "s_allocation_json_testonly")]
pub fn clone_bytes(v: &CodecVec<u8>) -> Result<CodecVec<u8>, CodecError> {
    v.try_clone()
}
#[cfg(feature = "s_allocation_json_testonly")]
pub fn extend_bytes(v: &mut CodecVec<u8>, bytes: &[u8]) -> Result<(), CodecError> {
    v.extend_from_slice(bytes)
}

/// SQL等借用输入直接写同一arena；不先构造ordinary Value/String再复制。
#[cfg(feature = "s_allocation_json_testonly")]
pub struct ObjectMut<'a> {
    tree: &'a mut OwnedJson,
    id: usize,
}
#[cfg(feature = "s_allocation_json_testonly")]
impl ObjectMut<'_> {
    pub fn view(&self) -> JsonView<'_> {
        JsonView::Owned(OwnedView {
            tree: self.tree,
            id: self.id,
        })
    }
    fn insert_node(&mut self, key: &str, node: Node) -> Result<(), CodecError> {
        let existing = self.view().as_object().unwrap().owned_edge(key);
        let value = self.tree.add_node(node)?;
        if let Some(e) = existing {
            self.tree.edges[e].value = value;
            return Ok(());
        }
        let key = self.tree.copy_text(key)?;
        self.tree.attach(self.id, key, value)
    }
    pub fn insert_str(&mut self, key: &str, value: &str) -> Result<(), CodecError> {
        let span = self.tree.copy_text(value)?;
        self.insert_node(key, Node::String(span))
    }
    pub fn insert_i64(&mut self, key: &str, value: i64) -> Result<(), CodecError> {
        self.insert_node(key, Node::Number(Number::from(value)))
    }
    pub fn insert_null(&mut self, key: &str) -> Result<(), CodecError> {
        self.insert_node(key, Node::Null)
    }
}
#[cfg(feature = "s_allocation_json_testonly")]
impl OwnedJson {
    pub fn append_object(
        &mut self,
        build: impl FnOnce(&mut ObjectMut<'_>) -> Result<(), CodecError>,
    ) -> Result<(), CodecError> {
        if self.view().as_array().is_none() {
            return Err(invalid("append_object requires array"));
        }
        let id = self.add_node(Node::Object {
            first: None,
            last: None,
            len: 0,
        })?;
        build(&mut ObjectMut { tree: self, id })?;
        self.sort_fields(id);
        self.attach(self.root, Span::default(), id)
    }
    pub fn object_at_mut(&mut self, index: usize) -> Result<ObjectMut<'_>, CodecError> {
        let v = self
            .view()
            .index(index)
            .ok_or_else(|| invalid("array index absent"))?;
        let JsonView::Owned(OwnedView { id, .. }) = v else {
            unreachable!()
        };
        if v.as_object().is_none() {
            return Err(invalid("array element must object"));
        }
        Ok(ObjectMut { tree: self, id })
    }
}
impl serde::Serialize for JsonView<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self.kind() {
            JsonKind::Null => s.serialize_unit(),
            JsonKind::Bool => s.serialize_bool(self.as_bool().unwrap()),
            JsonKind::String => s.serialize_str(self.as_str().unwrap()),
            JsonKind::Number => self.number().unwrap().serialize(s),
            JsonKind::Array => {
                let a = self.as_array().unwrap();
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for v in a.iter() {
                    seq.serialize_element(&v)?;
                }
                seq.end()
            }
            JsonKind::Object => {
                let a = self.as_object().unwrap();
                let mut map = s.serialize_map(Some(a.len()))?;
                for (k, v) in a.iter() {
                    map.serialize_entry(k, &v)?;
                }
                map.end()
            }
        }
    }
}

#[cfg(feature = "s_allocation_json_testonly")]
impl Drop for ObjectMut<'_> {
    fn drop(&mut self) {
        self.tree.sort_fields(self.id);
    }
}

/// 同账fallible业务映射，键/值的内部分配必须由其各自owner承担；本表不调用Clone。
pub struct BusinessMap<K, V>(super::work::WorkMap<K, V>);
/// 两种allocator使用同一hashbrown迭代器，不装Box、不复制key/value。
pub struct BusinessIter<'a, K, V>(hashbrown::hash_map::Iter<'a, K, V>);
pub struct BusinessIterMut<'a, K, V>(hashbrown::hash_map::IterMut<'a, K, V>);
impl<'a, K: 'a, V: 'a> Iterator for BusinessIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> ExactSizeIterator for BusinessIterMut<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for BusinessIterMut<'_, K, V> {}
impl<'a, K: 'a, V: 'a> Iterator for BusinessIter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<K, V> ExactSizeIterator for BusinessIter<'_, K, V> {}
impl<K, V> std::iter::FusedIterator for BusinessIter<'_, K, V> {}
impl<K: Eq + std::hash::Hash, V> BusinessMap<K, V> {
    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.0.retain(f)
    }
    pub fn iter_mut(&mut self) -> BusinessIterMut<'_, K, V> {
        BusinessIterMut(self.0.iter_mut())
    }
    pub fn plain() -> Self {
        Self(super::work::WorkMap::new(None))
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn owned(b: &OwnedBudget) -> Self {
        Self(super::work::WorkMap::new(Some(b)))
    }
    pub fn get<Q: std::hash::Hash + hashbrown::Equivalent<K> + ?Sized>(
        &self,
        key: &Q,
    ) -> Option<&V> {
        self.0.get(key)
    }
    pub fn get_mut<Q: std::hash::Hash + hashbrown::Equivalent<K> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Option<&mut V> {
        self.0.get_mut(key)
    }
    pub fn remove<Q: std::hash::Hash + hashbrown::Equivalent<K> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Option<V> {
        self.0.remove(key)
    }
    pub fn contains_key<Q: std::hash::Hash + hashbrown::Equivalent<K> + ?Sized>(
        &self,
        key: &Q,
    ) -> bool {
        self.0.contains_key(key)
    }
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, CodecError> {
        self.0.insert(key, value)
    }
    pub fn iter(&self) -> BusinessIter<'_, K, V> {
        BusinessIter(self.0.iter())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

/// 生命周期只借真实所有者，组合投影不复制key或payload。
pub trait JsonArray {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Option<JsonView<'_>>;
}
pub trait JsonObject {
    fn len(&self) -> usize;
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)>;
}
#[derive(Clone, Copy)]
pub struct BorrowedObject<'a> {
    owner: &'a dyn JsonObject,
}
pub struct BorrowedArrayIter<'a> {
    owner: &'a dyn JsonArray,
    index: usize,
}
pub struct BorrowedObjectIter<'a> {
    owner: &'a dyn JsonObject,
    last: Option<&'a str>,
}
impl<'a> JsonView<'a> {
    pub fn borrowed_object(owner: &'a dyn JsonObject) -> Result<Self, CodecError> {
        for i in 0..owner.len() {
            let (k, _) = owner
                .field(i)
                .ok_or_else(|| invalid("borrowed object field absent"))?;
            for j in 0..i {
                if owner.field(j).is_some_and(|(prior, _)| prior == k) {
                    return Err(invalid("duplicate key"));
                }
            }
        }
        Ok(Self::BorrowedObject(BorrowedObject { owner }))
    }
    /// 只选输入已持有的同一账本；组合不同账本拒绝，不另造budget或fallback。
    pub fn allocation_budget(self) -> Result<Option<&'a OwnedBudget>, CodecError> {
        let direct = match self {
            Self::Owned(OwnedView { tree, .. }) => return Ok(Some(tree.budget())),
            _ => None,
        };
        let mut found = direct;
        if let Some(children) = self.children() {
            for (_, child) in children {
                if let Some(b) = child.allocation_budget()? {
                    if found.is_some_and(|old: &OwnedBudget| !std::sync::Arc::ptr_eq(&old.0, &b.0))
                    {
                        return Err(CodecError::static_detail(
                            super::INCOMPLETE,
                            "借用JSON组合含不同owned账本",
                        ));
                    }
                    found = Some(b);
                }
            }
        }
        Ok(found)
    }
}

#[cfg(feature = "s_allocation_json_testonly")]
pub fn owned_bytes(bytes: &[u8], budget: &OwnedBudget) -> Result<CodecVec<u8>, CodecError> {
    let mut out = owned_vec(budget);
    extend_bytes(&mut out, bytes)?;
    Ok(out)
}

#[cfg(feature = "s_allocation_json_testonly")]
#[doc(hidden)]
pub fn static_error(code: &'static str, detail: &'static str) -> CodecError {
    CodecError::static_detail(code, detail)
}

/// 内部业务文本真实字节所有者；move保账、深clone fallible、没有Clone/std擦除接口。
pub struct TextOwner {
    bytes: CodecVec<u8>,
}
impl TextOwner {
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn from_owned_bytes(bytes: CodecVec<u8>) -> Result<Self, CodecError> {
        if bytes.allocation_budget().is_none() {
            return Err(CodecError::static_detail(
                super::INCOMPLETE,
                "文本消费入口只接受实际Owned字节所有者",
            ));
        }
        std::str::from_utf8(&bytes)
            .map_err(|_| CodecError::static_detail(SCHEMA, "文本字节不是UTF8"))?;
        Ok(Self { bytes })
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn copy_from(value: &str, budget: &OwnedBudget) -> Result<Self, CodecError> {
        Ok(Self {
            bytes: owned_bytes(value.as_bytes(), budget)?,
        })
    }
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes).expect("private text copied from UTF8")
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn try_clone(&self) -> Result<Self, CodecError> {
        Ok(Self {
            bytes: self.bytes.try_clone()?,
        })
    }
}
impl std::ops::Deref for TextOwner {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Debug for TextOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl std::fmt::Display for TextOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl serde::Serialize for TextOwner {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl PartialEq for TextOwner {
    fn eq(&self, o: &Self) -> bool {
        self.as_str() == o.as_str()
    }
}
impl Eq for TextOwner {}
impl PartialOrd for TextOwner {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for TextOwner {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(o.as_str())
    }
}
impl std::hash::Hash for TextOwner {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}
