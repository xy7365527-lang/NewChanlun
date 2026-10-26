//! #1394 业务原始账的入口存储选择与同一借用判据。Owned失败直接Err，不转回std。
pub(crate) mod json_layout;
pub(crate) mod scoped_legacy;
use super::audit_store::{CodecRead, NativeAdmission, NativeTree};
use super::*;
use aj::{ArrayIter, ArrayView, JsonView};
use newchan_rust::s_content as sc;
#[cfg_attr(
    not(all(test, feature = "s_capture_native_testonly")),
    allow(
        unused_imports,
        reason = "#1488 再合 #1608：native 非测试、s_session 非测试、s_session 测试 构建不触达"
    )
)]
use newchan_rust::sqlite_checked::{CheckedRow, CheckedStatement};
use sc::capture_allocation::native_owned::NativeValue;
use sc::capture_allocation::scratch::{NativeReservation, WorkScope};
use sc::capture_allocation::{audit_json as aj, OwnedBudget};
use sha2::{Digest, Sha256};
use std::time::Instant;

pub(crate) enum RawEvents {
    Plain(Vec<Value>),
    Native(PlainJson),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(aj::OwnedJson),
}
impl Default for RawEvents {
    fn default() -> Self {
        Self::Plain(Vec::new())
    }
}
impl RawEvents {
    /// 保留实际数组拥有者；旧None入口只移动既有Vec，不授Native资格。
    pub(crate) fn from_json_owner(owner: JsonOwner) -> Result<Self, String> {
        match owner {
            JsonOwner::Plain(value) => {
                if !value.is_array() {
                    return Err("StorageUnavailable：原始账拥有者不是数组".into());
                }
                if value.is_native_admitted() {
                    if value.layout != NativeJsonLayout::ItemizedArray {
                        return Err(
                            "prototype_audit_allocation_incomplete：原始账不接受聚合Native数组许可"
                                .into(),
                        );
                    }
                    Ok(Self::Native(value))
                } else {
                    let Value::Array(rows) = value.into_unscoped()? else {
                        unreachable!()
                    };
                    Ok(Self::Plain(rows))
                }
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(value) => {
                if value.view().as_array().is_none() {
                    return Err("StorageUnavailable：Owned原始账不是数组".into());
                }
                Ok(Self::Owned(value))
            }
        }
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub(crate) fn push_native(
        &mut self,
        value: JsonView<'_>,
        work: &WorkScope,
    ) -> Result<(), String> {
        let Self::Native(owner) = self else {
            return Err("prototype_audit_allocation_incomplete：Native追加要求原始账拥有者".into());
        };
        push_native_array(owner, value, work)
    }
    pub fn view(&self) -> RawEventsView<'_> {
        match self {
            Self::Plain(v) => RawEventsView::plain(v),
            Self::Native(v) => RawEventsView::plain(v.as_array().expect("已核数组拥有者")),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => RawEventsView {
                storage: RawStorage::Arena(v.view().as_array().unwrap()),
                len: v.view().as_array().unwrap().len(),
                budget: Some(v.budget()),
            },
        }
    }
    pub fn budget(&self) -> Option<&OwnedBudget> {
        self.view().budget()
    }
    pub fn len(&self) -> usize {
        self.view().len()
    }
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn iter(&self) -> RawIter<'_> {
        self.view().iter()
    }
    #[cfg_attr(
        not(all(test, feature = "s_capture_native_testonly")),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
    )]
    #[cfg_attr(
        all(test, feature = "s_capture_native_testonly"),
        allow(dead_code, reason = "#1488：native 测试不使用旧 plain 入口")
    )]
    pub fn plain(&self) -> Result<&[Value], String> {
        match self {
            Self::Plain(v) => Ok(v),
            Self::Native(_) => {
                Err("prototype_audit_allocation_incomplete：Native原始账不可擦除为std值".into())
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(_) => {
                Err("prototype_audit_allocation_incomplete：owned原始账不可擦除为std值".into())
            }
        }
    }
}
impl serde::Serialize for RawEvents {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut a = s.serialize_seq(Some(self.len()))?;
        for v in self.iter() {
            a.serialize_element(&v)?;
        }
        a.end()
    }
}
impl std::fmt::Debug for RawEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
#[derive(Clone, Copy)]
enum RawStorage<'a> {
    Plain(&'a [Value]),
    #[cfg_attr(
        not(feature = "s_allocation_json_testonly"),
        allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 下使用")
    )]
    Arena(ArrayView<'a>),
    Borrowed(&'a [JsonView<'a>]),
}
#[derive(Clone, Copy)]
pub(crate) struct RawEventsView<'a> {
    storage: RawStorage<'a>,
    len: usize,
    budget: Option<&'a OwnedBudget>,
}
impl<'a> RawEventsView<'a> {
    pub fn plain(v: &'a [Value]) -> Self {
        Self {
            storage: RawStorage::Plain(v),
            len: v.len(),
            budget: None,
        }
    }
    pub fn len(self) -> usize {
        self.len
    }
    #[allow(dead_code, reason = "#1608：类型配套访问器，当前无调用方；保留不删")]
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
    pub fn prefix(self, n: usize) -> Result<Self, String> {
        if n > self.len {
            return Err("StorageUnavailable：原始账前缀越界".into());
        }
        Ok(Self { len: n, ..self })
    }
    pub fn get(self, i: usize) -> Option<JsonView<'a>> {
        if i >= self.len {
            return None;
        }
        match self.storage {
            RawStorage::Plain(v) => Some((&v[i]).into()),
            RawStorage::Arena(v) => v.get(i),
            RawStorage::Borrowed(v) => v.get(i).copied(),
        }
    }
    pub fn iter(self) -> RawIter<'a> {
        RawIter {
            storage: match self.storage {
                RawStorage::Plain(v) => RawIterStorage::Plain(v.iter()),
                RawStorage::Arena(v) => RawIterStorage::Arena(v.iter()),
                RawStorage::Borrowed(v) => RawIterStorage::Borrowed(v.iter()),
            },
            left: self.len,
        }
    }
    pub fn budget(self) -> Option<&'a OwnedBudget> {
        self.budget
    }
}
impl<'a> From<&'a [Value]> for RawEventsView<'a> {
    fn from(v: &'a [Value]) -> Self {
        Self::plain(v)
    }
}
impl<'a> From<&'a Vec<Value>> for RawEventsView<'a> {
    fn from(v: &'a Vec<Value>) -> Self {
        Self::plain(v)
    }
}
impl<'a, const N: usize> From<&'a [Value; N]> for RawEventsView<'a> {
    fn from(v: &'a [Value; N]) -> Self {
        Self::plain(v)
    }
}
impl<'a> From<&'a JsonOwner> for RawEventsView<'a> {
    fn from(owner: &'a JsonOwner) -> Self {
        let values = owner.view().as_array().expect("变化拥有者须为数组");
        let budget = match owner {
            JsonOwner::Plain(_) => None,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(value) => Some(value.budget()),
        };
        Self {
            storage: RawStorage::Arena(values),
            len: values.len(),
            budget,
        }
    }
}
impl<'a> From<&'a RawEvents> for RawEventsView<'a> {
    fn from(v: &'a RawEvents) -> Self {
        v.view()
    }
}
impl<'a> IntoIterator for RawEventsView<'a> {
    type Item = JsonView<'a>;
    type IntoIter = RawIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
enum RawIterStorage<'a> {
    Plain(std::slice::Iter<'a, Value>),
    Arena(ArrayIter<'a>),
    Borrowed(std::slice::Iter<'a, JsonView<'a>>),
}
pub(crate) struct RawIter<'a> {
    storage: RawIterStorage<'a>,
    left: usize,
}
impl<'a> Iterator for RawIter<'a> {
    type Item = JsonView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        match &mut self.storage {
            RawIterStorage::Plain(i) => i.next().map(JsonView::Plain),
            RawIterStorage::Arena(i) => i.next(),
            RawIterStorage::Borrowed(i) => i.next().copied(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}
impl ExactSizeIterator for RawIter<'_> {}
pub(crate) fn field<'a>(v: JsonView<'a>, k: &str) -> JsonView<'a> {
    v.get(k).unwrap_or(JsonView::Null)
}
pub(crate) fn text<'a>(v: JsonView<'a>, k: &str) -> Result<&'a str, String> {
    field(v, k)
        .as_str()
        .ok_or_else(|| format!("StorageUnavailable：{k}非文本"))
}
pub(crate) fn coord(v: JsonView<'_>) -> Result<i64, String> {
    crate::parse_source_coord(text(v, "source_coord")?)
        .map_err(|e| format!("StorageUnavailable：{e}"))
}
pub(crate) fn new_work<T>(b: Option<&OwnedBudget>) -> sc::CodecVec<T> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = b {
        return aj::owned_vec(b);
    }
    let _ = b;
    sc::CodecVec::from_plain(Vec::new())
}
pub(crate) use aj::BusinessMap;
pub(crate) fn new_map<K: Eq + std::hash::Hash, V>(b: Option<&OwnedBudget>) -> BusinessMap<K, V> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = b {
        return aj::BusinessMap::owned(b);
    }
    let _ = b;
    aj::BusinessMap::plain()
}
fn workmap<K: Eq + std::hash::Hash, V>(b: Option<&OwnedBudget>) -> BusinessMap<K, V> {
    new_map(b)
}
#[cfg_attr(
    not(all(test, feature = "s_allocation_json_testonly")),
    allow(dead_code, reason = "#1608：仅 s_allocation_json_testonly 测试使用")
)]
pub(crate) fn bytes_extend(bytes: &mut sc::CodecVec<u8>, value: &[u8]) -> Result<(), String> {
    bytes.try_extend_bytes(value).map_err(|e| e.to_string())
}
pub(crate) struct Effective<'a> {
    values: sc::CodecVec<JsonView<'a>>,
    budget: Option<&'a OwnedBudget>,
}
impl<'a> Effective<'a> {
    pub fn raw_view(&self) -> RawEventsView<'_> {
        RawEventsView {
            storage: RawStorage::Borrowed(&self.values),
            len: self.values.len(),
            budget: self.budget,
        }
    }
    #[allow(dead_code, reason = "#1608：类型配套访问器，当前无调用方；保留不删")]
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = JsonView<'a>> + '_ {
        self.values.iter().copied()
    }
    #[allow(dead_code, reason = "#1608：类型配套访问器，当前无调用方；保留不删")]
    pub fn view(&self) -> JsonView<'_> {
        JsonView::ArraySlice(&self.values)
    }
}
/// #1394：最大修订优先，同修订保首次；来源证书与实际 effective 共用此判据。
pub(crate) fn effective_precedence(
    left_revision: i64,
    left_ordinal: usize,
    right_revision: i64,
    right_ordinal: usize,
) -> std::cmp::Ordering {
    right_revision
        .cmp(&left_revision)
        .then_with(|| left_ordinal.cmp(&right_ordinal))
}
pub(crate) fn effective<'a>(
    all: RawEventsView<'a>,
    deadline: Option<Instant>,
) -> Result<Effective<'a>, String> {
    check_read_deadline(deadline)?;
    // Same max-revision/first-on-tie decision, ordered in O(n log n); no payload/key clones.
    let mut candidates = new_work::<(&str, i64, usize, JsonView<'a>)>(all.budget());
    for (ordinal, ev) in all.iter().enumerate() {
        check_read_deadline(deadline)?;
        candidates
            .try_push((
                field(ev, "identity_key").as_str().unwrap_or(""),
                field(ev, "revision").as_i64().unwrap_or(0),
                ordinal,
                ev,
            ))
            .map_err(|e| e.to_string())?;
    }
    candidates.sort_unstable_by(|a, b| {
        a.0.cmp(b.0)
            .then_with(|| effective_precedence(a.1, a.2, b.1, b.2))
    });
    let mut ordered = new_work::<(i64, JsonView<'a>)>(all.budget());
    let mut last = None;
    for (identity, _, _, v) in candidates {
        check_read_deadline(deadline)?;
        if last == Some(identity) {
            continue;
        }
        last = Some(identity);
        ordered
            .try_push((coord(v)?, v))
            .map_err(|e| e.to_string())?;
    }
    ordered.sort_unstable_by_key(|(c, _)| *c);
    if ordered.windows(2).any(|v| v[0].0 == v[1].0) {
        return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖业务身份".into());
    }
    let mut values = new_work(all.budget());
    for (_, v) in ordered {
        values.try_push(v).map_err(|e| e.to_string())?;
    }
    Ok(Effective {
        values,
        budget: all.budget(),
    })
}

/// raw 的事务内 facade；所有前缀状态只持有稳定 seq ordinal 和完整身份文本。
/// 原件资格由上层 owner 负责；取走 facts 不会保留 RawEventsView 或事务借用。
pub(super) struct RawPrefixIndex<'raw> {
    raw: RawEventsView<'raw>,
    facts: RawPrefixFacts,
    #[cfg(feature = "s_capture_native_testonly")]
    retained: super::audit_retention::RetainedDelta,
}
pub(super) struct RawPrefixFacts {
    prefix: usize,
    winners: BusinessMap<NativeValue<String>, (i64, usize, i64)>,
    ordered: NativeTree<i64, usize>,
    coordinates: BusinessMap<usize, i64>,
}
impl Default for RawPrefixFacts {
    fn default() -> Self {
        Self {
            prefix: 0,
            winners: BusinessMap::plain(),
            ordered: NativeTree::default(),
            coordinates: BusinessMap::plain(),
        }
    }
}
impl RawPrefixFacts {
    #[cfg_attr(
        not(all(test, feature = "s_capture_native_testonly")),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
    )]
    pub(super) fn prefix_len(&self) -> usize {
        self.prefix
    }
}
impl std::ops::Deref for RawPrefixIndex<'_> {
    type Target = RawPrefixFacts;
    fn deref(&self) -> &Self::Target {
        &self.facts
    }
}
impl std::ops::DerefMut for RawPrefixIndex<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.facts
    }
}
/// #1488：既有最终赢家算法的值结果，不授予原件或事务资格。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawWinner {
    pub coord: i64,
    pub ordinal: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawWinnerEdit {
    pub old: Option<RawWinner>,
    pub new: Option<RawWinner>,
}
pub(super) struct RawPrefixChanges {
    pub removed: sc::CodecVec<i64>,
    pub added: sc::CodecVec<(i64, usize)>,
    /// 按 identity 排序，每个 identity 只含最终赢家；消费时先撤全部 old 再加全部 new。
    pub winner_edits: sc::CodecVec<RawWinnerEdit>,
    #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
    pub raw_visits: usize,
    pub coord_decodes: usize,
    pub rewound: bool,
}

/// #1488：跨模块测试复用真实前缀核，不扩大生产索引的可见范围。
#[cfg(test)]
pub(crate) struct RawPrefixFixture<'raw>(RawPrefixIndex<'raw>);
#[cfg(test)]
pub(crate) struct RawPrefixFixtureChanges {
    pub winner_edits: sc::CodecVec<RawWinnerEdit>,
}
#[cfg(test)]
impl<'raw> RawPrefixFixture<'raw> {
    pub(crate) fn new(raw: RawEventsView<'raw>) -> Self {
        Self(RawPrefixIndex::new(raw))
    }

    pub(crate) fn advance(&mut self, prefix: usize) -> Result<RawPrefixFixtureChanges, String> {
        self.0
            .advance(prefix)
            .map(|changes| RawPrefixFixtureChanges {
                winner_edits: changes.winner_edits,
            })
    }

