//! #1640 ordinary 单 open 成功配方。集合效应仍在当前 cut 原位执行。
use super::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

type Key = (String, String, &'static str, Option<u64>);
fn key(r: sc::RefView<'_>) -> Key {
    (
        r.format().into(),
        r.root().into(),
        r.kind().name(),
        r.extent(),
    )
}
fn reference(r: sc::RefView<'_>) -> ContentRef {
    ContentRef {
        format: r.format().into(),
        root: r.root().into(),
        kind: r.kind(),
        extent: r.extent(),
    }
}
fn ref_matches(expected: &ContentRef, actual: sc::RefView<'_>) -> bool {
    expected.format == actual.format()
        && expected.root == actual.root()
        && expected.kind == actual.kind()
        && expected.extent == actual.extent()
}
fn ref_bytes(r: &ContentRef) -> usize {
    r.format.capacity() + r.root.capacity()
}
type Fields = Vec<(&'static str, ContentRef)>;
fn fields(f: &SourceFields<'static>, names: &[&'static str]) -> Fields {
    names
        .iter()
        .filter_map(|&n| f.0.get(n).map(|r| (n, reference(r))))
        .collect()
}
fn restore_fields(f: &Fields) -> Result<SourceFields<'static>, PublishError> {
    NativeRefFields::try_from_views(f.iter().map(|(n, r)| (*n, r.into())), None)
        .map(|f| SourceFields(super::super::audit_store::RefFields::Plain(f)))
        .map_err(err)
}
fn fields_match(expected: &Fields, actual: &SourceFields<'_>) -> bool {
    expected
        .iter()
        .all(|(name, value)| actual.0.get(name).is_some_and(|r| ref_matches(value, r)))
}
fn fields_bytes(f: &Fields) -> usize {
    f.capacity() * std::mem::size_of::<(&str, ContentRef)>()
        + f.iter().map(|(_, r)| ref_bytes(r)).sum::<usize>()
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ObjectData {
    id: String,
    kind: String,
    fields: Fields,
    refs: ContentRef,
    sources: Vec<i64>,
}
impl ObjectData {
    pub(super) fn read(o: &Object) -> Self {
        let names: &[&'static str] = if o.kind == CC006_SHAPE_KIND {
            &SHAPE_SOURCE_FIELDS
        } else {
            &TYPED_SOURCE_FIELDS
        };
        Self {
            id: o.id.as_str().into(),
            kind: o.kind.as_str().into(),
            fields: fields(&o.fields, names),
            refs: reference(o.refs.view()),
            sources: o.sources.to_vec(),
        }
    }
    fn matches(&self, object: &Object) -> bool {
        self.id == object.id.as_str()
            && self.kind == object.kind.as_str()
            && fields_match(&self.fields, &object.fields)
            && ref_matches(&self.refs, object.refs.view())
            && self.sources[..] == *object.sources
    }
    fn restore(&self, s: &(impl CodecRead + ?Sized)) -> Result<Object, PublishError> {
        Ok(Object {
            id: s.native_text(&self.id)?,
            kind: s.native_text(&self.kind)?,
            fields: restore_fields(&self.fields)?,
            refs: self.refs.clone().into(),
            sources: SourceCoords::Original(copy_coords(s, &self.sources)?),
        })
    }
    fn bytes(&self) -> usize {
        self.id.capacity()
            + self.kind.capacity()
            + fields_bytes(&self.fields)
            + ref_bytes(&self.refs)
            + self.sources.capacity() * 8
    }
}
#[derive(Clone, PartialEq, Eq)]
struct GroupData {
    payload: ContentRef,
    members: Vec<i64>,
    sources: Vec<i64>,
    anchor: i64,
}
impl GroupData {
    fn read(g: &Group) -> Self {
        Self {
            payload: reference(g.payload.view()),
            members: g.members.to_vec(),
            sources: g.sources.to_vec(),
            anchor: g.anchor,
        }
    }
    fn matches(&self, g: &Group) -> bool {
        ref_matches(&self.payload, g.payload.view())
            && self.members[..] == *g.members
            && self.sources[..] == *g.sources
            && self.anchor == g.anchor
    }
    fn restore(&self, s: &(impl CodecRead + ?Sized)) -> Result<Group, PublishError> {
        Ok(Group {
            payload: self.payload.clone().into(),
            members: SourceCoords::Original(copy_coords(s, &self.members)?),
            sources: SourceCoords::Original(copy_coords(s, &self.sources)?),
            anchor: self.anchor,
        })
    }
    fn bytes(&self) -> usize {
        ref_bytes(&self.payload) + (self.members.capacity() + self.sources.capacity()) * 8
    }
}
#[derive(Default)]
struct Dependencies {
    raw: Vec<(i64, Option<Source>)>,
    groups: Vec<(i64, Option<GroupData>)>,
    shapes: Vec<(String, Option<[i64; 3]>)>,
}
impl Dependencies {
    fn valid(
        &self,
        raw: &impl RawLookup,
        groups: &impl GroupLookup,
        shapes: &impl ShapeLookup,
    ) -> bool {
        self.raw.iter().all(|(c, v)| raw.get(c).copied() == *v)
            && self.groups.iter().all(|(i, v)| match (v, groups.get(i)) {
                (Some(a), Some(b)) => a.matches(b),
                (None, None) => true,
                _ => false,
            })
            && self
                .shapes
                .iter()
                .all(|(id, v)| shapes.get(id).map(|v| v.groups) == *v)
    }
    fn bytes(&self) -> usize {
        self.raw.capacity() * std::mem::size_of::<(i64, Option<Source>)>()
            + self.groups.capacity() * std::mem::size_of::<(i64, Option<GroupData>)>()
            + self
                .groups
                .iter()
                .filter_map(|(_, g)| g.as_ref())
                .map(GroupData::bytes)
                .sum::<usize>()
            + self.shapes.capacity() * std::mem::size_of::<(String, Option<[i64; 3]>)>()
            + self.shapes.iter().map(|(s, _)| s.capacity()).sum::<usize>()
    }
}
struct RawReads<'a, T: ?Sized> {
    raw: &'a T,
    deps: &'a RefCell<Dependencies>,
}
impl<T: RawLookup + ?Sized> RawLookup for RawReads<'_, T> {
    fn len(&self) -> usize {
        self.raw.len()
    }
    fn get(&self, c: &i64) -> Option<&Source> {
        let value = self.raw.get(c);
        self.deps.borrow_mut().raw.push((*c, value.copied()));
        value
    }
}
struct GroupReads<'a, T: ?Sized> {
    groups: &'a T,
    deps: &'a RefCell<Dependencies>,
}
impl<T: GroupLookup + ?Sized> GroupLookup for GroupReads<'_, T> {
    fn get(&self, i: &i64) -> Option<&Group> {
        let value = self.groups.get(i);
        self.deps
            .borrow_mut()
            .groups
            .push((*i, value.map(GroupData::read)));
        value
    }
    fn len(&self) -> usize {
        self.groups.len()
    }
}
struct ShapeReads<'a, T: ?Sized> {
    shapes: &'a T,
    deps: &'a RefCell<Dependencies>,
}
impl<T: ShapeLookup + ?Sized> ShapeLookup for ShapeReads<'_, T> {
    fn get(&self, id: &str) -> Option<&Shape> {
        let value = self.shapes.get(id);
        self.deps
            .borrow_mut()
            .shapes
            .push((id.into(), value.map(|s| s.groups)));
        value
    }
}
// checked_refs 可省去 RawLookup。成功结果的语义坐标仍必须逐项登记四摘要及存在性。
fn semantic_sources(deps: &RefCell<Dependencies>, sources: &[i64], raw: &impl RawLookup) {
    deps.borrow_mut()
        .raw
        .extend(sources.iter().map(|c| (*c, raw.get(c).copied())));
}
enum Output {
    Object(ObjectData, Option<(i64, GroupData)>),
    Shape([i64; 3]),
    Witness(String, i64),
    Step(StepData),
    Request(RequestData),
    Observation(String),
}
impl Output {
    fn bytes(&self) -> usize {
        match self {
            Self::Object(o, g) => o.bytes() + g.as_ref().map_or(0, |(_, g)| g.bytes()),
            Self::Shape(_) => 0,
            Self::Witness(s, _) => s.capacity(),
            Self::Step(s) => s.bytes(),
            Self::Request(r) => r.bytes(),
            Self::Observation(kind) => kind.capacity(),
        }
    }
}
struct Proof {
    deps: Dependencies,
    output: Output,
}
type ProofKey = (u8, Key);
#[derive(Default, Clone, Copy, Debug)]
pub(in crate::v3) struct ProofStats {
    pub hits: usize,
    pub executions: usize,
    pub invalidations: usize,
    pub retained_bytes: usize,
    pub peak_bytes: usize,
    pub released_bytes: usize,
    pub entries: usize,
    pub map_capacity: usize,
}
#[cfg(test)]
thread_local! { static STATS: std::cell::Cell<ProofStats> = const { std::cell::Cell::new(ProofStats {
    hits: 0, executions: 0, invalidations: 0, retained_bytes: 0, peak_bytes: 0,
    released_bytes: 0, entries: 0, map_capacity: 0,
}) }; }
#[cfg(all(test, feature = "s_incremental"))]
pub(in crate::v3) fn stats() -> ProofStats {
    STATS.with(std::cell::Cell::get)
}
#[cfg(all(test, feature = "s_incremental"))]
pub(in crate::v3) fn reset_stats() {
    STATS.with(|s| s.set(ProofStats::default()));
}

