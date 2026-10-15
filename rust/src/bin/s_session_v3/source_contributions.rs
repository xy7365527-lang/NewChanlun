//! #1488 B2：实际 owner 材料下的来源贡献。没有独立授信构造器或失败回退。
use super::super::audit_owner::{
    BoundRawAdvance, Edit, RawAdvanceBinding, SourceArray, SourceCutBinding, SourceMaterial,
};
use super::super::audit_retention::{self as retention, Retained, RetainedDelta};
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
#[path = "source_order.rs"]
mod source_order;
use source_order::{Handle, Order};
use source_pending::{RequestSlots, StepLookup, StepView};

impl SourceFieldStore for SourceMaterial<'_, '_, '_, '_> {}

fn fields_bytes(fields: &SourceFields<'_>, _names: &[&str]) -> usize {
    match &fields.0 {
        super::super::audit_store::RefFields::Plain(values) => {
            values.plain_retained_bytes().unwrap_or(usize::MAX)
        }
        super::super::audit_store::RefFields::Owned(_) => usize::MAX,
    }
}

fn coords_bytes(coords: &SourceCoords) -> usize {
    match coords {
        SourceCoords::Original(v) => v.capacity().saturating_mul(std::mem::size_of::<i64>()),
        SourceCoords::Shared(v) => (2 * std::mem::size_of::<usize>())
            .saturating_add(v.len().saturating_mul(std::mem::size_of::<i64>())),
    }
}
impl Retained for Shape {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl Retained for Object {
    fn heap_bytes(&self) -> usize {
        retention::sum([
            self.id.capacity(),
            self.kind.capacity(),
            self.refs.heap_bytes(),
            coords_bytes(&self.sources),
            fields_bytes(
                &self.fields,
                if self.kind == CC006_SHAPE_KIND {
                    &SHAPE_SOURCE_FIELDS
                } else {
                    &TYPED_SOURCE_FIELDS
                },
            ),
        ])
    }
}
impl Retained for Group {
    fn heap_bytes(&self) -> usize {
        retention::sum([
            self.payload.heap_bytes(),
            coords_bytes(&self.members),
            coords_bytes(&self.sources),
        ])
    }
}
impl Retained for source_pending::StepContribution {
    fn heap_bytes(&self) -> usize {
        retention::sum([
            self.object.heap_bytes(),
            fields_bytes(&self.fields, &source_pending::STEP),
            self.key.capacity(),
            self.reasons.heap_bytes(),
        ])
    }
}

// 各容器只在实际修改处增减账；最终 retained_bytes 不遍历历史。
struct Map<K, V> {
    values: BTreeMap<K, V>,
    bytes: usize,
}
impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            bytes: 0,
        }
    }
}
impl<K, V> std::ops::Deref for Map<K, V> {
    type Target = BTreeMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl<K: Ord + Retained, V: Retained> Map<K, V> {
    fn put(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        key: K,
        value: V,
    ) -> Result<Option<V>, String> {
        let before = self.values.len();
        let old_bytes = self
            .values
            .get_key_value(&key)
            .map_or(0, |(k, v)| k.heap_bytes().saturating_add(v.heap_bytes()));
        let new_bytes = key.heap_bytes().saturating_add(value.heap_bytes());
        // BTreeMap::insert retains the original key when replacing.
        let retained_key = self.values.get_key_value(&key).map(|(k, _)| k.heap_bytes());
        let new_bytes = retained_key.map_or(new_bytes, |k| k.saturating_add(value.heap_bytes()));
        if !self.values.contains_key(&key) {
            s.native_tree_room::<K, V>(self.values.len()).map_err(err)?;
        }
        let old = self.values.insert(key, value);
        let mut delta = RetainedDelta::default();
        delta.tree_len::<K, V>(before, self.values.len());
        delta.release_bytes(old_bytes);
        delta.add_bytes(new_bytes);
        self.bytes = delta.apply(self.bytes).map_err(|failure| match failure {
            super::super::audit_owner::native::Failure::BudgetExceeded => err("来源保留费溢出"),
            super::super::audit_owner::native::Failure::Invalid(message) => err(message),
        })?;
        Ok(old)
    }
    fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Result<Option<V>, String>
    where
        K: std::borrow::Borrow<Q>,
    {
        let Some((key, value)) = self.values.remove_entry(key) else {
            return Ok(None);
        };
        self.bytes = self
            .bytes
            .checked_sub(
                key.heap_bytes()
                    .saturating_add(value.heap_bytes())
                    .saturating_add(retention::tree_slots::<K, V>(1)),
            )
            .ok_or_else(|| err("来源保留费下溢"))?;
        if self.values.is_empty() {
            self.values.clear();
        }
        Ok(Some(value))
    }
    fn mutate<Q: Ord + ?Sized, R>(
        &mut self,
        key: &Q,
        f: impl FnOnce(&mut V) -> Result<R, String>,
    ) -> Result<R, String>
    where
        K: std::borrow::Borrow<Q>,
    {
        let value = self
            .values
            .get_mut(key)
            .ok_or_else(|| err("来源索引悬空"))?;
        let before = value.heap_bytes();
        let result = f(value)?;
        let after = value.heap_bytes();
        self.bytes = self
            .bytes
            .checked_sub(before)
            .and_then(|n| n.checked_add(after))
            .filter(|n| *n != usize::MAX)
            .ok_or_else(|| err("来源保留费溢出"))?;
        Ok(result)
    }
}
impl<K, V> Retained for Map<K, V> {
    fn heap_bytes(&self) -> usize {
        self.bytes
    }
}
impl RawLookup for Map<i64, Source> {
    fn get(&self, coord: &i64) -> Option<&Source> {
        self.values.get(coord)
    }
}
impl GroupLookup for Map<i64, Group> {
    fn get(&self, index: &i64) -> Option<&Group> {
        self.values.get(index)
    }
    fn len(&self) -> usize {
        self.values.len()
    }
}
impl ShapeLookup for Map<String, Shape> {
    fn get(&self, id: &str) -> Option<&Shape> {
        self.values.get(id)
    }
}
struct Tree<K> {
    values: BTreeSet<K>,
    bytes: usize,
}
impl<K> Default for Tree<K> {
    fn default() -> Self {
        Self {
            values: BTreeSet::new(),
            bytes: 0,
        }
    }
}
impl<K> std::ops::Deref for Tree<K> {
    type Target = BTreeSet<K>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl<K: Ord + Retained> Tree<K> {
    fn insert(&mut self, s: &(impl CodecRead + ?Sized), key: K) -> Result<bool, String> {
        if self.values.contains(&key) {
            return Ok(false);
        }
        s.native_tree_room::<K, ()>(self.values.len())
            .map_err(err)?;
        self.bytes = self
            .bytes
            .checked_add(retention::tree_slots::<K, ()>(1))
            .and_then(|n| n.checked_add(key.heap_bytes()))
            .filter(|n| *n != usize::MAX)
            .ok_or_else(|| err("来源保留费溢出"))?;
        self.values.insert(key);
        Ok(true)
    }
    fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Result<bool, String>
    where
        K: std::borrow::Borrow<Q>,
    {
        if let Some(old) = self.values.take(key) {
            self.bytes = self
                .bytes
                .checked_sub(retention::tree_slots::<K, ()>(1).saturating_add(old.heap_bytes()))
                .ok_or_else(|| err("来源保留费下溢"))?;
            if self.values.is_empty() {
                self.values.clear();
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
impl<K> Retained for Tree<K> {
    fn heap_bytes(&self) -> usize {
        self.bytes
    }
}
fn owned(s: &(impl CodecRead + ?Sized), text: &str) -> Result<String, String> {
    s.native_string(text).map_err(err)
}
fn vec_push<T>(s: &(impl CodecRead + ?Sized), v: &mut Vec<T>, value: T) -> Result<(), String> {
    s.native_vec_room(v, 1).map_err(err)?;
    v.push(value);
    Ok(())
}
fn link<K: Ord + Retained>(
    s: &(impl CodecRead + ?Sized),
    map: &mut Map<K, Tree<Handle>>,
    key: K,
    handle: Handle,
) -> Result<(), String> {
    if !map.contains_key(&key) {
        map.put(s, key, Tree::default())?;
    } else {
        drop(key);
    }
    // Callers use link_ref for borrowed keys after establishing the entry.
    let _ = handle;
    Ok(())
}
fn link_ref<K: Ord + Retained, Q: Ord + ?Sized>(
    s: &(impl CodecRead + ?Sized),
    map: &mut Map<K, Tree<Handle>>,
    key: &Q,
    handle: Handle,
) -> Result<(), String>
where
    K: std::borrow::Borrow<Q>,
{
    map.mutate(key, |set| set.insert(s, handle).map(|_| ()))
}
fn unlink<K: Ord + Retained, Q: Ord + ?Sized>(
    map: &mut Map<K, Tree<Handle>>,
    key: &Q,
    handle: Handle,
) -> Result<(), String>
where
    K: std::borrow::Borrow<Q>,
{
    if map.contains_key(key) {
        map.mutate(key, |set| set.remove(&handle).map(|_| ()))?;
        if map.get(key).is_some_and(|set| set.is_empty()) {
            map.remove(key)?;
        }
    }
    Ok(())
}
fn dirty_from<K: Ord, Q: Ord + ?Sized>(
    s: &(impl CodecRead + ?Sized),
    map: &Map<K, Tree<Handle>>,
    key: &Q,
    dirty: &mut Tree<Handle>,
) -> Result<(), String>
where
    K: std::borrow::Borrow<Q>,
{
    if let Some(set) = map.get(key) {
        for handle in set.iter() {
            dirty.insert(s, *handle)?;
        }
    }
    Ok(())
}
fn schedule(
    s: &(impl CodecRead + ?Sized),
    order: &Order,
    events: &mut Tree<(usize, Handle)>,
    handle: Handle,
) -> Result<(), String> {
    events.insert(s, (order.rank(handle)?, handle))?;
    Ok(())
}
fn before<K: Ord + Retained>(
    s: &(impl CodecRead + ?Sized),
    index: &mut Map<K, Handle>,
    key: K,
    at: Handle,
    order: &Order,
    events: &mut Tree<(usize, Handle)>,
) -> Result<bool, String> {
    if let Some(other) = index.get(&key).copied() {
        if other == at {
            return Ok(false);
        }
        if order.rank(other)? < order.rank(at)? {
            return Ok(true);
        }
        schedule(s, order, events, other)?;
    }
    index.put(s, key, at)?;
    Ok(false)
}
fn forget<K: Ord + Retained, Q: Ord + ?Sized>(
    index: &mut Map<K, Handle>,
    key: &Q,
    at: Handle,
) -> Result<(), String>
where
    K: std::borrow::Borrow<Q>,
{
    if index.get(key) == Some(&at) {
        index.remove(key)?;
    }
    Ok(())
}

struct Entry {
    value: Rc<Object>,
    group: Option<i64>,
}
impl Retained for Entry {
    fn heap_bytes(&self) -> usize {
        self.value.heap_bytes()
    }
}
struct Witness {
    id: String,
    slot: i64,
}
impl Retained for Witness {
    fn heap_bytes(&self) -> usize {
        self.id.capacity()
    }
}
struct Observation {
    count: usize,
    insufficient: bool,
    outside: bool,
}
impl Retained for Observation {
    fn heap_bytes(&self) -> usize {
        0
    }
}
struct Request {
    key: String,
    subject: Option<String>,
    coord: Option<i64>,
}
impl Retained for Request {
    fn heap_bytes(&self) -> usize {
        self.key
            .capacity()
            .saturating_add(self.subject.heap_bytes())
    }
}
struct StepState {
    handle: Handle,
    data: source_pending::StepContribution,
    seed: Option<RefValue>,
    unsettled: bool,
    waiting: bool,
}
impl Retained for StepState {
    fn heap_bytes(&self) -> usize {
        self.data
            .heap_bytes()
            .saturating_add(self.seed.heap_bytes())
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct SourceWork {
    pub(crate) full_source_calls: usize,
    pub(crate) full_pending_calls: usize,
    pub(crate) full_members_enumerations: usize,
    pub(crate) object_local: usize,
    pub(crate) shape_local: usize,
    pub(crate) witness_local: usize,
    pub(crate) step_local: usize,
    pub(crate) step_sequence: usize,
    pub(crate) request_local: usize,
    pub(crate) observations: usize,
    pub(crate) raw_hashes: usize,
    pub(crate) members_updated: usize,
    pub(crate) order_events: usize,
    pub(crate) retained_spans: usize,
    pub(crate) order_height: usize,
    pub(crate) order_path_visits: usize,
    pub(crate) sort_comparisons: usize,
    pub(crate) prefix_members_checked: usize,
}
pub(crate) struct NativeSourceState {
    root: Built,
    raw_binding: RawAdvanceBinding,
    raw_prefix: usize,
    rule: String,
    raw: Map<i64, Source>,
    raw_ordinals: Map<i64, usize>,
    raw_order: Tree<i64>,
    objects: Map<Handle, Entry>,
    object_order: Order,
    object_sequence: Option<sc::SequenceRef>,
    ids: Map<String, Handle>,
    groups: Map<i64, Group>,
    group_ids: Map<i64, Handle>,
    group_indices: Tree<i64>,
    gaps: Tree<i64>,
    shapes: Map<String, Shape>,
    shape_slots: Map<String, (u8, usize)>,
    missing_shapes: Tree<String>,
    raw_objects: Map<i64, Tree<Handle>>,
    group_shapes: Map<i64, Tree<Handle>>,
    shape_witnesses: Map<String, Tree<Handle>>,
    witnesses: Map<Handle, Witness>,
    witness_order: Order,
    witness_sequence: Option<sc::SequenceRef>,
    slots: Map<(String, i64), Handle>,
    observations: Map<NodeDigest, Observation>,
    observation_sequence: Option<sc::SequenceRef>,
    insufficient: usize,
    outside: usize,
    steps: Map<i64, StepState>,
    step_coords: Tree<i64>,
    step_handles: Map<Handle, i64>,
    step_subjects: Map<String, i64>,
    knowledge: Option<Handle>,
    pending: Tree<i64>,
    members: Map<i64, Tree<(i64, usize)>>,
    overlaps: Tree<(i64, usize)>,
    pending_overlap: Tree<i64>,
    coverage: Tree<i64>,
    requests: Map<Handle, Request>,
    request_order: Order,
    request_sequence: Option<sc::SequenceRef>,
    request_keys: Map<String, Handle>,
    request_bound: Map<i64, Handle>,
    request_subjects: Map<String, Tree<Handle>>,
    step_requests: Map<i64, Tree<Handle>>,
    missing_requests: Tree<i64>,
    pub(crate) work: SourceWork,
}
impl NativeSourceState {
    fn empty(
        s: &(impl CodecRead + ?Sized),
        raw: &BoundRawAdvance<'_>,
        root: Built,
        rule: &str,
    ) -> Result<Self, String> {
        if raw.previous_prefix() != 0 {
            return Err(err("新来源贡献必须从空发布前缀建立"));
        }
        Ok(Self {
            root,
            raw_binding: raw.previous_binding(),
            raw_prefix: 0,
            rule: owned(s, rule)?,
            raw: Map::default(),
            raw_ordinals: Map::default(),
            raw_order: Tree::default(),
            objects: Map::default(),
            object_order: Order::default(),
            object_sequence: None,
            ids: Map::default(),
            groups: Map::default(),
            group_ids: Map::default(),
            group_indices: Tree::default(),
            gaps: Tree::default(),
            shapes: Map::default(),
            shape_slots: Map::default(),
            missing_shapes: Tree::default(),
            raw_objects: Map::default(),
            group_shapes: Map::default(),
            shape_witnesses: Map::default(),
            witnesses: Map::default(),
            witness_order: Order::default(),
            witness_sequence: None,
            slots: Map::default(),
            observations: Map::default(),
            observation_sequence: None,
            insufficient: 0,
            outside: 0,
            steps: Map::default(),
            step_coords: Tree::default(),
            step_handles: Map::default(),
            step_subjects: Map::default(),
            knowledge: None,
            pending: Tree::default(),
            members: Map::default(),
            overlaps: Tree::default(),
            pending_overlap: Tree::default(),
            coverage: Tree::default(),
            requests: Map::default(),
            request_order: Order::default(),
            request_sequence: None,
            request_keys: Map::default(),
            request_bound: Map::default(),
            request_subjects: Map::default(),
            step_requests: Map::default(),
            missing_requests: Tree::default(),
            work: SourceWork::default(),
        })
    }
    pub(crate) fn retained_bytes(&self) -> usize {
        retention::sum([
            self.rule.capacity(),
            self.raw.heap_bytes(),
            self.raw_ordinals.heap_bytes(),
            self.raw_order.heap_bytes(),
            self.objects.heap_bytes(),
            self.object_order.retained_bytes(),
            self.ids.heap_bytes(),
            self.groups.heap_bytes(),
            self.group_ids.heap_bytes(),
            self.group_indices.heap_bytes(),
            self.gaps.heap_bytes(),
            self.shapes.heap_bytes(),
            self.shape_slots.heap_bytes(),
            self.missing_shapes.heap_bytes(),
            self.raw_objects.heap_bytes(),
            self.group_shapes.heap_bytes(),
            self.shape_witnesses.heap_bytes(),
            self.witnesses.heap_bytes(),
            self.witness_order.retained_bytes(),
            self.slots.heap_bytes(),
            self.observations.heap_bytes(),
            self.steps.heap_bytes(),
            self.step_coords.heap_bytes(),
            self.step_handles.heap_bytes(),
            self.step_subjects.heap_bytes(),
            self.pending.heap_bytes(),
            self.members.heap_bytes(),
            self.overlaps.heap_bytes(),
            self.pending_overlap.heap_bytes(),
            self.coverage.heap_bytes(),
            self.requests.heap_bytes(),
            self.request_order.retained_bytes(),
            self.request_keys.heap_bytes(),
            self.request_bound.heap_bytes(),
            self.request_subjects.heap_bytes(),
            self.step_requests.heap_bytes(),
            self.missing_requests.heap_bytes(),
        ])
    }
    fn counts(&self) -> Counts {
        Counts {
            objects: self.objects.len(),
            groups: self.groups.len(),
            windows: self.groups.len().saturating_sub(2),
            effective: self.raw.len(),
            insufficient: self.insufficient,
            outside_domain: self.outside,
        }
    }
    #[cfg(test)]
    pub(crate) fn counts_for_test(&self) -> Counts {
        self.counts()
    }
}
pub(crate) struct NativeSourceVerdict {
    binding: SourceCutBinding,
    counts: Counts,
    state: NativeSourceState,
}
impl NativeSourceVerdict {
    pub(crate) fn into_parts(self) -> (SourceCutBinding, Counts, NativeSourceState) {
        (self.binding, self.counts, self.state)
    }
}
#[derive(Clone, Copy)]
enum Stream {
    Array(SourceArray),
    Requests(u64),
}
fn delta<'a>(
    s: &'a mut SourceMaterial<'_, '_, '_, '_>,
    stream: Stream,
) -> Result<&'a super::super::audit_owner::SequenceDelta, String> {
    match stream {
        Stream::Array(a) => s.sequence_delta(a),
        Stream::Requests(at) => s.request_delta(at),
    }
}
fn member(
    s: &mut SourceMaterial<'_, '_, '_, '_>,
    stream: Stream,
    ordinal: usize,
) -> Result<RefValue, String> {
    let ordinal = u64::try_from(ordinal).map_err(|_| err("来源成员位置溢出"))?;
    match stream {
        Stream::Array(a) => s.array_member(a, ordinal),
        Stream::Requests(at) => s.request_member(at, ordinal),
    }
}
struct Changes {
    removed: Vec<(Handle, NodeDigest)>,
    added: Vec<Handle>,
}
fn apply_order(
    s: &mut SourceMaterial<'_, '_, '_, '_>,
    stream: Stream,
    sequence: &mut Option<sc::SequenceRef>,
    order: &mut Order,
    work: &mut SourceWork,
) -> Result<Changes, String> {
    let d = delta(s, stream)?;
    if d.old != *sequence {
        return Err(err("来源数组前根与贡献不同"));
    }
    let current = d.current;
    let len = d.edits.len();
    let mut removals = Vec::new();
    let mut additions = Vec::new();
    let mut retained = 0usize;
    for i in 0..len {
        match delta(s, stream)?.edits[i] {
            Edit::Remove { ordinal, value } => vec_push(
                s,
                &mut removals,
                (
                    usize::try_from(ordinal).map_err(|_| err("来源位置溢出"))?,
                    value,
                ),
            )?,
            Edit::Add { ordinal, value } => vec_push(
                s,
                &mut additions,
                (
                    usize::try_from(ordinal).map_err(|_| err("来源位置溢出"))?,
                    value,
                ),
            )?,
            Edit::Retain { old, current } => {
                if old.sequence != current.sequence {
                    return Err(err("来源保留span原件不同"));
                }
                retained = retained
                    .checked_add(
                        usize::try_from(old.sequence.count()).map_err(|_| err("来源span溢出"))?,
                    )
                    .ok_or_else(|| err("来源span溢出"))?;
                work.retained_spans += 1;
            }
        }
    }
    let current_len = current.map_or(0, |r| r.count());
    if retained.checked_add(removals.len()) != Some(order.len())
        || retained
            .checked_add(additions.len())
            .and_then(|n| u64::try_from(n).ok())
            != Some(current_len)
    {
        return Err(err("来源数组差分未精确覆盖"));
    }
    removals.sort_unstable_by(|a, b| {
        work.sort_comparisons += 1;
        b.0.cmp(&a.0)
    });
    additions.sort_unstable_by(|a, b| {
        work.sort_comparisons += 1;
        a.0.cmp(&b.0)
    });
    let mut removed = Vec::new();
    let mut added = Vec::new();
    for (rank, value) in removals {
        let handle = order.remove(s, rank, value)?;
        vec_push(s, &mut removed, (handle, value))?;
    }
    for (rank, value) in additions {
        let handle = order.insert(s, rank, value)?;
        vec_push(s, &mut added, handle)?;
    }
    *sequence = current;
    work.order_height = work.order_height.max(order.height_bound());
    Ok(Changes { removed, added })
}
impl NativeSourceState {
    fn coverage_at(&mut self, s: &(impl CodecRead + ?Sized), coord: i64) -> Result<(), String> {
        let group = self.members.get(&coord).is_some_and(|m| !m.is_empty());
        let pending = self.pending.contains(&coord);
        if group && pending {
            self.pending_overlap.insert(s, coord)?;
        } else {
            self.pending_overlap.remove(&coord)?;
        }
        if self.raw.contains_key(&coord) != (group || pending) {
            self.coverage.insert(s, coord)?;
        } else {
            self.coverage.remove(&coord)?;
        }
        Ok(())
    }
    fn member_change(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        coord: i64,
        index: i64,
        position: usize,
        add: bool,
    ) -> Result<(), String> {
        if let Some(second) = self
            .members
            .get(&coord)
            .and_then(|m| m.iter().nth(1))
            .copied()
        {
            self.overlaps.remove(&second)?;
        }
        if !self.members.contains_key(&coord) {
            self.members.put(s, coord, Tree::default())?;
        }
        self.members.mutate(&coord, |members| {
            if add {
                members.insert(s, (index, position)).map(|_| ())
            } else {
                members.remove(&(index, position)).map(|_| ())
            }
        })?;
        if let Some(second) = self
            .members
            .get(&coord)
            .and_then(|m| m.iter().nth(1))
            .copied()
        {
            self.overlaps.insert(s, second)?;
        }
        if self.members.get(&coord).is_some_and(|m| m.is_empty()) {
            self.members.remove(&coord)?;
        }
        self.work.members_updated += 1;
        self.coverage_at(s, coord)
    }
    fn gap_at(&mut self, s: &(impl CodecRead + ?Sized), index: i64) -> Result<(), String> {
        if !self.group_indices.contains(&index) {
            self.gaps.remove(&index)?;
            return Ok(());
        }
        let expected = self
            .group_indices
            .range(..index)
            .next_back()
            .copied()
            .map_or(Some(0), |v| v.checked_add(1));
        if expected != Some(index) {
            self.gaps.insert(s, index)?;
        } else {
            self.gaps.remove(&index)?;
        }
        Ok(())
    }
    fn group_changed(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        index: i64,
        group: &Group,
        add: bool,
    ) -> Result<(), String> {
        if add {
            self.group_indices.insert(s, index)?;
        } else {
            self.group_indices.remove(&index)?;
        }
        self.gap_at(s, index)?;
        if let Some(next) = self
            .group_indices
            .range((std::ops::Bound::Excluded(index), std::ops::Bound::Unbounded))
            .next()
            .copied()
        {
            self.gap_at(s, next)?;
        }
        for (position, coord) in group.members.iter().enumerate() {
            self.member_change(s, *coord, index, position, add)?;
        }
        Ok(())
    }
    fn remove_shape(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        handle: Handle,
        dirty_witness: &mut Tree<Handle>,
    ) -> Result<(), String> {
        dirty_from(s, &self.shape_witnesses, id, dirty_witness)?;
        if let Some(shape) = self.shapes.remove(id)? {
            for index in shape.groups {
                unlink(&mut self.group_shapes, &index, handle)?;
            }
        }
        self.shape_slots.remove(id)?;
        self.missing_shapes.remove(id)?;
        Ok(())
    }
    fn remove_object(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        handle: Handle,
        dirty_groups: &mut Tree<i64>,
        dirty_witness: &mut Tree<Handle>,
    ) -> Result<Entry, String> {
        let entry = self
            .objects
            .remove(&handle)?
            .ok_or_else(|| err("来源对象贡献缺失"))?;
        forget(&mut self.ids, entry.value.id.as_str(), handle)?;
        for coord in entry.value.sources.iter() {
            unlink(&mut self.raw_objects, coord, handle)?;
        }
        if let Some(index) = entry.group {
            forget(&mut self.group_ids, &index, handle)?;
            if let Some(group) = self.groups.remove(&index)? {
                self.group_changed(s, index, &group, false)?;
            }
            dirty_groups.insert(s, index)?;
        }
        if entry.value.kind == CC006_SHAPE_KIND {
            self.remove_shape(s, entry.value.id.as_str(), handle, dirty_witness)?;
        }
        Ok(entry)
    }
    fn raw_advance(
        &mut self,
        s: &mut SourceMaterial<'_, '_, '_, '_>,
        raw: &BoundRawAdvance<'_>,
        dirty_objects: &mut Tree<Handle>,
    ) -> Result<Option<i64>, String> {
        raw.ensure_current()?;
        if self.raw_binding != raw.previous_binding() || self.raw_prefix != raw.previous_prefix() {
            return Err(err("来源raw发布前沿不连续"));
        }
        let mut touched = Tree::default();
        let mut old_presence: Map<i64, bool> = Map::default();
        for change in raw.winner_edits() {
            for w in [change.old, change.new].into_iter().flatten() {
                touched.insert(s, w.coord)?;
                if !old_presence.contains_key(&w.coord) {
                    old_presence.put(s, w.coord, self.raw.contains_key(&w.coord))?;
                }
            }
        }
        for change in raw.winner_edits() {
            if let Some(old) = change.old {
                if self.raw_ordinals.get(&old.coord) != Some(&old.ordinal) {
                    return Err(err("来源raw旧赢家不是原发布原件"));
                }
                self.raw_ordinals.remove(&old.coord)?;
                self.raw.remove(&old.coord)?;
            }
        }
        for change in raw.winner_edits() {
            if let Some(new) = change.new {
                if self.raw.contains_key(&new.coord) {
                    return Err(err("来源raw坐标冲突"));
                }
                let event = raw
                    .raw()
                    .get(new.ordinal)
                    .ok_or_else(|| err("来源raw赢家越界"))?;
                if bj::coord(event)? != new.coord {
                    return Err(err("来源raw赢家坐标不同"));
                }
                let wire = bj::RawReference::new(event, false)?;
                let numeric = bj::RawReference::new(event, true)?;
                let source = Source {
                    wire: super::super::history::hash_json_in_store(s, wire.view(), None, None)?,
                    numeric: super::super::history::hash_json_in_store(
                        s,
                        numeric.view(),
                        None,
                        None,
                    )?,
                    raw: super::super::history::hash_json_in_store(s, event, None, None)?,
                    bar: source_pending::raw_bar(s, event, None, None)?,
                };
                self.work.raw_hashes += 4;
                self.raw.put(s, new.coord, source)?;
                self.raw_ordinals.put(s, new.coord, new.ordinal)?;
            }
        }
        let mut first_membership = None;
        for coord in touched.iter().copied() {
            dirty_from(s, &self.raw_objects, &coord, dirty_objects)?;
            let current = self.raw.contains_key(&coord);
            if old_presence.get(&coord).copied() != Some(current) {
                first_membership = Some(first_membership.map_or(coord, |p: i64| p.min(coord)));
            }
            if current {
                self.raw_order.insert(s, coord)?;
            } else {
                self.raw_order.remove(&coord)?;
            }
            self.coverage_at(s, coord)?;
        }
        Ok(first_membership)
    }
}
struct ObjectKeys<'a> {
    at: Handle,
    order: &'a Order,
    events: &'a mut Tree<(usize, Handle)>,
    ids: &'a mut Map<String, Handle>,
    group_ids: &'a mut Map<i64, Handle>,
    groups: &'a mut Map<i64, Group>,
    group: Option<i64>,
}
impl ObjectSlots for ObjectKeys<'_> {
    fn insert_id(&mut self, s: &(impl CodecRead + ?Sized), id: &str) -> Result<bool, String> {
        before(s, self.ids, owned(s, id)?, self.at, self.order, self.events)
    }
    fn insert_group(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        index: i64,
        group: Group,
    ) -> Result<bool, String> {
        if before(s, self.group_ids, index, self.at, self.order, self.events)? {
            return Ok(true);
        }
        self.group = Some(index);
        self.groups.put(s, index, group)?;
        Ok(false)
    }
}
impl NativeSourceState {
    fn object_phase(
        &mut self,
        s: &mut SourceMaterial<'_, '_, '_, '_>,
        producer: &Cc004Producer,
        prefix: usize,
        mut dirty: Tree<Handle>,
        dirty_groups: &mut Tree<i64>,
        dirty_shapes: &mut Tree<Handle>,
        dirty_witness: &mut Tree<Handle>,
        pending_changed: &mut Tree<Handle>,
    ) -> Result<(), String> {
        let changes = apply_order(
            s,
            Stream::Array(SourceArray::Objects),
            &mut self.object_sequence,
            &mut self.object_order,
            &mut self.work,
        )?;
        for (handle, _) in &changes.removed {
            dirty.remove(handle)?;
            pending_changed.insert(s, *handle)?;
            self.remove_object(s, *handle, dirty_groups, dirty_witness)?;
        }
        let mut events = Tree::default();
        for handle in dirty.iter().copied() {
            self.remove_object(s, handle, dirty_groups, dirty_witness)?;
            pending_changed.insert(s, handle)?;
            schedule(s, &self.object_order, &mut events, handle)?;
        }
        for handle in changes.added {
            pending_changed.insert(s, handle)?;
            schedule(s, &self.object_order, &mut events, handle)?;
        }
        let mut checked = bj::new_map(None);
        while let Some((rank, handle)) = events.first().copied() {
            events.remove(&(rank, handle))?;
            self.work.order_events += 1;
            if self.objects.contains_key(&handle) {
                // A newly inserted earlier duplicate scheduled this retained record. Read at its
                // original stage so id failure precedes payload, group failure follows payload.
                self.remove_object(s, handle, dirty_groups, dirty_witness)?;
                pending_changed.insert(s, handle)?;
            }
            let record = member(s, Stream::Array(SourceArray::Objects), rank)?;
            if digest(&record)? != self.object_order.value(handle)? {
                return Err(err("当前对象原件与材料差分不同"));
            }
            let mut keys = ObjectKeys {
                at: handle,
                order: &self.object_order,
                events: &mut events,
                ids: &mut self.ids,
                group_ids: &mut self.group_ids,
                groups: &mut self.groups,
                group: None,
            };
            let object = audit_object_record_with_slots(
                s,
                &record,
                &self.raw,
                producer,
                &mut keys,
                &mut checked,
                None,
                prefix,
                false,
                None,
                None,
            )?;
            let group = keys.group;
            drop(keys);
            self.work.object_local += 1;
            for coord in object.sources.iter() {
                link(s, &mut self.raw_objects, *coord, handle)?;
                link_ref(s, &mut self.raw_objects, coord, handle)?;
            }
            if object.kind == CC006_SHAPE_KIND {
                dirty_shapes.insert(s, handle)?;
            }
            if let Some(index) = group {
                let group = self
                    .groups
                    .remove(&index)?
                    .ok_or_else(|| err("组贡献未写入"))?;
                self.group_changed(s, index, &group, true)?;
                self.groups.put(s, index, group)?;
                dirty_groups.insert(s, index)?;
            }
            s.native_admit(std::mem::size_of::<Object>() + 2 * std::mem::size_of::<usize>())
                .map_err(err)?;
            self.objects.put(
                s,
                handle,
                Entry {
                    value: Rc::new(object),
                    group,
                },
            )?;
        }
        for (handle, _) in changes.removed {
            self.object_order.reclaim(s, handle)?;
        }
        Ok(())
    }
}
impl NativeSourceState {
    fn remove_step(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        handle: Handle,
        dirty_requests: &mut Tree<Handle>,
        sequence: &mut Tree<i64>,
        old: &mut Map<i64, StepState>,
    ) -> Result<(), String> {
        if let Some(coord) = self.step_handles.remove(&handle)? {
            if self
                .steps
                .get(&coord)
                .is_some_and(|step| step.handle == handle)
            {
                let step = self
                    .steps
                    .remove(&coord)?
                    .ok_or_else(|| err("步骤贡献悬空"))?;
                dirty_from(s, &self.step_requests, &coord, dirty_requests)?;
                dirty_from(
                    s,
                    &self.request_subjects,
                    step.data.object.id.as_str(),
                    dirty_requests,
                )?;
                self.step_subjects.remove(step.data.object.id.as_str())?;
                self.step_coords.remove(&coord)?;
                self.pending.remove(&coord)?;
                self.missing_requests.remove(&coord)?;
                self.coverage_at(s, coord)?;
                sequence.insert(s, coord)?;
                old.put(s, coord, step)?;
            }
        }
        if self.knowledge == Some(handle) {
            self.knowledge = None;
        }
        Ok(())
    }
    fn pending_phase(
        &mut self,
        s: &mut SourceMaterial<'_, '_, '_, '_>,
        changed: &Tree<Handle>,
        raw_membership: Option<i64>,
        dirty_groups: &Tree<i64>,
    ) -> Result<(), String> {
        let mut sequence = Tree::default();
        let mut dirty_requests = Tree::default();
        let mut old = Map::default();
        let mut events = Tree::default();
        let mut knowledge_changed = false;
        for handle in changed.iter().copied() {
            knowledge_changed |= self.knowledge == Some(handle);
            self.remove_step(s, handle, &mut dirty_requests, &mut sequence, &mut old)?;
            if self.objects.get(&handle).is_some_and(|entry| {
                entry.value.kind == CC004_STEP_KIND || entry.value.kind == CC054_KNOWLEDGE_KIND
            }) {
                schedule(s, &self.object_order, &mut events, handle)?;
            }
        }
        while let Some((rank, handle)) = events.first().copied() {
            events.remove(&(rank, handle))?;
            self.work.order_events += 1;
            let object = Rc::clone(
                &self
                    .objects
                    .get(&handle)
                    .ok_or_else(|| err("pending对象贡献悬空"))?
                    .value,
            );
            if object.kind == CC054_KNOWLEDGE_KIND {
                source_pending::knowledge_record(s, &object, &mut None)?;
                if let Some(other) = self.knowledge {
                    if other != handle {
                        if self.object_order.rank(other)? < rank {
                            return Err(err("同cut CC054知识域重复或不符"));
                        }
                        schedule(s, &self.object_order, &mut events, other)?;
                    }
                }
                self.knowledge = Some(handle);
                knowledge_changed = true;
            } else {
                let (coord, data) = source_pending::StepContribution::bind(s, object, &self.raw)?;
                self.work.step_local += 1;
                if let Some(other) = self.steps.get(&coord).map(|step| step.handle) {
                    if other != handle {
                        if self.object_order.rank(other)? < rank {
                            return Err(err("同cut CC004步骤坐标重复"));
                        }
                        schedule(s, &self.object_order, &mut events, other)?;
                    }
                }
                dirty_from(s, &self.step_requests, &coord, &mut dirty_requests)?;
                dirty_from(
                    s,
                    &self.request_subjects,
                    data.object.id.as_str(),
                    &mut dirty_requests,
                )?;
                self.step_subjects
                    .put(s, owned(s, data.object.id.as_str())?, coord)?;
                self.step_handles.put(s, handle, coord)?;
                self.step_coords.insert(s, coord)?;
                sequence.insert(s, coord)?;
                let (seed, unsettled, waiting) = if let Some(previous) = old.remove(&coord)? {
                    (previous.seed, previous.unsettled, previous.waiting)
                } else {
                    (None, false, false)
                };
                self.steps.put(
                    s,
                    coord,
                    StepState {
                        handle,
                        data,
                        seed,
                        unsettled,
                        waiting,
                    },
                )?;
                if !self.request_bound.contains_key(&coord) {
                    self.missing_requests.insert(s, coord)?;
                }
            }
        }
        if self.steps.len() != self.raw.len() {
            return Err(err("同cut CC004未一一覆盖生效原始账"));
        }
        // 真实 raw 成员前缀改变的后缀都是受影响依赖；只枚举这一实际 fanout。
        if let Some(start) = raw_membership {
            for coord in self.step_coords.range(start..).copied() {
                sequence.insert(s, coord)?;
            }
        }
        // 已删除步骤的后继仍可能改变 seed/waiting 边界。
        let mut successors = Vec::new();
        for coord in sequence.iter() {
            if !self.steps.contains_key(coord) {
                if let Some(next) = self.step_coords.range(*coord..).next() {
                    vec_push(s, &mut successors, *next)?;
                }
            }
        }
        for coord in successors {
            sequence.insert(s, coord)?;
        }
        let mut sequence_changed = false;
        while let Some(coord) = sequence.first().copied() {
            sequence.remove(&coord)?;
            let Some(mut step) = self.steps.remove(&coord)? else {
                continue;
            };
            sequence_changed = true;
            self.work.step_sequence += 1;
            let previous = self
                .step_coords
                .range(..coord)
                .next_back()
                .and_then(|at| self.steps.get(at));
            let mut seed = previous
                .and_then(|p| p.seed.as_ref())
                .map(|r| s.native_ref_clone(r).map_err(err))
                .transpose()?;
            let mut unsettled = previous.is_some_and(|p| p.unsettled);
            let ordinal = if self.raw_order.first() == Some(&coord) {
                0
            } else if self.raw_order.range(..coord).next_back() == self.raw_order.first() {
                1
            } else {
                2
            };
            let prefix_equal = step.data.object.sources.iter().copied().eq(self
                .raw_order
                .range(..=coord)
                .copied()
                .inspect(|_| self.work.prefix_members_checked += 1));
            let mut pending = bj::new_work(None);
            source_pending::step_sequence_record_view(
                s,
                &step.data,
                &coord,
                ordinal,
                prefix_equal,
                &mut seed,
                &mut unsettled,
                &mut pending,
            )?;
            let boundary_changed = step.seed != seed || step.unsettled != unsettled;
            let waiting = !pending.is_empty();
            step.seed = seed;
            step.unsettled = unsettled;
            step.waiting = waiting;
            if waiting {
                self.pending.insert(s, coord)?;
            } else {
                self.pending.remove(&coord)?;
            }
            self.coverage_at(s, coord)?;
            self.steps.put(s, coord, step)?;
            if boundary_changed {
                if let Some(next) = self
                    .step_coords
                    .range((std::ops::Bound::Excluded(coord), std::ops::Bound::Unbounded))
                    .next()
                    .copied()
                {
                    sequence.insert(s, next)?;
                }
            }
        }
        if let Some(last) = self
            .step_coords
            .last()
            .and_then(|coord| self.steps.get(coord))
        {
            // 唯一 seed 组的判据还依赖 groups.len()：新增/撤销任何组都必须重核。
            if last.unsettled && (sequence_changed || !dirty_groups.is_empty()) {
                let coord = *self
                    .raw_order
                    .first()
                    .ok_or_else(|| err("raw缺当前CC004步骤"))?;
                source_pending::unsettled_seed_group(s, coord, &self.groups, &last.seed)?;
            }
        }
        let knowledge = self.knowledge.ok_or_else(|| err("同cut缺CC054知识原件"))?;
        let object = &self
            .objects
            .get(&knowledge)
            .ok_or_else(|| err("知识贡献悬空"))?
            .value;
        if (knowledge_changed || raw_membership.is_some())
            && !object.sources.iter().copied().eq(self
                .raw_order
                .iter()
                .copied()
                .inspect(|_| self.work.prefix_members_checked += 1))
        {
            return Err(err("CC054来源不是同cut全部生效原始账"));
        }
        let ordinal = self.object_order.rank(knowledge)?;
        self.request_phase(s, ordinal, dirty_requests)?;
        Ok(())
    }
    fn remove_request(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        handle: Handle,
    ) -> Result<(), String> {
        let request = self
            .requests
            .remove(&handle)?
            .ok_or_else(|| err("请求贡献悬空"))?;
        forget(&mut self.request_keys, request.key.as_str(), handle)?;
        if let Some(subject) = request.subject {
            unlink(&mut self.request_subjects, subject.as_str(), handle)?;
        }
        if let Some(coord) = request.coord {
            unlink(&mut self.step_requests, &coord, handle)?;
            forget(&mut self.request_bound, &coord, handle)?;
            if self.steps.contains_key(&coord) && !self.request_bound.contains_key(&coord) {
                self.missing_requests.insert(s, coord)?;
            }
        }
        Ok(())
    }
    fn request_phase(
        &mut self,
        s: &mut SourceMaterial<'_, '_, '_, '_>,
        knowledge: usize,
        mut dirty: Tree<Handle>,
    ) -> Result<(), String> {
        let stream = Stream::Requests(u64::try_from(knowledge).map_err(|_| err("知识位置溢出"))?);
        let changes = apply_order(
            s,
            stream,
            &mut self.request_sequence,
            &mut self.request_order,
            &mut self.work,
        )?;
        for (handle, _) in &changes.removed {
            dirty.remove(handle)?;
            self.remove_request(s, *handle)?;
        }
        let mut events = Tree::default();
        for handle in dirty.iter().copied() {
            if self.requests.contains_key(&handle) {
                self.remove_request(s, handle)?;
                schedule(s, &self.request_order, &mut events, handle)?;
            }
        }
        for handle in changes.added {
            schedule(s, &self.request_order, &mut events, handle)?;
        }
        while let Some((rank, handle)) = events.first().copied() {
            events.remove(&(rank, handle))?;
            self.work.order_events += 1;
            self.work.request_local += 1;
            if self.requests.contains_key(&handle) {
                self.remove_request(s, handle)?;
            }
            let request = member(s, stream, rank)?;
            if digest(&request)? != self.request_order.value(handle)? {
                return Err(err("请求原件与材料差分不同"));
            }
            let lookup = NativeSteps {
                steps: &self.steps,
                subjects: &self.step_subjects,
            };
            let mut keys = RequestKeys {
                at: handle,
                order: &self.request_order,
                events: &mut events,
                keys: &mut self.request_keys,
                bound: &mut self.request_bound,
            };
            source_pending::request_record_with_slots(s, &request, &lookup, &mut keys)?;
            drop(keys);
            // 成功后才记录逆依赖，不提前解码未来请求以构造索引。
            let key = text(s, &request, "request_id")?;
            let parsed = parsed_json(s, key.as_bytes())?;
            let coord = if parsed.view().index(0).and_then(|v| v.as_str()) == Some(CC004_STEP_KIND)
            {
                Some(decimal(
                    parsed
                        .view()
                        .index(1)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| err("步骤请求槽位非坐标"))?,
                )?)
            } else {
                None
            };
            drop(parsed);
            let subject = scalar(s, &request, "subject_id")?;
            let subject = subject.view().as_str().map(|id| owned(s, id)).transpose()?;
            if let Some(id) = &subject {
                link(s, &mut self.request_subjects, owned(s, id)?, handle)?;
                link_ref(s, &mut self.request_subjects, id.as_str(), handle)?;
            }
            if let Some(coord) = coord {
                link(s, &mut self.step_requests, coord, handle)?;
                link_ref(s, &mut self.step_requests, &coord, handle)?;
                self.missing_requests.remove(&coord)?;
            }
            self.requests.put(
                s,
                handle,
                Request {
                    key: owned(s, key.as_str())?,
                    subject,
                    coord,
                },
            )?;
        }
        if !self.missing_requests.is_empty() {
            return Err(err("CC004缺一一绑定的知识请求"));
        }
        for (handle, _) in changes.removed {
            self.request_order.reclaim(s, handle)?;
        }
        Ok(())
    }
}
struct NativeSteps<'a> {
    steps: &'a Map<i64, StepState>,
    subjects: &'a Map<String, i64>,
}
impl StepLookup for NativeSteps<'_> {
    fn get_step(&self, coord: i64) -> Option<&dyn StepView> {
        self.steps.get(&coord).map(|s| &s.data as &dyn StepView)
    }
    fn contains_subject(&self, id: &str) -> bool {
        self.subjects.contains_key(id)
    }
}
struct RequestKeys<'a> {
    at: Handle,
    order: &'a Order,
    events: &'a mut Tree<(usize, Handle)>,
    keys: &'a mut Map<String, Handle>,
    bound: &'a mut Map<i64, Handle>,
}
impl RequestSlots for RequestKeys<'_> {
    fn insert_request(&mut self, s: &(impl CodecRead + ?Sized), key: &str) -> Result<bool, String> {
        before(
            s,
            self.keys,
            owned(s, key)?,
            self.at,
            self.order,
            self.events,
        )
    }
    fn insert_bound(&mut self, s: &(impl CodecRead + ?Sized), coord: i64) -> Result<bool, String> {
        before(s, self.bound, coord, self.at, self.order, self.events)
    }
}
impl NativeSourceState {
    fn partition_phase(&self) -> Result<(), String> {
        let gap = self.gaps.first().copied();
        let overlap = self.overlaps.first().copied();
        if gap.is_some_and(|index| overlap.is_none_or(|(at, _)| index <= at)) {
            return Err(err("组合并序号不连续"));
        }
        if overlap.is_some() {
            return Err(err("同cut组成员重叠"));
        }
        if !self.pending_overlap.is_empty() {
            return Err(err("同cut组成员与初始未定来源重叠"));
        }
        if !self.coverage.is_empty() {
            return Err(err("同cut组未完整覆盖生效原始账"));
        }
        Ok(())
    }
    fn shape_phase(
        &mut self,
        s: &mut SourceMaterial<'_, '_, '_, '_>,
        producer: &Cc004Producer,
        prefix: usize,
        dirty_groups: &Tree<i64>,
        mut dirty: Tree<Handle>,
        dirty_witness: &mut Tree<Handle>,
    ) -> Result<(), String> {
        for index in dirty_groups.iter() {
            dirty_from(s, &self.group_shapes, index, &mut dirty)?;
        }
        let mut events = Tree::default();
        for handle in dirty.iter().copied() {
            if self.objects.contains_key(&handle) {
                schedule(s, &self.object_order, &mut events, handle)?;
            }
        }
        let mut checked = bj::new_map(None);
        while let Some((rank, handle)) = events.first().copied() {
            events.remove(&(rank, handle))?;
            self.work.order_events += 1;
            let object = Rc::clone(
                &self
                    .objects
                    .get(&handle)
                    .ok_or_else(|| err("形态贡献悬空"))?
                    .value,
            );
            self.remove_shape(s, object.id.as_str(), handle, dirty_witness)?;
            let shape = audit_shape_record(
                s,
                &object,
                &self.raw,
                &self.groups,
                producer,
                &mut checked,
                None,
                prefix,
                None,
                None,
            )?;
            self.work.shape_local += 1;
            for index in shape.groups {
                link(s, &mut self.group_shapes, index, handle)?;
                link_ref(s, &mut self.group_shapes, &index, handle)?;
            }
            let key = owned(s, object.id.as_str())?;
            let key_bytes = key.capacity();
            self.shapes.put(s, key, shape)?;
            self.shape_slots
                .put(s, owned(s, object.id.as_str())?, (0, key_bytes))?;
            self.missing_shapes
                .insert(s, owned(s, object.id.as_str())?)?;
        }
        Ok(())
    }
    fn mask(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        slot: i64,
        add: bool,
    ) -> Result<(), String> {
        if self.shape_slots.contains_key(id) {
            self.shape_slots.mutate(id, |mask| {
                if add {
                    mask.0 |= 1u8 << (slot as u32);
                } else {
                    mask.0 &= !(1u8 << (slot as u32));
                }
                Ok(())
            })?;
            if self.shape_slots.get(id).is_some_and(|mask| mask.0 == 7) {
                self.missing_shapes.remove(id)?;
            } else {
                self.missing_shapes.insert(s, owned(s, id)?)?;
            }
        }
        Ok(())
    }
    fn remove_witness(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        handle: Handle,
    ) -> Result<(), String> {
        let witness = self
            .witnesses
            .remove(&handle)?
            .ok_or_else(|| err("见证贡献悬空"))?;
        let key = (owned(s, &witness.id)?, witness.slot);
        if self.slots.get(&key) == Some(&handle) {
            self.slots.remove(&key)?;
            self.mask(s, &witness.id, witness.slot, false)?;
        }
        unlink(&mut self.shape_witnesses, witness.id.as_str(), handle)?;
        Ok(())
    }
    fn witness_phase(
        &mut self,
        s: &mut SourceMaterial<'_, '_, '_, '_>,
        mut dirty: Tree<Handle>,
    ) -> Result<(), String> {
        let changes = apply_order(
            s,
            Stream::Array(SourceArray::Witnesses),
            &mut self.witness_sequence,
            &mut self.witness_order,
            &mut self.work,
        )?;
        for (handle, _) in &changes.removed {
            dirty.remove(handle)?;
            self.remove_witness(s, *handle)?;
        }
        let mut events = Tree::default();
        for handle in dirty.iter().copied() {
            if self.witnesses.contains_key(&handle) {
                self.remove_witness(s, handle)?;
                schedule(s, &self.witness_order, &mut events, handle)?;
            }
        }
        for handle in changes.added {
            schedule(s, &self.witness_order, &mut events, handle)?;
        }
        while let Some((rank, handle)) = events.first().copied() {
            events.remove(&(rank, handle))?;
            self.work.order_events += 1;
            if self.witnesses.contains_key(&handle) {
                self.remove_witness(s, handle)?;
            }
            let record = member(s, Stream::Array(SourceArray::Witnesses), rank)?;
            if digest(&record)? != self.witness_order.value(handle)? {
                return Err(err("见证原件与材料差分不同"));
            }
            let mut slots = WitnessKeys {
                at: handle,
                order: &self.witness_order,
                events: &mut events,
                slots: &mut self.slots,
            };
            let (id, slot) = audit_witness_record_with_slots(
                s,
                &record,
                &self.raw,
                &self.groups,
                &self.shapes,
                &mut slots,
                None,
                None,
            )?;
            drop(slots);
            self.work.witness_local += 1;
            self.mask(s, id.as_str(), slot, true)?;
            link(s, &mut self.shape_witnesses, owned(s, id.as_str())?, handle)?;
            link_ref(s, &mut self.shape_witnesses, id.as_str(), handle)?;
            self.witnesses.put(
                s,
                handle,
                Witness {
                    id: owned(s, id.as_str())?,
                    slot,
                },
            )?;
        }
        if !self.missing_shapes.is_empty() {
            return Err(err("形态缺完整三槽见证"));
        }
        for (handle, _) in changes.removed {
            self.witness_order.reclaim(s, handle)?;
        }
        Ok(())
    }
    fn observation_phase(&mut self, s: &mut SourceMaterial<'_, '_, '_, '_>) -> Result<(), String> {
        let stream = Stream::Array(SourceArray::Observations);
        let d = delta(s, stream)?;
        if d.old != self.observation_sequence {
            return Err(err("观察数组前根与贡献不同"));
        }
        let current = d.current;
        let len = d.edits.len();
        let mut additions = Vec::new();
        for i in 0..len {
            match delta(s, stream)?.edits[i] {
                Edit::Remove { value, .. } => {
                    let old = self
                        .observations
                        .get(&value)
                        .ok_or_else(|| err("观察贡献缺失"))?;
                    self.insufficient = self
                        .insufficient
                        .checked_sub(usize::from(old.insufficient))
                        .ok_or_else(|| err("观察计数下溢"))?;
                    self.outside = self
                        .outside
                        .checked_sub(usize::from(old.outside))
                        .ok_or_else(|| err("观察计数下溢"))?;
                    if old.count == 1 {
                        self.observations.remove(&value)?;
                    } else {
                        self.observations.mutate(&value, |old| {
                            old.count -= 1;
                            Ok(())
                        })?;
                    }
                }
                Edit::Add { ordinal, value } => vec_push(s, &mut additions, (ordinal, value))?,
                Edit::Retain { .. } => {
                    self.work.retained_spans += 1;
                }
            }
        }
        additions.sort_unstable_by(|a, b| {
            self.work.sort_comparisons += 1;
            a.0.cmp(&b.0)
        });
        for (ordinal, digest_want) in additions {
            let record = s.array_member(SourceArray::Observations, ordinal)?;
            if digest(&record)? != digest_want {
                return Err(err("观察原件与材料差分不同"));
            }
            let kind = text(s, &record, "kind")?;
            let insufficient = kind == "insufficient_knowledge";
            let outside = kind == "domain_not_satisfied";
            self.work.observations += 1;
            self.insufficient = self
                .insufficient
                .checked_add(usize::from(insufficient))
                .ok_or_else(|| err("观察计数溢出"))?;
            self.outside = self
                .outside
                .checked_add(usize::from(outside))
                .ok_or_else(|| err("观察计数溢出"))?;
            if self.observations.contains_key(&digest_want) {
                self.observations.mutate(&digest_want, |v| {
                    v.count = v.count.checked_add(1).ok_or_else(|| err("观察计数溢出"))?;
                    Ok(())
                })?;
            } else {
                self.observations.put(
                    s,
                    digest_want,
                    Observation {
                        count: 1,
                        insufficient,
                        outside,
                    },
                )?;
            }
        }
        self.observation_sequence = current;
        Ok(())
    }
}
struct WitnessKeys<'a> {
    at: Handle,
    order: &'a Order,
    events: &'a mut Tree<(usize, Handle)>,
    slots: &'a mut Map<(String, i64), Handle>,
}
impl WitnessSlots for WitnessKeys<'_> {
    fn reserve(&mut self, _: &(impl CodecRead + ?Sized)) -> Result<(), String> {
        Ok(())
    }
    fn insert_slot(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        id: &str,
        slot: i64,
    ) -> Result<bool, String> {
        before(
            s,
            self.slots,
            (owned(s, id)?, slot),
            self.at,
            self.order,
            self.events,
        )
    }
}