    pub(crate) fn ordered(&self) -> impl Iterator<Item = (i64, usize)> + '_ {
        self.0.ordered()
    }
}

impl<'raw> RawPrefixIndex<'raw> {
    pub(super) fn new(raw: RawEventsView<'raw>) -> Self {
        Self {
            raw,
            facts: RawPrefixFacts::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            retained: Default::default(),
        }
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 下使用")
    )]
    pub(super) fn restore(
        raw: RawEventsView<'raw>,
        facts: RawPrefixFacts,
        _bound: &super::audit_owner::BoundOriginals,
    ) -> Result<Self, String> {
        if facts.prefix > raw.len() || raw.budget().is_some() {
            return Err("StorageUnavailable：raw facts 不属于已绑定前缀".into());
        }
        Ok(Self {
            raw,
            facts,
            #[cfg(feature = "s_capture_native_testonly")]
            retained: Default::default(),
        })
    }
    pub(super) fn into_facts(self) -> RawPrefixFacts {
        self.facts
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn into_facts_retained(
        self,
        delta: &mut super::audit_retention::RetainedDelta,
    ) -> RawPrefixFacts {
        delta.merge(self.retained);
        self.facts
    }
    fn clear_winners(&mut self) {
        #[cfg(feature = "s_capture_native_testonly")]
        for (key, _) in self.facts.winners.iter() {
            self.retained.release(key);
        }
        #[cfg(feature = "s_capture_native_testonly")]
        let ordered_before = self.facts.ordered.allocation_bytes();
        // HashMap clear 保留桶；BTree clear 真实释放根。
        self.facts.winners.retain(|_, _| false);
        self.facts.ordered.clear();
        #[cfg(feature = "s_capture_native_testonly")]
        self.ordered_retained(ordered_before);
    }
    /// 有序树按其实际许可/节点上界（allocation_bytes）记增量，与全量 heap_bytes 同口径。
    #[cfg(feature = "s_capture_native_testonly")]
    fn ordered_retained(&mut self, before: usize) {
        let after = self.facts.ordered.allocation_bytes();
        if after >= before {
            self.retained.add_bytes(after - before);
        } else {
            self.retained.release_bytes(before - after);
        }
    }

    #[cfg_attr(
        not(all(test, feature = "s_capture_native_testonly")),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
    )]
    pub(super) fn is_empty(&self) -> bool {
        self.winners.is_empty()
    }
    pub(super) fn ordered(&self) -> impl Iterator<Item = (i64, usize)> + '_ {
        self.ordered.iter().map(|(c, i)| (*c, *i))
    }
    #[cfg(test)]
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1608：测试辅助，仅 s_capture_native_testonly 测试使用"
        )
    )]
    pub(super) fn retained_slots(&self) -> [usize; 3] {
        // HashMap 实际 capacity；BTreeMap 只报条目，不冒充节点布局/allocator开销。
        [
            self.winners.capacity(),
            self.coordinates.capacity(),
            self.ordered.len(),
        ]
    }
    pub(super) fn clear(&mut self) {
        self.prefix = 0;
        self.clear_winners();
        self.facts.coordinates.retain(|_, _| false);
    }
    #[cfg_attr(
        not(all(test, feature = "s_capture_native_testonly")),
        allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
    )]
    pub(super) fn advance(&mut self, prefix: usize) -> Result<RawPrefixChanges, String> {
        self.advance_admitted(prefix, &super::audit_store::NativeAdmission(None))
    }
    pub(super) fn advance_admitted(
        &mut self,
        prefix: usize,
        admission: &(impl super::audit_store::CodecRead + ?Sized),
    ) -> Result<RawPrefixChanges, String> {
        self.advance_measured(prefix, admission, None, None)
    }
    pub(super) fn advance_measured(
        &mut self,
        prefix: usize,
        admission: &(impl super::audit_store::CodecRead + ?Sized),
        deadline: Option<Instant>,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<RawPrefixChanges, String> {
        let result = self.advance_inner(prefix, admission, deadline, budget);
        if result.is_err() {
            self.clear();
        }
        result
    }
    fn advance_inner(
        &mut self,
        prefix: usize,
        admission: &(impl super::audit_store::CodecRead + ?Sized),
        deadline: Option<Instant>,
        budget: Option<&dyn super::audit_budget::Budget>,
    ) -> Result<RawPrefixChanges, String> {
        if self.raw.budget().is_some() {
            return Err("prototype_audit_allocation_incomplete：有序raw索引不接收Owned原件".into());
        }
        self.raw.prefix(prefix)?;
        let rewound = prefix < self.prefix;
        if rewound {
            self.prefix = 0;
            self.clear_winners();
        }
        // 同一 cut 内先选最终赢家；不会因跨越的暂态修订提前拒绝。
        let raw = self.raw;
        let mut changed = NativeTree::<&str, (i64, usize)>::default();
        for ordinal in self.prefix..prefix {
            super::history::check_deadline(deadline)?;
            let event = raw.get(ordinal).expect("已截前缀");
            if let Some(budget) = budget {
                budget.record().map_err(|e| e.to_string())?;
            }
            let identity = field(event, "identity_key").as_str().unwrap_or("");
            let revision = field(event, "revision").as_i64().unwrap_or(0);
            let previous = changed
                .get(identity)
                .copied()
                .or_else(|| self.winners.get(identity).map(|(r, i, _)| (*r, *i)));
            if previous.is_none_or(|(r, i)| effective_precedence(revision, ordinal, r, i).is_lt()) {
                changed
                    .insert_admitted(identity, (revision, ordinal), admission)
                    .map_err(|e| e.to_string())?;
            }
        }
        let mut updates = RawPrefixChanges {
            removed: new_work(None),
            added: new_work(None),
            winner_edits: new_work(self.raw.budget()),
            raw_visits: prefix - self.prefix,
            coord_decodes: 0,
            rewound,
        };
        // 与 effective 一样先按身份序验证最终坐标，随后按坐标输出。
        for (&identity, &(revision, ordinal)) in changed.iter() {
            let c = match self.coordinates.get(&ordinal) {
                Some(c) => *c,
                None => {
                    super::history::check_deadline(deadline)?;
                    let event = self.raw.get(ordinal).expect("赢家原件");
                    if let Some(budget) = budget {
                        budget.record().map_err(|e| e.to_string())?;
                    }
                    let c = coord(event)?;
                    #[cfg(feature = "s_capture_native_testonly")]
                    let capacity = self.coordinates.capacity();
                    admission
                        .native_map_room(&mut self.facts.coordinates)
                        .map_err(|e| e.to_string())?;
                    self.coordinates
                        .insert(ordinal, c)
                        .map_err(|e| e.to_string())?;
                    #[cfg(feature = "s_capture_native_testonly")]
                    self.retained
                        .hash_capacity::<usize, i64>(capacity, self.facts.coordinates.capacity());
                    updates.coord_decodes += 1;
                    c
                }
            };
            let previous = match self.winners.get_mut(identity) {
                Some(winner) => Some(std::mem::replace(winner, (revision, ordinal, c))),
                None => {
                    let key = NativeValue::<String>::try_from_str(identity, admission.codec_work())
                        .map_err(|e| e.to_string())?;
                    #[cfg(feature = "s_capture_native_testonly")]
                    let capacity = self.winners.capacity();
                    #[cfg(feature = "s_capture_native_testonly")]
                    self.retained.add(&key);
                    admission
                        .native_map_room(&mut self.facts.winners)
                        .map_err(|e| e.to_string())?;
                    self.winners
                        .insert(key, (revision, ordinal, c))
                        .map_err(|e| e.to_string())?;
                    #[cfg(feature = "s_capture_native_testonly")]
                    self.retained
                        .hash_capacity::<NativeValue<String>, (i64, usize, i64)>(
                            capacity,
                            self.facts.winners.capacity(),
                        );
                    None
                }
            };
            admission
                .native_codec_room(&mut updates.winner_edits, 1)
                .map_err(|e| e.to_string())?;
            updates
                .winner_edits
                .try_push(RawWinnerEdit {
                    old: previous.map(|(_, ordinal, coord)| RawWinner { coord, ordinal }),
                    new: Some(RawWinner { coord: c, ordinal }),
                })
                .map_err(|e| e.to_string())?;
            if let Some((_, _, old)) = previous {
                #[cfg(feature = "s_capture_native_testonly")]
                let ordered_before = self.facts.ordered.allocation_bytes();
                self.ordered.remove(&old);
                if self.ordered.is_empty() {
                    self.ordered.clear();
                }
                #[cfg(feature = "s_capture_native_testonly")]
                self.ordered_retained(ordered_before);
                admission
                    .native_codec_room(&mut updates.removed, 1)
                    .map_err(|e| e.to_string())?;
                updates.removed.try_push(old).map_err(|e| e.to_string())?;
            }
            admission
                .native_codec_room(&mut updates.added, 1)
                .map_err(|e| e.to_string())?;
            updates
                .added
                .try_push((c, ordinal))
                .map_err(|e| e.to_string())?;
        }
        updates.added.sort_unstable_by_key(|(c, _)| *c);
        for &(c, ordinal) in &updates.added {
            #[cfg(feature = "s_capture_native_testonly")]
            let ordered_before = self.facts.ordered.allocation_bytes();
            if self
                .ordered
                .insert_admitted(c, ordinal, admission)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖业务身份".into());
            }
            #[cfg(feature = "s_capture_native_testonly")]
            self.ordered_retained(ordered_before);
        }
        self.prefix = prefix;
        Ok(updates)
    }
}

fn hex(bytes: &[u8; 32]) -> [u8; 64] {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = [0; 64];
    for (i, v) in bytes.iter().enumerate() {
        out[2 * i] = H[(v >> 4) as usize];
        out[2 * i + 1] = H[(v & 15) as usize];
    }
    out
}
pub(crate) fn sha_json(v: JsonView<'_>, b: Option<&OwnedBudget>) -> Result<[u8; 32], String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = b {
        let out = aj::canonical_bytes(v, b).map_err(|e| e.to_string())?;
        #[cfg(test)]
        super::state::trace_local_sha256(1, out.len());
        return Ok(Sha256::digest(&*out).into());
    }
    let _ = b;
    let out = serde_json::to_vec(&v).map_err(|e| e.to_string())?;
    #[cfg(test)]
    super::state::trace_local_sha256(1, out.len());
    Ok(Sha256::digest(&out).into())
}
/// Native 哈希直接消费同一 serde 规范字节，不建立未准入的中间 Vec。
pub(crate) fn sha_json_admitted(
    s: &(impl super::audit_store::CodecRead + ?Sized),
    value: JsonView<'_>,
) -> Result<[u8; 32], String> {
    if s.codec_work().is_none() {
        return sha_json(value, s.allocation_budget().as_ref());
    }
    struct Out {
        hash: Sha256,
        bytes: usize,
    }
    impl std::io::Write for Out {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.hash.update(bytes);
            self.bytes += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Out {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut out, &value).map_err(|e| e.to_string())?;
    #[cfg(test)]
    super::state::trace_local_sha256(1, out.bytes);
    Ok(out.hash.finalize().into())
}

#[allow(dead_code, reason = "#1527：保留原无计量入口，measured 入口共用原核")]
pub(crate) fn validate_raw(
    rows: RawEventsView<'_>,
    deadline: Option<Instant>,
) -> Result<(), String> {
    validate_raw_admitted(rows, deadline, None)
}
#[allow(dead_code, reason = "#1527：保留原无计量入口，measured 入口共用原核")]
pub(crate) fn validate_raw_admitted(
    rows: RawEventsView<'_>,
    deadline: Option<Instant>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    validate_raw_measured(rows, deadline, work, None)
}

pub(crate) fn validate_raw_measured(
    rows: RawEventsView<'_>,
    deadline: Option<Instant>,
    work: Option<&WorkScope>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<(), String> {
    check_native_owner(rows.budget(), work).map_err(|e| e.to_string())?;
    let admission = NativeAdmission(work);
    let mut owners = workmap::<i64, &str>(rows.budget());
    let mut positions = workmap::<&str, i64>(rows.budget());
    let mut prior = workmap::<&str, i64>(rows.budget());
    for (expected_seq, event) in rows.iter().enumerate() {
        check_read_deadline(deadline)?;
        if let Some(budget) = budget {
            budget.record().map_err(|e| e.to_string())?;
        }
        let c = coord(event)?;
        let identity = text(event, "identity_key")?;
        admission
            .native_map_room(&mut owners)
            .map_err(|e| e.to_string())?;
        admission
            .native_map_room(&mut positions)
            .map_err(|e| e.to_string())?;
        validate_raw_coordinate(
            identity,
            c,
            owners.insert(c, identity).map_err(|e| e.to_string())?,
            positions.insert(identity, c).map_err(|e| e.to_string())?,
        )?;
        let revision = parse_canonical_i64(text(event, "input_revision")?, "input_revision")
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        admission
            .native_map_room(&mut prior)
            .map_err(|e| e.to_string())?;
        let previous = prior
            .insert(identity, revision)
            .map_err(|e| e.to_string())?;
        validate_raw_row_core(event, expected_seq, previous, rows.budget(), work)?;
    }
    check_read_deadline(deadline)
}
/// 坐标双向归属共用谓词；调用者只提供已核映射的旧值。
pub(super) fn validate_raw_coordinate(
    identity: &str,
    coordinate: i64,
    previous_identity: Option<&str>,
    previous_coordinate: Option<i64>,
) -> Result<(), String> {
    if previous_identity.is_some_and(|old| old != identity)
        || previous_coordinate.is_some_and(|old| old != coordinate)
    {
        return Err("StorageUnavailable：原始源坐标与业务身份的唯一归属被破坏".into());
    }
    Ok(())
}
/// cold 与 owner 增量账共用：seq/修订前件由同一已核索引提供，内容谓词不另写一份。
#[cfg_attr(
    not(feature = "s_capture_native_testonly"),
    allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 下使用")
)]
pub(super) fn validate_raw_row(
    event: JsonView<'_>,
    expected_seq: usize,
    previous: Option<i64>,
    budget: Option<&OwnedBudget>,
) -> Result<(), String> {
    validate_raw_row_core(event, expected_seq, previous, budget, None)
}
#[cfg_attr(
    not(all(test, feature = "s_capture_native_testonly")),
    allow(dead_code, reason = "#1608：仅 s_capture_native_testonly 测试使用")
)]
pub(crate) fn validate_raw_row_admitted(
    event: JsonView<'_>,
    expected_seq: usize,
    previous: Option<i64>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    validate_raw_row_core(event, expected_seq, previous, None, work)
}
fn validate_raw_row_core(
    event: JsonView<'_>,
    expected_seq: usize,
    previous: Option<i64>,
    budget: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    validate_raw_row_typed(event, expected_seq, previous, budget, work).map_err(|e| match e {
        MixedJsonError::Business(e) => e,
        MixedJsonError::Resource(e) => e.to_string(),
    })
}
pub(crate) fn validate_raw_mixed(
    rows: &[PlainJson],
    work: &WorkScope,
    observer: &dyn super::audit_budget::LegacyObserver,
) -> Result<(), MixedJsonError> {
    observer
        .check_work(work)
        .map_err(MixedJsonError::Resource)?;
    let admission = NativeAdmission(Some(work));
    let mut owners = super::audit_store::NativeTree::default();
    let mut positions = super::audit_store::NativeTree::default();
    let mut prior = super::audit_store::NativeTree::default();
    for (expected_seq, record) in rows.iter().enumerate() {
        observer.precheck().map_err(MixedJsonError::Resource)?;
        observer
            .visit_record(
                "raw_history",
                "validate-raw",
                super::audit_budget::LegacyRecordRef::Json(record.as_ref()),
            )
            .map_err(MixedJsonError::Resource)?;
        let event: JsonView<'_> = record.as_ref().into();
        let coordinate = coord(event)?;
        let identity = text(event, "identity_key")?;
        let previous_identity = owners
            .insert_admitted(coordinate, identity, &admission)
            .map_err(MixedJsonError::Resource)?;
        let previous_coordinate = positions
            .insert_admitted(identity, coordinate, &admission)
            .map_err(MixedJsonError::Resource)?;
        validate_raw_coordinate(identity, coordinate, previous_identity, previous_coordinate)?;
        let revision = parse_canonical_i64(text(event, "input_revision")?, "input_revision")
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let previous = prior
            .insert_admitted(identity, revision, &admission)
            .map_err(MixedJsonError::Resource)?;
        validate_raw_row_typed(event, expected_seq, previous, None, Some(work))?;
    }
    observer.precheck().map_err(MixedJsonError::Resource)
}
fn validate_raw_row_typed(
    event: JsonView<'_>,
    expected_seq: usize,
    previous: Option<i64>,
    budget: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), MixedJsonError> {
    check_native_owner(budget, work).map_err(MixedJsonError::Resource)?;
    let identity = text(event, "identity_key")?;
    let price = text(event, "price")?;
    let volume = text(event, "volume")?;
    let ohlc = crate::tb02a::stored_prices_view(event)?;
    crate::tb02a::validate_prices_view(price, volume, ohlc)
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let input_rev = text(event, "input_revision")?;
    let revision = parse_canonical_i64(input_rev, "input_revision")
        .map_err(|e| format!("StorageUnavailable：{e}"))?;
    let mut fields = [("", JsonView::Null); 12];
    let mut used = 0;
    for (key, raw_key) in [
        ("event_id", "event_id"),
        ("revision", "input_revision"),
        ("seq", "source_coord"),
        ("received_at", "received_at"),
        ("raw_text", "raw_text"),
        ("timestamp", "ts"),
        ("volume", "volume"),
    ] {
        fields[used] = (key, field(event, raw_key));
        used += 1;
    }
    if let Some(p) = ohlc {
        for (k, v) in [
            ("open", p.open),
            ("high", p.high),
            ("low", p.low),
            ("close", p.close),
        ] {
            fields[used] = (k, JsonView::String(v));
            used += 1;
        }
    } else {
        fields[used] = ("price", JsonView::String(price));
        used += 1;
    }
    let content = JsonView::object_fields(&fields[..used]).map_err(|e| e.to_string())?;
    let payload = hex(&if work.is_some() {
        sha_json_admitted(&NativeAdmission(work), content)?
    } else {
        sha_json(content, budget)?
    });
    #[cfg(test)]
    super::state::trace_raw_hashes(false, 1);
    let payload = std::str::from_utf8(&payload).unwrap();
    let mut h = Sha256::new();
    h.update(identity.as_bytes());
    h.update(b"|");
    h.update(payload.as_bytes());
    #[cfg(test)]
    super::state::trace_local_sha256(1, identity.len() + 1 + payload.len());
    let receipt_hex = hex(&h.finalize().into());
    let mut receipt = [0; 21];
    receipt[..5].copy_from_slice(b"rcpt-");
    receipt[5..].copy_from_slice(&receipt_hex[..16]);
    let receipt = std::str::from_utf8(&receipt).unwrap();
    let ns = text(event, "source_namespace")?;
    let epoch = text(event, "source_epoch")?;
    let instrument = text(event, "instrument")?;
    let event_id = text(event, "event_id")?;
    let expected_id = ns
        .bytes()
        .chain([b'|'])
        .chain(epoch.bytes())
        .chain([b'|'])
        .chain(instrument.bytes())
        .chain([b'|'])
        .chain(event_id.bytes());
    let supersedes = field(event, "supersedes_revision");
    let sup_matches = match previous {
        None => supersedes.is_null(),
        Some(v) => supersedes.as_i64() == Some(v),
    };
    if field(event, "seq").as_i64() != i64::try_from(expected_seq).ok()
        || revision < 1
        || field(event, "revision").as_i64() != Some(revision)
        || previous.is_some_and(|p| p >= revision)
        || !sup_matches
        || !identity.bytes().eq(expected_id)
        || text(event, "payload_hash")? != payload
        || text(event, "receipt_id")? != receipt
    {
        return Err("StorageUnavailable：原始身份/接纳序/修订链/内容hash/receipt矛盾".into());
    }
    for (name, value) in [
        ("price", price),
        ("timestamp", text(event, "ts")?),
        ("volume", volume),
    ] {
        parse_canonical_i64(value, name).map_err(|e| format!("StorageUnavailable：{e}"))?;
    }
    Ok(())
}