#[derive(Default)]
pub(in crate::v3) struct OrdinaryProofs {
    previous: HashMap<ProofKey, Rc<Proof>>,
    current: HashMap<ProofKey, Rc<Proof>>,
    profile: String,
    rule: String,
    object_records: HashMap<String, Key>,
    stats: ProofStats,
}
impl OrdinaryProofs {
    pub(in crate::v3) fn begin(&mut self, profile: &str, rule: &str) {
        if self.profile != profile || self.rule != rule {
            self.previous.clear();
            self.profile = profile.into();
            self.rule = rule.into();
        }
        self.current.clear();
        self.object_records.clear();
    }
    pub(in crate::v3) fn commit(&mut self) {
        self.measure();
        let before = self.stats.retained_bytes;
        self.previous = std::mem::take(&mut self.current);
        self.measure();
        self.stats.released_bytes += before.saturating_sub(self.stats.retained_bytes);
        self.publish();
    }
    fn publish(&self) {
        #[cfg(test)]
        STATS.with(|s| s.set(self.stats));
    }
    fn measure(&mut self) {
        let mut unique = std::collections::HashSet::new();
        let mut bytes = self.profile.capacity()
            + self.rule.capacity()
            + self.object_records.capacity() * (std::mem::size_of::<(String, Key)>() + 1)
            + self
                .object_records
                .iter()
                .map(|(id, k)| id.capacity() + k.0.capacity() + k.1.capacity())
                .sum::<usize>();
        for map in [&self.previous, &self.current] {
            bytes += map.capacity() * (std::mem::size_of::<(ProofKey, Rc<Proof>)>() + 1);
            for ((_, k), p) in map {
                bytes += k.0.capacity() + k.1.capacity();
                if unique.insert(Rc::as_ptr(p)) {
                    bytes += std::mem::size_of::<Proof>()
                        + 2 * std::mem::size_of::<usize>()
                        + p.deps.bytes()
                        + p.output.bytes();
                }
            }
        }
        self.stats.retained_bytes = bytes;
        self.stats.peak_bytes = self.stats.peak_bytes.max(bytes);
        self.stats.entries = unique.len();
        self.stats.map_capacity = self.previous.capacity() + self.current.capacity();
        self.publish();
    }
    fn candidate(
        &mut self,
        phase: u8,
        k: &Key,
        raw: &impl RawLookup,
        groups: &impl GroupLookup,
        shapes: &impl ShapeLookup,
    ) -> Option<Rc<Proof>> {
        let p = self.previous.get(&(phase, k.clone()))?;
        if p.deps.valid(raw, groups, shapes) {
            self.stats.hits += 1;
            self.publish();
            Some(Rc::clone(p))
        } else {
            self.stats.invalidations += 1;
            self.publish();
            None
        }
    }
    fn save(&mut self, phase: u8, k: Key, deps: Dependencies, output: Output) {
        self.stats.executions += 1;
        self.current
            .insert((phase, k), Rc::new(Proof { deps, output }));
        self.publish();
    }
    fn retain(&mut self, phase: u8, k: Key, p: Rc<Proof>) {
        self.current.insert((phase, k), p);
    }
}
impl Drop for OrdinaryProofs {
    fn drop(&mut self) {
        self.measure();
        self.stats.released_bytes += self.stats.retained_bytes;
        self.stats.retained_bytes = 0;
        self.stats.entries = 0;
        self.stats.map_capacity = 0;
        self.publish();
    }
}
struct ObjectWrites<'a, T> {
    slots: &'a mut T,
    group: Option<(i64, GroupData)>,
}
impl<T: ObjectSlots> ObjectSlots for ObjectWrites<'_, T> {
    fn reserve_group(&mut self, s: &(impl CodecRead + ?Sized)) -> Result<(), PublishError> {
        self.slots.reserve_group(s)
    }
    fn insert_id(&mut self, s: &(impl CodecRead + ?Sized), id: &str) -> Result<bool, PublishError> {
        self.slots.insert_id(s, id)
    }
    fn insert_group(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        index: i64,
        group: Group,
    ) -> Result<bool, PublishError> {
        self.group = Some((index, GroupData::read(&group)));
        self.slots.insert_group(s, index, group)
    }
}
impl OrdinaryProofs {
    pub(super) fn object(
        &mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        record: &RefValue,
        raw: &impl RawLookup,
        producer: &Cc004Producer,
        slots: &mut impl ObjectSlots,
        checked: &mut BusinessMap<(NodeDigest, bool), i64>,
        prefix: usize,
        deadline: Option<std::time::Instant>,
        budget: Option<&dyn super::super::audit_budget::Budget>,
    ) -> Result<Object, PublishError> {
        let k = key(record.view());
        let groups = bj::new_map::<i64, Group>(None);
        let shapes = bj::new_map::<bj::TextValue, Shape>(None);
        if let Some(p) = self.candidate(0, &k, raw, &groups, &shapes) {
            let Output::Object(o, g) = &p.output else {
                unreachable!()
            };
            if slots.insert_id(s, &o.id)? {
                return Err(err("对象身份重复或修订不符"));
            }
            if let Some((i, g)) = g {
                slots.reserve_group(s)?;
                if slots.insert_group(s, *i, g.restore(s)?)? {
                    return Err(err("同cut组合并序号重复"));
                }
            }
            let output = o.restore(s)?;
            self.object_records.insert(o.id.clone(), k.clone());
            self.retain(0, k, p);
            return Ok(output);
        }
        let deps = RefCell::new(Dependencies::default());
        let lookup = RawReads { raw, deps: &deps };
        let mut writes = ObjectWrites { slots, group: None };
        let output = audit_object_record_with_slots(
            s,
            record,
            &lookup,
            producer,
            &self.rule,
            0,
            &mut writes,
            checked,
            None,
            prefix,
            false,
            deadline,
            budget,
        )?;
        semantic_sources(&deps, &output.sources, raw);
        self.object_records
            .insert(output.id.as_str().into(), k.clone());
        self.save(
            0,
            k,
            deps.into_inner(),
            Output::Object(ObjectData::read(&output), writes.group),
        );
        Ok(output)
    }
    pub(super) fn shape(
        &mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        object: &Object,
        raw: &impl RawLookup,
        groups: &impl GroupLookup,
        producer: &Cc004Producer,
        checked: &mut BusinessMap<(NodeDigest, bool), i64>,
        prefix: usize,
        deadline: Option<std::time::Instant>,
        budget: Option<&dyn super::super::audit_budget::Budget>,
    ) -> Result<Shape, PublishError> {
        // object_id 不是完整记录身份；配方使用的全部字段均由记录引用绑定。
        let k = self
            .object_records
            .get(object.id.as_str())
            .expect("ordinary object was audited")
            .clone();
        let shapes = bj::new_map::<bj::TextValue, Shape>(None);
        if let Some(p) = self.candidate(1, &k, raw, groups, &shapes) {
            let Output::Shape(groups) = p.output else {
                unreachable!()
            };
            self.retain(1, k, p);
            return Ok(Shape { groups });
        }
        let deps = RefCell::new(Dependencies::default());
        let output = audit_shape_record(
            s,
            object,
            &RawReads { raw, deps: &deps },
            &GroupReads {
                groups,
                deps: &deps,
            },
            producer,
            checked,
            None,
            prefix,
            deadline,
            budget,
        )?;
        semantic_sources(&deps, &object.sources, raw);
        self.save(1, k, deps.into_inner(), Output::Shape(output.groups));
        Ok(output)
    }
    pub(super) fn witness(
        &mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        record: &RefValue,
        raw: &impl RawLookup,
        groups: &impl GroupLookup,
        shapes: &impl ShapeLookup,
        slots: &mut impl WitnessSlots,
        deadline: Option<std::time::Instant>,
    ) -> Result<(), PublishError> {
        let k = key(record.view());
        if let Some(p) = self.candidate(2, &k, raw, groups, shapes) {
            let Output::Witness(id, slot) = &p.output else {
                unreachable!()
            };
            slots.reserve(s)?;
            if slots.insert_slot(s, id, *slot)? {
                return Err(err("见证槽位重复/越界"));
            }
            self.retain(2, k, p);
            return Ok(());
        }
        let deps = RefCell::new(Dependencies::default());
        let (id, slot) = audit_witness_record_with_slots(
            s,
            record,
            &RawReads { raw, deps: &deps },
            &GroupReads {
                groups,
                deps: &deps,
            },
            &ShapeReads {
                shapes,
                deps: &deps,
            },
            slots,
            None,
            deadline,
        )?;
        self.save(
            2,
            k,
            deps.into_inner(),
            Output::Witness(id.as_str().into(), slot),
        );
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
struct StepData {
    coord: i64,
    fields: Fields,
    key: String,
    reasons: ContentRef,
    waiting_count: usize,
    insufficient: bool,
    incomplete: bool,
}
impl StepData {
    fn read(coord: i64, step: &impl source_pending::StepView) -> Self {
        Self {
            coord,
            fields: fields(step.fields(), &source_pending::STEP),
            key: step.key().as_str().into(),
            reasons: reference(step.reasons().view()),
            waiting_count: step.waiting_count(),
            insufficient: step.insufficient(),
            incomplete: step.incomplete(),
        }
    }
    fn restore<'a>(
        &self,
        s: &(impl CodecRead + ?Sized),
        object: &'a Object,
    ) -> Result<source_pending::Step<'a>, PublishError> {
        Ok(source_pending::Step {
            object,
            fields: restore_fields(&self.fields)?,
            key: s.native_text(&self.key)?,
            reasons: self.reasons.clone().into(),
            waiting_count: self.waiting_count,
            insufficient: self.insufficient,
            incomplete: self.incomplete,
        })
    }
    fn bytes(&self) -> usize {
        fields_bytes(&self.fields) + self.key.capacity() + ref_bytes(&self.reasons)
    }
}
#[derive(Clone, PartialEq, Eq)]
struct StepState {
    object: ObjectData,
    data: StepData,
}
impl StepState {
    fn read(coord: i64, step: &dyn source_pending::StepView) -> Self {
        Self {
            object: ObjectData::read(step.object()),
            data: StepData {
                coord,
                fields: fields(step.fields(), &source_pending::STEP),
                key: step.key().as_str().into(),
                reasons: reference(step.reasons().view()),
                waiting_count: step.waiting_count(),
                insufficient: step.insufficient(),
                incomplete: step.incomplete(),
            },
        }
    }
    fn matches(&self, coord: i64, step: &dyn source_pending::StepView) -> bool {
        self.object.matches(step.object())
            && self.data.coord == coord
            && fields_match(&self.data.fields, step.fields())
            && self.data.key == step.key().as_str()
            && ref_matches(&self.data.reasons, step.reasons().view())
            && self.data.waiting_count == step.waiting_count()
            && self.data.insufficient == step.insufficient()
            && self.data.incomplete == step.incomplete()
    }
    fn bytes(&self) -> usize {
        self.object.bytes() + self.data.bytes()
    }
}
#[derive(Default)]
struct RequestData {
    key: String,
    bound: Option<i64>,
    steps: Vec<(i64, Option<StepState>)>,
    subjects: Vec<(String, bool)>,
}
impl RequestData {
    fn valid(&self, steps: &impl source_pending::StepLookup) -> bool {
        self.steps
            .iter()
            .all(|(c, old)| match (old, steps.get_step(*c)) {
                (Some(old), Some(current)) => old.matches(*c, current),
                (None, None) => true,
                _ => false,
            })
            && self
                .subjects
                .iter()
                .all(|(id, present)| steps.contains_subject(id) == *present)
    }
    fn bytes(&self) -> usize {
        self.key.capacity()
            + self.steps.capacity() * std::mem::size_of::<(i64, Option<StepState>)>()
            + self
                .steps
                .iter()
                .filter_map(|(_, v)| v.as_ref())
                .map(StepState::bytes)
                .sum::<usize>()
            + self.subjects.capacity() * std::mem::size_of::<(String, bool)>()
            + self
                .subjects
                .iter()
                .map(|(s, _)| s.capacity())
                .sum::<usize>()
    }
}
struct StepReads<'a, T> {
    steps: &'a T,
    reads: &'a RefCell<RequestData>,
}
impl<T: source_pending::StepLookup> source_structure::Lookup for StepReads<'_, T> {
    fn object(&self, id: &str) -> Option<&Object> {
        self.steps.object(id)
    }
}
impl<T: source_pending::StepLookup> source_pending::StepLookup for StepReads<'_, T> {
    fn get_step(&self, coord: i64) -> Option<&dyn source_pending::StepView> {
        let value = self.steps.get_step(coord);
        self.reads
            .borrow_mut()
            .steps
            .push((coord, value.map(|s| StepState::read(coord, s))));
        value
    }
    fn contains_subject(&self, id: &str) -> bool {
        let present = self.steps.contains_subject(id);
        self.reads.borrow_mut().subjects.push((id.into(), present));
        present
    }
}
struct RequestWrites<'a, T> {
    slots: &'a mut T,
    writes: &'a RefCell<RequestData>,
}
impl<T: source_pending::RequestSlots> source_pending::RequestSlots for RequestWrites<'_, T> {
    fn insert_request(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        key: &str,
    ) -> Result<bool, PublishError> {
        self.writes.borrow_mut().key = key.into();
        self.slots.insert_request(s, key)
    }
    fn insert_bound(
        &mut self,
        s: &(impl CodecRead + ?Sized),
        coord: i64,
    ) -> Result<bool, PublishError> {
        self.writes.borrow_mut().bound = Some(coord);
        self.slots.insert_bound(s, coord)
    }
}
impl OrdinaryProofs {
    pub(super) fn step<'a>(
        &mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        object: &'a Object,
        raw: &impl RawLookup,
        equal: bool,
    ) -> Result<(i64, source_pending::Step<'a>), PublishError> {
        let k = self
            .object_records
            .get(object.id.as_str())
            .expect("ordinary object was audited")
            .clone();
        let groups = bj::new_map::<i64, Group>(None);
        let shapes = bj::new_map::<bj::TextValue, Shape>(None);
        if let Some(p) = self.candidate(3, &k, raw, &groups, &shapes) {
            let Output::Step(data) = &p.output else {
                unreachable!()
            };
            let output = (data.coord, data.restore(s, object)?);
            self.retain(3, k, p);
            return Ok(output);
        }
        let deps = RefCell::new(Dependencies::default());
        let (coord, step) =
            source_pending::step_record(s, object, &RawReads { raw, deps: &deps }, equal)?;
        self.save(
            3,
            k,
            deps.into_inner(),
            Output::Step(StepData::read(coord, &step)),
        );
        Ok((coord, step))
    }
    pub(super) fn observation(
        &mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        record: &RefValue,
    ) -> Result<bj::TextValue, PublishError> {
        let k = key(record.view());
        if let Some(proof) = self.previous.get(&(5, k.clone())).cloned() {
            let Output::Observation(kind) = &proof.output else {
                unreachable!()
            };
            let output = s.native_text(kind)?;
            self.stats.hits += 1;
            self.publish();
            self.retain(5, k, proof);
            return Ok(output);
        }
        let kind = text(s, record, "kind")?;
        self.save(
            5,
            k,
            Dependencies::default(),
            Output::Observation(kind.as_str().into()),
        );
        Ok(kind)
    }
    pub(super) fn request(
        &mut self,
        s: &mut (impl SourceFieldStore + ?Sized),
        request: &RefValue,
        steps: &impl source_pending::StepLookup,
        slots: &mut impl source_pending::RequestSlots,
        equal: bool,
    ) -> Result<(), PublishError> {
        let k = key(request.view());
        if let Some(p) = self.previous.get(&(4, k.clone())).cloned() {
            let Output::Request(data) = &p.output else {
                unreachable!()
            };
            if data.valid(steps) {
                self.stats.hits += 1;
                self.publish();
                if slots.insert_request(s, &data.key)? {
                    return Err(err("同cut知识请求重复"));
                }
                if let Some(coord) = data.bound {
                    if slots.insert_bound(s, coord)? {
                        return Err(err("CC004知识请求key/subject/来源不一致"));
                    }
                }
                self.retain(4, k, p);
                return Ok(());
            }
            self.stats.invalidations += 1;
            self.publish();
        }
        let data = RefCell::new(RequestData::default());
        source_pending::request_record_with_slots(
            s,
            request,
            &StepReads {
                steps,
                reads: &data,
            },
            &mut RequestWrites {
                slots,
                writes: &data,
            },
            equal,
            &self.rule,
        )?;
        self.save(
            4,
            k,
            Dependencies::default(),
            Output::Request(data.into_inner()),
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "s_incremental"))]
mod tests {
    use super::super::tests::{original_sources, replace_field, STATE};
    use super::*;