pub(crate) fn audit_native_bound(
    previous: Option<NativeSourceState>,
    material: &mut SourceMaterial<'_, '_, '_, '_>,
    raw: &BoundRawAdvance<'_>,
    profile: &str,
    rule: &str,
) -> Result<NativeSourceVerdict, String> {
    #[cfg(test)]
    let before_calls = source_call_stats();
    let result = (|| {
        // MaterialSource 的构造已在原 closure 阶段完整核定资格；此函数不能自行补签。
        raw.ensure_current()?;
        if profile != tb02a::PROFILE {
            return Err(err(
                "当前来源审计仅实现OHLC档；其他档仍待接线，拒绝按此口径放行",
            ));
        }
        if material.allocation_budget().is_some() || raw.raw().budget().is_some() {
            return Err(err("Native来源增量不接Owned账"));
        }
        if material.codec_work().is_none() {
            return Err(err("Native来源增量缺原父工作账"));
        }
        let mut state = match previous {
            Some(state) => state,
            None => NativeSourceState::empty(material, raw, material.root(), rule)?,
        };
        if state.rule != rule {
            return Err(err("来源贡献规则域改变"));
        }
        let old_path_visits = state.object_order.visits.get()
            + state.witness_order.visits.get()
            + state.request_order.visits.get();
        state.work = SourceWork::default();
        material
            .native_admit(
                profile
                    .len()
                    .checked_add(rule.len())
                    .and_then(|n| n.checked_add(2 * std::mem::size_of::<[usize; 2]>()))
                    .ok_or_else(|| err(material.native_overflow()))?,
            )
            .map_err(err)?;
        let producer = Cc004Producer::new(profile, rule);
        let mut dirty_objects = Tree::default();
        let mut dirty_groups = Tree::default();
        let mut dirty_shapes = Tree::default();
        let mut dirty_witness = Tree::default();
        let mut pending_changed = Tree::default();
        let membership = state.raw_advance(material, raw, &mut dirty_objects)?;
        #[cfg(test)]
        super::super::state::trace_event("source_objects", None);
        state.object_phase(
            material,
            &producer,
            raw.current_prefix(),
            dirty_objects,
            &mut dirty_groups,
            &mut dirty_shapes,
            &mut dirty_witness,
            &mut pending_changed,
        )?;
        state.pending_phase(material, &pending_changed, membership, &dirty_groups)?;
        state.partition_phase()?;
        #[cfg(test)]
        super::super::state::trace_event("source_shapes", None);
        state.shape_phase(
            material,
            &producer,
            raw.current_prefix(),
            &dirty_groups,
            dirty_shapes,
            &mut dirty_witness,
        )?;
        #[cfg(test)]
        super::super::state::trace_event("source_witnesses", None);
        state.witness_phase(material, dirty_witness)?;
        #[cfg(test)]
        super::super::state::trace_event("source_observations", None);
        state.observation_phase(material)?;
        raw.ensure_current()?;
        state.work.order_path_visits = state.object_order.visits.get()
            + state.witness_order.visits.get()
            + state.request_order.visits.get()
            - old_path_visits;
        state.root = material.root();
        state.raw_binding = raw.current_binding();
        state.raw_prefix = raw.current_prefix();
        if state.retained_bytes() == usize::MAX {
            return Err(err("来源总保留费溢出"));
        }
        #[cfg(test)]
        {
            let after = source_call_stats();
            state.work.full_source_calls = after.full_source_calls - before_calls.full_source_calls;
            state.work.full_pending_calls =
                after.full_pending_calls - before_calls.full_pending_calls;
            state.work.full_members_enumerations =
                after.full_members_enumerations - before_calls.full_members_enumerations;
        }
        Ok(NativeSourceVerdict {
            binding: material.binding(),
            counts: state.counts(),
            state,
        })
    })();
    if result.is_err() {
        raw.invalidate_after_consumer_failure();
    }
    result
}