#[cfg(feature = "s_allocation_json_testonly")]
pub(crate) fn read_owned(
    conn: &Connection,
    deadline: Option<Instant>,
    b: &OwnedBudget,
) -> Result<RawEvents, String> {
    check_read_deadline(deadline)?;
    let mut tree = aj::OwnedJson::array(b).map_err(|e| e.to_string())?;
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace,source_epoch, instrument, event_id, received_at, raw_text, price,ts, volume, source_coord, supersedes_revision FROM raw_events ORDER BY seq ASC").map_err(|e|format!("prepare 读取失败：{e}"))?;
    while let Some(mut row) = stmt.next().map_err(|e| format!("collect 失败：{e}"))? {
        check_read_deadline(deadline)?;
        tree.append_object(|o| {
            for (i, k) in [
                (0, "identity_key"),
                (2, "input_revision"),
                (3, "payload_hash"),
                (4, "receipt_id"),
                (6, "source_namespace"),
                (7, "source_epoch"),
                (8, "instrument"),
                (9, "event_id"),
                (10, "received_at"),
                (11, "raw_text"),
                (12, "price"),
                (13, "ts"),
                (14, "volume"),
                (15, "source_coord"),
            ] {
                let value = row.text(i).map_err(|e| {
                    sc::CodecError::new("StorageUnavailable", format!("SQL raw文本列读取失败：{e}"))
                })?;
                o.insert_str(k, value)?;
            }
            for (i, k) in [(1, "revision"), (5, "seq")] {
                o.insert_i64(
                    k,
                    row.integer(i).map_err(|e| {
                        sc::CodecError::new(
                            "StorageUnavailable",
                            format!("SQL raw整数列读取失败：{e}"),
                        )
                    })?,
                )?;
            }
            match row.optional_integer(16).map_err(|e| {
                sc::CodecError::new(
                    "StorageUnavailable",
                    format!("SQL raw supersedes列读取失败：{e}"),
                )
            })? {
                Some(v) => o.insert_i64("supersedes_revision", v)?,
                None => o.insert_null("supersedes_revision")?,
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    }
    stmt.finalize()
        .map_err(|e| format!("finalize 读取失败：{e}"))?;
    crate::tb02a::enrich_raw_owned(conn, &mut tree, deadline)?;
    let result = RawEvents::Owned(tree);
    validate_raw(result.view(), deadline)?;
    Ok(result)
}

pub(crate) struct Decimal {
    bytes: [u8; 20],
    len: usize,
}
impl std::fmt::Write for Decimal {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
impl Decimal {
    pub(crate) fn new(v: i64) -> Self {
        use std::fmt::Write;
        let mut out = Self {
            bytes: [0; 20],
            len: 0,
        };
        write!(&mut out, "{v}").expect("i64 at most20 ASCII bytes");
        out
    }
    pub(crate) fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
}
/// 撤回 wire 的唯一投影核；typed 保留完整原件，shape 只将三个整数变成规范文本。
/// 借用的子值与栈内十进制文本不制造临时 JSON 树；数组工作集仍由调用者实际账本持有。
pub(crate) struct WithdrawalWire<'a> {
    original: JsonView<'a>,
    windows: Option<[Decimal; 3]>,
}
impl<'a> WithdrawalWire<'a> {
    pub(crate) fn new(original: JsonView<'a>) -> Result<Self, String> {
        let windows = if original.get("fact_key").is_some() {
            None
        } else {
            let number = |key| {
                field(original, key)
                    .as_i64()
                    .map(Decimal::new)
                    .ok_or_else(|| "持久整数必须是 i64，不能是浮点、布尔、空值或文本".to_owned())
            };
            Some([
                number("window_start")?,
                number("window_mid")?,
                number("window_end")?,
            ])
        };
        Ok(Self { original, windows })
    }
    pub(crate) fn view(&self) -> JsonView<'_> {
        if self.windows.is_none() {
            return self.original;
        }
        JsonView::borrowed_object(self).expect("撤回 wire 六个固定字段唯一")
    }
}
impl aj::JsonObject for WithdrawalWire<'_> {
    fn len(&self) -> usize {
        6
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        let windows = self.windows.as_ref()?;
        Some(match index {
            0 => ("object_id", field(self.original, "object_id")),
            1 => ("reason", field(self.original, "reason")),
            2 => ("superseded_by", field(self.original, "superseded_by")),
            3 => ("window_end", JsonView::String(windows[2].text())),
            4 => ("window_mid", JsonView::String(windows[1].text())),
            5 => ("window_start", JsonView::String(windows[0].text())),
            _ => return None,
        })
    }
}
pub(crate) struct WithdrawalArray<'a>(pub sc::CodecVec<WithdrawalWire<'a>>);
impl WithdrawalArray<'_> {
    pub(crate) fn view(&self) -> JsonView<'_> {
        JsonView::BorrowedArray(self)
    }
}
impl aj::JsonArray for WithdrawalArray<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(WithdrawalWire::view)
    }
}
/// 源引用wire/numeric共用字段投影，只让revision/seq的值出现差异。
pub(crate) struct RawReference<'a> {
    event: JsonView<'a>,
    revision: Decimal,
    seq: Decimal,
    numeric: bool,
}
impl<'a> RawReference<'a> {
    pub fn new(event: JsonView<'a>, numeric: bool) -> Result<Self, String> {
        Ok(Self {
            event,
            revision: Decimal::new(
                field(event, "revision")
                    .as_i64()
                    .ok_or("StorageUnavailable：revision非i64")?,
            ),
            seq: Decimal::new(
                field(event, "seq")
                    .as_i64()
                    .ok_or("StorageUnavailable：seq非i64")?,
            ),
            numeric,
        })
    }
    pub fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("fixed unique raw-reference keys")
    }
}
impl aj::JsonObject for RawReference<'_> {
    fn len(&self) -> usize {
        8
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        let k = *[
            "identity_key",
            "payload_hash",
            "receipt_id",
            "event_id",
            "input_revision",
            "revision",
            "seq",
            "source_coord",
        ]
        .get(index)?;
        let v = match k {
            "revision" if !self.numeric => JsonView::String(self.revision.text()),
            "seq" if !self.numeric => JsonView::String(self.seq.text()),
            _ => field(self.event, k),
        };
        Some((k, v))
    }
}
pub(crate) struct RawWire<'a> {
    event: JsonView<'a>,
    revision: Decimal,
    seq: Decimal,
    supersedes: Option<Decimal>,
    ohlc: bool,
}
impl<'a> RawWire<'a> {
    pub fn new(event: JsonView<'a>) -> Result<Self, String> {
        Ok(Self {
            event,
            revision: Decimal::new(
                field(event, "revision")
                    .as_i64()
                    .ok_or("StorageUnavailable：revision非i64")?,
            ),
            seq: Decimal::new(
                field(event, "seq")
                    .as_i64()
                    .ok_or("StorageUnavailable：seq非i64")?,
            ),
            supersedes: field(event, "supersedes_revision")
                .as_i64()
                .map(Decimal::new),
            ohlc: field(event, "schema_revision").as_str() == Some(tb02a::RAW_SCHEMA),
        })
    }
    pub fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("fixed unique raw-wire keys")
    }
}
impl aj::JsonObject for RawWire<'_> {
    fn len(&self) -> usize {
        if self.ohlc {
            21
        } else {
            17
        }
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        const BASE: [&str; 17] = [
            "identity_key",
            "revision",
            "input_revision",
            "payload_hash",
            "receipt_id",
            "seq",
            "source_namespace",
            "source_epoch",
            "instrument",
            "event_id",
            "received_at",
            "raw_text",
            "price",
            "ts",
            "volume",
            "source_coord",
            "supersedes_revision",
        ];
        let k = if self.ohlc {
            let mut position = 0;
            let mut found = None;
            for name in BASE.iter().copied().filter(|k| *k != "price").chain([
                "schema_revision",
                "open",
                "high",
                "low",
                "close",
            ]) {
                if position == index {
                    found = Some(name);
                    break;
                }
                position += 1;
            }
            found?
        } else {
            *BASE.get(index)?
        };
        let v = match k {
            "revision" => JsonView::String(self.revision.text()),
            "seq" => JsonView::String(self.seq.text()),
            "supersedes_revision" => self
                .supersedes
                .as_ref()
                .map(|v| JsonView::String(v.text()))
                .unwrap_or(JsonView::Null),
            _ => field(self.event, k),
        };
        Some((k, v))
    }
}
pub(crate) struct ReferenceArray<'a>(pub sc::CodecVec<RawReference<'a>>);
impl aj::JsonArray for ReferenceArray<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(RawReference::view)
    }
}
#[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
pub(crate) struct ValuesArray<'a>(pub &'a [Value]);
impl aj::JsonArray for ValuesArray<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(JsonView::Plain)
    }
}

impl aj::JsonArray for RawEventsView<'_> {
    fn len(&self) -> usize {
        self.len
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        (*self).get(index)
    }
}

impl serde::Serialize for RawReference<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.view().serialize(s)
    }
}