    fn fixture() -> (sc::MemoryNodes, ContentRef, Vec<Value>, String) {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/s4-legacy4df/source.json"),
        )
        .unwrap();
        let fixture: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(fixture["head"], "4df2d10f92d1075f5fb761a1b112c9c949da5408");
        let mut nodes = sc::MemoryNodes::new();
        let originals: Vec<(NodeDigest, u8, Vec<u8>)> =
            serde_json::from_value(fixture["nodes"].clone()).unwrap();
        for (digest, tag, body) in originals {
            nodes.insert_unchecked(digest, tag, body);
        }
        (
            nodes,
            ContentRef::from_transport_json(&fixture["root"]).unwrap(),
            serde_json::from_value(fixture["raw"].clone()).unwrap(),
            fixture["rule"].as_str().unwrap().to_string(),
        )
    }

    fn run(
        nodes: &mut sc::MemoryNodes,
        root: &ContentRef,
        raw: &[Value],
        rule: &str,
        book: &mut OrdinaryProofs,
    ) -> Result<Counts, PublishError> {
        let mut reader = super::super::super::history::HashReader::metered(nodes, None, None, None);
        audit_generated_with_proofs(
            &mut reader,
            root.into(),
            raw.into(),
            tb02a::PROFILE,
            rule,
            None,
            None,
            book,
        )
    }
    fn run_counted(
        nodes: &mut sc::MemoryNodes,
        root: &ContentRef,
        raw: &[Value],
        rule: &str,
        book: &mut OrdinaryProofs,
    ) -> (Counts, usize) {
        struct Reads<'a> {
            source: &'a mut sc::MemoryNodes,
            count: usize,
        }
        impl NodeSource for Reads<'_> {
            fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                self.count += 1;
                self.source.read(digest)
            }
            fn checked_read(
                &mut self,
                digest: &NodeDigest,
            ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
                self.count += 1;
                self.source.checked_read(digest)
            }
        }
        let mut source = Reads {
            source: nodes,
            count: 0,
        };
        let mut reader =
            super::super::super::history::HashReader::metered(&mut source, None, None, None);
        let counts = audit_generated_with_proofs(
            &mut reader,
            root.into(),
            raw.into(),
            tb02a::PROFILE,
            rule,
            None,
            None,
            book,
        )
        .unwrap();
        drop(reader);
        (counts, source.count)
    }
    fn oracle(
        nodes: &mut sc::MemoryNodes,
        root: &ContentRef,
        raw: &[Value],
        rule: &str,
    ) -> Result<Counts, PublishError> {
        let mut reader = super::super::super::history::HashReader::metered(nodes, None, None, None);
        audit_generated_frozen_for_test(&mut reader, root, raw, tb02a::PROFILE, rule, None, None)
    }
    #[test]
    fn ordinary_proof_success_reuse_oracle_is_independent_and_two_frontiers_release() {
        let (mut nodes, root, raw, rule) = fixture();
        reset_stats();
        let expected = oracle(&mut nodes, &root, &raw, &rule).unwrap();
        assert_eq!((stats().hits, stats().executions), (0, 0));
        #[cfg(feature = "s_incremental")]
        let heap = super::super::super::audit_owner::allocation_probe::Window::start();
        let live_before = super::super::super::audit_owner::allocation_probe::live_bytes();
        let mut book = OrdinaryProofs::default();
        let (actual, first_reads) = run_counted(&mut nodes, &root, &raw, &rule, &mut book);
        assert_eq!(actual, expected);
        assert!(book.current.keys().any(|(phase, _)| *phase == 3));
        assert!(book.current.keys().any(|(phase, _)| *phase == 4));
        book.commit();
        let one_frontier =
            super::super::super::audit_owner::allocation_probe::live_bytes() - live_before;
        let executions = book.stats.executions;
        let (actual, reused_reads) = run_counted(&mut nodes, &root, &raw, &rule, &mut book);
        assert_eq!(actual, expected);
        assert!(reused_reads < first_reads);
        let two_frontiers =
            super::super::super::audit_owner::allocation_probe::live_bytes() - live_before;
        assert!(book.stats.hits > 0);
        assert!(one_frontier > 0 && two_frontiers > one_frontier);
        assert_eq!(book.stats.executions, executions);
        book.commit();
        let old_entries = book.previous.len();
        assert!(book.current.is_empty());
        book.begin(tb02a::PROFILE, &rule);
        book.commit();
        assert_eq!(book.previous.len(), 0);
        assert!(book.stats.released_bytes > 0);
        let capacity = book.stats;
        drop(book);
        assert_eq!(stats().retained_bytes, 0);
        #[cfg(feature = "s_incremental")]
        {
            let observed = heap.finish();
            eprintln!("ORDINARY_PROOF_CAPACITY entries_before={old_entries} actual_one_frontier={one_frontier} actual_two_frontiers={two_frontiers} {capacity:?}");
            eprintln!("ORDINARY_PROOF_NODE_READS first={first_reads} reused={reused_reads}");
            eprintln!("ORDINARY_PROOF_HEAP {observed}");
            assert_eq!(observed["live_before"], observed["live_after"]);
        }
    }
    #[test]
    fn ordinary_proof_raw_wire_bar_and_original_changes_match_frozen_first_error() {
        let (mut nodes, root, raw, rule) = fixture();
        for (field, value) in [
            ("revision", "2"),
            ("high", "999999"),
            ("raw_text", "modified original"),
        ] {
            let mut book = OrdinaryProofs::default();
            run(&mut nodes, &root, &raw, &rule, &mut book).unwrap();
            book.commit();
            let mut changed = raw.clone();
            changed[0][field] = if field == "revision" {
                json!(raw[0][field].as_i64().unwrap() + 1)
            } else {
                json!(value)
            };
            let expected = oracle(&mut nodes, &root, &changed, &rule).unwrap_err();
            let got = run(&mut nodes, &root, &changed, &rule, &mut book).unwrap_err();
            assert_eq!(got, expected, "{field}");
            assert!(book.stats.invalidations > 0, "{field}");
            let mut cold = OrdinaryProofs::default();
            assert_eq!(
                run(&mut nodes, &root, &changed, &rule, &mut cold).unwrap_err(),
                expected
            );
        }
    }
    #[test]
    fn ordinary_proof_duplicate_global_effect_precedes_later_malformed_record() {
        let (mut nodes, root, raw, rule) = fixture();
        for (collection, expected_text) in
            [("objects", "对象身份重复"), ("witnesses", "见证槽位重复")]
        {
            let array = field(&mut nodes, &root, collection).unwrap();
            let values = elements(&mut nodes, &array).unwrap();
            let malformed = sc::write_value(&mut nodes, &V::Object(vec![])).unwrap();
            let bad = replace_field(
                &mut nodes,
                &root,
                STATE,
                collection,
                V::Array(vec![
                    V::Edge(values[0].digest().unwrap()),
                    V::Edge(values[0].digest().unwrap()),
                    V::Edge(malformed.digest),
                ]),
            );
            let expected = oracle(&mut nodes, &bad, &raw, &rule).unwrap_err();
            assert!(expected.to_string().contains(expected_text), "{expected}");
            let mut book = OrdinaryProofs::default();
            run(&mut nodes, &root, &raw, &rule, &mut book).unwrap();
            book.commit();
            let hits = book.stats.hits;
            assert_eq!(
                run(&mut nodes, &bad, &raw, &rule, &mut book).unwrap_err(),
                expected
            );
            assert!(book.stats.hits > hits);
            let mut cold = OrdinaryProofs::default();
            assert_eq!(
                run(&mut nodes, &bad, &raw, &rule, &mut cold).unwrap_err(),
                expected
            );
            // 第三路：保留完整前缀，再附晚处坏记录；已有成功前沿的早期原件依赖失效。
            let mut late_bad_records: Vec<V> = values
                .iter()
                .map(|v| V::Edge(v.digest().unwrap()))
                .collect();
            late_bad_records.push(V::Edge(malformed.digest));
            let invalidated_bad = replace_field(
                &mut nodes,
                &root,
                STATE,
                collection,
                V::Array(late_bad_records),
            );
            let mut invalidated = OrdinaryProofs::default();
            run(&mut nodes, &root, &raw, &rule, &mut invalidated).unwrap();
            invalidated.commit();
            let mut changed = raw.clone();
            changed[0]["revision"] = json!(raw[0]["revision"].as_i64().unwrap() + 1);
            let early_error = oracle(&mut nodes, &root, &changed, &rule).unwrap_err();
            let dual_error = oracle(&mut nodes, &invalidated_bad, &changed, &rule).unwrap_err();
            assert_eq!(
                dual_error, early_error,
                "{collection}: 晚处坏记录不得盖过早期来源错"
            );
            assert_eq!(
                run(
                    &mut nodes,
                    &invalidated_bad,
                    &changed,
                    &rule,
                    &mut invalidated
                )
                .unwrap_err(),
                dual_error
            );
            assert!(invalidated.stats.invalidations > 0);
            let mut cold = OrdinaryProofs::default();
            assert_eq!(
                run(&mut nodes, &invalidated_bad, &changed, &rule, &mut cold).unwrap_err(),
                dual_error
            );
        }
    }
    #[test]
    fn ordinary_proof_checked_refs_hit_registers_coord_after_preceding_object_withdraws() {
        let (mut nodes, root, events, rule) = fixture();
        let mut raw = original_sources(&events, events.len()).unwrap();
        let array = field(&mut nodes, &root, "objects").unwrap();
        let records = elements(&mut nodes, &array).unwrap();
        let producer = Cc004Producer::new(tb02a::PROFILE, &rule);
        let mut book = OrdinaryProofs::default();
        book.begin(tb02a::PROFILE, &rule);
        let mut ids = bj::new_map(None);
        let mut groups = bj::new_map(None);
        let mut checked = bj::new_map(None);
        let mut selected = None;
        for record in &records {
            let before = checked.len();
            let object = book
                .object(
                    &mut nodes,
                    record,
                    &raw,
                    &producer,
                    &mut FullObjectSlots {
                        ids: &mut ids,
                        groups: &mut groups,
                    },
                    &mut checked,
                    events.len(),
                    None,
                    None,
                )
                .unwrap();
            if before > 0
                && checked.len() == before
                && object.kind != CC006_SHAPE_KIND
                && !object.sources.is_empty()
            {
                selected = Some((reference(record.view()), object.sources[0]));
                break;
            }
        }
        let (record, coord) = selected.expect("真实B对象复用A已登记的引用");
        let k = key((&record).into());
        assert!(book.current[&(0, k)]
            .deps
            .raw
            .iter()
            .any(|(c, value)| *c == coord && value.is_some()));
        book.commit();
        let mut changed = *raw.get(&coord).unwrap();
        changed.wire[0] ^= 1;
        raw.insert(coord, changed).unwrap();
        let record = RefValue::from(record);
        let old = audit_object_record(
            &mut nodes,
            &record,
            &raw,
            &producer,
            &rule,
            0,
            &mut bj::new_map(None),
            &mut bj::new_map(None),
            &mut bj::new_map(None),
            None,
            events.len(),
            false,
            None,
            None,
        )
        .err()
        .unwrap();
        book.begin(tb02a::PROFILE, &rule);
        let new = book
            .object(
                &mut nodes,
                &record,
                &raw,
                &producer,
                &mut FullObjectSlots {
                    ids: &mut bj::new_map(None),
                    groups: &mut bj::new_map(None),
                },
                &mut bj::new_map(None),
                events.len(),
                None,
                None,
            )
            .err()
            .unwrap();
        assert_eq!(new, old);
        assert!(new.to_string().contains("真实完整八字段原件"));
        assert_eq!(book.stats.invalidations, 1);
    }
    #[test]
    fn ordinary_proof_group_dependency_changes_invalidate_same_shape_root() {
        let (mut nodes, root, events, rule) = fixture();
        let raw = original_sources(&events, events.len()).unwrap();
        let mut book = OrdinaryProofs::default();
        run(&mut nodes, &root, &events, &rule, &mut book).unwrap();
        book.commit();
        let shape_key = book
            .previous
            .iter()
            .find_map(|((phase, k), _)| (*phase == 1).then(|| k.clone()))
            .unwrap();
        let Output::Object(object, _) = &book.previous[&(0, shape_key.clone())].output else {
            panic!()
        };
        let object = object.restore(&nodes).unwrap();
        let mut groups = bj::new_map(None);
        for proof in book.previous.values() {
            if let Output::Object(_, Some((index, g))) = &proof.output {
                groups.insert(*index, g.restore(&nodes).unwrap()).unwrap();
            }
        }
        let Output::Shape(indices) = book.previous[&(1, shape_key)].output else {
            panic!()
        };
        let group = groups.get_mut(&indices[0]).unwrap();
        group.anchor += 1;
        let producer = Cc004Producer::new(tb02a::PROFILE, &rule);
        let old = audit_shape_record(
            &mut nodes,
            &object,
            &raw,
            &groups,
            &producer,
            &mut bj::new_map(None),
            None,
            events.len(),
            None,
            None,
        )
        .err()
        .unwrap();
        book.current.clear();
        let new = book
            .shape(
                &mut nodes,
                &object,
                &raw,
                &groups,
                &producer,
                &mut bj::new_map(None),
                events.len(),
                None,
                None,
            )
            .err()
            .unwrap();
        assert_eq!(new, old);
        assert!(new.to_string().contains("形态窗口"));
        assert!(book.stats.invalidations > 0);
    }
    #[test]
    fn ordinary_proof_request_negative_subject_dependency_cannot_be_reused() {
        struct Steps(bool);
        impl source_structure::Lookup for Steps {
            fn object(&self, _: &str) -> Option<&Object> {
                None
            }
        }
        impl source_pending::StepLookup for Steps {
            fn get_step(&self, _: i64) -> Option<&dyn source_pending::StepView> {
                None
            }
            fn contains_subject(&self, id: &str) -> bool {
                self.0 && id == "subject"
            }
        }
        #[derive(Default)]
        struct Slots;
        impl source_pending::RequestSlots for Slots {
            fn insert_request(
                &mut self,
                _: &(impl CodecRead + ?Sized),
                _: &str,
            ) -> Result<bool, PublishError> {
                Ok(false)
            }
            fn insert_bound(
                &mut self,
                _: &(impl CodecRead + ?Sized),
                _: i64,
            ) -> Result<bool, PublishError> {
                Ok(false)
            }
        }
        let mut nodes = sc::MemoryNodes::default();
        let request: RefValue = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("request_id".into(), V::String("[\"other\",\"0\"]".into())),
                ("subject_id".into(), V::String("subject".into())),
                ("axes".into(), V::Null),
                ("reasons".into(), V::Null),
                ("source_coords".into(), V::Null),
            ]),
        )
        .unwrap()
        .content_ref()
        .into();
        let mut book = OrdinaryProofs::default();
        book.request(&mut nodes, &request, &Steps(false), &mut Slots, true)
            .unwrap();
        book.commit();
        let expected = source_pending::request_record_with_slots(
            &mut nodes,
            &request,
            &Steps(true),
            &mut Slots,
            true,
            "",
        )
        .unwrap_err();
        assert_eq!(
            book.request(&mut nodes, &request, &Steps(true), &mut Slots, true)
                .unwrap_err(),
            expected
        );
        assert!(expected.to_string().contains("知识主体被绑定"));
        assert_eq!(book.stats.invalidations, 1);
    }
    #[test]
    fn ordinary_proof_step_bar_and_witness_shape_dependencies_match_old_local_kernel() {
        let (mut nodes, root, events, rule) = fixture();
        let mut raw = original_sources(&events, events.len()).unwrap();
        let mut book = OrdinaryProofs::default();
        run(&mut nodes, &root, &events, &rule, &mut book).unwrap();
        book.commit();
        let (step_key, coord) = book
            .previous
            .iter()
            .find_map(|((phase, key), proof)| {
                if *phase != 3 {
                    return None;
                }
                let Output::Step(step) = &proof.output else {
                    unreachable!()
                };
                Some((key.clone(), step.coord))
            })
            .unwrap();
        let Output::Object(object, _) = &book.previous[&(0, step_key)].output else {
            unreachable!()
        };
        let object = object.restore(&nodes).unwrap();
        let mut changed = *raw.get(&coord).unwrap();
        changed.bar[0] ^= 1;
        raw.insert(coord, changed).unwrap();
        let expected = source_pending::step_record(&mut nodes, &object, &raw, true)
            .err()
            .unwrap();
        assert_eq!(
            book.step(&mut nodes, &object, &raw, true).err().unwrap(),
            expected
        );
        assert!(expected.to_string().contains("同cut原始来源"));
        let raw = original_sources(&events, events.len()).unwrap();
        let (witness_key, id, slot) = book
            .previous
            .iter()
            .find_map(|((phase, key), proof)| {
                if *phase != 2 {
                    return None;
                }
                let Output::Witness(id, slot) = &proof.output else {
                    unreachable!()
                };
                Some((key.clone(), id.clone(), *slot))
            })
            .unwrap();
        let mut groups = bj::new_map(None);
        let mut shapes = bj::new_map(None);
        for ((phase, key), proof) in &book.previous {
            if let Output::Object(_, Some((index, group))) = &proof.output {
                groups
                    .insert(*index, group.restore(&nodes).unwrap())
                    .unwrap();
            }
            if *phase == 1 {
                let Output::Shape(indices) = proof.output else {
                    unreachable!()
                };
                let Output::Object(object, _) = &book.previous[&(0, key.clone())].output else {
                    unreachable!()
                };
                shapes
                    .insert(
                        nodes.native_text(&object.id).unwrap(),
                        Shape { groups: indices },
                    )
                    .unwrap();
            }
        }
        let shape = shapes.get_mut(id.as_str()).unwrap();
        shape.groups[slot as usize] = (shape.groups[slot as usize] + 1) % groups.len() as i64;
        let witness: RefValue = ContentRef {
            format: witness_key.0,
            root: witness_key.1,
            kind: Kind::Object,
            extent: witness_key.3,
        }
        .into();
        let expected = audit_witness_record(
            &mut nodes,
            &witness,
            &raw,
            &groups,
            &shapes,
            &mut bj::new_map(None),
            None,
            None,
        )
        .unwrap_err();
        let invalidations = book.stats.invalidations;
        assert_eq!(
            book.witness(
                &mut nodes,
                &witness,
                &raw,
                &groups,
                &shapes,
                &mut FullWitnessSlots(&mut bj::new_map(None)),
                None
            )
            .unwrap_err(),
            expected
        );
        assert!(book.stats.invalidations > invalidations);
    }

    #[test]
    fn ordinary_proof_root_declaration_errors_match_independent_oracle() {
        let (mut nodes, root, raw, rule) = fixture();
        for index in 0..3 {
            let mut book = OrdinaryProofs::default();
            run(&mut nodes, &root, &raw, &rule, &mut book).unwrap();
            book.commit();
            let mut altered = root.clone();
            match index {
                0 => altered.kind = Kind::Array,
                1 => altered.extent = Some(root.extent.unwrap() + 1),
                _ => altered.format = "wrong-format".into(),
            }
            let expected = oracle(&mut nodes, &altered, &raw, &rule).unwrap_err();
            assert_eq!(
                run(&mut nodes, &altered, &raw, &rule, &mut book).unwrap_err(),
                expected
            );
        }
    }

    #[test]
    fn ordinary_proof_complete_ref_and_profile_rule_are_not_digest_only_keys() {
        let (mut nodes, root, raw, rule) = fixture();
        let mut book = OrdinaryProofs::default();
        run(&mut nodes, &root, &raw, &rule, &mut book).unwrap();
        book.commit();
        let ((phase, original), _) = book.previous.iter().find(|((p, _), _)| *p == 0).unwrap();
        let phase = *phase;
        for field in 0..3 {
            let mut k = original.clone();
            match field {
                0 => k.0.push_str("-wrong"),
                1 => k.2 = "JsonText",
                _ => k.3 = Some(999),
            }
            assert!(!book.previous.contains_key(&(phase, k)));
        }
        assert!(!book.previous.contains_key(&(9, original.clone())));
        book.begin("different-profile", &rule);
        assert!(book.previous.is_empty());
        book.begin(tb02a::PROFILE, "different-rule");
        assert!(book.previous.is_empty());
    }
}