#[cfg(test)]
mod component_tests {
    use super::*;
    #[test]
    fn b2_source_retained_tree_uses_actual_string_capacity_and_symmetric_release() {
        let s = sc::MemoryNodes::default();
        let mut map = Map::<String, String>::default();
        let mut key = String::with_capacity(128);
        key.push_str("slot");
        let key_cap = key.capacity();
        let mut value = String::with_capacity(1024);
        value.push_str("payload");
        let value_cap = value.capacity();
        map.put(&s, key, value).unwrap();
        assert_eq!(
            map.heap_bytes(),
            retention::tree_slots::<String, String>(1) + key_cap + value_cap
        );
        let mut replacement_key = String::with_capacity(4096);
        replacement_key.push_str("slot");
        let mut replacement = String::with_capacity(32);
        replacement.push_str("next");
        let next_cap = replacement.capacity();
        map.put(&s, replacement_key, replacement).unwrap();
        assert_eq!(
            map.heap_bytes(),
            retention::tree_slots::<String, String>(1) + key_cap + next_cap
        );
        assert_eq!(map.remove("slot").unwrap().unwrap(), "next");
        assert_eq!(map.heap_bytes(), 0);
        assert!(map.values.is_empty());
    }
    #[test]
    fn b2_source_nested_reverse_index_charges_each_member_and_clears_empty_root() {
        let s = sc::MemoryNodes::default();
        let mut map = Map::<i64, Tree<usize>>::default();
        map.put(&s, 7, Tree::default()).unwrap();
        let base = map.heap_bytes();
        map.mutate(&7, |set| set.insert(&s, 11).map(|_| ()))
            .unwrap();
        assert_eq!(
            map.heap_bytes(),
            base + retention::tree_slots::<usize, ()>(1)
        );
        map.mutate(&7, |set| set.insert(&s, 11).map(|_| ()))
            .unwrap();
        assert_eq!(
            map.heap_bytes(),
            base + retention::tree_slots::<usize, ()>(1)
        );
        unlink(&mut map, &7, 11).unwrap();
        assert!(map.is_empty());
        assert_eq!(map.heap_bytes(), 0);
    }
    #[test]
    fn b2_source_duplicate_key_is_reported_at_second_current_occurrence() {
        let s = sc::MemoryNodes::default();
        let mut order = Order::default();
        let first = order.insert(&s, 0, [1; 32]).unwrap();
        let last = order.insert(&s, 1, [2; 32]).unwrap();
        let mut index = Map::default();
        index.put(&s, owned(&s, "same").unwrap(), last).unwrap();
        let mut events = Tree::default();
        assert!(!before(
            &s,
            &mut index,
            owned(&s, "same").unwrap(),
            first,
            &order,
            &mut events
        )
        .unwrap());
        assert_eq!(events.first(), Some(&(1, last)));
        assert!(before(
            &s,
            &mut index,
            owned(&s, "same").unwrap(),
            last,
            &order,
            &mut events
        )
        .unwrap());
    }
}