/// 新业务返回文本的入口存储选择。Owned实际字节持续持账；Clone只开放fallible深复制。
pub(crate) enum TextValue {
    Plain(NativeValue<String>),
    #[cfg_attr(not(feature = "s_allocation_json_testonly"), allow(dead_code))]
    Owned(aj::TextOwner),
}
impl TextValue {
    pub fn from_str_admitted(
        text: &str,
        owner: Option<&OwnedBudget>,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        Self::from_str_admitted_typed(text, owner, work).map_err(|e| e.to_string())
    }
    pub fn from_str_admitted_typed(
        text: &str,
        owner: Option<&OwnedBudget>,
        work: Option<&WorkScope>,
    ) -> Result<Self, sc::CodecError> {
        check_native_owner(owner, work)?;
        if work.is_some() {
            return NativeAdmission(work).native_string(text).map(Self::Plain);
        }
        Self::from_str_typed(text, owner)
    }
    pub fn from_sql_admitted(
        row: &mut CheckedRow<'_>,
        column: usize,
        owner: Option<&OwnedBudget>,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        let text = row.text(column).map_err(|e| e.to_string())?;
        Self::from_str_admitted(text, owner, work)
    }
    pub fn optional_sql_admitted(
        row: &mut CheckedRow<'_>,
        column: usize,
        owner: Option<&OwnedBudget>,
        work: Option<&WorkScope>,
    ) -> Result<Option<Self>, String> {
        check_native_owner(owner, work).map_err(|e| e.to_string())?;
        row.optional_text(column)
            .map_err(|e| e.to_string())?
            .map(|text| Self::from_str_admitted(text, owner, work))
            .transpose()
    }
    pub(crate) fn try_clone_mixed(&self, work: &WorkScope) -> Result<Self, sc::CodecError> {
        match self {
            Self::Plain(value) => {
                value.check_retained_source()?;
                value.try_clone_admitted(work).map(Self::Plain)
            }
            Self::Owned(_) => {
                Err(work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger))
            }
        }
    }
    pub fn try_clone_admitted(&self, work: Option<&WorkScope>) -> Result<Self, String> {
        if let Some(work) = work {
            if matches!(self, Self::Owned(_)) {
                return Err(work
                    .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)
                    .to_string());
            }
            return Self::from_str_admitted(self.as_str(), None, Some(work));
        }
        self.try_clone()
    }
    pub fn from_owned_bytes(bytes: sc::CodecVec<u8>) -> Result<Self, String> {
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            return aj::TextOwner::from_owned_bytes(bytes)
                .map(Self::Owned)
                .map_err(|e| e.to_string());
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = bytes;
            Err("prototype_audit_allocation_incomplete：Owned文本入口未启用".into())
        }
    }
    /// 存储选择来自传入真实Vec owner；Owned错误直接返回，不回普通分支。
    pub fn from_bytes(bytes: sc::CodecVec<u8>) -> Result<Self, String> {
        if bytes.allocation_budget().is_some() {
            return Self::from_owned_bytes(bytes);
        }
        // 保留原 String::from_utf8 的诊断文本；借用验证不拆所有权、不复制。
        std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        bytes
            .try_into_native_text()
            .map(Self::Plain)
            .map_err(|e| e.to_string())
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Owned(s) => s.as_str(),
        }
    }
    pub fn capacity(&self) -> usize {
        match self {
            Self::Plain(s) => s.capacity(),
            Self::Owned(s) => s.capacity(),
        }
    }
    pub fn try_clone(&self) -> Result<Self, String> {
        match self {
            Self::Plain(s) => {
                if s.is_native_admitted() {
                    return Err(
                        "prototype_audit_allocation_incomplete：Native文本复制需要当前WorkScope"
                            .into(),
                    );
                }
                Self::from_str(s, None)
            }
            Self::Owned(s) => {
                #[cfg(feature = "s_allocation_json_testonly")]
                {
                    Ok(Self::Owned(s.try_clone().map_err(|e| e.to_string())?))
                }
                #[cfg(not(feature = "s_allocation_json_testonly"))]
                {
                    let _ = s;
                    Err("prototype_audit_allocation_incomplete：owned text clone未启用".into())
                }
            }
        }
    }
    pub fn from_str(s: &str, b: Option<&OwnedBudget>) -> Result<Self, String> {
        Self::from_str_typed(s, b).map_err(|e| {
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            if b.is_some() {
                return "prototype_audit_allocation_incomplete：Owned文本入口未启用".into();
            }
            e.to_string()
        })
    }
    pub fn from_str_typed(s: &str, b: Option<&OwnedBudget>) -> Result<Self, sc::CodecError> {
        #[cfg(feature = "s_allocation_json_testonly")]
        if let Some(b) = b {
            return aj::TextOwner::copy_from(s, b).map(Self::Owned);
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        if b.is_some() {
            return Err(sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Owned文本入口未启用",
            ));
        }
        let mut value = String::new();
        value.try_reserve_exact(s.len()).map_err(|_| {
            sc::CodecError::new(sc::capture_allocation::LIMIT, "Native文本分配失败")
        })?;
        value.push_str(s);
        Ok(Self::Plain(NativeValue::<String>::from_unscoped(value)))
    }
    pub fn from_sql(
        row: &mut CheckedRow<'_>,
        i: usize,
        b: Option<&OwnedBudget>,
    ) -> Result<Self, String> {
        let value = row.text(i).map_err(|e| e.to_string())?;
        Self::from_str(value, b)
    }
    #[cfg_attr(
        not(all(test, feature = "s_allocation_json_testonly")),
        allow(dead_code, reason = "#1488：仅 JSON testonly 测试调用")
    )]
    pub fn optional_sql(
        row: &mut CheckedRow<'_>,
        i: usize,
        b: Option<&OwnedBudget>,
    ) -> Result<Option<Self>, String> {
        row.optional_text(i)
            .map_err(|e| e.to_string())?
            .map(|value| Self::from_str(value, b))
            .transpose()
    }
}
impl std::borrow::Borrow<str> for TextValue {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl std::ops::Deref for TextValue {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Debug for TextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl std::fmt::Display for TextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl serde::Serialize for TextValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl PartialEq for TextValue {
    fn eq(&self, o: &Self) -> bool {
        self.as_str() == o.as_str()
    }
}
impl Eq for TextValue {}
impl PartialEq<str> for TextValue {
    fn eq(&self, o: &str) -> bool {
        self.as_str() == o
    }
}
impl PartialEq<&str> for TextValue {
    fn eq(&self, o: &&str) -> bool {
        self.as_str() == *o
    }
}
impl PartialEq<String> for TextValue {
    fn eq(&self, o: &String) -> bool {
        self.as_str() == o
    }
}
impl PartialOrd for TextValue {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for TextValue {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(o.as_str())
    }
}
impl std::hash::Hash for TextValue {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}
impl From<String> for TextValue {
    fn from(s: String) -> Self {
        Self::Plain(NativeValue::<String>::from_unscoped(s))
    }
}
impl From<&str> for TextValue {
    fn from(s: &str) -> Self {
        Self::Plain(NativeValue::<String>::from_unscoped(s.to_owned()))
    }
}

/// HTTP和域请求共享同一个strict输入；clone只增加真实共享owner的引用计数。
#[derive(Clone)]
pub(crate) struct SharedEnvelope(sc::capture_allocation::dispatch::DispatchArc<JsonOwner>);
impl SharedEnvelope {
    pub fn try_new(
        owner: JsonOwner,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, (sc::CodecError, JsonOwner)> {
        sc::capture_allocation::dispatch::DispatchArc::try_new(owner, allocation).map(Self)
    }
    pub fn view(&self) -> JsonView<'_> {
        self.0.view()
    }
    pub fn budget(&self) -> Option<&OwnedBudget> {
        match &*self.0 {
            JsonOwner::Plain(_) => None,
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(v) => Some(v.budget()),
        }
    }
}
impl std::ops::Deref for SharedEnvelope {
    type Target = JsonOwner;
    fn deref(&self) -> &JsonOwner {
        &self.0
    }
}
/// #1488 bin 的严格解析拥有者。解析准入发生在构造前；payload 先析构，许可随后。
/// 与库 NativeValue<Value> 的只读复制用途相同，但这里须承接本 bin JsonShape/strict
/// visitor 的原解析许可，不能先解析再复制补签。私有字段不开放 take 或 DerefMut。
#[derive(Clone, Copy, PartialEq, Eq)]
enum NativeJsonLayout {
    AggregateTree,
    ItemizedArray,
    ItemizedObject,
}
pub(crate) struct PlainJson {
    value: Value,
    permit: Option<NativeReservation>,
    layout: NativeJsonLayout,
    // 数组子树的原许可；CodecVec 自持 metadata buffer 许可。payload 必须先析构。
    child_permits: sc::CodecVec<NativeReservation>,
}
impl PlainJson {
    fn from_reserved(value: Value, permit: Option<NativeReservation>) -> Self {
        Self {
            value,
            permit,
            layout: NativeJsonLayout::AggregateTree,
            child_permits: sc::CodecVec::default(),
        }
    }
    #[cfg_attr(
        not(feature = "s_capture_native_testonly"),
        allow(
            dead_code,
            reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
        )
    )]
    pub(crate) fn with_reservation(mut self, permit: Option<NativeReservation>) -> Self {
        assert!(self.permit.is_none());
        self.permit = permit;
        self
    }
    // 只供ScopedJsonArray空数组/空替换buffer构造；每个子树许可由push逐项携入。
    fn itemized_array(permit: NativeReservation, rows: Vec<Value>) -> Self {
        debug_assert!(rows.is_empty());
        Self {
            value: Value::Array(rows),
            permit: Some(permit),
            layout: NativeJsonLayout::ItemizedArray,
            child_permits: sc::CodecVec::default(),
        }
    }
    pub(crate) fn as_ref(&self) -> &Value {
        &self.value
    }
    pub(crate) fn is_native_admitted(&self) -> bool {
        self.permit.is_some()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(crate) fn retained_heap_bytes(&self) -> usize {
        use super::audit_retention::Retained;
        let admitted = self.child_permits.iter().fold(
            self.permit.as_ref().map_or(0, NativeReservation::bytes),
            |n, p| n.saturating_add(p.bytes()),
        );
        self.value.heap_bytes().max(admitted).saturating_add(
            self.child_permits
                .capacity()
                .saturating_mul(std::mem::size_of::<NativeReservation>()),
        )
    }
    fn unscoped_mut(&mut self) -> Result<&mut Value, String> {
        if self.permit.is_some() {
            return Err(
                "prototype_audit_allocation_incomplete：Native JSON修改需要当前WorkScope".into(),
            );
        }
        Ok(&mut self.value)
    }
    pub(crate) fn into_unscoped(self) -> Result<Value, String> {
        if self.permit.is_some() {
            return Err(
                "prototype_audit_allocation_incomplete：Native JSON不可拆出无许可载荷".into(),
            );
        }
        Ok(self.value)
    }
    pub(crate) fn check_retained_source(&self) -> Result<(), sc::CodecError> {
        if let Some(permit) = &self.permit {
            permit.check_retained_source()?;
        }
        self.child_permits.check_retained_source()?;
        for permit in &self.child_permits {
            permit.check_retained_source()?;
        }
        Ok(())
    }
    pub(crate) fn try_clone_mixed(&self, work: &WorkScope) -> Result<Self, sc::CodecError> {
        self.check_retained_source()?;
        Self::try_clone_from(self.as_ref(), Some(work))
    }
    pub(crate) fn try_clone_from(
        value: &Value,
        work: Option<&WorkScope>,
    ) -> Result<Self, sc::CodecError> {
        fn bytes(value: &Value) -> Option<usize> {
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
        let admission = NativeAdmission(work);
        let amount = bytes(value).ok_or_else(|| admission.native_overflow())?;
        let permit = work.map(|w| w.reserve_native(amount)).transpose()?;
        Ok(Self::from_reserved(value.clone(), permit))
    }
}
impl From<Value> for PlainJson {
    /// 仅无scope既有值的所有权转交，不复制、不授予已准入资格。
    fn from(value: Value) -> Self {
        Self::from_reserved(value, None)
    }
}
impl std::ops::Deref for PlainJson {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.value
    }
}
impl std::fmt::Debug for PlainJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
impl serde::Serialize for PlainJson {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(s)
    }
}

pub(crate) enum JsonOwner {
    Plain(PlainJson),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(aj::OwnedJson),
}
impl serde::Serialize for JsonOwner {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.view().serialize(serializer)
    }
}
impl PartialEq for JsonOwner {
    fn eq(&self, other: &Self) -> bool {
        self.view() == other.view()
    }
}
impl std::fmt::Debug for JsonOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.view(), f)
    }
}
impl JsonOwner {
    pub fn view(&self) -> JsonView<'_> {
        match self {
            Self::Plain(v) => v.as_ref().into(),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => v.view(),
        }
    }
    pub(crate) fn array_len(&self) -> usize {
        self.view().as_array().expect("变化拥有者须为数组").len()
    }
    pub(crate) fn object(allocation: Option<&OwnedBudget>) -> Result<Self, String> {
        #[cfg(feature = "s_allocation_json_testonly")]
        if let Some(b) = allocation {
            return aj::OwnedJson::object(b)
                .map(Self::Owned)
                .map_err(|e| e.to_string());
        }
        let _ = allocation;
        Ok(Self::Plain(Value::Object(serde_json::Map::new()).into()))
    }
    pub(crate) fn array(allocation: Option<&OwnedBudget>) -> Result<Self, String> {
        #[cfg(feature = "s_allocation_json_testonly")]
        if let Some(b) = allocation {
            return aj::OwnedJson::array(b)
                .map(Self::Owned)
                .map_err(|e| e.to_string());
        }
        let _ = allocation;
        Ok(Self::Plain(Value::Array(Vec::new()).into()))
    }
    pub(crate) fn insert(&mut self, key: &str, value: JsonView<'_>) -> Result<(), String> {
        match self {
            Self::Plain(v) => {
                v.unscoped_mut()?
                    .as_object_mut()
                    .ok_or("JSON非对象")?
                    .insert(
                        key.into(),
                        serde_json::to_value(value).map_err(|e| e.to_string())?,
                    );
                Ok(())
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => v.insert_field(key, value).map_err(|e| e.to_string()),
        }
    }
    pub(crate) fn push(&mut self, value: JsonView<'_>) -> Result<(), String> {
        match self {
            Self::Plain(v) => {
                v.unscoped_mut()?
                    .as_array_mut()
                    .ok_or("JSON非数组")?
                    .push(serde_json::to_value(value).map_err(|e| e.to_string())?);
                Ok(())
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => v.push(value).map_err(|e| e.to_string()),
        }
    }
}
/// #1488 D1：固定一个 WorkScope 的数组构造器，只从借用 view 独立复制。
/// 没有接入任意 PlainJson/裸 Value 的入口，也没有拆出无许可 payload 的出口。
pub(crate) struct ScopedJsonArray<'work> {
    owner: JsonOwner,
    work: Option<&'work WorkScope>,
}
impl<'work> ScopedJsonArray<'work> {
    /// 生产 Native 调用显式要求活动 scope；不接受 Option 形成缺省逃逸。
    pub(crate) fn native(work: &'work WorkScope) -> Result<Self, String> {
        Self::new(None, Some(work))
    }
    pub(crate) fn owned(allocation: &OwnedBudget) -> Result<Self, String> {
        Self::new(Some(allocation), None)
    }
    /// 既有无 scope 兼容路径，不授予 Native 资格。
    pub(crate) fn unscoped() -> Self {
        Self {
            owner: JsonOwner::Plain(Value::Array(Vec::new()).into()),
            work: None,
        }
    }
    pub(crate) fn new(
        allocation: Option<&OwnedBudget>,
        work: Option<&'work WorkScope>,
    ) -> Result<Self, String> {
        check_native_owner(allocation, work).map_err(|e| e.to_string())?;
        let owner = if let Some(work) = work {
            let permit = work.reserve_native(0).map_err(|e| e.to_string())?;
            JsonOwner::Plain(PlainJson::itemized_array(permit, Vec::new()))
        } else {
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            if allocation.is_some() {
                return Err("prototype_audit_allocation_incomplete：Owned JSON入口未启用".into());
            }
            JsonOwner::array(allocation)?
        };
        Ok(Self { owner, work })
    }

    pub(crate) fn push(&mut self, value: JsonView<'_>) -> Result<(), String> {
        let Some(work) = self.work else {
            return self.owner.push(value);
        };
        #[cfg_attr(
            not(feature = "s_allocation_json_testonly"),
            allow(irrefutable_let_patterns)
        )]
        let JsonOwner::Plain(parent) = &mut self.owner
        else {
            return Err(work
                .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)
                .to_string());
        };
        push_native_array(parent, value, work)
    }

    pub(crate) fn finish(self) -> JsonOwner {
        self.owner
    }
}

