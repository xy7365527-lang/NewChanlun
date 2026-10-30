//! #1640 ordinary 单 open 成功配方。集合效应仍在当前 cut 原位执行。
use super::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

type Key = (String, String, &'static str, Option<u64>);
fn key(r: sc::RefView<'_>) -> Key {
    (r.format().into(), r.root().into(), r.kind().name(), r.extent())
}
fn reference(r: sc::RefView<'_>) -> ContentRef {
    ContentRef { format: r.format().into(), root: r.root().into(), kind: r.kind(), extent: r.extent() }
}
fn ref_bytes(r: &ContentRef) -> usize { r.format.capacity() + r.root.capacity() }
type Fields = Vec<(&'static str, ContentRef)>;
fn fields(f: &SourceFields<'static>, names: &[&'static str]) -> Fields {
    names.iter().filter_map(|&n| f.0.get(n).map(|r| (n, reference(r)))).collect()
}
fn restore_fields(f: &Fields) -> Result<SourceFields<'static>, String> {
    NativeRefFields::try_from_views(f.iter().map(|(n, r)| (*n, r.into())), None)
        .map(|f| SourceFields(super::super::audit_store::RefFields::Plain(f))).map_err(err)
}
fn fields_bytes(f: &Fields) -> usize {
    f.capacity() * std::mem::size_of::<(&str, ContentRef)>() + f.iter().map(|(_, r)| ref_bytes(r)).sum::<usize>()
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ObjectData {
    id: String, kind: String, fields: Fields, refs: ContentRef, sources: Vec<i64>,
}
impl ObjectData {
    pub(super) fn read(o: &Object) -> Self {
        let names: &[&'static str] = if o.kind == CC006_SHAPE_KIND { &SHAPE_SOURCE_FIELDS } else { &TYPED_SOURCE_FIELDS };
        Self { id: o.id.as_str().into(), kind: o.kind.as_str().into(), fields: fields(&o.fields, names),
            refs: reference(o.refs.view()), sources: o.sources.to_vec() }
    }
    fn restore(&self, s: &(impl CodecRead + ?Sized)) -> Result<Object, String> {
        Ok(Object { id: s.native_text(&self.id)?, kind: s.native_text(&self.kind)?,
            fields: restore_fields(&self.fields)?, refs: self.refs.clone().into(),
            sources: SourceCoords::Original(copy_coords(s, &self.sources)?) })
    }
    fn bytes(&self) -> usize {
        self.id.capacity() + self.kind.capacity() + fields_bytes(&self.fields)
            + ref_bytes(&self.refs) + self.sources.capacity() * 8
    }
}
#[derive(Clone, PartialEq, Eq)]
struct GroupData { payload: ContentRef, members: Vec<i64>, sources: Vec<i64>, anchor: i64 }
impl GroupData {
    fn read(g: &Group) -> Self {
        Self { payload: reference(g.payload.view()), members: g.members.to_vec(), sources: g.sources.to_vec(), anchor: g.anchor }
    }
    fn matches(&self, g: &Group) -> bool {
        self.payload == reference(g.payload.view()) && self.members[..] == *g.members
            && self.sources[..] == *g.sources && self.anchor == g.anchor
    }
    fn restore(&self, s: &(impl CodecRead + ?Sized)) -> Result<Group, String> {
        Ok(Group { payload: self.payload.clone().into(), members: SourceCoords::Original(copy_coords(s, &self.members)?),
            sources: SourceCoords::Original(copy_coords(s, &self.sources)?), anchor: self.anchor })
    }
    fn bytes(&self) -> usize { ref_bytes(&self.payload) + (self.members.capacity() + self.sources.capacity()) * 8 }
}
#[derive(Default)]
struct Dependencies {
    raw: Vec<(i64, Option<Source>)>,
    groups: Vec<(i64, Option<GroupData>)>,
    shapes: Vec<(String, Option<[i64; 3]>)>,
}
impl Dependencies {
    fn valid(&self, raw: &impl RawLookup, groups: &impl GroupLookup, shapes: &impl ShapeLookup) -> bool {
        self.raw.iter().all(|(c, v)| raw.get(c).copied() == *v)
            && self.groups.iter().all(|(i, v)| match (v, groups.get(i)) {
                (Some(a), Some(b)) => a.matches(b), (None, None) => true, _ => false,
            })
            && self.shapes.iter().all(|(id, v)| shapes.get(id).map(|v| v.groups) == *v)
    }
    fn bytes(&self) -> usize {
        self.raw.capacity() * std::mem::size_of::<(i64, Option<Source>)>()
            + self.groups.capacity() * std::mem::size_of::<(i64, Option<GroupData>)>()
            + self.groups.iter().filter_map(|(_, g)| g.as_ref()).map(GroupData::bytes).sum::<usize>()
            + self.shapes.capacity() * std::mem::size_of::<(String, Option<[i64; 3]>)>()
            + self.shapes.iter().map(|(s, _)| s.capacity()).sum::<usize>()
    }
}
struct RawReads<'a, T: ?Sized> { raw: &'a T, deps: &'a RefCell<Dependencies> }
impl<T: RawLookup + ?Sized> RawLookup for RawReads<'_, T> {
    fn get(&self, c: &i64) -> Option<&Source> {
        let value = self.raw.get(c);
        self.deps.borrow_mut().raw.push((*c, value.copied()));
        value
    }
}
struct GroupReads<'a, T: ?Sized> { groups: &'a T, deps: &'a RefCell<Dependencies> }
impl<T: GroupLookup + ?Sized> GroupLookup for GroupReads<'_, T> {
    fn get(&self, i: &i64) -> Option<&Group> {
        let value = self.groups.get(i);
        self.deps.borrow_mut().groups.push((*i, value.map(GroupData::read)));
        value
    }
    fn len(&self) -> usize { self.groups.len() }
}
struct ShapeReads<'a, T: ?Sized> { shapes: &'a T, deps: &'a RefCell<Dependencies> }
impl<T: ShapeLookup + ?Sized> ShapeLookup for ShapeReads<'_, T> {
    fn get(&self, id: &str) -> Option<&Shape> {
        let value = self.shapes.get(id);
        self.deps.borrow_mut().shapes.push((id.into(), value.map(|s| s.groups)));
        value
    }
}
// checked_refs 可省去 RawLookup。成功结果的语义坐标仍必须逐项登记四摘要及存在性。
fn semantic_sources(deps: &RefCell<Dependencies>, sources: &[i64], raw: &impl RawLookup) {
    deps.borrow_mut().raw.extend(sources.iter().map(|c| (*c, raw.get(c).copied())));
}
enum Output {
    Object(ObjectData, Option<(i64, GroupData)>),
    Shape([i64; 3]),
    Witness(String, i64),
    Step(StepData),
    Request(RequestData),
}
impl Output {
    fn bytes(&self) -> usize { match self {
        Self::Object(o, g) => o.bytes() + g.as_ref().map_or(0, |(_, g)| g.bytes()),
        Self::Shape(_) => 0, Self::Witness(s, _) => s.capacity(),
        Self::Step(s) => s.bytes(), Self::Request(r) => r.bytes(),
    } }
}
struct Proof { deps: Dependencies, output: Output }
type ProofKey = (u8, Key);
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct ProofStats {
    pub hits: usize, pub executions: usize, pub invalidations: usize,
    pub retained_bytes: usize, pub peak_bytes: usize, pub released_bytes: usize,
    pub entries: usize, pub map_capacity: usize,
}
#[cfg(test)]
thread_local! { static STATS: std::cell::Cell<ProofStats> = const { std::cell::Cell::new(ProofStats {
    hits: 0, executions: 0, invalidations: 0, retained_bytes: 0, peak_bytes: 0,
    released_bytes: 0, entries: 0, map_capacity: 0,
}) }; }
#[cfg(test)]
pub(super) fn stats() -> ProofStats { STATS.with(std::cell::Cell::get) }
#[cfg(test)]
pub(super) fn reset_stats() { STATS.with(|s| s.set(ProofStats::default())); }