/// 复用ScopedJsonArray的失败原子增长核，同父跨work允许，外父在准入前拒绝。
fn push_native_array(
    parent: &mut PlainJson,
    value: JsonView<'_>,
    work: &WorkScope,
) -> Result<(), String> {
    if parent.layout != NativeJsonLayout::ItemizedArray {
        return Err("prototype_audit_allocation_incomplete：追加不接受聚合Native数组许可".into());
    }
    let permit = parent
        .permit
        .as_ref()
        .ok_or_else(|| "prototype_audit_allocation_incomplete：数组缺原Native许可".to_string())?;
    if !permit.belongs_to_parent(work) {
        return Err("prototype_audit_allocation_incomplete：数组与追加域不是同父账".into());
    }
    // 父身份不等于操作资格；失败/失效work仍由原check_write拒绝。
    let _operation = work.reserve_native(0).map_err(|e| e.to_string())?;
    let rows = parent.value.as_array().expect("ScopedJsonArray只构造数组");
    let needed = rows
        .len()
        .checked_add(1)
        .ok_or_else(|| NativeAdmission(Some(work)).native_overflow().to_string())?;
    let grow = |capacity: usize| -> Result<usize, String> {
        Ok(needed
            .max(
                capacity
                    .checked_mul(2)
                    .ok_or_else(|| NativeAdmission(Some(work)).native_overflow().to_string())?,
            )
            .max(4))
    };

    // 三个准备件各自持费。任何失败都只销毁准备件，父载荷/容量/原许可不变。
    // 新旧完整 buffer 同时持费；不能先给父 reserve 再遇 child 拒绝而留下增长。
    let mut replacement = if needed > rows.capacity() {
        let capacity = grow(rows.capacity())?;
        let bytes = capacity
            .checked_mul(std::mem::size_of::<Value>())
            .ok_or_else(|| NativeAdmission(Some(work)).native_overflow().to_string())?;
        let permit = work.reserve_native(bytes).map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(capacity).map_err(|_| {
            work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation)
                .to_string()
        })?;
        Some(PlainJson::itemized_array(permit, rows))
    } else {
        None
    };
    let mut metadata = if needed > parent.child_permits.capacity() {
        let mut permits = sc::CodecVec::default();
        permits
            .try_reserve_native(work, grow(parent.child_permits.capacity())?)
            .map_err(|e| e.to_string())?;
        Some(permits)
    } else {
        None
    };
    let child = json_from_view_admitted(&NativeAdmission(Some(work)), value)?;
    #[cfg_attr(
        not(feature = "s_allocation_json_testonly"),
        allow(irrefutable_let_patterns)
    )]
    let JsonOwner::Plain(mut child) = child
    else {
        return Err(work
            .reject(sc::capture_allocation::scratch::ScratchFailure::Ledger)
            .to_string());
    };
    // 只有上面的固定 NativeAdmission 能产生该 child，未导入任何外来 owner。
    debug_assert!(child.permit.is_some() && child.child_permits.is_empty());

    // 至此所有可能失败的准入/分配已完成。以下只在预留空间内移动。
    if let Some(mut replacement) = replacement.take() {
        replacement
            .value
            .as_array_mut()
            .unwrap()
            .append(parent.value.as_array_mut().unwrap());
        std::mem::swap(&mut parent.value, &mut replacement.value);
        std::mem::swap(&mut parent.permit, &mut replacement.permit);
        drop(replacement); // 旧空 buffer 先销毁，再退其原许可。
    }
    if let Some(mut replacement) = metadata.take() {
        let old = std::mem::take(&mut parent.child_permits);
        for permit in old {
            replacement.try_push(permit).expect("已准入metadata容量");
        }
        parent.child_permits = replacement;
    }
    let permit = child.permit.take().expect("同scope复制持原许可");
    let value = std::mem::replace(&mut child.value, Value::Null);
    parent.value.as_array_mut().unwrap().push(value);
    parent
        .child_permits
        .try_push(permit)
        .expect("已准入metadata容量");
    Ok(())
}

pub(crate) fn json_from_view(
    value: JsonView<'_>,
    allocation: Option<&OwnedBudget>,
) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = allocation {
        return aj::OwnedJson::from_view(value, b)
            .map(JsonOwner::Owned)
            .map_err(|e| e.to_string());
    }
    let _ = allocation;
    serde_json::to_value(value)
        .map(|v| JsonOwner::Plain(PlainJson::from(v)))
        .map_err(|e| e.to_string())
}
/// #1488：先计算与旧递归构造相同的结构上界，再构造；原许可随结果销毁。
pub(crate) fn json_from_view_admitted(
    s: &(impl super::audit_store::CodecRead + ?Sized),
    value: JsonView<'_>,
) -> Result<JsonOwner, String> {
    if s.codec_work().is_none() {
        return json_from_view(value, s.allocation_budget().as_ref());
    }
    fn bytes(value: JsonView<'_>) -> Option<usize> {
        if let Some(text) = value.as_str() {
            return Some(text.len());
        }
        if let Some(array) = value.as_array() {
            let buffer = if array.len() == 0 {
                0
            } else {
                array.len().max(4)
            };
            return array
                .iter()
                .try_fold(buffer.checked_mul(std::mem::size_of::<Value>())?, |n, v| {
                    n.checked_add(bytes(v)?)
                });
        }
        if let Some(object) = value.as_object() {
            let mut total = 0usize;
            for (len, (key, child)) in object.iter().enumerate() {
                let depth = usize::BITS as usize - len.leading_zeros() as usize + 2;
                total = total
                    .checked_add(
                        (11 * std::mem::size_of::<(String, Value)>()
                            + 16 * std::mem::size_of::<usize>())
                        .checked_mul(depth)?,
                    )?
                    .checked_add(key.len())?
                    .checked_add(bytes(child)?)?;
            }
            return Some(total);
        }
        Some(0)
    }
    fn plain(value: JsonView<'_>) -> Result<Value, String> {
        if let Some(text) = value.as_str() {
            return Ok(Value::String(text.to_owned()));
        }
        if let Some(array) = value.as_array() {
            let mut rows = Vec::new();
            if array.len() != 0 {
                rows.try_reserve_exact(array.len().max(4))
                    .map_err(|_| "Native JSON数组分配失败")?;
            }
            for child in array.iter() {
                rows.push(plain(child)?);
            }
            return Ok(Value::Array(rows));
        }
        if let Some(object) = value.as_object() {
            let mut fields = serde_json::Map::new();
            for (key, child) in object.iter() {
                fields.insert(key.to_owned(), plain(child)?);
            }
            return Ok(Value::Object(fields));
        }
        serde_json::to_value(value).map_err(|e| e.to_string())
    }
    let amount = bytes(value).ok_or_else(|| s.native_overflow().to_string())?;
    let permit = s
        .codec_work()
        .unwrap()
        .reserve_native(amount)
        .map_err(|e| e.to_string())?;
    plain(value).map(|v| JsonOwner::Plain(PlainJson::from_reserved(v, Some(permit))))
}

/// 旧 wire 整数投影的共同核：只转换整数，浮点/空值/文本保持原值。
pub(crate) fn project_wire_integers_with_owner(
    value: JsonView<'_>,
    allocation: Option<&OwnedBudget>,
) -> Result<JsonOwner, String> {
    if let Some(number) = value.number() {
        if let Some(i) = number.as_i64() {
            let decimal = Decimal::new(i);
            return json_from_view(JsonView::String(decimal.text()), allocation);
        }
        if let Some(u) = number.as_u64() {
            use std::fmt::Write;
            let mut decimal = Decimal {
                bytes: [0; 20],
                len: 0,
            };
            write!(&mut decimal, "{u}").expect("u64最多20个ASCII字节");
            return json_from_view(JsonView::String(decimal.text()), allocation);
        }
    }
    if let Some(values) = value.as_array() {
        let mut out = JsonOwner::array(allocation)?;
        for v in values.iter() {
            let projected = project_wire_integers_with_owner(v, allocation)?;
            out.push(projected.view())?;
        }
        return Ok(out);
    }
    if let Some(fields) = value.as_object() {
        let mut out = JsonOwner::object(allocation)?;
        for (key, v) in fields.iter() {
            let projected = project_wire_integers_with_owner(v, allocation)?;
            out.insert(key, projected.view())?;
        }
        return Ok(out);
    }
    json_from_view(value, allocation)
}
/// 同一读取/渲染/严格解码核；预算随实际store选择，拒绝后没有Plain分支。
pub(crate) fn render_json(
    s: &mut (impl super::audit_store::CodecRead + ?Sized),
    r: sc::RefView<'_>,
) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = s.allocation_budget() {
        struct Out {
            bytes: sc::CodecVec<u8>,
            error: Option<sc::CodecError>,
        }
        impl std::io::Write for Out {
            fn write(&mut self, v: &[u8]) -> std::io::Result<usize> {
                match aj::extend_bytes(&mut self.bytes, v) {
                    Ok(()) => Ok(v.len()),
                    Err(e) => {
                        self.error = Some(e);
                        Err(std::io::ErrorKind::Other.into())
                    }
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut out = Out {
            bytes: aj::owned_vec(&b),
            error: None,
        };
        let result = s.render_ref(r, &mut out);
        if let Some(e) = out.error {
            return Err(e.to_string());
        }
        result.map_err(|e| e.to_string())?;
        return aj::OwnedJson::parse(&out.bytes, &b)
            .map(JsonOwner::Owned)
            .map_err(|e| e.to_string());
    }
    if let Some(work) = s.codec_work() {
        struct ScratchOut {
            bytes: sc::capture_allocation::scratch::ScratchBytes,
            error: Option<sc::CodecError>,
        }
        impl std::io::Write for ScratchOut {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                match self.bytes.try_extend_bytes(bytes) {
                    Ok(()) => Ok(bytes.len()),
                    Err(e) => {
                        self.error = Some(e);
                        Err(std::io::ErrorKind::Other.into())
                    }
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut out = ScratchOut {
            bytes: work.bytes(),
            error: None,
        };
        let result = s.render_ref(r, &mut out);
        if let Some(e) = out.error {
            return Err(e.to_string());
        }
        result.map_err(|e| e.to_string())?;
        // 借用 scratch bytes 解析；不把 Owned 容器 try_into_plain 擦域。
        return s.parse_rendered_plain(&out.bytes).map(JsonOwner::Plain);
    }
    let mut out = Vec::new();
    s.render_ref(r, &mut out).map_err(|e| e.to_string())?;
    s.parse_rendered_plain(&out).map(JsonOwner::Plain)
}
pub(crate) fn read_text(
    s: &mut (impl super::audit_store::CodecRead + ?Sized),
    r: sc::RefView<'_>,
) -> Result<TextValue, String> {
    let value = render_json(s, r)?;
    let text = value
        .view()
        .as_str()
        .ok_or("StorageUnavailable：预期文本")?;
    s.native_text(text)
}
impl PartialEq<TextValue> for String {
    fn eq(&self, o: &TextValue) -> bool {
        self.as_str() == o.as_str()
    }
}
impl PartialEq<TextValue> for str {
    fn eq(&self, o: &TextValue) -> bool {
        self == o.as_str()
    }
}

impl rusqlite::ToSql for TextValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Borrowed(
            rusqlite::types::ValueRef::Text(self.as_str().as_bytes()),
        ))
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod collection_owner_tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    struct PhaseQuery<'a>(i64, &'a str);
    impl Hash for PhaseQuery<'_> {
        fn hash<H: Hasher>(&self, h: &mut H) {
            (self.0, self.1).hash(h);
        }
    }
    impl hashbrown::Equivalent<(i64, TextValue)> for PhaseQuery<'_> {
        fn equivalent(&self, key: &(i64, TextValue)) -> bool {
            self.0 == key.0 && self.1 == key.1.as_str()
        }
    }
    struct MessageQuery<'a>(&'a str, &'a str, i64);
    impl Hash for MessageQuery<'_> {
        fn hash<H: Hasher>(&self, h: &mut H) {
            (self.0, self.1, self.2).hash(h);
        }
    }
    impl hashbrown::Equivalent<(TextValue, TextValue, i64)> for MessageQuery<'_> {
        fn equivalent(&self, key: &(TextValue, TextValue, i64)) -> bool {
            self.0 == key.0.as_str() && self.1 == key.1.as_str() && self.2 == key.2
        }
    }
    // 强制同hash，只检验既有hashbrown删除后的探测链；不另写桶或删除算法。
    #[derive(Eq, PartialEq)]
    struct Collision(TextValue);
    impl Hash for Collision {
        fn hash<H: Hasher>(&self, h: &mut H) {
            0u8.hash(h);
        }
    }
    struct CollisionQuery<'a>(&'a str);
    impl Hash for CollisionQuery<'_> {
        fn hash<H: Hasher>(&self, h: &mut H) {
            0u8.hash(h);
        }
    }
    impl hashbrown::Equivalent<Collision> for CollisionQuery<'_> {
        fn equivalent(&self, key: &Collision) -> bool {
            self.0 == key.0.as_str()
        }
    }

    #[test]
    fn owned_business_map_borrowed_unicode_tuple_and_collision_delete_chains() {
        let budget = OwnedBudget::new(1_000_000);
        {
            let mut plain = new_map::<TextValue, i64>(None);
            let mut owned = new_map::<TextValue, i64>(Some(&budget));
            for (n, key) in ["中文😀", "é", "e\u{301}", "消息\\\""].iter().enumerate() {
                plain
                    .insert(TextValue::from_str(key, None).unwrap(), n as i64)
                    .unwrap();
                owned
                    .insert(TextValue::from_str(key, Some(&budget)).unwrap(), n as i64)
                    .unwrap();
            }
            let attempts = budget.snapshot().allocation_attempts;
            for key in ["中文😀", "é", "e\u{301}", "消息\\\"", "不存在"] {
                assert_eq!(plain.get(key), owned.get(key));
                assert_eq!(plain.contains_key(key), owned.contains_key(key));
            }
            *owned.get_mut("中文😀").unwrap() = 9;
            assert_eq!(owned.remove("中文😀"), Some(9));
            assert_eq!(owned.remove("中文😀"), None);
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            assert_eq!(owned.iter().len(), owned.len());

            let mut phase = new_map::<(i64, TextValue), i64>(Some(&budget));
            phase
                .insert(
                    (7, TextValue::from_str("批次😀", Some(&budget)).unwrap()),
                    42,
                )
                .unwrap();
            let mut messages = new_map::<(TextValue, TextValue, i64), i64>(Some(&budget));
            messages
                .insert(
                    (
                        TextValue::from_str("消息", Some(&budget)).unwrap(),
                        TextValue::from_str("阶段é", Some(&budget)).unwrap(),
                        3,
                    ),
                    11,
                )
                .unwrap();
            let attempts = budget.snapshot().allocation_attempts;
            assert_eq!(phase.get(&PhaseQuery(7, "批次😀")), Some(&42));
            assert_eq!(phase.remove(&PhaseQuery(7, "批次😀")), Some(42));
            assert!(phase.is_empty());
            assert_eq!(messages.get(&MessageQuery("消息", "阶段é", 3)), Some(&11));
            assert_eq!(messages.remove(&MessageQuery("消息", "阶段é", 3)), Some(11));
            assert_eq!(budget.snapshot().allocation_attempts, attempts);

            let mut collisions = new_map::<Collision, i64>(Some(&budget));
            let keys = [
                "头😀",
                "中文",
                "é",
                "e\u{301}",
                "尾",
                "中间",
                "再次😀",
                "终",
            ];
            for (n, key) in keys.iter().enumerate() {
                collisions
                    .insert(
                        Collision(TextValue::from_str(key, Some(&budget)).unwrap()),
                        n as i64,
                    )
                    .unwrap();
            }
            let attempts = budget.snapshot().allocation_attempts;
            for removed in [0usize, 3, 5, 7] {
                assert_eq!(
                    collisions.remove(&CollisionQuery(keys[removed])),
                    Some(removed as i64)
                );
            }
            for (n, key) in keys.iter().enumerate() {
                assert_eq!(
                    collisions.get(&CollisionQuery(key)).copied(),
                    if [0usize, 3, 5, 7].contains(&n) {
                        None
                    } else {
                        Some(n as i64)
                    }
                );
            }
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            for n in [1usize, 2, 4, 6] {
                assert_eq!(collisions.remove(&CollisionQuery(keys[n])), Some(n as i64));
            }
            assert!(collisions.is_empty());
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn owned_business_map_old_new_live_rehash_one_byte_short_preserves_values() {
        let budget = OwnedBudget::new(1_000_000);
        {
            let mut old = new_map::<TextValue, TextValue>(Some(&budget));
            old.insert(
                TextValue::from_str("旧键😀", Some(&budget)).unwrap(),
                TextValue::from_str("旧值é", Some(&budget)).unwrap(),
            )
            .unwrap();
            let mut new = new_map::<i64, i64>(Some(&budget));
            new.insert(0, 10).unwrap();
            for n in 1..new.capacity() {
                new.insert(n as i64, n as i64 + 10).unwrap();
            }
            let count = new.len();
            let mut probe = new_map::<i64, i64>(Some(&budget));
            for n in 0..count {
                probe.insert(n as i64, n as i64 + 10).unwrap();
            }
            probe.insert(count as i64, 99).unwrap();
            let request = budget.allocation_request_probe().last_request_bytes;
            drop(probe);
            let live = budget.snapshot().live_requested_layout_bytes;
            let fill = sc::capture_allocation::CaptureSlab::new(
                0,
                budget.snapshot().limit_bytes - live - request + 1,
                &budget,
            )
            .unwrap();
            let before_refusal = budget.snapshot();
            let before_capacity = new.capacity();
            let error = new.insert(count as i64, 99).unwrap_err();
            assert_eq!(error.code(), sc::capture_allocation::LIMIT);
            assert_eq!(new.len(), count);
            assert_eq!(new.capacity(), before_capacity);
            for n in 0..count {
                assert_eq!(new.get(&(n as i64)), Some(&(n as i64 + 10)));
            }
            assert!(!new.contains_key(&(count as i64)));
            assert_eq!(old.get("旧键😀").unwrap().as_str(), "旧值é");
            let refusal = budget.allocation_request_probe();
            assert_eq!(refusal.refused_bytes, request);
            assert_eq!(
                refusal.live_before_refusal + request,
                budget.snapshot().limit_bytes + 1
            );
            // reserve_on先拒绝超额请求，再计实际allocator尝试；预检拒绝不能要求attempts增加。
            let after_refusal = budget.snapshot();
            assert_eq!(
                after_refusal.rejected_layouts,
                before_refusal.rejected_layouts + 1
            );
            assert_eq!(
                after_refusal.allocation_attempts,
                before_refusal.allocation_attempts
            );
            assert_eq!(
                after_refusal.allocated_layouts,
                before_refusal.allocated_layouts
            );
            assert_eq!(
                after_refusal.released_layouts,
                before_refusal.released_layouts
            );
            assert_eq!(
                after_refusal.live_requested_layout_bytes,
                before_refusal.live_requested_layout_bytes
            );
            let admitted_attempts = after_refusal.allocation_attempts;
            // 删除和已有值替换无需table分配，在同一拒绝头寸下仍保持其既有语义。
            assert_eq!(new.remove(&0), Some(10));
            assert_eq!(new.insert(1, 77).unwrap(), Some(11));
            assert_eq!(new.get(&1), Some(&77));
            assert_eq!(budget.snapshot().allocation_attempts, admitted_attempts);
            let removed = old.remove("旧键😀").unwrap();
            assert_eq!(removed.as_str(), "旧值é");
            assert!(old.is_empty());
            assert_eq!(budget.snapshot().allocation_attempts, admitted_attempts);
            drop(fill);
            drop(removed);
            new.insert(count as i64, 99).unwrap();
            assert_eq!(new.get(&(count as i64)), Some(&99));
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn owned_text_consumes_real_bytes_zero_copy_utf8_and_drops_zero() {
        let budget = OwnedBudget::new(10000);
        {
            let mut bytes = new_work(Some(&budget));
            bytes.try_reserve_exact("中文😀é".len()).unwrap();
            bytes_extend(&mut bytes, "中文😀é".as_bytes()).unwrap();
            let pointer = bytes.as_ptr();
            let live = budget.snapshot().live_requested_layout_bytes;
            let attempts = budget.snapshot().allocation_attempts;
            let text = TextValue::from_owned_bytes(bytes).unwrap();
            assert!(matches!(text, TextValue::Owned(_)));
            assert_eq!(text.as_str(), "中文😀é");
            assert_eq!(text.as_str().as_ptr(), pointer);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, live);
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            let clone = text.try_clone().unwrap();
            assert_eq!(clone.as_str(), text.as_str());
            assert!(budget.snapshot().live_requested_layout_bytes > live);
            drop(clone);
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let plain = sc::CodecVec::from_plain("普通😀".as_bytes().to_vec());
        let pointer = plain.as_ptr();
        let text = TextValue::from_bytes(plain).unwrap();
        assert!(matches!(text, TextValue::Plain(_)));
        assert_eq!(text.as_str().as_ptr(), pointer);
        assert!(TextValue::from_owned_bytes(sc::CodecVec::from_plain(vec![b'a'])).is_err());
        let mut invalid = new_work(Some(&budget));
        bytes_extend(&mut invalid, &[0xff]).unwrap();
        let attempts = budget.snapshot().allocation_attempts;
        assert!(TextValue::from_owned_bytes(invalid).is_err());
        assert_eq!(budget.snapshot().allocation_attempts, attempts);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod checked_sql_owner_tests {
    use super::*;

    #[test]
    fn checked_sql_text_owners_match_legacy_types_null_utf8_and_order() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE conversion_rows(position INTEGER, text_value, optional_value, integer_value, optional_integer, blob_value); INSERT INTO conversion_rows VALUES(2,'中文😀',NULL,9223372036854775807,NULL,x'00ff'); INSERT INTO conversion_rows VALUES(1,'é','é',-9223372036854775808,7,x'');").unwrap();
        let ordinary=conn.prepare("SELECT text_value,optional_value,integer_value,optional_integer,blob_value FROM conversion_rows ORDER BY position").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<i64>>(3)?,r.get::<_,Vec<u8>>(4)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        let budget = OwnedBudget::new(1_000_000);
        for owned in [false, true] {
            let mut stmt=CheckedStatement::prepare(&conn,c"SELECT text_value,optional_value,integer_value,optional_integer,blob_value FROM conversion_rows ORDER BY position").unwrap();
            let mut ordinal = 0;
            while let Some(mut row) = stmt.next().unwrap() {
                let owner = owned.then_some(&budget);
                let text = TextValue::from_sql(&mut row, 0, owner).unwrap();
                let optional = TextValue::optional_sql(&mut row, 1, owner).unwrap();
                assert_eq!(text.as_str(), ordinary[ordinal].0);
                assert_eq!(
                    optional.as_ref().map(TextValue::as_str),
                    ordinary[ordinal].1.as_deref()
                );
                assert_eq!(row.integer(2).unwrap(), ordinary[ordinal].2);
                assert_eq!(row.optional_integer(3).unwrap(), ordinary[ordinal].3);
                assert_eq!(row.blob(4).unwrap(), &ordinary[ordinal].4);
                assert_eq!(matches!(text, TextValue::Owned(_)), owned);
                ordinal += 1;
            }
            assert_eq!(ordinal, ordinary.len());
            stmt.finalize().unwrap();
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        let mut stmt =
            CheckedStatement::prepare(&conn, c"SELECT NULL,7,x'ff',CAST(x'ff' AS TEXT)").unwrap();
        let mut row = stmt.next().unwrap().unwrap();
        assert!(TextValue::from_sql(&mut row, 0, Some(&budget)).is_err());
        assert!(TextValue::optional_sql(&mut row, 0, Some(&budget))
            .unwrap()
            .is_none());
        for column in [1, 2, 3] {
            assert!(TextValue::from_sql(&mut row, column, Some(&budget)).is_err());
            assert!(TextValue::optional_sql(&mut row, column, Some(&budget)).is_err());
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        drop(row);
        stmt.finalize().unwrap();
    }
}

/// None保留旧serde grammar；Some只走一次owned严格解码，不重解析或转回Plain。
/// Owned语法诊断与动态重复键所有者保留至错误回复flush；frame/envelope另属旧通道。
pub(crate) enum StrictOwnerError {
    Syntax(serde_json::Error),
    #[cfg(feature = "s_allocation_json_testonly")]
    OwnedSyntax(aj::strict::Diagnostic),
    Allocation(sc::CodecError),
}
/// Native 与 Owned 是不同父账，禁止把两种能力同时传入。
fn check_native_owner(
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<(), sc::CodecError> {
    if let Some(work) = work {
        if owner.is_some() {
            return Err(work.reject(sc::capture_allocation::scratch::ScratchFailure::Ledger));
        }
    }
    Ok(())
}
fn admit_json_parse(
    bytes: &[u8],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<Option<NativeReservation>, sc::CodecError> {
    check_native_owner(owner, work)?;
    if let Some(work) = work {
        #[cfg(feature = "s_capture_native_testonly")]
        {
            let peak = super::audit_owner::native::JsonShape::scan(bytes).parse_peak();
            return work.reserve_native(peak).map(Some);
        }
        #[cfg(not(feature = "s_capture_native_testonly"))]
        {
            let _ = (bytes, work);
            return Err(sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Native JSON入口未启用",
            ));
        }
    }
    Ok(None)
}
/// #1527 mixed专用：许可先于唯一原parser，不受Native测试feature控制。
#[derive(Debug)]
pub(crate) enum MixedJsonError {
    Resource(sc::CodecError),
    Business(String),
}

impl From<String> for MixedJsonError {
    fn from(error: String) -> Self {
        Self::Business(error)
    }
}
impl From<&str> for MixedJsonError {
    fn from(error: &str) -> Self {
        Self::Business(error.to_owned())
    }
}

/// mixed资源拒绝保持原静态CodecError；只有许可成功才进入原解析核。
pub(crate) fn mixed_parse_admitted(
    bytes: &[u8],
    work: &WorkScope,
    parse: impl FnOnce() -> Result<PlainJson, MixedJsonError>,
) -> Result<PlainJson, MixedJsonError> {
    let permit = work
        .reserve_native(json_layout::JsonShape::scan(bytes).parse_peak())
        .map_err(MixedJsonError::Resource)?;
    parse().map(|value| value.with_reservation(Some(permit)))
}

pub(crate) fn mixed_strict_json_owner(
    bytes: &[u8],
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    mixed_parse_admitted(bytes, work, || {
        let parsed = strict_json_owner(bytes, None).map_err(|error| match error {
            StrictOwnerError::Syntax(e) => {
                MixedJsonError::Business(format!("SchemaUnsupported：{e}"))
            }
            StrictOwnerError::Allocation(e) => MixedJsonError::Resource(e),
            #[cfg(feature = "s_allocation_json_testonly")]
            StrictOwnerError::OwnedSyntax(e) => {
                MixedJsonError::Business(format!("SchemaUnsupported：{e}"))
            }
        })?;
        match parsed {
            JsonOwner::Plain(value) => Ok(value),
            #[cfg(feature = "s_allocation_json_testonly")]
            JsonOwner::Owned(_) => unreachable!("mixed None-owner原parser产出Plain"),
        }
    })
}
pub(crate) fn mixed_batch_json_owner(bytes:&[u8],id:&str,work:&WorkScope)->Result<PlainJson,MixedJsonError>{
    mixed_parse_admitted(bytes,work,||{
        let parsed=strict_json_owner(bytes,None).map_err(|e|match e{
            StrictOwnerError::Syntax(e)=>MixedJsonError::Business(format!("StorageUnavailable：batch {id} 规范字节非严格 JSON：{e}")),
            StrictOwnerError::Allocation(e)=>MixedJsonError::Resource(e),
            #[cfg(feature="s_allocation_json_testonly")]
            StrictOwnerError::OwnedSyntax(e)=>MixedJsonError::Business(format!("StorageUnavailable：batch {id} 规范字节非严格 JSON：{e}")),
        })?;
        match parsed{JsonOwner::Plain(value)=>Ok(value),#[cfg(feature="s_allocation_json_testonly")]JsonOwner::Owned(_)=>unreachable!("mixed parser uses plain storage")}
    })
}
pub(crate) fn mixed_legacy_json_owner(
    bytes: &[u8],
    work: &WorkScope,
) -> Result<PlainJson, MixedJsonError> {
    mixed_parse_admitted(bytes, work, || {
        serde_json::from_slice(bytes)
            .map(|v| PlainJson::from_reserved(v, None))
            .map_err(|e| MixedJsonError::Business(e.to_string()))
    })
}

pub(crate) fn strict_json_owner_admitted(
    bytes: &[u8],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<JsonOwner, StrictOwnerError> {
    let permit = admit_json_parse(bytes, owner, work).map_err(StrictOwnerError::Allocation)?;
    let mut parsed = strict_json_owner(bytes, owner)?;
    #[cfg_attr(
        not(feature = "s_allocation_json_testonly"),
        allow(irrefutable_let_patterns)
    )]
    if let JsonOwner::Plain(value) = &mut parsed {
        value.permit = permit;
    }
    Ok(parsed)
}
pub(crate) fn strict_plain_json_admitted(
    bytes: &[u8],
    work: Option<&WorkScope>,
) -> Result<PlainJson, String> {
    let parsed = strict_json_owner_admitted(bytes, None, work).map_err(|error| match error {
        StrictOwnerError::Syntax(e) => format!("SchemaUnsupported：{e}"),
        StrictOwnerError::Allocation(e) => e.to_string(),
        #[cfg(feature = "s_allocation_json_testonly")]
        StrictOwnerError::OwnedSyntax(e) => format!("SchemaUnsupported：{e}"),
    })?;
    match parsed {
        JsonOwner::Plain(value) => Ok(value),
        #[cfg(feature = "s_allocation_json_testonly")]
        JsonOwner::Owned(_) => unreachable!("None owner parser always produces Plain"),
    }
}
pub(crate) fn legacy_json_owner_admitted(
    bytes: &[u8],
    owner: Option<&OwnedBudget>,
    work: Option<&WorkScope>,
) -> Result<JsonOwner, String> {
    let permit = admit_json_parse(bytes, owner, work).map_err(|e| e.to_string())?;
    if let Some(owner) = owner {
        #[cfg(feature = "s_allocation_json_testonly")]
        return aj::OwnedJson::parse_legacy_shape(bytes, owner)
            .map(JsonOwner::Owned)
            .map_err(|e| e.to_string());
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = owner;
            return Err("prototype_audit_allocation_incomplete：Owned JSON入口未启用".into());
        }
    }
    serde_json::from_slice(bytes)
        .map(|v| JsonOwner::Plain(PlainJson::from_reserved(v, permit)))
        .map_err(|e| e.to_string())
}
pub(crate) fn strict_json_owner(
    bytes: &[u8],
    allocation: Option<&OwnedBudget>,
) -> Result<JsonOwner, StrictOwnerError> {
    if let Some(budget) = allocation {
        #[cfg(feature = "s_allocation_json_testonly")]
        return aj::strict::parse(bytes, budget)
            .map(JsonOwner::Owned)
            .map_err(|cause| match cause {
                aj::strict::ParseFailure::Syntax(cause) => StrictOwnerError::OwnedSyntax(cause),
                aj::strict::ParseFailure::Allocation(cause) => StrictOwnerError::Allocation(cause),
            });
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = budget;
            return Err(StrictOwnerError::Allocation(sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Owned Q严格入口未启用",
            )));
        }
    }
    use serde::de::DeserializeSeed;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let parsed = StrictOwnerSeed.deserialize(&mut decoder).and_then(|owner| {
        decoder.end()?;
        Ok(owner)
    });
    parsed.map_err(StrictOwnerError::Syntax)
}
#[derive(Clone, Copy)]
struct StrictOwnerSeed;
impl StrictOwnerSeed {
    fn scalar<E: serde::de::Error>(self, value: JsonView<'_>) -> Result<JsonOwner, E> {
        serde_json::to_value(value)
            .map(|v| JsonOwner::Plain(PlainJson::from(v)))
            .map_err(E::custom)
    }
    fn container(self, object: bool) -> JsonOwner {
        JsonOwner::Plain(PlainJson::from(if object {
            Value::Object(serde_json::Map::new())
        } else {
            Value::Array(Vec::new())
        }))
    }
    fn push<E: serde::de::Error>(self, parent: &mut JsonOwner, child: JsonOwner) -> Result<(), E> {
        match (parent, child) {
            (JsonOwner::Plain(parent), JsonOwner::Plain(child)) => {
                parent
                    .unscoped_mut()
                    .map_err(E::custom)?
                    .as_array_mut()
                    .ok_or_else(|| E::custom("Q owner representation mismatch"))?
                    .push(child.into_unscoped().map_err(E::custom)?);
                Ok(())
            }
            #[cfg_attr(
                not(feature = "s_allocation_json_testonly"),
                allow(unreachable_patterns)
            )]
            _ => Err(E::custom("Q owner representation mismatch")),
        }
    }
    fn insert<E: serde::de::Error>(
        self,
        parent: &mut JsonOwner,
        key: String,
        child: JsonOwner,
    ) -> Result<(), E> {
        match (parent, child) {
            (JsonOwner::Plain(parent), JsonOwner::Plain(child)) => {
                parent
                    .unscoped_mut()
                    .map_err(E::custom)?
                    .as_object_mut()
                    .ok_or_else(|| E::custom("Q owner representation mismatch"))?
                    .insert(key, child.into_unscoped().map_err(E::custom)?);
                Ok(())
            }
            #[cfg_attr(
                not(feature = "s_allocation_json_testonly"),
                allow(unreachable_patterns)
            )]
            _ => Err(E::custom("Q owner representation mismatch")),
        }
    }
}
impl<'de> serde::de::DeserializeSeed<'de> for StrictOwnerSeed {
    type Value = JsonOwner;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<JsonOwner, D::Error> {
        decoder.deserialize_any(self)
    }
}
impl<'de> serde::de::Visitor<'de> for StrictOwnerSeed {
    type Value = JsonOwner;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON without duplicate keys")
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<JsonOwner, E> {
        self.scalar(JsonView::Bool(value))
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<JsonOwner, E> {
        self.scalar((&Value::Number(value.into())).into())
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<JsonOwner, E> {
        self.scalar((&Value::Number(value.into())).into())
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<JsonOwner, E> {
        let number =
            serde_json::Number::from_f64(value).ok_or_else(|| E::custom("number out of range"))?;
        self.scalar((&Value::Number(number)).into())
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<JsonOwner, E> {
        self.scalar(JsonView::String(value))
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<JsonOwner, E> {
        Ok(JsonOwner::Plain(Value::String(value).into()))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<JsonOwner, E> {
        self.scalar(JsonView::Null)
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<JsonOwner, E> {
        self.scalar(JsonView::Null)
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut values: A) -> Result<JsonOwner, A::Error> {
        let mut owner = self.container(false);
        while let Some(child) = values.next_element_seed(self)? {
            self.push(&mut owner, child)?;
        }
        Ok(owner)
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut values: A) -> Result<JsonOwner, A::Error> {
        let mut owner = self.container(true);
        while let Some(key) = values.next_key_seed(StrictKeySeed)? {
            // 与原Strict一致：next_key之后、next_value之前拒绝，保serde行列位置。
            if owner.view().get(&key).is_some() {
                return Err(serde::de::Error::custom(format!("duplicate key {key}")));
            }
            self.insert(&mut owner, key, values.next_value_seed(self)?)?;
        }
        Ok(owner)
    }
}
struct StrictKeySeed;
impl<'de> serde::de::DeserializeSeed<'de> for StrictKeySeed {
    type Value = String;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<String, D::Error> {
        decoder.deserialize_string(self)
    }
}
impl<'de> serde::de::Visitor<'de> for StrictKeySeed {
    type Value = String;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a string")
    }
    fn visit_str<E: serde::de::Error>(self, key: &str) -> Result<String, E> {
        Ok(key.to_owned())
    }
    fn visit_string<E: serde::de::Error>(self, key: String) -> Result<String, E> {
        Ok(key)
    }
}

#[cfg(test)]
#[cfg_attr(
    not(feature = "s_allocation_json_testonly"),
    allow(
        dead_code,
        reason = "#1608：测试辅助，仅 s_allocation_json_testonly 测试使用"
    )
)]
pub(crate) fn strict_owner_protocol_matrix() -> Vec<Vec<u8>> {
    let mut cases: Vec<Vec<u8>> = [
        r#"{"x":1,"x":2}"#,
        r#"{"prototype_capture_owned_payload_limit":0,"prototype_capture_owned_payload_limit":1}"#,
        r#"{"nested":{"prototype_audit_allocation_\u0069ncomplete":0,"prototype_audit_allocation_incomplete":1}}"#,
        r#"{"a":{"x":1,"x":2}}"#,
        r#"{"\u0078":1,"x":2}"#,
        r#"{"中文😀":0,"中文😀":1}"#,
        "{\n\"é\":0,\n\"é\":1}",
        r#"{"":0,"":1}"#,
        "",
        "{",
        "[",
        "[1",
        "[1,]",
        "{\"x\":1,}",
        "{\"x\" 1}",
        "{\"x\":}",
        "[1 2]",
        "{\"x\":1 \"y\":2}",
        "true false",
        "tru",
        "nul",
        "falsx",
        "01",
        "-",
        "1.",
        "1e",
        "1e+",
        "1e400",
        "NaN",
        "+1",
        ".1",
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\uD800\u0041""#,
        r#""\uGGGG""#,
        r#""\q""#,
        "\"unterminated",
        "\"raw\nnewline\"",
        "null",
        "true",
        "false",
        "-9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775809",
        "1.25",
        "-0",
        "1e-400",
        r#""中文😀\n\u00e9\uD83D\uDE00""#,
        "{}",
        "[]",
        "{\"x\":[true,null,1.5]}",
    ]
    .into_iter()
    .map(|v| v.as_bytes().to_vec())
    .collect();
    // 动态诊断须保完整escaped巨键、五条legacy关键词与两prototype标记。
    let giant_key = r#"\u4e2d\u6587\uD83D\uDE00\n\"\u005c"#.repeat(2048);
    cases.push(format!("{{\"{giant_key}\":0,\"{giant_key}\":1}}").into_bytes());
    for key in [
        "Incomplete",
        "CaptureDeadline",
        "VerifyDeadline",
        "CaptureBytes",
        "atomic_delta_limit",
        "prototype_capture_owned_payload_limit",
        "prototype_audit_allocation_incomplete",
        "中文😀\"\\\n",
        "x at line 12 column 34",
        "x at line 0 column 0",
        "x at line 184467440737095516160 column 1",
        "x at line 1 column 2 more",
        "x at line 1 column 2 at line 3 column 4",
        "x at line 1 column 2 at line bad",
        "at line 1 column 2",
        "x at line 1 column 2\n中文",
    ] {
        let wire_key = serde_json::to_string(key).unwrap();
        cases.push(format!("{{\n{wire_key}:0,\n{wire_key}:1}}").into_bytes());
    }
    cases.push(vec![b'"', 0xff, b'"']);
    cases.push(vec![b'"', 0xe4, 0xb8, b'"']);
    for depth in [127, 128] {
        let mut v = vec![b'['; depth];
        v.extend_from_slice(b"null");
        v.extend(vec![b']'; depth]);
        cases.push(v);
    }
    cases
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod strict_owner_protocol_tests {
    use super::*;
    use newchan_rust::session_protocol::strict_json;
    fn diagnostic(result: Result<JsonOwner, StrictOwnerError>) -> Result<JsonOwner, String> {
        result.map_err(|error| match error {
            StrictOwnerError::Syntax(error) => format!("SchemaUnsupported：{error}"),
            StrictOwnerError::OwnedSyntax(error) => format!("SchemaUnsupported：{error}"),
            StrictOwnerError::Allocation(error) => format!("unexpected allocation: {error}"),
        })
    }
    #[test]
    fn strict_owner_none_some_preserve_original_grammar_first_error_and_typed_budget_cause() {
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        for bytes in strict_owner_protocol_matrix() {
            let original = strict_json(&bytes);
            let none = diagnostic(strict_json_owner(&bytes, None));
            let owned = diagnostic(strict_json_owner(&bytes, Some(&budget)));
            match original {
                Ok(original) => {
                    assert_eq!(none.unwrap().view(), JsonView::from(&original));
                    assert_eq!(owned.unwrap().view(), JsonView::from(&original));
                }
                Err(original) => {
                    assert_eq!(none.unwrap_err(), original, "None {:?}", bytes);
                    assert_eq!(owned.unwrap_err(), original, "Some {:?}", bytes);
                }
            }
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        let empty = OwnedBudget::new(0);
        for bytes in [
            b"[1".as_slice(),
            b"{\"x\":0,\"x\":1}".as_slice(),
            b"true".as_slice(),
        ] {
            match strict_json_owner(bytes, Some(&empty)) {
                Err(StrictOwnerError::Allocation(cause)) => {
                    assert_eq!(cause.code(), sc::capture_allocation::LIMIT)
                }
                _ => panic!("实际Layout拒绝必须保持typed首错"),
            }
            assert_eq!(empty.snapshot().live_requested_layout_bytes, 0);
        }
        // grammar先拒数字溢出，尚未走arena分配；零额度不能遮掉这个首错。
        match strict_json_owner(b"1e400", Some(&empty)) {
            Err(StrictOwnerError::OwnedSyntax(error)) => assert_eq!(
                format!("SchemaUnsupported：{error}"),
                strict_json(b"1e400").unwrap_err()
            ),
            _ => panic!("必须保留原grammar首错"),
        }
    }
}

#[cfg(feature = "s_capture_native_testonly")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::{retained_struct, Retained};

    retained_struct!(RawPrefixFacts {
        prefix,
        winners,
        ordered,
        coordinates
    });
    impl Retained for RawEvents {
        fn heap_bytes(&self) -> usize {
            match self {
                Self::Plain(v) => v.heap_bytes(),
                // 原许可随payload自退；旧手工归属账不得再次预付或退费。
                Self::Native(_) => 0,
                #[cfg(feature = "s_allocation_json_testonly")]
                Self::Owned(_) => usize::MAX,
            }
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod native_consumer_tests {
    use super::*;

    // #1488 源码锁：同父账下跨 scope 移动真实 facts/edits，NOT_RUN（本机禁 cargo）。
    #[test]
    fn raw_native_owners_move_out_of_scope_and_release_only_on_drop() {
        let raw = vec![
            serde_json::json!({"identity_key":"a", "revision":1, "source_coord":"0"}),
            serde_json::json!({"identity_key":"b", "revision":1, "source_coord":"1"}),
            serde_json::json!({"identity_key":"a", "revision":2, "source_coord":"2"}),
        ];
        let budget = super::super::audit_owner::native::Budget::new(2 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        for _ in 0..3 {
            let (facts, edits) = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let mut index = RawPrefixIndex::new((&raw).into());
                    let changes = index.advance_admitted(3, &NativeAdmission(Some(work)))?;
                    assert_eq!(index.ordered().collect::<Vec<_>>(), vec![(1, 1), (2, 2)]);
                    let edits = changes.winner_edits;
                    let mut delta = super::super::audit_retention::RetainedDelta::default();
                    let facts = index.into_facts_retained(&mut delta);
                    use super::super::audit_retention::Retained;
                    assert_eq!(delta.apply(0).unwrap(), facts.heap_bytes());
                    Ok::<_, String>((facts, edits))
                })
                .unwrap();
            let held = budget.live();
            assert!(held > baseline, "离开 scope 不得释放已移动载荷许可");
            assert_eq!(edits.len(), 2);
            drop(facts);
            assert!(budget.live() < held);
            assert!(budget.live() > baseline, "edits buffer 仍存活");
            drop(edits);
            assert_eq!(budget.live(), baseline, "每代真实 owner 销毁后归还原许可");
        }
    }

    #[test]
    fn raw_native_tree_reuses_high_water_and_retains_buffer_until_drop() {
        let budget = super::super::audit_owner::native::Budget::new(2 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let admission = NativeAdmission(Some(work));
                let mut tree = NativeTree::<i64, usize>::default();
                tree.insert_admitted(0, 0, &admission)?;
                tree.insert_admitted(1, 1, &admission)?;
                let held = budget.live();
                for i in 2..20 {
                    tree.remove(&(i - 2));
                    tree.insert_admitted(i, i as usize, &admission)?;
                    assert_eq!(budget.live(), held, "删除重插不累计历史节点许可");
                }
                tree.clear();
                assert!(budget.live() < held);
                Ok::<_, sc::CodecError>(())
            })
            .unwrap();
        assert_eq!(budget.live(), baseline);
    }

    #[test]
    fn admitted_sql_text_clone_and_json_parsers_match_plain() {
        let conn = Connection::open_in_memory().unwrap();
        let input = br#"{"escaped":"\u6c49\n\"quoted\"","rows":[null,true,-7,1.25,{"x":"text"}]}"#;
        let expected: Value = serde_json::from_slice(input).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(2 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let strict = strict_json_owner_admitted(input, None, Some(work))
                    .unwrap_or_else(|_| panic!("正常strict JSON须解析成功"));
                let legacy = legacy_json_owner_admitted(input, None, Some(work))?;
                assert_eq!(strict.view(), JsonView::Plain(&expected));
                assert_eq!(legacy.view(), JsonView::Plain(&expected));
                let mut stmt = CheckedStatement::prepare(&conn, c"SELECT '汉😀', NULL, '正常'")
                    .map_err(|e| e.to_string())?;
                let mut row = stmt.next().map_err(|e| e.to_string())?.unwrap();
                let text = TextValue::from_sql_admitted(&mut row, 0, None, Some(work))?;
                assert_eq!(text.as_str(), "汉😀");
                assert_eq!(text.try_clone_admitted(Some(work))?.as_str(), text.as_str());
                assert!(TextValue::optional_sql_admitted(&mut row, 1, None, Some(work))?.is_none());
                assert_eq!(
                    TextValue::optional_sql_admitted(&mut row, 2, None, Some(work))?
                        .unwrap()
                        .as_str(),
                    "正常"
                );
                drop(row);
                stmt.finalize().map_err(|e| e.to_string())?;
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(budget.live(), baseline);
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
    }

    #[test]
    fn admitted_json_and_hash_match_normal_with_payload_raii() {
        let value = serde_json::json!({
            "撤回": [{"object_id": "obj3-正常", "superseded_by": null, "window_end": -7}],
            "escaped": "line\n\"quoted\"",
            "scalars": [true, false, 0, 1.25]
        });
        let expected_hash = sha_json((&value).into(), None).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let before = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let admission = super::super::audit_store::NativeAdmission(Some(scope));
                let actual = json_from_view_admitted(&admission, (&value).into())?;
                assert_eq!(actual.view(), JsonView::Plain(&value));
                assert_eq!(
                    sha_json_admitted(&admission, (&value).into())?,
                    expected_hash
                );
                let held = budget.live();
                assert!(
                    held > before + sc::capture_allocation::scratch::CodecWork::metadata_bytes()
                );
                drop(actual);
                assert!(budget.live() < held, "JSON真实载荷析构归还原许可");
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(budget.live(), before, "JSON和scope都析构后父账回到原基线");
        assert_eq!(
            budget.scratch_observation().0,
            0,
            "真实scratch Layout已释放"
        );
        assert_eq!(budget.scratch_observation().2, 0, "真实scope元数据已释放");
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod plain_raii_tests {
    use super::*;

    #[test]
    fn native_plain_values_keep_original_permits_past_scope_and_drop_independently() {
        let budget = super::super::audit_owner::native::Budget::new(2 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let input = br#"{"rows":["child","other"],"flag":true}"#;
        for _ in 0..3 {
            let (text, reference, json) = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let source = NativeAdmission(Some(work));
                    let text = TextValue::from_str_admitted("persistent text", None, Some(work))?;
                    let built = sc::Built {
                        digest: [7; 32],
                        kind: sc::Kind::Array,
                        extent: Some(2),
                    };
                    let reference = source.native_built_ref(&built).map_err(|e| e.to_string())?;
                    let json = strict_plain_json_admitted(input, Some(work))?;
                    Ok::<_, String>((text, reference, json))
                })
                .unwrap();
            let held = budget.live();
            assert!(held > baseline);
            assert_eq!(text.as_str(), "persistent text");
            assert_eq!(reference.extent(), Some(2));
            assert!(json.is_native_admitted());
            assert!(text.try_clone().is_err(), "离域只读不能隐式免费深复制");
            assert!(NativeAdmission(None).native_ref_clone(&reference).is_err());
            let child = budget
                .codec_work()
                .unwrap()
                .run(|work| NativeAdmission(Some(work)).native_json_clone(&json["rows"][0]))
                .unwrap();
            assert_eq!(child.as_str(), Some("child"));
            assert!(budget.live() > held, "父与独立子值并存期间分别持费");
            drop(json);
            assert_eq!(child.as_str(), Some("child"));
            let before_text_drop = budget.live();
            drop(text);
            assert!(budget.live() < before_text_drop);
            let before_ref_drop = budget.live();
            drop(reference);
            assert!(budget.live() < before_ref_drop);
            assert!(budget.live() > baseline, "子值仍存活，原父析构不能退其费");
            drop(child);
            assert_eq!(budget.live(), baseline);
        }
    }

    #[test]
    fn native_strict_parse_preserves_diagnostics_and_failed_parse_returns_permit() {
        let budget = super::super::audit_owner::native::Budget::new(2 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        for input in [
            br#"{"duplicate":1,"duplicate":2}"#.as_slice(),
            br#"{"broken":[1,]}"#.as_slice(),
        ] {
            let expected = newchan_rust::session_protocol::strict_json(input).unwrap_err();
            let actual = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    Ok::<_, String>(strict_plain_json_admitted(input, Some(work)).unwrap_err())
                })
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(
                budget.live(),
                baseline,
                "语法失败无载荷逃逸，原准入许可释放"
            );
        }
    }

    #[test]
    fn native_json_cannot_be_mutated_or_detached_without_admission() {
        let budget = super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let mut json = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                strict_json_owner_admitted(b"[1,2]", None, Some(work)).map_err(|_| "parse failed")
            })
            .unwrap();
        let held = budget.live();
        assert!(json.push(JsonView::Null).is_err());
        assert_eq!(json.view().as_array().unwrap().len(), 2);
        assert_eq!(budget.live(), held);
        #[cfg_attr(
            not(feature = "s_allocation_json_testonly"),
            allow(irrefutable_let_patterns)
        )]
        let JsonOwner::Plain(json) = json
        else {
            panic!("Native解析必须保留Plain名分")
        };
        assert!(json.into_unscoped().is_err());
        assert_eq!(budget.live(), baseline);
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod scoped_json_array_tests {
    use super::super::audit_retention::Retained;
    use super::*;

    #[test]
    fn scoped_array_outlives_scope_retains_metadata_and_releases_at_last_owner_drop() {
        let budget = super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let initial = budget.live();
        let input = serde_json::json!({"rows":["中文😀", 7, null], "flag":true});
        let expected = serde_json::json!([input, input]);
        for _ in 0..3 {
            let owner = budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let mut array = ScopedJsonArray::native(work)?;
                    array.push((&input).into())?;
                    array.push((&input).into())?;
                    Ok::<_, String>(array.finish())
                })
                .unwrap();
            assert_eq!(owner.view(), JsonView::Plain(&expected));
            #[cfg_attr(
                not(feature = "s_allocation_json_testonly"),
                allow(irrefutable_let_patterns)
            )]
            let JsonOwner::Plain(value) = &owner
            else {
                panic!("Native必须保持Plain名分")
            };
            assert!(value.is_native_admitted());
            assert!(
                value.retained_heap_bytes() > value.as_ref().heap_bytes(),
                "metadata不可漏账"
            );
            assert!(budget.live() > initial);
            drop(owner);
            assert_eq!(
                budget.live(),
                initial,
                "跨scope后仍由实际owner析构归还，保留父自身metadata"
            );
        }
    }

    #[test]
    fn scoped_array_growth_accounts_old_new_payload_and_metadata_together() {
        let budget = super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let initial = budget.live();
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let baseline = budget.live();
                let mut array = ScopedJsonArray::native(work)?;
                for _ in 0..4 {
                    array.push(JsonView::Null)?;
                }
                let before = budget.live();
                let new_blocks =
                    8 * (std::mem::size_of::<Value>() + std::mem::size_of::<NativeReservation>());
                array.push(JsonView::Null)?;
                assert!(
                    budget.peak() >= before + new_blocks,
                    "两个旧buffer保留到两个新块同时准入完成"
                );
                #[cfg_attr(
                    not(feature = "s_allocation_json_testonly"),
                    allow(irrefutable_let_patterns)
                )]
                let JsonOwner::Plain(value) = &array.owner
                else {
                    panic!("Native必须保持Plain名分")
                };
                assert_eq!(value.child_permits.capacity(), 8);
                assert_eq!(value.as_array().unwrap().capacity(), 8);
                assert_eq!(
                    budget.live() - baseline,
                    value.retained_heap_bytes(),
                    "空值隔离验证metadata费用与Retained一致"
                );
                drop(array);
                assert_eq!(budget.live(), baseline);
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(budget.live(), initial);
    }

    #[test]
    fn scoped_array_each_preparation_refusal_preserves_parent_payload_capacity_and_live() {
        let value_block = 8 * std::mem::size_of::<Value>();
        let metadata_block = 8 * std::mem::size_of::<NativeReservation>();
        let text = "late child allocation";
        // 分别在payload buffer、metadata buffer、child许可准入差一字节处拒绝。
        for available in [
            value_block - 1,
            value_block + metadata_block - 1,
            value_block + metadata_block + text.len() - 1,
        ] {
            let limit = 1024 * 1024;
            let budget = super::super::audit_owner::native::Budget::new(limit, 0).unwrap();
            let initial = budget.live();
            let result = budget.codec_work().unwrap().run(|work| {
                let mut array = ScopedJsonArray::native(work)?;
                for _ in 0..4 {
                    array.push(JsonView::Null)?;
                }
                let held = budget.reserve(limit - budget.live() - available).unwrap();
                let before = budget.live();
                let error = array.push(JsonView::String(text)).unwrap_err();
                #[cfg_attr(
                    not(feature = "s_allocation_json_testonly"),
                    allow(irrefutable_let_patterns)
                )]
                let JsonOwner::Plain(value) = &array.owner
                else {
                    panic!("Native必须保持Plain名分")
                };
                assert_eq!(value.as_ref(), &serde_json::json!([null, null, null, null]));
                assert_eq!(value.as_array().unwrap().capacity(), 4);
                assert_eq!(value.child_permits.capacity(), 4);
                assert_eq!(budget.live(), before, "失败不得改变父账live");
                drop(held);
                Err::<(), String>(error)
            });
            assert!(result.is_err(), "首次资源拒绝必须传播出work.run");
            assert_eq!(budget.live(), initial);
        }
    }

    #[test]
    fn scoped_array_refuses_dual_parent_and_finished_native_rejects_unscoped_mutation() {
        let budget = super::super::audit_owner::native::Budget::new(1024 * 1024, 0).unwrap();
        let initial = budget.live();
        let allocation = OwnedBudget::new(1024 * 1024);
        assert!(budget
            .codec_work()
            .unwrap()
            .run(|work| { ScopedJsonArray::new(Some(&allocation), Some(work)).map(|_| ()) })
            .is_err());
        assert_eq!(budget.live(), initial);
        let mut owner = budget
            .codec_work()
            .unwrap()
            .run(|work| Ok::<_, String>(ScopedJsonArray::native(work)?.finish()))
            .unwrap();
        let held = budget.live();
        assert!(owner.push(JsonView::Null).is_err());
        assert_eq!(owner.view().as_array().unwrap().len(), 0);
        assert_eq!(budget.live(), held);
        drop(owner);
        assert_eq!(budget.live(), initial);
        let mut plain = ScopedJsonArray::unscoped();
        plain.push(JsonView::Null).unwrap();
        assert_eq!(
            plain.finish().view(),
            JsonView::Plain(&serde_json::json!([null]))
        );
    }
}