#[derive(Default)]
pub(super) struct OrdinaryProofs {
    previous: HashMap<ProofKey, Rc<Proof>>,
    current: HashMap<ProofKey, Rc<Proof>>,
    profile: String, rule: String,
    object_records: HashMap<String, Key>,
    stats: ProofStats,
}
impl OrdinaryProofs {
    pub(super) fn begin(&mut self, profile: &str, rule: &str) {
        if self.profile != profile || self.rule != rule {
            self.previous.clear();
            self.profile = profile.into(); self.rule = rule.into();
        }
        self.current.clear();
        self.object_records.clear();
    }
    pub(super) fn commit(&mut self) {
        self.measure();
        let before = self.stats.retained_bytes;
        self.previous = std::mem::take(&mut self.current);
        self.measure();
        self.stats.released_bytes += before.saturating_sub(self.stats.retained_bytes);
        self.publish();
    }
    fn publish(&self) { #[cfg(test)] STATS.with(|s| s.set(self.stats)); }
    fn measure(&mut self) {
        let mut unique = std::collections::HashSet::new();
        let mut bytes = self.profile.capacity() + self.rule.capacity()
            + self.object_records.capacity() * (std::mem::size_of::<(String, Key)>() + 1)
            + self.object_records.iter().map(|(id, k)| id.capacity() + k.0.capacity() + k.1.capacity()).sum::<usize>();
        for map in [&self.previous, &self.current] {
            bytes += map.capacity() * (std::mem::size_of::<(ProofKey, Rc<Proof>)>() + 1);
            for ((_, k), p) in map {
                bytes += k.0.capacity() + k.1.capacity();
                if unique.insert(Rc::as_ptr(p)) {
                    bytes += std::mem::size_of::<Proof>() + 2 * std::mem::size_of::<usize>() + p.deps.bytes() + p.output.bytes();
                }
            }
        }
        self.stats.retained_bytes = bytes;
        self.stats.peak_bytes = self.stats.peak_bytes.max(bytes);
        self.stats.entries = unique.len();
        self.stats.map_capacity = self.previous.capacity() + self.current.capacity();
        self.publish();
    }
    fn candidate(&mut self, phase: u8, k: &Key, raw: &impl RawLookup, groups: &impl GroupLookup, shapes: &impl ShapeLookup) -> Option<Rc<Proof>> {
        let p = self.previous.get(&(phase, k.clone()))?;
        if p.deps.valid(raw, groups, shapes) {
            self.stats.hits += 1; self.publish(); Some(Rc::clone(p))
        } else {
            self.stats.invalidations += 1; self.publish(); None
        }
    }
    fn save(&mut self, phase: u8, k: Key, deps: Dependencies, output: Output) {
        self.stats.executions += 1;
        self.current.insert((phase, k), Rc::new(Proof { deps, output }));
        self.publish();
    }
    fn retain(&mut self, phase: u8, k: Key, p: Rc<Proof>) { self.current.insert((phase, k), p); }
}
impl Drop for OrdinaryProofs {
    fn drop(&mut self) {
        self.measure();
        self.stats.released_bytes += self.stats.retained_bytes;
        self.stats.retained_bytes = 0; self.stats.entries = 0; self.stats.map_capacity = 0;
        self.publish();
    }
}
struct ObjectWrites<'a, T> { slots: &'a mut T, group: Option<(i64, GroupData)> }
impl<T: ObjectSlots> ObjectSlots for ObjectWrites<'_, T> {
    fn reserve_group(&mut self, s: &(impl CodecRead + ?Sized)) -> Result<(), String> { self.slots.reserve_group(s) }
    fn insert_id(&mut self, s: &(impl CodecRead + ?Sized), id: &str) -> Result<bool, String> { self.slots.insert_id(s, id) }
    fn insert_group(&mut self, s: &(impl CodecRead + ?Sized), index: i64, group: Group) -> Result<bool, String> {
        self.group = Some((index, GroupData::read(&group)));
        self.slots.insert_group(s, index, group)
    }
}
impl OrdinaryProofs {
    pub(super) fn object(
        &mut self, s: &mut (impl SourceFieldStore + ?Sized), record: &RefValue,
        raw: &impl RawLookup, producer: &Cc004Producer, slots: &mut impl ObjectSlots,
        checked: &mut BusinessMap<(NodeDigest, bool), i64>, prefix: usize,
        deadline: Option<std::time::Instant>, budget: Option<&dyn super::super::audit_budget::Budget>,
    ) -> Result<Object, String> {
        let k = key(record.view());
        let groups = bj::new_map::<i64, Group>(None);
        let shapes = bj::new_map::<bj::TextValue, Shape>(None);
        if let Some(p) = self.candidate(0, &k, raw, &groups, &shapes) {
            let Output::Object(o, g) = &p.output else { unreachable!() };
            if slots.insert_id(s, &o.id)? { return Err(err("对象身份重复或修订不符")); }
            if let Some((i, g)) = g {
                slots.reserve_group(s)?;
                if slots.insert_group(s, *i, g.restore(s)?)? { return Err(err("同cut组合并序号重复")); }
            }
            let output = o.restore(s)?;
            self.object_records.insert(o.id.clone(), k.clone());
            self.retain(0, k, p);
            return Ok(output);
        }
        let deps = RefCell::new(Dependencies::default());
        let lookup = RawReads { raw, deps: &deps };
        let mut writes = ObjectWrites { slots, group: None };
        let output = audit_object_record_with_slots(s, record, &lookup, producer, &mut writes, checked,
            None, prefix, false, deadline, budget)?;
        semantic_sources(&deps, &output.sources, raw);
        self.object_records.insert(output.id.as_str().into(), k.clone());
        self.save(0, k, deps.into_inner(), Output::Object(ObjectData::read(&output), writes.group));
        Ok(output)
    }
    pub(super) fn shape(
        &mut self, s: &mut (impl SourceFieldStore + ?Sized), object: &Object,
        raw: &impl RawLookup, groups: &impl GroupLookup, producer: &Cc004Producer,
        checked: &mut BusinessMap<(NodeDigest, bool), i64>, prefix: usize,
        deadline: Option<std::time::Instant>, budget: Option<&dyn super::super::audit_budget::Budget>,
    ) -> Result<Shape, String> {
        // object_id alone is not the record key. All fields used by the recipe are included.
        let k = self.object_records.get(object.id.as_str()).expect("ordinary object was audited").clone();
        let shapes = bj::new_map::<bj::TextValue, Shape>(None);
        if let Some(p) = self.candidate(1, &k, raw, groups, &shapes) {
            let Output::Shape(groups) = p.output else { unreachable!() };
            self.retain(1, k, p); return Ok(Shape { groups });
        }
        let deps = RefCell::new(Dependencies::default());
        let output = audit_shape_record(s, object, &RawReads { raw, deps: &deps },
            &GroupReads { groups, deps: &deps }, producer, checked, None, prefix, deadline, budget)?;
        semantic_sources(&deps, &object.sources, raw);
        self.save(1, k, deps.into_inner(), Output::Shape(output.groups));
        Ok(output)
    }
    pub(super) fn witness(
        &mut self, s: &mut (impl SourceFieldStore + ?Sized), record: &RefValue,
        raw: &impl RawLookup, groups: &impl GroupLookup, shapes: &impl ShapeLookup,
        slots: &mut impl WitnessSlots, deadline: Option<std::time::Instant>,
    ) -> Result<(), String> {
        let k = key(record.view());
        if let Some(p) = self.candidate(2, &k, raw, groups, shapes) {
            let Output::Witness(id, slot) = &p.output else { unreachable!() };
            slots.reserve(s)?;
            if slots.insert_slot(s, id, *slot)? { return Err(err("见证槽位重复/越界")); }
            self.retain(2, k, p); return Ok(());
        }
        let deps = RefCell::new(Dependencies::default());
        let (id, slot) = audit_witness_record_with_slots(s, record, &RawReads { raw, deps: &deps },
            &GroupReads { groups, deps: &deps }, &ShapeReads { shapes, deps: &deps }, slots, None, deadline)?;
        self.save(2, k, deps.into_inner(), Output::Witness(id.as_str().into(), slot));
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
struct StepData {
    coord: i64, fields: Fields, key: String, reasons: ContentRef,
    waiting_count: usize, insufficient: bool, incomplete: bool,
}
impl StepData {
    fn read(coord: i64, step: &impl source_pending::StepView) -> Self {
        Self { coord, fields: fields(step.fields(), &source_pending::STEP), key: step.key().as_str().into(),
            reasons: reference(step.reasons().view()), waiting_count: step.waiting_count(),
            insufficient: step.insufficient(), incomplete: step.incomplete() }
    }
    fn restore<'a>(&self, s: &(impl CodecRead + ?Sized), object: &'a Object) -> Result<source_pending::Step<'a>, String> {
        Ok(source_pending::Step { object, fields: restore_fields(&self.fields)?, key: s.native_text(&self.key)?,
            reasons: self.reasons.clone().into(), waiting_count: self.waiting_count,
            insufficient: self.insufficient, incomplete: self.incomplete })
    }
    fn bytes(&self) -> usize { fields_bytes(&self.fields) + self.key.capacity() + ref_bytes(&self.reasons) }
}
#[derive(Clone, PartialEq, Eq)]
struct StepState { object: ObjectData, data: StepData }
impl StepState {
    fn read(coord: i64, step: &dyn source_pending::StepView) -> Self {
        Self { object: ObjectData::read(step.object()), data: StepData {
            coord, fields: fields(step.fields(), &source_pending::STEP), key: step.key().as_str().into(),
            reasons: reference(step.reasons().view()), waiting_count: step.waiting_count(),
            insufficient: step.insufficient(), incomplete: step.incomplete(),
        } }
    }
    fn bytes(&self) -> usize { self.object.bytes() + self.data.bytes() }
}
#[derive(Default)]
struct RequestData {
    key: String, bound: Option<i64>,
    steps: Vec<(i64, Option<StepState>)>, subjects: Vec<(String, bool)>,
}
impl RequestData {
    fn valid(&self, steps: &impl source_pending::StepLookup) -> bool {
        self.steps.iter().all(|(c, old)| steps.get_step(*c).map(|s| StepState::read(*c, s)) == *old)
            && self.subjects.iter().all(|(id, present)| steps.contains_subject(id) == *present)
    }
    fn bytes(&self) -> usize {
        self.key.capacity() + self.steps.capacity() * std::mem::size_of::<(i64, Option<StepState>)>()
            + self.steps.iter().filter_map(|(_, v)| v.as_ref()).map(StepState::bytes).sum::<usize>()
            + self.subjects.capacity() * std::mem::size_of::<(String, bool)>()
            + self.subjects.iter().map(|(s, _)| s.capacity()).sum::<usize>()
    }
}
struct StepReads<'a, T> { steps: &'a T, reads: &'a RefCell<RequestData> }
impl<T: source_pending::StepLookup> source_pending::StepLookup for StepReads<'_, T> {
    fn get_step(&self, coord: i64) -> Option<&dyn source_pending::StepView> {
        let value = self.steps.get_step(coord);
        self.reads.borrow_mut().steps.push((coord, value.map(|s| StepState::read(coord, s))));
        value
    }
    fn contains_subject(&self, id: &str) -> bool {
        let present = self.steps.contains_subject(id);
        self.reads.borrow_mut().subjects.push((id.into(), present));
        present
    }
}
struct RequestWrites<'a, T> { slots: &'a mut T, writes: &'a RefCell<RequestData> }
impl<T: source_pending::RequestSlots> source_pending::RequestSlots for RequestWrites<'_, T> {
    fn insert_request(&mut self, s: &(impl CodecRead + ?Sized), key: &str) -> Result<bool, String> {
        self.writes.borrow_mut().key = key.into();
        self.slots.insert_request(s, key)
    }
    fn insert_bound(&mut self, s: &(impl CodecRead + ?Sized), coord: i64) -> Result<bool, String> {
        self.writes.borrow_mut().bound = Some(coord);
        self.slots.insert_bound(s, coord)
    }
}
impl OrdinaryProofs {
    pub(super) fn step<'a>(
        &mut self, s: &mut (impl SourceFieldStore + ?Sized), object: &'a Object,
        raw: &impl RawLookup, equal: bool,
    ) -> Result<(i64, source_pending::Step<'a>), String> {
        let k = self.object_records.get(object.id.as_str()).expect("ordinary object was audited").clone();
        let groups = bj::new_map::<i64, Group>(None);
        let shapes = bj::new_map::<bj::TextValue, Shape>(None);
        if let Some(p) = self.candidate(3, &k, raw, &groups, &shapes) {
            let Output::Step(data) = &p.output else { unreachable!() };
            let output = (data.coord, data.restore(s, object)?);
            self.retain(3, k, p); return Ok(output);
        }
        let deps = RefCell::new(Dependencies::default());
        let (coord, step) = source_pending::step_record(s, object, &RawReads { raw, deps: &deps }, equal)?;
        self.save(3, k, deps.into_inner(), Output::Step(StepData::read(coord, &step)));
        Ok((coord, step))
    }
    pub(super) fn request(
        &mut self, s: &mut (impl SourceFieldStore + ?Sized), request: &RefValue,
        steps: &impl source_pending::StepLookup, slots: &mut impl source_pending::RequestSlots, equal: bool,
    ) -> Result<(), String> {
        let k = key(request.view());
        if let Some(p) = self.previous.get(&(4, k.clone())).cloned() {
            let Output::Request(data) = &p.output else { unreachable!() };
            if data.valid(steps) {
                self.stats.hits += 1; self.publish();
                if slots.insert_request(s, &data.key)? { return Err(err("同cut知识请求重复")); }
                if let Some(coord) = data.bound {
                    if slots.insert_bound(s, coord)? { return Err(err("CC004知识请求key/subject/来源不一致")); }
                }
                self.retain(4, k, p); return Ok(());
            }
            self.stats.invalidations += 1; self.publish();
        }
        let data = RefCell::new(RequestData::default());
        source_pending::request_record_with_slots(s, request,
            &StepReads { steps, reads: &data }, &mut RequestWrites { slots, writes: &data }, equal)?;
        self.save(4, k, Dependencies::default(), Output::Request(data.into_inner()));
        Ok(())
    }
}