// #1488：跨代RawEvents保留同父许可；以下是测试源码，运行由远端统一执行。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod raw_events_native_append_tests {
    use super::super::audit_owner::native::Budget;
    use super::*;

    fn empty(budget: &Budget) -> RawEvents {
        budget
            .codec_work()
            .unwrap()
            .run(|work| RawEvents::from_json_owner(ScopedJsonArray::native(work)?.finish()))
            .unwrap()
    }

    #[test]
    fn same_parent_new_work_appends_and_foreign_parent_refuses_without_mutation() {
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        let foreign = Budget::new(1024 * 1024, 0).unwrap();
        let initial = budget.live();
        let foreign_initial = foreign.live();
        let mut raw = empty(&budget);
        let row =
            serde_json::json!({"seq":0,"identity_key":"same","revision":1,"raw_text":"中文😀"});
        budget
            .codec_work()
            .unwrap()
            .run(|work| raw.push_native((&row).into(), work))
            .unwrap();
        let held = budget.live();
        assert!(held > initial);
        assert_eq!(raw.len(), 1);
        foreign
            .codec_work()
            .unwrap()
            .run(|work| {
                let local_before = budget.live();
                let foreign_before = foreign.live();
                assert!(raw.push_native(JsonView::Null, work).is_err());
                assert_eq!(budget.live(), local_before);
                assert_eq!(foreign.live(), foreign_before);
                assert_eq!(raw.view().get(0), Some((&row).into()));
                Ok::<_, String>(())
            })
            .unwrap();
        assert_eq!(budget.live(), held);
        assert_eq!(foreign.live(), foreign_initial);
        budget
            .codec_work()
            .unwrap()
            .run(|work| raw.push_native(JsonView::Null, work))
            .unwrap();
        assert_eq!(raw.len(), 2);
        drop(raw);
        assert_eq!(budget.live(), initial);
    }

    #[test]
    fn failed_work_and_capacity_refusal_preserve_raw_payload_and_balance() {
        for poison_first in [false, true] {
            let limit = 1024 * 1024;
            let budget = Budget::new(limit, 0).unwrap();
            let initial = budget.live();
            let mut raw = empty(&budget);
            budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    for _ in 0..4 {
                        raw.push_native(JsonView::Null, work)?;
                    }
                    Ok::<_, String>(())
                })
                .unwrap();
            let held = budget.live();
            let result = budget.codec_work().unwrap().run(|work| {
                let before = budget.live();
                if poison_first {
                    let _ =
                        work.reject(sc::capture_allocation::scratch::ScratchFailure::Allocation);
                }
                let blocker = if poison_first {
                    None
                } else {
                    Some(budget.reserve(limit - budget.live()).unwrap())
                };
                let blocked = budget.live();
                let error = raw
                    .push_native(JsonView::String("cannot append"), work)
                    .unwrap_err();
                assert_eq!(raw.len(), 4);
                assert!(raw.view().iter().all(|v| v == JsonView::Null));
                assert_eq!(budget.live(), blocked);
                drop(blocker);
                assert_eq!(budget.live(), before);
                Err::<(), String>(error)
            });
            assert!(result.is_err());
            assert_eq!(budget.live(), held);
            drop(raw);
            assert_eq!(budget.live(), initial);
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod raw_events_layout_tests {
    use super::super::audit_owner::native::Budget;
    use super::*;

    #[test]
    fn aggregate_tree_conversion_and_direct_append_refuse_without_releasing_live_children() {
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        let initial = budget.live();
        let input = serde_json::json!([{"nested":["persistent child","中文😀"]}]);
        for strict in [false, true] {
            budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let baseline = budget.live();
                    let make = || -> Result<JsonOwner, String> {
                        if strict {
                            strict_plain_json_admitted(
                                r#"[{"nested":["persistent child","中文😀"]}]"#.as_bytes(),
                                Some(work),
                            )
                            .map(JsonOwner::Plain)
                        } else {
                            json_from_view_admitted(&NativeAdmission(Some(work)), (&input).into())
                        }
                    };
                    let aggregate = make()?;
                    assert_eq!(aggregate.view(), (&input).into());
                    assert!(budget.live() > baseline);
                    assert!(RawEvents::from_json_owner(aggregate).is_err());
                    assert_eq!(budget.live(), baseline, "转换拒绝消费owner后一次归还");
                    #[cfg_attr(
                        not(feature = "s_allocation_json_testonly"),
                        allow(irrefutable_let_patterns)
                    )]
                    let JsonOwner::Plain(aggregate) = make()?
                    else {
                        unreachable!()
                    };
                    let mut bypass = RawEvents::Native(aggregate);
                    let held = budget.live();
                    let error = bypass.push_native(JsonView::Null, work).unwrap_err();
                    assert!(error.contains("聚合Native数组许可"));
                    assert_eq!(budget.live(), held, "现存子树许可不得早退");
                    assert_eq!(bypass.view().get(0), Some((&input[0]).into()));
                    drop(bypass);
                    assert_eq!(budget.live(), baseline);
                    Ok::<_, String>(())
                })
                .unwrap();
            assert_eq!(budget.live(), initial);
        }
    }

    #[test]
    fn itemized_array_multiple_growths_retain_exact_buffers_across_work_scopes() {
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        let initial = budget.live();
        let mut raw = budget
            .codec_work()
            .unwrap()
            .run(|work| RawEvents::from_json_owner(ScopedJsonArray::native(work)?.finish()))
            .unwrap();
        for n in 1..=17 {
            budget
                .codec_work()
                .unwrap()
                .run(|work| raw.push_native(JsonView::Null, work))
                .unwrap();
            let RawEvents::Native(owner) = &raw else {
                unreachable!()
            };
            assert_eq!(raw.len(), n);
            assert!(raw.iter().all(|row| row == JsonView::Null));
            assert_eq!(
                budget.live() - initial,
                owner.retained_heap_bytes(),
                "null隔离子树费用，多个旧新buffer换代不遗留旧许可"
            );
        }
        drop(raw);
        assert_eq!(budget.live(), initial);
    }
}
